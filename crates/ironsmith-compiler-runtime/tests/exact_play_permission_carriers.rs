//! Typed carrier scenarios only. No frozen card credit; all scenarios unrun.
use ironsmith_core::{AbilityKind, StaticAbilityPayload};
use ironsmith_core::value_model::ManaSpendMode;
use ironsmith_compiler_runtime::{compile_to_artifact, into_runtime_definition};

#[test]
fn default_shapes_and_explicit_modes_survive_direct_and_artifact_materializers() {
    let source = "Type: Enchantment\nYou may play lands from your graveyard.";
    let (baseline, _) = compile_to_artifact("Carrier probe", source, false).unwrap();
    let old = baseline.to_json().unwrap();
    assert!(!std::str::from_utf8(&old).unwrap().contains("cast_mana_spend_mode"));
    ironsmith_compiled_artifact::CompiledCardArtifact::from_json(&old).unwrap().validate().unwrap();
    for mode in [ManaSpendMode::AnyColor, ManaSpendMode::AnyType] {
        let mut compiled = ironsmith_compiler::CompilerFacade::new().compile_definition(
            ironsmith_compiler::CardDefinitionBuilder::new(ironsmith::CardId::new(), "Carrier probe"),
            source, ironsmith_compiler::CompilePolicy { allow_unsupported: false }).unwrap().definition;
        let mut found = false;
        for ability in &mut compiled.abilities {
            if let AbilityKind::Static(ability) = &mut ability.kind
                && let StaticAbilityPayload::Grants(spec) = &mut ability.payload {
                spec.cast_mana_spend_mode = mode; found = true;
            }
        }
        assert!(found, "the grammar must construct the actual typed grant carrier");
        let mut artifact = baseline.clone();
        artifact.payload.definition = ironsmith_compiled_artifact::wire_definition_from_serializable(&compiled).unwrap();
        artifact.refresh_checksum();
        artifact.validate().unwrap();
        let decoded = ironsmith_compiled_artifact::CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
        assert_eq!(artifact, decoded);
        let direct = into_runtime_definition(compiled).unwrap();
        let loaded = ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap();
        for definition in [direct, loaded] {
            let retained = definition.abilities.iter().find_map(|ability| match &ability.kind {
                ironsmith::ability::AbilityKind::Static(ability) => ability.grant_spec(), _ => None,
            }).unwrap();
            assert_eq!(retained.cast_mana_spend_mode, mode);
        }
    }
}

#[test]
fn tagged_effect_omits_legacy_false_and_retains_explicit_selected_permission_mode() {
    let legacy = ironsmith_core::GrantPlayTaggedEffect::<ironsmith_compiled_artifact::WireCost>::new(
        "exact".into(), ironsmith_core::PlayerFilter::You, ironsmith_core::GrantPlayTaggedDuration::ForAsLongAsExiled,
        true, ManaSpendMode::AnyColor);
    let old = serde_json::to_value(&legacy).unwrap();
    assert!(old.get("permission_bound_mana").is_none());
    let decoded: ironsmith_core::GrantPlayTaggedEffect<ironsmith_compiled_artifact::WireCost> = serde_json::from_value(old).unwrap();
    assert!(!decoded.permission_bound_mana);
    let mut marked = legacy;
    marked.permission_bound_mana = true;
    let encoded = serde_json::to_value(&marked).unwrap();
    assert_eq!(encoded["permission_bound_mana"], true);
    let decoded: ironsmith_core::GrantPlayTaggedEffect<ironsmith_compiled_artifact::WireCost> = serde_json::from_value(encoded).unwrap();
    assert!(decoded.permission_bound_mana);
    assert_eq!(decoded.mana_spend_mode, ManaSpendMode::AnyColor);
}

#[test]
fn admitted_non_normal_grants_without_new_surface_keep_canonical_and_public_object_text() {
    for source in [
        "Type: Enchantment\nYou may play lands from your graveyard.",
        "Mana cost: {U}{B}\nType: Creature\nPower/Toughness: 2/2\nWhenever this creature deals combat damage to a player, exile the top card of that player's library.\nYou may play cards exiled with this creature.",
    ] {
        let (baseline, _) = compile_to_artifact("Legacy typed mana surface", source, false).unwrap();
        let parsed = ironsmith_compiler::CompilerFacade::new().compile_definition(
            ironsmith_compiler::CardDefinitionBuilder::new(ironsmith::CardId::new(), "Legacy typed mana surface"),
            source, ironsmith_compiler::CompilePolicy { allow_unsupported: false }).unwrap().definition;
        let old_runtime = into_runtime_definition(parsed.clone()).unwrap();
        let old_rendered = ironsmith_text::canonical_compiled_lines(&old_runtime);
        let mut game = ironsmith::GameState::new(vec!["A".into(), "B".into()], 20);
        let old_object = game.create_object_from_definition(&old_runtime, ironsmith::PlayerId::from_index(0), ironsmith::Zone::Battlefield);
        let old_oracle = game.object(old_object).unwrap().compiled_card_text.clone();
        for mode in [ManaSpendMode::AnyColor, ManaSpendMode::AnyType] {
            let mut compiled = parsed.clone(); let mut count = 0;
            for ability in &mut compiled.abilities {
                if let AbilityKind::Static(ability) = &mut ability.kind
                    && let StaticAbilityPayload::Grants(spec) = &mut ability.payload {
                    spec.cast_mana_spend_mode = mode; count += 1;
                    assert!(spec.source_exiled_surface.as_ref().is_none_or(|surface| surface.mana_rider.is_none()));
                }
            }
            assert_eq!(count, 1);
            let mut artifact = baseline.clone();
            artifact.payload.definition = ironsmith_compiled_artifact::wire_definition_from_serializable(&compiled).unwrap();
            artifact.refresh_checksum(); let bytes = artifact.to_json().unwrap();
            assert!(!std::str::from_utf8(&bytes).unwrap().contains("mana_rider"));
            let decoded = ironsmith_compiled_artifact::CompiledCardArtifact::from_json(&bytes).unwrap();
            decoded.validate().unwrap(); assert_eq!(decoded.to_json().unwrap(), bytes);
            for runtime in [into_runtime_definition(compiled).unwrap(),
                ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap()] {
                assert_eq!(ironsmith_text::canonical_compiled_lines(&runtime), old_rendered);
                let object = game.create_object_from_definition(&runtime, ironsmith::PlayerId::from_index(0), ironsmith::Zone::Battlefield);
                assert_eq!(game.object(object).unwrap().compiled_card_text, old_oracle,
                    "public audit oracle_text reads this exact stored carrier");
            }
        }
    }
}

#[test]
fn absent_class_scope_preserves_admitted_grants_and_legacy_class_activations_keep_their_surface() {
    let (baseline, _) = compile_to_artifact("Existing grant", "Type: Enchantment\nYou may play lands from your graveyard.", false).unwrap();
    let bytes = baseline.to_json().unwrap(); assert!(!std::str::from_utf8(&bytes).unwrap().contains("linked_exile_class_level"));
    let decoded = ironsmith_compiled_artifact::CompiledCardArtifact::from_json(&bytes).unwrap(); decoded.validate().unwrap(); assert_eq!(decoded.to_json().unwrap(), bytes);
    let source = "Mana cost: {R}\nType: Enchantment — Class\n{1}{R}: Level 2\nCreatures you control have menace.\n{2}{R}: Level 3\nCreatures you control have haste.";
    let (baseline, current) = compile_to_artifact("Existing Class shape", source, false).unwrap();
    let mut legacy = ironsmith_compiler::CompilerFacade::new().compile_definition(
        ironsmith_compiler::CardDefinitionBuilder::new(ironsmith::CardId::new(), "Existing Class shape"), source,
        ironsmith_compiler::CompilePolicy { allow_unsupported: false }).unwrap().definition;
    let mut count = 0;
    for ability in &mut legacy.abilities { if let AbilityKind::Activated(ability) = &mut ability.kind {
        assert!(matches!(ability.keyword, Some(ironsmith_core::ActivatedAbilityKeyword::ClassLevel(_)))); ability.keyword = None; count += 1;
    } }
    assert_eq!(count, 2); let mut artifact = baseline;
    artifact.payload.definition = ironsmith_compiled_artifact::wire_definition_from_serializable(&legacy).unwrap(); artifact.refresh_checksum();
    let bytes = artifact.to_json().unwrap(); assert!(!std::str::from_utf8(&bytes).unwrap().contains("ClassLevel\""));
    let restored = ironsmith_compiled_artifact::CompiledCardArtifact::from_json(&bytes).unwrap(); restored.validate().unwrap(); assert_eq!(restored.to_json().unwrap(), bytes);
    for definition in [into_runtime_definition(legacy).unwrap(), ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()] {
        assert_eq!(ironsmith_text::canonical_compiled_lines(&definition), ironsmith_text::canonical_compiled_lines(&current));
        let a = ironsmith::PlayerId::from_index(0); let mut game = ironsmith::GameState::new(vec!["A".into(), "B".into()], 20);
        let source = game.create_object_from_definition(&definition, a, ironsmith::Zone::Battlefield);
        let current_source = game.create_object_from_definition(&current, a, ironsmith::Zone::Battlefield);
        assert_eq!(game.object(source).unwrap().compiled_card_text, game.object(current_source).unwrap().compiled_card_text);
        let activated = definition.abilities.iter().find_map(|ability| match &ability.kind { ironsmith::ability::AbilityKind::Activated(ability) => Some(ability), _ => None }).unwrap();
        let effects = activated.effects.all_effects();
        let effect = effects[0].downcast_ref::<ironsmith::effects::SetClassLevelEffect>().unwrap();
        assert_eq!(effect.level, 2);
    }
    assert_eq!(serde_json::to_string(&ironsmith_core::ActivatedAbilityKeyword::Equip).unwrap(), "\"Equip\"");
    assert_eq!(serde_json::to_string(&ironsmith_core::ActivatedAbilityKeyword::PowerUp).unwrap(), "\"PowerUp\"");
}
