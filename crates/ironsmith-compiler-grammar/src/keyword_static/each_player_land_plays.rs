use super::*;

/// "Each player may play an additional land on each of their turns." (Rites
/// of Flourishing, Ghirapur Orrery, Storm Cauldron): CR 305.2 lets an effect
/// raise every player's land-play allowance; the allowance only matters on
/// that player's own turn, so the restriction scopes to every player.
pub fn parse_each_player_additional_land_play_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(count) = late_static_facts::parse_each_player_additional_land_play_count(tokens)
    else {
        return Ok(None);
    };
    let display = if count == 1 {
        "Each player may play an additional land on each of their turns".to_string()
    } else {
        format!("Each player may play {count} additional lands on each of their turns")
    };
    Ok(Some(StaticAbility::restriction(
        crate::effect::Restriction::additional_land_plays(PlayerFilter::Any, count),
        display,
    )))
}
