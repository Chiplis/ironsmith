//! "Whenever this permanent deals damage" trigger.

use crate::events::DamageEvent;
use crate::events::EventKind;
use crate::filter::Comparison;
use crate::game_event::DamageTarget;
use crate::target::PlayerFilter;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct ThisDealsDamageTrigger {
    pub damaged_player: Option<PlayerFilter>,
    pub amount: Option<Comparison>,
    pub combat_only: bool,
}

impl ThisDealsDamageTrigger {
    pub fn new() -> Self {
        Self {
            damaged_player: None,
            amount: None,
            combat_only: false,
        }
    }

    pub fn with_player_filter(mut self, player: PlayerFilter) -> Self {
        self.damaged_player = Some(player);
        self
    }

    pub fn with_amount(mut self, amount: Comparison) -> Self {
        self.amount = Some(amount);
        self
    }

    pub fn combat_only(mut self) -> Self {
        self.combat_only = true;
        self
    }
}

fn describe_comparison(cmp: &Comparison) -> String {
    match cmp {
        Comparison::Equal(v) => format!("{v}"),
        Comparison::OneOf(values) => match values.len() {
            0 => String::new(),
            1 => values[0].to_string(),
            2 => format!("{} or {}", values[0], values[1]),
            _ => {
                let head = values[..values.len() - 1]
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{head}, or {}", values[values.len() - 1])
            }
        },
        Comparison::NotEqual(v) => format!("not {v}"),
        Comparison::LessThan(v) => format!("less than {v}"),
        Comparison::LessThanOrEqual(v) => format!("{v} or less"),
        Comparison::GreaterThan(v) => format!("greater than {v}"),
        Comparison::GreaterThanOrEqual(v) => format!("{v} or greater"),
    }
}

fn describe_player_filter(filter: &PlayerFilter) -> &'static str {
    match filter {
        PlayerFilter::You => "you",
        PlayerFilter::Opponent => "an opponent",
        _ => "a player",
    }
}

impl TriggerMatcher for ThisDealsDamageTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::Damage {
            return false;
        }
        let Some(e) = event.downcast::<DamageEvent>() else {
            return false;
        };
        if e.source != ctx.source_id {
            return false;
        }
        if self.combat_only && !e.is_combat {
            return false;
        }
        if let Some(amount) = &self.amount
            && !amount.satisfies(e.amount as i32)
        {
            return false;
        }
        if let Some(player_filter) = &self.damaged_player {
            let DamageTarget::Player(player) = e.target else {
                return false;
            };
            if !player_filter.matches_player(player, &ctx.filter_ctx) {
                return false;
            }
        }
        true
    }

    fn display(&self) -> String {
        let amount = self.amount.as_ref().map(describe_comparison);
        let mut text = if self.combat_only {
            if let Some(amount) = amount {
                format!("Whenever this permanent deals {amount} combat damage")
            } else {
                "Whenever this permanent deals combat damage".to_string()
            }
        } else if let Some(amount) = amount {
            format!("Whenever this permanent deals {amount} damage")
        } else {
            "Whenever this permanent deals damage".to_string()
        };
        if let Some(player_filter) = &self.damaged_player {
            text.push_str(" to ");
            text.push_str(describe_player_filter(player_filter));
        }
        text
    }

    fn clone_box(&self) -> Box<dyn TriggerMatcher> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_event::DamageTarget;
    use crate::game_state::GameState;
    use crate::ids::{ObjectId, PlayerId};

    #[test]
    fn test_matches() {
        let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source_id = ObjectId::from_raw(1);

        let trigger = ThisDealsDamageTrigger::new();
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let event = TriggerEvent::new(DamageEvent::new(
            source_id,
            DamageTarget::Player(bob),
            3,
            false,
        ));

        assert!(trigger.matches(&event, &ctx));
    }
}
