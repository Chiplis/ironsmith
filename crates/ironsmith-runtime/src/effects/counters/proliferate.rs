//! Proliferate effect implementation.

use crate::decision::FallbackStrategy;
use crate::decisions::{ProliferateSpec, make_decision_with_fallback};
use crate::effect::{EffectOutcome, Value};
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_value;
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
/// let effect = ProliferateEffect::new(1);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ProliferateEffect {
    /// How many times to proliferate.
    pub count: Value,
}

impl ProliferateEffect {
    /// Create a new proliferate effect.
    pub fn new(count: impl Into<Value>) -> Self {
        Self {
            count: count.into(),
        }
    }
}

impl Default for ProliferateEffect {
    fn default() -> Self {
        Self::new(1)
    }
}

impl EffectExecutor for ProliferateEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let count = resolve_value(game, &self.count, ctx)?.max(0) as usize;
        if count == 0 {
            return Ok(EffectOutcome::resolved());
        }

        let mut proliferated_total = 0;
        let mut outcome = EffectOutcome::count(0);
        let mut action_events = Vec::with_capacity(count);

        for _ in 0..count {
            let mut proliferated_count = 0;

            let eligible_permanents: Vec<crate::ids::ObjectId> = game
                .battlefield
                .iter()
                .filter_map(|&perm_id| {
                    game.object(perm_id).and_then(|obj| {
                        if obj.counters.is_empty() {
                            None
                        } else {
                            Some(perm_id)
                        }
                    })
                })
                .collect();

            let eligible_players: Vec<crate::ids::PlayerId> = game
                .players
                .iter()
                .filter_map(|p| {
                    let has_counters =
                        p.poison_counters > 0 || p.energy_counters > 0 || p.experience_counters > 0;
                    has_counters.then_some(p.id)
                })
                .collect();

            let selections = make_decision_with_fallback(
                game,
                &mut ctx.decision_maker,
                ctx.controller,
                Some(ctx.source),
                ProliferateSpec::new(
                    ctx.source,
                    eligible_permanents.clone(),
                    eligible_players.clone(),
                ),
                FallbackStrategy::Maximum,
            );

            let chosen_permanents: Vec<_> = selections
                .permanents
                .into_iter()
                .filter(|perm_id| eligible_permanents.contains(perm_id))
                .collect();
            let chosen_players: Vec<_> = selections
                .players
                .into_iter()
                .filter(|player_id| eligible_players.contains(player_id))
                .collect();

            for perm_id in chosen_permanents {
                let Some(counter_types): Option<Vec<CounterType>> =
                    game.object(perm_id).and_then(|obj| {
                        (!obj.counters.is_empty()).then(|| obj.counters.keys().copied().collect())
                    })
                else {
                    continue;
                };

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

            for player_id in chosen_players {
                let Some(counters) = game.player(player_id).map(|p| {
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
                    counters
                }) else {
                    continue;
                };
                if counters.is_empty() {
                    continue;
                }

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

            proliferated_total += proliferated_count;
            action_events.push(TriggerEvent::new_with_provenance(
                KeywordActionEvent::new(
                    KeywordActionKind::Proliferate,
                    ctx.controller,
                    ctx.source,
                    1,
                ),
                ctx.provenance,
            ));
        }

        outcome.set_value(crate::effect::OutcomeValue::Count(proliferated_total));
        Ok(outcome.with_events(action_events))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::decision::DecisionMaker;
    use crate::decisions::specs::ProliferateResponse;
    use crate::events::EventKind;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::Object;
    use crate::types::CardType;
    use crate::zone::Zone;
    use std::collections::VecDeque;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
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

    struct ScriptedProliferateDecisionMaker {
        responses: VecDeque<ProliferateResponse>,
    }

    impl DecisionMaker for ScriptedProliferateDecisionMaker {
        fn decide_proliferate(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::ProliferateContext,
        ) -> ProliferateResponse {
            self.responses.pop_front().unwrap_or_default()
        }
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

        let effect = ProliferateEffect::new(1);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(1)); // 1 permanent proliferated
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

        let effect = ProliferateEffect::new(1);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(1)); // 1 permanent proliferated
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

        let effect = ProliferateEffect::new(1);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(1)); // 1 player counter proliferated
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

        let effect = ProliferateEffect::new(1);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(1));
        assert_eq!(game.players[0].energy_counters, 4); // 3 + 1
    }

    #[test]
    fn test_proliferate_nothing() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        // No permanents with counters, no players with counters
        let effect = ProliferateEffect::new(1);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(0));
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

        let effect = ProliferateEffect::new(1);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(2)); // 2 permanents proliferated

        let obj1 = game.object(creature1).unwrap();
        assert_eq!(obj1.counters.get(&CounterType::PlusOnePlusOne), Some(&3)); // 2 + 1

        let obj2 = game.object(creature2).unwrap();
        assert_eq!(obj2.counters.get(&CounterType::MinusOneMinusOne), Some(&2)); // 1 + 1
    }

    #[test]
    fn test_proliferate_clone_box() {
        let effect = ProliferateEffect::new(1);
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("ProliferateEffect"));
    }

    #[test]
    fn test_proliferate_default() {
        let effect = ProliferateEffect::default();
        assert_eq!(effect, ProliferateEffect::new(1));
    }

    #[test]
    fn test_proliferate_twice_repeats_action() {
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

        let effect = ProliferateEffect::new(2);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(2));
        let obj = game.object(creature_id).unwrap();
        assert_eq!(obj.counters.get(&CounterType::PlusOnePlusOne), Some(&5));
    }

    #[test]
    fn test_proliferate_can_choose_subset_of_eligible_permanents_and_players() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let untouched = create_creature_with_counters(
            &mut game,
            "Untouched",
            alice,
            CounterType::PlusOnePlusOne,
            2,
        );
        let chosen =
            create_creature_with_counters(&mut game, "Chosen", bob, CounterType::Charge, 1);
        game.players[0].poison_counters = 2;
        game.players[0].energy_counters = 3;
        game.players[1].experience_counters = 1;

        let source = game.new_object_id();
        let mut decision_maker = ScriptedProliferateDecisionMaker {
            responses: VecDeque::from([ProliferateResponse {
                permanents: vec![chosen],
                players: vec![alice],
            }]),
        };
        let mut ctx = ExecutionContext::new(source, alice, &mut decision_maker);

        let result = ProliferateEffect::new(1)
            .execute(&mut game, &mut ctx)
            .expect("subset proliferate should resolve");

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(2));
        assert_eq!(
            game.object(untouched)
                .and_then(|obj| obj.counters.get(&CounterType::PlusOnePlusOne).copied()),
            Some(2)
        );
        assert_eq!(
            game.object(chosen)
                .and_then(|obj| obj.counters.get(&CounterType::Charge).copied()),
            Some(2)
        );
        assert_eq!(game.players[0].poison_counters, 3);
        assert_eq!(game.players[0].energy_counters, 4);
        assert_eq!(game.players[1].experience_counters, 1);
    }

    #[test]
    fn test_proliferate_can_choose_nothing_and_still_perform_keyword_action() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let creature_id = create_creature_with_counters(
            &mut game,
            "Hangarback Walker",
            alice,
            CounterType::PlusOnePlusOne,
            3,
        );
        game.players[0].poison_counters = 4;

        let source = game.new_object_id();
        let mut decision_maker = ScriptedProliferateDecisionMaker {
            responses: VecDeque::from([ProliferateResponse::default()]),
        };
        let mut ctx = ExecutionContext::new(source, alice, &mut decision_maker);

        let result = ProliferateEffect::new(1)
            .execute(&mut game, &mut ctx)
            .expect("empty proliferate choice should resolve");

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(0));
        assert_eq!(
            game.object(creature_id)
                .and_then(|obj| obj.counters.get(&CounterType::PlusOnePlusOne).copied()),
            Some(3)
        );
        assert_eq!(game.players[0].poison_counters, 4);
        assert_eq!(result.events.len(), 1);
        assert_eq!(result.events[0].kind(), EventKind::KeywordAction);
        let keyword = result.events[0]
            .inner()
            .as_any()
            .downcast_ref::<KeywordActionEvent>()
            .expect("expected keyword action event");
        assert_eq!(keyword.action, KeywordActionKind::Proliferate);
        assert_eq!(keyword.player, alice);
        assert_eq!(keyword.amount, 1);
    }

    #[test]
    fn test_proliferate_twice_rechooses_targets_each_time() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let creature_id = create_creature_with_counters(
            &mut game,
            "Hangarback Walker",
            alice,
            CounterType::PlusOnePlusOne,
            1,
        );
        game.players[1].poison_counters = 2;

        let source = game.new_object_id();
        let mut decision_maker = ScriptedProliferateDecisionMaker {
            responses: VecDeque::from([
                ProliferateResponse {
                    permanents: vec![creature_id],
                    players: Vec::new(),
                },
                ProliferateResponse {
                    permanents: Vec::new(),
                    players: vec![bob],
                },
            ]),
        };
        let mut ctx = ExecutionContext::new(source, alice, &mut decision_maker);

        let result = ProliferateEffect::new(2)
            .execute(&mut game, &mut ctx)
            .expect("proliferate twice should resolve");

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(2));
        assert_eq!(
            game.object(creature_id)
                .and_then(|obj| obj.counters.get(&CounterType::PlusOnePlusOne).copied()),
            Some(2)
        );
        assert_eq!(game.players[1].poison_counters, 3);
        assert_eq!(
            result
                .events
                .iter()
                .filter(|event| event.kind() == EventKind::KeywordAction)
                .count(),
            2
        );
    }
}
