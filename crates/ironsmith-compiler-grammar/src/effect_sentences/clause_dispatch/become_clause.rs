use super::super::super::keyword_static::{keyword_action_to_static_ability, parse_ability_line};
use super::super::super::lexer::{LexedClause, OwnedLexToken, TokenKind};
use super::super::super::object_filters::parse_object_filter_lexed;
use super::super::super::util::{
    parse_subject, parse_target_phrase, parse_value, span_from_tokens,
};
use super::super::clause_pattern_helpers::extract_subject_player;
use super::super::parse_granted_abilities_for_gain_clause;
use super::super::search_library::parse_restriction_duration;
use super::super::zone_counter_helpers::parse_half_starting_life_total_value;
use super::helpers::render_lower_words;
use crate::cards::builders::GrantedAbilityAst;
use crate::effect::{Until, Value};
use crate::grammar::effects::become_shapes as become_grammar;
use crate::host::{CardTextError, EffectAst, PredicateAst, TagKey, TargetAst};
use crate::target::{ChooseSpec, ObjectFilter};
use crate::types::{CardType, SubtypeFamily};

fn trailing_duration_belongs_to_quoted_ability(
    tokens: &[OwnedLexToken],
    remainder: &[OwnedLexToken],
) -> bool {
    // A suffix duration is outer only when it begins outside quoted rules text.
    // Sentence splitting intentionally trims a closing quote that follows a
    // period, so an odd quote count in the retained prefix is meaningful here.
    if remainder.is_empty() || !crate::slice_primitives::starts_with(tokens, remainder) {
        return false;
    }
    remainder
        .iter()
        .filter(|token| token.kind == TokenKind::Quote)
        .count()
        % 2
        == 1
}

#[cfg(test)]
#[path = "become_clause_inline_quoted_duration_tests.rs"]
mod quoted_duration_tests;

#[path = "become_clause/object_action.rs"]
mod object_action_programs;
pub use object_action_programs::parse_become_clause;
