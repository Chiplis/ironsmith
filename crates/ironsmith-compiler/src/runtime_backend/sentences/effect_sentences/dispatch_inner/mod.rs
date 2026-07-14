use super::super::activation_and_restrictions::activated_line_core::{
    is_activate_only_restriction_sentence_lexed, is_trigger_only_restriction_sentence_lexed,
};
use super::super::clause_support::parse_trigger_clause_lexed;
use super::super::grammar::effects as effect_grammar;
use super::super::grammar::effects::sentence_predicate_shapes as sentence_shapes;
use super::super::grammar::effects::{
    parse_conditional_sentence_family_lexed, split_labeled_effect_prefix_lexed,
};
use super::super::grammar::primitives::{
    TokenWordView, split_lexed_slices_on_and, split_lexed_slices_on_comma,
};
use super::super::grammar::structure::{
    LeadingResultPrefixKind, split_leading_result_prefix_lexed,
};
use super::super::keyword_static::{parse_value_binding_clause, parse_value_binding_clause_lexed};
use super::super::lexer::{LexedClause, OwnedLexToken, render_token_slice, token_slice_first_is};
use super::super::object_filters::{parse_object_filter, parse_object_filter_lexed};
use super::super::rule_engine::LexClauseView;
use super::super::token_primitives::iter_contains;
use super::super::util::{
    is_article, parse_card_type, parse_subject, parse_target_phrase, parse_value,
};
pub(crate) use super::super::util::{strip_leading_articles, trim_commas, trim_edge_punctuation};
use super::sentence_helpers::*;
use super::{
    TokenCopyFollowup, parse_cant_effect_sentence_lexed, parse_effect_chain_lexed,
    parse_reveal_source_exiled_permanents_sentence_lexed, parse_search_library_sentence_lexed,
};
use crate::cards::builders::{
    CardTextError, EffectAst, GrantedAbilityAst, IT_TAG, KeywordAction, PlayerAst, SubjectAst,
    SubjectVerbActionAst, SubjectVerbEffectAst, SubjectVerbRoleAst, SubjectVerbSubjectAst, TagKey,
    TargetAst, TextSpan, TriggerSpec,
};
use crate::effect::{ChoiceCount, EventValueSpec, Until, Value};
use crate::object::CounterType;
use crate::target::{
    ChooseSpec, ObjectFilter, PlayerFilter, TaggedObjectConstraint, TaggedOpbjectRelation,
};
use crate::types::CardType;
use crate::zone::Zone;

include!("sentence_shape_predicates.rs");
include!("generic_subject_verb_programs.rs");
include!("labeled_prefixes.rs");
include!("copy_and_next_spell_shapes.rs");
include!("replacement_and_prevention_shapes.rs");
include!("unsupported_shape_diagnostics.rs");

#[cfg(test)]
mod must_be_blocked_composition_tests;
