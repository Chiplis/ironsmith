//! Cost system for abilities and spells.
//!
//! Costs represent what must be paid to cast a spell or activate an ability.
//! A total cost is a conjunction of individual costs that must all be paid.
//!
//! The main types are:
//! - `TotalCost`: A complete cost (conjunction of Cost components)
//! - `Cost` (in the `costs` module): Individual cost components (trait objects)

pub(crate) mod prospective_references;

use crate::costs::Cost;
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::mana::ManaCost;
use crate::object::CounterType;
use crate::target::ChooseSpec;
pub type TotalCost = ironsmith_core::TotalCost<Cost>;
pub type OptionalCost = ironsmith_core::OptionalCost<Cost>;
pub type OptionalCostKind = ironsmith_core::OptionalCostKind;
pub type OptionalCostRef = ironsmith_core::OptionalCostRef;
pub type OptionalCostsPaid = ironsmith_core::OptionalCostsPaid;

impl ironsmith_core::CostComponent for Cost {
    fn mana(mana_cost: ManaCost) -> Self {
        Self::mana(mana_cost)
    }

    fn display(&self) -> String {
        self.display()
    }

    fn sacrifice_filter(&self) -> Option<&crate::target::ObjectFilter> {
        Cost::sacrifice_filter(self)
    }

    fn is_mana_cost(&self) -> bool {
        self.is_mana_cost()
    }

    fn requires_tap(&self) -> bool {
        self.requires_tap()
    }

    fn life_amount(&self) -> Option<u32> {
        self.life_amount()
    }

    fn is_sacrifice_self(&self) -> bool {
        self.is_sacrifice_self()
    }

    fn discard_details(&self) -> Option<(u32, Option<crate::types::CardType>)> {
        self.discard_details()
    }

    fn exile_from_graveyard_excludes_source(&self) -> bool {
        self.effect_ref()
            .and_then(|effect| effect.downcast_ref::<crate::effects::ExileEffect>())
            .is_some_and(|exile| {
                matches!(exile.spec.base(), ChooseSpec::Object(filter)
                if filter.zone == Some(crate::zone::Zone::Graveyard) && filter.other)
            })
    }

    fn exile_from_hand_details(&self) -> Option<(u32, Option<crate::color::ColorSet>)> {
        self.exile_from_hand_details()
    }

    fn exile_from_graveyard_details(&self) -> Option<(u32, &[crate::types::CardType])> {
        self.exile_from_graveyard_details()
    }

    fn mana_cost_ref(&self) -> Option<&ManaCost> {
        self.mana_cost_ref()
    }

    fn dynamic_mana_cost_ref(&self) -> Option<&ironsmith_core::DynamicManaCost> {
        self.dynamic_mana_cost_ref()
    }

    fn is_loyalty_activation_cost(&self) -> bool {
        fn is_source(spec: &ChooseSpec) -> bool {
            matches!(spec.base(), ChooseSpec::Source)
        }

        self.effect_ref().is_some_and(|effect| {
            effect
                .downcast_ref::<crate::effects::PutCountersEffect>()
                .is_some_and(|put| {
                    put.counter_type == CounterType::Loyalty && is_source(&put.target)
                })
                || effect
                    .downcast_ref::<crate::effects::RemoveCountersEffect>()
                    .is_some_and(|remove| {
                        remove.counter_type == CounterType::Loyalty && is_source(&remove.target)
                    })
                || effect
                    .downcast_ref::<crate::effects::RemoveAnyCountersFromSourceEffect>()
                    .is_some_and(|remove| remove.counter_type == Some(CounterType::Loyalty))
        })
    }
}

impl ironsmith_core::CoreCostComponent for Cost {
    fn tap_cost() -> Self {
        Self::tap()
    }
}

// ============================================================================
// Cost Payment Validation
// ============================================================================

/// Error type for when a cost cannot be paid.
#[derive(Debug, Clone, PartialEq)]
pub enum CostPaymentError {
    /// The player explicitly cancelled an interactive payment.
    Cancelled,
    /// The source object doesn't exist.
    SourceNotFound,

    /// The player doesn't exist.
    PlayerNotFound,

    /// Not enough mana to pay the mana cost.
    InsufficientMana,

    /// Can't tap - permanent is already tapped.
    AlreadyTapped,

    /// Can't tap - creature has summoning sickness (rule 302.6).
    SummoningSickness,

    /// Can't untap - permanent is already untapped.
    AlreadyUntapped,

    /// Not enough life to pay the life cost.
    InsufficientLife,

    /// Source not on battlefield (for sacrifice/exile self).
    SourceNotOnBattlefield,

    /// No valid permanent to sacrifice.
    NoValidSacrificeTarget,

    /// Not enough cards in hand to discard.
    InsufficientCardsInHand,

    /// Not enough counters on the source.
    InsufficientCounters,

    /// Not enough energy counters.
    InsufficientEnergy,

    /// Not enough cards in hand matching the filter for exile.
    InsufficientCardsToExile,

    /// Not enough cards in graveyard matching the filter.
    InsufficientCardsInGraveyard,

    /// No valid permanent to return to hand.
    NoValidReturnTarget,

    /// Not enough cards in hand to reveal.
    InsufficientCardsToReveal,

    /// A replacement/program failed while paying an otherwise legal cost.
    ExecutionFailed(crate::effects::ExecutionError),

    /// Generic/other failure while validating or paying a cost.
    Other(String),
}

impl std::fmt::Display for CostPaymentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CostPaymentError::Cancelled => f.write_str("Payment cancelled"),
            CostPaymentError::SourceNotFound => f.write_str("Source object not found"),
            CostPaymentError::PlayerNotFound => f.write_str("Player not found"),
            CostPaymentError::InsufficientMana => f.write_str("Not enough mana"),
            CostPaymentError::AlreadyTapped => f.write_str("That permanent is already tapped"),
            CostPaymentError::SummoningSickness => {
                f.write_str("That creature has summoning sickness")
            }
            CostPaymentError::AlreadyUntapped => f.write_str("That permanent is already untapped"),
            CostPaymentError::InsufficientLife => f.write_str("Not enough life"),
            CostPaymentError::SourceNotOnBattlefield => {
                f.write_str("The source is not on the battlefield")
            }
            CostPaymentError::NoValidSacrificeTarget => {
                f.write_str("No valid permanent can be sacrificed")
            }
            CostPaymentError::InsufficientCardsInHand => f.write_str("Not enough cards in hand"),
            CostPaymentError::InsufficientCounters => {
                f.write_str("Not enough counters on the source")
            }
            CostPaymentError::InsufficientEnergy => f.write_str("Not enough energy counters"),
            CostPaymentError::InsufficientCardsToExile => {
                f.write_str("Not enough cards in hand to exile")
            }
            CostPaymentError::InsufficientCardsInGraveyard => {
                f.write_str("Not enough cards in the graveyard")
            }
            CostPaymentError::NoValidReturnTarget => {
                f.write_str("No valid permanent can be returned to hand")
            }
            CostPaymentError::InsufficientCardsToReveal => {
                f.write_str("Not enough cards in hand to reveal")
            }
            CostPaymentError::ExecutionFailed(error) => {
                write!(f, "Replacement during payment failed: {error}")
            }
            CostPaymentError::Other(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for CostPaymentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ExecutionFailed(error) => Some(error),
            _ => None,
        }
    }
}

/// Check if a player can pay an activated ability's or spell's cost.
///
/// This checks all cost components against the current game state.
/// The `source_id` is the permanent or spell whose cost is being paid.
pub fn can_pay_cost(
    game: &GameState,
    source_id: ObjectId,
    player: PlayerId,
    cost: &TotalCost,
) -> Result<(), CostPaymentError> {
    can_pay_cost_with_reason(
        game,
        source_id,
        player,
        cost,
        crate::costs::PaymentReason::Other,
    )
}

pub fn can_pay_cost_with_reason(
    game: &GameState,
    source_id: ObjectId,
    player: PlayerId,
    cost: &TotalCost,
    reason: crate::costs::PaymentReason,
) -> Result<(), CostPaymentError> {
    use crate::costs::{CostCheckContext, can_pay_with_check_context};

    // CR 800.4f: if an effect requires a player who left the game to pay a
    // cost, that cost is not paid.
    if !game
        .player(player)
        .is_some_and(|candidate| candidate.is_in_game())
    {
        return Err(CostPaymentError::Other(
            "a player who left the game cannot pay costs".to_string(),
        ));
    }

    // Choice-backed costs can pass objects to a later component through a tag
    // (for example, "choose half your lands, then sacrifice the chosen
    // lands"). The ordinary component-by-component check has no tag state, so
    // route those totals through the context-aware checker that can construct a
    // legal speculative selection without making the player's actual choice.
    if total_cost_has_tagged_choice_consumer(cost) {
        let mut decision_maker = crate::decision::SelectFirstDecisionMaker;
        let mut execution_ctx =
            crate::effects::ExecutionContext::new(source_id, player, &mut decision_maker);
        return crate::special_actions::can_pay_total_cost_with_reason_in_context(
            game,
            player,
            source_id,
            cost,
            reason,
            &mut execution_ctx,
        );
    }

    let ctx = CostCheckContext::new(source_id, player).with_reason(reason);

    match cost.kind() {
        ironsmith_core::TotalCostKind::All(costs) => {
            for cost_component in costs {
                let adjusted_component =
                    adjusted_component_for_check(game, player, source_id, cost_component, reason)?;
                game.validate_cost_for_payment_reason(
                    player,
                    source_id,
                    &adjusted_component,
                    reason,
                )?;
                can_pay_with_check_context(&*adjusted_component.0, game, &ctx)?;
            }
            Ok(())
        }
        ironsmith_core::TotalCostKind::OneOf(branches) => {
            if branches.iter().any(|branch| {
                can_pay_cost_with_reason(game, source_id, player, branch, reason).is_ok()
            }) {
                Ok(())
            } else {
                Err(CostPaymentError::Other(
                    "no payable alternative cost branch".to_string(),
                ))
            }
        }
    }
}

fn total_cost_has_tagged_choice_consumer(cost: &TotalCost) -> bool {
    match cost.kind() {
        ironsmith_core::TotalCostKind::All(components) => {
            (0..components.len()).any(|idx| tagged_choice_pair_at(components, idx).is_some())
        }
        ironsmith_core::TotalCostKind::OneOf(branches) => {
            branches.iter().any(total_cost_has_tagged_choice_consumer)
        }
    }
}

/// The tag named by a filter that selects exactly "the tagged objects".
fn filter_consumed_tag(filter: &crate::target::ObjectFilter) -> Option<&crate::tag::TagKey> {
    (!filter.tagged_constraints.is_empty()
        && filter.tagged_constraints.iter().all(|constraint| {
            constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
                && constraint.tag == filter.tagged_constraints[0].tag
        }))
    .then(|| &filter.tagged_constraints[0].tag)
}

fn spec_consumed_tag(spec: &ChooseSpec) -> Option<&crate::tag::TagKey> {
    match spec.base() {
        ChooseSpec::Tagged(tag) => Some(tag),
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => filter_consumed_tag(filter),
        _ => None,
    }
}

/// The choice tag an effect-backed cost component consumes: a sacrifice,
/// exile, return, move, unattach, tap (or any other effect whose object spec
/// is exactly "the chosen objects") that pays with the objects a preceding
/// `ChooseObjectsEffect` cost published under that tag. Transparent wrappers
/// (such as `WithIdEffect`) are peeled first.
pub(crate) fn effect_consumed_choice_tag(
    effect: &crate::effect::Effect,
) -> Option<crate::tag::TagKey> {
    let mut consumer = effect;
    while let Some(inner) = consumer.transparent_child_effect() {
        consumer = inner;
    }
    // A sequence that selects its own payment objects must execute that
    // producer before requiring the consumer's tag. The whole cost's
    // preflight simulates the dependency without an existing external tag.
    if let Some(sequence) = consumer.downcast_ref::<crate::effects::SequenceEffect>()
        && sequence.effects.iter().any(|effect| {
            let mut inner = effect;
            while let Some(child) = inner.transparent_child_effect() {
                inner = child;
            }
            inner
                .downcast_ref::<crate::effects::ChooseObjectsEffect>()
                .is_some()
        })
    {
        return None;
    }
    if consumer
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()
        .is_some()
    {
        return None;
    }
    if let Some(sacrifice) = consumer.downcast_ref::<crate::effects::SacrificeEffect>() {
        return (sacrifice.player == crate::target::PlayerFilter::You)
            .then(|| filter_consumed_tag(&sacrifice.filter).cloned())
            .flatten();
    }
    if let Some(sacrifice) = consumer.downcast_ref::<ironsmith_core::SacrificePlayerEffect>() {
        return (sacrifice.player == crate::target::PlayerFilter::You)
            .then(|| filter_consumed_tag(&sacrifice.filter).cloned())
            .flatten();
    }
    if let Some(discard) = consumer.downcast_ref::<crate::effects::DiscardEffect>() {
        return discard
            .card_filter
            .as_ref()
            .and_then(filter_consumed_tag)
            .cloned();
    }
    if let Some(reveal) = consumer.downcast_ref::<crate::effects::RevealTaggedEffect>() {
        return Some(reveal.tag.clone());
    }
    if let Some(exile) = consumer.downcast_ref::<crate::effects::ExileEffect>() {
        return spec_consumed_tag(&exile.spec).cloned();
    }
    if let Some(returned) = consumer.downcast_ref::<crate::effects::ReturnToHandEffect>() {
        return spec_consumed_tag(&returned.spec).cloned();
    }
    if let Some(moved) = consumer.downcast_ref::<crate::effects::MoveToZoneEffect>() {
        return spec_consumed_tag(&moved.target).cloned();
    }
    if let Some(unattach) = consumer.downcast_ref::<crate::effects::UnattachObjectsEffect>() {
        return spec_consumed_tag(&unattach.objects).cloned();
    }
    if let Some(tap) = consumer.downcast_ref::<crate::effects::TapEffect>() {
        return spec_consumed_tag(&tap.target).cloned();
    }
    if let Some(untap) = consumer.downcast_ref::<crate::effects::UntapEffect>() {
        return spec_consumed_tag(&untap.target).cloned();
    }
    consumer
        .0
        .get_target_spec()
        .into_iter()
        .cloned()
        .chain(consumer.0.decision_related_object_specs())
        .find_map(|spec| spec_consumed_tag(&spec).cloned())
}

/// The choice tag a cost component consumes (see [`effect_consumed_choice_tag`]).
pub(crate) fn cost_consumed_choice_tag(cost: &Cost) -> Option<crate::tag::TagKey> {
    cost.effect_ref().and_then(effect_consumed_choice_tag)
}

/// Whether `components[idx]` is a `ChooseObjectsEffect` cost immediately
/// followed by a component that pays with the objects it chooses. Such a
/// pair must be checked (and paid) together: checking the consumer on its own
/// sees no published tag and wrongly rejects a payable cost.
pub(crate) fn tagged_choice_pair_at(
    components: &[Cost],
    idx: usize,
) -> Option<&crate::effects::ChooseObjectsEffect> {
    let choose = components
        .get(idx)?
        .effect_ref()?
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let consumer = components.get(idx + 1)?;
    (cost_consumed_choice_tag(consumer).as_ref() == Some(&choose.tag)).then_some(choose)
}

/// Check the "choose objects, then consume them" pair starting at
/// `components[idx]` together, building a representative legal selection for
/// the choice without prompting.
pub(crate) fn tagged_choice_pair_is_payable(
    game: &GameState,
    payer: PlayerId,
    source: ObjectId,
    components: &[Cost],
    idx: usize,
    reason: crate::costs::PaymentReason,
    x_value: Option<u32>,
) -> bool {
    let Some(choose) = tagged_choice_pair_at(components, idx) else {
        return false;
    };
    let mut choose_cost = components[idx].clone();
    let consumer = &components[idx + 1];
    // "{T}, Tap two untapped creatures you control": the source is tapped by
    // its own {T} component, so it can't also be one of the chosen untapped
    // creatures (Harmonized Trio). Likewise {Q} consumes its tapped state
    // before a separate written untap cost (Crackleburr).
    let state_change = consumer.effect_ref().and_then(|effect| {
        let mut effect = effect;
        while let Some(inner) = effect.transparent_child_effect() {
            effect = inner;
        }
        if effect.downcast_ref::<crate::effects::TapEffect>().is_some() {
            Some(true)
        } else if effect
            .downcast_ref::<crate::effects::UntapEffect>()
            .is_some()
        {
            Some(false)
        } else {
            None
        }
    });
    let source_state_reserved = match state_change {
        Some(true) => choose.filter.untapped && components.iter().any(Cost::requires_tap),
        Some(false) => choose.filter.tapped && components.iter().any(Cost::requires_untap),
        None => false,
    };
    if source_state_reserved && !choose.filter.other {
        let mut choose = choose.clone();
        choose.filter.other = true;
        choose_cost = Cost::validated_effect(crate::effect::Effect::new(choose));
    }
    let pair = TotalCost::from_costs(vec![choose_cost, consumer.clone()]);
    let mut decision_maker = crate::decision::SelectFirstDecisionMaker;
    let mut execution_ctx =
        crate::effects::ExecutionContext::new(source, payer, &mut decision_maker);
    execution_ctx.x_value = x_value;
    crate::special_actions::can_pay_total_cost_with_reason_in_context(
        game,
        payer,
        source,
        &pair,
        reason,
        &mut execution_ctx,
    )
    .is_ok()
}

/// The branch of an optional spell cost that is paid when the optional cost
/// is announced. A one-of optional cost (Waterbend: pay {N}, or tap
/// artifacts/creatures to help pay) pays the branch the caster announced
/// (`OptionalCostsPaid::branch_choice`), defaulting to its first, all-mana
/// branch; mana and non-mana steps must read the same branch.
pub(crate) fn optional_cost_payment_branch(cost: &TotalCost, branch: Option<usize>) -> &TotalCost {
    match cost.kind() {
        ironsmith_core::TotalCostKind::All(_) => cost,
        ironsmith_core::TotalCostKind::OneOf(branches) => branches
            .get(branch.unwrap_or(0))
            .or_else(|| branches.first())
            .map_or(cost, |branch| optional_cost_payment_branch(branch, None)),
    }
}

pub(crate) fn adjusted_component_for_check(
    game: &GameState,
    player: PlayerId,
    source_id: ObjectId,
    cost_component: &crate::costs::Cost,
    reason: crate::costs::PaymentReason,
) -> Result<crate::costs::Cost, CostPaymentError> {
    if let Some(mana_cost) = cost_component.mana_cost_ref() {
        return Ok(crate::costs::Cost::mana(
            game.adjust_mana_cost_for_payment_reason(player, Some(source_id), mana_cost, reason),
        ));
    }
    if let Some(dynamic_mana) = cost_component.dynamic_mana_cost_ref() {
        if let Some(static_base) = dynamic_mana.resolved_static_base() {
            return Ok(crate::costs::Cost::mana(
                game.adjust_mana_cost_for_payment_reason(
                    player,
                    Some(source_id),
                    &static_base,
                    reason,
                ),
            ));
        }
        return Err(CostPaymentError::Other(
            "dynamic mana cost requires an execution context".to_string(),
        ));
    }
    Ok(cost_component.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mana::ManaSymbol;

    #[test]
    fn test_free_cost() {
        let cost = TotalCost::free();
        assert!(cost.is_free());
        assert!(cost.mana_cost().is_none());
        assert!(!cost.has_non_mana_costs());
    }

    #[test]
    fn test_mana_cost() {
        let mana = ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)], vec![ManaSymbol::White]]);
        let cost = TotalCost::mana(mana.clone());

        assert!(!cost.is_free());
        assert_eq!(cost.mana_cost(), Some(&mana));
        assert!(!cost.has_non_mana_costs());
    }

    #[test]
    fn gift_prefix_lookup_matches_descriptive_gift_label() {
        let paid = OptionalCostsPaid {
            costs: vec![("Gift a tapped Fish".into(), 1)],
            cast_at_sorcery_timing: false,
            branch_choices: Vec::new(),
        };

        assert!(paid.was_paid_label("Gift"));
        assert_eq!(paid.times_paid_label("Gift"), 1);
    }
}
