//! Shared search-result movement and shuffle phases. Selection and disclosure
//! remain with the search adapter; movement retains its original action boundary.
use super::search_overrides::{
    FoundCardsExileReceipt, exile_found_cards_for_opposition_agent_with_outputs,
};
use crate::effects::zones::{
    BattlefieldEntryOptions, BattlefieldEntryOutcome,
    apply_zone_change_with_context_and_additional_effects_with_outputs,
    move_to_battlefield_batch_with_options,
};
use crate::effects::{CompletedEffectOutputs, ExecutionContext, ExecutionError};
use crate::events::processing::EventOutcome;
use crate::{
    game_state::GameState,
    ids::{ObjectId, PlayerId},
    zone::Zone,
};

pub(super) fn prepare_found_cards(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    chosen_ids: &[ObjectId],
    destination: Zone,
    chooser: PlayerId,
    overridden: bool,
) -> Result<
    (
        FoundCardsExileReceipt,
        Vec<crate::effects::PublishedEffectOutputs>,
    ),
    ExecutionError,
> {
    let mut published_entry_outputs = Vec::new();
    let mut moved_ids = Vec::new();
    let mut receipts = Vec::new();
    if overridden {
        let (found, published) =
            exile_found_cards_for_opposition_agent_with_outputs(game, ctx, &chosen_ids, chooser)?;
        crate::effects::PublishedEffectOutputs::append_distinct(
            &mut published_entry_outputs,
            published,
        );
        moved_ids = found.moved_ids;
        receipts = found.receipts;
        if ctx.decision_maker.awaiting_choice() {
            return Ok((
                FoundCardsExileReceipt {
                    moved_ids: vec![],
                    receipts: vec![],
                },
                published_entry_outputs,
            ));
        }
    } else if destination == Zone::Battlefield {
        let entries = move_to_battlefield_batch_with_options(
            game,
            ctx,
            chosen_ids
                .iter()
                .copied()
                .map(|id| (id, BattlefieldEntryOptions::preserve(false)))
                .collect(),
        )?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok((
                FoundCardsExileReceipt {
                    moved_ids: vec![],
                    receipts: vec![],
                },
                published_entry_outputs,
            ));
        }
        if entries.len() != chosen_ids.len() {
            return Err(ExecutionError::InternalError(
                "search entry batch lost a receipt".into(),
            ));
        }
        for (original, entry) in chosen_ids.iter().zip(entries) {
            match &entry.outcome {
                BattlefieldEntryOutcome::Moved(id) => moved_ids.push(*id),
                BattlefieldEntryOutcome::Redirected(change) => {
                    moved_ids.extend(change.new_object_ids.iter().copied())
                }
                BattlefieldEntryOutcome::Prevented => {}
            }
            let ((object, receipt), published) = entry.into_zone_receipt_with_outputs();
            crate::effects::PublishedEffectOutputs::append_distinct(
                &mut published_entry_outputs,
                published,
            );
            if object != *original {
                return Err(ExecutionError::InternalError(
                    "search entry receipt changed original identity".into(),
                ));
            }
            receipts.push((object, receipt));
        }
    } else {
        let opened_batch = game.open_simultaneous_action();
        let moves = (|| -> Result<(), ExecutionError> {
            for id in chosen_ids {
                let Some(from) = game.object(*id).map(|card| card.zone) else {
                    continue;
                };
                if destination == Zone::Library && from == Zone::Library {
                    moved_ids.push(*id);
                    continue;
                }
                let additional = ctx.additional_replacement_effects_snapshot();
                let committed = apply_zone_change_with_context_and_additional_effects_with_outputs(
                    game,
                    *id,
                    from,
                    destination,
                    ctx.cause.clone(),
                    ctx,
                    &additional,
                )?;
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(());
                }
                crate::effects::PublishedEffectOutputs::append_distinct(
                    &mut published_entry_outputs,
                    committed.published_outputs,
                );
                let receipt = committed.receipt;
                match &receipt.original {
                    EventOutcome::Proceed(change) => {
                        moved_ids.extend(change.new_object_ids.iter().copied())
                    }
                    EventOutcome::Replaced => {
                        let ids = game.take_zone_change_results(*id);
                        if !ids.is_empty() {
                            game.record_zone_change_results(*id, ids.clone());
                        }
                        moved_ids.extend(ids);
                    }
                    EventOutcome::Prevented | EventOutcome::NotApplicable => {}
                }
                receipts.push((*id, receipt));
            }
            Ok(())
        })();
        game.close_simultaneous_action(opened_batch);
        moves?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok((
                FoundCardsExileReceipt {
                    moved_ids: vec![],
                    receipts: vec![],
                },
                published_entry_outputs,
            ));
        }
    }
    Ok((
        FoundCardsExileReceipt {
            moved_ids,
            receipts,
        },
        published_entry_outputs,
    ))
}

/// Finish the prepared original and its additions exactly once, before shuffle.
pub(super) fn finish_found_cards(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    found: FoundCardsExileReceipt,
    published: Vec<crate::effects::PublishedEffectOutputs>,
    original: crate::effect::EffectOutcome,
) -> Result<CompletedEffectOutputs, ExecutionError> {
    let mut original = CompletedEffectOutputs::aggregate_only(original);
    original.retain_published_references(published);
    crate::effects::zones::finish_zone_change_receipts_with_outputs(
        game,
        ctx,
        original,
        found.receipts,
    )
}

pub(super) fn shuffle_after_search(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    player: PlayerId,
    retained: &[ObjectId],
    position: usize,
    reason: &str,
) -> Result<CompletedEffectOutputs, ExecutionError> {
    let retained = retained
        .iter()
        .copied()
        .filter(|id| {
            game.object(*id)
                .is_some_and(|card| card.zone == Zone::Library)
        })
        .collect::<Vec<_>>();
    super::shuffle_library_with_outputs(game, ctx, player, &retained, position, reason)
}
