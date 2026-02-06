//! Create emblem effect implementation.

use crate::ability::Ability;
use crate::effect::{EffectOutcome, EffectResult, EmblemDescription};
use crate::effects::EffectExecutor;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::object::Object;
use crate::zone::Zone;

/// Effect that creates an emblem for the controller.
///
/// Emblems are permanent game objects created by planeswalker ultimates.
/// They exist in the command zone and have abilities that function from there.
/// Emblems cannot be removed once created.
///
/// # Fields
///
/// * `emblem` - Description of the emblem to create
///
/// # Example
///
/// ```ignore
/// // Create an emblem with a static ability
/// let emblem = EmblemDescription::new("Elspeth", "Creatures you control get +2/+2")
///     .with_ability(ability);
/// let effect = CreateEmblemEffect::new(emblem);
/// ```
#[derive(Debug, Clone)]
pub struct CreateEmblemEffect {
    /// The emblem description.
    pub emblem: EmblemDescription,
}

impl CreateEmblemEffect {
    /// Create a new create emblem effect.
    pub fn new(emblem: EmblemDescription) -> Self {
        Self { emblem }
    }
}

impl EffectExecutor for CreateEmblemEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let id = game.new_object_id();

        // Convert emblem abilities to have Command zone as their functional zone
        let abilities: Vec<Ability> = self
            .emblem
            .abilities
            .iter()
            .map(|ab| ab.clone().in_zones(vec![Zone::Command]))
            .collect();

        // Create emblem using the proper constructor
        let emblem_obj =
            Object::new_emblem(id, ctx.controller, self.emblem.name.clone(), abilities);

        // add_object handles adding to command_zone for Zone::Command
        game.add_object(emblem_obj);

        Ok(EffectOutcome::from_result(EffectResult::Objects(vec![id])))
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;
    use crate::ids::PlayerId;
    use crate::static_abilities::StaticAbility;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    #[test]
    fn test_create_emblem_basic() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let emblem = EmblemDescription::new("Test Emblem", "You have hexproof.");
        let effect = CreateEmblemEffect::new(emblem);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Should return Objects with the emblem ID
        if let EffectResult::Objects(ids) = result.result {
            assert_eq!(ids.len(), 1);
            let emblem_id = ids[0];

            // Emblem should be in command zone
            assert!(game.command_zone.contains(&emblem_id));

            // Check emblem properties
            let obj = game.object(emblem_id).unwrap();
            assert_eq!(obj.name, "Test Emblem");
            assert_eq!(obj.zone, Zone::Command);
            assert_eq!(obj.controller, alice);
        } else {
            panic!("Expected Objects result");
        }
    }

    #[test]
    fn test_create_emblem_with_ability() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        // Create an emblem with a static ability
        let ability = Ability {
            kind: AbilityKind::Static(StaticAbility::hexproof()),
            functional_zones: vec![Zone::Battlefield], // Will be converted to Command
            text: Some("You have hexproof.".to_string()),
        };
        let emblem =
            EmblemDescription::new("Teferi Emblem", "You have hexproof.").with_ability(ability);

        let effect = CreateEmblemEffect::new(emblem);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        if let EffectResult::Objects(ids) = result.result {
            let emblem_id = ids[0];
            let obj = game.object(emblem_id).unwrap();

            // Ability should be present with Command zone functional
            assert_eq!(obj.abilities.len(), 1);
            assert!(obj.abilities[0].functional_zones.contains(&Zone::Command));
        } else {
            panic!("Expected Objects result");
        }
    }

    #[test]
    fn test_create_emblem_controller() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();

        // Bob creates the emblem (as controller)
        let mut ctx = ExecutionContext::new_default(source, bob);

        let emblem = EmblemDescription::new("Bob's Emblem", "Test");
        let effect = CreateEmblemEffect::new(emblem);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        if let EffectResult::Objects(ids) = result.result {
            let emblem_id = ids[0];
            let obj = game.object(emblem_id).unwrap();
            // Controller should be Bob
            assert_eq!(obj.controller, bob);
            // Alice doesn't control it
            assert_ne!(obj.controller, alice);
        } else {
            panic!("Expected Objects result");
        }
    }

    #[test]
    fn test_create_emblem_clone_box() {
        let emblem = EmblemDescription::new("Test", "Text");
        let effect = CreateEmblemEffect::new(emblem);
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("CreateEmblemEffect"));
    }
}
