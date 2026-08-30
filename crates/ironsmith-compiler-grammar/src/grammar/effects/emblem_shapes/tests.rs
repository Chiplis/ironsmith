use super::*;
use crate::lexer::{lex_line, render_token_slice};

#[path = "tests/resource.rs"]
mod resource_programs;
use resource_programs::{
    quoted_emblem_payload_accepts_only_a_synthetic_outer_period,
    quoted_emblem_payload_does_not_consume_an_unquoted_followup,
};
#[path = "tests/ability.rs"]
mod ability_programs;
use ability_programs::captures_one_or_multiple_quoted_ability_groups;
