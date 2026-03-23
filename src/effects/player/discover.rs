//! Discover keyword action implementation.
//!
//! Discover N (701.55): Exile cards from the top of your library until you exile a
//! nonland card with mana value N or less. You may cast that card without paying
//! its mana cost or put it into your hand. Put the rest on the bottom of your
//! library in a random order.

use crate::alternative_cast::CastingMethod;
use crate::cost::OptionalCostsPaid;
use crate::effect::{Effect, EffectOutcome, OutcomeValue, Value};
use crate::effects::EffectExecutor;
use crate::effects::consult_helpers::{
    LibraryBottomOrder, LibraryConsultMode, LibraryConsultStopRule, execute_library_consult,
};
use crate::effects::helpers::{resolve_player_filter, resolve_value};
use crate::events::{KeywordActionEvent, KeywordActionKind};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::{GameState, StackEntry};
use crate::mana::ManaCost;
use crate::tag::TagKey;
use crate::target::PlayerFilter;
use crate::triggers::TriggerEvent;
use crate::zone::Zone;

use super::runtime_helpers::register_effect_driven_spell_cast;

/// Effect that resolves a discover action for a player.
#[derive(Debug, Clone, PartialEq)]
pub struct DiscoverEffect {
    pub count: Value,
    pub player: PlayerFilter,
}

impl DiscoverEffect {
    pub fn new(count: impl Into<Value>, player: PlayerFilter) -> Self {
        Self {
            count: count.into(),
            player,
        }
    }

    /// The controller discovers N.
    pub fn you(count: impl Into<Value>) -> Self {
        Self::new(count, PlayerFilter::You)
    }
}

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
                card.mana_cost.as_ref().map_or(0, ManaCost::mana_value) <= count
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

            let candidate_name = candidate_obj.name.clone();
            let choice_ctx = crate::decisions::context::BooleanContext::new(
                player_id,
                Some(candidate_id),
                format!("Cast {candidate_name} without paying its mana cost?"),
            );
            let should_cast = ctx.decision_maker.decide_boolean(game, &choice_ctx);

            if should_cast {
                let from_zone = candidate_obj.zone;
                let mana_cost = candidate_obj.mana_cost.clone();
                let stable_id = candidate_obj.stable_id;
                let x_value = mana_cost
                    .as_ref()
                    .and_then(|cost| if cost.has_x() { Some(0u32) } else { None });

                if let Some(new_id) = game.move_object_by_effect(candidate_id, Zone::Stack) {
                    if let Some(obj) = game.object_mut(new_id) {
                        obj.x_value = x_value;
                    }

                    let stack_entry = StackEntry {
                        object_id: new_id,
                        controller: player_id,
                        provenance: ctx.provenance,
                        targets: vec![],
                        target_assignments: vec![],
                        x_value,
                        ability_effects: None,
                        is_ability: false,
                        casting_method: CastingMethod::PlayFrom {
                            source: ctx.source,
                            zone: from_zone,
                            use_alternative: None,
                        },
                        optional_costs_paid: OptionalCostsPaid::default(),
                        defending_player: None,
                        saga_final_chapter_source: None,
                        source_stable_id: Some(stable_id),
                        source_snapshot: None,
                        source_name: Some(candidate_name),
                        triggering_event: None,
                        intervening_if: None,
                        keyword_payment_contributions: vec![],
                        crew_contributors: vec![],
                        saddle_contributors: vec![],
                        chosen_modes: None,
                        tagged_objects: std::collections::HashMap::new(),
                    };
                    game.push_to_stack(stack_entry);
                    selected_object = Some(new_id);
                    casted_spell = Some((new_id, from_zone));
                }
            } else if let Some((new_id, final_zone)) = game.move_object_with_commander_options(
                candidate_id,
                Zone::Hand,
                ctx.cause.clone(),
                &mut *ctx.decision_maker,
            ) {
                if final_zone == Zone::Hand {
                    selected_object = Some(new_id);
                }
            }
        }
        let keep_tagged = selected_object.as_ref().map(|_| match_tag.clone());
        crate::executor::execute_effect(
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
