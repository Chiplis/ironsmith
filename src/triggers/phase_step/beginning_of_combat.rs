//! "At the beginning of combat on [player]'s turn" trigger.

use crate::events::EventKind;
use crate::ids::PlayerId;
use crate::target::PlayerFilter;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

/// Trigger that fires at the beginning of combat.
///
/// Used by cards like Hero of Bladehold, Aurelia, and many others.
#[derive(Debug, Clone, PartialEq)]
pub struct BeginningOfCombatTrigger {
    /// Which player's combat triggers this ability.
    pub player: PlayerFilter,
}

impl BeginningOfCombatTrigger {
    /// Create a new combat trigger for the specified player.
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }

    /// Create a combat trigger for your combat.
    pub fn your_combat() -> Self {
        Self::new(PlayerFilter::You)
    }

    /// Create a combat trigger for each player's combat.
    pub fn each_combat() -> Self {
        Self::new(PlayerFilter::Any)
    }
}

impl TriggerMatcher for BeginningOfCombatTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::BeginningOfCombat {
            return false;
        }
        let Some(player) = event.player() else {
            return false;
        };
        player_filter_matches(&self.player, player, ctx)
    }

    fn display(&self) -> String {
        match &self.player {
            PlayerFilter::You => "At the beginning of combat on your turn".to_string(),
            PlayerFilter::Any => "At the beginning of each combat".to_string(),
            _ => format!("At the beginning of combat on {:?}'s turn", self.player),
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
    use crate::events::phase::BeginningOfCombatEvent;
    use crate::game_state::GameState;
    use crate::ids::ObjectId;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    #[test]
    fn test_matches_own_combat() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let source_id = ObjectId::from_raw(1);

        let trigger = BeginningOfCombatTrigger::your_combat();
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let event = TriggerEvent::new(BeginningOfCombatEvent::new(alice));
        assert!(trigger.matches(&event, &ctx));
    }

    #[test]
    fn test_display() {
        let trigger = BeginningOfCombatTrigger::your_combat();
        assert!(trigger.display().contains("combat"));
    }
}
