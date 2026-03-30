//! Sacrifice effect implementation.

use crate::effect::{EffectOutcome, ExecutionFact, Value};
use crate::effects::helpers::{
    normalize_object_selection, resolve_player_filter, resolve_single_object_for_effect,
    resolve_value,
};
use crate::effects::{CostExecutableEffect, EffectExecutor};
use crate::event_processor::EventOutcome;
use crate::events::permanents::SacrificeEvent;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::snapshot::ObjectSnapshot;
use crate::target::{ChooseSpec, ObjectFilter, PlayerFilter};
use crate::triggers::TriggerEvent;
use crate::zone::Zone;

use super::apply_zone_change_with_additional_effects;

fn players_in_turn_order(game: &GameState) -> Vec<PlayerId> {
    if game.turn_order.is_empty() {
        return Vec::new();
    }

    let start = game
        .turn_order
        .iter()
        .position(|&player_id| player_id == game.turn.active_player)
        .unwrap_or(0);

    (0..game.turn_order.len())
        .filter_map(|offset| {
            let player_id = game.turn_order[(start + offset) % game.turn_order.len()];
            game.player(player_id)
                .filter(|player| player.is_in_game())
                .map(|_| player_id)
        })
        .collect()
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
            obj.controller == player_id
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

    Ok(normalize_object_selection(chosen, &matching, required))
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
        let player_id = resolve_player_filter(game, &self.player, ctx)?;
        let count = resolve_value(game, &self.count, ctx)?.max(0) as usize;
        let explicit_targets: Vec<ObjectId> = ctx
            .targets
            .iter()
            .filter_map(|target| match target {
                crate::executor::ResolvedTarget::Object(id) => Some(*id),
                crate::executor::ResolvedTarget::Player(_) => None,
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
                    obj.controller == player_id
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
        let chosen_to_sacrifice = to_sacrifice.clone();
        let mut sacrificed_count = 0;
        let mut sacrificed_objects = Vec::new();
        let mut sacrifice_events = Vec::new();

        for id in to_sacrifice {
            let pre_snapshot = game
                .object(id)
                .map(|obj| ObjectSnapshot::from_object_with_calculated_characteristics(obj, game));
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
                    sacrificed_count += 1;
                    let _ = result;
                    sacrificed_objects.push(id);
                    sacrifice_events.push(TriggerEvent::new_with_provenance(
                        SacrificeEvent::new(id, Some(ctx.source))
                            .with_snapshot(pre_snapshot, sacrificing_player),
                        ctx.provenance,
                    ));
                }
                EventOutcome::Replaced => {
                    // Replacement effects already executed by process_zone_change
                    sacrificed_count += 1;
                    sacrificed_objects.push(id);
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
            .with_execution_fact(ExecutionFact::ChosenObjects(chosen_to_sacrifice));
        if !sacrificed_objects.is_empty() {
            outcome =
                outcome.with_execution_fact(ExecutionFact::AffectedObjects(sacrificed_objects));
        }
        Ok(outcome)
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
                    obj.controller == controller
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
    pub fn new(
        filter: ObjectFilter,
        count: impl Into<Value>,
        player_filter: PlayerFilter,
    ) -> Self {
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
        for player_id in players {
            let chosen = ctx.with_temp_iterated_player(Some(player_id), |ctx| {
                choose_objects_to_sacrifice(game, ctx, player_id, &self.filter, count)
            })?;
            all_chosen.extend(chosen.iter().copied());
            chosen_by_player.push((player_id, chosen));
        }

        let additional_effects = ctx.additional_replacement_effects_snapshot();
        let mut sacrificed_count = 0;
        let mut sacrificed_objects = Vec::new();
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
            .with_execution_fact(ExecutionFact::ChosenObjects(all_chosen));
        if !sacrificed_objects.is_empty() {
            outcome =
                outcome.with_execution_fact(ExecutionFact::AffectedObjects(sacrificed_objects));
        }
        Ok(outcome)
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
            .map(|obj| ObjectSnapshot::from_object_with_calculated_characteristics(obj, game));
        let sacrificing_player = pre_snapshot.as_ref().map(|snapshot| snapshot.controller);
        let additional_effects = ctx.additional_replacement_effects_snapshot();

        // Process sacrifice through replacement effects
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

        let (sacrificed, event) = Self::sacrifice_object(game, ctx, object_id)?;
        let mut outcome = EffectOutcome::count(if sacrificed { 1 } else { 0 });
        if let Some(event) = event {
            outcome = outcome.with_event(event);
        }
        outcome = outcome.with_execution_fact(ExecutionFact::ChosenObjects(vec![object_id]));
        if sacrificed {
            outcome = outcome.with_execution_fact(ExecutionFact::AffectedObjects(vec![object_id]));
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
    use crate::effect::{Effect, Restriction};
    use crate::effect::ExecutionFact;
    use crate::effects::CostExecutableEffect;
    use crate::effects::EarthbendEffect;
    use crate::executor::{ExecutionContext, execute_effect};
    use crate::ids::{CardId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::Object;
    use crate::static_abilities::StaticAbility;
    use crate::target::ChooseSpec;
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
        game.object_mut(creature_id)
            .expect("borrowed creature should exist")
            .controller = bob;

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
            graveyard_object.map(|object| (object.name.as_str(), object.owner, object.controller)),
            Some(("Borrowed Bear", alice, bob)),
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

        let result = EachPlayerSacrificesEffect::new(ObjectFilter::creature(), 1, PlayerFilter::Any)
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
