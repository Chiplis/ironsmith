//! "Commander ninjutsu {U}{B}" (Yuriko, the Tiger's Shadow): ninjutsu that
//! functions from the hand and the command zone (CR 702.49d).
//! Source-authored, deliberately unrun.
use ironsmith::ability::AbilityKind;
use ironsmith::Zone;

#[path = "p02_line_families/compile.rs"]
mod compile;

const YURIKO: &str = "Mana cost: {1}{U}{B}\nType: Legendary Creature — Human Ninja\nPower/Toughness: 1/3\nCommander ninjutsu {U}{B} ({U}{B}, Return an unblocked attacker you control to hand: Put this card onto the battlefield from your hand or the command zone tapped and attacking.)\nWhenever a Ninja you control deals combat damage to a player, reveal the top card of your library and put that card into your hand. Each opponent loses life equal to that card's mana value.";

#[test]
fn yuriko_ninjutsu_functions_from_hand_and_command_zone() {
    for definition in compile::compile_both("Yuriko, the Tiger's Shadow", YURIKO) {
        let ninjutsu = definition
            .abilities
            .iter()
            .find(|ability| match &ability.kind {
                AbilityKind::Activated(activated) => {
                    activated.keyword == Some(ironsmith_core::ActivatedAbilityKeyword::Ninjutsu)
                }
                _ => false,
            })
            .expect("commander ninjutsu activated ability");
        assert!(ninjutsu.functional_zones.contains(&Zone::Hand));
        assert!(ninjutsu.functional_zones.contains(&Zone::Command));
        let debug = format!("{ninjutsu:?}");
        assert!(debug.contains("NinjutsuCostEffect"), "{debug}");
        assert!(debug.contains("NinjutsuEffect"), "{debug}");
    }
}
