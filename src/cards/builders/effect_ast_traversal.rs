use super::EffectAst;

// Keep the list of wrapper variants with `effects: Vec<EffectAst>` in one place.
// This avoids drift between immutable/mutable/fallible traversal helpers.
macro_rules! nested_effects_variants {
    ($effects:ident) => {
        EffectAst::UnlessPays {
            effects: $effects, ..
        }
            | EffectAst::May { effects: $effects }
            | EffectAst::MayByPlayer {
                effects: $effects, ..
            }
            | EffectAst::MayByTaggedController {
                effects: $effects, ..
            }
            | EffectAst::IfResult {
                effects: $effects, ..
            }
            | EffectAst::ForEachOpponent { effects: $effects }
            | EffectAst::ForEachPlayersFiltered {
                effects: $effects, ..
            }
            | EffectAst::ForEachPlayer { effects: $effects }
            | EffectAst::ForEachTargetPlayers {
                effects: $effects, ..
            }
            | EffectAst::ForEachObject {
                effects: $effects, ..
            }
            | EffectAst::ForEachTagged {
                effects: $effects, ..
            }
            | EffectAst::ForEachOpponentDoesNot { effects: $effects }
            | EffectAst::ForEachPlayerDoesNot { effects: $effects }
            | EffectAst::ForEachOpponentDid {
                effects: $effects, ..
            }
            | EffectAst::ForEachPlayerDid {
                effects: $effects, ..
            }
            | EffectAst::ForEachTaggedPlayer {
                effects: $effects, ..
            }
            | EffectAst::DelayedUntilNextEndStep {
                effects: $effects, ..
            }
            | EffectAst::DelayedUntilNextUpkeep {
                effects: $effects, ..
            }
            | EffectAst::DelayedUntilEndStepOfExtraTurn {
                effects: $effects, ..
            }
            | EffectAst::DelayedUntilEndOfCombat { effects: $effects }
            | EffectAst::DelayedTriggerThisTurn {
                effects: $effects, ..
            }
            | EffectAst::DelayedWhenLastObjectDiesThisTurn {
                effects: $effects, ..
            }
            | EffectAst::VoteOption {
                effects: $effects, ..
            }
    };
}

pub(super) fn for_each_nested_effects(
    effect: &EffectAst,
    include_unless_action_alternative: bool,
    mut visit: impl FnMut(&[EffectAst]),
) {
    match effect {
        EffectAst::Conditional {
            if_true, if_false, ..
        } => {
            visit(if_true);
            visit(if_false);
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

pub(super) fn for_each_nested_effects_mut(
    effect: &mut EffectAst,
    include_unless_action_alternative: bool,
    mut visit: impl FnMut(&mut [EffectAst]),
) {
    match effect {
        EffectAst::Conditional {
            if_true, if_false, ..
        } => {
            visit(if_true);
            visit(if_false);
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

pub(super) fn try_for_each_nested_effects_mut<E>(
    effect: &mut EffectAst,
    include_unless_action_alternative: bool,
    mut visit: impl FnMut(&mut [EffectAst]) -> Result<(), E>,
) -> Result<(), E> {
    match effect {
        EffectAst::Conditional {
            if_true, if_false, ..
        } => {
            visit(if_true)?;
            visit(if_false)?;
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
