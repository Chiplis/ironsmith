use crate::cards::builders::CardTextError;
use crate::color::ColorSet;
use crate::static_abilities::{Anthem, AnthemCountExpression, AnthemValue, StaticAbility};
use crate::target::{ObjectFilter, PlayerFilter};
use crate::types::{CardType, Subtype, Supertype};
use crate::zone::Zone;

use super::{
    CreationPhrase, CreationWordClass, CreationWords, parse_pt_word, parse_unsigned_pt_word,
};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CopyModifierSpec {
    pub(crate) set_colors: Option<ColorSet>,
    pub(crate) set_card_types: Option<Vec<CardType>>,
    pub(crate) set_subtypes: Option<Vec<Subtype>>,
    pub(crate) added_card_types: Vec<CardType>,
    pub(crate) added_subtypes: Vec<Subtype>,
    pub(crate) removed_supertypes: Vec<Supertype>,
    pub(crate) set_base_power_toughness: Option<(i32, i32)>,
    pub(crate) granted_abilities: Vec<StaticAbility>,
    /// "except it has haste and loses soulbond": the copy is created without
    /// the soulbond pairing ability.
    pub(crate) loses_soulbond: bool,
}

impl Default for CopyModifierSpec {
    fn default() -> Self {
        Self {
            set_colors: None,
            set_card_types: None,
            set_subtypes: None,
            added_card_types: Vec::new(),
            added_subtypes: Vec::new(),
            removed_supertypes: Vec::new(),
            set_base_power_toughness: None,
            granted_abilities: Vec::new(),
            loses_soulbond: false,
        }
    }
}

fn last_class_location(words: &[&str], class: CreationWordClass) -> Option<usize> {
    let mut cursor = 0usize;
    let mut result = None;
    while cursor < words.len() {
        let Some(relative) = CreationWords::new(&words[cursor..]).location(class) else {
            break;
        };
        let location = cursor + relative;
        result = Some(location);
        cursor = location + 1;
    }
    result
}

fn push_unique<T: PartialEq>(values: &mut Vec<T>, value: T) {
    if !values.iter().any(|existing| existing == &value) {
        values.push(value);
    }
}

pub(crate) fn parse_copy_modifier_words(
    tail_words: &[&str],
) -> Result<CopyModifierSpec, CardTextError> {
    let modifier_words = last_class_location(tail_words, CreationWordClass::Except)
        .and_then(|idx| tail_words.get(idx + 1..))
        .unwrap_or_default();
    let surface = CreationWords::new(modifier_words);
    let mut spec = CopyModifierSpec::default();
    if modifier_words.is_empty() {
        return Ok(spec);
    }

    if surface.has(CreationWordClass::LoseVerb) && surface.has(CreationWordClass::Soulbond) {
        // "loses soulbond" (Mirage Phalanx): the copy is created without the
        // soulbond pairing ability. Only the adjacent "lose(s) soulbond" pair
        // has that meaning; anything else stays unsupported.
        if modifier_words
            .windows(2)
            .any(|pair| matches!(pair[0], "lose" | "loses") && pair[1] == "soulbond")
        {
            spec.loses_soulbond = true;
        } else {
            return Err(CardTextError::ParseError(
                "removing soulbond requires non-marker semantics".to_string(),
            ));
        }
    }
    if surface.has_phrase(CreationPhrase::NotLegendary) {
        spec.removed_supertypes.push(Supertype::Legendary);
    }
    spec.set_base_power_toughness = modifier_words
        .iter()
        .filter_map(|word| parse_unsigned_pt_word(word))
        .next();

    let grants_keyword = |phrase, keyword: &str| {
        surface.has_phrase(phrase)
            || (surface.has(CreationWordClass::GrantVerb)
                && CreationWords::new(modifier_words).has_literal(keyword))
    };
    if grants_keyword(CreationPhrase::WithFlying, "flying") {
        spec.granted_abilities.push(StaticAbility::flying());
    }
    if grants_keyword(CreationPhrase::WithTrample, "trample") {
        spec.granted_abilities.push(StaticAbility::trample());
    }
    if let Some(amount) = modifier_words
        .windows(2)
        .find(|pair| pair[0] == "toxic")
        .and_then(|pair| pair[1].parse::<u32>().ok())
    {
        push_unique(
            &mut spec.granted_abilities,
            StaticAbility::keyword_marker(format!("toxic {amount}")),
        );
    }

    if let Some(idx) = surface.phrase_location(CreationPhrase::GetsForEach) {
        let mut tail = modifier_words.get(idx + 6..).unwrap_or_default();
        while CreationWords::new(tail).first_is(CreationWordClass::ArticleOrThe) {
            tail = &tail[1..];
        }
        if let Some(subtype) = tail.first().and_then(|word| {
            crate::runtime_backend::front_end::shared::util::parse_subtype_flexible(word)
        }) && CreationWords::new(tail).has_phrase(CreationPhrase::YouControl)
        {
            let mut filter = ObjectFilter::default();
            filter.zone = Some(Zone::Battlefield);
            filter.controller = Some(PlayerFilter::You);
            filter.subtypes = vec![subtype];
            let count = AnthemCountExpression::MatchingFilter(filter);
            let anthem = Anthem::for_source(0, 0).with_values(
                AnthemValue::scaled(1, count.clone()),
                AnthemValue::scaled(1, count),
            );
            spec.granted_abilities.push(StaticAbility::new(anthem));
        }
    }

    if let Some(addition) = surface.phrase_location(CreationPhrase::AdditionToOtherTypes) {
        let mut colors = ColorSet::new();
        for word in &modifier_words[..addition] {
            if let Some(color) = crate::runtime_backend::front_end::shared::util::parse_color(word)
            {
                colors = colors.union(color);
            }
            if let Some(card_type) =
                crate::runtime_backend::front_end::shared::util::parse_card_type(word)
            {
                push_unique(&mut spec.added_card_types, card_type);
            }
            if let Some(subtype) =
                crate::runtime_backend::front_end::shared::util::parse_subtype_flexible(word)
            {
                push_unique(&mut spec.added_subtypes, subtype);
            }
        }
        spec.set_colors = (!colors.is_empty()).then_some(colors);
    } else if surface.starts(CreationPhrase::IdentityClause) {
        let descriptor_end = surface
            .location(CreationWordClass::DescriptorEnd)
            .unwrap_or(modifier_words.len());
        let mut colors = ColorSet::new();
        let mut card_types = Vec::new();
        let mut subtypes = Vec::new();
        for word in &modifier_words[..descriptor_end] {
            if CreationWords::new(&[*word]).first_is(CreationWordClass::Article)
                || matches!(
                    *word,
                    "its"
                        | "it"
                        | "is"
                        | "s"
                        | "it's"
                        | "it’s"
                        | "they"
                        | "are"
                        | "re"
                        | "theyre"
                        | "they're"
                        | "they’re"
                )
                || parse_pt_word(word).is_some()
            {
                continue;
            }
            if let Some(color) = crate::runtime_backend::front_end::shared::util::parse_color(word)
            {
                colors = colors.union(color);
            }
            if let Some(card_type) =
                crate::runtime_backend::front_end::shared::util::parse_card_type(word)
            {
                push_unique(&mut card_types, card_type);
            }
            if let Some(subtype) =
                crate::runtime_backend::front_end::shared::util::parse_subtype_flexible(word)
            {
                push_unique(&mut subtypes, subtype);
            }
        }
        spec.set_colors = (!colors.is_empty()).then_some(colors);
        spec.set_card_types = (!card_types.is_empty()).then_some(card_types);
        spec.set_subtypes = (!subtypes.is_empty()).then_some(subtypes);
    }
    Ok(spec)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_copy_modifiers_into_typed_spec() {
        let spec = parse_copy_modifier_words(&[
            "except",
            "it",
            "is",
            "a",
            "red",
            "dragon",
            "with",
            "flying",
            "and",
            "isnt",
            "legendary",
        ])
        .unwrap();
        assert!(spec.set_colors.is_some());
        assert_eq!(spec.set_subtypes, Some(vec![Subtype::Dragon]));
        assert_eq!(spec.removed_supertypes, vec![Supertype::Legendary]);
        assert_eq!(spec.granted_abilities, vec![StaticAbility::flying()]);
    }

    #[test]
    fn parses_counted_keyword_copy_modifier() {
        let spec = parse_copy_modifier_words(&["except", "its", "1/1", "and", "has", "toxic", "1"])
            .unwrap();
        assert_eq!(
            spec.granted_abilities,
            vec![StaticAbility::keyword_marker("toxic 1")]
        );
    }
}
