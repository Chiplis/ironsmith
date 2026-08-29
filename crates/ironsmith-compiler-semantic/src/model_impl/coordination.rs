//! Typed coordination between effect clauses.
//!
//! Coordination is grammar, not presentation repair.  Recognition records
//! every authored connective and every omitted clause role before any member
//! is lowered.  The resulting program therefore says which effects are
//! ordered, which are alternatives, and which facts flow across a boundary
//! without consulting source text again.

use crate::model::ast::EffectAst;
use crate::model::clauses::{ClauseActionAst, ClauseObjectAst, ClauseSubjectAst};
use crate::model::provenance::SemanticProvenance;
use crate::model::symbols::SymbolReference;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CoordinationKindAst {
    Sequence,
    Conjunction,
    Disjunction,
    SharedSubject,
    SharedObject,
    Carry,
    Mixed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CoordinationOperatorAst {
    And,
    Or,
    Then,
    Comma,
    CommaThen,
    Semicolon,
    SentenceBoundary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EffectOrderingAst {
    Ordered,
    Unordered,
    Alternative,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EffectDependencyAst {
    Independent,
    DependsOnMembers(Vec<usize>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CarryKindAst {
    Actor,
    Subject,
    Action,
    Object,
    Duration,
    Reference,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CarriedFactAst {
    Actor,
    Subject(Option<ClauseSubjectAst>),
    Action(Option<ClauseActionAst>),
    Object(Option<ClauseObjectAst>),
    Duration,
    Reference(Option<SymbolReference>),
}

impl CarriedFactAst {
    pub fn kind(&self) -> CarryKindAst {
        match self {
            Self::Actor => CarryKindAst::Actor,
            Self::Subject(_) => CarryKindAst::Subject,
            Self::Action(_) => CarryKindAst::Action,
            Self::Object(_) => CarryKindAst::Object,
            Self::Duration => CarryKindAst::Duration,
            Self::Reference(_) => CarryKindAst::Reference,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CoordinationCarryAst {
    pub from_member: usize,
    pub to_member: usize,
    pub fact: CarriedFactAst,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CoordinationBoundaryAst {
    pub operator: CoordinationOperatorAst,
    pub ordering: EffectOrderingAst,
    pub dependency: EffectDependencyAst,
    pub carries: Vec<CoordinationCarryAst>,
    pub provenance: Option<SemanticProvenance>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CoordinationMemberAst {
    pub effects: Vec<EffectAst>,
    pub imports: Vec<SymbolReference>,
    pub exports: Vec<SymbolReference>,
    pub provenance: Option<SemanticProvenance>,
}

impl CoordinationMemberAst {
    pub fn new(effects: Vec<EffectAst>) -> Self {
        Self {
            effects,
            imports: Vec::new(),
            exports: Vec::new(),
            provenance: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoordinationError {
    EmptyProgram,
    BoundaryCount {
        members: usize,
        boundaries: usize,
    },
    InvalidDependency {
        member: usize,
        dependency: usize,
    },
    InvalidCarry {
        boundary: usize,
        from_member: usize,
        to_member: usize,
    },
    DuplicateCarry {
        boundary: usize,
        to_member: usize,
        kind: CarryKindAst,
    },
}

/// A compiler-owned effect program whose clause relationships have already
/// been resolved by grammar recognition.
#[derive(Debug, Clone, PartialEq)]
pub struct CoordinationAst {
    pub kind: CoordinationKindAst,
    pub members: Vec<CoordinationMemberAst>,
    pub boundaries: Vec<CoordinationBoundaryAst>,
    pub provenance: Option<SemanticProvenance>,
}

impl CoordinationAst {
    pub fn new(
        kind: CoordinationKindAst,
        members: Vec<CoordinationMemberAst>,
        boundaries: Vec<CoordinationBoundaryAst>,
        provenance: Option<SemanticProvenance>,
    ) -> Result<Self, CoordinationError> {
        if members.is_empty() {
            return Err(CoordinationError::EmptyProgram);
        }
        if boundaries.len() != members.len().saturating_sub(1) {
            return Err(CoordinationError::BoundaryCount {
                members: members.len(),
                boundaries: boundaries.len(),
            });
        }
        for (boundary_index, boundary) in boundaries.iter().enumerate() {
            let to_member = boundary_index + 1;
            if let EffectDependencyAst::DependsOnMembers(dependencies) = &boundary.dependency
                && let Some(dependency) = dependencies
                    .iter()
                    .copied()
                    .find(|dependency| *dependency >= to_member)
            {
                return Err(CoordinationError::InvalidDependency {
                    member: to_member,
                    dependency,
                });
            }
            let mut carried_kinds = Vec::new();
            for carry in &boundary.carries {
                if carry.to_member != to_member || carry.from_member >= carry.to_member {
                    return Err(CoordinationError::InvalidCarry {
                        boundary: boundary_index,
                        from_member: carry.from_member,
                        to_member: carry.to_member,
                    });
                }
                let kind = carry.fact.kind();
                if carried_kinds.contains(&kind) {
                    return Err(CoordinationError::DuplicateCarry {
                        boundary: boundary_index,
                        to_member,
                        kind,
                    });
                }
                carried_kinds.push(kind);
            }
        }
        Ok(Self {
            kind,
            members,
            boundaries,
            provenance,
        })
    }

    pub fn effects(&self) -> impl Iterator<Item = &EffectAst> {
        self.members.iter().flat_map(|member| member.effects.iter())
    }

    pub fn effects_mut(&mut self) -> impl Iterator<Item = &mut EffectAst> {
        self.members
            .iter_mut()
            .flat_map(|member| member.effects.iter_mut())
    }

    pub fn into_effects(self) -> Vec<EffectAst> {
        self.members
            .into_iter()
            .flat_map(|member| member.effects)
            .collect()
    }
}
