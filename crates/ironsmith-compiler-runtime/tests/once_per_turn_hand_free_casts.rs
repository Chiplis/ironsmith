//! "Once during each of your turns, you may cast <spells> from your hand
//! without paying its mana cost." (CR 118.9 with a per-turn usage budget).
//! Source-authored, deliberately unrun.
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{LegalAction, compute_legal_actions};
use ironsmith::{GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};

const A: PlayerId = PlayerId(0);

const ZAFFAI: &str = "Mana cost: {5}{U}{R}\nType: Legendary Creature — Human Bard Sorcerer\nPower/Toughness: 5/7\nOnce during each of your turns, you may cast an instant or sorcery spell from your hand without paying its mana cost.";
const VISION: &str = "Mana cost: {6}{U}{U}\nType: Legendary Artifact Creature — Robot Hero\nPower/Toughness: 2/5\nFlying\nOnce during each of your turns, you may cast a noncreature or Robot spell from your hand without paying its mana cost.";

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

fn free_casts(game: &GameState, id: ObjectId) -> usize {
    compute_legal_actions(game, A)
        .unwrap()
        .into_iter()
        .filter(|action| matches!(action, LegalAction::CastSpell { spell_id, casting_method: CastingMethod::Alternative(_), .. } if *spell_id == id))
        .count()
}

fn card(game: &mut GameState, name: &str, text: &str) -> ObjectId {
    game.create_object_from_definition(&compile_to_runtime_definition(name, text, false).unwrap(), A, Zone::Hand)
}

#[test]
fn once_per_turn_free_hand_casts_compile_with_a_usage_budget() {
    for (name, text) in [("Zaffai and the Tempests", ZAFFAI), ("Vision, Spectral Synthezoid", VISION)] {
        for definition in routes(name, text) {
            assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
            let text = format!("{:?}", definition.abilities);
            assert!(text.contains("OnceDuringEachOfYourTurns"), "{name}: {text}");
            assert!(text.contains("Cast without paying mana cost"), "{name}: {text}");
        }
    }
}

#[test]
fn zaffai_offers_free_casts_only_for_instants_and_sorceries_on_your_turn() {
    for definition in routes("Zaffai and the Tempests", ZAFFAI) {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.turn.active_player = A;
        game.turn.priority_player = Some(A);
        game.turn.phase = ironsmith::Phase::FirstMain;
        game.turn.step = None;
        game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let sorcery = card(&mut game, "Sorcery", "Mana cost: {5}{U}\nType: Sorcery\nDraw a card.");
        let creature = card(&mut game, "Creature", "Mana cost: {5}{G}\nType: Creature — Bear\nPower/Toughness: 2/2");
        assert!(free_casts(&game, sorcery) > 0, "free instant/sorcery cast");
        assert_eq!(free_casts(&game, creature), 0, "creature spells are outside the filter");
    }
}
