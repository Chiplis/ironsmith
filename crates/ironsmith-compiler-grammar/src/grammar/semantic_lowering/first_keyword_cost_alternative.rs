//! "You may pay {0} rather than pay the equip cost of the first equip ability
//! you activate each turn." (Bruenor Battlehammer, Forge Anew, Kíli) and its
//! cycling ("the cycling cost of the first card you cycle each turn", Gavi)
//! and power-up ("the power-up cost of the first power-up ability you
//! activate during each of your turns", Advancing the Spirit) siblings: an
//! alternative mana payment for the first activation of one keyword ability
//! kind each turn (CR 118.9, 602.2b).

use winnow::combinator::{alt, peek, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use super::super::primitives;
use crate::lexer::{LexStream, OwnedLexToken};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FirstKeywordCostAlternative<'a> {
    /// The replacement price between "pay" and "rather than".
    pub replacement_cost_tokens: &'a [OwnedLexToken],
    pub keyword: ironsmith_core::ActivatedAbilityKeyword,
    /// "during each of your turns" rather than "each turn".
    pub your_turns_only: bool,
}

fn power_up_word(input: &mut LexStream<'_>) -> WResult<()> {
    alt((
        primitives::kw("power-up").void(),
        primitives::phrase(&["power", "up"]),
    ))
    .parse_next(input)
}

fn equip_or_power_up(input: &mut LexStream<'_>) -> WResult<ironsmith_core::ActivatedAbilityKeyword> {
    alt((
        primitives::kw("equip").value(ironsmith_core::ActivatedAbilityKeyword::Equip),
        power_up_word.value(ironsmith_core::ActivatedAbilityKeyword::PowerUp),
    ))
    .parse_next(input)
}

/// "<kw> cost of the first <kw> ability you activate" for equip / power-up.
fn first_named_ability(input: &mut LexStream<'_>) -> WResult<ironsmith_core::ActivatedAbilityKeyword> {
    let keyword = equip_or_power_up.parse_next(input)?;
    primitives::phrase(&["cost", "of", "the", "first"]).parse_next(input)?;
    let repeated = equip_or_power_up.parse_next(input)?;
    if repeated != keyword {
        return Err(primitives::backtrack_err(
            "first keyword cost alternative",
            "the same ability keyword",
        ));
    }
    primitives::phrase(&["ability", "you", "activate"]).parse_next(input)?;
    Ok(keyword)
}

/// "cycling cost of the first card you cycle".
fn first_cycled_card(input: &mut LexStream<'_>) -> WResult<ironsmith_core::ActivatedAbilityKeyword> {
    primitives::phrase(&[
        "cycling", "cost", "of", "the", "first", "card", "you", "cycle",
    ])
    .parse_next(input)?;
    Ok(ironsmith_core::ActivatedAbilityKeyword::Cycling)
}

fn first_keyword_cost_alternative<'a>(
    input: &mut LexStream<'a>,
) -> WResult<FirstKeywordCostAlternative<'a>> {
    primitives::phrase(&["you", "may", "pay"]).parse_next(input)?;
    let replacement_cost_tokens = repeat_till::<_, _, (), _, _, _, _>(
        1..,
        any.void(),
        peek(primitives::phrase(&["rather", "than", "pay", "the"])),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    primitives::phrase(&["rather", "than", "pay", "the"]).parse_next(input)?;
    let keyword = alt((first_named_ability, first_cycled_card)).parse_next(input)?;
    let your_turns_only = alt((
        primitives::phrase(&["each", "turn"]).value(false),
        primitives::phrase(&["during", "each", "of", "your", "turns"]).value(true),
    ))
    .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(FirstKeywordCostAlternative {
        replacement_cost_tokens,
        keyword,
        your_turns_only,
    })
}

/// Reads a complete first-keyword-activation cost alternative line.
pub fn parse_first_keyword_cost_alternative_tokens(
    tokens: &[OwnedLexToken],
) -> Option<FirstKeywordCostAlternative<'_>> {
    first_keyword_cost_alternative
        .parse(LexStream::new(tokens))
        .ok()
}
