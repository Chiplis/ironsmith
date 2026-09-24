//! Haunt exile effect: exiles the source card and schedules a delayed trigger
//! to fire the haunt card's effects when the targeted (haunted) creature dies.

use crate::effect::{Effect, EffectOutcome};
use crate::effects::EffectExecutor;
use crate::effects::delayed::trigger_queue::{DelayedTriggerConfig, queue_delayed_trigger};
use crate::effects::{ExecutionContext, ExecutionError, ResolvedTarget};
use crate::game_state::GameState;
use crate::triggers::Trigger;
use crate::zone::Zone;
pub type HauntExileEffect = ironsmith_core::HauntExileEffect<Effect>;

impl EffectExecutor for HauntExileEffect {
    fn visit_child_effects(&self, visitor: &mut dyn FnMut(&Effect)) {
        for effect in &self.haunt_effects {
            visitor(effect);
        }
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        // Get the target creature (the one being haunted) from resolved targets.
        let haunted_creature_id = ctx
            .targets
            .iter()
            .find_map(|t| {
                if let ResolvedTarget::Object(id) = t {
                    Some(*id)
                } else {
                    None
                }
            })
            .ok_or(ExecutionError::InvalidTarget)?;

        // Verify the haunted creature is still on the battlefield.
        if game
            .object(haunted_creature_id)
            .is_none_or(|obj| obj.zone != Zone::Battlefield)
        {
            return Ok(EffectOutcome::resolved());
        }

        // CR 702.55a: exile the haunt card from the graveyard. The dies (or
        // "put into a graveyard during its resolution") trigger's source is
        // the pre-move object; follow the zone change to the graveyard card.
        let Some(graveyard_card) = crate::effects::helpers::resolve_source_object_id(game, ctx)
            .filter(|&id| game.object(id).is_some_and(|obj| obj.zone == Zone::Graveyard))
        else {
            return Ok(EffectOutcome::resolved());
        };
        let exiled_id = match crate::effects::zones::apply_zone_change(
            game,
            graveyard_card,
            Zone::Graveyard,
            Zone::Exile,
            ctx.cause.clone(),
            &mut *ctx.decision_maker,
        ) {
            crate::events::processing::EventOutcome::Proceed(result)
                if result.final_zone == Zone::Exile =>
            {
                result.new_object_ids.first().copied()
            }
            _ => None,
        };
        let Some(exiled_id) = exiled_id else {
            return Ok(EffectOutcome::resolved());
        };
        let Some(exiled_snapshot) = game
            .object(exiled_id)
            .map(|obj| crate::snapshot::ObjectSnapshot::from_object(obj, game))
        else {
            return Ok(EffectOutcome::resolved());
        };
        let haunting_tag = crate::tag::TagKey::from("__haunting_card");

        // Schedule a one-shot delayed trigger: when the haunted creature dies,
        // execute the haunt card's effects. It only functions while the card
        // is still in exile haunting that creature (CR 702.55c).
        let mut config = DelayedTriggerConfig::new(
            Trigger::this_dies(),
            self.haunt_effects.clone(),
            true, // one-shot
            vec![haunted_creature_id],
            ctx.controller,
        )
        .with_ability_source(Some(exiled_id))
        .with_choices(self.haunt_choices.clone())
        .with_tagged_objects(std::collections::HashMap::from([(
            haunting_tag.clone(),
            vec![exiled_snapshot],
        )]));
        config.while_any_tagged_object_in_zone = Some((haunting_tag, Zone::Exile));
        queue_delayed_trigger(game, config);

        Ok(EffectOutcome::resolved())
    }
}
