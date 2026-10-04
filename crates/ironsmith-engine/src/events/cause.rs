//! Event causation tracking for composable replacement effect matching.

use crate::filter::FilterContext;
use crate::filter::ObjectFilterExt as _;
use crate::game_state::GameState;
use crate::ids::PlayerId;

pub use ironsmith_core::{CauseFilter, CauseType, CauseTypeFilter, ControllerFilter, EventCause};

pub trait CauseFilterRuntimeExt {
    fn matches(&self, cause: &EventCause, game: &GameState, affected_player: PlayerId) -> bool;
    fn matches_with_context_controller(
        &self,
        cause: &EventCause,
        game: &GameState,
        affected_player: PlayerId,
        context_controller: PlayerId,
    ) -> bool;
}

impl CauseFilterRuntimeExt for CauseFilter {
    fn matches(&self, cause: &EventCause, game: &GameState, affected_player: PlayerId) -> bool {
        self.matches_with_context_controller(cause, game, affected_player, affected_player)
    }

    fn matches_with_context_controller(
        &self,
        cause: &EventCause,
        game: &GameState,
        affected_player: PlayerId,
        context_controller: PlayerId,
    ) -> bool {
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
                    .is_some_and(|controller| game.are_opponents(affected_player, controller)),
                ControllerFilter::ContextController => {
                    cause.source_controller == Some(context_controller)
                }
                ControllerFilter::ContextOpponent => cause
                    .source_controller
                    .is_some_and(|controller| game.are_opponents(context_controller, controller)),
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
    fn opponent_cause_filter_respects_teams_and_captured_controller() {
        let mut game = GameState::new(
            vec!["Alice".into(), "Bob".into(), "Carol".into(), "Dan".into()],
            20,
        );
        let [alice, bob, carol, dan] = [0, 1, 2, 3].map(PlayerId::from_index);
        game.set_teams(vec![vec![alice, bob], vec![carol, dan]])
            .unwrap();
        let filter = CauseFilter::any().with_controller(ControllerFilter::Opponent);
        // The source need not still exist for its captured controller to matter.
        for (controller, expected) in [(alice, false), (bob, false), (carol, true), (dan, true)] {
            assert_eq!(
                filter.matches(
                    &EventCause::from_effect(ObjectId::from_raw(999), controller),
                    &game,
                    alice
                ),
                expected
            );
        }
        assert!(!filter.matches(&EventCause::from_game_rule(), &game, alice));
    }

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

#[cfg(test)]
mod trigger_cause_tests {
    use super::*;
    use crate::condition_eval::{ExternalEvaluationContext, evaluate_condition_external};
    use crate::ids::ObjectId;
    use crate::triggers::TriggerEvent;
    #[test]
    fn trigger_cause_controller_is_frozen_distinct_from_victim_and_respects_teams_and_action_kind() {
        let mut game = GameState::new(vec!["A".into(), "B".into(), "C".into(), "D".into()], 20);
        let [a,b,c,d] = [0,1,2,3].map(PlayerId::from_index);
        game.set_teams(vec![vec![a,b],vec![c,d]]).unwrap();
        let condition = crate::effect::Condition::TriggeringEventCausedBy { controller: crate::target::PlayerFilter::Opponent, effect_like_only: true };
        for (cause, expected) in [
            (Some(EventCause::from_effect(ObjectId::from_raw(77), c)), true),
            (Some(EventCause::from_effect(ObjectId::from_raw(77), b)), false),
            (Some(EventCause::from_cost(ObjectId::from_raw(77), c)), false),
            (Some(EventCause::from_sba()), false),
            (None, false),
        ] {
            let mut event = crate::events::SpellCounteredEvent::new(ObjectId::from_raw(88), a, None);
            event.cause = cause;
            let event = TriggerEvent::new_with_provenance(event, Default::default());
            let context = ExternalEvaluationContext { controller: a, source: ObjectId::from_raw(99), triggering_event: Some(&event), ..Default::default() };
            assert_eq!(evaluate_condition_external(&game, &condition, &context), expected);
            // No live object 77 exists: the causing spell/ability's captured
            // controller cannot be replaced by the affected spell's controller.
        }
    }
}
