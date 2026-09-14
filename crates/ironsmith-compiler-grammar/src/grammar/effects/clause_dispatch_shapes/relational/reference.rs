use super::*;

pub fn parse_hexproof_targeting_override_shape(
    tokens: &[OwnedLexToken],
) -> Option<HexproofTargetingOverrideShape<'_>> {
    primitives::find_prefix(tokens, || {
        primitives::any_phrase(&[
            &["as", "though", "they", "didnt", "have", "hexproof"],
            &["as", "though", "they", "didn't", "have", "hexproof"],
        ])
    })?;
    let (creatures, _, after_creatures) =
        primitives::find_prefix(tokens, || primitives::kw("creatures"))?;
    let (can_relative, _, _) = primitives::find_prefix(after_creatures, || {
        primitives::phrase(&["can", "be", "the", "targets"])
    })?;
    let can = creatures + 1 + can_relative;
    let filter_tokens = trim_lexed_commas(tokens.get(creatures..can)?);
    (!filter_tokens.is_empty()).then_some(HexproofTargetingOverrideShape { filter_tokens })
}

pub fn parse_control_player_shape(tokens: &[OwnedLexToken]) -> Option<ControlPlayerShape<'_>> {
    let (control, _, after_control) = primitives::find_prefix(tokens, || {
        alt((primitives::kw("control"), primitives::kw("controls")))
    })?;
    if control == 0 {
        return None;
    }
    let subject_tokens = tokens.get(..control)?;
    let (_, player, _) = primitives::find_prefix(subject_tokens, || {
        alt((
            primitives::kw("you").value(PlayerAst::You),
            primitives::phrase(&["that", "player"]).value(PlayerAst::That),
            primitives::phrase(&["target", "player"]).value(PlayerAst::Target),
            primitives::phrase(&["each", "opponent"]).value(PlayerAst::Opponent),
        ))
    })?;
    let (during, _, _) = primitives::find_prefix(after_control, || primitives::kw("during"))?;
    if during == 0 {
        return None;
    }
    let mut target_tokens = trim_lexed_commas(after_control.get(..during)?);
    // "you gain control of target opponent during ..." keeps the "of" of the
    // gain-control idiom in front of the target phrase.
    if let Some((first, rest)) = target_tokens.split_first()
        && first.is_word("of")
    {
        target_tokens = trim_lexed_commas(rest);
    }
    let duration_start = control + 1 + during;
    Some(ControlPlayerShape {
        player,
        target_tokens,
        duration_tokens: tokens.get(duration_start..)?,
    })
}
