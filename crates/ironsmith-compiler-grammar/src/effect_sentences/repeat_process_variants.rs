//! "Repeat this process" forms that name what changes or who acts.
//!
//! * "Then repeat this process for instant cards with mana values 2 and 1."
//!   (Firemind's Foresight) / "Repeat this process for first strike, double
//!   strike, ... and vigilance." (Kathril, Aspect Warper): the previous
//!   instruction is performed again once for each new value, in the order
//!   listed, with that value in place of the one it named.
//! * "Repeat the following process for each opponent in turn order." followed
//!   by the rest of the ability (Protection Racket): the following
//!   instructions are performed once for each opponent in turn, and "that
//!   player"/"they" in them is the opponent the process is for.
//!
//! The condition-driven loop ("repeat this process until/if/unless ...") is
//! [`crate::effect_sentences`]'s `RepeatProcessEffect`; these forms repeat a
//! fixed number of times with known values, so each repetition is the same
//! typed program read with its own value.

use super::dispatch_entry::SentenceInput;
use crate::cards::builders::{CardTextError, EffectAst, ForEachEffectAst};
use crate::grammar::primitives;
use crate::lexer::{OwnedLexToken, TokenKind, trim_lexed_commas};

/// What the "repeat this process for ..." sentence substitutes.
enum RepeatValues<'a> {
    /// "for <cards> with mana values 2 and 1": each number replaces the one
    /// number the previous instruction named.
    Numbers(Vec<&'a OwnedLexToken>),
    /// "for first strike, double strike, ... and vigilance": each keyword
    /// replaces the keyword the previous instruction named.
    Keywords(Vec<&'a [OwnedLexToken]>),
}

fn strip_sentence_edges(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    let mut tokens = trim_lexed_commas(tokens);
    while tokens.last().is_some_and(OwnedLexToken::is_period) {
        tokens = trim_lexed_commas(&tokens[..tokens.len() - 1]);
    }
    if tokens.first().is_some_and(|token| token.is_word("then")) {
        tokens = trim_lexed_commas(&tokens[1..]);
    }
    tokens
}

/// Items of a serial list: "a, b, and c" / "a and b".
fn list_items(tokens: &[OwnedLexToken]) -> Vec<&[OwnedLexToken]> {
    tokens
        .split(|token| token.is_comma() || token.is_word("and"))
        .filter(|item| !item.is_empty())
        .collect()
}

fn is_keyword_value(tokens: &[OwnedLexToken]) -> bool {
    !tokens.is_empty()
        && tokens.iter().all(|token| token.kind == TokenKind::Word)
        && !tokens
            .iter()
            .any(|token| token.is_any_word(&["counter", "counters"]))
        && crate::grammar::filters::parse_counter_type_from_tokens(tokens).is_some()
}

fn parse_repeat_values(tokens: &[OwnedLexToken]) -> Option<RepeatValues<'_>> {
    let tokens = strip_sentence_edges(tokens);
    let ((), rest) = primitives::parse_prefix(
        tokens,
        primitives::phrase(&["repeat", "this", "process", "for"]),
    )?;
    if let Some(first_number) = rest.iter().position(|token| token.kind == TokenKind::Number) {
        // The descriptor ("instant cards with mana values") names what the
        // numbers measure; it must precede them.
        if first_number == 0 {
            return None;
        }
        let items = list_items(&rest[first_number..]);
        if items.is_empty()
            || items
                .iter()
                .any(|item| item.len() != 1 || item[0].kind != TokenKind::Number)
        {
            return None;
        }
        return Some(RepeatValues::Numbers(items.iter().map(|item| &item[0]).collect()));
    }
    let items = list_items(rest);
    if items.is_empty() || !items.iter().all(|item| is_keyword_value(item)) {
        return None;
    }
    Some(RepeatValues::Keywords(items))
}

/// The previous instruction read once for each new value.
fn substituted_instructions(
    base: &[OwnedLexToken],
    values: &RepeatValues<'_>,
) -> Option<Vec<Vec<OwnedLexToken>>> {
    match values {
        RepeatValues::Numbers(numbers) => {
            let mut positions = base
                .iter()
                .enumerate()
                .filter(|(_, token)| token.kind == TokenKind::Number)
                .map(|(index, _)| index);
            let slot = positions.next()?;
            if positions.next().is_some() {
                // Which number changes would be ambiguous.
                return None;
            }
            Some(
                numbers
                    .iter()
                    .map(|number| {
                        let mut tokens = base.to_vec();
                        tokens[slot] = (*number).clone();
                        tokens
                    })
                    .collect(),
            )
        }
        RepeatValues::Keywords(keywords) => {
            // The keyword the instruction names: the first word run that
            // reads as a keyword counter type. Every occurrence is replaced
            // ("a flying counter ... has flying").
            let (start, len) = (0..base.len()).find_map(|start| {
                [2usize, 1]
                    .into_iter()
                    .find(|len| {
                        start + len <= base.len() && is_keyword_value(&base[start..start + len])
                    })
                    .map(|len| (start, len))
            })?;
            let slot = &base[start..start + len];
            let same_words = |candidate: &[OwnedLexToken]| {
                candidate.len() == slot.len()
                    && candidate
                        .iter()
                        .zip(slot)
                        .all(|(left, right)| left.kind == right.kind && left.parser_text == right.parser_text)
            };
            Some(
                keywords
                    .iter()
                    .map(|keyword| {
                        let mut tokens = Vec::with_capacity(base.len() + keyword.len());
                        let mut index = 0;
                        while index < base.len() {
                            if index + slot.len() <= base.len()
                                && same_words(&base[index..index + slot.len()])
                            {
                                tokens.extend(keyword.iter().cloned());
                                index += slot.len();
                            } else {
                                tokens.push(base[index].clone());
                                index += 1;
                            }
                        }
                        tokens
                    })
                    .collect(),
            )
        }
    }
}

/// "<instruction>. [Then] repeat this process for <values>." CR 608.2c: the
/// instructions are followed in order, so each repetition happens after the
/// previous one, with its own value.
pub(super) fn read_repeat_with_new_values(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let (Some(base), Some(repeat)) = (sentences.get(sentence_idx), sentences.get(sentence_idx + 1))
    else {
        return Ok(None);
    };
    let Some(values) = parse_repeat_values(repeat.lowered()) else {
        return Ok(None);
    };
    let base_tokens = strip_sentence_edges(base.lowered());
    let Some(repetitions) = substituted_instructions(base_tokens, &values) else {
        return Ok(None);
    };
    let mut effects = super::parse_effect_sentence_lexed(base.lowered())?;
    for repetition in repetitions {
        effects.extend(super::parse_effect_sentence_lexed(&repetition)?);
    }
    Ok(Some(effects))
}

/// "Repeat the following process for each opponent [in turn order]": true
/// when the turn order is stated.
fn following_process_for_each_opponent(tokens: &[OwnedLexToken]) -> Option<bool> {
    primitives::probe_all(
        strip_sentence_edges(tokens),
        (
            primitives::phrase(&[
                "repeat", "the", "following", "process", "for", "each", "opponent",
            ]),
            winnow::combinator::opt(primitives::phrase(&["in", "turn", "order"])),
        ),
        "repeat-following-process-for-each-opponent",
    )
    .map(|((), in_turn_order)| in_turn_order.is_some())
}

/// The following process runs to the end of the ability: `consumed` counts
/// the announcing sentence and every sentence after it.
pub(super) fn read_following_process_for_each_opponent(
    sentences: &[SentenceInput],
    sentence_idx: usize,
    consumed: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if sentences.len() != sentence_idx + consumed || consumed < 2 {
        return Ok(None);
    }
    let Some(in_turn_order) = following_process_for_each_opponent(sentences[sentence_idx].lowered())
    else {
        return Ok(None);
    };
    let body = sentences[sentence_idx + 1..]
        .iter()
        .map(|sentence| SentenceInput::from_lexed(sentence.lexed()))
        .collect::<Vec<_>>();
    // Within the body "that player" and "they" are the opponent the process
    // is for; the opponents go one at a time.
    let effects = super::dispatch_entry::parse_effect_sentences_from_sentence_inputs(body)?;
    let process = EffectAst::ForEach(ForEachEffectAst::ForEachOpponent { effects });
    Ok(Some(vec![if in_turn_order {
        // "in turn order": starting with the next opponent after the
        // controller (CR 101.4 sequencing).
        EffectAst::SourceSentence {
            effects: vec![process],
            leading_then: false,
            starting_with_controller: true,
        }
    } else {
        process
    }]))
}

/// "Each player exiles the top card of their library. The player who exiled
/// the card with the greatest mana value <action>. If two or more players'
/// cards are tied for greatest, the tied players repeat this process until
/// the tie is broken." (Timesifter): a repeated round among the tied players
/// (CR 608.2c) whose unique winner then performs the action.
pub(super) fn read_greatest_mana_value_tie_break(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let (Some(round), Some(winner), Some(tie)) = (
        sentences.get(sentence_idx),
        sentences.get(sentence_idx + 1),
        sentences.get(sentence_idx + 2),
    ) else {
        return Ok(None);
    };
    if primitives::probe_all(
        strip_sentence_edges(round.lowered()),
        primitives::phrase(&[
            "each", "player", "exiles", "the", "top", "card", "of", "their", "library",
        ]),
        "tie-break-round",
    )
    .is_none()
    {
        return Ok(None);
    }
    let tie_words = crate::lexer::parser_token_word_refs(strip_sentence_edges(tie.lowered()));
    let tie_words = tie_words
        .iter()
        .map(|word| if *word == "players'" { "players" } else { *word })
        .collect::<Vec<_>>();
    const TIE: &[&str] = &[
        "if", "two", "or", "more", "players", "cards", "are", "tied", "for", "greatest", "the",
        "tied", "players", "repeat", "this", "process", "until", "the", "tie", "is", "broken",
    ];
    if tie_words.as_slice() != TIE {
        return Ok(None);
    }
    let winner_tokens = strip_sentence_edges(winner.lowered());
    let Some(((), action)) = primitives::parse_prefix(
        winner_tokens,
        primitives::phrase(&[
            "the", "player", "who", "exiled", "the", "card", "with", "the", "greatest", "mana",
            "value",
        ]),
    ) else {
        return Ok(None);
    };
    if action.is_empty() {
        return Ok(None);
    }
    let mut actor_action = crate::lexer::synthetic_word_tokens(["that", "player"]);
    actor_action.extend_from_slice(action);
    let winner_effects = super::parse_effect_sentence_lexed(&actor_action)?;
    let contenders = crate::util::helper_tag_for_tokens(round.lowered(), "tie_break_players");
    let exiled = crate::util::helper_tag_for_tokens(round.lowered(), "tie_break_exiled");
    Ok(Some(vec![
        EffectAst::GreatestManaValueTieBreakExile {
            contenders_tag: contenders.clone(),
            exiled_tag: exiled,
        },
        EffectAst::ForEach(ForEachEffectAst::ForEachTaggedPlayer {
            tag: contenders,
            effects: winner_effects,
            require_evidence: false,
        }),
    ]))
}
