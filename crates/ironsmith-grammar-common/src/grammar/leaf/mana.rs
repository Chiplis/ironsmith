use winnow::ascii::digit1;
use winnow::combinator::{alt, repeat, separated, terminated};
use winnow::error::{ModalResult as WResult, StrContext, StrContextValue};
use winnow::prelude::*;
use winnow::token::one_of;

use crate::cards::builders::CardTextError;
use crate::mana::{ManaCost, ManaSymbol};

use super::super::super::lexer::{LexStream, OwnedLexToken, TokenKind};
use super::super::primitives;
use super::common::{finish_text_parse, spaced, word_boundary};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LeafManaPipToken {
    ManaGroup(Vec<ManaSymbol>),
    LegacyBare(ManaSymbol),
}

impl LeafManaPipToken {
    pub fn into_pip(self) -> Vec<ManaSymbol> {
        match self {
            Self::ManaGroup(group) => group,
            Self::LegacyBare(symbol) => vec![symbol],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeafManaCostPrefix {
    pub cost: ManaCost,
    pub consumed: usize,
}

pub fn parse_leaf_mana_symbol_inner(input: &mut &str) -> WResult<ManaSymbol> {
    alt((
        digit1.try_map(|digits: &str| digits.parse::<u8>().map(ManaSymbol::Generic)),
        one_of([
            'W', 'w', 'U', 'u', 'B', 'b', 'R', 'r', 'G', 'g', 'C', 'c', 'S', 's', 'X', 'x', 'P',
            'p',
        ])
        .map(|ch: char| match ch.to_ascii_uppercase() {
            'W' => ManaSymbol::White,
            'U' => ManaSymbol::Blue,
            'B' => ManaSymbol::Black,
            'R' => ManaSymbol::Red,
            'G' => ManaSymbol::Green,
            'C' => ManaSymbol::Colorless,
            'S' => ManaSymbol::Snow,
            'X' => ManaSymbol::X,
            'P' => ManaSymbol::Life(2),
            _ => unreachable!("one_of constrains supported mana-symbol letters"),
        }),
    ))
    .context(StrContext::Label("mana symbol"))
    .context(StrContext::Expected(StrContextValue::Description(
        "mana symbol",
    )))
    .parse_next(input)
}

pub fn parse_leaf_mana_symbol_group_inner(input: &mut &str) -> WResult<Vec<ManaSymbol>> {
    separated(1.., parse_leaf_mana_symbol_inner, spaced('/'))
        .context(StrContext::Label("mana symbol group"))
        .context(StrContext::Expected(StrContextValue::Description(
            "slash-delimited mana symbols",
        )))
        .parse_next(input)
}

pub fn parse_leaf_spelled_mana_word(input: &mut &str) -> WResult<ManaSymbol> {
    terminated(
        alt((
            "white".value(ManaSymbol::White),
            "blue".value(ManaSymbol::Blue),
            "black".value(ManaSymbol::Black),
            "red".value(ManaSymbol::Red),
            "green".value(ManaSymbol::Green),
            "colorless".value(ManaSymbol::Colorless),
        )),
        word_boundary,
    )
    .context(StrContext::Label("spelled mana word"))
    .context(StrContext::Expected(StrContextValue::Description(
        "spelled color or colorless mana word",
    )))
    .parse_next(input)
}

pub fn parse_leaf_mana_group_token<'a>(input: &mut LexStream<'a>) -> WResult<Vec<ManaSymbol>> {
    let token = primitives::token_kind(TokenKind::ManaGroup).parse_next(input)?;
    parse_leaf_mana_symbol_group_complete(token.slice.as_str())
        .map_err(|_| primitives::backtrack_err("mana group", "braced mana symbols"))
}

pub fn parse_leaf_legacy_bare_mana_token<'a>(input: &mut LexStream<'a>) -> WResult<ManaSymbol> {
    let checkpoint = input.checkpoint();
    let token = alt((
        primitives::token_kind(TokenKind::Word),
        primitives::token_kind(TokenKind::Number),
    ))
    .parse_next(input)?;
    match parse_leaf_bare_mana_symbol_complete(token.parser_text()) {
        Ok(symbol) => Ok(symbol),
        Err(_) => {
            input.reset(&checkpoint);
            Err(primitives::backtrack_err(
                "legacy bare mana",
                "bare mana-symbol word or number",
            ))
        }
    }
}

pub fn parse_leaf_surface_mana_pip_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<LeafManaPipToken> {
    alt((
        parse_leaf_mana_group_token.map(LeafManaPipToken::ManaGroup),
        parse_leaf_legacy_bare_mana_token.map(LeafManaPipToken::LegacyBare),
    ))
    .context(StrContext::Label("surface mana pip"))
    .context(StrContext::Expected(StrContextValue::Description(
        "braced mana group or legacy bare mana symbol",
    )))
    .parse_next(input)
}

pub fn parse_leaf_mana_cost_prefix_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<LeafManaCostPrefix> {
    repeat(1.., parse_leaf_surface_mana_pip_lexed)
        .map(|pips: Vec<LeafManaPipToken>| LeafManaCostPrefix {
            consumed: pips.len(),
            cost: ManaCost::from_pips(pips.into_iter().map(LeafManaPipToken::into_pip).collect()),
        })
        .context(StrContext::Label("mana cost prefix"))
        .context(StrContext::Expected(StrContextValue::Description(
            "one or more surface mana pips",
        )))
        .parse_next(input)
}

pub fn parse_leaf_fixed_mana_cost_prefix_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<LeafManaCostPrefix> {
    repeat(
        1..,
        parse_leaf_surface_mana_pip_lexed.verify(|pip| {
            pip_symbols(pip).all(|symbol| {
                !matches!(
                    symbol,
                    ManaSymbol::X | ManaSymbol::Snow | ManaSymbol::Life(_)
                )
            })
        }),
    )
    .map(|pips: Vec<LeafManaPipToken>| LeafManaCostPrefix {
        consumed: pips.len(),
        cost: ManaCost::from_pips(pips.into_iter().map(LeafManaPipToken::into_pip).collect()),
    })
    .context(StrContext::Label("fixed mana cost prefix"))
    .context(StrContext::Expected(StrContextValue::Description(
        "one or more fixed mana pips",
    )))
    .parse_next(input)
}

pub fn parse_leaf_mana_cost_lexed<'a>(input: &mut LexStream<'a>) -> WResult<ManaCost> {
    repeat(1.., parse_leaf_mana_group_token)
        .map(ManaCost::from_pips)
        .context(StrContext::Label("mana cost"))
        .context(StrContext::Expected(StrContextValue::Description(
            "mana group",
        )))
        .parse_next(input)
}

pub fn parse_leaf_fixed_mana_output_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<Vec<ManaSymbol>> {
    repeat(
        1..,
        alt((
            parse_leaf_mana_group_token
                .verify(|group: &Vec<ManaSymbol>| group.len() == 1)
                .map(|mut group| Some(group.remove(0))),
            primitives::comma().value(None),
            primitives::period().value(None),
        )),
    )
    .verify(|parts: &Vec<Option<ManaSymbol>>| parts.iter().any(Option::is_some))
    .map(|parts: Vec<Option<ManaSymbol>>| parts.into_iter().flatten().collect())
    .context(StrContext::Label("fixed mana output"))
    .context(StrContext::Expected(StrContextValue::Description(
        "one or more single-symbol mana groups",
    )))
    .parse_next(input)
}

pub fn parse_leaf_mana_symbol_complete(raw: &str) -> Result<ManaSymbol, CardTextError> {
    let unbraced = trim_single_mana_brace_pair(raw.trim());
    finish_text_parse(unbraced, parse_leaf_mana_symbol_spaced, "leaf-mana-symbol")
}

pub fn parse_leaf_bare_mana_symbol_complete(raw: &str) -> Result<ManaSymbol, CardTextError> {
    finish_text_parse(raw, parse_leaf_mana_symbol_spaced, "leaf-bare-mana-symbol")
}

pub fn parse_leaf_mana_symbol_group_complete(raw: &str) -> Result<Vec<ManaSymbol>, CardTextError> {
    let trimmed = raw.trim().trim_matches('{').trim_matches('}');
    finish_text_parse(
        trimmed,
        parse_leaf_mana_symbol_group_spaced,
        "leaf-mana-group",
    )
}

pub fn parse_leaf_spelled_mana_word_complete(raw: &str) -> Result<ManaSymbol, CardTextError> {
    finish_text_parse(raw, parse_leaf_spelled_mana_word, "leaf-spelled-mana-word")
}

pub fn parse_leaf_pawprint_label_count_complete(raw: &str) -> Result<u32, CardTextError> {
    finish_text_parse(raw, parse_leaf_pawprint_label_count, "leaf-pawprint-label")
}

pub fn parse_leaf_pawprint_label_count_token(token: &OwnedLexToken) -> Option<u32> {
    match token.kind {
        TokenKind::ManaGroup => parse_leaf_pawprint_label_count_complete(token.parser_text()).ok(),
        TokenKind::Word if token.parser_text() == "p" => Some(1),
        _ => None,
    }
}

fn parse_leaf_pawprint_label_count(input: &mut &str) -> WResult<u32> {
    let pawprints: Vec<&str> = repeat(1.., "{p}").parse_next(input)?;
    u32::try_from(pawprints.len())
        .map_err(|_| winnow::error::ErrMode::Backtrack(winnow::error::ContextError::new()))
}

pub fn parse_leaf_surface_mana_pip_token(token: &OwnedLexToken) -> Option<LeafManaPipToken> {
    primitives::parse_all(
        std::slice::from_ref(token),
        parse_leaf_surface_mana_pip_lexed,
        "leaf-surface-mana-pip",
    )
    .ok()
}

pub fn parse_leaf_mana_cost_prefix_tokens(tokens: &[OwnedLexToken]) -> Option<LeafManaCostPrefix> {
    let mut input = LexStream::new(tokens);
    parse_leaf_mana_cost_prefix_lexed
        .parse_next(&mut input)
        .ok()
}

pub fn parse_leaf_fixed_mana_cost_prefix_tokens(
    tokens: &[OwnedLexToken],
) -> Option<LeafManaCostPrefix> {
    let mut input = LexStream::new(tokens);
    parse_leaf_fixed_mana_cost_prefix_lexed
        .parse_next(&mut input)
        .ok()
}

#[cfg(test)]
pub fn parse_leaf_legacy_mana_cost_prefix_words(words: &[&str]) -> Option<LeafManaCostPrefix> {
    let mut input: primitives::WordSliceInput<'_> = words;
    let cost = parse_leaf_legacy_mana_cost_prefix_word_slice
        .parse_next(&mut input)
        .ok()?;
    Some(LeafManaCostPrefix {
        cost,
        consumed: words.len().checked_sub(input.len())?,
    })
}

pub fn parse_leaf_mana_symbol_group_tokens(
    tokens: &[OwnedLexToken],
) -> Result<Vec<ManaSymbol>, CardTextError> {
    primitives::parse_all(tokens, parse_leaf_mana_group_token, "leaf-mana-group")
}

pub fn parse_leaf_mana_cost_tokens(tokens: &[OwnedLexToken]) -> Result<ManaCost, CardTextError> {
    primitives::parse_all(tokens, parse_leaf_mana_cost_lexed, "leaf-mana-cost")
}

pub fn parse_leaf_fixed_mana_output_tokens(tokens: &[OwnedLexToken]) -> Option<Vec<ManaSymbol>> {
    primitives::parse_all(
        tokens,
        parse_leaf_fixed_mana_output_lexed,
        "leaf-fixed-mana-output",
    )
    .ok()
}

fn parse_leaf_mana_symbol_spaced(input: &mut &str) -> WResult<ManaSymbol> {
    spaced(parse_leaf_mana_symbol_inner).parse_next(input)
}

fn parse_leaf_mana_symbol_group_spaced(input: &mut &str) -> WResult<Vec<ManaSymbol>> {
    spaced(parse_leaf_mana_symbol_group_inner).parse_next(input)
}

#[cfg(test)]
fn parse_leaf_legacy_bare_mana_word_slice(
    input: &mut primitives::WordSliceInput<'_>,
) -> WResult<ManaSymbol> {
    let checkpoint = *input;
    let Some((word, rest)) = checkpoint.split_first() else {
        return Err(primitives::backtrack_err(
            "legacy bare mana",
            "bare mana-symbol word or number",
        ));
    };
    let symbol = parse_leaf_bare_mana_symbol_complete(word).map_err(|_| {
        primitives::backtrack_err("legacy bare mana", "bare mana-symbol word or number")
    })?;
    *input = rest;
    Ok(symbol)
}

#[cfg(test)]
fn parse_leaf_legacy_mana_cost_prefix_word_slice(
    input: &mut primitives::WordSliceInput<'_>,
) -> WResult<ManaCost> {
    repeat(1.., parse_leaf_legacy_bare_mana_word_slice)
        .map(|symbols: Vec<ManaSymbol>| ManaCost::from_symbols(symbols))
        .context(StrContext::Label("legacy bare mana cost prefix"))
        .context(StrContext::Expected(StrContextValue::Description(
            "one or more bare mana symbols",
        )))
        .parse_next(input)
}

fn pip_symbols(pip: &LeafManaPipToken) -> impl Iterator<Item = &ManaSymbol> {
    match pip {
        LeafManaPipToken::ManaGroup(group) => group.as_slice(),
        LeafManaPipToken::LegacyBare(symbol) => std::slice::from_ref(symbol),
    }
    .iter()
}

fn trim_single_mana_brace_pair(raw: &str) -> &str {
    let bytes = raw.as_bytes();
    if bytes.len() >= 2 && bytes[0] == b'{' && bytes[bytes.len() - 1] == b'}' {
        &raw[1..bytes.len() - 1]
    } else {
        raw
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex_line;

    #[test]
    fn strict_group_parser_rejects_legacy_bare_tokens() {
        let group = lex_line("{W/U}", 0).unwrap();
        assert_eq!(
            primitives::parse_all(&group, parse_leaf_mana_group_token, "test").unwrap(),
            vec![ManaSymbol::White, ManaSymbol::Blue]
        );

        for raw in ["w", "2"] {
            let tokens = lex_line(raw, 0).unwrap();
            assert!(primitives::parse_all(&tokens, parse_leaf_mana_group_token, "test").is_err());
        }
    }

    #[test]
    fn legacy_bare_parser_is_explicit_and_does_not_accept_groups() {
        for (raw, expected) in [("w", ManaSymbol::White), ("2", ManaSymbol::Generic(2))] {
            let tokens = lex_line(raw, 0).unwrap();
            assert_eq!(
                primitives::parse_all(&tokens, parse_leaf_legacy_bare_mana_token, "test").unwrap(),
                expected
            );
        }

        let group = lex_line("{W}", 0).unwrap();
        assert!(primitives::parse_all(&group, parse_leaf_legacy_bare_mana_token, "test").is_err());
        assert!(parse_leaf_bare_mana_symbol_complete("{W}").is_err());
    }

    #[test]
    fn surface_pip_preserves_hybrid_group_structure() {
        let tokens = lex_line("{2/W}", 0).unwrap();
        assert_eq!(
            parse_leaf_surface_mana_pip_token(&tokens[0]),
            Some(LeafManaPipToken::ManaGroup(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::White,
            ]))
        );
    }

    #[test]
    fn token_cost_prefix_preserves_groups_and_consumption() {
        let tokens = lex_line("{2}{W/U} target creature", 0).unwrap();
        let prefix = parse_leaf_mana_cost_prefix_tokens(&tokens).unwrap();
        assert_eq!(prefix.cost.to_oracle(), "{2}{W/U}");
        assert_eq!(prefix.cost.pips()[1], [ManaSymbol::White, ManaSymbol::Blue]);
        assert_eq!(prefix.consumed, 2);

        let legacy = lex_line("2 w target creature", 0).unwrap();
        let prefix = parse_leaf_mana_cost_prefix_tokens(&legacy).unwrap();
        assert_eq!(prefix.cost.to_oracle(), "{2}{W}");
        assert_eq!(prefix.consumed, 2);
    }

    #[test]
    fn fixed_cost_prefix_stops_before_dynamic_or_life_pips() {
        let tokens = lex_line("{2}{W/U}{X}{R}", 0).unwrap();
        let prefix = parse_leaf_fixed_mana_cost_prefix_tokens(&tokens).unwrap();
        assert_eq!(prefix.cost.to_oracle(), "{2}{W/U}");
        assert_eq!(prefix.consumed, 2);

        for raw in ["{X}", "{S}", "p"] {
            let tokens = lex_line(raw, 0).unwrap();
            assert!(parse_leaf_fixed_mana_cost_prefix_tokens(&tokens).is_none());
        }
    }

    #[test]
    fn fixed_mana_output_requires_single_symbol_groups() {
        let tokens = lex_line("{W}, {U}.", 0).unwrap();
        assert_eq!(
            parse_leaf_fixed_mana_output_tokens(&tokens),
            Some(vec![ManaSymbol::White, ManaSymbol::Blue])
        );

        let hybrid = lex_line("{W/U}", 0).unwrap();
        assert!(parse_leaf_fixed_mana_output_tokens(&hybrid).is_none());
        let bare = lex_line("w", 0).unwrap();
        assert!(parse_leaf_fixed_mana_output_tokens(&bare).is_none());
    }

    #[test]
    fn legacy_word_cost_prefix_is_typed_and_stops_at_non_mana() {
        let prefix = parse_leaf_legacy_mana_cost_prefix_words(&["x", "r", "ability"]).unwrap();
        assert_eq!(prefix.cost.to_oracle(), "{X}{R}");
        assert_eq!(prefix.consumed, 2);
    }

    #[test]
    fn spelled_mana_words_map_colors_and_colorless() {
        assert_eq!(
            parse_leaf_spelled_mana_word_complete("White").unwrap(),
            ManaSymbol::White
        );
        assert_eq!(
            parse_leaf_spelled_mana_word_complete("colorless").unwrap(),
            ManaSymbol::Colorless
        );
        assert!(parse_leaf_spelled_mana_word_complete("whitestone").is_err());
    }

    #[test]
    fn specialized_non_mana_symbols_remain_outside_this_leaf() {
        for raw in ["t", "q", "e", "tk", "pawprint"] {
            let tokens = lex_line(raw, 0).unwrap();
            assert!(parse_leaf_surface_mana_pip_token(&tokens[0]).is_none());
        }
    }

    #[test]
    fn pawprint_modal_label_count_is_typed() {
        assert_eq!(
            parse_leaf_pawprint_label_count_complete("{P}{p}").unwrap(),
            2
        );
        let tokens = lex_line("{P}{P}", 0).unwrap();
        assert_eq!(tokens.len(), 2);
        assert_eq!(parse_leaf_pawprint_label_count_token(&tokens[0]), Some(1));
        assert_eq!(parse_leaf_pawprint_label_count_token(&tokens[1]), Some(1));
        let plain = lex_line("p", 0).unwrap();
        assert_eq!(parse_leaf_pawprint_label_count_token(&plain[0]), Some(1));
        assert!(parse_leaf_pawprint_label_count_complete("{p}{w}").is_err());
    }
}
