//! "[You may] put a card you own from outside the game into your hand / on top
//! of your library" chooses from the owner's sideboard (CR 400.11).
//! Source-authored, deliberately unrun.
use ironsmith::card::CardBuilder;
use ironsmith::cards::CardDefinition;
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::{CardId, CardType, GameState, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;

const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);

fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!("../../../fixtures/outside_game_put.json.fixture")).unwrap()
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
    for definition in [&direct, &decoded] {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(definition));
    }
    [direct, decoded]
}

#[test]
fn every_wish_body_chooses_an_owned_outside_game_card_on_both_routes() {
    let rows = fixtures();
    assert_eq!(rows.len(), 4);
    for row in &rows {
        let name = row["name"].as_str().unwrap();
        for definition in definitions(name, row["text"].as_str().unwrap()) {
            let debug = format!("{definition:?}");
            assert!(debug.contains("OutsideGame"), "{name}: {debug}");
            assert!(debug.contains("ChooseObjects"), "{name}");
        }
    }
}

#[test]
fn put_from_outside_the_game_moves_only_an_owned_sideboard_card() {
    let text = "Mana cost: {2}{B}{B}\nType: Sorcery\nPut a card you own from outside the game into your hand.";
    for definition in definitions("Outside game probe", text) {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&definition, A, Zone::Exile);
        let card = |name: &str| CardBuilder::new(CardId::new(), name).card_types(vec![CardType::Artifact]).build();
        let theirs = game.create_object_from_card(&card("Theirs"), B, Zone::OutsideGame);
        let mine = game.create_object_from_card(&card("Mine"), A, Zone::OutsideGame);
        let mut dm = SelectFirstDecisionMaker;
        for effect in definition.spell_effect.as_ref().unwrap().flattened_default_effects() {
            execute_effect(&mut game, effect, &mut EffectContext::new(source, A, &mut dm)).unwrap();
        }
        assert_eq!(game.player(A).unwrap().hand.len(), 1);
        let in_hand = game.player(A).unwrap().hand.iter().next().copied().unwrap();
        assert_eq!(game.object(in_hand).unwrap().name.to_string(), "Mine");
        assert_eq!(game.object(theirs).map(|object| object.zone), Some(Zone::OutsideGame));
        let _ = mine;
    }
}

#[test]
fn research_shuffles_up_to_four_owned_outside_game_cards() {
    let text = "Type: Instant\nShuffle up to four cards you own from outside the game into your library.";
    for definition in definitions("Research probe", text) {
        let debug = format!("{definition:?}");
        assert!(debug.contains("OutsideGame"), "{debug}");
        assert!(debug.contains("max: Some(4)"), "{debug}");
    }
}
