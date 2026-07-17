//! "Unless pays" effect implementation.

use crate::costs::Cost;
use crate::decision::FallbackStrategy;
use crate::decisions::make_boolean_decision;
use crate::effect::{Effect, EffectOutcome, Value};
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::effects::{ExecutionContext, ExecutionError, execute_effect};
use crate::game_state::GameState;
use crate::ids::PlayerId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::special_actions::{
    can_pay_total_cost_with_reason_in_context, pay_total_cost_with_choice_in_context,
};
use crate::target::PlayerFilter;

/// Effect that executes inner effects unless a player pays a mana cost.
///
/// "Sacrifice this creature unless you pay {U}" - the player can choose to pay
/// the mana to prevent the inner effects from happening.
///
/// # Fields
///
/// * `effects` - The effects to execute if the player does NOT pay
/// * `player` - Which player is asked to pay
/// * `mana` - The mana cost that must be paid to prevent the effects
///
/// # Result
///
/// - If player pays: `crate::effect::OutcomeStatus::Declined` (effects prevented)
/// - If player doesn't pay: the result of executing inner effects
#[derive(Debug, Clone, PartialEq)]
pub struct UnlessPaysEffect {
    /// The effects to execute if the player does not pay.
    pub effects: Vec<Effect>,
    /// Which player is asked to pay.
    pub player: PlayerFilter,
    /// Total cost required to prevent the effects.
    pub cost: crate::cost::TotalCost,
}

impl UnlessPaysEffect {
    /// Create a new "unless pays" effect.
    pub fn new(effects: Vec<Effect>, player: PlayerFilter, mana: Vec<ManaSymbol>) -> Self {
        Self::new_with_life_and_additional_and_multiplier_and_x(
            effects, player, mana, None, None, None, None,
        )
    }

    /// Create a new "unless pays" effect with a composable total cost.
    pub fn new_total_cost(
        effects: Vec<Effect>,
        player: PlayerFilter,
        cost: crate::cost::TotalCost,
    ) -> Self {
        Self {
            effects,
            player,
            cost,
        }
    }

    /// Create a new "unless pays" effect with optional life payment.
    pub fn new_with_life(
        effects: Vec<Effect>,
        player: PlayerFilter,
        mana: Vec<ManaSymbol>,
        life: Option<Value>,
    ) -> Self {
        Self::new_with_life_and_additional_and_multiplier_and_x(
            effects, player, mana, life, None, None, None,
        )
    }

    /// Create a new "unless pays" effect with optional life and dynamic generic payment.
    pub fn new_with_life_and_additional(
        effects: Vec<Effect>,
        player: PlayerFilter,
        mana: Vec<ManaSymbol>,
        life: Option<Value>,
        additional_generic: Option<Value>,
    ) -> Self {
        Self::new_with_life_and_additional_and_multiplier_and_x(
            effects,
            player,
            mana,
            life,
            additional_generic,
            None,
            None,
        )
    }

    /// Create a new "unless pays" effect with optional life, dynamic generic payment,
    /// and dynamic mana multiplier.
    pub fn new_with_life_and_additional_and_multiplier(
        effects: Vec<Effect>,
        player: PlayerFilter,
        mana: Vec<ManaSymbol>,
        life: Option<Value>,
        additional_generic: Option<Value>,
        mana_multiplier: Option<Value>,
    ) -> Self {
        Self::new_with_life_and_additional_and_multiplier_and_x(
            effects,
            player,
            mana,
            life,
            additional_generic,
            mana_multiplier,
            None,
        )
    }

    /// Create a new "unless pays" effect with optional life, additional generic mana,
    /// mana multiplier, and a bound X value.
    pub fn new_with_life_and_additional_and_multiplier_and_x(
        effects: Vec<Effect>,
        player: PlayerFilter,
        mana: Vec<ManaSymbol>,
        life: Option<Value>,
        additional_generic: Option<Value>,
        mana_multiplier: Option<Value>,
        x_value: Option<Value>,
    ) -> Self {
        Self::new_total_cost(
            effects,
            player,
            build_unless_payment_total_cost(
                mana,
                life,
                additional_generic,
                mana_multiplier,
                x_value,
            ),
        )
    }
}

fn build_unless_payment_total_cost(
    mana: Vec<ManaSymbol>,
    life: Option<Value>,
    additional_generic: Option<Value>,
    mana_multiplier: Option<Value>,
    x_value: Option<Value>,
) -> crate::cost::TotalCost {
    let mut components = Vec::new();
    let mana_cost = ManaCost::from_symbols(mana);
    if !mana_cost.is_empty()
        || additional_generic.is_some()
        || mana_multiplier.is_some()
        || x_value.is_some()
    {
        if additional_generic.is_some() || mana_multiplier.is_some() || x_value.is_some() {
            components.push(Cost::dynamic_mana(ironsmith_core::DynamicManaCost::new(
                mana_cost,
                x_value,
                additional_generic,
                mana_multiplier,
                ironsmith_core::DynamicManaDisplayHint::Default,
            )));
        } else {
            components.push(Cost::mana(mana_cost));
        }
    }
    if let Some(life) = life {
        let effect = Effect::lose_life_player(life, PlayerFilter::You);
        components.push(Cost::try_effect(effect).unwrap_or_else(|detail| {
            panic!("unless-pays life cost is not cost-executable: {detail}")
        }));
    }
    crate::cost::TotalCost::from_costs(components)
}

fn players_in_turn_order(game: &GameState) -> Vec<PlayerId> {
    game.team_apnap_player_order()
}

impl EffectExecutor for UnlessPaysEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn visit_child_effects(&self, visitor: &mut dyn FnMut(&Effect)) {
        for effect in &self.effects {
            visitor(effect);
        }
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let paying_players = if matches!(self.player, PlayerFilter::Any) {
            players_in_turn_order(game)
        } else {
            vec![resolve_player_filter(game, &self.player, ctx)?]
        };
        for paying_player in paying_players {
            let can_afford = can_pay_total_cost_with_reason_in_context(
                game,
                paying_player,
                ctx.source,
                &self.cost,
                crate::costs::PaymentReason::Effect,
                ctx,
            )
            .is_ok();

            let payment_prompt = format!("{} to prevent effect?", self.cost.display());

            // Ask this player if they want to pay.
            let wants_to_pay = if can_afford {
                make_boolean_decision(
                    game,
                    &mut ctx.decision_maker,
                    paying_player,
                    ctx.source,
                    payment_prompt,
                    FallbackStrategy::Accept,
                )
            } else {
                false
            };

            if wants_to_pay {
                if pay_total_cost_with_choice_in_context(
                    game,
                    paying_player,
                    ctx.source,
                    &self.cost,
                    crate::costs::PaymentReason::Effect,
                    ctx,
                )
                .is_ok()
                {
                    return Ok(EffectOutcome::declined());
                }
            }
        }

        // Player didn't pay (or couldn't), execute the inner effects
        let mut outcomes = Vec::new();
        for effect in &self.effects {
            outcomes.push(execute_effect(game, effect, ctx)?);
        }
        Ok(EffectOutcome::aggregate(outcomes))
    }

    fn get_target_spec(&self) -> Option<&crate::target::ChooseSpec> {
        super::target_metadata::first_target_spec(&[&self.effects])
    }

    fn decision_related_object_specs(&self) -> Vec<crate::target::ChooseSpec> {
        super::target_metadata::related_object_specs(&[&self.effects])
    }

    fn target_description(&self) -> &'static str {
        super::target_metadata::first_target_description(&[&self.effects], "target")
    }

    fn get_target_count(&self) -> Option<crate::effect::ChoiceCount> {
        super::target_metadata::first_target_count(&[&self.effects])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::Ability;
    use crate::card::CardBuilder;
    use crate::cost::TotalCost;
    use crate::costs::Cost;
    use crate::decision::SelectFirstDecisionMaker;
    use crate::effect::Effect;
    use crate::effects::ExecutionContext;
    use crate::ids::{CardId, PlayerId};
    use crate::static_abilities::StaticAbility;
    use crate::types::CardType;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn add_payment_replacement_permanent(
        game: &mut GameState,
        controller: PlayerId,
        name: &str,
        ability: StaticAbility,
    ) {
        let source = CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Creature])
            .build();
        let source_id = game.create_object_from_card(&source, controller, Zone::Battlefield);
        game.object_mut(source_id)
            .expect("static-ability source should exist")
            .abilities_mut()
            .push(Ability::static_ability(ability));
    }

    #[test]
    fn unless_pays_effect_can_use_krrik_life_for_black_under_yasharn() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        add_payment_replacement_permanent(
            &mut game,
            alice,
            "Krrik Effect Helper",
            StaticAbility::krrik_black_mana_may_be_paid_with_life(),
        );
        add_payment_replacement_permanent(
            &mut game,
            alice,
            "Yasharn Effect Helper",
            StaticAbility::cant_pay_life_or_sacrifice_nonland_for_cast_or_activate(),
        );

        let source = game.new_object_id();
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);
        let effect = UnlessPaysEffect::new(
            vec![Effect::lose_life(3)],
            PlayerFilter::You,
            vec![ManaSymbol::Black],
        );

        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("unless pays effect should execute");

        assert_eq!(result.status, crate::effect::OutcomeStatus::Declined);
        assert_eq!(game.player(alice).expect("alice exists").life, 18);
    }

    #[test]
    fn unless_pays_total_cost_life_payment_prevents_inner_effect() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);
        let effect = UnlessPaysEffect::new_total_cost(
            vec![Effect::lose_life(5)],
            PlayerFilter::You,
            TotalCost::from_costs(vec![Cost::life(2)]),
        );

        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("unless pays total cost effect should execute");

        assert_eq!(result.status, crate::effect::OutcomeStatus::Declined);
        assert_eq!(game.player(alice).expect("alice exists").life, 18);
    }

    #[test]
    fn unless_pays_total_cost_executes_inner_effect_when_unaffordable() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);
        let effect = UnlessPaysEffect::new_total_cost(
            vec![Effect::lose_life(5)],
            PlayerFilter::You,
            TotalCost::from_costs(vec![Cost::life(30)]),
        );

        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("unless pays total cost effect should execute");

        assert_ne!(result.status, crate::effect::OutcomeStatus::Declined);
        assert_eq!(game.player(alice).expect("alice exists").life, 15);
    }

    #[test]
    fn unless_pays_one_of_pays_selected_branch_only() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);
        let effect = UnlessPaysEffect::new_total_cost(
            vec![Effect::lose_life(5)],
            PlayerFilter::You,
            TotalCost::one_of(vec![
                TotalCost::from_cost(Cost::life(2)),
                TotalCost::from_cost(Cost::life(4)),
            ]),
        );

        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("unless pays one-of cost should execute");

        assert_eq!(result.status, crate::effect::OutcomeStatus::Declined);
        assert_eq!(game.player(alice).expect("alice exists").life, 18);
    }

    #[test]
    fn unless_pays_one_of_pays_only_affordable_branch() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);
        let effect = UnlessPaysEffect::new_total_cost(
            vec![Effect::lose_life(5)],
            PlayerFilter::You,
            TotalCost::one_of(vec![
                TotalCost::from_cost(Cost::life(30)),
                TotalCost::from_cost(Cost::life(2)),
            ]),
        );

        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("unless pays one-of cost should execute");

        assert_eq!(result.status, crate::effect::OutcomeStatus::Declined);
        assert_eq!(game.player(alice).expect("alice exists").life, 18);
    }

    #[test]
    fn unless_pays_dynamic_x_mana_resolves_in_effect_context() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        game.player_mut(alice)
            .expect("alice exists")
            .mana_pool
            .add(ManaSymbol::Colorless, 3);
        let source = game.new_object_id();
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);
        let effect = UnlessPaysEffect::new_total_cost(
            vec![Effect::lose_life(5)],
            PlayerFilter::You,
            TotalCost::from_cost(Cost::dynamic_mana(ironsmith_core::DynamicManaCost::new(
                ManaCost::from_symbols(vec![ManaSymbol::X]),
                Some(Value::Fixed(3)),
                None,
                None,
                ironsmith_core::DynamicManaDisplayHint::Default,
            ))),
        );

        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("unless pays dynamic mana should execute");

        assert_eq!(result.status, crate::effect::OutcomeStatus::Declined);
        assert_eq!(game.player(alice).expect("alice exists").life, 20);
        assert_eq!(
            game.player(alice).expect("alice exists").mana_pool.total(),
            0
        );
    }
}
