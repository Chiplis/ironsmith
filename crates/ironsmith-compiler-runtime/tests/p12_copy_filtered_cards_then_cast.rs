//! Copying cards named by a filter or "for each card exiled this way", then
//! casting the copies (CR 707.12). Source-authored, unrun.
use ironsmith::cards::CardDefinition;
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler::parse_loss;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};

const ARCANE_BOMBARDMENT: &str = "Mana cost: {4}{R}{R}\nType: Enchantment\nWhenever you cast your first instant or sorcery spell each turn, exile an instant or sorcery card at random from your graveyard. Then copy each card exiled with this enchantment. You may cast any number of the copies without paying their mana costs.";
const ZETHI: &str = "Mana cost: {1}{W}{U}\nType: Legendary Creature — Human Soldier\nPower/Toughness: 3/3\nMultikicker {W/U}\nWhen Zethi, Arcane Blademaster enters, exile up to X target instant cards from your graveyard, where X is the number of times Zethi was kicked. Put a kick counter on each of them.\nWhenever Zethi attacks, copy each exiled card you own with a kick counter on it. You may cast the copies.";
const MIZZIXS_MASTERY: &str = "Mana cost: {3}{R}\nType: Sorcery\nExile target card that's an instant or sorcery from your graveyard. For each card exiled this way, copy it, and you may cast the copy without paying its mana cost. Exile Mizzix's Mastery.\nOverload {5}{R}{R}{R} (You may cast this spell for its overload cost. If you do, change \"target\" in its text to \"each.\")";

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

fn assert_each_copy_cast(name: &str, text: &str) {
    for definition in definitions(name, text) {
        let debug = format!("{definition:?}");
        assert!(debug.contains("ForEachTagged"), "{name}: {debug}");
        assert!(debug.contains("as_copy: true"), "{name}: {debug}");
        assert!(!debug.contains("CopySpellEffect"), "{name}: {debug}");
    }
}

#[test]
fn arcane_bombardment_copies_every_card_exiled_with_it() {
    assert_each_copy_cast("Arcane Bombardment", ARCANE_BOMBARDMENT);
}

#[test]
fn zethi_copies_each_kicked_exiled_card() {
    assert_each_copy_cast("Zethi, Arcane Blademaster", ZETHI);
}

#[test]
fn mizzixs_mastery_copies_each_card_exiled_this_way() {
    assert_each_copy_cast("Mizzix's Mastery", MIZZIXS_MASTERY);
}
