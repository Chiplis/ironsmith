//! Destroy effect implementation.

use crate::effect::{ChoiceCount, EffectOutcome, EffectResult};
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_objects_from_spec;
use crate::event_processor::{EventOutcome, process_destroy};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::{ChooseSpec, ObjectFilter};

/// Effect that destroys permanents.
///
/// Destruction moves permanents from the battlefield to the graveyard,
/// subject to replacement effects (regeneration, indestructible, etc.).
///
/// Supports both targeted and non-targeted (all) selection modes.
///
/// # Examples
///
/// ```ignore
/// // Destroy target creature (targeted - can fizzle)
/// let effect = DestroyEffect::target(ChooseSpec::creature());
///
/// // Destroy all creatures (non-targeted - cannot fizzle)
/// let effect = DestroyEffect::all(ObjectFilter::creature());
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct DestroyEffect {
    /// What to destroy - can be targeted, all matching, source, etc.
    pub spec: ChooseSpec,
}

impl DestroyEffect {
    /// Create a destroy effect with a custom spec.
    pub fn with_spec(spec: ChooseSpec) -> Self {
        Self { spec }
    }

    /// Create a targeted destroy effect (single target).
    pub fn target(spec: ChooseSpec) -> Self {
        Self {
            spec: ChooseSpec::target(spec),
        }
    }

    /// Create a targeted destroy effect with a specific target count.
    pub fn targets(spec: ChooseSpec, count: ChoiceCount) -> Self {
        Self {
            spec: ChooseSpec::target(spec).with_count(count),
        }
    }

    /// Create a non-targeted destroy effect for all matching permanents.
    pub fn all(filter: ObjectFilter) -> Self {
        Self {
            spec: ChooseSpec::all(filter),
        }
    }

    /// Create a destroy effect targeting any creature.
    pub fn creature() -> Self {
        Self::target(ChooseSpec::creature())
    }

    /// Create a destroy effect targeting any permanent.
    pub fn permanent() -> Self {
        Self::target(ChooseSpec::permanent())
    }

    /// Helper to destroy a single object (shared logic).
    ///
    /// Uses `process_destroy` to handle all destruction logic through
    /// the trait-based event/replacement system with decision maker support.
    fn destroy_object(
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        object_id: crate::ids::ObjectId,
    ) -> Result<Option<EffectResult>, ExecutionError> {
        let result = process_destroy(game, object_id, Some(ctx.source), &mut ctx.decision_maker);

        match result {
            EventOutcome::Proceed(_) => Ok(None), // Successfully destroyed
            EventOutcome::Prevented => Ok(Some(EffectResult::Protected)),
            EventOutcome::Replaced => Ok(Some(EffectResult::Replaced)),
            EventOutcome::NotApplicable => Ok(Some(EffectResult::TargetInvalid)),
        }
    }
}

impl EffectExecutor for DestroyEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        // Handle targeted effects with special single-target behavior
        if self.spec.is_target() && self.spec.is_single() {
            // Original single-target behavior
            for target in ctx.targets.clone() {
                if let crate::executor::ResolvedTarget::Object(object_id) = target {
                    if let Some(result) = Self::destroy_object(game, ctx, object_id)? {
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

        let mut destroyed_count = 0;
        for object_id in objects {
            let result =
                process_destroy(game, object_id, Some(ctx.source), &mut ctx.decision_maker);

            // For "all" effects, count only actual destructions
            if matches!(result, EventOutcome::Proceed(_)) {
                destroyed_count += 1;
            }
        }

        Ok(EffectOutcome::count(destroyed_count))
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
        "permanent to destroy"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::Ability;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::executor::ResolvedTarget;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::Object;
    use crate::static_abilities::StaticAbility;
    use crate::types::CardType;
    use crate::zone::Zone;

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

    // === Targeted destroy tests ===

    #[test]
    fn test_destroy_creature() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature(&mut game, "Grizzly Bears", alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let effect = DestroyEffect::creature();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Resolved);
        assert!(!game.battlefield.contains(&creature_id));
        assert!(!game.players[0].graveyard.is_empty());
    }

    #[test]
    fn test_destroy_indestructible() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature(&mut game, "Darksteel Colossus", alice);
        let source = game.new_object_id();

        if let Some(obj) = game.object_mut(creature_id) {
            obj.abilities
                .push(Ability::static_ability(StaticAbility::indestructible()));
        }

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let effect = DestroyEffect::creature();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Protected);
        assert!(game.battlefield.contains(&creature_id));
    }

    #[test]
    fn test_destroy_with_regeneration() {
        use crate::effects::RegenerateEffect;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature(&mut game, "Grizzly Bears", alice);
        let source = game.new_object_id();

        // Apply regeneration via the proper effect (creates replacement effect)
        let mut regen_ctx = ExecutionContext::new_default(creature_id, alice);
        RegenerateEffect::source(crate::effect::Until::EndOfTurn)
            .execute(&mut game, &mut regen_ctx)
            .unwrap();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let effect = DestroyEffect::creature();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Replaced);
        assert!(game.battlefield.contains(&creature_id));
        assert!(game.is_tapped(creature_id));
        // One-shot replacement effect should be consumed
        assert_eq!(
            game.replacement_effects
                .count_one_shot_effects_from_source(creature_id),
            0
        );
        assert_eq!(game.damage_on(creature_id), 0);
    }

    #[test]
    fn test_destroy_no_target() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = DestroyEffect::creature();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::TargetInvalid);
    }

    #[test]
    fn test_destroy_target_not_on_battlefield() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature(&mut game, "Grizzly Bears", alice);
        let source = game.new_object_id();

        game.move_object(creature_id, Zone::Graveyard);

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let effect = DestroyEffect::creature();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::TargetInvalid);
    }

    #[test]
    fn test_destroy_clone_box() {
        let effect = DestroyEffect::creature();
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("DestroyEffect"));
    }

    #[test]
    fn test_destroy_get_target_spec() {
        let effect = DestroyEffect::creature();
        assert!(effect.get_target_spec().is_some());
    }

    // === DestroyAll tests ===

    #[test]
    fn test_destroy_all_creatures() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let creature1 = create_creature(&mut game, "Grizzly Bears", alice);
        let creature2 = create_creature(&mut game, "Hill Giant", bob);
        let _artifact = create_artifact(&mut game, "Sol Ring", alice);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = DestroyEffect::all(ObjectFilter::creature());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(2));
        assert!(!game.battlefield.contains(&creature1));
        assert!(!game.battlefield.contains(&creature2));
        assert_eq!(game.battlefield.len(), 1); // Only artifact remains
    }

    #[test]
    fn test_destroy_all_indestructible_survives() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature(&mut game, "Darksteel Colossus", alice);
        let source = game.new_object_id();

        if let Some(obj) = game.object_mut(creature_id) {
            obj.abilities
                .push(Ability::static_ability(StaticAbility::indestructible()));
        }

        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = DestroyEffect::all(ObjectFilter::creature());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(0));
        assert!(game.battlefield.contains(&creature_id));
    }

    #[test]
    fn test_destroy_all_regeneration_used() {
        use crate::effects::RegenerateEffect;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature(&mut game, "River Boa", alice);
        let source = game.new_object_id();

        // Apply regeneration via the proper effect (creates replacement effect)
        let mut regen_ctx = ExecutionContext::new_default(creature_id, alice);
        RegenerateEffect::source(crate::effect::Until::EndOfTurn)
            .execute(&mut game, &mut regen_ctx)
            .unwrap();

        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = DestroyEffect::all(ObjectFilter::creature());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(0));
        assert!(game.battlefield.contains(&creature_id));
        assert!(game.is_tapped(creature_id));
        // One-shot replacement effect should be consumed
        assert_eq!(
            game.replacement_effects
                .count_one_shot_effects_from_source(creature_id),
            0
        );
        assert_eq!(game.damage_on(creature_id), 0);
    }

    #[test]
    fn test_destroy_all_no_matching() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let _artifact = create_artifact(&mut game, "Sol Ring", alice);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = DestroyEffect::all(ObjectFilter::creature());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(0));
        assert_eq!(game.battlefield.len(), 1);
    }

    #[test]
    fn test_destroy_all_no_target_spec() {
        let effect = DestroyEffect::all(ObjectFilter::creature());
        assert!(effect.get_target_spec().is_none());
    }

    #[test]
    fn test_destroy_all_clone_box() {
        let effect = DestroyEffect::all(ObjectFilter::creature());
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("DestroyEffect"));
    }
}
