//! Melee keyword effect implementation.

use crate::combat_state::AttackTarget;
use crate::continuous::{EffectTarget, Modification};
use crate::effect::{Effect, EffectOutcome, Until};
use crate::effects::{ApplyContinuousEffect, EffectExecutor};
use crate::executor::{ExecutionContext, ExecutionError, execute_effect};
use crate::game_state::GameState;
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MeleeEffect;

impl MeleeEffect {
    pub fn new() -> Self {
        Self
    }
}

impl EffectExecutor for MeleeEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let source = game
            .object(ctx.source)
            .ok_or(ExecutionError::ObjectNotFound(ctx.source))?;
        let Some(combat) = game.combat.as_ref() else {
            return Ok(EffectOutcome::count(0));
        };
        let mut attacked_opponents = HashSet::new();

        for attacker in &combat.attackers {
            let Some(attacking_creature) = game.object(attacker.creature) else {
                continue;
            };
            if attacking_creature.controller != source.controller {
                continue;
            }
            match attacker.target {
                AttackTarget::Player(player) => {
                    attacked_opponents.insert(player);
                }
                AttackTarget::Planeswalker(planeswalker_id) => {
                    if let Some(planeswalker) = game.object(planeswalker_id) {
                        attacked_opponents.insert(planeswalker.controller);
                    }
                }
            }
        }

        attacked_opponents.remove(&source.controller);
        let amount = attacked_opponents.len() as i32;
        if amount <= 0 {
            return Ok(EffectOutcome::count(0));
        }

        let apply = ApplyContinuousEffect::new(
            EffectTarget::Specific(ctx.source),
            Modification::ModifyPowerToughness {
                power: amount,
                toughness: amount,
            },
            Until::EndOfTurn,
        );
        execute_effect(game, &Effect::new(apply), ctx)?;
        Ok(EffectOutcome::count(amount))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::combat_state::{AttackTarget, AttackerInfo, CombatState};
    use crate::executor::ExecutionContext;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::Object;
    use crate::types::CardType;
    use crate::zone::Zone;

    fn create_creature(
        game: &mut GameState,
        name: &str,
        controller: PlayerId,
        power: i32,
        toughness: i32,
    ) -> ObjectId {
        let card = CardBuilder::new(CardId::new(), name)
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(power, toughness))
            .build();
        let id = game.new_object_id();
        game.add_object(Object::from_card(id, &card, controller, Zone::Battlefield));
        id
    }

    #[test]
    fn melee_counts_each_opponent_you_attacked_this_combat() {
        let mut game = GameState::new(
            vec!["Alice".to_string(), "Bob".to_string(), "Cara".to_string()],
            20,
        );
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let cara = PlayerId::from_index(2);

        let melee_creature = create_creature(&mut game, "Melee Probe", alice, 2, 2);
        let support_attacker = create_creature(&mut game, "Support Probe", alice, 2, 2);
        game.combat = Some(CombatState {
            attackers: vec![
                AttackerInfo {
                    creature: melee_creature,
                    target: AttackTarget::Player(bob),
                },
                AttackerInfo {
                    creature: support_attacker,
                    target: AttackTarget::Player(cara),
                },
            ],
            ..CombatState::default()
        });

        let source = melee_creature;
        let mut dm = crate::decision::SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);
        MeleeEffect::new()
            .execute(&mut game, &mut ctx)
            .expect("melee should resolve");

        assert_eq!(game.current_power(melee_creature), Some(4));
        assert_eq!(game.current_toughness(melee_creature), Some(4));
    }
}
