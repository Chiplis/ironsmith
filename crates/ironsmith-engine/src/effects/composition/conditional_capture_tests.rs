//! Source-authored captured conditional lifecycle cases. UNRUN.
use super::*;
use crate::effect::{Effect, EffectId};
use crate::effects::{CompletedEffectOutputs, ForPlayersEffect, SequenceEffect,
    SimultaneousEffectCommit, SimultaneousEffectCompletion, SimultaneousEffectProposal};
use crate::target::{ObjectFilter, PlayerFilter};
use crate::{PlayerId, Zone};
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
const C: PlayerId = PlayerId(2);

#[derive(Debug, Clone)]
struct PreparedWithAddition;
#[derive(Debug)]
struct Original(PlayerId);
struct Addition;
impl EffectExecutor for PreparedWithAddition {
    fn clone_box(&self) -> Box<dyn EffectExecutor> { Box::new(self.clone()) }
    fn execute(&self, _: &mut GameState, _: &mut ExecutionContext) -> Result<EffectOutcome, ExecutionError> {
        Err(ExecutionError::InternalError("test action requires the original/completion owner".into()))
    }
    fn supports_simultaneous_player_action(&self) -> bool { true }
    fn prepare_simultaneous_player_action(&self, _: &GameState, ctx: &mut ExecutionContext)
        -> Result<Box<dyn SimultaneousEffectProposal>, ExecutionError> {
        Ok(Box::new(Original(ctx.iteration.iterated_player.unwrap())))
    }
}
impl SimultaneousEffectProposal for Original {
    fn commit_original_with_outputs(self: Box<Self>, game: &mut GameState, _: &mut ExecutionContext)
        -> Result<SimultaneousEffectCommit<CompletedEffectOutputs>, ExecutionError> {
        game.player_mut(self.0).unwrap().life += 2;
        Ok(SimultaneousEffectCommit {
            outcome: CompletedEffectOutputs::aggregate_only(EffectOutcome::count(2)),
            completion: Some(Box::new(Addition)),
        })
    }
    fn commit(self: Box<Self>, game: &mut GameState, ctx: &mut ExecutionContext)
        -> Result<EffectOutcome, ExecutionError> {
        super::super::complete_prepared_original(self, game, ctx)
    }
}
impl SimultaneousEffectCompletion for Addition {
    fn freeze(&mut self, _: &mut GameState) -> Result<(), ExecutionError> { Ok(()) }
    fn complete(self: Box<Self>, game: &mut GameState, _: &mut ExecutionContext, original: EffectOutcome)
        -> Result<EffectOutcome, ExecutionError> {
        assert_eq!(game.player(B).unwrap().life, 22);
        assert_eq!(game.player(C).unwrap().life, 22,
            "every participant original must finish before either addition");
        game.player_mut(A).unwrap().life += 5;
        Ok(original)
    }
}
fn game() -> GameState { GameState::new(vec!["A".into(), "B".into(), "C".into()], 20) }

#[test]
fn native_captured_condition_preserves_all_originals_before_additions_and_sampled_gate() {
    let mut game = game();
    let source = game.new_object_id();
    let gate = ConditionalEffect::if_only(Condition::LifeTotalOrLess(20),
        vec![Effect::new(PreparedWithAddition)]).with_condition_result(true);
    let effect = ForPlayersEffect::new(PlayerFilter::Opponent,
        vec![Effect::with_id(41, Effect::new(gate))]);
    let mut ctx = ExecutionContext::new_default(source, A);
    effect.execute(&mut game, &mut ctx).unwrap();
    assert_eq!(game.player(A).unwrap().life, 30);
    assert_eq!(game.player(B).unwrap().life, 22);
    assert_eq!(game.player(C).unwrap().life, 22);
    assert_eq!(ctx.get_outcome(EffectId(41)).unwrap().player_counts(),
        Some([(B, 1), (C, 1)].as_slice()),
        "each completion publishes its captured Boolean, not its child's numeric result");
}

#[test]
fn captured_unprepared_branch_rejects_before_mutation_and_rolls_back_earlier_program_work() {
    let mut game = game(); let source = game.new_object_id();
    let gate = ConditionalEffect::if_only(Condition::LifeTotalOrGreater(0),
        vec![Effect::gain_life(2), Effect::gain_life(3)]).with_condition_result(true);
    let mut ctx = ExecutionContext::new_default(source, A);
    ctx.store_outcome(EffectId(42), EffectOutcome::count(9));
    assert!(matches!(gate.prepare_simultaneous_player_action(&game, &mut ctx),
        Err(ExecutionError::IncompleteEvidence(_))));
    assert_eq!(game.player(A).unwrap().life, 20);
    let program = SequenceEffect::new(vec![Effect::gain_life(7), Effect::new(
        ForPlayersEffect::new(PlayerFilter::Opponent,
            vec![Effect::with_id(42, Effect::new(gate))]),
    )]);
    assert!(matches!(program.execute(&mut game, &mut ctx), Err(ExecutionError::IncompleteEvidence(_))));
    assert_eq!(game.player(A).unwrap().life, 20);
    assert_eq!(ctx.get_outcome(EffectId(42)).unwrap().as_count(), Some(9));
}

#[test]
fn captured_concealed_identity_guard_rejects_without_reveal_or_false_receipt_and_rolls_back() {
    let mut game = game(); let source = game.new_object_id();
    let hidden = game.create_hidden_card_placeholder(A, Zone::Hand, 0, "test-only-commitment".into());
    let tag = crate::TagKey::from("guarded");
    let snapshot = crate::snapshot::ObjectSnapshot::from_object(game.object(hidden).unwrap(), &game);
    let gate = ConditionalEffect::if_only(Condition::TaggedObjectMatches(tag.clone(), ObjectFilter::land()),
        vec![Effect::new(crate::effects::MayEffect::new(vec![
            Effect::new(crate::effects::RevealTaggedEffect::new(tag.clone()))
        ]))]).with_condition_result(true);
    let mut ctx = ExecutionContext::new_default(source, A);
    ctx.tag_object(tag, snapshot);
    assert!(optional_hidden_reveal_guard(&gate, &game, &ctx, false).is_some());
    ctx.store_outcome(EffectId(43), EffectOutcome::count(9));
    assert!(matches!(gate.execute(&mut game, &mut ctx), Err(ExecutionError::IncompleteEvidence(_))));
    assert!(matches!(gate.prepare_simultaneous_player_action(&game, &mut ctx),
        Err(ExecutionError::IncompleteEvidence(_))));
    let program = SequenceEffect::new(vec![Effect::gain_life(7), Effect::with_id(43, Effect::new(gate))]);
    assert!(matches!(program.execute(&mut game, &mut ctx), Err(ExecutionError::IncompleteEvidence(_))));
    assert_eq!(game.player(A).unwrap().life, 20);
    assert!(game.is_hidden_card_placeholder(hidden));
    assert!(!game.is_publicly_revealed_hidden_card(hidden));
    assert_eq!(ctx.get_outcome(EffectId(43)).unwrap().as_count(), Some(9));
}

#[test]
fn a_pending_captured_proposal_has_no_receipt_or_missing_owner_error() {
    struct Pending;
    impl crate::decision::DecisionMaker for Pending { fn awaiting_choice(&self) -> bool { true } }
    let mut game = game(); let source = game.new_object_id(); let mut pending = Pending;
    let mut ctx = ExecutionContext::new(source, A, &mut pending);
    let gate = ConditionalEffect::if_only(Condition::LifeTotalOrGreater(0),
        vec![Effect::gain_life(2), Effect::gain_life(3)]).with_condition_result(true);
    let proposal = gate.prepare_simultaneous_player_action(&game, &mut ctx).unwrap();
    let receipt = proposal.commit_original_with_outputs(&mut game, &mut ctx).unwrap();
    assert_eq!(receipt.outcome.outcome.as_count(), Some(0));
    assert!(receipt.completion.is_none());
    assert_eq!(game.player(A).unwrap().life, 20);
}
