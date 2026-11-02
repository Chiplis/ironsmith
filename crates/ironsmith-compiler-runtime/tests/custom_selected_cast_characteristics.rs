use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;
use ironsmith::types::Supertype;
#[test]
fn selected_price_characteristic_riders_are_independent_and_survive_artifact_roundtrip() {
    for tail in [
        "If you cast this spell this way, it has base power and toughness 4/4.",
        "If you cast this spell this way, it is legendary. If you cast this spell this way, it has base power and toughness 4/4.",
        "If you cast this spell this way, it has base power and toughness 4/4. If you cast this spell this way, it enters tapped.",
    ] {
        let input = format!("Mana cost: {{G}}\nType: Creature\nPower/Toughness: 1/3\nYou may cast this spell for {{2}} rather than pay its mana cost. {tail}");
        let (direct, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_runtime_definition("Cast characteristic audit", &input, false));
        assert!(!loss.is_lossy(), "{}", loss.reasons_text());
        let (artifact, _) = compile_to_artifact("Cast characteristic audit", &input, false).unwrap();
        let restored = ironsmith_compiled_artifact::CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
        for definition in [direct.unwrap(), materialize_artifact(&restored).unwrap()] {
            assert_eq!(definition.alternative_casts.len(), 1);
            let instructions = definition.alternative_casts[0].battlefield_instructions();
            assert_eq!(instructions.casting_power_toughness, Some(ironsmith::card::PowerToughness::fixed(4, 4)));
            assert_eq!(instructions.casting_added_supertypes.contains(&Supertype::Legendary), tail.contains("legendary"));
            assert_eq!(instructions.enters_tapped, tail.contains("tapped"));
            assert!(definition.alternative_casts[0].prototype_power_toughness().is_none(), "the price must not replace mana characteristics");
        }
    }
}
