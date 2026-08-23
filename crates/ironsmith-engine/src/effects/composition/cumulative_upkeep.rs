//! Cumulative upkeep payment composition.

use crate::decision::{FallbackStrategy, SelectFirstDecisionMaker};
use crate::decisions::make_boolean_decision;
use crate::effect::{Effect, EffectOutcome};
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_player_filter, resolve_value};
use crate::effects::{ExecutionContext, ExecutionError, execute_effect};
use crate::game_state::GameState;
use crate::object::CounterType;

pub type CumulativeUpkeepEffect = ironsmith_core::CumulativeUpkeepEffect<Effect>;

fn execute_sequence(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    effects: &[Effect],
) -> Result<EffectOutcome, ExecutionError> {
    let mut outcomes = Vec::new();
    for effect in effects {
        let outcome = execute_effect(game, effect, ctx)?;
        let status = outcome.status;
        outcomes.push(outcome);
        if status.is_failure() {
            break;
        }
    }
    Ok(EffectOutcome::aggregate(outcomes))
}

fn execute_failure(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    effects: &[Effect],
) -> Result<EffectOutcome, ExecutionError> {
    execute_sequence(game, ctx, effects)
}

fn execute_unpaid_failure(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    player: crate::ids::PlayerId,
    effects: &[Effect],
) -> Result<EffectOutcome, ExecutionError> {
    let mut lookback_source_snapshots = game.trigger_source_lookback_snapshots();
    let source_snapshot = game
        .object(ctx.source)
        .map(|object| game.cached_object_snapshot_with_calculated_characteristics(object));
    if let Some(snapshot) = source_snapshot.as_ref()
        && !lookback_source_snapshots
            .iter()
            .any(|candidate| candidate.stable_id == snapshot.stable_id)
    {
        lookback_source_snapshots.push(snapshot.clone());
    }
    let outcome = execute_failure(game, ctx, effects)?;
    game.queue_trigger_event(
        ctx.provenance,
        crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::other::KeywordActionEvent::new(
                crate::events::other::KeywordActionKind::CumulativeUpkeepNotPaid,
                player,
                ctx.source,
                1,
            )
            .with_snapshot(source_snapshot),
            ctx.provenance,
        )
        .with_lookback_source_snapshots(lookback_source_snapshots),
    );
    Ok(outcome)
}

fn restore_payment_checkpoint(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    game_checkpoint: GameState,
    ctx_checkpoint: ExecutionContextCheckpoint,
) {
    *game = game_checkpoint;
    ctx.source = ctx_checkpoint.source;
    ctx.controller = ctx_checkpoint.controller;
    ctx.targets = ctx_checkpoint.targets;
    ctx.target_assignments = ctx_checkpoint.target_assignments;
    ctx.x_value = ctx_checkpoint.x_value;
    ctx.effect_outcomes = ctx_checkpoint.effect_outcomes;
    ctx.vote_results = ctx_checkpoint.vote_results;
    ctx.iteration = ctx_checkpoint.iteration;
    ctx.optional_costs_paid = ctx_checkpoint.optional_costs_paid;
    ctx.casting_method = ctx_checkpoint.casting_method;
    ctx.combat = ctx_checkpoint.combat;
    ctx.target_snapshots = ctx_checkpoint.target_snapshots;
    ctx.source_snapshot = ctx_checkpoint.source_snapshot;
    ctx.tagged_objects = ctx_checkpoint.tagged_objects;
    ctx.tagged_players = ctx_checkpoint.tagged_players;
    ctx.face_down_exile_viewers = ctx_checkpoint.face_down_exile_viewers;
    ctx.triggering_event = ctx_checkpoint.triggering_event;
    ctx.chosen_modes = ctx_checkpoint.chosen_modes;
    ctx.cause = ctx_checkpoint.cause;
    ctx.provenance = ctx_checkpoint.provenance;
    ctx.mana = ctx_checkpoint.mana;
    ctx.replacement = ctx_checkpoint.replacement;
}

struct ExecutionContextCheckpoint {
    source: crate::ids::ObjectId,
    controller: crate::ids::PlayerId,
    targets: Vec<crate::effects::ResolvedTarget>,
    target_assignments: Vec<crate::game_state::TargetAssignment>,
    x_value: Option<u32>,
    effect_outcomes: std::collections::HashMap<ironsmith_core::EffectId, EffectOutcome>,
    vote_results: std::collections::HashMap<crate::ids::ObjectId, crate::effects::VoteResult>,
    iteration: crate::effects::context::IterationContext,
    optional_costs_paid: crate::cost::OptionalCostsPaid,
    casting_method: crate::alternative_cast::CastingMethod,
    combat: crate::effects::context::CombatExecutionContext,
    target_snapshots:
        std::collections::HashMap<crate::ids::ObjectId, crate::snapshot::ObjectSnapshot>,
    source_snapshot: Option<crate::snapshot::ObjectSnapshot>,
    tagged_objects:
        std::collections::HashMap<crate::tag::TagKey, Vec<crate::snapshot::ObjectSnapshot>>,
    tagged_players: std::collections::HashMap<crate::tag::TagKey, Vec<crate::ids::PlayerId>>,
    face_down_exile_viewers: std::collections::HashMap<
        crate::ids::ObjectId,
        std::collections::HashSet<crate::ids::PlayerId>,
    >,
    triggering_event: Option<crate::triggers::TriggerEvent>,
    chosen_modes: Option<Vec<usize>>,
    cause: crate::events::cause::EventCause,
    provenance: crate::provenance::ProvNodeId,
    mana: crate::effects::context::ManaExecutionContext,
    replacement: crate::effects::context::ReplacementExecutionContext,
}

impl ExecutionContextCheckpoint {
    fn from_context(ctx: &ExecutionContext) -> Self {
        Self {
            source: ctx.source,
            controller: ctx.controller,
            targets: ctx.targets.clone(),
            target_assignments: ctx.target_assignments.clone(),
            x_value: ctx.x_value,
            effect_outcomes: ctx.effect_outcomes.clone(),
            vote_results: ctx.vote_results.clone(),
            iteration: ctx.iteration,
            optional_costs_paid: ctx.optional_costs_paid.clone(),
            casting_method: ctx.casting_method.clone(),
            combat: ctx.combat,
            target_snapshots: ctx.target_snapshots.clone(),
            source_snapshot: ctx.source_snapshot.clone(),
            tagged_objects: ctx.tagged_objects.clone(),
            tagged_players: ctx.tagged_players.clone(),
            face_down_exile_viewers: ctx.face_down_exile_viewers.clone(),
            triggering_event: ctx.triggering_event.clone(),
            chosen_modes: ctx.chosen_modes.clone(),
            cause: ctx.cause.clone(),
            provenance: ctx.provenance,
            mana: ctx.mana.clone(),
            replacement: ctx.replacement.clone(),
        }
    }
}

fn payment_can_complete(
    effects: &[Effect],
    count: usize,
    game: &GameState,
    ctx: &ExecutionContext,
) -> bool {
    let mut simulated_game = game.clone();
    let mut simulated_dm = SelectFirstDecisionMaker;
    let mut simulated_ctx = ExecutionContext::new_default(ctx.source, ctx.controller)
        .with_decision_maker(&mut simulated_dm);
    simulated_ctx.mana.payment_reason = Some(crate::costs::PaymentReason::CumulativeUpkeep);

    for _ in 0..count {
        let Ok(outcome) = execute_sequence(&mut simulated_game, &mut simulated_ctx, effects) else {
            return false;
        };
        if outcome.status.is_failure() {
            return false;
        }
    }
    true
}

fn execute_payment_atomically(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    effects: &[Effect],
    count: usize,
) -> Result<Option<EffectOutcome>, ExecutionError> {
    let game_checkpoint = game.clone();
    let ctx_checkpoint = ExecutionContextCheckpoint::from_context(ctx);
    ctx.mana.payment_reason = Some(crate::costs::PaymentReason::CumulativeUpkeep);
    let mut outcomes = Vec::new();
    let mut failed_to_pay = false;
    let mut execution_error = None;

    for _ in 0..count {
        match execute_sequence(game, ctx, effects) {
            Ok(outcome) => {
                let status = outcome.status;
                outcomes.push(outcome);
                if status.is_failure() {
                    failed_to_pay = true;
                    break;
                }
            }
            Err(err) => {
                execution_error = Some(err);
                break;
            }
        }
    }

    if let Some(err) = execution_error {
        restore_payment_checkpoint(game, ctx, game_checkpoint, ctx_checkpoint);
        return Err(err);
    }
    if failed_to_pay {
        restore_payment_checkpoint(game, ctx, game_checkpoint, ctx_checkpoint);
        return Ok(None);
    }

    Ok(Some(EffectOutcome::aggregate_summing_counts(outcomes)))
}

impl EffectExecutor for CumulativeUpkeepEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn visit_child_effects(&self, visitor: &mut dyn FnMut(&Effect)) {
        for effect in &self.payment {
            visitor(effect);
        }
        for effect in &self.failure {
            visitor(effect);
        }
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player = resolve_player_filter(game, &self.player, ctx)?;
        let count = resolve_value(
            game,
            &crate::effect::Value::CountersOnSource(CounterType::Age),
            ctx,
        )?
        .max(0) as usize;
        if count == 0 {
            return Ok(EffectOutcome::count(0));
        }

        let can_attempt = payment_can_complete(&self.payment, count, game, ctx);
        let wants_to_pay = can_attempt
            && make_boolean_decision(
                game,
                &mut ctx.decision_maker,
                player,
                ctx.source,
                format!("Pay cumulative upkeep {count} time(s)?"),
                FallbackStrategy::Accept,
            );

        if !wants_to_pay {
            if game.controller_of_id(ctx.source) != Some(player) {
                return Ok(EffectOutcome::count(0));
            }
            return execute_unpaid_failure(game, ctx, player, &self.failure);
        }

        let Some(outcome) = execute_payment_atomically(game, ctx, &self.payment, count)? else {
            return execute_unpaid_failure(game, ctx, player, &self.failure);
        };
        Ok(outcome)
    }

    fn get_target_spec(&self) -> Option<&crate::target::ChooseSpec> {
        super::target_metadata::first_target_spec(&[&self.payment, &self.failure])
    }

    fn decision_related_object_specs(&self) -> Vec<crate::target::ChooseSpec> {
        super::target_metadata::related_object_specs(&[&self.payment, &self.failure])
    }

    fn target_description(&self) -> &'static str {
        super::target_metadata::first_target_description(&[&self.payment, &self.failure], "target")
    }

    fn get_target_count(&self) -> Option<crate::effect::ChoiceCount> {
        super::target_metadata::first_target_count(&[&self.payment, &self.failure])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::decision::DecisionMaker;
    use crate::ids::{CardId, PlayerId};
    use crate::target::PlayerFilter;
    use crate::types::CardType;
    use crate::zone::Zone;

    struct BooleanDecisionMaker {
        response: bool,
    }

    impl DecisionMaker for BooleanDecisionMaker {
        fn decide_boolean(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::BooleanContext,
        ) -> bool {
            self.response
        }
    }

    struct ScriptedBooleanDecisionMaker {
        responses: Vec<bool>,
        index: usize,
    }

    impl ScriptedBooleanDecisionMaker {
        fn new(responses: Vec<bool>) -> Self {
            Self {
                responses,
                index: 0,
            }
        }
    }

    impl DecisionMaker for ScriptedBooleanDecisionMaker {
        fn decide_boolean(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::BooleanContext,
        ) -> bool {
            let response = self.responses.get(self.index).copied().unwrap_or(false);
            self.index += 1;
            response
        }
    }

    #[derive(Debug, Clone)]
    struct BooleanGatedLoseLifePayment;

    impl EffectExecutor for BooleanGatedLoseLifePayment {
        fn clone_box(&self) -> Box<dyn EffectExecutor> {
            Box::new(self.clone())
        }

        fn execute(
            &self,
            game: &mut GameState,
            ctx: &mut ExecutionContext,
        ) -> Result<EffectOutcome, ExecutionError> {
            let accepted = make_boolean_decision(
                game,
                &mut ctx.decision_maker,
                ctx.controller,
                ctx.source,
                "Pay 1 life for this age counter?",
                FallbackStrategy::Accept,
            );
            if !accepted {
                return Ok(EffectOutcome::impossible());
            }

            game.player_mut(ctx.controller)
                .expect("controller exists")
                .lose_life(1);
            Ok(EffectOutcome::count(1))
        }
    }

    fn source_with_age_counters(
        game: &mut GameState,
        controller: PlayerId,
        count: u32,
    ) -> crate::ids::ObjectId {
        let card = CardBuilder::new(CardId::new(), "Cumulative Permanent")
            .card_types(vec![CardType::Creature])
            .build();
        let source = game.create_object_from_card(&card, controller, Zone::Battlefield);
        game.object_mut(source)
            .expect("source exists")
            .add_counters(CounterType::Age, count);
        source
    }

    #[test]
    fn cumulative_upkeep_runs_payment_once_per_age_counter() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = source_with_age_counters(&mut game, alice, 2);
        let mut dm = BooleanDecisionMaker { response: true };
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);

        let effect = CumulativeUpkeepEffect::new(
            PlayerFilter::You,
            vec![Effect::lose_life_player(1, PlayerFilter::You)],
            vec![Effect::sacrifice_source()],
        );

        effect
            .execute(&mut game, &mut ctx)
            .expect("effect resolves");

        assert_eq!(game.player(alice).expect("alice").life, 18);
        assert!(game.battlefield.contains(&source));
    }

    #[test]
    fn cumulative_upkeep_sacrifices_source_when_declined() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = source_with_age_counters(&mut game, alice, 1);
        let mut dm = BooleanDecisionMaker { response: false };
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);

        let effect = CumulativeUpkeepEffect::new(
            PlayerFilter::You,
            vec![Effect::lose_life_player(1, PlayerFilter::You)],
            vec![Effect::sacrifice_source()],
        );

        effect
            .execute(&mut game, &mut ctx)
            .expect("effect resolves");

        assert_eq!(game.player(alice).expect("alice").life, 20);
        assert!(!game.battlefield.contains(&source));
        let events = game.take_pending_trigger_events();
        let unpaid = events
            .iter()
            .find_map(|event| {
                event
                    .downcast::<crate::events::other::KeywordActionEvent>()
                    .filter(|event| {
                        event.action
                            == crate::events::other::KeywordActionKind::CumulativeUpkeepNotPaid
                    })
            })
            .expect("declining cumulative upkeep should emit a typed unpaid action");
        assert_eq!(unpaid.player, alice);
        assert_eq!(unpaid.source, source);
        assert!(
            events.iter().any(|event| {
                event
                    .lookback_source_snapshots()
                    .iter()
                    .any(|snapshot| snapshot.object_id == source)
            }),
            "unpaid cumulative upkeep event should retain pre-sacrifice trigger-source LKI"
        );
    }

    #[test]
    fn cumulative_upkeep_declined_after_control_change_does_not_sacrifice_source() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = source_with_age_counters(&mut game, alice, 1);
        game.set_current_controller(source, bob);
        let mut dm = BooleanDecisionMaker { response: false };
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);

        let effect = CumulativeUpkeepEffect::new(
            PlayerFilter::You,
            vec![Effect::lose_life_player(1, PlayerFilter::You)],
            vec![Effect::sacrifice_source()],
        );

        effect
            .execute(&mut game, &mut ctx)
            .expect("effect resolves");

        assert!(game.battlefield.contains(&source));
        assert_eq!(game.controller_of_id(source), Some(bob));
    }

    #[test]
    fn cumulative_upkeep_rolls_back_partial_payment_before_sacrificing_source() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = source_with_age_counters(&mut game, alice, 2);
        let mut dm = ScriptedBooleanDecisionMaker::new(vec![
            true,  // choose to pay cumulative upkeep
            true,  // first per-counter payment succeeds
            false, // second per-counter payment fails
        ]);
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);

        let effect = CumulativeUpkeepEffect::new(
            PlayerFilter::You,
            vec![Effect::new(BooleanGatedLoseLifePayment)],
            vec![Effect::sacrifice_source()],
        );

        effect
            .execute(&mut game, &mut ctx)
            .expect("effect resolves");

        assert_eq!(
            game.player(alice).expect("alice").life,
            20,
            "partial cumulative upkeep payments must not be kept"
        );
        assert!(!game.battlefield.contains(&source));
    }
}
