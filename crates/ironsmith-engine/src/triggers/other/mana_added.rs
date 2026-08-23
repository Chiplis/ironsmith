//! "Whenever mana is added to [player]'s mana pool" trigger.

use crate::events::{EventKind, ManaAddedEvent};
use crate::target::PlayerFilter;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct ManaAddedTrigger {
    pub player: PlayerFilter,
}

impl ManaAddedTrigger {
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }
}

impl TriggerMatcher for ManaAddedTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::ManaAdded {
            return false;
        }
        let Some(e) = event.downcast::<ManaAddedEvent>() else {
            return false;
        };
        if e.mana.is_empty() {
            return false;
        }

        match &self.player {
            PlayerFilter::You => e.player == ctx.controller,
            PlayerFilter::Opponent => e.player != ctx.controller,
            PlayerFilter::Any => true,
            PlayerFilter::Active => ctx.game.is_active_player(e.player),
            PlayerFilter::Specific(id) => e.player == *id,
            PlayerFilter::IteratedPlayer => event.trigger_player() == Some(e.player),
            _ => true,
        }
    }

    fn display(&self) -> String {
        let player = match &self.player {
            PlayerFilter::You => "you add mana".to_string(),
            PlayerFilter::Opponent => "an opponent adds mana".to_string(),
            PlayerFilter::Any => "a player adds mana".to_string(),
            PlayerFilter::Active => "the active player adds mana".to_string(),
            PlayerFilter::Specific(_) | PlayerFilter::IteratedPlayer => {
                "that player adds mana".to_string()
            }
            _ => "a player adds mana".to_string(),
        };
        format!("Whenever {player}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_state::GameState;
    use crate::ids::{ObjectId, PlayerId};
    use crate::mana::ManaSymbol;

    #[test]
    fn mana_added_trigger_matches_controller_mana() {
        let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source = ObjectId::from_raw(1);
        let trigger = ManaAddedTrigger::new(PlayerFilter::You);
        let ctx = TriggerContext::for_source(source, alice, &game);
        let event = ManaAddedEvent::trigger_event(source, alice, alice, vec![ManaSymbol::Green]);

        assert!(trigger.matches(&event, &ctx));
    }

    #[test]
    fn mana_added_trigger_ignores_empty_mana_events() {
        let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source = ObjectId::from_raw(1);
        let trigger = ManaAddedTrigger::new(PlayerFilter::You);
        let ctx = TriggerContext::for_source(source, alice, &game);
        let event = ManaAddedEvent::trigger_event(source, alice, alice, Vec::new());

        assert!(!trigger.matches(&event, &ctx));
    }
}
