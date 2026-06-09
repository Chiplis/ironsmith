//! Tap effect implementation.

use crate::effect::EffectOutcome;
use crate::effects::helpers::{ObjectApplyResultPolicy, apply_to_selected_objects};
use crate::effects::{CostExecutableEffect, EffectExecutor};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::events::PermanentTappedEvent;
use crate::game_state::GameState;
use crate::target::ChooseSpec;
use crate::triggers::TriggerEvent;
pub use ironsmith_core::TapEffect;

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
impl EffectExecutor for TapEffect {
    fn as_cost_executable(&self) -> Option<&dyn CostExecutableEffect> {
        Some(self)
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let mut events = Vec::new();
        let result_policy = if self.target.is_target() && self.target.is_single() {
            ObjectApplyResultPolicy::SingleTargetResolvedOrInvalid
        } else {
            ObjectApplyResultPolicy::CountApplied
        };
        let provenance = ctx.provenance;

        let apply_result = apply_to_selected_objects(
            game,
            ctx,
            &self.target,
            result_policy,
            |game, _ctx, object_id| {
                if game.object(object_id).is_some() && !game.is_tapped(object_id) {
                    game.tap(object_id);
                    events.push(TriggerEvent::new_with_provenance(
                        PermanentTappedEvent::new(object_id),
                        provenance,
                    ));
                    Ok(true)
                } else {
                    Ok(false)
                }
            },
        )?;

        Ok(apply_result.outcome.with_events(events))
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        if self.target.is_target() {
            Some(&self.target)
        } else {
            None
        }
    }

    fn get_target_count(&self) -> Option<crate::effect::ChoiceCount> {
        if self.target.is_target() {
            Some(self.target.count())
        } else {
            None
        }
    }

    fn target_description(&self) -> &'static str {
        "permanent to tap"
    }
    fn is_tap_source_cost(&self) -> bool {
        matches!(self.target, ChooseSpec::Source)
    }

    fn cost_description(&self) -> Option<String> {
        if matches!(self.target, ChooseSpec::Source) {
            Some("{T}".to_string())
        } else {
            None
        }
    }
}

impl CostExecutableEffect for TapEffect {
    fn can_execute_as_cost(
        &self,
        game: &GameState,
        source: crate::ids::ObjectId,
        _controller: crate::ids::PlayerId,
    ) -> Result<(), crate::effects::CostValidationError> {
        use crate::effects::CostValidationError;

        // Only check for Source selection (tap source as cost)
        if !matches!(self.target, ChooseSpec::Source) {
            return Ok(());
        }

        // Check if source is already tapped
        if game.is_tapped(source) {
            return Err(CostValidationError::AlreadyTapped);
        }

        // Check summoning sickness for creatures
        if game.object(source).is_some()
            && game.current_is_creature(source)
            && game.is_summoning_sick(source)
        {
            if !game.current_has_static_ability_id(
                source,
                crate::static_abilities::StaticAbilityId::Haste,
            ) {
                return Err(CostValidationError::SummoningSickness);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::effect::{ChoiceCount, Value};
    use crate::effects::ResolvedTarget;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::{CounterType, Object};
    use crate::target::{ChooseSpecSurfaceHint, SourceReferenceSurface};
    use crate::test_prelude::*;
    use crate::types::CardType;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
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

    fn create_permanent(
        game: &mut GameState,
        name: &str,
        controller: PlayerId,
        card_types: Vec<CardType>,
    ) -> ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .card_types(card_types)
            .build();
        let obj = Object::from_card(id, &card, controller, Zone::Battlefield);
        game.add_object(obj);
        id
    }

    fn tangle_wire_dynamic_tap_effect() -> TapEffect {
        let mut filter = ObjectFilter::default().in_zone(Zone::Battlefield);
        filter.controller = Some(PlayerFilter::Active);
        filter.card_types = vec![CardType::Artifact, CardType::Creature, CardType::Land];
        filter.untapped = true;

        let source = ChooseSpec::Source.with_surface_hint(
            ChooseSpecSurfaceHint::SourceReference(SourceReferenceSurface::ThisPermanentType(
                "this artifact".to_string(),
            )),
        );
        TapEffect::with_spec(ChooseSpec::Object(filter).with_count_value(
            ChoiceCount::dynamic_x(),
            Value::CountersOn(Box::new(source), Some(CounterType::Fade)),
        ))
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

        assert_eq!(result.status, crate::effect::OutcomeStatus::Succeeded);
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
        assert_eq!(result.status, crate::effect::OutcomeStatus::Succeeded);
        assert!(game.is_tapped(creature_id));
    }

    #[test]
    fn test_tap_nonexistent_target() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let fake_id = game.new_object_id();

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(fake_id)])
            .with_target_assignments(vec![crate::game_state::TargetAssignment {
                spec: ChooseSpec::target(ChooseSpec::creature()),
                range: 0..1,
            }]);

        let effect = TapEffect::target(ChooseSpec::creature());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // For single target, returns Resolved (target existed in ctx.targets)
        // The object just didn't exist in the game
        assert_eq!(result.status, crate::effect::OutcomeStatus::Succeeded);
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

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn test_tap_source_cost_allows_summoning_sick_earthbent_land_with_haste() {
        use crate::cards::definitions::basic_mountain;
        use crate::effect::Effect;
        use crate::effects::EarthbendEffect;
        use crate::effects::execute_effect;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let earthbend_source = create_creature(&mut game, "Kyoshi", alice);
        let land_id =
            game.create_object_from_definition(&basic_mountain(), alice, Zone::Battlefield);

        let effect = Effect::new(EarthbendEffect::new(ChooseSpec::SpecificObject(land_id), 8));
        let mut ctx = ExecutionContext::new_default(earthbend_source, alice);
        execute_effect(&mut game, &effect, &mut ctx).expect("earthbend should resolve");
        game.set_summoning_sick(land_id);

        let tap_cost = TapEffect::source();
        assert_eq!(
            CostExecutableEffect::can_execute_as_cost(&tap_cost, &game, land_id, alice),
            Ok(()),
            "earthbend grants haste, so a summoning-sick animated land should still tap for a source cost"
        );
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

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(3));
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

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(1));
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
        assert_eq!(result.value, crate::effect::OutcomeValue::Count(1));
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

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(0));
    }

    #[test]
    fn test_tap_all_no_target_spec() {
        let effect = TapEffect::all(ObjectFilter::creature());
        // All effects don't have a target spec
        assert!(effect.get_target_spec().is_none());
    }

    #[test]
    fn tangle_wire_taps_active_players_untapped_permanents_for_each_fade_counter() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        game.turn.active_player = bob;

        let tangle_wire =
            create_permanent(&mut game, "Tangle Wire", alice, vec![CardType::Artifact]);
        game.add_counters(tangle_wire, CounterType::Fade, 2);
        let alice_artifact =
            create_permanent(&mut game, "Alice Relic", alice, vec![CardType::Artifact]);
        let bob_artifact =
            create_permanent(&mut game, "Bob Relic", bob, vec![CardType::Artifact]);
        let bob_creature = create_creature(&mut game, "Bob Bear", bob);
        let bob_land = create_permanent(&mut game, "Bob Land", bob, vec![CardType::Land]);

        let effect = tangle_wire_dynamic_tap_effect();
        let mut ctx = ExecutionContext::new_default(tangle_wire, alice);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(2));
        let bob_tapped = [bob_artifact, bob_creature, bob_land]
            .into_iter()
            .filter(|id| game.is_tapped(*id))
            .count();
        assert_eq!(bob_tapped, 2, "exactly two Bob permanents should be tapped");
        assert!(
            !game.is_tapped(alice_artifact),
            "Tangle Wire should not tap the non-active player's permanent"
        );
    }

    #[test]
    fn tangle_wire_taps_no_permanents_when_it_has_no_fade_counters() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        game.turn.active_player = bob;

        let tangle_wire =
            create_permanent(&mut game, "Tangle Wire", alice, vec![CardType::Artifact]);
        let bob_artifact =
            create_permanent(&mut game, "Bob Relic", bob, vec![CardType::Artifact]);
        let bob_creature = create_creature(&mut game, "Bob Bear", bob);

        let effect = tangle_wire_dynamic_tap_effect();
        let mut ctx = ExecutionContext::new_default(tangle_wire, alice);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(0));
        assert!(!game.is_tapped(bob_artifact));
        assert!(!game.is_tapped(bob_creature));
    }

    #[test]
    fn tangle_wire_with_no_fade_counters_allows_no_eligible_permanents() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        game.turn.active_player = bob;

        let tangle_wire =
            create_permanent(&mut game, "Tangle Wire", alice, vec![CardType::Artifact]);

        let effect = tangle_wire_dynamic_tap_effect();
        let mut ctx = ExecutionContext::new_default(tangle_wire, alice);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(0));
    }

    #[test]
    fn tangle_wire_taps_available_permanents_when_fade_counters_exceed_choices() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        game.turn.active_player = bob;

        let tangle_wire =
            create_permanent(&mut game, "Tangle Wire", alice, vec![CardType::Artifact]);
        game.add_counters(tangle_wire, CounterType::Fade, 3);
        let alice_artifact =
            create_permanent(&mut game, "Alice Relic", alice, vec![CardType::Artifact]);
        let bob_artifact =
            create_permanent(&mut game, "Bob Relic", bob, vec![CardType::Artifact]);

        let effect = tangle_wire_dynamic_tap_effect();
        let mut ctx = ExecutionContext::new_default(tangle_wire, alice);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(1));
        assert!(game.is_tapped(bob_artifact));
        assert!(!game.is_tapped(alice_artifact));
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
