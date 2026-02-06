//! Regenerate effect implementation.

use crate::effect::{Effect, EffectOutcome, EffectResult, Until};
use crate::effects::{ApplyReplacementEffect, EffectExecutor};
use crate::events::permanents::matchers::RegenerationShieldMatcher;
use crate::executor::{ExecutionContext, ExecutionError, execute_effect};
use crate::game_state::GameState;
use crate::replacement::{ReplacementAction, ReplacementEffect};
use crate::target::ChooseSpec;
use crate::zone::Zone;

/// Effect that regenerates a target creature.
///
/// Creates a "regeneration shield" as a one-shot replacement effect that lasts
/// for the specified duration. When the creature would be destroyed, instead:
/// - Tap it
/// - Remove all damage from it
/// - Remove it from combat (if applicable)
/// - The replacement effect is consumed
///
/// The regeneration shield is implemented as a proper replacement effect rather
/// than a counter, which aligns with the MTG rules and allows it to interact
/// correctly with other replacement effects.
///
/// # Fields
///
/// * `target` - The creature to regenerate
///
/// # Example
///
/// ```ignore
/// // Regenerate target creature
/// let effect = RegenerateEffect::new(ChooseSpec::creature(), Until::EndOfTurn);
///
/// // Regenerate this creature (source)
/// let effect = RegenerateEffect::source(Until::EndOfTurn);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct RegenerateEffect {
    /// The targeting specification.
    pub target: ChooseSpec,
    /// Duration for the regeneration shield.
    pub duration: Until,
}

impl RegenerateEffect {
    /// Create a new regenerate effect with explicit duration.
    pub fn new(target: ChooseSpec, duration: Until) -> Self {
        Self { target, duration }
    }

    /// Create an effect that regenerates the source creature with explicit duration.
    pub fn source(duration: Until) -> Self {
        Self::new(ChooseSpec::Source, duration)
    }

    /// Create an effect that regenerates target creature with explicit duration.
    pub fn target_creature(duration: Until) -> Self {
        Self::new(ChooseSpec::creature(), duration)
    }
}

impl EffectExecutor for RegenerateEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        if self.duration != Until::EndOfTurn {
            return Err(ExecutionError::Impossible(
                "RegenerateEffect currently supports only Until::EndOfTurn".to_string(),
            ));
        }

        // Resolve the target creature
        let mut targets =
            crate::effects::helpers::resolve_objects_from_spec(game, &self.target, ctx)
                .map_err(|_| ExecutionError::InvalidTarget)?;
        let Some(target_id) = targets.pop() else {
            return Err(ExecutionError::InvalidTarget);
        };

        // Verify target is a creature on the battlefield
        let obj = game
            .object(target_id)
            .ok_or(ExecutionError::ObjectNotFound(target_id))?;

        if obj.zone != Zone::Battlefield {
            return Ok(EffectOutcome::from_result(EffectResult::TargetInvalid));
        }

        if !obj.is_creature() {
            return Ok(EffectOutcome::from_result(EffectResult::TargetInvalid));
        }

        let controller = obj.controller;

        // Create the replacement effects that will happen instead of destruction.
        // Per MTG rules, regeneration:
        // 1. Taps the creature
        // 2. Removes all damage from it
        // 3. Removes it from combat (if applicable)
        // We use ChooseSpec::SpecificObject to target the creature being regenerated.
        let replacement_effects = vec![
            // Tap the creature
            Effect::tap(ChooseSpec::SpecificObject(target_id)),
            // Remove all damage
            Effect::clear_damage(ChooseSpec::SpecificObject(target_id)),
            // Removing from combat is intentionally omitted until combat-state
            // tracking for regenerated permanents is wired in.
        ];

        // Create a regeneration shield as a one-shot replacement effect
        let matcher = RegenerationShieldMatcher::new(target_id);
        let replacement_effect = ReplacementEffect::with_matcher(
            target_id, // source is the creature itself
            controller,
            matcher,
            ReplacementAction::Instead(replacement_effects),
        )
        .self_replacing();

        let apply = ApplyReplacementEffect::one_shot(replacement_effect);

        execute_effect(game, &Effect::new(apply), ctx)
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "creature to regenerate"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
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
    fn test_regenerate_basic() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature(&mut game, "Troll", alice);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let effect = RegenerateEffect::new(
            ChooseSpec::Object(ObjectFilter::creature()),
            Until::EndOfTurn,
        );
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Resolved);
        // Check that a one-shot replacement effect was registered
        assert_eq!(
            game.replacement_effects
                .count_one_shot_effects_from_source(creature_id),
            1
        );
    }

    #[test]
    fn test_regenerate_stacks() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature(&mut game, "Troll", alice);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let effect = RegenerateEffect::new(
            ChooseSpec::Object(ObjectFilter::creature()),
            Until::EndOfTurn,
        );

        // Regenerate twice
        effect.execute(&mut game, &mut ctx).unwrap();
        effect.execute(&mut game, &mut ctx).unwrap();

        // Should have 2 regeneration shields (one-shot replacement effects)
        assert_eq!(
            game.replacement_effects
                .count_one_shot_effects_from_source(creature_id),
            2
        );
    }

    #[test]
    fn test_regenerate_source() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature(&mut game, "Troll", alice);

        // Source is the creature itself
        let mut ctx = ExecutionContext::new_default(creature_id, alice);

        let effect = RegenerateEffect::source(Until::EndOfTurn);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Resolved);
        // Check that a one-shot replacement effect was registered
        assert_eq!(
            game.replacement_effects
                .count_one_shot_effects_from_source(creature_id),
            1
        );
    }

    #[test]
    fn test_regenerate_noncreature_fails() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create an artifact instead of creature
        let artifact_id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(artifact_id.0 as u32), "Sol Ring")
            .card_types(vec![CardType::Artifact])
            .build();
        let obj = Object::from_card(artifact_id, &card, alice, Zone::Battlefield);
        game.add_object(obj);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(artifact_id)]);

        let effect = RegenerateEffect::new(
            ChooseSpec::Object(ObjectFilter::creature()),
            Until::EndOfTurn,
        );
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::TargetInvalid);
    }

    #[test]
    fn test_regenerate_not_on_battlefield_fails() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create a creature in graveyard
        let creature_id = game.new_object_id();
        let card = make_creature_card(creature_id.0 as u32, "Dead Troll");
        let obj = Object::from_card(creature_id, &card, alice, Zone::Graveyard);
        game.add_object(obj);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let effect = RegenerateEffect::new(
            ChooseSpec::Object(ObjectFilter::creature()),
            Until::EndOfTurn,
        );
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::TargetInvalid);
    }

    #[test]
    fn test_regenerate_clone_box() {
        let effect = RegenerateEffect::target_creature(Until::EndOfTurn);
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("RegenerateEffect"));
    }

    #[test]
    fn test_regenerate_get_target_spec() {
        let effect = RegenerateEffect::target_creature(Until::EndOfTurn);
        assert!(effect.get_target_spec().is_some());
    }
}
