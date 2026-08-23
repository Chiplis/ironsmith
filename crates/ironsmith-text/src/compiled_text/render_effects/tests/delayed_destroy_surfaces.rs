use super::*;

fn destroy_all_permanents() -> Effect {
    let filter = ObjectFilter::permanent_card().in_zone(Zone::Battlefield);
    Effect::new(crate::effects::DestroyEffect::all(filter))
}

#[test]
fn complete_permanent_type_set_uses_the_permanent_noun() {
    assert_eq!(
        describe_effect(&destroy_all_permanents()),
        "Destroy all permanents"
    );
}

#[test]
fn one_shot_end_step_destroy_keeps_action_first_timing() {
    let schedule = crate::effects::ScheduleDelayedTriggerEffect::new(
        crate::triggers::Trigger::beginning_of_end_step(PlayerFilter::Any),
        vec![destroy_all_permanents()],
        true,
        Vec::new(),
        PlayerFilter::You,
    );

    assert_eq!(
        describe_effect(&Effect::new(schedule)),
        "Destroy all permanents at the beginning of the next end step"
    );
}

#[test]
fn one_shot_end_step_phase_out_keeps_action_first_timing() {
    let mut filter = ObjectFilter::default()
        .with_type(CardType::Planeswalker)
        .controlled_by(PlayerFilter::You)
        .other();
    filter.zone = Some(Zone::Battlefield);
    let target = ChooseSpec::target(ChooseSpec::Object(filter))
        .with_count(crate::effect::ChoiceCount::up_to(2));
    let schedule = crate::effects::ScheduleDelayedTriggerEffect::new(
        crate::triggers::Trigger::beginning_of_end_step(PlayerFilter::Any),
        vec![Effect::new(crate::effects::PhaseOutEffect::with_spec(
            target,
        ))],
        true,
        Vec::new(),
        PlayerFilter::You,
    );

    assert_eq!(
        describe_effect(&Effect::new(schedule)),
        "Phase out up to two other target planeswalkers you control at the beginning of the next end step"
    );
}
