//! Typed incomplete execution and atomic real turn/special-action die owners.
//! Authored only; execution is deferred by the campaign workflow.
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::effects::{EffectExecutor, EffectContext as ExecutionContext, ExecutionError, OpenAttractionEffect};
use ironsmith::triggers::TriggerQueue;
use ironsmith::{
     CardId, CardType, Effect, GameState, PlanarCardKind, PlayerId,
};
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
fn game() -> GameState {
    GameState::new(vec!["A".into(), "B".into()], 30)
}
fn planar() -> GameState {
    let mut g = game();
    let deck = || {
        (0..10)
            .map(|i| {
                (
                    CardDefinitionBuilder::new(CardId::new(), format!("Plane {i}"))
                        .card_types(vec![CardType::Plane])
                        .build(),
                    PlanarCardKind::Plane,
                )
            })
            .collect()
    };
    g.enable_planechase(vec![(A, deck()), (B, deck())]).unwrap();
    g.reveal_starting_plane().unwrap();
    g.turn.phase = ironsmith::Phase::FirstMain;
    g.turn.step = None;
    g.turn.priority_player = Some(A);
    g
}
#[test]
fn planar_capacity_failure_is_atomic_typed_and_never_invalid_timing() {
    let mut g = planar();
    g.turn_store
        .turn_history
        .completed_die_rolls_this_turn
        .insert(A, i32::MAX as u32);
    g.force_next_die_roll(6);
    let random = g.irreversible_random_count();
    let cost = g.planar_die_roll_cost(A);
    assert!(matches!(
        g.roll_planar_die(A, true),
        Err(ExecutionError::ResourceLimitExceeded { .. })
    ));
    assert_eq!(g.irreversible_random_count(), random);
    assert_eq!(g.planar_die_roll_cost(A), cost);
    let error = ironsmith::decision::compute_legal_actions(&g, A).unwrap_err();
    assert!(matches!(
        error,
        ExecutionError::ResourceLimitExceeded { .. }
    ));
    let mut dm = SelectFirstDecisionMaker;
    for result in [
        ironsmith::special_actions::can_perform(
            &ironsmith::special_actions::SpecialAction::RollPlanarDie,
            &g,
            A,
            &mut dm,
        ),
        ironsmith::special_actions::perform(
            ironsmith::special_actions::SpecialAction::RollPlanarDie,
            &mut g,
            A,
            &mut dm,
        ),
    ] {
        assert!(matches!(
            result,
            Err(ironsmith::special_actions::ActionError::ExecutionFailure {
                error: ExecutionError::ResourceLimitExceeded { .. },
                ..
            })
        ));
    }
    assert_eq!(g.irreversible_random_count(), random);
    assert_eq!(g.planar_die_roll_cost(A), cost);
    g.turn_store
        .turn_history
        .completed_die_rolls_this_turn
        .insert(A, 0);
    assert_eq!(
        g.roll_planar_die(A, true).unwrap(),
        ironsmith::PlanarDieFace::Blank,
        "preflight did not consume the forced result"
    );
    assert_eq!(g.planar_die_roll_cost(A), Some(1));
    assert_eq!(g.irreversible_random_count(), random + 1);
}
#[test]
fn attraction_capacity_failure_preserves_typed_error_queue_and_random_attempt() {
    let mut g = game();
    let d = CardDefinitionBuilder::new(CardId::new(), "Attraction fixture")
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![ironsmith::types::Subtype::Attraction])
        .attraction_lights(vec![6])
        .with_spell_effect(vec![Effect::gain_life(1)])
        .build();
    g.enable_attractions(vec![(
        A,
        ironsmith::game_state::AttractionDeckFormat::Limited,
        vec![d.clone(), d.clone(), d],
    )])
    .unwrap();
    let source = g.new_object_id();
    OpenAttractionEffect::new()
        .execute(&mut g, &mut ExecutionContext::new_default(source, A))
        .unwrap();
    g.turn_store
        .turn_history
        .completed_die_rolls_this_turn
        .insert(A, i32::MAX as u32);
    g.force_next_die_roll(6);
    let random = g.irreversible_random_count();
    let mut queue = TriggerQueue::new();
    let error = ironsmith::game_loop::roll_to_visit_attractions_with_dm(
        &mut g,
        &mut queue,
        &mut SelectFirstDecisionMaker,
    )
    .unwrap_err();
    assert!(matches!(
        error,
        ironsmith::game_loop::GameLoopError::ExecutionFailed(
            ExecutionError::ResourceLimitExceeded { .. }
        )
    ));
    assert_eq!(g.irreversible_random_count(), random);
    assert!(queue.is_empty());
    g.turn_store
        .turn_history
        .completed_die_rolls_this_turn
        .insert(A, 0);
    assert_eq!(
        ironsmith::game_loop::roll_to_visit_attractions_with_dm(
            &mut g,
            &mut queue,
            &mut SelectFirstDecisionMaker
        )
        .unwrap(),
        Some(6)
    );
    assert_eq!(g.turn_store.turn_history.completed_die_roll_count(A), 1);
}
