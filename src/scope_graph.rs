use crate::parser::{SyntaxKind, SyntaxNode, SyntaxToken};
use std::collections::HashMap;
use string_interner::{DefaultBackend, DefaultSymbol, StringInterner};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ScopeId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DeclId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RefId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ImportId(pub u32);

pub type NameId = DefaultSymbol;

#[derive(Debug)]
pub struct Scope {
    pub parent: Option<ScopeId>,
    pub enclosing: Option<ScopeId>,
}

#[derive(Debug)]
pub struct Declaration {
    pub scope: ScopeId,
    pub node: SyntaxNode,
    pub child_scope: Option<ScopeId>,
}

#[derive(Debug)]
pub struct Reference {
    pub scope: ScopeId,
    pub node: SyntaxNode,
}

#[derive(Debug)]
pub struct Import {
    pub path_node: SyntaxNode,
}

#[derive(Debug, Default)]
struct Binding {
    decl: Option<DeclId>,
    import: Option<ImportId>,
}

type BindingKey = (ScopeId, NameId);

#[derive(Debug, Default)]
pub struct ScopeGraph {
    scopes: Vec<Scope>,
    declarations: Vec<Declaration>,
    references: Vec<Reference>,
    imports: Vec<Import>,
    names: StringInterner<DefaultBackend>,
    bindings: HashMap<BindingKey, Binding>,
}

impl ScopeGraph {
    pub fn build(root: &SyntaxNode) -> Self {
        let mut graph = ScopeGraph {
            scopes: Vec::with_capacity(16),
            declarations: Vec::with_capacity(32),
            references: Vec::with_capacity(32),
            imports: Vec::with_capacity(16),
            names: StringInterner::default(),
            bindings: HashMap::with_capacity(64),
        };

        let root_scope = graph.add_scope(None, None);
        graph.walk_node(root, root_scope);
        graph
    }

    // ------------------------------------------------------------------------
    // Accessors
    // ------------------------------------------------------------------------

    #[inline]
    pub fn declaration(&self, id: DeclId) -> Option<&Declaration> {
        self.declarations.get(id.0 as usize)
    }

    #[inline]
    pub fn references(&self) -> &[Reference] {
        &self.references
    }

    #[inline]
    pub fn scope_count(&self) -> usize {
        self.scopes.len()
    }

    #[inline]
    pub fn declaration_count(&self) -> usize {
        self.declarations.len()
    }

    #[inline]
    pub fn reference_count(&self) -> usize {
        self.references.len()
    }

    #[inline]
    pub fn import_count(&self) -> usize {
        self.imports.len()
    }

    // ------------------------------------------------------------------------
    // Internal Graph Construction
    // ------------------------------------------------------------------------

    fn add_scope(&mut self, parent: Option<ScopeId>, enclosing: Option<ScopeId>) -> ScopeId {
        let id = ScopeId(self.scopes.len() as u32);
        self.scopes.push(Scope { parent, enclosing });
        id
    }

    fn add_decl(
        &mut self,
        scope: ScopeId,
        node: SyntaxNode,
        child_scope: Option<ScopeId>,
    ) -> DeclId {
        let id = DeclId(self.declarations.len() as u32);

        self.declarations.push(Declaration {
            scope,
            node,
            child_scope,
        });

        if let Some(name_token) = Self::name_token(&self.declarations[id.0 as usize].node) {
            let name_id = self.names.get_or_intern(name_token.text());

            self.bindings.entry((scope, name_id)).or_default().decl = Some(id);
        }

        id
    }

    fn add_ref(&mut self, scope: ScopeId, node: SyntaxNode) -> RefId {
        let id = RefId(self.references.len() as u32);
        self.references.push(Reference { scope, node });
        id
    }

    fn add_import(&mut self, scope: ScopeId, path_node: SyntaxNode) -> ImportId {
        let id = ImportId(self.imports.len() as u32);

        let alias =
            Self::last_ident_token(&path_node).map(|token| self.names.get_or_intern(token.text()));

        self.imports.push(Import { path_node });

        if let Some(name_id) = alias {
            self.bindings.entry((scope, name_id)).or_default().import = Some(id);
        }

        id
    }

    fn walk_node(&mut self, node: &SyntaxNode, current_scope: ScopeId) {
        match node.kind() {
            SyntaxKind::SCOPE_DEF => {
                let mut children = node.children();
                let name_node = children.find(|c| c.kind() == SyntaxKind::NAME_REF);
                let block_node = children.find(|c| c.kind() == SyntaxKind::BLOCK_EXPR);

                let named_scope = self.add_scope(None, Some(current_scope));

                if let Some(name) = name_node {
                    self.add_decl(current_scope, name, Some(named_scope));
                }

                if let Some(block) = block_node {
                    for child in block.children() {
                        self.walk_node(&child, named_scope);
                    }
                }
            }
            SyntaxKind::BLOCK_EXPR => {
                let anon_scope = self.add_scope(Some(current_scope), Some(current_scope));

                for child in node.children() {
                    self.walk_node(&child, anon_scope);
                }
            }
            SyntaxKind::LET_STMT => {
                let mut children = node.children();

                if let Some(name_node) = children.find(|c| c.kind() == SyntaxKind::NAME_REF) {
                    self.add_decl(current_scope, name_node, None);
                }

                for child in children {
                    self.walk_node(&child, current_scope);
                }
            }
            SyntaxKind::IMPORT_STMT => {
                if let Some(path) = node.children().find(|c| c.kind() == SyntaxKind::PATH) {
                    self.add_import(current_scope, path);
                }
            }
            SyntaxKind::NAME_REF => {
                let parent_kind = node.parent().map(|p| p.kind());

                if parent_kind != Some(SyntaxKind::PATH)
                    && parent_kind != Some(SyntaxKind::SCOPE_DEF)
                {
                    self.add_ref(current_scope, node.clone());
                }
            }
            _ => {
                for child in node.children() {
                    self.walk_node(&child, current_scope);
                }
            }
        }
    }

    #[inline]
    fn name_token(node: &SyntaxNode) -> Option<SyntaxToken> {
        node.children_with_tokens()
            .filter_map(|element| element.into_token())
            .find(|token| token.kind() == SyntaxKind::IDENT)
    }

    #[inline]
    fn last_ident_token(path_node: &SyntaxNode) -> Option<SyntaxToken> {
        path_node
            .children_with_tokens()
            .filter_map(|element| element.into_token())
            .filter(|token| token.kind() == SyntaxKind::IDENT)
            .last()
    }

    #[inline]
    fn binding(&self, scope: ScopeId, name: NameId) -> Option<&Binding> {
        self.bindings.get(&(scope, name))
    }

    #[inline]
    fn lookup_local(&self, scope: ScopeId, name: NameId) -> Option<DeclId> {
        self.binding(scope, name).and_then(|b| b.decl)
    }

    #[inline]
    fn lookup_import(&self, scope: ScopeId, name: NameId) -> Option<ImportId> {
        self.binding(scope, name).and_then(|b| b.import)
    }

    fn lookup_symbol_id(&self, start_scope: ScopeId, name: NameId) -> Option<DeclId> {
        let mut current = Some(start_scope);

        while let Some(scope_id) = current {
            let scope = &self.scopes[scope_id.0 as usize];

            if let Some(decl_id) = self.lookup_local(scope_id, name) {
                return Some(decl_id);
            }

            if let Some(import_id) = self.lookup_import(scope_id, name) {
                let import = &self.imports[import_id.0 as usize];
                let eval_scope = scope.enclosing.unwrap_or(scope_id);

                return self.resolve_path(eval_scope, &import.path_node);
            }

            current = scope.parent;
        }

        None
    }

    pub fn resolve_path(&self, start_scope: ScopeId, path_node: &SyntaxNode) -> Option<DeclId> {
        let mut idents = path_node
            .children_with_tokens()
            .filter_map(|element| element.into_token())
            .filter(|token| token.kind() == SyntaxKind::IDENT);

        let first = idents.next()?;
        let first_name = self.names.get(first.text())?;
        let mut current_decl = self.lookup_symbol_id(start_scope, first_name)?;

        for token in idents {
            let name = self.names.get(token.text())?;
            let decl = &self.declarations[current_decl.0 as usize];
            let child_scope = decl.child_scope?;

            current_decl = self.lookup_local(child_scope, name)?;
        }

        Some(current_decl)
    }

    pub fn resolve(&self, ref_id: RefId) -> Option<DeclId> {
        let reference = &self.references[ref_id.0 as usize];
        self.resolve_path(reference.scope, &reference.node)
    }
}
