//! Ability-word-labelled triggers whose bodies used reference forms the
//! grammar rejected: a gendered source possessive in a trailing condition
//! (Viv Vision) and an untyped "for each counter on it" draw count
//! (Cleopatra). Source-authored, deliberately unrun.
use ironsmith::ability::AbilityKind;
use ironsmith::cards::CardDefinition;

#[path = "p02_line_families/compile.rs"]
mod compile;

const VIV_VISION: &str = "Mana cost: {3}\nType: Legendary Artifact Creature — Robot Hero\nPower/Toughness: 2/2\nFlying\nCybernetic Senses — Whenever Viv Vision attacks, draw a card if her power is 4 or greater.\nPower-up — {7}: Put two +1/+1 counters on Viv Vision. (Activate each power-up ability only once. Reduce the cost by her mana cost if she entered this turn.)";
const CLEOPATRA: &str = "Mana cost: {2}{B}{G}\nType: Legendary Creature — Human Noble\nPower/Toughness: 2/4\nAllies — At the beginning of your end step, put a +1/+1 counter on each of up to two other target legendary creatures.\nBetrayal — Whenever a legendary creature with counters on it dies, draw a card for each counter on it. You lose 2 life.";

fn triggered_debug(definition: &CardDefinition) -> Vec<String> {
    definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(format!("{triggered:?}")),
            _ => None,
        })
        .collect()
}

#[test]
fn viv_vision_draws_only_while_her_power_is_at_least_four() {
    for definition in compile::compile_both("Viv Vision, Teen Synthezoid", VIV_VISION) {
        let triggers = triggered_debug(&definition);
        let attack = triggers
            .iter()
            .find(|debug| debug.contains("DrawCardsEffect"))
            .expect("attack trigger draws");
        assert!(attack.contains("ConditionalEffect"), "{attack}");
        assert!(attack.contains("SourcePowerAtLeast(4)"), "{attack}");
    }
}

#[test]
fn cleopatra_counts_every_counter_on_the_dying_creature() {
    for definition in compile::compile_both("Cleopatra, Exiled Pharaoh", CLEOPATRA) {
        let triggers = triggered_debug(&definition);
        let betrayal = triggers
            .iter()
            .find(|debug| debug.contains("DrawCardsEffect"))
            .expect("betrayal trigger draws");
        assert!(betrayal.contains("CountersOn"), "{betrayal}");
        assert!(betrayal.contains("LoseLifeEffect"), "{betrayal}");
        assert!(
            !betrayal.contains("CountersOnSource"),
            "the count is the dying creature's counters, not Cleopatra's: {betrayal}"
        );
    }
}
