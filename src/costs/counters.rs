//! Counter-related cost implementations.

use crate::cost::CostPaymentError;
use crate::costs::{CostContext, CostPayer, CostPaymentResult};
use crate::decision::FallbackStrategy;
use crate::decisions::{CounterRemovalSpec, DistributeSpec, make_decision_with_fallback};
use crate::filter::FilterContext;
use crate::game_state::{GameState, Target};
use crate::ids::ObjectId;
use crate::object::CounterType;
use crate::target::ObjectFilter;
use crate::zone::Zone;

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

        let distributed_total: u32 = distribution.iter().map(|(_, amount)| *amount).sum();
        if distributed_total != self.count {
            return Err(CostPaymentError::Other(
                "counter distribution must assign the full cost".to_string(),
            ));
        }

        let mut removed_total = 0u32;
        for (target, amount_for_target) in distribution {
            if amount_for_target == 0 {
                continue;
            }
            let Target::Object(object_id) = target else {
                continue;
            };

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
