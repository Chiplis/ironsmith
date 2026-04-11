use winnow::Parser as _;
use winnow::combinator::{alt, cut_err, dispatch, fail, opt, peek};
use winnow::error::{ContextError, ErrMode, StrContext, StrContextValue};
use winnow::prelude::*;
use winnow::token::{any, take_till};

use self::sentence_followups::{
    PostParseFollowupResult, PreParseFollowupResult, is_still_lands_followup_sentence,
    previous_sentence_is_temporary_land_animation, run_post_parse_followup_registry,
    run_pre_parse_followup_registry,
};
use super::super::activation_and_restrictions::{
    contains_word_sequence, find_word_sequence_start, parse_choose_card_type_phrase_words,
    parse_target_player_choose_objects_clause, parse_you_choose_objects_clause,
};
use super::super::effect_ast_traversal::{
    for_each_nested_effects, for_each_nested_effects_mut, try_for_each_nested_effects_mut,
};
use super::super::grammar::filters::parse_spell_filter_with_grammar_entrypoint_lexed as parse_spell_filter_lexed;
use super::super::grammar::primitives::{self as grammar, TokenWordView};
use super::super::keyword_static::parse_where_x_value_clause;
use super::super::lexer::{LexStream, OwnedLexToken, TokenKind, split_lexed_sentences};
use super::super::object_filters::{
    is_comparison_or_delimiter, parse_object_filter, parse_object_filter_lexed,
};
use super::super::permission_helpers::{
    parse_until_end_of_turn_may_play_tagged_clause,
    parse_until_your_next_turn_may_play_tagged_clause,
};
use super::super::token_primitives::{
    LeadingMayActor, TurnDurationPhrase, find_index, find_window_by,
    parse_leading_may_action_lexed, parse_turn_duration_prefix, parse_value_comparison_tokens,
    slice_contains, slice_ends_with, slice_starts_with, str_contains, str_ends_with,
    str_starts_with, strip_leading_if_you_do_lexed, word_view_has_any_prefix, word_view_has_prefix,
};
use super::super::util::{
    helper_tag_for_tokens, is_article, mana_pips_from_token, parse_number, parse_subject,
    parse_target_phrase, span_from_tokens, token_index_for_word_index, trim_commas, words,
};
use super::super::value_helpers::parse_value_from_lexed;
use super::bundle_rules::{
    parse_exact_card_effect_bundle_lexed, parse_same_sentence_copy_and_may_cast_copy,
};
use super::consult_family;
use super::divvy::try_parse_divvy_sentence_sequence;
use super::looked_cards_family;
use super::sentence_helpers::*;
use super::sequence_rules::try_parse_registered_sequence_rule;
use super::zone_handlers::parse_exile_top_library_clause;
use super::{
    find_verb, parse_effect_sentence_lexed, parse_search_library_disjunction_filter,
    parse_token_copy_modifier_sentence, trim_edge_punctuation,
};
#[allow(unused_imports)]
use crate::cards::builders::{
    CardTextError, CarryContext, EffectAst, GrantedAbilityAst, IT_TAG, IfResultPredicate,
    InsteadSemantics, KeywordAction, LibraryBottomOrderAst, LibraryConsultModeAst,
    LibraryConsultStopRuleAst, PlayerAst, PredicateAst, ReturnControllerAst, SubjectAst, TagKey,
    TargetAst, TextSpan, TokenCopyFollowup, Verb, ZoneReplacementDurationAst,
};
use crate::effect::{ChoiceCount, Until, Value};
use crate::filter::Comparison;
use crate::target::{
    ChooseSpec, ObjectFilter, PlayerFilter, TaggedObjectConstraint, TaggedOpbjectRelation,
};
use crate::zone::Zone;
use std::cell::OnceCell;

mod sentence_followups;

pub(super) fn leading_may_actor_to_player(
    actor: LeadingMayActor,
    default_player: PlayerAst,
) -> PlayerAst {
    match actor {
        LeadingMayActor::You => PlayerAst::You,
        LeadingMayActor::ThatPlayer => PlayerAst::That,
        LeadingMayActor::Default => default_player,
    }
}

fn attach_copy_cost_reduction_to_effect(
    effect: &mut EffectAst,
    reduction: &crate::mana::ManaCost,
) -> bool {
    match effect {
        EffectAst::CastTagged {
            as_copy,
            cost_reduction,
            ..
        } if *as_copy => {
            *cost_reduction = Some(reduction.clone());
            true
        }
        _ => {
            let mut attached = false;
            for_each_nested_effects_mut(effect, true, |nested| {
                if attached {
                    return;
                }
                for nested_effect in nested.iter_mut().rev() {
                    if attach_copy_cost_reduction_to_effect(nested_effect, reduction) {
                        attached = true;
                        break;
                    }
                }
            });
            attached
        }
    }
}

fn attach_copy_cost_reduction_to_effects(
    effects: &mut [EffectAst],
    reduction: &crate::mana::ManaCost,
) -> bool {
    for effect in effects.iter_mut().rev() {
        if attach_copy_cost_reduction_to_effect(effect, reduction) {
            return true;
        }
    }
    false
}

const PRONOUN_TRIGGER_PREFIXES: &[&[&str]] = &[
    &["when", "it"],
    &["whenever", "it"],
    &["when", "they"],
    &["whenever", "they"],
];

fn normalize_parser_tokens(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    let mut normalized = tokens.to_vec();
    for token in &mut normalized {
        match token.kind {
            TokenKind::Word | TokenKind::Number | TokenKind::Tilde => {
                let replacement = token.parser_text().to_string();
                let _ = token.replace_word(replacement);
            }
            _ => {}
        }
    }
    normalized
}

#[derive(Debug, Clone)]
pub(super) struct ConsultSentenceParts {
    pub(super) effects: Vec<EffectAst>,
    pub(super) player: PlayerAst,
    pub(super) all_tag: TagKey,
    pub(super) match_tag: TagKey,
}

pub(super) struct ConsultCastClause {
    pub(super) caster: PlayerAst,
    pub(super) allow_land: bool,
    pub(super) timing: ConsultCastTiming,
    pub(super) cost: ConsultCastCost,
    pub(super) mana_value_condition: Option<ConsultCastManaValueCondition>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ConsultCastTiming {
    Immediate,
    UntilEndOfTurn,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ConsultCastCost {
    Normal,
    WithoutPayingManaCost,
    PayLifeEqualToManaValue,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct ConsultCastManaValueCondition {
    pub(super) operator: crate::effect::ValueComparisonOperator,
    pub(super) right: Value,
}

pub(super) fn parse_prefixed_top_of_your_library_count<T: Copy>(
    tokens: &[OwnedLexToken],
    prefixes: &[(&[&str], T)],
) -> Option<(T, u32)> {
    let tokens = trim_commas(tokens);
    let word_view = TokenWordView::new(&tokens);
    let (count_word_idx, marker) = prefixes.iter().find_map(|(prefix, marker)| {
        word_view_has_prefix(&word_view, prefix).then_some((prefix.len(), *marker))
    })?;
    let count_start = word_view.token_index_for_word_index(count_word_idx)?;
    let count_tokens = &tokens[count_start..];
    let (count, used) = parse_number(count_tokens)?;
    let tail_word_view = TokenWordView::new(&count_tokens[used..]);
    let tail_words = tail_word_view.word_refs();
    matches!(
        tail_words.as_slice(),
        ["card", "of", "your", "library"] | ["cards", "of", "your", "library"]
    )
    .then_some((marker, count))
}

pub(super) fn find_from_among_looked_cards_phrase(
    word_view: &TokenWordView<'_>,
) -> Option<(usize, usize)> {
    word_view
        .find_phrase_start(&["from", "among", "those", "cards"])
        .map(|idx| (idx, 4usize))
        .or_else(|| {
            word_view
                .find_phrase_start(&["from", "among", "them"])
                .map(|idx| (idx, 3usize))
        })
}

pub(super) fn parse_consult_traversal_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<ConsultSentenceParts>, CardTextError> {
    consult_family::parse_consult_traversal_sentence(tokens)
}

pub(super) fn parse_consult_remainder_order(words: &[&str]) -> Option<LibraryBottomOrderAst> {
    consult_family::parse_consult_remainder_order(words)
}

pub(super) fn consult_stop_rule_is_single_match(stop_rule: &LibraryConsultStopRuleAst) -> bool {
    matches!(
        stop_rule,
        LibraryConsultStopRuleAst::FirstMatch
            | LibraryConsultStopRuleAst::MatchCount(Value::Fixed(1))
    )
}

#[cfg(test)]
fn parse_consult_condition_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    consult_family::parse_consult_condition_value(tokens)
}

pub(super) fn parse_bargained_face_down_cast_mana_value_gate(
    tokens: &[OwnedLexToken],
) -> Result<Option<(crate::effect::ValueComparisonOperator, Value)>, CardTextError> {
    consult_family::parse_bargained_face_down_cast_mana_value_gate(tokens)
}

#[cfg(test)]
fn parse_consult_mana_value_condition_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ConsultCastManaValueCondition> {
    consult_family::parse_consult_mana_value_condition_tokens(tokens)
}

pub(super) fn parse_consult_cast_clause(tokens: &[OwnedLexToken]) -> Option<ConsultCastClause> {
    consult_family::parse_consult_cast_clause(tokens)
}

pub(super) fn parse_consult_bottom_remainder_clause(
    tokens: &[OwnedLexToken],
    mode: LibraryConsultModeAst,
) -> Option<LibraryBottomOrderAst> {
    consult_family::parse_consult_bottom_remainder_clause(tokens, mode)
}

pub(super) fn parse_if_declined_put_match_into_hand(
    tokens: &[OwnedLexToken],
    match_tag: TagKey,
) -> Option<Vec<EffectAst>> {
    consult_family::parse_if_declined_put_match_into_hand(tokens, match_tag)
}

pub(super) fn consult_cast_effects(
    clause: &ConsultCastClause,
    match_tag: TagKey,
) -> Result<Vec<EffectAst>, CardTextError> {
    consult_family::consult_cast_effects(clause, match_tag)
}

pub(crate) struct SentenceInput {
    lowered: OnceCell<Vec<OwnedLexToken>>,
    lexed: Vec<OwnedLexToken>,
}

impl SentenceInput {
    pub(crate) fn from_lexed(tokens: &[OwnedLexToken]) -> Self {
        Self {
            lowered: OnceCell::new(),
            lexed: tokens.to_vec(),
        }
    }

    pub(crate) fn lowered(&self) -> &[OwnedLexToken] {
        self.lowered
            .get_or_init(|| normalize_parser_tokens(&self.lexed))
            .as_slice()
    }

    pub(crate) fn lexed(&self) -> &[OwnedLexToken] {
        self.lexed.as_slice()
    }
}

struct SentenceDispatchState<'a> {
    effects: &'a mut Vec<EffectAst>,
    carried_context: &'a mut Option<CarryContext>,
}

struct SentenceParsePlan {
    tokens: Vec<OwnedLexToken>,
    wrap_if_result: Option<IfResultPredicate>,
    direct_effects: Option<Vec<EffectAst>>,
    consumed_sentences: usize,
}

impl SentenceParsePlan {
    fn new(tokens: Vec<OwnedLexToken>) -> Self {
        Self {
            tokens,
            wrap_if_result: None,
            direct_effects: None,
            consumed_sentences: 1,
        }
    }
}

fn future_zone_replacement_from_sentence_text(sentence_text: &str) -> Option<EffectAst> {
    let normalized = sentence_text.to_ascii_lowercase();
    let target = TargetAst::Tagged(TagKey::from(IT_TAG), None);

    if str_contains(&normalized, "countered this way")
        && str_contains(&normalized, "instead of putting it into")
        && str_contains(&normalized, "graveyard")
    {
        return Some(EffectAst::RegisterZoneReplacement {
            target,
            from_zone: Some(Zone::Stack),
            to_zone: Some(Zone::Graveyard),
            replacement_zone: Zone::Exile,
            duration: ZoneReplacementDurationAst::OneShot,
        });
    }

    if str_contains(&normalized, "would die this turn") && str_contains(&normalized, "exile") {
        return Some(EffectAst::RegisterZoneReplacement {
            target,
            from_zone: Some(Zone::Battlefield),
            to_zone: Some(Zone::Graveyard),
            replacement_zone: Zone::Exile,
            duration: ZoneReplacementDurationAst::OneShot,
        });
    }

    if str_contains(&normalized, "would be put into")
        && str_contains(&normalized, "graveyard")
        && str_contains(&normalized, "this turn")
        && str_contains(&normalized, "exile")
    {
        return Some(EffectAst::RegisterZoneReplacement {
            target,
            from_zone: None,
            to_zone: Some(Zone::Graveyard),
            replacement_zone: Zone::Exile,
            duration: ZoneReplacementDurationAst::OneShot,
        });
    }

    None
}

fn maybe_rewrite_future_zone_replacement_sentence(
    sentence_effects: &mut Vec<EffectAst>,
    sentence_text: &str,
) {
    if !matches!(
        classify_instead_followup_text(sentence_text),
        InsteadSemantics::FutureReplacement
    ) {
        return;
    }

    let Some(replacement) = future_zone_replacement_from_sentence_text(sentence_text) else {
        return;
    };

    if sentence_effects.iter().any(|effect| {
        matches!(
            effect,
            EffectAst::ExileInsteadOfGraveyardThisTurn { .. }
                | EffectAst::PreventNextTimeDamage { .. }
                | EffectAst::RedirectNextTimeDamageToSource { .. }
        )
    }) {
        return;
    }

    if sentence_effects.len() == 1 {
        if let Some(EffectAst::IfResult { effects, .. }) = sentence_effects.first_mut() {
            *effects = vec![replacement];
            return;
        }
        *sentence_effects = vec![replacement];
    }
}

pub(super) fn parse_top_cards_view_sentence(
    tokens: &[OwnedLexToken],
) -> Option<(PlayerAst, Value, bool)> {
    looked_cards_family::parse_top_cards_view_sentence(tokens)
}

pub(super) fn parse_reveal_top_count_put_all_matching_into_hand_rest_graveyard(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    super::sequence_rules::pairs::parse_reveal_top_count_put_all_matching_into_hand_rest_graveyard(
        &[
            SentenceInput::from_lexed(first),
            SentenceInput::from_lexed(second),
        ],
        0,
    )
}

#[cfg(test)]
pub(super) fn parse_top_cards_for_each_card_type_among_spells_put_matching_into_hand_rest_bottom(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
    third: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    super::sequence_rules::triples::parse_top_cards_for_each_card_type_among_spells_put_matching_into_hand_rest_bottom(
        &[
            SentenceInput::from_lexed(first),
            SentenceInput::from_lexed(second),
            SentenceInput::from_lexed(third),
        ],
        0,
    )
}

pub(super) fn parse_looked_card_choice_filter(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    looked_cards_family::parse_looked_card_choice_filter(tokens)
}

pub(super) fn parse_counted_looked_cards_into_your_hand_tokens(
    tokens: &[OwnedLexToken],
) -> Option<u32> {
    looked_cards_family::parse_counted_looked_cards_into_your_hand_tokens(tokens)
}

pub(super) fn parse_if_this_spell_was_kicked_counted_looked_cards_into_hand(
    tokens: &[OwnedLexToken],
) -> Option<u32> {
    looked_cards_family::parse_if_this_spell_was_kicked_counted_looked_cards_into_hand(tokens)
}

pub(super) fn parse_may_put_filtered_looked_card_onto_battlefield(
    tokens: &[OwnedLexToken],
) -> Result<Option<(PlayerAst, ObjectFilter, bool)>, CardTextError> {
    looked_cards_family::parse_may_put_filtered_looked_card_onto_battlefield(tokens)
}

pub(super) fn parse_may_put_filtered_looked_card_onto_battlefield_and_filtered_into_hand(
    tokens: &[OwnedLexToken],
) -> Result<Option<(PlayerAst, ObjectFilter, bool, ObjectFilter)>, CardTextError> {
    looked_cards_family::parse_may_put_filtered_looked_card_onto_battlefield_and_filtered_into_hand(
        tokens,
    )
}

pub(super) fn parse_if_you_dont_put_card_from_among_them_into_your_hand(
    tokens: &[OwnedLexToken],
) -> bool {
    looked_cards_family::parse_if_you_dont_put_card_from_among_them_into_your_hand(tokens)
}

pub(super) fn is_put_rest_on_bottom_of_library_sentence(tokens: &[OwnedLexToken]) -> bool {
    looked_cards_family::is_put_rest_on_bottom_of_library_sentence(tokens)
}

pub(super) fn parse_looked_card_reveal_filter(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    looked_cards_family::parse_looked_card_reveal_filter(tokens)
}

pub(super) fn parse_if_no_card_into_hand_this_way_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    consult_family::parse_if_no_card_into_hand_this_way_sentence(tokens)
}

pub(super) fn parse_if_you_dont_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    consult_family::parse_if_you_dont_sentence(tokens)
}

fn parse_effect_sentences_from_sentence_inputs(
    sentences: Vec<SentenceInput>,
) -> Result<Vec<EffectAst>, CardTextError> {
    if let Some(effects) = try_parse_divvy_sentence_sequence(&sentences)? {
        return Ok(effects);
    }

    let mut effects = Vec::new();
    let mut sentence_idx = 0usize;
    let mut carried_context: Option<CarryContext> = None;

    while sentence_idx < sentences.len() {
        let sentence = sentences[sentence_idx].lowered();
        if sentence.is_empty() {
            sentence_idx += 1;
            continue;
        }

        if let Some(mut matched) = try_parse_registered_sequence_rule(&sentences, sentence_idx)? {
            let stage = if let Some(feature_tag) = matched.feature_tag {
                format!(
                    "parse_effect_sentences:registry-hit:{}:{feature_tag}",
                    matched.name
                )
            } else {
                format!("parse_effect_sentences:registry-hit:{}", matched.name)
            };
            parser_trace(stage.as_str(), sentence);
            effects.append(&mut matched.effects);
            sentence_idx += matched.consumed_sentences;
            continue;
        }

        let mut sentence_tokens = strip_embedded_token_rules_text(sentence);
        sentence_tokens = trim_edge_punctuation(&sentence_tokens);
        if sentence_tokens.is_empty()
            || crate::cards::builders::compiler::token_word_refs(&sentence_tokens).is_empty()
        {
            sentence_idx += 1;
            continue;
        }
        sentence_tokens = rewrite_when_one_or_more_this_way_clause_prefix(&sentence_tokens);

        if is_still_lands_followup_sentence(&sentence_tokens)
            && previous_sentence_is_temporary_land_animation(&sentences, sentence_idx)
        {
            sentence_idx += 1;
            continue;
        }

        let mut parse_plan = {
            let mut state = SentenceDispatchState {
                effects: &mut effects,
                carried_context: &mut carried_context,
            };
            match run_pre_parse_followup_registry(
                &mut state,
                &sentences,
                sentence_idx,
                &sentence_tokens,
            )? {
                Some(PreParseFollowupResult::Handled { consumed_sentences }) => {
                    sentence_idx += consumed_sentences;
                    continue;
                }
                Some(PreParseFollowupResult::Plan(plan)) => plan,
                None => SentenceParsePlan::new(sentence_tokens.clone()),
            }
        };
        parser_trace("parse_effect_sentences:sentence", &parse_plan.tokens);

        let mut sentence_effects = if let Some(direct_effects) = parse_plan.direct_effects.take() {
            direct_effects
        } else if parse_plan.tokens.as_slice() == sentences[sentence_idx].lexed() {
            parse_effect_sentence_lexed(sentences[sentence_idx].lexed())?
        } else {
            parse_effect_sentence_lexed(&parse_plan.tokens)?
        };
        if let Some(predicate) = parse_plan.wrap_if_result {
            sentence_effects = vec![EffectAst::IfResult {
                predicate,
                effects: sentence_effects,
            }];
            carried_context = None;
        }
        if crate::cards::builders::compiler::token_word_refs(&parse_plan.tokens)
            .first()
            .copied()
            == Some("you")
        {
            carried_context = None;
        }
        if sentence_effects.is_empty()
            && !is_round_up_each_time_sentence(&parse_plan.tokens)
            && !is_nonsemantic_restriction_sentence(&parse_plan.tokens)
        {
            return Err(CardTextError::ParseError(format!(
                "sentence parsed to no semantic effects (clause: '{}')",
                crate::cards::builders::compiler::token_word_refs(&parse_plan.tokens).join(" ")
            )));
        }
        for effect in &mut sentence_effects {
            if let Some(context) = carried_context {
                maybe_apply_carried_player_with_clause(effect, context, &parse_plan.tokens);
            }
            if let Some(context) = explicit_player_for_carry(effect) {
                carried_context = Some(context);
            }
        }
        if sentence_effects.len() == 1
            && let Some(previous_effect) = effects.last()
            && let Some(effect) = sentence_effects.first_mut()
            && let EffectAst::IfResult {
                predicate,
                effects: if_result_effects,
            } = effect
        {
            if matches!(*predicate, IfResultPredicate::Did)
                && matches!(previous_effect, EffectAst::UnlessPays { .. })
            {
                *predicate = IfResultPredicate::DidNot;
            }
            if let Some(previous_target) = primary_damage_target_from_effect(previous_effect) {
                replace_it_damage_target_in_effects(
                    if_result_effects.as_mut_slice(),
                    &previous_target,
                );
            }
        }
        {
            let mut state = SentenceDispatchState {
                effects: &mut effects,
                carried_context: &mut carried_context,
            };
            if let Some(PostParseFollowupResult::Handled { consumed_sentences }) =
                run_post_parse_followup_registry(
                    &mut state,
                    &sentences,
                    sentence_idx,
                    &parse_plan.tokens,
                    &mut sentence_effects,
                )?
            {
                sentence_idx += consumed_sentences;
                continue;
            }
        }

        effects.extend(sentence_effects);
        sentence_idx += parse_plan.consumed_sentences;
    }

    if let Some(last_sentence) = sentences.last() {
        parser_trace("parse_effect_sentences:done", last_sentence.lowered());
    }
    Ok(effects)
}

pub(crate) fn parse_effect_sentences_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    if let Some(effects) = parse_exact_card_effect_bundle_lexed(tokens) {
        return Ok(effects);
    }

    let sentences = split_lexed_sentences(tokens)
        .into_iter()
        .map(SentenceInput::from_lexed)
        .collect::<Vec<_>>();
    parse_effect_sentences_from_sentence_inputs(sentences)
}

pub(crate) fn is_cant_be_regenerated_followup_sentence(tokens: &[OwnedLexToken]) -> bool {
    let words_storage = normalize_cant_words(tokens);
    let words = words_storage.iter().map(String::as_str).collect::<Vec<_>>();
    matches!(
        words.as_slice(),
        ["it", "cant", "be", "regenerated"]
            | ["it", "cant", "be", "regenerated", "this", "turn"]
            | ["they", "cant", "be", "regenerated"]
            | ["they", "cant", "be", "regenerated", "this", "turn"]
    )
}

pub(crate) fn is_cant_be_regenerated_this_turn_followup_sentence(tokens: &[OwnedLexToken]) -> bool {
    let words_storage = normalize_cant_words(tokens);
    let words = words_storage.iter().map(String::as_str).collect::<Vec<_>>();
    matches!(
        words.as_slice(),
        ["it", "cant", "be", "regenerated", "this", "turn"]
            | ["they", "cant", "be", "regenerated", "this", "turn"]
    )
}

pub(crate) fn apply_cant_be_regenerated_to_last_destroy_effect(
    effects: &mut Vec<EffectAst>,
) -> bool {
    let Some(last) = effects.last_mut() else {
        return false;
    };
    apply_cant_be_regenerated_to_effect(last)
}

pub(crate) fn apply_cant_be_regenerated_to_last_target_effect(
    effects: &mut Vec<EffectAst>,
) -> bool {
    let Some(previous_target) = effects.last().and_then(primary_target_from_effect) else {
        return false;
    };
    let Some(mut filter) = target_ast_to_object_filter(previous_target) else {
        return false;
    };
    if !filter
        .tagged_constraints
        .iter()
        .any(|constraint| constraint.tag.as_str() == IT_TAG)
    {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: TagKey::from(IT_TAG),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
    }

    effects.push(EffectAst::Cant {
        restriction: crate::effect::Restriction::be_regenerated(filter),
        duration: Until::EndOfTurn,
        condition: None,
    });
    true
}

fn apply_cant_be_regenerated_to_effect(effect: &mut EffectAst) -> bool {
    match effect {
        EffectAst::Destroy { target } => {
            let target = target.clone();
            *effect = EffectAst::DestroyNoRegeneration { target };
            true
        }
        EffectAst::DestroyAll { filter } => {
            let filter = filter.clone();
            *effect = EffectAst::DestroyAllNoRegeneration { filter };
            true
        }
        EffectAst::DestroyAllOfChosenColor { filter } => {
            let filter = filter.clone();
            *effect = EffectAst::DestroyAllOfChosenColorNoRegeneration { filter };
            true
        }
        _ => {
            let mut applied = false;
            for_each_nested_effects_mut(effect, true, |nested| {
                if !applied {
                    applied = apply_cant_be_regenerated_to_effects_tail(nested);
                }
            });
            applied
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::cards::builders::find_verb;
    use crate::effect::{Value, ValueComparisonOperator};
    use crate::filter::TaggedOpbjectRelation;
    use crate::target::PlayerFilter;

    use super::super::super::grammar::structure::split_lexed_sentences;
    use super::super::super::lexer::lex_line;
    use super::super::super::permission_helpers::parse_until_end_of_turn_may_play_tagged_clause;
    use super::super::super::util::{parse_subject, trim_commas};
    use super::super::zone_handlers::parse_exile_top_library_clause;
    use super::super::{parse_effect_chain, parse_effect_sentence_lexed};
    use super::{
        ConsultCastCost, ConsultCastTiming, Verb, parse_bargained_face_down_cast_mana_value_gate,
        parse_consult_cast_clause, parse_consult_condition_value,
        parse_consult_mana_value_condition_tokens,
        parse_counted_looked_cards_into_your_hand_tokens, parse_exact_card_effect_bundle_lexed,
        parse_if_no_card_into_hand_this_way_sentence, parse_if_you_dont_sentence,
        parse_looked_card_reveal_filter,
        parse_reveal_top_count_put_all_matching_into_hand_rest_graveyard,
        parse_top_cards_view_sentence,
    };

    #[test]
    fn consult_mana_value_condition_normalizes_spell_apostrophe_prefix() {
        let tokens = lex_line("if that spell's mana value is 3 or less", 0)
            .expect("rewrite lexer should classify consult mana-value condition");

        let parsed = parse_consult_mana_value_condition_tokens(&tokens)
            .expect("consult mana-value condition should parse");

        assert_eq!(parsed.operator, ValueComparisonOperator::LessThanOrEqual);
        assert_eq!(parsed.right, Value::Fixed(3));
    }

    #[test]
    fn consult_cast_clause_keeps_this_turn_remainder_without_word_view() {
        let tokens = lex_line("You may cast it this turn", 0)
            .expect("rewrite lexer should classify consult cast clause");

        let parsed = parse_consult_cast_clause(&tokens).expect("consult cast clause should parse");

        assert_eq!(parsed.caster, crate::cards::builders::PlayerAst::You);
        assert!(!parsed.allow_land);
        assert_eq!(parsed.timing, ConsultCastTiming::UntilEndOfTurn);
        assert_eq!(parsed.cost, ConsultCastCost::Normal);
        assert!(parsed.mana_value_condition.is_none());
    }

    #[test]
    fn looked_card_reveal_filter_strips_same_name_suffix_without_word_view() {
        let tokens = lex_line("card with that name", 0)
            .expect("rewrite lexer should classify looked-card reveal filter");

        let parsed = parse_looked_card_reveal_filter(&tokens)
            .expect("looked-card reveal filter should parse");

        assert_eq!(parsed.tagged_constraints.len(), 1);
        assert_eq!(
            parsed.tagged_constraints[0].relation,
            TaggedOpbjectRelation::SameNameAsTagged
        );
    }

    #[test]
    fn consult_condition_value_reads_source_power_from_token_view() {
        let tokens = lex_line("this's power", 0)
            .expect("rewrite lexer should classify consult value clause");

        let parsed =
            parse_consult_condition_value(&tokens).expect("consult value clause should parse");

        assert_eq!(parsed, Value::SourcePower);
    }

    #[test]
    fn top_cards_view_sentence_reads_reveal_count_from_token_view() {
        let tokens = lex_line("Reveal the top two cards of your library", 0)
            .expect("rewrite lexer should classify top-cards reveal clause");

        let parsed =
            parse_top_cards_view_sentence(&tokens).expect("top-cards reveal clause should parse");

        assert_eq!(
            parsed,
            (
                crate::cards::builders::PlayerAst::You,
                Value::Fixed(2),
                true
            )
        );
    }

    #[test]
    fn counted_looked_cards_into_hand_tokens_parse_those_cards_instead() {
        let tokens = lex_line("Put two of those cards into your hand instead", 0)
            .expect("rewrite lexer should classify counted looked-cards clause");

        let parsed = parse_counted_looked_cards_into_your_hand_tokens(&tokens)
            .expect("counted looked-cards clause should parse");

        assert_eq!(parsed, 2);
    }

    #[test]
    fn reveal_top_put_all_matching_into_hand_rest_graveyard_stays_token_aware() {
        let first = lex_line("Reveal the top three cards of your library", 0)
            .expect("rewrite lexer should classify reveal-top clause");
        let second = lex_line(
            "Put all land cards revealed this way into your hand and the rest into your graveyard",
            0,
        )
        .expect("rewrite lexer should classify reveal follow-up clause");

        let parsed =
            parse_reveal_top_count_put_all_matching_into_hand_rest_graveyard(&first, &second)
                .expect("reveal-top follow-up parser should not error")
                .expect("reveal-top follow-up should parse");

        assert!(matches!(
            parsed.as_slice(),
            [
                crate::cards::builders::EffectAst::RevealTopPutMatchingIntoHandRestIntoGraveyard {
                    player: crate::cards::builders::PlayerAst::You,
                    count: 3,
                    ..
                }
            ]
        ));
    }

    #[test]
    fn parse_turnabout_mass_tap_sentence_uses_tap_or_untap_all_ast() {
        let tokens = lex_line(
            "Tap all untapped permanents of the chosen type target player controls, or untap all tapped permanents of that type that player controls",
            0,
        )
        .expect("rewrite lexer should classify turnabout mass-tap clause");

        let parsed =
            parse_effect_sentence_lexed(&tokens).expect("turnabout mass-tap clause should parse");

        let [
            crate::cards::builders::EffectAst::TapOrUntapAll {
                tap_filter,
                untap_filter,
            },
        ] = parsed.as_slice()
        else {
            panic!("expected shared tap-or-untap-all ast, got {parsed:?}");
        };

        assert_eq!(tap_filter.controller, Some(PlayerFilter::target_player()));
        assert_eq!(untap_filter.controller, Some(PlayerFilter::target_player()));
        assert!(tap_filter.chosen_creature_type, "{tap_filter:?}");
        assert!(untap_filter.chosen_creature_type, "{untap_filter:?}");
    }

    #[test]
    fn choose_then_for_each_of_those_bundle_builds_for_each_tagged_loop() {
        let tokens = lex_line(
            "Choose five permanents you control. For each of those permanents, you may search your library for a card with the same name as that permanent. Put those cards onto the battlefield tapped, then shuffle.",
            0,
        )
        .expect("rewrite lexer should classify choose/for-each bundle");

        let parsed = parse_exact_card_effect_bundle_lexed(&tokens)
            .expect("choose/for-each bundle should parse");

        assert!(matches!(
            parsed.as_slice(),
            [
                crate::cards::builders::EffectAst::ChooseObjects { .. },
                crate::cards::builders::EffectAst::ForEachTagged { .. },
                ..,
            ]
        ));
    }

    #[test]
    fn subject_first_exile_top_library_then_play_bundle_parses_directly() {
        let tokens = lex_line(
            "That player exiles the top two cards of their library. Until end of turn, you may play those cards without paying their mana costs.",
            0,
        )
        .expect("rewrite lexer should classify Fallen Shinobi style bundle");

        let sentences = split_lexed_sentences(&tokens);
        assert_eq!(sentences.len(), 2, "{sentences:#?}");
        let first = sentences[0];
        let second = sentences[1];

        let (verb, verb_idx) = find_verb(first).expect("first sentence should have a verb");
        assert_eq!(verb, Verb::Exile);
        let subject = parse_subject(&trim_commas(&first[..verb_idx]));
        let exile_tokens = trim_commas(&first[verb_idx + 1..]);
        let exile_effect = parse_exile_top_library_clause(&exile_tokens, Some(subject));
        assert!(exile_effect.is_some(), "expected exile clause to parse");

        let permission_effect = parse_until_end_of_turn_may_play_tagged_clause(second)
            .expect("permission clause should not error");
        assert!(
            permission_effect.is_some(),
            "expected permission clause to parse"
        );

        let parsed = parse_exact_card_effect_bundle_lexed(&tokens)
            .expect("subject-first exile/play bundle should parse directly");

        let debug = format!("{parsed:#?}").to_ascii_lowercase();
        assert!(
            debug.contains("exiletopoflibrary"),
            "expected exile-top-library effect, got {debug}"
        );
        assert!(
            debug.contains("grantplaytaggeduntilendofturn"),
            "expected play permission effect, got {debug}"
        );
    }

    #[test]
    fn exile_then_source_leaves_return_bundle_collapses_to_until_source_leaves() {
        let tokens = lex_line(
            "If there are two or more other creatures on the battlefield, exile that creature. Return that card to the battlefield under its owner's control when this artifact leaves the battlefield.",
            0,
        )
        .expect("rewrite lexer should classify source-leaves exile bundle");

        let parsed = parse_exact_card_effect_bundle_lexed(&tokens)
            .or_else(|| parse_effect_chain(&tokens).ok())
            .expect("source-leaves exile bundle should parse through a supported sentence path");

        let debug = format!("{parsed:#?}").to_ascii_lowercase();
        assert!(
            debug.contains("exileuntilsourceleaves")
                || (debug.contains("exile {") && debug.contains("__it__")),
            "expected source-leaves exile bundle or equivalent tagged exile scaffold, got {debug}"
        );
        assert!(
            !debug.contains("returnfromgraveyardtobattlefield"),
            "expected source-leaves bundle not to lower into graveyard-return, got {debug}"
        );
    }

    #[test]
    fn reveal_top_then_for_each_card_type_bundle_parses_directly() {
        let tokens = lex_line(
            "Reveal the top five cards of your library. For each card type among noncreature spells you've cast this turn, you may put a card of that type from among the revealed cards into your hand. Put the rest on the bottom of your library in a random order.",
            0,
        )
        .expect("rewrite lexer should classify Hurkyl reveal bundle");

        let sentences = split_lexed_sentences(&tokens);
        assert_eq!(sentences.len(), 3, "{sentences:#?}");

        let parsed =
            super::parse_top_cards_for_each_card_type_among_spells_put_matching_into_hand_rest_bottom(
                sentences[0],
                sentences[1],
                sentences[2],
            )
            .expect("Hurkyl reveal bundle helper should not error")
            .expect("Hurkyl reveal bundle helper should parse");

        assert!(matches!(
            parsed.as_slice(),
            [
                crate::cards::builders::EffectAst::LookAtTopCards { .. },
                crate::cards::builders::EffectAst::RevealTagged { .. },
                crate::cards::builders::EffectAst::ChooseFromLookedCardsForEachCardTypeAmongSpellsCastThisTurnIntoHandRestOnBottomOfLibrary { .. },
            ]
        ));
    }

    #[test]
    fn bargained_face_down_cast_gate_parses_with_winnow_clause_parser() {
        let tokens = lex_line(
            "If this spell was bargained, you may cast the exiled card without paying its mana cost if that spell's mana value is 3 or less",
            0,
        )
        .expect("rewrite lexer should classify bargained face-down cast clause");

        let parsed = parse_bargained_face_down_cast_mana_value_gate(&tokens)
            .expect("bargained face-down cast clause should not error")
            .expect("bargained face-down cast clause should parse");

        assert_eq!(parsed.0, ValueComparisonOperator::LessThanOrEqual);
        assert_eq!(parsed.1, Value::Fixed(3));
    }

    #[test]
    fn if_no_card_into_hand_clause_accepts_article_before_card() {
        let tokens = lex_line(
            "If you didn't put a card into your hand this way, draw a card",
            0,
        )
        .expect("rewrite lexer should classify if-no-card clause");

        let parsed = parse_if_no_card_into_hand_this_way_sentence(&tokens)
            .expect("if-no-card clause should not error")
            .expect("if-no-card clause should parse");

        assert_eq!(parsed.len(), 1);
    }

    #[test]
    fn if_you_dont_clause_reports_missing_comma_after_matched_prefix() {
        let tokens = lex_line("If you don't draw a card", 0)
            .expect("rewrite lexer should classify if-you-don't clause");

        let err = parse_if_you_dont_sentence(&tokens)
            .expect_err("matched if-you-don't clause without comma should cut");

        assert!(
            err.to_string().contains("comma after if-you-don't clause"),
            "unexpected error: {err}"
        );
    }
}

fn apply_cant_be_regenerated_to_effects_tail(effects: &mut [EffectAst]) -> bool {
    for effect in effects.iter_mut().rev() {
        if apply_cant_be_regenerated_to_effect(effect) {
            return true;
        }
    }
    false
}

pub(crate) fn primary_damage_target_from_effect(effect: &EffectAst) -> Option<TargetAst> {
    match effect {
        EffectAst::DealDamage { target, .. } | EffectAst::DealDamageEqualToPower { target, .. } => {
            Some(target.clone())
        }
        _ => {
            let mut found = None;
            for_each_nested_effects(effect, false, |nested| {
                if found.is_none() {
                    found = nested.iter().find_map(primary_damage_target_from_effect);
                }
            });
            found
        }
    }
}

pub(crate) fn primary_target_from_effect(effect: &EffectAst) -> Option<TargetAst> {
    match effect {
        EffectAst::DealDamage { target, .. }
        | EffectAst::DealDamageEqualToPower { target, .. }
        | EffectAst::Counter { target }
        | EffectAst::CounterUnlessPays { target, .. }
        | EffectAst::Explore { target }
        | EffectAst::Connive { target }
        | EffectAst::Detain { target }
        | EffectAst::Goad { target }
        | EffectAst::Tap { target }
        | EffectAst::Untap { target }
        | EffectAst::RemoveFromCombat { target }
        | EffectAst::TapOrUntap { target }
        | EffectAst::Destroy { target }
        | EffectAst::DestroyNoRegeneration { target }
        | EffectAst::Exile { target, .. }
        | EffectAst::ExileWhenSourceLeaves { target }
        | EffectAst::SacrificeSourceWhenLeaves { target }
        | EffectAst::ExileUntilSourceLeaves { target, .. }
        | EffectAst::LookAtHand { target }
        | EffectAst::Transform { target }
        | EffectAst::Convert { target }
        | EffectAst::Flip { target }
        | EffectAst::Regenerate { target }
        | EffectAst::PhaseOut { target }
        | EffectAst::TargetOnly { target }
        | EffectAst::ReturnToHand { target, .. }
        | EffectAst::ReturnToBattlefield { target, .. }
        | EffectAst::MoveToZone { target, .. }
        | EffectAst::PutCounters { target, .. }
        | EffectAst::PutOrRemoveCounters { target, .. }
        | EffectAst::RemoveUpToAnyCounters { target, .. }
        | EffectAst::Pump { target, .. }
        | EffectAst::GrantAbilitiesToTarget { target, .. }
        | EffectAst::GrantToTarget { target, .. }
        | EffectAst::GrantAbilitiesChoiceToTarget { target, .. }
        | EffectAst::GrantProtectionChoice { target, .. }
        | EffectAst::PreventDamage { target, .. }
        | EffectAst::PreventAllDamageToTarget { target, .. }
        | EffectAst::PreventDamageToTargetPutCounters { target, .. }
        | EffectAst::PreventAllCombatDamageFromSource { source: target, .. }
        | EffectAst::RedirectNextDamageFromSourceToTarget { target, .. }
        | EffectAst::RedirectNextTimeDamageToSource { target, .. }
        | EffectAst::GainControl { target, .. } => Some(target.clone()),
        _ => {
            let mut found = None;
            for_each_nested_effects(effect, false, |nested| {
                if found.is_none() {
                    found = nested.iter().find_map(primary_target_from_effect);
                }
            });
            found
        }
    }
}

pub(crate) fn replace_it_damage_target_in_effects(effects: &mut [EffectAst], target: &TargetAst) {
    for effect in effects {
        replace_it_damage_target(effect, target);
    }
}

pub(crate) fn replace_it_target_in_effects(effects: &mut [EffectAst], target: &TargetAst) {
    for effect in effects {
        replace_it_target(effect, target);
    }
}

pub(crate) fn is_placeholder_damage_target(target: &TargetAst) -> bool {
    matches!(
        target,
        TargetAst::PlayerOrPlaneswalker(PlayerFilter::Any, None)
    )
}

pub(crate) fn replace_placeholder_damage_target_in_effects(
    effects: &mut [EffectAst],
    target: &TargetAst,
) {
    for effect in effects {
        replace_placeholder_damage_target(effect, target);
    }
}

pub(crate) fn replace_placeholder_damage_target(effect: &mut EffectAst, target: &TargetAst) {
    match effect {
        EffectAst::DealDamage {
            target: damage_target,
            ..
        }
        | EffectAst::DealDamageEqualToPower {
            target: damage_target,
            ..
        } => {
            if is_placeholder_damage_target(damage_target) {
                *damage_target = target.clone();
            }
        }
        _ => for_each_nested_effects_mut(effect, true, |nested| {
            replace_placeholder_damage_target_in_effects(nested, target);
        }),
    }
}

pub(crate) fn replace_unbound_x_in_damage_effects(
    effects: &mut [EffectAst],
    replacement: &Value,
    clause: &str,
) -> Result<(), CardTextError> {
    for effect in effects {
        replace_unbound_x_in_damage_effect(effect, replacement, clause)?;
    }
    Ok(())
}

pub(crate) fn replace_unbound_x_in_damage_effect(
    effect: &mut EffectAst,
    replacement: &Value,
    clause: &str,
) -> Result<(), CardTextError> {
    match effect {
        EffectAst::DealDamage { amount, .. }
        | EffectAst::DealDamageEach { amount, .. }
        | EffectAst::GainLife { amount, .. }
        | EffectAst::LoseLife { amount, .. } => {
            if value_contains_unbound_x(amount) {
                *amount = replace_unbound_x_with_value(amount.clone(), replacement, clause)?;
            }
        }
        _ => {
            try_for_each_nested_effects_mut(effect, true, |nested| {
                replace_unbound_x_in_damage_effects(nested, replacement, clause)
            })?;
        }
    }
    Ok(())
}

pub(crate) fn replace_unbound_x_in_effects_anywhere(
    effects: &mut [EffectAst],
    replacement: &Value,
    clause: &str,
) -> Result<(), CardTextError> {
    for effect in effects {
        replace_unbound_x_in_effect_anywhere(effect, replacement, clause)?;
    }
    Ok(())
}

pub(crate) fn replace_unbound_x_in_effect_anywhere(
    effect: &mut EffectAst,
    replacement: &Value,
    clause: &str,
) -> Result<(), CardTextError> {
    fn replace_in_comparison(
        comparison: &mut crate::filter::Comparison,
        replacement: &Value,
        clause: &str,
    ) -> Result<(), CardTextError> {
        use crate::filter::Comparison;

        let value = match comparison {
            Comparison::EqualExpr(value)
            | Comparison::NotEqualExpr(value)
            | Comparison::LessThanExpr(value)
            | Comparison::LessThanOrEqualExpr(value)
            | Comparison::GreaterThanExpr(value)
            | Comparison::GreaterThanOrEqualExpr(value) => value,
            _ => return Ok(()),
        };

        if value_contains_unbound_x(value) {
            **value = replace_unbound_x_with_value((**value).clone(), replacement, clause)?;
        }
        Ok(())
    }

    fn replace_in_filter(
        filter: &mut ObjectFilter,
        replacement: &Value,
        clause: &str,
    ) -> Result<(), CardTextError> {
        if let Some(power) = filter.power.as_mut() {
            replace_in_comparison(power, replacement, clause)?;
        }
        if let Some(toughness) = filter.toughness.as_mut() {
            replace_in_comparison(toughness, replacement, clause)?;
        }
        if let Some(mana_value) = filter.mana_value.as_mut() {
            replace_in_comparison(mana_value, replacement, clause)?;
        }
        if let Some(targets_object) = filter.targets_object.as_mut() {
            replace_in_filter(targets_object, replacement, clause)?;
        }
        if let Some(targets_only_object) = filter.targets_only_object.as_mut() {
            replace_in_filter(targets_only_object, replacement, clause)?;
        }
        for nested in &mut filter.any_of {
            replace_in_filter(nested, replacement, clause)?;
        }
        Ok(())
    }

    fn replace_value(
        value: &mut Value,
        replacement: &Value,
        clause: &str,
    ) -> Result<(), CardTextError> {
        if value_contains_unbound_x(value) {
            *value = replace_unbound_x_with_value(value.clone(), replacement, clause)?;
        }
        Ok(())
    }

    match effect {
        EffectAst::DealDamage { amount, .. }
        | EffectAst::DealDamageEach { amount, .. }
        | EffectAst::Draw { count: amount, .. }
        | EffectAst::LoseLife { amount, .. }
        | EffectAst::GainLife { amount, .. }
        | EffectAst::PreventDamage { amount, .. }
        | EffectAst::PreventDamageEach { amount, .. }
        | EffectAst::PutCounters { count: amount, .. }
        | EffectAst::PutCountersAll { count: amount, .. }
        | EffectAst::Mill { count: amount, .. }
        | EffectAst::Discard { count: amount, .. }
        | EffectAst::Scry { count: amount, .. }
        | EffectAst::Surveil { count: amount, .. }
        | EffectAst::Discover { count: amount, .. }
        | EffectAst::LookAtTopCards { count: amount, .. }
        | EffectAst::PayEnergy { amount, .. }
        | EffectAst::CopySpell { count: amount, .. }
        | EffectAst::SetLifeTotal { amount, .. }
        | EffectAst::Monstrosity { amount } => {
            replace_value(amount, replacement, clause)?;
        }
        EffectAst::PreventDamageToTargetPutCounters {
            amount: Some(amount),
            ..
        } => {
            replace_value(amount, replacement, clause)?;
        }
        EffectAst::Pump {
            power, toughness, ..
        }
        | EffectAst::SetBasePowerToughness {
            power, toughness, ..
        }
        | EffectAst::BecomeBasePtCreature {
            power, toughness, ..
        }
        | EffectAst::PumpAll {
            power, toughness, ..
        } => {
            replace_value(power, replacement, clause)?;
            replace_value(toughness, replacement, clause)?;
        }
        EffectAst::SetBasePower { power, .. } => {
            replace_value(power, replacement, clause)?;
        }
        EffectAst::PutOrRemoveCounters {
            put_count,
            remove_count,
            ..
        } => {
            replace_value(put_count, replacement, clause)?;
            replace_value(remove_count, replacement, clause)?;
        }
        EffectAst::RemoveUpToAnyCounters { amount, .. } => {
            replace_value(amount, replacement, clause)?;
        }
        EffectAst::AddManaScaled { amount, .. }
        | EffectAst::AddManaAnyColor { amount, .. }
        | EffectAst::AddManaAnyOneColor { amount, .. }
        | EffectAst::AddManaChosenColor { amount, .. }
        | EffectAst::AddManaFromLandCouldProduce { amount, .. }
        | EffectAst::AddManaCommanderIdentity { amount, .. } => {
            replace_value(amount, replacement, clause)?;
        }
        EffectAst::CreateTokenCopy { count, .. }
        | EffectAst::CreateTokenCopyFromSource { count, .. } => {
            replace_value(count, replacement, clause)?;
        }
        EffectAst::SearchLibrary {
            filter,
            count_value,
            ..
        } => {
            replace_in_filter(filter, replacement, clause)?;
            if let Some(count_value) = count_value.as_mut() {
                replace_value(count_value, replacement, clause)?;
            }
        }
        EffectAst::CreateTokenWithMods {
            count,
            dynamic_power_toughness,
            ..
        } => {
            replace_value(count, replacement, clause)?;
            if let Some((power, toughness)) = dynamic_power_toughness {
                replace_value(power, replacement, clause)?;
                replace_value(toughness, replacement, clause)?;
            }
        }
        EffectAst::CounterUnlessPays {
            life,
            additional_generic,
            ..
        } => {
            if let Some(life) = life.as_mut() {
                replace_value(life, replacement, clause)?;
            }
            if let Some(generic) = additional_generic.as_mut() {
                replace_value(generic, replacement, clause)?;
            }
        }
        EffectAst::PumpForEach { count, .. } => {
            replace_value(count, replacement, clause)?;
        }
        _ => {
            try_for_each_nested_effects_mut(effect, true, |nested| {
                replace_unbound_x_in_effects_anywhere(nested, replacement, clause)
            })?;
        }
    }
    Ok(())
}

pub(crate) fn apply_where_x_to_damage_amounts(
    tokens: &[OwnedLexToken],
    effects: &mut [EffectAst],
) -> Result<(), CardTextError> {
    let clause_words = crate::cards::builders::compiler::token_word_refs(tokens);
    let has_deal_x = find_window_by(&clause_words, 3, |window| {
        (window[0] == "deal" || window[0] == "deals") && window[1] == "x" && window[2] == "damage"
    })
    .is_some();
    let has_x_life = find_window_by(&clause_words, 3, |window| {
        (window[0] == "gain" || window[0] == "gains" || window[0] == "lose" || window[0] == "loses")
            && window[1] == "x"
            && window[2] == "life"
    })
    .is_some();
    if !has_deal_x && !has_x_life {
        return Ok(());
    }
    let Some(where_idx) = find_word_sequence_start(&clause_words, &["where", "x", "is"]) else {
        return Ok(());
    };
    let Some(where_token_idx) = token_index_for_word_index(tokens, where_idx) else {
        return Ok(());
    };
    let where_tokens = &tokens[where_token_idx..];
    let Some(where_value) = parse_where_x_value_clause(where_tokens) else {
        return Ok(());
    };
    replace_unbound_x_in_damage_effects(effects, &where_value, &clause_words.join(" "))
}

pub(crate) fn replace_it_damage_target(effect: &mut EffectAst, target: &TargetAst) {
    match effect {
        EffectAst::DealDamage {
            target: damage_target,
            ..
        } => {
            if target_references_it(damage_target) {
                *damage_target = target.clone();
            }
        }
        _ => for_each_nested_effects_mut(effect, true, |nested| {
            replace_it_damage_target_in_effects(nested, target);
        }),
    }
}

pub(crate) fn replace_it_target(effect: &mut EffectAst, target: &TargetAst) {
    match effect {
        EffectAst::DealDamage {
            target: effect_target,
            ..
        }
        | EffectAst::DealDamageEqualToPower {
            target: effect_target,
            ..
        }
        | EffectAst::Counter {
            target: effect_target,
        }
        | EffectAst::CounterUnlessPays {
            target: effect_target,
            ..
        }
        | EffectAst::Explore {
            target: effect_target,
        }
        | EffectAst::Connive {
            target: effect_target,
        }
        | EffectAst::Detain {
            target: effect_target,
        }
        | EffectAst::Goad {
            target: effect_target,
        }
        | EffectAst::Tap {
            target: effect_target,
        }
        | EffectAst::Untap {
            target: effect_target,
        }
        | EffectAst::PhaseOut {
            target: effect_target,
        }
        | EffectAst::RemoveFromCombat {
            target: effect_target,
        }
        | EffectAst::TapOrUntap {
            target: effect_target,
        }
        | EffectAst::Destroy {
            target: effect_target,
        }
        | EffectAst::DestroyNoRegeneration {
            target: effect_target,
        }
        | EffectAst::Exile {
            target: effect_target,
            ..
        }
        | EffectAst::ExileWhenSourceLeaves {
            target: effect_target,
        }
        | EffectAst::SacrificeSourceWhenLeaves {
            target: effect_target,
        }
        | EffectAst::ExileUntilSourceLeaves {
            target: effect_target,
            ..
        }
        | EffectAst::LookAtHand {
            target: effect_target,
        }
        | EffectAst::Transform {
            target: effect_target,
        }
        | EffectAst::Convert {
            target: effect_target,
        }
        | EffectAst::Flip {
            target: effect_target,
        }
        | EffectAst::Regenerate {
            target: effect_target,
        }
        | EffectAst::TargetOnly {
            target: effect_target,
        }
        | EffectAst::ReturnToHand {
            target: effect_target,
            ..
        }
        | EffectAst::ReturnToBattlefield {
            target: effect_target,
            ..
        }
        | EffectAst::MoveToZone {
            target: effect_target,
            ..
        }
        | EffectAst::PutCounters {
            target: effect_target,
            ..
        }
        | EffectAst::PutOrRemoveCounters {
            target: effect_target,
            ..
        }
        | EffectAst::RemoveUpToAnyCounters {
            target: effect_target,
            ..
        }
        | EffectAst::Pump {
            target: effect_target,
            ..
        }
        | EffectAst::GrantAbilitiesToTarget {
            target: effect_target,
            ..
        }
        | EffectAst::GrantToTarget {
            target: effect_target,
            ..
        }
        | EffectAst::GrantAbilitiesChoiceToTarget {
            target: effect_target,
            ..
        }
        | EffectAst::GrantProtectionChoice {
            target: effect_target,
            ..
        }
        | EffectAst::PreventDamage {
            target: effect_target,
            ..
        }
        | EffectAst::PreventAllDamageToTarget {
            target: effect_target,
            ..
        }
        | EffectAst::PreventDamageToTargetPutCounters {
            target: effect_target,
            ..
        }
        | EffectAst::PreventAllCombatDamageFromSource {
            source: effect_target,
            ..
        }
        | EffectAst::RedirectNextDamageFromSourceToTarget {
            target: effect_target,
            ..
        }
        | EffectAst::RedirectNextTimeDamageToSource {
            target: effect_target,
            ..
        }
        | EffectAst::GainControl {
            target: effect_target,
            ..
        } => {
            if target_references_it(effect_target) {
                *effect_target = target.clone();
            }
        }
        _ => for_each_nested_effects_mut(effect, true, |nested| {
            replace_it_target_in_effects(nested, target);
        }),
    }
}

pub(crate) fn target_references_it(target: &TargetAst) -> bool {
    match target {
        TargetAst::Tagged(tag, _) => tag.as_str() == IT_TAG,
        TargetAst::Object(filter, _, _) => filter
            .tagged_constraints
            .iter()
            .any(|constraint| constraint.tag.as_str() == IT_TAG),
        TargetAst::WithCount(inner, _) => target_references_it(inner),
        _ => false,
    }
}

pub(crate) fn is_that_turn_end_step_sentence(tokens: &[OwnedLexToken]) -> bool {
    grammar::words_match_prefix(
        tokens,
        &[
            "at",
            "the",
            "beginning",
            "of",
            "that",
            "turn",
            "end",
            "step",
        ],
    )
    .is_some()
        || grammar::words_match_prefix(
            tokens,
            &[
                "at",
                "the",
                "beginning",
                "of",
                "that",
                "turns",
                "end",
                "step",
            ],
        )
        .is_some()
}

pub(crate) fn most_recent_extra_turn_player(effects: &[EffectAst]) -> Option<PlayerAst> {
    effects.iter().rev().find_map(|effect| {
        if let EffectAst::ExtraTurnAfterTurn { player, .. } = effect {
            Some(*player)
        } else {
            None
        }
    })
}

pub(crate) fn rewrite_when_one_or_more_this_way_clause_prefix(
    tokens: &[OwnedLexToken],
) -> Vec<OwnedLexToken> {
    // Generic "When one or more ... this way, ..." follow-ups are semantically
    // "If you do, ..." against the immediately previous effect result.
    let has_this_way = grammar::contains_phrase(tokens, &["this", "way"]);
    if (grammar::strip_lexed_prefix_phrase(tokens, &["when", "one", "or", "more"]).is_some()
        || grammar::strip_lexed_prefix_phrase(tokens, &["whenever", "one", "or", "more"]).is_some())
        && has_this_way
    {
        let Some((_before, after)) =
            grammar::split_lexed_once_on_delimiter(tokens, TokenKind::Comma)
        else {
            return tokens.to_vec();
        };
        let mut rewritten = Vec::new();

        let mut if_token = tokens[0].clone();
        if_token.replace_word("if");
        rewritten.push(if_token);

        let mut you_token = tokens.get(1).cloned().unwrap_or_else(|| tokens[0].clone());
        you_token.replace_word("you");
        rewritten.push(you_token);

        let mut do_token = tokens.get(2).cloned().unwrap_or_else(|| tokens[0].clone());
        do_token.replace_word("do");
        rewritten.push(do_token);

        rewritten.push(OwnedLexToken::word(",".to_string(), tokens[0].span()));
        rewritten.extend_from_slice(after);
        return rewritten;
    }

    tokens.to_vec()
}

pub(crate) fn strip_otherwise_sentence_prefix(
    tokens: &[OwnedLexToken],
) -> Option<Vec<OwnedLexToken>> {
    if !tokens
        .first()
        .is_some_and(|token| token.is_word("otherwise"))
    {
        return None;
    }

    let mut idx = 1usize;
    while tokens.get(idx).is_some_and(OwnedLexToken::is_comma) {
        idx += 1;
    }
    if tokens.get(idx).is_some_and(|token| token.is_word("then")) {
        idx += 1;
    }
    while tokens.get(idx).is_some_and(OwnedLexToken::is_comma) {
        idx += 1;
    }

    let remainder = trim_commas(&tokens[idx..]);
    if remainder.is_empty() {
        None
    } else {
        Some(remainder)
    }
}

pub(crate) fn rewrite_otherwise_referential_subject(
    tokens: Vec<OwnedLexToken>,
) -> Vec<OwnedLexToken> {
    let clause_words = crate::cards::builders::compiler::token_word_refs(&tokens);
    let is_referential_get = clause_words.len() >= 3
        && clause_words[0] == "that"
        && matches!(clause_words[1], "creature" | "permanent")
        && matches!(clause_words[2], "gets" | "get" | "gains" | "gain");
    if !is_referential_get {
        return tokens;
    }

    let mut rewritten = tokens;
    if let Some(first) = rewritten.get_mut(0) {
        first.replace_word("target");
    }
    rewritten
}

pub(crate) fn is_nonsemantic_restriction_sentence(tokens: &[OwnedLexToken]) -> bool {
    is_activate_only_restriction_sentence(tokens) || is_trigger_only_restriction_sentence(tokens)
}

fn token_copy_followup_container_effects_mut(
    effect: &mut EffectAst,
) -> Option<&mut Vec<EffectAst>> {
    match effect {
        EffectAst::May { effects }
        | EffectAst::MayByPlayer { effects, .. }
        | EffectAst::IfResult { effects, .. }
        | EffectAst::WhenResult { effects, .. }
        | EffectAst::ResolvedIfResult { effects, .. }
        | EffectAst::ResolvedWhenResult { effects, .. }
        | EffectAst::ForEachOpponent { effects }
        | EffectAst::ForEachPlayersFiltered { effects, .. }
        | EffectAst::ForEachPlayer { effects }
        | EffectAst::ForEachTargetPlayers { effects, .. }
        | EffectAst::ForEachObject { effects, .. }
        | EffectAst::ForEachTagged { effects, .. }
        | EffectAst::ForEachOpponentDoesNot { effects, .. }
        | EffectAst::ForEachPlayerDoesNot { effects, .. }
        | EffectAst::ForEachOpponentDid { effects, .. }
        | EffectAst::ForEachPlayerDid { effects, .. }
        | EffectAst::ForEachTaggedPlayer { effects, .. }
        | EffectAst::RepeatProcess { effects, .. }
        | EffectAst::DelayedUntilNextEndStep { effects, .. }
        | EffectAst::DelayedUntilNextUpkeep { effects, .. }
        | EffectAst::DelayedUntilNextDrawStep { effects, .. }
        | EffectAst::DelayedUntilEndStepOfExtraTurn { effects, .. }
        | EffectAst::DelayedUntilEndOfCombat { effects }
        | EffectAst::DelayedTriggerThisTurn { effects, .. }
        | EffectAst::DelayedWhenLastObjectDiesThisTurn { effects, .. }
        | EffectAst::VoteOption { effects, .. } => Some(effects),
        _ => None,
    }
}

pub(crate) fn parse_token_copy_followup_sentence(
    tokens: &[OwnedLexToken],
) -> Option<TokenCopyFollowup> {
    let filtered: Vec<&str> = crate::cards::builders::compiler::token_word_refs(tokens)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();
    if matches!(
        filtered.as_slice(),
        [
            "sacrifice",
            "that",
            "token",
            "at",
            "beginning",
            "of",
            "next",
            "end",
            "step"
        ] | [
            "sacrifice",
            "those",
            "tokens",
            "at",
            "beginning",
            "of",
            "next",
            "end",
            "step"
        ]
    ) {
        return Some(TokenCopyFollowup::SacrificeAtNextEndStep);
    }

    parse_token_copy_modifier_sentence(tokens)
        .or_else(|| {
            is_exile_that_token_at_end_of_combat(tokens)
                .then_some(TokenCopyFollowup::ExileAtEndOfCombat)
        })
        .or_else(|| {
            is_sacrifice_that_token_at_end_of_combat(tokens)
                .then_some(TokenCopyFollowup::SacrificeAtEndOfCombat)
        })
}

pub(crate) fn parse_token_copy_followup_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> Option<TokenCopyFollowup> {
    let filtered: Vec<&str> = crate::cards::builders::compiler::token_word_refs(tokens)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();
    if matches!(
        filtered.as_slice(),
        [
            "sacrifice",
            "that",
            "token",
            "at",
            "beginning",
            "of",
            "next",
            "end",
            "step"
        ] | [
            "sacrifice",
            "those",
            "tokens",
            "at",
            "beginning",
            "of",
            "next",
            "end",
            "step"
        ]
    ) {
        return Some(TokenCopyFollowup::SacrificeAtNextEndStep);
    }

    super::parse_token_copy_modifier_sentence_lexed(tokens)
        .or_else(|| {
            super::is_exile_that_token_at_end_of_combat_lexed(tokens)
                .then_some(TokenCopyFollowup::ExileAtEndOfCombat)
        })
        .or_else(|| {
            super::is_sacrifice_that_token_at_end_of_combat_lexed(tokens)
                .then_some(TokenCopyFollowup::SacrificeAtEndOfCombat)
        })
}

fn apply_unapplied_token_copy_followup(
    sentence: &[OwnedLexToken],
    _sentence_tokens: &[OwnedLexToken],
    followup: TokenCopyFollowup,
) -> Result<Vec<EffectAst>, CardTextError> {
    let span = span_from_tokens(sentence);
    let effects = match followup {
        TokenCopyFollowup::HasHaste => vec![EffectAst::GrantAbilitiesToTarget {
            target: TargetAst::Tagged(TagKey::from(IT_TAG), span),
            abilities: vec![GrantedAbilityAst::KeywordAction(KeywordAction::Haste)],
            duration: Until::Forever,
        }],
        TokenCopyFollowup::GainHasteUntilEndOfTurn => vec![EffectAst::GrantAbilitiesToTarget {
            target: TargetAst::Tagged(TagKey::from(IT_TAG), span),
            abilities: vec![GrantedAbilityAst::KeywordAction(KeywordAction::Haste)],
            duration: Until::EndOfTurn,
        }],
        TokenCopyFollowup::EnterTappedAndAttacking => {
            return Err(CardTextError::ParseError(
                "standalone 'enters tapped and attacking' follow-up requires a preceding token-copy, populate, or meld effect".to_string(),
            ));
        }
        TokenCopyFollowup::SacrificeAtNextEndStep => vec![EffectAst::DelayedUntilNextEndStep {
            player: PlayerFilter::Any,
            effects: vec![EffectAst::Sacrifice {
                filter: ObjectFilter::tagged(TagKey::from(IT_TAG)),
                player: PlayerAst::Implicit,
                count: 1,
                target: None,
            }],
        }],
        TokenCopyFollowup::ExileAtNextEndStep => vec![EffectAst::DelayedUntilNextEndStep {
            player: PlayerFilter::Any,
            effects: vec![EffectAst::Exile {
                target: TargetAst::Object(ObjectFilter::tagged(TagKey::from(IT_TAG)), span, None),
                face_down: false,
            }],
        }],
        TokenCopyFollowup::ExileAtEndOfCombat => vec![EffectAst::DelayedUntilEndOfCombat {
            effects: vec![EffectAst::Exile {
                target: TargetAst::Object(ObjectFilter::tagged(TagKey::from(IT_TAG)), span, None),
                face_down: false,
            }],
        }],
        TokenCopyFollowup::SacrificeAtEndOfCombat => vec![EffectAst::DelayedUntilEndOfCombat {
            effects: vec![EffectAst::Sacrifice {
                filter: ObjectFilter::tagged(TagKey::from(IT_TAG)),
                player: PlayerAst::Implicit,
                count: 1,
                target: None,
            }],
        }],
    };
    Ok(effects)
}

pub(crate) fn try_apply_token_copy_followup(
    effects: &mut [EffectAst],
    followup: TokenCopyFollowup,
) -> Result<bool, CardTextError> {
    let Some(last) = effects.last_mut() else {
        return Ok(false);
    };

    match last {
        EffectAst::CreateTokenCopy {
            has_haste,
            enters_tapped,
            enters_attacking,
            exile_at_end_of_combat,
            sacrifice_at_next_end_step,
            exile_at_next_end_step,
            ..
        }
        | EffectAst::CreateTokenCopyFromSource {
            has_haste,
            enters_tapped,
            enters_attacking,
            exile_at_end_of_combat,
            sacrifice_at_next_end_step,
            exile_at_next_end_step,
            ..
        }
        | EffectAst::Populate {
            has_haste,
            enters_tapped,
            enters_attacking,
            exile_at_end_of_combat,
            sacrifice_at_next_end_step,
            exile_at_next_end_step,
            ..
        } => {
            match followup {
                TokenCopyFollowup::HasHaste => *has_haste = true,
                TokenCopyFollowup::EnterTappedAndAttacking => {
                    *enters_tapped = true;
                    *enters_attacking = true;
                }
                TokenCopyFollowup::SacrificeAtNextEndStep => *sacrifice_at_next_end_step = true,
                TokenCopyFollowup::ExileAtNextEndStep => *exile_at_next_end_step = true,
                TokenCopyFollowup::ExileAtEndOfCombat => *exile_at_end_of_combat = true,
                TokenCopyFollowup::GainHasteUntilEndOfTurn
                | TokenCopyFollowup::SacrificeAtEndOfCombat => return Ok(false),
            }
            Ok(true)
        }
        EffectAst::Meld {
            enters_tapped,
            enters_attacking,
            ..
        } => match followup {
            TokenCopyFollowup::EnterTappedAndAttacking => {
                *enters_tapped = true;
                *enters_attacking = true;
                Ok(true)
            }
            _ => Ok(false),
        },
        EffectAst::Conditional {
            if_true, if_false, ..
        }
        | EffectAst::SelfReplacement {
            if_true, if_false, ..
        } => {
            if try_apply_token_copy_followup(if_true.as_mut_slice(), followup)? {
                return Ok(true);
            }
            if try_apply_token_copy_followup(if_false.as_mut_slice(), followup)? {
                return Ok(true);
            }
            Ok(false)
        }
        EffectAst::CreateTokenWithMods {
            exile_at_end_of_combat,
            sacrifice_at_end_of_combat,
            ..
        } => {
            match followup {
                TokenCopyFollowup::ExileAtEndOfCombat => *exile_at_end_of_combat = true,
                TokenCopyFollowup::SacrificeAtEndOfCombat => *sacrifice_at_end_of_combat = true,
                TokenCopyFollowup::HasHaste
                | TokenCopyFollowup::EnterTappedAndAttacking
                | TokenCopyFollowup::GainHasteUntilEndOfTurn
                | TokenCopyFollowup::SacrificeAtNextEndStep
                | TokenCopyFollowup::ExileAtNextEndStep => return Ok(false),
            }
            Ok(true)
        }
        _ => {
            let Some(nested_effects) = token_copy_followup_container_effects_mut(last) else {
                return Ok(false);
            };
            if nested_effects.is_empty() {
                return Ok(false);
            }
            try_apply_token_copy_followup(nested_effects.as_mut_slice(), followup)
        }
    }
}
