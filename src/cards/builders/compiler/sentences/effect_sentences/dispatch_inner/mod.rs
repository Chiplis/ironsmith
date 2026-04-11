use super::super::activation_and_restrictions::activated_line_core::{
    is_activate_only_restriction_sentence_lexed, is_trigger_only_restriction_sentence_lexed,
};
use super::super::clause_support::parse_trigger_clause_lexed;
use super::super::grammar::effects::parse_conditional_sentence_family_lexed;
use super::super::grammar::primitives::{
    self as grammar, TokenWordView, split_lexed_slices_on_and, split_lexed_slices_on_comma,
    split_lexed_slices_on_commas_or_semicolons,
};
use super::super::grammar::structure::{
    LeadingResultPrefixKind, split_leading_result_prefix_lexed,
};
use super::super::keyword_static::{
    parse_ability_line, parse_pt_modifier, parse_where_x_value_clause,
    parse_where_x_value_clause_lexed,
};
use super::super::lexer::OwnedLexToken;
use super::super::object_filters::{
    find_word_slice_phrase_start, parse_object_filter, parse_object_filter_lexed,
};
use super::super::rule_engine::{LexClauseView, LexUnsupportedDiagnoser, LexUnsupportedRuleDef};
use super::super::token_primitives::{
    find_index, find_window_by, iter_contains, rfind_index, slice_contains, slice_ends_with,
    slice_starts_with,
};
use super::super::util::{
    is_article, is_source_reference_words, parse_card_type, parse_subject, parse_target_phrase,
    parse_value, token_index_for_word_index, words,
};
use super::sentence_helpers::*;
use super::zone_handlers::collapse_leading_signed_pt_modifier_tokens;
use super::{
    TokenCopyFollowup, parse_cant_effect_sentence_lexed, parse_effect_chain_lexed,
    parse_search_library_sentence_lexed, parse_simple_gain_ability_clause,
};
#[allow(unused_imports)]
use crate::cards::builders::{
    CardTextError, EffectAst, ExtraTurnAnchorAst, IT_TAG, LineAst, PlayerAst, SubjectAst, TagKey,
    TargetAst, TextSpan, TriggerSpec, Verb,
};
use crate::effect::{ChoiceCount, EventValueSpec, Until, Value};
use crate::object::CounterType;
use crate::target::{
    ChooseSpec, ObjectFilter, PlayerFilter, TaggedObjectConstraint, TaggedOpbjectRelation,
};
use crate::types::CardType;
use crate::zone::Zone;

const EACH_PLAYER_PREFIXES: &[&[&str]] = &[&["each", "player"]];
const EACH_PLAYER_EXILES_ALL_PREFIXES: &[&[&str]] = &[&["each", "player", "exiles", "all"]];
const PREVENT_DAMAGE_BY_PREFIXES: &[&[&str]] = &[&["that", "would", "be", "dealt", "by"]];
const PREVENT_DAMAGE_TO_AND_BY_PREFIXES: &[&[&str]] =
    &[&["that", "would", "be", "dealt", "to", "and", "dealt", "by"]];
const PREVENT_DAMAGE_TO_PREFIXES: &[&[&str]] = &[&["that", "would", "be", "dealt", "to"]];
const EXILE_PREFIXES: &[&[&str]] = &[&["exile"]];
const RETURN_EACH_CREATURE_ISNT_PREFIXES: &[&[&str]] =
    &[&["return", "each", "creature", "that", "isnt"]];
const EXILE_ALL_CARDS_FROM_PREFIXES: &[&[&str]] = &[&["exile", "all", "cards", "from"]];
include!("sentence_shape_predicates.rs");
include!("labeled_prefixes.rs");
include!("copy_and_next_spell_shapes.rs");
include!("replacement_and_prevention_shapes.rs");
include!("unsupported_shape_diagnostics.rs");
