//! Remove attacker from combat effect implementation.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::{ObjectApplyResultPolicy, apply_to_selected_objects};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::ChooseSpec;

/// Effect that removes attacking creatures from combat.
#[derive(Debug, Clone, PartialEq)]
pub struct RemoveFromCombatEffect {
    pub spec: ChooseSpec,
}

impl RemoveFromCombatEffect {
    pub fn with_spec(spec: ChooseSpec) -> Self {
        Self { spec }
    }

    pub fn target(spec: ChooseSpec) -> Self {
        Self {
            spec: ChooseSpec::target(spec),
        }
    }
}

impl EffectExecutor for RemoveFromCombatEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let result_policy = if self.spec.is_target() && self.spec.is_single() {
            ObjectApplyResultPolicy::SingleTargetResolvedOrInvalid
        } else {
            ObjectApplyResultPolicy::CountApplied
        };

        let apply_result = apply_to_selected_objects(
            game,
            ctx,
            &self.spec,
            result_policy,
            |game, _ctx, object_id| {
                let removed = if let Some(combat) = game.combat.as_mut() {
                    let was_attacking = combat.attackers.iter().any(|info| info.creature == object_id);
                    if !was_attacking {
                        false
                    } else {
                        combat.attackers.retain(|info| info.creature != object_id);
                        combat.blockers.remove(&object_id);
                        combat.damage_assignment_order.remove(&object_id);
                        true
                    }
                } else {
                    false
                };

                if removed {
                    game.ninjutsu_attack_targets.remove(&object_id);
                }

                Ok(removed)
            },
        )?;

        Ok(apply_result.outcome)
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        if self.spec.is_target() {
            Some(&self.spec)
        } else {
            None
        }
    }

    fn get_target_count(&self) -> Option<crate::effect::ChoiceCount> {
        if self.spec.is_target() {
            Some(self.spec.count())
        } else {
            None
        }
    }

    fn target_description(&self) -> &'static str {
        "attacking creature to remove from combat"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::combat_state::{AttackTarget, AttackerInfo, is_attacking};
    use crate::effect::EffectResult;
    use crate::executor::ResolvedTarget;
    use crate::ids::{CardId, PlayerId};
    use crate::types::CardType;
    use crate::zone::Zone;

    fn creature_card(id: u32, name: &str) -> crate::card::Card {
        CardBuilder::new(CardId::from_raw(id), name)
            .card_types(vec![CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(2, 2))
            .build()
    }

    #[test]
    fn remove_from_combat_removes_attacker() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let attacker = game.create_object_from_card(&creature_card(1, "Attacker"), alice, Zone::Battlefield);
        let mut combat = crate::combat_state::CombatState::default();
        combat.attackers.push(AttackerInfo {
            creature: attacker,
            target: AttackTarget::Player(bob),
        });
        combat.blockers.insert(attacker, Vec::new());
        game.combat = Some(combat);

        assert!(
            is_attacking(game.combat.as_ref().expect("combat"), attacker),
            "attacker should start in combat"
        );

        let mut ctx = ExecutionContext::new_default(game.new_object_id(), alice);
        let effect = RemoveFromCombatEffect::with_spec(ChooseSpec::SpecificObject(attacker));
        let outcome = effect.execute(&mut game, &mut ctx).expect("effect should resolve");
        assert_eq!(outcome.result, EffectResult::Count(1));
        assert!(
            !is_attacking(game.combat.as_ref().expect("combat"), attacker),
            "attacker should be removed from combat"
        );
    }

    #[test]
    fn remove_from_combat_targeted_resolves_when_target_not_attacking() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        let creature =
            game.create_object_from_card(&creature_card(2, "Idle Creature"), alice, Zone::Battlefield);
        game.combat = Some(crate::combat_state::CombatState::default());

        let mut ctx = ExecutionContext::new_default(game.new_object_id(), alice)
            .with_targets(vec![ResolvedTarget::Object(creature)]);
        let effect = RemoveFromCombatEffect::target(ChooseSpec::creature());
        let outcome = effect.execute(&mut game, &mut ctx).expect("effect should resolve");
        assert_eq!(outcome.result, EffectResult::Resolved);
    }
}
