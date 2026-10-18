//! Full-body regression for both exile receipts and each suspended card's owner.
#[path = "p02_line_families/compile.rs"]
mod compile;
use ironsmith::{CardId, GameState, PlayerId, Zone};
use ironsmith::object::CounterType;

fn settle(game: &mut GameState) {
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    ironsmith::game_loop::drain_pending_trigger_events(game, &mut queue);
    let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
    ironsmith::game_loop::put_triggers_on_stack_with_dm(game, &mut queue, &mut dm).unwrap();
    while !game.stack.is_empty() {
        ironsmith::game_loop::resolve_stack_entry_with(game, &mut dm).unwrap();
    }
}

#[test]
fn sinister_concierge_complete_body_suspends_both_exiles_for_their_owners() {
    let row: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/sinister_concierge_hold.json.fixture"
    )).unwrap();
    assert_eq!(row["oracle_id"], "9b14c20f-c4ee-42ea-99cd-099b4eb25883");
    let name = row["card_name"].as_str().unwrap();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for field in ["raw_oracle_text", "normalized_oracle_text"] {
        let text = format!("Type: Creature — Human Wizard\nPower/Toughness: 2/1\n{}", row[field].as_str().unwrap());
        for definition in compile::compile_both(name, &text) {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
            let target = game.create_object_from_card(
                &ironsmith::card::CardBuilder::new(CardId::new(), "Other creature")
                    .card_types(vec![ironsmith::types::CardType::Creature])
                    .power_toughness(ironsmith::card::PowerToughness::fixed(2, 2)).build(),
                bob, Zone::Battlefield);
            let source_stable = game.object(source).unwrap().stable_id;
            let target_stable = game.object(target).unwrap().stable_id;
            game.move_object(source, Zone::Graveyard, ironsmith::events::EventCause::effect()).unwrap();
            settle(&mut game);
            let source = game.find_object_by_stable_id(source_stable).unwrap();
            let target = game.find_object_by_stable_id(target_stable).unwrap();
            for id in [source, target] {
                assert_eq!(game.object(id).unwrap().zone, Zone::Exile, "{field}");
                assert_eq!(game.counter_count(id, CounterType::Time), 3, "{field}");
            }
            for (player, expected) in [(alice, [2, 3]), (bob, [2, 2])] {
                game.turn.active_player = player;
                let event = ironsmith::triggers::TriggerEvent::new(
                    ironsmith::events::BeginningOfUpkeepEvent::new(player), Default::default());
                game.queue_trigger_event(event.provenance(), event);
                settle(&mut game);
                assert_eq!(game.counter_count(source, CounterType::Time), expected[0]);
                assert_eq!(game.counter_count(target, CounterType::Time), expected[1]);
            }
        }
    }
}
