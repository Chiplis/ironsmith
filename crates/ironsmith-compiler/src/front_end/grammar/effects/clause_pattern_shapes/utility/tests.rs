use super::*;
use crate::lexer::{TokenWordView, lex_line};

#[path = "tests/core_programs.rs"]
mod core_programs;
use core_programs::{parses_inflected_connive_clause_shapes, parses_utility_clause_shapes};
#[path = "tests/counter_programs.rs"]
mod counter_programs;
use counter_programs::double_counter_holder_distinguishes_singular_source_from_filter_wide_sets;
