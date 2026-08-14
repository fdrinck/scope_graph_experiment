use rowan::{
    GreenNodeBuilder, Language, SyntaxNode as RowanSyntaxNode, SyntaxToken as RowanSyntaxToken,
};

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u16)]
pub enum SyntaxKind {
    // Tokens
    WHITESPACE = 0,
    LET_KW,
    SCOPE_KW,
    IMPORT_KW,
    IDENT,
    INT,
    PLUS,
    MINUS,
    STAR,
    EQ,
    DOT,
    SEMICOLON,
    L_CURLY,
    R_CURLY,
    ERROR,

    // Composite Nodes
    SOURCE_FILE,
    LET_STMT,
    SCOPE_DEF,
    IMPORT_STMT,
    EXPR_STMT,
    BLOCK_EXPR,
    BINARY_EXPR,
    LITERAL,
    NAME_REF,
    PATH,
}

use SyntaxKind::*;

impl From<SyntaxKind> for rowan::SyntaxKind {
    fn from(kind: SyntaxKind) -> Self {
        Self(kind as u16)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Lang {}

impl Language for Lang {
    type Kind = SyntaxKind;

    fn kind_from_raw(raw: rowan::SyntaxKind) -> Self::Kind {
        assert!(raw.0 <= PATH as u16);
        unsafe { std::mem::transmute(raw.0) }
    }

    fn kind_to_raw(kind: Self::Kind) -> rowan::SyntaxKind {
        kind.into()
    }
}

pub type SyntaxNode = RowanSyntaxNode<Lang>;
pub type SyntaxToken = RowanSyntaxToken<Lang>;

pub struct Lexer<'a> {
    source: &'a str,
    cursor: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self { source, cursor: 0 }
    }

    /// Peeks the character at the current cursor position.
    #[inline]
    fn peek(&self) -> Option<char> {
        self.source[self.cursor..].chars().next()
    }

    /// Consumes the current character and advances the cursor.
    #[inline]
    fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.cursor += c.len_utf8();
        Some(c)
    }

    /// Advances while `predicate` holds true.
    #[inline]
    fn eat_while(&mut self, mut predicate: impl FnMut(char) -> bool) {
        while self.peek().is_some_and(&mut predicate) {
            self.bump();
        }
    }

    pub fn next_token(&mut self) -> Option<(SyntaxKind, &'a str)> {
        if self.cursor >= self.source.len() {
            return None;
        }

        let start = self.cursor;
        let first = self.bump()?;

        let kind = match first {
            c if c.is_whitespace() => {
                self.eat_while(|c| c.is_whitespace());
                WHITESPACE
            }
            '0'..='9' => {
                self.eat_while(|c| c.is_ascii_digit());
                INT
            }
            'a'..='z' | 'A'..='Z' | '_' => {
                self.eat_while(|c| c.is_alphanumeric() || c == '_');
                match &self.source[start..self.cursor] {
                    "let" => LET_KW,
                    "scope" => SCOPE_KW,
                    "import" => IMPORT_KW,
                    _ => IDENT,
                }
            }
            '+' => PLUS,
            '-' => MINUS,
            '*' => STAR,
            '=' => EQ,
            '.' => DOT,
            ';' => SEMICOLON,
            '{' => L_CURLY,
            '}' => R_CURLY,
            _ => ERROR,
        };

        Some((kind, &self.source[start..self.cursor]))
    }
}

pub struct Parser<'a> {
    tokens: Vec<(SyntaxKind, &'a str)>,
    pos: usize,
    builder: GreenNodeBuilder<'static>,
}

impl<'a> Parser<'a> {
    pub fn new(input: &'a str) -> Self {
        let mut lexer = Lexer::new(input);
        let mut tokens = Vec::new();
        while let Some(token) = lexer.next_token() {
            tokens.push(token);
        }

        Self {
            tokens,
            pos: 0,
            builder: GreenNodeBuilder::new(),
        }
    }

    fn peek(&self) -> Option<SyntaxKind> {
        self.tokens.get(self.pos).map(|(k, _)| *k)
    }

    fn bump(&mut self) {
        if let Some((kind, text)) = self.tokens.get(self.pos) {
            self.builder.token((*kind).into(), text);
            self.pos += 1;
        }
    }

    fn eat_ws(&mut self) {
        while self.peek() == Some(WHITESPACE) {
            self.bump();
        }
    }

    pub fn parse(mut self) -> SyntaxNode {
        self.builder.start_node(SOURCE_FILE.into());
        while self.peek().is_some() {
            self.parse_stmt();
        }
        self.builder.finish_node();
        SyntaxNode::new_root(self.builder.finish())
    }

    fn parse_stmt(&mut self) {
        self.eat_ws();
        match self.peek() {
            Some(LET_KW) => self.parse_let_stmt(),
            Some(SCOPE_KW) => self.parse_scope_def(),
            Some(IMPORT_KW) => self.parse_import_stmt(),
            Some(_) => self.parse_expr_stmt(),
            None => {}
        }
    }

    fn parse_scope_def(&mut self) {
        self.builder.start_node(SCOPE_DEF.into());
        self.bump();
        self.eat_ws();

        if self.peek() == Some(IDENT) {
            self.builder.start_node(NAME_REF.into());
            self.bump();
            self.builder.finish_node();
        }
        self.eat_ws();

        if self.peek() == Some(L_CURLY) {
            let checkpoint = self.builder.checkpoint();
            self.parse_block(checkpoint);
        }
        self.builder.finish_node();
    }

    fn parse_import_stmt(&mut self) {
        self.builder.start_node(IMPORT_STMT.into());
        self.bump();
        self.eat_ws();

        self.parse_path();
        self.eat_ws();

        if self.peek() == Some(SEMICOLON) {
            self.bump();
        }
        self.builder.finish_node();
    }

    fn parse_path(&mut self) {
        self.builder.start_node(PATH.into());
        if self.peek() == Some(IDENT) {
            self.bump();
        }
        loop {
            self.eat_ws();
            if self.peek() == Some(DOT) {
                self.bump();
                self.eat_ws();
                if self.peek() == Some(IDENT) {
                    self.bump();
                }
            } else {
                break;
            }
        }
        self.builder.finish_node();
    }

    fn parse_let_stmt(&mut self) {
        self.builder.start_node(LET_STMT.into());
        self.bump();
        self.eat_ws();

        if self.peek() == Some(IDENT) {
            self.builder.start_node(NAME_REF.into());
            self.bump();
            self.builder.finish_node();
        }
        self.eat_ws();

        if self.peek() == Some(EQ) {
            self.bump();
        }
        self.eat_ws();

        self.parse_expr(0);
        self.eat_ws();

        if self.peek() == Some(SEMICOLON) {
            self.bump();
        }
        self.builder.finish_node();
    }

    fn parse_expr_stmt(&mut self) {
        self.builder.start_node(EXPR_STMT.into());
        self.parse_expr(0);
        self.eat_ws();
        if self.peek() == Some(SEMICOLON) {
            self.bump();
        }
        self.builder.finish_node();
    }

    fn parse_expr(&mut self, min_bp: u8) {
        self.eat_ws();
        let checkpoint = self.builder.checkpoint();

        match self.peek() {
            Some(INT) => {
                self.builder.start_node_at(checkpoint, LITERAL.into());
                self.bump();
                self.builder.finish_node();
            }
            Some(IDENT) => {
                self.builder.start_node_at(checkpoint, NAME_REF.into());
                self.bump();
                self.builder.finish_node();
            }
            Some(L_CURLY) => {
                self.parse_block(checkpoint);
            }
            _ => return,
        }

        loop {
            self.eat_ws();
            let (l_bp, r_bp) = match self.peek() {
                Some(PLUS) | Some(MINUS) => (1, 2),
                Some(STAR) => (3, 4),
                _ => break,
            };

            if l_bp < min_bp {
                break;
            }

            self.builder.start_node_at(checkpoint, BINARY_EXPR.into());
            self.bump();
            self.parse_expr(r_bp);
            self.builder.finish_node();
        }
    }

    fn parse_block(&mut self, checkpoint: rowan::Checkpoint) {
        self.builder.start_node_at(checkpoint, BLOCK_EXPR.into());
        self.bump();

        loop {
            self.eat_ws();
            match self.peek() {
                Some(R_CURLY) => {
                    self.bump();
                    break;
                }
                None => break,
                _ => self.parse_stmt(),
            }
        }
        self.builder.finish_node();
    }
}

pub fn parse(code: &str) -> SyntaxNode {
    Parser::new(code).parse()
}
