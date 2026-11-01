use ironsmith_core::custom_subtypes::{CustomSubtypeDefinition, normalize_name};
pub fn with_custom_subtypes<T>(definitions: &[CustomSubtypeDefinition], f: impl FnOnce() -> T) -> Result<T, String> {
    ironsmith_core::custom_subtypes::with_vocabulary(&[], || {
        for def in definitions {
            for name in [&def.singular, &def.plural] {
                let name = normalize_name(name)?;
                if crate::leaf::parse_leaf_subtype_flexible_complete(&name).is_ok()
                    || crate::leaf::classify_token_definition_subtype(&name).is_some()
                    || crate::util::parse_card_type(&name).is_some()
                    || ["card", "cards", "permanent", "permanents", "spell", "spells", "token", "tokens", "target", "each", "all", "other", "you", "your", "this", "that", "the", "a", "an"].contains(&name.as_str()) {
                    return Err(format!("Subtype form is reserved: {name}"));
                }
            }
        }
        ironsmith_core::custom_subtypes::with_vocabulary(definitions, f)
    })?
}
