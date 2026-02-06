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
use crate::decisions::specs::ChooseObjectsSpec;
use crate::decisions::{WardSpec, make_decision};
use crate::event_processor::execute_discard;
use crate::event_processor::{EventOutcome, process_zone_change};
use crate::events::cause::EventCause;
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::static_abilities::StaticAbility;

use super::types::{PendingWardCost, WardCost, WardPaymentResult};

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
    if target.controller == caster {
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
                ward_controller: target.controller,
                cost: convert_ward_cost(cost),
            });
        }
    }

    None
}

/// Convert a TotalCost (from the ability system) to a WardCost.
fn convert_ward_cost(cost: &TotalCost) -> WardCost {
    if cost.costs().len() == 1 {
        let component = &cost.costs()[0];
        if let Some(life) = component.life_amount() {
            return WardCost::Life(life);
        }
        if let Some((count, card_type)) = component.discard_details()
            && card_type.is_none()
        {
            return WardCost::Discard(count);
        }
        if let Some(filter) = component.sacrifice_filter() {
            return WardCost::Sacrifice(permanent_filter_to_object_filter(filter));
        }
    }

    WardCost::Mana(cost.clone())
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
    decision_maker: &mut impl DecisionMaker,
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
fn format_ward_cost_description(cost: &WardCost) -> String {
    match cost {
        WardCost::Mana(total_cost) => {
            if let Some(mana_cost) = total_cost.mana_cost() {
                format!("Pay {:?}", mana_cost)
            } else {
                "Pay mana cost".to_string()
            }
        }
        WardCost::Life(amount) => format!("Pay {} life", amount),
        WardCost::Discard(count) => {
            if *count == 1 {
                "Discard a card".to_string()
            } else {
                format!("Discard {} cards", count)
            }
        }
        WardCost::Sacrifice(filter) => format!("Sacrifice a permanent matching {:?}", filter),
    }
}

/// Attempt to pay a ward cost.
///
/// Returns true if the cost was successfully paid, false otherwise.
fn pay_ward_cost(
    game: &mut GameState,
    payer: PlayerId,
    source: ObjectId,
    cost: &WardCost,
    decision_maker: &mut impl DecisionMaker,
) -> bool {
    match cost {
        WardCost::Mana(total_cost) => {
            // Try to pay the mana cost from the player's mana pool
            if let Some(mana_cost) = total_cost.mana_cost() {
                // X is 0 for ward costs (ward costs don't have X)
                if game.try_pay_mana_cost(payer, None, mana_cost, 0) {
                    return true;
                }
            } else {
                // No mana cost required
                return true;
            }
            false
        }
        WardCost::Life(amount) => {
            // Deduct life from the player
            if let Some(player) = game.player_mut(payer)
                && player.life >= *amount as i32
            {
                player.life -= *amount as i32;
                return true;
            }
            false
        }
        WardCost::Discard(count) => {
            let Some(player) = game.player(payer) else {
                return false;
            };
            if player.hand.len() < *count as usize {
                return false;
            }

            let hand_cards = player.hand.clone();
            let spec = ChooseObjectsSpec::new(
                source,
                format!(
                    "Choose {} card{} to discard for ward",
                    count,
                    if *count == 1 { "" } else { "s" }
                ),
                hand_cards.clone(),
                *count as usize,
                Some(*count as usize),
            );
            let chosen: Vec<_> = make_decision(game, decision_maker, payer, Some(source), spec);
            let to_discard = normalize_selection(chosen, &hand_cards, *count as usize);

            if to_discard.len() != *count as usize {
                return false;
            }

            let cause = EventCause::from_cost(source, payer);
            for card_id in to_discard {
                let outcome =
                    execute_discard(game, card_id, payer, cause.clone(), false, decision_maker);
                if outcome.prevented {
                    return false;
                }
            }
            true
        }
        WardCost::Sacrifice(filter) => {
            let filter_ctx = game.filter_context_for(payer, Some(source));
            let candidates: Vec<ObjectId> = game
                .battlefield
                .iter()
                .filter_map(|&id| game.object(id).map(|obj| (id, obj)))
                .filter(|(id, obj)| {
                    obj.controller == payer
                        && filter.matches(obj, &filter_ctx, game)
                        && game.can_be_sacrificed(*id)
                })
                .map(|(id, _)| id)
                .collect();

            if candidates.is_empty() {
                return false;
            }

            let spec = ChooseObjectsSpec::new(
                source,
                "Choose a permanent to sacrifice for ward",
                candidates.clone(),
                1,
                Some(1),
            );
            let chosen: Vec<_> = make_decision(game, decision_maker, payer, Some(source), spec);
            let Some(target_id) = normalize_selection(chosen, &candidates, 1).first().copied()
            else {
                return false;
            };

            match process_zone_change(
                game,
                target_id,
                crate::zone::Zone::Battlefield,
                crate::zone::Zone::Graveyard,
                decision_maker,
            ) {
                EventOutcome::Prevented | EventOutcome::NotApplicable => false,
                EventOutcome::Proceed(final_zone) => {
                    game.move_object(target_id, final_zone);
                    true
                }
                EventOutcome::Replaced => true,
            }
        }
    }
}

fn permanent_filter_to_object_filter(
    filter: &crate::cost::PermanentFilter,
) -> crate::target::ObjectFilter {
    let mut object_filter = crate::target::ObjectFilter::permanent();
    object_filter.card_types = filter.card_types.clone();
    object_filter.subtypes = filter.subtypes.clone();
    object_filter.other = filter.other;
    object_filter.token = filter.token;
    object_filter.nontoken = filter.nontoken;
    object_filter
}

fn normalize_selection(
    chosen: Vec<ObjectId>,
    candidates: &[ObjectId],
    required: usize,
) -> Vec<ObjectId> {
    let mut selected = Vec::with_capacity(required);

    for id in chosen {
        if selected.len() == required {
            break;
        }
        if candidates.contains(&id) && !selected.contains(&id) {
            selected.push(id);
        }
    }

    if selected.len() < required {
        for &id in candidates {
            if selected.len() == required {
                break;
            }
            if !selected.contains(&id) {
                selected.push(id);
            }
        }
    }

    selected
}
