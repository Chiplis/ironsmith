use winnow::Parser;
use winnow::combinator::{alt, repeat};
use winnow::error::{ContextError, ErrMode};

use super::super::compile_support::effects_reference_it_tag;
use super::super::effect_ast_traversal::for_each_nested_effects_mut;
use super::super::grammar::effects::{
    chain_carry as chain_grammar, for_each_shapes, parse_additional_phases_shape,
    parse_any_player_may_sacrifice_shape, parse_choose_then_exile_reference_shape,
    parse_conditional_sentence_family_lexed, parse_exile_reference_action_shape,
    parse_reveal_source_exiled_permanents_tokens, parse_tap_object_union_then_tokens,
    sacrifice_discard_shapes as sacrifice_discard_grammar,
};
use super::super::grammar::primitives::{self as grammar, TokenWordView};
use super::super::grammar::structure::{
    LeadingResultPrefixKind, split_leading_result_prefix_lexed, split_trailing_if_clause_lexed,
};
use super::super::lexer::{OwnedLexToken, TokenKind, token_word_refs, trim_lexed_commas};
use super::super::object_filters::parse_object_filter;
use super::super::permission_helpers::{
    PermissionClauseSpec, PermissionLifetime, parse_additional_land_plays_clause_lexed,
    parse_cast_or_play_tagged_clause, parse_permission_clause_spec_lexed,
    parse_unsupported_play_cast_permission_clause_lexed,
};
use super::super::rule_engine::{LexClauseView, LexRuleDef, LexRuleIndex};
use super::super::span_from_tokens;
#[cfg(test)]
use super::super::token_primitives::str_contains as string_contains;
use super::super::util::{
    parse_target_phrase, remove_first_may_word as remove_first_may_word_tokens,
    remove_through_first_may_word as remove_through_first_may_word_tokens,
};
use super::dispatch_inner::parse_subject_verb_extension_sentence;
use super::lex_chain_helpers::{
    find_verb_lexed, has_effect_head_without_verb_lexed, segment_has_effect_head_lexed,
    split_effect_chain_on_and_lexed, split_segments_on_comma_effect_head_lexed,
    split_segments_on_comma_then_lexed, strip_leading_instead_prefix_lexed,
};
#[cfg(test)]
use super::parse_effect_sentence_lexed;
use super::search_library::parse_for_each_exiled_this_way_sentence;
use super::sentence_helpers::*;
use super::{
    SubjectVerbPrimitiveClause, parse_cant_effect_sentence_lexed, parse_effect_clause_lexed,
    parse_search_library_sentence_lexed, parse_sentence_exile_source_with_counters_lexed,
    parse_sentence_put_onto_battlefield_with_counters_on_it_lexed,
    parse_sentence_return_with_counters_on_it_lexed, parse_sentence_unless_pays,
    parse_simple_gain_ability_clause_lexed, parse_simple_lose_ability_clause_lexed,
    parse_token_copy_followup_sentence_lexed, try_apply_token_copy_followup,
};
use crate::runtime_backend::grammar::shared_util::value_semantics::{
    parse_number_prefix_lexed, parse_value_prefix_lexed,
};

const ENCHANTED_TAG_NAME: &str = "enchanted";
const SENTENCE_HELPER_REVEALED_TAG_PREFIX: &str = "__sentence_helper_revealed";
use crate::cards::builders::{
    CardTextError, EffectAst, IT_TAG, PlayerAst, PredicateAst, ReturnControllerAst,
    SubjectVerbActionAst, SubjectVerbEffectAst, SubjectVerbSubjectAst, TagKey, TargetAst, TextSpan,
};
use crate::effect::{ChoiceCount, Until, Value};
use crate::target::{ObjectFilter, PlayerFilter, TaggedOpbjectRelation};
use crate::types::{CardType, Subtype};
use crate::zone::Zone;

/// Whether a clause begins with "have/has <explicit player> …" — a causative
/// where the player after "have" is the subject ("have that player lose 2 life").
/// In that case the "have" must NOT be stripped, or the explicit player subject
/// is lost and the effect wrongly binds to the may-clause's player. The noun
/// must be a player ("that player", "each opponent") — not an object such as
/// "that creature", which is an ordinary causative the limited parser handles.
fn leading_have_introduces_causative_player(tokens: &[OwnedLexToken]) -> bool {
    chain_grammar::is_causative_have_player_tokens(tokens)
}

fn synthetic_lexed_word(word: &str) -> OwnedLexToken {
    OwnedLexToken::word(word, TextSpan::synthetic())
}

fn parse_choose_land_of_each_basic_land_type_segment(
    tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    if !chain_grammar::parse_choose_each_basic_land_type_tokens(tokens) {
        return None;
    }

    let basic_land_types = [
        Subtype::Plains,
        Subtype::Island,
        Subtype::Swamp,
        Subtype::Mountain,
        Subtype::Forest,
    ];
    Some(
        basic_land_types
            .into_iter()
            .map(|subtype| {
                let mut filter = ObjectFilter::land().with_subtype(subtype);
                filter.controller = Some(PlayerFilter::Any);
                EffectAst::ChooseObjects {
                    filter,
                    count: ChoiceCount::exactly(1),
                    count_value: None,
                    player: PlayerAst::Implicit,
                    tag: TagKey::from(crate::cards::builders::IT_TAG),
                }
            })
            .collect(),
    )
}

fn rest_action_effect(
    action: chain_grammar::RestActionShape,
    filter: ObjectFilter,
    player: PlayerAst,
) -> EffectAst {
    match action {
        chain_grammar::RestActionShape::Destroy => EffectAst::subject_verb_destroy_all(filter),
        chain_grammar::RestActionShape::Exile => EffectAst::subject_verb_exile_all(filter, false),
        chain_grammar::RestActionShape::Sacrifice => {
            EffectAst::subject_verb_sacrifice_all(player, filter)
        }
    }
}

fn try_apply_rest_action_followup(
    effects: &mut Vec<EffectAst>,
    action: chain_grammar::RestActionShape,
) -> bool {
    if let Some(EffectAst::ChooseObjects {
        filter,
        tag,
        player,
        ..
    }) = effects.last()
    {
        let rest_filter = filter.clone().not_tagged(tag.clone());
        let player = *player;
        effects.push(rest_action_effect(action, rest_filter, player));
        return true;
    }

    let Some(last) = effects.last_mut() else {
        return false;
    };
    match last {
        EffectAst::ForEachPlayer {
            effects: inner_effects,
        }
        | EffectAst::ForEachOpponent {
            effects: inner_effects,
        } => {
            let Some(EffectAst::ChooseObjects {
                filter,
                tag,
                player,
                ..
            }) = inner_effects.last()
            else {
                return false;
            };
            let rest_filter = filter.clone().not_tagged(tag.clone());
            let player = *player;
            inner_effects.push(rest_action_effect(action, rest_filter, player));
            true
        }
        _ => false,
    }
}

fn starts_like_create_fragment_lexed(tokens: &[OwnedLexToken]) -> bool {
    chain_grammar::parse_create_fragment_tokens(tokens)
}

fn is_for_each_counter_group_removed_this_way_prefix_lexed(tokens: &[OwnedLexToken]) -> bool {
    super::super::grammar::effects::clause_dispatch_shapes::parse_counter_group_removed_shape(
        tokens,
    )
    .is_some_and(|shape| shape.effect_tokens.is_empty())
}

fn merge_for_each_counter_group_segments_lexed(
    segments: Vec<Vec<OwnedLexToken>>,
) -> Vec<Vec<OwnedLexToken>> {
    let mut merged = Vec::new();
    let mut iter = segments.into_iter().peekable();
    while let Some(mut segment) = iter.next() {
        if is_for_each_counter_group_removed_this_way_prefix_lexed(&segment)
            && let Some(next) = iter.next()
        {
            segment.extend(next);
        }
        merged.push(segment);
    }
    merged
}

pub(super) fn parse_effect_chain_rule_lexed(
    view: &LexClauseView<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_effect_chain_lexed(view.tokens).map(Some)
}

pub(super) const FALLBACK_POST_DIAGNOSTIC_RULES_LEXED: [LexRuleDef<Vec<EffectAst>>; 1] =
    [LexRuleDef {
        id: "effect-chain",
        priority: 170,
        heads: &[],
        shape_mask: 0,
        run: parse_effect_chain_rule_lexed,
    }];

pub(super) const FALLBACK_POST_DIAGNOSTIC_INDEX_LEXED: LexRuleIndex<Vec<EffectAst>> =
    LexRuleIndex::new(&FALLBACK_POST_DIAGNOSTIC_RULES_LEXED);

fn parse_exile_library_then_shuffle_graveyard_chain_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(spec) = chain_grammar::parse_exile_library_shuffle_tokens(tokens) else {
        return Ok(None);
    };
    let (owner_filter, owner_player) = match spec.owner {
        chain_grammar::ChainOwner::You => (PlayerFilter::You, PlayerAst::You),
        chain_grammar::ChainOwner::TargetPlayer => {
            (PlayerFilter::target_player(), PlayerAst::Target)
        }
        chain_grammar::ChainOwner::TargetOpponent => {
            (PlayerFilter::target_opponent(), PlayerAst::TargetOpponent)
        }
    };

    let mut filter = crate::target::ObjectFilter::default().in_zone(Zone::Library);
    filter.owner = Some(owner_filter);
    Ok(Some(vec![
        EffectAst::subject_verb_exile_all(filter, true),
        EffectAst::subject_verb_shuffle_graveyard_into_library(owner_player),
    ]))
}

pub(crate) fn looks_like_multi_create_chain_lexed(tokens: &[OwnedLexToken]) -> bool {
    matches!(find_verb_lexed(tokens), Some((Verb::Create, _)))
        && chain_grammar::count_token_mentions(tokens) >= 2
}

pub(crate) fn parse_reveal_source_exiled_permanents_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    parse_reveal_source_exiled_permanents_tokens(tokens)?;
    let mut source_exiled =
        ObjectFilter::tagged(crate::tag::SOURCE_EXILED_TAG).in_zone(Zone::Exile);
    source_exiled.owner = Some(PlayerFilter::IteratedPlayer);
    let reveal = EffectAst::subject_verb(
        crate::cards::builders::SubjectVerbRoleAst::Actor,
        PlayerAst::That,
        SubjectVerbActionAst::TurnFaceUp {
            target: TargetAst::Object(source_exiled.clone(), None, None),
        },
    );

    let mut permanents = source_exiled;
    permanents.card_types = vec![
        CardType::Artifact,
        CardType::Creature,
        CardType::Enchantment,
        CardType::Land,
        CardType::Planeswalker,
        CardType::Battle,
    ];
    let put_onto_battlefield = EffectAst::subject_verb_put_all_onto_battlefield(
        permanents,
        false,
        false,
        ReturnControllerAst::Owner,
    );
    Some(vec![EffectAst::ForEachPlayer {
        effects: vec![reveal, put_onto_battlefield],
    }])
}

pub(crate) fn parse_effect_chain_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    fn immediate_tagged_permission_spec(tokens: &[OwnedLexToken]) -> Result<bool, CardTextError> {
        Ok(matches!(
            parse_permission_clause_spec_lexed(tokens)?,
            Some(PermissionClauseSpec::Tagged {
                lifetime: PermissionLifetime::Immediate,
                ..
            })
        ))
    }

    if let Some(effects) = parse_reveal_source_exiled_permanents_sentence_lexed(tokens) {
        return Ok(effects);
    }

    if let Some(effects) = parse_for_each_exiled_this_way_sentence(tokens)? {
        return Ok(effects);
    }

    if let Some(shape) = for_each_shapes::parse_for_each_object_effect_shape(tokens) {
        let mut count_words = vec!["for", "each"];
        count_words.extend(crate::runtime_backend::token_word_refs(shape.filter_tokens));
        if let Some((count, used)) =
            crate::runtime_backend::util::parse_for_each_count_value_words(&count_words)
            && used == count_words.len()
            && !matches!(count.unhinted(), Value::Count(_))
        {
            let effects = parse_effect_chain_lexed(shape.effect_tokens)?;
            if effects.is_empty() {
                return Err(CardTextError::ParseError(
                    "for-each scalar sentence missing effect payload".to_string(),
                ));
            }
            return Ok(vec![EffectAst::RepeatEffects {
                count: count.with_surface_hint(ironsmith_core::ValueSurfaceHint::ForEach),
                effects,
            }]);
        }
        let filter = parse_object_filter(shape.filter_tokens, false)?;
        let effects = parse_effect_chain_lexed(shape.effect_tokens)?;
        if effects.is_empty() {
            return Err(CardTextError::ParseError(
                "for-each object sentence missing effect payload".to_string(),
            ));
        }
        return Ok(vec![EffectAst::ForEachObject { filter, effects }]);
    }

    // A phase-insertion clause has no ordinary subject/verb head ("there is
    // an additional combat phase").  Conditional and labeled effect bodies
    // enter through the chain parser, so route the already-typed phase shape
    // before generic verb discovery as well as at the sentence entrypoint.
    if let Some(shape) = parse_additional_phases_shape(tokens) {
        return Ok(vec![EffectAst::subject_verb_additional_phases(
            shape.phases,
        )]);
    }

    // Preserve coordinated object operands as one tap result so a subsequent
    // "them" refers to the entire affected set, not only the first operand.
    if let Some(shape) = parse_tap_object_union_then_tokens(tokens) {
        let first = parse_target_phrase(shape.first_target_tokens)?;
        let first_filter = match first {
            TargetAst::Source(_) => ObjectFilter::source(),
            TargetAst::Object(filter, None, _) => filter,
            TargetAst::Tagged(tag, _) => ObjectFilter::tagged(tag),
            _ => {
                return Err(CardTextError::ParseError(
                    "coordinated tap operand must be a non-target object reference".to_string(),
                ));
            }
        };
        let all_filter = parse_object_filter(shape.all_filter_tokens, false)?;
        let mut union = ObjectFilter::default();
        union.any_of = vec![first_filter, all_filter];
        let mut effects = vec![EffectAst::subject_verb_tap_all(union)];
        effects.extend(parse_effect_chain_lexed(shape.followup_tokens)?);
        return Ok(effects);
    }

    if let Some(effects) = parse_destroy_then_temporary_cant_attack_block_chain_lexed(tokens)? {
        return Ok(effects);
    }
    if let Some(effects) = parse_exile_library_then_shuffle_graveyard_chain_lexed(tokens)? {
        return Ok(effects);
    }
    if let Some(shape) =
        sacrifice_discard_grammar::parse_each_player_may_discard_hand_and_draw_tokens(tokens)
    {
        let optional_effects = vec![
            EffectAst::subject_verb_discard_hand(PlayerAst::That),
            EffectAst::subject_verb(
                crate::cards::builders::SubjectVerbRoleAst::AffectedPlayer,
                PlayerAst::That,
                SubjectVerbActionAst::Draw {
                    count: shape.draw_count,
                },
            ),
        ];
        return Ok(vec![EffectAst::ForEachPlayer {
            effects: vec![EffectAst::MayByPlayer {
                player: PlayerAst::That,
                effects: optional_effects,
            }],
        }]);
    }
    if let Some(effects) =
        super::dispatch_inner::parse_generic_top_cards_put_counted_into_hand_rest_graveyard_subject_verb(tokens)
    {
        return Ok(effects);
    }

    if let Some(result_tokens) = chain_grammar::parse_meld_them_into_tokens(tokens) {
        let result_words = token_word_refs(result_tokens);
        if result_words.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing meld result name (clause: '{}')",
                token_word_refs(tokens).join(" ")
            )));
        }
        return Ok(vec![EffectAst::subject_verb_meld(
            result_words.join(" "),
            false,
            false,
        )]);
    }

    if let Some(stripped) = strip_leading_instead_prefix_lexed(tokens) {
        return parse_effect_chain_lexed(stripped);
    }
    let leading_scope = chain_grammar::parse_leading_chain_scope_tokens(tokens);
    let starts_with_each_opponent =
        leading_scope == Some(chain_grammar::ChainPlayerScope::EachOpponent);
    let starts_with_each_player =
        leading_scope == Some(chain_grammar::ChainPlayerScope::EachPlayer);

    if let Some(shape) = parse_any_player_may_sacrifice_shape(tokens) {
        let sacrifice = super::zone_handlers::parse_sacrifice(
            shape.action_tokens,
            Some(crate::cards::builders::SubjectAst::Player(PlayerAst::That)),
            None,
        )?;
        return Ok(vec![EffectAst::AnyPlayerMay {
            effects: vec![sacrifice],
        }]);
    }

    if let Some(trailing_if) = split_trailing_if_clause_lexed(tokens) {
        if let Some(player) = parse_leading_player_may_lexed(trailing_if.leading_tokens) {
            let mut stripped = remove_through_first_word(trailing_if.leading_tokens);
            if leading_have_introduces_causative_player(&stripped) {
                // keep "have": it introduces a causative on an explicit player
                // ("have that player lose 2 life"); stripping it drops the subject.
            } else if let Some(rest) = chain_grammar::strip_leading_have_tokens(&stripped) {
                stripped = rest.to_vec();
            }
            if let Some(rest) = chain_grammar::strip_leading_choose_to_tokens(&stripped) {
                stripped = rest.to_vec();
            }
            let mut effects = parse_effect_chain_lexed(&stripped)?;
            for effect in &mut effects {
                bind_implicit_player_context(effect, player);
            }
            return Ok(vec![EffectAst::Conditional {
                predicate: trailing_if.predicate,
                if_true: vec![EffectAst::MayByPlayer { player, effects }],
                if_false: Vec::new(),
            }]);
        }

        if chain_grammar::starts_with_may_tokens(trailing_if.leading_tokens)
            && !starts_with_each_opponent
            && !starts_with_each_player
        {
            let stripped = remove_first_word(trailing_if.leading_tokens);
            let effects = parse_effect_chain_lexed(&stripped)?;
            return Ok(vec![EffectAst::Conditional {
                predicate: trailing_if.predicate,
                if_true: vec![EffectAst::May { effects }],
                if_false: Vec::new(),
            }]);
        }
    }

    if let Some(player) = parse_leading_player_may_lexed(tokens) {
        let mut stripped = remove_through_first_word(tokens);
        if leading_have_introduces_causative_player(&stripped) {
            // keep "have" — see the trailing-if branch above.
        } else if let Some(rest) = chain_grammar::strip_leading_have_tokens(&stripped) {
            stripped = rest.to_vec();
        }
        if let Some(rest) = chain_grammar::strip_leading_choose_to_tokens(&stripped) {
            stripped = rest.to_vec();
        }
        let mut effects = parse_effect_chain_lexed(&stripped)?;
        for effect in &mut effects {
            bind_implicit_player_context(effect, player);
        }
        if leading_may_is_permission_clause_lexed(&stripped)? {
            if immediate_tagged_permission_spec(&stripped)? {
                return Ok(vec![EffectAst::MayByPlayer { player, effects }]);
            }
            return Ok(effects);
        }
        return Ok(vec![EffectAst::MayByPlayer { player, effects }]);
    }

    if chain_grammar::starts_with_may_tokens(tokens)
        && !starts_with_each_opponent
        && !starts_with_each_player
    {
        let stripped = remove_first_word(tokens);
        let effects = parse_effect_chain_lexed(&stripped)?;
        if leading_may_is_permission_clause_lexed(&stripped)? {
            if immediate_tagged_permission_spec(&stripped)? {
                return Ok(vec![EffectAst::May { effects }]);
            }
            return Ok(effects);
        }
        return Ok(vec![EffectAst::May { effects }]);
    }

    if chain_grammar::parse_tap_or_untap_all_choice_tokens(tokens) {
        return parse_effect_chain_with_subject_verb_primitives_lexed(tokens);
    }

    if let Some(unless_action) = parse_or_action_clause_lexed(tokens)? {
        return Ok(vec![unless_action]);
    }

    if clause_may_contain_cast_or_play_permission_lexed(tokens)
        && let Some(effect) = parse_cast_or_play_tagged_clause(tokens)?
    {
        return Ok(vec![effect]);
    }

    parse_effect_chain_with_subject_verb_primitives_lexed(tokens)
}

pub(crate) fn parse_destroy_then_temporary_cant_attack_block_chain_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    for split in chain_grammar::parse_destroy_restriction_splits_tokens(tokens) {
        let mut effects = vec![parse_effect_clause_lexed(split.destroy_tokens)?];
        let Some(tail_effects) = parse_cant_effect_sentence_lexed(split.restriction_tokens)? else {
            return Err(CardTextError::ParseError(format!(
                "unsupported destroy plus attack/block restriction tail (clause: '{}')",
                token_word_refs(split.restriction_tokens).join(" ")
            )));
        };
        effects.extend(tail_effects);
        return Ok(Some(effects));
    }
    Ok(None)
}

fn clause_may_contain_cast_or_play_permission_lexed(tokens: &[OwnedLexToken]) -> bool {
    tokens
        .iter()
        .filter_map(OwnedLexToken::as_word)
        .any(|word| {
            matches!(
                word,
                "may" | "cast" | "casts" | "casting" | "play" | "plays" | "playing" | "played"
            )
        })
}

fn leading_may_is_permission_clause_lexed(tokens: &[OwnedLexToken]) -> Result<bool, CardTextError> {
    Ok(parse_additional_land_plays_clause_lexed(tokens)?.is_some()
        || parse_permission_clause_spec_lexed(tokens)?.is_some()
        || parse_unsupported_play_cast_permission_clause_lexed(tokens)?.is_some())
}

fn starts_with_until_end_of_turn_trigger_clause(tokens: &[OwnedLexToken]) -> bool {
    chain_grammar::parse_until_end_of_turn_trigger_tokens(tokens)
}

fn is_would_enter_replacement_clause(tokens: &[OwnedLexToken]) -> bool {
    chain_grammar::parse_would_enter_replacement_tokens(tokens)
}

pub(crate) fn parse_or_action_clause_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    if chain_grammar::parse_tap_or_untap_all_choice_tokens(tokens) {
        return Ok(None);
    }

    for split in chain_grammar::parse_or_action_splits_tokens(tokens) {
        let first = split.first_tokens;
        let second = split.second_tokens;

        let first_starts_effect = find_verb_lexed(first).is_some_and(|(_, verb_idx)| verb_idx == 0)
            || has_effect_head_without_verb_lexed(first);
        let second_starts_effect = find_verb_lexed(second)
            .is_some_and(|(_, verb_idx)| verb_idx == 0)
            || has_effect_head_without_verb_lexed(second);
        if !first_starts_effect || !second_starts_effect {
            continue;
        }

        let first_effects = match parse_effect_chain_with_subject_verb_primitives_lexed(first) {
            Ok(effects) if !effects.is_empty() => effects,
            _ => continue,
        };
        let second_effects = match parse_effect_chain_with_subject_verb_primitives_lexed(second) {
            Ok(effects) if !effects.is_empty() => effects,
            _ => continue,
        };

        return Ok(Some(EffectAst::UnlessAction {
            effects: first_effects,
            alternative: second_effects,
            player: PlayerAst::Implicit,
        }));
    }

    Ok(None)
}

#[cfg(test)]
#[path = "chain_carry/tests.rs"]
mod tests;

pub(crate) fn parse_effect_chain_with_subject_verb_primitives_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    if let Some(rest) = chain_grammar::strip_leading_and_tokens(tokens) {
        return parse_effect_chain_with_subject_verb_primitives_lexed(rest);
    }

    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if starts_with_until_end_of_turn_trigger_clause(tokens) {
        return Err(CardTextError::ParseError(format!(
            "unsupported until-end-of-turn permission clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    if is_would_enter_replacement_clause(tokens) {
        return Err(CardTextError::ParseError(format!(
            "unsupported would-enter replacement clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    if let Some(effects) = parse_return_it_then_loses_all_abilities_lexed(tokens)? {
        return Ok(effects);
    }

    if let Some(effects) = run_subject_verb_primitives_lexed(
        tokens,
        PRE_CONDITIONAL_SUBJECT_VERB_PRIMITIVES,
        &PRE_CONDITIONAL_SUBJECT_VERB_PRIMITIVE_INDEX,
    )? {
        return Ok(effects);
    }
    if let Some(effects) = parse_subject_verb_extension_sentence(tokens)? {
        return Ok(effects);
    }
    if chain_grammar::starts_with_unless_tokens(tokens)
        && let Some(effects) = parse_sentence_unless_pays(SubjectVerbPrimitiveClause::new(tokens))?
    {
        return Ok(effects);
    }
    if let Some(effects) =
        parse_conditional_sentence_family_lexed(tokens, parse_effect_chain_lexed)?
    {
        return Ok(effects);
    }
    if let Some(effects) = run_subject_verb_primitives_lexed(
        tokens,
        POST_CONDITIONAL_SUBJECT_VERB_PRIMITIVES,
        &POST_CONDITIONAL_SUBJECT_VERB_PRIMITIVE_INDEX,
    )? {
        let mut effects = effects;
        append_missing_coordinated_return_discard_tail(tokens, &mut effects)?;
        return Ok(effects);
    }
    parse_effect_chain_inner_lexed(tokens)
}

pub(crate) fn append_missing_coordinated_return_discard_tail(
    tokens: &[OwnedLexToken],
    effects: &mut Vec<EffectAst>,
) -> Result<(), CardTextError> {
    if !matches!(
        chain_grammar::coordinated_target_action_kind(tokens),
        Some(chain_grammar::CoordinatedTargetActionKind::Return)
    ) || effects.iter().any(|effect| {
        matches!(
            effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::Discard { .. } | SubjectVerbActionAst::DiscardHand,
                ..
            })
        )
    }) {
        return Ok(());
    }
    if let Some(discard_tokens) = chain_grammar::trailing_then_discard_tokens(tokens) {
        let mut discard_effects = parse_effect_chain_lexed(discard_tokens)?;
        if discard_tokens
            .first()
            .is_some_and(|token| token.is_word("discard"))
        {
            for effect in &mut discard_effects {
                bind_implicit_player_context(effect, PlayerAst::You);
            }
        }
        effects.extend(discard_effects);
    }
    Ok(())
}

pub(crate) fn parse_effect_chain_inner_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    if let Some(stripped) = strip_leading_instead_prefix_lexed(tokens) {
        return parse_effect_chain_inner_lexed(stripped);
    }

    if chain_grammar::starts_with_unless_tokens(tokens)
        && let Some(effects) = parse_sentence_unless_pays(SubjectVerbPrimitiveClause::new(tokens))?
    {
        return Ok(effects);
    }

    if let Some(effects) = parse_search_library_sentence_lexed(tokens)? {
        return Ok(effects);
    }
    if let Some(effects) = parse_tap_those_then_unattach_equipment_lexed(tokens)? {
        return Ok(effects);
    }
    if let Some(effects) = parse_return_it_then_loses_all_abilities_lexed(tokens)? {
        return Ok(effects);
    }
    if let Some(clauses) =
        super::player_subject_sequences::split_quantified_opponent_then_controller_clauses(tokens)
    {
        let mut effects = Vec::new();
        for clause in clauses {
            effects.extend(parse_effect_chain_inner_lexed(clause)?);
        }
        return Ok(effects);
    }

    let choose_then_exile_reference = parse_choose_then_exile_reference_shape(tokens).is_some();
    let mut effects = Vec::new();
    let raw_segments = split_effect_chain_on_and_lexed(tokens);
    let mut lexed_segments = Vec::new();
    for segment in raw_segments {
        if segment.is_empty() {
            continue;
        }
        lexed_segments.push(segment);
    }

    let mut merged_lexed_segments: Vec<Vec<OwnedLexToken>> = Vec::new();
    for lexed_segment in lexed_segments {
        let segment = lexed_segment.to_vec();
        if merged_lexed_segments.is_empty() {
            merged_lexed_segments.push(segment);
            continue;
        }
        if !super::lex_chain_helpers::segment_has_effect_head_lexed(&segment) {
            if let Some(previous) = merged_lexed_segments.last()
                && let Some(expanded) = expand_missing_verb_segment_lexed(previous, &segment)
            {
                merged_lexed_segments.push(expanded);
                continue;
            }
            let last = merged_lexed_segments
                .last_mut()
                .expect("non-empty segments");
            last.push(synthetic_lexed_word("and"));
            last.extend(segment);
            continue;
        }
        merged_lexed_segments.push(segment);
    }
    while merged_lexed_segments.len() > 1
        && !super::lex_chain_helpers::segment_has_effect_head_lexed(&merged_lexed_segments[0])
    {
        let mut first = merged_lexed_segments.remove(0);
        first.push(synthetic_lexed_word("and"));
        let mut next = merged_lexed_segments.remove(0);
        first.append(&mut next);
        merged_lexed_segments.insert(0, first);
    }
    let merged_segment_slices = merged_lexed_segments
        .iter()
        .map(Vec::as_slice)
        .collect::<Vec<_>>();
    let mut segments: Vec<Vec<OwnedLexToken>> = split_segments_on_comma_effect_head_lexed(
        split_segments_on_comma_then_lexed(merged_segment_slices),
    )
    .into_iter()
    .map(|segment| segment.to_vec())
    .collect();
    segments = expand_segments_with_comma_action_clauses_lexed(segments);
    segments = expand_segments_with_multi_create_clauses_lexed(segments);
    segments = merge_for_each_counter_group_segments_lexed(segments);
    let mut carried_context: Option<CarryContext> = None;
    let leading_duration = leading_duration_for_followup_carry(tokens);
    let mut carried_duration: Option<Until> = leading_duration.clone();
    let mut previous_segment: Option<Vec<OwnedLexToken>> = None;
    for segment in segments {
        let mut segment = segment;
        let bind_source_exiled =
            choose_then_exile_reference && parse_exile_reference_action_shape(&segment).is_some();
        if is_orphan_rounded_up_where_x_tail(&segment, previous_segment.as_deref(), effects.last())
        {
            continue;
        }
        if let Some(previous) = &previous_segment
            && let Some(expanded) = expand_gain_lose_followup_segment_lexed(previous, &segment)
        {
            segment = expanded;
        }

        let carry_gain_duration = find_verb_lexed(&segment).is_some_and(|(verb, verb_idx)| {
            verb_idx == 0 && matches!(verb, Verb::Gain | Verb::Lose)
        });
        let carry_leading_duration = leading_duration.is_some();
        let segment_effects =
            if let Some(effects) = parse_sentence_return_with_counters_on_it_lexed(&segment)? {
                Some(effects)
            } else if let Some(effects) =
                parse_sentence_put_onto_battlefield_with_counters_on_it_lexed(&segment)?
            {
                Some(effects)
            } else if let Some(prefix) = split_leading_result_prefix_lexed(&segment) {
                Some(vec![match prefix.kind {
                    LeadingResultPrefixKind::If => EffectAst::IfResult {
                        predicate: prefix.predicate,
                        effects: parse_effect_chain_inner_lexed(prefix.trailing_tokens)?,
                    },
                    LeadingResultPrefixKind::When => EffectAst::WhenResult {
                        predicate: prefix.predicate,
                        effects: parse_effect_chain_inner_lexed(prefix.trailing_tokens)?,
                    },
                }])
            } else {
                parse_sentence_exile_source_with_counters_lexed(&segment)?
            };
        if let Some(segment_effects) = segment_effects {
            for mut effect in segment_effects {
                if let Some(context) = carried_context {
                    maybe_apply_carried_player_with_clause_lexed(&mut effect, context, &segment);
                }
                if (carry_gain_duration || carry_leading_duration)
                    && let Some(duration) = &carried_duration
                {
                    apply_carried_effect_duration(&mut effect, duration);
                }
                if let Some(context) = explicit_player_for_carry(&effect) {
                    carried_context = Some(context);
                }
                if let Some(duration) = effect_duration_for_gain_followup_carry(&effect) {
                    carried_duration = Some(duration);
                }
                effects.push(bind_source_exiled_effect(effect, bind_source_exiled));
            }
            continue;
        }
        if let Some(segment_effects) = parse_search_library_sentence_lexed(&segment)? {
            for mut effect in segment_effects {
                if let Some(context) = carried_context {
                    maybe_apply_carried_player_with_clause_lexed(&mut effect, context, &segment);
                }
                if (carry_gain_duration || carry_leading_duration)
                    && let Some(duration) = &carried_duration
                {
                    apply_carried_effect_duration(&mut effect, duration);
                }
                if let Some(context) = explicit_player_for_carry(&effect) {
                    carried_context = Some(context);
                }
                if let Some(duration) = effect_duration_for_gain_followup_carry(&effect) {
                    carried_duration = Some(duration);
                }
                effects.push(bind_source_exiled_effect(effect, bind_source_exiled));
            }
            continue;
        }
        if let Some(segment_effects) = parse_cant_effect_sentence_lexed(&segment)? {
            for mut effect in segment_effects {
                if let Some(context) = carried_context {
                    maybe_apply_carried_player_with_clause_lexed(&mut effect, context, &segment);
                }
                if (carry_gain_duration || carry_leading_duration)
                    && let Some(duration) = &carried_duration
                {
                    apply_carried_effect_duration(&mut effect, duration);
                }
                if let Some(context) = explicit_player_for_carry(&effect) {
                    carried_context = Some(context);
                }
                if let Some(duration) = effect_duration_for_gain_followup_carry(&effect) {
                    carried_duration = Some(duration);
                }
                effects.push(bind_source_exiled_effect(effect, bind_source_exiled));
            }
            continue;
        }
        if let Some(segment_effects) = parse_subject_verb_extension_sentence(&segment)? {
            for mut effect in segment_effects {
                if let Some(context) = carried_context {
                    maybe_apply_carried_player_with_clause_lexed(&mut effect, context, &segment);
                }
                if (carry_gain_duration || carry_leading_duration)
                    && let Some(duration) = &carried_duration
                {
                    apply_carried_effect_duration(&mut effect, duration);
                }
                if let Some(context) = explicit_player_for_carry(&effect) {
                    carried_context = Some(context);
                }
                if let Some(duration) = effect_duration_for_gain_followup_carry(&effect) {
                    carried_duration = Some(duration);
                }
                effects.push(bind_source_exiled_effect(effect, bind_source_exiled));
            }
            previous_segment = Some(segment);
            continue;
        }
        let primitive_segment_effects = if let Some(effects) = run_subject_verb_primitives_lexed(
            &segment,
            PRE_CONDITIONAL_SUBJECT_VERB_PRIMITIVES,
            &PRE_CONDITIONAL_SUBJECT_VERB_PRIMITIVE_INDEX,
        )? {
            Some(effects)
        } else if let Some(effects) =
            parse_conditional_sentence_family_lexed(&segment, parse_effect_chain_lexed)?
        {
            Some(effects)
        } else {
            run_subject_verb_primitives_lexed(
                &segment,
                POST_CONDITIONAL_SUBJECT_VERB_PRIMITIVES,
                &POST_CONDITIONAL_SUBJECT_VERB_PRIMITIVE_INDEX,
            )?
        };
        if let Some(segment_effects) = primitive_segment_effects {
            for mut effect in segment_effects {
                if let Some(context) = carried_context {
                    maybe_apply_carried_player_with_clause_lexed(&mut effect, context, &segment);
                }
                if (carry_gain_duration || carry_leading_duration)
                    && let Some(duration) = &carried_duration
                {
                    apply_carried_effect_duration(&mut effect, duration);
                }
                if let Some(context) = explicit_player_for_carry(&effect) {
                    carried_context = Some(context);
                }
                if let Some(duration) = effect_duration_for_gain_followup_carry(&effect) {
                    carried_duration = Some(duration);
                }
                effects.push(bind_source_exiled_effect(effect, bind_source_exiled));
            }
            previous_segment = Some(segment);
            continue;
        }
        if let Some(followup) = parse_token_copy_followup_sentence_lexed(&segment)
            && try_apply_token_copy_followup(&mut effects, followup)?
        {
            continue;
        }
        if let Some(segment_effects) = parse_choose_land_of_each_basic_land_type_segment(&segment) {
            effects.extend(segment_effects);
            previous_segment = Some(segment);
            continue;
        }
        if let Some(action) = chain_grammar::parse_rest_action_tokens(&segment)
            && try_apply_rest_action_followup(&mut effects, action)
        {
            previous_segment = Some(segment);
            continue;
        }
        if let Some(gain_tail) = chain_grammar::split_all_abilities_and_gain_tokens(&segment) {
            let mut gain_tokens = Vec::new();
            gain_tokens.push(synthetic_lexed_word("it"));
            gain_tokens.extend(gain_tail.iter().cloned());
            if let Some(mut effect) = parse_simple_gain_ability_clause_lexed(&gain_tokens)? {
                if let Some(duration) = &carried_duration {
                    apply_carried_effect_duration(&mut effect, duration);
                }
                effects.push(bind_source_exiled_effect(effect, bind_source_exiled));
                previous_segment = Some(segment);
                continue;
            }
        }
        let mut effect = parse_effect_clause_with_trailing_if_lexed(&segment)?;
        if let Some(context) = carried_context {
            maybe_apply_carried_player_with_clause_lexed(&mut effect, context, &segment);
        }
        if (carry_gain_duration || carry_leading_duration)
            && let Some(duration) = &carried_duration
        {
            apply_carried_effect_duration(&mut effect, duration);
        }
        if let Some(context) = explicit_player_for_carry(&effect) {
            carried_context = Some(context);
        }
        if let Some(duration) = effect_duration_for_gain_followup_carry(&effect) {
            carried_duration = Some(duration);
        }
        effects.push(bind_source_exiled_effect(effect, bind_source_exiled));
        previous_segment = Some(segment);
    }
    collapse_for_each_player_it_tag_followups(&mut effects);
    collapse_for_each_object_it_tag_followups(&mut effects);
    collapse_token_copy_next_end_step_exile_followup_lexed(&mut effects, tokens);
    collapse_token_copy_next_end_step_sacrifice_followup_lexed(&mut effects, tokens);
    collapse_token_copy_end_of_combat_exile_followup_lexed(&mut effects, tokens);
    append_missing_coordinated_return_discard_tail(tokens, &mut effects)?;
    bind_adjacent_discard_count_draws(&mut effects);
    bind_adjacent_implicit_draw_discard_subjects(&mut effects);
    bind_adjacent_life_stat_pronouns(&mut effects, tokens);
    if let Some(kind) = chain_grammar::coordinated_target_action_kind(tokens) {
        wrap_leading_coordinated_target_actions(&mut effects, kind);
    }
    if chain_grammar::coordinated_tap_then_next_untap(tokens)
        && tap_then_next_untap_actions(&effects)
    {
        return Ok(vec![EffectAst::Coordinated {
            effects,
            leading_duration: false,
        }]);
    }
    if chain_grammar::coordinated_source_damage_then_gain(tokens)
        && source_damage_then_gain_ability_actions(&effects)
    {
        return Ok(vec![EffectAst::Coordinated {
            effects,
            leading_duration: false,
        }]);
    }
    if let Some(leading_duration) =
        chain_grammar::coordinated_target_stat_modifier_leading_duration(tokens)
        && effects.len() >= 2
        && effects.iter().all(|effect| {
            matches!(
                effect,
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::Pump { .. },
                    ..
                })
            )
        })
    {
        return Ok(vec![EffectAst::Coordinated {
            effects,
            leading_duration,
        }]);
    }
    Ok(effects)
}

fn tap_then_next_untap_actions(effects: &[EffectAst]) -> bool {
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Tap { .. },
            ..
        }),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Cant {
                    restriction: crate::effect::Restriction::Untap(_),
                    duration: Until::ControllersNextUntapStep,
                    condition: None,
                },
            ..
        }),
    ] = effects
    else {
        return false;
    };
    true
}

fn coordinated_target_action_matches(
    effect: &EffectAst,
    kind: chain_grammar::CoordinatedTargetActionKind,
) -> bool {
    let EffectAst::SubjectVerb(SubjectVerbEffectAst { action, .. }) = effect else {
        return false;
    };
    matches!(
        (kind, action),
        (
            chain_grammar::CoordinatedTargetActionKind::Destroy,
            SubjectVerbActionAst::Destroy { .. }
        ) | (
            chain_grammar::CoordinatedTargetActionKind::Exile,
            SubjectVerbActionAst::Exile { .. }
        ) | (
            chain_grammar::CoordinatedTargetActionKind::Return,
            SubjectVerbActionAst::ReturnToHand { .. }
        )
    )
}

fn wrap_leading_coordinated_target_actions(
    effects: &mut Vec<EffectAst>,
    kind: chain_grammar::CoordinatedTargetActionKind,
) {
    let coordinated_len = effects
        .iter()
        .take_while(|effect| coordinated_target_action_matches(effect, kind))
        .count();
    if coordinated_len < 2 {
        return;
    }
    let remainder = effects.split_off(coordinated_len);
    let coordinated = std::mem::take(effects);
    effects.push(EffectAst::Coordinated {
        effects: coordinated,
        leading_duration: false,
    });
    effects.extend(remainder);
}

fn target_ast_is_source(target: &TargetAst) -> bool {
    match target {
        TargetAst::Source(_) => true,
        TargetAst::Object(filter, _, _) => filter.source,
        TargetAst::WithCount(inner, _) | TargetAst::WithCountValue(inner, _, _) => {
            target_ast_is_source(inner)
        }
        _ => false,
    }
}

fn source_damage_then_gain_ability_actions(effects: &[EffectAst]) -> bool {
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::DealDamageEqualToPower { source, .. },
            ..
        }),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::GrantAbilitiesToTarget { target, .. },
            ..
        }),
    ] = effects
    else {
        return false;
    };
    target_ast_is_source(source) && target_ast_is_source(target)
}

fn bind_adjacent_discard_count_draws(effects: &mut [EffectAst]) {
    fn is_discard(effect: &EffectAst) -> bool {
        matches!(
            effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::Discard { .. },
                ..
            })
        )
    }

    fn bind_draw(effect: &mut EffectAst) {
        let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Draw { count },
            ..
        }) = effect
        else {
            return;
        };
        *count = match count.unhinted() {
            Value::EventValue(crate::effect::EventValueSpec::Amount) => {
                Value::PendingEffectMetric {
                    source: ironsmith_core::EffectMetricSource::Outcome,
                    metric: ironsmith_core::EffectMetric::Count,
                }
            }
            Value::EventValueOffset(crate::effect::EventValueSpec::Amount, offset) => {
                Value::PendingEffectMetricOffset {
                    source: ironsmith_core::EffectMetricSource::Outcome,
                    metric: ironsmith_core::EffectMetric::Count,
                    offset: *offset,
                }
            }
            _ => return,
        };
    }

    for index in 0..effects.len().saturating_sub(1) {
        if is_discard(&effects[index]) {
            bind_draw(&mut effects[index + 1]);
        }
    }
}

fn bind_adjacent_implicit_draw_discard_subjects(effects: &mut [EffectAst]) {
    for index in 0..effects.len().saturating_sub(1) {
        let draw_is_implicit = matches!(
            &effects[index],
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                subject,
                action: SubjectVerbActionAst::Draw { .. },
            }) if subject.player == PlayerAst::Implicit
        );
        if !draw_is_implicit {
            continue;
        }
        if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            subject,
            action: SubjectVerbActionAst::Discard { .. },
        }) = &mut effects[index + 1]
            && subject.player == PlayerAst::Implicit
        {
            subject.player = PlayerAst::You;
        }
    }
}

fn bind_adjacent_life_stat_pronouns(effects: &mut [EffectAst], tokens: &[OwnedLexToken]) {
    let words = token_word_refs(tokens);
    if !words.windows(2).any(|pair| {
        pair[0].eq_ignore_ascii_case("its")
            && matches!(pair[1].to_ascii_lowercase().as_str(), "power" | "toughness")
    }) {
        return;
    }

    fn tagged_stat_reference(value: &Value) -> Option<crate::target::ChooseSpec> {
        let spec = match value.unhinted() {
            Value::PowerOf(spec) | Value::ToughnessOf(spec) => spec.as_ref(),
            _ => return None,
        };
        matches!(spec.unhinted(), crate::target::ChooseSpec::Tagged(tag) if tag.as_str() == IT_TAG)
            .then(|| spec.clone())
    }

    fn life_amount(effect: &EffectAst) -> Option<&Value> {
        let EffectAst::SubjectVerb(SubjectVerbEffectAst { action, .. }) = effect else {
            return None;
        };
        match action {
            SubjectVerbActionAst::GainLife { amount }
            | SubjectVerbActionAst::LoseLife { amount } => Some(amount),
            _ => None,
        }
    }

    fn retarget_source_stat(value: &mut Value, antecedent: &crate::target::ChooseSpec) {
        match value {
            Value::SurfaceHinted { value, .. } => retarget_source_stat(value, antecedent),
            Value::SourcePower => {
                *value = Value::PowerOf(Box::new(antecedent.clone()));
            }
            Value::SourceToughness => {
                *value = Value::ToughnessOf(Box::new(antecedent.clone()));
            }
            Value::PowerOf(spec)
                if matches!(spec.unhinted(), crate::target::ChooseSpec::Source) =>
            {
                *spec = Box::new(antecedent.clone());
            }
            Value::ToughnessOf(spec)
                if matches!(spec.unhinted(), crate::target::ChooseSpec::Source) =>
            {
                *spec = Box::new(antecedent.clone());
            }
            _ => {}
        }
    }

    for index in 0..effects.len().saturating_sub(1) {
        let Some(antecedent) = life_amount(&effects[index]).and_then(tagged_stat_reference) else {
            continue;
        };
        let EffectAst::SubjectVerb(SubjectVerbEffectAst { action, .. }) = &mut effects[index + 1]
        else {
            continue;
        };
        let amount = match action {
            SubjectVerbActionAst::GainLife { amount }
            | SubjectVerbActionAst::LoseLife { amount } => amount,
            _ => continue,
        };
        retarget_source_stat(amount, &antecedent);
    }
}

fn bind_source_exiled_effect(effect: EffectAst, bind: bool) -> EffectAst {
    if bind {
        EffectAst::TagAffected {
            effect: Box::new(effect),
            tag: TagKey::from(crate::tag::SOURCE_EXILED_TAG),
        }
    } else {
        effect
    }
}

fn parse_tap_those_then_unattach_equipment_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if !chain_grammar::parse_tap_then_unattach_tokens(tokens) {
        return Ok(None);
    }

    let mut tapped_filter = ObjectFilter::creature();
    tapped_filter.zone = Some(Zone::Battlefield);
    tapped_filter
        .tagged_constraints
        .push(crate::filter::TaggedObjectConstraint {
            tag: TagKey::from(IT_TAG),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });

    let mut equipment_filter = ObjectFilter::permanent();
    equipment_filter.card_types.push(CardType::Artifact);
    equipment_filter.subtypes.push(Subtype::Equipment);
    equipment_filter.zone = Some(Zone::Battlefield);
    equipment_filter
        .tagged_constraints
        .push(crate::filter::TaggedObjectConstraint {
            tag: TagKey::from(IT_TAG),
            relation: TaggedOpbjectRelation::AttachedToTaggedObject,
        });

    Ok(Some(vec![
        EffectAst::subject_verb_tap(TargetAst::Object(tapped_filter, None, None)),
        EffectAst::subject_verb_unattach(TargetAst::WithCount(
            Box::new(TargetAst::Object(equipment_filter, None, None)),
            ChoiceCount::any_number(),
        )),
    ]))
}

pub(crate) fn parse_return_it_then_loses_all_abilities_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(return_tokens) = chain_grammar::split_return_then_loses_tokens(tokens) else {
        return Ok(None);
    };
    let mut effects = parse_effect_chain_inner_lexed(return_tokens)?;
    effects.push(EffectAst::subject_verb_remove_abilities_from_target(
        TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(tokens)),
        Vec::new(),
        Until::Forever,
    ));
    Ok(Some(effects))
}

fn is_orphan_rounded_up_where_x_tail(
    segment: &[OwnedLexToken],
    previous: Option<&[OwnedLexToken]>,
    previous_effect: Option<&EffectAst>,
) -> bool {
    if !chain_grammar::is_rounded_up_segment_tokens(segment) {
        return false;
    }
    if previous.is_none() && previous_effect.is_none() {
        return true;
    }
    previous.is_some_and(chain_grammar::has_where_x_is_half_tokens)
        || previous_effect.is_some_and(effect_uses_half_life_total_value)
}

fn effect_uses_half_life_total_value(effect: &EffectAst) -> bool {
    match effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::CreateTokenWithMods {
                    dynamic_power_toughness,
                    ..
                },
            ..
        }) => dynamic_power_toughness
            .as_ref()
            .is_some_and(|(power, toughness)| {
                value_is_half_life_total(power) || value_is_half_life_total(toughness)
            }),
        EffectAst::ForEachObject { effects, .. }
        | EffectAst::ForEachOpponent { effects }
        | EffectAst::ForEachPlayer { effects }
        | EffectAst::ForEachTagged { effects, .. }
        | EffectAst::ForEachPlayersFiltered { effects, .. }
        | EffectAst::May { effects }
        | EffectAst::MayByPlayer { effects, .. }
        | EffectAst::AnyPlayerMay { effects }
        | EffectAst::IfResult { effects, .. }
        | EffectAst::WhenResult { effects, .. }
        | EffectAst::ManaRestricted { effects, .. } => {
            effects.iter().any(effect_uses_half_life_total_value)
        }
        _ => false,
    }
}

fn value_is_half_life_total(value: &Value) -> bool {
    matches!(value.unhinted(), Value::HalfLifeTotalRoundedUp(_))
}

fn leading_duration_for_followup_carry(tokens: &[OwnedLexToken]) -> Option<Until> {
    chain_grammar::parse_carry_duration_prefix_tokens(tokens).map(|shape| shape.duration)
}

fn effect_duration_for_gain_followup_carry(effect: &EffectAst) -> Option<Until> {
    let duration = match effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::GainControl { duration, .. }
                | SubjectVerbActionAst::Pump { duration, .. }
                | SubjectVerbActionAst::PumpAll { duration, .. }
                | SubjectVerbActionAst::SetBasePowerToughness { duration, .. }
                | SubjectVerbActionAst::SetBasePower { duration, .. }
                | SubjectVerbActionAst::BecomeBasePtCreature { duration, .. }
                | SubjectVerbActionAst::AddCardTypes { duration, .. }
                | SubjectVerbActionAst::SetCardTypes { duration, .. }
                | SubjectVerbActionAst::RemoveCardTypes { duration, .. }
                | SubjectVerbActionAst::AddSubtypes { duration, .. }
                | SubjectVerbActionAst::AddColors { duration, .. }
                | SubjectVerbActionAst::AddAllSubtypesOfFamily { duration, .. }
                | SubjectVerbActionAst::RemoveAllSubtypesOfFamily { duration, .. }
                | SubjectVerbActionAst::SetColors { duration, .. }
                | SubjectVerbActionAst::MakeColorless { duration, .. }
                | SubjectVerbActionAst::BecomeBasicLandType { duration, .. }
                | SubjectVerbActionAst::BecomeBasicLandTypeChoice { duration, .. }
                | SubjectVerbActionAst::BecomeColorChoice { duration, .. }
                | SubjectVerbActionAst::BecomeCreatureTypeChoice { duration, .. }
                | SubjectVerbActionAst::BecomeCopy { duration, .. }
                | SubjectVerbActionAst::GrantAbilitiesToTarget { duration, .. }
                | SubjectVerbActionAst::GrantAbilitiesAll { duration, .. }
                | SubjectVerbActionAst::GrantAbilitiesChoiceToTarget { duration, .. }
                | SubjectVerbActionAst::RemoveAbilitiesFromTarget { duration, .. }
                | SubjectVerbActionAst::RemoveAbilitiesAll { duration, .. },
            ..
        }) => duration,
        _ => return None,
    };

    if matches!(duration, Until::Forever) {
        None
    } else {
        Some(duration.clone())
    }
}

fn apply_carried_effect_duration(effect: &mut EffectAst, duration: &Until) {
    match effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::GainControl {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::Pump {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::PumpAll {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::SetBasePowerToughness {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::SetBasePower {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::BecomeBasePtCreature {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::AddCardTypes {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::SetCardTypes {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::RemoveCardTypes {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::AddSubtypes {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::AddColors {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::AddAllSubtypesOfFamily {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::RemoveAllSubtypesOfFamily {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::SetColors {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::MakeColorless {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::BecomeBasicLandType {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::BecomeBasicLandTypeChoice {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::BecomeColorChoice {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::BecomeCreatureTypeChoice {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::BecomeCopy {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::GrantAbilitiesToTarget {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::GrantAbilitiesAll {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::GrantAbilitiesChoiceToTarget {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::RemoveAbilitiesFromTarget {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::RemoveAbilitiesAll {
                    duration: effect_duration,
                    ..
                },
            ..
        }) if matches!(effect_duration, Until::Forever) => {
            *effect_duration = duration.clone();
        }
        EffectAst::Conditional {
            if_true, if_false, ..
        } => {
            for nested in if_true.iter_mut().chain(if_false.iter_mut()) {
                apply_carried_effect_duration(nested, duration);
            }
        }
        _ => {}
    }
}

pub(crate) fn collapse_for_each_player_it_tag_followups(effects: &mut Vec<EffectAst>) {
    let mut idx = 0usize;
    while idx + 1 < effects.len() {
        let should_merge = match (&effects[idx], &effects[idx + 1]) {
            (
                EffectAst::ForEachPlayer { .. },
                EffectAst::ForEachPlayer {
                    effects: followup_effects,
                },
            ) => effects_reference_it_tag(followup_effects),
            _ => false,
        };

        if !should_merge {
            idx += 1;
            continue;
        }

        let followup = effects.remove(idx + 1);
        match (&mut effects[idx], followup) {
            (
                EffectAst::ForEachPlayer {
                    effects: first_effects,
                },
                EffectAst::ForEachPlayer {
                    effects: mut followup_effects,
                },
            ) => {
                first_effects.append(&mut followup_effects);
            }
            _ => {
                // Defensive: should be unreachable given should_merge checks.
            }
        }
        // Re-check this index in case we have a longer chain of followups.
    }
}

pub(crate) fn collapse_for_each_object_it_tag_followups(effects: &mut Vec<EffectAst>) {
    let mut idx = 0usize;
    while idx + 1 < effects.len() {
        let should_merge = match (&effects[idx], &effects[idx + 1]) {
            (EffectAst::ForEachObject { filter, .. }, followup) => {
                effects_reference_it_tag(std::slice::from_ref(followup))
                    || (for_each_revealed_this_way_filter(filter)
                        && is_revealed_this_way_scalar_reward(followup))
            }
            _ => false,
        };

        if !should_merge {
            idx += 1;
            continue;
        }

        let followup = effects.remove(idx + 1);
        match (&mut effects[idx], followup) {
            (EffectAst::ForEachObject { effects: inner, .. }, followup) => {
                inner.push(followup);
            }
            _ => {
                // Defensive: should be unreachable given should_merge checks.
            }
        }
        // Re-check this index in case we have a longer chain of followups.
    }
}

fn for_each_revealed_this_way_filter(filter: &ObjectFilter) -> bool {
    filter.tagged_constraints.iter().any(|constraint| {
        constraint.relation == TaggedOpbjectRelation::IsTaggedObject
            && (constraint.tag.as_str() == IT_TAG
                || sentence_helper_revealed_tag(constraint.tag.as_str()))
    })
}

fn sentence_helper_revealed_tag(tag: &str) -> bool {
    tag.starts_with(SENTENCE_HELPER_REVEALED_TAG_PREFIX)
}

fn is_revealed_this_way_scalar_reward(effect: &EffectAst) -> bool {
    matches!(
        effect,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::GainLife { .. }
                | SubjectVerbActionAst::AddMana { .. }
                | SubjectVerbActionAst::AddManaScaled { .. }
                | SubjectVerbActionAst::AddManaAnyColor { .. }
                | SubjectVerbActionAst::AddManaAnyOneColor { .. }
                | SubjectVerbActionAst::AddManaChosenColor { .. }
                | SubjectVerbActionAst::AddManaCommanderIdentity { .. },
            ..
        })
    )
}

pub(crate) fn parse_effect_clause_with_trailing_if_lexed(
    tokens: &[OwnedLexToken],
) -> Result<EffectAst, CardTextError> {
    let Some(trailing_if) = split_trailing_if_clause_lexed(tokens) else {
        return parse_effect_clause_lexed(tokens);
    };
    let predicate = trailing_if.predicate;
    if !trailing_if_predicate_supported(&predicate) {
        return parse_effect_clause_lexed(tokens);
    }

    let base_effect = if let Ok(effect) = parse_effect_clause_lexed(trailing_if.leading_tokens) {
        effect
    } else {
        if let Some(effect) = parse_simple_lose_ability_clause_lexed(trailing_if.leading_tokens)? {
            effect
        } else if let Some(effect) =
            parse_simple_gain_ability_clause_lexed(trailing_if.leading_tokens)?
        {
            effect
        } else {
            return parse_effect_clause_lexed(tokens);
        }
    };

    Ok(EffectAst::Conditional {
        predicate,
        if_true: vec![base_effect],
        if_false: Vec::new(),
    })
}

fn trailing_if_predicate_supported(predicate: &PredicateAst) -> bool {
    matches!(
        predicate,
        PredicateAst::ManaSpentToCastThisSpellAtLeast { .. }
            | PredicateAst::ColoredManaSpentToCastThisSpellAtLeast(_)
            | PredicateAst::SameColorManaSpentToCastThisSpellAtLeast(_)
            | PredicateAst::ItMatches(_)
            | PredicateAst::ItMatchedLastKnown(_)
            | PredicateAst::TargetMatches(_)
            | PredicateAst::PlayerControlsMoreThanYou { .. }
            | PredicateAst::PlayerControls { .. }
            | PredicateAst::PlayerHasAtLeast { .. }
            | PredicateAst::PlayerControlsExactly { .. }
            | PredicateAst::PlayerHasAtLeastWithDifferentPowers { .. }
            | PredicateAst::PlayerLifeAtMostHalfStartingLifeTotal { .. }
            | PredicateAst::PlayerLifeLessThanHalfStartingLifeTotal { .. }
            | PredicateAst::PlayerHasMoreLifeThanYou { .. }
            | PredicateAst::PlayerHasNoOpponentWithMoreLifeThan { .. }
            | PredicateAst::PlayerHasMoreLifeThanEachOtherPlayer { .. }
            | PredicateAst::PlayerIsMonarch { .. }
            | PredicateAst::PlayerHasInitiative { .. }
            | PredicateAst::PlayerHasCitysBlessing { .. }
            | PredicateAst::PlayerHasMoreCardsInHandThanYou { .. }
            | PredicateAst::PlayerHasCardTypesInGraveyardOrMore { .. }
            | PredicateAst::YouControlMoreCreaturesThanTargetSpellController
            | PredicateAst::ValueComparison { .. }
    ) || matches!(predicate, PredicateAst::TaggedMatches(tag, _) if tag.as_str() == ENCHANTED_TAG_NAME)
}

pub(crate) fn target_is_generic_token_filter(target: &TargetAst) -> bool {
    let TargetAst::Object(filter, _, _) = target else {
        return false;
    };
    filter.token
        && filter.zone.is_none()
        && filter.card_types.is_empty()
        && filter.subtypes.is_empty()
        && filter.tagged_constraints.is_empty()
        && filter.controller.is_none()
        && filter.owner.is_none()
}

pub(crate) fn collapse_token_copy_next_end_step_exile_followup_lexed(
    effects: &mut Vec<EffectAst>,
    tokens: &[OwnedLexToken],
) {
    let facts = chain_grammar::parse_delayed_copy_facts_tokens(tokens);
    let Some(chain_grammar::DelayedCopyTiming::EndStep { player_is_you }) = facts.timing else {
        return;
    };
    if !facts.has_exile || !facts.has_token {
        return;
    }
    let next_end_step_player = if player_is_you {
        PlayerFilter::You
    } else {
        PlayerFilter::Any
    };

    let mut idx = 0usize;
    while idx + 1 < effects.len() {
        let mark_next_end_step_exile = match (&effects[idx], &effects[idx + 1]) {
            (
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action:
                        SubjectVerbActionAst::CreateTokenCopy { .. }
                        | SubjectVerbActionAst::CreateTokenCopyFromSource { .. },
                    ..
                }),
                EffectAst::SubjectVerb(subject_verb),
            ) => match &subject_verb.action {
                SubjectVerbActionAst::MoveToZone {
                    target,
                    zone: Zone::Exile,
                    ..
                }
                | SubjectVerbActionAst::Exile { target, .. } => {
                    target_is_generic_token_filter(target)
                }
                _ => false,
            },
            _ => false,
        };

        if !mark_next_end_step_exile {
            idx += 1;
            continue;
        }

        match &mut effects[idx] {
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::CreateTokenCopy {
                        exile_at_next_end_step,
                        next_end_step_player: effect_next_end_step_player,
                        ..
                    }
                    | SubjectVerbActionAst::CreateTokenCopyFromSource {
                        exile_at_next_end_step,
                        next_end_step_player: effect_next_end_step_player,
                        ..
                    },
                ..
            }) => {
                *exile_at_next_end_step = true;
                *effect_next_end_step_player = next_end_step_player.clone();
            }
            _ => {}
        }
        effects.remove(idx + 1);
    }
}

pub(crate) fn collapse_token_copy_next_end_step_sacrifice_followup_lexed(
    effects: &mut Vec<EffectAst>,
    tokens: &[OwnedLexToken],
) {
    let facts = chain_grammar::parse_delayed_copy_facts_tokens(tokens);
    if !facts.has_sacrifice || !facts.has_token {
        return;
    }
    let (is_next_upkeep, upkeep_player_is_you, next_end_step_player) = match facts.timing {
        Some(chain_grammar::DelayedCopyTiming::EndStep { player_is_you }) => (
            false,
            false,
            if player_is_you {
                PlayerFilter::You
            } else {
                PlayerFilter::Any
            },
        ),
        Some(chain_grammar::DelayedCopyTiming::Upkeep { player_is_you }) => {
            (true, player_is_you, PlayerFilter::Any)
        }
        _ => return,
    };

    let mut idx = 0usize;
    while idx + 1 < effects.len() {
        let mark_next_end_step_sacrifice = match (&effects[idx], &effects[idx + 1]) {
            (
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action:
                        SubjectVerbActionAst::CreateTokenCopy { .. }
                        | SubjectVerbActionAst::CreateTokenCopyFromSource { .. },
                    ..
                }),
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::Sacrifice { filter, count, .. },
                    ..
                }),
            ) => *count == 1 && filter.token,
            _ => false,
        };

        if !mark_next_end_step_sacrifice {
            idx += 1;
            continue;
        }

        if is_next_upkeep {
            let sacrifice = effects.remove(idx + 1);
            effects.insert(
                idx + 1,
                EffectAst::DelayedUntilNextUpkeep {
                    player: if upkeep_player_is_you {
                        PlayerAst::You
                    } else {
                        PlayerAst::Any
                    },
                    effects: vec![sacrifice],
                },
            );
            idx += 2;
            continue;
        }

        match &mut effects[idx] {
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::CreateTokenCopy {
                        sacrifice_at_next_end_step,
                        next_end_step_player: effect_next_end_step_player,
                        ..
                    }
                    | SubjectVerbActionAst::CreateTokenCopyFromSource {
                        sacrifice_at_next_end_step,
                        next_end_step_player: effect_next_end_step_player,
                        ..
                    },
                ..
            }) => {
                *sacrifice_at_next_end_step = true;
                *effect_next_end_step_player = next_end_step_player.clone();
            }
            _ => {}
        }
        effects.remove(idx + 1);
    }
}

pub(crate) fn collapse_token_copy_end_of_combat_exile_followup_lexed(
    effects: &mut Vec<EffectAst>,
    tokens: &[OwnedLexToken],
) {
    let facts = chain_grammar::parse_delayed_copy_facts_tokens(tokens);
    if !facts.has_exile
        || !facts.has_token
        || facts.timing != Some(chain_grammar::DelayedCopyTiming::EndOfCombat)
    {
        return;
    }

    let mut idx = 0usize;
    while idx + 1 < effects.len() {
        let mark_end_of_combat_exile = match (&effects[idx], &effects[idx + 1]) {
            (
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action:
                        SubjectVerbActionAst::CreateTokenCopy { .. }
                        | SubjectVerbActionAst::CreateTokenCopyFromSource { .. }
                        | SubjectVerbActionAst::CreateTokenWithMods { .. },
                    ..
                }),
                EffectAst::SubjectVerb(subject_verb),
            ) => match &subject_verb.action {
                SubjectVerbActionAst::MoveToZone {
                    target,
                    zone: Zone::Exile,
                    ..
                }
                | SubjectVerbActionAst::Exile { target, .. } => {
                    target_is_generic_token_filter(target)
                }
                _ => false,
            },
            _ => false,
        };

        if !mark_end_of_combat_exile {
            idx += 1;
            continue;
        }

        match &mut effects[idx] {
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::CreateTokenCopy {
                        exile_at_end_of_combat,
                        ..
                    }
                    | SubjectVerbActionAst::CreateTokenCopyFromSource {
                        exile_at_end_of_combat,
                        ..
                    }
                    | SubjectVerbActionAst::CreateTokenWithMods {
                        exile_at_end_of_combat,
                        ..
                    },
                ..
            }) => {
                *exile_at_end_of_combat = true;
            }
            _ => {}
        }
        effects.remove(idx + 1);
    }
}

fn split_on_comma_or_semicolon_lexed(tokens: &[OwnedLexToken]) -> Vec<Vec<OwnedLexToken>> {
    let mut segments = Vec::new();
    let mut start = 0usize;
    let mut inside_quotes = false;
    for (idx, token) in tokens.iter().enumerate() {
        if token.kind == TokenKind::Quote {
            inside_quotes = !inside_quotes;
            continue;
        }
        if inside_quotes || !matches!(token.kind, TokenKind::Comma | TokenKind::Semicolon) {
            continue;
        }
        let current = trim_lexed_commas(&tokens[start..idx]);
        if !current.is_empty() {
            segments.push(current.to_vec());
        }
        start = idx + 1;
    }
    let tail = trim_lexed_commas(&tokens[start..]);
    if !tail.is_empty() {
        segments.push(tail.to_vec());
    }
    segments
}

pub(crate) fn expand_segments_with_comma_action_clauses_lexed(
    segments: Vec<Vec<OwnedLexToken>>,
) -> Vec<Vec<OwnedLexToken>> {
    let mut expanded = Vec::new();

    for segment in segments {
        let looks_like_sac_discard_chain = (grammar::contains_word(&segment, "sacrifice")
            || grammar::contains_word(&segment, "sacrifices"))
            && (grammar::contains_word(&segment, "discard")
                || grammar::contains_word(&segment, "discards"));
        if !looks_like_sac_discard_chain {
            expanded.push(segment);
            continue;
        }

        let comma_parts = split_on_comma_or_semicolon_lexed(&segment);
        if comma_parts.len() < 2 {
            expanded.push(segment);
            continue;
        }

        let mut local_parts: Vec<Vec<OwnedLexToken>> = Vec::new();
        let mut valid_split = true;

        for raw_part in comma_parts {
            let mut part = trim_lexed_commas(&raw_part).to_vec();
            while let Some(rest) = chain_grammar::strip_leading_and_tokens(&part) {
                part = rest.to_vec();
            }
            if part.is_empty() {
                continue;
            }

            if segment_has_effect_head_lexed(&part) {
                local_parts.push(part);
                continue;
            }
            if let Some(previous) = local_parts.last()
                && let Some(expanded_part) = expand_missing_verb_segment_lexed(previous, &part)
            {
                local_parts.push(expanded_part);
                continue;
            }

            valid_split = false;
            break;
        }

        if valid_split && local_parts.len() > 1 {
            expanded.extend(local_parts);
        } else {
            expanded.push(segment);
        }
    }

    expanded
}

pub(crate) fn expand_segments_with_multi_create_clauses_lexed(
    segments: Vec<Vec<OwnedLexToken>>,
) -> Vec<Vec<OwnedLexToken>> {
    let mut expanded = Vec::new();

    for segment in segments {
        let Some((Verb::Create, _)) = find_verb_lexed(&segment) else {
            expanded.push(segment);
            continue;
        };
        let has_token_rules_tail = chain_grammar::has_token_rules_tail_tokens(&segment);
        if has_token_rules_tail {
            expanded.push(segment);
            continue;
        }
        let token_mentions = chain_grammar::count_token_mentions(&segment);
        if token_mentions < 2 {
            expanded.push(segment);
            continue;
        }

        let comma_parts = split_on_comma_or_semicolon_lexed(&segment);
        if comma_parts.len() < 2 {
            expanded.push(segment);
            continue;
        }

        let mut local_parts: Vec<Vec<OwnedLexToken>> = Vec::new();
        for raw_part in comma_parts {
            let mut part = trim_lexed_commas(&raw_part).to_vec();
            while let Some(rest) = chain_grammar::strip_leading_and_tokens(&part) {
                part = rest.to_vec();
            }
            if part.is_empty() {
                continue;
            }
            if let Some(previous) = local_parts.last()
                && is_token_creation_context(previous)
                && starts_with_inline_token_rules_tail(&part)
            {
                if let Some(last) = local_parts.last_mut() {
                    last.push(OwnedLexToken::comma(TextSpan::synthetic()));
                    last.extend(part);
                }
                continue;
            }
            if segment_has_effect_head_lexed(&part) {
                local_parts.push(part);
                continue;
            }
            if let Some(previous) = local_parts.last()
                && let Some(expanded_part) = expand_missing_verb_segment_lexed(previous, &part)
            {
                local_parts.push(expanded_part);
                continue;
            }
            if let Some(last) = local_parts.last_mut() {
                last.push(OwnedLexToken::comma(TextSpan::synthetic()));
                last.extend(part);
            } else {
                local_parts.push(part);
            }
        }

        if local_parts.len() > 1 {
            expanded.extend(local_parts);
        } else {
            expanded.push(segment);
        }
    }

    expanded
}

pub(crate) fn expand_missing_verb_segment_lexed(
    previous: &[OwnedLexToken],
    segment: &[OwnedLexToken],
) -> Option<Vec<OwnedLexToken>> {
    let (verb, verb_idx) = find_verb_lexed(previous)?;
    match verb {
        Verb::Deal => {
            if parse_value_prefix_lexed(segment).is_none()
                || !grammar::contains_word(segment, "damage")
            {
                return None;
            }
            let mut expanded = Vec::new();
            expanded.extend(previous.iter().take(verb_idx + 1).cloned());
            expanded.extend(segment.iter().cloned());
            Some(expanded)
        }
        Verb::Sacrifice => {
            let segment_words = token_word_refs(segment);
            let starts_like_object_phrase = matches!(
                segment_words.first().copied(),
                Some("a" | "an" | "another" | "target")
            ) || parse_number_prefix_lexed(segment).is_some();
            if !starts_like_object_phrase {
                return None;
            }
            let mut expanded = Vec::new();
            expanded.extend(previous.iter().take(verb_idx + 1).cloned());
            expanded.extend(segment.iter().cloned());
            Some(expanded)
        }
        Verb::Create => {
            if !starts_like_create_fragment_lexed(segment) {
                return None;
            }
            let mut expanded = Vec::new();
            expanded.extend(previous.iter().take(verb_idx + 1).cloned());
            expanded.extend(segment.iter().cloned());
            Some(expanded)
        }
        _ => None,
    }
}

fn strip_leading_gain_duration_prefix(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    chain_grammar::parse_carry_duration_prefix_tokens(tokens)
        .map_or_else(|| trim_lexed_commas(tokens), |shape| shape.rest)
}

fn previous_segment_has_carryable_subject(previous: &[OwnedLexToken]) -> bool {
    let Some((_, verb_idx)) = find_verb_lexed(previous) else {
        return false;
    };
    if verb_idx == 0 {
        return false;
    }

    let prefix = trim_lexed_commas(&previous[..verb_idx]);
    let subject_tokens = strip_leading_gain_duration_prefix(prefix);
    if subject_tokens.is_empty() {
        return false;
    }

    chain_grammar::parse_carryable_subject_tokens(subject_tokens).is_some()
}

fn expand_gain_lose_followup_segment_lexed(
    previous: &[OwnedLexToken],
    segment: &[OwnedLexToken],
) -> Option<Vec<OwnedLexToken>> {
    let (verb, verb_idx) = find_verb_lexed(segment)?;
    if verb_idx != 0 || !matches!(verb, Verb::Gain | Verb::Lose) {
        return None;
    }
    if !previous_segment_has_carryable_subject(previous) {
        return None;
    }

    let previous_verb_idx = find_verb_lexed(previous)?.1;
    let mut expanded = Vec::new();
    let previous_subject =
        strip_leading_gain_duration_prefix(trim_lexed_commas(&previous[..previous_verb_idx]));
    let previous_subject_words = TokenWordView::new(previous_subject).word_refs();
    if matches!(
        previous_subject_words.as_slice(),
        ["target", "player"] | ["target", "opponent"]
    ) {
        // The subject of a bare gain/lose follow-up is the already chosen target,
        // not a second target. Preserve that provenance explicitly while retaining
        // the synthetic subject this early parser path needs.
        expanded.push(synthetic_lexed_word("that"));
        expanded.push(synthetic_lexed_word("player"));
    } else {
        expanded.extend(previous.iter().take(previous_verb_idx).cloned());
    }
    expanded.extend(segment.iter().cloned());
    Some(expanded)
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CarryContext {
    Player(PlayerAst),
    ForEachPlayer,
    ForEachTargetPlayers(ChoiceCount),
    ForEachOpponent,
}

pub(crate) fn player_ast_from_filter_for_carry(filter: &PlayerFilter) -> Option<PlayerAst> {
    match filter {
        PlayerFilter::You => Some(PlayerAst::You),
        PlayerFilter::Opponent => Some(PlayerAst::Opponent),
        PlayerFilter::Any => Some(PlayerAst::Any),
        PlayerFilter::IteratedPlayer => Some(PlayerAst::That),
        PlayerFilter::Target(inner) => {
            if matches!(inner.as_ref(), PlayerFilter::Opponent) {
                Some(PlayerAst::TargetOpponent)
            } else {
                Some(PlayerAst::Target)
            }
        }
        PlayerFilter::AliasedTarget(_) => Some(PlayerAst::That),
        _ => None,
    }
}

pub(crate) fn player_owner_filter_from_target_for_carry(target: &TargetAst) -> Option<PlayerAst> {
    match target {
        TargetAst::Player(filter, _) => player_ast_from_filter_for_carry(filter),
        TargetAst::Object(filter, _, _) => {
            if !matches!(
                filter.zone,
                Some(Zone::Hand) | Some(Zone::Graveyard) | Some(Zone::Library) | Some(Zone::Exile)
            ) {
                return None;
            }
            filter
                .owner
                .as_ref()
                .and_then(player_ast_from_filter_for_carry)
        }
        TargetAst::WithCount(inner, _) => player_owner_filter_from_target_for_carry(inner),
        _ => None,
    }
}

fn player_target_carry_context(target: &TargetAst) -> Option<CarryContext> {
    match target {
        TargetAst::Player(filter, _) => {
            player_ast_from_filter_for_carry(filter).map(CarryContext::Player)
        }
        TargetAst::WithCount(inner, count) => {
            let inner_context = player_target_carry_context(inner.as_ref())?;
            if count.min > 1 && count.max == Some(count.min) {
                Some(CarryContext::ForEachTargetPlayers(*count))
            } else {
                Some(inner_context)
            }
        }
        _ => None,
    }
}

pub(crate) fn explicit_player_for_carry(effect: &EffectAst) -> Option<CarryContext> {
    if matches!(effect, EffectAst::ForEachPlayer { .. }) {
        return Some(CarryContext::ForEachPlayer);
    }
    if let EffectAst::ForEachTargetPlayers { count, .. } = effect {
        return Some(CarryContext::ForEachTargetPlayers(*count));
    }
    if matches!(effect, EffectAst::ForEachOpponent { .. }) {
        return Some(CarryContext::ForEachOpponent);
    }
    if let EffectAst::SubjectVerb(subject_verb) = effect
        && let SubjectVerbActionAst::TargetOnly { target } = &subject_verb.action
        && let Some(context) = player_target_carry_context(target)
    {
        return Some(context);
    }
    if let EffectAst::SubjectVerb(subject_verb) = effect
        && let SubjectVerbActionAst::Exile { target, .. } = &subject_verb.action
        && let Some(player) = player_owner_filter_from_target_for_carry(target)
    {
        return Some(CarryContext::Player(player));
    }
    if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::ExileUntilSourceLeaves { target, .. },
        ..
    }) = effect
        && let Some(player) = player_owner_filter_from_target_for_carry(target)
    {
        return Some(CarryContext::Player(player));
    }
    if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::ExileAll { filter, .. },
        ..
    }) = effect
        && let Some(owner) = filter.owner.as_ref()
        && let Some(player) = player_ast_from_filter_for_carry(owner)
    {
        return Some(CarryContext::Player(player));
    }
    if matches!(
        effect,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::ChoosePlayer { .. },
            ..
        })
    ) {
        return Some(CarryContext::Player(PlayerAst::That));
    }

    let player = match effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::SearchLibrary {
                    chooser, player, ..
                },
            ..
        }) => {
            if !matches!(player, PlayerAst::Implicit) {
                *player
            } else if !matches!(chooser, PlayerAst::Implicit) {
                *chooser
            } else {
                return None;
            }
        }
        EffectAst::SubjectVerb(_) => subject_verb_player_action_player(effect)?,
        EffectAst::ChooseObjects { player, .. }
        | EffectAst::ChooseObjectsWithAggregateConstraint { player, .. } => *player,
        _ => return None,
    };

    if matches!(player, PlayerAst::Implicit) {
        None
    } else {
        Some(CarryContext::Player(player))
    }
}

pub(crate) fn effect_uses_implicit_player(effect: &EffectAst) -> bool {
    match effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::SearchLibrary {
                    chooser, player, ..
                },
            ..
        }) => matches!(*chooser, PlayerAst::Implicit) || matches!(*player, PlayerAst::Implicit),
        EffectAst::SubjectVerb(_) => {
            matches!(
                subject_verb_player_action_player(effect),
                Some(PlayerAst::Implicit)
            )
        }
        EffectAst::ChooseObjects { player, .. }
        | EffectAst::ChooseObjectsWithAggregateConstraint { player, .. } => {
            matches!(*player, PlayerAst::Implicit)
        }
        _ => false,
    }
}

fn effect_uses_that_player(effect: &EffectAst) -> bool {
    match effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::SearchLibrary {
                    chooser, player, ..
                },
            ..
        }) => matches!(*chooser, PlayerAst::That) || matches!(*player, PlayerAst::That),
        EffectAst::SubjectVerb(_) => {
            matches!(
                subject_verb_player_action_player(effect),
                Some(PlayerAst::That)
            )
        }
        EffectAst::ChooseObjects { player, .. }
        | EffectAst::ChooseObjectsWithAggregateConstraint { player, .. } => {
            matches!(*player, PlayerAst::That)
        }
        _ => false,
    }
}

fn subject_verb_player_action_player_mut(effect: &mut EffectAst) -> Option<&mut PlayerAst> {
    match effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::CreateTokenCopy { player, .. }
                | SubjectVerbActionAst::CreateTokenCopyFromSource { player, .. }
                | SubjectVerbActionAst::CreateTokenWithMods { player, .. },
            ..
        }) => Some(player),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            subject: SubjectVerbSubjectAst { player, .. },
            action:
                SubjectVerbActionAst::Draw { .. }
                | SubjectVerbActionAst::LoseLife { .. }
                | SubjectVerbActionAst::GainLife { .. }
                | SubjectVerbActionAst::RevealHand
                | SubjectVerbActionAst::RevealTop
                | SubjectVerbActionAst::RevealCardsFromHand { .. }
                | SubjectVerbActionAst::Mill { .. }
                | SubjectVerbActionAst::Scry { .. }
                | SubjectVerbActionAst::Surveil { .. }
                | SubjectVerbActionAst::Discard { .. }
                | SubjectVerbActionAst::DiscardHand
                | SubjectVerbActionAst::PoisonCounters { .. }
                | SubjectVerbActionAst::EnergyCounters { .. }
                | SubjectVerbActionAst::ExperienceCounters { .. }
                | SubjectVerbActionAst::TicketCounters { .. }
                | SubjectVerbActionAst::PayEnergy { .. }
                | SubjectVerbActionAst::PayAnyEnergy { .. }
                | SubjectVerbActionAst::PayAnyLife { .. }
                | SubjectVerbActionAst::PayMana { .. }
                | SubjectVerbActionAst::DoubleManaPool
                | SubjectVerbActionAst::EmptyManaPool
                | SubjectVerbActionAst::SetLifeTotal { .. }
                | SubjectVerbActionAst::SkipTurn
                | SubjectVerbActionAst::SkipCombatPhases
                | SubjectVerbActionAst::SkipNextCombatPhaseThisTurn
                | SubjectVerbActionAst::SkipMainPhasesThisTurn
                | SubjectVerbActionAst::SkipCombatPhasesThisTurn
                | SubjectVerbActionAst::SkipDrawStep
                | SubjectVerbActionAst::RingTemptsYou
                | SubjectVerbActionAst::VentureIntoDungeon { .. }
                | SubjectVerbActionAst::BecomeMonarch
                | SubjectVerbActionAst::TakeInitiative
                | SubjectVerbActionAst::CreateEmblem { .. }
                | SubjectVerbActionAst::LoseGame
                | SubjectVerbActionAst::WinGame
                | SubjectVerbActionAst::FlipCoin
                | SubjectVerbActionAst::RollDie { .. }
                | SubjectVerbActionAst::RollDiceChooseResult { .. }
                | SubjectVerbActionAst::ShuffleHandAndGraveyardIntoLibrary
                | SubjectVerbActionAst::ShuffleGraveyardIntoLibrary
                | SubjectVerbActionAst::ReorderGraveyard
                | SubjectVerbActionAst::ChooseColor
                | SubjectVerbActionAst::ChooseCardType { .. }
                | SubjectVerbActionAst::ChooseNamedOption { .. }
                | SubjectVerbActionAst::ChooseCreatureType { .. }
                | SubjectVerbActionAst::ChooseLandType { .. }
                | SubjectVerbActionAst::ChooseCardName { .. }
                | SubjectVerbActionAst::ChoosePlayer { .. }
                | SubjectVerbActionAst::NoteLifeTotal
                | SubjectVerbActionAst::AddMana { .. }
                | SubjectVerbActionAst::AddManaScaled { .. }
                | SubjectVerbActionAst::AddManaAnyColor { .. }
                | SubjectVerbActionAst::AddManaAnyOneColor { .. }
                | SubjectVerbActionAst::AddManaChosenColor { .. }
                | SubjectVerbActionAst::AddManaFromLandCouldProduce { .. }
                | SubjectVerbActionAst::AddManaCommanderIdentity { .. }
                | SubjectVerbActionAst::MoveToZone { .. }
                | SubjectVerbActionAst::AdditionalLandPlays { .. }
                | SubjectVerbActionAst::ExtraTurnAfterTurn { .. }
                | SubjectVerbActionAst::Sacrifice { .. }
                | SubjectVerbActionAst::ShuffleLibrary,
        }) => Some(player),
        _ => None,
    }
}

fn subject_verb_player_action_player(effect: &EffectAst) -> Option<PlayerAst> {
    match effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::CreateTokenCopy { player, .. }
                | SubjectVerbActionAst::CreateTokenCopyFromSource { player, .. }
                | SubjectVerbActionAst::CreateTokenWithMods { player, .. },
            ..
        }) => Some(*player),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            subject: SubjectVerbSubjectAst { player, .. },
            action:
                SubjectVerbActionAst::Draw { .. }
                | SubjectVerbActionAst::LoseLife { .. }
                | SubjectVerbActionAst::GainLife { .. }
                | SubjectVerbActionAst::RevealHand
                | SubjectVerbActionAst::RevealTop
                | SubjectVerbActionAst::RevealCardsFromHand { .. }
                | SubjectVerbActionAst::Mill { .. }
                | SubjectVerbActionAst::Scry { .. }
                | SubjectVerbActionAst::Surveil { .. }
                | SubjectVerbActionAst::Discard { .. }
                | SubjectVerbActionAst::DiscardHand
                | SubjectVerbActionAst::PoisonCounters { .. }
                | SubjectVerbActionAst::EnergyCounters { .. }
                | SubjectVerbActionAst::ExperienceCounters { .. }
                | SubjectVerbActionAst::TicketCounters { .. }
                | SubjectVerbActionAst::PayEnergy { .. }
                | SubjectVerbActionAst::PayAnyEnergy { .. }
                | SubjectVerbActionAst::PayAnyLife { .. }
                | SubjectVerbActionAst::PayMana { .. }
                | SubjectVerbActionAst::DoubleManaPool
                | SubjectVerbActionAst::EmptyManaPool
                | SubjectVerbActionAst::SetLifeTotal { .. }
                | SubjectVerbActionAst::SkipTurn
                | SubjectVerbActionAst::SkipCombatPhases
                | SubjectVerbActionAst::SkipNextCombatPhaseThisTurn
                | SubjectVerbActionAst::SkipMainPhasesThisTurn
                | SubjectVerbActionAst::SkipCombatPhasesThisTurn
                | SubjectVerbActionAst::SkipDrawStep
                | SubjectVerbActionAst::RingTemptsYou
                | SubjectVerbActionAst::VentureIntoDungeon { .. }
                | SubjectVerbActionAst::BecomeMonarch
                | SubjectVerbActionAst::TakeInitiative
                | SubjectVerbActionAst::CreateEmblem { .. }
                | SubjectVerbActionAst::LoseGame
                | SubjectVerbActionAst::WinGame
                | SubjectVerbActionAst::FlipCoin
                | SubjectVerbActionAst::RollDie { .. }
                | SubjectVerbActionAst::RollDiceChooseResult { .. }
                | SubjectVerbActionAst::ShuffleHandAndGraveyardIntoLibrary
                | SubjectVerbActionAst::ShuffleGraveyardIntoLibrary
                | SubjectVerbActionAst::ReorderGraveyard
                | SubjectVerbActionAst::ChooseColor
                | SubjectVerbActionAst::ChooseCardType { .. }
                | SubjectVerbActionAst::ChooseNamedOption { .. }
                | SubjectVerbActionAst::ChooseCreatureType { .. }
                | SubjectVerbActionAst::ChooseLandType { .. }
                | SubjectVerbActionAst::ChooseCardName { .. }
                | SubjectVerbActionAst::ChoosePlayer { .. }
                | SubjectVerbActionAst::NoteLifeTotal
                | SubjectVerbActionAst::AddMana { .. }
                | SubjectVerbActionAst::AddManaScaled { .. }
                | SubjectVerbActionAst::AddManaAnyColor { .. }
                | SubjectVerbActionAst::AddManaAnyOneColor { .. }
                | SubjectVerbActionAst::AddManaChosenColor { .. }
                | SubjectVerbActionAst::AddManaFromLandCouldProduce { .. }
                | SubjectVerbActionAst::AddManaCommanderIdentity { .. }
                | SubjectVerbActionAst::MoveToZone { .. }
                | SubjectVerbActionAst::AdditionalLandPlays { .. }
                | SubjectVerbActionAst::ExtraTurnAfterTurn { .. }
                | SubjectVerbActionAst::Sacrifice { .. }
                | SubjectVerbActionAst::ShuffleLibrary,
        }) => Some(*player),
        _ => None,
    }
}

pub(crate) fn maybe_apply_carried_player(effect: &mut EffectAst, carried_context: CarryContext) {
    match carried_context {
        CarryContext::Player(carried_player) => {
            // When carrying an explicit target player/opponent into an implicit clause,
            // bind to the previously selected target ("that player") instead of creating
            // a fresh explicit target. This preserves shared-target semantics for chains
            // like "Target player mills..., draws..., and loses...".
            let carried_player = match carried_player {
                PlayerAst::Target | PlayerAst::TargetOpponent => PlayerAst::That,
                other => other,
            };
            match effect {
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action:
                        SubjectVerbActionAst::SearchLibrary {
                            chooser, player, ..
                        },
                    ..
                }) => {
                    if matches!(*chooser, PlayerAst::Implicit) {
                        *chooser = carried_player;
                    }
                    if matches!(*player, PlayerAst::Implicit) {
                        *player = carried_player;
                    }
                }
                EffectAst::SubjectVerb(_) => {
                    if let Some(player) = subject_verb_player_action_player_mut(effect)
                        && *player == PlayerAst::Implicit
                    {
                        *player = carried_player;
                    }
                }
                EffectAst::ChooseObjects { player, .. }
                | EffectAst::ChooseObjectsWithAggregateConstraint { player, .. } => {
                    if matches!(*player, PlayerAst::Implicit) {
                        *player = carried_player;
                    }
                }
                _ => {}
            }
        }
        CarryContext::ForEachPlayer => {
            if effect_uses_implicit_player(effect) || effect_uses_that_player(effect) {
                let wrapped = effect.clone();
                *effect = EffectAst::ForEachPlayer {
                    effects: vec![wrapped],
                };
            }
        }
        CarryContext::ForEachTargetPlayers(count) => {
            if effect_uses_implicit_player(effect) || effect_uses_that_player(effect) {
                let wrapped = effect.clone();
                *effect = EffectAst::ForEachTargetPlayers {
                    count,
                    effects: vec![wrapped],
                };
            }
        }
        CarryContext::ForEachOpponent => {
            if effect_uses_implicit_player(effect) || effect_uses_that_player(effect) {
                let wrapped = effect.clone();
                *effect = EffectAst::ForEachOpponent {
                    effects: vec![wrapped],
                };
            }
        }
    }
}

pub(crate) fn maybe_apply_carried_player_with_clause_lexed(
    effect: &mut EffectAst,
    carried_context: CarryContext,
    clause_tokens: &[OwnedLexToken],
) {
    let clause_head = chain_grammar::parse_carry_clause_head_tokens(clause_tokens);
    let explicitly_conjugated_player_action = clause_tokens.first().is_some_and(|token| {
        token.is_word("draws") || token.is_word("scries") || token.is_word("surveils")
    });
    if clause_head == chain_grammar::CarryClauseHead::Choose
        && normalize_imperative_choose_player(effect)
    {
        return;
    }
    if clause_head == chain_grammar::CarryClauseHead::Create
        && normalize_imperative_create_player(effect)
    {
        return;
    }
    let should_skip = match carried_context {
        CarryContext::Player(_) => {
            (matches!(
                effect,
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    subject: SubjectVerbSubjectAst {
                        player: PlayerAst::Implicit,
                        ..
                    },
                    action: SubjectVerbActionAst::Draw { .. },
                })
            ) && clause_head == chain_grammar::CarryClauseHead::Draw)
                && !explicitly_conjugated_player_action
                || (matches!(
                    effect,
                    EffectAst::SubjectVerb(SubjectVerbEffectAst {
                        subject: SubjectVerbSubjectAst {
                            player: PlayerAst::Implicit,
                            ..
                        },
                        action: SubjectVerbActionAst::Scry { .. }
                            | SubjectVerbActionAst::Surveil { .. },
                    })
                ) && matches!(
                    clause_head,
                    chain_grammar::CarryClauseHead::Scry | chain_grammar::CarryClauseHead::Surveil
                ) && !explicitly_conjugated_player_action)
        }
        CarryContext::ForEachPlayer
        | CarryContext::ForEachTargetPlayers(_)
        | CarryContext::ForEachOpponent => {
            let is_implicit_vision_effect = matches!(
                effect,
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    subject: SubjectVerbSubjectAst {
                        player: PlayerAst::Implicit,
                        ..
                    },
                    action: SubjectVerbActionAst::Draw { .. }
                        | SubjectVerbActionAst::Scry { .. }
                        | SubjectVerbActionAst::Surveil { .. },
                })
            );
            is_implicit_vision_effect
                && matches!(
                    clause_head,
                    chain_grammar::CarryClauseHead::Draw
                        | chain_grammar::CarryClauseHead::Scry
                        | chain_grammar::CarryClauseHead::Surveil
                )
                && !explicitly_conjugated_player_action
        }
    };
    if should_skip {
        return;
    }
    maybe_apply_carried_player(effect, carried_context);
}

fn normalize_imperative_choose_player(effect: &mut EffectAst) -> bool {
    let player = match effect {
        EffectAst::ChooseObjects { player, .. }
        | EffectAst::ChooseObjectsWithAggregateConstraint { player, .. }
        | EffectAst::ChooseObjectsAcrossZones { player, .. } => player,
        _ => return false,
    };

    if matches!(
        player,
        PlayerAst::Implicit | PlayerAst::Target | PlayerAst::TargetOpponent | PlayerAst::That
    ) {
        *player = PlayerAst::You;
        return true;
    }
    false
}

fn normalize_imperative_create_player(effect: &mut EffectAst) -> bool {
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::CreateTokenWithMods { player, .. },
        ..
    }) = effect
    else {
        return false;
    };

    if matches!(
        player,
        PlayerAst::Implicit | PlayerAst::Target | PlayerAst::TargetOpponent | PlayerAst::That
    ) {
        *player = PlayerAst::You;
        return true;
    }
    false
}

pub(crate) fn bind_implicit_player_context(effect: &mut EffectAst, player: PlayerAst) {
    match effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            subject,
            action: SubjectVerbActionAst::RetargetStackObject { .. },
        }) => {
            if matches!(subject.player, PlayerAst::Implicit) {
                subject.player = player;
            }
        }
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::CopySpell {
                    player: effect_player,
                    ..
                }
                | SubjectVerbActionAst::CopySpellForEachTarget {
                    player: effect_player,
                    ..
                }
                | SubjectVerbActionAst::CastTagged {
                    player: effect_player,
                    ..
                }
                | SubjectVerbActionAst::GrantPlayTaggedUntilEndOfTurn {
                    player: effect_player,
                    ..
                }
                | SubjectVerbActionAst::GrantTaggedSpellAlternativeCostPayLifeByManaValueUntilEndOfTurn {
                    player: effect_player,
                    ..
                }
                | SubjectVerbActionAst::GrantPlayTaggedUntilYourNextTurn {
                    player: effect_player,
                    ..
                }
                | SubjectVerbActionAst::GrantPlayTaggedForAsLongAsExiled {
                    player: effect_player,
                    ..
                },
            ..
        }) => {
            if matches!(*effect_player, PlayerAst::Implicit) {
                *effect_player = player;
            }
        }
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::SearchLibrary {
                    player: effect_player,
                    ..
                },
            ..
        }) => {
            if matches!(*effect_player, PlayerAst::Implicit) {
                *effect_player = player;
            }
        }
        EffectAst::SubjectVerb(_) => {
            if let Some(effect_player) = subject_verb_player_action_player_mut(effect)
                && matches!(*effect_player, PlayerAst::Implicit)
            {
                *effect_player = player;
            }
        }
        EffectAst::ChooseObjects {
            player: effect_player,
            ..
        }
        | EffectAst::ChooseObjectsWithAggregateConstraint {
            player: effect_player,
            ..
        }
        | EffectAst::ChooseObjectsAcrossZones {
            player: effect_player,
            ..
        } => {
            if matches!(*effect_player, PlayerAst::Implicit) {
                *effect_player = player;
            }
        }
        _ => for_each_nested_effects_mut(effect, true, |nested| {
            for nested_effect in nested {
                bind_implicit_player_context(nested_effect, player);
            }
        }),
    }
}

fn parse_leading_player_may_words(words: &[&str]) -> Option<PlayerAst> {
    type WordInput<'a> = grammar::WordSliceInput<'a>;
    use grammar::word_slice_exact as word_eq;

    fn player_word<'a>() -> impl Parser<WordInput<'a>, (), ErrMode<ContextError>> {
        alt((word_eq("player"), word_eq("players"))).void()
    }

    fn opponent_word<'a>() -> impl Parser<WordInput<'a>, (), ErrMode<ContextError>> {
        alt((word_eq("opponent"), word_eq("opponents"))).void()
    }

    fn controller_subject_word<'a>() -> impl Parser<WordInput<'a>, (), ErrMode<ContextError>> {
        alt((
            word_eq("creatures"),
            word_eq("permanents"),
            word_eq("planeswalkers"),
            word_eq("sources"),
            word_eq("spells"),
        ))
        .void()
    }

    fn controller_or_owner_subject_word<'a>()
    -> impl Parser<WordInput<'a>, (), ErrMode<ContextError>> {
        alt((
            word_eq("creatures"),
            word_eq("permanents"),
            word_eq("sources"),
            word_eq("spells"),
        ))
        .void()
    }

    fn leading_conjunctions<'a>(input: &mut WordInput<'a>) -> Result<(), ErrMode<ContextError>> {
        repeat::<_, _, (), _, _>(0.., alt((word_eq("then"), word_eq("and")))).parse_next(input)
    }

    fn parse_player_may_prefix<'a>(
        input: &mut WordInput<'a>,
    ) -> Result<PlayerAst, ErrMode<ContextError>> {
        (
            leading_conjunctions,
            alt((
                alt((
                    (word_eq("you"), word_eq("may")).value(PlayerAst::You),
                    (word_eq("any"), player_word(), word_eq("may")).value(PlayerAst::Any),
                    (word_eq("any"), opponent_word(), word_eq("may")).value(PlayerAst::Opponent),
                )),
                alt((
                    (word_eq("target"), opponent_word(), word_eq("may"))
                        .value(PlayerAst::TargetOpponent),
                    (word_eq("target"), player_word(), word_eq("may")).value(PlayerAst::Target),
                    (word_eq("that"), player_word(), word_eq("may")).value(PlayerAst::That),
                    (word_eq("that"), opponent_word(), word_eq("may")).value(PlayerAst::That),
                    (word_eq("they"), word_eq("may")).value(PlayerAst::That),
                    (
                        word_eq("that"),
                        word_eq("player"),
                        word_eq("or"),
                        word_eq("that"),
                        controller_subject_word(),
                        word_eq("controller"),
                        word_eq("may"),
                    )
                        .value(PlayerAst::ThatPlayerOrTargetController),
                    (
                        word_eq("that"),
                        controller_or_owner_subject_word(),
                        word_eq("controller"),
                        word_eq("may"),
                    )
                        .value(PlayerAst::ItsController),
                    (
                        word_eq("that"),
                        controller_or_owner_subject_word(),
                        word_eq("owner"),
                        word_eq("may"),
                    )
                        .value(PlayerAst::ItsOwner),
                )),
                alt((
                    (word_eq("the"), player_word(), word_eq("may")).value(PlayerAst::That),
                    (word_eq("defending"), word_eq("player"), word_eq("may"))
                        .value(PlayerAst::Defending),
                    alt((
                        (word_eq("attacking"), word_eq("player"), word_eq("may"))
                            .value(PlayerAst::Attacking),
                        (
                            word_eq("that"),
                            word_eq("attacking"),
                            word_eq("player"),
                            word_eq("may"),
                        )
                            .value(PlayerAst::Attacking),
                        (
                            word_eq("the"),
                            word_eq("attacking"),
                            word_eq("player"),
                            word_eq("may"),
                        )
                            .value(PlayerAst::Attacking),
                    )),
                    (
                        alt((word_eq("its"), word_eq("their"))),
                        word_eq("controller"),
                        word_eq("may"),
                    )
                        .value(PlayerAst::ItsController),
                    (
                        alt((word_eq("its"), word_eq("their"))),
                        word_eq("owner"),
                        word_eq("may"),
                    )
                        .value(PlayerAst::ItsOwner),
                    alt((
                        (opponent_word(), word_eq("may")).value(PlayerAst::Opponent),
                        (word_eq("an"), word_eq("opponent"), word_eq("may"))
                            .value(PlayerAst::Opponent),
                    )),
                )),
            )),
        )
            .map(|(_, player)| player)
            .parse_next(input)
    }

    let mut input = words;
    parse_player_may_prefix(&mut input).ok()
}

pub(crate) fn parse_leading_player_may_lexed(tokens: &[OwnedLexToken]) -> Option<PlayerAst> {
    let word_view = TokenWordView::new(tokens);
    let words = word_view.word_refs();
    parse_leading_player_may_words(&words)
}

pub(crate) fn find_verb(tokens: &[OwnedLexToken]) -> Option<(Verb, usize)> {
    find_verb_lexed(tokens)
}

pub(crate) fn parse_effect_chain(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    parse_effect_chain_lexed(tokens)
}

pub(crate) fn parse_effect_chain_with_subject_verb_primitives(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    parse_effect_chain_with_subject_verb_primitives_lexed(tokens)
}

pub(crate) fn parse_effect_chain_inner(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    parse_effect_chain_inner_lexed(tokens)
}

pub(crate) fn parse_effect_clause_with_trailing_if(
    tokens: &[OwnedLexToken],
) -> Result<EffectAst, CardTextError> {
    parse_effect_clause_with_trailing_if_lexed(tokens)
}

pub(crate) fn collapse_token_copy_next_end_step_exile_followup(
    effects: &mut Vec<EffectAst>,
    tokens: &[OwnedLexToken],
) {
    collapse_token_copy_next_end_step_exile_followup_lexed(effects, tokens);
}

pub(crate) fn collapse_token_copy_end_of_combat_exile_followup(
    effects: &mut Vec<EffectAst>,
    tokens: &[OwnedLexToken],
) {
    collapse_token_copy_end_of_combat_exile_followup_lexed(effects, tokens);
}

pub(crate) fn maybe_apply_carried_player_with_clause(
    effect: &mut EffectAst,
    carried_context: CarryContext,
    clause_tokens: &[OwnedLexToken],
) {
    maybe_apply_carried_player_with_clause_lexed(effect, carried_context, clause_tokens);
}

pub(crate) fn remove_first_word(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    remove_first_may_word_tokens(tokens)
}

pub(crate) fn remove_through_first_word(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    remove_through_first_may_word_tokens(tokens)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Verb {
    Add,
    Move,
    Deal,
    Draw,
    Counter,
    Destroy,
    Exile,
    Untap,
    Scry,
    Discard,
    Transform,
    Convert,
    Flip,
    Roll,
    Regenerate,
    Mill,
    Get,
    Reveal,
    Look,
    Lose,
    Gain,
    Put,
    Sacrifice,
    Create,
    Investigate,
    Proliferate,
    Tap,
    Attach,
    Unattach,
    Remove,
    Return,
    Exchange,
    Become,
    Switch,
    Skip,
    Surveil,
    Incubate,
    Shuffle,
    Reorder,
    Pay,
    Take,
    Detain,
    Goad,
    Suspect,
    Note,
    End,
}
