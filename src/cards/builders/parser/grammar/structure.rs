use winnow::combinator::alt;
use winnow::error::{ContextError, ErrMode, StrContext, StrContextValue};
use winnow::prelude::*;
use winnow::stream::Stream;
use winnow::token::any;

use crate::cards::TextSpan;
use crate::cards::builders::{CardTextError, EffectAst, IfResultPredicate, PredicateAst};
use crate::effect::Value;

use super::super::lexer::{LexStream, LexToken, OwnedLexToken, TokenKind, trim_lexed_commas};
use super::{primitives, values};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ModalHeaderChooseSpec {
    pub(crate) choose_idx: usize,
    pub(crate) min: Value,
    pub(crate) max: Option<Value>,
    pub(crate) x_clause_start: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ModalHeaderFlags {
    pub(crate) commander_allows_both: bool,
    pub(crate) same_mode_more_than_once: bool,
    pub(crate) mode_must_be_unchosen: bool,
    pub(crate) mode_must_be_unchosen_this_turn: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TrailingModalGateSpec<'a> {
    pub(crate) prefix_tokens: &'a [OwnedLexToken],
    pub(crate) predicate: IfResultPredicate,
    pub(crate) remove_mode_only: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MetadataLineKind {
    ManaCost,
    TypeLine,
    PowerToughness,
    Loyalty,
    Defense,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MetadataLineSpec<'a> {
    pub(crate) kind: MetadataLineKind,
    pub(crate) value_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LeadingResultPrefixKind {
    If,
    When,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LeadingResultPrefixSpec<'a> {
    pub(crate) kind: LeadingResultPrefixKind,
    pub(crate) predicate: IfResultPredicate,
    pub(crate) trailing_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TrailingIfClauseSpec<'a> {
    pub(crate) leading_tokens: &'a [OwnedLexToken],
    pub(crate) predicate: PredicateAst,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum IfClausePredicateSpec {
    Conditional(PredicateAst),
    Result(IfResultPredicate),
}

#[derive(Debug, Clone)]
pub(crate) struct IfClauseSplitSpec {
    pub(crate) predicate: IfClausePredicateSpec,
    pub(crate) effects: Vec<EffectAst>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ConditionalPredicateTailSpec {
    Plain(PredicateAst),
    InsteadIf {
        base_predicate: PredicateAst,
        outer_predicate: PredicateAst,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TriggeredConditionalClauseSpec<'a> {
    pub(crate) trigger_tokens: &'a [OwnedLexToken],
    pub(crate) predicate: PredicateAst,
    pub(crate) effects_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct StateTriggeredClauseSpec<'a> {
    pub(crate) trigger_tokens: &'a [OwnedLexToken],
    pub(crate) display_tokens: &'a [OwnedLexToken],
    pub(crate) predicate: PredicateAst,
    pub(crate) effects_tokens: &'a [OwnedLexToken],
}

fn is_sentence_quote(token: &LexToken) -> bool {
    token.kind == TokenKind::Quote && matches!(token.slice.as_str(), "\"" | "“" | "”")
}

fn parse_remove_mode_only_prefix<'a>(
    input: &mut LexStream<'a>,
) -> Result<(), ErrMode<ContextError>> {
    (
        alt((primitives::kw("you"), primitives::kw("they"))),
        alt((primitives::kw("remove"), primitives::kw("removed"))),
    )
        .void()
        .parse_next(input)
}

fn normalized_parser_word(token: &LexToken) -> Option<String> {
    match token.kind {
        TokenKind::Word | TokenKind::Number | TokenKind::Tilde | TokenKind::Half => Some(
            token
                .parser_text()
                .chars()
                .filter(|ch| !matches!(*ch, '\'' | '’' | '‘'))
                .collect(),
        ),
        _ => None,
    }
}

fn parser_text_non_article_words(tokens: &[LexToken]) -> Vec<String> {
    tokens
        .iter()
        .filter_map(normalized_parser_word)
        .filter(|word| !matches!(word.as_str(), "a" | "an" | "the"))
        .collect()
}

pub(crate) fn split_metadata_line_lexed(tokens: &[OwnedLexToken]) -> Option<MetadataLineSpec<'_>> {
    fn match_metadata_prefix<'a>(
        tokens: &'a [OwnedLexToken],
        phrase: &'static [&'static str],
        kind: MetadataLineKind,
    ) -> Option<MetadataLineSpec<'a>> {
        let (_, value_tokens) =
            primitives::parse_prefix(tokens, (primitives::phrase(phrase), primitives::colon()))?;
        Some(MetadataLineSpec { kind, value_tokens })
    }

    match_metadata_prefix(tokens, &["mana", "cost"], MetadataLineKind::ManaCost)
        .or_else(|| match_metadata_prefix(tokens, &["type", "line"], MetadataLineKind::TypeLine))
        .or_else(|| match_metadata_prefix(tokens, &["type"], MetadataLineKind::TypeLine))
        .or_else(|| {
            match_metadata_prefix(
                tokens,
                &["power/toughness"],
                MetadataLineKind::PowerToughness,
            )
        })
        .or_else(|| match_metadata_prefix(tokens, &["loyalty"], MetadataLineKind::Loyalty))
        .or_else(|| match_metadata_prefix(tokens, &["defense"], MetadataLineKind::Defense))
}

pub(crate) fn looks_like_pact_next_upkeep_line_lexed(tokens: &[OwnedLexToken]) -> bool {
    primitives::contains_phrase(
        tokens,
        &["at", "the", "beginning", "of", "your", "next", "upkeep"],
    ) && primitives::contains_phrase(tokens, &["lose", "the", "game"])
        && (primitives::contains_phrase(tokens, &["if", "you", "dont"])
            || primitives::contains_phrase(tokens, &["if", "you", "don't"])
            || primitives::contains_phrase(tokens, &["if", "you", "do", "not"]))
}

pub(crate) fn looks_like_untap_all_during_each_other_players_untap_step_line_lexed(
    tokens: &[OwnedLexToken],
) -> bool {
    primitives::parse_prefix(tokens, primitives::phrase(&["untap", "all"])).is_some()
        && primitives::contains_any_phrase(
            tokens,
            &[
                &["during", "each", "other", "player's", "untap", "step"],
                &["during", "each", "other", "players", "untap", "step"],
            ],
        )
}

pub(crate) fn looks_like_next_turn_cant_cast_line_lexed(tokens: &[OwnedLexToken]) -> bool {
    primitives::contains_any_phrase(
        tokens,
        &[
            &["during", "that", "player's", "next", "turn"],
            &["during", "that", "players", "next", "turn"],
        ],
    ) && primitives::contains_any_phrase(
        tokens,
        &[
            &["can't", "cast"],
            &["cant", "cast"],
            &["can", "not", "cast"],
        ],
    )
}

pub(crate) fn looks_like_divvy_statement_line_lexed(tokens: &[OwnedLexToken]) -> bool {
    primitives::contains_any_phrase(
        tokens,
        &[
            &["into", "two", "piles"],
            &["into", "three", "piles"],
            &["chooses", "two", "of", "those", "cards"],
            &["chooses", "one", "of", "those", "piles"],
            &["pile", "of", "your", "choice"],
            &["pile", "of", "that", "player's", "choice"],
            &["chosen", "pile"],
            &["chosen", "piles"],
        ],
    )
}

pub(crate) fn looks_like_vote_statement_line_lexed(tokens: &[OwnedLexToken]) -> bool {
    (primitives::parse_prefix(tokens, primitives::phrase(&["starting", "with"])).is_some()
        || primitives::parse_prefix(tokens, primitives::phrase(&["each", "player", "votes"]))
            .is_some()
        || primitives::parse_prefix(
            tokens,
            primitives::phrase(&["each", "player", "secretly", "votes"]),
        )
        .is_some())
        && (primitives::contains_word(tokens, "vote")
            || primitives::contains_word(tokens, "votes")
            || primitives::contains_word(tokens, "voting"))
}

fn is_statement_verb_word(word: &str) -> bool {
    matches!(
        word,
        "add"
            | "adds"
            | "choose"
            | "chooses"
            | "counter"
            | "counters"
            | "create"
            | "creates"
            | "deal"
            | "deals"
            | "destroy"
            | "destroys"
            | "discard"
            | "discards"
            | "draw"
            | "draws"
            | "enchant"
            | "enchants"
            | "exchange"
            | "exchanges"
            | "exile"
            | "exiles"
            | "gain"
            | "gains"
            | "look"
            | "looks"
            | "mill"
            | "mills"
            | "put"
            | "puts"
            | "return"
            | "returns"
            | "reveal"
            | "reveals"
            | "sacrifice"
            | "sacrifices"
            | "search"
            | "searches"
            | "shuffle"
            | "shuffles"
            | "surveil"
            | "tap"
            | "taps"
            | "until"
            | "untap"
            | "untaps"
    )
}

pub(crate) fn looks_like_generic_statement_line_lexed(tokens: &[OwnedLexToken]) -> bool {
    let words = tokens
        .iter()
        .filter_map(OwnedLexToken::as_word)
        .map(|word| word.to_ascii_lowercase())
        .collect::<Vec<_>>();
    let word_refs = words.iter().map(String::as_str).collect::<Vec<_>>();
    if word_refs.is_empty() {
        return false;
    }

    let starts_with_each_player_statement = matches!(
        word_refs.as_slice(),
        ["each", "player", third, ..] if is_statement_verb_word(third)
    );
    let starts_with_quantified_target_player_statement = matches!(
        word_refs.as_slice(),
        [_, "target", "player", fourth, ..] | [_, "target", "players", fourth, ..]
            if is_statement_verb_word(fourth)
    );

    starts_with_each_player_statement
        || starts_with_quantified_target_player_statement
        || is_statement_verb_word(word_refs[0])
        || matches!(word_refs.as_slice(), ["this", "spell", third, ..] if is_statement_verb_word(third))
        || matches!(word_refs.as_slice(), [_, second, ..] if is_statement_verb_word(second))
        || matches!(word_refs.first(), Some(&"target")
            if word_refs.iter().skip(1).any(|word| is_statement_verb_word(word)))
}

pub(crate) fn looks_like_generic_static_line_lexed(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_prefix(tokens, primitives::phrase(&["this"])).is_some()
        || primitives::parse_prefix(tokens, primitives::phrase(&["enchanted"])).is_some()
        || primitives::parse_prefix(tokens, primitives::phrase(&["equipped"])).is_some()
        || primitives::parse_prefix(tokens, primitives::phrase(&["fortified"])).is_some()
        || primitives::parse_prefix(tokens, primitives::phrase(&["spells"])).is_some()
        || primitives::parse_prefix(tokens, primitives::phrase(&["creatures"])).is_some()
        || primitives::parse_prefix(tokens, primitives::phrase(&["other"])).is_some()
        || primitives::parse_prefix(tokens, primitives::phrase(&["each"])).is_some()
        || primitives::parse_prefix(tokens, primitives::phrase(&["as", "long", "as"])).is_some()
        || primitives::contains_word(tokens, "can't")
        || primitives::contains_word(tokens, "can")
        || primitives::contains_word(tokens, "has")
        || primitives::contains_word(tokens, "have")
        || primitives::contains_phrase(tokens, &["maximum", "hand", "size"])
}

fn parse_modeled_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let predicate = parse_predicate_with_grammar_entrypoint_lexed(tokens).ok()?;
    if matches!(predicate, PredicateAst::Unmodeled(_)) {
        return None;
    }

    Some(predicate)
}

fn classify_if_result_predicate(words: &[&str]) -> Option<IfResultPredicate> {
    let is_result_verb = |word: &str| {
        matches!(
            word,
            "remove"
                | "removed"
                | "sacrifice"
                | "sacrificed"
                | "discard"
                | "discarded"
                | "exile"
                | "exiled"
        )
    };
    let is_unqualified_this_way_result = |subject: &str| {
        if words.len() < 4
            || words[0] != subject
            || !is_result_verb(words[1])
            || words[words.len() - 2] != "this"
            || words[words.len() - 1] != "way"
        {
            return false;
        }
        let qualifiers = &words[2..words.len() - 2];
        matches!(qualifiers, [] | ["it"] | ["them"] | ["that"])
    };
    let is_exact_negated_result = |subject: &str| {
        (words.len() == 2 && words[0] == subject && matches!(words[1], "dont" | "didnt" | "cant"))
            || (words.len() == 3
                && words[0] == subject
                && (matches!(words[1], "do" | "did" | "can") && words[2] == "not"))
    };
    let is_negated_this_way_result = |subject: &str| {
        let action_idx =
            if words.len() >= 5 && words[0] == subject && matches!(words[1], "dont" | "didnt") {
                2
            } else if words.len() >= 6
                && words[0] == subject
                && ((words[1] == "do" && words[2] == "not")
                    || (words[1] == "did" && words[2] == "not"))
            {
                3
            } else {
                return false;
            };
        if !is_result_verb(words[action_idx])
            || words[words.len() - 2] != "this"
            || words[words.len() - 1] != "way"
        {
            return false;
        }
        let qualifiers = &words[action_idx + 1..words.len() - 2];
        matches!(qualifiers, [] | ["it"] | ["them"] | ["that"])
    };

    if words.len() == 2 && words[0] == "you" && words[1] == "do" {
        return Some(IfResultPredicate::Did);
    }
    if words.len() >= 2
        && words[0] == "you"
        && (words[1] == "win" || words[1] == "won")
        && (words.len() == 2 || words.iter().any(|word| *word == "clash"))
    {
        return Some(IfResultPredicate::Did);
    }
    if words.len() >= 3
        && words[0] == "you"
        && (words[1] == "win" || words[1] == "won")
        && words.contains(&"flip")
    {
        return Some(IfResultPredicate::Did);
    }
    if words.len() == 2 && words[0] == "they" && words[1] == "do" {
        return Some(IfResultPredicate::Did);
    }
    if words.len() == 2
        && (words[0] == "player" || words[0] == "players")
        && (words[1] == "do" || words[1] == "does")
    {
        return Some(IfResultPredicate::Did);
    }
    if words.len() == 3
        && words[0] == "that"
        && words[1] == "player"
        && (words[2] == "do" || words[2] == "does")
    {
        return Some(IfResultPredicate::Did);
    }
    if words.len() >= 6
        && words[0] == "you"
        && words[1] == "searched"
        && words[words.len() - 2] == "this"
        && words[words.len() - 1] == "way"
    {
        return Some(IfResultPredicate::Did);
    }
    if is_unqualified_this_way_result("you") {
        return Some(IfResultPredicate::Did);
    }
    if is_unqualified_this_way_result("they") {
        return Some(IfResultPredicate::Did);
    }

    if words.len() >= 5
        && (words[0] == "that" || words[0] == "it")
        && words[1] == "spell"
        && words.iter().any(|word| *word == "countered")
        && words[words.len() - 2] == "this"
        && words[words.len() - 1] == "way"
    {
        return Some(IfResultPredicate::Did);
    }

    if words.len() >= 5
        && (words[0] == "that" || words[0] == "it")
        && (words[1] == "creature" || words[1] == "permanent" || words[1] == "card")
        && words[2] == "dies"
        && words[3] == "this"
        && words[4] == "way"
    {
        return Some(IfResultPredicate::DiesThisWay);
    }
    if words.len() >= 8
        && matches!(words[0], "creature" | "permanent" | "card")
        && words[1] == "dealt"
        && words[2] == "damage"
        && words[3] == "this"
        && words[4] == "way"
        && words[5] == "would"
        && words[6] == "die"
        && words[7] == "this"
        && words.get(8) == Some(&"turn")
    {
        return Some(IfResultPredicate::DiesThisWay);
    }

    if matches!(
        words,
        ["it", "deals", "excess", "damage", "this", "way"]
            | ["its", "power", "becomes", _, "this", "way"]
            | ["it", "power", "becomes", _, "this", "way"]
    ) {
        return Some(IfResultPredicate::Did);
    }

    if is_exact_negated_result("you") || is_negated_this_way_result("you") {
        return Some(IfResultPredicate::DidNot);
    }
    if words.len() >= 3
        && words[0] == "you"
        && matches!(words[1], "lose" | "lost")
        && words.contains(&"flip")
    {
        return Some(IfResultPredicate::DidNot);
    }
    if is_exact_negated_result("they") || is_negated_this_way_result("they") {
        return Some(IfResultPredicate::DidNot);
    }
    if (words.len() == 2
        && matches!(words[0], "player" | "players")
        && matches!(words[1], "dont" | "doesnt" | "didnt" | "cant"))
        || (words.len() == 3
            && matches!(words[0], "player" | "players")
            && matches!(words[1], "do" | "does" | "did" | "can")
            && words[2] == "not")
    {
        return Some(IfResultPredicate::DidNot);
    }
    if (words.len() == 3
        && words[0] == "that"
        && words[1] == "player"
        && matches!(words[2], "dont" | "doesnt" | "didnt" | "cant"))
        || (words.len() == 4
            && words[0] == "that"
            && words[1] == "player"
            && matches!(words[2], "do" | "does" | "did" | "can")
            && words[3] == "not")
    {
        return Some(IfResultPredicate::DidNot);
    }

    None
}

fn parse_if_result_predicate_inner<'a>(
    input: &mut LexStream<'a>,
) -> Result<IfResultPredicate, ErrMode<ContextError>> {
    let tokens = input.peek_finish();
    let words = parser_text_non_article_words(tokens);
    let word_refs: Vec<&str> = words.iter().map(String::as_str).collect();
    let Some(predicate) = classify_if_result_predicate(&word_refs) else {
        return Err(primitives::backtrack_err(
            "if-result predicate",
            "result predicate clause",
        ));
    };

    input.finish();
    Ok(predicate)
}

pub(crate) fn parse_if_result_predicate(tokens: &[OwnedLexToken]) -> Option<IfResultPredicate> {
    primitives::parse_prefix(tokens, parse_if_result_predicate_inner)
        .and_then(|(predicate, rest)| rest.is_empty().then_some(predicate))
}

fn parse_sentence_segment_len<'a>(
    input: &mut LexStream<'a>,
) -> Result<usize, ErrMode<ContextError>> {
    fn quoted_period_continues_sentence(next: Option<&LexToken>) -> bool {
        match next {
            Some(token) if token.kind == TokenKind::Comma => true,
            Some(token)
                if token.kind == TokenKind::Word
                    && matches!(
                        token.parser_text(),
                        "and" | "during" | "for" | "this" | "until" | "where" | "with" | "without"
                    ) =>
            {
                true
            }
            _ => false,
        }
    }

    let initial_len = input.len();
    let mut inside_quotes = false;
    let mut last_inner_token_was_period = false;

    while let Some(token) = input.peek_token() {
        if is_sentence_quote(token) {
            primitives::quote().parse_next(input)?;
            if inside_quotes
                && last_inner_token_was_period
                && !quoted_period_continues_sentence(input.peek_token())
            {
                let consumed = initial_len - input.len();
                return Ok(consumed);
            }
            inside_quotes = !inside_quotes;
            last_inner_token_was_period = false;
            continue;
        }

        if token.kind == TokenKind::Period {
            primitives::period().parse_next(input)?;
            if inside_quotes {
                last_inner_token_was_period = true;
                continue;
            }

            let consumed = initial_len - input.len();
            return Ok(consumed.saturating_sub(1));
        }

        any.parse_next(input)?;
        last_inner_token_was_period = false;
    }

    Ok(initial_len - input.len())
}

pub(crate) fn split_lexed_sentences<'a>(tokens: &'a [OwnedLexToken]) -> Vec<&'a [OwnedLexToken]> {
    let mut segments = Vec::new();
    let mut remaining = tokens;

    while !remaining.is_empty() {
        let Some((segment_len, rest)) =
            primitives::parse_prefix(remaining, parse_sentence_segment_len)
        else {
            break;
        };

        if segment_len > 0 {
            segments.push(&remaining[..segment_len]);
        }

        if rest.len() == remaining.len() {
            break;
        }
        remaining = rest;
    }

    segments
}

pub(crate) fn scan_modal_header_flags(tokens: &[OwnedLexToken]) -> ModalHeaderFlags {
    let mode_must_be_unchosen_this_turn = primitives::contains_any_phrase(
        tokens,
        &[
            &["that", "hasnt", "been", "chosen", "this", "turn"],
            &["that", "hasn't", "been", "chosen", "this", "turn"],
            &["that", "has", "not", "been", "chosen", "this", "turn"],
        ],
    );
    let mode_must_be_unchosen = mode_must_be_unchosen_this_turn
        || primitives::contains_any_phrase(
            tokens,
            &[
                &["that", "hasnt", "been", "chosen"],
                &["that", "hasn't", "been", "chosen"],
                &["that", "has", "not", "been", "chosen"],
            ],
        );

    ModalHeaderFlags {
        commander_allows_both: primitives::contains_word(tokens, "commander")
            && primitives::contains_word(tokens, "both"),
        same_mode_more_than_once: primitives::contains_phrase(
            tokens,
            &["same", "mode", "more", "than", "once"],
        ),
        mode_must_be_unchosen,
        mode_must_be_unchosen_this_turn,
    }
}

pub(crate) fn split_leading_result_prefix_lexed<'a>(
    tokens: &'a [OwnedLexToken],
) -> Option<LeadingResultPrefixSpec<'a>> {
    let trimmed = trim_lexed_commas(tokens);
    let kind = if trimmed.first().is_some_and(|token| token.is_word("if")) {
        LeadingResultPrefixKind::If
    } else if trimmed.first().is_some_and(|token| token.is_word("when")) {
        LeadingResultPrefixKind::When
    } else {
        return None;
    };

    let comma_idx = primitives::find_token_index(trimmed, |token| token.kind == TokenKind::Comma)?;
    if comma_idx <= 1 || comma_idx + 1 >= trimmed.len() {
        return None;
    }

    let predicate_tokens = trim_lexed_commas(&trimmed[1..comma_idx]);
    if predicate_tokens.is_empty() {
        return None;
    }
    let predicate = parse_if_result_predicate(predicate_tokens)?;

    let trailing_tokens = trim_lexed_commas(&trimmed[comma_idx + 1..]);
    if trailing_tokens.is_empty() {
        return None;
    }

    Some(LeadingResultPrefixSpec {
        kind,
        predicate,
        trailing_tokens,
    })
}

pub(crate) fn split_trailing_if_clause_lexed<'a>(
    tokens: &'a [OwnedLexToken],
) -> Option<TrailingIfClauseSpec<'a>> {
    split_trailing_predicate_clause_lexed(tokens, "if")
}

pub(crate) fn parse_predicate_with_grammar_entrypoint_lexed(
    tokens: &[OwnedLexToken],
) -> Result<PredicateAst, CardTextError> {
    super::filters::parse_predicate(tokens)
}

pub(crate) fn split_if_clause_lexed(
    tokens: &[OwnedLexToken],
    mut parse_effects: impl FnMut(&[OwnedLexToken]) -> Result<Vec<EffectAst>, CardTextError>,
) -> Result<IfClauseSplitSpec, CardTextError> {
    if let Some(effect_token_idx) =
        primitives::find_phrase_start(tokens, &["exile", "them", "then", "meld", "them", "into"])
    {
        let predicate_tokens = trim_lexed_commas(&tokens[1..effect_token_idx]);
        let predicate_tokens_without_commas = predicate_tokens
            .iter()
            .filter(|token| !token.is_comma())
            .cloned()
            .collect::<Vec<_>>();
        let effect_tokens = &tokens[effect_token_idx..];
        if !predicate_tokens_without_commas.is_empty() {
            if let Ok(predicate) =
                parse_predicate_with_grammar_entrypoint_lexed(&predicate_tokens_without_commas)
                && let Ok(effects) = parse_effects(effect_tokens)
                && !effects.is_empty()
            {
                return Ok(IfClauseSplitSpec {
                    predicate: IfClausePredicateSpec::Conditional(predicate),
                    effects,
                });
            }
            if let Some(predicate) = parse_if_result_predicate(&predicate_tokens_without_commas)
                && let Ok(effects) = parse_effects(effect_tokens)
                && !effects.is_empty()
            {
                return Ok(IfClauseSplitSpec {
                    predicate: IfClausePredicateSpec::Result(predicate),
                    effects,
                });
            }
        }
    }

    let comma_indices = tokens
        .iter()
        .enumerate()
        .filter_map(|(idx, token)| token.is_comma().then_some(idx))
        .collect::<Vec<_>>();
    if comma_indices.is_empty() {
        for split_idx in (2..tokens.len()).rev() {
            let predicate_tokens = &tokens[1..split_idx];
            let effect_tokens = trim_lexed_commas(&tokens[split_idx..]);
            if effect_tokens.is_empty() {
                continue;
            }
            if let Ok(predicate) = parse_predicate_with_grammar_entrypoint_lexed(predicate_tokens)
                && let Ok(effects) = parse_effects(effect_tokens)
                && !effects.is_empty()
            {
                return Ok(IfClauseSplitSpec {
                    predicate: IfClausePredicateSpec::Conditional(predicate),
                    effects,
                });
            }
            if let Some(predicate) = parse_if_result_predicate(predicate_tokens)
                && let Ok(effects) = parse_effects(effect_tokens)
                && !effects.is_empty()
            {
                return Ok(IfClauseSplitSpec {
                    predicate: IfClausePredicateSpec::Result(predicate),
                    effects,
                });
            }
        }
        return Err(CardTextError::ParseError(
            "missing comma in if clause".to_string(),
        ));
    }

    let first_comma_idx = comma_indices[0];
    if first_comma_idx > 1 {
        let predicate_tokens = &tokens[1..first_comma_idx];
        if let Ok(predicate) = parse_predicate_with_grammar_entrypoint_lexed(predicate_tokens) {
            let effect_tokens = &tokens[first_comma_idx + 1..];
            let comma_fragment_looks_like_effect = if comma_indices.len() > 1 {
                let fragment_tokens = &tokens[first_comma_idx + 1..comma_indices[1]];
                parse_effects(fragment_tokens)
                    .map(|effects| !effects.is_empty())
                    .unwrap_or(false)
            } else {
                true
            };
            if comma_fragment_looks_like_effect
                && let Ok(effects) = parse_effects(effect_tokens)
                && !effects.is_empty()
            {
                return Ok(IfClauseSplitSpec {
                    predicate: IfClausePredicateSpec::Conditional(predicate),
                    effects,
                });
            }
        }
        if let Some(predicate) = parse_if_result_predicate(predicate_tokens) {
            let effect_tokens = &tokens[first_comma_idx + 1..];
            let effects = parse_effects(effect_tokens)?;
            return Ok(IfClauseSplitSpec {
                predicate: IfClausePredicateSpec::Result(predicate),
                effects,
            });
        }
    }

    let mut split: Option<(usize, Vec<EffectAst>)> = None;
    for idx in comma_indices.iter().rev().copied() {
        let effect_tokens = &tokens[idx + 1..];
        if effect_tokens.is_empty() {
            continue;
        }
        if let Ok(effects) = parse_effects(effect_tokens)
            && !effects.is_empty()
        {
            split = Some((idx, effects));
            break;
        }
    }

    let (comma_idx, effects) = if let Some(split) = split {
        split
    } else {
        let first_idx = comma_indices[0];
        let effect_tokens = &tokens[first_idx + 1..];
        (first_idx, parse_effects(effect_tokens)?)
    };
    let predicate_tokens = &tokens[1..comma_idx];

    if let Ok(predicate) = parse_predicate_with_grammar_entrypoint_lexed(predicate_tokens) {
        return Ok(IfClauseSplitSpec {
            predicate: IfClausePredicateSpec::Conditional(predicate),
            effects,
        });
    }
    let Some(predicate) = parse_if_result_predicate(predicate_tokens) else {
        let predicate = parse_predicate_with_grammar_entrypoint_lexed(predicate_tokens)?;
        return Ok(IfClauseSplitSpec {
            predicate: IfClausePredicateSpec::Conditional(predicate),
            effects,
        });
    };

    Ok(IfClauseSplitSpec {
        predicate: IfClausePredicateSpec::Result(predicate),
        effects,
    })
}

pub(crate) fn split_trailing_unless_clause_lexed<'a>(
    tokens: &'a [OwnedLexToken],
) -> Option<TrailingIfClauseSpec<'a>> {
    split_trailing_predicate_clause_lexed(tokens, "unless")
}

pub(crate) fn parse_trailing_if_predicate_lexed(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let trimmed = trim_lexed_commas(tokens);
    if !trimmed.first().is_some_and(|token| token.is_word("if")) {
        return None;
    }

    let predicate_tokens = trim_lexed_commas(&trimmed[1..]);
    if predicate_tokens.is_empty() {
        return None;
    }

    parse_predicate_with_grammar_entrypoint_lexed(predicate_tokens).ok()
}

pub(crate) fn parse_conditional_predicate_tail_lexed(
    tokens: &[OwnedLexToken],
) -> Option<ConditionalPredicateTailSpec> {
    let mut trimmed = trim_lexed_commas(tokens).to_vec();
    while trimmed.last().is_some_and(|token| token.is_word("instead")) {
        trimmed.pop();
    }
    let trimmed = trim_lexed_commas(&trimmed);
    if trimmed.is_empty() {
        return None;
    }

    let mut instead_if_idx = None;
    let mut idx = 0usize;
    while idx < trimmed.len() {
        if primitives::parse_prefix(
            &trimmed[idx..],
            (primitives::kw("instead"), primitives::kw("if")),
        )
        .is_some()
        {
            instead_if_idx = Some(idx);
            break;
        }
        idx += 1;
    }

    if let Some(instead_if_idx) = instead_if_idx {
        let base_predicate_tokens = trim_lexed_commas(&trimmed[..instead_if_idx]);
        let outer_predicate_tokens = trim_lexed_commas(&trimmed[instead_if_idx + 2..]);
        if base_predicate_tokens.is_empty() || outer_predicate_tokens.is_empty() {
            return None;
        }

        let base_predicate =
            parse_predicate_with_grammar_entrypoint_lexed(base_predicate_tokens).ok()?;
        let outer_predicate =
            parse_predicate_with_grammar_entrypoint_lexed(outer_predicate_tokens).ok()?;
        return Some(ConditionalPredicateTailSpec::InsteadIf {
            base_predicate,
            outer_predicate,
        });
    }

    let predicate = parse_predicate_with_grammar_entrypoint_lexed(trimmed).ok()?;
    Some(ConditionalPredicateTailSpec::Plain(predicate))
}

fn split_trailing_predicate_clause_lexed<'a>(
    tokens: &'a [OwnedLexToken],
    keyword: &'static str,
) -> Option<TrailingIfClauseSpec<'a>> {
    let split_idx = primitives::rfind_token_index(tokens, |token| token.is_word(keyword))?;
    if split_idx == 0 || split_idx + 1 >= tokens.len() {
        return None;
    }

    let predicate_tokens = trim_lexed_commas(&tokens[split_idx + 1..]);
    if predicate_tokens.is_empty() {
        return None;
    }
    let predicate = parse_predicate_with_grammar_entrypoint_lexed(predicate_tokens).ok()?;

    let leading_tokens = trim_lexed_commas(&tokens[..split_idx]);
    if leading_tokens.is_empty() {
        return None;
    }

    Some(TrailingIfClauseSpec {
        leading_tokens,
        predicate,
    })
}

pub(crate) fn parse_who_player_predicate_lexed(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let trimmed = trim_lexed_commas(tokens);
    if !trimmed.first().is_some_and(|token| token.is_word("who")) {
        return None;
    }

    let predicate_tail = trim_lexed_commas(&trimmed[1..]);
    if predicate_tail.is_empty() {
        return None;
    }

    let mut predicate_tokens = Vec::with_capacity(predicate_tail.len() + 2);
    predicate_tokens.push(OwnedLexToken::word(
        "that".to_string(),
        TextSpan::synthetic(),
    ));
    predicate_tokens.push(OwnedLexToken::word(
        "player".to_string(),
        TextSpan::synthetic(),
    ));
    predicate_tokens.extend(predicate_tail.iter().cloned());

    parse_predicate_with_grammar_entrypoint_lexed(&predicate_tokens).ok()
}

pub(crate) fn parse_trailing_instead_if_predicate_lexed(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let trimmed = trim_lexed_commas(tokens);
    if !trimmed
        .first()
        .is_some_and(|token| token.is_word("instead"))
        || !trimmed.get(1).is_some_and(|token| token.is_word("if"))
    {
        return None;
    }

    let predicate_tokens = trim_lexed_commas(&trimmed[2..]);
    if predicate_tokens.is_empty() {
        return None;
    }

    parse_predicate_with_grammar_entrypoint_lexed(predicate_tokens).ok()
}

pub(crate) fn split_triggered_conditional_clause_lexed<'a>(
    tokens: &'a [OwnedLexToken],
    start_idx: usize,
) -> Option<TriggeredConditionalClauseSpec<'a>> {
    let (leading_tokens, after_first_comma) = primitives::split_lexed_once_on_comma(tokens)?;
    if leading_tokens.len() <= start_idx {
        return None;
    }

    let trigger_tokens = &leading_tokens[start_idx..];
    let after_first_comma = trim_lexed_commas(after_first_comma);
    let (_, after_if) = primitives::parse_prefix(after_first_comma, primitives::kw("if"))?;

    let (predicate_tokens, effects_tokens) = primitives::split_lexed_once_on_comma(after_if)?;
    let predicate_tokens = trim_lexed_commas(predicate_tokens);
    let effects_tokens = trim_lexed_commas(effects_tokens);
    if predicate_tokens.is_empty() || effects_tokens.is_empty() {
        return None;
    }

    Some(TriggeredConditionalClauseSpec {
        trigger_tokens,
        predicate: parse_modeled_predicate(predicate_tokens)?,
        effects_tokens,
    })
}

pub(crate) fn split_state_triggered_clause_lexed<'a>(
    tokens: &'a [OwnedLexToken],
    start_idx: usize,
    split_idx: usize,
) -> Option<StateTriggeredClauseSpec<'a>> {
    if split_idx <= start_idx || split_idx >= tokens.len() {
        return None;
    }
    if !tokens
        .first()
        .is_some_and(|token| token.is_word("when") || token.is_word("whenever"))
    {
        return None;
    }

    let trigger_tokens = &tokens[start_idx..split_idx];
    let effects_tokens = trim_lexed_commas(&tokens[split_idx + 1..]);
    if effects_tokens.is_empty() {
        return None;
    }

    Some(StateTriggeredClauseSpec {
        trigger_tokens,
        display_tokens: &tokens[..split_idx],
        predicate: parse_modeled_predicate(trigger_tokens)?,
        effects_tokens,
    })
}

pub(crate) fn split_trailing_modal_gate_clause<'a>(
    tokens: &'a [OwnedLexToken],
) -> Option<TrailingModalGateSpec<'a>> {
    let sentence_start =
        primitives::rfind_token_index(tokens, |token| token.kind == TokenKind::Period)
            .map(|idx| idx + 1)
            .unwrap_or(0);
    let sentence_tokens = trim_lexed_commas(&tokens[sentence_start..]);
    if sentence_tokens.is_empty() {
        return None;
    }
    let (_, predicate_tail) = primitives::parse_prefix(
        sentence_tokens,
        alt((primitives::kw("if"), primitives::kw("when"))),
    )?;
    let (predicate_tokens, trailing_tokens) = if let Some((predicate_tokens, trailing_tokens)) =
        primitives::split_lexed_once_on_comma(predicate_tail)
    {
        (
            trim_lexed_commas(predicate_tokens),
            trim_lexed_commas(trailing_tokens),
        )
    } else {
        (trim_lexed_commas(predicate_tail), &[][..])
    };
    if predicate_tokens.is_empty() || !trailing_tokens.is_empty() {
        return None;
    }

    let mut prefix_end = sentence_start;
    while prefix_end > 0 && tokens[prefix_end - 1].kind == TokenKind::Comma {
        prefix_end -= 1;
    }

    let predicate = parse_if_result_predicate(predicate_tokens)?;

    Some(TrailingModalGateSpec {
        prefix_tokens: &tokens[..prefix_end],
        predicate,
        remove_mode_only: primitives::parse_prefix(predicate_tokens, parse_remove_mode_only_prefix)
            .is_some(),
    })
}

fn parse_modal_header_choose_spec_inner<'a>(
    input: &mut LexStream<'a>,
) -> Result<Option<ModalHeaderChooseSpec>, ErrMode<ContextError>> {
    let tokens = input.peek_finish();
    let choose_indices = tokens
        .iter()
        .enumerate()
        .filter_map(|(idx, token)| token.is_word("choose").then_some(idx))
        .collect::<Vec<_>>();
    if choose_indices.is_empty() {
        input.finish();
        return Ok(None);
    }

    for choose_idx in choose_indices.iter().copied() {
        let choose_tail = &tokens[choose_idx + 1..];
        let Some((Some(min), max)) = values::parse_modal_choose_range(choose_tail).ok().flatten()
        else {
            continue;
        };
        let x_clause_start = primitives::find_phrase_start(choose_tail, &["x", "is"])
            .map(|idx| choose_idx + 1 + idx);

        input.finish();
        return Ok(Some(ModalHeaderChooseSpec {
            choose_idx,
            min,
            max,
            x_clause_start,
        }));
    }

    let choose_idx = *choose_indices.last().expect("checked non-empty");
    input.next_slice(choose_idx + 1);
    Err(primitives::cut_err_ctx(
        "modal header choose clause",
        "modal choice range",
    ))
}

pub(crate) fn parse_modal_header_choose_spec<'a>(
    input: &mut LexStream<'a>,
) -> Result<Option<ModalHeaderChooseSpec>, ErrMode<ContextError>> {
    parse_modal_header_choose_spec_inner
        .context(StrContext::Label("modal header"))
        .context(StrContext::Expected(StrContextValue::Description(
            "modal header line",
        )))
        .parse_next(input)
}
