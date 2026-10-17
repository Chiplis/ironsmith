//! Claude package 53 runtime checks for the compilation-completion task
//! (see reports/compilation-completion-20261016/COORDINATION.md).
#![allow(unused_imports)]
use super::compile_to_runtime_definition;
use ironsmith::effect::Effect;

#[test]
fn p53_karplusan_minotaur_opponent_chooses_the_lost_flip_damage_target() {
    let definition = compile_to_runtime_definition(
        "Karplusan Minotaur",
        "Mana cost: {2}{R}{R}\nType: Creature — Minotaur Warrior\nPower/Toughness: 3/3\nCumulative upkeep—Flip a coin. (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)\nWhenever you win a coin flip, this creature deals 1 damage to any target.\nWhenever you lose a coin flip, this creature deals 1 damage to any target of an opponent's choice.",
        false,
    )
    .expect("Karplusan Minotaur should compile");
    let debug = format!("{definition:?}");
    // Exactly one delegated declaration: the lost-flip damage target is
    // announced by an opponent; the won-flip target stays the controller's.
    assert_eq!(debug.matches("chooser: Some(Opponent)").count(), 1, "{debug}");
    assert!(debug.matches("DealDamageEffect").count() >= 2, "{debug}");
}

#[test]
fn p53_keen_duelist_each_loses_the_other_reveals_mana_value() {
    let definition = compile_to_runtime_definition(
        "Keen Duelist",
        "Mana cost: {1}{B}\nType: Creature — Human Wizard\nPower/Toughness: 2/2\nAt the beginning of your upkeep, you and target opponent each reveal the top card of your library. You each lose life equal to the mana value of the card revealed by the other player. You each put the card you revealed into your hand.",
        false,
    )
    .expect("Keen Duelist should compile");
    let debug = format!("{definition:?}");
    let yours = "paired_reveal_yours";
    let theirs = "paired_reveal_theirs";
    assert!(debug.contains(yours) && debug.contains(theirs), "{debug}");
    // Each life loss reads the other participant's revealed card.
    let losses = debug.match_indices("LoseLifeEffect").map(|(at, _)| at).collect::<Vec<_>>();
    assert_eq!(losses.len(), 2, "{debug}");
    let first = &debug[losses[0]..losses[1]];
    let second = &debug[losses[1]..];
    assert!(first.contains(theirs) && !first.contains(yours), "you lose their card's MV: {first}");
    assert!(second.split("MoveToZoneEffect").next().unwrap().contains(yours), "they lose your card's MV: {second}");
    assert!(debug.matches("MoveToZoneEffect").count() >= 2, "both cards go to hand: {debug}");
}

#[test]
fn p53_invasion_of_alara_casts_one_and_keeps_the_other() {
    let definition = compile_to_runtime_definition(
        "Invasion of Alara",
        "Mana cost: {W}{U}{B}{R}{G}\nType: Battle — Siege\nWhen this Siege enters, exile cards from the top of your library until you exile two nonland cards with mana value 4 or less. You may cast one of those two cards without paying its mana cost. Put one of them into your hand. Then put the other cards exiled this way on the bottom of your library in a random order.",
        false,
    )
    .expect("Invasion of Alara should compile");
    let debug = format!("{definition:?}");
    assert!(debug.contains("ConsultTopOfLibraryEffect"), "{debug}");
    assert!(debug.contains("cast_from_consulted") && debug.contains("CastTaggedEffect"), "{debug}");
    // The hand pick is a separate choice among the matched cards still in
    // exile, never the card just cast.
    let kept = debug.find("kept_from_consulted").expect("separate hand choice");
    assert!(debug[kept..].contains("zone: Hand"), "{debug}");
    assert!(debug.contains("PutTaggedRemainderOnLibraryBottomEffect"), "{debug}");
    assert!(debug.contains("Random"), "{debug}");

    let back = compile_to_runtime_definition(
        "Awaken the Maelstrom",
        "Type: Sorcery\nAwaken the Maelstrom is all colors.\nTarget player draws two cards. You may put an artifact card from your hand onto the battlefield. Create a token that's a copy of a permanent you control. Distribute three +1/+1 counters among one, two, or three creatures you control. Destroy target permanent an opponent controls.",
        false,
    )
    .expect("Awaken the Maelstrom should compile");
    let back = format!("{back:?}");
    assert!(back.contains("PutCountersEffect") || back.contains("PutCounters"), "{back}");
}

#[test]
fn p53_invoke_calamity_casts_up_to_two_from_graveyard_or_hand_under_total_cap() {
    let definition = compile_to_runtime_definition(
        "Invoke Calamity",
        "Mana cost: {1}{R}{R}{R}{R}\nType: Instant\nYou may cast up to two instant and/or sorcery spells with total mana value 6 or less from your graveyard and/or hand without paying their mana costs. If those spells would be put into your graveyard, exile them instead. Exile Invoke Calamity.",
        false,
    )
    .expect("Invoke Calamity should compile");
    let debug = format!("{definition:?}");
    assert!(!debug.contains("MayCastMatchingSpellWithoutPayingManaCost"), "{debug}");
    assert!(debug.contains("aggregate_constraint: Some("), "total mana value cap: {debug}");
    assert!(debug.contains("max: Some(2)"), "up to two: {debug}");
    assert!(debug.contains("Graveyard") && debug.contains("Hand"), "both zones: {debug}");
    assert!(debug.contains("CastTaggedEffect"), "{debug}");
    assert!(debug.contains("cast_from_collection"), "{debug}");
    // The exile-instead replacement watches those cast spells.
    let rider = debug.find("RegisterFutureZoneReplacement").or_else(|| debug.find("FutureZoneReplacement"))
        .expect("graveyard exile rider");
    assert!(debug[rider..].contains("cast_from_collection"), "{debug}");
}

#[test]
fn p53_juxtapose_exchanges_greatest_mana_value_creature_then_artifact() {
    let definition = compile_to_runtime_definition(
        "Juxtapose",
        "Mana cost: {3}{U}\nType: Sorcery\nYou and target player exchange control of the creature you each control with the greatest mana value. Then exchange control of artifacts the same way. If two or more permanents a player controls are tied for greatest, their controller chooses one of them.",
        false,
    )
    .expect("Juxtapose should compile");
    let debug = format!("{definition:?}");
    assert_eq!(debug.matches("ExchangeControlEffect").count(), 2, "{debug}");
    assert!(debug.contains("GreatestManaValue"), "{debug}");
    assert!(debug.contains("Creature") && debug.contains("Artifact"), "{debug}");
    assert!(debug.contains("greatest_exchange_yours") && debug.contains("greatest_exchange_theirs"), "{debug}");
}

#[test]
fn p53_jace_architect_of_thought_piles_and_ultimate() {
    let definition = compile_to_runtime_definition(
        "Jace, Architect of Thought",
        "Mana cost: {2}{U}{U}\nType: Legendary Planeswalker — Jace\nLoyalty: 4\n+1: Until your next turn, whenever a creature an opponent controls attacks, it gets -1/-0 until end of turn.\n−2: Reveal the top three cards of your library. An opponent separates those cards into two piles. Put one pile into your hand and the other on the bottom of your library in any order.\n−8: For each player, search that player's library for a nonland card and exile it, then that player shuffles. You may cast those cards without paying their mana costs.",
        false,
    )
    .expect("Jace, Architect of Thought should compile");
    let debug = format!("{definition:?}");
    // −2: an opponent splits; the other pile goes to the library bottom, not
    // the graveyard.
    assert!(debug.contains("chooser: Opponent") || debug.contains("player: Opponent"), "{debug}");
    assert!(debug.contains("PutTaggedRemainderOnLibraryBottomEffect"), "{debug}");
    assert!(debug.contains("ChooserChooses"), "{debug}");
    // −8: any number of the exiled cards may be cast, each in turn.
    let ultimate = debug.find("SearchLibraryEffect").expect("per-player search");
    let tail = &debug[ultimate..];
    assert!(tail.contains("ChooseObjectsEffect") && tail.contains("ForEachTaggedEffect") && tail.contains("CastTaggedEffect"), "{tail}");
}

#[test]
fn p53_indominus_rex_enters_with_keyword_counters_from_discards() {
    let definition = compile_to_runtime_definition(
        "Indominus Rex, Alpha",
        "Mana cost: {1}{U/B}{U/B}{G}{G}\nType: Legendary Creature — Dinosaur Mutant\nPower/Toughness: 6/6\nAs Indominus Rex enters, discard any number of creature cards. It enters with a flying counter on it if a card discarded this way has flying. The same is true for first strike, double strike, deathtouch, hexproof, haste, indestructible, lifelink, menace, reach, trample, and vigilance.\nWhen Indominus Rex enters, draw a card for each counter on it.",
        false,
    )
    .expect("Indominus Rex, Alpha should compile");
    let debug = format!("{definition:?}");
    assert!(debug.contains("AsEntersEffectProgram"), "{debug}");
    assert!(debug.contains("DiscardEffect") || debug.contains("Discard"), "{debug}");
    assert!(debug.contains("discarded_for_entry_counters"), "{debug}");
    // One conditional keyword counter per listed keyword.
    assert_eq!(debug.matches("TaggedObjectMatches(").count(), 12, "{debug}");
    assert!(debug.contains("Flying") && debug.contains("Vigilance"), "{debug}");
}

#[test]
fn p53_invasion_plans_routes_block_declarations_to_the_attacking_player() {
    use ironsmith::ids::PlayerId;
    use ironsmith::zone::Zone;
    let definition = compile_to_runtime_definition(
        "Invasion Plans",
        "Mana cost: {2}{R}\nType: Enchantment\nAll creatures block each combat if able.\nThe attacking player chooses how each creature blocks each combat.",
        false,
    )
    .expect("Invasion Plans should compile");
    let debug = format!("{definition:?}");
    assert!(debug.contains("AttackingPlayerChoosesBlocks"), "{debug}");

    let mut game = ironsmith::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    assert_eq!(game.combat_choice_controller_for_blockers(), None);
    // Bob controls the enchantment; on Alice's turn she declares Bob's blocks.
    game.create_object_from_definition(&definition, bob, Zone::Battlefield);
    game.refresh_continuous_state().unwrap();
    assert_eq!(game.turn.active_player, alice);
    assert_eq!(game.combat_choice_controller_for_blockers(), Some(alice));
    assert_eq!(game.combat_choice_controller_for_attackers(), None);
}
