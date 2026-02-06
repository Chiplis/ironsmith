//! Counter spell effect implementation.

use crate::ability::AbilityKind;
use crate::effect::{EffectOutcome, EffectResult};
use crate::effects::EffectExecutor;
use crate::effects::helpers::find_target_object;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::ChooseSpec;
use crate::zone::Zone;

/// Effect that counters a target spell on the stack.
///
/// This removes the spell from the stack and puts it into its owner's graveyard.
/// Abilities that are countered simply disappear.
///
/// # Fields
///
/// * `target` - Which spell to counter (resolved from ctx.targets)
///
/// # Example
///
/// ```ignore
/// // Counter target spell
/// let effect = CounterEffect::new(ChooseSpec::spell());
///
/// // Counter target creature spell
/// let effect = CounterEffect::new(ChooseSpec::creature_spell());
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct CounterEffect {
    /// The targeting specification (for UI/validation purposes).
    pub target: ChooseSpec,
}

impl CounterEffect {
    /// Create a new counter effect.
    pub fn new(target: ChooseSpec) -> Self {
        Self { target }
    }

    /// Create an effect that counters any spell.
    pub fn any_spell() -> Self {
        Self::new(ChooseSpec::spell())
    }
}

impl EffectExecutor for CounterEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        // Counter removes the spell/ability from the stack
        // Find the target in our resolved targets
        let target_id = find_target_object(&ctx.targets)?;

        // Check if the spell can't be countered
        if let Some(obj) = game.object(target_id) {
            let cant_be_countered = obj.abilities.iter().any(|ability| {
                if let AbilityKind::Static(s) = &ability.kind {
                    s.cant_be_countered()
                } else {
                    false
                }
            });
            if cant_be_countered {
                // Spell can't be countered - effect does nothing
                return Ok(EffectOutcome::from_result(EffectResult::Protected));
            }
        }

        // Find the stack entry for this object
        if let Some(idx) = game.stack.iter().position(|e| e.object_id == target_id) {
            let entry = game.stack.remove(idx);
            // Move countered spell to graveyard (abilities just disappear)
            if !entry.is_ability {
                game.move_object(entry.object_id, Zone::Graveyard);
            }
            Ok(EffectOutcome::resolved())
        } else {
            // Target is no longer on the stack
            Ok(EffectOutcome::from_result(EffectResult::TargetInvalid))
        }
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "spell to counter"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::Ability;
    use crate::card::{Card, CardBuilder, PowerToughness};
    use crate::executor::ResolvedTarget;
    use crate::game_state::StackEntry;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::Object;
    use crate::static_abilities::StaticAbility;
    use crate::types::CardType;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    fn make_instant_card(card_id: u32, name: &str) -> Card {
        CardBuilder::new(CardId::from_raw(card_id), name)
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Red]]))
            .card_types(vec![CardType::Instant])
            .build()
    }

    fn make_creature_card(card_id: u32, name: &str) -> Card {
        CardBuilder::new(CardId::from_raw(card_id), name)
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Green],
            ]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build()
    }

    fn create_spell_on_stack(game: &mut GameState, name: &str, caster: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let card = make_instant_card(id.0 as u32, name);
        let obj = Object::from_card(id, &card, caster, Zone::Stack);
        game.add_object(obj);
        game.stack.push(StackEntry {
            object_id: id,
            controller: caster,
            is_ability: false,
            targets: vec![],
            x_value: None,
            ability_effects: None,
            casting_method: crate::alternative_cast::CastingMethod::Normal,
            optional_costs_paid: Default::default(),
            defending_player: None,
            saga_final_chapter_source: None,
            source_stable_id: None,
            source_name: Some(name.to_string()),
            triggering_event: None,
            intervening_if: None,
            chosen_modes: None,
        });
        id
    }

    fn create_creature(game: &mut GameState, name: &str, controller: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let card = make_creature_card(id.0 as u32, name);
        let obj = Object::from_card(id, &card, controller, Zone::Battlefield);
        game.add_object(obj);
        id
    }

    #[test]
    fn test_counter_spell_basic() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Create a spell on the stack (Bob casting Lightning Bolt)
        let spell_id = create_spell_on_stack(&mut game, "Lightning Bolt", bob);

        let counterspell_source = create_creature(&mut game, "Source", alice);
        let mut ctx = ExecutionContext::new_default(counterspell_source, alice)
            .with_targets(vec![ResolvedTarget::Object(spell_id)]);

        let effect = CounterEffect::any_spell();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Should resolve successfully
        assert_eq!(result.result, EffectResult::Resolved);

        // Stack should be empty
        assert!(game.stack.is_empty());

        // The spell should have been moved to the graveyard
        let bob_graveyard = &game.player(bob).unwrap().graveyard;
        assert_eq!(bob_graveyard.len(), 1);

        // The graveyard object should be the countered spell
        let gy_obj = game.object(bob_graveyard[0]).unwrap();
        assert_eq!(gy_obj.name, "Lightning Bolt");
        assert_eq!(gy_obj.zone, Zone::Graveyard);
    }

    #[test]
    fn test_counter_spell_target_not_on_stack() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Create a creature (not a spell on the stack)
        let creature_id = create_creature(&mut game, "Target", bob);

        let counterspell_source = create_creature(&mut game, "Source", alice);
        let mut ctx = ExecutionContext::new_default(counterspell_source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        // Try to counter something not on the stack
        let effect = CounterEffect::any_spell();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Should return TargetInvalid since the target isn't on the stack
        assert_eq!(result.result, EffectResult::TargetInvalid);
    }

    #[test]
    fn test_counter_spell_cant_be_countered() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Create a spell on the stack that can't be countered
        let spell_id = create_spell_on_stack(&mut game, "Carnage Tyrant", bob);

        // Add "can't be countered" ability to the spell
        if let Some(obj) = game.object_mut(spell_id) {
            obj.abilities.push(Ability {
                kind: AbilityKind::Static(StaticAbility::uncounterable()),
                functional_zones: vec![Zone::Stack],
                text: Some("can't be countered".to_string()),
            });
        }

        let counterspell_source = create_creature(&mut game, "Source", alice);
        let mut ctx = ExecutionContext::new_default(counterspell_source, alice)
            .with_targets(vec![ResolvedTarget::Object(spell_id)]);

        let effect = CounterEffect::any_spell();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Should return Protected since the spell can't be countered
        assert_eq!(result.result, EffectResult::Protected);

        // Stack should still have the spell
        assert_eq!(game.stack.len(), 1);
    }

    #[test]
    fn test_counter_no_target() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = CounterEffect::any_spell();
        let result = effect.execute(&mut game, &mut ctx);

        // Should return error - no target
        assert!(result.is_err());
    }

    #[test]
    fn test_counter_clone_box() {
        let effect = CounterEffect::any_spell();
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("CounterEffect"));
    }

    #[test]
    fn test_counter_get_target_spec() {
        let effect = CounterEffect::any_spell();
        assert!(effect.get_target_spec().is_some());
    }
}
