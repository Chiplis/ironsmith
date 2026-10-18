//! Claude package 7 runtime checks for the compilation-completion task
//! (see reports/compilation-completion-20261016/COORDINATION.md).
#![allow(unused_imports)]
use super::compile_to_runtime_definition;
use ironsmith::effect::Effect;

#[test]
fn p7_martyrdom_only_you_may_activate_binds_the_granted_ability_to_the_caster() {
    use ironsmith::ability::{AbilityKind, ActivationTiming};
    use ironsmith::card::{CardBuilder, PowerToughness};
    use ironsmith::ids::{CardId, ObjectId, PlayerId};
    use ironsmith::types::CardType;
    use ironsmith::zone::Zone;
    let definition = compile_to_runtime_definition("Martyrdom",
        "Mana cost: {1}{W}{W}\nType: Instant\nUntil end of turn, target creature you control gains \"{0}: The next 1 damage that would be dealt to target creature, planeswalker, or player this turn is dealt to this creature instead.\" Only you may activate this ability.", false)
        .expect("Martyrdom should compile");
    let debug = format!("{:?}", definition.spell_effect);
    assert!(debug.contains("AnyTimeByGrantingPlayer(None)"), "the granted ability is marked: {debug}");

    let mut game = ironsmith::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let bear = CardBuilder::new(CardId::new(), "Bear").card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2)).build();
    let creature = game.create_object_from_card(&bear, alice, Zone::Battlefield);
    let program = definition.spell_effect.as_ref().expect("spell effect");
    let mut ctx = ironsmith::effects::EffectContext::new_default(ObjectId::from_raw(9_999), alice)
        .with_targets(vec![ironsmith::effects::ResolvedTarget::Object(creature)]);
    for effect in program.all_effects() {
        ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
    }
    game.refresh_continuous_state().unwrap();
    let granted = game.current_abilities(creature).unwrap().into_iter().find_map(|ability| match ability.kind {
        AbilityKind::Activated(activated) => Some(activated.timing),
        _ => None,
    }).expect("the creature has the granted ability");
    assert_eq!(granted, ActivationTiming::AnyTimeByGrantingPlayer(Some(alice)),
        "\"you\" is bound to the granting spell's controller as the grant resolves");
}

#[test]
fn p7_repl_pandemonium_entering_creatures_controller_decides_and_chooses_the_target() {
    let definition = compile_to_runtime_definition("Pandemonium",
        "Mana cost: {3}{R}\nType: Enchantment\nWhenever a creature enters, that creature's controller may have it deal damage equal to its power to any target of their choice.", false)
        .expect("Pandemonium should compile");
    let debug = format!("{definition:?}");
    assert!(debug.contains("MayEffect") && debug.contains("decider: Some(ControllerOf("),
        "the entering creature's controller makes the may decision: {debug}");
    assert!(debug.contains("TargetOnlyEffect") && debug.contains("chooser: Some(ControllerOf("),
        "the entering creature's controller chooses the target: {debug}");
    assert!(debug.contains("DealDamageEffect") && debug.contains("ExecuteWithSourceEffect"),
        "the entering creature is the damage source: {debug}");
    assert_eq!(debug.matches("TargetOnlyEffect").count(), 1,
        "one shared target declaration, not a second caster-chosen target: {debug}");
}

#[test]
fn p7_combat_meandering_towershell_returns_attacking_at_next_turns_declare_attackers_step() {
    let definition = compile_to_runtime_definition("Meandering Towershell",
        "Mana cost: {3}{G}{G}\nType: Creature — Turtle\nIslandwalk (This creature can't be blocked as long as defending player controls an Island.)\nWhenever this creature attacks, exile it. Return it to the battlefield under your control tapped and attacking at the beginning of the declare attackers step on your next turn.", false)
        .expect("Meandering Towershell should compile");
    let debug = format!("{definition:?}");
    assert!(debug.contains("BeginningOfDeclareAttackersStepTrigger { player: You }"),
        "the return waits for a declare attackers step on your turn: {debug}");
    assert!(debug.contains("start_next_turn: true"), "only from your next turn on: {debug}");
    assert!(debug.contains("ExileEffect") && debug.contains("attacking"),
        "exiles, then returns it attacking: {debug}");

    // The step itself publishes the event the delayed return watches.
    let mut game = ironsmith::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.phase = ironsmith::game_state::Phase::Combat;
    game.turn.step = Some(ironsmith::game_state::Step::DeclareAttackers);
    let event = ironsmith::triggers::generate_step_trigger_events(&game)
        .expect("the declare attackers step publishes a step event");
    assert_eq!(event.kind(), ironsmith::events::EventKind::BeginningOfDeclareAttackersStep);
}

#[test]
fn p7_repl_rain_of_gore_replaces_only_gains_caused_by_the_gainers_own_spell_or_ability() {
    use ironsmith::ids::{ObjectId, PlayerId};
    use ironsmith::target::{ChooseSpec, PlayerFilter};
    use ironsmith::zone::Zone;
    let definition = compile_to_runtime_definition("Rain of Gore",
        "Mana cost: {B}{R}\nType: Enchantment\nIf a spell or ability would cause its controller to gain life, that player loses that much life instead.", false)
        .expect("Rain of Gore should compile");
    let debug = format!("{definition:?}");
    assert!(debug.contains("from_own_spell_or_ability: true"), "{debug}");
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    // (controller of the gaining effect, expected Alice life)
    for (effect_controller, expected) in [(alice, 17), (bob, 23)] {
        let mut game = ironsmith::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.create_object_from_definition(&definition, bob, Zone::Battlefield);
        game.refresh_continuous_state().unwrap();
        let gain = Effect::gain_life_player(3, ChooseSpec::Player(PlayerFilter::Specific(alice)));
        let mut ctx = ironsmith::effects::EffectContext::new_default(ObjectId::from_raw(9_999), effect_controller);
        ironsmith::effects::execute_effect(&mut game, &gain, &mut ctx).unwrap();
        assert_eq!(game.player(alice).unwrap().life, expected,
            "own-effect gain becomes loss; a gain caused by an opponent's effect is untouched");
    }
}

#[test]
fn p7_combat_ramses_wins_when_a_player_attacked_by_your_assassin_loses() {
    let definition = compile_to_runtime_definition("Ramses, Assassin Lord",
        "Mana cost: {2}{U}{B}\nType: Legendary Creature — Human Assassin\nDeathtouch\nOther Assassins you control get +1/+1.\nWhenever a player loses the game, if they were attacked this turn by an Assassin you controlled, you win the game.", false)
        .expect("Ramses, Assassin Lord should compile");
    let debug = format!("{definition:?}");
    assert!(debug.contains("TriggeringPlayerWasAttackedThisTurnBy"),
        "the intervening if reads the losing player's attack history: {debug}");
    let start = debug.find("TriggeringPlayerWasAttackedThisTurnBy").unwrap();
    let condition = &debug[start..(start + 8000).min(debug.len())];
    assert!(condition.contains("Assassin") && condition.contains("controller: Some(You)"),
        "only Assassins you controlled count: {condition}");
    assert!(debug.contains("WinTheGameEffect"), "you win the game: {debug}");
}

#[test]
fn p7_secret_per_player_choices_read_highest_lowest_and_unanimous_options() {
    use ironsmith::decisions::context::NumberContext;
    use ironsmith::game_state::GameState;
    use ironsmith::ids::PlayerId;
    use ironsmith::zone::Zone;
    struct Numbers(Vec<u32>);
    impl ironsmith::decision::DecisionMaker for Numbers {
        fn decide_number(&mut self, _: &GameState, ctx: &NumberContext) -> u32 {
            self.0[ctx.player.0 as usize]
        }
    }
    fn run(definition: &ironsmith::cards::CardDefinition, game: &mut GameState, dm: &mut dyn ironsmith::decision::DecisionMaker) {
        let alice = PlayerId::from_index(0);
        let spell = game.create_object_from_definition(definition, alice, Zone::Stack);
        let mut ctx = ironsmith::effects::EffectContext::new(spell, alice, dm);
        for effect in definition.spell_effect.as_ref().expect("spell effect").all_effects() {
            ironsmith::effects::execute_effect(game, effect, &mut ctx).unwrap();
        }
    }
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let ogre = compile_to_runtime_definition("Menacing Ogre",
        "Mana cost: {3}{R}{R}\nType: Creature — Ogre\nPower/Toughness: 3/3\nTrample, haste\nWhen this creature enters, each player secretly chooses a number. Then those numbers are revealed. Each player with the highest number loses that much life. If you are one of those players, put two +1/+1 counters on this creature.", false)
        .expect("Menacing Ogre should compile");
    let debug = format!("{:?}", ogre.abilities);
    assert!(debug.contains("ChoosePlayerOptionEffect") && debug.contains("secret: true"), "{debug}");
    assert!(debug.contains("PlayersChosenNumberExtreme { highest: true, guess: false }"), "that much = the highest number: {debug}");
    assert!(debug.contains("__player_number_choice__:highest") && debug.contains("PlayerIsYou"), "{debug}");

    let wheel = compile_to_runtime_definition("Wheel of Misfortune",
        "Mana cost: {2}{R}\nType: Sorcery\nEach player secretly chooses a number 0 or greater, then all players reveal those numbers simultaneously and determine the highest and lowest numbers revealed this way. Wheel of Misfortune deals damage equal to the highest number to each player who chose that number. Each player who didn't choose the lowest number discards their hand, then draws seven cards.", false)
        .expect("Wheel of Misfortune should compile");
    let debug = format!("{:?}", wheel.spell_effect);
    assert!(debug.contains("__player_number_choice__:lowest") && debug.contains("Excluding"), "{debug}");
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    run(&wheel, &mut game, &mut Numbers(vec![3, 1]));
    assert_eq!(game.player(alice).unwrap().life, 17, "the highest chooser is dealt that much damage");
    assert_eq!(game.player(bob).unwrap().life, 20);

    let dilemma = compile_to_runtime_definition("Prisoner's Dilemma",
        "Mana cost: {3}{R}{R}\nType: Sorcery\nEach opponent secretly chooses silence or snitch, then the choices are revealed. If each opponent chose silence, Prisoner's Dilemma deals 4 damage to each of them. If each opponent chose snitch, Prisoner's Dilemma deals 8 damage to each of them. Otherwise, Prisoner's Dilemma deals 12 damage to each opponent who chose silence.\nFlashback {5}{R}{R}", false)
        .expect("Prisoner's Dilemma should compile");
    let debug = format!("{:?}", dilemma.spell_effect);
    assert!(debug.contains("__player_option_choice__:silence") && debug.contains("Not("), "{debug}");
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    // The default decision maker picks the first option: Bob chooses silence.
    run(&dilemma, &mut game, &mut ironsmith::decision::SelectFirstDecisionMaker);
    assert_eq!(game.player(bob).unwrap().life, 16, "unanimous silence: 4 damage only, never the otherwise branch");
    assert_eq!(game.player(alice).unwrap().life, 20);
}

#[test]
fn p7_repl_minamos_meddling_discards_cards_named_like_the_countered_spells_splices() {
    use ironsmith::card::CardBuilder;
    use ironsmith::game_state::StackEntry;
    use ironsmith::ids::{CardId, PlayerId};
    use ironsmith::types::CardType;
    use ironsmith::zone::Zone;
    let definition = compile_to_runtime_definition("Minamo's Meddling",
        "Mana cost: {2}{U}{U}\nType: Instant\nCounter target spell. That spell's controller reveals their hand, then discards each card with the same name as a card spliced onto that spell.", false)
        .expect("Minamo's Meddling should compile");
    let debug = format!("{:?}", definition.spell_effect);
    let capture = debug.find("TagSplicedCardsEffect").expect("splice capture compiled");
    let counter = debug.find("CounterEffect").expect("counter compiled");
    assert!(capture < counter, "the splice capture runs while the spell is still on the stack: {debug}");
    assert!(debug.contains("SameNameAsTagged"), "{debug}");

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = ironsmith::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let instant = |name: &str| CardBuilder::new(CardId::new(), name).card_types(vec![CardType::Instant]).build();
    let spell = game.create_object_from_card(&instant("Host Arcane"), bob, Zone::Stack);
    let spliced = game.create_object_from_card(&instant("Splice Witness"), bob, Zone::Hand);
    game.create_object_from_card(&instant("Splice Witness"), bob, Zone::Hand);
    let kept = game.create_object_from_card(&instant("Unrelated Card"), bob, Zone::Hand);
    let spliced_stable = game.object(spliced).unwrap().stable_id;
    game.push_to_stack(StackEntry::new(spell, bob).with_spliced_cards(vec![spliced_stable]));
    let meddling = game.create_object_from_definition(&definition, alice, Zone::Stack);
    let program = definition.spell_effect.as_ref().expect("spell effect");
    let mut ctx = ironsmith::effects::EffectContext::new_default(meddling, alice)
        .with_targets(vec![ironsmith::effects::ResolvedTarget::Object(spell)]);
    for effect in program.all_effects() {
        ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
    }
    drop(ctx);
    let hand = &game.player(bob).unwrap().hand;
    assert_eq!(hand.len(), 1, "both cards named like the spliced card are discarded: {hand:?}");
    assert_eq!(hand[0], kept);
}

#[test]
fn p7_combat_winters_chill_caps_x_and_tiers_each_controllers_payment() {
    let definition = compile_to_runtime_definition("Winter's Chill",
        "Mana cost: {X}{U}\nType: Instant\nCast this spell only during combat before blockers are declared.\nX can't be greater than the number of snow lands you control.\nChoose X target attacking creatures. For each of those creatures, its controller may pay {1} or {2}. If that player doesn't, destroy that creature at end of combat. If that player pays only {1}, prevent all combat damage that would be dealt to and dealt by that creature this combat.", false)
        .expect("Winter's Chill should compile");
    let debug = format!("{definition:?}");
    assert!(debug.contains("ThisSpellXMaximum"), "X is capped as the spell is cast: {debug}");
    let cap = &debug[debug.find("ThisSpellXMaximum").unwrap()..];
    assert!(cap.contains("Count(") && cap.contains("Snow"), "by the number of snow lands you control: {cap}");
    let body = &debug[debug.find("ForEachTaggedEffect").expect("one loop over the chosen attackers")..];
    let greater = body.find("Generic(2)").expect("the {2} offer");
    let lesser = body.find("Generic(1)").expect("the {1} offer");
    assert!(greater < lesser, "the greater payment is offered first, the lesser after declining it: {body}");
    assert!(body.contains("DidNotHappen") && body.contains("Happened"),
        "both the declined and the pays-only-{{1}} outcomes are gated inside the loop: {body}");
    assert!(body.contains("DestroyEffect") && body.contains("ScheduleDelayedTriggerEffect"),
        "declining both destroys that creature at end of combat: {body}");
    assert_eq!(debug.matches("ForEachTaggedEffect").count(), 1, "one loop: {debug}");
}

#[test]
fn p7_repl_sokrates_grants_combat_damage_prevention_with_split_draws() {
    let definition = compile_to_runtime_definition("Sokrates, Athenian Teacher",
        "Mana cost: {1}{W}{U}\nType: Legendary Creature — Human Advisor\nPower/Toughness: 1/4\nDefender\nSokrates has hexproof as long as it's untapped.\nSokratic Dialogue — {T}: Until end of turn, target creature gains \"If this creature would deal combat damage to a player, prevent that damage. This creature's controller and that player each draw half that many cards, rounded down.\"", false)
        .expect("Sokrates should compile");
    let debug = format!("{definition:?}");
    assert!(debug.contains("PreventMatchingDamageWithFollowUp"), "{debug}");
    assert!(debug.contains("combat_only: true"), "only combat damage is prevented: {debug}");
    assert!(debug.contains("HalfRoundedDown"), "half that many, rounded down: {debug}");
    assert!(debug.contains("DamagedPlayer"), "that player is the damaged player: {debug}");
    assert!(debug.matches("DrawCardsEffect").count() >= 2, "both players draw: {debug}");
}

#[test]
fn p7_combat_reclamation_charges_each_black_attacker_a_land_sacrifice() {
    let definition = compile_to_runtime_definition("Reclamation",
        "Mana cost: {2}{G}{W}\nType: Enchantment\nBlack creatures can't attack unless their controller sacrifices a land of their choice for each black creature they control that's attacking. (This cost is paid as attackers are declared.)", false)
        .expect("Reclamation should compile");
    let debug = format!("{definition:?}");
    assert!(debug.contains("BlockCost") && debug.contains("attack_only: true"),
        "an attack-only declaration cost on the restricted creatures: {debug}");
    assert!(debug.contains("Black"), "only black creatures are taxed: {debug}");
    assert!(debug.contains("Sacrifice") && debug.contains("Land"),
        "each black attacker costs one land sacrifice: {debug}");
    assert!(!debug.contains("RepeatEffectsEffect"), "no per-declaration repeat loop: {debug}");
}

#[test]
fn p7_stalking_leonin_remembers_the_secret_opponent_for_its_reveal_ability() {
    use ironsmith::ability::AbilityKind;
    use ironsmith::ids::PlayerId;
    use ironsmith::zone::Zone;
    let definition = compile_to_runtime_definition("Stalking Leonin",
        "Mana cost: {2}{W}\nType: Creature — Cat Archer\nPower/Toughness: 3/3\nWhen this creature enters, secretly choose an opponent.\nReveal the player you chose: Exile target creature that's attacking you if it's controlled by the chosen player. Activate only once.", false)
        .expect("Stalking Leonin should compile");
    let activated = definition.abilities.iter().find_map(|ability| match &ability.kind {
        AbilityKind::Activated(activated) => Some(activated),
        _ => None,
    }).expect("reveal ability");
    let activated_debug = format!("{activated:?}");
    assert!(activated_debug.contains("ChosenPlayer"), "the exile is gated on the chosen player: {activated_debug}");
    assert!(activated_debug.contains("Generic(0)"), "revealing costs nothing: {activated_debug}");
    let triggered = definition.abilities.iter().find_map(|ability| match &ability.kind {
        AbilityKind::Triggered(triggered) => Some(triggered),
        _ => None,
    }).expect("entry trigger");
    assert!(format!("{:?}", triggered.effects).contains("remember_as_chosen_player: true"));

    let mut game = ironsmith::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let leonin = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let mut ctx = ironsmith::effects::EffectContext::new_default(leonin, alice);
    for effect in triggered.effects.all_effects() {
        ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
    }
    assert_eq!(game.chosen_player(leonin), Some(bob), "the only opponent is the chosen player");
}

#[test]
fn p7_repl_suffocation_reads_red_spell_damage_history_and_hits_its_controller() {
    use ironsmith::card::CardBuilder;
    use ironsmith::events::{DamageEvent, DamageTarget, EventCause};
    use ironsmith::ids::{CardId, PlayerId};
    use ironsmith::mana::{ManaCost, ManaSymbol};
    use ironsmith::types::CardType;
    use ironsmith::zone::Zone;
    let definition = compile_to_runtime_definition("Suffocation",
        "Mana cost: {1}{U}\nType: Instant\nCast this spell only if you were dealt damage this turn by a red instant or sorcery spell.\nSuffocation deals 4 damage to the controller of the last red instant or sorcery spell that dealt damage to you this turn.\nDraw a card at the beginning of the next turn's upkeep.", false)
        .expect("Suffocation should compile");
    let debug = format!("{definition:?}");
    assert!(debug.contains("ThisSpellCastRestriction") && debug.contains("DamageHistory"),
        "the cast restriction reads this turn's damage history: {debug}");
    assert!(debug.contains("TagLastDamageSourceEffect") && debug.contains("ControllerOf(Tagged("),
        "damage goes to the last matching source's controller: {debug}");

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = ironsmith::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let red_instant = CardBuilder::new(CardId::new(), "Red Burn Witness")
        .card_types(vec![CardType::Instant])
        .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Red])).build();
    let burn = game.create_object_from_card(&red_instant, bob, Zone::Stack);
    let snapshot = ironsmith::snapshot::ObjectSnapshot::from_object_id(&game, burn).unwrap();
    let damage = ironsmith::triggers::TriggerEvent::new_with_provenance(
        DamageEvent::with_cause(burn, DamageTarget::Player(alice), 3, false, EventCause::effect()),
        Default::default());
    game.turn_store.turn_history.record_event(&damage, None, Some(snapshot));
    let suffocation = game.create_object_from_definition(&definition, alice, Zone::Stack);
    let program = definition.spell_effect.as_ref().expect("spell effect");
    let mut ctx = ironsmith::effects::EffectContext::new_default(suffocation, alice);
    for effect in program.all_effects() {
        ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
    }
    drop(ctx);
    assert_eq!(game.player(bob).unwrap().life, 16, "the red spell's controller is dealt 4 damage");
    assert_eq!(game.player(alice).unwrap().life, 20);
}

#[test]
fn p7_selective_obliteration_exiles_permanents_off_their_controllers_color() {
    use ironsmith::card::{CardBuilder, PowerToughness};
    use ironsmith::ids::{CardId, PlayerId};
    use ironsmith::mana::{ManaCost, ManaSymbol};
    use ironsmith::types::CardType;
    use ironsmith::zone::Zone;
    let definition = compile_to_runtime_definition("Selective Obliteration",
        "Mana cost: {3}{C}{C}\nType: Sorcery\nEach player chooses a color. Then exile each permanent unless it's colorless or it's only the color its controller chose.", false)
        .expect("Selective Obliteration should compile");
    let debug = format!("{:?}", definition.spell_effect);
    assert!(debug.contains("__player_option_choice__:white") && debug.contains("any_of"), "{debug}");
    let mut game = ironsmith::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let card = |name: &str, pips: Vec<Vec<ManaSymbol>>| CardBuilder::new(CardId::new(), name)
        .mana_cost(ManaCost::from_pips(pips)).card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1)).build();
    let white = game.create_object_from_card(&card("White", vec![vec![ManaSymbol::White]]), alice, Zone::Battlefield);
    let red = game.create_object_from_card(&card("Red", vec![vec![ManaSymbol::Red]]), alice, Zone::Battlefield);
    let azorius = game.create_object_from_card(&card("Azorius", vec![vec![ManaSymbol::White], vec![ManaSymbol::Blue]]), bob, Zone::Battlefield);
    let colorless = game.create_object_from_card(&card("Golem", vec![vec![ManaSymbol::Generic(2)]]), bob, Zone::Battlefield);
    game.refresh_continuous_state().unwrap();
    let spell = game.create_object_from_definition(&definition, alice, Zone::Stack);
    // The default decision maker picks the first option: both players choose white.
    let mut ctx = ironsmith::effects::EffectContext::new_default(spell, alice);
    for effect in definition.spell_effect.as_ref().unwrap().all_effects() {
        ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
    }
    let on_battlefield = |id| game.object(id).is_some_and(|object| object.zone == Zone::Battlefield);
    assert!(on_battlefield(white), "only its controller's chosen color");
    assert!(on_battlefield(colorless), "colorless permanents stay");
    assert!(!on_battlefield(red) && !on_battlefield(azorius), "another color is exiled");
}

#[test]
fn p7_piles_whims_of_the_fates_each_player_makes_three_piles_and_sacrifices_one_at_random() {
    let definition = compile_to_runtime_definition("Whims of the Fates",
        "Mana cost: {5}{R}\nType: Sorcery\nStarting with you, each player separates all permanents they control into three piles. Then each player chooses one of their piles at random and sacrifices those permanents. (Piles can be empty.)", false)
        .expect("Whims of the Fates should compile");
    let debug = format!("{:?}", definition.spell_effect);
    assert!(debug.matches("ChooseObjectsEffect").count() >= 2,
        "each player picks the members of two piles; the third is the rest: {debug}");
    assert!(debug.contains("ChooseModeEffect") && debug.contains("random: true"),
        "the sacrificed pile is chosen at random, not by a player: {debug}");
    assert!(debug.contains("\"Pile 1\"") && debug.contains("\"Pile 2\"") && debug.contains("\"Pile 3\""),
        "three pile modes: {debug}");
    assert!(debug.contains("Sacrifice"), "the chosen pile is sacrificed: {debug}");
    assert!(debug.contains("IteratedPlayer"), "piles belong to each separating player: {debug}");
}

#[test]
fn p7_piles_memories_returning_alternates_hand_and_bottom_picks_from_the_revealed_five() {
    use ironsmith::card::CardBuilder;
    use ironsmith::ids::{CardId, ObjectId, PlayerId};
    use ironsmith::types::CardType;
    use ironsmith::zone::Zone;
    let definition = compile_to_runtime_definition("Memories Returning",
        "Mana cost: {2}{U}{U}\nType: Sorcery\nReveal the top five cards of your library. Put one of them into your hand. Then choose an opponent. They put one on the bottom of your library. Then you put one into your hand. Then they put one on the bottom of your library. Put the other into your hand.\nFlashback {7}{U}{U}", false)
        .expect("Memories Returning should compile");
    let debug = format!("{:?}", definition.spell_effect);
    assert!(debug.contains("ChoosePlayerEffect"), "an opponent is chosen: {debug}");
    assert!(debug.matches("ChooseObjectsEffect").count() >= 4, "four alternating picks: {debug}");

    let mut game = ironsmith::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let card = CardBuilder::new(CardId::new(), "Memory witness").card_types(vec![CardType::Land]).build();
    for _ in 0..7 { game.create_object_from_card(&card, alice, Zone::Library); }
    let mut ctx = ironsmith::effects::EffectContext::new_default(ObjectId::from_raw(9_999), alice);
    for effect in definition.spell_effect.as_ref().expect("spell effect").all_effects() {
        ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
    }
    assert_eq!(game.player(alice).unwrap().hand.len(), 3,
        "you keep the first pick, the third pick and the last card");
    assert_eq!(game.player(alice).unwrap().library.len(), 4,
        "the opponent's two picks go back to the bottom of your library");
}

#[test]
fn p7_piles_phyrexian_portal_opponent_splits_ten_face_down_you_exile_one_and_search_the_other() {
    let definition = compile_to_runtime_definition("Phyrexian Portal",
        "Mana cost: {3}\nType: Artifact\n{3}: If your library has ten or more cards in it, target opponent looks at the top ten cards of your library and separates them into two face-down piles. Exile one of those piles. Search the other pile for a card, put it into your hand, then shuffle the rest of that pile into your library.", false)
        .expect("Phyrexian Portal should compile");
    let debug = format!("{definition:?}");
    assert!(debug.contains("CardsInLibrary") && debug.contains("GreaterThanOrEqual"),
        "gated on ten or more cards in your library: {debug}");
    assert!(debug.contains("\"Exile face-down pile 1\"") && debug.contains("\"Exile face-down pile 2\""),
        "you choose which face-down pile to exile: {debug}");
    assert!(debug.contains("ShuffleLibraryEffect"), "the rest is shuffled back: {debug}");
    assert!(debug.contains("Target(") && debug.contains("Opponent"), "the separator is a target opponent: {debug}");
}

#[test]
fn p7_piles_raging_river_labels_defender_piles_and_restricts_blockers_per_attacker() {
    let definition = compile_to_runtime_definition("Raging River",
        "Mana cost: {R}{R}\nType: Enchantment\nWhenever one or more creatures you control attack, each defending player divides all creatures without flying they control into a \"left\" pile and a \"right\" pile. Then, for each attacking creature you control, choose \"left\" or \"right.\" That creature can't be blocked this combat except by creatures with flying and creatures in a pile with the chosen label.", false)
        .expect("Raging River should compile");
    let debug = format!("{definition:?}");
    assert!(debug.contains("Defending"), "each defending player divides: {debug}");
    assert!(debug.contains("BlockSpecificAttacker") && debug.contains("EndOfCombat"),
        "a per-attacker blocking restriction for this combat: {debug}");
    assert!(debug.contains("excluded_static_abilities: [Flying]"),
        "flyers are never in a pile and may always block: {debug}");
    assert!(debug.contains("ForEachObjectEffect") || debug.contains("ForEachObject"),
        "one label choice per attacking creature: {debug}");
}

#[test]
fn p7_piles_parallel_thoughts_draw_replacement_takes_a_random_card_of_the_exiled_pile() {
    let definition = compile_to_runtime_definition("Parallel Thoughts",
        "Mana cost: {3}{U}{U}\nType: Enchantment\nWhen this enchantment enters, search your library for seven cards, exile them in a face-down pile, and shuffle that pile. Then shuffle your library.\nIf you would draw a card, you may instead put the top card of the pile you exiled into your hand.", false)
        .expect("Parallel Thoughts should compile");
    let debug = format!("{definition:?}");
    assert!(debug.contains("face_down: true"), "the seven cards are exiled face down: {debug}");
    assert!(debug.contains("ConditionalDrawReplacement") && debug.contains("optional: true"),
        "an optional draw replacement: {debug}");
    assert!(debug.contains("__source_exiled__") && debug.contains("random: true"),
        "the shuffled face-down pile's top card is a random card exiled with this: {debug}");
}

#[test]
fn p7_piles_memory_vessel_each_player_may_play_only_their_own_exiled_cards() {
    let definition = compile_to_runtime_definition("Memory Vessel",
        "Mana cost: {3}{R}{R}\nType: Artifact\n{T}, Exile this artifact: Each player exiles the top seven cards of their library. Until your next turn, players may play cards they exiled this way, and they can't play cards from their hand. Activate only as a sorcery.", false)
        .expect("Memory Vessel should compile");
    let debug = format!("{definition:?}");
    assert!(debug.contains("ExileTopOfLibraryEffect"), "each player exiles seven: {debug}");
    assert!(debug.contains("GrantPlayTaggedEffect") && debug.contains("UntilYourNextTurnStart"),
        "play permission until your next turn: {debug}");
    assert!(debug.contains("IteratedPlayer"), "each player is granted only their own cards: {debug}");
    assert!(debug.contains("PlayLandsMatching") && debug.contains("CastSpellsMatching"),
        "no plays from hand: {debug}");
}

#[test]
fn p7_repl_strict_proctor_watches_entry_caused_triggers_and_taxes_that_ability() {
    use ironsmith::events::{AbilityTriggerZoneChangeCause, AbilityTriggeredEvent, EventKind};
    use ironsmith::ids::PlayerId;
    use ironsmith::triggers::{TriggerEvent, TriggerIdentity, check_triggers};
    use ironsmith::zone::Zone;
    let definition = compile_to_runtime_definition("Strict Proctor",
        "Mana cost: {1}{W}\nType: Creature — Spirit Cleric\nPower/Toughness: 1/3\nFlying\nWhenever a permanent entering causes a triggered ability to trigger, counter that ability unless its controller pays {2}.", false)
        .expect("Strict Proctor should compile");
    let debug = format!("{definition:?}");
    assert!(debug.contains("caused_by_permanent_entering: true"), "{debug}");
    assert!(debug.contains("CounterEffect") && debug.contains("UnlessPays"), "{debug}");

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = ironsmith::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let other = ironsmith::cards::builders::CardDefinitionBuilder::new(ironsmith::ids::CardId::new(), "Trigger Host")
        .card_types(vec![ironsmith::types::CardType::Creature])
        .power_toughness(ironsmith::card::PowerToughness::fixed(1, 1)).build();
    let host = game.create_object_from_definition(&other, bob, Zone::Battlefield);
    let entrant = game.create_object_from_definition(&other, bob, Zone::Battlefield);
    let host_stable = game.object(host).unwrap().stable_id;
    game.refresh_continuous_state().unwrap();
    // Another permanent's ability triggered on a third permanent entering.
    let entry_caused = TriggerEvent::new_with_provenance(
        AbilityTriggeredEvent::new(host, host_stable, bob, TriggerIdentity(11)).with_cause(
            EventKind::ZoneChange,
            Some(entrant),
            Some(AbilityTriggerZoneChangeCause { from: Zone::Hand, to: Zone::Battlefield, destination_objects: vec![entrant] }),
        ),
        Default::default());
    let damage_caused = TriggerEvent::new_with_provenance(
        AbilityTriggeredEvent::new(host, host_stable, bob, TriggerIdentity(11))
            .with_cause(EventKind::Damage, Some(host), None),
        Default::default());
    assert_eq!(check_triggers(&game, &entry_caused).len(), 1, "an entry-caused trigger is watched");
    assert!(check_triggers(&game, &damage_caused).is_empty(), "other trigger causes are not");
}

#[test]
fn p7_combat_midnight_crusader_shuttle_defending_player_choice_and_stolen_attacker() {
    let definition = compile_to_runtime_definition("Midnight Crusader Shuttle",
        "Mana cost: {4}\nType: Artifact — Vehicle\nMidnight Entity — Whenever this Vehicle attacks, defending player faces a villainous choice — That player sacrifices a creature of their choice, or you gain control of a creature of your choice that player controls until end of turn. If you gain control of a creature this way, tap it, and it's attacking that player.\nCrew 2", false)
        .expect("Midnight Crusader Shuttle should compile");
    let debug = format!("{definition:?}");
    assert!(debug.contains("VillainousChoiceEffect"),
        "the defending player makes a villainous choice, not the controller: {debug}");
    assert!(debug.contains("Defending"), "the choice is bound to the defending player: {debug}");
    let choice = &debug[debug.find("VillainousChoiceEffect").unwrap()..];
    assert!(choice.contains("TapEffect") && choice.contains("ReselectAttackTargetEffect"),
        "the gain-control branch taps the creature and puts it into combat: {choice}");
    assert!(choice.contains("enters_combat: true"),
        "the stolen creature becomes an attacking creature attacking that player: {choice}");
}

#[test]
fn p7_toymakers_trap_excludes_earlier_numbers_and_scores_the_guess() {
    use ironsmith::ability::AbilityKind;
    use ironsmith::decisions::context::NumberContext;
    use ironsmith::game_state::GameState;
    use ironsmith::ids::PlayerId;
    use ironsmith::zone::Zone;
    struct Guess(u32);
    impl ironsmith::decision::DecisionMaker for Guess {
        fn decide_number(&mut self, _: &GameState, _: &NumberContext) -> u32 { self.0 }
    }
    let definition = compile_to_runtime_definition("The Toymaker's Trap",
        "Mana cost: {2}{B}\nType: Enchantment\nAt the beginning of your upkeep, secretly choose a number between 1 and 5 that hasn't been chosen. If you do, an opponent guesses which number you chose, then you reveal the number you chose. If they guessed wrong, they lose life equal to the number they guessed and you draw a card. If they guessed right, sacrifice this enchantment.", false)
        .expect("The Toymaker's Trap should compile");
    let triggered = definition.abilities.iter().find_map(|ability| match &ability.kind {
        AbilityKind::Triggered(triggered) => Some(triggered),
        _ => None,
    }).expect("upkeep trigger");
    let debug = format!("{:?}", triggered.effects);
    assert!(debug.contains("exclude_source_history: true") && debug.contains("guess: true"), "{debug}");
    assert!(debug.contains("__player_number_guess__:wrong"), "{debug}");

    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let trap = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let resolve = |game: &mut GameState| {
        let mut dm = Guess(2);
        let mut ctx = ironsmith::effects::EffectContext::new(trap, alice, &mut dm);
        for effect in triggered.effects.all_effects() {
            ironsmith::effects::execute_effect(game, effect, &mut ctx).unwrap();
        }
    };
    // Alice takes the first available number (1); Bob guesses 2: wrong.
    resolve(&mut game);
    assert_eq!(game.player(bob).unwrap().life, 18, "a wrong guesser loses the number they guessed");
    assert_eq!(game.numbers_chosen_by_source(trap), &[1]);
    // 1 can't be chosen again, so Alice takes 2 and Bob's guess is right.
    resolve(&mut game);
    assert_eq!(game.player(bob).unwrap().life, 18);
    assert!(game.object(trap).is_none_or(|object| object.zone != Zone::Battlefield),
        "a right guess sacrifices the enchantment");
}

#[test]
fn p7_combat_sorrows_path_exchanges_the_attackers_two_blockers_block() {
    use ironsmith::card::{CardBuilder, PowerToughness};
    use ironsmith::combat_state::{AttackTarget, AttackerInfo, CombatState};
    use ironsmith::ids::{CardId, ObjectId, PlayerId};
    use ironsmith::types::CardType;
    use ironsmith::zone::Zone;
    let definition = compile_to_runtime_definition("Sorrow's Path",
        "Type: Land\n{T}: Choose two target blocking creatures controlled by the same opponent. If each of those creatures could block all creatures that the other is blocking, remove both of them from combat. Each one then blocks all creatures the other was blocking.\nWhenever this land becomes tapped, it deals 2 damage to you and each creature you control.", false)
        .expect("Sorrow's Path should compile");
    let debug = format!("{definition:?}");
    assert!(debug.contains("ExchangeBlocksEffect"), "one block exchange: {debug}");
    assert!(debug.contains("target_set_same_controller: true"),
        "both targets share a controller: {debug}");

    // Behavior: the exchange swaps which attackers each blocker blocks.
    let mut game = ironsmith::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let bear = |name: &str| CardBuilder::new(CardId::new(), name).card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2)).build();
    let attacker_a = game.create_object_from_card(&bear("A1"), alice, Zone::Battlefield);
    let attacker_b = game.create_object_from_card(&bear("A2"), alice, Zone::Battlefield);
    let blocker_a = game.create_object_from_card(&bear("B1"), bob, Zone::Battlefield);
    let blocker_b = game.create_object_from_card(&bear("B2"), bob, Zone::Battlefield);
    game.turn.phase = ironsmith::game_state::Phase::Combat;
    game.turn.step = Some(ironsmith::game_state::Step::DeclareBlockers);
    let mut combat = CombatState::default();
    combat.attackers.push(AttackerInfo { creature: attacker_a, target: AttackTarget::Player(bob) });
    combat.attackers.push(AttackerInfo { creature: attacker_b, target: AttackTarget::Player(bob) });
    combat.blockers.insert(attacker_a, vec![blocker_a]);
    combat.blockers.insert(attacker_b, vec![blocker_b]);
    game.combat = Some(combat);
    game.refresh_continuous_state().unwrap();
    let effect = ironsmith::effects::ExchangeBlocksEffect::new(ironsmith::target::ChooseSpec::All(
        ironsmith::target::ObjectFilter::creature()
            .in_zone(Zone::Battlefield)
            .controlled_by(ironsmith::target::PlayerFilter::Specific(bob)),
    ));
    let mut ctx = ironsmith::effects::EffectContext::new_default(ObjectId::from_raw(9_999), alice);
    ironsmith::effects::execute_effect(&mut game, &ironsmith::effect::Effect::new(effect), &mut ctx).unwrap();
    let combat = game.combat.as_ref().unwrap();
    assert_eq!(combat.blockers.get(&attacker_a), Some(&vec![blocker_b]));
    assert_eq!(combat.blockers.get(&attacker_b), Some(&vec![blocker_a]));
}

#[test]
fn p7_combat_snowblind_attack_state_gated_minus_x_minus_y() {
    let definition = compile_to_runtime_definition("Snowblind",
        "Mana cost: {3}{G}\nType: Enchantment — Aura\nEnchant creature\nEnchanted creature gets -X/-Y. If that creature is attacking, X is the number of snow lands defending player controls. Otherwise, X is the number of snow lands its controller controls. Y is equal to X or to enchanted creature's toughness minus 1, whichever is smaller.", false)
        .expect("Snowblind should compile");
    let debug = format!("{definition:?}");
    assert!(debug.matches("Anthem(Anthem").count() >= 2,
        "one modification while attacking, one otherwise: {debug}");
    assert!(debug.contains("Defending"), "attacking: the defending player's snow lands: {debug}");
    assert!(debug.contains("ControllerOf(Tagged(TagKey(\\\"enchanted\\\")))") || debug.contains("ControllerOf(Tagged(TagKey(\"enchanted\")))"),
        "otherwise: the enchanted creature's controller's snow lands: {debug}");
    assert!(debug.contains("ToughnessOf(Iterated)") && debug.contains("Min("),
        "Y = min(X, the creature's toughness - 1): {debug}");
    assert!(debug.contains("AttachedToSourceMatches") && debug.contains("Not("),
        "gated on whether the enchanted creature is attacking: {debug}");
}

#[test]
fn p7_mindblaze_reveals_target_library_and_counts_the_chosen_name_against_the_chosen_number() {
    use ironsmith::card::{CardBuilder, PowerToughness};
    use ironsmith::decisions::context::{NumberContext, TextInputContext};
    use ironsmith::game_state::GameState;
    use ironsmith::ids::{CardId, PlayerId};
    use ironsmith::types::CardType;
    use ironsmith::zone::Zone;
    struct NameAndNumber;
    impl ironsmith::decision::DecisionMaker for NameAndNumber {
        fn decide_text(&mut self, _: &GameState, _: &TextInputContext) -> String {
            "Grizzly Bears".to_string()
        }
        fn decide_number(&mut self, _: &GameState, _: &NumberContext) -> u32 { 2 }
    }
    let definition = compile_to_runtime_definition("Mindblaze",
        "Mana cost: {5}{R}\nType: Sorcery\nChoose a nonland card name and a number greater than 0. Target player reveals their library. If that library contains exactly the chosen number of cards with the chosen name, Mindblaze deals 8 damage to that player. Then that player shuffles.", false)
        .expect("Mindblaze should compile");
    let debug = format!("{:?}", definition.spell_effect);
    assert!(debug.contains("ChooseCardNameEffect") && debug.contains("ChooseNumber"),
        "the coordinated sentence makes two independent choices: {debug}");
    assert!(!debug.contains("ChooseObjectsEffect"), "no object choice is invented: {debug}");
    assert!(debug.contains("RevealTaggedEffect") && debug.contains("{chosen name}"), "{debug}");
    assert_eq!(debug.matches("TargetOnlyEffect").count(), 1, "one player target: {debug}");

    let program = definition.spell_effect.as_ref().expect("spell effect");
    let run = |bears_in_library: usize| -> i32 {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let bears = CardBuilder::new(CardId::new(), "Grizzly Bears").card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2)).build();
        let other = CardBuilder::new(CardId::new(), "Hill Giant").card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(3, 3)).build();
        for _ in 0..bears_in_library {
            game.create_object_from_card(&bears, bob, Zone::Library);
        }
        game.create_object_from_card(&other, bob, Zone::Library);
        let source = game.create_object_from_definition(&definition, alice, Zone::Hand);
        let mut dm = NameAndNumber;
        let mut ctx = ironsmith::effects::EffectContext::new(source, alice, &mut dm)
            .with_targets(vec![ironsmith::effects::ResolvedTarget::Player(bob)]);
        for effect in program.all_effects() {
            ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
        }
        game.player(bob).unwrap().life
    };
    assert_eq!(run(2), 12, "exactly the chosen number of named cards: 8 damage to the target");
    assert_eq!(run(3), 20, "a different count deals no damage");
    assert_eq!(run(1), 20, "a different count deals no damage");
}

#[test]
fn p7_toymaker_end_step_drains_once_per_spell_or_ability_that_made_piles_or_guesses() {
    use ironsmith::ability::AbilityKind;
    use ironsmith::game_state::GameState;
    use ironsmith::ids::{ObjectId, PlayerId};
    use ironsmith::zone::Zone;
    let definition = compile_to_runtime_definition("The Celestial Toymaker",
        "Mana cost: {W}{U}{B}\nType: Legendary Creature — Rogue Artificer\nPower/Toughness: 3/3\nWhenever The Celestial Toymaker attacks, look at the top three cards of your library. Exile any number of them in a face-down pile and the rest in a face-up pile. Defending player chooses one of those piles. Put that pile into your hand.\nAt the beginning of each end step, each opponent loses 2 life for each spell or ability that caused one or more players to guess or to group cards or permanents into a pile this turn.", false)
        .expect("The Celestial Toymaker should compile");
    let triggers = definition.abilities.iter().filter_map(|ability| match &ability.kind {
        AbilityKind::Triggered(triggered) => Some(triggered),
        _ => None,
    }).collect::<Vec<_>>();
    assert_eq!(triggers.len(), 2, "attack and end-step triggers");
    let attack = triggers.iter().copied().find(|triggered| format!("{:?}", triggered.effects).contains("GroupIntoPiles"))
        .expect("the attack trigger's pile program records that it grouped cards into piles");
    let end_step = triggers.iter().copied()
        .find(|triggered| format!("{:?}", triggered.effects).contains("SpellsOrAbilitiesThatMadePilesOrGuessesThisTurn"))
        .expect("the end-step drain counts pile/guess spells and abilities");
    assert!(format!("{:?}", end_step.effects).contains("LoseLife"), "{:?}", end_step.effects);

    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let toymaker = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let drain = |game: &mut GameState| {
        let mut ctx = ironsmith::effects::EffectContext::new_default(toymaker, alice);
        for effect in end_step.effects.all_effects() {
            ironsmith::effects::execute_effect(game, effect, &mut ctx).unwrap();
        }
    };
    drain(&mut game);
    assert_eq!(game.player(bob).unwrap().life, 20, "nothing made piles or guesses yet");

    let records = attack.effects.all_effects().into_iter()
        .filter(|effect| format!("{effect:?}").contains("GroupIntoPiles"))
        .collect::<Vec<_>>();
    assert!(!records.is_empty());
    assert!(records.iter().all(|effect| !format!("{effect:?}").contains("ChooseOneOf")),
        "the pile record is its own top-level instruction: {records:?}");
    // One resolution that groups twice still counts once; a second
    // resolution (a later object-id floor) counts again.
    for floor in [1_000, 2_000] {
        let mut ctx = ironsmith::effects::EffectContext::new_default(toymaker, alice);
        ctx.resolution_object_id_floor = Some(ObjectId::from_raw(floor));
        for _ in 0..2 {
            for effect in records.iter().copied() {
                ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
            }
        }
    }
    drain(&mut game);
    assert_eq!(game.player(bob).unwrap().life, 16, "2 life for each of the two pile-making resolutions");
    assert_eq!(game.player(alice).unwrap().life, 20, "only opponents lose life");
}

#[test]
fn p7_space_beleren_sectors_restrict_blocks_and_choose_a_sector_for_counters_and_destruction() {
    use ironsmith::ability::AbilityKind;
    use ironsmith::card::{CardBuilder, PowerToughness};
    use ironsmith::ids::{CardId, PlayerId};
    use ironsmith::marker::SectorDesignation;
    use ironsmith::object::CounterType;
    use ironsmith::static_abilities::StaticAbilityId;
    use ironsmith::types::CardType;
    use ironsmith::zone::Zone;
    let definition = compile_to_runtime_definition("Space Beleren",
        "Mana cost: {2}{W}{U}\nType: Legendary Planeswalker — Jace\nLoyalty: 3\nSpace sculptor (Space Beleren divides the battlefield into alpha, beta, and gamma sectors. If a creature isn't assigned to a sector, its controller assigns it to one. Opponents assign first.)\n+1: Creatures in each sector can be blocked this turn only by creatures in the same sector.\n−1: Put a +1/+1 counter on each creature in the sector of your choice.\n−5: Destroy all creatures in the sector of your choice.", false)
        .expect("Space Beleren should compile");
    assert!(definition.abilities.iter().any(|ability| matches!(&ability.kind,
        AbilityKind::Static(ability) if ability.id() == StaticAbilityId::SpaceSculptor)),
        "\"Space sculptor\" is the keyword, not a self-reference: {:#?}", definition.abilities);
    let activated: Vec<_> = definition.abilities.iter().filter_map(|ability| match &ability.kind {
        AbilityKind::Activated(activated) => Some(activated),
        _ => None,
    }).collect();
    assert_eq!(activated.len(), 3, "three loyalty abilities: {:#?}", definition.abilities);
    let plus = format!("{:?}", activated[0]);
    assert_eq!(plus.matches("BlockSpecificAttacker").count(), 3, "one restriction per sector: {plus}");
    for sector in ["Alpha", "Beta", "Gamma"] {
        assert!(plus.contains(&format!("In({sector})")) && plus.contains(&format!("NotIn({sector})")),
            "{sector}: its creatures are blocked only from inside it: {plus}");
    }
    for ability in &activated[1..] {
        let debug = format!("{ability:?}");
        assert!(debug.contains("ChooseNamedOptionEffect") && debug.contains("\"alpha\"")
            && debug.contains("ChosenBySource"), "a sector is chosen, then read: {debug}");
    }

    let mut game = ironsmith::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let bear = CardBuilder::new(CardId::new(), "Bear").card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2)).build();
    let attacker = game.create_object_from_card(&bear, alice, Zone::Battlefield);
    let alpha_blocker = game.create_object_from_card(&bear, bob, Zone::Battlefield);
    let beta_blocker = game.create_object_from_card(&bear, bob, Zone::Battlefield);
    assert!(game.set_sector_designation(attacker, SectorDesignation::Alpha));
    assert!(game.set_sector_designation(alpha_blocker, SectorDesignation::Alpha));
    assert!(game.set_sector_designation(beta_blocker, SectorDesignation::Beta));
    game.refresh_continuous_state().unwrap();

    let run = |game: &mut ironsmith::game_state::GameState, index: usize| {
        let mut ctx = ironsmith::effects::EffectContext::new_default(source, alice);
        for effect in activated[index].effects.all_effects() {
            // The sector choice is preset below; it is not part of this check.
            if effect.downcast_ref::<ironsmith::effects::ChooseNamedOptionEffect>().is_some() {
                continue;
            }
            ironsmith::effects::execute_effect(game, effect, &mut ctx).unwrap();
        }
        game.refresh_continuous_state().unwrap();
        game.update_cant_effects();
    };
    let object = |game: &ironsmith::game_state::GameState, id| game.object(id).unwrap().clone();

    run(&mut game, 0);
    assert!(ironsmith::rules::combat::can_block(&object(&game, attacker), &object(&game, alpha_blocker), &game),
        "a creature in the same sector may block");
    assert!(!ironsmith::rules::combat::can_block(&object(&game, attacker), &object(&game, beta_blocker), &game),
        "a creature in another sector can't block");

    game.set_chosen_named_option(source, "alpha".to_string());
    run(&mut game, 1);
    assert_eq!(game.counter_count(attacker, CounterType::PlusOnePlusOne), 1);
    assert_eq!(game.counter_count(alpha_blocker, CounterType::PlusOnePlusOne), 1);
    assert_eq!(game.counter_count(beta_blocker, CounterType::PlusOnePlusOne), 0,
        "only the chosen sector's creatures get counters");

    game.set_chosen_named_option(source, "beta".to_string());
    run(&mut game, 2);
    assert!(!game.battlefield.contains(&beta_blocker), "the chosen sector's creatures are destroyed");
    assert!(game.battlefield.contains(&attacker) && game.battlefield.contains(&alpha_blocker));
}

#[test]
fn p7_takklemaggot_returns_attached_or_as_a_non_aura_bound_to_the_dead_creatures_controller() {
    use ironsmith::ability::AbilityKind;
    use ironsmith::card::{CardBuilder, PowerToughness};
    use ironsmith::ids::{CardId, PlayerId};
    use ironsmith::snapshot::ObjectSnapshot;
    use ironsmith::target::PlayerFilter;
    use ironsmith::types::{CardType, Subtype};
    use ironsmith::zone::Zone;
    let definition = compile_to_runtime_definition("Takklemaggot",
        "Mana cost: {2}{B}{B}\nType: Enchantment — Aura\nEnchant creature\nAt the beginning of the upkeep of enchanted creature's controller, put a -0/-1 counter on that creature.\nWhen enchanted creature dies, that creature's controller chooses a creature that this card could enchant. If the player does, return this card to the battlefield under your control attached to that creature. If they don't, return this card to the battlefield under your control as a non-Aura enchantment. It loses \"enchant creature\" and gains \"At the beginning of that player's upkeep, this enchantment deals 1 damage to that player.\"", false)
        .expect("Takklemaggot should compile");
    let dies = definition.abilities.iter().find_map(|ability| match &ability.kind {
        AbilityKind::Triggered(triggered) if format!("{:?}", triggered.trigger).contains("ZoneChangeTrigger") => Some(triggered),
        _ => None,
    }).expect("dies trigger");
    let debug = format!("{:?}", dies.effects);
    assert!(debug.contains("ChooseObjectsEffect") && debug.contains("could_be_enchanted_by_source: true"),
        "a non-target choice of a creature this Aura could enchant: {debug}");
    assert!(debug.contains("chooser: ControllerOf("), "the dead creature's controller chooses: {debug}");
    assert!(debug.contains("Happened") && debug.contains("DidNotHappen"), "both branches gate on the choice: {debug}");
    assert!(debug.contains("AttachObjectsEffect"), "the chosen branch returns attached: {debug}");
    assert!(debug.contains("enters_as_non_aura: true"), "\"as a non-Aura enchantment\" is not dropped: {debug}");
    assert!(debug.contains("BeginningOfUpkeepTrigger { player: ControllerOf("),
        "\"that player's upkeep\" names the chooser, not each player: {debug}");
    let otherwise = debug.split("DidNotHappen").nth(1).expect("the \"If they don't\" branch");
    assert!(otherwise.contains("enters_as_non_aura: true") && otherwise.contains("AddAbilityGeneric"),
        "the loses/gains sentence is folded into the \"If they don't\" branch: {debug}");

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let bear = CardBuilder::new(CardId::new(), "Dead bear").card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2)).build();
    let run = |with_host: bool| {
        let mut game = ironsmith::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let dead = game.create_object_from_card(&bear, bob, Zone::Graveyard);
        let host = with_host.then(|| game.create_object_from_card(&bear, alice, Zone::Battlefield));
        let aura = game.create_object_from_definition(&definition, alice, Zone::Graveyard);
        let mut ctx = ironsmith::effects::EffectContext::new_default(aura, alice);
        ctx.set_tagged_objects("triggering", vec![ObjectSnapshot::from_object_id(&game, dead).unwrap()]);
        for effect in dies.effects.all_effects() {
            if format!("{effect:?}").contains("TagTriggeringObjectEffect") {
                continue;
            }
            // "It loses ... and gains ..." lives inside the "If they don't"
            // branch, so neither branch leaves an effect without its object.
            if let Err(error) = ironsmith::effects::execute_effect(&mut game, effect, &mut ctx) {
                panic!("Takklemaggot dies trigger failed (with_host={with_host}): {error:?}");
            }
        }
        drop(ctx);
        game.refresh_continuous_state().unwrap();
        let returned = *game.battlefield.iter()
            .find(|id| game.object(**id).unwrap().name == "Takklemaggot")
            .expect("Takklemaggot returns to the battlefield");
        (game, returned, host)
    };

    let (game, returned, host) = run(true);
    let host = host.unwrap();
    assert_eq!(game.object(returned).unwrap().attached_to,
        Some(ironsmith::object::AttachmentTarget::Object(host)), "returns attached to the chosen creature");
    assert!(game.current_subtypes(returned).unwrap_or_default().contains(&Subtype::Aura));
    assert_eq!(game.controller_of(game.object(returned).unwrap()), alice, "under your control");

    let (game, returned, _) = run(false);
    let object = game.object(returned).unwrap();
    assert!(object.attached_to.is_none(), "no creature was chosen: it enters attached to nothing");
    assert!(!game.current_subtypes(returned).unwrap_or_default().contains(&Subtype::Aura),
        "it entered as a non-Aura enchantment");
    assert_eq!(game.controller_of(object), alice);
    let upkeep_players: Vec<PlayerFilter> = game.current_abilities(returned).unwrap().into_iter()
        .filter_map(|ability| match ability.kind {
            AbilityKind::Triggered(triggered) => triggered.trigger
                .downcast_ref::<ironsmith::triggers::BeginningOfUpkeepTrigger>()
                .map(|upkeep| upkeep.player.clone()),
            _ => None,
        })
        .collect();
    assert!(upkeep_players.contains(&PlayerFilter::Specific(bob)),
        "the granted trigger is bound to the player who chose (the dead creature's controller): {upkeep_players:?}");
}
