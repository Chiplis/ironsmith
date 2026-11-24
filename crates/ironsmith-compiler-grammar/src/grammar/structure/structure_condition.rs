use super::*;

pub(super) fn split_trailing_predicate_clause_lexed<'a>(
    tokens: &'a [OwnedLexToken],
    keyword: &'static str,
) -> Option<TrailingIfClauseSpec<'a>> {
    let mut split_idx = rfind_unquoted_dynamic_word(tokens, keyword)?;
    // "<effect> if <A> or if <B>": the alternatives form one trailing
    // predicate, so split at the keyword that opens the disjunction.
    while split_idx >= 2 && tokens[split_idx - 1].is_word("or")
        && let Some(earlier) = rfind_unquoted_dynamic_word(&tokens[..split_idx - 1], keyword)
        && earlier > 0
        && crate::word_claims::probe_only(|| parse_predicate_with_grammar_entrypoint_lexed(trim_lexed_commas(&tokens[earlier + 1..]))).is_ok()
    {
        split_idx = earlier;
    }
    if split_idx == 0 || split_idx + 1 >= tokens.len() {
        return None;
    }
    // A sentence-initial keyword leads its own sentence ("... reveal it. If
    // it's a creature card, that player discards it", Breathstealer's
    // Crypt); it never trails the previous one, and the rest of its
    // sentence is no predicate.
    if tokens[split_idx - 1].is_period() {
        return None;
    }

    let predicate_tokens = trim_lexed_commas(&tokens[split_idx + 1..]);
    if predicate_tokens.is_empty() {
        return None;
    }
    let mut predicate = parse_predicate_with_grammar_entrypoint_lexed(predicate_tokens).ok()?;

    let leading_tokens = trim_lexed_commas(&tokens[..split_idx]);
    if leading_tokens.is_empty() {
        return None;
    }
    // "Counter target spell if it was kicked." (Ertai's Trickery): the
    // pronoun's antecedent is the target spell the leading instruction just
    // named, not the resolving spell (CR 702.33d: "kicked" describes the
    // spell that paid its kicker). The bare predicate reader cannot see it.
    if matches!(predicate, PredicateAst::ThisSpellWasKicked)
        && crate::lexer::token_word_refs(predicate_tokens).first() == Some(&"it")
        && crate::lexer::token_word_refs(leading_tokens)
            .windows(2)
            .any(|pair| pair == ["target", "spell"])
    {
        predicate = PredicateAst::TargetWasKicked;
    }
    // The keyword that opens the predicate just read, claimed with it.
    crate::word_claims::claim_tokens(&tokens[split_idx..=split_idx]);

    Some(TrailingIfClauseSpec {
        leading_tokens,
        predicate_tokens,
        predicate,
    })
}

pub fn parse_trailing_instead_if_predicate_lexed(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let trimmed = trim_lexed_commas(tokens);
    let trailing = split_trailing_predicate_clause_lexed(trimmed, "if")?;
    if !trailing
        .leading_tokens
        .iter()
        .any(|token| structure_token_is(token, "instead"))
    {
        return None;
    }
    Some(trailing.predicate)
}
