//! Claude package 1 runtime checks for the compilation-completion task
//! (see reports/compilation-completion-20261016/COORDINATION.md).
#![allow(unused_imports)]
use super::compile_to_runtime_definition;
use ironsmith::effect::Effect;

#[test]
fn master_of_cruelties_attacks_only_alone_and_triggers_on_unblocked_player_attacks() {
    use ironsmith::combat_state::{AttackTarget, AttackerInfo, CombatState};
    use ironsmith::decision::AttackerDeclaration;
    use ironsmith::events::combat::CreatureAttackedAndUnblockedEvent;
    use ironsmith::game_loop::apply_attacker_declarations;
    use ironsmith::game_state::{GameState, Phase, Step};
    use ironsmith::triggers::{AttackEventTarget, TriggerEvent, TriggerQueue, check_triggers};
    use ironsmith::{PlayerId, Zone};
    let definition = compile_to_runtime_definition("Master of Cruelties",
        "Type: Creature — Demon\nPower/Toughness: 1/4\nFirst strike, deathtouch\nThis creature can only attack alone.\nWhenever this creature attacks a player and isn't blocked, that player's life total becomes 1. This creature assigns no combat damage this combat.", false)
        .expect("Master of Cruelties should compile");
    let debug = format!("{definition:?}");
    assert!(debug.contains("AttackOnlyAlone"), "{debug}");
    assert!(debug.contains("SetLifeTotal") && debug.contains("AssignNoCombatDamage"), "{debug}");
    let other = ironsmith::cards::builders::CardDefinitionBuilder::new(ironsmith::ids::CardId::new(), "Other Attacker")
        .card_types(vec![ironsmith::types::CardType::Creature])
        .subtypes(vec![ironsmith::types::Subtype::Human])
        .power_toughness(ironsmith::card::PowerToughness::fixed(2, 2)).build();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let setup = || {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let master = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let buddy = game.create_object_from_definition(&other, alice, Zone::Battlefield);
        game.remove_summoning_sickness(master);
        game.remove_summoning_sickness(buddy);
        game.turn.active_player = alice;
        game.turn.phase = Phase::Combat;
        game.turn.step = Some(Step::DeclareAttackers);
        game.refresh_continuous_state().unwrap();
        (game, master, buddy)
    };
    let (mut game, master, buddy) = setup();
    let mut combat = CombatState::default();
    let together = [
        AttackerDeclaration { creature: master, target: AttackTarget::Player(bob) },
        AttackerDeclaration { creature: buddy, target: AttackTarget::Player(bob) },
    ];
    assert!(apply_attacker_declarations(&mut game, &mut combat, &mut TriggerQueue::new(), &together).is_err(),
        "a creature that can only attack alone can't attack alongside another");
    let (mut game, _master, buddy) = setup();
    let mut combat = CombatState::default();
    apply_attacker_declarations(&mut game, &mut combat, &mut TriggerQueue::new(),
        &[AttackerDeclaration { creature: buddy, target: AttackTarget::Player(bob) }]).unwrap();
    let (mut game, master, _buddy) = setup();
    let mut combat = CombatState::default();
    apply_attacker_declarations(&mut game, &mut combat, &mut TriggerQueue::new(),
        &[AttackerDeclaration { creature: master, target: AttackTarget::Player(bob) }]).unwrap();
    let unblocked = |game: &GameState| check_triggers(game, &TriggerEvent::new_with_provenance(
        CreatureAttackedAndUnblockedEvent::new(master, AttackEventTarget::Player(bob)), Default::default()));
    game.combat = Some(combat.clone());
    assert_eq!(unblocked(&game).len(), 1, "an unblocked attack on a player triggers");
    let mut planeswalker_attack = combat.clone();
    planeswalker_attack.attackers = vec![AttackerInfo { creature: master, target: AttackTarget::Planeswalker(master) }];
    game.combat = Some(planeswalker_attack);
    assert!(unblocked(&game).is_empty(), "attacking a planeswalker is not attacking a player");
}

#[test]
fn shared_subject_dies_or_deals_combat_damage_trigger_keeps_both_events() {
    let definition = compile_to_runtime_definition("Millicent, Restless Revenant",
        "Type: Legendary Creature — Spirit Soldier\nPower/Toughness: 4/4\nAffinity for Spirits\nFlying\nWhenever Millicent or another nontoken Spirit you control dies or deals combat damage to a player, create a 1/1 white Spirit creature token with flying.", false)
        .expect("shared-subject dies-or-damage trigger should compile");
    let triggered = definition.abilities.iter().filter_map(|ability| match &ability.kind {
        ironsmith::ability::AbilityKind::Triggered(triggered) => Some(triggered),
        _ => None,
    }).collect::<Vec<_>>();
    assert_eq!(triggered.len(), 1);
    let trigger = format!("{:?}", triggered[0].trigger);
    assert!(trigger.contains("OrTrigger"), "{trigger}");
    assert!(trigger.contains("ZoneChangeTrigger"), "dies arm: {trigger}");
    assert!(trigger.contains("DealsCombatDamageToPlayerTrigger"), "combat damage arm: {trigger}");
    assert!(trigger.contains("Spirit"), "both arms keep the Spirit subject: {trigger}");
}

#[test]
fn target_player_discards_cards_of_each_of_sacrificed_creatures_colors() {
    let definition = compile_to_runtime_definition("Mind Extraction",
        "Type: Sorcery\nAs an additional cost to cast this spell, sacrifice a creature.\nTarget player reveals their hand and discards all cards of each of the sacrificed creature's colors.", false)
        .expect("a later 'each of' must not be read as a target-player participant list");
    let effects = definition.spell_effect.as_ref().expect("spell effect").all_effects();
    fn find(effect: &Effect, found: &mut Vec<String>) {
        if effect.downcast_ref::<ironsmith::effects::DiscardEffect>().is_some() {
            found.push(format!("{effect:?}"));
        }
        effect.visit_child_effects(&mut |child| find(child, found));
    }
    let mut discards = Vec::new();
    for effect in effects { find(effect, &mut discards); }
    assert_eq!(discards.len(), 1, "{definition:#?}");
    assert!(discards[0].contains("SharesColorWithTagged") && discards[0].contains("sacrificed"), "{}", discards[0]);
}

#[test]
fn leading_attached_condition_rebinds_it_to_the_attached_creature() {
    for (name, text, needles) in [
        ("Stunning Strike", "Type: Enchantment — Aura\nFlash\nEnchant creature\nWhen this Aura enters, tap enchanted creature and remove it from combat.\nAs long as enchanted creature isn't legendary, it doesn't untap during its controller's untap step.",
            &["AttachedToSourceMatches", "Legendary"][..]),
        ("Security Bypass", "Type: Enchantment — Aura\nEnchant creature\nAs long as enchanted creature is attacking alone, it can't be blocked.\nEnchanted creature has \"Whenever this creature deals combat damage to a player, it connives.\"",
            &["AttachedToSourceMatches", "attacking_alone: true", "Unblockable"][..]),
    ] {
        let definition = compile_to_runtime_definition(name, text, false)
            .unwrap_or_else(|err| panic!("{name} should compile: {err:?}"));
        let debug = format!("{:?}", definition.abilities);
        for needle in needles {
            assert!(debug.contains(needle), "{name} missing {needle}: {debug}");
        }
        assert!(!debug.contains("SourceIsAttacking"), "{name}: the condition is about the enchanted creature");
    }
}

#[test]
fn player_or_planeswalker_controller_life_relation_reads_both_arms() {
    let definition = compile_to_runtime_definition("Pulse of the Forge",
        "Type: Instant\nPulse of the Forge deals 4 damage to target player or planeswalker. Then if that player or that planeswalker's controller has more life than you, return Pulse of the Forge to its owner's hand.", false)
        .expect("player-or-planeswalker-controller life relation should compile");
    let debug = format!("{:?}", definition.spell_effect);
    assert!(debug.contains("ConditionalEffect"), "{debug}");
    assert!(debug.contains("LifeTotal(TargetPlayerOrControllerOfTarget)"), "{debug}");
    assert!(debug.contains("GreaterThan") && debug.contains("LifeTotal(You)"), "{debug}");
}

#[test]
fn the_spells_controller_may_choose_new_targets_after_exchange() {
    let definition = compile_to_runtime_definition("Sudden Substitution",
        "Type: Instant\nSplit second\nExchange control of target noncreature spell and target creature. Then the spell's controller may choose new targets for it.", false)
        .expect("'the spell's controller may' should read as the spell's controller");
    let debug = format!("{:?}", definition.spell_effect);
    assert!(debug.contains("ExchangeControlEffect"), "{debug}");
    assert!(debug.contains("RetargetStackObjectEffect"), "{debug}");
    assert!(debug.contains("chooser: ControllerOf("), "the retargeting player is the spell's controller: {debug}");
}

#[test]
fn same_opponent_target_pair_sacrifices_one_and_counters_the_other() {
    let definition = compile_to_runtime_definition("Retribution",
        "Type: Sorcery\nChoose two target creatures controlled by the same opponent. That player chooses and sacrifices one of those creatures. Put a -1/-1 counter on the other.", false)
        .expect("same-opponent target pair should compile");
    let debug = format!("{:?}", definition.spell_effect);
    assert!(debug.contains("target_set_same_controller: true"), "{debug}");
    assert!(debug.contains("controller: Some(Opponent)"), "the shared controller is an opponent: {debug}");
    assert!(debug.contains("Sacrifice"), "{debug}");
    assert!(debug.contains("MinusOneMinusOne"), "{debug}");
    assert!(debug.contains("IsNotTaggedObject"), "the counter goes on the unchosen creature: {debug}");
}

#[test]
fn each_creature_pump_per_poison_counter_reads_each_creatures_controller() {
    use ironsmith::card::{CardBuilder, PowerToughness};
    use ironsmith::ids::CardId;
    use ironsmith::types::CardType;
    use ironsmith::{PlayerId, Zone};
    let definition = compile_to_runtime_definition("Phyresis Outbreak",
        "Type: Sorcery\nEach opponent gets a poison counter. Then each creature your opponents control gets -1/-1 until end of turn for each poison counter its controller has.", false)
        .expect("per-object poison count should compile");
    let debug = format!("{:?}", definition.spell_effect);
    assert!(debug.contains("PlayerCounters(ControllerOf(Tagged("), "{debug}");
    let mut game = ironsmith::game_state::GameState::new(vec!["Alice".into(), "Bob".into(), "Cara".into()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let cara = PlayerId::from_index(2);
    game.player_mut(bob).unwrap().add_poison(1);
    let card = |name: &str| CardBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(4, 4)).build();
    let bobs = game.create_object_from_card(&card("Bob's creature"), bob, Zone::Battlefield);
    let caras = game.create_object_from_card(&card("Cara's creature"), cara, Zone::Battlefield);
    let mine = game.create_object_from_card(&card("Alice's creature"), alice, Zone::Battlefield);
    let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
    let mut ctx = ironsmith::effects::EffectContext::new_default(source, alice);
    for effect in definition.spell_effect.as_ref().unwrap().all_effects() {
        ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
    }
    game.refresh_continuous_state().unwrap();
    assert_eq!(game.player(bob).unwrap().poison_counters, 2);
    assert_eq!(game.player(cara).unwrap().poison_counters, 1);
    assert_eq!(game.calculated_toughness(bobs), Some(2), "Bob now has two poison counters");
    assert_eq!(game.calculated_toughness(caras), Some(3), "Cara has one poison counter");
    assert_eq!(game.calculated_toughness(mine), Some(4), "your creatures are unaffected");
    game.player_mut(bob).unwrap().add_poison(3);
    let later = game.create_object_from_card(&card("Later creature"), bob, Zone::Battlefield);
    assert_eq!(game.calculated_toughness(bobs), Some(2), "the amount is captured at resolution");
    assert_eq!(game.calculated_toughness(later), Some(4), "later entrants are outside the resolution set");
    game.effect_store.continuous_effects.cleanup_end_of_turn();
    assert_eq!(game.calculated_toughness(bobs), Some(4));
    assert_eq!(game.calculated_toughness(caras), Some(4));
}

#[test]
fn destroy_each_permanent_whose_controller_was_dealt_combat_damage_by_source() {
    let definition = compile_to_runtime_definition("Steel Hellkite",
        "Type: Artifact Creature — Dragon\nPower/Toughness: 5/5\nFlying\n{2}: This creature gets +1/+0 until end of turn.\n{X}: Destroy each nonland permanent with mana value X whose controller was dealt combat damage by this creature this turn. Activate only once each turn.", false)
        .expect("controller combat-damage history destroy should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("WasDealtCombatDamageByDistinctSourcesThisTurn"), "{debug}");
    assert!(debug.contains("minimum: 1"), "{debug}");
}

#[test]
fn back_referenced_creature_attacks_during_its_controllers_next_combat() {
    let definition = compile_to_runtime_definition("Sizzling Soloist",
        "Type: Creature — Human Citizen\nPower/Toughness: 3/2\nAlliance — Whenever another creature you control enters, target creature an opponent controls can't block this turn. If this is the second time this ability has resolved this turn, that creature attacks during its controller's next combat phase if able.", false)
        .expect("back-referenced next-combat attack requirement should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("MustAttackPlayerThisTurnEffect"), "{debug}");
    assert!(debug.contains("controllers_next_combat: true"), "{debug}");
    assert!(debug.contains("Tagged("), "the requirement names the targeted creature: {debug}");
}

#[test]
fn blocked_creature_cant_attack_during_its_controllers_next_turn() {
    let definition = compile_to_runtime_definition("Wall of Dust",
        "Type: Creature — Wall\nPower/Toughness: 1/4\nDefender (This creature can't attack.)\nWhenever this creature blocks a creature, that creature can't attack during its controller's next turn.", false)
        .expect("controller's-next-turn attack restriction should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("NextTurn(ControllerOf(Tagged("), "the restriction waits for that creature's controller's turn: {debug}");
    assert!(debug.contains("Attack("), "{debug}");
    assert!(debug.contains("EndOfTurn"), "{debug}");
}

#[test]
fn same_opponent_target_pair_damages_the_chosen_and_stops_the_other_blocking() {
    let definition = compile_to_runtime_definition("Trial of Agony",
        "Type: Sorcery\nChoose two target creatures controlled by the same opponent. That player chooses one of those creatures. Trial of Agony deals 5 damage to that creature, and the other can't block this turn.", false)
        .expect("same-opponent choose-one program should compile");
    let debug = format!("{:?}", definition.spell_effect);
    assert!(debug.contains("target_set_same_controller: true"), "{debug}");
    assert!(debug.contains("controller: Some(Opponent)"), "{debug}");
    assert!(debug.contains("ChooseObjectsEffect"), "{debug}");
    assert!(debug.contains("Fixed(5)"), "{debug}");
    assert!(debug.contains("Block(") && debug.contains("IsNotTaggedObject"), "the other creature can't block: {debug}");
    assert!(!debug.contains("Sacrifice"), "{debug}");
}

#[test]
fn basic_land_type_demonstrative_names_the_triggering_land() {
    let definition = compile_to_runtime_definition("Snowfall",
        "Type: Enchantment\nCumulative upkeep {U} (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)\nWhenever an Island is tapped for mana, its controller may add an additional {U}. If that Island is snow, its controller may add an additional {U}{U} instead. Spend this mana only to pay cumulative upkeep costs.", false)
        .expect("'that Island is snow' should read like 'that land is snow'");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("TapForManaTrigger"), "{debug}");
    assert!(debug.contains("TaggedObjectMatches") && debug.contains("Snow"), "{debug}");
    assert!(debug.contains("ManaRestrictedEffect"), "{debug}");
}

#[test]
fn teamwork_counter_unless_pays_replacement_keeps_both_taxes() {
    let definition = compile_to_runtime_definition("We Say Thee Nay!",
        "Type: Instant — Arcane\nTeamwork 2 (As an additional cost to cast this spell, you may tap any number of creatures you control with total power 2 or more.)\nCounter target spell unless its controller pays {2}. Counter that spell unless its controller pays {4} instead if this spell was cast using teamwork.", false)
        .expect("counter-unless-pays local replacement should compile");
    let debug = format!("{:?}", definition.spell_effect);
    let program = definition.spell_effect.as_ref().unwrap();
    let [segment] = program.segments.as_slice() else { panic!("one replaced instruction: {debug}") };
    let [replacement] = segment.self_replacements.as_slice() else { panic!("one conditional self replacement: {debug}") };
    assert!(matches!(&replacement.condition, ironsmith::effect::Condition::ThisSpellPaidLabel(label)
        if label.kind == ironsmith_core::OptionalCostKind::Teamwork), "{debug}");
    assert!(debug.matches("CounterUnless").count() >= 2 || debug.matches("Unless").count() >= 2, "both counter taxes survive: {debug}");
    assert!(debug.contains("Generic(4)") && debug.contains("Generic(2)"), "{debug}");
}

#[test]
fn damage_source_qualifier_batches_the_opponent_combat_damage_trigger() {
    let definition = compile_to_runtime_definition("Mindblade Render",
        "Type: Creature — Azra Warrior\nPower/Toughness: 1/3\nWhenever your opponents are dealt combat damage, if any of that damage was dealt by a Warrior, you draw a card and you lose 1 life.", false)
        .expect("damage-source intervening-if should fold into the trigger");
    let triggered = definition.abilities.iter().find_map(|ability| match &ability.kind {
        ironsmith::ability::AbilityKind::Triggered(triggered) => Some(triggered),
        _ => None,
    }).expect("triggered ability");
    let trigger = format!("{:?}", triggered.trigger);
    assert!(trigger.contains("DealsCombatDamageToPlayerTrigger"), "{trigger}");
    assert!(trigger.contains("Warrior"), "{trigger}");
    assert!(trigger.contains("one_or_more: true") && trigger.contains("each_damaged_player: false"), "one trigger per damage step: {trigger}");
    let effects = format!("{:?}", triggered.effects);
    assert!(!effects.contains("ConditionalEffect"), "the qualifier is absorbed, not a resolution check: {effects}");
}

#[test]
fn only_you_may_activate_restatement_after_granted_ability_compiles_to_the_grant() {
    let definition = compile_to_runtime_definition("Martyrdom",
        "Type: Instant\nUntil end of turn, target creature you control gains \"{0}: The next 1 damage that would be dealt to target creature, planeswalker, or player this turn is dealt to this creature instead.\" Only you may activate this ability.", false)
        .expect("restated controller-only activation should compile");
    let debug = format!("{:?}", definition.spell_effect);
    assert!(debug.contains("Activated"), "the grant carries the activated ability: {debug}");
    assert!(!debug.contains("AnyTimeByOpponents") && !debug.contains("AnyTimeBySourceOwner"), "{debug}");
}

#[test]
fn aura_moves_to_another_permanent_controlled_by_its_hosts_controller() {
    let definition = compile_to_runtime_definition("Simic Guildmage",
        "Type: Creature — Elf Wizard\nPower/Toughness: 2/2\n({G/U} can be paid with either {G} or {U}.)\n{1}{G}: Move a +1/+1 counter from target creature onto another target creature with the same controller.\n{1}{U}: Attach target Aura attached to a permanent to another permanent with the same controller.", false)
        .expect("same-controller aura move should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("TagMatchingObjects"), "the Aura's host is captured: {debug}");
    assert!(debug.contains("with_attached_object: Some("), "{debug}");
    assert!(debug.contains("SameControllerAsTagged") && debug.contains("IsNotTaggedObject"), "{debug}");
    assert!(debug.contains("Attach"), "{debug}");
}

#[test]
fn excess_damage_history_gates_the_end_step_dragon() {
    let definition = compile_to_runtime_definition("Rith, Liberated Primeval",
        "Type: Legendary Creature — Dragon\nPower/Toughness: 5/5\nFlying, ward {2}\nOther Dragons you control have ward {2}.\nAt the beginning of your end step, if a creature or planeswalker an opponent controlled was dealt excess damage this turn, create a 4/4 red Dragon creature token with flying.", false)
        .expect("excess-damage history condition should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("DealtExcessDamage("), "{debug}");
    assert!(debug.contains("Creature") && debug.contains("Planeswalker"), "{debug}");
    assert!(debug.contains("controller: Some(Opponent)"), "{debug}");
}

#[test]
fn chosen_direction_limits_attacks_to_the_nearest_opponent() {
    use ironsmith::card::{CardBuilder, PowerToughness};
    use ironsmith::ids::CardId;
    use ironsmith::types::CardType;
    use ironsmith::{PlayerId, Zone};
    let barrier = compile_to_runtime_definition("Mystic Barrier",
        "Type: Enchantment\nWhen this enchantment enters and at the beginning of your upkeep, choose left or right.\nEach player may attack only the nearest opponent in the last chosen direction and planeswalkers controlled by that opponent.", false)
        .expect("Mystic Barrier should compile");
    let pramikon = compile_to_runtime_definition("Pramikon, Sky Rampart",
        "Type: Legendary Creature — Wall\nPower/Toughness: 1/5\nFlying, defender\nAs Pramikon enters, choose left or right.\nEach player may attack only the nearest opponent in the chosen direction and planeswalkers controlled by that opponent.", false)
        .expect("Pramikon should compile");
    let teyo = compile_to_runtime_definition("Teyo, Geometric Tactician",
        "Type: Legendary Planeswalker — Teyo\nLoyalty: 5\nWhen Teyo enters, create a 0/4 white Wall creature token with defender and flying.\n+1: You and target opponent each draw a card.\n−2: Choose left or right. Until your next turn, each player may attack only the nearest opponent in the last chosen direction and planeswalkers controlled by that opponent.", false)
        .expect("Teyo should compile");
    for (name, definition) in [("Mystic Barrier", &barrier), ("Pramikon", &pramikon), ("Teyo", &teyo)] {
        let debug = format!("{:?}", definition.abilities);
        assert!(debug.contains("AttackOnlyNearestOpponentInChosenDirection"), "{name}: {debug}");
    }
    assert!(format!("{:?}", teyo.abilities).contains("YourNextTurn"));

    let mut game = ironsmith::game_state::GameState::new(
        vec!["Alice".into(), "Bob".into(), "Cara".into(), "Dan".into()], 20);
    let players: Vec<PlayerId> = (0..4).map(PlayerId::from_index).collect();
    let source = game.create_object_from_definition(&barrier, players[0], Zone::Battlefield);
    let card = CardBuilder::new(CardId::new(), "Attacker")
        .card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(2, 2)).build();
    let attackers: Vec<_> = players.iter()
        .map(|player| game.create_object_from_card(&card, *player, Zone::Battlefield)).collect();
    game.set_chosen_named_option(source, "left".to_string());
    game.refresh_continuous_state().unwrap();
    game.update_cant_effects();
    for (index, attacker) in attackers.iter().enumerate() {
        let nearest = players[(index + 1) % 4];
        for (other, player) in players.iter().enumerate() {
            if other == index { continue; }
            assert_eq!(game.can_attack_defending_player(*attacker, *player), *player == nearest,
                "left: player {index} may attack only player {}", (index + 1) % 4);
        }
    }
    game.set_chosen_named_option(source, "right".to_string());
    game.refresh_continuous_state().unwrap();
    game.update_cant_effects();
    assert!(game.can_attack_defending_player(attackers[0], players[3]));
    assert!(!game.can_attack_defending_player(attackers[0], players[1]));
}

#[test]
fn visiting_an_attraction_is_a_keyword_action_trigger() {
    let definition = compile_to_runtime_definition("Motion Sickness",
        "Type: Enchantment — Aura\nEnchant creature\nWhen this Aura enters, tap enchanted creature.\nEnchanted creature doesn't untap during its controller's untap step.\nWhenever you visit an Attraction, you may attach this Aura to target tapped creature.", false)
        .expect("visit trigger should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("VisitAttraction"), "{debug}");
    assert!(debug.contains("Attach"), "{debug}");
    assert!(debug.contains("MayEffect"), "{debug}");
}

#[test]
fn valeyard_doubles_opponents_villainous_choices_and_adds_a_vote() {
    let definition = compile_to_runtime_definition("The Valeyard",
        "Type: Legendary Creature — Time Lord Noble\nPower/Toughness: 3/3\nIf an opponent would face a villainous choice, they face that choice an additional time. (They can make the same or different choices.)\nWhile voting, you may vote an additional time.", false)
        .expect("The Valeyard should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("OpponentsFaceVillainousChoiceAdditionalTime"), "{debug}");
    assert!(debug.contains("VoteAdditionalTimeWhileVoting"), "{debug}");
}

#[test]
fn becomes_your_choice_announces_one_target_and_branches_on_the_choice() {
    let definition = compile_to_runtime_definition("Mercurial Transformation",
        "Type: Sorcery — Lesson\nUntil end of turn, target nonland permanent loses all abilities and becomes your choice of a blue Frog creature with base power and toughness 1/1 or a blue Octopus creature with base power and toughness 4/4.", false)
        .expect("becomes-your-choice should compile");
    let debug = format!("{:?}", definition.spell_effect);
    assert!(debug.contains("ChooseNamedOptionEffect"), "{debug}");
    assert!(debug.contains("SourceChosenOption"), "{debug}");
    assert!(debug.contains("Frog") && debug.contains("Octopus"), "{debug}");
    assert!(debug.matches("TargetOnlyEffect").count() <= 1, "one announced target: {debug}");
}

#[test]
fn roll_compared_to_triggering_damage_or_an_exact_result() {
    let definition = compile_to_runtime_definition("Sword of Hours",
        "Type: Artifact — Equipment\nWhenever equipped creature attacks, put a +1/+1 counter on it.\nWhenever equipped creature deals combat damage, roll a d12. If the result is greater than the damage dealt or the result is 12, double the number of +1/+1 counters on that creature.\nEquip {2} ({2}: Attach to target creature you control. Equip only as a sorcery.)", false)
        .expect("roll-vs-damage predicate should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("RollDieEffect"), "{debug}");
    assert!(debug.contains("EventValue(Amount)"), "result compared with the damage dealt: {debug}");
    assert!(debug.contains("Fixed(12)"), "{debug}");
}

#[test]
fn pump_scales_with_name_stickers_on_the_source() {
    let definition = compile_to_runtime_definition("_____ _____ _____ Trespasser",
        "Type: Creature — Human Rogue Guest\nPower/Toughness: 1/3\nWhen this creature enters, you may put a name sticker on it.\n{3}{U}: This creature gets +1/+0 until end of turn for each name sticker on it. It can't be blocked this turn.", false)
        .expect("sticker-count pump should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("StickersOnSource") && debug.contains("NameSticker"), "{debug}");
    assert!(debug.contains("PutStickerEffect"), "{debug}");
}
