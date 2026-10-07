//! Definition-origin prerequisite, not a full-card coverage claim.
//! All scenarios are authored and unrun under the campaign execution gate.
use ironsmith::ability::Ability;
use ironsmith::cards::CardDefinition;
use ironsmith::continuous::{AbilityOrigin, ContinuousEffect, EffectTarget, Modification};
use ironsmith::{GameState, PlayerId, Subtype, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_core::TextChange;

const A: PlayerId = PlayerId::from_index(0);

fn definitions(text: &str) -> [CardDefinition; 2] {
    let (direct, direct_loss) = ironsmith_compiler::parse_loss::capture(||
        ironsmith_compiler_runtime::compile_to_runtime_definition("Provenance witness", text, false));
    let direct = direct.unwrap();
    assert!(!direct_loss.is_lossy());
    let (artifact, artifact_loss) = ironsmith_compiler::parse_loss::capture(||
        ironsmith_compiler_runtime::compile_to_artifact("Provenance witness", text, false));
    let (artifact, _) = artifact.unwrap();
    assert!(!artifact_loss.is_lossy());
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    let materialized = ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap();
    assert_eq!(direct.canonical_text, materialized.canonical_text);
    assert_eq!(direct.ability_labels, materialized.ability_labels);
    [direct, materialized]
}

#[test]
fn basic_and_nonbasic_reminders_have_no_printed_mana_and_rederive_after_text_change() {
    for (text, unchanged_type) in [
        ("Type: Basic Land — Forest\n({T}: Add {G}.)", None),
        ("Type: Land — Forest\n({T}: Add {G}.)", None),
        ("Type: Land — Forest Plains\n({T}: Add {G} or {W}.)", Some(Subtype::Plains)),
        ("({T}: Add {G}.)\nType: Land — Forest", None),
    ] {
        for definition in definitions(text) {
            assert!(definition.abilities.is_empty());
            assert!(definition.canonical_text.is_empty(), "rule text is not authored text");
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let id = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            game.refresh_continuous_state().unwrap();
            let before = game.calculated_characteristics(id).unwrap();
            assert!(before.abilities.contains(&Ability::basic_land_mana(Subtype::Forest).unwrap()));
            game.effect_store.continuous_effects.add_effect(ContinuousEffect::from_resolution(id, A, vec![id],
                Modification::RewriteText(TextChange::basic_land_type(Subtype::Forest, Subtype::Island).unwrap())));
            game.refresh_continuous_state().unwrap();
            let after = game.calculated_characteristics(id).unwrap();
            assert!(after.subtypes.contains(&Subtype::Island));
            assert!(!after.subtypes.contains(&Subtype::Forest));
            assert!(!after.abilities.contains(&Ability::basic_land_mana(Subtype::Forest).unwrap()));
            assert!(after.abilities.contains(&Ability::basic_land_mana(Subtype::Island).unwrap()));
            assert_eq!(after.abilities.len(), if unchanged_type.is_some() { 2 } else { 1 });
            if let Some(subtype) = unchanged_type {
                assert!(after.abilities.contains(&Ability::basic_land_mana(subtype).unwrap()));
            }
            for (index, ability) in after.abilities.iter().enumerate() {
                assert!(matches!(after.abilities.origin(index), Some(AbilityOrigin::IntrinsicBasicLandMana(_))));
                assert_eq!(game.current_ability(id, index), Some(ability.clone()));
            }
        }
    }
}

#[test]
fn ordinary_printed_mana_keeps_its_definition_even_when_equal_to_the_rule() {
    for (text, current_count) in [
        ("Type: Land — Forest\n{T}: Add {G}.", 2),
        ("Type: Land\n{T}: Add {G}.", 1),
        ("Type: Artifact\n{T}: Add {G}.", 1),
        ("Type: Land — Forest\n({T}: Add {G}.)\n{T}: Add {G}.", 2),
    ] {
        for definition in definitions(text) {
            assert_eq!(definition.abilities.len(), 1);
            assert!(definition.abilities[0].is_mana_ability());
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let id = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            game.refresh_continuous_state().unwrap();
            let chars = game.calculated_characteristics(id).unwrap();
            assert_eq!(chars.abilities.len(), current_count);
            assert_eq!(chars.abilities[0], definition.abilities[0]);
            assert_eq!(chars.abilities.origin(0), Some(&AbilityOrigin::Printed(0)));
            let copied = ironsmith::snapshot::CopiableValues::from_object(game.object(id).unwrap());
            assert_eq!(copied.abilities.as_slice(), definition.abilities.as_slice());
        }
    }
}

#[test]
fn independent_green_grant_survives_forest_to_island_with_its_exact_origin() {
    for definition in definitions("Type: Land — Forest\n({T}: Add {G}.)") {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let id = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let green = Ability::basic_land_mana(Subtype::Forest).unwrap();
        game.effect_store.continuous_effects.add_effect(ContinuousEffect::new(id, A,
            EffectTarget::Specific(id), Modification::AddAbilityGeneric(green.clone())));
        game.refresh_continuous_state().unwrap();
        let before = game.calculated_characteristics(id).unwrap();
        let origin = before.abilities.origin(1).unwrap().clone();
        game.effect_store.continuous_effects.add_effect(ContinuousEffect::from_resolution(id, A, vec![id],
            Modification::RewriteText(TextChange::basic_land_type(Subtype::Forest, Subtype::Island).unwrap())));
        game.refresh_continuous_state().unwrap();
        let after = game.calculated_characteristics(id).unwrap();
        assert_eq!(after.abilities.as_slice(), &[Ability::basic_land_mana(Subtype::Island).unwrap(), green]);
        assert_eq!(after.abilities.origin(1), Some(&origin));
    }
}

#[test]
fn contradictory_reminder_metadata_is_explicitly_unsupported_in_both_routes() {
    for text in ["Type: Land — Island\n({T}: Add {G}.)", "Type: Artifact\n({T}: Add {G}.)"] {
        assert!(ironsmith_compiler_runtime::compile_to_runtime_definition("Mismatch", text, false).is_err());
        assert!(ironsmith_compiler_runtime::compile_to_artifact("Mismatch", text, false).is_err());
    }
    // A rejection is migration evidence only; it is never card functionality.
}

#[test]
fn absent_reminder_is_the_canonical_default_and_explicit_mana_changes_the_definition() {
    let bare = definitions("Type: Basic Land — Forest");
    let reminder = definitions("Type: Basic Land — Forest\n({T}: Add {G}.)");
    let authored = definitions("Type: Basic Land — Forest\n{T}: Add {G}.");
    for index in 0..2 {
        assert_eq!(bare[index].abilities, reminder[index].abilities);
        assert_eq!(bare[index].canonical_text, reminder[index].canonical_text);
        assert_eq!(bare[index].ability_labels, reminder[index].ability_labels);
        assert_ne!(bare[index].abilities, authored[index].abilities);
        assert_ne!(bare[index].canonical_text, authored[index].canonical_text);
    }
}

#[test]
fn old_format_is_rejected_by_direct_materialization_and_registry_admission() {
    use ironsmith_compiled_artifact::ArtifactValidationError;
    use ironsmith_runtime_catalog::CardRegistryArtifactExt;
    use ironsmith_runtime_catalog::artifact_materializer::{ArtifactMaterializationError, materialize_artifact};
    let (mut artifact, _) = ironsmith_compiler_runtime::compile_to_artifact(
        "Old provenance", "Type: Land — Forest\n{T}: Add {G}.", false).unwrap();
    artifact.format_version = 6;
    artifact.refresh_checksum();
    assert!(matches!(materialize_artifact(&artifact), Err(ArtifactMaterializationError::InvalidArtifact(
        ArtifactValidationError::UnsupportedFormat { found: 6, .. }))));
    let mut registry = ironsmith::cards::CardRegistry::new();
    assert!(registry.register_compiled_artifact(&artifact).is_err());
    assert!(registry.get("Old provenance").is_none());
    // A decode/admission rejection is explicit unknown old provenance,
    // not a claim that this legacy card can execute text changes correctly.
}

#[test]
fn authored_green_mana_survives_forest_to_island_with_printed_index_in_both_routes() {
    use ironsmith::ability::{AbilityKind, ActivatedAbilityRuntimeExt as _};
    use ironsmith::mana::ManaSymbol;
    for definition in definitions("Type: Land — Forest\n{T}: Add {G}.") {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let id = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        game.refresh_continuous_state().unwrap();
        let before = game.calculated_characteristics(id).unwrap();
        assert_eq!(before.abilities.len(), 2, "equal authored and intrinsic green abilities are distinct");
        let old_origin = before.abilities.origin(0).unwrap().clone();
        let AbilityKind::Activated(printed) = &before.abilities[0].kind else { panic!("printed mana"); };
        let captured = printed.effects.clone();
        game.effect_store.continuous_effects.add_effect(ContinuousEffect::from_resolution(id, A, vec![id],
            Modification::RewriteText(TextChange::basic_land_type(Subtype::Forest, Subtype::Island).unwrap())));
        game.refresh_continuous_state().unwrap();
        let after = game.calculated_characteristics(id).unwrap();
        assert_eq!(after.abilities.origin(0), Some(&old_origin));
        assert_eq!(old_origin, AbilityOrigin::Printed(0));
        assert_eq!(after.abilities.origin(1), Some(&AbilityOrigin::IntrinsicBasicLandMana(Subtype::Island)));
        let AbilityKind::Activated(printed) = &after.abilities[0].kind else { panic!("printed mana"); };
        assert_eq!(printed.effects, captured, "printed mana symbols and immutable executors stay unchanged");
        assert_eq!(printed.inferred_mana_symbols(&game, id, A), vec![ManaSymbol::Green]);
        let AbilityKind::Activated(intrinsic) = &after.abilities[1].kind else { panic!("intrinsic mana"); };
        assert_eq!(intrinsic.mana_output, Some(vec![ManaSymbol::Blue]));
    }
}
