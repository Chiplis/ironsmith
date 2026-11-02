//! Recipient-bound reminder animations, with native optionality and expiry.
use ironsmith_compiler_runtime::compile_builder_to_runtime_definition;
use ironsmith::decision::DecisionMaker;
use ironsmith::decisions::context::BooleanContext;
use ironsmith::{GameState, PlayerId, Zone};
use ironsmith::types::{CardType, Subtype, Supertype};
fn card(name: &str, text: &str) -> ironsmith::cards::CardDefinition {
    let builder = ironsmith_compiler::CardDefinitionBuilder::new(ironsmith_compiler::ids::CardId::new(), name).with_custom_keyword("Dare");
    ironsmith_compiler::word_claims::with_enforcement_options(true, true, || compile_builder_to_runtime_definition(builder, text, false)).unwrap()
}
struct Accept(bool);
impl DecisionMaker for Accept {
    fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool { self.0 }
}
#[test]
fn recipient_action_animation_preserves_optional_source_and_duration() {
    let builder = ironsmith_compiler::CardDefinitionBuilder::new(ironsmith_compiler::ids::CardId::new(), "Volunteer").with_custom_keyword("Dare");
    let text = "Type: Artifact Creature — Human\nPower/Toughness: 1/2\n{0}: You may have this creature become a legendary Rebel creature with base power and toughness 4/4 in addition to its other types until end of turn.";
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| ironsmith_compiler::word_claims::with_enforcement_options(true, true, || ironsmith_compiler_runtime::compile_builder_to_artifact(builder, text, false)));
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (artifact, direct) = result.unwrap();
    let decoded = ironsmith_compiled_artifact::CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(decoded, artifact);
    let materialized = ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap();
    for definition in [direct, materialized] { for accept in [false, true] {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let a = PlayerId::from_index(0);
        let source = game.create_object_from_definition(&definition, a, Zone::Battlefield);
        let other = game.create_object_from_definition(&card("Observer", "Type: Creature\nPower/Toughness: 2/3"), a, Zone::Battlefield);
        let ability = definition.abilities.iter().find_map(|ability| match &ability.kind { ironsmith::ability::AbilityKind::Activated(ability) => Some(ability), _ => None }).expect("expected activated animation");
        let mut dm = Accept(accept);
        let mut ctx = ironsmith::effects::EffectContext::new(source, a, &mut dm);
        for effect in ability.effects.all_effects() { ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap(); }
        let characteristics = game.current_characteristics(source).unwrap();
        assert!(characteristics.card_types.contains(&CardType::Artifact));
        assert!(characteristics.subtypes.contains(&Subtype::Human));
        assert_eq!(characteristics.subtypes.contains(&Subtype::Rebel), accept);
        assert_eq!(characteristics.supertypes.contains(&Supertype::Legendary), accept);
        assert_eq!(game.calculated_power(source), Some(if accept {4} else {1}));
        assert_eq!(game.calculated_toughness(source), Some(if accept {4} else {2}));
        assert_eq!(game.calculated_power(other), Some(2));
        ironsmith::turn::execute_cleanup_step(&mut game);
        assert_eq!(game.calculated_power(source), Some(1));
        assert!(!game.current_characteristics(source).unwrap().supertypes.contains(&Supertype::Legendary));
    } }
}
#[test]
fn recipient_action_actual_sources_compile_strictly() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/revolution-actions-complete.json")).unwrap();
    for mechanic in fixture["mechanics"].as_array().unwrap() {
        if mechanic["keyword"] != "Dare" { continue; }
        for source in mechanic["representative_cards"].as_array().unwrap() {
            for face in source["executable_faces"].as_array().unwrap() {
                let name = face["name"].as_str().unwrap();
                card(name, face["executable_text"].as_str().unwrap());
            }
        }
    }
}

#[path = "custom_final_next_wave_support.rs"]
mod characteristic_source_support;
fn characteristic_sources() -> serde_json::Value {
    serde_json::from_str(include_str!("fixtures/characteristic_transition_sources.json")).unwrap()
}
#[test]
fn actual_dare_and_stardom_observers_retain_complete_independent_source_routes() {
    let sources = characteristic_sources();
    assert_eq!(sources["cards"].as_array().unwrap().len(), 4);
    for source in sources["cards"].as_array().unwrap() {
        let [native, artifact] = characteristic_source_support::definitions(source)
            .unwrap_or_else(|error| panic!("{}: {error}", source["name"]));
        assert_eq!(serde_json::to_value(ironsmith_runtime_catalog::artifact_materializer::encode_runtime_definition(native).unwrap()).unwrap(),
            serde_json::to_value(ironsmith_runtime_catalog::artifact_materializer::encode_runtime_definition(artifact).unwrap()).unwrap(),
            "all semantic fields preserved for {}", source["name"]);
    }
}
#[test]
fn actual_meowsician_gain_notifies_itself_and_mob_instigator_then_repeated_dare_is_noop() {
    let sources = characteristic_sources();
    let find = |name: &str| sources["cards"].as_array().unwrap().iter()
        .find(|card| card["name"] == name).unwrap();
    let actors = characteristic_source_support::definitions(find("Meowsician")).unwrap();
    let observers = characteristic_source_support::definitions(find("Mob Instigator")).unwrap();
    for (actor, observer) in actors.into_iter().zip(observers) {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let a = PlayerId::from_index(0);
        let source = game.create_object_from_definition(&actor, a, Zone::Battlefield);
        let other = game.create_object_from_definition(&observer, a, Zone::Battlefield);
        game.refresh_continuous_state().unwrap();
        let ability = actor.abilities.iter().find_map(|ability| match &ability.kind {
            ironsmith::ability::AbilityKind::Activated(ability) => Some(ability), _ => None,
        }).expect("actual source's dare ability");
        let mut dm = Accept(true);
        {
            let mut ctx = ironsmith::effects::EffectContext::new(source, a, &mut dm);
            for effect in ability.effects.all_effects() { ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap(); }
        }
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut game, &mut queue, &mut dm).unwrap();
        assert_eq!(queue.entries.len(), 2, "self token observer and independent anthem observer each see the actual Rebel gain");
        ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
        while !game.stack.is_empty() { ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap(); }
        assert_eq!(game.calculated_power(source), Some(5));
        assert_eq!(game.calculated_power(other), Some(3));
        let warriors = game.battlefield.iter().filter(|id| game.object(**id).is_some_and(|object|
            matches!(object.kind, ironsmith::object::ObjectKind::Token))
            && game.current_characteristics(**id).unwrap().subtypes.contains(&Subtype::Warrior)).count();
        assert_eq!(warriors, 1, "the actual Meowsician observer creates its Warrior");
        {
            let mut ctx = ironsmith::effects::EffectContext::new(source, a, &mut dm);
            for effect in ability.effects.all_effects() { ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap(); }
        }
        ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut game, &mut queue, &mut dm).unwrap();
        assert!(queue.entries.is_empty(), "another Dare on an existing Rebel does not gain Rebel again");
    }
}
