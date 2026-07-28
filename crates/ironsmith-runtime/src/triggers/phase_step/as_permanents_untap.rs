//! Delayed "as [player] untaps their permanents" timing matcher.

use crate::events::EventKind;
use crate::filter::PlayerFilterExt as _;
use crate::target::PlayerFilter;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};
use crate::triggers::{TriggerEvent, describe_player_filter_possessive};
use crate::zone::Zone;

/// Matches the turn-based permanent-untap action in a player's untap step.
///
/// This matcher is used only for delayed instructions that the turn runner
/// executes immediately, without putting a triggered ability on the stack.
#[derive(Debug, Clone, PartialEq)]
pub struct AsPermanentsUntapTrigger {
    pub player: PlayerFilter,
    pub source_must_be_controlled: bool,
}

impl AsPermanentsUntapTrigger {
    pub fn new(player: PlayerFilter, source_must_be_controlled: bool) -> Self {
        Self {
            player,
            source_must_be_controlled,
        }
    }

    /// Whether this player's untap action consumes the delayed instruction,
    /// independent of whether the source is still one of their permanents.
    pub(crate) fn timing_matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        event.kind() == EventKind::PermanentsUntapStep
            && event
                .player()
                .is_some_and(|player| self.player.matches_player(player, &ctx.filter_ctx))
    }
}

impl TriggerMatcher for AsPermanentsUntapTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if !self.timing_matches(event, ctx) {
            return false;
        }
        if !self.source_must_be_controlled {
            return true;
        }
        let Some(player) = event.player() else {
            return false;
        };
        ctx.game.object(ctx.source_id).is_some_and(|source| {
            source.zone == Zone::Battlefield
                && !ctx.game.is_phased_out(ctx.source_id)
                && ctx.game.controller_of(source) == player
        })
    }

    fn subscribed_kinds(&self) -> Option<Vec<EventKind>> {
        Some(vec![EventKind::PermanentsUntapStep])
    }

    fn display(&self) -> String {
        match &self.player {
            PlayerFilter::You => "During your untap step, as you untap your permanents".to_string(),
            PlayerFilter::Any => {
                "During each player's untap step, as that player untaps their permanents"
                    .to_string()
            }
            PlayerFilter::Opponent => {
                "During each opponent's untap step, as that player untaps their permanents"
                    .to_string()
            }
            player => format!(
                "During {} untap step, as that player untaps their permanents",
                describe_player_filter_possessive(player)
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::events::phase::PermanentsUntapStepEvent;
    use crate::ids::{CardId, PlayerId};
    use crate::triggers::TriggerEvent;
    use crate::types::CardType;

    #[test]
    fn requires_source_to_remain_among_players_permanents() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.create_object_from_card(
            &CardBuilder::new(CardId::new(), "Delayed Land")
                .card_types(vec![CardType::Land])
                .build(),
            alice,
            Zone::Battlefield,
        );
        let trigger = AsPermanentsUntapTrigger::new(PlayerFilter::You, true);
        let event = TriggerEvent::new_with_provenance(
            PermanentsUntapStepEvent::new(alice),
            crate::provenance::ProvNodeId::default(),
        );
        let ctx = TriggerContext::for_source(source, alice, &game);
        assert!(trigger.matches(&event, &ctx));

        game.object_mut(source).expect("source exists").owner = bob;
        let ctx = TriggerContext::for_source(source, alice, &game);
        assert!(trigger.timing_matches(&event, &ctx));
        assert!(!trigger.matches(&event, &ctx));
    }
}
