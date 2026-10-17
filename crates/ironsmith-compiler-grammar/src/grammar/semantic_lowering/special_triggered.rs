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
    /// Oath of Mages: the upkeep player targets an opponent with more life.
    OpponentLifeMajorityDamage,
    /// Oath of Scholars: the upkeep player targets an opponent with more
    /// cards in hand.
    OpponentHandMajorityRedraw,
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
        alt((
            parse_opponent_combat_attack_pile,
            parse_opponent_life_majority_damage,
            parse_opponent_hand_majority_redraw,
        )),
    ))
    .parse_next(input)
}

/// "At the beginning of each player's upkeep, that player chooses target
/// player who has more <life | cards in hand> than they do and is their
/// opponent." The comparison is required word for word: a different chooser
/// or comparison must not fall into these programs.
fn parse_upkeep_player_targets_opponent_with_more(
    input: &mut LexStream<'_>,
    quantity: &'static [&'static str],
) -> WResult<()> {
    primitives::phrase(&["at", "the", "beginning", "of", "each", "player's", "upkeep"])
        .parse_next(input)?;
    primitives::comma().parse_next(input)?;
    primitives::phrase(&["that", "player", "chooses", "target", "player", "who", "has", "more"])
        .parse_next(input)?;
    primitives::phrase(quantity).parse_next(input)?;
    primitives::phrase(&["than", "they", "do", "and", "is", "their", "opponent"])
        .parse_next(input)?;
    primitives::period().parse_next(input)?;
    Ok(())
}

fn parse_opponent_life_majority_damage(
    input: &mut LexStream<'_>,
) -> WResult<SpecialTriggeredProgram> {
    parse_upkeep_player_targets_opponent_with_more(input, &["life"])?;
    primitives::phrase(&["the", "first", "player", "may", "have", "this"]).parse_next(input)?;
    alt((primitives::kw("enchantment").void(), winnow::combinator::empty)).parse_next(input)?;
    primitives::phrase(&["deal", "1", "damage", "to", "the", "second", "player"])
        .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(SpecialTriggeredProgram::OpponentLifeMajorityDamage)
}

fn parse_opponent_hand_majority_redraw(
    input: &mut LexStream<'_>,
) -> WResult<SpecialTriggeredProgram> {
    parse_upkeep_player_targets_opponent_with_more(input, &["cards", "in", "hand"])?;
    primitives::phrase(&[
        "the", "first", "player", "may", "discard", "their", "hand", "and", "draw", "three",
        "cards",
    ])
    .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(SpecialTriggeredProgram::OpponentHandMajorityRedraw)
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
    // Every participant and comparison is required: skipping words here can
    // silently turn a different chooser, threshold, or destination into this program.
    primitives::phrase(&["at", "the", "beginning", "of", "each", "player's", "upkeep"])
        .parse_next(input)?;
    primitives::comma().parse_next(input)?;
    primitives::phrase(&[
        "that", "player", "chooses", "target", "player", "who", "controls", "more", "lands",
        "than", "they", "do", "and", "is", "their", "opponent",
    ])
    .parse_next(input)?;
    primitives::period().parse_next(input)?;
    primitives::phrase(&[
        "the", "first", "player", "may", "search", "their", "library", "for", "a", "basic", "land",
        "card",
    ])
    .parse_next(input)?;
    primitives::comma().parse_next(input)?;
    primitives::phrase(&["put", "that", "card", "onto", "the", "battlefield"]).parse_next(input)?;
    primitives::comma().parse_next(input)?;
    primitives::phrase(&["then", "shuffle"]).parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(SpecialTriggeredProgram::OpponentLandMajoritySearch)
}

/// Effects of the Oath opponent-majority programs (Oath of Mages / Oath of
/// Scholars): the upkeep player announces a target opponent who has more of
/// the compared quantity (a real target, CR 115.1, rechecked on resolution),
/// then may perform the reward. "The first player" is that upkeep player and
/// "the second player" the announced target.
pub fn oath_opponent_majority_effects(
    program: SpecialTriggeredProgram,
) -> Option<Vec<crate::cards::builders::EffectAst>> {
    use crate::cards::builders::{
        DamageActionAst, EffectAst, LifeResourceActionAst, PermissionEffectAst, PlayerAst,
        SubjectVerbActionAst, SubjectVerbRoleAst, TargetAst,
    };
    use crate::target::PlayerFilter;
    let (quantity, reward) = match program {
        SpecialTriggeredProgram::OpponentLifeMajorityDamage => (
            ironsmith_core::PlayerQuantity::Life,
            vec![EffectAst::subject_verb(
                SubjectVerbRoleAst::Actor,
                PlayerAst::Implicit,
                SubjectVerbActionAst::Damage(DamageActionAst::DealDamage {
                    amount: crate::effect::Value::Fixed(1),
                    target: TargetAst::Player(
                        PlayerFilter::Target(Box::new(PlayerFilter::Any)),
                        None,
                    ),
                    unpreventable: false,
                }),
            )],
        ),
        SpecialTriggeredProgram::OpponentHandMajorityRedraw => (
            ironsmith_core::PlayerQuantity::CardsInHand,
            vec![
                EffectAst::subject_verb_discard_hand(PlayerAst::Active),
                EffectAst::subject_verb(
                    SubjectVerbRoleAst::Actor,
                    PlayerAst::Active,
                    SubjectVerbActionAst::LifeResources(LifeResourceActionAst::Draw {
                        count: crate::effect::Value::Fixed(3),
                    }),
                ),
            ],
        ),
        _ => return None,
    };
    Some(vec![
        EffectAst::subject_verb_explicit_target_only_for_chooser(
            TargetAst::Player(
                PlayerFilter::QuantityGreaterThanPlayer {
                    base: Box::new(PlayerFilter::OpponentOf(Box::new(PlayerFilter::Active))),
                    other: Box::new(PlayerFilter::Active),
                    quantity,
                },
                Some(crate::TextSpan::synthetic()),
            ),
            PlayerAst::Active,
        ),
        EffectAst::Permissions(PermissionEffectAst::MayByPlayer {
            player: PlayerAst::Active,
            effects: reward,
        }),
    ])
}

#[cfg(test)]
#[path = "special_triggered_programs/tests.rs"]
mod tests;

#[path = "special_triggered_programs/combat.rs"]
mod combat_programs;
use combat_programs::parse_opponent_combat_attack_pile;
#[path = "special_triggered_programs/library.rs"]
mod library_programs;
use library_programs::parse_random_discard_creature_return;
