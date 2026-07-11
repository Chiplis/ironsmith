use winnow::combinator::{alt, eof, opt, peek, repeat, repeat_till};
use winnow::error::{ContextError, ErrMode, ModalResult as WResult};
use winnow::prelude::*;
use winnow::token::any;

use super::super::super::lexer::{LexStream, OwnedLexToken, trim_lexed_commas};
use super::super::primitives;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EtbPlayedByOpponentKind {
    YourOpponents,
    AnOpponent,
    Opponents,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct EtbPlayedByOpponentSuffix<'a> {
    pub(crate) filter_tokens: &'a [OwnedLexToken],
    pub(crate) kind: EtbPlayedByOpponentKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EtbLandRevealTappedSubject {
    ThisLand,
    It,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EtbTaggedManaValueReference {
    ExiledCard,
    TriggeringSpell,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EtbAmongMetric {
    CardTypesAmongCards,
    CardTypesAmong,
    BasicLandTypesAmong,
    CreatureTypesAmong,
    ColorsAmong,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct EtbAmongScope<'a> {
    pub(crate) metric: EtbAmongMetric,
    pub(crate) scope_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct EtbStaticAbilitiesAmongScope<'a> {
    pub(crate) ability_tokens: &'a [OwnedLexToken],
    pub(crate) scope_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EtbGraveyardOwner {
    You,
    Opponent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EtbAsLongAsClause<'a> {
    ThisInYourGraveyard {
        continuation_tokens: &'a [OwnedLexToken],
    },
    Condition {
        condition_tokens: &'a [OwnedLexToken],
        continuation_tokens: &'a [OwnedLexToken],
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EtbOtherTypeNoun {
    Type,
    Types,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct EtbBecomesAdditionalTypeTail<'a> {
    pub(crate) descriptor_tokens: &'a [OwnedLexToken],
    pub(crate) other_type_noun: EtbOtherTypeNoun,
    pub(crate) trailing_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EtbSelfSubject {
    Creature,
    Permanent,
    Object,
}

pub(crate) fn etb_tokens_have_entry_verb(tokens: &[OwnedLexToken]) -> bool {
    tokens_have_parser(tokens, || {
        alt((semantic_kw("enter"), semantic_kw("enters"))).void()
    })
}

pub(crate) fn etb_tokens_have_or_marker(tokens: &[OwnedLexToken]) -> bool {
    tokens_have_parser(tokens, || semantic_kw("or"))
}

pub(crate) fn etb_tokens_have_unless_marker(tokens: &[OwnedLexToken]) -> bool {
    tokens_have_parser(tokens, || semantic_kw("unless"))
}

pub(crate) fn etb_tokens_have_tapped_marker(tokens: &[OwnedLexToken]) -> bool {
    tokens_have_parser(tokens, || semantic_kw("tapped"))
}

pub(crate) fn etb_tokens_have_untapped_marker(tokens: &[OwnedLexToken]) -> bool {
    tokens_have_parser(tokens, || semantic_kw("untapped"))
}

pub(crate) fn etb_tokens_have_copy_marker(tokens: &[OwnedLexToken]) -> bool {
    tokens_have_parser(tokens, || semantic_kw("copy"))
}

pub(crate) fn parse_etb_played_by_opponent_suffix_tokens(
    tokens: &[OwnedLexToken],
) -> Option<EtbPlayedByOpponentSuffix<'_>> {
    primitives::parse_all(
        tokens,
        parse_etb_played_by_opponent_suffix_lexed,
        "ETB played-by-opponent suffix",
    )
    .ok()
}

pub(crate) fn parse_as_this_land_enters_prefix_tokens(
    tokens: &[OwnedLexToken],
) -> Option<&[OwnedLexToken]> {
    parse_semantic_prefix(tokens, &["as", "this", "land", "enters"])
}

pub(crate) fn etb_tokens_have_reveal_from_hand_marker(tokens: &[OwnedLexToken]) -> bool {
    tokens_have_word(tokens, "reveal")
        && tokens_have_word(tokens, "from")
        && tokens_have_word(tokens, "hand")
}

pub(crate) fn find_if_you_dont_tail_tokens(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    let (_, _, rest) = primitives::find_prefix(tokens, || {
        (
            semantic_phrase(&["if", "you"]),
            alt((semantic_kw("dont"), semantic_kw("don't"))),
        )
            .void()
    })?;
    Some(rest)
}

pub(crate) fn parse_land_reveal_trailing_tapped_prefix_tokens(
    tokens: &[OwnedLexToken],
) -> Option<EtbLandRevealTappedSubject> {
    primitives::parse_prefix(tokens, parse_land_reveal_trailing_tapped_prefix_lexed)
        .map(|(subject, _)| subject)
}

pub(crate) fn parse_first_three_turns_prefix_tokens(
    tokens: &[OwnedLexToken],
) -> Option<&[OwnedLexToken]> {
    let parser = (
        alt((
            semantic_kw("its"),
            semantic_kw("it's"),
            semantic_phrase(&["it", "s"]),
        )),
        semantic_phrase(&[
            "your", "first", "second", "or", "third", "turn", "of", "the", "game",
        ]),
    )
        .void();
    primitives::parse_prefix(tokens, parser).map(|(_, rest)| rest)
}

pub(crate) fn etb_tokens_have_devotion_value_marker(tokens: &[OwnedLexToken]) -> bool {
    tokens_have_word(tokens, "devotion")
}

pub(crate) fn etb_tokens_have_all_players_hand_count_value(tokens: &[OwnedLexToken]) -> bool {
    tokens_have_word(tokens, "cards")
        && tokens_have_word(tokens, "in")
        && tokens_have_word(tokens, "all")
        && tokens_have_word(tokens, "players")
        && tokens_have_parser(tokens, || {
            alt((semantic_kw("hand"), semantic_kw("hands"))).void()
        })
}

pub(crate) fn parse_same_name_as_triggering_spell_graveyard_value_tokens(
    tokens: &[OwnedLexToken],
) -> bool {
    primitives::parse_all(
        tokens,
        parse_same_name_as_triggering_spell_graveyard_value_lexed,
        "same-name-as-triggering-spell graveyard value",
    )
    .is_ok()
}

pub(crate) fn parse_tagged_mana_value_reference_tokens(
    tokens: &[OwnedLexToken],
) -> Option<EtbTaggedManaValueReference> {
    primitives::parse_all(
        tokens,
        parse_tagged_mana_value_reference_lexed,
        "tagged mana-value reference",
    )
    .ok()
}

pub(crate) fn etb_tokens_have_your_hand_count_value(tokens: &[OwnedLexToken]) -> bool {
    tokens_have_word(tokens, "cards")
        && tokens_have_word(tokens, "in")
        && tokens_have_word(tokens, "your")
        && tokens_have_parser(tokens, || {
            alt((semantic_kw("hand"), semantic_kw("hands"))).void()
        })
}

pub(crate) fn etb_tokens_have_common_creature_type_value(tokens: &[OwnedLexToken]) -> bool {
    tokens_have_word(tokens, "creature")
        && tokens_have_word(tokens, "type")
        && tokens_have_word(tokens, "common")
}

pub(crate) fn parse_etb_among_scope_tokens(tokens: &[OwnedLexToken]) -> Option<EtbAmongScope<'_>> {
    primitives::parse_all(tokens, parse_etb_among_scope_lexed, "ETB among scope").ok()
}

pub(crate) fn parse_etb_static_abilities_among_scope_tokens(
    tokens: &[OwnedLexToken],
) -> Option<EtbStaticAbilitiesAmongScope<'_>> {
    primitives::parse_all(
        tokens,
        parse_etb_static_abilities_among_scope_lexed,
        "ETB static-abilities-among scope",
    )
    .ok()
}

pub(crate) fn etb_tokens_have_graveyard_marker(tokens: &[OwnedLexToken]) -> bool {
    tokens_have_parser(tokens, || {
        alt((semantic_kw("graveyard"), semantic_kw("graveyards"))).void()
    })
}

pub(crate) fn etb_tokens_have_and_graveyard_marker(tokens: &[OwnedLexToken]) -> bool {
    tokens_have_parser(tokens, || semantic_phrase(&["and", "graveyard"]))
}

pub(crate) fn etb_tokens_have_sacrificed_marker(tokens: &[OwnedLexToken]) -> bool {
    tokens_have_word(tokens, "sacrificed")
}

pub(crate) fn parse_etb_graveyard_owner_tokens(
    tokens: &[OwnedLexToken],
) -> Option<EtbGraveyardOwner> {
    if tokens_have_parser(tokens, || semantic_phrase(&["your", "graveyard"])) {
        return Some(EtbGraveyardOwner::You);
    }
    if tokens_have_parser(tokens, || {
        (
            alt((
                semantic_kw("opponents"),
                semantic_kw("opponent's"),
                semantic_kw("opponent"),
            )),
            semantic_kw("graveyard"),
        )
            .void()
    }) {
        return Some(EtbGraveyardOwner::Opponent);
    }
    None
}

pub(crate) fn parse_etb_as_long_as_clause_tokens(
    tokens: &[OwnedLexToken],
) -> Option<EtbAsLongAsClause<'_>> {
    let parsed = primitives::parse_all(
        tokens,
        parse_etb_as_long_as_clause_lexed,
        "ETB as-long-as clause",
    )
    .ok()?;
    if primitives::parse_all(
        parsed.condition_tokens,
        parse_this_in_your_graveyard_condition_lexed,
        "this-in-your-graveyard condition",
    )
    .is_ok()
    {
        Some(EtbAsLongAsClause::ThisInYourGraveyard {
            continuation_tokens: parsed.continuation_tokens,
        })
    } else {
        Some(EtbAsLongAsClause::Condition {
            condition_tokens: parsed.condition_tokens,
            continuation_tokens: parsed.continuation_tokens,
        })
    }
}

pub(crate) fn etb_tokens_have_with_additional_counters(tokens: &[OwnedLexToken]) -> bool {
    tokens_have_word(tokens, "with")
        && tokens_have_word(tokens, "additional")
        && tokens_have_parser(tokens, || {
            alt((semantic_kw("counter"), semantic_kw("counters"))).void()
        })
}

pub(crate) fn parse_it_becomes_your_choice_of_prefix_tokens(
    tokens: &[OwnedLexToken],
) -> Option<&[OwnedLexToken]> {
    parse_semantic_prefix(tokens, &["it", "becomes", "your", "choice", "of"])
}

pub(crate) fn parse_it_becomes_additional_type_tail_tokens(
    tokens: &[OwnedLexToken],
) -> Option<EtbBecomesAdditionalTypeTail<'_>> {
    primitives::parse_all(
        tokens,
        parse_it_becomes_additional_type_tail_lexed,
        "ETB becomes-additional-type tail",
    )
    .ok()
}

pub(crate) fn parse_etb_self_subject_tokens(tokens: &[OwnedLexToken]) -> Option<EtbSelfSubject> {
    primitives::parse_all(tokens, parse_etb_self_subject_lexed, "ETB self subject").ok()
}

pub(crate) fn parse_face_up_choice_tail_tokens(
    tokens: &[OwnedLexToken],
) -> Option<&[OwnedLexToken]> {
    primitives::parse_all(
        tokens,
        parse_face_up_choice_tail_lexed,
        "ETB face-up choice tail",
    )
    .ok()
}

pub(crate) fn etb_tokens_have_your_party_size_value(tokens: &[OwnedLexToken]) -> bool {
    tokens_have_word(tokens, "party")
        && tokens_have_word(tokens, "your")
        && tokens_have_parser(tokens, || {
            alt((semantic_kw("creature"), semantic_kw("creatures"))).void()
        })
}

fn parse_etb_played_by_opponent_suffix_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<EtbPlayedByOpponentSuffix<'a>> {
    let filter_tokens = repeat_till(1.., any.void(), peek(parse_played_by_opponent_kind))
        .map(|((), _)| ())
        .take()
        .parse_next(input)?;
    let kind = parse_played_by_opponent_kind(input)?;
    semantic_finish(input)?;
    Ok(EtbPlayedByOpponentSuffix {
        filter_tokens: trim_lexed_commas(filter_tokens),
        kind,
    })
}

fn parse_played_by_opponent_kind<'a>(
    input: &mut LexStream<'a>,
) -> WResult<EtbPlayedByOpponentKind> {
    alt((
        semantic_phrase(&["played", "by", "your", "opponents"])
            .value(EtbPlayedByOpponentKind::YourOpponents),
        (
            semantic_phrase(&["played", "by"]),
            alt((semantic_kw("an"), semantic_kw("a"))),
            semantic_kw("opponent"),
        )
            .value(EtbPlayedByOpponentKind::AnOpponent),
        semantic_phrase(&["played", "by", "opponents"]).value(EtbPlayedByOpponentKind::Opponents),
    ))
    .parse_next(input)
}

fn parse_land_reveal_trailing_tapped_prefix_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<EtbLandRevealTappedSubject> {
    alt((
        (
            semantic_phrase(&["this", "land"]),
            alt((semantic_kw("enter"), semantic_kw("enters"))),
            semantic_kw("tapped"),
        )
            .value(EtbLandRevealTappedSubject::ThisLand),
        (
            semantic_kw("it"),
            alt((semantic_kw("enter"), semantic_kw("enters"))),
            opt(semantic_phrase(&["the", "battlefield"])),
            semantic_kw("tapped"),
        )
            .value(EtbLandRevealTappedSubject::It),
    ))
    .parse_next(input)
}

fn parse_same_name_as_triggering_spell_graveyard_value_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<()> {
    semantic_phrase(&[
        "the",
        "number",
        "of",
        "cards",
        "in",
        "all",
        "graveyards",
        "with",
        "the",
        "same",
        "name",
        "as",
    ])
    .parse_next(input)?;
    alt((semantic_kw("the"), semantic_kw("that"))).parse_next(input)?;
    semantic_kw("spell").parse_next(input)?;
    semantic_finish(input)
}

fn parse_tagged_mana_value_reference_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<EtbTaggedManaValueReference> {
    let reference = alt((
        semantic_phrase(&["the", "mana", "value", "of", "the", "exiled", "card"])
            .value(EtbTaggedManaValueReference::ExiledCard),
        (
            semantic_phrase(&["the", "exiled"]),
            alt((
                semantic_kw("card"),
                semantic_kw("cards"),
                semantic_kw("card's"),
            )),
            semantic_phrase(&["mana", "value"]),
        )
            .value(EtbTaggedManaValueReference::ExiledCard),
        (
            alt((semantic_kw("the"), semantic_kw("that"))),
            alt((
                semantic_kw("spell"),
                semantic_kw("spell's"),
                semantic_kw("spells"),
            )),
            semantic_phrase(&["mana", "value"]),
        )
            .value(EtbTaggedManaValueReference::TriggeringSpell),
    ))
    .parse_next(input)?;
    semantic_finish(input)?;
    Ok(reference)
}

fn parse_etb_among_scope_lexed<'a>(input: &mut LexStream<'a>) -> WResult<EtbAmongScope<'a>> {
    let metric = alt((
        (
            semantic_kw("card"),
            alt((semantic_kw("type"), semantic_kw("types"))),
            semantic_phrase(&["among", "cards"]),
        )
            .value(EtbAmongMetric::CardTypesAmongCards),
        (
            semantic_kw("card"),
            alt((semantic_kw("type"), semantic_kw("types"))),
            semantic_kw("among"),
        )
            .value(EtbAmongMetric::CardTypesAmong),
        (
            semantic_kw("basic"),
            semantic_kw("land"),
            alt((semantic_kw("type"), semantic_kw("types"))),
            semantic_kw("among"),
        )
            .value(EtbAmongMetric::BasicLandTypesAmong),
        (
            semantic_kw("creature"),
            alt((semantic_kw("type"), semantic_kw("types"))),
            semantic_kw("among"),
        )
            .value(EtbAmongMetric::CreatureTypesAmong),
        (
            alt((semantic_kw("color"), semantic_kw("colors"))),
            semantic_kw("among"),
        )
            .value(EtbAmongMetric::ColorsAmong),
    ))
    .parse_next(input)?;
    opt(semantic_kw("the")).parse_next(input)?;
    let scope_tokens = take_nonempty_semantic_body(input)?;
    Ok(EtbAmongScope {
        metric,
        scope_tokens,
    })
}

fn parse_etb_static_abilities_among_scope_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<EtbStaticAbilitiesAmongScope<'a>> {
    alt((semantic_kw("ability"), semantic_kw("abilities"))).parse_next(input)?;
    semantic_phrase(&["from", "among"]).parse_next(input)?;
    let ability_tokens = repeat_till(1.., any.void(), peek(semantic_phrase(&["found", "among"])))
        .map(|((), ())| ())
        .take()
        .parse_next(input)?;
    semantic_phrase(&["found", "among"]).parse_next(input)?;
    let scope_tokens = take_nonempty_semantic_body(input)?;
    Ok(EtbStaticAbilitiesAmongScope {
        ability_tokens: trim_lexed_commas(ability_tokens),
        scope_tokens,
    })
}

#[derive(Debug, Clone, Copy)]
struct RawAsLongAsClause<'a> {
    condition_tokens: &'a [OwnedLexToken],
    continuation_tokens: &'a [OwnedLexToken],
}

fn parse_etb_as_long_as_clause_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<RawAsLongAsClause<'a>> {
    semantic_phrase(&["as", "long", "as"]).parse_next(input)?;
    let condition_tokens = repeat_till(1.., any.void(), peek(primitives::comma()))
        .map(|((), _)| ())
        .take()
        .parse_next(input)?;
    primitives::comma().parse_next(input)?;
    let continuation_tokens = take_nonempty_semantic_body(input)?;
    Ok(RawAsLongAsClause {
        condition_tokens: trim_lexed_commas(condition_tokens),
        continuation_tokens,
    })
}

fn parse_this_in_your_graveyard_condition_lexed<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    semantic_kw("this").parse_next(input)?;
    repeat_till(
        0..,
        any.void(),
        peek(semantic_phrase(&["is", "in", "your", "graveyard"])),
    )
    .map(|((), ())| ())
    .parse_next(input)?;
    semantic_phrase(&["is", "in", "your", "graveyard"]).parse_next(input)?;
    repeat::<_, _, (), _, _>(0.., any.void()).parse_next(input)?;
    Ok(())
}

fn parse_it_becomes_additional_type_tail_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<EtbBecomesAdditionalTypeTail<'a>> {
    semantic_phrase(&["it", "becomes"]).parse_next(input)?;
    let descriptor_tokens = repeat_till(
        1..,
        any.void(),
        peek(semantic_phrase(&["in", "addition", "to", "its", "other"])),
    )
    .map(|((), ())| ())
    .take()
    .parse_next(input)?;
    semantic_phrase(&["in", "addition", "to", "its", "other"]).parse_next(input)?;
    let other_type_noun = alt((
        semantic_kw("type").value(EtbOtherTypeNoun::Type),
        semantic_kw("types").value(EtbOtherTypeNoun::Types),
    ))
    .parse_next(input)?;
    let trailing_tokens = take_semantic_body(input)?;
    Ok(EtbBecomesAdditionalTypeTail {
        descriptor_tokens: trim_lexed_commas(descriptor_tokens),
        other_type_noun,
        trailing_tokens,
    })
}

fn parse_etb_self_subject_lexed<'a>(input: &mut LexStream<'a>) -> WResult<EtbSelfSubject> {
    semantic_kw("this").parse_next(input)?;
    let subject = alt((
        semantic_kw("creature").value(EtbSelfSubject::Creature),
        semantic_kw("permanent").value(EtbSelfSubject::Permanent),
        semantic_kw("object").value(EtbSelfSubject::Object),
    ))
    .parse_next(input)?;
    semantic_finish(input)?;
    Ok(subject)
}

fn parse_face_up_choice_tail_lexed<'a>(input: &mut LexStream<'a>) -> WResult<&'a [OwnedLexToken]> {
    semantic_phrase(&[
        "or", "is", "turned", "face", "up", "it", "becomes", "your", "choice", "of",
    ])
    .parse_next(input)?;
    take_nonempty_semantic_body(input)
}

fn tokens_have_word(tokens: &[OwnedLexToken], expected: &'static str) -> bool {
    tokens_have_parser(tokens, || semantic_kw(expected))
}

fn tokens_have_parser<'a, P, F>(tokens: &'a [OwnedLexToken], make_parser: F) -> bool
where
    F: Fn() -> P,
    P: Parser<LexStream<'a>, (), ErrMode<ContextError>>,
{
    primitives::find_prefix(tokens, make_parser).is_some()
}

fn parse_semantic_prefix<'a>(
    tokens: &'a [OwnedLexToken],
    expected: &'static [&'static str],
) -> Option<&'a [OwnedLexToken]> {
    primitives::parse_prefix(tokens, semantic_phrase(expected)).map(|(_, rest)| rest)
}

fn semantic_kw<'a>(
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

fn take_nonempty_semantic_body<'a>(input: &mut LexStream<'a>) -> WResult<&'a [OwnedLexToken]> {
    let body = repeat_till(1.., any.void(), peek(semantic_finish))
        .map(|((), ())| ())
        .take()
        .parse_next(input)?;
    semantic_finish(input)?;
    let body = trim_lexed_commas(body);
    if body.is_empty() {
        return Err(primitives::backtrack_err(
            "ETB semantic body",
            "non-empty body",
        ));
    }
    Ok(body)
}

fn take_semantic_body<'a>(input: &mut LexStream<'a>) -> WResult<&'a [OwnedLexToken]> {
    let body = repeat_till(0.., any.void(), peek(semantic_finish))
        .map(|((), ())| ())
        .take()
        .parse_next(input)?;
    semantic_finish(input)?;
    Ok(trim_lexed_commas(body))
}

#[cfg(test)]
mod tests {
    use super::super::super::super::lexer::{lex_line, render_token_slice};
    use super::*;

    #[test]
    fn parses_markers_and_opponent_suffixes() {
        let tokens = lex_line("Artifacts played by an opponent.", 0).unwrap();
        let parsed = parse_etb_played_by_opponent_suffix_tokens(&tokens).unwrap();
        assert_eq!(parsed.kind, EtbPlayedByOpponentKind::AnOpponent);
        assert_eq!(render_token_slice(parsed.filter_tokens), "Artifacts");

        let tokens = lex_line("If you don't, this land enters tapped.", 0).unwrap();
        let tail = find_if_you_dont_tail_tokens(&tokens).unwrap();
        assert_eq!(
            parse_land_reveal_trailing_tapped_prefix_tokens(tail),
            Some(EtbLandRevealTappedSubject::ThisLand)
        );
        assert!(etb_tokens_have_entry_verb(&tokens));
        assert!(etb_tokens_have_tapped_marker(&tokens));
    }

    #[test]
    fn parses_value_phrase_facts_and_contractions() {
        let tokens = lex_line("the number of cards in all players' hands.", 0).unwrap();
        assert!(etb_tokens_have_all_players_hand_count_value(&tokens));

        let tokens = lex_line(
            "the number of cards in all graveyards with the same name as that spell.",
            0,
        )
        .unwrap();
        assert!(parse_same_name_as_triggering_spell_graveyard_value_tokens(
            &tokens
        ));

        let tokens = lex_line("that spell's mana value.", 0).unwrap();
        assert_eq!(
            parse_tagged_mana_value_reference_tokens(&tokens),
            Some(EtbTaggedManaValueReference::TriggeringSpell)
        );

        let tokens = lex_line("where X is the number of creatures in your party.", 0).unwrap();
        assert!(etb_tokens_have_your_party_size_value(&tokens));

        let tokens = lex_line("It's your first, second, or third turn of the game.", 0).unwrap();
        assert!(parse_first_three_turns_prefix_tokens(&tokens).is_some());
    }

    #[test]
    fn parses_among_scopes() {
        let tokens = lex_line("basic land types among the lands you control.", 0).unwrap();
        let parsed = parse_etb_among_scope_tokens(&tokens).unwrap();
        assert_eq!(parsed.metric, EtbAmongMetric::BasicLandTypesAmong);
        assert_eq!(render_token_slice(parsed.scope_tokens), "lands you control");

        let tokens = lex_line("card types among cards in all graveyards.", 0).unwrap();
        let parsed = parse_etb_among_scope_tokens(&tokens).unwrap();
        assert_eq!(parsed.metric, EtbAmongMetric::CardTypesAmongCards);
        assert_eq!(render_token_slice(parsed.scope_tokens), "in all graveyards");
        assert!(etb_tokens_have_graveyard_marker(parsed.scope_tokens));

        let tokens = lex_line(
            "abilities from among flying, vigilance, and trample found among creatures you control.",
            0,
        )
        .unwrap();
        let parsed = parse_etb_static_abilities_among_scope_tokens(&tokens).unwrap();
        assert_eq!(
            render_token_slice(parsed.ability_tokens),
            "flying, vigilance, and trample"
        );
        assert_eq!(
            render_token_slice(parsed.scope_tokens),
            "creatures you control"
        );
    }

    #[test]
    fn parses_as_long_as_and_characteristic_tails() {
        let tokens = lex_line(
            "As long as this card is in your graveyard, creatures enter with an additional counter.",
            0,
        )
        .unwrap();
        assert!(matches!(
            parse_etb_as_long_as_clause_tokens(&tokens),
            Some(EtbAsLongAsClause::ThisInYourGraveyard { .. })
        ));

        let tokens =
            lex_line("it becomes a 3/3 Zombie in addition to its other types.", 0).unwrap();
        let parsed = parse_it_becomes_additional_type_tail_tokens(&tokens).unwrap();
        assert_eq!(render_token_slice(parsed.descriptor_tokens), "a 3/3 Zombie");

        let tokens = lex_line("this creature", 0).unwrap();
        assert_eq!(
            parse_etb_self_subject_tokens(&tokens),
            Some(EtbSelfSubject::Creature)
        );
    }
}
