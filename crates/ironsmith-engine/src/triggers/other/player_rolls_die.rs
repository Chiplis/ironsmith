//! "Whenever [player] rolls a die" trigger.

use crate::events::EventKind;
use crate::events::other::DieRolledEvent;
use crate::target::PlayerFilter;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};
use crate::triggers::{TriggerEvent, player_filter_matches_with_context};

#[derive(Debug, Clone, PartialEq)]
pub struct PlayerRollsDieTrigger {
    pub player: PlayerFilter,
    /// Oracle groups a single roll event as "one or more dice" even when the
    /// event contains multiple physical dice.
    pub one_or_more: bool,
    pub attraction_visit_only: bool,
}

impl PlayerRollsDieTrigger {
    pub fn for_attraction_visit(player: PlayerFilter) -> Self {
        Self {
            player,
            one_or_more: false,
            attraction_visit_only: true,
        }
    }

    pub fn new(player: PlayerFilter) -> Self {
        Self::with_surface(player, false)
    }

    pub fn with_surface(player: PlayerFilter, one_or_more: bool) -> Self {
        Self {
            player,
            one_or_more,
            attraction_visit_only: false,
        }
    }
}

impl TriggerMatcher for PlayerRollsDieTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::DieRolled {
            return false;
        }
        let Some(e) = event.downcast::<DieRolledEvent>() else {
            return false;
        };

        if self.attraction_visit_only && (!e.is_attraction_visit || e.is_planar) {
            return false;
        }
        player_filter_matches_with_context(&self.player, e.player, ctx.controller, ctx.game, None)
    }

    fn display(&self) -> String {
        if self.attraction_visit_only {
            return match &self.player {
                PlayerFilter::You => "Whenever you roll to visit your Attractions".to_string(),
                PlayerFilter::Opponent => {
                    "Whenever an opponent rolls to visit their Attractions".to_string()
                }
                PlayerFilter::Active => {
                    "Whenever the active player rolls to visit their Attractions".to_string()
                }
                PlayerFilter::Specific(_) => {
                    "Whenever that player rolls to visit their Attractions".to_string()
                }
                _ => "Whenever a player rolls to visit their Attractions".to_string(),
            };
        }
        if self.one_or_more {
            return match &self.player {
                PlayerFilter::You => "Whenever you roll one or more dice".to_string(),
                PlayerFilter::Opponent => "Whenever an opponent rolls one or more dice".to_string(),
                PlayerFilter::Any => "Whenever a player rolls one or more dice".to_string(),
                PlayerFilter::Active => {
                    "Whenever the active player rolls one or more dice".to_string()
                }
                PlayerFilter::Specific(_) => {
                    "Whenever that player rolls one or more dice".to_string()
                }
                _ => "Whenever a player rolls one or more dice".to_string(),
            };
        }
        match &self.player {
            PlayerFilter::You => "Whenever you roll a die".to_string(),
            PlayerFilter::Opponent => "Whenever an opponent rolls a die".to_string(),
            PlayerFilter::Any => "Whenever a player rolls a die".to_string(),
            PlayerFilter::Active => "Whenever the active player rolls a die".to_string(),
            PlayerFilter::Specific(_) => "Whenever that player rolls a die".to_string(),
            _ => "Whenever a player rolls a die".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grouped_roll_surface_is_preserved() {
        assert_eq!(
            PlayerRollsDieTrigger::with_surface(PlayerFilter::You, true).display(),
            "Whenever you roll one or more dice"
        );
        assert_eq!(
            PlayerRollsDieTrigger::new(PlayerFilter::You).display(),
            "Whenever you roll a die"
        );
    }
}

#[cfg(test)]
mod attraction_roll_tests {
    use super::*;
    use crate::{
        GameState,
        ids::{ObjectId, PlayerId},
    };

    #[test]
    fn attraction_roll_trigger_distinguishes_purpose_and_player() {
        let game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = ObjectId::from_raw(123);
        let ctx = TriggerContext::for_source(source, alice, &game);
        let visit = PlayerRollsDieTrigger::for_attraction_visit(PlayerFilter::You);
        let ordinary = PlayerRollsDieTrigger::new(PlayerFilter::You);
        for result in [1, 5, 6] {
            let event = TriggerEvent::new_with_provenance(
                DieRolledEvent::new(alice, source, result, 6).for_attraction_visit(),
                Default::default(),
            );
            assert!(visit.matches(&event, &ctx));
            assert!(ordinary.matches(&event, &ctx));
            let ordinary_event = TriggerEvent::new_with_provenance(
                DieRolledEvent::new(alice, source, result, 6),
                Default::default(),
            );
            assert!(!visit.matches(&ordinary_event, &ctx));
        }
        let other_player = TriggerEvent::new_with_provenance(
            DieRolledEvent::new(bob, source, 6, 6).for_attraction_visit(),
            Default::default(),
        );
        assert!(!visit.matches(&other_player, &ctx));
        let planar = TriggerEvent::new_with_provenance(
            DieRolledEvent::new_planar(alice, source, 6),
            Default::default(),
        );
        assert!(!visit.matches(&planar, &ctx));
    }
}
