//! Executable keyword programs, separate from completion event emission.
use crate::effect::EffectOutcome;
use crate::effects::{CostExecutableEffect, EffectExecutor, ExecutionContext, ExecutionError};
use crate::events::{Event, KeywordActionEvent, KeywordActionKind};
use crate::game_state::GameState;

pub(super) fn forage_payments(exclude_source: bool) -> [crate::effect::Effect; 2] {
    use crate::target::{ChooseSpec, ObjectFilter, PlayerFilter};
    let mut graveyard = ObjectFilter::default()
        .owned_by(PlayerFilter::You)
        .in_zone(crate::zone::Zone::Graveyard);
    graveyard.other = exclude_source;
    [
        crate::effect::Effect::new(crate::effects::ExileEffect::with_spec(
            ChooseSpec::Object(graveyard).with_count(crate::effect::ChoiceCount::exactly(3)),
        )),
        crate::effect::Effect::new(crate::effects::SacrificeEffect::you(
            ObjectFilter::default().with_subtype(crate::types::Subtype::Food),
            1,
        )),
    ]
}

#[derive(Debug, Clone)]
struct KeywordActionProgram {
    action: KeywordActionKind,
    amount: u32,
}
impl EffectExecutor for KeywordActionProgram {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.execute_with_outputs(game, ctx)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }

    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        super::execute_transaction(
            game,
            ctx,
            || crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            |game, ctx| commit_keyword_program(game, ctx, self.action, self.amount),
        )
    }
}
pub(super) fn execute_keyword_program_with_outputs(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    action: KeywordActionKind,
    amount: u32,
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    KeywordActionProgram { action, amount }.execute_child_with_outputs(game, ctx)
}

fn commit_keyword_program(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    action: KeywordActionKind,
    amount: u32,
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    if action == KeywordActionKind::Forage {
        let mut outcomes = Vec::new();
        let mut fallback_source_snapshot =
            crate::snapshot::ObjectSnapshot::from_object_id(game, ctx.source)
                .or_else(|| ctx.source_snapshot.clone());
        for _ in 0..amount {
            let source_snapshot = crate::snapshot::ObjectSnapshot::from_object_id(game, ctx.source)
                .or_else(|| fallback_source_snapshot.clone());
            let payments = forage_payments(false);
            let options: Vec<_> = payments
                .iter()
                .enumerate()
                .filter(|(_, effect)| {
                    effect
                        .0
                        .can_execute_as_cost(game, ctx.source, ctx.controller)
                        .is_ok()
                })
                .map(|(index, _)| {
                    (
                        if index == 0 {
                            "Exile three cards from your graveyard"
                        } else {
                            "Sacrifice a Food"
                        }
                        .to_string(),
                        index,
                    )
                })
                .collect();
            if options.is_empty() {
                return Err(ExecutionError::Impossible("cannot forage".into()));
            }
            let choice = crate::decisions::ask_choose_one(
                game,
                &mut ctx.decision_maker,
                ctx.controller,
                ctx.source,
                &options,
            );
            if ctx.decision_maker.awaiting_choice() {
                return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                    EffectOutcome::count(0),
                ));
            }
            let index = choice.unwrap_or(options[0].1);
            let outcome = crate::effects::execute_effect_with_outputs(game, &payments[index], ctx)?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(outcome);
            }
            if outcome.outcome.status.is_failure() {
                return Ok(outcome);
            }
            outcomes.push(outcome);
            let source_snapshot = crate::snapshot::ObjectSnapshot::from_object_id(game, ctx.source)
                .or(source_snapshot);
            fallback_source_snapshot = source_snapshot.clone();
            outcomes.push(super::publish_keyword_action_completion_receipt(
                game,
                ctx,
                crate::triggers::TriggerEvent::new_with_provenance(
                    KeywordActionEvent::new(action, ctx.controller, ctx.source, 1)
                        .with_snapshot(source_snapshot),
                    ctx.provenance,
                ),
            )?);
            if ctx.decision_maker.awaiting_choice() {
                return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                    EffectOutcome::count(0),
                ));
            }
        }
        return Ok(crate::effects::CompletedEffectOutputs::with_primary_result(
            EffectOutcome::count(amount),
            outcomes,
        ));
    }
    if action == KeywordActionKind::AssembleContraption {
        // CR 701.45a deliberately does not define the Unstable Contraption
        // procedure. Keep the action typed and observable for an external
        // profile, but never pretend ordinary CR-only play can execute it.
        return Err(ExecutionError::ExternalRulesProfileRequired {
            action: "assembling a Contraption",
            specification: "the Unstable FAQ",
        });
    }
    if action == KeywordActionKind::Planeswalk {
        if let Some(planar_roll) = ctx
            .triggering_event
            .as_ref()
            .and_then(|event| event.downcast::<crate::events::other::DieRolledEvent>())
            .filter(|event| event.is_planar)
            && !game.is_face_up_planar_object(planar_roll.source)
        {
            // CR 901.9a: this sourceless ability leaves the plane that was
            // face up when the die was rolled. If that plane has already
            // left the planar zone, the ability does nothing on resolution.
            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }
        // CR 701.31a: a player may planeswalk only during a Planechase
        // game, and only the planar controller may. Otherwise the
        // instruction does nothing.
        let may_planeswalk = if game.grand_melee().is_some() {
            game.planar_controllers().contains(&ctx.controller)
        } else {
            game.planar_controller_acting_for(ctx.controller).is_some()
        };
        if !may_planeswalk {
            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }
        let mut outcomes = Vec::with_capacity(amount as usize);
        for _ in 0..amount {
            let would_event = Event::new_with_provenance(
                KeywordActionEvent::new(
                    KeywordActionKind::Planeswalk,
                    ctx.controller,
                    ctx.source,
                    1,
                ),
                ctx.provenance,
            );
            let outcome = super::execute_keyword_action_with_outputs(
                game,
                ctx,
                would_event,
                super::KeywordActionOutput::Body,
                super::KeywordActionAmount::Repetitions,
                |game, _, action| {
                    let destination = game
                        .planeswalk(action.player, action.source)
                        .map_err(ExecutionError::Impossible)?;
                    Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(1).with_affected_objects(vec![destination]),
                    ))
                },
            )?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                    EffectOutcome::count(0),
                ));
            }
            outcomes.push(outcome);
        }
        return Ok(crate::effects::CompletedEffectOutputs::from_children(
            outcomes,
            EffectOutcome::aggregate_summing_counts,
        ));
    }
    if action == KeywordActionKind::SetSchemeInMotion {
        let mut schemes = Vec::with_capacity(amount as usize);
        for _ in 0..amount {
            schemes.push(
                game.set_scheme_in_motion(ctx.controller)
                    .map_err(ExecutionError::Impossible)?,
            );
        }
        return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
            EffectOutcome::resolved().with_affected_objects(schemes),
        ));
    }
    if action == KeywordActionKind::AbandonScheme {
        let scheme = game
            .abandon_scheme(ctx.source)
            .map_err(ExecutionError::Impossible)?;
        return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
            EffectOutcome::resolved().with_affected_objects(vec![scheme]),
        ));
    }
    Err(ExecutionError::InternalError(
        "unknown keyword action program".into(),
    ))
}

/// Harness is a designation transition. Completion publication is separate.
pub(super) fn commit_harness(game: &mut GameState, source: crate::ids::ObjectId) -> bool {
    game.harness(source)
}
