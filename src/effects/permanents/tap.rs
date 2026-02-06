//! Tap effect implementation.

use crate::effect::{ChoiceCount, EffectOutcome, EffectResult};
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_objects_from_spec;
use crate::events::PermanentTappedEvent;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::{ChooseSpec, ObjectFilter};
use crate::triggers::TriggerEvent;

/// Effect that taps permanents.
///
/// Supports both targeted and non-targeted (all) selection modes.
///
/// # Examples
///
/// ```ignore
/// // Tap target creature (targeted - can fizzle)
/// let effect = TapEffect::target(ChooseSpec::creature());
///
/// // Tap all creatures (non-targeted - cannot fizzle)
/// let effect = TapEffect::all(ObjectFilter::creature());
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct TapEffect {
    /// What to tap - can be targeted, all matching, source, etc.
    pub spec: ChooseSpec,
}

impl TapEffect {
    /// Create a tap effect with a custom spec.
    pub fn with_spec(spec: ChooseSpec) -> Self {
        Self { spec }
    }

    /// Create a targeted tap effect (single target).
    ///
    /// This is the most common case: "Tap target creature."
    pub fn target(spec: ChooseSpec) -> Self {
        Self {
            spec: ChooseSpec::target(spec),
        }
    }

    /// Create a targeted tap effect with a specific target count.
    ///
    /// Example: "Tap up to two target creatures."
    pub fn targets(spec: ChooseSpec, count: ChoiceCount) -> Self {
        Self {
            spec: ChooseSpec::target(spec).with_count(count),
        }
    }

    /// Create a non-targeted tap effect for all matching permanents.
    ///
    /// Example: "Tap all creatures you don't control."
    pub fn all(filter: ObjectFilter) -> Self {
        Self {
            spec: ChooseSpec::all(filter),
        }
    }

    /// Create a tap effect that taps the source permanent.
    ///
    /// Used for cost effects that require tapping the source.
    pub fn source() -> Self {
        Self {
            spec: ChooseSpec::Source,
        }
    }
}

impl EffectExecutor for TapEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let objects = resolve_objects_from_spec(game, &self.spec, ctx)?;

        if objects.is_empty() {
            return Ok(EffectOutcome::count(0));
        }

        let mut tapped_count = 0;
        let mut events = Vec::new();
        for object_id in objects {
            if game.object(object_id).is_some() && !game.is_tapped(object_id) {
                game.tap(object_id);
                tapped_count += 1;
                events.push(TriggerEvent::new(PermanentTappedEvent::new(object_id)));
            }
        }

        // For targeted single-target effects, return Resolved/TargetInvalid
        // For all effects, return Count
        let outcome = if self.spec.is_target() && self.spec.is_single() {
            // Single targeted effect - check if target existed
            let target_count = resolve_objects_from_spec(game, &self.spec, ctx)
                .map(|v| v.len())
                .unwrap_or(0);
            if target_count > 0 {
                EffectOutcome::resolved()
            } else {
                EffectOutcome::from_result(EffectResult::TargetInvalid)
            }
        } else {
            EffectOutcome::count(tapped_count)
        };

        Ok(outcome.with_events(events))
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
        "permanent to tap"
    }

    fn can_execute_as_cost(
        &self,
        game: &GameState,
        source: crate::ids::ObjectId,
        _controller: crate::ids::PlayerId,
    ) -> Result<(), crate::effects::CostValidationError> {
        use crate::ability::AbilityKind;
        use crate::effects::CostValidationError;

        // Only check for Source selection (tap source as cost)
        if !matches!(self.spec, ChooseSpec::Source) {
            return Ok(());
        }

        // Check if source is already tapped
        if game.is_tapped(source) {
            return Err(CostValidationError::AlreadyTapped);
        }

        // Check summoning sickness for creatures
        if let Some(obj) = game.object(source)
            && obj.is_creature()
            && game.is_summoning_sick(source)
        {
            // Check for haste
            let has_haste = obj.abilities.iter().any(|a| {
                if let AbilityKind::Static(s) = &a.kind {
                    s.has_haste()
                } else {
                    false
                }
            });
            if !has_haste {
                return Err(CostValidationError::SummoningSickness);
            }
        }

        Ok(())
    }

    fn is_tap_source_cost(&self) -> bool {
        matches!(self.spec, ChooseSpec::Source)
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

    // === Targeted tap tests ===

    #[test]
    fn test_tap_untapped_creature() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature(&mut game, "Bear", alice);

        assert!(!game.is_tapped(creature_id));

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let effect = TapEffect::target(ChooseSpec::creature());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Resolved);
        assert!(game.is_tapped(creature_id));
    }

    #[test]
    fn test_tap_already_tapped_creature() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature(&mut game, "Bear", alice);
        game.tap(creature_id);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let effect = TapEffect::target(ChooseSpec::creature());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Still resolves even if already tapped
        assert_eq!(result.result, EffectResult::Resolved);
        assert!(game.is_tapped(creature_id));
    }

    #[test]
    fn test_tap_nonexistent_target() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let fake_id = game.new_object_id();

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(fake_id)]);

        let effect = TapEffect::target(ChooseSpec::creature());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // For single target, returns Resolved (target existed in ctx.targets)
        // The object just didn't exist in the game
        assert_eq!(result.result, EffectResult::Resolved);
    }

    #[test]
    fn test_tap_no_target() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = TapEffect::target(ChooseSpec::creature());
        let result = effect.execute(&mut game, &mut ctx);

        assert!(result.is_err());
    }

    #[test]
    fn test_tap_get_target_spec() {
        let effect = TapEffect::target(ChooseSpec::creature());
        assert!(effect.get_target_spec().is_some());
    }

    #[test]
    fn test_tap_clone_box() {
        let effect = TapEffect::target(ChooseSpec::creature());
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("TapEffect"));
    }

    // === TapAll tests (using TapEffect::all) ===

    #[test]
    fn test_tap_all_creatures() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let creature1 = create_creature(&mut game, "Bear", alice);
        let creature2 = create_creature(&mut game, "Wolf", alice);
        let creature3 = create_creature(&mut game, "Lion", bob);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = TapEffect::all(ObjectFilter::creature());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(3));
        assert!(game.is_tapped(creature1));
        assert!(game.is_tapped(creature2));
        assert!(game.is_tapped(creature3));
    }

    #[test]
    fn test_tap_all_opponent_creatures() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let alice_creature = create_creature(&mut game, "Bear", alice);
        let bob_creature = create_creature(&mut game, "Wolf", bob);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = TapEffect::all(ObjectFilter::creature().opponent_controls());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(1));
        assert!(!game.is_tapped(alice_creature));
        assert!(game.is_tapped(bob_creature));
    }

    #[test]
    fn test_tap_all_skips_already_tapped() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let creature1 = create_creature(&mut game, "Bear", alice);
        let creature2 = create_creature(&mut game, "Wolf", alice);
        game.tap(creature1);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = TapEffect::all(ObjectFilter::creature());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Only 1 was actually tapped (the untapped one)
        assert_eq!(result.result, EffectResult::Count(1));
        assert!(game.is_tapped(creature1));
        assert!(game.is_tapped(creature2));
    }

    #[test]
    fn test_tap_all_no_matching_creatures() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // No creatures exist
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = TapEffect::all(ObjectFilter::creature());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(0));
    }

    #[test]
    fn test_tap_all_no_target_spec() {
        let effect = TapEffect::all(ObjectFilter::creature());
        // All effects don't have a target spec
        assert!(effect.get_target_spec().is_none());
    }

    #[test]
    fn test_tap_all_clone_box() {
        let effect = TapEffect::all(ObjectFilter::creature());
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("TapEffect"));
    }

    #[test]
    fn test_tap_returns_event() {
        use crate::events::EventKind;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature(&mut game, "Bear", alice);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let effect = TapEffect::target(ChooseSpec::creature());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.events.len(), 1);
        assert_eq!(result.events[0].kind(), EventKind::PermanentTapped);
    }

    #[test]
    fn test_tap_all_returns_multiple_events() {
        use crate::events::EventKind;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        create_creature(&mut game, "Bear", alice);
        create_creature(&mut game, "Wolf", alice);
        create_creature(&mut game, "Lion", alice);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = TapEffect::all(ObjectFilter::creature());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.events.len(), 3);
        for event in &result.events {
            assert_eq!(event.kind(), EventKind::PermanentTapped);
        }
    }

    #[test]
    fn test_tap_already_tapped_no_event() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature(&mut game, "Bear", alice);
        game.tap(creature_id);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let effect = TapEffect::target(ChooseSpec::creature());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // No event when already tapped
        assert!(result.events.is_empty());
    }
}
