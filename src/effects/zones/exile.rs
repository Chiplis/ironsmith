//! Exile effect implementation.

use crate::effect::{ChoiceCount, EffectOutcome, EffectResult};
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_objects_from_spec;
use crate::event_processor::{EventOutcome, process_zone_change};
use crate::executor::{ExecutionContext, ExecutionError, ResolvedTarget};
use crate::game_state::GameState;
use crate::target::{ChooseSpec, ObjectFilter};
use crate::zone::Zone;

/// Effect that exiles permanents.
///
/// Exile moves an object to the exile zone, subject to replacement effects.
/// Unlike destroy, exile is not affected by indestructible.
///
/// Supports both targeted and non-targeted (all) selection modes.
///
/// # Examples
///
/// ```ignore
/// // Exile target creature (targeted - can fizzle)
/// let effect = ExileEffect::target(ChooseSpec::creature());
///
/// // Exile all creatures (non-targeted - cannot fizzle)
/// let effect = ExileEffect::all(ObjectFilter::creature());
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ExileEffect {
    /// What to exile - can be targeted, all matching, source, etc.
    pub spec: ChooseSpec,
}

impl ExileEffect {
    /// Create an exile effect with a custom spec.
    pub fn with_spec(spec: ChooseSpec) -> Self {
        Self { spec }
    }

    /// Create a targeted exile effect (single target).
    pub fn target(spec: ChooseSpec) -> Self {
        Self {
            spec: ChooseSpec::target(spec),
        }
    }

    /// Create a targeted exile effect with a specific target count.
    pub fn targets(spec: ChooseSpec, count: ChoiceCount) -> Self {
        Self {
            spec: ChooseSpec::target(spec).with_count(count),
        }
    }

    /// Create a non-targeted exile effect for all matching permanents.
    pub fn all(filter: ObjectFilter) -> Self {
        Self {
            spec: ChooseSpec::all(filter),
        }
    }

    /// Create an exile effect targeting a single creature.
    pub fn creature() -> Self {
        Self::target(ChooseSpec::creature())
    }

    /// Create an exile effect targeting a single permanent.
    pub fn permanent() -> Self {
        Self::target(ChooseSpec::permanent())
    }

    /// Create an exile effect targeting any number of targets.
    pub fn any_number(target: ChooseSpec) -> Self {
        Self::targets(target, ChoiceCount::any_number())
    }

    /// Create an exile effect for a specific object.
    pub fn specific(object_id: crate::ids::ObjectId) -> Self {
        Self {
            spec: ChooseSpec::SpecificObject(object_id),
        }
    }

    /// Helper for convenience constructors that mirror ExileAllEffect.
    pub fn creatures() -> Self {
        Self::all(ObjectFilter::creature())
    }

    /// Create an effect that exiles all nonland permanents.
    pub fn nonland_permanents() -> Self {
        Self::all(ObjectFilter::nonland_permanent())
    }

    /// Helper to exile a single object (shared logic).
    fn exile_object(
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
                Zone::Exile,
                &mut ctx.decision_maker,
            );

            match result {
                EventOutcome::Prevented => return Ok(Some(EffectResult::Prevented)),
                EventOutcome::Proceed(final_zone) => {
                    game.move_object(object_id, final_zone);
                    return Ok(None); // Successfully exiled
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

    /// Check if spec uses ctx.targets (Object/Player/AnyTarget filters)
    fn uses_ctx_targets(&self) -> bool {
        matches!(
            self.spec.base(),
            ChooseSpec::Object(_) | ChooseSpec::Player(_) | ChooseSpec::AnyTarget
        )
    }
}

impl EffectExecutor for ExileEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        // Handle targeted effects with special single-target behavior
        // BUT skip for special specs (Tagged, Source, SpecificObject) which don't use ctx.targets
        if self.spec.is_target() && self.uses_ctx_targets() {
            let count = self.spec.count();
            if count.is_single() {
                // Original single-target behavior
                for target in ctx.targets.clone() {
                    if let ResolvedTarget::Object(object_id) = target {
                        if let Some(result) = Self::exile_object(game, ctx, object_id)? {
                            return Ok(EffectOutcome::from_result(result));
                        }
                        return Ok(EffectOutcome::resolved());
                    }
                }
                return Ok(EffectOutcome::from_result(EffectResult::TargetInvalid));
            }
            // Multi-target with count - handle "any number" specially
            if count.min == 0 {
                // "any number" effects - 0 targets is valid
                let mut exiled_count = 0;
                for target in ctx.targets.clone() {
                    if let ResolvedTarget::Object(object_id) = target
                        && Self::exile_object(game, ctx, object_id)?.is_none()
                    {
                        exiled_count += 1;
                    }
                }
                return Ok(EffectOutcome::count(exiled_count));
            }
        }

        // For all/non-targeted effects and special specs (Tagged, Source, etc.),
        // use resolve_objects_from_spec which handles them correctly
        let objects = match resolve_objects_from_spec(game, &self.spec, ctx) {
            Ok(objs) => objs,
            Err(_) => return Ok(EffectOutcome::from_result(EffectResult::TargetInvalid)),
        };

        let mut exiled_count = 0;
        for object_id in objects {
            if game.move_object(object_id, Zone::Exile).is_some() {
                exiled_count += 1;
            }
        }

        Ok(EffectOutcome::count(exiled_count))
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
        "target to exile"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::Ability;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::Object;
    use crate::static_abilities::StaticAbility;
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

    // === Targeted exile tests ===

    #[test]
    fn test_exile_creature() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature(&mut game, "Grizzly Bears", alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let effect = ExileEffect::creature();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Resolved);
        // Creature should no longer be on battlefield
        assert!(!game.battlefield.contains(&creature_id));
        // Object should have a new ID in exile (per rule 400.7)
        assert!(!game.exile.is_empty());
    }

    #[test]
    fn test_exile_indestructible_creature() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature(&mut game, "Darksteel Colossus", alice);
        let source = game.new_object_id();

        // Make it indestructible
        if let Some(obj) = game.object_mut(creature_id) {
            obj.abilities
                .push(Ability::static_ability(StaticAbility::indestructible()));
        }

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let effect = ExileEffect::creature();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Indestructible doesn't prevent exile
        assert_eq!(result.result, EffectResult::Resolved);
        assert!(!game.battlefield.contains(&creature_id));
        assert!(!game.exile.is_empty());
    }

    #[test]
    fn test_exile_multiple_targets() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature1 = create_creature(&mut game, "Grizzly Bears", alice);
        let creature2 = create_creature(&mut game, "Hill Giant", alice);
        let creature3 = create_creature(&mut game, "Air Elemental", alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice).with_targets(vec![
            ResolvedTarget::Object(creature1),
            ResolvedTarget::Object(creature2),
            ResolvedTarget::Object(creature3),
        ]);

        let effect = ExileEffect::any_number(ChooseSpec::creature());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(3));
        // All creatures should be gone from battlefield
        assert!(!game.battlefield.contains(&creature1));
        assert!(!game.battlefield.contains(&creature2));
        assert!(!game.battlefield.contains(&creature3));
        // Should have 3 objects in exile (new IDs)
        assert_eq!(game.exile.len(), 3);
    }

    #[test]
    fn test_exile_no_target() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = ExileEffect::creature();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::TargetInvalid);
    }

    #[test]
    fn test_exile_any_number_zero_is_valid() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = ExileEffect::any_number(ChooseSpec::creature());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // For "any number" effects, 0 is valid
        assert_eq!(result.result, EffectResult::Count(0));
    }

    #[test]
    fn test_exile_specific_object() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature(&mut game, "Grizzly Bears", alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let effect = ExileEffect::specific(creature_id);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // SpecificObject now goes through resolve_object_selection which returns Count
        assert_eq!(result.result, EffectResult::Count(1));
        assert!(!game.battlefield.contains(&creature_id));
        assert!(!game.exile.is_empty());
    }

    #[test]
    fn test_exile_clone_box() {
        let effect = ExileEffect::creature();
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("ExileEffect"));
    }

    #[test]
    fn test_exile_get_target_spec() {
        let effect = ExileEffect::creature();
        assert!(effect.get_target_spec().is_some());
    }

    // === ExileAll tests (using ExileEffect::all) ===

    #[test]
    fn test_exile_all_creatures() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let creature1 = create_creature(&mut game, "Grizzly Bears", alice);
        let creature2 = create_creature(&mut game, "Hill Giant", bob);
        let _artifact = create_artifact(&mut game, "Sol Ring", alice);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = ExileEffect::creatures();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(2));
        // Both creatures should be gone
        assert!(!game.battlefield.contains(&creature1));
        assert!(!game.battlefield.contains(&creature2));
        // Artifact should remain
        assert_eq!(game.battlefield.len(), 1);
        // Creatures should be in exile
        assert!(!game.exile.is_empty());
    }

    #[test]
    fn test_exile_all_ignores_indestructible() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature(&mut game, "Darksteel Colossus", alice);
        let source = game.new_object_id();

        // Make it indestructible
        if let Some(obj) = game.object_mut(creature_id) {
            obj.abilities
                .push(Ability::static_ability(StaticAbility::indestructible()));
        }

        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = ExileEffect::creatures();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Exile ignores indestructible
        assert_eq!(result.result, EffectResult::Count(1));
        assert!(!game.battlefield.contains(&creature_id));
    }

    #[test]
    fn test_exile_all_no_matching() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let _artifact = create_artifact(&mut game, "Sol Ring", alice);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        // No creatures on battlefield
        let effect = ExileEffect::creatures();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(0));
        // Artifact remains
        assert_eq!(game.battlefield.len(), 1);
    }

    #[test]
    fn test_exile_all_nonland_permanents() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature = create_creature(&mut game, "Grizzly Bears", alice);
        let artifact = create_artifact(&mut game, "Sol Ring", alice);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = ExileEffect::nonland_permanents();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(2));
        assert!(!game.battlefield.contains(&creature));
        assert!(!game.battlefield.contains(&artifact));
    }

    #[test]
    fn test_exile_all_clone_box() {
        let effect = ExileEffect::all(ObjectFilter::creature());
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("ExileEffect"));
    }

    #[test]
    fn test_exile_all_no_target_spec() {
        let effect = ExileEffect::all(ObjectFilter::creature());
        // All effects don't have a target spec
        assert!(effect.get_target_spec().is_none());
    }
}
