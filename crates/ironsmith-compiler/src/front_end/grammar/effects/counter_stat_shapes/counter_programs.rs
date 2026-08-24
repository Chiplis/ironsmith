use super::*;

pub fn parse_counter_reference(tokens: &[OwnedLexToken]) -> Option<CounterReferenceShape<'_>> {
    let words = TokenWordView::new(tokens).word_refs();
    let prefix = parse_prefix(&words, &[&["for", "each"]])?;
    let counter = find_word(&words[prefix.end..], &["counter", "counters"])? + prefix.end;
    if counter <= prefix.end || words.get(counter + 1) != Some(&"on") {
        return None;
    }
    let reference = words.get(counter + 2..)?;
    const SOURCE: &[&[&str]] = &[
        &["it"],
        &["this"],
        &["this", "artifact"],
        &["this", "aura"],
        &["this", "battle"],
        &["this", "card"],
        &["this", "creature"],
        &["this", "enchantment"],
        &["this", "land"],
        &["this", "permanent"],
        &["this", "planeswalker"],
        &["this", "source"],
    ];
    const TAGGED: &[&[&str]] = &[
        &["that"],
        &["that", "creature"],
        &["that", "permanent"],
        &["that", "object"],
        &["those"],
        &["those", "creatures"],
        &["those", "permanents"],
    ];
    let type_start = TokenWordView::new(tokens).token_index_after_words(prefix.end)?;
    let type_end = TokenWordView::new(tokens).token_index_after_words(counter + 1)?;
    let counter_type_tokens = tokens.get(type_start..type_end)?;
    if parse_exact(reference, SOURCE).is_some() {
        Some(CounterReferenceShape::Source {
            counter_type_tokens,
        })
    } else if parse_exact(reference, TAGGED).is_some() {
        Some(CounterReferenceShape::Tagged {
            counter_type_tokens,
        })
    } else {
        None
    }
}
