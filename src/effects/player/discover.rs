//! Discover keyword action implementation.
//!
//! Discover N (701.55): Exile cards from the top of your library until you exile a
//! nonland card with mana value N or less. You may cast that card without paying
//! its mana cost or put it into your hand. Put the rest on the bottom of your
//! library in a random order.

use rand::seq::SliceRandom;

use crate::alternative_cast::CastingMethod;
use crate::cost::OptionalCostsPaid;
use crate::effect::{EffectOutcome, EffectResult, Value};
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_player_filter, resolve_value};
use crate::events::{KeywordActionEvent, KeywordActionKind};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::{GameState, StackEntry};
use crate::mana::ManaCost;
use crate::target::PlayerFilter;
use crate::triggers::TriggerEvent;
use crate::zone::Zone;

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

        let mut exiled = Vec::new();
        let mut candidate = None;

        loop {
            let top_card = game
                .player(player_id)
                .and_then(|player| player.library.last().copied());
            let Some(top_card_id) = top_card else {
                break;
            };

            let Some(exiled_id) = game.move_object(top_card_id, Zone::Exile) else {
                break;
            };
            exiled.push(exiled_id);

            let Some(card) = game.object(exiled_id) else {
                continue;
            };
            if card.is_land() {
                continue;
            }

            let mv = card.mana_cost.as_ref().map_or(0, ManaCost::mana_value);
            if mv <= count {
                candidate = Some(exiled_id);
                break;
            }
        }

        let mut selected_object = None;
        if let Some(candidate_id) = candidate {
            let Some(candidate_obj) = game.object(candidate_id) else {
                return Ok(EffectOutcome::count(exiled.len() as i32).with_event(
                    TriggerEvent::new(KeywordActionEvent::new(
                        KeywordActionKind::Discover,
                        player_id,
                        ctx.source,
                        count,
                    )),
                ));
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

                if let Some(new_id) = game.move_object(candidate_id, Zone::Stack) {
                    if let Some(obj) = game.object_mut(new_id) {
                        obj.x_value = x_value;
                    }

                    let stack_entry = StackEntry {
                        object_id: new_id,
                        controller: player_id,
                        targets: vec![],
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
                    };
                    game.push_to_stack(stack_entry);
                    selected_object = Some(new_id);
                }
            } else if let Some(new_id) = game.move_object(candidate_id, Zone::Hand) {
                selected_object = Some(new_id);
            }
        }

        let mut to_bottom = exiled;
        if let Some(candidate_id) = candidate {
            to_bottom.retain(|id| *id != candidate_id);
        }
        to_bottom.shuffle(&mut rand::rng());

        for exiled_id in to_bottom {
            if let Some(new_id) = game.move_object(exiled_id, Zone::Library) {
                let owner = game.object(new_id).map(|obj| obj.owner);
                if let Some(owner) = owner
                    && let Some(player) = game.player_mut(owner)
                    && let Some(pos) = player.library.iter().position(|id| *id == new_id)
                {
                    // Library vec end = top, start = bottom. Ensure this card goes to bottom.
                    player.library.remove(pos);
                    player.library.insert(0, new_id);
                }
            }
        }

        let result = if let Some(id) = selected_object {
            EffectResult::Objects(vec![id])
        } else {
            EffectResult::Count(0)
        };

        Ok(
            EffectOutcome::from_result(result).with_event(TriggerEvent::new(
                KeywordActionEvent::new(KeywordActionKind::Discover, player_id, ctx.source, count),
            )),
        )
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }
}
