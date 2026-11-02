//! Choose a counter kind on a target and put another counter of that kind on it.

use crate::effect::EffectOutcome;
use crate::effects::helpers::resolve_objects_for_effect;
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::object::CounterType;
use crate::target::ChooseSpec;

pub use ironsmith_core::PutCounterOfChosenKindEffect;

use super::kind_selection::{choose_binding, counter_label, kinds_on, put_kind_on_recipients};

impl EffectExecutor for PutCounterOfChosenKindEffect {
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
        let mut program = super::ForEachCounterKindPutOrRemoveEffect::one_kind(self.target.clone());
        program.put_only = true;
        program.execute_child_with_outputs(game, ctx)
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        if matches!(self.target.base(), ChooseSpec::ObjectOrPlayer(..)) {
            "target permanent or player"
        } else {
            "target permanent"
        }
    }
}

pub use ironsmith_core::PutCounterOfKindChosenFromEffect;
pub use ironsmith_core::PutCounterOfRandomKindEffect;

impl EffectExecutor for PutCounterOfRandomKindEffect {
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
        crate::effects::composition::execute_transaction(
            game,
            ctx,
            || crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            |game, ctx| {
            let recipients = resolve_objects_for_effect(game, ctx, &self.recipient)?;
            let Some(&recipient) = recipients.first() else {
                return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                    EffectOutcome::count(0),
                ));
            };
            // The eligible kinds are read when the choice is made; a kind the
            // recipient already has a counter of is not eligible.
            let present = kinds_on(game, &[recipient]);
            let mut candidates = self
                .kinds
                .iter()
                .copied()
                .filter(|kind| !self.only_absent_kinds || !present.contains(kind))
                .collect::<Vec<CounterType>>();
            candidates.dedup();
            if candidates.is_empty() {
                return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                    EffectOutcome::resolved().with_execution_fact(crate::effect::ExecutionFact::ChoiceAvailability { available: false }),
                ));
            }
            let kind = if self.player_choice {
                let chooser = self.chooser.as_ref().map(|filter|
                    crate::effects::helpers::resolve_player_filter_as_chooser(game, filter, ctx))
                    .transpose()?.unwrap_or(ctx.controller);
                let options = candidates.iter().map(|kind|
                    (*kind, format!("Choose {} counter", super::kind_selection::counter_label(*kind))))
                    .collect::<Vec<_>>();
                let Some(kind) = super::kind_selection::choose_binding_for_player(
                    game, ctx, chooser, "Choose a counter kind", &options, false) else {
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0).with_execution_fact(crate::effect::ExecutionFact::ChoiceAvailability { available: true })));
                };
                kind
            } else {
                game.shuffle_slice(&mut candidates);
                candidates[0]
            };
            let mut outputs = put_kind_on_recipients(game, ctx, kind, &[recipient], None, false)?;
            outputs.outcome = outputs.outcome.with_execution_fact(crate::effect::ExecutionFact::ChoiceAvailability { available: true });
            Ok(outputs)
            },
        )
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        self.recipient.is_target().then_some(&self.recipient)
    }

    fn target_description(&self) -> &'static str {
        "target permanent"
    }
}

impl EffectExecutor for PutCounterOfKindChosenFromEffect {
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
        crate::effects::composition::execute_transaction(
            game,
            ctx,
            || crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            |game, ctx| {
                use crate::filter::ObjectFilterExt as _;

                // Recipients are fixed first: a declared target was chosen on the
                // stack, and "each" recipients are the matching objects now.
                let recipients = resolve_objects_for_effect(game, ctx, &self.recipients)?;

                // Every counter (object, kind) on the objects the source names.
                let filter_ctx = ctx.filter_context(game);
                let mut counters: Vec<(crate::ids::ObjectId, CounterType)> = Vec::new();
                for id in game.battlefield.iter().copied() {
                    let Some(object) = game.object(id) else {
                        continue;
                    };
                    if !self.kind_source.matches(object, &filter_ctx, game) {
                        continue;
                    }
                    let kinds = kinds_on(game, &[id]);
                    counters.extend(kinds.into_iter().map(|kind| (id, kind)));
                }
                if counters.is_empty() {
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::resolved(),
                    ));
                }
                let options = counters
                    .iter()
                    .map(|(id, kind)| {
                        let name = game
                            .object(*id)
                            .map(|object| object.name.to_string())
                            .unwrap_or_default();
                        (
                            (*id, *kind),
                            format!("{name}: {} counter", counter_label(*kind)),
                        )
                    })
                    .collect::<Vec<_>>();
                let Some((chosen_object, kind)) =
                    choose_binding(game, ctx, "Choose a counter", &options, true)
                else {
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                };
                put_kind_on_recipients(
                    game,
                    ctx,
                    kind,
                    &recipients,
                    self.exclude_kind_object.then_some(chosen_object),
                    self.only_if_absent,
                )
            },
        )
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        self.recipients.is_target().then_some(&self.recipients)
    }

    fn target_description(&self) -> &'static str {
        "target permanent"
    }
}

#[cfg(test)]
mod choice_availability_receipt_tests {
    use super::*;
    use crate::effect::{EffectPredicate, EffectPredicateRuntimeExt, ExecutionFact};
    use crate::ids::{CardId, PlayerId};
    use crate::zone::Zone;
    #[test]
    fn original_menu_receipt_records_eligibility_before_placement() {
        let player=PlayerId(0);
        for exhausted in [false,true] {
            let mut game=GameState::new(vec!["A".into(),"B".into()],20);
            let source=game.create_object_from_card(&crate::card::CardBuilder::new(CardId::new(),"Menu recipient").build(),player,Zone::Battlefield);
            if exhausted { game.add_counters(source,CounterType::Charge,1).unwrap(); }
            let effect=PutCounterOfRandomKindEffect::new(vec![CounterType::Charge],ChooseSpec::Source,true).with_player_choice(true);
            let result=effect.execute(&mut game,&mut crate::effects::EffectContext::new_default(source,player)).unwrap();
            assert_eq!(EffectPredicate::ChoiceUnavailable.evaluate_outcome(&result),exhausted);
            assert!(result.instruction_result().has_execution_fact(|fact|matches!(fact,ExecutionFact::ChoiceAvailability {available} if *available==!exhausted)));
        }
    }
    #[test]
    fn unavailable_choice_never_means_decline_zero_output_prevention_or_auxiliary_failure() {
        let gate=EffectPredicate::ChoiceUnavailable;
        for result in [EffectOutcome::resolved(),EffectOutcome::count(0),EffectOutcome::declined(),EffectOutcome::prevented(),EffectOutcome::count(0).with_execution_fact(ExecutionFact::ChoiceAvailability{available:true})] {
            assert!(!gate.evaluate_outcome(&result));
        }
        let available=EffectOutcome::prevented().with_execution_fact(ExecutionFact::ChoiceAvailability{available:true});
        let unavailable=EffectOutcome::resolved().with_execution_fact(ExecutionFact::ChoiceAvailability{available:false});
        assert!(!gate.evaluate_outcome(&EffectOutcome::aggregate_replacement_outcomes(available.clone(),[unavailable.clone()])));
        assert!(gate.evaluate_outcome(&EffectOutcome::aggregate_replacement_outcomes(unavailable,[available])));
    }
}
