use winnow::combinator::{alt, eof};
use winnow::prelude::*;

use crate::cards::builders::{PredicateAst, TargetAst, TextSpan};
use crate::effect::ChoiceCount;
use crate::grammar::primitives;
use crate::grammar::structure::{
    parse_predicate_with_grammar_entrypoint_lexed, parse_trailing_if_predicate_lexed,
    split_trailing_if_clause_lexed,
};
use crate::lexer::{OwnedLexToken, parser_token_word_refs, trim_lexed_commas};
use crate::target::PlayerFilter;
use crate::util::parse_target_phrase;
use crate::util::{parse_choice_count_before_target_prefix, parse_number_word_u32};

const ADDITIONAL_PREFIXES: &[&[&str]] = &[&["an", "additional"], &["additional"]];
const EVENT_AMOUNT_PREFIXES: &[&[&str]] = &[
    &["that", "amount", "of"],
    &["that", "much"],
    &["that", "many"],
];
const EACH_PLAYER_TARGETS: &[&[&str]] = &[&["each", "player"], &["each", "players"]];
const EACH_OPPONENT_TARGETS: &[&[&str]] = &[&["each", "opponent"], &["each", "opponents"]];
const EACH_OTHER_PLAYER_TARGETS: &[&[&str]] =
    &[&["each", "other", "player"], &["each", "other", "players"]];
const EACH_OTHER_OPPONENT_TARGETS: &[&[&str]] = &[
    &["each", "other", "opponent"],
    &["each", "other", "opponents"],
    &["all", "other", "opponents"],
];
const CREATURE_CONTROLLER_TARGETS: &[&[&str]] = &[
    &["the", "creatures", "controller"],
    &["that", "creatures", "controller"],
    &["the", "creature's", "controller"],
    &["that", "creature's", "controller"],
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CombatPlayerDamageTargetShape {
    EachPlayer,
    EachOtherPlayer,
    EachOpponent,
    EachOtherOpponent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CombatSimpleDamageTargetShape {
    DefaultAny,
    CreatureController,
    IteratedPlayer,
}

/// An object target declared inside a derived damage-recipient phrase.
///
/// “Target spell's controller” targets the spell, not its controller.  Keep
/// that distinction typed so lowering can materialize the spell target before
/// resolving both the controller and any same-clause “that spell” values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CombatEmbeddedTargetControllerShape {
    Spell,
}

#[derive(Debug, Clone, Copy)]
pub struct CombatDamageHeadShape<'a> {
    pub body_tokens: &'a [OwnedLexToken],
    pub direct_hand_size_each_opponent: bool,
    pub divided: bool,
    pub event_amount_prefix_len: Option<usize>,
    pub fallback_hand_size_each_opponent: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct CombatDividedEqualShape<'a> {
    pub amount_tokens: &'a [OwnedLexToken],
    pub target_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy)]
pub struct CombatDamageToTargetEqualShape<'a> {
    pub target_tokens: &'a [OwnedLexToken],
    pub amount_is_event_result: bool,
    pub target_is_each_or_all: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct CombatDamageEqualShape<'a> {
    pub amount_tokens: &'a [OwnedLexToken],
    pub target_tokens: &'a [OwnedLexToken],
    pub target_is_each_or_all: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CombatDividedTargetError {
    MissingTargetsAfterAmong,
    MissingTargetPhrase,
    UnsupportedTargetCount,
    MissingTargetCount,
}

#[derive(Debug, Clone)]
pub struct CombatDividedTargetShape<'a> {
    pub count: ChoiceCount,
    pub target_tokens: &'a [OwnedLexToken],
    pub any_target: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CombatDividedAmountError {
    MissingDamageKeyword,
}

#[derive(Debug, Clone, Copy)]
pub enum CombatDividedAmountShape<'a> {
    EvenlyEach {
        filter_tokens: &'a [OwnedLexToken],
    },
    Distributed {
        target_tokens: &'a [OwnedLexToken],
        evenly_rounded_down: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CombatDamageTargetShapeError {
    MissingDamageKeyword,
    UnsupportedTrailingIfClause,
    UnsupportedEmbeddedIfClause,
    MissingEachFilter,
}

#[derive(Debug, Clone)]
pub enum CombatDamageTargetShape<'a> {
    InsteadIf {
        target_tokens: &'a [OwnedLexToken],
        predicate_tokens: &'a [OwnedLexToken],
        instead_tail_tokens: &'a [OwnedLexToken],
    },
    TrailingIf {
        target_tokens: &'a [OwnedLexToken],
        predicate: PredicateAst,
    },
    TrailingUnless {
        target_tokens: &'a [OwnedLexToken],
        predicate: PredicateAst,
    },
    OmittedTargetIf {
        predicate: PredicateAst,
    },
    Simple {
        shape: CombatSimpleDamageTargetShape,
        target_tokens: &'a [OwnedLexToken],
    },
    EachOfCount {
        count: ChoiceCount,
        span_tokens: &'a [OwnedLexToken],
    },
    EachOfTarget {
        target_tokens: &'a [OwnedLexToken],
    },
    PlayerGroup(CombatPlayerDamageTargetShape),
    MaxSpeedPlayers {
        has_max_speed: bool,
    },
    OpponentWho {
        predicate_tokens: &'a [OwnedLexToken],
    },
    PlayerWho {
        predicate_tokens: &'a [OwnedLexToken],
    },
    PlayerAndObjects {
        player_filter: PlayerFilter,
        player_span: Option<TextSpan>,
        filter_tokens: &'a [OwnedLexToken],
    },
    EachObjectsAndPlayer {
        filter_tokens: &'a [OwnedLexToken],
    },
    OpponentAndControlledCreaturePlaneswalker,
    HistoricalDamageRecipients {
        players: CombatPlayerDamageTargetShape,
        filter_tokens: &'a [OwnedLexToken],
    },
    EachFilter {
        filter_tokens: &'a [OwnedLexToken],
    },
    DelayedEndOfCombat {
        target_tokens: &'a [OwnedLexToken],
    },
    General {
        target_tokens: &'a [OwnedLexToken],
    },
}

fn exact_phrase(tokens: &[OwnedLexToken], phrases: &'static [&'static [&'static str]]) -> bool {
    primitives::parse_all(
        tokens,
        (primitives::any_phrase(phrases), eof).void(),
        "combat-damage-phrase",
    )
    .is_ok()
}

fn phrase_at_start(tokens: &[OwnedLexToken], phrases: &'static [&'static [&'static str]]) -> bool {
    primitives::parse_prefix(tokens, primitives::any_phrase(phrases)).is_some()
}

pub fn parse_combat_player_damage_target_shape_lexed(
    tokens: &[OwnedLexToken],
    allow_prefix: bool,
) -> Option<CombatPlayerDamageTargetShape> {
    let matched = |phrases| {
        if allow_prefix {
            phrase_at_start(tokens, phrases)
        } else {
            exact_phrase(tokens, phrases)
        }
    };
    if matched(EACH_PLAYER_TARGETS) {
        Some(CombatPlayerDamageTargetShape::EachPlayer)
    } else if matched(EACH_OTHER_PLAYER_TARGETS) {
        Some(CombatPlayerDamageTargetShape::EachOtherPlayer)
    } else if matched(EACH_OTHER_OPPONENT_TARGETS) {
        Some(CombatPlayerDamageTargetShape::EachOtherOpponent)
    } else if matched(EACH_OPPONENT_TARGETS) {
        Some(CombatPlayerDamageTargetShape::EachOpponent)
    } else {
        None
    }
}

pub fn parse_combat_simple_damage_target_shape_lexed(
    tokens: &[OwnedLexToken],
) -> Option<CombatSimpleDamageTargetShape> {
    if exact_phrase(tokens, &[&["instead"]]) {
        Some(CombatSimpleDamageTargetShape::DefaultAny)
    } else if exact_phrase(tokens, CREATURE_CONTROLLER_TARGETS) {
        Some(CombatSimpleDamageTargetShape::CreatureController)
    } else if exact_phrase(
        tokens,
        &[&["the", "player"], &["that", "player"], &["them"]],
    ) {
        Some(CombatSimpleDamageTargetShape::IteratedPlayer)
    } else {
        None
    }
}

pub fn parse_combat_embedded_target_controller_shape_lexed(
    tokens: &[OwnedLexToken],
) -> Option<CombatEmbeddedTargetControllerShape> {
    exact_phrase(
        tokens,
        &[
            &["target", "spell's", "controller"],
            &["target", "spell’s", "controller"],
            &["target", "spells", "controller"],
        ],
    )
    .then_some(CombatEmbeddedTargetControllerShape::Spell)
}

fn required_hand_size_markers(tokens: &[OwnedLexToken]) -> bool {
    ["number", "cards", "hand"]
        .into_iter()
        .all(|word| primitives::find_prefix(tokens, || primitives::kw(word)).is_some())
}

pub fn is_combat_divided_damage_clause_lexed(tokens: &[OwnedLexToken]) -> bool {
    let Some((_idx, (), after_divided)) =
        primitives::find_prefix(tokens, || primitives::kw("divided").void())
    else {
        return false;
    };
    primitives::find_prefix(after_divided, || primitives::kw("among")).is_some()
}

pub fn parse_combat_damage_head_shape_lexed(tokens: &[OwnedLexToken]) -> CombatDamageHeadShape<'_> {
    let tokens = primitives::parse_prefix(
        tokens,
        alt((primitives::kw("deal"), primitives::kw("deals"))).void(),
    )
    .map(|(_, rest)| rest)
    .unwrap_or(tokens);
    let body_tokens = primitives::parse_prefix(tokens, primitives::any_phrase(ADDITIONAL_PREFIXES))
        .map(|(_, rest)| rest)
        .unwrap_or(tokens);

    let direct_hand_size_each_opponent = primitives::parse_prefix(
        body_tokens,
        primitives::phrase(&["damage", "to", "each", "opponent", "equal", "to"]),
    )
    .is_some_and(|(_, tail)| required_hand_size_markers(tail));
    let fallback_hand_size_each_opponent = primitives::parse_prefix(
        body_tokens,
        primitives::phrase(&["damage", "to", "each", "opponent"]),
    )
    .is_some_and(|(_, tail)| required_hand_size_markers(tail));
    let event_amount_prefix_len =
        primitives::parse_prefix(body_tokens, primitives::any_phrase(EVENT_AMOUNT_PREFIXES))
            .map(|(prefix, _)| prefix.len());

    CombatDamageHeadShape {
        body_tokens: trim_lexed_commas(body_tokens),
        direct_hand_size_each_opponent,
        divided: is_combat_divided_damage_clause_lexed(body_tokens),
        event_amount_prefix_len,
        fallback_hand_size_each_opponent,
    }
}

pub fn parse_combat_divided_equal_shape_lexed(
    tokens: &[OwnedLexToken],
) -> Option<CombatDividedEqualShape<'_>> {
    let (_, after_equal_to) =
        primitives::parse_prefix(tokens, primitives::phrase(&["damage", "equal", "to"]))?;
    let (divided_idx, (), _after_divided) =
        primitives::find_prefix(after_equal_to, || primitives::kw("divided").void())?;
    Some(CombatDividedEqualShape {
        amount_tokens: trim_lexed_commas(&after_equal_to[..divided_idx]),
        target_tokens: &after_equal_to[divided_idx..],
    })
}

pub fn parse_combat_damage_to_target_equal_shape_lexed(
    tokens: &[OwnedLexToken],
) -> Option<CombatDamageToTargetEqualShape<'_>> {
    primitives::parse_prefix(tokens, primitives::phrase(&["damage", "to"]))?;
    let (equal_idx, (), after_equal) =
        primitives::find_prefix(tokens, || primitives::phrase(&["equal", "to"]).void())?;
    let before_equal = trim_lexed_commas(tokens.get(1..equal_idx)?);
    let target_tokens = primitives::parse_prefix(before_equal, primitives::kw("to").void())
        .map(|(_, rest)| trim_lexed_commas(rest))
        .unwrap_or(before_equal);
    if target_tokens.is_empty() {
        return None;
    }
    Some(CombatDamageToTargetEqualShape {
        target_tokens,
        amount_is_event_result: exact_phrase(trim_lexed_commas(after_equal), &[&["the", "result"]]),
        target_is_each_or_all: primitives::parse_prefix(
            target_tokens,
            primitives::any_phrase(&[&["each"], &["all"]]),
        )
        .is_some(),
    })
}

fn has_target_marker(tokens: &[OwnedLexToken]) -> bool {
    primitives::find_prefix(tokens, || {
        primitives::any_phrase(&[&["target"], &["targets"]])
    })
    .is_some()
}

fn target_tail_can_parse(tokens: &[OwnedLexToken]) -> bool {
    let words = parser_token_word_refs(tokens);
    has_target_marker(tokens)
        || words.first().is_some_and(|word| {
            matches!(
                *word,
                "any"
                    | "each"
                    | "all"
                    | "it"
                    | "itself"
                    | "them"
                    | "him"
                    | "her"
                    | "that"
                    | "this"
                    | "you"
                    | "player"
                    | "opponent"
                    | "creature"
                    | "planeswalker"
            )
        })
        || parse_target_phrase(tokens).is_ok()
}

fn last_target_to_marker(tokens: &[OwnedLexToken]) -> Option<usize> {
    for idx in (0..tokens.len()).rev() {
        if !tokens[idx].is_word("to") || idx > 0 && tokens[idx - 1].is_word("up") {
            continue;
        }
        if target_tail_can_parse(&tokens[idx + 1..]) {
            return Some(idx);
        }
    }
    None
}

pub fn parse_combat_damage_equal_shape_lexed(
    tokens: &[OwnedLexToken],
) -> Option<CombatDamageEqualShape<'_>> {
    primitives::parse_prefix(tokens, primitives::phrase(&["damage", "equal", "to"]))?;
    let target_to_idx = last_target_to_marker(tokens.get(3..)?).map(|idx| idx + 3)?;
    let amount_start = primitives::parse_prefix(tokens, primitives::kw("damage").void())
        .map(|_| 1)
        .unwrap_or(0);
    let amount_tokens = trim_lexed_commas(tokens.get(amount_start..target_to_idx)?);
    let raw_target_tokens = trim_lexed_commas(tokens.get(target_to_idx + 1..)?);
    if amount_tokens.is_empty() || raw_target_tokens.is_empty() {
        return None;
    }
    let target_tokens = if let Some((_prefix, each_of_tokens)) =
        primitives::parse_prefix(raw_target_tokens, primitives::phrase(&["each", "of"]))
        && has_target_marker(each_of_tokens)
    {
        each_of_tokens
    } else {
        raw_target_tokens
    };
    let target_is_each_or_all = primitives::parse_prefix(
        target_tokens,
        primitives::any_phrase(&[&["each"], &["all"]]),
    )
    .is_some();
    Some(CombatDamageEqualShape {
        amount_tokens,
        target_tokens,
        target_is_each_or_all,
    })
}

pub fn parse_combat_divided_target_shape_lexed(
    tokens: &[OwnedLexToken],
) -> Result<CombatDividedTargetShape<'_>, CombatDividedTargetError> {
    let Some((_among_idx, (), after_among)) =
        primitives::find_prefix(tokens, || primitives::kw("among").void())
    else {
        return Err(CombatDividedTargetError::MissingTargetsAfterAmong);
    };
    let among_tail = trim_lexed_commas(after_among);
    if let Some((_prefix, target_tokens)) = primitives::parse_prefix(
        among_tail,
        primitives::phrase(&["any", "number", "of"]).void(),
    ) {
        let target_tokens = trim_lexed_commas(target_tokens);
        if target_tokens
            .first()
            .is_some_and(|token| matches!(token.as_word(), Some("those" | "them")))
        {
            return Ok(CombatDividedTargetShape {
                count: ChoiceCount::any_number(),
                target_tokens,
                any_target: false,
            });
        }
    }
    let Some((target_idx, _target_marker, _after_target)) =
        primitives::find_prefix(among_tail, || {
            primitives::any_phrase(&[&["target"], &["targets"]])
        })
    else {
        return Err(CombatDividedTargetError::MissingTargetPhrase);
    };

    let count = if let Some((count, used)) = parse_choice_count_before_target_prefix(among_tail) {
        if used != target_idx {
            return Err(CombatDividedTargetError::UnsupportedTargetCount);
        }
        count
    } else {
        let max_targets = parser_token_word_refs(&among_tail[..target_idx])
            .into_iter()
            .filter_map(parse_number_word_u32)
            .max()
            .ok_or(CombatDividedTargetError::MissingTargetCount)?;
        ChoiceCount {
            min: 1,
            max: Some(max_targets as usize),
            dynamic_x: false,
            up_to_x: false,
            random: false,
            explicit_exactly: false,
        }
    };

    let target_tokens = &among_tail[target_idx..];
    let any_target = exact_phrase(target_tokens, &[&["target"], &["targets"]]);
    Ok(CombatDividedTargetShape {
        count,
        target_tokens,
        any_target,
    })
}

pub fn parse_combat_divided_amount_shape_lexed(
    tokens: &[OwnedLexToken],
    used: usize,
) -> Result<CombatDividedAmountShape<'_>, CombatDividedAmountError> {
    let rest = tokens
        .get(used..)
        .ok_or(CombatDividedAmountError::MissingDamageKeyword)?;
    let Some(((), after_damage)) = primitives::parse_prefix(rest, primitives::kw("damage").void())
    else {
        return Err(CombatDividedAmountError::MissingDamageKeyword);
    };
    let target_tokens = primitives::parse_prefix(after_damage, primitives::kw("to").void())
        .map(|(_, rest)| rest)
        .unwrap_or(after_damage);
    let evenly_rounded_down = primitives::find_prefix(target_tokens, || primitives::kw("evenly"))
        .is_some()
        && primitives::find_prefix(target_tokens, || primitives::phrase(&["rounded", "down"]))
            .is_some();
    if evenly_rounded_down
        && let Some((_among_idx, (), after_among)) =
            primitives::find_prefix(target_tokens, || primitives::kw("among").void())
    {
        let among_tail = trim_lexed_commas(after_among);
        if let Some((_head, filter_tokens)) = primitives::parse_prefix(
            among_tail,
            primitives::any_phrase(&[&["all"], &["each"], &["every"]]),
        ) && !filter_tokens.is_empty()
        {
            return Ok(CombatDividedAmountShape::EvenlyEach { filter_tokens });
        }
    }
    Ok(CombatDividedAmountShape::Distributed {
        target_tokens,
        evenly_rounded_down,
    })
}

fn phrase_occurs(tokens: &[OwnedLexToken], phrase: &'static [&'static str]) -> bool {
    primitives::find_prefix(tokens, || primitives::phrase(phrase)).is_some()
}

fn one_of_words_occurs(tokens: &[OwnedLexToken], words: &'static [&'static str]) -> bool {
    words
        .iter()
        .any(|word| primitives::find_prefix(tokens, || primitives::kw(word)).is_some())
}

fn normalize_damage_target_tokens(
    tokens: &[OwnedLexToken],
    used: usize,
) -> Result<&[OwnedLexToken], CombatDamageTargetShapeError> {
    let rest = tokens
        .get(used..)
        .ok_or(CombatDamageTargetShapeError::MissingDamageKeyword)?;
    let Some(((), after_damage)) = primitives::parse_prefix(rest, primitives::kw("damage").void())
    else {
        return Err(CombatDamageTargetShapeError::MissingDamageKeyword);
    };
    let mut target_tokens = primitives::parse_prefix(after_damage, primitives::kw("to").void())
        .map(|(_, rest)| rest)
        .unwrap_or(after_damage);
    if let Some((_among_idx, (), after_among)) =
        primitives::find_prefix(target_tokens, || primitives::kw("among").void())
    {
        let has_target = has_target_marker(after_among);
        let has_supported_kind =
            one_of_words_occurs(after_among, &["player", "players", "creature", "creatures"]);
        if has_target && has_supported_kind {
            target_tokens = after_among;
        }
    }
    // A damage amount may be bound by a trailing `where X is ...` clause.
    // That clause is consumed by the effect dispatcher after the target
    // shape is recognized, so it must not be treated as part of the target
    // phrase here.
    if let Some((where_idx, (), _)) =
        primitives::find_prefix(target_tokens, || primitives::kw("where").void())
    {
        target_tokens = &target_tokens[..where_idx];
    }
    target_tokens = trim_lexed_commas(target_tokens);
    while target_tokens.last().is_some_and(|token| token.is_period()) {
        target_tokens = trim_lexed_commas(&target_tokens[..target_tokens.len() - 1]);
    }
    Ok(target_tokens)
}

#[cfg(test)]
#[path = "damage_inline_tests.rs"]
mod tests;

#[path = "damage/combat_programs.rs"]
mod combat_programs;
pub use combat_programs::parse_combat_damage_target_shape_lexed;
