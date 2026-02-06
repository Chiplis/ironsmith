//! "At the beginning of [player]'s main phase" trigger.

use crate::events::EventKind;
use crate::ids::PlayerId;
use crate::target::PlayerFilter;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

/// Which main phase to trigger on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainPhaseType {
    /// Precombat main phase only.
    Precombat,
    /// Postcombat main phase only.
    Postcombat,
    /// Either main phase.
    Either,
}

/// Trigger that fires at the beginning of a player's main phase.
///
/// Can be configured to trigger on precombat, postcombat, or either main phase.
#[derive(Debug, Clone, PartialEq)]
pub struct BeginningOfMainPhaseTrigger {
    /// Which player's main phase triggers this ability.
    pub player: PlayerFilter,
    /// Which main phase(s) this triggers on.
    pub phase_type: MainPhaseType,
}

impl BeginningOfMainPhaseTrigger {
    /// Create a new main phase trigger for the specified player and phase type.
    pub fn new(player: PlayerFilter, phase_type: MainPhaseType) -> Self {
        Self { player, phase_type }
    }

    /// Create a precombat main phase trigger for your main phase.
    pub fn your_precombat_main_phase() -> Self {
        Self::new(PlayerFilter::You, MainPhaseType::Precombat)
    }

    /// Create a postcombat main phase trigger for your main phase.
    pub fn your_postcombat_main_phase() -> Self {
        Self::new(PlayerFilter::You, MainPhaseType::Postcombat)
    }

    /// Create a trigger for each precombat main phase.
    pub fn each_precombat_main_phase() -> Self {
        Self::new(PlayerFilter::Any, MainPhaseType::Precombat)
    }

    /// Create a trigger for each postcombat main phase.
    pub fn each_postcombat_main_phase() -> Self {
        Self::new(PlayerFilter::Any, MainPhaseType::Postcombat)
    }

    /// Create a trigger for each main phase (both precombat and postcombat).
    pub fn each_main_phase() -> Self {
        Self::new(PlayerFilter::Any, MainPhaseType::Either)
    }
}

impl TriggerMatcher for BeginningOfMainPhaseTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        let kind = event.kind();
        let is_precombat = kind == EventKind::BeginningOfPrecombatMainPhase;
        let is_postcombat = kind == EventKind::BeginningOfPostcombatMainPhase;

        if !is_precombat && !is_postcombat {
            return false;
        }

        // Check phase type restriction
        if is_precombat && self.phase_type == MainPhaseType::Postcombat {
            return false;
        }
        if is_postcombat && self.phase_type == MainPhaseType::Precombat {
            return false;
        }

        let Some(player) = event.player() else {
            return false;
        };
        player_filter_matches(&self.player, player, ctx)
    }

    fn display(&self) -> String {
        let phase_str = match self.phase_type {
            MainPhaseType::Precombat => "precombat main phase",
            MainPhaseType::Postcombat => "postcombat main phase",
            MainPhaseType::Either => "main phase",
        };
        match &self.player {
            PlayerFilter::You => format!("At the beginning of your {}", phase_str),
            PlayerFilter::Any => format!("At the beginning of each {}", phase_str),
            PlayerFilter::Opponent => format!("At the beginning of each opponent's {}", phase_str),
            _ => format!("At the beginning of {:?}'s {}", self.player, phase_str),
        }
    }

    fn clone_box(&self) -> Box<dyn TriggerMatcher> {
        Box::new(self.clone())
    }
}

fn player_filter_matches(filter: &PlayerFilter, player: PlayerId, ctx: &TriggerContext) -> bool {
    match filter {
        PlayerFilter::You => player == ctx.controller,
        PlayerFilter::Opponent => player != ctx.controller,
        PlayerFilter::Any => true,
        PlayerFilter::Specific(id) => player == *id,
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::phase::BeginningOfUpkeepEvent;
    use crate::events::phase::{
        BeginningOfPostcombatMainPhaseEvent, BeginningOfPrecombatMainPhaseEvent,
    };
    use crate::game_state::GameState;
    use crate::ids::ObjectId;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    #[test]
    fn test_display_precombat() {
        let trigger = BeginningOfMainPhaseTrigger::your_precombat_main_phase();
        assert!(trigger.display().contains("precombat main phase"));
    }

    #[test]
    fn test_display_postcombat() {
        let trigger = BeginningOfMainPhaseTrigger::your_postcombat_main_phase();
        assert!(trigger.display().contains("postcombat main phase"));
    }

    #[test]
    fn test_matches_own_precombat_main_phase() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let source_id = ObjectId::from_raw(1);

        let trigger = BeginningOfMainPhaseTrigger::your_precombat_main_phase();
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let event = TriggerEvent::new(BeginningOfPrecombatMainPhaseEvent::new(alice));
        assert!(trigger.matches(&event, &ctx));
    }

    #[test]
    fn test_matches_own_postcombat_main_phase() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let source_id = ObjectId::from_raw(1);

        let trigger = BeginningOfMainPhaseTrigger::your_postcombat_main_phase();
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let event = TriggerEvent::new(BeginningOfPostcombatMainPhaseEvent::new(alice));
        assert!(trigger.matches(&event, &ctx));
    }

    #[test]
    fn test_precombat_does_not_match_postcombat() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let source_id = ObjectId::from_raw(1);

        let trigger = BeginningOfMainPhaseTrigger::your_precombat_main_phase();
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let event = TriggerEvent::new(BeginningOfPostcombatMainPhaseEvent::new(alice));
        assert!(!trigger.matches(&event, &ctx));
    }

    #[test]
    fn test_postcombat_does_not_match_precombat() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let source_id = ObjectId::from_raw(1);

        let trigger = BeginningOfMainPhaseTrigger::your_postcombat_main_phase();
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let event = TriggerEvent::new(BeginningOfPrecombatMainPhaseEvent::new(alice));
        assert!(!trigger.matches(&event, &ctx));
    }

    #[test]
    fn test_either_matches_both() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let source_id = ObjectId::from_raw(1);

        let trigger = BeginningOfMainPhaseTrigger::new(PlayerFilter::You, MainPhaseType::Either);
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let precombat = TriggerEvent::new(BeginningOfPrecombatMainPhaseEvent::new(alice));
        let postcombat = TriggerEvent::new(BeginningOfPostcombatMainPhaseEvent::new(alice));
        assert!(trigger.matches(&precombat, &ctx));
        assert!(trigger.matches(&postcombat, &ctx));
    }

    #[test]
    fn test_does_not_match_opponent_main_phase() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source_id = ObjectId::from_raw(1);

        let trigger = BeginningOfMainPhaseTrigger::your_precombat_main_phase();
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let event = TriggerEvent::new(BeginningOfPrecombatMainPhaseEvent::new(bob));
        assert!(!trigger.matches(&event, &ctx));
    }

    #[test]
    fn test_each_main_phase_matches_any_player() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source_id = ObjectId::from_raw(1);

        let trigger = BeginningOfMainPhaseTrigger::each_precombat_main_phase();
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let alice_event = TriggerEvent::new(BeginningOfPrecombatMainPhaseEvent::new(alice));
        let bob_event = TriggerEvent::new(BeginningOfPrecombatMainPhaseEvent::new(bob));
        assert!(trigger.matches(&alice_event, &ctx));
        assert!(trigger.matches(&bob_event, &ctx));
    }

    #[test]
    fn test_does_not_match_unrelated_events() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let source_id = ObjectId::from_raw(1);

        let trigger = BeginningOfMainPhaseTrigger::your_precombat_main_phase();
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let event = TriggerEvent::new(BeginningOfUpkeepEvent::new(alice));
        assert!(!trigger.matches(&event, &ctx));
    }
}
