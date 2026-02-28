//! Move to zone effect implementation.

use crate::effect::{EffectOutcome, EffectResult};
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_objects_from_spec;
use crate::event_processor::{EventOutcome, process_zone_change};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::ChooseSpec;
use crate::zone::Zone;

use super::{BattlefieldEntryOptions, BattlefieldEntryOutcome, move_to_battlefield_with_options};

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
/// * `target` - Which object to move (resolved from ctx.targets)
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
        let object_ids = resolve_objects_from_spec(game, &self.target, ctx)?;
        if object_ids.is_empty() {
            return Ok(EffectOutcome::from_result(EffectResult::TargetInvalid));
        }

        let mut moved_ids = Vec::new();
        let mut any_prevented = false;
        let mut any_replaced = false;

        for object_id in object_ids {
            let Some(obj) = game.object(object_id) else {
                continue;
            };
            let from_zone = obj.zone;

            // Process through replacement effects with decision maker
            let result = process_zone_change(
                game,
                object_id,
                from_zone,
                self.zone,
                &mut ctx.decision_maker,
            );

            match result {
                EventOutcome::Prevented => {
                    return Ok(EffectOutcome::from_result(EffectResult::Prevented));
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

                    if let Some(new_id) = game.move_object(object_id, final_zone) {
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
                        moved_ids.push(new_id);
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
            return Ok(EffectOutcome::from_result(EffectResult::Objects(moved_ids)));
        }
        if any_prevented {
            return Ok(EffectOutcome::from_result(EffectResult::Prevented));
        }
        if any_replaced {
            return Ok(EffectOutcome::from_result(EffectResult::Replaced));
        }
        Ok(EffectOutcome::from_result(EffectResult::TargetInvalid))
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "target to move"
    }
}
