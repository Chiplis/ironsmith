//! Sacrifice effect implementation.

use crate::effect::{EffectOutcome, Value};
use crate::effects::helpers::{
    normalize_object_selection, resolve_player_filter, resolve_single_object_from_spec,
    resolve_value,
};
use crate::effects::{CostExecutableEffect, EffectExecutor};
use crate::event_processor::EventOutcome;
use crate::events::permanents::SacrificeEvent;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::ObjectId;
use crate::snapshot::ObjectSnapshot;
use crate::target::{ChooseSpec, ObjectFilter, PlayerFilter};
use crate::triggers::TriggerEvent;
use crate::zone::Zone;

use super::apply_zone_change;

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
    fn as_cost_executable(&self) -> Option<&dyn CostExecutableEffect> {
        Some(self)
    }

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
        let explicit_targets: Vec<ObjectId> = ctx
            .targets
            .iter()
            .filter_map(|target| match target {
                crate::executor::ResolvedTarget::Object(id) => Some(*id),
                crate::executor::ResolvedTarget::Player(_) => None,
            })
            .collect();
        let to_sacrifice = if required == 0 {
            Vec::new()
        } else if !explicit_targets.is_empty() {
            normalize_object_selection(explicit_targets, &matching, required)
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
            let result = apply_zone_change(
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
                EventOutcome::Proceed(result) => {
                    sacrificed_count += 1;
                    if result.final_zone == Zone::Graveyard {
                        sacrifice_events.push(TriggerEvent::new_with_provenance(
                            SacrificeEvent::new(id, Some(ctx.source))
                                .with_snapshot(pre_snapshot, sacrificing_player),
                            ctx.provenance,
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

    fn cost_description(&self) -> Option<String> {
        let count = match self.count {
            crate::effect::Value::Fixed(count) if count > 0 => count,
            _ => return None,
        };
        if self.player != PlayerFilter::You {
            return None;
        }
        Some(if count == 1 {
            format!("Sacrifice a {}", self.filter.description())
        } else {
            format!("Sacrifice {} {}", count, self.filter.description())
        })
    }
}

impl CostExecutableEffect for SacrificeEffect {
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
                obj.controller == controller
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
        let result = apply_zone_change(
            game,
            object_id,
            Zone::Battlefield,
            Zone::Graveyard,
            &mut *ctx.decision_maker,
        );

        match result {
            EventOutcome::Prevented => Ok((false, None)),
            EventOutcome::Proceed(result) => {
                let event = if result.final_zone == Zone::Graveyard {
                    Some(TriggerEvent::new_with_provenance(
                        SacrificeEvent::new(object_id, Some(ctx.source))
                            .with_snapshot(pre_snapshot, sacrificing_player),
                        ctx.provenance,
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
    fn as_cost_executable(&self) -> Option<&dyn CostExecutableEffect> {
        Some(self)
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        // Resolve through ChooseSpec helpers (targets, source, tagged, specific object, etc.).
        let object_id = match resolve_single_object_from_spec(game, &self.target, ctx) {
            Ok(id) => id,
            Err(ExecutionError::InvalidTarget) => return Ok(EffectOutcome::count(0)),
            Err(err) => return Err(err),
        };

        let (sacrificed, event) = Self::sacrifice_object(game, ctx, object_id)?;
        let mut outcome = EffectOutcome::count(if sacrificed { 1 } else { 0 });
        if let Some(event) = event {
            outcome = outcome.with_event(event);
        }
        Ok(outcome)
    }

    fn is_sacrifice_source_cost(&self) -> bool {
        matches!(self.target, ChooseSpec::Source)
    }

    fn cost_description(&self) -> Option<String> {
        if matches!(self.target, ChooseSpec::Source) {
            Some("Sacrifice ~".to_string())
        } else {
            None
        }
    }
}

impl CostExecutableEffect for SacrificeTargetEffect {
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
    use crate::card::{CardBuilder, PowerToughness};
    use crate::ids::{CardId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::Object;
    use crate::types::CardType;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
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

        assert_eq!(result.result, crate::effect::EffectResult::Count(1));
        assert!(!game.battlefield.contains(&target_id));
        assert_eq!(game.players[0].graveyard.len(), 1);
    }
}
