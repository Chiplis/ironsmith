//! Untap effect implementation.

use crate::effect::{ChoiceCount, EffectOutcome, EffectResult};
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_objects_from_spec;
use crate::events::PermanentUntappedEvent;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::{ChooseSpec, ObjectFilter};
use crate::triggers::TriggerEvent;

/// Effect that untaps permanents.
///
/// Supports both targeted and non-targeted (all) selection modes.
///
/// # Examples
///
/// ```ignore
/// // Untap target creature (targeted - can fizzle)
/// let effect = UntapEffect::target(ChooseSpec::creature());
///
/// // Untap all creatures you control (non-targeted - cannot fizzle)
/// let effect = UntapEffect::all(ObjectFilter::creature().you_control());
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct UntapEffect {
    /// What to untap - can be targeted, all matching, source, etc.
    pub spec: ChooseSpec,
}

impl UntapEffect {
    /// Create an untap effect with a custom spec.
    pub fn with_spec(spec: ChooseSpec) -> Self {
        Self { spec }
    }

    /// Create a targeted untap effect (single target).
    ///
    /// This is the most common case: "Untap target creature."
    pub fn target(spec: ChooseSpec) -> Self {
        Self {
            spec: ChooseSpec::target(spec),
        }
    }

    /// Create a targeted untap effect with a specific target count.
    ///
    /// Example: "Untap up to two target creatures."
    pub fn targets(spec: ChooseSpec, count: ChoiceCount) -> Self {
        Self {
            spec: ChooseSpec::target(spec).with_count(count),
        }
    }

    /// Create a non-targeted untap effect for all matching permanents.
    ///
    /// Example: "Untap all creatures you control."
    pub fn all(filter: ObjectFilter) -> Self {
        Self {
            spec: ChooseSpec::all(filter),
        }
    }
}

impl EffectExecutor for UntapEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let objects = resolve_objects_from_spec(game, &self.spec, ctx)?;

        if objects.is_empty() {
            return Ok(EffectOutcome::count(0));
        }

        let mut untapped_count = 0;
        let mut events = Vec::new();
        for object_id in objects {
            if game.object(object_id).is_some() && game.is_tapped(object_id) {
                game.untap(object_id);
                untapped_count += 1;
                events.push(TriggerEvent::new(PermanentUntappedEvent::new(object_id)));
            }
        }

        // For targeted single-target effects, return Resolved/TargetInvalid
        // For all effects, return Count
        if self.spec.is_target() && self.spec.is_single() {
            // Single targeted effect - check if target existed
            let target_count = resolve_objects_from_spec(game, &self.spec, ctx)
                .map(|v| v.len())
                .unwrap_or(0);
            if target_count > 0 {
                Ok(EffectOutcome::resolved().with_events(events))
            } else {
                Ok(EffectOutcome::from_result(EffectResult::TargetInvalid).with_events(events))
            }
        } else {
            Ok(EffectOutcome::count(untapped_count).with_events(events))
        }
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
        "permanent to untap"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::effect::EffectResult;
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

    fn create_creature(
        game: &mut GameState,
        name: &str,
        controller: PlayerId,
        tapped: bool,
    ) -> ObjectId {
        let id = game.new_object_id();
        let card = make_creature_card(id.0 as u32, name);
        let obj = Object::from_card(id, &card, controller, Zone::Battlefield);
        game.add_object(obj);
        if tapped {
            game.tap(id);
        }
        id
    }

    // === Targeted untap tests ===

    #[test]
    fn test_untap_tapped_creature() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature(&mut game, "Bear", alice, true);

        assert!(game.is_tapped(creature_id));

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let effect = UntapEffect::target(ChooseSpec::creature());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Resolved);
        assert!(!game.is_tapped(creature_id));
    }

    #[test]
    fn test_untap_already_untapped_creature() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature(&mut game, "Bear", alice, false);

        assert!(!game.is_tapped(creature_id));

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let effect = UntapEffect::target(ChooseSpec::creature());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Still resolves even if already untapped
        assert_eq!(result.result, EffectResult::Resolved);
        assert!(!game.is_tapped(creature_id));
    }

    #[test]
    fn test_untap_nonexistent_target() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let fake_id = game.new_object_id();

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(fake_id)]);

        let effect = UntapEffect::target(ChooseSpec::creature());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // For single target, returns Resolved (target existed in ctx.targets)
        assert_eq!(result.result, EffectResult::Resolved);
    }

    #[test]
    fn test_untap_no_target() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = UntapEffect::target(ChooseSpec::creature());
        let result = effect.execute(&mut game, &mut ctx);

        assert!(result.is_err());
    }

    #[test]
    fn test_untap_get_target_spec() {
        let effect = UntapEffect::target(ChooseSpec::creature());
        assert!(effect.get_target_spec().is_some());
    }

    #[test]
    fn test_untap_clone_box() {
        let effect = UntapEffect::target(ChooseSpec::creature());
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("UntapEffect"));
    }

    // === UntapAll tests (using UntapEffect::all) ===

    #[test]
    fn test_untap_all_creatures() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let creature1 = create_creature(&mut game, "Bear", alice, true);
        let creature2 = create_creature(&mut game, "Wolf", alice, true);
        let creature3 = create_creature(&mut game, "Lion", bob, true);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = UntapEffect::all(ObjectFilter::creature());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(3));
        assert!(!game.is_tapped(creature1));
        assert!(!game.is_tapped(creature2));
        assert!(!game.is_tapped(creature3));
    }

    #[test]
    fn test_untap_all_your_creatures() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let alice_creature = create_creature(&mut game, "Bear", alice, true);
        let bob_creature = create_creature(&mut game, "Wolf", bob, true);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = UntapEffect::all(ObjectFilter::creature().you_control());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(1));
        assert!(!game.is_tapped(alice_creature));
        assert!(game.is_tapped(bob_creature));
    }

    #[test]
    fn test_untap_all_skips_already_untapped() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let creature1 = create_creature(&mut game, "Bear", alice, true);
        let creature2 = create_creature(&mut game, "Wolf", alice, false);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = UntapEffect::all(ObjectFilter::creature());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Only 1 was actually untapped (the tapped one)
        assert_eq!(result.result, EffectResult::Count(1));
        assert!(!game.is_tapped(creature1));
        assert!(!game.is_tapped(creature2));
    }

    #[test]
    fn test_untap_all_no_matching_creatures() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // No creatures exist
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = UntapEffect::all(ObjectFilter::creature());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(0));
    }

    #[test]
    fn test_untap_all_no_target_spec() {
        let effect = UntapEffect::all(ObjectFilter::creature());
        // All effects don't have a target spec
        assert!(effect.get_target_spec().is_none());
    }

    #[test]
    fn test_untap_all_clone_box() {
        let effect = UntapEffect::all(ObjectFilter::creature());
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("UntapEffect"));
    }
}
