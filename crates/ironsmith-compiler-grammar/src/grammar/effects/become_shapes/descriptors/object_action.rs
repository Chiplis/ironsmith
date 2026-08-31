use super::*;

pub fn parse_become_animation_suffix_shape(
    tokens: &[OwnedLexToken],
) -> BecomeAnimationSuffixShape<'_> {
    let tokens = trim_lexed_commas(tokens);
    if tokens.is_empty() {
        return BecomeAnimationSuffixShape::Ignored {
            preserve_other_types: false,
            type_retention_surface: None,
        };
    }
    let stripped_addition = strip_addition_tail_tokens(tokens);
    let retained_card_type = split_still_a_card_type_tail_tokens(stripped_addition);
    if let Some((head, card_type)) = retained_card_type
        && head.is_empty()
    {
        let type_retention_surface = if card_type == CardType::Land {
            ironsmith_core::TypeRetentionSurface::StillALand
        } else {
            ironsmith_core::TypeRetentionSurface::StillACardType(card_type)
        };
        return BecomeAnimationSuffixShape::Ignored {
            preserve_other_types: true,
            type_retention_surface: Some(type_retention_surface),
        };
    }
    let stripped_still_land = strip_still_a_land_tail_tokens(stripped_addition);
    let stripped_retention = retained_card_type
        .map(|(head, _)| head)
        .unwrap_or(stripped_still_land);
    let type_retention_surface = if stripped_addition.len() != tokens.len() {
        Some(ironsmith_core::TypeRetentionSurface::InAdditionToOtherTypes)
    } else if let Some((_, CardType::Land)) = retained_card_type {
        Some(ironsmith_core::TypeRetentionSurface::StillALand)
    } else if let Some((_, card_type)) = retained_card_type {
        Some(ironsmith_core::TypeRetentionSurface::StillACardType(
            card_type,
        ))
    } else if stripped_still_land.len() != stripped_addition.len() || still_a_land(tokens) {
        Some(ironsmith_core::TypeRetentionSurface::StillALand)
    } else {
        None
    };
    let preserve_other_types = type_retention_surface.is_some();
    if stripped_retention.is_empty() || still_a_land(tokens) {
        return BecomeAnimationSuffixShape::Ignored {
            preserve_other_types,
            type_retention_surface,
        };
    }
    let Some((_, after_with)) =
        primitives::parse_prefix(stripped_retention, primitives::kw("with").void())
    else {
        return BecomeAnimationSuffixShape::Unsupported;
    };
    let ability_tokens = trim_lexed_commas(after_with);
    if let Some(without_family) = split_all_creature_types(ability_tokens) {
        return BecomeAnimationSuffixShape::With {
            ability_tokens: without_family,
            grants_all_creature_types: true,
            preserve_other_types,
            type_retention_surface,
        };
    }
    BecomeAnimationSuffixShape::With {
        ability_tokens,
        grants_all_creature_types: false,
        preserve_other_types,
        type_retention_surface,
    }
}

pub fn parse_become_simple_descriptor_words(words: &[&str]) -> BecomeSimpleDescriptorShape {
    let (words, had_addition_tail) = strip_become_addition_tail_words(words);
    if words.is_empty() {
        return BecomeSimpleDescriptorShape::None;
    }

    let mut colors = ColorSet::new();
    let mut subtypes = Vec::new();
    let mut color_or_subtype = true;
    for word in words {
        if *word == "and" {
            continue;
        }
        if let Ok(color) = leaf::parse_leaf_color_complete(word) {
            colors = colors.union(color);
        } else if let Ok(subtype) = leaf::parse_leaf_subtype_flexible_complete(word) {
            push_unique(&mut subtypes, subtype);
        } else {
            color_or_subtype = false;
            break;
        }
    }
    if color_or_subtype && !colors.is_empty() && !subtypes.is_empty() {
        return BecomeSimpleDescriptorShape::ColorsAndSubtypes { colors, subtypes };
    }

    let mut card_types = Vec::new();
    if words.iter().all(|word| {
        leaf::parse_leaf_card_type_complete(word)
            .map(|card_type| push_unique(&mut card_types, card_type))
            .is_ok()
    }) && !card_types.is_empty()
    {
        return BecomeSimpleDescriptorShape::CardTypes {
            card_types,
            preserve_other_types: had_addition_tail,
        };
    }

    let mut subtypes = Vec::new();
    if words.iter().all(|word| {
        leaf::parse_leaf_subtype_flexible_complete(word)
            .map(|subtype| push_unique(&mut subtypes, subtype))
            .is_ok()
    }) && !subtypes.is_empty()
    {
        let replace_creature_subtypes = !had_addition_tail && creature_subtypes_only(&subtypes);
        return BecomeSimpleDescriptorShape::Subtypes {
            subtypes,
            replace_creature_subtypes,
        };
    }
    BecomeSimpleDescriptorShape::None
}

pub fn parse_become_color_words(words: &[&str]) -> Option<ColorSet> {
    let mut colors = ColorSet::new();
    let mut saw_color = false;
    for word in words {
        if matches!(*word, "and" | "or") {
            continue;
        }
        let color = crate::grammar::primitives::probe_shape(leaf::parse_leaf_color_complete(word))?;
        colors = colors.union(color);
        saw_color = true;
    }
    saw_color.then_some(colors)
}
