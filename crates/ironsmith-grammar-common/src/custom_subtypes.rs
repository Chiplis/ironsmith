use ironsmith_core::custom_subtypes::{CustomSubtypeDefinition, normalize_name};
pub fn with_custom_subtypes<T>(
    definitions: &[CustomSubtypeDefinition],
    f: impl FnOnce() -> T,
) -> Result<T, String> {
    ironsmith_core::custom_subtypes::with_vocabulary(&[], || {
        for (index, def) in definitions.iter().enumerate() {
            for prior in &definitions[..index] {
                if normalize_name(&prior.singular)? == normalize_name(&def.singular)?
                    && (prior.family != def.family
                        || normalize_name(&prior.plural)? != normalize_name(&def.plural)?)
                {
                    return Err(format!("Conflicting subtype definition: {}", def.singular));
                }
            }
            for name in [&def.singular, &def.plural] {
                let name = normalize_name(name)?;
                if crate::leaf::parse_leaf_subtype_flexible_complete(&name).is_ok()
                    || crate::leaf::classify_token_definition_subtype(&name).is_some()
                    || crate::util::parse_card_type(&name).is_some()
                    || crate::util::parse_color(&name).is_some()
                    || crate::leaf::parse_leaf_supertype_complete(&name).is_ok()
                    || crate::util::parse_zone_word(&name).is_some()
                    || crate::util::parse_number_word_i32(&name).is_some()
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
