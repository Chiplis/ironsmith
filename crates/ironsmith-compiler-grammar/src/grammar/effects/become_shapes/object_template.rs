//! Complete quotation-aware object descriptors, preserving unstated characteristics.
use crate::lexer::{OwnedLexToken, TokenKind, parser_token_word_positions, parser_token_word_refs};
use crate::types::{CardType, Subtype, Supertype};
use crate::color::ColorSet;
use super::super::super::leaf;

#[derive(Debug, Clone)]
pub struct ObjectTemplateShape<'a> {
    pub base_power_toughness: Option<(crate::effect::Value, crate::effect::Value)>,
    pub name_override: Option<String>,
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

pub fn parse_object_template_tokens(tokens: &[OwnedLexToken]) -> Option<ObjectTemplateShape<'_>> {
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
            &["and", "loses", "all", "other", "abilities"][..],
            &["and", "lose", "all", "other", "abilities"][..],
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
    let with = tokens.iter().enumerate().find_map(|(i,t)| (t.is_word("with") && outside_quotes(tokens,i)).then_some(i));
    let mut descriptor_tokens = with.map(|i| trim_separators(&tokens[..i])).unwrap_or(tokens);
    let mut ability_tokens = with.map(|i| trim_separators(&tokens[i+1..])).unwrap_or(&[]);
    if with.is_some() && ability_tokens.is_empty() { return None; }
    // A literal name is a characteristic, not discarded presentation text.
    // Quotes in a granted ability never introduce this outer `named` tail.
    let named = descriptor_tokens.iter().enumerate().find_map(|(i,t)| (t.is_word("named") && outside_quotes(descriptor_tokens,i)).then_some(i));
    let name_override = if let Some(i) = named {
        let name_tokens = trim_separators(&descriptor_tokens[i+1..]);
        if name_tokens.is_empty() { return None; }
        let name = crate::lexer::render_literal_token_slice(name_tokens).trim().to_string();
        descriptor_tokens = trim_separators(&descriptor_tokens[..i]);
        Some(name)
    } else { None };
    let mut descriptor_words = parser_token_word_refs(descriptor_tokens);
    while matches!(descriptor_words.first(), Some(&"a" | &"an" | &"the")) { descriptor_words.remove(0); }
    let mut leading_supertypes = Vec::new();
    let mut base_power_toughness = None;
    if let Some(pt) = super::parse_become_leading_pt_shape(&descriptor_words, descriptor_tokens) {
        base_power_toughness = Some((pt.power, pt.toughness));
        leading_supertypes = pt.leading_supertypes;
        descriptor_words.drain(..pt.value_word_count);
    }
    // `blue and has base power and toughness 5/4` changes only color and
    // size. It does not turn a noncreature into a creature or erase its types.
    let all_words = parser_token_word_refs(tokens);
    let mut normalized = all_words.clone();
    if let Some(i) = normalized.windows(4).position(|w| w == ["and", "has", "base", "power"]) {
        normalized.remove(i+1); normalized[i] = "with";
    }
    if let Some(pt) = super::parse_become_base_pt_words(&normalized) {
        if base_power_toughness.is_some() || name_override.is_some() { return None; }
        base_power_toughness = Some((pt.power, pt.toughness));
        descriptor_words = pt.descriptor_words.to_vec();
        ability_tokens = &[];
    } else if ability_tokens.first().is_some_and(|t| t.is_word("base")) {
        // The existing complete P/T-with-abilities production owns this;
        // never reinterpret an unsupported size expression as an ability.
        return None;
    }
    let mut card_types = Vec::new();
    let mut subtypes = Vec::new();
    let mut supertypes = leading_supertypes;
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
    if card_types.is_empty() && !subtypes.iter().all(|s| s.belongs_to_family(crate::types::SubtypeFamily::Creature)) { return None; }
    if card_types.is_empty() && subtypes.is_empty() && supertypes.is_empty() && !color_stated { return None; }
    Some(ObjectTemplateShape { base_power_toughness, name_override, card_types, subtypes, supertypes, colors: color_stated.then_some(colors), ability_tokens, preserve_other_types, preserve_other_colors, remove_other_abilities })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn shape(text: &str) -> ObjectTemplateShape<'static> {
        let tokens = Box::leak(crate::lexer::lex_line(text, 0).unwrap().into_boxed_slice());
        parse_object_template_tokens(tokens).expect("complete object template")
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
    fn sized_subtypes_color_only_size_names_and_supertypes_are_complete() {
        let rogue = shape("a 3/2 Human Faerie Rogue");
        assert_eq!(rogue.base_power_toughness, Some((crate::effect::Value::Fixed(3), crate::effect::Value::Fixed(2))));
        assert!(rogue.card_types.is_empty());
        let blue = shape("blue and has base power and toughness 5/4");
        assert_eq!(blue.colors, Some(ColorSet::BLUE));
        assert!(blue.card_types.is_empty() && blue.subtypes.is_empty());
        assert_eq!(blue.base_power_toughness, Some((crate::effect::Value::Fixed(5), crate::effect::Value::Fixed(4))));
        let named = shape("a legendary Equipment artifact named Everflame, Heroes' Legacy");
        assert_eq!(named.card_types, vec![CardType::Artifact]);
        assert_eq!(named.subtypes, vec![Subtype::Equipment]);
        assert_eq!(named.supertypes, vec![Supertype::Legendary]);
        assert_eq!(named.name_override.as_deref(), Some("Everflame, Heroes' Legacy"));
        let hero = shape("a legendary Spider Hero in addition to its other types");
        assert!(hero.preserve_other_types);
        assert_eq!(hero.supertypes, vec![Supertype::Legendary]);
        let snow = shape("snow");
        assert_eq!(snow.supertypes, vec![Supertype::Snow]);
        assert!(snow.card_types.is_empty());
        let goat = shape("a black Demon in addition to its other colors and types");
        assert!(goat.preserve_other_types && goat.preserve_other_colors);
        let larcenist = shape("Treasure artifacts with \"{T}, Sacrifice this artifact: Add one mana of any color\" and lose all other abilities");
        assert!(larcenist.remove_other_abilities);
    }
    #[test]
    fn malformed_descriptors_are_not_partial_templates() {
        for text in ["a mysterious artifact with flying", "a Treasure with flying", "a creature with", "a creature with base power and toughness nonsense", "blue and gets +1/+1", "a Dragon gets +5/+3"] {
            assert!(parse_object_template_tokens(&crate::lexer::lex_line(text, 0).unwrap()).is_none(), "{text}");
        }
    }
}
