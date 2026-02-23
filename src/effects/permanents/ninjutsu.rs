//! Ninjutsu keyword support.
//!
//! Ninjutsu is modeled as:
//! - a cost effect (`NinjutsuCostEffect`) that returns an unblocked attacker you control
//!   to hand and records that attack target.
//! - a resolution effect (`NinjutsuEffect`) that puts the source card from hand onto the
//!   battlefield tapped and attacking the recorded target.

use crate::combat_state::{AttackTarget, AttackerInfo, get_attack_target, is_unblocked};
use crate::decisions::make_decision;
use crate::decisions::specs::ChooseObjectsSpec;
use crate::effect::{EffectOutcome, EffectResult};
use crate::effects::zones::{
    BattlefieldEntryOptions, BattlefieldEntryOutcome, move_to_battlefield_with_options,
};
use crate::effects::{CostValidationError, EffectExecutor};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::{GameState, Phase, Step};
use crate::ids::{ObjectId, PlayerId};
use crate::types::CardType;
use crate::zone::Zone;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NinjutsuCostEffect;

impl NinjutsuCostEffect {
    pub fn new() -> Self {
        Self
    }

    fn in_ninjutsu_window(game: &GameState) -> bool {
        if game.turn.phase != Phase::Combat {
            return false;
        }
        matches!(
            game.turn.step,
            Some(Step::DeclareBlockers | Step::CombatDamage | Step::EndCombat)
        )
    }

    fn unblocked_attackers(game: &GameState, controller: PlayerId) -> Vec<ObjectId> {
        let Some(combat) = game.combat.as_ref() else {
            return Vec::new();
        };

        combat
            .attackers
            .iter()
            .filter_map(|info| {
                let creature = info.creature;
                let obj = game.object(creature)?;
                if obj.zone != Zone::Battlefield || obj.controller != controller {
                    return None;
                }
                if !is_unblocked(combat, creature) {
                    return None;
                }
                Some(creature)
            })
            .collect()
    }
}

impl EffectExecutor for NinjutsuCostEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        if !Self::in_ninjutsu_window(game) {
            return Err(ExecutionError::Impossible(
                "Ninjutsu can only be activated during combat after blockers are declared"
                    .to_string(),
            ));
        }

        let Some(source_obj) = game.object(ctx.source) else {
            return Err(ExecutionError::ObjectNotFound(ctx.source));
        };
        if source_obj.zone != Zone::Hand {
            return Err(ExecutionError::Impossible(
                "Ninjutsu source must be in hand".to_string(),
            ));
        }

        let candidates = Self::unblocked_attackers(game, ctx.controller);
        if candidates.is_empty() {
            return Err(ExecutionError::Impossible(
                "No unblocked attacker you control to return".to_string(),
            ));
        }

        let chosen = {
            let spec = ChooseObjectsSpec::new(
                ctx.source,
                "Choose an unblocked attacker you control to return to hand",
                candidates.clone(),
                1,
                Some(1),
            );
            make_decision(
                game,
                ctx.decision_maker,
                ctx.controller,
                Some(ctx.source),
                spec,
            )
        };

        let chosen_attacker = chosen
            .into_iter()
            .find(|id| candidates.contains(id))
            .or_else(|| candidates.first().copied())
            .ok_or_else(|| {
                ExecutionError::Impossible(
                    "No valid unblocked attacker was chosen for ninjutsu".to_string(),
                )
            })?;

        let attack_target = game
            .combat
            .as_ref()
            .and_then(|combat| get_attack_target(combat, chosen_attacker))
            .cloned()
            .ok_or_else(|| {
                ExecutionError::Impossible(
                    "Chosen attacker has no combat attack target".to_string(),
                )
            })?;

        if let Some(combat) = game.combat.as_mut() {
            combat
                .attackers
                .retain(|info| info.creature != chosen_attacker);
            combat.blockers.remove(&chosen_attacker);
            combat.damage_assignment_order.remove(&chosen_attacker);
            for blockers in combat.blockers.values_mut() {
                blockers.retain(|id| *id != chosen_attacker);
            }
            for order in combat.damage_assignment_order.values_mut() {
                order.retain(|id| *id != chosen_attacker);
            }
        }

        let _new_id = game
            .move_object(chosen_attacker, Zone::Hand)
            .ok_or_else(|| {
                ExecutionError::Impossible("Failed to return chosen attacker to hand".to_string())
            })?;

        game.ninjutsu_attack_targets
            .entry(ctx.source)
            .or_default()
            .push(attack_target);

        Ok(EffectOutcome::from_result(EffectResult::Resolved))
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn can_execute_as_cost(
        &self,
        game: &GameState,
        source: ObjectId,
        controller: PlayerId,
    ) -> Result<(), CostValidationError> {
        if !Self::in_ninjutsu_window(game) {
            return Err(CostValidationError::Other(
                "Ninjutsu can only be activated during combat after blockers are declared"
                    .to_string(),
            ));
        }

        let Some(source_obj) = game.object(source) else {
            return Err(CostValidationError::Other(
                "Ninjutsu source does not exist".to_string(),
            ));
        };
        if source_obj.zone != Zone::Hand {
            return Err(CostValidationError::Other(
                "Ninjutsu source must be in hand".to_string(),
            ));
        }

        if Self::unblocked_attackers(game, controller).is_empty() {
            return Err(CostValidationError::Other(
                "No unblocked attacker you control to return".to_string(),
            ));
        }

        Ok(())
    }

    fn cost_description(&self) -> Option<String> {
        Some("Return an unblocked attacker you control to hand".to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NinjutsuEffect;

impl NinjutsuEffect {
    pub fn new() -> Self {
        Self
    }

    fn pop_attack_target(game: &mut GameState, source: ObjectId) -> Option<AttackTarget> {
        let mut remove_entry = false;
        let target = game
            .ninjutsu_attack_targets
            .get_mut(&source)
            .and_then(|targets| {
                let popped = targets.pop();
                if targets.is_empty() {
                    remove_entry = true;
                }
                popped
            });
        if remove_entry {
            game.ninjutsu_attack_targets.remove(&source);
        }
        target
    }

    fn attack_target_still_valid(game: &GameState, target: &AttackTarget) -> bool {
        match target {
            AttackTarget::Player(player) => game.player(*player).is_some(),
            AttackTarget::Planeswalker(planeswalker) => {
                game.object(*planeswalker).is_some_and(|obj| {
                    obj.zone == Zone::Battlefield && obj.has_card_type(CardType::Planeswalker)
                })
            }
        }
    }
}

impl EffectExecutor for NinjutsuEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let Some(attack_target) = Self::pop_attack_target(game, ctx.source) else {
            return Ok(EffectOutcome::from_result(EffectResult::TargetInvalid));
        };

        let Some(source_obj) = game.object(ctx.source) else {
            return Ok(EffectOutcome::from_result(EffectResult::TargetInvalid));
        };
        if source_obj.zone != Zone::Hand {
            return Ok(EffectOutcome::from_result(EffectResult::TargetInvalid));
        }

        let outcome = move_to_battlefield_with_options(
            game,
            ctx,
            ctx.source,
            BattlefieldEntryOptions::specific(ctx.controller, true),
        );

        match outcome {
            BattlefieldEntryOutcome::Moved(new_id) => {
                let valid_target = Self::attack_target_still_valid(game, &attack_target);
                if let Some(combat) = game.combat.as_mut()
                    && valid_target
                {
                    combat.attackers.push(AttackerInfo {
                        creature: new_id,
                        target: attack_target,
                    });
                }
                Ok(EffectOutcome::from_result(EffectResult::Objects(vec![
                    new_id,
                ])))
            }
            BattlefieldEntryOutcome::Prevented => {
                Ok(EffectOutcome::from_result(EffectResult::Prevented))
            }
        }
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::combat_state::CombatState;
    use crate::ids::{CardId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::Object;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    fn make_creature_card(card_id: u32, name: &str) -> crate::card::Card {
        CardBuilder::new(CardId::from_raw(card_id), name)
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Blue],
            ]))
            .card_types(vec![crate::types::CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build()
    }

    fn create_creature_in_zone(
        game: &mut GameState,
        name: &str,
        owner: PlayerId,
        zone: Zone,
    ) -> ObjectId {
        let id = game.new_object_id();
        let card = make_creature_card(id.0 as u32, name);
        let obj = Object::from_card(id, &card, owner, zone);
        game.add_object(obj);
        id
    }

    #[test]
    fn ninjutsu_cost_returns_unblocked_attacker_and_records_target() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let source = create_creature_in_zone(&mut game, "Ninja", alice, Zone::Hand);
        let attacker = create_creature_in_zone(&mut game, "Attacker", alice, Zone::Battlefield);
        game.remove_summoning_sickness(attacker);

        game.turn.phase = Phase::Combat;
        game.turn.step = Some(Step::DeclareBlockers);
        game.combat = Some(CombatState {
            attackers: vec![AttackerInfo {
                creature: attacker,
                target: AttackTarget::Player(bob),
            }],
            ..CombatState::default()
        });

        let mut ctx = ExecutionContext::new_default(source, alice);
        let result = NinjutsuCostEffect::new()
            .execute(&mut game, &mut ctx)
            .expect("ninjutsu cost should resolve");

        assert!(
            matches!(result.result, EffectResult::Resolved),
            "expected resolved cost effect, got {:?}",
            result.result
        );
        assert!(
            game.combat
                .as_ref()
                .is_some_and(|combat| combat.attackers.is_empty()),
            "returned attacker should be removed from combat"
        );
        assert!(
            game.players[0]
                .hand
                .iter()
                .filter_map(|id| game.object(*id))
                .any(|obj| obj.name == "Attacker"),
            "returned attacker should be in hand"
        );
        let recorded = game
            .ninjutsu_attack_targets
            .get(&source)
            .and_then(|targets| targets.last())
            .cloned();
        assert_eq!(recorded, Some(AttackTarget::Player(bob)));
    }

    #[test]
    fn ninjutsu_effect_puts_source_tapped_and_attacking_recorded_target() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let source = create_creature_in_zone(&mut game, "Ninja", alice, Zone::Hand);
        game.ninjutsu_attack_targets
            .insert(source, vec![AttackTarget::Player(bob)]);
        game.combat = Some(CombatState::default());
        game.turn.phase = Phase::Combat;
        game.turn.step = Some(Step::CombatDamage);

        let mut ctx = ExecutionContext::new_default(source, alice);
        let result = NinjutsuEffect::new()
            .execute(&mut game, &mut ctx)
            .expect("ninjutsu effect should resolve");

        let entered = match result.result {
            EffectResult::Objects(ids) => ids[0],
            other => panic!("expected moved object result, got {other:?}"),
        };

        assert!(
            game.battlefield.contains(&entered),
            "ninjutsu source should enter the battlefield"
        );
        assert!(
            game.is_tapped(entered),
            "ninjutsu source should enter tapped"
        );
        let attackers = game
            .combat
            .as_ref()
            .map(|combat| combat.attackers.clone())
            .unwrap_or_default();
        assert!(
            attackers
                .iter()
                .any(|info| info.creature == entered && info.target == AttackTarget::Player(bob)),
            "ninjutsu source should be attacking recorded target"
        );
    }

    #[test]
    fn ninjutsu_cost_not_payable_before_blockers_declared() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let source = create_creature_in_zone(&mut game, "Ninja", alice, Zone::Hand);
        let attacker = create_creature_in_zone(&mut game, "Attacker", alice, Zone::Battlefield);
        game.remove_summoning_sickness(attacker);
        game.turn.phase = Phase::Combat;
        game.turn.step = Some(Step::DeclareAttackers);
        game.combat = Some(CombatState {
            attackers: vec![AttackerInfo {
                creature: attacker,
                target: AttackTarget::Player(bob),
            }],
            ..CombatState::default()
        });

        let can_pay = NinjutsuCostEffect::new().can_execute_as_cost(&game, source, alice);
        assert!(
            can_pay.is_err(),
            "ninjutsu should not be payable before blockers are declared"
        );
    }
}
