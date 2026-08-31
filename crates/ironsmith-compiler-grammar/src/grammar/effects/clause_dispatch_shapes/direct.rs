use super::super::*;

use crate::cards::builders::KeywordAction;
use crate::effect::Until;
use winnow::combinator::{alt, opt, peek, repeat_till};
use winnow::error::{ContextError, ErrMode, ModalResult as WResult};
use winnow::token::any;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NextTurnCantShape<'a> {
    pub restriction_tokens: &'a [OwnedLexToken],
}

fn next_turn_cant<'a>(input: &mut LexStream<'a>) -> WResult<NextTurnCantShape<'a>> {
    let restriction_tokens = repeat_till(
        1..,
        any.void(),
        peek(alt((
            primitives::phrase(&["during", "that", "players", "next", "turn"]),
            primitives::phrase(&["during", "that", "player's", "next", "turn"]),
            primitives::phrase(&["during", "that", "player", "s", "next", "turn"]),
        ))),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    alt((
        primitives::phrase(&["during", "that", "players", "next", "turn"]),
        primitives::phrase(&["during", "that", "player's", "next", "turn"]),
        primitives::phrase(&["during", "that", "player", "s", "next", "turn"]),
    ))
    .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(NextTurnCantShape {
        restriction_tokens: trim_lexed_commas(restriction_tokens),
    })
}

pub fn parse_next_turn_cant_shape_tokens(
    tokens: &[OwnedLexToken],
) -> Option<NextTurnCantShape<'_>> {
    crate::grammar::primitives::probe_all(tokens, next_turn_cant, "next-turn restriction")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirectClauseShape {
    RingTemptsYou,
    TakeInitiative,
    ChooseOddOrEven,
    ChooseLeftOrRight,
    ClearSuspected,
    CopySourceExiledCard,
    PutTaggedPlusOneCounter,
    DamagedPlayersCantGainLife,
    DamageCantBePrevented,
    TurnSourceExiledFaceUp,
    TurnTaggedFaceUp,
    Planeswalk,
    AssembleContraption,
    ChaosEnsues,
    AbandonScheme,
    DoubleX,
    OnlyChosenCanAttack,
    OnlyChosenCanBlock,
    CastNonlandTaggedThisWay,
}

fn exact<'a, P>(tokens: &'a [OwnedLexToken], parser: P) -> bool
where
    P: Parser<LexStream<'a>, (), ErrMode<ContextError>>,
{
    primitives::parse_all(
        tokens,
        (parser, primitives::sentence_end()).void(),
        "direct dispatch shape",
    )
    .is_ok()
}

fn exact_shape(tokens: &[OwnedLexToken]) -> Option<DirectClauseShape> {
    let parser = alt((
        alt((
            primitives::phrase(&["the", "ring", "tempts", "you"])
                .value(DirectClauseShape::RingTemptsYou),
            primitives::phrase(&["you", "take", "the", "initiative"])
                .value(DirectClauseShape::TakeInitiative),
            primitives::phrase(&["choose", "odd", "or", "even"])
                .value(DirectClauseShape::ChooseOddOrEven),
            primitives::phrase(&["choose", "left", "or", "right"])
                .value(DirectClauseShape::ChooseLeftOrRight),
            primitives::phrase(&[
                "all",
                "suspected",
                "creatures",
                "are",
                "no",
                "longer",
                "suspected",
            ])
            .value(DirectClauseShape::ClearSuspected),
        )),
        alt((
            primitives::phrase(&["copy", "a", "card", "exiled", "with", "this", "artifact"])
                .value(DirectClauseShape::CopySourceExiledCard),
            primitives::any_phrase(&[
                &[
                    "this", "creature", "enters", "with", "a", "+1/+1", "counter", "on", "it",
                ],
                &[
                    "this",
                    "permanent",
                    "enters",
                    "with",
                    "a",
                    "+1/+1",
                    "counter",
                    "on",
                    "it",
                ],
                &["it", "enters", "with", "a", "+1/+1", "counter", "on", "it"],
            ])
            .value(DirectClauseShape::PutTaggedPlusOneCounter),
            primitives::any_phrase(&[
                &["the", "damage", "cant", "be", "prevented"],
                &["the", "damage", "can't", "be", "prevented"],
                &["damage", "cant", "be", "prevented"],
                &["damage", "can't", "be", "prevented"],
                &["that", "damage", "cant", "be", "prevented"],
                &["that", "damage", "can't", "be", "prevented"],
            ])
            .value(DirectClauseShape::DamageCantBePrevented),
            primitives::any_phrase(&[
                &["turn", "the", "exiled", "card", "face", "up"],
                &["turn", "exiled", "card", "face", "up"],
            ])
            .value(DirectClauseShape::TurnSourceExiledFaceUp),
            primitives::any_phrase(&[
                &["turn", "it", "face", "up"],
                &["turn", "that", "card", "face", "up"],
            ])
            .value(DirectClauseShape::TurnTaggedFaceUp),
        )),
        alt((
            primitives::kw("planeswalk").value(DirectClauseShape::Planeswalk),
            primitives::phrase(&["assemble", "a", "contraption"])
                .value(DirectClauseShape::AssembleContraption),
            primitives::phrase(&["chaos", "ensues"]).value(DirectClauseShape::ChaosEnsues),
            primitives::phrase(&["abandon", "this", "scheme"])
                .value(DirectClauseShape::AbandonScheme),
            primitives::phrase(&["double", "the", "value", "of", "x"])
                .value(DirectClauseShape::DoubleX),
            primitives::any_phrase(&[
                &[
                    "only",
                    "the",
                    "chosen",
                    "creatures",
                    "can",
                    "attack",
                    "during",
                    "that",
                    "combat",
                    "phase",
                ],
                &[
                    "only",
                    "chosen",
                    "creatures",
                    "can",
                    "attack",
                    "during",
                    "that",
                    "combat",
                    "phase",
                ],
            ])
            .value(DirectClauseShape::OnlyChosenCanAttack),
            primitives::any_phrase(&[
                &[
                    "only",
                    "the",
                    "chosen",
                    "creatures",
                    "can",
                    "block",
                    "during",
                    "that",
                    "combat",
                    "phase",
                ],
                &[
                    "only",
                    "chosen",
                    "creatures",
                    "can",
                    "block",
                    "during",
                    "that",
                    "combat",
                    "phase",
                ],
            ])
            .value(DirectClauseShape::OnlyChosenCanBlock),
        )),
        primitives::phrase(&[
            "cast", "any", "number", "of", "spells", "from", "among", "the", "nonland", "cards",
            "exiled", "this", "way", "without", "paying", "their", "mana", "costs",
        ])
        .value(DirectClauseShape::CastNonlandTaggedThisWay),
    ));
    crate::grammar::primitives::probe_all(
        tokens,
        (parser, primitives::sentence_end()).map(|(shape, _)| shape),
        "direct clause shape",
    )
}

fn damaged_players_cant_gain_life(tokens: &[OwnedLexToken]) -> bool {
    let Some((_, rest)) = primitives::parse_prefix(tokens, primitives::kw("if")) else {
        return false;
    };
    let has_condition = primitives::find_prefix(rest, || {
        primitives::phrase(&["would", "gain", "life", "this", "turn"])
    })
    .is_some();
    let has_tail = primitives::split_lexed_once_before_suffix(tokens, 1, || {
        (
            primitives::phrase(&["gains", "no", "life", "instead"]),
            primitives::sentence_end(),
        )
            .void()
    })
    .is_some();
    has_condition && has_tail
}

pub fn parse_direct_clause_shape(tokens: &[OwnedLexToken]) -> Option<DirectClauseShape> {
    if damaged_players_cant_gain_life(tokens) {
        Some(DirectClauseShape::DamagedPlayersCantGainLife)
    } else {
        exact_shape(tokens)
    }
}

/// Returns the damage body before a terminal unpreventable rider. Both the
/// explicit repeated subject (`and that damage can't be prevented`) and the
/// normalized elided subject are recognized by typed token grammar here.
pub fn split_terminal_unpreventable_damage_rider(
    tokens: &[OwnedLexToken],
) -> Option<&[OwnedLexToken]> {
    if let Some((head, ())) = primitives::split_lexed_once_before_suffix(tokens, 1, || {
        (
            primitives::kw("and"),
            alt((primitives::kw("the"), primitives::kw("that"))),
            primitives::kw("damage"),
            alt((primitives::kw("cant"), primitives::kw("can't"))),
            primitives::phrase(&["be", "prevented"]),
            primitives::sentence_end(),
        )
            .void()
    }) {
        return Some(trim_lexed_commas(head));
    }

    let (head, ()) = primitives::split_lexed_once_before_suffix(tokens, 1, || {
        (
            alt((primitives::kw("cant"), primitives::kw("can't"))),
            primitives::phrase(&["be", "prevented"]),
            primitives::sentence_end(),
        )
            .void()
    })?;
    let deals_damage = primitives::find_prefix(head, || {
        (
            alt((primitives::kw("deal"), primitives::kw("deals"))),
            repeat_till(0.., any.void(), peek(primitives::kw("damage"))).map(|((), _)| ()),
            primitives::kw("damage"),
        )
            .void()
    })
    .is_some();
    deals_damage.then(|| trim_lexed_commas(head))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TurnTargetFaceUpShape<'a> {
    pub target_tokens: &'a [OwnedLexToken],
}

pub fn parse_turn_target_face_up_shape(
    tokens: &[OwnedLexToken],
) -> Option<TurnTargetFaceUpShape<'_>> {
    let (_, tail) = primitives::parse_prefix(tokens, primitives::kw("turn"))?;
    let (target_tokens, ()) = primitives::split_lexed_once_before_suffix(tail, 1, || {
        (
            primitives::phrase(&["face", "up"]),
            primitives::sentence_end(),
        )
            .void()
    })?;
    let target_tokens = trim_lexed_commas(target_tokens);
    super::super::super::activation_restrictions::parse_target_indicator_tokens(target_tokens)?;
    (!target_tokens.is_empty()).then_some(TurnTargetFaceUpShape { target_tokens })
}

#[derive(Debug, Clone, PartialEq)]
pub struct SharedAbilityGainShape {
    pub abilities: Vec<KeywordAction>,
}

pub fn parse_shared_ability_gain_shape(tokens: &[OwnedLexToken]) -> Option<SharedAbilityGainShape> {
    let (_, body) =
        primitives::parse_prefix(tokens, primitives::phrase(&["all", "abilities", "and"]))?;
    let (_, _, ability_tokens) = primitives::find_prefix(body, || {
        alt((primitives::kw("gain"), primitives::kw("gains")))
    })?;
    let mut abilities = Vec::new();
    for (keyword, action) in [
        ("hexproof", KeywordAction::Hexproof),
        ("flying", KeywordAction::Flying),
        ("haste", KeywordAction::Haste),
    ] {
        if primitives::find_prefix(ability_tokens, || primitives::kw(keyword)).is_some() {
            abilities.push(action);
        }
    }
    (!abilities.is_empty()).then_some(SharedAbilityGainShape { abilities })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtectionChoiceChooserShape {
    You,
    TargetController,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProtectionChoiceShape {
    pub includes_colorless: bool,
    pub includes_artifacts: bool,
    pub chooses_card_type: bool,
    pub chooser: ProtectionChoiceChooserShape,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignDamageSourceShape<'a> {
    Source,
    Tagged,
    Target(&'a [OwnedLexToken]),
}

#[derive(Debug, Clone, PartialEq)]
pub enum AssignsNoCombatDamageShape<'a> {
    Supported {
        source: AssignDamageSourceShape<'a>,
        duration: Until,
    },
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChooseTargetChooserShape {
    AbilityController,
    ItsController,
    /// "That opponent" is the controller of the immediately preceding
    /// target, while also preserving the authored opponent attribution for a
    /// later "your opponent chose" reference.
    ThatOpponent,
    Unresolved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChooseTargetShape<'a> {
    pub target_tokens: &'a [OwnedLexToken],
    pub chooser: ChooseTargetChooserShape,
    /// `another player controls` is relative to the player making this
    /// authored target choice, not necessarily the ability's controller.
    pub excludes_chooser_controller: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TargetOnlyShape<'a> {
    pub target_tokens: &'a [OwnedLexToken],
    pub restriction_like: bool,
}

#[cfg(test)]
#[path = "direct/tests.rs"]
mod tests;

#[path = "direct/choice.rs"]
mod choice_programs;
use choice_programs::target_phrase_excludes_chooser_controller;
pub use choice_programs::{
    parse_choose_target_shape, parse_embedded_choose_target_shape, parse_protection_choice_shape,
    strip_optional_you_choice_tokens,
};
#[path = "direct/reference.rs"]
mod reference_programs;
pub use reference_programs::parse_target_only_shape;
#[path = "direct/combat.rs"]
mod combat_programs;
pub use combat_programs::parse_assigns_no_combat_damage_shape;
