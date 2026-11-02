//! Bare described entry is independent of manifest's printed-cost permission.
use ironsmith::{CardDefinition, Effect, GameState, PlayerId, Zone};
use ironsmith::effects::{EffectContext, ResolvedTarget, execute_effect};
use ironsmith_compiled_artifact::CompiledCardArtifact;
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
fn definitions(text: &str) -> [CardDefinition; 2] { definitions_named("Described entry program", text) }
fn definitions_named(name: &str, text: &str) -> [CardDefinition; 2] {
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| ironsmith_compiler::word_claims::with_enforcement_options(true, true, || ironsmith_compiler_runtime::compile_to_artifact(name, text, false)));
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (artifact, direct) = result.unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(restored, artifact);
    [direct, ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()]
}
#[test]
fn exile_then_bare_face_down_entry_preserves_owner_without_manifest_permission() {
    for definition in definitions("Type: Sorcery\nExile target creature. Put that card onto the battlefield face down under its owner's control as a 2/2 creature. Give that creature the submerged status until it turns face up.") {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let source = game.create_object_from_definition(&definition, A, Zone::Stack);
        let body = ironsmith_compiler_runtime::compile_to_runtime_definition("Printed witness", "Mana cost: {G}\nType: Creature — Elf\nPower/Toughness: 4/5\nFlying", false).unwrap();
        let target = game.create_object_from_definition(&body, B, Zone::Battlefield);
        game.set_current_controller(target, A).unwrap();
        let stable = game.object(target).unwrap().stable_id;
        let effects = definition.spell_effect.as_ref().unwrap().all_effects_owned();
        execute_effect(&mut game, &Effect::new(ironsmith::effects::SequenceEffect::new(effects)), &mut EffectContext::new_default(source, A).with_targets(vec![ResolvedTarget::Object(target)])).unwrap();
        let returned = game.find_object_by_stable_id(stable).unwrap();
        assert_ne!(returned, target, "exile and return create fresh incarnations");
        assert_eq!(game.object(returned).unwrap().zone, Zone::Battlefield);
        assert_eq!(game.controller_of_id(returned).unwrap(), B);
        assert!(game.is_face_down(returned));
        assert_eq!(game.calculated_power(returned), Some(2));
        assert_eq!(game.calculated_toughness(returned), Some(2));
        assert!(!game.is_manifested(returned));
        assert!(!game.is_cloaked(returned));
        assert!(!game.has_printed_cost_turn_face_up_permission(returned));
        assert!(game.has_custom_status(returned, "submerged"));
        game.set_face_up(returned).unwrap();
        assert!(!game.has_custom_status(returned, "submerged"));
        assert_eq!(game.calculated_power(returned), Some(4));
        assert_eq!(game.calculated_toughness(returned), Some(5));
    }
}
#[test]
fn owner_control_is_per_card_for_a_mixed_owner_collection() {
    for definition in definitions("Type: Sorcery\nPut all creature cards from all graveyards onto the battlefield face down under its owner's control as a 2/2 creature.") {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let source = game.create_object_from_definition(&definition, A, Zone::Stack);
        let body = ironsmith_compiler_runtime::compile_to_runtime_definition("Collection witness", "Mana cost: {G}\nType: Creature\nPower/Toughness: 4/5", false).unwrap();
        let cards: Vec<_> = [A, B].into_iter().map(|owner| { let id = game.create_object_from_definition(&body, owner, Zone::Graveyard); (game.object(id).unwrap().stable_id, owner) }).collect();
        let effects = definition.spell_effect.as_ref().unwrap().all_effects_owned();
        execute_effect(&mut game, &Effect::new(ironsmith::effects::SequenceEffect::new(effects)), &mut EffectContext::new_default(source, A)).unwrap();
        for (stable, owner) in cards {
            let id = game.find_object_by_stable_id(stable).unwrap();
            assert_eq!(game.object(id).unwrap().zone, Zone::Battlefield);
            assert_eq!(game.controller_of_id(id).unwrap(), owner);
            assert!(!game.has_printed_cost_turn_face_up_permission(id));
        }
    }
}

#[test]
fn event_status_applications_coexist_with_permanent_and_other_event_scopes() {
    use ironsmith_core::CustomStatusExpiryEvent::{PhasesIn, TurnsFaceUp};
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    let body = ironsmith_compiler_runtime::compile_to_runtime_definition("Scope witness", "Type: Creature\nPower/Toughness: 2/2", false).unwrap();
    let id = game.create_object_from_definition(&body, A, Zone::Battlefield);
    game.set_face_down(id);
    assert!(game.apply_custom_status_until_event(id, "shared", TurnsFaceUp));
    assert!(!game.apply_custom_status_until_event(id, "shared", PhasesIn));
    assert!(!game.apply_custom_status(id, "shared", false));
    assert!(game.apply_custom_status_until_event(id, "temporary", TurnsFaceUp));
    game.set_face_up(id).unwrap();
    assert!(!game.has_custom_status(id, "temporary"));
    assert!(game.has_custom_status(id, "shared"), "other applications survive face-up expiry");
    game.phase_out(id);
    game.phase_in(id);
    assert!(game.has_custom_status(id, "shared"), "permanent application survives phase-in expiry");
    assert!(game.apply_custom_status_until_event(id, "voyaging", PhasesIn));
    game.phase_out(id);
    assert!(game.has_custom_status(id, "voyaging"));
    game.phase_in(id);
    assert!(!game.has_custom_status(id, "voyaging"));
    assert!(game.has_custom_status(id, "shared"));
}

#[test]
fn named_phase_in_expiry_roundtrips_through_the_ordinary_status_reader() {
    for definition in definitions("Type: Sorcery\nGive target creature the travelling status until it phases in.") {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let source = game.create_object_from_definition(&definition, A, Zone::Stack);
        let body = ironsmith_compiler_runtime::compile_to_runtime_definition("Phasing witness", "Type: Creature\nPower/Toughness: 2/2", false).unwrap();
        let id = game.create_object_from_definition(&body, A, Zone::Battlefield);
        let effects = definition.spell_effect.as_ref().unwrap().all_effects_owned();
        execute_effect(&mut game, &Effect::new(ironsmith::effects::SequenceEffect::new(effects)), &mut EffectContext::new_default(source, A).with_targets(vec![ResolvedTarget::Object(id)])).unwrap();
        assert!(game.has_custom_status(id, "travelling"));
        game.phase_out(id);
        game.phase_in(id);
        assert!(!game.has_custom_status(id, "travelling"));
    }
}

#[test]
fn every_complete_submerge_source_compiles_with_structural_artifact_parity() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-linked-face-down.json")).unwrap();
    let cards = fixture["mechanics"][0]["representative_cards"].as_array().unwrap();
    assert_eq!(cards.len(), 15);
    for card in cards {
        assert_eq!(card["rewrite_status"], "frontend_materialized_unverified", "{}", card["name"]);
        for definition in definitions_named(card["name"].as_str().unwrap(), card["executable_text"].as_str().unwrap()) {
            assert!(!format!("{definition:?}").contains("Unimplemented"), "{}", card["name"]);
        }
    }
}
