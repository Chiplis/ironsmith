//! Ward handling for targeted spells and abilities.
//!
//! Ward is a keyword ability that requires opponents to pay an additional cost
//! when targeting the permanent with ward. If they don't pay, the spell or
//! ability is countered.
//!
//! Per MTG rules:
//! - Ward triggers when the permanent becomes the target of a spell or ability
//! - The trigger goes on the stack
//! - When it resolves, the opponent must pay the ward cost or the spell/ability
//!   is countered

use crate::ability::AbilityKind;
use crate::cost::TotalCost;
use crate::decision::DecisionMaker;
use crate::decisions::{WardSpec, make_decision};
use crate::filter::ObjectFilterExt as _;
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::special_actions::pay_resolution_cost_with_mana_abilities;
use crate::static_abilities::StaticAbility;

use super::types::{PendingWardCost, WardPaymentResult};

/// Check if a target has ward and return the pending ward cost if so.
///
/// Returns the first ward instance only; see [`get_ward_costs`] for every
/// instance (ward isn't redundant, CR 702.21a).
pub fn get_ward_cost(
    game: &GameState,
    target_id: ObjectId,
    caster: PlayerId,
) -> Option<PendingWardCost> {
    get_ward_costs(game, target_id, caster).into_iter().next()
}

/// Every ward instance that triggers when `caster`'s spell or ability targets
/// `target_id`.
///
/// Ward is "Whenever this permanent becomes the target of a spell or ability
/// an opponent controls" (CR 702.21a): it functions only on the battlefield
/// (CR 113.6), so a ward creature spell on the stack or a ward card in a
/// graveyard never taxes the spell targeting it. Each instance triggers
/// separately.
pub fn get_ward_costs(
    game: &GameState,
    target_id: ObjectId,
    caster: PlayerId,
) -> Vec<PendingWardCost> {
    let Some(target) = game.object(target_id) else {
        return Vec::new();
    };
    if target.zone != crate::zone::Zone::Battlefield || game.is_phased_out(target_id) {
        return Vec::new();
    }

    // Ward only triggers when an opponent targets
    let ward_controller = game.controller_of(target);
    if !game.are_opponents(ward_controller, caster) {
        return Vec::new();
    }

    // Check for ward ability
    let abilities: Vec<StaticAbility> = game
        .calculated_characteristics_arc(target_id)
        .map(|c| c.static_abilities.to_vec())
        .unwrap_or_else(|| {
            target
                .abilities
                .iter()
                .filter(|a| a.functions_in(&crate::zone::Zone::Battlefield))
                .filter_map(|a| {
                    if let AbilityKind::Static(sa) = &a.kind {
                        Some(sa.clone())
                    } else {
                        None
                    }
                })
                .collect()
        });

    if ward_abilities_are_suppressed(game, target_id, caster) {
        return Vec::new();
    }

    abilities
        .iter()
        .filter_map(|ability| ability.ward_cost())
        .map(|cost| PendingWardCost {
            target: target_id,
            ward_controller,
            cost: cost.clone(),
        })
        .collect()
}

fn ward_abilities_are_suppressed(game: &GameState, target_id: ObjectId, caster: PlayerId) -> bool {
    let Some(target) = game.object(target_id) else {
        return false;
    };
    let filter_ctx = game.filter_context_for(caster, Some(target_id));
    game.battlefield.iter().copied().any(|source_id| {
        game.calculated_characteristics(source_id)
            .map(|c| c.static_abilities)
            .unwrap_or_else(|| {
                game.object(source_id)
                    .map(|source| {
                        source
                            .abilities
                            .iter()
                            .filter_map(|ability| match &ability.kind {
                                AbilityKind::Static(static_ability) => Some(static_ability.clone()),
                                _ => None,
                            })
                            .collect::<Vec<_>>()
                            .into()
                    })
                    .unwrap_or_default()
            })
            .into_iter()
            .any(|ability| {
                ability
                    .display()
                    .to_ascii_lowercase()
                    .contains("ward abilities")
                    && ability
                        .trigger_suppression_spec()
                        .and_then(|spec| spec.source_filter)
                        .is_some_and(|filter| filter.matches(target, &filter_ctx, game))
            })
    })
}

/// Collect all ward costs for a set of targets.
pub fn collect_ward_costs(
    game: &GameState,
    targets: &[ObjectId],
    caster: PlayerId,
) -> Vec<PendingWardCost> {
    targets
        .iter()
        .flat_map(|&target_id| get_ward_costs(game, target_id, caster))
        .collect()
}

/// Handle ward cost payment with a decision maker.
///
/// This is called when the ward trigger resolves. The caster must pay
/// the ward cost or the spell/ability is countered.
///
/// The decision maker is prompted to decide whether to pay the ward cost.
/// If they agree to pay, the cost is deducted from the game state.
///
/// Returns the result of the ward payment attempt.
pub fn handle_ward_payment(
    game: &mut GameState,
    ward_cost: &PendingWardCost,
    caster: PlayerId,
    source: ObjectId,
    decision_maker: &mut dyn DecisionMaker,
) -> WardPaymentResult {
    // Create a description of the ward cost
    let description = format_ward_cost_description(&ward_cost.cost);

    // Ask player whether to pay using the spec-based system
    let spec = WardSpec::new(
        source,
        ward_cost.target,
        ward_cost.cost.clone(),
        description,
    );
    let should_pay: bool = make_decision(game, decision_maker, caster, Some(source), spec);
    if decision_maker.awaiting_choice() {
        // No answer yet: don't attempt payment. The caller must check
        // `awaiting_choice()` and unwind instead of countering.
        return WardPaymentResult::NotPaid;
    }

    if should_pay {
        // Player chose to pay - attempt to deduct the cost
        if pay_ward_cost(game, caster, source, &ward_cost.cost, decision_maker) {
            WardPaymentResult::Paid
        } else {
            // Couldn't actually pay the cost
            WardPaymentResult::NotPaid
        }
    } else {
        // Player declined to pay
        WardPaymentResult::NotPaid
    }
}

/// Custom trigger id for the ward keyword's triggered ability.
pub(crate) const WARD_TRIGGER_ID: &str = "intrinsic_ward";

/// Resolution of one ward trigger: "counter that spell or ability unless its
/// controller pays [cost]" (CR 702.21a).
///
/// The trigger is put on the stack when the permanent becomes the target, so
/// it can be responded to, Stifled or copied, each ward instance triggers on
/// its own, and it counters through the ordinary counter path (a spell that
/// "can't be countered" still resolves).
#[derive(Debug, Clone, PartialEq)]
pub struct WardCounterEffect {
    /// Object ID of the targeting spell, or the source of the targeting ability.
    pub targeting_source: ObjectId,
    /// The targeting ability's own stack id, when it has one.
    pub targeting_ability_id: Option<ObjectId>,
    /// Whether the targeting stack object is an ability.
    pub by_ability: bool,
    /// The ward permanent that became the target.
    pub ward_target: ObjectId,
    /// The ward cost.
    pub cost: TotalCost,
}

impl WardCounterEffect {
    /// Locate the stack object the ward trigger refers to. Abilities share
    /// their source's ID, so prefer the newest matching entry that still
    /// targets the ward permanent.
    fn stack_index(&self, game: &GameState) -> Option<usize> {
        if let Some(ability_id) = self.targeting_ability_id {
            return game
                .stack
                .iter()
                .position(|entry| entry.ability_id == Some(ability_id));
        }
        let matches = |entry: &crate::game_state::StackEntry| {
            entry.object_id == self.targeting_source && entry.is_ability == self.by_ability
        };
        game.stack
            .iter()
            .rposition(|entry| {
                matches(entry)
                    && entry
                        .targets
                        .contains(&crate::game_state::Target::Object(self.ward_target))
            })
            .or_else(|| game.stack.iter().rposition(matches))
    }
}

impl crate::effects::EffectExecutor for WardCounterEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut crate::effects::ExecutionContext,
    ) -> Result<crate::effect::EffectOutcome, crate::effects::ExecutionError> {
        // The spell or ability already left the stack: nothing to counter.
        let Some(index) = self.stack_index(game) else {
            return Ok(crate::effect::EffectOutcome::target_invalid());
        };
        let payer = game.stack[index].controller;
        let pending = PendingWardCost {
            target: self.ward_target,
            ward_controller: ctx.controller,
            cost: self.cost.clone(),
        };
        let payment = handle_ward_payment(
            game,
            &pending,
            payer,
            self.targeting_source,
            &mut *ctx.decision_maker,
        );
        if ctx.decision_maker.awaiting_choice() {
            return Ok(crate::effect::EffectOutcome::count(0));
        }
        if payment == WardPaymentResult::Paid {
            return Ok(crate::effect::EffectOutcome::resolved());
        }
        let Some(index) = self.stack_index(game) else {
            return Ok(crate::effect::EffectOutcome::target_invalid());
        };
        Ok(crate::effects::stack::counter_stack_entry_at(game, ctx, index))
    }
}

/// Format a ward cost for display.
fn format_ward_cost_description(cost: &TotalCost) -> String {
    fn is_mana_only(cost: &TotalCost) -> bool {
        match cost.kind() {
            ironsmith_core::TotalCostKind::All(costs) => costs
                .iter()
                .all(|component| component.mana_cost_ref().is_some()),
            ironsmith_core::TotalCostKind::OneOf(branches) => branches.iter().all(is_mana_only),
        }
    }

    let display = cost.display();
    if is_mana_only(cost) {
        format!("Pay {display}")
    } else {
        display
    }
}

/// Attempt to pay a ward cost.
///
/// Ward is paid while its trigger resolves, so the payer may tap mana sources
/// for it rather than needing the mana already floating (CR 605.3a).
///
/// Returns true if the cost was successfully paid, false otherwise.
fn pay_ward_cost(
    game: &mut GameState,
    payer: PlayerId,
    source: ObjectId,
    cost: &TotalCost,
    decision_maker: &mut dyn DecisionMaker,
) -> bool {
    pay_resolution_cost_with_mana_abilities(
        game,
        payer,
        source,
        cost,
        crate::costs::PaymentReason::Effect,
        decision_maker,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::Ability;
    use crate::card::CardBuilder;
    use crate::cost::TotalCost;
    use crate::decision::SelectFirstDecisionMaker;
    use crate::ids::CardId;
    use crate::static_abilities::StaticAbility;
    use crate::target::ObjectFilter;
    use crate::types::CardType;
    use crate::zone::Zone;

    fn create_test_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    fn permanent(
        game: &mut GameState,
        controller: PlayerId,
        name: &str,
        card_type: CardType,
    ) -> ObjectId {
        let card = CardBuilder::new(CardId::new(), name)
            .card_types(vec![card_type])
            .build();
        game.create_object_from_card(&card, controller, Zone::Battlefield)
    }

    fn add_ward(game: &mut GameState, object_id: ObjectId, cost: TotalCost) {
        let ability = Ability::static_ability(StaticAbility::ward(cost));
        game.object_mut(object_id)
            .expect("ward permanent exists")
            .abilities_mut()
            .push(ability);
    }

    #[test]
    fn ward_effect_backed_sacrifice_cost_is_paid_generically() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let target = permanent(&mut game, alice, "Ward Bear", CardType::Creature);
        let source = permanent(&mut game, bob, "Targeting Source", CardType::Artifact);
        let sacrifice = permanent(&mut game, bob, "Payment Bear", CardType::Creature);

        add_ward(
            &mut game,
            target,
            TotalCost::from_cost(crate::costs::Cost::sacrifice(
                ObjectFilter::creature().you_control(),
            )),
        );

        let ward = get_ward_cost(&game, target, bob).expect("ward cost");
        let mut dm = SelectFirstDecisionMaker;
        assert_eq!(
            handle_ward_payment(&mut game, &ward, bob, source, &mut dm),
            WardPaymentResult::Paid
        );
        assert!(
            game.object(sacrifice).is_none()
                || game
                    .object(sacrifice)
                    .is_some_and(|object| object.zone != Zone::Battlefield),
            "original payment creature object should leave the battlefield"
        );
        assert_eq!(
            game.player(bob).expect("bob exists").graveyard.len(),
            1,
            "payment should put a card/object into Bob's graveyard"
        );
    }

    #[test]
    fn ward_mixed_cost_fails_before_partial_payment_when_component_unpayable() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let target = permanent(&mut game, alice, "Ward Bear", CardType::Creature);
        let source = permanent(&mut game, bob, "Targeting Source", CardType::Artifact);

        add_ward(
            &mut game,
            target,
            TotalCost::from_costs(vec![
                crate::costs::Cost::life(2),
                crate::costs::Cost::sacrifice(ObjectFilter::creature().you_control()),
            ]),
        );

        let ward = get_ward_cost(&game, target, bob).expect("ward cost");
        let mut dm = SelectFirstDecisionMaker;
        assert_eq!(
            handle_ward_payment(&mut game, &ward, bob, source, &mut dm),
            WardPaymentResult::NotPaid
        );
        assert_eq!(game.player(bob).map(|player| player.life), Some(20));
    }

    #[test]
    fn ward_alternative_cost_selects_a_payable_branch() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let target = permanent(&mut game, alice, "Ward Bear", CardType::Creature);
        let source = permanent(&mut game, bob, "Targeting Source", CardType::Artifact);

        add_ward(
            &mut game,
            target,
            TotalCost::one_of(vec![
                TotalCost::from_cost(crate::costs::Cost::life(2)),
                TotalCost::mana(crate::mana::ManaCost::from_symbols(vec![
                    crate::mana::ManaSymbol::Generic(2),
                ])),
            ]),
        );

        let ward = get_ward_cost(&game, target, bob).expect("ward cost");
        assert_eq!(
            format_ward_cost_description(&ward.cost),
            "Pay 2 life or {2}"
        );

        let mut dm = SelectFirstDecisionMaker;
        assert_eq!(
            handle_ward_payment(&mut game, &ward, bob, source, &mut dm),
            WardPaymentResult::Paid
        );
        assert_eq!(game.player(bob).map(|player| player.life), Some(18));
    }

    #[test]
    fn ward_mana_cost_taps_untapped_lands_when_pool_is_empty() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let target = permanent(&mut game, alice, "Ward Bear", CardType::Creature);
        let source = permanent(&mut game, bob, "Targeting Source", CardType::Artifact);
        let mountains = [0, 1].map(|_| {
            game.create_object_from_definition(
                &crate::cards::definitions::basic_mountain(),
                bob,
                Zone::Battlefield,
            )
        });

        add_ward(
            &mut game,
            target,
            TotalCost::mana(crate::mana::ManaCost::from_symbols(vec![
                crate::mana::ManaSymbol::Generic(2),
            ])),
        );

        let ward = get_ward_cost(&game, target, bob).expect("ward cost");
        let mut dm = SelectFirstDecisionMaker;
        assert_eq!(
            handle_ward_payment(&mut game, &ward, bob, source, &mut dm),
            WardPaymentResult::Paid
        );
        assert!(mountains.iter().all(|&mountain| game.is_tapped(mountain)));
    }
}
