//! ForEachTagged effect implementations.
//!
//! These effects iterate over objects that were tagged by prior effects in the same
//! spell/ability resolution, enabling patterns like:
//! - "Destroy all creatures. Their controllers each create a token for each creature
//!   they controlled that was destroyed this way."

use crate::effect::{Effect, EffectOutcome};
use crate::effects::{EffectExecutor, SimultaneousEffectProposal};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::PlayerId;
use crate::snapshot::ObjectSnapshot;
use crate::tag::TagKey;
use super::object_iteration::correlated_player_count;

fn add_correlated_player_count(
    player_counts: &mut Vec<(PlayerId, i64)>,
    player: PlayerId,
    outcomes: &[EffectOutcome],
) {
    let count = correlated_player_count(outcomes);
    if let Some((_, total)) = player_counts.iter_mut().find(|(actor, _)| *actor == player) {
        *total += count;
    } else {
        player_counts.push((player, count));
    }
}

fn summarize_tagged_iterations(
    outcomes: Vec<EffectOutcome>,
    ranges: &[(PlayerId, usize, usize)],
) -> EffectOutcome {
    let mut player_counts = Vec::new();
    for &(player, start, end) in ranges {
        add_correlated_player_count(&mut player_counts, player, &outcomes[start..end]);
    }
    EffectOutcome::aggregate_summing_counts(outcomes).with_player_counts(player_counts)
}

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

/// Prepared iterations over one player's captured tagged set.
///
/// The shared execution context is reused while every player's simultaneous
/// action is prepared. Retaining these snapshots keeps each player's tagged
/// cards paired with the proposals prepared for those cards.
#[derive(Debug)]
struct ForEachTaggedProposal {
    iterated_players: Vec<PlayerId>,
    iterations: Vec<Vec<Box<dyn SimultaneousEffectProposal>>>,
}

impl SimultaneousEffectProposal for ForEachTaggedProposal {
    fn damage_action_inputs(&self) -> Option<crate::effects::damage::DamageActionInputs> {
        crate::effects::damage::DamageActionInputs::collect(
            self.iterations
                .iter()
                .flatten()
                .map(|proposal| proposal.damage_action_inputs()),
        )
    }

    fn bind_damage_action(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        owner: &crate::effects::CompletedEffectOutputs,
    ) -> Result<crate::effects::DamageActionBinding, ExecutionError> {
        let mut bindings = Vec::new();
        let mut ranges = Vec::new();
        for (player, proposals) in self.iterated_players.into_iter().zip(self.iterations) {
            let start = bindings.len();
            for proposal in proposals {
                // Each child retains the object, controller and tagged history
                // captured during preparation. Binding never repeats selection
                // or commits the shared physical damage action.
                bindings.push(proposal.bind_damage_action(game, ctx, owner)?);
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(crate::effects::DamageActionBinding::from_outcome(
                        EffectOutcome::count(0),
                    ));
                }
            }
            ranges.push((player, start, bindings.len()));
        }
        Ok(crate::effects::DamageActionBinding::from_bindings(
            bindings,
            |outcomes| summarize_tagged_iterations(outcomes, &ranges),
        ))
    }

    fn declared_payment_resources(&self) -> Vec<crate::effects::PaymentResourceClaim> {
        self.iterations
            .iter()
            .flatten()
            .flat_map(|proposal| proposal.declared_payment_resources())
            .collect()
    }

    fn declared_life_payments(&self) -> Vec<(crate::ids::PlayerId, u32)> {
        self.iterations
            .iter()
            .flatten()
            .flat_map(|proposal| proposal.declared_life_payments())
            .collect()
    }

    fn prepare_selection(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<(), ExecutionError> {
        for proposal in self.iterations.iter_mut().flatten() {
            proposal.prepare_selection(game, ctx)?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(());
            }
        }
        Ok(())
    }

    fn prepare_original(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<(), ExecutionError> {
        for proposal in self.iterations.iter_mut().flatten() {
            proposal.prepare_original(game, ctx)?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(());
            }
        }
        Ok(())
    }

    fn seal_original(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<(), ExecutionError> {
        for proposal in self.iterations.iter_mut().flatten() {
            proposal.seal_original(game, ctx)?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(());
            }
        }
        Ok(())
    }

    fn commit_original(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::SimultaneousEffectCommit, ExecutionError> {
        self.commit_original_with_outputs(game, ctx)
            .map(crate::effects::SimultaneousEffectCommit::into_aggregate)
    }

    fn commit_original_with_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<
        crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
        ExecutionError,
    > {
        let mut receipts = Vec::new();
        let mut ranges = Vec::new();
        for (player, proposals) in self.iterated_players.into_iter().zip(self.iterations) {
            let start = receipts.len();
            for proposal in proposals {
                receipts.push(proposal.commit_original_with_outputs(game, ctx)?);
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(crate::effects::SimultaneousEffectCommit::finished(
                        crate::effects::CompletedEffectOutputs::aggregate_only(
                            EffectOutcome::count(0),
                        ),
                    ));
                }
            }
            ranges.push((player, start, receipts.len()));
        }
        Ok(super::compose_original_commits_with_projection_outputs(
            receipts,
            Box::new(move |outcomes| summarize_tagged_iterations(outcomes, &ranges)),
        ))
    }

    fn commit(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        super::complete_prepared_original(self, game, ctx)
    }
}

fn tagged_iteration_snapshots(
    effect: &ForEachTaggedEffect,
    ctx: &ExecutionContext,
) -> Vec<ObjectSnapshot> {
    ctx.get_tagged_all(&effect.tag).cloned().unwrap_or_default()
}
fn prepare_tagged_iteration_proposal(
    effect: &ForEachTaggedEffect,
    game: &GameState,
    ctx: &mut ExecutionContext,
    snapshots: Vec<ObjectSnapshot>,
) -> Result<Box<dyn SimultaneousEffectProposal>, ExecutionError> {
    let it_tag = TagKey::from("__it__");
    let previous_tag = TagKey::from(ironsmith_core::PREVIOUS_ITERATED_OBJECTS_TAG);
    super::with_iteration_tags(
        ctx,
        vec![(it_tag.clone(), None), (previous_tag.clone(), None)],
        |ctx| {
            let mut iterations = Vec::with_capacity(snapshots.len());
            let mut iterated_players = Vec::with_capacity(snapshots.len());
            for (index, snapshot) in snapshots.iter().enumerate() {
                // Query the same loop-visible bindings before installing
                // the current iteration's history and subject tags.
                let iterated_player = effect.iterated_player(game, ctx, snapshot)?;
                ctx.set_tagged_objects(previous_tag.clone(), snapshots[..index].to_vec());
                ctx.set_tagged_objects(it_tag.clone(), vec![snapshot.clone()]);
                let proposals = super::with_object_iteration(
                    ctx,
                    snapshot.object_id,
                    iterated_player,
                    Vec::new(),
                    |ctx| {
                        effect
                            .effects
                            .iter()
                            .map_while(|child| {
                                if ctx.decision_maker.awaiting_choice() {
                                    return None;
                                }
                                Some(child.prepare_simultaneous_player_action(game, ctx).map(
                                    |inner| {
                                        super::scope_prepared_iteration(
                                            inner,
                                            snapshot.object_id,
                                            iterated_player,
                                            vec![
                                                (effect.tag.clone(), snapshots.clone()),
                                                (it_tag.clone(), vec![snapshot.clone()]),
                                                (previous_tag.clone(), snapshots[..index].to_vec()),
                                            ],
                                        )
                                    },
                                ))
                            })
                            .collect::<Result<Vec<_>, ExecutionError>>()
                    },
                )?;
                iterated_players.push(iterated_player);
                iterations.push(proposals);
                if ctx.decision_maker.awaiting_choice() {
                    break;
                }
            }
            Ok(Box::new(ForEachTaggedProposal {
                iterated_players,
                iterations,
            }) as Box<dyn SimultaneousEffectProposal>)
        },
    )
}

impl EffectExecutor for ForEachTaggedEffect {
    fn supports_prepared_action_program(&self) -> bool {
        self.effects
            .iter()
            .all(super::action_program::action_program_child_is_prepared)
    }
    fn select_prepared_action_program(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Option<Box<dyn crate::effects::ActionProgramCursor>>, ExecutionError> {
        if ctx.decision_maker.awaiting_choice() {
            return Ok(None);
        }
        Ok(Some(tagged_object_cursor(self, game, ctx)?))
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn visit_child_effects(&self, visitor: &mut dyn FnMut(&Effect)) {
        for effect in &self.effects {
            visitor(effect);
        }
    }

    fn supports_damage_action_cohort(&self) -> bool {
        self.tag.as_str() != "__it__"
            && self.tag.as_str() != ironsmith_core::PREVIOUS_ITERATED_OBJECTS_TAG
            && matches!(self.effects.as_slice(), [child] if
                child.0.shares_iterated_damage_action() && child.0.supports_damage_action_cohort())
    }

    fn supports_simultaneous_player_action(&self) -> bool {
        self.supports_damage_action_cohort()
            && self.effects[0].0.supports_simultaneous_player_action()
    }

    fn prepare_simultaneous_player_action(
        &self, game: &GameState, ctx: &mut ExecutionContext,
    ) -> Result<Box<dyn SimultaneousEffectProposal>, ExecutionError> {
        if !self.supports_simultaneous_player_action() {
            return Err(ExecutionError::Impossible(
                "tagged iteration requires selected-program scheduling".into(),
            ));
        }
        let snapshots = tagged_iteration_snapshots(self, ctx);
        prepare_tagged_iteration_proposal(self, game, ctx, snapshots)
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.execute_with_outputs(game, ctx)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }

    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        crate::effects::tokens::execute_resource_transaction_with_pending_value(
            game,
            ctx,
            || crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            |game, ctx| execute_tagged_object_iterations(self, game, ctx),
        )
    }

    fn supports_replacement_draw_continuation(&self) -> bool {
        self.effects.iter().all(crate::effects::replacement::replacement_effect_supported)
    }

    fn prepare_replacement_draw_continuation_with_outputs(
        &self, game: &mut GameState, ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>, ExecutionError> {
        let cursor = self.select_prepared_action_program(game, ctx)?;
        super::object_iteration::prepare_iteration_continuation(cursor, game, ctx)
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
    fn supports_prepared_action_program(&self) -> bool {
        self.effects
            .iter()
            .all(super::action_program::action_program_child_is_prepared)
    }
    fn select_prepared_action_program(
        &self,
        _game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Option<Box<dyn crate::effects::ActionProgramCursor>>, ExecutionError> {
        if ctx.decision_maker.awaiting_choice() {
            return Ok(None);
        }
        Ok(Some(tagged_controller_cursor(self, ctx)))
    }

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
        self.execute_with_outputs(game, ctx)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }

    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        crate::effects::tokens::execute_resource_transaction_with_pending_value(
            game,
            ctx,
            || crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            |game, ctx| execute_tagged_controller_iterations(self, game, ctx),
        )
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
    fn supports_prepared_action_program(&self) -> bool {
        self.effects
            .iter()
            .all(super::action_program::action_program_child_is_prepared)
    }
    fn select_prepared_action_program(
        &self,
        _game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Option<Box<dyn crate::effects::ActionProgramCursor>>, ExecutionError> {
        if ctx.decision_maker.awaiting_choice() {
            return Ok(None);
        }
        Ok(Some(tagged_player_cursor(self, ctx)?))
    }

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
        self.execute_with_outputs(game, ctx)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }

    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        crate::effects::tokens::execute_resource_transaction_with_pending_value(
            game,
            ctx,
            || crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            |game, ctx| execute_tagged_player_iterations(self, game, ctx),
        )
    }
}

struct TaggedObjectPlan {
    effect: ForEachTaggedEffect,
    snapshots: Vec<ObjectSnapshot>,
}
impl super::iteration_program::SelectedIterationPlan for TaggedObjectPlan {
    fn len(&self) -> usize {
        self.snapshots.len()
    }
    fn effects(&self) -> &[Effect] {
        &self.effect.effects
    }
    fn root_tags(&self) -> Vec<(TagKey, Option<Vec<ObjectSnapshot>>)> {
        vec![(
            TagKey::from(ironsmith_core::PREVIOUS_ITERATED_OBJECTS_TAG),
            None,
        )]
    }
    fn entry_attachment_hints(&self) -> bool {
        true
    }
    fn select(
        &mut self,
        game: &GameState,
        ctx: &mut ExecutionContext,
        index: usize,
    ) -> Result<super::iteration_program::IterationInput, ExecutionError> {
        let snapshot = &self.snapshots[index];
        // Query before installing this occurrence's history, preserving the
        // previous occurrence's visible history and child context exports.
        let player = self.effect.iterated_player(game, ctx, snapshot)?;
        ctx.set_tagged_objects(
            TagKey::from(ironsmith_core::PREVIOUS_ITERATED_OBJECTS_TAG),
            self.snapshots[..index].to_vec(),
        );
        Ok(super::iteration_program::IterationInput {
            object: Some(snapshot.object_id),
            player,
            tags: vec![(TagKey::from("__it__"), vec![snapshot.clone()])],
        })
    }
    fn project(
        &self,
        outcomes: Vec<EffectOutcome>,
        ranges: &[(PlayerId, usize, usize)],
    ) -> EffectOutcome {
        summarize_tagged_iterations(outcomes, ranges)
    }
}
fn tagged_object_cursor(
    effect: &ForEachTaggedEffect,
    game: &GameState,
    ctx: &mut ExecutionContext,
) -> Result<Box<dyn crate::effects::ActionProgramCursor>, ExecutionError> {
    let snapshots = tagged_iteration_snapshots(effect, ctx);
    if snapshots.is_empty() {
        return Ok(super::action_program::finished_program_cursor(
            crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
        ));
    }
    if effect.supports_simultaneous_player_action() {
        let proposal = prepare_tagged_iteration_proposal(effect, game, ctx, snapshots)?;
        return Ok(super::action_program::prepared_damage_program_cursor(
            Effect::new(effect.clone()),
            proposal,
        ));
    }
    Ok(super::iteration_program::selected_iteration_cursor(
        Box::new(TaggedObjectPlan {
            effect: effect.clone(),
            snapshots,
        }),
        ctx,
    ))
}

struct TaggedControllerPlan {
    effects: Vec<Effect>,
    controllers: Vec<(PlayerId, usize)>,
}
impl super::iteration_program::SelectedIterationPlan for TaggedControllerPlan {
    fn len(&self) -> usize {
        self.controllers.len()
    }
    fn effects(&self) -> &[Effect] {
        &self.effects
    }
    fn result_slot(&self) -> Option<crate::effect::EffectId> {
        Some(crate::effect::EffectId::TAGGED_COUNT)
    }
    fn select(
        &mut self,
        _game: &GameState,
        ctx: &mut ExecutionContext,
        index: usize,
    ) -> Result<super::iteration_program::IterationInput, ExecutionError> {
        let (player, count) = self.controllers[index];
        ctx.store_outcome(
            crate::effect::EffectId::TAGGED_COUNT,
            EffectOutcome::count(count as i32),
        );
        Ok(super::iteration_program::IterationInput {
            object: None,
            player,
            tags: Vec::new(),
        })
    }
}
fn tagged_controller_cursor(
    effect: &ForEachControllerOfTaggedEffect,
    ctx: &mut ExecutionContext,
) -> Box<dyn crate::effects::ActionProgramCursor> {
    let mut controllers = ctx
        .count_tagged_by_controller(&effect.tag)
        .into_iter()
        .collect::<Vec<_>>();
    controllers.sort_by_key(|(player, _)| player.0);
    super::iteration_program::selected_iteration_cursor(
        Box::new(TaggedControllerPlan {
            effects: effect.effects.clone(),
            controllers,
        }),
        ctx,
    )
}
struct TaggedPlayerPlan {
    effects: Vec<Effect>,
    players: Vec<PlayerId>,
}
impl super::iteration_program::SelectedIterationPlan for TaggedPlayerPlan {
    fn len(&self) -> usize {
        self.players.len()
    }
    fn effects(&self) -> &[Effect] {
        &self.effects
    }
    fn select(
        &mut self,
        _game: &GameState,
        _ctx: &mut ExecutionContext,
        index: usize,
    ) -> Result<super::iteration_program::IterationInput, ExecutionError> {
        Ok(super::iteration_program::IterationInput {
            object: None,
            player: self.players[index],
            tags: Vec::new(),
        })
    }
}
fn tagged_player_cursor(
    effect: &ForEachTaggedPlayerEffect,
    ctx: &mut ExecutionContext,
) -> Result<Box<dyn crate::effects::ActionProgramCursor>, ExecutionError> {
    if effect.require_evidence && ctx.get_tagged_players(&effect.tag).is_none() {
        return Err(ExecutionError::IncompleteEvidence("required player-result roster is absent".into()));
    }
    let players = ctx
        .get_tagged_players(&effect.tag)
        .cloned()
        .unwrap_or_default();
    Ok(super::iteration_program::selected_iteration_cursor(
        Box::new(TaggedPlayerPlan {
            effects: effect.effects.clone(),
            players,
        }),
        ctx,
    ))
}
fn execute_tagged_object_iterations(
    effect: &ForEachTaggedEffect,
    game: &mut GameState,
    ctx: &mut ExecutionContext,
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    super::action_program::execute_action_program_with_outputs(
        tagged_object_cursor(effect, game, ctx)?,
        game,
        ctx,
        crate::effects::EffectExecutionPurpose::Action,
    )
}
fn execute_tagged_controller_iterations(
    effect: &ForEachControllerOfTaggedEffect,
    game: &mut GameState,
    ctx: &mut ExecutionContext,
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    super::action_program::execute_action_program_with_outputs(
        tagged_controller_cursor(effect, ctx),
        game,
        ctx,
        crate::effects::EffectExecutionPurpose::Action,
    )
}
fn execute_tagged_player_iterations(
    effect: &ForEachTaggedPlayerEffect,
    game: &mut GameState,
    ctx: &mut ExecutionContext,
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    super::action_program::execute_action_program_with_outputs(
        tagged_player_cursor(effect, ctx)?,
        game,
        ctx,
        crate::effects::EffectExecutionPurpose::Action,
    )
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
