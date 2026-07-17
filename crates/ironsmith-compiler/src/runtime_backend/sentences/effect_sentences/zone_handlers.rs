use crate::cards::builders::{
    ADDITIONAL_COST_OBJECT_TAG, CardTextError, EffectAst, ExchangeValueAst, ExchangeValueKindAst,
    IT_TAG, OwnedLexToken, PlayerAst, PredicateAst, ReturnControllerAst, SharedTypeConstraintAst,
    SubjectAst, TagKey, TargetAst,
};
use crate::effect::{EventValueSpec, Until, Value};
use crate::mana::ManaCost;
use crate::object::CounterType;
use crate::target::{
    ChooseSpec, ObjectFilter, PlayerFilter, TaggedObjectConstraint, TaggedOpbjectRelation,
};
use crate::zone::Zone;

use super::super::activation_and_restrictions::controller_filter_for_token_player;
use super::super::grammar::primitives as grammar;
use super::super::grammar::structure::{
    ConditionalPredicateTailSpec, parse_conditional_predicate_tail_lexed,
    split_trailing_if_clause_lexed,
};
use super::super::keyword_static::{
    parse_add_mana_equal_amount_value, parse_dynamic_cost_modifier_value, parse_pt_modifier,
    parse_pt_modifier_values,
};
use super::super::lexer::{LexStream, TokenKind};
use super::super::object_filters::{parse_object_filter, parse_object_filter_lexed};
use super::super::util::{
    helper_tag_for_tokens, intern_counter_name, is_article, parse_counter_type_word,
    parse_target_phrase, parse_value, span_from_tokens, trim_commas,
};
use super::clause_pattern_helpers::extract_subject_player;
use super::conditionals::parse_mana_symbol_group;
pub(crate) use super::zone_counter_helpers::{
    apply_exile_subject_hand_owner_context, apply_exile_subject_owner_context,
    parse_half_starting_life_total_value, split_until_source_leaves_tail,
};
use crate::runtime_backend::grammar::shared_util::value_semantics::parse_equal_to_number_of_filter_value;

use super::for_each_helpers::{
    parse_get_for_each_count_value, parse_get_modifier_values_with_tail,
};
use super::search_library::parse_restriction_duration;
use super::subject_verb_primitives::find_color_choice_phrase;

const ADDITIONAL_PREFIXES: &[&[&str]] = &[&["an", "additional"], &["additional"]];
const ANY_AMOUNT_OF_PREFIXES: &[&[&str]] = &[&["any", "amount", "of"]];

#[path = "emblem_actions.rs"]
mod emblem_actions;
#[path = "exile_actions.rs"]
mod exile_actions;
#[path = "mana_actions.rs"]
mod mana_actions;
#[path = "misc_actions.rs"]
mod misc_actions;
#[path = "remove_destroy.rs"]
mod remove_destroy;
#[path = "return_exchange.rs"]
mod return_exchange;
#[path = "sacrifice_discard.rs"]
mod sacrifice_discard;
#[path = "tap_actions.rs"]
mod tap_actions;

pub(crate) use emblem_actions::*;
pub(crate) use exile_actions::*;
pub(crate) use mana_actions::*;
pub(crate) use misc_actions::*;
pub(crate) use remove_destroy::*;
pub(crate) use return_exchange::*;
pub(crate) use sacrifice_discard::*;
pub(crate) use tap_actions::*;

#[cfg(test)]
#[path = "zone_handlers_tests.rs"]
mod tests;
