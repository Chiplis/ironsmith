//! cf8/p06 granted optional self replacement: "If this permanent would be
//! put into a graveyard, you may put it on top of its owner's library
//! instead." (CR 614.1a, 616.1). Full frozen bodies on the direct and
//! artifact routes. Source-authored, deliberately unrun.
use ironsmith::cards::CardDefinition;
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;

fn rows() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!("../../../fixtures/p06_round4.json.fixture")).unwrap()
}

fn text(row: &serde_json::Value) -> String {
    let mut lines = vec![
        format!("Mana cost: {}", row["mana_cost"].as_str().unwrap()),
        format!("Type: {}", row["type_line"].as_str().unwrap()),
    ];
    if let (Some(power), Some(toughness)) = (row["power"].as_str(), row["toughness"].as_str()) {
        lines.push(format!("Power/Toughness: {power}/{toughness}"));
    }
    if let Some(loyalty) = row["loyalty"].as_str() {
        lines.push(format!("Loyalty: {loyalty}"));
    }
    lines.push(row["oracle_text"].as_str().unwrap().to_string());
    lines.join("\n")
}

fn definitions(name: &str) -> [CardDefinition; 2] {
    let row = rows().into_iter().find(|row| row["name"] == name).unwrap();
    let text = text(&row);
    let (direct, loss) = ironsmith_compiler::parse_loss::capture(|| {
        compile_to_runtime_definition(name, &text, false)
    });
    let direct = direct.unwrap_or_else(|error| panic!("direct {name}: {error}"));
    assert!(!loss.is_lossy(), "direct {name}: {}", loss.reasons_text());
    let (compiled, loss) =
        ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, &text, false));
    let (artifact, _) = compiled.unwrap_or_else(|error| panic!("artifact {name}: {error}"));
    assert!(!loss.is_lossy(), "artifact {name}: {}", loss.reasons_text());
    artifact.validate().unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    let decoded = materialize_artifact(&restored).unwrap();
    assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&direct));
    [direct, decoded]
}


#[test]
fn slivers_gain_an_optional_library_top_replacement() {
    for definition in definitions("Pulmonic Sliver") {
        let debug = format!("{definition:?}");
        assert!(debug.contains("EventReplacementWithEffects"), "{debug}");
        assert!(debug.contains("ZoneChange"), "{debug}");
        assert!(debug.contains("optional: true"), "{debug}");
        assert!(debug.contains("Sliver"), "{debug}");
    }
}

#[test]
fn entries_under_an_opponents_control_this_turn_enter_under_yours() {
    for name in ["Gather Specimens", "Crafty Cutpurse"] {
        for definition in definitions(name) {
            let debug = format!("{definition:?}");
            assert!(debug.contains("RegisterEnterUnderControlReplacement"), "{name}: {debug}");
            assert!(debug.contains("UntilEndOfTurn"), "{name}: {debug}");
            assert!(debug.contains("Opponent"), "{name}: {debug}");
        }
    }
}

#[test]
fn would_be_destroyed_regenerate_it_is_the_regeneration_replacement() {
    use ironsmith::decision::SelectFirstDecisionMaker;
    use ironsmith::effect::Effect;
    use ironsmith::effects::{EffectContext as ExecutionContext, execute_effect};
    use ironsmith::{GameState, PlayerId, Zone};
    for definition in definitions("Clergy of the Holy Nimbus") {
        let debug = format!("{definition:?}");
        assert!(debug.contains("SourceDestructionRegenerates"), "{debug}");
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let clergy =
            game.create_object_from_definition(&definition, PlayerId::from_index(0), Zone::Battlefield);
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(clergy, PlayerId::from_index(1), &mut dm);
        execute_effect(
            &mut game,
            &Effect::destroy(ironsmith::target::ChooseSpec::SpecificObject(clergy)),
            &mut ctx,
        )
        .unwrap();
        // CR 701.19a: regenerated, not destroyed; it stays and is tapped.
        assert_eq!(game.object(clergy).map(|object| object.zone), Some(Zone::Battlefield));
    }
}

#[test]
fn untap_step_untaps_are_replaced_by_instead_programs() {
    for (name, fragment) in [("Freyalise's Winds", "Wind"), ("Bewitching Leechcraft", "PlusOnePlusOne")] {
        for definition in definitions(name) {
            let debug = format!("{definition:?}");
            assert!(debug.contains("EventReplacementWithEffects"), "{name}: {debug}");
            assert!(debug.contains("Untap"), "{name}: {debug}");
            assert!(debug.contains("during_controllers_untap_step: true"), "{name}: {debug}");
            assert!(debug.contains(fragment), "{name}: {debug}");
        }
    }
}
