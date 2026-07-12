use super::super::activation_and_restrictions::activated_line_core::{
    is_activate_only_restriction_sentence_lexed, is_trigger_only_restriction_sentence_lexed,
};
use super::super::activation_and_restrictions::trigger_subject_filters::parse_trigger_subject_player_filter;
use super::super::clause_support::parse_trigger_clause_lexed;
use super::super::grammar::effects as effect_grammar;
use super::super::grammar::effects::sentence_predicate_shapes as sentence_shapes;
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
    LexedClause, OwnedLexToken, TokenKind, complete_word_sequence_at,
    complete_word_sequence_choice, complete_word_sequence_surface, contains_token_kind,
    find_token_word_sequence, locate_word, parser_token_word_refs, render_token_slice,
    token_prefix_present, token_slice_at_is, token_slice_at_is_any, token_slice_first_is,
    token_slice_first_kind, word_choice_present, word_prefix_choice_present, word_prefix_present,
    word_prefix_present_at, word_present, word_sequence_present, word_slice_at_is,
    word_slice_at_is_any, word_slice_first_is, word_slice_first_is_any, word_slice_matching_value,
    word_suffix_present,
};
use super::super::object_filters::{parse_object_filter, parse_object_filter_lexed};
use super::super::rule_engine::{LexClauseView, LexUnsupportedDiagnoser, LexUnsupportedRuleDef};
use super::super::token_primitives::{
    contains_window, find_window_by, items_end_with, items_have, items_start_with, iter_contains,
    locate_index, locate_last_index,
};
use super::super::util::{
    is_article, is_source_reference_words, parse_card_type,
    parse_choice_count_before_target_prefix, parse_color, parse_counter_type_word,
    parse_filter_counter_constraint_words, parse_subject, parse_target_phrase, parse_value,
    token_boundary_for_word, words,
};
pub(crate) use super::super::util::{strip_leading_articles, trim_commas, trim_edge_punctuation};
use super::sentence_helpers::*;
use super::zone_handlers::collapse_leading_signed_pt_modifier_tokens;
use super::{
    TokenCopyFollowup, parse_cant_effect_sentence_lexed,
    parse_destroy_then_temporary_cant_attack_block_chain_lexed, parse_effect_chain_lexed,
    parse_reveal_source_exiled_permanents_sentence_lexed, parse_search_library_sentence_lexed,
    parse_simple_gain_ability_clause,
};
#[allow(unused_imports)]
use crate::cards::builders::{
    CardTextError, EffectAst, ExtraTurnAnchorAst, GrantedAbilityAst, IT_TAG, KeywordAction,
    LineAst, PlayerAst, SubjectAst, SubjectVerbActionAst, SubjectVerbEffectAst, SubjectVerbRoleAst,
    SubjectVerbSubjectAst, TagKey, TargetAst, TextSpan, TriggerSpec, Verb,
};
use crate::effect::{ChoiceCount, EventValueSpec, Until, Value};
use crate::object::CounterType;
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
