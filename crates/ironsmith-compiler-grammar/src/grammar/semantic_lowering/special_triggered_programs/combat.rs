use super::*;

pub(super) fn parse_opponent_combat_attack_pile(
    input: &mut LexStream<'_>,
) -> WResult<SpecialTriggeredProgram> {
    seek_phrase(
        input,
        &["at", "the", "beginning", "of", "combat", "on", "each"],
    )?;
    seek_phrase(input, &["turn"])?;
    seek_phrase(
        input,
        &[
            "separate",
            "all",
            "creatures",
            "that",
            "player",
            "controls",
            "into",
            "two",
            "piles",
        ],
    )?;
    seek_phrase(
        input,
        &[
            "only",
            "creatures",
            "in",
            "the",
            "pile",
            "of",
            "their",
            "choice",
            "can",
            "attack",
            "this",
            "turn",
        ],
    )?;
    Ok(SpecialTriggeredProgram::OpponentCombatAttackPile)
}
