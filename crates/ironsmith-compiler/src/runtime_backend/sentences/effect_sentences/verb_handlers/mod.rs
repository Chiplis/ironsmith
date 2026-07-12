use crate::cards::builders::{
    CardTextError, ControlDurationAst, EffectAst, EventValueSpec, ExtraTurnAnchorAst, IT_TAG,
    ObjectRefAst, OwnedLexToken, PlayerAst, PredicateAst, ReturnControllerAst, SubjectAst,
    SubjectVerbActionAst, SubjectVerbEffectAst, SubjectVerbRoleAst, SubjectVerbSubjectAst, TagKey,
    TargetAst, TextSpan, Verb,
};
use crate::effect::{ChoiceCount, Until, Value};
use crate::mana::ManaSymbol;
use crate::target::{
    ChooseSpec, ObjectFilter, PlayerFilter, TaggedObjectConstraint, TaggedOpbjectRelation,
};
use crate::types::{CardType, Subtype, Supertype};
use crate::zone::Zone;

use super::super::activation_and_restrictions::parse_devotion_value_from_add_clause;
use super::super::activation_helpers::parse_add_mana;
use super::super::grammar::effects::resource_shapes::{
    self as resource_grammar, ResourceLookHandFollowup, ResourceLookObjectKind, ResourceLookShape,
    ResourceShuffleShape,
};
use super::super::grammar::primitives::{self as grammar, TokenWordView};
use super::super::grammar::structure::{
    parse_trailing_if_predicate_lexed, parse_trailing_instead_if_predicate_lexed,
    parse_who_player_predicate_lexed, split_trailing_if_clause_lexed,
    split_trailing_unless_clause_lexed,
};
use super::super::keyword_static::{
    parse_add_mana_equal_amount_value, parse_dynamic_cost_modifier_value,
    parse_value_binding_clause,
};
use super::super::lexer::{LexedClause, token_slice_at_is, token_slice_first_is};
use super::super::object_filters::parse_object_filter;
use super::super::util::{
    mana_pips_from_token, parse_choice_count_before_target_prefix, parse_mana_symbol, parse_number,
    parse_target_phrase, parse_value, replace_unbound_x_with_value, span_from_tokens, trim_commas,
    value_contains_unbound_x, wrap_target_count,
};
use super::clause_pattern_helpers::extract_subject_player;
use super::creation_handlers::{parse_create, parse_incubate, parse_investigate};
use super::for_each_helpers::parse_who_did_this_way_predicate;
use super::subject_verb_primitives::{SubjectVerbPrimitiveClause, try_build_unless};
use super::zone_counter_helpers::{parse_convert, parse_put_counters, parse_transform};
use super::zone_handlers::{
    DelayedReturnTimingAst, parse_become, parse_delayed_return_timing_words, parse_destroy,
    parse_discard, parse_end, parse_exchange, parse_exile, parse_flip, parse_get,
    parse_graveyard_owner_prefix_lexed, parse_mill, parse_pay, parse_regenerate, parse_remove,
    parse_return, parse_roll, parse_sacrifice, parse_scry, parse_skip, parse_surveil, parse_switch,
    parse_tap, parse_untap, wrap_return_with_delayed_timing,
};
use crate::runtime_backend::grammar::shared_util::value_semantics::{
    parse_equal_to_aggregate_filter_value, parse_equal_to_number_of_counters_on_reference_value,
    parse_equal_to_number_of_filter_plus_or_minus_fixed_value,
    parse_equal_to_number_of_filter_value, parse_equal_to_number_of_opponents_you_have_value,
};
include!("resource_verbs.rs");
include!("combat_verbs.rs");
include!("zone_move_verbs.rs");
include!("counter_stat_verbs.rs");
include!("control_copy_attach_verbs.rs");
