#![expect(
    clippy::single_element_loop,
    reason = "boundary assertions use uniform forbidden-pattern tables that intentionally remain extensible"
)]

#[path = "workspace_boundaries/mod.rs"]
mod suite;
