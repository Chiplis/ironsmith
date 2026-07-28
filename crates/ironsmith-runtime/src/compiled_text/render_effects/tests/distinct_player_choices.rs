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
