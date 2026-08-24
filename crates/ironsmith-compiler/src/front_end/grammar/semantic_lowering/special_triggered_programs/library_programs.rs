use super::*;

pub(super) fn parse_random_discard_creature_return(
    input: &mut LexStream<'_>,
) -> WResult<SpecialTriggeredProgram> {
    seek_phrase(input, &["at", "the", "beginning", "of", "your", "upkeep"])?;
    seek_phrase(input, &["discard", "a", "card", "at", "random"])?;
    seek_phrase(
        input,
        &[
            "if", "you", "discard", "a", "creature", "card", "this", "way",
        ],
    )?;
    seek_phrase(
        input,
        &[
            "return",
            "it",
            "from",
            "your",
            "graveyard",
            "to",
            "the",
            "battlefield",
        ],
    )?;
    seek_phrase(input, &["unless", "any", "player", "pays"])?;
    let life = leaf::parse_leaf_number_prefix_lexed.parse_next(input)?;
    primitives::kw("life").parse_next(input)?;
    Ok(SpecialTriggeredProgram::RandomDiscardCreatureReturnUnlessLife { life })
}
