use winnow::combinator::{alt, dispatch, fail, opt, peek};
use winnow::error::{ContextError, ErrMode};
use winnow::prelude::*;

use crate::cards::builders::{
    CardTextError, ChoiceCount, EffectAst, IT_TAG, LibraryBottomOrderAst, LibraryConsultModeAst,
    LibraryConsultStopRuleAst, PlayerAst, PredicateAst, ReturnControllerAst, SearchLibrarySlotAst,
    SubjectAst, SubjectVerbActionAst, SubjectVerbRoleAst, TagKey, TargetAst, TextSpan,
};
use crate::effect::SearchSelectionMode;
use crate::target::PlayerFilter;
use crate::target::{ObjectFilter, TaggedObjectConstraint, TaggedOpbjectRelation};
use crate::types::{CardType, Subtype};
use crate::zone::Zone;
use ironsmith_core::Value;

use super::super::activation_and_restrictions::{
    normalize_cant_words, parse_cant_restriction_clause, parse_cant_restrictions,
};
use super::super::grammar::structure::{IfClausePredicateSpec, split_if_clause_lexed};
use super::super::lexer::{
    LexStream, LexedClause, OwnedLexToken, TokenKind, lex_line, parser_token_word_positions,
    parser_token_word_refs, split_lexed_sentences, token_slice_all_are_kind, token_slice_at_is,
    token_slice_first_is, token_slice_starts_with, token_word_refs, trim_lexed_commas,
};
use super::super::object_filters::{parse_object_filter, parse_object_filter_lexed};
use super::super::search_library_support::{
    apply_search_library_mana_constraint, extract_search_library_mana_constraint,
    is_same_name_that_reference_words, normalize_search_library_filter,
    parse_restriction_duration_lexed, parse_search_library_disjunction_filter,
    split_search_different_name_reference_filter, split_search_library_count_value_clause_lexed,
    split_search_same_name_reference_filter, word_slice_mentions_nth_from_top, zone_slice_contains,
};
use super::super::token_primitives::{
    find_index as find_token_index, rfind_index as rfind_token_index,
};
use super::super::util::{
    is_article, parse_choice_count_token_prefix_consumed, parse_number, parse_subject,
    parse_target_phrase, span_from_tokens, trim_commas,
};
use super::primitives;

#[path = "effects/search_library.rs"]
mod search_library;
pub(crate) use search_library::*;
#[path = "effects/unsupported_shapes.rs"]
mod unsupported_shapes;
pub(crate) use unsupported_shapes::*;

const IF_YOU_PHRASE: &[&str] = &["if", "you"];
const THIS_TURN_PHRASE: &[&str] = &["this", "turn"];
const THIS_WAY_PHRASE: &[&str] = &["this", "way"];
const MAX_SPEED_LABEL: &[&str] = &["max", "speed"];
const SPLIT_NEGATED_ACTION_PHRASES: &[&[&str]] = &[&["do", "not"], &["did", "not"]];
const THAT_WOULD_BE_DEALT_PHRASE: &[&str] = &["that", "would", "be", "dealt"];
const LOSE_MANA_STEPS_PHASES_END_WORDS: &[&str] = &["lose", "mana", "steps", "phases", "end"];
const THAT_MANY_PREFIX: &[&str] = &["that", "many"];
const TRAILING_THAT_PLAYER_SHUFFLE_PHRASES: &[&[&str]] = &[
    &["then", "that", "player", "shuffle"],
    &["then", "that", "player", "shuffles"],
    &["that", "player", "shuffle"],
    &["that", "player", "shuffles"],
];

fn token_is_any_word(token: &OwnedLexToken, words: &[&str]) -> bool {
    token
        .as_word()
        .is_some_and(|_| words.contains(&token.parser_text()))
}

fn words_find_phrase(words: &[&str], phrase: &[&str]) -> Option<usize> {
    words
        .windows(phrase.len())
        .position(|window| window == phrase)
}

fn words_find_any_phrase(words: &[&str], phrases: &[&[&str]]) -> Option<usize> {
    phrases
        .iter()
        .find_map(|phrase| words_find_phrase(words, phrase))
}

fn search_put_attachment_target(
    search_tokens: &[OwnedLexToken],
    put_idx: Option<usize>,
) -> Result<Option<TargetAst>, CardTextError> {
    let Some(put_idx) = put_idx else {
        return Ok(None);
    };
    let put_tokens = &search_tokens[put_idx..];
    let word_positions = parser_token_word_positions(put_tokens);
    let words = word_positions
        .iter()
        .map(|(_, word)| *word)
        .collect::<Vec<_>>();
    let Some(attached_idx) = words_find_phrase(&words, &["attached", "to"]) else {
        return Ok(None);
    };
    let Some((target_token_idx, _)) = word_positions.get(attached_idx + 2) else {
        return Ok(None);
    };
    let target_tokens = &put_tokens[*target_token_idx..];
    let target_end = target_tokens
        .iter()
        .position(|token| token.is_comma() || token_is_any_word(token, &["and", "then"]))
        .unwrap_or(target_tokens.len());
    let target_tokens = trim_commas(&target_tokens[..target_end]);
    if target_tokens.is_empty() {
        return Ok(None);
    }
    parse_target_phrase(&target_tokens).map(Some)
}

fn words_contain_all(words: &[&str], required: &[&str]) -> bool {
    required
        .iter()
        .all(|required_word| words.iter().any(|word| word == required_word))
}

fn tokens_contain_any_non_article_word(tokens: &[OwnedLexToken], words: &[&str]) -> bool {
    let source_words = crate::runtime_backend::util::non_article_token_word_refs(tokens);
    source_words.iter().any(|word| words.contains(word))
}

fn is_cant_negation_word(word: &str) -> bool {
    matches!(word, "can't" | "cant" | "cannot")
}

fn is_dont_negation_word(word: &str) -> bool {
    matches!(word, "doesn't" | "doesnt" | "don't" | "dont")
}

fn is_does_do_can_word(word: &str) -> bool {
    matches!(word, "does" | "do" | "can")
}

fn is_does_or_do_word(word: &str) -> bool {
    matches!(word, "does" | "do")
}

fn is_control_or_own_word(word: &str) -> bool {
    matches!(word, "control" | "controls" | "own" | "owns")
}

fn is_compact_negated_action_word(word: &str) -> bool {
    matches!(word, "doesnt" | "didnt" | "doesn't" | "didn't")
}

fn is_prevent_damage_source_head_word(word: &str) -> bool {
    matches!(word, "target" | "that" | "this" | "it")
}

fn is_prevent_damage_explicit_reference_word(word: &str) -> bool {
    matches!(word, "this" | "that" | "it")
}

pub(crate) fn cant_sentence_clause_tokens_for_restriction_scan_lexed(
    clause_tokens: &[OwnedLexToken],
) -> Vec<OwnedLexToken> {
    split_lexed_sentences(clause_tokens)
        .into_iter()
        .next()
        .unwrap_or(clause_tokens)
        .to_vec()
}

pub(crate) fn cant_sentence_has_supported_negation_gate_lexed(
    clause_tokens: &[OwnedLexToken],
) -> bool {
    let Some((neg_start, _)) = find_cant_sentence_negation_span_lexed(clause_tokens) else {
        return false;
    };

    !clause_tokens[..neg_start]
        .iter()
        .any(|token| token_is_any_word(token, &["and"]))
}

pub(crate) fn find_cant_sentence_negation_span_lexed(
    tokens: &[OwnedLexToken],
) -> Option<(usize, usize)> {
    let mut cursor = 0usize;

    while cursor < tokens.len() {
        let token = &tokens[cursor];
        if token
            .as_word()
            .is_some_and(|word| is_cant_negation_word(word))
        {
            return Some((cursor, cursor + 1));
        }
        if token
            .as_word()
            .is_some_and(|word| is_dont_negation_word(word))
        {
            if cursor >= 2
                && token_word_refs(&tokens[cursor - 2..cursor]).as_slice() == IF_YOU_PHRASE
            {
                cursor += 1;
                continue;
            }
            if tokens.get(cursor + 1).is_some_and(|next| {
                next.as_word()
                    .is_some_and(|word| is_control_or_own_word(word))
            }) {
                cursor += 1;
                continue;
            }
            return Some((cursor, cursor + 1));
        }
        if token
            .as_word()
            .is_some_and(|word| is_does_do_can_word(word))
            && tokens
                .get(cursor + 1)
                .is_some_and(|next| token_is_any_word(next, &["not"]))
        {
            if cursor >= 2
                && token_word_refs(&tokens[cursor - 2..cursor]).as_slice() == IF_YOU_PHRASE
            {
                cursor += 2;
                continue;
            }
            if token.as_word().is_some_and(|word| is_does_or_do_word(word))
                && tokens.get(cursor + 2).is_some_and(|next| {
                    next.as_word()
                        .is_some_and(|word| is_control_or_own_word(word))
                })
            {
                cursor += 1;
                continue;
            }
            return Some((cursor, cursor + 2));
        }
        cursor += 1;
    }

    None
}

fn cant_sentence_next_turn_suffix<'a>(
    input: &mut LexStream<'a>,
) -> Result<(), ErrMode<ContextError>> {
    alt((
        primitives::phrase(&["during", "that", "players", "next", "turn"]),
        primitives::phrase(&["during", "that", "player's", "next", "turn"]),
        primitives::phrase(&["during", "that", "player", "s", "next", "turn"]),
    ))
    .void()
    .parse_next(input)
}

fn cant_sentence_for_as_long_as_marker<'a>(
    input: &mut LexStream<'a>,
) -> Result<(), ErrMode<ContextError>> {
    primitives::phrase(&["for", "as", "long", "as"])
        .void()
        .parse_next(input)
}

pub(crate) fn split_cant_sentence_next_turn_prefix_lexed(
    tokens: &[OwnedLexToken],
) -> Option<Vec<OwnedLexToken>> {
    let mut cursor = 0usize;

    while cursor < tokens.len() {
        let Some((_, rest)) =
            primitives::parse_prefix(&tokens[cursor..], cant_sentence_next_turn_suffix)
        else {
            cursor += 1;
            continue;
        };
        if token_slice_all_are_kind(rest, TokenKind::Period) {
            return Some(tokens[..cursor].to_vec());
        }
        cursor += 1;
    }

    None
}

#[derive(Debug, Clone)]
pub(crate) struct CantSentencePreparedClause {
    pub(crate) duration: crate::effect::Until,
    pub(crate) clause_tokens: Vec<OwnedLexToken>,
}

pub(crate) fn prepare_cant_sentence_restriction_clause_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<CantSentencePreparedClause>, CardTextError> {
    let Some((duration, clause_tokens)) = parse_restriction_duration_lexed(tokens)? else {
        return Ok(None);
    };
    if clause_tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "restriction clause missing body".to_string(),
        ));
    }
    if clause_tokens
        .first()
        .is_some_and(|token| token_is_any_word(token, &["if"]))
    {
        return Ok(None);
    }

    let clause_tokens = cant_sentence_clause_tokens_for_restriction_scan_lexed(&clause_tokens);
    if !cant_sentence_has_supported_negation_gate_lexed(&clause_tokens) {
        return Ok(None);
    }

    Ok(Some(CantSentencePreparedClause {
        duration,
        clause_tokens,
    }))
}

fn conditional_label_delimiter<'a>(input: &mut LexStream<'a>) -> Result<(), ErrMode<ContextError>> {
    alt((
        primitives::token_kind(TokenKind::Dash).void(),
        primitives::token_kind(TokenKind::EmDash).void(),
    ))
    .parse_next(input)
}

fn labeled_effect_prefix<'a>(input: &mut LexStream<'a>) -> Result<(), ErrMode<ContextError>> {
    (conditional_label_phrase, conditional_label_delimiter)
        .void()
        .parse_next(input)
}

pub(crate) fn split_labeled_effect_prefix_lexed(
    tokens: &[OwnedLexToken],
) -> Option<&[OwnedLexToken]> {
    let (_, rest) = primitives::parse_prefix(tokens, labeled_effect_prefix)?;
    Some(rest)
}

fn labeled_prefix_tokens(prefix: &str) -> Option<Vec<OwnedLexToken>> {
    lex_line(prefix.trim(), 0).ok()
}

pub(crate) fn is_labeled_ability_prefix_text(prefix: &str) -> bool {
    let Some(tokens) = labeled_prefix_tokens(prefix) else {
        return false;
    };
    let words = parser_token_word_refs(&tokens);
    is_labeled_ability_prefix_words(&words)
}

fn is_labeled_ability_prefix_words(words: &[&str]) -> bool {
    if words.is_empty() {
        return false;
    }

    if words.len() == 2 && words[0] == "descend" && words[1].chars().all(|ch| ch.is_ascii_digit()) {
        return true;
    }

    if matches!(
        words,
        ["spell", "mastery"]
            | ["totem", "armor"]
            | ["fateful", "hour"]
            | ["join", "forces"]
            | ["pack", "tactics"]
            | ["max", "speed"]
            | ["leading", "from", "the", "front"]
            | ["summary", "execution"]
            | ["will", "of", "the", "council"]
            | ["guardian", "protocols"]
            | ["jolly", "gutpipes"]
            | ["protection", "fighting", "style"]
            | ["relentless", "march"]
            | ["secret", "of", "the", "soul"]
            | ["secrets", "of", "the", "soul"]
            | ["flurry", "of", "blows"]
            | ["gust", "of", "wind"]
            | ["reverberating", "summons"]
    ) {
        return true;
    }

    matches!(
        words[0],
        "adamant"
            | "addendum"
            | "alliance"
            | "ascend"
            | "battalion"
            | "enrage"
            | "boast"
            | "buyback"
            | "cycling"
            | "bloodrush"
            | "channel"
            | "chroma"
            | "cohort"
            | "constellation"
            | "converge"
            | "corrupted"
            | "coven"
            | "eerie"
            | "equip"
            | "escape"
            | "exhaust"
            | "flashback"
            | "harmonize"
            | "delirium"
            | "domain"
            | "ferocious"
            | "flurry"
            | "formidable"
            | "hellbent"
            | "heroic"
            | "imprint"
            | "inspired"
            | "landfall"
            | "lieutenant"
            | "magecraft"
            | "metalcraft"
            | "morbid"
            | "parley"
            | "partner"
            | "protector"
            | "radiance"
            | "raid"
            | "renew"
            | "replicate"
            | "revolt"
            | "suspend"
            | "spectacle"
            | "strive"
            | "surge"
            | "threshold"
            | "undergrowth"
            | "ward"
    )
}

pub(crate) fn preserve_labeled_ability_prefix_for_parse_text(prefix: &str) -> bool {
    let Some(tokens) = labeled_prefix_tokens(prefix) else {
        return false;
    };
    let words = parser_token_word_refs(&tokens);
    let Some(first) = words.first().copied() else {
        return false;
    };
    if parser_token_word_refs(&tokens).as_slice() == MAX_SPEED_LABEL {
        return true;
    }

    matches!(
        first,
        "buyback"
            | "bestow"
            | "cumulative"
            | "cycling"
            | "echo"
            | "equip"
            | "escape"
            | "flashback"
            | "harmonize"
            | "boast"
            | "modular"
            | "partner"
            | "replicate"
            | "reinforce"
            | "renew"
            | "spectacle"
            | "strive"
            | "surge"
            | "suspend"
            | "ward"
    )
}

fn is_generic_ability_label_prefix_text(prefix: &str) -> bool {
    let Some(tokens) = labeled_prefix_tokens(prefix) else {
        return false;
    };
    let words = parser_token_word_refs(&tokens);
    if words.is_empty() || words.len() > 4 {
        return false;
    }

    words.iter().all(|word| {
        word.chars().all(|ch| ch.is_ascii_alphanumeric())
            && word.chars().any(|ch| ch.is_ascii_alphabetic())
    })
}

fn starts_with_if_clause_text(text: &str) -> bool {
    let Some(tokens) = lex_line(text.trim_start(), 0).ok() else {
        return false;
    };
    parser_token_word_refs(&tokens)
        .first()
        .is_some_and(|word| *word == "if")
}

pub(crate) fn should_strip_labeled_ability_prefix_text(prefix: &str, remainder: &str) -> bool {
    is_labeled_ability_prefix_text(prefix)
        || (starts_with_if_clause_text(remainder) && is_generic_ability_label_prefix_text(prefix))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ChooseNewTargetsClauseSplit<'a> {
    pub(crate) target_tokens: &'a [OwnedLexToken],
    pub(crate) count: Option<ChoiceCount>,
    pub(crate) explicit_target: bool,
    pub(crate) reference_target: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ChangeTargetClauseSplit {
    pub(crate) target_tokens: Vec<OwnedLexToken>,
    pub(crate) fixed_to_source: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ForEachDoesntClauseSplit<'a> {
    pub(crate) inner_tokens: &'a [OwnedLexToken],
    pub(crate) effect_tokens: &'a [OwnedLexToken],
    pub(crate) negation_idx: usize,
    pub(crate) negation_len: usize,
}

const CHOOSE_NEW_TARGET_PREFIXES: &[&[&str]] = &[
    &["choose", "new", "targets", "for"],
    &["chooses", "new", "targets", "for"],
    &["choose", "a", "new", "target", "for"],
    &["chooses", "a", "new", "target", "for"],
];
const CHOOSE_NEW_TARGET_REFERENCE_PREFIXES: &[&[&str]] = &[
    &["it"],
    &["them"],
    &["the", "copy"],
    &["the", "copies"],
    &["that", "copy"],
    &["those", "copies"],
    &["the", "spell"],
    &["that", "spell"],
];
const CHANGE_TARGET_PREFIXES: &[&[&str]] = &[
    &["change", "the", "target", "of"],
    &["change", "the", "targets", "of"],
    &["change", "a", "target", "of"],
];
const FOR_EACH_OPPONENT_PREFIXES: &[&[&str]] = &[
    &["for", "each", "opponent"],
    &["for", "each", "opponents"],
    &["each", "opponent"],
    &["each", "opponents"],
];
const FOR_EACH_PLAYER_PREFIXES: &[&[&str]] = &[
    &["for", "each", "player"],
    &["for", "each", "players"],
    &["each", "player"],
    &["each", "players"],
];

pub(crate) fn split_choose_new_targets_clause_lexed(
    tokens: &[OwnedLexToken],
) -> Option<ChooseNewTargetsClauseSplit<'_>> {
    let (_, mut tail_tokens) =
        primitives::strip_lexed_prefix_phrases(tokens, CHOOSE_NEW_TARGET_PREFIXES)?;
    if tail_tokens.is_empty() {
        return None;
    }

    if let Some(if_idx) = find_token_index(tail_tokens, |token| token_is_any_word(token, &["if"])) {
        tail_tokens = &tail_tokens[..if_idx];
    }
    if tail_tokens.is_empty() {
        return None;
    }

    if primitives::starts_with_any_phrase(tail_tokens, CHOOSE_NEW_TARGET_REFERENCE_PREFIXES) {
        return Some(ChooseNewTargetsClauseSplit {
            target_tokens: tail_tokens,
            count: None,
            explicit_target: false,
            reference_target: true,
        });
    }

    if let Some((prefix, rest)) = primitives::strip_lexed_prefix_phrases(
        tail_tokens,
        &[&["any", "number", "of"], &["target"]],
    ) {
        return Some(ChooseNewTargetsClauseSplit {
            target_tokens: rest,
            count: (prefix.len() == 3).then_some(ChoiceCount::any_number()),
            explicit_target: prefix.len() != 3,
            reference_target: false,
        });
    }

    Some(ChooseNewTargetsClauseSplit {
        target_tokens: tail_tokens,
        count: None,
        explicit_target: false,
        reference_target: false,
    })
}

pub(crate) fn split_change_target_unless_clause_lexed(
    tokens: &[OwnedLexToken],
) -> Option<(&[OwnedLexToken], &[OwnedLexToken])> {
    primitives::split_lexed_once_on_separator(tokens, || {
        use winnow::Parser as _;
        primitives::kw("unless").void()
    })
    .map(|(main, unless)| (trim_lexed_commas(main), trim_lexed_commas(unless)))
}

pub(crate) fn split_change_target_clause_lexed(
    tokens: &[OwnedLexToken],
) -> Option<ChangeTargetClauseSplit> {
    let (_, after_prefix_tokens) =
        primitives::strip_lexed_prefix_phrases(tokens, CHANGE_TARGET_PREFIXES)?;
    if after_prefix_tokens.is_empty() {
        return None;
    }

    let mut tail_tokens = trim_commas(after_prefix_tokens).to_vec();
    let mut fixed_to_source = false;
    if let Some((before_to, to_tail)) =
        primitives::split_lexed_once_on_separator(&tail_tokens, || {
            use winnow::Parser as _;
            primitives::kw("to").void()
        })
        && token_slice_first_is(to_tail, "this")
    {
        fixed_to_source = true;
        tail_tokens.truncate(before_to.len());
    }

    Some(ChangeTargetClauseSplit {
        target_tokens: tail_tokens,
        fixed_to_source,
    })
}

pub(crate) fn negated_action_word_index(words: &[&str]) -> Option<(usize, usize)> {
    if let Some(idx) = words
        .iter()
        .position(|word| is_compact_negated_action_word(word))
    {
        return Some((idx, 1));
    }
    if let Some(idx) = words_find_any_phrase(words, SPLIT_NEGATED_ACTION_PHRASES) {
        return Some((idx, 2));
    }
    None
}

fn split_for_each_doesnt_clause_lexed<'a>(
    tokens: &'a [OwnedLexToken],
    prefixes: &'static [&'static [&'static str]],
) -> Option<ForEachDoesntClauseSplit<'a>> {
    let mut clause_tokens = tokens;
    if token_word_refs(clause_tokens)
        .first()
        .is_some_and(|word| *word == "then")
    {
        clause_tokens = &clause_tokens[1..];
    }
    let start = primitives::words_match_any_prefix(clause_tokens, prefixes)?
        .0
        .len();
    let inner_tokens = trim_lexed_commas(&clause_tokens[start..]);
    let inner_words = token_word_refs(inner_tokens);
    if !inner_words.first().is_some_and(|word| *word == "who") {
        return None;
    }
    let (negation_idx, negation_len) = negated_action_word_index(&inner_words)?;
    let effect_token_start =
        if let Some(comma_idx) = find_token_index(inner_tokens, |token| token.is_comma()) {
            comma_idx + 1
        } else if let Some(this_way_idx) = words_find_phrase(&inner_words, THIS_WAY_PHRASE) {
            parser_token_word_positions(inner_tokens)
                .get(this_way_idx + 2)
                .map(|(idx, _)| *idx)
                .unwrap_or(inner_tokens.len())
        } else {
            parser_token_word_positions(inner_tokens)
                .get(negation_idx + negation_len)
                .map(|(idx, _)| *idx)
                .unwrap_or(inner_tokens.len())
        };
    let effect_tokens = trim_lexed_commas(&inner_tokens[effect_token_start..]);
    (!effect_tokens.is_empty()).then_some(ForEachDoesntClauseSplit {
        inner_tokens,
        effect_tokens,
        negation_idx,
        negation_len,
    })
}

pub(crate) fn split_for_each_opponent_doesnt_clause_lexed(
    tokens: &[OwnedLexToken],
) -> Option<ForEachDoesntClauseSplit<'_>> {
    split_for_each_doesnt_clause_lexed(tokens, FOR_EACH_OPPONENT_PREFIXES)
}

pub(crate) fn split_for_each_player_doesnt_clause_lexed(
    tokens: &[OwnedLexToken],
) -> Option<ForEachDoesntClauseSplit<'_>> {
    split_for_each_doesnt_clause_lexed(tokens, FOR_EACH_PLAYER_PREFIXES)
}

pub(crate) fn split_negated_who_this_way_filter_tokens_lexed(
    inner_tokens: &[OwnedLexToken],
) -> Option<&[OwnedLexToken]> {
    let inner_words = token_word_refs(inner_tokens);
    if !inner_words.first().is_some_and(|word| *word == "who") {
        return None;
    }
    let this_way_idx = words_find_phrase(&inner_words, THIS_WAY_PHRASE)?;
    let (negation_idx, negation_len) = negated_action_word_index(&inner_words)?;
    let verb_idx = negation_idx + negation_len;
    let verb = inner_words.get(verb_idx).copied().unwrap_or("");
    if !matches!(verb, "discard" | "discarded") || this_way_idx <= verb_idx + 1 {
        return None;
    }

    let parser_words = parser_token_word_positions(inner_tokens);
    let filter_start = parser_words.get(verb_idx + 1).map(|(idx, _)| *idx)?;
    let filter_end = parser_words.get(this_way_idx).map(|(idx, _)| *idx)?;
    let filter_tokens = trim_lexed_commas(&inner_tokens[filter_start..filter_end]);
    (!filter_tokens.is_empty()).then_some(filter_tokens)
}

const PREVENT_DAMAGE_BY_PREFIXES: &[&[&str]] = &[&["that", "would", "be", "dealt", "by"]];
const PREVENT_DAMAGE_TO_AND_BY_PREFIXES: &[&[&str]] =
    &[&["that", "would", "be", "dealt", "to", "and", "dealt", "by"]];
const PREVENT_DAMAGE_TO_PREFIXES: &[&[&str]] = &[&["that", "would", "be", "dealt", "to"]];

pub(crate) fn parse_prevent_damage_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let words = token_word_refs(tokens);
    let prefix = ["prevent", "all", "combat", "damage"];
    if primitives::words_match_prefix(tokens, &prefix).is_none() {
        return Ok(None);
    }

    let Some(this_turn_idx) = words_find_phrase(&words, THIS_TURN_PHRASE) else {
        return Err(CardTextError::ParseError(format!(
            "unsupported prevent-all-combat-damage duration (clause: '{}')",
            words.join(" ")
        )));
    };
    if LexedClause::new(tokens)
        .after_words(this_turn_idx + 2)
        .is_some_and(|tail| tail.word_refs().as_slice() == THIS_TURN_PHRASE)
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported prevent-all-combat-damage duration (clause: '{}')",
            words.join(" ")
        )));
    }
    if this_turn_idx < prefix.len() {
        return Err(CardTextError::ParseError(format!(
            "unsupported prevent-all-combat-damage duration (clause: '{}')",
            words.join(" ")
        )));
    }

    let mut core_words = Vec::with_capacity(words.len() - prefix.len() - 2);
    core_words.extend_from_slice(&words[prefix.len()..this_turn_idx]);
    core_words.extend_from_slice(&words[this_turn_idx + 2..]);
    let mut core_tokens = Vec::with_capacity(tokens.len() - prefix.len() - 2);
    core_tokens.extend_from_slice(&tokens[prefix.len()..this_turn_idx]);
    core_tokens.extend_from_slice(&tokens[this_turn_idx + 2..]);

    if token_word_refs(&core_tokens).as_slice() == THAT_WOULD_BE_DEALT_PHRASE {
        return Ok(Some(EffectAst::subject_verb_prevent_all_combat_damage(
            crate::effect::Until::EndOfTurn,
        )));
    }

    if primitives::words_match_any_prefix(&core_tokens, PREVENT_DAMAGE_BY_PREFIXES).is_some() {
        let source_tokens = &core_tokens[5..];
        if source_tokens
            .first()
            .and_then(OwnedLexToken::as_word)
            .is_some_and(is_prevent_damage_source_head_word)
        {
            let (source, has_color_condition) =
                parse_prevent_damage_source_target_lexed(source_tokens, &words)?;
            return Ok(Some(prevent_damage_effect_with_optional_condition(
                source,
                has_color_condition,
            )));
        }
        if let Ok(source_filter) = parse_object_filter(source_tokens, false) {
            return Ok(Some(
                EffectAst::subject_verb_prevent_all_combat_damage_from_source_filter(
                    source_filter,
                    crate::effect::Until::EndOfTurn,
                ),
            ));
        }
        let (source, has_color_condition) =
            parse_prevent_damage_source_target_lexed(source_tokens, &words)?;
        return Ok(Some(prevent_damage_effect_with_optional_condition(
            source,
            has_color_condition,
        )));
    }

    if primitives::words_match_any_prefix(&core_tokens, PREVENT_DAMAGE_TO_AND_BY_PREFIXES).is_some()
    {
        let source_tokens = &core_tokens[8..];
        let (source, has_color_condition) =
            parse_prevent_damage_source_target_lexed(source_tokens, &words)?;
        return Ok(Some(prevent_damage_effect_with_optional_condition(
            source,
            has_color_condition,
        )));
    }

    if primitives::words_match_any_prefix(&core_tokens, PREVENT_DAMAGE_TO_PREFIXES).is_some() {
        return parse_prevent_damage_target_scope_lexed(&core_tokens[5..], &words);
    }

    if let Some(would_idx) = core_words.iter().position(|word| *word == "would")
        && core_words
            .get(would_idx + 1)
            .is_some_and(|word| *word == "deal")
    {
        let source_tokens = &core_tokens[..would_idx];
        if !source_tokens
            .first()
            .and_then(OwnedLexToken::as_word)
            .is_some_and(is_prevent_damage_source_head_word)
            && let Ok(source_filter) = parse_object_filter(source_tokens, false)
        {
            return Ok(Some(
                EffectAst::subject_verb_prevent_all_combat_damage_from_source_filter(
                    source_filter,
                    crate::effect::Until::EndOfTurn,
                ),
            ));
        }
        let (source, has_color_condition) =
            parse_prevent_damage_source_target_lexed(source_tokens, &words)?;
        let has_color_condition = has_color_condition
            || prevent_damage_shares_color_clause_lexed(&core_tokens[would_idx + 2..]);
        return Ok(Some(prevent_damage_effect_with_optional_condition(
            source,
            has_color_condition,
        )));
    }

    Err(CardTextError::ParseError(format!(
        "unsupported prevent-all-combat-damage clause tail (clause: '{}')",
        words.join(" ")
    )))
}

pub(crate) fn parse_prevent_damage_source_target_lexed(
    tokens: &[OwnedLexToken],
    clause_words: &[&str],
) -> Result<(TargetAst, bool), CardTextError> {
    if tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing prevent-all source target (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let (tokens, has_color_condition) = strip_prevent_damage_shares_color_clause_lexed(tokens);
    let source_words = crate::runtime_backend::util::non_article_token_word_refs(tokens);
    let is_explicit_reference =
        tokens_contain_any_non_article_word(tokens, &["target", "this", "that", "it"])
            || source_words
                .first()
                .is_some_and(|word| is_prevent_damage_explicit_reference_word(word));
    if !is_explicit_reference {
        return Err(CardTextError::ParseError(format!(
            "unsupported prevent-all source target '{}'",
            source_words.join(" ")
        )));
    }

    let source = parse_target_phrase(tokens)?;
    match source {
        TargetAst::Source(_) | TargetAst::Object(_, _, _) | TargetAst::Tagged(_, _) => {
            Ok((source, has_color_condition))
        }
        _ => Err(CardTextError::ParseError(format!(
            "unsupported prevent-all source target '{}'",
            token_word_refs(tokens).join(" ")
        ))),
    }
}

fn prevent_damage_effect_with_optional_condition(
    source: TargetAst,
    has_color_condition: bool,
) -> EffectAst {
    let condition_filter = match &source {
        TargetAst::Object(filter, _, _) => Some(filter.clone()),
        _ => None,
    };
    let prevent = EffectAst::subject_verb_prevent_all_combat_damage_from_source(
        source,
        crate::effect::Until::EndOfTurn,
    );
    if has_color_condition {
        let predicate = condition_filter.map_or_else(
            || {
                PredicateAst::TargetMatches(
                    ObjectFilter::default().shares_color_with_tagged(TagKey::from(IT_TAG)),
                )
            },
            |filter| {
                PredicateAst::TargetMatches(filter.shares_color_with_tagged(TagKey::from(IT_TAG)))
            },
        );
        EffectAst::Conditional {
            predicate,
            if_true: vec![prevent],
            if_false: Vec::new(),
        }
    } else {
        prevent
    }
}

fn prevent_damage_shares_color_clause_lexed(tokens: &[OwnedLexToken]) -> bool {
    let words = crate::runtime_backend::util::non_article_token_word_refs(tokens);
    matches!(
        words.as_slice(),
        ["if", "it", "shares", "color", "with", "that", "permanent"]
            | ["if", "it", "shares", "color", "with", "that", "object"]
            | ["if", "it", "shares", "color", "with", "that", "creature"]
            | ["if", "it", "shares", "color", "with", "it"]
    )
}

fn strip_prevent_damage_shares_color_clause_lexed(
    tokens: &[OwnedLexToken],
) -> (&[OwnedLexToken], bool) {
    let Some(if_idx) = tokens
        .iter()
        .rposition(|token| token_is_any_word(token, &["if"]))
    else {
        return (tokens, false);
    };
    if prevent_damage_shares_color_clause_lexed(&tokens[if_idx..]) {
        return (&tokens[..if_idx], true);
    }
    (tokens, false)
}

pub(crate) fn parse_prevent_damage_target_scope_lexed(
    tokens: &[OwnedLexToken],
    clause_words: &[&str],
) -> Result<Option<EffectAst>, CardTextError> {
    if tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing prevent-all target scope (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let target_words = crate::runtime_backend::util::non_article_token_word_refs(tokens);
    if matches!(target_words.as_slice(), ["player"] | ["players"]) {
        return Ok(Some(
            EffectAst::subject_verb_prevent_all_combat_damage_to_players(
                crate::effect::Until::EndOfTurn,
            ),
        ));
    }
    if target_words.as_slice() == ["you"] {
        return Ok(Some(
            EffectAst::subject_verb_prevent_all_combat_damage_to_you(
                crate::effect::Until::EndOfTurn,
            ),
        ));
    }

    Err(CardTextError::ParseError(format!(
        "unsupported prevent-all target scope '{}'",
        token_word_refs(tokens).join(" ")
    )))
}

fn conditional_sentence_family_head<'a>(
    input: &mut LexStream<'a>,
) -> Result<(), ErrMode<ContextError>> {
    alt((
        primitives::phrase(&["then", "if"]),
        (
            conditional_label_phrase,
            opt(conditional_label_delimiter),
            primitives::kw("if"),
        )
            .void(),
        primitives::kw("if").void(),
    ))
    .parse_next(input)
}

pub(crate) fn split_conditional_sentence_family_head_lexed(
    tokens: &[OwnedLexToken],
) -> Option<&[OwnedLexToken]> {
    let (_, rest) = primitives::parse_prefix(tokens, conditional_sentence_family_head)?;
    let consumed = tokens.len().checked_sub(rest.len())?;
    consumed.checked_sub(1).map(|if_idx| &tokens[if_idx..])
}

pub(crate) fn parse_conditional_sentence_with_grammar_entrypoint_lexed(
    tokens: &[OwnedLexToken],
    parse_effect_chain_lexed: fn(&[OwnedLexToken]) -> Result<Vec<EffectAst>, CardTextError>,
) -> Result<Vec<EffectAst>, CardTextError> {
    let split = split_if_clause_lexed(tokens, parse_effect_chain_lexed)?;

    Ok(vec![match split.predicate {
        IfClausePredicateSpec::Conditional(predicate) => EffectAst::Conditional {
            predicate,
            if_true: split.effects,
            if_false: Vec::new(),
        },
        IfClausePredicateSpec::Result(predicate) => EffectAst::IfResult {
            predicate,
            effects: split.effects,
        },
    }])
}

pub(crate) fn parse_conditional_sentence_family_lexed(
    tokens: &[OwnedLexToken],
    parse_effect_chain_lexed: fn(&[OwnedLexToken]) -> Result<Vec<EffectAst>, CardTextError>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(conditional_tokens) = split_conditional_sentence_family_head_lexed(tokens) else {
        return Ok(None);
    };

    parse_conditional_sentence_with_grammar_entrypoint_lexed(
        conditional_tokens,
        parse_effect_chain_lexed,
    )
    .map(Some)
}

pub(crate) fn parse_cant_effect_sentence_with_grammar_entrypoint_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if let Some(prefix_tokens) = split_cant_sentence_next_turn_prefix_lexed(tokens) {
        let prefix_tokens = prefix_tokens.as_slice();
        if let Some(parsed) = parse_cant_restriction_clause(prefix_tokens)? {
            let next_turn_effects = match parsed.restriction {
                crate::effect::Restriction::CastSpellsMatching(player, spell_filter) => {
                    let nested = crate::effect::Restriction::cast_spells_matching(
                        PlayerFilter::Active,
                        spell_filter,
                    );
                    match player {
                        PlayerFilter::Opponent => Some(vec![EffectAst::ForEachOpponent {
                            effects: vec![EffectAst::DelayedUntilNextUpkeep {
                                player: crate::cards::builders::PlayerAst::That,
                                effects: vec![EffectAst::subject_verb_cant(
                                    nested,
                                    crate::effect::Until::EndOfTurn,
                                    None,
                                )],
                            }],
                        }]),
                        PlayerFilter::IteratedPlayer => {
                            Some(vec![EffectAst::DelayedUntilNextUpkeep {
                                player: crate::cards::builders::PlayerAst::That,
                                effects: vec![EffectAst::subject_verb_cant(
                                    nested,
                                    crate::effect::Until::EndOfTurn,
                                    None,
                                )],
                            }])
                        }
                        _ => None,
                    }
                }
                crate::effect::Restriction::CastMoreThanOneSpellEachTurn(player, spell_filter) => {
                    let nested = crate::effect::Restriction::CastMoreThanOneSpellEachTurn(
                        PlayerFilter::Active,
                        spell_filter,
                    );
                    match player {
                        PlayerFilter::Opponent => Some(vec![EffectAst::ForEachOpponent {
                            effects: vec![EffectAst::DelayedUntilNextUpkeep {
                                player: crate::cards::builders::PlayerAst::That,
                                effects: vec![EffectAst::subject_verb_cant(
                                    nested,
                                    crate::effect::Until::EndOfTurn,
                                    None,
                                )],
                            }],
                        }]),
                        PlayerFilter::IteratedPlayer => {
                            Some(vec![EffectAst::DelayedUntilNextUpkeep {
                                player: crate::cards::builders::PlayerAst::That,
                                effects: vec![EffectAst::subject_verb_cant(
                                    nested,
                                    crate::effect::Until::EndOfTurn,
                                    None,
                                )],
                            }])
                        }
                        _ => None,
                    }
                }
                _ => None,
            };

            if let Some(next_turn_effects) = next_turn_effects {
                return Ok(Some(next_turn_effects));
            }
        }
    }

    let source_tapped_duration = cant_sentence_has_source_remains_tapped_duration(tokens);
    let words = token_word_refs(tokens);
    if words_contain_all(&words, LOSE_MANA_STEPS_PHASES_END_WORDS) {
        return Ok(Some(vec![
            EffectAst::subject_verb_dont_lose_this_mana_as_steps_and_phases_end_this_turn(),
        ]));
    }
    let Some(prepared_clause) = prepare_cant_sentence_restriction_clause_lexed(tokens)? else {
        return Ok(None);
    };
    let duration = prepared_clause.duration;
    let clause_tokens = prepared_clause.clause_tokens;

    let Some(restrictions) = parse_cant_restrictions(&clause_tokens)? else {
        return Err(CardTextError::ParseError(format!(
            "unsupported restriction clause body (clause: '{}')",
            token_word_refs(&clause_tokens).join(" ")
        )));
    };

    let mut target: Option<crate::cards::builders::TargetAst> = None;
    let mut effects = Vec::new();
    for parsed in restrictions {
        if let Some(parsed_target) = parsed.target {
            if let Some(existing) = &target {
                if *existing != parsed_target {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported mixed restriction targets (clause: '{}')",
                        token_word_refs(&clause_tokens).join(" ")
                    )));
                }
            } else {
                target = Some(parsed_target);
            }
        }
        effects.push(EffectAst::subject_verb_cant(
            parsed.restriction,
            duration.clone(),
            source_tapped_duration.then_some(crate::ConditionExpr::SourceIsTapped),
        ));
    }
    if let Some(target) = target {
        effects.insert(0, EffectAst::subject_verb_target_only(target));
    }

    Ok(Some(effects))
}

pub(crate) fn parse_cant_effect_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_cant_effect_sentence_with_grammar_entrypoint_lexed(tokens)
}

pub(crate) fn parse_search_library_sentence_with_grammar_entrypoint_lexed(
    tokens: &[OwnedLexToken],
    subject_starts_effect_lexed: fn(&[OwnedLexToken]) -> bool,
    parse_leading_effects_lexed: fn(&[OwnedLexToken]) -> Result<Vec<EffectAst>, CardTextError>,
    parse_effect_clause_lexed: fn(&[OwnedLexToken]) -> Result<EffectAst, CardTextError>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    fn has_trailing_that_player_shuffle(tokens: &[OwnedLexToken]) -> bool {
        words_find_any_phrase(
            &crate::runtime_backend::token_word_refs(tokens),
            TRAILING_THAT_PLAYER_SHUFFLE_PHRASES,
        )
        .is_some()
    }

    let words_all = parser_token_word_refs(tokens);
    let Some(head_split) = split_search_library_sentence_head_lexed(tokens) else {
        return Ok(None);
    };

    let subject_prelude = parse_search_library_leading_effect_prelude_lexed(
        head_split.subject_tokens,
        subject_starts_effect_lexed,
        parse_leading_effects_lexed,
    )?;
    let subject_tokens = subject_prelude.subject_tokens;
    let sentence_has_direct_may = head_split.sentence_has_direct_may;
    let mut leading_effects = subject_prelude.leading_effects;
    let wrap_each_target_player =
        search_library_subject_wraps_each_target_player_lexed(subject_tokens);
    let player_iteration_filter =
        search_library_subject_player_iteration_filter_lexed(subject_tokens);
    let iterated_subject_filter =
        parse_search_library_iterated_object_subject_lexed(subject_tokens)?;
    let chooser = if player_iteration_filter.is_some() {
        PlayerAst::That
    } else {
        match parse_subject(subject_tokens) {
            SubjectAst::Player(player) => player,
            _ => PlayerAst::Implicit,
        }
    };
    crate::parse_trace::event(format!(
        "effect-route: subject-verb verb=Search subject={}",
        if subject_tokens.is_empty() {
            "implicit"
        } else {
            "explicit"
        }
    ));

    let search_tokens = head_split.search_tokens;
    if !search_library_starts_with_search_verb_lexed(search_tokens) {
        return Ok(None);
    }
    let search_words = parser_token_word_refs(search_tokens);
    if search_words.is_empty() {
        return Ok(None);
    }
    let Some(subject_routing) = derive_search_library_subject_routing_lexed(search_tokens, chooser)
    else {
        return Ok(None);
    };
    let player = subject_routing.player;
    let search_player_target = subject_routing.search_player_target;
    let forced_library_owner = subject_routing.forced_library_owner;
    let search_zones_override = subject_routing.search_zones_override;
    if search_library_has_unsupported_top_position_probe(&search_words) {
        return Err(CardTextError::ParseError(format!(
            "unsupported search-library top-position clause (clause: '{}')",
            words_all.join(" ")
        )));
    }

    let clause_markers = scan_search_library_clause_markers_lexed(search_tokens)
        .expect("grammar-owned search-library clause marker scan should produce defaults");
    let for_idx = clause_markers.for_idx;
    let put_idx = clause_markers.put_idx;
    let has_explicit_destination = clause_markers.has_explicit_destination;
    let filter_boundary = clause_markers.filter_boundary;

    let filter_end =
        find_search_library_filter_boundary_lexed(search_tokens, for_idx, filter_boundary)
            .filter_end;

    if filter_end <= for_idx + 1 {
        return Err(CardTextError::ParseError(format!(
            "missing search filter in search-library sentence (clause: '{}')",
            words_all.join(" ")
        )));
    }

    let count_tokens = &search_tokens[for_idx + 1..filter_end];
    let count_prefix = parse_search_library_count_prefix_lexed(count_tokens);
    let mut count = count_prefix.count;
    let search_mode = count_prefix.search_mode;
    let count_used = count_prefix.count_used;
    let mut prefix_count_value = count_prefix.count_value;

    let filter_start = for_idx + 1 + count_used;
    if filter_start >= filter_end {
        return Err(CardTextError::ParseError(format!(
            "missing object selector in search-library sentence (clause: '{}')",
            words_all.join(" ")
        )));
    }

    let mut raw_filter_tokens = trim_commas(&search_tokens[filter_start..filter_end]).to_vec();
    if token_slice_starts_with(&raw_filter_tokens, THAT_MANY_PREFIX) {
        prefix_count_value.get_or_insert(Value::Count(ObjectFilter::tagged(TagKey::from(IT_TAG))));
        count = if search_mode == SearchSelectionMode::Optional {
            ChoiceCount::up_to_dynamic_x()
        } else {
            ChoiceCount::dynamic_x()
        };
        raw_filter_tokens.drain(0..2);
    }
    let (filter_tokens, count_value) = if let Some((base_filter_tokens, count_value)) =
        split_search_library_count_value_clause_lexed(&raw_filter_tokens)?
    {
        (base_filter_tokens, Some(count_value))
    } else {
        (raw_filter_tokens, prefix_count_value)
    };
    let (filter_tokens, mana_constraint) = if let Some((base_filter_tokens, mana_constraint)) =
        extract_search_library_mana_constraint(&filter_tokens)
    {
        (base_filter_tokens, Some(mana_constraint))
    } else {
        (filter_tokens.to_vec(), None)
    };
    let (filter_tokens, distinct_names) =
        strip_search_library_different_names_clause_lexed(&filter_tokens);
    let mut basic_land_type_slots =
        parse_search_library_basic_land_type_slots_lexed(&filter_tokens);
    let same_name_split = if basic_land_type_slots.is_none() {
        parse_search_library_same_name_reference_lexed(
            &filter_tokens,
            filter_tokens.clone(),
            &words_all,
        )?
    } else {
        SearchLibrarySameNameSplit {
            filter_tokens: filter_tokens.clone(),
            same_name_reference: None,
            same_name_relation: TaggedOpbjectRelation::SameNameAsTagged,
        }
    };
    let filter_tokens = same_name_split.filter_tokens;
    let same_name_reference = same_name_split.same_name_reference;
    let same_name_relation = same_name_split.same_name_relation;
    let same_name_reference_requires_setup = matches!(
        same_name_reference,
        Some(SearchLibrarySameNameReference::Target(_))
            | Some(SearchLibrarySameNameReference::Choose { .. })
    );

    let named_filters = if basic_land_type_slots.is_none() && count_used == 0 {
        split_search_named_item_filters_lexed(&filter_tokens, &words_all)?
    } else {
        None
    };
    let mut filter = if basic_land_type_slots.is_none() {
        parse_search_library_object_filter_lexed(&filter_tokens, &words_all)?
    } else {
        ObjectFilter::default()
    };
    filter.distinct_names = distinct_names;
    if let Some(same_name_tag) = same_name_reference
        .as_ref()
        .map(|reference| match reference {
            SearchLibrarySameNameReference::Tagged(tag) => tag.clone(),
            SearchLibrarySameNameReference::Target(_) => TagKey::from(IT_TAG),
            SearchLibrarySameNameReference::Choose { tag, .. } => tag.clone(),
        })
    {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: same_name_tag.clone(),
            relation: same_name_relation,
        });
    }
    if filter.owner.is_none()
        && let Some(owner) = forced_library_owner.clone()
    {
        filter.owner = Some(owner);
    }
    normalize_search_library_filter(&mut filter);
    if let Some(mana_constraint) = mana_constraint {
        apply_search_library_mana_constraint(&mut filter, mana_constraint);
    }
    let search_zones_are_library_only = match search_zones_override.as_ref() {
        None => true,
        Some(zones) => zones.len() == 1 && zones[0] == Zone::Library,
    };
    if search_zones_are_library_only {
        filter.zone = Some(Zone::Library);
    }

    let discard_before_shuffle_followup =
        find_search_library_discard_before_shuffle_followup_lexed(search_tokens, put_idx);
    let trailing_discard_before_shuffle = discard_before_shuffle_followup.is_some();
    let effect_routing = derive_search_library_effect_routing_lexed(
        tokens,
        search_tokens,
        clause_markers,
        trailing_discard_before_shuffle,
    );
    let destination = effect_routing.destination;
    let reveal = effect_routing.reveal;
    let face_down_exile = effect_routing.face_down_exile;
    let original_shuffle = effect_routing.shuffle;
    let trailing_create_followup = find_search_library_trailing_create_followup_lexed(
        search_tokens,
        put_idx.unwrap_or(filter_boundary),
    );
    let shuffle = original_shuffle && trailing_create_followup.is_none();
    let split_battlefield_and_hand = effect_routing.split_battlefield_and_hand;
    let library_position_from_top = effect_routing.library_position_from_top.clone();
    let attachment_target = search_put_attachment_target(search_tokens, clause_markers.put_idx)?;
    let mut handled_direct_may_in_iterated_search = false;
    let mut effects = if let Some(mut slots) = basic_land_type_slots.take() {
        if !has_explicit_destination || !search_zones_are_library_only {
            return Err(CardTextError::ParseError(format!(
                "unsupported each-basic-land-type search-library clause (clause: '{}')",
                words_all.join(" ")
            )));
        }
        for slot in &mut slots {
            if slot.filter.owner.is_none()
                && let Some(owner) = forced_library_owner.clone()
            {
                slot.filter.owner = Some(owner);
            }
        }
        vec![EffectAst::subject_verb_search_library_slots(
            player,
            slots,
            destination,
            reveal,
            TagKey::from("search_library_slots_progress"),
        )]
    } else if let Some(iterated_filter) = iterated_subject_filter.clone()
        && has_explicit_destination
        && named_filters.is_none()
        && !split_battlefield_and_hand
        && !(destination == Zone::Exile && face_down_exile)
    {
        let searched_tag: TagKey = "searched".into();
        let search_zones = search_zones_override.unwrap_or_else(|| vec![Zone::Library]);
        let battlefield_tapped =
            destination == Zone::Battlefield && effect_routing.has_tapped_modifier;
        // Always use the search subject `player` so the shuffle references
        // the searcher, not the last-referenced player from a preceding effect.
        let shuffle_player = player;

        let mut per_object_effects = vec![EffectAst::ChooseObjectsAcrossZones {
            filter,
            count,
            count_value: count_value.clone(),
            player: chooser,
            tag: searched_tag.clone(),
            zones: search_zones.clone(),
            search_mode: Some(search_mode),
        }];
        if sentence_has_direct_may {
            handled_direct_may_in_iterated_search = true;
            per_object_effects = vec![if matches!(chooser, PlayerAst::You | PlayerAst::Implicit) {
                EffectAst::May {
                    effects: per_object_effects,
                }
            } else {
                EffectAst::MayByPlayer {
                    player: chooser,
                    effects: per_object_effects,
                }
            }];
        }

        let mut sequence = vec![EffectAst::ForEachObject {
            filter: iterated_filter,
            effects: per_object_effects,
        }];
        if reveal {
            sequence.push(EffectAst::subject_verb_reveal_tagged(searched_tag.clone()));
        }
        if shuffle
            && destination == Zone::Library
            && zone_slice_contains(&search_zones, Zone::Library)
        {
            sequence.push(EffectAst::subject_verb(
                SubjectVerbRoleAst::LibraryOwner,
                shuffle_player,
                SubjectVerbActionAst::ShuffleLibrary,
            ));
        }
        sequence.push(EffectAst::ForEachTagged {
            tag: searched_tag.clone(),
            effects: vec![EffectAst::subject_verb_move_to_zone(
                TargetAst::Tagged(searched_tag, span_from_tokens(tokens)),
                destination,
                matches!(destination, Zone::Library),
                ReturnControllerAst::Preserve,
                battlefield_tapped,
                None,
            )],
        });
        if shuffle
            && !(destination == Zone::Library && zone_slice_contains(&search_zones, Zone::Library))
        {
            sequence.push(EffectAst::subject_verb(
                SubjectVerbRoleAst::LibraryOwner,
                shuffle_player,
                SubjectVerbActionAst::ShuffleLibrary,
            ));
        }
        sequence
    } else if let Some(named_filters) = named_filters {
        let searched_tag: TagKey = "searched_named".into();
        let zones = search_zones_override.unwrap_or_else(|| vec![Zone::Library]);
        let mut sequence = Vec::new();
        for mut named_filter in named_filters {
            if named_filter.owner.is_none()
                && let Some(owner) = forced_library_owner.clone()
            {
                named_filter.owner = Some(owner);
            }
            normalize_search_library_filter(&mut named_filter);
            sequence.push(EffectAst::ChooseObjectsAcrossZones {
                filter: named_filter,
                count: ChoiceCount::exactly(1),
                count_value: None,
                player: chooser,
                tag: searched_tag.clone(),
                zones: zones.clone(),
                search_mode: Some(SearchSelectionMode::Exact),
            });
        }
        if reveal {
            sequence.push(EffectAst::subject_verb_reveal_tagged(searched_tag.clone()));
        }
        sequence.push(EffectAst::subject_verb_move_to_zone(
            TargetAst::Tagged(searched_tag, span_from_tokens(tokens)),
            destination,
            matches!(destination, Zone::Library),
            ReturnControllerAst::Preserve,
            destination == Zone::Battlefield && effect_routing.has_tapped_modifier,
            None,
        ));
        if shuffle && zones.contains(&Zone::Library) {
            sequence.push(EffectAst::subject_verb(
                SubjectVerbRoleAst::LibraryOwner,
                player,
                SubjectVerbActionAst::ShuffleLibrary,
            ));
        }
        sequence
    } else if !has_explicit_destination {
        let chosen_tag: TagKey = "searched".into();
        let mut sequence = vec![EffectAst::ChooseObjectsAcrossZones {
            filter,
            count,
            count_value: count_value.clone(),
            player: chooser,
            tag: chosen_tag.clone(),
            zones: search_zones_override.unwrap_or_else(|| vec![Zone::Library]),
            search_mode: Some(search_mode),
        }];
        if reveal {
            sequence.push(EffectAst::subject_verb_reveal_tagged(chosen_tag.clone()));
        }
        if shuffle {
            sequence.push(EffectAst::subject_verb(
                SubjectVerbRoleAst::LibraryOwner,
                player,
                SubjectVerbActionAst::ShuffleLibrary,
            ));
        }
        sequence
    } else if let Some(search_zones) = search_zones_override.clone() {
        let chosen_tag: TagKey = "searched_multi_zone".into();
        let battlefield_tapped =
            destination == Zone::Battlefield && effect_routing.has_tapped_modifier;
        // Use the search subject `player` (e.g. Implicit/You) rather than
        // PlayerAst::That, which would resolve to the last referenced player
        // in a preceding effect (e.g. "target player" from a damage clause).
        let shuffle_player = player;
        let mut sequence = vec![EffectAst::ChooseObjectsAcrossZones {
            filter,
            count,
            count_value: count_value.clone(),
            player: chooser,
            tag: chosen_tag.clone(),
            zones: search_zones.clone(),
            search_mode: Some(search_mode),
        }];
        if reveal {
            sequence.push(EffectAst::subject_verb_reveal_tagged(chosen_tag.clone()));
        }
        if shuffle
            && destination == Zone::Library
            && zone_slice_contains(&search_zones, Zone::Library)
        {
            sequence.push(EffectAst::subject_verb(
                SubjectVerbRoleAst::LibraryOwner,
                shuffle_player,
                SubjectVerbActionAst::ShuffleLibrary,
            ));
        }
        let mut per_tag_effects = vec![EffectAst::subject_verb_move_to_zone(
            TargetAst::Tagged(chosen_tag.clone(), span_from_tokens(tokens)),
            destination,
            matches!(destination, Zone::Library),
            ReturnControllerAst::Preserve,
            battlefield_tapped,
            None,
        )];
        if destination == Zone::Battlefield
            && let Some(target) = attachment_target.clone()
        {
            per_tag_effects.push(EffectAst::subject_verb_attach(
                TargetAst::Tagged(chosen_tag.clone(), span_from_tokens(tokens)),
                target,
            ));
        }
        sequence.push(EffectAst::ForEachTagged {
            tag: chosen_tag.clone(),
            effects: per_tag_effects,
        });
        if shuffle
            && !(destination == Zone::Library && zone_slice_contains(&search_zones, Zone::Library))
        {
            sequence.push(EffectAst::subject_verb(
                SubjectVerbRoleAst::LibraryOwner,
                shuffle_player,
                SubjectVerbActionAst::ShuffleLibrary,
            ));
        }
        sequence
    } else if split_battlefield_and_hand {
        let battlefield_tapped = effect_routing.has_tapped_modifier;
        vec![
            EffectAst::subject_verb_search_library(
                filter.clone(),
                Zone::Battlefield,
                chooser,
                player,
                search_mode,
                reveal,
                false,
                ChoiceCount::up_to(1),
                None,
                None,
                battlefield_tapped,
            ),
            EffectAst::subject_verb_search_library(
                filter,
                Zone::Hand,
                chooser,
                player,
                search_mode,
                reveal,
                shuffle,
                ChoiceCount::up_to(1),
                None,
                None,
                false,
            ),
        ]
    } else if destination == Zone::Exile && face_down_exile {
        let searched_tag: TagKey = "searched_face_down".into();
        let mut sequence = vec![
            EffectAst::ChooseObjectsAcrossZones {
                filter,
                count,
                count_value: count_value.clone(),
                player: chooser,
                tag: searched_tag.clone(),
                zones: vec![Zone::Library],
                search_mode: Some(search_mode),
            },
            EffectAst::subject_verb_exile(
                TargetAst::Tagged(searched_tag, span_from_tokens(tokens)),
                true,
            ),
        ];
        if shuffle {
            sequence.push(EffectAst::subject_verb(
                SubjectVerbRoleAst::LibraryOwner,
                player,
                SubjectVerbActionAst::ShuffleLibrary,
            ));
        }
        sequence
    } else {
        let battlefield_tapped =
            destination == Zone::Battlefield && effect_routing.has_tapped_modifier;
        vec![EffectAst::subject_verb_search_library(
            filter,
            destination,
            chooser,
            player,
            search_mode,
            reveal,
            shuffle,
            count,
            count_value.clone(),
            library_position_from_top,
            battlefield_tapped,
        )]
    };

    if let Some(discard_followup) = discard_before_shuffle_followup {
        let discard_tokens =
            trim_commas(&search_tokens[discard_followup.discard_idx..discard_followup.discard_end]);
        if !discard_tokens.is_empty() {
            effects.push(parse_effect_clause_lexed(&discard_tokens)?);
        }
        effects.push(EffectAst::subject_verb(
            SubjectVerbRoleAst::LibraryOwner,
            player,
            SubjectVerbActionAst::ShuffleLibrary,
        ));
    }

    if has_trailing_that_player_shuffle(tokens) {
        let mut has_existing_shuffle = false;
        for effect in &mut effects {
            if let EffectAst::SubjectVerb(subject_verb) = effect
                && matches!(subject_verb.action, SubjectVerbActionAst::ShuffleLibrary)
            {
                has_existing_shuffle = true;
                if matches!(
                    subject_verb.subject.player,
                    PlayerAst::You | PlayerAst::Implicit
                ) {
                    subject_verb.subject.player = PlayerAst::That;
                }
            }
        }
        if !has_existing_shuffle {
            effects.push(EffectAst::subject_verb(
                SubjectVerbRoleAst::LibraryOwner,
                PlayerAst::That,
                SubjectVerbActionAst::ShuffleLibrary,
            ));
        }
    }

    if let Some(target) = search_player_target {
        effects.insert(0, EffectAst::subject_verb_target_only(target));
    }

    if let Some(trailing_tokens) = find_search_library_trailing_life_followup_lexed(
        search_tokens,
        put_idx.unwrap_or(filter_boundary),
    ) {
        let trailing_effect = parse_effect_clause_lexed(trailing_tokens)?;
        effects.push(trailing_effect);
    }

    if let Some(trailing_tokens) = trailing_create_followup {
        let trailing_effect = parse_effect_clause_lexed(trailing_tokens)?;
        effects.push(trailing_effect);
        if original_shuffle {
            effects.push(EffectAst::subject_verb(
                SubjectVerbRoleAst::LibraryOwner,
                player,
                SubjectVerbActionAst::ShuffleLibrary,
            ));
        }
    }

    if let Some(reference) = same_name_reference {
        match reference {
            SearchLibrarySameNameReference::Tagged(_) => {}
            SearchLibrarySameNameReference::Target(target) => {
                effects.insert(0, EffectAst::subject_verb_target_only(target));
            }
            SearchLibrarySameNameReference::Choose { filter, tag } => {
                if same_name_relation == TaggedOpbjectRelation::DifferentNameFromTagged {
                    effects.insert(
                        0,
                        EffectAst::subject_verb_tag_matching_objects(
                            filter,
                            vec![Zone::Battlefield],
                            tag,
                        ),
                    );
                } else {
                    effects.insert(
                        0,
                        EffectAst::ChooseObjects {
                            filter,
                            count: ChoiceCount::exactly(1),
                            count_value: None,
                            player,
                            tag,
                        },
                    );
                }
            }
        }
    }

    if sentence_has_direct_may && !handled_direct_may_in_iterated_search {
        effects = vec![if matches!(chooser, PlayerAst::You | PlayerAst::Implicit) {
            EffectAst::May { effects }
        } else {
            EffectAst::MayByPlayer {
                player: chooser,
                effects,
            }
        }];
    }

    if let Some(filter) = player_iteration_filter {
        effects = vec![match filter {
            PlayerFilter::Opponent => EffectAst::ForEachOpponent { effects },
            PlayerFilter::Any => EffectAst::ForEachPlayer { effects },
            other => EffectAst::ForEachPlayersFiltered {
                filter: other,
                effects,
            },
        }];
    }

    if let Some(iterated_filter) = iterated_subject_filter
        && !has_explicit_destination
        && !same_name_reference_requires_setup
    {
        effects = vec![EffectAst::ForEachObject {
            filter: iterated_filter,
            effects,
        }];
    }

    if !leading_effects.is_empty() {
        leading_effects.extend(effects);
        return Ok(Some(leading_effects));
    }

    if wrap_each_target_player {
        effects = vec![EffectAst::ForEachPlayersFiltered {
            filter: PlayerFilter::target_player(),
            effects,
        }];
    }

    Ok(Some(effects))
}
pub(crate) fn cant_sentence_has_source_remains_tapped_duration(tokens: &[OwnedLexToken]) -> bool {
    let mut has_for_as_long_as = false;
    let mut has_remains = false;
    let mut has_tapped = false;
    let mut has_source_word = false;
    let mut cursor = 0usize;

    while cursor < tokens.len() {
        if !has_for_as_long_as
            && primitives::parse_prefix(&tokens[cursor..], cant_sentence_for_as_long_as_marker)
                .is_some()
        {
            has_for_as_long_as = true;
        }

        let token = &tokens[cursor];
        has_remains |= token_is_any_word(token, &["remains"]);
        has_tapped |= token_is_any_word(token, &["tapped"]);
        has_source_word |= token_is_any_word(
            token,
            &["this", "source", "artifact", "creature", "permanent"],
        );
        cursor += 1;
    }

    has_for_as_long_as && has_remains && has_tapped && has_source_word
}
