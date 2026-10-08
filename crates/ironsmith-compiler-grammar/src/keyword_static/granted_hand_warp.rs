//! "Artifact cards and red creature cards in your hand have warp {2}{R}."
//! (Tannuk, Steadfast Second): warp (CR 702.185) granted to cards in your
//! hand with an explicit cost. The grant is an alternative cast from the hand,
//! where the cards may already be cast, so it adds no new zone permission;
//! the engine's warp resolution (end-step exile and later recast) is keyed on
//! the announced method and applies to granted casts.

use super::*;

pub fn parse_granted_hand_warp_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let tokens = trim_edge_punctuation(tokens);
    let Some(warp_idx) = tokens.iter().position(|token| token.is_word("warp")) else {
        return Ok(None);
    };
    if warp_idx < 2 || !tokens[warp_idx - 1].is_any_word(&["have", "has"]) {
        return Ok(None);
    }
    let Some(mana) = parse_leaf_mana_cost_prefix_tokens(&tokens[warp_idx + 1..]) else {
        return Ok(None);
    };
    if warp_idx + 1 + mana.consumed != tokens.len() {
        return Ok(None);
    }
    let subject = &tokens[..warp_idx - 1];
    let mut filter = parse_object_filter_lexed(subject, false)?;
    let branches_in_hand = !filter.any_of.is_empty()
        && filter.any_of.iter().all(|branch| branch.zone == Some(Zone::Hand));
    if filter.zone != Some(Zone::Hand) && !branches_in_hand {
        return Ok(None);
    }
    filter.zone = None;
    for branch in &mut filter.any_of {
        branch.zone = None;
    }
    let method = crate::model::CompilerAlternativeCastingMethod::Warp {
        cost: mana.cost,
        additional_cost: ironsmith_core::TotalCost::free(),
    };
    let spec = crate::model::CompilerGrantSpecCore::new(
        crate::model::CompilerGrantableCore::AlternativeCast(method),
        filter,
        Zone::Hand,
    );
    Ok(Some(StaticAbility::grants(spec)))
}
