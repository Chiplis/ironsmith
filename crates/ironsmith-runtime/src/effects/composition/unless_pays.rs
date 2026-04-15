//! "Unless pays" effect implementation.

use crate::decision::FallbackStrategy;
use crate::decisions::make_boolean_decision;
use crate::effect::{Effect, EffectOutcome, Value};
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_player_filter, resolve_value};
use crate::events::LifeLossEvent;
use crate::executor::{ExecutionContext, ExecutionError, execute_effect};
use crate::game_state::GameState;
use crate::ids::PlayerId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::target::PlayerFilter;
use crate::triggers::TriggerEvent;

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
    /// The mana cost required to prevent the effects.
    pub mana: Vec<ManaSymbol>,
    /// Optional life payment required in addition to mana.
    pub life: Option<Value>,
    /// Optional dynamic additional generic mana payment.
    pub additional_generic: Option<Value>,
    /// Optional bound value for an X mana symbol in the payment cost.
    pub x_value: Option<Value>,
    /// Optional multiplier for the mana symbol sequence.
    pub mana_multiplier: Option<Value>,
}

impl UnlessPaysEffect {
    /// Create a new "unless pays" effect.
    pub fn new(effects: Vec<Effect>, player: PlayerFilter, mana: Vec<ManaSymbol>) -> Self {
        Self::new_with_life_and_additional_and_multiplier_and_x(
            effects, player, mana, None, None, None, None,
        )
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
        Self {
            effects,
            player,
            mana,
            life,
            additional_generic,
            x_value,
            mana_multiplier,
        }
    }
}

fn players_in_turn_order(game: &GameState) -> Vec<PlayerId> {
    if game.turn_store.turn_order.is_empty() {
        return Vec::new();
    }

    let start = game
        .turn_store
        .turn_order
        .iter()
        .position(|&player_id| player_id == game.turn.active_player)
        .unwrap_or(0);

    (0..game.turn_store.turn_order.len())
        .filter_map(|offset| {
            let player_id =
                game.turn_store.turn_order[(start + offset) % game.turn_store.turn_order.len()];
            game.player(player_id)
                .filter(|player| player.is_in_game())
                .map(|_| player_id)
        })
        .collect()
}

impl EffectExecutor for UnlessPaysEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
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
        let life_to_pay = self
            .life
            .as_ref()
            .map(|value| resolve_value(game, value, ctx).map(|n| n.max(0) as u32))
            .transpose()?
            .unwrap_or(0);
        let additional_generic = self
            .additional_generic
            .as_ref()
            .map(|value| resolve_value(game, value, ctx).map(|n| n.max(0) as u32))
            .transpose()?
            .unwrap_or(0);
        let x_value = self
            .x_value
            .as_ref()
            .map(|value| resolve_value(game, value, ctx).map(|n| n.max(0) as u32))
            .transpose()?
            .unwrap_or(0);
        let mana_multiplier = self
            .mana_multiplier
            .as_ref()
            .map(|value| resolve_value(game, value, ctx).map(|n| n.max(0) as u32))
            .transpose()?
            .unwrap_or(1);

        let mut mana_symbols = Vec::new();
        for _ in 0..mana_multiplier {
            mana_symbols.extend(self.mana.iter().copied());
        }
        if additional_generic > 0 {
            let capped = additional_generic.min(u8::MAX as u32) as u8;
            mana_symbols.push(ManaSymbol::Generic(capped));
        }

        let mana_display = if mana_symbols.is_empty() {
            None
        } else {
            Some(ManaCost::from_symbols(mana_symbols.clone()).to_oracle())
        };
        let payment_display = match (mana_display, life_to_pay) {
            (None, 0) => "no cost".to_string(),
            (None, life) => format!("{life} life"),
            (Some(mana), 0) => mana,
            (Some(mana), life) => format!("{mana} and {life} life"),
        };

        for paying_player in paying_players {
            // Check if this player can afford to pay mana/life.
            let can_afford_mana = {
                let cost = ManaCost::from_symbols(mana_symbols.clone());
                let adjusted_cost = game.adjust_mana_cost_for_payment_reason(
                    paying_player,
                    Some(ctx.source),
                    &cost,
                    crate::costs::PaymentReason::Effect,
                );
                game.can_pay_mana_cost_with_reason(
                    paying_player,
                    Some(ctx.source),
                    &adjusted_cost,
                    x_value,
                    crate::costs::PaymentReason::Effect,
                )
            };
            let can_afford_life = if life_to_pay == 0 {
                true
            } else if !game.can_lose_life(paying_player)
                || !game.can_change_life_total(paying_player)
            {
                false
            } else {
                game.player(paying_player)
                    .is_some_and(|player| player.life >= life_to_pay as i32)
            };
            let can_afford = can_afford_mana && can_afford_life;

            // Ask this player if they want to pay.
            let wants_to_pay = if can_afford {
                make_boolean_decision(
                    game,
                    &mut ctx.decision_maker,
                    paying_player,
                    ctx.source,
                    format!("Pay {} to prevent effect?", payment_display),
                    FallbackStrategy::Accept,
                )
            } else {
                false
            };

            if wants_to_pay {
                // Pay the mana/life cost; if paid successfully, prevent effects.
                let cost = ManaCost::from_symbols(mana_symbols.clone());
                let adjusted_cost = game.adjust_mana_cost_for_payment_reason(
                    paying_player,
                    Some(ctx.source),
                    &cost,
                    crate::costs::PaymentReason::Effect,
                );
                if game.try_pay_mana_cost_with_reason(
                    paying_player,
                    Some(ctx.source),
                    &adjusted_cost,
                    x_value,
                    crate::costs::PaymentReason::Effect,
                ) {
                    let mut outcome = EffectOutcome::declined();
                    if life_to_pay > 0 {
                        if let Some(player) = game.player_mut(paying_player) {
                            player.lose_life(life_to_pay);
                        }
                        outcome = outcome.with_event(TriggerEvent::new_with_provenance(
                            LifeLossEvent::from_effect(paying_player, life_to_pay),
                            ctx.provenance,
                        ));
                    }
                    return Ok(outcome);
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
    use crate::decision::SelectFirstDecisionMaker;
    use crate::effect::Effect;
    use crate::executor::ExecutionContext;
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
            .abilities
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
}
