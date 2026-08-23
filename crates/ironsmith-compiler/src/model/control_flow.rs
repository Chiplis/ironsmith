//! Compiler-owned control structure for nested effect programs.
//!
//! These nodes are deliberately richer than runtime scheduling primitives.
//! They preserve grammatical scope, branch-local reference environments, and
//! the semantic distinction between conditions, replacements, prevention,
//! permissions, durations, and delayed execution until lowering.

use std::collections::HashSet;

use crate::IfResultPredicate;
use crate::model::ast::{EffectAst, PredicateAst};
use crate::model::clauses::{ClauseActorAst, ClauseDurationAst, ClauseVerbAst};
use crate::model::provenance::SemanticProvenance;
use crate::model::symbols::{SymbolReference, SymbolScopeId, SymbolScopeKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ControlFlowSemanticAst {
    ControlFlow,
    Replacement,
    Prevention,
    Permission,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConditionPositionAst {
    Precondition,
    ResultCondition,
    InterveningCondition,
    Postcondition,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ControlPredicateAst {
    State(PredicateAst),
    Result(IfResultPredicate),
    Constant(bool),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ControlConditionAst {
    pub position: ConditionPositionAst,
    pub predicate: ControlPredicateAst,
    pub negated_surface: bool,
    pub provenance: Option<SemanticProvenance>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReplacementKindAst {
    Instead,
    As,
    Skip,
    Redirect,
    Modify,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReplacedEventAst {
    PriorEffect,
    Damage,
    Draw,
    ZoneChange,
    EnterBattlefield,
    CostPayment,
    TurnOrStep,
    ManaProduction,
    Other,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReplacementRelationshipAst {
    pub kind: ReplacementKindAst,
    pub event: ReplacedEventAst,
    pub condition: Option<ControlConditionAst>,
    pub original_program: Option<usize>,
    pub replacement_program: usize,
    pub affected_reference: Option<SymbolReference>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PreventionRelationshipAst {
    pub event: ReplacedEventAst,
    pub condition: Option<ControlConditionAst>,
    pub prevention_program: usize,
    pub protected_reference: Option<SymbolReference>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PermissionRelationshipAst {
    pub actor: ClauseActorAst,
    pub action: ClauseVerbAst,
    pub duration: Option<CompilerDurationAst>,
    pub program: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CompilerDurationAst {
    Clause(ClauseDurationAst),
    ThisTurn,
    UntilEndOfTurn,
    UntilEndOfCombat,
    UntilNextTurn,
    UntilCondition(PredicateAst),
    ForAsLongAs(PredicateAst),
    Permanent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DelayedScheduleAst {
    NextEndStep,
    NextCleanupStep,
    NextUntapStep,
    NextUpkeep,
    NextDrawStep,
    NextMainPhase,
    EndOfCombat,
    Event,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NestedProgramKindAst {
    Consequence,
    Alternative,
    Replacement,
    Prevention,
    Permission,
    Delayed,
    Reflexive,
    NestedAbility,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ControlFlowScopeAst {
    pub id: SymbolScopeId,
    pub parent: Option<SymbolScopeId>,
    pub kind: SymbolScopeKind,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ControlFlowReferenceEnvironmentAst {
    pub root: SymbolScopeId,
    pub scopes: Vec<ControlFlowScopeAst>,
}

impl ControlFlowReferenceEnvironmentAst {
    fn for_programs(programs: &mut [NestedProgramAst]) -> Self {
        let root = SymbolScopeId(0);
        let mut scopes = vec![ControlFlowScopeAst {
            id: root,
            parent: None,
            kind: SymbolScopeKind::Root,
        }];
        for (index, program) in programs.iter_mut().enumerate() {
            let id = SymbolScopeId(
                u32::try_from(index + 1).expect("nested control-flow scope identifier overflow"),
            );
            program.scope = id;
            program.parent_scope = root;
            scopes.push(ControlFlowScopeAst {
                id,
                parent: Some(root),
                kind: match program.kind {
                    NestedProgramKindAst::NestedAbility => SymbolScopeKind::NestedAbility,
                    _ => SymbolScopeKind::Branch,
                },
            });
        }
        Self { root, scopes }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NestedProgramAst {
    pub scope: SymbolScopeId,
    pub parent_scope: SymbolScopeId,
    pub kind: NestedProgramKindAst,
    pub effects: Vec<EffectAst>,
    pub imports: Vec<SymbolReference>,
    pub exports: Vec<SymbolReference>,
    pub provenance: Option<SemanticProvenance>,
}

impl NestedProgramAst {
    pub fn new(kind: NestedProgramKindAst, effects: Vec<EffectAst>) -> Self {
        Self {
            scope: SymbolScopeId(0),
            parent_scope: SymbolScopeId(0),
            kind,
            effects,
            imports: Vec::new(),
            exports: Vec::new(),
            provenance: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ControlFlowNodeAst {
    Condition {
        condition: ControlConditionAst,
        consequence_program: usize,
        alternative_program: Option<usize>,
        reflexive: bool,
    },
    Replacement(ReplacementRelationshipAst),
    Prevention(PreventionRelationshipAst),
    Permission(PermissionRelationshipAst),
    Duration {
        duration: CompilerDurationAst,
        program: usize,
    },
    Delayed {
        schedule: DelayedScheduleAst,
        duration: Option<CompilerDurationAst>,
        program: usize,
        one_shot: bool,
        reflexive: bool,
        watched_references: Vec<SymbolReference>,
    },
    NestedAbility {
        program: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControlFlowError {
    EmptyProgram,
    ProgramOutOfRange {
        program: usize,
        count: usize,
    },
    DuplicateScope(SymbolScopeId),
    InvalidParentScope {
        scope: SymbolScopeId,
        parent: SymbolScopeId,
    },
    SemanticMismatch {
        semantic: ControlFlowSemanticAst,
        node: &'static str,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompilerControlFlowAst {
    pub semantic: ControlFlowSemanticAst,
    pub node: ControlFlowNodeAst,
    pub programs: Vec<NestedProgramAst>,
    pub references: ControlFlowReferenceEnvironmentAst,
    pub provenance: Option<SemanticProvenance>,
}

impl CompilerControlFlowAst {
    pub fn new(
        semantic: ControlFlowSemanticAst,
        node: ControlFlowNodeAst,
        mut programs: Vec<NestedProgramAst>,
        provenance: Option<SemanticProvenance>,
    ) -> Result<Self, ControlFlowError> {
        if programs.is_empty() {
            return Err(ControlFlowError::EmptyProgram);
        }
        validate_semantic(semantic, &node)?;
        validate_program_indices(&node, programs.len())?;
        let references = ControlFlowReferenceEnvironmentAst::for_programs(&mut programs);
        validate_scopes(&references, &programs)?;
        Ok(Self {
            semantic,
            node,
            programs,
            references,
            provenance,
        })
    }

    pub fn program(&self, index: usize) -> Option<&NestedProgramAst> {
        self.programs.get(index)
    }

    pub fn program_mut(&mut self, index: usize) -> Option<&mut NestedProgramAst> {
        self.programs.get_mut(index)
    }
}

fn validate_semantic(
    semantic: ControlFlowSemanticAst,
    node: &ControlFlowNodeAst,
) -> Result<(), ControlFlowError> {
    let matches = matches!(
        (semantic, node),
        (
            ControlFlowSemanticAst::ControlFlow,
            ControlFlowNodeAst::Condition { .. }
                | ControlFlowNodeAst::Duration { .. }
                | ControlFlowNodeAst::Delayed { .. }
                | ControlFlowNodeAst::NestedAbility { .. }
        ) | (
            ControlFlowSemanticAst::Replacement,
            ControlFlowNodeAst::Replacement(_)
        ) | (
            ControlFlowSemanticAst::Prevention,
            ControlFlowNodeAst::Prevention(_)
        ) | (
            ControlFlowSemanticAst::Permission,
            ControlFlowNodeAst::Permission(_)
        )
    );
    if matches {
        Ok(())
    } else {
        Err(ControlFlowError::SemanticMismatch {
            semantic,
            node: node_name(node),
        })
    }
}

fn validate_program_indices(
    node: &ControlFlowNodeAst,
    count: usize,
) -> Result<(), ControlFlowError> {
    let mut indices = Vec::new();
    match node {
        ControlFlowNodeAst::Condition {
            consequence_program,
            alternative_program,
            ..
        } => {
            indices.push(*consequence_program);
            indices.extend(*alternative_program);
        }
        ControlFlowNodeAst::Replacement(replacement) => {
            indices.extend(replacement.original_program);
            indices.push(replacement.replacement_program);
        }
        ControlFlowNodeAst::Prevention(prevention) => indices.push(prevention.prevention_program),
        ControlFlowNodeAst::Permission(permission) => indices.push(permission.program),
        ControlFlowNodeAst::Duration { program, .. }
        | ControlFlowNodeAst::Delayed { program, .. }
        | ControlFlowNodeAst::NestedAbility { program } => indices.push(*program),
    }
    if let Some(program) = indices.into_iter().find(|program| *program >= count) {
        Err(ControlFlowError::ProgramOutOfRange { program, count })
    } else {
        Ok(())
    }
}

fn validate_scopes(
    references: &ControlFlowReferenceEnvironmentAst,
    programs: &[NestedProgramAst],
) -> Result<(), ControlFlowError> {
    let mut scopes = HashSet::new();
    for scope in &references.scopes {
        if !scopes.insert(scope.id) {
            return Err(ControlFlowError::DuplicateScope(scope.id));
        }
    }
    for program in programs {
        if program.parent_scope != references.root || !scopes.contains(&program.scope) {
            return Err(ControlFlowError::InvalidParentScope {
                scope: program.scope,
                parent: program.parent_scope,
            });
        }
    }
    Ok(())
}

fn node_name(node: &ControlFlowNodeAst) -> &'static str {
    match node {
        ControlFlowNodeAst::Condition { .. } => "condition",
        ControlFlowNodeAst::Replacement(_) => "replacement",
        ControlFlowNodeAst::Prevention(_) => "prevention",
        ControlFlowNodeAst::Permission(_) => "permission",
        ControlFlowNodeAst::Duration { .. } => "duration",
        ControlFlowNodeAst::Delayed { .. } => "delayed",
        ControlFlowNodeAst::NestedAbility { .. } => "nested-ability",
    }
}
