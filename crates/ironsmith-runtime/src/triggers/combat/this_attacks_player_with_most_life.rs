//! "Whenever this creature attacks the player with the most life or tied for most life" trigger.

use crate::events::EventKind;
use crate::events::combat::{AttackEventTarget, CreatureAttackedEvent};
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

/// Trigger that fires when the source creature attacks a player with the most life
/// (including ties).
#[derive(Debug, Clone, PartialEq)]
pub struct ThisAttacksPlayerWithMostLifeTrigger;

impl TriggerMatcher for ThisAttacksPlayerWithMostLifeTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::CreatureAttacked {
            return false;
        }
        let Some(e) = event.downcast::<CreatureAttackedEvent>() else {
            return false;
        };
        if e.attacker != ctx.source_id {
            return false;
        }

        let AttackEventTarget::Player(defending_player) = e.target else {
            return false;
        };

        let defending_life = ctx
            .game
            .player(defending_player)
            .map(|p| p.life)
            .unwrap_or(i32::MIN);

        let max_life = ctx.game.players.iter().map(|player| player.life).max();
        max_life.is_some_and(|max_life| defending_life == max_life)
    }

    fn display(&self) -> String {
        "Whenever this creature attacks the player with the most life or tied for most life"
            .to_string()
    }
}
