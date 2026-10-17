//! Claude package 48 runtime checks for the compilation-completion task
//! (see reports/compilation-completion-20261016/COORDINATION.md).
#![allow(unused_imports)]
use super::compile_to_runtime_definition;
use ironsmith::effect::Effect;

const GOBLIN_CHARBELCHER: &str = "Mana cost: {4}\nType: Artifact\n{3}, {T}: Reveal cards from the top of your library until you reveal a land card. This artifact deals damage equal to the number of nonland cards revealed this way to any target. If the revealed land card was a Mountain, this artifact deals double that damage instead. Put the revealed cards on the bottom of your library in any order.";

#[test]
fn goblin_charbelcher_tests_the_stopping_land_and_counts_the_revealed_nonlands() {
    let definition =
        compile_to_runtime_definition("Goblin Charbelcher", GOBLIN_CHARBELCHER, false).unwrap();
    let debug = format!("{definition:#?}");
    assert!(debug.contains("ConsultTopOfLibraryEffect"), "{debug}");
    assert!(debug.contains("Mountain"), "{debug}");
    // The Mountain test reads the card that stopped the traversal, never the
    // damage recipient.
    let condition = debug
        .find("TaggedObjectMatches(")
        .expect("the replacement condition tests a tagged object");
    let condition_tag = &debug[condition..debug[condition..].find(')').map_or(debug.len(), |end| condition + end)];
    assert!(condition_tag.contains("consult_match"), "{condition_tag}");
    // Both damage arms count the whole revealed collection (minus lands),
    // not the singular land match.
    let mut damage_arms = 0;
    for (start, _) in debug.match_indices("DealDamageEffect") {
        let arm = &debug[start..];
        let arm = &arm[..arm.find("target:").unwrap_or(arm.len())];
        assert!(arm.contains("_revealed_"), "{arm}");
        assert!(!arm.contains("consult_match"), "{arm}");
        damage_arms += 1;
    }
    assert!(damage_arms >= 2, "default and doubled damage arms: {debug}");
    assert!(debug.contains("Scaled"), "the replacement doubles the default amount: {debug}");
}
