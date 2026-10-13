//! Captured consultation phases; callers own lexical tags and primary results.
use crate::effect::{Effect, EffectOutcome};
use crate::effects::consult_helpers::{
    LibraryBottomOrder, LibraryConsultMode, LibraryConsultResult, LibraryConsultStopRule,
    execute_library_consult_with_outputs,
};
use crate::effects::{CompletedEffectOutputs, EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::tag::TagKey;
use crate::target::{ObjectFilter, PlayerFilter};
use crate::zone::Zone;

pub(super) struct FirstMatchCastProgram {
    pub player: PlayerId,
    pub all: TagKey,
    pub matched: TagKey,
    pub limit: u32,
    pub inclusive: bool,
    pub source_name: Option<String>,
    pub fallback_to_hand: bool,
}

pub(super) struct FirstMatchCastReceipt {
    pub consultation: LibraryConsultResult<CompletedEffectOutputs>,
    pub prelude: Option<CompletedEffectOutputs>,
    pub cast: Option<CompletedEffectOutputs>,
    pub fallback: Option<CompletedEffectOutputs>,
    pub cleanup: CompletedEffectOutputs,
    pub selected: Option<ObjectId>,
}

pub(super) fn cleanup_consultation(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    player: PlayerId,
    all: TagKey,
    kept: Option<TagKey>,
    order: LibraryBottomOrder,
) -> Result<CompletedEffectOutputs, ExecutionError> {
    crate::effects::execute_effect_with_outputs(
        game,
        &Effect::put_tagged_remainder_on_library_bottom(
            all,
            kept,
            order,
            PlayerFilter::Specific(player),
        ),
        ctx,
    )
}

impl FirstMatchCastProgram {
    pub(super) fn execute(
        self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        prelude: impl FnOnce(
            &mut GameState,
            &mut ExecutionContext,
            &TagKey,
        ) -> Result<Option<CompletedEffectOutputs>, ExecutionError>,
    ) -> Result<Option<FirstMatchCastReceipt>, ExecutionError> {
        let consultation = execute_library_consult_with_outputs(
            game,
            ctx,
            self.player,
            LibraryConsultMode::Exile,
            LibraryConsultStopRule::FirstMatch,
            Some(&self.all),
            Some(&self.matched),
            |card, _| {
                let value = crate::filter::object_mana_value_for_filter(card).max(0) as u32;
                !card.is_land()
                    && if self.inclusive {
                        value <= self.limit
                    } else {
                        value < self.limit
                    }
            },
        )?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(None);
        }
        let prelude = prelude(game, ctx, &self.all)?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(None);
        }
        let candidate = ctx
            .get_tagged(&self.matched)
            .map(|snapshot| snapshot.object_id);
        let comparison = if self.inclusive {
            crate::filter::Comparison::LessThanOrEqual(self.limit as i32)
        } else {
            crate::filter::Comparison::LessThan(self.limit as i32)
        };
        let cast = super::OfferTaggedCast {
            tag: self.matched.clone(),
            player: PlayerFilter::Specific(self.player),
            zone: Zone::Exile,
            spell_filter: ObjectFilter::nonland().with_mana_value(comparison),
            source_name: self.source_name,
        }
        .execute_child_with_outputs(game, ctx)?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(None);
        }
        let mut selected = cast
            .outcome
            .objects()
            .and_then(|objects| objects.first())
            .copied();
        let cast = selected.is_some().then_some(cast);
        let mut fallback = None;
        if self.fallback_to_hand
            && selected.is_none()
            && let Some(candidate) = candidate
            && game
                .object(candidate)
                .is_some_and(|card| card.zone == Zone::Exile)
        {
            let request = crate::effects::zones::PreparedZoneMove::capture(
                game,
                candidate,
                Zone::Exile,
                Zone::Hand,
                ctx.cause.clone(),
                None,
            );
            fallback = Some(crate::effects::zones::execute_zone_moves_with_outputs(
                game,
                ctx,
                vec![request],
                |game, _, receipts| {
                    selected =
                        crate::effects::zones::movement_arrivals(game, candidate, &receipts[0].1)
                            .into_iter()
                            .find(|id| {
                                game.object(*id)
                                    .is_some_and(|object| object.zone == Zone::Hand)
                            });
                    Ok(selected
                        .map(|id| EffectOutcome::with_objects(vec![id]))
                        .unwrap_or_else(|| EffectOutcome::count(0)))
                },
            )?);
            if ctx.decision_maker.awaiting_choice() {
                return Ok(None);
            }
        }
        let cleanup = cleanup_consultation(
            game,
            ctx,
            self.player,
            self.all,
            selected.map(|_| self.matched),
            LibraryBottomOrder::Random,
        )?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(None);
        }
        Ok(Some(FirstMatchCastReceipt {
            consultation,
            prelude,
            cast,
            fallback,
            cleanup,
            selected,
        }))
    }
}

/// Recompute offers after each cast's payment and capture its triggers before
/// another payment can change watchers. Failed methods are excluded only until
/// a successful cast changes the state. The retained candidate set is caller-owned.
pub(super) fn offer_refreshed_casts(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    player: PlayerId,
    prompt: &str,
    mut refresh: impl FnMut(
        &GameState,
        &ExecutionContext,
    ) -> Result<
        Vec<super::runtime_helpers::EffectDrivenCastOption>,
        ExecutionError,
    >,
) -> Result<
    Option<(
        Vec<CompletedEffectOutputs>,
        Vec<crate::snapshot::ObjectSnapshot>,
        Vec<ObjectId>,
    )>,
    ExecutionError,
> {
    use super::runtime_helpers::*;
    let mut failed = Vec::new();
    let mut outputs = Vec::new();
    let mut retained = Vec::new();
    let mut cast_ids = Vec::new();
    loop {
        let mut options = refresh(game, ctx)?;
        options.retain(|option| {
            !failed
                .iter()
                .any(|(id, method)| *id == option.object_id && *method == option.casting_method)
        });
        if options.is_empty() {
            break;
        }
        let menu = crate::decisions::context::SelectOptionsContext::new(
            player,
            Some(ctx.source),
            prompt,
            options
                .iter()
                .enumerate()
                .map(|(i, option)| {
                    crate::decisions::context::SelectableOption::new(i, option.label.clone())
                })
                .collect(),
            0,
            1,
        );
        let answer = ctx.decision_maker.decide_options(game, &menu);
        if ctx.decision_maker.awaiting_choice() {
            return Ok(None);
        }
        let index = match answer.as_slice() {
            [] => break,
            [index] if *index < options.len() => *index,
            _ => {
                return Err(ExecutionError::Impossible(
                    "invalid refreshed cast selection".into(),
                ));
            }
        };
        let option = &options[index];
        failed.push((option.object_id, option.casting_method.clone()));
        let snapshot = game
            .object(option.object_id)
            .map(|object| crate::snapshot::ObjectSnapshot::from_object(object, game))
            .ok_or(ExecutionError::ObjectNotFound(option.object_id))?;
        let cast = cast_effect_driven_spell_with_payment_policy(
            game,
            ctx,
            player,
            option,
            EffectDrivenCastPayment::WithoutPayingManaCost,
            true,
        )?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(None);
        }
        if let Some(cast) = cast {
            failed.clear();
            retained.push(snapshot);
            cast_ids.push(cast.new_id);
            let provenance = game
                .alloc_child_event_provenance(ctx.provenance, crate::events::EventKind::SpellCast);
            let mut output = complete_effect_driven_cast_with_outputs(
                EffectOutcome::with_objects(vec![cast.new_id]),
                game,
                cast,
                player,
                provenance,
            )?;
            crate::effects::capture_triggers_before_added_program(
                game,
                ctx,
                None,
                output.outcome.events.iter_mut(),
            )?;
            outputs.push(output);
        }
    }
    Ok(Some((outputs, retained, cast_ids)))
}
