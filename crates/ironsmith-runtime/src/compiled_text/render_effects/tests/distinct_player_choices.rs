use super::*;

fn choose_player(tag: &str, excluded_tags: &[&str]) -> Effect {
    Effect::new(
        crate::effects::ChoosePlayerEffect::new(PlayerFilter::You, PlayerFilter::Any, tag)
            .excluding_tags(excluded_tags.iter().map(|tag| TagKey::from(*tag)).collect()),
    )
}

#[test]
fn excluded_player_tags_render_an_ordinal_and_compact_the_linked_action() {
    let effects = vec![
        choose_player("chosen_player_1", &["chosen_player_0"]),
        Effect::new(crate::effects::DrawCardsEffect::new(
            1,
            PlayerFilter::TaggedPlayer("__it__".into()),
        )),
    ];

    assert_eq!(
        describe_distinct_player_choice_with_linked_action(&effects).as_deref(),
        Some("Choose a second player to draw a card")
    );
}

#[test]
fn sentence_leading_then_preserves_the_third_player_token_action() {
    let effects = vec![Effect::new(
        crate::effects::SequenceEffect::sentence_leading_then(vec![
            choose_player("chosen_player_2", &["chosen_player_0", "chosen_player_1"]),
            Effect::new(crate::effects::CreateTokenEffect::new(
                crate::cards::tokens::treasure_token_definition(),
                2,
                PlayerFilter::TaggedPlayer("__it__".into()),
            )),
        ]),
    )];

    assert_eq!(
        describe_distinct_player_choice_with_linked_action(&effects).as_deref(),
        Some("Then choose a third player to create two Treasure tokens")
    );
}

#[test]
fn a_different_player_tag_does_not_claim_the_linked_surface() {
    let effects = vec![
        choose_player("chosen_player_1", &["chosen_player_0"]),
        Effect::new(crate::effects::DrawCardsEffect::new(
            1,
            PlayerFilter::TaggedPlayer("some_other_player".into()),
        )),
    ];

    assert_eq!(
        describe_distinct_player_choice_with_linked_action(&effects),
        None
    );
}

#[test]
fn first_player_counter_choice_keeps_that_player_as_the_action_actor() {
    let mut creature = ObjectFilter::default();
    creature.zone = Some(Zone::Battlefield);
    creature.controller = Some(PlayerFilter::TaggedPlayer("__it__".into()));
    creature.card_types.push(CardType::Creature);
    let effects = vec![
        choose_player("chosen_player_0", &[]),
        Effect::new(crate::effects::PutCountersEffect::plus_one_counters(
            2,
            ChooseSpec::Object(creature).with_count(ChoiceCount::exactly(1)),
        )),
    ];

    assert_eq!(
        describe_distinct_player_choice_with_linked_action(&effects).as_deref(),
        Some("Choose a player. They put two +1/+1 counters on a creature they control")
    );
}
