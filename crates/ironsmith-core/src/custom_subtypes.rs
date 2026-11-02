//! Explicit compilation vocabulary. The scope is restored even after a panic;
//! runtime type universes belong to GameState, never this parser scope.
use crate::tag::TagKeyWalk;
use crate::{Subtype, SubtypeFamily};
use std::cell::RefCell;

#[derive(Debug, Clone, Copy, Eq, TagKeyWalk)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CustomSubtype {
    name: [u8; 32],
    /// Canonical first association; use belongs_to_family for membership.
    pub family: SubtypeFamily,
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "custom_family_mask_is_empty"))]
    additional_families: u8,
}
// Associations describe where an identity is legal; they are not a second
// spelling identity. Old single-family artifacts still match newer filters.
impl PartialEq for CustomSubtype {
    fn eq(&self, other: &Self) -> bool { self.name == other.name }
}
impl std::hash::Hash for CustomSubtype {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) { std::hash::Hash::hash(&self.name, state); }
}
impl CustomSubtype {
    pub fn new(name: &str, family: SubtypeFamily) -> Result<Self, String> {
        let name = normalize_name(name)?;
        let mut bytes = [0; 32];
        bytes[..name.len()].copy_from_slice(name.as_bytes());
        Ok(Self {
            name: bytes,
            family,
            additional_families: 0,
        })
    }
    pub fn new_with_families(name: &str, families: &[SubtypeFamily]) -> Result<Self, String> {
        let Some(first) = families.first() else { return Err("A custom subtype needs at least one family association.".into()); };
        let mut subtype = Self::new(name, *first)?;
        for family in families { subtype = subtype.with_family(*family); }
        Ok(subtype)
    }
    /// Association order is not identity: all constructors canonicalize the
    /// same declared family set to the same Copy value and serialized payload.
    pub fn with_family(mut self, family: SubtypeFamily) -> Self {
        let mask = self.additional_families | custom_family_bit(self.family) | custom_family_bit(family);
        self.family = CUSTOM_FAMILIES.into_iter().find(|family| mask & custom_family_bit(*family) != 0).unwrap();
        self.additional_families = mask & !custom_family_bit(self.family);
        self
    }
    pub fn union_associations(mut self, other: Self) -> Self {
        debug_assert_eq!(self,other,"only equal subtype identities can share associations");
        for family in other.families() { self=self.with_family(family); }
        self
    }
    pub fn belongs_to_family(&self, family: SubtypeFamily) -> bool {
        self.family == family || self.additional_families & custom_family_bit(family) != 0
    }
    pub fn families(&self) -> impl Iterator<Item = SubtypeFamily> + '_ {
        CUSTOM_FAMILIES.into_iter().filter(|family| self.belongs_to_family(*family))
    }
    pub fn name(&self) -> &str {
        std::str::from_utf8(&self.name[..self.name.iter().position(|c| *c == 0).unwrap_or(32)])
            .unwrap_or("")
    }
    pub fn display_name(&self) -> String {
        let mut chars = self.name().chars();
        let Some(first) = chars.next() else { return String::new(); };
        first.to_uppercase().chain(chars).collect()
    }
}
/// Learn explicit associations in a game-local type universe. Returns true
/// when a new identity or association was added; never changes parser scope.
pub fn merge_subtype_associations(types: &mut Vec<Subtype>, incoming: Subtype) -> bool {
    let Subtype::Custom(incoming_custom) = incoming else { return false; };
    if let Some(Subtype::Custom(prior)) = types.iter_mut().find(|subtype| **subtype == incoming) {
        let mut merged = *prior;
        for family in incoming_custom.families() { merged = merged.with_family(family); }
        if prior.family == merged.family && prior.additional_families == merged.additional_families { return false; }
        *prior = merged;
    } else { types.push(incoming); }
    true
}

const CUSTOM_FAMILIES: [SubtypeFamily; 8] = [SubtypeFamily::Land, SubtypeFamily::Creature,
    SubtypeFamily::Artifact, SubtypeFamily::Enchantment, SubtypeFamily::Spell,
    SubtypeFamily::Planeswalker, SubtypeFamily::Battle, SubtypeFamily::Planar];
fn custom_family_bit(family: SubtypeFamily) -> u8 {
    1 << match family { SubtypeFamily::Land => 0, SubtypeFamily::Creature => 1,
        SubtypeFamily::Artifact => 2, SubtypeFamily::Enchantment => 3, SubtypeFamily::Spell => 4,
        SubtypeFamily::Planeswalker => 5, SubtypeFamily::Battle => 6, SubtypeFamily::Planar => 7 }
}
#[cfg(feature = "serde")]
fn custom_family_mask_is_empty(mask: &u8) -> bool { *mask == 0 }

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CustomSubtypeDefinition {
    pub singular: String,
    pub plural: String,
    pub family: SubtypeFamily,
}
pub fn normalize_name(value: &str) -> Result<String, String> {
    let value = value.split_whitespace().collect::<Vec<_>>().join(" ")
        .replace(['’', '‘'], "'").replace('−', "-").to_lowercase();
    let valid_word = |word: &str| {
        let mut chars = word.chars();
        if !chars.next().is_some_and(char::is_alphabetic) { return false; }
        let mut connector = false;
        for ch in chars {
            if ch.is_alphanumeric() { connector = false; }
            else if matches!(ch, '\'' | '-') && !connector { connector = true; }
            else { return false; }
        }
        !connector
    };
    if value.is_empty() || value.len() > 32 || !value.split(' ').all(valid_word) {
        return Err("Subtype forms must fit 32 UTF-8 bytes and contain words made of letters/numbers with internal hyphens or apostrophes.".into());
    }
    Ok(value)
}

fn parser_surfaces(name: &str) -> Vec<String> {
    let mut variants = vec![name.to_owned()];
    // Lexical word pieces omit apostrophes and may split internal hyphens.
    // Their aliases retain the exact one scoped subtype identity.
    let no_apostrophe = name.replace('\'', "");
    for alias in [no_apostrophe.clone(), no_apostrophe.replace('-', " "), no_apostrophe.replace('-', "")] {
        if !variants.contains(&alias) { variants.push(alias); }
    }
    variants
}

#[derive(Default)]
struct Vocabulary {
    aliases: Vec<(String, Subtype)>,
}
thread_local! { static VOCABULARY: RefCell<Vocabulary> = RefCell::default(); }
struct Guard(Option<Vocabulary>);
impl Drop for Guard {
    fn drop(&mut self) {
        VOCABULARY.with(|v| *v.borrow_mut() = self.0.take().unwrap());
    }
}
/// The adapter must validate official-word collisions before opening a scope.
pub fn with_vocabulary<T>(
    definitions: &[CustomSubtypeDefinition],
    f: impl FnOnce() -> T,
) -> Result<T, String> {
    let mut next = Vocabulary::default();
    let mut types: Vec<(String, String, CustomSubtype)> = Vec::new();
    for def in definitions {
        let singular = normalize_name(&def.singular)?;
        let plural = normalize_name(&def.plural)?;
        if let Some((_, prior_plural, subtype)) = types.iter_mut().find(|(name, _, _)| name == &singular) {
            if *prior_plural != plural { return Err(format!("Conflicting subtype plural: {singular}")); }
            *subtype = subtype.with_family(def.family);
        } else {
            types.push((singular.clone(), plural, CustomSubtype::new(&singular, def.family)?));
        }
    }
    for (singular, plural, custom) in types {
        let subtype = Subtype::Custom(custom);
        for word in [singular, plural] {
            for word in parser_surfaces(&word) {
                if let Some((_, prior)) = next.aliases.iter().find(|(alias, _)| alias == &word) {
                    if *prior != subtype { return Err(format!("Ambiguous subtype form: {word}")); }
                } else { next.aliases.push((word, subtype)); }
            }
        }
    }
    let previous = VOCABULARY.with(|v| v.replace(next));
    let _guard = Guard(Some(previous));
    Ok(f())
}
pub fn lookup(word: &str) -> Option<Subtype> {
    let word = normalize_name(word).ok()?;
    VOCABULARY.with(|v| {
        let v = v.borrow();
        let result = v
            .aliases
            .iter()
            .find(|(alias, _)| alias == &word)
            .map(|(_, ty)| *ty)?;
        Some(result)
    })
}

/// Longest explicitly scoped type phrase, preserving unknown neighbors.
pub fn lookup_prefix(words: &[&str]) -> Option<(Subtype, usize)> {
    for end in (1..=words.len()).rev() {
        if let Some(subtype) = lookup(&words[..end].join(" ")) { return Some((subtype, end)); }
    }
    None
}
