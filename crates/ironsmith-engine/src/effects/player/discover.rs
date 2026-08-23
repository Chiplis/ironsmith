//! Discover keyword action implementation.
//!
//! Discover N (701.55): Exile cards from the top of your library until you exile a
//! nonland card with mana value N or less. You may cast that card without paying
//! its mana cost or put it into your hand. Put the rest on the bottom of your
//! library in a random order.

use crate::effect::{Effect, EffectOutcome, OutcomeValue};
use crate::effects::EffectExecutor;
use crate::effects::consult_helpers::{
    LibraryBottomOrder, LibraryConsultMode, LibraryConsultStopRule, execute_library_consult,
};
use crate::effects::helpers::{resolve_player_filter, resolve_value};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::events::{KeywordActionEvent, KeywordActionKind};
use crate::game_state::GameState;
use crate::tag::TagKey;
use crate::target::PlayerFilter;
use crate::triggers::TriggerEvent;
use crate::zone::Zone;
pub use ironsmith_core::DiscoverEffect;

use super::runtime_helpers::{
    EffectDrivenCastOption, cast_effect_driven_spell_without_paying,
    register_effect_driven_spell_cast,
};

/// Effect that resolves a discover action for a player.
impl EffectExecutor for DiscoverEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_filter(game, &self.player, ctx)?;
        let count = resolve_value(game, &self.count, ctx)?.max(0) as u32;
        let all_tag = TagKey::from("__discover_all");
        let match_tag = TagKey::from("__discover_match");
        execute_library_consult(
            game,
            ctx,
            player_id,
            LibraryConsultMode::Exile,
            LibraryConsultStopRule::FirstMatch,
            Some(&all_tag),
            Some(&match_tag),
            |card, _| {
                if card.is_land() {
                    return false;
                }
                card.mana_cost.as_ref().map_or(0, |cost| cost.mana_value()) <= count
            },
        )?;

        let mut selected_object = None;
        let mut casted_spell = None;
        if let Some(candidate_snapshot) = ctx.get_tagged(match_tag.as_str()).cloned() {
            let mut candidate_id = candidate_snapshot.object_id;
            if game.object(candidate_id).is_none() {
                if let Some(found) = game.find_object_by_stable_id(candidate_snapshot.stable_id) {
                    candidate_id = found;
                } else {
                    return Ok(EffectOutcome::count(0).with_event(
                        TriggerEvent::new_with_provenance(
                            KeywordActionEvent::new(
                                KeywordActionKind::Discover,
                                player_id,
                                ctx.source,
                                count,
                            ),
                            ctx.provenance,
                        ),
                    ));
                }
            }
            let Some(candidate_obj) = game.object(candidate_id) else {
                return Ok(
                    EffectOutcome::count(0).with_event(TriggerEvent::new_with_provenance(
                        KeywordActionEvent::new(
                            KeywordActionKind::Discover,
                            player_id,
                            ctx.source,
                            count,
                        ),
                        ctx.provenance,
                    )),
                );
            };

            let candidate_name = candidate_obj.name.to_string();
            let choice_ctx = crate::decisions::context::BooleanContext::new(
                player_id,
                Some(candidate_id),
                format!("Cast {candidate_name} without paying its mana cost?"),
            );
            let should_cast = ctx.decision_maker.decide_boolean(game, &choice_ctx);
            if ctx.decision_maker.awaiting_choice() {
                return Ok(EffectOutcome::count(0));
            }

            if should_cast {
                let from_zone = candidate_obj.zone;
                let option = EffectDrivenCastOption {
                    object_id: candidate_id,
                    from_zone,
                    casting_method: crate::alternative_cast::CastingMethod::PlayFrom {
                        source: ctx.source,
                        zone: from_zone,
                        use_alternative: None,
                    },
                    label: format!("Cast {candidate_name}"),
                };
                if let Some(result) =
                    cast_effect_driven_spell_without_paying(game, ctx, player_id, &option)?
                {
                    selected_object = Some(result.new_id);
                    casted_spell = Some((result.new_id, result.from_zone));
                } else if ctx.decision_maker.awaiting_choice() {
                    return Ok(EffectOutcome::count(0));
                }
            } else if let Some((new_id, final_zone)) = game.move_object_with_commander_options(
                candidate_id,
                Zone::Hand,
                ctx.cause.clone(),
                &mut *ctx.decision_maker,
            ) && final_zone == Zone::Hand
            {
                selected_object = Some(new_id);
            }
        }
        let keep_tagged = selected_object.as_ref().map(|_| match_tag.clone());
        crate::effects::execute_effect(
            game,
            &Effect::put_tagged_remainder_on_library_bottom(
                all_tag,
                keep_tagged,
                LibraryBottomOrder::Random,
                PlayerFilter::Specific(player_id),
            ),
            ctx,
        )?;

        let value = if let Some(id) = selected_object {
            OutcomeValue::Objects(vec![id])
        } else {
            OutcomeValue::Count(0)
        };

        let mut outcome = EffectOutcome::with_details(
            crate::effect::OutcomeStatus::Succeeded,
            value,
            vec![TriggerEvent::new_with_provenance(
                KeywordActionEvent::new(KeywordActionKind::Discover, player_id, ctx.source, count),
                ctx.provenance,
            )],
            Vec::new(),
        );
        if let Some((new_id, from_zone)) = casted_spell {
            outcome = outcome.with_event(register_effect_driven_spell_cast(
                game,
                new_id,
                player_id,
                from_zone,
                ctx.provenance,
            ));
        }
        Ok(outcome)
    }
}
