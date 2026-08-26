use winnow::combinator::{alt, peek, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use super::super::super::lexer::{LexStream, OwnedLexToken};
use super::super::{leaf, primitives};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecialTriggeredProgram {
    PreviousTurnCreatureEntryDraw,
    SecondSpellSuspend,
    DifferentNamesLibraryDivvy,
    OpponentCreatureMajorityConsult,
    PrimeControlledLandCountToken,
    OpponentLandMajoritySearch,
    OpponentGraveyardMinorityReturn,
    RandomDiscardCreatureReturnUnlessLife { life: u32 },
    OpponentCombatAttackPile,
}

pub fn parse_special_triggered_program_tokens(
    tokens: &[OwnedLexToken],
) -> Option<SpecialTriggeredProgram> {
    primitives::parse_prefix(tokens, parse_special_triggered_program).map(|(program, _)| program)
}

fn parse_special_triggered_program(input: &mut LexStream<'_>) -> WResult<SpecialTriggeredProgram> {
    alt((
        parse_previous_turn_creature_entry_draw,
        parse_second_spell_suspend,
        parse_different_names_library_divvy,
        parse_opponent_creature_majority_consult,
        parse_prime_controlled_land_count_token,
        parse_opponent_land_majority_search,
        parse_opponent_graveyard_minority_return,
        parse_random_discard_creature_return,
        parse_opponent_combat_attack_pile,
    ))
    .parse_next(input)
}

fn seek_phrase(input: &mut LexStream<'_>, expected: &'static [&'static str]) -> WResult<()> {
    repeat_till::<_, _, (), _, _, _, _>(0.., any.void(), peek(primitives::phrase(expected)))
        .void()
        .parse_next(input)?;
    primitives::phrase(expected).parse_next(input)
}

fn parse_previous_turn_creature_entry_draw(
    input: &mut LexStream<'_>,
) -> WResult<SpecialTriggeredProgram> {
    seek_phrase(input, &["at", "the", "beginning", "of", "each", "upkeep"])?;
    seek_phrase(
        input,
        &["another", "creature", "entered", "the", "battlefield"],
    )?;
    seek_phrase(input, &["under", "your", "control", "last", "turn"])?;
    seek_phrase(input, &["draw", "a", "card"])?;
    Ok(SpecialTriggeredProgram::PreviousTurnCreatureEntryDraw)
}

fn parse_second_spell_suspend(input: &mut LexStream<'_>) -> WResult<SpecialTriggeredProgram> {
    seek_phrase(
        input,
        &["you", "cast", "your", "second", "spell", "each", "turn"],
    )?;
    seek_phrase(input, &["copy", "it"])?;
    seek_phrase(input, &["then", "exile", "the", "spell", "you", "cast"])?;
    seek_phrase(input, &["time", "counters", "on", "it"])?;
    seek_phrase(input, &["if", "it", "doesn't", "have", "suspend"])?;
    seek_phrase(input, &["it", "gains", "suspend"])?;
    Ok(SpecialTriggeredProgram::SecondSpellSuspend)
}

fn parse_different_names_library_divvy(
    input: &mut LexStream<'_>,
) -> WResult<SpecialTriggeredProgram> {
    seek_phrase(
        input,
        &[
            "search", "your", "library", "for", "exactly", "two", "cards", "not", "named",
        ],
    )?;
    seek_phrase(input, &["that", "have", "different", "names"])?;
    seek_phrase(input, &["an", "opponent", "chooses", "one", "of", "them"])?;
    seek_phrase(
        input,
        &["put", "the", "chosen", "card", "into", "your", "hand"],
    )?;
    seek_phrase(input, &["the", "other", "into", "your", "graveyard"])?;
    Ok(SpecialTriggeredProgram::DifferentNamesLibraryDivvy)
}

fn parse_opponent_creature_majority_consult(
    input: &mut LexStream<'_>,
) -> WResult<SpecialTriggeredProgram> {
    seek_phrase(input, &["at", "the", "beginning", "of", "each"])?;
    seek_phrase(input, &["upkeep"])?;
    seek_phrase(
        input,
        &[
            "chooses",
            "target",
            "player",
            "who",
            "controls",
            "more",
            "creatures",
        ],
    )?;
    seek_phrase(
        input,
        &[
            "reveal", "cards", "from", "the", "top", "of", "their", "library",
        ],
    )?;
    seek_phrase(input, &["until", "they", "reveal", "a", "creature", "card"])?;
    seek_phrase(
        input,
        &["puts", "that", "card", "onto", "the", "battlefield"],
    )?;
    Ok(SpecialTriggeredProgram::OpponentCreatureMajorityConsult)
}

fn parse_prime_controlled_land_count_token(
    input: &mut LexStream<'_>,
) -> WResult<SpecialTriggeredProgram> {
    seek_phrase(
        input,
        &["at", "the", "beginning", "of", "your", "end", "step"],
    )?;
    seek_phrase(
        input,
        &[
            "a",
            "land",
            "entered",
            "the",
            "battlefield",
            "under",
            "your",
            "control",
            "this",
            "turn",
        ],
    )?;
    seek_phrase(
        input,
        &["you", "control", "a", "prime", "number", "of", "lands"],
    )?;
    seek_phrase(input, &["create"])?;
    seek_phrase(
        input,
        &[
            "then", "put", "that", "many", "+1/+1", "counters", "on", "it",
        ],
    )?;
    Ok(SpecialTriggeredProgram::PrimeControlledLandCountToken)
}

fn parse_opponent_graveyard_minority_return(
    input: &mut LexStream<'_>,
) -> WResult<SpecialTriggeredProgram> {
    seek_phrase(input, &["at", "the", "beginning", "of", "each"])?;
    seek_phrase(input, &["upkeep"])?;
    seek_phrase(
        input,
        &[
            "chooses",
            "target",
            "player",
            "whose",
            "graveyard",
            "has",
            "fewer",
            "creature",
            "cards",
        ],
    )?;
    seek_phrase(
        input,
        &[
            "return",
            "a",
            "creature",
            "card",
            "from",
            "their",
            "graveyard",
            "to",
            "their",
            "hand",
        ],
    )?;
    Ok(SpecialTriggeredProgram::OpponentGraveyardMinorityReturn)
}

fn parse_opponent_land_majority_search(
    input: &mut LexStream<'_>,
) -> WResult<SpecialTriggeredProgram> {
    seek_phrase(input, &["at", "the", "beginning", "of", "each"])?;
    seek_phrase(input, &["upkeep"])?;
    seek_phrase(
        input,
        &[
            "chooses", "target", "player", "who", "controls", "more", "lands",
        ],
    )?;
    seek_phrase(
        input,
        &[
            "may", "search", "their", "library", "for", "a", "basic", "land", "card",
        ],
    )?;
    seek_phrase(
        input,
        &["put", "that", "card", "onto", "the", "battlefield"],
    )?;
    seek_phrase(input, &["then", "shuffle"])?;
    Ok(SpecialTriggeredProgram::OpponentLandMajoritySearch)
}

#[cfg(test)]
#[path = "special_triggered_programs/tests.rs"]
mod tests;

#[path = "special_triggered_programs/combat_programs.rs"]
mod combat_programs;
use combat_programs::parse_opponent_combat_attack_pile;
#[path = "special_triggered_programs/library_programs.rs"]
mod library_programs;
use library_programs::parse_random_discard_creature_return;
