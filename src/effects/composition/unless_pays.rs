//! "Unless pays" effect implementation.

use crate::decision::FallbackStrategy;
use crate::decisions::make_boolean_decision;
use crate::effect::{Effect, EffectOutcome, EffectResult, Value};
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
/// - If player pays: `EffectResult::Declined` (effects prevented)
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
}

impl UnlessPaysEffect {
    /// Create a new "unless pays" effect.
    pub fn new(effects: Vec<Effect>, player: PlayerFilter, mana: Vec<ManaSymbol>) -> Self {
        Self::new_with_life_and_additional(effects, player, mana, None, None)
    }

    /// Create a new "unless pays" effect with optional life payment.
    pub fn new_with_life(
        effects: Vec<Effect>,
        player: PlayerFilter,
        mana: Vec<ManaSymbol>,
        life: Option<Value>,
    ) -> Self {
        Self::new_with_life_and_additional(effects, player, mana, life, None)
    }

    /// Create a new "unless pays" effect with optional life and dynamic generic payment.
    pub fn new_with_life_and_additional(
        effects: Vec<Effect>,
        player: PlayerFilter,
        mana: Vec<ManaSymbol>,
        life: Option<Value>,
        additional_generic: Option<Value>,
    ) -> Self {
        Self {
            effects,
            player,
            mana,
            life,
            additional_generic,
        }
    }
}

fn players_in_turn_order(game: &GameState) -> Vec<PlayerId> {
    if game.turn_order.is_empty() {
        return Vec::new();
    }

    let start = game
        .turn_order
        .iter()
        .position(|&player_id| player_id == game.turn.active_player)
        .unwrap_or(0);

    (0..game.turn_order.len())
        .filter_map(|offset| {
            let player_id = game.turn_order[(start + offset) % game.turn_order.len()];
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

        let mut mana_symbols = self.mana.clone();
        if additional_generic > 0 {
            let capped = additional_generic.min(u8::MAX as u32) as u8;
            mana_symbols.push(ManaSymbol::Generic(capped));
        }

        // Format the mana cost for display
        let mana_display: String = self
            .mana
            .iter()
            .map(|s| format!("{:?}", s))
            .collect::<Vec<_>>()
            .join(", ");
        let additional_display = if additional_generic > 0 {
            format!(" plus {additional_generic} generic mana")
        } else {
            String::new()
        };
        let payment_display = if mana_display.is_empty() && additional_generic == 0 {
            if life_to_pay > 0 {
                format!("{life_to_pay} life")
            } else {
                "no cost".to_string()
            }
        } else if mana_display.is_empty() {
            if life_to_pay > 0 {
                format!("{additional_generic} generic mana and {life_to_pay} life")
            } else {
                format!("{additional_generic} generic mana")
            }
        } else if life_to_pay > 0 {
            format!("{mana_display}{additional_display} and {life_to_pay} life")
        } else {
            format!("{mana_display}{additional_display}")
        };

        for paying_player in paying_players {
            // Check if this player can afford to pay mana/life.
            let can_afford_mana = {
                let cost = ManaCost::from_symbols(mana_symbols.clone());
                game.can_pay_mana_cost(paying_player, None, &cost, 0)
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
                if game.try_pay_mana_cost(paying_player, None, &cost, 0) {
                    let mut outcome = EffectOutcome::from_result(EffectResult::Declined);
                    if life_to_pay > 0 {
                        if let Some(player) = game.player_mut(paying_player) {
                            player.lose_life(life_to_pay);
                        }
                        outcome = outcome.with_event(TriggerEvent::new(
                            LifeLossEvent::from_effect(paying_player, life_to_pay),
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
