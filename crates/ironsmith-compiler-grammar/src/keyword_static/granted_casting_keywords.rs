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
//! - sneak {cost} (CR 702.190a) is a named alternative cost from hand. It may
//!   be granted to cards in the hand or the graveyard ("Creature cards in your
//!   graveyard have sneak {3}{B}.", Ninja Teen); a graveyard grant is usable
//!   only under a separate "cast ... from your graveyard using their sneak
//!   abilities" permission (`parse_cast_from_zone_using_keyword_abilities_line`).
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
    Sneak,
}

impl GrantedCastingKeyword {
    fn allows_zone(self, zone: Zone) -> bool {
        match self {
            Self::JumpStart => zone == Zone::Graveyard,
            Self::Sneak => matches!(zone, Zone::Hand | Zone::Graveyard),
            Self::Warp | Self::Prowl | Self::Freerunning | Self::Miracle => zone == Zone::Hand,
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
    } else if first.is_word("sneak") {
        Some(GrantedCastingKeyword::Sneak)
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
            descriptive: false,
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
        GrantedCastingKeyword::Sneak => Method::alternative_cost(
            "Sneak",
            Some(cost?),
            vec![crate::model::CompilerCost::Sneak],
        ),
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
    let mut filter = parse_object_filter_lexed(subject, false)?;
    // A single terminal zone phrase scopes the complete card list, including
    // repeated nouns: "Artifact cards and red creature cards in your hand".
    // Do not merge lists whose arms explicitly name different zones.
    let words = parser_token_word_refs(subject);
    let zone_words = words.iter().filter(|word| matches!(**word,
        "hand" | "graveyard" | "library" | "exile" | "battlefield" | "command")).count();
    if filter.zone.is_none() && filter.any_of.len() >= 2 && zone_words == 1
        && let Some(last) = filter.any_of.last()
        && let Some(zone) = last.zone.filter(|zone| *zone != Zone::Battlefield)
        && filter.any_of[..filter.any_of.len() - 1].iter().all(|arm| {
            matches!(arm.zone, None | Some(Zone::Battlefield))
                && arm.owner.is_none() && arm.controller.is_none()
        })
    {
        filter.zone = Some(zone);
        filter.owner = last.owner.clone();
        for arm in &mut filter.any_of {
            arm.zone = None;
            arm.owner = None;
        }
    }
    let Some((filter, zone)) = granted_subject_card_filter(filter) else {
        return Ok(None);
    };
    if !keyword.allows_zone(zone) {
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

/// "You may cast creature spells from your graveyard using their sneak
/// abilities." (Ninja Teen): matching cards in that zone may be cast with
/// that keyword's alternative cost, printed or granted (CR 601.2, 702.190a).
pub fn parse_cast_from_zone_using_keyword_abilities_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    use ironsmith_core::alternative_cast_model::AlternativeCastKeyword;
    let tokens = trim_edge_punctuation(tokens);
    let words = parser_token_word_refs(&tokens);
    if !crate::word_primitives::parse_sequence_prefix(&words, &["you", "may", "cast"]) {
        return Ok(None);
    }
    let Some(from_idx) = tokens.iter().position(|token| token.is_word("from")) else {
        return Ok(None);
    };
    if from_idx <= 3 {
        return Ok(None);
    }
    let tail = parser_token_word_refs(&tokens[from_idx..]);
    let (zone, keyword_word) = match tail.as_slice() {
        ["from", "your", "graveyard", "using", "their", keyword, "abilities" | "ability"] => {
            (Zone::Graveyard, *keyword)
        }
        ["from", "your", "hand", "using", "their", keyword, "abilities" | "ability"] => {
            (Zone::Hand, *keyword)
        }
        _ => return Ok(None),
    };
    let method = match keyword_word {
        "sneak" => AlternativeCastKeyword::Sneak,
        "blitz" => AlternativeCastKeyword::Blitz,
        "warp" => AlternativeCastKeyword::Warp,
        "bestow" => AlternativeCastKeyword::Bestow,
        _ => return Ok(None),
    };
    let mut filter = parse_object_filter_lexed(&tokens[3..from_idx], false)?;
    // "creature spells" names the cards those spells come from.
    filter.zone = None;
    filter.stack_kind = None;
    filter.cast_by = None;
    let display = crate::lexer::render_token_slice(&tokens)
        .trim()
        .trim_end_matches('.')
        .to_string();
    Ok(Some(StaticAbility::alternative_cast_from_zone_for_filter(
        filter, zone, method, display,
    )))
}

const MADNESS_MANA_COST_TAIL: &[&str] = &[
    "the", "madness", "cost", "is", "equal", "to", "its", "mana", "cost",
];
const NOT_ON_BATTLEFIELD_TAILS: &[&[&str]] = &[
    &["that", "isn't", "on", "the", "battlefield"],
    &["that", "isnt", "on", "the", "battlefield"],
    &["that", "is", "not", "on", "the", "battlefield"],
];

/// "Each Vampire creature card you own that isn't on the battlefield has
/// madness. The madness cost is equal to its mana cost." (Falkenrath Gorger)
///
/// Madness (CR 702.35a) is a discard replacement that functions from the hand
/// plus a linked trigger that casts the card from exile. The grant is the
/// card's madness casting method, derived from its mana cost, recorded in
/// every zone the card can be in other than the battlefield where that
/// method matters: hand (the discard replacement), exile (the trigger's cast),
/// graveyard, library, and command ("if it has madness" checks).
pub fn parse_granted_madness_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let tokens = trim_edge_punctuation(tokens);
    let Some(has_idx) = tokens
        .iter()
        .position(|token| token.is_any_word(&["have", "has"]))
    else {
        return Ok(None);
    };
    if !tokens
        .get(has_idx + 1)
        .is_some_and(|token| token.is_word("madness"))
    {
        return Ok(None);
    }
    let tail_words = parser_token_word_refs(&tokens[has_idx + 2..]);
    if !crate::word_primitives::parse_sequence_complete(&tail_words, MADNESS_MANA_COST_TAIL) {
        return Ok(None);
    }
    let mut subject = &tokens[..has_idx];
    if subject.first().is_some_and(|token| token.is_word("each")) {
        subject = &subject[1..];
    }
    // "... that isn't on the battlefield" scopes the grant to the zones
    // listed above; the remaining words name the cards.
    let Some(that_idx) = subject.iter().rposition(|token| token.is_word("that")) else {
        return Ok(None);
    };
    let scope_words = parser_token_word_refs(&subject[that_idx..]);
    if !NOT_ON_BATTLEFIELD_TAILS.iter()
        .any(|tail| crate::word_primitives::parse_sequence_complete(&scope_words, tail))
    {
        return Ok(None);
    }
    let mut filter = parse_object_filter_lexed(&subject[..that_idx], false)?;
    // Creature filters default to battlefield. This complete explicit scope
    // replaces that default with the nonbattlefield grant zones below.
    filter.zone = None;
    let mut spec = crate::model::CompilerGrantSpecCore::new(
        crate::model::CompilerGrantableCore::DerivedAlternativeCast(
            ironsmith_core::DerivedAlternativeCast::MadnessFromCardManaCost,
        ),
        filter,
        Zone::Hand,
    );
    spec.additional_zones = vec![Zone::Exile, Zone::Graveyard, Zone::Library, Zone::Command];
    Ok(Some(StaticAbility::grants(spec)))
}
