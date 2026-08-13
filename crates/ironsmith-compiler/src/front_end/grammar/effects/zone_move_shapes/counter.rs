use winnow::combinator::{alt, opt, repeat};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use crate::effect::Value;
use crate::mana::ManaSymbol;
use crate::grammar::{leaf, primitives, values};
use crate::front_end::lexer::{LexStream, LexedClause, OwnedLexToken};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CounterClauseShapeError {
    MissingPays,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum CounterPaymentTailShape<'a> {
    None,
    Life(Value),
    Other {
        tokens: &'a [OwnedLexToken],
        same_name_graveyard: bool,
        for_each: bool,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CounterUnlessShape<'a> {
    pub(crate) target_tokens: &'a [OwnedLexToken],
    pub(crate) normalized_payment_tokens: Vec<OwnedLexToken>,
    pub(crate) payment_tokens: &'a [OwnedLexToken],
    pub(crate) mana: Vec<ManaSymbol>,
    pub(crate) tail: CounterPaymentTailShape<'a>,
    pub(crate) where_tokens: Option<&'a [OwnedLexToken]>,
    pub(crate) has_x_mana_payment: bool,
    pub(crate) has_dynamic_payment_tail: bool,
    pub(crate) starts_with_mana_word: bool,
    pub(crate) twice_x_surface: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum CounterClauseShape<'a> {
    SecondSpellThisTurn { target_tokens: Vec<OwnedLexToken> },
    MalformedConditional,
    Unless(CounterUnlessShape<'a>),
    Plain { target_tokens: &'a [OwnedLexToken] },
}

fn trimmed(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    LexedClause::new(tokens).trimmed().tokens()
}

fn punctuation<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::comma(),
        primitives::period(),
        primitives::semicolon(),
    ))
    .void()
    .parse_next(input)
}

fn semantic_kw<'a>(
    expected: &'static str,
) -> impl Parser<LexStream<'a>, (), winnow::error::ErrMode<winnow::error::ContextError>> {
    (
        repeat::<_, _, (), _, _>(0.., punctuation),
        any.verify(move |token: &&OwnedLexToken| {
            token.is_word(expected)
                || matches!(token.parser_word_pieces(), [piece] if piece.text == expected)
        }),
    )
        .void()
}

fn semantic_phrase<'a>(
    expected: &'static [&'static str],
) -> impl Parser<LexStream<'a>, (), winnow::error::ErrMode<winnow::error::ContextError>> {
    move |input: &mut LexStream<'a>| {
        for word in expected {
            semantic_kw(word).parse_next(input)?;
        }
        Ok(())
    }
}

fn possessive_that<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    any.verify(|token: &&OwnedLexToken| {
        token.is_word("thats")
            || matches!(
                token.parser_word_pieces(),
                [piece] if piece.text == "thats"
            )
            || matches!(
                token.parser_word_pieces(),
                [first, second] if first.text == "that" && second.text == "s"
            )
    })
    .void()
    .parse_next(input)
}

fn second_spell_this_turn<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    (
        primitives::phrase(&["counter", "target", "spell"]),
        possessive_that,
        opt(primitives::kw("the")),
        primitives::phrase(&["second", "spell", "cast", "this", "turn"]),
        primitives::sentence_end(),
    )
        .void()
        .parse_next(input)
}

fn contains_if(tokens: &[OwnedLexToken]) -> bool {
    primitives::find_prefix(tokens, || primitives::kw("if")).is_some()
}

fn contains_x_mana(tokens: &[OwnedLexToken]) -> bool {
    let mut remaining = tokens;
    while let Some((_, pip, rest)) =
        primitives::find_prefix(remaining, || leaf::parse_leaf_surface_mana_pip_lexed)
    {
        if pip
            .into_pip()
            .iter()
            .any(|symbol| matches!(symbol, ManaSymbol::X))
        {
            return true;
        }
        if rest.len() >= remaining.len() {
            break;
        }
        remaining = rest;
    }
    false
}

fn dynamic_marker<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::kw("and"),
        primitives::kw("or"),
        primitives::kw("where"),
        primitives::kw("plus"),
        primitives::kw("additional"),
        primitives::kw("equal"),
        primitives::kw("equals"),
    ))
    .void()
    .parse_next(input)
}

fn has_dynamic_marker(tokens: &[OwnedLexToken]) -> bool {
    primitives::find_prefix(tokens, || dynamic_marker).is_some()
        || primitives::find_prefix(tokens, || primitives::phrase(&["for", "each"])).is_some()
}

fn parse_mana_prefix(tokens: &[OwnedLexToken]) -> (Vec<ManaSymbol>, &[OwnedLexToken]) {
    let Some((prefix, rest)) =
        primitives::parse_prefix(tokens, leaf::parse_leaf_mana_cost_prefix_lexed)
    else {
        return (Vec::new(), tokens);
    };
    let mana = prefix
        .cost
        .pips()
        .iter()
        .flatten()
        .copied()
        .collect::<Vec<_>>();
    (mana, trimmed(rest))
}

fn same_name_graveyard(tokens: &[OwnedLexToken]) -> bool {
    let Some((_, (), after_graveyard)) = primitives::find_prefix(tokens, || {
        alt((primitives::kw("graveyard"), primitives::kw("graveyards"))).void()
    }) else {
        return false;
    };
    primitives::find_prefix(after_graveyard, || {
        semantic_phrase(&["same", "name", "as", "the", "spell"])
    })
    .is_some()
        || primitives::find_prefix(after_graveyard, || {
            semantic_phrase(&["same", "name", "as", "that", "spell"])
        })
        .is_some()
}

fn twice_x_surface(tokens: &[OwnedLexToken]) -> bool {
    let Some(((), rest)) = primitives::parse_prefix(tokens, primitives::kw("twice").void()) else {
        return false;
    };
    contains_x_mana(rest)
        && primitives::parse_all(
            rest,
            (
                repeat::<_, _, Vec<_>, _, _>(1.., leaf::parse_leaf_surface_mana_pip_lexed),
                primitives::sentence_end(),
            )
                .void(),
            "twice X payment",
        )
        .is_ok()
}

fn parse_payment_tail(tokens: &[OwnedLexToken]) -> CounterPaymentTailShape<'_> {
    let tokens = trimmed(tokens);
    if tokens.is_empty() {
        return CounterPaymentTailShape::None;
    }
    if let Some(((), life_tokens)) = primitives::parse_prefix(tokens, primitives::kw("and").void())
        && let Some((amount, used)) = values::parse_value_prefix_lexed(life_tokens)
        && let Some(((), rest)) = primitives::parse_prefix(
            life_tokens.get(used..).unwrap_or_default(),
            primitives::kw("life").void(),
        )
        && trimmed(rest).is_empty()
    {
        return CounterPaymentTailShape::Life(amount);
    }
    let where_x = primitives::parse_prefix(
        tokens,
        alt((
            primitives::phrase(&["where", "x", "is"]),
            primitives::phrase(&["where", "x", "equals"]),
        ))
        .void(),
    )
    .is_some();
    CounterPaymentTailShape::Other {
        tokens,
        same_name_graveyard: where_x && same_name_graveyard(tokens),
        for_each: primitives::parse_prefix(tokens, primitives::phrase(&["for", "each"]).void())
            .is_some(),
    }
}

fn parse_unless_shape<'a>(
    target_tokens: &'a [OwnedLexToken],
    unless_tokens: &'a [OwnedLexToken],
) -> Result<CounterUnlessShape<'a>, CounterClauseShapeError> {
    let (pays_idx, (), after_pays) =
        primitives::find_prefix(unless_tokens, || primitives::kw("pays").void())
            .ok_or(CounterClauseShapeError::MissingPays)?;
    let mut normalized_payment_tokens = unless_tokens[pays_idx..].to_vec();
    if let Some(first) = normalized_payment_tokens.first_mut() {
        first.replace_word("pay");
    }
    let payment_tokens = trimmed(after_pays);
    let has_x_mana_payment = contains_x_mana(payment_tokens);
    let has_dynamic_payment_tail =
        has_dynamic_marker(&normalized_payment_tokens) || has_x_mana_payment;
    let (mana, trailing_tokens) = parse_mana_prefix(payment_tokens);
    let tail = if mana.is_empty() {
        CounterPaymentTailShape::None
    } else {
        parse_payment_tail(trailing_tokens)
    };
    let where_tokens = primitives::find_prefix(unless_tokens, || primitives::kw("where"))
        .map(|(idx, _, _)| trimmed(&unless_tokens[idx..]));
    Ok(CounterUnlessShape {
        target_tokens: trimmed(target_tokens),
        normalized_payment_tokens,
        payment_tokens,
        mana,
        tail,
        where_tokens,
        has_x_mana_payment,
        has_dynamic_payment_tail,
        starts_with_mana_word: primitives::parse_prefix(payment_tokens, primitives::kw("mana"))
            .is_some(),
        twice_x_surface: twice_x_surface(payment_tokens),
    })
}

pub(crate) fn parse_counter_clause_shape(
    tokens: &[OwnedLexToken],
) -> Result<CounterClauseShape<'_>, CounterClauseShapeError> {
    let tokens = trimmed(tokens);
    if primitives::parse_all(
        tokens,
        second_spell_this_turn,
        "second spell counter clause",
    )
    .is_ok()
    {
        let (_, after_counter) =
            primitives::parse_prefix(tokens, primitives::kw("counter")).expect("parsed counter");
        let ((), after_target) = primitives::parse_prefix(
            after_counter,
            primitives::phrase(&["target", "spell"]).void(),
        )
        .expect("parsed target spell");
        let consumed = after_counter.len().saturating_sub(after_target.len());
        return Ok(CounterClauseShape::SecondSpellThisTurn {
            target_tokens: after_counter[..consumed].to_vec(),
        });
    }
    if contains_if(tokens) {
        return Ok(CounterClauseShape::MalformedConditional);
    }
    if let Some((target_tokens, unless_tokens)) =
        primitives::split_lexed_once_on_separator(tokens, || primitives::kw("unless").void())
    {
        return parse_unless_shape(target_tokens, unless_tokens).map(CounterClauseShape::Unless);
    }
    Ok(CounterClauseShape::Plain {
        target_tokens: tokens,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::lex_line;

    fn tokens(text: &str) -> Vec<OwnedLexToken> {
        lex_line(text, 0).unwrap()
    }

    #[test]
    fn parses_counter_clause_shapes() {
        assert!(matches!(
            parse_counter_clause_shape(&tokens(
                "counter target spell that's the second spell cast this turn"
            )),
            Ok(CounterClauseShape::SecondSpellThisTurn { .. })
        ));
        let unless_tokens = tokens("target spell unless its controller pays {2} and 2 life");
        let shape = parse_counter_clause_shape(&unless_tokens).unwrap();
        let CounterClauseShape::Unless(shape) = shape else {
            panic!("expected unless shape")
        };
        assert_eq!(shape.mana, vec![ManaSymbol::Generic(2)]);
        assert!(matches!(
            shape.tail,
            CounterPaymentTailShape::Life(Value::Fixed(2))
        ));
    }
}
