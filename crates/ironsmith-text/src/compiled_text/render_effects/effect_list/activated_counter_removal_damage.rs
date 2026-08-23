use super::*;

fn wrapped_effect_id(effect: &Effect) -> Option<crate::effect::EffectId> {
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return Some(with_id.id);
    }
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return wrapped_effect_id(&tagged.effect);
    }
    if let Some(tag_all) = effect.downcast_ref::<crate::effects::TagAllEffect>() {
        return wrapped_effect_id(&tag_all.effect);
    }
    None
}

fn exact_removed_counter_metric(
    value: &Value,
    removal_id: crate::effect::EffectId,
    counter_type: crate::object::CounterType,
) -> bool {
    matches!(
        value.unhinted(),
        Value::PriorEffectMetric { effect_id, query }
            if *effect_id == removal_id
                && query.source == crate::effect::EffectMetricSource::Outcome
                && query.metric == crate::effect::EffectMetric::Count
                && query.filter.is_none()
                && query.player.is_none()
                && query.action == Some(crate::effect::PriorEffectAction::Removed)
                && query.counter_type == Some(counter_type)
    )
}

fn exact_each_creature_damage(
    effect: &Effect,
    removal_id: crate::effect::EffectId,
    counter_type: crate::object::CounterType,
) -> bool {
    let Some(for_each) =
        structural_unwrap_render_wrappers(effect).downcast_ref::<crate::effects::ForEachObject>()
    else {
        return false;
    };
    let expected = ObjectFilter::creature().in_zone(Zone::Battlefield);
    let [damage_effect] = for_each.effects.as_slice() else {
        return false;
    };
    let Some(damage) = structural_unwrap_render_wrappers(damage_effect)
        .downcast_ref::<crate::effects::DealDamageEffect>()
    else {
        return false;
    };
    for_each.filter == expected
        && matches!(damage.target.base(), ChooseSpec::Iterated)
        && !damage.source_is_combat
        && !damage.unpreventable
        && exact_removed_counter_metric(&damage.amount, removal_id, counter_type)
}

fn exact_each_player_damage(
    effect: &Effect,
    removal_id: crate::effect::EffectId,
    counter_type: crate::object::CounterType,
) -> bool {
    let Some(for_players) = structural_unwrap_render_wrappers(effect)
        .downcast_ref::<crate::effects::ForPlayersEffect>()
    else {
        return false;
    };
    let [damage_effect] = for_players.effects.as_slice() else {
        return false;
    };
    let Some(damage) = structural_unwrap_render_wrappers(damage_effect)
        .downcast_ref::<crate::effects::DealDamageEffect>()
    else {
        return false;
    };
    for_players.filter == PlayerFilter::Any
        && !for_players.starting_with_controller
        && !for_players.stop_after_first_happened
        && matches!(
            damage.target.base(),
            ChooseSpec::Player(PlayerFilter::IteratedPlayer)
        )
        && !damage.source_is_combat
        && !damage.unpreventable
        && exact_removed_counter_metric(&damage.amount, removal_id, counter_type)
}

fn exact_player_partitioned_creature_and_player_damage(
    effect: &Effect,
    removal_id: crate::effect::EffectId,
    counter_type: crate::object::CounterType,
) -> bool {
    let Some(for_players) = structural_unwrap_render_wrappers(effect)
        .downcast_ref::<crate::effects::ForPlayersEffect>()
    else {
        return false;
    };
    let [player_damage_effect, creature_fanout_effect] = for_players.effects.as_slice() else {
        return false;
    };
    let Some(player_damage) = structural_unwrap_render_wrappers(player_damage_effect)
        .downcast_ref::<crate::effects::DealDamageEffect>()
    else {
        return false;
    };
    let Some(creature_fanout) = structural_unwrap_render_wrappers(creature_fanout_effect)
        .downcast_ref::<crate::effects::ForEachObject>()
    else {
        return false;
    };
    let [creature_damage_effect] = creature_fanout.effects.as_slice() else {
        return false;
    };
    let Some(creature_damage) = structural_unwrap_render_wrappers(creature_damage_effect)
        .downcast_ref::<crate::effects::DealDamageEffect>()
    else {
        return false;
    };
    let mut expected_creatures = ObjectFilter::creature().in_zone(Zone::Battlefield);
    expected_creatures.controller = Some(PlayerFilter::IteratedPlayer);

    for_players.filter == PlayerFilter::Any
        && !for_players.starting_with_controller
        && !for_players.stop_after_first_happened
        && matches!(
            player_damage.target.base(),
            ChooseSpec::Player(PlayerFilter::IteratedPlayer)
        )
        && !player_damage.source_is_combat
        && !player_damage.unpreventable
        && exact_removed_counter_metric(&player_damage.amount, removal_id, counter_type)
        && creature_fanout.filter == expected_creatures
        && matches!(creature_damage.target.base(), ChooseSpec::Iterated)
        && !creature_damage.source_is_combat
        && !creature_damage.unpreventable
        && exact_removed_counter_metric(&creature_damage.amount, removal_id, counter_type)
}

/// Restore the authored single conditional result clause for an activated
/// ability whose exact counter-removal outcome feeds both damage recipients.
/// The effect ID, typed `Removed` metric, counter kind, condition, and both
/// complete recipient domains are required, so unrelated damage fanouts do
/// not acquire the compact surface.
fn describe_activated_counter_removal_damage_with_subject(
    effects: &[Effect],
    removal_subject: Option<&str>,
) -> Option<String> {
    let [effect] = effects else {
        return None;
    };
    let conditional = structural_unwrap_render_wrappers(effect)
        .downcast_ref::<crate::effects::ConditionalEffect>()?;
    let Condition::ThisAbilityResolvedThisTurnExactly(_) = &conditional.condition else {
        return None;
    };
    if conditional.surface != ironsmith_core::ConditionalSurface::LeadingIf
        || !conditional.if_false.is_empty()
    {
        return None;
    }
    let [sequence_effect] = conditional.if_true.as_slice() else {
        return None;
    };
    let sequence = structural_unwrap_render_wrappers(sequence_effect)
        .downcast_ref::<crate::effects::SequenceEffect>()?;
    if sequence.surface != ironsmith_core::SequenceSurface::Coordinated {
        return None;
    }
    let Some((remove_effect, damage_effects)) = sequence.effects.split_first() else {
        return None;
    };
    let removal_id = wrapped_effect_id(remove_effect)?;
    let remove = structural_unwrap_render_wrappers(remove_effect)
        .downcast_ref::<crate::effects::RemoveCountersEffect>()?;
    if !matches!(remove.target.base(), ChooseSpec::Source)
        || !matches!(
            remove.count.unhinted(),
            Value::CountersOnSource(counter_type) if *counter_type == remove.counter_type
        )
    {
        return None;
    }
    let exact_damage_fanout = match damage_effects {
        [creature_damage, player_damage] => {
            exact_each_creature_damage(creature_damage, removal_id, remove.counter_type)
                && exact_each_player_damage(player_damage, removal_id, remove.counter_type)
        }
        [partitioned_damage] => exact_player_partitioned_creature_and_player_damage(
            partitioned_damage,
            removal_id,
            remove.counter_type,
        ),
        _ => false,
    };
    if !exact_damage_fanout {
        return None;
    }

    Some(format!(
        "If {}, remove all {} counters from {}, and it deals that much damage to each creature and each player",
        describe_condition(&conditional.condition),
        describe_counter_type(remove.counter_type),
        removal_subject
            .map(str::to_string)
            .unwrap_or_else(|| describe_choose_spec(&remove.target))
    ))
}

pub(super) fn describe_activated_counter_removal_damage(effects: &[Effect]) -> Option<String> {
    describe_activated_counter_removal_damage_with_subject(effects, None)
}

pub(in crate::compiled_text) fn describe_activated_counter_removal_damage_with_source_surface(
    effects: &[Effect],
    source_surface: &str,
) -> Option<String> {
    if source_surface.trim().is_empty() {
        return None;
    }
    describe_activated_counter_removal_damage_with_subject(effects, Some(source_surface.trim()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(second_damage_id: u32) -> Vec<Effect> {
        let counter_type = crate::object::CounterType::PlusOnePlusOne;
        let removal_id = crate::effect::EffectId(17);
        let source = ChooseSpec::Source.with_surface_hint(
            crate::target::ChooseSpecSurfaceHint::SourceReference(
                crate::target::SourceReferenceSurface::ShortName("Ashling".to_string()),
            ),
        );
        let remove = Effect::with_id(
            removal_id.0,
            Effect::new(crate::effects::RemoveCountersEffect::new(
                counter_type,
                Value::CountersOnSource(counter_type),
                source,
            )),
        )
        .tag("counters_0");
        let removed_count = |effect_id| Value::PriorEffectMetric {
            effect_id: crate::effect::EffectId(effect_id),
            query: crate::effect::PriorEffectMetricQuery::new(
                crate::effect::EffectMetricSource::Outcome,
                crate::effect::EffectMetric::Count,
            )
            .with_action(crate::effect::PriorEffectAction::Removed)
            .with_counter_type(Some(counter_type)),
        };
        let creature_damage = Effect::for_each(
            ObjectFilter::creature().in_zone(Zone::Battlefield),
            vec![Effect::deal_damage(
                removed_count(removal_id.0),
                ChooseSpec::Iterated,
            )],
        );
        let player_damage = Effect::for_players(
            PlayerFilter::Any,
            vec![Effect::deal_damage(
                removed_count(second_damage_id),
                ChooseSpec::Player(PlayerFilter::IteratedPlayer),
            )],
        );
        vec![Effect::new(crate::effects::ConditionalEffect::if_only(
            Condition::ThisAbilityResolvedThisTurnExactly(3),
            vec![Effect::new(crate::effects::SequenceEffect::coordinated(
                vec![remove, creature_damage, player_damage],
            ))],
        ))]
    }

    fn partitioned_fixture(second_damage_id: u32) -> Vec<Effect> {
        let counter_type = crate::object::CounterType::PlusOnePlusOne;
        let removal_id = crate::effect::EffectId(17);
        let source = ChooseSpec::Source.with_surface_hint(
            crate::target::ChooseSpecSurfaceHint::SourceReference(
                crate::target::SourceReferenceSurface::ShortName("Ashling".to_string()),
            ),
        );
        let remove = Effect::with_id(
            removal_id.0,
            Effect::new(crate::effects::RemoveCountersEffect::new(
                counter_type,
                Value::CountersOnSource(counter_type),
                source,
            )),
        )
        .tag("counters_0");
        let removed_count = |effect_id| Value::PriorEffectMetric {
            effect_id: crate::effect::EffectId(effect_id),
            query: crate::effect::PriorEffectMetricQuery::new(
                crate::effect::EffectMetricSource::Outcome,
                crate::effect::EffectMetric::Count,
            )
            .with_action(crate::effect::PriorEffectAction::Removed)
            .with_counter_type(Some(counter_type)),
        };
        let mut controlled_creatures = ObjectFilter::creature().in_zone(Zone::Battlefield);
        controlled_creatures.controller = Some(PlayerFilter::IteratedPlayer);
        let partition = Effect::for_players(
            PlayerFilter::Any,
            vec![
                Effect::deal_damage(
                    removed_count(removal_id.0),
                    ChooseSpec::Player(PlayerFilter::IteratedPlayer),
                ),
                Effect::for_each(
                    controlled_creatures,
                    vec![Effect::deal_damage(
                        removed_count(second_damage_id),
                        ChooseSpec::Iterated,
                    )],
                ),
            ],
        );
        vec![Effect::new(crate::effects::ConditionalEffect::if_only(
            Condition::ThisAbilityResolvedThisTurnExactly(3),
            vec![Effect::new(crate::effects::SequenceEffect::coordinated(
                vec![remove, partition],
            ))],
        ))]
    }

    #[test]
    fn activated_third_resolution_counter_removal_damage_is_one_clause() {
        assert_eq!(
            describe_activated_counter_removal_damage(&fixture(17)).as_deref(),
            Some(
                "If this is the third time this ability has resolved this turn, remove all +1/+1 counters from Ashling, and it deals that much damage to each creature and each player"
            )
        );
    }

    #[test]
    fn player_partitioned_fanout_renders_the_same_complete_clause() {
        assert_eq!(
            describe_activated_counter_removal_damage(&partitioned_fixture(17)).as_deref(),
            Some(
                "If this is the third time this ability has resolved this turn, remove all +1/+1 counters from Ashling, and it deals that much damage to each creature and each player"
            )
        );
    }

    #[test]
    fn activated_counter_damage_rejects_mismatched_second_producer() {
        assert!(describe_activated_counter_removal_damage(&fixture(18)).is_none());
        assert!(describe_activated_counter_removal_damage(&partitioned_fixture(18)).is_none());
    }
}
