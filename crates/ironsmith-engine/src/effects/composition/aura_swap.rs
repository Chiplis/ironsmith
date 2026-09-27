//! Aura swap keyword action.

use crate::decisions::make_decision;
use crate::decisions::specs::ChooseObjectsSpec;
use crate::effect::EffectOutcome;
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
            || game.is_phased_out(ctx.source)
            || source.owner != ctx.controller
        {
            return Ok(EffectOutcome::resolved());
        }
        let Some(attached_to) = source.attached_to else {
            return Ok(EffectOutcome::resolved());
        };
        if !game.attachment_target_exists(attached_to) {
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

        // CR 701.12a / 702.65b: stage the exchange and publish it only if
        // both movements and the required attachment can be completed. This
        // also keeps replacement/as-enters decisions from exposing a partial
        // exchange when the decision maker needs to suspend for input.
        let mut exchange = game.clone();
        // Both replacement proposals see the pre-exchange battlefield.
        let crate::events::processing::EventOutcome::Proceed(return_zone) =
            crate::events::processing::process_zone_change(
                &mut exchange,
                ctx.source,
                Zone::Battlefield,
                Zone::Hand,
                ctx.cause.clone(),
                ctx.decision_maker,
            )
        else {
            return Ok(EffectOutcome::prevented());
        };
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }
        let entry_proposal = crate::events::processing::process_etb_with_event_and_dm_with_initial_counters_and_controller(
            &mut exchange, hand_aura, Zone::Hand, ctx.decision_maker,
            Vec::new(), Some(ctx.controller),
        );
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }
        if entry_proposal.prevented && entry_proposal.new_destination.is_none() {
            return Ok(EffectOutcome::prevented());
        }
        let Some(prepared) = exchange.prepare_etb_entry_with_controller_and_dm(
            hand_aura,
            entry_proposal,
            Some(ctx.controller),
            ctx.decision_maker,
        ) else {
            return Ok(EffectOutcome::prevented());
        };
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }
        let Some(returned_source) =
            exchange.move_object(ctx.source, return_zone, ctx.cause.clone())
        else {
            return Ok(EffectOutcome::prevented());
        };
        let Some(entry) = exchange.commit_prepared_exchange_etb_with_dm(
            hand_aura,
            prepared,
            ctx.controller,
            ctx.cause.clone(),
            ctx.decision_maker,
        ) else {
            return Ok(EffectOutcome::prevented());
        };
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }
        let new_aura = entry.new_id;
        // Exchange legality was checked with the old Aura still present
        // (CR 701.12e). Its departure may remove a type/quality needed by the
        // new Aura. That doesn't undo the exchange; the subsequent SBA will
        // deal with an Aura that is no longer legally enchanting its object.
        // A destination replacement modifies this otherwise legal exchange
        // (CR 614.6). A redirected card does not become attached.
        if exchange
            .object(new_aura)
            .is_some_and(|aura| aura.zone == Zone::Battlefield)
        {
            if !exchange.attach_object_to_target(new_aura, attached_to) {
                return Ok(EffectOutcome::impossible());
            }
            exchange
                .effect_store
                .continuous_effects
                .record_attachment(new_aura);
        }

        *game = exchange;

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
    if aura.zone != Zone::Hand
        || !aura.subtypes.contains(&Subtype::Aura)
        || game.card_cannot_enter_battlefield(aura_id)
        || !game.attachment_target_is_within_range(controller, target, Some(aura_id))
    {
        return false;
    }
    match target {
        AttachmentTarget::Object(target_id) => {
            if game.is_phased_out(target_id)
                || crate::targeting::has_protection_from_source(game, target_id, aura_id)
            {
                return false;
            }
        }
        AttachmentTarget::Player(player) => {
            if crate::effects::permanents::player_has_protection_from_object(game, player, aura) {
                return false;
            }
        }
    }
    let Some(filter) = aura.aura_attach_filter_owned() else {
        return false;
    };
    let filter_ctx = game.filter_context_for(controller, Some(aura_id));
    filter.matches_target(target, &filter_ctx, game)
}
