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

#[test]
fn linked_exile_duration_return_has_authored_delayed_surface() {
    let tag = ironsmith_core::SOURCE_EXILED_THIS_RESOLUTION_TAG;
    let trigger = crate::triggers::ZoneChangeTrigger::new()
        .from(crate::zone::Zone::Exile)
        .to(crate::zone::Zone::Battlefield)
        .filter(
            ObjectFilter::default()
                .controlled_by(PlayerFilter::You)
                .match_tagged(tag, TaggedOpbjectRelation::IsTaggedObject),
        )
        .duration_return_from_source();
    let counters = Effect::put_counters(
        crate::CounterType::PlusOnePlusOne,
        2,
        ChooseSpec::Tagged(TagKey::from("triggering")),
    );
    let schedule = crate::effects::ScheduleDelayedTriggerEffect::from_tag(
        crate::triggers::Trigger::new(trigger.clone()),
        vec![counters.clone()],
        true,
        tag,
        PlayerFilter::You,
    )
    .watch_ability_source();
    assert_eq!(
        describe_effect(&Effect::new(schedule)),
        "When an exiled card enters under your control this way, put two +1/+1 counters on it"
    );

    let authored = crate::effects::ScheduleDelayedTriggerEffect::from_tag(
        crate::triggers::Trigger::new(trigger.clone()),
        vec![counters.clone()],
        true,
        tag,
        PlayerFilter::You,
    )
    .watch_ability_source()
    .with_leading_duration_surface();
    assert_eq!(
        describe_effect(&Effect::new(authored)),
        "When an exiled card enters under your control this way, put two +1/+1 counters on it",
        "the lowering presentation bit must not discard the duration provenance"
    );

    // An ordinary exile-to-battlefield watcher must not claim the linked
    // duration's "this way" provenance.
    let ordinary = crate::effects::ScheduleDelayedTriggerEffect::from_tag(
        crate::triggers::Trigger::new(
            crate::triggers::ZoneChangeTrigger::new()
                .from(crate::zone::Zone::Exile)
                .to(crate::zone::Zone::Battlefield)
                .filter(
                    ObjectFilter::default()
                        .controlled_by(PlayerFilter::You)
                        .match_tagged(tag, TaggedOpbjectRelation::IsTaggedObject),
                ),
        ),
        vec![counters],
        true,
        tag,
        PlayerFilter::You,
    )
    .watch_ability_source();
    assert!(!describe_effect(&Effect::new(ordinary)).contains("this way"));
}
