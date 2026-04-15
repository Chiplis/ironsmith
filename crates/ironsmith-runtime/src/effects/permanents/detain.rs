//! Detain effect implementation.

use crate::effect::{EffectOutcome, Restriction, Until};
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_objects_for_effect;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::{ChooseSpec, ObjectFilter};
use crate::zone::Zone;

/// Effect that detains permanent(s).
#[derive(Debug, Clone, PartialEq)]
pub struct DetainEffect {
    /// Permanent target specification.
    pub target: ChooseSpec,
}

impl DetainEffect {
    /// Create a new detain effect.
    pub fn new(target: ChooseSpec) -> Self {
        Self { target }
    }
}

impl EffectExecutor for DetainEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let objects = match resolve_objects_for_effect(game, ctx, &self.target) {
            Ok(objects) => objects,
            Err(ExecutionError::InvalidTarget) if !self.target.is_target() => {
                return Ok(EffectOutcome::count(0));
            }
            Err(ExecutionError::InvalidTarget) => return Ok(EffectOutcome::target_invalid()),
            Err(err) => return Err(err),
        };

        let mut count = 0_i32;
        for object_id in objects {
            let Some(object) = game.object(object_id) else {
                continue;
            };
            if object.zone != Zone::Battlefield {
                continue;
            }

            let filter = ObjectFilter::specific(object_id);
            game.add_restriction_effect(
                Restriction::attack_or_block(filter.clone()),
                Until::YourNextTurn,
                ctx.source,
                ctx.controller,
            );
            game.add_restriction_effect(
                Restriction::activate_abilities_of(filter),
                Until::YourNextTurn,
                ctx.source,
                ctx.controller,
            );
            count += 1;
        }

        game.update_cant_effects();
        Ok(EffectOutcome::count(count))
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "permanent to detain"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CardDefinitionBuilder;
    use crate::card::PowerToughness;
    use crate::effect::{OutcomeStatus, OutcomeValue};
    use crate::executor::ExecutionContext;
    use crate::ids::{CardId, PlayerId};
    use crate::target::PlayerFilter;
    use crate::types::CardType;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn create_mana_creature(
        game: &mut GameState,
        owner: PlayerId,
        name: &str,
    ) -> crate::ids::ObjectId {
        let def = CardDefinitionBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .parse_text("{T}: Add {G}.")
            .expect("creature text should parse");
        game.create_object_from_definition(&def, owner, Zone::Battlefield)
    }

    fn create_mana_artifact(
        game: &mut GameState,
        owner: PlayerId,
        name: &str,
    ) -> crate::ids::ObjectId {
        let def = CardDefinitionBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Artifact])
            .parse_text("{T}: Add {G}.")
            .expect("artifact text should parse");
        game.create_object_from_definition(&def, owner, Zone::Battlefield)
    }

    fn create_land(game: &mut GameState, owner: PlayerId, name: &str) -> crate::ids::ObjectId {
        let def = CardDefinitionBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Land])
            .parse_text("{T}: Add {G}.")
            .expect("land text should parse");
        game.create_object_from_definition(&def, owner, Zone::Battlefield)
    }

    #[test]
    fn detain_prevents_attack_block_and_activated_abilities_until_your_next_turn() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        game.turn.active_player = alice;
        game.turn.turn_number = 1;

        let source = game.new_object_id();
        let creature = create_mana_creature(&mut game, bob, "Detained Creature");

        let mut ctx = ExecutionContext::new_default(source, alice);
        let result = DetainEffect::new(ChooseSpec::SpecificObject(creature))
            .execute(&mut game, &mut ctx)
            .expect("detain should execute");

        assert_eq!(result.status, OutcomeStatus::Succeeded);
        assert_eq!(result.value, OutcomeValue::Count(1));
        assert!(!game.can_attack(creature));
        assert!(!game.can_block(creature));
        assert!(!game.can_activate_abilities_of(creature));

        game.turn.active_player = bob;
        game.turn.turn_number = 2;
        game.update_cant_effects();
        assert!(!game.can_attack(creature));
        assert!(!game.can_block(creature));
        assert!(!game.can_activate_abilities_of(creature));

        game.turn.active_player = alice;
        game.turn.turn_number = 3;
        game.update_cant_effects();
        assert!(game.can_attack(creature));
        assert!(game.can_block(creature));
        assert!(game.can_activate_abilities_of(creature));
    }

    #[test]
    fn detain_all_matching_permanents_hits_noncreatures_and_skips_lands() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let source = game.new_object_id();
        let creature = create_mana_creature(&mut game, bob, "Detained Creature");
        let artifact = create_mana_artifact(&mut game, bob, "Detained Artifact");
        let land = create_land(&mut game, bob, "Untouched Land");

        let mut ctx = ExecutionContext::new_default(source, alice);
        let result = DetainEffect::new(ChooseSpec::All(
            ObjectFilter::nonland_permanent().controlled_by(PlayerFilter::Specific(bob)),
        ))
        .execute(&mut game, &mut ctx)
        .expect("detain-all should execute");

        assert_eq!(result.status, OutcomeStatus::Succeeded);
        assert_eq!(result.value, OutcomeValue::Count(2));
        assert!(!game.can_attack(creature));
        assert!(!game.can_block(creature));
        assert!(!game.can_activate_abilities_of(creature));
        assert!(!game.can_activate_abilities_of(artifact));
        assert!(game.can_activate_abilities_of(land));
    }

    #[test]
    fn detain_all_with_no_matching_permanents_resolves_cleanly() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);
        let result = DetainEffect::new(ChooseSpec::All(
            ObjectFilter::nonland_permanent().controlled_by(PlayerFilter::Specific(alice)),
        ))
        .execute(&mut game, &mut ctx)
        .expect("detain-all with no matches should still resolve");

        assert_eq!(result.status, OutcomeStatus::Succeeded);
        assert_eq!(result.value, OutcomeValue::Count(0));
    }
}
