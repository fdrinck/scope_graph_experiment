use rowan::{GreenNodeBuilder, Language};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u16)]
pub enum SyntaxKind {
    Ident,
    Int,
    LetKw,
    ScopeKw,
    ImportKw,
    Dot,
    Semicolon,
    Eq,
    Star,
    Plus,
    LCurly,
    RCurly,
    Whitespace,
    Error,

    SourceFile,
    ScopeDef,
    BlockExpr,
    LetStmt,
    ImportStmt,
    Name,
    NameRef,
    Path,
}

impl From<SyntaxKind> for rowan::SyntaxKind {
    fn from(kind: SyntaxKind) -> Self {
        Self(kind as u16)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Lang;

impl Language for Lang {
    type Kind = SyntaxKind;

    fn kind_from_raw(raw: rowan::SyntaxKind) -> Self::Kind {
        unsafe { std::mem::transmute(raw.0) }
    }

    fn kind_to_raw(kind: Self::Kind) -> rowan::SyntaxKind {
        kind.into()
    }
}

pub type SyntaxNode = rowan::SyntaxNode<Lang>;
pub type SyntaxToken = rowan::SyntaxToken<Lang>;

pub fn parse(code: &str) -> SyntaxNode {
    let mut builder = GreenNodeBuilder::new();
    {
        let mut parser = Parser::new(code, &mut builder);
        parser.parse_source_file();
    }
    SyntaxNode::new_root(builder.finish())
}

struct Parser<'a, 'b> {
    code: &'a str,
    pos: usize,
    builder: &'b mut GreenNodeBuilder<'static>,
}

impl<'a, 'b> Parser<'a, 'b> {
    fn new(code: &'a str, builder: &'b mut GreenNodeBuilder<'static>) -> Self {
        Self {
            code,
            pos: 0,
            builder,
        }
    }

    fn parse_source_file(&mut self) {
        self.builder.start_node(SyntaxKind::SourceFile.into());
        self.skip_whitespace();

        while self.pos < self.code.len() {
            self.parse_stmt();
            self.skip_whitespace();
        }

        self.builder.finish_node();
    }

    fn parse_stmt(&mut self) {
        self.skip_whitespace();
        if self.looking_at("scope") {
            self.parse_scope_def();
        } else if self.looking_at("let") {
            self.parse_let_stmt();
        } else if self.looking_at("import") {
            self.parse_import_stmt();
        } else if self.looking_at("{") {
            self.parse_block();
        } else {
            self.parse_expr_stmt();
        }
    }

    fn parse_scope_def(&mut self) {
        self.builder.start_node(SyntaxKind::ScopeDef.into());
        self.consume_keyword("scope", SyntaxKind::ScopeKw);
        self.skip_whitespace();

        if let Some(ident) = self.consume_ident_str() {
            self.builder.start_node(SyntaxKind::Name.into());
            self.builder.token(SyntaxKind::Ident.into(), ident);
            self.builder.finish_node();
        }

        self.skip_whitespace();
        if self.looking_at("{") {
            self.parse_block();
        }
        self.builder.finish_node();
    }

    fn parse_let_stmt(&mut self) {
        self.builder.start_node(SyntaxKind::LetStmt.into());
        self.consume_keyword("let", SyntaxKind::LetKw);
        self.skip_whitespace();

        if let Some(ident) = self.consume_ident_str() {
            self.builder.start_node(SyntaxKind::Name.into());
            self.builder.token(SyntaxKind::Ident.into(), ident);
            self.builder.finish_node();
        }

        self.skip_whitespace();
        if self.consume_char('=') {
            self.skip_whitespace();
            self.parse_expr();
        }

        self.skip_whitespace();
        self.consume_char(';');
        self.builder.finish_node();
    }

    fn parse_import_stmt(&mut self) {
        self.builder.start_node(SyntaxKind::ImportStmt.into());
        self.consume_keyword("import", SyntaxKind::ImportKw);
        self.skip_whitespace();

        self.parse_path();

        self.skip_whitespace();
        self.consume_char(';');
        self.builder.finish_node();
    }

    fn parse_block(&mut self) {
        self.builder.start_node(SyntaxKind::BlockExpr.into());
        self.consume_char('{');

        while self.pos < self.code.len() && !self.looking_at("}") {
            self.parse_stmt();
            self.skip_whitespace();
        }

        self.consume_char('}');
        self.builder.finish_node();
    }

    fn parse_expr_stmt(&mut self) {
        self.parse_expr();
        self.skip_whitespace();
        self.consume_char(';');
    }

    fn parse_expr(&mut self) {
        self.parse_binary_expr(0);
    }

    fn parse_binary_expr(&mut self, min_bp: u8) {
        self.skip_whitespace();
        self.parse_primary_expr();

        loop {
            self.skip_whitespace();
            let op = if self.looking_at("+") {
                Some(('+', 1))
            } else if self.looking_at("*") {
                Some(('*', 2))
            } else {
                None
            };

            if let Some((ch, bp)) = op {
                if bp < min_bp {
                    break;
                }
                self.consume_char(ch);
                self.parse_binary_expr(bp + 1);
            } else {
                break;
            }
        }
    }

    fn parse_primary_expr(&mut self) {
        self.skip_whitespace();
        if self.looking_at_ident_start() {
            self.parse_path();
        } else if self.looking_at_number() {
            let num = self.consume_number();
            self.builder.token(SyntaxKind::Int.into(), num);
        }
    }

    fn parse_path(&mut self) {
        self.builder.start_node(SyntaxKind::Path.into());
        if let Some(ident) = self.consume_ident_str() {
            self.builder.token(SyntaxKind::Ident.into(), ident);
        }

        while self.looking_at(".") {
            self.consume_char('.');
            self.skip_whitespace();
            if let Some(ident) = self.consume_ident_str() {
                self.builder.token(SyntaxKind::Ident.into(), ident);
            }
        }
        self.builder.finish_node();
    }

    fn skip_whitespace(&mut self) {
        let start = self.pos;
        while self.pos < self.code.len() {
            let ch = self.code[self.pos..].chars().next().unwrap();
            if ch.is_whitespace() {
                self.pos += ch.len_utf8();
            } else {
                break;
            }
        }
        if self.pos > start {
            self.builder
                .token(SyntaxKind::Whitespace.into(), &self.code[start..self.pos]);
        }
    }

    fn looking_at(&self, s: &str) -> bool {
        self.code[self.pos..].starts_with(s)
    }

    fn looking_at_ident_start(&self) -> bool {
        self.code[self.pos..]
            .chars()
            .next()
            .map_or(false, |c| c.is_alphabetic() || c == '_')
    }

    fn looking_at_number(&self) -> bool {
        self.code[self.pos..]
            .chars()
            .next()
            .map_or(false, |c| c.is_ascii_digit())
    }

    fn consume_char(&mut self, ch: char) -> bool {
        if self.code[self.pos..].starts_with(ch) {
            let kind = match ch {
                '=' => SyntaxKind::Eq,
                ';' => SyntaxKind::Semicolon,
                '.' => SyntaxKind::Dot,
                '+' => SyntaxKind::Plus,
                '*' => SyntaxKind::Star,
                '{' => SyntaxKind::LCurly,
                '}' => SyntaxKind::RCurly,
                _ => SyntaxKind::Error,
            };
            self.builder.token(kind.into(), &ch.to_string());
            self.pos += ch.len_utf8();
            true
        } else {
            false
        }
    }

    fn consume_keyword(&mut self, kw: &str, kind: SyntaxKind) {
        if self.looking_at(kw) {
            self.builder.token(kind.into(), kw);
            self.pos += kw.len();
        }
    }

    fn consume_ident_str(&mut self) -> Option<&'a str> {
        let start = self.pos;
        let mut chars = self.code[self.pos..].chars();
        if let Some(first) = chars.next() {
            if first.is_alphabetic() || first == '_' {
                self.pos += first.len_utf8();
                while let Some(ch) = chars.next() {
                    if ch.is_alphanumeric() || ch == '_' {
                        self.pos += ch.len_utf8();
                    } else {
                        break;
                    }
                }
                return Some(&self.code[start..self.pos]);
            }
        }
        None
    }

    fn consume_number(&mut self) -> &'a str {
        let start = self.pos;
        while self.pos < self.code.len() {
            let ch = self.code[self.pos..].chars().next().unwrap();
            if ch.is_ascii_digit() {
                self.pos += ch.len_utf8();
            } else {
                break;
            }
        }
        &self.code[start..self.pos]
    }
}
