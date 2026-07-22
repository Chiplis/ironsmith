//! ForEachTagged effect implementations.
//!
//! These effects iterate over objects that were tagged by prior effects in the same
//! spell/ability resolution, enabling patterns like:
//! - "Destroy all creatures. Their controllers each create a token for each creature
//!    they controlled that was destroyed this way."

use crate::effect::{Effect, EffectOutcome};
use crate::effects::{EffectExecutor, SimultaneousEffectProposal};
use crate::effects::{ExecutionContext, ExecutionError, execute_effect};
use crate::game_state::GameState;
use crate::ids::PlayerId;
use crate::snapshot::ObjectSnapshot;
use crate::tag::TagKey;

/// Effect that applies effects once for each tagged object.
///
/// Sets `ctx.iteration.iterated_object` for each iteration, and also sets
/// `ctx.iteration.iterated_player` to that object's controller.
///
/// # Fields
///
/// * `tag` - The tag name to iterate over
/// * `effects` - Effects to execute for each tagged object
///
/// # Example
///
/// ```ignore
/// // For each creature destroyed, its controller loses 1 life
/// let effect = ForEachTaggedEffect::new("destroyed", vec![
///     Effect::lose_life_player(1, PlayerFilter::ControllerOf(ObjectRef::Iterated)),
/// ]);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ForEachTaggedEffect {
    /// The tag name to iterate over.
    pub tag: TagKey,
    /// Effects to execute for each tagged object.
    pub effects: Vec<Effect>,
}

impl ForEachTaggedEffect {
    /// Create a new ForEachTagged effect.
    pub fn new(tag: impl Into<TagKey>, effects: Vec<Effect>) -> Self {
        Self {
            tag: tag.into(),
            effects,
        }
    }
}

/// Prepared iterations over one player's captured tagged set.
///
/// The shared execution context is reused while every player's simultaneous
/// action is prepared. Retaining these snapshots keeps each player's tagged
/// cards paired with the proposals prepared for those cards.
#[derive(Debug)]
struct ForEachTaggedProposal {
    tag: TagKey,
    snapshots: Vec<ObjectSnapshot>,
    iterations: Vec<Vec<Box<dyn SimultaneousEffectProposal>>>,
}

impl SimultaneousEffectProposal for ForEachTaggedProposal {
    fn commit(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let Self {
            tag,
            snapshots,
            iterations,
        } = *self;
        let it_tag = TagKey::from("__it__");
        let previous_tag = TagKey::from(ironsmith_core::PREVIOUS_ITERATED_OBJECTS_TAG);
        let original_tagged = ctx.tagged_objects.remove(&tag);
        let original_it = ctx.tagged_objects.remove(&it_tag);
        let original_previous = ctx.tagged_objects.remove(&previous_tag);
        ctx.set_tagged_objects(tag.clone(), snapshots.clone());

        let result = (|| {
            let mut outcomes = Vec::new();
            let mut player_counts: Vec<(PlayerId, i32)> = Vec::new();
            for (index, (snapshot, proposals)) in
                snapshots.iter().zip(iterations.into_iter()).enumerate()
            {
                ctx.set_tagged_objects(previous_tag.clone(), snapshots[..index].to_vec());
                ctx.set_tagged_objects(it_tag.clone(), vec![snapshot.clone()]);
                let start = outcomes.len();
                ctx.with_temp_iterated_object(Some(snapshot.object_id), |ctx| {
                    ctx.with_temp_iterated_player(Some(snapshot.controller), |ctx| {
                        for proposal in proposals {
                            outcomes.push(proposal.commit(game, ctx)?);
                        }
                        Ok::<(), ExecutionError>(())
                    })
                })?;
                let count =
                    EffectOutcome::aggregate_summing_counts(outcomes[start..].iter().cloned())
                        .as_count()
                        .unwrap_or(0);
                if let Some((_, total)) = player_counts
                    .iter_mut()
                    .find(|(player, _)| *player == snapshot.controller)
                {
                    *total += count;
                } else {
                    player_counts.push((snapshot.controller, count));
                }
            }
            Ok(EffectOutcome::aggregate_summing_counts(outcomes).with_player_counts(player_counts))
        })();

        match original_tagged {
            Some(value) => {
                ctx.tagged_objects.insert(tag, value);
            }
            None => {
                ctx.tagged_objects.remove(&tag);
            }
        }
        match original_it {
            Some(value) => {
                ctx.tagged_objects.insert(it_tag, value);
            }
            None => {
                ctx.tagged_objects.remove(&it_tag);
            }
        }
        match original_previous {
            Some(value) => {
                ctx.tagged_objects.insert(previous_tag, value);
            }
            None => {
                ctx.tagged_objects.remove(&previous_tag);
            }
        }
        result
    }
}

impl EffectExecutor for ForEachTaggedEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn visit_child_effects(&self, visitor: &mut dyn FnMut(&Effect)) {
        for effect in &self.effects {
            visitor(effect);
        }
    }

    fn supports_simultaneous_player_action(&self) -> bool {
        self.tag.as_str() != "__it__"
            && self.tag.as_str() != ironsmith_core::PREVIOUS_ITERATED_OBJECTS_TAG
            && self
                .effects
                .iter()
                .all(|effect| effect.0.supports_simultaneous_player_action())
    }

    fn prepare_simultaneous_player_action(
        &self,
        game: &GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Box<dyn SimultaneousEffectProposal>, ExecutionError> {
        let snapshots = ctx.get_tagged_all(&self.tag).cloned().unwrap_or_default();
        let it_tag = TagKey::from("__it__");
        let previous_tag = TagKey::from(ironsmith_core::PREVIOUS_ITERATED_OBJECTS_TAG);
        let original_it = ctx.tagged_objects.remove(&it_tag);
        let original_previous = ctx.tagged_objects.remove(&previous_tag);

        let result = (|| {
            let mut iterations = Vec::with_capacity(snapshots.len());
            for (index, snapshot) in snapshots.iter().enumerate() {
                ctx.set_tagged_objects(previous_tag.clone(), snapshots[..index].to_vec());
                ctx.set_tagged_objects(it_tag.clone(), vec![snapshot.clone()]);
                let proposals = ctx.with_temp_iterated_object(Some(snapshot.object_id), |ctx| {
                    ctx.with_temp_iterated_player(Some(snapshot.controller), |ctx| {
                        self.effects
                            .iter()
                            .map(|effect| effect.0.prepare_simultaneous_player_action(game, ctx))
                            .collect::<Result<Vec<_>, _>>()
                    })
                })?;
                iterations.push(proposals);
            }
            Ok::<_, ExecutionError>(Box::new(ForEachTaggedProposal {
                tag: self.tag.clone(),
                snapshots: snapshots.clone(),
                iterations,
            }) as Box<dyn SimultaneousEffectProposal>)
        })();

        match original_it {
            Some(value) => {
                ctx.tagged_objects.insert(it_tag, value);
            }
            None => {
                ctx.tagged_objects.remove(&it_tag);
            }
        }
        match original_previous {
            Some(value) => {
                ctx.tagged_objects.insert(previous_tag, value);
            }
            None => {
                ctx.tagged_objects.remove(&previous_tag);
            }
        }
        result
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        // Get all tagged objects
        let snapshots = match ctx.get_tagged_all(&self.tag) {
            Some(snaps) => snaps.clone(), // Clone to avoid borrow issues
            None => return Ok(EffectOutcome::count(0)),
        };

        if snapshots.is_empty() {
            return Ok(EffectOutcome::count(0));
        }

        let mut outcomes = Vec::new();
        let mut player_counts: Vec<(PlayerId, i32)> = Vec::new();

        let it_tag = TagKey::from("__it__");
        let previous_tag = TagKey::from(ironsmith_core::PREVIOUS_ITERATED_OBJECTS_TAG);
        let original_previous = ctx.tagged_objects.remove(&previous_tag);
        for (index, snapshot) in snapshots.iter().enumerate() {
            // Ordered iterations expose exactly the objects processed before
            // the current one. Ordinary loops can ignore this tag; values such
            // as "for each creature chosen before it" count it directly.
            ctx.set_tagged_objects(previous_tag.clone(), snapshots[..index].to_vec());
            // Expose the current iterated object as "__it__" for tagged constraints like
            // "shares a card type with it" inside the loop body.
            let original_it = ctx.tagged_objects.remove(&it_tag);
            ctx.tag_object(it_tag.clone(), snapshot.clone());

            let start = outcomes.len();
            ctx.with_temp_iterated_object(Some(snapshot.object_id), |ctx| {
                // Also expose this object's controller as the iterated player.
                // This lets inner effects naturally say "its controller" via IteratedPlayer.
                ctx.with_temp_iterated_player(Some(snapshot.controller), |ctx| {
                    // Execute all inner effects for this object
                    for effect in &self.effects {
                        outcomes.push(execute_effect(game, effect, ctx)?);
                    }
                    Ok::<(), ExecutionError>(())
                })
            })?;
            let count = EffectOutcome::aggregate_summing_counts(outcomes[start..].iter().cloned())
                .as_count()
                .unwrap_or(0);
            if let Some((_, total)) = player_counts
                .iter_mut()
                .find(|(player, _)| *player == snapshot.controller)
            {
                *total += count;
            } else {
                player_counts.push((snapshot.controller, count));
            }

            match original_it {
                Some(value) => {
                    ctx.tagged_objects.insert(it_tag.clone(), value);
                }
                None => {
                    ctx.tagged_objects.remove(&it_tag);
                }
            }
        }

        match original_previous {
            Some(value) => {
                ctx.tagged_objects.insert(previous_tag, value);
            }
            None => {
                ctx.tagged_objects.remove(&previous_tag);
            }
        }

        Ok(EffectOutcome::aggregate_summing_counts(outcomes).with_player_counts(player_counts))
    }
}

/// Effect that groups tagged objects by controller and executes effects for each controller.
///
/// This enables patterns like "Destroy all creatures. Their controllers each create a token
/// for each creature they controlled that was destroyed this way."
///
/// Sets `ctx.iteration.iterated_player` to each controller, and provides a count value that can be
/// used to determine how many objects that controller controlled.
///
/// # Fields
///
/// * `tag` - The tag name to iterate over
/// * `effects` - Effects to execute for each controller (use `Value::TaggedCount` to get count)
///
/// # Example
///
/// ```ignore
/// // Each player creates a 3/3 for each creature they controlled that was destroyed
/// vec![
///     Effect::destroy_all(ObjectFilter::creature()).tag_all("destroyed"),
///     Effect::for_each_controller_of_tagged("destroyed", vec![
///         Effect::create_tokens_player(
///             elephant_token(),
///             Value::TaggedCount("destroyed"),  // Count for this controller
///             PlayerFilter::IteratedPlayer,
///         ),
///     ]),
/// ]
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ForEachControllerOfTaggedEffect {
    /// The tag name to iterate over.
    pub tag: TagKey,
    /// Effects to execute for each controller.
    pub effects: Vec<Effect>,
}

impl ForEachControllerOfTaggedEffect {
    /// Create a new ForEachControllerOfTagged effect.
    pub fn new(tag: impl Into<TagKey>, effects: Vec<Effect>) -> Self {
        Self {
            tag: tag.into(),
            effects,
        }
    }
}

impl EffectExecutor for ForEachControllerOfTaggedEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn visit_child_effects(&self, visitor: &mut dyn FnMut(&Effect)) {
        for effect in &self.effects {
            visitor(effect);
        }
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        // Get counts grouped by controller
        let counts = ctx.count_tagged_by_controller(&self.tag);

        if counts.is_empty() {
            return Ok(EffectOutcome::count(0));
        }

        let mut outcomes = Vec::new();

        // Sort by player index for deterministic ordering
        let mut controller_counts: Vec<(PlayerId, usize)> = counts.into_iter().collect();
        controller_counts.sort_by_key(|(p, _)| p.0);

        for (controller, count) in controller_counts {
            ctx.with_temp_iterated_player(Some(controller), |ctx| {
                // Store the count as a temporary outcome so Value::TaggedCount can retrieve it.
                // We use a special EffectId for this purpose.
                ctx.store_outcome(
                    crate::effect::EffectId::TAGGED_COUNT,
                    EffectOutcome::count(count as i32),
                );

                // Execute all inner effects for this controller
                for effect in &self.effects {
                    outcomes.push(execute_effect(game, effect, ctx)?);
                }
                Ok::<(), ExecutionError>(())
            })?;
        }
        ctx.effect_outcomes
            .remove(&crate::effect::EffectId::TAGGED_COUNT);

        Ok(EffectOutcome::aggregate_summing_counts(outcomes))
    }
}

/// Effect that applies effects once for each tagged player.
///
/// Sets `ctx.iteration.iterated_player` for each iteration, allowing inner effects
/// to reference the current player via `PlayerFilter::IteratedPlayer`.
///
/// # Fields
///
/// * `tag` - The tag name to iterate over (e.g., "voted_with_you")
/// * `effects` - Effects to execute for each tagged player
///
/// # Example
///
/// ```ignore
/// // Each opponent who voted with you may scry 2
/// let effect = ForEachTaggedPlayerEffect::new("voted_with_you", vec![
///     Effect::may_player(PlayerFilter::IteratedPlayer, vec![Effect::scry(2)]),
/// ]);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ForEachTaggedPlayerEffect {
    /// The tag name to iterate over.
    pub tag: TagKey,
    /// Effects to execute for each tagged player.
    pub effects: Vec<Effect>,
}

impl ForEachTaggedPlayerEffect {
    /// Create a new ForEachTaggedPlayer effect.
    pub fn new(tag: impl Into<TagKey>, effects: Vec<Effect>) -> Self {
        Self {
            tag: tag.into(),
            effects,
        }
    }
}

impl EffectExecutor for ForEachTaggedPlayerEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn visit_child_effects(&self, visitor: &mut dyn FnMut(&Effect)) {
        for effect in &self.effects {
            visitor(effect);
        }
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        // Get all tagged players
        let players = match ctx.get_tagged_players(&self.tag) {
            Some(players) => players.clone(), // Clone to avoid borrow issues
            None => return Ok(EffectOutcome::count(0)),
        };

        if players.is_empty() {
            return Ok(EffectOutcome::count(0));
        }

        let mut outcomes = Vec::new();

        for player_id in &players {
            ctx.with_temp_iterated_player(Some(*player_id), |ctx| {
                // Execute all inner effects for this player
                for effect in &self.effects {
                    outcomes.push(execute_effect(game, effect, ctx)?);
                }
                Ok::<(), ExecutionError>(())
            })?;
        }

        Ok(EffectOutcome::aggregate_summing_counts(outcomes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::Object;
    use crate::snapshot::ObjectSnapshot;
    use crate::types::CardType;
    use crate::zone::Zone;

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

    #[test]
    fn test_for_each_tagged_iterates_all() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        // Create some creatures
        let creature1 = create_creature(&mut game, "Bear 1", alice);
        let creature2 = create_creature(&mut game, "Bear 2", alice);

        let mut ctx = ExecutionContext::new_default(source, alice);

        // Tag both creatures
        let snap1 = ObjectSnapshot::from_object(game.object(creature1).unwrap(), &game);
        let snap2 = ObjectSnapshot::from_object(game.object(creature2).unwrap(), &game);
        ctx.tag_objects("destroyed", vec![snap1, snap2]);

        // ForEachTagged: gain 1 life for each tagged object
        let effect = ForEachTaggedEffect::new("destroyed", vec![Effect::gain_life(1)]);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Should have executed twice (2 creatures)
        assert_eq!(result.value, crate::effect::OutcomeValue::Count(2));
        // Alice gained 2 life total
        assert_eq!(game.player(alice).unwrap().life, 22);
    }

    #[test]
    fn test_for_each_tagged_empty() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        // No tagged objects
        let effect = ForEachTaggedEffect::new("nonexistent", vec![Effect::gain_life(5)]);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(0));
        assert_eq!(game.player(alice).unwrap().life, 20);
    }

    #[test]
    fn test_for_each_tagged_preserves_iterated_object() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let creature = create_creature(&mut game, "Bear", alice);

        let mut ctx = ExecutionContext::new_default(source, alice);

        // Set an initial iterated_object
        let original = ObjectId::from_raw(999);
        ctx.iteration.iterated_object = Some(original);

        // Tag a creature
        let snap = ObjectSnapshot::from_object(game.object(creature).unwrap(), &game);
        ctx.tag_object("test", snap);

        let effect = ForEachTaggedEffect::new("test", vec![Effect::gain_life(1)]);
        effect.execute(&mut game, &mut ctx).unwrap();

        // Should restore original iterated_object
        assert_eq!(ctx.iteration.iterated_object, Some(original));
    }

    #[test]
    fn test_for_each_controller_of_tagged_groups_by_controller() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();

        // Create creatures for both players
        let alice_creature1 = create_creature(&mut game, "Alice Bear 1", alice);
        let alice_creature2 = create_creature(&mut game, "Alice Bear 2", alice);
        let bob_creature = create_creature(&mut game, "Bob Bear", bob);

        let mut ctx = ExecutionContext::new_default(source, alice);

        // Tag all three creatures
        let snap1 = ObjectSnapshot::from_object(game.object(alice_creature1).unwrap(), &game);
        let snap2 = ObjectSnapshot::from_object(game.object(alice_creature2).unwrap(), &game);
        let snap3 = ObjectSnapshot::from_object(game.object(bob_creature).unwrap(), &game);
        ctx.tag_objects("destroyed", vec![snap1, snap2, snap3]);

        // Check the grouped counts
        let counts = ctx.count_tagged_by_controller("destroyed");
        assert_eq!(counts.get(&alice), Some(&2));
        assert_eq!(counts.get(&bob), Some(&1));
    }

    #[test]
    fn test_for_each_controller_of_tagged_executes_for_each_controller() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();

        // Create creatures for both players
        let alice_creature1 = create_creature(&mut game, "Alice Bear 1", alice);
        let bob_creature = create_creature(&mut game, "Bob Bear", bob);

        let mut ctx = ExecutionContext::new_default(source, alice);

        // Tag both creatures
        let snap1 = ObjectSnapshot::from_object(game.object(alice_creature1).unwrap(), &game);
        let snap2 = ObjectSnapshot::from_object(game.object(bob_creature).unwrap(), &game);
        ctx.tag_objects("destroyed", vec![snap1, snap2]);

        // ForEachControllerOfTagged: each controller gains 3 life
        // Note: this uses IteratedPlayer to target the current controller
        let effect = ForEachControllerOfTaggedEffect::new(
            "destroyed",
            vec![Effect::gain_life(3)], // This gains life for ctx.controller, not iterated player
        );
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Should have executed twice (2 controllers)
        assert_eq!(result.value, crate::effect::OutcomeValue::Count(6));
    }

    #[test]
    fn test_for_each_controller_of_tagged_empty() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect =
            ForEachControllerOfTaggedEffect::new("nonexistent", vec![Effect::gain_life(5)]);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(0));
    }

    #[test]
    fn test_for_each_controller_of_tagged_preserves_iterated_player() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let creature = create_creature(&mut game, "Bear", alice);

        let mut ctx = ExecutionContext::new_default(source, alice);

        // Set an initial iterated_player
        let original = PlayerId::from_index(99);
        ctx.iteration.iterated_player = Some(original);

        // Tag a creature
        let snap = ObjectSnapshot::from_object(game.object(creature).unwrap(), &game);
        ctx.tag_object("test", snap);

        let effect = ForEachControllerOfTaggedEffect::new("test", vec![Effect::gain_life(1)]);
        effect.execute(&mut game, &mut ctx).unwrap();

        // Should restore original iterated_player
        assert_eq!(ctx.iteration.iterated_player, Some(original));
    }

    #[test]
    fn test_clone_box() {
        let effect1 = ForEachTaggedEffect::new("test", vec![Effect::gain_life(1)]);
        let cloned1 = effect1.clone_box();
        assert!(format!("{:?}", cloned1).contains("ForEachTaggedEffect"));

        let effect2 = ForEachControllerOfTaggedEffect::new("test", vec![Effect::gain_life(1)]);
        let cloned2 = effect2.clone_box();
        assert!(format!("{:?}", cloned2).contains("ForEachControllerOfTaggedEffect"));

        let effect3 = ForEachTaggedPlayerEffect::new("test", vec![Effect::gain_life(1)]);
        let cloned3 = effect3.clone_box();
        assert!(format!("{:?}", cloned3).contains("ForEachTaggedPlayerEffect"));
    }

    #[test]
    fn test_for_each_tagged_player_iterates_all() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice);

        // Tag both players
        ctx.tag_players("voters", vec![alice, bob]);

        // ForEachTaggedPlayer: gain 1 life for each tagged player
        // (Alice is controller, so she gains the life)
        let effect = ForEachTaggedPlayerEffect::new("voters", vec![Effect::gain_life(1)]);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Should have executed twice (2 players)
        assert_eq!(result.value, crate::effect::OutcomeValue::Count(2));
        // Alice gained 2 life total
        assert_eq!(game.player(alice).unwrap().life, 22);
    }

    #[test]
    fn test_for_each_tagged_player_empty() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        // No tagged players
        let effect = ForEachTaggedPlayerEffect::new("nonexistent", vec![Effect::gain_life(5)]);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(0));
        assert_eq!(game.player(alice).unwrap().life, 20);
    }

    #[test]
    fn test_for_each_tagged_player_preserves_iterated_player() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice);

        // Set an initial iterated_player
        let original = PlayerId::from_index(99);
        ctx.iteration.iterated_player = Some(original);

        // Tag a player
        ctx.tag_player("test", bob);

        let effect = ForEachTaggedPlayerEffect::new("test", vec![Effect::gain_life(1)]);
        effect.execute(&mut game, &mut ctx).unwrap();

        // Should restore original iterated_player
        assert_eq!(ctx.iteration.iterated_player, Some(original));
    }
}
