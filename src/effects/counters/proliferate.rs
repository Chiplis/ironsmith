//! Proliferate effect implementation.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::events::{KeywordActionEvent, KeywordActionKind};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::object::CounterType;
use crate::triggers::TriggerEvent;

/// Effect that proliferates (adds counters to permanents/players with counters).
///
/// For each permanent with counters and each player with counters, adds one
/// counter of each type they already have.
///
/// # Example
///
/// ```ignore
/// let effect = ProliferateEffect;
/// ```
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ProliferateEffect;

impl ProliferateEffect {
    /// Create a new proliferate effect.
    pub fn new() -> Self {
        Self
    }
}

impl EffectExecutor for ProliferateEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        // Find all permanents and players with counters
        // For simplicity, add one counter of each type they already have

        let mut proliferated_count = 0;
        let mut outcome = EffectOutcome::count(0);

        // Collect permanents with counters and their counter types
        let permanents_with_counters: Vec<(crate::ids::ObjectId, Vec<CounterType>)> = game
            .battlefield
            .iter()
            .filter_map(|&perm_id| {
                game.object(perm_id).and_then(|obj| {
                    if obj.counters.is_empty() {
                        None
                    } else {
                        Some((perm_id, obj.counters.keys().copied().collect()))
                    }
                })
            })
            .collect();

        // Proliferate permanents using centralized method
        for (perm_id, counter_types) in permanents_with_counters {
            for ct in counter_types {
                if let Some(event) = game.add_counters_with_source(
                    perm_id,
                    ct,
                    1,
                    Some(ctx.source),
                    Some(ctx.controller),
                ) {
                    outcome = outcome.with_event(event);
                }
            }
            proliferated_count += 1;
        }

        // Proliferate players and emit marker events for player counters.
        let players_with_counters: Vec<(crate::ids::PlayerId, Vec<CounterType>)> = game
            .players
            .iter()
            .map(|p| {
                let mut counters = Vec::new();
                if p.poison_counters > 0 {
                    counters.push(CounterType::Poison);
                }
                if p.energy_counters > 0 {
                    counters.push(CounterType::Energy);
                }
                if p.experience_counters > 0 {
                    counters.push(CounterType::Experience);
                }
                (p.id, counters)
            })
            .filter(|(_, counters)| !counters.is_empty())
            .collect();

        for (player_id, counters) in players_with_counters {
            for counter_type in counters {
                if let Some(event) = game.add_player_counters_with_source(
                    player_id,
                    counter_type,
                    1,
                    Some(ctx.source),
                    Some(ctx.controller),
                ) {
                    outcome = outcome.with_event(event);
                }
            }
            proliferated_count += 1;
        }

        outcome.result = crate::effect::EffectResult::Count(proliferated_count);
        Ok(
            outcome.with_event(TriggerEvent::new(KeywordActionEvent::new(
                KeywordActionKind::Proliferate,
                ctx.controller,
                ctx.source,
                1,
            ))),
        )
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::effect::EffectResult;
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

    fn create_creature_with_counters(
        game: &mut GameState,
        name: &str,
        controller: PlayerId,
        counter_type: CounterType,
        count: u32,
    ) -> ObjectId {
        let id = game.new_object_id();
        let card = make_creature_card(id.0 as u32, name);
        let mut obj = Object::from_card(id, &card, controller, Zone::Battlefield);
        obj.counters.insert(counter_type, count);
        game.add_object(obj);
        id
    }

    #[test]
    fn test_proliferate_permanents() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature_with_counters(
            &mut game,
            "Hangarback Walker",
            alice,
            CounterType::PlusOnePlusOne,
            3,
        );
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = ProliferateEffect::new();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(1)); // 1 permanent proliferated
        let obj = game.object(creature_id).unwrap();
        assert_eq!(obj.counters.get(&CounterType::PlusOnePlusOne), Some(&4)); // 3 + 1
    }

    #[test]
    fn test_proliferate_multiple_counter_types() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let id = game.new_object_id();
        let card = make_creature_card(id.0 as u32, "Multi-Counter Creature");
        let mut obj = Object::from_card(id, &card, alice, Zone::Battlefield);
        obj.counters.insert(CounterType::PlusOnePlusOne, 2);
        obj.counters.insert(CounterType::MinusOneMinusOne, 1);
        game.add_object(obj);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = ProliferateEffect::new();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(1)); // 1 permanent proliferated
        let obj = game.object(id).unwrap();
        assert_eq!(obj.counters.get(&CounterType::PlusOnePlusOne), Some(&3)); // 2 + 1
        assert_eq!(obj.counters.get(&CounterType::MinusOneMinusOne), Some(&2)); // 1 + 1
    }

    #[test]
    fn test_proliferate_poison_counters() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Give Alice some poison counters
        game.players[0].poison_counters = 5;

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = ProliferateEffect::new();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(1)); // 1 player counter proliferated
        assert_eq!(game.players[0].poison_counters, 6); // 5 + 1
    }

    #[test]
    fn test_proliferate_energy_counters() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Give Alice some energy counters
        game.players[0].energy_counters = 3;

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = ProliferateEffect::new();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(1));
        assert_eq!(game.players[0].energy_counters, 4); // 3 + 1
    }

    #[test]
    fn test_proliferate_nothing() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        // No permanents with counters, no players with counters
        let effect = ProliferateEffect::new();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(0));
    }

    #[test]
    fn test_proliferate_multiple_permanents() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let creature1 = create_creature_with_counters(
            &mut game,
            "Creature 1",
            alice,
            CounterType::PlusOnePlusOne,
            2,
        );
        let creature2 = create_creature_with_counters(
            &mut game,
            "Creature 2",
            bob,
            CounterType::MinusOneMinusOne,
            1,
        );

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = ProliferateEffect::new();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(2)); // 2 permanents proliferated

        let obj1 = game.object(creature1).unwrap();
        assert_eq!(obj1.counters.get(&CounterType::PlusOnePlusOne), Some(&3)); // 2 + 1

        let obj2 = game.object(creature2).unwrap();
        assert_eq!(obj2.counters.get(&CounterType::MinusOneMinusOne), Some(&2)); // 1 + 1
    }

    #[test]
    fn test_proliferate_clone_box() {
        let effect = ProliferateEffect::new();
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("ProliferateEffect"));
    }

    #[test]
    fn test_proliferate_default() {
        let effect = ProliferateEffect::default();
        assert_eq!(effect, ProliferateEffect);
    }
}
