use std::ops::ControlFlow;

use crate::cost::TotalCost;
use crate::effect::Value;
use crate::model::ast::{EffectAst, PredicateAst, SubjectVerbActionAst};
use crate::model::clauses::{
    ClauseActorAst, ClauseConditionAst, ClauseDurationAst, ClauseObjectAst, ClausePredicateAst,
    ClauseSubjectAst,
};
use crate::model::control_flow::{
    CompilerControlFlowAst, CompilerDurationAst, ControlFlowNodeAst, ControlPredicateAst,
};
use crate::model::coordination::CarriedFactAst;
use crate::model::costs::CompilerTotalCost;
use crate::model::document_program::{CompilerDocumentProgramAst, CompilerStatementEdgeKindAst};
use crate::model::object_action_clauses::CompilerObjectOperandAst;
use crate::model::resource_choice_clauses::{
    CompilerChoiceDomainAst, CompilerIterationAst, CompilerIterationSourceAst, CompilerVoteAst,
};
use crate::model::selections::{
    CompilerFilterAst, CompilerSelectionAst, CompilerValueAst, SelectionDomainAst,
};
use crate::model::symbols::SymbolReference;
use crate::target::ObjectFilter;

/// Runtime result-producing leaf at the end of a presentation-only AST
/// wrapper chain.
///
/// These producers expose typed outcomes whose identity must be attached to
/// the leaf runtime effect rather than to an enclosing sequence. Semantic
/// wrappers such as `May` deliberately stop this query because their own
/// outcome, not the nested action's outcome, controls a result follow-up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalResultProducer {
    Clash,
    FlipCoin,
}

pub fn terminal_result_producer(effect: &EffectAst) -> Option<TerminalResultProducer> {
    match effect {
        EffectAst::SubjectVerb(subject_verb) => match &subject_verb.action {
            SubjectVerbActionAst::Clash { .. } => Some(TerminalResultProducer::Clash),
            SubjectVerbActionAst::FlipCoin | SubjectVerbActionAst::FlipCoinFaceOnly => {
                Some(TerminalResultProducer::FlipCoin)
            }
            _ => None,
        },
        EffectAst::Coordination(coordination) => coordination
            .members
            .last()
            .and_then(|member| member.effects.last())
            .and_then(terminal_result_producer),
        EffectAst::ControlFlow(control) => control
            .programs
            .last()
            .and_then(|program| program.effects.last())
            .and_then(terminal_result_producer),
        EffectAst::Iteration(iteration) => iteration.body.last().and_then(terminal_result_producer),
        EffectAst::DocumentProgram(program) => program
            .statements
            .last()
            .and_then(|statement| statement.effects.last())
            .and_then(terminal_result_producer),
        EffectAst::Sequence { effects }
        | EffectAst::CommaThen { effects }
        | EffectAst::SourceSentence { effects, .. }
        | EffectAst::Coordinated { effects, .. }
        | EffectAst::ResultBranchLabel { effects, .. } => {
            effects.last().and_then(terminal_result_producer)
        }
        _ => None,
    }
}

// Keep the list of wrapper variants with `effects: Vec<EffectAst>` in one place.
// This avoids drift between immutable/mutable/fallible traversal helpers.
macro_rules! nested_effects_variants {
    ($effects:ident) => {
        EffectAst::Sequence { effects: $effects }
            | EffectAst::CommaThen { effects: $effects }
            | EffectAst::PlaySubgame {
                nonwinner_effects: $effects,
            }
            | EffectAst::SourceSentence {
                effects: $effects,
                ..
            }
            | EffectAst::Coordinated {
                effects: $effects,
                ..
            }
            | EffectAst::ResultBranchLabel {
                effects: $effects,
                ..
            }
            | EffectAst::UnlessPays {
                effects: $effects,
                ..
            }
            | EffectAst::TrailingUnless {
                effects: $effects,
                ..
            }
            | EffectAst::TrailingIf {
                effects: $effects,
                ..
            }
            | EffectAst::May { effects: $effects }
            | EffectAst::MayByPlayer {
                effects: $effects,
                ..
            }
            | EffectAst::AnyPlayerMay {
                effects: $effects,
                ..
            }
            | EffectAst::ResolvedIfResult {
                effects: $effects,
                ..
            }
            | EffectAst::ResolvedWhenResult {
                effects: $effects,
                ..
            }
            | EffectAst::IfResult {
                effects: $effects,
                ..
            }
            | EffectAst::WhenResult {
                effects: $effects,
                ..
            }
            | EffectAst::ForEachOpponent { effects: $effects }
            | EffectAst::ForEachPlayersFiltered {
                effects: $effects,
                ..
            }
            | EffectAst::ForEachPlayer { effects: $effects }
            | EffectAst::ForEachTargetPlayers {
                effects: $effects,
                ..
            }
            | EffectAst::ForEachObject {
                effects: $effects,
                ..
            }
            | EffectAst::ForEachTagged {
                effects: $effects,
                ..
            }
            | EffectAst::ForEachTaggedWithControllerAtLastBlockedBy {
                effects: $effects,
                ..
            }
            | EffectAst::ForEachOpponentDoesNot {
                effects: $effects,
                ..
            }
            | EffectAst::ForEachPlayerDoesNot {
                effects: $effects,
                ..
            }
            | EffectAst::ForEachOpponentDid {
                effects: $effects,
                ..
            }
            | EffectAst::ForEachPlayerDid {
                effects: $effects,
                ..
            }
            | EffectAst::ForEachTaggedPlayer {
                effects: $effects,
                ..
            }
            | EffectAst::RepeatProcess {
                effects: $effects,
                ..
            }
            | EffectAst::RepeatEffects {
                effects: $effects,
                ..
            }
            | EffectAst::BidLife {
                winner_effects: $effects,
                ..
            }
            | EffectAst::DelayedUntilNextEndStep {
                effects: $effects,
                ..
            }
            | EffectAst::DelayedUntilNextCleanupStep {
                effects: $effects,
                ..
            }
            | EffectAst::DelayedUntilNextUntapStep {
                effects: $effects,
                ..
            }
            | EffectAst::DelayedUntilNextUpkeep {
                effects: $effects,
                ..
            }
            | EffectAst::DelayedUntilNextDrawStep {
                effects: $effects,
                ..
            }
            | EffectAst::DelayedUntilNextMainPhase {
                effects: $effects,
                ..
            }
            | EffectAst::DelayedUntilNextFirstMainPhase {
                effects: $effects,
                ..
            }
            | EffectAst::DelayedUntilEndStepOfExtraTurn {
                effects: $effects,
                ..
            }
            | EffectAst::DelayedUntilEndOfCombat { effects: $effects }
            | EffectAst::DelayedTriggerThisTurn {
                effects: $effects,
                ..
            }
            | EffectAst::DelayedTriggerForDuration {
                effects: $effects,
                ..
            }
            | EffectAst::DelayedWhenLastObjectDiesThisTurn {
                effects: $effects,
                ..
            }
            | EffectAst::DelayedWhenLastObjectLeavesBattlefield {
                effects: $effects,
                ..
            }
            | EffectAst::VoteOption {
                effects: $effects,
                ..
            }
            | EffectAst::ManaRestricted {
                effects: $effects,
                ..
            }
    };
}

pub fn assert_effect_ast_variant_coverage(effect: &EffectAst) {
    match effect {
        EffectAst::Coordination(_) => {}
        EffectAst::ControlFlow(_) => {}
        EffectAst::Iteration(_) => {}
        EffectAst::Vote(_) => {}
        EffectAst::DocumentProgram(_) => {}
        EffectAst::SubjectVerb(_) => {}
        EffectAst::SolveCase => {}
        EffectAst::RestartGame { .. } => {}
        EffectAst::PlaySubgame { .. } => {}
        EffectAst::Sequence { .. } => {}
        EffectAst::CommaThen { .. } => {}
        EffectAst::SourceSentence { .. } => {}
        EffectAst::Coordinated { .. } => {}
        EffectAst::ResultBranchLabel { .. } => {}
        EffectAst::UnlessPays { .. } => {}
        EffectAst::UnlessAction { .. } => {}
        EffectAst::DelayedUntilNextEndStep { .. } => {}
        EffectAst::DelayedUntilNextCleanupStep { .. } => {}
        EffectAst::DelayedUntilNextUntapStep { .. } => {}
        EffectAst::DelayedUntilNextUpkeep { .. } => {}
        EffectAst::DelayedUntilNextDrawStep { .. } => {}
        EffectAst::DelayedUntilNextMainPhase { .. } => {}
        EffectAst::DelayedUntilNextFirstMainPhase { .. } => {}
        EffectAst::DelayedUntilEndStepOfExtraTurn { .. } => {}
        EffectAst::DelayedUntilEndOfCombat { .. } => {}
        EffectAst::DelayedTriggerThisTurn { .. } => {}
        EffectAst::DelayedTriggerForDuration { .. } => {}
        EffectAst::DelayedWhenLastObjectDiesThisTurn { .. } => {}
        EffectAst::DelayedWhenLastObjectLeavesBattlefield { .. } => {}
        EffectAst::Conditional { .. } => {}
        EffectAst::TrailingIf { .. } => {}
        EffectAst::TrailingUnless { .. } => {}
        EffectAst::ManaRestricted { .. } => {}
        EffectAst::SelfReplacement { .. } => {}
        EffectAst::ChooseObjects { .. } => {}
        EffectAst::ChooseObjectsWithAggregateConstraint { .. } => {}
        EffectAst::ChooseObjectsBottomOfLibrary { .. } => {}
        EffectAst::ChooseObjectsTopOfLibrary { .. } => {}
        EffectAst::ChooseTaggedObjectsInZone { .. } => {}
        EffectAst::ChooseObjectsAcrossZones { .. } => {}
        EffectAst::ChooseOneOf { .. } => {}
        EffectAst::VillainousChoice { .. } => {}
        EffectAst::IfEffectDidNotHappen { .. } => {}
        EffectAst::IfEffectResult { .. } => {}
        EffectAst::TagAffected { .. } => {}
        EffectAst::DirectionalAdjacentPlayerControl { .. } => {}
        EffectAst::MayCastMatchingSpellWithoutPayingManaCost { .. } => {}
        EffectAst::RepeatThisProcess => {}
        EffectAst::RepeatThisProcessMay => {}
        EffectAst::RepeatThisProcessOnce => {}
        EffectAst::RepeatEffects { .. } => {}
        EffectAst::May { .. } => {}
        EffectAst::MayByPlayer { .. } => {}
        EffectAst::AnyPlayerMay { .. } => {}
        EffectAst::ResolvedIfResult { .. } => {}
        EffectAst::ResolvedWhenResult { .. } => {}
        EffectAst::IfResult { .. } => {}
        EffectAst::WhenResult { .. } => {}
        EffectAst::ForEachOpponent { .. } => {}
        EffectAst::ForEachPlayersFiltered { .. } => {}
        EffectAst::ForEachPlayer { .. } => {}
        EffectAst::ForEachTargetPlayers { .. } => {}
        EffectAst::ForEachObject { .. } => {}
        EffectAst::ForEachTagged { .. } => {}
        EffectAst::ForEachTaggedWithControllerAtLastBlockedBy { .. } => {}
        EffectAst::MoveTaggedGroupToZone { .. } => {}
        EffectAst::SnapshotLastObjectTag { .. } => {}
        EffectAst::ForEachOpponentDoesNot { .. } => {}
        EffectAst::ForEachPlayerDoesNot { .. } => {}
        EffectAst::ForEachOpponentDid { .. } => {}
        EffectAst::ForEachPlayerDid { .. } => {}
        EffectAst::ForEachTaggedPlayer { .. } => {}
        EffectAst::RepeatProcess { .. } => {}
        EffectAst::BidLife { .. } => {}
        EffectAst::VoteStart { .. } => {}
        EffectAst::SecretChoiceStart { .. } => {}
        EffectAst::SecretChoiceReveal => {}
        EffectAst::VoteStartObjects { .. } => {}
        EffectAst::VoteStartPlayers { .. } => {}
        EffectAst::VoteOption { .. } => {}
        EffectAst::VoteExtra { .. } => {}
    }
}

pub fn for_each_nested_effects(
    effect: &EffectAst,
    include_unless_action_alternative: bool,
    mut visit: impl FnMut(&[EffectAst]),
) {
    assert_effect_ast_variant_coverage(effect);
    match effect {
        EffectAst::Conditional {
            if_true, if_false, ..
        }
        | EffectAst::SelfReplacement {
            if_true, if_false, ..
        } => {
            visit(if_true);
            visit(if_false);
        }
        EffectAst::ChooseOneOf { modes } | EffectAst::VillainousChoice { modes, .. } => {
            for mode in modes {
                visit(&mode.effects);
            }
        }
        EffectAst::IfEffectDidNotHappen { effect, otherwise } => {
            visit(std::slice::from_ref(effect.as_ref()));
            visit(otherwise);
        }
        EffectAst::IfEffectResult {
            effect, if_true, ..
        } => {
            visit(std::slice::from_ref(effect.as_ref()));
            visit(if_true);
        }
        EffectAst::TagAffected { effect, .. } => {
            visit(std::slice::from_ref(effect.as_ref()));
        }
        EffectAst::Coordination(coordination) => {
            for member in &coordination.members {
                visit(&member.effects);
            }
        }
        EffectAst::ControlFlow(control) => {
            for program in &control.programs {
                visit(&program.effects);
            }
        }
        EffectAst::Iteration(iteration) => visit(&iteration.body),
        EffectAst::DocumentProgram(program) => {
            for statement in &program.statements {
                visit(&statement.effects);
            }
        }
        nested_effects_variants!(effects) => {
            visit(effects);
        }
        EffectAst::UnlessAction {
            effects,
            alternative,
            ..
        } => {
            visit(effects);
            if include_unless_action_alternative {
                visit(alternative);
            }
        }
        _ => {}
    }
}

pub fn for_each_nested_effects_mut(
    effect: &mut EffectAst,
    include_unless_action_alternative: bool,
    mut visit: impl FnMut(&mut [EffectAst]),
) {
    assert_effect_ast_variant_coverage(effect);
    match effect {
        EffectAst::Conditional {
            if_true, if_false, ..
        }
        | EffectAst::SelfReplacement {
            if_true, if_false, ..
        } => {
            visit(if_true);
            visit(if_false);
        }
        EffectAst::ChooseOneOf { modes } | EffectAst::VillainousChoice { modes, .. } => {
            for mode in modes {
                visit(&mut mode.effects);
            }
        }
        EffectAst::IfEffectDidNotHappen { effect, otherwise } => {
            visit(std::slice::from_mut(effect.as_mut()));
            visit(otherwise);
        }
        EffectAst::IfEffectResult {
            effect, if_true, ..
        } => {
            visit(std::slice::from_mut(effect.as_mut()));
            visit(if_true);
        }
        EffectAst::TagAffected { effect, .. } => {
            visit(std::slice::from_mut(effect.as_mut()));
        }
        EffectAst::Coordination(coordination) => {
            for member in &mut coordination.members {
                visit(&mut member.effects);
            }
        }
        EffectAst::ControlFlow(control) => {
            for program in &mut control.programs {
                visit(&mut program.effects);
            }
        }
        EffectAst::Iteration(iteration) => visit(&mut iteration.body),
        EffectAst::DocumentProgram(program) => {
            for statement in &mut program.statements {
                visit(&mut statement.effects);
            }
        }
        nested_effects_variants!(effects) => {
            visit(effects);
        }
        EffectAst::UnlessAction {
            effects,
            alternative,
            ..
        } => {
            visit(effects);
            if include_unless_action_alternative {
                visit(alternative);
            }
        }
        _ => {}
    }
}

/// Visit each directly owned child vector while transparently descending
/// through boxed single-child wrappers.
///
/// Most traversal only needs slices. Presentation provenance occasionally
/// needs to replace a whole child program with one typed wrapper, which
/// requires access to the owning `Vec`.
pub fn for_each_nested_effect_vec_mut(
    effect: &mut EffectAst,
    include_unless_action_alternative: bool,
    mut visit: impl FnMut(&mut Vec<EffectAst>),
) {
    fn walk(
        effect: &mut EffectAst,
        include_unless_action_alternative: bool,
        visit: &mut impl FnMut(&mut Vec<EffectAst>),
    ) {
        assert_effect_ast_variant_coverage(effect);
        match effect {
            EffectAst::Conditional {
                if_true, if_false, ..
            }
            | EffectAst::SelfReplacement {
                if_true, if_false, ..
            } => {
                visit(if_true);
                visit(if_false);
            }
            EffectAst::ChooseOneOf { modes } | EffectAst::VillainousChoice { modes, .. } => {
                for mode in modes {
                    visit(&mut mode.effects);
                }
            }
            EffectAst::IfEffectDidNotHappen { effect, otherwise } => {
                walk(effect.as_mut(), include_unless_action_alternative, visit);
                visit(otherwise);
            }
            EffectAst::IfEffectResult {
                effect, if_true, ..
            } => {
                walk(effect.as_mut(), include_unless_action_alternative, visit);
                visit(if_true);
            }
            EffectAst::TagAffected { effect, .. } => {
                walk(effect.as_mut(), include_unless_action_alternative, visit);
            }
            EffectAst::Coordination(coordination) => {
                for member in &mut coordination.members {
                    visit(&mut member.effects);
                }
            }
            EffectAst::ControlFlow(control) => {
                for program in &mut control.programs {
                    visit(&mut program.effects);
                }
            }
            EffectAst::Iteration(iteration) => visit(&mut iteration.body),
            EffectAst::DocumentProgram(program) => {
                for statement in &mut program.statements {
                    visit(&mut statement.effects);
                }
            }
            nested_effects_variants!(effects) => {
                visit(effects);
            }
            EffectAst::UnlessAction {
                effects,
                alternative,
                ..
            } => {
                visit(effects);
                if include_unless_action_alternative {
                    visit(alternative);
                }
            }
            _ => {}
        }
    }

    walk(effect, include_unless_action_alternative, &mut visit);
}

pub fn try_for_each_nested_effects_mut<E>(
    effect: &mut EffectAst,
    include_unless_action_alternative: bool,
    mut visit: impl FnMut(&mut [EffectAst]) -> Result<(), E>,
) -> Result<(), E> {
    assert_effect_ast_variant_coverage(effect);
    match effect {
        EffectAst::Conditional {
            if_true, if_false, ..
        }
        | EffectAst::SelfReplacement {
            if_true, if_false, ..
        } => {
            visit(if_true)?;
            visit(if_false)?;
        }
        EffectAst::ChooseOneOf { modes } | EffectAst::VillainousChoice { modes, .. } => {
            for mode in modes {
                visit(&mut mode.effects)?;
            }
        }
        EffectAst::IfEffectDidNotHappen { effect, otherwise } => {
            visit(std::slice::from_mut(effect.as_mut()))?;
            visit(otherwise)?;
        }
        EffectAst::IfEffectResult {
            effect, if_true, ..
        } => {
            visit(std::slice::from_mut(effect.as_mut()))?;
            visit(if_true)?;
        }
        EffectAst::TagAffected { effect, .. } => {
            visit(std::slice::from_mut(effect.as_mut()))?;
        }
        EffectAst::Coordination(coordination) => {
            for member in &mut coordination.members {
                visit(&mut member.effects)?;
            }
        }
        EffectAst::ControlFlow(control) => {
            for program in &mut control.programs {
                visit(&mut program.effects)?;
            }
        }
        EffectAst::Iteration(iteration) => visit(&mut iteration.body)?,
        EffectAst::DocumentProgram(program) => {
            for statement in &mut program.statements {
                visit(&mut statement.effects)?;
            }
        }
        nested_effects_variants!(effects) => {
            visit(effects)?;
        }
        EffectAst::UnlessAction {
            effects,
            alternative,
            ..
        } => {
            visit(effects)?;
            if include_unless_action_alternative {
                visit(alternative)?;
            }
        }
        _ => {}
    }
    Ok(())
}

/// One traversal contract shared by recognition, reference resolution,
/// normalization, and lowering. Each semantic domain has a dedicated hook so
/// a pass does not need to invent another parallel recursion API.
pub trait SemanticVisitor {
    type Break;

    fn visit_effect(&mut self, _effect: &EffectAst) -> ControlFlow<Self::Break> {
        ControlFlow::Continue(())
    }

    fn visit_predicate(&mut self, _predicate: &PredicateAst) -> ControlFlow<Self::Break> {
        ControlFlow::Continue(())
    }

    fn visit_value(&mut self, _value: &Value) -> ControlFlow<Self::Break> {
        ControlFlow::Continue(())
    }

    fn visit_filter(&mut self, _filter: &ObjectFilter) -> ControlFlow<Self::Break> {
        ControlFlow::Continue(())
    }

    fn visit_cost(&mut self, _cost: &TotalCost) -> ControlFlow<Self::Break> {
        ControlFlow::Continue(())
    }

    fn visit_reference(&mut self, _reference: &SymbolReference) -> ControlFlow<Self::Break> {
        ControlFlow::Continue(())
    }

    fn visit_reference_binding(&mut self, reference: &SymbolReference) -> ControlFlow<Self::Break> {
        self.visit_reference(reference)
    }

    fn visit_compiler_selection(
        &mut self,
        _selection: &CompilerSelectionAst,
    ) -> ControlFlow<Self::Break> {
        ControlFlow::Continue(())
    }

    fn visit_compiler_filter(&mut self, _filter: &CompilerFilterAst) -> ControlFlow<Self::Break> {
        ControlFlow::Continue(())
    }

    fn visit_compiler_value(&mut self, _value: &CompilerValueAst) -> ControlFlow<Self::Break> {
        ControlFlow::Continue(())
    }

    fn visit_compiler_cost(&mut self, _cost: &CompilerTotalCost) -> ControlFlow<Self::Break> {
        ControlFlow::Continue(())
    }
}

fn visit_choice_domain<V: SemanticVisitor + ?Sized>(
    visitor: &mut V,
    domain: &CompilerChoiceDomainAst,
) -> ControlFlow<V::Break> {
    match domain {
        CompilerChoiceDomainAst::CardName(Some(filter)) => visitor.visit_filter(filter),
        CompilerChoiceDomainAst::Number { minimum, maximum } => {
            visit_compiler_value_tree(visitor, minimum)?;
            if let Some(maximum) = maximum {
                visit_compiler_value_tree(visitor, maximum)?;
            }
            ControlFlow::Continue(())
        }
        CompilerChoiceDomainAst::Object(object) => visit_object_operand(visitor, object),
        CompilerChoiceDomainAst::Color
        | CompilerChoiceDomainAst::CardType(_)
        | CompilerChoiceDomainAst::Named(_)
        | CompilerChoiceDomainAst::CreatureType { .. }
        | CompilerChoiceDomainAst::LandType { .. }
        | CompilerChoiceDomainAst::CardName(None)
        | CompilerChoiceDomainAst::Player { .. } => ControlFlow::Continue(()),
    }
}

fn visit_object_operand<V: SemanticVisitor + ?Sized>(
    visitor: &mut V,
    operand: &CompilerObjectOperandAst,
) -> ControlFlow<V::Break> {
    match operand {
        CompilerObjectOperandAst::Source => ControlFlow::Continue(()),
        CompilerObjectOperandAst::Selection(selection) => {
            visit_compiler_selection(visitor, selection)
        }
        CompilerObjectOperandAst::Reference(reference) => visitor.visit_reference(reference),
        CompilerObjectOperandAst::Filter(filter) => visit_compiler_filter(visitor, filter),
    }
}

fn visit_clause_actor<V: SemanticVisitor + ?Sized>(
    visitor: &mut V,
    actor: &ClauseActorAst,
) -> ControlFlow<V::Break> {
    match actor {
        ClauseActorAst::Selection(selection) => visit_compiler_selection(visitor, selection),
        ClauseActorAst::Reference(reference) => visitor.visit_reference(reference),
        ClauseActorAst::SourceController
        | ClauseActorAst::ActivePlayer
        | ClauseActorAst::EachOpponent
        | ClauseActorAst::EachPlayer
        | ClauseActorAst::Player(_) => ControlFlow::Continue(()),
    }
}

fn visit_clause_subject<V: SemanticVisitor + ?Sized>(
    visitor: &mut V,
    subject: &ClauseSubjectAst,
) -> ControlFlow<V::Break> {
    match subject {
        ClauseSubjectAst::Actor(actor) => visit_clause_actor(visitor, actor),
        ClauseSubjectAst::Selection(selection) => visit_compiler_selection(visitor, selection),
        ClauseSubjectAst::Filter(filter) => visit_compiler_filter(visitor, filter),
        ClauseSubjectAst::Reference(reference) => visitor.visit_reference(reference),
        ClauseSubjectAst::Source => ControlFlow::Continue(()),
    }
}

fn visit_clause_object<V: SemanticVisitor + ?Sized>(
    visitor: &mut V,
    object: &ClauseObjectAst,
) -> ControlFlow<V::Break> {
    match object {
        ClauseObjectAst::Subject(subject) => visit_clause_subject(visitor, subject),
        ClauseObjectAst::Selection(selection) => visit_compiler_selection(visitor, selection),
        ClauseObjectAst::Filter(filter) => visit_compiler_filter(visitor, filter),
        ClauseObjectAst::Reference(reference) => visitor.visit_reference(reference),
        ClauseObjectAst::Cost(cost) => visitor.visit_compiler_cost(cost),
    }
}

fn visit_clause_predicate<V: SemanticVisitor + ?Sized>(
    visitor: &mut V,
    predicate: &ClausePredicateAst,
) -> ControlFlow<V::Break> {
    match predicate {
        ClausePredicateAst::Matches { subject, filter } => {
            visit_clause_subject(visitor, subject)?;
            visit_compiler_filter(visitor, filter)
        }
        ClausePredicateAst::Compare { left, right, .. } => {
            visit_compiler_value_tree(visitor, left)?;
            visit_compiler_value_tree(visitor, right)
        }
        ClausePredicateAst::ReferenceExists(reference) => visitor.visit_reference(reference),
        ClausePredicateAst::Not(predicate) => visit_clause_predicate(visitor, predicate),
        ClausePredicateAst::All(predicates) | ClausePredicateAst::Any(predicates) => {
            for predicate in predicates {
                visit_clause_predicate(visitor, predicate)?;
            }
            ControlFlow::Continue(())
        }
        ClausePredicateAst::Constant(_) => ControlFlow::Continue(()),
    }
}

fn visit_compiler_selection<V: SemanticVisitor + ?Sized>(
    visitor: &mut V,
    selection: &CompilerSelectionAst,
) -> ControlFlow<V::Break> {
    visitor.visit_compiler_selection(selection)?;
    visitor.visit_reference_binding(&selection.binding)?;
    match &selection.domain {
        SelectionDomainAst::Filter(filter) => visit_compiler_filter(visitor, filter)?,
        SelectionDomainAst::ObjectOrPlayer { object, .. } | SelectionDomainAst::Spell(object) => {
            visitor.visit_filter(object)?
        }
        SelectionDomainAst::Source
        | SelectionDomainAst::AnyTarget
        | SelectionDomainAst::AnyOtherTarget
        | SelectionDomainAst::PlayerOrPlaneswalker(_)
        | SelectionDomainAst::AttackedPlayerOrPlaneswalker => {}
    }
    visit_compiler_value_tree(visitor, &selection.cardinality.min)?;
    if let Some(max) = &selection.cardinality.max {
        visit_compiler_value_tree(visitor, max)?;
    }
    ControlFlow::Continue(())
}

fn visit_compiler_filter<V: SemanticVisitor + ?Sized>(
    visitor: &mut V,
    filter: &CompilerFilterAst,
) -> ControlFlow<V::Break> {
    visitor.visit_compiler_filter(filter)?;
    match filter {
        CompilerFilterAst::Object(filter)
        | CompilerFilterAst::Spell(filter)
        | CompilerFilterAst::Card(filter) => visitor.visit_filter(filter),
        CompilerFilterAst::Player(_) => ControlFlow::Continue(()),
    }
}

fn visit_compiler_value_tree<V: SemanticVisitor + ?Sized>(
    visitor: &mut V,
    value: &CompilerValueAst,
) -> ControlFlow<V::Break> {
    visitor.visit_compiler_value(value)?;
    match value {
        CompilerValueAst::Dynamic(value) => visitor.visit_value(value),
        CompilerValueAst::Count(filter) => visit_compiler_filter(visitor, filter),
        CompilerValueAst::Arithmetic { operands, .. } => {
            for operand in operands {
                visit_compiler_value_tree(visitor, operand)?;
            }
            ControlFlow::Continue(())
        }
        CompilerValueAst::Compared { value, .. } => visit_compiler_value_tree(visitor, value),
        CompilerValueAst::Fixed(_) | CompilerValueAst::X => ControlFlow::Continue(()),
    }
}

pub fn visit_effect_tree<V: SemanticVisitor + ?Sized>(
    visitor: &mut V,
    effect: &EffectAst,
) -> ControlFlow<V::Break> {
    visit_effect_node(visitor, effect)?;
    if let EffectAst::Coordination(coordination) = effect {
        for member in &coordination.members {
            for reference in &member.imports {
                visitor.visit_reference(reference)?;
            }
            for reference in &member.exports {
                visitor.visit_reference_binding(reference)?;
            }
        }
    }
    if let EffectAst::ControlFlow(control) = effect {
        for program in &control.programs {
            for reference in &program.imports {
                visitor.visit_reference(reference)?;
            }
            for reference in &program.exports {
                visitor.visit_reference_binding(reference)?;
            }
        }
    }
    if let EffectAst::DocumentProgram(program) = effect {
        for statement in &program.statements {
            for reference in &statement.imports {
                visitor.visit_reference(reference)?;
            }
            for reference in &statement.exports {
                visitor.visit_reference_binding(reference)?;
            }
        }
        for edge in &program.edges {
            for reference in &edge.references {
                match edge.kind {
                    CompilerStatementEdgeKindAst::Reference
                    | CompilerStatementEdgeKindAst::Result => {
                        visitor.visit_reference(reference)?;
                    }
                    CompilerStatementEdgeKindAst::Ordered | CompilerStatementEdgeKindAst::Then => {}
                }
            }
        }
    }
    let mut flow = ControlFlow::Continue(());
    for_each_nested_effects(effect, true, |nested| {
        if matches!(&flow, ControlFlow::Break(_)) {
            return;
        }
        for child in nested {
            if let ControlFlow::Break(value) = visit_effect_tree(visitor, child) {
                flow = ControlFlow::Break(value);
                break;
            }
        }
    });
    flow
}

/// Visit the semantic payload owned directly by one effect node without
/// descending into child programs. Control-flow-sensitive passes use this to
/// share the canonical semantic visitor while applying their own branch/join
/// rules to child edges.
pub fn visit_effect_node<V: SemanticVisitor + ?Sized>(
    visitor: &mut V,
    effect: &EffectAst,
) -> ControlFlow<V::Break> {
    visitor.visit_effect(effect)?;
    if let EffectAst::Coordination(coordination) = effect {
        for carry in coordination
            .boundaries
            .iter()
            .flat_map(|boundary| &boundary.carries)
        {
            match &carry.fact {
                CarriedFactAst::Subject(Some(subject)) => visit_clause_subject(visitor, subject)?,
                CarriedFactAst::Object(Some(object)) => visit_clause_object(visitor, object)?,
                CarriedFactAst::Reference(Some(reference)) => {
                    visitor.visit_reference(reference)?;
                }
                CarriedFactAst::Actor
                | CarriedFactAst::Subject(None)
                | CarriedFactAst::Action(_)
                | CarriedFactAst::Object(None)
                | CarriedFactAst::Duration
                | CarriedFactAst::Reference(None) => {}
            }
        }
    }
    if let EffectAst::ControlFlow(control) = effect {
        visit_control_flow_semantics(visitor, control)?;
    }
    if let EffectAst::Iteration(iteration) = effect {
        visit_iteration_semantics(visitor, iteration)?;
    }
    if let EffectAst::Vote(vote) = effect {
        visit_vote_semantics(visitor, vote)?;
    }
    ControlFlow::Continue(())
}

fn visit_iteration_semantics<V: SemanticVisitor + ?Sized>(
    visitor: &mut V,
    iteration: &CompilerIterationAst,
) -> ControlFlow<V::Break> {
    match &iteration.source {
        CompilerIterationSourceAst::Reference(reference) => visitor.visit_reference(reference)?,
        CompilerIterationSourceAst::SelectedPlayers { collection, .. } => {
            visitor.visit_reference(collection)?
        }
        CompilerIterationSourceAst::Count(count) => visit_compiler_value_tree(visitor, count)?,
        CompilerIterationSourceAst::Objects(filter) => visitor.visit_filter(filter)?,
        CompilerIterationSourceAst::Opponents | CompilerIterationSourceAst::Players(_) => {}
    }
    visitor.visit_reference_binding(&iteration.iterator)?;
    if let Some(cardinality) = &iteration.selection_cardinality {
        visit_compiler_value_tree(visitor, &cardinality.min)?;
        if let Some(maximum) = &cardinality.max {
            visit_compiler_value_tree(visitor, maximum)?;
        }
    }
    if let Some(aggregate) = &iteration.aggregate {
        visitor.visit_reference_binding(aggregate)?;
    }
    ControlFlow::Continue(())
}

fn visit_vote_semantics<V: SemanticVisitor + ?Sized>(
    visitor: &mut V,
    vote: &CompilerVoteAst,
) -> ControlFlow<V::Break> {
    visit_choice_domain(visitor, &vote.options)?;
    visit_compiler_value_tree(visitor, &vote.votes_per_voter.min)?;
    if let Some(maximum) = &vote.votes_per_voter.max {
        visit_compiler_value_tree(visitor, maximum)?;
    }
    visitor.visit_reference_binding(&vote.choices)?;
    visitor.visit_reference_binding(&vote.tally)
}

fn visit_control_flow_semantics<V: SemanticVisitor + ?Sized>(
    visitor: &mut V,
    control: &CompilerControlFlowAst,
) -> ControlFlow<V::Break> {
    match &control.node {
        ControlFlowNodeAst::Condition { condition, .. } => {
            visit_control_predicate(visitor, &condition.predicate)
        }
        ControlFlowNodeAst::Replacement(replacement) => {
            if let Some(condition) = &replacement.condition {
                visit_control_predicate(visitor, &condition.predicate)?;
            }
            if let Some(reference) = &replacement.affected_reference {
                visitor.visit_reference(reference)?;
            }
            ControlFlow::Continue(())
        }
        ControlFlowNodeAst::Prevention(prevention) => {
            if let Some(condition) = &prevention.condition {
                visit_control_predicate(visitor, &condition.predicate)?;
            }
            if let Some(reference) = &prevention.protected_reference {
                visitor.visit_reference(reference)?;
            }
            ControlFlow::Continue(())
        }
        ControlFlowNodeAst::Permission(permission) => {
            visit_clause_actor(visitor, &permission.actor)?;
            if let Some(duration) = &permission.duration {
                visit_compiler_duration(visitor, duration)?;
            }
            ControlFlow::Continue(())
        }
        ControlFlowNodeAst::Duration { duration, .. } => visit_compiler_duration(visitor, duration),
        ControlFlowNodeAst::Delayed {
            duration,
            watched_references,
            ..
        } => {
            if let Some(duration) = duration {
                visit_compiler_duration(visitor, duration)?;
            }
            for reference in watched_references {
                visitor.visit_reference(reference)?;
            }
            ControlFlow::Continue(())
        }
        ControlFlowNodeAst::NestedAbility { .. } => ControlFlow::Continue(()),
    }
}

fn visit_control_predicate<V: SemanticVisitor + ?Sized>(
    visitor: &mut V,
    predicate: &ControlPredicateAst,
) -> ControlFlow<V::Break> {
    match predicate {
        ControlPredicateAst::State(predicate) => visit_predicate_tree(visitor, predicate),
        ControlPredicateAst::Result(_) | ControlPredicateAst::Constant(_) => {
            ControlFlow::Continue(())
        }
    }
}

fn visit_compiler_duration<V: SemanticVisitor + ?Sized>(
    visitor: &mut V,
    duration: &CompilerDurationAst,
) -> ControlFlow<V::Break> {
    match duration {
        CompilerDurationAst::Clause(duration) => match duration {
            ClauseDurationAst::ForTurns(value) => visit_compiler_value_tree(visitor, value),
            ClauseDurationAst::While(condition) => {
                visit_clause_predicate(visitor, &condition.predicate)
            }
            _ => ControlFlow::Continue(()),
        },
        CompilerDurationAst::UntilCondition(predicate)
        | CompilerDurationAst::ForAsLongAs(predicate) => visit_predicate_tree(visitor, predicate),
        CompilerDurationAst::ThisTurn
        | CompilerDurationAst::UntilEndOfTurn
        | CompilerDurationAst::UntilEndOfCombat
        | CompilerDurationAst::UntilNextTurn
        | CompilerDurationAst::Permanent => ControlFlow::Continue(()),
    }
}

pub fn visit_predicate_tree<V: SemanticVisitor + ?Sized>(
    visitor: &mut V,
    predicate: &PredicateAst,
) -> ControlFlow<V::Break> {
    if let ControlFlow::Break(value) = visitor.visit_predicate(predicate) {
        return ControlFlow::Break(value);
    }
    match predicate {
        PredicateAst::Not(inner) => visit_predicate_tree(visitor, inner),
        PredicateAst::And(left, right) | PredicateAst::Or(left, right) => {
            if let ControlFlow::Break(value) = visit_predicate_tree(visitor, left) {
                return ControlFlow::Break(value);
            }
            visit_predicate_tree(visitor, right)
        }
        _ => ControlFlow::Continue(()),
    }
}

/// Owning counterpart to `SemanticVisitor`. Defaults are identity folds so a
/// pass overrides only the domains it transforms.
pub trait SemanticFolder {
    fn fold_effect(&mut self, effect: EffectAst) -> EffectAst {
        effect
    }

    fn fold_predicate(&mut self, predicate: PredicateAst) -> PredicateAst {
        predicate
    }

    fn fold_value(&mut self, value: Value) -> Value {
        value
    }

    fn fold_filter(&mut self, filter: ObjectFilter) -> ObjectFilter {
        filter
    }

    fn fold_cost(&mut self, cost: TotalCost) -> TotalCost {
        cost
    }

    fn fold_reference(&mut self, reference: SymbolReference) -> SymbolReference {
        reference
    }

    fn fold_compiler_selection(&mut self, selection: CompilerSelectionAst) -> CompilerSelectionAst {
        selection
    }

    fn fold_compiler_filter(&mut self, filter: CompilerFilterAst) -> CompilerFilterAst {
        filter
    }

    fn fold_compiler_value(&mut self, value: CompilerValueAst) -> CompilerValueAst {
        value
    }

    fn fold_compiler_cost(&mut self, cost: CompilerTotalCost) -> CompilerTotalCost {
        cost
    }
}

fn fold_choice_domain<F: SemanticFolder + ?Sized>(
    folder: &mut F,
    domain: CompilerChoiceDomainAst,
) -> CompilerChoiceDomainAst {
    match domain {
        CompilerChoiceDomainAst::CardName(filter) => {
            CompilerChoiceDomainAst::CardName(filter.map(|filter| folder.fold_filter(filter)))
        }
        CompilerChoiceDomainAst::Player {
            filter,
            exclude_previous,
        } => CompilerChoiceDomainAst::Player {
            filter,
            exclude_previous,
        },
        CompilerChoiceDomainAst::Number { minimum, maximum } => CompilerChoiceDomainAst::Number {
            minimum: fold_compiler_value_tree(folder, minimum),
            maximum: maximum.map(|maximum| fold_compiler_value_tree(folder, maximum)),
        },
        CompilerChoiceDomainAst::Object(object) => {
            CompilerChoiceDomainAst::Object(fold_object_operand(folder, object))
        }
        domain => domain,
    }
}

fn fold_object_operand<F: SemanticFolder + ?Sized>(
    folder: &mut F,
    operand: CompilerObjectOperandAst,
) -> CompilerObjectOperandAst {
    match operand {
        CompilerObjectOperandAst::Source => CompilerObjectOperandAst::Source,
        CompilerObjectOperandAst::Selection(selection) => {
            CompilerObjectOperandAst::Selection(fold_compiler_selection(folder, selection))
        }
        CompilerObjectOperandAst::Reference(reference) => {
            CompilerObjectOperandAst::Reference(folder.fold_reference(reference))
        }
        CompilerObjectOperandAst::Filter(filter) => {
            CompilerObjectOperandAst::Filter(fold_compiler_filter(folder, filter))
        }
    }
}

fn fold_clause_actor<F: SemanticFolder + ?Sized>(
    folder: &mut F,
    actor: ClauseActorAst,
) -> ClauseActorAst {
    match actor {
        ClauseActorAst::Selection(selection) => {
            ClauseActorAst::Selection(fold_compiler_selection(folder, selection))
        }
        ClauseActorAst::Reference(reference) => {
            ClauseActorAst::Reference(folder.fold_reference(reference))
        }
        actor => actor,
    }
}

fn fold_clause_subject<F: SemanticFolder + ?Sized>(
    folder: &mut F,
    subject: ClauseSubjectAst,
) -> ClauseSubjectAst {
    match subject {
        ClauseSubjectAst::Actor(actor) => ClauseSubjectAst::Actor(fold_clause_actor(folder, actor)),
        ClauseSubjectAst::Selection(selection) => {
            ClauseSubjectAst::Selection(fold_compiler_selection(folder, selection))
        }
        ClauseSubjectAst::Filter(filter) => {
            ClauseSubjectAst::Filter(fold_compiler_filter(folder, filter))
        }
        ClauseSubjectAst::Reference(reference) => {
            ClauseSubjectAst::Reference(folder.fold_reference(reference))
        }
        ClauseSubjectAst::Source => ClauseSubjectAst::Source,
    }
}

fn fold_clause_object<F: SemanticFolder + ?Sized>(
    folder: &mut F,
    object: ClauseObjectAst,
) -> ClauseObjectAst {
    match object {
        ClauseObjectAst::Subject(subject) => {
            ClauseObjectAst::Subject(fold_clause_subject(folder, subject))
        }
        ClauseObjectAst::Selection(selection) => {
            ClauseObjectAst::Selection(fold_compiler_selection(folder, selection))
        }
        ClauseObjectAst::Filter(filter) => {
            ClauseObjectAst::Filter(fold_compiler_filter(folder, filter))
        }
        ClauseObjectAst::Reference(reference) => {
            ClauseObjectAst::Reference(folder.fold_reference(reference))
        }
        ClauseObjectAst::Cost(cost) => ClauseObjectAst::Cost(folder.fold_compiler_cost(cost)),
    }
}

fn fold_clause_duration<F: SemanticFolder + ?Sized>(
    folder: &mut F,
    duration: ClauseDurationAst,
) -> ClauseDurationAst {
    match duration {
        ClauseDurationAst::ForTurns(value) => {
            ClauseDurationAst::ForTurns(fold_compiler_value_tree(folder, value))
        }
        ClauseDurationAst::While(condition) => {
            ClauseDurationAst::While(fold_clause_condition(folder, condition))
        }
        duration => duration,
    }
}

fn fold_clause_condition<F: SemanticFolder + ?Sized>(
    folder: &mut F,
    mut condition: ClauseConditionAst,
) -> ClauseConditionAst {
    condition.predicate = fold_clause_predicate(folder, condition.predicate);
    condition
}

fn fold_clause_predicate<F: SemanticFolder + ?Sized>(
    folder: &mut F,
    predicate: ClausePredicateAst,
) -> ClausePredicateAst {
    match predicate {
        ClausePredicateAst::Matches { subject, filter } => ClausePredicateAst::Matches {
            subject: fold_clause_subject(folder, subject),
            filter: fold_compiler_filter(folder, filter),
        },
        ClausePredicateAst::Compare {
            left,
            operator,
            right,
        } => ClausePredicateAst::Compare {
            left: fold_compiler_value_tree(folder, left),
            operator,
            right: fold_compiler_value_tree(folder, right),
        },
        ClausePredicateAst::ReferenceExists(reference) => {
            ClausePredicateAst::ReferenceExists(folder.fold_reference(reference))
        }
        ClausePredicateAst::Not(predicate) => {
            ClausePredicateAst::Not(Box::new(fold_clause_predicate(folder, *predicate)))
        }
        ClausePredicateAst::All(predicates) => ClausePredicateAst::All(
            predicates
                .into_iter()
                .map(|predicate| fold_clause_predicate(folder, predicate))
                .collect(),
        ),
        ClausePredicateAst::Any(predicates) => ClausePredicateAst::Any(
            predicates
                .into_iter()
                .map(|predicate| fold_clause_predicate(folder, predicate))
                .collect(),
        ),
        ClausePredicateAst::Constant(value) => ClausePredicateAst::Constant(value),
    }
}

fn fold_compiler_selection<F: SemanticFolder + ?Sized>(
    folder: &mut F,
    mut selection: CompilerSelectionAst,
) -> CompilerSelectionAst {
    selection.binding = folder.fold_reference(selection.binding);
    selection.domain = match selection.domain {
        SelectionDomainAst::Filter(filter) => {
            SelectionDomainAst::Filter(fold_compiler_filter(folder, filter))
        }
        SelectionDomainAst::ObjectOrPlayer { object, player } => {
            SelectionDomainAst::ObjectOrPlayer {
                object: folder.fold_filter(object),
                player,
            }
        }
        SelectionDomainAst::Spell(filter) => SelectionDomainAst::Spell(folder.fold_filter(filter)),
        SelectionDomainAst::AttackedPlayerOrPlaneswalker => {
            SelectionDomainAst::AttackedPlayerOrPlaneswalker
        }
        domain => domain,
    };
    selection.cardinality.min = fold_compiler_value_tree(folder, selection.cardinality.min);
    selection.cardinality.max = selection
        .cardinality
        .max
        .map(|value| fold_compiler_value_tree(folder, value));
    folder.fold_compiler_selection(selection)
}

fn fold_compiler_filter<F: SemanticFolder + ?Sized>(
    folder: &mut F,
    filter: CompilerFilterAst,
) -> CompilerFilterAst {
    let filter = match filter {
        CompilerFilterAst::Object(filter) => CompilerFilterAst::Object(folder.fold_filter(filter)),
        CompilerFilterAst::Spell(filter) => CompilerFilterAst::Spell(folder.fold_filter(filter)),
        CompilerFilterAst::Card(filter) => CompilerFilterAst::Card(folder.fold_filter(filter)),
        CompilerFilterAst::Player(filter) => CompilerFilterAst::Player(filter),
    };
    folder.fold_compiler_filter(filter)
}

fn fold_compiler_value_tree<F: SemanticFolder + ?Sized>(
    folder: &mut F,
    value: CompilerValueAst,
) -> CompilerValueAst {
    let value = match value {
        CompilerValueAst::Dynamic(value) => CompilerValueAst::Dynamic(folder.fold_value(value)),
        CompilerValueAst::Count(filter) => {
            CompilerValueAst::Count(fold_compiler_filter(folder, filter))
        }
        CompilerValueAst::Arithmetic { operator, operands } => CompilerValueAst::Arithmetic {
            operator,
            operands: operands
                .into_iter()
                .map(|operand| fold_compiler_value_tree(folder, operand))
                .collect(),
        },
        CompilerValueAst::Compared { value, comparison } => CompilerValueAst::Compared {
            value: Box::new(fold_compiler_value_tree(folder, *value)),
            comparison,
        },
        value => value,
    };
    folder.fold_compiler_value(value)
}

pub fn fold_effect_tree<F: SemanticFolder + ?Sized>(
    folder: &mut F,
    mut effect: EffectAst,
) -> EffectAst {
    for_each_nested_effects_mut(&mut effect, true, |nested| {
        for child in nested {
            let owned = std::mem::replace(child, EffectAst::SolveCase);
            *child = fold_effect_tree(folder, owned);
        }
    });
    effect = match effect {
        EffectAst::Coordination(mut coordination) => {
            for member in &mut coordination.members {
                for reference in member.imports.iter_mut().chain(&mut member.exports) {
                    *reference = folder.fold_reference(*reference);
                }
            }
            for carry in coordination
                .boundaries
                .iter_mut()
                .flat_map(|boundary| &mut boundary.carries)
            {
                carry.fact = match std::mem::replace(&mut carry.fact, CarriedFactAst::Actor) {
                    CarriedFactAst::Subject(Some(subject)) => {
                        CarriedFactAst::Subject(Some(fold_clause_subject(folder, subject)))
                    }
                    CarriedFactAst::Object(Some(object)) => {
                        CarriedFactAst::Object(Some(fold_clause_object(folder, object)))
                    }
                    CarriedFactAst::Reference(Some(reference)) => {
                        CarriedFactAst::Reference(Some(folder.fold_reference(reference)))
                    }
                    fact => fact,
                };
            }
            EffectAst::Coordination(coordination)
        }
        EffectAst::ControlFlow(control) => {
            EffectAst::ControlFlow(Box::new(fold_control_flow_tree(folder, *control)))
        }
        EffectAst::Iteration(iteration) => {
            EffectAst::Iteration(Box::new(fold_iteration_tree(folder, *iteration)))
        }
        EffectAst::Vote(vote) => {
            let mut vote = *vote;
            vote.options = fold_choice_domain(folder, vote.options);
            vote.votes_per_voter.min = fold_compiler_value_tree(folder, vote.votes_per_voter.min);
            vote.votes_per_voter.max = vote
                .votes_per_voter
                .max
                .map(|maximum| fold_compiler_value_tree(folder, maximum));
            vote.choices = folder.fold_reference(vote.choices);
            vote.tally = folder.fold_reference(vote.tally);
            EffectAst::Vote(Box::new(vote))
        }
        EffectAst::DocumentProgram(program) => {
            EffectAst::DocumentProgram(Box::new(fold_document_program_tree(folder, *program)))
        }
        effect => effect,
    };
    folder.fold_effect(effect)
}

fn fold_document_program_tree<F: SemanticFolder + ?Sized>(
    folder: &mut F,
    mut program: CompilerDocumentProgramAst,
) -> CompilerDocumentProgramAst {
    for statement in &mut program.statements {
        for reference in statement.imports.iter_mut().chain(&mut statement.exports) {
            *reference = folder.fold_reference(*reference);
        }
    }
    for edge in &mut program.edges {
        for reference in &mut edge.references {
            *reference = folder.fold_reference(*reference);
        }
    }
    program
}

fn fold_iteration_tree<F: SemanticFolder + ?Sized>(
    folder: &mut F,
    mut iteration: CompilerIterationAst,
) -> CompilerIterationAst {
    iteration.source = match iteration.source {
        CompilerIterationSourceAst::Objects(filter) => {
            CompilerIterationSourceAst::Objects(folder.fold_filter(filter))
        }
        CompilerIterationSourceAst::Reference(reference) => {
            CompilerIterationSourceAst::Reference(folder.fold_reference(reference))
        }
        CompilerIterationSourceAst::SelectedPlayers { filter, collection } => {
            CompilerIterationSourceAst::SelectedPlayers {
                filter,
                collection: folder.fold_reference(collection),
            }
        }
        CompilerIterationSourceAst::Count(count) => {
            CompilerIterationSourceAst::Count(fold_compiler_value_tree(folder, count))
        }
        source => source,
    };
    iteration.iterator = folder.fold_reference(iteration.iterator);
    if let Some(cardinality) = &mut iteration.selection_cardinality {
        cardinality.min = fold_compiler_value_tree(folder, cardinality.min.clone());
        cardinality.max = cardinality
            .max
            .take()
            .map(|maximum| fold_compiler_value_tree(folder, maximum));
    }
    iteration.aggregate = iteration
        .aggregate
        .map(|aggregate| folder.fold_reference(aggregate));
    iteration
}

fn fold_control_flow_tree<F: SemanticFolder + ?Sized>(
    folder: &mut F,
    mut control: CompilerControlFlowAst,
) -> CompilerControlFlowAst {
    for program in &mut control.programs {
        for reference in program.imports.iter_mut().chain(&mut program.exports) {
            *reference = folder.fold_reference(*reference);
        }
    }
    match &mut control.node {
        ControlFlowNodeAst::Condition { condition, .. } => {
            fold_control_predicate(folder, &mut condition.predicate)
        }
        ControlFlowNodeAst::Replacement(replacement) => {
            if let Some(condition) = &mut replacement.condition {
                fold_control_predicate(folder, &mut condition.predicate);
            }
            if let Some(reference) = &mut replacement.affected_reference {
                *reference = folder.fold_reference(*reference);
            }
        }
        ControlFlowNodeAst::Prevention(prevention) => {
            if let Some(condition) = &mut prevention.condition {
                fold_control_predicate(folder, &mut condition.predicate);
            }
            if let Some(reference) = &mut prevention.protected_reference {
                *reference = folder.fold_reference(*reference);
            }
        }
        ControlFlowNodeAst::Permission(permission) => {
            permission.actor = fold_clause_actor(folder, permission.actor.clone());
            if let Some(duration) = &mut permission.duration {
                fold_compiler_duration(folder, duration);
            }
        }
        ControlFlowNodeAst::Duration { duration, .. } => {
            fold_compiler_duration(folder, duration);
        }
        ControlFlowNodeAst::Delayed {
            duration,
            watched_references,
            ..
        } => {
            if let Some(duration) = duration {
                fold_compiler_duration(folder, duration);
            }
            for reference in watched_references {
                *reference = folder.fold_reference(*reference);
            }
        }
        ControlFlowNodeAst::NestedAbility { .. } => {}
    }
    control
}

fn fold_control_predicate<F: SemanticFolder + ?Sized>(
    folder: &mut F,
    predicate: &mut ControlPredicateAst,
) {
    if let ControlPredicateAst::State(state) = predicate {
        *state = fold_predicate_tree(folder, state.clone());
    }
}

fn fold_compiler_duration<F: SemanticFolder + ?Sized>(
    folder: &mut F,
    duration: &mut CompilerDurationAst,
) {
    *duration = match duration.clone() {
        CompilerDurationAst::Clause(duration) => {
            CompilerDurationAst::Clause(fold_clause_duration(folder, duration))
        }
        CompilerDurationAst::UntilCondition(predicate) => {
            CompilerDurationAst::UntilCondition(fold_predicate_tree(folder, predicate))
        }
        CompilerDurationAst::ForAsLongAs(predicate) => {
            CompilerDurationAst::ForAsLongAs(fold_predicate_tree(folder, predicate))
        }
        duration => duration,
    };
}

pub fn fold_predicate_tree<F: SemanticFolder + ?Sized>(
    folder: &mut F,
    predicate: PredicateAst,
) -> PredicateAst {
    let predicate = match predicate {
        PredicateAst::Not(inner) => {
            PredicateAst::Not(Box::new(fold_predicate_tree(folder, *inner)))
        }
        PredicateAst::And(left, right) => PredicateAst::And(
            Box::new(fold_predicate_tree(folder, *left)),
            Box::new(fold_predicate_tree(folder, *right)),
        ),
        PredicateAst::Or(left, right) => PredicateAst::Or(
            Box::new(fold_predicate_tree(folder, *left)),
            Box::new(fold_predicate_tree(folder, *right)),
        ),
        predicate => predicate,
    };
    folder.fold_predicate(predicate)
}
