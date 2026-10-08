//! "for each card you've discarded this turn" reads the turn's discard
//! history (CR 701.9a), not a current zone. Source-authored, deliberately unrun.
use ironsmith::cards::CardDefinition;
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::effects::{DiscardEffect, EffectContext, execute_effect};
use ironsmith::{GameState, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;

const A: PlayerId = PlayerId(0);

fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/discarded_this_turn_counts.json.fixture"
    ))
    .unwrap()
}

fn definitions(name: &str, text: &str) -> [CardDefinition; 2] {
    let (direct, loss) = ironsmith_compiler::parse_loss::capture(|| {
        ironsmith_compiler_runtime::compile_to_runtime_definition(name, text, false)
    });
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let direct = direct.unwrap_or_else(|error| panic!("{name}: {error}"));
    let (compiled, loss) = ironsmith_compiler::parse_loss::capture(|| {
        ironsmith_compiler_runtime::compile_to_artifact(name, text, false)
    });
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let (artifact, _) = compiled.unwrap_or_else(|error| panic!("{name}: {error}"));
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    restored.validate().unwrap();
    assert_eq!(artifact, restored);
    let decoded =
        ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap();
    [direct, decoded]
}

#[test]
fn every_discarded_this_turn_card_counts_the_turn_history_on_both_routes() {
    let rows = fixtures();
    assert_eq!(rows.len(), 6);
    for row in &rows {
        let name = row["name"].as_str().unwrap();
        for definition in definitions(name, row["text"].as_str().unwrap()) {
            assert!(
                !ironsmith::cards::generated_definition_has_unimplemented_content(&definition),
                "{name}"
            );
            let debug = format!("{definition:?}");
            assert!(
                debug.contains("CardsDiscardedThisTurn(You)"),
                "{name}: the count must be the controller's discard history"
            );
            assert!(
                !debug.contains("DiscardedOrCycled"),
                "{name}: cycling is not discarding-only history"
            );
            assert!(
                !debug.contains("MaxCardsDrawnThisTurn"),
                "{name}: do not read the draw history"
            );
        }
    }
}

#[test]
fn change_of_fortune_draws_for_earlier_discards_and_the_discarded_hand() {
    let row = fixtures()
        .into_iter()
        .find(|row| row["name"] == "Change of Fortune")
        .unwrap();
    let filler =
        ironsmith_compiler_runtime::compile_to_runtime_definition("Filler", "Type: Sorcery", false)
            .unwrap();
    for definition in definitions("Change of Fortune", row["text"].as_str().unwrap()) {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&definition, A, Zone::Exile);
        for _ in 0..5 {
            game.create_object_from_definition(&filler, A, Zone::Hand);
        }
        for _ in 0..12 {
            game.create_object_from_definition(&filler, A, Zone::Library);
        }
        let mut dm = SelectFirstDecisionMaker;
        // Two earlier discards this turn.
        execute_effect(
            &mut game,
            &ironsmith::effect::Effect::new(DiscardEffect::you(2)),
            &mut EffectContext::new(source, A, &mut dm),
        )
        .unwrap();
        assert_eq!(game.player(A).unwrap().hand.len(), 3);
        let program = definition.spell_effect.as_ref().unwrap();
        for effect in program.flattened_default_effects() {
            execute_effect(&mut game, effect, &mut EffectContext::new(source, A, &mut dm)).unwrap();
        }
        // Discarded 2 + 3 this turn, so five draws.
        assert_eq!(game.player(A).unwrap().hand.len(), 5);
        assert_eq!(game.player(A).unwrap().graveyard.len(), 5);
    }
}
