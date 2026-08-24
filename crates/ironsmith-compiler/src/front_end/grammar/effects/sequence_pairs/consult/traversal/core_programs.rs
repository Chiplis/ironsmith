use super::*;

pub fn parse_consult_traversal_shape(tokens: &[OwnedLexToken]) -> Option<ConsultTraversalShape> {
    let (prefix, consult) = split_prefix_and_consult(tokens)?;
    let (verb, mode) = consult_verb(consult)?;
    if crate::grammar::effects::for_each_shapes::parse_for_each_object_effect_shape(consult)
        .is_some()
    {
        // The iteration header owns this sentence.  Its payload is parsed as a
        // consult only after the outer typed for-each shape has bound `it`.
        return None;
    }
    let player = if verb.start == 0 {
        ConsultTraversalPlayerShape::ImpliedByPrefixOrYou
    } else if permission_shapes::exact_tokens(&consult[..verb.start], &["they"]) {
        ConsultTraversalPlayerShape::ThatPlayer
    } else {
        ConsultTraversalPlayerShape::Subject(trim_commas(&consult[..verb.start]).to_vec())
    };

    let until = find_phrase_span(consult, &[&["until"]])?;
    if until.start <= verb.end {
        return None;
    }
    let library_head = &consult[verb.end..until.start];
    if !starts_content_sequence(library_head, &[&["cards", "from", "top", "of"]])
        || !ends_content_sequence(library_head, &[&["library"]])
    {
        return None;
    }

    let mut stop_tokens = trim_commas(&consult[until.end..]);
    let (where_x, mut trailing_effect) =
        if let Some(comma) = first_consult_trailing_comma(stop_tokens) {
            let trailing = trim_commas(&stop_tokens[comma + 1..]);
            stop_tokens = trim_commas(&stop_tokens[..comma]);
            parse_consult_trailing(trailing)
        } else {
            (None, Vec::new())
        };
    let stop = parse_matching_filter_or_exposed_count_stop(stop_tokens, mode)
        .or_else(|| parse_passive_stop(stop_tokens, mode))
        .or_else(|| parse_active_stop(stop_tokens))?;
    if stop.max_exposed.is_some()
        && permission_shapes::exact_tokens(&trailing_effect, &["whichever", "comes", "first"])
    {
        trailing_effect.clear();
    }
    Some(ConsultTraversalShape {
        prefix,
        player,
        mode,
        stop,
        where_x,
        trailing_effect,
    })
}
