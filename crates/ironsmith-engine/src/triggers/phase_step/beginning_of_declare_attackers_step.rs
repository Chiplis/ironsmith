//! "At the beginning of the declare attackers step on [player's] turn".

use crate::events::EventKind;
use crate::filter::PlayerFilterExt as _;
use crate::target::PlayerFilter;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};
use crate::triggers::{TriggerEvent, describe_player_filter_possessive};

/// Fires at the beginning of a declare attackers step (CR 508), after the
/// attack declaration turn-based action.
#[derive(Debug, Clone, PartialEq)]
pub struct BeginningOfDeclareAttackersStepTrigger {
    /// Whose turn's declare attackers step triggers this ability.
    pub player: PlayerFilter,
}

impl BeginningOfDeclareAttackersStepTrigger {
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }
}

impl TriggerMatcher for BeginningOfDeclareAttackersStepTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::BeginningOfDeclareAttackersStep {
            return false;
        }
        let Some(player) = event.player() else {
            return false;
        };
        self.player.matches_player(player, &ctx.filter_ctx)
    }

    fn subscribed_kinds(&self) -> Option<Vec<EventKind>> {
        Some(vec![EventKind::BeginningOfDeclareAttackersStep])
    }

    fn display(&self) -> String {
        match &self.player {
            PlayerFilter::You => "At the beginning of the declare attackers step on your turn".to_string(),
            PlayerFilter::Any => "At the beginning of each declare attackers step".to_string(),
            _ => format!(
                "At the beginning of the declare attackers step on {} turn",
                describe_player_filter_possessive(&self.player)
            ),
        }
    }
}
