//! Turn structure and priority system for MTG.
//!
//! This module handles:
//! - Turn and phase progression (untap, upkeep, draw, main, combat, etc.)
//! - Priority passing and resolution
//! - Step-specific actions (untapping, drawing, cleanup)

use crate::DecisionMaker;
use crate::game_state::{GameState, Phase, Step};
use crate::ids::PlayerId;

/// Errors that can occur during turn progression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TurnError {
    /// Cannot advance past the current step/phase.
    CannotAdvance,
    /// No players left in the game.
    NoPlayersRemaining,
    /// Invalid state for the requested operation.
    InvalidState { message: String },
}

/// Result of passing priority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PriorityResult {
    /// More players need to pass priority.
    Continue,
    /// All players passed in succession; resolve the top of the stack.
    StackResolves,
    /// All players passed with an empty stack; the phase/step ends.
    PhaseEnds,
}

/// Tracks consecutive priority passes for determining when all players have passed.
#[derive(Debug, Clone, Default)]
pub struct PriorityTracker {
    /// Number of consecutive passes without any player taking an action.
    pub consecutive_passes: usize,
    /// Number of players in the game (for determining when all have passed).
    pub players_in_game: usize,
}

impl PriorityTracker {
    /// Creates a new priority tracker for the given number of players.
    pub fn new(players_in_game: usize) -> Self {
        Self {
            consecutive_passes: 0,
            players_in_game,
        }
    }

    /// Records a priority pass. Returns true if all players have now passed.
    pub fn record_pass(&mut self) -> bool {
        self.consecutive_passes += 1;
        self.consecutive_passes >= self.players_in_game
    }

    /// Resets the pass counter (called when a player takes an action).
    pub fn reset(&mut self) {
        self.consecutive_passes = 0;
    }

    /// Updates the number of players (called when a player leaves the game).
    pub fn set_players_in_game(&mut self, count: usize) {
        self.players_in_game = count;
    }

    /// Returns true if all players have passed in succession.
    pub fn all_passed(&self) -> bool {
        self.consecutive_passes >= self.players_in_game
    }
}

/// Returns the next step within a phase, or None if the phase is over.
pub fn next_step(phase: Phase, current_step: Option<Step>) -> Option<Step> {
    match (phase, current_step) {
        // Beginning phase
        (Phase::Beginning, Some(Step::Untap)) => Some(Step::Upkeep),
        (Phase::Beginning, Some(Step::Upkeep)) => Some(Step::Draw),
        (Phase::Beginning, Some(Step::Draw)) => None,
        (Phase::Beginning, None) => Some(Step::Untap),

        // Main phases have no steps
        (Phase::FirstMain, _) => None,
        (Phase::NextMain, _) => None,

        // Combat phase
        (Phase::Combat, Some(Step::BeginCombat)) => Some(Step::DeclareAttackers),
        (Phase::Combat, Some(Step::DeclareAttackers)) => Some(Step::DeclareBlockers),
        (Phase::Combat, Some(Step::DeclareBlockers)) => Some(Step::CombatDamage),
        (Phase::Combat, Some(Step::CombatDamage)) => Some(Step::EndCombat),
        (Phase::Combat, Some(Step::EndCombat)) => None,
        (Phase::Combat, None) => Some(Step::BeginCombat),

        // Ending phase
        (Phase::Ending, Some(Step::End)) => Some(Step::Cleanup),
        (Phase::Ending, Some(Step::Cleanup)) => None,
        (Phase::Ending, None) => Some(Step::End),

        // Invalid combinations
        _ => None,
    }
}

/// Returns the next phase after the given phase.
pub fn next_phase(phase: Phase) -> Option<Phase> {
    match phase {
        Phase::Beginning => Some(Phase::FirstMain),
        Phase::FirstMain => Some(Phase::Combat),
        Phase::Combat => Some(Phase::NextMain),
        Phase::NextMain => Some(Phase::Ending),
        Phase::Ending => None, // Turn ends
    }
}

/// Returns the first step of a phase, if any.
pub fn first_step_of_phase(phase: Phase) -> Option<Step> {
    match phase {
        Phase::Beginning => Some(Step::Untap),
        Phase::FirstMain => None,
        Phase::Combat => Some(Step::BeginCombat),
        Phase::NextMain => None,
        Phase::Ending => Some(Step::End),
    }
}

/// Advances the game to the next step within the current phase.
/// If at the end of a phase, advances to the next phase.
/// If at the end of the turn, advances to the next turn.
pub fn advance_step(game: &mut GameState) -> Result<(), TurnError> {
    if game.players_in_game() == 0 {
        return Err(TurnError::NoPlayersRemaining);
    }

    let current_phase = game.turn.phase;
    let current_step = game.turn.step;

    // Try to advance to next step in current phase
    if let Some(next) = next_step(current_phase, current_step) {
        game.turn.step = Some(next);
        game.turn.priority_player = Some(game.turn.active_player);
        return Ok(());
    }

    // No more steps in this phase - advance to next phase
    advance_phase(game)
}

/// Advances the game to the next phase.
/// If at the end of the turn, advances to the next turn.
pub fn advance_phase(game: &mut GameState) -> Result<(), TurnError> {
    if game.players_in_game() == 0 {
        return Err(TurnError::NoPlayersRemaining);
    }

    let current_phase = game.turn.phase;

    if let Some(next) = next_phase(current_phase) {
        game.turn.phase = next;
        game.turn.step = first_step_of_phase(next);
        game.turn.priority_player = Some(game.turn.active_player);
        Ok(())
    } else {
        // End of turn - advance to next player
        game.next_turn();
        Ok(())
    }
}

/// Returns true if the given player currently has priority.
pub fn has_priority(game: &GameState, player: PlayerId) -> bool {
    game.turn.priority_player == Some(player)
}

/// Returns the current priority holder, if any.
pub fn priority_holder(game: &GameState) -> Option<PlayerId> {
    game.turn.priority_player
}

/// Passes priority for the current player.
/// Returns the result indicating what should happen next.
pub fn pass_priority(game: &mut GameState, tracker: &mut PriorityTracker) -> PriorityResult {
    if tracker.record_pass() {
        // All players have passed
        if game.stack_is_empty() {
            PriorityResult::PhaseEnds
        } else {
            PriorityResult::StackResolves
        }
    } else {
        // Move priority to next player
        advance_priority_to_next_player(game);
        PriorityResult::Continue
    }
}

/// Resets priority to the active player (called after a spell/ability is put on stack).
pub fn reset_priority(game: &mut GameState, tracker: &mut PriorityTracker) {
    tracker.reset();
    game.turn.priority_player = Some(game.turn.active_player);
}

/// Advances priority to the next player in turn order.
fn advance_priority_to_next_player(game: &mut GameState) {
    let current = match game.turn.priority_player {
        Some(p) => p,
        None => return,
    };

    let current_index = game
        .turn_order
        .iter()
        .position(|&p| p == current)
        .unwrap_or(0);

    // Find next player who is still in the game
    for i in 1..=game.turn_order.len() {
        let next_index = (current_index + i) % game.turn_order.len();
        let next_player = game.turn_order[next_index];

        if game.player(next_player).is_some_and(|p| p.is_in_game()) {
            game.turn.priority_player = Some(next_player);
            return;
        }
    }
}

/// Returns true if it's currently "sorcery timing" - main phase with empty stack.
pub fn is_sorcery_timing(game: &GameState) -> bool {
    matches!(game.turn.phase, Phase::FirstMain | Phase::NextMain) && game.stack_is_empty()
}

/// Returns true if the current step doesn't grant priority (untap, cleanup normally).
pub fn is_no_priority_step(game: &GameState) -> bool {
    matches!(game.turn.step, Some(Step::Untap) | Some(Step::Cleanup))
}

/// Executes the untap step for the active player.
/// Untaps all permanents controlled by the active player (except those that don't untap).
pub fn execute_untap_step(game: &mut GameState) {
    use crate::ability::AbilityKind;

    let active_player = game.turn.active_player;

    // Get all permanents controlled by active player
    let permanents: Vec<_> = game.permanents_controlled_by(active_player);

    // First pass: collect which permanents should untap
    let should_untap: Vec<_> = permanents
        .iter()
        .filter_map(|&id| {
            let obj = game.object(id)?;
            // Check if the permanent has "doesn't untap during your untap step"
            let has_doesnt_untap = obj.abilities.iter().any(|ability| {
                if let AbilityKind::Static(s) = &ability.kind {
                    s.affects_untap()
                } else {
                    false
                }
            });
            if has_doesnt_untap { None } else { Some(id) }
        })
        .collect();

    // Second pass: untap eligible permanents and remove summoning sickness from all
    for id in permanents {
        // Only untap if the permanent doesn't have DoesntUntap
        if should_untap.contains(&id) {
            game.untap(id);
        }
        // Always remove summoning sickness at untap step
        game.remove_summoning_sickness(id);
    }

    // No priority during untap step
    game.turn.priority_player = None;
}

/// Executes the draw step for the active player.
/// Active player draws a card.
///
/// Returns a list of TriggerEvents for cards that were drawn, which can be used
/// to check for card-draw triggers (including Miracle).
pub fn execute_draw_step(game: &mut GameState) -> Vec<crate::triggers::TriggerEvent> {
    use crate::events::other::CardsDrawnEvent;
    use crate::triggers::TriggerEvent;

    let active_player = game.turn.active_player;
    if game.skip_next_draw_step.remove(&active_player) {
        game.turn.priority_player = Some(active_player);
        return Vec::new();
    }

    // Check if player can draw (the draw step draw is the first draw of the turn)
    let current_draws = game
        .cards_drawn_this_turn
        .get(&active_player)
        .copied()
        .unwrap_or(0);

    // Track if this is the first draw of the turn (before drawing)
    let is_first_draw = current_draws == 0;

    // Check for "can't draw extra cards" restriction (e.g., Narset)
    // The draw step draw is only blocked if they've already drawn this turn
    let can_draw = if !game.can_draw_extra_cards(active_player) {
        // Only allow if they haven't drawn yet this turn
        current_draws == 0
    } else {
        true
    };

    let mut draw_events = Vec::new();

    if can_draw {
        // Draw using GameState method to properly update object zones
        let drawn = game.draw_cards(active_player, 1);

        // Track cards drawn this turn
        *game.cards_drawn_this_turn.entry(active_player).or_insert(0) += drawn.len() as u32;

        // Create a single CardsDrawnEvent if any cards were drawn
        if !drawn.is_empty() {
            let event = CardsDrawnEvent::new(active_player, drawn, is_first_draw);
            draw_events.push(TriggerEvent::new(event));
        }
    }

    // Priority is granted during draw step (after draw)
    game.turn.priority_player = Some(active_player);

    draw_events
}

/// Checks if the active player needs to discard during cleanup.
/// Returns a spec and player ID if the player must choose which cards to discard.
pub fn get_cleanup_discard_spec(
    game: &GameState,
) -> Option<(PlayerId, crate::decisions::specs::DiscardToHandSizeSpec)> {
    use crate::decisions::specs::DiscardToHandSizeSpec;

    let active_player = game.turn.active_player;

    if let Some(player) = game.player(active_player) {
        let max_hand = player.max_hand_size.max(0) as usize;
        let excess = player.hand.len().saturating_sub(max_hand);

        if excess > 0 {
            return Some((
                active_player,
                DiscardToHandSizeSpec::new(excess, player.hand.clone()),
            ));
        }
    }

    None
}

/// Applies the discard chosen by the player during cleanup.
pub fn apply_cleanup_discard(
    game: &mut GameState,
    cards_to_discard: &[crate::ids::ObjectId],
    decision_maker: &mut impl DecisionMaker,
) -> Vec<crate::ids::ObjectId> {
    use crate::event_processor::execute_discard;
    use crate::events::cause::EventCause;
    use crate::zone::Zone;

    let mut madness_cards = Vec::new();
    let active_player = game.turn.active_player;

    // All discards go through execute_discard which handles:
    // - Madness (replacement effect that exiles instead)
    // - Library of Leng (player choice to put on top of library)
    // - Normal discard to graveyard
    // Cleanup discard is a GAME RULE discard, so Library of Leng can't apply
    let cause = EventCause::from_game_rule();

    for &card_id in cards_to_discard {
        let result = execute_discard(
            game,
            card_id,
            active_player,
            cause.clone(),
            false,
            decision_maker,
        );

        // Track cards that were exiled via Madness (can be cast from exile)
        if result.final_zone == Zone::Exile
            && let Some(new_id) = result.new_id
        {
            madness_cards.push(new_id);
        }
    }

    madness_cards
}

/// Executes the cleanup step (damage removal, mana emptying).
/// This should be called after any required discard decision has been resolved.
pub fn execute_cleanup_step(game: &mut GameState) {
    let active_player = game.turn.active_player;

    // Empty mana pool
    if let Some(player) = game.player_mut(active_player) {
        player.mana_pool.empty();
    }

    // Remove all damage marked on creatures and clear regeneration shields
    for &id in &game.battlefield.clone() {
        game.clear_damage(id);
        game.clear_regeneration_shields(id);
    }

    // Clear one-shot replacement effects (like regeneration shields)
    // These only last "until end of turn" per MTG rules
    game.replacement_effects.clear_one_shot_effects();

    // Clean up expired grants (e.g., flashback from Snapcaster Mage)
    let turn_number = game.turn.turn_number;
    let battlefield = game.battlefield.clone();
    game.grant_registry
        .cleanup_expired(turn_number, &battlefield);

    game.cleanup_restrictions_end_of_turn();

    // End "until end of turn" effects would happen here
    // (Handled by continuous effect manager)
    game.cleanup_player_control_end_of_turn();

    // Normally no priority during cleanup, but if triggers/SBAs happen, there's a new cleanup
    game.turn.priority_player = None;
}

/// Returns a human-readable description of the current phase/step.
pub fn current_phase_description(game: &GameState) -> String {
    let phase_name = match game.turn.phase {
        Phase::Beginning => "Beginning",
        Phase::FirstMain => "Precombat Main",
        Phase::Combat => "Combat",
        Phase::NextMain => "Postcombat Main",
        Phase::Ending => "Ending",
    };

    if let Some(step) = game.turn.step {
        let step_name = match step {
            Step::Untap => "Untap",
            Step::Upkeep => "Upkeep",
            Step::Draw => "Draw",
            Step::BeginCombat => "Beginning of Combat",
            Step::DeclareAttackers => "Declare Attackers",
            Step::DeclareBlockers => "Declare Blockers",
            Step::CombatDamage => "Combat Damage",
            Step::EndCombat => "End of Combat",
            Step::End => "End Step",
            Step::Cleanup => "Cleanup",
        };
        format!("{} Phase - {} Step", phase_name, step_name)
    } else {
        format!("{} Phase", phase_name)
    }
}

/// Checks if the game is in a main phase (pre or post combat).
pub fn is_main_phase(game: &GameState) -> bool {
    matches!(game.turn.phase, Phase::FirstMain | Phase::NextMain)
}

/// Checks if the game is in the combat phase.
pub fn is_combat_phase(game: &GameState) -> bool {
    game.turn.phase == Phase::Combat
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::zone::Zone;

    fn test_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    #[test]
    fn test_next_step_beginning_phase() {
        assert_eq!(
            next_step(Phase::Beginning, Some(Step::Untap)),
            Some(Step::Upkeep)
        );
        assert_eq!(
            next_step(Phase::Beginning, Some(Step::Upkeep)),
            Some(Step::Draw)
        );
        assert_eq!(next_step(Phase::Beginning, Some(Step::Draw)), None);
    }

    #[test]
    fn test_next_step_combat_phase() {
        assert_eq!(
            next_step(Phase::Combat, Some(Step::BeginCombat)),
            Some(Step::DeclareAttackers)
        );
        assert_eq!(
            next_step(Phase::Combat, Some(Step::DeclareAttackers)),
            Some(Step::DeclareBlockers)
        );
        assert_eq!(
            next_step(Phase::Combat, Some(Step::DeclareBlockers)),
            Some(Step::CombatDamage)
        );
        assert_eq!(
            next_step(Phase::Combat, Some(Step::CombatDamage)),
            Some(Step::EndCombat)
        );
        assert_eq!(next_step(Phase::Combat, Some(Step::EndCombat)), None);
    }

    #[test]
    fn test_next_step_ending_phase() {
        assert_eq!(
            next_step(Phase::Ending, Some(Step::End)),
            Some(Step::Cleanup)
        );
        assert_eq!(next_step(Phase::Ending, Some(Step::Cleanup)), None);
    }

    #[test]
    fn test_main_phases_have_no_steps() {
        assert_eq!(next_step(Phase::FirstMain, None), None);
        assert_eq!(next_step(Phase::NextMain, None), None);
    }

    #[test]
    fn test_next_phase() {
        assert_eq!(next_phase(Phase::Beginning), Some(Phase::FirstMain));
        assert_eq!(next_phase(Phase::FirstMain), Some(Phase::Combat));
        assert_eq!(next_phase(Phase::Combat), Some(Phase::NextMain));
        assert_eq!(next_phase(Phase::NextMain), Some(Phase::Ending));
        assert_eq!(next_phase(Phase::Ending), None);
    }

    #[test]
    fn test_first_step_of_phase() {
        assert_eq!(first_step_of_phase(Phase::Beginning), Some(Step::Untap));
        assert_eq!(first_step_of_phase(Phase::FirstMain), None);
        assert_eq!(first_step_of_phase(Phase::Combat), Some(Step::BeginCombat));
        assert_eq!(first_step_of_phase(Phase::NextMain), None);
        assert_eq!(first_step_of_phase(Phase::Ending), Some(Step::End));
    }

    #[test]
    fn test_advance_step_through_beginning_phase() {
        let mut game = test_game();
        assert_eq!(game.turn.step, Some(Step::Untap));

        advance_step(&mut game).unwrap();
        assert_eq!(game.turn.step, Some(Step::Upkeep));

        advance_step(&mut game).unwrap();
        assert_eq!(game.turn.step, Some(Step::Draw));

        // Advancing from draw should go to next phase
        advance_step(&mut game).unwrap();
        assert_eq!(game.turn.phase, Phase::FirstMain);
        assert_eq!(game.turn.step, None);
    }

    #[test]
    fn test_advance_phase_through_turn() {
        let mut game = test_game();

        // Skip to main phase
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;

        advance_phase(&mut game).unwrap();
        assert_eq!(game.turn.phase, Phase::Combat);
        assert_eq!(game.turn.step, Some(Step::BeginCombat));

        advance_phase(&mut game).unwrap();
        assert_eq!(game.turn.phase, Phase::NextMain);

        advance_phase(&mut game).unwrap();
        assert_eq!(game.turn.phase, Phase::Ending);
        assert_eq!(game.turn.step, Some(Step::End));

        // Advancing from ending should go to next turn
        let turn_number = game.turn.turn_number;
        advance_phase(&mut game).unwrap();
        assert_eq!(game.turn.turn_number, turn_number + 1);
        assert_eq!(game.turn.phase, Phase::Beginning);
    }

    #[test]
    fn test_priority_tracker() {
        let mut tracker = PriorityTracker::new(2);

        assert!(!tracker.all_passed());
        assert!(!tracker.record_pass()); // First pass
        assert!(tracker.record_pass()); // Second pass - all passed

        tracker.reset();
        assert!(!tracker.all_passed());
    }

    #[test]
    fn test_pass_priority_empty_stack() {
        let mut game = test_game();
        let mut tracker = PriorityTracker::new(2);

        // First player passes
        let result = pass_priority(&mut game, &mut tracker);
        assert_eq!(result, PriorityResult::Continue);

        // Second player passes with empty stack
        let result = pass_priority(&mut game, &mut tracker);
        assert_eq!(result, PriorityResult::PhaseEnds);
    }

    #[test]
    fn test_pass_priority_with_stack() {
        let mut game = test_game();
        let mut tracker = PriorityTracker::new(2);

        // Add something to the stack
        use crate::game_state::StackEntry;
        use crate::ids::ObjectId;
        let entry = StackEntry::new(ObjectId::from_raw(1), PlayerId::from_index(0));
        game.push_to_stack(entry);

        // First player passes
        let result = pass_priority(&mut game, &mut tracker);
        assert_eq!(result, PriorityResult::Continue);

        // Second player passes with non-empty stack
        let result = pass_priority(&mut game, &mut tracker);
        assert_eq!(result, PriorityResult::StackResolves);
    }

    #[test]
    fn test_has_priority() {
        let game = test_game();

        assert!(has_priority(&game, PlayerId::from_index(0)));
        assert!(!has_priority(&game, PlayerId::from_index(1)));
    }

    #[test]
    fn test_reset_priority() {
        let mut game = test_game();
        let mut tracker = PriorityTracker::new(2);

        // Pass priority once
        pass_priority(&mut game, &mut tracker);
        assert_eq!(tracker.consecutive_passes, 1);

        // Reset should clear passes and give priority to active player
        reset_priority(&mut game, &mut tracker);
        assert_eq!(tracker.consecutive_passes, 0);
        assert_eq!(game.turn.priority_player, Some(game.turn.active_player));
    }

    #[test]
    fn test_is_sorcery_timing() {
        let mut game = test_game();

        // Beginning phase - not sorcery timing
        assert!(!is_sorcery_timing(&game));

        // Precombat main with empty stack - sorcery timing
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        assert!(is_sorcery_timing(&game));

        // Add something to stack - no longer sorcery timing
        use crate::game_state::StackEntry;
        use crate::ids::ObjectId;
        game.push_to_stack(StackEntry::new(
            ObjectId::from_raw(1),
            PlayerId::from_index(0),
        ));
        assert!(!is_sorcery_timing(&game));
    }

    #[test]
    fn test_is_no_priority_step() {
        let mut game = test_game();

        // Untap step - no priority
        game.turn.step = Some(Step::Untap);
        assert!(is_no_priority_step(&game));

        // Upkeep step - has priority
        game.turn.step = Some(Step::Upkeep);
        assert!(!is_no_priority_step(&game));

        // Cleanup step - no priority
        game.turn.step = Some(Step::Cleanup);
        assert!(is_no_priority_step(&game));
    }

    #[test]
    fn test_execute_untap_step() {
        let mut game = test_game();
        let active_player = game.turn.active_player;

        // Create a tapped creature
        use crate::card::{CardBuilder, PowerToughness};
        use crate::ids::CardId;
        use crate::mana::{ManaCost, ManaSymbol};
        use crate::types::{CardType, Subtype};

        let card = CardBuilder::new(CardId::from_raw(1), "Test Creature")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Green]]))
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Bear])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();

        let id = game.create_object_from_card(&card, active_player, Zone::Battlefield);
        game.tap(id);
        game.set_summoning_sick(id);

        execute_untap_step(&mut game);

        assert!(!game.is_tapped(id));
        assert!(!game.is_summoning_sick(id));
        assert!(game.turn.priority_player.is_none());
    }

    #[test]
    fn test_execute_draw_step() {
        let mut game = test_game();
        let active_player = game.turn.active_player;

        // Create a proper card in the library
        use crate::card::CardBuilder;
        use crate::ids::CardId;
        use crate::types::CardType;

        let card = CardBuilder::new(CardId::from_raw(1), "Test Card")
            .card_types(vec![CardType::Sorcery])
            .build();
        game.create_object_from_card(&card, active_player, Zone::Library);

        let hand_size_before = game.player(active_player).unwrap().hand.len();

        execute_draw_step(&mut game);

        let hand_size_after = game.player(active_player).unwrap().hand.len();
        assert_eq!(hand_size_after, hand_size_before + 1);
        assert_eq!(game.turn.priority_player, Some(active_player));
    }

    #[test]
    fn test_execute_cleanup_step() {
        let mut game = test_game();
        let active_player = game.turn.active_player;

        // Add mana to pool
        use crate::mana::ManaSymbol;
        game.player_mut(active_player)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::White, 5);

        // Add damage to a creature
        use crate::card::{CardBuilder, PowerToughness};
        use crate::ids::CardId;
        use crate::mana::ManaCost;
        use crate::types::{CardType, Subtype};

        let card = CardBuilder::new(CardId::from_raw(1), "Test Creature")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Green]]))
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Bear])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();

        let id = game.create_object_from_card(&card, active_player, Zone::Battlefield);
        game.mark_damage(id, 1);

        execute_cleanup_step(&mut game);

        // Mana pool should be empty
        assert_eq!(game.player(active_player).unwrap().mana_pool.total(), 0);

        // Damage should be removed
        assert_eq!(game.damage_on(id), 0);

        // No priority during cleanup
        assert!(game.turn.priority_player.is_none());
    }

    #[test]
    fn test_current_phase_description() {
        let mut game = test_game();

        game.turn.phase = Phase::Beginning;
        game.turn.step = Some(Step::Upkeep);
        assert_eq!(
            current_phase_description(&game),
            "Beginning Phase - Upkeep Step"
        );

        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        assert_eq!(current_phase_description(&game), "Precombat Main Phase");

        game.turn.phase = Phase::Combat;
        game.turn.step = Some(Step::DeclareAttackers);
        assert_eq!(
            current_phase_description(&game),
            "Combat Phase - Declare Attackers Step"
        );
    }

    #[test]
    fn test_is_main_phase() {
        let mut game = test_game();

        game.turn.phase = Phase::Beginning;
        assert!(!is_main_phase(&game));

        game.turn.phase = Phase::FirstMain;
        assert!(is_main_phase(&game));

        game.turn.phase = Phase::Combat;
        assert!(!is_main_phase(&game));

        game.turn.phase = Phase::NextMain;
        assert!(is_main_phase(&game));

        game.turn.phase = Phase::Ending;
        assert!(!is_main_phase(&game));
    }

    #[test]
    fn test_is_combat_phase() {
        let mut game = test_game();

        game.turn.phase = Phase::FirstMain;
        assert!(!is_combat_phase(&game));

        game.turn.phase = Phase::Combat;
        assert!(is_combat_phase(&game));
    }

    #[test]
    fn test_cleanup_step_discard_to_graveyard() {
        let mut game = test_game();
        let active_player = game.turn.active_player;

        // Create cards in hand (more than max hand size of 7)
        use crate::card::CardBuilder;
        use crate::ids::CardId;
        use crate::types::CardType;

        // Add 9 cards to hand (2 over the limit)
        for i in 0..9u32 {
            let card = CardBuilder::new(CardId::new(), &format!("Card {}", i))
                .card_types(vec![CardType::Sorcery])
                .build();
            game.create_object_from_card(&card, active_player, Zone::Hand);
        }

        // Verify hand has 9 cards
        assert_eq!(game.player(active_player).unwrap().hand.len(), 9);
        assert_eq!(game.player(active_player).unwrap().graveyard.len(), 0);

        // Check if discard is needed using the new spec-based flow
        let spec = get_cleanup_discard_spec(&game);
        assert!(spec.is_some());

        // Get the cards to discard (simulating player choice - last 2 cards)
        if let Some((player, discard_spec)) = spec {
            assert_eq!(player, active_player);
            let count = discard_spec.count;
            assert_eq!(count, 2);
            let cards_to_discard: Vec<_> = discard_spec
                .hand
                .iter()
                .rev()
                .take(count)
                .copied()
                .collect();
            let mut dm = crate::decision::AutoPassDecisionMaker;
            apply_cleanup_discard(&mut game, &cards_to_discard, &mut dm);
        }

        // Execute cleanup step (damage removal, mana emptying)
        execute_cleanup_step(&mut game);

        // Hand should now have 7 cards (max hand size)
        assert_eq!(game.player(active_player).unwrap().hand.len(), 7);

        // Graveyard should have 2 cards (the discarded ones)
        assert_eq!(game.player(active_player).unwrap().graveyard.len(), 2);

        // Verify the graveyard cards are in the Graveyard zone
        for &card_id in &game.player(active_player).unwrap().graveyard {
            let obj = game.object(card_id).unwrap();
            assert_eq!(obj.zone, Zone::Graveyard);
        }
    }

    #[test]
    fn test_cleanup_step_no_discard_under_max() {
        let mut game = test_game();
        let active_player = game.turn.active_player;

        // Create 5 cards in hand (under the limit)
        use crate::card::CardBuilder;
        use crate::ids::CardId;
        use crate::types::CardType;

        for i in 0..5u32 {
            let card = CardBuilder::new(CardId::new(), &format!("Card {}", i))
                .card_types(vec![CardType::Sorcery])
                .build();
            game.create_object_from_card(&card, active_player, Zone::Hand);
        }

        // Verify hand has 5 cards
        assert_eq!(game.player(active_player).unwrap().hand.len(), 5);

        // Check if discard is needed - should be None
        let spec = get_cleanup_discard_spec(&game);
        assert!(spec.is_none());

        // Execute cleanup step
        execute_cleanup_step(&mut game);

        // Hand should still have 5 cards
        assert_eq!(game.player(active_player).unwrap().hand.len(), 5);

        // Graveyard should be empty
        assert_eq!(game.player(active_player).unwrap().graveyard.len(), 0);
    }
}
