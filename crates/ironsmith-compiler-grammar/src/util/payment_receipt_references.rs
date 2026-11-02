//! Exact source-attributed optional-cost participants are retained payment facts.
use super::*;
pub(crate) fn selected_tap_target(tokens: &[OwnedLexToken]) -> Option<crate::tag::TagRef> {
    let words = crate::lexer::TokenWordView::new(tokens).word_refs();
    let label = words
        .strip_prefix(&["the", "selected"])?
        .strip_suffix(&["payment", "object"])?;
    if label.is_empty() {
        return None;
    }
    crate::word_claims::claim_tokens(tokens);
    Some(crate::tag::declared_key(
        ironsmith_core::tag::selected_additional_cost_tap(&label.join(" ")),
    ))
}
pub(crate) fn tap_target(tokens: &[OwnedLexToken]) -> Option<crate::tag::TagRef> {
    let words = crate::lexer::TokenWordView::new(tokens).word_refs();
    let rest = words.strip_prefix(&["the", "creature", "tapped", "to", "pay", "its"])?;
    let label = rest.strip_suffix(&["additional", "cost"])?;
    if label.is_empty()
        || label.iter().any(|word| {
            !word
                .chars()
                .all(|c| c.is_alphabetic() || c == '\'' || c == '-')
        })
    {
        return None;
    }
    crate::word_claims::claim_tokens(tokens);
    Some(crate::tag::declared_key(
        ironsmith_core::tag::additional_cost_tap_receipt(&label.join(" ")),
    ))
}
pub(crate) fn tap_predicate(
    tokens: &[OwnedLexToken],
) -> Result<Option<crate::cards::builders::PredicateAst>, CardTextError> {
    let tokens = crate::util::trim_edge_punctuation_tokens(tokens);
    let view = crate::lexer::TokenWordView::new(tokens);
    let words = view.word_refs();
    let Some(was) = words
        .windows(5)
        .position(|part| part == ["was", "tapped", "to", "pay", "its"])
    else {
        return Ok(None);
    };
    if was == 0 || was + 7 >= words.len() || !words.ends_with(&["additional", "cost"]) {
        return Ok(None);
    }
    let label = &words[was + 5..words.len() - 2];
    if label.is_empty() {
        return Ok(None);
    }
    let end = view.token_start_indices()[was];
    let filter = crate::grammar::filters::parse_object_filter_with_grammar_entrypoint_lexed(
        &tokens[..end],
        false,
    )?;
    let tag = crate::tag::declared_key(ironsmith_core::tag::additional_cost_tap_receipt(
        &label.join(" "),
    ));
    crate::word_claims::claim_tokens(&tokens[end..]);
    Ok(Some(
        crate::cards::builders::PredicateAst::TaggedMatchedLastKnown(tag, filter),
    ))
}

pub(crate) fn named_paid_predicate(
    tokens: &[OwnedLexToken],
) -> Option<crate::cards::builders::PredicateAst> {
    let words = crate::lexer::TokenWordView::new(tokens).word_refs();
    let label = words.strip_prefix(&[
        "this",
        "spell",
        "paid",
        "the",
        "additional",
        "cost",
        "named",
    ])?;
    if label.is_empty()
        || label.iter().any(|word| {
            !word
                .chars()
                .all(|c| c.is_alphanumeric() || c == '\'' || c == '-')
        })
    {
        return None;
    }
    let mut reference =
        crate::cost::OptionalCostRef::new(crate::cost::OptionalCostKind::Additional);
    reference.custom_keyword = Some(label.join(" "));
    crate::word_claims::claim_tokens(tokens);
    Some(crate::cards::builders::PredicateAst::ThisSpellPaidLabel(
        reference,
    ))
}

/// A negative qualified paid-tap reference keeps its ordinary candidate
/// filter, excluding exactly the completed contributor incarnations.
pub(crate) fn negative_tap_filter(tokens:&[OwnedLexToken],other:bool)->Option<Result<crate::target::ObjectFilter,CardTextError>> {
    let view=crate::lexer::TokenWordView::new(tokens);let words=view.word_refs();
    let marker=["that","was","not","tapped","to","pay","its"];
    let index=words.windows(marker.len()).position(|part|part==marker)?;
    if index==0 || !words.ends_with(&["additional","cost"]) {return None;}
    let label=&words[index+marker.len()..words.len()-2];
    if label.is_empty() || label.iter().any(|word|!word.chars().all(|ch|ch.is_alphabetic()||matches!(ch,'-'|'\''))) {return None;}
    let boundary=view.token_start_indices()[index];
    let mut filter=match crate::object_filters::parse_object_filter_lexed(&tokens[..boundary],other) {Ok(filter)=>filter,Err(error)=>return Some(Err(error))};
    filter.tagged_constraints.push(crate::target::TaggedObjectConstraint {
        tag:ironsmith_core::tag::additional_cost_tap_receipt(&label.join(" ")),
        relation:crate::target::TaggedOpbjectRelation::IsNotTaggedObject,
    });
    crate::word_claims::claim_tokens(&tokens[boundary..]);Some(Ok(filter))
}

#[cfg(test)]
mod active_negative_donor_filter_tests {
    use super::*;
    #[test]
    fn negative_paid_reference_is_exact_and_keeps_ordinary_qualifiers() {
        let tokens=crate::util::tokenize_line("creature without flying that was not tapped to pay its draw strength additional cost",0);
        let filter=negative_tap_filter(&tokens,false).unwrap().unwrap();
        assert!(filter.excluded_static_abilities.contains(&crate::static_abilities::StaticAbilityId::Flying));
        assert_eq!(filter.tagged_constraints.last().unwrap().relation,crate::target::TaggedOpbjectRelation::IsNotTaggedObject);
        assert_eq!(filter.tagged_constraints.last().unwrap().tag,ironsmith_core::tag::additional_cost_tap_receipt("draw strength"));
    }
}
