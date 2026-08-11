use super::*;

pub(crate) fn parse_for_each_mana_symbol_spent_effect_shape(
    tokens: &[OwnedLexToken],
) -> Option<ForEachManaSymbolSpentEffectShape<'_>> {
    let (subject_tokens, effect_tokens) =
        primitives::split_lexed_once_on_separator(tokens, || primitives::comma().void())?;
    let (_, after_prefix) = primitives::parse_prefix(
        trim_lexed_commas(subject_tokens),
        primitives::phrase(&["for", "each"]),
    )?;
    let mana_token_count = after_prefix
        .iter()
        .take_while(|token| token.kind == crate::runtime_backend::lexer::TokenKind::ManaGroup)
        .count();
    if mana_token_count == 0 {
        return None;
    }
    let mut symbols = Vec::new();
    for token in &after_prefix[..mana_token_count] {
        symbols.extend(
            crate::runtime_backend::grammar::values::parse_mana_symbol_group(token.parser_text())
                .ok()?,
        );
    }
    let suffix = &after_prefix[mana_token_count..];
    let (&symbol, remaining) = symbols.split_first()?;
    if !remaining.iter().all(|candidate| *candidate == symbol)
        || !matches!(
            symbol,
            crate::mana::ManaSymbol::White
                | crate::mana::ManaSymbol::Blue
                | crate::mana::ManaSymbol::Black
                | crate::mana::ManaSymbol::Red
                | crate::mana::ManaSymbol::Green
                | crate::mana::ManaSymbol::Colorless
        )
    {
        return None;
    }
    let suffix_words = primitives::TokenWordView::new(suffix).to_word_refs();
    let reference = match suffix_words.as_slice() {
        ["spent", "to", "cast", "it"] => ironsmith_core::ManaSpentCastReferenceSurface::It,
        ["spent", "to", "cast", "this", "spell"] => {
            ironsmith_core::ManaSpentCastReferenceSurface::ThisSpell
        }
        ["spent", "to", "cast", "this", "creature"] => {
            ironsmith_core::ManaSpentCastReferenceSurface::ThisCreature
        }
        _ => return None,
    };
    let effect_tokens = trim_lexed_commas(effect_tokens);
    (!effect_tokens.is_empty()).then_some(ForEachManaSymbolSpentEffectShape {
        symbol,
        group_size: symbols.len().try_into().ok()?,
        reference,
        effect_tokens,
    })
}

pub(crate) fn parse_for_each_spent_mana_effect_shape(
    tokens: &[OwnedLexToken],
) -> Option<ForEachSpentManaEffectShape<'_>> {
    let (subject_tokens, effect_tokens) =
        primitives::split_lexed_once_on_separator(tokens, || primitives::comma().void())?;
    let (_, after_prefix) = primitives::parse_prefix(
        trim_lexed_commas(subject_tokens),
        primitives::phrase(&["for", "each", "mana", "from"]),
    )?;
    let spent_index = after_prefix
        .iter()
        .position(|token| token.is_word("spent"))?;
    let source_tokens = &after_prefix[..spent_index];
    let reference_words =
        primitives::TokenWordView::new(&after_prefix[spent_index..]).to_word_refs();
    let reference = match reference_words.as_slice() {
        ["spent", "to", "cast", "it"] => ironsmith_core::ManaSpentCastReferenceSurface::It,
        ["spent", "to", "cast", "this", "spell"] => {
            ironsmith_core::ManaSpentCastReferenceSurface::ThisSpell
        }
        ["spent", "to", "cast", "this", "creature"] => {
            ironsmith_core::ManaSpentCastReferenceSurface::ThisCreature
        }
        _ => return None,
    };
    let source_tokens = trim_lexed_commas(source_tokens);
    let effect_tokens = trim_lexed_commas(effect_tokens);
    (!source_tokens.is_empty() && !effect_tokens.is_empty()).then_some(
        ForEachSpentManaEffectShape {
            source_tokens,
            reference,
            effect_tokens,
        },
    )
}

pub(crate) fn parse_for_each_dynamic_target_effect_shape(
    tokens: &[OwnedLexToken],
) -> Option<ForEachDynamicTargetEffectShape<'_>> {
    let (subject_tokens, effect_tokens) =
        primitives::split_lexed_once_on_separator(tokens, || primitives::comma().void())?;
    let (_, filter_tokens) = primitives::parse_prefix(
        trim_lexed_commas(subject_tokens),
        (
            primitives::phrase(&["for", "each", "of"]),
            primitives::kw("x"),
            primitives::kw("target"),
        )
            .void(),
    )?;
    let filter_tokens = trim_lexed_commas(filter_tokens);
    let effect_tokens = trim_lexed_commas(effect_tokens);
    (!filter_tokens.is_empty() && !effect_tokens.is_empty()).then_some(
        ForEachDynamicTargetEffectShape {
            filter_tokens,
            effect_tokens,
        },
    )
}
