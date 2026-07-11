use super::*;

pub(crate) use crate::runtime_backend::activation_helpers::{
    is_mana_pool_tail_tokens, parse_add_mana, parse_any_combination_mana_colors,
    parse_or_mana_color_choices,
};

const STRIKE_WORD: &str = "strike";
const ANOTHER_WORD: &str = "another";
const STRIKE_COUNTER_PREFIXES: &[(&str, CounterType)] = &[
    ("double", CounterType::DoubleStrike),
    ("first", CounterType::FirstStrike),
];

pub(crate) fn parse_counter_type_from_descriptor_tokens(
    tokens: &[OwnedLexToken],
) -> Option<CounterType> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    let last = *words.last()?;
    if let Some(counter_type) = parse_counter_type_word(last) {
        return Some(counter_type);
    }
    if last == STRIKE_WORD && words.len() >= 2 {
        return strike_counter_type_from_prefix(words[words.len() - 2]);
    }
    if last == ANOTHER_WORD
        || crate::runtime_backend::grammar::leaf::parse_number_complete(last).is_ok()
    {
        return None;
    }
    if last
        .chars()
        .all(|character| character.is_ascii_alphabetic())
    {
        return Some(CounterType::Named(intern_counter_name(last)));
    }
    None
}

fn strike_counter_type_from_prefix(word: &str) -> Option<CounterType> {
    STRIKE_COUNTER_PREFIXES
        .iter()
        .find_map(|(prefix, counter_type)| (*prefix == word).then(|| counter_type.clone()))
}
