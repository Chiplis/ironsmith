//! cf8 p01: lines that were blocked by stale fail-loud rules now compile
//! through their typed owners. Source-authored, deliberately unrun.
use ironsmith::ability::AbilityKind;

#[path = "p01_support/mod.rs"]
mod support;

fn static_count(definition: &ironsmith::cards::CardDefinition) -> usize {
    definition
        .abilities
        .iter()
        .filter(|ability| matches!(ability.kind, AbilityKind::Static(_)))
        .count()
}

#[test]
fn tetsuko_power_or_toughness_unblockable_subject() {
    for definition in support::definitions("Tetsuko Umezawa, Fugitive") {
        let text = support::rendered(&definition);
        assert!(text.contains("power or toughness 1 or less"), "{text}");
        assert!(text.contains("can't be blocked"), "{text}");
    }
}

#[test]
fn hellraiser_goblin_grants_haste_and_must_attack() {
    for definition in support::definitions("Hellraiser Goblin") {
        assert!(static_count(&definition) >= 2, "{definition:?}");
        let text = support::rendered(&definition);
        assert!(text.contains("haste"), "{text}");
        assert!(text.contains("attack each combat if able"), "{text}");
    }
}

#[test]
fn leviathan_enters_tapped_and_skips_untap_as_two_statics() {
    for definition in support::definitions("Leviathan") {
        let text = support::rendered(&definition);
        assert!(text.contains("enters tapped"), "{text}");
        assert!(text.contains("untap during your untap step"), "{text}");
        assert!(text.contains("sacrifice two islands"), "{text}");
        assert!(text.contains("can't attack unless"), "{text}");
    }
}
