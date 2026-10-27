use super::*;
use crate::cards::builders::CardDefinitionBuilder;
use crate::effects::{CopySpellEffect, EffectExecutor, ExecutionContext};
use crate::events::{EventKind, MonarchChangedEvent};
use crate::triggers::TriggerEvent;

fn setup() -> (GameState, ObjectId) {
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    let definition = CardDefinitionBuilder::new(CardId::new(), "Duration source")
        .card_types(vec![crate::types::CardType::Artifact])
        .build();
    let source = game.create_object_from_definition(&definition, PlayerId(0), Zone::Battlefield);
    (game, source)
}
fn event(game: &mut GameState, monarch: PlayerId) -> TriggerEvent {
    let provenance = game
        .provenance_graph_mut()
        .alloc_root_event(EventKind::MonarchChanged);
    TriggerEvent::new_with_provenance(
        MonarchChangedEvent {
            previous: None,
            monarch,
        },
        provenance,
    )
}
fn records_monarch(event: &TriggerEvent) -> bool {
    event.downcast::<MonarchChangedEvent>().is_some()
}

#[test]
fn completion_order_determines_a_duration_window_even_when_provenance_was_allocated_earlier() {
    let (mut game, _) = setup();
    let earlier = event(&mut game, PlayerId(1));
    let completed = event(&mut game, PlayerId(0));
    game.record_turn_history_event(&completed);
    let start = game.duration_window_now();
    assert!(
        !game
            .duration_event_occurred(Some(start), records_monarch)
            .unwrap()
    );
    game.record_turn_history_event(&earlier);
    assert!(earlier.provenance().raw() < completed.provenance().raw());
    assert!(
        game.duration_event_occurred(Some(start), records_monarch)
            .unwrap()
    );
}

#[test]
fn the_triggering_event_is_excluded_but_a_later_occurrence_ends_the_window() {
    let (mut game, source) = setup();
    let triggering = event(&mut game, PlayerId(1));
    game.record_turn_history_event(&triggering);
    let annotated = game.provenance_graph_mut().alloc_child(
        triggering.provenance(),
        crate::provenance::ProvenanceNodeKind::TriggerMatched {
            source,
            controller: PlayerId(0),
        },
    );
    let triggering = triggering.with_provenance(annotated);
    let entry = StackEntry::ability(
        source,
        PlayerId(0),
        vec![crate::effect::Effect::gain_life(1)],
    )
    .with_triggering_event(triggering);
    let start = game
        .duration_window_for_stack_entry(&entry)
        .expect("logged native trigger owns a window");
    assert!(
        !game
            .duration_event_occurred(Some(start), records_monarch)
            .unwrap()
    );
    let later = event(&mut game, PlayerId(0));
    game.record_turn_history_event(&later);
    assert!(
        game.duration_event_occurred(Some(start), records_monarch)
            .unwrap()
    );
}

#[test]
fn copying_an_ability_starts_a_new_window_after_events_that_ended_the_original() {
    let (mut game, source) = setup();
    let triggering = event(&mut game, PlayerId(0));
    game.record_turn_history_event(&triggering);
    let mut original = StackEntry::ability(
        source,
        PlayerId(0),
        vec![crate::effect::Effect::gain_life(1)],
    )
    .with_triggering_event(triggering);
    original.ability_id = Some(game.allocate_stack_ability_id());
    let target = original.target_id();
    let original_start = game.duration_window_for_stack_entry(&original).unwrap();
    game.stack.push(original);
    let ending = event(&mut game, PlayerId(1));
    game.record_turn_history_event(&ending);
    assert!(
        game.duration_event_occurred(Some(original_start), records_monarch)
            .unwrap()
    );
    CopySpellEffect::single(crate::target::ChooseSpec::SpecificObject(target))
        .execute(
            &mut game,
            &mut ExecutionContext::new_default(source, PlayerId(0)),
        )
        .unwrap();
    assert_eq!(game.stack.len(), 2);
    let copy = game.stack.last().unwrap();
    assert_ne!(copy.object_id, source);
    let copy_start = game
        .duration_window_for_stack_entry(copy)
        .expect("native copy insertion owns a new window");
    assert!(
        !game
            .duration_event_occurred(Some(copy_start), records_monarch)
            .unwrap()
    );
    let later = event(&mut game, PlayerId(0));
    game.record_turn_history_event(&later);
    assert!(
        game.duration_event_occurred(Some(copy_start), records_monarch)
            .unwrap()
    );
}

#[test]
fn an_unavailable_history_prefix_is_rejected_and_native_checkpoints_retain_the_window() {
    let (mut game, source) = setup();
    let checkpoint = game.clone();
    let completed = event(&mut game, PlayerId(1));
    game.record_turn_history_event(&completed);
    let start = game.duration_window_now();
    assert!(matches!(
        checkpoint.duration_event_occurred(Some(start), records_monarch),
        Err(crate::effects::ExecutionError::IncompleteEvidence(_))
    ));
    let mut ctx = ExecutionContext::new_default(source, PlayerId(0));
    ctx.duration_window_start = Some(start);
    let saved = crate::effects::ExecutionContextCheckpoint::capture(&ctx);
    ctx.duration_window_start = None;
    saved.restore(&mut ctx);
    let later = event(&mut game, PlayerId(0));
    game.record_turn_history_event(&later);
    assert!(
        game.duration_event_occurred(ctx.duration_window_start, records_monarch)
            .unwrap()
    );
}

#[test]
fn a_queued_annotation_can_itself_own_the_completed_observation() {
    let (mut game, source) = setup();
    let original = event(&mut game, PlayerId(1));
    let queued = game.provenance_graph_mut().alloc_child(
        original.provenance(),
        crate::provenance::ProvenanceNodeKind::TriggerQueued,
    );
    let observation = original.with_provenance(queued);
    game.record_turn_history_event(&observation);
    let matched = game.provenance_graph_mut().alloc_child(
        queued,
        crate::provenance::ProvenanceNodeKind::TriggerMatched {
            source,
            controller: PlayerId(0),
        },
    );
    let entry = StackEntry::ability(
        source,
        PlayerId(0),
        vec![crate::effect::Effect::gain_life(1)],
    )
    .with_triggering_event(observation.with_provenance(matched));
    let start = game
        .duration_window_for_stack_entry(&entry)
        .expect("queued observation is the exact trigger history owner");
    assert!(
        !game
            .duration_event_occurred(Some(start), records_monarch)
            .unwrap()
    );
    let later = event(&mut game, PlayerId(0));
    game.record_turn_history_event(&later);
    assert!(
        game.duration_event_occurred(Some(start), records_monarch)
            .unwrap()
    );
}
