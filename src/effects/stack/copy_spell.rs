//! Copy spell effect implementation.

use crate::effect::{EffectOutcome, EffectResult, Value};
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_objects_from_spec, resolve_value};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::{GameState, StackEntry};
use crate::object::Object;
use crate::target::ChooseSpec;
use crate::zone::Zone;

/// Effect that copies a spell on the stack.
///
/// Per Rule 707.10, when a spell is copied:
/// - The copy has the same characteristics and choices (modes, targets, X value)
/// - The copy is controlled by the player who copied it
/// - The copy is put on the stack above the original
///
/// # Fields
///
/// * `target` - The target specification for the spell to copy
/// * `count` - How many copies to create
///
/// # Example
///
/// ```ignore
/// // Copy target instant or sorcery spell
/// let effect = CopySpellEffect::new(ChooseSpec::spell(), 1);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct CopySpellEffect {
    /// The target specification (spell to copy).
    pub target: ChooseSpec,
    /// The number of copies to create.
    pub count: Value,
}

impl CopySpellEffect {
    /// Create a new copy spell effect.
    pub fn new(target: ChooseSpec, count: impl Into<Value>) -> Self {
        Self {
            target,
            count: count.into(),
        }
    }

    /// Create an effect that copies a spell once.
    pub fn single(target: ChooseSpec) -> Self {
        Self::new(target, 1)
    }
}

impl EffectExecutor for CopySpellEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let copy_count = resolve_value(game, &self.count, ctx)?.max(0) as usize;

        // Resolve the spell object to copy (source, specific, tagged, or targeted spell).
        let target_id = *resolve_objects_from_spec(game, &self.target, ctx)?
            .first()
            .ok_or(ExecutionError::InvalidTarget)?;

        // Verify target is on the stack
        let target_obj = game
            .object(target_id)
            .ok_or(ExecutionError::ObjectNotFound(target_id))?;

        if target_obj.zone != Zone::Stack {
            return Ok(EffectOutcome::from_result(EffectResult::TargetInvalid));
        }

        // Find the corresponding stack entry for this spell
        let stack_entry_opt = game
            .stack
            .iter()
            .find(|e| e.object_id == target_id)
            .cloned();

        let Some(original_entry) = stack_entry_opt else {
            return Ok(EffectOutcome::from_result(EffectResult::TargetInvalid));
        };

        let mut created_ids = Vec::with_capacity(copy_count);

        for _ in 0..copy_count {
            // Create a new object ID for the copy
            let copy_id = game.new_object_id();

            // Get fresh reference to target and create copy
            let target = game
                .object(target_id)
                .ok_or(ExecutionError::ObjectNotFound(target_id))?;
            let mut copy_obj = Object::token_copy_of(target, copy_id, ctx.controller);
            copy_obj.zone = Zone::Stack;
            // token_copy_of already sets kind to Token

            // Add the copy object
            game.add_object(copy_obj);

            // Create a new stack entry for the copy
            // The copy has the same targets, X value, etc. but is controlled by the copier
            let mut copy_entry = StackEntry::new(copy_id, ctx.controller);
            copy_entry.targets = original_entry.targets.clone();
            copy_entry.x_value = original_entry.x_value;
            copy_entry.ability_effects = original_entry.ability_effects.clone();
            copy_entry.is_ability = original_entry.is_ability;
            copy_entry.optional_costs_paid = original_entry.optional_costs_paid.clone();

            // Put the copy on top of the stack
            game.stack.push(copy_entry);
            created_ids.push(copy_id);
        }

        Ok(EffectOutcome::from_result(EffectResult::Objects(
            created_ids,
        )))
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "spell to copy"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::ids::{CardId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::types::CardType;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    fn create_instant_on_stack(
        game: &mut GameState,
        name: &str,
        controller: PlayerId,
    ) -> crate::ids::ObjectId {
        let card = CardBuilder::new(CardId::from_raw(1), name)
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Red]]))
            .card_types(vec![CardType::Instant])
            .build();

        let id = game.create_object_from_card(&card, controller, Zone::Stack);

        // Add stack entry using the constructor
        let entry = StackEntry::new(id, controller);
        game.stack.push(entry);

        id
    }

    #[test]
    fn test_copy_spell_basic() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        let spell_id = create_instant_on_stack(&mut game, "Lightning Bolt", alice);

        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.targets = vec![crate::executor::ResolvedTarget::Object(spell_id)];

        let effect = CopySpellEffect::single(ChooseSpec::spell());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Should return Objects with the copy ID
        if let EffectResult::Objects(ids) = result.result {
            assert_eq!(ids.len(), 1);
            let copy_id = ids[0];

            // Copy should be on stack
            let copy_obj = game.object(copy_id).unwrap();
            assert_eq!(copy_obj.zone, Zone::Stack);
            assert_eq!(copy_obj.name, "Lightning Bolt");
            assert_eq!(copy_obj.controller, alice);

            // Stack should have 2 entries (original + copy)
            assert_eq!(game.stack.len(), 2);

            // Copy should be on top (last in vec)
            assert_eq!(game.stack.last().unwrap().object_id, copy_id);
        } else {
            panic!("Expected Objects result");
        }
    }

    #[test]
    fn test_copy_spell_multiple() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        let spell_id = create_instant_on_stack(&mut game, "Shock", alice);

        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.targets = vec![crate::executor::ResolvedTarget::Object(spell_id)];

        let effect = CopySpellEffect::new(ChooseSpec::spell(), 3);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        if let EffectResult::Objects(ids) = result.result {
            assert_eq!(ids.len(), 3);

            // Stack should have 4 entries (original + 3 copies)
            assert_eq!(game.stack.len(), 4);

            // All copies should be on stack
            for copy_id in ids {
                let copy_obj = game.object(copy_id).unwrap();
                assert_eq!(copy_obj.zone, Zone::Stack);
                assert_eq!(copy_obj.name, "Shock");
            }
        } else {
            panic!("Expected Objects result");
        }
    }

    #[test]
    fn test_copy_spell_preserves_targets() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();

        let spell_id = create_instant_on_stack(&mut game, "Lightning Bolt", alice);

        // Set original spell's targets (using game_state::Target)
        let target = crate::game_state::Target::Player(bob);
        if let Some(entry) = game.stack.iter_mut().find(|e| e.object_id == spell_id) {
            entry.targets.push(target);
        }

        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.targets = vec![crate::executor::ResolvedTarget::Object(spell_id)];

        let effect = CopySpellEffect::single(ChooseSpec::spell());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        if let EffectResult::Objects(ids) = result.result {
            let copy_id = ids[0];

            // Copy's stack entry should have same targets
            let copy_entry = game.stack.iter().find(|e| e.object_id == copy_id).unwrap();
            assert_eq!(copy_entry.targets.len(), 1);
            assert!(matches!(
                copy_entry.targets[0],
                crate::game_state::Target::Player(p) if p == bob
            ));
        } else {
            panic!("Expected Objects result");
        }
    }

    #[test]
    fn test_copy_spell_preserves_x_value() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        let spell_id = create_instant_on_stack(&mut game, "Fireball", alice);

        // Set X value on original
        if let Some(entry) = game.stack.iter_mut().find(|e| e.object_id == spell_id) {
            entry.x_value = Some(5);
        }

        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.targets = vec![crate::executor::ResolvedTarget::Object(spell_id)];

        let effect = CopySpellEffect::single(ChooseSpec::spell());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        if let EffectResult::Objects(ids) = result.result {
            let copy_id = ids[0];

            // Copy should preserve X value
            let copy_entry = game.stack.iter().find(|e| e.object_id == copy_id).unwrap();
            assert_eq!(copy_entry.x_value, Some(5));
        } else {
            panic!("Expected Objects result");
        }
    }

    #[test]
    fn test_copy_spell_not_on_stack() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        // Create a spell NOT on stack
        let card = CardBuilder::new(CardId::from_raw(1), "Lightning Bolt")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Red]]))
            .card_types(vec![CardType::Instant])
            .build();
        let spell_id = game.create_object_from_card(&card, alice, Zone::Hand);

        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.targets = vec![crate::executor::ResolvedTarget::Object(spell_id)];

        let effect = CopySpellEffect::single(ChooseSpec::spell());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::TargetInvalid);
    }

    #[test]
    fn test_copy_spell_different_controller() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();

        // Alice's spell on stack
        let spell_id = create_instant_on_stack(&mut game, "Lightning Bolt", alice);

        // Bob copies it
        let mut ctx = ExecutionContext::new_default(source, bob);
        ctx.targets = vec![crate::executor::ResolvedTarget::Object(spell_id)];

        let effect = CopySpellEffect::single(ChooseSpec::spell());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        if let EffectResult::Objects(ids) = result.result {
            let copy_id = ids[0];

            // Copy should be controlled by Bob
            let copy_obj = game.object(copy_id).unwrap();
            assert_eq!(copy_obj.controller, bob);

            let copy_entry = game.stack.iter().find(|e| e.object_id == copy_id).unwrap();
            assert_eq!(copy_entry.controller, bob);
        } else {
            panic!("Expected Objects result");
        }
    }

    #[test]
    fn test_copy_spell_clone_box() {
        let effect = CopySpellEffect::single(ChooseSpec::spell());
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("CopySpellEffect"));
    }
}
