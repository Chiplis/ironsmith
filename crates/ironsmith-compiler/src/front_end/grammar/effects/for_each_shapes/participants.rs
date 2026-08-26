use winnow::combinator::{alt, opt};
use winnow::prelude::*;
use winnow::token::any;

use crate::cards::builders::CardTextError;
use crate::grammar::effects::chain_splitting;
use crate::grammar::primitives;
use crate::lexer::{LexStream, OwnedLexToken};
use crate::util::{
    parse_greater_than_or_equal_quantity_prefix, parse_quantity_comparison_prefix,
    trim_edge_punctuation_tokens,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForEachParticipantScope {
    Opponent,
    OpponentExceptDefending,
    Player,
    PlayerExceptYou,
    PlayerExceptTarget,
    PlayerExceptItsController,
    PlayerOnYourTeam,
}

#[derive(Debug, Clone, Copy)]
pub struct ForEachParticipantClauseShape<'a> {
    pub scope: ForEachParticipantScope,
    /// `Each player/opponent <verbs>` names the iterated participant as the
    /// actor. `For each player/opponent, <imperative>` keeps the ability's
    /// controller as the implicit actor.
    pub participant_is_actor: bool,
    pub inner_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy)]
pub struct RelativeControlClauseShape<'a> {
    pub controls_most: bool,
    pub count_comparison: Option<crate::effect::Comparison>,
    pub fewer_than_most_filter_tokens: Option<&'a [OwnedLexToken]>,
    pub fewer_than_you: bool,
    pub filter_tokens: &'a [OwnedLexToken],
    pub effect_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy)]
pub struct SourceAttackedPlayerClauseShape<'a> {
    pub effect_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy)]
pub struct CombatDamageHistoryPlayerClauseShape<'a> {
    pub source_tokens: &'a [OwnedLexToken],
    pub effect_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy)]
pub enum WhoClauseShape<'a> {
    TappedLandForMana {
        effect_tokens: &'a [OwnedLexToken],
    },
    Negated {
        effect_tokens: &'a [OwnedLexToken],
        tagged_filter_tokens: Option<&'a [OwnedLexToken]>,
        implicit_player_is_iterated: bool,
    },
    DidThisWay {
        effect_tokens: &'a [OwnedLexToken],
        tagged_filter_tokens: Option<&'a [OwnedLexToken]>,
    },
    DidAction {
        effect_tokens: &'a [OwnedLexToken],
        implicit_player_is_you: bool,
    },
}

#[derive(Debug, Clone, Copy)]
pub enum OpponentSpecialShape<'a> {
    IgnoreScryOrSurveil,
    ChooseReturnUnlessDraw {
        target_tokens: &'a [OwnedLexToken],
    },
    LessLifeThanYou {
        effect_tokens: &'a [OwnedLexToken],
    },
    PoisonCounters {
        count: u32,
        effect_tokens: &'a [OwnedLexToken],
    },
}

fn semantic_kw<'a>(
    expected: &'static str,
) -> impl Parser<LexStream<'a>, (), winnow::error::ErrMode<winnow::error::ContextError>> {
    any.verify(move |token: &&OwnedLexToken| {
        token.is_word(expected)
            || matches!(token.parser_word_pieces(), [piece] if piece.text == expected)
    })
    .void()
}

fn opponent_prefix<'a>(input: &mut LexStream<'a>) -> winnow::error::ModalResult<()> {
    alt((
        (
            primitives::phrase(&["for", "each"]),
            semantic_kw("opponent"),
        )
            .void(),
        (
            primitives::phrase(&["for", "each"]),
            semantic_kw("opponents"),
        )
            .void(),
        (primitives::kw("each"), semantic_kw("opponent")).void(),
        (primitives::kw("each"), semantic_kw("opponents")).void(),
    ))
    .void()
    .parse_next(input)
}

fn player_prefix<'a>(input: &mut LexStream<'a>) -> winnow::error::ModalResult<()> {
    alt((
        (primitives::phrase(&["for", "each"]), semantic_kw("player")).void(),
        (primitives::phrase(&["for", "each"]), semantic_kw("players")).void(),
        (primitives::kw("each"), semantic_kw("player")).void(),
        (primitives::kw("each"), semantic_kw("players")).void(),
    ))
    .void()
    .parse_next(input)
}

fn negated_auxiliary<'a>(input: &mut LexStream<'a>) -> winnow::error::ModalResult<()> {
    alt((
        alt((
            primitives::kw("doesn't"),
            primitives::kw("doesnt"),
            primitives::kw("don't"),
            primitives::kw("dont"),
            primitives::kw("didn't"),
            primitives::kw("didnt"),
            primitives::kw("can't"),
            primitives::kw("cant"),
            primitives::kw("cannot"),
        ))
        .void(),
        alt((
            primitives::phrase(&["does", "not"]),
            primitives::phrase(&["do", "not"]),
            primitives::phrase(&["did", "not"]),
            primitives::phrase(&["can", "not"]),
        ))
        .void(),
        alt((
            primitives::phrase(&["doesn", "t"]),
            primitives::phrase(&["don", "t"]),
            primitives::phrase(&["didn", "t"]),
            primitives::phrase(&["can", "t"]),
        ))
        .void(),
    ))
    .void()
    .parse_next(input)
}

fn tagged_action<'a>(input: &mut LexStream<'a>) -> winnow::error::ModalResult<()> {
    alt((
        primitives::kw("sacrificed"),
        primitives::kw("destroyed"),
        primitives::kw("exiled"),
        primitives::kw("discarded"),
    ))
    .void()
    .parse_next(input)
}

fn discard_action<'a>(input: &mut LexStream<'a>) -> winnow::error::ModalResult<()> {
    alt((primitives::kw("discard"), primitives::kw("discarded")))
        .void()
        .parse_next(input)
}

fn trim(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    trim_edge_punctuation_tokens(tokens)
}

pub fn parse_participant_clause_shape(
    tokens: &[OwnedLexToken],
) -> Option<ForEachParticipantClauseShape<'_>> {
    let tokens = trim(tokens);
    let tokens = primitives::parse_prefix(tokens, opt(primitives::kw("then")).void())
        .map(|(_, rest)| trim(rest))
        .unwrap_or(tokens);
    let participant_is_actor =
        primitives::parse_prefix(tokens, primitives::kw("each").void()).is_some();
    if let Some((_, rest)) = primitives::parse_prefix(tokens, opponent_prefix) {
        let mut scope = ForEachParticipantScope::Opponent;
        let mut inner_tokens = trim(rest);
        if let Some((_, rest)) = primitives::parse_prefix(
            inner_tokens,
            primitives::phrase(&["other", "than", "defending", "player"]),
        ) {
            scope = ForEachParticipantScope::OpponentExceptDefending;
            inner_tokens = trim(rest);
        }
        return Some(ForEachParticipantClauseShape {
            scope,
            participant_is_actor,
            inner_tokens,
        });
    }
    if let Some((_, rest)) = primitives::parse_prefix(
        tokens,
        (
            primitives::kw("each"),
            primitives::kw("other"),
            semantic_kw("player"),
        )
            .void(),
    ) {
        return Some(ForEachParticipantClauseShape {
            scope: ForEachParticipantScope::PlayerExceptYou,
            participant_is_actor: true,
            inner_tokens: trim(rest),
        });
    }
    let (_, rest) = primitives::parse_prefix(tokens, player_prefix)?;
    let mut scope = ForEachParticipantScope::Player;
    let mut inner_tokens = trim(rest);
    if let Some((_, rest)) = primitives::parse_prefix(
        inner_tokens,
        primitives::phrase(&["other", "than", "its", "controller"]),
    ) {
        scope = ForEachParticipantScope::PlayerExceptItsController;
        inner_tokens = trim(rest);
    }
    if let Some((_, rest)) = primitives::parse_prefix(
        inner_tokens,
        primitives::phrase(&["other", "than", "target", "player"]),
    ) {
        scope = ForEachParticipantScope::PlayerExceptTarget;
        inner_tokens = trim(rest);
    }
    if let Some((_, rest)) =
        primitives::parse_prefix(inner_tokens, primitives::phrase(&["on", "your", "team"]))
    {
        scope = ForEachParticipantScope::PlayerOnYourTeam;
        inner_tokens = trim(rest);
    }
    Some(ForEachParticipantClauseShape {
        scope,
        participant_is_actor,
        inner_tokens,
    })
}

/// Parse the source-relative participant qualifier in clauses such as
/// "each player this creature attacked this turn loses the game."
pub fn parse_source_attacked_player_clause_shape(
    tokens: &[OwnedLexToken],
) -> Option<SourceAttackedPlayerClauseShape<'_>> {
    let (_, effect_tokens) = primitives::parse_prefix(
        trim(tokens),
        (
            alt((
                primitives::phrase(&["this", "creature"]),
                primitives::phrase(&["this", "permanent"]),
                primitives::phrase(&["this", "source"]),
            )),
            primitives::phrase(&["attacked", "this", "turn"]),
        )
            .void(),
    )?;
    let effect_tokens = trim(effect_tokens);
    (!effect_tokens.is_empty()).then_some(SourceAttackedPlayerClauseShape { effect_tokens })
}

/// Parse a full-game combat-damage participant qualifier, retaining the
/// damage source as an LKI object filter rather than discarding the relative
/// clause before the participant's action.
pub fn parse_combat_damage_history_player_clause_shape(
    tokens: &[OwnedLexToken],
) -> Option<CombatDamageHistoryPlayerClauseShape<'_>> {
    let (_, tail) = primitives::parse_prefix(
        trim(tokens),
        primitives::phrase(&["dealt", "combat", "damage", "this", "game", "by"]),
    )?;
    let action_start = effect_start(tail)?;
    let source_tokens = trim(tail.get(..action_start)?);
    let effect_tokens = trim(tail.get(action_start..)?);
    (!source_tokens.is_empty() && !effect_tokens.is_empty()).then_some(
        CombatDamageHistoryPlayerClauseShape {
            source_tokens,
            effect_tokens,
        },
    )
}

fn effect_start(tokens: &[OwnedLexToken]) -> Option<usize> {
    let mut index = 0usize;
    while index < tokens.len() {
        if tokens[index].is_word("may")
            // Search and choice programs are parsed by dedicated sentence
            // grammars, so they are intentionally absent from the generic
            // chain-verb registry. They are still valid action boundaries
            // after a participant-relative predicate:
            //
            //   each player who controls ... chooses ...
            //   each player who controls ... searches ...
            //
            // Missing either boundary makes the object-filter parser absorb
            // the action and resume at a later generic verb such as
            // `sacrifices` or `puts`.
            || tokens[index].is_word("choose")
            || tokens[index].is_word("chooses")
            || tokens[index].is_word("search")
            || tokens[index].is_word("searches")
            || chain_splitting::find_chain_verb_tokens(&tokens[index..])
                .is_some_and(|found| found.word_index == 0)
        {
            return Some(index);
        }
        index += 1;
    }
    None
}

pub fn parse_relative_control_clause_shape(
    tokens: &[OwnedLexToken],
) -> Option<RelativeControlClauseShape<'_>> {
    let (_, tail) = primitives::parse_prefix(
        trim(tokens),
        (
            primitives::kw("who"),
            alt((primitives::kw("control"), primitives::kw("controls"))),
        )
            .void(),
    )?;

    // "who controls fewer lands than the player who controls the most
    // lands ..." compares the current participant's count with a global
    // per-controller maximum. Split after the nested relative clause before
    // looking for the action verb; otherwise the inner "controls" is easily
    // mistaken for the outer action boundary.
    if let Some((_, after_fewer)) = primitives::parse_prefix(tail, primitives::kw("fewer").void())
        && let Some((than_index, _, after_most)) = primitives::find_prefix(after_fewer, || {
            primitives::phrase(&["than", "the", "player", "who", "controls", "the", "most"]).void()
        })
    {
        let filter_tokens = trim(after_fewer.get(..than_index)?);
        let split = effect_start(after_most)?;
        let most_filter_tokens = trim(after_most.get(..split)?);
        let effect_tokens = trim(after_most.get(split..)?);
        if !filter_tokens.is_empty() && !most_filter_tokens.is_empty() && !effect_tokens.is_empty()
        {
            return Some(RelativeControlClauseShape {
                controls_most: false,
                count_comparison: None,
                fewer_than_most_filter_tokens: Some(most_filter_tokens),
                fewer_than_you: false,
                filter_tokens,
                effect_tokens,
            });
        }
    }

    // The participant is compared against the ability controller using the
    // same counted object set:
    //
    //   each opponent who controls fewer creatures than you draws a card
    if let Some((_, after_fewer)) = primitives::parse_prefix(tail, primitives::kw("fewer").void())
        && let Some((than_index, _, after_you)) =
            primitives::find_prefix(after_fewer, || primitives::phrase(&["than", "you"]).void())
    {
        let filter_tokens = trim(after_fewer.get(..than_index)?);
        let split = effect_start(after_you)?;
        let effect_tokens = trim(after_you.get(split..)?);
        if !filter_tokens.is_empty() && !effect_tokens.is_empty() {
            return Some(RelativeControlClauseShape {
                controls_most: false,
                count_comparison: None,
                fewer_than_most_filter_tokens: None,
                fewer_than_you: true,
                filter_tokens,
                effect_tokens,
            });
        }
    }

    // Preserve authored numeric thresholds ("six or more lands", "four or
    // fewer lands") as an actual count comparison rather than allowing the
    // object-filter parser to discard the quantity.
    if let Ok((comparison, used)) =
        parse_quantity_comparison_prefix(tail, false, false, "for-each relative control predicate")
        && !matches!(comparison, crate::effect::Comparison::Equal(_))
    {
        let after_count = tail.get(used..)?;
        let split = effect_start(after_count)?;
        let filter_tokens = trim(after_count.get(..split)?);
        let effect_tokens = trim(after_count.get(split..)?);
        if !filter_tokens.is_empty() && !effect_tokens.is_empty() {
            return Some(RelativeControlClauseShape {
                controls_most: false,
                count_comparison: Some(comparison),
                fewer_than_most_filter_tokens: None,
                fewer_than_you: false,
                filter_tokens,
                effect_tokens,
            });
        }
    }

    let split = effect_start(tail)?;
    let mut filter_tokens = trim(tail.get(..split)?);
    let effect_tokens = trim(tail.get(split..)?);
    let mut controls_most = false;
    if let Some((_, rest)) = primitives::parse_prefix(
        filter_tokens,
        alt((
            primitives::phrase(&["the", "most"]),
            primitives::kw("most").void(),
        ))
        .void(),
    ) {
        controls_most = true;
        filter_tokens = trim(rest);
    }
    (!filter_tokens.is_empty() && !effect_tokens.is_empty()).then_some(RelativeControlClauseShape {
        controls_most,
        count_comparison: None,
        fewer_than_most_filter_tokens: None,
        fewer_than_you: false,
        filter_tokens,
        effect_tokens,
    })
}

fn tagged_filter_after_action(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    let (_, after_who) = primitives::parse_prefix(trim(tokens), primitives::kw("who"))?;
    let (_, after_action) = primitives::parse_prefix(after_who, tagged_action)?;
    let (way_index, _, _) =
        primitives::find_prefix(after_action, || primitives::phrase(&["this", "way"]).void())?;
    let filter = trim(after_action.get(..way_index)?);
    (!filter.is_empty()).then_some(filter)
}

fn tagged_filter_after_negation(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    let (_, after_who) = primitives::parse_prefix(trim(tokens), primitives::kw("who"))?;
    let (_, after_negation) = primitives::parse_prefix(after_who, negated_auxiliary)?;
    let (_, after_action) = primitives::parse_prefix(after_negation, discard_action)?;
    let (way_index, _, _) =
        primitives::find_prefix(after_action, || primitives::phrase(&["this", "way"]).void())?;
    let filter = trim(after_action.get(..way_index)?);
    (!filter.is_empty()).then_some(filter)
}

pub fn parse_who_tagged_filter_shape(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    tagged_filter_after_action(tokens)
}

#[cfg(test)]
#[path = "participants_inline_tests.rs"]
mod tests;

#[path = "participants/core_programs.rs"]
mod core_programs;
use core_programs::{
    did_action_shape, did_this_way_shape, ignore_scry_or_surveil, negated_shape, tapped_land_shape,
};
pub use core_programs::{parse_opponent_special_shape, parse_who_clause_shape};
#[path = "participants/counter_programs.rs"]
mod counter_programs;
use counter_programs::poison_counters;
#[path = "participants/resource_programs.rs"]
mod resource_programs;
use resource_programs::less_life;
#[path = "participants/choice_programs.rs"]
mod choice_programs;
use choice_programs::choose_return_unless;
