use super::*;

pub(super) fn parse_mana_spend_counter_shape(
    tokens: &[OwnedLexToken],
) -> Option<ManaSpendCounterShape<'_>> {
    let view = TokenWordView::new(tokens);
    let words = view.word_refs();
    let enter = first_word_choice(&words, &["enter", "enters"])?;
    if enter == 0 || enter + 2 > words.len() {
        return None;
    }
    let subject = token_slice_for_words(tokens, &view, 0, enter)?;
    if !mana_spend_counter_subject_matches(subject) {
        return None;
    }
    let mut tail: primitives::WordSliceInput<'_> = words.get(enter + 1..)?;
    crate::grammar::primitives::take_leaf(&mut tail, primitives::word_slice_exact("with"))?;
    if tail.is_empty() {
        return None;
    }
    let start = words.len().checked_sub(tail.len())?;
    Some(ManaSpendCounterShape {
        counter_tail_tokens: token_slice_for_words(tokens, &view, start, words.len())?,
    })
}

pub(super) fn mana_spend_counter_subject_matches(tokens: &[OwnedLexToken]) -> bool {
    let words = TokenWordView::new(tokens).word_refs();
    if crate::word_primitives::parse_sequence_complete(&words, &["it"]) {
        return true;
    }
    let mut input: primitives::WordSliceInput<'_> = &words;
    if primitives::word_slice_exact("that")
        .parse_next(&mut input)
        .is_err()
    {
        return false;
    }
    let Ok(noun) = take_word(&mut input) else {
        return false;
    };
    matches!(noun, "creature" | "spell" | "permanent" | "card")
        || leaf::parse_leaf_card_type_complete(noun).is_ok()
}

pub(super) fn parse_mana_spend_counter_tail(
    bonus: ManaSpendCounterShape<'_>,
) -> Option<(CounterType, u32)> {
    let tokens = bonus.counter_tail_tokens;
    let (count, used) = if tokens
        .first()
        .is_some_and(|token| token.is_any_word(&["a", "an"]))
        && tokens
            .get(1)
            .is_some_and(|token| token.is_word("additional"))
    {
        (1, 2)
    } else if tokens
        .first()
        .is_some_and(|token| token.is_word("additional"))
    {
        (1, 1)
    } else if let Some((parsed, number_used)) = parse_number(tokens) {
        let used = if tokens
            .get(number_used)
            .is_some_and(|token| token.is_word("additional"))
        {
            number_used + 1
        } else {
            number_used
        };
        (parsed, used)
    } else {
        return None;
    };
    let counter_type = parse_counter_type_from_tokens(tokens.get(used..)?)?;
    let view = TokenWordView::new(tokens);
    let words = view.word_refs();
    let counter = first_word_choice(&words, &["counter", "counters"])?;
    let tail = token_slice_for_words(tokens, &view, counter + 1, words.len())?;
    matches_any_exact_tokens(
        tail,
        &[
            &[],
            &["on", "it"],
            &["on", "that", "creature"],
            &["on", "that", "spell"],
            &["on", "that", "permanent"],
            &["on", "that", "card"],
        ],
    )
    .then_some((counter_type, count))
}
