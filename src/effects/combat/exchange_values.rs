//! Exchange numerical values effect implementation.

use crate::continuous::{EffectTarget, Modification, PtSublayer};
use crate::effect::{Effect, EffectOutcome, Until, Value};
use crate::effects::helpers::{resolve_player_filter, resolve_single_object_for_effect};
use crate::effects::{ApplyContinuousEffect, EffectExecutor};
use crate::event_processor::process_life_gain_with_event;
use crate::executor::{ExecutionContext, ExecutionError, execute_effect};
use crate::game_state::GameState;
use crate::target::{ChooseSpec, PlayerFilter};
use crate::triggers::TriggerEvent;
use crate::types::CardType;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExchangeValueKind {
    Power,
    Toughness,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExchangeValueOperand {
    LifeTotal(PlayerFilter),
    Power(ChooseSpec),
    Toughness(ChooseSpec),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExchangeValuesEffect {
    pub left: ExchangeValueOperand,
    pub right: ExchangeValueOperand,
    pub duration: Until,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResolvedExchangeValue {
    LifeTotal {
        player: crate::ids::PlayerId,
        value: i32,
    },
    Stat {
        object: crate::ids::ObjectId,
        kind: ExchangeValueKind,
        value: i32,
    },
}

impl ExchangeValuesEffect {
    pub fn new(left: ExchangeValueOperand, right: ExchangeValueOperand, duration: Until) -> Self {
        Self {
            left,
            right,
            duration,
        }
    }

    fn resolve_operand(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        operand: &ExchangeValueOperand,
    ) -> Result<Option<ResolvedExchangeValue>, ExecutionError> {
        match operand {
            ExchangeValueOperand::LifeTotal(player) => {
                let player_id = match resolve_player_filter(game, player, ctx) {
                    Ok(player_id) => player_id,
                    Err(ExecutionError::InvalidTarget) => return Ok(None),
                    Err(err) => return Err(err),
                };
                let Some(player_state) = game.player(player_id) else {
                    return Ok(None);
                };
                Ok(Some(ResolvedExchangeValue::LifeTotal {
                    player: player_id,
                    value: player_state.life,
                }))
            }
            ExchangeValueOperand::Power(target) => {
                self.resolve_stat_operand(game, ctx, target, ExchangeValueKind::Power)
            }
            ExchangeValueOperand::Toughness(target) => {
                self.resolve_stat_operand(game, ctx, target, ExchangeValueKind::Toughness)
            }
        }
    }

    fn resolve_stat_operand(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        target: &ChooseSpec,
        kind: ExchangeValueKind,
    ) -> Result<Option<ResolvedExchangeValue>, ExecutionError> {
        let object_id = match resolve_single_object_for_effect(game, ctx, target) {
            Ok(object_id) => object_id,
            Err(ExecutionError::InvalidTarget) => return Ok(None),
            Err(err) => return Err(err),
        };
        let Some(object) = game.object(object_id) else {
            return Ok(None);
        };
        if !object.has_card_type(CardType::Creature) {
            return Ok(None);
        }
        let value = match kind {
            ExchangeValueKind::Power => game.calculated_power(object_id).or_else(|| object.power()),
            ExchangeValueKind::Toughness => game
                .calculated_toughness(object_id)
                .or_else(|| object.toughness()),
        }
        .unwrap_or(0);
        Ok(Some(ResolvedExchangeValue::Stat {
            object: object_id,
            kind,
            value,
        }))
    }

    fn can_apply_life_exchange(
        game: &GameState,
        current: ResolvedExchangeValue,
        next_value: i32,
    ) -> bool {
        match current {
            ResolvedExchangeValue::LifeTotal { player, value } => {
                if next_value > value {
                    game.can_gain_life(player)
                } else if next_value < value {
                    game.can_lose_life(player)
                } else {
                    true
                }
            }
            ResolvedExchangeValue::Stat { .. } => true,
        }
    }

    fn apply_resolved_value(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        current: ResolvedExchangeValue,
        next_value: i32,
    ) -> Result<EffectOutcome, ExecutionError> {
        match current {
            ResolvedExchangeValue::LifeTotal { player, value } => {
                Self::apply_life_total_change(game, ctx, player, value, next_value)
            }
            ResolvedExchangeValue::Stat {
                object,
                kind,
                value,
            } => {
                if value == next_value {
                    return Ok(EffectOutcome::resolved());
                }
                let modification = match kind {
                    ExchangeValueKind::Power => Modification::SetPower {
                        value: Value::Fixed(next_value),
                        sublayer: PtSublayer::Setting,
                    },
                    ExchangeValueKind::Toughness => Modification::SetToughness {
                        value: Value::Fixed(next_value),
                        sublayer: PtSublayer::Setting,
                    },
                };
                let apply = ApplyContinuousEffect::new(
                    EffectTarget::Specific(object),
                    modification,
                    self.duration.clone(),
                );
                execute_effect(game, &Effect::new(apply), ctx)
            }
        }
    }

    fn apply_life_total_change(
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        player: crate::ids::PlayerId,
        current: i32,
        next_value: i32,
    ) -> Result<EffectOutcome, ExecutionError> {
        if current == next_value {
            return Ok(EffectOutcome::resolved());
        }

        let mut outcome = EffectOutcome::resolved();
        if next_value > current {
            let gained = process_life_gain_with_event(game, player, (next_value - current) as u32);
            if gained > 0
                && let Some(player_state) = game.player_mut(player)
            {
                player_state.gain_life(gained);
            }
            if gained > 0 {
                outcome = outcome.with_event(TriggerEvent::new_with_provenance(
                    crate::events::LifeGainEvent::new(player, gained),
                    ctx.provenance,
                ));
            }
        } else {
            let lost = (current - next_value) as u32;
            if let Some(player_state) = game.player_mut(player) {
                player_state.lose_life(lost);
            }
            if lost > 0 {
                outcome = outcome.with_event(TriggerEvent::new_with_provenance(
                    crate::events::LifeLossEvent::from_effect(player, lost),
                    ctx.provenance,
                ));
            }
        }

        Ok(outcome)
    }
}

impl EffectExecutor for ExchangeValuesEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let Some(left) = self.resolve_operand(game, ctx, &self.left)? else {
            return Ok(EffectOutcome::target_invalid());
        };
        let Some(right) = self.resolve_operand(game, ctx, &self.right)? else {
            return Ok(EffectOutcome::target_invalid());
        };

        let left_value = match left {
            ResolvedExchangeValue::LifeTotal { value, .. }
            | ResolvedExchangeValue::Stat { value, .. } => value,
        };
        let right_value = match right {
            ResolvedExchangeValue::LifeTotal { value, .. }
            | ResolvedExchangeValue::Stat { value, .. } => value,
        };

        if left_value == right_value {
            return Ok(EffectOutcome::resolved());
        }

        if !Self::can_apply_life_exchange(game, left, right_value)
            || !Self::can_apply_life_exchange(game, right, left_value)
        {
            return Ok(EffectOutcome::prevented());
        }

        let outcomes = vec![
            self.apply_resolved_value(game, ctx, left, right_value)?,
            self.apply_resolved_value(game, ctx, right, left_value)?,
        ];
        Ok(EffectOutcome::aggregate(outcomes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::executor::ResolvedTarget;
    use crate::ids::{CardId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::Object;
    use crate::zone::Zone;

    fn make_creature_card(
        card_id: u32,
        name: &str,
        power: i32,
        toughness: i32,
    ) -> crate::card::Card {
        CardBuilder::new(CardId::from_raw(card_id), name)
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(power, toughness))
            .build()
    }

    fn create_creature(
        game: &mut GameState,
        name: &str,
        controller: PlayerId,
        power: i32,
        toughness: i32,
    ) -> crate::ids::ObjectId {
        let id = game.new_object_id();
        let card = make_creature_card(id.0 as u32, name, power, toughness);
        let object = Object::from_card(id, &card, controller, Zone::Battlefield);
        game.add_object(object);
        id
    }

    #[test]
    fn exchange_values_swaps_life_total_and_source_toughness() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        game.player_mut(alice).expect("alice exists").life = 7;
        let source = create_creature(&mut game, "Tree", alice, 0, 13);
        let mut ctx = ExecutionContext::new_default(source, alice);

        let outcome = ExchangeValuesEffect::new(
            ExchangeValueOperand::LifeTotal(PlayerFilter::You),
            ExchangeValueOperand::Toughness(ChooseSpec::Source),
            Until::Forever,
        )
        .execute(&mut game, &mut ctx)
        .expect("exchange should resolve");

        assert_eq!(outcome.status, crate::effect::OutcomeStatus::Succeeded);
        assert_eq!(game.player(alice).expect("alice exists").life, 13);
        assert_eq!(game.calculated_toughness(source), Some(7));
    }

    #[test]
    fn exchange_values_swaps_source_power_with_target_power_until_end_of_combat() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = create_creature(&mut game, "Serene Master", alice, 0, 2);
        let target = create_creature(&mut game, "Attacker", bob, 5, 5);
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(target)]);

        let outcome = ExchangeValuesEffect::new(
            ExchangeValueOperand::Power(ChooseSpec::Source),
            ExchangeValueOperand::Power(ChooseSpec::target(ChooseSpec::creature())),
            Until::EndOfCombat,
        )
        .execute(&mut game, &mut ctx)
        .expect("exchange should resolve");

        assert_eq!(outcome.status, crate::effect::OutcomeStatus::Succeeded);
        assert_eq!(game.calculated_power(source), Some(5));
        assert_eq!(game.calculated_power(target), Some(0));
    }

    #[test]
    fn exchange_values_is_all_or_nothing_when_player_cant_lose_life() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = create_creature(&mut game, "Tree", alice, 0, 13);
        game.player_mut(alice).expect("alice exists").life = 20;
        game.cant_effects.add_life_total_cant_change(alice);
        let mut ctx = ExecutionContext::new_default(source, alice);

        let outcome = ExchangeValuesEffect::new(
            ExchangeValueOperand::LifeTotal(PlayerFilter::You),
            ExchangeValueOperand::Toughness(ChooseSpec::Source),
            Until::Forever,
        )
        .execute(&mut game, &mut ctx)
        .expect("exchange should resolve");

        assert_eq!(outcome.status, crate::effect::OutcomeStatus::Prevented);
        assert_eq!(game.player(alice).expect("alice exists").life, 20);
        assert_eq!(game.calculated_toughness(source), Some(13));
    }
}
