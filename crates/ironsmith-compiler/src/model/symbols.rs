use std::collections::HashMap;

use crate::model::provenance::{ProvenanceId, SemanticProvenance};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SymbolId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SymbolScopeId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReferenceRole {
    Source,
    Target,
    Chosen,
    Affected,
    Revealed,
    Searched,
    Exiled,
    Discarded,
    Sacrificed,
    Triggering,
    CostPaid,
    Created,
    Copied,
    Iteration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Cardinality {
    ExactlyOne,
    ZeroOrOne,
    OneOrMore,
    Any,
    Fixed(u32),
    Range { min: u32, max: Option<u32> },
}

impl Cardinality {
    pub fn accepts(self, count: u32) -> bool {
        match self {
            Self::ExactlyOne => count == 1,
            Self::ZeroOrOne => count <= 1,
            Self::OneOrMore => count >= 1,
            Self::Any => true,
            Self::Fixed(expected) => count == expected,
            Self::Range { min, max } => count >= min && max.is_none_or(|max| count <= max),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ObjectDomain {
    Object,
    Card,
    Spell,
    Permanent,
    Player,
    EffectResult,
    Event,
    Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SymbolScopeKind {
    Root,
    Document,
    Line,
    NestedAbility,
    ModalMode,
    TokenDefinition,
    Branch,
    Iteration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SymbolScope {
    pub id: SymbolScopeId,
    pub parent: Option<SymbolScopeId>,
    pub kind: SymbolScopeKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolBinding {
    pub id: SymbolId,
    pub scope: SymbolScopeId,
    pub role: ReferenceRole,
    pub cardinality: Cardinality,
    pub domain: ObjectDomain,
    pub provenance: Option<SemanticProvenance>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReferenceQuery {
    pub scope: SymbolScopeId,
    pub role: ReferenceRole,
    pub domain: ObjectDomain,
    pub required_cardinality: Option<Cardinality>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SymbolResolutionError {
    Unresolved(ReferenceQuery),
    Ambiguous {
        query: ReferenceQuery,
        candidates: Vec<SymbolId>,
    },
    WrongDomain {
        query: ReferenceQuery,
        candidates: Vec<SymbolId>,
    },
    WrongCardinality {
        query: ReferenceQuery,
        candidates: Vec<SymbolId>,
    },
    UnknownScope(SymbolScopeId),
}

#[derive(Debug, Clone)]
pub struct SymbolTable {
    scopes: Vec<SymbolScope>,
    bindings: Vec<SymbolBinding>,
    by_scope: HashMap<SymbolScopeId, Vec<SymbolId>>,
    next_scope: u32,
    next_symbol: u32,
}

impl Default for SymbolTable {
    fn default() -> Self {
        let root = SymbolScope {
            id: SymbolScopeId(0),
            parent: None,
            kind: SymbolScopeKind::Root,
        };
        Self {
            scopes: vec![root],
            bindings: Vec::new(),
            by_scope: HashMap::from([(root.id, Vec::new())]),
            next_scope: 1,
            next_symbol: 0,
        }
    }
}

impl SymbolTable {
    pub const fn root_scope(&self) -> SymbolScopeId {
        SymbolScopeId(0)
    }

    pub fn create_scope(
        &mut self,
        parent: SymbolScopeId,
        kind: SymbolScopeKind,
    ) -> Result<SymbolScopeId, SymbolResolutionError> {
        if self.scope(parent).is_none() {
            return Err(SymbolResolutionError::UnknownScope(parent));
        }
        let id = SymbolScopeId(self.next_scope);
        self.next_scope = self
            .next_scope
            .checked_add(1)
            .expect("symbol scope identifier overflow");
        self.scopes.push(SymbolScope {
            id,
            parent: Some(parent),
            kind,
        });
        self.by_scope.insert(id, Vec::new());
        Ok(id)
    }

    pub fn bind(
        &mut self,
        scope: SymbolScopeId,
        role: ReferenceRole,
        cardinality: Cardinality,
        domain: ObjectDomain,
        provenance: Option<ProvenanceId>,
    ) -> Result<SymbolId, SymbolResolutionError> {
        if self.scope(scope).is_none() {
            return Err(SymbolResolutionError::UnknownScope(scope));
        }
        let id = SymbolId(self.next_symbol);
        self.next_symbol = self
            .next_symbol
            .checked_add(1)
            .expect("symbol identifier overflow");
        self.bindings.push(SymbolBinding {
            id,
            scope,
            role,
            cardinality,
            domain,
            provenance: provenance.map(|primary| SemanticProvenance {
                primary,
                related: Vec::new(),
            }),
        });
        self.by_scope.entry(scope).or_default().push(id);
        Ok(id)
    }

    pub fn binding(&self, id: SymbolId) -> Option<&SymbolBinding> {
        self.bindings
            .get(id.0 as usize)
            .filter(|binding| binding.id == id)
    }

    pub fn scope(&self, id: SymbolScopeId) -> Option<&SymbolScope> {
        self.scopes
            .get(id.0 as usize)
            .filter(|scope| scope.id == id)
    }

    pub fn resolve(&self, query: ReferenceQuery) -> Result<SymbolId, SymbolResolutionError> {
        let mut scope = Some(query.scope);
        let mut wrong_domain = Vec::new();
        let mut wrong_cardinality = Vec::new();
        while let Some(scope_id) = scope {
            let Some(scope_record) = self.scope(scope_id) else {
                return Err(SymbolResolutionError::UnknownScope(scope_id));
            };
            let mut candidates = Vec::new();
            for id in self.by_scope.get(&scope_id).into_iter().flatten().rev() {
                let Some(binding) = self.binding(*id) else {
                    continue;
                };
                if binding.role != query.role {
                    continue;
                }
                if binding.domain != query.domain {
                    wrong_domain.push(binding.id);
                    continue;
                }
                if query
                    .required_cardinality
                    .is_some_and(|required| required != binding.cardinality)
                {
                    wrong_cardinality.push(binding.id);
                    continue;
                }
                candidates.push(binding.id);
            }
            match candidates.as_slice() {
                [id] => return Ok(*id),
                [] => {}
                _ => {
                    return Err(SymbolResolutionError::Ambiguous { query, candidates });
                }
            }
            scope = scope_record.parent;
        }
        if !wrong_domain.is_empty() {
            Err(SymbolResolutionError::WrongDomain {
                query,
                candidates: wrong_domain,
            })
        } else if !wrong_cardinality.is_empty() {
            Err(SymbolResolutionError::WrongCardinality {
                query,
                candidates: wrong_cardinality,
            })
        } else {
            Err(SymbolResolutionError::Unresolved(query))
        }
    }

    pub fn bindings(&self) -> &[SymbolBinding] {
        &self.bindings
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SymbolReference {
    pub symbol: SymbolId,
    pub role: ReferenceRole,
    pub domain: ObjectDomain,
    pub cardinality: Cardinality,
}
