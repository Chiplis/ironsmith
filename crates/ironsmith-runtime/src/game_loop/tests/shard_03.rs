#![allow(unused_imports)]
use super::shard_00::*;
use super::shard_01::*;
use super::shard_02::*;
use super::shard_04::*;
use super::shard_05::*;
use super::shard_06::*;
use super::shard_07::*;
use super::shard_08::*;
use super::shard_09::*;
use super::shard_10::*;
use super::shard_11::*;
use super::shard_12::*;
use super::shard_13::*;
use super::shard_14::*;
use super::shard_15::*;
use super::shard_16::*;
use super::shard_17::*;
use super::*;

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn all_of_history_all_at_once_removes_time_counters_from_each_eligible_object() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let def = all_of_history_definition();
    let spell_id = game.create_object_from_definition(&def, alice, Zone::Stack);

    let alice_permanent = time_travel_permanent(&mut game, alice);
    game.add_counters(alice_permanent, crate::object::CounterType::Time, 2)
        .expect("eligible controlled permanent should accept time counters");
    let suspended = suspended_card_definition();
    let alice_suspended = game.create_object_from_definition(&suspended, alice, Zone::Exile);
    game.add_counters(alice_suspended, crate::object::CounterType::Time, 2)
        .expect("eligible owned suspended card should accept time counters");

    let mut dm = TimeTravelDecisionMaker {
        mode_index: 1,
        prompts: 0,
    };
    let mut ctx =
        crate::effects::ExecutionContext::new_default(spell_id, alice).with_decision_maker(&mut dm);
    crate::effects::execute_effect(&mut game, all_of_history_time_travel_effect(&def), &mut ctx)
        .expect("All of History, All at Once time travel remove branch should execute");

    assert_eq!(
        game.counter_count(alice_permanent, crate::object::CounterType::Time),
        1,
        "time travel should remove one time counter from each eligible permanent when chosen"
    );
    assert_eq!(
        game.counter_count(alice_suspended, crate::object::CounterType::Time),
        1,
        "time travel should remove one time counter from each eligible suspended card when chosen"
    );
    assert_eq!(
        dm.prompts, 2,
        "time travel should offer one remove choice per eligible object"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn all_of_history_all_at_once_can_leave_eligible_objects_unchanged() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let def = all_of_history_definition();
    let spell_id = game.create_object_from_definition(&def, alice, Zone::Stack);

    let alice_permanent = time_travel_permanent(&mut game, alice);
    game.add_counters(alice_permanent, crate::object::CounterType::Time, 2)
        .expect("eligible controlled permanent should accept time counters");
    let suspended = suspended_card_definition();
    let alice_suspended = game.create_object_from_definition(&suspended, alice, Zone::Exile);
    game.add_counters(alice_suspended, crate::object::CounterType::Time, 2)
        .expect("eligible owned suspended card should accept time counters");

    let mut dm = TimeTravelDecisionMaker {
        mode_index: 2,
        prompts: 0,
    };
    let mut ctx =
        crate::effects::ExecutionContext::new_default(spell_id, alice).with_decision_maker(&mut dm);
    crate::effects::execute_effect(&mut game, all_of_history_time_travel_effect(&def), &mut ctx)
        .expect("All of History, All at Once time travel skip branch should execute");

    assert_eq!(
        game.counter_count(alice_permanent, crate::object::CounterType::Time),
        2,
        "time travel should let you leave an eligible permanent unchanged"
    );
    assert_eq!(
        game.counter_count(alice_suspended, crate::object::CounterType::Time),
        2,
        "time travel should let you leave an eligible suspended card unchanged"
    );
    assert_eq!(
        dm.prompts, 2,
        "time travel should offer one skip choice per eligible object"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn all_of_history_all_at_once_does_nothing_when_no_objects_are_eligible() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let def = all_of_history_definition();
    let spell_id = game.create_object_from_definition(&def, alice, Zone::Stack);
    let permanent = time_travel_permanent(&mut game, alice);
    game.add_counters(permanent, crate::object::CounterType::Charge, 1)
        .expect("ineligible permanent should accept non-time counters");

    let mut dm = TimeTravelDecisionMaker {
        mode_index: 0,
        prompts: 0,
    };
    let mut ctx =
        crate::effects::ExecutionContext::new_default(spell_id, alice).with_decision_maker(&mut dm);
    crate::effects::execute_effect(&mut game, all_of_history_time_travel_effect(&def), &mut ctx)
        .expect("All of History, All at Once should resolve with no eligible objects");

    assert_eq!(
        game.counter_count(permanent, crate::object::CounterType::Charge),
        1,
        "time travel should not alter non-time counters on otherwise ineligible permanents"
    );
    assert_eq!(
        dm.prompts, 0,
        "time travel should not ask for choices with no eligible objects"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn feudkillers_verdict_creates_token_when_you_have_more_life_than_an_opponent() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.player_mut(alice).expect("alice should exist").life = 21;
    game.player_mut(bob).expect("bob should exist").life = 20;

    let verdict = CardDefinitionBuilder::new(CardId::new(), "Feudkiller's Verdict")
        .card_types(vec![CardType::Kindred, CardType::Sorcery])
        .parse_text(
            "You gain 10 life. Then if you have more life than an opponent, create a 5/5 white Giant Warrior creature token.",
        )
        .expect("Feudkiller's Verdict should parse");
    let permanents_before = game
        .battlefield
        .iter()
        .filter(|&&id| {
            game.object(id)
                .is_some_and(|obj| game.controller_of(obj) == alice)
        })
        .count();
    let spell_id = game.create_object_from_definition(&verdict, alice, Zone::Stack);
    game.push_to_stack(StackEntry::new(spell_id, alice));

    resolve_stack_entry(&mut game).expect("Feudkiller's Verdict should resolve");

    assert_eq!(
        game.player(alice).expect("alice should exist").life,
        31,
        "Feudkiller's Verdict should gain 10 life before checking the condition"
    );

    let permanents_after = game
        .battlefield
        .iter()
        .filter(|&&id| {
            game.object(id)
                .is_some_and(|obj| game.controller_of(obj) == alice)
        })
        .count();
    assert_eq!(
        permanents_after,
        permanents_before + 1,
        "Feudkiller's Verdict should add one permanent when the condition is true"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn feudkillers_verdict_skips_token_when_no_opponent_has_less_life() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.player_mut(alice).expect("alice should exist").life = 5;
    game.player_mut(bob).expect("bob should exist").life = 20;

    let verdict = CardDefinitionBuilder::new(CardId::new(), "Feudkiller's Verdict")
        .card_types(vec![CardType::Kindred, CardType::Sorcery])
        .parse_text(
            "You gain 10 life. Then if you have more life than an opponent, create a 5/5 white Giant Warrior creature token.",
        )
        .expect("Feudkiller's Verdict should parse");
    let permanents_before = game
        .battlefield
        .iter()
        .filter(|&&id| {
            game.object(id)
                .is_some_and(|obj| game.controller_of(obj) == alice)
        })
        .count();
    let spell_id = game.create_object_from_definition(&verdict, alice, Zone::Stack);
    game.push_to_stack(StackEntry::new(spell_id, alice));

    resolve_stack_entry(&mut game).expect("Feudkiller's Verdict should resolve");

    assert_eq!(
        game.player(alice).expect("alice should exist").life,
        15,
        "Feudkiller's Verdict should still gain 10 life even if token clause fails"
    );

    let permanents_after = game
        .battlefield
        .iter()
        .filter(|&&id| {
            game.object(id)
                .is_some_and(|obj| game.controller_of(obj) == alice)
        })
        .count();
    assert_eq!(
        permanents_after, permanents_before,
        "Feudkiller's Verdict should not add a permanent when no opponent has less life"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn guild_artisan_does_not_trigger_when_attacked_player_is_not_the_life_leader() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let guild_artisan = CardDefinitionBuilder::new(CardId::new(), "Guild Artisan")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::Red],
        ]))
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![crate::types::Subtype::Background])
        .parse_text(
            "Commander creatures you own have \"Whenever this creature attacks a player, if no opponent has more life than that player, you create two Treasure tokens.\"",
        )
        .expect("Guild Artisan should parse");

    let commander = CardBuilder::new(CardId::from_raw(71_021), "Guild Artisan Commander")
        .card_types(vec![CardType::Creature])
        .supertypes(vec![crate::types::Supertype::Legendary])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();

    let commander_id = game.create_object_from_card(&commander, alice, Zone::Battlefield);
    game.set_as_commander(commander_id, alice);
    game.remove_summoning_sickness(commander_id);
    game.create_object_from_definition(&guild_artisan, alice, Zone::Battlefield);
    game.refresh_continuous_state();

    game.player_mut(bob).expect("bob exists").life = 19;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    game.turn.phase = Phase::Combat;
    game.turn.step = Some(crate::game_state::Step::DeclareAttackers);

    let mut combat = CombatState::default();
    let mut trigger_queue = TriggerQueue::new();
    let declarations = vec![AttackerDeclaration {
        creature: commander_id,
        target: AttackTarget::Player(bob),
    }];

    apply_attacker_declarations(&mut game, &mut combat, &mut trigger_queue, &declarations)
        .expect("commander attack should still be legal");
    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("stack processing should succeed even without a trigger");
    assert_eq!(
        game.stack.len(),
        0,
        "Guild Artisan should not create a trigger when the attacked player is behind in life"
    );

    let treasure_count = game
        .battlefield
        .iter()
        .filter(|&&id| game.object(id).is_some_and(|obj| obj.name == "Treasure"))
        .count();
    assert_eq!(treasure_count, 0, "no Treasure tokens should be created");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn cloakwood_hermit_triggers_at_end_step_after_creature_card_hits_graveyard() {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let alice = PlayerId::from_index(0);

    let cloakwood = CardDefinitionBuilder::new(CardId::from_raw(81_100), "Cloakwood Hermit")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![crate::types::Subtype::Background])
        .parse_text(
            "Commander creatures you own have \"At the beginning of your end step, if a creature card was put into your graveyard from anywhere this turn, create two tapped 1/1 green Squirrel creature tokens.\"",
        )
        .expect("Cloakwood Hermit should parse");
    game.create_object_from_definition(&cloakwood, alice, Zone::Battlefield);

    let commander = CardBuilder::new(CardId::from_raw(81_101), "Cloakwood Commander")
        .card_types(vec![CardType::Creature])
        .supertypes(vec![crate::types::Supertype::Legendary])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let commander_id = game.create_object_from_card(&commander, alice, Zone::Battlefield);
    game.set_as_commander(commander_id, alice);
    game.refresh_continuous_state();

    let creature_card = CardBuilder::new(CardId::from_raw(81_102), "Fallen Scout")
        .card_types(vec![CardType::Creature])
        .build();
    let creature_card_id = game.create_object_from_card(&creature_card, alice, Zone::Hand);
    game.move_object_by_effect(creature_card_id, Zone::Graveyard)
        .expect("creature card should move to graveyard this turn");

    game.turn.active_player = alice;
    game.turn.phase = Phase::Ending;
    game.turn.step = Some(crate::game_state::Step::End);

    generate_and_queue_step_triggers(&mut game, &mut trigger_queue);
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "Cloakwood Hermit should grant one end-step trigger when condition is met"
    );

    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("Cloakwood Hermit trigger should go on the stack");
    resolve_stack_entry(&mut game).expect("Cloakwood Hermit trigger should resolve");

    let squirrel_ids = game
        .battlefield
        .iter()
        .copied()
        .filter(|id| game.object(*id).is_some_and(|obj| obj.name == "Squirrel"))
        .collect::<Vec<_>>();
    assert_eq!(
        squirrel_ids.len(),
        2,
        "Cloakwood Hermit should create two Squirrel tokens"
    );
    assert!(
        squirrel_ids.iter().all(|id| game.is_tapped(*id)),
        "Cloakwood Hermit tokens should enter tapped"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn cloakwood_hermit_does_not_trigger_without_creature_card_in_graveyard_this_turn() {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let alice = PlayerId::from_index(0);

    let cloakwood = CardDefinitionBuilder::new(CardId::from_raw(81_103), "Cloakwood Hermit")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![crate::types::Subtype::Background])
        .parse_text(
            "Commander creatures you own have \"At the beginning of your end step, if a creature card was put into your graveyard from anywhere this turn, create two tapped 1/1 green Squirrel creature tokens.\"",
        )
        .expect("Cloakwood Hermit should parse");
    game.create_object_from_definition(&cloakwood, alice, Zone::Battlefield);

    let commander = CardBuilder::new(CardId::from_raw(81_104), "Cloakwood Commander")
        .card_types(vec![CardType::Creature])
        .supertypes(vec![crate::types::Supertype::Legendary])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let commander_id = game.create_object_from_card(&commander, alice, Zone::Battlefield);
    game.set_as_commander(commander_id, alice);
    game.refresh_continuous_state();

    game.turn.active_player = alice;
    game.turn.phase = Phase::Ending;
    game.turn.step = Some(crate::game_state::Step::End);

    generate_and_queue_step_triggers(&mut game, &mut trigger_queue);
    assert!(
        trigger_queue.entries.is_empty(),
        "Cloakwood Hermit should not trigger without a creature card put into your graveyard this turn"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn ignite_memories_reveals_a_random_card_from_target_players_hand_and_damages_them() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let ignite_memories = CardDefinitionBuilder::new(CardId::from_raw(70_001), "Ignite Memories")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(4)],
            vec![ManaSymbol::Red],
        ]))
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Target player reveals a card at random from their hand. Ignite Memories deals damage to that player equal to that card's mana value.\nStorm (When you cast this spell, copy it for each spell cast before it this turn. You may choose new targets for the copies.)",
        )
        .expect("Ignite Memories should parse");

    let low_card = CardBuilder::new(CardId::from_raw(70_002), "Low Probe")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)]]))
        .card_types(vec![CardType::Artifact])
        .build();
    let high_card = CardBuilder::new(CardId::from_raw(70_003), "High Probe")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(5)]]))
        .card_types(vec![CardType::Artifact])
        .build();

    let low_id = game.create_object_from_card(&low_card, bob, Zone::Hand);
    let high_id = game.create_object_from_card(&high_card, bob, Zone::Hand);
    let spell_id = game.create_object_from_definition(&ignite_memories, alice, Zone::Stack);
    game.stack.push(
        crate::game_state::StackEntry::new(spell_id, alice)
            .with_targets(vec![crate::game_state::Target::Player(bob)])
            .with_target_assignments(vec![crate::game_state::TargetAssignment {
                spec: crate::target::ChooseSpec::target_player(),
                range: 0..1,
            }]),
    );

    let bob_life_before = game.player(bob).expect("bob exists").life;
    let mut dm = CaptureRevealDecisionMaker::default();

    resolve_stack_entry_with(&mut game, &mut dm).expect("Ignite Memories should resolve");

    assert!(
        dm.view_calls.len() >= 2,
        "the random reveal should be shown to all players"
    );
    let mut unique_reveals = dm
        .view_calls
        .iter()
        .map(|(_, subject, zone, public, cards)| {
            assert_eq!(
                *subject, bob,
                "the revealed card should come from Bob's hand"
            );
            assert_eq!(*zone, Zone::Hand, "the reveal should come from hand");
            assert!(*public, "the reveal should be public");
            assert_eq!(cards.len(), 1, "only one card should be revealed");
            cards[0]
        })
        .collect::<Vec<_>>();
    unique_reveals.sort();
    unique_reveals.dedup();
    assert_eq!(
        unique_reveals.len(),
        1,
        "the same random card should be shown to each viewer"
    );

    let revealed_id = unique_reveals[0];
    assert!(
        revealed_id == low_id || revealed_id == high_id,
        "the revealed card should come from Bob's hand"
    );

    let revealed_card = game.object(revealed_id).expect("revealed card exists");
    let expected_damage = revealed_card
        .mana_cost
        .as_ref()
        .expect("revealed card should have a mana cost")
        .mana_value() as i32;
    assert_eq!(
        game.player(bob).expect("bob exists").life,
        bob_life_before - expected_damage,
        "Ignite Memories should deal damage equal to the revealed card's mana value"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn singe_mind_ogre_reveals_a_random_card_from_target_players_hand_and_makes_them_lose_life()
 {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let singe_mind_ogre = CardDefinitionBuilder::new(CardId::from_raw(70_010), "Singe-Mind Ogre")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Black],
            vec![ManaSymbol::Red],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Ogre, Subtype::Mutant])
        .power_toughness(PowerToughness::fixed(3, 2))
        .parse_text(
            "When this creature enters, target player reveals a card at random from their hand, then loses life equal to that card's mana value.",
        )
        .expect("Singe-Mind Ogre should parse");

    let low_card = CardBuilder::new(CardId::from_raw(70_011), "Low Probe")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)]]))
        .card_types(vec![CardType::Artifact])
        .build();
    let high_card = CardBuilder::new(CardId::from_raw(70_012), "High Probe")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(5)]]))
        .card_types(vec![CardType::Artifact])
        .build();

    let low_id = game.create_object_from_card(&low_card, bob, Zone::Hand);
    let high_id = game.create_object_from_card(&high_card, bob, Zone::Hand);
    let source_id = game.create_object_from_definition(&singe_mind_ogre, alice, Zone::Battlefield);
    let triggered_effects = game
        .object(source_id)
        .expect("Singe-Mind Ogre should be on the battlefield")
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered.effects.clone()),
            _ => None,
        })
        .expect("Singe-Mind Ogre should have an enters trigger");
    game.stack.push(
        crate::game_state::StackEntry::ability(source_id, alice, triggered_effects)
            .with_targets(vec![crate::game_state::Target::Player(bob)])
            .with_target_assignments(vec![crate::game_state::TargetAssignment {
                spec: crate::target::ChooseSpec::target_player(),
                range: 0..1,
            }])
            .with_triggering_event(TriggerEvent::new_with_provenance(
                EnterBattlefieldEvent::new(source_id, Zone::Hand),
                crate::provenance::ProvNodeId::default(),
            )),
    );

    let bob_life_before = game.player(bob).expect("bob exists").life;
    let mut dm = CaptureRevealDecisionMaker::default();

    resolve_stack_entry_with(&mut game, &mut dm).expect("Singe-Mind Ogre trigger should resolve");

    assert!(
        dm.view_calls.len() >= 2,
        "the random reveal should be shown to all players"
    );
    let mut unique_reveals = dm
        .view_calls
        .iter()
        .map(|(_, subject, zone, public, cards)| {
            assert_eq!(
                *subject, bob,
                "the revealed card should come from Bob's hand"
            );
            assert_eq!(*zone, Zone::Hand, "the reveal should come from hand");
            assert!(*public, "the reveal should be public");
            assert_eq!(cards.len(), 1, "only one card should be revealed");
            cards[0]
        })
        .collect::<Vec<_>>();
    unique_reveals.sort();
    unique_reveals.dedup();
    assert_eq!(
        unique_reveals.len(),
        1,
        "the same random card should be shown to each viewer"
    );

    let revealed_id = unique_reveals[0];
    assert!(
        revealed_id == low_id || revealed_id == high_id,
        "the revealed card should come from Bob's hand"
    );

    let revealed_card = game.object(revealed_id).expect("revealed card exists");
    let expected_life_loss = revealed_card
        .mana_cost
        .as_ref()
        .expect("revealed card should have a mana cost")
        .mana_value() as i32;
    assert_eq!(
        game.player(bob).expect("bob exists").life,
        bob_life_before - expected_life_loss,
        "Singe-Mind Ogre should make that player lose life equal to the revealed card's mana value"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn ruin_raider_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(70_020), "Ruin Raider")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Black],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Orc, Subtype::Pirate])
        .power_toughness(PowerToughness::fixed(3, 2))
        .parse_text(
            "Raid — At the beginning of your end step, if you attacked this turn, reveal the top card of your library and put that card into your hand. You lose life equal to the card's mana value.",
        )
        .expect("Ruin Raider should parse for runtime tests")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn ruin_raider_end_step_event(player: PlayerId) -> TriggerEvent {
    TriggerEvent::new_with_provenance(
        crate::events::phase::BeginningOfEndStepEvent::new(player),
        crate::provenance::ProvNodeId::default(),
    )
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn ruin_raider_top_card(raw_id: u32, name: &str, mana_value: u8) -> crate::card::Card {
    CardBuilder::new(CardId::from_raw(raw_id), name)
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(
            mana_value,
        )]]))
        .card_types(vec![CardType::Artifact])
        .build()
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn ruin_raider_does_not_trigger_at_end_step_without_raid() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let ruin_raider = ruin_raider_definition();
    let ruin_raider_id = game.create_object_from_definition(&ruin_raider, alice, Zone::Battlefield);
    game.turn.active_player = alice;

    let event = ruin_raider_end_step_event(alice);
    let triggers = crate::triggers::check_triggers(&game, &event);

    assert!(
        triggers
            .into_iter()
            .all(|trigger| trigger.source != ruin_raider_id),
        "Ruin Raider should not trigger if its controller did not attack this turn"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn ruin_raider_reveals_top_card_puts_it_into_hand_and_loses_life_after_raid() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let ruin_raider = ruin_raider_definition();
    let ruin_raider_id = game.create_object_from_definition(&ruin_raider, alice, Zone::Battlefield);
    let top_card = ruin_raider_top_card(70_021, "Ruin Raider Revealed Probe", 4);
    let top_id = game.create_object_from_card(&top_card, alice, Zone::Library);
    let top_stable = game.object(top_id).expect("top card exists").stable_id;
    game.turn.active_player = alice;
    game.turn_store
        .turn_history
        .players_attacked_this_turn
        .insert(alice);

    let event = ruin_raider_end_step_event(alice);
    let mut trigger_queue = TriggerQueue::new();
    for trigger in crate::triggers::check_triggers(&game, &event) {
        if trigger.source == ruin_raider_id {
            trigger_queue.add(trigger);
        }
    }
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "Ruin Raider should trigger at your end step after you attacked"
    );
    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("Ruin Raider trigger should go on the stack");

    let life_before = game.player(alice).expect("alice exists").life;
    let mut dm = CaptureRevealDecisionMaker::default();
    resolve_stack_entry_with(&mut game, &mut dm).expect("Ruin Raider trigger should resolve");

    assert!(
        dm.view_calls
            .iter()
            .any(|(_, subject, zone, public, cards)| {
                *subject == alice && *zone == Zone::Library && *public && cards.contains(&top_id)
            }),
        "Ruin Raider should publicly reveal the top card of Alice's library"
    );
    let revealed_id = game
        .find_object_by_stable_id(top_stable)
        .expect("revealed card should still exist");
    assert!(
        game.player(alice)
            .expect("alice exists")
            .hand
            .contains(&revealed_id),
        "Ruin Raider should put the revealed card into its controller's hand"
    );
    assert_eq!(
        game.player(alice).expect("alice exists").life,
        life_before - 4,
        "Ruin Raider should make its controller lose life equal to the revealed card's mana value"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn dinrova_horror_returns_target_and_its_owner_discards() {
    struct ChooseDiscardCardDecisionMaker {
        card_to_discard: ObjectId,
    }

    impl DecisionMaker for ChooseDiscardCardDecisionMaker {
        fn decide_objects(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            if ctx
                .candidates
                .iter()
                .any(|candidate| candidate.id == self.card_to_discard && candidate.legal)
            {
                vec![self.card_to_discard]
            } else {
                ctx.candidates
                    .iter()
                    .filter(|candidate| candidate.legal)
                    .map(|candidate| candidate.id)
                    .take(ctx.min)
                    .collect()
            }
        }
    }

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let dinrova = CardDefinitionBuilder::new(CardId::from_raw(78_010), "Dinrova Horror")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(4)],
            vec![ManaSymbol::Blue],
            vec![ManaSymbol::Black],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Horror])
        .power_toughness(PowerToughness::fixed(4, 4))
        .parse_text(
            "When this creature enters, return target permanent to its owner's hand, then that player discards a card.",
        )
        .expect("Dinrova Horror should parse for runtime test");
    let dinrova_id = game.create_object_from_definition(&dinrova, alice, Zone::Battlefield);

    let borrowed_permanent = CardBuilder::new(CardId::from_raw(78_011), "Borrowed Relic")
        .card_types(vec![CardType::Artifact])
        .build();
    let target_id = game.create_object_from_card(&borrowed_permanent, alice, Zone::Battlefield);
    game.set_current_controller(target_id, bob);

    let alice_discard = CardBuilder::new(CardId::from_raw(78_012), "Alice Discard")
        .card_types(vec![CardType::Artifact])
        .build();
    let bob_hand_card = CardBuilder::new(CardId::from_raw(78_013), "Bob Keeps")
        .card_types(vec![CardType::Artifact])
        .build();
    let alice_discard_id = game.create_object_from_card(&alice_discard, alice, Zone::Hand);
    let _bob_hand_id = game.create_object_from_card(&bob_hand_card, bob, Zone::Hand);

    let etb_trigger = dinrova
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered.clone()),
            _ => None,
        })
        .expect("Dinrova Horror should have an enters trigger");
    let target_spec = etb_trigger
        .choices
        .first()
        .cloned()
        .expect("Dinrova Horror should target a permanent");
    let event = TriggerEvent::new_with_provenance(
        EnterBattlefieldEvent::new(dinrova_id, Zone::Stack),
        crate::provenance::ProvNodeId::default(),
    );
    let mut dm = ChooseDiscardCardDecisionMaker {
        card_to_discard: alice_discard_id,
    };
    let mut ctx = crate::effects::ExecutionContext::new(dinrova_id, alice, &mut dm)
        .with_triggering_event(event)
        .with_targets(vec![crate::effects::ResolvedTarget::Object(target_id)])
        .with_target_assignments(vec![crate::game_state::TargetAssignment {
            spec: target_spec,
            range: 0..1,
        }]);

    for effect in &etb_trigger.effects {
        crate::effects::execute_effect(&mut game, effect, &mut ctx)
            .expect("Dinrova Horror ETB effect should resolve");
    }

    assert!(
        game.player(alice)
            .is_some_and(|player| player.hand.iter().any(|&id| game
                .object(id)
                .is_some_and(|obj| obj.name == "Borrowed Relic"))),
        "the targeted permanent should return to its owner's hand"
    );
    assert!(
        game.player(alice)
            .is_some_and(|player| player.graveyard.iter().any(|&id| game
                .object(id)
                .is_some_and(|obj| obj.name == "Alice Discard"))),
        "the target's owner should discard a card"
    );
    assert!(
        game.player(bob).is_some_and(|player| player
            .hand
            .iter()
            .any(|&id| game.object(id).is_some_and(|obj| obj.name == "Bob Keeps"))),
        "the target's controller should not discard when they do not own the returned permanent"
    );
}

#[test]
pub(super) fn test_monarch_changes_when_creature_deals_combat_damage_to_monarch() {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let attacker_id = create_creature(&mut game, "Monarch Raider", alice, 3, 3);
    game.monarch = Some(bob);

    let events = vec![CombatDamageEvent {
        source: attacker_id,
        target: DamageEventTarget::Player(bob),
        amount: 3,
        life_lost: 3,
        result: DamageResult {
            damage_dealt: 3,
            ..DamageResult::default()
        },
    }];

    generate_damage_triggers(&mut game, &events, &mut trigger_queue);

    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "combat damage to the monarch should queue the designation trigger"
    );
    assert_eq!(
        trigger_queue.entries[0].source_name.as_str(),
        "The Monarch",
        "designation transfer should come from the monarch rules object"
    );
    assert_eq!(
        trigger_queue.entries[0].controller, bob,
        "the damaged monarch controls the transfer trigger"
    );

    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("monarch transfer trigger should go on the stack");
    assert_eq!(
        game.stack.len(),
        1,
        "the monarch transfer trigger should be put on the stack"
    );

    resolve_stack_entry(&mut game).expect("monarch transfer trigger should resolve");

    assert_eq!(
        game.monarch,
        Some(alice),
        "the attacking creature's controller should become the monarch"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn archon_of_coronation_enters_trigger_makes_controller_the_monarch() {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let alice = PlayerId::from_index(0);
    let archon = archon_of_coronation_definition();
    let archon_id = game.create_object_from_definition(&archon, alice, Zone::Battlefield);
    let event = crate::events::RawEvent::new(
        crate::events::ZoneChangeEvent::with_cause(
            archon_id,
            Zone::Stack,
            Zone::Battlefield,
            crate::events::cause::EventCause::from_game_rule(),
            None,
        ),
        crate::provenance::ProvNodeId::default(),
    );

    for trigger in crate::triggers::check_triggers(&game, &event) {
        if trigger.source == archon_id {
            trigger_queue.add(trigger);
        }
    }
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "Archon of Coronation should trigger when it enters"
    );
    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("Archon of Coronation enters trigger should go on the stack");
    resolve_stack_entry(&mut game).expect("Archon of Coronation enters trigger should resolve");

    assert_eq!(
        game.monarch,
        Some(alice),
        "Archon of Coronation's controller should become the monarch"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn archon_of_coronation_monarch_takes_damage_without_losing_life_and_still_loses_monarch()
 {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let archon = archon_of_coronation_definition();
    game.create_object_from_definition(&archon, alice, Zone::Battlefield);
    let attacker = create_creature(&mut game, "Archon Challenger", bob, 3, 3);
    game.monarch = Some(alice);
    game.update_cant_effects();

    assert!(
        game.can_lose_life(alice),
        "Archon should not stop non-damage life loss"
    );
    assert_eq!(
        game.lose_life(alice, 2),
        2,
        "non-damage life loss should still happen while Archon's controller is monarch"
    );
    let life_before_damage = game.player(alice).expect("alice exists").life;

    let event = deal_test_combat_damage_to_player(&mut game, attacker, alice, 3);
    assert_eq!(
        event.life_lost, 0,
        "combat damage should not cause life loss"
    );
    assert_eq!(
        game.player(alice).expect("alice exists").life,
        life_before_damage,
        "damage should be dealt without reducing the monarch's life total"
    );

    generate_damage_triggers(&mut game, &[event], &mut trigger_queue);
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "combat damage still dealt to the monarch should queue the transfer trigger"
    );
    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("monarch transfer trigger should go on the stack");
    resolve_stack_entry(&mut game).expect("monarch transfer trigger should resolve");

    assert_eq!(
        game.monarch,
        Some(bob),
        "combat damage to the monarch should still make the attacker's controller the monarch"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn archon_of_coronation_nonmonarch_controller_still_loses_life_to_damage() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let archon = archon_of_coronation_definition();
    game.create_object_from_definition(&archon, alice, Zone::Battlefield);
    let attacker = create_creature(&mut game, "Archon Challenger", bob, 3, 3);
    game.monarch = Some(bob);
    game.update_cant_effects();
    let life_before_damage = game.player(alice).expect("alice exists").life;

    let event = deal_test_combat_damage_to_player(&mut game, attacker, alice, 3);

    assert_eq!(
        event.life_lost, 3,
        "Archon should not stop damage-caused life loss when its controller is not monarch"
    );
    assert_eq!(
        game.player(alice).expect("alice exists").life,
        life_before_damage - 3,
        "nonmonarch Archon controller should lose life from damage normally"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_suspended_card_removes_time_counter_during_upkeep() {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let lotus_bloom = CardDefinitionBuilder::new(CardId::from_raw(99001), "Lotus Bloom")
        .parse_text(
            "Type: Artifact\n\
             Suspend 3—{0} (Rather than cast this card from your hand, pay {0} and exile it with three time counters on it. At the beginning of your upkeep, remove a time counter. When the last is removed, you may cast it without paying its mana cost.)\n\
             {T}, Sacrifice this artifact: Add three mana of any one color.",
        )
        .expect("Lotus Bloom text should parse");
    let card_id = game.create_object_from_definition(&lotus_bloom, alice, Zone::Hand);

    let mut dm = SelectFirstDecisionMaker;
    crate::special_actions::perform(
        crate::special_actions::SpecialAction::Suspend { card_id },
        &mut game,
        alice,
        &mut dm,
    )
    .expect("suspend special action should resolve");

    let exiled_id = *game.exile.first().expect("Lotus Bloom should be exiled");
    assert_eq!(
        game.counter_count(exiled_id, crate::object::CounterType::Time),
        3,
        "suspended card should start with three time counters"
    );

    game.turn.phase = Phase::Beginning;
    game.turn.step = Some(crate::game_state::Step::Upkeep);
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    generate_and_queue_step_triggers(&mut game, &mut trigger_queue);

    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "suspended card should queue its upkeep trigger from exile"
    );

    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("suspend upkeep trigger should go on the stack");
    assert_eq!(
        game.stack.len(),
        1,
        "suspend upkeep trigger should use the stack once it triggers"
    );

    resolve_stack_entry(&mut game).expect("suspend upkeep trigger should resolve");

    assert_eq!(
        game.counter_count(exiled_id, crate::object::CounterType::Time),
        2,
        "resolving the upkeep trigger should remove one time counter"
    );
    assert!(
        game.object(exiled_id)
            .is_some_and(|obj| obj.zone == Zone::Exile),
        "suspended card should remain in exile until the last time counter is removed"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_suspend_declined_cast_does_not_keep_triggering_without_time_counters() {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let lotus_bloom = CardDefinitionBuilder::new(CardId::from_raw(99002), "Lotus Bloom")
        .parse_text(
            "Type: Artifact\n\
             Suspend 1—{0} (Rather than cast this card from your hand, pay {0} and exile it with one time counter on it. At the beginning of your upkeep, remove a time counter. When the last is removed, you may cast it without paying its mana cost.)\n\
             {T}, Sacrifice this artifact: Add three mana of any one color.",
        )
        .expect("Lotus Bloom text should parse");
    let card_id = game.create_object_from_definition(&lotus_bloom, alice, Zone::Hand);

    let mut dm = SelectFirstDecisionMaker;
    crate::special_actions::perform(
        crate::special_actions::SpecialAction::Suspend { card_id },
        &mut game,
        alice,
        &mut dm,
    )
    .expect("suspend special action should resolve");

    let exiled_id = *game.exile.first().expect("Lotus Bloom should be exiled");

    game.turn.phase = Phase::Beginning;
    game.turn.step = Some(crate::game_state::Step::Upkeep);
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    generate_and_queue_step_triggers(&mut game, &mut trigger_queue);
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "expected upkeep trigger once"
    );

    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("suspend upkeep trigger should go on the stack");
    resolve_stack_entry(&mut game).expect("declining the suspend cast should still resolve");

    assert_eq!(
        game.counter_count(exiled_id, crate::object::CounterType::Time),
        0,
        "last upkeep trigger should remove the final time counter"
    );
    assert!(
        game.object(exiled_id)
            .is_some_and(|obj| obj.zone == Zone::Exile),
        "declining the free cast should leave the card in exile"
    );

    game.stack.clear();
    game.turn.turn_number += 1;
    generate_and_queue_step_triggers(&mut game, &mut trigger_queue);
    assert!(
        trigger_queue.entries.is_empty(),
        "card without time counters is no longer suspended and should not trigger again"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn jhoira_of_the_ghitu_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(73_200), "Jhoira of the Ghitu")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Human, Subtype::Wizard])
        .power_toughness(PowerToughness::fixed(2, 2))
        .parse_text(
            "{2}, Exile a nonland card from your hand: Put four time counters on the exiled card. If it doesn't have suspend, it gains suspend.",
        )
        .expect("Jhoira of the Ghitu should parse")
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn jhoira_exiles_nonland_card_and_granted_suspend_triggers_from_exile() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let jhoira_def = jhoira_of_the_ghitu_definition();
    let jhoira_id = game.create_object_from_definition(&jhoira_def, alice, Zone::Battlefield);
    game.remove_summoning_sickness(jhoira_id);
    let land_def = CardDefinitionBuilder::new(CardId::from_raw(73_201), "Island Probe")
        .card_types(vec![CardType::Land])
        .build();
    let land_id = game.create_object_from_definition(&land_def, alice, Zone::Hand);
    game.player_mut(alice)
        .expect("Alice should exist")
        .mana_pool
        .add(ManaSymbol::Colorless, 2);

    assert!(
        !crate::decision::compute_legal_actions(&game, alice)
            .into_iter()
            .any(|action| matches!(
                action,
                crate::decision::LegalAction::ActivateAbility { source, .. }
                    if source == jhoira_id
            )),
        "Jhoira should not be activatable when the only card in hand is a land"
    );

    let spell_def = CardDefinitionBuilder::new(CardId::from_raw(73_202), "Suspend Gift Probe")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 3))
        .build();
    let spell_id = game.create_object_from_definition(&spell_def, alice, Zone::Hand);
    let ability_index = game
        .object(jhoira_id)
        .expect("Jhoira should exist")
        .abilities
        .iter()
        .position(|ability| matches!(ability.kind, AbilityKind::Activated(_)))
        .expect("Jhoira should have an activated ability");
    let AbilityKind::Activated(activated) = &game
        .object(jhoira_id)
        .expect("Jhoira should exist")
        .abilities[ability_index]
        .kind
    else {
        unreachable!("ability index should point at Jhoira's activation")
    };
    assert!(
        crate::decision::can_activate_ability_with_restrictions(
            &game,
            jhoira_id,
            ability_index,
            activated,
        ),
        "Jhoira activation should pass direct activation checks"
    );
    let actions = crate::decision::compute_legal_actions(&game, alice);
    let activate_action = actions
        .iter()
        .find(|action| {
            matches!(
                action,
                crate::decision::LegalAction::ActivateAbility { source, .. }
                    if *source == jhoira_id
            )
        })
        .cloned()
        .unwrap_or_else(|| {
            panic!("Jhoira should be activatable with a nonland card in hand; actions={actions:?}")
        });

    let mut trigger_queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = SelectFirstDecisionMaker;
    let mut progress = apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(activate_action),
        &mut dm,
    )
    .expect("Jhoira activation should start cost payment");
    let mut paid_exile_cost = false;
    for _ in 0..8 {
        progress = match progress {
            crate::decision::GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::SelectObjects(_),
            ) => {
                paid_exile_cost = true;
                apply_priority_response_with_dm(
                    &mut game,
                    &mut trigger_queue,
                    &mut state,
                    &PriorityResponse::CardCostChoice(spell_id),
                    &mut dm,
                )
                .expect("choosing Jhoira's nonland exile cost should continue activation")
            }
            crate::decision::GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::SelectOptions(cost_ctx),
            ) if cost_ctx
                .description
                .to_ascii_lowercase()
                .contains("choose the next cost") =>
            {
                let option = cost_ctx
                    .options
                    .iter()
                    .find(|option| {
                        option.legal
                            && !paid_exile_cost
                            && option.description.to_ascii_lowercase().contains("exile")
                    })
                    .or_else(|| cost_ctx.options.iter().find(|option| option.legal))
                    .expect("Jhoira should have a payable remaining cost");
                apply_priority_response_with_dm(
                    &mut game,
                    &mut trigger_queue,
                    &mut state,
                    &PriorityResponse::NextCostChoice(option.index),
                    &mut dm,
                )
                .expect("choosing next Jhoira cost should continue activation")
            }
            other => {
                progress = other;
                break;
            }
        };
    }
    assert!(
        matches!(
            progress,
            crate::decision::GameProgress::Continue
                | crate::decision::GameProgress::NeedsDecisionCtx(
                    crate::decisions::context::DecisionContext::Priority(_)
                )
        ),
        "expected Jhoira activation to finish cost payment, got {progress:?}"
    );
    resolve_stack_entry(&mut game).expect("Jhoira activation should resolve");

    assert!(
        game.player(alice)
            .expect("Alice should exist")
            .hand
            .contains(&land_id),
        "Jhoira's exile cost must not choose the land card"
    );
    assert!(
        !game
            .player(alice)
            .expect("Alice should exist")
            .hand
            .contains(&spell_id),
        "Jhoira should move the nonland card out of hand"
    );
    let exiled_id = *game
        .exile
        .iter()
        .find(|&&id| {
            game.object(id)
                .is_some_and(|object| object.name == "Suspend Gift Probe")
        })
        .expect("Jhoira should exile the chosen nonland card");
    assert_eq!(
        game.counter_count(exiled_id, crate::object::CounterType::Time),
        4,
        "Jhoira should put four time counters on the exiled card"
    );

    game.turn.phase = Phase::Beginning;
    game.turn.step = Some(crate::game_state::Step::Upkeep);
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    generate_and_queue_step_triggers(&mut game, &mut trigger_queue);
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "the suspend ability granted by Jhoira should trigger from exile"
    );
    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("granted suspend upkeep trigger should go on the stack");
    resolve_stack_entry(&mut game).expect("granted suspend upkeep trigger should resolve");
    assert_eq!(
        game.counter_count(exiled_id, crate::object::CounterType::Time),
        3,
        "the granted suspend upkeep trigger should remove a time counter"
    );

    for expected_counters in [2, 1, 0] {
        game.turn.turn_number += 1;
        game.turn.phase = Phase::Beginning;
        game.turn.step = Some(crate::game_state::Step::Upkeep);
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);
        generate_and_queue_step_triggers(&mut game, &mut trigger_queue);
        assert_eq!(
            trigger_queue.entries.len(),
            1,
            "Jhoira-granted suspend should keep queueing upkeep triggers while time counters remain"
        );
        put_triggers_on_stack(&mut game, &mut trigger_queue)
            .expect("granted suspend upkeep trigger should go on the stack");
        resolve_stack_entry_with_dm_and_triggers(&mut game, &mut dm, &mut trigger_queue)
            .expect("granted suspend upkeep trigger should resolve");
        assert_eq!(
            game.counter_count(exiled_id, crate::object::CounterType::Time),
            expected_counters,
            "granted suspend upkeep trigger should remove the next time counter"
        );
    }

    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("Jhoira-granted suspend last-counter trigger should go on the stack");
    resolve_stack_entry_with_dm_and_triggers(&mut game, &mut dm, &mut trigger_queue)
        .expect("Jhoira-granted suspend cast trigger should resolve");
    assert_eq!(
        game.stack.len(),
        1,
        "Jhoira-granted suspend should cast the exiled creature when the last time counter is removed"
    );

    resolve_stack_entry(&mut game).expect("Jhoira-granted suspended creature should resolve");

    let creature_id = *game
        .battlefield
        .iter()
        .find(|&&id| {
            game.object(id)
                .is_some_and(|object| object.name == "Suspend Gift Probe")
        })
        .expect("Jhoira-granted suspended creature should enter the battlefield");

    let has_haste = game
        .current_abilities(creature_id)
        .expect("Jhoira-granted suspended creature should exist")
        .iter()
        .any(|ability| {
            matches!(&ability.kind, AbilityKind::Static(static_ability) if static_ability.has_haste())
        });
    assert!(
        has_haste,
        "Jhoira-granted suspended creature should gain suspend haste"
    );

    game.set_current_controller(creature_id, bob);

    let has_haste_after_control_change = game
        .current_abilities(creature_id)
        .expect("Jhoira-granted suspended creature should still exist")
        .iter()
        .any(|ability| {
            matches!(&ability.kind, AbilityKind::Static(static_ability) if static_ability.has_haste())
        });
    assert!(
        !has_haste_after_control_change,
        "Jhoira-granted suspend haste should end once its controller stops controlling it"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn the_face_of_boe_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(73_100), "The Face of Boe")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::Blue],
            vec![ManaSymbol::Red],
            vec![ManaSymbol::White],
        ]))
        .supertypes(vec![Supertype::Legendary])
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Alien, Subtype::Advisor])
        .power_toughness(PowerToughness::fixed(0, 4))
        .parse_text(
            "{T}: You may cast a spell with suspend from your hand. If you do, pay its suspend cost rather than its mana cost. Activate only as a sorcery.",
        )
        .expect("The Face of Boe should parse")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn the_face_of_boe_effects(
    def: &crate::cards::CardDefinition,
) -> Vec<crate::effect::Effect> {
    def.abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => {
                Some(activated.effects.flattened_default_effects().to_vec())
            }
            _ => None,
        })
        .expect("The Face of Boe should have an activated ability")
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn the_face_of_boe_activation_is_sorcery_speed() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let face_def = the_face_of_boe_definition();
    let face_id = game.create_object_from_definition(&face_def, alice, Zone::Battlefield);
    game.remove_summoning_sickness(face_id);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    assert!(
        crate::decision::compute_legal_actions(&game, alice)
            .into_iter()
            .any(|action| matches!(
                action,
                crate::decision::LegalAction::ActivateAbility { source, .. }
                    if source == face_id
            )),
        "The Face of Boe should be activatable during its controller's main phase"
    );

    game.turn.phase = Phase::Combat;
    assert!(
        !crate::decision::compute_legal_actions(&game, alice)
            .into_iter()
            .any(|action| matches!(
                action,
                crate::decision::LegalAction::ActivateAbility { source, .. }
                    if source == face_id
            )),
        "The Face of Boe should not be activatable outside sorcery timing"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn the_face_of_boe_casts_suspend_spell_from_hand_for_suspend_cost() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let face_def = the_face_of_boe_definition();
    let face_id = game.create_object_from_definition(&face_def, alice, Zone::Battlefield);
    let effects = the_face_of_boe_effects(&face_def);

    let suspend_spell = CardDefinitionBuilder::new(CardId::from_raw(73_101), "Suspend Cost Probe")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(6)],
            vec![ManaSymbol::Blue],
        ]))
        .card_types(vec![CardType::Sorcery])
        .parse_text("Suspend 3—{U}\nDraw a card.")
        .expect("suspend spell should parse");
    let _spell_id = game.create_object_from_definition(&suspend_spell, alice, Zone::Hand);
    game.player_mut(alice)
        .expect("Alice should exist")
        .mana_pool
        .add(ManaSymbol::Blue, 1);

    let mut dm = SelectFirstDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new(face_id, alice, &mut dm);
    for effect in &effects {
        crate::effects::execute_effect(&mut game, effect, &mut ctx)
            .expect("The Face of Boe effect should resolve");
    }

    assert_eq!(
        game.stack.len(),
        1,
        "expected the suspend spell on the stack"
    );
    let stack_entry = game.stack.last().expect("stack entry should exist");
    assert!(
        game.object(stack_entry.object_id)
            .is_some_and(|object| object.name == "Suspend Cost Probe"),
        "expected the suspend spell object on the stack"
    );
    assert!(matches!(
        stack_entry.casting_method,
        crate::alternative_cast::CastingMethod::Alternative(_)
    ));
    assert_eq!(
        game.player(alice)
            .expect("Alice should exist")
            .mana_pool
            .blue,
        0,
        "The Face of Boe should spend the suspend cost, not cast for free"
    );
    assert!(
        game.object(stack_entry.object_id)
            .is_some_and(|object| object.zone == Zone::Stack),
        "The Face of Boe should cast the suspend spell from hand, not exile it with time counters"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn the_face_of_boe_casting_suspend_creature_does_not_grant_suspend_haste() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let face_def = the_face_of_boe_definition();
    let face_id = game.create_object_from_definition(&face_def, alice, Zone::Battlefield);
    let effects = the_face_of_boe_effects(&face_def);

    let suspend_creature =
        CardDefinitionBuilder::new(CardId::from_raw(73_104), "Suspend Creature Probe")
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(6)],
                vec![ManaSymbol::Green],
            ]))
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Beast])
            .power_toughness(PowerToughness::fixed(3, 3))
            .parse_text("Suspend 3—{G}")
            .expect("suspend creature should parse");
    game.create_object_from_definition(&suspend_creature, alice, Zone::Hand);
    game.player_mut(alice)
        .expect("Alice should exist")
        .mana_pool
        .add(ManaSymbol::Green, 1);

    let mut dm = SelectFirstDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new(face_id, alice, &mut dm);
    for effect in &effects {
        crate::effects::execute_effect(&mut game, effect, &mut ctx)
            .expect("The Face of Boe effect should cast the suspend creature");
    }

    resolve_stack_entry(&mut game).expect("suspend creature should resolve");
    let permanent_id = game
        .battlefield
        .iter()
        .copied()
        .find(|id| {
            game.object(*id).is_some_and(|object| {
                object.owner == alice && object.name == "Suspend Creature Probe"
            })
        })
        .expect("suspend creature should enter the battlefield");
    assert!(
        !game.current_has_static_ability_id(
            permanent_id,
            crate::static_abilities::StaticAbilityId::Haste,
        ),
        "The Face of Boe pays the suspend cost but does not cast via the suspend delayed trigger"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn the_face_of_boe_does_not_cast_non_suspend_spell() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let face_def = the_face_of_boe_definition();
    let face_id = game.create_object_from_definition(&face_def, alice, Zone::Battlefield);
    let effects = the_face_of_boe_effects(&face_def);

    let ordinary_spell = CardDefinitionBuilder::new(CardId::from_raw(73_102), "Ordinary Probe")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Blue]]))
        .card_types(vec![CardType::Sorcery])
        .parse_text("Draw a card.")
        .expect("ordinary spell should parse");
    let spell_id = game.create_object_from_definition(&ordinary_spell, alice, Zone::Hand);
    game.player_mut(alice)
        .expect("Alice should exist")
        .mana_pool
        .add(ManaSymbol::Blue, 1);

    let mut dm = SelectFirstDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new(face_id, alice, &mut dm);
    for effect in &effects {
        crate::effects::execute_effect(&mut game, effect, &mut ctx)
            .expect("The Face of Boe effect should resolve without a suspend spell");
    }

    assert!(
        game.stack.is_empty(),
        "non-suspend spells should not be cast"
    );
    assert!(
        game.object(spell_id)
            .is_some_and(|object| object.zone == Zone::Hand),
        "ordinary spell should remain in hand"
    );
    assert_eq!(
        game.player(alice)
            .expect("Alice should exist")
            .mana_pool
            .blue,
        1,
        "no suspend-cost payment should be made when no suspend spell is cast"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn the_face_of_boe_does_not_cast_suspend_spell_without_suspend_cost_mana() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let face_def = the_face_of_boe_definition();
    let face_id = game.create_object_from_definition(&face_def, alice, Zone::Battlefield);
    let effects = the_face_of_boe_effects(&face_def);

    let suspend_spell =
        CardDefinitionBuilder::new(CardId::from_raw(73_103), "Unpaid Suspend Probe")
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(6)],
                vec![ManaSymbol::Blue],
            ]))
            .card_types(vec![CardType::Sorcery])
            .parse_text("Suspend 3—{U}\nDraw a card.")
            .expect("suspend spell should parse");
    game.create_object_from_definition(&suspend_spell, alice, Zone::Hand);

    let mut dm = SelectFirstDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new(face_id, alice, &mut dm);
    for effect in &effects {
        crate::effects::execute_effect(&mut game, effect, &mut ctx)
            .expect("The Face of Boe effect should resolve without payable suspend mana");
    }

    assert!(
        game.stack.is_empty(),
        "unpaid suspend-cost casts should not reach the stack"
    );
    assert!(
        game.player(alice)
            .expect("Alice should exist")
            .hand
            .iter()
            .any(|id| {
                game.object(*id)
                    .is_some_and(|object| object.name == "Unpaid Suspend Probe")
            }),
        "unpaid suspend spell should remain in hand"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn all_hallows_eve_exiles_with_counters_and_returns_graveyard_creatures_after_countdown()
{
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let all_hallows_eve = CardDefinitionBuilder::new(CardId::from_raw(99_300), "All Hallow's Eve")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Exile All Hallow's Eve with two scream counters on it.\n\
             At the beginning of your upkeep, if this card is exiled with a scream counter on it, remove a scream counter from it. If there are no more scream counters on it, put it into your graveyard and each player returns all creature cards from their graveyard to the battlefield.",
        )
        .expect("All Hallow's Eve text should parse");

    let returned_a = game.create_object_from_card(
        &CardBuilder::new(CardId::from_raw(99_301), "Returned Card Alpha")
            .card_types(vec![CardType::Creature])
            .build(),
        alice,
        Zone::Graveyard,
    );
    let returned_a_stable_id = game
        .object(returned_a)
        .expect("first return card should exist")
        .stable_id;
    let returned_b = game.create_object_from_card(
        &CardBuilder::new(CardId::from_raw(99_302), "Returned Card Beta")
            .card_types(vec![CardType::Creature])
            .build(),
        bob,
        Zone::Graveyard,
    );
    let returned_b_stable_id = game
        .object(returned_b)
        .expect("second return card should exist")
        .stable_id;

    let spell_id = game.create_object_from_definition(&all_hallows_eve, alice, Zone::Stack);
    let (spell_stable_id, spell_name) = game
        .object(spell_id)
        .map(|object| (object.stable_id, object.name.to_string()))
        .expect("spell should exist on stack");
    game.push_to_stack(
        StackEntry::new(spell_id, alice).with_source_info(spell_stable_id, spell_name),
    );

    resolve_stack_entry(&mut game).expect("All Hallow's Eve should resolve");

    let exiled_id = game
        .find_object_by_stable_id(spell_stable_id)
        .expect("All Hallow's Eve should still be trackable after exile");
    assert!(
        game.object(exiled_id)
            .is_some_and(|object| object.zone == Zone::Exile),
        "All Hallow's Eve should be in exile after resolving"
    );
    assert_eq!(
        game.counter_count(exiled_id, crate::object::CounterType::Named("scream")),
        2,
        "All Hallow's Eve should enter exile with two scream counters"
    );

    game.turn.phase = Phase::Beginning;
    game.turn.step = Some(crate::game_state::Step::Upkeep);
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    generate_and_queue_step_triggers(&mut game, &mut trigger_queue);
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "All Hallow's Eve should trigger from exile while it has a scream counter"
    );
    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("All Hallow's Eve upkeep trigger should go on the stack");
    resolve_stack_entry(&mut game).expect("first All Hallow's Eve upkeep trigger should resolve");
    assert_eq!(
        game.counter_count(exiled_id, crate::object::CounterType::Named("scream")),
        1,
        "first upkeep trigger should remove one scream counter"
    );
    assert!(
        game.object(exiled_id)
            .is_some_and(|object| object.zone == Zone::Exile),
        "All Hallow's Eve should remain exiled with one counter"
    );

    generate_and_queue_step_triggers(&mut game, &mut trigger_queue);
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "All Hallow's Eve should trigger again while it has its last scream counter"
    );
    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("second All Hallow's Eve upkeep trigger should go on the stack");
    resolve_stack_entry(&mut game).expect("second All Hallow's Eve upkeep trigger should resolve");

    let final_id = game
        .find_object_by_stable_id(spell_stable_id)
        .expect("All Hallow's Eve should still be trackable after moving to graveyard");
    assert!(
        game.object(final_id)
            .is_some_and(|object| object.zone == Zone::Graveyard),
        "All Hallow's Eve should go to its owner's graveyard when the last counter is removed"
    );
    let returned_a_current_id = game
        .find_object_by_stable_id(returned_a_stable_id)
        .expect("Alice's returned card should still be trackable after zone change");
    let returned_b_current_id = game
        .find_object_by_stable_id(returned_b_stable_id)
        .expect("Bob's returned card should still be trackable after zone change");
    for (current_id, expected_controller) in
        [(returned_a_current_id, alice), (returned_b_current_id, bob)]
    {
        let object = game
            .object(current_id)
            .expect("returned card should still exist after zone change");
        assert_eq!(
            object.zone,
            Zone::Battlefield,
            "creature cards from both graveyards should return to the battlefield"
        );
        assert_eq!(
            game.controller_of(object),
            expected_controller,
            "each returned creature should enter under its owner's control"
        );
        assert!(
            game.battlefield.contains(&current_id),
            "returned creature should be present in the battlefield index used by UI snapshots"
        );
    }
    assert!(
        !game.players[0].graveyard.contains(&returned_a_current_id)
            && !game.players[1].graveyard.contains(&returned_b_current_id),
        "returned creatures should leave both players' graveyard indexes"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn all_hallows_eve_sees_creatures_put_into_opponents_graveyard_by_sbas_before_it_resolves()
 {
    use crate::decision::LegalAction;

    struct YawgmothIntoAllHallowsDecisionMaker {
        yawgmoth: ObjectId,
        sacrifice: ObjectId,
        target: ObjectId,
        activated: bool,
    }

    impl DecisionMaker for YawgmothIntoAllHallowsDecisionMaker {
        fn decide_priority(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::PriorityContext,
        ) -> LegalAction {
            if !self.activated
                && let Some(action) = ctx.actions.iter().find(|action| {
                    matches!(
                        action,
                        LegalAction::ActivateAbility { source, .. } if *source == self.yawgmoth
                    )
                })
            {
                self.activated = true;
                return action.clone();
            }

            LegalAction::PassPriority
        }

        fn decide_targets(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::TargetsContext,
        ) -> Vec<Target> {
            if ctx.requirements.iter().any(|requirement| {
                requirement
                    .legal_targets
                    .contains(&Target::Object(self.target))
            }) {
                vec![Target::Object(self.target)]
            } else {
                Vec::new()
            }
        }

        fn decide_options(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            ctx.options
                .iter()
                .find(|option| {
                    option.legal && option.description.to_ascii_lowercase().contains("life")
                })
                .or_else(|| ctx.options.iter().find(|option| option.legal))
                .map(|option| vec![option.index])
                .unwrap_or_default()
        }

        fn decide_objects(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            if ctx
                .candidates
                .iter()
                .any(|candidate| candidate.legal && candidate.id == self.sacrifice)
            {
                vec![self.sacrifice]
            } else {
                Vec::new()
            }
        }
    }

    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    game.turn.phase = Phase::Beginning;
    game.turn.step = Some(crate::game_state::Step::Upkeep);
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    // Yawgmoth's activated ability draws a card. Keep this scenario focused on
    // the SBA/stack ordering instead of making Alice lose to an empty library.
    let draw_fodder = CardBuilder::new(CardId::from_raw(99_312), "Yawgmoth Draw Fodder")
        .card_types(vec![CardType::Artifact])
        .build();
    game.create_object_from_card(&draw_fodder, alice, Zone::Library);

    let all_hallows_eve = CardDefinitionBuilder::new(CardId::from_raw(99_310), "All Hallow's Eve")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Exile All Hallow's Eve with two scream counters on it.\n\
             At the beginning of your upkeep, if this card is exiled with a scream counter on it, remove a scream counter from it. If there are no more scream counters on it, put it into your graveyard and each player returns all creature cards from their graveyard to the battlefield.",
        )
        .expect("All Hallow's Eve text should parse");
    let all_hallows_id = game.create_object_from_definition(&all_hallows_eve, alice, Zone::Exile);
    game.add_counters(
        all_hallows_id,
        crate::object::CounterType::Named("scream"),
        1,
    );

    let registry =
        crate::cards::CardRegistry::with_builtin_cards_for_names(["Yawgmoth, Thran Physician"]);
    let yawgmoth_def = registry
        .get("Yawgmoth, Thran Physician")
        .expect("Yawgmoth, Thran Physician should be present in registry");
    let yawgmoth_id = game.create_object_from_definition(yawgmoth_def, alice, Zone::Battlefield);

    let myr = CardDefinitionBuilder::new(CardId::from_raw(99_311), "Myr Moonvessel")
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .parse_text("When this creature dies, add {C}.")
        .expect("Myr Moonvessel should parse");
    let alice_myr_id = game.create_object_from_definition(&myr, alice, Zone::Battlefield);
    let bob_myr_id = game.create_object_from_definition(&myr, bob, Zone::Battlefield);
    let alice_myr_stable_id = game
        .object(alice_myr_id)
        .expect("Alice's Myr should exist")
        .stable_id;
    let bob_myr_stable_id = game
        .object(bob_myr_id)
        .expect("Bob's Myr should exist")
        .stable_id;

    generate_and_queue_step_triggers(&mut game, &mut trigger_queue);
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "All Hallow's Eve should trigger from exile with one scream counter"
    );
    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("All Hallow's Eve trigger should go on the stack");

    let mut dm = YawgmothIntoAllHallowsDecisionMaker {
        yawgmoth: yawgmoth_id,
        sacrifice: alice_myr_id,
        target: bob_myr_id,
        activated: false,
    };
    let result = run_priority_loop_with(&mut game, &mut trigger_queue, &mut dm)
        .expect("priority loop should resolve Yawgmoth and then All Hallow's Eve");
    assert!(
        matches!(result, crate::decision::GameProgress::Continue),
        "priority loop should finish the upkeep priority window, got {result:?}"
    );
    assert!(dm.activated, "the test should activate Yawgmoth once");

    for (stable_id, expected_controller) in [(alice_myr_stable_id, alice), (bob_myr_stable_id, bob)]
    {
        let current_id = game
            .find_object_by_stable_id(stable_id)
            .expect("Myr should still be trackable after returning");
        let object = game.object(current_id).expect("Myr should still exist");
        assert_eq!(
            object.zone,
            Zone::Battlefield,
            "All Hallow's Eve should return Myr Moonvessel from each player's graveyard"
        );
        assert_eq!(
            game.controller_of(object),
            expected_controller,
            "each returned Myr should enter under its owner's control"
        );
        assert!(
            object.counters.is_empty(),
            "counters from a previous zone should not remain on the returned Myr"
        );
        assert!(
            game.battlefield.contains(&current_id),
            "returned Myr should be present in the battlefield index"
        );
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn oath_of_druids_upkeep_trigger_puts_revealed_creature_onto_battlefield() {
    #[derive(Debug)]
    struct OathDecisionMaker {
        expected_chooser: PlayerId,
        expected_target: PlayerId,
        target_context: Option<crate::decisions::context::TargetsContext>,
    }

    impl DecisionMaker for OathDecisionMaker {
        fn decide_targets(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::TargetsContext,
        ) -> Vec<Target> {
            self.target_context = Some(ctx.clone());
            assert_eq!(
                ctx.player, self.expected_chooser,
                "the active player must choose Oath of Druids' target"
            );
            let target = Target::Player(self.expected_target);
            ctx.requirements
                .first()
                .is_some_and(|requirement| requirement.legal_targets.contains(&target))
                .then_some(target)
                .into_iter()
                .collect()
        }

        fn decide_boolean(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::BooleanContext,
        ) -> bool {
            true
        }
    }

    let mut game = GameState::new(
        vec![
            "Alice".to_string(),
            "Bob".to_string(),
            "Charlie".to_string(),
        ],
        20,
    );
    let mut trigger_queue = TriggerQueue::new();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);

    let oath = CardDefinitionBuilder::new(CardId::from_raw(99_100), "Oath of Druids")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "At the beginning of each player's upkeep, that player chooses target player who controls more creatures than they do and is their opponent. The first player may reveal cards from the top of their library until they reveal a creature card. If the first player does, that player puts that card onto the battlefield and all other cards revealed this way into their graveyard.",
        )
        .expect("Oath of Druids should parse for runtime test");
    let _oath_id = game.create_object_from_definition(&oath, alice, Zone::Battlefield);

    create_creature(&mut game, "Alice Bear", alice, 2, 2);
    create_creature(&mut game, "Alice Wolf", alice, 2, 2);
    create_creature(&mut game, "Bob Scout", bob, 1, 1);
    create_creature(&mut game, "Charlie Scout", charlie, 1, 1);

    let bottom_library_id = game.create_object_from_card(
        &CardBuilder::new(CardId::from_raw(99_101), "Bottom Forest")
            .card_types(vec![CardType::Land])
            .build(),
        bob,
        Zone::Library,
    );
    let _creature_library_id = game.create_object_from_card(
        &CardBuilder::new(CardId::from_raw(99_102), "Revealed Beast")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(4, 4))
            .build(),
        bob,
        Zone::Library,
    );
    let _top_library_id = game.create_object_from_card(
        &CardBuilder::new(CardId::from_raw(99_103), "Top Relic")
            .card_types(vec![CardType::Artifact])
            .build(),
        bob,
        Zone::Library,
    );

    game.turn.phase = Phase::Beginning;
    game.turn.step = Some(crate::game_state::Step::Upkeep);
    game.turn.active_player = bob;
    game.turn.priority_player = Some(bob);

    generate_and_queue_step_triggers(&mut game, &mut trigger_queue);

    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "Oath of Druids should trigger on the upkeep where an opponent has more creatures"
    );

    let mut dm = OathDecisionMaker {
        expected_chooser: bob,
        expected_target: alice,
        target_context: None,
    };
    put_triggers_on_stack_with_dm(&mut game, &mut trigger_queue, &mut dm)
        .expect("Oath of Druids trigger should go on the stack");

    let target_context = dm.target_context.as_ref().expect("Oath target prompt");
    let legal_targets = &target_context.requirements[0].legal_targets;
    assert!(
        legal_targets.contains(&Target::Player(alice)),
        "Alice is Bob's opponent and controls more creatures than Bob"
    );
    assert!(
        !legal_targets.contains(&Target::Player(bob)),
        "the active player can't target themselves"
    );
    assert!(
        !legal_targets.contains(&Target::Player(charlie)),
        "an opponent with an equal creature count isn't legal"
    );
    assert_eq!(
        game.stack.last().map(|entry| entry.targets.as_slice()),
        Some(&[Target::Player(alice)][..]),
        "the active player's chosen opponent must be retained on the stack"
    );

    resolve_stack_entry_with(&mut game, &mut dm)
        .expect("Oath of Druids trigger should resolve when accepted");

    assert!(
        game.battlefield.iter().any(|&id| {
            game.object(id)
                .is_some_and(|obj| obj.name == "Revealed Beast" && game.controller_of(obj) == bob)
        }),
        "the first revealed creature should enter under the active player's control"
    );
    assert!(
        game.player(bob)
            .expect("bob exists")
            .graveyard
            .iter()
            .any(|&id| {
                game.object(id)
                    .is_some_and(|obj| obj.name == "Top Relic" && obj.owner == bob)
            }),
        "noncreature cards revealed before the creature should go to that player's graveyard"
    );
    assert!(
        game.object(bottom_library_id)
            .is_some_and(|obj| obj.zone == Zone::Library && obj.owner == bob),
        "cards below the first revealed creature should stay in the library"
    );
    assert_eq!(
        game.player(bob).expect("bob exists").graveyard.len(),
        1,
        "only the noncreature card revealed before the creature should hit the graveyard"
    );
    assert_eq!(
        game.player(bob).expect("bob exists").library.len(),
        1,
        "resolving Oath should stop after the first creature is revealed"
    );
}

pub(super) fn dream_tides_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(99_120), "Dream Tides")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Blue],
            vec![ManaSymbol::Blue],
        ]))
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "Creatures don't untap during their controllers' untap steps.\n\
             At the beginning of each player's upkeep, that player may choose any number of tapped nongreen creatures they control and pay {2} for each creature chosen this way. If the player does, untap those creatures.",
        )
        .expect("Dream Tides should parse for runtime tests")
}

pub(super) fn create_colored_creature(
    game: &mut GameState,
    name: &str,
    owner: PlayerId,
    colors: Option<crate::color::ColorSet>,
) -> ObjectId {
    let mut builder = CardBuilder::new(CardId::from_raw(99_121), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2));
    if let Some(colors) = colors {
        builder = builder.color_indicator(colors);
    }
    let card = builder.build();
    game.create_object_from_card(&card, owner, Zone::Battlefield)
}

pub(super) fn put_dream_tides_upkeep_trigger_on_stack(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
) {
    generate_and_queue_step_triggers(game, trigger_queue);
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "Dream Tides should create one upkeep trigger"
    );
    put_triggers_on_stack(game, trigger_queue).expect("Dream Tides trigger should go on the stack");
}

#[test]
pub(super) fn dream_tides_prevents_creatures_from_untapping_during_untap_step() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let dream_tides = dream_tides_definition();
    game.create_object_from_definition(&dream_tides, alice, Zone::Battlefield);
    let bob_creature = create_creature(&mut game, "Bob Bear", bob, 2, 2);
    let bob_relic = game.create_object_from_card(
        &CardBuilder::new(CardId::from_raw(99_122), "Bob Relic")
            .card_types(vec![CardType::Artifact])
            .build(),
        bob,
        Zone::Battlefield,
    );
    game.tap(bob_creature);
    game.tap(bob_relic);

    game.turn.active_player = bob;
    game.turn.phase = Phase::Beginning;
    game.turn.step = Some(crate::game_state::Step::Untap);
    let mut dm = AutoPassDecisionMaker;
    crate::turn::execute_untap_step_with(&mut game, &mut dm);

    assert!(
        game.is_tapped(bob_creature),
        "Dream Tides should keep creatures tapped during their controller's untap step"
    );
    assert!(
        !game.is_tapped(bob_relic),
        "Dream Tides should not stop noncreature permanents from untapping"
    );
}

pub(super) struct DreamTidesChoiceDecisionMaker {
    pub(super) chooser: PlayerId,
    pub(super) selected: Vec<ObjectId>,
    pub(super) rejected: Vec<ObjectId>,
}

impl DecisionMaker for DreamTidesChoiceDecisionMaker {
    fn decide_boolean(
        &mut self,
        _game: &GameState,
        _ctx: &crate::decisions::context::BooleanContext,
    ) -> bool {
        true
    }

    fn decide_objects(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        assert_eq!(
            ctx.player, self.chooser,
            "active player should choose for Dream Tides"
        );
        assert_eq!(
            ctx.min, 0,
            "Dream Tides should allow choosing zero creatures"
        );
        assert_eq!(
            ctx.max,
            Some(ctx.candidates.len()),
            "Dream Tides any-number choice should be capped only by available candidates"
        );
        for selected in &self.selected {
            assert!(
                ctx.candidates
                    .iter()
                    .any(|candidate| candidate.id == *selected && candidate.legal),
                "selected tapped nongreen creature should be legal for Dream Tides"
            );
        }
        for rejected in &self.rejected {
            assert!(
                !ctx.candidates
                    .iter()
                    .any(|candidate| candidate.id == *rejected && candidate.legal),
                "green, untapped, and non-controlled creatures should not be legal \
                 Dream Tides choices"
            );
        }
        self.selected.clone()
    }

    fn decide_options(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        ctx.options
            .iter()
            .find(|option| option.legal)
            .map(|option| vec![option.index])
            .unwrap_or_default()
    }
}

#[test]
pub(super) fn dream_tides_upkeep_payment_untaps_only_chosen_tapped_nongreen_creature() {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let dream_tides = dream_tides_definition();
    game.create_object_from_definition(&dream_tides, alice, Zone::Battlefield);
    let first_chosen = create_colored_creature(&mut game, "First Chosen Bob Bear", bob, None);
    let second_chosen = create_colored_creature(&mut game, "Second Chosen Bob Bear", bob, None);
    let unchosen = create_colored_creature(&mut game, "Unchosen Bob Bear", bob, None);
    let green = create_colored_creature(
        &mut game,
        "Green Bob Bear",
        bob,
        Some(crate::color::ColorSet::GREEN),
    );
    let untapped = create_colored_creature(&mut game, "Untapped Bob Bear", bob, None);
    let alice_creature = create_colored_creature(&mut game, "Alice Bear", alice, None);
    game.tap(first_chosen);
    game.tap(second_chosen);
    game.tap(unchosen);
    game.tap(green);
    game.tap(alice_creature);
    game.player_mut(bob)
        .expect("Bob exists")
        .mana_pool
        .add(ManaSymbol::Colorless, 4);

    game.turn.active_player = bob;
    game.turn.phase = Phase::Beginning;
    game.turn.step = Some(crate::game_state::Step::Upkeep);
    put_dream_tides_upkeep_trigger_on_stack(&mut game, &mut trigger_queue);

    let mut dm = DreamTidesChoiceDecisionMaker {
        chooser: bob,
        selected: vec![first_chosen, second_chosen],
        rejected: vec![green, untapped, alice_creature],
    };
    resolve_stack_entry_with(&mut game, &mut dm).expect("Dream Tides trigger should resolve");

    assert!(
        !game.is_tapped(first_chosen) && !game.is_tapped(second_chosen),
        "paid-for chosen creatures should untap"
    );
    assert!(
        game.is_tapped(unchosen),
        "unchosen tapped nongreen creatures should remain tapped"
    );
    assert!(
        game.is_tapped(green),
        "green creatures should not be legal choices"
    );
    assert!(
        !game.is_tapped(untapped),
        "untapped creatures should remain unchanged"
    );
    assert!(
        game.is_tapped(alice_creature),
        "active player should not choose creatures they do not control"
    );
}

#[test]
pub(super) fn dream_tides_upkeep_without_payment_leaves_chosen_creature_tapped() {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let dream_tides = dream_tides_definition();
    game.create_object_from_definition(&dream_tides, alice, Zone::Battlefield);
    let chosen = create_colored_creature(&mut game, "Unpaid Bob Bear", bob, None);
    game.tap(chosen);

    game.turn.active_player = bob;
    game.turn.phase = Phase::Beginning;
    game.turn.step = Some(crate::game_state::Step::Upkeep);
    put_dream_tides_upkeep_trigger_on_stack(&mut game, &mut trigger_queue);

    let mut dm = DreamTidesChoiceDecisionMaker {
        chooser: bob,
        selected: vec![chosen],
        rejected: Vec::new(),
    };
    resolve_stack_entry_with(&mut game, &mut dm)
        .expect("Dream Tides trigger should resolve even when payment is impossible");

    assert!(
        game.is_tapped(chosen),
        "chosen creature should remain tapped when its controller cannot pay {{2}}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn aether_rift_returns_randomly_discarded_creature_when_no_player_pays() {
    struct DeclineUnlessPayment;

    impl DecisionMaker for DeclineUnlessPayment {
        fn decide_boolean(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::BooleanContext,
        ) -> bool {
            false
        }
    }

    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let alice = PlayerId::from_index(0);

    let aether_rift = CardDefinitionBuilder::new(CardId::from_raw(99_150), "Aether Rift")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "At the beginning of your upkeep, discard a card at random. If you discard a creature card this way, return it from your graveyard to the battlefield unless any player pays 5 life.",
        )
        .expect("Aether Rift should parse for runtime test");
    let _rift_id = game.create_object_from_definition(&aether_rift, alice, Zone::Battlefield);
    let _creature_id = game.create_object_from_card(
        &CardBuilder::new(CardId::from_raw(99_151), "Rift Beast")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(3, 3))
            .build(),
        alice,
        Zone::Hand,
    );

    game.turn.phase = Phase::Beginning;
    game.turn.step = Some(crate::game_state::Step::Upkeep);
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    generate_and_queue_step_triggers(&mut game, &mut trigger_queue);
    assert_eq!(trigger_queue.entries.len(), 1, "Aether Rift should trigger");

    put_triggers_on_stack(&mut game, &mut trigger_queue).expect("Aether Rift trigger stacks");
    let mut dm = DeclineUnlessPayment;
    resolve_stack_entry_with(&mut game, &mut dm).expect("Aether Rift trigger resolves");

    assert!(
        game.battlefield.iter().any(|&id| {
            game.object(id)
                .is_some_and(|obj| obj.name == "Rift Beast" && game.controller_of(obj) == alice)
        }),
        "discarded creature should return to the battlefield"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn aether_rift_leaves_randomly_discarded_noncreature_in_graveyard() {
    struct DeclineUnlessPayment;

    impl DecisionMaker for DeclineUnlessPayment {
        fn decide_boolean(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::BooleanContext,
        ) -> bool {
            false
        }
    }

    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let alice = PlayerId::from_index(0);

    let aether_rift = CardDefinitionBuilder::new(CardId::from_raw(99_152), "Aether Rift")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "At the beginning of your upkeep, discard a card at random. If you discard a creature card this way, return it from your graveyard to the battlefield unless any player pays 5 life.",
        )
        .expect("Aether Rift should parse for noncreature runtime test");
    let _rift_id = game.create_object_from_definition(&aether_rift, alice, Zone::Battlefield);
    let _relic_id = game.create_object_from_card(
        &CardBuilder::new(CardId::from_raw(99_153), "Rift Relic")
            .card_types(vec![CardType::Artifact])
            .build(),
        alice,
        Zone::Hand,
    );

    game.turn.phase = Phase::Beginning;
    game.turn.step = Some(crate::game_state::Step::Upkeep);
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    generate_and_queue_step_triggers(&mut game, &mut trigger_queue);
    assert_eq!(trigger_queue.entries.len(), 1, "Aether Rift should trigger");

    put_triggers_on_stack(&mut game, &mut trigger_queue).expect("Aether Rift trigger stacks");
    let mut dm = DeclineUnlessPayment;
    resolve_stack_entry_with(&mut game, &mut dm).expect("Aether Rift trigger resolves");

    assert!(
        game.player(alice)
            .expect("alice exists")
            .graveyard
            .iter()
            .any(|&id| game.object(id).is_some_and(|obj| obj.name == "Rift Relic")),
        "discarded noncreature should stay in the graveyard"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn mind_funeral_mills_until_four_lands_and_moves_every_revealed_card_to_graveyard() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let mind_funeral = CardDefinitionBuilder::new(CardId::from_raw(99_200), "Mind Funeral")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Target opponent reveals cards from the top of their library until four land cards are revealed. That player puts all cards revealed this way into their graveyard.",
        )
        .expect("Mind Funeral should parse for runtime test");

    let spell_id = game.create_object_from_definition(&mind_funeral, alice, Zone::Stack);
    game.push_to_stack(
        StackEntry::new(spell_id, alice)
            .with_targets(vec![Target::Player(bob)])
            .with_target_assignments(vec![crate::game_state::TargetAssignment {
                spec: ChooseSpec::target_opponent(),
                range: 0..1,
            }]),
    );

    let make_library_card = |id: u32, name: &str, card_types: Vec<CardType>| {
        CardBuilder::new(CardId::from_raw(id), name)
            .card_types(card_types)
            .build()
    };

    game.create_object_from_card(
        &make_library_card(99_201, "Bottom Archive", vec![CardType::Artifact]),
        bob,
        Zone::Library,
    );
    game.create_object_from_card(
        &make_library_card(99_202, "Eighth Land", vec![CardType::Land]),
        bob,
        Zone::Library,
    );
    game.create_object_from_card(
        &make_library_card(99_203, "Seventh Land", vec![CardType::Land]),
        bob,
        Zone::Library,
    );
    game.create_object_from_card(
        &make_library_card(99_204, "Sixth Charm", vec![CardType::Sorcery]),
        bob,
        Zone::Library,
    );
    game.create_object_from_card(
        &make_library_card(99_205, "Fifth Land", vec![CardType::Land]),
        bob,
        Zone::Library,
    );
    game.create_object_from_card(
        &make_library_card(99_206, "Fourth Pulse", vec![CardType::Instant]),
        bob,
        Zone::Library,
    );
    game.create_object_from_card(
        &make_library_card(99_207, "Third Land", vec![CardType::Land]),
        bob,
        Zone::Library,
    );
    game.create_object_from_card(
        &make_library_card(99_208, "Second Shade", vec![CardType::Creature]),
        bob,
        Zone::Library,
    );
    game.create_object_from_card(
        &make_library_card(99_209, "Top Relic", vec![CardType::Artifact]),
        bob,
        Zone::Library,
    );

    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;

    let mut dm = AutoPassDecisionMaker;
    resolve_stack_entry_with(&mut game, &mut dm).expect("Mind Funeral should resolve");

    let mut graveyard_names = game
        .player(bob)
        .expect("bob exists")
        .graveyard
        .iter()
        .map(|id| {
            game.object(*id)
                .expect("graveyard object exists")
                .name
                .clone()
        })
        .collect::<Vec<_>>();
    graveyard_names.sort();
    assert_eq!(
        graveyard_names,
        vec![
            "Eighth Land".to_string(),
            "Fifth Land".to_string(),
            "Fourth Pulse".to_string(),
            "Second Shade".to_string(),
            "Seventh Land".to_string(),
            "Sixth Charm".to_string(),
            "Third Land".to_string(),
            "Top Relic".to_string(),
        ],
        "Mind Funeral should move every revealed card into the graveyard"
    );
    assert_eq!(
        game.player(bob)
            .expect("bob exists")
            .library
            .iter()
            .map(|id| game
                .object(*id)
                .expect("library object exists")
                .name
                .clone())
            .collect::<Vec<_>>(),
        vec!["Bottom Archive".to_string()],
        "Mind Funeral should stop after the fourth land card is revealed"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn wild_dogs_upkeep_trigger_hands_control_to_the_life_leader() {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let wild_dogs = CardDefinitionBuilder::new(CardId::from_raw(99_104), "Wild Dogs")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Green]]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![crate::types::Subtype::Dog])
        .power_toughness(PowerToughness::fixed(2, 1))
        .parse_text(
            "At the beginning of your upkeep, if a player has more life than each other player, the player with the most life gains control of this creature.\nCycling {2} ({2}, Discard this card: Draw a card.)",
        )
        .expect("Wild Dogs should parse for the runtime regression test");
    let wild_dogs_id = game.create_object_from_definition(&wild_dogs, alice, Zone::Battlefield);

    game.turn.phase = Phase::Beginning;
    game.turn.step = Some(crate::game_state::Step::Upkeep);
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    generate_and_queue_step_triggers(&mut game, &mut trigger_queue);
    assert!(
        trigger_queue.entries.is_empty(),
        "tied life totals should not trigger Wild Dogs"
    );

    game.player_mut(bob).expect("bob exists").life = 21;

    generate_and_queue_step_triggers(&mut game, &mut trigger_queue);
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "Wild Dogs should trigger once a player has the most life"
    );

    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("Wild Dogs upkeep trigger should go on the stack");
    resolve_stack_entry(&mut game).expect("Wild Dogs upkeep trigger should resolve");

    assert_eq!(
        game.current_controller(wild_dogs_id),
        Some(bob),
        "the player with the most life should gain control of Wild Dogs"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn chaos_lord_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(2_614), "Chaos Lord")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(4)],
            vec![ManaSymbol::Red],
            vec![ManaSymbol::Red],
            vec![ManaSymbol::Red],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![crate::types::Subtype::Human])
        .power_toughness(PowerToughness::fixed(7, 7))
        .parse_text(
            "First strike\nAt the beginning of your upkeep, target opponent gains control of this creature if the number of permanents is even.\nThis creature can attack as though it had haste unless it entered this turn.",
        )
        .expect("Chaos Lord should parse for runtime tests")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn chaos_lord_filler_permanent_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(2_615), "Chaos Lord Count Filler")
        .card_types(vec![CardType::Artifact])
        .build()
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn chaos_lord_tap_ability_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(2_616), "Chaos Lord Tap Test")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .parse_text(
            "This creature can attack as though it had haste unless it entered this turn.\n{T}: Add {R}.",
        )
        .expect("tap-test creature should parse")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn queue_chaos_lord_upkeep_trigger(game: &mut GameState) -> TriggerQueue {
    let alice = PlayerId::from_index(0);
    game.turn.phase = Phase::Beginning;
    game.turn.step = Some(crate::game_state::Step::Upkeep);
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let mut trigger_queue = TriggerQueue::new();
    generate_and_queue_step_triggers(game, &mut trigger_queue);
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "Chaos Lord should trigger at the beginning of its controller's upkeep"
    );
    trigger_queue
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn chaos_lord_upkeep_trigger_gives_target_opponent_control_when_permanent_count_is_even()
{
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let chaos_lord = chaos_lord_definition();
    let filler = chaos_lord_filler_permanent_definition();
    let chaos_lord_id = game.create_object_from_definition(&chaos_lord, alice, Zone::Battlefield);
    game.create_object_from_definition(&filler, alice, Zone::Battlefield);

    let mut trigger_queue = queue_chaos_lord_upkeep_trigger(&mut game);
    let mut dm = SelectFirstDecisionMaker;
    put_triggers_on_stack_with_dm(&mut game, &mut trigger_queue, &mut dm)
        .expect("Chaos Lord upkeep trigger should go on the stack with target opponent");
    resolve_stack_entry(&mut game).expect("Chaos Lord upkeep trigger should resolve");

    assert_eq!(
        game.current_controller(chaos_lord_id),
        Some(bob),
        "target opponent should gain control when the number of permanents is even"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn chaos_lord_upkeep_trigger_queues_but_does_not_change_control_when_permanent_count_is_odd()
 {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let chaos_lord = chaos_lord_definition();
    let chaos_lord_id = game.create_object_from_definition(&chaos_lord, alice, Zone::Battlefield);

    let mut trigger_queue = queue_chaos_lord_upkeep_trigger(&mut game);
    let mut dm = SelectFirstDecisionMaker;
    put_triggers_on_stack_with_dm(&mut game, &mut trigger_queue, &mut dm)
        .expect("Chaos Lord upkeep trigger should still go on the stack with a target opponent");
    resolve_stack_entry(&mut game).expect("Chaos Lord upkeep trigger should resolve");

    assert_eq!(
        game.current_controller(chaos_lord_id),
        Some(alice),
        "the conditional control-change effect should do nothing when the number of permanents is odd"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn chaos_lord_can_attack_as_though_hasty_only_if_it_did_not_enter_this_turn() {
    let mut old_permanent_game = setup_game();
    let alice = PlayerId::from_index(0);
    let chaos_lord = chaos_lord_definition();
    let old_chaos_lord =
        old_permanent_game.create_object_from_definition(&chaos_lord, alice, Zone::Battlefield);
    old_permanent_game.set_summoning_sick(old_chaos_lord);
    assert!(
        crate::rules::combat::can_attack(
            old_permanent_game
                .object(old_chaos_lord)
                .expect("old Chaos Lord exists"),
            &old_permanent_game,
        ),
        "Chaos Lord should attack as though it had haste when it is summoning sick but did not enter this turn"
    );

    let mut entered_this_turn_game = setup_game();
    let chaos_lord_in_hand =
        entered_this_turn_game.create_object_from_definition(&chaos_lord, alice, Zone::Hand);
    let entered_chaos_lord = entered_this_turn_game
        .move_object_by_effect(chaos_lord_in_hand, Zone::Battlefield)
        .expect("Chaos Lord should enter the battlefield");
    assert!(
        !crate::rules::combat::can_attack(
            entered_this_turn_game
                .object(entered_chaos_lord)
                .expect("entered Chaos Lord exists"),
            &entered_this_turn_game,
        ),
        "Chaos Lord should not get the as-though-haste attack permission if it entered this turn"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn chaos_lord_attack_as_haste_clause_does_not_grant_haste_for_tap_abilities() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let tap_creature = chaos_lord_tap_ability_definition();
    let tap_creature_id =
        game.create_object_from_definition(&tap_creature, alice, Zone::Battlefield);
    game.set_summoning_sick(tap_creature_id);

    assert!(
        crate::rules::combat::can_attack(
            game.object(tap_creature_id)
                .expect("old tap-test creature exists"),
            &game,
        ),
        "the as-though-haste clause should grant attack-only permission"
    );
    assert!(
        !game.object_has_static_ability_id(
            tap_creature_id,
            crate::static_abilities::StaticAbilityId::Haste,
        ),
        "the as-though-haste attack permission must not become the Haste keyword"
    );
    assert!(
        !crate::decision::compute_legal_actions(&game, alice)
            .iter()
            .any(|action| matches!(
                action,
                crate::decision::LegalAction::ActivateManaAbility { source, .. }
                    if *source == tap_creature_id
            )),
        "summoning-sick tap ability should remain illegal without true haste"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn touch_of_the_eternal_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(278_197), "Touch of the Eternal")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(5)],
            vec![ManaSymbol::White],
            vec![ManaSymbol::White],
        ]))
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "At the beginning of your upkeep, count the number of permanents you control. Your life total becomes that number.",
        )
        .expect("Touch of the Eternal should parse for runtime tests")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn create_touch_counted_permanent(
    game: &mut GameState,
    name: &str,
    controller: PlayerId,
    card_type: CardType,
) -> ObjectId {
    let card = CardBuilder::new(CardId::new(), name)
        .card_types(vec![card_type])
        .build();
    game.create_object_from_card(&card, controller, Zone::Battlefield)
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn touch_of_the_eternal_upkeep_sets_life_to_current_permanents_you_control() {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let touch = touch_of_the_eternal_definition();
    let _touch_id = game.create_object_from_definition(&touch, alice, Zone::Battlefield);
    create_touch_counted_permanent(&mut game, "Alice Relic", alice, CardType::Artifact);
    create_touch_counted_permanent(&mut game, "Bob Relic", bob, CardType::Artifact);
    game.player_mut(alice).expect("alice exists").life = 20;

    game.turn.phase = Phase::Beginning;
    game.turn.step = Some(crate::game_state::Step::Upkeep);
    game.turn.active_player = bob;
    game.turn.priority_player = Some(bob);

    generate_and_queue_step_triggers(&mut game, &mut trigger_queue);
    assert!(
        trigger_queue.entries.is_empty(),
        "Touch of the Eternal should not trigger at the beginning of an opponent's upkeep"
    );

    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    generate_and_queue_step_triggers(&mut game, &mut trigger_queue);
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "Touch of the Eternal should trigger at the beginning of your upkeep"
    );
    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("Touch of the Eternal upkeep trigger should go on the stack");

    create_touch_counted_permanent(&mut game, "Alice Soldier", alice, CardType::Creature);
    resolve_stack_entry(&mut game).expect("Touch of the Eternal trigger should resolve");

    assert_eq!(
        game.player(alice).expect("alice exists").life,
        3,
        "Touch of the Eternal should set life to the current number of permanents Alice controls and ignore Bob's permanent"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn crystalline_resonance_copies_target_permanent_when_you_cycle() {
    use crate::PriorityResponse;
    use crate::decision::{LegalAction, compute_legal_actions};
    use crate::game_loop::apply_decision_context_with_dm;
    use crate::game_loop::{
        PriorityLoopState, apply_priority_response_with_dm,
        resolve_stack_entry_with_dm_and_triggers,
    };
    use crate::zone::Zone;

    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let mut dm = SelectFirstDecisionMaker;
    let mut state = PriorityLoopState::new(game.players_in_game());
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let resonance_def = CardDefinitionBuilder::new(CardId::new(), "Crystalline Resonance")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Blue],
        ]))
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "Whenever you cycle a card, you may have this enchantment become a copy of another target permanent until your next turn, except it has this ability.",
        )
        .expect("Crystalline Resonance should parse");
    let resonance_id = game.create_object_from_definition(&resonance_def, alice, Zone::Battlefield);

    let target_def = CardDefinitionBuilder::new(CardId::new(), "Target Bear")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 3))
        .build();
    let target_id = game.create_object_from_definition(&target_def, bob, Zone::Battlefield);

    let cycling_def = CardDefinitionBuilder::new(CardId::new(), "Cycle Probe One")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]]))
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .parse_text("Cycling {2} ({2}, Discard this card: Draw a card.)")
        .expect("cycling probe should parse");
    let cycling_id = game.create_object_from_definition(&cycling_def, alice, Zone::Hand);
    let cycling_id_two = game.create_object_from_definition(&cycling_def, alice, Zone::Hand);

    let library_card = CardBuilder::new(CardId::new(), "Draw Target One")
        .card_types(vec![CardType::Artifact])
        .build();
    game.create_object_from_card(&library_card, alice, Zone::Library);
    game.create_object_from_card(
        &CardBuilder::new(CardId::new(), "Draw Target Two")
            .card_types(vec![CardType::Artifact])
            .build(),
        alice,
        Zone::Library,
    );

    game.player_mut(alice)
        .expect("alice exists")
        .mana_pool
        .add(ManaSymbol::Colorless, 4);

    fn finish_cycling_activation(
        game: &mut GameState,
        trigger_queue: &mut TriggerQueue,
        state: &mut PriorityLoopState,
        dm: &mut SelectFirstDecisionMaker,
        mut progress: crate::decision::GameProgress,
        label: &str,
    ) {
        for _ in 0..8 {
            progress = match progress {
                crate::decision::GameProgress::NeedsDecisionCtx(ctx) => {
                    apply_decision_context_with_dm(game, trigger_queue, state, &ctx, dm)
                        .unwrap_or_else(|err| {
                            panic!("{label} decision should resolve: {err}");
                        })
                }
                crate::decision::GameProgress::Continue
                | crate::decision::GameProgress::StackResolved
                | crate::decision::GameProgress::GameOver(_) => return,
            };
        }
        panic!("{label} did not finish producing stack or trigger work");
    }

    let activate_action = compute_legal_actions(&game, alice)
        .into_iter()
        .find(|action| {
            matches!(
                action,
                LegalAction::ActivateAbility { source, .. } if *source == cycling_id
            )
        })
        .expect("cycling should be available from hand");

    let progress = apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(activate_action),
        &mut dm,
    )
    .expect("cycling activation should succeed");

    finish_cycling_activation(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &mut dm,
        progress,
        "cycling activation",
    );

    assert!(
        !game.stack.is_empty() || !trigger_queue.entries.is_empty(),
        "cycling activation should leave pending stack or trigger work"
    );

    for _ in 0..4 {
        if !trigger_queue.entries.is_empty() {
            put_triggers_on_stack_with_dm(&mut game, &mut trigger_queue, &mut dm)
                .expect("queued cycling trigger should go on the stack");
        }
        if game.stack.is_empty() {
            break;
        }
        resolve_stack_entry_with_dm_and_triggers(&mut game, &mut dm, &mut trigger_queue)
            .expect("cycling stack work should resolve");
    }

    game.refresh_continuous_state();
    let resonance_chars = game
        .calculated_characteristics(resonance_id)
        .expect("Crystalline Resonance should still have characteristics");

    assert_eq!(
        resonance_chars.name,
        game.object(target_id)
            .expect("target bear should still exist")
            .name
            .clone(),
        "the enchantment should copy the target permanent's name"
    );
    assert!(
        resonance_chars.card_types.contains(&CardType::Creature)
            && !resonance_chars.card_types.contains(&CardType::Enchantment),
        "the enchantment should become the copied permanent type"
    );
    assert!(
        resonance_chars
            .abilities
            .iter()
            .any(|ability| matches!(&ability.kind, AbilityKind::Triggered(_))),
        "the enchantment should keep its cycling trigger while copied"
    );
    assert_eq!(
        resonance_chars.power,
        Some(3),
        "the copied permanent should contribute the target's power"
    );
    assert_eq!(
        resonance_chars.toughness,
        Some(3),
        "the copied permanent should contribute the target's toughness"
    );

    let second_activate_action = compute_legal_actions(&game, alice)
        .into_iter()
        .find(|action| {
            matches!(
                action,
                LegalAction::ActivateAbility { source, .. } if *source == cycling_id_two
            )
        })
        .expect("the second cycling card should still be available");

    let progress = apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(second_activate_action),
        &mut dm,
    )
    .expect("second cycling activation should succeed");

    finish_cycling_activation(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &mut dm,
        progress,
        "second cycling activation",
    );

    assert!(
        !game.stack.is_empty() || !trigger_queue.entries.is_empty(),
        "second cycling activation should also leave pending stack or trigger work"
    );
    for _ in 0..4 {
        if !trigger_queue.entries.is_empty() {
            put_triggers_on_stack_with_dm(&mut game, &mut trigger_queue, &mut dm)
                .expect("queued second cycling trigger should go on the stack");
        }
        if game.stack.is_empty() {
            break;
        }
        resolve_stack_entry_with_dm_and_triggers(&mut game, &mut dm, &mut trigger_queue)
            .expect("second cycling stack work should resolve");
    }

    game.turn.active_player = bob;
    game.turn.turn_number += 1;
    game.refresh_continuous_state();
    let still_copied_chars = game
        .calculated_characteristics(resonance_id)
        .expect("Crystalline Resonance should still have characteristics on the opponent's turn");

    assert!(
        still_copied_chars.card_types.contains(&CardType::Creature)
            && !still_copied_chars
                .card_types
                .contains(&CardType::Enchantment),
        "the copy should last until the start of the controller's next turn"
    );

    game.turn.active_player = alice;
    game.turn.turn_number += 1;
    game.refresh_continuous_state();
    let post_turn_chars = game.calculated_characteristics(resonance_id).expect(
        "Crystalline Resonance should still have characteristics after its next turn begins",
    );

    assert!(
        post_turn_chars.card_types.contains(&CardType::Enchantment)
            && !post_turn_chars.card_types.contains(&CardType::Creature),
        "the copy should expire on the controller's next turn"
    );
    assert_eq!(
        post_turn_chars.name, "Crystalline Resonance",
        "the enchantment should return to its original name once the copy expires"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_make_an_example_leaves_unselected_creatures_on_the_battlefield() {
    use crate::effects::ExecutionContext;

    struct ChooseFirstObjectDecisionMaker;

    impl DecisionMaker for ChooseFirstObjectDecisionMaker {
        fn decide_boolean(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::BooleanContext,
        ) -> bool {
            panic!("Make an Example should not use a boolean pile-choice prompt: {ctx:?}");
        }

        fn decide_objects(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            ctx.candidates
                .iter()
                .filter(|candidate| candidate.legal)
                .map(|candidate| candidate.id)
                .take(1)
                .collect()
        }

        fn decide_options(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            assert_eq!(ctx.description, "Choose a mode");
            assert!(
                ctx.options
                    .iter()
                    .any(|option| option.description == "Choose the separated pile"),
                "expected a named separated-pile choice, got {ctx:?}"
            );
            assert!(
                ctx.options.iter().any(|option| {
                    option.description == "Choose the separated pile"
                        && option
                            .related_object_ids
                            .as_ref()
                            .is_some_and(|object_ids| !object_ids.is_empty())
                }),
                "expected pile options to expose their related objects, got {ctx:?}"
            );
            vec![0]
        }
    }

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let make_an_example = CardDefinitionBuilder::new(CardId::new(), "Make an Example")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::Black],
        ]))
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Each opponent separates the creatures they control into two piles. For each opponent, you choose one of their piles. Each opponent sacrifices the creatures in their chosen pile. (Piles can be empty.)",
        )
        .expect("Make an Example should parse");

    let source_id = game.create_object_from_definition(&make_an_example, alice, Zone::Hand);
    let chosen_pile_creature = CardBuilder::new(CardId::new(), "Chosen Pile Bear")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let untouched_pile_creature = CardBuilder::new(CardId::new(), "Untouched Pile Bear")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 3))
        .build();
    let _chosen_pile_creature_id =
        game.create_object_from_card(&chosen_pile_creature, bob, Zone::Battlefield);
    let untouched_pile_creature_id =
        game.create_object_from_card(&untouched_pile_creature, bob, Zone::Battlefield);

    let spell_effects = make_an_example
        .spell_effect
        .as_ref()
        .expect("Make an Example should have spell effects");
    let mut dm = ChooseFirstObjectDecisionMaker;
    let mut ctx = ExecutionContext::new_default(source_id, alice).with_decision_maker(&mut dm);

    for effect in spell_effects {
        execute_effect(&mut game, effect, &mut ctx).expect("Make an Example effect should resolve");
    }

    assert!(
        game.player(bob)
            .expect("bob exists")
            .graveyard
            .iter()
            .any(|&id| {
                game.object(id)
                    .is_some_and(|obj| obj.name == "Chosen Pile Bear")
            }),
        "the chosen pile should be sacrificed"
    );
    assert!(
        game.battlefield.contains(&untouched_pile_creature_id),
        "the unchosen pile should remain on the battlefield"
    );
}

#[test]
pub(super) fn test_queue_triggers_tracks_noncombat_damage_to_players_this_turn() {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let bob = PlayerId::from_index(1);
    let source = ObjectId::from_raw(200);

    let event = TriggerEvent::new_with_provenance(
        DamageEvent::with_cause(
            source,
            EventDamageTarget::Player(bob),
            4,
            false,
            crate::events::cause::EventCause::effect(),
        ),
        crate::provenance::ProvNodeId::default(),
    );
    queue_triggers_from_event(&mut game, &mut trigger_queue, event, false);

    assert_eq!(game.turn_store.turn_history.total_damage_to_player(bob), 4);
    assert_eq!(
        game.turn_store
            .turn_history
            .total_noncombat_damage_to_players(&[bob]),
        4
    );
}

#[test]
pub(super) fn test_queue_triggers_tracks_life_gained_this_turn() {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let alice = PlayerId::from_index(0);

    let event = TriggerEvent::new_with_provenance(
        LifeGainEvent::new(alice, 5),
        crate::provenance::ProvNodeId::default(),
    );
    queue_triggers_from_event(&mut game, &mut trigger_queue, event, false);

    assert_eq!(
        game.turn_store
            .turn_history
            .total_life_gained_for_players(&[alice]),
        5
    );
}

#[test]
pub(super) fn test_queue_triggers_tracks_life_lost_this_turn() {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let bob = PlayerId::from_index(1);

    let event = TriggerEvent::new_with_provenance(
        LifeLossEvent::from_effect(bob, 3),
        crate::provenance::ProvNodeId::default(),
    );
    queue_triggers_from_event(&mut game, &mut trigger_queue, event, false);

    assert_eq!(
        game.turn_store
            .turn_history
            .total_life_lost_for_players(&[bob]),
        3
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn vengeful_warchief_triggers_only_on_the_first_life_loss_each_turn() {
    use crate::events::life::LifeLossEvent;
    use crate::object::CounterType;
    use crate::triggers::TriggerEvent;

    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let alice = PlayerId::from_index(0);

    let warchief = CardDefinitionBuilder::new(CardId::from_raw(81_700), "Vengeful Warchief")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(4)],
            vec![ManaSymbol::Black],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![crate::types::Subtype::Orc, crate::types::Subtype::Warrior])
        .power_toughness(PowerToughness::fixed(4, 4))
        .parse_text(
            "Whenever you lose life for the first time each turn, put a +1/+1 counter on this creature.",
        )
        .expect("Vengeful Warchief should parse");
    let warchief_id =
        game.create_object_from_definition(&warchief, alice, crate::zone::Zone::Battlefield);

    let first_loss = TriggerEvent::new_with_provenance(
        LifeLossEvent::from_effect(alice, 1),
        crate::provenance::ProvNodeId::default(),
    );
    queue_triggers_from_event(&mut game, &mut trigger_queue, first_loss, false);
    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("Vengeful Warchief should trigger on the first life loss");
    assert_eq!(
        game.stack.len(),
        1,
        "the first trigger should use the stack"
    );
    resolve_stack_entry(&mut game).expect("Vengeful Warchief trigger should resolve");
    assert_eq!(
        game.object(warchief_id)
            .expect("warchief should exist")
            .counters
            .get(&CounterType::PlusOnePlusOne)
            .copied()
            .unwrap_or(0),
        1,
        "the first life loss should place one +1/+1 counter"
    );

    let second_loss = TriggerEvent::new_with_provenance(
        LifeLossEvent::from_effect(alice, 1),
        crate::provenance::ProvNodeId::default(),
    );
    queue_triggers_from_event(&mut game, &mut trigger_queue, second_loss, false);
    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("second life loss should be processed cleanly");
    assert!(
        game.stack.is_empty(),
        "the trigger should not go on the stack a second time this turn"
    );
    assert_eq!(
        game.object(warchief_id)
            .expect("warchief should still exist")
            .counters
            .get(&CounterType::PlusOnePlusOne)
            .copied()
            .unwrap_or(0),
        1,
        "the second life loss should not add another counter"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn stonebinders_familiar_triggers_once_each_turn_and_only_during_your_turn() {
    use crate::events::zones::ZoneChangeEvent;
    use crate::object::CounterType;
    use crate::triggers::TriggerEvent;

    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let familiar = CardDefinitionBuilder::new(CardId::from_raw(81_710), "Stonebinder's Familiar")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::White]]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![crate::types::Subtype::Spirit, crate::types::Subtype::Dog])
        .power_toughness(PowerToughness::fixed(1, 1))
        .parse_text(
            "Whenever one or more cards are put into exile during your turn, put a +1/+1 counter on this creature. This ability triggers only once each turn.",
        )
        .expect("Stonebinder's Familiar should parse");
    let familiar_id =
        game.create_object_from_definition(&familiar, alice, crate::zone::Zone::Battlefield);

    let exiled_token_card = CardBuilder::new(CardId::from_raw(81_714), "Exiled Token")
        .card_types(vec![CardType::Creature])
        .build();
    let exiled_token_id =
        game.create_object_from_card(&exiled_token_card, alice, Zone::Battlefield);
    if let Some(token) = game.object_mut(exiled_token_id) {
        token.kind = ObjectKind::Token;
    }

    let exiled_card = CardBuilder::new(CardId::from_raw(81_711), "Exiled Card")
        .card_types(vec![CardType::Creature])
        .build();
    let exiled_id = game.create_object_from_card(&exiled_card, alice, Zone::Graveyard);

    game.turn.active_player = alice;
    let token_exile = TriggerEvent::new_with_provenance(
        ZoneChangeEvent::with_cause(
            exiled_token_id,
            Zone::Battlefield,
            Zone::Exile,
            crate::events::cause::EventCause::from_effect(familiar_id, alice),
            None,
        ),
        crate::provenance::ProvNodeId::default(),
    );
    queue_triggers_from_event(&mut game, &mut trigger_queue, token_exile, false);
    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("token exile event should be processed cleanly");
    assert!(
        game.stack.is_empty(),
        "Stonebinder's Familiar should not trigger when a token is exiled"
    );
    assert_eq!(
        game.counter_count(familiar_id, CounterType::PlusOnePlusOne),
        0,
        "token exile should not add a +1/+1 counter"
    );

    let first_exile = TriggerEvent::new_with_provenance(
        ZoneChangeEvent::with_cause(
            exiled_id,
            Zone::Graveyard,
            Zone::Exile,
            crate::events::cause::EventCause::from_effect(familiar_id, alice),
            None,
        ),
        crate::provenance::ProvNodeId::default(),
    );
    queue_triggers_from_event(&mut game, &mut trigger_queue, first_exile, false);
    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("Stonebinder's Familiar should trigger when cards are exiled during your turn");
    assert_eq!(game.stack.len(), 1, "first exile event should trigger once");
    resolve_stack_entry(&mut game).expect("Stonebinder's Familiar trigger should resolve");
    assert_eq!(
        game.counter_count(familiar_id, CounterType::PlusOnePlusOne),
        1,
        "first trigger should add one +1/+1 counter"
    );

    let second_exiled_card = CardBuilder::new(CardId::from_raw(81_712), "Second Exiled Card")
        .card_types(vec![CardType::Creature])
        .build();
    let second_exiled_id = game.create_object_from_card(&second_exiled_card, alice, Zone::Hand);
    let second_exile_same_turn = TriggerEvent::new_with_provenance(
        ZoneChangeEvent::with_cause(
            second_exiled_id,
            Zone::Hand,
            Zone::Exile,
            crate::events::cause::EventCause::from_effect(familiar_id, alice),
            None,
        ),
        crate::provenance::ProvNodeId::default(),
    );
    queue_triggers_from_event(&mut game, &mut trigger_queue, second_exile_same_turn, false);
    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("second same-turn exile should be processed cleanly");
    assert!(
        game.stack.is_empty(),
        "Stonebinder's Familiar should not trigger a second time in the same turn"
    );
    assert_eq!(
        game.counter_count(familiar_id, CounterType::PlusOnePlusOne),
        1,
        "second same-turn exile should not add another counter"
    );

    game.turn.active_player = bob;
    let opponent_exiled_card = CardBuilder::new(CardId::from_raw(81_713), "Opponent Exiled Card")
        .card_types(vec![CardType::Creature])
        .build();
    let opponent_exiled_id =
        game.create_object_from_card(&opponent_exiled_card, bob, Zone::Graveyard);
    let exile_on_opponents_turn = TriggerEvent::new_with_provenance(
        ZoneChangeEvent::with_cause(
            opponent_exiled_id,
            Zone::Graveyard,
            Zone::Exile,
            crate::events::cause::EventCause::from_effect(familiar_id, bob),
            None,
        ),
        crate::provenance::ProvNodeId::default(),
    );
    queue_triggers_from_event(
        &mut game,
        &mut trigger_queue,
        exile_on_opponents_turn,
        false,
    );
    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("opponent-turn exile event should be processed cleanly");
    assert!(
        game.stack.is_empty(),
        "Stonebinder's Familiar should not trigger during an opponent's turn"
    );
    assert_eq!(
        game.counter_count(familiar_id, CounterType::PlusOnePlusOne),
        1,
        "opponent-turn exile should not add a counter"
    );
}
