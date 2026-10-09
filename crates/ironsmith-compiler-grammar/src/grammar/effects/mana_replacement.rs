use winnow::combinator::{alt, opt, repeat_till};
use winnow::error::{ContextError, ErrMode};
use winnow::prelude::*;
use winnow::token::any;

use crate::lexer::{LexStream, OwnedLexToken, trim_lexed_commas};
use crate::mana::ManaSymbol;

use super::super::{leaf, primitives};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ManaReplacementClauseSpec {
    pub replacement_mana: ManaSymbol,
}

fn parse_replacement_mana_symbol<'a>(
    input: &mut LexStream<'a>,
) -> Result<ManaSymbol, ErrMode<ContextError>> {
    let pip = leaf::parse_leaf_surface_mana_pip_lexed
        .parse_next(input)?
        .into_pip();
    let [symbol] = pip.as_slice() else {
        return Err(primitives::backtrack_err(
            "mana replacement symbol",
            "one colored or colorless mana symbol",
        ));
    };
    if matches!(
        symbol,
        ManaSymbol::White
            | ManaSymbol::Blue
            | ManaSymbol::Black
            | ManaSymbol::Red
            | ManaSymbol::Green
            | ManaSymbol::Colorless
    ) {
        Ok(*symbol)
    } else {
        Err(primitives::backtrack_err(
            "mana replacement symbol",
            "one colored or colorless mana symbol",
        ))
    }
}

fn parse_mana_replacement_clause<'a>(
    input: &mut LexStream<'a>,
) -> Result<ManaReplacementClauseSpec, ErrMode<ContextError>> {
    primitives::phrase(&["until", "end", "of", "turn"]).parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&[
        "if", "you", "tap", "a", "land", "you", "control", "for", "mana",
    ])
    .parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&["it", "produces"]).parse_next(input)?;
    let replacement_mana = parse_replacement_mana_symbol(input)?;
    primitives::phrase(&["instead", "of", "any", "other", "type"]).parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;

    Ok(ManaReplacementClauseSpec { replacement_mana })
}

/// "If a land is tapped for two or more mana, it produces {C} instead of any
/// other type and amount." (Damping Sphere)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TappedForAmountManaReplacementSpec<'a> {
    pub source_tokens: &'a [OwnedLexToken],
    pub minimum_amount: u32,
    pub replacement_mana: ManaSymbol,
}

fn parse_tapped_for_amount_mana_replacement<'a>(
    input: &mut LexStream<'a>,
) -> Result<TappedForAmountManaReplacementSpec<'a>, ErrMode<ContextError>> {
    primitives::kw("if").parse_next(input)?;
    opt(winnow::combinator::alt((
        primitives::kw("a"),
        primitives::kw("an"),
    )))
    .parse_next(input)?;
    let source_tokens: &'a [OwnedLexToken] =
        winnow::combinator::repeat_till::<_, _, (), _, _, _, _>(
            1..,
            winnow::token::any.void(),
            winnow::combinator::peek(primitives::phrase(&["is", "tapped", "for"])),
        )
        .map(|((), _)| ())
        .take()
        .parse_next(input)?;
    primitives::phrase(&["is", "tapped", "for"]).parse_next(input)?;
    let minimum_amount = leaf::parse_leaf_number_prefix_lexed.parse_next(input)?;
    primitives::phrase(&["or", "more", "mana"]).parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&["it", "produces"]).parse_next(input)?;
    let replacement_mana = parse_replacement_mana_symbol(input)?;
    primitives::phrase(&["instead", "of", "any", "other", "type", "and", "amount"])
        .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(TappedForAmountManaReplacementSpec {
        source_tokens: crate::lexer::trim_lexed_commas(source_tokens),
        minimum_amount,
        replacement_mana,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ManaMultiplierReplacementSpec<'a> {
    pub source_tokens: &'a [OwnedLexToken],
    pub factor: u32,
}

/// "If you tap a permanent for mana, it produces three times as much of that
/// mana instead." (Nyxbloom Ancient, Mana Reflection)
fn parse_mana_multiplier_replacement<'a>(
    input: &mut LexStream<'a>,
) -> Result<ManaMultiplierReplacementSpec<'a>, ErrMode<ContextError>> {
    primitives::phrase(&["if", "you", "tap"]).parse_next(input)?;
    let source_tokens = repeat_till::<_, _, (), _, _, _, _>(
        1..,
        any.void(),
        winnow::combinator::peek(primitives::phrase(&["for", "mana"])),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    primitives::phrase(&["for", "mana"]).parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&["it", "produces"]).parse_next(input)?;
    let factor = alt((
        primitives::kw("twice").value(2),
        primitives::phrase(&["two", "times"]).value(2),
        primitives::phrase(&["three", "times"]).value(3),
    ))
    .parse_next(input)?;
    primitives::phrase(&["as", "much", "of", "that", "mana", "instead"]).parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(ManaMultiplierReplacementSpec {
        source_tokens: trim_lexed_commas(source_tokens),
        factor,
    })
}

pub fn parse_mana_multiplier_replacement_spec_lexed(
    tokens: &[OwnedLexToken],
) -> Option<ManaMultiplierReplacementSpec<'_>> {
    crate::grammar::primitives::probe_all(
        tokens,
        parse_mana_multiplier_replacement,
        "mana multiplier replacement",
    )
}

pub fn parse_tapped_for_amount_mana_replacement_spec_lexed(
    tokens: &[OwnedLexToken],
) -> Option<TappedForAmountManaReplacementSpec<'_>> {
    crate::grammar::primitives::probe_all(
        tokens,
        parse_tapped_for_amount_mana_replacement,
        "tapped-for-amount mana replacement",
    )
}

pub fn parse_mana_replacement_clause_spec_lexed(
    tokens: &[OwnedLexToken],
) -> Option<ManaReplacementClauseSpec> {
    crate::grammar::primitives::probe_all(
        tokens,
        parse_mana_replacement_clause,
        "mana-replacement-clause",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex_line;

    #[test]
    fn parses_typed_replacement_symbol() {
        let tokens = lex_line(
            "Until end of turn, if you tap a land you control for mana, it produces {U} instead of any other type.",
            0,
        )
        .unwrap();
        let spec = parse_mana_replacement_clause_spec_lexed(&tokens).unwrap();

        assert_eq!(spec.replacement_mana, ManaSymbol::Blue);
    }

    #[test]
    fn accepts_each_colored_and_colorless_symbol() {
        for (raw, expected) in [
            ("{W}", ManaSymbol::White),
            ("{U}", ManaSymbol::Blue),
            ("{B}", ManaSymbol::Black),
            ("{R}", ManaSymbol::Red),
            ("{G}", ManaSymbol::Green),
            ("{C}", ManaSymbol::Colorless),
        ] {
            let line = format!(
                "Until end of turn, if you tap a land you control for mana, it produces {raw} instead of any other type."
            );
            let tokens = lex_line(&line, 0).unwrap();
            assert_eq!(
                parse_mana_replacement_clause_spec_lexed(&tokens)
                    .unwrap()
                    .replacement_mana,
                expected
            );
        }
    }

    #[test]
    fn rejects_generic_and_hybrid_replacement_pips() {
        for raw in ["{2}", "{W/U}"] {
            let line = format!(
                "Until end of turn, if you tap a land you control for mana, it produces {raw} instead of any other type."
            );
            let tokens = lex_line(&line, 0).unwrap();
            assert!(parse_mana_replacement_clause_spec_lexed(&tokens).is_none());
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ManaOutputRewriteShape<'a> {
    pub source_tokens: Option<&'a [OwnedLexToken]>,
    pub controller: Option<crate::target::PlayerFilter>,
    pub tapped_for_mana: bool,
    pub input: ironsmith_core::ManaRewriteInput,
    pub output: ironsmith_core::ManaRewriteOutput,
    pub quantity: ironsmith_core::ManaRewriteQuantity,
    pub mode: Option<crate::effects::ReplacementApplyMode>,
}

fn mana_rewrite_output<'a>(input: &mut LexStream<'a>) -> Result<ironsmith_core::ManaRewriteOutput, ErrMode<ContextError>> {
    use ironsmith_core::ManaRewriteOutput as Output;
    alt((
        parse_replacement_mana_symbol.map(Output::Symbol),
        primitives::phrase(&["colorless", "mana"]).value(Output::Symbol(ManaSymbol::Colorless)),
        primitives::phrase(&["one", "mana", "of", "a", "color", "of", "your", "choice"]).value(Output::ChooseColor),
        primitives::phrase(&["mana", "of", "a", "color", "of", "your", "choice"]).value(Output::ChooseColor),
        primitives::phrase(&["mana", "of", "the", "chosen", "color"]).value(Output::ChosenColor),
    )).parse_next(input)
}
fn mana_rewrite_tail<'a>(input: &mut LexStream<'a>)
    -> Result<(ironsmith_core::ManaRewriteInput, ironsmith_core::ManaRewriteQuantity), ErrMode<ContextError>> {
    use ironsmith_core::{ManaRewriteInput as Input, ManaRewriteQuantity as Quantity};
    primitives::phrase(&["instead", "of"]).parse_next(input)?;
    let input_kind = alt((
        primitives::phrase(&["any", "other", "type"]).value(Input::Any),
        primitives::phrase(&["any", "other", "color"]).value(Input::Colored),
        primitives::phrase(&["white", "mana"]).value(Input::Symbol(ManaSymbol::White)),
    )).parse_next(input)?;
    let exact = opt(primitives::phrase(&["and", "amount"])).parse_next(input)?.is_some();
    primitives::sentence_end().parse_next(input)?;
    if exact && input_kind != Input::Any {
        return Err(primitives::backtrack_err("mana rewrite", "type-and-amount production"));
    }
    Ok((input_kind, if exact {Quantity::Exact(1)} else {Quantity::Preserve}))
}
fn mana_rewrite_basic_mapping<'a>(input: &mut LexStream<'a>) -> Result<ManaOutputRewriteShape<'a>, ErrMode<ContextError>> {
    use ironsmith_core::{ManaRewriteInput as Input, ManaRewriteOutput as Output, ManaRewriteQuantity as Quantity};
    primitives::phrase(&["if", "tapped", "for", "mana"]).parse_next(input)?;
    primitives::comma().parse_next(input)?;
    let mut mapping = [None; 5]; let mut entries = 0;
    loop {
        if entries > 0 {
            alt((primitives::comma().void(), primitives::kw("and").void())).parse_next(input)?;
            opt(primitives::kw("and")).parse_next(input)?;
        }
        let index = alt((primitives::kw("plains").value(0), primitives::kw("islands").value(1),
            primitives::kw("swamps").value(2), primitives::kw("mountains").value(3),
            primitives::kw("forests").value(4))).parse_next(input)?;
        primitives::kw("produce").parse_next(input)?;
        if mapping[index].is_some() { return Err(primitives::backtrack_err("mana rewrite", "distinct land-type clauses")); }
        mapping[index] = Some(parse_replacement_mana_symbol(input)?); entries += 1;
        if input.as_ref().first().is_some_and(|token| token.is_word("instead")) { break; }
        if entries == 5 { return Err(primitives::backtrack_err("mana rewrite", "complete land-type mapping")); }
    }
    let (affected, quantity) = mana_rewrite_tail(input)?;
    if entries < 2 || affected != Input::Any || quantity != Quantity::Preserve {
        return Err(primitives::backtrack_err("mana rewrite", "type-preserving land mapping"));
    }
    Ok(ManaOutputRewriteShape {source_tokens: None, controller: None, tapped_for_mana: true,
        input: affected, output: Output::ByBasicLandType(mapping), quantity, mode: None})
}
fn mana_rewrite_controlled_colored<'a>(input: &mut LexStream<'a>) -> Result<ManaOutputRewriteShape<'a>, ErrMode<ContextError>> {
    primitives::phrase(&["spells", "and", "abilities", "you", "control", "that", "would", "add", "colored", "mana", "instead", "add", "that", "much", "white", "mana"]).parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(ManaOutputRewriteShape {source_tokens: None, controller: Some(crate::target::PlayerFilter::You), tapped_for_mana: false,
        input: ironsmith_core::ManaRewriteInput::Colored,
        output: ironsmith_core::ManaRewriteOutput::Symbol(ManaSymbol::White),
        quantity: ironsmith_core::ManaRewriteQuantity::Preserve, mode: None})
}
fn mana_rewrite_filtered_source<'a>(input: &mut LexStream<'a>) -> Result<ManaOutputRewriteShape<'a>, ErrMode<ContextError>> {
    let has_if = opt(primitives::kw("if")).parse_next(input)?.is_some();
    let actor = if has_if {
        opt(alt((primitives::phrase(&["you", "tap"]).value(true),
            primitives::phrase(&["a", "player", "taps"]).value(false)))).parse_next(input)?
    } else { None };
    let source_tokens: &'a [OwnedLexToken] = if actor.is_some() {
        let source = repeat_till::<_, _, (), _, _, _, _>(1.., any.void(),
            winnow::combinator::peek(primitives::phrase(&["for", "mana"])))
            .map(|((), _)| ()).take().parse_next(input)?;
        primitives::phrase(&["for", "mana"]).parse_next(input)?;
        source
    } else {
        let source = repeat_till::<_, _, (), _, _, _, _>(1.., any.void(),
            winnow::combinator::peek(alt((primitives::phrase(&["is", "tapped", "for", "mana"]),
                primitives::phrase(&["tapped", "for", "mana"]))))).map(|((), _)| ()).take().parse_next(input)?;
        if has_if { primitives::kw("is").parse_next(input)?; }
        primitives::phrase(&["tapped", "for", "mana"]).parse_next(input)?;
        source
    };
    if has_if {
        opt(primitives::comma()).parse_next(input)?;
        // "..., that Mountain produces colorless mana instead" (Chaos Moon):
        // the tapped source named again, same as "it".
        alt((
            primitives::phrase(&["it", "produces"]),
            (primitives::kw("that"), any, primitives::kw("produces")).void(),
        ))
        .parse_next(input)?;
    } else { primitives::kw("produce").parse_next(input)?; }
    let output = mana_rewrite_output(input)?;
    let (affected, quantity) = mana_rewrite_tail(input)?;
    Ok(ManaOutputRewriteShape {source_tokens: Some(trim_lexed_commas(source_tokens)),
        controller: actor.filter(|you| *you).map(|_| crate::target::PlayerFilter::You), tapped_for_mana: true,
        input: affected, output, quantity, mode: None})
}
fn mana_output_rewrite<'a>(input: &mut LexStream<'a>) -> Result<ManaOutputRewriteShape<'a>, ErrMode<ContextError>> {
    let mode = opt((primitives::phrase(&["until", "end", "of", "turn"]), opt(primitives::comma())))
        .parse_next(input)?.map(|_| crate::effects::ReplacementApplyMode::UntilEndOfTurn);
    let mut shape = alt((mana_rewrite_basic_mapping, mana_rewrite_controlled_colored, mana_rewrite_filtered_source)).parse_next(input)?;
    shape.mode = mode;
    Ok(shape)
}
pub fn parse_mana_output_rewrite_shape(tokens: &[OwnedLexToken]) -> Option<ManaOutputRewriteShape<'_>> {
    primitives::probe_all(tokens, mana_output_rewrite, "typed-mana-output-rewrite")
}

fn temporary_symbol_spend_permission<'a>(input: &mut LexStream<'a>) -> Result<ManaSymbol, ErrMode<ContextError>> {
    primitives::phrase(&["until", "end", "of", "turn"]).parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&["you", "may", "spend"]).parse_next(input)?;
    let symbol = alt((primitives::kw("white").value(ManaSymbol::White),
        primitives::kw("blue").value(ManaSymbol::Blue), primitives::kw("black").value(ManaSymbol::Black),
        primitives::kw("red").value(ManaSymbol::Red), primitives::kw("green").value(ManaSymbol::Green))).parse_next(input)?;
    primitives::phrase(&["mana", "as", "though", "it", "were", "mana", "of", "any", "color"]).parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(symbol)
}
pub fn parse_temporary_symbol_spend_permission_shape(tokens: &[OwnedLexToken]) -> Option<ManaSymbol> {
    primitives::probe_all(tokens, temporary_symbol_spend_permission, "temporary-symbol-mana-spend-permission")
}

#[cfg(test)]
mod mana_output_rewrite_contract {
    use super::*;
    use ironsmith_core::{ManaRewriteInput as Input, ManaRewriteOutput as Output, ManaRewriteQuantity as Quantity};
    #[test]
    fn exact_complete_clauses_retain_input_scope_amount_and_actor() {
        for (text, input, quantity) in [
            ("If a land is tapped for mana, it produces {B} instead of any other type and amount.", Input::Any, Quantity::Exact(1)),
            ("If a land is tapped for mana, it produces {B} instead of any other type.", Input::Any, Quantity::Preserve),
            ("If target Plains is tapped for mana, it produces colorless mana instead of white mana.", Input::Symbol(ManaSymbol::White), Quantity::Preserve),
            ("Until end of turn, lands tapped for mana produce mana of the chosen color instead of any other color.", Input::Colored, Quantity::Preserve),
        ] {
            let tokens = crate::lexer::lex_line(text, 0).unwrap();
            let shape = parse_mana_output_rewrite_shape(&tokens).unwrap();
            assert_eq!(shape.input, input); assert_eq!(shape.quantity, quantity);
        }
        let tokens = crate::lexer::lex_line("Until end of turn, if you tap a land for mana, it produces one mana of a color of your choice instead of any other type and amount.", 0).unwrap();
        let shape = parse_mana_output_rewrite_shape(&tokens).unwrap();
        assert_eq!(shape.controller, Some(crate::target::PlayerFilter::You));
        assert_eq!(shape.output, Output::ChooseColor); assert_eq!(shape.quantity, Quantity::Exact(1));
        assert_eq!(shape.mode, Some(crate::effects::ReplacementApplyMode::UntilEndOfTurn));
    }
    #[test]
    fn mapping_is_one_typed_output_table_and_unknown_tails_are_not_ignored() {
        let text = "If tapped for mana, Plains produce {R}, Islands produce {G}, Swamps produce {W}, Mountains produce {U}, and Forests produce {B} instead of any other type.";
        let tokens = crate::lexer::lex_line(text, 0).unwrap();
        let shape = parse_mana_output_rewrite_shape(&tokens).unwrap();
        assert_eq!(shape.output, Output::ByBasicLandType([Some(ManaSymbol::Red), Some(ManaSymbol::Green),
            Some(ManaSymbol::White), Some(ManaSymbol::Blue), Some(ManaSymbol::Black)]));
        for text in ["If a land is tapped for mana, it produces {2} instead of any other type.",
            "If a land is tapped for mana, it produces {W/U} instead of any other type.",
            "If a land is tapped for mana, it produces {B} instead of any other type and impossible quantity.",
            "If a land is tapped for mana, it produces mana of a color of your choice instead of any other type with a secret bonus."] {
            assert!(parse_mana_output_rewrite_shape(&crate::lexer::lex_line(text, 0).unwrap()).is_none(), "{text}");
        }
    }
}

fn unspent_mana_conversion<'a>(input: &mut LexStream<'a>) -> Result<ManaSymbol, ErrMode<ContextError>> {
    primitives::phrase(&["if", "you", "would", "lose", "unspent", "mana"]).parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&["that", "mana", "becomes"]).parse_next(input)?;
    let symbol = alt((primitives::kw("white").value(ManaSymbol::White),
        primitives::kw("blue").value(ManaSymbol::Blue), primitives::kw("black").value(ManaSymbol::Black),
        primitives::kw("red").value(ManaSymbol::Red), primitives::kw("green").value(ManaSymbol::Green),
        primitives::kw("colorless").value(ManaSymbol::Colorless))).parse_next(input)?;
    primitives::kw("instead").parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(symbol)
}
pub fn parse_unspent_mana_conversion(tokens: &[OwnedLexToken]) -> Option<ManaSymbol> {
    primitives::probe_all(tokens, unspent_mana_conversion, "unspent-mana conversion")
}

fn unspent_mana_threshold<'a>(input: &mut LexStream<'a>) -> Result<u32, ErrMode<ContextError>> {
    primitives::phrase(&["you", "have"]).parse_next(input)?;
    let count = leaf::parse_leaf_number_prefix_lexed.parse_next(input)?;
    primitives::phrase(&["or", "more", "unspent", "mana"]).parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(count)
}
pub fn parse_unspent_mana_threshold(tokens: &[OwnedLexToken]) -> Option<u32> {
    primitives::probe_all(tokens, unspent_mana_threshold, "unspent-mana threshold")
}

#[cfg(test)]
mod mana_loss_shapes {
    use super::*;
    #[test]
    fn conversion_and_live_threshold_consume_complete_typed_clauses() {
        use crate::lexer::lex_line;
        let lex = |text| lex_line(text, 0).unwrap();
        assert_eq!(parse_unspent_mana_conversion(&lex("If you would lose unspent mana, that mana becomes colorless instead.")), Some(ManaSymbol::Colorless));
        assert_eq!(parse_unspent_mana_conversion(&lex("If you would lose unspent mana, that mana becomes red instead.")), Some(ManaSymbol::Red));
        assert_eq!(parse_unspent_mana_threshold(&lex("you have six or more unspent mana")), Some(6));
        for text in ["If you would lose unspent mana, that mana becomes snow instead.",
            "If you would lose unspent mana, that mana becomes black instead and draw a card.",
            "If you would lose unspent green mana, that mana becomes black instead."] {
            assert!(parse_unspent_mana_conversion(&lex(text)).is_none(), "{text}");
        }
        assert!(parse_unspent_mana_threshold(&lex("you have six or more unspent mana from lands")).is_none());
    }
}
