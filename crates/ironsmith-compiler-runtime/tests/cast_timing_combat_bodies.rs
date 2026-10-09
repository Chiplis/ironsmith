//! Combat-control spell bodies behind "Cast this spell only ..." timing:
//! referenced-creature requirements in the combat phase an instruction adds
//! (CR 500.8, 508.1c-d), must-block requirements (CR 509.1c), block
//! declaration control (CR 509.1) and two-die rolls that ignore the lower
//! roll. Source-authored, deliberately unrun.
#[path = "p02_line_families/compile.rs"]
mod compile;

const GAMBIT: &str = "Mana cost: {2}{U}{U}\nType: Instant\nCast this spell only during the declare blockers step on an opponent's turn.\nRemove all attacking creatures from combat and untap them. After this phase, there is an additional combat phase. Each of those creatures attacks that combat if able. They can't attack you or planeswalkers you control that combat.";
const FRENZY: &str = "Mana cost: {2}{R}\nType: Instant\nCast this spell only before combat or during combat before blockers are declared.\nRoll two d20 and ignore the lower roll.\n1—14 | Choose any number of creatures. They block this turn if able.\n15—20 | You choose which creatures block this turn and how those creatures block.";

#[test]
fn illusionists_gambit_adds_a_combat_bound_to_the_removed_attackers() {
    for definition in compile::compile_both("Illusionist's Gambit", GAMBIT) {
        let debug = format!("{definition:?}");
        assert!(debug.contains("RemoveFromCombat"), "{debug}");
        assert!(debug.contains("AdditionalPhases"), "{debug}");
        // Both "that combat" rules start in the added combat phase.
        assert_eq!(debug.matches("LastAddedCombatPhase").count(), 2, "{debug}");
        assert!(debug.contains("MustAttack"), "{debug}");
        assert!(debug.contains("AttackPlayerOrPlaneswalkersControlledBy"), "{debug}");
        assert!(debug.contains("EndOfCombat"), "{debug}");
    }
}

#[test]
fn berserkers_frenzy_keeps_the_higher_d20_and_has_both_block_rows() {
    for definition in compile::compile_both("Berserker's Frenzy", FRENZY) {
        let debug = format!("{definition:?}");
        assert!(debug.contains("ignore_lower: true"), "{debug}");
        assert!(debug.contains("sides: 20"), "{debug}");
        assert!(debug.contains("MustBlock"), "{debug}");
        assert!(debug.contains("ControlCombatChoicesThisTurn"), "{debug}");
    }
}

const TAUNT: &str = "Mana cost: {U}\nType: Sorcery\nDuring target player's next turn, creatures that player controls attack you if able.";

#[test]
fn taunt_requires_the_target_players_creatures_to_attack_you_next_turn() {
    for definition in compile::compile_both("Taunt", TAUNT) {
        let debug = format!("{definition:?}");
        assert!(debug.contains("MustAttackPlayer"), "{debug}");
        assert!(debug.contains("NextTurn"), "{debug}");
        assert!(debug.contains("player: You"), "{debug}");
    }
}

const SIRENS_CALL: &str = "Mana cost: {U}\nType: Instant\nCast this spell only during an opponent's turn, before attackers are declared.\nCreatures the active player controls attack this turn if able.\nAt the beginning of the next end step, destroy all non-Wall creatures that player controls that didn't attack this turn. Ignore this effect for each creature the player didn't control continuously since the beginning of the turn.";

#[test]
fn sirens_call_binds_that_player_to_the_active_player_and_spares_newcomers() {
    for definition in compile::compile_both("Siren's Call", SIRENS_CALL) {
        let debug = format!("{definition:?}");
        // Line 2: the requirement covers the active player's creatures only,
        // and "the active player" becomes the player antecedent.
        assert!(debug.contains("MustAttack"), "{debug}");
        // Line 3: "that player" binds to that antecedent through ordinary
        // reference resolution, so both the requirement and the delayed
        // destroy are scoped to the active player's creatures.
        assert!(
            debug.matches("controller: Some(Active)").count() >= 2,
            "{debug}"
        );
        assert!(!debug.contains("IteratedPlayer"), "'that player' is bound: {debug}");
        // The trailing "Ignore this effect for each creature the player
        // didn't control continuously since the beginning of the turn"
        // folds into the destroy as the complementary relation (CR 302.6).
        assert!(
            debug.contains("controlled_continuously_since_turn_began: Some(true)"),
            "{debug}"
        );
        assert!(debug.contains("Wall"), "{debug}");
    }
}

const IGNORE_OWN_CREATURES_DESTROY: &str = "Mana cost: {2}{B}{B}\nType: Sorcery\nDestroy all creatures. Ignore this effect for each creature you control.";
const IGNORE_OWN_CREATURES_DAMAGE: &str = "Mana cost: {2}{R}\nType: Sorcery\nThis spell deals 2 damage to each creature. Ignore this effect for each creature you control.";

#[test]
fn ignore_this_effect_for_each_excludes_objects_from_any_preceding_set_instruction() {
    for (name, text) in [
        ("Ignore Exclusion Destroy", IGNORE_OWN_CREATURES_DESTROY),
        ("Ignore Exclusion Damage", IGNORE_OWN_CREATURES_DAMAGE),
    ] {
        for definition in compile::compile_both(name, text) {
            let debug = format!("{definition:?}");
            // "each creature you control" is removed from the affected set:
            // only creatures you don't control are destroyed / dealt damage.
            assert!(debug.contains("controller: Some(NotYou)"), "{name}: {debug}");
            assert!(!debug.contains("controller: Some(You)"), "{name}: {debug}");
        }
    }
}
