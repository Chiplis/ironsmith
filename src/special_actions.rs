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
use crate::filter::{FilterContext, ObjectFilter};
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

    // Check the object exists
    let object = game.object(card_id).ok_or(ActionError::ObjectNotFound)?;
    let can_play_from_zone = if object.zone == Zone::Hand {
        true
    } else {
        game.grant_registry
            .card_can_play_from_zone(game, card_id, object.zone, player)
    };
    if !can_play_from_zone {
        return Err(ActionError::WrongZone {
            expected: Zone::Hand,
            actual: object.zone,
        });
    }

    // Check the object is a land
    if !object.has_card_type(CardType::Land) {
        return Err(ActionError::NotALand);
    }

    // Normal land plays from hand require ownership. External permissions
    // (e.g. "you may play that card from exile") can bypass this.
    if object.zone == Zone::Hand && object.owner != player {
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
    let object = game
        .object(permanent_id)
        .ok_or(ActionError::ObjectNotFound)?;

    // Check the player controls the object
    if object.controller != player {
        return Err(ActionError::InvalidTarget);
    }

    // Rule restriction: activated abilities of this permanent can't be activated.
    if !game.can_activate_abilities_of(permanent_id) {
        return Err(ActionError::CantPayCost);
    }

    // Check the ability exists and is a mana ability
    let ability = object
        .abilities
        .get(ability_index)
        .ok_or(ActionError::NoSuchAbility)?;

    if !ability.is_mana_ability() {
        return Err(ActionError::NoSuchAbility);
    }

    // Check the ability functions in this zone
    if !ability.functions_in(&object.zone) {
        return Err(ActionError::WrongZone {
            expected: Zone::Battlefield,
            actual: object.zone,
        });
    }

    // Check if the cost can be paid
    if let crate::ability::AbilityKind::Activated(mana_ability) = &ability.kind
        && mana_ability.is_mana_ability()
    {
        if mana_ability.has_tap_cost() && !game.can_activate_tap_abilities_of(permanent_id) {
            return Err(ActionError::CantPayCost);
        }
        let total_cost = crate::decision::calculate_effective_activation_total_cost(
            game,
            player,
            permanent_id,
            &mana_ability.mana_cost,
        );
        // Check mana costs from TotalCost (for abilities like Blood Celebrant that cost {B})
        let ctx = CostContext::new(permanent_id, player, decision_maker);
        for cost in total_cost.costs() {
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
            && !check_mana_ability_condition(game, player, permanent_id, ability_index, condition)
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

    let object = game
        .object(permanent_id)
        .ok_or(ActionError::ObjectNotFound)?;

    // Check the player controls the object
    if object.controller != player {
        return Err(ActionError::InvalidTarget);
    }

    // Rule restriction: activated abilities of this permanent can't be activated.
    if !game.can_activate_abilities_of(permanent_id) {
        return Err(ActionError::CantPayCost);
    }

    // Check the ability exists and is a mana ability
    let ability = object
        .abilities
        .get(ability_index)
        .ok_or(ActionError::NoSuchAbility)?;

    if !ability.is_mana_ability() {
        return Err(ActionError::NoSuchAbility);
    }

    // Check the ability functions in this zone
    if !ability.functions_in(&object.zone) {
        return Err(ActionError::WrongZone {
            expected: Zone::Battlefield,
            actual: object.zone,
        });
    }

    // Check if the cost can be paid
    if let crate::ability::AbilityKind::Activated(mana_ability) = &ability.kind
        && mana_ability.is_mana_ability()
    {
        if mana_ability.has_tap_cost() && !game.can_activate_tap_abilities_of(permanent_id) {
            return Err(ActionError::CantPayCost);
        }
        let total_cost = crate::decision::calculate_effective_activation_total_cost(
            game,
            player,
            permanent_id,
            &mana_ability.mana_cost,
        );
        // Check mana costs from TotalCost (for abilities like Blood Celebrant that cost {B})
        let ctx = CostCheckContext::new(permanent_id, player);
        for cost in total_cost.costs() {
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
            && !check_mana_ability_condition(game, player, permanent_id, ability_index, condition)
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
    source: ObjectId,
    ability_index: usize,
    condition: &crate::ConditionExpr,
) -> bool {
    let eval_ctx = crate::condition_eval::ExternalEvaluationContext {
        controller: player,
        source,
        filter_source: Some(source),
        triggering_event: None,
        trigger_identity: None,
        ability_index: Some(ability_index),
        options: crate::condition_eval::ExternalEvaluationOptions::default(),
    };
    crate::condition_eval::evaluate_condition_external(game, condition, &eval_ctx)
}

pub fn perform_activate_mana_ability(
    game: &mut GameState,
    player: PlayerId,
    permanent_id: ObjectId,
    ability_index: usize,
    decision_maker: &mut impl crate::decision::DecisionMaker,
) -> Result<(), ActionError> {
    perform_activate_mana_ability_restricted_colors(
        game,
        player,
        permanent_id,
        ability_index,
        None,
        decision_maker,
    )
}

pub fn perform_activate_mana_ability_restricted_colors(
    game: &mut GameState,
    player: PlayerId,
    permanent_id: ObjectId,
    ability_index: usize,
    mana_color_restriction: Option<Vec<crate::color::Color>>,
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

    if let crate::ability::AbilityKind::Activated(mana_ability) = &ability.kind
        && mana_ability.is_mana_ability()
    {
        let total_cost = crate::decision::calculate_effective_activation_total_cost(
            game,
            player,
            permanent_id,
            &mana_ability.mana_cost,
        );
        let effects = mana_ability.effects.clone();
        let mana = mana_ability.mana_output.clone().unwrap_or_default();

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
        if !effects.is_empty() {
            let mut effect_ctx = ExecutionContext::new(permanent_id, player, decision_maker)
                .with_mana_color_restriction(mana_color_restriction.clone());
            if let Some(x) = x_value_from_costs {
                effect_ctx = effect_ctx.with_x(x);
            }
            for effect in effects {
                // Ignore effect execution errors for mana abilities
                let _ = effect.0.execute(game, &mut effect_ctx);
            }
        }

        game.record_ability_activation(permanent_id, ability_index);
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
        CostProcessingMode::DiscardCards { count, card_types } => {
            let candidates = legal_discard_cards(game, ctx.payer, ctx.source, &card_types);
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
        CostProcessingMode::ExileFromGraveyard { count, card_type } => {
            let candidates = legal_exile_from_graveyard_cards(game, ctx.payer, card_type);
            let required = count as usize;
            if candidates.len() < required {
                return Err(CostPaymentError::InsufficientCardsInGraveyard);
            }

            let spec = ChooseObjectsSpec::new(
                ctx.source,
                format!(
                    "Choose {} card{} to exile from your graveyard",
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
                return Err(CostPaymentError::InsufficientCardsInGraveyard);
            }

            ctx.pre_chosen_cards.extend(to_exile);
            match cost.pay(game, ctx)? {
                CostPaymentResult::Paid => Ok(()),
                CostPaymentResult::NeedsChoice(_) => Err(CostPaymentError::Other(
                    "Exile-from-graveyard cost still needs choice after preselection".to_string(),
                )),
            }
        }
        CostProcessingMode::RevealFromHand { count, card_type } => {
            let candidates = legal_reveal_cards(game, ctx.payer, ctx.source, card_type);
            let required = count as usize;
            if candidates.len() < required {
                return Err(CostPaymentError::InsufficientCardsToReveal);
            }

            let spec = ChooseObjectsSpec::new(
                ctx.source,
                format!(
                    "Choose {} card{} to reveal from your hand",
                    required,
                    if required == 1 { "" } else { "s" }
                ),
                candidates.clone(),
                required,
                Some(required),
            );
            let chosen: Vec<ObjectId> =
                make_decision(game, ctx.decision_maker, ctx.payer, Some(ctx.source), spec);
            let to_reveal = normalize_selection(chosen, &candidates, required);
            if to_reveal.len() != required {
                return Err(CostPaymentError::InsufficientCardsToReveal);
            }

            ctx.pre_chosen_cards.extend(to_reveal);
            match cost.pay(game, ctx)? {
                CostPaymentResult::Paid => Ok(()),
                CostPaymentResult::NeedsChoice(_) => Err(CostPaymentError::Other(
                    "Reveal cost still needs choice after preselection".to_string(),
                )),
            }
        }
        CostProcessingMode::ReturnToHandTarget { filter } => {
            let candidates = legal_return_targets(game, ctx.payer, ctx.source, &filter);
            if candidates.is_empty() {
                return Err(CostPaymentError::NoValidReturnTarget);
            }
            let spec = ChooseObjectsSpec::new(
                ctx.source,
                format!(
                    "Choose {} to return to hand",
                    describe_permanent_filter(&filter)
                ),
                candidates.clone(),
                1,
                Some(1),
            );
            let chosen: Vec<ObjectId> =
                make_decision(game, ctx.decision_maker, ctx.payer, Some(ctx.source), spec);
            let Some(target) = normalize_selection(chosen, &candidates, 1).first().copied() else {
                return Err(CostPaymentError::NoValidReturnTarget);
            };

            ctx.pre_chosen_cards.push(target);
            match cost.pay(game, ctx)? {
                CostPaymentResult::Paid => Ok(()),
                CostPaymentResult::NeedsChoice(_) => Err(CostPaymentError::Other(
                    "Return-to-hand cost still needs choice after preselection".to_string(),
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
    filter: &ObjectFilter,
) -> Vec<ObjectId> {
    let ctx = FilterContext {
        you: Some(payer),
        source: Some(source),
        ..Default::default()
    };
    game.battlefield
        .iter()
        .copied()
        .filter(|&id| {
            game.object(id)
                .is_some_and(|obj| filter.matches(obj, &ctx, game) && game.can_be_sacrificed(id))
        })
        .collect()
}

fn legal_discard_cards(
    game: &GameState,
    payer: PlayerId,
    source: ObjectId,
    card_types: &[crate::types::CardType],
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
                    if !card_types.is_empty() {
                        return game
                            .object(card_id)
                            .is_some_and(|obj| card_types.iter().any(|ct| obj.has_card_type(*ct)));
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

fn legal_exile_from_graveyard_cards(
    game: &GameState,
    payer: PlayerId,
    card_type: Option<crate::types::CardType>,
) -> Vec<ObjectId> {
    game.player(payer)
        .map(|p| {
            p.graveyard
                .iter()
                .copied()
                .filter(|&card_id| {
                    if let Some(ct) = card_type {
                        return game.object(card_id).is_some_and(|obj| obj.has_card_type(ct));
                    }
                    true
                })
                .collect()
        })
        .unwrap_or_default()
}

fn legal_reveal_cards(
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
                        return game.object(card_id).is_some_and(|obj| obj.has_card_type(ct));
                    }
                    true
                })
                .collect()
        })
        .unwrap_or_default()
}

fn legal_return_targets(
    game: &GameState,
    payer: PlayerId,
    source: ObjectId,
    filter: &ObjectFilter,
) -> Vec<ObjectId> {
    let ctx = FilterContext {
        you: Some(payer),
        source: Some(source),
        ..Default::default()
    };
    game.battlefield
        .iter()
        .copied()
        .filter(|&id| {
            game.object(id)
                .is_some_and(|obj| filter.matches(obj, &ctx, game))
        })
        .collect()
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

fn describe_permanent_filter(filter: &ObjectFilter) -> String {
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
