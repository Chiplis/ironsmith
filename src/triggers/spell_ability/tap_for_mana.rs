//! "Whenever [player] taps [object] for mana" trigger.

use crate::events::EventKind;
use crate::events::spells::AbilityActivatedEvent;
use crate::target::{ObjectFilter, PlayerFilter};
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct TapForManaTrigger {
    pub player: PlayerFilter,
    pub filter: ObjectFilter,
}

impl TapForManaTrigger {
    pub fn new(player: PlayerFilter, filter: ObjectFilter) -> Self {
        Self { player, filter }
    }
}

impl TriggerMatcher for TapForManaTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::AbilityActivated {
            return false;
        }
        let Some(e) = event.downcast::<AbilityActivatedEvent>() else {
            return false;
        };
        if !e.is_mana_ability {
            return false;
        }

        let player_matches = match &self.player {
            PlayerFilter::You => e.activator == ctx.controller,
            PlayerFilter::Opponent => e.activator != ctx.controller,
            PlayerFilter::Any => true,
            PlayerFilter::Specific(id) => e.activator == *id,
            _ => true,
        };
        if !player_matches {
            return false;
        }

        if let Some(obj) = ctx.game.object(e.source) {
            self.filter.matches(obj, &ctx.filter_ctx, ctx.game)
        } else if let Some(snapshot) = e.snapshot.as_ref() {
            self.filter
                .matches_snapshot(snapshot, &ctx.filter_ctx, ctx.game)
        } else {
            false
        }
    }

    fn display(&self) -> String {
        let player = match &self.player {
            PlayerFilter::You => "you".to_string(),
            PlayerFilter::Opponent => "an opponent".to_string(),
            PlayerFilter::Any => "a player".to_string(),
            PlayerFilter::Specific(_) | PlayerFilter::IteratedPlayer => "that player".to_string(),
            other => describe_player_filter_fallback(other),
        };
        let verb = if matches!(self.player, PlayerFilter::You) {
            "tap"
        } else {
            "taps"
        };
        let object = self.filter.description();
        let object_phrase = if starts_with_determiner(&object) {
            object
        } else {
            format!("a {object}")
        };
        format!("Whenever {player} {verb} {object_phrase} for mana")
    }

    fn clone_box(&self) -> Box<dyn TriggerMatcher> {
        Box::new(self.clone())
    }
}

fn describe_player_filter_fallback(filter: &PlayerFilter) -> String {
    match filter {
        PlayerFilter::Defending => "the defending player".to_string(),
        PlayerFilter::Attacking => "the attacking player".to_string(),
        PlayerFilter::Active => "the active player".to_string(),
        PlayerFilter::Teammate => "a teammate".to_string(),
        _ => "a player".to_string(),
    }
}

fn starts_with_determiner(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.starts_with("a ")
        || lower.starts_with("an ")
        || lower.starts_with("the ")
        || lower.starts_with("another ")
        || lower.starts_with("other ")
        || lower.starts_with("target ")
        || lower.starts_with("that ")
}
