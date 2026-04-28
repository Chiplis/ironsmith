//! Effect for removing counters from among matching permanents.

use crate::decision::FallbackStrategy;
use crate::decisions::{
    CounterRemovalSpec, DistributeSpec, NumberSpec, make_decision_with_fallback,
};
use crate::effect::EffectOutcome;
use crate::effects::{CostExecutableEffect, CostValidationError, EffectExecutor};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::filter::ObjectFilterExt as _;
use crate::filter::{FilterContext, ObjectFilter, PlayerFilter};
use crate::game_state::{GameState, Target};
use crate::ids::{ObjectId, PlayerId};
use crate::object::CounterType;
use crate::snapshot::ObjectSnapshot;
use crate::tag::TagKey;
use crate::target::ChooseSpec;
use crate::types::CardType;
use crate::zone::Zone;
pub use ironsmith_core::RemoveAnyCountersAmongEffect;
use std::collections::HashMap;

/// Remove a total number of counters from among permanents matching a filter.
///
/// When used as a cost, this is wrapped by `CostEffect` via `Cost::effect(...)`.
pub(crate) fn valid_targets_with_tags(
    effect: &RemoveAnyCountersAmongEffect,
    game: &GameState,
    source: ObjectId,
    payer: PlayerId,
    tagged_objects: &HashMap<TagKey, Vec<ObjectSnapshot>>,
) -> Vec<ObjectId> {
    let filter_ctx = FilterContext::new(payer)
        .with_source(source)
        .with_tagged_objects(tagged_objects);

    game.battlefield
        .iter()
        .copied()
        .filter(|id| {
            let Some(obj) = game.object(*id) else {
                return false;
            };
            let available = if let Some(counter_type) = effect.counter_type {
                obj.counters.get(&counter_type).copied().unwrap_or(0)
            } else {
                obj.counters.values().copied().sum::<u32>()
            };
            effect.filter.matches(obj, &filter_ctx, game) && available > 0
        })
        .collect()
}

#[allow(dead_code)]
pub fn valid_targets(
    effect: &RemoveAnyCountersAmongEffect,
    game: &GameState,
    source: ObjectId,
    payer: PlayerId,
) -> Vec<ObjectId> {
    valid_targets_with_tags(effect, game, source, payer, &HashMap::new())
}

fn total_available_with_tags(
    effect: &RemoveAnyCountersAmongEffect,
    game: &GameState,
    source: ObjectId,
    payer: PlayerId,
    tagged_objects: &HashMap<TagKey, Vec<ObjectSnapshot>>,
) -> u32 {
    valid_targets_with_tags(effect, game, source, payer, tagged_objects)
        .iter()
        .filter_map(|id| game.object(*id))
        .map(|obj| {
            if let Some(counter_type) = effect.counter_type {
                obj.counters.get(&counter_type).copied().unwrap_or(0)
            } else {
                obj.counters.values().copied().sum::<u32>()
            }
        })
        .sum()
}

fn total_available(
    effect: &RemoveAnyCountersAmongEffect,
    game: &GameState,
    source: ObjectId,
    payer: PlayerId,
) -> u32 {
    total_available_with_tags(effect, game, source, payer, &HashMap::new())
}

pub fn cost_display(effect: &RemoveAnyCountersAmongEffect) -> String {
    let target_phrase_single = remove_counters_target_phrase(&effect.filter, false);
    let target_phrase_plural = remove_counters_target_phrase(&effect.filter, true);
    if effect.dynamic_count {
        let amount_text = if effect.display_x {
            "X".to_string()
        } else if effect.min_count > 0 {
            "one or more".to_string()
        } else {
            "any number of".to_string()
        };
        return match effect.counter_type {
            Some(counter_type) => format!(
                "Remove {amount_text} {} counters from among {}",
                counter_type.description(),
                target_phrase_plural
            ),
            None => format!("Remove {amount_text} counters from among {target_phrase_plural}"),
        };
    }
    match (effect.count, effect.counter_type) {
        (1, Some(counter_type)) => {
            let counter_name = counter_type.description();
            format!(
                "Remove {} {} counter from {}",
                counter_article(&counter_name),
                counter_name,
                target_phrase_single
            )
        }
        (count, Some(counter_type)) => {
            let counter_name = counter_type.description();
            format!(
                "Remove {} {} counters from among {}",
                count, counter_name, target_phrase_plural
            )
        }
        (1, None) => format!("Remove a counter from {}", target_phrase_single),
        (count, None) => {
            format!(
                "Remove {} counters from among {}",
                count, target_phrase_plural
            )
        }
    }
}

impl EffectExecutor for RemoveAnyCountersAmongEffect {
    fn as_cost_executable(&self) -> Option<&dyn CostExecutableEffect> {
        Some(self)
    }

    fn decision_related_object_specs(&self) -> Vec<ChooseSpec> {
        vec![ChooseSpec::All(self.filter.clone())]
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let total_available =
            total_available_with_tags(self, game, ctx.source, ctx.controller, &ctx.tagged_objects);
        if total_available < self.min_count {
            return Ok(EffectOutcome::impossible());
        }
        let requested_count = if self.dynamic_count {
            let max_count = self.count.min(total_available);
            if max_count < self.min_count {
                return Ok(EffectOutcome::impossible());
            }
            make_decision_with_fallback(
                game,
                &mut ctx.decision_maker,
                ctx.controller,
                Some(ctx.source),
                NumberSpec::range(ctx.source, self.min_count, max_count, "counters to remove"),
                FallbackStrategy::Maximum,
            )
            .clamp(self.min_count, max_count)
        } else {
            self.count
        };

        let valid_targets =
            valid_targets_with_tags(self, game, ctx.source, ctx.controller, &ctx.tagged_objects);
        let distribute_targets: Vec<Target> =
            valid_targets.iter().copied().map(Target::Object).collect();
        let distribution = make_decision_with_fallback(
            game,
            &mut ctx.decision_maker,
            ctx.controller,
            Some(ctx.source),
            DistributeSpec::counters(ctx.source, requested_count, distribute_targets),
            FallbackStrategy::Maximum,
        );

        let mut allocations: HashMap<ObjectId, u32> = HashMap::new();
        for (target, amount) in distribution {
            if let Target::Object(object_id) = target {
                *allocations.entry(object_id).or_insert(0) += amount;
            }
        }

        let distributed_total: u32 = allocations.values().copied().sum();
        if distributed_total > requested_count {
            return Ok(EffectOutcome::impossible());
        }

        if distributed_total < requested_count {
            let mut remaining = requested_count - distributed_total;
            for object_id in &valid_targets {
                if remaining == 0 {
                    break;
                }
                let available_total = game
                    .object(*object_id)
                    .map(|obj| {
                        if let Some(counter_type) = self.counter_type {
                            obj.counters.get(&counter_type).copied().unwrap_or(0)
                        } else {
                            obj.counters.values().copied().sum::<u32>()
                        }
                    })
                    .unwrap_or(0);
                let already_allocated = allocations.get(object_id).copied().unwrap_or(0);
                let free_capacity = available_total.saturating_sub(already_allocated);
                if free_capacity == 0 {
                    continue;
                }
                let add = remaining.min(free_capacity);
                *allocations.entry(*object_id).or_insert(0) += add;
                remaining -= add;
            }
            if remaining > 0 {
                return Ok(EffectOutcome::impossible());
            }
        }

        let mut removed_total = 0u32;
        let mut outcome = EffectOutcome::count(0);
        for (object_id, amount_for_target) in allocations {
            if amount_for_target == 0 {
                continue;
            }

            let removed_from_target = if let Some(counter_type) = self.counter_type {
                let available_total = game
                    .object(object_id)
                    .and_then(|obj| obj.counters.get(&counter_type).copied())
                    .unwrap_or(0);
                if available_total < amount_for_target {
                    return Ok(EffectOutcome::impossible());
                }
                match game.remove_counters(
                    object_id,
                    counter_type,
                    amount_for_target,
                    Some(ctx.source),
                    Some(ctx.controller),
                ) {
                    Some((removed, event)) => {
                        outcome = outcome.with_event(event);
                        removed
                    }
                    None => 0,
                }
            } else {
                let available_counters: Vec<(CounterType, u32)> = game
                    .object(object_id)
                    .map(|obj| {
                        obj.counters
                            .iter()
                            .filter(|(_, count)| **count > 0)
                            .map(|(counter_type, count)| (*counter_type, *count))
                            .collect()
                    })
                    .unwrap_or_default();
                let available_total: u32 = available_counters.iter().map(|(_, count)| *count).sum();
                if available_total < amount_for_target {
                    return Ok(EffectOutcome::impossible());
                }

                let selections = make_decision_with_fallback(
                    game,
                    &mut ctx.decision_maker,
                    ctx.controller,
                    Some(ctx.source),
                    CounterRemovalSpec::new(
                        ctx.source,
                        object_id,
                        amount_for_target,
                        available_counters,
                    ),
                    FallbackStrategy::Maximum,
                );

                let mut removed_from_target = 0u32;
                for (counter_type, requested) in selections {
                    if removed_from_target >= amount_for_target {
                        break;
                    }
                    let remaining = amount_for_target - removed_from_target;
                    let to_remove = requested.min(remaining);
                    if to_remove == 0 {
                        continue;
                    }
                    if let Some((removed, event)) = game.remove_counters(
                        object_id,
                        counter_type,
                        to_remove,
                        Some(ctx.source),
                        Some(ctx.controller),
                    ) {
                        outcome = outcome.with_event(event);
                        removed_from_target += removed;
                    }
                }
                removed_from_target
            };

            removed_total += removed_from_target;
            if removed_from_target != amount_for_target {
                return Ok(EffectOutcome::impossible());
            }
        }

        if removed_total != requested_count {
            return Ok(EffectOutcome::impossible());
        }

        outcome.set_value(crate::effect::OutcomeValue::Count(removed_total as i32));
        Ok(outcome)
    }

    fn cost_description(&self) -> Option<String> {
        Some(cost_display(self))
    }
}

impl CostExecutableEffect for RemoveAnyCountersAmongEffect {
    fn can_execute_as_cost(
        &self,
        game: &GameState,
        source: ObjectId,
        controller: PlayerId,
    ) -> Result<(), CostValidationError> {
        if total_available(self, game, source, controller) < self.min_count {
            return Err(CostValidationError::Other(
                "not enough counters".to_string(),
            ));
        }
        Ok(())
    }
}

fn counter_article(counter_name: &str) -> &'static str {
    let starts_with_vowel = counter_name
        .chars()
        .next()
        .map(|ch| matches!(ch.to_ascii_lowercase(), 'a' | 'e' | 'i' | 'o' | 'u'))
        .unwrap_or(false);
    if starts_with_vowel { "an" } else { "a" }
}

fn remove_counters_target_phrase(filter: &ObjectFilter, plural: bool) -> String {
    if is_simple_permanent_you_control_filter(filter) {
        return if plural {
            "permanents you control".to_string()
        } else {
            "a permanent you control".to_string()
        };
    }

    if is_simple_nonland_permanent_you_control_filter(filter) {
        return if plural {
            "nonland permanents you control".to_string()
        } else {
            "a nonland permanent you control".to_string()
        };
    }

    if let Some(card_type) = simple_you_controlled_battlefield_card_type(filter) {
        let noun = if plural {
            card_type.plural_name()
        } else {
            card_type.name()
        };
        return if plural {
            format!("{noun} you control")
        } else {
            format!("a {noun} you control")
        };
    }

    let mut noun = if filter.card_types.is_empty() {
        if plural {
            "permanents".to_string()
        } else {
            "a permanent".to_string()
        }
    } else {
        let joined = filter
            .card_types
            .iter()
            .map(|card_type| card_type.name().to_ascii_lowercase())
            .collect::<Vec<_>>()
            .join(" or ");
        if plural {
            format!("{joined}s")
        } else {
            format!("a {joined}")
        }
    };

    if filter.controller == Some(PlayerFilter::You) {
        noun.push_str(" you control");
    }

    noun
}

fn is_simple_permanent_you_control_filter(filter: &ObjectFilter) -> bool {
    let base = ObjectFilter::permanent().you_control();
    if *filter == base {
        return true;
    }

    let mut expanded = base;
    expanded.card_types = vec![
        CardType::Artifact,
        CardType::Creature,
        CardType::Enchantment,
        CardType::Land,
        CardType::Planeswalker,
        CardType::Battle,
    ];
    *filter == expanded
}

fn simple_you_controlled_battlefield_card_type(filter: &ObjectFilter) -> Option<CardType> {
    if filter.card_types.len() != 1 {
        return None;
    }

    let mut expected = ObjectFilter::default();
    expected.zone = Some(Zone::Battlefield);
    expected.controller = Some(PlayerFilter::You);
    expected.card_types = vec![filter.card_types[0]];
    if *filter == expected {
        Some(filter.card_types[0])
    } else {
        None
    }
}

fn is_simple_nonland_permanent_you_control_filter(filter: &ObjectFilter) -> bool {
    let mut expected = ObjectFilter::default();
    expected.zone = Some(Zone::Battlefield);
    expected.controller = Some(PlayerFilter::You);
    expected.card_types = vec![
        CardType::Artifact,
        CardType::Creature,
        CardType::Enchantment,
        CardType::Land,
        CardType::Planeswalker,
        CardType::Battle,
    ];
    expected.excluded_card_types = vec![CardType::Land];
    *filter == expected
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::costs::{Cost, CostContext};
    use crate::ids::{CardId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::types::CardType;
    use crate::zone::Zone;

    fn create_test_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn simple_card(name: &str, raw_id: u32) -> crate::card::Card {
        CardBuilder::new(CardId::from_raw(raw_id), name)
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Green],
            ]))
            .card_types(vec![CardType::Creature])
            .build()
    }

    #[test]
    fn can_pay_with_total_across_permanents() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);

        let card_a = simple_card("A", 1);
        let a_id = game.create_object_from_card(&card_a, alice, Zone::Battlefield);
        let card_b = simple_card("B", 2);
        let b_id = game.create_object_from_card(&card_b, alice, Zone::Battlefield);

        if let Some(obj) = game.object_mut(a_id) {
            obj.counters.insert(CounterType::PlusOnePlusOne, 1);
        }
        if let Some(obj) = game.object_mut(b_id) {
            obj.counters.insert(CounterType::Charge, 2);
        }

        let cost = Cost::effect(RemoveAnyCountersAmongEffect::new(
            3,
            ObjectFilter::creature().you_control(),
        ));
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let ctx = CostContext::new(a_id, alice, &mut dm);
        assert!(cost.can_pay(&game, &ctx).is_ok());
    }

    #[test]
    fn pay_removes_counters() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);

        let card = simple_card("A", 1);
        let card_id = game.create_object_from_card(&card, alice, Zone::Battlefield);
        if let Some(obj) = game.object_mut(card_id) {
            obj.counters.insert(CounterType::PlusOnePlusOne, 3);
        }

        let cost = Cost::effect(RemoveAnyCountersAmongEffect::new(
            2,
            ObjectFilter::creature().you_control(),
        ));
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let mut ctx = CostContext::new(card_id, alice, &mut dm);

        let result = cost.pay(&mut game, &mut ctx);
        assert_eq!(result, Ok(crate::costs::CostPaymentResult::Paid));
        assert_eq!(game.counter_count(card_id, CounterType::PlusOnePlusOne), 1);
        assert_eq!(ctx.x_value, Some(2));
    }

    #[test]
    fn dynamic_cost_removes_chosen_counter_total_and_sets_x() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);

        let card_a = simple_card("A", 1);
        let a_id = game.create_object_from_card(&card_a, alice, Zone::Battlefield);
        let card_b = simple_card("B", 2);
        let b_id = game.create_object_from_card(&card_b, alice, Zone::Battlefield);
        if let Some(obj) = game.object_mut(a_id) {
            obj.counters.insert(CounterType::PlusOnePlusOne, 2);
        }
        if let Some(obj) = game.object_mut(b_id) {
            obj.counters.insert(CounterType::PlusOnePlusOne, 1);
        }

        let cost = Cost::effect(
            RemoveAnyCountersAmongEffect::dynamic(
                1,
                u32::MAX / 4,
                ObjectFilter::creature().you_control(),
                false,
            )
            .with_counter_type(Some(CounterType::PlusOnePlusOne)),
        );
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let mut ctx = CostContext::new(a_id, alice, &mut dm);

        let result = cost.pay(&mut game, &mut ctx);
        assert_eq!(result, Ok(crate::costs::CostPaymentResult::Paid));
        assert_eq!(game.counter_count(a_id, CounterType::PlusOnePlusOne), 0);
        assert_eq!(game.counter_count(b_id, CounterType::PlusOnePlusOne), 0);
        assert_eq!(ctx.x_value, Some(3));
    }

    #[test]
    fn display_permanent_you_control_singular() {
        let effect = RemoveAnyCountersAmongEffect::new(1, ObjectFilter::permanent().you_control());
        assert_eq!(
            cost_display(&effect),
            "Remove a counter from a permanent you control"
        );
    }

    #[test]
    fn display_dynamic_one_or_more_typed_counters_among_creatures() {
        let effect = RemoveAnyCountersAmongEffect::dynamic(
            1,
            u32::MAX / 4,
            ObjectFilter::creature().you_control(),
            false,
        )
        .with_counter_type(Some(CounterType::PlusOnePlusOne));
        assert_eq!(
            cost_display(&effect),
            "Remove one or more +1/+1 counters from among creatures you control"
        );
    }

    #[test]
    fn typed_counters_cannot_pay_without_type() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);

        let card = simple_card("A", 1);
        let card_id = game.create_object_from_card(&card, alice, Zone::Battlefield);
        if let Some(obj) = game.object_mut(card_id) {
            obj.counters.insert(CounterType::Charge, 2);
        }

        let cost = Cost::effect(
            RemoveAnyCountersAmongEffect::new(1, ObjectFilter::creature().you_control())
                .with_counter_type(Some(CounterType::PlusOnePlusOne)),
        );
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let ctx = CostContext::new(card_id, alice, &mut dm);
        assert_eq!(
            cost.can_pay(&game, &ctx),
            Err(crate::cost::CostPaymentError::Other(
                "not enough counters".to_string()
            ))
        );
    }

    #[test]
    fn typed_counters_pay_removes_only_typed_counters() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);

        let card = simple_card("A", 1);
        let card_id = game.create_object_from_card(&card, alice, Zone::Battlefield);
        if let Some(obj) = game.object_mut(card_id) {
            obj.counters.insert(CounterType::PlusOnePlusOne, 3);
            obj.counters.insert(CounterType::Charge, 2);
        }

        let cost = Cost::effect(
            RemoveAnyCountersAmongEffect::new(2, ObjectFilter::creature().you_control())
                .with_counter_type(Some(CounterType::PlusOnePlusOne)),
        );
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let mut ctx = CostContext::new(card_id, alice, &mut dm);

        let result = cost.pay(&mut game, &mut ctx);
        assert_eq!(result, Ok(crate::costs::CostPaymentResult::Paid));
        assert_eq!(game.counter_count(card_id, CounterType::PlusOnePlusOne), 1);
        assert_eq!(game.counter_count(card_id, CounterType::Charge), 2);
    }
}
