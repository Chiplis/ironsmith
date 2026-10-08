//! Source-authored and deliberately unrun (cf8 p04): "Count the number of
//! <things>. <...> that number <...>" (Rumbling Ruin). The count is taken as
//! the ability resolves and the can't-block restriction stores it as a fixed
//! number (CR 608.2h): counters added later do not widen the restriction,
//! while the blockers' own power stays live.
use ironsmith::ability::AbilityKind;
use ironsmith::cards::CardDefinition;
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::{GameState, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;

const A: PlayerId = PlayerId::from_index(0);

fn definitions(index: usize) -> [CardDefinition; 2] {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/counted_number_references.json.fixture"
    ))
    .unwrap();
    let row = &rows[index];
    let name = row["name"].as_str().unwrap();
    let text = row["text"].as_str().unwrap();
    let (direct, loss) = ironsmith_compiler::parse_loss::capture(|| {
        compile_to_runtime_definition(name, text, false)
    });
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (compiled, loss) =
        ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, text, false));
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (artifact, _) = compiled.unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    [direct.unwrap(), materialize_artifact(&restored).unwrap()]
}

#[test]
fn rumbling_ruin_freezes_the_counted_number_in_its_restriction() {
    for definition in definitions(0) {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
        let debug = format!("{definition:?}");
        assert!(debug.contains("CantEffect"), "{debug}");
        assert!(debug.contains("LessThanOrEqualExpr"), "{debug}");
        assert!(debug.contains("PlusOnePlusOne"), "{debug}");
        let trigger = definition
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Triggered(triggered) => Some(triggered.effects.clone()),
                _ => None,
            })
            .unwrap();
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = EffectContext::new(source, A, &mut dm);
        for effect in trigger.flattened_default_effects() {
            execute_effect(&mut game, effect, &mut ctx).unwrap();
        }
        // The stored restriction compares power with a fixed number.
        let stored = format!("{:?}", game.effect_store.cant_effects);
        assert!(!stored.contains("LessThanOrEqualExpr"), "{stored}");
    }
}
