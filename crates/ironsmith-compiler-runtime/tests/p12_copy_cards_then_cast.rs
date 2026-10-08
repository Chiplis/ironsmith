//! Copies of exiled cards that may be cast (CR 707.12): "Copy that card three
//! times. You may cast the copies ..." and "Copy them. You may cast any
//! number of the copies." Source-authored, unrun.
use ironsmith::cards::CardDefinition;
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler::parse_loss;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};

const MNEMONIC_DELUGE: &str = "Mana cost: {6}{U}{U}{U}\nType: Sorcery\nExile target instant or sorcery card from a graveyard. Copy that card three times. You may cast the copies without paying their mana costs. Exile Mnemonic Deluge.";
const TALE_OF_TAMIYO: &str = "Mana cost: {2}{U}\nType: Legendary Enchantment — Saga\n(As this Saga enters and after your draw step, add a lore counter. Sacrifice after IV.)\nI, II, III — Mill two cards. If two cards that share a card type were milled this way, draw a card and repeat this process.\nIV — Exile any number of target instant, sorcery, and/or Tamiyo planeswalker cards from your graveyard. Copy them. You may cast any number of the copies.";

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
fn mnemonic_deluge_casts_up_to_three_free_copies_of_the_exiled_card() {
    for definition in definitions("Mnemonic Deluge", MNEMONIC_DELUGE) {
        let debug = format!("{definition:?}");
        assert!(debug.contains("CastTagged"), "{debug}");
        assert!(debug.contains("as_copy: true"), "{debug}");
        assert!(debug.contains("Fixed(3)"), "three copies: {debug}");
        assert!(debug.contains("without_paying_mana_cost: true"), "{debug}");
        assert!(!debug.contains("CopySpellEffect"), "a card is not a stack spell: {debug}");
    }
}

#[test]
fn tale_of_tamiyo_copies_each_exiled_card() {
    for definition in definitions("The Tale of Tamiyo", TALE_OF_TAMIYO) {
        let debug = format!("{definition:?}");
        assert!(debug.contains("ForEachTagged"), "{debug}");
        assert!(debug.contains("as_copy: true"), "{debug}");
        assert!(!debug.contains("CopySpellEffect"), "{debug}");
    }
}
