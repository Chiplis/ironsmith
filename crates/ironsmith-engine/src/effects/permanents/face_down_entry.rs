//! Prepared face-down battlefield entry shared by manifest and cloak programs.
use crate::effect::EffectOutcome;
use crate::effects::zones::{
    BattlefieldEntryOptions, BattlefieldEntryOutcome, move_to_battlefield_with_options,
};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::events::{KeywordActionEvent, KeywordActionKind};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId, StableId};

#[derive(Debug, Clone)]
pub(crate) struct ManifestPreparation {
    pub(crate) object_id: ObjectId,
    stable_id: StableId,
    original_abilities: std::sync::Arc<Vec<crate::ability::Ability>>,
    overlay_applied: bool,
}

pub(crate) fn prepare_manifest_card(
    game: &mut GameState,
    card_id: ObjectId,
    cloak: bool,
) -> Option<ManifestPreparation> {
    let card = game.object_mut(card_id)?;
    let stable_id = card.stable_id;
    let original_abilities = card.abilities.clone();
    let overlay_applied = card.apply_face_down_cast_overlay();
    // Disguise's shared face-down overlay adds ward {2}, but manifesting a
    // disguise card does not make the disguise action's ward apply. Cloak has
    // ward {2} independently, so normalize the overlay before adding it.
    card.abilities_mut().retain(|ability| {
        !matches!(
            &ability.kind,
            crate::ability::AbilityKind::Static(static_ability)
                if static_ability.id() == crate::static_abilities::StaticAbilityId::Ward
        )
    });
    if cloak {
        card.abilities_mut()
            .push(crate::ability::Ability::static_ability(
                crate::static_abilities::StaticAbility::ward(crate::cost::TotalCost::mana(
                    crate::mana::ManaCost::from_pips(vec![vec![crate::mana::ManaSymbol::Generic(
                        2,
                    )]]),
                )),
            ));
    }
    Some(ManifestPreparation {
        object_id: card_id,
        stable_id,
        original_abilities,
        overlay_applied,
    })
}

pub(crate) fn rollback_manifest_preparation(
    game: &mut GameState,
    preparation: &ManifestPreparation,
) {
    let Some(card) = game.object_mut(preparation.object_id) else {
        return;
    };
    if card.stable_id != preparation.stable_id {
        return;
    }
    if preparation.overlay_applied {
        card.end_face_down_cast_overlay();
    } else {
        card.abilities = preparation.original_abilities.clone();
    }
}

pub(crate) fn prepare_manifest_entry(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    card_id: ObjectId,
    controller: PlayerId,
    cloak: bool,
) -> Result<
    (
        EffectOutcome,
        Option<crate::effects::zones::BattlefieldEntryReceipt>,
    ),
    ExecutionError,
> {
    let Some(preparation) = prepare_manifest_card(game, card_id, cloak) else {
        return Ok((EffectOutcome::count(0), None));
    };
    let receipt = move_to_battlefield_with_options(
        game,
        ctx,
        card_id,
        BattlefieldEntryOptions::specific(controller, false),
    )?;
    if ctx.decision_maker.awaiting_choice() {
        return Ok((EffectOutcome::count(0), None));
    }
    let receipt = receipt.ok_or_else(|| {
        ExecutionError::InternalError("manifest entry returned no terminal receipt".into())
    })?;
    let original = match &receipt.outcome {
        BattlefieldEntryOutcome::Moved(id) => {
            if cloak {
                game.set_cloaked(*id);
            } else {
                game.set_manifested(*id);
            }
            EffectOutcome::with_objects(vec![*id])
        }
        BattlefieldEntryOutcome::Redirected(change) => {
            EffectOutcome::with_objects(change.new_object_ids.clone())
        }
        BattlefieldEntryOutcome::Prevented => {
            rollback_manifest_preparation(game, &preparation);
            EffectOutcome::count(0)
        }
    };
    Ok((original, Some(receipt)))
}

pub(crate) fn manifest_card_with_outputs(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    card_id: ObjectId,
    controller: PlayerId,
    cloak: bool,
    action: KeywordActionKind,
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    crate::effects::composition::execute_transaction(
        game,
        ctx,
        || crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
        |game, ctx| {
            let (mut original, receipt) =
                prepare_manifest_entry(game, ctx, card_id, controller, cloak)?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                    EffectOutcome::count(0),
                ));
            }
            let mut keyword_outputs = None;
            if receipt
                .as_ref()
                .is_some_and(|receipt| matches!(receipt.outcome, BattlefieldEntryOutcome::Moved(_)))
            {
                let keyword = crate::effects::composition::complete_keyword_action_with_outputs(
                    game,
                    ctx,
                    crate::effects::CompletedEffectOutputs::aggregate_only(original),
                    KeywordActionEvent::new(action, controller, ctx.source, 1),
                )?;
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }
                original = keyword.outcome.clone();
                keyword_outputs = Some(keyword);
            }
            let mut outputs =
                crate::effects::zones::finish_battlefield_entry_receipts_with_outputs(
                    game,
                    ctx,
                    original,
                    receipt.into_iter().collect(),
                )?;
            if !ctx.decision_maker.awaiting_choice() {
                // This keyword history already belongs to the zone aggregate.
                outputs.retain_published_children(keyword_outputs);
            }
            Ok(outputs)
        },
    )
}
