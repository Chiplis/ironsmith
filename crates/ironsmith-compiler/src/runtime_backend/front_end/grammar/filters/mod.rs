use winnow::Parser;
use winnow::combinator::alt;

use super::super::activation_and_restrictions::activated_line_core::parse_named_number;
use super::super::keyword_static::parse_pt_modifier;
use super::super::lexer::OwnedLexToken;
use super::super::object_filters::{
    parse_attached_reference_or_another_disjunction, parse_object_filter_lexed, push_unique,
    set_has, slice_has,
};
use super::super::util::{
    apply_filter_keyword_constraint, comparison_to_at_least_threshold,
    comparison_to_strict_at_least_threshold, comparison_to_value_comparison_operator, is_article,
    is_demonstrative_object_head, is_non_outlaw_word, is_outlaw_word, is_permanent_type,
    is_source_reference_words, non_article_token_word_refs, non_article_word_refs,
    parse_alternative_cast_words, parse_card_type, parse_color,
    parse_filter_keyword_constraint_words, parse_greater_than_or_equal_quantity_prefix,
    parse_less_than_or_equal_quantity_prefix, parse_mana_symbol_word_flexible, parse_non_color,
    parse_non_subtype, parse_non_supertype, parse_non_type, parse_number, parse_number_word_u32,
    parse_quantity_comparison_prefix, parse_subtype_flexible, parse_subtype_word,
    parse_supertype_word, parse_unsigned_pt_word, parse_zone_word, push_outlaw_subtypes,
    source_reference_surface_for_words, strip_leading_article_word_refs,
    this_source_surface_for_words, trim_commas, word_refs_except,
};
use super::primitives::{self, TokenWordView, split_lexed_slices_on_and, split_lexed_slices_on_or};
use super::values::parse_mana_symbol;
use crate::cards::builders::{
    CardTextError, IT_TAG, PlayerAst, PredicateAst, THIS_WAY_SACRIFICED_TAG, TagKey,
    TurnHistoryPredicateAst,
};
use crate::color::{Color, ColorSet};
use crate::effect::Value;
use crate::effects::VOTE_WINNERS_TAG;
use crate::filter::TaggedObjectConstraint;
use crate::mana::ManaSymbol;
use crate::runtime_backend::grammar::shared_util::value_semantics::parse_filter_comparison_tokens;
use crate::target::{
    ObjectFilter, ObjectRef, PlayerFilter, SourceReferenceSurface, TaggedOpbjectRelation,
    TargetabilityConstraint,
};
use crate::types::{CardType, Subtype, Supertype};
use crate::zone::Zone;

mod chosen_type_references;
mod color_and_sticker_facts;
mod counter_constraints;
mod decorations;
mod domain_unions;
mod extremum;
mod meld_and_special_subjects;
mod naming_and_reference;
mod player_relations;
mod predicate_phrases;
pub(crate) mod reference_tag_stage;
mod reference_tag_word_facts;
mod simple;
pub(crate) mod spell_filters;

pub(super) use chosen_type_references::*;
use color_and_sticker_facts::*;
use domain_unions::*;
pub(crate) use extremum::{parse_extremum_object_filter_lexed, parse_extremum_object_filter_words};
pub(super) use meld_and_special_subjects::*;
use naming_and_reference::*;
use player_relations::*;
pub(super) use predicate_phrases::*;
pub(crate) use predicate_phrases::{
    WinnowAtom as PermissionAtom, WinnowCaptureKind as PermissionCaptureKind,
    WinnowCaptureRole as PermissionCaptureRole, WinnowSequence as PermissionSequence,
    parse_source_keyword_condition_filter as parse_source_keyword_condition_filter_lexed,
};
pub(super) use reference_tag_stage::*;

pub(crate) use counter_constraints::{
    intern_counter_name, parse_counter_type_from_tokens, parse_counter_type_word,
    parse_counter_type_words, parse_filter_counter_constraint_words,
    preserve_filter_counter_constraint_surface_tokens,
    preserve_filter_counter_constraint_surface_words,
};
pub(crate) use decorations::{
    apply_filter_tail_decoration, apply_parity_filter_phrases, parse_filter_distinct_names_tokens,
    parse_filter_lexed_envelope, parse_filter_tail_decoration_split_words,
    parse_filter_tail_decoration_tokens, parse_filter_word_envelope,
    strip_not_on_battlefield_phrase, trim_vote_winner_suffix,
};
pub(crate) use meld_and_special_subjects::parse_same_color_mana_spent_to_cast_predicate;
pub(crate) use reference_tag_stage::parse_object_filter_with_grammar_entrypoint_lexed;
pub(crate) use simple::{
    parse_filter_face_state_words, parse_simple_object_filter_lexed,
    parse_simple_object_filter_words,
};
pub(crate) use spell_filters::{
    parse_object_filter_with_grammar_entrypoint, parse_spell_filter_with_grammar_entrypoint,
    parse_spell_filter_with_grammar_entrypoint_lexed,
};
