use crate::events::{CoinFlippedEvent, EventKind};
use crate::target::PlayerFilter;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};
use crate::triggers::{TriggerEvent, player_filter_matches_with_context};

#[derive(Debug, Clone, PartialEq)]
pub struct PlayerCoinFlipResultTrigger {
    pub player: PlayerFilter,
    pub won: bool,
}

impl PlayerCoinFlipResultTrigger {
    pub fn new(player: PlayerFilter, won: bool) -> Self {
        Self { player, won }
    }
}

impl TriggerMatcher for PlayerCoinFlipResultTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::CoinFlipped {
            return false;
        }
        let Some(event) = event.downcast::<CoinFlippedEvent>() else {
            return false;
        };
        let matches_result = if self.won {
            event.flipper_won()
        } else {
            event.flipper_lost()
        };
        matches_result
            && player_filter_matches_with_context(
                &self.player,
                event.player,
                ctx.controller,
                ctx.game,
                None,
            )
    }

    fn subscribed_kinds(&self) -> Option<Vec<EventKind>> {
        Some(vec![EventKind::CoinFlipped])
    }

    fn display(&self) -> String {
        let result = if self.won { "wins" } else { "loses" };
        match &self.player {
            PlayerFilter::You => format!(
                "Whenever you {} a coin flip",
                if self.won { "win" } else { "lose" }
            ),
            PlayerFilter::Opponent => format!("Whenever an opponent {result} a coin flip"),
            PlayerFilter::Any => format!("Whenever a player {result} a coin flip"),
            PlayerFilter::Specific(_) => format!("Whenever that player {result} a coin flip"),
            _ => format!("Whenever a player {result} a coin flip"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_state::GameState;
    use crate::ids::{ObjectId, PlayerId};

    fn event(
        player: PlayerId,
        call: Option<ironsmith_core::CoinFace>,
        winner: Option<PlayerId>,
    ) -> TriggerEvent {
        TriggerEvent::new_with_provenance(
            CoinFlippedEvent {
                player,
                source: ObjectId::from_raw(99),
                face: ironsmith_core::CoinFace::Heads,
                call,
                winner,
                loser: call
                    .is_some()
                    .then_some(player)
                    .filter(|_| winner.is_none()),
            },
            crate::provenance::ProvNodeId::default(),
        )
    }

    #[test]
    fn win_and_loss_triggers_ignore_face_only_flips() {
        let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source = ObjectId::from_raw(1);
        let ctx = TriggerContext::for_source(source, alice, &game);
        let win = PlayerCoinFlipResultTrigger::new(PlayerFilter::You, true);
        let loss = PlayerCoinFlipResultTrigger::new(PlayerFilter::You, false);

        assert!(win.matches(
            &event(alice, Some(ironsmith_core::CoinFace::Heads), Some(alice)),
            &ctx
        ));
        assert!(loss.matches(
            &event(alice, Some(ironsmith_core::CoinFace::Tails), None),
            &ctx
        ));
        assert!(!win.matches(&event(alice, None, None), &ctx));
        assert!(!loss.matches(&event(alice, None, None), &ctx));
    }
}
