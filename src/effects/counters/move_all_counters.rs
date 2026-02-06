//! Move all counters effect implementation.

use crate::effect::{EffectOutcome, EffectResult};
use crate::effects::EffectExecutor;
use crate::executor::{ExecutionContext, ExecutionError, ResolvedTarget};
use crate::game_state::GameState;
use crate::object::CounterType;
use crate::target::ChooseSpec;

/// Effect that moves ALL counters of ALL types from one creature to another.
///
/// Used by Fate Transfer: "Move all counters from target creature onto another target creature."
///
/// # Fields
///
/// * `from` - Source creature (first target)
/// * `to` - Destination creature (second target)
///
/// # Example
///
/// ```ignore
/// // Move all counters from one creature to another
/// let effect = MoveAllCountersEffect::new(
///     ChooseSpec::creature(),
///     ChooseSpec::creature(),
/// );
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct MoveAllCountersEffect {
    /// Source creature (first target).
    pub from: ChooseSpec,
    /// Destination creature (second target).
    pub to: ChooseSpec,
}

impl MoveAllCountersEffect {
    /// Create a new move all counters effect.
    pub fn new(from: ChooseSpec, to: ChooseSpec) -> Self {
        Self { from, to }
    }

    /// Create an effect that moves all counters between creatures.
    pub fn between_creatures() -> Self {
        Self::new(ChooseSpec::creature(), ChooseSpec::creature())
    }
}

impl EffectExecutor for MoveAllCountersEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        // Get from and to targets from resolved targets
        if ctx.targets.len() < 2 {
            return Ok(EffectOutcome::from_result(EffectResult::TargetInvalid));
        }

        let from_id = match &ctx.targets[0] {
            ResolvedTarget::Object(id) => *id,
            _ => return Ok(EffectOutcome::from_result(EffectResult::TargetInvalid)),
        };

        let to_id = match &ctx.targets[1] {
            ResolvedTarget::Object(id) => *id,
            _ => return Ok(EffectOutcome::from_result(EffectResult::TargetInvalid)),
        };

        // Get all counters from source
        let counters_to_move: Vec<(CounterType, u32)> = game
            .object(from_id)
            .map(|obj| {
                obj.counters
                    .iter()
                    .map(|(ct, &count)| (*ct, count))
                    .collect()
            })
            .unwrap_or_default();

        if counters_to_move.is_empty() {
            return Ok(EffectOutcome::count(0));
        }

        let mut total_moved = 0u32;
        let mut outcome = EffectOutcome::count(0);

        // Move each counter type using centralized methods
        for (counter_type, count) in counters_to_move {
            // Remove from source
            if let Some((removed, remove_event)) = game.remove_counters(
                from_id,
                counter_type,
                count,
                Some(ctx.source),
                Some(ctx.controller),
            ) {
                outcome = outcome.with_event(remove_event);
                total_moved += removed;

                // Add to destination (only the amount actually removed)
                if let Some(add_event) = game.add_counters_with_source(
                    to_id,
                    counter_type,
                    removed,
                    Some(ctx.source),
                    Some(ctx.controller),
                ) {
                    outcome = outcome.with_event(add_event);
                }
            }
        }

        outcome.result = crate::effect::EffectResult::Count(total_moved as i32);
        Ok(outcome)
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.from)
    }

    fn target_description(&self) -> &'static str {
        "creature to move counters from"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::Object;
    use crate::types::CardType;
    use crate::zone::Zone;

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

    fn create_creature_with_multiple_counters(
        game: &mut GameState,
        name: &str,
        controller: PlayerId,
    ) -> ObjectId {
        let id = game.new_object_id();
        let card = make_creature_card(id.0 as u32, name);
        let mut obj = Object::from_card(id, &card, controller, Zone::Battlefield);
        obj.counters.insert(CounterType::PlusOnePlusOne, 3);
        obj.counters.insert(CounterType::MinusOneMinusOne, 2);
        game.add_object(obj);
        id
    }

    fn create_creature(game: &mut GameState, name: &str, controller: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let card = make_creature_card(id.0 as u32, name);
        let obj = Object::from_card(id, &card, controller, Zone::Battlefield);
        game.add_object(obj);
        id
    }

    #[test]
    fn test_move_all_counters() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let from_id = create_creature_with_multiple_counters(&mut game, "Source Creature", alice);
        let to_id = create_creature(&mut game, "Target Creature", alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice).with_targets(vec![
            ResolvedTarget::Object(from_id),
            ResolvedTarget::Object(to_id),
        ]);

        let effect = MoveAllCountersEffect::between_creatures();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(5)); // 3 + 2

        let from_obj = game.object(from_id).unwrap();
        assert!(from_obj.counters.is_empty());

        let to_obj = game.object(to_id).unwrap();
        assert_eq!(to_obj.counters.get(&CounterType::PlusOnePlusOne), Some(&3));
        assert_eq!(
            to_obj.counters.get(&CounterType::MinusOneMinusOne),
            Some(&2)
        );
    }

    #[test]
    fn test_move_all_counters_no_counters() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let from_id = create_creature(&mut game, "Source Creature", alice);
        let to_id = create_creature(&mut game, "Target Creature", alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice).with_targets(vec![
            ResolvedTarget::Object(from_id),
            ResolvedTarget::Object(to_id),
        ]);

        let effect = MoveAllCountersEffect::between_creatures();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(0));
    }

    #[test]
    fn test_move_all_counters_adds_to_existing() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let from_id = create_creature_with_multiple_counters(&mut game, "Source Creature", alice);

        // Target already has some counters
        let to_id = game.new_object_id();
        let card = make_creature_card(to_id.0 as u32, "Target Creature");
        let mut to_obj = Object::from_card(to_id, &card, alice, Zone::Battlefield);
        to_obj.counters.insert(CounterType::PlusOnePlusOne, 1);
        game.add_object(to_obj);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice).with_targets(vec![
            ResolvedTarget::Object(from_id),
            ResolvedTarget::Object(to_id),
        ]);

        let effect = MoveAllCountersEffect::between_creatures();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(5)); // 3 + 2 moved

        let to_obj = game.object(to_id).unwrap();
        assert_eq!(to_obj.counters.get(&CounterType::PlusOnePlusOne), Some(&4)); // 1 + 3
        assert_eq!(
            to_obj.counters.get(&CounterType::MinusOneMinusOne),
            Some(&2)
        );
    }

    #[test]
    fn test_move_all_counters_insufficient_targets() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let from_id = create_creature_with_multiple_counters(&mut game, "Source Creature", alice);
        let source = game.new_object_id();

        // Only one target provided
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(from_id)]);

        let effect = MoveAllCountersEffect::between_creatures();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::TargetInvalid);
    }

    #[test]
    fn test_move_all_counters_clone_box() {
        let effect = MoveAllCountersEffect::between_creatures();
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("MoveAllCountersEffect"));
    }
}
