//! A spell kept with its card to come back onto the stack later as a copy of
//! the original spell (Ertai's Meddling). See
//! [`ironsmith_core::RetainSpellChoicesEffect`].

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_objects_for_effect;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::{GameState, RetainedSpellChoices};
use crate::zone::Zone;
pub use ironsmith_core::{PutRetainedSpellCopyOntoStackEffect, RetainSpellChoicesEffect};

impl EffectExecutor for RetainSpellChoicesEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let objects = resolve_objects_for_effect(game, ctx, &self.target)?;
        let mut retained = 0;
        for object in objects {
            let Some(entry) = game
                .stack
                .iter()
                .rev()
                .find(|entry| entry.object_id == object && !entry.is_ability)
            else {
                continue;
            };
            let choices = RetainedSpellChoices::from_stack_entry(entry);
            let Some(card) = game.object(object).map(|object| object.stable_id) else {
                continue;
            };
            // "Target spell's controller exiles it ... each of that player's
            // upkeeps": the spell's controller, for later instructions and
            // delayed triggers scheduled in this resolution.
            ctx.set_tagged_players(self.controller_tag.clone(), vec![choices.controller]);
            game.retain_spell_choices(card, choices);
            retained += 1;
        }
        Ok(EffectOutcome::count(retained))
    }
}

impl EffectExecutor for PutRetainedSpellCopyOntoStackEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let objects = resolve_objects_for_effect(game, ctx, &self.target)?;
        let mut put = 0;
        for object in objects {
            let Some((card, zone)) = game
                .object(object)
                .map(|object| (object.stable_id, object.zone))
            else {
                continue;
            };
            // Only the card still where the original spell left it to.
            if zone == Zone::Stack || zone == Zone::Battlefield {
                continue;
            }
            let Some(choices) = game.retained_spell_choices(card).cloned() else {
                continue;
            };
            // Put onto the stack, not cast (CR 601.1a): no cast triggers and
            // no costs; the copy keeps the original spell's choices
            // (CR 707.10) under that spell's controller.
            let Some(spell) = game.move_object_by_effect(object, Zone::Stack) else {
                continue;
            };
            game.take_retained_spell_choices(card);
            if let Some(object) = game.object_mut(spell) {
                object.x_value = choices.x_value;
                object.optional_costs_paid = choices.optional_costs_paid.clone();
            }
            let mut entry = choices.stack_entry(spell, choices.controller);
            entry.provenance = ctx.provenance;
            game.push_to_stack(entry);
            put += 1;
        }
        Ok(EffectOutcome::count(put))
    }
}
