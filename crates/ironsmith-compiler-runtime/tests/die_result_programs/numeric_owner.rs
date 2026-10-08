//! Source-authored regression scenarios; not executed during this source-only repair.
use super::*;
use ironsmith::decision::compute_legal_actions;

// Freeze the exact fresh official record and authenticate all printed fields
// before either independent compiler route sees the complete card body.
fn troll_definitions() -> [CardDefinition; 2] {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../fixtures/loathsome_troll_numeric_owner.json.fixture"
    )).unwrap();
    assert_eq!(fixture["source_sha256"], "bae465b9d536fffa24c656daff5577a87f5a963dcb2160be0c1dc9ba8e225750");
    let record = &fixture["source_record"];
    assert_eq!(record["oracle_id"], "d360ce89-d80d-4be6-be8c-7e7758cd5840");
    assert_eq!(record["id"], "d0505a8a-841b-471b-970d-125583b9ba9d");
    assert_eq!(record["name"], "Loathsome Troll");
    assert_eq!(record["layout"], "normal");
    assert_eq!(record["mana_cost"], "{3}{G}{G}");
    assert_eq!(record["type_line"], "Creature — Troll");
    assert_eq!(record["power"], "6");
    assert_eq!(record["toughness"], "2");
    assert_eq!(record["colors"], serde_json::json!(["G"]));
    assert_eq!(record["color_identity"], serde_json::json!(["G"]));
    assert_eq!(record["oracle_text"], "{3}{G}: Roll a d20. Activate only if this card is in your graveyard.\n1—9 | Put this card on top of your library.\n10—19 | Return this card to your hand.\n20 | Return this card to the battlefield tapped.");
    let expected_text = format!("Mana cost: {}\nType: {}\nPower/Toughness: {}/{}\n{}",
        record["mana_cost"].as_str().unwrap(), record["type_line"].as_str().unwrap(),
        record["power"].as_str().unwrap(), record["toughness"].as_str().unwrap(),
        record["oracle_text"].as_str().unwrap());
    assert_eq!(fixture["text"].as_str().unwrap(), expected_text);
    let definitions = program_definitions(record["name"].as_str().unwrap(), &expected_text);
    for definition in &definitions {
        assert_eq!(definition.card.name, "Loathsome Troll");
        assert_eq!(definition.card.mana_cost.as_ref().unwrap().to_oracle(), "{3}{G}{G}");
        assert_eq!(definition.card.card_types, vec![ironsmith::CardType::Creature]);
        assert_eq!(definition.card.subtypes, vec![ironsmith::Subtype::Troll]);
        assert_eq!(definition.card.colors(), ironsmith::color::ColorSet::GREEN);
        let pt = definition.card.power_toughness.as_ref().unwrap();
        assert_eq!((pt.power.base_value(), pt.toughness.base_value()), (6, 2));
    }
    definitions
}

#[test]
fn troll_full_body_keeps_one_graveyard_activation_and_one_exact_die_owner() {
    for definition in troll_definitions() {
        assert_eq!(definition.abilities.len(), 1);
        assert!(definition.spell_effect.is_none());
        let ability = &definition.abilities[0];
        assert_eq!(ability.functional_zones, vec![Zone::Graveyard]);
        let AbilityKind::Activated(activated) = &ability.kind else { panic!("missing activation"); };
        assert_eq!(activated.mana_cost.mana_cost().unwrap().to_oracle(), "{3}{G}");
        let mut nodes = Vec::new();
        for effect in activated.effects.all_effects() { collect_runtime_nodes(effect, &mut nodes); }
        assert_eq!(nodes.iter().filter(|effect| effect.downcast_ref::<RollDieEffect>().is_some()).count(), 1);
        let gates = nodes.iter().filter_map(|effect| effect.downcast_ref::<ironsmith::effects::IfEffect>())
            .filter(|gate| matches!(gate.predicate, ironsmith::effect::EffectPredicate::Value(_)))
            .collect::<Vec<_>>();
        assert_eq!(gates.len(), 3);
        let die_id = gates[0].condition;
        assert!(gates.iter().all(|gate| gate.condition == die_id));
        let producers = nodes.iter().filter_map(|effect| effect.downcast_ref::<ironsmith::effects::WithIdEffect>())
            .filter(|producer| producer.id == die_id).collect::<Vec<_>>();
        assert_eq!(producers.len(), 1);
        assert!(exact_die_instruction(&producers[0].effect));
    }
}

#[test]
fn troll_pays_exact_cost_and_moves_only_itself_to_each_branch_destination() {
    for definition in troll_definitions() {
        for result in [1, 9, 10, 19, 20] {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Graveyard);
            let other = object(&mut game, A, Zone::Graveyard, "Other graveyard card", "Type: Land");
            let old_top = object(&mut game, A, Zone::Library, "Old top", "Type: Land");
            game.player_mut(A).unwrap().mana_pool = Default::default();
            game.player_mut(A).unwrap().mana_pool.add(ironsmith::ManaSymbol::Colorless, 3);
            game.player_mut(A).unwrap().mana_pool.add(ironsmith::ManaSymbol::Green, 1);
            let activation = LegalAction::ActivateAbility { source, ability_index: 0 };
            assert!(compute_legal_actions(&game, A).unwrap().contains(&activation));
            let mut dm = Choices::default();
            game.force_next_die_roll(result);
            action(&mut game, activation, &mut dm);
            assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
            assert!(game.player(A).unwrap().graveyard.contains(&source), "movement must wait for resolution");
            settle(&mut game, &mut dm);
            assert_die_receipt(&game, result, result);
            assert_eq!(game.player(A).unwrap().graveyard, vec![other]);
            let destination = if result <= 9 { Zone::Library } else if result <= 19 { Zone::Hand } else { Zone::Battlefield };
            let moved = game.player(A).unwrap().library.iter()
                .chain(game.player(A).unwrap().hand.iter()).chain(game.battlefield.iter())
                .filter_map(|id| game.object(*id))
                .filter(|object| object.name == "Loathsome Troll").collect::<Vec<_>>();
            assert_eq!(moved.len(), 1);
            assert_eq!(moved[0].zone, destination);
            if destination == Zone::Library {
                assert_eq!(game.player(A).unwrap().library, vec![old_top, moved[0].id]);
            } else {
                assert_eq!(game.player(A).unwrap().library, vec![old_top]);
            }
            assert_eq!(game.player(A).unwrap().hand.len(), usize::from(destination == Zone::Hand));
            if destination == Zone::Battlefield { assert!(game.is_tapped(moved[0].id)); }
        }
    }
}

#[test]
fn troll_cannot_activate_from_other_zones_or_without_the_green_mana() {
    for definition in troll_definitions() {
        for zone in [Zone::Hand, Zone::Battlefield, Zone::Exile, Zone::Library] {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, zone);
            assert!(!compute_legal_actions(&game, A).unwrap().contains(&LegalAction::ActivateAbility { source, ability_index: 0 }));
        }
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Graveyard);
        game.player_mut(A).unwrap().mana_pool = Default::default();
        game.player_mut(A).unwrap().mana_pool.add(ironsmith::ManaSymbol::Colorless, 4);
        assert!(!compute_legal_actions(&game, A).unwrap().contains(&LegalAction::ActivateAbility { source, ability_index: 0 }));
    }
}

#[test]
fn restrictions_do_not_rescue_unowned_or_non_immediate_numeric_rows() {
    for head in [
        "{0}: Draw a card.",
        "{0}: Roll a d20. Draw a card.",
        "{0}: You may roll a d20.",
        "{0}: If you control a creature, roll a d20.",
        "{0}: Roll a d20.\n{1}: Draw a card.",
    ] {
        let text = format!("Type: Artifact\n{head} Activate only once each turn.\n1—20 | You gain 3 life.");
        assert!(compile_to_runtime_definition("Unowned restricted table", &text, false).is_err(), "direct: {text}");
        assert!(compile_to_artifact("Unowned restricted table", &text, false).is_err(), "artifact: {text}");
    }
}
