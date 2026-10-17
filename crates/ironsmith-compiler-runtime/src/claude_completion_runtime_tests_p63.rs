//! Claude package 63 runtime checks for the compilation-completion task
//! (see reports/compilation-completion-20261016/COORDINATION.md).
#![allow(unused_imports)]
use super::compile_to_runtime_definition;
use ironsmith::effect::Effect;

/// "Return target creature ..., then roll a six-sided die. If the result is 3
/// or less, scry a number of cards equal to the result.": the roll is the
/// terminal member of an ordered typed coordination, so the coordination's
/// result ID observes it and both "the result" readings bind that one roll.
#[test]
fn boing_binds_the_result_of_a_roll_after_comma_then() {
    let definition = compile_to_runtime_definition(
        "Boing!",
        "Mana cost: {1}{U}\nType: Instant\nReturn target creature to its owner's hand, then roll a six-sided die. If the result is 3 or less, scry a number of cards equal to the result.",
        false,
    )
    .expect("Boing! should compile");
    let debug = format!("{definition:?}");
    assert!(debug.contains("RollDie"), "{debug}");
    assert!(debug.contains("Scry"), "{debug}");
    assert!(debug.contains("Rolled"), "the scry count reads the roll: {debug}");
    assert!(!debug.contains("Scry { count: Fixed(1)"), "the result is not dropped to Scry 1: {debug}");
}

/// "Enchanted creature gets +2/+2 and can't be blocked unless it's attacking
/// its owner or a permanent its owner controls.": the anthem plus an
/// attack-target evasion marker granted to the enchanted host.
#[test]
fn become_the_pilot_grants_owner_relative_evasion() {
    let definition = compile_to_runtime_definition(
        "Become the Pilot",
        "Mana cost: {2}{U}{U}\nType: Enchantment — Aura\nEnchant noncommander creature\nYou control enchanted creature.\nEnchanted creature gets +2/+2 and can't be blocked unless it's attacking its owner or a permanent its owner controls.",
        false,
    )
    .expect("Become the Pilot should compile");
    let debug = format!("{definition:?}");
    assert!(debug.contains("CantBeBlockedUnlessAttackingOwnerOrOwnersPermanent"), "{debug}");
    assert!(!debug.contains("Unblockable"), "not plain unblockable: {debug}");
}

/// "{3}{R/W}{R/W}: Creatures your opponents control block this turn if able,
/// and you choose how those creatures block.": a resolving block requirement
/// over the described set plus this turn's block-choice control.
#[test]
fn brutal_hordechief_forces_blocks_and_takes_block_choices() {
    let definition = compile_to_runtime_definition(
        "Brutal Hordechief",
        "Mana cost: {3}{B}\nType: Creature — Orc Warrior\nPower/Toughness: 3/3\nWhenever a creature you control attacks, defending player loses 1 life and you gain 1 life.\n{3}{R/W}{R/W}: Creatures your opponents control block this turn if able, and you choose how those creatures block.",
        false,
    )
    .expect("Brutal Hordechief should compile");
    let debug = format!("{definition:?}");
    assert!(debug.contains("MustBlock"), "{debug}");
    assert!(debug.contains("ControlCombatChoicesThisTurn"), "{debug}");
}

/// "After this phase, there is an additional combat phase. Only land
/// creatures can attack during that combat phase.": nonland creatures can't
/// attack during the combat phase that trigger added.
#[test]
fn bumi_limits_the_added_combat_to_land_creatures() {
    let definition = compile_to_runtime_definition(
        "Bumi, Unleashed",
        "Mana cost: {3}{R}{G}\nType: Legendary Creature — Human Avatar Ally\nPower/Toughness: 5/4\nTrample\nWhen Bumi enters, earthbend 4.\nWhenever Bumi deals combat damage to a player, untap all lands you control. After this phase, there is an additional combat phase. Only land creatures can attack during that combat phase.",
        false,
    )
    .expect("Bumi should compile");
    let debug = format!("{definition:?}");
    assert!(debug.contains("LastAddedCombatPhase"), "{debug}");
    assert!(debug.contains("excluded_card_types: [Land]"), "{debug}");
}

/// "You may cast this card from your graveyard using its mutate ability.":
/// the intrinsic mutate alternative cost is authorized from the graveyard.
#[test]
fn brokkos_may_mutate_from_the_graveyard() {
    let definition = compile_to_runtime_definition(
        "Brokkos, Apex of Forever",
        "Mana cost: {2}{B}{G}{U}\nType: Legendary Creature — Nightmare Beast Elemental\nPower/Toughness: 6/6\nMutate {2}{U/B}{G}{G} (If you cast this spell for its mutate cost, put it over or under target non-Human creature you own. They mutate into the creature on top plus all abilities from under it.)\nTrample\nYou may cast this card from your graveyard using its mutate ability.",
        false,
    )
    .expect("Brokkos should compile");
    let debug = format!("{definition:?}");
    assert!(debug.contains("NativeAlternativeCastFromZone"), "{debug}");
    assert!(debug.contains("method: Mutate"), "{debug}");
}

/// "You may assign this creature's combat damage divided as you choose among
/// defending player and/or any number of creatures they control.": a marker
/// the combat-damage division reads as a free recipient set.
#[test]
fn butcher_orgg_may_divide_damage_freely() {
    let definition = compile_to_runtime_definition(
        "Butcher Orgg",
        "Mana cost: {4}{R}{R}{R}\nType: Creature — Orgg\nPower/Toughness: 6/6\nYou may assign this creature's combat damage divided as you choose among defending player and/or any number of creatures they control.",
        false,
    )
    .expect("Butcher Orgg should compile");
    let debug = format!("{definition:?}");
    assert!(debug.contains("MayDivideCombatDamageAmongDefendingPlayerAndCreatures"), "{debug}");
}

/// "Note the mana value of each card as it's put into exile. Bell Borca's
/// power is equal to the greatest number noted for it this turn.": the CDA
/// reads this turn's exile history since the source entered.
#[test]
fn bell_borca_power_reads_exiled_mana_values_noted_this_turn() {
    let definition = compile_to_runtime_definition(
        "Bell Borca, Spectral Sergeant",
        "Mana cost: {2}{R}{W}\nType: Legendary Creature — Spirit Soldier\nPower/Toughness: */5\nNote the mana value of each card as it's put into exile.\nBell Borca's power is equal to the greatest number noted for it this turn.\nAt the beginning of your upkeep, exile the top card of your library. You may play that card this turn.",
        false,
    )
    .expect("Bell Borca should compile");
    let debug = format!("{definition:?}");
    assert!(debug.contains("GreatestManaValueExiledThisTurnSinceSourceEntered"), "{debug}");
    assert!(debug.contains("Note the mana value of each card"), "{debug}");
    assert!(!debug.contains("KeywordFallbackText"), "{debug}");
}

/// "Whenever a player attacks one of your opponents, if that opponent has
/// more life than another of your opponents, ...": a per-defender attack
/// trigger with an intervening-if over the attacked opponent's life.
#[test]
fn breena_triggers_per_attacked_opponent_with_life_comparison() {
    let definition = compile_to_runtime_definition(
        "Breena, the Demagogue",
        "Mana cost: {1}{W}{B}\nType: Legendary Creature — Bird Warlock\nPower/Toughness: 1/3\nFlying\nWhenever a player attacks one of your opponents, if that opponent has more life than another of your opponents, that attacking player draws a card and you put two +1/+1 counters on a creature you control.",
        false,
    )
    .expect("Breena should compile");
    let debug = format!("{definition:?}");
    assert!(debug.contains("PlayerHasMoreLifeThanAnotherOpponent"), "{debug}");
    assert!(debug.contains("Defending"), "{debug}");
}
