//! Sacrifice effect implementation.

use crate::effect::{EffectOutcome, ExecutionFact, Value};
use crate::effects::helpers::{
    normalize_object_selection, resolve_player_filter, resolve_single_object_for_effect,
    resolve_value,
};
use crate::effects::{CostExecutableEffect, EffectExecutor};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::events::permanents::SacrificeEvent;
use crate::events::processing::EventOutcome;
use crate::filter::ObjectFilterExt as _;
use crate::filter::PlayerFilterExt;
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::snapshot::ObjectSnapshot;
use crate::tag::TagKey;
use crate::target::{ChooseSpec, ObjectFilter, PlayerFilter};
use crate::triggers::TriggerEvent;
use crate::zone::Zone;
pub use ironsmith_core::SacrificePlayerEffect;

/// Retain the permanent incarnation used to pay a sacrifice cost. Movement
/// journals supply immutable departure snapshots; later payment/reflexive
/// frames must not substitute the new graveyard or redirected incarnation.
fn retain_sacrifice_payment_bindings(
    game: &GameState,
    outcome: &EffectOutcome,
    execution: &mut ExecutionContext,
) {
    let receipts = execution
        .tagged_objects
        .iter()
        .filter_map(|(tag, snapshots)| {
            tag.as_str()
                .strip_prefix("__pre_move_history__")
                .map(|tag| (crate::tag::TagKey::from(tag), snapshots.clone()))
        })
        .collect::<Vec<_>>();
    for (tag, snapshots) in receipts {
        execution.set_tagged_objects(tag, snapshots);
    }
    let frozen = execution
        .tagged_objects
        .iter()
        .filter_map(|(tag, snapshots)| {
            if tag.as_str().starts_with("__") {
                return None;
            }
            let departed = snapshots
                .iter()
                .filter(|snapshot| {
                    snapshot.zone == crate::zone::Zone::Battlefield
                        && game.object(snapshot.object_id).is_none()
                })
                .cloned()
                .collect::<Vec<_>>();
            (!departed.is_empty()).then(|| {
                (
                    crate::tag::TagKey::from(format!("__paid_departure__{}", tag.as_str())),
                    departed,
                )
            })
        })
        .collect::<Vec<_>>();
    for (tag, snapshots) in frozen {
        execution.set_tagged_objects(tag, snapshots);
    }
    let originals = outcome
        .instruction_result()
        .execution_facts
        .iter()
        .rev()
        .find_map(|fact| match fact {
            ExecutionFact::OriginalSacrificeObjects(objects) => Some(objects.clone()),
            _ => None,
        })
        .unwrap_or_default();
    let result_tags = execution
        .tagged_objects
        .iter()
        .filter_map(|(tag, selected)| {
            let tag = ironsmith_core::tag::SacrificeCostTag::parse(tag)?;
            if !matches!(tag, ironsmith_core::tag::SacrificeCostTag::Selected(_)) {
                return None;
            }
            let actual = originals
                .iter()
                .filter(|original| {
                    selected.iter().any(|selected| {
                        selected.object_id == original.object_id
                            || selected.stable_id == original.stable_id
                    })
                })
                .cloned()
                .collect();
            Some((tag.original_result_key(), actual))
        })
        .collect::<Vec<_>>();
    for (tag, originals) in result_tags {
        execution.set_tagged_objects(tag, originals);
    }
}

fn players_in_turn_order(game: &GameState) -> Vec<PlayerId> {
    game.team_apnap_player_order()
}

fn choose_objects_to_sacrifice(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    player_id: PlayerId,
    filter: &ObjectFilter,
    count: usize,
) -> Result<Vec<ObjectId>, ExecutionError> {
    use crate::decisions::make_decision;
    use crate::decisions::specs::ChooseObjectsSpec;

    let filter_ctx = ctx.filter_context(game);
    let matching: Vec<ObjectId> = game
        .battlefield
        .iter()
        .filter_map(|&id| game.object(id).map(|obj| (id, obj)))
        .filter(|(id, obj)| {
            game.controller_of(obj) == player_id
                && filter.matches(obj, &filter_ctx, game)
                && game.can_be_sacrificed_with_cause(*id, &ctx.cause)
        })
        .map(|(id, _)| id)
        .collect();

    let required = count.min(matching.len());
    if required == 0 {
        return Ok(Vec::new());
    }

    let chosen = if required == matching.len() {
        matching.clone()
    } else {
        let spec = ChooseObjectsSpec::new(
            ctx.source,
            format!("Choose {} {} to sacrifice", required, filter.description()),
            matching.clone(),
            required,
            Some(required),
        );
        make_decision(game, ctx.decision_maker, player_id, Some(ctx.source), spec)
    };
    if ctx.decision_maker.awaiting_choice() {
        return Ok(Vec::new());
    }

    Ok(normalize_object_selection(chosen, &matching, required))
}

fn max_sacrifice_cost_x(
    game: &GameState,
    source: ObjectId,
    controller: PlayerId,
    filter: &ObjectFilter,
    player: &PlayerFilter,
) -> Option<u32> {
    let mut dm = crate::decision::SelectFirstDecisionMaker;
    let ctx = ExecutionContext::new(source, controller, &mut dm);
    let player_id = resolve_player_filter(game, player, &ctx).unwrap_or(controller);
    let filter_ctx = ctx.filter_context(game);

    Some(
        game.battlefield
            .iter()
            .filter_map(|&id| game.object(id).map(|obj| (id, obj)))
            .filter(|(id, obj)| {
                game.controller_of(obj) == player_id
                    && filter.matches(obj, &filter_ctx, game)
                    && game.can_be_sacrificed_with_cause(*id, &ctx.cause)
            })
            .count() as u32,
    )
}

fn tagged_selection_tag(filter: &ObjectFilter) -> Option<&crate::tag::TagKey> {
    filter
        .tagged_constraints
        .iter()
        .find(|constraint| {
            constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
        })
        .map(|constraint| &constraint.tag)
}

fn dynamic_count_tracks_tagged_selection(
    count_filter: &ObjectFilter,
    sacrifice_filter: &ObjectFilter,
) -> bool {
    if count_filter == sacrifice_filter {
        return true;
    }
    tagged_selection_tag(count_filter)
        .zip(tagged_selection_tag(sacrifice_filter))
        .is_some_and(|(count_tag, sacrifice_tag)| count_tag == sacrifice_tag)
}

fn tag_sacrifice_zone_change_event(
    game: &mut GameState,
    event_object: ObjectId,
    object_tags: &[TagKey],
    source_tags: &[TagKey],
    sacrificed_snapshot: Option<&ObjectSnapshot>,
    source_snapshot: Option<&ObjectSnapshot>,
) {
    if let Some(snapshot) = sacrificed_snapshot {
        for tag in object_tags {
            game.tag_pending_zone_change_event_for_object(
                event_object,
                tag.clone(),
                snapshot.clone(),
            );
        }
    }
    if let Some(snapshot) = source_snapshot {
        for tag in source_tags {
            game.tag_pending_zone_change_event_for_object(
                event_object,
                tag.clone(),
                snapshot.clone(),
            );
        }
    }
}

/// Effect that makes a player sacrifice permanents.
///
/// Sacrifice moves permanents from the battlefield to the graveyard.
/// The player chooses which permanents to sacrifice from among those
/// they control that match the filter.
///
/// Note: Unlike destroy, sacrifice is not prevented by indestructible.
///
/// # Fields
///
/// * `filter` - Which permanents can be sacrificed
/// * `count` - How many permanents to sacrifice
/// * `player` - Which player sacrifices
///
/// # Example
///
/// ```ignore
/// // Sacrifice a creature
/// let effect = SacrificeEffect::you(ObjectFilter::creature(), 1);
///
/// // Each opponent sacrifices a creature
/// // (use ForEachOpponent with this effect)
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct SacrificeEffect {
    /// Which permanents can be sacrificed.
    pub filter: ObjectFilter,
    /// How many permanents to sacrifice.
    pub count: Value,
    /// Which player sacrifices.
    pub player: PlayerFilter,
    /// Tags to attach to the sacrificed object's zone-change event.
    pub event_object_tags: Vec<TagKey>,
    /// Tags to attach to the source object on the sacrificed object's zone-change event.
    pub event_source_tags: Vec<TagKey>,
}

impl SacrificeEffect {
    /// Create a new sacrifice effect.
    pub fn new(filter: ObjectFilter, count: impl Into<Value>, player: PlayerFilter) -> Self {
        Self {
            filter,
            count: count.into(),
            player,
            event_object_tags: Vec::new(),
            event_source_tags: Vec::new(),
        }
    }

    /// Create an effect where you sacrifice permanents.
    pub fn you(filter: ObjectFilter, count: impl Into<Value>) -> Self {
        Self::new(filter, count, PlayerFilter::You)
    }

    /// Create an effect where you sacrifice a creature.
    pub fn you_creature(count: impl Into<Value>) -> Self {
        Self::you(ObjectFilter::creature(), count)
    }

    /// Create an effect where a specific player sacrifices.
    pub fn player(filter: ObjectFilter, count: impl Into<Value>, player: PlayerFilter) -> Self {
        Self::new(filter, count, player)
    }

    pub fn with_event_object_tag(mut self, tag: impl Into<TagKey>) -> Self {
        self.event_object_tags.push(tag.into());
        self
    }

    pub fn with_event_source_tag(mut self, tag: impl Into<TagKey>) -> Self {
        self.event_source_tags.push(tag.into());
        self
    }
}

impl EffectExecutor for SacrificePlayerEffect {
    fn cost_choice_bindings(&self) -> crate::effects::CostChoiceBindings {
        if self.player == PlayerFilter::You {
            crate::effects::CostChoiceBindings::from_filter(&self.filter)
        } else {
            crate::effects::CostChoiceBindings::default()
        }
    }

    fn result_action(&self) -> Option<crate::effect::PriorEffectAction> {
        Some(crate::effect::PriorEffectAction::Sacrificed)
    }
    fn as_cost_executable(&self) -> Option<&dyn CostExecutableEffect> {
        Some(self)
    }

    fn supports_simultaneous_player_action(&self) -> bool {
        true
    }

    fn prepare_simultaneous_player_action(
        &self,
        game: &GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Box<dyn crate::effects::SimultaneousEffectProposal>, ExecutionError> {
        let effect =
            SacrificeEffect::player(self.filter.clone(), self.count.clone(), self.player.clone());
        Ok(Box::new(effect.prepare_proposal(game, ctx)?))
    }

    fn decision_related_object_specs(&self) -> Vec<ChooseSpec> {
        vec![ChooseSpec::All(self.filter.clone())]
    }

    fn references_cost_x(&self) -> bool {
        self.count == Value::X
    }

    fn max_cost_x(&self, game: &GameState, source: ObjectId, controller: PlayerId) -> Option<u32> {
        if !self.references_cost_x() {
            return None;
        }
        max_sacrifice_cost_x(game, source, controller, &self.filter, &self.player)
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
        game.refresh_continuous_state()
            .map_err(ExecutionError::ContinuousDiscovery)?;
        SacrificeEffect::player(self.filter.clone(), self.count.clone(), self.player.clone())
            .execute_child_with_outputs(game, ctx)
    }
}

impl CostExecutableEffect for SacrificePlayerEffect {
    fn cost_choice_candidate_is_eligible(
        &self,
        game: &GameState,
        execution: &mut ExecutionContext,
        reason: crate::costs::PaymentReason,
        tag: &crate::tag::TagKey,
        object: ObjectId,
    ) -> Option<bool> {
        sacrifice_cost_choice_candidate_is_eligible(
            &self.filter,
            &self.player,
            game,
            execution,
            reason,
            tag,
            object,
        )
    }

    fn finalize_payment_bindings(
        &self,
        game: &GameState,
        outcome: &EffectOutcome,
        execution: &mut ExecutionContext,
        _payment_x: Option<u32>,
    ) -> Result<(), crate::cost::CostPaymentError> {
        retain_sacrifice_payment_bindings(game, outcome, execution);
        Ok(())
    }

    fn can_execute_as_cost_with_reason(
        &self,
        game: &GameState,
        source: crate::ids::ObjectId,
        controller: crate::ids::PlayerId,
        reason: crate::costs::PaymentReason,
    ) -> Result<(), crate::effects::CostValidationError> {
        let effect =
            SacrificeEffect::player(self.filter.clone(), self.count.clone(), self.player.clone());
        CostExecutableEffect::can_execute_as_cost_with_reason(
            &effect, game, source, controller, reason,
        )
    }

    fn can_execute_as_cost(
        &self,
        game: &GameState,
        source: crate::ids::ObjectId,
        controller: crate::ids::PlayerId,
    ) -> Result<(), crate::effects::CostValidationError> {
        let effect =
            SacrificeEffect::player(self.filter.clone(), self.count.clone(), self.player.clone());
        CostExecutableEffect::can_execute_as_cost(&effect, game, source, controller)
    }
}

impl EffectExecutor for SacrificeEffect {
    fn cost_choice_bindings(&self) -> crate::effects::CostChoiceBindings {
        if self.player == PlayerFilter::You {
            crate::effects::CostChoiceBindings::from_filter(&self.filter)
        } else {
            crate::effects::CostChoiceBindings::default()
        }
    }

    fn result_action(&self) -> Option<crate::effect::PriorEffectAction> {
        Some(crate::effect::PriorEffectAction::Sacrificed)
    }
    fn as_cost_executable(&self) -> Option<&dyn CostExecutableEffect> {
        Some(self)
    }

    fn supports_simultaneous_player_action(&self) -> bool {
        true
    }

    fn prepare_simultaneous_player_action(
        &self,
        game: &GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Box<dyn crate::effects::SimultaneousEffectProposal>, ExecutionError> {
        Ok(Box::new(self.prepare_proposal(game, ctx)?))
    }

    fn decision_related_object_specs(&self) -> Vec<ChooseSpec> {
        vec![ChooseSpec::All(self.filter.clone())]
    }

    fn references_cost_x(&self) -> bool {
        self.count == Value::X
    }

    fn max_cost_x(&self, game: &GameState, source: ObjectId, controller: PlayerId) -> Option<u32> {
        if !self.references_cost_x() {
            return None;
        }
        max_sacrifice_cost_x(game, source, controller, &self.filter, &self.player)
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
        game.refresh_continuous_state()
            .map_err(ExecutionError::ContinuousDiscovery)?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }
        crate::effects::composition::execute_transaction(
            game,
            ctx,
            || crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            |game, ctx| {
                let player_id = resolve_player_filter(game, &self.player, ctx)?;
                let count = resolve_value(game, &self.count, ctx)?.max(0) as usize;
                let explicit_targets: Vec<ObjectId> = ctx
                    .targets
                    .iter()
                    .filter_map(|target| match target {
                        crate::effects::ResolvedTarget::Object(id) => Some(*id),
                        crate::effects::ResolvedTarget::Player(_) => None,
                    })
                    .collect();
                let to_sacrifice = if count == 0 {
                    Vec::new()
                } else if !explicit_targets.is_empty() {
                    let filter_ctx = ctx.filter_context(game);
                    let matching: Vec<ObjectId> = game
                        .battlefield
                        .iter()
                        .filter_map(|&id| game.object(id).map(|obj| (id, obj)))
                        .filter(|(id, obj)| {
                            game.controller_of(obj) == player_id
                                && self.filter.matches(obj, &filter_ctx, game)
                                && game.can_be_sacrificed_with_cause(*id, &ctx.cause)
                        })
                        .map(|(id, _)| id)
                        .collect();
                    let required = count.min(matching.len());
                    normalize_object_selection(explicit_targets, &matching, required)
                } else {
                    choose_objects_to_sacrifice(game, ctx, player_id, &self.filter, count)?
                };
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }
                sacrifice_selected_objects_with_outputs(
                    game,
                    ctx,
                    &self.event_object_tags,
                    &self.event_source_tags,
                    to_sacrifice,
                )
            },
        )
    }

    fn cost_description(&self) -> Option<String> {
        let count = match self.count {
            crate::effect::Value::Fixed(count) if count > 0 => count,
            _ => return None,
        };
        if self.player != PlayerFilter::You {
            return None;
        }
        let mut display_filter = self.filter.clone();
        // A player can sacrifice only a permanent they control. The chooser
        // constraint remains executable, but repeating it in a cost invents
        // an unauthored "you control" qualifier for ordinary sacrifice costs.
        if display_filter.controller == Some(PlayerFilter::You) {
            display_filter.controller = None;
        }
        let description = display_filter.description();
        Some(if count == 1 {
            if description.starts_with("a ")
                || description.starts_with("an ")
                || description.starts_with("another ")
                || description.starts_with("target ")
                || description.starts_with("this ")
            {
                format!("Sacrifice {description}")
            } else {
                format!("Sacrifice a {description}")
            }
        } else {
            format!("Sacrifice {} {}", count, description)
        })
    }
}

/// One player's fully determined part of a simultaneous "each player
/// sacrifices ..." action (CR 101.4, 608.2f): the objects were chosen against
/// the pre-action game state; committing performs the zone changes.
#[derive(Debug)]
struct SacrificeProposal {
    prepared: PreparedSacrifices,
    event_object_tags: Vec<TagKey>,
    event_source_tags: Vec<TagKey>,
}

impl crate::effects::SimultaneousEffectProposal for SacrificeProposal {
    fn prepare_original(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<(), ExecutionError> {
        self.prepared.prepare_original(game, ctx)
    }
    fn commit_original(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::SimultaneousEffectCommit, ExecutionError> {
        commit_sacrifices(
            game,
            ctx,
            &self.event_object_tags,
            &self.event_source_tags,
            self.prepared,
        )
    }
    fn commit_original_with_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<
        crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
        ExecutionError,
    > {
        self.commit_original(game, ctx)
            .map(crate::effects::SimultaneousEffectCommit::into_retained)
    }
    fn commit(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        crate::effects::composition::complete_prepared_original_with_outputs(self, game, ctx, true)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }
}

impl SacrificeEffect {
    /// Choose this player's sacrifices against immutable pre-action state.
    fn prepare_proposal(
        &self,
        game: &GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<SacrificeProposal, ExecutionError> {
        let checked = game
            .continuous_query_snapshot()
            .map_err(ExecutionError::ContinuousDiscovery)?;
        let game = &checked;
        let player_id = resolve_player_filter(game, &self.player, ctx)?;
        let count = resolve_value(game, &self.count, ctx)?.max(0) as usize;
        let filter_ctx = ctx.filter_context(game);
        let matching: Vec<ObjectId> = game
            .battlefield
            .iter()
            .filter_map(|&id| game.object(id).map(|obj| (id, obj)))
            .filter(|(id, obj)| {
                game.controller_of(obj) == player_id
                    && self.filter.matches(obj, &filter_ctx, game)
                    && game.can_be_sacrificed_with_cause(*id, &ctx.cause)
            })
            .map(|(id, _)| id)
            .collect();
        let required = count.min(matching.len());
        let chosen = if required == 0 {
            Vec::new()
        } else if required == matching.len() {
            matching.clone()
        } else {
            let spec = crate::decisions::specs::ChooseObjectsSpec::new(
                ctx.source,
                format!(
                    "Choose {} {} to sacrifice",
                    required,
                    self.filter.description()
                ),
                matching.clone(),
                required,
                Some(required),
            );
            let selected = crate::decisions::make_decision(
                game,
                ctx.decision_maker,
                player_id,
                Some(ctx.source),
                spec,
            );
            normalize_object_selection(selected, &matching, required)
        };
        Ok(SacrificeProposal {
            prepared: PreparedSacrifices::capture(game, ctx, chosen)?,
            event_object_tags: self.event_object_tags.clone(),
            event_source_tags: self.event_source_tags.clone(),
        })
    }
}

/// Permanents sacrificed by one instruction leave the battlefield at the same
/// time, so every sacrifice (and dies) event of the batch looks back at the
/// same pre-batch trigger sources (CR 603.10a, 603.6c). A watcher such as
/// Mayhem Devil sacrificed together with a Treasure still triggers for both.
///
/// Returns the look-back to attach to the batch's sacrifice events, and
/// whether this call pinned it (and so must release it with
/// [`end_sacrifice_batch_lookback`]).
fn begin_sacrifice_batch_lookback(
    game: &mut GameState,
    batch_size: usize,
) -> (Option<Vec<ObjectSnapshot>>, bool) {
    if let Some(existing) = game.simultaneous_event_lookback() {
        return (Some(existing.to_vec()), false);
    }
    if batch_size < 2 {
        return (None, false);
    }
    let lookback = game.trigger_source_lookback_snapshots();
    game.set_simultaneous_event_lookback(Some(lookback.clone()));
    (Some(lookback), true)
}

fn end_sacrifice_batch_lookback(game: &mut GameState, pinned: bool) {
    if pinned {
        game.set_simultaneous_event_lookback(None);
    }
}

fn with_sacrifice_batch_lookback(
    events: Vec<TriggerEvent>,
    lookback: Option<Vec<ObjectSnapshot>>,
) -> Vec<TriggerEvent> {
    let Some(lookback) = lookback else {
        return events;
    };
    events
        .into_iter()
        .map(|event| event.with_lookback_source_snapshots(lookback.clone()))
        .collect()
}

/// Move the already-chosen objects to the graveyard as sacrifices, emitting
/// the same events and outcome facts regardless of whether the selection came
/// from a live execution or a simultaneous each-player proposal.
#[derive(Debug)]
struct PreparedSacrifices {
    chosen: Vec<ObjectId>,
    eligible: std::collections::HashSet<ObjectId>,
    source_snapshot: Option<ObjectSnapshot>,
    draws: super::ZoneInstructionDraws,
    originals_prepared: bool,
}
impl PreparedSacrifices {
    /// Every participant captures original characteristics and eligibility
    /// before another participant's replacement prefix can mutate the world.
    fn capture(
        game: &GameState,
        ctx: &ExecutionContext,
        chosen: Vec<ObjectId>,
    ) -> Result<Self, ExecutionError> {
        let mut ids = chosen.clone();
        ids.push(ctx.source);
        for id in ids.clone() {
            if let Some(object) = game.object(id) {
                ids.extend(object.attachments.iter().copied());
            }
        }
        ids.sort_unstable();
        ids.dedup();
        let characteristics = game
            .try_current_characteristics_batch(&ids)
            .map_err(ExecutionError::ContinuousDiscovery)?;
        let snapshot = |id| -> Result<Option<ObjectSnapshot>, ExecutionError> {
            let Some(object) = game.object(id) else {
                return Ok(None);
            };
            // A phased permanent is a known nonparticipant, not a failed
            // characteristic query. The ability source uses its retained LKI.
            if object.zone == Zone::Battlefield && game.is_phased_out(id) {
                return Ok(None);
            }
            let calculated = characteristics.get(&id).ok_or_else(|| ExecutionError::ContinuousDiscovery(
                crate::static_ability_processor::StaticEffectDiscoveryError::UnavailableCharacteristics { object: id }))?;
            Ok(Some(
                ObjectSnapshot::from_object_with_known_characteristics(
                    object,
                    game,
                    Some(calculated),
                ),
            ))
        };
        let mut snapshots = std::collections::HashMap::new();
        for id in &chosen {
            if let Some(snapshot) = snapshot(*id)? {
                snapshots.insert(*id, snapshot);
            }
        }
        let eligible = chosen
            .iter()
            .copied()
            .filter(|id| game.can_be_sacrificed_with_cause(*id, &ctx.cause))
            .collect();
        let source_snapshot = snapshot(ctx.source)?.or_else(|| {
            ctx.source_snapshot
                .as_ref()
                .filter(|snapshot| snapshot.object_id == ctx.source)
                .cloned()
        });
        if let Some(error) = game.token_resource_failure() {
            return Err(error);
        }
        Ok(Self {
            chosen,
            eligible,
            source_snapshot,
            draws: super::ZoneInstructionDraws {
                snapshots,
                ..Default::default()
            },
            originals_prepared: false,
        })
    }
    fn prepare_original(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<(), ExecutionError> {
        if self.originals_prepared {
            return Ok(());
        }
        let additional = ctx.additional_replacement_effects_snapshot();
        for id in self
            .chosen
            .iter()
            .copied()
            .filter(|id| self.eligible.contains(id))
        {
            super::prepare_zone_change_with_context_and_draws(
                game,
                id,
                Zone::Battlefield,
                Zone::Graveyard,
                ctx.cause.clone(),
                ctx,
                &additional,
                &mut self.draws,
            )?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(());
            }
        }
        self.originals_prepared = true;
        Ok(())
    }
}

fn sacrifice_selected_objects_with_outputs(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    event_object_tags: &[TagKey],
    event_source_tags: &[TagKey],
    to_sacrifice: Vec<ObjectId>,
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    sacrifice_selected_objects_with_original_outputs(
        game,
        ctx,
        event_object_tags,
        event_source_tags,
        to_sacrifice,
        |_game, _ctx, original| Ok(original),
    )
}

/// A follow-up to the original sacrifice composes its actual outputs before
/// deferred zone replacement additions. No child is replayed to recover data.
pub(crate) fn sacrifice_selected_objects_with_original_outputs<'a>(
    game: &mut GameState,
    ctx: &mut ExecutionContext<'a>,
    event_object_tags: &[TagKey],
    event_source_tags: &[TagKey],
    to_sacrifice: Vec<ObjectId>,
    after_original: impl FnOnce(
        &mut GameState,
        &mut ExecutionContext<'a>,
        crate::effects::CompletedEffectOutputs,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError>,
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    crate::effects::tokens::execute_resource_transaction_with_pending_value(
        game,
        ctx,
        || crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
        |game, ctx| {
            let mut prepared = PreparedSacrifices::capture(game, ctx, to_sacrifice)?;
            prepared.prepare_original(game, ctx)?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                    EffectOutcome::count(0),
                ));
            }
            let mut committed =
                commit_sacrifices(game, ctx, event_object_tags, event_source_tags, prepared)?
                    .into_retained();
            if ctx.decision_maker.awaiting_choice() {
                return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                    EffectOutcome::count(0),
                ));
            }
            committed.outcome = after_original(game, ctx, committed.outcome)?;
            crate::effects::composition::complete_standalone_original_with_outputs(
                game, ctx, committed,
            )
        },
    )
}

/// Commit only retained originals. Every participant's original completes
/// before ForPlayers freezes arrivals and executes replacement-added programs.
fn commit_sacrifices(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    event_object_tags: &[TagKey],
    event_source_tags: &[TagKey],
    prepared: PreparedSacrifices,
) -> Result<crate::effects::SimultaneousEffectCommit, ExecutionError> {
    if !prepared.originals_prepared {
        return Err(ExecutionError::InternalError(
            "sacrifice originals were not prepared".into(),
        ));
    }
    let PreparedSacrifices {
        chosen: chosen_to_sacrifice,
        eligible,
        source_snapshot: original_source_snapshot,
        mut draws,
        ..
    } = prepared;
    let pending_start = game.effect_store.pending_trigger_events.len();
    let original_snapshots = draws.snapshots.clone();
    let chosen_memory = chosen_to_sacrifice
        .iter()
        .filter_map(|id| original_snapshots.get(id).map(Clone::clone))
        .collect();
    let mut receipts = Vec::new();
    let mut sacrificed_count = 0;
    let mut sacrificed_objects = Vec::new();
    let mut sacrificed_memory = Vec::new();
    let mut original_sacrifice_memory = Vec::new();
    let mut sacrifice_events = Vec::new();
    let (batch_lookback, pinned_lookback) =
        begin_sacrifice_batch_lookback(game, chosen_to_sacrifice.len());

    let original = (|| -> Result<EffectOutcome, ExecutionError> {
        for id in chosen_to_sacrifice.iter().copied() {
            if !eligible.contains(&id) {
                continue;
            }
            let pre_snapshot = original_snapshots.get(&id).cloned();
            let source_snapshot_for_event = if event_source_tags.is_empty() {
                None
            } else if pre_snapshot
                .as_ref()
                .is_some_and(|snapshot| snapshot.object_id == ctx.source)
            {
                pre_snapshot.clone()
            } else {
                original_source_snapshot.clone()
            };
            let sacrificing_player = pre_snapshot.as_ref().map(|snapshot| snapshot.controller);
            let additional_effects = ctx.additional_replacement_effects_snapshot();

            // Process each sacrifice through replacement effects with decision maker
            let result = super::apply_zone_change_with_context_and_draws(
                game,
                id,
                Zone::Battlefield,
                Zone::Graveyard,
                ctx.cause.clone(),
                ctx,
                &additional_effects,
                &mut draws,
            )?;

            if ctx.decision_maker.awaiting_choice() {
                return Ok(EffectOutcome::count(0));
            }
            let verdict = result.original.clone();
            receipts.push((id, result));
            match verdict {
                EventOutcome::Prevented => {
                    // Sacrifice was prevented (unusual but possible)
                    continue;
                }
                EventOutcome::Proceed(result) => {
                    if result.final_zone == Zone::Battlefield
                        || (result.new_object_id.is_none() && result.new_object_ids.is_empty())
                    {
                        continue;
                    }
                    if let Some(snapshot) = pre_snapshot.as_ref() {
                        original_sacrifice_memory.push(Clone::clone(snapshot));
                    }
                    tag_sacrifice_zone_change_event(
                        game,
                        id,
                        event_object_tags,
                        event_source_tags,
                        pre_snapshot.as_ref(),
                        source_snapshot_for_event.as_ref(),
                    );
                    if let Some(snapshot) = pre_snapshot.clone() {
                        ctx.refresh_target_snapshot(snapshot);
                    }
                    if let Some(snapshot) = pre_snapshot.clone()
                        && snapshot.object_id == ctx.source
                    {
                        ctx.refresh_source_snapshot(snapshot);
                    }
                    sacrificed_count += 1;
                    let _ = result;
                    sacrificed_objects.push(id);
                    if let Some(snapshot) = pre_snapshot.as_ref() {
                        sacrificed_memory.push(Clone::clone(snapshot));
                    }
                    let occurrence = game.alloc_child_event_provenance(
                        ctx.provenance,
                        crate::events::EventKind::Sacrifice,
                    );
                    sacrifice_events.push(TriggerEvent::new_with_provenance(
                        SacrificeEvent::new(id, Some(ctx.source))
                            .with_snapshot(pre_snapshot, sacrificing_player),
                        occurrence,
                    ));
                }
                EventOutcome::Replaced => {
                    // The modified cost can be paid (CR 118.11), but a wholly
                    // replaced instruction did not sacrifice its selected
                    // permanent. Its added actions retain their own observations
                    // in the retained zone-instruction completion.
                    continue;
                }
                EventOutcome::NotApplicable => {
                    // Object no longer exists or isn't applicable
                    continue;
                }
            }
        }
        let sacrifice_events = with_sacrifice_batch_lookback(sacrifice_events, batch_lookback);

        let mut outcome = EffectOutcome::count(sacrificed_count)
            .with_events(sacrifice_events)
            .with_execution_fact(ExecutionFact::ChosenObjects(chosen_to_sacrifice))
            .with_chosen_object_memory(chosen_memory);
        outcome = outcome.with_execution_fact(ExecutionFact::AffectedObjects(sacrificed_objects));
        outcome = outcome.with_affected_object_memory(sacrificed_memory);
        Ok(outcome)
    })();
    end_sacrifice_batch_lookback(game, pinned_lookback);
    let original = original?;
    if ctx.decision_maker.awaiting_choice() {
        return Ok(crate::effects::SimultaneousEffectCommit::finished(
            EffectOutcome::count(0),
        ));
    }
    super::group_zone_move_observations(
        game,
        ctx,
        pending_start,
        &receipts,
        &original_snapshots,
        Zone::Battlefield,
        Zone::Graveyard,
    );
    Ok(draws.finish(
        original.with_execution_fact(ExecutionFact::OriginalSacrificeObjects(
            original_sacrifice_memory,
        )),
        receipts,
        ctx,
    ))
}

/// Share payment subject eligibility across sacrifice adapters. The actual
/// cost query still validates the whole collection, amount and bindings.
fn sacrifice_cost_choice_candidate_is_eligible(
    filter: &ObjectFilter,
    player: &PlayerFilter,
    game: &GameState,
    execution: &ExecutionContext,
    reason: crate::costs::PaymentReason,
    tag: &crate::tag::TagKey,
    object: ObjectId,
) -> Option<bool> {
    if player != &PlayerFilter::You || !crate::game_loop::tagged_filter_matches(filter, tag) {
        return None;
    }
    let mut filter = filter.clone();
    filter.tagged_constraints.retain(|constraint| {
        !(constraint.tag == *tag
            && constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject)
    });
    Some(game.object(object).is_some_and(|subject| {
        game.controller_of(subject) == execution.controller
            && filter.matches(subject, &execution.filter_context(game), game)
            && game.can_be_sacrificed_with_cause(object, &execution.cause)
            && !(reason.is_cast_or_ability_payment()
                && game.player_cant_sacrifice_nonland_to_cast_or_activate(execution.controller)
                && !game.current_has_card_type(object, crate::types::CardType::Land))
    }))
}

impl CostExecutableEffect for SacrificeEffect {
    fn cost_choice_candidate_is_eligible(
        &self,
        game: &GameState,
        execution: &mut ExecutionContext,
        reason: crate::costs::PaymentReason,
        tag: &crate::tag::TagKey,
        object: ObjectId,
    ) -> Option<bool> {
        sacrifice_cost_choice_candidate_is_eligible(
            &self.filter,
            &self.player,
            game,
            execution,
            reason,
            tag,
            object,
        )
    }

    fn finalize_payment_bindings(
        &self,
        game: &GameState,
        outcome: &EffectOutcome,
        execution: &mut ExecutionContext,
        _payment_x: Option<u32>,
    ) -> Result<(), crate::cost::CostPaymentError> {
        retain_sacrifice_payment_bindings(game, outcome, execution);
        Ok(())
    }

    fn can_execute_as_cost_with_reason(
        &self,
        game: &GameState,
        source: crate::ids::ObjectId,
        controller: crate::ids::PlayerId,
        reason: crate::costs::PaymentReason,
    ) -> Result<(), crate::effects::CostValidationError> {
        use crate::effects::CostValidationError;

        if reason.is_cast_or_ability_payment()
            && game.player_cant_sacrifice_nonland_to_cast_or_activate(controller)
        {
            let filter = self.filter.clone().with_type(crate::types::CardType::Land);
            let required = match self.count {
                crate::effect::Value::Fixed(count) => count.max(0) as usize,
                _ => 1,
            };
            let filter_ctx = crate::filter::FilterContext::new(controller).with_source(source);
            let available_land_targets = game
                .battlefield
                .iter()
                .filter_map(|&id| game.object(id).map(|obj| (id, obj)))
                .filter(|(id, obj)| {
                    game.controller_of(obj) == controller
                        && filter.matches(obj, &filter_ctx, game)
                        && game.can_be_sacrificed(*id)
                })
                .count();
            if available_land_targets < required {
                return Err(CostValidationError::CannotSacrifice);
            }
        }

        crate::effects::CostExecutableEffect::can_execute_as_cost(self, game, source, controller)
    }

    fn can_execute_as_cost(
        &self,
        game: &GameState,
        source: crate::ids::ObjectId,
        controller: crate::ids::PlayerId,
    ) -> Result<(), crate::effects::CostValidationError> {
        if self.player != PlayerFilter::You {
            return Err(crate::effects::CostValidationError::Other(
                "sacrifice costs support only 'you'".to_string(),
            ));
        }
        let count = match self.count {
            crate::effect::Value::Fixed(count) => count.max(0) as usize,
            crate::effect::Value::Count(ref count_filter)
                if dynamic_count_tracks_tagged_selection(count_filter, &self.filter) =>
            {
                return Ok(());
            }
            _ => {
                return Err(crate::effects::CostValidationError::Other(
                    "dynamic sacrifice cost amount is unsupported".to_string(),
                ));
            }
        };
        if count == 0 {
            return Ok(());
        }

        let filter_ctx = crate::filter::FilterContext::new(controller).with_source(source);
        let available = game
            .battlefield
            .iter()
            .filter_map(|&id| game.object(id).map(|obj| (id, obj)))
            .filter(|(id, obj)| {
                game.controller_of(obj) == controller
                    && self.filter.matches(obj, &filter_ctx, game)
                    && game.can_be_sacrificed(*id)
            })
            .count();
        if available < count {
            return Err(crate::effects::CostValidationError::CannotSacrifice);
        }
        Ok(())
    }
}

/// Effect that makes each player sacrifice permanents simultaneously.
///
/// Players choose in turn order starting with the active player, then the chosen
/// permanents are sacrificed after all choices are locked in.
#[derive(Debug, Clone, PartialEq)]
pub struct EachPlayerSacrificesEffect {
    /// Which permanents can be sacrificed.
    pub filter: ObjectFilter,
    /// How many permanents each player sacrifices.
    pub count: Value,
    /// Which players are included.
    pub player_filter: PlayerFilter,
}

impl EachPlayerSacrificesEffect {
    pub fn new(filter: ObjectFilter, count: impl Into<Value>, player_filter: PlayerFilter) -> Self {
        Self {
            filter,
            count: count.into(),
            player_filter,
        }
    }
}

impl EffectExecutor for EachPlayerSacrificesEffect {
    fn result_action(&self) -> Option<crate::effect::PriorEffectAction> {
        Some(crate::effect::PriorEffectAction::Sacrificed)
    }
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn decision_related_object_specs(&self) -> Vec<ChooseSpec> {
        vec![ChooseSpec::All(self.filter.clone())]
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
        game.refresh_continuous_state()
            .map_err(ExecutionError::ContinuousDiscovery)?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }
        crate::effects::composition::execute_transaction(
            game,
            ctx,
            || crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            |game, ctx| {
                let count = resolve_value(game, &self.count, ctx)?.max(0) as usize;
                if count == 0 {
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }

                let filter_ctx = ctx.filter_context(game);
                let players: Vec<PlayerId> = players_in_turn_order(game)
                    .into_iter()
                    .filter(|player_id| self.player_filter.matches_player(*player_id, &filter_ctx))
                    .collect();
                if players.is_empty() {
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }

                let mut all_chosen = Vec::new();
                for player_id in players {
                    let chosen = ctx.with_temp_iterated_player(Some(player_id), |ctx| {
                        choose_objects_to_sacrifice(game, ctx, player_id, &self.filter, count)
                    })?;
                    if ctx.decision_maker.awaiting_choice() {
                        return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                            EffectOutcome::count(0),
                        ));
                    }
                    all_chosen.extend(chosen.iter().copied());
                }

                sacrifice_selected_objects_with_outputs(game, ctx, &[], &[], all_chosen)
            },
        )
    }
}

/// Effect that sacrifices a specific target (e.g., the source permanent).
///
/// Unlike `SacrificeEffect` which uses filters, this effect sacrifices a specific
/// object identified by a `ChooseSpec`. Commonly used for source-sacrifice costs.
///
/// # Example
///
/// ```ignore
/// // Sacrifice the source permanent
/// let effect = SacrificeTargetEffect::source();
/// ```
pub type SacrificeTargetEffect = ironsmith_core::SacrificeTargetEffect;

impl EffectExecutor for SacrificeTargetEffect {
    fn result_action(&self) -> Option<crate::effect::PriorEffectAction> {
        Some(crate::effect::PriorEffectAction::Sacrificed)
    }
    fn as_cost_executable(&self) -> Option<&dyn CostExecutableEffect> {
        Some(self)
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
        game.refresh_continuous_state()
            .map_err(ExecutionError::ContinuousDiscovery)?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }
        crate::effects::composition::execute_transaction(
            game,
            ctx,
            || crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            |game, ctx| {
                // Resolve through ChooseSpec helpers (targets, source, tagged, specific object, etc.).
                let object_id = match resolve_single_object_for_effect(game, ctx, &self.target) {
                    Ok(id) => id,
                    Err(ExecutionError::InvalidTarget) => {
                        return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                            EffectOutcome::count(0),
                        ));
                    }
                    Err(err) => return Err(err),
                };

                // CR 701.21a: a player can't sacrifice a permanent they don't control.
                if let Some(player) = &self.player {
                    let sacrificing_player = resolve_player_filter(game, player, ctx)?;
                    if ctx.decision_maker.awaiting_choice() {
                        return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                            EffectOutcome::count(0),
                        ));
                    }
                    if game
                        .object(object_id)
                        .is_some_and(|object| game.controller_of(object) != sacrificing_player)
                    {
                        return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                            EffectOutcome::count(0),
                        ));
                    }
                }
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }
                sacrifice_selected_objects_with_outputs(game, ctx, &[], &[], vec![object_id])
            },
        )
    }

    // "Target creature's controller sacrifices it": the targeted object is
    // announced as the spell's target (CR 601.2c) like any other targeted
    // instruction, so the tag bound by the sacrifice exists at resolution.
    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        self.target.is_target().then_some(&self.target)
    }

    fn get_target_count(&self) -> Option<crate::effect::ChoiceCount> {
        self.target.is_target().then(|| self.target.count())
    }

    fn target_description(&self) -> &'static str {
        "permanent to sacrifice"
    }

    fn is_sacrifice_source_cost(&self) -> bool {
        matches!(self.target.unhinted(), ChooseSpec::Source)
    }

    fn cost_description(&self) -> Option<String> {
        matches!(self.target.unhinted(), ChooseSpec::Source).then(|| {
            let subject = self
                .target
                .source_reference_surface()
                .map(crate::target::SourceReferenceSurface::display_text)
                .unwrap_or_else(|| "this source".to_string());
            format!("Sacrifice {subject}")
        })
    }
}

impl CostExecutableEffect for SacrificeTargetEffect {
    fn finalize_payment_bindings(
        &self,
        game: &GameState,
        outcome: &EffectOutcome,
        execution: &mut ExecutionContext,
        _payment_x: Option<u32>,
    ) -> Result<(), crate::cost::CostPaymentError> {
        retain_sacrifice_payment_bindings(game, outcome, execution);
        Ok(())
    }

    fn can_execute_as_cost_with_reason(
        &self,
        game: &GameState,
        source: crate::ids::ObjectId,
        controller: crate::ids::PlayerId,
        reason: crate::costs::PaymentReason,
    ) -> Result<(), crate::effects::CostValidationError> {
        use crate::effects::CostValidationError;

        if reason.is_cast_or_ability_payment()
            && game.player_cant_sacrifice_nonland_to_cast_or_activate(controller)
            && !game
                .calculated_characteristics(source)
                .is_some_and(|chars| chars.card_types.contains(&crate::types::CardType::Land))
        {
            return Err(CostValidationError::CannotSacrifice);
        }

        crate::effects::CostExecutableEffect::can_execute_as_cost(self, game, source, controller)
    }

    fn can_execute_as_cost(
        &self,
        game: &GameState,
        source: crate::ids::ObjectId,
        _controller: crate::ids::PlayerId,
    ) -> Result<(), crate::effects::CostValidationError> {
        if !matches!(self.target, ChooseSpec::Source) {
            return Err(crate::effects::CostValidationError::Other(
                "sacrifice-target costs support only source".to_string(),
            ));
        }
        if !game.battlefield.contains(&source) || !game.can_be_sacrificed(source) {
            return Err(crate::effects::CostValidationError::CannotSacrifice);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::Ability;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::cards::CardDefinitionBuilder;
    #[cfg(ironsmith_runtime_parser_tests)]
    use crate::cards::definitions::basic_mountain;
    #[cfg(ironsmith_runtime_parser_tests)]
    use crate::effect::Effect;
    use crate::effect::ExecutionFact;
    use crate::effect::Restriction;
    #[cfg(ironsmith_runtime_parser_tests)]
    use crate::effects::CostExecutableEffect;
    #[cfg(ironsmith_runtime_parser_tests)]
    use crate::effects::EarthbendEffect;
    use crate::effects::ExecutionContext;
    #[cfg(ironsmith_runtime_parser_tests)]
    use crate::effects::execute_effect;
    use crate::ids::{CardId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::Object;
    use crate::static_abilities::StaticAbility;
    use crate::target::ChooseSpec;
    use crate::types::CardType;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    #[test]
    fn cause_filtered_sacrifice_protection_distinguishes_effects_and_payments() {
        use crate::events::cause::{
            CauseFilter, CauseType, CauseTypeFilter, ControllerFilter, EventCause,
        };
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        for (cause_controller, cause_type, protected) in [
            (bob, CauseType::Effect, true),
            (bob, CauseType::Cost, true),
            (alice, CauseType::Effect, false),
            (alice, CauseType::Cost, false),
            (bob, CauseType::GameRule, false),
        ] {
            for explicit in [false, true] {
                let mut game = setup_game();
                let definition = CardDefinitionBuilder::new(CardId::new(), "Sacrifice Protector")
                    .card_types(vec![CardType::Enchantment])
                    .with_ability(Ability::static_ability(StaticAbility::restriction(
                        Restriction::BeSacrificedByCause {
                            filter: ObjectFilter::creature().controlled_by(PlayerFilter::You),
                            cause: CauseFilter {
                                cause_type: Some(CauseTypeFilter::OneOf(vec![
                                    CauseType::Effect,
                                    CauseType::Cost,
                                ])),
                                source_filter: None,
                                controller_filter: Some(ControllerFilter::Opponent),
                            },
                        },
                        String::new(),
                    )))
                    .build();
                let protector =
                    game.create_object_from_definition(&definition, alice, Zone::Battlefield);
                let creature = create_creature_on_battlefield(&mut game, "Bear", alice);
                game.update_cant_effects();
                let cause = EventCause {
                    resolving_spell: None,
                    cause_type,
                    source: Some(protector),
                    source_controller: Some(cause_controller),
                };
                let mut ctx = ExecutionContext::new_default(protector, alice).with_cause(cause);
                if explicit {
                    ctx.targets
                        .push(crate::effects::ResolvedTarget::Object(creature));
                }
                SacrificeEffect::you(ObjectFilter::creature(), 1)
                    .execute(&mut game, &mut ctx)
                    .unwrap();
                assert_eq!(
                    game.battlefield.contains(&creature),
                    protected,
                    "{cause_controller:?} {cause_type:?} explicit={explicit}"
                );
            }
        }
    }

    #[test]
    fn cause_filtered_protection_survives_simultaneous_selection_and_expires_with_source() {
        use crate::events::cause::{
            CauseFilter, CauseType, CauseTypeFilter, ControllerFilter, EventCause,
        };
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let definition = CardDefinitionBuilder::new(CardId::new(), "Sacrifice Protector")
            .card_types(vec![CardType::Enchantment])
            .with_ability(Ability::static_ability(StaticAbility::restriction(
                Restriction::BeSacrificedByCause {
                    filter: ObjectFilter::creature().controlled_by(PlayerFilter::You),
                    cause: CauseFilter {
                        cause_type: Some(CauseTypeFilter::OneOf(vec![
                            CauseType::Effect,
                            CauseType::Cost,
                        ])),
                        source_filter: None,
                        controller_filter: Some(ControllerFilter::Opponent),
                    },
                },
                String::new(),
            )))
            .build();
        let protector = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let alice_creature = create_creature_on_battlefield(&mut game, "Alice Bear", alice);
        let bob_creature = create_creature_on_battlefield(&mut game, "Bob Bear", bob);
        game.update_cant_effects();
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, bob)
            .with_cause(EventCause::from_effect(source, bob));
        EachPlayerSacrificesEffect::new(ObjectFilter::creature(), 1, PlayerFilter::Any)
            .execute(&mut game, &mut ctx)
            .unwrap();
        assert!(game.battlefield.contains(&alice_creature));
        assert!(!game.battlefield.contains(&bob_creature));
        // Losing the static ability must remove its cached restriction.
        game.object_mut(protector).unwrap().abilities_mut().clear();
        game.update_cant_effects();
        SacrificeTargetEffect::new(ChooseSpec::SpecificObject(alice_creature))
            .execute(&mut game, &mut ctx)
            .unwrap();
        assert!(!game.battlefield.contains(&alice_creature));
    }

    #[test]
    fn named_source_sacrifice_cost_keeps_exact_source_surface() {
        let target = ChooseSpec::Source.with_surface_hint(
            crate::target::ChooseSpecSurfaceHint::SourceReference(
                crate::target::SourceReferenceSurface::ShortName("ED-E".to_string()),
            ),
        );
        let sacrifice = SacrificeTargetEffect::new(target);

        assert_eq!(
            sacrifice.cost_description().as_deref(),
            Some("Sacrifice ED-E")
        );
        assert!(sacrifice.is_sacrifice_source_cost());
    }

    #[test]
    fn filtered_sacrifice_cost_omits_redundant_controller_scope() {
        let sacrifice = SacrificeEffect::you(
            ObjectFilter::default()
                .with_type(CardType::Land)
                .controlled_by(PlayerFilter::You),
            1,
        );

        assert_eq!(
            sacrifice.cost_description().as_deref(),
            Some("Sacrifice a land")
        );
    }

    fn create_creature_on_battlefield(
        game: &mut GameState,
        name: &str,
        controller: PlayerId,
    ) -> ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Green],
            ]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let object = Object::from_card(id, &card, controller, Zone::Battlefield);
        game.add_object(object);
        id
    }

    fn create_indestructible_creature_on_battlefield(
        game: &mut GameState,
        name: &str,
        controller: PlayerId,
    ) -> ObjectId {
        let definition = CardDefinitionBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .with_ability(crate::ability::indestructible())
            .build();
        game.create_object_from_definition(&definition, controller, Zone::Battlefield)
    }

    #[test]
    fn test_sacrifice_target_tagged_without_ctx_targets() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let target_id = create_creature_on_battlefield(&mut game, "Bear", alice);
        let snapshot = ObjectSnapshot::from_object(game.object(target_id).unwrap(), &game);

        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.tag_object("sac_target", snapshot);

        let effect = SacrificeTargetEffect::new(ChooseSpec::Tagged("sac_target".into()));
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(1));
        assert!(!game.battlefield.contains(&target_id));
        assert_eq!(game.players[0].graveyard.len(), 1);
        assert!(
            result
                .execution_facts()
                .contains(&ExecutionFact::ChosenObjects(vec![target_id]))
        );
        assert!(
            result
                .execution_facts()
                .contains(&ExecutionFact::AffectedObjects(vec![target_id]))
        );
    }

    #[test]
    fn sacrifice_event_tags_mark_zone_change_with_object_and_source() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source_id = create_creature_on_battlefield(&mut game, "Exploiter", alice);
        let victim_id = create_creature_on_battlefield(&mut game, "Victim", alice);

        let effect = SacrificeEffect::you(ObjectFilter::creature().other(), 1)
            .with_event_object_tag(crate::tag::EXPLOITED_TAG)
            .with_event_source_tag(crate::tag::EXPLOITER_TAG);
        let mut ctx = ExecutionContext::new_default(source_id, alice);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(1));
        let events = game.take_pending_trigger_events();
        let zone_change = events
            .iter()
            .find_map(|event| event.downcast::<crate::events::ZoneChangeEvent>())
            .expect("sacrifice should queue a zone-change event");
        let exploited = zone_change
            .object_tags
            .get(crate::tag::EXPLOITED_TAG)
            .expect("exploited object tag should be attached to the zone change");
        assert!(
            exploited
                .iter()
                .any(|snapshot| snapshot.object_id == victim_id)
        );
        let exploiters = zone_change
            .object_tags
            .get(crate::tag::EXPLOITER_TAG)
            .expect("exploiter source tag should be attached to the zone change");
        assert!(
            exploiters
                .iter()
                .any(|snapshot| snapshot.object_id == source_id)
        );
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn test_creature_sacrifice_cost_accepts_earthbent_land() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source_id = create_creature_on_battlefield(&mut game, "Kyoshi", alice);
        let land_id =
            game.create_object_from_definition(&basic_mountain(), alice, Zone::Battlefield);

        let effect = Effect::new(EarthbendEffect::new(ChooseSpec::SpecificObject(land_id), 8));
        let mut ctx = ExecutionContext::new_default(source_id, alice);
        execute_effect(&mut game, &effect, &mut ctx).expect("earthbend should resolve");

        let sacrifice_cost = SacrificeEffect::you_creature(1);
        assert_eq!(
            CostExecutableEffect::can_execute_as_cost(&sacrifice_cost, &game, source_id, alice),
            Ok(()),
            "animated lands should satisfy creature sacrifice costs"
        );
    }

    #[test]
    fn sacrifice_ignores_indestructible() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let creature_id =
            create_indestructible_creature_on_battlefield(&mut game, "Darksteel Test", alice);

        let mut ctx = ExecutionContext::new_default(source, alice);
        let result = SacrificeEffect::you_creature(1)
            .execute(&mut game, &mut ctx)
            .expect("sacrifice should resolve");

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(1));
        assert!(!game.battlefield.contains(&creature_id));
        assert_eq!(game.players[0].graveyard.len(), 1);
        let graveyard_object = game
            .player(alice)
            .and_then(|player| player.graveyard.first().copied())
            .and_then(|id| game.object(id));
        assert_eq!(
            graveyard_object.map(|object| object.name.as_str()),
            Some("Darksteel Test")
        );
    }

    #[test]
    fn sacrifice_moves_controlled_permanent_to_owners_graveyard() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();
        let creature_id = create_creature_on_battlefield(&mut game, "Borrowed Bear", alice);
        game.set_current_controller(creature_id, bob).expect("finite controller fixture must refresh successfully");

        let mut ctx = ExecutionContext::new_default(source, bob);
        let result = SacrificeEffect::player(ObjectFilter::creature(), 1, PlayerFilter::You)
            .execute(&mut game, &mut ctx)
            .expect("sacrifice should resolve");

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(1));
        assert!(!game.battlefield.contains(&creature_id));
        assert_eq!(game.players[0].graveyard.len(), 1);
        assert_eq!(game.players[1].graveyard.len(), 0);
        let graveyard_object = game
            .player(alice)
            .and_then(|player| player.graveyard.first().copied())
            .and_then(|id| game.object(id));
        assert_eq!(
            graveyard_object.map(|object| (
                object.name.as_str(),
                object.owner,
                game.controller_of(object)
            )),
            Some(("Borrowed Bear", alice, alice)),
            "sacrificed permanents should go to their owner's graveyard"
        );
    }

    #[test]
    fn each_player_sacrifices_locks_choices_before_any_permanent_leaves() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        game.turn.active_player = alice;

        let restrictor = CardDefinitionBuilder::new(CardId::new(), "Sacrifice Lock")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .with_ability(Ability::static_ability(StaticAbility::restriction(
                Restriction::be_sacrificed(
                    ObjectFilter::creature().controlled_by(PlayerFilter::Opponent),
                ),
                "Creatures your opponents control can't be sacrificed".to_string(),
            )))
            .build();
        let bob_creature = CardDefinitionBuilder::new(CardId::new(), "Bob Bear")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();

        let restrictor_id =
            game.create_object_from_definition(&restrictor, alice, Zone::Battlefield);
        let bob_creature_id =
            game.create_object_from_definition(&bob_creature, bob, Zone::Battlefield);
        game.update_cant_effects();
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let result =
            EachPlayerSacrificesEffect::new(ObjectFilter::creature(), 1, PlayerFilter::Any)
                .execute(&mut game, &mut ctx)
                .expect("each-player sacrifice should resolve");

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(1));
        assert!(
            !game.battlefield.contains(&restrictor_id),
            "the active player's chosen creature should be sacrificed"
        );
        assert!(
            game.battlefield.contains(&bob_creature_id),
            "the nonactive player should not gain a new sacrifice option after the first sacrifice happens"
        );
        assert_eq!(game.players[0].graveyard.len(), 1);
        assert_eq!(game.players[1].graveyard.len(), 0);
    }
}

#[cfg(test)]
mod replacement_sacrifice_owner_contract_tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::decision::DecisionMaker;
    use crate::effect::Effect;
    use crate::ids::CardId;
    use crate::object::CounterType;
    use crate::replacement::{ReplacementAction, ReplacementEffect};
    use crate::types::CardType;
    struct Answers { originals: Vec<ObjectId>, stable: crate::ids::StableId, pause: bool, pending: bool, calls: usize, binding: bool }
    impl DecisionMaker for Answers {
        fn decide_boolean(&mut self, game: &GameState, _: &crate::decisions::context::BooleanContext) -> bool {
            self.calls += 1;
            for id in &self.originals { assert!(game.object(*id).is_none(), "all original sacrifices precede additions"); }
            if self.binding { let arrival = game.objects_in_deterministic_order().into_iter().find(|object| object.stable_id == self.stable).unwrap(); assert_eq!(arrival.zone, Zone::Graveyard); assert_eq!(game.counter_count(arrival.id, CounterType::PlusOnePlusOne), 1); }
            self.pending = self.pause; !self.pending
        }
        fn awaiting_choice(&self) -> bool { self.pending }
    }
    fn card(game: &mut GameState, owner: PlayerId, creature: bool) -> ObjectId {
        game.create_object_from_card(&CardBuilder::new(CardId::new(), "Sacrifice owner fixture")
            .card_types(vec![if creature { CardType::Creature } else { CardType::Artifact }]).build(), owner, Zone::Battlefield)
    }
    fn check(path: u8, mode: u8) {
        let mut game = crate::tests::test_helpers::setup_two_player_game(); let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1);
        let source = card(&mut game, alice, false); let replacement_source = card(&mut game, bob, false); let sentinel = card(&mut game, alice, false);
        let first = card(&mut game, alice, true); let stable = game.object(first).unwrap().stable_id;
        let mut victims = vec![first]; if path != 2 { victims.push(card(&mut game, if path == 1 { bob } else { alice }, true)); }
        let snapshots = victims.iter().map(|id| ObjectSnapshot::from_object(game.object(*id).unwrap(), &game)).collect::<Vec<_>>();
        let sentinel_snapshot = ObjectSnapshot::from_object(game.object(sentinel).unwrap(), &game);
        let actions = match mode { 1 => vec![Effect::gain_life(3), Effect::lose_life(Value::X)],
            3 => vec![Effect::new(crate::effects::PutCountersEffect::new(CounterType::PlusOnePlusOne, 1, ChooseSpec::tagged("it"))), Effect::may(vec![Effect::gain_life(0)])],
            _ => vec![Effect::gain_life(3), Effect::may(vec![Effect::gain_life(4)])] };
        let shield = game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(replacement_source, bob,
            crate::events::zones::matchers::WouldChangeZoneMatcher::new(ObjectFilter::specific(first), Some(Zone::Battlefield), Some(Zone::Graveyard)), ReplacementAction::Additionally(actions)));
        let effect = match path { 0 => Effect::new(SacrificeEffect::you(ObjectFilter::tagged("victims"), 2)),
            1 => Effect::new(EachPlayerSacrificesEffect::new(ObjectFilter::tagged("victims"), 1, PlayerFilter::Any)),
            _ => Effect::new(SacrificeTargetEffect::new(ChooseSpec::SpecificObject(first))) };
        game.take_pending_trigger_events(); let ids = game.next_object_id_counter(); let objects = game.objects_in_deterministic_order().len();
        let mut dm = Answers { originals: victims.clone(), stable, pause: mode == 2, pending: false, calls: 0, binding: mode == 3 };
        let mut ctx = ExecutionContext::new(source, alice, &mut dm); ctx.set_tagged_objects("victims", snapshots.clone()); ctx.set_tagged_objects("it", vec![sentinel_snapshot.clone()]);
        let result = crate::effects::execute_effect(&mut game, &effect, &mut ctx);
        if mode == 1 { assert!(matches!(result, Err(ExecutionError::UnresolvableValue(_)))); }
        else if mode == 2 { assert!(ctx.decision_maker.awaiting_choice()); assert!(result.unwrap().events.is_empty()); }
        else {
            let outcome = result.unwrap(); assert_eq!(outcome.count_or_zero(), victims.len() as i64);
            for id in &victims { assert!(game.object(*id).is_none()); }
            assert_eq!(outcome.events.iter().filter(|event| event.downcast::<SacrificeEvent>().is_some()).count(), victims.len());
            assert_eq!(game.player(alice).unwrap().life, 20); assert_eq!(game.player(bob).unwrap().life, if mode == 3 { 20 } else { 27 });
            if mode == 3 { let arrival = game.objects_in_deterministic_order().into_iter().find(|object| object.stable_id == stable).unwrap(); assert_eq!(arrival.zone, Zone::Graveyard); assert_eq!(game.counter_count(arrival.id, CounterType::PlusOnePlusOne), 1); }
            else { assert_eq!(outcome.events.iter().filter_map(|event| event.downcast::<crate::events::LifeGainEvent>()).map(|event| (event.player, event.amount)).collect::<Vec<_>>(), vec![(bob, 3), (bob, 4)]); }
            assert!(game.effect_store.replacement_effects.get_effect(shield).is_none());
        }
        assert_eq!(ctx.source, source); assert_eq!(ctx.controller, alice); assert!(ctx.targets.is_empty()); assert_eq!(ctx.get_tagged_all("it").unwrap()[0].object_id, sentinel_snapshot.object_id);
        assert_eq!(game.counter_count(sentinel, CounterType::PlusOnePlusOne), 0);
        if mode == 1 || mode == 2 {
            for id in &victims { assert_eq!(game.object(*id).unwrap().zone, Zone::Battlefield); }
            assert!(game.player(alice).unwrap().graveyard.is_empty()); assert!(game.player(bob).unwrap().graveyard.is_empty()); assert_eq!(game.player(bob).unwrap().life, 20);
            assert_eq!(game.next_object_id_counter(), ids); assert_eq!(game.objects_in_deterministic_order().len(), objects); assert!(game.effect_store.replacement_effects.get_effect(shield).is_some()); assert!(game.take_pending_trigger_events().is_empty());
        }
        drop(ctx); if mode == 0 || mode == 3 { assert_eq!(dm.calls, 1); }
        if mode == 2 { assert_eq!(dm.calls, 1); dm.pause = false; dm.pending = false;
            let mut ctx = ExecutionContext::new(source, alice, &mut dm); ctx.set_tagged_objects("victims", snapshots);
            let outcome = crate::effects::execute_effect(&mut game, &effect, &mut ctx).unwrap(); assert_eq!(outcome.count_or_zero(), victims.len() as i64);
            for id in &victims { assert!(game.object(*id).is_none()); } assert_eq!(game.player(bob).unwrap().life, 27); assert!(!ctx.decision_maker.awaiting_choice()); drop(ctx); assert_eq!(dm.calls, 2);
        }
    }
    #[test] fn selected_additions_follow_all_sacrifices() { check(0, 0); }
    #[test] fn selected_error_restores_batch() { check(0, 1); }
    #[test] fn selected_pending_replays_once() { check(0, 2); }
    #[test] fn selected_addition_binds_arrival() { check(0, 3); }
    #[test] fn each_player_additions_follow_all_sacrifices() { check(1, 0); }
    #[test] fn each_player_error_restores_batch() { check(1, 1); }
    #[test] fn each_player_pending_replays_once() { check(1, 2); }
    #[test] fn each_player_addition_binds_arrival() { check(1, 3); }
    #[test] fn target_additions_follow_sacrifice() { check(2, 0); }
    #[test] fn target_error_restores_sacrifice() { check(2, 1); }
    #[test] fn target_pending_replays_once() { check(2, 2); }
    #[test] fn target_addition_binds_arrival() { check(2, 3); }
}

#[cfg(test)]
mod original_result_quantity_tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::replacement::{ReplacementAction, ReplacementEffect};
    #[test]
    fn direct_and_dispatch_results_do_not_count_replacement_only_sacrifices() {
        for dispatched in [false, true] { for scenario in 0..4 {
            let player = PlayerId::from_index(0);
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let card = CardBuilder::new(crate::CardId::new(), "Sacrifice witness")
                .card_types(vec![crate::CardType::Creature]).power_toughness(PowerToughness::fixed(3, 4)).build();
            let source = game.create_object_from_card(&card, player, Zone::Battlefield);
            let original = game.create_object_from_card(&card, player, Zone::Battlefield);
            let added = game.create_object_from_card(&card, player, Zone::Battlefield);
            let added_sacrifice = crate::effect::Effect::new(crate::effects::SacrificeTargetEffect::new(ChooseSpec::SpecificObject(added)));
            let action = match scenario {
                0 => ReplacementAction::Prevent,
                1 => ReplacementAction::Instead(vec![added_sacrifice]),
                2 => ReplacementAction::ChangeDestination(Zone::Exile),
                _ => ReplacementAction::Additionally(vec![added_sacrifice]),
            };
            game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(source, player,
                crate::events::zones::matchers::WouldChangeZoneMatcher::new(ObjectFilter::specific(original), Some(Zone::Battlefield), Some(Zone::Graveyard)), action));
            let sacrifice = crate::effects::SacrificeTargetEffect::new(ChooseSpec::SpecificObject(original));
            let mut ctx = ExecutionContext::new_default(source, player);
            let outcome = if dispatched { crate::effects::execute_effect(&mut game, &crate::effect::Effect::new(sacrifice), &mut ctx) }
                else { sacrifice.execute(&mut game, &mut ctx) }.unwrap();
            let actual = usize::from(scenario >= 2);
            assert_eq!(outcome.instruction_result().count_or_zero(), actual as i64);
            assert_eq!(outcome.chosen_objects(), Some([original].as_slice()));
            assert_eq!(outcome.affected_object_memory().unwrap().len(), actual);
            assert!(outcome.affected_object_memory().unwrap().iter().all(|memory| memory.object_id == original));
            let all_events = outcome.events.iter().filter(|event| event.downcast::<SacrificeEvent>().is_some()).count();
            assert_eq!(all_events, actual + usize::from(scenario == 1 || scenario == 3));
        }}
    }

    #[test]
    fn simultaneous_players_share_one_shot_consumption_and_keep_actual_original_facts() {
        for dispatched in [false, true] {
            let a = PlayerId::from_index(0); let b = PlayerId::from_index(1); let c = PlayerId::from_index(2);
            let mut game = GameState::new(vec!["A".into(), "B".into(), "C".into()], 20);
            let creature = CardBuilder::new(crate::CardId::new(), "Original sacrifice")
                .card_types(vec![crate::CardType::Creature]).power_toughness(PowerToughness::fixed(2, 3)).build();
            let objects = [a, b, c].map(|player| game.create_object_from_card(&creature, player, Zone::Battlefield));
            let source = game.create_object_from_card(&CardBuilder::new(crate::CardId::new(), "Sacrifice source")
                .card_types(vec![crate::CardType::Artifact]).build(), a, Zone::Battlefield);
            let replacement = game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(source, a,
                crate::events::zones::matchers::WouldChangeZoneMatcher::new(ObjectFilter::creature(), Some(Zone::Battlefield), Some(Zone::Graveyard)),
                ReplacementAction::Prevent));
            let each = crate::effects::ForPlayersEffect::new(PlayerFilter::Any,
                vec![crate::effect::Effect::sacrifice_player(ObjectFilter::creature(), 1, PlayerFilter::IteratedPlayer)]);
            let mut ctx = ExecutionContext::new_default(source, a);
            let outcome = if dispatched {
                crate::effects::execute_effect(&mut game, &crate::effect::Effect::new(each), &mut ctx)
            } else { each.execute(&mut game, &mut ctx) }.unwrap();
            assert_eq!(game.object(objects[0]).unwrap().zone, Zone::Battlefield);
            assert!(objects[1..].iter().all(|id| game.object(*id).is_none()));
            assert!(game.effect_store.replacement_effects.get_effect(replacement).is_none());
            let actual: Vec<_> = outcome.instruction_result().execution_facts.iter().filter_map(|fact| match fact {
                ExecutionFact::OriginalSacrificeObjects(memory) => Some(memory), _ => None,
            }).flatten().map(|memory| memory.object_id).collect();
            assert_eq!(actual, objects[1..]);
            assert_eq!(outcome.events.iter().filter(|event| event.downcast::<SacrificeEvent>().is_some()).count(), 2);
        }
    }

    #[test]
    fn one_instruction_completes_all_originals_before_its_added_observation() {
        for dispatched in [false, true] {
            let a = PlayerId::from_index(0);
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let creature = CardBuilder::new(crate::CardId::new(), "Simultaneous original")
                .card_types(vec![crate::CardType::Creature]).power_toughness(PowerToughness::fixed(2, 3)).build();
            let first = game.create_object_from_card(&creature, a, Zone::Battlefield);
            game.create_object_from_card(&creature, a, Zone::Battlefield);
            let source = game.create_object_from_card(&CardBuilder::new(crate::CardId::new(), "Observation source")
                .card_types(vec![crate::CardType::Artifact]).build(), a, Zone::Battlefield);
            game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(source, a,
                crate::events::zones::matchers::WouldChangeZoneMatcher::new(ObjectFilter::specific(first), Some(Zone::Battlefield), Some(Zone::Graveyard)),
                ReplacementAction::Additionally(vec![crate::effect::Effect::gain_life(Value::Count(ObjectFilter::creature()))])));
            let sacrifice = SacrificeEffect::you_creature(2);
            let mut ctx = ExecutionContext::new_default(source, a);
            let outcome = if dispatched { crate::effects::execute_effect(&mut game, &crate::effect::Effect::new(sacrifice), &mut ctx) }
                else { sacrifice.execute(&mut game, &mut ctx) }.unwrap();
            assert_eq!(game.player(a).unwrap().life, 20);
            assert_eq!(outcome.instruction_result().count_or_zero(), 2);
            assert_eq!(outcome.affected_object_memory().unwrap().len(), 2);
        }
    }

    #[test]
    fn each_actual_original_has_one_occurrence_in_staged_committed_and_republished_history() {
        for dispatched in [false, true] {
            let player = PlayerId::from_index(0);
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let creature = CardBuilder::new(crate::CardId::new(), "History original")
                .card_types(vec![crate::CardType::Creature]).power_toughness(PowerToughness::fixed(2, 3)).build();
            for _ in 0..3 { game.create_object_from_card(&creature, player, Zone::Battlefield); }
            let source = game.create_object_from_card(&CardBuilder::new(crate::CardId::new(), "History source")
                .card_types(vec![crate::CardType::Artifact]).build(), player, Zone::Battlefield);
            let parent = game.provenance_graph_mut().alloc_root_event(crate::events::EventKind::SpellCast);
            let mut ctx = ExecutionContext::new_default(source, player).with_provenance(parent);
            let sacrifice = SacrificeEffect::you_creature(3);
            let outcome = if dispatched { crate::effects::execute_effect(&mut game, &crate::effect::Effect::new(sacrifice), &mut ctx) }
                else { sacrifice.execute(&mut game, &mut ctx) }.unwrap();
            let events: Vec<_> = outcome.events.iter().filter(|event| event.downcast::<SacrificeEvent>().is_some()).collect();
            assert_eq!(events.len(), 3);
            let occurrences: std::collections::HashSet<_> = events.iter().map(|event| event.provenance()).collect();
            assert_eq!(occurrences.len(), 3, "distinct originals cannot overwrite each other's staged history");
            let group = game.provenance_graph().node(events[0].provenance()).unwrap().parent;
            for event in &events {
                assert_eq!(game.provenance_graph().node(event.provenance()).unwrap().parent, group);
                assert!(game.provenance_graph().is_descendant_of(event.provenance(), parent));
            }
            assert_eq!(game.turn_store.turn_history.event_kind_count(crate::events::EventKind::Sacrifice), 3);
            let mut history = crate::turn_history::TurnHistory::default();
            for (index, event) in events.iter().enumerate() {
                let snapshot = event.downcast::<SacrificeEvent>().unwrap().snapshot.clone();
                history.stage_event(event, snapshot, None);
                assert_eq!(history.event_kind_count(crate::events::EventKind::Sacrifice), index as u32 + 1);
            }
            for event in &events {
                history.record_event(event, event.downcast::<SacrificeEvent>().unwrap().snapshot.clone(), None);
                assert_eq!(history.event_kind_count(crate::events::EventKind::Sacrifice), 3);
            }
            for event in &events {
                let snapshot = event.downcast::<SacrificeEvent>().unwrap().snapshot.clone();
                history.stage_event(event, snapshot.clone(), None);
                history.record_event(event, snapshot, None);
            }
            assert_eq!(history.event_kind_count(crate::events::EventKind::Sacrifice), 3);
        }
    }

    #[test]
    fn direct_and_dispatched_admission_fail_before_selection_or_snapshot_fallback() {
        struct NoSelection;
        impl crate::decision::DecisionMaker for NoSelection {
            fn decide_objects(&mut self, _: &GameState, _: &crate::decisions::context::SelectObjectsContext) -> Vec<ObjectId> {
                panic!("incomplete characteristics must fail before asking for a sacrifice");
            }
        }
        for dispatched in [false, true] { for variant in 0..3 { for invalid_source in [false, true] {
            let player = PlayerId::from_index(0);
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let creature = CardBuilder::new(crate::CardId::new(), "Checked sacrifice")
                .card_types(vec![crate::CardType::Creature]).power_toughness(PowerToughness::fixed(2, 3)).build();
            let source = game.create_object_from_card(&creature, player, Zone::Battlefield);
            let selected = game.create_object_from_card(&creature, player, Zone::Battlefield);
            let invalid = if invalid_source { source } else { selected };
            game.object_mut(invalid).unwrap().counters.insert(crate::CounterType::PlusOnePlusOne, u32::MAX);
            let effect = match variant {
                0 => crate::effect::Effect::new(SacrificeEffect::you_creature(1)),
                1 => crate::effect::Effect::new(SacrificeTargetEffect::new(ChooseSpec::SpecificObject(selected))),
                _ => crate::effect::Effect::new(EachPlayerSacrificesEffect::new(ObjectFilter::creature(), 1, PlayerFilter::Any)),
            };
            let ids = game.next_object_id_counter(); let history = game.turn_store.turn_history.event_records.len();
            let mut dm = NoSelection; let mut ctx = ExecutionContext::new(source, player, &mut dm);
            let result = if dispatched { crate::effects::execute_effect(&mut game, &effect, &mut ctx) }
                else { effect.0.execute(&mut game, &mut ctx) };
            assert!(matches!(result, Err(ExecutionError::ContinuousDiscovery(_))), "{result:?}");
            assert_eq!(game.object(selected).unwrap().zone, Zone::Battlefield);
            assert_eq!(game.next_object_id_counter(), ids); assert_eq!(game.turn_store.turn_history.event_records.len(), history);
            assert!(ctx.tagged_objects.is_empty() && ctx.effect_outcomes.is_empty());
            assert!(!game.effect_store.has_pending_trigger_work());
            assert!(matches!(PreparedSacrifices::capture(&game, &ctx, vec![selected]), Err(ExecutionError::ContinuousDiscovery(_))));
            assert!(SacrificeEffect::you_creature(1).prepare_proposal(&game, &mut ctx).is_err());
        }}}
    }

    #[test]
    fn incomplete_discovery_is_not_an_empty_zero_sacrifice_or_proposal() {
        use crate::continuous::{ContinuousEffect, EffectTarget, Modification};
        #[derive(Debug, Clone)]
        struct UnboundedSacrificeSource(std::sync::Arc<std::sync::atomic::AtomicUsize>);
        impl crate::static_abilities::StaticAbilityKind for UnboundedSacrificeSource {
            fn id(&self) -> crate::static_abilities::StaticAbilityId { crate::static_abilities::StaticAbilityId::GrantObjectAbilityForFilter }
            fn display(&self) -> String { "Unbounded sacrifice admission fixture".into() }
            fn generate_effects(&self, source: ObjectId, controller: PlayerId, game: &GameState) -> Vec<ContinuousEffect> {
                assert!(self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst) < 32_768);
                let crate::ability::AbilityKind::Static(parent) = &game.object(source).unwrap().abilities[0].kind else { panic!("static fixture"); };
                vec![ContinuousEffect::new(source, controller, EffectTarget::Source, Modification::AddAbility(parent.clone())),
                    ContinuousEffect::new(source, controller, EffectTarget::Source, Modification::ModifyPower(1))]
            }
        }
        std::thread::Builder::new().stack_size(128 * 1024 * 1024).spawn(|| {
            for dispatched in [false, true] {
                let a = PlayerId::from_index(0);
                let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
                let source = game.create_object_from_card(&CardBuilder::new(crate::CardId::new(), "Incomplete sacrifice source")
                    .card_types(vec![crate::CardType::Creature]).power_toughness(PowerToughness::fixed(1, 3)).build(), a, Zone::Battlefield);
                game.object_mut(source).unwrap().abilities_mut().push(crate::ability::Ability::static_ability(
                    crate::static_abilities::StaticAbility::new(UnboundedSacrificeSource(Default::default()))));
                let mut ctx = ExecutionContext::new_default(source, a);
                let sacrifice = SacrificeEffect::you_creature(0);
                let result = if dispatched { crate::effects::execute_effect(&mut game, &crate::effect::Effect::new(sacrifice.clone()), &mut ctx) }
                    else { sacrifice.execute(&mut game, &mut ctx) };
                assert!(matches!(result, Err(ExecutionError::ContinuousDiscovery(crate::static_ability_processor::StaticEffectDiscoveryError::RoundLimit { .. }))));
                assert!(sacrifice.prepare_proposal(&game, &mut ctx).is_err());
                assert_eq!(game.object(source).unwrap().zone, Zone::Battlefield);
                assert!(game.turn_store.turn_history.event_records.is_empty());
                assert!(ctx.tagged_objects.is_empty() && ctx.effect_outcomes.is_empty());
            }
        }).unwrap().join().unwrap();
    }

    #[test]
    fn phased_source_uses_exact_lki_and_phased_recipient_is_known_ineligible() {
        for dispatched in [false, true] { for phased_recipient in [false, true] {
            let player = PlayerId::from_index(0);
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let source = game.create_object_from_card(&CardBuilder::new(crate::CardId::new(), "Phased ability source")
                .card_types(vec![crate::CardType::Artifact]).build(), player, Zone::Battlefield);
            let recipient = game.create_object_from_card(&CardBuilder::new(crate::CardId::new(), "Sacrifice recipient")
                .card_types(vec![crate::CardType::Creature]).power_toughness(PowerToughness::fixed(2, 3)).build(), player, Zone::Battlefield);
            let source_snapshot = ObjectSnapshot::from_object_with_calculated_characteristics(game.object(source).unwrap(), &game);
            game.phase_out(source);
            if phased_recipient { game.phase_out(recipient); }
            let mut ctx = ExecutionContext::new_default(source, player).with_source_snapshot(source_snapshot.clone());
            let prepared = PreparedSacrifices::capture(&game, &ctx, vec![recipient]).unwrap();
            assert_eq!(prepared.source_snapshot.as_ref(), Some(&source_snapshot));
            assert_eq!(prepared.eligible.contains(&recipient), !phased_recipient);
            assert_eq!(prepared.draws.snapshots.contains_key(&recipient), !phased_recipient);
            let effect = crate::effect::Effect::new(SacrificeTargetEffect::new(ChooseSpec::SpecificObject(recipient)));
            let outcome = if dispatched { crate::effects::execute_effect(&mut game, &effect, &mut ctx) }
                else { effect.0.execute(&mut game, &mut ctx) }.unwrap();
            assert_eq!(outcome.instruction_result().count_or_zero(), i64::from(!phased_recipient));
            assert_eq!(game.object(recipient).is_some(), phased_recipient);
            assert!(game.is_phased_out(source));
        }}
    }

    #[test]
    fn direct_and_dispatched_sacrifice_wrappers_retain_pending_player_routing() {
        struct Pending { pending: bool, pause: bool, routed: Vec<PlayerId> }
        impl crate::decision::DecisionMaker for Pending {
            fn awaiting_choice(&self) -> bool { self.pending }
            fn decide_objects(&mut self, game: &GameState, ctx: &crate::decisions::context::SelectObjectsContext) -> Vec<ObjectId> {
                self.routed.push(game.controlling_player_for(ctx.player));
                self.pending = self.pause;
                if self.pending { Vec::new() } else { crate::decision::DecisionMaker::decide_objects(&mut crate::decision::SelectFirstDecisionMaker, game, ctx) }
            }
        }
        let a = PlayerId::from_index(0); let b = PlayerId::from_index(1); let c = PlayerId::from_index(2);
        for dispatched in [false, true] { for variant in 0..3 {
            let mut game = GameState::new(vec!["A".into(), "B".into(), "C".into()], 20);
            let creature = CardBuilder::new(crate::CardId::new(), "Suspended sacrifice")
                .card_types(vec![crate::CardType::Creature]).power_toughness(PowerToughness::fixed(2, 3)).build();
            let selected = game.create_object_from_card(&creature, a, Zone::Battlefield);
            let artifact = CardBuilder::new(crate::CardId::new(), "Routing source").card_types(vec![crate::CardType::Artifact]).build();
            let source = game.create_object_from_card(&artifact, a, Zone::Battlefield);
            let replacement_source = game.create_object_from_card(&artifact, c, Zone::Battlefield);
            game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(replacement_source, c,
                crate::events::zones::matchers::WouldChangeZoneMatcher::new(ObjectFilter::specific(selected), Some(Zone::Battlefield), Some(Zone::Graveyard)),
                ReplacementAction::Instead(vec![
                    crate::effect::Effect::control_player_until_end_of_turn(PlayerFilter::Specific(b)),
                    crate::effect::Effect::choose_objects(ObjectFilter::artifact(), 1, PlayerFilter::Specific(b), "routing_choice"),
                ])));
            let effect = match variant {
                0 => crate::effect::Effect::new(SacrificeEffect::you_creature(1)),
                1 => crate::effect::Effect::new(SacrificeTargetEffect::new(ChooseSpec::SpecificObject(selected))),
                _ => crate::effect::Effect::new(EachPlayerSacrificesEffect::new(ObjectFilter::creature(), 1, PlayerFilter::Any)),
            };
            let mut dm = Pending { pending: false, pause: true, routed: Vec::new() };
            {
                let mut ctx = ExecutionContext::new(source, a, &mut dm);
                if dispatched { crate::effects::execute_effect(&mut game, &effect, &mut ctx) }
                    else { effect.0.execute(&mut game, &mut ctx) }.unwrap();
                assert!(ctx.tagged_objects.is_empty() && ctx.effect_outcomes.is_empty());
            }
            assert!(dm.pending); assert_eq!(dm.routed, vec![c]);
            assert_eq!(game.controlling_player_for(b), c, "outer rollback preserves the actual pending prompt route");
            assert_eq!(game.object(selected).unwrap().zone, Zone::Battlefield);
            game.clear_pending_decision_controllers();
            assert_eq!(game.controlling_player_for(b), b, "the replacement prefix's physical control mutation was rolled back");
            dm.pending = false; dm.pause = false;
            let mut ctx = ExecutionContext::new(source, a, &mut dm);
            if dispatched { crate::effects::execute_effect(&mut game, &effect, &mut ctx) }
                else { effect.0.execute(&mut game, &mut ctx) }.unwrap();
            assert_eq!(game.controlling_player_for(b), c);
            assert_eq!(game.object(selected).unwrap().zone, Zone::Battlefield);
        }}
    }
}
