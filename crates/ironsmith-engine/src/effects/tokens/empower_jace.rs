//! Empower Jace (CR 701.71), using the final Reality Fracture rules.
use crate::ability::{Ability, AbilityKind, ActivationTiming};
use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::color::ColorSet;
use crate::cost::TotalCost;
use crate::costs::Cost;
use crate::decisions::make_decision;
use crate::decisions::specs::ChooseObjectsSpec;
use crate::effect::{Effect, EffectOutcome, ExecutionFact};
use crate::effects::helpers::{normalize_object_selection, resolve_value};
use crate::effects::{CreateTokenEffect, EffectExecutor, PutCountersEffect};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::events::{KeywordActionEvent, KeywordActionKind};
use crate::game_state::GameState;
use crate::ids::{CardId, ObjectId, PlayerId};
use crate::object::CounterType;
use crate::target::ChooseSpec;
use crate::types::{CardType, Subtype};

pub type EmpowerJaceEffect = ironsmith_core::EmpowerJaceEffect;

fn jace_token_candidates(game: &GameState, controller: PlayerId) -> Vec<ObjectId> {
    game.battlefield
        .iter()
        .copied()
        .filter(|id| !game.is_phased_out(*id))
        .filter(|id| {
            game.object(*id).is_some_and(|object| {
                object.kind == crate::object::ObjectKind::Token
                    && game.controller_of(object) == controller
                    && game.object_has_card_type(*id, CardType::Planeswalker)
                    && game.calculated_subtypes(*id).contains(&Subtype::Jace)
            })
        })
        .collect()
}

fn loyalty_ability(amount: u32, effect: Effect) -> Ability {
    let mut ability = Ability::activated_with_timing(
        TotalCost::from_cost(Cost::remove_counters(CounterType::Loyalty, amount)),
        vec![effect],
        ActivationTiming::SorcerySpeed,
    );
    if let AbilityKind::Activated(activated) = &mut ability.kind {
        activated.is_loyalty_ability = true;
    }
    ability
}

fn jace_token_definition() -> CardDefinition {
    // The final mechanic specifies blue, nonlegendary, and zero printed loyalty.
    // Counters are added by the action after token entry, not as entry counters.
    // CR 111.4 supplies the subtype-based name when no name is specified.
    CardDefinitionBuilder::new(CardId::new(), "Jace Token")
        .token()
        .card_types(vec![CardType::Planeswalker])
        .subtypes(vec![Subtype::Jace])
        .color_indicator(ColorSet::BLUE)
        .loyalty(0)
        .with_ability(loyalty_ability(1, Effect::surveil(1)))
        .with_ability(loyalty_ability(3, Effect::draw(1)))
        .build()
}

impl EffectExecutor for EmpowerJaceEffect {
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
        if ctx.decision_maker.awaiting_choice() {
            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }
        crate::effects::composition::execute_transaction(
            game,
            ctx,
            || crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            |game, ctx| {
                let amount = resolve_value(game, &self.amount, ctx)?.max(0) as u32;
                let mut outcomes = Vec::new();
                let mut outputs = crate::effects::CompletedEffectOutputs::aggregate_only(
                    EffectOutcome::resolved(),
                );
                let mut candidates = jace_token_candidates(game, ctx.controller);
                if candidates.is_empty() {
                    let child = CreateTokenEffect::you(jace_token_definition(), 1)
                        .execute_child_with_outputs(game, ctx)?;
                    outcomes.push(child.outcome.clone());
                    outputs.retain_owned_child(child);
                    if ctx.decision_maker.awaiting_choice() {
                        return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                            EffectOutcome::count(0),
                        ));
                    }
                    candidates = jace_token_candidates(game, ctx.controller);
                }
                let action = KeywordActionEvent::new(
                    KeywordActionKind::EmpowerJace,
                    ctx.controller,
                    ctx.source,
                    amount,
                );
                if candidates.is_empty() {
                    // Token creation may have been prevented or replaced. There is
                    // still an empower action, but no eligible permanent to modify.
                    return crate::effects::composition::complete_keyword_action_with_outputs(
                        game,
                        ctx,
                        outputs.project_aggregate(EffectOutcome::aggregate(outcomes)),
                        action,
                    );
                }
                let chosen = if candidates.len() == 1 {
                    candidates[0]
                } else {
                    let spec = ChooseObjectsSpec::new(
                        ctx.source,
                        "Choose a Jace planeswalker token you control to empower",
                        candidates.clone(),
                        1,
                        Some(1),
                    );
                    let selected = make_decision(
                        game,
                        ctx.decision_maker,
                        ctx.controller,
                        Some(ctx.source),
                        spec,
                    );
                    if ctx.decision_maker.awaiting_choice() {
                        return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                            EffectOutcome::count(0),
                        ));
                    }
                    normalize_object_selection(selected, &candidates, 1)
                        .first()
                        .copied()
                        .ok_or(ExecutionError::InvalidTarget)?
                };
                let child = PutCountersEffect::new(
                    CounterType::Loyalty,
                    amount,
                    ChooseSpec::SpecificObject(chosen),
                )
                .execute_child_with_outputs(game, ctx)?;
                outcomes.push(child.outcome.clone());
                outputs.retain_owned_child(child);
                // No state-based actions run between the zero-loyalty token's entry
                // and this counter instruction. The enclosing resolution checks
                // them only after the whole keyword action/spell has finished.
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }
                crate::effects::composition::complete_keyword_action_with_outputs(
                    game,
                    ctx,
                    outputs.project_aggregate(
                        EffectOutcome::aggregate(outcomes)
                            .with_execution_fact(ExecutionFact::ChosenObjects(vec![chosen])),
                    ),
                    action,
                )
            },
        )
    }
}
