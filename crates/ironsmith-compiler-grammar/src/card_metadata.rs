//! Metadata-line folding for the card's face facts.
//!
//! Metadata lines (mana cost, type line, power/toughness, loyalty, ...) are part
//! of the document, and recognition reads the resulting face facts to decide how
//! later lines parse. Keeping the fold here, over the kernel `CardBuilder`, is
//! what lets recognition run without ever seeing the definition builder.

use ironsmith_core::card::CardBuilder;

use crate::cards::builders::CardTextError;

/// Fold one metadata line into the card's face facts.
///
/// Metadata lines are document structure, so this reads and writes only the
/// kernel `CardBuilder`; nothing here touches the definition being lowered.
pub fn apply_metadata_line(
    mut card: CardBuilder,
    meta: impl Into<crate::front_end::MetadataLine>,
) -> Result<CardBuilder, CardTextError> {
    let meta = meta.into();
    match meta {
        crate::front_end::MetadataLine::ManaCost(raw) => {
            let cost = crate::util::parse_scryfall_mana_cost(&raw)?;
            if !cost.is_empty() {
                card = card.mana_cost(cost);
            }
        }
        crate::front_end::MetadataLine::TypeLine(raw) => {
            let (supertypes, card_types, subtypes) =
                crate::effect_sentences::parse_type_line(&raw)?;
            if !supertypes.is_empty() {
                card = card.supertypes(supertypes);
            }
            if !card_types.is_empty() {
                card = card.card_types(card_types);
            }
            if !subtypes.is_empty() {
                card = card.subtypes(subtypes);
            }
        }
        crate::front_end::MetadataLine::FirstPrintedSet(raw) => {
            let set_name = raw.trim();
            if !set_name.is_empty() {
                card = card.first_printed_set_name(set_name.to_string());
            }
        }
        crate::front_end::MetadataLine::AttractionLights(raw) => {
            let mut lights = raw
                .split(|character: char| character == ',' || character.is_whitespace())
                .filter(|part| !part.is_empty())
                .map(|part| {
                    part.parse::<u8>().map_err(|_| {
                        CardTextError::ParseError(format!(
                            "invalid Attraction light number: {part}"
                        ))
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            lights.sort_unstable();
            lights.dedup();
            if lights.iter().any(|light| !(1..=6).contains(light)) {
                return Err(CardTextError::ParseError(
                    "Attraction light numbers must be between 1 and 6".to_string(),
                ));
            }
            card = card.attraction_lights(lights);
        }
        crate::front_end::MetadataLine::PowerToughness(raw) => {
            if let Some(pt) = crate::util::parse_power_toughness(&raw) {
                card = card.power_toughness(pt);
            }
        }
        crate::front_end::MetadataLine::Loyalty(raw) => {
            if let Ok(value) = raw.trim().parse::<u32>() {
                card = card.loyalty(value);
            }
        }
        crate::front_end::MetadataLine::Defense(raw) => {
            if let Ok(value) = raw.trim().parse::<u32>() {
                card = card.defense(value);
            }
        }
    }
    Ok(card)
}

/// Fold a recognized metadata line into the card's face facts.
///
/// The recognizer's metadata vocabulary and the document's are the same set of
/// lines under two names; this maps one onto the other and folds it in.
pub fn apply_compiler_metadata_line(
    card: CardBuilder,
    meta: crate::model::facts::MetadataLine,
) -> Result<CardBuilder, CardTextError> {
    let structural = match meta {
        crate::model::facts::MetadataLine::ManaCost(value) => {
            crate::front_end::MetadataLine::ManaCost(value)
        }
        crate::model::facts::MetadataLine::TypeLine(value) => {
            crate::front_end::MetadataLine::TypeLine(value)
        }
        crate::model::facts::MetadataLine::FirstPrintedSet(value) => {
            crate::front_end::MetadataLine::FirstPrintedSet(value)
        }
        crate::model::facts::MetadataLine::AttractionLights(value) => {
            crate::front_end::MetadataLine::AttractionLights(value)
        }
        crate::model::facts::MetadataLine::PowerToughness(value) => {
            crate::front_end::MetadataLine::PowerToughness(value)
        }
        crate::model::facts::MetadataLine::Loyalty(value) => {
            crate::front_end::MetadataLine::Loyalty(value)
        }
        crate::model::facts::MetadataLine::Defense(value) => {
            crate::front_end::MetadataLine::Defense(value)
        }
    };
    apply_metadata_line(card, structural)
}
