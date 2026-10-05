//! ForEachTagged effect implementations.
//!
//! These effects iterate over objects that were tagged by prior effects in the same
//! spell/ability resolution, enabling patterns like:
//! - "Destroy all creatures. Their controllers each create a token for each creature
//!   they controlled that was destroyed this way."

use crate::effect::{Effect, EffectOutcome};
use crate::effects::{EffectExecutor, SimultaneousEffectProposal};
use crate::effects::{ExecutionContext, ExecutionError, execute_effect};
use crate::game_state::GameState;
use crate::ids::PlayerId;
use crate::snapshot::ObjectSnapshot;
use crate::tag::TagKey;
use super::object_iteration::{ObjectIterationBinding, ObjectIterationProposal, ObjectIterationState};

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
    /// Bind the iterated player to the controller recorded by the latest block
    /// event in which this object was blocked by an object in the tagged set.
    pub controller_at_last_blocked_by: Option<TagKey>,
}

impl ForEachTaggedEffect {
    /// Create a new ForEachTagged effect.
    pub fn new(tag: impl Into<TagKey>, effects: Vec<Effect>) -> Self {
        Self {
            tag: tag.into(),
            effects,
            controller_at_last_blocked_by: None,
        }
    }

    pub fn with_controller_at_last_blocked_by(mut self, tag: impl Into<TagKey>) -> Self {
        self.controller_at_last_blocked_by = Some(tag.into());
        self
    }

    fn iterated_player(
        &self,
        game: &GameState,
        ctx: &ExecutionContext,
        snapshot: &ObjectSnapshot,
    ) -> Result<PlayerId, ExecutionError> {
        let Some(blocker_tag) = &self.controller_at_last_blocked_by else {
            return Ok(snapshot.controller);
        };
        let blockers = ctx.get_tagged_all(blocker_tag).ok_or_else(|| {
            ExecutionError::UnresolvableValue(format!(
                "historical block controller requires tagged blockers '{blocker_tag}'"
            ))
        })?;
        game.turn_store
            .turn_history
            .projected_records()
            .rev()
            .filter_map(|record| {
                record
                    .event
                    .downcast::<crate::events::combat::CreatureBlockedEvent>()
            })
            .find_map(|event| {
                let attacker = event.attacker_snapshot.as_ref()?;
                if attacker.stable_id != snapshot.stable_id {
                    return None;
                }
                let blocker = event.blocker_snapshot.as_ref()?;
                blockers
                    .iter()
                    .any(|candidate| candidate.stable_id == blocker.stable_id)
                    .then_some(attacker.controller)
            })
            .ok_or_else(|| {
                ExecutionError::UnresolvableValue(format!(
                    "no matching historical block event for iterated object {:?} and tagged blockers '{blocker_tag}'",
                    snapshot.object_id
                ))
            })
    }
}

fn tagged_bindings(effect: &ForEachTaggedEffect, game: &GameState, ctx: &ExecutionContext)
    -> Result<Vec<ObjectIterationBinding>, ExecutionError> {
    let snapshots = ctx.get_tagged_all(&effect.tag).cloned().unwrap_or_default();
    snapshots.iter().enumerate().map(|(index, snapshot)| Ok(ObjectIterationBinding {
        object: snapshot.object_id, snapshot: snapshot.clone(),
        player: effect.iterated_player(game, ctx, snapshot)?,
        previous: Some(snapshots[..index].to_vec()),
    })).collect()
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
        // Multiple authored instructions use ForPlayers' ordered native
        // route so a later instruction cannot overtake an earlier draw.
        // Transparent child wrappers retain their scopes through that route.
        self.effects.len() == 1
            && self.tag.as_str() != "__it__"
            && self.tag.as_str() != ironsmith_core::PREVIOUS_ITERATED_OBJECTS_TAG
            && self
                .effects
                .iter()
                .all(|effect| effect.transparent_child_effect().is_none()
                    && effect.0.supports_simultaneous_player_action())
    }

    fn prepare_simultaneous_player_action(
        &self, game: &GameState, ctx: &mut ExecutionContext,
    ) -> Result<Box<dyn SimultaneousEffectProposal>, ExecutionError> {
        let bindings = tagged_bindings(self, game, ctx)?;
        let mut iterations = Vec::new();
        for binding in &bindings {
            let proposals = binding.with_scope(ctx, |ctx| self.effects.iter()
                .map(|effect| effect.0.prepare_simultaneous_player_action(game, ctx))
                .collect::<Result<Vec<_>, _>>())?;
            iterations.push(proposals);
            if ctx.decision_maker.awaiting_choice() { break; }
        }
        let snapshots = bindings.iter().map(|binding| binding.snapshot.clone()).collect();
        Ok(Box::new(ObjectIterationProposal { bindings, iterations, correlated: true,
            tagged_set: Some((self.tag.clone(), snapshots)), shuffle_owners: Vec::new(),
            attachments: self.effects.iter().enumerate().map(|(index, effect)|
                crate::effects::permanents::entry_attachment_for_move(effect, self.effects.get(index + 1))).collect(),
        }))
    }

    fn supports_replacement_draw_continuation(&self) -> bool {
        self.effects.iter().all(crate::effects::replacement::replacement_effect_supported)
    }

    fn prepare_replacement_draw_continuation(&self, game: &mut GameState, ctx: &mut ExecutionContext)
        -> Result<crate::effects::SimultaneousEffectCommit, ExecutionError> {
        ObjectIterationState::new(tagged_bindings(self, game, ctx)?, self.effects.clone(), true, true)
            .run(game, ctx, true)
    }

    fn execute(&self, game: &mut GameState, ctx: &mut ExecutionContext)
        -> Result<EffectOutcome, ExecutionError> {
        crate::effects::tokens::execute_resource_transaction_atomically(game, ctx, |game, ctx| {
            crate::effects::runtime::with_per_event_trigger_matching(game, true, |game| {
                ObjectIterationState::new(tagged_bindings(self, game, ctx)?, self.effects.clone(), true, true)
                    .run(game, ctx, false).map(|committed| committed.outcome)
            })
        })
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
    /// Missing evidence is an execution error; a present empty roster is valid.
    pub require_evidence: bool,
}

impl ForEachTaggedPlayerEffect {
    /// Create a new ForEachTaggedPlayer effect.
    pub fn new(tag: impl Into<TagKey>, effects: Vec<Effect>) -> Self {
        Self {
            tag: tag.into(),
            effects,
            require_evidence: false,
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
        crate::effects::tokens::execute_resource_transaction_atomically(game, ctx, |game, ctx| {
            let players = match ctx.get_tagged_players(&self.tag) {
                Some(players) => players.clone(),
                None if self.require_evidence => return Err(ExecutionError::IncompleteEvidence(
                    "required player-result roster is absent".into(),
                )),
                None => return Ok(EffectOutcome::count(0)),
            };
            let mut outcomes = Vec::new();
            for player_id in players {
                ctx.with_temp_iterated_player(Some(player_id), |ctx| {
                    for effect in &self.effects {
                        outcomes.push(execute_effect(game, effect, ctx)?);
                        if ctx.decision_maker.awaiting_choice() { break; }
                    }
                    Ok::<(), ExecutionError>(())
                })?;
                if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
            }
            Ok(EffectOutcome::aggregate_summing_counts(outcomes))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effect::ExecutionFact;
    use super::super::object_iteration::correlated_player_count;
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

    #[test]
    fn accepted_zero_result_still_counts_as_correlated_player_action() {
        let accepted = EffectOutcome::count(0).with_execution_fact(ExecutionFact::Accepted);
        assert_eq!(correlated_player_count(&[accepted]), 1);
        assert_eq!(correlated_player_count(&[EffectOutcome::declined()]), 0);
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
    fn historical_block_controller_binds_iterated_player_by_stable_identity() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();
        let attacker = create_creature(&mut game, "Attacker", bob);
        let wall = create_creature(&mut game, "Wall", alice);

        let wall_at_block = ObjectSnapshot::from_object(game.object(wall).unwrap(), &game);
        let attacker_at_block = ObjectSnapshot::from_object(game.object(attacker).unwrap(), &game);
        let event = crate::triggers::TriggerEvent::new(
            crate::events::combat::CreatureBlockedEvent::with_snapshots(
                wall,
                attacker,
                wall_at_block.clone(),
                attacker_at_block.clone(),
            ),
            crate::provenance::ProvNodeId::default(),
        );
        game.record_turn_history_event(&event);

        // The successful-destroy result may carry a later controller. Stable
        // identity must still recover the controller stored by the block event.
        let mut destroyed_lki = attacker_at_block;
        destroyed_lki.controller = alice;
        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.tag_objects("wall", vec![wall_at_block]);
        ctx.tag_objects("destroyed", vec![destroyed_lki]);

        let effect = ForEachTaggedEffect::new(
            "destroyed",
            vec![Effect::gain_life_player(
                1,
                crate::target::ChooseSpec::Player(crate::target::PlayerFilter::IteratedPlayer),
            )],
        )
        .with_controller_at_last_blocked_by("wall");
        effect.execute(&mut game, &mut ctx).expect("execute");

        assert_eq!(game.player(alice).unwrap().life, 20);
        assert_eq!(game.player(bob).unwrap().life, 21);
    }

    #[test]
    fn historical_block_controller_skips_later_unrelated_blocker_and_rejects_no_match() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();
        let attacker = create_creature(&mut game, "Attacker", bob);
        let wall = create_creature(&mut game, "Wall", alice);
        let other_blocker = create_creature(&mut game, "Other blocker", alice);

        let wall_snapshot = ObjectSnapshot::from_object(game.object(wall).unwrap(), &game);
        let attacker_snapshot = ObjectSnapshot::from_object(game.object(attacker).unwrap(), &game);
        for blocker in [wall, other_blocker] {
            let blocker_snapshot =
                ObjectSnapshot::from_object(game.object(blocker).unwrap(), &game);
            let mut event_attacker = attacker_snapshot.clone();
            if blocker == other_blocker {
                event_attacker.controller = alice;
            }
            let event = crate::triggers::TriggerEvent::new(
                crate::events::combat::CreatureBlockedEvent::with_snapshots(
                    blocker,
                    attacker,
                    blocker_snapshot,
                    event_attacker,
                ),
                crate::provenance::ProvNodeId::default(),
            );
            game.record_turn_history_event(&event);
        }

        let mut destroyed_lki = attacker_snapshot;
        destroyed_lki.controller = alice;
        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.tag_objects("wall", vec![wall_snapshot]);
        ctx.tag_objects("destroyed", vec![destroyed_lki.clone()]);
        let effect = ForEachTaggedEffect::new(
            "destroyed",
            vec![Effect::gain_life_player(
                1,
                crate::target::ChooseSpec::Player(crate::target::PlayerFilter::IteratedPlayer),
            )],
        )
        .with_controller_at_last_blocked_by("wall");
        effect.execute(&mut game, &mut ctx).expect("matching wall");
        assert_eq!(game.player(bob).unwrap().life, 21);

        let unrelated = create_creature(&mut game, "Unrelated", alice);
        let unrelated_snapshot =
            ObjectSnapshot::from_object(game.object(unrelated).unwrap(), &game);
        ctx.set_tagged_objects("wall", vec![unrelated_snapshot]);
        ctx.set_tagged_objects("destroyed", vec![destroyed_lki]);
        assert!(matches!(
            effect.execute(&mut game, &mut ctx),
            Err(ExecutionError::UnresolvableValue(_))
        ));
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
