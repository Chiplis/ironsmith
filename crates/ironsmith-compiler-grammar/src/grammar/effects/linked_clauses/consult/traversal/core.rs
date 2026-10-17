use super::*;

pub fn parse_consult_traversal_shape(tokens: &[OwnedLexToken]) -> Option<ConsultTraversalShape> {
    if let Some(shape) = parse_mill_repeat_traversal_shape(tokens) {
        return Some(shape);
    }
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

/// "<player> mills a card, then repeats this process until a creature card or
/// X cards have been put into their graveyard this way, whichever comes
/// first" (Helm of Obedience): a mill-mode consult traversal whose exposed
/// cards are the ones each one-card mill put into a graveyard (CR 701.13a).
fn parse_mill_repeat_traversal_shape(tokens: &[OwnedLexToken]) -> Option<ConsultTraversalShape> {
    let tokens = trim_commas(tokens);
    let mill = find_phrase_span(tokens, &[&["mills", "a", "card"], &["mill", "a", "card"]])?;
    let repeat = find_phrase_span(
        tokens,
        &[
            &["then", "repeats", "this", "process", "until"],
            &["then", "repeat", "this", "process", "until"],
        ],
    )?;
    if repeat.start < mill.end || !trim_commas(&tokens[mill.end..repeat.start]).is_empty() {
        return None;
    }
    let player = if mill.start == 0 {
        ConsultTraversalPlayerShape::ImpliedByPrefixOrYou
    } else if permission_shapes::exact_tokens(&tokens[..mill.start], &["they"]) {
        ConsultTraversalPlayerShape::ThatPlayer
    } else {
        ConsultTraversalPlayerShape::Subject(trim_commas(&tokens[..mill.start]).to_vec())
    };
    let mut stop_tokens = trim_commas(&tokens[repeat.end..]);
    let mut trailing: &[OwnedLexToken] = &[];
    if let Some(comma) = first_consult_trailing_comma(stop_tokens) {
        trailing = trim_commas(&stop_tokens[comma + 1..]);
        stop_tokens = trim_commas(&stop_tokens[..comma]);
    }
    let stop = parse_matching_filter_or_exposed_count_stop(stop_tokens, LibraryConsultModeAst::Mill)?;
    if !permission_shapes::exact_tokens(trailing, &["whichever", "comes", "first"]) {
        return None;
    }
    Some(ConsultTraversalShape {
        prefix: None,
        player,
        mode: LibraryConsultModeAst::Mill,
        stop,
        where_x: None,
        trailing_effect: Vec::new(),
    })
}
