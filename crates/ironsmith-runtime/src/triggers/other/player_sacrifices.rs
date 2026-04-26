//! "Whenever [player] sacrifices [filter]" trigger.

use crate::events::EventKind;
use crate::events::permanents::SacrificeEvent;
use crate::filter::ObjectFilterExt as _;
use crate::target::{ObjectFilter, PlayerFilter};
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

fn with_indefinite_article(text: &str) -> String {
    let trimmed = text.trim();
    if trimmed.starts_with("a ")
        || trimmed.starts_with("an ")
        || trimmed.starts_with("the ")
        || trimmed.starts_with("another ")
    {
        return trimmed.to_string();
    }
    let article = if matches!(
        trimmed.chars().next().map(|ch| ch.to_ascii_lowercase()),
        Some('a' | 'e' | 'i' | 'o' | 'u')
    ) {
        "an"
    } else {
        "a"
    };
    format!("{article} {trimmed}")
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlayerSacrificesTrigger {
    pub player: PlayerFilter,
    pub filter: ObjectFilter,
}

impl PlayerSacrificesTrigger {
    pub fn new(player: PlayerFilter, filter: ObjectFilter) -> Self {
        Self { player, filter }
    }
}

impl TriggerMatcher for PlayerSacrificesTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::Sacrifice {
            return false;
        }
        let Some(e) = event.downcast::<SacrificeEvent>() else {
            return false;
        };

        let sacrificing_player = e
            .sacrificing_player
            .or_else(|| e.snapshot.as_ref().map(|s| s.controller))
            .or_else(|| {
                ctx.game
                    .object(e.permanent)
                    .map(|o| ctx.game.controller_of(o))
            });

        let player_matches = match (&self.player, sacrificing_player) {
            (PlayerFilter::You, Some(player)) => player == ctx.controller,
            (PlayerFilter::Opponent, Some(player)) => player != ctx.controller,
            (PlayerFilter::Any, Some(_)) => true,
            (PlayerFilter::Specific(expected), Some(player)) => player == *expected,
            (PlayerFilter::You, None)
            | (PlayerFilter::Opponent, None)
            | (PlayerFilter::Any, None)
            | (PlayerFilter::Specific(_), None) => false,
            _ => true,
        };
        if !player_matches {
            return false;
        }

        if let Some(obj) = ctx.game.object(e.permanent) {
            self.filter.matches(obj, &ctx.filter_ctx, ctx.game)
        } else if let Some(snapshot) = e.snapshot.as_ref() {
            self.filter
                .matches_snapshot(snapshot, &ctx.filter_ctx, ctx.game)
        } else {
            false
        }
    }

    fn display(&self) -> String {
        let player_text = match &self.player {
            PlayerFilter::You => "you sacrifice",
            PlayerFilter::Opponent => "an opponent sacrifices",
            PlayerFilter::Any => "a player sacrifices",
            _ => "someone sacrifices",
        };
        format!(
            "Whenever {} {}",
            player_text,
            with_indefinite_article(&self.filter.description())
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display() {
        let trigger = PlayerSacrificesTrigger::new(PlayerFilter::Any, ObjectFilter::creature());
        assert!(trigger.display().contains("sacrifices"));
    }
}
