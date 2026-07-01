use super::super::activation_and_restrictions::activated_line_core::{
    is_activate_only_restriction_sentence_lexed, is_trigger_only_restriction_sentence_lexed,
};
use super::super::activation_and_restrictions::trigger_subject_filters::parse_trigger_subject_player_filter;
use super::super::clause_support::parse_trigger_clause_lexed;
use super::super::grammar::effects as effect_grammar;
use super::super::grammar::effects::{
    parse_conditional_sentence_family_lexed, split_labeled_effect_prefix_lexed,
};
use super::super::grammar::primitives::{
    self as grammar, TokenWordView, split_lexed_slices_on_and, split_lexed_slices_on_comma,
    split_lexed_slices_on_commas_or_semicolons, split_lexed_slices_on_or,
};
use super::super::grammar::structure::{
    LeadingResultPrefixKind, split_leading_result_prefix_lexed,
};
use super::super::keyword_static::{
    parse_ability_line, parse_cost_modifier_mana_cost, parse_pt_modifier,
    parse_value_binding_clause, parse_value_binding_clause_lexed,
};
use super::super::lexer::{
    LexedClause, OwnedLexToken, TokenKind, contains_token_kind, find_token_word_sequence,
    parser_token_word_refs, render_token_slice, token_slice_at_is, token_slice_at_is_any,
    token_slice_first_is, token_slice_first_kind, token_slice_starts_with, word_slice_at_is,
    word_slice_at_is_any, word_slice_contains_any_word, word_slice_contains_phrase,
    word_slice_contains_word, word_slice_ends_with, word_slice_eq, word_slice_eq_any,
    word_slice_eq_at, word_slice_find_word, word_slice_first_is, word_slice_first_is_any,
    word_slice_matching_value, word_slice_starts_with, word_slice_starts_with_any,
    word_slice_starts_with_at,
};
use super::super::object_filters::{parse_object_filter, parse_object_filter_lexed};
use super::super::rule_engine::{LexClauseView, LexUnsupportedDiagnoser, LexUnsupportedRuleDef};
use super::super::token_primitives::{
    find_index, find_window_by, iter_contains, rfind_index, slice_contains, slice_ends_with,
    slice_starts_with,
};
use super::super::util::{
    is_article, is_source_reference_words, parse_card_type,
    parse_choice_count_before_target_prefix, parse_color, parse_counter_type_word,
    parse_filter_counter_constraint_words, parse_subject, parse_target_phrase, parse_value,
    token_index_for_word_index, words,
};
pub(crate) use super::super::util::{strip_leading_articles, trim_commas, trim_edge_punctuation};
use super::sentence_helpers::*;
use super::zone_handlers::collapse_leading_signed_pt_modifier_tokens;
use super::{
    TokenCopyFollowup, parse_cant_effect_sentence_lexed,
    parse_destroy_then_temporary_cant_attack_block_chain_lexed, parse_effect_chain_lexed,
    parse_search_library_sentence_lexed, parse_simple_gain_ability_clause,
};
#[allow(unused_imports)]
use crate::cards::builders::{
    CardTextError, EffectAst, ExtraTurnAnchorAst, GrantedAbilityAst, IT_TAG, KeywordAction,
    LineAst, PlayerAst, SubjectAst, SubjectVerbActionAst, SubjectVerbEffectAst, SubjectVerbRoleAst,
    SubjectVerbSubjectAst, TagKey, TargetAst, TextSpan, TriggerSpec, Verb,
};
use crate::effect::{ChoiceCount, EventValueSpec, Until, Value};
use crate::object::CounterType;
use crate::runtime_backend::lex_patterns::{
    LexCaptureKind, LexCaptureRole, LexPattern, LexPatternAtom,
};
use crate::target::{
    ChooseSpec, ObjectFilter, PlayerFilter, TaggedObjectConstraint, TaggedOpbjectRelation,
};
use crate::types::{CardType, Subtype};
use crate::zone::Zone;
use ironsmith_core::ValueSurfaceHint;

include!("sentence_shape_predicates.rs");
include!("generic_subject_verb_programs.rs");
include!("labeled_prefixes.rs");
include!("copy_and_next_spell_shapes.rs");
include!("replacement_and_prevention_shapes.rs");
include!("unsupported_shape_diagnostics.rs");
