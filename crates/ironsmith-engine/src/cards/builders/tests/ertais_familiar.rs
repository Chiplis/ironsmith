#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;
use crate::decision::SelectFirstDecisionMaker;
use crate::effects::ExecutionContext;
use crate::mana::ManaSymbol;
use crate::snapshot::ObjectSnapshot;

const COMPILED_TEXT: &str = "Phasing\nWhen this phases out or this leaves the battlefield, mill three cards.\n{U}: This creature can't phase out until your next upkeep.";

fn activated_ability(definition: &CardDefinition) -> &crate::ability::ActivatedAbility {
    definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("Ertai's Familiar should have its blue activated ability")
}

fn library_card() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Mill Fodder")
        .card_types(vec![CardType::Sorcery])
        .build()
}

fn fill_library(game: &mut crate::GameState, player: PlayerId, count: usize) {
    let card = library_card();
    for _ in 0..count {
        game.create_object_from_definition(&card, player, Zone::Library);
    }
}

fn resolve_pending_triggers_for_source(game: &mut crate::GameState, source: ObjectId) -> usize {
    let mut pending = crate::triggers::TriggerQueue::new();
    crate::game_loop::drain_pending_trigger_events(game, &mut pending);
    let matching = pending
        .entries
        .into_iter()
        .filter(|entry| entry.source == source)
        .collect::<Vec<_>>();
    let count = matching.len();
    let mut queue = crate::triggers::TriggerQueue::new();
    for trigger in matching {
        queue.add(trigger);
    }
    if count > 0 {
        crate::game_loop::put_triggers_on_stack(game, &mut queue)
            .expect("Ertai's Familiar trigger should go on the stack");
        while !game.stack.is_empty() {
            crate::game_loop::resolve_stack_entry(game)
                .expect("Ertai's Familiar trigger should resolve");
        }
    }
    count
}

fn activate_prevention(
    game: &mut crate::GameState,
    source: ObjectId,
    controller: PlayerId,
    activated: &crate::ability::ActivatedAbility,
) {
    game.player_mut(controller)
        .expect("controller exists")
        .mana_pool
        .add(ManaSymbol::Blue, 1);
    let source_snapshot =
        ObjectSnapshot::from_object(game.object(source).expect("Ertai's Familiar exists"), game);
    let mut decisions = SelectFirstDecisionMaker;
    let mut ctx = ExecutionContext::new(source, controller, &mut decisions)
        .with_source_snapshot(source_snapshot);
    crate::special_actions::pay_total_cost_with_choice_in_context(
        game,
        controller,
        source,
        &activated.mana_cost,
        crate::costs::PaymentReason::ActivateAbility,
        &mut ctx,
    )
    .expect("{U} should pay for Ertai's Familiar's activation");
    crate::game_loop::execute_resolution_program(
        game,
        &mut ctx,
        controller,
        source,
        &activated.effects,
        None,
        &[],
    )
    .expect("Ertai's Familiar's prevention ability should resolve");
}

#[test]
fn ertais_familiar_named_definition_keeps_both_trigger_branches_and_phase_out_restriction() {
    let definition = parse_oracle_card_definition("Ertai's Familiar");
    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        COMPILED_TEXT
    );
    let debug = format!("{:#?}", definition.abilities);
    assert!(debug.contains("Phasing"), "{debug}");
    assert!(debug.contains("AnyOfTrigger"), "{debug}");
    assert!(debug.contains("PermanentPhasedOut"), "{debug}");
    assert!(debug.contains("ZoneChangeTrigger"), "{debug}");
    assert!(debug.contains("MillEffect"), "{debug}");
    assert!(debug.contains("PhaseOut"), "{debug}");
    assert!(debug.contains("YourNextUpkeep"), "{debug}");
}

#[test]
fn ertais_familiar_phasing_and_leaving_each_mill_three_exactly_once() {
    let definition = parse_oracle_card_definition("Ertai's Familiar");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    fill_library(&mut game, alice, 8);

    let first = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    game.phase_out(first);
    assert!(game.is_phased_out(first));
    assert_eq!(resolve_pending_triggers_for_source(&mut game, first), 1);
    assert_eq!(game.player(alice).expect("Alice").graveyard.len(), 3);

    let second = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    game.move_object_by_effect(second, Zone::Graveyard)
        .expect("the second Familiar should leave the battlefield");
    assert_eq!(resolve_pending_triggers_for_source(&mut game, second), 1);
    assert_eq!(
        game.player(alice).expect("Alice").graveyard.len(),
        7,
        "three milled cards plus the departed Familiar should be in Alice's graveyard"
    );
}

#[test]
fn ertais_familiar_activation_prevents_turn_based_phasing_until_next_upkeep() {
    let definition = parse_oracle_card_definition("Ertai's Familiar");
    let activated = activated_ability(&definition);
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let familiar = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    fill_library(&mut game, alice, 4);

    game.turn.active_player = bob;
    game.turn.priority_player = Some(bob);
    activate_prevention(&mut game, familiar, alice, activated);
    assert_eq!(game.player(alice).expect("Alice").mana_pool.total(), 0);
    assert!(!game.can_phase_out(familiar));

    game.next_turn();
    assert_eq!(game.turn.active_player, alice);
    game.turn.phase = crate::game_state::Phase::Beginning;
    game.turn.step = Some(crate::game_state::Step::Untap);
    crate::turn::execute_untap_step(&mut game);

    assert!(
        !game.is_phased_out(familiar),
        "the turn-based phasing action must honor the active can't-phase-out restriction"
    );
    assert_eq!(resolve_pending_triggers_for_source(&mut game, familiar), 0);
    assert_eq!(game.player(alice).expect("Alice").graveyard.len(), 0);

    game.turn.step = Some(crate::game_state::Step::Upkeep);
    game.update_cant_effects();
    assert!(
        game.can_phase_out(familiar),
        "the restriction should expire as Alice's next upkeep begins"
    );
}
