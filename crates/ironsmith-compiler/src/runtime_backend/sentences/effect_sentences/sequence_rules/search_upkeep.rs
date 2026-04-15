use super::super::super::grammar::primitives as grammar;
use super::super::super::util::{mana_pips_from_token, token_index_for_word_index};
use super::super::{parse_effect_chain, trim_commas};
use crate::cards::builders::{CardTextError, EffectAst, PlayerAst};
use crate::mana::ManaSymbol;

pub(super) fn try_parse(
    first: &[crate::cards::builders::OwnedLexToken],
    second: &[crate::cards::builders::OwnedLexToken],
    third: &[crate::cards::builders::OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Ok(first_effects) = parse_effect_chain(first) else {
        return Ok(None);
    };
    if first_effects.is_empty()
        || grammar::words_match_prefix(first, &["search", "your", "library"]).is_none()
    {
        return Ok(None);
    }

    let upkeep_tokens = trim_commas(second);
    let pay_idx = if grammar::words_match_prefix(
        &upkeep_tokens,
        &[
            "at",
            "the",
            "beginning",
            "of",
            "your",
            "next",
            "upkeep",
            "pay",
        ],
    )
    .is_some()
    {
        7usize
    } else if grammar::words_match_prefix(
        &upkeep_tokens,
        &[
            "at",
            "the",
            "beginning",
            "of",
            "the",
            "next",
            "upkeep",
            "pay",
        ],
    )
    .is_some()
    {
        7usize
    } else {
        return Ok(None);
    };
    let Some(pay_token_idx) = token_index_for_word_index(&upkeep_tokens, pay_idx) else {
        return Ok(None);
    };
    let mana_tokens = trim_commas(&upkeep_tokens[pay_token_idx + 1..]);
    if mana_tokens.is_empty() {
        return Ok(None);
    }

    let mut mana = Vec::<ManaSymbol>::new();
    for token in mana_tokens {
        if let Some(pips) = mana_pips_from_token(&token) {
            mana.extend(pips);
            continue;
        }
        let Some(word) = token.as_word() else {
            continue;
        };
        if let Ok(generic) = word.parse::<u8>() {
            mana.push(ManaSymbol::Generic(generic));
            continue;
        }
        return Ok(None);
    }
    if mana.is_empty() {
        return Ok(None);
    }

    let lose_tokens = trim_commas(third);
    let lose_words = crate::cards::builders::compiler::token_word_refs(&lose_tokens);
    let valid_lose_clause = lose_words == ["if", "you", "dont", "you", "lose", "the", "game"]
        || lose_words == ["if", "you", "don't", "you", "lose", "the", "game"]
        || lose_words == ["if", "you", "do", "not", "you", "lose", "the", "game"];
    if !valid_lose_clause {
        return Ok(None);
    }

    let mut effects = first_effects;
    effects.push(EffectAst::DelayedUntilNextUpkeep {
        player: PlayerAst::You,
        effects: vec![EffectAst::UnlessPays {
            effects: vec![EffectAst::LoseGame {
                player: PlayerAst::You,
            }],
            player: PlayerAst::You,
            mana,
        }],
    });
    Ok(Some(effects))
}
