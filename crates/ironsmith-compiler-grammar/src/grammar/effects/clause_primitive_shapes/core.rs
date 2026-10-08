use super::*;

pub fn parse_clash_shape(tokens: &[OwnedLexToken]) -> Option<ClashOpponentAst> {
    let tokens = trim_shape_edges(tokens);
    let (_, tail) = primitives::parse_prefix(
        tokens,
        (
            alt((primitives::kw("clash"), primitives::kw("clashes"))),
            opt(primitives::kw("with")),
        ),
    )?;
    let target_tokens = primitives::split_lexed_once_on_separator(tail, || {
        alt((primitives::kw("then").void(), primitives::comma().void()))
    })
    .map(|(head, _)| head)
    .unwrap_or(tail);
    crate::grammar::primitives::probe_all(
        trim_shape_edges(target_tokens),
        (clash_opponent, eof).map(|(opponent, _)| opponent),
        "clash opponent",
    )
}

pub(super) fn parse_repeat_process<'a>(
    input: &mut crate::lexer::LexStream<'a>,
) -> WResult<(bool, RepeatProcessShape)> {
    opt(primitives::kw("and")).parse_next(input)?;
    let explicit_may = opt(primitives::phrase(&["you", "may"]))
        .parse_next(input)?
        .is_some();
    primitives::phrase(&["repeat", "this", "process"]).parse_next(input)?;
    let shape = alt((
        primitives::phrase(&["any", "number", "of", "times"]).value(RepeatProcessShape::May),
        primitives::phrase(&["as", "many", "times", "as", "they", "choose"]).value(RepeatProcessShape::May),
        primitives::phrase(&["as", "many", "times", "as", "you", "choose"]).value(RepeatProcessShape::May),
        primitives::kw("once").value(RepeatProcessShape::Once),
        eof.value(RepeatProcessShape::Required),
    ))
    .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok((explicit_may, shape))
}

pub fn parse_repeat_process_shape(tokens: &[OwnedLexToken]) -> Option<RepeatProcessShape> {
    let tokens = trim_shape_edges(tokens);
    if let Some((_, tail)) = primitives::parse_prefix(
        tokens,
        (opt(primitives::kw("and")), primitives::phrase(&["repeat", "this", "process"])),
    )
        && let Some((count, used)) = crate::util::parse_value_expr(tail)
        && matches!(count, Value::Fixed(0..) | Value::X)
        && exact_phrase(&tail[used..], &["more", "times"])
    {
        return Some(RepeatProcessShape::Additional(count));
    }
    let (explicit_may, shape) = crate::grammar::primitives::probe_all(
        trim_shape_edges(tokens),
        parse_repeat_process,
        "repeat process clause",
    )?;
    match (explicit_may, shape) {
        (true, RepeatProcessShape::Required | RepeatProcessShape::May) => {
            Some(RepeatProcessShape::May)
        }
        (false, shape) => Some(shape),
        (true, RepeatProcessShape::Once | RepeatProcessShape::Additional(_)) => None,
    }
}
