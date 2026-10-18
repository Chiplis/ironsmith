//! Claude package 31 runtime checks for the compilation-completion task
//! (see reports/compilation-completion-20261016/COORDINATION.md).
#![allow(unused_imports)]
use super::compile_to_runtime_definition;
use ironsmith::effect::Effect;
use ironsmith::ability::AbilityKind;
use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::ids::CardId;
use ironsmith::object::CounterType;
use ironsmith::target::ChooseSpec;
use ironsmith::types::{CardType, Subtype};
use ironsmith::{PlayerId, Zone};

/// Every effect reachable from a definition (spell, activated, triggered),
/// including nested children.
fn p31_effects(definition: &ironsmith::cards::CardDefinition) -> Vec<Effect> {
    fn walk(effect: &Effect, out: &mut Vec<Effect>) {
        out.push(effect.clone());
        effect.visit_child_effects(&mut |child| walk(child, out));
    }
    let mut out = Vec::new();
    if let Some(program) = definition.spell_effect.as_ref() {
        for effect in program.all_effects() { walk(effect, &mut out); }
    }
    for ability in &definition.abilities {
        match &ability.kind {
            AbilityKind::Activated(activated) => {
                for effect in activated.effects.all_effects() { walk(effect, &mut out); }
            }
            AbilityKind::Triggered(triggered) => {
                for effect in triggered.effects.all_effects() { walk(effect, &mut out); }
            }
            _ => {}
        }
    }
    out
}

fn p31_compile(name: &str, text: &str) -> ironsmith::cards::CardDefinition {
    let (definition, trace) = ironsmith_compiler::parse_trace::capture(|| {
        compile_to_runtime_definition(name, text, false)
    });
    definition.unwrap_or_else(|error| panic!("{name}: {error:?}\n{}", trace.render()))
}

fn p31_intervening_if(definition: &ironsmith::cards::CardDefinition) -> ironsmith::effect::Condition {
    definition.abilities.iter().find_map(|ability| match &ability.kind {
        AbilityKind::Triggered(trigger) => trigger.intervening_if.clone(),
        _ => None,
    }).expect("intervening-if condition")
}

#[test]
fn fire_and_brimstone_targets_only_players_who_attacked_this_turn() {
    let definition = p31_compile("Fire and Brimstone",
        "Mana cost: {3}{W}{W}\nType: Instant\nFire and Brimstone deals 4 damage to target player who attacked this turn and 4 damage to you.");
    let damage: Vec<_> = p31_effects(&definition).into_iter()
        .filter_map(|effect| effect.downcast_ref::<ironsmith::effects::DealDamageEffect>().cloned())
        .collect();
    assert_eq!(damage.len(), 2, "{damage:?}");
    assert!(format!("{:?}", damage[0].target).contains("TurnHistory(AttackedWithCreature)"),
        "the target qualifier is a target-legality turn-history fact: {:?}", damage[0].target);
    assert!(matches!(damage[1].target, ChooseSpec::SourceController), "{:?}", damage[1].target);
}

#[test]
fn fevered_visions_damages_only_opponents_with_four_or_more_cards() {
    let definition = p31_compile("Fevered Visions",
        "Mana cost: {1}{U}{R}\nType: Enchantment\nAt the beginning of each player's end step, that player draws a card. If the player is your opponent and has four or more cards in hand, this enchantment deals 2 damage to that player.");
    let debug = format!("{:?}", p31_effects(&definition));
    assert!(debug.contains("CountPlayers(Excluding"), "opponent check: {debug}");
    assert!(debug.contains("PlayerCardsInHandOrMore"), "hand-size check: {debug}");
    assert!(debug.contains("DealDamageEffect"), "{debug}");
}

#[test]
fn fearless_swashbuckler_requires_attacking_pirate_and_vehicle() {
    let definition = p31_compile("Fearless Swashbuckler",
        "Mana cost: {1}{U}{R}\nType: Creature — Fish Pirate\nPower/Toughness: 3/2\nHaste\nVehicles you control have haste.\nWhenever you attack, if a Pirate and a Vehicle attacked this combat, draw three cards, then discard two cards.");
    let debug = format!("{:?}", p31_intervening_if(&definition));
    assert!(debug.contains("Pirate") && debug.contains("Vehicle"), "{debug}");
    assert!(debug.contains("attacking: true"), "{debug}");
    assert!(debug.starts_with("And("), "both attackers are required: {debug}");
}

#[test]
fn firemane_commando_other_player_draws_only_when_not_attacking_you() {
    let definition = p31_compile("Firemane Commando",
        "Mana cost: {3}{W}\nType: Creature — Angel Soldier\nPower/Toughness: 4/3\nFlying\nWhenever you attack with two or more creatures, draw a card.\nWhenever another player attacks with two or more creatures, they draw a card if none of those creatures attacked you.");
    let triggers: Vec<_> = definition.abilities.iter().filter_map(|ability| match &ability.kind {
        AbilityKind::Triggered(trigger) => Some(format!("{:?}", trigger.effects)),
        _ => None,
    }).collect();
    assert_eq!(triggers.len(), 2);
    assert!(triggers[1].contains("attacking_player_only: true") && triggers[1].contains("Not("),
        "draw is gated on no creature attacking you: {}", triggers[1]);
}

#[test]
fn eye_of_singularity_spares_basic_lands_but_not_nonbasic_lands() {
    let definition = p31_compile("Eye of Singularity",
        "Mana cost: {3}{W}\nType: World Enchantment\nWhen this enchantment enters, destroy each permanent with the same name as another permanent, except for basic lands. They can't be regenerated.\nWhenever a permanent other than a basic land enters, destroy all other permanents with that name. They can't be regenerated.");
    let debug = format!("{definition:?}");
    assert!(debug.contains("excluded_supertypes: [Basic]"), "{debug}");
    assert!(!debug.contains("excluded_card_types: [Land]"),
        "a basic-land exception must not exclude nonbasic lands: {debug}");
}

#[test]
fn fear_of_sleep_paralysis_keeps_stun_counters_on_opponents_permanents() {
    let definition = p31_compile("Fear of Sleep Paralysis",
        "Mana cost: {5}{U}\nType: Enchantment Creature — Nightmare\nPower/Toughness: 6/6\nFlying\nEerie — Whenever this creature or another enchantment you control enters and whenever you fully unlock a Room, tap up to one target creature and put a stun counter on it.\nStun counters can't be removed from permanents your opponents control. (They won't untap if they have stun counters.)");
    assert!(format!("{definition:?}").contains("HaveCounterTypeRemoved"));
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let creature = CardBuilder::new(CardId::new(), "Stunned creature")
        .card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(2, 2)).build();
    let theirs = game.create_object_from_card(&creature, bob, Zone::Battlefield);
    let mine = game.create_object_from_card(&creature, alice, Zone::Battlefield);
    for id in [theirs, mine] {
        game.add_counters(id, CounterType::Stun, 2);
    }
    game.refresh_continuous_state().unwrap();
    assert!(game.remove_counters(theirs, CounterType::Stun, 1, None, None).is_none());
    assert_eq!(game.counter_count(theirs, CounterType::Stun), 2);
    assert!(game.remove_counters(mine, CounterType::Stun, 1, None, None).is_some());
    assert_eq!(game.counter_count(mine, CounterType::Stun), 1);
}

#[test]
fn fealty_to_the_realm_monarch_controls_enchanted_creature() {
    let definition = p31_compile("Fealty to the Realm",
        "Mana cost: {4}{U}\nType: Enchantment — Aura\nEnchant creature\nWhen this Aura enters, you become the monarch.\nThe monarch controls enchanted creature.\nEnchanted creature attacks each combat if able and can't attack you.");
    let debug = format!("{definition:?}");
    assert!(debug.contains("MonarchControlsAttachedPermanent"), "{debug}");
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let creature = CardBuilder::new(CardId::new(), "Enchanted creature")
        .card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(2, 2)).build();
    let enchanted = game.create_object_from_card(&creature, bob, Zone::Battlefield);
    let aura = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    assert!(game.attach_object_to_target(aura, ironsmith::object::AttachmentTarget::Object(enchanted)));
    game.refresh_continuous_state().unwrap();
    assert_eq!(game.current_controller(enchanted), Some(bob), "no monarch, no control change");
    assert!(game.set_monarch(Some(alice)).unwrap());
    game.refresh_continuous_state().unwrap();
    assert_eq!(game.current_controller(enchanted), Some(alice));
    assert!(game.set_monarch(Some(bob)).unwrap());
    game.refresh_continuous_state().unwrap();
    assert_eq!(game.current_controller(enchanted), Some(bob));
}

#[test]
fn flamewar_puts_owned_intel_exiled_cards_into_hand() {
    let definition = p31_compile("Flamewar, Brash Veteran",
        "Mana cost: {B}{R}\nType: Legendary Artifact Creature — Robot\nPower/Toughness: 3/2\nMore Than Meets the Eye {B}{R} (You may cast this card converted for {B}{R}.)\nSacrifice another artifact: Put a +1/+1 counter on Flamewar and convert it. Activate only as a sorcery.\n{1}, Discard your hand: Put all exiled cards you own with intel counters on them into your hand.");
    let activated: Vec<_> = definition.abilities.iter().filter_map(|ability| match &ability.kind {
        AbilityKind::Activated(activated) => Some(format!("{:?}", activated.effects)),
        _ => None,
    }).collect();
    let intel = activated.iter().find(|debug| debug.contains("intel")).expect("intel ability");
    assert!(intel.contains("Hand") && intel.contains("Exile"), "{intel}");
    assert!(!intel.contains("PutCounters"), "the counters only qualify the moved cards: {intel}");
}

#[test]
fn faramir_draws_only_if_chosen_opponent_did_not_attack_you() {
    let definition = p31_compile("Faramir, Prince of Ithilien",
        "Mana cost: {2}{W}{U}\nType: Legendary Creature — Human Noble\nPower/Toughness: 3/3\nAt the beginning of your end step, choose an opponent. At the beginning of that player's next end step, you draw a card if they didn't attack you that turn. Otherwise, create three 1/1 white Human Soldier creature tokens.");
    let debug = format!("{definition:?}");
    assert!(debug.contains("ActivePlayerAttackedControllerThisTurn"), "{debug}");
    assert!(debug.contains("Soldier"), "otherwise branch: {debug}");
}

#[test]
fn flash_sacrifices_unless_reduced_mana_cost_is_paid() {
    let definition = p31_compile("Flash",
        "Mana cost: {1}{U}\nType: Instant\nYou may put a creature card from your hand onto the battlefield. If you do, sacrifice it unless you pay its mana cost reduced by {2}.");
    let debug = format!("{:?}", p31_effects(&definition));
    assert!(debug.contains("UnlessPays"), "{debug}");
    assert!(debug.contains("generic_reduction: 2"), "{debug}");
    assert!(debug.contains("mana_cost_of: Some"), "its = the put creature: {debug}");
    // The reduction removes generic mana only (CR 601.2f).
    let cost = ironsmith::mana::ManaCost::from_symbols(vec![
        ironsmith::mana::ManaSymbol::Generic(3),
        ironsmith::mana::ManaSymbol::Blue,
    ]);
    assert_eq!(cost.reduce_generic(2).mana_value(), 2);
    assert_eq!(ironsmith::mana::ManaCost::from_symbols(vec![ironsmith::mana::ManaSymbol::Blue])
        .reduce_generic(2).mana_value(), 1);
}

#[test]
fn fblthp_the_lost_draws_two_when_entering_from_or_cast_from_library() {
    let definition = p31_compile("Fblthp, the Lost",
        "Mana cost: {1}{U}\nType: Legendary Creature — Homunculus\nPower/Toughness: 1/1\nWhen Fblthp enters, draw a card. If it entered from your library or was cast from your library, draw two cards instead.\nWhen Fblthp becomes the target of a spell, shuffle Fblthp into its owner's library.");
    let debug = format!("{definition:?}");
    assert!(debug.contains("TriggeringObjectEnteredFromZone(Library)"), "{debug}");
    assert!(debug.contains("TriggeringObjectWasCastFromZone(Library)"), "{debug}");
    let alice = PlayerId::from_index(0);
    for (origin, cast_origin, expected) in [
        (Zone::Hand, None, 1),
        (Zone::Library, None, 2),
        (Zone::Stack, Some(Zone::Hand), 1),
        (Zone::Stack, Some(Zone::Library), 2),
    ] {
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let witness = CardBuilder::new(CardId::new(), "Draw witness")
            .card_types(vec![CardType::Instant]).build();
        for _ in 0..5 { game.create_object_from_card(&witness, alice, Zone::Library); }
        let source = game.create_object_from_definition(&definition, alice, origin);
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        if let Some(cast_origin) = cast_origin {
            let snapshot = ironsmith::snapshot::ObjectSnapshot::from_object_id(&game, source).unwrap();
            game.queue_trigger_event(Default::default(), ironsmith::triggers::TriggerEvent::new(
                ironsmith::events::SpellCastEvent::new_with_snapshot(source, alice, cast_origin, snapshot),
                Default::default()));
            ironsmith::game_loop::drain_pending_trigger_events(&mut game, &mut queue);
        }
        game.move_object(source, Zone::Battlefield, ironsmith::events::EventCause::effect());
        ironsmith::game_loop::drain_pending_trigger_events(&mut game, &mut queue);
        ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
        assert_eq!(game.stack.len(), 1, "one entry trigger: {origin:?}/{cast_origin:?}");
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        assert_eq!(game.player(alice).unwrap().hand.len(), expected,
            "replacement draw count: {origin:?}/{cast_origin:?}");
    }
}

#[test]
fn fblthp_impossibly_lost_triggers_once_on_combat_damage_to_opponents_during_your_turn() {
    let definition = p31_compile("Fblthp, Impossibly Lost",
        "Mana cost: {1}{U}\nType: Legendary Creature — Homunculus\nPower/Toughness: 1/1\nWhen one or more of your opponents are dealt combat damage during your turn, draw two cards. If your library has no cards in it, you win the game. Fblthp's owner shuffles him into their library. (If you draw from an empty library this way, you still win the game.)");
    let trigger = definition.abilities.iter().find_map(|ability| match &ability.kind {
        AbilityKind::Triggered(trigger) => Some(format!("{trigger:?}")),
        _ => None,
    }).expect("combat-damage trigger");
    assert!(trigger.contains("Opponent"), "{trigger}");
    assert!(trigger.contains("WinTheGame") || trigger.contains("Win"), "{trigger}");
}

#[test]
fn festival_of_embers_casts_instants_and_sorceries_from_graveyard_for_one_life_on_your_turn() {
    let definition = p31_compile("Festival of Embers",
        "Mana cost: {4}{R}\nType: Enchantment\nDuring your turn, you may cast instant and sorcery spells from your graveyard by paying 1 life in addition to their other costs.\nIf a card or token would be put into your graveyard from anywhere, exile it instead.\n{1}{R}: Sacrifice this enchantment.");
    let debug = format!("{definition:?}");
    assert!(debug.contains("Graveyard") && debug.contains("Life"), "{debug}");
    assert!(debug.contains("YourTurn") || debug.contains("DuringYourTurn"), "{debug}");
}

#[test]
fn fire_lord_ozai_retains_sacrifice_mana_until_end_of_combat() {
    let definition = p31_compile("Fire Lord Ozai",
        "Mana cost: {3}{B}\nType: Legendary Creature — Human Noble\nPower/Toughness: 4/4\nWhenever Fire Lord Ozai attacks, you may sacrifice another creature. If you do, add an amount of {R} equal to the sacrificed creature's power. Until end of combat, you don't lose this mana as steps end.\n{6}: Exile the top card of each opponent's library. Until end of turn, you may play one of those cards without paying its mana cost.");
    let retain = p31_effects(&definition).into_iter()
        .find_map(|effect| effect.downcast_ref::<ironsmith::effects::ManaRetainedEffect>().cloned())
        .expect("mana retention");
    assert_eq!(retain.duration, ironsmith_core::ManaRetentionDuration::EndOfCombat);
    struct Accept;
    impl ironsmith::decision::DecisionMaker for Accept {
        fn decide_boolean(&mut self, _: &ironsmith::GameState, _: &ironsmith::decisions::context::BooleanContext) -> bool { true }
    }
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.phase = ironsmith::game_state::Phase::Combat;
    game.turn.step = Some(ironsmith::game_state::Step::DeclareAttackers);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let fodder = CardBuilder::new(CardId::new(), "Sacrifice mana witness")
        .card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(3, 3)).build();
    game.create_object_from_card(&fodder, alice, Zone::Battlefield);
    game.remove_summoning_sickness(source);
    let mut ctx = ironsmith::effects::EffectContext::new_default(source, alice);
    ironsmith::effects::execute_effect(&mut game, &Effect::new(ironsmith::effects::AddManaEffect::new(
        vec![ironsmith::mana::ManaSymbol::Red], ironsmith::target::PlayerFilter::You)), &mut ctx).unwrap();
    drop(ctx);
    let mut combat = ironsmith::combat_state::CombatState::default();
    ironsmith::combat_state::declare_attackers(&mut game, &mut combat,
        vec![(source, ironsmith::combat_state::AttackTarget::Player(bob))]).unwrap();
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    ironsmith::game_loop::drain_pending_trigger_events(&mut game, &mut queue);
    let mut dm = Accept;
    ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
    assert_eq!(game.stack.len(), 1);
    ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
    assert!(game.player(alice).unwrap().graveyard.iter()
        .any(|id| game.object(*id).unwrap().name.as_ref() == "Sacrifice mana witness"));
    assert_eq!(game.player(alice).unwrap().mana_pool.red, 4, "one old plus three sacrifice mana");
    game.turn.step = Some(ironsmith::game_state::Step::DeclareBlockers);
    game.empty_mana_pools().unwrap();
    assert_eq!(game.player(alice).unwrap().mana_pool.red, 3, "only sacrifice mana is retained");
    game.turn.step = Some(ironsmith::game_state::Step::EndCombat);
    game.empty_mana_pools().unwrap();
    assert_eq!(game.player(alice).unwrap().mana_pool.total_wide(), 0);
}

#[test]
fn face_yourself_copies_each_creature_with_haste_and_conditional_sacrifice() {
    let definition = p31_compile("Face Yourself",
        "Mana cost: {5}{R}{R}\nType: Sorcery\nFor each creature target player controls, create a token that's a copy of that creature, except it has haste and \"At the beginning of the end step, if you don't control a planeswalker, sacrifice this creature.\"");
    assert!(definition.abilities.iter().all(|ability| !matches!(ability.kind, AbilityKind::Static(_))),
        "a sorcery's iterated creation is not a static anthem");
    let effects = format!("{:?}", p31_effects(&definition));
    assert!(effects.contains("CreateTokenCopyEffect"), "{effects}");
    assert!(effects.contains("has_haste: true"), "{effects}");
    assert!(effects.contains("sacrifice_at_next_end_step: false"),
        "the planeswalker condition lives only on the granted trigger: {effects}");
    assert!(effects.contains("Planeswalker"), "{effects}");
}

#[test]
fn expose_the_culprit_cloaks_the_shuffled_disguise_pile() {
    let definition = p31_compile("Expose the Culprit",
        "Mana cost: {1}{R}\nType: Instant\nChoose one or both —\n• Turn target face-down creature face up.\n• Exile any number of face-up creatures you control with disguise in a face-down pile, shuffle that pile, then cloak them. (To cloak a card, put it onto the battlefield face down as a 2/2 creature with ward {2}. Turn it face up any time for its mana cost if it's a creature card.)");
    let effects = format!("{:?}", p31_effects(&definition));
    assert!(effects.contains("ManifestObjectsEffect"), "{effects}");
    assert!(effects.contains("cloak: true"), "{effects}");
}

#[test]
fn eye_for_an_eye_reflects_without_preventing() {
    let definition = p31_compile("Eye for an Eye",
        "Mana cost: {W}{W}\nType: Instant\nThe next time a source of your choice would deal damage to you this turn, instead that source deals that much damage to you and Eye for an Eye deals that much damage to that source's controller.");
    let shield = p31_effects(&definition).into_iter()
        .find_map(|effect| effect.downcast_ref::<ironsmith::effects::PreventNextTimeDamageEffect>().cloned())
        .expect("next-time damage replacement");
    assert_eq!(shield.portion, ironsmith_core::NextTimeDamagePreventionPortion::ReflectOnly);
    assert!(shield.reflect_damage_to_source_controller);
    assert!(matches!(shield.source, ironsmith::effects::PreventNextTimeDamageSource::Choice));
    assert!(matches!(shield.target, ironsmith::effects::PreventNextTimeDamageTarget::You));
}
