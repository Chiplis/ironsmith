use super::*;

const STRIKE_WORD: &str = "strike";
const ANOTHER_WORD: &str = "another";
const STRIKE_COUNTER_PREFIXES: &[(&str, CounterType)] = &[
    ("double", CounterType::DoubleStrike),
    ("first", CounterType::FirstStrike),
];

pub fn parse_counter_type_from_descriptor_tokens(tokens: &[OwnedLexToken]) -> Option<CounterType> {
    let words = crate::lexer::token_word_refs(tokens);
    let last = *words.last()?;
    if let Some(counter_type) = parse_counter_type_word(last) {
        return Some(counter_type);
    }
    if last == STRIKE_WORD && words.len() >= 2 {
        return strike_counter_type_from_prefix(words[words.len() - 2]);
    }
    if last == ANOTHER_WORD || crate::grammar::leaf::parse_number_complete(last).is_ok() {
        return None;
    }
    if last
        .chars()
        .all(|character| character.is_ascii_alphabetic())
    {
        return Some(CounterType::Named(intern_counter_name(last).into()));
    }
    None
}

fn strike_counter_type_from_prefix(word: &str) -> Option<CounterType> {
    STRIKE_COUNTER_PREFIXES
        .iter()
        .find_map(|(prefix, counter_type)| (*prefix == word).then_some(*counter_type))
}
