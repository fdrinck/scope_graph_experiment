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

struct Lexer<'a> {
    input: &'a str,
}

impl<'a> Lexer<'a> {
    fn new(input: &'a str) -> Self {
        Self { input }
    }

    fn next_token(&mut self) -> Option<(SyntaxKind, &'a str)> {
        if self.input.is_empty() {
            return None;
        }

        let mut chars = self.input.char_indices();
        let (_, c) = chars.next()?;

        let (kind, len) = match c {
            c if c.is_whitespace() => {
                let len = self
                    .input
                    .find(|c: char| !c.is_whitespace())
                    .unwrap_or(self.input.len());
                (WHITESPACE, len)
            }
            '+' => (PLUS, 1),
            '-' => (MINUS, 1),
            '*' => (STAR, 1),
            '=' => (EQ, 1),
            '.' => (DOT, 1),
            ';' => (SEMICOLON, 1),
            '{' => (L_CURLY, 1),
            '}' => (R_CURLY, 1),
            '0'..='9' => {
                let len = self
                    .input
                    .find(|c: char| !c.is_numeric())
                    .unwrap_or(self.input.len());
                (INT, len)
            }
            'a'..='z' | 'A'..='Z' | '_' => {
                let len = self
                    .input
                    .find(|c: char| !c.is_alphanumeric() && c != '_')
                    .unwrap_or(self.input.len());
                let text = &self.input[..len];
                match text {
                    "let" => (LET_KW, len),
                    "scope" => (SCOPE_KW, len),
                    "import" => (IMPORT_KW, len),
                    _ => (IDENT, len),
                }
            }
            _ => (ERROR, 1),
        };

        let (token_text, rest) = self.input.split_at(len);
        self.input = rest;
        Some((kind, token_text))
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
