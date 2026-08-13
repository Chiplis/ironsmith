use std::ops::ControlFlow;

use crate::cost::TotalCost;
use crate::effect::Value;
use crate::model::ast::{EffectAst, PredicateAst, SubjectVerbActionAst};
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
}

pub(crate) fn visit_effect_tree<V: SemanticVisitor + ?Sized>(
    visitor: &mut V,
    effect: &EffectAst,
) -> ControlFlow<V::Break> {
    if let ControlFlow::Break(value) = visitor.visit_effect(effect) {
        return ControlFlow::Break(value);
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
