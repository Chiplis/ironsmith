use winnow::combinator::{alt, eof, opt, repeat};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use crate::cards::builders::CardTextError;
use crate::effect::Value;
use crate::mana::ManaCost;
use crate::target::PlayerFilter;

use super::super::super::lexer::{LexStream, OwnedLexToken, TokenKind, render_token_slice};
use super::super::leaf;
use super::super::primitives;
use super::ActivationCostSegmentCst;

pub fn parse_bare_symbol_segment_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ActivationCostSegmentCst> {
    crate::grammar::primitives::probe_all(
        tokens,
        parse_bare_symbol_segment_lexed,
        "activation-bare-symbol-segment",
    )
}

pub fn parse_pay_segment_tokens(
    tokens: &[OwnedLexToken],
) -> Result<ActivationCostSegmentCst, CardTextError> {
    if crate::lexer::parser_token_word_refs(tokens) == ["pay", "x"] {
        return Err(crate::cards::builders::CardTextError::ParseError(
            "pay X requires a resource or a mana symbol".into(),
        ));
    }

    let words = crate::lexer::parser_token_word_refs(tokens);
    let reference = match words.as_slice() {
        ["pay", "its", "mana", "cost"] => Some(crate::target::ChooseSpec::tagged(
            crate::tag::CompilerReferenceTag::It.key(),
        )),
        [
            "pay",
            "enchanted",
            "creatures" | "creature's",
            "mana",
            "cost",
        ] => {
            let mut filter = crate::target::ObjectFilter::creature();
            filter.with_attached_object = Some(Box::new(crate::target::ObjectFilter::source()));
            Some(crate::target::ChooseSpec::Object(filter))
        }
        _ => None,
    };
    if let Some(reference) = reference {
        let mut cost = ironsmith_core::DynamicManaCost::from_object_mana_cost(reference);
        if words.get(1) == Some(&"enchanted") {
            cost.display_hint = ironsmith_core::DynamicManaDisplayHint::EnchantedCreatureManaCost;
        }
        return Ok(ActivationCostSegmentCst::DynamicMana(cost));
    }
    parse_simple_segment(tokens, parse_pay_segment_lexed, "pay-cost")
}

pub fn parse_mill_segment_tokens(
    tokens: &[OwnedLexToken],
) -> Result<ActivationCostSegmentCst, CardTextError> {
    parse_simple_segment(tokens, parse_mill_segment_lexed, "mill")
}

pub fn parse_behold_segment_tokens(
    tokens: &[OwnedLexToken],
) -> Result<ActivationCostSegmentCst, CardTextError> {
    parse_simple_segment(tokens, parse_behold_segment_lexed, "behold")
}

pub fn parse_blight_segment_tokens(
    tokens: &[OwnedLexToken],
) -> Result<ActivationCostSegmentCst, CardTextError> {
    parse_simple_segment(tokens, parse_blight_segment_lexed, "blight")
}

pub fn parse_forage_segment_tokens(
    tokens: &[OwnedLexToken],
) -> Result<ActivationCostSegmentCst, CardTextError> {
    parse_simple_segment(
        tokens,
        |input: &mut LexStream<'_>| {
            primitives::kw("forage").parse_next(input)?;
            eof.parse_next(input)?;
            Ok(ActivationCostSegmentCst::Forage)
        },
        "forage",
    )
}

pub fn parse_collect_evidence_segment_tokens(
    tokens: &[OwnedLexToken],
) -> Result<ActivationCostSegmentCst, CardTextError> {
    parse_simple_segment(
        tokens,
        |input: &mut LexStream<'_>| {
            primitives::phrase(&["collect", "evidence"]).parse_next(input)?;
            let amount = if opt(primitives::kw("x")).parse_next(input)?.is_some() {
                Value::X
            } else {
                Value::Fixed(leaf::parse_leaf_number_prefix_lexed.parse_next(input)? as i32)
            };
            eof.parse_next(input)?;
            Ok(ActivationCostSegmentCst::CollectEvidence { amount })
        },
        "collect-evidence",
    )
}

pub fn parse_exert_segment_tokens(
    tokens: &[OwnedLexToken],
) -> Result<ActivationCostSegmentCst, CardTextError> {
    primitives::parse_all(
        tokens,
        parse_exert_segment_lexed,
        "activation-exert-segment",
    )
    .map(|()| ActivationCostSegmentCst::ExertSelf {
        display_text: render_token_slice(tokens).trim().to_string(),
    })
    .map_err(|_| {
        CardTextError::ParseError(format!(
            "rewrite exert-cost parser does not yet support '{}'",
            render_token_slice(tokens).trim()
        ))
    })
}

fn parse_simple_segment<'a>(
    tokens: &'a [OwnedLexToken],
    parser: impl Parser<
        LexStream<'a>,
        ActivationCostSegmentCst,
        winnow::error::ErrMode<winnow::error::ContextError>,
    >,
    label: &str,
) -> Result<ActivationCostSegmentCst, CardTextError> {
    primitives::parse_all(tokens, parser, label).map_err(|_| {
        CardTextError::ParseError(format!(
            "rewrite {label} parser does not yet support '{}'",
            render_token_slice(tokens).trim().to_ascii_lowercase()
        ))
    })
}

fn parse_bare_symbol_segment_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<ActivationCostSegmentCst> {
    let tokens: Vec<&OwnedLexToken> = repeat(1.., any).parse_next(input)?;
    if tokens.len() == 1 {
        if is_tap_activation_symbol_token(tokens[0]) {
            return Ok(ActivationCostSegmentCst::Tap);
        }
        if is_untap_symbol_token(tokens[0]) {
            return Ok(ActivationCostSegmentCst::Untap);
        }
    }

    if tokens.iter().all(|token| is_energy_symbol_token(token)) {
        return u32::try_from(tokens.len())
            .map(ActivationCostSegmentCst::Energy)
            .map_err(|_| primitives::backtrack_err("energy cost", "representable energy count"));
    }
    if tokens
        .iter()
        .any(|token| is_reserved_activation_symbol_token(token))
    {
        return Err(primitives::backtrack_err(
            "activation symbol",
            "unmixed tap, untap, or energy symbol",
        ));
    }

    let mut pips = Vec::new();
    for token in tokens {
        let parsed = leaf::parse_leaf_surface_mana_pip_token(token)
            .ok_or_else(|| primitives::backtrack_err("mana cost", "one or more mana symbols"))?;
        pips.push(parsed.into_pip());
    }
    Ok(ActivationCostSegmentCst::Mana(ManaCost::from_pips(pips)))
}

fn parse_pay_segment_lexed<'a>(input: &mut LexStream<'a>) -> WResult<ActivationCostSegmentCst> {
    primitives::kw("pay").parse_next(input)?;
    alt((
        parse_life_payment,
        parse_half_life_payment,
        parse_mana_per_count_payment,
        parse_life_equal_payment,
        parse_counted_energy_payment,
        parse_energy_payment,
        parse_bare_symbol_segment_lexed,
    ))
    .parse_next(input)
}

fn parse_exert_segment_lexed<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::kw("exert").parse_next(input)?;
    repeat::<_, _, (), _, _>(1.., any.void()).parse_next(input)?;
    eof.parse_next(input)?;
    Ok(())
}

fn parse_life_payment<'a>(input: &mut LexStream<'a>) -> WResult<ActivationCostSegmentCst> {
    let amount = alt((
        primitives::kw("x").value(Value::X),
        leaf::parse_leaf_number_prefix_lexed.map(|n| Value::Fixed(n as i32)),
    ))
    .parse_next(input)?;
    alt((primitives::kw("life"), primitives::kw("lives"))).parse_next(input)?;
    let suffix: &[OwnedLexToken] = repeat::<_, _, (), _, _>(0.., any.void())
        .take()
        .parse_next(input)?;
    eof.parse_next(input)?;
    if suffix.is_empty() {
        return Ok(ActivationCostSegmentCst::Life(amount));
    }
    let words = crate::lexer::parser_token_word_refs(suffix);
    let per = if words == ["for", "each", "card", "in", "your", "hand"] {
        // Preserve the existing fixed-card-count payload shape.
        Value::CardsInHand(PlayerFilter::You)
    } else {
        complete_payment_multiplier(suffix)?
    };
    let Value::Fixed(scale) = amount else {
        return Err(primitives::backtrack_err(
            "life payment",
            "fixed multiplier before a counted amount",
        ));
    };
    Ok(ActivationCostSegmentCst::Life(if scale == 1 {
        per
    } else {
        Value::Scaled(Box::new(per), scale)
    }))
}

fn complete_payment_multiplier(tokens: &[OwnedLexToken]) -> WResult<Value> {
    let words = crate::lexer::parser_token_word_refs(tokens);
    if words.iter().any(|word| {
        matches!(
            *word,
            "draw" | "discard" | "then" | "pay" | "gain" | "lose" | "create"
        )
    }) {
        return Err(primitives::backtrack_err(
            "payment multiplier",
            "complete quantity without an instruction tail",
        ));
    }
    let (value, used) = crate::util::parse_for_each_count_value_words(&words)
        .ok_or_else(|| primitives::backtrack_err("payment multiplier", "typed for-each amount"))?;
    if used != words.len() {
        return Err(primitives::backtrack_err(
            "payment multiplier",
            "complete for-each amount",
        ));
    }
    Ok(value)
}

fn parse_half_life_payment<'a>(input: &mut LexStream<'a>) -> WResult<ActivationCostSegmentCst> {
    primitives::phrase(&["half", "your", "life"]).parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::kw("rounded").parse_next(input)?;
    let value = alt((
        primitives::kw("up").value(Value::HalfLifeTotalRoundedUp(PlayerFilter::You)),
        primitives::kw("down").value(Value::HalfLifeTotalRoundedDown(PlayerFilter::You)),
    ))
    .parse_next(input)?;
    eof.parse_next(input)?;
    Ok(ActivationCostSegmentCst::Life(value))
}

fn parse_mana_per_count_payment<'a>(
    input: &mut LexStream<'a>,
) -> WResult<ActivationCostSegmentCst> {
    let tokens: &[OwnedLexToken] = repeat::<_, _, (), _, _>(1.., any.void())
        .take()
        .parse_next(input)?;
    let for_index = tokens
        .iter()
        .position(|token| token.is_word("for"))
        .ok_or_else(|| primitives::backtrack_err("mana payment", "for-each multiplier"))?;
    let Some(ActivationCostSegmentCst::Mana(base)) =
        parse_bare_symbol_segment_tokens(&tokens[..for_index])
    else {
        return Err(primitives::backtrack_err(
            "mana payment",
            "mana symbols before multiplier",
        ));
    };
    let multiplier = complete_payment_multiplier(&tokens[for_index..])?;
    Ok(ActivationCostSegmentCst::DynamicMana(
        ironsmith_core::DynamicManaCost::new(
            base,
            None,
            None,
            Some(multiplier),
            ironsmith_core::DynamicManaDisplayHint::Default,
        ),
    ))
}

/// "Pay life equal to <value>" (War Room: "the number of colors in your
/// commanders' color identity").
fn parse_life_equal_payment<'a>(input: &mut LexStream<'a>) -> WResult<ActivationCostSegmentCst> {
    primitives::phrase(&["life", "equal", "to"]).parse_next(input)?;
    let rest: &[OwnedLexToken] = repeat::<_, _, (), _, _>(1.., any.void())
        .take()
        .parse_next(input)?;
    eof.parse_next(input)?;
    let words = crate::lexer::token_word_refs(rest);
    // Pronoun-relative amounts ("life equal to its toughness") are resolved by
    // the target-aware unless-cost grammar; this segment reads only
    // self-contained value phrases. An explicit `this creature's ...` is
    // source-bound, not an unresolved target pronoun, and is read by the
    // shared typed value grammar below.
    if words.iter().any(|word| {
        matches!(
            *word,
            "its" | "it" | "it's" | "their" | "that" | "his" | "her"
        )
    }) {
        return Err(primitives::backtrack_err(
            "life payment",
            "self-contained value phrase",
        ));
    }
    let value = if matches!(
        words.as_slice(),
        ["the", "number", "of", "colors", "in", "your", commanders, "color", "identity"]
            if commanders.starts_with("commander")
    ) {
        Value::CommanderColorIdentityColors(PlayerFilter::You)
    } else {
        let (value, used) = crate::grammar::shared_util::value_expr::parse_value_expr_tokens(rest)
            .ok_or_else(|| primitives::backtrack_err("life payment", "value expression"))?;
        if used != rest.len() {
            return Err(primitives::backtrack_err(
                "life payment",
                "complete value expression",
            ));
        }
        value
    };
    Ok(ActivationCostSegmentCst::Life(value))
}

fn parse_counted_energy_payment<'a>(
    input: &mut LexStream<'a>,
) -> WResult<ActivationCostSegmentCst> {
    let amount = alt((
        primitives::kw("x").value(Value::X),
        leaf::parse_leaf_number_prefix_lexed.map(|n| Value::Fixed(n as i32)),
    ))
    .parse_next(input)?;
    parse_energy_symbol.parse_next(input)?;
    eof.parse_next(input)?;
    Ok(match amount {
        Value::Fixed(amount) => ActivationCostSegmentCst::Energy(amount as u32),
        other => ActivationCostSegmentCst::EnergyValue(other),
    })
}

fn parse_energy_payment<'a>(input: &mut LexStream<'a>) -> WResult<ActivationCostSegmentCst> {
    let symbols: Vec<()> = repeat(1.., parse_energy_symbol).parse_next(input)?;
    eof.parse_next(input)?;
    let count = u32::try_from(symbols.len())
        .map_err(|_| primitives::backtrack_err("energy payment", "representable energy count"))?;
    Ok(ActivationCostSegmentCst::Energy(count))
}

fn parse_energy_symbol<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    any.verify(|token: &&OwnedLexToken| is_energy_symbol_token(token))
        .void()
        .parse_next(input)
}

fn parse_mill_segment_lexed<'a>(input: &mut LexStream<'a>) -> WResult<ActivationCostSegmentCst> {
    primitives::kw("mill").parse_next(input)?;
    let count = leaf::parse_leaf_number_prefix_lexed.parse_next(input)?;
    alt((primitives::kw("card"), primitives::kw("cards"))).parse_next(input)?;
    eof.parse_next(input)?;
    Ok(ActivationCostSegmentCst::Mill(count))
}

fn parse_behold_segment_lexed<'a>(input: &mut LexStream<'a>) -> WResult<ActivationCostSegmentCst> {
    primitives::kw("behold").parse_next(input)?;
    let count = leaf::parse_leaf_number_prefix_lexed.parse_next(input)?;
    let subtype_word = primitives::word_parser_text.parse_next(input)?;
    let subtype = leaf::parse_leaf_subtype_flexible_complete(subtype_word)
        .map_err(|_| primitives::backtrack_err("behold subtype", "known subtype"))?;
    // "behold a Gamma creature" (Hulk's Thunderclap): the noun after a
    // creature type restates that the beheld object is a creature.
    if subtype.is_creature_type() {
        opt(alt((primitives::kw("creature"), primitives::kw("creatures")))).parse_next(input)?;
    }
    eof.parse_next(input)?;
    Ok(ActivationCostSegmentCst::Behold { subtype, count })
}

fn parse_blight_segment_lexed<'a>(input: &mut LexStream<'a>) -> WResult<ActivationCostSegmentCst> {
    primitives::kw("blight").parse_next(input)?;
    let (count, x) = alt((
        primitives::kw("x").value((0, true)),
        leaf::parse_leaf_number_prefix_lexed.map(|count| (count, false)),
    ))
    .parse_next(input)?;
    eof.parse_next(input)?;
    Ok(ActivationCostSegmentCst::Blight { count, x })
}

fn is_energy_symbol_token(token: &OwnedLexToken) -> bool {
    match token.kind {
        TokenKind::ManaGroup => token.slice.eq_ignore_ascii_case("{e}"),
        TokenKind::Word | TokenKind::Number => token
            .as_word()
            .is_some_and(|word| word.eq_ignore_ascii_case("e")),
        _ => false,
    }
}

pub fn is_tap_activation_symbol_token(token: &OwnedLexToken) -> bool {
    token
        .as_word()
        .is_some_and(|word| word.eq_ignore_ascii_case("t"))
        || token.slice.eq_ignore_ascii_case("{t}")
}

fn is_untap_symbol_token(token: &OwnedLexToken) -> bool {
    token
        .as_word()
        .is_some_and(|word| word.eq_ignore_ascii_case("q"))
        || token.slice.eq_ignore_ascii_case("{q}")
}

fn is_reserved_activation_symbol_token(token: &OwnedLexToken) -> bool {
    is_energy_symbol_token(token)
        || is_tap_activation_symbol_token(token)
        || is_untap_symbol_token(token)
}

#[cfg(test)]
mod tests {
    use super::super::super::super::lexer::lex_line;
    use super::super::{ActivationCostSegmentKind, parse_activation_cost_segment_kind_tokens};
    use super::*;

    fn parse(raw: &str) -> ActivationCostSegmentCst {
        let tokens = lex_line(raw, 0).unwrap();
        match parse_activation_cost_segment_kind_tokens(&tokens) {
            ActivationCostSegmentKind::Pay => parse_pay_segment_tokens(&tokens).unwrap(),
            ActivationCostSegmentKind::Mill => parse_mill_segment_tokens(&tokens).unwrap(),
            ActivationCostSegmentKind::Behold => parse_behold_segment_tokens(&tokens).unwrap(),
            ActivationCostSegmentKind::Blight => parse_blight_segment_tokens(&tokens).unwrap(),
            _ => parse_bare_symbol_segment_tokens(&tokens).unwrap(),
        }
    }

    #[test]
    fn equal_life_payment_accepts_explicit_typed_source_characteristics() {
        for (text, power) in [
            ("pay life equal to this creature's power", true),
            ("pay life equal to this creature's toughness", false),
        ] {
            let ActivationCostSegmentCst::Life(value) = parse(text) else {
                panic!("expected a typed life payment");
            };
            let target = match value.unhinted() {
                Value::PowerOf(target) if power => target,
                Value::ToughnessOf(target) if !power => target,
                value => panic!("source characteristic was not retained: {value:?}"),
            };
            assert!(matches!(
                target.unhinted(),
                crate::target::ChooseSpec::Source
            ));
        }
        for text in [
            "pay life equal to its power",
            "pay life equal to that creature's power",
            "pay life equal to this creature's prestige",
            "pay life equal to this",
        ] {
            let tokens = lex_line(text, 0).unwrap();
            assert!(parse_pay_segment_tokens(&tokens).is_err(), "{text}");
        }
    }

    #[test]
    fn simple_segments_return_typed_cst() {
        assert_eq!(
            parse("pay 2 life"),
            ActivationCostSegmentCst::Life(Value::Fixed(2))
        );
        assert_eq!(
            parse("pay 1 life for each card in your hand"),
            ActivationCostSegmentCst::Life(Value::CardsInHand(PlayerFilter::You))
        );
        assert_eq!(parse("pay {e}"), ActivationCostSegmentCst::Energy(1));
        assert_eq!(parse("mill three cards"), ActivationCostSegmentCst::Mill(3));
        assert_eq!(
            parse("behold a goblin"),
            ActivationCostSegmentCst::Behold {
                subtype: crate::types::Subtype::Goblin,
                count: 1,
            }
        );
        assert_eq!(
            parse("blight 2"),
            ActivationCostSegmentCst::Blight { count: 2, x: false }
        );
    }
    #[test]
    fn variable_resource_costs_reuse_existing_typed_amounts() {
        assert_eq!(
            parse("pay x {e}"),
            ActivationCostSegmentCst::EnergyValue(Value::X)
        );
        assert_eq!(
            parse("pay x life"),
            ActivationCostSegmentCst::Life(Value::X)
        );
        assert_eq!(parse("pay 3 {e}"), ActivationCostSegmentCst::Energy(3));
        assert_eq!(
            parse("pay half your life, rounded up"),
            ActivationCostSegmentCst::Life(Value::HalfLifeTotalRoundedUp(PlayerFilter::You))
        );
        assert_eq!(
            parse("pay half your life rounded down"),
            ActivationCostSegmentCst::Life(Value::HalfLifeTotalRoundedDown(PlayerFilter::You))
        );
        let ActivationCostSegmentCst::Life(Value::Scaled(count, 3)) =
            parse("pay 3 life for each velocity counter on this enchantment")
        else {
            panic!("expected a typed three-times counter amount");
        };
        assert!(matches!(count.unhinted(), Value::CountersOn(_, Some(kind))
            if *kind == crate::object::CounterType::Velocity));
        let ActivationCostSegmentCst::DynamicMana(cost) =
            parse("pay {1} for each +1/+1 counter on this creature")
        else {
            panic!("expected dynamic mana, not a textual or fixed cost");
        };
        assert_eq!(cost.base.generic_mana_total(), 1);
        assert!(matches!(
            cost.multiplier.as_ref().unwrap().unhinted(),
            Value::CountersOn(_, Some(crate::object::CounterType::PlusOnePlusOne))
        ));
        for malformed in [
            "pay x",
            "pay x energy",
            "pay half your life",
            "pay half your life rounded sideways",
            "pay 3 life for each velocity counter on this enchantment or draw a card",
            "pay {1} for each",
            "pay {1} for each +1/+1 counter on this creature then draw a card",
        ] {
            assert!(
                parse_pay_segment_tokens(&lex_line(malformed, 0).unwrap()).is_err(),
                "{malformed}"
            );
        }
    }
}

#[cfg(test)]
mod referenced_mana_cost_tests {
    use super::*;
    #[test]
    fn referenced_mana_costs_are_typed_and_do_not_accept_partial_suffixes() {
        for text in ["Pay its mana cost", "Pay enchanted creature's mana cost"] {
            let tokens = crate::lexer::lex_line(text, 0).unwrap();
            let ActivationCostSegmentCst::DynamicMana(cost) =
                parse_pay_segment_tokens(&tokens).unwrap()
            else {
                panic!("typed object mana cost expected");
            };
            assert!(cost.mana_cost_of.is_some());
            assert!(!cost.source_mana_cost);
            assert!(cost.resolved_static_base().is_none());
        }
        for text in [
            "Pay its mana",
            "Pay its mana cost banana",
            "Pay enchanted creature's mana cost banana",
        ] {
            assert!(
                parse_pay_segment_tokens(&crate::lexer::lex_line(text, 0).unwrap()).is_err(),
                "{text}"
            );
        }
    }
}
