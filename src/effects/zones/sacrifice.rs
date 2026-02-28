//! Sacrifice effect implementation.

use crate::effect::{EffectOutcome, Value};
use crate::effects::EffectExecutor;
use crate::effects::helpers::{normalize_object_selection, resolve_player_filter, resolve_value};
use crate::event_processor::{EventOutcome, process_zone_change};
use crate::events::permanents::SacrificeEvent;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::ObjectId;
use crate::snapshot::ObjectSnapshot;
use crate::target::{ChooseSpec, ObjectFilter, PlayerFilter};
use crate::triggers::TriggerEvent;
use crate::zone::Zone;

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
}

impl SacrificeEffect {
    /// Create a new sacrifice effect.
    pub fn new(filter: ObjectFilter, count: impl Into<Value>, player: PlayerFilter) -> Self {
        Self {
            filter,
            count: count.into(),
            player,
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
}

impl EffectExecutor for SacrificeEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        use crate::decisions::make_decision;
        use crate::decisions::specs::ChooseObjectsSpec;
        let player_id = resolve_player_filter(game, &self.player, ctx)?;
        let count = resolve_value(game, &self.count, ctx)?.max(0) as usize;
        let filter_ctx = ctx.filter_context(game);

        // Find permanents the player controls that match the filter
        // Also filter out permanents that can't be sacrificed (Sigarda, Tajuru Preserver effects)
        let matching: Vec<ObjectId> = game
            .battlefield
            .iter()
            .filter_map(|&id| game.object(id).map(|obj| (id, obj)))
            .filter(|(id, obj)| {
                obj.controller == player_id
                    && self.filter.matches(obj, &filter_ctx, game)
                    && game.can_be_sacrificed(*id)
            })
            .map(|(id, _)| id)
            .collect();

        let required = count.min(matching.len());
        let to_sacrifice = if required == 0 {
            Vec::new()
        } else if required == matching.len() {
            // No choice remains: all matching permanents must be sacrificed.
            matching.clone()
        } else {
            let spec = ChooseObjectsSpec::new(
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
            let chosen: Vec<_> =
                make_decision(game, ctx.decision_maker, player_id, Some(ctx.source), spec);
            normalize_object_selection(chosen, &matching, required)
        };
        let mut sacrificed_count = 0;
        let mut sacrifice_events = Vec::new();

        for id in to_sacrifice {
            let pre_snapshot = game
                .object(id)
                .map(|obj| ObjectSnapshot::from_object(obj, game));
            let sacrificing_player = pre_snapshot.as_ref().map(|snapshot| snapshot.controller);

            // Process each sacrifice through replacement effects with decision maker
            let result = process_zone_change(
                game,
                id,
                Zone::Battlefield,
                Zone::Graveyard,
                &mut *ctx.decision_maker,
            );

            match result {
                EventOutcome::Prevented => {
                    // Sacrifice was prevented (unusual but possible)
                    continue;
                }
                EventOutcome::Proceed(final_zone) => {
                    game.move_object(id, final_zone);
                    sacrificed_count += 1;
                    if final_zone == Zone::Graveyard {
                        sacrifice_events.push(TriggerEvent::new(
                            SacrificeEvent::new(id, Some(ctx.source))
                                .with_snapshot(pre_snapshot, sacrificing_player),
                        ));
                    }
                }
                EventOutcome::Replaced => {
                    // Replacement effects already executed by process_zone_change
                    sacrificed_count += 1;
                }
                EventOutcome::NotApplicable => {
                    // Object no longer exists or isn't applicable
                    continue;
                }
            }
        }

        Ok(EffectOutcome::count(sacrificed_count).with_events(sacrifice_events))
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }
}

/// Effect that sacrifices a specific target (e.g., the source permanent).
///
/// Unlike `SacrificeEffect` which uses filters, this effect sacrifices a specific
/// object identified by a `ChooseSpec`. Commonly used for "Sacrifice ~" effects.
///
/// # Example
///
/// ```ignore
/// // Sacrifice the source permanent
/// let effect = SacrificeTargetEffect::source();
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct SacrificeTargetEffect {
    /// The target to sacrifice.
    pub target: ChooseSpec,
}

impl SacrificeTargetEffect {
    /// Create a new sacrifice target effect.
    pub fn new(target: ChooseSpec) -> Self {
        Self { target }
    }

    /// Create an effect that sacrifices the source permanent.
    pub fn source() -> Self {
        Self::new(ChooseSpec::Source)
    }

    /// Helper to sacrifice a single object.
    fn sacrifice_object(
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        object_id: ObjectId,
    ) -> Result<(bool, Option<TriggerEvent>), ExecutionError> {
        // Verify the object can be sacrificed
        if !game.can_be_sacrificed(object_id) {
            return Ok((false, None));
        }

        // Verify it's on the battlefield
        if !game.battlefield.contains(&object_id) {
            return Ok((false, None));
        }

        let pre_snapshot = game
            .object(object_id)
            .map(|obj| ObjectSnapshot::from_object(obj, game));
        let sacrificing_player = pre_snapshot.as_ref().map(|snapshot| snapshot.controller);

        // Process sacrifice through replacement effects
        let result = process_zone_change(
            game,
            object_id,
            Zone::Battlefield,
            Zone::Graveyard,
            &mut *ctx.decision_maker,
        );

        match result {
            EventOutcome::Prevented => Ok((false, None)),
            EventOutcome::Proceed(final_zone) => {
                game.move_object(object_id, final_zone);
                let event = if final_zone == Zone::Graveyard {
                    Some(TriggerEvent::new(
                        SacrificeEvent::new(object_id, Some(ctx.source))
                            .with_snapshot(pre_snapshot, sacrificing_player),
                    ))
                } else {
                    None
                };
                Ok((true, event))
            }
            EventOutcome::Replaced => Ok((true, None)),
            EventOutcome::NotApplicable => Ok((false, None)),
        }
    }
}

impl EffectExecutor for SacrificeTargetEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        // Resolve the target based on the spec
        let object_id: Option<ObjectId> = match &self.target {
            ChooseSpec::Source => Some(ctx.source),
            ChooseSpec::SpecificObject(id) => Some(*id),
            ChooseSpec::Tagged(tag) => {
                // Get tagged objects - extract ObjectId from ObjectSnapshot
                ctx.tagged_objects
                    .get(tag)
                    .and_then(|objects| objects.first())
                    .map(|snapshot| snapshot.object_id)
            }
            _ => {
                // For other target specs, try to get from resolved targets
                ctx.targets.first().and_then(|t| {
                    if let crate::executor::ResolvedTarget::Object(id) = t {
                        Some(*id)
                    } else {
                        None
                    }
                })
            }
        };

        let Some(object_id) = object_id else {
            return Ok(EffectOutcome::count(0));
        };

        let (sacrificed, event) = Self::sacrifice_object(game, ctx, object_id)?;
        let mut outcome = EffectOutcome::count(if sacrificed { 1 } else { 0 });
        if let Some(event) = event {
            outcome = outcome.with_event(event);
        }
        Ok(outcome)
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn is_sacrifice_source_cost(&self) -> bool {
        matches!(self.target, ChooseSpec::Source)
    }
}
