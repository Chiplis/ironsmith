//! Move to zone effect implementation.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_objects_for_effect;
use crate::event_processor::{EventOutcome, process_zone_change_with_additional_effects};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::ChooseSpec;
use crate::zone::Zone;

use super::{
    BattlefieldEntryOptions, BattlefieldEntryOutcome, finalize_zone_change_move,
    maybe_prompt_for_split_result_order, move_to_battlefield_with_options,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BattlefieldController {
    Preserve,
    Owner,
    You,
}

/// Effect that moves a target object to a specified zone.
///
/// This is a generic zone change effect used for various purposes like
/// putting cards on top/bottom of library, moving to exile, etc.
///
/// # Fields
///
/// * `target` - Which object to move (resolved from `ChooseSpec`)
/// * `zone` - The destination zone
/// * `to_top` - If moving to library, whether to put on top (vs bottom)
///
/// # Example
///
/// ```ignore
/// // Put target card on top of its owner's library
/// let effect = MoveToZoneEffect::new(ChooseSpec::card(), Zone::Library, true);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct MoveToZoneEffect {
    /// The targeting specification (for UI/validation purposes).
    pub target: ChooseSpec,
    /// The destination zone.
    pub zone: Zone,
    /// If moving to library, put on top (true) or bottom (false).
    pub to_top: bool,
    /// Controller override when the destination is the battlefield.
    pub battlefield_controller: BattlefieldController,
    /// If moving to the battlefield, the permanent enters tapped.
    pub enters_tapped: bool,
}

impl MoveToZoneEffect {
    /// Create a new move to zone effect.
    pub fn new(target: ChooseSpec, zone: Zone, to_top: bool) -> Self {
        Self {
            target,
            zone,
            to_top,
            battlefield_controller: BattlefieldController::Preserve,
            enters_tapped: false,
        }
    }

    /// Create an effect to put a card on top of its owner's library.
    pub fn to_top_of_library(target: ChooseSpec) -> Self {
        Self::new(target, Zone::Library, true)
    }

    /// Create an effect to put a card on bottom of its owner's library.
    pub fn to_bottom_of_library(target: ChooseSpec) -> Self {
        Self::new(target, Zone::Library, false)
    }

    /// Create an effect to move a card to exile.
    pub fn to_exile(target: ChooseSpec) -> Self {
        Self::new(target, Zone::Exile, false)
    }

    /// Create an effect to move a card to graveyard.
    pub fn to_graveyard(target: ChooseSpec) -> Self {
        Self::new(target, Zone::Graveyard, false)
    }

    pub fn under_owner_control(mut self) -> Self {
        self.battlefield_controller = BattlefieldController::Owner;
        self
    }

    pub fn under_you_control(mut self) -> Self {
        self.battlefield_controller = BattlefieldController::You;
        self
    }

    pub fn tapped(mut self) -> Self {
        self.enters_tapped = true;
        self
    }
}

impl EffectExecutor for MoveToZoneEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let mut object_ids = resolve_objects_for_effect(game, ctx, &self.target)?;
        // When a tag snapshot carries a stale ObjectId (the tagged object
        // changed zones since the snapshot was taken), resolve through
        // stable_id so the move can find the actual game object.
        if let ChooseSpec::Tagged(tag) = &self.target {
            if let Some(tagged) = ctx.get_tagged_all(tag) {
                for (idx, snapshot) in tagged.iter().enumerate() {
                    if idx < object_ids.len() && game.object(object_ids[idx]).is_none() {
                        if let Some(resolved) =
                            crate::effects::helpers::resolve_tagged_object_id(game, snapshot)
                        {
                            object_ids[idx] = resolved;
                        }
                    }
                }
            }
        }
        if object_ids.is_empty() {
            return Ok(EffectOutcome::target_invalid());
        }

        let mut moved_ids = Vec::new();
        let mut any_prevented = false;
        let mut any_replaced = false;

        for object_id in object_ids {
            let Some(obj) = game.object(object_id) else {
                continue;
            };
            let from_zone = obj.zone;
            let additional_effects = ctx.additional_replacement_effects_snapshot();

            // Process through replacement effects with decision maker
            let result = process_zone_change_with_additional_effects(
                game,
                object_id,
                from_zone,
                self.zone,
                ctx.cause.clone(),
                &mut ctx.decision_maker,
                &additional_effects,
            );

            match result {
                EventOutcome::Prevented => {
                    return Ok(EffectOutcome::prevented());
                }
                EventOutcome::Proceed(final_zone) => {
                    if final_zone == Zone::Battlefield {
                        let options = match self.battlefield_controller {
                            BattlefieldController::Preserve => {
                                BattlefieldEntryOptions::preserve(self.enters_tapped)
                            }
                            BattlefieldController::Owner => {
                                BattlefieldEntryOptions::owner(self.enters_tapped)
                            }
                            BattlefieldController::You => BattlefieldEntryOptions::specific(
                                ctx.controller,
                                self.enters_tapped,
                            ),
                        };
                        match move_to_battlefield_with_options(game, ctx, object_id, options) {
                            BattlefieldEntryOutcome::Moved(new_id) => {
                                moved_ids.push(new_id);
                            }
                            BattlefieldEntryOutcome::Prevented => {
                                any_prevented = true;
                            }
                        }
                        continue;
                    }

                    let mut result =
                        finalize_zone_change_move(game, object_id, final_zone, ctx.cause.clone());
                    if !result.new_object_ids.is_empty() {
                        for &new_id in &result.new_object_ids {
                            if final_zone == Zone::Exile {
                                game.add_exiled_with_source_link(ctx.source, new_id);
                            }
                            if final_zone == Zone::Library && !self.to_top {
                                if let Some(obj) = game.object(new_id) {
                                    if let Some(player) = game.player_mut(obj.owner) {
                                        if let Some(pos) =
                                            player.library.iter().position(|id| *id == new_id)
                                        {
                                            player.library.remove(pos);
                                            player.library.insert(0, new_id);
                                        }
                                    }
                                }
                            }
                        }
                        if final_zone == Zone::Library && from_zone == Zone::Battlefield {
                            maybe_prompt_for_split_result_order(
                                game,
                                &mut ctx.decision_maker,
                                final_zone,
                                &ctx.cause,
                                &mut result,
                            );
                            game.record_zone_change_results(
                                object_id,
                                result.new_object_ids.clone(),
                            );
                        }
                        moved_ids.extend(result.new_object_ids.iter().copied());
                        continue;
                    }

                    continue;
                }
                EventOutcome::Replaced => {
                    any_replaced = true;
                }
                EventOutcome::NotApplicable => {
                    continue;
                }
            }
        }

        if !moved_ids.is_empty() {
            return Ok(EffectOutcome::with_objects(moved_ids));
        }
        if any_prevented {
            return Ok(EffectOutcome::prevented());
        }
        if any_replaced {
            return Ok(EffectOutcome::replaced());
        }
        Ok(EffectOutcome::target_invalid())
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "target to move"
    }
}
