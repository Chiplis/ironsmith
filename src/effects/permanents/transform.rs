//! Transform effect implementation.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::find_target_object;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::ChooseSpec;

/// Effect that transforms a double-faced permanent.
///
/// Toggles the face state of a DFC (double-faced card).
/// When face_down is false, the card shows its front face.
/// When face_down is true, the card shows its back face.
///
/// # Fields
///
/// * `target` - The permanent to transform
///
/// # Example
///
/// ```ignore
/// // Transform target permanent
/// let effect = TransformEffect::new(ChooseSpec::permanent());
///
/// // Transform this permanent (the source)
/// let effect = TransformEffect::source();
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct TransformEffect {
    /// The targeting specification.
    pub target: ChooseSpec,
}

impl TransformEffect {
    /// Create a new transform effect.
    pub fn new(target: ChooseSpec) -> Self {
        Self { target }
    }

    /// Create an effect that transforms the source permanent.
    pub fn source() -> Self {
        Self::new(ChooseSpec::Source)
    }

    /// Create an effect that transforms target permanent.
    pub fn target_permanent() -> Self {
        Self::new(ChooseSpec::permanent())
    }
}

impl EffectExecutor for TransformEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let target_id = match &self.target {
            ChooseSpec::Source => ctx.source,
            ChooseSpec::SpecificObject(id) => *id,
            _ => find_target_object(&ctx.targets)?,
        };

        // Toggle the face state (for DFCs, face_down = back face)
        if game.is_face_down(target_id) {
            game.set_face_up(target_id);
        } else {
            game.set_face_down(target_id);
        }

        Ok(EffectOutcome::resolved())
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "permanent to transform"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::effect::EffectResult;
    use crate::executor::ResolvedTarget;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::object::Object;
    use crate::target::ObjectFilter;
    use crate::types::CardType;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    fn create_dfc(game: &mut GameState, name: &str, controller: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .card_types(vec![CardType::Creature])
            .build();
        let obj = Object::from_card(id, &card, controller, Zone::Battlefield);
        game.add_object(obj);
        id
    }

    #[test]
    fn test_transform_basic() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_dfc(&mut game, "Werewolf", alice);

        // Initially not face down (showing front face)
        assert!(!game.is_face_down(creature_id));

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let effect = TransformEffect::new(ChooseSpec::Object(ObjectFilter::creature()));
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Resolved);
        // Should now be face down (showing back face)
        assert!(game.is_face_down(creature_id));
    }

    #[test]
    fn test_transform_toggles_back() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_dfc(&mut game, "Werewolf", alice);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let effect = TransformEffect::new(ChooseSpec::Object(ObjectFilter::creature()));

        // Transform once
        effect.execute(&mut game, &mut ctx).unwrap();
        assert!(game.is_face_down(creature_id));

        // Transform again - should toggle back
        effect.execute(&mut game, &mut ctx).unwrap();
        assert!(!game.is_face_down(creature_id));
    }

    #[test]
    fn test_transform_source() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_dfc(&mut game, "Self-transforming", alice);

        // Source is the creature itself
        let mut ctx = ExecutionContext::new_default(creature_id, alice);

        let effect = TransformEffect::source();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Resolved);
        assert!(game.is_face_down(creature_id));
    }

    #[test]
    fn test_transform_no_target() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = TransformEffect::target_permanent();
        let result = effect.execute(&mut game, &mut ctx);

        assert!(result.is_err());
    }

    #[test]
    fn test_transform_clone_box() {
        let effect = TransformEffect::target_permanent();
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("TransformEffect"));
    }

    #[test]
    fn test_transform_get_target_spec() {
        let effect = TransformEffect::target_permanent();
        assert!(effect.get_target_spec().is_some());
    }
}
