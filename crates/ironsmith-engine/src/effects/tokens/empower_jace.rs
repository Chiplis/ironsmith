//! Empower Jace (CR 701.71), using the final Reality Fracture rules.
use crate::ability::{Ability, AbilityKind, ActivationTiming};
use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::color::ColorSet;
use crate::cost::TotalCost;
use crate::costs::Cost;
use crate::effect::{Effect, EffectOutcome, ExecutionFact};
use crate::effects::helpers::resolve_value;
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
    jace_token_blueprint(CardId::new())
}

fn jace_token_blueprint(card_id: CardId) -> CardDefinition {
    // The final mechanic specifies blue, nonlegendary, and zero printed loyalty.
    // Counters are added by the action after token entry, not as entry counters.
    // CR 111.4 supplies the subtype-based name when no name is specified.
    CardDefinitionBuilder::new(card_id, "Jace Token")
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
    fn visit_child_effects(&self, visitor: &mut dyn FnMut(&Effect)) {
        visitor(&Effect::new(CreateTokenEffect::you(jace_token_blueprint(CardId::from_raw(0)), 1)));
        visitor(&Effect::new(PutCountersEffect::new(CounterType::Loyalty, self.amount.clone(),
            ChooseSpec::Object(crate::target::ObjectFilter::default().with_type(CardType::Planeswalker)
                .with_subtype(Subtype::Jace).token().you_control()))));
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
                let (chosen, children) = crate::effects::composition::ensure_chosen_object(
                    game, ctx, "Choose a Jace planeswalker token you control to empower",
                    jace_token_candidates,
                    |game, ctx| CreateTokenEffect::you(jace_token_definition(), 1)
                        .execute_child_with_outputs(game, ctx),
                )?;
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)));
                }
                let mut outcomes = children.iter().map(|child| child.outcome.clone()).collect::<Vec<_>>();
                let mut outputs = crate::effects::composition::aggregate_children(children);
                let action = KeywordActionEvent::new(KeywordActionKind::EmpowerJace,
                    ctx.controller, ctx.source, amount);
                let Some(chosen) = chosen else {
                    return crate::effects::composition::complete_keyword_action_with_outputs(
                        game, ctx, outputs, action);
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
