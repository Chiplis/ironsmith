//! Counter-related cost implementations.

use crate::cost::CostPaymentError;
use crate::costs::{CostContext, CostPayer, CostPaymentResult};
use crate::decision::FallbackStrategy;
use crate::decisions::{
    CounterRemovalSpec, DistributeSpec, NumberSpec, make_decision_with_fallback,
};
use crate::filter::FilterContext;
use crate::game_state::{GameState, Target};
use crate::ids::ObjectId;
use crate::object::CounterType;
use crate::target::ObjectFilter;
use crate::zone::Zone;
use std::collections::HashMap;

/// A remove counters cost.
///
/// The player must remove counters from the source permanent.
#[derive(Debug, Clone, PartialEq)]
pub struct RemoveCountersCost {
    /// The type of counter to remove.
    pub counter_type: CounterType,
    /// The number of counters to remove.
    pub count: u32,
}

impl RemoveCountersCost {
    /// Create a new remove counters cost.
    pub fn new(counter_type: CounterType, count: u32) -> Self {
        Self {
            counter_type,
            count,
        }
    }

    /// Create a cost to remove +1/+1 counters.
    pub fn plus_one(count: u32) -> Self {
        Self::new(CounterType::PlusOnePlusOne, count)
    }

    /// Create a cost to remove -1/-1 counters.
    pub fn minus_one(count: u32) -> Self {
        Self::new(CounterType::MinusOneMinusOne, count)
    }

    /// Create a cost to remove charge counters.
    pub fn charge(count: u32) -> Self {
        Self::new(CounterType::Charge, count)
    }
}

impl CostPayer for RemoveCountersCost {
    fn can_pay(&self, game: &GameState, ctx: &CostContext) -> Result<(), CostPaymentError> {
        let source = game
            .object(ctx.source)
            .ok_or(CostPaymentError::SourceNotFound)?;

        let current = source
            .counters
            .get(&self.counter_type)
            .copied()
            .unwrap_or(0);
        if current < self.count {
            return Err(CostPaymentError::InsufficientCounters);
        }

        Ok(())
    }

    fn pay(
        &self,
        game: &mut GameState,
        ctx: &mut CostContext,
    ) -> Result<CostPaymentResult, CostPaymentError> {
        // Verify we can still pay
        self.can_pay(game, ctx)?;

        // Remove the counters
        if let Some(obj) = game.object_mut(ctx.source)
            && let Some(count) = obj.counters.get_mut(&self.counter_type)
        {
            *count = count.saturating_sub(self.count);
        }

        // Expose removed count for effects that reference
        // "for each counter removed this way".
        if ctx.x_value.is_none() {
            ctx.x_value = Some(self.count);
        }

        Ok(CostPaymentResult::Paid)
    }

    fn clone_box(&self) -> Box<dyn CostPayer> {
        Box::new(self.clone())
    }

    fn display(&self) -> String {
        let counter_name = format_counter_type(&self.counter_type);
        if self.count == 1 {
            format!("Remove a {} counter from ~", counter_name)
        } else {
            format!("Remove {} {} counters from ~", self.count, counter_name)
        }
    }

    fn is_remove_counters(&self) -> bool {
        true
    }

    // RemoveCountersCost is an immediate cost - no player choice needed
}

/// A remove-variable-counters cost from the source permanent.
///
/// Used for costs like:
/// - "Remove any number of charge counters from this artifact"
/// - "Remove X storage counters from this land"
#[derive(Debug, Clone, PartialEq)]
pub struct RemoveAnyCountersFromSourceCost {
    /// Optional counter type restriction.
    /// If `None`, any counter types can be removed.
    pub counter_type: Option<CounterType>,
    /// Whether display should use `X` instead of `any number`.
    pub display_x: bool,
}

impl RemoveAnyCountersFromSourceCost {
    /// Create a source-counter cost that removes any number of counters.
    pub fn any_number(counter_type: Option<CounterType>) -> Self {
        Self {
            counter_type,
            display_x: false,
        }
    }

    /// Create a source-counter cost that removes `X` counters.
    pub fn x(counter_type: Option<CounterType>) -> Self {
        Self {
            counter_type,
            display_x: true,
        }
    }

    fn max_removable(&self, game: &GameState, source: ObjectId) -> Result<u32, CostPaymentError> {
        let obj = game.object(source).ok_or(CostPaymentError::SourceNotFound)?;
        if obj.zone != Zone::Battlefield {
            return Err(CostPaymentError::SourceNotOnBattlefield);
        }

        let max = if let Some(counter_type) = self.counter_type {
            obj.counters.get(&counter_type).copied().unwrap_or(0)
        } else {
            obj.counters.values().copied().sum::<u32>()
        };

        Ok(max)
    }
}

impl CostPayer for RemoveAnyCountersFromSourceCost {
    fn can_pay(&self, game: &GameState, ctx: &CostContext) -> Result<(), CostPaymentError> {
        let _ = self.max_removable(game, ctx.source)?;
        Ok(())
    }

    fn pay(
        &self,
        game: &mut GameState,
        ctx: &mut CostContext,
    ) -> Result<CostPaymentResult, CostPaymentError> {
        self.can_pay(game, ctx)?;

        let max_removable = self.max_removable(game, ctx.source)?;
        let description = if self.display_x {
            "Choose X counters to remove"
        } else {
            "Choose counters to remove"
        };
        let to_remove = make_decision_with_fallback(
            game,
            &mut ctx.decision_maker,
            ctx.payer,
            Some(ctx.source),
            NumberSpec::up_to(ctx.source, max_removable, description),
            FallbackStrategy::Maximum,
        )
        .min(max_removable);

        let mut removed_total = 0u32;
        if let Some(counter_type) = self.counter_type {
            if to_remove > 0
                && let Some((removed, _event)) = game.remove_counters(
                    ctx.source,
                    counter_type,
                    to_remove,
                    Some(ctx.source),
                    Some(ctx.payer),
                )
            {
                removed_total = removed;
            }
        } else {
            let available_counters: Vec<(CounterType, u32)> = game
                .object(ctx.source)
                .map(|obj| {
                    obj.counters
                        .iter()
                        .filter(|(_, count)| **count > 0)
                        .map(|(counter_type, count)| (*counter_type, *count))
                        .collect()
                })
                .unwrap_or_default();

            let selections = make_decision_with_fallback(
                game,
                &mut ctx.decision_maker,
                ctx.payer,
                Some(ctx.source),
                CounterRemovalSpec::new(ctx.source, ctx.source, to_remove, available_counters),
                FallbackStrategy::Maximum,
            );

            for (counter_type, requested) in selections {
                if removed_total >= to_remove {
                    break;
                }
                let remaining = to_remove - removed_total;
                let to_remove_now = requested.min(remaining);
                if to_remove_now == 0 {
                    continue;
                }
                if let Some((removed, _event)) = game.remove_counters(
                    ctx.source,
                    counter_type,
                    to_remove_now,
                    Some(ctx.source),
                    Some(ctx.payer),
                ) {
                    removed_total += removed;
                }
            }
        }

        if removed_total != to_remove {
            return Err(CostPaymentError::InsufficientCounters);
        }

        // Expose removed count for effects that reference
        // "for each counter removed this way".
        if ctx.x_value.is_none() {
            ctx.x_value = Some(removed_total);
        }

        Ok(CostPaymentResult::Paid)
    }

    fn clone_box(&self) -> Box<dyn CostPayer> {
        Box::new(self.clone())
    }

    fn display(&self) -> String {
        let amount_text = if self.display_x { "X" } else { "any number of" };
        if let Some(counter_type) = self.counter_type {
            let counter_name = format_counter_type(&counter_type);
            format!("Remove {} {} counters from ~", amount_text, counter_name)
        } else {
            format!("Remove {} counters from ~", amount_text)
        }
    }

    fn is_remove_counters(&self) -> bool {
        true
    }
}

/// An add counters cost (e.g., cumulative upkeep).
///
/// The player adds counters to the source permanent as part of the cost.
#[derive(Debug, Clone, PartialEq)]
pub struct AddCountersCost {
    /// The type of counter to add.
    pub counter_type: CounterType,
    /// The number of counters to add.
    pub count: u32,
}

impl AddCountersCost {
    /// Create a new add counters cost.
    pub fn new(counter_type: CounterType, count: u32) -> Self {
        Self {
            counter_type,
            count,
        }
    }

    /// Create a cost to add age counters.
    pub fn age(count: u32) -> Self {
        Self::new(CounterType::Age, count)
    }

    /// Create a cost to add +1/+1 counters.
    pub fn plus_one(count: u32) -> Self {
        Self::new(CounterType::PlusOnePlusOne, count)
    }
}

impl CostPayer for AddCountersCost {
    fn can_pay(&self, game: &GameState, ctx: &CostContext) -> Result<(), CostPaymentError> {
        let source = game
            .object(ctx.source)
            .ok_or(CostPaymentError::SourceNotFound)?;

        // Can add counters if source is on battlefield
        if source.zone != Zone::Battlefield {
            return Err(CostPaymentError::SourceNotOnBattlefield);
        }

        Ok(())
    }

    fn pay(
        &self,
        game: &mut GameState,
        ctx: &mut CostContext,
    ) -> Result<CostPaymentResult, CostPaymentError> {
        // Verify we can still pay
        self.can_pay(game, ctx)?;

        // Add the counters
        if let Some(obj) = game.object_mut(ctx.source) {
            let count = obj.counters.entry(self.counter_type).or_insert(0);
            *count += self.count;
        }

        Ok(CostPaymentResult::Paid)
    }

    fn clone_box(&self) -> Box<dyn CostPayer> {
        Box::new(self.clone())
    }

    fn display(&self) -> String {
        let counter_name = format_counter_type(&self.counter_type);
        if self.count == 1 {
            format!("Put a {} counter on ~", counter_name)
        } else {
            format!("Put {} {} counters on ~", self.count, counter_name)
        }
    }
}

/// Remove a total number of counters from among permanents matching a filter.
///
/// This is used for costs like "Remove three counters from among creatures you control."
#[derive(Debug, Clone, PartialEq)]
pub struct RemoveAnyCountersAmongCost {
    /// Total counters to remove.
    pub count: u32,
    /// Which permanents can have counters removed.
    pub filter: ObjectFilter,
}

impl RemoveAnyCountersAmongCost {
    /// Create a new remove-counters-among cost.
    pub fn new(count: u32, filter: ObjectFilter) -> Self {
        Self { count, filter }
    }

    fn valid_targets(&self, game: &GameState, ctx: &CostContext) -> Vec<ObjectId> {
        let filter_ctx = FilterContext::new(ctx.payer)
            .with_source(ctx.source)
            .with_tagged_objects(&ctx.tagged_objects);

        game.battlefield
            .iter()
            .copied()
            .filter(|id| {
                let Some(obj) = game.object(*id) else {
                    return false;
                };
                self.filter.matches(obj, &filter_ctx, game)
                    && obj.counters.values().copied().sum::<u32>() > 0
            })
            .collect()
    }

    fn total_available(&self, game: &GameState, ctx: &CostContext) -> u32 {
        self.valid_targets(game, ctx)
            .iter()
            .filter_map(|id| game.object(*id))
            .map(|obj| obj.counters.values().copied().sum::<u32>())
            .sum()
    }
}

impl CostPayer for RemoveAnyCountersAmongCost {
    fn can_pay(&self, game: &GameState, ctx: &CostContext) -> Result<(), CostPaymentError> {
        if self.total_available(game, ctx) < self.count {
            return Err(CostPaymentError::InsufficientCounters);
        }
        Ok(())
    }

    fn pay(
        &self,
        game: &mut GameState,
        ctx: &mut CostContext,
    ) -> Result<CostPaymentResult, CostPaymentError> {
        self.can_pay(game, ctx)?;

        let valid_targets = self.valid_targets(game, ctx);
        let distribute_targets: Vec<Target> =
            valid_targets.iter().copied().map(Target::Object).collect();
        let distribution = make_decision_with_fallback(
            game,
            &mut ctx.decision_maker,
            ctx.payer,
            Some(ctx.source),
            DistributeSpec::counters(ctx.source, self.count, distribute_targets),
            FallbackStrategy::Maximum,
        );

        let mut allocations: HashMap<ObjectId, u32> = HashMap::new();
        for (target, amount) in distribution {
            if let Target::Object(object_id) = target {
                *allocations.entry(object_id).or_insert(0) += amount;
            }
        }

        let distributed_total: u32 = allocations.values().copied().sum();
        if distributed_total > self.count {
            return Err(CostPaymentError::Other(
                "counter distribution assigned too many counters".to_string(),
            ));
        }

        if distributed_total < self.count {
            let mut remaining = self.count - distributed_total;
            for object_id in &valid_targets {
                if remaining == 0 {
                    break;
                }
                let available_total = game
                    .object(*object_id)
                    .map(|obj| obj.counters.values().copied().sum::<u32>())
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
                return Err(CostPaymentError::InsufficientCounters);
            }
        }

        let mut removed_total = 0u32;
        for (object_id, amount_for_target) in allocations {
            if amount_for_target == 0 {
                continue;
            }

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
                return Err(CostPaymentError::InsufficientCounters);
            }

            let selections = make_decision_with_fallback(
                game,
                &mut ctx.decision_maker,
                ctx.payer,
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
                if let Some((removed, _event)) = game.remove_counters(
                    object_id,
                    counter_type,
                    to_remove,
                    Some(ctx.source),
                    Some(ctx.payer),
                ) {
                    removed_from_target += removed;
                    removed_total += removed;
                }
            }

            if removed_from_target != amount_for_target {
                return Err(CostPaymentError::InsufficientCounters);
            }
        }

        if removed_total != self.count {
            return Err(CostPaymentError::InsufficientCounters);
        }

        // Expose the actual removed count for effects that reference
        // "for each counter removed this way".
        if ctx.x_value.is_none() {
            ctx.x_value = Some(removed_total);
        }

        Ok(CostPaymentResult::Paid)
    }

    fn clone_box(&self) -> Box<dyn CostPayer> {
        Box::new(self.clone())
    }

    fn display(&self) -> String {
        if self.count == 1 {
            "Remove a counter from among permanents you control".to_string()
        } else {
            format!(
                "Remove {} counters from among permanents you control",
                self.count
            )
        }
    }

    fn is_remove_counters(&self) -> bool {
        true
    }
}

/// Format a counter type for display.
fn format_counter_type(counter_type: &CounterType) -> &'static str {
    match counter_type {
        CounterType::PlusOnePlusOne => "+1/+1",
        CounterType::MinusOneMinusOne => "-1/-1",
        CounterType::Loyalty => "loyalty",
        CounterType::Charge => "charge",
        CounterType::Storage => "storage",
        CounterType::Age => "age",
        CounterType::Time => "time",
        CounterType::Level => "level",
        CounterType::Lore => "lore",
        CounterType::Page => "page",
        CounterType::Quest => "quest",
        CounterType::Verse => "verse",
        CounterType::Fade => "fade",
        CounterType::Fuse => "fuse",
        CounterType::Trap => "trap",
        CounterType::Shield => "shield",
        CounterType::Flood => "flood",
        CounterType::Blood => "blood",
        CounterType::Study => "study",
        CounterType::Knowledge => "knowledge",
        CounterType::Muster => "muster",
        CounterType::Strife => "strife",
        CounterType::Slumber => "slumber",
        CounterType::Filibuster => "filibuster",
        CounterType::Pressure => "pressure",
        CounterType::Divinity => "divinity",
        CounterType::Gem => "gem",
        CounterType::Isolation => "isolation",
        CounterType::Doom => "doom",
        CounterType::Incarnation => "incarnation",
        CounterType::Depletion => "depletion",
        CounterType::Music => "music",
        CounterType::Ice => "ice",
        CounterType::Wind => "wind",
        CounterType::Luck => "luck",
        // Match any other counter types with a generic name
        _ => "counter",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::ids::{CardId, PlayerId};
    use crate::types::CardType;

    fn create_test_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    fn simple_card(name: &str, id: u32) -> crate::card::Card {
        CardBuilder::new(CardId::from_raw(id), name)
            .card_types(vec![CardType::Creature])
            .build()
    }

    // === RemoveCountersCost tests ===

    #[test]
    fn test_remove_counters_display() {
        assert_eq!(
            RemoveCountersCost::plus_one(1).display(),
            "Remove a +1/+1 counter from ~"
        );
        assert_eq!(
            RemoveCountersCost::charge(3).display(),
            "Remove 3 charge counters from ~"
        );
    }

    #[test]
    fn test_remove_counters_can_pay_with_counters() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);

        let card = simple_card("Card", 1);
        let card_id = game.create_object_from_card(&card, alice, Zone::Battlefield);

        // Add counters
        if let Some(obj) = game.object_mut(card_id) {
            obj.counters.insert(CounterType::PlusOnePlusOne, 3);
        }

        let cost = RemoveCountersCost::plus_one(2);
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let ctx = CostContext::new(card_id, alice, &mut dm);

        assert!(cost.can_pay(&game, &ctx).is_ok());
    }

    #[test]
    fn test_remove_counters_cannot_pay_insufficient() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);

        let card = simple_card("Card", 1);
        let card_id = game.create_object_from_card(&card, alice, Zone::Battlefield);

        // Add only 1 counter
        if let Some(obj) = game.object_mut(card_id) {
            obj.counters.insert(CounterType::PlusOnePlusOne, 1);
        }

        let cost = RemoveCountersCost::plus_one(2);
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let ctx = CostContext::new(card_id, alice, &mut dm);

        assert_eq!(
            cost.can_pay(&game, &ctx),
            Err(CostPaymentError::InsufficientCounters)
        );
    }

    #[test]
    fn test_remove_counters_pay_success() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);

        let card = simple_card("Card", 1);
        let card_id = game.create_object_from_card(&card, alice, Zone::Battlefield);

        // Add counters
        if let Some(obj) = game.object_mut(card_id) {
            obj.counters.insert(CounterType::PlusOnePlusOne, 3);
        }

        let cost = RemoveCountersCost::plus_one(2);
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let mut ctx = CostContext::new(card_id, alice, &mut dm);

        let result = cost.pay(&mut game, &mut ctx);
        assert_eq!(result, Ok(CostPaymentResult::Paid));

        // Should have 1 counter left
        let obj = game.object(card_id).unwrap();
        assert_eq!(obj.counters.get(&CounterType::PlusOnePlusOne), Some(&1));
    }

    #[test]
    fn test_remove_any_counters_from_source_display() {
        assert_eq!(
            RemoveAnyCountersFromSourceCost::any_number(Some(CounterType::Charge)).display(),
            "Remove any number of charge counters from ~"
        );
        assert_eq!(
            RemoveAnyCountersFromSourceCost::x(Some(CounterType::Storage)).display(),
            "Remove X storage counters from ~"
        );
    }

    #[test]
    fn test_remove_any_counters_from_source_pay_sets_x() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);

        let card = simple_card("Card", 1);
        let card_id = game.create_object_from_card(&card, alice, Zone::Battlefield);

        if let Some(obj) = game.object_mut(card_id) {
            obj.counters.insert(CounterType::Charge, 3);
        }

        let cost = RemoveAnyCountersFromSourceCost::any_number(Some(CounterType::Charge));
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let mut ctx = CostContext::new(card_id, alice, &mut dm);

        let result = cost.pay(&mut game, &mut ctx);
        assert_eq!(result, Ok(CostPaymentResult::Paid));
        assert_eq!(ctx.x_value, Some(3), "removed amount should propagate to x_value");

        let obj = game.object(card_id).expect("source should still exist");
        let remaining = obj.counters.get(&CounterType::Charge).copied().unwrap_or(0);
        assert_eq!(remaining, 0, "all charge counters should be removed");
    }

    // === AddCountersCost tests ===

    #[test]
    fn test_add_counters_display() {
        assert_eq!(AddCountersCost::age(1).display(), "Put a age counter on ~");
        assert_eq!(
            AddCountersCost::plus_one(2).display(),
            "Put 2 +1/+1 counters on ~"
        );
    }

    #[test]
    fn test_add_counters_can_pay_on_battlefield() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);

        let card = simple_card("Card", 1);
        let card_id = game.create_object_from_card(&card, alice, Zone::Battlefield);

        let cost = AddCountersCost::age(1);
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let ctx = CostContext::new(card_id, alice, &mut dm);

        assert!(cost.can_pay(&game, &ctx).is_ok());
    }

    #[test]
    fn test_add_counters_cannot_pay_in_graveyard() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);

        let card = simple_card("Card", 1);
        let card_id = game.create_object_from_card(&card, alice, Zone::Graveyard);

        let cost = AddCountersCost::age(1);
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let ctx = CostContext::new(card_id, alice, &mut dm);

        assert_eq!(
            cost.can_pay(&game, &ctx),
            Err(CostPaymentError::SourceNotOnBattlefield)
        );
    }

    #[test]
    fn test_add_counters_pay_success() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);

        let card = simple_card("Card", 1);
        let card_id = game.create_object_from_card(&card, alice, Zone::Battlefield);

        let cost = AddCountersCost::age(2);
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let mut ctx = CostContext::new(card_id, alice, &mut dm);

        let result = cost.pay(&mut game, &mut ctx);
        assert_eq!(result, Ok(CostPaymentResult::Paid));

        // Should have 2 age counters
        let obj = game.object(card_id).unwrap();
        assert_eq!(obj.counters.get(&CounterType::Age), Some(&2));
    }

    #[test]
    fn test_counters_cost_clone_box() {
        let cost = RemoveCountersCost::charge(1);
        let cloned = cost.clone_box();
        assert!(format!("{:?}", cloned).contains("RemoveCountersCost"));
    }

    #[test]
    fn test_remove_any_counters_among_can_pay_with_total_across_permanents() {
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

        let cost = RemoveAnyCountersAmongCost::new(3, ObjectFilter::creature().you_control());
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let ctx = CostContext::new(a_id, alice, &mut dm);
        assert!(cost.can_pay(&game, &ctx).is_ok());
    }

    #[test]
    fn test_remove_any_counters_among_pay_removes_counters() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);

        let card = simple_card("A", 1);
        let card_id = game.create_object_from_card(&card, alice, Zone::Battlefield);
        if let Some(obj) = game.object_mut(card_id) {
            obj.counters.insert(CounterType::PlusOnePlusOne, 3);
        }

        let cost = RemoveAnyCountersAmongCost::new(2, ObjectFilter::creature().you_control());
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let mut ctx = CostContext::new(card_id, alice, &mut dm);

        let result = cost.pay(&mut game, &mut ctx);
        assert_eq!(result, Ok(CostPaymentResult::Paid));
        assert_eq!(game.counter_count(card_id, CounterType::PlusOnePlusOne), 1);
    }
}
