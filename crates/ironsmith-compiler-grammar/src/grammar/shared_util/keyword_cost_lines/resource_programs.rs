use super::*;

pub(super) fn keyword_cost_clause(
    tokens: &[OwnedLexToken],
    first_cost_token: usize,
    boundary: ReminderBoundary,
) -> &[OwnedLexToken] {
    let mut start = first_cost_token.min(tokens.len());
    if tokens
        .get(start)
        .is_some_and(|token| matches!(token.kind, TokenKind::Dash | TokenKind::EmDash))
    {
        start += 1;
    }
    let tail = tokens.get(start..).unwrap_or_default();
    let view = TokenWordView::new(tail);
    let words = view.word_refs();
    let reminder_word = permission_shapes::find_words(&words, &["you", "may", "pay"])
        .or_else(|| permission_shapes::find_words(&words, &["you", "may"]));
    let reminder_token = reminder_word
        .and_then(|word| view.token_start_indices().get(word).copied())
        .unwrap_or(tail.len());
    let period = match boundary {
        ReminderBoundary::MayPay => tail.len(),
        ReminderBoundary::MayPayOrPeriod => {
            first_kind_after(tail, 0, TokenKind::Period).unwrap_or(tail.len())
        }
    };
    trim_edge_commas(&tail[..reminder_token.min(period)])
}

pub(super) fn morph_cost_clause(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    let tail = tokens.get(1..).unwrap_or_default();
    let view = TokenWordView::new(tail);
    let words = view.word_refs();
    let reminder_word = permission_shapes::find_words(&words, &["you", "may", "cast"])
        .or_else(|| permission_shapes::find_words(&words, &["turn", "it", "face", "up"]));
    let reminder = reminder_word
        .and_then(|word| view.token_start_indices().get(word).copied())
        .unwrap_or(tail.len());
    let period = first_kind_after(tail, 0, TokenKind::Period).unwrap_or(tail.len());
    trim_edge_commas(&tail[..reminder.min(period)])
}

pub(super) fn ensure_mana_component(
    parsed: ironsmith_core::TotalCost<CompilerCost>,
    mana_cost: crate::mana::ManaCost,
) -> ironsmith_core::TotalCost<CompilerCost> {
    if parsed.mana_cost().is_some() {
        return parsed;
    }
    let mut components = parsed.costs().to_vec();
    components.insert(0, CompilerCost::Mana(mana_cost));
    ironsmith_core::TotalCost::from_costs(components)
}
