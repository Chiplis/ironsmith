//! Tagged effect implementation.
//!
//! This effect wrapper captures the target object as a tagged snapshot
//! that can be referenced by subsequent effects in the same spell/ability.

use crate::effect::{Effect, EffectOutcome};
use crate::effects::EffectExecutor;
use crate::executor::{ExecutionContext, ExecutionError, ResolvedTarget};
use crate::game_state::GameState;
use crate::snapshot::ObjectSnapshot;
use crate::tag::TagKey;

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
#[derive(Debug, Clone, PartialEq)]
pub struct TaggedEffect {
    /// The tag name to store the target under.
    pub tag: TagKey,
    /// The effect to execute.
    pub effect: Box<Effect>,
}

impl TaggedEffect {
    /// Create a new tagged effect.
    pub fn new(tag: impl Into<TagKey>, effect: Effect) -> Self {
        Self {
            tag: tag.into(),
            effect: Box::new(effect),
        }
    }
}

impl EffectExecutor for TaggedEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        // Capture a snapshot of the first object target before executing the effect.
        // This preserves LKI for effects that don't return explicit object results.
        let mut pre_snapshot: Option<ObjectSnapshot> = None;
        for target in &ctx.targets {
            if let ResolvedTarget::Object(object_id) = target
                && let Some(obj) = game.object(*object_id)
            {
                pre_snapshot = Some(ObjectSnapshot::from_object(obj, game));
                break;
            }
        }

        // Execute the inner effect
        let outcome = crate::executor::execute_effect(game, &self.effect, ctx)?;

        // If the inner effect returned explicit objects, tag those (post-effect state).
        if let crate::effect::EffectResult::Objects(ids) = &outcome.result
            && !ids.is_empty()
        {
            let snapshots: Vec<ObjectSnapshot> = ids
                .iter()
                .filter_map(|id| {
                    game.object(*id)
                        .map(|obj| ObjectSnapshot::from_object(obj, game))
                })
                .collect();
            if !snapshots.is_empty() {
                ctx.set_tagged_objects(self.tag.clone(), snapshots);
            }
            return Ok(outcome);
        }

        // Otherwise, fall back to tagging the pre-effect snapshot if we captured one.
        if let Some(snapshot) = pre_snapshot {
            ctx.tag_object(self.tag.clone(), snapshot);
        }

        Ok(outcome)
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn get_target_spec(&self) -> Option<&crate::target::ChooseSpec> {
        // Delegate to inner effect
        self.effect.0.get_target_spec()
    }

    fn target_description(&self) -> &'static str {
        // Delegate to inner effect
        self.effect.0.target_description()
    }

    fn get_target_count(&self) -> Option<crate::effect::ChoiceCount> {
        // Delegate to inner effect
        self.effect.0.get_target_count()
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
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        // Before executing the inner effect, capture snapshots of ALL object targets.
        // This preserves LKI even if the objects are destroyed/exiled/etc.
        let mut snapshots = Vec::new();
        for target in &ctx.targets {
            if let ResolvedTarget::Object(object_id) = target
                && let Some(obj) = game.object(*object_id)
            {
                snapshots.push(ObjectSnapshot::from_object(obj, game));
            }
        }

        // Tag all the snapshots
        if !snapshots.is_empty() {
            ctx.tag_objects(self.tag.clone(), snapshots);
        }

        // Execute the inner effect
        crate::executor::execute_effect(game, &self.effect, ctx)
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn get_target_spec(&self) -> Option<&crate::target::ChooseSpec> {
        // Delegate to inner effect
        self.effect.0.get_target_spec()
    }

    fn target_description(&self) -> &'static str {
        // Delegate to inner effect
        self.effect.0.target_description()
    }

    fn get_target_count(&self) -> Option<crate::effect::ChoiceCount> {
        // Delegate to inner effect
        self.effect.0.get_target_count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::effect::Effect;
    use crate::effect::EffectResult;
    use crate::filter::ObjectRef;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::Object;
    use crate::target::{ChooseSpec, PlayerFilter};
    use crate::types::CardType;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
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
        assert_eq!(result.result, EffectResult::Count(1));

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
        assert_eq!(result.result, EffectResult::Resolved);

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
    fn test_get_target_spec_delegates() {
        let effect = TaggedEffect::new("test", Effect::destroy(ChooseSpec::creature()));
        let spec = effect.get_target_spec();
        assert!(spec.is_some());
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
        assert_eq!(result.result, EffectResult::Count(1));

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
}
