//! Integration tests for complex game mechanics.

#[cfg(feature = "engine-integration-tests")]
mod inferno_support_tests;
pub mod integration_tests;
#[cfg(feature = "engine-integration-tests")]
mod layer_system_tests;
pub(crate) mod test_helpers;
