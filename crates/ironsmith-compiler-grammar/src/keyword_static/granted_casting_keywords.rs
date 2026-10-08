//! One shared path for casting keywords granted to cards or spells:
//! "<subject> have|has <keyword> [cost]".
//!
//! - warp {cost} (CR 702.185), prowl {cost} (CR 702.76), freerunning {cost}
//!   (CR 702.173) and miracle {cost} (CR 702.94) are alternative costs paid
//!   while casting from hand. Grants of them are recorded in the hand only,
//!   where the card may already be cast, so no new zone permission arises.
//!   "<X> spells you cast have prowl {2}{R}" (Hunting Velociraptor) names the
//!   cards those spells come from; the grant is attached to those cards in hand.
//! - jump-start (CR 702.133) is a graveyard cast; its grant lives in the
//!   graveyard ("Each instant and sorcery card in your graveyard that's exactly
//!   two colors has jump-start.", Niv-Mizzet, Supreme).
//!
//! The engine already routes granted alternative casts through
//! `resolve_play_from_alternative_method`, so method-keyed resolution (warp's
//! end-step exile, jump-start's exile, miracle's draw trigger, prowl and
//! freerunning conditions) applies to granted casts as to printed ones.

use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GrantedCastingKeyword {
    Warp,
    Prowl,
    Freerunning,
    Miracle,
    JumpStart,
}

impl GrantedCastingKeyword {
    fn zone(self) -> Zone {
        match self {
            Self::JumpStart => Zone::Graveyard,
            Self::Warp | Self::Prowl | Self::Freerunning | Self::Miracle => Zone::Hand,
        }
    }

    fn takes_mana_cost(self) -> bool {
        !matches!(self, Self::JumpStart)
    }
}

/// The keyword at `tokens[0..]` and the number of tokens it spans.
fn granted_casting_keyword(tokens: &[OwnedLexToken]) -> Option<(GrantedCastingKeyword, usize)> {
    let first = tokens.first()?;
    let single = if first.is_word("warp") {
        Some(GrantedCastingKeyword::Warp)
    } else if first.is_word("prowl") {
        Some(GrantedCastingKeyword::Prowl)
    } else if first.is_word("freerunning") {
        Some(GrantedCastingKeyword::Freerunning)
    } else if first.is_word("miracle") {
        Some(GrantedCastingKeyword::Miracle)
    } else if first.is_any_word(&["jump-start", "jumpstart"]) {
        Some(GrantedCastingKeyword::JumpStart)
    } else {
        None
    };
    if let Some(keyword) = single {
        return Some((keyword, 1));
    }
    (first.is_word("jump") && tokens.get(1).is_some_and(|token| token.is_word("start")))
        .then_some((GrantedCastingKeyword::JumpStart, 2))
}

fn granted_casting_method(
    keyword: GrantedCastingKeyword,
    cost: Option<crate::mana::ManaCost>,
) -> Option<crate::model::CompilerAlternativeCastingMethod> {
    use crate::model::CompilerAlternativeCastingMethod as Method;
    Some(match keyword {
        GrantedCastingKeyword::Warp => Method::Warp {
            cost: cost?,
            additional_cost: ironsmith_core::TotalCost::free(),
        },
        GrantedCastingKeyword::Prowl => Method::Composed {
            name: "Prowl".into(),
            total_cost: ironsmith_core::TotalCost::mana(cost?),
            // CR 702.76a: any of the spell's own creature types.
            condition: Some(
                crate::static_abilities::ThisSpellCostCondition::YouDealtCombatDamageToPlayerSharingCreatureTypeThisTurn,
            ),
            prototype_power_toughness: None,
        },
        GrantedCastingKeyword::Freerunning => Method::alternative_cost_with_condition(
            "Freerunning",
            Some(cost?),
            Vec::new(),
            crate::static_abilities::ThisSpellCostCondition::YouDealtCombatDamageToPlayerWithSubtypeOrCommanderThisTurn(
                crate::types::Subtype::Assassin,
            ),
        ),
        GrantedCastingKeyword::Miracle => Method::Miracle { cost: cost? },
        GrantedCastingKeyword::JumpStart => Method::JumpStart {
            additional_cost: ironsmith_core::TotalCost::from_cost(
                crate::model::CompilerCost::Discard {
                    count: 1,
                    card_types: Vec::new(),
                    supertypes: Vec::new(),
                    filter: None,
                    random: false,
                    name: None,
                    other: false,
                    binding: None,
                },
            ),
        },
    })
}

/// The zone the subject's cards are in, with that zone (and any spell-only
/// qualifiers) removed from the card filter.
fn granted_subject_card_filter(mut filter: ObjectFilter) -> Option<(ObjectFilter, Zone)> {
    let is_spell_subject = filter.zone == Some(Zone::Stack) || filter.stack_kind.is_some();
    if is_spell_subject {
        filter.zone = None;
        filter.stack_kind = None;
        filter.cast_by = None;
        return Some((filter, Zone::Hand));
    }
    let zone = match filter.zone {
        Some(zone) => zone,
        None => {
            let first = filter.any_of.first()?.zone?;
            if !filter.any_of.iter().all(|branch| branch.zone == Some(first)) {
                return None;
            }
            first
        }
    };
    filter.zone = None;
    for branch in &mut filter.any_of {
        branch.zone = None;
    }
    Some((filter, zone))
}

pub fn parse_granted_casting_keyword_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let tokens = trim_edge_punctuation(tokens);
    let Some(have_idx) = tokens
        .iter()
        .position(|token| token.is_any_word(&["have", "has"]))
    else {
        return Ok(None);
    };
    if have_idx == 0 {
        return Ok(None);
    }
    let Some((keyword, keyword_len)) = granted_casting_keyword(&tokens[have_idx + 1..]) else {
        return Ok(None);
    };
    let tail = &tokens[have_idx + 1 + keyword_len..];
    let cost = if keyword.takes_mana_cost() {
        let Some(mana) = parse_leaf_mana_cost_prefix_tokens(tail) else {
            return Ok(None);
        };
        if mana.consumed != tail.len() {
            return Ok(None);
        }
        Some(mana.cost)
    } else {
        if !tail.is_empty() {
            return Ok(None);
        }
        None
    };
    let mut subject = &tokens[..have_idx];
    if subject.first().is_some_and(|token| token.is_word("each")) {
        subject = &subject[1..];
    }
    let filter = parse_object_filter_lexed(subject, false)?;
    let Some((filter, zone)) = granted_subject_card_filter(filter) else {
        return Ok(None);
    };
    if zone != keyword.zone() {
        return Ok(None);
    }
    let Some(method) = granted_casting_method(keyword, cost) else {
        return Ok(None);
    };
    let spec = crate::model::CompilerGrantSpecCore::new(
        crate::model::CompilerGrantableCore::AlternativeCast(method),
        filter,
        zone,
    );
    Ok(Some(StaticAbility::grants(spec)))
}
