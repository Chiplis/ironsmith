use winnow::combinator::{alt, eof};
use winnow::prelude::*;

use crate::cards::builders::{PredicateAst, TargetAst, TextSpan};
use crate::effect::ChoiceCount;
use crate::runtime_backend::grammar::primitives;
use crate::runtime_backend::grammar::structure::{
    parse_predicate_with_grammar_entrypoint_lexed, parse_trailing_if_predicate_lexed,
    split_trailing_if_clause_lexed,
};
use crate::runtime_backend::lexer::{OwnedLexToken, parser_token_word_refs, trim_lexed_commas};
use crate::runtime_backend::util::parse_target_phrase;
use crate::runtime_backend::util::{
    parse_choice_count_before_target_prefix, parse_number_word_u32,
};
use crate::target::PlayerFilter;

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
pub(crate) enum CombatPlayerDamageTargetShape {
    EachPlayer,
    EachOtherPlayer,
    EachOpponent,
    EachOtherOpponent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CombatSimpleDamageTargetShape {
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
pub(crate) enum CombatEmbeddedTargetControllerShape {
    Spell,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct CombatDamageHeadShape<'a> {
    pub(crate) body_tokens: &'a [OwnedLexToken],
    pub(crate) direct_hand_size_each_opponent: bool,
    pub(crate) divided: bool,
    pub(crate) event_amount_prefix_len: Option<usize>,
    pub(crate) fallback_hand_size_each_opponent: bool,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct CombatDividedEqualShape<'a> {
    pub(crate) amount_tokens: &'a [OwnedLexToken],
    pub(crate) target_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct CombatDamageToTargetEqualShape<'a> {
    pub(crate) target_tokens: &'a [OwnedLexToken],
    pub(crate) amount_is_event_result: bool,
    pub(crate) target_is_each_or_all: bool,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct CombatDamageEqualShape<'a> {
    pub(crate) amount_tokens: &'a [OwnedLexToken],
    pub(crate) target_tokens: &'a [OwnedLexToken],
    pub(crate) target_is_each_or_all: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CombatDividedTargetError {
    MissingTargetsAfterAmong,
    MissingTargetPhrase,
    UnsupportedTargetCount,
    MissingTargetCount,
}

#[derive(Debug, Clone)]
pub(crate) struct CombatDividedTargetShape<'a> {
    pub(crate) count: ChoiceCount,
    pub(crate) target_tokens: &'a [OwnedLexToken],
    pub(crate) any_target: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CombatDividedAmountError {
    MissingDamageKeyword,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum CombatDividedAmountShape<'a> {
    EvenlyEach {
        filter_tokens: &'a [OwnedLexToken],
    },
    Distributed {
        target_tokens: &'a [OwnedLexToken],
        evenly_rounded_down: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CombatDamageTargetShapeError {
    MissingDamageKeyword,
    UnsupportedTrailingIfClause,
    UnsupportedEmbeddedIfClause,
    MissingEachFilter,
}

#[derive(Debug, Clone)]
pub(crate) enum CombatDamageTargetShape<'a> {
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

pub(crate) fn parse_combat_player_damage_target_shape_lexed(
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

pub(crate) fn parse_combat_simple_damage_target_shape_lexed(
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

pub(crate) fn parse_combat_embedded_target_controller_shape_lexed(
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

pub(crate) fn is_combat_divided_damage_clause_lexed(tokens: &[OwnedLexToken]) -> bool {
    let Some((_idx, (), after_divided)) =
        primitives::find_prefix(tokens, || primitives::kw("divided").void())
    else {
        return false;
    };
    primitives::find_prefix(after_divided, || primitives::kw("among")).is_some()
}

pub(crate) fn parse_combat_damage_head_shape_lexed(
    tokens: &[OwnedLexToken],
) -> CombatDamageHeadShape<'_> {
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

pub(crate) fn parse_combat_divided_equal_shape_lexed(
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

pub(crate) fn parse_combat_damage_to_target_equal_shape_lexed(
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
    let (idx, (), after_to) = primitives::find_prefix(tokens, || primitives::kw("to").void())?;
    let later = last_target_to_marker(after_to).map(|next| idx + 1 + next);
    if later.is_some() {
        later
    } else if target_tail_can_parse(after_to) {
        Some(idx)
    } else {
        None
    }
}

pub(crate) fn parse_combat_damage_equal_shape_lexed(
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

pub(crate) fn parse_combat_divided_target_shape_lexed(
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

pub(crate) fn parse_combat_divided_amount_shape_lexed(
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
        .any(|word| primitives::find_prefix(tokens, || primitives::kw(*word)).is_some())
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

pub(crate) fn parse_combat_damage_target_shape_lexed(
    tokens: &[OwnedLexToken],
    used: usize,
) -> Result<CombatDamageTargetShape<'_>, CombatDamageTargetShapeError> {
    let target_tokens = normalize_damage_target_tokens(tokens, used)?;

    if let Some((instead_idx, (), predicate_tokens)) =
        primitives::find_prefix(target_tokens, || {
            primitives::phrase(&["instead", "if"]).void()
        })
    {
        return Ok(CombatDamageTargetShape::InsteadIf {
            target_tokens: trim_lexed_commas(&target_tokens[..instead_idx]),
            predicate_tokens: trim_lexed_commas(predicate_tokens),
            instead_tail_tokens: &target_tokens[instead_idx..],
        });
    }
    if let Some((unless_idx, (), predicate_tokens)) =
        primitives::find_prefix(target_tokens, || primitives::kw("unless").void())
    {
        let leading_tokens = trim_lexed_commas(&target_tokens[..unless_idx]);
        let predicate_tokens = trim_lexed_commas(predicate_tokens);
        if !leading_tokens.is_empty()
            && !predicate_tokens.is_empty()
            && let Ok(predicate) = parse_predicate_with_grammar_entrypoint_lexed(predicate_tokens)
        {
            return Ok(CombatDamageTargetShape::TrailingUnless {
                target_tokens: leading_tokens,
                predicate,
            });
        }
    }
    if let Some(spec) = split_trailing_if_clause_lexed(target_tokens) {
        return Ok(CombatDamageTargetShape::TrailingIf {
            target_tokens: spec.leading_tokens,
            predicate: spec.predicate,
        });
    }
    if primitives::parse_prefix(target_tokens, primitives::kw("if")).is_some() {
        let predicate = parse_trailing_if_predicate_lexed(target_tokens)
            .ok_or(CombatDamageTargetShapeError::UnsupportedTrailingIfClause)?;
        return Ok(CombatDamageTargetShape::OmittedTargetIf { predicate });
    }
    if primitives::find_prefix(target_tokens, || primitives::kw("if")).is_some() {
        return Err(CombatDamageTargetShapeError::UnsupportedEmbeddedIfClause);
    }

    if let Some(shape) = parse_combat_simple_damage_target_shape_lexed(target_tokens) {
        return Ok(CombatDamageTargetShape::Simple {
            shape,
            target_tokens,
        });
    }
    if let Some((_prefix, each_of_tokens)) =
        primitives::parse_prefix(target_tokens, primitives::phrase(&["each", "of"]))
    {
        if let Some((count, used)) = parse_choice_count_before_target_prefix(each_of_tokens)
            && each_of_tokens.len() == used + 1
        {
            return Ok(CombatDamageTargetShape::EachOfCount {
                count,
                span_tokens: each_of_tokens,
            });
        }
        if has_target_marker(each_of_tokens) {
            return Ok(CombatDamageTargetShape::EachOfTarget {
                target_tokens: each_of_tokens,
            });
        }
    }
    if let Some(shape) = parse_combat_player_damage_target_shape_lexed(target_tokens, false) {
        return Ok(CombatDamageTargetShape::PlayerGroup(shape));
    }

    let each_or_all = primitives::parse_prefix(
        target_tokens,
        primitives::any_phrase(&[&["each"], &["all"]]),
    );
    let max_speed_players = each_or_all.is_some()
        && one_of_words_occurs(target_tokens, &["player", "players"])
        && phrase_occurs(target_tokens, &["max", "speed"]);
    if max_speed_players {
        let negated =
            one_of_words_occurs(target_tokens, &["does", "doesnt", "doesn", "dont", "not"])
                || phrase_occurs(target_tokens, &["does", "not"]);
        return Ok(CombatDamageTargetShape::MaxSpeedPlayers {
            has_max_speed: !negated,
        });
    }

    if primitives::parse_prefix(
        target_tokens,
        primitives::any_phrase(&[&["each", "opponent", "who"], &["each", "opponents", "who"]]),
    )
    .is_some()
        && phrase_occurs(target_tokens, &["this", "way"])
    {
        return Ok(CombatDamageTargetShape::OpponentWho {
            predicate_tokens: target_tokens.get(2..).unwrap_or_default(),
        });
    }
    if primitives::parse_prefix(
        target_tokens,
        primitives::any_phrase(&[&["each", "player", "who"], &["each", "players", "who"]]),
    )
    .is_some()
        && phrase_occurs(target_tokens, &["this", "way"])
    {
        return Ok(CombatDamageTargetShape::PlayerWho {
            predicate_tokens: target_tokens.get(2..).unwrap_or_default(),
        });
    }

    if let Some((and_idx, _phrase, _after)) = primitives::find_prefix(target_tokens, || {
        primitives::any_phrase(&[&["and", "each"], &["and", "all"]])
    }) && and_idx > 0
    {
        let player_tokens = trim_lexed_commas(&target_tokens[..and_idx]);
        let filter_tokens = trim_lexed_commas(&target_tokens[and_idx + 1..]);
        if !player_tokens.is_empty()
            && !filter_tokens.is_empty()
            && one_of_words_occurs(filter_tokens, &["creature", "creatures"])
            && let Ok(TargetAst::Player(player_filter, player_span)) =
                parse_target_phrase(player_tokens)
        {
            return Ok(CombatDamageTargetShape::PlayerAndObjects {
                player_filter,
                player_span,
                filter_tokens,
            });
        }
    }

    if each_or_all.is_some()
        && let Some((and_idx, _phrase, after_phrase)) =
            primitives::find_prefix(target_tokens, || {
                primitives::any_phrase(&[&["and", "each", "player"], &["and", "each", "players"]])
            })
        && and_idx >= 1
        && parser_token_word_refs(after_phrase).is_empty()
    {
        return Ok(CombatDamageTargetShape::EachObjectsAndPlayer {
            filter_tokens: &target_tokens[1..and_idx],
        });
    }

    if primitives::parse_prefix(
        target_tokens,
        primitives::phrase(&["each", "opponent", "and", "each"]),
    )
    .is_some()
        && one_of_words_occurs(target_tokens, &["creature"])
        && one_of_words_occurs(target_tokens, &["planeswalker"])
        && (phrase_occurs(target_tokens, &["they", "control"])
            || phrase_occurs(target_tokens, &["that", "player", "controls"]))
    {
        return Ok(CombatDamageTargetShape::OpponentAndControlledCreaturePlaneswalker);
    }

    if let Some((history_idx, (), after_history)) = primitives::find_prefix(target_tokens, || {
        primitives::phrase(&["it", "has", "dealt", "damage", "to", "this", "game"]).void()
    }) && parser_token_word_refs(after_history).is_empty()
    {
        let domains = trim_lexed_commas(&target_tokens[..history_idx]);
        if let Some((and_idx, (), _)) =
            primitives::find_prefix(domains, || primitives::kw("and").void())
        {
            let player_tokens = trim_lexed_commas(&domains[..and_idx]);
            let filter_tokens = trim_lexed_commas(&domains[and_idx + 1..]);
            if !filter_tokens.is_empty()
                && let Some(players) =
                    parse_combat_player_damage_target_shape_lexed(player_tokens, false)
            {
                return Ok(CombatDamageTargetShape::HistoricalDamageRecipients {
                    players,
                    filter_tokens,
                });
            }
        }
    }

    if let Some((_head, filter_tokens)) = each_or_all {
        if filter_tokens.is_empty() {
            return Err(CombatDamageTargetShapeError::MissingEachFilter);
        }
        return Ok(CombatDamageTargetShape::EachFilter { filter_tokens });
    }

    if let Some((at_idx, (), _after_at)) =
        primitives::find_prefix(target_tokens, || primitives::kw("at").void())
        && at_idx >= 1
        && exact_phrase(
            &target_tokens[at_idx..],
            &[
                &["at", "end", "of", "combat"],
                &["at", "the", "end", "of", "combat"],
            ],
        )
    {
        let target_tokens = trim_lexed_commas(&target_tokens[..at_idx]);
        if !target_tokens.is_empty() {
            return Ok(CombatDamageTargetShape::DelayedEndOfCombat { target_tokens });
        }
    }

    Ok(CombatDamageTargetShape::General { target_tokens })
}

#[cfg(test)]
mod tests {
    use crate::runtime_backend::lexer::lex_line;

    use super::*;

    #[test]
    fn parses_damage_head_and_target_shapes() {
        let tokens = lex_line(
            "Deals damage to each opponent equal to the number of cards in their hand",
            0,
        )
        .unwrap();
        let shape = parse_combat_damage_head_shape_lexed(&tokens);
        assert!(shape.direct_hand_size_each_opponent);
        assert!(!shape.divided);

        let tokens = lex_line("2 damage to each other player.", 0).unwrap();
        assert!(matches!(
            parse_combat_damage_target_shape_lexed(&tokens, 1),
            Ok(CombatDamageTargetShape::PlayerGroup(
                CombatPlayerDamageTargetShape::EachOtherPlayer
            ))
        ));

        let tokens = lex_line("each other opponent", 0).unwrap();
        assert_eq!(
            parse_combat_player_damage_target_shape_lexed(&tokens, false),
            Some(CombatPlayerDamageTargetShape::EachOtherOpponent)
        );
        let tokens = lex_line("each other player", 0).unwrap();
        assert_eq!(
            parse_combat_player_damage_target_shape_lexed(&tokens, false),
            Some(CombatPlayerDamageTargetShape::EachOtherPlayer)
        );

        let tokens = lex_line(
            "divided as its controller chooses among any number of those Wolves",
            0,
        )
        .unwrap();
        let shape = parse_combat_divided_target_shape_lexed(&tokens).unwrap();
        assert!(shape.count.is_any_number());
        assert_eq!(
            parser_token_word_refs(shape.target_tokens),
            ["those", "wolves"]
        );
    }

    #[test]
    fn parses_trailing_unless_before_the_damage_target_fallback() {
        for text in [
            "4 damage to that player unless they control a commander",
            "2 damage to that player unless they control two or more basic lands",
            "2 damage to that player unless they have exactly three or exactly four cards in hand",
        ] {
            let tokens = lex_line(text, 0).unwrap();
            let shape = parse_combat_damage_target_shape_lexed(&tokens, 1).unwrap();
            let CombatDamageTargetShape::TrailingUnless {
                target_tokens,
                predicate,
            } = shape
            else {
                panic!("expected trailing-unless shape for {text}");
            };
            assert_eq!(parser_token_word_refs(target_tokens), ["that", "player"]);
            let predicate_debug = format!("{predicate:?}");
            assert!(
                predicate_debug.contains("Player") || predicate_debug.contains("ValueComparison"),
                "unexpected predicate for {text}: {predicate_debug}"
            );
        }
    }

    #[test]
    fn parses_player_object_union_with_full_game_source_damage_history() {
        let tokens = lex_line(
            "1 damage to each opponent and planeswalker it has dealt damage to this game",
            0,
        )
        .unwrap();
        let shape = parse_combat_damage_target_shape_lexed(&tokens, 1).unwrap();
        let CombatDamageTargetShape::HistoricalDamageRecipients {
            players,
            filter_tokens,
        } = shape
        else {
            panic!("expected historical mixed-recipient shape");
        };
        assert_eq!(players, CombatPlayerDamageTargetShape::EachOpponent);
        assert_eq!(parser_token_word_refs(filter_tokens), ["planeswalker"]);

        let near_miss = lex_line(
            "1 damage to each opponent and planeswalker it has dealt damage to this turn",
            0,
        )
        .unwrap();
        assert!(!matches!(
            parse_combat_damage_target_shape_lexed(&near_miss, 1),
            Ok(CombatDamageTargetShape::HistoricalDamageRecipients { .. })
        ));
    }

    #[test]
    fn parses_damage_pronouns_as_the_bound_event_player() {
        for text in ["the player", "that player", "them"] {
            let tokens = lex_line(text, 0).unwrap();
            assert_eq!(
                parse_combat_simple_damage_target_shape_lexed(&tokens),
                Some(CombatSimpleDamageTargetShape::IteratedPlayer),
                "damage recipient should use the typed event-player binding: {text}"
            );
        }
    }

    #[test]
    fn recognizes_spell_target_inside_controller_recipient() {
        for text in ["target spell's controller", "target spells controller"] {
            let tokens = lex_line(text, 0).unwrap();
            assert_eq!(
                parse_combat_embedded_target_controller_shape_lexed(&tokens),
                Some(CombatEmbeddedTargetControllerShape::Spell),
                "{text}"
            );
        }
        let tokens = lex_line("that spell's controller", 0).unwrap();
        assert_eq!(
            parse_combat_embedded_target_controller_shape_lexed(&tokens),
            None
        );
    }

    #[test]
    fn distinguishes_even_rounded_down_from_chosen_distribution() {
        let evenly = lex_line(
            "damage divided evenly, rounded down, among any number of targets",
            0,
        )
        .unwrap();
        assert!(matches!(
            parse_combat_divided_amount_shape_lexed(&evenly, 0).unwrap(),
            CombatDividedAmountShape::Distributed {
                evenly_rounded_down: true,
                ..
            }
        ));

        let chosen = lex_line(
            "damage divided as you choose among any number of targets",
            0,
        )
        .unwrap();
        assert!(matches!(
            parse_combat_divided_amount_shape_lexed(&chosen, 0).unwrap(),
            CombatDividedAmountShape::Distributed {
                evenly_rounded_down: false,
                ..
            }
        ));
    }
}
