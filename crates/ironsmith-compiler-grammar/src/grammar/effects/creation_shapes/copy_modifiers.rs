use crate::cards::builders::CardTextError;
use crate::color::ColorSet;
use crate::model::CompilerStaticAbilityCore as StaticAbility;
use crate::static_abilities::{Anthem, AnthemCountExpression, AnthemValue};
use crate::target::{ObjectFilter, PlayerFilter};
use crate::types::{CardType, Subtype, Supertype};
use crate::zone::Zone;

use super::{
    CreationPhrase, CreationWordClass, CreationWords, parse_pt_word, parse_unsigned_pt_word,
};

#[derive(Debug, Clone, PartialEq, Default)]
pub struct CopyModifierSpec {
    pub set_colors: Option<ColorSet>,
    pub set_card_types: Option<Vec<CardType>>,
    pub set_subtypes: Option<Vec<Subtype>>,
    pub added_card_types: Vec<CardType>,
    pub added_subtypes: Vec<Subtype>,
    pub removed_supertypes: Vec<Supertype>,
    pub set_base_power_toughness: Option<(i32, i32)>,
    /// The copy's base power and toughness are the respective totals of the
    /// authored collection from which its copy source is chosen.
    pub set_base_power_toughness_to_source_totals: bool,
    pub starting_loyalty: Option<u32>,
    pub granted_abilities: Vec<StaticAbility>,
    /// "except it has haste and loses soulbond": the copy is created without
    /// the soulbond pairing ability.
    pub loses_soulbond: bool,
}

fn last_class_location(words: &[&str], class: CreationWordClass) -> Option<usize> {
    let mut cursor = 0usize;
    let mut result = None;
    while cursor < words.len() {
        let Some(relative) = CreationWords::new(&words[cursor..]).location(class) else {
            break;
        };
        let location = cursor + relative;
        result = Some(location);
        cursor = location + 1;
    }
    result
}

fn push_unique<T: PartialEq>(values: &mut Vec<T>, value: T) {
    if !values.iter().any(|existing| existing == &value) {
        values.push(value);
    }
}

fn contains_words(words: &[&str], phrase: &[&str]) -> bool {
    crate::word_primitives::sequence_occurs(words, phrase)
}

#[cfg(test)]
#[path = "copy_modifiers_inline_tests.rs"]
mod tests;

#[path = "copy_modifiers/object_action_programs.rs"]
mod object_action_programs;
pub use object_action_programs::parse_copy_modifier_words;
