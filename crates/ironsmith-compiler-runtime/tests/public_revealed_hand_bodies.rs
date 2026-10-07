//! Exact original-source evidence only: UNVALIDATED / UNRUN.
//! No corpus-recovery credit. Browser proof coverage is reviewed separately.
use ironsmith::ability::AbilityKind;
use ironsmith::card::PowerToughness;
use ironsmith::cards::CardDefinition;
use ironsmith::color::ColorSet;
use ironsmith::continuous::Modification;
use ironsmith::effect::Until;
use ironsmith::effects::{ApplyContinuousEffect, EffectContext, EffectExecutor};
use ironsmith::mana::{ManaCost, ManaSymbol};
use ironsmith::rules::{combat::can_block, state_based::apply_state_based_actions};
use ironsmith::static_abilities::{StaticAbility, StaticAbilityId};
use ironsmith::{CardType, ChooseSpec, GameState, ObjectId, PlayerId, Subtype, Supertype, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler::parse_loss;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;

const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);

fn rows() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!("../../../fixtures/public_revealed_hand_bodies.json.fixture")).unwrap()
}
fn source(row: &serde_json::Value) -> String {
    let mut text = format!("Mana cost: {}\nType: {}\n", row["mana_cost"].as_str().unwrap(), row["type_line"].as_str().unwrap());
    if let (Some(p), Some(t)) = (row["power"].as_str(), row["toughness"].as_str()) {
        text.push_str(&format!("Power/Toughness: {p}/{t}\n"));
    }
    text.push_str(row["oracle_text"].as_str().unwrap());
    text
}
fn definitions(name: &str) -> [CardDefinition; 2] {
    let row = rows().into_iter().find(|row| row["name"] == name).unwrap();
    let text = source(&row);
    // These are two independent compiles of the entire original body. The
    // materialized sibling returned by compile_to_artifact is not the direct route.
    let (direct, loss) = parse_loss::capture(|| compile_to_runtime_definition(name, &text, false));
    let direct = direct.unwrap_or_else(|error| panic!("direct {name}: {error}"));
    assert!(!loss.is_lossy(), "direct {name}: {}", loss.reasons_text());
    let (compiled, loss) = parse_loss::capture(|| compile_to_artifact(name, &text, false));
    let (artifact, _) = compiled.unwrap_or_else(|error| panic!("artifact {name}: {error}"));
    assert!(!loss.is_lossy(), "artifact {name}: {}", loss.reasons_text());
    artifact.validate().unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    restored.validate().unwrap();
    assert_eq!(artifact, restored);
    let mut corrupted = restored.clone();
    corrupted.payload_checksum = "incorrect-checksum".into();
    assert!(corrupted.validate().is_err());
    assert!(materialize_artifact(&corrupted).is_err());
    let definitions = [direct, materialize_artifact(&restored).unwrap()];
    for definition in &definitions {
        assert_eq!(definition.card.name, name);
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(definition));
        assert!(definition.spell_effect.is_none());
        let ids: Vec<_> = definition.abilities.iter().map(|ability| match &ability.kind {
            AbilityKind::Static(ability) => ability.id(),
            other => panic!("unexpected nonstatic ability: {other:?}"),
        }).collect();
        assert_eq!(ids.iter().filter(|&&id| id == StaticAbilityId::PlayersPlayWithHandsRevealed).count(), 1);
        assert_eq!(definition.card.color_indicator, None);
        assert_eq!(ironsmith_text::compiled_text_lines(definition).join("\n"), row["oracle_text"].as_str().unwrap());
        if name == "Revelation" {
            assert_eq!(ids, vec![StaticAbilityId::PlayersPlayWithHandsRevealed]);
            assert_eq!(definition.card.mana_cost, Some(ManaCost::from_symbols(vec![ManaSymbol::Green])));
            assert_eq!(definition.card.colors(), ColorSet::GREEN);
            assert_eq!(definition.card.supertypes, vec![Supertype::World]);
            assert_eq!(definition.card.card_types, vec![CardType::Enchantment]);
            assert!(definition.card.subtypes.is_empty());
            assert!(definition.card.power_toughness.is_none());
        } else {
            assert_eq!(ids, vec![StaticAbilityId::Flying, StaticAbilityId::PlayersPlayWithHandsRevealed]);
            assert_eq!(definition.card.mana_cost, Some(ManaCost::from_symbols(vec![ManaSymbol::Generic(2), ManaSymbol::Blue])));
            assert_eq!(definition.card.colors(), ColorSet::BLUE);
            assert!(definition.card.supertypes.is_empty());
            assert_eq!(definition.card.card_types, vec![CardType::Creature]);
            assert_eq!(definition.card.subtypes, vec![Subtype::Illusion]);
            assert_eq!(definition.card.power_toughness, Some(PowerToughness::fixed(1, 3)));
        }
    }
    definitions
}
fn game() -> GameState { GameState::new(vec!["Alice".into(), "Bob".into()], 20) }
fn live_sources(game: &GameState) -> Vec<ObjectId> {
    game.battlefield.iter().copied().filter(|&id| !game.is_phased_out(id)
        && game.object_has_static_ability_id(id, StaticAbilityId::PlayersPlayWithHandsRevealed)).collect()
}

#[test]
fn exact_frozen_ids_metadata_text_and_strict_transport_are_independently_retained() {
    let rows = rows();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["oracle_id"], "251ff4e4-3be4-424e-8dd7-eb4ede7b415c");
    assert_eq!(rows[1]["oracle_id"], "59015068-6868-4235-981b-96137123c685");
    for row in rows { definitions(row["name"].as_str().unwrap()); }
}

#[test]
fn complete_revelation_obeys_world_removal_independently_of_controller() {
    for definition in definitions("Revelation") {
        let mut game = game();
        let old = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let next_world = compile_to_runtime_definition("New World witness", "Type: World Enchantment", false).unwrap();
        let newest = game.create_object_from_definition(&next_world, B, Zone::Battlefield);
        assert!(apply_state_based_actions(&mut game).unwrap());
        assert!(game.object(old).is_none());
        assert!(game.object(newest).is_some_and(|object| object.zone == Zone::Battlefield));
        assert!(game.player(A).unwrap().graveyard.iter().any(|&id| game.object(id).unwrap().name == "Revelation"));
        assert!(live_sources(&game).is_empty());
    }
}

#[test]
fn complete_wandering_eye_has_real_flying_and_dies_to_lethal_damage() {
    for definition in definitions("Wandering Eye") {
        let mut game = game();
        let eye = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let ground = compile_to_runtime_definition("Ground witness", "Type: Creature — Bear\nPower/Toughness: 2/2", false).unwrap();
        let ground = game.create_object_from_definition(&ground, B, Zone::Battlefield);
        assert_eq!((game.current_power(eye), game.current_toughness(eye)), (Some(1), Some(3)));
        assert!(game.object_has_ability(eye, &StaticAbility::flying()));
        assert!(!can_block(game.object(eye).unwrap(), game.object(ground).unwrap(), &game));
        let reach = compile_to_runtime_definition("Reach witness", "Type: Creature — Spider\nPower/Toughness: 1/4\nReach", false).unwrap();
        let reach = game.create_object_from_definition(&reach, B, Zone::Battlefield);
        assert!(can_block(game.object(eye).unwrap(), game.object(reach).unwrap(), &game));
        game.mark_damage(eye, 3);
        assert!(apply_state_based_actions(&mut game).unwrap());
        assert!(game.object(eye).is_none());
        assert!(game.player(A).unwrap().graveyard.iter().any(|&id| game.object(id).unwrap().name == "Wandering Eye"));
        assert!(live_sources(&game).is_empty());
    }
}

#[test]
fn each_complete_body_loses_its_real_visibility_ability_through_layers_and_phasing() {
    for name in ["Revelation", "Wandering Eye"] { for definition in definitions(name) {
        let mut game = game();
        let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        assert_eq!(live_sources(&game), vec![host]);
        game.set_current_controller(host, B).unwrap();
        assert_eq!(live_sources(&game), vec![host]);
        game.phase_out(host);
        assert!(live_sources(&game).is_empty());
        game.phase_in(host);
        assert_eq!(live_sources(&game), vec![host]);
        ApplyContinuousEffect::with_spec(ChooseSpec::SpecificObject(host), Modification::RemoveAllAbilities, Until::EndOfTurn)
            .execute(&mut game, &mut EffectContext::new_default(host, B)).unwrap();
        game.refresh_continuous_state().unwrap();
        assert!(live_sources(&game).is_empty());
        if name == "Wandering Eye" { assert!(!game.object_has_ability(host, &StaticAbility::flying())); }
    } }
}
