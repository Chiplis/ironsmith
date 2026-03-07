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

    if let Some(mut next) = next_phase(current_phase) {
        if matches!(next, Phase::Combat)
            && game
                .skip_next_combat_phases
                .remove(&game.turn.active_player)
        {
            next = Phase::NextMain;
        }
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
    let mut dm = crate::decision::SelectFirstDecisionMaker;
    execute_untap_step_with(game, &mut dm);
}

/// Executes the untap step for the active player with an explicit decision maker.
///
/// This variant prompts for optional "you may choose not to untap ..." abilities.
pub fn execute_untap_step_with(game: &mut GameState, decision_maker: &mut impl DecisionMaker) {
    use crate::ability::AbilityKind;
    use crate::decisions::context::BooleanContext;
    use crate::static_abilities::StaticAbilityId;

    let active_player = game.turn.active_player;

    let phased_in: Vec<_> = game
        .phased_out
        .iter()
        .copied()
        .filter(|&id| {
            game.object(id).is_some_and(|obj| {
                obj.zone == crate::zone::Zone::Battlefield && obj.controller == active_player
            })
        })
        .collect();
    for id in phased_in {
        game.phase_in(id);
    }

    // Get all permanents controlled by active player
    let permanents: Vec<_> = game.permanents_controlled_by(active_player);

    // First pass: collect which permanents should untap
    let should_untap: std::collections::HashSet<_> = permanents
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
            let has_optional_choice = obj.abilities.iter().any(|ability| {
                matches!(
                    &ability.kind,
                    AbilityKind::Static(static_ability)
                        if static_ability.id()
                            == StaticAbilityId::MayChooseNotToUntapDuringUntapStep
                )
            });
            let blocked_by_restriction = !game.can_untap(id);
            if has_doesnt_untap || blocked_by_restriction {
                None
            } else if has_optional_choice && game.is_tapped(id) {
                let choice_ctx = BooleanContext::new(
                    active_player,
                    Some(id),
                    format!("untap {} during your untap step", obj.name),
                );
                decision_maker
                    .decide_boolean(game, &choice_ctx)
                    .then_some(id)
            } else {
                Some(id)
            }
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
    let mut dm = crate::decision::AutoPassDecisionMaker;
    execute_draw_step_with(game, &mut dm)
}

/// Executes the draw step for the active player with an explicit decision maker.
pub fn execute_draw_step_with(
    game: &mut GameState,
    decision_maker: &mut (impl DecisionMaker + ?Sized),
) -> Vec<crate::triggers::TriggerEvent> {
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
        let drawn = game.draw_cards_with_dm(active_player, 1, decision_maker);

        // Track cards drawn this turn
        *game.cards_drawn_this_turn.entry(active_player).or_insert(0) += drawn.len() as u32;

        // Create a single CardsDrawnEvent if any cards were drawn
        if !drawn.is_empty() {
            let draw_event_provenance = game
                .provenance_graph
                .alloc_root_event(crate::events::EventKind::CardsDrawn);
            let event = CardsDrawnEvent::new(active_player, drawn, is_first_draw);
            draw_events.push(TriggerEvent::new_with_provenance(
                event,
                draw_event_provenance,
            ));
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
        let discard_provenance = game
            .provenance_graph
            .alloc_root_event(crate::events::EventKind::Discard);
        let result = execute_discard(
            game,
            card_id,
            active_player,
            cause.clone(),
            false,
            discard_provenance,
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
        if !game.damage_persists.contains(&id) {
            game.clear_damage(id);
        }
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
    game.cleanup_granted_mana_abilities_end_of_turn();

    // End "until end of turn" effects would happen here
    // (Handled by continuous effect manager)
    game.continuous_effects.cleanup_end_of_turn();
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
    use crate::ability::Ability;
    use crate::card::CardBuilder;
    use crate::ids::{CardId, ObjectId};
    use crate::object::Object;
    use crate::static_abilities::StaticAbility;
    use crate::types::CardType;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn create_artifact(
        game: &mut GameState,
        name: &str,
        controller: PlayerId,
        abilities: Vec<StaticAbility>,
    ) -> ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .card_types(vec![CardType::Artifact])
            .build();
        let mut obj = Object::from_card(id, &card, controller, Zone::Battlefield);
        for ability in abilities {
            obj.abilities.push(Ability::static_ability(ability));
        }
        game.add_object(obj);
        id
    }

    #[derive(Default)]
    struct AlwaysYesDecisionMaker;

    impl DecisionMaker for AlwaysYesDecisionMaker {
        fn decide_boolean(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::BooleanContext,
        ) -> bool {
            true
        }
    }

    #[derive(Default)]
    struct AlwaysNoDecisionMaker;

    impl DecisionMaker for AlwaysNoDecisionMaker {
        fn decide_boolean(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::BooleanContext,
        ) -> bool {
            false
        }
    }

    #[test]
    fn execute_untap_step_with_optional_choice_can_untap_when_chosen() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let artifact = create_artifact(
            &mut game,
            "Courier Relic",
            alice,
            vec![StaticAbility::may_choose_not_to_untap_during_untap_step(
                "this artifact",
            )],
        );
        game.tap(artifact);

        let mut dm = AlwaysYesDecisionMaker;
        execute_untap_step_with(&mut game, &mut dm);

        assert!(
            !game.is_tapped(artifact),
            "artifact should untap when controller chooses to untap"
        );
    }

    #[test]
    fn execute_untap_step_with_optional_choice_can_stay_tapped_when_declined() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let artifact = create_artifact(
            &mut game,
            "Courier Relic",
            alice,
            vec![StaticAbility::may_choose_not_to_untap_during_untap_step(
                "this artifact",
            )],
        );
        game.tap(artifact);

        let mut dm = AlwaysNoDecisionMaker;
        execute_untap_step_with(&mut game, &mut dm);

        assert!(
            game.is_tapped(artifact),
            "artifact should stay tapped when controller declines untap"
        );
    }

    #[test]
    fn execute_untap_step_with_optional_choice_respects_doesnt_untap_and_restrictions() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let doesnt_untap_artifact = create_artifact(
            &mut game,
            "Locked Relic",
            alice,
            vec![
                StaticAbility::may_choose_not_to_untap_during_untap_step("this artifact"),
                StaticAbility::doesnt_untap(),
            ],
        );
        let cant_untap_artifact = create_artifact(
            &mut game,
            "Frozen Relic",
            alice,
            vec![StaticAbility::may_choose_not_to_untap_during_untap_step(
                "this artifact",
            )],
        );
        game.tap(doesnt_untap_artifact);
        game.tap(cant_untap_artifact);
        game.cant_effects.add_cant_untap(cant_untap_artifact);

        let mut dm = AlwaysYesDecisionMaker;
        execute_untap_step_with(&mut game, &mut dm);

        assert!(
            game.is_tapped(doesnt_untap_artifact),
            "doesn't-untap static ability should prevent untapping"
        );
        assert!(
            game.is_tapped(cant_untap_artifact),
            "can't-untap restriction should prevent untapping"
        );
    }

    #[test]
    fn execute_draw_step_with_can_move_drawn_commander_to_command_zone() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let commander = CardBuilder::new(CardId::from_raw(9000), "Topdeck Commander")
            .card_types(vec![CardType::Creature])
            .build();
        let commander_id = game.create_object_from_card(&commander, alice, Zone::Library);
        game.set_as_commander(commander_id, alice);

        let mut dm = AlwaysYesDecisionMaker;
        let events = execute_draw_step_with(&mut game, &mut dm);

        assert!(
            events.is_empty(),
            "redirected commander should not count as a draw"
        );
        assert!(
            game.player(alice)
                .expect("alice should exist")
                .hand
                .is_empty()
        );
        assert_eq!(game.objects_in_zone(Zone::Command).len(), 1);
        assert_eq!(
            game.cards_drawn_this_turn.get(&alice).copied().unwrap_or(0),
            0
        );
    }

    #[test]
    fn execute_draw_step_with_can_leave_commander_in_hand() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let commander = CardBuilder::new(CardId::from_raw(9001), "Honest Commander")
            .card_types(vec![CardType::Creature])
            .build();
        let commander_id = game.create_object_from_card(&commander, alice, Zone::Library);
        game.set_as_commander(commander_id, alice);

        let mut dm = AlwaysNoDecisionMaker;
        let events = execute_draw_step_with(&mut game, &mut dm);

        assert_eq!(
            events.len(),
            1,
            "keeping the commander should produce a draw event"
        );
        assert_eq!(
            game.player(alice).expect("alice should exist").hand.len(),
            1
        );
        assert!(game.objects_in_zone(Zone::Command).is_empty());
        assert_eq!(
            game.cards_drawn_this_turn.get(&alice).copied().unwrap_or(0),
            1
        );
    }
}
