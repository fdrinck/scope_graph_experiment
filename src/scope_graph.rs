use std::collections::HashMap;
use string_interner::{DefaultBackend, DefaultSymbol, StringInterner};

use crate::ast::{AstNode, NameRef, PathNode, SourceFile, Stmt};
use crate::parser::{SyntaxKind, SyntaxNode};

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
    pub fn build(file: &SourceFile) -> Self {
        let mut graph = ScopeGraph {
            scopes: Vec::with_capacity(16),
            declarations: Vec::with_capacity(32),
            references: Vec::with_capacity(32),
            imports: Vec::with_capacity(16),
            names: StringInterner::default(),
            bindings: HashMap::with_capacity(64),
        };

        let root_scope = graph.add_scope(None);
        for stmt in file.statements() {
            graph.walk_stmt(&stmt, root_scope);
        }
        graph
    }

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

    fn add_scope(&mut self, parent: Option<ScopeId>) -> ScopeId {
        let id = ScopeId(self.scopes.len() as u32);
        self.scopes.push(Scope { parent });
        id
    }

    fn add_decl(
        &mut self,
        scope: ScopeId,
        name_ref: NameRef,
        child_scope: Option<ScopeId>,
    ) -> DeclId {
        let id = DeclId(self.declarations.len() as u32);

        if let Some(token) = name_ref.ident_token() {
            let name_id = self.names.get_or_intern(token.text());
            self.bindings.entry((scope, name_id)).or_default().decl = Some(id);
        }

        self.declarations.push(Declaration {
            scope,
            node: name_ref.0,
            child_scope,
        });
        id
    }

    fn add_ref(&mut self, scope: ScopeId, name_ref: NameRef) -> RefId {
        let id = RefId(self.references.len() as u32);
        self.references.push(Reference {
            scope,
            node: name_ref.0,
        });
        id
    }

    fn add_import(&mut self, scope: ScopeId, path: PathNode) -> ImportId {
        let id = ImportId(self.imports.len() as u32);

        if let Some(token) = path.last_segment() {
            let name_id = self.names.get_or_intern(token.text());
            self.bindings.entry((scope, name_id)).or_default().import = Some(id);
        }

        self.imports.push(Import { path_node: path.0 });
        id
    }

    fn walk_stmt(&mut self, stmt: &Stmt, current_scope: ScopeId) {
        match stmt {
            Stmt::ScopeDef(def) => {
                let named_scope = self.add_scope(Some(current_scope));

                if let Some(name) = def.name_node() {
                    self.add_decl(current_scope, name, Some(named_scope));
                }

                if let Some(block) = def.block() {
                    for child_stmt in block.statements() {
                        self.walk_stmt(&child_stmt, named_scope);
                    }
                }
            }
            Stmt::Block(block) => {
                let anon_scope = self.add_scope(Some(current_scope));

                for child_stmt in block.statements() {
                    self.walk_stmt(&child_stmt, anon_scope);
                }
            }
            Stmt::Let(let_stmt) => {
                if let Some(name) = let_stmt.name_node() {
                    self.add_decl(current_scope, name, None);
                }

                for init_stmt in let_stmt.initializer() {
                    self.walk_stmt(&init_stmt, current_scope);
                }
            }
            Stmt::Import(import_stmt) => {
                if let Some(path) = import_stmt.path() {
                    self.add_import(current_scope, path);
                }
            }
            Stmt::NameRef(name_ref) => {
                let node = stmt.syntax();
                let parent_kind = node.parent().map(|p| p.kind());

                if parent_kind != Some(SyntaxKind::PATH)
                    && parent_kind != Some(SyntaxKind::SCOPE_DEF)
                {
                    self.add_ref(current_scope, name_ref.clone());
                }
            }
            Stmt::Other(node) => {
                for child in node.children() {
                    self.walk_stmt(&Stmt::cast(child), current_scope);
                }
            }
        }
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

    fn resolve_symbol_in_scope(&self, scope: ScopeId, name: NameId) -> Option<DeclId> {
        if let Some(decl_id) = self.lookup_local(scope, name) {
            return Some(decl_id);
        }

        if let Some(import_id) = self.lookup_import(scope, name) {
            let import = &self.imports[import_id.0 as usize];
            if let Some(path) = PathNode::cast(import.path_node.clone()) {
                return self.resolve_path(ScopeId(0), &path);
            }
        }

        None
    }

    fn lookup_symbol_id(&self, start_scope: ScopeId, name: NameId) -> Option<DeclId> {
        let mut current = Some(start_scope);

        while let Some(scope_id) = current {
            if let Some(decl_id) = self.resolve_symbol_in_scope(scope_id, name) {
                return Some(decl_id);
            }
            current = self.scopes[scope_id.0 as usize].parent;
        }

        None
    }

    pub fn resolve_path(&self, start_scope: ScopeId, path: &PathNode) -> Option<DeclId> {
        let mut segments = path.segments();

        let first = segments.next()?;
        let first_name = self.names.get(first.text())?;
        let mut current_decl = self.lookup_symbol_id(start_scope, first_name)?;

        for token in segments {
            let name = self.names.get(token.text())?;
            let decl = &self.declarations[current_decl.0 as usize];
            let child_scope = decl.child_scope?;

            current_decl = self.resolve_symbol_in_scope(child_scope, name)?;
        }

        Some(current_decl)
    }

    pub fn resolve(&self, ref_id: RefId) -> Option<DeclId> {
        let reference = &self.references[ref_id.0 as usize];
        let path = PathNode::cast(reference.node.clone())?;
        self.resolve_path(reference.scope, &path)
    }
}
