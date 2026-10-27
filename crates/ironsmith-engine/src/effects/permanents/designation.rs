//! Permanent designations, independent of the actions that precede them.

use crate::effect::EffectOutcome;
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::events::other::{KeywordActionEvent, KeywordActionKind};
use crate::game_state::GameState;
use crate::ids::ObjectId;
use crate::triggers::TriggerEvent;

#[derive(Debug, Clone, Copy)]
pub(super) enum PermanentDesignation {
    Monstrous,
    Renowned,
}

impl PermanentDesignation {
    pub(super) fn is_present(self, game: &GameState, object: ObjectId) -> bool {
        match self {
            Self::Monstrous => game.is_monstrous(object),
            Self::Renowned => game.is_renowned(object),
        }
    }
}

#[derive(Debug, Clone)]
struct ApplyDesignation {
    object: ObjectId,
    designation: PermanentDesignation,
    amount: u32,
}

impl EffectExecutor for ApplyDesignation {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        crate::effects::composition::execute_compound(game, ctx, |game, ctx| {
            if ctx.decision_maker.awaiting_choice()
                || !game
                    .object(self.object)
                    .is_some_and(|object| object.zone == crate::zone::Zone::Battlefield)
                || game.is_phased_out(self.object)
                || self.designation.is_present(game, self.object)
            {
                return Ok(EffectOutcome::count(0));
            }
            let (label, outcome) = match self.designation {
                PermanentDesignation::Monstrous => {
                    game.set_monstrous(self.object);
                    let completion = crate::effects::observe_action_completion(
                        game,
                        TriggerEvent::new_with_provenance(
                            crate::events::BecameMonstrousEvent::new(
                                self.object,
                                ctx.controller,
                                self.amount,
                            ),
                            ctx.provenance,
                        ),
                        Some(ctx.provenance),
                    )?;
                    (
                        "monstrous",
                        EffectOutcome::monstrosity_applied(self.object, self.amount)
                            .with_event(completion),
                    )
                }
                PermanentDesignation::Renowned => {
                    game.set_renowned(self.object);
                    (
                        "renown",
                        crate::effects::composition::complete_keyword_action_with_result(
                            game,
                            ctx,
                            EffectOutcome::count(1),
                            KeywordActionEvent::new(
                                KeywordActionKind::Renown,
                                ctx.controller,
                                self.object,
                                self.amount,
                            ),
                        )?,
                    )
                }
            };
            if let Some(stable_id) = game.object(self.object).map(|object| object.stable_id) {
                game.record_ui_effect_event(
                    "level_up",
                    Some(ctx.controller),
                    None,
                    vec![stable_id],
                    Some(i64::from(self.amount)),
                    Some(label.into()),
                );
            }
            Ok(outcome)
        })
    }
}

pub(super) fn apply_designation(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    object: ObjectId,
    designation: PermanentDesignation,
    amount: u32,
) -> Result<EffectOutcome, ExecutionError> {
    ApplyDesignation {
        object,
        designation,
        amount,
    }
    .execute_child(game, ctx)
}

/// Counter placement followed by a one-time designation. The requested amount
/// is frozen before replacements run; a departed incarnation cannot acquire the
/// designation when counter replacement finishes.
#[derive(Debug, Clone)]
pub(super) struct DesignationProgram {
    pub designation: PermanentDesignation,
    pub amount: crate::effect::Value,
    pub missing_source_is_invalid: bool,
}

impl EffectExecutor for DesignationProgram {
    fn library_movement_policy(
        &self,
    ) -> ironsmith_core::effect_capabilities::LibraryMovementPolicy {
        ironsmith_core::effect_capabilities::LibraryMovementPolicy::Leaf
    }

    fn visit_child_effects(&self, visitor: &mut dyn FnMut(&crate::effect::Effect)) {
        visitor(&crate::effect::Effect::new(
            crate::effects::PutCountersEffect::on_source(
                crate::object::CounterType::PlusOnePlusOne,
                self.amount.clone(),
            ),
        ));
    }

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
        use crate::effects::CompletedEffectOutputs;
        let result = crate::effects::composition::execute_transaction(
            game,
            ctx,
            || CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            |game, ctx| {
                let source = ctx.source;
                if self.missing_source_is_invalid && game.object(source).is_none() {
                    return Ok(CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::target_invalid(),
                    ));
                }
                if !game
                    .object(source)
                    .is_some_and(|object| object.zone == crate::zone::Zone::Battlefield)
                    || game.is_phased_out(source)
                    || self.designation.is_present(game, source)
                {
                    return Ok(CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }
                let amount =
                    crate::effects::helpers::resolve_nonnegative_u32(game, &self.amount, ctx)?;
                let placement = crate::effects::PutCountersEffect::on_source(
                    crate::object::CounterType::PlusOnePlusOne,
                    amount,
                )
                .execute_child_with_outputs(game, ctx)?;
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }
                let designation = apply_designation(game, ctx, source, self.designation, amount)?;
                let outcome = EffectOutcome::aggregate_with_primary_result(
                    designation.summary_projection(),
                    [placement.outcome.clone(), designation.clone()],
                );
                let mut outputs = placement;
                outputs
                    .retain_batch_children([CompletedEffectOutputs::aggregate_only(designation)]);
                Ok(outputs.project_aggregate(outcome))
            },
        );
        if ctx.decision_maker.awaiting_choice() {
            return Ok(CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }
        result
    }
}

impl EffectExecutor for ironsmith_core::SetPermanentDesignationEffect {
    fn execute(&self, game: &mut GameState, ctx: &mut ExecutionContext) -> Result<EffectOutcome, ExecutionError> {
        crate::effects::composition::execute_compound(game, ctx, |game, ctx| {
            let targets = crate::effects::helpers::resolve_objects_for_effect(game, ctx, &self.target)?;
            let mut changed = Vec::new();
            for id in targets {
                if game.is_phased_out(id) { continue; }
                match self.designation {
                    ironsmith_core::PermanentDesignation::Harnessed => if game.harness(id) { changed.push(id); },
                }
            }
            Ok(EffectOutcome::with_objects(changed))
        })
    }
    fn get_target_spec(&self) -> Option<&crate::target::ChooseSpec> { Some(&self.target) }
}
