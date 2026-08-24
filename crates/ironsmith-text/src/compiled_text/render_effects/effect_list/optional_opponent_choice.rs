use super::super::*;

fn exact_optional_sacrifice_or_discard(
    effect: &Effect,
) -> Option<&crate::effects::VillainousChoiceEffect> {
    let with_id = effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let may = with_id.effect.downcast_ref::<crate::effects::MayEffect>()?;
    if may.decider != Some(PlayerFilter::IteratedPlayer)
        || may.fallback != crate::decision::FallbackStrategy::Decline
    {
        return None;
    }
    let [choice_effect] = may.effects.as_slice() else {
        return None;
    };
    let choice = choice_effect.downcast_ref::<crate::effects::VillainousChoiceEffect>()?;
    if choice.player != PlayerFilter::IteratedPlayer
        || choice.player_surface.is_some()
        || choice.modes.len() != 2
    {
        return None;
    }

    let expected_filter = ObjectFilter::nonland()
        .in_zone(Zone::Battlefield)
        .controlled_by(PlayerFilter::IteratedPlayer);
    match choice.modes[0].effects.as_slice() {
        [sacrifice_effect] => {
            let sacrifice = sacrifice_view(sacrifice_effect)?;
            if sacrifice.player != &PlayerFilter::IteratedPlayer
                || sacrifice.count != &Value::Fixed(1)
                || sacrifice.filter != &expected_filter
            {
                return None;
            }
        }
        [choose_effect, sacrifice_effect] => {
            let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
            if choose.filter != expected_filter
                || choose.count != crate::effect::ChoiceCount::exactly(1)
                || choose.count_value.is_some()
                || choose.aggregate_constraint.is_some()
                || choose.chooser != PlayerFilter::IteratedPlayer
                || choose.zone.is_some()
                || !choose.additional_zones.is_empty()
                || choose.is_search
                || choose.reveal
                || choose.top_only
                || choose.bottom_only
                || choose.replace_tagged_objects
                || choose.remember_as_chosen_object
            {
                return None;
            }
            let sacrifice = sacrifice_view(sacrifice_effect)?;
            let mut selected_filter = ObjectFilter::default();
            selected_filter
                .tagged_constraints
                .push(crate::filter::TaggedObjectConstraint {
                    tag: choose.tag.clone(),
                    relation: crate::filter::TaggedOpbjectRelation::IsTaggedObject,
                });
            if sacrifice.player != &PlayerFilter::IteratedPlayer
                || sacrifice.count != &Value::Fixed(1)
                || sacrifice.filter != &selected_filter
            {
                return None;
            }
        }
        _ => return None,
    }

    let [discard_effect] = choice.modes[1].effects.as_slice() else {
        return None;
    };
    let discard = discard_effect.downcast_ref::<crate::effects::DiscardEffect>()?;
    if discard.player != PlayerFilter::IteratedPlayer
        || discard.count != Value::Fixed(1)
        || discard.random
        || discard.any_number
        || discard.card_filter.is_some()
    {
        return None;
    }
    Some(choice)
}

fn is_source_power_damage_to_iterated_player(effect: &Effect) -> bool {
    let Some(execute) = effect.downcast_ref::<crate::effects::ExecuteWithSourceEffect>() else {
        return false;
    };
    if !matches!(execute.source.base(), ChooseSpec::Source) {
        return false;
    }
    let Some(damage) = execute
        .effect
        .downcast_ref::<crate::effects::DealDamageEffect>()
    else {
        return false;
    };
    !damage.source_is_combat
        && !damage.unpreventable
        && matches!(
            damage.target.base(),
            ChooseSpec::Player(PlayerFilter::IteratedPlayer)
        )
        && matches!(
            damage.amount.unhinted(),
            Value::PowerOf(source) if matches!(source.base(), ChooseSpec::Source)
        )
}

/// Preserve an optional per-opponent sacrifice-or-discard choice together
/// with the result branch for the same opponent. The effect ID and the
/// iterated-player frame are executable provenance, not merely prose.
pub(super) fn describe_each_opponent_optional_sacrifice_or_discard_then_damage(
    effects: &[Effect],
) -> Option<String> {
    let [for_players_effect] = effects else {
        return None;
    };
    let for_players = for_players_effect.downcast_ref::<crate::effects::ForPlayersEffect>()?;
    if for_players.filter != PlayerFilter::Opponent
        || for_players.starting_with_controller
        || for_players.stop_after_first_happened
    {
        return None;
    }
    let [choice_effect, conditional_effect] = for_players.effects.as_slice() else {
        return None;
    };
    let with_id = choice_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    exact_optional_sacrifice_or_discard(choice_effect)?;
    let conditional = conditional_effect.downcast_ref::<crate::effects::IfEffect>()?;
    if conditional.condition != with_id.id
        || conditional.predicate != EffectPredicate::DidNotHappen
        || !conditional.else_.is_empty()
        || conditional.per_player_result
        || conditional.prior_result_replacement_surface
    {
        return None;
    }
    let [damage_effect] = conditional.then.as_slice() else {
        return None;
    };
    if !is_source_power_damage_to_iterated_player(damage_effect) {
        return None;
    }

    Some("Each opponent may sacrifice a nonland permanent of their choice or discard a card. Then this creature deals damage equal to its power to each opponent who didn't sacrifice a permanent or discard a card this way".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effect::{EffectId, EffectMode};

    fn program(
        loop_filter: PlayerFilter,
        conditional_id: EffectId,
        link_selected_sacrifice: bool,
    ) -> Vec<Effect> {
        let modes = vec![
            EffectMode {
                source_text: "Sacrifice a nonland permanent".to_string(),
                effects: {
                    let tag = crate::tag::TagKey::from("sacrificed_0");
                    let choose = Effect::new(crate::effects::ChooseObjectsEffect::new(
                        ObjectFilter::nonland()
                            .in_zone(Zone::Battlefield)
                            .controlled_by(PlayerFilter::IteratedPlayer),
                        crate::effect::ChoiceCount::exactly(1),
                        PlayerFilter::IteratedPlayer,
                        tag.clone(),
                    ));
                    let mut selected = ObjectFilter::default();
                    if link_selected_sacrifice {
                        selected
                            .tagged_constraints
                            .push(crate::filter::TaggedObjectConstraint {
                                tag,
                                relation: crate::filter::TaggedOpbjectRelation::IsTaggedObject,
                            });
                    }
                    vec![
                        choose,
                        Effect::sacrifice_player(selected, 1, PlayerFilter::IteratedPlayer),
                    ]
                },
            },
            EffectMode {
                source_text: "Discard a card".to_string(),
                effects: vec![Effect::discard_player(
                    1,
                    PlayerFilter::IteratedPlayer,
                    false,
                )],
            },
        ];
        let choice = Effect::villainous_choice(PlayerFilter::IteratedPlayer, None, modes);
        let offer = Effect::with_id(
            7,
            Effect::may_player(PlayerFilter::IteratedPlayer, vec![choice]),
        );
        let damage = Effect::new(crate::effects::ExecuteWithSourceEffect::new(
            ChooseSpec::Source,
            Effect::deal_damage(
                Value::PowerOf(Box::new(ChooseSpec::Source)),
                ChooseSpec::Player(PlayerFilter::IteratedPlayer),
            ),
        ));
        let failure = Effect::if_then(conditional_id, EffectPredicate::DidNotHappen, vec![damage]);
        vec![Effect::for_players(loop_filter, vec![offer, failure])]
    }

    fn exact_program() -> Vec<Effect> {
        program(PlayerFilter::Opponent, EffectId(7), true)
    }

    #[test]
    fn renders_exact_correlated_opponent_choice_and_failure() {
        assert_eq!(
            describe_each_opponent_optional_sacrifice_or_discard_then_damage(&exact_program()),
            Some("Each opponent may sacrifice a nonland permanent of their choice or discard a card. Then this creature deals damage equal to its power to each opponent who didn't sacrifice a permanent or discard a card this way".to_string())
        );
    }

    #[test]
    fn rejects_changed_participant_or_result_id() {
        let changed_player = program(PlayerFilter::Any, EffectId(7), true);
        assert!(
            describe_each_opponent_optional_sacrifice_or_discard_then_damage(&changed_player)
                .is_none()
        );

        let changed_id = program(PlayerFilter::Opponent, EffectId(8), true);
        assert!(
            describe_each_opponent_optional_sacrifice_or_discard_then_damage(&changed_id).is_none()
        );
    }

    #[test]
    fn rejects_sacrifice_that_is_not_linked_to_the_selected_permanent() {
        let effects = program(PlayerFilter::Opponent, EffectId(7), false);
        assert!(
            describe_each_opponent_optional_sacrifice_or_discard_then_damage(&effects).is_none()
        );
    }
}
