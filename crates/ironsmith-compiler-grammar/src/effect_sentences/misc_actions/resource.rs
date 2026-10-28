use super::*;

use crate::recognition::ParseOutcome;
#[path = "resource/pay_readings.rs"]
mod pay_readings;

pub fn parse_pay(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);
    // "you may pay that much life again" (Verrak): repeating the amount
    // already paid is an ordinary payment of that amount.
    let tokens = match tokens.iter().rposition(|token| token.as_word().is_some()) {
        Some(last) if last > 0 && tokens[last].is_word("again") => &tokens[..last],
        _ => tokens,
    };
    let energy_symbol_count = tokens
        .iter()
        .filter(|token| energy_symbol_token(token))
        .count();

    let clause_words = crate::lexer::token_word_refs(tokens);
    let input = pay_readings::PayClause {
        tokens,
        player,
        energy_symbol_count,
        clause_words: &clause_words,
        read_by_cache: Default::default(),
    };
    match pay_readings::read(&input) {
        ParseOutcome::Match(matched) => return Ok(matched.value.value),
        ParseOutcome::NoMatch => {}
        ParseOutcome::Error(diagnostic) => return Err(diagnostic.into_card_text_error()),
    }
    // "You may pay {W} to end this effect" (Licids) is a later special
    // action that ends a continuous effect, not a mana payment made now.
    // The leading-pip reading below would keep only the payment.
    if clause_words
        .windows(4)
        .any(|window| window == ["to", "end", "this", "effect"])
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported pay-to-end-effect clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    // "you may pay {G}, {W}, or {U}" (Isu the Abominable): one payment of
    // any one of the listed symbols, not all of them.
    if let Some(alternatives) = single_symbol_payment_alternatives(tokens) {
        return Ok(EffectAst::subject_verb_pay_mana(
            player,
            ManaCost::from_pips(vec![alternatives]),
        ));
    }
    let (pips, rest) = {
        use winnow::prelude::*;
        use winnow::stream::Stream;
        let mut stream = LexStream::new(tokens);
        let pips = grammar::collect_mana_pip_groups
            .parse_next(&mut stream)
            .map_err(|_| {
                CardTextError::ParseError(format!(
                    "missing payment cost (clause: '{}')",
                    crate::lexer::token_word_refs(tokens).join(" ")
                ))
            })?;
        (pips, &tokens[tokens.len() - stream.eof_offset()..])
    };
    // "you may pay {1} to draw a card": a purpose clause after the payment.
    // The leading-pip reading would keep only the payment and silently drop
    // what it buys.
    if rest.iter().find_map(OwnedLexToken::as_word) == Some("to") {
        return Err(CardTextError::ParseError(format!(
            "unsupported pay-to clause; write \"pay ... . If you do, ...\" instead (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    Ok(EffectAst::subject_verb_pay_mana(
        player,
        ManaCost::from_pips(pips),
    ))
}

/// "{G}, {W}, or {U}": a disjunctive list of single mana symbols.
fn single_symbol_payment_alternatives(
    tokens: &[OwnedLexToken],
) -> Option<Vec<crate::mana::ManaSymbol>> {
    let tokens = crate::util::trim_edge_punctuation_tokens(tokens);
    let mut symbols = Vec::new();
    let mut saw_or = false;
    let mut after_separator = true;
    for token in tokens {
        if token.is_comma() {
            if after_separator {
                return None;
            }
            after_separator = true;
            continue;
        }
        if token.is_word("or") {
            if symbols.is_empty() || saw_or {
                return None;
            }
            saw_or = true;
            after_separator = true;
            continue;
        }
        if !after_separator {
            return None;
        }
        let pip = crate::grammar::leaf::parse_leaf_surface_mana_pip_token(token)?;
        let crate::grammar::leaf::LeafManaPipToken::ManaGroup(group) = pip else {
            return None;
        };
        let [symbol] = group.as_slice() else {
            return None;
        };
        symbols.push(*symbol);
        after_separator = false;
    }
    (saw_or && !after_separator && symbols.len() >= 2).then_some(symbols)
}
