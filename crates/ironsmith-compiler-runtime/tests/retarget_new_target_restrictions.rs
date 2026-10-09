//! "Change the target of target spell that targets only a player. The new
//! target must be a player." (Rebound): CR 115.7 retarget restricted to a
//! player. Source-authored, deliberately unrun.

#[path = "p02_line_families/compile.rs"]
mod compile;

const REBOUND: &str = "Mana cost: {1}{U}\nType: Instant\nChange the target of target spell that targets only a player. The new target must be a player.";

#[test]
fn rebound_retargets_only_to_a_player() {
    for definition in compile::compile_both("Rebound", REBOUND) {
        let debug = format!("{:?}", definition.spell_effect);
        assert!(debug.contains("RetargetStackObjectEffect"), "{debug}");
        assert!(debug.contains("new_target_restriction: Some(Player(Any))"), "{debug}");
        assert!(debug.contains("targets_only_player"), "{debug}");
    }
}
