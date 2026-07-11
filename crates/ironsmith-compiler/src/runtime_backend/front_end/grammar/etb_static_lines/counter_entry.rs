use winnow::combinator::{alt, eof, opt, peek, repeat, repeat_till, separated};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::{any, rest};

use crate::effect::Value;
use crate::object::CounterType;

use super::super::super::lexer::{
    LexStream, OwnedLexToken, TokenKind, TokenWordView, trim_lexed_commas,
};
use super::super::{filters, leaf, primitives};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EtbTriggerIntro {
    If,
    When,
    Whenever,
    As,
    At,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EtbSourceReference {
    It,
    Its,
    This,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct EntersTappedWithCountersClause<'a> {
    pub(crate) subject_tokens: &'a [OwnedLexToken],
    pub(crate) action_tokens: &'a [OwnedLexToken],
    pub(crate) entry_modifier_tokens: &'a [OwnedLexToken],
    pub(crate) with_tokens: &'a [OwnedLexToken],
    pub(crate) counter_clause_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct EntersWithCountersClause<'a> {
    pub(crate) subject_tokens: &'a [OwnedLexToken],
    pub(crate) action_tokens: &'a [OwnedLexToken],
    pub(crate) counter_clause_tokens: &'a [OwnedLexToken],
    pub(crate) escaped: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EntersWithCounterConditionTailKind {
    If,
    Unless,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct EntersWithCounterConditionTail<'a> {
    pub(crate) kind: EntersWithCounterConditionTailKind,
    pub(crate) condition_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct EntersWithCounterChoice {
    pub(crate) counter_types: Vec<CounterType>,
    pub(crate) count: Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EntersWithAddedAbilitiesTail<'a> {
    CanAttackAsThoughNoDefender,
    AbilityTokens(&'a [OwnedLexToken]),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EntersWithCounterKnownForEachKind {
    CreaturesDiedThisTurn,
    ColorsOfManaSpent,
    ControlledCreaturesDiedThisTurn,
    KickCount,
    LoyaltyCountersOnPlaneswalkersYouControl,
    MagicGamesLost,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EntersWithCounterPlusTail<'a> {
    Unsupported,
    ForEach(&'a [OwnedLexToken]),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EntersWithCounterConditionShape<'a> {
    AttackedThisTurn,
    SourceWasCast,
    ThisSpellWasKicked,
    ThisSpellEscaped,
    CreatureDiedThisTurn,
    OpponentLostLifeThisTurn,
    PermanentLeftUnderYourControl,
    NotCastOrNoManaSpent,
    XValueAtLeast(&'a [OwnedLexToken]),
    YouCastSpellsThisTurn(&'a [OwnedLexToken]),
    ColorsOfManaSpent(&'a [OwnedLexToken]),
}

pub(crate) fn parse_etb_trigger_intro_prefix_tokens(
    tokens: &[OwnedLexToken],
) -> Option<EtbTriggerIntro> {
    primitives::parse_prefix(tokens, parse_etb_trigger_intro_lexed).map(|(intro, _)| intro)
}

pub(crate) fn parse_etb_source_reference_tokens(
    tokens: &[OwnedLexToken],
) -> Option<EtbSourceReference> {
    primitives::parse_all(
        tokens,
        parse_etb_source_reference_lexed,
        "etb-source-reference",
    )
    .ok()
}

pub(crate) fn parse_enters_with_counter_condition_tail_tokens(
    tokens: &[OwnedLexToken],
) -> Option<EntersWithCounterConditionTail<'_>> {
    primitives::parse_all(
        tokens,
        parse_enters_with_counter_condition_tail_lexed,
        "enters-with-counter-condition-tail",
    )
    .ok()
}

pub(crate) fn parse_enters_with_counters_clause_tokens(
    tokens: &[OwnedLexToken],
) -> Option<EntersWithCountersClause<'_>> {
    primitives::parse_all(
        tokens,
        parse_enters_with_counters_clause_lexed,
        "enters-with-counters-clause",
    )
    .ok()
}

pub(crate) fn parse_enters_tapped_with_counters_clause_tokens(
    tokens: &[OwnedLexToken],
) -> Option<EntersTappedWithCountersClause<'_>> {
    primitives::parse_all(
        tokens,
        parse_enters_tapped_with_counters_clause_lexed,
        "enters-tapped-with-counters-clause",
    )
    .ok()
}

pub(crate) fn parse_enters_with_counter_choice_tokens(
    tokens: &[OwnedLexToken],
) -> Option<EntersWithCounterChoice> {
    primitives::parse_all(
        tokens,
        parse_enters_with_counter_choice_lexed,
        "enters-with-counter-choice",
    )
    .ok()
}

pub(crate) fn parse_enters_with_added_abilities_tail_tokens(
    tokens: &[OwnedLexToken],
) -> Option<EntersWithAddedAbilitiesTail<'_>> {
    primitives::parse_all(
        tokens,
        parse_enters_with_added_abilities_tail_lexed,
        "enters-with-added-abilities-tail",
    )
    .ok()
}

pub(crate) fn parse_enters_with_counter_known_for_each_tail_tokens(
    tokens: &[OwnedLexToken],
) -> Option<EntersWithCounterKnownForEachKind> {
    primitives::parse_all(
        tokens,
        parse_enters_with_counter_known_for_each_tail_lexed,
        "enters-with-counter-known-for-each-tail",
    )
    .ok()
}

pub(crate) fn parse_enters_with_counter_for_each_payload_tokens(
    tokens: &[OwnedLexToken],
) -> Option<&[OwnedLexToken]> {
    primitives::parse_all(
        tokens,
        parse_enters_with_counter_for_each_payload_lexed,
        "enters-with-counter-for-each-tail",
    )
    .ok()
}

pub(crate) fn parse_enters_with_counter_equal_to_body_tokens(
    tokens: &[OwnedLexToken],
) -> Option<&[OwnedLexToken]> {
    primitives::parse_all(
        tokens,
        parse_enters_with_counter_equal_to_body_lexed,
        "enters-with-counter-equal-to-tail",
    )
    .ok()
}

pub(crate) fn parse_enters_with_counter_plus_tail_tokens(
    tokens: &[OwnedLexToken],
) -> Option<EntersWithCounterPlusTail<'_>> {
    primitives::parse_all(
        tokens,
        parse_enters_with_counter_plus_tail_lexed,
        "enters-with-counter-plus-tail",
    )
    .ok()
}

pub(crate) fn parse_enters_with_counter_condition_shape_tokens(
    tokens: &[OwnedLexToken],
) -> Option<EntersWithCounterConditionShape<'_>> {
    primitives::parse_all(
        tokens,
        parse_enters_with_counter_condition_shape_lexed,
        "enters-with-counter-condition",
    )
    .ok()
}

fn parse_etb_trigger_intro_lexed<'a>(input: &mut LexStream<'a>) -> WResult<EtbTriggerIntro> {
    alt((
        primitives::kw("if").value(EtbTriggerIntro::If),
        primitives::kw("when").value(EtbTriggerIntro::When),
        primitives::kw("whenever").value(EtbTriggerIntro::Whenever),
        primitives::kw("as").value(EtbTriggerIntro::As),
        primitives::kw("at").value(EtbTriggerIntro::At),
    ))
    .parse_next(input)
}

fn parse_etb_source_reference_lexed<'a>(input: &mut LexStream<'a>) -> WResult<EtbSourceReference> {
    let tokens = rest.parse_next(input)?;
    let view = TokenWordView::new(tokens);
    let words = view.word_refs();
    if primitives::parse_word_sequence_complete(&words, &["it"]).is_some() {
        Ok(EtbSourceReference::It)
    } else if primitives::parse_word_sequence_complete(&words, &["its"]).is_some() {
        Ok(EtbSourceReference::Its)
    } else if leaf::parse_leaf_this_source_reference_words(&words).is_some() {
        Ok(EtbSourceReference::This)
    } else {
        Err(primitives::backtrack_err(
            "ETB source reference",
            "it, its, or this source",
        ))
    }
}

fn parse_enters_with_counter_condition_tail_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<EntersWithCounterConditionTail<'a>> {
    let kind = alt((
        primitives::kw("if").value(EntersWithCounterConditionTailKind::If),
        primitives::kw("unless").value(EntersWithCounterConditionTailKind::Unless),
    ))
    .parse_next(input)?;
    let condition_tokens = repeat_till(1.., any.void(), peek(primitives::sentence_end()))
        .map(|((), ())| ())
        .take()
        .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(EntersWithCounterConditionTail {
        kind,
        condition_tokens: trim_lexed_commas(condition_tokens),
    })
}

fn parse_enters_with_counters_clause_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<EntersWithCountersClause<'a>> {
    let subject_tokens = repeat_till(1.., any.void(), peek(parse_enters_or_escapes))
        .map(|((), _)| ())
        .take()
        .parse_next(input)?;
    let (escaped, action_tokens) = parse_enters_or_escapes.with_taken().parse_next(input)?;
    repeat_till(0.., any.void(), peek(primitives::kw("with")))
        .map(|((), _)| ())
        .parse_next(input)?;
    primitives::kw("with").parse_next(input)?;
    let counter_clause_tokens = repeat_till(1.., any.void(), peek(primitives::sentence_end()))
        .map(|((), ())| ())
        .take()
        .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    if subject_tokens.iter().any(|token| {
        matches!(
            token.kind,
            TokenKind::Period | TokenKind::Colon | TokenKind::Semicolon
        )
    }) || parse_etb_source_reference_tokens(subject_tokens).is_none()
        || !tokens_contain_counter_word(counter_clause_tokens)
    {
        return Err(primitives::backtrack_err(
            "enters with counters",
            "source enters or escapes with counters",
        ));
    }
    Ok(EntersWithCountersClause {
        subject_tokens,
        action_tokens,
        counter_clause_tokens,
        escaped,
    })
}

fn parse_enters_tapped_with_counters_clause_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<EntersTappedWithCountersClause<'a>> {
    let subject_tokens = repeat_till(1.., any.void(), peek(parse_enter_or_enters))
        .map(|((), _)| ())
        .take()
        .parse_next(input)?;
    let (_, action_tokens) = parse_enter_or_enters.with_taken().parse_next(input)?;
    let entry_modifier_tokens = repeat_till(1.., any.void(), peek(primitives::kw("with")))
        .map(|((), _)| ())
        .take()
        .parse_next(input)?;
    let (_, with_tokens) = primitives::kw("with").with_taken().parse_next(input)?;
    let counter_clause_tokens = repeat_till(1.., any.void(), peek(primitives::sentence_end()))
        .map(|((), ())| ())
        .take()
        .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    if parse_etb_source_reference_tokens(subject_tokens).is_none()
        || !tokens_contain_word(entry_modifier_tokens, "tapped")
        || !tokens_contain_counter_word(counter_clause_tokens)
    {
        return Err(primitives::backtrack_err(
            "enters tapped with counters",
            "source enters tapped with counters",
        ));
    }
    Ok(EntersTappedWithCountersClause {
        subject_tokens,
        action_tokens,
        entry_modifier_tokens,
        with_tokens,
        counter_clause_tokens,
    })
}

fn parse_enters_with_counter_choice_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<EntersWithCounterChoice> {
    primitives::phrase(&["your", "choice", "of"]).parse_next(input)?;
    let counter_types = separated(
        2..,
        parse_counter_choice_type,
        primitives::comma_or_separator,
    )
    .parse_next(input)?;
    opt((primitives::kw("on"), primitives::kw("it"))).parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(EntersWithCounterChoice {
        counter_types,
        count: Value::Fixed(1),
    })
}

fn parse_counter_choice_type<'a>(input: &mut LexStream<'a>) -> WResult<CounterType> {
    opt(alt((primitives::kw("a"), primitives::kw("an")))).parse_next(input)?;
    let descriptor = repeat_till(
        1..,
        any.void(),
        alt((primitives::kw("counter"), primitives::kw("counters"))),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    filters::parse_counter_type_from_tokens(descriptor)
        .ok_or_else(|| primitives::backtrack_err("counter choice", "known counter type"))
}

fn parse_enters_with_added_abilities_tail_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<EntersWithAddedAbilitiesTail<'a>> {
    opt(primitives::kw("and")).parse_next(input)?;
    primitives::kw("with").parse_next(input)?;
    let quoted = opt(primitives::quote()).parse_next(input)?.is_some();
    let ability_tokens = if quoted {
        let ability_tokens = repeat_till(1.., any.void(), peek(primitives::quote()))
            .map(|((), _)| ())
            .take()
            .parse_next(input)?;
        primitives::quote().parse_next(input)?;
        primitives::sentence_end().parse_next(input)?;
        ability_tokens
    } else {
        let ability_tokens = repeat_till(1.., any.void(), peek(primitives::sentence_end()))
            .map(|((), ())| ())
            .take()
            .parse_next(input)?;
        primitives::sentence_end().parse_next(input)?;
        ability_tokens
    };
    if parse_can_attack_as_though_no_defender_tokens(ability_tokens) {
        Ok(EntersWithAddedAbilitiesTail::CanAttackAsThoughNoDefender)
    } else {
        Ok(EntersWithAddedAbilitiesTail::AbilityTokens(ability_tokens))
    }
}

fn parse_enters_with_counter_known_for_each_tail_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<EntersWithCounterKnownForEachKind> {
    alt((
        (
            primitives::phrase(&["for", "each"]),
            alt((primitives::kw("creature"), primitives::kw("creatures"))),
            primitives::phrase(&["that", "died", "this", "turn"]),
            primitives::sentence_end(),
        )
            .value(EntersWithCounterKnownForEachKind::CreaturesDiedThisTurn),
        (
            primitives::phrase(&["for", "each"]),
            alt((primitives::kw("color"), primitives::kw("colour"))),
            primitives::phrase(&["of", "mana", "spent", "to", "cast"]),
            alt((primitives::kw("it"), primitives::kw("this"))),
            primitives::sentence_end(),
        )
            .value(EntersWithCounterKnownForEachKind::ColorsOfManaSpent),
        (
            primitives::phrase(&["for", "each"]),
            alt((primitives::kw("creature"), primitives::kw("creatures"))),
            primitives::phrase(&["that", "died", "under", "your", "control", "this", "turn"]),
            primitives::sentence_end(),
        )
            .value(EntersWithCounterKnownForEachKind::ControlledCreaturesDiedThisTurn),
        (
            primitives::phrase(&["for", "each", "time"]),
            alt((
                primitives::kw("it").void(),
                primitives::phrase(&["this", "spell"]),
            )),
            primitives::phrase(&["was", "kicked"]),
            primitives::sentence_end(),
        )
            .value(EntersWithCounterKnownForEachKind::KickCount),
        (
            primitives::phrase(&["for", "each", "loyalty"]),
            alt((primitives::kw("counter"), primitives::kw("counters"))),
            primitives::kw("on"),
            alt((
                primitives::kw("planeswalker"),
                primitives::kw("planeswalkers"),
            )),
            primitives::phrase(&["you", "control"]),
            primitives::sentence_end(),
        )
            .value(EntersWithCounterKnownForEachKind::LoyaltyCountersOnPlaneswalkersYouControl),
        (
            primitives::phrase(&["for", "each"]),
            primitives::kw("magic"),
            alt((primitives::kw("game"), primitives::kw("games"))),
            primitives::phrase(&[
                "you",
                "have",
                "lost",
                "to",
                "one",
                "of",
                "your",
                "opponents",
                "since",
                "you",
                "last",
                "won",
                "a",
                "game",
                "against",
                "them",
            ]),
            primitives::sentence_end(),
        )
            .value(EntersWithCounterKnownForEachKind::MagicGamesLost),
    ))
    .parse_next(input)
}

fn parse_enters_with_counter_for_each_payload_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<&'a [OwnedLexToken]> {
    primitives::phrase(&["for", "each"]).parse_next(input)?;
    let payload = repeat_till(1.., any.void(), peek(primitives::sentence_end()))
        .map(|((), ())| ())
        .take()
        .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(payload)
}

fn parse_enters_with_counter_equal_to_body_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<&'a [OwnedLexToken]> {
    primitives::phrase(&["equal", "to"]).parse_next(input)?;
    let body = repeat_till(1.., any.void(), peek(primitives::sentence_end()))
        .map(|((), ())| ())
        .take()
        .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(body)
}

fn parse_enters_with_counter_plus_tail_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<EntersWithCounterPlusTail<'a>> {
    primitives::kw("plus").parse_next(input)?;
    let body = repeat_till(0.., any.void(), peek(primitives::sentence_end()))
        .map(|((), ())| ())
        .take()
        .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    let body = trim_lexed_commas(body);
    if body.is_empty() {
        return Ok(EntersWithCounterPlusTail::Unsupported);
    }
    let Some((first, _, _)) =
        primitives::find_prefix(body, || primitives::phrase(&["for", "each"]))
    else {
        return Ok(EntersWithCounterPlusTail::Unsupported);
    };
    Ok(EntersWithCounterPlusTail::ForEach(&body[first..]))
}

fn parse_enters_with_counter_condition_shape_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<EntersWithCounterConditionShape<'a>> {
    alt((
        parse_fixed_counter_condition_shape,
        parse_x_value_condition_shape,
        parse_cast_spells_condition_shape,
        parse_colors_mana_spent_condition_shape,
    ))
    .parse_next(input)
}

fn parse_fixed_counter_condition_shape<'a>(
    input: &mut LexStream<'a>,
) -> WResult<EntersWithCounterConditionShape<'a>> {
    alt((
        (
            alt((
                primitives::phrase(&["you", "attacked", "this", "turn"]),
                primitives::phrase(&["youve", "attacked", "this", "turn"]),
                primitives::phrase(&["you've", "attacked", "this", "turn"]),
                primitives::phrase(&["you", "ve", "attacked", "this", "turn"]),
            )),
            primitives::sentence_end(),
        )
            .value(EntersWithCounterConditionShape::AttackedThisTurn),
        (
            alt((
                primitives::phrase(&["you", "cast", "it"]),
                primitives::phrase(&["you", "cast", "this"]),
                primitives::phrase(&["you", "cast", "this", "spell"]),
            )),
            primitives::sentence_end(),
        )
            .value(EntersWithCounterConditionShape::SourceWasCast),
        (
            alt((
                primitives::phrase(&["this", "spell", "was", "kicked"]),
                primitives::phrase(&["this", "creature", "was", "kicked"]),
                primitives::phrase(&["this", "permanent", "was", "kicked"]),
                primitives::phrase(&["it", "was", "kicked"]),
            )),
            primitives::sentence_end(),
        )
            .value(EntersWithCounterConditionShape::ThisSpellWasKicked),
        (
            alt((
                primitives::phrase(&["this", "spell", "escaped"]),
                primitives::phrase(&["it", "escaped"]),
            )),
            primitives::sentence_end(),
        )
            .value(EntersWithCounterConditionShape::ThisSpellEscaped),
        (
            alt((
                primitives::phrase(&["a", "creature", "died", "this", "turn"]),
                primitives::phrase(&["one", "or", "more", "creatures", "died", "this", "turn"]),
            )),
            primitives::sentence_end(),
        )
            .value(EntersWithCounterConditionShape::CreatureDiedThisTurn),
        (
            alt((
                primitives::phrase(&["an", "opponent", "lost", "life", "this", "turn"]),
                primitives::phrase(&[
                    "one",
                    "or",
                    "more",
                    "opponents",
                    "lost",
                    "life",
                    "this",
                    "turn",
                ]),
            )),
            primitives::sentence_end(),
        )
            .value(EntersWithCounterConditionShape::OpponentLostLifeThisTurn),
        (
            alt((
                primitives::phrase(&[
                    "a",
                    "permanent",
                    "left",
                    "the",
                    "battlefield",
                    "under",
                    "your",
                    "control",
                    "this",
                    "turn",
                ]),
                primitives::phrase(&[
                    "one",
                    "or",
                    "more",
                    "permanents",
                    "left",
                    "the",
                    "battlefield",
                    "under",
                    "your",
                    "control",
                    "this",
                    "turn",
                ]),
            )),
            primitives::sentence_end(),
        )
            .value(EntersWithCounterConditionShape::PermanentLeftUnderYourControl),
        (
            alt((
                primitives::phrase(&[
                    "it", "wasnt", "cast", "or", "no", "mana", "was", "spent", "to", "cast", "it",
                ]),
                primitives::phrase(&[
                    "it", "wasn't", "cast", "or", "no", "mana", "was", "spent", "to", "cast", "it",
                ]),
            )),
            primitives::sentence_end(),
        )
            .value(EntersWithCounterConditionShape::NotCastOrNoManaSpent),
    ))
    .parse_next(input)
}

fn parse_x_value_condition_shape<'a>(
    input: &mut LexStream<'a>,
) -> WResult<EntersWithCounterConditionShape<'a>> {
    primitives::phrase(&["x", "is"]).parse_next(input)?;
    let amount = repeat_till(1.., any.void(), peek(primitives::sentence_end()))
        .map(|((), ())| ())
        .take()
        .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(EntersWithCounterConditionShape::XValueAtLeast(amount))
}

fn parse_cast_spells_condition_shape<'a>(
    input: &mut LexStream<'a>,
) -> WResult<EntersWithCounterConditionShape<'a>> {
    alt((
        primitives::phrase(&["youve", "cast"]),
        primitives::phrase(&["you've", "cast"]),
        primitives::phrase(&["you", "ve", "cast"]),
        primitives::phrase(&["you", "have", "cast"]),
        primitives::phrase(&["you", "cast"]),
    ))
    .parse_next(input)?;
    let amount = repeat_till(
        1..,
        any.void(),
        peek((
            alt((primitives::kw("spell"), primitives::kw("spells"))),
            primitives::phrase(&["this", "turn"]),
        )),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    alt((primitives::kw("spell"), primitives::kw("spells"))).parse_next(input)?;
    primitives::phrase(&["this", "turn"]).parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(EntersWithCounterConditionShape::YouCastSpellsThisTurn(
        amount,
    ))
}

fn parse_colors_mana_spent_condition_shape<'a>(
    input: &mut LexStream<'a>,
) -> WResult<EntersWithCounterConditionShape<'a>> {
    let amount = repeat_till(
        1..,
        any.void(),
        peek((
            alt((
                primitives::kw("color"),
                primitives::kw("colors"),
                primitives::kw("colour"),
                primitives::kw("colours"),
            )),
            primitives::phrase(&["of", "mana"]),
        )),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    alt((
        primitives::kw("color"),
        primitives::kw("colors"),
        primitives::kw("colour"),
        primitives::kw("colours"),
    ))
    .parse_next(input)?;
    primitives::phrase(&["of", "mana"]).parse_next(input)?;
    repeat_till(
        0..,
        any.void(),
        peek(primitives::phrase(&["spent", "to", "cast"])),
    )
    .map(|((), ())| ())
    .parse_next(input)?;
    primitives::phrase(&["spent", "to", "cast"]).parse_next(input)?;
    alt((
        primitives::kw("it").void(),
        primitives::kw("this").void(),
        primitives::phrase(&["this", "spell"]),
    ))
    .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(EntersWithCounterConditionShape::ColorsOfManaSpent(amount))
}

fn parse_enter_or_enters<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((primitives::kw("enter"), primitives::kw("enters")))
        .void()
        .parse_next(input)
}

fn parse_enters_or_escapes<'a>(input: &mut LexStream<'a>) -> WResult<bool> {
    alt((
        primitives::kw("enters").value(false),
        primitives::kw("escapes").value(true),
    ))
    .parse_next(input)
}

fn tokens_contain_counter_word(tokens: &[OwnedLexToken]) -> bool {
    tokens_contain_word(tokens, "counter") || tokens_contain_word(tokens, "counters")
}

fn tokens_contain_word(tokens: &[OwnedLexToken], expected: &'static str) -> bool {
    let mut input = LexStream::new(tokens);
    repeat_till(0.., any.void(), peek(primitives::kw(expected)))
        .map(|((), _)| ())
        .parse_next(&mut input)
        .is_ok()
}

fn parse_can_attack_as_though_no_defender_tokens(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_all(
        tokens,
        (
            primitives::phrase(&["this", "creature", "can", "attack", "as", "though", "it"]),
            alt((
                primitives::kw("didnt"),
                primitives::kw("didn't"),
                primitives::kw("doesnt"),
                primitives::kw("doesn't"),
            )),
            primitives::phrase(&["have", "defender"]),
            primitives::sentence_end(),
        ),
        "can-attack-as-though-no-defender",
    )
    .is_ok()
}

#[cfg(test)]
#[path = "counter_entry/tests.rs"]
mod tests;
