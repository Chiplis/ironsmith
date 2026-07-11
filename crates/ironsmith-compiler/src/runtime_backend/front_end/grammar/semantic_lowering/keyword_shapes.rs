use winnow::combinator::{alt, eof, peek, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use crate::runtime_backend::model::semantic::GiftTimingAst;

use super::super::super::lexer::{
    LexStream, OwnedLexToken, TokenKind, TokenWordView, lex_line, parser_token_word_refs,
    render_token_slice, trim_lexed_commas,
};
use super::super::primitives;
use super::statement_shapes::parse_comma_split_tokens;
use super::{
    any_phrase_is_present, phrase_is_exact, phrase_is_prefix, phrase_is_present, phrase_is_suffix,
    phrase_location,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExertAttackHead {
    pub(crate) only_if_not_exerted_this_turn: bool,
    pub(crate) source_ref: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ExertReflexiveFollowup<'a> {
    pub(crate) effect_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StandardGiftVariant {
    Card,
    Treasure,
    Food,
    TappedFish,
    ExtraTurn,
    Octopus,
}

#[derive(Debug, Clone)]
pub(crate) struct StandardGiftSpec {
    pub(crate) variant: StandardGiftVariant,
    pub(crate) timing: GiftTimingAst,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PartnerVariantLabel {
    pub(crate) display: String,
}

fn exert_prefix(words: &[&str]) -> Option<(bool, usize)> {
    let conditional = &[
        "if", "this", "creature", "hasnt", "been", "exerted", "this", "turn", "you", "may", "exert",
    ];
    let apostrophe_conditional = &[
        "if", "this", "creature", "hasn't", "been", "exerted", "this", "turn", "you", "may",
        "exert",
    ];
    if phrase_is_prefix(words, conditional) {
        Some((true, conditional.len()))
    } else if phrase_is_prefix(words, apostrophe_conditional) {
        Some((true, apostrophe_conditional.len()))
    } else if phrase_is_prefix(words, &["you", "may", "exert"]) {
        Some((false, 3))
    } else {
        None
    }
}

pub(crate) fn parse_exert_attack_head_tokens(
    tokens: &[OwnedLexToken],
) -> Result<ExertAttackHead, &'static str> {
    let words = parser_token_word_refs(tokens);
    let Some((only_if_not_exerted_this_turn, source_start)) = exert_prefix(&words) else {
        return Err("could not parse exert attack line");
    };
    let Some(as_word) =
        phrase_location(&words[source_start..], &["as"]).map(|offset| offset + source_start)
    else {
        return Err("could not parse exert attack head");
    };
    if as_word == source_start {
        return Err("missing exert source");
    }
    let attack_words = words
        .get(as_word + 1..)
        .ok_or("could not isolate exert attack clause")?;
    if !(phrase_is_suffix(attack_words, &["attack"])
        || phrase_is_suffix(attack_words, &["attacks"]))
    {
        return Err("expected attack clause");
    }
    let view = TokenWordView::new(tokens);
    let source_range = view
        .token_span_for_words(source_start, as_word)
        .ok_or("could not isolate exert source")?;
    Ok(ExertAttackHead {
        only_if_not_exerted_this_turn,
        source_ref: render_token_slice(&tokens[source_range]).trim().to_string(),
    })
}

pub(crate) fn parse_exert_reflexive_followup_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ExertReflexiveFollowup<'_>> {
    let words = parser_token_word_refs(tokens);
    if !phrase_is_prefix(&words, &["when", "you", "do"]) {
        return None;
    }
    let view = TokenWordView::new(tokens);
    let range = view.token_span_for_words(3, words.len())?;
    let effect_tokens = trim_lexed_commas(&tokens[range]);
    (!effect_tokens.is_empty()).then_some(ExertReflexiveFollowup { effect_tokens })
}

pub(crate) fn parse_when_followup_intro_tokens(tokens: &[OwnedLexToken]) -> bool {
    phrase_is_prefix(&parser_token_word_refs(tokens), &["when"])
}

pub(crate) fn normalize_exert_followup_source_tokens(
    source_ref: &str,
    followup_tokens: &[OwnedLexToken],
) -> Vec<OwnedLexToken> {
    let words = parser_token_word_refs(followup_tokens);
    let replacement_words = if any_phrase_is_present(
        words.get(..1).unwrap_or_default(),
        &[&["he"], &["she"], &["they"]],
    ) {
        Some(1)
    } else if let Ok(source_tokens) = lex_line(source_ref, 0) {
        let source_words = parser_token_word_refs(&source_tokens);
        if !source_words.is_empty()
            && !phrase_is_exact(&source_words, &["this", "creature"])
            && phrase_is_prefix(&words, &source_words)
        {
            Some(source_words.len())
        } else {
            None
        }
    } else {
        None
    };

    let Some(replacement_words) = replacement_words else {
        return followup_tokens.to_vec();
    };
    let view = TokenWordView::new(followup_tokens);
    let remainder = view
        .token_span_for_words(replacement_words, words.len())
        .and_then(|range| followup_tokens.get(range.start..))
        .unwrap_or_default();
    let mut normalized =
        lex_line("this creature", 0).expect("semantic exert subject should always lex");
    normalized.extend_from_slice(remainder);
    normalized
}

fn visible_keyword_tokens<'a>(input: &mut LexStream<'a>) -> WResult<&'a [OwnedLexToken]> {
    repeat_till(
        0..,
        any.void(),
        peek(alt((
            primitives::token_kind(TokenKind::LParen).void(),
            primitives::token_kind(TokenKind::Period).void(),
            eof.void(),
        ))),
    )
    .map(|((), ())| ())
    .take()
    .parse_next(input)
}

fn keyword_visible_prefix(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    let mut input = LexStream::new(tokens);
    visible_keyword_tokens(&mut input).unwrap_or(tokens)
}

pub(crate) fn parse_standard_gift_spec_tokens(
    tokens: &[OwnedLexToken],
) -> Option<StandardGiftSpec> {
    let visible = keyword_visible_prefix(tokens);
    let words = parser_token_word_refs(visible);
    let variant = if phrase_is_exact(&words, &["gift", "a", "card"]) {
        StandardGiftVariant::Card
    } else if phrase_is_exact(&words, &["gift", "a", "treasure"]) {
        StandardGiftVariant::Treasure
    } else if phrase_is_exact(&words, &["gift", "a", "food"]) {
        StandardGiftVariant::Food
    } else if phrase_is_exact(&words, &["gift", "a", "tapped", "fish"]) {
        StandardGiftVariant::TappedFish
    } else if phrase_is_exact(&words, &["gift", "an", "extra", "turn"]) {
        StandardGiftVariant::ExtraTurn
    } else if phrase_is_exact(&words, &["gift", "an", "octopus"]) {
        StandardGiftVariant::Octopus
    } else {
        return None;
    };
    let all_words = parser_token_word_refs(tokens);
    let timing = if phrase_is_present(&all_words, &["when", "it", "enters"]) {
        GiftTimingAst::PermanentEtb
    } else if variant == StandardGiftVariant::Octopus {
        GiftTimingAst::PermanentEtb
    } else {
        GiftTimingAst::SpellResolution
    };
    Some(StandardGiftSpec { variant, timing })
}

pub(crate) fn parse_partner_variant_label_tokens(
    tokens: &[OwnedLexToken],
) -> Option<PartnerVariantLabel> {
    let visible_tokens = keyword_visible_prefix(tokens);
    let words = parser_token_word_refs(visible_tokens);
    if phrase_is_exact(&words, &["partner"]) || phrase_is_prefix(&words, &["partner", "with"]) {
        return None;
    }
    if phrase_is_prefix(&words, &["character", "select"]) {
        return Some(PartnerVariantLabel {
            display: render_token_slice(visible_tokens).trim().to_string(),
        });
    }
    if phrase_is_prefix(&words, &["partner"]) && !phrase_is_prefix(&words, &["partner", "with"]) {
        return Some(PartnerVariantLabel {
            display: super::super::keyword_special_lines::parse_partner_visible_label_tokens(
                tokens,
            )?,
        });
    }
    None
}

fn parse_generic_mana_group(input: &mut LexStream<'_>) -> WResult<u32> {
    any.verify_map(|token: &OwnedLexToken| {
        token
            .mana_group_inner()
            .and_then(|inner| inner.parse::<u32>().ok())
    })
    .parse_next(input)
}

fn additional_cost_tail(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    if let Some(split) = parse_comma_split_tokens(tokens) {
        return Some(split.after);
    }
    let words = parser_token_word_refs(tokens);
    let after_spell = phrase_location(&words, &["spell"])? + 1;
    let view = TokenWordView::new(tokens);
    let range = view.token_span_for_words(after_spell, words.len())?;
    let tail = trim_lexed_commas(&tokens[range]);
    (!tail.is_empty()).then_some(tail)
}

pub(crate) fn parse_optional_waterbend_generic_tokens(tokens: &[OwnedLexToken]) -> Option<u32> {
    let tail = additional_cost_tail(tokens)?;
    let words = parser_token_word_refs(tail);
    if !phrase_is_prefix(&words, &["you", "may", "waterbend"]) {
        return None;
    }
    primitives::find_prefix(tail, || parse_generic_mana_group).map(|(_, generic, _)| generic)
}

#[cfg(test)]
mod tests {
    use super::super::super::super::lexer::lex_line;
    use super::*;

    #[test]
    fn parses_exert_head_and_normalizes_followup() {
        let head = lex_line("You may exert Arni as he attacks.", 0).unwrap();
        let parsed = parse_exert_attack_head_tokens(&head).unwrap();
        assert_eq!(parsed.source_ref, "Arni");
        let followup = lex_line("Arni deals 2 damage.", 0).unwrap();
        assert_eq!(
            parser_token_word_refs(&normalize_exert_followup_source_tokens(
                &parsed.source_ref,
                &followup
            )),
            vec!["this", "creature", "deals", "2", "damage"]
        );
    }

    #[test]
    fn parses_gift_partner_and_waterbend() {
        let gift = lex_line("Gift an Octopus (When it enters, ...)", 0).unwrap();
        let gift = parse_standard_gift_spec_tokens(&gift).unwrap();
        assert_eq!(gift.variant, StandardGiftVariant::Octopus);
        assert!(matches!(gift.timing, GiftTimingAst::PermanentEtb));
        let partner = lex_line("Partner — Friends forever (Reminder.)", 0).unwrap();
        assert!(parse_partner_variant_label_tokens(&partner).is_some());
        let waterbend = lex_line(
            "As an additional cost to cast this spell, you may waterbend {4}.",
            0,
        )
        .unwrap();
        assert_eq!(parse_optional_waterbend_generic_tokens(&waterbend), Some(4));
    }
}
