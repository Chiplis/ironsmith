//! Exhaustive, indexed ports of the pinned Wymi cases. See fixtures/wymi/README.md.
#[path = "wymi/adapter.rs"]
mod adapter;
include!("wymi/cases.rs");
