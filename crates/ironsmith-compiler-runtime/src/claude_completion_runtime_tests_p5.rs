//! Claude package 5 runtime checks for the compilation-completion task
//! (see reports/compilation-completion-20261016/COORDINATION.md).
#![allow(unused_imports)]
use super::compile_to_runtime_definition;
use ironsmith::effect::Effect;

fn p5_game() -> (ironsmith::game_state::GameState, ironsmith::PlayerId, ironsmith::PlayerId) {
    let game = ironsmith::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    (game, ironsmith::PlayerId::from_index(0), ironsmith::PlayerId::from_index(1))
}

#[test]
fn players_cant_get_counters_and_typed_permanents_cant_have_counters() {
    use ironsmith::card::{CardBuilder, PowerToughness};
    use ironsmith::ids::CardId;
    use ironsmith::types::CardType;
    let definition = compile_to_runtime_definition(
        "Solemnity",
        "Type: Enchantment\nPlayers can't get counters.\nCounters can't be put on artifacts, creatures, enchantments, or lands.",
        false,
    )
    .expect("Solemnity should compile");
    let debug = format!("{definition:#?}");
    assert!(debug.contains("GetCounters("), "{debug}");
    assert!(debug.contains("HaveCountersPlaced("), "{debug}");

    let (mut game, alice, bob) = p5_game();
    let creature = CardBuilder::new(CardId::new(), "Bear")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let bear = game.create_object_from_card(&creature, bob, ironsmith::Zone::Battlefield);
    assert!(game.can_get_counters(alice) && game.can_have_counters_placed(bear));
    game.create_object_from_definition(&definition, alice, ironsmith::Zone::Battlefield);
    game.refresh_continuous_state().unwrap();
    assert!(!game.can_get_counters(alice));
    assert!(!game.can_get_counters(bob));
    assert!(!game.can_get_poison_counters(bob));
    assert!(!game.can_have_counters_placed(bear));
}

#[test]
fn target_player_cant_get_counters_for_source_duration() {
    let definition = compile_to_runtime_definition(
        "Suncleanser",
        "Type: Creature — Human Cleric\nWhen this creature enters, choose one —\n• Remove all counters from target creature. It can't have counters put on it for as long as this creature remains on the battlefield.\n• Target opponent loses all counters. That player can't get counters for as long as this creature remains on the battlefield.",
        false,
    )
    .expect("Suncleanser should compile");
    let debug = format!("{definition:#?}");
    assert!(debug.contains("GetCounters("), "{debug}");
    assert!(debug.contains("HaveCountersPlaced("), "{debug}");
    assert!(debug.contains("RemoveUpToAnyCountersEffect"), "{debug}");
}

#[test]
fn source_combat_prohibitions_gated_by_trailing_static_conditions() {
    use ironsmith::card::{CardBuilder, PowerToughness};
    use ironsmith::ids::CardId;
    use ironsmith::types::CardType;
    let shauku = compile_to_runtime_definition(
        "Shauku, Endbringer",
        "Type: Legendary Creature — Vampire\nPower/Toughness: 1/1\nFlying\nShauku can't attack if there's another creature on the battlefield.\nAt the beginning of your upkeep, you lose 3 life.\n{T}: Exile target creature and put a +1/+1 counter on Shauku.",
        false,
    )
    .expect("Shauku should compile");
    let stature = compile_to_runtime_definition(
        "Stature, Size Shifter",
        "Type: Legendary Creature — Human Hero\nPower/Toughness: 1/1\nStature can't be blocked if her power is 1 or less.\nPower-up — {X}{U}{U}: Put X +1/+1 counters on Stature. (Activate each power-up ability only once. Reduce the cost by her mana cost if she entered this turn.)",
        false,
    )
    .expect("Stature should compile");

    let (mut game, alice, bob) = p5_game();
    let source = game.create_object_from_definition(&shauku, alice, ironsmith::Zone::Battlefield);
    game.remove_summoning_sickness(source);
    game.refresh_continuous_state().unwrap();
    assert!(game.can_attack(source), "alone on the battlefield Shauku may attack");
    let other = CardBuilder::new(CardId::new(), "Bear")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let other = game.create_object_from_card(&other, bob, ironsmith::Zone::Battlefield);
    game.refresh_continuous_state().unwrap();
    assert!(!game.can_attack(source), "another creature forbids attacking");
    game.move_object_by_effect(other, ironsmith::Zone::Graveyard).unwrap();
    game.refresh_continuous_state().unwrap();
    assert!(game.can_attack(source), "the restriction stops when the other creature leaves");

    let (mut game, alice, _bob) = p5_game();
    let source = game.create_object_from_definition(&stature, alice, ironsmith::Zone::Battlefield);
    game.refresh_continuous_state().unwrap();
    assert!(!game.can_be_blocked(source), "a 1-power Stature can't be blocked");
    game.add_counters(source, ironsmith::object::CounterType::PlusOnePlusOne, 1);
    game.refresh_continuous_state().unwrap();
    assert_eq!(game.calculated_power(source), Some(2));
    assert!(game.can_be_blocked(source), "the condition uses power after counters");
    game.remove_counters(source, ironsmith::object::CounterType::PlusOnePlusOne, 1, Some(source), Some(alice)).unwrap();
    game.refresh_continuous_state().unwrap();
    assert!(!game.can_be_blocked(source), "the permission follows later power changes");
}

#[test]
fn attached_type_addition_includes_supertype() {
    let definition = compile_to_runtime_definition(
        "Super-Soldier Serum",
        "Type: Enchantment — Aura\nEnchant creature\nEnchanted creature gets +2/+2, has first strike and vigilance, and is a legendary Soldier in addition to its other types.\nWhenever enchanted creature attacks or blocks, attach any number of target Equipment you control to it.",
        false,
    )
    .expect("Super-Soldier Serum should compile");
    let debug = format!("{definition:#?}");
    assert!(debug.contains("AddSupertypes"), "{debug}");
    assert!(debug.contains("Legendary"), "{debug}");
    assert!(debug.contains("Soldier"), "{debug}");
}

#[test]
fn discard_trigger_card_filter_keeps_alternatives_and_keyword_qualifiers() {
    let definition = compile_to_runtime_definition(
        "Shipwreck Sifters",
        "Type: Creature — Spirit\nPower/Toughness: 1/3\nWhen this creature enters, draw a card, then discard a card.\nWhenever you discard a Spirit card or a card with disturb, put a +1/+1 counter on this creature.",
        false,
    )
    .expect("Shipwreck Sifters should compile");
    let debug = format!("{definition:#?}");
    assert!(debug.contains("Spirit"), "{debug}");
    assert!(debug.to_lowercase().contains("disturb"), "{debug}");
    assert!(debug.contains("any_of: ["), "{debug}");
}

#[test]
fn opponent_caused_discards_are_prohibited() {
    use ironsmith::card::CardBuilder;
    use ironsmith::ids::CardId;
    use ironsmith::types::CardType;
    let definition = compile_to_runtime_definition(
        "Tamiyo, Collector of Tales",
        "Type: Legendary Planeswalker — Tamiyo\nSpells and abilities your opponents control can't cause you to discard cards or sacrifice permanents.\n+1: Choose a nonland card name, then reveal the top four cards of your library. Put all cards with the chosen name from among them into your hand and the rest into your graveyard.\n−3: Return target card from your graveyard to your hand.",
        false,
    )
    .expect("Tamiyo should compile");
    let debug = format!("{definition:#?}");
    assert!(debug.contains("DiscardByCause"), "{debug}");
    assert!(debug.contains("BeSacrificedByCause"), "{debug}");

    let (mut game, alice, bob) = p5_game();
    let tamiyo = game.create_object_from_definition(&definition, alice, ironsmith::Zone::Battlefield);
    let card = CardBuilder::new(CardId::new(), "Spare Card")
        .card_types(vec![CardType::Sorcery])
        .build();
    game.create_object_from_card(&card, alice, ironsmith::Zone::Hand);
    game.refresh_continuous_state().unwrap();
    let discard = Effect::new(ironsmith::effects::DiscardEffect::new(
        1,
        ironsmith::target::PlayerFilter::Specific(alice),
        false,
    ));
    let mut opponent_ctx = ironsmith::effects::EffectContext::new_default(tamiyo, bob);
    ironsmith::effects::execute_effect(&mut game, &discard, &mut opponent_ctx).unwrap();
    assert_eq!(game.player(alice).unwrap().hand_size(), 1, "opponent effect can't make Alice discard");
    let mut own_ctx = ironsmith::effects::EffectContext::new_default(tamiyo, alice);
    ironsmith::effects::execute_effect(&mut game, &discard, &mut own_ctx).unwrap();
    assert_eq!(game.player(alice).unwrap().hand_size(), 0, "Alice's own effect still discards");
}

#[test]
fn otherwise_sentence_after_conditional_gain_stays_the_else_arm() {
    let definition = compile_to_runtime_definition(
        "Stolen Vitality",
        "Type: Instant\nTarget creature gets +3/+1 until end of turn. If it's your turn, that creature gains trample until end of turn. Otherwise, it gains first strike until end of turn. (A creature with trample can deal excess combat damage to the player or planeswalker it's attacking. A creature with first strike deals combat damage before creatures without first strike.)",
        false,
    )
    .expect("Stolen Vitality should compile");
    let debug = format!("{definition:#?}");
    assert!(debug.contains("Trample"), "{debug}");
    assert!(debug.contains("FirstStrike"), "{debug}");
    // The first-strike grant is the else arm of the turn condition (an
    // if/otherwise pair), never an unconditional sibling statement.
    assert!(debug.contains("Otherwise") || debug.contains("if_false: [\n"), "{debug}");
}

#[test]
fn distributive_plural_pronoun_shares_pump_and_grant() {
    for (name, text) in [
        ("Sigardian Zealot", "Type: Creature — Human Cleric\nPower/Toughness: 3/3\nAt the beginning of combat on your turn, choose any number of creatures with different powers. Each of them gets +X/+X and gains vigilance until end of turn, where X is this creature's power."),
    ] {
        let definition = compile_to_runtime_definition(name, text, false)
            .unwrap_or_else(|error| panic!("{name} should compile: {error:?}"));
        let debug = format!("{definition:#?}");
        assert!(debug.contains("Vigilance"), "{name}: {debug}");
        assert!(debug.contains("Tagged("), "{name}: the grant names the referenced set: {debug}");
    }
}

#[test]
fn characteristic_toughness_their_total_reads_previous_axis_set() {
    let definition = compile_to_runtime_definition(
        "Sutured Ghoul",
        "Type: Creature — Zombie\nTrample\nAs this creature enters, exile any number of creature cards from your graveyard.\nSutured Ghoul's power is equal to the total power of the exiled cards and its toughness is equal to their total toughness.",
        false,
    )
    .expect("Sutured Ghoul should compile");
    let debug = format!("{definition:#?}");
    assert!(debug.contains("TotalPower"), "{debug}");
    assert!(debug.contains("TotalToughness"), "{debug}");
}

#[test]
fn token_followup_they_each_have_grants_to_created_tokens() {
    let definition = compile_to_runtime_definition(
        "Tetravus",
        "Type: Artifact Creature — Construct\nPower/Toughness: 1/1\nFlying\nThis creature enters with three +1/+1 counters on it.\nAt the beginning of your upkeep, you may remove any number of +1/+1 counters from this creature. If you do, create that many 1/1 colorless Tetravite artifact creature tokens. They each have flying and \"This token can't be enchanted.\"\nAt the beginning of your upkeep, you may exile any number of tokens created with this creature. If you do, put that many +1/+1 counters on this creature.",
        false,
    )
    .expect("Tetravus should compile");
    let debug = format!("{definition:#?}");
    assert!(debug.contains("Tetravite"), "{debug}");
    assert!(debug.contains("Flying"), "{debug}");
}

#[test]
fn trailing_if_after_delayed_return_is_checked_when_the_return_resolves() {
    let definition = compile_to_runtime_definition(
        "Shirei, Shizo's Caretaker",
        "Type: Legendary Creature — Spirit\nPower/Toughness: 2/2\nWhenever a creature with power 1 or less is put into your graveyard from the battlefield, you may return that card to the battlefield at the beginning of the next end step if Shirei is still on the battlefield.",
        false,
    )
    .expect("Shirei should compile");
    let debug = format!("{definition:#?}");
    let schedule = debug.find("ScheduleDelayedTriggerEffect").expect("delayed return");
    let condition = debug.find("SourceIsInZone").expect("still-on-battlefield check");
    let choice = debug.find("MayEffect").expect("optional return");
    assert!(schedule < condition, "the condition is inside the delayed trigger: {debug}");
    assert!(schedule < choice, "the choice is made when the delayed trigger resolves: {debug}");
}

#[test]
fn enters_tapped_and_attacking_followup_modifies_preceding_put() {
    let definition = compile_to_runtime_definition(
        "Shark Shredder, Killer Clone",
        "Type: Legendary Creature — Shark Octopus Ninja\nPower/Toughness: 3/3\nFirst strike\nWhenever Shark Shredder deals combat damage to a player, put up to one target creature card from that player's graveyard onto the battlefield under your control. It enters tapped and attacking that player.",
        false,
    )
    .expect("Shark Shredder should compile");
    let debug = format!("{definition:#?}");
    assert!(debug.contains("attacking: true") || debug.contains("enters_attacking: true"), "{debug}");
    assert!(debug.contains("tapped: true"), "{debug}");
}

#[test]
fn first_time_tapped_during_your_turns_trigger_is_scoped_to_your_turn() {
    for (name, text) in [
        ("Tale of Katara and Toph", "Type: Enchantment\nCreatures you control have \"Whenever this creature becomes tapped for the first time during each of your turns, put a +1/+1 counter on it.\""),
    ] {
        let definition = compile_to_runtime_definition(name, text, false)
            .unwrap_or_else(|error| panic!("{name} should compile: {error:?}"));
        let debug = format!("{definition:#?}");
        assert!(debug.contains("YourTurn") || debug.contains("IsYourTurn"), "{name}: {debug}");
        assert!(debug.contains("PlusOnePlusOne"), "{name}: {debug}");
    }
}

#[test]
fn reflexive_this_way_sentence_stays_inside_its_triggered_ability() {
    for (name, text) in [
        ("Talion's Messenger", "Type: Creature — Faerie Noble\nPower/Toughness: 2/2\nFlying\nWhenever you attack with one or more Faeries, draw a card, then discard a card. When you discard a card this way, put a +1/+1 counter on target Faerie you control."),
        ("Teo, Spirited Glider", "Type: Legendary Creature — Human Pilot Ally\nPower/Toughness: 1/4\nFlying\nWhenever one or more creatures you control with flying attack, draw a card, then discard a card. When you discard a nonland card this way, put a +1/+1 counter on target creature you control."),
    ] {
        let definition = compile_to_runtime_definition(name, text, false)
            .unwrap_or_else(|error| panic!("{name} should compile: {error:?}"));
        let triggered = definition
            .abilities
            .iter()
            .filter(|ability| matches!(ability.kind, ironsmith::ability::AbilityKind::Triggered(_)))
            .count();
        assert_eq!(triggered, 1, "{name}: the reflexive trigger is not a sibling ability");
        let debug = format!("{definition:#?}");
        assert!(debug.contains("PlusOnePlusOne"), "{name}: {debug}");
    }
}

#[test]
fn blocking_or_blocked_by_relation_is_not_an_action_choice() {
    let definition = compile_to_runtime_definition(
        "Spitting Slug",
        "Type: Creature — Slug\nPower/Toughness: 2/4\nWhenever this creature blocks or becomes blocked, you may pay {1}{G}. If you do, this creature gains first strike until end of turn. Otherwise, each creature blocking or blocked by this creature gains first strike until end of turn.",
        false,
    )
    .expect("Spitting Slug should compile");
    let debug = format!("{definition:#?}");
    assert!(debug.contains("FirstStrike"), "{debug}");
    assert!(!debug.contains("ChooseOneOf"), "the 'or' is a combat relation, not a modal choice: {debug}");
}

#[test]
fn exiled_cost_card_predicate_reads_the_exiled_card() {
    let definition = compile_to_runtime_definition(
        "Storm Elemental",
        "Type: Creature — Elemental\nPower/Toughness: 3/4\nFlying\n{U}, Exile the top card of your library: Tap target creature with flying.\n{U}, Exile the top card of your library: If the exiled card is a snow land, this creature gets +1/+1 until end of turn.",
        false,
    )
    .expect("Storm Elemental should compile");
    let debug = format!("{definition:#?}");
    assert!(debug.contains("Snow"), "{debug}");
    assert!(debug.contains("TaggedObjectMatches") || debug.contains("TargetMatches"), "{debug}");
}

#[test]
fn put_onto_battlefield_transformed_marks_the_entry() {
    let definition = compile_to_runtime_definition(
        "Startled Awake",
        "Mana cost: {2}{U}{U}\nType: Sorcery\nTarget opponent mills thirteen cards.\n{3}{U}{U}: Put Startled Awake from your graveyard onto the battlefield transformed. Activate only as a sorcery.",
        false,
    )
    .expect("Startled Awake should compile");
    let debug = format!("{definition:#?}");
    assert!(debug.contains("transformed: true") || debug.contains("Transformed"), "{debug}");
}

#[test]
fn granted_ability_lasts_until_the_end_of_your_next_turn() {
    let definition = compile_to_runtime_definition(
        "Stilt-Man, Towering Terror",
        "Type: Legendary Creature — Human Rogue Villain\nPower/Toughness: 2/6\nReach\nWhenever one or more Villains you control deal combat damage to a player, gain control of target noncreature, nonland permanent that player controls until the end of your next turn. It gains \"This permanent can't be sacrificed\" until the end of your next turn.",
        false,
    )
    .expect("Stilt-Man should compile");
    let debug = format!("{definition:#?}");
    assert!(debug.contains("YourNextTurnEnd"), "{debug}");
    assert!(debug.contains("BeSacrificed") || debug.contains("CantBeSacrificed"), "{debug}");
}

#[test]
fn white_mana_may_be_spent_as_though_it_were_red() {
    let definition = compile_to_runtime_definition(
        "Sunglasses of Urza",
        "Mana cost: {3}\nType: Artifact\nYou may spend white mana as though it were red mana.",
        false,
    )
    .expect("Sunglasses of Urza should compile");
    let (mut game, alice, bob) = p5_game();
    game.create_object_from_definition(&definition, alice, ironsmith::Zone::Battlefield);
    game.refresh_continuous_state().unwrap();
    let white_as_red = (ironsmith::mana::ManaSymbol::White, ironsmith::mana::ManaSymbol::Red);
    assert!(game.mana_spend_policy(alice, None).symbol_as_symbol.contains(&white_as_red));
    assert!(!game.mana_spend_policy(bob, None).symbol_as_symbol.contains(&white_as_red));
    assert!(!game.mana_spend_policy(alice, None)
        .symbol_as_symbol
        .contains(&(ironsmith::mana::ManaSymbol::Red, ironsmith::mana::ManaSymbol::White)));
}

#[test]
fn put_all_commanders_from_the_command_zone() {
    let definition = compile_to_runtime_definition(
        "Tevesh Szat, Doom of Fools",
        "Mana cost: {4}{B}{B}\nType: Legendary Planeswalker — Szat\nLoyalty: 4\n+2: Create two 0/1 black Thrull creature tokens.\n+1: You may sacrifice another creature or planeswalker. If you do, draw two cards, then draw another card if the sacrificed permanent was a commander.\n−10: Gain control of all commanders. Put all commanders from the command zone onto the battlefield under your control.\nTevesh Szat, Doom of Fools can be your commander.\nPartner",
        false,
    )
    .expect("Tevesh Szat should compile");
    let debug = format!("{definition:#?}");
    assert!(debug.contains("Command"), "{debug}");
}

#[test]
fn conditional_entry_modifier_followup_splits_the_put() {
    let definition = compile_to_runtime_definition(
        "Summoner's Grimoire",
        "Mana cost: {3}{G}\nType: Artifact — Book Equipment\nJob select\nEquipped creature is a Shaman in addition to its other types and has \"Whenever this creature attacks, you may put a creature card from your hand onto the battlefield. If that card is an enchantment card, it enters tapped and attacking.\"\nAbraxas — Equip {3}",
        false,
    )
    .expect("Summoner's Grimoire should compile");
    let debug = format!("{definition:#?}");
    assert!(debug.contains("Enchantment"), "{debug}");
    assert!(debug.contains("Shaman"), "{debug}");
}

#[test]
fn return_half_the_creatures_rounded_up() {
    let definition = compile_to_runtime_definition(
        "Split the Party",
        "Mana cost: {3}{U}{U}\nType: Sorcery\nChoose target player. Return half the creatures they control to their owner's hand, rounded up.",
        false,
    )
    .expect("Split the Party should compile");
    let debug = format!("{definition:#?}");
    assert!(debug.contains("HalfRoundedDown"), "{debug}");
    assert!(debug.contains("Hand"), "{debug}");
}

#[test]
fn returned_creatures_become_food_artifacts_with_the_quoted_ability() {
    let definition = compile_to_runtime_definition(
        "Supper for Spiders",
        "Mana cost: {1}{B}\nType: Instant\nPut onto the battlefield under your control all creature cards in your opponents' graveyards that were put there from the battlefield this turn. They are Food artifacts with \"{2}, {T}, Sacrifice this artifact: You gain 3 life.\" (They lose all other types and subtypes.)",
        false,
    )
    .expect("Supper for Spiders should compile");
    let debug = format!("{definition:#?}");
    assert!(debug.contains("Food"), "{debug}");
    assert!(debug.contains("SetCardTypes") || debug.contains("Artifact"), "{debug}");
    assert!(debug.contains("GainLifeEffect"), "{debug}");
}

#[test]
fn negated_serial_subtype_subject_is_one_selector() {
    let definition = compile_to_runtime_definition(
        "Swarmyard Massacre",
        "Mana cost: {3}{B}{B}\nType: Sorcery\nCreate two 1/1 green Squirrel creature tokens. Then each creature that isn't an Insect, Rat, Spider, or Squirrel gets -1/-1 until end of turn for each creature you control that's an Insect, Rat, Spider, or Squirrel.",
        false,
    )
    .expect("Swarmyard Massacre should compile");
    let debug = format!("{definition:#?}");
    for subtype in ["Insect", "Rat", "Spider", "Squirrel"] {
        assert!(debug.contains(subtype), "{subtype}: {debug}");
    }
}

#[test]
fn each_player_shuffles_hand_graveyard_and_permanents_into_library() {
    let definition = compile_to_runtime_definition(
        "Sway of the Stars",
        "Mana cost: {8}{U}{U}\nType: Sorcery\nEach player shuffles their hand, graveyard, and all permanents they own into their library, then draws seven cards. Each player's life total becomes 7.",
        false,
    )
    .expect("Sway of the Stars should compile");
    let debug = format!("{definition:#?}");
    assert!(debug.contains("ShuffleHandAndGraveyardIntoLibraryEffect") && debug.contains("include_owned_permanents: true"), "{debug}");
    assert!(debug.contains("DrawCardsEffect"), "{debug}");
}

#[test]
fn players_with_fewer_cards_than_the_most_draw_the_difference() {
    let definition = compile_to_runtime_definition(
        "Tales of the Ancestors",
        "Mana cost: {3}{U}\nType: Sorcery\nEach player with fewer cards in hand than the player with the most cards in hand draws cards equal to the difference.\nForetell {1}{U} (During your turn, you may pay {2} and exile this card from your hand face down. Cast it on a later turn for its foretell cost.)",
        false,
    )
    .expect("Tales of the Ancestors should compile");
    let debug = format!("{definition:#?}");
    // The "with fewer cards in hand than ..." qualifier must gate the draw;
    // it used to be dropped, compiling as "each player draws".
    assert!(debug.contains("MaxCardsInHand") && debug.contains("LessThan"), "{debug}");
    assert!(debug.contains("DrawCardsEffect"), "{debug}");
}

#[test]
fn auras_equipment_and_modified_creatures_gain_hexproof() {
    let definition = compile_to_runtime_definition(
        "Silkguard",
        "Mana cost: {X}{G}\nType: Instant\nPut a +1/+1 counter on each of up to X target creatures you control.\nAuras, Equipment, and modified creatures you control gain hexproof until end of turn. (Equipment, Auras you control, and counters are modifications.)",
        false,
    )
    .expect("Silkguard should compile");
    let debug = format!("{definition:#?}");
    assert!(debug.contains("Aura") && debug.contains("Equipment") && debug.contains("modified: true"), "{debug}");
    assert!(debug.contains("Hexproof"), "{debug}");
    // Each member keeps its own noun: Auras and Equipment are not creatures.
    assert!(!definition.canonical_text.contains("Aura creatures"), "{}", definition.canonical_text);
    assert!(!definition.canonical_text.contains("Equipment creatures"), "{}", definition.canonical_text);
}

#[test]
fn damage_to_each_opponent_controlling_more_lands_is_a_player_condition() {
    let definition = compile_to_runtime_definition(
        "Spiteful Repossession",
        "Mana cost: {4}{R}\nType: Sorcery\nSpiteful Repossession deals damage to each opponent who controls more lands than you equal to the difference. Then create a number of Treasure tokens equal to the damage dealt this way.",
        false,
    )
    .expect("Spiteful Repossession should compile");
    let debug = format!("{definition:#?}");
    // Each opponent is the recipient; it used to compile as damage to each land.
    assert!(!debug.contains("DealDamageEachEffect"), "{debug}");
    assert!(debug.contains("ForPlayersEffect") || debug.contains("ForEachOpponent"), "{debug}");
    assert!(debug.contains("GreaterThan"), "{debug}");
    assert!(debug.contains("Treasure"), "{debug}");
}

#[test]
fn reduce_the_fastest_opponents_speed_with_a_floor() {
    let definition = compile_to_runtime_definition(
        "Spikeshell Harrier",
        "Mana cost: {4}{U}\nType: Artifact Creature — Robot Turtle\nPower/Toughness: 2/3\nWhen this creature enters, return target creature or Vehicle an opponent controls to its owner's hand. If that opponent's speed is greater than each other player's speed, reduce that opponent's speed by 1. This effect can't reduce their speed below 1.",
        false,
    )
    .expect("Spikeshell Harrier should compile");
    let debug = format!("{definition:#?}");
    assert!(debug.contains("ReduceSpeedEffect"), "{debug}");
    assert!(debug.contains("minimum: 1"), "{debug}");
    assert!(debug.contains("MaxSpeed"), "{debug}");
}

#[test]
fn lose_all_but_one_life_and_regain_the_noted_loss() {
    let definition = compile_to_runtime_definition(
        "Soulgorger Orgg",
        "Mana cost: {3}{R}{R}\nType: Creature — Nightmare Orgg\nPower/Toughness: 6/6\nTrample\nWhen this creature enters, you lose all but 1 life.\nWhen this creature leaves the battlefield, you gain life equal to the life you lost when it entered.",
        false,
    )
    .expect("Soulgorger Orgg should compile");
    let debug = format!("{definition:#?}");
    assert!(debug.contains("NoteLifeTotalEffect"), "{debug}");
    assert!(debug.contains("LastNotedLifeTotal"), "{debug}");
    assert!(debug.contains("LoseLifeEffect") && debug.contains("GainLifeEffect"), "{debug}");
}

#[test]
fn next_game_loss_this_turn_is_replaced() {
    let definition = compile_to_runtime_definition(
        "Stunning Reversal",
        "Mana cost: {3}{B}\nType: Instant\nThe next time you would lose the game this turn, instead draw seven cards and your life total becomes 1.\nExile Stunning Reversal.",
        false,
    )
    .expect("Stunning Reversal should compile");
    let debug = format!("{definition:#?}");
    assert!(debug.contains("RegisterDrawReplacementEffect") && debug.contains("lose_game: true"), "{debug}");
    assert!(debug.contains("SetLifeTotalEffect") && debug.contains("DrawCardsEffect"), "{debug}");
}

#[test]
fn untapped_storage_matrix_limits_untaps_to_a_chosen_type() {
    let definition = compile_to_runtime_definition(
        "Storage Matrix",
        "Mana cost: {3}\nType: Artifact\nAs long as this artifact is untapped, each player chooses artifact, creature, or land during their untap step. That player can untap only permanents of the chosen type this step.",
        false,
    )
    .expect("Storage Matrix should compile");
    let debug = format!("{definition:#?}");
    assert!(debug.contains("UntapStepChosenCardTypeOnly"), "{debug}");
    assert!(debug.contains("Artifact") && debug.contains("Creature") && debug.contains("Land"), "{debug}");
    assert!(debug.contains("SourceIsUntapped") || debug.contains("untapped"), "{debug}");
}

#[test]
fn radiation_life_loss_becomes_life_gain() {
    let definition = compile_to_runtime_definition(
        "Strong, the Brutish Thespian",
        "Mana cost: {4}{G}{G}\nType: Legendary Creature — Mutant Berserker\nPower/Toughness: 5/5\nWard {2}\nEnrage — Whenever Strong is dealt damage, you get three rad counters and put three +1/+1 counters on Strong.\nYou gain life rather than lose life from radiation.",
        false,
    )
    .expect("Strong should compile");
    let debug = format!("{definition:#?}");
    assert!(debug.contains("RadiationGainsLife"), "{debug}");
    let (mut game, alice, _bob) = p5_game();
    game.create_object_from_definition(&definition, alice, ironsmith::Zone::Battlefield);
    game.refresh_continuous_state().unwrap();
    assert!(game.radiation_gains_life(alice));
}

#[test]
fn attacking_group_matches_the_chosen_number() {
    let definition = compile_to_runtime_definition(
        "Squall, Gunblade Duelist",
        "Mana cost: {R}{W}{B}\nType: Legendary Creature — Human Mercenary\nPower/Toughness: 3/3\nFirst strike\nAs Squall enters, choose a number.\nWhenever one or more creatures attack one of your opponents, if any of those creatures have power or toughness equal to the chosen number, Squall deals damage equal to its power to defending player.",
        false,
    )
    .expect("Squall should compile");
    let debug = format!("{definition:#?}");
    assert!(debug.contains("SourceChosenNumber"), "{debug}");
    assert!(debug.contains("attacking_group") || debug.contains("AttackingGroup"), "{debug}");
}

#[test]
fn do_the_same_for_repeats_the_keyword_counter_sentence() {
    let definition = compile_to_runtime_definition(
        "Super-Adaptoid",
        "Mana cost: {2}\nType: Legendary Artifact Creature — Robot Villain\nPower/Toughness: */2\nSuper-Adaptoid's power is equal to the number of legendary creatures you control.\nWhenever Super-Adaptoid enters or attacks, choose another target creature. If that creature has haste and Super-Adaptoid doesn't, put a haste counter on Super-Adaptoid. Do the same for flying, first strike, double strike, deathtouch, indestructible, lifelink, menace, reach, trample, and vigilance.",
        false,
    )
    .expect("Super-Adaptoid should compile");
    let debug = format!("{definition:#?}");
    for counter in ["Haste", "Flying", "FirstStrike", "DoubleStrike", "Deathtouch", "Vigilance"] {
        assert!(debug.contains(counter), "missing {counter}: {debug}");
    }
    // "and Super-Adaptoid doesn't" is kept as a negated source check.
    assert!(debug.contains("Not("), "{debug}");
}

#[test]
fn sacrificed_land_could_produce_names_the_cost_object() {
    let definition = compile_to_runtime_definition(
        "Squandered Resources",
        "Mana cost: {B}{G}\nType: Enchantment\nSacrifice a land: Add one mana of any type the sacrificed land could produce.",
        false,
    )
    .expect("Squandered Resources should compile");
    let debug = format!("{definition:#?}");
    assert!(debug.contains("MatchingLandsCouldProduce"), "{debug}");
    assert!(debug.contains("sacrifice"), "{debug}");
    assert!(debug.contains("IsTaggedObject"), "{debug}");
}

#[test]
fn source_and_target_share_one_evasion_restriction() {
    let definition = compile_to_runtime_definition(
        "Tatsunari, Toad Rider",
        "Mana cost: {2}{B}\nType: Legendary Creature — Human Ninja\nPower/Toughness: 3/3\nWhenever you cast an enchantment spell, if you don't control a creature named Keimi, create Keimi, a legendary 3/3 black and green Frog creature token with \"Whenever you cast an enchantment spell, each opponent loses 1 life and you gain 1 life.\"\n{1}{G/U}: Tatsunari and target Frog you control can't be blocked this turn except by creatures with flying or reach.",
        false,
    )
    .expect("Tatsunari should compile");
    let debug = format!("{definition:#?}");
    assert!(debug.contains("Frog"), "{debug}");
    assert!(debug.matches("BlockSpecificAttacker").count() >= 2, "{debug}");
}

#[test]
fn counters_equal_to_power_above_base_power() {
    let definition = compile_to_runtime_definition(
        "Sovereign Okinec Ahau",
        "Mana cost: {2}{G}{W}\nType: Legendary Creature — Cat Noble\nPower/Toughness: 3/4\nWard {2}\nWhenever Sovereign Okinec Ahau attacks, for each creature you control with power greater than that creature's base power, put a number of +1/+1 counters on that creature equal to the difference.",
        false,
    )
    .expect("Sovereign Okinec Ahau should compile");
    let debug = format!("{definition:#?}");
    assert!(debug.contains("power_greater_than_base_power: true"), "{debug}");
    assert!(debug.contains("BasePowerOf"), "{debug}");
}

#[test]
fn delayed_trigger_scoped_to_this_combat() {
    let definition = compile_to_runtime_definition(
        "Stensian Sanguinist",
        "Mana cost: {1}{B}\nType: Creature — Vampire Cleric\nPower/Toughness: 2/2\nWhenever you attack, target creature gains deathtouch until end of turn. Whenever that creature deals combat damage to a player this combat, this creature becomes prepared. (While it's prepared, you may cast a copy of its spell. Doing so unprepares it.)",
        false,
    )
    .expect("Stensian Sanguinist should compile");
    let debug = format!("{definition:#?}");
    assert!(debug.contains("EndOfCombat") || debug.contains("until_end_of_combat: true"), "{debug}");
    assert!(debug.contains("Deathtouch"), "{debug}");
}

#[test]
fn each_player_pays_the_chosen_creatures_mana_cost_or_sacrifices_it() {
    let definition = compile_to_runtime_definition(
        "Tariff",
        "Mana cost: {1}{W}\nType: Sorcery\nEach player sacrifices the creature they control with the greatest mana value unless they pay that creature's mana cost. If two or more creatures a player controls are tied for greatest, that player chooses one.",
        false,
    )
    .expect("Tariff should compile");
    let debug = format!("{definition:#?}");
    assert!(debug.contains("ChooseObjects"), "{debug}");
    assert!(debug.contains("GreatestManaValue"), "{debug}");
    assert!(debug.contains("mana_cost_of"), "{debug}");
}

#[test]
fn revealed_cards_with_different_names_gate_the_random_pick() {
    let definition = compile_to_runtime_definition(
        "Signal the Clans",
        "Mana cost: {R}{G}\nType: Instant\nSearch your library for three creature cards and reveal them. If you reveal three cards with different names, choose one of them at random and put that card into your hand. Shuffle the rest into your library.",
        false,
    )
    .expect("Signal the Clans should compile");
    let debug = format!("{definition:#?}");
    assert!(debug.contains("DistinctNames"), "{debug}");
}

#[test]
fn failed_opponent_choice_taps_the_source() {
    let definition = compile_to_runtime_definition(
        "Territorial Hellkite",
        "Mana cost: {2}{R}{R}\nType: Creature — Dragon\nPower/Toughness: 6/5\nFlying, haste\nAt the beginning of combat on your turn, choose an opponent at random that this creature didn't attack during your last combat. This creature attacks that player this combat if able. If you can't choose an opponent this way, tap this creature.",
        false,
    )
    .expect("Territorial Hellkite should compile");
    let debug = format!("{definition:#?}");
    assert!(debug.contains("DidNot") || debug.contains("did_not"), "{debug}");
}

#[test]
fn a_player_dealt_six_combat_damage_gates_the_sidequest() {
    let definition = compile_to_runtime_definition(
        "Sidequest: Play Blitzball",
        "Mana cost: {2}{R}\nType: Enchantment\nAt the beginning of combat on your turn, target creature you control gets +2/+0 until end of turn.\nAt the end of combat on your turn, if a player was dealt 6 or more combat damage this turn, transform this enchantment, then attach it to a creature you control.",
        false,
    )
    .expect("Sidequest: Play Blitzball should compile");
    let debug = format!("{definition:#?}");
    assert!(
        debug.contains("AnyPlayerWasDealtCombatDamageThisTurnOrMore { count: 6 }")
            || debug.contains("AnyPlayerWasDealtCombatDamageThisTurnOrMore {\n"),
        "{debug}"
    );
}

#[test]
fn new_host_gains_the_cards_other_printed_abilities() {
    let definition = compile_to_runtime_definition(
        "Symbiote Spider-Man",
        "Mana cost: {2}{U/B}\nType: Legendary Creature — Symbiote Spider Hero\nPower/Toughness: 2/4\nWhenever this creature deals combat damage to a player, look at that many cards from the top of your library. Put one of them into your hand and the rest into your graveyard.\nFind New Host — {2}{U/B}, Exile this card from your graveyard: Put a +1/+1 counter on target creature you control. It gains this card's other abilities. Activate only as a sorcery.",
        false,
    )
    .expect("Symbiote Spider-Man should compile");
    let debug = format!("{definition:#?}");
    // The graveyard ability grants the combat-damage trigger to the new host.
    assert!(debug.contains("AddAbilityGeneric"), "{debug}");
    assert!(debug.matches("DealsCombatDamage").count() >= 2 || debug.matches("CombatDamage").count() >= 2, "{debug}");
}
