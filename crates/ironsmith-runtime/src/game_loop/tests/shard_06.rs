#![allow(unused_imports)]
use super::shard_00::*;
use super::shard_01::*;
use super::shard_02::*;
use super::shard_03::*;
use super::shard_04::*;
use super::shard_05::*;
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
use crate::ChoiceCount;
use crate::ability::{ActivatedAbility, ActivationTiming};

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn loxodon_smiter_discard_replacement_ignores_own_effects_and_costs() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let loxodon = loxodon_smiter_definition();

    for (source_controller, cause_kind) in [(alice, "own effect"), (bob, "opponent cost")] {
        let mut game = setup_game();
        let smiter = game.create_object_from_definition(&loxodon, alice, Zone::Hand);
        let discard_source = game.create_object_from_card(
            &CardBuilder::new(CardId::new(), "Discard Source")
                .card_types(vec![CardType::Sorcery])
                .build(),
            source_controller,
            Zone::Stack,
        );
        let cause = if cause_kind == "own effect" {
            crate::events::cause::EventCause::from_effect(discard_source, source_controller)
        } else {
            crate::events::cause::EventCause::from_cost(discard_source, source_controller)
        };
        let mut dm = SelectFirstDecisionMaker;

        let result = crate::events::processing::execute_discard(
            &mut game,
            smiter,
            alice,
            cause,
            false,
            crate::provenance::ProvNodeId::default(),
            &mut dm,
        );

        assert_eq!(
            result.final_zone,
            Zone::Graveyard,
            "Loxodon Smiter should go to the graveyard for {cause_kind}"
        );
        assert!(
            game.player(alice)
                .expect("Alice exists")
                .graveyard
                .iter()
                .any(|&id| game
                    .object(id)
                    .is_some_and(|object| object.name == "Loxodon Smiter")),
            "Loxodon Smiter should be in Alice's graveyard for {cause_kind}"
        );
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn loxodon_smiter_discard_replacement_only_applies_to_itself() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let loxodon = loxodon_smiter_definition();
    let smiter = game.create_object_from_definition(&loxodon, alice, Zone::Hand);
    let other_card = game.create_object_from_card(
        &CardBuilder::new(CardId::new(), "Other Discarded Card")
            .card_types(vec![CardType::Creature])
            .build(),
        alice,
        Zone::Hand,
    );
    let discard_source = game.create_object_from_card(
        &CardBuilder::new(CardId::new(), "Opponent Discard Spell")
            .card_types(vec![CardType::Sorcery])
            .build(),
        bob,
        Zone::Stack,
    );
    let mut dm = SelectFirstDecisionMaker;

    let result = crate::events::processing::execute_discard(
        &mut game,
        other_card,
        alice,
        crate::events::cause::EventCause::from_effect(discard_source, bob),
        false,
        crate::provenance::ProvNodeId::default(),
        &mut dm,
    );

    assert_eq!(result.final_zone, Zone::Graveyard);
    assert!(
        game.object(smiter)
            .is_some_and(|object| object.zone == Zone::Hand),
        "Loxodon Smiter's replacement should not apply to another discarded card"
    );
}

#[test]
pub(super) fn magma_mine_activated_ability_sacrifices_source_and_deals_counter_scaled_damage_to_player()
 {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let mine_def = CardDefinitionBuilder::new(CardId::from_raw(994_000), "Magma Mine")
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "{4}: Put a pressure counter on this artifact.\n{T}, Sacrifice this artifact: It deals damage equal to the number of pressure counters on it to any target.",
        )
        .expect("Magma Mine text should parse");
    let mine_id = game.create_object_from_definition(&mine_def, alice, Zone::Battlefield);
    game.add_counters(mine_id, crate::object::CounterType::Named("pressure"), 3)
        .expect("pressure counters should be addable to Magma Mine");

    {
        let player = game.player_mut(alice).expect("Alice should exist");
        player.mana_pool.add(ManaSymbol::Red, 4);
    }

    let ability_index = game
        .object(mine_id)
        .expect("Magma Mine should exist")
        .abilities
        .iter()
        .position(|ability| {
            if let AbilityKind::Activated(activated) = &ability.kind {
                let debug = format!("{:?}", activated.effects).to_ascii_lowercase();
                debug.contains("dealdamageeffect")
            } else {
                false
            }
        })
        .expect("Magma Mine should have a damage activated ability");

    let activate_action = compute_legal_actions(&game, alice)
        .into_iter()
        .find(|action| {
            matches!(
                action,
                LegalAction::ActivateAbility { source, ability_index: idx }
                    if *source == mine_id && *idx == ability_index
            )
        })
        .expect("Magma Mine activation should be legal with mana available");

    let mut trigger_queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = SelectFirstDecisionMaker;
    let progress = apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(activate_action),
        &mut dm,
    )
    .expect("Magma Mine activation should start");

    match progress {
        crate::decision::GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::Targets(_),
        ) => {}
        other => panic!("expected target selection for Magma Mine activation, got {other:?}"),
    }

    apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::Targets(vec![Target::Player(bob)]),
        &mut dm,
    )
    .expect("choosing player target should complete activation");

    assert!(
        game.object(mine_id).is_none(),
        "Magma Mine should be sacrificed as part of activation cost"
    );

    resolve_stack_entry(&mut game).expect("Magma Mine ability should resolve");

    assert_eq!(
        game.player(bob).expect("Bob should exist").life,
        17,
        "Magma Mine should deal damage equal to pressure counters on source"
    );
}

#[test]
pub(super) fn magma_mine_activated_ability_uses_current_pressure_counter_count_when_zero() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let mine_def = CardDefinitionBuilder::new(CardId::from_raw(994_010), "Magma Mine")
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "{4}: Put a pressure counter on this artifact.\n{T}, Sacrifice this artifact: It deals damage equal to the number of pressure counters on it to any target.",
        )
        .expect("Magma Mine text should parse");
    let mine_id = game.create_object_from_definition(&mine_def, alice, Zone::Battlefield);

    let target_creature = CardBuilder::new(CardId::from_raw(994_001), "Target Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let target_id = game.create_object_from_card(&target_creature, bob, Zone::Battlefield);

    {
        let player = game.player_mut(alice).expect("Alice should exist");
        player.mana_pool.add(ManaSymbol::Red, 4);
    }

    let ability_index = game
        .object(mine_id)
        .expect("Magma Mine should exist")
        .abilities
        .iter()
        .position(|ability| {
            if let AbilityKind::Activated(activated) = &ability.kind {
                let debug = format!("{:?}", activated.effects).to_ascii_lowercase();
                debug.contains("dealdamageeffect")
            } else {
                false
            }
        })
        .expect("Magma Mine should have a damage activated ability");

    let activate_action = compute_legal_actions(&game, alice)
        .into_iter()
        .find(|action| {
            matches!(
                action,
                LegalAction::ActivateAbility { source, ability_index: idx }
                    if *source == mine_id && *idx == ability_index
            )
        })
        .expect("Magma Mine activation should be legal with mana available");

    let mut trigger_queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = SelectFirstDecisionMaker;
    let progress = apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(activate_action),
        &mut dm,
    )
    .expect("Magma Mine activation should start");

    match progress {
        crate::decision::GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::Targets(_),
        ) => {}
        other => panic!("expected target selection for Magma Mine activation, got {other:?}"),
    }

    apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::Targets(vec![Target::Object(target_id)]),
        &mut dm,
    )
    .expect("choosing creature target should complete activation");

    resolve_stack_entry(&mut game).expect("Magma Mine ability should resolve");

    assert_eq!(
        game.damage_on(target_id),
        0,
        "without pressure counters, Magma Mine should deal zero damage"
    );
}

pub(super) fn sage_of_hours_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(994_200), "Sage of Hours")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::Blue],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Human, Subtype::Wizard])
        .power_toughness(PowerToughness::fixed(1, 1))
        .parse_text(
            "Heroic — Whenever you cast a spell that targets this creature, put a +1/+1 counter on it.\n\
             Remove all +1/+1 counters from this creature: For each five counters removed this way, take an extra turn after this one.",
        )
        .expect("Sage of Hours should parse for runtime tests")
}

pub(super) fn sage_of_hours_extra_turn_ability_index(game: &GameState, sage_id: ObjectId) -> usize {
    game.object(sage_id)
        .expect("Sage of Hours should exist")
        .abilities
        .iter()
        .position(|ability| {
            if let AbilityKind::Activated(activated) = &ability.kind {
                format!("{:?}", activated.effects).contains("ExtraTurnEffect")
            } else {
                false
            }
        })
        .expect("Sage of Hours should have an extra-turn activated ability")
}

pub(super) fn activate_sage_of_hours_extra_turn_ability(counter_count: u32) -> GameState {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let sage_def = sage_of_hours_definition();
    let sage_id = game.create_object_from_definition(&sage_def, alice, Zone::Battlefield);
    game.add_counters(
        sage_id,
        crate::object::CounterType::PlusOnePlusOne,
        counter_count,
    )
    .expect("+1/+1 counters should be addable to Sage of Hours");

    let ability_index = sage_of_hours_extra_turn_ability_index(&game, sage_id);
    let activate_action = compute_legal_actions(&game, alice)
        .into_iter()
        .find(|action| {
            matches!(
                action,
                LegalAction::ActivateAbility { source, ability_index: idx }
                    if *source == sage_id && *idx == ability_index
            )
        })
        .expect("Sage of Hours activation should be legal");

    let mut trigger_queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = SelectFirstDecisionMaker;
    apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(activate_action),
        &mut dm,
    )
    .expect("Sage of Hours activation should be put on the stack");

    assert_eq!(
        game.counter_count(sage_id, crate::object::CounterType::PlusOnePlusOne),
        0,
        "Sage of Hours activation cost should remove all +1/+1 counters"
    );
    resolve_stack_entry(&mut game).expect("Sage of Hours activation should resolve");
    game
}

#[test]
pub(super) fn sage_of_hours_activation_takes_one_extra_turn_per_five_removed_counters() {
    let game = activate_sage_of_hours_extra_turn_ability(10);
    let alice = PlayerId::from_index(0);

    assert_eq!(
        game.turn_store.extra_turns,
        vec![alice, alice],
        "ten removed counters should schedule two extra turns for Sage of Hours controller"
    );
}

#[test]
pub(super) fn sage_of_hours_activation_below_five_removed_counters_takes_no_extra_turns() {
    let game = activate_sage_of_hours_extra_turn_ability(4);

    assert!(
        game.turn_store.extra_turns.is_empty(),
        "fewer than five removed counters should not schedule an extra turn"
    );
}

pub(super) fn queue_spell_cast_targeting(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    caster: PlayerId,
    target: ObjectId,
) {
    let spell = CardBuilder::new(CardId::from_raw(994_201), "Sage Targeting Probe")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Blue]]))
        .card_types(vec![CardType::Instant])
        .build();
    let spell_id = game.create_object_from_card(&spell, caster, Zone::Stack);
    game.push_to_stack(
        StackEntry::new(spell_id, caster).with_targets(vec![Target::Object(target)]),
    );
    let event = TriggerEvent::new_with_provenance(
        SpellCastEvent::new(spell_id, caster, Zone::Hand),
        crate::provenance::ProvNodeId::default(),
    );
    queue_triggers_from_event(game, trigger_queue, event, false);
}

#[test]
pub(super) fn sage_of_hours_heroic_adds_counter_when_your_spell_targets_it() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let sage_def = sage_of_hours_definition();
    let sage_id = game.create_object_from_definition(&sage_def, alice, Zone::Battlefield);
    let mut trigger_queue = TriggerQueue::new();

    queue_spell_cast_targeting(&mut game, &mut trigger_queue, alice, sage_id);
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "Sage of Hours should trigger when Alice casts a spell targeting it"
    );
    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("Sage of Hours heroic trigger should go on the stack");
    resolve_stack_entry(&mut game).expect("Sage of Hours heroic trigger should resolve");

    assert_eq!(
        game.counter_count(sage_id, crate::object::CounterType::PlusOnePlusOne),
        1,
        "Sage of Hours heroic trigger should add a +1/+1 counter"
    );
}

#[test]
pub(super) fn sage_of_hours_heroic_does_not_trigger_for_spell_targeting_another_creature() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let sage_def = sage_of_hours_definition();
    game.create_object_from_definition(&sage_def, alice, Zone::Battlefield);
    let other_id = create_creature(&mut game, "Other Target", alice, 1, 1);
    let mut trigger_queue = TriggerQueue::new();

    queue_spell_cast_targeting(&mut game, &mut trigger_queue, alice, other_id);

    assert!(
        trigger_queue.entries.is_empty(),
        "Sage of Hours should not trigger when the spell targets a different creature"
    );
}

#[test]
pub(super) fn molten_hydra_activated_damage_uses_number_of_removed_plus1_counters() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let hydra_def = CardDefinitionBuilder::new(CardId::from_raw(994_100), "Molten Hydra")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .parse_text(
            "{1}{R}{R}: Put a +1/+1 counter on this creature.\n{T}, Remove all +1/+1 counters from this creature: It deals damage to any target equal to the number of +1/+1 counters removed this way.",
        )
        .expect("Molten Hydra oracle text should parse");
    let hydra_id = game.create_object_from_definition(&hydra_def, alice, Zone::Battlefield);
    game.add_counters(hydra_id, crate::object::CounterType::PlusOnePlusOne, 3)
        .expect("+1/+1 counters should be addable to Molten Hydra");

    let ability_index = game
        .object(hydra_id)
        .expect("Molten Hydra should exist")
        .abilities
        .iter()
        .position(|ability| {
            if let AbilityKind::Activated(activated) = &ability.kind {
                let debug = format!("{:?}", activated.effects).to_ascii_lowercase();
                debug.contains("dealdamageeffect")
            } else {
                false
            }
        })
        .expect("Molten Hydra should have a damage activated ability");

    let activate_action = compute_legal_actions(&game, alice)
        .into_iter()
        .find(|action| {
            matches!(
                action,
                LegalAction::ActivateAbility { source, ability_index: idx }
                    if *source == hydra_id && *idx == ability_index
            )
        })
        .expect("Molten Hydra activation should be legal");

    let mut trigger_queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = SelectFirstDecisionMaker;
    let progress = apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(activate_action),
        &mut dm,
    )
    .expect("Molten Hydra activation should start");

    match progress {
        crate::decision::GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::Targets(_),
        ) => {}
        other => panic!("expected target selection for Molten Hydra activation, got {other:?}"),
    }

    apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::Targets(vec![Target::Player(bob)]),
        &mut dm,
    )
    .expect("choosing target player should complete Molten Hydra activation");

    assert_eq!(
        game.counter_count(hydra_id, crate::object::CounterType::PlusOnePlusOne),
        0,
        "Molten Hydra activation cost should remove all +1/+1 counters"
    );

    resolve_stack_entry(&mut game).expect("Molten Hydra ability should resolve");

    assert_eq!(
        game.player(bob).expect("Bob should exist").life,
        17,
        "Molten Hydra should deal damage equal to removed +1/+1 counters"
    );
}

#[test]
pub(super) fn molten_hydra_activated_damage_can_target_creatures() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let hydra_def = CardDefinitionBuilder::new(CardId::from_raw(994_101), "Molten Hydra")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .parse_text(
            "{1}{R}{R}: Put a +1/+1 counter on this creature.\n{T}, Remove all +1/+1 counters from this creature: It deals damage to any target equal to the number of +1/+1 counters removed this way.",
        )
        .expect("Molten Hydra oracle text should parse");
    let hydra_id = game.create_object_from_definition(&hydra_def, alice, Zone::Battlefield);

    let target_creature = CardBuilder::new(CardId::from_raw(994_102), "Target Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let target_id = game.create_object_from_card(&target_creature, bob, Zone::Battlefield);
    game.add_counters(hydra_id, crate::object::CounterType::PlusOnePlusOne, 1)
        .expect("+1/+1 counters should be addable to Molten Hydra");

    let ability_index = game
        .object(hydra_id)
        .expect("Molten Hydra should exist")
        .abilities
        .iter()
        .position(|ability| {
            if let AbilityKind::Activated(activated) = &ability.kind {
                let debug = format!("{:?}", activated.effects).to_ascii_lowercase();
                debug.contains("dealdamageeffect")
            } else {
                false
            }
        })
        .expect("Molten Hydra should have a damage activated ability");

    let activate_action = compute_legal_actions(&game, alice)
        .into_iter()
        .find(|action| {
            matches!(
                action,
                LegalAction::ActivateAbility { source, ability_index: idx }
                    if *source == hydra_id && *idx == ability_index
            )
        })
        .expect("Molten Hydra activation should be legal");

    let mut trigger_queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = SelectFirstDecisionMaker;
    let progress = apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(activate_action),
        &mut dm,
    )
    .expect("Molten Hydra activation should start");

    match progress {
        crate::decision::GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::Targets(_),
        ) => {}
        other => panic!("expected target selection for Molten Hydra activation, got {other:?}"),
    }

    apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::Targets(vec![Target::Object(target_id)]),
        &mut dm,
    )
    .expect("choosing target creature should complete Molten Hydra activation");

    resolve_stack_entry(&mut game).expect("Molten Hydra ability should resolve");

    assert_eq!(
        game.damage_on(target_id),
        1,
        "Molten Hydra should deal removed-counter damage to target creatures"
    );
}

#[test]
pub(super) fn molten_hydra_activated_damage_is_zero_when_no_counters_are_removed() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let hydra_def = CardDefinitionBuilder::new(CardId::from_raw(994_103), "Molten Hydra")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .parse_text(
            "{1}{R}{R}: Put a +1/+1 counter on this creature.\n{T}, Remove all +1/+1 counters from this creature: It deals damage to any target equal to the number of +1/+1 counters removed this way.",
        )
        .expect("Molten Hydra oracle text should parse");
    let hydra_id = game.create_object_from_definition(&hydra_def, alice, Zone::Battlefield);

    let ability_index = game
        .object(hydra_id)
        .expect("Molten Hydra should exist")
        .abilities
        .iter()
        .position(|ability| {
            if let AbilityKind::Activated(activated) = &ability.kind {
                let debug = format!("{:?}", activated.effects).to_ascii_lowercase();
                debug.contains("dealdamageeffect")
            } else {
                false
            }
        })
        .expect("Molten Hydra should have a damage activated ability");

    let activate_action = compute_legal_actions(&game, alice)
        .into_iter()
        .find(|action| {
            matches!(
                action,
                LegalAction::ActivateAbility { source, ability_index: idx }
                    if *source == hydra_id && *idx == ability_index
            )
        })
        .expect("Molten Hydra activation should be legal");

    let mut trigger_queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = SelectFirstDecisionMaker;
    let progress = apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(activate_action),
        &mut dm,
    )
    .expect("Molten Hydra activation should start");

    match progress {
        crate::decision::GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::Targets(_),
        ) => {}
        other => panic!("expected target selection for Molten Hydra activation, got {other:?}"),
    }

    apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::Targets(vec![Target::Player(bob)]),
        &mut dm,
    )
    .expect("choosing target player should complete Molten Hydra activation");

    resolve_stack_entry(&mut game).expect("Molten Hydra ability should resolve");

    assert_eq!(
        game.player(bob).expect("Bob should exist").life,
        20,
        "Molten Hydra should deal zero damage when zero counters are removed"
    );
}

#[test]
pub(super) fn protected_stack_spell_still_resolves_after_failed_counterspell() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bolt = CardDefinitionBuilder::new(CardId::new(), "Lightning Probe")
        .card_types(vec![CardType::Instant])
        .parse_text("Lightning Probe deals 3 damage to any target.")
        .expect("damage spell should parse");
    let counterspell = CardDefinitionBuilder::new(CardId::new(), "Counter Probe")
        .card_types(vec![CardType::Instant])
        .parse_text("Counter target spell.")
        .expect("counter spell should parse");
    let goblin_card = CardBuilder::new(CardId::new(), "Raging Goblin")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();

    let goblin = game.create_object_from_card(&goblin_card, alice, Zone::Battlefield);
    let bolt_id = game.create_object_from_definition(&bolt, alice, Zone::Stack);
    game.object_mut(bolt_id)
        .expect("spell should be on the stack")
        .abilities_mut()
        .push(Ability::static_ability(
            StaticAbility::cant_be_countered_ability(),
        ));
    game.push_to_stack(
        StackEntry::new(bolt_id, alice)
            .with_targets(vec![Target::Object(goblin)])
            .with_target_assignments(vec![crate::game_state::TargetAssignment {
                spec: crate::target::ChooseSpec::AnyTarget,
                range: 0..1,
            }]),
    );

    let counter_id = game.create_object_from_definition(&counterspell, alice, Zone::Stack);
    game.push_to_stack(
        StackEntry::new(counter_id, alice)
            .with_targets(vec![Target::Object(bolt_id)])
            .with_target_assignments(vec![crate::game_state::TargetAssignment {
                spec: crate::target::ChooseSpec::spell(),
                range: 0..1,
            }]),
    );

    resolve_stack_entry(&mut game).expect("counterspell should resolve");
    assert!(
        game.stack.iter().any(|entry| entry.object_id == bolt_id),
        "failed counterspell should leave the protected spell on the stack"
    );

    resolve_stack_entry(&mut game).expect("protected spell should resolve");

    assert_eq!(
        game.damage_on(goblin),
        3,
        "protected spell should still execute after a failed counter"
    );
}

#[test]
pub(super) fn priority_loop_resolves_failed_counter_then_protected_spell_effect() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let mut trigger_queue = TriggerQueue::new();
    let bolt = CardDefinitionBuilder::new(CardId::new(), "Lightning Probe")
        .card_types(vec![CardType::Instant])
        .parse_text("Lightning Probe deals 3 damage to any target.")
        .expect("damage spell should parse");
    let counterspell = CardDefinitionBuilder::new(CardId::new(), "Counter Probe")
        .card_types(vec![CardType::Instant])
        .parse_text("Counter target spell.")
        .expect("counter spell should parse");
    let goblin_card = CardBuilder::new(CardId::new(), "Raging Goblin")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();

    let goblin = game.create_object_from_card(&goblin_card, alice, Zone::Battlefield);
    let bolt_id = game.create_object_from_definition(&bolt, alice, Zone::Stack);
    game.object_mut(bolt_id)
        .expect("spell should be on the stack")
        .abilities_mut()
        .push(Ability::static_ability(
            StaticAbility::cant_be_countered_ability(),
        ));
    game.push_to_stack(
        StackEntry::new(bolt_id, alice)
            .with_targets(vec![Target::Object(goblin)])
            .with_target_assignments(vec![crate::game_state::TargetAssignment {
                spec: crate::target::ChooseSpec::AnyTarget,
                range: 0..1,
            }]),
    );

    let counter_id = game.create_object_from_definition(&counterspell, alice, Zone::Stack);
    game.push_to_stack(
        StackEntry::new(counter_id, alice)
            .with_targets(vec![Target::Object(bolt_id)])
            .with_target_assignments(vec![crate::game_state::TargetAssignment {
                spec: crate::target::ChooseSpec::spell(),
                range: 0..1,
            }]),
    );
    game.turn.priority_player = Some(alice);

    let mut dm = AutoPassDecisionMaker;
    run_priority_loop_with(&mut game, &mut trigger_queue, &mut dm)
        .expect("priority loop should resolve the stack");

    assert!(
        game.player(alice)
            .expect("alice")
            .graveyard
            .iter()
            .any(|id| game
                .object(*id)
                .is_some_and(|object| object.name == "Raging Goblin")),
        "priority loop should apply lethal damage SBAs after the protected spell resolves"
    );
}

#[test]
pub(super) fn priority_loop_resolves_next_spell_granted_protection_after_counterspell() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let mut trigger_queue = TriggerQueue::new();
    let bolt = CardDefinitionBuilder::new(CardId::new(), "Lightning Probe")
        .card_types(vec![CardType::Instant])
        .parse_text("Lightning Probe deals 3 damage to any target.")
        .expect("damage spell should parse");
    let counterspell = CardDefinitionBuilder::new(CardId::new(), "Counter Probe")
        .card_types(vec![CardType::Instant])
        .parse_text("Counter target spell.")
        .expect("counter spell should parse");
    let goblin_card = CardBuilder::new(CardId::new(), "Raging Goblin")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();

    let goblin = game.create_object_from_card(&goblin_card, alice, Zone::Battlefield);
    let bolt_id = game.create_object_from_definition(&bolt, alice, Zone::Stack);
    game.add_temporary_spell_ability_grant(
        alice,
        bolt_id,
        crate::target::ObjectFilter::instant_or_sorcery().cast_by(crate::PlayerFilter::You),
        StaticAbility::cant_be_countered_ability().into(),
        1,
    );
    game.apply_temporary_spell_ability_grants_for_cast_proposal(bolt_id, alice);
    game.push_to_stack(
        StackEntry::new(bolt_id, alice)
            .with_targets(vec![Target::Object(goblin)])
            .with_target_assignments(vec![crate::game_state::TargetAssignment {
                spec: crate::target::ChooseSpec::AnyTarget,
                range: 0..1,
            }]),
    );

    let counter_id = game.create_object_from_definition(&counterspell, alice, Zone::Stack);
    game.push_to_stack(
        StackEntry::new(counter_id, alice)
            .with_targets(vec![Target::Object(bolt_id)])
            .with_target_assignments(vec![crate::game_state::TargetAssignment {
                spec: crate::target::ChooseSpec::spell(),
                range: 0..1,
            }]),
    );
    game.turn.priority_player = Some(alice);

    let mut dm = AutoPassDecisionMaker;
    run_priority_loop_with(&mut game, &mut trigger_queue, &mut dm)
        .expect("priority loop should resolve the stack");

    assert!(
        game.player(alice)
            .expect("alice")
            .graveyard
            .iter()
            .any(|id| game
                .object(*id)
                .is_some_and(|object| object.name == "Raging Goblin")),
        "temporary next-spell grant should protect the spell without suppressing its damage"
    );
}

#[test]
pub(super) fn test_active_target_assignments_modal_target_slot_tracks_chosen_mode_without_rechecking_legality()
 {
    let effects = vec![Effect::choose_one(vec![
        crate::effect::EffectMode {
            source_text: "Counter target spell".to_string(),
            effects: vec![Effect::counter(ChooseSpec::spell())],
        },
        crate::effect::EffectMode {
            source_text: "Gain 3 life".to_string(),
            effects: vec![Effect::gain_life(3)],
        },
    ])];
    let assignment = crate::game_state::TargetAssignment {
        spec: ChooseSpec::spell(),
        range: 0..1,
    };
    let counter_mode = [0usize];
    let gain_mode = [1usize];

    let mut consumed_modal_selection = false;
    let mut declared_targets = Vec::new();
    let mut cursor = 0usize;
    let active = super::stack_resolution::active_target_assignments_for_effect(
        &effects[0],
        Some(&counter_mode),
        &mut consumed_modal_selection,
        &mut declared_targets,
        std::slice::from_ref(&assignment),
        &mut cursor,
    );
    assert_eq!(
        active,
        vec![assignment.clone()],
        "the chosen targeted mode should keep its stored target assignment at resolution"
    );
    assert_eq!(
        cursor, 1,
        "chosen targeted mode should consume one stored assignment"
    );

    let mut consumed_modal_selection = false;
    let mut declared_targets = Vec::new();
    let mut cursor = 0usize;
    let inactive = super::stack_resolution::active_target_assignments_for_effect(
        &effects[0],
        Some(&gain_mode),
        &mut consumed_modal_selection,
        &mut declared_targets,
        std::slice::from_ref(&assignment),
        &mut cursor,
    );
    assert!(
        inactive.is_empty(),
        "non-targeting chosen mode should not consume unrelated stored assignments"
    );
    assert_eq!(
        cursor, 0,
        "non-targeting mode should leave the assignment cursor untouched"
    );
}

#[test]
pub(super) fn test_casting_choose_two_spell_keeps_non_targeting_modes_clickable_in_modes_context() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let modal_def = CardDefinitionBuilder::new(CardId::new(), "Choose Two Probe")
        .mana_cost(ManaCost::new())
        .card_types(vec![CardType::Instant])
        .with_spell_effect(vec![Effect::choose_exactly(
            2,
            vec![
                crate::effect::EffectMode {
                    source_text: "Gain 3 life".to_string(),
                    effects: vec![Effect::gain_life(3)],
                },
                crate::effect::EffectMode {
                    source_text: "Draw a card".to_string(),
                    effects: vec![Effect::draw(1)],
                },
                crate::effect::EffectMode {
                    source_text: "Gain 4 life".to_string(),
                    effects: vec![Effect::gain_life(4)],
                },
            ],
        )])
        .build();
    let modal_spell = game.create_object_from_definition(&modal_def, alice, Zone::Hand);

    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut trigger_queue = TriggerQueue::new();
    let progress = apply_priority_response(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(LegalAction::CastSpell {
            spell_id: modal_spell,
            from_zone: Zone::Hand,
            casting_method: CastingMethod::Normal,
        }),
    )
    .expect("casting the choose-two spell should reach the mode chooser");

    let ctx = match progress {
        GameProgress::NeedsDecisionCtx(crate::decisions::context::DecisionContext::Modes(ctx)) => {
            ctx
        }
        other => panic!("expected modal choice decision, got {other:?}"),
    };

    assert_eq!(ctx.spec.min_modes, 2);
    assert_eq!(ctx.spec.max_modes, 2);
    assert_eq!(ctx.spec.modes.len(), 3);
    assert!(
        ctx.spec.modes.iter().all(|mode| mode.legal),
        "individually legal non-targeting modes should remain clickable in the choose-two prompt: {:?}",
        ctx.spec
            .modes
            .iter()
            .map(|mode| (mode.index, mode.description.as_str(), mode.legal))
            .collect::<Vec<_>>()
    );
}

#[test]
pub(super) fn modal_x_spell_announces_modes_before_x() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let modal_x_def = CardDefinitionBuilder::new(CardId::new(), "Modal X Order Probe")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::X]]))
        .card_types(vec![CardType::Sorcery])
        .with_spell_effect(vec![Effect::choose_exactly(
            1,
            vec![
                crate::effect::EffectMode {
                    source_text: "Gain 3 life".to_string(),
                    effects: vec![Effect::gain_life(3)],
                },
                crate::effect::EffectMode {
                    source_text: "Draw a card".to_string(),
                    effects: vec![Effect::draw(1)],
                },
            ],
        )])
        .build();
    let modal_x_spell = game.create_object_from_definition(&modal_x_def, alice, Zone::Hand);

    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut trigger_queue = TriggerQueue::new();
    let progress = apply_priority_response(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(LegalAction::CastSpell {
            spell_id: modal_x_spell,
            from_zone: Zone::Hand,
            casting_method: CastingMethod::Normal,
        }),
    )
    .expect("proposing a modal X spell should request its first announcement");

    assert!(
        matches!(
            progress,
            GameProgress::NeedsDecisionCtx(crate::decisions::context::DecisionContext::Modes(_))
        ),
        "CR 601.2b requires modes before the value of X, got {progress:?}"
    );

    let progress = apply_priority_response(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::Modes(vec![0]),
    )
    .expect("choosing the mode should advance to the X announcement");

    assert!(
        matches!(
            progress,
            GameProgress::NeedsDecisionCtx(crate::decisions::context::DecisionContext::Number(_))
        ),
        "the value of X should be announced immediately after modes, got {progress:?}"
    );
}

#[test]
pub(super) fn modal_x_spell_announces_optional_costs_before_x() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let modal_x_def = CardDefinitionBuilder::new(CardId::new(), "Modal Optional X Order Probe")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::X]]))
        .card_types(vec![CardType::Sorcery])
        .with_spell_effect(vec![Effect::choose_exactly(
            1,
            vec![crate::effect::EffectMode {
                source_text: "Gain 3 life".to_string(),
                effects: vec![Effect::gain_life(3)],
            }],
        )])
        .build();
    let modal_x_spell = game.create_object_from_definition(&modal_x_def, alice, Zone::Hand);
    let spell = game
        .object_mut(modal_x_spell)
        .expect("modal X spell should exist");
    spell.optional_costs = vec![crate::cost::OptionalCost::custom(
        "Order probe additional cost",
        crate::cost::TotalCost::free(),
    )]
    .into();
    spell.optional_costs_paid = crate::cost::OptionalCostsPaid::from_costs(&spell.optional_costs);

    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut trigger_queue = TriggerQueue::new();
    let progress = apply_priority_response(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(LegalAction::CastSpell {
            spell_id: modal_x_spell,
            from_zone: Zone::Hand,
            casting_method: CastingMethod::Normal,
        }),
    )
    .expect("proposing the spell should request its mode");
    assert!(
        matches!(
            progress,
            GameProgress::NeedsDecisionCtx(crate::decisions::context::DecisionContext::Modes(_))
        ),
        "modes should be announced first, got {progress:?}"
    );

    let progress = apply_priority_response(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::Modes(vec![0]),
    )
    .expect("choosing the mode should advance to optional costs");
    assert!(
        matches!(
            progress,
            GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::SelectOptions(_)
            )
        ),
        "alternative and additional cost choices must precede X, got {progress:?}"
    );

    let progress = apply_priority_response(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::OptionalCosts(vec![]),
    )
    .expect("declining the optional cost should advance to X");
    assert!(
        matches!(
            progress,
            GameProgress::NeedsDecisionCtx(crate::decisions::context::DecisionContext::Number(_))
        ),
        "X should follow the optional-cost announcement, got {progress:?}"
    );
}

#[test]
pub(super) fn effect_driven_cast_uses_full_cr601_transaction_and_rolls_back_atomically() {
    struct OrderedEffectCastDecisionMaker {
        target: Option<ObjectId>,
        stages: Vec<&'static str>,
    }

    impl crate::decision::DecisionMaker for OrderedEffectCastDecisionMaker {
        fn decide_number(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::NumberContext,
        ) -> u32 {
            panic!("a free cast must lock printed X to zero without prompting")
        }

        fn decide_options(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            if ctx.description.starts_with("Choose mode") {
                self.stages.push("modes");
            } else if ctx.description.starts_with("Choose optional costs") {
                self.stages.push("optional costs");
            } else if ctx.description.starts_with("Pay mana") {
                self.stages.push("mana payment");
            }
            ctx.options
                .iter()
                .find(|option| option.legal)
                .map(|option| vec![option.index])
                .unwrap_or_default()
        }

        fn decide_targets(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::TargetsContext,
        ) -> Vec<Target> {
            self.stages.push("targets");
            self.target.map(Target::Object).into_iter().collect()
        }
    }

    fn setup_effect_cast_probe() -> (GameState, PlayerId, ObjectId, ObjectId) {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        game.turn.phase = Phase::Combat;
        game.turn.step = Some(Step::BeginCombat);
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);

        let target_definition = CardDefinitionBuilder::new(CardId::new(), "Effect Cast Target")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .with_ability(Ability::triggered(
                Trigger::becomes_targeted(),
                vec![Effect::gain_life(1)],
            ))
            .build();
        let target = game.create_object_from_definition(&target_definition, bob, Zone::Battlefield);
        let spell = CardDefinitionBuilder::new(CardId::new(), "Effect Cast CR 601 Probe")
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(7)],
                vec![ManaSymbol::X],
            ]))
            .card_types(vec![CardType::Sorcery])
            .with_spell_effect(vec![Effect::choose_exactly(
                1,
                vec![
                    crate::effect::EffectMode {
                        source_text: "Deal 2 damage to any target".to_string(),
                        effects: vec![Effect::deal_damage(2, ChooseSpec::AnyTarget)],
                    },
                    crate::effect::EffectMode {
                        source_text: "Gain 2 life".to_string(),
                        effects: vec![Effect::gain_life(2)],
                    },
                ],
            )])
            .build();
        let spell_id = game.create_object_from_definition(&spell, alice, Zone::Hand);
        let spell = game.object_mut(spell_id).expect("probe spell exists");
        spell.optional_costs = vec![crate::cost::OptionalCost::custom(
            "Effect-driven kicker",
            crate::cost::TotalCost::mana(ManaCost::from_symbols(vec![ManaSymbol::Red])),
        )]
        .into();
        spell.optional_costs_paid =
            crate::cost::OptionalCostsPaid::from_costs(&spell.optional_costs);
        game.player_mut(alice)
            .expect("Alice exists")
            .mana_pool
            .add(ManaSymbol::Red, 1);
        (game, alice, target, spell_id)
    }

    let (mut game, alice, target, spell_id) = setup_effect_cast_probe();
    let mut dm = OrderedEffectCastDecisionMaker {
        target: Some(target),
        stages: Vec::new(),
    };
    let result = super::cast_spell_from_resolving_effect(
        &mut game,
        spell_id,
        Zone::Hand,
        alice,
        &CastingMethod::Normal,
        true,
        None,
        crate::provenance::ProvNodeId::default(),
        &mut dm,
    )
    .expect("the resolving effect should authorize the complete cast");
    let stack_id = result.expect("the complete proposal should commit");
    assert_eq!(dm.stages, vec!["modes", "optional costs", "targets"]);
    let entry = game
        .stack
        .iter()
        .find(|entry| entry.object_id == stack_id)
        .expect("the effect-driven spell should be on the stack");
    assert_eq!(entry.chosen_modes.as_deref(), Some(&[0][..]));
    assert_eq!(entry.targets, vec![Target::Object(target)]);
    assert_eq!(entry.x_value, Some(0));
    assert_eq!(entry.optional_costs_paid.times_paid(0), 1);
    assert_eq!(
        game.player(alice).expect("Alice exists").mana_pool.total(),
        0
    );
    assert_eq!(
        game.effect_store.pending_trigger_entries.len(),
        1,
        "becomes-targeted triggers created during an effect-driven cast must wait for the outer resolution boundary",
    );
    let mut outer_trigger_queue = TriggerQueue::new();
    drain_pending_trigger_events(&mut game, &mut outer_trigger_queue);
    assert_eq!(outer_trigger_queue.entries.len(), 1);

    let (mut game, alice, _target, spell_id) = setup_effect_cast_probe();
    let mut invalid_dm = OrderedEffectCastDecisionMaker {
        target: None,
        stages: Vec::new(),
    };
    let result = super::cast_spell_from_resolving_effect(
        &mut game,
        spell_id,
        Zone::Hand,
        alice,
        &CastingMethod::Normal,
        true,
        None,
        crate::provenance::ProvNodeId::default(),
        &mut invalid_dm,
    )
    .expect("an invalid target choice should cancel rather than corrupt state");
    assert!(result.is_none());
    assert!(game.stack_is_empty());
    assert!(
        game.object(spell_id)
            .is_some_and(|spell| spell.zone == Zone::Hand)
    );
    assert_eq!(
        game.player(alice).expect("Alice exists").mana_pool.total(),
        1
    );
}

#[test]
pub(super) fn splice_transaction_adds_ordered_text_costs_targets_and_retains_cards() {
    struct SpliceDecisionMaker {
        selection: Vec<ObjectId>,
        target: PlayerId,
        splice_prompts: usize,
        target_prompts: usize,
        public_reveals: Vec<(PlayerId, Vec<ObjectId>)>,
    }

    impl DecisionMaker for SpliceDecisionMaker {
        fn decide_objects(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            if ctx.description.starts_with("Reveal cards to splice") {
                self.splice_prompts += 1;
                return self.selection.clone();
            }
            ctx.candidates
                .iter()
                .find(|candidate| candidate.legal)
                .map(|candidate| vec![candidate.id])
                .unwrap_or_default()
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

        fn decide_targets(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::TargetsContext,
        ) -> Vec<Target> {
            self.target_prompts += 1;
            vec![Target::Player(self.target)]
        }

        fn view_cards(
            &mut self,
            _game: &GameState,
            viewer: PlayerId,
            cards: &[ObjectId],
            ctx: &crate::decisions::context::ViewCardsContext,
        ) {
            if ctx.public && ctx.description.contains("spliced") {
                self.public_reveals.push((viewer, cards.to_vec()));
            }
        }
    }

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let main = CardDefinitionBuilder::new(CardId::new(), "Main Arcane Spell")
        .mana_cost(ManaCost::new())
        .card_types(vec![CardType::Sorcery])
        .subtypes(vec![Subtype::Arcane])
        .with_spell_effect(vec![Effect::gain_life(1)])
        .build();
    let life_splice = CardDefinitionBuilder::new(CardId::new(), "Life Splice")
        .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::White]))
        .card_types(vec![CardType::Instant])
        .with_ability(Ability::static_ability(StaticAbility::splice(
            crate::static_abilities::SpliceSpec {
                quality: crate::static_abilities::SpliceQuality::Arcane,
                cost: crate::cost::TotalCost::from_cost(crate::costs::Cost::validated_effect(
                    Effect::new(crate::effects::GainLifeEffect::with_filter(
                        2,
                        PlayerFilter::Opponent,
                    )),
                )),
                cost_surface: None,
            },
        )))
        .with_spell_effect(vec![Effect::gain_life(4)])
        .build();
    let damage_splice = CardDefinitionBuilder::new(CardId::new(), "Damage Splice")
        .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Red]))
        .card_types(vec![CardType::Instant])
        .with_ability(Ability::static_ability(StaticAbility::splice(
            crate::static_abilities::SpliceSpec {
                quality: crate::static_abilities::SpliceQuality::Arcane,
                cost: crate::cost::TotalCost::mana(ManaCost::from_symbols(vec![ManaSymbol::Red])),
                cost_surface: None,
            },
        )))
        .with_spell_effect(vec![Effect::deal_damage(
            2,
            ChooseSpec::Target(Box::new(ChooseSpec::Player(PlayerFilter::Opponent))),
        )])
        .build();

    let main_id = game.create_object_from_definition(&main, alice, Zone::Hand);
    let life_id = game.create_object_from_definition(&life_splice, alice, Zone::Hand);
    let damage_id = game.create_object_from_definition(&damage_splice, alice, Zone::Hand);
    let life_stable = game.object(life_id).expect("life splice exists").stable_id;
    let damage_stable = game
        .object(damage_id)
        .expect("damage splice exists")
        .stable_id;
    game.player_mut(alice)
        .expect("Alice exists")
        .mana_pool
        .add(ManaSymbol::Red, 1);

    let mut dm = SpliceDecisionMaker {
        selection: vec![life_id, damage_id],
        target: bob,
        splice_prompts: 0,
        target_prompts: 0,
        public_reveals: Vec::new(),
    };
    let stack_id = super::cast_spell_from_resolving_effect(
        &mut game,
        main_id,
        Zone::Hand,
        alice,
        &CastingMethod::Normal,
        false,
        None,
        crate::provenance::ProvNodeId::default(),
        &mut dm,
    )
    .expect("splice cast transaction should run")
    .expect("legal splice proposal should commit");

    assert_eq!(dm.splice_prompts, 1);
    assert_eq!(dm.target_prompts, 1);
    assert_eq!(
        dm.public_reveals,
        vec![
            (alice, vec![life_id, damage_id]),
            (bob, vec![life_id, damage_id]),
        ]
    );
    let entry = game
        .stack
        .iter()
        .find(|entry| entry.object_id == stack_id)
        .expect("spliced spell should be on stack");
    assert_eq!(entry.spliced_cards, vec![life_stable, damage_stable]);
    assert_eq!(entry.targets, vec![Target::Player(bob)]);
    assert!(
        game.player(alice)
            .expect("Alice exists")
            .hand
            .contains(&life_id)
    );
    assert!(
        game.player(alice)
            .expect("Alice exists")
            .hand
            .contains(&damage_id)
    );
    assert_eq!(game.player(alice).expect("Alice exists").life, 20);
    assert_eq!(game.player(bob).expect("Bob exists").life, 22);
    assert_eq!(
        game.player(alice).expect("Alice exists").mana_pool.total(),
        0
    );

    let effects = game
        .object(stack_id)
        .and_then(|spell| spell.spell_effect.as_ref())
        .expect("spliced spell should have a combined program")
        .flattened_default_effects();
    assert_eq!(effects.len(), 3);
    assert!(
        effects[0]
            .downcast_ref::<crate::effects::GainLifeEffect>()
            .is_some_and(|effect| effect.amount == Value::Fixed(1))
    );
    assert!(
        effects[1]
            .downcast_ref::<crate::effects::GainLifeEffect>()
            .is_some_and(|effect| effect.amount == Value::Fixed(4))
    );
    assert!(
        effects[2]
            .downcast_ref::<crate::effects::DealDamageEffect>()
            .is_some_and(|effect| effect.amount == Value::Fixed(2))
    );

    let mut resolve_dm = SelectFirstDecisionMaker;
    resolve_stack_entry_with(&mut game, &mut resolve_dm)
        .expect("spliced main spell should resolve");
    assert_eq!(game.player(alice).expect("Alice exists").life, 25);
    assert_eq!(game.player(bob).expect("Bob exists").life, 20);
    assert!(
        game.player(alice)
            .expect("Alice exists")
            .hand
            .contains(&life_id)
    );
    assert!(
        game.player(alice)
            .expect("Alice exists")
            .hand
            .contains(&damage_id)
    );
    let graveyard_spell = game
        .player(alice)
        .expect("Alice exists")
        .graveyard
        .iter()
        .find_map(|id| {
            game.object(*id)
                .filter(|object| object.name == "Main Arcane Spell")
        })
        .expect("main spell should move to the graveyard");
    assert_eq!(
        graveyard_spell
            .spell_effect
            .as_ref()
            .expect("main spell keeps its printed program")
            .flattened_default_effects()
            .len(),
        1,
        "splice text changes must end when the spell leaves the stack"
    );
}

#[test]
pub(super) fn splice_can_be_declined_and_illegal_selection_rolls_back_cast() {
    struct SelectSplice {
        selected: Vec<ObjectId>,
    }

    impl DecisionMaker for SelectSplice {
        fn decide_objects(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            if ctx.description.starts_with("Reveal cards to splice") {
                return self.selected.clone();
            }
            Vec::new()
        }
    }

    fn setup_splice_target_probe() -> (GameState, PlayerId, ObjectId, ObjectId) {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);
        let main = CardDefinitionBuilder::new(CardId::new(), "Arcane Target Probe")
            .mana_cost(ManaCost::new())
            .card_types(vec![CardType::Sorcery])
            .subtypes(vec![Subtype::Arcane])
            .with_spell_effect(vec![Effect::gain_life(1)])
            .build();
        let splice = CardDefinitionBuilder::new(CardId::new(), "Creature Target Splice")
            .card_types(vec![CardType::Instant])
            .with_ability(Ability::static_ability(StaticAbility::splice(
                crate::static_abilities::SpliceSpec {
                    quality: crate::static_abilities::SpliceQuality::Arcane,
                    cost: crate::cost::TotalCost::free(),
                    cost_surface: None,
                },
            )))
            .with_spell_effect(vec![Effect::deal_damage(
                2,
                ChooseSpec::Target(Box::new(ChooseSpec::Object(ObjectFilter::creature()))),
            )])
            .build();
        let main_id = game.create_object_from_definition(&main, alice, Zone::Hand);
        let splice_id = game.create_object_from_definition(&splice, alice, Zone::Hand);
        (game, alice, main_id, splice_id)
    }

    let (mut declined_game, alice, main_id, splice_id) = setup_splice_target_probe();
    let mut decline = SelectSplice {
        selected: Vec::new(),
    };
    let declined = super::cast_spell_from_resolving_effect(
        &mut declined_game,
        main_id,
        Zone::Hand,
        alice,
        &CastingMethod::Normal,
        false,
        None,
        crate::provenance::ProvNodeId::default(),
        &mut decline,
    )
    .expect("declining splice should be legal")
    .expect("main spell should commit without splice");
    assert!(
        declined_game
            .stack
            .iter()
            .find(|entry| entry.object_id == declined)
            .is_some_and(|entry| entry.spliced_cards.is_empty())
    );
    assert!(
        declined_game
            .player(alice)
            .expect("Alice exists")
            .hand
            .contains(&splice_id)
    );

    let (mut invalid_game, alice, main_id, splice_id) = setup_splice_target_probe();
    let mut invalid = SelectSplice {
        selected: vec![splice_id],
    };
    let cancelled = super::cast_spell_from_resolving_effect(
        &mut invalid_game,
        main_id,
        Zone::Hand,
        alice,
        &CastingMethod::Normal,
        false,
        None,
        crate::provenance::ProvNodeId::default(),
        &mut invalid,
    )
    .expect("illegal splice proposal should cancel cleanly");
    assert!(cancelled.is_none());
    assert!(invalid_game.stack_is_empty());
    assert!(
        invalid_game
            .object(main_id)
            .is_some_and(|spell| spell.zone == Zone::Hand)
    );
    assert!(
        invalid_game
            .object(splice_id)
            .is_some_and(|spell| spell.zone == Zone::Hand)
    );

    let (mut duplicate_game, alice, main_id, splice_id) = setup_splice_target_probe();
    let mut duplicate = SelectSplice {
        selected: vec![splice_id, splice_id],
    };
    let duplicate_result = super::cast_spell_from_resolving_effect(
        &mut duplicate_game,
        main_id,
        Zone::Hand,
        alice,
        &CastingMethod::Normal,
        false,
        None,
        crate::provenance::ProvNodeId::default(),
        &mut duplicate,
    )
    .expect("revealing one physical card twice should cancel cleanly");
    assert!(duplicate_result.is_none());
    assert!(duplicate_game.stack_is_empty());
    assert!(
        duplicate_game
            .object(main_id)
            .is_some_and(|spell| spell.zone == Zone::Hand)
    );
}

#[test]
pub(super) fn splice_is_announced_before_hybrid_payment_symbols_and_targets() {
    struct AnnouncementOrderDm {
        splice: ObjectId,
        target: PlayerId,
        events: Vec<&'static str>,
    }

    impl DecisionMaker for AnnouncementOrderDm {
        fn decide_objects(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            if ctx.description.starts_with("Reveal cards to splice") {
                self.events.push("splice");
                return vec![self.splice];
            }
            Vec::new()
        }

        fn decide_options(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            if ctx.description.starts_with("Choose how to pay pip") {
                self.events.push("hybrid");
            }
            ctx.options
                .iter()
                .find(|option| option.legal)
                .map(|option| vec![option.index])
                .unwrap_or_default()
        }

        fn decide_targets(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::TargetsContext,
        ) -> Vec<Target> {
            self.events.push("targets");
            vec![Target::Player(self.target)]
        }
    }

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    let main = CardDefinitionBuilder::new(CardId::new(), "Arcane Hybrid Probe")
        .mana_cost(ManaCost::new())
        .card_types(vec![CardType::Sorcery])
        .subtypes(vec![Subtype::Arcane])
        .with_spell_effect(vec![Effect::gain_life(1)])
        .build();
    let splice = CardDefinitionBuilder::new(CardId::new(), "Hybrid Splice")
        .card_types(vec![CardType::Instant])
        .with_ability(Ability::static_ability(StaticAbility::splice(
            crate::static_abilities::SpliceSpec {
                quality: crate::static_abilities::SpliceQuality::Arcane,
                cost: crate::cost::TotalCost::mana(ManaCost::from_pips(vec![vec![
                    ManaSymbol::Red,
                    ManaSymbol::Green,
                ]])),
                cost_surface: None,
            },
        )))
        .with_spell_effect(vec![Effect::deal_damage(
            1,
            ChooseSpec::Target(Box::new(ChooseSpec::Player(PlayerFilter::Opponent))),
        )])
        .build();
    let main_id = game.create_object_from_definition(&main, alice, Zone::Hand);
    let splice_id = game.create_object_from_definition(&splice, alice, Zone::Hand);
    game.player_mut(alice)
        .expect("Alice exists")
        .mana_pool
        .add(ManaSymbol::Red, 1);
    let mut dm = AnnouncementOrderDm {
        splice: splice_id,
        target: bob,
        events: Vec::new(),
    };

    let cast = super::cast_spell_from_resolving_effect(
        &mut game,
        main_id,
        Zone::Hand,
        alice,
        &CastingMethod::Normal,
        false,
        None,
        crate::provenance::ProvNodeId::default(),
        &mut dm,
    )
    .expect("hybrid splice transaction should run")
    .expect("hybrid splice proposal should commit");

    assert_eq!(dm.events, vec!["splice", "hybrid", "targets"]);
    assert_eq!(
        game.stack
            .iter()
            .find(|entry| entry.object_id == cast)
            .expect("hybrid-spliced spell on stack")
            .targets,
        vec![Target::Player(bob)]
    );
    assert_eq!(
        game.player(alice).expect("Alice exists").mana_pool.total(),
        0
    );
}

#[test]
pub(super) fn spell_announces_damage_distribution_after_targets_and_before_resolution() {
    struct DivideDuringAnnouncement {
        bob: PlayerId,
        charlie: PlayerId,
        events: Vec<&'static str>,
        distribution_prompts: usize,
    }

    impl DecisionMaker for DivideDuringAnnouncement {
        fn decide_targets(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::TargetsContext,
        ) -> Vec<Target> {
            self.events.push("targets");
            vec![Target::Player(self.bob), Target::Player(self.charlie)]
        }

        fn decide_distribute(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::DistributeContext,
        ) -> Vec<(Target, u32)> {
            self.events.push("distribution");
            self.distribution_prompts += 1;
            assert_eq!(ctx.total, 3);
            assert_eq!(ctx.min_per_target, 1);
            vec![
                (Target::Player(self.bob), 1),
                (Target::Player(self.charlie), 2),
            ]
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
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);
    game.turn.phase = Phase::FirstMain;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let distributed_target = ChooseSpec::WithCount(
        Box::new(ChooseSpec::Target(Box::new(ChooseSpec::Player(
            PlayerFilter::Opponent,
        )))),
        ChoiceCount::exactly(2),
    );
    let spell = CardDefinitionBuilder::new(CardId::new(), "Ordered Distribution Probe")
        .mana_cost(ManaCost::new())
        .card_types(vec![CardType::Sorcery])
        .with_spell_effect(vec![Effect::new(
            crate::effects::DealDistributedDamageEffect::new(3, distributed_target),
        )])
        .build();
    let spell_id = game.create_object_from_definition(&spell, alice, Zone::Hand);
    let mut dm = DivideDuringAnnouncement {
        bob,
        charlie,
        events: Vec::new(),
        distribution_prompts: 0,
    };

    let stack_id = super::cast_spell_from_resolving_effect(
        &mut game,
        spell_id,
        Zone::Hand,
        alice,
        &CastingMethod::Normal,
        false,
        None,
        crate::provenance::ProvNodeId::default(),
        &mut dm,
    )
    .expect("distributed spell cast transaction should run")
    .expect("distributed spell proposal should commit");

    assert_eq!(dm.events, vec!["targets", "distribution"]);
    let entry = game
        .stack
        .iter()
        .find(|entry| entry.object_id == stack_id)
        .expect("distributed spell should be on the stack");
    assert_eq!(entry.target_distributions.len(), 1);
    assert_eq!(
        entry.target_distributions[0].allocations,
        vec![(Target::Player(bob), 1), (Target::Player(charlie), 2)]
    );

    resolve_stack_entry_with(&mut game, &mut dm)
        .expect("distributed spell should use its announced division");
    assert_eq!(
        dm.distribution_prompts, 1,
        "resolution must not divide again"
    );
    assert_eq!(game.player(bob).expect("Bob exists").life, 19);
    assert_eq!(game.player(charlie).expect("Charlie exists").life, 18);
}

#[test]
pub(super) fn activation_announces_damage_distribution_after_targets() {
    struct NoResolutionDistribution;

    impl DecisionMaker for NoResolutionDistribution {
        fn decide_distribute(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::DistributeContext,
        ) -> Vec<(Target, u32)> {
            panic!("an activated ability must retain its announced distribution")
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
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);
    game.turn.phase = Phase::FirstMain;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let source = create_creature(&mut game, "Distribution Ability Probe", alice, 1, 1);
    let distributed_target = ChooseSpec::WithCount(
        Box::new(ChooseSpec::Target(Box::new(ChooseSpec::Player(
            PlayerFilter::Opponent,
        )))),
        ChoiceCount::exactly(2),
    );
    game.object_mut(source)
        .expect("ability source should exist")
        .abilities_mut()
        .push(Ability {
            kind: AbilityKind::Activated(ActivatedAbility {
                mana_cost: crate::cost::TotalCost::free(),
                effects: crate::resolution::ResolutionProgram::from_effects(vec![Effect::new(
                    crate::effects::DealDistributedDamageEffect::new(4, distributed_target),
                )]),
                choices: vec![],
                timing: ActivationTiming::AnyTime,
                additional_restrictions: vec![],
                activation_restrictions: vec![],
                mana_output: None,
                activation_condition: None,
                mana_usage_restrictions: vec![],
                is_loyalty_ability: false,
            }),
            functional_zones: vec![Zone::Battlefield],
        });

    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut trigger_queue = TriggerQueue::new();
    let progress = apply_priority_response(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(LegalAction::ActivateAbility {
            source,
            ability_index: 0,
        }),
    )
    .expect("distributed activation should begin");
    assert!(matches!(
        progress,
        GameProgress::NeedsDecisionCtx(crate::decisions::context::DecisionContext::Targets(_))
    ));

    let progress = apply_priority_response(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::Targets(vec![Target::Player(bob), Target::Player(charlie)]),
    )
    .expect("chosen targets should lead to the division announcement");
    assert!(matches!(
        progress,
        GameProgress::NeedsDecisionCtx(crate::decisions::context::DecisionContext::Distribute(_))
    ));

    apply_priority_response(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::Distribution(vec![
            (Target::Player(bob), 3),
            (Target::Player(charlie), 1),
        ]),
    )
    .expect("valid activation distribution should commit");
    let entry = game
        .stack
        .last()
        .expect("activated ability should be on stack");
    assert_eq!(
        entry.target_distributions[0].allocations,
        vec![(Target::Player(bob), 3), (Target::Player(charlie), 1)]
    );

    let mut dm = NoResolutionDistribution;
    resolve_stack_entry_with(&mut game, &mut dm)
        .expect("activated ability should use its announced distribution");
    assert_eq!(game.player(bob).expect("Bob exists").life, 17);
    assert_eq!(game.player(charlie).expect("Charlie exists").life, 19);
}

#[test]
pub(super) fn invalid_announced_distribution_rolls_back_spell_proposal() {
    struct InvalidDivision {
        target: PlayerId,
    }

    impl DecisionMaker for InvalidDivision {
        fn decide_targets(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::TargetsContext,
        ) -> Vec<Target> {
            vec![Target::Player(self.target)]
        }

        fn decide_distribute(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::DistributeContext,
        ) -> Vec<(Target, u32)> {
            vec![(Target::Player(self.target), 2)]
        }
    }

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let spell = CardDefinitionBuilder::new(CardId::new(), "Invalid Distribution Probe")
        .mana_cost(ManaCost::new())
        .card_types(vec![CardType::Sorcery])
        .with_spell_effect(vec![Effect::new(
            crate::effects::DealDistributedDamageEffect::new(
                3,
                ChooseSpec::Target(Box::new(ChooseSpec::Player(PlayerFilter::Opponent))),
            ),
        )])
        .build();
    let spell_id = game.create_object_from_definition(&spell, alice, Zone::Hand);
    let mut dm = InvalidDivision { target: bob };

    let result = super::cast_spell_from_resolving_effect(
        &mut game,
        spell_id,
        Zone::Hand,
        alice,
        &CastingMethod::Normal,
        false,
        None,
        crate::provenance::ProvNodeId::default(),
        &mut dm,
    )
    .expect("invalid announced distribution should be handled as a cancelled proposal");
    assert!(
        result.is_none(),
        "a distribution that does not assign the announced total must not commit"
    );
    assert!(game.stack_is_empty());
    assert!(
        game.object(spell_id)
            .is_some_and(|spell| spell.zone == Zone::Hand),
        "CR 601.6 rollback must restore the proposed spell"
    );
    assert_eq!(game.player(bob).expect("Bob exists").life, 20);
}

#[test]
pub(super) fn x_distribution_lookahead_requires_a_feasible_announced_division() {
    let mut game = GameState::new(
        vec![
            "Alice".to_string(),
            "Bob".to_string(),
            "Charlie".to_string(),
        ],
        20,
    );
    let alice = PlayerId::from_index(0);
    game.turn.phase = Phase::FirstMain;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.player_mut(alice)
        .expect("Alice exists")
        .mana_pool
        .add(ManaSymbol::Colorless, 1);

    let two_opponents = ChooseSpec::WithCount(
        Box::new(ChooseSpec::Target(Box::new(ChooseSpec::Player(
            PlayerFilter::Opponent,
        )))),
        ChoiceCount::exactly(2),
    );
    let spell = CardDefinitionBuilder::new(CardId::new(), "X Distribution Lookahead Probe")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::X]]))
        .card_types(vec![CardType::Sorcery])
        .with_spell_effect(vec![Effect::new(
            crate::effects::DealDistributedDamageEffect::new(Value::X, two_opponents),
        )])
        .build();
    let spell_id = game.create_object_from_definition(&spell, alice, Zone::Hand);

    assert!(
        !compute_legal_actions(&game, alice)
            .into_iter()
            .any(|action| {
                matches!(action, LegalAction::CastSpell { spell_id: id, .. } if id == spell_id)
            }),
        "X=0 or 1 cannot assign at least 1 damage to each of two targets"
    );

    game.player_mut(alice)
        .expect("Alice exists")
        .mana_pool
        .add(ManaSymbol::Colorless, 1);
    assert!(
        compute_legal_actions(&game, alice)
            .into_iter()
            .any(|action| {
                matches!(action, LegalAction::CastSpell { spell_id: id, .. } if id == spell_id)
            }),
        "CR 601.4 look-ahead must find the payable X=2 division"
    );
}

#[test]
pub(super) fn modal_x_activation_announces_modes_before_x() {
    use crate::ability::{ActivatedAbility, ActivationTiming};

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let source = create_creature(&mut game, "Modal X Ability Probe", alice, 1, 1);
    let modal_effect = Effect::choose_exactly(
        1,
        vec![
            crate::effect::EffectMode {
                source_text: "Gain 3 life".to_string(),
                effects: vec![Effect::gain_life(3)],
            },
            crate::effect::EffectMode {
                source_text: "Draw a card".to_string(),
                effects: vec![Effect::draw(1)],
            },
        ],
    );
    game.object_mut(source)
        .expect("ability source should exist")
        .abilities_mut()
        .push(Ability {
            kind: AbilityKind::Activated(ActivatedAbility {
                mana_cost: crate::cost::TotalCost::mana(ManaCost::from_pips(vec![vec![
                    ManaSymbol::X,
                ]])),
                effects: crate::resolution::ResolutionProgram::from_effects(vec![modal_effect]),
                choices: vec![],
                timing: ActivationTiming::AnyTime,
                additional_restrictions: vec![],
                activation_restrictions: vec![],
                mana_output: None,
                activation_condition: None,
                mana_usage_restrictions: vec![],
                is_loyalty_ability: false,
            }),
            functional_zones: vec![Zone::Battlefield],
        });

    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut trigger_queue = TriggerQueue::new();
    let progress = apply_priority_response(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(LegalAction::ActivateAbility {
            source,
            ability_index: 0,
        }),
    )
    .expect("proposing a modal X activation should request its first announcement");

    assert!(
        matches!(
            progress,
            GameProgress::NeedsDecisionCtx(crate::decisions::context::DecisionContext::Modes(_))
        ),
        "CR 602.2b imports the 601.2b modes-before-X order, got {progress:?}"
    );

    let progress = apply_priority_response(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::Modes(vec![0]),
    )
    .expect("choosing the activation mode should advance to X");

    assert!(
        matches!(
            progress,
            GameProgress::NeedsDecisionCtx(crate::decisions::context::DecisionContext::Number(_))
        ),
        "the activation's X value should follow modes, got {progress:?}"
    );
}

#[test]
pub(super) fn x_dependent_cast_prohibition_uses_lookahead_and_rolls_back_illegal_proposal() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.player_mut(alice)
        .expect("Alice should exist")
        .mana_pool
        .add(ManaSymbol::Colorless, 1);

    let x_spell = CardDefinitionBuilder::new(CardId::new(), "X Proposal Legality Probe")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::X]]))
        .card_types(vec![CardType::Sorcery])
        .with_spell_effect(vec![Effect::gain_life(1)])
        .build();
    let spell_id = game.create_object_from_definition(&x_spell, alice, Zone::Hand);
    let restriction_source = CardDefinitionBuilder::new(CardId::new(), "X Restriction Probe")
        .card_types(vec![CardType::Artifact])
        .with_ability(Ability {
            kind: AbilityKind::Static(StaticAbility::restriction(
                crate::effect::Restriction::cast_spells_matching(
                    crate::target::PlayerFilter::You,
                    ObjectFilter::default().with_mana_value(crate::filter::Comparison::Equal(0)),
                ),
                "You can't cast spells with mana value 0".to_string(),
            )),
            functional_zones: vec![Zone::Battlefield],
        })
        .build();
    game.create_object_from_definition(&restriction_source, alice, Zone::Battlefield);
    game.refresh_continuous_state();

    let cast_action = compute_legal_actions(&game, alice)
        .into_iter()
        .find(|action| {
            matches!(
                action,
                LegalAction::CastSpell { spell_id: id, .. } if *id == spell_id
            )
        })
        .expect("CR 601.3a look-ahead should expose the cast because X=1 is legal");

    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut trigger_queue = TriggerQueue::new();
    let progress = apply_priority_response(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(cast_action.clone()),
    )
    .expect("the legal proposal should begin with an X choice");
    assert!(matches!(
        progress,
        GameProgress::NeedsDecisionCtx(crate::decisions::context::DecisionContext::Number(_))
    ));

    let error = apply_priority_response(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::XValue(0),
    )
    .expect_err("the completed X=0 proposal must fail the final prohibition check");
    assert!(matches!(error, GameLoopError::ActionCancelled(_)));
    assert!(
        game.stack_is_empty(),
        "rollback must remove the proposed spell"
    );
    assert!(
        game.object(spell_id)
            .is_some_and(|spell| spell.zone == Zone::Hand),
        "rollback must restore the spell to its original zone and identity"
    );
    assert_eq!(
        game.player(alice).expect("Alice exists").mana_pool.total(),
        1,
        "rollback must preserve mana because no cost may be paid before 601.2e"
    );
    assert!(!state.has_pending_action());
    assert!(state.checkpoint.is_none());

    let progress = apply_priority_response(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(cast_action),
    )
    .expect("the restored spell should be castable again");
    assert!(matches!(
        progress,
        GameProgress::NeedsDecisionCtx(crate::decisions::context::DecisionContext::Number(_))
    ));
    apply_priority_response(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::XValue(1),
    )
    .expect("the completed X=1 proposal should pass and pay its cost");

    assert!(!game.stack_is_empty(), "the legal proposal should commit");
    let stack_spell = game
        .stack
        .last()
        .and_then(|entry| game.object(entry.object_id))
        .expect("the committed spell should be on the stack");
    assert_eq!(stack_spell.x_value, Some(1));
    assert_eq!(
        game.player(alice).expect("Alice exists").mana_pool.total(),
        0
    );
}

#[test]
pub(super) fn kicked_modal_range_looks_ahead_and_requires_the_promised_cost() {
    struct ConditionalModeDecisionMaker {
        pay_kicker: bool,
        optional_prompt_min: Option<usize>,
    }

    impl DecisionMaker for ConditionalModeDecisionMaker {
        fn decide_options(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            if ctx.description.starts_with("Choose mode for") && ctx.min == 0 && ctx.max == 3 {
                return vec![0, 1];
            }
            if ctx.description.starts_with("Choose optional costs") {
                self.optional_prompt_min = Some(ctx.min);
                return self.pay_kicker.then_some(0).into_iter().collect();
            }
            ctx.options
                .iter()
                .find(|option| option.legal)
                .map(|option| vec![option.index])
                .unwrap_or_default()
        }
    }

    fn setup_probe() -> (GameState, PlayerId, ObjectId) {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);
        game.player_mut(alice)
            .expect("Alice exists")
            .mana_pool
            .add(ManaSymbol::Colorless, 1);

        let modes = vec![
            crate::effect::EffectMode::new("Gain 1 life", vec![Effect::gain_life(1)]),
            crate::effect::EffectMode::new("Gain 2 life", vec![Effect::gain_life(2)]),
            crate::effect::EffectMode::new("Gain 3 life", vec![Effect::gain_life(3)]),
        ];
        let conditional_modal = crate::effects::ChooseModeEffect::choose_one(modes)
            .with_conditional_mode_range(crate::effect::ConditionalModeRange::new(
                crate::cost::OptionalCostRef::from("Kicker"),
                0,
                3,
            ));
        let definition = CardDefinitionBuilder::new(CardId::new(), "Conditional Mode Probe")
            .mana_cost(ManaCost::new())
            .card_types(vec![CardType::Sorcery])
            .with_spell_effect(vec![Effect::new(conditional_modal)])
            .kicker_mana(ManaCost::from_symbols(vec![ManaSymbol::Generic(1)]))
            .build();
        let spell_id = game.create_object_from_definition(&definition, alice, Zone::Hand);
        (game, alice, spell_id)
    }

    let (mut game, alice, spell_id) = setup_probe();
    let mut decline = ConditionalModeDecisionMaker {
        pay_kicker: false,
        optional_prompt_min: None,
    };
    let declined = super::cast_spell_from_resolving_effect(
        &mut game,
        spell_id,
        Zone::Hand,
        alice,
        &CastingMethod::Normal,
        true,
        None,
        crate::provenance::ProvNodeId::default(),
        &mut decline,
    )
    .expect("declining the required later cost should cancel cleanly");
    assert!(declined.is_none());
    assert_eq!(decline.optional_prompt_min, Some(1));
    assert!(game.stack_is_empty());
    assert!(
        game.object(spell_id)
            .is_some_and(|spell| spell.zone == Zone::Hand)
    );
    assert_eq!(
        game.player(alice).expect("Alice exists").mana_pool.total(),
        1
    );

    let mut accept = ConditionalModeDecisionMaker {
        pay_kicker: true,
        optional_prompt_min: None,
    };
    let stack_id = super::cast_spell_from_resolving_effect(
        &mut game,
        spell_id,
        Zone::Hand,
        alice,
        &CastingMethod::Normal,
        true,
        None,
        crate::provenance::ProvNodeId::default(),
        &mut accept,
    )
    .expect("the joint mode/kicker proposal should run")
    .expect("paying the promised kicker should commit the proposal");
    let entry = game
        .stack
        .iter()
        .find(|entry| entry.object_id == stack_id)
        .expect("conditional modal spell should be on the stack");
    assert_eq!(entry.chosen_modes.as_deref(), Some(&[0, 1][..]));
    assert!(entry.optional_costs_paid.was_kicked());
    assert_eq!(accept.optional_prompt_min, Some(1));
    assert_eq!(
        game.player(alice).expect("Alice exists").mana_pool.total(),
        0
    );
}

#[test]
pub(super) fn mana_optional_cost_target_hypothesis_exposes_legal_cast() {
    struct KickedTargetDecisionMaker {
        target: PlayerId,
    }

    impl DecisionMaker for KickedTargetDecisionMaker {
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

        fn decide_targets(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::TargetsContext,
        ) -> Vec<Target> {
            vec![Target::Player(self.target)]
        }
    }

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.player_mut(alice)
        .expect("Alice exists")
        .mana_pool
        .add(ManaSymbol::Colorless, 1);

    let program =
        crate::resolution::ResolutionProgram::new(vec![crate::resolution::ResolutionSegment {
            default_effects: vec![Effect::deal_damage(1, ChooseSpec::target_creature())],
            self_replacements: vec![crate::resolution::SelfReplacementBranch::new(
                crate::effect::Condition::ThisSpellWasKicked,
                vec![Effect::deal_damage(1, ChooseSpec::target_player())],
            )],
            starts_new_source_line: false,
        }]);
    let mut definition = CardDefinitionBuilder::new(CardId::new(), "Kicked Target Probe")
        .mana_cost(ManaCost::new())
        .card_types(vec![CardType::Sorcery])
        .kicker_mana(ManaCost::from_symbols(vec![ManaSymbol::Generic(1)]))
        .build();
    definition.spell_effect = Some(program);
    let spell_id = game.create_object_from_definition(&definition, alice, Zone::Hand);

    assert!(
        compute_legal_actions(&game, alice)
            .into_iter()
            .any(|action| {
                matches!(action, LegalAction::CastSpell { spell_id: id, .. } if id == spell_id)
            }),
        "CR 601.4 look-ahead must consider a payable mana kicker that changes target requirements"
    );

    let mut dm = KickedTargetDecisionMaker { target: bob };
    let stack_id = super::cast_spell_from_resolving_effect(
        &mut game,
        spell_id,
        Zone::Hand,
        alice,
        &CastingMethod::Normal,
        false,
        None,
        crate::provenance::ProvNodeId::default(),
        &mut dm,
    )
    .expect("kicked target hypothesis should run")
    .expect("payable kicker and its legal target should commit");
    let entry = game
        .stack
        .iter()
        .find(|entry| entry.object_id == stack_id)
        .expect("kicked spell should be on stack");
    assert!(entry.optional_costs_paid.was_kicked());
    assert_eq!(entry.targets, vec![Target::Player(bob)]);
}

#[test]
pub(super) fn joint_optional_cost_hypothesis_exposes_targetable_cast() {
    struct JointOptionalTargetDecisionMaker {
        target: PlayerId,
    }

    impl DecisionMaker for JointOptionalTargetDecisionMaker {
        fn decide_options(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            ctx.options
                .iter()
                .filter(|option| option.legal)
                .map(|option| option.index)
                .collect()
        }

        fn decide_targets(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::TargetsContext,
        ) -> Vec<Target> {
            vec![Target::Player(self.target)]
        }
    }

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.player_mut(alice)
        .expect("Alice exists")
        .mana_pool
        .add(ManaSymbol::Colorless, 2);

    let both_costs_paid = crate::effect::Condition::And(
        Box::new(crate::effect::Condition::ThisSpellPaidLabel(
            "Kicker".into(),
        )),
        Box::new(crate::effect::Condition::ThisSpellPaidLabel(
            "Buyback".into(),
        )),
    );
    let program =
        crate::resolution::ResolutionProgram::new(vec![crate::resolution::ResolutionSegment {
            default_effects: vec![Effect::deal_damage(1, ChooseSpec::target_creature())],
            self_replacements: vec![crate::resolution::SelfReplacementBranch::new(
                both_costs_paid,
                vec![Effect::deal_damage(1, ChooseSpec::target_player())],
            )],
            starts_new_source_line: false,
        }]);
    let mut definition = CardDefinitionBuilder::new(CardId::new(), "Joint Proposal Probe")
        .mana_cost(ManaCost::new())
        .card_types(vec![CardType::Sorcery])
        .kicker_mana(ManaCost::from_symbols(vec![ManaSymbol::Generic(1)]))
        .buyback_mana(ManaCost::from_symbols(vec![ManaSymbol::Generic(1)]))
        .build();
    definition.spell_effect = Some(program);
    let spell_id = game.create_object_from_definition(&definition, alice, Zone::Hand);

    assert!(
        compute_legal_actions(&game, alice)
            .into_iter()
            .any(|action| {
                matches!(action, LegalAction::CastSpell { spell_id: id, .. } if id == spell_id)
            }),
        "CR 601.4 look-ahead must evaluate optional-cost choices jointly"
    );

    let mut dm = JointOptionalTargetDecisionMaker { target: bob };
    let stack_id = super::cast_spell_from_resolving_effect(
        &mut game,
        spell_id,
        Zone::Hand,
        alice,
        &CastingMethod::Normal,
        false,
        None,
        crate::provenance::ProvNodeId::default(),
        &mut dm,
    )
    .expect("joint optional-cost target proposal should run")
    .expect("joint optional-cost target proposal should commit");
    let entry = game
        .stack
        .iter()
        .find(|entry| entry.object_id == stack_id)
        .expect("joint proposal should be on the stack");
    assert!(entry.optional_costs_paid.was_paid_label("Kicker"));
    assert!(entry.optional_costs_paid.was_paid_label("Buyback"));
    assert_eq!(entry.targets, vec![Target::Player(bob)]);
    assert_eq!(
        game.player(alice).expect("Alice exists").mana_pool.total(),
        0
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn bestow_aura_view_can_receive_flash_before_casting_begins() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::Combat;
    game.turn.step = Some(Step::DeclareAttackers);
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let flash_source = CardBuilder::new(CardId::from_raw(994_022), "Aura Flash Source")
        .card_types(vec![CardType::Enchantment])
        .build();
    let flash_source_id = game.create_object_from_card(&flash_source, alice, Zone::Battlefield);
    let aura_flash = crate::grant::GrantSpec::flash_to_spells_matching(
        crate::filter::ObjectFilter::default().with_subtype(Subtype::Aura),
    );
    game.object_mut(flash_source_id)
        .expect("flash-grant source should exist")
        .abilities_mut()
        .push(Ability::static_ability(StaticAbility::grants(aura_flash)));

    let host = create_creature(&mut game, "Bestow Host", alice, 2, 2);
    let bestow = CardDefinitionBuilder::new(CardId::from_raw(994_023), "Bestow Flash Probe")
        .mana_cost(ManaCost::new())
        .card_types(vec![CardType::Enchantment, CardType::Creature])
        .subtypes(vec![Subtype::Spirit])
        .power_toughness(PowerToughness::fixed(2, 2))
        .parse_text("Bestow {0}\nEnchanted creature gets +1/+1.")
        .expect("bestow probe should parse");
    let bestow_id = game.create_object_from_definition(&bestow, alice, Zone::Hand);

    let actions = crate::decision::compute_legal_actions(&game, alice);
    assert!(
        !actions.iter().any(|action| matches!(
            action,
            LegalAction::CastSpell {
                spell_id,
                casting_method: CastingMethod::Normal,
                ..
            } if *spell_id == bestow_id
        )),
        "the printed creature spell should not receive an Aura-only flash grant"
    );
    assert!(
        actions.iter().any(|action| matches!(
            action,
            LegalAction::CastSpell {
                spell_id,
                casting_method: CastingMethod::Alternative(0),
                ..
            } if *spell_id == bestow_id
        )),
        "CR 601.3b must test the proposed Bestow Aura characteristics when applying flash"
    );

    let cast_action = actions
        .into_iter()
        .find(|action| {
            matches!(
                action,
                LegalAction::CastSpell {
                    spell_id,
                    casting_method: CastingMethod::Alternative(0),
                    ..
                } if *spell_id == bestow_id
            )
        })
        .expect("Bestow action should be available");
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut trigger_queue = TriggerQueue::new();
    let progress = apply_priority_response(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(cast_action),
    )
    .expect("Bestow cast should begin through the proposed Aura flash permission");
    assert!(matches!(
        progress,
        GameProgress::NeedsDecisionCtx(crate::decisions::context::DecisionContext::Targets(_))
    ));
    apply_priority_response(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::Targets(vec![Target::Object(host)]),
    )
    .expect("the completed Bestow proposal should retain its Aura flash permission");
    assert!(
        game.stack.iter().any(|entry| game
            .object(entry.object_id)
            .is_some_and(|spell| spell.name == "Bestow Flash Probe")),
        "the legal completed proposal should commit"
    );
}

#[test]
pub(super) fn next_spell_ability_grant_applies_before_announcements_and_rolls_back() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let spell = CardDefinitionBuilder::new(CardId::new(), "Granted Conspire Proposal Probe")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)]]))
        .color_indicator(crate::color::ColorSet::RED)
        .card_types(vec![CardType::Sorcery])
        .with_spell_effect(vec![Effect::gain_life(1)])
        .build();
    let spell_id = game.create_object_from_definition(&spell, alice, Zone::Hand);
    let grant_source = CardDefinitionBuilder::new(CardId::new(), "Conspire Grant Source")
        .card_types(vec![CardType::Enchantment])
        .build();
    let grant_source_id =
        game.create_object_from_definition(&grant_source, alice, Zone::Battlefield);
    game.player_mut(alice)
        .expect("Alice exists")
        .mana_pool
        .add(ManaSymbol::Colorless, 1);
    game.add_temporary_spell_ability_grant(
        alice,
        grant_source_id,
        ObjectFilter::instant_or_sorcery().cast_by(crate::PlayerFilter::You),
        Ability::static_ability(StaticAbility::keyword_marker("Conspire"))
            .in_zones(vec![Zone::Stack]),
        1,
    );

    let cast_action = LegalAction::CastSpell {
        spell_id,
        from_zone: Zone::Hand,
        casting_method: CastingMethod::Normal,
    };
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut trigger_queue = TriggerQueue::new();
    let progress = apply_priority_response(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(cast_action),
    )
    .expect("the granted ability should participate in the cast proposal");
    let optional_costs = match progress {
        GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::SelectOptions(ctx),
        ) => ctx,
        other => panic!("expected granted Conspire during 601.2b, got {other:?}"),
    };
    assert!(
        optional_costs
            .options
            .iter()
            .any(|option| option.description.contains("Conspire")),
        "the one-shot ability must add its announcement-time cost before SpellCastEvent"
    );

    let stack_spell_id = state
        .pending_cast
        .as_ref()
        .map(|pending| pending.spell_id)
        .expect("the proposal should be tracked while the spell is on the stack");
    assert!(
        game.object(stack_spell_id)
            .expect("proposed spell exists")
            .abilities
            .iter()
            .any(|ability| matches!(
                &ability.kind,
                AbilityKind::Static(static_ability)
                    if static_ability.id()
                        == crate::static_abilities::StaticAbilityId::KeywordMarker
                        && static_ability.display() == "Conspire"
            )),
        "the grant should be attached to the stack object during 601.2a"
    );
    assert_eq!(
        game.effect_store.temporary_spell_ability_grants[0].remaining_uses, 0,
        "the matching one-shot grant should be reserved by this proposal"
    );

    let error = apply_priority_response(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::OptionalCosts(vec![(usize::MAX, 1)]),
    )
    .expect_err("an invalid optional-cost response should cancel the proposal");
    assert!(matches!(error, GameLoopError::ActionCancelled(_)));
    assert!(game.stack_is_empty());
    let restored_spell = game
        .object(spell_id)
        .expect("rollback restores spell identity");
    assert_eq!(restored_spell.zone, Zone::Hand);
    assert!(
        restored_spell.optional_costs.is_empty()
            && restored_spell.abilities.iter().all(|ability| !matches!(
                &ability.kind,
                AbilityKind::Static(static_ability)
                    if static_ability.id()
                        == crate::static_abilities::StaticAbilityId::KeywordMarker
                        && static_ability.display() == "Conspire"
            )),
        "CR 601.6 rollback must remove proposal-only cost and ability state"
    );
    assert_eq!(
        game.effect_store.temporary_spell_ability_grants[0].remaining_uses, 1,
        "rollback must restore the one-shot grant for the next legal proposal"
    );
    assert!(!state.has_pending_action());
    assert!(state.checkpoint.is_none());
}

pub(super) fn setup_spell_mana_window_probe()
-> (GameState, PlayerId, ObjectId, ObjectId, LegalAction) {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let mountain = CardDefinitionBuilder::new(CardId::new(), "Ordering Mountain")
        .card_types(vec![CardType::Land])
        .with_ability(Ability::mana(
            crate::cost::TotalCost::from_cost(crate::costs::Cost::tap()),
            vec![ManaSymbol::Red],
        ))
        .build();
    let mountain_id = game.create_object_from_definition(&mountain, alice, Zone::Battlefield);
    let spell = CardDefinitionBuilder::new(CardId::new(), "Mana Window Order Probe")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)]]))
        .card_types(vec![CardType::Sorcery])
        .additional_cost(crate::cost::TotalCost::from_cost(
            crate::costs::Cost::sacrifice(ObjectFilter::land().you_control()),
        ))
        .with_spell_effect(vec![Effect::gain_life(1)])
        .build();
    let spell_id = game.create_object_from_definition(&spell, alice, Zone::Hand);
    let cast_action = compute_legal_actions(&game, alice)
        .into_iter()
        .find(|action| {
            matches!(
                action,
                LegalAction::CastSpell { spell_id: id, .. } if *id == spell_id
            )
        })
        .expect("the spell should be potentially payable with the Mountain");

    (game, alice, mountain_id, spell_id, cast_action)
}

#[test]
pub(super) fn spell_mana_ability_window_precedes_every_cost_payment() {
    let (mut game, alice, mountain_id, _spell_id, cast_action) = setup_spell_mana_window_probe();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut trigger_queue = TriggerQueue::new();

    let progress = apply_priority_response(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(cast_action),
    )
    .expect("the cast should reach its pre-payment mana-ability window");
    let payment = match progress {
        GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::ManaPayment(ctx),
        ) => ctx,
        other => panic!("expected the authoritative mana proposal, got {other:?}"),
    };
    assert_eq!(payment.plan.mana_ability_steps.len(), 1);
    assert_eq!(payment.plan.mana_ability_steps[0].source, mountain_id);
    assert!(
        game.object(mountain_id)
            .is_some_and(|mountain| mountain.zone == Zone::Battlefield)
    );
    assert!(!game.is_tapped(mountain_id));

    let progress = apply_priority_response(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::ManaPaymentPlan(crate::mana_payment::ManaPaymentResponse::Confirm {
            plan_id: payment.plan.id,
            request_hash: payment.plan.request_hash,
        }),
    )
    .expect("confirming the plan should prepare mana before costs are paid");
    let cost_order = match progress {
        GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::SelectOptions(ctx),
        ) => ctx,
        other => panic!("expected the locked cost-order choice, got {other:?}"),
    };
    assert!(game.is_tapped(mountain_id));
    assert_eq!(
        game.player(alice).expect("Alice exists").mana_pool.total(),
        1
    );
    assert!(
        game.object(mountain_id)
            .is_some_and(|mountain| mountain.zone == Zone::Battlefield),
        "the nonmana sacrifice cost must still be unpaid when 601.2g closes"
    );

    let sacrifice_choice = cost_order
        .options
        .iter()
        .find(|option| {
            option
                .description
                .to_ascii_lowercase()
                .contains("sacrifice")
        })
        .map(|option| option.index)
        .expect("the locked cost list should include the sacrifice");
    let progress = apply_priority_response(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::NextCostChoice(sacrifice_choice),
    )
    .expect("choosing the sacrifice component should ask for its object");
    assert!(matches!(
        progress,
        GameProgress::NeedsDecisionCtx(crate::decisions::context::DecisionContext::SelectObjects(
            _
        ))
    ));
    apply_priority_response(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::CardCostChoice(mountain_id),
    )
    .expect("the locked mana should remain usable after sacrificing its source");

    assert!(!game.stack_is_empty());
    assert_eq!(
        game.player(alice).expect("Alice exists").mana_pool.total(),
        0
    );
}

#[test]
pub(super) fn closed_spell_mana_window_does_not_reopen_after_nonmana_cost_and_rolls_back() {
    let (mut game, alice, mountain_id, spell_id, cast_action) = setup_spell_mana_window_probe();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut trigger_queue = TriggerQueue::new();

    let progress = apply_priority_response(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(cast_action),
    )
    .expect("the cast should reach its pre-payment mana-ability window");
    let _payment = match progress {
        GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::ManaPayment(ctx),
        ) => ctx,
        other => panic!("expected the authoritative mana proposal, got {other:?}"),
    };
    let error = apply_priority_response(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::ManaPaymentPlan(crate::mana_payment::ManaPaymentResponse::Replan {
            preferences: crate::mana_payment::ManaPaymentPreferences {
                excluded_sources: vec![mountain_id],
                ..Default::default()
            },
        }),
    )
    .expect_err("excluding the only source must reject the plan before any cost is paid");

    assert!(matches!(error, GameLoopError::ActionCancelled(_)));
    assert!(game.stack_is_empty());
    assert!(
        game.object(spell_id)
            .is_some_and(|spell| spell.zone == Zone::Hand)
    );
    assert!(
        game.object(mountain_id)
            .is_some_and(|mountain| mountain.zone == Zone::Battlefield)
    );
    assert!(!game.is_tapped(mountain_id));
    assert_eq!(
        game.player(alice).expect("Alice exists").mana_pool.total(),
        0
    );
    assert!(!state.has_pending_action());
}

#[test]
pub(super) fn activation_mana_ability_window_precedes_every_cost_payment() {
    use crate::ability::{ActivatedAbility, ActivationTiming};

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let mountain = CardDefinitionBuilder::new(CardId::new(), "Activation Ordering Mountain")
        .card_types(vec![CardType::Land])
        .with_ability(Ability::mana(
            crate::cost::TotalCost::from_cost(crate::costs::Cost::tap()),
            vec![ManaSymbol::Red],
        ))
        .build();
    let mountain_id = game.create_object_from_definition(&mountain, alice, Zone::Battlefield);
    let source = create_creature(&mut game, "Activation Mana Window Probe", alice, 1, 1);
    game.object_mut(source)
        .expect("ability source should exist")
        .abilities_mut()
        .push(Ability {
            kind: AbilityKind::Activated(ActivatedAbility {
                mana_cost: crate::cost::TotalCost::from_costs(vec![
                    crate::costs::Cost::mana(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(
                        1,
                    )]])),
                    crate::costs::Cost::sacrifice(ObjectFilter::land().you_control()),
                ]),
                effects: crate::resolution::ResolutionProgram::from_effects(vec![
                    Effect::gain_life(1),
                ]),
                choices: vec![],
                timing: ActivationTiming::AnyTime,
                additional_restrictions: vec![],
                activation_restrictions: vec![],
                mana_output: None,
                activation_condition: None,
                mana_usage_restrictions: vec![],
                is_loyalty_ability: false,
            }),
            functional_zones: vec![Zone::Battlefield],
        });

    let activate_action = compute_legal_actions(&game, alice)
        .into_iter()
        .find(|action| {
            matches!(
                action,
                LegalAction::ActivateAbility { source: id, ability_index: 0 } if *id == source
            )
        })
        .expect("the activation should be potentially payable with the Mountain");
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut trigger_queue = TriggerQueue::new();
    let progress = apply_priority_response(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(activate_action),
    )
    .expect("the activation should reach its pre-payment mana-ability window");
    let payment = match progress {
        GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::ManaPayment(ctx),
        ) => ctx,
        other => panic!("expected the authoritative activation proposal, got {other:?}"),
    };
    assert_eq!(payment.plan.mana_ability_steps.len(), 1);
    assert_eq!(payment.plan.mana_ability_steps[0].source, mountain_id);
    assert!(!game.is_tapped(mountain_id));
    let progress = apply_priority_response(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::ManaPaymentPlan(crate::mana_payment::ManaPaymentResponse::Confirm {
            plan_id: payment.plan.id,
            request_hash: payment.plan.request_hash,
        }),
    )
    .expect("confirming the plan should precede activation-cost payment");
    let cost_order = match progress {
        GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::SelectOptions(ctx),
        ) => ctx,
        other => panic!("expected locked activation cost ordering, got {other:?}"),
    };
    assert!(game.is_tapped(mountain_id));
    assert_eq!(
        game.player(alice).expect("Alice exists").mana_pool.total(),
        1
    );
    assert!(game.object(mountain_id).is_some());

    let sacrifice_choice = cost_order
        .options
        .iter()
        .find(|option| {
            option
                .description
                .to_ascii_lowercase()
                .contains("sacrifice")
        })
        .map(|option| option.index)
        .expect("the activation's sacrifice cost should remain unpaid");
    let progress = apply_priority_response(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::NextCostChoice(sacrifice_choice),
    )
    .expect("selecting the activation sacrifice should request its object");
    assert!(matches!(
        progress,
        GameProgress::NeedsDecisionCtx(crate::decisions::context::DecisionContext::SelectObjects(
            _
        ))
    ));
    apply_priority_response(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::SacrificeTarget(mountain_id),
    )
    .expect("locked mana should pay after its source is sacrificed");

    assert!(!game.stack_is_empty());
    assert_eq!(
        game.player(alice).expect("Alice exists").mana_pool.total(),
        0
    );
}

#[test]
pub(super) fn test_casting_repeated_mode_spell_exposes_repeatable_modes_and_accepts_duplicates() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let modal_def = CardDefinitionBuilder::new(CardId::new(), "Repeatable Mode Probe")
        .mana_cost(ManaCost::new())
        .card_types(vec![CardType::Instant])
        .with_spell_effect(vec![Effect::choose_exactly_allow_repeated_modes(
            2,
            vec![
                crate::effect::EffectMode {
                    source_text: "Gain 3 life".to_string(),
                    effects: vec![Effect::gain_life(3)],
                },
                crate::effect::EffectMode {
                    source_text: "Draw a card".to_string(),
                    effects: vec![Effect::draw(1)],
                },
            ],
        )])
        .build();
    let modal_spell = game.create_object_from_definition(&modal_def, alice, Zone::Hand);

    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut trigger_queue = TriggerQueue::new();
    let progress = apply_priority_response(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(LegalAction::CastSpell {
            spell_id: modal_spell,
            from_zone: Zone::Hand,
            casting_method: CastingMethod::Normal,
        }),
    )
    .expect("casting the repeatable-mode spell should reach the mode chooser");

    let ctx = match progress {
        GameProgress::NeedsDecisionCtx(crate::decisions::context::DecisionContext::Modes(ctx)) => {
            ctx
        }
        other => panic!("expected modal choice decision, got {other:?}"),
    };

    assert!(ctx.spec.allow_repeated_modes);

    apply_priority_response(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::Modes(vec![0, 0]),
    )
    .expect("choosing the same mode twice should be accepted when repeats are allowed");

    let stack_entry = game
        .stack
        .last()
        .expect("the repeated-mode spell should be on the stack after mode selection");
    assert_eq!(stack_entry.chosen_modes.as_deref(), Some(&[0, 0][..]));
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn soul_transfer_mode_prompt_allows_two_modes_only_with_artifact_and_enchantment() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let soul_transfer = CardDefinitionBuilder::new(CardId::new(), "Soul Transfer")
        .mana_cost(ManaCost::new())
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Choose one. If you control an artifact and an enchantment as you cast this spell, you may choose both instead.\n\
• Exile target creature or planeswalker.\n\
• Return target creature or planeswalker card from your graveyard to your hand.",
        )
        .expect("Soul Transfer should parse");

    let artifact = CardBuilder::new(CardId::new(), "Artifact Probe")
        .card_types(vec![CardType::Artifact])
        .build();
    let enchantment = CardBuilder::new(CardId::new(), "Enchantment Probe")
        .card_types(vec![CardType::Enchantment])
        .build();
    let enemy_creature = CardBuilder::new(CardId::new(), "Enemy Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let graveyard_creature = CardBuilder::new(CardId::new(), "Graveyard Creature")
        .card_types(vec![CardType::Creature])
        .build();

    game.create_object_from_card(&artifact, alice, Zone::Battlefield);
    game.create_object_from_card(&enchantment, alice, Zone::Battlefield);
    game.create_object_from_card(&enemy_creature, bob, Zone::Battlefield);
    game.create_object_from_card(&graveyard_creature, alice, Zone::Graveyard);

    let spell_id = game.create_object_from_definition(&soul_transfer, alice, Zone::Hand);
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut trigger_queue = TriggerQueue::new();
    let progress = apply_priority_response(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(LegalAction::CastSpell {
            spell_id,
            from_zone: Zone::Hand,
            casting_method: CastingMethod::Normal,
        }),
    )
    .expect("casting Soul Transfer should reach mode selection");

    let ctx = match progress {
        GameProgress::NeedsDecisionCtx(crate::decisions::context::DecisionContext::Modes(ctx)) => {
            ctx
        }
        other => panic!("expected modal mode selection for Soul Transfer, got {other:?}"),
    };

    assert_eq!(ctx.spec.min_modes, 1);
    assert_eq!(ctx.spec.max_modes, 2);
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn soul_transfer_mode_prompt_stays_choose_one_without_full_control_condition() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let soul_transfer = CardDefinitionBuilder::new(CardId::new(), "Soul Transfer")
        .mana_cost(ManaCost::new())
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Choose one. If you control an artifact and an enchantment as you cast this spell, you may choose both instead.\n\
• Exile target creature or planeswalker.\n\
• Return target creature or planeswalker card from your graveyard to your hand.",
        )
        .expect("Soul Transfer should parse");

    let artifact = CardBuilder::new(CardId::new(), "Artifact Probe")
        .card_types(vec![CardType::Artifact])
        .build();
    let enemy_creature = CardBuilder::new(CardId::new(), "Enemy Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();

    game.create_object_from_card(&artifact, alice, Zone::Battlefield);
    game.create_object_from_card(&enemy_creature, bob, Zone::Battlefield);

    let spell_id = game.create_object_from_definition(&soul_transfer, alice, Zone::Hand);
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut trigger_queue = TriggerQueue::new();
    let progress = apply_priority_response(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(LegalAction::CastSpell {
            spell_id,
            from_zone: Zone::Hand,
            casting_method: CastingMethod::Normal,
        }),
    )
    .expect("casting Soul Transfer should reach mode selection");

    let ctx = match progress {
        GameProgress::NeedsDecisionCtx(crate::decisions::context::DecisionContext::Modes(ctx)) => {
            ctx
        }
        other => panic!("expected modal mode selection for Soul Transfer, got {other:?}"),
    };

    assert_eq!(ctx.spec.min_modes, 1);
    assert_eq!(ctx.spec.max_modes, 1);
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn sakashimas_will_mode_prompt_allows_two_modes_when_you_control_commander() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let sakashimas_will = CardDefinitionBuilder::new(CardId::new(), "Sakashima's Will")
        .mana_cost(ManaCost::new())
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Choose one. If you control a commander as you cast this spell, you may choose both instead.\n\
• Target opponent chooses a creature they control. You gain control of it.\n\
• Choose a creature you control. Each other creature you control becomes a copy of that creature until end of turn.",
        )
        .expect("Sakashima's Will should parse");

    let commander = CardBuilder::new(CardId::new(), "Commander Probe")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 3))
        .build();
    let enemy_creature = CardBuilder::new(CardId::new(), "Enemy Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let my_creature = CardBuilder::new(CardId::new(), "My Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();

    let commander_id = game.create_object_from_card(&commander, alice, Zone::Battlefield);
    game.set_as_commander(commander_id, alice);
    game.create_object_from_card(&enemy_creature, bob, Zone::Battlefield);
    game.create_object_from_card(&my_creature, alice, Zone::Battlefield);

    let spell_id = game.create_object_from_definition(&sakashimas_will, alice, Zone::Hand);
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut trigger_queue = TriggerQueue::new();
    let progress = apply_priority_response(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(LegalAction::CastSpell {
            spell_id,
            from_zone: Zone::Hand,
            casting_method: CastingMethod::Normal,
        }),
    )
    .expect("casting Sakashima's Will should reach mode selection");

    let ctx = match progress {
        GameProgress::NeedsDecisionCtx(crate::decisions::context::DecisionContext::Modes(ctx)) => {
            ctx
        }
        other => panic!("expected modal mode selection for Sakashima's Will, got {other:?}"),
    };

    assert_eq!(ctx.spec.min_modes, 1);
    assert_eq!(ctx.spec.max_modes, 2);
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn sakashimas_will_mode_prompt_stays_choose_one_without_commander() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let sakashimas_will = CardDefinitionBuilder::new(CardId::new(), "Sakashima's Will")
        .mana_cost(ManaCost::new())
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Choose one. If you control a commander as you cast this spell, you may choose both instead.\n\
• Target opponent chooses a creature they control. You gain control of it.\n\
• Choose a creature you control. Each other creature you control becomes a copy of that creature until end of turn.",
        )
        .expect("Sakashima's Will should parse");

    let enemy_creature = CardBuilder::new(CardId::new(), "Enemy Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let my_creature = CardBuilder::new(CardId::new(), "My Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();

    game.create_object_from_card(&enemy_creature, bob, Zone::Battlefield);
    game.create_object_from_card(&my_creature, alice, Zone::Battlefield);

    let spell_id = game.create_object_from_definition(&sakashimas_will, alice, Zone::Hand);
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut trigger_queue = TriggerQueue::new();
    let progress = apply_priority_response(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(LegalAction::CastSpell {
            spell_id,
            from_zone: Zone::Hand,
            casting_method: CastingMethod::Normal,
        }),
    )
    .expect("casting Sakashima's Will should reach mode selection");

    let ctx = match progress {
        GameProgress::NeedsDecisionCtx(crate::decisions::context::DecisionContext::Modes(ctx)) => {
            ctx
        }
        other => panic!("expected modal mode selection for Sakashima's Will, got {other:?}"),
    };

    assert_eq!(ctx.spec.min_modes, 1);
    assert_eq!(ctx.spec.max_modes, 1);
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn sakashimas_will_second_mode_chooses_copy_source_once_and_does_not_target_each_other_creature()
 {
    struct ChooseNamedCreatureDecisionMaker {
        name: &'static str,
        object_choice_calls: usize,
        object_choice_descriptions: Vec<String>,
    }

    impl DecisionMaker for ChooseNamedCreatureDecisionMaker {
        fn decide_objects(
            &mut self,
            game: &GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            self.object_choice_calls += 1;
            self.object_choice_descriptions
                .push(ctx.description.clone());
            ctx.candidates
                .iter()
                .find(|candidate| {
                    candidate.legal
                        && game
                            .object(candidate.id)
                            .is_some_and(|object| object.name == self.name)
                })
                .map(|candidate| vec![candidate.id])
                .unwrap_or_default()
        }
    }

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let sakashimas_will = CardDefinitionBuilder::new(CardId::new(), "Sakashima's Will")
        .mana_cost(ManaCost::new())
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Choose one. If you control a commander as you cast this spell, you may choose both instead.\n\
• Target opponent chooses a creature they control. You gain control of it.\n\
• Choose a creature you control. Each other creature you control becomes a copy of that creature until end of turn.",
        )
        .expect("Sakashima's Will should parse");

    let commander = CardBuilder::new(CardId::new(), "Commander Probe")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 3))
        .build();
    let copy_source = CardBuilder::new(CardId::new(), "Copy Source")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(5, 5))
        .build();
    let other_one = CardBuilder::new(CardId::new(), "Other One")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let other_two = CardBuilder::new(CardId::new(), "Other Two")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();

    let commander_id = game.create_object_from_card(&commander, alice, Zone::Battlefield);
    game.set_as_commander(commander_id, alice);
    game.create_object_from_card(&copy_source, alice, Zone::Battlefield);
    let other_one_id = game.create_object_from_card(&other_one, alice, Zone::Battlefield);
    let other_two_id = game.create_object_from_card(&other_two, alice, Zone::Battlefield);

    let spell_id = game.create_object_from_definition(&sakashimas_will, alice, Zone::Hand);
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut trigger_queue = TriggerQueue::new();
    let progress = apply_priority_response(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(LegalAction::CastSpell {
            spell_id,
            from_zone: Zone::Hand,
            casting_method: CastingMethod::Normal,
        }),
    )
    .expect("casting Sakashima's Will should reach mode selection");

    match progress {
        GameProgress::NeedsDecisionCtx(crate::decisions::context::DecisionContext::Modes(_)) => {}
        other => panic!("expected mode selection for Sakashima's Will, got {other:?}"),
    }

    let progress = apply_priority_response(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::Modes(vec![1]),
    )
    .expect("choosing Sakashima's Will copy mode should finish casting");

    assert!(
        matches!(
            progress,
            GameProgress::NeedsDecisionCtx(crate::decisions::context::DecisionContext::Priority(_))
                | GameProgress::Continue
        ),
        "copy mode should not ask for targets after mode selection, got {progress:?}"
    );
    assert!(
        game.stack
            .last()
            .is_some_and(|entry| entry.targets.is_empty()),
        "copy mode should not target each other creature"
    );

    let mut dm = ChooseNamedCreatureDecisionMaker {
        name: "Copy Source",
        object_choice_calls: 0,
        object_choice_descriptions: Vec::new(),
    };
    resolve_stack_entry_with(&mut game, &mut dm).expect("Sakashima's Will should resolve");

    assert_eq!(
        dm.object_choice_calls, 1,
        "the copy source should be chosen exactly once; prompts: {:?}",
        dm.object_choice_descriptions
    );
    assert_eq!(game.calculated_power(other_one_id), Some(5));
    assert_eq!(game.calculated_toughness(other_one_id), Some(5));
    assert_eq!(game.calculated_power(other_two_id), Some(5));
    assert_eq!(game.calculated_toughness(other_two_id), Some(5));
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn sakashimas_will_second_mode_copying_legendary_creature_triggers_legend_rule() {
    struct ChooseCopySourceThenLegendDecisionMaker {
        copy_source: ObjectId,
        object_choice_calls: usize,
        legend_choice_calls: usize,
    }

    impl DecisionMaker for ChooseCopySourceThenLegendDecisionMaker {
        fn decide_objects(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            if ctx.description.contains("legend rule") {
                self.legend_choice_calls += 1;
                assert!(
                    ctx.candidates
                        .iter()
                        .any(|candidate| candidate.id == self.copy_source),
                    "the original legendary copy source should be one keep option"
                );
                return vec![self.copy_source];
            }

            self.object_choice_calls += 1;
            vec![self.copy_source]
        }
    }

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let sakashimas_will = CardDefinitionBuilder::new(CardId::new(), "Sakashima's Will")
        .mana_cost(ManaCost::new())
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Choose one. If you control a commander as you cast this spell, you may choose both instead.\n\
• Target opponent chooses a creature they control. You gain control of it.\n\
• Choose a creature you control. Each other creature you control becomes a copy of that creature until end of turn.",
        )
        .expect("Sakashima's Will should parse");

    let commander = CardBuilder::new(CardId::new(), "Commander Probe")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 3))
        .build();
    let legendary_source = CardBuilder::new(CardId::new(), "Legendary Source")
        .supertypes(vec![Supertype::Legendary])
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(5, 5))
        .build();
    let other_one = CardBuilder::new(CardId::new(), "Other One")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let other_two = CardBuilder::new(CardId::new(), "Other Two")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();

    let commander_id = game.create_object_from_card(&commander, alice, Zone::Battlefield);
    game.set_as_commander(commander_id, alice);
    let copy_source_id = game.create_object_from_card(&legendary_source, alice, Zone::Battlefield);
    let other_one_id = game.create_object_from_card(&other_one, alice, Zone::Battlefield);
    let other_two_id = game.create_object_from_card(&other_two, alice, Zone::Battlefield);

    let spell_id = game.create_object_from_definition(&sakashimas_will, alice, Zone::Hand);
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut trigger_queue = TriggerQueue::new();
    apply_priority_response(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(LegalAction::CastSpell {
            spell_id,
            from_zone: Zone::Hand,
            casting_method: CastingMethod::Normal,
        }),
    )
    .expect("casting Sakashima's Will should reach mode selection");
    apply_priority_response(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::Modes(vec![1]),
    )
    .expect("choosing the copy mode should finish casting");

    let mut dm = ChooseCopySourceThenLegendDecisionMaker {
        copy_source: copy_source_id,
        object_choice_calls: 0,
        legend_choice_calls: 0,
    };
    resolve_stack_entry_with(&mut game, &mut dm).expect("Sakashima's Will should resolve");

    for copied in [commander_id, other_one_id, other_two_id] {
        let chars = game
            .calculated_characteristics(copied)
            .expect("copied creature should have calculated characteristics");
        assert_eq!(chars.name, "Legendary Source");
        assert!(chars.supertypes.contains(&Supertype::Legendary));
    }

    crate::game_loop::check_and_apply_sbas_with(&mut game, &mut trigger_queue, &mut dm)
        .expect("legend rule should be handled after Sakashima's Will resolves");

    assert_eq!(
        dm.object_choice_calls, 1,
        "Sakashima's Will should choose the copy source exactly once"
    );
    assert_eq!(
        dm.legend_choice_calls, 1,
        "copying a legendary creature should produce one legend-rule decision"
    );
    assert!(game.battlefield.contains(&copy_source_id));
    assert!(!game.battlefield.contains(&commander_id));
    assert!(!game.battlefield.contains(&other_one_id));
    assert!(!game.battlefield.contains(&other_two_id));
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn sakashimas_will_first_mode_targets_opponent_then_gains_their_chosen_creature() {
    struct ChooseNamedCreatureDecisionMaker {
        chooser: PlayerId,
        name: &'static str,
        object_choice_calls: usize,
    }

    impl DecisionMaker for ChooseNamedCreatureDecisionMaker {
        fn decide_objects(
            &mut self,
            game: &GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            self.object_choice_calls += 1;
            assert_eq!(
                ctx.player, self.chooser,
                "the targeted opponent should choose the creature"
            );
            ctx.candidates
                .iter()
                .find(|candidate| {
                    candidate.legal
                        && game
                            .object(candidate.id)
                            .is_some_and(|object| object.name == self.name)
                })
                .map(|candidate| vec![candidate.id])
                .unwrap_or_default()
        }
    }

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let sakashimas_will = CardDefinitionBuilder::new(CardId::new(), "Sakashima's Will")
        .mana_cost(ManaCost::new())
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Choose one. If you control a commander as you cast this spell, you may choose both instead.\n\
• Target opponent chooses a creature they control. You gain control of it.\n\
• Choose a creature you control. Each other creature you control becomes a copy of that creature until end of turn.",
        )
        .expect("Sakashima's Will should parse");

    let commander = CardBuilder::new(CardId::new(), "Commander Probe")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 3))
        .build();
    let enemy_creature = CardBuilder::new(CardId::new(), "Enemy Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let enemy_decoy = CardBuilder::new(CardId::new(), "Enemy Decoy")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();

    let commander_id = game.create_object_from_card(&commander, alice, Zone::Battlefield);
    game.set_as_commander(commander_id, alice);
    let enemy_creature_id = game.create_object_from_card(&enemy_creature, bob, Zone::Battlefield);
    game.create_object_from_card(&enemy_decoy, bob, Zone::Battlefield);

    let spell_id = game.create_object_from_definition(&sakashimas_will, alice, Zone::Hand);
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut trigger_queue = TriggerQueue::new();
    let progress = apply_priority_response(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(LegalAction::CastSpell {
            spell_id,
            from_zone: Zone::Hand,
            casting_method: CastingMethod::Normal,
        }),
    )
    .expect("casting Sakashima's Will should reach mode selection");

    match progress {
        GameProgress::NeedsDecisionCtx(crate::decisions::context::DecisionContext::Modes(_)) => {}
        other => panic!("expected mode selection for Sakashima's Will, got {other:?}"),
    }

    let progress = apply_priority_response(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::Modes(vec![0]),
    )
    .expect("choosing Sakashima's Will control mode should ask for a target opponent");

    let targets_ctx = match progress {
        GameProgress::NeedsDecisionCtx(crate::decisions::context::DecisionContext::Targets(
            ctx,
        )) => ctx,
        other => panic!("expected target selection after choosing mode 1, got {other:?}"),
    };
    assert_eq!(targets_ctx.requirements.len(), 1);
    assert_eq!(targets_ctx.requirements[0].min_targets, 1);
    assert!(
        targets_ctx.requirements[0]
            .legal_targets
            .contains(&Target::Player(bob)),
        "targeted opponent should be a legal target: {:?}",
        targets_ctx.requirements[0].legal_targets
    );

    apply_priority_response(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::Targets(vec![Target::Player(bob)]),
    )
    .expect("targeting Bob should finish casting");

    let mut dm = ChooseNamedCreatureDecisionMaker {
        chooser: bob,
        name: "Enemy Creature",
        object_choice_calls: 0,
    };
    resolve_stack_entry_with(&mut game, &mut dm).expect("Sakashima's Will should resolve");

    assert_eq!(
        dm.object_choice_calls, 1,
        "the targeted opponent should choose exactly one creature"
    );
    assert_eq!(game.controller_of_id(enemy_creature_id), Some(alice));
}

#[test]
pub(super) fn test_apply_blocker_declarations_allows_blocking_multiple_attackers_with_ability() {
    let mut game = setup_game();
    let mut tq = TriggerQueue::new();
    let mut combat = CombatState::default();

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let attacker1 = create_creature(&mut game, "Attacker 1", alice, 2, 2);
    let attacker2 = create_creature(&mut game, "Attacker 2", alice, 2, 2);
    let blocker = create_creature(&mut game, "Blocker", bob, 1, 4);

    // Grant: "can block an additional creature each combat" so it can block two attackers.
    game.object_mut(blocker)
        .expect("blocker exists")
        .abilities_mut()
        .push(Ability {
            kind: AbilityKind::Static(StaticAbility::can_block_additional_creature_each_combat(1)),
            functional_zones: vec![Zone::Battlefield],
        });

    combat.attackers.push(crate::combat_state::AttackerInfo {
        creature: attacker1,
        target: AttackTarget::Player(bob),
    });
    combat.attackers.push(crate::combat_state::AttackerInfo {
        creature: attacker2,
        target: AttackTarget::Player(bob),
    });

    let decls = vec![
        BlockerDeclaration {
            blocker,
            blocking: attacker1,
        },
        BlockerDeclaration {
            blocker,
            blocking: attacker2,
        },
    ];

    apply_blocker_declarations(&mut game, &mut combat, &mut tq, &decls, bob)
        .expect("should allow blocker to block multiple attackers with ability");
}

#[test]
pub(super) fn watcher_in_the_web_can_block_eight_attackers() {
    let mut game = setup_game();
    let mut tq = TriggerQueue::new();
    let mut combat = CombatState::default();

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let watcher = create_creature(&mut game, "Watcher in the Web", bob, 2, 5);

    game.object_mut(watcher)
        .expect("watcher exists")
        .abilities_mut()
        .push(Ability {
            kind: AbilityKind::Static(StaticAbility::can_block_additional_creature_each_combat(7)),
            functional_zones: vec![Zone::Battlefield],
        });

    let mut declarations = Vec::new();
    for idx in 0..8 {
        let attacker = create_creature(&mut game, &format!("Attacker {idx}"), alice, 2, 2);
        combat.attackers.push(crate::combat_state::AttackerInfo {
            creature: attacker,
            target: AttackTarget::Player(bob),
        });
        declarations.push(BlockerDeclaration {
            blocker: watcher,
            blocking: attacker,
        });
    }

    apply_blocker_declarations(&mut game, &mut combat, &mut tq, &declarations, bob)
        .expect("Watcher in the Web should block up to eight attackers");
}

#[test]
pub(super) fn watcher_in_the_web_cannot_block_nine_attackers() {
    let mut game = setup_game();
    let mut tq = TriggerQueue::new();
    let mut combat = CombatState::default();

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let watcher = create_creature(&mut game, "Watcher in the Web", bob, 2, 5);

    game.object_mut(watcher)
        .expect("watcher exists")
        .abilities_mut()
        .push(Ability {
            kind: AbilityKind::Static(StaticAbility::can_block_additional_creature_each_combat(7)),
            functional_zones: vec![Zone::Battlefield],
        });

    let mut declarations = Vec::new();
    for idx in 0..9 {
        let attacker = create_creature(&mut game, &format!("Attacker {idx}"), alice, 2, 2);
        combat.attackers.push(crate::combat_state::AttackerInfo {
            creature: attacker,
            target: AttackTarget::Player(bob),
        });
        declarations.push(BlockerDeclaration {
            blocker: watcher,
            blocking: attacker,
        });
    }

    let err = apply_blocker_declarations(&mut game, &mut combat, &mut tq, &declarations, bob)
        .expect_err("Watcher in the Web should not block nine attackers");
    let message = format!("{err:?}");
    assert!(
        message.contains("InvalidBlockers"),
        "expected invalid blockers error, got {message}"
    );
}

#[test]
pub(super) fn test_apply_blocker_declarations_enforces_maximum_blockers() {
    let mut game = setup_game();
    let mut tq = TriggerQueue::new();
    let mut combat = CombatState::default();

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let attacker = create_creature(&mut game, "Elusive Attacker", alice, 2, 2);
    let blocker1 = create_creature(&mut game, "Blocker 1", bob, 1, 1);
    let blocker2 = create_creature(&mut game, "Blocker 2", bob, 1, 1);

    // "Can't be blocked by more than one creature."
    game.object_mut(attacker)
        .expect("attacker exists")
        .abilities_mut()
        .push(Ability {
            kind: AbilityKind::Static(StaticAbility::cant_be_blocked_by_more_than(1)),
            functional_zones: vec![Zone::Battlefield],
        });

    combat.attackers.push(crate::combat_state::AttackerInfo {
        creature: attacker,
        target: AttackTarget::Player(bob),
    });

    let decls = vec![
        BlockerDeclaration {
            blocker: blocker1,
            blocking: attacker,
        },
        BlockerDeclaration {
            blocker: blocker2,
            blocking: attacker,
        },
    ];

    let err = apply_blocker_declarations(&mut game, &mut combat, &mut tq, &decls, bob)
        .expect_err("should reject too many blockers");
    let msg = format!("{err:?}");
    assert!(
        msg.contains("InvalidBlockers"),
        "expected invalid blockers error, got {msg}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn phyrexian_colossus_requires_three_or_more_blockers() {
    let mut game = setup_game();
    let mut tq = TriggerQueue::new();
    let mut combat = CombatState::default();

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let colossus_def = CardDefinitionBuilder::new(CardId::new(), "Phyrexian Colossus")
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .power_toughness(PowerToughness::fixed(8, 8))
        .parse_text(
            "Trample\nPhyrexian Colossus doesn't untap during your untap step.\nPay 8 life: Untap Phyrexian Colossus.\nPhyrexian Colossus can't be blocked except by three or more creatures.",
        )
        .expect("Phyrexian Colossus should parse for combat test");
    let attacker = game.create_object_from_definition(&colossus_def, alice, Zone::Battlefield);
    let blocker1 = create_creature(&mut game, "Blocker 1", bob, 1, 1);
    let blocker2 = create_creature(&mut game, "Blocker 2", bob, 1, 1);
    let blocker3 = create_creature(&mut game, "Blocker 3", bob, 1, 1);

    combat.attackers.push(crate::combat_state::AttackerInfo {
        creature: attacker,
        target: AttackTarget::Player(bob),
    });

    let two_blockers = vec![
        BlockerDeclaration {
            blocker: blocker1,
            blocking: attacker,
        },
        BlockerDeclaration {
            blocker: blocker2,
            blocking: attacker,
        },
    ];

    let err = apply_blocker_declarations(&mut game, &mut combat, &mut tq, &two_blockers, bob)
        .expect_err("two blockers should be illegal against Phyrexian Colossus");
    let msg = format!("{err:?}");
    assert!(
        msg.contains("requires 3 blockers") || msg.contains("needs at least 3 blockers"),
        "expected minimum-blockers rejection, got {msg}"
    );

    let three_blockers = vec![
        BlockerDeclaration {
            blocker: blocker1,
            blocking: attacker,
        },
        BlockerDeclaration {
            blocker: blocker2,
            blocking: attacker,
        },
        BlockerDeclaration {
            blocker: blocker3,
            blocking: attacker,
        },
    ];

    apply_blocker_declarations(&mut game, &mut combat, &mut tq, &three_blockers, bob)
        .expect("three blockers should be legal against Phyrexian Colossus");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn phyrexian_colossus_untap_activation_requires_eight_life() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let colossus_def = CardDefinitionBuilder::new(CardId::new(), "Phyrexian Colossus")
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .power_toughness(PowerToughness::fixed(8, 8))
        .parse_text(
            "Trample\nPhyrexian Colossus doesn't untap during your untap step.\nPay 8 life: Untap Phyrexian Colossus.\nPhyrexian Colossus can't be blocked except by three or more creatures.",
        )
        .expect("Phyrexian Colossus should parse for activation test");
    let colossus_id = game.create_object_from_definition(&colossus_def, alice, Zone::Battlefield);

    game.player_mut(alice).expect("alice exists").life = 20;
    let can_activate_with_twenty = compute_legal_actions(&game, alice)
        .into_iter()
        .any(|action| matches!(action, LegalAction::ActivateAbility { source, .. } if source == colossus_id));
    assert!(
        can_activate_with_twenty,
        "Phyrexian Colossus untap ability should be legal at 20 life"
    );

    game.player_mut(alice).expect("alice exists").life = 7;
    let can_activate_with_seven = compute_legal_actions(&game, alice)
        .into_iter()
        .any(|action| matches!(action, LegalAction::ActivateAbility { source, .. } if source == colossus_id));
    assert!(
        !can_activate_with_seven,
        "Phyrexian Colossus untap ability should be illegal below 8 life"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn elven_riders_only_walls_or_fliers_can_block() {
    let can_block = |blocker_kind: &str| {
        let mut game = setup_game();
        let mut tq = TriggerQueue::new();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let elven_riders_def = CardDefinitionBuilder::new(CardId::new(), "Elven Riders")
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Elf])
            .power_toughness(PowerToughness::fixed(3, 3))
            .parse_text(
                "This creature can't be blocked except by Walls and/or creatures with flying.",
            )
            .expect("Elven Riders should parse for combat test");
        let attacker =
            game.create_object_from_definition(&elven_riders_def, alice, Zone::Battlefield);

        let blocker = match blocker_kind {
            "wall" => {
                let wall_def = CardDefinitionBuilder::new(CardId::new(), "Wall Blocker")
                    .card_types(vec![CardType::Creature])
                    .subtypes(vec![Subtype::Wall])
                    .power_toughness(PowerToughness::fixed(0, 4))
                    .build();
                game.create_object_from_definition(&wall_def, bob, Zone::Battlefield)
            }
            "flying" => {
                let flying_def = CardDefinitionBuilder::new(CardId::new(), "Flying Blocker")
                    .card_types(vec![CardType::Creature])
                    .power_toughness(PowerToughness::fixed(1, 1))
                    .parse_text("Flying")
                    .expect("flying blocker definition should parse");
                game.create_object_from_definition(&flying_def, bob, Zone::Battlefield)
            }
            _ => create_creature(&mut game, "Ground Blocker", bob, 2, 2),
        };

        let mut combat = CombatState::default();
        combat.attackers.push(crate::combat_state::AttackerInfo {
            creature: attacker,
            target: AttackTarget::Player(bob),
        });
        game.update_cant_effects();

        apply_blocker_declarations(
            &mut game,
            &mut combat,
            &mut tq,
            &[BlockerDeclaration {
                blocker,
                blocking: attacker,
            }],
            bob,
        )
        .is_ok()
    };

    let legal_with_wall = can_block("wall");
    assert!(
        legal_with_wall,
        "Wall blocker should be legal against Elven Riders"
    );

    let legal_with_flying = can_block("flying");
    assert!(
        legal_with_flying,
        "Flying blocker should be legal against Elven Riders"
    );

    let illegal_with_ground = !can_block("ground");
    assert!(
        illegal_with_ground,
        "Non-Wall nonflying blocker should be illegal against Elven Riders"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn skoa_embermage_grandeur_activation_requires_named_card_and_two_mountains() {
    use crate::decision::LegalAction;

    let can_activate = |mountains: usize, has_named_copy: bool| {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let skoa_def = CardDefinitionBuilder::new(CardId::new(), "Skoa, Embermage")
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Goblin, Subtype::Wizard])
            .power_toughness(PowerToughness::fixed(4, 4))
            .parse_text(
                "When Skoa enters, it deals 4 damage to any target.\nDiscard another card named Skoa, Embermage, Sacrifice two Mountains: Skoa deals 4 damage to any target.",
            )
            .expect("Skoa, Embermage should parse");
        let skoa_id = game.create_object_from_definition(&skoa_def, alice, Zone::Battlefield);

        let mountain = CardBuilder::new(CardId::new(), "Mountain")
            .card_types(vec![CardType::Land])
            .subtypes(vec![Subtype::Mountain])
            .build();
        for _ in 0..mountains {
            game.create_object_from_card(&mountain, alice, Zone::Battlefield);
        }

        if has_named_copy {
            let named_copy = CardBuilder::new(CardId::new(), "Skoa, Embermage")
                .card_types(vec![CardType::Creature])
                .subtypes(vec![Subtype::Goblin, Subtype::Wizard])
                .power_toughness(PowerToughness::fixed(4, 4))
                .build();
            game.create_object_from_card(&named_copy, alice, Zone::Hand);
        }

        crate::decision::compute_legal_actions(&game, alice)
            .into_iter()
            .any(|action| matches!(action, LegalAction::ActivateAbility { source, .. } if source == skoa_id))
    };

    assert!(
        !can_activate(1, true),
        "Skoa grandeur should be illegal with fewer than two Mountains"
    );
    assert!(
        !can_activate(2, false),
        "Skoa grandeur should be illegal without another card named Skoa, Embermage in hand"
    );
    let _ = can_activate(2, true);
}

#[test]
pub(super) fn test_marhault_elsdragon_rampage_buffs_for_blockers_beyond_first() {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let mut combat = CombatState::default();

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let marhault_card = CardBuilder::new(CardId::from_raw(2001), "Marhault Elsdragon")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(4, 6))
        .build();
    let marhault_id = game.create_object_from_card(&marhault_card, alice, Zone::Battlefield);
    game.object_mut(marhault_id)
        .expect("Marhault should exist")
        .abilities_mut()
        .push(Ability::triggered(
            Trigger::this_becomes_blocked(),
            vec![Effect::pump(
                Value::EventValue(EventValueSpec::BlockersBeyondFirst { multiplier: 1 }),
                Value::EventValue(EventValueSpec::BlockersBeyondFirst { multiplier: 1 }),
                crate::target::ChooseSpec::Source,
                Until::EndOfTurn,
            )],
        ));

    let blocker_1 = create_creature(&mut game, "Blocker 1", bob, 1, 1);
    let blocker_2 = create_creature(&mut game, "Blocker 2", bob, 1, 1);
    let blocker_3 = create_creature(&mut game, "Blocker 3", bob, 1, 1);

    combat.attackers.push(crate::combat_state::AttackerInfo {
        creature: marhault_id,
        target: AttackTarget::Player(bob),
    });

    let declarations = vec![
        BlockerDeclaration {
            blocker: blocker_1,
            blocking: marhault_id,
        },
        BlockerDeclaration {
            blocker: blocker_2,
            blocking: marhault_id,
        },
        BlockerDeclaration {
            blocker: blocker_3,
            blocking: marhault_id,
        },
    ];

    apply_blocker_declarations(
        &mut game,
        &mut combat,
        &mut trigger_queue,
        &declarations,
        bob,
    )
    .expect("should apply blocker declarations");
    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("should put combat triggers on stack");

    while !game.stack_is_empty() {
        resolve_stack_entry(&mut game).expect("trigger should resolve");
    }

    game.refresh_continuous_state();
    assert_eq!(
        game.calculated_power(marhault_id),
        Some(6),
        "Rampage 1 with three blockers should grant +2/+2"
    );
    assert_eq!(
        game.calculated_toughness(marhault_id),
        Some(8),
        "Rampage 1 with three blockers should grant +2/+2"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn rampaging_cyclops_loses_power_only_when_two_or_more_creatures_block_it() {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let mut combat = CombatState::default();

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let rampaging_cyclops_def =
        CardDefinitionBuilder::new(CardId::from_raw(1), "Rampaging Cyclops")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(4, 4))
            .parse_text(
                "This creature gets -2/-0 as long as two or more creatures are blocking it.",
            )
            .expect("Rampaging Cyclops should parse for runtime test");
    let cyclops =
        game.create_object_from_definition(&rampaging_cyclops_def, alice, Zone::Battlefield);
    let blocker_1 = create_creature(&mut game, "Blocker 1", bob, 1, 1);
    let blocker_2 = create_creature(&mut game, "Blocker 2", bob, 1, 1);

    combat.attackers.push(crate::combat_state::AttackerInfo {
        creature: cyclops,
        target: AttackTarget::Player(bob),
    });

    apply_blocker_declarations(
        &mut game,
        &mut combat,
        &mut trigger_queue,
        &[BlockerDeclaration {
            blocker: blocker_1,
            blocking: cyclops,
        }],
        bob,
    )
    .expect("one blocker should be a legal block");
    game.refresh_continuous_state();
    assert_eq!(
        game.calculated_power(cyclops),
        Some(4),
        "Rampaging Cyclops should keep 4 power with only one blocker"
    );
    assert_eq!(
        game.calculated_toughness(cyclops),
        Some(4),
        "Rampaging Cyclops toughness should not change"
    );

    apply_blocker_declarations(
        &mut game,
        &mut combat,
        &mut trigger_queue,
        &[
            BlockerDeclaration {
                blocker: blocker_1,
                blocking: cyclops,
            },
            BlockerDeclaration {
                blocker: blocker_2,
                blocking: cyclops,
            },
        ],
        bob,
    )
    .expect("two blockers should be a legal block");
    game.refresh_continuous_state();
    assert_eq!(
        game.calculated_power(cyclops),
        Some(2),
        "Rampaging Cyclops should get -2/-0 with two blockers"
    );
    assert_eq!(
        game.calculated_toughness(cyclops),
        Some(4),
        "Rampaging Cyclops toughness should still not change"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn duplicant_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(20_512), "Duplicant")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(6)]]))
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .subtypes(vec![Subtype::Shapeshifter])
        .power_toughness(PowerToughness::fixed(2, 4))
        .parse_text(
            "Imprint — When this creature enters, you may exile target nontoken creature.\n\
             As long as a card exiled with this creature is a creature card, this creature has the power, toughness, and creature types of the last creature card exiled with it. It's still a Shapeshifter.",
        )
        .expect("Duplicant should parse strictly for runtime tests")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) struct DuplicantMayDecisionMaker {
    pub(super) accept: bool,
}

#[cfg(ironsmith_runtime_parser_tests)]
impl DecisionMaker for DuplicantMayDecisionMaker {
    fn decide_boolean(
        &mut self,
        _game: &GameState,
        _ctx: &crate::decisions::context::BooleanContext,
    ) -> bool {
        self.accept
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn duplicant_enters_trigger(
    def: &crate::cards::CardDefinition,
) -> crate::ability::TriggeredAbility {
    def.abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered.clone()),
            _ => None,
        })
        .expect("Duplicant should have an enters trigger")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn resolve_duplicant_enters_trigger(
    game: &mut GameState,
    def: &crate::cards::CardDefinition,
    duplicant_id: ObjectId,
    controller: PlayerId,
    target: ObjectId,
    accept: bool,
) {
    let triggered = duplicant_enters_trigger(def);
    let target_spec = triggered
        .choices
        .first()
        .cloned()
        .expect("Duplicant enters trigger should target a nontoken creature");
    let event = TriggerEvent::new_with_provenance(
        EnterBattlefieldEvent::new(duplicant_id, Zone::Stack),
        crate::provenance::ProvNodeId::default(),
    );
    let mut dm = DuplicantMayDecisionMaker { accept };
    let mut ctx = crate::effects::ExecutionContext::new(duplicant_id, controller, &mut dm)
        .with_triggering_event(event)
        .with_targets(vec![crate::effects::ResolvedTarget::Object(target)])
        .with_target_assignments(vec![crate::game_state::TargetAssignment {
            spec: target_spec,
            range: 0..1,
        }]);

    for effect in &triggered.effects {
        crate::effects::execute_effect(game, effect, &mut ctx)
            .expect("Duplicant enters trigger should resolve");
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn duplicant_exiles_chosen_nontoken_creature_and_copies_last_creature_card_characteristics()
 {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let duplicant = duplicant_definition();
    let duplicant_id = game.create_object_from_definition(&duplicant, alice, Zone::Battlefield);
    let target_card = CardBuilder::new(CardId::from_raw(20_513), "Zombie Warrior")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Zombie, Subtype::Warrior])
        .power_toughness(PowerToughness::fixed(5, 3))
        .build();
    let target_id = game.create_object_from_card(&target_card, bob, Zone::Battlefield);

    resolve_duplicant_enters_trigger(&mut game, &duplicant, duplicant_id, alice, target_id, true);
    game.refresh_continuous_state();

    let linked = game.get_exiled_with_source_links(duplicant_id);
    assert_eq!(
        linked.len(),
        1,
        "accepting Duplicant's optional trigger should link exactly one exiled card"
    );
    assert_eq!(
        game.object(linked[0]).map(|object| object.name.as_str()),
        Some("Zombie Warrior"),
        "Duplicant should link the creature card exiled by its own trigger"
    );
    assert_eq!(game.current_power(duplicant_id), Some(5));
    assert_eq!(game.current_toughness(duplicant_id), Some(3));
    let subtypes = game
        .current_subtypes(duplicant_id)
        .expect("Duplicant should have current subtypes");
    assert!(
        subtypes.contains(&Subtype::Zombie)
            && subtypes.contains(&Subtype::Warrior)
            && subtypes.contains(&Subtype::Shapeshifter),
        "Duplicant should copy creature types and remain a Shapeshifter, got {subtypes:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn duplicant_declining_optional_exile_keeps_printed_characteristics() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let duplicant = duplicant_definition();
    let duplicant_id = game.create_object_from_definition(&duplicant, alice, Zone::Battlefield);
    let target_id = create_creature(&mut game, "Declined Bear", bob, 6, 6);

    resolve_duplicant_enters_trigger(&mut game, &duplicant, duplicant_id, alice, target_id, false);
    game.refresh_continuous_state();

    assert!(
        game.get_exiled_with_source_links(duplicant_id).is_empty(),
        "declining Duplicant's optional trigger should not exile or link a card"
    );
    assert_eq!(game.current_power(duplicant_id), Some(2));
    assert_eq!(game.current_toughness(duplicant_id), Some(4));
    assert!(
        game.objects_in_zone(Zone::Battlefield).contains(&target_id),
        "declining Duplicant's optional trigger should leave the target on the battlefield"
    );
    let subtypes = game
        .current_subtypes(duplicant_id)
        .expect("Duplicant should have current subtypes");
    assert!(subtypes.contains(&Subtype::Shapeshifter));
    assert!(
        !subtypes.contains(&Subtype::Bear),
        "Duplicant should not copy creature types without a linked exiled creature card"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn duplicant_ignores_linked_exiled_creature_tokens_for_card_characteristics() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let duplicant = duplicant_definition();
    let duplicant_id = game.create_object_from_definition(&duplicant, alice, Zone::Battlefield);
    let token_card = CardBuilder::new(CardId::from_raw(20_514), "Exiled Zombie Token")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Zombie])
        .power_toughness(PowerToughness::fixed(7, 7))
        .build();
    let token_id = game.create_object_from_card(&token_card, bob, Zone::Exile);
    game.object_mut(token_id)
        .expect("linked exiled token should exist")
        .kind = ObjectKind::Token;
    game.add_exiled_with_source_link(duplicant_id, token_id);

    game.refresh_continuous_state();

    assert_eq!(game.current_power(duplicant_id), Some(2));
    assert_eq!(game.current_toughness(duplicant_id), Some(4));
    let subtypes = game
        .current_subtypes(duplicant_id)
        .expect("Duplicant should have current subtypes");
    assert!(subtypes.contains(&Subtype::Shapeshifter));
    assert!(
        !subtypes.contains(&Subtype::Zombie),
        "Duplicant should ignore linked exiled tokens because its static ability cares about creature cards, got {subtypes:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn duplicant_enters_trigger_targets_nontoken_creatures_only() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let duplicant = duplicant_definition();
    let duplicant_id = game.create_object_from_definition(&duplicant, alice, Zone::Battlefield);
    let nontoken = create_creature(&mut game, "Nontoken Target", bob, 2, 2);
    let token = create_creature(&mut game, "Token Target", bob, 2, 2);
    game.object_mut(token)
        .expect("token target should exist")
        .kind = ObjectKind::Token;

    let triggered = duplicant_enters_trigger(&duplicant);
    let requirements =
        extract_target_requirements(&game, &triggered.effects, alice, Some(duplicant_id));
    assert_eq!(
        requirements.len(),
        1,
        "Duplicant should have one target requirement"
    );
    let legal_targets = &requirements[0].legal_targets;
    assert!(
        legal_targets.contains(&Target::Object(nontoken)),
        "nontoken creatures should be legal Duplicant targets, got {legal_targets:?}"
    );
    assert!(
        !legal_targets.contains(&Target::Object(token)),
        "token creatures should not be legal Duplicant targets, got {legal_targets:?}"
    );
}

pub(super) fn create_creature(
    game: &mut GameState,
    name: &str,
    owner: PlayerId,
    power: i32,
    toughness: i32,
) -> ObjectId {
    let card = CardBuilder::new(CardId::from_raw(1), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(power, toughness))
        .build();
    game.create_object_from_card(&card, owner, Zone::Battlefield)
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) struct ChooseSecondDieResult;

#[cfg(ironsmith_runtime_parser_tests)]
impl DecisionMaker for ChooseSecondDieResult {
    fn decide_options(
        &mut self,
        _game: &GameState,
        _ctx: &crate::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        vec![1]
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn valiant_endeavor_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(73_600), "Valiant Endeavor")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Roll two d6 and choose one result. Destroy each creature with power greater than or equal to that result. Then create a number of 2/2 white Knight creature tokens with vigilance equal to the other result.")
        .expect("Valiant Endeavor should parse strictly")
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn valiant_endeavor_uses_chosen_result_for_destroy_and_other_result_for_tokens() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let def = valiant_endeavor_definition();
    let source = game.create_object_from_definition(&def, alice, Zone::Stack);
    let small = create_creature(&mut game, "Small Creature", bob, 1, 1);
    let equal = create_creature(&mut game, "Equal Creature", bob, 2, 2);
    let large = create_creature(&mut game, "Large Creature", alice, 5, 5);
    let equal_stable = game.object(equal).expect("equal creature exists").stable_id;
    let large_stable = game.object(large).expect("large creature exists").stable_id;

    game.force_next_die_roll(5);
    game.force_next_die_roll(2);

    let mut decisions = ChooseSecondDieResult;
    let mut ctx = ExecutionContext::new(source, alice, &mut decisions);
    execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        source,
        def.spell_effect
            .as_ref()
            .expect("Valiant Endeavor should have a spell effect"),
        None,
        &[],
    )
    .expect("Valiant Endeavor spell effect should resolve");

    assert!(
        game.battlefield.contains(&small),
        "creature below the chosen result should survive"
    );
    let equal_graveyard_id = game
        .find_object_by_stable_id(equal_stable)
        .expect("destroyed equal creature should still be tracked by stable id");
    assert!(
        !game.battlefield.contains(&equal)
            && game
                .player(bob)
                .unwrap()
                .graveyard
                .contains(&equal_graveyard_id),
        "creature with power equal to the chosen result should be destroyed"
    );
    let large_graveyard_id = game
        .find_object_by_stable_id(large_stable)
        .expect("destroyed large creature should still be tracked by stable id");
    assert!(
        !game.battlefield.contains(&large)
            && game
                .player(alice)
                .unwrap()
                .graveyard
                .contains(&large_graveyard_id),
        "creature with power greater than the chosen result should be destroyed"
    );

    let knight_tokens = game
        .battlefield
        .iter()
        .copied()
        .filter(|&id| {
            game.object(id)
                .is_some_and(|object| object.name == "Knight")
        })
        .collect::<Vec<_>>();
    assert_eq!(
        knight_tokens.len(),
        5,
        "the token count should use the unchosen die result"
    );
    for token_id in knight_tokens {
        let token = game.object(token_id).expect("Knight token should exist");
        assert_eq!(
            token.owner, alice,
            "Valiant Endeavor should create tokens for its controller"
        );
        assert!(
            token.card_types.contains(&CardType::Creature),
            "Valiant Endeavor tokens should be creatures"
        );
        assert!(
            token.subtypes.contains(&Subtype::Knight),
            "Valiant Endeavor tokens should be Knights"
        );
        assert_eq!(game.current_power(token_id), Some(2));
        assert_eq!(game.current_toughness(token_id), Some(2));
        assert_eq!(
            game.current_colors(token_id),
            Some(crate::color::ColorSet::WHITE),
            "Valiant Endeavor tokens should be white"
        );
        assert!(
            game.object_has_static_ability_id(
                token_id,
                crate::static_abilities::StaticAbilityId::Vigilance,
            ),
            "Valiant Endeavor tokens should have vigilance"
        );
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn valiant_endeavor_first_result_choice_uses_second_result_for_tokens() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let def = valiant_endeavor_definition();
    let source = game.create_object_from_definition(&def, alice, Zone::Stack);
    let medium = create_creature(&mut game, "Medium Creature", bob, 4, 4);
    let large = create_creature(&mut game, "Large Creature", bob, 5, 5);
    let large_stable = game.object(large).expect("large creature exists").stable_id;

    game.force_next_die_roll(5);
    game.force_next_die_roll(2);

    let mut decisions = SelectFirstDecisionMaker;
    let mut ctx = ExecutionContext::new(source, alice, &mut decisions);
    execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        source,
        def.spell_effect
            .as_ref()
            .expect("Valiant Endeavor should have a spell effect"),
        None,
        &[],
    )
    .expect("Valiant Endeavor spell effect should resolve");

    assert!(
        game.battlefield.contains(&medium),
        "creature below the chosen first result should survive"
    );
    let large_graveyard_id = game
        .find_object_by_stable_id(large_stable)
        .expect("destroyed large creature should still be tracked by stable id");
    assert!(
        !game.battlefield.contains(&large)
            && game
                .player(bob)
                .unwrap()
                .graveyard
                .contains(&large_graveyard_id),
        "creature with power equal to the chosen first result should be destroyed"
    );

    let knight_count = game
        .battlefield
        .iter()
        .filter(|&&id| {
            game.object(id)
                .is_some_and(|object| object.name == "Knight")
        })
        .count();
    assert_eq!(
        knight_count, 2,
        "choosing the first die result should create tokens equal to the second result"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn firkraag_cunning_instigator_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(72_940), "Firkraag, Cunning Instigator")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::Blue],
            vec![ManaSymbol::Red],
        ]))
        .supertypes(vec![Supertype::Legendary])
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Dragon])
        .power_toughness(PowerToughness::fixed(3, 3))
        .parse_text(
            "Flying, haste\n\
             Whenever one or more Dragons you control attack an opponent, goad target creature that player controls.\n\
             Whenever a creature deals combat damage to one of your opponents, if that creature had to attack this combat, you put a +1/+1 counter on Firkraag and you draw a card.",
        )
        .expect("Firkraag, Cunning Instigator should parse strictly")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn create_firkraag_game() -> (GameState, PlayerId, PlayerId, PlayerId, ObjectId) {
    let mut game = setup_three_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);
    let firkraag = game.create_object_from_definition(
        &firkraag_cunning_instigator_definition(),
        alice,
        Zone::Battlefield,
    );
    (game, alice, bob, charlie, firkraag)
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn firkraag_attack_trigger_goads_only_creature_that_attacked_player_controls() {
    let (mut game, alice, bob, charlie, _firkraag) = create_firkraag_game();
    let dragon = create_creature(&mut game, "Dragon Ally", alice, 4, 4);
    game.object_mut(dragon)
        .expect("Dragon Ally should exist")
        .subtypes
        .push(Subtype::Dragon);
    let bob_creature = create_creature(&mut game, "Bob Creature", bob, 2, 2);
    let charlie_creature = create_creature(&mut game, "Charlie Creature", charlie, 2, 2);

    game.combat = Some(crate::combat_state::CombatState {
        attackers: vec![crate::combat_state::AttackerInfo {
            creature: dragon,
            target: AttackTarget::Player(bob),
        }],
        ..Default::default()
    });
    let event = TriggerEvent::new_with_provenance(
        crate::events::combat::CreatureAttackedEvent::with_total_attackers(
            dragon,
            crate::triggers::AttackEventTarget::Player(bob),
            1,
        ),
        crate::provenance::ProvNodeId::default(),
    );
    let mut trigger_queue = TriggerQueue::new();
    for trigger in crate::triggers::check_triggers(&game, &event) {
        trigger_queue.add(trigger);
    }
    let mut dm = ChooseSpecificObjectDecisionMaker::new(bob_creature);
    put_triggers_on_stack_with_dm(&mut game, &mut trigger_queue, &mut dm)
        .expect("Firkraag attack trigger should go on the stack");

    assert!(
        dm.seen_candidates.contains(&bob_creature),
        "Bob's creature should be a legal goad target"
    );
    assert!(
        !dm.seen_candidates.contains(&charlie_creature),
        "Charlie's creature should not be targetable by the trigger for attacking Bob"
    );

    let mut auto = AutoPassDecisionMaker;
    resolve_stack_entry_with(&mut game, &mut auto).expect("Firkraag goad trigger should resolve");
    assert!(game.is_goaded(bob_creature));
    assert!(!game.is_goaded(charlie_creature));
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn firkraag_attack_trigger_fires_once_for_each_attacked_opponent() {
    let (mut game, alice, bob, charlie, _firkraag) = create_firkraag_game();
    let bob_dragon = create_creature(&mut game, "Bob-Bound Dragon", alice, 4, 4);
    game.object_mut(bob_dragon)
        .expect("Bob-Bound Dragon should exist")
        .subtypes
        .push(Subtype::Dragon);
    let charlie_dragon = create_creature(&mut game, "Charlie-Bound Dragon", alice, 4, 4);
    game.object_mut(charlie_dragon)
        .expect("Charlie-Bound Dragon should exist")
        .subtypes
        .push(Subtype::Dragon);
    game.remove_summoning_sickness(bob_dragon);
    game.remove_summoning_sickness(charlie_dragon);
    let bob_creature = create_creature(&mut game, "Bob Creature", bob, 2, 2);
    let charlie_creature = create_creature(&mut game, "Charlie Creature", charlie, 2, 2);

    game.turn.active_player = alice;
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(crate::game_state::Step::DeclareAttackers);
    let mut combat = crate::combat_state::CombatState::default();
    let mut trigger_queue = TriggerQueue::new();
    apply_attacker_declarations(
        &mut game,
        &mut combat,
        &mut trigger_queue,
        &[
            AttackerDeclaration {
                creature: bob_dragon,
                target: AttackTarget::Player(bob),
            },
            AttackerDeclaration {
                creature: charlie_dragon,
                target: AttackTarget::Player(charlie),
            },
        ],
    )
    .expect("both Dragons should be declared as attackers");
    game.combat = Some(combat);

    assert_eq!(
        trigger_queue.entries.len(),
        2,
        "Firkraag should trigger once for each opponent attacked by Dragons"
    );

    let mut dm = ChooseObjectByOnlyLegalSetDecisionMaker::new(vec![bob_creature, charlie_creature]);
    put_triggers_on_stack_with_dm(&mut game, &mut trigger_queue, &mut dm)
        .expect("Firkraag attack triggers should go on the stack");

    assert_eq!(dm.chosen.len(), 2);
    assert!(dm.chosen.contains(&bob_creature));
    assert!(dm.chosen.contains(&charlie_creature));
    assert!(
        dm.seen_candidate_sets.iter().any(|candidates| {
            candidates.contains(&bob_creature) && !candidates.contains(&charlie_creature)
        }),
        "one trigger should target only the attacked Bob's creatures"
    );
    assert!(
        dm.seen_candidate_sets.iter().any(|candidates| {
            candidates.contains(&charlie_creature) && !candidates.contains(&bob_creature)
        }),
        "one trigger should target only the attacked Charlie's creatures"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn firkraag_attack_trigger_fires_once_for_multiple_dragons_attacking_same_opponent() {
    let (mut game, alice, bob, _charlie, _firkraag) = create_firkraag_game();
    let first_dragon = create_creature(&mut game, "First Dragon", alice, 4, 4);
    game.object_mut(first_dragon)
        .expect("First Dragon should exist")
        .subtypes
        .push(Subtype::Dragon);
    let second_dragon = create_creature(&mut game, "Second Dragon", alice, 4, 4);
    game.object_mut(second_dragon)
        .expect("Second Dragon should exist")
        .subtypes
        .push(Subtype::Dragon);
    game.remove_summoning_sickness(first_dragon);
    game.remove_summoning_sickness(second_dragon);

    game.turn.active_player = alice;
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(crate::game_state::Step::DeclareAttackers);
    let mut combat = crate::combat_state::CombatState::default();
    let mut trigger_queue = TriggerQueue::new();
    apply_attacker_declarations(
        &mut game,
        &mut combat,
        &mut trigger_queue,
        &[
            AttackerDeclaration {
                creature: first_dragon,
                target: AttackTarget::Player(bob),
            },
            AttackerDeclaration {
                creature: second_dragon,
                target: AttackTarget::Player(bob),
            },
        ],
    )
    .expect("both Dragons should be declared as attackers");

    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "Firkraag should trigger once for one or more Dragons attacking the same opponent"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn firkraag_attack_trigger_does_not_fire_for_dragon_attacking_planeswalker() {
    let (mut game, alice, bob, _charlie, _firkraag) = create_firkraag_game();
    let dragon = create_creature(&mut game, "Planeswalker-Bound Dragon", alice, 4, 4);
    game.object_mut(dragon)
        .expect("Planeswalker-Bound Dragon should exist")
        .subtypes
        .push(Subtype::Dragon);
    game.remove_summoning_sickness(dragon);
    let planeswalker = CardBuilder::new(CardId::from_raw(72_942), "Bob Planeswalker")
        .card_types(vec![CardType::Planeswalker])
        .loyalty(4)
        .build();
    let planeswalker_id = game.create_object_from_card(&planeswalker, bob, Zone::Battlefield);

    game.turn.active_player = alice;
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(crate::game_state::Step::DeclareAttackers);
    let mut combat = crate::combat_state::CombatState::default();
    let mut trigger_queue = TriggerQueue::new();
    apply_attacker_declarations(
        &mut game,
        &mut combat,
        &mut trigger_queue,
        &[AttackerDeclaration {
            creature: dragon,
            target: AttackTarget::Planeswalker(planeswalker_id),
        }],
    )
    .expect("Dragon should be able to attack an opponent's planeswalker");

    assert!(
        trigger_queue.entries.is_empty(),
        "Firkraag should trigger only for Dragons attacking an opponent, not a planeswalker"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn firkraag_damage_trigger_requires_creature_that_had_to_attack() {
    let (mut game, alice, bob, charlie, firkraag) = create_firkraag_game();
    let forced_attacker = create_creature(&mut game, "Forced Attacker", bob, 2, 2);
    game.add_goad_effect(forced_attacker, alice, Until::Forever, firkraag);
    game.remove_summoning_sickness(forced_attacker);
    let library_card = CardBuilder::new(CardId::from_raw(72_941), "Drawn Card").build();
    game.create_object_from_card(&library_card, alice, Zone::Library);

    game.turn.active_player = bob;
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(crate::game_state::Step::DeclareAttackers);
    let mut combat = crate::combat_state::CombatState::default();
    let mut declaration_triggers = TriggerQueue::new();
    apply_attacker_declarations(
        &mut game,
        &mut combat,
        &mut declaration_triggers,
        &[AttackerDeclaration {
            creature: forced_attacker,
            target: AttackTarget::Player(charlie),
        }],
    )
    .expect("goaded creature should be declared as an attacker");
    assert!(
        combat.creature_had_to_attack_this_combat(forced_attacker),
        "combat state should snapshot that the creature had to attack when declared"
    );
    game.combat = Some(combat);
    game.effect_store.goad_effects.clear();
    assert!(
        !game.is_goaded(forced_attacker),
        "the runtime check must not rely on current goad state at damage time"
    );

    let damage_event = TriggerEvent::new_with_provenance(
        crate::events::DamageEvent::with_cause(
            forced_attacker,
            crate::events::DamageTarget::Player(charlie),
            2,
            true,
            crate::events::EventCause::from_combat_damage(forced_attacker, bob),
        ),
        crate::provenance::ProvNodeId::default(),
    );
    let mut trigger_queue = TriggerQueue::new();
    for trigger in crate::triggers::check_triggers(&game, &damage_event) {
        trigger_queue.add(trigger);
    }
    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("Firkraag damage trigger should go on the stack for forced attacker");
    assert_eq!(game.stack.len(), 1);

    let mut auto = AutoPassDecisionMaker;
    resolve_stack_entry_with(&mut game, &mut auto).expect("Firkraag damage trigger should resolve");
    assert_eq!(
        game.object(firkraag)
            .expect("Firkraag should still exist")
            .counters
            .get(&crate::object::CounterType::PlusOnePlusOne)
            .copied()
            .unwrap_or(0),
        1,
        "Firkraag should get a +1/+1 counter"
    );
    assert_eq!(
        game.player(alice).expect("Alice should exist").hand.len(),
        1
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn firkraag_damage_trigger_does_not_fire_when_creature_did_not_have_to_attack() {
    let (game, _alice, bob, charlie, _firkraag) = create_firkraag_game();
    let mut game = game;
    let voluntary_attacker = create_creature(&mut game, "Voluntary Attacker", bob, 2, 2);
    let damage_event = TriggerEvent::new_with_provenance(
        crate::events::DamageEvent::with_cause(
            voluntary_attacker,
            crate::events::DamageTarget::Player(charlie),
            2,
            true,
            crate::events::EventCause::from_combat_damage(voluntary_attacker, bob),
        ),
        crate::provenance::ProvNodeId::default(),
    );

    assert!(
        crate::triggers::check_triggers(&game, &damage_event).is_empty(),
        "Firkraag should not trigger for a creature that did not have to attack"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn sp_dr_piloted_by_peni_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(1_001_337), "SP//dr, Piloted by Peni")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::White],
            vec![ManaSymbol::Blue],
        ]))
        .supertypes(vec![Supertype::Legendary])
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .subtypes(vec![Subtype::Spider, Subtype::Hero])
        .power_toughness(PowerToughness::fixed(4, 4))
        .parse_text(
            "Vigilance
When SP//dr enters, put a +1/+1 counter on target creature.
Whenever a modified creature you control deals combat damage to a player, draw a card. (Equipment, Auras you control, and counters are modifications.)",
        )
        .expect("SP//dr, Piloted by Peni should parse strictly")
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn sp_dr_enters_trigger_puts_counter_on_target_creature() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let target = create_creature(&mut game, "Counter Target", alice, 2, 2);
    let sp_dr = game.create_object_from_definition(
        &sp_dr_piloted_by_peni_definition(),
        alice,
        Zone::Battlefield,
    );
    let event = crate::events::RawEvent::new(
        crate::events::ZoneChangeEvent::with_cause(
            sp_dr,
            Zone::Stack,
            Zone::Battlefield,
            crate::events::cause::EventCause::from_game_rule(),
            None,
        ),
        crate::provenance::ProvNodeId::default(),
    );

    let mut trigger_queue = TriggerQueue::new();
    for trigger in crate::triggers::check_triggers(&game, &event) {
        trigger_queue.add(trigger);
    }
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "SP//dr should trigger when it enters"
    );

    let mut dm = ChooseSpecificObjectDecisionMaker::new(target);
    put_triggers_on_stack_with_dm(&mut game, &mut trigger_queue, &mut dm)
        .expect("SP//dr ETB trigger should go on the stack with a target");
    resolve_stack_entry(&mut game).expect("SP//dr ETB trigger should resolve");

    assert_eq!(
        game.counter_count(target, crate::object::CounterType::PlusOnePlusOne),
        1,
        "SP//dr should put a +1/+1 counter on the chosen target creature"
    );
    assert_eq!(
        game.counter_count(sp_dr, crate::object::CounterType::PlusOnePlusOne),
        0,
        "the chosen target, not SP//dr by default, should get the counter"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn sp_dr_draws_when_modified_creature_you_control_hits_player() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.create_object_from_definition(
        &sp_dr_piloted_by_peni_definition(),
        alice,
        Zone::Battlefield,
    );
    let attacker = create_creature(&mut game, "Modified Attacker", alice, 2, 2);
    game.add_counters(attacker, crate::object::CounterType::PlusOnePlusOne, 1)
        .expect("attacker should get a +1/+1 counter");
    let library_card = CardBuilder::new(CardId::from_raw(1_001_338), "Drawn Card").build();
    game.create_object_from_card(&library_card, alice, Zone::Library);

    let damage_event = TriggerEvent::new_with_provenance(
        crate::events::DamageEvent::with_cause(
            attacker,
            crate::events::DamageTarget::Player(bob),
            2,
            true,
            crate::events::EventCause::from_combat_damage(attacker, alice),
        ),
        crate::provenance::ProvNodeId::default(),
    );
    let mut trigger_queue = TriggerQueue::new();
    for trigger in crate::triggers::check_triggers(&game, &damage_event) {
        trigger_queue.add(trigger);
    }
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "SP//dr should trigger for modified creatures its controller controls"
    );

    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("SP//dr combat-damage trigger should go on the stack");
    resolve_stack_entry(&mut game).expect("SP//dr combat-damage trigger should resolve");

    assert_eq!(
        game.player(alice).expect("Alice exists").hand.len(),
        1,
        "SP//dr should draw Alice a card"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn sp_dr_combat_damage_trigger_requires_modified_creature_you_control() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.create_object_from_definition(
        &sp_dr_piloted_by_peni_definition(),
        alice,
        Zone::Battlefield,
    );
    let unmodified_alice_creature = create_creature(&mut game, "Unmodified Attacker", alice, 2, 2);
    let modified_bob_creature = create_creature(&mut game, "Bob Modified Attacker", bob, 2, 2);
    game.add_counters(
        modified_bob_creature,
        crate::object::CounterType::PlusOnePlusOne,
        1,
    )
    .expect("Bob's creature should get a +1/+1 counter");

    for (source, controller, message) in [
        (
            unmodified_alice_creature,
            alice,
            "unmodified creatures should not trigger SP//dr",
        ),
        (
            modified_bob_creature,
            bob,
            "modified creatures controlled by an opponent should not trigger SP//dr",
        ),
    ] {
        let damage_event = TriggerEvent::new_with_provenance(
            crate::events::DamageEvent::with_cause(
                source,
                crate::events::DamageTarget::Player(bob),
                2,
                true,
                crate::events::EventCause::from_combat_damage(source, controller),
            ),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(
            crate::triggers::check_triggers(&game, &damage_event).is_empty(),
            "{message}"
        );
    }
}

#[test]
pub(super) fn awaken_cast_action_is_available_even_when_normal_cast_is_legal() {
    use crate::cards::definitions::{basic_plains, basic_swamp};

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    game.turn.active_player = alice;
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);

    for _ in 0..6 {
        game.create_object_from_definition(&basic_swamp(), alice, Zone::Battlefield);
    }
    game.create_object_from_definition(&basic_plains(), alice, Zone::Battlefield);
    create_creature(&mut game, "Silvercoat Lion", bob, 2, 2);

    let destroy_target = crate::target::ChooseSpec::target(crate::target::ChooseSpec::Object(
        crate::target::ObjectFilter::creature().opponent_controls(),
    ));
    let spell = CardDefinitionBuilder::new(CardId::from_raw(881_010), "Awaken Action Probe")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::Black],
            vec![ManaSymbol::Black],
        ]))
        .card_types(vec![CardType::Sorcery])
        .with_spell_effect(vec![Effect::destroy(destroy_target)])
        .awaken(
            4,
            ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(5)],
                vec![ManaSymbol::Black],
                vec![ManaSymbol::Black],
            ]),
        )
        .build();
    let spell_id = game.create_object_from_definition(&spell, alice, Zone::Hand);

    let actions = crate::decision::compute_legal_actions(&game, alice);
    assert!(
        actions.iter().any(|action| matches!(
            action,
            LegalAction::CastSpell {
                spell_id: id,
                from_zone: Zone::Hand,
                casting_method: CastingMethod::Normal,
            } if *id == spell_id
        )),
        "normal cast should be available; actions={actions:?}"
    );
    assert!(
        actions.iter().any(|action| matches!(
            action,
            LegalAction::CastSpell {
                spell_id: id,
                from_zone: Zone::Hand,
                casting_method: CastingMethod::Alternative(_),
            } if *id == spell_id
        )),
        "awaken cast should also be available when the normal cast is legal; actions={actions:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn act_of_aggression_gains_control_untaps_and_grants_haste_until_end_of_turn() {
    use crate::static_abilities::StaticAbilityId;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let act = CardDefinitionBuilder::new(CardId::from_raw(82_300), "Act of Aggression")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::Red, ManaSymbol::Life(2)],
            vec![ManaSymbol::Red, ManaSymbol::Life(2)],
        ]))
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Gain control of target creature an opponent controls until end of turn. Untap that creature. It gains haste until end of turn.",
        )
        .expect("Act of Aggression should parse");

    let stolen_id = create_creature(&mut game, "Borrowed Bear", bob, 2, 2);
    let friendly_id = create_creature(&mut game, "Friendly Bear", alice, 2, 2);
    game.tap(stolen_id);

    let spell_effect = act.spell_effect.as_ref().expect("Act should have effects");
    let target_requirements = super::targeting::extract_target_requirements(
        &game,
        spell_effect.flattened_default_effects(),
        alice,
        None,
    );
    assert_eq!(
        target_requirements.len(),
        1,
        "Act should ask for one creature target"
    );
    assert!(
        target_requirements[0]
            .legal_targets
            .contains(&Target::Object(stolen_id)),
        "Act should be able to target an opponent's creature"
    );
    assert!(
        !target_requirements[0]
            .legal_targets
            .contains(&Target::Object(friendly_id)),
        "Act should not be able to target your own creature"
    );

    let act_id = game.create_object_from_definition(&act, alice, Zone::Stack);
    game.push_to_stack(
        StackEntry::new(act_id, alice)
            .with_targets(vec![Target::Object(stolen_id)])
            .with_target_assignments(vec![crate::game_state::TargetAssignment {
                spec: target_requirements[0].spec.clone(),
                range: 0..1,
            }]),
    );

    resolve_stack_entry(&mut game).expect("Act of Aggression should resolve");
    game.refresh_continuous_state();

    assert_eq!(
        game.current_controller(stolen_id),
        Some(alice),
        "Alice should control the targeted opponent creature after Act resolves"
    );
    assert!(
        !game.is_tapped(stolen_id),
        "Act should untap the targeted creature"
    );
    assert!(
        game.object_has_static_ability_id(stolen_id, StaticAbilityId::Haste),
        "Act should grant haste to the targeted creature"
    );

    crate::turn::execute_cleanup_step(&mut game);
    game.refresh_continuous_state();

    assert_eq!(
        game.current_controller(stolen_id),
        Some(bob),
        "control should revert at end of turn"
    );
    assert!(
        !game.object_has_static_ability_id(stolen_id, StaticAbilityId::Haste),
        "haste should expire at end of turn"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn target_opponent_gains_control_keeps_player_target_visible_to_object_effect() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let wrong_turn = CardDefinitionBuilder::new(CardId::from_raw(82_301), "Wrong Turn")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Blue],
        ]))
        .card_types(vec![CardType::Instant])
        .parse_text("Target opponent gains control of target creature.")
        .expect("Wrong Turn should parse");

    let creature_id = create_creature(&mut game, "Borrowed Mulldrifter", alice, 2, 2);
    let spell_effect = wrong_turn
        .spell_effect
        .as_ref()
        .expect("Wrong Turn should have effects");
    let target_requirements = super::targeting::extract_target_requirements(
        &game,
        spell_effect.flattened_default_effects(),
        alice,
        None,
    );
    let player_spec = target_requirements
        .iter()
        .find(|requirement| requirement.legal_targets.contains(&Target::Player(bob)))
        .expect("Wrong Turn should target an opponent")
        .spec
        .clone();
    let creature_spec = target_requirements
        .iter()
        .find(|requirement| {
            requirement
                .legal_targets
                .contains(&Target::Object(creature_id))
        })
        .expect("Wrong Turn should target a creature")
        .spec
        .clone();

    let wrong_turn_id = game.create_object_from_definition(&wrong_turn, alice, Zone::Stack);
    game.push_to_stack(
        StackEntry::new(wrong_turn_id, alice)
            .with_targets(vec![Target::Player(bob), Target::Object(creature_id)])
            .with_target_assignments(vec![
                crate::game_state::TargetAssignment {
                    spec: player_spec,
                    range: 0..1,
                },
                crate::game_state::TargetAssignment {
                    spec: creature_spec,
                    range: 1..2,
                },
            ]),
    );

    resolve_stack_entry(&mut game).expect("Wrong Turn should resolve");
    game.refresh_continuous_state();

    assert_eq!(
        game.current_controller(creature_id),
        Some(bob),
        "the target opponent should gain control of the target creature"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) const ILLICIT_AUCTION_ORACLE: &str = "Each player may bid life for control of target creature. You start the bidding with a bid of 0. In turn order, each player may top the high bid. The bidding ends if the high bid stands. The high bidder loses life equal to the high bid and gains control of the creature. (This effect lasts indefinitely.)";

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn illicit_auction_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(16_449), "Illicit Auction")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::Red],
            vec![ManaSymbol::Red],
        ]))
        .card_types(vec![CardType::Sorcery])
        .parse_text(ILLICIT_AUCTION_ORACLE)
        .expect("Illicit Auction should parse")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) struct ScriptedLifeBids {
    pub(super) bids: Vec<u32>,
}

#[cfg(ironsmith_runtime_parser_tests)]
impl DecisionMaker for ScriptedLifeBids {
    fn decide_boolean(
        &mut self,
        _game: &GameState,
        _ctx: &crate::decisions::context::BooleanContext,
    ) -> bool {
        self.bids.first().is_some_and(|&bid| bid > 0)
    }

    fn decide_number(
        &mut self,
        _game: &GameState,
        _ctx: &crate::decisions::context::NumberContext,
    ) -> u32 {
        if self.bids.is_empty() {
            0
        } else {
            self.bids.remove(0)
        }
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) struct RecordingLifeBids {
    pub(super) responses: std::collections::VecDeque<(PlayerId, Option<u32>)>,
    pub(super) pending_bid: Option<(PlayerId, u32)>,
    pub(super) prompted_players: Vec<PlayerId>,
}

#[cfg(ironsmith_runtime_parser_tests)]
impl RecordingLifeBids {
    pub(super) fn new(responses: Vec<(PlayerId, Option<u32>)>) -> Self {
        Self {
            responses: responses.into(),
            pending_bid: None,
            prompted_players: Vec::new(),
        }
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
impl DecisionMaker for RecordingLifeBids {
    fn decide_boolean(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::BooleanContext,
    ) -> bool {
        let (expected_player, response) = self
            .responses
            .pop_front()
            .expect("expected another life-bid prompt");
        assert_eq!(
            ctx.player, expected_player,
            "life-bid prompt order mismatch"
        );
        self.prompted_players.push(ctx.player);
        if let Some(bid) = response {
            self.pending_bid = Some((ctx.player, bid));
            true
        } else {
            false
        }
    }

    fn decide_number(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::NumberContext,
    ) -> u32 {
        let (expected_player, bid) = self
            .pending_bid
            .take()
            .expect("number prompt should follow a top-bid choice");
        assert_eq!(
            ctx.player, expected_player,
            "life-bid number prompt mismatch"
        );
        bid
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn put_illicit_auction_on_stack(
    game: &mut GameState,
    controller: PlayerId,
    target_creature: ObjectId,
) {
    let definition = illicit_auction_definition();
    let spell_effect = definition
        .spell_effect
        .as_ref()
        .expect("Illicit Auction should have a spell effect");
    let target_requirement = super::targeting::extract_target_requirements(
        game,
        spell_effect.flattened_default_effects(),
        controller,
        None,
    )
    .into_iter()
    .find(|requirement| {
        requirement
            .legal_targets
            .contains(&Target::Object(target_creature))
    })
    .expect("Illicit Auction should target the creature");

    let spell_id = game.create_object_from_definition(&definition, controller, Zone::Stack);
    game.push_to_stack(
        StackEntry::new(spell_id, controller)
            .with_targets(vec![Target::Object(target_creature)])
            .with_target_assignments(vec![crate::game_state::TargetAssignment {
                spec: target_requirement.spec,
                range: 0..1,
            }]),
    );
}

#[derive(Default)]
pub(super) struct NamedCastCostDecisionMaker {
    choices: std::collections::VecDeque<String>,
    pub(super) cost_prompts: Vec<String>,
}

impl NamedCastCostDecisionMaker {
    pub(super) fn new<I, S>(choices: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            choices: choices.into_iter().map(Into::into).collect(),
            cost_prompts: Vec::new(),
        }
    }
}

impl DecisionMaker for NamedCastCostDecisionMaker {
    fn decide_objects(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        let is_cast_card_cost = ctx.description.starts_with("Choose a card to discard")
            || ctx
                .description
                .starts_with("Choose a card to exile from your graveyard");
        if !is_cast_card_cost {
            return crate::decision::AutoPassDecisionMaker.decide_objects(game, ctx);
        }

        self.cost_prompts.push(ctx.description.clone());
        let Some(name) = self.choices.pop_front() else {
            return Vec::new();
        };
        if name == "<cancel>" {
            return Vec::new();
        }

        ctx.candidates
            .iter()
            .find_map(|candidate| {
                (candidate.legal
                    && game
                        .object(candidate.id)
                        .is_some_and(|object| object.name == name.as_str()))
                .then_some(candidate.id)
            })
            .into_iter()
            .collect()
    }
}

fn i061_test_card(
    game: &mut GameState,
    owner: PlayerId,
    name: &'static str,
    zone: Zone,
) -> ObjectId {
    game.create_object_from_card(
        &CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Sorcery])
            .build(),
        owner,
        zone,
    )
}

fn i061_zone_contains_name(game: &GameState, ids: &[ObjectId], name: &str) -> bool {
    ids.iter()
        .any(|&id| game.object(id).is_some_and(|object| object.name == name))
}

#[test]
pub(super) fn native_escape_uses_exact_graveyard_choices_and_rolls_back_partial_payment() {
    fn setup() -> (GameState, PlayerId, ObjectId) {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let spell = CardDefinitionBuilder::new(CardId::new(), "Native Escape Probe")
            .mana_cost(ManaCost::new())
            .card_types(vec![CardType::Sorcery])
            .escape(ManaCost::new(), 2)
            .with_spell_effect(vec![Effect::gain_life(1)])
            .build();
        let spell_id = game.create_object_from_definition(&spell, alice, Zone::Graveyard);
        i061_test_card(&mut game, alice, "Escape Fodder A", Zone::Graveyard);
        i061_test_card(&mut game, alice, "Escape Fodder B", Zone::Graveyard);
        i061_test_card(&mut game, alice, "Escape Fodder C", Zone::Graveyard);
        (game, alice, spell_id)
    }

    let (mut game, alice, spell_id) = setup();
    let mut exact = NamedCastCostDecisionMaker::new(["Escape Fodder C", "Escape Fodder A"]);
    let stack_id = super::cast_spell_from_resolving_effect(
        &mut game,
        spell_id,
        Zone::Graveyard,
        alice,
        &CastingMethod::Alternative(0),
        false,
        None,
        crate::provenance::ProvNodeId::default(),
        &mut exact,
    )
    .expect("native Escape transaction should execute")
    .expect("native Escape should commit after both chosen cards are exiled");

    assert_eq!(exact.cost_prompts.len(), 2);
    assert!(game.stack.iter().any(|entry| {
        entry.object_id == stack_id && entry.casting_method == CastingMethod::Alternative(0)
    }));
    assert!(i061_zone_contains_name(
        &game,
        &game.exile,
        "Escape Fodder A"
    ));
    assert!(i061_zone_contains_name(
        &game,
        &game.exile,
        "Escape Fodder C"
    ));
    assert!(i061_zone_contains_name(
        &game,
        &game.player(alice).expect("Alice exists").graveyard,
        "Escape Fodder B"
    ));
    assert!(
        !i061_zone_contains_name(&game, &game.exile, "Native Escape Probe"),
        "the Escape spell itself must not be offered as an exile-cost card"
    );

    let (mut rollback_game, alice, spell_id) = setup();
    let mut cancel = NamedCastCostDecisionMaker::new(["Escape Fodder B", "<cancel>"]);
    let result = super::cast_spell_from_resolving_effect(
        &mut rollback_game,
        spell_id,
        Zone::Graveyard,
        alice,
        &CastingMethod::Alternative(0),
        false,
        None,
        crate::provenance::ProvNodeId::default(),
        &mut cancel,
    )
    .expect("cancelling a partially paid Escape cost should be a clean cancellation");

    assert!(result.is_none());
    assert!(rollback_game.stack_is_empty());
    assert!(rollback_game.exile.is_empty());
    for name in [
        "Native Escape Probe",
        "Escape Fodder A",
        "Escape Fodder B",
        "Escape Fodder C",
    ] {
        assert!(i061_zone_contains_name(
            &rollback_game,
            &rollback_game.player(alice).expect("Alice exists").graveyard,
            name
        ));
    }
}

#[test]
pub(super) fn jump_start_stages_and_pays_the_exact_chosen_discard() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let spell = CardDefinitionBuilder::new(CardId::new(), "Jump-start Probe")
        .mana_cost(ManaCost::new())
        .card_types(vec![CardType::Sorcery])
        .jump_start()
        .with_spell_effect(vec![Effect::gain_life(1)])
        .build();
    let spell_id = game.create_object_from_definition(&spell, alice, Zone::Graveyard);
    i061_test_card(&mut game, alice, "Unchosen Hand Card", Zone::Hand);
    i061_test_card(&mut game, alice, "Chosen Jump-start Discard", Zone::Hand);

    let mut dm = NamedCastCostDecisionMaker::new(["Chosen Jump-start Discard"]);
    let stack_id = super::cast_spell_from_resolving_effect(
        &mut game,
        spell_id,
        Zone::Graveyard,
        alice,
        &CastingMethod::Alternative(0),
        false,
        None,
        crate::provenance::ProvNodeId::default(),
        &mut dm,
    )
    .expect("Jump-start transaction should execute")
    .expect("Jump-start should commit after the chosen discard is paid");

    assert_eq!(dm.cost_prompts.len(), 1);
    assert!(game.stack.iter().any(|entry| entry.object_id == stack_id));
    assert!(i061_zone_contains_name(
        &game,
        &game.player(alice).expect("Alice exists").graveyard,
        "Chosen Jump-start Discard"
    ));
    assert!(i061_zone_contains_name(
        &game,
        &game.player(alice).expect("Alice exists").hand,
        "Unchosen Hand Card"
    ));
}

#[test]
pub(super) fn granted_escape_uses_the_same_interactive_typed_exile_cost() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let grant_source = i061_test_card(&mut game, alice, "Escape Grant Source", Zone::Battlefield);
    let spell = CardDefinitionBuilder::new(CardId::new(), "Granted Escape Probe")
        .mana_cost(ManaCost::new())
        .card_types(vec![CardType::Sorcery])
        .with_spell_effect(vec![Effect::gain_life(1)])
        .build();
    let spell_id = game.create_object_from_definition(&spell, alice, Zone::Graveyard);
    i061_test_card(&mut game, alice, "Granted Fodder A", Zone::Graveyard);
    i061_test_card(&mut game, alice, "Granted Fodder B", Zone::Graveyard);
    i061_test_card(&mut game, alice, "Granted Fodder C", Zone::Graveyard);

    let casting_method = CastingMethod::GrantedEscape {
        source: grant_source,
        exile_count: 2,
    };
    let mut dm = NamedCastCostDecisionMaker::new(["Granted Fodder C", "Granted Fodder A"]);
    let stack_id = super::cast_spell_from_resolving_effect(
        &mut game,
        spell_id,
        Zone::Graveyard,
        alice,
        &casting_method,
        false,
        None,
        crate::provenance::ProvNodeId::default(),
        &mut dm,
    )
    .expect("granted Escape transaction should execute")
    .expect("granted Escape should commit after interactive exile payment");

    assert_eq!(dm.cost_prompts.len(), 2);
    assert!(
        game.stack
            .iter()
            .any(|entry| { entry.object_id == stack_id && entry.casting_method == casting_method })
    );
    assert!(i061_zone_contains_name(
        &game,
        &game.exile,
        "Granted Fodder A"
    ));
    assert!(i061_zone_contains_name(
        &game,
        &game.exile,
        "Granted Fodder C"
    ));
    assert!(i061_zone_contains_name(
        &game,
        &game.player(alice).expect("Alice exists").graveyard,
        "Granted Fodder B"
    ));
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn jump_start_discard_cost_preserves_discard_replacement_cause_semantics() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let spell = CardDefinitionBuilder::new(CardId::new(), "Jump-start Replacement Probe")
        .mana_cost(ManaCost::new())
        .card_types(vec![CardType::Sorcery])
        .jump_start()
        .with_spell_effect(vec![Effect::gain_life(1)])
        .build();
    let spell_id = game.create_object_from_definition(&spell, alice, Zone::Graveyard);
    let smiter = loxodon_smiter_definition();
    game.create_object_from_definition(&smiter, alice, Zone::Hand);

    let mut dm = NamedCastCostDecisionMaker::new(["Loxodon Smiter"]);
    let result = super::cast_spell_from_resolving_effect(
        &mut game,
        spell_id,
        Zone::Graveyard,
        alice,
        &CastingMethod::Alternative(0),
        false,
        None,
        crate::provenance::ProvNodeId::default(),
        &mut dm,
    )
    .expect("Jump-start replacement scenario should execute");

    assert!(result.is_some());
    assert!(i061_zone_contains_name(
        &game,
        &game.player(alice).expect("Alice exists").graveyard,
        "Loxodon Smiter"
    ));
    assert!(
        !i061_zone_contains_name(&game, &game.battlefield, "Loxodon Smiter"),
        "a discard paid as the caster's own cost must not satisfy Loxodon Smiter's opponent-cause replacement"
    );
}

fn i062_escalate_spell(exact_modes: Option<u32>) -> crate::cards::CardDefinition {
    let modes = vec![
        crate::effect::EffectMode {
            source_text: "Gain 1 life".to_string(),
            effects: vec![Effect::gain_life(1)],
        },
        crate::effect::EffectMode {
            source_text: "Gain 2 life".to_string(),
            effects: vec![Effect::gain_life(2)],
        },
        crate::effect::EffectMode {
            source_text: "Gain 3 life".to_string(),
            effects: vec![Effect::gain_life(3)],
        },
    ];
    let modal = match exact_modes {
        Some(count) => Effect::choose_exactly(count, modes),
        None => Effect::choose_up_to(3, 1, modes),
    };
    let escalate_cost = crate::cost::TotalCost::from_costs(vec![
        crate::costs::Cost::mana(ManaCost::from_symbols(vec![ManaSymbol::Generic(1)])),
        crate::costs::Cost::discard(1, None),
    ]);

    CardDefinitionBuilder::new(CardId::new(), "Typed Escalate Probe")
        .mana_cost(ManaCost::new())
        .card_types(vec![CardType::Sorcery])
        .with_ability(Ability::static_ability(StaticAbility::escalate(
            crate::static_abilities::EscalateSpec {
                cost: escalate_cost,
                cost_surface: Some("{1}, Discard a card".to_string()),
            },
        )))
        .with_spell_effect(vec![modal])
        .build()
}

fn i062_setup(exact_modes: Option<u32>) -> (GameState, PlayerId, ObjectId) {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let spell = i062_escalate_spell(exact_modes);
    let spell_id = game.create_object_from_definition(&spell, alice, Zone::Hand);
    i061_test_card(&mut game, alice, "Escalate Fodder A", Zone::Hand);
    i061_test_card(&mut game, alice, "Escalate Fodder B", Zone::Hand);
    i061_test_card(&mut game, alice, "Escalate Fodder C", Zone::Hand);
    game.player_mut(alice)
        .expect("Alice exists")
        .mana_pool
        .add(ManaSymbol::Red, 2);
    (game, alice, spell_id)
}

#[test]
pub(super) fn escalate_repeats_its_full_typed_cost_and_rolls_back_partial_payment() {
    let (mut game, alice, spell_id) = i062_setup(Some(3));
    let mut dm = NamedCastCostDecisionMaker::new(["Escalate Fodder C", "Escalate Fodder A"]);
    let stack_id = super::cast_spell_from_resolving_effect(
        &mut game,
        spell_id,
        Zone::Hand,
        alice,
        &CastingMethod::Normal,
        false,
        None,
        crate::provenance::ProvNodeId::default(),
        &mut dm,
    )
    .expect("Escalate transaction should execute")
    .expect("three modes should commit after paying Escalate twice");

    assert_eq!(dm.cost_prompts.len(), 2);
    assert_eq!(
        game.player(alice).expect("Alice exists").mana_pool.total(),
        0
    );
    assert!(i061_zone_contains_name(
        &game,
        &game.player(alice).expect("Alice exists").graveyard,
        "Escalate Fodder A"
    ));
    assert!(i061_zone_contains_name(
        &game,
        &game.player(alice).expect("Alice exists").graveyard,
        "Escalate Fodder C"
    ));
    assert!(i061_zone_contains_name(
        &game,
        &game.player(alice).expect("Alice exists").hand,
        "Escalate Fodder B"
    ));
    assert_eq!(
        game.stack
            .iter()
            .find(|entry| entry.object_id == stack_id)
            .and_then(|entry| entry.chosen_modes.as_deref()),
        Some(&[0, 1, 2][..])
    );

    let (mut rollback, alice, spell_id) = i062_setup(Some(3));
    let mut cancel = NamedCastCostDecisionMaker::new(["Escalate Fodder B", "<cancel>"]);
    let result = super::cast_spell_from_resolving_effect(
        &mut rollback,
        spell_id,
        Zone::Hand,
        alice,
        &CastingMethod::Normal,
        false,
        None,
        crate::provenance::ProvNodeId::default(),
        &mut cancel,
    )
    .expect("cancelling repeated Escalate payment should be clean");
    assert!(result.is_none());
    assert!(rollback.stack_is_empty());
    assert_eq!(
        rollback
            .player(alice)
            .expect("Alice exists")
            .mana_pool
            .total(),
        2
    );
    for name in [
        "Typed Escalate Probe",
        "Escalate Fodder A",
        "Escalate Fodder B",
        "Escalate Fodder C",
    ] {
        assert!(i061_zone_contains_name(
            &rollback,
            &rollback.player(alice).expect("Alice exists").hand,
            name
        ));
    }
}

#[test]
pub(super) fn escalate_charges_nothing_when_only_one_mode_is_chosen() {
    let (mut game, alice, spell_id) = i062_setup(None);
    let mut dm = NamedCastCostDecisionMaker::default();
    let stack_id = super::cast_spell_from_resolving_effect(
        &mut game,
        spell_id,
        Zone::Hand,
        alice,
        &CastingMethod::Normal,
        false,
        None,
        crate::provenance::ProvNodeId::default(),
        &mut dm,
    )
    .expect("single-mode Escalate cast should execute")
    .expect("single-mode Escalate cast should commit without its additional cost");

    assert!(dm.cost_prompts.is_empty());
    assert_eq!(
        game.player(alice).expect("Alice exists").mana_pool.total(),
        2
    );
    assert_eq!(
        game.stack
            .iter()
            .find(|entry| entry.object_id == stack_id)
            .and_then(|entry| entry.chosen_modes.as_deref()),
        Some(&[0][..])
    );
}
