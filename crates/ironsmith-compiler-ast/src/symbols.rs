use ironsmith_core::tag::TagKeyWalk;

use std::collections::HashMap;

use crate::model::provenance::{ProvenanceId, SemanticProvenance};
use ironsmith_core::TagKey;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[derive(TagKeyWalk)]
pub struct SymbolId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[derive(TagKeyWalk)]
pub struct SymbolScopeId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[derive(TagKeyWalk)]
pub enum ReferenceRole {
    Source,
    Target,
    Chosen,
    Affected,
    Revealed,
    Milled,
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
#[derive(TagKeyWalk)]
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

    /// Whether a binding with this cardinality is safe to consume through a
    /// reference that requires `required`. This compares semantic ranges, not
    /// enum spellings, so `Fixed(1)` and `ExactlyOne` are interchangeable.
    pub fn satisfies(self, required: Self) -> bool {
        let (actual_min, actual_max) = self.bounds();
        let (required_min, required_max) = required.bounds();
        actual_min >= required_min
            && match (actual_max, required_max) {
                (_, None) => true,
                (Some(actual), Some(required)) => actual <= required,
                (None, Some(_)) => false,
            }
    }

    const fn bounds(self) -> (u32, Option<u32>) {
        match self {
            Self::ExactlyOne => (1, Some(1)),
            Self::ZeroOrOne => (0, Some(1)),
            Self::OneOrMore => (1, None),
            Self::Any => (0, None),
            Self::Fixed(count) => (count, Some(count)),
            Self::Range { min, max } => (min, max),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[derive(TagKeyWalk)]
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
#[derive(TagKeyWalk)]
pub enum SymbolScopeKind {
    Root,
    Document,
    /// One physical line of the card text, by its display index.
    Line { source_line: usize },
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
    /// The string reference key the grammar minted for this binding, while
    /// string keys are still the identity consumers read (item 6).
    pub key: Option<TagKey>,
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
                    key: None,
        });
        self.by_scope.entry(scope).or_default().push(id);
        Ok(id)
    }

    /// Bind a symbol the grammar minted under `key` in `scope`; one key is one
    /// symbol per scope, so a repeated mint returns the existing binding.
    pub fn bind_keyed(
        &mut self,
        scope: SymbolScopeId,
        key: TagKey,
        role: ReferenceRole,
        cardinality: Cardinality,
        domain: ObjectDomain,
    ) -> Result<SymbolId, SymbolResolutionError> {
        if let Some(existing) = self
            .by_scope
            .get(&scope)
            .and_then(|ids| ids.iter().find(|id| self.bindings[id.0 as usize].key.as_ref() == Some(&key)))
        {
            return Ok(*existing);
        }
        let id = self.bind(scope, role, cardinality, domain, None)?;
        self.bindings[id.0 as usize].key = Some(key);
        Ok(id)
    }

    /// The symbol bound under `key` in `scope` or an enclosing scope.
    /// The scope of the physical line with this display index. A line no scope
    /// was opened for was consumed by the block a preceding line opened (a
    /// level-up body, a modal mode): it shares that block's scope.
    pub fn line_scope(&self, source_line: usize) -> Option<SymbolScopeId> {
        let mut nearest: Option<(usize, SymbolScopeId)> = None;
        for scope in &self.scopes {
            let SymbolScopeKind::Line { source_line: opened } = scope.kind else {
                continue;
            };
            if opened == source_line {
                return Some(scope.id);
            }
            if opened < source_line && nearest.is_none_or(|(best, _)| opened > best) {
                nearest = Some((opened, scope.id));
            }
        }
        nearest.map(|(_, id)| id)
    }

    /// The symbol `key` was bound to, looking from `scope` outward through its
    /// ancestors. A physical line parsed more than once opens one `Line` scope
    /// per parse; those siblings are one namespace, so a key bound in any of
    /// them resolves from any other.
    pub fn symbol_for_key(&self, scope: SymbolScopeId, key: &TagKey) -> Option<SymbolId> {
        if let Some(symbol) = self.symbol_for_key_in_chain(scope, key) {
            return Some(symbol);
        }
        let kind = self.scope(scope)?.kind;
        if !matches!(kind, SymbolScopeKind::Line { .. }) {
            return None;
        }
        self.scopes
            .iter()
            .filter(|sibling| sibling.kind == kind && sibling.id != scope)
            .find_map(|sibling| self.symbol_for_key_in_chain(sibling.id, key))
    }

    fn symbol_for_key_in_chain(&self, scope: SymbolScopeId, key: &TagKey) -> Option<SymbolId> {
        let mut current = Some(scope);
        while let Some(scope) = current {
            if let Some(found) = self.by_scope.get(&scope).and_then(|ids| {
                ids.iter().copied().find(|id| self.bindings[id.0 as usize].key.as_ref() == Some(key))
            }) {
                return Some(found);
            }
            current = self.scopes.get(scope.0 as usize).and_then(|s| s.parent);
        }
        None
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

    pub fn scope_depth(&self, id: SymbolScopeId) -> Option<usize> {
        let mut depth = 0;
        let mut current = Some(id);
        while let Some(scope) = current {
            let record = self.scope(scope)?;
            current = record.parent;
            depth += 1;
        }
        Some(depth)
    }

    pub fn binding_visible_from(&self, binding: SymbolId, use_scope: SymbolScopeId) -> bool {
        let Some(binding) = self.binding(binding) else {
            return false;
        };
        self.scope_is_ancestor_of(binding.scope, use_scope)
    }

    pub fn scope_is_ancestor_of(&self, ancestor: SymbolScopeId, descendant: SymbolScopeId) -> bool {
        let mut current = Some(descendant);
        while let Some(scope) = current {
            if scope == ancestor {
                return true;
            }
            current = self.scope(scope).and_then(|record| record.parent);
        }
        false
    }

    pub fn visible_bindings(&self, scope: SymbolScopeId) -> Vec<SymbolId> {
        let mut visible = Vec::new();
        let mut current = Some(scope);
        while let Some(scope) = current {
            if let Some(bindings) = self.by_scope.get(&scope) {
                visible.extend(bindings.iter().copied());
            }
            current = self.scope(scope).and_then(|record| record.parent);
        }
        visible
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
                    .is_some_and(|required| !binding.cardinality.satisfies(required))
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

    pub fn scopes(&self) -> &[SymbolScope] {
        &self.scopes
    }

    pub fn bindings(&self) -> &[SymbolBinding] {
        &self.bindings
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[derive(TagKeyWalk)]
pub struct SymbolReference {
    pub symbol: SymbolId,
    pub role: ReferenceRole,
    pub domain: ObjectDomain,
    pub cardinality: Cardinality,
}
