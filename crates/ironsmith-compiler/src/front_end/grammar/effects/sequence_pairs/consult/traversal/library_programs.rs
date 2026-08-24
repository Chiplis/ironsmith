use super::*;

pub(super) fn parse_matching_filter_or_exposed_count_stop(
    tokens: &[OwnedLexToken],
    mode: LibraryConsultModeAst,
) -> Option<ConsultTraversalStopShape> {
    let (match_tokens, count_tokens) =
        primitives::split_lexed_once_on_separator(tokens, || primitives::kw("or").void())?;
    let match_tokens = trim_commas(match_tokens);
    if match_tokens.is_empty() {
        return None;
    }
    let count_stop = parse_passive_stop(count_tokens, mode)?;
    if !count_stop.filter.is_empty() || count_stop.max_exposed.is_some() {
        return None;
    }
    let LibraryConsultStopRuleAst::MatchCount(max_exposed) = count_stop.stop_rule else {
        return None;
    };
    Some(ConsultTraversalStopShape {
        stop_rule: LibraryConsultStopRuleAst::FirstMatch,
        max_exposed: Some(max_exposed),
        filter: match_tokens.to_vec(),
        kind: ConsultTraversalStopKind::Passive,
    })
}

pub(super) fn parse_active_stop(tokens: &[OwnedLexToken]) -> Option<ConsultTraversalStopShape> {
    let tokens = trim_commas(tokens);
    let verb = find_phrase_span(tokens, CONSULT_VERBS)?;
    if verb.start == 0 {
        return None;
    }
    let filter = trim_commas(&tokens[verb.end..]);
    if filter.is_empty() {
        return None;
    }
    if let Some(stop) = parse_equal_to_counted_active_stop(filter) {
        return Some(stop);
    }
    let (stop_rule, filter) = counted_stop_prefix(filter)
        .filter(|(_, filter)| !filter.is_empty())
        .unwrap_or((LibraryConsultStopRuleAst::FirstMatch, filter));
    Some(ConsultTraversalStopShape {
        stop_rule,
        max_exposed: None,
        filter: filter.to_vec(),
        kind: ConsultTraversalStopKind::Active,
    })
}
