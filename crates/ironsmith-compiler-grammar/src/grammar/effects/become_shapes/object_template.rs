//! Complete, quotation-aware descriptors with no authored base P/T.
use crate::lexer::{OwnedLexToken, TokenKind, parser_token_word_positions, parser_token_word_refs};
use crate::types::{CardType, Subtype, Supertype};
use crate::color::ColorSet;
use super::super::super::leaf;

#[derive(Debug, Clone)]
pub struct UnsizedObjectTemplateShape<'a> {
    pub card_types: Vec<CardType>,
    pub subtypes: Vec<Subtype>,
    pub supertypes: Vec<Supertype>,
    pub colors: Option<ColorSet>,
    pub ability_tokens: &'a [OwnedLexToken],
    pub preserve_other_types: bool,
    pub preserve_other_colors: bool,
    pub remove_other_abilities: bool,
}

fn outside_quotes(tokens: &[OwnedLexToken], index: usize) -> bool {
    tokens[..index].iter().filter(|token| token.kind == TokenKind::Quote).count() % 2 == 0
}
fn trim_separators(mut tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    while tokens.first().is_some_and(|t| matches!(t.kind, TokenKind::Comma | TokenKind::Period | TokenKind::Semicolon)) { tokens = &tokens[1..]; }
    while tokens.last().is_some_and(|t| matches!(t.kind, TokenKind::Comma | TokenKind::Period | TokenKind::Semicolon)) { tokens = &tokens[..tokens.len()-1]; }
    tokens
}

pub fn parse_unsized_object_template_tokens(tokens: &[OwnedLexToken]) -> Option<UnsizedObjectTemplateShape<'_>> {
    let mut tokens = trim_separators(tokens);
    let words = parser_token_word_refs(tokens);
    let positions = parser_token_word_positions(tokens);
    let (prefix, mut preserve_other_types) = super::strip_become_addition_tail_words(&words);
    let mut preserve_other_colors = false;
    let mut remove_other_abilities = false;
    if prefix.len() < words.len() {
        let start = positions[prefix.len()].0;
        if outside_quotes(tokens, start) {
            preserve_other_colors = words[prefix.len()..].contains(&"colors");
            tokens = trim_separators(&tokens[..start]);
        } else { preserve_other_types = false; }
    } else {
        for suffix in [
            &["and", "loses", "all", "other", "card", "types", "and", "abilities"][..],
            &["and", "lose", "all", "other", "card", "types", "and", "abilities"][..],
        ] {
            if words.ends_with(suffix) {
                let start = positions[words.len()-suffix.len()].0;
                if !outside_quotes(tokens, start) { return None; }
                tokens = trim_separators(&tokens[..start]);
                remove_other_abilities = true;
                break;
            }
        }
    }
    let with = tokens.iter().enumerate().find_map(|(i,t)| (t.is_word("with") && outside_quotes(tokens,i)).then_some(i))?;
    let descriptor_words = parser_token_word_refs(&tokens[..with]);
    let ability_tokens = trim_separators(&tokens[with+1..]);
    // Base-size and copy productions own those complete forms. This route
    // must not reinterpret their suffix as an ability or silently ignore it.
    if ability_tokens.is_empty() || ability_tokens.first().is_some_and(|t| t.is_word("base")) { return None; }
    let mut card_types = Vec::new();
    let mut subtypes = Vec::new();
    let mut supertypes = Vec::new();
    let mut colors = ColorSet::new();
    let mut color_stated = false;
    let mut index = 0;
    while index < descriptor_words.len() {
        let word = descriptor_words[index];
        if matches!(word, "a" | "an" | "and") { index += 1; continue; }
        if word == "colorless" { color_stated = true; index += 1; continue; }
        if let Ok(color) = leaf::parse_leaf_color_complete(word) { colors = colors.union(color); color_stated = true; }
        else if let Some(kind) = crate::util::parse_supertype_word(word) { if !supertypes.contains(&kind) { supertypes.push(kind); } }
        else if let Ok(kind) = leaf::parse_leaf_card_type_complete(word) { if !card_types.contains(&kind) { card_types.push(kind); } }
        else if let Ok(kind) = leaf::parse_leaf_subtype_flexible_complete(word) { if !subtypes.contains(&kind) { subtypes.push(kind); } }
        else if let Some(next) = descriptor_words.get(index+1) {
            let kind = leaf::parse_leaf_subtype_flexible_complete(&format!("{word}-{next}")).ok()?;
            if !subtypes.contains(&kind) { subtypes.push(kind); } index += 1;
        } else { return None; }
        index += 1;
    }
    // Bare creature subtypes retain existing card types. A Treasure subtype
    // never implies Creature, and an unspecified unrelated card type is not
    // invented. Explicit artifact/land/etc descriptors retain their own type.
    if card_types.is_empty() && (subtypes.is_empty() || !subtypes.iter().all(|s| s.belongs_to_family(crate::types::SubtypeFamily::Creature))) { return None; }
    Some(UnsizedObjectTemplateShape { card_types, subtypes, supertypes, colors: color_stated.then_some(colors), ability_tokens, preserve_other_types, preserve_other_colors, remove_other_abilities })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn shape(text: &str) -> UnsizedObjectTemplateShape<'static> {
        let tokens = Box::leak(crate::lexer::lex_line(text, 0).unwrap().into_boxed_slice());
        parse_unsized_object_template_tokens(tokens).expect("complete object template")
    }
    #[test]
    fn no_size_templates_preserve_absence_and_actual_type_families() {
        let creature = shape("a Human Spirit Warrior with trample and lifelink");
        assert!(creature.card_types.is_empty(), "subtype-only conversion must retain card types");
        assert_eq!(creature.subtypes, vec![Subtype::Human, Subtype::Spirit, Subtype::Warrior]);
        let artifact = shape("a Treasure artifact with \"{T}, Sacrifice this artifact: Add one mana of any color\" and loses all other card types and abilities");
        assert_eq!(artifact.card_types, vec![CardType::Artifact]);
        assert_eq!(artifact.subtypes, vec![Subtype::Treasure]);
        assert!(artifact.remove_other_abilities);
        assert!(!parser_token_word_refs(artifact.ability_tokens).contains(&"loses"));
    }
    #[test]
    fn quoted_inner_retention_is_not_an_outer_descriptor_tail() {
        let inner = shape("a creature with \"{T}: Target creature becomes a Zombie in addition to its other types.\"");
        assert!(!inner.preserve_other_types);
        let outer = shape("a blue creature with flying in addition to its other colors and types");
        assert!(outer.preserve_other_types && outer.preserve_other_colors);
    }
    #[test]
    fn malformed_or_sized_descriptors_are_not_partial_templates() {
        for text in ["a 3/3 creature with flying", "a creature with base power and toughness 4/4", "a mysterious artifact with flying", "a Treasure with flying", "a creature with"] {
            assert!(parse_unsized_object_template_tokens(&crate::lexer::lex_line(text, 0).unwrap()).is_none(), "{text}");
        }
    }
}
