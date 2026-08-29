use super::*;

#[test]
fn creature_and_color_choices_return_typed_exclusions() {
    let creature = parse_choice_creature_type_phrase_words(&[
        "choose", "a", "creature", "type", "other", "than", "dragon", "now",
    ])
    .unwrap()
    .unwrap();
    assert_eq!(creature.consumed, 7);
    assert_eq!(creature.excluded_subtypes, [Subtype::Dragon]);

    let color =
        parse_choice_color_phrase_words(&["choose", "a", "color", "other", "than", "blue", "now"])
            .unwrap()
            .unwrap();
    assert_eq!(color.consumed, 6);
    assert_eq!(color.excluded, Some(ColorSet::BLUE));
}

#[test]
fn card_type_choices_return_typed_options() {
    let parsed = parse_choice_card_type_phrase_words(&[
        "choose", "artifact", "creature", "or", "land", "now",
    ])
    .unwrap();
    assert_eq!(parsed.consumed, 5);
    assert_eq!(
        parsed.options,
        [CardType::Artifact, CardType::Creature, CardType::Land]
    );
}

#[test]
fn simple_choice_phrases_report_consumed_words() {
    assert_eq!(
        parse_choice_basic_land_type_phrase_words(&["choose", "a", "basic", "land", "type", "now"])
            .unwrap()
            .consumed,
        5
    );
    assert_eq!(
        parse_choice_player_phrase_words(&["choose", "a", "player", "now"])
            .unwrap()
            .consumed,
        3
    );

    let nonbasic =
        parse_choice_land_type_phrase_words(&["choose", "a", "nonbasic", "land", "type", "now"])
            .unwrap();
    assert_eq!(nonbasic.consumed, 5);
    assert!(nonbasic.exclude_basic);

    let unrestricted =
        parse_choice_land_type_phrase_words(&["choose", "a", "land", "type", "now"]).unwrap();
    assert_eq!(unrestricted.consumed, 4);
    assert!(!unrestricted.exclude_basic);

    let planeswalker =
        parse_choice_subtype_family_phrase_words(&["choose", "a", "planeswalker", "type", "now"])
            .unwrap();
    assert_eq!(planeswalker.consumed, 4);
    assert_eq!(planeswalker.family, SubtypeFamily::Planeswalker);
    assert_eq!(
        parse_choice_subtype_family_phrase_words(&["choose", "a", "planeswalker"]),
        None,
        "choosing a planeswalker object is not choosing a planeswalker type"
    );
}
