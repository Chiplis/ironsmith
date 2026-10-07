//! Compiler-to-runtime checks for event references. A successful compile or
//! rendered-text comparison cannot establish that the event supplies the
//! player and amount consumed during stack resolution.

use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::events::{CoinFlippedEvent, LifeLossEvent};
use ironsmith::game_loop::{put_triggers_on_stack, resolve_stack_entry};
use ironsmith::triggers::{TriggerEvent, TriggerQueue, check_triggers};
use ironsmith::{CardDefinition, CardId, CardType, GameState, PlayerId, Zone};

fn assert_mill_trigger(
    definition: &CardDefinition,
    event_player: PlayerId,
    count: usize,
    should_trigger: bool,
    event: impl FnOnce(ironsmith::ObjectId) -> TriggerEvent,
) {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Cara".into()], 20);
    let controller = PlayerId::from_index(0);
    // Keep the active player different from the controller to expose accidental
    // fallbacks to either when the event belongs to the other opponent.
    game.turn.active_player = PlayerId::from_index(2);
    let source = game.create_object_from_definition(definition, controller, Zone::Battlefield);
    let filler = CardDefinitionBuilder::new(CardId::new(), "Library fixture")
        .card_types(vec![CardType::Land])
        .build();
    for index in 0..3 {
        for _ in 0..8 {
            game.create_object_from_definition(&filler, PlayerId::from_index(index), Zone::Library);
        }
    }

    let mut queue = TriggerQueue::new();
    for entry in check_triggers(&game, &event(source)) {
        queue.add(entry);
    }
    assert_eq!(
        queue.entries.len(),
        usize::from(should_trigger),
        "{}: trigger matching for {event_player:?}",
        definition.name(),
    );
    put_triggers_on_stack(&mut game, &mut queue).expect("trigger should reach the stack");
    if should_trigger {
        assert_eq!(game.stack.len(), 1);
        resolve_stack_entry(&mut game).unwrap_or_else(|error| {
            panic!(
                "{}: compiled trigger failed to execute: {error}",
                definition.name()
            )
        });
    }
    assert!(game.stack.is_empty());
    for index in 0..3 {
        let player = PlayerId::from_index(index);
        let state = game.player(player).unwrap();
        let expected = if should_trigger && player == event_player {
            count
        } else {
            0
        };
        assert_eq!(
            state.graveyard.len(),
            expected,
            "{}: wrong mill recipient or amount",
            definition.name()
        );
        assert_eq!(state.library.len(), 8 - expected);
    }
}

#[test]
fn catalog_life_loss_triggers_compile_and_mill_the_event_player_and_amount() {
    let names = vec![
        "The Master of Lake-town".to_string(),
        "Mindcrank".to_string(),
    ];
    let payloads = ironsmith_tools::load_card_payloads_by_names(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        &names,
    )
    .expect("canonical card payloads should load");
    assert_eq!(payloads.len(), names.len());
    for payload in payloads.into_values().flatten() {
        let definition = ironsmith_tools::compile_runtime_definition_from_payload(&payload)
            .unwrap_or_else(|error| panic!("{} should compile: {error}", payload.name));
        for index in 0..3 {
            let player = PlayerId::from_index(index);
            for from_damage in [false, true] {
                for amount in [1, 3] {
                    let should_trigger = payload.name != "Mindcrank" || index != 0;
                    assert_mill_trigger(&definition, player, amount, should_trigger, |_| {
                        TriggerEvent::new_with_provenance(
                            LifeLossEvent::new(player, amount as u32, from_damage),
                            Default::default(),
                        )
                    });
                }
            }
        }
    }
}

#[test]
fn compiled_coin_flip_triggers_bind_the_event_player() {
    for won in [false, true] {
        let verb = if won { "wins" } else { "loses" };
        let definition = ironsmith_tools::parse_card_definition_with_runtime_builder(
            "Coin-flip reference fixture",
            format!(
                "Type: Enchantment\nWhenever a player {verb} a coin flip, that player mills a card."
            ),
            false,
        )
        .expect("coin-flip reference fixture should compile");
        let player = PlayerId::from_index(1);
        assert_mill_trigger(&definition, player, 1, true, |source| {
            TriggerEvent::new_with_provenance(
                CoinFlippedEvent {
                    turn_ordinal: 0,
                    instruction_ordinal: 0,
                    player,
                    source,
                    face: ironsmith_core::CoinFace::Heads,
                    call: Some(if won {
                        ironsmith_core::CoinFace::Heads
                    } else {
                        ironsmith_core::CoinFace::Tails
                    }),
                    winner: won.then_some(player),
                    loser: (!won).then_some(player),
                },
                Default::default(),
            )
        });
    }
}
