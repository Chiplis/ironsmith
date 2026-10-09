//! "As long as the top card of your library is <quality>" statics (CR 401.1).
//! Source-authored, deliberately unrun.
use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::cards::CardDefinition;
use ironsmith::{CardId, CardType, ColorSet, GameState, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;

const A: PlayerId = PlayerId::from_index(0);

fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!("../../../fixtures/top_of_library_conditions.json.fixture"))
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
    for definition in [&direct, &decoded] {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(definition));
    }
    [direct, decoded]
}

#[test]
fn library_top_statics_carry_the_typed_condition_on_both_routes() {
    let rows = fixtures();
    assert_eq!(rows.len(), 2);
    for row in &rows {
        let name = row["name"].as_str().unwrap();
        for definition in definitions(name, row["text"].as_str().unwrap()) {
            let debug = format!("{definition:?}");
            let expected = if name == "Mul Daya Channelers" { 2 } else { 1 };
            assert!(
                debug.matches("TopCardOfYourLibraryMatches").count() >= expected,
                "{name}: {debug}"
            );
        }
    }
}

fn card(name: &str, card_type: CardType, colors: ColorSet) -> ironsmith::card::Card {
    let mut builder = CardBuilder::new(CardId::new(), name)
        .card_types(vec![card_type])
        .color_indicator(colors);
    if card_type == CardType::Creature {
        builder = builder.power_toughness(PowerToughness::fixed(1, 1));
    }
    builder.build()
}

#[test]
fn mul_daya_channelers_tracks_the_current_top_card() {
    let row = fixtures()
        .into_iter()
        .find(|row| row["name"] == "Mul Daya Channelers")
        .unwrap();
    for definition in definitions("Mul Daya Channelers", row["text"].as_str().unwrap()) {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let channelers = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        game.create_object_from_card(&card("Bear", CardType::Creature, ColorSet::GREEN), A, Zone::Library);
        assert_eq!(game.calculated_power(channelers), Some(5), "creature on top");
        game.create_object_from_card(&card("Forest", CardType::Land, ColorSet::default()), A, Zone::Library);
        assert_eq!(game.calculated_power(channelers), Some(2), "a land is now on top");
    }
}
