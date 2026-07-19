use winnow::error::{ModalResult as WResult, StrContext, StrContextValue};
use winnow::prelude::*;
use winnow::token::{literal, rest};

use crate::cards::builders::CardTextError;
use crate::color::{Color, ColorSet};
use crate::types::{CardType, Subtype, SubtypeFamily, Supertype};
use crate::zone::Zone;

use super::super::primitives;
use super::common::finish_text_parse;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LeafDemonstrativeObjectHead {
    CardType(CardType),
    Permanent,
    Card,
    Spell,
    Source,
    Token,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LeafObjectReferenceHead {
    Demonstrative(LeafDemonstrativeObjectHead),
    Subtype(Subtype),
}

pub(crate) fn parse_leaf_card_type(input: &mut &str) -> WResult<CardType> {
    let raw = rest.parse_next(input)?;
    classify_card_type(raw).ok_or_else(|| {
        primitives::backtrack_err(
            "card type",
            "artifact, creature, enchantment, or other card type",
        )
    })
}

pub(crate) fn parse_leaf_card_type_complete(raw: &str) -> Result<CardType, CardTextError> {
    finish_text_parse(raw, parse_leaf_card_type, "leaf-card-type")
}

pub(crate) fn parse_leaf_supertype(input: &mut &str) -> WResult<Supertype> {
    let raw = rest.parse_next(input)?;
    classify_supertype(raw)
        .ok_or_else(|| primitives::backtrack_err("supertype", "basic, legendary, snow, or world"))
}

pub(crate) fn parse_leaf_supertype_complete(raw: &str) -> Result<Supertype, CardTextError> {
    finish_text_parse(raw, parse_leaf_supertype, "leaf-supertype")
}

pub(crate) fn parse_leaf_subtype(input: &mut &str) -> WResult<Subtype> {
    let raw = rest.parse_next(input)?;
    classify_subtype_word(raw)
        .ok_or_else(|| primitives::backtrack_err("subtype", "known Magic subtype"))
}

pub(crate) fn parse_leaf_subtype_complete(raw: &str) -> Result<Subtype, CardTextError> {
    finish_text_parse(raw, parse_leaf_subtype, "leaf-subtype")
}

pub(crate) fn parse_leaf_subtype_flexible(input: &mut &str) -> WResult<Subtype> {
    let raw = rest.parse_next(input)?;
    classify_flexible_subtype(raw).ok_or_else(|| {
        primitives::backtrack_err("flexible subtype", "known singular or plural subtype")
    })
}

pub(crate) fn parse_leaf_subtype_flexible_complete(raw: &str) -> Result<Subtype, CardTextError> {
    finish_text_parse(raw, parse_leaf_subtype_flexible, "leaf-flexible-subtype")
}

pub(crate) fn parse_leaf_color(input: &mut &str) -> WResult<ColorSet> {
    let raw = rest.parse_next(input)?;
    Color::from_name(raw)
        .map(ColorSet::from_color)
        .ok_or_else(|| primitives::backtrack_err("color", "white, blue, black, red, or green"))
}

pub(crate) fn parse_leaf_color_complete(raw: &str) -> Result<ColorSet, CardTextError> {
    finish_text_parse(raw, parse_leaf_color, "leaf-color")
}

pub(crate) fn parse_leaf_non_card_type(input: &mut &str) -> WResult<CardType> {
    literal("non").parse_next(input)?;
    parse_leaf_card_type
        .context(StrContext::Label("non-card-type descriptor"))
        .context(StrContext::Expected(StrContextValue::Description(
            "non followed by a card type",
        )))
        .parse_next(input)
}

pub(crate) fn parse_leaf_non_card_type_complete(raw: &str) -> Result<CardType, CardTextError> {
    finish_text_parse(raw, parse_leaf_non_card_type, "leaf-non-card-type")
}

pub(crate) fn parse_leaf_non_supertype(input: &mut &str) -> WResult<Supertype> {
    literal("non").parse_next(input)?;
    parse_leaf_supertype
        .context(StrContext::Label("non-supertype descriptor"))
        .parse_next(input)
}

pub(crate) fn parse_leaf_non_supertype_complete(raw: &str) -> Result<Supertype, CardTextError> {
    finish_text_parse(raw, parse_leaf_non_supertype, "leaf-non-supertype")
}

pub(crate) fn parse_leaf_non_color(input: &mut &str) -> WResult<ColorSet> {
    literal("non").parse_next(input)?;
    parse_leaf_color
        .context(StrContext::Label("non-color descriptor"))
        .parse_next(input)
}

pub(crate) fn parse_leaf_non_color_complete(raw: &str) -> Result<ColorSet, CardTextError> {
    finish_text_parse(raw, parse_leaf_non_color, "leaf-non-color")
}

pub(crate) fn parse_leaf_non_subtype(input: &mut &str) -> WResult<Subtype> {
    literal("non").parse_next(input)?;
    parse_leaf_subtype_flexible
        .context(StrContext::Label("non-subtype descriptor"))
        .parse_next(input)
}

pub(crate) fn parse_leaf_non_subtype_complete(raw: &str) -> Result<Subtype, CardTextError> {
    finish_text_parse(raw, parse_leaf_non_subtype, "leaf-non-subtype")
}

pub(crate) fn parse_leaf_zone(input: &mut &str) -> WResult<Zone> {
    let raw = rest.parse_next(input)?;
    classify_zone(raw).ok_or_else(|| primitives::backtrack_err("zone", "named game zone"))
}

pub(crate) fn parse_leaf_zone_complete(raw: &str) -> Result<Zone, CardTextError> {
    finish_text_parse(raw, parse_leaf_zone, "leaf-zone")
}

pub(crate) fn parse_leaf_demonstrative_object_head(
    input: &mut &str,
) -> WResult<LeafDemonstrativeObjectHead> {
    let raw = rest.parse_next(input)?;
    classify_demonstrative_object_head(raw).ok_or_else(|| {
        primitives::backtrack_err(
            "object head",
            "card, permanent, spell, source, token, or card type",
        )
    })
}

pub(crate) fn parse_leaf_demonstrative_object_head_complete(
    raw: &str,
) -> Result<LeafDemonstrativeObjectHead, CardTextError> {
    finish_text_parse(
        raw,
        parse_leaf_demonstrative_object_head,
        "leaf-demonstrative-object-head",
    )
}

pub(crate) fn parse_leaf_object_reference_head(
    input: &mut &str,
) -> WResult<LeafObjectReferenceHead> {
    let raw = rest.parse_next(input)?;
    if let Some(head) = classify_demonstrative_object_head(raw) {
        return Ok(LeafObjectReferenceHead::Demonstrative(head));
    }
    classify_flexible_subtype(raw)
        .map(LeafObjectReferenceHead::Subtype)
        .ok_or_else(|| {
            primitives::backtrack_err(
                "object reference head",
                "card, permanent, spell, source, token, card type, or subtype",
            )
        })
}

pub(crate) fn parse_leaf_object_reference_head_complete(
    raw: &str,
) -> Result<LeafObjectReferenceHead, CardTextError> {
    finish_text_parse(
        raw,
        parse_leaf_object_reference_head,
        "leaf-object-reference-head",
    )
}

fn classify_card_type(raw: &str) -> Option<CardType> {
    match raw {
        "creature" | "creatures" => Some(CardType::Creature),
        "artifact" | "artifacts" => Some(CardType::Artifact),
        "enchantment" | "enchantments" => Some(CardType::Enchantment),
        "land" | "lands" => Some(CardType::Land),
        "planeswalker" | "planeswalkers" => Some(CardType::Planeswalker),
        "instant" | "instants" => Some(CardType::Instant),
        "sorcery" | "sorceries" => Some(CardType::Sorcery),
        "battle" | "battles" => Some(CardType::Battle),
        "kindred" => Some(CardType::Kindred),
        _ => None,
    }
}

fn classify_supertype(raw: &str) -> Option<Supertype> {
    match normalized_atom_word(raw).as_str() {
        "basic" => Some(Supertype::Basic),
        "legendary" => Some(Supertype::Legendary),
        "snow" => Some(Supertype::Snow),
        "world" => Some(Supertype::World),
        _ => None,
    }
}

fn classify_subtype_word(raw: &str) -> Option<Subtype> {
    let candidate = normalized_atom_word(raw);
    if candidate.is_empty() {
        return None;
    }

    match candidate.as_str() {
        "mice" => return Some(Subtype::Mouse),
        "ouphe" => return Some(Subtype::Ouphe),
        "oxen" => return Some(Subtype::Ox),
        "spacecraft" => return Some(Subtype::Spacecraft),
        _ => {}
    }

    // Ordinary English words that are also (obscure) subtypes. In rules text
    // these are essentially always the English noun ("the number of times
    // this ability has resolved", "your plan", "seal it"), never the subtype;
    // printed type lines don't parse through this path, so the subtypes still
    // work where they actually appear. Cover plural surfaces too — the family
    // matcher below accepts base+"s"/"es" forms.
    for stem in [
        Some(candidate.as_str()),
        candidate.strip_suffix('s'),
        candidate.strip_suffix("es"),
    ]
    .into_iter()
    .flatten()
    {
        if matches!(
            stem,
            "time"
                | "blood"
                | "carrier"
                | "child"
                | "chorus"
                | "eternal"
                | "lord"
                | "mine"
                | "omen"
                | "plan"
                | "sand"
                | "seal"
                | "sphere"
                | "spy"
                | "stone"
                | "tower"
        ) {
            return None;
        }
    }

    for family in [
        SubtypeFamily::Land,
        SubtypeFamily::Creature,
        SubtypeFamily::Artifact,
        SubtypeFamily::Enchantment,
        SubtypeFamily::Spell,
        SubtypeFamily::Planeswalker,
        SubtypeFamily::Battle,
    ] {
        for subtype in family.all_subtypes() {
            if subtype_surface_matches(*subtype, candidate.as_str()) {
                return Some(*subtype);
            }
        }
    }

    None
}

fn classify_flexible_subtype(raw: &str) -> Option<Subtype> {
    let candidate = normalized_atom_word(raw);
    classify_subtype_word(candidate.as_str())
        .or_else(|| stem_before_tail(candidate.as_str(), b"s").and_then(classify_subtype_word))
        .or_else(|| stem_before_tail(candidate.as_str(), b"es").and_then(classify_subtype_word))
        .or_else(|| {
            stem_before_tail(candidate.as_str(), b"ves")
                .and_then(|stem| classify_subtype_word(format!("{stem}f").as_str()))
        })
        .or_else(|| {
            stem_before_tail(candidate.as_str(), b"ves")
                .and_then(|stem| classify_subtype_word(format!("{stem}fe").as_str()))
        })
}

fn subtype_surface_matches(subtype: Subtype, candidate: &str) -> bool {
    let base = normalized_atom_word(&subtype.to_string());
    if base.is_empty() {
        return false;
    }
    if candidate == base {
        return true;
    }
    if let Some(stem) = stem_before_tail(base.as_str(), b"y")
        && candidate == format!("{stem}ies")
    {
        return true;
    }
    if let Some(stem) = stem_before_tail(base.as_str(), b"fe")
        && candidate == format!("{stem}ves")
    {
        return true;
    }
    if let Some(stem) = stem_before_tail(base.as_str(), b"f")
        && candidate == format!("{stem}ves")
    {
        return true;
    }
    candidate == format!("{base}s")
}

fn normalized_atom_word(raw: &str) -> String {
    raw.chars()
        .filter_map(|ch| match ch {
            '\'' | '’' | '-' => None,
            _ if ch.is_ascii_alphanumeric() => Some(ch.to_ascii_lowercase()),
            _ => None,
        })
        .collect()
}

fn stem_before_tail<'a>(word: &'a str, tail: &[u8]) -> Option<&'a str> {
    let bytes = word.as_bytes();
    let head_len = bytes.len().checked_sub(tail.len())?;
    if bytes.get(head_len..)? != tail {
        return None;
    }
    word.get(..head_len)
}

fn classify_zone(raw: &str) -> Option<Zone> {
    match raw {
        "battlefield" => Some(Zone::Battlefield),
        "graveyard" | "graveyards" => Some(Zone::Graveyard),
        "hand" | "hands" => Some(Zone::Hand),
        "library" | "libraries" => Some(Zone::Library),
        "exile" | "exiled" => Some(Zone::Exile),
        "stack" => Some(Zone::Stack),
        _ => None,
    }
}

fn classify_demonstrative_object_head(raw: &str) -> Option<LeafDemonstrativeObjectHead> {
    if let Some(card_type) = classify_card_type(raw) {
        return Some(LeafDemonstrativeObjectHead::CardType(card_type));
    }
    let generic = match raw {
        "permanent" | "permanents" => LeafDemonstrativeObjectHead::Permanent,
        "card" | "cards" => LeafDemonstrativeObjectHead::Card,
        "spell" | "spells" => LeafDemonstrativeObjectHead::Spell,
        "source" | "sources" => LeafDemonstrativeObjectHead::Source,
        "token" | "tokens" => LeafDemonstrativeObjectHead::Token,
        _ => {
            let singular = stem_before_tail(raw, b"s")?;
            return classify_card_type(singular).map(LeafDemonstrativeObjectHead::CardType);
        }
    };
    Some(generic)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn card_types_and_zones_preserve_singular_and_plural_surfaces() {
        assert_eq!(
            parse_leaf_card_type_complete("sorceries").unwrap(),
            CardType::Sorcery
        );
        assert_eq!(
            parse_leaf_zone_complete("libraries").unwrap(),
            Zone::Library
        );
    }

    #[test]
    fn subtype_parser_preserves_irregular_and_flexible_plurals() {
        assert_eq!(parse_leaf_subtype_complete("mice").unwrap(), Subtype::Mouse);
        assert_eq!(
            parse_leaf_subtype_flexible_complete("foxes").unwrap(),
            Subtype::Fox
        );
        assert_eq!(
            parse_leaf_subtype_flexible_complete("wolves").unwrap(),
            Subtype::Wolf
        );
    }

    #[test]
    fn negated_descriptors_return_the_positive_typed_atom() {
        assert_eq!(
            parse_leaf_non_card_type_complete("noncreature").unwrap(),
            CardType::Creature
        );
        assert_eq!(
            parse_leaf_non_subtype_complete("nonwolves").unwrap(),
            Subtype::Wolf
        );
        assert!(parse_leaf_non_card_type_complete("creature").is_err());
    }

    #[test]
    fn demonstrative_heads_are_typed() {
        assert_eq!(
            parse_leaf_demonstrative_object_head_complete("sources").unwrap(),
            LeafDemonstrativeObjectHead::Source
        );
        assert_eq!(
            parse_leaf_demonstrative_object_head_complete("creatures").unwrap(),
            LeafDemonstrativeObjectHead::CardType(CardType::Creature)
        );
        assert_eq!(
            parse_leaf_object_reference_head_complete("goblins").unwrap(),
            LeafObjectReferenceHead::Subtype(Subtype::Goblin)
        );
    }
}
