use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::{literal, take_till};

use super::super::lexer::OwnedLexToken;
use super::primitives;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct WordTokenBoundary {
    pub(crate) token: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PtComponents<'a> {
    pub(crate) power: &'a str,
    pub(crate) toughness: &'a str,
}

pub(crate) fn parse_word_token_boundary(
    tokens: &[OwnedLexToken],
    word_offset: usize,
) -> Option<WordTokenBoundary> {
    let view = primitives::TokenWordView::new(tokens);
    Some(WordTokenBoundary {
        token: view.token_start_indices().get(word_offset).copied()?,
    })
}

pub(crate) fn parse_word_token_offset(
    tokens: &[OwnedLexToken],
    word_offset: usize,
) -> Option<usize> {
    parse_word_token_boundary(tokens, word_offset).map(|boundary| boundary.token)
}

pub(crate) fn parse_rule_id_head(rule_id: &str) -> Option<&str> {
    let mut input = rule_id;
    let prefix: WResult<&str> = literal("parse_").parse_next(&mut input);
    prefix.ok()?;
    let head: WResult<&str> = take_till(1.., '_').parse_next(&mut input);
    head.ok()
}

pub(crate) fn parse_pt_components(raw: &str) -> Option<PtComponents<'_>> {
    let mut input = raw;
    let parsed_power: WResult<&str> = take_till(1.., '/').parse_next(&mut input);
    let power = parsed_power.ok()?;
    let separator: WResult<&str> = literal('/').parse_next(&mut input);
    separator.ok()?;
    if input.is_empty() {
        return None;
    }
    Some(PtComponents {
        power,
        toughness: input,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::builders::TextSpan;

    fn words(values: &[&str]) -> Vec<OwnedLexToken> {
        values
            .iter()
            .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
            .collect()
    }

    #[test]
    fn maps_word_offsets() {
        let tokens = words(&["as", "this", "enters"]);
        assert_eq!(
            parse_word_token_boundary(&tokens, 2),
            Some(WordTokenBoundary { token: 2 })
        );
    }

    #[test]
    fn parses_rule_head_and_pt_components() {
        assert_eq!(parse_rule_id_head("parse_ward_line"), Some("ward"));
        assert_eq!(
            parse_pt_components("+2/-1"),
            Some(PtComponents {
                power: "+2",
                toughness: "-1",
            })
        );
    }
}
