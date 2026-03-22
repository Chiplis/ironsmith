use crate::cards::builders::EffectAst;

pub(crate) fn normalize_effects_ast(effects: &[EffectAst]) -> Vec<EffectAst> {
    let mut normalized = effects.to_vec();
    normalize_effects_vec(&mut normalized);
    normalized
}

fn normalize_effects_vec(effects: &mut Vec<EffectAst>) {
    for effect in effects.iter_mut() {
        normalize_nested_effects(effect);
    }
    if let Some(rewritten) = rewrite_repeat_process(effects) {
        *effects = rewritten;
    }
    if let Some(rewritten) = rewrite_repeat_process_once(effects) {
        *effects = rewritten;
    }
    effects.retain(|effect| !is_noop_effect(effect));
}

fn normalize_nested_effects(effect: &mut EffectAst) {
    match effect {
        EffectAst::Conditional {
            if_true, if_false, ..
        }
        | EffectAst::SelfReplacement {
            if_true, if_false, ..
        } => {
            normalize_effects_vec(if_true);
            normalize_effects_vec(if_false);
        }
        EffectAst::UnlessPays { effects, .. }
        | EffectAst::May { effects }
        | EffectAst::MayByPlayer { effects, .. }
        | EffectAst::ResolvedIfResult { effects, .. }
        | EffectAst::ResolvedWhenResult { effects, .. }
        | EffectAst::IfResult { effects, .. }
        | EffectAst::WhenResult { effects, .. }
        | EffectAst::ForEachOpponent { effects }
        | EffectAst::ForEachPlayersFiltered { effects, .. }
        | EffectAst::ForEachPlayer { effects }
        | EffectAst::ForEachTargetPlayers { effects, .. }
        | EffectAst::ForEachObject { effects, .. }
        | EffectAst::ForEachTagged { effects, .. }
        | EffectAst::ForEachOpponentDoesNot { effects, .. }
        | EffectAst::ForEachPlayerDoesNot { effects, .. }
        | EffectAst::ForEachOpponentDid { effects, .. }
        | EffectAst::ForEachPlayerDid { effects, .. }
        | EffectAst::ForEachTaggedPlayer { effects, .. }
        | EffectAst::RepeatProcess { effects, .. }
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

fn rewrite_repeat_process(effects: &[EffectAst]) -> Option<Vec<EffectAst>> {
    if effects.len() < 2 {
        return None;
    }

    let last_index = effects.len() - 1;
    let EffectAst::IfResult {
        predicate,
        effects: tail_effects,
    } = &effects[last_index]
    else {
        return None;
    };
    if !matches!(tail_effects.last(), Some(EffectAst::RepeatThisProcess)) {
        return None;
    }

    let continue_effect_index = last_index.saturating_sub(1);
    let mut body = effects.to_vec();
    let EffectAst::IfResult { effects, .. } = &mut body[last_index] else {
        return None;
    };
    effects.pop();

    Some(vec![EffectAst::RepeatProcess {
        effects: body,
        continue_effect_index,
        continue_predicate: *predicate,
    }])
}

fn rewrite_repeat_process_once(effects: &[EffectAst]) -> Option<Vec<EffectAst>> {
    if effects.len() < 2 || !matches!(effects.last(), Some(EffectAst::RepeatThisProcessOnce)) {
        return None;
    }

    let body = effects[..effects.len() - 1].to_vec();
    let mut duplicated = body.clone();
    duplicated.extend(body);
    Some(duplicated)
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
    use crate::cards::builders::IfResultPredicate;
    use crate::cards::builders::{EffectAst, PlayerAst};
    use crate::effect::{Until, Value};
    use crate::filter::ObjectFilter;

    use super::normalize_effects_ast;

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

    #[test]
    fn normalize_rewrites_repeat_this_process_tail_into_loop_effect() {
        let effects = vec![
            EffectAst::May {
                effects: vec![EffectAst::Draw {
                    count: Value::Fixed(1),
                    player: PlayerAst::You,
                }],
            },
            EffectAst::IfResult {
                predicate: IfResultPredicate::Did,
                effects: vec![
                    EffectAst::GainLife {
                        amount: Value::Fixed(1),
                        player: PlayerAst::You,
                    },
                    EffectAst::RepeatThisProcess,
                ],
            },
        ];

        let normalized = normalize_effects_ast(&effects);
        assert!(matches!(
            normalized.as_slice(),
            [EffectAst::RepeatProcess {
                continue_effect_index: 0,
                continue_predicate: IfResultPredicate::Did,
                ..
            }]
        ));
    }
}
