//! Thin WebAssembly binding link for the browser gameplay module.
//!
//! Session orchestration and wire conversion compile in the reusable
//! `ironsmith-web-session` rlib. The binding product owns only the final
//! `cdylib` boundary.

pub use ironsmith_web_session::*;
