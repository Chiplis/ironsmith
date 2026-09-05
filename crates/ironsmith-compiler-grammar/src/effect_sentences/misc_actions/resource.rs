use super::*;

use crate::recognition::ParseOutcome;
#[path = "resource/pay_readings.rs"]
mod pay_readings;

pub fn parse_pay(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);
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
    let pips = {
        use winnow::prelude::*;
        let mut stream = LexStream::new(tokens);
        grammar::collect_mana_pip_groups
            .parse_next(&mut stream)
            .map_err(|_| {
                CardTextError::ParseError(format!(
                    "missing payment cost (clause: '{}')",
                    crate::lexer::token_word_refs(tokens).join(" ")
                ))
            })?
    };

    Ok(EffectAst::subject_verb_pay_mana(
        player,
        ManaCost::from_pips(pips),
    ))
}
