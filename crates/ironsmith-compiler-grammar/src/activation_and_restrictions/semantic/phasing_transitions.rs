use super::*;
pub(super) fn parse_phasing_transition_trigger(
    tokens: &[OwnedLexToken],
) -> Result<Option<TriggerSpec>, CardTextError> {
    let view = ActivationRestrictionCompatWords::new(tokens);
    let words = view.to_word_refs();
    let [subject @ .., "phase" | "phases", direction @ ("in" | "out")] = words.as_slice() else {
        return Ok(None);
    };
    let one_or_more = subject.starts_with(&["one", "or", "more"]);
    let start = if one_or_more {
        trigger_word_token_start(tokens, 3).unwrap_or(tokens.len())
    } else {
        0
    };
    let end = trigger_word_token_start(tokens, subject.len()).unwrap_or(tokens.len());
    if start >= end {
        return Ok(None);
    }
    let subject = &tokens[start..end];
    let filter = match parse_trigger_subject_filter_lexed(subject)? {
        Some(filter) => filter,
        None => source_reference_surface_for_trigger_subject(subject)
            .map(ObjectFilter::source_with_surface)
            .ok_or_else(|| {
                CardTextError::ParseError("phasing transition requires an object subject".into())
            })?,
    };
    Ok(Some(TriggerSpec::PhasingChanged {
        filter,
        phased_in: *direction == "in",
        one_or_more,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn phasing_preserves_source_or_other_filters_direction_and_grouping() {
        let parse = |text| {
            parse_phasing_transition_trigger(&crate::lexer::lex_line(text, 0).unwrap())
                .unwrap()
                .unwrap()
        };
        let TriggerSpec::PhasingChanged {
            filter,
            phased_in,
            one_or_more,
        } = parse("one or more other permanents phase out")
        else {
            panic!("phasing")
        };
        assert!(filter.other && !phased_in && one_or_more);
        let TriggerSpec::PhasingChanged {
            filter,
            phased_in,
            one_or_more,
        } = parse("this creature or another Spirit you control phases in")
        else {
            panic!("phasing union")
        };
        assert!(phased_in && !one_or_more);
        assert_eq!(filter.any_of.len(), 2);
        assert!(filter.any_of.iter().any(|filter| filter.source));
        assert!(
            filter
                .any_of
                .iter()
                .any(|filter| filter.other && filter.controller == Some(PlayerFilter::You))
        );
        assert!(
            parse_phasing_transition_trigger(
                &crate::lexer::lex_line("one or more phase out", 0).unwrap()
            )
            .unwrap()
            .is_none()
        );
    }
}
