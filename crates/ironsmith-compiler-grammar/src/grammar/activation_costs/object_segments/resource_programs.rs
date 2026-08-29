use super::*;

pub(super) fn parse_unattach_cost_shape_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<UnattachCostShape<'a>> {
    primitives::kw("unattach").parse_next(input)?;

    let mut chosen_input = input.clone();
    if let Ok(chosen) = parse_unattach_chosen_tail_lexed(&mut chosen_input) {
        *input = chosen_input;
        return Ok(UnattachCostShape::Chosen(chosen));
    }

    let reference_tokens = rest.parse_next(input)?;
    if reference_tokens.is_empty() {
        return Err(primitives::backtrack_err(
            "unattach cost",
            "source reference or object filter",
        ));
    }
    Ok(UnattachCostShape::Source { reference_tokens })
}
