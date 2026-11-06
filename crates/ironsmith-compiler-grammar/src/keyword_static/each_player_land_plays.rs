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
        return parse_enchanted_player_additional_land_play_line(tokens);
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

/// "Enchanted player may play an additional land on each of their turns."
/// (Fabled Aura back faces): the same allowance scoped to the player this
/// Aura is attached to.
fn parse_enchanted_player_additional_land_play_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let tokens = crate::util::trim_edge_punctuation_tokens(tokens);
    let words = crate::lexer::parser_token_word_refs(tokens);
    let [
        "enchanted",
        "player",
        "may",
        "play",
        amount,
        "additional",
        "land" | "lands",
        "on" | "during",
        "each",
        "of",
        "their",
        "turns",
    ] = words.as_slice()
    else {
        return Ok(None);
    };
    let count = match *amount {
        "a" | "an" => 1,
        other => match crate::util::parse_number_word_u32(other) {
            Some(count) if count > 0 => count,
            _ => return Ok(None),
        },
    };
    crate::word_claims::claim_tokens(tokens);
    let display = if count == 1 {
        "Enchanted player may play an additional land on each of their turns".to_string()
    } else {
        format!("Enchanted player may play {count} additional lands on each of their turns")
    };
    Ok(Some(StaticAbility::restriction(
        crate::effect::Restriction::additional_land_plays(
            PlayerFilter::TaggedPlayer("enchanted".into()),
            count,
        ),
        display,
    )))
}
