use ironsmith::WasmGame;
use serde_json::Value;

fn snapshot_json(game: &WasmGame) -> Value {
    serde_json::from_str(
        &game
            .snapshot_json()
            .expect("snapshot json should serialize"),
    )
    .expect("snapshot json should parse")
}

fn priority_action_index(snapshot: &Value, label: &str) -> usize {
    snapshot["decision"]["actions"]
        .as_array()
        .expect("priority decision should expose actions")
        .iter()
        .find(|action| action["label"].as_str() == Some(label))
        .and_then(|action| action["index"].as_u64())
        .map(|index| index as usize)
        .unwrap_or_else(|| panic!("expected priority action labeled {label}"))
}

fn perspective_player<'a>(snapshot: &'a Value) -> &'a Value {
    let perspective = snapshot["perspective"]
        .as_u64()
        .expect("snapshot should include perspective") as u8;
    snapshot["players"]
        .as_array()
        .expect("snapshot should include players")
        .iter()
        .find(|player| player["id"].as_u64() == Some(perspective as u64))
        .expect("perspective player should be present")
}

fn count_named_cards(cards: &Value, name: &str) -> usize {
    cards
        .as_array()
        .expect("zone should be an array")
        .iter()
        .filter(|card| card["name"].as_str() == Some(name))
        .count()
}

#[test]
fn canceling_spell_chain_after_land_play_keeps_played_land() {
    let mut game = WasmGame::new();
    game.load_demo_decks()
        .expect("demo decks should load for the regression test");
    game.add_card_to_zone(0, "Plains".to_string(), "hand".to_string(), true)
        .expect("should be able to add plains to hand");
    game.add_card_to_zone(0, "Lightning Bolt".to_string(), "hand".to_string(), true)
        .expect("should be able to add lightning bolt to hand");

    let before = snapshot_json(&game);
    let before_player = perspective_player(&before);
    let plains_battlefield_before = count_named_cards(&before_player["battlefield"], "Plains");
    let bolt_hand_before = count_named_cards(&before_player["hand_cards"], "Lightning Bolt");

    let play_plains_index = priority_action_index(&before, "Play Plains");
    game.dispatch(
        serde_wasm_bindgen::to_value(&serde_json::json!({
            "type": "priority_action",
            "action_index": play_plains_index,
        }))
        .expect("priority action command should serialize"),
    )
    .expect("playing plains should succeed");

    let after_land = snapshot_json(&game);
    let after_land_player = perspective_player(&after_land);
    assert_eq!(
        count_named_cards(&after_land_player["battlefield"], "Plains"),
        plains_battlefield_before + 1,
        "playing the land should move it onto the battlefield",
    );

    let cast_bolt_index = priority_action_index(&after_land, "Cast Lightning Bolt");
    game.dispatch(
        serde_wasm_bindgen::to_value(&serde_json::json!({
            "type": "priority_action",
            "action_index": cast_bolt_index,
        }))
        .expect("priority action command should serialize"),
    )
    .expect("casting lightning bolt should enter its decision chain");

    let during_cast = snapshot_json(&game);
    assert_eq!(
        during_cast["decision"]["kind"].as_str(),
        Some("targets"),
        "lightning bolt should be waiting on targets before cancel",
    );
    assert_eq!(
        during_cast["cancelable"].as_bool(),
        Some(true),
        "the in-progress spell should remain cancelable",
    );

    game.cancel_decision()
        .expect("canceling the in-progress spell should succeed");

    let after_cancel = snapshot_json(&game);
    let after_cancel_player = perspective_player(&after_cancel);
    assert_eq!(
        count_named_cards(&after_cancel_player["battlefield"], "Plains"),
        plains_battlefield_before + 1,
        "canceling the spell should keep the played land on the battlefield",
    );
    assert_eq!(
        count_named_cards(&after_cancel_player["hand_cards"], "Lightning Bolt"),
        bolt_hand_before,
        "canceling the spell should put lightning bolt back into hand",
    );
    assert_eq!(
        after_cancel["stack_size"].as_u64(),
        Some(0),
        "canceling the spell should clear it from the stack",
    );
}
