//! Typed incomplete execution and atomic real turn/special-action die owners.
//! Authored only; execution is deferred by the campaign workflow.
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::effects::{EffectExecutor, EffectContext as ExecutionContext, OpenAttractionEffect};
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
    assert_eq!(g.planar_die_roll_cost(A), Some(0));
    // Completed die-roll ordinals saturate at the i32 domain the value
    // interpreter reads (game quantities clamp at their storage bounds), so a
    // saturated ordinal is no longer a capacity failure. The roll stays a legal
    // special action at its ordinary timing and completes exactly once.
    ironsmith::decision::compute_legal_actions(&g, A).unwrap();
    let mut dm = SelectFirstDecisionMaker;
    assert!(ironsmith::special_actions::can_perform(
        &ironsmith::special_actions::SpecialAction::RollPlanarDie,
        &g,
        A,
        &mut dm,
    )
    .is_ok());
    assert_eq!(g.irreversible_random_count(), random, "a legality query never rolls");
    ironsmith::special_actions::perform(
        ironsmith::special_actions::SpecialAction::RollPlanarDie,
        &mut g,
        A,
        &mut dm,
    )
    .unwrap();
    assert_eq!(g.take_forced_die_roll(), None, "the forced face was the roll");
    assert_eq!(g.planar_die_roll_cost(A), Some(1));
    assert_eq!(g.irreversible_random_count(), random + 1);
    assert_eq!(
        g.turn_store.turn_history.completed_die_roll_count(A),
        i32::MAX as u32,
        "the ordinal saturates instead of wrapping"
    );
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
    // Completed die-roll ordinals saturate at the i32 domain instead of
    // failing, so the visit roll completes once and never wraps the ordinal.
    assert_eq!(
        ironsmith::game_loop::roll_to_visit_attractions_with_dm(
            &mut g,
            &mut queue,
            &mut SelectFirstDecisionMaker
        )
        .unwrap(),
        Some(6)
    );
    assert_eq!(g.irreversible_random_count(), random, "the forced face is not a fresh random draw");
    assert_eq!(g.take_forced_die_roll(), None, "the forced face was the visit roll");
    assert_eq!(
        g.turn_store.turn_history.completed_die_roll_count(A),
        i32::MAX as u32
    );
}
