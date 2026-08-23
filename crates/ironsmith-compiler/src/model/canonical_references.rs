//! Lexical reference resolution over the canonical compiler AST.
//!
//! Runtime tags are serialization details. This pass validates compiler
//! symbols by identity, role, domain, cardinality, and lexical accessibility,
//! and gives every control-flow edge an explicit join rule before lowering.

use std::collections::{BTreeMap, BTreeSet};
use std::convert::Infallible;
use std::ops::ControlFlow;

use crate::model::ast::EffectAst;
use crate::model::compiler_semantic::{
    LineAst, ParsedCardItem, ParsedLevelAbilityItemAst, ParsedModalAst,
};
use crate::model::control_flow::{CompilerControlFlowAst, ControlFlowNodeAst};
use crate::model::coordination::CoordinationKindAst;
use crate::model::symbols::{
    Cardinality, ObjectDomain, ReferenceRole, SymbolId, SymbolReference, SymbolScopeId, SymbolTable,
};
use crate::model::visit::{SemanticVisitor, for_each_nested_effects, visit_effect_node};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ReferenceJoinKindAst {
    Conditional,
    Modal,
    Optional,
    Iteration,
    Delayed,
    Replacement,
    Disjunction,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceJoinAst {
    pub kind: ReferenceJoinKindAst,
    pub incoming: Vec<SymbolId>,
    pub branch_exports: Vec<Vec<SymbolId>>,
    pub outgoing: Vec<SymbolId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CanonicalReferenceDiagnostic {
    MissingBinding {
        reference: SymbolReference,
    },
    Inaccessible {
        reference: SymbolReference,
        use_scope: SymbolScopeId,
        declaration_scope: SymbolScopeId,
    },
    WrongRole {
        reference: SymbolReference,
        declared: ReferenceRole,
    },
    WrongDomain {
        reference: SymbolReference,
        declared: ObjectDomain,
    },
    WrongCardinality {
        reference: SymbolReference,
        declared: Cardinality,
    },
    Ambiguous {
        role: ReferenceRole,
        domain: ObjectDomain,
        candidates: Vec<SymbolId>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedReferenceAst {
    pub reference: SymbolReference,
    pub use_scope: SymbolScopeId,
    pub binding: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CanonicalReferenceResolutionAst {
    pub resolved: Vec<ResolvedReferenceAst>,
    pub joins: Vec<ReferenceJoinAst>,
    pub diagnostics: Vec<CanonicalReferenceDiagnostic>,
}

impl CanonicalReferenceResolutionAst {
    pub fn append(&mut self, mut other: Self) {
        self.resolved.append(&mut other.resolved);
        self.joins.append(&mut other.joins);
        self.diagnostics.append(&mut other.diagnostics);
    }
}

#[derive(Debug, Clone)]
struct LexicalReferenceEnv {
    scope: SymbolScopeId,
    visible: BTreeSet<SymbolId>,
}

impl LexicalReferenceEnv {
    fn at_scope(symbols: &SymbolTable, scope: SymbolScopeId) -> Self {
        Self {
            scope,
            visible: symbols.visible_bindings(scope).into_iter().collect(),
        }
    }
}

#[derive(Default)]
struct NodeReferenceCollector {
    bindings: Vec<SymbolReference>,
    uses: Vec<SymbolReference>,
}

impl SemanticVisitor for NodeReferenceCollector {
    type Break = Infallible;

    fn visit_reference(&mut self, reference: &SymbolReference) -> ControlFlow<Self::Break> {
        self.uses.push(*reference);
        ControlFlow::Continue(())
    }

    fn visit_reference_binding(&mut self, reference: &SymbolReference) -> ControlFlow<Self::Break> {
        self.bindings.push(*reference);
        ControlFlow::Continue(())
    }
}

struct CanonicalReferenceResolver<'a> {
    symbols: &'a SymbolTable,
    report: CanonicalReferenceResolutionAst,
}

impl<'a> CanonicalReferenceResolver<'a> {
    fn new(symbols: &'a SymbolTable) -> Self {
        Self {
            symbols,
            report: CanonicalReferenceResolutionAst::default(),
        }
    }

    fn resolve_sequence(
        &mut self,
        effects: &[EffectAst],
        mut env: LexicalReferenceEnv,
    ) -> LexicalReferenceEnv {
        for effect in effects {
            env = self.resolve_effect(effect, env);
        }
        env
    }

    fn resolve_effect(
        &mut self,
        effect: &EffectAst,
        mut env: LexicalReferenceEnv,
    ) -> LexicalReferenceEnv {
        let mut references = NodeReferenceCollector::default();
        match visit_effect_node(&mut references, effect) {
            ControlFlow::Continue(()) => {}
            ControlFlow::Break(never) => match never {},
        }
        for binding in references.bindings {
            self.resolve_binding(binding, &mut env);
        }
        for reference in references.uses {
            self.resolve_use(reference, &env);
        }

        match effect {
            EffectAst::Coordination(coordination)
                if coordination.kind == CoordinationKindAst::Disjunction =>
            {
                let branches = coordination
                    .members
                    .iter()
                    .map(|member| {
                        self.resolve_program(
                            &member.effects,
                            &member.imports,
                            &member.exports,
                            env.clone(),
                        )
                    })
                    .collect();
                self.join(ReferenceJoinKindAst::Disjunction, env, branches)
            }
            EffectAst::Coordination(coordination) => {
                for member in &coordination.members {
                    env = self.resolve_program(
                        &member.effects,
                        &member.imports,
                        &member.exports,
                        env,
                    );
                }
                env
            }
            EffectAst::ControlFlow(control) => match &control.node {
                ControlFlowNodeAst::Condition {
                    consequence_program,
                    alternative_program,
                    ..
                } => {
                    let mut branches = Vec::new();
                    if control.program(*consequence_program).is_some() {
                        branches.push(self.resolve_control_program(
                            control,
                            *consequence_program,
                            env.clone(),
                        ));
                    }
                    if let Some(alternative) = alternative_program
                        && control.program(*alternative).is_some()
                    {
                        branches.push(self.resolve_control_program(
                            control,
                            *alternative,
                            env.clone(),
                        ));
                    } else {
                        branches.push(env.clone());
                    }
                    self.join(ReferenceJoinKindAst::Conditional, env, branches)
                }
                ControlFlowNodeAst::Replacement(replacement) => {
                    let mut branches = Vec::new();
                    if let Some(original) = replacement.original_program
                        && control.program(original).is_some()
                    {
                        branches.push(self.resolve_control_program(control, original, env.clone()));
                    } else {
                        branches.push(env.clone());
                    }
                    if control.program(replacement.replacement_program).is_some() {
                        branches.push(self.resolve_control_program(
                            control,
                            replacement.replacement_program,
                            env.clone(),
                        ));
                    }
                    self.join(ReferenceJoinKindAst::Replacement, env, branches)
                }
                ControlFlowNodeAst::Prevention(prevention) => {
                    let branch = control
                        .program(prevention.prevention_program)
                        .map(|_| {
                            self.resolve_control_program(
                                control,
                                prevention.prevention_program,
                                env.clone(),
                            )
                        })
                        .into_iter()
                        .collect();
                    self.join(ReferenceJoinKindAst::Replacement, env, branch)
                }
                ControlFlowNodeAst::Duration { program, .. } => control
                    .program(*program)
                    .map(|_| self.resolve_control_program(control, *program, env.clone()))
                    .unwrap_or(env),
                ControlFlowNodeAst::Permission(permission) => {
                    let branches = control
                        .program(permission.program)
                        .map(|_| {
                            self.resolve_control_program(control, permission.program, env.clone())
                        })
                        .into_iter()
                        .collect();
                    self.join(ReferenceJoinKindAst::Delayed, env, branches)
                }
                ControlFlowNodeAst::Delayed { program, .. }
                | ControlFlowNodeAst::NestedAbility { program } => {
                    let branches = control
                        .program(*program)
                        .map(|_| self.resolve_control_program(control, *program, env.clone()))
                        .into_iter()
                        .collect();
                    self.join(ReferenceJoinKindAst::Delayed, env, branches)
                }
            },
            EffectAst::Conditional {
                if_true, if_false, ..
            }
            | EffectAst::SelfReplacement {
                if_true, if_false, ..
            } => {
                let true_env = self.resolve_sequence(if_true, env.clone());
                let false_env = if if_false.is_empty() {
                    env.clone()
                } else {
                    self.resolve_sequence(if_false, env.clone())
                };
                self.join(
                    ReferenceJoinKindAst::Conditional,
                    env,
                    vec![true_env, false_env],
                )
            }
            EffectAst::ChooseOneOf { modes } | EffectAst::VillainousChoice { modes, .. } => {
                let branches = modes
                    .iter()
                    .map(|mode| self.resolve_sequence(&mode.effects, env.clone()))
                    .collect();
                self.join(ReferenceJoinKindAst::Modal, env, branches)
            }
            EffectAst::May { effects }
            | EffectAst::MayByPlayer { effects, .. }
            | EffectAst::AnyPlayerMay { effects, .. }
            | EffectAst::TrailingIf { effects, .. }
            | EffectAst::TrailingUnless { effects, .. } => {
                let branch = self.resolve_sequence(effects, env.clone());
                self.join(
                    ReferenceJoinKindAst::Optional,
                    env.clone(),
                    vec![env, branch],
                )
            }
            EffectAst::RepeatEffects { effects, .. }
            | EffectAst::RepeatProcess { effects, .. }
            | EffectAst::ForEachOpponent { effects }
            | EffectAst::ForEachPlayersFiltered { effects, .. }
            | EffectAst::ForEachPlayer { effects }
            | EffectAst::ForEachTargetPlayers { effects, .. }
            | EffectAst::ForEachObject { effects, .. }
            | EffectAst::ForEachTagged { effects, .. }
            | EffectAst::ForEachTaggedWithControllerAtLastBlockedBy { effects, .. }
            | EffectAst::ForEachOpponentDoesNot { effects, .. }
            | EffectAst::ForEachPlayerDoesNot { effects, .. }
            | EffectAst::ForEachOpponentDid { effects, .. }
            | EffectAst::ForEachPlayerDid { effects, .. }
            | EffectAst::ForEachTaggedPlayer { effects, .. } => {
                let body = self.resolve_sequence(effects, env.clone());
                self.join(
                    ReferenceJoinKindAst::Iteration,
                    env.clone(),
                    vec![env, body],
                )
            }
            EffectAst::DelayedUntilNextEndStep { effects, .. }
            | EffectAst::DelayedUntilNextCleanupStep { effects, .. }
            | EffectAst::DelayedUntilNextUntapStep { effects, .. }
            | EffectAst::DelayedUntilNextUpkeep { effects, .. }
            | EffectAst::DelayedUntilNextDrawStep { effects, .. }
            | EffectAst::DelayedUntilNextMainPhase { effects, .. }
            | EffectAst::DelayedUntilNextFirstMainPhase { effects, .. }
            | EffectAst::DelayedUntilEndStepOfExtraTurn { effects, .. }
            | EffectAst::DelayedUntilEndOfCombat { effects }
            | EffectAst::DelayedTriggerThisTurn { effects, .. }
            | EffectAst::DelayedTriggerForDuration { effects, .. }
            | EffectAst::DelayedWhenLastObjectDiesThisTurn { effects, .. }
            | EffectAst::DelayedWhenLastObjectLeavesBattlefield { effects, .. } => {
                let branch = self.resolve_sequence(effects, env.clone());
                self.join(ReferenceJoinKindAst::Delayed, env, vec![branch])
            }
            _ => {
                for_each_nested_effects(effect, true, |children| {
                    env = self.resolve_sequence(children, env.clone());
                });
                env
            }
        }
    }

    fn resolve_control_program(
        &mut self,
        control: &CompilerControlFlowAst,
        index: usize,
        env: LexicalReferenceEnv,
    ) -> LexicalReferenceEnv {
        let Some(program) = control.program(index) else {
            return env;
        };
        self.resolve_program(&program.effects, &program.imports, &program.exports, env)
    }

    fn resolve_program(
        &mut self,
        effects: &[EffectAst],
        imports: &[SymbolReference],
        exports: &[SymbolReference],
        mut env: LexicalReferenceEnv,
    ) -> LexicalReferenceEnv {
        for reference in imports {
            self.resolve_use(*reference, &env);
        }
        env = self.resolve_sequence(effects, env);
        for reference in exports {
            self.resolve_binding(*reference, &mut env);
        }
        env
    }

    fn resolve_binding(&mut self, reference: SymbolReference, env: &mut LexicalReferenceEnv) {
        let Some(binding) = self.validate_metadata(reference) else {
            return;
        };
        if self.symbols.scope_is_ancestor_of(env.scope, binding.scope) {
            env.scope = binding.scope;
            env.visible
                .extend(self.symbols.visible_bindings(binding.scope));
        } else if !self
            .symbols
            .binding_visible_from(reference.symbol, env.scope)
        {
            self.report
                .diagnostics
                .push(CanonicalReferenceDiagnostic::Inaccessible {
                    reference,
                    use_scope: env.scope,
                    declaration_scope: binding.scope,
                });
            return;
        }
        env.visible.insert(reference.symbol);
        self.report.resolved.push(ResolvedReferenceAst {
            reference,
            use_scope: env.scope,
            binding: true,
        });
    }

    fn resolve_use(&mut self, reference: SymbolReference, env: &LexicalReferenceEnv) {
        let Some(binding) = self.validate_metadata(reference) else {
            return;
        };
        if !env.visible.contains(&reference.symbol)
            && !self
                .symbols
                .binding_visible_from(reference.symbol, env.scope)
        {
            self.report
                .diagnostics
                .push(CanonicalReferenceDiagnostic::Inaccessible {
                    reference,
                    use_scope: env.scope,
                    declaration_scope: binding.scope,
                });
            return;
        }
        self.report.resolved.push(ResolvedReferenceAst {
            reference,
            use_scope: env.scope,
            binding: false,
        });
    }

    fn validate_metadata(
        &mut self,
        reference: SymbolReference,
    ) -> Option<&'a crate::model::symbols::SymbolBinding> {
        let Some(binding) = self.symbols.binding(reference.symbol) else {
            self.report
                .diagnostics
                .push(CanonicalReferenceDiagnostic::MissingBinding { reference });
            return None;
        };
        if binding.role != reference.role {
            self.report
                .diagnostics
                .push(CanonicalReferenceDiagnostic::WrongRole {
                    reference,
                    declared: binding.role,
                });
        }
        if binding.domain != reference.domain {
            self.report
                .diagnostics
                .push(CanonicalReferenceDiagnostic::WrongDomain {
                    reference,
                    declared: binding.domain,
                });
        }
        if !binding.cardinality.satisfies(reference.cardinality) {
            self.report
                .diagnostics
                .push(CanonicalReferenceDiagnostic::WrongCardinality {
                    reference,
                    declared: binding.cardinality,
                });
        }
        Some(binding)
    }

    fn join(
        &mut self,
        kind: ReferenceJoinKindAst,
        incoming: LexicalReferenceEnv,
        branches: Vec<LexicalReferenceEnv>,
    ) -> LexicalReferenceEnv {
        if branches.is_empty() {
            return incoming;
        }
        let mut outgoing = branches[0].visible.clone();
        for branch in &branches[1..] {
            outgoing = outgoing.intersection(&branch.visible).copied().collect();
        }
        outgoing.extend(incoming.visible.iter().copied());

        let branch_exports: Vec<Vec<SymbolId>> = branches
            .iter()
            .map(|branch| {
                branch
                    .visible
                    .difference(&incoming.visible)
                    .copied()
                    .collect()
            })
            .collect();
        self.record_ambiguous_branch_exports(&branch_exports);
        self.report.joins.push(ReferenceJoinAst {
            kind,
            incoming: incoming.visible.iter().copied().collect(),
            branch_exports,
            outgoing: outgoing.iter().copied().collect(),
        });
        LexicalReferenceEnv {
            scope: incoming.scope,
            visible: outgoing,
        }
    }

    fn record_ambiguous_branch_exports(&mut self, branch_exports: &[Vec<SymbolId>]) {
        let mut candidates: BTreeMap<(ReferenceRole, ObjectDomain), BTreeSet<SymbolId>> =
            BTreeMap::new();
        for symbol in branch_exports.iter().flatten() {
            if let Some(binding) = self.symbols.binding(*symbol) {
                candidates
                    .entry((binding.role, binding.domain))
                    .or_default()
                    .insert(*symbol);
            }
        }
        for ((role, domain), symbols) in candidates {
            if symbols.len() > 1 {
                self.report
                    .diagnostics
                    .push(CanonicalReferenceDiagnostic::Ambiguous {
                        role,
                        domain,
                        candidates: symbols.into_iter().collect(),
                    });
            }
        }
    }
}

pub fn resolve_effect_references(
    effects: &[EffectAst],
    symbols: &SymbolTable,
    scope: SymbolScopeId,
) -> CanonicalReferenceResolutionAst {
    let mut resolver = CanonicalReferenceResolver::new(symbols);
    resolver.resolve_sequence(effects, LexicalReferenceEnv::at_scope(symbols, scope));
    resolver.report
}

pub fn resolve_parsed_items_references(
    items: &[ParsedCardItem],
    symbols: &SymbolTable,
) -> CanonicalReferenceResolutionAst {
    let mut report = CanonicalReferenceResolutionAst::default();
    for item in items {
        resolve_item(item, symbols, &mut report);
    }
    report
}

fn resolve_item(
    item: &ParsedCardItem,
    symbols: &SymbolTable,
    report: &mut CanonicalReferenceResolutionAst,
) {
    match item {
        ParsedCardItem::Line(line) => {
            let mut env = LexicalReferenceEnv::at_scope(symbols, symbols.root_scope());
            let mut resolver = CanonicalReferenceResolver::new(symbols);
            for chunk in &line.chunks {
                env = resolve_line_chunk(chunk, &mut resolver, env);
            }
            if let Some(ability) = &line.semantic_facts.triggered_ability.compiler_ability {
                let trigger_scope = symbols
                    .binding(ability.event.bindings.triggering_event.symbol)
                    .map(|binding| binding.scope)
                    .unwrap_or(symbols.root_scope());
                let mut trigger_env = LexicalReferenceEnv::at_scope(symbols, trigger_scope);
                resolver.resolve_binding(ability.event.bindings.triggering_event, &mut trigger_env);
                if let Some(object) = ability.event.bindings.triggering_object {
                    resolver.resolve_binding(object, &mut trigger_env);
                }
                let wrapper = EffectAst::ControlFlow(Box::new(ability.program.clone()));
                resolver.resolve_effect(&wrapper, trigger_env);
            }
            report.append(resolver.report);
        }
        ParsedCardItem::Modal(modal) => report.append(resolve_modal(modal, symbols)),
        ParsedCardItem::LevelAbility(level) => {
            for item in &level.items {
                if let ParsedLevelAbilityItemAst::ActivatedAbility(activated) = item {
                    let mut resolver = CanonicalReferenceResolver::new(symbols);
                    resolve_line_chunk(
                        &activated.chunk,
                        &mut resolver,
                        LexicalReferenceEnv::at_scope(symbols, symbols.root_scope()),
                    );
                    report.append(resolver.report);
                }
            }
        }
    }
}

fn resolve_line_chunk(
    chunk: &LineAst,
    resolver: &mut CanonicalReferenceResolver<'_>,
    mut env: LexicalReferenceEnv,
) -> LexicalReferenceEnv {
    match chunk {
        LineAst::Multiple(chunks) => {
            for chunk in chunks {
                env = resolve_line_chunk(chunk, resolver, env);
            }
            env
        }
        LineAst::Ability(ability) => ability
            .effects_ast
            .as_deref()
            .map(|effects| resolver.resolve_sequence(effects, env.clone()))
            .unwrap_or(env),
        LineAst::Triggered { effects, .. }
        | LineAst::Statement { effects }
        | LineAst::AdditionalCost { effects }
        | LineAst::GiftKeyword { effects, .. }
        | LineAst::OptionalCostWithCastTrigger { effects, .. } => {
            resolver.resolve_sequence(effects, env)
        }
        LineAst::AdditionalCostChoice { options } => {
            let branches = options
                .iter()
                .map(|option| resolver.resolve_sequence(&option.effects, env.clone()))
                .collect();
            resolver.join(ReferenceJoinKindAst::Modal, env, branches)
        }
        LineAst::Abilities(_)
        | LineAst::StaticAbility(_)
        | LineAst::StaticAbilities(_)
        | LineAst::OptionalCost(_)
        | LineAst::AlternativeCastingMethod(_) => env,
    }
}

fn resolve_modal(modal: &ParsedModalAst, symbols: &SymbolTable) -> CanonicalReferenceResolutionAst {
    let mut resolver = CanonicalReferenceResolver::new(symbols);
    let mut env = LexicalReferenceEnv::at_scope(symbols, symbols.root_scope());
    env = resolver.resolve_sequence(&modal.header.prefix_effects_ast, env);
    env = resolver.resolve_sequence(&modal.header.common_prefix_effects_ast, env);
    let branches = modal
        .modes
        .iter()
        .map(|mode| resolver.resolve_sequence(&mode.effects_ast, env.clone()))
        .collect();
    env = resolver.join(ReferenceJoinKindAst::Modal, env, branches);
    resolver.resolve_sequence(&modal.header.common_suffix_effects_ast, env);
    resolver.report
}
