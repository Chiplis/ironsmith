//! cf8 p05: "Only the player this creature is attacking may activate this
//! ability and only during the declare attackers step" plus "reselect which
//! player this creature is attacking". Source-authored; deliberately unrun.
#[path = "cf8_p05_support/mod.rs"]
mod support;
use ironsmith::ability::{AbilityKind, ActivationTiming};

#[test]
fn capricopian_is_activated_by_the_attacked_player() {
    for definition in support::definitions(&support::row("attacked_player_activation", "Capricopian")) {
        assert!(definition.abilities.iter().any(|ability| matches!(
            &ability.kind,
            AbilityKind::Activated(activated)
                if activated.timing == ActivationTiming::DeclareAttackersStepByAttackedPlayer
                    && activated.allows_any_player_to_activate()
        )));
        let debug = support::debug(&definition);
        assert!(debug.contains("ReselectAttackTargetEffect"));
        assert!(debug.contains("players_only: true"));
    }
}
