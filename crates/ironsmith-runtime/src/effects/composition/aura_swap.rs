//! Aura swap keyword action.

use crate::decisions::make_decision;
use crate::decisions::specs::ChooseObjectsSpec;
use crate::effect::EffectOutcome;
use crate::effects::permanents::attach_battlefield_object_to_target;
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::object::{AttachmentTarget, AuraAttachmentFilterRuntimeExt};
use crate::types::Subtype;
use crate::zone::Zone;

pub type AuraSwapEffect = ironsmith_core::AuraSwapEffect;

impl EffectExecutor for AuraSwapEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let Some(source) = game.object(ctx.source) else {
            return Ok(EffectOutcome::resolved());
        };
        if source.zone != Zone::Battlefield
            || source.owner != ctx.controller
            || game.controller_of(source) != ctx.controller
        {
            return Ok(EffectOutcome::resolved());
        }
        let Some(attached_to) = source.attached_to else {
            return Ok(EffectOutcome::resolved());
        };
        if !game.attachment_target_exists_on_battlefield(attached_to) {
            return Ok(EffectOutcome::resolved());
        }

        let candidates = aura_swap_candidates(game, ctx.controller, attached_to);
        if candidates.is_empty() {
            return Ok(EffectOutcome::resolved());
        }

        let spec = ChooseObjectsSpec::new(
            ctx.source,
            "Choose an Aura card in your hand",
            candidates.clone(),
            0,
            Some(1),
        );
        let chosen = make_decision(
            game,
            ctx.decision_maker,
            ctx.controller,
            Some(ctx.source),
            spec,
        );
        if ctx.decision_maker.awaiting_choice() || chosen.is_empty() {
            return Ok(EffectOutcome::resolved());
        }
        let hand_aura = chosen[0];
        if !candidates.contains(&hand_aura) {
            return Ok(EffectOutcome::resolved());
        }

        let Some(returned_source) = game.move_object_by_effect(ctx.source, Zone::Hand) else {
            return Ok(EffectOutcome::prevented());
        };
        let Some(new_aura) = game.move_object_by_effect(hand_aura, Zone::Battlefield) else {
            return Ok(EffectOutcome::prevented());
        };
        if !attach_battlefield_object_to_target(game, new_aura, attached_to) {
            return Ok(EffectOutcome::impossible());
        }

        Ok(EffectOutcome::with_objects(vec![returned_source, new_aura]))
    }
}

fn aura_swap_candidates(
    game: &GameState,
    player: PlayerId,
    attached_to: AttachmentTarget,
) -> Vec<ObjectId> {
    let Some(player_state) = game.player(player) else {
        return Vec::new();
    };
    player_state
        .hand
        .iter()
        .copied()
        .filter(|id| aura_card_can_attach_to_target(game, *id, player, attached_to))
        .collect()
}

fn aura_card_can_attach_to_target(
    game: &GameState,
    aura_id: ObjectId,
    controller: PlayerId,
    target: AttachmentTarget,
) -> bool {
    let Some(aura) = game.object(aura_id) else {
        return false;
    };
    if aura.zone != Zone::Hand || !aura.subtypes.contains(&Subtype::Aura) {
        return false;
    }
    let Some(filter) = aura.aura_attach_filter.clone() else {
        return false;
    };
    let filter_ctx = game.filter_context_for(controller, Some(aura_id));
    filter.matches_target(target, &filter_ctx, game)
}
