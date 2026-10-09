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

#[test]
fn death_cloud_each_player_chain_keeps_every_x_step() {
    for definition in support::definitions("Death Cloud") {
        let text = support::rendered(&definition);
        for phrase in ["loses x life", "discards x cards", "x creatures", "x lands"] {
            assert!(text.contains(phrase), "{phrase}: {text}");
        }
    }
}

/// Round 5: the copy-exception keywords are owned by p12 ("they have
/// vigilance and menace", CR 707.9a), so the ChooseLeadingSpell fail-loud
/// rule is retired; the body is a land target plus three token copies.
#[test]
fn rebuild_the_city_creates_three_land_creature_copies() {
    for definition in support::definitions("Rebuild the City") {
        let debug = format!("{definition:?}");
        assert!(debug.contains("CreateTokenCopy"), "{debug}");
        let text = support::rendered(&definition);
        support::assert_no_internal_markers("Rebuild the City", &text);
        assert!(text.contains("choose target land"), "{text}");
        assert!(text.contains("three tokens that are copies of it"), "{text}");
        assert!(text.contains("3/3"), "{text}");
        assert!(text.contains("vigilance and menace"), "{text}");
    }
}
