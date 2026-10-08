//! "Commander ninjutsu {U}{B}" (Yuriko, the Tiger's Shadow): ninjutsu that
//! also functions while the card is in the command zone (CR 702.49d). The
//! grammar emits the keyword action; lowering builds the activated ability.

use super::*;

pub fn parse_commander_ninjutsu_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let tokens = trim_edge_punctuation(tokens);
    let [commander, ninjutsu, cost_tokens @ ..] = tokens.as_slice() else {
        return Ok(None);
    };
    if !commander.is_word("commander") || !ninjutsu.is_word("ninjutsu") {
        return Ok(None);
    }
    let Some(mana) = parse_leaf_mana_cost_prefix_tokens(cost_tokens) else {
        return Ok(None);
    };
    if mana.consumed != cost_tokens.len() {
        return Ok(None);
    }
    Ok(Some(StaticAbilityAst::KeywordAction(
        KeywordAction::CommanderNinjutsu(mana.cost),
    )))
}
