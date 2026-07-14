use super::*;

#[test]
pub(super) fn modal_header_tracks_distinct_player_targets_per_mode() {
    let header = parse_modal_header_for_test(
        "When this creature enters, choose one or both. Each mode must target a different player.",
    )
    .expect("modal header should parse")
    .expect("modal header should be recognized");

    assert_eq!(header.min, Value::Fixed(1));
    assert_eq!(header.max, Some(Value::Fixed(2)));
    assert!(header.distinct_player_targets_per_mode, "{header:#?}");
}

#[test]
pub(super) fn modal_distinct_player_rule_lowers_to_choose_mode_metadata()
-> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Distinct Player Modal Variant")
        .card_types(vec![CardType::Creature]);
    let (definition, _) = parse_text_with_annotations_lowered(
        builder,
        "When this creature enters, choose one or both. Each mode must target a different player.\n\
         • Target player draws a card.\n\
         • Target player loses 1 life."
            .to_string(),
        false,
    )?;
    let modal = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => triggered
                .effects
                .flattened_default_effects()
                .iter()
                .find_map(|effect| effect.downcast_ref::<crate::effects::ChooseModeEffect>()),
            _ => None,
        })
        .expect("trigger should lower to a modal effect");

    assert_eq!(modal.min_choose_count, Value::Fixed(1));
    assert_eq!(modal.choose_count, Value::Fixed(2));
    assert!(modal.distinct_player_targets_per_mode, "{modal:#?}");
    Ok(())
}
