//! "~ has all activated abilities of lands your opponents control except mana
//! abilities" (CR 613.1f, CR 605.1a). Source-authored, unrun. Sharkey's
//! any-type mana line belongs to p05's spend-as-any-type mechanism.
use ironsmith::cards::CardDefinition;
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler::parse_loss;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};

const SHARKEY: &str = "Mana cost: {2}{B}{R}\nType: Legendary Creature — Half-Elf Human Rogue\nPower/Toughness: 3/4\nActivated abilities of lands your opponents control can't be activated unless they're mana abilities.\nSharkey has all activated abilities of lands your opponents control except mana abilities.\nMana of any type can be spent to activate Sharkey's abilities.";

fn definitions(name: &str, text: &str) -> [CardDefinition; 2] {
    let (direct, loss) = parse_loss::capture(|| compile_to_runtime_definition(name, text, false));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let direct = direct.unwrap_or_else(|error| panic!("{name}: {error}"));
    let (artifact, _) = compile_to_artifact(name, text, false).unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    restored.validate().unwrap();
    let decoded =
        ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap();
    for definition in [&direct, &decoded] {
        assert!(
            !ironsmith::cards::generated_definition_has_unimplemented_content(definition),
            "{name}: unimplemented content"
        );
    }
    [direct, decoded]
}

#[test]
fn sharkey_copies_only_nonmana_land_abilities() {
    for definition in definitions("Sharkey, Tyrant of the Shire", SHARKEY) {
        let debug = format!("{definition:?}");
        assert!(debug.contains("CopyActivatedAbilities"), "{debug}");
        assert!(debug.contains("include_mana: false"), "{debug}");
    }
}
