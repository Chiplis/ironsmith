//! Numbered-card draw triggers such as "your second card each turn" and
//! "your first or second card each turn".

use crate::events::EventKind;
use crate::events::other::CardsDrawnEvent;
use crate::target::PlayerFilter;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};
use crate::triggers::{TriggerEvent, describe_player_filter_subject};

/// Trigger for "Whenever [player] draws their Nth card each turn".
///
/// This fires once when the draw event includes the configured draw number.
#[derive(Debug, Clone, PartialEq)]
pub struct PlayerDrawsNthCardEachTurnTrigger {
    pub player: PlayerFilter,
    pub card_number: u32,
}

/// Trigger for any of a reusable set of numbered draws each turn.
///
/// Unlike an `OrTrigger` of single-number matchers, this matcher preserves
/// multiplicity when one batched draw event crosses more than one configured
/// ordinal.
#[derive(Debug, Clone, PartialEq)]
pub struct PlayerDrawsNumberedCardsEachTurnTrigger {
    pub player: PlayerFilter,
    pub card_numbers: Vec<u32>,
}

impl PlayerDrawsNthCardEachTurnTrigger {
    pub fn new(player: PlayerFilter, card_number: u32) -> Self {
        Self {
            player,
            card_number,
        }
    }
}

impl PlayerDrawsNumberedCardsEachTurnTrigger {
    pub fn new(player: PlayerFilter, card_numbers: impl IntoIterator<Item = u32>) -> Self {
        let mut card_numbers = card_numbers
            .into_iter()
            .filter(|number| *number > 0)
            .collect::<Vec<_>>();
        card_numbers.sort_unstable();
        card_numbers.dedup();
        Self {
            player,
            card_numbers,
        }
    }

    fn matching_card_numbers(&self, event: &TriggerEvent, ctx: &TriggerContext) -> u32 {
        let Some((total_before, total_after)) = draw_number_window(&self.player, event, ctx) else {
            return 0;
        };
        self.card_numbers
            .iter()
            .filter(|number| total_before < **number && **number <= total_after)
            .count() as u32
    }
}

fn draw_number_window(
    player: &PlayerFilter,
    event: &TriggerEvent,
    ctx: &TriggerContext,
) -> Option<(u32, u32)> {
    if event.kind() != EventKind::CardsDrawn {
        return None;
    }
    let e = event.downcast::<CardsDrawnEvent>()?;
    let player_matches = match player {
        PlayerFilter::You => e.player == ctx.controller,
        PlayerFilter::Opponent => e.player != ctx.controller,
        PlayerFilter::Any => true,
        PlayerFilter::Specific(id) => e.player == *id,
        _ => true,
    };
    if !player_matches {
        return None;
    }
    let total_after = ctx
        .game
        .turn_store
        .turn_history
        .cards_drawn_by_player(e.player);
    Some((total_after.saturating_sub(e.amount()), total_after))
}

fn numbered_draw_display(player: &PlayerFilter, card_numbers: &[u32]) -> String {
    let ordinals = card_numbers
        .iter()
        .map(|number| {
            ironsmith_core::ordinal_word(*number).unwrap_or_else(|| format!("{number}th"))
        })
        .collect::<Vec<_>>();
    let ordinal_text = match ordinals.as_slice() {
        [] => "numbered".to_string(),
        [ordinal] => ordinal.clone(),
        [first, second] => format!("{first} or {second}"),
        many => format!(
            "{}, or {}",
            many[..many.len() - 1].join(", "),
            many.last().expect("numbered draw list is nonempty")
        ),
    };
    match player {
        PlayerFilter::You => format!("Whenever you draw your {ordinal_text} card each turn"),
        PlayerFilter::Any => {
            format!("Whenever a player draws their {ordinal_text} card each turn")
        }
        PlayerFilter::Opponent => {
            format!("Whenever an opponent draws their {ordinal_text} card each turn")
        }
        PlayerFilter::Specific(_) | PlayerFilter::IteratedPlayer => {
            format!("Whenever that player draws their {ordinal_text} card each turn")
        }
        _ => format!(
            "Whenever {} draws their {ordinal_text} card each turn",
            describe_player_filter_subject(player)
        ),
    }
}

impl TriggerMatcher for PlayerDrawsNthCardEachTurnTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if self.card_number == 0 {
            return false;
        }
        draw_number_window(&self.player, event, ctx).is_some_and(|(total_before, total_after)| {
            total_before < self.card_number && self.card_number <= total_after
        })
    }

    fn subscribed_kinds(&self) -> Option<Vec<EventKind>> {
        Some(vec![EventKind::CardsDrawn])
    }

    fn display(&self) -> String {
        let ordinal =
            ironsmith_core::ordinal_word(self.card_number).unwrap_or_else(|| "nth".to_string());
        match &self.player {
            PlayerFilter::You => format!("Whenever you draw your {ordinal} card each turn"),
            PlayerFilter::Any => format!("Whenever a player draws their {ordinal} card each turn"),
            PlayerFilter::Opponent => {
                format!("Whenever an opponent draws their {ordinal} card each turn")
            }
            PlayerFilter::Specific(_) | PlayerFilter::IteratedPlayer => {
                format!("Whenever that player draws their {ordinal} card each turn")
            }
            _ => format!(
                "Whenever {} draws their {ordinal} card each turn",
                describe_player_filter_subject(&self.player)
            ),
        }
    }
}

impl TriggerMatcher for PlayerDrawsNumberedCardsEachTurnTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        self.matching_card_numbers(event, ctx) > 0
    }

    fn trigger_count_with_context(&self, event: &TriggerEvent, ctx: &TriggerContext) -> u32 {
        self.matching_card_numbers(event, ctx)
    }

    fn subscribed_kinds(&self) -> Option<Vec<EventKind>> {
        Some(vec![EventKind::CardsDrawn])
    }

    fn display(&self) -> String {
        numbered_draw_display(&self.player, &self.card_numbers)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_state::GameState;
    use crate::ids::{ObjectId, PlayerId};

    #[test]
    fn test_display() {
        let trigger = PlayerDrawsNthCardEachTurnTrigger::new(PlayerFilter::You, 2);
        assert!(trigger.display().contains("second card each turn"));
    }

    #[test]
    fn numbered_set_display_preserves_all_ordinals() {
        let trigger = PlayerDrawsNumberedCardsEachTurnTrigger::new(PlayerFilter::You, [1, 2]);
        assert_eq!(
            trigger.display(),
            "Whenever you draw your first or second card each turn"
        );
    }

    #[test]
    fn numbered_set_matches_first_and_second_separate_draws() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source_id = ObjectId::from_raw(1);
        let trigger = PlayerDrawsNumberedCardsEachTurnTrigger::new(PlayerFilter::You, [1, 2]);

        let first = TriggerEvent::new_with_provenance(
            CardsDrawnEvent::single(alice, ObjectId::from_raw(2), false),
            crate::provenance::ProvNodeId::default(),
        );
        game.stage_turn_history_event(&first);
        let first_ctx = TriggerContext::for_source(source_id, alice, &game);
        assert!(trigger.matches(&first, &first_ctx));
        assert_eq!(trigger.trigger_count_with_context(&first, &first_ctx), 1);

        let second = TriggerEvent::new_with_provenance(
            CardsDrawnEvent::single(alice, ObjectId::from_raw(3), false),
            crate::provenance::ProvNodeId::default(),
        );
        game.stage_turn_history_event(&second);
        let second_ctx = TriggerContext::for_source(source_id, alice, &game);
        assert!(trigger.matches(&second, &second_ctx));
        assert_eq!(trigger.trigger_count_with_context(&second, &second_ctx), 1);
    }

    #[test]
    fn numbered_set_counts_each_ordinal_crossed_by_one_batched_draw() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source_id = ObjectId::from_raw(1);
        let event = TriggerEvent::new_with_provenance(
            CardsDrawnEvent::new(
                alice,
                vec![ObjectId::from_raw(2), ObjectId::from_raw(3)],
                true,
            ),
            crate::provenance::ProvNodeId::default(),
        );
        game.stage_turn_history_event(&event);
        let ctx = TriggerContext::for_source(source_id, alice, &game);
        let trigger = PlayerDrawsNumberedCardsEachTurnTrigger::new(PlayerFilter::You, [1, 2]);

        assert!(trigger.matches(&event, &ctx));
        assert_eq!(trigger.trigger_count_with_context(&event, &ctx), 2);
    }

    #[test]
    fn test_matches_second_draw() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source_id = ObjectId::from_raw(1);

        let prior_event = TriggerEvent::new_with_provenance(
            CardsDrawnEvent::single(alice, ObjectId::from_raw(10), true),
            crate::provenance::ProvNodeId::default(),
        );
        game.stage_turn_history_event(&prior_event);
        let event = TriggerEvent::new_with_provenance(
            CardsDrawnEvent::single(alice, ObjectId::from_raw(2), false),
            crate::provenance::ProvNodeId::default(),
        );
        game.stage_turn_history_event(&event);
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let trigger = PlayerDrawsNthCardEachTurnTrigger::new(PlayerFilter::You, 2);
        assert!(trigger.matches(&event, &ctx));
    }

    #[test]
    fn test_matches_second_draw_in_two_card_draw_event() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source_id = ObjectId::from_raw(1);

        let event = TriggerEvent::new_with_provenance(
            CardsDrawnEvent::new(
                alice,
                vec![ObjectId::from_raw(2), ObjectId::from_raw(3)],
                true,
            ),
            crate::provenance::ProvNodeId::default(),
        );
        game.stage_turn_history_event(&event);
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let trigger = PlayerDrawsNthCardEachTurnTrigger::new(PlayerFilter::You, 2);
        assert!(trigger.matches(&event, &ctx));
    }

    #[test]
    fn test_does_not_match_wrong_number() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source_id = ObjectId::from_raw(1);

        let prior_event = TriggerEvent::new_with_provenance(
            CardsDrawnEvent::new(
                alice,
                vec![ObjectId::from_raw(10), ObjectId::from_raw(11)],
                true,
            ),
            crate::provenance::ProvNodeId::default(),
        );
        game.stage_turn_history_event(&prior_event);
        let event = TriggerEvent::new_with_provenance(
            CardsDrawnEvent::single(alice, ObjectId::from_raw(2), false),
            crate::provenance::ProvNodeId::default(),
        );
        game.stage_turn_history_event(&event);
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let trigger = PlayerDrawsNthCardEachTurnTrigger::new(PlayerFilter::You, 2);
        assert!(!trigger.matches(&event, &ctx));
    }
}
