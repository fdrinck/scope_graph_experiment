use crate::parser::{SyntaxKind, SyntaxNode, SyntaxToken};

pub trait AstNode: Sized {
    fn syntax(&self) -> &SyntaxNode;
}

macro_rules! ast_node {
    ($name:ident, $kind:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        pub struct $name(pub SyntaxNode);

        impl AstNode for $name {
            #[inline]
            fn syntax(&self) -> &SyntaxNode {
                &self.0
            }
        }

        impl $name {
            #[inline]
            #[allow(dead_code)]
            pub fn cast(node: SyntaxNode) -> Option<Self> {
                if node.kind() == SyntaxKind::$kind {
                    Some(Self(node))
                } else {
                    None
                }
            }
        }
    };
}

ast_node!(ScopeDef, SCOPE_DEF);
ast_node!(BlockExpr, BLOCK_EXPR);
ast_node!(LetStmt, LET_STMT);
ast_node!(ImportStmt, IMPORT_STMT);
ast_node!(NameRef, NAME_REF);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SourceFile(pub SyntaxNode);

impl AstNode for SourceFile {
    #[inline]
    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl SourceFile {
    pub fn cast(node: SyntaxNode) -> Self {
        Self(node)
    }

    pub fn statements(&self) -> impl Iterator<Item = Stmt> {
        self.0.children().map(Stmt::cast)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PathNode(pub SyntaxNode);

impl AstNode for PathNode {
    #[inline]
    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl PathNode {
    pub fn cast(node: SyntaxNode) -> Option<Self> {
        if node.kind() == SyntaxKind::PATH || node.kind() == SyntaxKind::NAME_REF {
            Some(Self(node))
        } else {
            None
        }
    }

    pub fn segments(&self) -> impl Iterator<Item = SyntaxToken> {
        self.0
            .descendants_with_tokens()
            .filter_map(|e| e.into_token())
            .filter(|t| t.kind() == SyntaxKind::IDENT)
    }

    pub fn last_segment(&self) -> Option<SyntaxToken> {
        self.segments().last()
    }
}

#[derive(Debug, Clone)]
pub enum Stmt {
    ScopeDef(ScopeDef),
    Block(BlockExpr),
    Let(LetStmt),
    Import(ImportStmt),
    NameRef(NameRef),
    Other(SyntaxNode),
}

impl AstNode for Stmt {
    fn syntax(&self) -> &SyntaxNode {
        match self {
            Stmt::ScopeDef(n) => n.syntax(),
            Stmt::Block(n) => n.syntax(),
            Stmt::Let(n) => n.syntax(),
            Stmt::Import(n) => n.syntax(),
            Stmt::NameRef(n) => n.syntax(),
            Stmt::Other(n) => n,
        }
    }
}

impl Stmt {
    pub fn cast(node: SyntaxNode) -> Self {
        match node.kind() {
            SyntaxKind::SCOPE_DEF => Stmt::ScopeDef(ScopeDef(node)),
            SyntaxKind::BLOCK_EXPR => Stmt::Block(BlockExpr(node)),
            SyntaxKind::LET_STMT => Stmt::Let(LetStmt(node)),
            SyntaxKind::IMPORT_STMT => Stmt::Import(ImportStmt(node)),
            SyntaxKind::NAME_REF => Stmt::NameRef(NameRef(node)),
            _ => Stmt::Other(node),
        }
    }
}

impl ScopeDef {
    pub fn name_node(&self) -> Option<NameRef> {
        self.0.children().find_map(NameRef::cast)
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
    pub fn name_node(&self) -> Option<NameRef> {
        self.0.children().find_map(NameRef::cast)
    }

    pub fn initializer(&self) -> impl Iterator<Item = Stmt> {
        let name_node = self.name_node().map(|n| n.0);
        self.0
            .children()
            .filter(move |child| Some(child) != name_node.as_ref())
            .map(Stmt::cast)
    }
}

impl ImportStmt {
    pub fn path(&self) -> Option<PathNode> {
        self.0.children().find_map(PathNode::cast)
    }
}

impl NameRef {
    pub fn ident_token(&self) -> Option<SyntaxToken> {
        self.0
            .descendants_with_tokens()
            .filter_map(|e| e.into_token())
            .find(|t| t.kind() == SyntaxKind::IDENT)
    }
}
