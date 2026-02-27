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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::effect::ChoiceCount;
    use crate::executor::ResolvedTarget;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::Object;
    use crate::target::ObjectFilter;
    use crate::types::CardType;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    fn make_creature_card(card_id: u32, name: &str) -> crate::card::Card {
        CardBuilder::new(CardId::from_raw(card_id), name)
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Green],
            ]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build()
    }

    fn create_creature(game: &mut GameState, name: &str, controller: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let card = make_creature_card(id.0 as u32, name);
        let obj = Object::from_card(id, &card, controller, Zone::Battlefield);
        game.add_object(obj);
        id
    }

    #[test]
    fn test_move_to_graveyard() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature(&mut game, "Grizzly Bears", alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let effect = MoveToZoneEffect::to_graveyard(ChooseSpec::creature());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert!(
            matches!(result.result, EffectResult::Objects(ref ids) if ids.len() == 1),
            "Expected moved object id"
        );
        assert!(!game.battlefield.contains(&creature_id));
        // Object has new ID in graveyard (rule 400.7)
        assert!(!game.players[0].graveyard.is_empty());
    }

    #[test]
    fn test_move_to_exile() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature(&mut game, "Grizzly Bears", alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let effect = MoveToZoneEffect::to_exile(ChooseSpec::creature());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert!(
            matches!(result.result, EffectResult::Objects(ref ids) if ids.len() == 1),
            "Expected moved object id"
        );
        assert!(!game.battlefield.contains(&creature_id));
        assert!(!game.exile.is_empty());
    }

    #[test]
    fn test_move_to_top_of_library() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature(&mut game, "Grizzly Bears", alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let effect = MoveToZoneEffect::to_top_of_library(ChooseSpec::creature());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert!(
            matches!(result.result, EffectResult::Objects(ref ids) if ids.len() == 1),
            "Expected moved object id"
        );
        assert!(!game.battlefield.contains(&creature_id));
        // Object has new ID in library (rule 400.7)
        assert!(!game.players[0].library.is_empty());
    }

    #[test]
    fn test_move_no_target() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = MoveToZoneEffect::to_graveyard(ChooseSpec::creature());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::TargetInvalid);
    }

    #[test]
    fn test_move_to_zone_moves_all_resolved_targets() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_a = create_creature(&mut game, "A", alice);
        let creature_b = create_creature(&mut game, "B", alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice).with_targets(vec![
            crate::executor::ResolvedTarget::Object(creature_a),
            crate::executor::ResolvedTarget::Object(creature_b),
        ]);

        let target_spec = ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature()))
            .with_count(ChoiceCount::exactly(2));
        let effect = MoveToZoneEffect::to_graveyard(target_spec);
        let result = effect.execute(&mut game, &mut ctx).expect("move should resolve");

        let moved_ids = match result.result {
            EffectResult::Objects(ids) => ids,
            other => panic!("expected Objects result, got {other:?}"),
        };
        assert_eq!(moved_ids.len(), 2, "expected two moved objects");
        assert!(!game.battlefield.contains(&creature_a));
        assert!(!game.battlefield.contains(&creature_b));
    }

    #[test]
    fn test_move_to_zone_clone_box() {
        let effect = MoveToZoneEffect::to_graveyard(ChooseSpec::creature());
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("MoveToZoneEffect"));
    }

    #[test]
    fn test_move_to_zone_get_target_spec() {
        let effect = MoveToZoneEffect::to_exile(ChooseSpec::creature());
        assert!(effect.get_target_spec().is_some());
    }
}
