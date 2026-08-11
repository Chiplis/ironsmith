use super::*;

fn search_parts() -> (
    crate::effects::ChooseObjectsEffect,
    crate::effects::ConditionalEffect,
    crate::effects::ShuffleLibraryEffect,
) {
    let tag = TagKey::from("searched");
    let choose = crate::effects::ChooseObjectsEffect::new(
        ObjectFilter::default()
            .in_zone(Zone::Library)
            .owned_by(PlayerFilter::You),
        ChoiceCount::exactly(1),
        PlayerFilter::You,
        tag.clone(),
    )
    .in_zone(Zone::Library)
    .as_search();
    let battlefield_move = Effect::new(crate::effects::MoveToZoneEffect::new(
        ChooseSpec::Tagged(tag.clone()),
        Zone::Battlefield,
        false,
    ));
    let hand_move = Effect::new(crate::effects::MoveToZoneEffect::new(
        ChooseSpec::Tagged(tag.clone()),
        Zone::Hand,
        false,
    ));
    let conditional = crate::effects::ConditionalEffect::new(
        Condition::TaggedObjectMatches(tag, ObjectFilter::artifact()),
        vec![Effect::new(crate::effects::MayEffect::new(vec![
            battlefield_move,
        ]))],
        vec![hand_move],
    );
    let shuffle = crate::effects::ShuffleLibraryEffect::new(PlayerFilter::You);
    (choose, conditional, shuffle)
}

#[test]
fn nonrevealing_search_keeps_conditional_may_and_otherwise_partition() {
    let (choose, conditional, shuffle) = search_parts();
    assert_eq!(
        super::super::continuous_and_choices::describe_search_conditional_may_battlefield_else_hand_then_shuffle(
            &choose,
            &conditional,
            &shuffle,
        )
        .as_deref(),
        Some(
            "Search your library for a card. If it's an artifact card, you may put it onto the battlefield. Otherwise, put that card into your hand. Then shuffle"
        )
    );
}

#[test]
fn revealing_search_does_not_claim_the_nonrevealing_partition_surface() {
    let (mut choose, conditional, shuffle) = search_parts();
    choose.reveal = true;
    assert!(
        super::super::continuous_and_choices::describe_search_conditional_may_battlefield_else_hand_then_shuffle(
            &choose,
            &conditional,
            &shuffle,
        )
        .is_none()
    );
}
