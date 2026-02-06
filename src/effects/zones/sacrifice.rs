//! Sacrifice effect implementation.

use crate::effect::{EffectOutcome, Value};
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_player_filter, resolve_value};
use crate::event_processor::{EventOutcome, process_zone_change};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::ObjectId;
use crate::target::{ChooseSpec, ObjectFilter, PlayerFilter};
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
            normalize_selection(chosen, &matching, required)
        };
        let mut sacrificed_count = 0;

        for id in to_sacrifice {
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

        Ok(EffectOutcome::count(sacrificed_count))
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }
}

fn normalize_selection(
    chosen: Vec<crate::ids::ObjectId>,
    candidates: &[crate::ids::ObjectId],
    required: usize,
) -> Vec<crate::ids::ObjectId> {
    let mut selected = Vec::with_capacity(required);

    for id in chosen {
        if selected.len() == required {
            break;
        }
        if candidates.contains(&id) && !selected.contains(&id) {
            selected.push(id);
        }
    }

    if selected.len() < required {
        for &id in candidates {
            if selected.len() == required {
                break;
            }
            if !selected.contains(&id) {
                selected.push(id);
            }
        }
    }

    selected
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
    ) -> Result<bool, ExecutionError> {
        // Verify the object can be sacrificed
        if !game.can_be_sacrificed(object_id) {
            return Ok(false);
        }

        // Verify it's on the battlefield
        if !game.battlefield.contains(&object_id) {
            return Ok(false);
        }

        // Process sacrifice through replacement effects
        let result = process_zone_change(
            game,
            object_id,
            Zone::Battlefield,
            Zone::Graveyard,
            &mut *ctx.decision_maker,
        );

        match result {
            EventOutcome::Prevented => Ok(false),
            EventOutcome::Proceed(final_zone) => {
                game.move_object(object_id, final_zone);
                Ok(true)
            }
            EventOutcome::Replaced => Ok(true),
            EventOutcome::NotApplicable => Ok(false),
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

        let sacrificed = Self::sacrifice_object(game, ctx, object_id)?;
        Ok(EffectOutcome::count(if sacrificed { 1 } else { 0 }))
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn is_sacrifice_source_cost(&self) -> bool {
        matches!(self.target, ChooseSpec::Source)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::card::PowerToughness;
    use crate::effect::EffectResult;
    use crate::ids::{CardId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::Object;
    use crate::static_abilities::StaticAbility;
    use crate::types::CardType;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    fn make_creature_card(card_id: u32, name: &str) -> crate::card::Card {
        CardBuilder::new(CardId::from_raw(card_id), name)
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Green],
            ]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build()
    }

    fn create_creature(game: &mut GameState, name: &str, controller: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let card = make_creature_card(id.0 as u32, name);
        let obj = Object::from_card(id, &card, controller, Zone::Battlefield);
        game.add_object(obj);
        id
    }

    #[test]
    fn test_sacrifice_creature() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature(&mut game, "Grizzly Bears", alice);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = SacrificeEffect::you_creature(1);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(1));
        // Creature should no longer be on battlefield
        assert!(!game.battlefield.contains(&creature_id));
        // Object should have a new ID in graveyard (per rule 400.7)
        assert!(!game.players[0].graveyard.is_empty());
    }

    #[test]
    fn test_sacrifice_indestructible_creature() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature(&mut game, "Darksteel Colossus", alice);
        let source = game.new_object_id();

        // Make it indestructible
        if let Some(obj) = game.object_mut(creature_id) {
            obj.abilities.push(crate::ability::Ability::static_ability(
                StaticAbility::indestructible(),
            ));
        }

        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = SacrificeEffect::you_creature(1);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Indestructible doesn't prevent sacrifice
        assert_eq!(result.result, EffectResult::Count(1));
        assert!(!game.battlefield.contains(&creature_id));
        assert!(!game.players[0].graveyard.is_empty());
    }

    #[test]
    fn test_sacrifice_multiple() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature1 = create_creature(&mut game, "Grizzly Bears", alice);
        let creature2 = create_creature(&mut game, "Hill Giant", alice);
        let creature3 = create_creature(&mut game, "Air Elemental", alice);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = SacrificeEffect::you_creature(2);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(2));
        // Only one should remain on battlefield
        let on_battlefield = [creature1, creature2, creature3]
            .iter()
            .filter(|id| game.battlefield.contains(id))
            .count();
        assert_eq!(on_battlefield, 1);
    }

    #[test]
    fn test_sacrifice_no_matching() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        // No creatures on battlefield
        let effect = SacrificeEffect::you_creature(1);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Zero is returned when no matching permanents
        assert_eq!(result.result, EffectResult::Count(0));
    }

    #[test]
    fn test_sacrifice_fewer_than_required() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature(&mut game, "Grizzly Bears", alice);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        // Ask to sacrifice 3 but only have 1
        let effect = SacrificeEffect::you_creature(3);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Should sacrifice what's available
        assert_eq!(result.result, EffectResult::Count(1));
        assert!(!game.battlefield.contains(&creature_id));
        assert!(!game.players[0].graveyard.is_empty());
    }

    #[test]
    fn test_sacrifice_only_own_creatures() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let _alice_creature = create_creature(&mut game, "Grizzly Bears", alice);
        let bob_creature = create_creature(&mut game, "Hill Giant", bob);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, bob);

        // Bob sacrifices a creature
        let effect = SacrificeEffect::you_creature(1);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(1));
        // Bob's creature should be sacrificed (no longer on battlefield)
        assert!(!game.battlefield.contains(&bob_creature));
        assert!(!game.players[1].graveyard.is_empty());
    }

    #[test]
    fn test_sacrifice_clone_box() {
        let effect = SacrificeEffect::you_creature(1);
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("SacrificeEffect"));
    }
}
