//! Suppress combat-damage assignment by a chosen object.

use crate::effect::{ChoiceCount, EffectOutcome, Until};
use crate::effects::helpers::resolve_objects_for_effect;
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::ChooseSpec;

pub use ironsmith_core::AssignNoCombatDamageEffect;

impl EffectExecutor for AssignNoCombatDamageEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        if !matches!(self.until, Until::EndOfTurn | Until::EndOfCombat) {
            return Err(ExecutionError::InternalError(format!(
                "unsupported combat-damage assignment suppression duration: {:?}",
                self.until
            )));
        }

        let sources = resolve_objects_for_effect(game, ctx, &self.source)?;
        if sources.is_empty() {
            return Err(ExecutionError::InvalidTarget);
        }
        for source in sources {
            game.suppress_combat_damage_assignment(source, self.until.clone());
        }

        Ok(EffectOutcome::resolved())
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        self.source.is_target().then_some(&self.source)
    }

    fn get_target_count(&self) -> Option<ChoiceCount> {
        self.source.is_target().then(|| self.source.count())
    }

    fn target_description(&self) -> &'static str {
        "creature that assigns no combat damage"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::combat_state::{AttackTarget, AttackerInfo, CombatState};
    use crate::ids::{CardId, PlayerId};
    use crate::types::CardType;
    use crate::zone::Zone;

    fn creature_definition(name: &str) -> crate::cards::CardDefinition {
        crate::cards::CardDefinition::new(
            CardBuilder::new(CardId::new(), name)
                .card_types(vec![CardType::Creature])
                .power_toughness(PowerToughness::fixed(3, 3))
                .build(),
        )
    }

    #[test]
    fn suppresses_assignment_instead_of_registering_prevention() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let attacker = game.create_object_from_definition(
            &creature_definition("Assignment Probe"),
            alice,
            Zone::Battlefield,
        );
        let mut ctx = ExecutionContext::new_default(attacker, alice);

        AssignNoCombatDamageEffect::new(ChooseSpec::Source, Until::EndOfTurn)
            .execute(&mut game, &mut ctx)
            .expect("assignment suppression should resolve");

        assert!(game.combat_damage_assignment_is_suppressed(attacker));
        assert!(game.effect_store.prevention_effects.shields().is_empty());

        // The distinction matters when damage cannot be prevented. Assignment
        // suppression happens before any damage event is created.
        game.effect_store
            .cant_effects
            .set_damage_cant_be_prevented(true);
        let mut combat = CombatState::default();
        combat.attackers.push(AttackerInfo {
            creature: attacker,
            target: AttackTarget::Player(bob),
        });
        combat.blockers.insert(attacker, Vec::new());
        let events = crate::execute_combat_damage_step(&mut game, &combat, false);

        assert!(events.is_empty());
        assert_eq!(game.player(bob).expect("Bob exists").life, 20);
    }

    #[test]
    fn end_of_combat_suppression_expires_without_clearing_turn_suppression() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let source = game.create_object_from_definition(
            &creature_definition("Turn Probe"),
            alice,
            Zone::Battlefield,
        );
        let combat_only = game.create_object_from_definition(
            &creature_definition("Combat Probe"),
            alice,
            Zone::Battlefield,
        );

        game.suppress_combat_damage_assignment(source, Until::EndOfTurn);
        game.suppress_combat_damage_assignment(combat_only, Until::EndOfCombat);
        game.cleanup_combat_damage_assignment_suppressions_end_of_combat();

        assert!(game.combat_damage_assignment_is_suppressed(source));
        assert!(!game.combat_damage_assignment_is_suppressed(combat_only));
    }
}
