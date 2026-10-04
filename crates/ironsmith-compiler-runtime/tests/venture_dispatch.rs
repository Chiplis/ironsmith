use ironsmith::ability::Ability;
use ironsmith::card::CardBuilder;
use ironsmith::cards::CardDefinition;
use ironsmith::effect::Effect;
use ironsmith::game_loop::{put_triggers_on_stack, resolve_stack_entry};
use ironsmith::game_state::StackEntry;
use ironsmith::triggers::{Trigger, TriggerQueue};
use ironsmith::{CardId, CardType, GameState, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::compile_to_artifact;
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;

#[test]
fn venture_dispatch_compiles_complete_frozen_candidate_cards() {
    let cards: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/venture_dispatch.json.fixture"
    ))
    .unwrap();
    assert_eq!(cards.len(), 32);
    let mut failures = Vec::new();
    for card in cards {
        let name = card["name"].as_str().unwrap();
        let mut text = format!(
            "Mana cost: {}\nType: {}\n",
            card["mana_cost"].as_str().unwrap_or(""),
            card["type_line"].as_str().unwrap()
        );
        if let (Some(power), Some(toughness)) = (card["power"].as_str(), card["toughness"].as_str())
        {
            text.push_str(&format!("Power/Toughness: {power}/{toughness}\n"));
        }
        if let Some(loyalty) = card["loyalty"].as_str() {
            text.push_str(&format!("Loyalty: {loyalty}\n"));
        }
        text.push_str(card["oracle_text"].as_str().unwrap());
        match compile_to_artifact(name, text, false) {
            Ok((artifact, _)) => {
                artifact.validate().unwrap();
                let restored =
                    CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
                materialize_artifact(&restored).unwrap();
            }
            Err(error) => failures.push(format!("{name}: {error}")),
        }
    }
    assert!(
        failures.is_empty(),
        "{} remaining candidates:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

fn register_dungeon() {
    let mut dungeon = CardDefinition::new(
        CardBuilder::new(CardId::new(), "Dispatch test dungeon")
            .card_types(vec![CardType::Dungeon])
            .build(),
    );
    for (room, next, life) in [("Entry", vec!["Exit".to_owned()], 1), ("Exit", vec![], 2)] {
        dungeon.abilities.push(Ability::triggered(
            Trigger::dungeon_room(room, next),
            vec![Effect::gain_life(life)],
        ));
    }
    ironsmith::dungeon::register_dungeon_definition(&dungeon).unwrap();
}

fn resolve_venture(game: &mut GameState, definition: &CardDefinition, player: PlayerId) {
    let spell = game.create_object_from_definition(definition, player, Zone::Stack);
    game.push_to_stack(StackEntry::new(spell, player));
    resolve_stack_entry(game).unwrap();
}

#[test]
fn venture_dispatch_advances_only_the_controller_and_queues_room_abilities() {
    register_dungeon();
    let (artifact, direct) = compile_to_artifact(
        "Venture probe",
        "Type: Instant\nVenture into the dungeon.",
        false,
    )
    .unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    for definition in [direct, materialize_artifact(&restored).unwrap()] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        resolve_venture(&mut game, &definition, alice);
        assert_eq!(game.active_dungeon(alice).unwrap().room_name, "Entry");
        assert!(game.active_dungeon(bob).is_none());
        assert_eq!(
            game.player(alice).unwrap().life,
            20,
            "room effects wait for their triggered ability"
        );
        put_triggers_on_stack(&mut game, &mut TriggerQueue::new()).unwrap();
        assert_eq!(game.stack.len(), 1);
        resolve_stack_entry(&mut game).unwrap();
        assert_eq!(game.player(alice).unwrap().life, 21);
        resolve_venture(&mut game, &definition, alice);
        assert_eq!(game.active_dungeon(alice).unwrap().room_name, "Exit");
        assert_eq!(game.player(alice).unwrap().life, 21);
        put_triggers_on_stack(&mut game, &mut TriggerQueue::new()).unwrap();
        assert_eq!(game.stack.len(), 1);
        resolve_stack_entry(&mut game).unwrap();
        assert_eq!(game.player(alice).unwrap().life, 23);
        assert_eq!(game.player(bob).unwrap().life, 20);
    }
}
