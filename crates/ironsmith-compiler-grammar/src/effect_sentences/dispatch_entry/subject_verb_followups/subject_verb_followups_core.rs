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
    let IfResultPredicate::DieValue(_) = &prefix.predicate else {
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
    let [EffectAst::Conditionals(ConditionalEffectAst::IfResult { predicate, effects })] =
        sentence_effects.as_mut_slice()
    else {
        return Ok(None);
    };
    if predicate != &prefix.predicate
        || effects.is_empty()
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

/// "You may pay {X}. If you do, draw X cards. X can't be greater than the
/// amount of life you gained this turn." (Shanna, Purifying Blade): the
/// trailing sentence bounds the X the paying player announces for the
/// preceding `{X}` payment (CR 107.3a: the player chooses X). It is the
/// typed inclusive maximum of that payment, not a separate instruction.
pub(super) fn pre_rule_x_maximum_followup(
    state: &mut SentenceDispatchState<'_>,
    _sentences: &[SentenceInput],
    _sentence_idx: usize,
    sentence_tokens: &[OwnedLexToken],
) -> Result<Option<PreParseFollowupResult>, CardTextError> {
    let tokens = crate::util::trim_edge_punctuation_tokens(sentence_tokens);
    let [x, negation, be, greater, than, value_tokens @ ..] = tokens else {
        return Ok(None);
    };
    if !x.is_word("x")
        || !(negation.is_word("can't") || negation.is_word("cant") || negation.is_word("cannot"))
        || !be.is_word("be")
        || !greater.is_word("greater")
        || !than.is_word("than")
        || value_tokens.is_empty()
    {
        return Ok(None);
    }
    let Some((maximum, used)) = crate::util::parse_value(value_tokens) else {
        return Ok(None);
    };
    if used != value_tokens.len() {
        return Ok(None);
    }
    let Some(slot) = last_unbounded_x_payment_maximum_mut(state.effects) else {
        return Ok(None);
    };
    *slot = Some(maximum);
    Ok(Some(PreParseFollowupResult::Handled {
        consumed_sentences: 1,
        route: None,
    }))
}

fn is_unbounded_x_payment(effect: &EffectAst) -> bool {
    matches!(
        effect,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Mana(ManaActionAst::PayMana {
                cost,
                x_value: None,
                x_maximum: None,
                independent_x_choice: false,

            }),
            ..
        }) if cost.has_x()
    )
}

fn last_unbounded_x_payment_maximum_mut(
    effects: &mut [EffectAst],
) -> Option<&mut Option<crate::effect::Value>> {
    for effect in effects.iter_mut().rev() {
        if is_unbounded_x_payment(effect) {
            let EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::Mana(ManaActionAst::PayMana { x_maximum, .. }),
                ..
            }) = effect
            else {
                return None;
            };
            return Some(x_maximum);
        }
        if let Some(children) = token_copy_followup_container_effects_mut(effect)
            && let Some(found) = last_unbounded_x_payment_maximum_mut(children)
        {
            return Some(found);
        }
    }
    None
}
