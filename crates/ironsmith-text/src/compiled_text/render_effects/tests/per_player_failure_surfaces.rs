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
            "each player may put a land card from their hand onto the battlefield, then each opponent who didn't draws a card"
        )
    );
}

#[test]
fn skull_storm_keeps_rounded_half_life_failure_followup() {
    let oracle = "When you cast this spell, copy it for each time you've cast your commander from the command zone this game.\nEach opponent sacrifices a creature of their choice. Each opponent who can't loses half their life, rounded up.";
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Skull Storm")
        .card_types(vec![CardType::Sorcery])
        .parse_text(oracle)
        .expect("Skull Storm should compile");

    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        oracle
    );
    let debug = format!("{definition:#?}");
    assert!(debug.contains("DidNotHappen"), "{debug}");
    assert!(debug.contains("HalfLifeTotalRoundedUp(\n"), "{debug}");
    assert!(debug.contains("IteratedPlayer"), "{debug}");
}
