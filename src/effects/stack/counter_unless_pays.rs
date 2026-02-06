//! Counter unless pays effect implementation.

use crate::ability::AbilityKind;
use crate::decision::FallbackStrategy;
use crate::decisions::make_boolean_decision;
use crate::effect::{EffectOutcome, EffectResult};
use crate::effects::EffectExecutor;
use crate::effects::helpers::find_target_object;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::mana::{ManaCost, ManaSymbol};
use crate::target::ChooseSpec;
use crate::zone::Zone;

/// Effect that counters a target spell unless its controller pays a mana cost.
///
/// This is the "soft counter" pattern used by cards like Mana Leak, Spell Pierce, etc.
/// The spell's controller can choose to pay the specified mana cost to prevent the spell
/// from being countered.
///
/// # Fields
///
/// * `target` - Which spell to counter (resolved from ctx.targets)
/// * `mana` - The mana cost that must be paid to prevent countering
///
/// # Example
///
/// ```ignore
/// // Counter target spell unless its controller pays {2}
/// let effect = CounterUnlessPaysEffect::new(
///     ChooseSpec::spell(),
///     vec![ManaSymbol::Generic(2)],
/// );
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct CounterUnlessPaysEffect {
    /// The targeting specification (for UI/validation purposes).
    pub target: ChooseSpec,
    /// The mana cost required to prevent countering.
    pub mana: Vec<ManaSymbol>,
}

impl CounterUnlessPaysEffect {
    /// Create a new counter unless pays effect.
    pub fn new(target: ChooseSpec, mana: Vec<ManaSymbol>) -> Self {
        Self { target, mana }
    }

    /// Create a Mana Leak-style effect (counter unless they pay {3}).
    pub fn mana_leak() -> Self {
        Self::new(ChooseSpec::spell(), vec![ManaSymbol::Generic(3)])
    }

    /// Create a generic counter unless pays {2} effect.
    pub fn counter_unless_two() -> Self {
        Self::new(ChooseSpec::spell(), vec![ManaSymbol::Generic(2)])
    }
}

impl EffectExecutor for CounterUnlessPaysEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        // Get target from resolved targets
        let target_id = find_target_object(&ctx.targets)?;

        // Verify target is on the stack
        let obj = game
            .object(target_id)
            .ok_or(ExecutionError::ObjectNotFound(target_id))?;

        if obj.zone != Zone::Stack {
            return Ok(EffectOutcome::from_result(EffectResult::TargetInvalid));
        }

        // Check if spell has CantBeCountered
        let cant_be_countered = obj.abilities.iter().any(|ability| {
            if let AbilityKind::Static(s) = &ability.kind {
                s.cant_be_countered()
            } else {
                false
            }
        });

        if cant_be_countered {
            return Ok(EffectOutcome::from_result(EffectResult::Protected));
        }

        // Get the controller of the target spell
        let spell_controller = obj.controller;

        // Format the mana cost for display
        let mana_display: String = self
            .mana
            .iter()
            .map(|s| format!("{:?}", s))
            .collect::<Vec<_>>()
            .join(", ");

        // Check if player can afford to pay
        let can_afford = {
            let cost = ManaCost::from_symbols(self.mana.clone());
            game.can_pay_mana_cost(spell_controller, None, &cost, 0)
        };

        // Ask the spell's controller if they want to pay
        let wants_to_pay = if can_afford {
            make_boolean_decision(
                game,
                &mut ctx.decision_maker,
                spell_controller,
                ctx.source,
                format!(
                    "Pay {} to prevent your spell from being countered?",
                    mana_display
                ),
                FallbackStrategy::Decline,
            )
        } else {
            // Can't afford, don't even ask
            false
        };

        if wants_to_pay {
            // Pay the mana cost
            let cost = ManaCost::from_symbols(self.mana.clone());
            if game.try_pay_mana_cost(spell_controller, None, &cost, 0) {
                // Payment successful, spell is NOT countered
                return Ok(EffectOutcome::from_result(EffectResult::Declined));
            }
        }

        // Counter the spell - remove from stack first, then move to graveyard
        if let Some(idx) = game.stack.iter().position(|e| e.object_id == target_id) {
            let entry = game.stack.remove(idx);
            if !entry.is_ability {
                game.move_object(entry.object_id, Zone::Graveyard);
            }
            Ok(EffectOutcome::resolved())
        } else {
            // Shouldn't happen since we verified it was on the stack earlier
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
    use crate::card::{Card, CardBuilder};
    use crate::executor::ResolvedTarget;
    use crate::game_state::StackEntry;
    use crate::ids::{CardId, ObjectId, PlayerId};
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

    #[test]
    fn test_counter_unless_pays_no_mana() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Bob casts a spell with no mana in pool
        let spell_id = create_spell_on_stack(&mut game, "Lightning Bolt", bob);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(spell_id)]);

        let effect = CounterUnlessPaysEffect::mana_leak();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Bob can't pay, spell is countered
        assert_eq!(result.result, EffectResult::Resolved);
        assert!(game.stack.is_empty());
        // Spell should be in graveyard
        let bob_gy = &game.player(bob).unwrap().graveyard;
        assert_eq!(bob_gy.len(), 1);
    }

    #[test]
    fn test_counter_unless_pays_cant_be_countered() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let spell_id = create_spell_on_stack(&mut game, "Carnage Tyrant", bob);

        // Add "can't be countered" to the spell
        if let Some(obj) = game.object_mut(spell_id) {
            obj.abilities.push(crate::ability::Ability {
                kind: AbilityKind::Static(StaticAbility::uncounterable()),
                functional_zones: vec![Zone::Stack],
                text: Some("can't be countered".to_string()),
            });
        }

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(spell_id)]);

        let effect = CounterUnlessPaysEffect::mana_leak();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Spell can't be countered
        assert_eq!(result.result, EffectResult::Protected);
        assert_eq!(game.stack.len(), 1);
    }

    #[test]
    fn test_counter_unless_pays_target_not_on_stack() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Create a creature on battlefield, not a spell on stack
        let creature_id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(creature_id.0 as u32), "Bear")
            .card_types(vec![CardType::Creature])
            .build();
        let obj = Object::from_card(creature_id, &card, bob, Zone::Battlefield);
        game.add_object(obj);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let effect = CounterUnlessPaysEffect::mana_leak();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::TargetInvalid);
    }

    #[test]
    fn test_counter_unless_pays_no_target() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = CounterUnlessPaysEffect::mana_leak();
        let result = effect.execute(&mut game, &mut ctx);

        assert!(result.is_err());
    }

    #[test]
    fn test_counter_unless_pays_clone_box() {
        let effect = CounterUnlessPaysEffect::mana_leak();
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("CounterUnlessPaysEffect"));
    }

    #[test]
    fn test_counter_unless_pays_get_target_spec() {
        let effect = CounterUnlessPaysEffect::mana_leak();
        assert!(effect.get_target_spec().is_some());
    }
}
