//! Authored source regressions for the complete selected resolution owner.
//! These scenarios are intentionally unrun during the main/campaign port.
use crate::effect::{Effect, EffectOutcome, Value};
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError, ForPlayersEffect, RepeatEffectsEffect, SequenceEffect};
use crate::game_state::GameState;
use crate::ids::PlayerId;
use crate::target::{ChooseSpec, PlayerFilter};
use crate::zone::Zone;

// Like a resolving coin choice, this read-only scheduling leaf can terminate
// its enclosing resolution. The life mark makes repeated/late actors visible.
#[derive(Debug, Clone)]
struct MarkActor { stop: bool }
impl EffectExecutor for MarkActor {
    fn is_read_only_simultaneous_player_action(&self) -> bool { true }
    fn execute(&self, game: &mut GameState, ctx: &mut ExecutionContext) -> Result<EffectOutcome, ExecutionError> {
        let actor = ctx.iteration.iterated_player.expect("selected participant");
        game.player_mut(actor).unwrap().life += 1;
        if self.stop { ctx.stop_resolution(); }
        Ok(EffectOutcome::count(1))
    }
}
fn body(stop: bool, prefix: bool) -> Effect {
    let mut effects = Vec::new();
    if prefix {
        effects.push(Effect::gain_life_player(1, ChooseSpec::Player(PlayerFilter::IteratedPlayer)));
    }
    effects.push(Effect::new(MarkActor { stop }));
    effects.push(Effect::gain_life_player(10, ChooseSpec::Player(PlayerFilter::IteratedPlayer)));
    Effect::new(SequenceEffect::new(vec![
        Effect::new(ForPlayersEffect::new(PlayerFilter::Any, vec![
            Effect::new(RepeatEffectsEffect::new(Value::Fixed(1), effects)),
        ])),
        Effect::gain_life(100),
    ]))
}
#[test]
fn selected_repeat_stop_cancels_later_participants_and_enclosing_suffix() {
    for stop in [false, true] {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);
        crate::effects::execute_effect(&mut game, &body(stop, false), &mut ctx).unwrap();
        assert_eq!(ctx.resolution_stopped(), stop);
        assert_eq!(game.player(alice).unwrap().life, if stop { 21 } else { 131 });
        assert_eq!(game.player(bob).unwrap().life, if stop { 20 } else { 31 });
    }
}
#[test]
fn resumed_selected_program_stop_matches_direct_resolution_without_replaying_originals() {
    use crate::replacement::{ReplacementAction, ReplacementEffect};
    for stop in [false, true] {
        for deferred in [false, true] {
            let mut game = crate::tests::test_helpers::setup_two_player_game();
            let alice = PlayerId::from_index(0);
            let bob = PlayerId::from_index(1);
            let definition = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Retained draw").build();
            let source = game.create_object_from_card(&definition, alice, Zone::Battlefield);
            game.create_object_from_card(&definition, alice, Zone::Library);
            game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(
                source, alice, crate::events::life::matchers::WouldGainLifeMatcher::new(PlayerFilter::Specific(alice)),
                ReplacementAction::Additionally(vec![Effect::draw(1)]),
            ));
            let effect = body(stop, true);
            let mut ctx = ExecutionContext::new_default(source, alice);
            let outcome = if deferred {
                let receipt = crate::effects::runtime::prepare_effect_draw_continuation_with_outputs(&mut game, &effect, &mut ctx).unwrap();
                assert!(receipt.completion.is_some());
                assert_eq!(game.player(alice).unwrap().life, 21);
                assert_eq!(game.player(bob).unwrap().life, 21);
                assert!(game.player(alice).unwrap().hand.is_empty());
                crate::effects::composition::complete_standalone_original_with_outputs(&mut game, &mut ctx, receipt).unwrap().into_outcome()
            } else {
                crate::effects::execute_effect(&mut game, &effect, &mut ctx).unwrap()
            };
            assert_eq!(ctx.resolution_stopped(), stop);
            assert_eq!(game.player(alice).unwrap().life, if stop { 22 } else { 132 });
            assert_eq!(game.player(bob).unwrap().life, if stop { 21 } else { 32 });
            assert_eq!(game.player(alice).unwrap().hand.len(), 1);
            assert_eq!(outcome.events_of_type::<crate::events::LifeGainEvent>().count(), if stop { 2 } else { 5 });
        }
    }
}

#[test]
fn scoped_original_completion_preserves_successful_stop_but_failed_attempt_restores_it() {
    struct StopsOnCompletion { fail: bool }
    impl crate::effects::SimultaneousEffectCompletion for StopsOnCompletion {
        fn freeze(&mut self, _: &mut GameState) -> Result<(), ExecutionError> { Ok(()) }
        fn complete(self: Box<Self>, _: &mut GameState, ctx: &mut ExecutionContext,
            original: EffectOutcome) -> Result<EffectOutcome, ExecutionError> {
            ctx.stop_resolution();
            if self.fail { Err(ExecutionError::IncompleteEvidence("completion fixture".into())) } else { Ok(original) }
        }
    }
    for fail in [false, true] {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);
        let result = super::execute_transaction(&mut game, &mut ctx,
            || crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            |game, ctx| {
                let receipt = super::with_original_execution_context(crate::effects::SimultaneousEffectCommit {
                    outcome: EffectOutcome::count(7), completion: Some(Box::new(StopsOnCompletion { fail })),
                }, ctx);
                super::complete_standalone_original_with_outputs(game, ctx, receipt)
            });
        assert_eq!(result.is_err(), fail);
        assert_eq!(ctx.resolution_stopped(), !fail);
        if !fail {
            crate::effects::execute_effect(&mut game, &Effect::gain_life(100), &mut ctx).unwrap();
            assert_eq!(game.player(alice).unwrap().life, 20);
        }
    }
}

#[test]
fn stopped_modal_keeps_its_chosen_option_and_actual_prefix_in_both_owners() {
    for per_player in [false, true] {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();
        let modal = Effect::new(crate::effects::ChooseModeEffect::choose_one(vec![
            crate::effect::EffectMode::new("Selected stopping mode", vec![
                Effect::new(MarkActor { stop: true }), Effect::gain_life(10),
            ]),
        ]));
        let selected = if per_player {
            Effect::new(ForPlayersEffect::new(PlayerFilter::Any, vec![
                Effect::new(RepeatEffectsEffect::new(1, vec![modal])),
            ]))
        } else { modal };
        let effect = Effect::new(SequenceEffect::new(vec![selected, Effect::gain_life(100)]));
        let mut ctx = ExecutionContext::new_default(source, alice).with_chosen_modes(Some(vec![0]));
        ctx.iteration.iterated_player = Some(alice);
        let outcome = crate::effects::execute_effect(&mut game, &effect, &mut ctx).unwrap();
        assert!(ctx.resolution_stopped());
        assert_eq!(game.player(alice).unwrap().life, 21);
        assert_eq!(game.player(bob).unwrap().life, 20);
        assert_eq!(outcome.execution_facts.iter().filter(|fact|
            matches!(fact, crate::effect::ExecutionFact::ChosenOptions(indices) if indices == &vec![0])).count(), 1);
        assert_eq!(game.resolving_mode_context(source), None);
    }
}

#[test]
fn completion_prefix_stop_keeps_every_original_and_skips_later_prefixes() {
    #[derive(Debug, Clone)] struct PrefixStop { stop: bool }
    #[derive(Debug)] struct Original { actor: PlayerId, stop: bool }
    struct Completion { actor: PlayerId, stop: bool }
    impl crate::effects::SimultaneousEffectCompletion for Completion {
        fn freeze(&mut self, _: &mut GameState) -> Result<(), ExecutionError> { Ok(()) }
        fn complete(self: Box<Self>, game: &mut GameState, ctx: &mut ExecutionContext,
            original: EffectOutcome) -> Result<EffectOutcome, ExecutionError> {
            assert!(game.players.iter().all(|player| player.life >= 30), "prefix preceded another original");
            game.player_mut(self.actor).unwrap().life += 1;
            if self.stop { ctx.stop_resolution(); }
            Ok(original)
        }
    }
    impl crate::effects::SimultaneousEffectProposal for Original {
        fn commit_original(self: Box<Self>, game: &mut GameState, _: &mut ExecutionContext)
            -> Result<crate::effects::SimultaneousEffectCommit, ExecutionError> {
            game.player_mut(self.actor).unwrap().life += 10;
            Ok(crate::effects::SimultaneousEffectCommit {
                outcome: EffectOutcome::count(10), completion: Some(Box::new(Completion { actor: self.actor, stop: self.stop })),
            })
        }
        fn commit(self: Box<Self>, game: &mut GameState, ctx: &mut ExecutionContext) -> Result<EffectOutcome, ExecutionError> {
            super::complete_prepared_original(self, game, ctx)
        }
    }
    impl EffectExecutor for PrefixStop {
        fn supports_simultaneous_player_action(&self) -> bool { true }
        fn prepare_simultaneous_player_action(&self, _: &GameState, ctx: &mut ExecutionContext)
            -> Result<Box<dyn crate::effects::SimultaneousEffectProposal>, ExecutionError> {
            Ok(Box::new(Original { actor: ctx.iteration.iterated_player.unwrap_or(ctx.controller), stop: self.stop }))
        }
        fn execute(&self, game: &mut GameState, ctx: &mut ExecutionContext) -> Result<EffectOutcome, ExecutionError> {
            let original = self.prepare_simultaneous_player_action(game, ctx)?;
            super::complete_prepared_original(original, game, ctx)
        }
    }
    for selected in [false, true] {
        for stop in [false, true] {
            let mut game = crate::tests::test_helpers::setup_two_player_game();
            let alice = PlayerId::from_index(0);
            let bob = PlayerId::from_index(1);
            let source = game.new_object_id();
            let instruction = Effect::new(PrefixStop { stop });
            let instruction = if selected {
                Effect::new(RepeatEffectsEffect::new(1, vec![instruction]))
            } else { instruction };
            let mut ctx = ExecutionContext::new_default(source, alice);
            let progress = ForPlayersEffect::new(PlayerFilter::Any, vec![instruction])
                .prepare_draw_continuation(&mut game, &mut ctx).unwrap();
            assert!(progress.resume.is_none(), "Stop is not a deferred draw boundary");
            assert_eq!(ctx.resolution_stopped(), stop);
            assert_eq!(game.player(alice).unwrap().life, 31);
            assert_eq!(game.player(bob).unwrap().life, if stop { 30 } else { 31 });
            crate::effects::execute_effect(&mut game, &Effect::gain_life(100), &mut ctx).unwrap();
            assert_eq!(game.player(alice).unwrap().life, if stop { 31 } else { 131 });
        }
    }
}
