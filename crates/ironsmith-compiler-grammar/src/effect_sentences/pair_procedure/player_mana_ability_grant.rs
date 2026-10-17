//! "Until end of turn, any time you could activate a mana ability, you may
//! pay 1 life." followed by "If you do, add {C}." (Channel): the two
//! sentences together grant the player a mana ability of their own for the
//! rest of the turn (CR 605.1a), whose cost is the life payment and whose
//! effect is the mana added. Neither sentence is an instruction performed
//! while the spell resolves.
use super::*;

const OPENING: &[&str] = &[
    "until", "end", "of", "turn", "any", "time", "you", "could", "activate", "a", "mana",
    "ability", "you", "may", "pay",
];

pub(crate) fn read(
    sentences: &[SentenceInput],
    index: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let (Some(first), Some(second)) = (sentences.get(index), sentences.get(index + 1)) else {
        return Ok(None);
    };
    let first_words = crate::lexer::parser_token_word_refs(first.lowered());
    let Some(payment) = first_words.strip_prefix(OPENING) else {
        return Ok(None);
    };
    let [amount, "life"] = payment else {
        return Ok(None);
    };
    let Some(life_cost) = crate::util::parse_number_word_u32(amount) else {
        return Ok(None);
    };
    let second_tokens = second.lowered();
    let Some(add_idx) = second_tokens.iter().position(|token| token.is_word("add")) else {
        return Ok(None);
    };
    let head = crate::lexer::parser_token_word_refs(&second_tokens[..add_idx]);
    if head != ["if", "you", "do"] {
        return Ok(None);
    }
    let mut mana = Vec::new();
    for token in &second_tokens[add_idx + 1..] {
        if token.is_comma() || token.is_period() {
            continue;
        }
        let Some(inner) = token.mana_group_inner() else {
            return Ok(None);
        };
        let Ok(symbol) = crate::grammar::values::parse_mana_symbol(inner) else {
            return Ok(None);
        };
        mana.push(symbol);
    }
    if mana.is_empty() {
        return Ok(None);
    }
    Ok(Some(vec![EffectAst::GrantPayLifeManaAbilityUntilEndOfTurn(
        ironsmith_core::GrantPayLifeManaAbilityUntilEndOfTurnEffect::new(
            PlayerFilter::You,
            life_cost,
            mana,
        ),
    )]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channel_sentences_grant_a_pay_life_mana_ability_until_end_of_turn() {
        let tokens = crate::lexer::lex_line(
            "Until end of turn, any time you could activate a mana ability, you may pay 1 life. If you do, add {C}.",
            0,
        )
        .unwrap();
        let effects = crate::effect_sentences::parse_effect_sentences_lexed(&tokens).unwrap();
        assert_eq!(
            effects,
            vec![EffectAst::GrantPayLifeManaAbilityUntilEndOfTurn(
                ironsmith_core::GrantPayLifeManaAbilityUntilEndOfTurnEffect::new(
                    PlayerFilter::You,
                    1,
                    vec![crate::mana::ManaSymbol::Colorless],
                ),
            )]
        );
    }
}
