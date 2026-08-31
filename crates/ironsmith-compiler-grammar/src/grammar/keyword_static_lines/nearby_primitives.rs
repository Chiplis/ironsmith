use winnow::combinator::{alt, eof, opt, peek, repeat, repeat_till};
use winnow::error::{ContextError, ErrMode, ModalResult as WResult};
use winnow::prelude::*;
use winnow::token::any;

use super::super::super::lexer::{LexStream, OwnedLexToken, TokenKind, trim_lexed_commas};
use super::super::primitives;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DontUntapDuringControllersStepSpec<'a> {
    pub subject_tokens: &'a [OwnedLexToken],
    pub singular_subject: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraveyardMetricKind {
    CardTypes,
    ManaValues,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WardCostSpec<'a> {
    pub cost_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdditionalVoteKind {
    OptionalTime,
    MandatoryVote,
}

pub fn parse_dungeon_room_trigger_duplication_marker_tokens(tokens: &[OwnedLexToken]) -> bool {
    semantic_all(
        tokens,
        semantic_phrase(&[
            "room",
            "abilities",
            "of",
            "dungeons",
            "you",
            "own",
            "trigger",
            "additional",
            "time",
        ]),
        "dungeon-room trigger duplication marker",
    )
}

pub fn parse_ward_abilities_dont_trigger_marker_tokens(tokens: &[OwnedLexToken]) -> bool {
    semantic_all(
        tokens,
        (
            semantic_phrase(&["ward", "abilities", "of", "those", "creatures"]),
            alt((semantic_kw("dont"), semantic_kw("don't"))),
            semantic_kw("trigger"),
        )
            .void(),
        "ward abilities do-not-trigger marker",
    )
}

pub fn parse_ward_cost_tokens(tokens: &[OwnedLexToken]) -> Option<WardCostSpec<'_>> {
    crate::grammar::primitives::probe_all(tokens, parse_ward_cost_lexed, "ward cost")
}

pub fn parse_additional_vote_tokens(tokens: &[OwnedLexToken]) -> Option<AdditionalVoteKind> {
    crate::grammar::primitives::probe_all(
        tokens,
        parse_additional_vote_lexed,
        "additional vote static line",
    )
}

pub fn parse_dont_untap_during_controllers_step_tokens(
    tokens: &[OwnedLexToken],
) -> Option<DontUntapDuringControllersStepSpec<'_>> {
    crate::grammar::primitives::probe_all(
        tokens,
        parse_dont_untap_during_controllers_step_lexed,
        "don't-untap during controller step",
    )
}

pub fn parse_there_is_or_are_quantified_tokens(
    tokens: &[OwnedLexToken],
) -> Option<&[OwnedLexToken]> {
    primitives::parse_prefix(
        tokens,
        (
            primitives::kw("there"),
            alt((primitives::kw("is"), primitives::kw("are"))),
        ),
    )
    .map(|(_, rest)| trim_lexed_commas(rest))
}

pub fn parse_graveyard_metric_tokens(tokens: &[OwnedLexToken]) -> Option<GraveyardMetricKind> {
    crate::grammar::primitives::probe_all(tokens, parse_graveyard_metric_lexed, "graveyard metric")
}

pub fn parse_damage_not_removed_cleanup_tokens(tokens: &[OwnedLexToken]) -> bool {
    semantic_all(
        tokens,
        (
            semantic_kw("damage"),
            alt((semantic_kw("isnt"), semantic_kw("isn't"))),
            semantic_phrase(&[
                "removed", "from", "this", "creature", "during", "cleanup", "steps",
            ]),
        )
            .void(),
        "damage-not-removed cleanup line",
    )
}

fn parse_dont_untap_during_controllers_step_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<DontUntapDuringControllersStepSpec<'a>> {
    let subject_tokens = repeat_till(1.., any.void(), peek(parse_dont_verb))
        .map(|((), _)| ())
        .take()
        .parse_next(input)?;
    let singular_subject = parse_dont_verb(input)?;
    semantic_kw("untap").parse_next(input)?;
    semantic_kw("during").parse_next(input)?;
    alt((semantic_kw("their"), semantic_kw("its"))).parse_next(input)?;
    alt((
        semantic_kw("controller"),
        semantic_kw("controller's"),
        semantic_kw("controllers"),
    ))
    .parse_next(input)?;
    semantic_kw("untap").parse_next(input)?;
    alt((semantic_kw("step"), semantic_kw("steps"))).parse_next(input)?;
    semantic_finish(input)?;
    Ok(DontUntapDuringControllersStepSpec {
        subject_tokens: trim_lexed_commas(subject_tokens),
        singular_subject,
    })
}

fn parse_ward_cost_lexed<'a>(input: &mut LexStream<'a>) -> WResult<WardCostSpec<'a>> {
    primitives::kw("ward").parse_next(input)?;
    opt(alt((
        primitives::token_kind(TokenKind::Dash),
        primitives::token_kind(TokenKind::EmDash),
        primitives::token_kind(TokenKind::Comma),
    ))
    .void())
    .parse_next(input)?;
    let cost_tokens = repeat::<_, _, (), _, _>(1.., any.void())
        .take()
        .parse_next(input)?;
    Ok(WardCostSpec { cost_tokens })
}

fn parse_additional_vote_lexed<'a>(input: &mut LexStream<'a>) -> WResult<AdditionalVoteKind> {
    let kind = alt((
        semantic_phrase(&[
            "while",
            "voting",
            "you",
            "may",
            "vote",
            "additional",
            "time",
        ])
        .value(AdditionalVoteKind::OptionalTime),
        semantic_phrase(&["while", "voting", "you", "get", "additional", "vote"])
            .value(AdditionalVoteKind::MandatoryVote),
    ))
    .parse_next(input)?;
    opt(semantic_phrase(&[
        "votes",
        "can",
        "be",
        "for",
        "different",
        "choices",
        "or",
        "for",
        "same",
        "choice",
    ]))
    .parse_next(input)?;
    semantic_finish(input)?;
    Ok(kind)
}

fn parse_dont_verb<'a>(input: &mut LexStream<'a>) -> WResult<bool> {
    alt((
        alt((primitives::kw("doesn't"), primitives::kw("doesnt"))).value(true),
        alt((primitives::kw("don't"), primitives::kw("dont"))).value(false),
    ))
    .parse_next(input)
}

fn parse_graveyard_metric_lexed<'a>(input: &mut LexStream<'a>) -> WResult<GraveyardMetricKind> {
    let kind = alt((
        (
            semantic_kw("card"),
            alt((semantic_kw("type"), semantic_kw("types"))),
        )
            .value(GraveyardMetricKind::CardTypes),
        (
            winnow::combinator::opt(alt((semantic_kw("card"), semantic_kw("cards")))),
            semantic_kw("mana"),
            alt((semantic_kw("value"), semantic_kw("values"))),
        )
            .value(GraveyardMetricKind::ManaValues),
    ))
    .parse_next(input)?;
    semantic_phrase(&["among", "cards", "in", "your", "graveyard"]).parse_next(input)?;
    semantic_finish(input)?;
    Ok(kind)
}

pub(super) fn semantic_all<'a, P>(tokens: &'a [OwnedLexToken], parser: P, label: &str) -> bool
where
    P: Parser<LexStream<'a>, (), ErrMode<ContextError>>,
{
    primitives::parse_all(tokens, (parser, semantic_finish).void(), label).is_ok()
}

pub(super) fn semantic_kw<'a>(
    expected: &'static str,
) -> impl Parser<LexStream<'a>, (), ErrMode<ContextError>> {
    (
        repeat::<_, _, (), _, _>(0.., semantic_noise),
        any.verify(move |token: &&OwnedLexToken| {
            token.is_word(expected)
                || matches!(token.parser_word_pieces(), [piece] if piece.text == expected)
        }),
    )
        .void()
}

pub(super) fn semantic_phrase<'a>(
    expected: &'static [&'static str],
) -> impl Parser<LexStream<'a>, (), ErrMode<ContextError>> {
    move |input: &mut LexStream<'a>| {
        for word in expected {
            semantic_kw(word).parse_next(input)?;
        }
        Ok(())
    }
}

pub(super) fn semantic_noise<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    any.verify(|token: &&OwnedLexToken| {
        token.parser_word_pieces().is_empty()
            || token.is_word("a")
            || token.is_word("an")
            || token.is_word("the")
    })
    .void()
    .parse_next(input)
}

pub(super) fn semantic_finish<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    repeat::<_, _, (), _, _>(0.., semantic_noise).parse_next(input)?;
    eof.void().parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::super::super::super::lexer::{lex_line, render_token_slice};
    use super::*;

    #[test]
    fn parses_nearby_static_primitives() {
        let tokens = lex_line(
            "Artifacts don't untap during their controllers' untap steps.",
            0,
        )
        .unwrap();
        let parsed = parse_dont_untap_during_controllers_step_tokens(&tokens).unwrap();
        assert_eq!(render_token_slice(parsed.subject_tokens), "Artifacts");
        assert!(!parsed.singular_subject);

        let tokens = lex_line(
            "Enchanted creature doesn't untap during its controller's untap step.",
            0,
        )
        .unwrap();
        let parsed = parse_dont_untap_during_controllers_step_tokens(&tokens).unwrap();
        assert_eq!(
            render_token_slice(parsed.subject_tokens),
            "Enchanted creature"
        );
        assert!(parsed.singular_subject);

        let tokens = lex_line("card types among cards in your graveyard", 0).unwrap();
        assert_eq!(
            parse_graveyard_metric_tokens(&tokens),
            Some(GraveyardMetricKind::CardTypes)
        );

        let tokens = lex_line(
            "Damage isn't removed from this creature during cleanup steps.",
            0,
        )
        .unwrap();
        assert!(parse_damage_not_removed_cleanup_tokens(&tokens));

        let tokens = lex_line("Ward—Sacrifice a permanent.", 0).unwrap();
        let ward = parse_ward_cost_tokens(&tokens).unwrap();
        assert_eq!(
            render_token_slice(ward.cost_tokens),
            "Sacrifice a permanent."
        );

        let tokens = lex_line(
            "While voting, you get an additional vote. (The votes can be for different choices or for the same choice.)",
            0,
        )
        .unwrap();
        assert_eq!(
            parse_additional_vote_tokens(&tokens),
            Some(AdditionalVoteKind::MandatoryVote)
        );

        let tokens = lex_line("While voting, you may vote an additional time.", 0).unwrap();
        assert_eq!(
            parse_additional_vote_tokens(&tokens),
            Some(AdditionalVoteKind::OptionalTime)
        );
    }
}
