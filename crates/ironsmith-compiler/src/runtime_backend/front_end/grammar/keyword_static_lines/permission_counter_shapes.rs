use winnow::combinator::{alt, opt, peek, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use crate::object::CounterType;

use super::super::super::lexer::{LexStream, OwnedLexToken, trim_lexed_commas};
use super::super::{filters, primitives};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExileCounterPermissionFamily {
    CastNonlandCards,
    PlayLandsAndCastNoncreatureCardsExiledBySource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExileCounterPermissionOwner {
    Any,
    Opponent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExileCounterManaPermission {
    AnyMana,
    SnowSources,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExileCounterPermissionSpec {
    pub(crate) family: ExileCounterPermissionFamily,
    pub(crate) owner: ExileCounterPermissionOwner,
    pub(crate) counter_type: CounterType,
    pub(crate) mana_permission: ExileCounterManaPermission,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PlayPermissionEnterCounterSpec<'a> {
    pub(crate) permission_tokens: &'a [OwnedLexToken],
    pub(crate) counter_type: CounterType,
}

pub(crate) fn parse_exile_counter_permission_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ExileCounterPermissionSpec> {
    primitives::parse_all(
        tokens,
        alt((
            parse_cast_countered_exile_cards_lexed,
            parse_play_source_exiled_countered_cards_lexed,
        )),
        "countered exile-card permission",
    )
    .ok()
}

pub(crate) fn parse_play_permission_enter_counter_tokens(
    tokens: &[OwnedLexToken],
) -> Option<PlayPermissionEnterCounterSpec<'_>> {
    primitives::parse_all(
        tokens,
        parse_play_permission_enter_counter_lexed,
        "play permission with enters-counter rider",
    )
    .ok()
}

fn parse_cast_countered_exile_cards_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<ExileCounterPermissionSpec> {
    primitives::phrase(&[
        "you", "may", "cast", "spells", "from", "among", "cards", "in", "exile",
    ])
    .parse_next(input)?;
    let owner = opt(alt((
        primitives::phrase(&["your", "opponents", "own"]),
        primitives::phrase(&["your", "opponent", "owns"]),
        primitives::phrase(&["opponents", "own"]),
        primitives::phrase(&["opponent", "owns"]),
    )))
    .map(|owner| {
        if owner.is_some() {
            ExileCounterPermissionOwner::Opponent
        } else {
            ExileCounterPermissionOwner::Any
        }
    })
    .parse_next(input)?;
    primitives::kw("with").parse_next(input)?;
    let counter_type = parse_counter_type_before_on_them(input)?;
    let mana_permission = parse_countered_exile_mana_permission(input)?;
    Ok(ExileCounterPermissionSpec {
        family: ExileCounterPermissionFamily::CastNonlandCards,
        owner,
        counter_type,
        mana_permission,
    })
}

fn parse_play_source_exiled_countered_cards_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<ExileCounterPermissionSpec> {
    primitives::phrase(&[
        "you",
        "may",
        "play",
        "lands",
        "and",
        "cast",
        "noncreature",
        "spells",
        "from",
        "among",
        "cards",
        "you",
        "exiled",
        "that",
        "have",
    ])
    .parse_next(input)?;
    let counter_type = parse_counter_type_before_on_them(input)?;
    let mana_permission = parse_countered_exile_mana_permission(input)?;
    Ok(ExileCounterPermissionSpec {
        family: ExileCounterPermissionFamily::PlayLandsAndCastNoncreatureCardsExiledBySource,
        owner: ExileCounterPermissionOwner::Any,
        counter_type,
        mana_permission,
    })
}

fn parse_counter_type_before_on_them(input: &mut LexStream<'_>) -> WResult<CounterType> {
    let counter_tokens = repeat_till::<_, _, (), _, _, _, _>(
        1..,
        any.void(),
        peek(alt((primitives::kw("counter"), primitives::kw("counters")))),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    alt((primitives::kw("counter"), primitives::kw("counters"))).parse_next(input)?;
    primitives::phrase(&["on", "them"]).parse_next(input)?;
    filters::parse_counter_type_from_tokens(trim_lexed_commas(counter_tokens)).ok_or_else(|| {
        primitives::backtrack_err("countered exile-card permission", "known counter type")
    })
}

fn parse_countered_exile_mana_permission(
    input: &mut LexStream<'_>,
) -> WResult<ExileCounterManaPermission> {
    opt(primitives::comma()).parse_next(input)?;
    let permission = alt((
        primitives::phrase(&[
            "and", "you", "may", "spend", "mana", "from", "snow", "sources", "as", "though", "it",
            "were", "mana", "of", "any", "color", "to", "cast", "those", "spells",
        ])
        .value(ExileCounterManaPermission::SnowSources),
        primitives::phrase(&[
            "and", "you", "may", "spend", "mana", "as", "though", "it", "were", "mana", "of",
            "any", "color", "to", "cast", "those", "spells",
        ])
        .value(ExileCounterManaPermission::AnyMana),
    ))
    .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(permission)
}

fn parse_play_permission_enter_counter_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<PlayPermissionEnterCounterSpec<'a>> {
    let permission_tokens =
        repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(primitives::end_of_sentence()))
            .map(|((), _)| ())
            .take()
            .parse_next(input)?;
    primitives::end_of_sentence().parse_next(input)?;
    primitives::phrase(&["if", "you", "do"]).parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&["it", "enters", "with"]).parse_next(input)?;
    alt((primitives::kw("a"), primitives::kw("an"))).parse_next(input)?;
    let counter_tokens =
        repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(primitives::kw("counter")))
            .map(|((), _)| ())
            .take()
            .parse_next(input)?;
    primitives::phrase(&["counter", "on", "it"]).parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    let counter_type = filters::parse_counter_type_from_tokens(trim_lexed_commas(counter_tokens))
        .ok_or_else(|| {
        primitives::backtrack_err("play permission enters-counter rider", "known counter type")
    })?;
    Ok(PlayPermissionEnterCounterSpec {
        permission_tokens: trim_lexed_commas(permission_tokens),
        counter_type,
    })
}

#[cfg(test)]
mod tests {
    use super::super::super::super::lexer::lex_line;
    use super::*;

    #[test]
    fn parses_both_countered_exile_permission_families() {
        let tokens = lex_line(
            "You may cast spells from among cards in exile your opponents own with ice counters on them, and you may spend mana from snow sources as though it were mana of any color to cast those spells.",
            0,
        )
        .unwrap();
        let spec = parse_exile_counter_permission_tokens(&tokens).unwrap();
        assert_eq!(spec.owner, ExileCounterPermissionOwner::Opponent);
        assert_eq!(
            spec.mana_permission,
            ExileCounterManaPermission::SnowSources
        );

        let tokens = lex_line(
            "You may play lands and cast noncreature spells from among cards you exiled that have fetch counters on them, and you may spend mana as though it were mana of any color to cast those spells.",
            0,
        )
        .unwrap();
        let spec = parse_exile_counter_permission_tokens(&tokens).unwrap();
        assert_eq!(
            spec.family,
            ExileCounterPermissionFamily::PlayLandsAndCastNoncreatureCardsExiledBySource
        );
    }

    #[test]
    fn parses_permission_enter_counter_rider() {
        let tokens = lex_line(
            "You may cast this card from your graveyard. If you do, it enters with a finality counter on it.",
            0,
        )
        .unwrap();
        let spec = parse_play_permission_enter_counter_tokens(&tokens).unwrap();
        assert!(!spec.permission_tokens.is_empty());
    }
}
