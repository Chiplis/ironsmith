//! Sacrifice effect implementation.

use crate::effect::{EffectOutcome, ExecutionFact, OutcomeObjectMemory, Value};
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

use super::apply_zone_change_with_additional_effects;

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
                && game.can_be_sacrificed(*id)
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
                    && game.can_be_sacrificed(*id)
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
        SacrificeEffect::player(self.filter.clone(), self.count.clone(), self.player.clone())
            .execute(game, ctx)
    }
}

impl CostExecutableEffect for SacrificePlayerEffect {
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
                        && game.can_be_sacrificed(*id)
                })
                .map(|(id, _)| id)
                .collect();
            let required = count.min(matching.len());
            normalize_object_selection(explicit_targets, &matching, required)
        } else {
            choose_objects_to_sacrifice(game, ctx, player_id, &self.filter, count)?
        };
        sacrifice_selected_objects(
            game,
            ctx,
            &self.event_object_tags,
            &self.event_source_tags,
            to_sacrifice,
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
        let description = self.filter.description();
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
    chosen: Vec<ObjectId>,
    event_object_tags: Vec<TagKey>,
    event_source_tags: Vec<TagKey>,
}

impl crate::effects::SimultaneousEffectProposal for SacrificeProposal {
    fn commit(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        sacrifice_selected_objects(
            game,
            ctx,
            &self.event_object_tags,
            &self.event_source_tags,
            self.chosen.clone(),
        )
    }
}

impl SacrificeEffect {
    /// Choose this player's sacrifices against immutable pre-action state.
    fn prepare_proposal(
        &self,
        game: &GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<SacrificeProposal, ExecutionError> {
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
                    && game.can_be_sacrificed(*id)
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
            chosen,
            event_object_tags: self.event_object_tags.clone(),
            event_source_tags: self.event_source_tags.clone(),
        })
    }
}

/// Move the already-chosen objects to the graveyard as sacrifices, emitting
/// the same events and outcome facts regardless of whether the selection came
/// from a live execution or a simultaneous each-player proposal.
fn sacrifice_selected_objects(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    event_object_tags: &[TagKey],
    event_source_tags: &[TagKey],
    to_sacrifice: Vec<ObjectId>,
) -> Result<EffectOutcome, ExecutionError> {
    let chosen_to_sacrifice = to_sacrifice.clone();
    let chosen_memory: Vec<_> = chosen_to_sacrifice
        .iter()
        .filter_map(|id| OutcomeObjectMemory::from_object_id(game, *id))
        .collect();
    let mut sacrificed_count = 0;
    let mut sacrificed_objects = Vec::new();
    let mut sacrificed_memory = Vec::new();
    let mut sacrifice_events = Vec::new();

    for id in to_sacrifice {
        let pre_snapshot = game
            .object(id)
            .map(|obj| ObjectSnapshot::from_object_with_calculated_characteristics(obj, game));
        let source_snapshot_for_event = if event_source_tags.is_empty() {
            None
        } else if pre_snapshot
            .as_ref()
            .is_some_and(|snapshot| snapshot.object_id == ctx.source)
        {
            pre_snapshot.clone()
        } else {
            game.object(ctx.source)
                .map(|obj| ObjectSnapshot::from_object_with_calculated_characteristics(obj, game))
                .or_else(|| ctx.source_snapshot.clone())
        };
        let sacrificing_player = pre_snapshot.as_ref().map(|snapshot| snapshot.controller);
        let additional_effects = ctx.additional_replacement_effects_snapshot();

        // Process each sacrifice through replacement effects with decision maker
        let result = apply_zone_change_with_additional_effects(
            game,
            id,
            Zone::Battlefield,
            Zone::Graveyard,
            ctx.cause.clone(),
            &mut *ctx.decision_maker,
            &additional_effects,
        );

        match result {
            EventOutcome::Prevented => {
                // Sacrifice was prevented (unusual but possible)
                continue;
            }
            EventOutcome::Proceed(result) => {
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
                    sacrificed_memory.push(OutcomeObjectMemory::from_snapshot(snapshot));
                }
                sacrifice_events.push(TriggerEvent::new_with_provenance(
                    SacrificeEvent::new(id, Some(ctx.source))
                        .with_snapshot(pre_snapshot, sacrificing_player),
                    ctx.provenance,
                ));
            }
            EventOutcome::Replaced => {
                // Replacement effects already executed by process_zone_change
                tag_sacrifice_zone_change_event(
                    game,
                    id,
                    event_object_tags,
                    event_source_tags,
                    pre_snapshot.as_ref(),
                    source_snapshot_for_event.as_ref(),
                );
                sacrificed_count += 1;
                sacrificed_objects.push(id);
                if let Some(snapshot) = pre_snapshot.as_ref() {
                    sacrificed_memory.push(OutcomeObjectMemory::from_snapshot(snapshot));
                }
                sacrifice_events.push(TriggerEvent::new_with_provenance(
                    SacrificeEvent::new(id, Some(ctx.source))
                        .with_snapshot(pre_snapshot, sacrificing_player),
                    ctx.provenance,
                ));
            }
            EventOutcome::NotApplicable => {
                // Object no longer exists or isn't applicable
                continue;
            }
        }
    }

    let mut outcome = EffectOutcome::count(sacrificed_count)
        .with_events(sacrifice_events)
        .with_execution_fact(ExecutionFact::ChosenObjects(chosen_to_sacrifice))
        .with_chosen_object_memory(chosen_memory);
    if !sacrificed_objects.is_empty() {
        outcome = outcome.with_execution_fact(ExecutionFact::AffectedObjects(sacrificed_objects));
        outcome = outcome.with_affected_object_memory(sacrificed_memory);
    }
    Ok(outcome)
}

impl CostExecutableEffect for SacrificeEffect {
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
        let count = resolve_value(game, &self.count, ctx)?.max(0) as usize;
        if count == 0 {
            return Ok(EffectOutcome::count(0));
        }

        let filter_ctx = ctx.filter_context(game);
        let players: Vec<PlayerId> = players_in_turn_order(game)
            .into_iter()
            .filter(|player_id| self.player_filter.matches_player(*player_id, &filter_ctx))
            .collect();
        if players.is_empty() {
            return Ok(EffectOutcome::count(0));
        }

        let mut chosen_by_player = Vec::new();
        let mut all_chosen = Vec::new();
        let mut chosen_memory = Vec::new();
        for player_id in players {
            let chosen = ctx.with_temp_iterated_player(Some(player_id), |ctx| {
                choose_objects_to_sacrifice(game, ctx, player_id, &self.filter, count)
            })?;
            chosen_memory.extend(
                chosen
                    .iter()
                    .filter_map(|id| OutcomeObjectMemory::from_object_id(game, *id)),
            );
            all_chosen.extend(chosen.iter().copied());
            chosen_by_player.push((player_id, chosen));
        }

        let additional_effects = ctx.additional_replacement_effects_snapshot();
        let mut sacrificed_count = 0;
        let mut sacrificed_objects = Vec::new();
        let mut sacrificed_memory = Vec::new();
        let mut sacrifice_events = Vec::new();

        for (_player_id, chosen) in chosen_by_player {
            for id in chosen {
                let pre_snapshot = game.object(id).map(|obj| {
                    ObjectSnapshot::from_object_with_calculated_characteristics(obj, game)
                });
                let sacrificing_player = pre_snapshot.as_ref().map(|snapshot| snapshot.controller);

                let result = apply_zone_change_with_additional_effects(
                    game,
                    id,
                    Zone::Battlefield,
                    Zone::Graveyard,
                    ctx.cause.clone(),
                    &mut *ctx.decision_maker,
                    &additional_effects,
                );

                match result {
                    EventOutcome::Prevented | EventOutcome::NotApplicable => continue,
                    EventOutcome::Proceed(_) | EventOutcome::Replaced => {
                        sacrificed_count += 1;
                        sacrificed_objects.push(id);
                        if let Some(snapshot) = pre_snapshot.as_ref() {
                            sacrificed_memory.push(OutcomeObjectMemory::from_snapshot(snapshot));
                        }
                        sacrifice_events.push(TriggerEvent::new_with_provenance(
                            SacrificeEvent::new(id, Some(ctx.source))
                                .with_snapshot(pre_snapshot, sacrificing_player),
                            ctx.provenance,
                        ));
                    }
                }
            }
        }

        let mut outcome = EffectOutcome::count(sacrificed_count)
            .with_events(sacrifice_events)
            .with_execution_fact(ExecutionFact::ChosenObjects(all_chosen))
            .with_chosen_object_memory(chosen_memory);
        if !sacrificed_objects.is_empty() {
            outcome =
                outcome.with_execution_fact(ExecutionFact::AffectedObjects(sacrificed_objects));
            outcome = outcome.with_affected_object_memory(sacrificed_memory);
        }
        Ok(outcome)
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

fn sacrifice_target_object(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    object_id: ObjectId,
) -> Result<(bool, Option<TriggerEvent>), ExecutionError> {
    if !game.can_be_sacrificed(object_id) {
        return Ok((false, None));
    }
    if !game.battlefield.contains(&object_id) {
        return Ok((false, None));
    }

    let pre_snapshot = game
        .object(object_id)
        .map(|obj| ObjectSnapshot::from_object_with_calculated_characteristics(obj, game));
    let sacrificing_player = pre_snapshot.as_ref().map(|snapshot| snapshot.controller);
    let additional_effects = ctx.additional_replacement_effects_snapshot();

    let result = apply_zone_change_with_additional_effects(
        game,
        object_id,
        Zone::Battlefield,
        Zone::Graveyard,
        ctx.cause.clone(),
        &mut *ctx.decision_maker,
        &additional_effects,
    );

    match result {
        EventOutcome::Prevented => Ok((false, None)),
        EventOutcome::Proceed(result) => {
            if let Some(snapshot) = pre_snapshot.clone() {
                ctx.refresh_target_snapshot(snapshot);
            }
            if let Some(snapshot) = pre_snapshot.clone()
                && snapshot.object_id == ctx.source
            {
                ctx.refresh_source_snapshot(snapshot);
            }
            let _ = result;
            let event = Some(TriggerEvent::new_with_provenance(
                SacrificeEvent::new(object_id, Some(ctx.source))
                    .with_snapshot(pre_snapshot, sacrificing_player),
                ctx.provenance,
            ));
            Ok((true, event))
        }
        EventOutcome::Replaced => Ok((
            true,
            Some(TriggerEvent::new_with_provenance(
                SacrificeEvent::new(object_id, Some(ctx.source))
                    .with_snapshot(pre_snapshot, sacrificing_player),
                ctx.provenance,
            )),
        )),
        EventOutcome::NotApplicable => Ok((false, None)),
    }
}

impl EffectExecutor for SacrificeTargetEffect {
    fn as_cost_executable(&self) -> Option<&dyn CostExecutableEffect> {
        Some(self)
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        // Resolve through ChooseSpec helpers (targets, source, tagged, specific object, etc.).
        let object_id = match resolve_single_object_for_effect(game, ctx, &self.target) {
            Ok(id) => id,
            Err(ExecutionError::InvalidTarget) => return Ok(EffectOutcome::count(0)),
            Err(err) => return Err(err),
        };

        let object_memory = OutcomeObjectMemory::from_object_id(game, object_id);
        let (sacrificed, event) = sacrifice_target_object(game, ctx, object_id)?;
        let mut outcome = EffectOutcome::count(if sacrificed { 1 } else { 0 });
        if let Some(event) = event {
            outcome = outcome.with_event(event);
        }
        outcome = outcome.with_execution_fact(ExecutionFact::ChosenObjects(vec![object_id]));
        if let Some(memory) = object_memory.clone() {
            outcome = outcome.with_chosen_object_memory(vec![memory]);
        }
        if sacrificed {
            outcome = outcome.with_execution_fact(ExecutionFact::AffectedObjects(vec![object_id]));
            if let Some(memory) = object_memory {
                outcome = outcome.with_affected_object_memory(vec![memory]);
            }
        }
        Ok(outcome)
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
    use crate::cards::definitions::basic_mountain;
    use crate::effect::ExecutionFact;
    use crate::effect::{Effect, Restriction};
    use crate::effects::CostExecutableEffect;
    use crate::effects::EarthbendEffect;
    use crate::effects::{ExecutionContext, execute_effect};
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
        game.set_current_controller(creature_id, bob);

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
