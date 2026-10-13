//! Melee keyword effect implementation.

use crate::continuous::{EffectTarget, Modification};
use crate::effect::{EffectOutcome, Until};
use crate::effects::{ApplyContinuousEffect, EffectExecutor};
use crate::effects::{CompletedEffectOutputs, ExecutionContext, ExecutionError};
use crate::game_state::GameState;

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
        self.execute_with_outputs(game, ctx)
            .map(CompletedEffectOutputs::into_outcome)
    }
    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<CompletedEffectOutputs, ExecutionError> {
        let source = game
            .object(ctx.source)
            .ok_or(ExecutionError::ObjectNotFound(ctx.source))?;
        if game.combat.is_none() {
            return Ok(CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }
        let controller = game.controller_of(source);
        let amount = crate::effects::helpers::resolve_value(
            game,
            &crate::effect::Value::TurnHistoryCount(
                crate::effect::TurnHistoryCount::PlayersAttackedThisCombat(
                    crate::target::PlayerFilter::Specific(controller),
                ),
            ),
            ctx,
        )?;
        if amount <= 0 {
            return Ok(CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }

        let apply = ApplyContinuousEffect::new(
            EffectTarget::Specific(ctx.source),
            Modification::ModifyPowerToughness {
                power: amount,
                toughness: amount,
            },
            Until::EndOfTurn,
        );
        let child = apply.execute_child_with_outputs(game, ctx)?;
        Ok(CompletedEffectOutputs::with_primary_result(
            EffectOutcome::count(amount),
            [child],
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::combat_state::{AttackTarget, AttackerInfo, CombatState};
    use crate::effects::ExecutionContext;
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
        // Attack declaration records the attacked players (CR 702.121a).
        game.turn_store
            .turn_history
            .players_attacked_in_combat
            .insert(
                (game.turn_store.combat_phases_started_this_turn, alice),
                [bob, cara].into_iter().collect(),
            );

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
