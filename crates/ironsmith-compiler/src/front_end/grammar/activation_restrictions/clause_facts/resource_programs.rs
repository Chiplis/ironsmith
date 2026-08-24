use super::*;

pub fn parse_mana_retention_tail_words(words: &[&str]) -> Option<ManaRetentionTailKind> {
    if let Some(unspent) = super::super::parse_unspent_mana_retention_tail_words(words) {
        return Some(ManaRetentionTailKind::Unspent(unspent));
    }
    exact_any(
        words,
        &[
            &["lose", "this", "mana", "as", "steps"],
            &[
                "lose", "this", "mana", "as", "steps", "and", "phases", "end",
            ],
        ],
    )
    .then_some(ManaRetentionTailKind::ThisMana)
}

pub fn parse_mana_retention_negated_clause_words(
    words: &[&str],
) -> Option<ManaRetentionNegatedClause> {
    let tail = prefix_remainder(words, &["you", "dont"])
        .or_else(|| prefix_remainder(words, &["you", "don't"]))
        .or_else(|| prefix_remainder(words, &["you", "do", "not"]))?;
    Some(ManaRetentionNegatedClause {
        tail: parse_mana_retention_tail_words(tail)?,
    })
}
