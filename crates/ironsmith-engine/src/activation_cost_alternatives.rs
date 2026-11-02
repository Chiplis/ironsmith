//! Scoped alternatives to whole activation costs and their selected-use receipts.
use crate::continuous::AbilityOrigin;
use crate::cost::{CostPaymentError, TotalCost};
use crate::costs::{Cost, CostContext, CostPayer, CostPaymentResult};
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::{GameState, ObjectId, Zone};

#[derive(Debug, Clone, PartialEq)]
pub struct ActivationCostAlternative {
    pub source: ObjectId,
    pub status: String,
    pub replacement_cost: crate::mana::ManaCost,
    pub max_uses_per_ability_each_turn: u32,
    pub nonloyalty_only: bool,
}
pub type ActivationAlternativeUseKey = crate::action_use_receipts::ActionUseKey;
#[derive(Debug, Clone)]
struct AlternativeUseReceipt {
    key: ActivationAlternativeUseKey,
    limit: u32,
}
impl AlternativeUseReceipt {
    fn used(&self, game: &GameState) -> u32 {
        crate::action_use_receipts::used_this_turn(game, &self.key)
    }
}
impl CostPayer for AlternativeUseReceipt {
    fn can_pay(&self, game: &GameState, _: &CostContext) -> Result<(), CostPaymentError> {
        if self.used(game) < self.limit {
            Ok(())
        } else {
            Err(CostPaymentError::Other(
                "This ability's alternative activation cost has already been used this turn".into(),
            ))
        }
    }
    fn pay(
        &self,
        game: &mut GameState,
        ctx: &mut CostContext,
    ) -> Result<CostPaymentResult, CostPaymentError> {
        self.can_pay(game, ctx)?;
        crate::action_use_receipts::consume_this_turn(game, &self.key, self.limit)
            .map_err(CostPaymentError::Other)?;
        Ok(CostPaymentResult::Paid)
    }
    fn display(&self) -> String {
        if self.limit == 1 {
            "Use this ability's once-per-turn alternative cost".into()
        } else {
            format!(
                "Use this ability's alternative cost ({} uses each turn)",
                self.limit
            )
        }
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
fn contains_receipt(cost: &TotalCost) -> bool {
    match cost.kind() {
        ironsmith_core::TotalCostKind::All(costs) => costs
            .iter()
            .any(|cost| cost.downcast_ref::<AlternativeUseReceipt>().is_some()),
        ironsmith_core::TotalCostKind::OneOf(branches) => branches.iter().any(contains_receipt),
    }
}

/// A consumer rider is shared by all original prices. Component rules combine,
/// while mutually exclusive prices contribute only their common obligations.
fn consumer_spending_rules(original: &TotalCost) -> Vec<ironsmith_core::mana::ManaSpendingRestriction> {
    match original.kind() {
        ironsmith_core::TotalCostKind::All(costs) => {
            let mut rules=Vec::new();
            for cost in costs {
                for mana in cost.mana_cost_ref().into_iter().chain(cost.dynamic_mana_cost_ref().map(|dynamic| &dynamic.base)) {
                    for rule in mana.spending_restrictions() {
                        if !rules.contains(rule) {rules.push(rule.clone());}
                    }
                }
            }
            rules
        },
        ironsmith_core::TotalCostKind::OneOf(branches) => {
            let mut branches=branches.iter();
            let Some(first)=branches.next() else{return Vec::new()};
            let mut common=consumer_spending_rules(first);
            for branch in branches {
                let rules=consumer_spending_rules(branch);
                common.retain(|rule| rules.contains(rule));
            }
            common
        },
    }
}
fn inherit_consumer_spending_rules(original:&TotalCost,mut replacement:crate::mana::ManaCost)->crate::mana::ManaCost {
    for rule in consumer_spending_rules(original) {
        replacement=replacement.with_spending_restriction(rule);
    }
    replacement
}

/// Each eligible grant yields one independent complete cost branch. Already
/// priced branches retain their receipt instead of acquiring it a second time.
pub(crate) fn alternative_branches(
    game: &GameState,
    source: ObjectId,
    facts: &crate::decision::ActivationCostAbility,
    original: &TotalCost,
    view: &crate::derived_view::DerivedGameView<'_>,
) -> Vec<(TotalCost, Cost)> {
    if contains_receipt(original)
        || !game
            .object(source)
            .is_some_and(|object| object.zone == Zone::Battlefield)
        || game.is_phased_out(source)
    {
        return Vec::new();
    }
    let Some(index) = facts.ability_index else {
        return Vec::new();
    };
    let origin = view
        .calculated_characteristics(source)
        .and_then(|chars| chars.abilities.origin(index).cloned())
        .unwrap_or(AbilityOrigin::Printed(index));
    let definition = view.calculated_characteristics(source).and_then(|chars| {
        chars
            .abilities
            .get(index)
            .and_then(|ability| match &ability.kind {
                crate::ability::AbilityKind::Activated(activated) => {
                    activated.effects.activation_definition
                }
                _ => None,
            })
    });
    game.effect_store
        .activation_cost_alternatives
        .iter()
        .filter_map(|grant| {
            if grant.source != source
                || (grant.nonloyalty_only && facts.loyalty_ability)
                || !game.has_custom_status(source, &grant.status)
            {
                return None;
            }
            let receipt = AlternativeUseReceipt {
                key: ActivationAlternativeUseKey {
                    source,
                    ability: Some(origin.clone()),
                    definition,
                    group: grant.status.clone(),
                },
                limit: grant.max_uses_per_ability_each_turn,
            };
            if receipt.used(game) >= receipt.limit {
                return None;
            }
            Some((
                TotalCost::mana(inherit_consumer_spending_rules(original,grant.replacement_cost.clone())),
                Cost::new(receipt),
            ))
        })
        .collect()
}

/// Capture unpriced complete branches once at activation announcement. The
/// resulting receipt-bearing tree is immutable for later target/X repricing.
pub(crate) fn unpriced_cost(
    game: &GameState,
    source: ObjectId,
    facts: &crate::decision::ActivationCostAbility,
    original: &TotalCost,
    view: &crate::derived_view::DerivedGameView<'_>,
) -> TotalCost {
    let grants = alternative_branches(game, source, &facts, original, view);
    if grants.is_empty() {
        return original.clone();
    }
    let mut branches = original
        .as_one_of()
        .map(|branches| branches.to_vec())
        .unwrap_or_else(|| vec![original.clone()]);
    for (cost, receipt) in grants {
        let mut components = cost.costs().to_vec();
        components.push(receipt);
        branches.push(TotalCost::from_costs(components));
    }
    TotalCost::one_of(branches)
}

pub(crate) fn available_uses(
    game: &GameState,
    source: ObjectId,
    facts: &crate::decision::ActivationCostAbility,
    original: &TotalCost,
    view: &crate::derived_view::DerivedGameView<'_>,
) -> u32 {
    alternative_branches(game, source, &facts, original, view)
        .iter()
        .filter_map(|(_, cost)| cost.downcast_ref::<AlternativeUseReceipt>())
        .map(|receipt| receipt.limit.saturating_sub(receipt.used(game)))
        .sum()
}

/// Used only for early cost-resource gates. Timing, zone, targeting and explicit
/// activation prohibitions remain with the ordinary legality owner.
pub(crate) fn has_available_alternative(
    game: &GameState,
    source: ObjectId,
    facts: &crate::decision::ActivationCostAbility,
    original: &TotalCost,
) -> bool {
    !alternative_branches(
        game,
        source,
        &facts,
        original,
        &crate::derived_view::DerivedGameView::new(game),
    )
    .is_empty()
}

impl EffectExecutor for ironsmith_core::GrantActivationCostAlternativeEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effect::EffectOutcome, ExecutionError> {
        crate::effects::composition::execute_compound(game, ctx, |game, ctx| {
            let objects =
                crate::effects::helpers::resolve_objects_for_effect(game, ctx, &self.target)?;
            let mut affected = Vec::new();
            for source in objects {
                if !game
                    .object(source)
                    .is_some_and(|object| object.zone == Zone::Battlefield)
                    || game.is_phased_out(source)
                {
                    continue;
                }
                let grant = ActivationCostAlternative {
                    source,
                    status: self.status.clone(),
                    replacement_cost: self.replacement_cost.clone(),
                    max_uses_per_ability_each_turn: self.max_uses_per_ability_each_turn,
                    nonloyalty_only: self.nonloyalty_only,
                };
                if !game
                    .effect_store
                    .activation_cost_alternatives
                    .contains(&grant)
                {
                    game.effect_store.activation_cost_alternatives.push(grant);
                    game.mark_object_state_changed_keeping_characteristics(source);
                    game.mark_continuous_state_dirty();
                }
                affected.push(source);
            }
            Ok(crate::effect::EffectOutcome::with_objects(affected))
        })
    }
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }
    fn get_target_spec(&self) -> Option<&crate::target::ChooseSpec> {
        Some(&self.target)
    }
}
