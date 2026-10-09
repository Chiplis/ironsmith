//! Encore granted to graveyard cards with a cost derived from each card
//! (CR 702.141): "Each artifact creature card in your graveyard has encore.
//! Its encore cost is equal to its mana cost." (Wire Surgeons) and "Each
//! outlaw creature card in your graveyard has encore {X}, where X is its mana
//! value." (Graywater's Fixer, Sliver Gravemother). Encore functions from the
//! graveyard, so only graveyard-scoped subjects are claimed.

use super::*;

const MANA_COST_TAIL: &[&str] = &["its", "encore", "cost", "is", "equal", "to", "its", "mana", "cost"];
const MANA_VALUE_TAIL: &[&str] = &["where", "x", "is", "its", "mana", "value"];

pub fn parse_granted_encore_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let tokens = trim_edge_punctuation(tokens);
    let Some(encore_idx) = tokens.iter().position(|token| token.is_word("encore")) else {
        return Ok(None);
    };
    if encore_idx < 2 || !tokens[encore_idx - 1].is_any_word(&["have", "has"]) {
        return Ok(None);
    }
    let tail = &tokens[encore_idx + 1..];
    let mana_value_generic = if crate::word_primitives::parse_sequence_complete(
        &parser_token_word_refs(tail),
        MANA_COST_TAIL,
    ) {
        false
    } else {
        let Some(mana) = parse_leaf_mana_cost_prefix_tokens(tail) else {
            return Ok(None);
        };
        let is_bare_x = mana.cost.pips() == [vec![crate::mana::ManaSymbol::X]];
        if !is_bare_x
            || !crate::word_primitives::parse_sequence_complete(
                &parser_token_word_refs(&tail[mana.consumed..]),
                MANA_VALUE_TAIL,
            )
        {
            return Ok(None);
        }
        true
    };
    let mut subject = &tokens[..encore_idx - 1];
    if subject.first().is_some_and(|token| token.is_word("each")) {
        subject = &subject[1..];
    }
    let filter = parse_object_filter_lexed(subject, false)?;
    if filter.zone != Some(Zone::Graveyard) {
        return Ok(None);
    }
    Ok(Some(StaticAbilityAst::GrantKeywordAction {
        filter,
        action: KeywordAction::EncoreFromSourceCost { mana_value_generic },
        condition: None,
    }))
}
