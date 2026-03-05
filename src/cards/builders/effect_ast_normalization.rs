use super::*;

pub(crate) fn normalize_effects_ast(effects: &[EffectAst]) -> Vec<EffectAst> {
    let mut normalized = effects.to_vec();
    normalize_effects_vec(&mut normalized);
    normalized
}

fn normalize_effects_vec(effects: &mut Vec<EffectAst>) {
    for effect in effects.iter_mut() {
        normalize_nested_effects(effect);
    }
    effects.retain(|effect| !is_noop_effect(effect));
}

fn normalize_nested_effects(effect: &mut EffectAst) {
    match effect {
        EffectAst::Conditional {
            if_true, if_false, ..
        } => {
            normalize_effects_vec(if_true);
            normalize_effects_vec(if_false);
        }
        EffectAst::UnlessPays { effects, .. }
        | EffectAst::May { effects }
        | EffectAst::MayByPlayer { effects, .. }
        | EffectAst::MayByTaggedController { effects, .. }
        | EffectAst::IfResult { effects, .. }
        | EffectAst::ForEachOpponent { effects }
        | EffectAst::ForEachPlayersFiltered { effects, .. }
        | EffectAst::ForEachPlayer { effects }
        | EffectAst::ForEachTargetPlayers { effects, .. }
        | EffectAst::ForEachObject { effects, .. }
        | EffectAst::ForEachTagged { effects, .. }
        | EffectAst::ForEachOpponentDoesNot { effects }
        | EffectAst::ForEachPlayerDoesNot { effects }
        | EffectAst::ForEachOpponentDid { effects, .. }
        | EffectAst::ForEachPlayerDid { effects, .. }
        | EffectAst::ForEachTaggedPlayer { effects, .. }
        | EffectAst::DelayedUntilNextEndStep { effects, .. }
        | EffectAst::DelayedUntilNextUpkeep { effects, .. }
        | EffectAst::DelayedUntilEndStepOfExtraTurn { effects, .. }
        | EffectAst::DelayedUntilEndOfCombat { effects }
        | EffectAst::DelayedTriggerThisTurn { effects, .. }
        | EffectAst::DelayedWhenLastObjectDiesThisTurn { effects, .. }
        | EffectAst::VoteOption { effects, .. } => normalize_effects_vec(effects),
        EffectAst::UnlessAction {
            effects,
            alternative,
            ..
        } => {
            normalize_effects_vec(effects);
            normalize_effects_vec(alternative);
        }
        _ => {}
    }
}

fn is_noop_effect(effect: &EffectAst) -> bool {
    match effect {
        EffectAst::GrantAbilitiesAll { abilities, .. }
        | EffectAst::RemoveAbilitiesAll { abilities, .. }
        | EffectAst::GrantAbilitiesChoiceAll { abilities, .. } => abilities.is_empty(),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_removes_empty_global_grant_effect() {
        let effects = vec![EffectAst::GrantAbilitiesAll {
            filter: ObjectFilter::default(),
            abilities: Vec::new(),
            duration: Until::EndOfTurn,
        }];

        let normalized = normalize_effects_ast(&effects);
        assert!(normalized.is_empty());
    }

    #[test]
    fn normalize_removes_empty_global_grant_effect_inside_wrappers() {
        let effects = vec![EffectAst::May {
            effects: vec![
                EffectAst::GrantAbilitiesAll {
                    filter: ObjectFilter::default(),
                    abilities: Vec::new(),
                    duration: Until::EndOfTurn,
                },
                EffectAst::Draw {
                    count: Value::Fixed(1),
                    player: PlayerAst::You,
                },
            ],
        }];

        let normalized = normalize_effects_ast(&effects);
        let EffectAst::May { effects } = &normalized[0] else {
            panic!("expected wrapped may effect");
        };
        assert_eq!(effects.len(), 1);
        assert!(matches!(effects[0], EffectAst::Draw { .. }));
    }
}
