//! Authored only; no execution before the campaign's deferred validation gate.
use ironsmith::ability::AbilityKind;
use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::cards::CardDefinition;
use ironsmith::color::{Color, ColorSet};
use ironsmith::game_state::{StackEntry, TargetAssignment};
use ironsmith::game_loop::{extract_target_requirements_from_program_with_modes, put_triggers_on_stack, resolve_stack_entry};
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardId, CardType, GameState, ObjectId, PlayerId, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::compile_to_artifact;
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;

fn definitions(name: &str) -> [CardDefinition; 2] {
    let cards: Vec<serde_json::Value> = serde_json::from_str(include_str!("../../../fixtures/conditional_life_gain.json.fixture")).unwrap();
    let card = cards.iter().find(|card| card["name"] == name).unwrap();
    let mut text = format!("Mana cost: {}\nType: {}\n", card["mana_cost"].as_str().unwrap_or(""), card["type_line"].as_str().unwrap());
    if let (Some(power), Some(toughness)) = (card["power"].as_str(), card["toughness"].as_str()) { text.push_str(&format!("Power/Toughness: {power}/{toughness}\n")); }
    text.push_str(card["oracle_text"].as_str().unwrap());
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, text, false));
    let (artifact, direct) = result.unwrap();
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    artifact.validate().unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    [direct, materialize_artifact(&restored).unwrap()]
}
fn creature(game: &mut GameState, player: PlayerId, name: &str, power: i32) -> ObjectId {
    game.create_object_from_card(&CardBuilder::new(CardId::new(), name).card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(power, 5)).build(), player, Zone::Battlefield)
}
fn put_spell(game: &mut GameState, definition: &CardDefinition, target: Option<PlayerId>) {
    let alice = PlayerId::from_index(0);
    let spell = game.create_object_from_definition(definition, alice, Zone::Stack);
    let program = definition.spell_effect.as_ref().unwrap();
    assert!(program.segments.iter().any(|segment| !segment.self_replacements.is_empty()));
    let requirements = extract_target_requirements_from_program_with_modes(game, program, alice, Some(spell), None);
    let mut entry = StackEntry::new(spell, alice);
    if let Some(target) = target {
        assert_eq!(requirements.len(), 1, "the replacement shares the original declared target");
        assert_eq!(requirements[0].min_targets, 1);
        entry = entry.with_targets(vec![Target::Player(target)])
            .with_target_assignments(vec![TargetAssignment { spec: requirements[0].spec.clone(), range: 0..1 }]);
    } else { assert!(requirements.is_empty()); }
    game.push_to_stack(entry);
}

#[test]
fn death_history_selects_one_life_gain_at_resolution() {
    let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1);
    for definition in definitions("Life Goes On") {
        for died in [false, true] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let doomed = creature(&mut game, bob, "Death history probe", 1);
            put_spell(&mut game, &definition, None);
            if died { game.move_object_by_effect(doomed, Zone::Graveyard).unwrap(); }
            resolve_stack_entry(&mut game).unwrap();
            assert_eq!(game.player(alice).unwrap().life, if died { 28 } else { 24 });
        }
    }
}

#[test]
fn ferocious_checks_live_power_and_replaces_five_with_ten() {
    let alice = PlayerId::from_index(0);
    for definition in definitions("Feed the Clan") {
        for grow in [false, true] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let host = creature(&mut game, alice, "Growing probe", 3);
            put_spell(&mut game, &definition, None);
            if grow { game.add_counters(host, ironsmith::object::CounterType::PlusOnePlusOne, 1).unwrap(); }
            resolve_stack_entry(&mut game).unwrap();
            assert_eq!(game.player(alice).unwrap().life, if grow { 30 } else { 25 });
        }
    }
}

#[test]
fn landfall_gate_is_caster_relative_and_both_arms_share_target_player() {
    let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1);
    for definition in definitions("Rest for the Weary") {
        for land_owner in [None, Some(alice), Some(bob)] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            put_spell(&mut game, &definition, Some(bob));
            if let Some(owner) = land_owner {
                let land = game.create_object_from_card(&CardBuilder::new(CardId::new(), "Landfall probe").card_types(vec![CardType::Land]).build(), owner, Zone::Hand);
                let mut decisions = ironsmith::decision::SelectFirstDecisionMaker;
                let receipt = game.move_object_with_etb_processing_with_dm(land, Zone::Battlefield, &mut decisions).unwrap();
                assert!(!receipt.pending);
                assert!(receipt.programs.is_empty(), "fixture must not discard added entry instructions");
                receipt.original.into_result().unwrap();
            }
            resolve_stack_entry(&mut game).unwrap();
            assert_eq!(game.player(alice).unwrap().life, 20);
            assert_eq!(game.player(bob).unwrap().life, if land_owner == Some(alice) { 28 } else { 24 });
        }
    }
}

#[test]
fn named_worker_condition_requires_both_controlled_names() {
    let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1);
    for definition in definitions("Mine Worker") {
        for tower_owner in [None, Some(alice), Some(bob)] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
            creature(&mut game, alice, "Power Plant Worker", 4);
            if let Some(owner) = tower_owner { creature(&mut game, owner, "Tower Worker", 1); }
            let ability = definition.abilities.iter().find_map(|ability| match &ability.kind { AbilityKind::Activated(activated) => Some(activated), _ => None }).unwrap();
            assert!(ability.effects.segments.iter().any(|segment| !segment.self_replacements.is_empty()));
            game.push_to_stack(StackEntry::ability(source, alice, ability.effects.clone()));
            resolve_stack_entry(&mut game).unwrap();
            assert_eq!(game.player(alice).unwrap().life, if tower_owner == Some(alice) { 23 } else { 21 });
        }
    }
}

#[test]
fn sanctuary_uses_real_upkeep_and_rechecks_both_control_conditions() {
    let alice = PlayerId::from_index(0);
    for definition in definitions("Dega Sanctuary") {
        for colors in [ColorSet::COLORLESS, ColorSet::BLACK, ColorSet::BLACK.with(Color::Red)] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            game.create_object_from_definition(&definition, alice, Zone::Battlefield);
            game.create_object_from_card(&CardBuilder::new(CardId::new(), "Colored permanent").card_types(vec![CardType::Artifact]).color_indicator(colors).build(), alice, Zone::Battlefield);
            let mut runner = ironsmith::turn_runner::TurnRunner::from_state_for_sync(ironsmith::turn_runner::TurnState::Upkeep);
            let mut queue = TriggerQueue::new();
            runner.advance(&mut game, &mut queue).unwrap();
            put_triggers_on_stack(&mut game, &mut queue).unwrap();
            while !game.stack_is_empty() { resolve_stack_entry(&mut game).unwrap(); }
            let expected = if colors == ColorSet::COLORLESS { 20 } else if colors == ColorSet::BLACK { 22 } else { 24 };
            assert_eq!(game.player(alice).unwrap().life, expected);
        }
    }
}

#[test]
fn sanctuary_inner_replacement_rechecks_after_the_trigger_is_on_the_stack() {
    let alice = PlayerId::from_index(0);
    for definition in definitions("Dega Sanctuary") {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        for color in [ColorSet::BLACK, ColorSet::RED] {
            game.create_object_from_card(&CardBuilder::new(CardId::new(), if color == ColorSet::RED { "Remove red" } else { "Keep black" })
                .card_types(vec![CardType::Artifact]).color_indicator(color).build(), alice, Zone::Battlefield);
        }
        let red = game.battlefield.iter().copied().find(|id| game.object(*id).is_some_and(|object| object.name == "Remove red")).unwrap();
        let mut runner = ironsmith::turn_runner::TurnRunner::from_state_for_sync(ironsmith::turn_runner::TurnState::Upkeep);
        let mut queue = TriggerQueue::new();
        runner.advance(&mut game, &mut queue).unwrap();
        put_triggers_on_stack(&mut game, &mut queue).unwrap();
        assert_eq!(game.stack.len(), 1);
        game.move_object_by_effect(red, Zone::Graveyard).unwrap();
        resolve_stack_entry(&mut game).unwrap();
        assert_eq!(game.player(alice).unwrap().life, 22, "outer black-or-red condition remains true, inner black-and-red condition is now false");
    }
}
