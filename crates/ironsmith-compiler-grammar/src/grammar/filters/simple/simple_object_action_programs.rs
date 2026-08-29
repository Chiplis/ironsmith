use super::*;

pub(super) fn parse_controller_owner_suffix(
    input: &mut WordInput<'_>,
) -> WResult<SimpleObjectFilterSuffix> {
    let controller = parse_controller_player.parse_next(input)?;
    parse_control_action.parse_next(input)?;
    primitives::word_slice_exact("but")
        .void()
        .parse_next(input)?;
    parse_control_negation.parse_next(input)?;
    parse_own_action.parse_next(input)?;
    if controller != PlayerFilter::You {
        return Err(primitives::backtrack_err(
            "simple object filter suffix",
            "you control but do not own",
        ));
    }
    Ok(SimpleObjectFilterSuffix::ControllerOwner(
        controller,
        PlayerFilter::NotYou,
    ))
}

pub(super) fn parse_negated_controller_suffix(
    input: &mut WordInput<'_>,
) -> WResult<SimpleObjectFilterSuffix> {
    primitives::word_slice_exact("you")
        .void()
        .parse_next(input)?;
    parse_control_negation.parse_next(input)?;
    parse_control_action.parse_next(input)?;
    Ok(SimpleObjectFilterSuffix::Controller(PlayerFilter::NotYou))
}

pub(super) fn parse_controller_suffix(
    input: &mut WordInput<'_>,
) -> WResult<SimpleObjectFilterSuffix> {
    let controller = parse_controller_player.parse_next(input)?;
    parse_control_action.parse_next(input)?;
    Ok(SimpleObjectFilterSuffix::Controller(controller))
}

pub(super) fn parse_control_action(input: &mut WordInput<'_>) -> WResult<()> {
    alt((
        primitives::word_slice_exact("control"),
        primitives::word_slice_exact("controls"),
    ))
    .void()
    .parse_next(input)
}

pub(super) fn parse_control_negation(input: &mut WordInput<'_>) -> WResult<()> {
    alt((
        word_phrase(&["do", "not"]),
        primitives::word_slice_exact("dont").void(),
        primitives::word_slice_exact("don't").void(),
    ))
    .parse_next(input)
}
