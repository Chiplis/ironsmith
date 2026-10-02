use super::*;

include!("helpers.rs");
include!("external_registry.rs");
include!("dispatch.rs");
include!("undo.rs");
include!("pregame.rs");
include!("runtime_flow.rs");
include!("sync_checkpoint.rs");
include!("manabrew_compat.rs");

#[cfg(test)]
#[path = "manabrew_payment_conformance.rs"]
mod manabrew_payment_conformance;

include!("priority_analysis.rs");

include!("runtime_savepoint.rs");

#[cfg(test)]
mod runtime_audit_devourer;

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "../tests/hidden_resolution.rs"]
mod hidden_resolution_tests;
