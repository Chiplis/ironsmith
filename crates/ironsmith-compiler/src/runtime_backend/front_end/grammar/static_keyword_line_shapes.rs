use winnow::combinator::alt;
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::{any, rest};

use super::super::lexer::{LexStream, OwnedLexToken, TokenKind};
use super::primitives;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TokenBoundary {
    pub(crate) token: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct WordBoundary {
    pub(crate) word: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TokenSpan {
    pub(crate) start: usize,
    pub(crate) end: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PregameBattlefieldShape {
    pub(crate) battlefield: TokenSpan,
    pub(crate) if_you_do: Option<TokenSpan>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ComposedAnthemHead {
    Temporary,
    Permanent { action: Option<TokenBoundary> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PlayerCounterGainHead {
    pub(crate) get: TokenBoundary,
    pub(crate) has_counter_resource: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AsEntersSubject<'a> {
    This(Option<&'a str>),
    It,
    SourceReference,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AsEntersSubjectShape<'a> {
    pub(crate) subject: AsEntersSubject<'a>,
    pub(crate) tail_word: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AnimationVerbShape {
    pub(crate) be: TokenBoundary,
    pub(crate) has: TokenBoundary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SubtypeGrantVerbShape {
    pub(crate) be: TokenBoundary,
    pub(crate) with: TokenBoundary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PayLifeEtbShape {
    pub(crate) pay: TokenBoundary,
    pub(crate) saw_enter: bool,
    pub(crate) saw_may: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AffinityForFilter<'a> {
    pub(crate) filter_tokens: &'a [OwnedLexToken],
    pub(crate) is_artifacts: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConditionalSpellKeyword {
    Flash,
    Cascade,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ConditionalSpellKeywordShape<'a> {
    pub(crate) keyword: ConditionalSpellKeyword,
    pub(crate) condition_tokens: &'a [OwnedLexToken],
}

pub(crate) fn parse_affinity_for_filter(tokens: &[OwnedLexToken]) -> Option<AffinityForFilter<'_>> {
    primitives::parse_all(
        tokens,
        parse_affinity_for_filter_lexed,
        "affinity-for-filter",
    )
    .ok()
}

pub(crate) fn parse_conditional_spell_keyword(
    tokens: &[OwnedLexToken],
) -> Option<ConditionalSpellKeywordShape<'_>> {
    primitives::parse_all(
        tokens,
        parse_conditional_spell_keyword_lexed,
        "conditional-spell-keyword",
    )
    .ok()
}

pub(crate) fn parse_attachment_restriction_span(tokens: &[OwnedLexToken]) -> Option<TokenSpan> {
    phrase_span(tokens, &["can", "be", "attached", "only", "to"])
}

pub(crate) fn parse_count_as_card_count_word(words: &[&str]) -> Option<WordBoundary> {
    first_word(words, &["count"])
}

pub(crate) fn parse_player_counter_gain_head(
    tokens: &[OwnedLexToken],
) -> Option<PlayerCounterGainHead> {
    let get = first_token_word(tokens, &["get", "gets"])?;
    let mut input = LexStream::new(tokens);
    let mut has_counter_resource = false;
    loop {
        let parsed: WResult<&OwnedLexToken> = any.parse_next(&mut input);
        let Ok(token) = parsed else {
            break;
        };
        if token
            .as_word()
            .is_some_and(|word| matches!(word, "energy" | "poison" | "ticket" | "e" | "tk"))
            || (token.kind == TokenKind::ManaGroup
                && token.mana_group_inner().is_some_and(|inner| {
                    inner.eq_ignore_ascii_case("e") || inner.eq_ignore_ascii_case("tk")
                }))
        {
            has_counter_resource = true;
        }
    }
    Some(PlayerCounterGainHead {
        get,
        has_counter_resource,
    })
}

pub(crate) fn parse_pregame_battlefield_shape(
    tokens: &[OwnedLexToken],
) -> Option<PregameBattlefieldShape> {
    Some(PregameBattlefieldShape {
        battlefield: phrase_span(tokens, &["on", "the", "battlefield"])?,
        if_you_do: phrase_span(tokens, &["if", "you", "do"]),
    })
}

pub(crate) fn parse_composed_anthem_head(tokens: &[OwnedLexToken]) -> ComposedAnthemHead {
    if phrase_span(tokens, &["until", "end", "of", "turn"]).is_some() {
        return ComposedAnthemHead::Temporary;
    }
    ComposedAnthemHead::Permanent {
        action: first_token_word(tokens, &["get", "gets", "have", "has"]),
    }
}

pub(crate) fn parse_as_enters_subject<'a>(
    words: &[&'a str],
    allowed_this_kinds: &[&str],
) -> Option<AsEntersSubjectShape<'a>> {
    if words.first().copied()? != "as" {
        return None;
    }
    let mut cursor = 1usize;
    let subject = match words.get(cursor).copied()? {
        "this" => {
            cursor += 1;
            let kind = words.get(cursor).copied().filter(|candidate| {
                allowed_this_kinds
                    .iter()
                    .any(|allowed| candidate == allowed)
            });
            if kind.is_some() {
                cursor += 1;
            }
            AsEntersSubject::This(kind)
        }
        "it" => {
            cursor += 1;
            AsEntersSubject::It
        }
        _ => {
            let enters = first_word(&words[cursor + 1..], &["enters"])?;
            let enters_word = cursor + 1 + enters.word;
            if crate::runtime_backend::util::source_reference_surface_for_words(
                &words[cursor..enters_word],
            )
            .is_none()
            {
                return None;
            }
            cursor = enters_word;
            AsEntersSubject::SourceReference
        }
    };
    if words.get(cursor).copied() != Some("enters") {
        return None;
    }
    cursor += 1;
    if words.get(cursor..cursor + 2) == Some(&["the", "battlefield"][..]) {
        cursor += 2;
    }
    Some(AsEntersSubjectShape {
        subject,
        tail_word: cursor,
    })
}

pub(crate) fn parse_choice_word(words: &[&str]) -> Option<WordBoundary> {
    first_word(words, &["choose"])
}

pub(crate) fn parse_trigger_duplication_triggers_word(words: &[&str]) -> Option<WordBoundary> {
    first_word(words, &["triggers"])
}

pub(crate) fn parse_trigger_duplication_causes_word(words: &[&str]) -> Option<WordBoundary> {
    first_word(words, &["causes"])
}

pub(crate) fn parse_copy_exception_type_removal_span(
    tokens: &[OwnedLexToken],
) -> Option<TokenSpan> {
    let (start, remove_len, _) = primitives::find_prefix(tokens, || copy_exception_type_removal)?;
    let remove_start = start.checked_add(4)?;
    Some(TokenSpan {
        start: remove_start,
        end: remove_start.checked_add(remove_len)?,
    })
}

fn copy_exception_type_removal(input: &mut LexStream<'_>) -> WResult<usize> {
    primitives::kw("except").parse_next(input)?;
    alt((primitives::kw("its"), primitives::kw("it's"))).parse_next(input)?;
    primitives::kw("an").parse_next(input)?;
    alt((
        (
            primitives::kw("artifact"),
            primitives::phrase(&["and", "it", "loses", "all", "other", "card", "types"]),
        )
            .value(7usize),
        (
            primitives::kw("enchantment"),
            primitives::phrase(&["and", "it", "loses", "all", "other", "card", "types"]),
        )
            .value(7usize),
        (
            primitives::kw("enchantment"),
            primitives::phrase(&["and", "loses", "all", "other", "card", "types"]),
        )
            .value(6usize),
    ))
    .parse_next(input)
}

pub(crate) fn parse_animation_verbs(tokens: &[OwnedLexToken]) -> Option<AnimationVerbShape> {
    let be = first_token_word(tokens, &["is", "are"])?;
    let tail = &tokens[be.token + 1..];
    let relative_has = first_token_word(tail, &["have", "has"])?;
    Some(AnimationVerbShape {
        be,
        has: TokenBoundary {
            token: be.token + 1 + relative_has.token,
        },
    })
}

pub(crate) fn parse_animation_creature_word(words: &[&str]) -> Option<WordBoundary> {
    first_word(words, &["creature", "creatures"])
}

pub(crate) fn parse_subtype_grant_verbs(tokens: &[OwnedLexToken]) -> Option<SubtypeGrantVerbShape> {
    let be = first_token_word(tokens, &["is", "are"])?;
    let relative_with = first_token_word(&tokens[be.token + 1..], &["with"])?;
    Some(SubtypeGrantVerbShape {
        be,
        with: TokenBoundary {
            token: be.token + 1 + relative_with.token,
        },
    })
}

pub(crate) fn parse_pay_life_etb_shape(tokens: &[OwnedLexToken]) -> Option<PayLifeEtbShape> {
    let pay = first_token_word(tokens, &["pay"])?;
    let prefix = &tokens[..pay.token];
    Some(PayLifeEtbShape {
        pay,
        saw_enter: first_token_word(prefix, &["enter", "enters"]).is_some(),
        saw_may: first_token_word(prefix, &["may"]).is_some(),
    })
}

fn parse_affinity_for_filter_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<AffinityForFilter<'a>> {
    primitives::phrase(&["affinity", "for"]).parse_next(input)?;
    let filter_tokens: &'a [OwnedLexToken] = rest.parse_next(input)?;
    if filter_tokens.is_empty() {
        return Err(primitives::backtrack_err(
            "affinity filter",
            "one or more filter tokens",
        ));
    }
    let mut filter_input = LexStream::new(filter_tokens);
    let is_artifacts = (primitives::kw("artifacts"), winnow::combinator::eof)
        .parse_next(&mut filter_input)
        .is_ok();
    Ok(AffinityForFilter {
        filter_tokens,
        is_artifacts,
    })
}

fn parse_conditional_spell_keyword_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<ConditionalSpellKeywordShape<'a>> {
    primitives::phrase(&["this", "spell", "has"]).parse_next(input)?;
    let keyword = alt((
        primitives::kw("flash").value(ConditionalSpellKeyword::Flash),
        primitives::kw("cascade").value(ConditionalSpellKeyword::Cascade),
    ))
    .parse_next(input)?;
    primitives::phrase(&["as", "long", "as"]).parse_next(input)?;
    let condition_tokens: &'a [OwnedLexToken] = rest.parse_next(input)?;
    if condition_tokens.is_empty() {
        return Err(primitives::backtrack_err(
            "spell keyword condition",
            "one or more condition tokens",
        ));
    }
    Ok(ConditionalSpellKeywordShape {
        keyword,
        condition_tokens,
    })
}

fn first_token_word(tokens: &[OwnedLexToken], expected: &[&str]) -> Option<TokenBoundary> {
    let mut input = LexStream::new(tokens);
    let initial_len = input.len();
    loop {
        let token_offset = initial_len.saturating_sub(input.len());
        let parsed: WResult<&OwnedLexToken> = any.parse_next(&mut input);
        let token = parsed.ok()?;
        let Some(candidate) = token.as_word() else {
            continue;
        };
        if expected.iter().any(|word| candidate == *word) {
            return Some(TokenBoundary {
                token: token_offset,
            });
        }
    }
}

fn first_word(words: &[&str], expected: &[&str]) -> Option<WordBoundary> {
    let mut input: primitives::WordSliceInput<'_> = words;
    let initial_len = input.len();
    loop {
        let word = initial_len.saturating_sub(input.len());
        let parsed: WResult<&str> = any.parse_next(&mut input);
        let candidate = parsed.ok()?;
        if expected
            .iter()
            .any(|expected_word| candidate == *expected_word)
        {
            return Some(WordBoundary { word });
        }
    }
}

fn phrase_span(tokens: &[OwnedLexToken], expected: &'static [&'static str]) -> Option<TokenSpan> {
    let mut input = LexStream::new(tokens);
    let initial_len = input.len();
    loop {
        let start = initial_len.saturating_sub(input.len());
        let mut candidate = input.clone();
        if primitives::phrase(expected)
            .parse_next(&mut candidate)
            .is_ok()
        {
            return Some(TokenSpan {
                start,
                end: initial_len.saturating_sub(candidate.len()),
            });
        }
        let parsed: WResult<&OwnedLexToken> = any.parse_next(&mut input);
        parsed.ok()?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::builders::TextSpan;

    fn tokens(words: &[&str]) -> Vec<OwnedLexToken> {
        words
            .iter()
            .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
            .collect()
    }

    #[test]
    fn parses_pregame_and_attachment_shapes() {
        let line = tokens(&[
            "if",
            "this",
            "is",
            "on",
            "the",
            "battlefield",
            "if",
            "you",
            "do",
        ]);
        let shape = parse_pregame_battlefield_shape(&line).unwrap();
        assert_eq!(shape.battlefield, TokenSpan { start: 3, end: 6 });
        assert_eq!(shape.if_you_do, Some(TokenSpan { start: 6, end: 9 }));
        let attach = tokens(&["this", "can", "be", "attached", "only", "to", "creatures"]);
        assert_eq!(parse_attachment_restriction_span(&attach).unwrap().end, 6);
    }

    #[test]
    fn parses_as_enters_and_animation_verbs() {
        let words = ["as", "this", "creature", "enters", "choose", "a", "color"];
        let shape = parse_as_enters_subject(&words, &["creature"]).unwrap();
        assert_eq!(shape.subject, AsEntersSubject::This(Some("creature")));
        assert_eq!(shape.tail_word, 4);
        let line = tokens(&["lands", "are", "creatures", "and", "have", "flying"]);
        assert_eq!(
            parse_animation_verbs(&line),
            Some(AnimationVerbShape {
                be: TokenBoundary { token: 1 },
                has: TokenBoundary { token: 4 },
            })
        );
    }

    #[test]
    fn parses_copy_exception_boundary() {
        let copy = tokens(&[
            "it", "enters", "as", "a", "copy", "except", "its", "an", "artifact", "and", "it",
            "loses", "all", "other", "card", "types",
        ]);
        assert_eq!(
            parse_copy_exception_type_removal_span(&copy),
            Some(TokenSpan { start: 9, end: 16 })
        );
    }

    #[test]
    fn parses_affinity_and_conditional_spell_keyword_shapes() {
        let affinity = tokens(&["affinity", "for", "creatures", "you", "control"]);
        let parsed = parse_affinity_for_filter(&affinity).unwrap();
        assert_eq!(parsed.filter_tokens.len(), 3);
        assert!(!parsed.is_artifacts);

        let affinity = tokens(&["affinity", "for", "artifacts"]);
        assert!(parse_affinity_for_filter(&affinity).unwrap().is_artifacts);

        let conditional = tokens(&[
            "this", "spell", "has", "cascade", "as", "long", "as", "you", "have", "seven", "cards",
        ]);
        let parsed = parse_conditional_spell_keyword(&conditional).unwrap();
        assert_eq!(parsed.keyword, ConditionalSpellKeyword::Cascade);
        assert_eq!(parsed.condition_tokens.len(), 4);
    }
}
