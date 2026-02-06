//! Attach to effect implementation.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::find_target_object;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::ChooseSpec;
use crate::zone::Zone;

/// Effect that attaches the source permanent to a target permanent.
///
/// Used primarily by Auras that grant control or Equipment that auto-attach.
/// The source becomes attached to the target, and the target gains
/// the source in its attachments list.
///
/// # Fields
///
/// * `target` - The target specification for what to attach to
///
/// # Example
///
/// ```ignore
/// // Create an attach effect for an aura
/// let effect = AttachToEffect::new(ChooseSpec::target_creature());
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct AttachToEffect {
    /// The target to attach to.
    pub target: ChooseSpec,
}

impl AttachToEffect {
    /// Create a new attach to effect.
    pub fn new(target: ChooseSpec) -> Self {
        Self { target }
    }
}

impl EffectExecutor for AttachToEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let target_id = find_target_object(&ctx.targets)?;

        // If this is a spell on the stack (Aura resolving), defer attachment
        if let Some(source) = game.object(ctx.source)
            && source.zone == Zone::Stack
        {
            return Ok(EffectOutcome::resolved());
        }

        // Attach the source to the target
        if let Some(source) = game.object_mut(ctx.source) {
            source.attached_to = Some(target_id);
        }

        if let Some(target) = game.object_mut(target_id)
            && !target.attachments.contains(&ctx.source)
        {
            target.attachments.push(ctx.source);
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
        "target to attach to"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::effect::EffectResult;
    use crate::ids::{CardId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::types::{CardType, Subtype};
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    fn create_creature(game: &mut GameState, name: &str, owner: PlayerId) -> crate::ids::ObjectId {
        let card = CardBuilder::new(CardId::from_raw(1), name)
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Green],
            ]))
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Bear])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();

        game.create_object_from_card(&card, owner, Zone::Battlefield)
    }

    fn create_aura(game: &mut GameState, name: &str, owner: PlayerId) -> crate::ids::ObjectId {
        let card = CardBuilder::new(CardId::from_raw(2), name)
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::White]]))
            .card_types(vec![CardType::Enchantment])
            .subtypes(vec![Subtype::Aura])
            .build();

        game.create_object_from_card(&card, owner, Zone::Battlefield)
    }

    #[test]
    fn test_attach_to_basic() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature(&mut game, "Grizzly Bears", alice);
        let aura_id = create_aura(&mut game, "Test Aura", alice);

        let mut ctx = ExecutionContext::new_default(aura_id, alice);
        ctx.targets = vec![crate::executor::ResolvedTarget::Object(creature_id)];

        let effect = AttachToEffect::new(ChooseSpec::AnyTarget);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Resolved);

        // Verify aura is attached to creature
        let aura = game.object(aura_id).unwrap();
        assert_eq!(aura.attached_to, Some(creature_id));

        // Verify creature has the aura as attachment
        let creature = game.object(creature_id).unwrap();
        assert!(creature.attachments.contains(&aura_id));
    }

    #[test]
    fn test_attach_to_already_attached() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature(&mut game, "Grizzly Bears", alice);
        let aura_id = create_aura(&mut game, "Test Aura", alice);

        // Pre-attach the aura
        if let Some(aura) = game.object_mut(aura_id) {
            aura.attached_to = Some(creature_id);
        }
        if let Some(creature) = game.object_mut(creature_id) {
            creature.attachments.push(aura_id);
        }

        let mut ctx = ExecutionContext::new_default(aura_id, alice);
        ctx.targets = vec![crate::executor::ResolvedTarget::Object(creature_id)];

        let effect = AttachToEffect::new(ChooseSpec::AnyTarget);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Resolved);

        // Should not duplicate in attachments list
        let creature = game.object(creature_id).unwrap();
        assert_eq!(
            creature
                .attachments
                .iter()
                .filter(|&&id| id == aura_id)
                .count(),
            1
        );
    }

    #[test]
    fn test_attach_to_no_target() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let aura_id = create_aura(&mut game, "Test Aura", alice);

        let mut ctx = ExecutionContext::new_default(aura_id, alice);
        // No targets

        let effect = AttachToEffect::new(ChooseSpec::AnyTarget);
        let result = effect.execute(&mut game, &mut ctx);

        assert!(result.is_err());
    }

    #[test]
    fn test_attach_equipment() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature(&mut game, "Grizzly Bears", alice);

        // Create equipment
        let card = CardBuilder::new(CardId::from_raw(3), "Test Sword")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]]))
            .card_types(vec![CardType::Artifact])
            .subtypes(vec![Subtype::Equipment])
            .build();
        let equipment_id = game.create_object_from_card(&card, alice, Zone::Battlefield);

        let mut ctx = ExecutionContext::new_default(equipment_id, alice);
        ctx.targets = vec![crate::executor::ResolvedTarget::Object(creature_id)];

        let effect = AttachToEffect::new(ChooseSpec::AnyTarget);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Resolved);

        // Verify equipment is attached
        let equipment = game.object(equipment_id).unwrap();
        assert_eq!(equipment.attached_to, Some(creature_id));

        let creature = game.object(creature_id).unwrap();
        assert!(creature.attachments.contains(&equipment_id));
    }

    #[test]
    fn test_attach_to_clone_box() {
        let effect = AttachToEffect::new(ChooseSpec::AnyTarget);
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("AttachToEffect"));
    }
}
