use crate::parser::{SyntaxKind, SyntaxNode, SyntaxToken};
use rowan::TextRange;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IdentToken(pub(crate) SyntaxToken);

impl IdentToken {
    #[inline]
    pub fn text(&self) -> &str {
        self.0.text()
    }
}

macro_rules! ast_node {
    ($name:ident, $kind:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        pub struct $name(pub(crate) SyntaxNode);

        impl $name {
            #[inline]
            #[allow(dead_code)]
            fn cast(node: SyntaxNode) -> Option<Self> {
                if node.kind() == SyntaxKind::$kind {
                    Some(Self(node))
                } else {
                    None
                }
            }

            #[inline]
            #[allow(dead_code)]
            pub fn text_range(&self) -> TextRange {
                self.0.text_range()
            }
        }
    };
}

ast_node!(Name, Name);
ast_node!(ScopeDef, ScopeDef);
ast_node!(BlockExpr, BlockExpr);
ast_node!(LetStmt, LetStmt);
ast_node!(ImportStmt, ImportStmt);

impl Name {
    pub fn ident_token(&self) -> Option<IdentToken> {
        self.0
            .descendants_with_tokens()
            .filter_map(|e| e.into_token())
            .find(|t| t.kind() == SyntaxKind::Ident)
            .map(IdentToken)
    }

    pub fn text(&self) -> String {
        self.ident_token().unwrap().text().into()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SourceFile(pub(crate) SyntaxNode);

impl SourceFile {
    pub fn cast(node: SyntaxNode) -> Self {
        Self(node)
    }

    pub fn statements(&self) -> impl Iterator<Item = Stmt> {
        self.0.children().map(Stmt::cast)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PathNode(pub(crate) SyntaxNode);

impl PathNode {
    pub(crate) fn cast(node: SyntaxNode) -> Option<Self> {
        if node.kind() == SyntaxKind::Path || node.kind() == SyntaxKind::NameRef {
            Some(Self(node))
        } else {
            None
        }
    }

    #[inline]
    pub fn text_range(&self) -> TextRange {
        self.0.text_range()
    }

    pub fn segments(&self) -> impl Iterator<Item = IdentToken> {
        self.0
            .descendants_with_tokens()
            .filter_map(|e| e.into_token())
            .filter(|t| t.kind() == SyntaxKind::Ident)
            .map(IdentToken)
    }

    pub fn last_segment(&self) -> Option<IdentToken> {
        self.segments().last()
    }

    pub fn text(&self) -> String {
        self.segments()
            .map(|t| t.text().to_string())
            .collect::<Vec<_>>()
            .join("::")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UnparsedStmt(pub(crate) SyntaxNode);

impl UnparsedStmt {
    pub fn children(&self) -> impl Iterator<Item = Stmt> {
        self.0.children().map(Stmt::cast)
    }
}

#[derive(Debug, Clone)]
pub enum Stmt {
    ScopeDef(ScopeDef),
    Block(BlockExpr),
    Let(LetStmt),
    Import(ImportStmt),
    Path(PathNode),
    Other(UnparsedStmt),
}

impl Stmt {
    pub(crate) fn cast(node: SyntaxNode) -> Self {
        match node.kind() {
            SyntaxKind::ScopeDef => Stmt::ScopeDef(ScopeDef(node)),
            SyntaxKind::BlockExpr => Stmt::Block(BlockExpr(node)),
            SyntaxKind::LetStmt => Stmt::Let(LetStmt(node)),
            SyntaxKind::ImportStmt => Stmt::Import(ImportStmt(node)),
            SyntaxKind::Path | SyntaxKind::NameRef => Stmt::Path(PathNode(node)),
            _ => Stmt::Other(UnparsedStmt(node)),
        }
    }
}

impl ScopeDef {
    pub fn name_node(&self) -> Option<Name> {
        self.0.children().find_map(Name::cast)
    }

    pub fn block(&self) -> Option<BlockExpr> {
        self.0.children().find_map(BlockExpr::cast)
    }
}

impl BlockExpr {
    pub fn statements(&self) -> impl Iterator<Item = Stmt> {
        self.0.children().map(Stmt::cast)
    }
}

impl LetStmt {
    pub fn name_node(&self) -> Option<Name> {
        self.0.children().find_map(Name::cast)
    }

    pub fn initializer(&self) -> impl Iterator<Item = Stmt> {
        let name_syntax = self.name_node().map(|n| n.0);
        self.0
            .children()
            .filter(move |child| Some(child) != name_syntax.as_ref())
            .map(Stmt::cast)
    }
}

impl ImportStmt {
    pub fn path(&self) -> Option<PathNode> {
        self.0.children().find_map(PathNode::cast)
    }
}
