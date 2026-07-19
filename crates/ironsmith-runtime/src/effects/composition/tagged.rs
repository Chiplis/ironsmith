//! Tagged effect implementation.
//!
//! This effect wrapper captures the target object as a tagged snapshot
//! that can be referenced by subsequent effects in the same spell/ability.

use crate::effect::{Effect, EffectOutcome};
use crate::effects::{
    CostExecutableEffect, CostValidationError, EffectExecutor, TargetReusePolicy,
};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::events::{DamageEvent, DamageTarget};
use crate::game_state::GameState;
use crate::snapshot::ObjectSnapshot;
use crate::tag::TagKey;
pub type TaggedEffect = ironsmith_core::TaggedEffect<crate::effect::Effect>;

use super::tagging_runtime::{
    TaggedRuntimeState, apply_tagged_runtime_state, capture_all_effect_target_snapshots,
    capture_tagged_runtime_state,
};

/// Effect that executes an inner effect and tags its target for later reference.
///
/// This enables patterns like "Destroy target permanent. Its controller creates a token."
/// where the second effect needs to reference the controller of the destroyed permanent.
///
/// # How it works
///
/// 1. Before executing the inner effect, this wrapper captures a snapshot of the
///    first object target (if any) and stores it under the given tag name.
/// 2. The inner effect is executed.
/// 3. Subsequent effects can reference the tagged object using
///    `PlayerFilter::ControllerOf(ObjectRef::tagged("tag_name"))` etc.
///
/// # Example
///
/// ```ignore
/// // In card definition:
/// vec![
///     Effect::destroy(ChooseSpec::permanent()).tag("destroyed"),
///     Effect::create_tokens_player(
///         token,
///         1,
///         PlayerFilter::ControllerOf(ObjectRef::tagged("destroyed")),
///     ),
/// ]
/// ```
/// Tag the execution context (and runtime tag state) from an inner effect's
/// outcome — shared by live execution and batched simultaneous commits.
/// (Free function because `TaggedEffect` aliases a foreign core type.)
fn apply_outcome_tags(
    effect: &TaggedEffect,
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    outcome: &EffectOutcome,
    runtime: TaggedRuntimeState,
) {
        for damage in outcome.events_of_type::<DamageEvent>() {
            if damage.amount == 0 {
                continue;
            }
            match damage.target {
                DamageTarget::Player(player_id) => {
                    ctx.tag_player(effect.tag.clone(), player_id);
                    if effect.tag.as_str() != "__it__"
                        && effect.tag.as_str() != "__copied_stack_object__"
                    {
                        ctx.tag_player(TagKey::from("__it__"), player_id);
                    }
                }
                DamageTarget::Object(object_id) => {
                    if let Some(snapshot) = damage.target_snapshot.clone().or_else(|| {
                        game.object(object_id)
                            .map(|obj| ObjectSnapshot::from_object(obj, game))
                    }) {
                        ctx.tag_object(effect.tag.clone(), snapshot.clone());
                        if effect.tag.as_str() != "__it__"
                            && effect.tag.as_str() != "__copied_stack_object__"
                        {
                            ctx.tag_object(TagKey::from("__it__"), snapshot);
                        }
                    }
                }
            }
        }
        apply_tagged_runtime_state(game, ctx, effect.tag.clone(), outcome, runtime.clone());
        if effect.tag.as_str() != "__it__" && effect.tag.as_str() != "__copied_stack_object__" {
            apply_tagged_runtime_state(game, ctx, TagKey::from("__it__"), outcome, runtime);
        }
}

/// A tagged wrapper around another effect's simultaneous proposal: committing
/// runs the inner commit, then applies the same outcome-driven tagging as a
/// live execution.
#[derive(Debug)]
struct TaggedProposal {
    effect: TaggedEffect,
    inner: Box<dyn crate::effects::SimultaneousEffectProposal>,
    runtime: TaggedRuntimeState,
}

impl crate::effects::SimultaneousEffectProposal for TaggedProposal {
    fn commit(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let outcome = self.inner.commit(game, ctx)?;
        apply_outcome_tags(&self.effect, game, ctx, &outcome, self.runtime);
        Ok(outcome)
    }
}

impl EffectExecutor for TaggedEffect {
    fn as_cost_executable(&self) -> Option<&dyn CostExecutableEffect> {
        self.effect
            .0
            .as_cost_executable()
            .map(|_| self as &dyn CostExecutableEffect)
    }

    fn visit_child_effects(&self, visitor: &mut dyn FnMut(&Effect)) {
        visitor(&self.effect);
    }

    fn transparent_child_effect(&self) -> Option<&Effect> {
        Some(&self.effect)
    }

    fn is_resolution_prelude(&self) -> bool {
        self.effect
            .downcast_ref::<crate::effects::SequenceEffect>()
            .is_some_and(|sequence| sequence.effects.is_empty())
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let runtime = capture_tagged_runtime_state(game, &self.effect, ctx);

        // Execute the inner effect
        let outcome = crate::effects::execute_effect(game, &self.effect, ctx)?;
        apply_outcome_tags(self, game, ctx, &outcome, runtime);
        Ok(outcome)
    }

    fn supports_simultaneous_player_action(&self) -> bool {
        self.effect.0.supports_simultaneous_player_action()
    }

    fn is_read_only_simultaneous_player_action(&self) -> bool {
        self.effect.0.is_read_only_simultaneous_player_action()
    }

    fn prepare_simultaneous_player_action(
        &self,
        game: &GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Box<dyn crate::effects::SimultaneousEffectProposal>, ExecutionError> {
        let runtime = capture_tagged_runtime_state(game, &self.effect, ctx);
        let inner = self.effect.0.prepare_simultaneous_player_action(game, ctx)?;
        Ok(Box::new(TaggedProposal {
            effect: self.clone(),
            inner,
            runtime,
        }))
    }

    fn get_target_spec(&self) -> Option<&crate::target::ChooseSpec> {
        // Delegate to inner effect
        self.effect.0.get_target_spec()
    }

    fn target_chooser(&self) -> Option<&crate::target::PlayerFilter> {
        self.effect.0.target_chooser()
    }

    fn decision_related_object_specs(&self) -> Vec<crate::target::ChooseSpec> {
        self.effect.0.decision_related_object_specs()
    }

    fn target_description(&self) -> &'static str {
        // Delegate to inner effect
        self.effect.0.target_description()
    }

    fn get_target_count(&self) -> Option<crate::effect::ChoiceCount> {
        // Delegate to inner effect
        self.effect.0.get_target_count()
    }

    fn target_reuse_policy(&self) -> TargetReusePolicy {
        if self.effect.0.get_target_spec().is_some() {
            TargetReusePolicy::AlwaysDeclareNew
        } else {
            self.effect.0.target_reuse_policy()
        }
    }
}

impl CostExecutableEffect for TaggedEffect {
    fn can_execute_as_cost(
        &self,
        game: &GameState,
        source: crate::ids::ObjectId,
        controller: crate::ids::PlayerId,
    ) -> Result<(), CostValidationError> {
        self.effect.0.can_execute_as_cost(game, source, controller)
    }
}

/// Effect that executes an inner effect and tags ALL object targets for later reference.
///
/// Unlike `TaggedEffect` which only tags the first target, this variant tags all
/// object targets. This is useful for effects like "destroy all creatures" where
/// subsequent effects need to reference all the destroyed creatures.
///
/// # Example
///
/// ```ignore
/// // "Destroy all creatures. Their controllers each create a 3/3 for each
/// // creature they controlled that was destroyed this way."
/// vec![
///     Effect::destroy_all(ObjectFilter::creature()).tag_all("destroyed"),
///     Effect::for_each_controller_of_tagged("destroyed", vec![
///         Effect::create_tokens_player(
///             elephant_token(),
///             Value::TaggedCount,
///             PlayerFilter::IteratedPlayer,
///         ),
///     ]),
/// ]
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct TagAllEffect {
    /// The tag name to store the targets under.
    pub tag: TagKey,
    /// The effect to execute.
    pub effect: Box<Effect>,
}

impl TagAllEffect {
    /// Create a new tag-all effect.
    pub fn new(tag: impl Into<TagKey>, effect: Effect) -> Self {
        Self {
            tag: tag.into(),
            effect: Box::new(effect),
        }
    }
}

impl EffectExecutor for TagAllEffect {
    fn as_cost_executable(&self) -> Option<&dyn CostExecutableEffect> {
        self.effect
            .0
            .as_cost_executable()
            .map(|_| self as &dyn CostExecutableEffect)
    }

    fn visit_child_effects(&self, visitor: &mut dyn FnMut(&Effect)) {
        visitor(&self.effect);
    }

    fn transparent_child_effect(&self) -> Option<&Effect> {
        Some(&self.effect)
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let fallback_snapshots = capture_all_effect_target_snapshots(game, &self.effect, ctx);

        // Execute the inner effect, then tag the objects the effect actually
        // reports as affected. This keeps "destroyed this way" style tags from
        // including objects protected by replacement/prevention.
        let outcome = crate::effects::execute_effect(game, &self.effect, ctx)?;
        let has_result_objects = outcome.objects().is_some_and(|objects| !objects.is_empty())
            || outcome
                .affected_objects()
                .is_some_and(|objects| !objects.is_empty())
            || outcome
                .affected_object_memory()
                .is_some_and(|memory| !memory.is_empty())
            || outcome
                .chosen_objects()
                .is_some_and(|objects| !objects.is_empty())
            || outcome
                .chosen_object_memory()
                .is_some_and(|memory| !memory.is_empty());
        if has_result_objects {
            let runtime =
                TaggedRuntimeState::from_pre_snapshot(fallback_snapshots.first().cloned());
            apply_tagged_runtime_state(game, ctx, self.tag.clone(), &outcome, runtime);
        } else if outcome.something_happened() && !fallback_snapshots.is_empty() {
            ctx.tag_objects(self.tag.clone(), fallback_snapshots);
        }
        Ok(outcome)
    }

    fn get_target_spec(&self) -> Option<&crate::target::ChooseSpec> {
        // Delegate to inner effect
        self.effect.0.get_target_spec()
    }

    fn target_chooser(&self) -> Option<&crate::target::PlayerFilter> {
        self.effect.0.target_chooser()
    }

    fn decision_related_object_specs(&self) -> Vec<crate::target::ChooseSpec> {
        self.effect.0.decision_related_object_specs()
    }

    fn target_description(&self) -> &'static str {
        // Delegate to inner effect
        self.effect.0.target_description()
    }

    fn get_target_count(&self) -> Option<crate::effect::ChoiceCount> {
        // Delegate to inner effect
        self.effect.0.get_target_count()
    }

    fn target_reuse_policy(&self) -> TargetReusePolicy {
        if self.effect.0.get_target_spec().is_some() {
            TargetReusePolicy::AlwaysDeclareNew
        } else {
            self.effect.0.target_reuse_policy()
        }
    }
}

impl CostExecutableEffect for TagAllEffect {
    fn can_execute_as_cost(
        &self,
        game: &GameState,
        source: crate::ids::ObjectId,
        controller: crate::ids::PlayerId,
    ) -> Result<(), CostValidationError> {
        self.effect.0.can_execute_as_cost(game, source, controller)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::effect::Effect;
    use crate::effects::ResolvedTarget;
    use crate::filter::ObjectRef;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::Object;
    use crate::target::{ChooseSpec, ObjectFilter, PlayerFilter};
    use crate::types::CardType;
    use crate::zone::Zone;
    use std::collections::HashMap;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn create_creature(game: &mut GameState, name: &str, controller: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Green],
            ]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let obj = Object::from_card(id, &card, controller, Zone::Battlefield);
        game.add_object(obj);
        id
    }

    fn create_graveyard_creature(game: &mut GameState, name: &str, owner: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Green],
            ]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let obj = Object::from_card(id, &card, owner, Zone::Graveyard);
        game.add_object(obj);
        id
    }

    fn create_library_creature(game: &mut GameState, name: &str, owner: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Green],
            ]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let obj = Object::from_card(id, &card, owner, Zone::Library);
        game.add_object(obj);
        game.player_mut(owner).expect("player").library.push(id);
        id
    }

    #[test]
    fn test_tagged_effect_captures_snapshot() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature(&mut game, "Grizzly Bears", alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        // Create a tagged effect (we use gain_life as a simple effect that won't change the target)
        let effect = TaggedEffect::new("target", Effect::gain_life(1));
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Effect should have executed
        assert_eq!(result.value, crate::effect::OutcomeValue::Count(1));

        // Tagged object should be stored
        let tagged = ctx.get_tagged("target");
        assert!(tagged.is_some());
        let snapshot = tagged.unwrap();
        assert_eq!(snapshot.name, "Grizzly Bears");
        assert_eq!(snapshot.controller, alice);
    }

    #[test]
    fn test_tagged_effect_preserves_lki_after_destroy() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature(&mut game, "Grizzly Bears", alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        // Create a tagged destroy effect
        let effect = TaggedEffect::new("destroyed", Effect::destroy(ChooseSpec::creature()));
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Effect should have resolved
        assert_eq!(result.status, crate::effect::OutcomeStatus::Succeeded);

        // Creature should be destroyed (in graveyard)
        assert!(!game.battlefield.contains(&creature_id));

        // Tagged object should still have the snapshot
        let tagged = ctx.get_tagged("destroyed");
        assert!(tagged.is_some());
        let snapshot = tagged.unwrap();
        assert_eq!(snapshot.name, "Grizzly Bears");
        assert_eq!(snapshot.controller, alice);
    }

    #[test]
    fn test_controller_of_tagged_object() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Create a creature controlled by Bob
        let creature_id = create_creature(&mut game, "Grizzly Bears", bob);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        // Tag the creature
        let effect = TaggedEffect::new("target", Effect::gain_life(1));
        effect.execute(&mut game, &mut ctx).unwrap();

        // Now test that ControllerOf(ObjectRef::tagged("target")) resolves to Bob
        let _filter = PlayerFilter::ControllerOf(ObjectRef::tagged("target"));
        let _filter_ctx = ctx.filter_context(&game);

        // The controller should be Bob
        let tagged = ctx.get_tagged("target").unwrap();
        assert_eq!(tagged.controller, bob);
    }

    #[test]
    fn test_clone_box() {
        let effect = TaggedEffect::new("test", Effect::gain_life(1));
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("TaggedEffect"));
    }

    #[test]
    fn test_tagged_effect_tracks_return_all_to_battlefield_objects() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        create_graveyard_creature(&mut game, "Grizzly Bears", alice);
        create_graveyard_creature(&mut game, "Runeclaw Bear", alice);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let filter = ObjectFilter::creature()
            .in_zone(Zone::Graveyard)
            .owned_by(PlayerFilter::You);
        let effect = TaggedEffect::new(
            "returned",
            Effect::new(crate::effects::ReturnAllToBattlefieldEffect::new(
                filter, false,
            )),
        );
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(2));
        let tagged = ctx
            .get_tagged_all("returned")
            .expect("returned objects should be tagged");
        assert_eq!(tagged.len(), 2);
        assert!(
            tagged
                .iter()
                .all(|snapshot| snapshot.zone == Zone::Battlefield)
        );
        assert!(tagged.iter().all(|snapshot| snapshot.controller == alice));
    }

    #[test]
    fn test_get_target_spec_delegates() {
        let effect = TaggedEffect::new("test", Effect::destroy(ChooseSpec::creature()));
        let spec = effect.get_target_spec();
        assert!(spec.is_some());
    }

    #[test]
    fn test_tagged_target_only_with_count_captures_all_targets() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let alice_target = create_creature(&mut game, "Alice Target", alice);
        let bob_target = create_creature(&mut game, "Bob Target", bob);
        let spec = ChooseSpec::target(ChooseSpec::creature())
            .with_count(crate::effect::ChoiceCount::exactly(2));
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![
                ResolvedTarget::Object(alice_target),
                ResolvedTarget::Object(bob_target),
            ])
            .with_target_assignments(vec![crate::game_state::TargetAssignment {
                spec: spec.clone(),
                range: 0..2,
            }]);

        let effect = TaggedEffect::new(
            "targeted",
            Effect::new(crate::effects::TargetOnlyEffect::new(spec)),
        );
        effect.execute(&mut game, &mut ctx).expect("execute");

        let tagged = ctx
            .get_tagged_all("targeted")
            .expect("tagged targets should exist");
        assert_eq!(tagged.len(), 2);
        assert_eq!(tagged[0].object_id, alice_target);
        assert_eq!(tagged[1].object_id, bob_target);
    }

    // ========================================
    // TagAllEffect Tests
    // ========================================

    #[test]
    fn test_tag_all_effect_captures_all_targets() {
        use super::TagAllEffect;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature1 = create_creature(&mut game, "Bear 1", alice);
        let creature2 = create_creature(&mut game, "Bear 2", alice);
        let creature3 = create_creature(&mut game, "Bear 3", alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice).with_targets(vec![
            ResolvedTarget::Object(creature1),
            ResolvedTarget::Object(creature2),
            ResolvedTarget::Object(creature3),
        ]);

        // Create a tag-all effect
        let effect = TagAllEffect::new("targets", Effect::gain_life(1));
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Effect should have executed
        assert_eq!(result.value, crate::effect::OutcomeValue::Count(1));

        // All three objects should be tagged
        let tagged_all = ctx.get_tagged_all("targets");
        assert!(tagged_all.is_some());
        let snapshots = tagged_all.unwrap();
        assert_eq!(snapshots.len(), 3);
        assert_eq!(snapshots[0].name, "Bear 1");
        assert_eq!(snapshots[1].name, "Bear 2");
        assert_eq!(snapshots[2].name, "Bear 3");
    }

    #[test]
    fn test_tag_all_effect_with_mixed_targets() {
        use super::TagAllEffect;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let creature1 = create_creature(&mut game, "Alice Bear", alice);
        let creature2 = create_creature(&mut game, "Bob Bear", bob);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice).with_targets(vec![
            ResolvedTarget::Object(creature1),
            ResolvedTarget::Player(bob), // Non-object target should be ignored
            ResolvedTarget::Object(creature2),
        ]);

        let effect = TagAllEffect::new("creatures", Effect::gain_life(1));
        effect.execute(&mut game, &mut ctx).unwrap();

        // Only the two object targets should be tagged
        let tagged_all = ctx.get_tagged_all("creatures");
        assert!(tagged_all.is_some());
        let snapshots = tagged_all.unwrap();
        assert_eq!(snapshots.len(), 2);
        assert_eq!(snapshots[0].name, "Alice Bear");
        assert_eq!(snapshots[1].name, "Bob Bear");
    }

    #[test]
    fn test_tag_all_effect_clone_box() {
        use super::TagAllEffect;

        let effect = TagAllEffect::new("test", Effect::gain_life(1));
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("TagAllEffect"));
    }

    #[test]
    fn test_tag_all_effect_delegates_target_chooser_and_reuse_policy() {
        use super::TagAllEffect;

        let chooser = PlayerFilter::ControllerOf(ObjectRef::Tagged(TagKey::from("first")));
        let effect = TagAllEffect::new(
            "second",
            Effect::new(
                crate::effects::TargetOnlyEffect::explicit(ChooseSpec::target(
                    ChooseSpec::creature(),
                ))
                .with_chooser(chooser.clone()),
            ),
        );

        assert_eq!(effect.target_chooser(), Some(&chooser));
        assert!(matches!(
            effect.target_reuse_policy(),
            TargetReusePolicy::AlwaysDeclareNew
        ));
    }

    #[test]
    fn test_tag_all_effect_uses_effect_target_spec_when_targets_are_implicit() {
        use super::TagAllEffect;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let creature_id = create_library_creature(&mut game, "Tagged Library Creature", alice);
        let chosen_snapshot = game
            .object(creature_id)
            .map(|obj| crate::snapshot::ObjectSnapshot::from_object(obj, &game))
            .expect("library creature snapshot");

        let mut ctx = ExecutionContext::new_default(source, alice).with_tagged_objects(
            HashMap::from([(TagKey::from("chosen"), vec![chosen_snapshot.clone()])]),
        );

        let effect = TagAllEffect::new(
            "kept",
            Effect::move_to_zone(
                ChooseSpec::Tagged(TagKey::from("chosen")),
                Zone::Hand,
                false,
            ),
        );
        effect.execute(&mut game, &mut ctx).unwrap();

        let tagged_all = ctx.get_tagged_all("kept").expect("kept tag should exist");
        assert_eq!(tagged_all.len(), 1);
        assert_eq!(tagged_all[0].stable_id, chosen_snapshot.stable_id);
    }

    #[test]
    fn test_tag_all_effect_tags_actual_runtime_result_memory() {
        use super::TagAllEffect;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let alice_target = create_creature(&mut game, "Alice Target", alice);
        let bob_target = create_creature(&mut game, "Bob Target", bob);
        let alice_stable_id = game.object(alice_target).expect("alice target").stable_id;
        let bob_stable_id = game.object(bob_target).expect("bob target").stable_id;
        let spec = ChooseSpec::target(ChooseSpec::creature())
            .with_count(crate::effect::ChoiceCount::exactly(2));

        let mut ctx = ExecutionContext::new_default(game.new_object_id(), alice)
            .with_targets(vec![
                ResolvedTarget::Object(alice_target),
                ResolvedTarget::Object(bob_target),
            ])
            .with_target_assignments(vec![crate::game_state::TargetAssignment {
                spec: spec.clone(),
                range: 0..2,
            }]);

        let effect = TagAllEffect::new(
            "destroyed",
            Effect::new(crate::effects::DestroyEffect::with_spec(spec)),
        );
        let outcome = effect.execute(&mut game, &mut ctx).expect("execute");

        assert_eq!(outcome.as_count(), Some(2));
        let tagged = ctx
            .get_tagged_all("destroyed")
            .expect("destroyed objects should be tagged from result memory");
        assert_eq!(tagged.len(), 2);
        assert_eq!(tagged[0].name, "Alice Target");
        assert_eq!(tagged[0].stable_id, alice_stable_id);
        assert_eq!(tagged[0].zone, Zone::Graveyard);
        assert_eq!(tagged[1].name, "Bob Target");
        assert_eq!(tagged[1].stable_id, bob_stable_id);
        assert_eq!(tagged[1].zone, Zone::Graveyard);
    }
}
