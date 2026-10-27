#![allow(dead_code)]

//! Oracle lexer and allocation-light token utilities.
//!
//! This crate physically owns lossless lexing and allocation-light token
//! primitives. Later parser phases consume these modules through the crate
//! boundary rather than recompiling their source with path attributes.

pub mod diagnostics {
    pub use ironsmith_compiler_api::{CardTextError, ParseAnnotations, TextSpan};
}

pub mod slice_primitives;
pub mod string_primitives;
pub mod word_primitives;

pub mod lexer;
pub mod token_utils;

pub use lexer::*;
pub use token_utils::*;
