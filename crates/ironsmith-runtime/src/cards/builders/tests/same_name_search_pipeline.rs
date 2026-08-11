use super::shard_16::parse_oracle_card_definition;
use super::*;

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn hand_reveal_and_same_name_search_cards_hide_internal_choices() {
    for name in ["Assembly Hall", "Infernal Tutor"] {
        let definition = parse_oracle_card_definition(name);
        let rendered = unprocessed_compiled_lines(&definition)
            .join(" ")
            .to_ascii_lowercase();
        assert!(
            rendered.contains("reveal a") && rendered.contains("from your hand")
                || rendered.contains("in your hand"),
            "{name}: {rendered}"
        );
        assert!(
            rendered.contains("with the same name as that card"),
            "{name}: {rendered}"
        );
        assert!(
            !rendered.contains("choose a card, then reveal it"),
            "{name}: {rendered}"
        );
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn target_antecedent_same_name_searches_render_inline() {
    for (name, reference) in [
        ("Mask of the Mimic", "target nontoken creature"),
        ("Pack Hunt", "target creature"),
    ] {
        let definition = parse_oracle_card_definition(name);
        let rendered = unprocessed_compiled_lines(&definition)
            .join(" ")
            .to_ascii_lowercase();
        assert!(
            rendered.contains(&format!("same name as that creature"))
                || rendered.contains(&format!("same name as {reference}")),
            "{name}: {rendered}"
        );
        assert!(
            !rendered.contains(&format!("choose {reference}."))
                && !rendered.contains("same name as it"),
            "{name}: {rendered}"
        );
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn targeted_library_exile_pipelines_hide_target_setup() {
    for name in ["Denying Wind", "Supreme Inquisitor"] {
        let definition = parse_oracle_card_definition(name);
        let rendered = unprocessed_compiled_lines(&definition)
            .join(" ")
            .to_ascii_lowercase();
        assert!(
            rendered.contains("search target player's library")
                && rendered.contains("and exile them")
                && rendered.contains("then that player shuffles"),
            "{name}: {rendered}"
        );
        assert!(
            !rendered.contains("choose target player"),
            "{name}: {rendered}"
        );
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn kicked_search_pipeline_has_one_shuffle_per_branch() {
    let definition = parse_oracle_card_definition("Sadistic Sacrament");
    assert_eq!(
        canonical_compiled_lines(&definition),
        [
            "Kicker {7}",
            "Search target player's library for up to three cards, exile them, then that player shuffles. If this spell was kicked, instead search that player's library for up to fifteen cards, exile them, then that player shuffles."
        ]
    );
    let debug = format!("{:?}", definition.spell_effect);
    assert!(
        !debug.contains("IteratedPlayer"),
        "targeted spell search must bind that player to its selected target: {debug}"
    );
    let rendered = unprocessed_compiled_lines(&definition)
        .join(" ")
        .to_ascii_lowercase();
    assert_eq!(rendered.matches("shuffles").count(), 2, "{rendered}");
    assert!(
        rendered.contains("if this spell was kicked, instead"),
        "{rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
fn resolve_sadistic_sacrament_search(kicked: bool) -> (usize, usize) {
    let definition = parse_oracle_card_definition("Sadistic Sacrament");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for index in 0..20 {
        let library_card =
            CardDefinitionBuilder::new(CardId::new(), format!("Searchable Library Card {index}"))
                .card_types(vec![CardType::Sorcery])
                .build();
        game.create_object_from_definition(&library_card, bob, Zone::Library);
    }

    let spell = game.create_object_from_definition(&definition, alice, Zone::Stack);
    let mut paid = crate::cost::OptionalCostsPaid::from_costs(&definition.optional_costs);
    if kicked {
        paid.pay(0);
    }
    game.object_mut(spell)
        .expect("Sadistic Sacrament should exist on the stack")
        .optional_costs_paid = paid.clone();
    let mut decisions = crate::decision::SelectFirstDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new(spell, alice, &mut decisions)
        .with_optional_costs_paid(paid)
        .with_targets(vec![crate::effects::ResolvedTarget::Player(bob)]);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        spell,
        definition
            .spell_effect
            .as_ref()
            .expect("Sadistic Sacrament should have a resolution program"),
        None,
        &[],
    )
    .expect("the targeted search/exile/shuffle program should resolve");

    let exiled = game
        .objects_in_zone(Zone::Exile)
        .into_iter()
        .filter(|object| {
            game.object(*object)
                .is_some_and(|object| object.owner == bob)
        })
        .count();
    let remaining = game
        .player(bob)
        .expect("target player should exist")
        .library
        .len();
    (exiled, remaining)
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn kicked_search_replacement_executes_only_the_selected_count_branch() {
    assert_eq!(resolve_sadistic_sacrament_search(false), (3, 17));
    assert_eq!(resolve_sadistic_sacrament_search(true), (15, 5));
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn dual_nature_keeps_typed_antecedent_and_creation_provenance() {
    let definition = parse_oracle_card_definition("Dual Nature");
    let rendered = unprocessed_compiled_lines(&definition)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("same name as that creature")
            && rendered.contains("tokens created with this enchantment"),
        "{rendered}"
    );
    assert!(!rendered.contains("enchantment tokens"), "{rendered}");
}
