//! Fresh-main chosen-type regressions. Authored source tests; UNRUN.
use ironsmith::card::PowerToughness;
use ironsmith::cards::{CardDefinition, builders::CardDefinitionBuilder};
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::object::ObjectKind;
use ironsmith::game_state::StackEntry;
use ironsmith::{CardId, CardType, GameState, ObjectId, PlayerId, Subtype, Zone};
use ironsmith_compiled_artifact::{ArtifactValidationError, CompiledCardArtifact, ENGINE_SCHEMA_HASH, FORMAT_VERSION};
use ironsmith_compiler::parse_loss;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_core::{AbilityKind, StaticAbilityPayload};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;

const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);

fn rows() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!("../../../fixtures/chosen_type_domain_regressions.json.fixture")).unwrap()
}
fn artifact(name: &str, text: &str) -> CompiledCardArtifact {
    let (result, loss) = parse_loss::capture(|| compile_to_artifact(name, text, false));
    let (artifact, _) = result.unwrap_or_else(|error| panic!("{name}: {error}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    artifact.validate().unwrap();
    artifact
}
fn routes(row: &serde_json::Value) -> [CardDefinition; 2] {
    let name = row["name"].as_str().unwrap();
    let text = row["metadata"].as_str().unwrap().to_owned() + row["oracle_text"].as_str().unwrap();
    let (direct, loss) = parse_loss::capture(|| compile_to_runtime_definition(name, &text, false));
    let direct = direct.unwrap();
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let encoded = artifact(name, &text);
    let decoded = CompiledCardArtifact::from_json(&encoded.to_json().unwrap()).unwrap();
    assert_eq!(encoded, decoded);
    [direct, materialize_artifact(&decoded).unwrap()]
}

fn chosen_filters(artifact: &CompiledCardArtifact) -> Vec<ironsmith_core::ObjectFilter> {
    artifact.payload.definition.abilities.iter().filter_map(|ability| {
        let AbilityKind::Static(model) = &ability.kind else { return None };
        let StaticAbilityPayload::AddChosenCreatureType { filter, .. } = &model.payload else { return None };
        Some(filter.clone())
    }).collect()
}

#[test]
fn full_bodies_keep_three_scopes_and_round_trip_current_artifacts_and_text() {
    for row in rows() {
        let name = row["name"].as_str().unwrap();
        let metadata = row["metadata"].as_str().unwrap();
        let compiled = artifact(name, &(metadata.to_owned() + row["oracle_text"].as_str().unwrap()));
        let abilities = &compiled.payload.definition.abilities;
        let additions: Vec<_> = abilities.iter().filter(|ability| matches!(&ability.kind,
            AbilityKind::Static(model) if matches!(&model.payload, StaticAbilityPayload::AddChosenCreatureType { .. }))).collect();
        assert_eq!(additions.len(), 3, "{name}: {abilities:#?}");
        assert!(additions.iter().all(|ability| ability.functional_zones == [Zone::Battlefield]));
        assert!(abilities.iter().any(|ability| matches!(&ability.kind,
            AbilityKind::Static(model) if model.id() == ironsmith_core::StaticAbilityId::ChooseCreatureTypeAsEnters)));
        if name == "Leyline of Transformation" {
            assert!(abilities.iter().any(|ability| matches!(&ability.kind,
                AbilityKind::Static(model) if model.id() == ironsmith_core::StaticAbilityId::PregameAction)));
        }
        if name == "Rukarumel, Biologist" {
            assert!(abilities.iter().any(|ability| matches!(&ability.kind, AbilityKind::Activated(_))));
        }
        for definition in routes(&row) {
            assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
            let rendered = ironsmith_text::compiled_text_lines(&definition).join("\n");
            let reparsed = artifact(name, &(metadata.to_owned() + &rendered));
            assert_eq!(reparsed.payload.definition.abilities.len(), abilities.len(), "{rendered}");
            assert_eq!(chosen_filters(&reparsed), chosen_filters(&compiled),
                "canonical text must retain executable chosen-type recipient scopes: {rendered}");
        }
        assert_eq!(compiled.format_version, FORMAT_VERSION);
        assert_eq!(compiled.engine_schema_hash, ENGINE_SCHEMA_HASH);
        let mut obsolete = compiled.clone();
        obsolete.format_version = FORMAT_VERSION - 1;
        obsolete.refresh_checksum();
        assert!(matches!(obsolete.validate(), Err(ArtifactValidationError::UnsupportedFormat { .. })));
        assert!(materialize_artifact(&obsolete).is_err());
        let mut wrong_schema = compiled;
        wrong_schema.engine_schema_hash = "stale-chosen-type-domain-schema".into();
        wrong_schema.refresh_checksum();
        assert!(matches!(wrong_schema.validate(), Err(ArtifactValidationError::EngineSchemaMismatch { .. })));
        assert!(materialize_artifact(&wrong_schema).is_err());
    }
}

fn witness(game: &mut GameState, owner: PlayerId, controller: PlayerId, zone: Zone,
    creature: bool, subtype: Subtype, token: bool) -> ObjectId {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Scope witness")
        .card_types(vec![if creature { CardType::Creature } else { CardType::Artifact }])
        .subtypes(if creature { vec![subtype] } else { vec![] })
        .power_toughness(PowerToughness::fixed(2, 2)).build();
    let id = game.create_object_from_definition(&definition, owner, zone);
    let object = game.object_mut(id).unwrap();
    object.initial_controller = controller;
    if token { object.kind = ObjectKind::Token; }
    if zone == Zone::Stack { game.push_to_stack(StackEntry::new(id, controller)); }
    id
}

#[test]
fn native_direct_and_materialized_scopes_follow_control_ownership_and_source_lifetime() {
    for row in rows() {
        let rukarumel = row["name"] == "Rukarumel, Biologist";
        for definition in routes(&row) {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let hand = game.create_object_from_definition(&definition, A, Zone::Hand);
            let receipt = game.move_object_with_etb_processing_with_dm(hand, Zone::Battlefield,
                &mut SelectFirstDecisionMaker).unwrap();
            assert!(!receipt.pending);
            let source = receipt.original.into_result().unwrap().new_id;
            assert!(game.chosen_creature_type(source).is_some(), "full body must perform its entry choice");
            game.set_chosen_creature_type(source, Subtype::Dragon);
            let controlled = witness(&mut game, B, A, Zone::Battlefield, true, Subtype::Bear, false);
            let merely_owned = witness(&mut game, A, B, Zone::Battlefield, true, Subtype::Bear, false);
            let token_sliver = witness(&mut game, A, A, Zone::Battlefield, true, Subtype::Sliver, true);
            let token_bear = witness(&mut game, A, A, Zone::Battlefield, true, Subtype::Bear, true);
            let opposing_sliver = witness(&mut game, B, B, Zone::Battlefield, true, Subtype::Sliver, true);
            let mut yes = vec![controlled, token_sliver];
            let mut no = vec![merely_owned, opposing_sliver];
            if rukarumel { no.push(token_bear); } else { yes.push(token_bear); }
            for zone in [Zone::Hand, Zone::Library, Zone::Graveyard, Zone::Exile, Zone::Command] {
                yes.push(witness(&mut game, A, B, zone, true, Subtype::Bear, false));
                no.push(witness(&mut game, B, A, zone, true, Subtype::Bear, false));
                no.push(witness(&mut game, A, A, zone, false, Subtype::Bear, false));
            }
            // The stack is covered independently by controlled spells and owned
            // creature cards: either relationship is sufficient there.
            yes.push(witness(&mut game, B, A, Zone::Stack, true, Subtype::Bear, false));
            yes.push(witness(&mut game, A, B, Zone::Stack, true, Subtype::Bear, false));
            no.push(witness(&mut game, B, B, Zone::Stack, true, Subtype::Bear, false));
            no.push(witness(&mut game, A, A, Zone::Stack, false, Subtype::Bear, false));
            no.push(witness(&mut game, A, A, Zone::OutsideGame, true, Subtype::Bear, false));
            game.refresh_continuous_state().unwrap();
            for id in &yes { assert!(game.current_has_subtype(*id, Subtype::Dragon), "{:?} {id:?}", row["name"]); }
            for id in &no { assert!(!game.current_has_subtype(*id, Subtype::Dragon), "{:?} {id:?}", row["name"]); }
            assert!(game.current_has_subtype(controlled, Subtype::Bear));
            assert!(game.current_has_subtype(token_sliver, Subtype::Sliver));
            game.set_current_controller(controlled, B).unwrap();
            game.refresh_continuous_state().unwrap();
            assert!(!game.current_has_subtype(controlled, Subtype::Dragon), "battlefield scope is live control");
            game.set_current_controller(controlled, A).unwrap();
            game.move_object_by_effect(source, Zone::Exile).unwrap();
            game.refresh_continuous_state().unwrap();
            for id in yes { assert!(!game.current_has_subtype(id, Subtype::Dragon)); }
        }
    }
}

#[test]
fn unsupported_qualifiers_cannot_be_silently_admitted() {
    for text in [
        "Creature cards you own that aren't on the battlefield mystery are the chosen type in addition to their other types.",
        "Slivers you control and mystery nontoken creatures you control are the chosen type in addition to their other creature types.",
        "Creatures you control are the chosen type in addition to their other types. The same is true for creature cards you own that aren't on the battlefield mystery.",
    ] {
        let (result, loss) = parse_loss::capture(|| compile_to_artifact("Unsupported chosen domain",
            &format!("Type: Enchantment\nAs this enchantment enters, choose a creature type.\n{text}"), false));
        assert!(result.is_err() || loss.is_lossy(), "must not cleanly admit {text}");
    }
}

#[test]
fn independent_nominal_qualifiers_stay_branch_local_in_both_orders_and_routes() {
    for qualifier in ["nontoken", "tapped"] {
        let restricted = format!("{qualifier} creatures you control");
        for subject in [format!("{restricted} and Slivers you control"),
            format!("Slivers you control and {restricted}")] {
            let row = serde_json::json!({
                "name": "Independent chosen-type union",
                "metadata": "Type: Enchantment\n",
                "oracle_text": format!("As this enchantment enters, choose a creature type.\n{subject} are the chosen type in addition to their other creature types.")
            });
            for definition in routes(&row) {
                let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
                game.set_chosen_creature_type(source, Subtype::Dragon);
                let token_sliver = witness(&mut game, B, A, Zone::Battlefield, true, Subtype::Sliver, true);
                let token_bear = witness(&mut game, A, A, Zone::Battlefield, true, Subtype::Bear, true);
                let ordinary_bear = witness(&mut game, A, A, Zone::Battlefield, true, Subtype::Bear, false);
                let tapped_bear = witness(&mut game, A, A, Zone::Battlefield, true, Subtype::Bear, false);
                game.tap(tapped_bear);
                let opposing_sliver = witness(&mut game, A, B, Zone::Battlefield, true, Subtype::Sliver, true);
                game.refresh_continuous_state().unwrap();
                assert!(game.current_has_subtype(token_sliver, Subtype::Dragon), "{subject}: untapped token Sliver");
                assert!(!game.current_has_subtype(token_bear, Subtype::Dragon), "{subject}: untapped token Bear");
                assert_eq!(game.current_has_subtype(ordinary_bear, Subtype::Dragon), qualifier == "nontoken", "{subject}");
                assert!(game.current_has_subtype(tapped_bear, Subtype::Dragon), "{subject}");
                assert!(!game.current_has_subtype(opposing_sliver, Subtype::Dragon), "{subject}");
                let text = ironsmith_text::compiled_text_lines(&definition).join("\n");
                let original = artifact("Independent chosen-type union", &format!("Type: Enchantment\n{}", row["oracle_text"].as_str().unwrap()));
                let reparsed = artifact("Independent chosen-type union", &format!("Type: Enchantment\n{text}"));
                assert_eq!(chosen_filters(&original), chosen_filters(&reparsed), "{text}");
            }
        }
    }
}

#[test]
fn independent_card_domain_keeps_ownership_separate_from_battlefield_control_in_both_orders() {
    for subject in [
        "nontoken creatures you control and creature cards you own that aren't on the battlefield",
        "creature cards you own that aren't on the battlefield and nontoken creatures you control",
    ] {
        let row = serde_json::json!({
            "name": "Mixed chosen-type domain",
            "metadata": "Type: Enchantment\n",
            "oracle_text": format!("As this enchantment enters, choose a creature type.\n{subject} are the chosen type in addition to their other creature types.")
        });
        for definition in routes(&row) {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            game.set_chosen_creature_type(source, Subtype::Dragon);
            let controlled = witness(&mut game, B, A, Zone::Battlefield, true, Subtype::Bear, false);
            let owned = witness(&mut game, A, B, Zone::Graveyard, true, Subtype::Bear, false);
            let not_controlled = witness(&mut game, A, B, Zone::Battlefield, true, Subtype::Bear, false);
            let not_owned = witness(&mut game, B, A, Zone::Graveyard, true, Subtype::Bear, false);
            let token = witness(&mut game, A, A, Zone::Battlefield, true, Subtype::Bear, true);
            game.refresh_continuous_state().unwrap();
            for id in [controlled, owned] { assert!(game.current_has_subtype(id, Subtype::Dragon), "{subject}"); }
            for id in [not_controlled, not_owned, token] { assert!(!game.current_has_subtype(id, Subtype::Dragon), "{subject}"); }
        }
    }
}

#[test]
fn full_body_admission_rejects_unconsumed_qualifiers_on_either_union_arm() {
    for bad in ["mystery creatures you control", "creatures you control mystery",
        "creatures you control until end of turn", "creatures you control and",
        "\"creatures\" you control", "creatures you control \"mystery\""] {
        for subject in [format!("{bad} and Slivers you control"),
            format!("Slivers you control and {bad}")] {
            let text = format!("Type: Enchantment\nAs this enchantment enters, choose a creature type.\n{subject} are the chosen type in addition to their other creature types.");
            let (result, loss) = parse_loss::capture(|| compile_to_artifact("Unsupported union arm", &text, false));
            assert!(result.is_err() || loss.is_lossy(), "must not cleanly admit {subject}");
        }
    }
}
