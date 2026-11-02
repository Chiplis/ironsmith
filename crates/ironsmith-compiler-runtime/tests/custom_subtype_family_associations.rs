//! One explicitly declared subtype identity can belong to several families.
use ironsmith::{CardDefinition, GameState, PlayerId, Zone};
use ironsmith_core::{Subtype, SubtypeFamily};
use ironsmith_core::custom_subtypes::{CustomSubtype, CustomSubtypeDefinition};
#[path = "custom_final_next_wave_support.rs"]
mod source_support;
const A: PlayerId = PlayerId(0);
fn vocabulary() -> [CustomSubtypeDefinition; 2] {
    [SubtypeFamily::Spell, SubtypeFamily::Artifact].map(|family| CustomSubtypeDefinition {
        singular: "Inscription".into(), plural: "Inscriptions".into(), family,
    })
}
fn identity() -> Subtype {
    Subtype::Custom(CustomSubtype::new_with_families("Inscription", &[SubtypeFamily::Spell, SubtypeFamily::Artifact]).unwrap())
}
fn definitions(name: &str, text: &str) -> [CardDefinition; 2] {
    let (result, loss) = ironsmith_compiler::parse_loss::capture(||
        ironsmith_compiler::with_custom_subtypes(&vocabulary(), ||
            ironsmith_compiler::word_claims::with_enforcement_options(true, true, ||
                ironsmith_compiler_runtime::compile_to_artifact(name, text, false))).unwrap());
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let (artifact, direct) = result.unwrap();
    let restored = ironsmith_compiled_artifact::CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    [direct, ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()]
}
#[test]
fn spell_and_artifact_source_forms_share_identity_through_artifacts_filters_and_type_universe() {
    let spells = definitions("Spell inscription", "Type: Sorcery — Inscription\nDraw a card.");
    let artifacts = definitions("Artifact inscription", "Type: Artifact — Equipment Inscriptions\nEquip {3}");
    assert!(identity().belongs_to_family(SubtypeFamily::Spell));
    assert!(identity().belongs_to_family(SubtypeFamily::Artifact));
    assert!(!identity().belongs_to_family(SubtypeFamily::Enchantment));
    for index in 0..2 {
        assert!(spells[index].card.subtypes.contains(&identity()));
        assert!(artifacts[index].card.subtypes.contains(&identity()));
        for definition in [&spells[index], &artifacts[index]] {
            let subtype = definition.card.subtypes.iter().find(|subtype| **subtype == identity()).unwrap();
            assert!(subtype.belongs_to_family(SubtypeFamily::Spell));
            assert!(subtype.belongs_to_family(SubtypeFamily::Artifact), "artifact transport must retain associations, not just spelling");
        }
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let spell = game.create_object_from_definition(&spells[index], A, Zone::Exile);
        let artifact = game.create_object_from_definition(&artifacts[index], A, Zone::Battlefield);
        for id in [spell, artifact] { assert!(game.calculated_characteristics(id).unwrap().subtypes.contains(&identity())); }
        let filter = ironsmith::target::ObjectFilter::default().with_subtype(identity());
        for id in [spell, artifact] { assert!(object_matches(&game,&filter,id)); }
        for family in [SubtypeFamily::Spell, SubtypeFamily::Artifact] {
            assert_eq!(game.subtypes_in_family(family).iter().filter(|subtype| **subtype == identity()).count(), 1);
        }
    }
    // Legacy printed vocabulary remains supported outside the source
    // scope, but must not retain the scoped multi-family custom identity.
    assert!(ironsmith_core::custom_subtypes::lookup("Inscription").is_none());
    let (_, outside) = ironsmith_compiler_runtime::compile_to_artifact(
        "Outside scope", "Type: Artifact — Inscription", false,
    ).unwrap();
    assert!(!outside.card.subtypes.contains(&identity()));
    assert!(outside.card.subtypes.contains(&Subtype::ArtifactInscription));
}
#[test]
fn continuous_subtype_admission_accepts_any_declared_family_and_rejects_unrelated_card_types() {
    for (card_type, expected) in [("Artifact", true), ("Sorcery", true), ("Enchantment", false), ("Artifact Enchantment", true)] {
        for definition in definitions("Family admission", &format!("Type: {card_type}\nThis card is an Inscription in addition to its other types.")) {
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let source = game.create_object_from_definition(&definition, A, if card_type == "Sorcery" { Zone::Exile } else { Zone::Battlefield });
            assert_eq!(game.calculated_characteristics(source).unwrap().subtypes.contains(&identity()), expected, "{card_type}");
        }
    }
}
#[test]
fn exact_steelscribed_katana_source_preserves_its_artifact_inscription_and_real_abilities() {
    let cohort: serde_json::Value = serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-inscribe-all-sources-candidate.json")).unwrap();
    let row = cohort["mechanics"].as_array().unwrap().iter().find(|row| row["id"] == "msem-442").unwrap();
    let card = row["representative_cards"].as_array().unwrap().iter()
        .find(|card| card["source_card_key"] == "Steelscribed Katana_OTH").unwrap();
    assert_eq!(card["rewrite_status"], "frontend_materialized_unverified",
        "Parent must materialize the complete collection candidate before native verification");
    assert!(card["original_text"].as_str().unwrap().contains("Type: Artifact — Equipment Inscription"));
    for definition in source_support::definitions(card).unwrap() {
        assert!(definition.card.subtypes.contains(&identity()));
        assert!(definition.card.subtypes.contains(&Subtype::Equipment));
        assert!(definition.abilities.len() >= 3, "actual attach/equip/inscribe abilities must remain");
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let katana = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        assert!(game.calculated_characteristics(katana).unwrap().subtypes.contains(&identity()));
    }
}
#[test]
fn association_order_aliases_and_legacy_single_family_payload_are_stable() {
    let a = CustomSubtype::new_with_families("Écho-mark", &[SubtypeFamily::Spell, SubtypeFamily::Artifact]).unwrap();
    let b = CustomSubtype::new_with_families("Écho-mark", &[SubtypeFamily::Artifact, SubtypeFamily::Spell, SubtypeFamily::Artifact]).unwrap();
    assert_eq!(a, b);
    assert_eq!(a.families().collect::<Vec<_>>(), b.families().collect::<Vec<_>>());
    let unicode = [SubtypeFamily::Spell, SubtypeFamily::Artifact].map(|family| CustomSubtypeDefinition {
        singular: "Écho-mark".into(), plural: "Écho-marks".into(), family,
    });
    ironsmith_compiler::with_custom_subtypes(&unicode, || {
        for alias in ["Écho-mark", "Échomarks", "Écho marks"] {
            let subtype = ironsmith_core::custom_subtypes::lookup(alias).unwrap();
            assert_eq!(subtype, Subtype::Custom(a));
            assert!(subtype.belongs_to_family(SubtypeFamily::Spell));
            assert!(subtype.belongs_to_family(SubtypeFamily::Artifact));
        }
    }).unwrap();
    let legacy = CustomSubtype::new("Archive", SubtypeFamily::Artifact).unwrap();
    let json = serde_json::to_value(legacy).unwrap();
    assert!(json.get("additional_families").is_none());
    assert_eq!(serde_json::from_value::<CustomSubtype>(json).unwrap(), legacy);
    let mut reversed = vocabulary(); reversed.reverse();
    for vocabulary in [vocabulary().to_vec(), reversed.to_vec()] {
        let subtype = ironsmith_compiler::with_custom_subtypes(&vocabulary, || ironsmith_core::custom_subtypes::lookup("Inscriptions").unwrap()).unwrap();
        assert_eq!(subtype, identity());
    }
    let conflicts = [vocabulary()[0].clone(), CustomSubtypeDefinition { plural: "Differentplural".into(), ..vocabulary()[1].clone() }];
    assert!(ironsmith_compiler::with_custom_subtypes(&conflicts, || ()).is_err());
}
#[test]
fn legacy_single_family_cards_match_shared_identity_and_merge_game_local_associations() {
    let mut compiled = Vec::new();
    for (family, text) in [(SubtypeFamily::Spell, "Type: Sorcery — Inscription\nDraw a card."), (SubtypeFamily::Artifact, "Type: Artifact — Inscription")] {
        let vocabulary = [CustomSubtypeDefinition { singular: "Inscription".into(), plural: "Inscriptions".into(), family }];
        let (artifact, direct) = ironsmith_compiler::with_custom_subtypes(&vocabulary, ||
            ironsmith_compiler_runtime::compile_to_artifact("Legacy declaration", text, false)).unwrap().unwrap();
        let restored = ironsmith_compiled_artifact::CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
        compiled.push([direct, ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()]);
    }
    for route in 0..2 {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        for definitions in &compiled {
            let id = game.create_object_from_definition(&definitions[route], A, Zone::Exile);
            let filter = ironsmith::target::ObjectFilter::default().with_subtype(identity());
                assert!(object_matches(&game,&filter,id));
        }
        for family in [SubtypeFamily::Spell, SubtypeFamily::Artifact] {
            let types = game.subtypes_in_family(family);
            assert_eq!(types.iter().filter(|subtype| **subtype == identity()).count(), 1);
        }
        assert!(!GameState::new(vec!["A".into(), "B".into()], 20).subtypes_in_family(SubtypeFamily::Spell).contains(&identity()));
    }
}

// Ruin is explicitly both Artifact and Land in Revolution205.3h/205.3m.
// Partial old artifact metadata must not become a competing runtime identity.
fn ruin_definitions(families:&[SubtypeFamily],text:&str)->[CardDefinition;2]{
 let vocabulary:Vec<_>=families.iter().map(|family|CustomSubtypeDefinition{singular:"Ruin".into(),plural:"Ruins".into(),family:*family}).collect();
 let (artifact,direct)=ironsmith_compiler::with_custom_subtypes(&vocabulary,||ironsmith_compiler_runtime::compile_to_artifact("Ruin association control",text,false)).unwrap().unwrap();
 let restored=ironsmith_compiled_artifact::CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
 [direct,ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()]
}
fn alter(game:&mut GameState,id:ironsmith::ObjectId,modification:ironsmith::continuous::Modification){
 let effect=ironsmith::Effect::new(ironsmith::effects::ApplyContinuousEffect::new(ironsmith::continuous::EffectTarget::Specific(id),modification,ironsmith::effect::Until::EndOfTurn));
 ironsmith::effects::execute_effect(game,&effect,&mut ironsmith::effects::EffectContext::new_default(id,A)).unwrap();
}
#[test]
fn two_legacy_declarations_update_derived_family_metadata_before_all_types_and_type_loss(){
 use ironsmith::continuous::Modification as M;use ironsmith::CardType;
 let artifacts=ruin_definitions(&[SubtypeFamily::Artifact],"Type: Artifact Land — Ruin");
 let lands=ruin_definitions(&[SubtypeFamily::Land],"Type: Land — Ruin");
 let ruin=Subtype::Custom(CustomSubtype::new("Ruin",SubtypeFamily::Artifact).unwrap());
 for route in 0..2{
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);let id=game.create_object_from_definition(&artifacts[route],A,Zone::Battlefield);
  assert!(!game.current_characteristics(id).unwrap().subtypes.iter().find(|ty|**ty==ruin).unwrap().belongs_to_family(SubtypeFamily::Land));
  game.create_object_from_definition(&lands[route],A,Zone::Exile);
  let chars=game.current_characteristics(id).unwrap();let subtype=chars.subtypes.iter().find(|ty|**ty==ruin).unwrap();assert!(subtype.belongs_to_family(SubtypeFamily::Artifact));assert!(subtype.belongs_to_family(SubtypeFamily::Land),"new admitted association invalidates cached old object metadata");
  alter(&mut game,id,M::AddAllSubtypesOfFamily(SubtypeFamily::Land));
  alter(&mut game,id,M::RemoveCardTypes(vec![CardType::Artifact]));
  assert!(game.current_characteristics(id).unwrap().subtypes.contains(&ruin),"all-land-type membership survives artifact loss");
  alter(&mut game,id,M::RemoveCardTypes(vec![CardType::Land]));assert!(!game.current_characteristics(id).unwrap().subtypes.contains(&ruin));
 }
}
#[test]
fn old_operands_use_game_admitted_associations_for_addition_and_family_replacement(){
 use ironsmith::continuous::Modification as M;
 let artifacts=ruin_definitions(&[SubtypeFamily::Artifact],"Type: Artifact — Ruin");let lands=ruin_definitions(&[SubtypeFamily::Land],"Type: Land — Ruin");let targets=ruin_definitions(&[SubtypeFamily::Land],"Type: Land — Forest");
 let old_operand=Subtype::Custom(CustomSubtype::new("Ruin",SubtypeFamily::Artifact).unwrap());
 for route in 0..2{
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);game.create_object_from_definition(&artifacts[route],A,Zone::Exile);game.create_object_from_definition(&lands[route],A,Zone::Exile);
  let target=game.create_object_from_definition(&targets[route],A,Zone::Battlefield);alter(&mut game,target,M::AddSubtypes(vec![old_operand]));
  let chars=game.current_characteristics(target).unwrap();assert!(chars.subtypes.contains(&old_operand));assert!(chars.subtypes.contains(&Subtype::Forest));
  alter(&mut game,target,M::SetSubtypes(vec![old_operand]));let chars=game.current_characteristics(target).unwrap();assert!(chars.subtypes.contains(&old_operand));assert!(!chars.subtypes.contains(&Subtype::Forest),"old scalar operand replaces its complete admitted land family");
 }
}
#[test]
fn program_equivalence_distinguishes_family_metadata_even_when_subtype_identity_matches(){
 let narrow=ruin_definitions(&[SubtypeFamily::Artifact],"Type: Artifact\nThis card is a Ruin in addition to its other types.");
 let wide=ruin_definitions(&[SubtypeFamily::Artifact,SubtypeFamily::Land],"Type: Artifact\nThis card is a Ruin in addition to its other types.");
 for route in 0..2{assert!(!ironsmith::ability_program_identity::ability_programs_equivalent(&narrow[route].abilities,&wide[route].abilities),"family-sensitive executable metadata cannot be discarded by name-only Eq");assert!(ironsmith::ability_program_identity::ability_programs_equivalent(&wide[route].abilities,&wide[route].abilities));}
}
#[test]
fn supplied_counter_only_subtype_dependencies_keep_every_explicit_association(){
 let vocabulary=vocabulary();let (artifact,direct)=ironsmith_compiler::with_custom_subtypes(&vocabulary,||ironsmith_compiler_runtime::compile_to_artifact_with_counter_definitions("Counter metadata carrier","Type: Artifact",&[("inscribed".into(),"This creature is an Inscription in addition to its other types.".into())],false)).unwrap().unwrap();
 let restored=ironsmith_compiled_artifact::CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();let materialized=ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap();
 for definition in [direct,materialized]{let subtype=definition.card.custom_subtypes.iter().find(|ty|**ty==identity()).unwrap();assert!(subtype.belongs_to_family(SubtypeFamily::Artifact));assert!(subtype.belongs_to_family(SubtypeFamily::Spell));assert!(definition.ability_counter_definitions.contains_key("inscribed"));}
}

fn object_matches(game:&GameState,filter:&ironsmith::ObjectFilter,id:ironsmith::ObjectId)->bool{
    let mut filter=filter.clone();filter.zone=Some(game.object(id).unwrap().zone);
    let ctx=ironsmith::effects::EffectContext::new_default(id,A);
    ironsmith::effects::helpers::resolve_objects_from_spec(game,&ironsmith::target::ChooseSpec::All(filter),&ctx).unwrap().contains(&id)
}
