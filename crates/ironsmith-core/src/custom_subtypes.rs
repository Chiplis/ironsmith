//! Explicit compilation vocabulary. The scope is restored even after a panic;
//! runtime type universes belong to GameState, never this parser scope.
use crate::{Subtype, SubtypeFamily};
use crate::tag::TagKeyWalk;
use std::cell::RefCell;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, TagKeyWalk)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CustomSubtype {
    name: [u8; 32],
    pub family: SubtypeFamily,
}
impl CustomSubtype {
    pub fn new(name: &str, family: SubtypeFamily) -> Result<Self, String> {
        let name = normalize_name(name)?;
        let mut bytes = [0; 32];
        bytes[..name.len()].copy_from_slice(name.as_bytes());
        Ok(Self { name: bytes, family })
    }
    pub fn name(&self) -> &str {
        std::str::from_utf8(&self.name[..self.name.iter().position(|c| *c == 0).unwrap_or(32)]).unwrap_or("")
    }
    pub fn display_name(&self) -> String {
        let mut name = self.name().to_owned();
        if let Some(first) = name.get_mut(..1) { first.make_ascii_uppercase(); }
        name
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CustomSubtypeDefinition {
    pub singular: String,
    pub plural: String,
    pub family: SubtypeFamily,
}
pub fn normalize_name(value: &str) -> Result<String, String> {
    let value = value.trim().to_ascii_lowercase();
    if value.is_empty() || value.len() > 32 || !value.bytes().all(|b| b.is_ascii_alphabetic()) {
        return Err("Subtype forms must contain 1–32 letters (one word).".into());
    }
    Ok(value)
}
#[derive(Default)]
struct Vocabulary { aliases: Vec<(String, Subtype)>, used: Vec<Subtype> }
thread_local! { static VOCABULARY: RefCell<Vocabulary> = RefCell::default(); }
struct Guard(Option<Vocabulary>);
impl Drop for Guard { fn drop(&mut self) { VOCABULARY.with(|v| *v.borrow_mut() = self.0.take().unwrap()); } }
/// The adapter must validate official-word collisions before opening a scope.
pub fn with_vocabulary<T>(definitions: &[CustomSubtypeDefinition], f: impl FnOnce() -> T) -> Result<T, String> {
    let mut next = Vocabulary::default();
    for def in definitions {
        let subtype = Subtype::Custom(CustomSubtype::new(&def.singular, def.family)?);
        for word in [&def.singular, &def.plural] {
            let word = normalize_name(word)?;
            if let Some((_, prior)) = next.aliases.iter().find(|(alias, _)| alias == &word) {
                if *prior != subtype { return Err(format!("Ambiguous subtype form: {word}")); }
            } else { next.aliases.push((word, subtype)); }
        }
    }
    let previous = VOCABULARY.with(|v| v.replace(next));
    let _guard = Guard(Some(previous));
    Ok(f())
}
pub fn lookup(word: &str) -> Option<Subtype> {
    VOCABULARY.with(|v| {
        let mut v = v.borrow_mut();
        let result = v.aliases.iter().find(|(alias, _)| alias.eq_ignore_ascii_case(word)).map(|(_, ty)| *ty)?;
        if !v.used.contains(&result) { v.used.push(result); }
        Some(result)
    })
}
pub fn used_subtypes() -> Vec<Subtype> { VOCABULARY.with(|v| v.borrow().used.clone()) }
