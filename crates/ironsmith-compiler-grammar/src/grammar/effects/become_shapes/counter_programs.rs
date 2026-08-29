use super::*;

pub fn parse_counter_state_pronoun_tokens(tokens: &[OwnedLexToken]) -> bool {
    [
        &["counter", "on", "it"][..],
        &["counter", "on", "them"],
        &["counters", "on", "it"],
        &["counters", "on", "them"],
    ]
    .iter()
    .any(|phrase| primitives::find_prefix(tokens, || primitives::phrase(phrase).void()).is_some())
}

pub(super) fn parse_become_iterated_counter_value_words(words: &[&str]) -> Option<Value> {
    let mut index = usize::from(words.first().is_some_and(|word| *word == "the"));
    if !permission_shapes::starts_at_words(words, index, &["number", "of"]) {
        return None;
    }
    index += 2;

    let counter_offset =
        crate::word_primitives::select_word_position(words.get(index..)?, |word| {
            matches!(word, "counter" | "counters")
        })?;
    if counter_offset > 2 {
        return None;
    }
    let counter_word = index + counter_offset;
    let counter_type = (counter_word > index)
        .then(|| crate::grammar::filters::parse_counter_type_words(&words[index..=counter_word]))
        .flatten();
    let reference_words = words.get(counter_word + 1..)?;
    if !crate::word_primitives::parse_any_sequence_complete(
        reference_words,
        &[
            &["on", "it"],
            &["on", "them"],
            &["on", "each", "of", "them"],
        ],
    ) {
        return None;
    }

    Some(Value::CountersOn(
        Box::new(ChooseSpec::Iterated),
        counter_type,
    ))
}
