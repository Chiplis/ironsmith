use crate::cards::builders::{EffectAst, IT_TAG, PredicateAst, SubjectVerbActionAst, TargetAst};
use crate::effect::Value;
use ironsmith_core::ValueSurfaceHint;

pub(crate) fn normalize_effects_ast(effects: &[EffectAst]) -> Vec<EffectAst> {
    let mut normalized = effects.to_vec();
    bind_typed_where_x_references(&mut normalized, None);
    normalize_effects_vec(&mut normalized);
    normalized
}

fn typed_where_x_binding(effect: &EffectAst) -> Option<Value> {
    let EffectAst::SubjectVerb(subject_verb) = effect else {
        return None;
    };
    let SubjectVerbActionAst::LookAtTopCards { count, .. } = &subject_verb.action else {
        return None;
    };
    let Value::SurfaceHinted { value, hints } = count else {
        return None;
    };
    hints
        .contains(&ValueSurfaceHint::WhereXIs)
        .then(|| value.as_ref().clone())
}

fn replace_bound_x_in_value(value: &mut Value, replacement: &Value) {
    match value {
        Value::X => *value = replacement.clone(),
        Value::XTimes(multiplier) => {
            let multiplier = *multiplier;
            *value = if multiplier == 1 {
                replacement.clone()
            } else if let Value::Fixed(fixed) = replacement {
                Value::Fixed(fixed * multiplier)
            } else {
                Value::Scaled(Box::new(replacement.clone()), multiplier)
            };
        }
        Value::SurfaceHinted { value, .. }
        | Value::Scaled(value, _)
        | Value::DividedRoundedDown(value, _)
        | Value::HalfRoundedDown(value) => replace_bound_x_in_value(value, replacement),
        Value::Add(left, right) | Value::Min(left, right) => {
            replace_bound_x_in_value(left, replacement);
            replace_bound_x_in_value(right, replacement);
        }
        _ => {}
    }
}

fn replace_bound_x_in_predicate(predicate: &mut PredicateAst, replacement: &Value) {
    match predicate {
        PredicateAst::ValueComparison { left, right, .. } => {
            replace_bound_x_in_value(left, replacement);
            replace_bound_x_in_value(right, replacement);
        }
        PredicateAst::Not(inner) => replace_bound_x_in_predicate(inner, replacement),
        PredicateAst::And(left, right) | PredicateAst::Or(left, right) => {
            replace_bound_x_in_predicate(left, replacement);
            replace_bound_x_in_predicate(right, replacement);
        }
        _ => {}
    }
}

fn bind_typed_where_x_references(effects: &mut [EffectAst], inherited: Option<Value>) {
    let mut binding = inherited;
    for effect in effects {
        match effect {
            EffectAst::Conditional {
                predicate,
                if_true,
                if_false,
            }
            | EffectAst::SelfReplacement {
                predicate,
                if_true,
                if_false,
                ..
            } => {
                if let Some(replacement) = binding.as_ref() {
                    replace_bound_x_in_predicate(predicate, replacement);
                }
                bind_typed_where_x_references(if_true, binding.clone());
                bind_typed_where_x_references(if_false, binding.clone());
            }
            EffectAst::ChooseOneOf { modes } | EffectAst::VillainousChoice { modes, .. } => {
                for mode in modes {
                    bind_typed_where_x_references(&mut mode.effects, binding.clone());
                }
            }
            EffectAst::IfEffectDidNotHappen { effect, otherwise } => {
                bind_typed_where_x_references(
                    std::slice::from_mut(effect.as_mut()),
                    binding.clone(),
                );
                bind_typed_where_x_references(otherwise, binding.clone());
            }
            EffectAst::TagAffected { effect, .. } => bind_typed_where_x_references(
                std::slice::from_mut(effect.as_mut()),
                binding.clone(),
            ),
            _ => super::effect_ast_traversal::for_each_nested_effects_mut(effect, true, |nested| {
                bind_typed_where_x_references(nested, binding.clone())
            }),
        }

        if let Some(next_binding) = typed_where_x_binding(effect) {
            binding = Some(next_binding);
        }
    }
}

fn normalize_effects_vec(effects: &mut Vec<EffectAst>) {
    for effect in effects.iter_mut() {
        normalize_nested_effects(effect);
    }
    if let Some(rewritten) = rewrite_repeat_process(effects) {
        *effects = rewritten;
    }
    if let Some(rewritten) = rewrite_repeat_process_may(effects) {
        *effects = rewritten;
    }
    if let Some(rewritten) = rewrite_repeat_process_once(effects) {
        *effects = rewritten;
    }
    if let Some(rewritten) = rewrite_return_as_aura(effects) {
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
        | EffectAst::AnyPlayerMay { effects }
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
        | EffectAst::RepeatEffects { effects, .. }
        | EffectAst::BidLife {
            winner_effects: effects,
            ..
        }
        | EffectAst::DelayedUntilNextEndStep { effects, .. }
        | EffectAst::DelayedUntilNextUpkeep { effects, .. }
        | EffectAst::DelayedUntilNextDrawStep { effects, .. }
        | EffectAst::DelayedUntilEndStepOfExtraTurn { effects, .. }
        | EffectAst::DelayedUntilEndOfCombat { effects }
        | EffectAst::DelayedTriggerThisTurn { effects, .. }
        | EffectAst::DelayedWhenLastObjectDiesThisTurn { effects, .. }
        | EffectAst::VoteOption { effects, .. }
        | EffectAst::ManaRestricted { effects, .. } => normalize_effects_vec(effects),
        EffectAst::UnlessAction {
            effects,
            alternative,
            ..
        } => {
            normalize_effects_vec(effects);
            normalize_effects_vec(alternative);
        }
        // NOTE: this walker stays hand-rolled (rather than routing through
        // effect_ast_traversal's shared helper) because normalize_effects_vec
        // resizes/replaces the Vec (retain + whole-Vec rewrites), which the
        // slice-exposing helper cannot express. New wrapper variants must be
        // added here and kept in sync with the traversal macro.
        EffectAst::ChooseOneOf { modes } | EffectAst::VillainousChoice { modes, .. } => {
            for mode in modes {
                normalize_effects_vec(&mut mode.effects);
            }
        }
        EffectAst::IfEffectDidNotHappen { effect, otherwise } => {
            normalize_nested_effects(effect);
            normalize_effects_vec(otherwise);
        }
        EffectAst::TagAffected { effect, .. } => {
            normalize_nested_effects(effect);
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

fn rewrite_repeat_process_may(effects: &[EffectAst]) -> Option<Vec<EffectAst>> {
    if effects.len() < 2 || !matches!(effects.last(), Some(EffectAst::RepeatThisProcessMay)) {
        return None;
    }

    Some(vec![EffectAst::RepeatProcess {
        effects: effects.to_vec(),
        continue_effect_index: effects.len() - 1,
        continue_predicate: crate::cards::builders::IfResultPredicate::Did,
    }])
}

fn rewrite_return_as_aura(effects: &[EffectAst]) -> Option<Vec<EffectAst>> {
    use crate::cards::builders::{IT_TAG, ReturnAsAuraAst, SubjectVerbActionAst, TargetAst};

    let mut rewritten = Vec::with_capacity(effects.len());
    let mut index = 0;
    let mut changed = false;
    while index < effects.len() {
        let Some(EffectAst::SubjectVerb(return_subject_verb)) = effects.get(index) else {
            rewritten.push(effects[index].clone());
            index += 1;
            continue;
        };
        let SubjectVerbActionAst::ReturnToBattlefield { as_aura: None, .. } =
            &return_subject_verb.action
        else {
            rewritten.push(effects[index].clone());
            index += 1;
            continue;
        };
        let Some(EffectAst::SubjectVerb(aura_subject_verb)) = effects.get(index + 1) else {
            rewritten.push(effects[index].clone());
            index += 1;
            continue;
        };
        let SubjectVerbActionAst::BecomeAuraEnchantment {
            target,
            attachment_filter,
            granted_abilities,
            ..
        } = &aura_subject_verb.action
        else {
            rewritten.push(effects[index].clone());
            index += 1;
            continue;
        };
        if !matches!(target, TargetAst::Tagged(tag, _) if tag.as_str() == IT_TAG) {
            rewritten.push(effects[index].clone());
            index += 1;
            continue;
        }

        let mut remove_all_abilities = false;
        let mut consumed = 2;
        if let Some(EffectAst::SubjectVerb(remove_subject_verb)) = effects.get(index + 2)
            && is_return_as_aura_remove_all_marker(&remove_subject_verb.action)
        {
            remove_all_abilities = true;
            consumed = 3;
        }

        let mut combined = effects[index].clone();
        if let EffectAst::SubjectVerb(subject_verb) = &mut combined
            && let SubjectVerbActionAst::ReturnToBattlefield { as_aura, .. } =
                &mut subject_verb.action
        {
            *as_aura = Some(ReturnAsAuraAst {
                attachment_filter: attachment_filter.clone(),
                remove_all_abilities,
                granted_abilities: granted_abilities.clone(),
            });
        }
        rewritten.push(combined);
        index += consumed;
        changed = true;
    }

    changed.then_some(rewritten)
}

fn is_return_as_aura_remove_all_marker(action: &SubjectVerbActionAst) -> bool {
    match action {
        SubjectVerbActionAst::RemoveAbilitiesAll {
            abilities,
            duration,
            ..
        } => abilities.is_empty() && matches!(duration, crate::effect::Until::Forever),
        SubjectVerbActionAst::RemoveAbilitiesFromTarget {
            target,
            abilities,
            duration,
        } => {
            abilities.is_empty()
                && matches!(duration, crate::effect::Until::Forever)
                && matches!(target, TargetAst::Tagged(tag, _) if tag.as_str() == IT_TAG)
        }
        _ => false,
    }
}

fn is_noop_effect(effect: &EffectAst) -> bool {
    match effect {
        EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst {
            action:
                crate::cards::builders::SubjectVerbActionAst::GrantAbilitiesAll { abilities, .. }
                | crate::cards::builders::SubjectVerbActionAst::GrantAbilitiesChoiceAll {
                    abilities, ..
                },
            ..
        }) => abilities.is_empty(),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use crate::cards::builders::IfResultPredicate;
    use crate::cards::builders::{EffectAst, PlayerAst, PredicateAst, TagKey};
    use crate::effect::{Until, Value};
    use crate::filter::{ObjectFilter, PlayerFilter};
    use ironsmith_core::ValueSurfaceHint;

    use super::normalize_effects_ast;

    #[test]
    fn normalize_removes_empty_global_grant_effect() {
        let effects = vec![EffectAst::subject_verb_grant_abilities_all(
            ObjectFilter::default(),
            Vec::new(),
            Until::EndOfTurn,
        )];

        let normalized = normalize_effects_ast(&effects);
        assert!(normalized.is_empty());
    }

    #[test]
    fn normalize_removes_empty_global_grant_effect_inside_wrappers() {
        let effects = vec![EffectAst::May {
            effects: vec![
                EffectAst::subject_verb_grant_abilities_all(
                    ObjectFilter::default(),
                    Vec::new(),
                    Until::EndOfTurn,
                ),
                EffectAst::subject_verb(
                    crate::cards::builders::SubjectVerbRoleAst::AffectedPlayer,
                    PlayerAst::You,
                    crate::cards::builders::SubjectVerbActionAst::Draw {
                        count: Value::Fixed(1),
                    },
                ),
            ],
        }];

        let normalized = normalize_effects_ast(&effects);
        let EffectAst::May { effects } = &normalized[0] else {
            panic!("expected wrapped may effect");
        };
        assert_eq!(effects.len(), 1);
        assert!(matches!(
            effects[0],
            EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst {
                action: crate::cards::builders::SubjectVerbActionAst::Draw { .. },
                ..
            })
        ));
    }

    #[test]
    fn normalize_binds_later_predicate_x_to_typed_where_x_value() {
        let where_x =
            Value::CardsInHand(PlayerFilter::You).with_surface_hint(ValueSurfaceHint::WhereXIs);
        let effects = vec![
            EffectAst::subject_verb_look_at_top_cards(
                PlayerAst::You,
                where_x,
                TagKey::from("looked"),
            ),
            EffectAst::Conditional {
                predicate: PredicateAst::ValueComparison {
                    left: Value::X,
                    operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                    right: Value::Fixed(1),
                },
                if_true: Vec::new(),
                if_false: Vec::new(),
            },
        ];

        let normalized = normalize_effects_ast(&effects);
        let EffectAst::Conditional {
            predicate: PredicateAst::ValueComparison { left, .. },
            ..
        } = &normalized[1]
        else {
            panic!("expected typed value comparison");
        };
        assert_eq!(*left, Value::CardsInHand(PlayerFilter::You));
    }

    #[test]
    fn normalize_rewrites_repeat_this_process_tail_into_loop_effect() {
        let effects = vec![
            EffectAst::May {
                effects: vec![EffectAst::subject_verb(
                    crate::cards::builders::SubjectVerbRoleAst::AffectedPlayer,
                    PlayerAst::You,
                    crate::cards::builders::SubjectVerbActionAst::Draw {
                        count: Value::Fixed(1),
                    },
                )],
            },
            EffectAst::IfResult {
                predicate: IfResultPredicate::Did,
                effects: vec![
                    EffectAst::subject_verb(
                        crate::cards::builders::SubjectVerbRoleAst::AffectedPlayer,
                        PlayerAst::You,
                        crate::cards::builders::SubjectVerbActionAst::GainLife {
                            amount: Value::Fixed(1),
                        },
                    ),
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

    #[test]
    fn normalize_rewrites_optional_repeat_this_process_tail_into_loop_effect() {
        let effects = vec![
            EffectAst::subject_verb(
                crate::cards::builders::SubjectVerbRoleAst::AffectedPlayer,
                PlayerAst::You,
                crate::cards::builders::SubjectVerbActionAst::Draw {
                    count: Value::Fixed(1),
                },
            ),
            EffectAst::subject_verb(
                crate::cards::builders::SubjectVerbRoleAst::AffectedPlayer,
                PlayerAst::You,
                crate::cards::builders::SubjectVerbActionAst::LoseLife {
                    amount: Value::Fixed(1),
                },
            ),
            EffectAst::RepeatThisProcessMay,
        ];

        let normalized = normalize_effects_ast(&effects);
        assert!(matches!(
            normalized.as_slice(),
            [EffectAst::RepeatProcess {
                continue_effect_index: 2,
                continue_predicate: IfResultPredicate::Did,
                ..
            }]
        ));
    }
}
