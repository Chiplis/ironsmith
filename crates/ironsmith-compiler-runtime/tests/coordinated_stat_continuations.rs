use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::cards::CardDefinition;
use ironsmith::object::{AttachmentTarget, CounterType};
use ironsmith::{CardId, CardType, GameState, ObjectId, PlayerId, Subtype, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::compile_to_artifact;
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;

fn definitions(name: &str) -> [CardDefinition; 2] {
    let cards: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/coordinated_stat_continuations.json.fixture"
    ))
    .unwrap();
    let card = cards.into_iter().find(|card| card["name"] == name).unwrap();
    let mut text = format!(
        "Mana cost: {}\nType: {}\n",
        card["mana_cost"].as_str().unwrap(),
        card["type_line"].as_str().unwrap()
    );
    if let (Some(power), Some(toughness)) = (card["power"].as_str(), card["toughness"].as_str()) {
        text.push_str(&format!("Power/Toughness: {power}/{toughness}\n"));
    }
    text.push_str(card["oracle_text"].as_str().unwrap());
    let (artifact, direct) = compile_to_artifact(name, text, false).unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    [direct, materialize_artifact(&restored).unwrap()]
}

fn target(game: &mut GameState, owner: PlayerId) -> ObjectId {
    game.create_object_from_card(
        &CardBuilder::new(CardId::new(), "Large fixture")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(9, 9))
            .build(),
        owner,
        Zone::Battlefield,
    )
}

#[test]
fn replacement_anthem_uses_its_controller_condition_and_replaces_rather_than_stacks() {
    for definition in definitions("Precipitous Drop") {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let charlie = PlayerId::from_index(2);
        let enchanted = target(&mut game, bob);
        let other = target(&mut game, alice);
        let aura = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        assert!(game.attach_object_to_target(aura, AttachmentTarget::Object(enchanted)));
        assert_eq!(
            (
                game.current_power(enchanted),
                game.current_toughness(enchanted)
            ),
            (Some(7), Some(7))
        );
        game.record_completed_dungeon(bob, "Opponent dungeon");
        game.refresh_continuous_state().unwrap();
        assert_eq!(
            game.current_power(enchanted),
            Some(7),
            "recipient controller's achievement must not qualify the Aura controller"
        );
        assert!(
            game.continuous_state_is_clean_public(),
            "prime a clean static-effect cache"
        );
        game.record_completed_dungeon(alice, "Controller dungeon");
        assert!(
            !game.continuous_state_is_clean_public(),
            "recording completion must invalidate conditional continuous effects"
        );
        assert_eq!(
            game.current_power(enchanted),
            Some(4),
            "read-only characteristic queries must see the completion immediately"
        );
        game.refresh_continuous_state().unwrap();
        assert_eq!(
            (
                game.current_power(enchanted),
                game.current_toughness(enchanted)
            ),
            (Some(4), Some(4)),
            "-5/-5 replaces -2/-2; it must not become -7/-7"
        );
        assert_eq!(game.current_power(other), Some(9));
        game.set_current_controller(aura, charlie).unwrap();
        assert_eq!(
            game.current_power(enchanted),
            Some(7),
            "an unqualified new Aura controller restores the base modifier"
        );
        game.set_current_controller(aura, alice).unwrap();
        assert_eq!(game.current_power(enchanted), Some(4));
        game.next_turn();
        assert!(
            game.has_completed_dungeon(alice),
            "completion is game-scoped, not turn-scoped"
        );
        assert_eq!(
            game.current_power(enchanted),
            Some(4),
            "the modifier stays active on another player's turn"
        );
        assert!(game.attach_object_to_target(aura, AttachmentTarget::Object(other)));
        assert_eq!(game.current_power(enchanted), Some(9));
        assert_eq!(game.current_power(other), Some(4));
        game.move_object_by_effect(aura, Zone::Graveyard).unwrap();
        assert_eq!(game.current_power(other), Some(9));
    }
}

