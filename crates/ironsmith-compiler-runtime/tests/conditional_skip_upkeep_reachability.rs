//! "Hellbent — Skip your upkeep step if you have no cards in hand." The
//! conditional skip-upkeep grammar existed but was reachable only from a
//! "players" head. Skipping a step: CR 614.1b/614.10.
//! Source-authored, deliberately unrun.
use ironsmith::cards::CardDefinition;
use ironsmith::{GameState, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};

const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);

const GIBBERING_DESCENT: &str = "Mana cost: {4}{B}{B}\nType: Enchantment\nAt the beginning of each player's upkeep, that player loses 1 life and discards a card.\nHellbent — Skip your upkeep step if you have no cards in hand.\nMadness {2}{B}{B} (If you discard this card, discard it into exile. When you do, cast it for its madness cost or put it into your graveyard.)";

fn routes(name: &str, text: &str) -> [CardDefinition; 2] {
    let (direct, loss) =
        ironsmith_compiler::parse_loss::capture(|| compile_to_runtime_definition(name, text, false));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let direct = direct.unwrap_or_else(|error| panic!("{name}: {error}"));
    let (compiled, loss) =
        ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, text, false));
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
fn gibbering_descent_skips_only_its_controllers_upkeep_while_hellbent() {
    for definition in routes("Gibbering Descent", GIBBERING_DESCENT) {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let in_hand = game.create_object_from_definition(
            &compile_to_runtime_definition("Hand card", "Type: Basic Land — Swamp", false).unwrap(),
            A,
            Zone::Hand,
        );
        assert!(!game.player_skips_upkeep_step(A), "a card in hand: not hellbent");
        game.move_object_by_effect(in_hand, Zone::Graveyard).unwrap();
        assert!(game.player_skips_upkeep_step(A), "hellbent: skip your upkeep");
        assert!(!game.player_skips_upkeep_step(B), "only the controller's upkeep");
    }
}

#[test]
fn unlabeled_skip_your_upkeep_line_is_reachable() {
    let text = "Mana cost: {2}\nType: Artifact\nSkip your upkeep step.";
    for definition in routes("Skip probe", text) {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.create_object_from_definition(&definition, A, Zone::Battlefield);
        assert!(game.player_skips_upkeep_step(A));
    }
}
