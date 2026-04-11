use winnow::Parser;
use winnow::combinator::alt;
use winnow::error::{ContextError, ErrMode};

use super::super::activation_and_restrictions::activated_line_core::parse_named_number;
use super::super::keyword_static::parse_pt_modifier;
use super::super::lexer::OwnedLexToken;
use super::super::object_filters::{
    apply_parity_filter_phrases, find_word_slice_phrase_start, lower_words_find_index,
    normalized_token_index_after_words, normalized_token_index_for_word_index,
    parse_attached_reference_or_another_disjunction, parse_filter_face_state_words,
    parse_object_filter_lexed, push_unique, set_has, slice_has, strip_not_on_battlefield_phrase,
    token_find_index, trim_vote_winner_suffix,
};
use super::super::token_primitives::{
    find_index, rfind_index, slice_contains, slice_ends_with, slice_starts_with, slice_strip_prefix,
};
use super::super::util::{
    apply_filter_keyword_constraint, is_article, is_demonstrative_object_head, is_non_outlaw_word,
    is_outlaw_word, is_permanent_type, is_source_reference_words, parse_alternative_cast_words,
    parse_card_type, parse_color, parse_counter_type_word, parse_filter_counter_constraint_words,
    parse_filter_keyword_constraint_words, parse_mana_symbol_word_flexible, parse_non_color,
    parse_non_subtype, parse_non_supertype, parse_non_type, parse_number, parse_subtype_flexible,
    parse_subtype_word, parse_supertype_word, parse_unsigned_pt_word, parse_zone_word,
    push_outlaw_subtypes, trim_commas,
};
use super::super::value_helpers::parse_filter_comparison_tokens;
use super::primitives::{self, TokenWordView, split_lexed_slices_on_and, split_lexed_slices_on_or};
use super::values::parse_mana_symbol;
use crate::cards::TextSpan;
use crate::cards::builders::{CardTextError, IT_TAG, PlayerAst, PredicateAst, TagKey};
use crate::color::{Color, ColorSet};
use crate::effect::Value;
use crate::effects::VOTE_WINNERS_TAG;
use crate::filter::TaggedObjectConstraint;
use crate::mana::ManaSymbol;
use crate::target::{ObjectFilter, PlayerFilter, TaggedOpbjectRelation};
use crate::types::{CardType, Supertype};
use crate::zone::Zone;

mod meld_and_special_subjects;
mod naming_and_reference;
mod player_relations;
mod predicate_phrases;
pub(crate) mod reference_tag_stage;
pub(crate) mod spell_filters;
mod with_without_clauses;

pub(super) use meld_and_special_subjects::*;
pub(super) use naming_and_reference::*;
pub(super) use player_relations::*;
pub(super) use predicate_phrases::*;
pub(super) use reference_tag_stage::*;
pub(super) use spell_filters::*;
pub(super) use with_without_clauses::*;

pub(crate) use reference_tag_stage::parse_object_filter_with_grammar_entrypoint_lexed;
pub(crate) use spell_filters::{
    parse_object_filter_with_grammar_entrypoint, parse_spell_filter_with_grammar_entrypoint,
    parse_spell_filter_with_grammar_entrypoint_lexed,
};
