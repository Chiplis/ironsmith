//! Claude package 73 runtime checks for the compilation-completion task
//! (see reports/compilation-completion-20261016/claude2/BRIEF.md).
#![allow(unused_imports)]
use super::compile_to_runtime_definition;
use ironsmith::effect::Effect;

#[test]
fn motivated_pony_reads_modern_entered_under_your_control_wording() {
    let definition = compile_to_runtime_definition("Motivated Pony",
        "Mana cost: {4}{G}\nType: Creature — Horse\nPower/Toughness: 3/3\nTrample, haste\nWhenever this creature attacks, attacking creatures get +1/+1 until end of turn. If a Food entered under your control this turn, untap those creatures and they get an additional +2/+2 until end of turn.", false)
        .expect("Motivated Pony should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("ObjectEnteredBattlefieldThisTurn"),
        "'a Food entered under your control this turn' is the battlefield-entry history: {debug}");
    assert!(debug.contains("Food"), "{debug}");
    assert!(debug.contains("Untap"), "{debug}");
}

#[test]
fn lightning_runner_if_you_pay_gates_untap_and_extra_combat_on_the_energy_payment() {
    let definition = compile_to_runtime_definition("Lightning Runner",
        "Mana cost: {3}{R}{R}\nType: Creature — Human Warrior\nPower/Toughness: 2/2\nDouble strike, haste\nWhenever this creature attacks, you get {E}{E}, then you may pay eight {E}. If you pay, untap all creatures you control, and after this phase, there is an additional combat phase.", false)
        .expect("Lightning Runner should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("PayEnergyEffect"), "{debug}");
    assert!(debug.contains("UntapEffect"), "{debug}");
    assert!(debug.contains("AdditionalPhasesEffect"), "{debug}");
    let pay = debug.find("PayEnergyEffect").unwrap();
    let untap = debug.find("UntapEffect").unwrap();
    assert!(pay < untap, "the untap is gated after the optional payment: {debug}");
}

#[test]
fn halfdane_imperative_base_pair_change_copies_target_until_end_of_next_upkeep() {
    let definition = compile_to_runtime_definition("Halfdane",
        "Mana cost: {1}{W}{U}{B}\nType: Legendary Creature — Shapeshifter\nPower/Toughness: 3/3\nAt the beginning of your upkeep, change Halfdane's base power and toughness to the power and toughness of target creature other than Halfdane until the end of your next upkeep.", false)
        .expect("Halfdane should compile");
    let debug = format!("{:?}", definition.abilities);
    for required in ["YourNextUpkeepEnd", "SetPowerToughness", "PowerOf", "ToughnessOf", "other: true"] {
        assert!(debug.contains(required), "missing {required}: {debug}");
    }
}

#[test]
fn monument_to_perfection_searches_basic_sphere_or_locus_lands() {
    let definition = compile_to_runtime_definition("Monument to Perfection",
        "Mana cost: {2}\nType: Artifact\n{3}, {T}: Search your library for a basic, Sphere, or Locus land card, reveal it, put it into your hand, then shuffle.\n{3}: This artifact becomes a 9/9 Phyrexian Construct artifact creature, loses all abilities, and gains indestructible and toxic 9. Activate only if there are nine or more lands with different names among the basic, Sphere, and Locus lands you control.", false)
        .expect("Monument to Perfection should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("Sphere"), "Sphere is a land subtype in the land-quality list: {debug}");
    assert!(debug.contains("Locus"), "{debug}");
    assert!(debug.contains("Basic"), "{debug}");
    assert!(debug.contains("activation_condition: Some(ValueComparison { left: DistinctNames("), "the activation gate counts distinct names: {debug}");
}

#[test]
fn impulsive_maneuvers_gates_next_combat_damage_double_or_prevent_on_the_flip() {
    let definition = compile_to_runtime_definition("Impulsive Maneuvers",
        "Mana cost: {2}{R}{R}\nType: Enchantment\nWhenever a creature attacks, flip a coin. If you win the flip, the next time that creature would deal combat damage this turn, it deals double that damage instead. If you lose the flip, the next time that creature would deal combat damage this turn, prevent that damage.", false)
        .expect("Impulsive Maneuvers should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("RegisterDamageMultiplierEffect"), "{debug}");
    assert!(debug.contains("factor: 2"), "{debug}");
    assert!(debug.contains("OneShot"), "{debug}");
    assert!(debug.contains("combat_only: true"), "{debug}");
    assert!(debug.contains("PreventNextTime"), "the lose branch prevents the next combat damage: {debug}");
    assert!(!debug.contains("DealDamageEffect"), "no bogus damage clause: {debug}");
}

#[test]
fn lynde_moves_a_curse_attached_to_you_onto_an_opponent() {
    let definition = compile_to_runtime_definition("Lynde, Cheerful Tormentor",
        "Mana cost: {1}{U}{B}{R}\nType: Legendary Creature — Human Warlock\nPower/Toughness: 2/4\nDeathtouch\nWhenever a Curse is put into your graveyard from the battlefield, return it to the battlefield attached to you at the beginning of the next end step.\nAt the beginning of your upkeep, you may attach a Curse attached to you to one of your opponents. If you do, draw two cards.", false)
        .expect("Lynde should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("attached_to_player: Some(You)"),
        "the moved Curse is one enchanting you: {debug}");
    assert!(debug.contains("Curse"), "{debug}");
    assert!(debug.contains("Opponent"), "the new host is one of your opponents: {debug}");
}

#[test]
fn lumbering_laundry_grants_face_down_look_permission_until_end_of_turn() {
    let definition = compile_to_runtime_definition("Lumbering Laundry",
        "Mana cost: {5}\nType: Artifact Creature — Golem\nPower/Toughness: 4/5\n{2}: Until end of turn, you may look at face-down creatures you don't control any time.\nDisguise {5}", false)
        .expect("Lumbering Laundry should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("LookAtFaceDownCreaturesYouDontControl"), "{debug}");
    assert!(debug.contains("EndOfTurn"), "the permission lasts until end of turn: {debug}");
}

#[test]
fn honest_work_turns_enchanted_creature_into_named_mana_citizen() {
    let definition = compile_to_runtime_definition("Honest Work",
        "Mana cost: {U}\nType: Enchantment — Aura\nEnchant creature an opponent controls\nWhen this Aura enters, tap enchanted creature and remove all counters from it.\nEnchanted creature loses all abilities and is a Citizen with base power and toughness 1/1 and \"{T}: Add {C}\" named Humble Merchant.", false)
        .expect("Honest Work should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("Humble Merchant"), "{debug}");
    assert!(debug.contains("Citizen"), "{debug}");
    assert!(debug.contains("RemoveAllAbilities") || debug.contains("LoseAllAbilities") || debug.contains("RemoveAbilities"),
        "the creature loses all abilities: {debug}");
}

#[test]
fn iron_mastiff_rolls_per_attacked_player_and_keeps_the_highest() {
    let definition = compile_to_runtime_definition("Iron Mastiff",
        "Mana cost: {4}\nType: Artifact Creature — Dog\nPower/Toughness: 4/4\nWhenever this creature attacks, roll a d20 for each player being attacked and ignore all but the highest roll.\n1—9 | This creature deals damage equal to its power to you.\n10—19 | This creature deals damage equal to its power to defending player.\n20 | This creature deals damage equal to its power to each opponent.", false)
        .expect("Iron Mastiff should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("RollDiceChooseResultEffect"), "{debug}");
    assert!(debug.contains("PlayersBeingAttacked"), "one die per attacked player: {debug}");
    assert!(debug.contains("sides: 20"), "{debug}");
}

#[test]
fn mercurial_transformation_chooses_frog_or_octopus_package_on_resolution() {
    let definition = compile_to_runtime_definition("Mercurial Transformation",
        "Mana cost: {1}{U}\nType: Sorcery — Lesson\nUntil end of turn, target nonland permanent loses all abilities and becomes your choice of a blue Frog creature with base power and toughness 1/1 or a blue Octopus creature with base power and toughness 4/4.", false)
        .expect("Mercurial Transformation should compile");
    let debug = format!("{:?}", definition.spell_effect);
    assert!(debug.contains("Frog"), "{debug}");
    assert!(debug.contains("Octopus"), "{debug}");
    assert!(debug.contains("SourceChosenOption") || debug.contains("ChosenOption"),
        "the package is the resolution-time choice: {debug}");
}

#[test]
fn gilgamesh_equipment_put_this_way_is_a_reflexive_gate() {
    let definition = compile_to_runtime_definition("Gilgamesh, Master-at-Arms",
        "Mana cost: {4}{R}{R}\nType: Legendary Creature — Human Samurai\nPower/Toughness: 6/6\nWhenever Gilgamesh enters or attacks, look at the top six cards of your library. You may put any number of Equipment cards from among them onto the battlefield. Put the rest on the bottom of your library in a random order. When you put one or more Equipment onto the battlefield this way, you may attach one of them to a Samurai you control.", false)
        .expect("Gilgamesh should compile");
    assert_eq!(definition.abilities.len(), 1, "the reflexive follow-up belongs to the trigger, not a sibling ability");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("ReflexiveTriggerEffect"), "{debug}");
    assert!(debug.contains("action: PutOntoBattlefield"), "{debug}");
    assert!(debug.contains("Samurai"), "{debug}");
}

#[test]
fn hans_eriksson_creature_put_this_way_fights_hans() {
    let definition = compile_to_runtime_definition("Hans Eriksson",
        "Mana cost: {2}{R}{G}\nType: Legendary Creature — Human Scout\nPower/Toughness: 1/4\nWhenever Hans Eriksson attacks, reveal the top card of your library. If it's a creature card, put it onto the battlefield tapped and attacking defending player or a planeswalker they control. Otherwise, put that card into your hand. When you put a creature card onto the battlefield this way, it fights Hans Eriksson.", false)
        .expect("Hans Eriksson should compile");
    assert_eq!(definition.abilities.len(), 1, "{:?}", definition.abilities);
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("ReflexiveTriggerEffect"), "{debug}");
    assert!(debug.contains("action: PutOntoBattlefield"), "{debug}");
    assert!(debug.contains("FightEffect"), "{debug}");
}

#[test]
fn jon_irenicus_goads_the_gift_forever_and_grants_cant_be_sacrificed() {
    let definition = compile_to_runtime_definition("Jon Irenicus, Shattered One",
        "Mana cost: {2}{U}{B}\nType: Legendary Creature — Elf Wizard\nPower/Toughness: 3/3\nAt the beginning of your end step, target opponent gains control of up to one target creature you control. Put two +1/+1 counters on it and tap it. It's goaded for the rest of the game and it gains \"This creature can't be sacrificed.\"\nWhenever a creature you own but don't control attacks, you draw a card.", false)
        .expect("Jon Irenicus should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("Goad"), "{debug}");
    assert!(debug.to_lowercase().contains("sacrificed"), "the gift gains the sacrifice restriction: {debug}");
}

#[test]
fn hold_for_ransom_granted_ability_sacrifices_the_aura_for_its_controller() {
    let definition = compile_to_runtime_definition("Hold for Ransom",
        "Mana cost: {1}{W}\nType: Enchantment — Aura\nEnchant creature\nEnchanted creature can't attack or block and has \"{7}: Hold for Ransom's controller sacrifices it and draws a card. Activate only as a sorcery.\"", false)
        .expect("Hold for Ransom should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("SacrificeTargetEffect"), "{debug}");
    assert!(debug.contains("__granting_source__"),
        "the sacrificed object and the drawing player are the granting Aura and its controller: {debug}");
    assert!(debug.contains("DrawCardsEffect"), "{debug}");
}

#[test]
fn goblin_charbelcher_doubles_damage_when_the_stopping_land_is_a_mountain() {
    let definition = compile_to_runtime_definition("Goblin Charbelcher",
        "Mana cost: {4}\nType: Artifact\n{3}, {T}: Reveal cards from the top of your library until you reveal a land card. This artifact deals damage equal to the number of nonland cards revealed this way to any target. If the revealed land card was a Mountain, this artifact deals double that damage instead. Put the revealed cards on the bottom of your library in any order.", false)
        .expect("Goblin Charbelcher should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("Mountain"), "{debug}");
    assert!(debug.contains("Scaled"), "the replacement arm doubles the same amount: {debug}");
}
