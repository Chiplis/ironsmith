use super::super::activation_and_restrictions::choice_object_clauses::{
    parse_target_player_choose_objects_clause, parse_you_choose_objects_clause,
};
use super::super::grammar::effects::parse_conditional_sentence_with_grammar_entrypoint_lexed;
use super::super::grammar::primitives::TokenWordView;
use super::super::keyword_static::parse_value_binding_clause;
use super::super::lexer::{LexedClause, OwnedLexToken};
use super::super::object_filters::parse_object_filter;
use super::super::rule_engine::{
    LexClauseView, LexRuleDef, LexRuleHandler, LexRuleHeadHint, LexRuleHintIndex, LexRuleIndex,
    build_lex_rule_hint_index,
};
use super::super::token_primitives::iter_contains;
use super::super::util::{
    parse_card_type, parse_choice_count_before_target_prefix, parse_color,
    parse_counter_type_from_tokens, parse_subject, parse_subtype_flexible,
};
use super::super::util::{parse_target_phrase, parse_value, span_from_tokens};
use super::dispatch_inner::merge_filters;
use super::search_library::parse_restriction_duration;
use super::sentence_helpers::*;
use super::verb_handlers::parse_half_rounded_down_draw_count_words;
use super::zone_counter_helpers::parse_convert;
use super::{
    bind_implicit_player_context, parse_cant_effect_sentence,
    parse_compound_damage_fanout_sentence, parse_delayed_until_next_end_step_sentence,
    parse_destroy_or_exile_all_split_sentence,
    parse_each_player_put_permanent_cards_exiled_with_source_sentence, parse_effect_chain,
    parse_effect_chain_inner, parse_effect_chain_lexed, parse_effect_clause,
    parse_effect_sentence_lexed, parse_exile_hand_and_graveyard_bundle_sentence,
    parse_exile_up_to_one_each_target_type_sentence, parse_for_each_destroyed_this_way_sentence,
    parse_for_each_exiled_this_way_sentence, parse_for_each_player_doesnt,
    parse_for_each_put_into_graveyard_this_way_sentence, parse_gain_ability_sentence,
    parse_gain_life_equal_to_age_sentence, parse_gain_x_plus_life_sentence,
    parse_look_at_hand_sentence, parse_look_at_top_then_exile_one_sentence,
    parse_same_name_gets_fanout_sentence, parse_same_name_target_fanout_sentence,
    parse_search_library_sentence, parse_serial_target_pt_modifiers_sentence,
    parse_shared_color_target_fanout_sentence, parse_shuffle_graveyard_into_library_sentence,
    parse_shuffle_object_into_library_sentence,
    parse_target_player_exiles_creature_and_graveyard_sentence,
};
use crate::cards::builders::{
    CardTextError, ClashOpponentAst, EffectAst, IT_TAG, IfResultPredicate, PlayerAst, PredicateAst,
    ReturnControllerAst, SubjectAst, SubjectVerbActionAst, SubjectVerbEffectAst,
    SubjectVerbRoleAst, TagKey, TargetAst, TextSpan,
};
use crate::effect::{ChoiceCount, Until, Value};
use crate::recognition::RuleId;
use crate::target::{ObjectFilter, PlayerFilter, TaggedObjectConstraint, TaggedOpbjectRelation};
use crate::types::{CardType, Subtype};
use crate::zone::Zone;
use std::cell::OnceCell;
use std::sync::LazyLock;

pub fn parse_sentence_put_multiple_counters_on_target(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    clause.parse_with_lexed(
        super::zone_counter_helpers::parse_sentence_put_multiple_counters_on_target,
    )
}

pub fn parse_sentence_target_player_chooses_then_puts_on_top_of_library(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    clause.parse_with_lexed(
        super::super::activation_and_restrictions::parse_sentence_target_player_chooses_then_puts_on_top_of_library,
    )
}

pub fn parse_sentence_target_player_chooses_then_you_put_it_onto_battlefield(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    clause.parse_with_lexed(
        super::super::activation_and_restrictions::parse_sentence_target_player_chooses_then_you_put_it_onto_battlefield,
    )
}

pub fn parse_sentence_counter_target_spell_if_it_was_kicked(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    clause
        .parse_with_lexed(super::conditionals::parse_sentence_counter_target_spell_if_it_was_kicked)
}

pub fn parse_sentence_counter_target_spell_thats_second_cast_this_turn(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    clause.parse_with_lexed(
        super::conditionals::parse_sentence_counter_target_spell_thats_second_cast_this_turn,
    )
}

pub fn parse_sentence_exile_target_creature_with_greatest_power(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    clause.parse_with_lexed(
        super::conditionals::parse_sentence_exile_target_creature_with_greatest_power,
    )
}

pub fn parse_delayed_when_that_dies_this_turn_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    clause.parse_with_lexed(super::dispatch_inner::parse_delayed_when_that_dies_this_turn_sentence)
}

pub fn parse_delayed_when_that_leaves_battlefield_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    clause.parse_with_lexed(
        super::dispatch_inner::parse_delayed_when_that_leaves_battlefield_sentence,
    )
}

pub fn parse_sentence_delayed_trigger_this_turn(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    clause.parse_with_lexed(super::dispatch_inner::parse_sentence_delayed_trigger_this_turn)
}

pub fn parse_if_any_tagged_cards_share_card_type_with_triggering_spell(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = super::super::grammar::effects::clause_dispatch_shapes::parse_tagged_shares_card_type_condition_tokens(
        clause.tokens(),
    ) else {
        return Ok(None);
    };
    if shape.effect_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing effect after revealed-card card-type condition (clause: '{}')",
            clause.text()
        )));
    }

    let mut filter = ObjectFilter::default();
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: crate::tag::CompilerReferenceTag::Triggering.key(),
        relation: TaggedOpbjectRelation::SharesCardType,
    });
    let if_true = parse_effect_chain_inner(shape.effect_tokens)?;

    Ok(Some(vec![EffectAst::Conditional {
        predicate: PredicateAst::TaggedMatches(TagKey::from(IT_TAG), filter),
        if_true,
        if_false: Vec::new(),
    }]))
}

#[path = "choice_damage_family.rs"]
mod choice_damage_family;
mod combat_and_damage_family;
mod counter_marker_family;
mod delayed_step_family;
mod mechanic_marker_family;
mod registry;
mod token_copy_control_family;

pub use choice_damage_family::*;
pub use combat_and_damage_family::*;
pub use counter_marker_family::*;
pub use delayed_step_family::*;
pub use mechanic_marker_family::*;
pub use registry::*;
pub use token_copy_control_family::*;
