//! Return to hand effect implementation.

use crate::effect::{ChoiceCount, EffectOutcome, EffectResult};
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_objects_from_spec;
use crate::event_processor::{EventOutcome, process_zone_change};
use crate::executor::{ExecutionContext, ExecutionError, ResolvedTarget};
use crate::game_state::GameState;
use crate::target::{ChooseSpec, ObjectFilter};
use crate::zone::Zone;

/// Effect that returns permanents to their owners' hands.
///
/// This is commonly called "bouncing" in MTG terminology.
///
/// Supports both targeted and non-targeted (all) selection modes.
///
/// # Examples
///
/// ```ignore
/// // Return target creature to its owner's hand (targeted - can fizzle)
/// let effect = ReturnToHandEffect::target(ChooseSpec::creature());
///
/// // Return all creatures to their owners' hands (non-targeted - cannot fizzle)
/// let effect = ReturnToHandEffect::all(ObjectFilter::creature());
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ReturnToHandEffect {
    /// What to return - can be targeted, all matching, source, etc.
    pub spec: ChooseSpec,
}

impl ReturnToHandEffect {
    /// Create a return to hand effect with a custom spec.
    pub fn with_spec(spec: ChooseSpec) -> Self {
        Self { spec }
    }

    /// Create a targeted return to hand effect (single target).
    pub fn target(spec: ChooseSpec) -> Self {
        Self {
            spec: ChooseSpec::target(spec),
        }
    }

    /// Create a targeted return to hand effect with a specific target count.
    pub fn targets(spec: ChooseSpec, count: ChoiceCount) -> Self {
        Self {
            spec: ChooseSpec::target(spec).with_count(count),
        }
    }

    /// Create a non-targeted return to hand effect for all matching permanents.
    pub fn all(filter: ObjectFilter) -> Self {
        Self {
            spec: ChooseSpec::all(filter),
        }
    }

    /// Create a return to hand effect targeting any creature.
    pub fn creature() -> Self {
        Self::target(ChooseSpec::creature())
    }

    /// Create a return to hand effect targeting any permanent.
    pub fn permanent() -> Self {
        Self::target(ChooseSpec::permanent())
    }

    /// Create an effect that returns all creatures.
    pub fn creatures() -> Self {
        Self::all(ObjectFilter::creature())
    }

    /// Create an effect that returns all nonland permanents.
    pub fn nonland_permanents() -> Self {
        Self::all(ObjectFilter::nonland_permanent())
    }

    /// Helper to return a single object to hand (shared logic).
    fn return_object(
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        object_id: crate::ids::ObjectId,
    ) -> Result<Option<EffectResult>, ExecutionError> {
        if let Some(obj) = game.object(object_id) {
            let from_zone = obj.zone;

            // Process through replacement effects with decision maker
            let result = process_zone_change(
                game,
                object_id,
                from_zone,
                Zone::Hand,
                &mut ctx.decision_maker,
            );

            match result {
                EventOutcome::Prevented => return Ok(Some(EffectResult::Prevented)),
                EventOutcome::Proceed(final_zone) => {
                    game.move_object(object_id, final_zone);
                    return Ok(None); // Successfully returned
                }
                EventOutcome::Replaced => {
                    // Replacement effects already executed
                    return Ok(Some(EffectResult::Replaced));
                }
                EventOutcome::NotApplicable => {
                    return Ok(Some(EffectResult::TargetInvalid));
                }
            }
        }
        // Object doesn't exist - target is invalid
        Ok(Some(EffectResult::TargetInvalid))
    }
}

impl EffectExecutor for ReturnToHandEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        // Handle targeted effects with special single-target behavior
        if self.spec.is_target() && self.spec.is_single() {
            // Original single-target behavior
            for target in ctx.targets.clone() {
                if let ResolvedTarget::Object(object_id) = target {
                    if let Some(result) = Self::return_object(game, ctx, object_id)? {
                        return Ok(EffectOutcome::from_result(result));
                    }
                    return Ok(EffectOutcome::resolved());
                }
            }
            return Ok(EffectOutcome::from_result(EffectResult::TargetInvalid));
        }

        // For all/multi-target effects, use resolve_objects_from_spec
        let objects = match resolve_objects_from_spec(game, &self.spec, ctx) {
            Ok(objs) => objs,
            Err(_) => return Ok(EffectOutcome::from_result(EffectResult::TargetInvalid)),
        };

        let mut returned_count = 0;
        for object_id in objects {
            if game.move_object(object_id, Zone::Hand).is_some() {
                returned_count += 1;
            }
        }

        Ok(EffectOutcome::count(returned_count))
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        if self.spec.is_target() {
            Some(&self.spec)
        } else {
            None
        }
    }

    fn get_target_count(&self) -> Option<crate::effect::ChoiceCount> {
        if self.spec.is_target() {
            Some(self.spec.count())
        } else {
            None
        }
    }

    fn target_description(&self) -> &'static str {
        "permanent to return"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::Object;
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

    fn make_artifact_card(card_id: u32, name: &str) -> crate::card::Card {
        CardBuilder::new(CardId::from_raw(card_id), name)
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]]))
            .card_types(vec![CardType::Artifact])
            .build()
    }

    fn create_artifact(game: &mut GameState, name: &str, controller: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let card = make_artifact_card(id.0 as u32, name);
        let obj = Object::from_card(id, &card, controller, Zone::Battlefield);
        game.add_object(obj);
        id
    }

    // === Targeted return to hand tests ===

    #[test]
    fn test_return_to_hand_creature() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature(&mut game, "Grizzly Bears", alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let effect = ReturnToHandEffect::creature();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Resolved);
        // Creature should no longer be on battlefield
        assert!(!game.battlefield.contains(&creature_id));
        // Should be in owner's hand (new ID per rule 400.7)
        assert!(!game.players[0].hand.is_empty());
    }

    #[test]
    fn test_return_to_hand_no_target() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = ReturnToHandEffect::creature();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::TargetInvalid);
    }

    #[test]
    fn test_return_to_hand_opponent_creature() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let creature_id = create_creature(&mut game, "Hill Giant", bob);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let effect = ReturnToHandEffect::creature();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Resolved);
        assert!(!game.battlefield.contains(&creature_id));
        // Returns to owner's (Bob's) hand, not controller's
        assert!(!game.players[1].hand.is_empty());
    }

    #[test]
    fn test_return_to_hand_clone_box() {
        let effect = ReturnToHandEffect::creature();
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("ReturnToHandEffect"));
    }

    #[test]
    fn test_return_to_hand_get_target_spec() {
        let effect = ReturnToHandEffect::creature();
        assert!(effect.get_target_spec().is_some());
    }

    // === ReturnAllToHand tests (using ReturnToHandEffect::all) ===

    #[test]
    fn test_return_all_creatures() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let creature1 = create_creature(&mut game, "Grizzly Bears", alice);
        let creature2 = create_creature(&mut game, "Hill Giant", bob);
        let _artifact = create_artifact(&mut game, "Sol Ring", alice);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = ReturnToHandEffect::creatures();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(2));
        // Both creatures should be gone from battlefield
        assert!(!game.battlefield.contains(&creature1));
        assert!(!game.battlefield.contains(&creature2));
        // Artifact should remain
        assert_eq!(game.battlefield.len(), 1);
        // Creatures should be in owners' hands (rule 400.7 - new IDs)
        assert!(!game.players[0].hand.is_empty());
        assert!(!game.players[1].hand.is_empty());
    }

    #[test]
    fn test_return_all_to_owners_hands() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Create a creature owned by Bob but controlled by Alice (e.g., via Mind Control)
        let id = game.new_object_id();
        let card = make_creature_card(id.0 as u32, "Stolen Creature");
        let mut obj = Object::from_card(id, &card, bob, Zone::Battlefield);
        obj.controller = alice; // Alice controls it
        game.add_object(obj);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = ReturnToHandEffect::creatures();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(1));
        // Creature returns to owner's (Bob's) hand, not controller's (Alice's)
        assert!(!game.players[1].hand.is_empty());
    }

    #[test]
    fn test_return_all_no_matching() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let _artifact = create_artifact(&mut game, "Sol Ring", alice);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        // No creatures on battlefield
        let effect = ReturnToHandEffect::creatures();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(0));
        // Artifact remains
        assert_eq!(game.battlefield.len(), 1);
    }

    #[test]
    fn test_return_all_nonland_permanents() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature = create_creature(&mut game, "Grizzly Bears", alice);
        let artifact = create_artifact(&mut game, "Sol Ring", alice);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = ReturnToHandEffect::nonland_permanents();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(2));
        assert!(!game.battlefield.contains(&creature));
        assert!(!game.battlefield.contains(&artifact));
    }

    #[test]
    fn test_return_all_clone_box() {
        let effect = ReturnToHandEffect::all(ObjectFilter::creature());
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("ReturnToHandEffect"));
    }

    #[test]
    fn test_return_all_no_target_spec() {
        let effect = ReturnToHandEffect::all(ObjectFilter::creature());
        // All effects don't have a target spec
        assert!(effect.get_target_spec().is_none());
    }
}
