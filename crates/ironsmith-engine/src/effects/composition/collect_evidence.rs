//! Collect evidence using one typed action for effects and costs.
use crate::decisions::{make_decision, specs::XValueSpec};
use crate::effect::{
    ChoiceAggregateConstraint, ChoiceAggregateMetric, ChoiceCount, EffectOutcome, ExecutionFact,
    OutcomeValue, Value,
};
use crate::effects::helpers::resolve_value;
use crate::effects::{
    ChooseObjectsEffect, CostExecutableEffect, CostValidationError, EffectExecutor,
    ExecutionContext, ExecutionContextCheckpoint, ExecutionError, ExileEffect,
};
use crate::events::{KeywordActionEvent, KeywordActionKind};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::object::ObjectKind;
use crate::target::{ChooseSpec, ObjectFilter, PlayerFilter};
use crate::triggers::TriggerEvent;
use crate::zone::Zone;

pub type CollectEvidenceEffect = ironsmith_core::CollectEvidenceEffect;
const CHOSEN_EVIDENCE: &str = "__collect_evidence_cards";

pub(crate) fn evidence_capacity(
    game: &GameState,
    player: PlayerId,
    excluded: Option<ObjectId>,
) -> u32 {
    let ids = game
        .player(player)
        .into_iter()
        .flat_map(|player| player.graveyard.iter().copied())
        .filter(|id| Some(*id) != excluded)
        .filter(|id| {
            game.object(*id).is_some_and(|object| {
                object.kind == ObjectKind::Card
                    && object.zone == Zone::Graveyard
                    && object.owner == player
            })
        });
    crate::targeting::aggregate_object_set_value(game, ids, ChoiceAggregateMetric::ManaValue).max(0)
        as u32
}

pub(crate) fn evidence_requirement(
    effect: &CollectEvidenceEffect,
    game: &GameState,
    ctx: &ExecutionContext,
) -> Result<u32, ExecutionError> {
    if matches!(effect.amount.unhinted(), Value::X) && ctx.x_value.is_none() {
        return Ok(0);
    }
    Ok(resolve_value(game, &effect.amount, ctx)?.max(0) as u32)
}

impl EffectExecutor for CollectEvidenceEffect {
    fn as_cost_executable(&self) -> Option<&dyn CostExecutableEffect> {
        Some(self)
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }
        let checkpoint = game.clone();
        let tagged_before = ctx.tagged_objects.clone();
        let context_checkpoint = ExecutionContextCheckpoint::capture(ctx);
        let result = (|| {
            let available = evidence_capacity(game, ctx.controller, None);
            if matches!(self.amount.unhinted(), Value::X) && ctx.x_value.is_none() {
                let selected = make_decision(
                    game,
                    ctx.decision_maker,
                    ctx.controller,
                    Some(ctx.source),
                    XValueSpec::new(ctx.source, available),
                );
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(EffectOutcome::count(0));
                }
                if selected > available {
                    return Err(ExecutionError::InvalidTarget);
                }
                // This is an announcement made by the action, not the total
                // value overpaid. Reflexive follow-ups retain this chosen X.
                ctx.x_value = Some(selected);
            }
            let required = evidence_requirement(self, game, ctx)?;
            if available < required {
                return Ok(EffectOutcome::declined());
            }
            let filter = ObjectFilter::default()
                .nontoken()
                .in_zone(Zone::Graveyard)
                .owned_by(PlayerFilter::You);
            let choose = ChooseObjectsEffect::new(
                filter,
                ChoiceCount::any_number(),
                PlayerFilter::You,
                CHOSEN_EVIDENCE,
            )
            .with_aggregate_constraint(
                ChoiceAggregateConstraint::total_mana_value_at_least(required as i32),
            );
            ctx.tagged_objects.remove(CHOSEN_EVIDENCE);
            let choice = choose.execute(game, ctx)?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(EffectOutcome::count(0));
            }
            let chosen = ctx
                .tagged_objects
                .get(CHOSEN_EVIDENCE)
                .cloned()
                .unwrap_or_default();
            let ids: Vec<_> = chosen.iter().map(|snapshot| snapshot.object_id).collect();
            let identities_valid = ids.iter().all(|id| {
                game.object(*id).is_some_and(|object| {
                    object.kind == ObjectKind::Card
                        && object.zone == Zone::Graveyard
                        && object.owner == ctx.controller
                })
            });
            let total = crate::targeting::aggregate_object_set_value(
                game,
                ids.iter().copied(),
                ChoiceAggregateMetric::ManaValue,
            )
            .max(0) as u32;
            if !identities_valid || total < required {
                return Err(ExecutionError::InvalidTarget);
            }
            let mut outcomes = vec![choice];
            // The normal zone-change pipeline owns replacements and source
            // linkage. There is no substitute direct object move here.
            if !ids.is_empty() {
                outcomes.push(
                    ExileEffect::with_spec(ChooseSpec::Tagged(CHOSEN_EVIDENCE.into()))
                        .execute(game, ctx)?,
                );
            }
            if ctx.decision_maker.awaiting_choice() {
                return Ok(EffectOutcome::count(0));
            }
            ctx.tagged_objects = tagged_before;
            let event = TriggerEvent::new_with_provenance(
                KeywordActionEvent::new(
                    KeywordActionKind::CollectEvidence,
                    ctx.controller,
                    ctx.source,
                    required,
                ),
                ctx.provenance,
            );
            let mut outcome = EffectOutcome::aggregate(outcomes)
                .with_execution_fact(ExecutionFact::ChosenObjects(ids))
                .with_event(event);
            // Collecting zero (including choosing no cards) is one completed
            // action and triggers “whenever you collect evidence”.
            outcome.set_value(OutcomeValue::Count(1));
            outcome.set_status(crate::effect::OutcomeStatus::Succeeded);
            Ok(outcome)
        })();
        if result.is_err() || ctx.decision_maker.awaiting_choice() {
            game.restore_execution_checkpoint(
                checkpoint,
                result.is_ok() && ctx.decision_maker.awaiting_choice(),
            );
            context_checkpoint.restore(ctx);
        }
        result
    }

    fn references_cost_x(&self) -> bool {
        matches!(self.amount.unhinted(), Value::X)
    }

    fn max_cost_x(&self, game: &GameState, source: ObjectId, controller: PlayerId) -> Option<u32> {
        let _ = source;
        matches!(self.amount.unhinted(), Value::X)
            .then(|| evidence_capacity(game, controller, None))
    }
    fn cost_description(&self) -> Option<String> {
        let amount = match self.amount.unhinted() {
            Value::Fixed(amount) => amount.to_string(),
            Value::X => "X".into(),
            _ => return None,
        };
        Some(format!("Collect evidence {amount}"))
    }
}
impl CostExecutableEffect for CollectEvidenceEffect {
    fn can_execute_as_cost(
        &self,
        game: &GameState,
        source: ObjectId,
        controller: PlayerId,
    ) -> Result<(), CostValidationError> {
        CostExecutableEffect::can_execute_as_cost_with_reason(
            self,
            game,
            source,
            controller,
            crate::costs::PaymentReason::ActivateAbility,
        )
    }
    fn can_execute_as_cost_with_reason(
        &self,
        game: &GameState,
        source: ObjectId,
        controller: PlayerId,
        reason: crate::costs::PaymentReason,
    ) -> Result<(), CostValidationError> {
        let ctx = ExecutionContext::new_default(source, controller);
        let required = evidence_requirement(self, game, &ctx)
            .map_err(|error| CostValidationError::Other(format!("{error:?}")))?;
        let exclude = matches!(reason, crate::costs::PaymentReason::CastSpell).then_some(source);
        if evidence_capacity(game, controller, exclude) < required {
            return Err(CostValidationError::NotEnoughCards);
        }
        Ok(())
    }
}
