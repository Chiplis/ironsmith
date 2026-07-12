use super::super::*;

use crate::cards::builders::KeywordAction;
use winnow::combinator::{alt, opt, peek, repeat_till};
use winnow::error::{ContextError, ErrMode, ModalResult as WResult};
use winnow::token::any;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NextTurnCantShape<'a> {
    pub(crate) restriction_tokens: &'a [OwnedLexToken],
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

pub(crate) fn parse_next_turn_cant_shape_tokens(
    tokens: &[OwnedLexToken],
) -> Option<NextTurnCantShape<'_>> {
    primitives::parse_all(tokens, next_turn_cant, "next-turn restriction").ok()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DirectClauseShape {
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
    ChaosEnsues,
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
            primitives::phrase(&["chaos", "ensues"]).value(DirectClauseShape::ChaosEnsues),
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
    primitives::parse_all(
        tokens,
        (parser, primitives::sentence_end()).map(|(shape, _)| shape),
        "direct clause shape",
    )
    .ok()
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

pub(crate) fn parse_direct_clause_shape(tokens: &[OwnedLexToken]) -> Option<DirectClauseShape> {
    if damaged_players_cant_gain_life(tokens) {
        Some(DirectClauseShape::DamagedPlayersCantGainLife)
    } else {
        exact_shape(tokens)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SharedAbilityGainShape {
    pub(crate) abilities: Vec<KeywordAction>,
}

pub(crate) fn parse_shared_ability_gain_shape(
    tokens: &[OwnedLexToken],
) -> Option<SharedAbilityGainShape> {
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
pub(crate) struct ProtectionChoiceShape {
    pub(crate) includes_colorless: bool,
    pub(crate) includes_artifacts: bool,
}

pub(crate) fn parse_protection_choice_shape(
    tokens: &[OwnedLexToken],
) -> Option<ProtectionChoiceShape> {
    let fixed_option = alt((
        primitives::phrase(&["colorless", "or", "from"]).value((true, false)),
        primitives::phrase(&["artifacts", "or", "from"]).value((false, true)),
        winnow::combinator::empty.value((false, false)),
    ));
    let choice = primitives::any_phrase(&[
        &["the", "color", "of", "your", "choice"],
        &["the", "color", "of", "its", "controller's", "choice"],
        &["the", "color", "of", "its", "controllers", "choice"],
    ]);
    primitives::parse_all(
        tokens,
        (
            primitives::phrase(&["protection", "from"]),
            fixed_option,
            choice,
            primitives::phrase(&["until", "end", "of", "turn"]),
            primitives::sentence_end(),
        )
            .map(|(_, (includes_colorless, includes_artifacts), _, _, _)| {
                ProtectionChoiceShape {
                    includes_colorless,
                    includes_artifacts,
                }
            }),
        "protection choice shape",
    )
    .ok()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AssignDamageSourceShape<'a> {
    Source,
    Tagged,
    Target(&'a [OwnedLexToken]),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AssignsNoCombatDamageShape<'a> {
    Supported(AssignDamageSourceShape<'a>),
    Unsupported,
}

pub(crate) fn parse_assigns_no_combat_damage_shape(
    tokens: &[OwnedLexToken],
) -> Option<AssignsNoCombatDamageShape<'_>> {
    let (subject_tokens, tail_tokens) = primitives::split_lexed_once_on_separator(tokens, || {
        alt((primitives::kw("assign"), primitives::kw("assigns"))).void()
    })?;
    if primitives::parse_prefix(tail_tokens, primitives::phrase(&["no", "combat", "damage"]))
        .is_none()
    {
        return None;
    }
    let supported_tail = exact(
        tail_tokens,
        (
            primitives::phrase(&["no", "combat", "damage"]),
            opt(primitives::any_phrase(&[
                &["this", "turn"],
                &["this", "combat"],
            ])),
        )
            .void(),
    );
    if !supported_tail {
        return Some(AssignsNoCombatDamageShape::Unsupported);
    }
    let subject_tokens = trim_lexed_commas(subject_tokens);
    let source = if subject_tokens.is_empty()
        || exact(
            subject_tokens,
            primitives::any_phrase(&[&["this"], &["this", "creature"]]).void(),
        ) {
        AssignDamageSourceShape::Source
    } else if exact(subject_tokens, primitives::kw("it").void()) {
        AssignDamageSourceShape::Tagged
    } else {
        AssignDamageSourceShape::Target(subject_tokens)
    };
    Some(AssignsNoCombatDamageShape::Supported(source))
}

pub(crate) fn strip_optional_you_choice_tokens(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    primitives::parse_prefix(tokens, primitives::kw("you"))
        .map(|(_, rest)| rest)
        .unwrap_or(tokens)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ChooseTargetShape<'a> {
    pub(crate) target_tokens: &'a [OwnedLexToken],
}

pub(crate) fn parse_choose_target_shape(tokens: &[OwnedLexToken]) -> Option<ChooseTargetShape<'_>> {
    let (_, tail) = primitives::parse_prefix(
        tokens,
        alt((primitives::kw("choose"), primitives::kw("chooses"))),
    )?;
    primitives::parse_prefix(tail, primitives::kw("target"))?;
    Some(ChooseTargetShape {
        target_tokens: trim_lexed_commas(tail),
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TargetOnlyShape<'a> {
    pub(crate) target_tokens: &'a [OwnedLexToken],
    pub(crate) restriction_like: bool,
}

pub(crate) fn parse_target_only_shape(tokens: &[OwnedLexToken]) -> Option<TargetOnlyShape<'_>> {
    primitives::parse_prefix(tokens, primitives::kw("target"))?;
    if super::parse_clause_subject_verb_shape(tokens).is_some() {
        return None;
    }
    let restriction_like = [
        "blocked", "except", "unless", "attack", "attacks", "block", "blocks",
    ]
    .into_iter()
    .any(|word| primitives::find_prefix(tokens, || primitives::kw(word)).is_some());
    Some(TargetOnlyShape {
        target_tokens: tokens,
        restriction_like,
    })
}

pub(crate) fn parse_embedded_choose_target_shape(
    tokens: &[OwnedLexToken],
) -> Option<ChooseTargetShape<'_>> {
    let (_, _, target_tokens) = primitives::find_prefix(tokens, || {
        (
            alt((primitives::kw("choose"), primitives::kw("chooses"))),
            peek(primitives::kw("target")),
        )
            .void()
    })?;
    Some(ChooseTargetShape {
        target_tokens: trim_lexed_commas(target_tokens),
    })
}

#[cfg(test)]
#[path = "direct/tests.rs"]
mod tests;
