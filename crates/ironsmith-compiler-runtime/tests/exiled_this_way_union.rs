//! Source-authored and deliberately unrun (cf8 p04): Crabomination's one
//! instruction exiles three groups (library top, random graveyard card,
//! random hand card); "cards exiled this way" names all three, so the free
//! cast chooses among the union, not only the last exile.
use ironsmith::cards::CardDefinition;
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;

fn definitions() -> [CardDefinition; 2] {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/exiled_this_way_union.json.fixture"
    ))
    .unwrap();
    let row = &rows[0];
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
fn crabomination_casts_from_the_union_of_its_three_exiles() {
    for definition in definitions() {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
        let debug = format!("{debug:?}", debug = definition);
        // The cast pool is a disjunction over the three exile result tags.
        let any_of = debug.matches("any_of: [ObjectFilter").count();
        assert!(any_of >= 1, "{debug}");
        assert!(debug.contains("CastTagged"), "{debug}");
        assert!(debug.contains("without_paying_mana_cost: true"), "{debug}");
    }
}

/// Two separate exile sentences are two instructions: "the card exiled this
/// way" keeps naming the most recent exile only (no union).
#[test]
fn separate_exile_sentences_bind_this_way_to_the_latest_exile() {
    let definition = compile_to_runtime_definition(
        "Two Exiles",
        "Mana cost: {3}{R}\nType: Sorcery\n\
         Exile target creature an opponent controls. Exile the top card of your library. \
         Until end of turn, you may play the card exiled this way.",
        false,
    )
    .unwrap();
    assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
    let debug = format!("{definition:?}");
    assert!(!debug.contains("any_of: [ObjectFilter"), "{debug}");
}
