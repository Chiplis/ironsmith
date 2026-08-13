use std::ops::ControlFlow;

use crate::cost::TotalCost;
use crate::effect::Value;
use crate::model::ast::{EffectAst, PredicateAst, SubjectVerbActionAst};
use crate::model::clauses::{
    ClauseActorAst, ClauseComplementAst, ClauseConditionAst, ClauseDestinationAst,
    ClauseDurationAst, ClauseObjectAst, ClausePredicateAst, ClauseSubjectAst, CompilerClauseAst,
};
use crate::model::coordination::CarriedFactAst;
use crate::model::costs::CompilerTotalCost;
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
pub(crate) enum TerminalResultProducer {
    Clash,
    FlipCoin,
}

pub(crate) fn terminal_result_producer(effect: &EffectAst) -> Option<TerminalResultProducer> {
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

pub(crate) fn assert_effect_ast_variant_coverage(effect: &EffectAst) {
    match effect {
        EffectAst::Clause(_) => {}
        EffectAst::Coordination(_) => {}
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

pub(crate) fn for_each_nested_effects(
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

pub(crate) fn for_each_nested_effects_mut(
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
pub(crate) fn for_each_nested_effect_vec_mut(
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

pub(crate) fn try_for_each_nested_effects_mut<E>(
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
pub(crate) trait SemanticVisitor {
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

    fn visit_clause(&mut self, _clause: &CompilerClauseAst) -> ControlFlow<Self::Break> {
        ControlFlow::Continue(())
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

pub(crate) fn visit_clause_tree<V: SemanticVisitor + ?Sized>(
    visitor: &mut V,
    clause: &CompilerClauseAst,
) -> ControlFlow<V::Break> {
    visitor.visit_clause(clause)?;
    visit_clause_actor(visitor, &clause.actor)?;
    visit_clause_subject(visitor, &clause.subject)?;
    if let Some(object) = &clause.object {
        visit_clause_object(visitor, object)?;
    }
    if let Some(quantity) = &clause.quantity {
        visit_compiler_value_tree(visitor, &quantity.value)?;
    }
    if let Some(destination) = &clause.destination {
        visit_clause_destination(visitor, destination)?;
    }
    if let Some(duration) = &clause.duration {
        visit_clause_duration(visitor, duration)?;
    }
    if let Some(condition) = &clause.condition {
        visit_clause_condition(visitor, condition)?;
    }
    for binding in &clause.bindings {
        visitor.visit_reference(&binding.reference)?;
    }
    for complement in &clause.complements {
        match complement {
            ClauseComplementAst::Object(object) => visit_clause_object(visitor, object)?,
            ClauseComplementAst::Quantity(quantity) => {
                visit_compiler_value_tree(visitor, &quantity.value)?
            }
            ClauseComplementAst::Destination(destination) => {
                visit_clause_destination(visitor, destination)?
            }
            ClauseComplementAst::Duration(duration) => visit_clause_duration(visitor, duration)?,
            ClauseComplementAst::Condition(condition) => {
                visit_clause_condition(visitor, condition)?
            }
            ClauseComplementAst::Binding(binding) => visitor.visit_reference(&binding.reference)?,
        }
    }
    ControlFlow::Continue(())
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
        | ClauseActorAst::EachPlayer => ControlFlow::Continue(()),
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

fn visit_clause_destination<V: SemanticVisitor + ?Sized>(
    visitor: &mut V,
    destination: &ClauseDestinationAst,
) -> ControlFlow<V::Break> {
    if let Some(controller) = &destination.controller {
        visit_clause_actor(visitor, controller)?;
    }
    ControlFlow::Continue(())
}

fn visit_clause_duration<V: SemanticVisitor + ?Sized>(
    visitor: &mut V,
    duration: &ClauseDurationAst,
) -> ControlFlow<V::Break> {
    match duration {
        ClauseDurationAst::ForTurns(value) => visit_compiler_value_tree(visitor, value),
        ClauseDurationAst::While(condition) => visit_clause_condition(visitor, condition),
        ClauseDurationAst::Permanent
        | ClauseDurationAst::ThisTurn
        | ClauseDurationAst::UntilEndOfTurn
        | ClauseDurationAst::UntilEndOfCombat
        | ClauseDurationAst::UntilNextTurn => ControlFlow::Continue(()),
    }
}

fn visit_clause_condition<V: SemanticVisitor + ?Sized>(
    visitor: &mut V,
    condition: &ClauseConditionAst,
) -> ControlFlow<V::Break> {
    visit_clause_predicate(visitor, &condition.predicate)
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
    visitor.visit_reference(&selection.binding)?;
    match &selection.domain {
        SelectionDomainAst::Filter(filter) => visit_compiler_filter(visitor, filter)?,
        SelectionDomainAst::ObjectOrPlayer { object, .. } | SelectionDomainAst::Spell(object) => {
            visitor.visit_filter(object)?
        }
        SelectionDomainAst::Source
        | SelectionDomainAst::AnyTarget
        | SelectionDomainAst::AnyOtherTarget
        | SelectionDomainAst::PlayerOrPlaneswalker(_) => {}
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

pub(crate) fn visit_effect_tree<V: SemanticVisitor + ?Sized>(
    visitor: &mut V,
    effect: &EffectAst,
) -> ControlFlow<V::Break> {
    if let ControlFlow::Break(value) = visitor.visit_effect(effect) {
        return ControlFlow::Break(value);
    }
    if let EffectAst::Clause(clause) = effect {
        visit_clause_tree(visitor, clause)?;
    }
    if let EffectAst::Coordination(coordination) = effect {
        for member in &coordination.members {
            for reference in member.imports.iter().chain(&member.exports) {
                visitor.visit_reference(reference)?;
            }
        }
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

pub(crate) fn visit_predicate_tree<V: SemanticVisitor + ?Sized>(
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
pub(crate) trait SemanticFolder {
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

    fn fold_clause(&mut self, clause: CompilerClauseAst) -> CompilerClauseAst {
        clause
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

pub(crate) fn fold_clause_tree<F: SemanticFolder + ?Sized>(
    folder: &mut F,
    mut clause: CompilerClauseAst,
) -> CompilerClauseAst {
    clause.actor = fold_clause_actor(folder, clause.actor);
    clause.subject = fold_clause_subject(folder, clause.subject);
    clause.object = clause
        .object
        .map(|object| fold_clause_object(folder, object));
    if let Some(quantity) = &mut clause.quantity {
        quantity.value = fold_compiler_value_tree(folder, quantity.value.clone());
    }
    clause.destination = clause
        .destination
        .map(|destination| fold_clause_destination(folder, destination));
    clause.duration = clause
        .duration
        .map(|duration| fold_clause_duration(folder, duration));
    clause.condition = clause
        .condition
        .map(|condition| fold_clause_condition(folder, condition));
    for binding in &mut clause.bindings {
        binding.reference = folder.fold_reference(binding.reference.clone());
    }
    clause.complements = clause
        .complements
        .into_iter()
        .map(|complement| match complement {
            ClauseComplementAst::Object(object) => {
                ClauseComplementAst::Object(fold_clause_object(folder, object))
            }
            ClauseComplementAst::Quantity(mut quantity) => {
                quantity.value = fold_compiler_value_tree(folder, quantity.value);
                ClauseComplementAst::Quantity(quantity)
            }
            ClauseComplementAst::Destination(destination) => {
                ClauseComplementAst::Destination(fold_clause_destination(folder, destination))
            }
            ClauseComplementAst::Duration(duration) => {
                ClauseComplementAst::Duration(fold_clause_duration(folder, duration))
            }
            ClauseComplementAst::Condition(condition) => {
                ClauseComplementAst::Condition(fold_clause_condition(folder, condition))
            }
            ClauseComplementAst::Binding(mut binding) => {
                binding.reference = folder.fold_reference(binding.reference);
                ClauseComplementAst::Binding(binding)
            }
        })
        .collect();
    folder.fold_clause(clause)
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

fn fold_clause_destination<F: SemanticFolder + ?Sized>(
    folder: &mut F,
    mut destination: ClauseDestinationAst,
) -> ClauseDestinationAst {
    destination.controller = destination
        .controller
        .map(|actor| fold_clause_actor(folder, actor));
    destination
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

pub(crate) fn fold_effect_tree<F: SemanticFolder + ?Sized>(
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
        EffectAst::Clause(clause) => EffectAst::Clause(fold_clause_tree(folder, clause)),
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
        effect => effect,
    };
    folder.fold_effect(effect)
}

pub(crate) fn fold_predicate_tree<F: SemanticFolder + ?Sized>(
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
