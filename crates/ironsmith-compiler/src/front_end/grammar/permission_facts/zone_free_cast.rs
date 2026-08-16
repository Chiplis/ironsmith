use winnow::combinator::{alt, eof, peek, repeat, repeat_till, separated};
use winnow::error::{ContextError, ErrMode, ModalResult as WResult};
use winnow::prelude::*;
use winnow::token::any;

use crate::zone::Zone;

use super::super::super::lexer::{LexStream, OwnedLexToken, TokenKind, trim_lexed_commas};
use super::super::primitives;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ZonePermissionLifetimeFact {
    Static,
    ThisTurn,
    UntilEndOfTurn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ManaValuePlacementFact {
    BeforeZone,
    AfterZone,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SpellNumberFact {
    Singular,
    Plural,
    Mixed,
    Unspecified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PlayFromZoneFact<'a> {
    pub(crate) filter_tokens: &'a [OwnedLexToken],
    pub(crate) zone: Zone,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LandsFromTopLibraryFact;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LandsAndCastFromTopLibraryFact<'a> {
    pub(crate) spell_filter_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FlashGrantFact<'a> {
    pub(crate) filter_tokens: &'a [OwnedLexToken],
    pub(crate) lifetime: ZonePermissionLifetimeFact,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ManaValueComparisonTokens<'a> {
    pub(crate) tokens: &'a [OwnedLexToken],
    pub(crate) placement: ManaValuePlacementFact,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FreeCastFromZoneFact<'a> {
    pub(crate) filter_tokens: &'a [OwnedLexToken],
    pub(crate) zone: Zone,
    pub(crate) mana_value: Option<ManaValueComparisonTokens<'a>>,
    pub(crate) subject_number: SpellNumberFact,
}

pub(crate) fn parse_play_from_zone_tokens(
    tokens: &[OwnedLexToken],
) -> Option<PlayFromZoneFact<'_>> {
    primitives::parse_all(tokens, parse_play_from_zone, "play-from-zone permission").ok()
}

pub(crate) fn parse_lands_from_top_library_tokens(
    tokens: &[OwnedLexToken],
) -> Option<LandsFromTopLibraryFact> {
    primitives::parse_all(
        tokens,
        parse_lands_from_top_library,
        "lands from top library permission",
    )
    .ok()
}

pub(crate) fn parse_lands_and_cast_from_top_library_tokens(
    tokens: &[OwnedLexToken],
) -> Option<LandsAndCastFromTopLibraryFact<'_>> {
    primitives::parse_all(
        tokens,
        parse_lands_and_cast_from_top_library,
        "lands and cast from top library permission",
    )
    .ok()
}

pub(crate) fn parse_flash_grant_tokens(tokens: &[OwnedLexToken]) -> Option<FlashGrantFact<'_>> {
    primitives::parse_all(tokens, parse_flash_grant, "flash grant permission").ok()
}

pub(crate) fn parse_free_cast_from_zone_tokens(
    tokens: &[OwnedLexToken],
) -> Option<FreeCastFromZoneFact<'_>> {
    primitives::parse_all(tokens, parse_free_cast_from_zone, "free cast from zone").ok()
}

pub(crate) fn parse_mana_value_one_of_tokens(tokens: &[OwnedLexToken]) -> Option<Vec<i32>> {
    primitives::parse_all(tokens, parse_mana_value_one_of, "mana-value disjunction").ok()
}

fn parse_mana_value_one_of(input: &mut LexStream<'_>) -> WResult<Vec<i32>> {
    let values: Vec<u32> = separated(
        2..,
        super::super::leaf::parse_leaf_number_prefix_lexed,
        primitives::comma_or_separator,
    )
    .parse_next(input)?;
    semantic_finish(input)?;
    values
        .into_iter()
        .map(|value| {
            i32::try_from(value).map_err(|_| {
                primitives::backtrack_err("mana-value disjunction", "signed mana value")
            })
        })
        .collect()
}

fn parse_play_from_zone<'a>(input: &mut LexStream<'a>) -> WResult<PlayFromZoneFact<'a>> {
    let filter_tokens = take_until(input, 1, || semantic_kw("from"))?;
    semantic_kw("from").parse_next(input)?;
    let zone = zone_location.parse_next(input)?;
    semantic_finish(input)?;
    let filter_tokens = trim_sentence_edges(filter_tokens);
    Ok(PlayFromZoneFact {
        filter_tokens,
        zone,
    })
}

fn parse_lands_from_top_library<'a>(input: &mut LexStream<'a>) -> WResult<LandsFromTopLibraryFact> {
    semantic_kw("lands").parse_next(input)?;
    semantic_kw("from").parse_next(input)?;
    top_of_your_library.parse_next(input)?;
    semantic_finish(input)?;
    Ok(LandsFromTopLibraryFact)
}

fn parse_lands_and_cast_from_top_library<'a>(
    input: &mut LexStream<'a>,
) -> WResult<LandsAndCastFromTopLibraryFact<'a>> {
    semantic_phrase(&["lands", "and", "cast"]).parse_next(input)?;
    let spell_filter_tokens = take_until(input, 1, || semantic_kw("from"))?;
    semantic_kw("from").parse_next(input)?;
    top_of_your_library.parse_next(input)?;
    semantic_finish(input)?;
    Ok(LandsAndCastFromTopLibraryFact {
        spell_filter_tokens: trim_sentence_edges(spell_filter_tokens),
    })
}

fn parse_flash_grant<'a>(input: &mut LexStream<'a>) -> WResult<FlashGrantFact<'a>> {
    let filter_tokens = take_until(input, 1, || flash_grant_tail)?;
    let lifetime = flash_grant_tail(input)?;
    semantic_finish(input)?;
    Ok(FlashGrantFact {
        filter_tokens: trim_sentence_edges(filter_tokens),
        lifetime,
    })
}

fn parse_free_cast_from_zone<'a>(input: &mut LexStream<'a>) -> WResult<FreeCastFromZoneFact<'a>> {
    alt((
        parse_mana_value_before_zone_free_cast,
        parse_mana_value_after_zone_free_cast,
        parse_command_zone_free_cast,
        parse_plain_free_cast,
    ))
    .parse_next(input)
}

fn parse_plain_free_cast<'a>(input: &mut LexStream<'a>) -> WResult<FreeCastFromZoneFact<'a>> {
    let filter_tokens = take_until(input, 1, || semantic_kw("from"))?;
    semantic_kw("from").parse_next(input)?;
    let zone = zone_location.parse_next(input)?;
    without_paying_mana_cost(input)?;
    semantic_finish(input)?;
    Ok(free_cast_fact(filter_tokens, zone, None))
}

fn parse_mana_value_before_zone_free_cast<'a>(
    input: &mut LexStream<'a>,
) -> WResult<FreeCastFromZoneFact<'a>> {
    let filter_tokens = take_until(input, 1, || semantic_phrase(&["with", "mana", "value"]))?;
    semantic_phrase(&["with", "mana", "value"]).parse_next(input)?;
    let comparison_tokens = take_until(input, 1, || semantic_kw("from"))?;
    semantic_kw("from").parse_next(input)?;
    let zone = zone_location.parse_next(input)?;
    without_paying_mana_cost(input)?;
    semantic_finish(input)?;
    Ok(free_cast_fact(
        filter_tokens,
        zone,
        Some(ManaValueComparisonTokens {
            tokens: trim_sentence_edges(comparison_tokens),
            placement: ManaValuePlacementFact::BeforeZone,
        }),
    ))
}

fn parse_mana_value_after_zone_free_cast<'a>(
    input: &mut LexStream<'a>,
) -> WResult<FreeCastFromZoneFact<'a>> {
    let filter_tokens = take_until(input, 1, || semantic_kw("from"))?;
    semantic_kw("from").parse_next(input)?;
    let zone = zone_location.parse_next(input)?;
    semantic_phrase(&["with", "mana", "value"]).parse_next(input)?;
    let comparison_tokens = take_until(input, 1, || without_paying_mana_cost)?;
    without_paying_mana_cost(input)?;
    semantic_finish(input)?;
    Ok(free_cast_fact(
        filter_tokens,
        zone,
        Some(ManaValueComparisonTokens {
            tokens: trim_sentence_edges(comparison_tokens),
            placement: ManaValuePlacementFact::AfterZone,
        }),
    ))
}

fn parse_command_zone_free_cast<'a>(
    input: &mut LexStream<'a>,
) -> WResult<FreeCastFromZoneFact<'a>> {
    let filter_tokens = take_until(input, 1, || {
        semantic_phrase(&["from", "the", "command", "zone"])
    })?;
    semantic_phrase(&["from", "the", "command", "zone"]).parse_next(input)?;
    without_paying_mana_cost(input)?;
    semantic_finish(input)?;
    Ok(free_cast_fact(filter_tokens, Zone::Command, None))
}

fn free_cast_fact<'a>(
    filter_tokens: &'a [OwnedLexToken],
    zone: Zone,
    mana_value: Option<ManaValueComparisonTokens<'a>>,
) -> FreeCastFromZoneFact<'a> {
    let filter_tokens = trim_sentence_edges(filter_tokens);
    FreeCastFromZoneFact {
        filter_tokens,
        zone,
        mana_value,
        subject_number: spell_number(filter_tokens),
    }
}

fn spell_number(tokens: &[OwnedLexToken]) -> SpellNumberFact {
    let singular = primitives::find_prefix(tokens, || semantic_kw("spell")).is_some();
    let plural = primitives::find_prefix(tokens, || semantic_kw("spells")).is_some();
    match (singular, plural) {
        (true, false) => SpellNumberFact::Singular,
        (false, true) => SpellNumberFact::Plural,
        (true, true) => SpellNumberFact::Mixed,
        (false, false) => SpellNumberFact::Unspecified,
    }
}

fn zone_location<'a>(input: &mut LexStream<'a>) -> WResult<Zone> {
    alt((
        top_of_your_library.value(Zone::Library),
        semantic_phrase(&["your", "graveyard"]).value(Zone::Graveyard),
        semantic_phrase(&["your", "hand"]).value(Zone::Hand),
        semantic_kw("exile").value(Zone::Exile),
    ))
    .parse_next(input)
}

fn top_of_your_library<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    semantic_phrase(&["the", "top", "of", "your", "library"])
        .void()
        .parse_next(input)
}

fn flash_grant_tail<'a>(input: &mut LexStream<'a>) -> WResult<ZonePermissionLifetimeFact> {
    alt((
        (
            semantic_phrase(&["as", "though", "they"]),
            alt((semantic_kw("had"), semantic_kw("have"))),
            semantic_kw("flash"),
        )
            .value(ZonePermissionLifetimeFact::Static),
        (
            semantic_phrase(&["this", "turn", "as", "though", "they"]),
            alt((semantic_kw("had"), semantic_kw("have"))),
            semantic_kw("flash"),
        )
            .value(ZonePermissionLifetimeFact::ThisTurn),
        (
            semantic_kw("until"),
            alt((
                semantic_phrase(&["end", "of", "turn", "as", "though", "they"]),
                semantic_phrase(&["the", "end", "of", "turn", "as", "though", "they"]),
            )),
            alt((semantic_kw("had"), semantic_kw("have"))),
            semantic_kw("flash"),
        )
            .value(ZonePermissionLifetimeFact::UntilEndOfTurn),
    ))
    .parse_next(input)
}

fn without_paying_mana_cost<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        semantic_phrase(&["without", "paying", "its", "mana", "cost"]),
        semantic_phrase(&["without", "paying", "their", "mana", "cost"]),
        semantic_phrase(&["without", "paying", "their", "mana", "costs"]),
        semantic_phrase(&["without", "paying", "that", "card", "mana", "cost"]),
        semantic_phrase(&["without", "paying", "that", "cards", "mana", "cost"]),
    ))
    .void()
    .parse_next(input)
}

fn take_until<'a, O, P, F>(
    input: &mut LexStream<'a>,
    minimum: usize,
    make_end: F,
) -> WResult<&'a [OwnedLexToken]>
where
    F: Fn() -> P + Copy,
    P: Parser<LexStream<'a>, O, ErrMode<ContextError>>,
{
    repeat_till::<_, _, (), _, _, _, _>(minimum.., any.void(), peek(make_end()).void())
        .map(|((), ())| ())
        .take()
        .parse_next(input)
}

fn trim_sentence_edges(mut tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    tokens = trim_lexed_commas(tokens);
    while tokens
        .last()
        .is_some_and(|token| matches!(token.kind, TokenKind::Period | TokenKind::Semicolon))
    {
        tokens = &tokens[..tokens.len() - 1];
    }
    trim_lexed_commas(tokens)
}

fn semantic_kw<'a>(
    expected: &'static str,
) -> impl Parser<LexStream<'a>, (), ErrMode<ContextError>> {
    move |input: &mut LexStream<'a>| {
        while input
            .peek_token()
            .is_some_and(|token| token.parser_word_pieces().is_empty())
        {
            any.parse_next(input)?;
        }
        any.verify(|token: &&OwnedLexToken| {
            token.is_word(expected)
                || matches!(token.parser_word_pieces(), [piece] if piece.text == expected)
        })
        .void()
        .parse_next(input)
    }
}

fn semantic_phrase<'a>(
    expected: &'static [&'static str],
) -> impl Parser<LexStream<'a>, (), ErrMode<ContextError>> {
    move |input: &mut LexStream<'a>| {
        for word in expected {
            semantic_kw(word).parse_next(input)?;
        }
        Ok(())
    }
}

fn semantic_noise<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    any.verify(|token: &&OwnedLexToken| token.parser_word_pieces().is_empty())
        .void()
        .parse_next(input)
}

fn semantic_finish<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    repeat::<_, _, (), _, _>(0.., semantic_noise).parse_next(input)?;
    eof.void().parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::{lex_line, render_token_slice};

    fn lex(text: &str) -> Vec<OwnedLexToken> {
        lex_line(text, 0).expect("zone permission fixture should lex")
    }

    #[test]
    fn typed_permission_zone_free_cast_migration_parses_zone_and_flash_facts() {
        let play_tokens = lex("instant and sorcery spells from your graveyard");
        let play = parse_play_from_zone_tokens(&play_tokens).unwrap();
        assert_eq!(play.zone, Zone::Graveyard);
        assert_eq!(
            render_token_slice(play.filter_tokens),
            "instant and sorcery spells"
        );

        assert!(
            parse_lands_from_top_library_tokens(&lex("lands from the top of your library"))
                .is_some()
        );

        let combined_tokens = lex("lands and cast creature spells from the top of your library");
        let combined = parse_lands_and_cast_from_top_library_tokens(&combined_tokens).unwrap();
        assert_eq!(
            render_token_slice(combined.spell_filter_tokens),
            "creature spells"
        );

        let flash_tokens = lex("creature spells this turn as though they had flash");
        let flash = parse_flash_grant_tokens(&flash_tokens).unwrap();
        assert_eq!(flash.lifetime, ZonePermissionLifetimeFact::ThisTurn);

        let end_step_flash_tokens =
            lex("artifact spells until the end of turn as though they had flash");
        assert_eq!(
            parse_flash_grant_tokens(&end_step_flash_tokens)
                .unwrap()
                .lifetime,
            ZonePermissionLifetimeFact::UntilEndOfTurn
        );
    }

    #[test]
    fn typed_permission_zone_free_cast_migration_parses_free_cast_variants() {
        let plain_tokens = lex("a spell from your hand without paying its mana cost");
        let plain = parse_free_cast_from_zone_tokens(&plain_tokens).unwrap();
        assert_eq!(plain.zone, Zone::Hand);
        assert_eq!(plain.subject_number, SpellNumberFact::Singular);
        assert!(plain.mana_value.is_none());

        let limited_tokens =
            lex("a spell with mana value 3 or less from your hand without paying its mana cost");
        let limited = parse_free_cast_from_zone_tokens(&limited_tokens).unwrap();
        let comparison = limited.mana_value.unwrap();
        assert_eq!(comparison.placement, ManaValuePlacementFact::BeforeZone);
        assert_eq!(render_token_slice(comparison.tokens), "3 or less");

        let discrete_tokens = lex(
            "an instant or sorcery spell with mana value 1 or 2 from your hand without paying its mana cost",
        );
        let discrete = parse_free_cast_from_zone_tokens(&discrete_tokens).unwrap();
        assert_eq!(
            parse_mana_value_one_of_tokens(discrete.mana_value.unwrap().tokens),
            Some(vec![1, 2])
        );

        let zone_first_tokens = lex(
            "a spell from your graveyard with mana value 2 or less without paying its mana cost",
        );
        let zone_first = parse_free_cast_from_zone_tokens(&zone_first_tokens).unwrap();
        assert_eq!(
            zone_first.mana_value.unwrap().placement,
            ManaValuePlacementFact::AfterZone
        );

        let command_tokens =
            lex("your commander from the command zone without paying its mana cost");
        assert_eq!(
            parse_free_cast_from_zone_tokens(&command_tokens)
                .unwrap()
                .zone,
            Zone::Command
        );
    }
}
