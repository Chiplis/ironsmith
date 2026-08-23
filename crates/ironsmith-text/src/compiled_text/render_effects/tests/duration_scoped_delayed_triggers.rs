use super::*;

#[test]
fn until_controller_next_turn_delayed_triggers_render_as_duration_scoped_rules() {
    let triggering = TagKey::from("triggering");

    let tamiyo = crate::effects::ScheduleDelayedTriggerEffect::new(
        crate::triggers::Trigger::deals_combat_damage(ObjectFilter::source()),
        vec![Effect::draw(1)],
        false,
        Vec::new(),
        PlayerFilter::You,
    )
    .with_target_filter(ObjectFilter::creature())
    .watch_all_object_targets()
    .with_either_of_watched_objects_surface()
    .until_controller_next_turn();
    assert_eq!(
        describe_effect(&Effect::new(tamiyo)),
        "Until your next turn, whenever either of those creatures deals combat damage, you draw a card"
    );

    let dont_move = crate::effects::ScheduleDelayedTriggerEffect::new(
        crate::triggers::Trigger::permanent_becomes_tapped(ObjectFilter::creature()),
        vec![Effect::destroy(ChooseSpec::Tagged(triggering.clone()))],
        false,
        Vec::new(),
        PlayerFilter::You,
    )
    .until_controller_next_turn();
    assert_eq!(
        describe_effect(&Effect::new(dont_move)),
        "Until your next turn, whenever creature becomes tapped, destroy it"
    );

    let vraska = crate::effects::ScheduleDelayedTriggerEffect::new(
        crate::triggers::Trigger::deals_combat_damage_to(
            ObjectFilter::creature(),
            ObjectFilter::source_with_surface(
                crate::target::SourceReferenceSurface::ThisPermanentType(
                    "this permanent".to_string(),
                ),
            ),
        ),
        vec![Effect::destroy(ChooseSpec::Tagged(triggering))],
        false,
        Vec::new(),
        PlayerFilter::You,
    )
    .watch_ability_source()
    .until_controller_next_turn();
    assert_eq!(
        describe_effect(&Effect::new(vraska)),
        "Until your next turn, whenever creature deals combat damage to this permanent, destroy it"
    );
}

#[test]
fn leading_end_of_turn_surface_stays_before_the_delayed_trigger() {
    let schedule = crate::effects::ScheduleDelayedTriggerEffect::new(
        crate::triggers::Trigger::spell_cast(None, PlayerFilter::You),
        vec![Effect::draw(1)],
        false,
        Vec::new(),
        PlayerFilter::You,
    )
    .until_end_of_turn()
    .with_leading_duration_surface();

    assert_eq!(
        describe_effect(&Effect::new(schedule)),
        "Until end of turn, whenever you cast a spell, you draw a card"
    );
}

#[test]
fn event_scoped_end_of_turn_trigger_keeps_its_this_turn_suffix() {
    let schedule = crate::effects::ScheduleDelayedTriggerEffect::new(
        crate::triggers::Trigger::spell_cast(None, PlayerFilter::You),
        vec![Effect::draw(1)],
        false,
        Vec::new(),
        PlayerFilter::You,
    )
    .until_end_of_turn();

    assert_eq!(
        describe_effect(&Effect::new(schedule)),
        "Whenever you cast a spell this turn, you draw a card"
    );
}

#[test]
fn permanent_delayed_trigger_does_not_invent_a_duration() {
    let schedule = crate::effects::ScheduleDelayedTriggerEffect::new(
        crate::triggers::Trigger::spell_cast(None, PlayerFilter::You),
        vec![Effect::draw(1)],
        false,
        Vec::new(),
        PlayerFilter::You,
    );

    assert_eq!(
        describe_effect(&Effect::new(schedule)),
        "Whenever you cast a spell, you draw a card"
    );
}
