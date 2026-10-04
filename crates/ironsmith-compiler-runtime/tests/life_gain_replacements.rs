//! All scenarios are authored only; execution is deferred by the campaign.
use ironsmith::card::CardBuilder;
use ironsmith::cards::CardDefinition;
use ironsmith::effect::{Effect, EffectOutcome};
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::events::LifeGainEvent;
use ironsmith::decision::DecisionMaker;
use ironsmith::{CardId, CardType, GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::compile_to_artifact;
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;

fn definitions(name: &str, text: &str) -> [CardDefinition; 2] {
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, text, false));
    let (artifact, direct) = result.unwrap_or_else(|error| panic!("{name}: {error}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    artifact.validate().unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    [direct, materialize_artifact(&restored).unwrap()]
}
fn fixture_definitions(name: &str) -> [CardDefinition; 2] {
    let cards: Vec<serde_json::Value> = serde_json::from_str(include_str!("../../../fixtures/life_gain_replacements.json.fixture")).unwrap();
    let card = cards.iter().find(|card| card["name"] == name).unwrap();
    let mut text = format!("Mana cost: {}\nType: {}\n", card["mana_cost"].as_str().unwrap_or(""), card["type_line"].as_str().unwrap());
    if let (Some(power), Some(toughness)) = (card["power"].as_str(), card["toughness"].as_str()) {
        text.push_str(&format!("Power/Toughness: {power}/{toughness}\n"));
    }
    text.push_str(card["oracle_text"].as_str().unwrap());
    definitions(name, &text)
}
fn patrons(text: &str) -> [CardDefinition; 2] {
    definitions("Unlisted Life Patron", &format!("Mana cost: {{2}}{{W}}\nType: Creature — Cleric\nPower/Toughness: 2/3\n{text}"))
}
fn gain(game: &mut GameState, player: PlayerId, amount: i32) -> EffectOutcome {
    let source = game.new_object_id();
    let mut context = EffectContext::new_default(source, player);
    execute_effect(game, &Effect::gain_life(amount), &mut context).unwrap()
}
fn gain_amounts(outcome: &EffectOutcome) -> Vec<u32> {
    outcome.events.iter().filter_map(|event| event.downcast::<LifeGainEvent>().map(|gain| gain.amount)).collect()
}

#[test]
fn all_nine_frozen_cards_keep_complete_strict_artifacts() {
    for name in ["Angel of Vitality", "Bilbo, Birthday Celebrant", "Cleric Class", "Heron of Hope", "Honor Troll", "Knight of Dawn's Light", "Leyline of Hope", "Pest Rescuer", "Phial of Galadriel"] {
        for definition in fixture_definitions(name) { assert_eq!(definition.card.name, name); }
    }
}

#[test]
fn addition_modifies_one_event_and_tracks_live_controller_and_presence() {
    let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1);
    for definition in patrons("If you would gain life, you gain that much life plus 1 instead.") {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let host = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        assert_eq!(gain_amounts(&gain(&mut game, alice, 3)), vec![4]);
        assert_eq!(game.player(alice).unwrap().life, 24);
        assert!(gain_amounts(&gain(&mut game, alice, 0)).is_empty());
        assert_eq!(game.player(alice).unwrap().life, 24);
        assert_eq!(gain_amounts(&gain(&mut game, bob, 3)), vec![3]);
        game.set_current_controller(host, bob).unwrap();
        assert_eq!(gain_amounts(&gain(&mut game, alice, 3)), vec![3]);
        assert_eq!(gain_amounts(&gain(&mut game, bob, 3)), vec![4]);
        game.move_object_by_effect(host, Zone::Graveyard).unwrap();
        assert_eq!(gain_amounts(&gain(&mut game, bob, 3)), vec![3]);
    }
}

struct PreferReplacement(ObjectId);
impl DecisionMaker for PreferReplacement {
    fn decide_options(&mut self, _game: &GameState, context: &ironsmith::decisions::context::SelectOptionsContext) -> Vec<usize> {
        assert_eq!(context.player, PlayerId::from_index(0));
        vec![context.options.iter().find(|option| option.legal && option.object_id == Some(self.0))
            .or_else(|| context.options.iter().find(|option| option.legal)).unwrap().index]
    }
}

#[test]
fn affected_player_orders_addition_and_doubling_and_each_applies_once() {
    let alice = PlayerId::from_index(0);
    for (adder, doubler) in patrons("If you would gain life, you gain that much life plus 1 instead.").into_iter()
        .zip(patrons("If you would gain life, you gain twice that much life instead.")) {
        for add_first in [false, true] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let add = game.create_object_from_definition(&adder, alice, Zone::Battlefield);
            let double = game.create_object_from_definition(&doubler, alice, Zone::Battlefield);
            let mut decisions = PreferReplacement(if add_first { add } else { double });
            let mut context = EffectContext::new(add, alice, &mut decisions);
            let outcome = execute_effect(&mut game, &Effect::gain_life(3), &mut context).unwrap();
            let expected = if add_first { 8 } else { 7 };
            assert_eq!(gain_amounts(&outcome), vec![expected]);
            assert_eq!(game.player(alice).unwrap().life, 20 + expected as i32);
        }
    }
}

#[test]
fn separate_additive_instances_stack_but_cannot_create_forbidden_life_gain() {
    let alice = PlayerId::from_index(0);
    for definition in patrons("If you would gain life, you gain that much life plus 1 instead.") {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        assert_eq!(gain_amounts(&gain(&mut game, alice, 2)), vec![4]);
        game.effect_store.cant_effects.add_cant_gain_life(alice);
        assert!(gain_amounts(&gain(&mut game, alice, 2)).is_empty());
        assert_eq!(game.player(alice).unwrap().life, 24);
    }
}

fn fill_library(game: &mut GameState, player: PlayerId) {
    for index in 0..8 {
        game.create_object_from_card(&CardBuilder::new(CardId::new(), format!("Draw probe {index}")).card_types(vec![CardType::Artifact]).build(), player, Zone::Library);
    }
}
fn draw(game: &mut GameState, player: PlayerId) {
    let source = game.new_object_id();
    execute_effect(game, &Effect::draw(1), &mut EffectContext::new_default(source, player)).unwrap();
}

#[test]
fn conditional_artifact_keeps_both_live_life_and_empty_hand_replacements() {
    let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1);
    for definition in fixture_definitions("Phial of Galadriel") {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        game.player_mut(alice).unwrap().life = 4;
        assert_eq!(gain_amounts(&gain(&mut game, alice, 2)), vec![4]);
        game.player_mut(alice).unwrap().life = 5;
        assert_eq!(gain_amounts(&gain(&mut game, alice, 1)), vec![2]);
        game.player_mut(alice).unwrap().life = 6;
        assert_eq!(gain_amounts(&gain(&mut game, alice, 1)), vec![1]);
        game.player_mut(bob).unwrap().life = 4;
        assert_eq!(gain_amounts(&gain(&mut game, bob, 2)), vec![2]);
        fill_library(&mut game, alice); fill_library(&mut game, bob);
        draw(&mut game, alice);
        assert_eq!(game.player(alice).unwrap().hand.len(), 2, "one replacement creates two draws without recursive replacement");
        draw(&mut game, alice);
        assert_eq!(game.player(alice).unwrap().hand.len(), 3);
        draw(&mut game, bob);
        assert_eq!(game.player(bob).unwrap().hand.len(), 1);
    }
}
