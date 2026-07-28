use super::*;

#[test]
fn split_per_player_failure_branch_keeps_the_prior_action_and_partition() {
    let action_id = crate::effect::EffectId(17);
    let discard = Effect::with_id(
        action_id.0,
        Effect::for_players(
            PlayerFilter::Any,
            vec![Effect::new(crate::effects::DiscardEffect::new(
                1,
                PlayerFilter::IteratedPlayer,
                false,
            ))],
        ),
    );
    let failure = Effect::for_players(
        PlayerFilter::Opponent,
        vec![Effect::if_then(
            action_id,
            EffectPredicate::DidNotHappen,
            vec![Effect::new(crate::effects::LoseLifeEffect::with_filter(
                3,
                PlayerFilter::IteratedPlayer,
            ))],
        )],
    );

    assert_eq!(
        describe_effect_clause_list(&[discard, failure]).as_deref(),
        Some("each player discards a card. Each opponent who can't loses 3 life")
    );
}

#[test]
fn explicit_iterated_decider_is_the_same_per_player_may_subject() {
    let action_id = crate::effect::EffectId(23);
    let tag = TagKey::from("chosen");
    let choose = Effect::new(crate::effects::ChooseObjectsEffect::new(
        ObjectFilter::land()
            .in_zone(Zone::Hand)
            .owned_by(PlayerFilter::IteratedPlayer),
        ChoiceCount::exactly(1),
        PlayerFilter::IteratedPlayer,
        tag.clone(),
    ));
    let move_chosen = Effect::new(
        crate::effects::MoveToZoneEffect::new(ChooseSpec::Tagged(tag), Zone::Battlefield, false)
            .with_actor_surface(PlayerFilter::IteratedPlayer),
    );
    let action = Effect::with_id(
        action_id.0,
        Effect::for_players(
            PlayerFilter::Any,
            vec![Effect::new(crate::effects::MayEffect::new_for_player(
                vec![choose, move_chosen],
                PlayerFilter::IteratedPlayer,
            ))],
        ),
    );
    let failure = Effect::for_players(
        PlayerFilter::Opponent,
        vec![Effect::if_then(
            action_id,
            EffectPredicate::DidNotHappen,
            vec![Effect::new(crate::effects::DrawCardsEffect::new(
                1,
                PlayerFilter::IteratedPlayer,
            ))],
        )],
    );

    assert_eq!(
        describe_effect_clause_list(&[action, failure]).as_deref(),
        Some(
            "each player may put a land card from their hand onto the battlefield. For each opponent who doesn't, that player draws a card"
        )
    );
}
