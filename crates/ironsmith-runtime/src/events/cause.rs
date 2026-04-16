//! Event causation tracking for composable replacement effect matching.

use crate::filter::FilterContext;
use crate::filter::ObjectFilterExt as _;
use crate::game_state::GameState;
use crate::ids::PlayerId;

pub use ironsmith_core::{CauseFilter, CauseType, CauseTypeFilter, ControllerFilter, EventCause};

pub trait CauseFilterRuntimeExt {
    fn matches(&self, cause: &EventCause, game: &GameState, affected_player: PlayerId) -> bool;
}

impl CauseFilterRuntimeExt for CauseFilter {
    fn matches(&self, cause: &EventCause, game: &GameState, affected_player: PlayerId) -> bool {
        if let Some(ref type_filter) = self.cause_type
            && !type_filter.matches(cause.cause_type)
        {
            return false;
        }

        if let Some(ref source_filter) = self.source_filter {
            let Some(source_id) = cause.source else {
                return false;
            };
            let Some(source_obj) = game.object(source_id) else {
                return false;
            };
            let filter_ctx = FilterContext::new(affected_player);
            if !source_filter.matches(source_obj, &filter_ctx, game) {
                return false;
            }
        }

        if let Some(ref controller_filter) = self.controller_filter {
            let matches_controller = match controller_filter {
                ControllerFilter::Player(player) => cause.source_controller == Some(*player),
                ControllerFilter::You => cause.source_controller == Some(affected_player),
                ControllerFilter::Opponent => cause
                    .source_controller
                    .is_some_and(|controller| controller != affected_player),
                ControllerFilter::Any => true,
            };
            if !matches_controller {
                return false;
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::ObjectId;

    #[test]
    fn test_cause_type_is_effect_like() {
        assert!(CauseType::Effect.is_effect_like());
        assert!(!CauseType::GameRule.is_effect_like());
        assert!(!CauseType::StateBasedAction.is_effect_like());
        assert!(!CauseType::CombatDamage.is_effect_like());
        assert!(!CauseType::Cost.is_effect_like());
        assert!(!CauseType::SpecialAction.is_effect_like());
        assert!(!CauseType::LegendRule.is_effect_like());
    }

    #[test]
    fn test_cause_filter_any() {
        let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        let filter = CauseFilter::any();

        assert!(filter.matches(
            &EventCause::from_effect(ObjectId::from_raw(1), alice),
            &game,
            alice
        ));
        assert!(filter.matches(
            &EventCause::from_cost(ObjectId::from_raw(1), alice),
            &game,
            alice
        ));
    }

    #[test]
    fn test_effect_like_filter() {
        let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let filter = CauseFilter::effect_like();

        assert!(filter.matches(
            &EventCause::from_effect(ObjectId::from_raw(1), alice),
            &game,
            alice
        ));
        assert!(!filter.matches(&EventCause::from_game_rule(), &game, alice));
        assert!(!filter.matches(
            &EventCause::from_cost(ObjectId::from_raw(1), alice),
            &game,
            alice
        ));
    }

    #[test]
    fn test_not_type_filter() {
        let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let filter = CauseFilter::not_type(CauseType::SpecialAction);

        assert!(filter.matches(
            &EventCause::from_effect(ObjectId::from_raw(1), alice),
            &game,
            alice
        ));
        assert!(!filter.matches(
            &EventCause::from_special_action(Some(ObjectId::from_raw(1)), alice),
            &game,
            alice
        ));
    }
}
