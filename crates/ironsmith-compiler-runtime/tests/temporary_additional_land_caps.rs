//! Frozen complete bodies and runtime gates. Source-authored, deliberately unrun.
use ironsmith::cards::CardDefinition;
use ironsmith::effect::{Effect, Until, Value};
use ironsmith::effects::{AdditionalLandPlaysEffect, ChooseModeEffect, ChooseObjectsEffect, MayEffect, MoveToZoneEffect, ShuffleLibraryEffect};
use ironsmith::target::PlayerFilter;
use ironsmith::{CardType, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;

#[path = "temporary_additional_land_caps/runtime.rs"]
mod runtime;

fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!("../../../fixtures/temporary_additional_land_caps.json.fixture")).unwrap()
}

fn definitions(row: &serde_json::Value) -> [CardDefinition; 2] {
    let name = row["name"].as_str().unwrap();
    let text = row["text"].as_str().unwrap();
    assert_eq!(text, format!("Mana cost: {}\nType: {}\n{}", row["mana_cost"].as_str().unwrap(), row["type_line"].as_str().unwrap(), row["oracle_text"].as_str().unwrap()));
    let (direct, loss) = ironsmith_compiler::parse_loss::capture(||
        ironsmith_compiler_runtime::compile_to_runtime_definition(name, text, false));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let direct = direct.unwrap_or_else(|error| panic!("{name}: {error}"));
    let (compiled, loss) = ironsmith_compiler::parse_loss::capture(||
        ironsmith_compiler_runtime::compile_to_artifact(name, text, false));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let (artifact, _) = compiled.unwrap_or_else(|error| panic!("{name}: {error}"));
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    restored.validate().unwrap();
    assert_eq!(artifact, restored);
    let decoded = ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap();
    [direct, decoded]
}

fn collect(effect: &Effect, all: &mut Vec<Effect>) {
    all.push(effect.clone());
    effect.visit_child_effects(&mut |child| collect(child, all));
}
fn effects(definition: &CardDefinition) -> Vec<Effect> {
    let mut all = Vec::new();
    for effect in definition.spell_effect.as_ref().unwrap().all_effects() { collect(effect, &mut all); }
    all
}

#[test]
fn exact_full_bodies_survive_direct_and_artifact_routes_without_resolution_may() {
    let rows = fixtures();
    assert_eq!(rows.len(), 2);
    for row in &rows {
        let (id, count, mana) = match row["name"].as_str().unwrap() {
            "Summer Bloom" => ("e5df4597-1647-4ac2-bdb3-a517598d1431", 3, "{1}{G}"),
            "Journey of Discovery" => ("1c586d8a-9d1a-48a7-bb3e-9b2c0c329f8d", 2, "{2}{G}"),
            other => panic!("unexpected cohort member {other}"),
        };
        assert_eq!(row["oracle_id"], id);
        for definition in definitions(row) {
            assert_eq!(definition.card.name, row["name"].as_str().unwrap());
            assert_eq!(definition.card.mana_cost.as_ref().unwrap().to_oracle(), mana);
            assert_eq!(definition.card.card_types, vec![CardType::Sorcery]);
            assert!(definition.card.supertypes.is_empty());
            assert!(definition.card.subtypes.is_empty());
            assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
            let all = effects(&definition);
            assert!(!all.iter().any(|effect| effect.downcast_ref::<MayEffect>().is_some()));
            let grants: Vec<_> = all.iter().filter_map(|effect| effect.downcast_ref::<AdditionalLandPlaysEffect>()).collect();
            assert_eq!(grants.len(), 1);
            assert_eq!(grants[0].count, Value::Fixed(count));
            assert_eq!(grants[0].player, PlayerFilter::You);
            assert_eq!(grants[0].duration, Until::EndOfTurn);
            if count == 3 {
                assert!(definition.optional_costs.is_empty());
                assert!(!all.iter().any(|effect| effect.downcast_ref::<ChooseModeEffect>().is_some()));
                continue;
            }
            assert_eq!(definition.optional_costs.len(), 1);
            let cost = &definition.optional_costs[0];
            assert_eq!(cost.kind, ironsmith_core::OptionalCostKind::Entwine);
            assert_eq!(cost.cost.mana_cost().unwrap().to_oracle(), "{2}{G}");
            assert!(!cost.repeatable);
            let modal = all.iter().find_map(|effect| effect.downcast_ref::<ChooseModeEffect>()).unwrap();
            assert_eq!(modal.modes.len(), 2);
            assert_eq!(modal.min_choose_count, Value::Fixed(1));
            assert_eq!(modal.choose_count, Value::Fixed(1));
            assert!(modal.chooser.is_none());
            assert!(!modal.allow_repeated_modes);
            let mut search_mode = Vec::new();
            for effect in &modal.modes[0].effects { collect(effect, &mut search_mode); }
            let search = search_mode.iter().find_map(|effect| effect.downcast_ref::<ChooseObjectsEffect>()).unwrap();
            assert_eq!(search.count.min, 0);
            assert_eq!(search.count.max, Some(2));
            assert_eq!(search.zone, Some(Zone::Library));
            assert_eq!(search.chooser, PlayerFilter::You);
            assert_eq!(search.filter.owner, Some(PlayerFilter::You));
            assert_eq!(search.filter.card_types, vec![CardType::Land]);
            assert!(search.filter.supertypes.contains(&ironsmith::Supertype::Basic));
            assert!(search.is_search && search.reveal);
            assert_eq!(search.search_mode, ironsmith_core::effect::SearchSelectionMode::Optional);
            let move_position = search_mode.iter().position(|effect| effect.downcast_ref::<MoveToZoneEffect>().is_some_and(|move_| move_.zone == Zone::Hand)).unwrap();
            let shuffle_position = search_mode.iter().position(|effect| effect.downcast_ref::<ShuffleLibraryEffect>().is_some_and(|shuffle| shuffle.player == PlayerFilter::You)).unwrap();
            assert!(move_position < shuffle_position);
            assert!(!search_mode.iter().any(|effect| effect.downcast_ref::<AdditionalLandPlaysEffect>().is_some()));
            let mut grant_mode = Vec::new();
            for effect in &modal.modes[1].effects { collect(effect, &mut grant_mode); }
            assert!(grant_mode.iter().any(|effect| effect.downcast_ref::<AdditionalLandPlaysEffect>().is_some()));
            assert!(!grant_mode.iter().any(|effect| effect.downcast_ref::<ChooseObjectsEffect>().is_some()));
        }
    }
}

#[test]
fn malformed_temporary_permission_bodies_fail_closed_on_both_routes() {
    for text in [
        "You may play up to additional lands this turn.",
        "You may play up to up to two additional lands this turn.",
        "You may play two nonsense additional lands this turn.",
    ] {
        let source = format!("Mana cost: {{1}}{{G}}\nType: Sorcery\n{text}");
        assert!(ironsmith_compiler_runtime::compile_to_runtime_definition("Malformed cap", &source, false).is_err(), "{text}");
        assert!(ironsmith_compiler_runtime::compile_to_artifact("Malformed cap", &source, false).is_err(), "{text}");
    }
}

#[test]
fn existing_bare_cap_and_explore_style_sequence_keep_controller_and_draw() {
    for (text, cap) in [
        ("You may play an additional land this turn. Draw a card.", 1),
        ("You may play two additional lands this turn. Draw a card.", 2),
    ] {
        let row = serde_json::json!({
            "name": "Temporary cap neighboring control",
            "mana_cost": "{1}{G}", "type_line": "Sorcery", "oracle_text": text,
            "text": format!("Mana cost: {{1}}{{G}}\nType: Sorcery\n{text}"),
        });
        for definition in definitions(&row) {
            assert!(definition.abilities.is_empty(), "temporary permission is not a battlefield static ability");
            let all = effects(&definition);
            let grant = all.iter().find_map(|effect| effect.downcast_ref::<AdditionalLandPlaysEffect>()).unwrap();
            assert_eq!(grant.count, Value::Fixed(cap));
            assert_eq!(grant.player, PlayerFilter::You);
            assert_eq!(grant.duration, Until::EndOfTurn);
            assert!(!all.iter().any(|effect| effect.downcast_ref::<MayEffect>().is_some()));
            assert!(all.iter().any(|effect| effect.downcast_ref::<ironsmith::effects::DrawCardsEffect>().is_some_and(|draw| draw.count == Value::Fixed(1))));
        }
    }
}
