//! Complete quotation-aware object descriptors, preserving unstated characteristics.
use crate::word_primitives::WordWindows as _;
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
    pub remove_other_supertypes: bool,
    pub remove_other_card_types: bool,
    pub remove_other_subtypes: bool,
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
    let mut remove_other_supertypes = false;
    let mut remove_other_card_types = false;
    let mut remove_other_subtypes = false;
    if prefix.len() < words.len() {
        let start = positions[prefix.len()].0;
        if outside_quotes(tokens, start) {
            preserve_other_colors = words[prefix.len()..].contains(&"colors");
            tokens = trim_separators(&tokens[..start]);
        } else { preserve_other_types = false; }
    } else {
        // Read the list of characteristic families independently. Quoted
        // abilities cannot supply the outer descriptor's clearing clause.
        if let Some(index) = words.windows(4).enumerate().rev().find_map(|(i, w)| {
            (w[0] == "and" && matches!(w[1], "lose" | "loses")
                && w[2..] == ["all", "other"] && outside_quotes(tokens, positions[i].0)).then_some(i)
        }) {
            let mut rest = &words[index + 4..];
            let mut seen = std::collections::BTreeSet::new();
            while !rest.is_empty() {
                if rest[0] == "and" { rest = &rest[1..]; if rest.is_empty() { return None; } }
                let (category, count) = if rest.starts_with(&["card", "types"]) {
                    ("card types", 2)
                } else {
                    match rest[0] {
                        "supertypes" => ("supertypes", 1),
                        "subtypes" => ("subtypes", 1),
                        "abilities" => ("abilities", 1),
                        _ => return None,
                    }
                };
                if !seen.insert(category) { return None; }
                rest = &rest[count..];
            }
            if seen.is_empty() { return None; }
            remove_other_supertypes = seen.contains("supertypes");
            remove_other_card_types = seen.contains("card types");
            remove_other_subtypes = seen.contains("subtypes");
            remove_other_abilities = seen.contains("abilities");
            let start = positions[index].0;
            crate::word_claims::claim_tokens(&tokens[start..]);
            tokens = trim_separators(&tokens[..start]);
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
        // The literal name is read whole as the characteristic.
        crate::word_claims::claim_tokens(&descriptor_tokens[i..]);
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
    if let Some(i) = normalized.word_windows(4).position(|w| w == ["and", "has", "base", "power"]) {
        normalized.remove(i+1); normalized[i] = "with";
    }
    if let Some(pt) = super::parse_become_base_pt_words(&normalized) {
        if base_power_toughness.is_some() || name_override.is_some() { return None; }
        // "a Plant with base power and toughness 0/8": "with" attaches the
        // stated base size read here.
        if let Some(i) = with { crate::word_claims::claim_tokens(&tokens[i..=i]); }
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
            crate::word_claims::claim_word(word);
            crate::word_claims::claim_word(next);
            if !subtypes.contains(&kind) { subtypes.push(kind); } index += 1;
        } else { return None; }
        index += 1;
    }
    // Bare creature subtypes retain existing card types. A Treasure subtype
    // never implies Creature, and an unspecified unrelated card type is not
    // invented. Explicit artifact/land/etc descriptors retain their own type.
    if card_types.is_empty() && !subtypes.iter().all(|s| s.belongs_to_family(crate::types::SubtypeFamily::Creature)) { return None; }
    if card_types.is_empty() && subtypes.is_empty() && supertypes.is_empty() && !color_stated { return None; }
    Some(ObjectTemplateShape { base_power_toughness, name_override, card_types, subtypes, supertypes, colors: color_stated.then_some(colors), ability_tokens, preserve_other_types, preserve_other_colors, remove_other_abilities, remove_other_supertypes, remove_other_card_types, remove_other_subtypes })
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
    #[test]
    fn characteristic_family_clearing_is_independent_and_quotation_aware() {
        let wasted = shape("a basic land named Wastes with \"{T}: Add {C}\" and loses all other supertypes, card types, subtypes, and abilities");
        assert_eq!(wasted.name_override.as_deref(), Some("Wastes"));
        assert_eq!(wasted.card_types, vec![CardType::Land]);
        assert_eq!(wasted.supertypes, vec![Supertype::Basic]);
        assert!(wasted.subtypes.is_empty());
        assert!(wasted.remove_other_supertypes && wasted.remove_other_card_types
            && wasted.remove_other_subtypes && wasted.remove_other_abilities);
        assert_eq!(crate::lexer::render_literal_token_slice(wasted.ability_tokens), "\"{T}: Add {C}\"");
        let isolated = shape("a legendary artifact named Relic with indestructible and loses all other subtypes");
        assert!(isolated.remove_other_subtypes);
        assert!(!isolated.remove_other_supertypes && !isolated.remove_other_card_types && !isolated.remove_other_abilities);
        let quoted = shape("an artifact with \"{T}: Target permanent becomes a land and loses all other supertypes and subtypes.\"");
        assert!(!quoted.remove_other_supertypes && !quoted.remove_other_subtypes);
        for bad in [
            "a land and loses all other subtypes and subtypes",
            "a land and loses all other supertypes and colors",
            "a land and loses all other abilities and",
        ] {
            assert!(parse_object_template_tokens(&crate::lexer::lex_line(bad, 0).unwrap()).is_none(), "{bad}");
        }
    }

}
