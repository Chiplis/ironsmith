//! Qualified die results and captured completed-turn ordinals.
use crate::events::{EventKind, other::DieRolledEvent};
use crate::target::{Comparison, PlayerFilter};
use crate::triggers::matcher_trait::{SimultaneousTriggerKey, TriggerContext, TriggerMatcher};
use crate::triggers::{TriggerEvent, player_filter_matches_with_context};
#[derive(Debug, Clone, PartialEq)]
pub struct QualifiedDieRollTrigger {
    pub player: PlayerFilter,
    pub result: Option<Comparison>,
    pub natural: bool,
    pub ordinal: Option<u32>,
}
impl TriggerMatcher for QualifiedDieRollTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        let Some(roll) = event.downcast::<DieRolledEvent>() else {
            return false;
        };
        if !player_filter_matches_with_context(
            &self.player,
            roll.player,
            ctx.controller,
            ctx.game,
            None,
        ) {
            return false;
        }
        if let Some(ordinal) = self.ordinal {
            if ordinal == 0 || roll.ordinal_this_turn != Some(ordinal) {
                return false;
            }
        }
        if let Some(result) = &self.result {
            if roll.is_planar {
                return false;
            }
            let value = if self.natural {
                roll.natural_result
            } else {
                roll.result
            };
            if !i32::try_from(value)
                .ok()
                .is_some_and(|value| result.satisfies(value))
            {
                return false;
            }
        }
        true
    }
    fn subscribed_kinds(&self) -> Option<Vec<EventKind>> {
        Some(vec![EventKind::DieRolled])
    }
    fn simultaneous_trigger_key(&self, event: &TriggerEvent) -> Option<SimultaneousTriggerKey> {
        self.ordinal.and_then(|_| {
            event
                .downcast::<DieRolledEvent>()
                .map(|roll| SimultaneousTriggerKey::PlayerDieRollBatch(roll.player))
        })
    }
    fn display(&self) -> String {
        let who = self.player.description();
        let verb = if self.player == PlayerFilter::You {
            "roll"
        } else {
            "rolls"
        };
        if let Some(ordinal) = self.ordinal {
            let ordinal =
                ironsmith_core::ordinal_word(ordinal).unwrap_or_else(|| ordinal.to_string());
            let possessive = if self.player == PlayerFilter::You {
                "your"
            } else {
                "their"
            };
            return format!("Whenever {who} {verb} {possessive} {ordinal} die each turn");
        }
        let result = match self.result.as_ref() {
            Some(Comparison::Equal(n)) => n.to_string(),
            Some(Comparison::GreaterThanOrEqual(n)) => format!("{n} or higher"),
            Some(Comparison::OneOf(values)) => values
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(" or "),
            other => format!("{other:?}"),
        };
        let natural = if self.natural { "natural " } else { "" };
        format!("Whenever {who} {verb} a {natural}{result}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{GameState, ObjectId, PlayerId};
    #[test]
    fn natural_modified_result_and_captured_ordinal_never_alias() {
        let game = GameState::new(vec!["A".into(), "B".into()], 30);
        let source = ObjectId::from_raw(5);
        let ctx = TriggerContext::for_source(source, PlayerId(0), &game);
        let natural = QualifiedDieRollTrigger {
            player: PlayerFilter::You,
            result: Some(Comparison::Equal(20)),
            natural: true,
            ordinal: None,
        };
        let modified = QualifiedDieRollTrigger {
            natural: false,
            ..natural.clone()
        };
        let nth = QualifiedDieRollTrigger {
            player: PlayerFilter::You,
            result: None,
            natural: false,
            ordinal: Some(3),
        };
        let roll = |player, n, r, ordinal| {
            TriggerEvent::new_with_provenance(
                DieRolledEvent::new_with_natural_result(player, source, n, r, 20)
                    .with_turn_ordinal(ordinal),
                Default::default(),
            )
        };
        assert!(modified.matches(&roll(PlayerId(0), 19, 20, 2), &ctx));
        assert!(!natural.matches(&roll(PlayerId(0), 19, 20, 2), &ctx));
        assert!(natural.matches(&roll(PlayerId(0), 20, 21, 3), &ctx));
        assert!(!modified.matches(&roll(PlayerId(0), 20, 21, 3), &ctx));
        assert!(nth.matches(&roll(PlayerId(0), 2, 2, 3), &ctx));
        assert!(!nth.matches(&roll(PlayerId(1), 2, 2, 3), &ctx));
        let planar = TriggerEvent::new_with_provenance(
            DieRolledEvent::new_planar(PlayerId(0), source, 2).with_turn_ordinal(3),
            Default::default(),
        );
        assert!(nth.matches(&planar, &ctx));
        assert!(!natural.matches(&planar, &ctx));
        assert!(!modified.matches(&planar, &ctx));
        let unknown = TriggerEvent::new_with_provenance(
            DieRolledEvent::new(PlayerId(0), source, 3, 6),
            Default::default(),
        );
        assert!(!nth.matches(&unknown, &ctx));
    }
}
