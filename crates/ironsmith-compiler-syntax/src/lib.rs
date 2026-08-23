#![allow(dead_code)]

//! Oracle lexer and allocation-light token utilities.
//!
//! The implementation is compiled here even while its source files remain in
//! their historical locations, allowing the compatibility compiler facade to
//! keep stable module paths during the staged move.

pub mod diagnostics {
    pub use ironsmith_compiler_api::{CardTextError, ParseAnnotations, TextSpan};
}

#[path = "../../ironsmith-compiler/src/slice_primitives.rs"]
mod slice_primitives;
#[path = "../../ironsmith-compiler/src/string_primitives.rs"]
mod string_primitives;
#[path = "../../ironsmith-compiler/src/word_primitives.rs"]
mod word_primitives;

#[path = "../../ironsmith-compiler/src/front_end/lexer.rs"]
pub mod lexer;
#[path = "../../ironsmith-compiler/src/front_end/token_utils.rs"]
pub mod token_utils;

pub use lexer::*;
pub use token_utils::*;
