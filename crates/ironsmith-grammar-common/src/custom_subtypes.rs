use ironsmith_core::custom_subtypes::{CustomSubtypeDefinition, normalize_name};
pub fn with_custom_subtypes<T>(
    definitions: &[CustomSubtypeDefinition],
    f: impl FnOnce() -> T,
) -> Result<T, String> {
    ironsmith_core::custom_subtypes::with_vocabulary(&[], || {
        for (index, def) in definitions.iter().enumerate() {
            for prior in &definitions[..index] {
                if normalize_name(&prior.singular)? == normalize_name(&def.singular)?
                    && normalize_name(&prior.plural)? != normalize_name(&def.plural)?
                {
                    return Err(format!("Conflicting subtype definition: {}", def.singular));
                }
            }
            for name in [&def.singular, &def.plural] {
                let name = normalize_name(name)?;
                // Historical custom enum variants are compatibility vocabulary,
                // not official-word reservations. An explicit source scope must
                // be able to supply their complete family associations.
                if crate::leaf::parse_leaf_subtype_flexible_complete(&name)
                    .is_ok_and(|subtype| !subtype.is_custom_vocabulary())
                    || crate::leaf::classify_token_definition_subtype(&name)
                        .is_some_and(|subtype| !subtype.is_custom_vocabulary())
                    || crate::util::parse_card_type(&name).is_some()
                    || crate::util::parse_color(&name).is_some()
                    || crate::leaf::parse_leaf_supertype_complete(&name).is_ok()
                    || crate::util::parse_zone_word(&name).is_some()
                    // Planeswalker types are proper names (MSEM's "Zero",
                    // CR 205.3j); they only appear as planeswalker subtypes,
                    // so a name that spells a number is not a quantity word.
                    || (def.family != ironsmith_core::SubtypeFamily::Planeswalker
                        && crate::util::parse_number_word_i32(&name).is_some())
                    || crate::leaf::parse_leaf_non_subtype_complete(&name).is_ok()
                    || crate::static_abilities::StaticAbility::keyword_marker(name.clone()).id()
                        != ironsmith_core::StaticAbilityId::KeywordMarker
                    || [
                        "tapped",
                        "untapped",
                        "attacking",
                        "blocking",
                        "nontoken",
                        "noncreature",
                        "colorless",
                        "multicolored",
                        "monocolored",
                        "another",
                        "chosen",
                        "enchanted",
                        "equipped",
                        "it",
                        "they",
                        "those",
                        "these",
                        "card",
                        "cards",
                        "permanent",
                        "permanents",
                        "spell",
                        "spells",
                        "token",
                        "tokens",
                        "target",
                        "each",
                        "all",
                        "other",
                        "you",
                        "your",
                        "this",
                        "that",
                        "the",
                        "a",
                        "an",
                    ]
                    .contains(&name.as_str())
                {
                    return Err(format!("Subtype form is reserved: {name}"));
                }
            }
        }
        ironsmith_core::custom_subtypes::with_vocabulary(definitions, f)
    })?
}

#[cfg(test)]
mod tests {
    use super::*;
    use ironsmith_core::{Subtype, SubtypeFamily};

    #[test]
    fn explicit_legacy_custom_spelling_preserves_one_scoped_identity() {
        let definitions = [SubtypeFamily::Spell, SubtypeFamily::Artifact].map(|family|
            CustomSubtypeDefinition { singular: "Inscription".into(), plural: "Inscriptions".into(), family });
        with_custom_subtypes(&definitions, || {
            for spelling in ["Inscription", "Inscriptions"] {
                let scoped = ironsmith_core::custom_subtypes::lookup(spelling).unwrap();
                assert!(matches!(scoped, Subtype::Custom(_)));
                assert!(scoped.belongs_to_family(SubtypeFamily::Spell));
                assert!(scoped.belongs_to_family(SubtypeFamily::Artifact));
                assert!(crate::leaf::ambiguous_subtype_spellings(spelling).is_empty());
                assert_eq!(crate::leaf::classify_token_definition_subtype(spelling), Some(scoped));
            }
        }).unwrap();
        assert!(ironsmith_core::custom_subtypes::lookup("Inscription").is_none());
        assert!(!crate::leaf::ambiguous_subtype_spellings("Inscription").is_empty());
    }

    #[test]
    fn explicit_custom_scope_keeps_official_and_grammatical_words_reserved() {
        for spelling in ["Elf", "Elves", "Omen", "Creature", "Red", "Target", "Legendary"] {
            let definition = CustomSubtypeDefinition {
                singular: "Witnesskind".into(), plural: spelling.into(), family: SubtypeFamily::Artifact,
            };
            assert!(with_custom_subtypes(&[definition], || ()).is_err(), "{spelling}");
        }
    }
}
