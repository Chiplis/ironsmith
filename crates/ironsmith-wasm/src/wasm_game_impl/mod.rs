use super::*;

include!("helpers.rs");
include!("opponent_choices.rs");
include!("external_registry.rs");
include!("dispatch.rs");
include!("undo.rs");
include!("pregame.rs");
include!("runtime_flow.rs");
include!("public_audit.rs");
include!("manabrew_compat.rs");

#[cfg(test)]
#[path = "manabrew_payment_conformance.rs"]
mod manabrew_payment_conformance;

include!("priority_analysis.rs");

include!("runtime_savepoint.rs");
include!("payment_disclosure_transaction.rs");
include!("blind_exile_play.rs");

#[cfg(test)]
mod runtime_audit_devourer;

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "../tests/hidden_resolution.rs"]
mod hidden_resolution_tests;

#[cfg(test)]
mod static_top_visibility_tests;
#[cfg(test)]
mod exact_permission_savepoint_tests;

#[cfg(test)]
mod face_down_zone_permission_tests;

#[cfg(test)]
mod resource_payment_view_tests {
    use super::*;
    #[test]
    fn inventory_failure_is_explicit_and_does_not_change_preferences_or_game() {
        let _ids = crate::test_id_counter_guard();
        let (mut game, _, source, mut request) = resource_payment_test_fixture();
        game.set_token_creation_limits(ironsmith::effects::tokens::TokenCreationLimits { max_created_tokens: 1, ..Default::default() });
        request.preferences.preserve_sources.push(source);
        let before = request.clone();
        let ids = snapshot_id_counters();
        assert!(matches!(mana_activation_option_views(&game, &request), Err(ironsmith::effects::ExecutionError::ResourceLimitExceeded { .. })));
        assert!(matches!(manual_mana_ability_views(&game, &request), Err(ironsmith::effects::ExecutionError::ResourceLimitExceeded { .. })));
        assert!(matches!(mana_payment_options_view(&game, &mut request), Err(ironsmith::effects::ExecutionError::ResourceLimitExceeded { .. })));
        assert_eq!(request, before);
        let after = snapshot_id_counters();
        assert_eq!((after.player, after.object, after.card), (ids.player, ids.object, ids.card));
        assert!(!game.is_tapped(source)); assert_eq!(game.battlefield.len(), 1);
    }
    #[test]
    fn eager_snapshot_and_manabrew_prompt_never_publish_a_failed_payment_inventory() {
        let _ids = crate::test_id_counter_guard();
        let (game, player, source, request) = resource_payment_test_fixture();
        let plan = ironsmith::mana_payment::plan_first_mana_payment(&game, &request).unwrap();
        let context = DecisionContext::ManaPayment(ironsmith::decisions::context::ManaPaymentContext::new(
            player, source, "Resource payment", request, plan));
        let mut wasm = WasmGame::new(); wasm.game = game;
        wasm.pending_decision = Some(context.clone());
        wasm.defer_mana_options = false;
        wasm.game.set_token_creation_limits(ironsmith::effects::tokens::TokenCreationLimits { max_created_tokens: 1, ..Default::default() });
        assert!(matches!(wasm.current_mana_payment_view_checked(), Err(ironsmith::effects::ExecutionError::ResourceLimitExceeded { .. })));
        assert!(wasm.mana_activation_inventory_cache.borrow().is_none());
        assert!(wasm.snapshot_json_for_host().is_err());
        assert!(wasm.build_manabrew_prompt(&context).is_err());
        assert!(!wasm.game.is_tapped(source)); assert_eq!(wasm.game.battlefield.len(), 1);
        wasm.game.set_token_creation_limits(Default::default());
        assert!(wasm.current_mana_payment_view_checked().unwrap().is_some());
    }
}

#[cfg(test)]
mod activation_threshold_savepoint_tests;

#[cfg(test)]
mod activation_kind_cost_savepoints;

#[cfg(test)]
mod combat_participant_savepoint_tests;

#[cfg(test)]
mod runner_decision_rollback_tests;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod next_step_duration_savepoint_tests;

#[cfg(test)]
mod prevention_step_savepoint_tests;
