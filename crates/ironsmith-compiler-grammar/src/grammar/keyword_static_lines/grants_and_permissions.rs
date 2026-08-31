use winnow::combinator::{alt, opt, peek, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use crate::mana::{ManaCost, ManaSymbol};

use super::super::super::lexer::{LexStream, OwnedLexToken, trim_lexed_commas};
use super::super::{leaf, primitives};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ManaValueGrantSpec<'a> {
    pub subject_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LifeManaValueGrantSpec<'a> {
    pub subject_tokens: &'a [OwnedLexToken],
    pub usage_limit: LifeManaValueGrantUsageLimit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifeManaValueGrantUsageLimit {
    OnceDuringEachOfYourTurns,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixedManaCostGrantSpec<'a> {
    pub subject_tokens: &'a [OwnedLexToken],
    pub mana_cost: ManaCost,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandSizePlayerKind {
    You,
    Opponent,
    Any,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandSizeOperation {
    Reduce(u32),
    Increase(u32),
    Set(u32),
    SevenMinusGraveyardCardTypes,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HandSizeLineSpec<'a> {
    pub condition_tokens: Option<&'a [OwnedLexToken]>,
    pub player: HandSizePlayerKind,
    pub operation: HandSizeOperation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManaSpendPlayerKind {
    You,
    Any,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManaSpendPermissionShape<'a> {
    SymbolAsAnyColorOtherAsColorless {
        symbol: ManaSymbol,
    },
    AnyTypeToCast {
        filter_tokens: &'a [OwnedLexToken],
    },
    AnyColor {
        player: ManaSpendPlayerKind,
        activation_filter_tokens: Option<&'a [OwnedLexToken]>,
        source_activation_only: bool,
    },
}

pub fn parse_mana_value_grant_tokens(tokens: &[OwnedLexToken]) -> Option<ManaValueGrantSpec<'_>> {
    crate::grammar::primitives::probe_all(tokens, parse_mana_value_grant_lexed, "mana-value grant")
}

pub fn parse_life_mana_value_grant_tokens(
    tokens: &[OwnedLexToken],
) -> Option<LifeManaValueGrantSpec<'_>> {
    crate::grammar::primitives::probe_all(
        tokens,
        parse_life_mana_value_grant_lexed,
        "life mana-value grant",
    )
}

pub fn parse_fixed_mana_cost_grant_tokens(
    tokens: &[OwnedLexToken],
) -> Option<FixedManaCostGrantSpec<'_>> {
    crate::grammar::primitives::probe_all(
        tokens,
        parse_fixed_mana_cost_grant_lexed,
        "fixed mana-cost grant",
    )
}

pub fn parse_cascade_land_drop_tokens(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_all(tokens, parse_cascade_land_drop_lexed, "cascade land drop").is_ok()
}

pub fn parse_hand_size_line_tokens(tokens: &[OwnedLexToken]) -> Option<HandSizeLineSpec<'_>> {
    crate::grammar::primitives::probe_all(
        tokens,
        parse_hand_size_line_lexed,
        "maximum hand-size line",
    )
}

pub fn parse_mana_spend_permission_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ManaSpendPermissionShape<'_>> {
    crate::grammar::primitives::probe_all(
        tokens,
        alt((
            parse_symbol_mana_spend_lexed,
            parse_any_type_cast_mana_spend_lexed,
            parse_any_color_mana_spend_lexed,
        )),
        "mana spend permission",
    )
}

fn parse_mana_value_grant_lexed<'a>(input: &mut LexStream<'a>) -> WResult<ManaValueGrantSpec<'a>> {
    primitives::phrase(&["you", "may", "pay"]).parse_next(input)?;
    leaf::parse_leaf_mana_group_token
        .verify(|symbols: &Vec<ManaSymbol>| {
            symbols.len() == 1 && symbols.first() == Some(&ManaSymbol::X)
        })
        .parse_next(input)?;
    primitives::phrase(&["rather", "than", "pay"]).parse_next(input)?;
    opt(primitives::kw("the")).parse_next(input)?;
    primitives::phrase(&["mana", "cost", "for"]).parse_next(input)?;
    let subject_tokens = repeat_till::<_, _, (), _, _, _, _>(
        1..,
        any.void(),
        peek((
            opt(primitives::comma()),
            primitives::phrase(&["where", "x", "is", "that"]),
        )),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&["where", "x", "is", "that"]).parse_next(input)?;
    alt((primitives::kw("spell's"), primitives::kw("spells"))).parse_next(input)?;
    primitives::phrase(&["mana", "value"]).parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    validate_spell_subject(subject_tokens)?;
    Ok(ManaValueGrantSpec {
        subject_tokens: trim_lexed_commas(subject_tokens),
    })
}

fn parse_life_mana_value_grant_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<LifeManaValueGrantSpec<'a>> {
    primitives::phrase(&["once", "during", "each", "of", "your", "turns"]).parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&["you", "may", "cast"]).parse_next(input)?;
    let subject_tokens =
        repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(primitives::kw("by")))
            .map(|((), _)| ())
            .take()
            .parse_next(input)?;
    primitives::kw("by").parse_next(input)?;
    alt((primitives::kw("pay"), primitives::kw("paying"))).parse_next(input)?;
    primitives::phrase(&[
        "life", "equal", "to", "its", "mana", "value", "rather", "than",
    ])
    .parse_next(input)?;
    alt((primitives::kw("pay"), primitives::kw("paying"))).parse_next(input)?;
    primitives::phrase(&["its", "mana", "cost"]).parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    validate_spell_subject(subject_tokens)?;
    Ok(LifeManaValueGrantSpec {
        subject_tokens: trim_lexed_commas(subject_tokens),
        usage_limit: LifeManaValueGrantUsageLimit::OnceDuringEachOfYourTurns,
    })
}

fn parse_fixed_mana_cost_grant_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<FixedManaCostGrantSpec<'a>> {
    primitives::phrase(&["you", "may", "pay"]).parse_next(input)?;
    let mana_cost = leaf::parse_leaf_fixed_mana_cost_prefix_lexed
        .map(|prefix| prefix.cost)
        .parse_next(input)?;
    primitives::phrase(&["rather", "than", "pay"]).parse_next(input)?;
    opt(primitives::kw("the")).parse_next(input)?;
    primitives::phrase(&["mana", "cost", "for"]).parse_next(input)?;
    let subject_tokens = take_nonempty_sentence_body(input)?;
    validate_spell_subject(subject_tokens)?;
    Ok(FixedManaCostGrantSpec {
        subject_tokens: trim_lexed_commas(subject_tokens),
        mana_cost,
    })
}

fn parse_cascade_land_drop_lexed<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["as", "you", "cascade"]).parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&[
        "you",
        "may",
        "put",
        "a",
        "land",
        "card",
        "from",
        "among",
        "the",
        "exiled",
        "cards",
        "onto",
        "the",
        "battlefield",
        "tapped",
    ])
    .parse_next(input)?;
    primitives::sentence_end().parse_next(input)
}

fn parse_hand_size_line_lexed<'a>(input: &mut LexStream<'a>) -> WResult<HandSizeLineSpec<'a>> {
    let condition_tokens = if peek(primitives::phrase(&["as", "long", "as"]))
        .parse_next(input)
        .is_ok()
    {
        primitives::phrase(&["as", "long", "as"]).parse_next(input)?;
        let condition_tokens = repeat_till::<_, _, (), _, _, _, _>(
            1..,
            any.void(),
            peek((opt(primitives::comma()), parse_hand_size_subject_and_head)),
        )
        .map(|((), _)| ())
        .take()
        .parse_next(input)?;
        opt(primitives::comma()).parse_next(input)?;
        Some(trim_lexed_commas(condition_tokens))
    } else {
        None
    };
    let player = parse_hand_size_subject_and_head(input)?;
    let operation = alt((
        (
            primitives::phrase(&["is", "reduced", "by"]),
            leaf::parse_leaf_number_prefix_lexed,
        )
            .map(|(_, amount)| HandSizeOperation::Reduce(amount)),
        (
            primitives::phrase(&["is", "increased", "by"]),
            leaf::parse_leaf_number_prefix_lexed,
        )
            .map(|(_, amount)| HandSizeOperation::Increase(amount)),
        (
            primitives::phrase(&[
                "is", "equal", "to", "seven", "minus", "the", "number", "of", "those", "card",
            ]),
            alt((primitives::kw("type"), primitives::kw("types"))),
        )
            .value(HandSizeOperation::SevenMinusGraveyardCardTypes),
        (primitives::kw("is"), leaf::parse_leaf_number_prefix_lexed)
            .map(|(_, amount)| HandSizeOperation::Set(amount)),
    ))
    .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(HandSizeLineSpec {
        condition_tokens,
        player,
        operation,
    })
}

fn parse_hand_size_subject_and_head<'a>(input: &mut LexStream<'a>) -> WResult<HandSizePlayerKind> {
    let player = alt((
        alt((primitives::kw("your"), primitives::kw("you"))).value(HandSizePlayerKind::You),
        (
            opt(primitives::kw("each")),
            alt((
                primitives::kw("opponent"),
                primitives::kw("opponents"),
                primitives::kw("opponent's"),
            )),
            opt(primitives::kw("s")),
        )
            .value(HandSizePlayerKind::Opponent),
        (
            opt(primitives::kw("each")),
            alt((
                primitives::kw("player"),
                primitives::kw("players"),
                primitives::kw("player's"),
            )),
            opt(primitives::kw("s")),
        )
            .value(HandSizePlayerKind::Any),
    ))
    .parse_next(input)?;
    primitives::phrase(&["maximum", "hand", "size"]).parse_next(input)?;
    Ok(player)
}

fn parse_symbol_mana_spend_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<ManaSpendPermissionShape<'a>> {
    primitives::phrase(&["you", "may", "spend"]).parse_next(input)?;
    let symbol = alt((
        primitives::kw("white").value(ManaSymbol::White),
        primitives::kw("blue").value(ManaSymbol::Blue),
        primitives::kw("black").value(ManaSymbol::Black),
        primitives::kw("red").value(ManaSymbol::Red),
        primitives::kw("green").value(ManaSymbol::Green),
    ))
    .parse_next(input)?;
    primitives::phrase(&[
        "mana", "as", "though", "it", "were", "mana", "of", "any", "color",
    ])
    .parse_next(input)?;
    primitives::end_of_sentence().parse_next(input)?;
    primitives::phrase(&[
        "you",
        "may",
        "spend",
        "other",
        "mana",
        "only",
        "as",
        "though",
        "it",
        "were",
        "colorless",
        "mana",
    ])
    .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(ManaSpendPermissionShape::SymbolAsAnyColorOtherAsColorless { symbol })
}

fn parse_any_type_cast_mana_spend_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<ManaSpendPermissionShape<'a>> {
    primitives::kw("you").parse_next(input)?;
    alt((primitives::kw("can"), primitives::kw("may"))).parse_next(input)?;
    primitives::phrase(&["spend", "mana", "of", "any", "type", "to", "cast"]).parse_next(input)?;
    let filter_tokens = take_nonempty_sentence_body(input)?;
    Ok(ManaSpendPermissionShape::AnyTypeToCast {
        filter_tokens: trim_lexed_commas(filter_tokens),
    })
}

fn parse_any_color_mana_spend_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<ManaSpendPermissionShape<'a>> {
    let player = alt((
        primitives::kw("you").value(ManaSpendPlayerKind::You),
        primitives::kw("players").value(ManaSpendPlayerKind::Any),
    ))
    .parse_next(input)?;
    primitives::phrase(&[
        "may", "spend", "mana", "as", "though", "it", "were", "mana", "of", "any", "color",
    ])
    .parse_next(input)?;
    if peek(primitives::sentence_end()).parse_next(input).is_ok() {
        primitives::sentence_end().parse_next(input)?;
        return Ok(ManaSpendPermissionShape::AnyColor {
            player,
            activation_filter_tokens: None,
            source_activation_only: false,
        });
    }
    if peek(primitives::phrase(&["to", "pay"]))
        .parse_next(input)
        .is_ok()
    {
        primitives::phrase(&["to", "pay"]).parse_next(input)?;
        opt(primitives::kw("the")).parse_next(input)?;
        primitives::phrase(&["activation", "costs", "of"]).parse_next(input)?;
        let ability_tokens = take_nonempty_sentence_body(input)?;
        if primitives::find_prefix(ability_tokens, || {
            alt((primitives::kw("ability"), primitives::kw("abilities"))).void()
        })
        .is_none()
        {
            return Err(primitives::backtrack_err(
                "mana spend permission",
                "activation ability subject",
            ));
        }
        return Ok(ManaSpendPermissionShape::AnyColor {
            player,
            activation_filter_tokens: None,
            source_activation_only: true,
        });
    }
    primitives::phrase(&["to", "activate", "abilities", "of"]).parse_next(input)?;
    let filter_tokens = take_nonempty_sentence_body(input)?;
    Ok(ManaSpendPermissionShape::AnyColor {
        player,
        activation_filter_tokens: Some(trim_lexed_commas(filter_tokens)),
        source_activation_only: false,
    })
}

fn validate_spell_subject(tokens: &[OwnedLexToken]) -> WResult<()> {
    if primitives::find_prefix(tokens, || {
        alt((primitives::kw("spell"), primitives::kw("spells"))).void()
    })
    .is_some()
    {
        Ok(())
    } else {
        Err(primitives::backtrack_err("grant subject", "spell subject"))
    }
}

fn take_nonempty_sentence_body<'a>(input: &mut LexStream<'a>) -> WResult<&'a [OwnedLexToken]> {
    let tokens =
        repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(primitives::sentence_end()))
            .map(|((), _)| ())
            .take()
            .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::super::super::super::lexer::lex_line;
    use super::*;

    #[test]
    fn parses_grant_and_hand_size_shapes() {
        let tokens = lex_line(
            "You may pay {2}{U} rather than pay the mana cost for Wizard spells.",
            0,
        )
        .unwrap();
        assert!(parse_fixed_mana_cost_grant_tokens(&tokens).is_some());
        let tokens = lex_line("Your maximum hand size is reduced by two.", 0).unwrap();
        assert_eq!(
            parse_hand_size_line_tokens(&tokens).map(|spec| spec.operation),
            Some(HandSizeOperation::Reduce(2))
        );
        let tokens = lex_line(
            "As you cascade, you may put a land card from among the exiled cards onto the battlefield tapped.",
            0,
        )
        .unwrap();
        assert!(parse_cascade_land_drop_tokens(&tokens));
    }

    #[test]
    fn parses_mana_spend_shapes() {
        for line in [
            "You may spend mana as though it were mana of any color to activate abilities of artifacts you control.",
            "You may spend mana as though it were mana of any color to pay the activation costs of Manascape Refractor's abilities.",
        ] {
            let tokens = lex_line(line, 0).unwrap();
            assert!(
                parse_mana_spend_permission_tokens(&tokens).is_some(),
                "expected typed mana-spend shape for {line}"
            );
        }

        let tokens = lex_line(
            "You may spend white mana as though it were mana of any color. You may spend other mana only as though it were colorless mana.",
            0,
        )
        .unwrap();
        assert_eq!(
            parse_mana_spend_permission_tokens(&tokens),
            Some(ManaSpendPermissionShape::SymbolAsAnyColorOtherAsColorless {
                symbol: ManaSymbol::White,
            })
        );
    }

    #[test]
    fn parses_braced_x_mana_value_grant_shape() {
        let tokens = lex_line(
            "You may pay {X} rather than pay the mana cost for Samurai spells you cast, where X is that spell's mana value.",
            0,
        )
        .unwrap();
        let spec = parse_mana_value_grant_tokens(&tokens)
            .expect("braced X should select the derived mana-value grant grammar");
        assert_eq!(
            super::super::super::super::lexer::TokenWordView::new(spec.subject_tokens).word_refs(),
            ["samurai", "spells", "you", "cast"]
        );
    }

    #[test]
    fn parses_once_each_turn_life_equal_mana_value_grant_shape() {
        let tokens = lex_line(
            "Once during each of your turns, you may cast an enchantment spell by paying life equal to its mana value rather than paying its mana cost.",
            0,
        )
        .unwrap();
        let spec = parse_life_mana_value_grant_tokens(&tokens)
            .expect("typed life-equal-mana-value grant should accept the clause comma");
        assert_eq!(
            spec.usage_limit,
            LifeManaValueGrantUsageLimit::OnceDuringEachOfYourTurns
        );
        assert_eq!(
            super::super::super::super::lexer::TokenWordView::new(spec.subject_tokens).word_refs(),
            ["an", "enchantment", "spell"]
        );
    }
}
