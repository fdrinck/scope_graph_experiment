use string_interner::{DefaultBackend, DefaultSymbol, StringInterner};

use crate::ast::{Name, PathNode, SourceFile, Stmt};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ScopeId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DeclId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RefId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ImportId(pub u32);

pub type NameId = DefaultSymbol;

#[derive(Debug, Clone, Copy)]
pub enum ScopeKind {
    Plain,
    Decl(DeclId, NameId),
    Import(ImportId, NameId),
}

#[derive(Debug)]
pub struct Scope {
    pub parent: Option<ScopeId>,
    pub kind: ScopeKind,
}

#[derive(Debug)]
pub struct Declaration {
    pub scope: ScopeId,
    pub node: Name,
    pub child_scope: Option<ScopeId>,
    pub child_head_scope: Option<ScopeId>,
}

#[derive(Debug)]
pub struct Reference {
    pub scope: ScopeId,
    pub node: PathNode,
}

#[derive(Debug)]
pub struct Import {
    pub path_node: PathNode,
}

#[derive(Clone, Copy)]
struct VisitNode<'a> {
    import_id: ImportId,
    prev: Option<&'a VisitNode<'a>>,
}

impl<'a> VisitNode<'a> {
    #[inline]
    fn contains(&self, id: ImportId) -> bool {
        let mut curr = Some(self);
        while let Some(node) = curr {
            if node.import_id == id {
                return true;
            }
            curr = node.prev;
        }
        false
    }
}

#[derive(Debug, Default)]
pub struct ScopeGraph {
    scopes: Vec<Scope>,
    declarations: Vec<Declaration>,
    references: Vec<Reference>,
    imports: Vec<Import>,
    names: StringInterner<DefaultBackend>,
}

impl ScopeGraph {
    pub fn build(file: &SourceFile) -> Self {
        let mut graph = ScopeGraph {
            scopes: Vec::with_capacity(32),
            declarations: Vec::with_capacity(32),
            references: Vec::with_capacity(32),
            imports: Vec::with_capacity(16),
            names: StringInterner::default(),
        };

        let root_scope = graph.add_scope(None, ScopeKind::Plain);
        let mut current_scope = root_scope;
        for stmt in file.statements() {
            current_scope = graph.walk_stmt(&stmt, current_scope);
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

    #[inline]
    fn add_scope(&mut self, parent: Option<ScopeId>, kind: ScopeKind) -> ScopeId {
        let id = ScopeId(self.scopes.len() as u32);
        self.scopes.push(Scope { parent, kind });
        id
    }

    fn add_decl(
        &mut self,
        current_scope: ScopeId,
        name: Name,
        child_scope: Option<ScopeId>,
        child_head_scope: Option<ScopeId>,
    ) -> (DeclId, ScopeId) {
        let decl_id = DeclId(self.declarations.len() as u32);

        let next_scope = if let Some(token) = name.ident_token() {
            let name_id = self.names.get_or_intern(token.text());
            self.add_scope(Some(current_scope), ScopeKind::Decl(decl_id, name_id))
        } else {
            current_scope
        };

        self.declarations.push(Declaration {
            scope: next_scope,
            node: name,
            child_scope,
            child_head_scope,
        });

        (decl_id, next_scope)
    }

    fn add_ref(&mut self, scope: ScopeId, path: PathNode) -> RefId {
        let id = RefId(self.references.len() as u32);
        self.references.push(Reference { scope, node: path });
        id
    }

    fn add_import(&mut self, current_scope: ScopeId, path: PathNode) -> (ImportId, ScopeId) {
        let import_id = ImportId(self.imports.len() as u32);

        let next_scope = if let Some(token) = path.last_segment() {
            let name_id = self.names.get_or_intern(token.text());
            self.add_scope(Some(current_scope), ScopeKind::Import(import_id, name_id))
        } else {
            current_scope
        };

        self.imports.push(Import { path_node: path });

        (import_id, next_scope)
    }

    fn walk_stmt(&mut self, stmt: &Stmt, current_scope: ScopeId) -> ScopeId {
        match stmt {
            Stmt::ScopeDef(def) => {
                let body_head_scope = self.add_scope(Some(current_scope), ScopeKind::Plain);

                let mut body_tail_scope = body_head_scope;
                if let Some(block) = def.block() {
                    for child_stmt in block.statements() {
                        body_tail_scope = self.walk_stmt(&child_stmt, body_tail_scope);
                    }
                }

                if let Some(name) = def.name_node() {
                    let (_decl_id, next_scope) = self.add_decl(
                        current_scope,
                        name,
                        Some(body_tail_scope),
                        Some(body_head_scope),
                    );
                    next_scope
                } else {
                    current_scope
                }
            }
            Stmt::Block(block) => {
                let block_head_scope = self.add_scope(Some(current_scope), ScopeKind::Plain);
                let mut block_tail_scope = block_head_scope;
                for child_stmt in block.statements() {
                    block_tail_scope = self.walk_stmt(&child_stmt, block_tail_scope);
                }
                current_scope
            }
            Stmt::Let(let_stmt) => {
                let mut init_scope = current_scope;
                for init_stmt in let_stmt.initializer() {
                    init_scope = self.walk_stmt(&init_stmt, init_scope);
                }

                if let Some(name) = let_stmt.name_node() {
                    let (_decl_id, next_scope) = self.add_decl(current_scope, name, None, None);
                    next_scope
                } else {
                    current_scope
                }
            }
            Stmt::Import(import_stmt) => {
                if let Some(path) = import_stmt.path() {
                    let (_import_id, next_scope) = self.add_import(current_scope, path);
                    next_scope
                } else {
                    current_scope
                }
            }
            Stmt::Path(path) => {
                self.add_ref(current_scope, path.clone());
                current_scope
            }
            Stmt::Other(unparsed) => {
                let mut inner_scope = current_scope;
                for child_stmt in unparsed.children() {
                    inner_scope = self.walk_stmt(&child_stmt, inner_scope);
                }
                inner_scope
            }
        }
    }

    fn lookup_symbol_id<'a>(
        &self,
        start_scope: ScopeId,
        name: NameId,
        visited: Option<&'a VisitNode<'a>>,
    ) -> Option<DeclId> {
        self.lookup_symbol_bounded(start_scope, name, None, visited)
    }

    fn lookup_symbol_bounded<'a>(
        &self,
        start_scope: ScopeId,
        name: NameId,
        stop_at: Option<ScopeId>,
        visited: Option<&'a VisitNode<'a>>,
    ) -> Option<DeclId> {
        let mut curr = Some(start_scope);

        while let Some(scope_id) = curr {
            let scope = &self.scopes[scope_id.0 as usize];

            match scope.kind {
                ScopeKind::Decl(decl_id, decl_name) => {
                    if decl_name == name {
                        return Some(decl_id);
                    }
                }
                ScopeKind::Import(import_id, import_name) => {
                    if import_name == name {
                        if let Some(decl_id) = self.resolve_import(import_id, scope.parent, visited)
                        {
                            return Some(decl_id);
                        }
                    }
                }
                ScopeKind::Plain => {}
            }

            if Some(scope_id) == stop_at {
                break;
            }

            curr = scope.parent;
        }

        None
    }

    fn resolve_import<'a>(
        &self,
        import_id: ImportId,
        parent_scope: Option<ScopeId>,
        visited: Option<&'a VisitNode<'a>>,
    ) -> Option<DeclId> {
        if visited.map_or(false, |v| v.contains(import_id)) {
            return None;
        }

        let parent_scope = parent_scope?;
        let import = &self.imports[import_id.0 as usize];
        let visit_node = VisitNode {
            import_id,
            prev: visited,
        };

        self.resolve_path_with_visited(parent_scope, &import.path_node, Some(&visit_node))
    }

    fn resolve_path_with_visited<'a>(
        &self,
        start_scope: ScopeId,
        path: &PathNode,
        visited: Option<&'a VisitNode<'a>>,
    ) -> Option<DeclId> {
        let mut segments = path.segments();

        let first = segments.next()?;
        let first_name = self.names.get(first.text())?;
        let mut current_decl = self.lookup_symbol_id(start_scope, first_name, visited)?;

        for token in segments {
            let name = self.names.get(token.text())?;
            let decl = &self.declarations[current_decl.0 as usize];
            let child_tail_scope = decl.child_scope?;
            let child_head_scope = decl.child_head_scope?;

            current_decl = self.lookup_symbol_bounded(
                child_tail_scope,
                name,
                Some(child_head_scope),
                visited,
            )?;
        }

        Some(current_decl)
    }

    pub fn resolve_path(&self, start_scope: ScopeId, path: &PathNode) -> Option<DeclId> {
        self.resolve_path_with_visited(start_scope, path, None)
    }

    pub fn resolve(&self, ref_id: RefId) -> Option<DeclId> {
        let reference = &self.references[ref_id.0 as usize];
        self.resolve_path(reference.scope, &reference.node)
    }
}
