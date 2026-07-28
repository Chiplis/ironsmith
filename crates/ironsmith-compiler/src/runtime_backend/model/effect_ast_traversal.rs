use crate::cards::builders::EffectAst;

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
        EffectAst::UnlessPays { .. } => {}
        EffectAst::UnlessAction { .. } => {}
        EffectAst::DelayedUntilNextEndStep { .. } => {}
        EffectAst::DelayedUntilNextCleanupStep { .. } => {}
        EffectAst::DelayedUntilNextUntapStep { .. } => {}
        EffectAst::DelayedUntilNextUpkeep { .. } => {}
        EffectAst::DelayedUntilNextDrawStep { .. } => {}
        EffectAst::DelayedUntilNextMainPhase { .. } => {}
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
                walk(
                    effect.as_mut(),
                    include_unless_action_alternative,
                    visit,
                );
                visit(otherwise);
            }
            EffectAst::TagAffected { effect, .. } => {
                walk(
                    effect.as_mut(),
                    include_unless_action_alternative,
                    visit,
                );
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
