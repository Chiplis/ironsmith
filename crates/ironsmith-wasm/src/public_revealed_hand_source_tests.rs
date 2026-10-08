//! Whole frozen original bodies at the real snapshot and crypto owners. UNRUN.
use super::*;
use ironsmith::cards::CardDefinition;
use ironsmith::continuous::Modification;
use ironsmith::effect::Until;
use ironsmith::effects::{ApplyContinuousEffect, EffectContext, EffectExecutor};
use ironsmith::game_loop::PriorityLoopState;
use ironsmith::ChooseSpec;
use ironsmith_compiler::parse_loss;
use ironsmith_compiler_runtime_build::{compile_to_artifact, compile_to_runtime_definition};

fn definitions(name: &str) -> [CardDefinition; 2] {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!("../../../fixtures/public_revealed_hand_bodies.json.fixture")).unwrap();
    let row = rows.iter().find(|row| row["name"] == name).unwrap();
    let mut text = format!("Mana cost: {}\nType: {}\n", row["mana_cost"].as_str().unwrap(), row["type_line"].as_str().unwrap());
    if let (Some(p), Some(t)) = (row["power"].as_str(), row["toughness"].as_str()) {
        text.push_str(&format!("Power/Toughness: {p}/{t}\n"));
    }
    text.push_str(row["oracle_text"].as_str().unwrap());
    let (direct, loss) = parse_loss::capture(|| compile_to_runtime_definition(name, &text, false));
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (artifact, loss) = parse_loss::capture(|| compile_to_artifact(name, &text, false));
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (artifact, _) = artifact.unwrap();
    let decoded = ironsmith_compiled_artifact::CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    decoded.validate().unwrap();
    assert_eq!(artifact, decoded);
    [direct.unwrap(), ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap()]
}

fn assert_views(game: &GameState, cache: &SnapshotObjectViewCache, public: bool) {
    for viewer in &game.players {
        let snapshot = GameSnapshot::from_game_with_object_view_cache(game, viewer.id,
            None, None, None, None, None, Vec::new(), None, false, None, 0, cache);
        for subject in &game.players {
            let player = snapshot.players.iter().find(|p| p.id == subject.id.0).unwrap();
            assert_eq!(player.can_view_hand, public || viewer.id == subject.id);
            assert!(!player.can_view_library_top, "global hand visibility must not reveal library");
            let actual: Vec<_> = player.hand_cards.iter().map(|card| (card.id, card.name.as_str())).collect();
            let expected: Vec<_> = if public || viewer.id == subject.id {
                subject.hand.iter().rev().map(|id| (id.0, game.object(*id).unwrap().name.as_str())).collect()
            } else { Vec::new() };
            assert_eq!(actual, expected, "actual identities for every viewer/subject pair");
            let actual: Vec<_> = player.persistent_look_cards.iter().map(|card| card.id).collect();
            let expected: Vec<_> = if public { subject.hand.iter().map(|id| id.0).collect() } else { Vec::new() };
            assert_eq!(actual, expected, "no stale identities and no library leakage");
            assert_eq!(crate::hand_revealed_by_static_ability(game, subject.id), public);
        }
    }
}

#[test]
fn frozen_bodies_render_all_hands_only_while_active_and_native_rollback_restores_cached_views() {
    let _ids = crate::test_id_counter_guard();
    for name in ["Revelation", "Wandering Eye"] { for definition in definitions(name) {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into(), "Dan".into()], 20);
        let players = [0, 1, 2, 3].map(PlayerId::from_index);
        let witness = compile_to_runtime_definition("Private witness", "Type: Artifact", false).unwrap();
        for player in players {
            game.create_object_from_definition(&witness, player, Zone::Hand);
            game.create_object_from_definition(&witness, player, Zone::Library);
        }
        let cache = SnapshotObjectViewCache::default();
        for zone in [Zone::Hand, Zone::Library, Zone::Graveyard, Zone::Exile, Zone::Stack] {
            let inactive = game.create_object_from_definition(&definition, players[0], zone);
            assert_views(&game, &cache, false);
            game.remove_object(inactive);
        }
        let host = game.create_object_from_definition(&definition, players[0], Zone::Battlefield);
        assert_views(&game, &cache, true);
        game.set_current_controller(host, players[1]).unwrap();
        assert_views(&game, &cache, true);
        game.phase_out(host); assert_views(&game, &cache, false);
        game.phase_in(host); assert_views(&game, &cache, true);
        // Membership changes must refresh real card identities, not just a boolean permission.
        let departed = game.player(players[2]).unwrap().hand[0];
        game.move_object_by_effect(departed, Zone::Library).unwrap();
        assert_views(&game, &cache, true);
        game.create_object_from_definition(&witness, players[2], Zone::Hand);
        assert_views(&game, &cache, true);
        ApplyContinuousEffect::with_spec(ChooseSpec::SpecificObject(host), Modification::RemoveAllAbilities, Until::EndOfTurn)
            .execute(&mut game, &mut EffectContext::new_default(host, players[1])).unwrap();
        game.refresh_continuous_state().unwrap(); assert_views(&game, &cache, false);
        game.effect_store.continuous_effects.cleanup_end_of_turn();
        game.refresh_continuous_state().unwrap(); assert_views(&game, &cache, true);
        let mut transaction = PriorityLoopState::new(4);
        transaction.save_checkpoint(&game);
        game.move_object_by_effect(host, Zone::Graveyard).unwrap();
        assert_views(&game, &cache, false);
        assert!(transaction.rollback_action(&mut game));
        assert!(transaction.checkpoint.is_none());
        assert_views(&game, &cache, true);
        if name == "Wandering Eye" { game.mark_damage(host, 3); }
        else {
            let world = compile_to_runtime_definition("New World", "Type: World Enchantment", false).unwrap();
            game.create_object_from_definition(&world, players[3], Zone::Battlefield);
        }
        assert!(ironsmith::rules::state_based::apply_state_based_actions(&mut game).unwrap());
        assert_views(&game, &cache, false);
    } }
}

#[test]
fn frozen_bodies_require_exact_public_hand_proofs_for_every_seat_and_no_library_proof() {
    let _ids = crate::test_id_counter_guard();
    for name in ["Revelation", "Wandering Eye"] { for definition in definitions(name) {
        let mut wasm = crate::WasmGame::new();
        wasm.initialize_empty_match(vec!["Alice".into(), "Bob".into(), "Carol".into(), "Dan".into()], 20, 1);
        let players = [0, 1, 2, 3].map(PlayerId::from_index);
        let hands: Vec<_> = players.iter().map(|&p| wasm.game.create_hidden_card_placeholder(p, Zone::Hand, 0, format!("hand-{}", p.index()))).collect();
        let _libraries: Vec<_> = players.iter().map(|&p| wasm.game.create_hidden_card_placeholder(p, Zone::Library, 1, format!("library-{}", p.index()))).collect();
        let check = |wasm: &crate::WasmGame, active: bool| {
            // Compare the complete serialized multiset, including cardinality,
            // visibility, producer, physical commitment and origin binding.
            // No extra private/public opening, wrong zone or duplicate is allowed.
            let mut expected = Vec::new();
            if active {
                for (i, player) in players.iter().enumerate() {
                    let owner = player.index() as u8;
                    let commitment = format!("hand-{}", player.index());
                    expected.push(serde_json::json!({
                        "id": format!("public_view:0:{}:hand:1", player.index()),
                        "type": "public_view_window", "owner": owner, "viewer": 0,
                        "zone": "hand", "visibility": "public", "count": 1,
                        "reason": "Static ability reveals a player's hand"
                    }));
                    expected.push(serde_json::json!({
                        "id": format!("public_open:{}:hand:0:{}", player.index(), hands[i].0),
                        "type": "public_open", "owner": owner, "zone": "hand",
                        "slot": 0, "objectId": hands[i].0, "commitment": commitment,
                        "originSlot": 0, "originCommitment": format!("hand-{}", player.index()),
                        "visibility": "public", "reason": "Static ability reveals a player's hand"
                    }));
                }
            }
            let mut actual: Vec<_> = wasm.last_crypto_requirements.iter()
                .map(|requirement| serde_json::to_value(requirement).unwrap()).collect();
            actual.sort_by_key(|value| value["id"].as_str().unwrap().to_owned());
            expected.sort_by_key(|value| value["id"].as_str().unwrap().to_owned());
            assert_eq!(actual, expected, "exact full proof list; no library exposure or stale requirement");
        };
        let before = wasm.capture_crypto_audit_state();
        wasm.update_crypto_requirements_from(before); check(&wasm, false);
        let before = wasm.capture_crypto_audit_state();
        let host = wasm.game.create_object_from_definition(&definition, players[0], Zone::Battlefield);
        wasm.update_crypto_requirements_from(before); check(&wasm, true);
        let before = wasm.capture_crypto_audit_state();
        wasm.game.set_current_controller(host, players[1]).unwrap();
        wasm.update_crypto_requirements_from(before); check(&wasm, true);
        let before = wasm.capture_crypto_audit_state(); wasm.game.phase_out(host);
        wasm.update_crypto_requirements_from(before); check(&wasm, false);
        let before = wasm.capture_crypto_audit_state(); wasm.game.phase_in(host);
        wasm.update_crypto_requirements_from(before); check(&wasm, true);
        let before = wasm.capture_crypto_audit_state();
        ApplyContinuousEffect::with_spec(ChooseSpec::SpecificObject(host), Modification::RemoveAllAbilities, Until::EndOfTurn)
            .execute(&mut wasm.game, &mut EffectContext::new_default(host, players[1])).unwrap();
        wasm.game.refresh_continuous_state().unwrap();
        wasm.update_crypto_requirements_from(before); check(&wasm, false);
        let before = wasm.capture_crypto_audit_state();
        wasm.game.effect_store.continuous_effects.cleanup_end_of_turn();
        wasm.game.refresh_continuous_state().unwrap();
        wasm.update_crypto_requirements_from(before); check(&wasm, true);
        let mut transaction = PriorityLoopState::new(4);
        transaction.save_checkpoint(&wasm.game);
        let before = wasm.capture_crypto_audit_state(); wasm.game.move_object_by_effect(host, Zone::Graveyard).unwrap();
        wasm.update_crypto_requirements_from(before); check(&wasm, false);
        let before = wasm.capture_crypto_audit_state();
        assert!(transaction.rollback_action(&mut wasm.game));
        wasm.update_crypto_requirements_from(before); check(&wasm, true);
    } }
}
