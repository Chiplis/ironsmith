use super::shard_16::{assert_oracle_card_parses_strict, parse_oracle_card_definition};
use super::*;

#[test]
pub(super) fn attached_destroy_followups_reuse_the_prior_object_reference() {
    for name in [
        "Blastfire Bolt",
        "Corrosive Ooze",
        "Treefolk Mystic",
        "Turn to Slag",
    ] {
        assert_oracle_card_parses_strict(name);
        let definition = parse_oracle_card_definition(name);
        let debug = format!("{definition:#?}");
        assert!(
            debug.contains("AttachedToTaggedObject"),
            "{name} must keep the prior creature as the attachment anchor: {debug}"
        );

        let compiled = compiled_text_lines(&definition)
            .join("\n")
            .to_ascii_lowercase();
        match name {
            "Blastfire Bolt" | "Turn to Slag" => assert!(
                compiled.contains(
                    "deals 5 damage to target creature. destroy all equipment attached to that creature"
                ),
                "{name}: {compiled}"
            ),
            "Corrosive Ooze" => assert!(
                compiled.contains("destroy all equipment attached to that creature at end of combat"),
                "{name}: {compiled}"
            ),
            "Treefolk Mystic" => assert!(
                compiled.contains("destroy all auras attached to that creature"),
                "{name}: {compiled}"
            ),
            _ => unreachable!(),
        }
    }
}
