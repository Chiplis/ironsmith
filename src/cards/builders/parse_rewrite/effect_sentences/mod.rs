#[allow(unused_imports)]
use crate::cards::builders::{
    CardTextError, EffectAst, IT_TAG, IfResultPredicate, PlayerAst, PredicateAst,
    ReturnControllerAst, SubjectAst, TagKey, TargetAst, TextSpan, Token,
};
#[allow(unused_imports)]
use crate::effect::{ChoiceCount, Value};
#[allow(unused_imports)]
use crate::target::{ObjectFilter, PlayerFilter, TaggedObjectConstraint, TaggedOpbjectRelation};
#[allow(unused_imports)]
use crate::types::{CardType, Subtype};
#[allow(unused_imports)]
use crate::zone::Zone;
#[allow(unused_imports)]
use super::keyword_static::parse_where_x_value_clause;
#[allow(unused_imports)]
use super::object_filters::parse_object_filter;
#[allow(unused_imports)]
use self::sentence_helpers::*;
#[allow(unused_imports)]
use super::util::{
    is_source_reference_words, parse_counter_type_from_tokens, parse_subject, parse_target_phrase,
    parse_value, span_from_tokens,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TokenCopyFollowup {
    HasHaste,
    GainHasteUntilEndOfTurn,
    SacrificeAtNextEndStep,
    ExileAtNextEndStep,
    ExileAtEndOfCombat,
    SacrificeAtEndOfCombat,
}

mod sentence_helpers;
mod clause_dispatch;
pub(crate) mod clause_pattern_helpers;
mod chain_carry;
pub(crate) mod conditionals;
mod creation_handlers;
mod dispatch_entry;
mod dispatch_inner;
mod for_each_helpers;
mod gain_ability;
mod lex_chain_helpers;
mod search_library;
mod sentence_primitives;
mod verb_dispatch;
mod verb_handlers;
mod zone_handlers;
mod zone_counter_helpers;

pub(crate) use clause_dispatch::*;
pub(crate) use chain_carry::*;
pub(crate) use conditionals::*;
pub(crate) use dispatch_entry::*;
pub(crate) use dispatch_inner::*;
pub(crate) use gain_ability::*;
pub(crate) use search_library::*;
pub(crate) use sentence_primitives::*;
