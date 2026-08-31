use winnow::combinator::{alt, opt, peek, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use crate::object::CounterType;
use crate::target::ObjectFilter;

use super::super::super::lexer::{LexStream, OwnedLexToken, trim_lexed_commas};
use super::super::{filters, primitives};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExileCounterPermissionFamily {
    CastNonlandCards,
    PlayLandsAndCastNoncreatureCardsExiledBySource,
    PlayLandsAndCastSpellsOwnedInExile,
    PlayCardsNotOwnedInExile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExileCounterPermissionOwner {
    Any,
    Opponent,
    You,
    NotYou,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExileCounterManaPermission {
    AnyMana,
    AnyTypeCanBeSpent,
    SnowSources,
    None,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExileCounterPermissionSpec {
    pub family: ExileCounterPermissionFamily,
    pub owner: ExileCounterPermissionOwner,
    pub counter_type: CounterType,
    pub mana_permission: ExileCounterManaPermission,
    pub during_your_turn: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlayPermissionEnterCounterSpec<'a> {
    pub permission_tokens: &'a [OwnedLexToken],
    pub counter_type: CounterType,
    pub additional: bool,
    pub cast_this_way_filter: Option<ObjectFilter>,
}

pub fn parse_exile_counter_permission_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ExileCounterPermissionSpec> {
    crate::grammar::primitives::probe_all(
        tokens,
        alt((
            parse_cast_countered_exile_cards_lexed,
            parse_play_source_exiled_countered_cards_lexed,
            parse_play_owned_countered_exile_cards_lexed,
            parse_play_not_owned_countered_exile_during_turn_lexed,
        )),
        "countered exile-card permission",
    )
}

pub fn parse_play_permission_enter_counter_tokens(
    tokens: &[OwnedLexToken],
) -> Option<PlayPermissionEnterCounterSpec<'_>> {
    crate::grammar::primitives::probe_all(
        tokens,
        parse_play_permission_enter_counter_lexed,
        "play permission with enters-counter rider",
    )
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
        during_your_turn: false,
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
        during_your_turn: false,
    })
}

/// "You may play lands and cast spells from among cards you own in exile
/// with <type> counters on them" — an unrestricted play permission over the
/// player's own countered exile pool (Grolnok, the Omnivore).
fn parse_play_owned_countered_exile_cards_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<ExileCounterPermissionSpec> {
    primitives::phrase(&[
        "you", "may", "play", "lands", "and", "cast", "spells", "from", "among", "cards", "you",
        "own", "in", "exile", "with",
    ])
    .parse_next(input)?;
    let counter_type = parse_counter_type_before_on_them(input)?;
    let mana_permission = alt((
        parse_countered_exile_mana_permission,
        primitives::sentence_end().value(ExileCounterManaPermission::None),
    ))
    .parse_next(input)?;
    Ok(ExileCounterPermissionSpec {
        family: ExileCounterPermissionFamily::PlayLandsAndCastSpellsOwnedInExile,
        owner: ExileCounterPermissionOwner::You,
        counter_type,
        mana_permission,
        during_your_turn: false,
    })
}

/// "During your turn, you may play cards you don't own with <type> counters
/// on them from exile, and mana of any type can be spent to cast those
/// spells" — the stolen-stash permission (Tinybones, Bauble Burglar).
fn parse_play_not_owned_countered_exile_during_turn_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<ExileCounterPermissionSpec> {
    primitives::phrase(&["during", "your", "turn"]).parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&["you", "may", "play", "cards", "you"]).parse_next(input)?;
    alt((primitives::kw("don't"), primitives::kw("dont"))).parse_next(input)?;
    primitives::phrase(&["own", "with"]).parse_next(input)?;
    let counter_type = parse_counter_type_before_on_them(input)?;
    primitives::phrase(&["from", "exile"]).parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&[
        "and", "mana", "of", "any", "type", "can", "be", "spent", "to", "cast", "those", "spells",
    ])
    .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(ExileCounterPermissionSpec {
        family: ExileCounterPermissionFamily::PlayCardsNotOwnedInExile,
        owner: ExileCounterPermissionOwner::NotYou,
        counter_type,
        mana_permission: ExileCounterManaPermission::AnyTypeCanBeSpent,
        during_your_turn: true,
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
    let cast_this_way_filter = alt((
        (
            primitives::phrase(&["if", "you", "do"]),
            opt(primitives::comma()),
            primitives::phrase(&["it", "enters", "with"]),
        )
            .value(None),
        parse_cast_this_way_enters_with_intro.map(Some),
    ))
    .parse_next(input)?;
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
        additional: counter_tokens
            .iter()
            .any(|token| token.is_word("additional")),
        cast_this_way_filter,
    })
}

fn parse_cast_this_way_enters_with_intro(input: &mut LexStream<'_>) -> WResult<ObjectFilter> {
    primitives::phrase(&["if", "you", "cast"]).parse_next(input)?;
    let spell_filter_tokens = repeat_till::<_, _, (), _, _, _, _>(
        1..,
        any.void(),
        peek(primitives::phrase(&["spell", "this", "way"])),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    primitives::phrase(&["spell", "this", "way"]).parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    alt((
        primitives::phrase(&["that", "creature", "enters", "with"]),
        primitives::phrase(&["that", "permanent", "enters", "with"]),
        primitives::phrase(&["it", "enters", "with"]),
    ))
    .void()
    .parse_next(input)?;
    Ok(filters::parse_spell_filter_with_grammar_entrypoint_lexed(
        trim_lexed_commas(spell_filter_tokens),
    ))
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
        assert!(!spec.additional);
        assert!(spec.cast_this_way_filter.is_none());

        let tokens = lex_line(
            "You may play lands and cast Mutant, Ninja, or Turtle spells from the top of your library. If you cast a creature spell this way, that creature enters with an additional +1/+1 counter on it.",
            0,
        )
        .unwrap();
        let spec = parse_play_permission_enter_counter_tokens(&tokens).unwrap();
        assert_eq!(spec.counter_type, CounterType::PlusOnePlusOne);
        assert!(!spec.permission_tokens.is_empty());
        assert!(spec.additional);
        assert_eq!(
            spec.cast_this_way_filter
                .as_ref()
                .map(|filter| filter.card_types.as_slice()),
            Some([crate::types::CardType::Creature].as_slice())
        );
    }
}
