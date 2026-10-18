//! Claude package 28 runtime checks for the compilation-completion task
//! (see reports/compilation-completion-20261016/COORDINATION.md).
#![allow(unused_imports)]
use super::compile_to_runtime_definition;
use ironsmith::effect::Effect;

fn p28_compile(name: &str, text: &str) -> ironsmith::cards::CardDefinition {
    let (definition, trace) = ironsmith_compiler::parse_trace::capture(|| {
        compile_to_runtime_definition(name, text, false)
    });
    definition.unwrap_or_else(|error| panic!("{name}: {error:?}\n{}", trace.render()))
}

#[test]
fn p28_devouring_spirit_sacrifice_costs_feed_the_body_counts() {
    let greed = p28_compile("Devouring Greed",
        "Mana cost: {2}{B}{B}\nType: Sorcery — Arcane\nAs an additional cost to cast this spell, you may sacrifice any number of Spirits.\nTarget player loses 2 life plus 2 life for each Spirit sacrificed this way. You gain that much life.");
    let rage = p28_compile("Devouring Rage",
        "Mana cost: {4}{R}\nType: Instant — Arcane\nAs an additional cost to cast this spell, you may sacrifice any number of Spirits.\nTarget creature gets +3/+0 until end of turn. For each Spirit sacrificed this way, that creature gets an additional +3/+0 until end of turn.");
    for definition in [&greed, &rage] {
        // Choosing zero Spirits is a legal payment of the ordinary
        // additional cost; it is not a separately announced optional cost.
        assert!(definition.optional_costs.is_empty(), "{:?}", definition.optional_costs);
        let cost = format!("{:?}", definition.additional_cost);
        assert!(cost.contains("Spirit") && cost.contains("SacrificePlayerEffect"), "{cost}");
        let body = format!("{:?}", definition.spell_effect);
        assert!(body.contains("Sacrificed") && body.contains("Spirit"), "{body}");
    }
    let body = format!("{:?}", greed.spell_effect);
    assert!(body.contains("LoseLifeEffect") && body.contains("GainLifeEffect"), "{body}");
}

#[test]
fn p28_daredevil_dragster_vehicle_self_reference_combat_predicate() {
    let definition = p28_compile("Daredevil Dragster",
        "Mana cost: {3}\nType: Artifact — Vehicle\nPower/Toughness: 4/4\nAt end of combat, if this Vehicle attacked or blocked this combat, put a velocity counter on it. Then if it has two or more velocity counters on it, sacrifice it and draw two cards.\nCrew 2 (Tap any number of creatures you control with total power 2 or more: This Vehicle becomes an artifact creature until end of turn.)");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("SourceAttackedOrBlockedThisCombat"), "{debug}");
    assert!(debug.contains("DrawCardsEffect"), "{debug}");
}

#[test]
fn p28_dermoplasm_face_up_battlefield_entry_is_the_default_entry() {
    let definition = p28_compile("Dermoplasm",
        "Mana cost: {2}{U}\nType: Creature — Shapeshifter\nPower/Toughness: 1/1\nFlying\nMorph {2}{U}{U} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)\nWhen this creature is turned face up, you may put a creature card with a morph ability from your hand onto the battlefield face up. If you do, return this creature to its owner's hand.");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("Battlefield") && debug.contains("Hand"), "{debug}");
    assert!(!debug.contains("face_down: true"), "{debug}");
}

#[test]
fn p28_deicide_shuffle_stays_inside_the_god_search_branch() {
    let definition = p28_compile("Deicide",
        "Mana cost: {1}{W}\nType: Instant\nExile target enchantment. If the exiled card is a God card, search its controller's graveyard, hand, and library for any number of cards with the same name as that card and exile them. Then that player shuffles.");
    let debug = format!("{:?}", definition.spell_effect);
    let shuffle = debug.find("ShuffleLibraryEffect").expect("one shuffle");
    assert_eq!(debug.matches("ShuffleLibraryEffect").count(), 1, "{debug}");
    let tail = &debug[shuffle..(shuffle + 200).min(debug.len())];
    assert!(tail.contains("ControllerOf") && tail.contains("target_spec: None"),
        "that player is the searched controller, not a new target: {tail}");
    let if_true = debug.find("if_true").expect("conditional");
    let if_false = debug[if_true..].find("if_false").map(|at| at + if_true).expect("else arm");
    assert!(if_true < shuffle && shuffle < if_false, "shuffle runs only with the God search: {debug}");
}

#[test]
fn p28_defiant_vanguard_destroys_itself_and_the_creatures_it_blocked() {
    let definition = p28_compile("Defiant Vanguard",
        "Mana cost: {2}{W}\nType: Creature — Human Rebel\nPower/Toughness: 2/2\nWhen this creature blocks, at end of combat, destroy it and all creatures it blocked this turn.\n{5}, {T}: Search your library for a Rebel permanent card with mana value 4 or less, put it onto the battlefield, then shuffle.");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("blocked_by: Some("), "the destroyed set is the creatures it blocked: {debug}");
    assert!(debug.matches("Destroy").count() >= 2, "{debug}");
}

#[test]
fn p28_cut_propulsion_flying_doubles_the_self_damage() {
    let definition = p28_compile("Cut Propulsion",
        "Mana cost: {2}{R}\nType: Instant\nTarget creature deals damage to itself equal to its power. If that creature has flying, it deals twice that much damage to itself instead.");
    let debug = format!("{:?}", definition.spell_effect);
    assert!(debug.contains("Flying"), "{debug}");
    assert!(debug.contains("Scaled("), "the replacement arm doubles the power-based amount: {debug}");
    assert_eq!(debug.matches("DealDamageEffect").count(), 2, "default and replacement arms: {debug}");
}

#[test]
fn p28_desperate_plea_mode_reads_the_sacrificed_cost_creature() {
    let definition = p28_compile("Desperate Plea",
        "Mana cost: {1}{B}\nType: Sorcery — Lesson\nAs an additional cost to cast this spell, sacrifice a creature.\nChoose one or both —\n• Return target creature card from your graveyard to the battlefield if its power is less than or equal to the sacrificed creature's power.\n• Destroy target creature.");
    let debug = format!("{:?}", definition.spell_effect);
    assert!(debug.contains("sacrifice_cost_") || debug.contains("sacrificed"),
        "the mode's comparison names the paid sacrifice: {debug}");
    assert!(debug.contains("DestroyEffect"), "{debug}");
}

#[test]
fn p28_die_young_shrinks_by_the_energy_paid_this_way() {
    let definition = p28_compile("Die Young",
        "Mana cost: {1}{B}\nType: Sorcery\nChoose target creature. You get {E}{E} (two energy counters), then you may pay any amount of {E}. The creature gets -1/-1 until end of turn for each {E} paid this way.");
    let debug = format!("{:?}", definition.spell_effect);
    assert!(debug.contains("Energy") && debug.contains("Removed"),
        "the -1/-1 count is the energy the payment removed: {debug}");
    assert!(!debug.contains("PendingPriorEffectMetric"), "bound to the payment: {debug}");
}

#[test]
fn p28_depala_puts_every_dwarf_and_vehicle_card_into_hand_and_the_rest_on_the_bottom() {
    let definition = p28_compile("Depala, Pilot Exemplar",
        "Mana cost: {1}{R}{W}\nType: Legendary Creature — Dwarf Pilot\nPower/Toughness: 2/2\nOther Dwarves you control get +1/+1.\nEach Vehicle you control gets +1/+1 as long as it's a creature.\nWhenever Depala becomes tapped, you may pay {X}. If you do, reveal the top X cards of your library, put all Dwarf and Vehicle cards from among them into your hand, then put the rest on the bottom of your library in a random order.");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("TagMatchingObjectsEffect"), "all matching cards move, no choice: {debug}");
    assert!(debug.contains("Dwarf") && debug.contains("Vehicle"), "{debug}");
    assert!(!debug.contains("zone: Graveyard"), "the rest go to the bottom, not the graveyard: {debug}");
}

#[test]
fn p28_dihada_counts_the_cards_the_remainder_put_into_the_graveyard() {
    let definition = p28_compile("Dihada, Binder of Wills",
        "Mana cost: {1}{R}{W}{B}\nType: Legendary Planeswalker — Dihada\nLoyalty: 5\n+2: Up to one target legendary creature gains vigilance, lifelink, and indestructible until your next turn.\n−3: Reveal the top four cards of your library. Put any number of legendary cards from among them into your hand and the rest into your graveyard. Create a Treasure token for each card put into your graveyard this way.\n−11: Gain control of all nonland permanents until end of turn. Untap them. They gain haste until end of turn.\nDihada, Binder of Wills can be your commander.");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("PutIntoGraveyard"), "the Treasure count reads the remainder move: {debug}");
    assert!(!debug.contains("ACTIVATION_COUNTER_COST"), "{debug}");
}

#[test]
fn p28_custody_battle_granted_upkeep_control_is_unless_you_sacrifice_a_land() {
    let definition = p28_compile("Custody Battle",
        "Mana cost: {1}{R}\nType: Enchantment — Aura\nEnchant creature\nEnchanted creature has \"At the beginning of your upkeep, target opponent gains control of this creature unless you sacrifice a land.\"");
    let debug = format!("{:?}", definition.abilities);
    let unless = debug.find("UnlessPaysEffect").expect("punisher cost wraps the control change");
    let tail = &debug[unless..];
    assert!(tail.contains("ChangeControllerToPlayer(Target(Opponent))"), "{debug}");
    assert!(tail.contains("Land"), "the alternative is sacrificing a land: {debug}");
    assert!(debug.contains("Opponent"), "the new controller is a target opponent: {debug}");
}

#[test]
fn p28_desolation_damages_the_players_who_sacrificed_a_plains() {
    let definition = p28_compile("Desolation",
        "Mana cost: {1}{B}{B}\nType: Enchantment\nAt the beginning of each end step, each player who tapped a land for mana this turn sacrifices a land of their choice. This enchantment deals 2 damage to each player who sacrificed a Plains this way.");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("PlayerTappedLandForManaThisTurn"), "{debug}");
    assert!(debug.contains("Plains") && debug.contains("DealDamageEffect"), "{debug}");
}

#[test]
fn p28_crystalline_giant_puts_a_random_absent_kind_until_every_kind_is_present() {
    use ironsmith::CounterType;
    let definition = p28_compile("Crystalline Giant",
        "Mana cost: {3}\nType: Artifact Creature — Giant\nPower/Toughness: 3/3\nAt the beginning of combat on your turn, choose a kind of counter at random that this creature doesn't have on it from among flying, first strike, deathtouch, hexproof, lifelink, menace, reach, trample, vigilance, and +1/+1. Put a counter of that kind on this creature.");
    let program = definition.abilities.iter().find_map(|ability| match &ability.kind {
        ironsmith::ability::AbilityKind::Triggered(trigger) => Some(&trigger.effects),
        _ => None,
    }).expect("beginning-of-combat trigger");
    let debug = format!("{program:?}");
    assert!(debug.contains("PutCounterOfRandomKindEffect") && debug.contains("only_absent_kinds: true"), "{debug}");
    let kinds = [
        CounterType::Flying, CounterType::FirstStrike, CounterType::Deathtouch, CounterType::Hexproof,
        CounterType::Lifelink, CounterType::Menace, CounterType::Reach, CounterType::Trample,
        CounterType::Vigilance, CounterType::PlusOnePlusOne,
    ];
    let alice = ironsmith::PlayerId::from_index(0);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let source = game.create_object_from_definition(&definition, alice, ironsmith::Zone::Battlefield);
    for resolution in 1..=11usize {
        let mut ctx = ironsmith::effects::EffectContext::new_default(source, alice);
        for effect in program.all_effects() {
            ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
        }
        drop(ctx);
        let present = kinds.iter().filter(|kind| game.counter_count(source, **kind) > 0).count();
        // Each resolution adds one new kind; once all ten are present, nothing happens.
        assert_eq!(present, resolution.min(kinds.len()), "after resolution {resolution}");
        for kind in kinds {
            assert!(game.counter_count(source, kind) <= 1, "{kind:?} was repeated");
        }
    }
}

#[test]
fn p28_denry_klin_copies_each_counter_kind_onto_the_entering_creature() {
    use ironsmith::CounterType;
    use ironsmith::card::{CardBuilder, PowerToughness};
    use ironsmith::ids::CardId;
    use ironsmith::types::CardType;
    let definition = p28_compile("Denry Klin, Editor in Chief",
        "Mana cost: {2}{W}{U}\nType: Legendary Creature — Cat Advisor\nPower/Toughness: 2/2\nDenry Klin enters with your choice of a +1/+1, first strike, or vigilance counter on it.\nWhenever a nontoken creature you control enters, if Denry Klin has counters on it, put the same number of each kind of counter on that creature.");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("MoveAllCountersEffect") && debug.contains("remove_from_source: false"), "{debug}");
    let alice = ironsmith::PlayerId::from_index(0);
    for with_counters in [true, false] {
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&definition, alice, ironsmith::Zone::Battlefield);
        if with_counters {
            game.add_counters(source, CounterType::PlusOnePlusOne, 1);
            game.add_counters(source, CounterType::FirstStrike, 2);
        }
        game.refresh_continuous_state().unwrap();
        let _ = game.take_pending_trigger_events();
        let card = CardBuilder::new(CardId::new(), "Entering creature witness")
            .card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(1, 1)).build();
        let entrant = game.create_object_from_card(&card, alice, ironsmith::Zone::Hand);
        let receipt = game.move_object_with_etb_processing(entrant, ironsmith::Zone::Battlefield).unwrap();
        let ironsmith::events::processing::EventOutcome::Proceed(result) = receipt.original else {
            panic!("the creature enters");
        };
        let entered = result.new_id;
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        for event in game.take_pending_trigger_events() {
            for trigger in ironsmith::triggers::check_triggers(&game, &event) { queue.add(trigger); }
        }
        ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
        while !game.stack.is_empty() { ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap(); }
        let expected = if with_counters { (1, 2) } else { (0, 0) };
        assert_eq!((game.counter_count(entered, CounterType::PlusOnePlusOne),
            game.counter_count(entered, CounterType::FirstStrike)), expected);
        // Denry keeps its own counters.
        assert_eq!((game.counter_count(source, CounterType::PlusOnePlusOne),
            game.counter_count(source, CounterType::FirstStrike)), expected);
    }
}

#[test]
fn p28_delirium_that_player_is_the_active_opponent_and_the_creature_is_the_tapped_target() {
    let definition = p28_compile("Delirium",
        "Mana cost: {1}{B}{R}\nType: Instant\nCast this spell only during an opponent's turn.\nTap target creature that player controls. That creature deals damage equal to its power to the player. Prevent all combat damage that would be dealt to and dealt by the creature this turn.");
    let debug = format!("{:?}", definition.spell_effect);
    assert!(!debug.contains("IteratedPlayer"), "no unbound that-player placeholder: {debug}");
    let tap = debug.find("TapEffect").expect("tap");
    let tap_tail = &debug[tap..];
    assert!(tap_tail.contains("controller: Some(Active)"),
        "the cast window's opponent is the active player: {debug}");
    assert!(debug.contains("DealDamageEffect"), "{debug}");
    let prevent = debug.find("PreventAllCombatDamage").expect("prevention of the creature's combat damage");
    assert!(!debug[prevent..].contains("Target(Object"),
        "the creature is the tapped target, not a second target: {debug}");
}

#[test]
fn p28_desperate_gambit_chooses_a_permanent_or_spell_source_then_flips() {
    let definition = p28_compile("Desperate Gambit",
        "Mana cost: {R}\nType: Instant\nChoose a source you control and flip a coin. If you win the flip, the next time that source would deal damage this turn, it deals double that damage instead. If you lose the flip, the next time it would deal damage this turn, prevent that damage.");
    let debug = format!("{:?}", definition.spell_effect);
    // A source is a permanent or a spell (CR 120.1): the choice spans the
    // battlefield and the stack, so it can name your own burn spell.
    let choose = debug.find("ChooseObjectsEffect").expect("source choice");
    let choice = &debug[choose..];
    assert!(choice.contains("Battlefield") && choice.contains("Stack"), "{choice}");
    assert!(choice.contains("controller: Some(You)"), "{choice}");
    assert!(debug.contains("FlipCoinEffect"), "{debug}");
    assert!(debug.contains("RegisterDamageMultiplierEffect") && debug.contains("factor: 2"), "{debug}");
    assert!(debug.contains("OneShot"), "the doubling applies to the next damage only: {debug}");
    assert!(debug.contains("PreventNextTimeDamageEffect"), "{debug}");
}

#[test]
fn p28_delaying_shield_offers_one_payment_per_delay_counter_removed() {
    let definition = p28_compile("Delaying Shield",
        "Mana cost: {3}{W}\nType: Enchantment\nIf damage would be dealt to you, put that many delay counters on this enchantment instead.\nAt the beginning of your upkeep, remove all delay counters from this enchantment. For each delay counter removed this way, you lose 1 life unless you pay {1}{W}.");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("delay"), "delay is a named counter kind: {debug}");
    let repeat = debug.find("RepeatEffectsEffect").expect("one payment offer per counter");
    let body = &debug[repeat..];
    assert!(body.contains("Removed"), "the count is the counters removed this way: {body}");
    assert!(body.contains("UnlessPaysEffect") && body.contains("LoseLifeEffect"), "{body}");
}

#[test]
fn p28_demonic_covenant_sacrifices_only_when_two_milled_cards_share_all_card_types() {
    use ironsmith::card::{CardBuilder, PowerToughness};
    use ironsmith::ids::CardId;
    use ironsmith::types::CardType;
    let definition = p28_compile("Demonic Covenant",
        "Mana cost: {4}{B}{B}\nType: Kindred Enchantment — Demon\nWhenever one or more Demons you control attack a player, you draw a card and lose 1 life.\nAt the beginning of your end step, create a 5/5 black Demon creature token with flying, then mill two cards. If two cards that share all their card types were milled this way, sacrifice this enchantment.");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("AllCardTypes"), "typed identical-card-type-set relation: {debug}");
    assert!(debug.contains("DrawCardsEffect") && debug.contains("LoseLifeEffect"),
        "the Demon attack trigger compiles too: {debug}");
    let end_step = definition.abilities.iter().find_map(|ability| match &ability.kind {
        ironsmith::ability::AbilityKind::Triggered(triggered)
            if format!("{:?}", triggered.effects).contains("MillEffect") =>
        {
            Some(triggered.effects.all_effects_owned())
        }
        _ => None,
    }).expect("end-step mill trigger");
    let alice = ironsmith::PlayerId::from_index(0);
    let card = |name: &str, types: Vec<CardType>| {
        let mut builder = CardBuilder::new(CardId::new(), name).card_types(types.clone());
        if types.contains(&CardType::Creature) {
            builder = builder.power_toughness(PowerToughness::fixed(1, 1));
        }
        builder.build()
    };
    for (first, second, sacrificed) in [
        (vec![CardType::Creature], vec![CardType::Creature], true),
        (vec![CardType::Artifact, CardType::Creature], vec![CardType::Creature, CardType::Artifact], true),
        // Sharing one card type is not sharing all of them.
        (vec![CardType::Artifact, CardType::Creature], vec![CardType::Creature], false),
        (vec![CardType::Instant], vec![CardType::Sorcery], false),
    ] {
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.create_object_from_card(&card("First milled", first.clone()), alice, ironsmith::Zone::Library);
        game.create_object_from_card(&card("Second milled", second.clone()), alice, ironsmith::Zone::Library);
        let source = game.create_object_from_definition(&definition, alice, ironsmith::Zone::Battlefield);
        let mut ctx = ironsmith::effects::EffectContext::new_default(source, alice);
        for effect in &end_step {
            ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
        }
        drop(ctx);
        assert_eq!(game.player(alice).unwrap().graveyard.len() >= 2, true, "both cards milled");
        let still_there = game
            .object(source)
            .is_some_and(|object| object.zone == ironsmith::Zone::Battlefield);
        assert_eq!(still_there, !sacrificed, "{first:?} vs {second:?}");
    }
}

#[test]
fn p28_delina_roll_again_repeats_the_roll_and_its_whole_table() {
    let definition = p28_compile("Delina, Wild Mage",
        "Mana cost: {3}{R}\nType: Legendary Creature — Elf Shaman\nPower/Toughness: 3/2\nWhenever Delina attacks, choose target creature you control, then roll a d20.\n1—14 | Create a tapped and attacking token that's a copy of that creature, except it's not legendary and it has \"At end of combat, exile this token.\"\n15—20 | Create one of those tokens. You may roll again.");
    let debug = format!("{:?}", definition.abilities);
    let repeat = debug.find("RepeatProcessEffect").expect("rolling again is a repeated process");
    let body = &debug[repeat..];
    // The roll itself is inside the loop, so each repetition rolls anew and
    // reads the full table.
    assert!(body.contains("RollDieEffect"), "{debug}");
    // "one of those tokens" is the same copy blueprint as the 1—14 row.
    assert!(body.matches("CreateTokenCopyEffect").count() >= 2, "{debug}");
    assert!(body.contains("RepeatProcessPromptEffect"), "the continuation is optional: {debug}");
    assert_eq!(debug.matches("RepeatProcessEffect").count(), 1, "{debug}");
}
