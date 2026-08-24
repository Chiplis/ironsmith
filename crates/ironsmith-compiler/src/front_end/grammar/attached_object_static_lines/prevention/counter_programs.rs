use super::*;

pub(super) fn parse_counter_destination<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        semantic_kw("it"),
        semantic_kw("him"),
        (semantic_kw("this"), semantic_kw("creature")).void(),
    ))
    .parse_next(input)
}
