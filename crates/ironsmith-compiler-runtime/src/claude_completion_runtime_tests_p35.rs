//! Claude package 35 runtime checks for the compilation-completion task
//! (see reports/compilation-completion-20261016/COORDINATION.md).
#![allow(unused_imports)]
use super::compile_to_runtime_definition;
use ironsmith::effect::Effect;

fn p35_compile(name: &str, text: &str) -> ironsmith::cards::CardDefinition {
    let (definition, trace) = ironsmith_compiler::parse_trace::capture(|| {
        compile_to_runtime_definition(name, text, false)
    });
    definition.unwrap_or_else(|error| panic!("{name}: {error:?}\n{}", trace.render()))
}

#[test]
fn kaervek_torch_taxes_spells_targeting_the_source_on_the_stack() {
    let definition = p35_compile("Kaervek's Torch",
        "Mana cost: {X}{R}\nType: Sorcery\nAs long as Kaervek's Torch is on the stack, spells that target it cost {2} more to cast.\nKaervek's Torch deals X damage to any target.");
    let debug = format!("{:?}", definition.abilities);
    let increase = debug.find("CostIncrease").expect("cost increase static");
    let tail = &debug[increase..];
    assert!(tail.contains("targets_object: Some("), "targets an object: {tail}");
    assert!(!tail.contains("__it__"), "'it' is the source, not an unbound tag: {tail}");
    assert!(tail.contains("source: true"), "the targeted object is the source: {tail}");
    assert!(tail.contains("SourceIsInZone(Stack)"), "{tail}");
}

#[test]
fn carthalion_all_colors_reference_predicates_compile() {
    let jenson = p35_compile("Jenson Carthalion, Druid Exile",
        "Mana cost: {G}{W}\nType: Legendary Creature — Human Druid\nPower/Toughness: 2/2\nWhenever you cast a multicolored spell, scry 1. If that spell was all colors, create a 4/4 white Angel creature token with flying and vigilance.\n{5}, {T}: Add {W}{U}{B}{R}{G}.");
    let debug = format!("{:?}", jenson.abilities);
    assert!(debug.contains("all_colors: Some(true)"), "{debug}");
    assert!(debug.contains("Angel"), "{debug}");
    let jared = p35_compile("Jared Carthalion",
        "Mana cost: {W}{U}{B}{R}{G}\nType: Legendary Planeswalker — Jared\nLoyalty: 5\n+1: Create a 3/3 Kavu creature token with trample that's all colors.\n−3: Choose up to two target creatures. For each of them, put a number of +1/+1 counters on it equal to the number of colors it is.\n−6: Return target multicolored card from your graveyard to your hand. If that card was all colors, draw a card and create two Treasure tokens.\nJared Carthalion can be your commander.");
    let debug = format!("{:?}", jared.abilities);
    assert!(debug.contains("all_colors: Some(true)"), "{debug}");
    assert!(debug.contains("Treasure"), "{debug}");
}

#[test]
fn jaws_of_defeat_life_loss_is_power_toughness_difference_of_entering_creature() {
    let definition = p35_compile("Jaws of Defeat",
        "Mana cost: {3}{B}\nType: Enchantment\nWhenever a creature you control enters, target opponent loses life equal to the difference between that creature's power and its toughness.");
    let debug = format!("{:?}", definition.abilities);
    let lose = debug.find("LoseLifeEffect").expect("life loss");
    let tail = &debug[lose..];
    assert!(tail.contains("PowerOf"), "{tail}");
    assert!(tail.contains("ToughnessOf"), "{tail}");
    assert!(tail.contains("Scaled"), "difference subtracts: {tail}");
    assert!(!tail[..tail.find("ToughnessOf").unwrap() + 40].contains("ToughnessOf(Source)"),
        "its toughness is the entering creature's: {tail}");
    let alice = ironsmith::PlayerId::from_index(0);
    let bob = ironsmith::PlayerId::from_index(1);
    let carol = ironsmith::PlayerId::from_index(2);
    struct TargetBob;
    impl ironsmith::decision::DecisionMaker for TargetBob {
        fn decide_targets(&mut self, _: &ironsmith::GameState, ctx: &ironsmith::decisions::context::TargetsContext) -> Vec<ironsmith::game_state::Target> {
            assert_eq!(ctx.player, ironsmith::PlayerId::from_index(0));
            vec![ironsmith::game_state::Target::Player(ironsmith::PlayerId::from_index(1))]
        }
    }
    for (power, toughness) in [(5, 2), (2, 5), (3, 3)] { for owner in [alice, bob] {
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
        game.create_object_from_definition(&definition, alice, ironsmith::Zone::Battlefield);
        let card = ironsmith::card::CardBuilder::new(ironsmith::ids::CardId::new(), "Entering creature witness")
            .card_types(vec![ironsmith::types::CardType::Creature])
            .power_toughness(ironsmith::card::PowerToughness::fixed(power, toughness)).build();
        let entering = game.create_object_from_card(&card, owner, ironsmith::Zone::Hand);
        game.move_object(entering, ironsmith::Zone::Battlefield, ironsmith::events::EventCause::effect());
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        ironsmith::game_loop::drain_pending_trigger_events(&mut game, &mut queue);
        let mut dm = TargetBob;
        ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
        if owner == alice {
            assert_eq!(game.stack.len(), 1);
            ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        } else { assert!(game.stack.is_empty(), "opposing creature does not trigger"); }
        let difference = if owner == alice { (power - toughness).abs() } else { 0 };
        assert_eq!(game.player(bob).unwrap().life, 20 - difference);
        assert_eq!(game.player(alice).unwrap().life, 20);
        assert_eq!(game.player(carol).unwrap().life, 20, "only the chosen opponent loses life");
    } }
}

#[test]
fn inspired_idea_reduces_maximum_hand_size_for_the_rest_of_the_game() {
    let definition = p35_compile("Inspired Idea",
        "Mana cost: {2}{U}\nType: Sorcery\nCleave {3}{U}{U} (You may cast this spell for its cleave cost. If you do, remove the words in square brackets.)\nDraw three cards. [Your maximum hand size is reduced by three for the rest of the game.]");
    let debug = format!("{definition:?}");
    assert!(debug.contains("ReduceMaximumHandSize(You, 3)"), "{debug}");
    assert!(debug.contains("Forever"), "lasting rule: {debug}");
    assert!(debug.contains("DrawCardsEffect"), "{debug}");
}

#[test]
fn jin_sakai_modal_attack_trigger_checks_no_other_attackers_on_that_player() {
    let definition = p35_compile("Jin Sakai, Ghost of Tsushima",
        "Mana cost: {2}{U}{R}\nType: Legendary Creature — Human Samurai\nPower/Toughness: 3/3\nWhenever Jin Sakai deals combat damage to a player, draw a card.\nWhenever a creature you control attacks a player, if no other creatures are attacking that player, choose one —\n• Standoff — It gains double strike until end of turn.\n• Ghost — It can't be blocked this turn.");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("attacking_player_or_planeswalker_controlled_by: Some(Defending)"), "{debug}");
    assert!(debug.contains("attacking_player_only: true"), "{debug}");
    assert!(debug.contains("IsNotTaggedObject"), "the triggering attacker is excluded: {debug}");
    assert!(debug.contains("DoubleStrike") || debug.contains("double strike"), "{debug}");
}

#[test]
fn instill_furor_granted_end_step_sacrifice_checks_the_source_attacked() {
    let definition = p35_compile("Instill Furor",
        "Mana cost: {1}{R}\nType: Enchantment — Aura\nEnchant creature\nEnchanted creature has \"At the beginning of your end step, sacrifice this creature unless it attacked this turn.\"");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("attacked_this_turn: true"), "{debug}");
    assert!(debug.contains("Sacrifice"), "{debug}");
    assert!(!debug.contains("TagKey(\"it\")"), "no dangling it reference: {debug}");
}

#[test]
fn iron_mastiff_rolls_a_die_per_attacked_player_and_keeps_the_highest() {
    let definition = p35_compile("Iron Mastiff",
        "Mana cost: {4}\nType: Artifact Creature — Dog\nPower/Toughness: 4/4\nWhenever this creature attacks, roll a d20 for each player being attacked and ignore all but the highest roll.\n1—9 | This creature deals damage equal to its power to you.\n10—19 | This creature deals damage equal to its power to defending player.\n20 | This creature deals damage equal to its power to each opponent.");
    let debug = format!("{:?}", definition.abilities);
    let roll = debug.find("RollDiceChooseResultEffect").expect("multi-die roll");
    let tail = &debug[roll..(roll + 600).min(debug.len())];
    assert!(tail.contains("ignore_lower: true"), "{tail}");
    assert!(tail.contains("total: false"), "{tail}");
    assert!(tail.contains("PlayersBeingAttacked"), "{tail}");
    assert!(debug.matches("DealDamageEffect").count() >= 3, "three result rows: {debug}");
}

#[test]
fn ironclaw_curse_block_ban_compares_attacker_power_to_enchanted_toughness() {
    let definition = p35_compile("Ironclaw Curse",
        "Mana cost: {R}\nType: Enchantment — Aura\nEnchant creature\nEnchanted creature gets -0/-1.\nEnchanted creature can't block creatures with power equal to or greater than the enchanted creature's toughness.");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("GreaterThanOrEqualExpr(ToughnessOf("), "inclusive dynamic comparison: {debug}");
    assert!(!debug.contains("power: Some(EqualExpr("), "not an equality: {debug}");
}

#[test]
fn jon_irenicus_goads_forever_and_grants_cant_be_sacrificed() {
    let definition = p35_compile("Jon Irenicus, Shattered One",
        "Mana cost: {2}{U}{B}\nType: Legendary Creature — Elf Wizard\nPower/Toughness: 2/2\nAt the beginning of your end step, target opponent gains control of up to one target creature you control. Put two +1/+1 counters on it and tap it. It's goaded for the rest of the game and it gains \"This creature can't be sacrificed.\" (It attacks each combat if able and attacks a player other than you if able.)\nWhenever a creature you own but don't control attacks, you draw a card.");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("Goad"), "{debug}");
    assert!(debug.contains("Forever"), "goaded for the rest of the game: {debug}");
    assert!(debug.contains("BeSacrificed"), "granted can't-be-sacrificed: {debug}");
    assert!(debug.contains("DrawCardsEffect"), "{debug}");
}

#[test]
fn jared_carthalion_kavu_token_is_all_colors() {
    let definition = p35_compile("Jared Carthalion",
        "Mana cost: {W}{U}{B}{R}{G}\nType: Legendary Planeswalker — Jared\nLoyalty: 5\n+1: Create a 3/3 Kavu creature token with trample that's all colors.\n−3: Choose up to two target creatures. For each of them, put a number of +1/+1 counters on it equal to the number of colors it is.\n−6: Return target multicolored card from your graveyard to your hand. If that card was all colors, draw a card and create two Treasure tokens.\nJared Carthalion can be your commander.");
    let debug = format!("{:?}", definition.abilities);
    let kavu = debug.find("Kavu").expect("Kavu token");
    let window = &debug[kavu.saturating_sub(1500)..(kavu + 1500).min(debug.len())];
    assert!(window.contains("ColorSet(31)"),
        "the Kavu token carries all five colors: {window}");
}

#[test]
fn journey_to_the_lost_city_top_roll_returns_exiled_permanents_then_sacrifices_itself() {
    let definition = p35_compile("Journey to the Lost City",
        "Mana cost: {3}{G}\nType: Enchantment\nAt the beginning of your upkeep, exile the top four cards of your library, then roll a d20.\n1—9 | You may put a land card from among those cards onto the battlefield.\n10—19 | Create a 2/2 green Wolf creature token, then put a +1/+1 counter on it for each creature card among those cards.\n20 | Put all permanent cards exiled with this enchantment onto the battlefield, then sacrifice it.");
    let debug = format!("{:?}", definition.abilities);
    let put_all = debug.find("ReturnAllToBattlefieldEffect").expect("mass return");
    let tail = &debug[put_all..];
    let sacrifice = tail.find("Sacrifice").expect("self-sacrifice after the mass return");
    let sacrifice_tail = &tail[sacrifice..(sacrifice + 3000).min(tail.len())];
    assert!(sacrifice_tail.contains("source: true") || sacrifice_tail.contains("Source"),
        "\"it\" is the enchantment: {sacrifice_tail}");
    assert!(!sacrifice_tail[..200.min(sacrifice_tail.len())].contains("__it__"), "{sacrifice_tail}");
}

#[test]
fn jayas_phoenix_copies_the_next_loyalty_ability_activated_this_turn() {
    let definition = p35_compile("Jaya's Phoenix",
        "Mana cost: {4}{R}{R}\nType: Creature — Phoenix\nPower/Toughness: 4/3\nFlying, haste\nWhenever this creature deals combat damage to a player or planeswalker, copy the next loyalty ability you activate this turn when you activate it. You may choose new targets for the copy.\nWhenever you cast a planeswalker spell, you may return this card from your graveyard to the battlefield.");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("loyalty_only: true"), "loyalty-ability activation trigger: {debug}");
    assert!(debug.contains("one_shot: true") || debug.contains("OneShot"), "only the next one: {debug}");
    assert!(debug.contains("CopySpell"), "{debug}");
}
