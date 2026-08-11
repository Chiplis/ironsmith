#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

fn three_player_game() -> crate::GameState {
    crate::GameState::new(
        vec![
            "Alice".to_string(),
            "Bob".to_string(),
            "Charlie".to_string(),
        ],
        20,
    )
}

fn card(name: &str, card_type: CardType) -> CardDefinition {
    let mut builder = CardDefinitionBuilder::new(CardId::new(), name).card_types(vec![card_type]);
    if card_type == CardType::Creature {
        builder = builder.power_toughness(PowerToughness::fixed(2, 2));
    }
    builder.build()
}

fn put_two_card_library(
    game: &mut crate::GameState,
    player: PlayerId,
    prefix: &str,
    top_type: CardType,
) -> (ObjectId, StableId) {
    let bottom = game.create_object_from_definition(
        &card(&format!("{prefix} Bottom"), CardType::Land),
        player,
        Zone::Library,
    );
    let top = game.create_object_from_definition(
        &card(&format!("{prefix} Top"), top_type),
        player,
        Zone::Library,
    );
    let stable = game.object(top).expect("top card should exist").stable_id;
    assert!(game.set_player_library_order_with_audit(
        player,
        vec![bottom, top],
        "each-player top-library behavior regression setup",
    ));
    (bottom, stable)
}

fn resolve_attack_trigger(game: &mut crate::GameState, source: ObjectId, defender: PlayerId) {
    let event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::combat::CreatureAttackedEvent::new(
            source,
            crate::events::combat::AttackEventTarget::Player(defender),
        ),
        crate::provenance::ProvNodeId::default(),
    );
    let triggers = crate::triggers::check_triggers(game, &event)
        .into_iter()
        .filter(|entry| entry.source == source)
        .collect::<Vec<_>>();
    assert_eq!(triggers.len(), 1, "the named creature should trigger once");
    let mut queue = crate::triggers::TriggerQueue::new();
    for trigger in triggers {
        queue.add(trigger);
    }
    crate::game_loop::put_triggers_on_stack(game, &mut queue)
        .expect("attack trigger should go on the stack");
    let mut decisions = crate::decision::SelectFirstDecisionMaker;
    crate::game_loop::resolve_stack_entry_with(game, &mut decisions)
        .expect("attack trigger should resolve");
}

fn exiled_id(game: &crate::GameState, stable: StableId) -> ObjectId {
    let id = game
        .find_object_by_stable_id(stable)
        .expect("top card should retain stable identity after exile");
    assert_eq!(
        game.object(id).expect("exiled card should exist").zone,
        Zone::Exile
    );
    id
}

#[test]
fn pako_exiles_every_players_top_card_and_counts_only_noncreatures() {
    let definition = parse_oracle_card_definition("Pako, Arcane Retriever");
    let debug = format!("{:#?}", definition.abilities);
    assert!(
        debug.contains("ForPlayersEffect")
            && debug.contains("filter: Any")
            && debug.contains("IteratedPlayer"),
        "Pako must lower the quantified library owner to an all-player loop: {debug}"
    );

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);
    let mut game = three_player_game();
    game.turn.active_player = alice;
    let pako = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let (alice_bottom, alice_top) =
        put_two_card_library(&mut game, alice, "Alice", CardType::Instant);
    let (bob_bottom, bob_top) = put_two_card_library(&mut game, bob, "Bob", CardType::Creature);
    let (charlie_bottom, charlie_top) =
        put_two_card_library(&mut game, charlie, "Charlie", CardType::Sorcery);

    resolve_attack_trigger(&mut game, pako, bob);

    for bottom in [alice_bottom, bob_bottom, charlie_bottom] {
        assert_eq!(
            game.object(bottom).expect("bottom card should remain").zone,
            Zone::Library,
            "Pako must exile only the top card of each library"
        );
    }
    for stable in [alice_top, bob_top, charlie_top] {
        let exiled = exiled_id(&game, stable);
        assert_eq!(
            game.counter_count(exiled, CounterType::Named("fetch")),
            1,
            "every card Pako exiles this way gets a fetch counter"
        );
    }
    assert_eq!(
        game.counter_count(pako, CounterType::PlusOnePlusOne),
        2,
        "only Alice's instant and Charlie's sorcery should count as noncreatures"
    );
}

#[test]
fn mezzio_mugger_exiles_and_grants_permission_for_every_players_top_card() {
    let definition = parse_oracle_card_definition("Mezzio Mugger");
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);
    let mut game = three_player_game();
    game.turn.active_player = alice;
    let mugger = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let (alice_bottom, alice_top) =
        put_two_card_library(&mut game, alice, "Alice Mugger", CardType::Instant);
    let (bob_bottom, bob_top) =
        put_two_card_library(&mut game, bob, "Bob Mugger", CardType::Instant);
    let (charlie_bottom, charlie_top) =
        put_two_card_library(&mut game, charlie, "Charlie Mugger", CardType::Instant);

    resolve_attack_trigger(&mut game, mugger, bob);

    for bottom in [alice_bottom, bob_bottom, charlie_bottom] {
        assert_eq!(
            game.object(bottom).expect("bottom card should remain").zone,
            Zone::Library
        );
    }
    for stable in [alice_top, bob_top, charlie_top] {
        let exiled = exiled_id(&game, stable);
        assert!(
            game.effect_store.grant_registry.card_can_play_from_zone(
                &game,
                exiled,
                Zone::Exile,
                alice
            ),
            "Mezzio Mugger must let its controller play every card exiled this way"
        );
        assert!(
            game.can_spend_mana_as_any_color(alice, Some(exiled)),
            "Mezzio Mugger must allow any-color mana for every exiled spell"
        );
    }
}
