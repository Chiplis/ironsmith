//! Claude package 40 runtime checks for the compilation-completion task
//! (see reports/compilation-completion-20261016/COORDINATION.md).
#![allow(unused_imports)]
use super::compile_to_runtime_definition;
use ironsmith::effect::Effect;

/// "the same number and kind of counters": the counter kind and amount are
/// read from the triggering counter placement at resolution.
#[test]
fn same_number_and_kind_of_counters_copies_the_triggering_placement() {
    use ironsmith::events::MarkersChangedEvent;
    use ironsmith::object::CounterType;
    use ironsmith::triggers::TriggerEvent;
    use ironsmith::{CardId, CardType, PlayerId, Zone};
    for (name, text) in [
        (
            "Captain Marvel, Apex Avenger",
            "Mana cost: {5}{R}{W}\nType: Legendary Creature — Human Kree Hero\nPower/Toughness: 6/6\nFlying, double strike, indestructible\nWhenever you put one or more counters on another creature, if it's not a Kree, you may put the same number and kind of counters on Captain Marvel.",
        ),
        (
            "Bold Plagiarist",
            "Mana cost: {3}{B}\nType: Creature — Faerie Rogue\nPower/Toughness: 2/3\nFlash\nWhenever an opponent puts one or more counters on a creature they control, they put the same number and kind of counters on this creature.",
        ),
    ] {
        let definition = compile_to_runtime_definition(name, text, false)
            .unwrap_or_else(|error| panic!("{name} should compile: {error:?}"));
        let debug = format!("{:?}", definition.abilities);
        assert!(debug.contains("counter_kind_from_trigger: true"), "{name}: {debug}");
        assert!(debug.contains("CounterPutOnTrigger") || debug.contains("CounterPutOn"), "{name}: {debug}");
    }

    let mut game = ironsmith::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let card = ironsmith::card::CardBuilder::new(CardId::new(), "Counter copier")
        .card_types(vec![CardType::Creature])
        .power_toughness(ironsmith::card::PowerToughness::fixed(1, 1))
        .build();
    let source = game.create_object_from_card(&card, alice, Zone::Battlefield);
    let other = game.create_object_from_card(&card, alice, Zone::Battlefield);
    let event = TriggerEvent::new_with_provenance(
        MarkersChangedEvent::added(CounterType::Flying, other, 2, None, None),
        Default::default(),
    );
    let effect = ironsmith::effects::PutCountersEffect::new(
        CounterType::PlusOnePlusOne,
        ironsmith::effect::Value::EventValue(ironsmith::effect::EventValueSpec::Amount),
        ironsmith::target::ChooseSpec::Source,
    )
    .with_counter_kind_from_trigger();
    let mut ctx =
        ironsmith::effects::EffectContext::new_default(source, alice).with_triggering_event(event);
    ironsmith::effects::execute_effect(&mut game, &Effect::new(effect), &mut ctx).unwrap();
    drop(ctx);
    assert_eq!(game.counter_count(source, CounterType::Flying), 2);
    assert_eq!(game.counter_count(source, CounterType::PlusOnePlusOne), 0);
}

/// "reselect the targets at random for the spell and the copy. The new
/// targets can't be you or a permanent you control."
#[test]
fn chefs_kiss_reselects_both_spells_targets_at_random_excluding_you() {
    let definition = compile_to_runtime_definition(
        "Chef's Kiss",
        "Mana cost: {1}{R}{R}\nType: Instant\nGain control of target spell that targets only a single permanent or player. Copy it, then reselect the targets at random for the spell and the copy. The new targets can't be you or a permanent you control.",
        false,
    )
    .expect("Chef's Kiss should compile");
    let debug = format!("{definition:?}");
    assert!(debug.contains("CopySpellEffect"), "{debug}");
    assert_eq!(debug.matches("random_targets: true").count(), 2, "{debug}");
    assert_eq!(debug.matches("PlayerOrObject").count(), 2, "{debug}");
    assert!(debug.contains("NotYou"), "{debug}");
    assert!(debug.contains("__copied_stack_object__"), "{debug}");
}

fn p40_triggered(
    definition: &ironsmith::cards::CardDefinition,
) -> Vec<&ironsmith::ability::TriggeredAbility> {
    definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            ironsmith::ability::AbilityKind::Triggered(trigger) => Some(trigger),
            _ => None,
        })
        .collect()
}

/// "for each color, put a +1/+1 counter on a Dragon you control of that
/// color. If you put +1/+1 counters on five Dragons this way, you win."
#[test]
fn call_the_spirit_dragons_counts_distinct_dragons_across_color_choices() {
    use ironsmith::card::{CardBuilder, PowerToughness};
    use ironsmith::color::ColorSet;
    use ironsmith::ids::{CardId, PlayerId};
    use ironsmith::types::{CardType, Subtype};
    use ironsmith::zone::Zone;
    let definition = compile_to_runtime_definition(
        "Call the Spirit Dragons",
        "Mana cost: {W}{U}{B}{R}{G}\nType: Enchantment\nDragons you control have indestructible.\nAt the beginning of your upkeep, for each color, put a +1/+1 counter on a Dragon you control of that color. If you put +1/+1 counters on five Dragons this way, you win the game.",
        false,
    )
    .expect("Call the Spirit Dragons should compile");
    let triggers = p40_triggered(&definition);
    assert_eq!(triggers.len(), 1);
    let debug = format!("{:?}", triggers[0].effects);
    assert_eq!(debug.matches("ChooseObjectsEffect").count(), 5, "{debug}");
    assert!(debug.contains("WinTheGame"), "{debug}");

    for (dragon_colors, wins) in [
        (vec![ColorSet::WHITE, ColorSet::BLUE, ColorSet::BLACK, ColorSet::RED, ColorSet::GREEN], true),
        // One white-blue Dragon receives both the white and blue counters:
        // four distinct Dragons.
        (vec![ColorSet::WHITE.union(ColorSet::BLUE), ColorSet::BLACK, ColorSet::RED, ColorSet::GREEN], false),
    ] {
        let mut game =
            ironsmith::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let mut dragons = Vec::new();
        for colors in &dragon_colors {
            let card = CardBuilder::new(CardId::new(), "Spirit Dragon fixture")
                .card_types(vec![CardType::Creature])
                .subtypes(vec![Subtype::Dragon])
                .color_indicator(*colors)
                .power_toughness(PowerToughness::fixed(4, 4))
                .build();
            dragons.push(game.create_object_from_card(&card, alice, Zone::Battlefield));
        }
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let mut ctx = ironsmith::effects::EffectContext::new_default(source, alice);
        for effect in triggers[0].effects.all_effects() {
            ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
        }
        drop(ctx);
        let total: u32 = dragons
            .iter()
            .map(|dragon| game.counter_count(*dragon, ironsmith::object::CounterType::PlusOnePlusOne))
            .sum();
        assert_eq!(total, 5, "one counter per color");
        assert_eq!(game.player(bob).unwrap().has_lost, wins, "{dragon_colors:?}");
    }
}

/// "as long as the chosen color is the most common color among nontoken
/// permanents the chosen player controls but isn't tied for most common".
#[test]
fn call_to_arms_anthem_needs_the_chosen_color_strictly_most_common() {
    use ironsmith::card::{CardBuilder, PowerToughness};
    use ironsmith::color::{Color, ColorSet};
    use ironsmith::ids::{CardId, PlayerId};
    use ironsmith::types::CardType;
    use ironsmith::zone::Zone;
    let definition = compile_to_runtime_definition(
        "Call to Arms",
        "Mana cost: {1}{W}\nType: Enchantment\nAs this enchantment enters, choose a color and an opponent.\nWhite creatures get +1/+1 as long as the chosen color is the most common color among nontoken permanents the chosen player controls but isn't tied for most common.\nWhen the chosen color isn't the most common color among nontoken permanents the chosen player controls or is tied for most common, sacrifice this enchantment.",
        false,
    )
    .expect("Call to Arms should compile");
    let debug = format!("{definition:?}");
    assert!(debug.contains("ChooseColor"), "{debug}");
    assert!(debug.contains("ChosenPlayer"), "{debug}");
    assert_eq!(
        debug.matches("SourceChosenColorMostCommonAmong").count(),
        2,
        "anthem gate and state trigger: {debug}"
    );
    assert!(debug.contains("strict: true"), "{debug}");
    assert!(debug.contains("StateBased") || debug.contains("State"), "{debug}");

    let mut game = ironsmith::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let creature = |name: &str, colors: ColorSet| {
        CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Creature])
            .color_indicator(colors)
            .power_toughness(PowerToughness::fixed(2, 2))
            .build()
    };
    let soldier = game.create_object_from_card(&creature("White soldier", ColorSet::WHITE), alice, Zone::Battlefield);
    game.create_object_from_card(&creature("Bob white 1", ColorSet::WHITE), bob, Zone::Battlefield);
    game.create_object_from_card(&creature("Bob white 2", ColorSet::WHITE), bob, Zone::Battlefield);
    game.create_object_from_card(&creature("Bob blue 1", ColorSet::BLUE), bob, Zone::Battlefield);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    game.set_chosen_color(source, Color::White);
    game.set_chosen_player(source, bob);
    assert_eq!(game.calculated_power(soldier), Some(3), "white leads 2-1");
    game.create_object_from_card(&creature("Bob blue 2", ColorSet::BLUE), bob, Zone::Battlefield);
    assert_eq!(game.calculated_power(soldier), Some(2), "a tie removes the bonus");
}

/// "each player rolls a d20. If one or more opponents had the highest result,
/// this creature can't attack those players or planeswalkers they control".
#[test]
fn chaos_dragon_restricts_attacks_against_the_highest_rolling_opponents() {
    let definition = compile_to_runtime_definition(
        "Chaos Dragon",
        "Mana cost: {1}{R}{R}\nType: Creature — Dragon\nPower/Toughness: 2/2\nFlying, haste\nThis creature attacks each combat if able.\nAt the beginning of combat on your turn, each player rolls a d20. If one or more opponents had the highest result, this creature can't attack those players or planeswalkers they control this combat.",
        false,
    )
    .expect("Chaos Dragon should compile");
    let triggers = p40_triggered(&definition);
    assert_eq!(triggers.len(), 1);
    let debug = format!("{:?}", triggers[0].effects);
    assert!(debug.contains("RollDieKeepHighestPlayersEffect"), "{debug}");
    assert!(debug.contains("sides: 20"), "{debug}");
    assert!(debug.contains("keep: Opponent"), "{debug}");
    assert!(debug.contains("AttackPlayerOrPlaneswalkersControlledBy"), "{debug}");
    assert!(debug.contains("IteratedPlayer"), "{debug}");
    assert!(debug.contains("EndOfCombat"), "{debug}");

    // The highest rollers are kept; with two players exactly the higher one
    // (or both on a tie) remains tagged, filtered to opponents.
    let mut game = ironsmith::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = ironsmith::PlayerId::from_index(0);
    let bob = ironsmith::PlayerId::from_index(1);
    let card = ironsmith::card::CardBuilder::new(ironsmith::ids::CardId::new(), "Roll source")
        .card_types(vec![ironsmith::types::CardType::Artifact])
        .build();
    let source = game.create_object_from_card(&card, alice, ironsmith::Zone::Battlefield);
    let mut ctx = ironsmith::effects::EffectContext::new_default(source, alice);
    for effect in [
        Effect::new(ironsmith::effects::TagPlayersEffect::new(
            ironsmith::target::PlayerFilter::Any,
            "p40_rollers",
        )),
        Effect::new(ironsmith::effects::RollDieKeepHighestPlayersEffect::new(
            "p40_rollers",
            20,
            ironsmith::target::PlayerFilter::Opponent,
        )),
    ] {
        ironsmith::effects::execute_effect(&mut game, &effect, &mut ctx).unwrap();
    }
    let kept = ctx
        .tagged_players
        .get(&ironsmith_core::tag::TagKey::from("p40_rollers"))
        .cloned()
        .unwrap_or_default();
    assert!(kept.iter().all(|player| *player == bob), "only opponents are kept: {kept:?}");
    assert!(kept.len() <= 1);
}

/// Two dice, each selecting a row of the bulleted result table, plus doubles.
#[test]
fn celebr_8000_rolls_two_dice_into_a_per_die_result_table() {
    let definition = compile_to_runtime_definition(
        "Celebr-8000",
        "Mana cost: {5}\nType: Artifact Creature — Clown Robot\nPower/Toughness: 4/4\nAt the beginning of combat on your turn, roll two six-sided dice. For each result of 1, this creature gets +1/+1 until end of turn. For each other result, it gains the indicated ability until end of turn. If you rolled doubles, it also gains double strike until end of turn.\n• 2 — menace\n• 3 — vigilance\n• 4 — lifelink\n• 5 — flying\n• 6 — indestructible",
        false,
    )
    .expect("Celebr-8000 should compile");
    let triggers = p40_triggered(&definition);
    assert_eq!(triggers.len(), 1);
    let debug = format!("{:?}", triggers[0].effects);
    assert!(debug.contains("RollDiceChooseResultEffect"), "{debug}");
    assert!(debug.contains("in_order: true"), "{debug}");
    // Six rows per die plus the doubles gate.
    assert!(debug.matches("OtherNumber").count() >= 7, "{debug}");
    assert!(!debug.contains("PendingEffectMetric"), "every row reads the roll: {debug}");
    assert!(debug.contains("OtherNumber"), "{debug}");
    for keyword in ["Menace", "Vigilance", "Lifelink", "Flying", "Indestructible", "DoubleStrike"] {
        assert!(debug.contains(keyword), "{keyword}: {debug}");
    }
}

/// "Planeswalkers' loyalty abilities you activate cost an additional [+1]".
#[test]
fn carth_adds_a_loyalty_surcharge_to_your_loyalty_abilities() {
    let definition = compile_to_runtime_definition(
        "Carth the Lion",
        "Mana cost: {2}{B}{G}\nType: Legendary Creature — Human Warrior\nPower/Toughness: 3/5\nWhenever Carth enters or a planeswalker you control dies, look at the top seven cards of your library. You may reveal a planeswalker card from among them and put it into your hand. Put the rest on the bottom of your library in a random order.\nPlaneswalkers' loyalty abilities you activate cost an additional [+1] to activate.",
        false,
    )
    .expect("Carth the Lion should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("ActivatedAbilityCostIncrease"), "{debug}");
    assert!(debug.contains("LoyaltyAbility"), "{debug}");
    assert!(debug.contains("Activator(You)"), "{debug}");
    assert!(debug.contains("Planeswalker"), "{debug}");
    assert!(debug.contains("Loyalty"), "{debug}");
}

/// "Until that player's next end step, they may play that card without paying
/// its mana cost": the holder's own next end step, for the targeted player.
#[test]
fn casey_and_raph_grant_lasts_until_the_target_players_next_end_step() {
    let definition = compile_to_runtime_definition(
        "Casey & Raph, Hotheads",
        "Mana cost: {4}{R}\nType: Legendary Creature — Mutant Ninja Human Turtle\nPower/Toughness: 5/4\nWhen Casey & Raph enter, choose one or both. Each mode must target a different player.\n• Target player exiles the top card of their library. Until that player's next end step, they may play that card without paying its mana cost.\n• Target player creates two Treasure tokens.",
        false,
    )
    .expect("Casey & Raph should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("UntilPlayerNextEndStep"), "{debug}");
    assert!(debug.contains("GrantTaggedSpellFreeCastUntilEndOfTurnEffect") || debug.contains("alternative_cost"), "{debug}");
    assert!(!debug.contains("UntilYourNextEndStep"), "{debug}");
    assert!(debug.contains("Treasure"), "{debug}");
}

/// "{T}, Mill a card: For each colored mana symbol in the milled card's mana
/// cost, add one mana of that color."
#[test]
fn charmed_pendant_adds_one_mana_per_colored_symbol_of_the_milled_card() {
    use ironsmith::mana::{ManaCost, ManaSymbol};
    let definition = compile_to_runtime_definition(
        "Charmed Pendant",
        "Mana cost: {4}\nType: Artifact\n{T}, Mill a card: For each colored mana symbol in the milled card's mana cost, add one mana of that color. (Activate only as an instant. For example, if the card's mana cost is {3}{U}{U}{B}, you add {U}{U}{B}.)",
        false,
    )
    .expect("Charmed Pendant should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("colored_symbols_only: true"), "{debug}");
    assert!(debug.contains(ironsmith_core::tag::COST_MILLED_TAG), "{debug}");

    let mut game = ironsmith::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = ironsmith::PlayerId::from_index(0);
    let milled_card = ironsmith::card::CardBuilder::new(ironsmith::ids::CardId::new(), "Milled card")
        .card_types(vec![ironsmith::types::CardType::Sorcery])
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::Blue],
            vec![ManaSymbol::Blue],
            vec![ManaSymbol::Black],
            vec![ManaSymbol::Generic(2), ManaSymbol::White],
        ]))
        .build();
    let milled = game.create_object_from_card(&milled_card, alice, ironsmith::Zone::Graveyard);
    let pendant = game.create_object_from_definition(&definition, alice, ironsmith::Zone::Battlefield);
    let mut ctx = ironsmith::effects::EffectContext::new_default(pendant, alice);
    ctx.set_tagged_objects(
        ironsmith_core::tag::COST_MILLED_TAG,
        vec![ironsmith::snapshot::ObjectSnapshot::from_object_id(&game, milled).unwrap()],
    );
    let effect = ironsmith::effects::AddManaEqualToManaCostEffect::new(
        ironsmith::target::ObjectFilter::tagged(ironsmith_core::tag::COST_MILLED_TAG),
        ironsmith::target::PlayerFilter::You,
    )
    .with_colored_symbols_only(true);
    ironsmith::effects::execute_effect(&mut game, &Effect::new(effect), &mut ctx).unwrap();
    drop(ctx);
    // {U}{U}{B} plus the colored half of {2/W}; generic adds nothing.
    assert_eq!(game.player(alice).unwrap().mana_pool.total(), 4);
}

/// "it becomes a Vehicle artifact with base power and toughness each equal to
/// its mana value, and it gains crew 2 and \"Crash Land — ...\"": a
/// noncreature Vehicle until crewed, with the crew and quoted abilities.
#[test]
fn captain_rex_nebula_makes_a_noncreature_vehicle_with_crew_and_crash_land() {
    let definition = compile_to_runtime_definition(
        "Captain Rex Nebula",
        "Mana cost: {1}{R}{W}\nType: Legendary Creature — Human Pilot Employee\nPower/Toughness: 3/3\nAt the beginning of combat on your turn, choose target nonland permanent you control. Until end of turn, it becomes a Vehicle artifact with base power and toughness each equal to its mana value, and it gains crew 2 and \"Crash Land — Whenever this Vehicle deals damage, roll a six-sided die. If the result is equal to this Vehicle's mana value, sacrifice this Vehicle, then it deals that much damage to any target.\"",
        false,
    )
    .expect("Captain Rex Nebula should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("Vehicle"), "{debug}");
    assert!(!debug.contains("AddCardTypes([Creature, Artifact])"), "not a creature until crewed: {debug}");
    assert!(debug.contains("ManaValueOf"), "{debug}");
    assert!(debug.contains("Crew") || debug.contains("crew"), "{debug}");
    assert!(debug.contains("RollDie"), "the quoted Crash Land trigger: {debug}");
}

/// "{X}, {T}: Create a token that's a copy of a card you exiled with cards
/// named Caller of the Untamed. X is the mana value of that card."
#[test]
fn caller_of_the_untamed_copies_the_linked_exiled_card_with_mana_value_x() {
    let definition = compile_to_runtime_definition(
        "Caller of the Untamed",
        "Mana cost: {3}{G}\nType: Creature — Elf Shaman\nPower/Toughness: 2/3\nBefore you shuffle your deck to start the game, you may reveal this card from your deck and exile a creature card you drafted that isn't in your deck.\n{X}, {T}: Create a token that's a copy of a card you exiled with cards named Caller of the Untamed. X is the mana value of that card.",
        false,
    )
    .expect("Caller of the Untamed should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("exiled_with_cards_named: Some("), "{debug}");
    assert!(debug.contains("EqualExpr(X)"), "the chosen card's mana value is X: {debug}");
    assert!(debug.contains("CreateTokenCopy") || debug.contains("TokenCopy"), "{debug}");
    assert!(debug.contains("ChooseObjectsEffect"), "{debug}");
}
