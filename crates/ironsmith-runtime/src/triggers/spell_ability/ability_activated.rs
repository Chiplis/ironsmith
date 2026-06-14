//! "Whenever an ability of [filter] is activated" trigger.

use crate::events::EventKind;
use crate::events::spells::AbilityActivatedEvent;
use crate::filter::ObjectFilterExt as _;
use crate::filter::PlayerFilterExt;
use crate::target::{ObjectFilter, PlayerFilter};
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct AbilityActivatedTrigger {
    pub activator: PlayerFilter,
    pub filter: ObjectFilter,
    pub non_mana_only: bool,
    pub loyalty_only: bool,
}

impl AbilityActivatedTrigger {
    pub fn new(activator: PlayerFilter, filter: ObjectFilter, non_mana_only: bool) -> Self {
        Self {
            activator,
            filter,
            non_mana_only,
            loyalty_only: false,
        }
    }

    pub fn loyalty_only(mut self, loyalty_only: bool) -> Self {
        self.loyalty_only = loyalty_only;
        self
    }
}

fn activate_verb(subject: &str) -> &'static str {
    if subject.eq_ignore_ascii_case("you") || subject.eq_ignore_ascii_case("they") {
        "activate"
    } else {
        "activates"
    }
}

fn source_filter_phrase(filter: &ObjectFilter) -> String {
    let description = filter.description();
    let lower = description.to_ascii_lowercase();
    if lower.starts_with("a ")
        || lower.starts_with("an ")
        || lower.starts_with("the ")
        || lower.starts_with("target ")
        || lower.starts_with("each ")
    {
        return description;
    }
    let article = if lower
        .chars()
        .next()
        .is_some_and(|ch| matches!(ch, 'a' | 'e' | 'i' | 'o' | 'u'))
    {
        "an"
    } else {
        "a"
    };
    format!("{article} {description}")
}

impl TriggerMatcher for AbilityActivatedTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::AbilityActivated {
            return false;
        }
        let Some(e) = event.downcast::<AbilityActivatedEvent>() else {
            return false;
        };
        if self.non_mana_only && e.is_mana_ability {
            return false;
        }
        if self.loyalty_only && !e.is_loyalty_ability {
            return false;
        }
        if self.filter.has_x_in_cost && !e.activation_cost_has_x {
            return false;
        }
        if !self.activator.matches_player(e.activator, &ctx.filter_ctx) {
            return false;
        }

        let mut source_filter = self.filter.clone();
        source_filter.has_x_in_cost = false;
        if let Some(obj) = ctx.game.object(e.source) {
            source_filter.matches(obj, &ctx.filter_ctx, ctx.game)
        } else if let Some(snapshot) = e.snapshot.as_ref() {
            source_filter.matches_snapshot(snapshot, &ctx.filter_ctx, ctx.game)
        } else {
            false
        }
    }

    fn display(&self) -> String {
        let subject = self.activator.description();
        let verb = activate_verb(&subject);
        let ability = if self.loyalty_only {
            "a loyalty ability"
        } else if self.non_mana_only {
            "a non-mana ability"
        } else {
            "an ability"
        };
        if self.filter.has_x_in_cost {
            format!(
                "Whenever {subject} {verb} {ability} with an activation cost that contains {{X}}"
            )
        } else if self.filter == ObjectFilter::default() {
            format!("Whenever {subject} {verb} {ability}")
        } else {
            format!(
                "Whenever {subject} {verb} {ability} of {}",
                source_filter_phrase(&self.filter)
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display() {
        let trigger =
            AbilityActivatedTrigger::new(PlayerFilter::Any, ObjectFilter::default(), false);
        assert!(trigger.display().contains("activates"));
    }
}
