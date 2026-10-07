//! Independent complete frozen-body and instruction-codec contracts.
//! Authored only; no compiler, artifact, test, or engine execution was run.
use ironsmith::ability::AbilityKind;
use ironsmith::cards::CardDefinition;
use ironsmith::effect::Effect;
use ironsmith::effects::ChangeTextEffect;
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_core::{TextChangeSelection, Until};

#[path = "text_change_card_bodies/full_body_execution.rs"]
mod full_body_execution;

fn definitions(row: &serde_json::Value) -> [CardDefinition; 2] {
    let name = row["name"].as_str().unwrap();
    let mut text = format!("Mana cost: {}\nType: {}\n", row["mana_cost"].as_str().unwrap(), row["type_line"].as_str().unwrap());
    if let (Some(power), Some(toughness)) = (row["power"].as_str(), row["toughness"].as_str()) {
        text.push_str(&format!("Power/Toughness: {power}/{toughness}\n"));
    }
    text.push_str(row["oracle_text"].as_str().unwrap());
    let (direct, loss) = ironsmith_compiler::parse_loss::capture(||
        ironsmith_compiler_runtime::compile_to_runtime_definition(name, &text, false));
    assert!(!loss.is_lossy(), "{name}");
    let direct = direct.unwrap_or_else(|error| panic!("{name}: {error:?}"));
    let (compiled, loss) = ironsmith_compiler::parse_loss::capture(||
        ironsmith_compiler_runtime::compile_to_artifact(name, &text, false));
    assert!(!loss.is_lossy(), "{name}");
    let (artifact, _) = compiled.unwrap_or_else(|error| panic!("{name}: {error:?}"));
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    restored.validate().unwrap();
    let restored = ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap();
    [direct, restored]
}

fn collect(effect: &Effect, all: &mut Vec<Effect>) {
    all.push(effect.clone());
    effect.visit_child_effects(&mut |child| collect(child, all));
}
fn all_effects(definition: &CardDefinition) -> Vec<Effect> {
    let mut effects = Vec::new();
    if let Some(program) = &definition.spell_effect { for effect in program.all_effects() { collect(effect, &mut effects); } }
    for ability in &definition.abilities {
        let program = match &ability.kind {
            AbilityKind::Activated(ability) => &ability.effects,
            AbilityKind::Triggered(ability) => &ability.effects,
            AbilityKind::Static(_) => continue,
        };
        for effect in program.all_effects() { collect(effect, &mut effects); }
    }
    effects
}

#[test]
fn every_frozen_body_retains_its_typed_selector_duration_and_secondary_mechanics() {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!("../../../fixtures/typed_text_changes.json.fixture")).unwrap();
    assert_eq!(rows.len(), 12);
    for row in rows {
        let name = row["name"].as_str().unwrap();
        for definition in definitions(&row) {
            assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition), "{name}");
            let effects = all_effects(&definition);
            let changes: Vec<_> = effects.iter().filter_map(|effect| effect.downcast_ref::<ChangeTextEffect>()).collect();
            assert_eq!(changes.len(), if name == "Spectral Shift" { 2 } else { 1 }, "{name}");
            let expected_duration = if matches!(name, "Crystal Spray" | "Trait Doctoring" | "Whim of Volrath") {
                Until::EndOfTurn
            } else { Until::Forever };
            assert!(changes.iter().all(|change| change.duration == expected_duration), "{name}");
            match name {
                "Alter Reality" => assert!(definition.alternative_casts.iter().any(|method| matches!(method,
                    ironsmith::alternative_cast::AlternativeCastingMethod::Flashback { total_cost }
                        if total_cost.mana_cost().is_some_and(|cost| cost.mana_value() == 2)))),
                "Artificial Evolution" => assert_eq!(changes[0].selection,
                    TextChangeSelection::Creature { excluded_new: vec![ironsmith::Subtype::Wall] }),
                "Balduvian Shaman" => {
                    let AbilityKind::Activated(ability) = &definition.abilities[0].kind else { panic!("tap activation"); };
                    assert!(ability.mana_cost.costs().iter().any(|cost| matches!(cost.compiled_model(), Some(ironsmith_core::Cost::Tap))));
                    let ironsmith::target::ChooseSpec::Object(filter) = changes[0].target.base() else { panic!("enchantment target"); };
                    assert_eq!(filter.has_cumulative_upkeep, Some(false));
                    assert_eq!(filter.colors, Some(ironsmith::ColorSet::WHITE));
                    assert_eq!(filter.controller, Some(ironsmith::target::PlayerFilter::You));
                    assert!(effects.iter().any(|effect| effect.downcast_ref::<ironsmith::effects::CumulativeUpkeepEffect>()
                        .is_some_and(|upkeep| upkeep.kind == ironsmith_core::effect::UpkeepPaymentKind::Cumulative)));
                }
                "Crystal Spray" => assert!(effects.iter().any(|effect| effect.downcast_ref::<ironsmith::effects::DrawCardsEffect>()
                    .is_some_and(|draw| draw.count == ironsmith::effect::Value::Fixed(1)))),
                "Glamerdye" => assert!(definition.alternative_casts.iter().any(|method| matches!(method,
                    ironsmith::alternative_cast::AlternativeCastingMethod::Retrace { .. }))),
                "New Blood" => {
                    assert_eq!(changes[0].selection, TextChangeSelection::CreatureTo(ironsmith::Subtype::Vampire));
                    assert!(definition.additional_cost.has_non_mana_costs());
                    assert!(effects.iter().any(|effect| effect.downcast_ref::<ironsmith::effects::ApplyContinuousEffect>()
                        .is_some_and(|control| !control.runtime_modifications.is_empty())));
                }
                "Spectral Shift" => {
                    assert_eq!(changes[0].selection, TextChangeSelection::BasicLand);
                    assert_eq!(changes[1].selection, TextChangeSelection::Color);
                    assert!(definition.optional_costs.iter().any(|cost| cost.kind == ironsmith_core::OptionalCostKind::Entwine
                        && cost.cost.mana_cost().is_some_and(|mana| mana.mana_value() == 2)));
                }
                "Trait Doctoring" => assert!(effects.iter().any(|effect| effect.downcast_ref::<ironsmith::effects::CipherEffect>().is_some())),
                "Whim of Volrath" => assert!(definition.optional_costs.iter().any(|cost| cost.kind == ironsmith_core::OptionalCostKind::Buyback
                    && cost.returns_to_hand && cost.cost.mana_cost().is_some_and(|mana| mana.mana_value() == 2))),
                "Magical Hack" => assert_eq!(changes[0].selection, TextChangeSelection::BasicLand),
                "Mind Bend" => assert_eq!(changes[0].selection, TextChangeSelection::ColorOrBasicLand),
                "Sleight of Mind" => assert_eq!(changes[0].selection, TextChangeSelection::Color),
                _ => unreachable!(),
            }
        }
    }
}

#[test]
fn fresh_native_text_choice_encodes_and_materializes_the_current_program() {
    let native = Effect::new(ChangeTextEffect::new(ironsmith::target::ChooseSpec::Source,
        TextChangeSelection::CreatureTo(ironsmith::Subtype::Vampire), Until::Forever));
    let wire = ironsmith_runtime_catalog::artifact_materializer::encode_runtime_effect(native).unwrap();
    let restored = ironsmith_runtime_catalog::artifact_materializer::materialize_effect(wire).unwrap();
    let changed = restored.with_text_change(ironsmith_core::TextChange::creature_type(
        ironsmith::Subtype::Vampire, ironsmith::Subtype::Zombie).unwrap()).unwrap();
    assert_eq!(changed.downcast_ref::<ChangeTextEffect>().unwrap().selection,
        TextChangeSelection::CreatureTo(ironsmith::Subtype::Zombie));
    let wire = ironsmith_runtime_catalog::artifact_materializer::encode_runtime_effect(changed).unwrap();
    let restored = ironsmith_runtime_catalog::artifact_materializer::materialize_effect(wire).unwrap();
    assert_eq!(restored.downcast_ref::<ChangeTextEffect>().unwrap().selection,
        TextChangeSelection::CreatureTo(ironsmith::Subtype::Zombie));
}
