use super::*;

pub(super) fn parse_controlled_creature_power_shape(
    tokens: &[OwnedLexToken],
) -> Option<ControlledCreaturePowerShape<'_>> {
    let condition_tokens = parse_activate_only_if_you_control_tail_tokens(tokens)?;
    let tail =
        parse_control_relation_tail_clause(condition_tokens, activate_only_you_control_options())?;
    let view = TokenWordView::new(tail.tokens());
    let words = view.word_refs();
    let power = phrase_offset_words(&words, &["with", "power"])?;
    let comparison_start = power + 2;
    if power == 0 || comparison_start >= words.len() {
        return None;
    }
    Some(ControlledCreaturePowerShape {
        object_tokens: token_slice_for_words(tail.tokens(), &view, 0, power)?,
        comparison_tokens: token_slice_for_words(
            tail.tokens(),
            &view,
            comparison_start,
            words.len(),
        )?,
    })
}

pub(super) fn parse_controlled_creature_power_condition(
    tokens: &[OwnedLexToken],
) -> Option<ConditionExpr> {
    let parsed = parse_controlled_creature_power_shape(tokens)?;
    if !matches_exact_tokens(parsed.object_tokens, &["creature"])
        && !matches_exact_tokens(parsed.object_tokens, &["a", "creature"])
        && !matches_exact_tokens(parsed.object_tokens, &["an", "creature"])
    {
        return None;
    }
    let comparison_words = TokenWordView::new(parsed.comparison_tokens).word_refs();
    let clause_words = TokenWordView::new(tokens).word_refs();
    let (comparison, used) = crate::grammar::primitives::probe_shape(
        parse_filter_comparison_tokens("power", &comparison_words, &clause_words),
    )??;
    (used == comparison_words.len()).then_some(ConditionExpr::YouControl(
        ObjectFilter::creature().with_power(comparison),
    ))
}

pub(super) fn parse_activate_only_if_you_control_tail_tokens(
    tokens: &[OwnedLexToken],
) -> Option<&[OwnedLexToken]> {
    let condition = parse_activate_only_if_tail_tokens(tokens)?;
    parse_control_relation_tail_clause(condition, activate_only_you_control_options())?;
    Some(condition)
}

pub(super) fn parse_activate_only_if_tail_tokens(
    tokens: &[OwnedLexToken],
) -> Option<&[OwnedLexToken]> {
    let view = TokenWordView::new(tokens);
    let words = view.word_refs();
    let mut input: primitives::WordSliceInput<'_> = &words;
    crate::grammar::primitives::take_leaf(&mut input, |input: &mut _| {
        parse_phrase_words(input, &["activate", "only", "if"])
    })?;
    let start = words.len().checked_sub(input.len())?;
    (start < words.len()).then_some(())?;
    token_slice_for_words(tokens, &view, start, words.len())
}

pub(super) fn parse_land_subtype_control_condition(
    control_tokens: &[OwnedLexToken],
) -> Option<ConditionExpr> {
    let object =
        parse_control_relation_tail_clause(control_tokens, activate_only_you_control_options())?;
    let mut subtypes = Vec::new();
    for word in TokenWordView::new(object.tokens()).word_refs() {
        if let Ok(subtype) = leaf::parse_leaf_subtype_flexible_complete(word) {
            crate::slice_primitives::push_unique(&mut subtypes, subtype);
        }
    }
    if subtypes.is_empty() {
        return None;
    }
    let mut combined = None;
    for subtype in subtypes {
        let next = ConditionExpr::YouControl(
            ObjectFilter::default()
                .with_type(crate::types::CardType::Land)
                .with_subtype(subtype),
        );
        combined = Some(match combined {
            Some(existing) => ConditionExpr::Or(Box::new(existing), Box::new(next)),
            None => next,
        });
    }
    combined
}

pub(super) fn token_slice_for_words<'a>(
    tokens: &'a [OwnedLexToken],
    view: &TokenWordView<'a>,
    start: usize,
    end: usize,
) -> Option<&'a [OwnedLexToken]> {
    Some(trim_lexed_commas(
        tokens.get(view.token_span_for_words(start, end)?)?,
    ))
}

pub(super) fn activate_only_you_control_options() -> ControlConditionOptions {
    ControlConditionOptions {
        allow_that_player: false,
        ..ControlConditionOptions::default()
    }
}
