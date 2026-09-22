use ironsmith_tools::parse_card_definition_with_runtime_builder;

#[test]
fn unscoped_card_targets_and_choices_are_rejected() {
    for text in [
        "Exile a nonland card.",
        "Exile target card.",
        "Exile all nonland cards.",
        "You may exile a nonland card.",
        "Choose a nonland card. Exile it.",
        "Look at target opponent's hand. Exile a nonland card.",
    ] {
        let error = parse_card_definition_with_runtime_builder(
            "Unscoped Selection Probe",
            format!("Type: Sorcery\n{text}"),
            false,
        )
        .expect_err(text);
        assert!(
            error
                .to_string()
                .contains("card selection has no resolved source zone or referenced collection"),
            "{text}: {error}"
        );
    }
}

#[test]
fn explicit_zones_and_permanent_defaults_still_compile() {
    for text in [
        "Exile target creature.",
        "Choose a creature you control. Exile it.",
        "Exile a nonland card from your hand.",
        "Exile target card from a graveyard.",
        "Return target creature card from your graveyard to your hand.",
        "Put target face-up exiled card into its owner's graveyard.",
        "Search your library for a basic land card, reveal it, put it into your hand, then shuffle.",
        "Look at the top three cards of your library. Put one of them into your hand and the rest on the bottom of your library in any order.",
    ] {
        parse_card_definition_with_runtime_builder(
            "Scoped Selection Probe",
            format!("Type: Sorcery\n{text}"),
            false,
        )
        .unwrap_or_else(|error| panic!("{text}: {error}"));
    }
}

#[test]
fn hand_and_library_collections_and_zone_change_event_filters_still_compile() {
    for name in [
        "Deep-Cavern Bat",
        "Thoughtseize",
        "Impulse",
        "Demonic Tutor",
        "Leyline of the Void",
        "Bloodchief Ascension",
        "Rest in Peace",
    ] {
        let payload = ironsmith_tools::load_card_payloads_by_name(
            ironsmith_tools::default_cards_path().to_str().unwrap(),
            name,
        )
        .unwrap()
        .remove(0);
        ironsmith_tools::compile_definition_from_payload(&payload)
            .unwrap_or_else(|error| panic!("{name}: {error}"));
    }
}
