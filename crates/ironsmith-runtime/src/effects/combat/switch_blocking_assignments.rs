//! Switch blocking assignments between two attacking creatures.

use std::collections::HashMap;

use crate::combat_state::CombatState;
use crate::effect::EffectOutcome;
use crate::effects::helpers::resolve_objects_for_effect;
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::ObjectId;
use crate::rules::combat::can_block;
use crate::target::ChooseSpec;

pub use ironsmith_core::SwitchBlockingAssignmentsEffect;

impl EffectExecutor for SwitchBlockingAssignmentsEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let attackers = resolve_objects_for_effect(game, ctx, &self.attackers)?;
        let [first, second] = attackers.as_slice() else {
            return Ok(EffectOutcome::target_invalid());
        };

        let Some(combat) = game.combat.as_ref() else {
            return Ok(EffectOutcome::target_invalid());
        };
        if !is_blocked_attacker(combat, *first) || !is_blocked_attacker(combat, *second) {
            return Ok(EffectOutcome::target_invalid());
        }

        let first_blockers = combat.blockers.get(first).cloned().unwrap_or_default();
        let second_blockers = combat.blockers.get(second).cloned().unwrap_or_default();
        if !blockers_can_block_attacker(game, *first, &second_blockers)
            || !blockers_can_block_attacker(game, *second, &first_blockers)
        {
            return Ok(EffectOutcome::count(0));
        }

        let first_only = first_blockers
            .iter()
            .copied()
            .filter(|blocker| !second_blockers.contains(blocker))
            .collect::<Vec<_>>();
        let second_only = second_blockers
            .iter()
            .copied()
            .filter(|blocker| !first_blockers.contains(blocker))
            .collect::<Vec<_>>();
        let moved = first_only.len() + second_only.len();

        let Some(combat) = game.combat.as_mut() else {
            return Ok(EffectOutcome::target_invalid());
        };
        switch_assignments(&mut combat.blockers, *first, *second, &first_only, &second_only);
        switch_assignments(
            &mut combat.damage_assignment_order,
            *first,
            *second,
            &first_only,
            &second_only,
        );

        Ok(EffectOutcome::count(moved as i32))
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.attackers)
    }

    fn target_description(&self) -> &'static str {
        "two blocked attacking creatures"
    }
}

fn is_blocked_attacker(combat: &CombatState, attacker: ObjectId) -> bool {
    combat
        .attackers
        .iter()
        .any(|info| info.creature == attacker)
        && combat
            .blockers
            .get(&attacker)
            .is_some_and(|blockers| !blockers.is_empty())
}

fn blockers_can_block_attacker(game: &GameState, attacker: ObjectId, blockers: &[ObjectId]) -> bool {
    let Some(attacker) = game.object(attacker) else {
        return false;
    };
    blockers.iter().all(|blocker| {
        game.object(*blocker)
            .is_some_and(|blocker| can_block(attacker, blocker, game))
    })
}

fn switch_assignments(
    assignments: &mut HashMap<ObjectId, Vec<ObjectId>>,
    first: ObjectId,
    second: ObjectId,
    first_only: &[ObjectId],
    second_only: &[ObjectId],
) {
    let first_assignments = assignments.entry(first).or_default();
    first_assignments.retain(|blocker| !first_only.contains(blocker));
    for blocker in second_only {
        if !first_assignments.contains(blocker) {
            first_assignments.push(*blocker);
        }
    }

    let second_assignments = assignments.entry(second).or_default();
    second_assignments.retain(|blocker| !second_only.contains(blocker));
    for blocker in first_only {
        if !second_assignments.contains(blocker) {
            second_assignments.push(*blocker);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat_state::{AttackTarget, AttackerInfo};
    use crate::ability::{Ability, AbilityKind};
    use crate::card::{CardBuilder, PowerToughness};
    use crate::effects::ResolvedTarget;
    use crate::ids::{CardId, PlayerId};
    use crate::snapshot::ObjectSnapshot;
    use crate::static_abilities::StaticAbility;
    use crate::tag::TagKey;
    use crate::types::CardType;
    use crate::zone::Zone;

    fn creature_card(id: u32, name: &str) -> crate::card::Card {
        CardBuilder::new(CardId::from_raw(id), name)
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build()
    }

    fn create_creature(game: &mut GameState, id: u32, name: &str, controller: PlayerId) -> ObjectId {
        game.create_object_from_card(&creature_card(id, name), controller, Zone::Battlefield)
    }

    fn tag_attackers(game: &GameState, attackers: &[ObjectId]) -> Vec<ObjectSnapshot> {
        attackers
            .iter()
            .filter_map(|attacker| game.object(*attacker))
            .map(|object| ObjectSnapshot::from_object(object, game))
            .collect()
    }

    #[test]
    fn general_jarkeld_effect_switches_exclusive_blockers_between_two_attackers() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let jarkeld = create_creature(&mut game, 1, "General Jarkeld", alice);
        let first_attacker = create_creature(&mut game, 2, "First Attacker", alice);
        let second_attacker = create_creature(&mut game, 3, "Second Attacker", alice);
        let first_blocker = create_creature(&mut game, 4, "First Blocker", bob);
        let second_blocker = create_creature(&mut game, 5, "Second Blocker", bob);
        let shared_blocker = create_creature(&mut game, 6, "Shared Blocker", bob);

        let mut combat = CombatState::default();
        combat.attackers.push(AttackerInfo {
            creature: first_attacker,
            target: AttackTarget::Player(bob),
        });
        combat.attackers.push(AttackerInfo {
            creature: second_attacker,
            target: AttackTarget::Player(bob),
        });
        combat
            .blockers
            .insert(first_attacker, vec![first_blocker, shared_blocker]);
        combat
            .blockers
            .insert(second_attacker, vec![second_blocker, shared_blocker]);
        combat
            .damage_assignment_order
            .insert(first_attacker, vec![first_blocker, shared_blocker]);
        combat
            .damage_assignment_order
            .insert(second_attacker, vec![second_blocker, shared_blocker]);
        game.combat = Some(combat);

        let tag = TagKey::from("general_jarkeld_targets");
        let mut ctx = ExecutionContext::new_default(jarkeld, alice)
            .with_targets(vec![
                ResolvedTarget::Object(first_attacker),
                ResolvedTarget::Object(second_attacker),
            ])
            .with_tagged_objects([(tag.clone(), tag_attackers(&game, &[first_attacker, second_attacker]))].into());
        let effect = SwitchBlockingAssignmentsEffect::new(ChooseSpec::Tagged(tag));

        effect
            .execute(&mut game, &mut ctx)
            .expect("General Jarkeld switch effect should resolve");

        let combat = game.combat.as_ref().expect("combat should remain active");
        assert_eq!(
            combat.blockers.get(&first_attacker),
            Some(&vec![shared_blocker, second_blocker])
        );
        assert_eq!(
            combat.blockers.get(&second_attacker),
            Some(&vec![shared_blocker, first_blocker])
        );
        assert_eq!(
            combat.damage_assignment_order.get(&first_attacker),
            Some(&vec![shared_blocker, second_blocker])
        );
        assert_eq!(
            combat.damage_assignment_order.get(&second_attacker),
            Some(&vec![shared_blocker, first_blocker])
        );
    }

    #[test]
    fn general_jarkeld_effect_does_not_switch_when_a_blocker_could_not_block_the_other_attacker() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let jarkeld = create_creature(&mut game, 11, "General Jarkeld", alice);
        let ground_attacker = create_creature(&mut game, 12, "Ground Attacker", alice);
        let flying_attacker = create_creature(&mut game, 13, "Flying Attacker", alice);
        game.object_mut(flying_attacker)
            .expect("flying attacker exists")
            .abilities
            .push(Ability {
                kind: AbilityKind::Static(StaticAbility::flying()),
                functional_zones: vec![Zone::Battlefield],
            });
        let ground_blocker = create_creature(&mut game, 14, "Ground Blocker", bob);
        let flying_blocker = create_creature(&mut game, 15, "Flying Blocker", bob);
        game.object_mut(flying_blocker)
            .expect("flying blocker exists")
            .abilities
            .push(Ability {
                kind: AbilityKind::Static(StaticAbility::flying()),
                functional_zones: vec![Zone::Battlefield],
            });

        let mut combat = CombatState::default();
        combat.attackers.push(AttackerInfo {
            creature: ground_attacker,
            target: AttackTarget::Player(bob),
        });
        combat.attackers.push(AttackerInfo {
            creature: flying_attacker,
            target: AttackTarget::Player(bob),
        });
        combat.blockers.insert(ground_attacker, vec![ground_blocker]);
        combat.blockers.insert(flying_attacker, vec![flying_blocker]);
        combat
            .damage_assignment_order
            .insert(ground_attacker, vec![ground_blocker]);
        combat
            .damage_assignment_order
            .insert(flying_attacker, vec![flying_blocker]);
        game.combat = Some(combat);

        let tag = TagKey::from("general_jarkeld_targets");
        let mut ctx = ExecutionContext::new_default(jarkeld, alice)
            .with_targets(vec![
                ResolvedTarget::Object(ground_attacker),
                ResolvedTarget::Object(flying_attacker),
            ])
            .with_tagged_objects([(tag.clone(), tag_attackers(&game, &[ground_attacker, flying_attacker]))].into());
        let effect = SwitchBlockingAssignmentsEffect::new(ChooseSpec::Tagged(tag));

        effect
            .execute(&mut game, &mut ctx)
            .expect("General Jarkeld condition should resolve as no-op");

        let combat = game.combat.as_ref().expect("combat should remain active");
        assert_eq!(combat.blockers.get(&ground_attacker), Some(&vec![ground_blocker]));
        assert_eq!(combat.blockers.get(&flying_attacker), Some(&vec![flying_blocker]));
    }
}
