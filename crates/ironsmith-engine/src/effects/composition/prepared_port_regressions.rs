//! Authored, unrun regressions for reviewed prepared-program integration seams.
use crate::effect::{Effect, EffectOutcome, Value};
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError, ForPlayersEffect, MayEffect, RepeatEffectsEffect};
use crate::game_state::GameState;
use crate::ids::PlayerId;
use crate::target::PlayerFilter;
use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};

#[derive(Debug, Clone)]
struct ReachedProbe { preflights: Arc<AtomicUsize>, selections: Arc<AtomicUsize> }
impl EffectExecutor for ReachedProbe {
    fn directly_mentions_player_filter(&self, _needle: &PlayerFilter) -> bool {
        self.preflights.fetch_add(1, Ordering::SeqCst);
        false
    }
    fn supports_prepared_action_program(&self) -> bool { true }
    fn select_prepared_action_program(&self, _game: &mut GameState, _ctx: &mut ExecutionContext)
        -> Result<Option<Box<dyn crate::effects::ActionProgramCursor>>, ExecutionError> {
        let selected = self.selections.fetch_add(1, Ordering::SeqCst) + 1;
        assert!(self.preflights.load(Ordering::SeqCst) >= selected, "selection bypassed reached-input acquisition");
        Ok(Some(super::action_program::finished_program_cursor(
            crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)))))
    }
    fn execute(&self, _game: &mut GameState, _ctx: &mut ExecutionContext) -> Result<EffectOutcome, ExecutionError> {
        panic!("selected program must use its cursor")
    }
}
#[test]
fn nested_selected_programs_acquire_reached_inputs_and_do_not_enter_declined_children() {
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let source = game.new_object_id();
    let preflights = Arc::new(AtomicUsize::new(0));
    let selections = Arc::new(AtomicUsize::new(0));
    let probe = Effect::new(ReachedProbe { preflights: preflights.clone(), selections: selections.clone() });
    let nested = Effect::new(ForPlayersEffect::new(PlayerFilter::Any, vec![
        Effect::new(RepeatEffectsEffect::new(Value::Fixed(1), vec![probe.clone()])),
    ]));
    crate::effects::execute_effect(&mut game, &nested, &mut ExecutionContext::new_default(source, alice)).unwrap();
    assert_eq!(selections.load(Ordering::SeqCst), 2);
    struct Decline;
    impl crate::decision::DecisionMaker for Decline {
        fn decide_boolean(&mut self, _: &GameState, _: &crate::decisions::context::BooleanContext) -> bool { false }
    }
    let before = preflights.load(Ordering::SeqCst);
    let mut dm = Decline;
    crate::effects::execute_effect(&mut game, &Effect::new(MayEffect::new(vec![probe])),
        &mut ExecutionContext::new(source, alice, &mut dm)).unwrap();
    assert_eq!(preflights.load(Ordering::SeqCst), before);
}
#[test]
fn unpaid_optional_cost_restores_offer_limit_but_paid_cost_consumes_it() {
    for amount in [1, 100] {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let identity = crate::triggers::TriggerIdentity(71);
        let limit = crate::effects::DoThisLimit { source, trigger_identity: identity, limit: 1 };
        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.do_this_limit = Some(limit);
        let effect = MayEffect::new(vec![Effect::pay_life(amount)]).with_pay_as_cost(true);
        let outcome = effect.execute(&mut game, &mut ctx).unwrap();
        assert_eq!(outcome.status.is_success(), amount == 1);
        assert_eq!(game.do_this_action_count_this_turn(source, identity), u32::from(amount == 1));
        assert_eq!(game.player(alice).unwrap().life, if amount == 1 { 19 } else { 20 });
        if amount == 100 { assert_eq!(ctx.do_this_limit, Some(limit)); }
    }
}
#[derive(Debug, Clone)]
struct ObservePaidOriginals;
impl EffectExecutor for ObservePaidOriginals {
    fn execute(&self, game: &mut GameState, _: &mut ExecutionContext) -> Result<EffectOutcome, ExecutionError> {
        assert!(game.players.iter().all(|player| player.life == 19), "addition overtook another payment original");
        Ok(EffectOutcome::resolved())
    }
}
#[test]
fn optional_total_cost_choices_and_originals_precede_every_added_program() {
    use crate::replacement::{ReplacementAction, ReplacementEffect};
    struct AcceptBeforeMutation;
    impl crate::decision::DecisionMaker for AcceptBeforeMutation {
        fn decide_boolean(&mut self, game: &GameState, _: &crate::decisions::context::BooleanContext) -> bool {
            assert!(game.players.iter().all(|player| player.life == 20), "payment offer was deferred to commitment");
            true
        }
    }
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let source = game.new_object_id();
    game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(
        source, alice, crate::events::life::matchers::WouldLoseLifeMatcher::new(PlayerFilter::Specific(alice)),
        ReplacementAction::Additionally(vec![Effect::new(ObservePaidOriginals)]),
    ));
    let optional = MayEffect::new_for_player(vec![Effect::pay_life(1)], PlayerFilter::IteratedPlayer).with_pay_as_cost(true);
    assert!(optional.supports_simultaneous_player_action());
    let mut dm = AcceptBeforeMutation;
    ForPlayersEffect::new(PlayerFilter::Any, vec![Effect::new(optional)])
        .execute(&mut game, &mut ExecutionContext::new(source, alice, &mut dm)).unwrap();
    assert!(game.players.iter().all(|player| player.life == 19));
}
