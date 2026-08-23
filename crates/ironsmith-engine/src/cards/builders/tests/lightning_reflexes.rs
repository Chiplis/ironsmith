#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;
use crate::cost::OptionalCostsPaid;
use crate::decision::SelectFirstDecisionMaker;

const CAST_TIMING_LINE: &str = "You may cast this spell as though it had flash. If you cast it any time a sorcery couldn't have been cast, the controller of the permanent it becomes sacrifices it at the beginning of the next cleanup step.";

fn resolve_as_permanent(
    game: &mut crate::game_state::GameState,
    definition: &crate::cards::CardDefinition,
    controller: PlayerId,
    cast_at_sorcery_timing: bool,
) -> ObjectId {
    let spell = game.create_object_from_definition(definition, controller, Zone::Stack);
    let mut paid = OptionalCostsPaid::default();
    if cast_at_sorcery_timing {
        paid.mark_cast_at_sorcery_timing();
    }
    game.object_mut(spell)
        .expect("Lightning Reflexes spell should exist")
        .optional_costs_paid = paid.clone();
    game.record_turn_history_event(&crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::spells::SpellCastEvent::new(spell, controller, Zone::Hand),
        crate::provenance::ProvNodeId::default(),
    ));
    let program = definition
        .spell_effect
        .as_ref()
        .expect("Lightning Reflexes should have a conditional spell program");
    let timing_segment = program
        .segments
        .iter()
        .find(|segment| {
            segment
                .default_effects
                .iter()
                .any(|effect| effect.downcast_ref::<ConditionalEffect>().is_some())
        })
        .expect("Lightning Reflexes should retain its cast-timing conditional")
        .clone();
    // Resolve the parsed timing instruction in isolation. The other spell
    // segment is the Aura attachment instruction and is exercised by the
    // engine's targeting tests; it would require an unrelated creature target
    // in this timing-focused scenario.
    let timing_program = crate::resolution::ResolutionProgram::new(vec![timing_segment]);
    let mut decisions = SelectFirstDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new(spell, controller, &mut decisions)
        .with_optional_costs_paid(paid);
    crate::game_loop::execute_resolution_program(
        game,
        &mut ctx,
        controller,
        spell,
        &timing_program,
        None,
        &[],
    )
    .expect("Lightning Reflexes spell program should resolve");
    game.move_object_by_effect(spell, Zone::Battlefield)
        .expect("Lightning Reflexes should become a permanent")
}

fn fire_cleanup(game: &mut crate::game_state::GameState, player: PlayerId) {
    let event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::phase::BeginningOfCleanupStepEvent::new(player),
        crate::provenance::ProvNodeId::default(),
    );
    let mut queue = crate::triggers::TriggerQueue::new();
    for trigger in crate::triggers::check_delayed_triggers(game, &event) {
        queue.add(trigger);
    }
    crate::game_loop::put_triggers_on_stack(game, &mut queue)
        .expect("cleanup trigger should go on the stack");
    while !game.stack_is_empty() {
        crate::game_loop::resolve_stack_entry(game).expect("cleanup trigger should resolve");
    }
}

#[test]
fn lightning_reflexes_uses_cast_timing_and_the_next_cleanup_step() {
    let definition = parse_oracle_card_definition("Lightning Reflexes");
    assert!(
        definition.abilities.iter().any(|ability| {
            matches!(&ability.kind, AbilityKind::Static(static_ability)
                if static_ability.id() == StaticAbilityId::Flash)
        }),
        "the authored permission must grant flash"
    );

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let mut on_time_game = crate::tests::test_helpers::setup_two_player_game();
    let on_time = resolve_as_permanent(&mut on_time_game, &definition, alice, true);
    assert!(on_time_game.effect_store.delayed_triggers.is_empty());
    assert_eq!(
        on_time_game.object(on_time).map(|object| object.zone),
        Some(Zone::Battlefield)
    );

    let mut off_time_game = crate::tests::test_helpers::setup_two_player_game();
    let off_time = resolve_as_permanent(&mut off_time_game, &definition, alice, false);
    assert_eq!(off_time_game.effect_store.delayed_triggers.len(), 1);
    let stable_id = off_time_game
        .object(off_time)
        .expect("off-time Aura should exist")
        .stable_id;

    let end_step = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::phase::BeginningOfEndStepEvent::new(alice),
        crate::provenance::ProvNodeId::default(),
    );
    assert!(crate::triggers::check_delayed_triggers(&mut off_time_game, &end_step).is_empty());

    off_time_game.set_current_controller(off_time, bob);
    fire_cleanup(&mut off_time_game, alice);
    let current = off_time_game
        .find_object_by_stable_id(stable_id)
        .unwrap_or(off_time);
    assert_eq!(
        off_time_game.object(current).map(|object| object.zone),
        Some(Zone::Graveyard),
        "the resulting Aura's current controller must sacrifice it at the next cleanup step"
    );

    let rendered = canonical_compiled_lines(&definition);
    assert!(
        rendered.iter().any(|line| line == CAST_TIMING_LINE),
        "Lightning Reflexes must retain the cast-timing consequence: {rendered:#?}"
    );
}
