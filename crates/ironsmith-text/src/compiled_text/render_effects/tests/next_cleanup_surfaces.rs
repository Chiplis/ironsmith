use super::*;

#[test]
fn one_shot_next_cleanup_keeps_action_first_and_plural_tagged_surface() {
    let created = TagKey::from("created_0");
    let exile =
        crate::effects::MoveToZoneEffect::new(ChooseSpec::Tagged(created), Zone::Exile, true)
            .with_target_plural_surface();
    let schedule = crate::effects::ScheduleDelayedTriggerEffect::new(
        crate::triggers::Trigger::beginning_of_next_cleanup_step(PlayerFilter::Any),
        vec![Effect::new(exile)],
        true,
        Vec::new(),
        PlayerFilter::You,
    );

    assert_eq!(
        describe_effect(&Effect::new(schedule)),
        "Exile them at the beginning of the next cleanup step"
    );
}
