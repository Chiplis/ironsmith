#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

const EXPECTED: &str = "Exile all artifacts, creatures, and lands from the battlefield, all cards from all graveyards, and all cards from all hands.\nCycling {5}{R}{R}\nWhen you cycle this card, destroy all lands.";

fn permanent(name: &str, card_type: CardType) -> CardDefinition {
    let mut builder = CardDefinitionBuilder::new(CardId::new(), name).card_types(vec![card_type]);
    if card_type == CardType::Creature {
        builder = builder.power_toughness(PowerToughness::fixed(2, 2));
    }
    builder.build()
}

fn card(name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Sorcery])
        .build()
}

fn stable_id(game: &crate::GameState, object: ObjectId) -> StableId {
    game.object(object).expect("fixture should exist").stable_id
}

fn current_zone(game: &crate::GameState, stable: StableId) -> Zone {
    let current = game
        .find_object_by_stable_id(stable)
        .expect("fixture should retain stable identity");
    game.object(current).expect("fixture should exist").zone
}

fn filter_contains_zone(filter: &ObjectFilter, zone: Zone) -> bool {
    filter.zone == Some(zone)
        || filter
            .any_of
            .iter()
            .any(|branch| filter_contains_zone(branch, zone))
}

fn filter_contains_battlefield_type(filter: &ObjectFilter, card_type: CardType) -> bool {
    (filter.zone == Some(Zone::Battlefield) && filter.card_types.contains(&card_type))
        || filter
            .any_of
            .iter()
            .any(|branch| filter_contains_battlefield_type(branch, card_type))
}

#[test]
fn exact_text_and_all_authored_exile_domains_are_typed() {
    let definition = parse_oracle_card_definition("Decree of Annihilation");
    assert_eq!(canonical_compiled_lines(&definition).join("\n"), EXPECTED);

    let program = definition
        .spell_effect
        .as_ref()
        .expect("Decree should have a spell-resolution program");
    let exile = program
        .flattened_default_effects()
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::ExileEffect>())
        .expect("Decree should retain one typed bulk-exile action");
    let ChooseSpec::All(filter) = &exile.spec else {
        panic!("Decree's authored `all` sets must remain exhaustive: {exile:#?}");
    };
    for card_type in [CardType::Artifact, CardType::Creature, CardType::Land] {
        assert!(
            filter_contains_battlefield_type(filter, card_type),
            "missing battlefield {card_type:?} domain: {filter:#?}"
        );
    }
    assert!(filter_contains_zone(filter, Zone::Graveyard), "{filter:#?}");
    assert!(filter_contains_zone(filter, Zone::Hand), "{filter:#?}");
}

#[test]
fn spell_exiles_every_included_domain_and_leaves_near_misses_in_place() {
    let definition = parse_oracle_card_definition("Decree of Annihilation");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let battlefield_artifact = game.create_object_from_definition(
        &permanent("Battlefield Artifact", CardType::Artifact),
        alice,
        Zone::Battlefield,
    );
    let battlefield_creature = game.create_object_from_definition(
        &permanent("Battlefield Creature", CardType::Creature),
        bob,
        Zone::Battlefield,
    );
    let battlefield_land = game.create_object_from_definition(
        &permanent("Battlefield Land", CardType::Land),
        bob,
        Zone::Battlefield,
    );
    let battlefield_enchantment = game.create_object_from_definition(
        &permanent("Battlefield Enchantment", CardType::Enchantment),
        bob,
        Zone::Battlefield,
    );
    let alice_graveyard =
        game.create_object_from_definition(&card("Alice Graveyard"), alice, Zone::Graveyard);
    let bob_graveyard =
        game.create_object_from_definition(&card("Bob Graveyard"), bob, Zone::Graveyard);
    let alice_hand = game.create_object_from_definition(&card("Alice Hand"), alice, Zone::Hand);
    let bob_hand = game.create_object_from_definition(&card("Bob Hand"), bob, Zone::Hand);
    let library_near_miss =
        game.create_object_from_definition(&card("Library Near Miss"), bob, Zone::Library);

    let included = [
        battlefield_artifact,
        battlefield_creature,
        battlefield_land,
        alice_graveyard,
        bob_graveyard,
        alice_hand,
        bob_hand,
    ]
    .map(|object| stable_id(&game, object));
    let battlefield_enchantment = stable_id(&game, battlefield_enchantment);
    let library_near_miss = stable_id(&game, library_near_miss);

    let spell = game.create_object_from_definition(&definition, alice, Zone::Stack);
    game.push_to_stack(crate::game_state::StackEntry::new(spell, alice));
    crate::game_loop::resolve_stack_entry(&mut game)
        .expect("Decree of Annihilation should resolve");

    for stable in included {
        assert_eq!(current_zone(&game, stable), Zone::Exile);
    }
    assert_eq!(
        current_zone(&game, battlefield_enchantment),
        Zone::Battlefield,
        "nonartifact enchantments are outside the battlefield set"
    );
    assert_eq!(current_zone(&game, library_near_miss), Zone::Library);
}
