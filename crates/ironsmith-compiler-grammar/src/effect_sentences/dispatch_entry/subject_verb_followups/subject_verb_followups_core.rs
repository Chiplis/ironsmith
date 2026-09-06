use super::*;

pub(super) fn pre_rule_otherwise_followup(
    _state: &mut SentenceDispatchState<'_>,
    _sentences: &[SentenceInput],
    _sentence_idx: usize,
    sentence_tokens: &[OwnedLexToken],
) -> Result<Option<PreParseFollowupResult>, CardTextError> {
    let Some(without_otherwise) = strip_otherwise_sentence_prefix(sentence_tokens) else {
        return Ok(None);
    };
    let mut plan = SentenceParsePlan::new(rewrite_otherwise_referential_subject(without_otherwise));
    plan.wrap_if_result = Some(IfResultPredicate::Otherwise);
    Ok(Some(PreParseFollowupResult::Plan(plan)))
}

pub(super) fn is_destroy_those_creatures_sentence(tokens: &[OwnedLexToken]) -> bool {
    followup_shapes::is_destroy_those_creatures_followup(tokens)
}

pub(super) fn pre_rule_destroy_those_creatures_followup(
    state: &mut SentenceDispatchState<'_>,
    _sentences: &[SentenceInput],
    _sentence_idx: usize,
    sentence_tokens: &[OwnedLexToken],
) -> Result<Option<PreParseFollowupResult>, CardTextError> {
    if !is_destroy_those_creatures_sentence(sentence_tokens) {
        return Ok(None);
    }
    let Some(filter) = last_remove_abilities_all_filter(state.effects) else {
        return Ok(None);
    };
    state
        .effects
        .push(EffectAst::subject_verb_destroy_all(filter));
    Ok(Some(PreParseFollowupResult::Handled {
        consumed_sentences: 1,
        route: None,
    }))
}

/// Retain an authored label inside an exact numeric-result row.
///
/// The document grammar keeps `N | ...` rows attached to the roll instruction,
/// while the ordinary statement-label parser intentionally strips a label such
/// as `Trapped! —` before parsing its executable body. Reattach that label only
/// when both pieces are still proven here: the outer typed numeric predicate and
/// the inner label/body split from the same source sentence.
pub(super) fn post_rule_numeric_result_branch_label(
    _state: &mut SentenceDispatchState<'_>,
    sentences: &[SentenceInput],
    sentence_idx: usize,
    sentence_tokens: &[OwnedLexToken],
    sentence_effects: &mut Vec<EffectAst>,
) -> Result<Option<PostParseFollowupResult>, CardTextError> {
    let Some(prefix) =
        crate::grammar::structure::split_leading_result_prefix_lexed(sentence_tokens)
    else {
        return Ok(None);
    };
    let IfResultPredicate::Value(_) = &prefix.predicate else {
        return Ok(None);
    };
    let authored_tokens = sentences
        .get(sentence_idx)
        .map(SentenceInput::lexed)
        .unwrap_or(sentence_tokens);
    let Some(authored_prefix) =
        crate::grammar::structure::split_leading_result_prefix_lexed(authored_tokens)
    else {
        return Ok(None);
    };
    let Some(label_split) = crate::grammar::document_shapes::parse_statement_label_split_tokens(
        authored_prefix.trailing_tokens,
    ) else {
        return Ok(None);
    };
    let label = crate::lexer::render_token_slice(label_split.label_tokens)
        .trim()
        .to_string();
    if label.is_empty() {
        return Ok(None);
    }
    let [EffectAst::Conditionals(ConditionalEffectAst::IfResult { predicate, effects })] = sentence_effects.as_mut_slice() else {
        return Ok(None);
    };
    if predicate != &prefix.predicate || effects.is_empty()
        || matches!(effects.as_slice(), [EffectAst::ResultBranchLabel { .. }])
    {
        return Ok(None);
    }
    let nested = std::mem::take(effects);
    effects.push(EffectAst::ResultBranchLabel {
        label,
        effects: nested,
    });
    // This is a local annotation of the current sentence, not a follow-up
    // consumed into an earlier effect. Let ordinary dispatch append it.
    Ok(Some(PostParseFollowupResult::Annotated))
}
