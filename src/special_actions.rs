//! Special actions in MTG that don't use the stack.
//!
//! Special actions include playing lands, turning face-down creatures face up,
//! suspending/foretelling cards, and activating mana abilities.

use crate::cost::CostPaymentError;
use crate::costs::{CostContext, CostPaymentResult};
use crate::decisions::make_decision;
use crate::decisions::specs::ChooseObjectsSpec;
use crate::event_processor::{EventOutcome, execute_discard, process_zone_change};
use crate::events::cause::EventCause;
use crate::events::permanents::SacrificeEvent;
use crate::game_state::{GameState, Phase, Step};
use crate::ids::{ObjectId, PlayerId};
use crate::snapshot::ObjectSnapshot;
use crate::triggers::TriggerEvent;
use crate::types::CardType;
use crate::zone::Zone;

#[derive(Debug, Clone, PartialEq)]
struct TurnFaceUpSpec {
    cost: crate::cost::TotalCost,
    megamorph: bool,
}

fn turn_face_up_spec(object: &crate::object::Object) -> Option<TurnFaceUpSpec> {
    let mut chosen: Option<TurnFaceUpSpec> = None;
    for ability in &object.abilities {
        if !ability.functions_in(&Zone::Battlefield) {
            continue;
        }
        let crate::ability::AbilityKind::Static(static_ability) = &ability.kind else {
            continue;
        };
        let Some(cost) = static_ability.turn_face_up_cost() else {
            continue;
        };
        let candidate = TurnFaceUpSpec {
            cost: crate::cost::TotalCost::mana(cost.clone()),
            megamorph: static_ability.is_megamorph(),
        };
        if chosen
            .as_ref()
            .is_none_or(|current| !current.megamorph && candidate.megamorph)
        {
            chosen = Some(candidate);
        }
    }
    chosen
}

/// A special action that can be performed without using the stack.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpecialAction {
    /// Play a land from hand to the battlefield.
    PlayLand { card_id: ObjectId },

    /// Turn a face-down permanent face up via a turn-face-up special action.
    TurnFaceUp { permanent_id: ObjectId },

    /// Suspend a card from hand (pay suspend cost, exile with time counters).
    Suspend { card_id: ObjectId },

    /// Foretell a card from hand (exile face-down, can cast later for foretell cost).
    Foretell { card_id: ObjectId },

    /// Activate a mana ability (doesn't use the stack).
    ActivateManaAbility {
        permanent_id: ObjectId,
        ability_index: usize,
    },
}

/// Errors that can occur when attempting to perform a special action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionError {
    /// You don't have priority.
    NotYourPriority,

    /// Wrong phase for this action.
    WrongPhase { required: Phase, actual: Phase },

    /// Wrong step for this action.
    WrongStep {
        required: Option<Step>,
        actual: Option<Step>,
    },

    /// The stack must be empty for this action.
    StackNotEmpty,

    /// Already played maximum lands this turn.
    AlreadyPlayedLand,

    /// The object is not a land.
    NotALand,

    /// Cannot pay the cost.
    CantPayCost,

    /// Creature has summoning sickness (rule 302.6).
    SummoningSickness,

    /// Invalid target for this action.
    InvalidTarget,

    /// The object is not in the expected zone.
    WrongZone { expected: Zone, actual: Zone },

    /// The object doesn't have the required ability.
    NoSuchAbility,

    /// The permanent is not face-down.
    NotFaceDown,

    /// Object not found.
    ObjectNotFound,

    /// Player not found.
    PlayerNotFound,

    /// Cannot perform action during this step.
    InvalidTiming,

    /// Not the active player.
    NotActivePlayer,
}

/// Check if a special action can be performed.
pub fn can_perform(
    action: &SpecialAction,
    game: &GameState,
    player: PlayerId,
    decision_maker: &mut impl crate::decision::DecisionMaker,
) -> Result<(), ActionError> {
    match action {
        SpecialAction::PlayLand { card_id } => can_play_land(game, player, *card_id),
        SpecialAction::TurnFaceUp { permanent_id } => can_turn_face_up(game, player, *permanent_id),
        SpecialAction::Suspend { card_id } => can_suspend(game, player, *card_id),
        SpecialAction::Foretell { card_id } => can_foretell(game, player, *card_id),
        SpecialAction::ActivateManaAbility {
            permanent_id,
            ability_index,
        } => can_activate_mana_ability(game, player, *permanent_id, *ability_index, decision_maker),
    }
}

/// Check if a special action can be performed (for query/legality checks).
///
/// This variant doesn't require a decision_maker because it only checks
/// if costs CAN be paid, not actually paying them. Used by functions like
/// `compute_legal_actions` that need to enumerate possible actions.
pub fn can_perform_check(
    action: &SpecialAction,
    game: &GameState,
    player: PlayerId,
) -> Result<(), ActionError> {
    match action {
        SpecialAction::PlayLand { card_id } => can_play_land(game, player, *card_id),
        SpecialAction::TurnFaceUp { permanent_id } => can_turn_face_up(game, player, *permanent_id),
        SpecialAction::Suspend { card_id } => can_suspend(game, player, *card_id),
        SpecialAction::Foretell { card_id } => can_foretell(game, player, *card_id),
        SpecialAction::ActivateManaAbility {
            permanent_id,
            ability_index,
        } => can_activate_mana_ability_check(game, player, *permanent_id, *ability_index),
    }
}

/// Perform a special action.
pub fn perform(
    action: SpecialAction,
    game: &mut GameState,
    player: PlayerId,
    decision_maker: &mut impl crate::decision::DecisionMaker,
) -> Result<(), ActionError> {
    // First validate that we can perform the action
    can_perform(&action, game, player, &mut *decision_maker)?;

    match action {
        SpecialAction::PlayLand { card_id } => {
            perform_play_land(game, player, card_id, decision_maker)
        }
        SpecialAction::TurnFaceUp { permanent_id } => {
            perform_turn_face_up(game, player, permanent_id, &mut *decision_maker)
        }
        SpecialAction::Suspend { card_id } => perform_suspend(game, player, card_id),
        SpecialAction::Foretell { card_id } => perform_foretell(game, player, card_id),
        SpecialAction::ActivateManaAbility {
            permanent_id,
            ability_index,
        } => perform_activate_mana_ability(
            game,
            player,
            permanent_id,
            ability_index,
            &mut *decision_maker,
        ),
    }
}

// === Play Land ===

fn can_play_land(game: &GameState, player: PlayerId, card_id: ObjectId) -> Result<(), ActionError> {
    // Must be the active player
    if game.turn.active_player != player {
        return Err(ActionError::NotActivePlayer);
    }

    // Must have priority (or be in a main phase where you would have priority)
    if game.turn.priority_player != Some(player) {
        return Err(ActionError::NotYourPriority);
    }

    // Must be in a main phase
    let is_main_phase = game.turn.phase == Phase::FirstMain || game.turn.phase == Phase::NextMain;
    if !is_main_phase {
        return Err(ActionError::WrongPhase {
            required: Phase::FirstMain,
            actual: game.turn.phase,
        });
    }

    // Stack must be empty
    if !game.stack_is_empty() {
        return Err(ActionError::StackNotEmpty);
    }

    // Check player can still play lands
    let player_data = game.player(player).ok_or(ActionError::PlayerNotFound)?;
    if !player_data.can_play_land() {
        return Err(ActionError::AlreadyPlayedLand);
    }

    // Check the object exists and is in hand
    let object = game.object(card_id).ok_or(ActionError::ObjectNotFound)?;
    if object.zone != Zone::Hand {
        return Err(ActionError::WrongZone {
            expected: Zone::Hand,
            actual: object.zone,
        });
    }

    // Check the object is a land
    if !object.has_card_type(CardType::Land) {
        return Err(ActionError::NotALand);
    }

    // Check the player owns the card
    if object.owner != player {
        return Err(ActionError::InvalidTarget);
    }

    Ok(())
}

fn perform_play_land(
    game: &mut GameState,
    player: PlayerId,
    card_id: ObjectId,
    decision_maker: &mut impl crate::decision::DecisionMaker,
) -> Result<(), ActionError> {
    // Move the land to the battlefield with ETB replacement processing.
    let result = game
        .move_object_with_etb_processing_with_dm(card_id, Zone::Battlefield, decision_maker)
        .ok_or(ActionError::ObjectNotFound)?;
    let new_id = result.new_id;

    // Mark that the player has played a land this turn
    if let Some(player_data) = game.player_mut(player) {
        player_data.record_land_play();
    }

    // Set the controller to the player who played it
    if let Some(obj) = game.object_mut(new_id) {
        obj.controller = player;
    }

    Ok(())
}

// === Turn Face Up ===

fn can_turn_face_up(
    game: &GameState,
    player: PlayerId,
    permanent_id: ObjectId,
) -> Result<(), ActionError> {
    use crate::costs::{CostCheckContext, can_pay_with_check_context};

    // Must have priority to take a special action.
    if game.turn.priority_player != Some(player) {
        return Err(ActionError::NotYourPriority);
    }

    // Check the permanent exists and is on the battlefield
    let object = game
        .object(permanent_id)
        .ok_or(ActionError::ObjectNotFound)?;
    if object.zone != Zone::Battlefield {
        return Err(ActionError::WrongZone {
            expected: Zone::Battlefield,
            actual: object.zone,
        });
    }

    // Check the permanent is face-down
    if !game.is_face_down(permanent_id) {
        return Err(ActionError::NotFaceDown);
    }

    // Check the player controls the permanent
    if object.controller != player {
        return Err(ActionError::InvalidTarget);
    }

    let Some(spec) = turn_face_up_spec(object) else {
        return Err(ActionError::NoSuchAbility);
    };

    // Check whether the player can currently pay the turn-face-up cost.
    // (Unlike spell casting, this path currently doesn't open a mana-payment subflow.)
    let check_ctx = CostCheckContext::new(permanent_id, player);
    for cost in spec.cost.costs() {
        can_pay_with_check_context(&*cost.0, game, &check_ctx)
            .map_err(cost_error_to_action_error)?;
    }

    Ok(())
}

fn perform_turn_face_up(
    game: &mut GameState,
    player: PlayerId,
    permanent_id: ObjectId,
    decision_maker: &mut impl crate::decision::DecisionMaker,
) -> Result<(), ActionError> {
    let spec = game
        .object(permanent_id)
        .ok_or(ActionError::ObjectNotFound)
        .and_then(|object| turn_face_up_spec(object).ok_or(ActionError::NoSuchAbility))?;

    // Pay the morph/megamorph turn-face-up cost.
    let mut cost_ctx = CostContext::new(permanent_id, player, decision_maker);
    for cost in spec.cost.costs() {
        pay_cost_component_with_choice(game, cost, &mut cost_ctx)
            .map_err(cost_error_to_action_error)?;
    }

    game.set_face_up(permanent_id);

    if spec.megamorph
        && let Some(object) = game.object_mut(permanent_id)
    {
        object.add_counters(crate::object::CounterType::PlusOnePlusOne, 1);
    }

    game.queue_trigger_event(TriggerEvent::new(crate::events::TurnedFaceUpEvent::new(
        permanent_id,
        player,
    )));

    Ok(())
}

// === Suspend ===

fn can_suspend(game: &GameState, player: PlayerId, card_id: ObjectId) -> Result<(), ActionError> {
    // Must have priority
    if game.turn.priority_player != Some(player) {
        return Err(ActionError::NotYourPriority);
    }

    // Check the card exists and is in hand
    let object = game.object(card_id).ok_or(ActionError::ObjectNotFound)?;
    if object.zone != Zone::Hand {
        return Err(ActionError::WrongZone {
            expected: Zone::Hand,
            actual: object.zone,
        });
    }

    // Check the player owns the card
    if object.owner != player {
        return Err(ActionError::InvalidTarget);
    }

    // Note: In full implementation, would check if card has suspend ability
    // and if player can pay the suspend cost

    Ok(())
}

fn perform_suspend(
    game: &mut GameState,
    _player: PlayerId,
    card_id: ObjectId,
) -> Result<(), ActionError> {
    // Move to exile
    let new_id = game
        .move_object(card_id, Zone::Exile)
        .ok_or(ActionError::ObjectNotFound)?;

    // In a full implementation, this would:
    // 1. Pay the suspend cost
    // 2. Add time counters equal to the suspend number
    // 3. Mark the card as suspended (for tracking "remove a time counter at upkeep")

    // For now, just mark it as exiled
    let _ = new_id;

    Ok(())
}

// === Foretell ===

fn can_foretell(game: &GameState, player: PlayerId, card_id: ObjectId) -> Result<(), ActionError> {
    // Must be during your turn
    if game.turn.active_player != player {
        return Err(ActionError::NotActivePlayer);
    }

    // Must have priority
    if game.turn.priority_player != Some(player) {
        return Err(ActionError::NotYourPriority);
    }

    // Check the card exists and is in hand
    let object = game.object(card_id).ok_or(ActionError::ObjectNotFound)?;
    if object.zone != Zone::Hand {
        return Err(ActionError::WrongZone {
            expected: Zone::Hand,
            actual: object.zone,
        });
    }

    // Check the player owns the card
    if object.owner != player {
        return Err(ActionError::InvalidTarget);
    }

    // Note: In full implementation, would check:
    // 1. Card has foretell ability
    // 2. Player can pay {2} for the foretell cost
    // 3. Haven't already foretold this turn (once per turn restriction)

    Ok(())
}

fn perform_foretell(
    game: &mut GameState,
    _player: PlayerId,
    card_id: ObjectId,
) -> Result<(), ActionError> {
    // Move to exile face-down
    let new_id = game
        .move_object(card_id, Zone::Exile)
        .ok_or(ActionError::ObjectNotFound)?;

    // Mark as face-down (foretold)
    game.set_face_down(new_id);

    // In a full implementation, this would:
    // 1. Pay {2} (the foretell cost)
    // 2. Track that this card was foretold (for later casting at foretell cost)

    Ok(())
}

// === Activate Mana Ability ===

/// Convert a CostPaymentError to an ActionError.
fn cost_error_to_action_error(err: CostPaymentError) -> ActionError {
    match err {
        CostPaymentError::SourceNotFound => ActionError::ObjectNotFound,
        CostPaymentError::PlayerNotFound => ActionError::PlayerNotFound,
        CostPaymentError::AlreadyTapped => ActionError::CantPayCost,
        CostPaymentError::SummoningSickness => ActionError::SummoningSickness,
        CostPaymentError::AlreadyUntapped => ActionError::CantPayCost,
        CostPaymentError::InsufficientMana => ActionError::CantPayCost,
        CostPaymentError::InsufficientLife => ActionError::CantPayCost,
        CostPaymentError::SourceNotOnBattlefield => ActionError::CantPayCost,
        CostPaymentError::NoValidSacrificeTarget => ActionError::CantPayCost,
        CostPaymentError::InsufficientCardsInHand => ActionError::CantPayCost,
        CostPaymentError::InsufficientCounters => ActionError::CantPayCost,
        CostPaymentError::InsufficientEnergy => ActionError::CantPayCost,
        CostPaymentError::InsufficientCardsToExile => ActionError::CantPayCost,
        CostPaymentError::InsufficientCardsInGraveyard => ActionError::CantPayCost,
        CostPaymentError::NoValidReturnTarget => ActionError::CantPayCost,
        CostPaymentError::InsufficientCardsToReveal => ActionError::CantPayCost,
        CostPaymentError::Other(_) => ActionError::CantPayCost,
    }
}

fn can_activate_mana_ability(
    game: &GameState,
    player: PlayerId,
    permanent_id: ObjectId,
    ability_index: usize,
    decision_maker: &mut impl crate::decision::DecisionMaker,
) -> Result<(), ActionError> {
    // Check the permanent exists and is on the battlefield
    let object = game
        .object(permanent_id)
        .ok_or(ActionError::ObjectNotFound)?;
    if object.zone != Zone::Battlefield {
        return Err(ActionError::WrongZone {
            expected: Zone::Battlefield,
            actual: object.zone,
        });
    }

    // Check the player controls the permanent
    if object.controller != player {
        return Err(ActionError::InvalidTarget);
    }

    // Check the ability exists and is a mana ability
    let ability = object
        .abilities
        .get(ability_index)
        .ok_or(ActionError::NoSuchAbility)?;

    if !ability.is_mana_ability() {
        return Err(ActionError::NoSuchAbility);
    }

    // Check if the cost can be paid
    if let crate::ability::AbilityKind::Mana(mana_ability) = &ability.kind {
        // Check mana costs from TotalCost (for abilities like Blood Celebrant that cost {B})
        let ctx = CostContext::new(permanent_id, player, decision_maker);
        for cost in mana_ability.mana_cost.costs() {
            // For mana costs, use can_potentially_pay to show abilities that could
            // be activated after tapping mana sources.
            if cost.processing_mode().is_mana_payment() {
                cost.can_potentially_pay(game, &ctx)
                    .map_err(cost_error_to_action_error)?;
            } else {
                cost.can_pay(game, &ctx)
                    .map_err(cost_error_to_action_error)?;
            }
        }

        // Check activation condition if present
        if let Some(condition) = &mana_ability.activation_condition
            && !check_mana_ability_condition(game, player, condition)
        {
            return Err(ActionError::CantPayCost);
        }
    }

    Ok(())
}

/// Check if a mana ability can be activated (for query/legality checks).
///
/// This variant doesn't require a decision_maker because it only checks costs.
fn can_activate_mana_ability_check(
    game: &GameState,
    player: PlayerId,
    permanent_id: ObjectId,
    ability_index: usize,
) -> Result<(), ActionError> {
    use crate::costs::{
        CostCheckContext, can_pay_with_check_context, can_potentially_pay_with_check_context,
    };

    // Check the permanent exists and is on the battlefield
    let object = game
        .object(permanent_id)
        .ok_or(ActionError::ObjectNotFound)?;
    if object.zone != Zone::Battlefield {
        return Err(ActionError::WrongZone {
            expected: Zone::Battlefield,
            actual: object.zone,
        });
    }

    // Check the player controls the permanent
    if object.controller != player {
        return Err(ActionError::InvalidTarget);
    }

    // Check the ability exists and is a mana ability
    let ability = object
        .abilities
        .get(ability_index)
        .ok_or(ActionError::NoSuchAbility)?;

    if !ability.is_mana_ability() {
        return Err(ActionError::NoSuchAbility);
    }

    // Check if the cost can be paid
    if let crate::ability::AbilityKind::Mana(mana_ability) = &ability.kind {
        // Check mana costs from TotalCost (for abilities like Blood Celebrant that cost {B})
        let ctx = CostCheckContext::new(permanent_id, player);
        for cost in mana_ability.mana_cost.costs() {
            // For mana costs, use can_potentially_pay to show abilities that could
            // be activated after tapping mana sources.
            if cost.processing_mode().is_mana_payment() {
                can_potentially_pay_with_check_context(&*cost.0, game, &ctx)
                    .map_err(cost_error_to_action_error)?;
            } else {
                can_pay_with_check_context(&*cost.0, game, &ctx)
                    .map_err(cost_error_to_action_error)?;
            }
        }

        // Check activation condition if present
        if let Some(condition) = &mana_ability.activation_condition
            && !check_mana_ability_condition(game, player, condition)
        {
            return Err(ActionError::CantPayCost);
        }
    }

    Ok(())
}

/// Check if a mana ability's activation condition is met.
fn check_mana_ability_condition(
    game: &GameState,
    player: PlayerId,
    condition: &crate::ability::ManaAbilityCondition,
) -> bool {
    match condition {
        crate::ability::ManaAbilityCondition::ControlLandWithSubtype(required_subtypes) => {
            // Check if the player controls a land with at least one of the required subtypes
            game.battlefield.iter().any(|&id| {
                if let Some(obj) = game.object(id) {
                    if obj.controller == player && obj.is_land() {
                        // Check if the land has any of the required subtypes
                        required_subtypes
                            .iter()
                            .any(|subtype| obj.has_subtype(*subtype))
                    } else {
                        false
                    }
                } else {
                    false
                }
            })
        }
        crate::ability::ManaAbilityCondition::ControlAtLeastArtifacts(required_count) => {
            let controlled_artifacts = game
                .battlefield
                .iter()
                .filter_map(|&id| game.object(id))
                .filter(|obj| {
                    obj.controller == player
                        && obj.card_types.contains(&crate::types::CardType::Artifact)
                })
                .count() as u32;
            controlled_artifacts >= *required_count
        }
        crate::ability::ManaAbilityCondition::ControlAtLeastLands(required_count) => {
            let controlled_lands = game
                .battlefield
                .iter()
                .filter_map(|&id| game.object(id))
                .filter(|obj| obj.controller == player && obj.is_land())
                .count() as u32;
            controlled_lands >= *required_count
        }
        crate::ability::ManaAbilityCondition::ControlCreatureWithPowerAtLeast(required_power) => {
            game.battlefield.iter().any(|&id| {
                game.object(id).is_some_and(|obj| {
                    obj.controller == player
                        && obj.is_creature()
                        && obj.power().is_some_and(|power| power >= *required_power as i32)
                })
            })
        }
        crate::ability::ManaAbilityCondition::ControlCreaturesTotalPowerAtLeast(required_power) => {
            let total_power = game
                .battlefield
                .iter()
                .filter_map(|&id| game.object(id))
                .filter(|obj| obj.controller == player && obj.is_creature())
                .map(|obj| obj.power().unwrap_or(0).max(0))
                .sum::<i32>();
            total_power >= *required_power as i32
        }
        crate::ability::ManaAbilityCondition::CardInYourGraveyard {
            card_types,
            subtypes,
        } => game.player(player).is_some_and(|player_state| {
            player_state.graveyard.iter().any(|&card_id| {
                let Some(card) = game.object(card_id) else {
                    return false;
                };
                let card_type_match = card_types.is_empty()
                    || card_types
                        .iter()
                        .any(|card_type| card.card_types.contains(card_type));
                let subtype_match = subtypes.is_empty()
                    || subtypes.iter().any(|subtype| card.has_subtype(*subtype));
                card_type_match && subtype_match
            })
        }),
        crate::ability::ManaAbilityCondition::Timing(timing) => match timing {
            crate::ability::ActivationTiming::AnyTime => true,
            crate::ability::ActivationTiming::DuringCombat => {
                matches!(game.turn.phase, Phase::Combat)
            }
            crate::ability::ActivationTiming::SorcerySpeed => {
                game.turn.active_player == player
                    && matches!(game.turn.phase, Phase::FirstMain | Phase::NextMain)
                    && game.stack_is_empty()
            }
            crate::ability::ActivationTiming::OncePerTurn => true,
            crate::ability::ActivationTiming::DuringYourTurn => game.turn.active_player == player,
            crate::ability::ActivationTiming::DuringOpponentsTurn => {
                game.turn.active_player != player
            }
        },
        crate::ability::ManaAbilityCondition::All(conditions) => conditions
            .iter()
            .all(|inner| check_mana_ability_condition(game, player, inner)),
    }
}

pub fn perform_activate_mana_ability(
    game: &mut GameState,
    player: PlayerId,
    permanent_id: ObjectId,
    ability_index: usize,
    decision_maker: &mut impl crate::decision::DecisionMaker,
) -> Result<(), ActionError> {
    use crate::executor::ExecutionContext;

    // Get the mana ability details
    let object = game
        .object(permanent_id)
        .ok_or(ActionError::ObjectNotFound)?;
    let ability = object
        .abilities
        .get(ability_index)
        .ok_or(ActionError::NoSuchAbility)?;

    if let crate::ability::AbilityKind::Mana(mana_ability) = &ability.kind {
        let total_cost = mana_ability.mana_cost.clone();
        let additional_effects = mana_ability.effects.clone();
        let mana = mana_ability.mana.clone();

        // Pay mana costs from TotalCost (for abilities like Blood Celebrant that cost {B})
        let mut cost_ctx = CostContext::new(permanent_id, player, decision_maker);
        for cost in total_cost.costs() {
            pay_cost_component_with_choice(game, cost, &mut cost_ctx)
                .map_err(cost_error_to_action_error)?;
        }
        let x_value_from_costs = cost_ctx.x_value;
        drop(cost_ctx);

        // Add mana to player's pool
        if let Some(player_data) = game.player_mut(player) {
            for symbol in mana {
                player_data.mana_pool.add(symbol, 1);
            }
        }

        // Execute additional effects if present (for complex mana abilities like Ancient Tomb)
        if let Some(effects) = additional_effects {
            let mut effect_ctx = ExecutionContext::new(permanent_id, player, decision_maker);
            if let Some(x) = x_value_from_costs {
                effect_ctx = effect_ctx.with_x(x);
            }
            for effect in effects {
                // Ignore effect execution errors for mana abilities
                let _ = effect.0.execute(game, &mut effect_ctx);
            }
        }

        Ok(())
    } else {
        Err(ActionError::NoSuchAbility)
    }
}

/// Pay a single cost component, resolving any required choices via decision specs.
///
/// This is shared between special-actions and game-loop mana-ability paths so
/// choice-based costs (discard/sacrifice/exile from hand) follow one path.
pub(crate) fn pay_cost_component_with_choice(
    game: &mut GameState,
    cost: &crate::costs::Cost,
    ctx: &mut CostContext,
) -> Result<(), CostPaymentError> {
    match cost.pay(game, ctx)? {
        CostPaymentResult::Paid => Ok(()),
        CostPaymentResult::NeedsChoice(_) => resolve_cost_choice(game, cost, ctx),
    }
}

fn resolve_cost_choice(
    game: &mut GameState,
    cost: &crate::costs::Cost,
    ctx: &mut CostContext,
) -> Result<(), CostPaymentError> {
    use crate::costs::CostProcessingMode;

    match cost.processing_mode() {
        CostProcessingMode::SacrificeTarget { filter } => {
            let candidates = legal_sacrifice_targets(game, ctx.payer, ctx.source, &filter);
            if candidates.is_empty() {
                return Err(CostPaymentError::NoValidSacrificeTarget);
            }

            let spec = ChooseObjectsSpec::new(
                ctx.source,
                format!("Choose {} to sacrifice", describe_permanent_filter(&filter)),
                candidates.clone(),
                1,
                Some(1),
            );
            let chosen: Vec<ObjectId> =
                make_decision(game, ctx.decision_maker, ctx.payer, Some(ctx.source), spec);
            let Some(target_id) = normalize_selection(chosen, &candidates, 1).first().copied()
            else {
                return Err(CostPaymentError::NoValidSacrificeTarget);
            };

            match process_zone_change(
                game,
                target_id,
                Zone::Battlefield,
                Zone::Graveyard,
                ctx.decision_maker,
            ) {
                EventOutcome::Prevented | EventOutcome::NotApplicable => {
                    Err(CostPaymentError::NoValidSacrificeTarget)
                }
                EventOutcome::Proceed(final_zone) => {
                    let snapshot = game
                        .object(target_id)
                        .map(|obj| ObjectSnapshot::from_object(obj, game));
                    let sacrificing_player = snapshot
                        .as_ref()
                        .map(|snap| snap.controller)
                        .or(Some(ctx.payer));
                    game.move_object(target_id, final_zone);
                    if final_zone == Zone::Graveyard {
                        game.queue_trigger_event(TriggerEvent::new(
                            SacrificeEvent::new(target_id, Some(ctx.source))
                                .with_snapshot(snapshot, sacrificing_player),
                        ));
                    }
                    Ok(())
                }
                EventOutcome::Replaced => Ok(()),
            }
        }
        CostProcessingMode::DiscardCards { count, card_type } => {
            let candidates = legal_discard_cards(game, ctx.payer, ctx.source, card_type);
            let required = (count as usize).min(candidates.len());
            if required < count as usize {
                return Err(CostPaymentError::InsufficientCardsInHand);
            }
            if required == 0 {
                return Ok(());
            }

            let spec = ChooseObjectsSpec::new(
                ctx.source,
                format!(
                    "Choose {} card{} to discard",
                    required,
                    if required == 1 { "" } else { "s" }
                ),
                candidates.clone(),
                required,
                Some(required),
            );
            let chosen: Vec<ObjectId> =
                make_decision(game, ctx.decision_maker, ctx.payer, Some(ctx.source), spec);
            let to_discard = normalize_selection(chosen, &candidates, required);

            if to_discard.len() != required {
                return Err(CostPaymentError::InsufficientCardsInHand);
            }

            let cause = EventCause::from_cost(ctx.source, ctx.payer);
            for card_id in to_discard {
                let result = execute_discard(
                    game,
                    card_id,
                    ctx.payer,
                    cause.clone(),
                    false,
                    ctx.decision_maker,
                );
                if result.prevented {
                    return Err(CostPaymentError::Other(
                        "Discard cost was prevented".to_string(),
                    ));
                }
            }
            Ok(())
        }
        CostProcessingMode::ExileFromHand {
            count,
            color_filter,
        } => {
            let candidates = legal_exile_cards(game, ctx.payer, ctx.source, color_filter);
            let required = count as usize;
            if candidates.len() < required {
                return Err(CostPaymentError::InsufficientCardsToExile);
            }

            let spec = ChooseObjectsSpec::new(
                ctx.source,
                format!(
                    "Choose {} card{} to exile from your hand",
                    required,
                    if required == 1 { "" } else { "s" }
                ),
                candidates.clone(),
                required,
                Some(required),
            );
            let chosen: Vec<ObjectId> =
                make_decision(game, ctx.decision_maker, ctx.payer, Some(ctx.source), spec);
            let to_exile = normalize_selection(chosen, &candidates, required);
            if to_exile.len() != required {
                return Err(CostPaymentError::InsufficientCardsToExile);
            }

            ctx.pre_chosen_cards.extend(to_exile);
            match cost.pay(game, ctx)? {
                CostPaymentResult::Paid => Ok(()),
                CostPaymentResult::NeedsChoice(_) => Err(CostPaymentError::Other(
                    "Exile-from-hand cost still needs choice after preselection".to_string(),
                )),
            }
        }
        CostProcessingMode::ManaPayment { .. }
        | CostProcessingMode::Immediate
        | CostProcessingMode::InlineWithTriggers => Err(CostPaymentError::Other(
            "Cost unexpectedly requested choice in non-choice mode".to_string(),
        )),
    }
}

fn legal_sacrifice_targets(
    game: &GameState,
    payer: PlayerId,
    source: ObjectId,
    filter: &crate::cost::PermanentFilter,
) -> Vec<ObjectId> {
    game.battlefield
        .iter()
        .filter(|&&id| {
            if let Some(obj) = game.object(id) {
                if obj.controller != payer {
                    return false;
                }
                if filter.other && id == source {
                    return false;
                }
                if !filter.card_types.is_empty()
                    && !filter.card_types.iter().any(|t| obj.has_card_type(*t))
                {
                    return false;
                }
                if !filter.subtypes.is_empty()
                    && !filter.subtypes.iter().any(|s| obj.subtypes.contains(s))
                {
                    return false;
                }
                if filter.token && obj.kind != crate::object::ObjectKind::Token {
                    return false;
                }
                if filter.nontoken && obj.kind == crate::object::ObjectKind::Token {
                    return false;
                }
                game.can_be_sacrificed(id)
            } else {
                false
            }
        })
        .copied()
        .collect()
}

fn legal_discard_cards(
    game: &GameState,
    payer: PlayerId,
    source: ObjectId,
    card_type: Option<crate::types::CardType>,
) -> Vec<ObjectId> {
    game.player(payer)
        .map(|p| {
            p.hand
                .iter()
                .copied()
                .filter(|&card_id| {
                    if card_id == source {
                        return false;
                    }
                    if let Some(ct) = card_type {
                        return game
                            .object(card_id)
                            .is_some_and(|obj| obj.has_card_type(ct));
                    }
                    true
                })
                .collect()
        })
        .unwrap_or_default()
}

fn legal_exile_cards(
    game: &GameState,
    payer: PlayerId,
    source: ObjectId,
    color_filter: Option<crate::color::ColorSet>,
) -> Vec<ObjectId> {
    game.player(payer)
        .map(|p| {
            p.hand
                .iter()
                .copied()
                .filter(|&card_id| {
                    if card_id == source {
                        return false;
                    }
                    if let Some(required) = color_filter {
                        return game.object(card_id).is_some_and(|obj| {
                            let colors = obj.colors();
                            !colors.intersection(required).is_empty()
                        });
                    }
                    true
                })
                .collect()
        })
        .unwrap_or_default()
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

fn describe_permanent_filter(filter: &crate::cost::PermanentFilter) -> String {
    let mut parts: Vec<String> = Vec::new();

    if filter.other {
        parts.push("another".to_string());
    }
    if filter.nontoken {
        parts.push("nontoken".to_string());
    }
    if filter.token {
        parts.push("token".to_string());
    }
    if !filter.card_types.is_empty() {
        let types = filter
            .card_types
            .iter()
            .map(|t| format!("{:?}", t).to_lowercase())
            .collect::<Vec<_>>()
            .join(" or ");
        parts.push(types);
    } else {
        parts.push("permanent".to_string());
    }

    parts.join(" ")
}

#[cfg(test)]
mod tests {
    use super::{can_perform_check, *};
    use crate::ability::Ability;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::cost::TotalCost;
    use crate::ids::CardId;
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::types::Subtype;

    fn basic_forest() -> crate::card::Card {
        CardBuilder::new(CardId::from_raw(1), "Forest")
            .card_types(vec![CardType::Land])
            .subtypes(vec![Subtype::Forest])
            .build()
    }

    fn grizzly_bears() -> crate::card::Card {
        CardBuilder::new(CardId::from_raw(2), "Grizzly Bears")
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Green],
            ]))
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Bear])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build()
    }

    /// Creates a Llanowar Elves-like creature with a tap-for-G mana ability
    fn llanowar_elves() -> crate::card::Card {
        CardBuilder::new(CardId::from_raw(3), "Llanowar Elves")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Green]]))
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Elf, Subtype::Druid])
            .power_toughness(PowerToughness::fixed(1, 1))
            .build()
    }

    #[test]
    fn test_can_play_land_success() {
        let mut game = GameState::new(vec!["Alice".to_string()], 20);
        let alice = PlayerId::from_index(0);

        // Set up main phase with empty stack
        game.turn.phase = Phase::FirstMain;
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);

        // Add a forest to hand
        let forest = basic_forest();
        let card_id = game.create_object_from_card(&forest, alice, Zone::Hand);

        let action = SpecialAction::PlayLand { card_id };
        assert!(can_perform_check(&action, &game, alice).is_ok());
    }

    #[test]
    fn test_play_land_wrong_phase() {
        let mut game = GameState::new(vec!["Alice".to_string()], 20);
        let alice = PlayerId::from_index(0);

        // Set up combat phase
        game.turn.phase = Phase::Combat;
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);

        let forest = basic_forest();
        let card_id = game.create_object_from_card(&forest, alice, Zone::Hand);

        let action = SpecialAction::PlayLand { card_id };
        let result = can_perform_check(&action, &game, alice);
        assert!(matches!(result, Err(ActionError::WrongPhase { .. })));
    }

    #[test]
    fn test_play_land_already_played() {
        let mut game = GameState::new(vec!["Alice".to_string()], 20);
        let alice = PlayerId::from_index(0);

        game.turn.phase = Phase::FirstMain;
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);

        // Record that a land was already played
        if let Some(player) = game.player_mut(alice) {
            player.record_land_play();
        }

        let forest = basic_forest();
        let card_id = game.create_object_from_card(&forest, alice, Zone::Hand);

        let action = SpecialAction::PlayLand { card_id };
        let result = can_perform_check(&action, &game, alice);
        assert!(matches!(result, Err(ActionError::AlreadyPlayedLand)));
    }

    #[test]
    fn test_play_land_not_a_land() {
        let mut game = GameState::new(vec!["Alice".to_string()], 20);
        let alice = PlayerId::from_index(0);

        game.turn.phase = Phase::FirstMain;
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);

        let bears = grizzly_bears();
        let card_id = game.create_object_from_card(&bears, alice, Zone::Hand);

        let action = SpecialAction::PlayLand { card_id };
        let result = can_perform_check(&action, &game, alice);
        assert!(matches!(result, Err(ActionError::NotALand)));
    }

    #[test]
    fn test_play_land_stack_not_empty() {
        let mut game = GameState::new(vec!["Alice".to_string()], 20);
        let alice = PlayerId::from_index(0);

        game.turn.phase = Phase::FirstMain;
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);

        // Put something on the stack
        let bears = grizzly_bears();
        let spell_id = game.create_object_from_card(&bears, alice, Zone::Stack);
        game.push_to_stack(crate::game_state::StackEntry::new(spell_id, alice));

        let forest = basic_forest();
        let card_id = game.create_object_from_card(&forest, alice, Zone::Hand);

        let action = SpecialAction::PlayLand { card_id };
        let result = can_perform_check(&action, &game, alice);
        assert!(matches!(result, Err(ActionError::StackNotEmpty)));
    }

    #[test]
    fn test_perform_play_land() {
        let mut game = GameState::new(vec!["Alice".to_string()], 20);
        let alice = PlayerId::from_index(0);

        game.turn.phase = Phase::FirstMain;
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);

        let forest = basic_forest();
        let card_id = game.create_object_from_card(&forest, alice, Zone::Hand);

        let action = SpecialAction::PlayLand { card_id };
        let mut dm = crate::decision::SelectFirstDecisionMaker;
        assert!(perform(action, &mut game, alice, &mut dm).is_ok());

        // Verify land is on battlefield
        assert_eq!(game.battlefield.len(), 1);

        // Verify player has played a land
        let player = game.player(alice).unwrap();
        assert!(!player.can_play_land());
    }

    #[test]
    fn test_can_activate_mana_ability() {
        let mut game = GameState::new(vec!["Alice".to_string()], 20);
        let alice = PlayerId::from_index(0);

        // Create a forest-like permanent with a mana ability
        let forest = basic_forest();
        let obj_id = game.create_object_from_card(&forest, alice, Zone::Battlefield);

        // Add a mana ability to the object
        if let Some(obj) = game.object_mut(obj_id) {
            obj.abilities
                .push(Ability::mana(TotalCost::free(), vec![ManaSymbol::Green]));
        }

        let action = SpecialAction::ActivateManaAbility {
            permanent_id: obj_id,
            ability_index: 0,
        };
        assert!(can_perform_check(&action, &game, alice).is_ok());
    }

    #[test]
    fn test_activate_mana_ability_tapped() {
        let mut game = GameState::new(vec!["Alice".to_string()], 20);
        let alice = PlayerId::from_index(0);

        let forest = basic_forest();
        let obj_id = game.create_object_from_card(&forest, alice, Zone::Battlefield);

        // Add mana ability and tap the permanent
        if let Some(obj) = game.object_mut(obj_id) {
            obj.abilities
                .push(Ability::mana(TotalCost::free(), vec![ManaSymbol::Green]));
        }
        game.tap(obj_id);

        let action = SpecialAction::ActivateManaAbility {
            permanent_id: obj_id,
            ability_index: 0,
        };
        let result = can_perform_check(&action, &game, alice);
        assert!(matches!(result, Err(ActionError::CantPayCost)));
    }

    #[test]
    fn test_mana_ability_summoning_sickness_blocks_creature() {
        let mut game = GameState::new(vec!["Alice".to_string()], 20);
        let alice = PlayerId::from_index(0);

        // Create Llanowar Elves (a creature with a tap mana ability)
        let elves = llanowar_elves();
        let obj_id = game.create_object_from_card(&elves, alice, Zone::Battlefield);

        // Add the tap-for-green mana ability and set summoning sickness
        if let Some(obj) = game.object_mut(obj_id) {
            obj.abilities
                .push(Ability::mana(TotalCost::free(), vec![ManaSymbol::Green]));
        }
        game.set_summoning_sick(obj_id); // Simulate just entering battlefield

        // The creature should have summoning sickness
        assert!(game.is_summoning_sick(obj_id));
        assert!(game.object(obj_id).unwrap().is_creature());

        let action = SpecialAction::ActivateManaAbility {
            permanent_id: obj_id,
            ability_index: 0,
        };
        let result = can_perform_check(&action, &game, alice);
        assert!(
            matches!(result, Err(ActionError::SummoningSickness)),
            "Expected SummoningSickness error for creature with tap mana ability, got: {:?}",
            result
        );
    }

    #[test]
    fn test_mana_ability_haste_bypasses_summoning_sickness() {
        use crate::ability::Ability;
        use crate::static_abilities::StaticAbility;

        let mut game = GameState::new(vec!["Alice".to_string()], 20);
        let alice = PlayerId::from_index(0);

        // Create Llanowar Elves and give it haste
        let elves = llanowar_elves();
        let obj_id = game.create_object_from_card(&elves, alice, Zone::Battlefield);

        // Add mana ability, haste ability, and set summoning sickness
        if let Some(obj) = game.object_mut(obj_id) {
            obj.abilities
                .push(Ability::mana(TotalCost::free(), vec![ManaSymbol::Green]));
            obj.abilities
                .push(Ability::static_ability(StaticAbility::haste()));
        }
        game.set_summoning_sick(obj_id); // Simulate just entering battlefield

        assert!(game.is_summoning_sick(obj_id));
        assert!(game.object(obj_id).unwrap().is_creature());

        let action = SpecialAction::ActivateManaAbility {
            permanent_id: obj_id,
            ability_index: 0,
        };
        assert!(
            can_perform_check(&action, &game, alice).is_ok(),
            "Haste should bypass summoning sickness for mana abilities"
        );
    }

    #[test]
    fn test_mana_ability_land_not_affected_by_summoning_sickness() {
        let mut game = GameState::new(vec!["Alice".to_string()], 20);
        let alice = PlayerId::from_index(0);

        // Create a forest (not a creature)
        let forest = basic_forest();
        let obj_id = game.create_object_from_card(&forest, alice, Zone::Battlefield);

        // Add a mana ability and set summoning sickness
        if let Some(obj) = game.object_mut(obj_id) {
            obj.abilities
                .push(Ability::mana(TotalCost::free(), vec![ManaSymbol::Green]));
        }
        game.set_summoning_sick(obj_id); // Simulate just entering battlefield

        // Even though summoning_sick flag is true, lands are not creatures
        // so rule 302.6 doesn't apply to them
        assert!(game.is_summoning_sick(obj_id));
        assert!(!game.object(obj_id).unwrap().is_creature());

        let action = SpecialAction::ActivateManaAbility {
            permanent_id: obj_id,
            ability_index: 0,
        };
        assert!(
            can_perform_check(&action, &game, alice).is_ok(),
            "Lands should not be affected by summoning sickness"
        );
    }

    #[test]
    fn test_perform_mana_ability() {
        let mut game = GameState::new(vec!["Alice".to_string()], 20);
        let alice = PlayerId::from_index(0);

        let forest = basic_forest();
        let obj_id = game.create_object_from_card(&forest, alice, Zone::Battlefield);

        // Add a mana ability
        if let Some(obj) = game.object_mut(obj_id) {
            obj.abilities
                .push(Ability::mana(TotalCost::free(), vec![ManaSymbol::Green]));
        }

        let action = SpecialAction::ActivateManaAbility {
            permanent_id: obj_id,
            ability_index: 0,
        };
        let mut dm = crate::decision::SelectFirstDecisionMaker;
        assert!(perform(action, &mut game, alice, &mut dm).is_ok());

        // Verify permanent is tapped
        assert!(game.is_tapped(obj_id));

        // Verify mana was added
        let player = game.player(alice).unwrap();
        assert_eq!(player.mana_pool.green, 1);
    }

    #[test]
    fn test_turn_face_up() {
        use crate::static_abilities::StaticAbility;

        let mut game = GameState::new(vec!["Alice".to_string()], 20);
        let alice = PlayerId::from_index(0);

        let bears = grizzly_bears();
        let obj_id = game.create_object_from_card(&bears, alice, Zone::Battlefield);

        if let Some(obj) = game.object_mut(obj_id) {
            obj.abilities
                .push(Ability::static_ability(StaticAbility::morph(
                    ManaCost::from_pips(vec![vec![ManaSymbol::Green]]),
                )));
        }
        if let Some(player) = game.player_mut(alice) {
            player.mana_pool.green = 1;
        }
        game.turn.priority_player = Some(alice);

        // Make the creature face-down
        game.set_face_down(obj_id);

        let action = SpecialAction::TurnFaceUp {
            permanent_id: obj_id,
        };
        assert!(can_perform_check(&action, &game, alice).is_ok());
        let mut dm = crate::decision::SelectFirstDecisionMaker;
        assert!(perform(action, &mut game, alice, &mut dm).is_ok());

        // Verify it's face-up
        assert!(!game.is_face_down(obj_id));
    }

    #[test]
    fn test_turn_face_up_megamorph_adds_counter() {
        use crate::object::CounterType;
        use crate::static_abilities::StaticAbility;

        let mut game = GameState::new(vec!["Alice".to_string()], 20);
        let alice = PlayerId::from_index(0);

        let bears = grizzly_bears();
        let obj_id = game.create_object_from_card(&bears, alice, Zone::Battlefield);

        if let Some(obj) = game.object_mut(obj_id) {
            obj.abilities
                .push(Ability::static_ability(StaticAbility::megamorph(
                    ManaCost::from_pips(vec![vec![ManaSymbol::Green]]),
                )));
        }
        if let Some(player) = game.player_mut(alice) {
            player.mana_pool.green = 1;
        }
        game.turn.priority_player = Some(alice);
        game.set_face_down(obj_id);

        let action = SpecialAction::TurnFaceUp {
            permanent_id: obj_id,
        };
        let mut dm = crate::decision::SelectFirstDecisionMaker;
        assert!(perform(action, &mut game, alice, &mut dm).is_ok());

        let counters = game
            .object(obj_id)
            .and_then(|obj| obj.counters.get(&CounterType::PlusOnePlusOne))
            .copied()
            .unwrap_or(0);
        assert_eq!(counters, 1, "megamorph should add a +1/+1 counter");
    }

    #[test]
    fn test_turn_face_up_is_special_action_not_stack_action() {
        use crate::static_abilities::StaticAbility;

        let mut game = GameState::new(vec!["Alice".to_string()], 20);
        let alice = PlayerId::from_index(0);

        let bears = grizzly_bears();
        let obj_id = game.create_object_from_card(&bears, alice, Zone::Battlefield);
        if let Some(obj) = game.object_mut(obj_id) {
            obj.abilities
                .push(Ability::static_ability(StaticAbility::morph(
                    ManaCost::from_pips(vec![vec![ManaSymbol::Green]]),
                )));
        }
        if let Some(player) = game.player_mut(alice) {
            player.mana_pool.green = 1;
        }
        game.turn.priority_player = Some(alice);
        game.set_face_down(obj_id);

        // Keep a spell on stack to verify turning face up does not use/modify it.
        let instant = CardBuilder::new(CardId::from_raw(77), "Test Instant")
            .card_types(vec![CardType::Instant])
            .build();
        let spell_id = game.create_object_from_card(&instant, alice, Zone::Stack);
        game.stack
            .push(crate::game_state::StackEntry::new(spell_id, alice));
        let stack_len_before = game.stack.len();

        let action = SpecialAction::TurnFaceUp {
            permanent_id: obj_id,
        };
        assert!(
            can_perform_check(&action, &game, alice).is_ok(),
            "turn-face-up should be legal with a non-empty stack when player has priority"
        );
        let mut dm = crate::decision::SelectFirstDecisionMaker;
        assert!(perform(action, &mut game, alice, &mut dm).is_ok());

        assert!(!game.is_face_down(obj_id), "creature should be face up");
        assert_eq!(
            game.stack.len(),
            stack_len_before,
            "turning face up should not add/remove stack entries"
        );
    }

    #[test]
    fn test_turn_face_up_queues_turned_face_up_event() {
        use crate::events::EventKind;
        use crate::static_abilities::StaticAbility;

        let mut game = GameState::new(vec!["Alice".to_string()], 20);
        let alice = PlayerId::from_index(0);

        let bears = grizzly_bears();
        let obj_id = game.create_object_from_card(&bears, alice, Zone::Battlefield);
        if let Some(obj) = game.object_mut(obj_id) {
            obj.abilities
                .push(Ability::static_ability(StaticAbility::morph(
                    ManaCost::from_pips(vec![vec![ManaSymbol::Green]]),
                )));
        }
        if let Some(player) = game.player_mut(alice) {
            player.mana_pool.green = 1;
        }
        game.turn.priority_player = Some(alice);
        game.set_face_down(obj_id);

        let action = SpecialAction::TurnFaceUp {
            permanent_id: obj_id,
        };
        let mut dm = crate::decision::SelectFirstDecisionMaker;
        assert!(perform(action, &mut game, alice, &mut dm).is_ok());

        let events = game.take_pending_trigger_events();
        assert!(
            events.iter().any(|event| {
                event.kind() == EventKind::TurnedFaceUp && event.object_id() == Some(obj_id)
            }),
            "turn-face-up special action should queue a TurnedFaceUp event"
        );
    }

    #[test]
    fn test_turn_face_up_not_face_down() {
        let mut game = GameState::new(vec!["Alice".to_string()], 20);
        let alice = PlayerId::from_index(0);

        let bears = grizzly_bears();
        let obj_id = game.create_object_from_card(&bears, alice, Zone::Battlefield);

        let action = SpecialAction::TurnFaceUp {
            permanent_id: obj_id,
        };
        let result = can_perform_check(&action, &game, alice);
        assert!(matches!(result, Err(ActionError::NotFaceDown)));
    }

    #[test]
    fn test_turn_face_up_requires_morph_or_megamorph() {
        let mut game = GameState::new(vec!["Alice".to_string()], 20);
        let alice = PlayerId::from_index(0);

        let bears = grizzly_bears();
        let obj_id = game.create_object_from_card(&bears, alice, Zone::Battlefield);
        game.turn.priority_player = Some(alice);
        game.set_face_down(obj_id);

        let action = SpecialAction::TurnFaceUp {
            permanent_id: obj_id,
        };
        let result = can_perform_check(&action, &game, alice);
        assert!(
            matches!(result, Err(ActionError::NoSuchAbility)),
            "expected NoSuchAbility when face-down permanent has no morph/megamorph"
        );
    }

    #[test]
    fn test_mana_ability_with_effects_ancient_tomb() {
        use crate::ability::AbilityKind;
        use crate::cards::CardRegistry;

        let mut game = GameState::new(vec!["Alice".to_string()], 20);
        let alice = PlayerId::from_index(0);

        // Create Ancient Tomb on the battlefield using the registry
        let registry = CardRegistry::with_builtin_cards();
        let def = registry
            .get("Ancient Tomb")
            .expect("Ancient Tomb should exist in registry");
        let obj_id = game.create_object_from_definition(def, alice, Zone::Battlefield);

        // Verify the object has the mana ability with effects
        let obj = game.object(obj_id).unwrap();
        assert_eq!(obj.abilities.len(), 1, "Should have 1 ability");

        if let AbilityKind::Mana(mana_ability) = &obj.abilities[0].kind {
            assert!(mana_ability.effects.is_some(), "Should have effects");
            let effects = mana_ability.effects.as_ref().unwrap();
            assert_eq!(
                effects.len(),
                2,
                "Should have 2 effects: add mana and deal damage"
            );
        } else {
            panic!("Expected mana ability");
        }

        let starting_life = game.player(alice).unwrap().life;
        assert_eq!(starting_life, 20);

        // Activate the mana ability
        let action = SpecialAction::ActivateManaAbility {
            permanent_id: obj_id,
            ability_index: 0,
        };
        let mut dm = crate::decision::SelectFirstDecisionMaker;
        assert!(perform(action, &mut game, alice, &mut dm).is_ok());

        // Verify permanent is tapped
        assert!(game.is_tapped(obj_id), "Ancient Tomb should be tapped");

        // Verify mana was added (2 colorless)
        let player = game.player(alice).unwrap();
        assert_eq!(
            player.mana_pool.colorless, 2,
            "Should have 2 colorless mana"
        );

        // Verify damage was dealt (2 damage to controller)
        assert_eq!(
            player.life,
            starting_life - 2,
            "Controller should have taken 2 damage"
        );
    }

    #[test]
    fn test_mana_ability_counter_cost_sets_x_for_effects() {
        use crate::effect::Value;
        use crate::object::CounterType;
        use crate::target::PlayerFilter;

        let mut game = GameState::new(vec!["Alice".to_string()], 20);
        let alice = PlayerId::from_index(0);

        let battery = CardBuilder::new(CardId::from_raw(88), "Counter Battery")
            .card_types(vec![CardType::Artifact])
            .build();
        let obj_id = game.create_object_from_card(&battery, alice, Zone::Battlefield);

        if let Some(obj) = game.object_mut(obj_id) {
            obj.add_counters(CounterType::Charge, 2);
            obj.abilities.push(Ability::mana_with_effects(
                TotalCost::from_cost(crate::costs::Cost::remove_counters(CounterType::Charge, 2)),
                vec![crate::effect::Effect::new(
                    crate::effects::mana::AddScaledManaEffect::new(
                        vec![ManaSymbol::Black],
                        Value::X,
                        PlayerFilter::You,
                    ),
                )],
            ));
        }

        let action = SpecialAction::ActivateManaAbility {
            permanent_id: obj_id,
            ability_index: 0,
        };
        let mut dm = crate::decision::SelectFirstDecisionMaker;
        assert!(perform(action, &mut game, alice, &mut dm).is_ok());

        let player = game.player(alice).expect("alice should exist");
        assert_eq!(player.mana_pool.black, 2, "expected X=2 black mana");

        let counters = game
            .object(obj_id)
            .and_then(|obj| obj.counters.get(&CounterType::Charge).copied())
            .unwrap_or(0);
        assert_eq!(counters, 0, "charge counters should be removed as a cost");
    }
}
