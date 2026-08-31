use super::*;

pub(super) fn parse_mana_spend_bonus_shape(
    tokens: &[OwnedLexToken],
) -> Option<ManaSpendBonusShape<'_>> {
    let view = TokenWordView::new(tokens);
    let words = view.word_refs();
    let (spec_start, condition) = parse_mana_spend_bonus_condition_prefix(&words)?;
    let spell_offset = first_word_choice(words.get(spec_start..)?, &["spell", "spells"])?;
    if spell_offset == 0 {
        return None;
    }
    let spell_word_end = spec_start + spell_offset + 1;
    let head_end = view.token_index_after_words(spell_word_end)?;
    let after_head = tokens.get(head_end..)?;
    let mut input = LexStream::new(after_head);
    let skipped: &[OwnedLexToken] = crate::grammar::primitives::take_leaf(
        &mut input,
        repeat_till(0.., any.void(), peek(primitives::comma()).void())
            .map(|((), ())| ())
            .take(),
    )?;
    crate::grammar::primitives::take_leaf(&mut input, primitives::comma())?;
    let bonus_start = head_end + skipped.len() + 1;
    Some(ManaSpendBonusShape {
        spec_tokens: token_slice_for_words(tokens, &view, spec_start, spec_start + spell_offset)?,
        bonus_tokens: tokens.get(bonus_start..)?,
        condition,
    })
}

pub(super) fn parse_mana_spend_bonus_condition_prefix(
    words: &[&str],
) -> Option<(usize, ManaSpendBonusCondition)> {
    let candidates = [
        (
            &["if", "this", "mana", "is", "spent", "to", "cast"] as &[&str],
            ManaSpendBonusCondition::IfThisManaIsSpentToCast,
        ),
        (
            &["if", "that", "mana", "is", "spent", "to", "cast"] as &[&str],
            ManaSpendBonusCondition::IfThatManaIsSpentToCast,
        ),
        (
            &["if", "this", "mana", "is", "spent", "on"] as &[&str],
            ManaSpendBonusCondition::IfThisManaIsSpentOn,
        ),
        (
            &["if", "that", "mana", "is", "spent", "on"] as &[&str],
            ManaSpendBonusCondition::IfThatManaIsSpentOn,
        ),
        (
            WHEN_MANA_SPENT_SPELL_PREFIXES[0],
            ManaSpendBonusCondition::WhenYouSpendThisManaToCast,
        ),
    ];
    candidates.into_iter().find_map(|(prefix, condition)| {
        let mut input: primitives::WordSliceInput<'_> = words;
        crate::grammar::primitives::take_leaf(&mut input, |input: &mut _| {
            parse_phrase_words(input, prefix)
        })?;
        Some((words.len().saturating_sub(input.len()), condition))
    })
}

pub(super) fn strip_mana_spend_duration_grant_suffix(
    tokens: &[OwnedLexToken],
) -> (
    &[OwnedLexToken],
    Vec<(StaticAbilityId, ManaSpendAbilityGrantDuration)>,
) {
    let view = TokenWordView::new(tokens);
    let words = view.word_refs();
    let suffixes: &[&[&str]] = &[
        &["and", "gains", "hexproof", "until", "your", "next", "turn"],
        &[
            "and", "it", "gains", "hexproof", "until", "your", "next", "turn",
        ],
    ];
    let Some(start) = last_exact_suffix_offset(&words, suffixes) else {
        return (tokens, Vec::new());
    };
    let Some(primary) = token_slice_for_words(tokens, &view, 0, start) else {
        return (tokens, Vec::new());
    };
    (
        primary,
        vec![(
            StaticAbilityId::Hexproof,
            ManaSpendAbilityGrantDuration::UntilYourNextTurn,
        )],
    )
}
