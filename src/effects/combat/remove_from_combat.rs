//! Remove creature from combat effect implementation.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::{ObjectApplyResultPolicy, apply_to_selected_objects};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::ChooseSpec;

/// Effect that removes creatures from combat.
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
                    let was_attacking = combat
                        .attackers
                        .iter()
                        .any(|info| info.creature == object_id);
                    let was_blocking = combat
                        .blockers
                        .values()
                        .any(|blockers| blockers.contains(&object_id));

                    if was_attacking {
                        combat.attackers.retain(|info| info.creature != object_id);
                        combat.blockers.remove(&object_id);
                        combat.damage_assignment_order.remove(&object_id);
                    }

                    if was_attacking || was_blocking {
                        for blockers in combat.blockers.values_mut() {
                            blockers.retain(|id| *id != object_id);
                        }
                        for order in combat.damage_assignment_order.values_mut() {
                            order.retain(|id| *id != object_id);
                        }
                    }

                    was_attacking || was_blocking
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
        "creature to remove from combat"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::combat_state::{AttackTarget, AttackerInfo, is_attacking, is_blocking};
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

        let attacker =
            game.create_object_from_card(&creature_card(1, "Attacker"), alice, Zone::Battlefield);
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
        let outcome = effect
            .execute(&mut game, &mut ctx)
            .expect("effect should resolve");
        assert_eq!(outcome.value, crate::effect::OutcomeValue::Count(1));
        assert!(
            !is_attacking(game.combat.as_ref().expect("combat"), attacker),
            "attacker should be removed from combat"
        );
    }

    #[test]
    fn remove_from_combat_targeted_resolves_when_target_not_attacking() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        let creature = game.create_object_from_card(
            &creature_card(2, "Idle Creature"),
            alice,
            Zone::Battlefield,
        );
        game.combat = Some(crate::combat_state::CombatState::default());

        let mut ctx = ExecutionContext::new_default(game.new_object_id(), alice)
            .with_targets(vec![ResolvedTarget::Object(creature)]);
        let effect = RemoveFromCombatEffect::target(ChooseSpec::creature());
        let outcome = effect
            .execute(&mut game, &mut ctx)
            .expect("effect should resolve");
        assert_eq!(outcome.status, crate::effect::OutcomeStatus::Succeeded);
    }

    #[test]
    fn remove_from_combat_removes_blocker() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let attacker =
            game.create_object_from_card(&creature_card(3, "Attacker"), alice, Zone::Battlefield);
        let blocker =
            game.create_object_from_card(&creature_card(4, "Blocker"), bob, Zone::Battlefield);

        let mut combat = crate::combat_state::CombatState::default();
        combat.attackers.push(AttackerInfo {
            creature: attacker,
            target: AttackTarget::Player(bob),
        });
        combat.blockers.insert(attacker, vec![blocker]);
        combat
            .damage_assignment_order
            .insert(attacker, vec![blocker]);
        game.combat = Some(combat);

        assert!(
            is_blocking(game.combat.as_ref().expect("combat"), blocker),
            "blocker should start in combat"
        );

        let mut ctx = ExecutionContext::new_default(game.new_object_id(), bob);
        let effect = RemoveFromCombatEffect::with_spec(ChooseSpec::SpecificObject(blocker));
        let outcome = effect
            .execute(&mut game, &mut ctx)
            .expect("effect should resolve");
        assert_eq!(outcome.value, crate::effect::OutcomeValue::Count(1));
        assert!(
            !is_blocking(game.combat.as_ref().expect("combat"), blocker),
            "blocker should be removed from combat"
        );
        assert_eq!(
            game.combat
                .as_ref()
                .expect("combat")
                .damage_assignment_order
                .get(&attacker)
                .cloned()
                .unwrap_or_default(),
            Vec::<crate::ids::ObjectId>::new(),
            "blocker should also be removed from damage assignment order"
        );
    }
}
