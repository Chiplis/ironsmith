//! "Players can cast spells only during their own turns." (Dosan the
//! Falling Leaf): non-active players can't cast spells. Source-authored,
//! deliberately unrun.
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{LegalAction, compute_legal_actions};
use ironsmith::mana::ManaSymbol;
use ironsmith::{GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};

const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);

const DOSAN: &str = "Mana cost: {1}{G}{G}\nType: Legendary Creature — Human Monk\nPower/Toughness: 2/2\nPlayers can cast spells only during their own turns.";

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

fn casts(game: &GameState, player: PlayerId, id: ObjectId) -> usize {
    compute_legal_actions(game, player)
        .unwrap()
        .into_iter()
        .filter(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == id))
        .count()
}

#[test]
fn only_the_active_player_may_cast_spells() {
    for definition in routes("Dosan the Falling Leaf", DOSAN) {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.turn.active_player = A;
        game.turn.phase = ironsmith::Phase::FirstMain;
        game.turn.step = None;
        for player in [A, B] {
            game.player_mut(player).unwrap().mana_pool.add(ManaSymbol::Blue, 5);
        }
        game.create_object_from_definition(&definition, B, Zone::Battlefield);
        let instant = "Mana cost: {U}\nType: Instant\nDraw a card.";
        let mine = game.create_object_from_definition(
            &compile_to_runtime_definition("Mine", instant, false).unwrap(), A, Zone::Hand);
        let theirs = game.create_object_from_definition(
            &compile_to_runtime_definition("Theirs", instant, false).unwrap(), B, Zone::Hand);
        game.update_cant_effects();
        game.turn.priority_player = Some(A);
        assert!(casts(&game, A, mine) > 0, "the active player may cast");
        game.turn.priority_player = Some(B);
        assert_eq!(casts(&game, B, theirs), 0, "a non-active player can't cast, even Dosan's controller");
    }
}
