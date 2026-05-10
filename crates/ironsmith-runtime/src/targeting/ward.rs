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
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::special_actions::pay_total_cost_with_choice;
use crate::static_abilities::StaticAbility;

use super::types::{PendingWardCost, WardPaymentResult};

/// Check if a target has ward and return the pending ward cost if so.
///
/// This should be called when a spell/ability is being put on the stack
/// with targets. Any ward costs should be collected and prompted for payment.
pub fn get_ward_cost(
    game: &GameState,
    target_id: ObjectId,
    caster: PlayerId,
) -> Option<PendingWardCost> {
    let Some(target) = game.object(target_id) else {
        return None;
    };

    // Ward only triggers when an opponent targets
    if game.controller_of(target) == caster {
        return None;
    }

    // Check for ward ability
    let abilities: Vec<StaticAbility> = game
        .calculated_characteristics(target_id)
        .map(|c| c.static_abilities)
        .unwrap_or_else(|| {
            target
                .abilities
                .iter()
                .filter_map(|a| {
                    if let AbilityKind::Static(sa) = &a.kind {
                        Some(sa.clone())
                    } else {
                        None
                    }
                })
                .collect()
        });

    for ability in abilities {
        if let Some(cost) = ability.ward_cost() {
            return Some(PendingWardCost {
                target: target_id,
                ward_controller: game.controller_of(target),
                cost: cost.clone(),
            });
        }
    }

    None
}

/// Collect all ward costs for a set of targets.
pub fn collect_ward_costs(
    game: &GameState,
    targets: &[ObjectId],
    caster: PlayerId,
) -> Vec<PendingWardCost> {
    targets
        .iter()
        .filter_map(|&target_id| get_ward_cost(game, target_id, caster))
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

/// Format a ward cost for display.
fn format_ward_cost_description(cost: &TotalCost) -> String {
    let display = cost.display();
    let mana_only = cost
        .costs()
        .iter()
        .all(|component| component.mana_cost_ref().is_some());
    if mana_only {
        format!("Pay {display}")
    } else {
        display
    }
}

/// Attempt to pay a ward cost.
///
/// Returns true if the cost was successfully paid, false otherwise.
fn pay_ward_cost(
    game: &mut GameState,
    payer: PlayerId,
    source: ObjectId,
    cost: &TotalCost,
    decision_maker: &mut dyn DecisionMaker,
) -> bool {
    pay_total_cost_with_choice(
        game,
        payer,
        source,
        cost,
        crate::costs::PaymentReason::Effect,
        decision_maker,
    )
    .is_ok()
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
            .abilities
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
}
