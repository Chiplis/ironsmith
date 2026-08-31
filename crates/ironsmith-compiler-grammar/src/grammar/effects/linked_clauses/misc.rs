use std::ops::Range;

use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use crate::lexer::{LexStream, OwnedLexToken, TokenKind};

use super::super::super::primitives;
use super::{
    contains_sequence_phrase, contains_sequence_word, ends_content_sequence, finish_sequence_words,
    matches_complete_content_sequence, matches_complete_sequence, same_words_without_articles,
    seek_sequence_phrase, sequence_any_phrase, sequence_phrase, starts_content_sequence,
    starts_sequence,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectionalAdjacentPlayerControlShape {
    pub choice_object: Range<usize>,
    pub gained_object: Range<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SameControllerSacrificeShape {
    pub target: Range<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraveyardCastReplacementShape {
    pub until_end_of_turn: bool,
    pub without_paying_mana_cost: bool,
    pub includes_artifact: bool,
    pub artifact_first: bool,
    pub mana_value_limit: Option<i32>,
    pub additional_mana_cost: Option<crate::mana::ManaCost>,
    pub mana_spend_mode: ironsmith_core::value_model::ManaSpendMode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConditionalSelfAnimateTail {
    pub effect: Range<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReturnTaggedBattlefieldShape {
    pub tapped: bool,
}

const EXILE_RESOLVING_CARD_INSTEAD: &[&str] = &[
    "exile",
    "that",
    "card",
    "instead",
    "of",
    "putting",
    "it",
    "into",
    "your",
    "graveyard",
    "as",
    "it",
    "resolves",
];
const IF_YOU_DO_RETURN_TO_HAND_NEXT_END_STEP: &[&str] = &[
    "if",
    "you",
    "do",
    "return",
    "it",
    "to",
    "your",
    "hand",
    "at",
    "the",
    "beginning",
    "of",
    "the",
    "next",
    "end",
    "step",
];

pub fn is_resolving_card_exile_then_return_next_end_step_shape(
    replacement: &[OwnedLexToken],
    delayed_return: &[OwnedLexToken],
) -> bool {
    matches_complete_sequence(replacement, &[EXILE_RESOLVING_CARD_INSTEAD])
        && matches_complete_sequence(delayed_return, &[IF_YOU_DO_RETURN_TO_HAND_NEXT_END_STEP])
}

const CHOICE_PREFIX: &[&str] = &[
    "starting",
    "with",
    "you",
    "and",
    "proceeding",
    "in",
    "the",
    "chosen",
    "direction",
    "each",
    "player",
    "chooses",
];
const CHOICE_SUFFIX: &[&str] = &[
    "controlled",
    "by",
    "the",
    "next",
    "player",
    "in",
    "that",
    "direction",
];
const GAIN_PREFIX: &[&str] = &["each", "player", "gains", "control", "of"];
const GAIN_SUFFIX: &[&str] = &["they", "chose"];

fn parse_between<'a>(
    input: &mut LexStream<'a>,
    prefix: &'static [&'static str],
    suffix: &'static [&'static str],
) -> WResult<Range<usize>> {
    let initial_len = input.len();
    sequence_phrase(prefix).parse_next(input)?;
    let start = initial_len.saturating_sub(input.len());
    seek_sequence_phrase(input, &[suffix])?;
    let end = initial_len.saturating_sub(input.len());
    sequence_phrase(suffix).parse_next(input)?;
    finish_sequence_words(input)?;
    Ok(start..end)
}

pub fn parse_directional_adjacent_player_control_shape(
    choice: &[OwnedLexToken],
    gain: &[OwnedLexToken],
) -> Option<DirectionalAdjacentPlayerControlShape> {
    let choice_object = crate::grammar::primitives::probe_all(
        choice,
        |input: &mut LexStream<'_>| parse_between(input, CHOICE_PREFIX, CHOICE_SUFFIX),
        "directional-control-choice",
    )?;
    let gained_object = crate::grammar::primitives::probe_all(
        gain,
        |input: &mut LexStream<'_>| parse_between(input, GAIN_PREFIX, GAIN_SUFFIX),
        "directional-control-gain",
    )?;
    if !same_words_without_articles(&choice[choice_object.clone()], &gain[gained_object.clone()]) {
        return None;
    }
    Some(DirectionalAdjacentPlayerControlShape {
        choice_object,
        gained_object,
    })
}

const CHOOSE_PHASES: &[&[&str]] = &[
    &[
        "that", "player", "chooses", "draw", "step", "main", "phase", "or", "combat", "phase",
    ],
    &[
        "that", "player", "choose", "draw", "step", "main", "phase", "or", "combat", "phase",
    ],
    &[
        "the", "player", "chooses", "draw", "step", "main", "phase", "or", "combat", "phase",
    ],
    &[
        "the", "player", "choose", "draw", "step", "main", "phase", "or", "combat", "phase",
    ],
];
const SKIP_PHASES: &[&[&str]] = &[
    &[
        "that", "player", "skips", "each", "instance", "of", "the", "chosen", "step", "or",
        "phase", "this", "turn",
    ],
    &[
        "that", "player", "skip", "each", "instance", "of", "the", "chosen", "step", "or", "phase",
        "this", "turn",
    ],
    &[
        "the", "player", "skips", "each", "instance", "of", "the", "chosen", "step", "or", "phase",
        "this", "turn",
    ],
    &[
        "the", "player", "skip", "each", "instance", "of", "the", "chosen", "step", "or", "phase",
        "this", "turn",
    ],
];

pub fn parse_choose_then_skip_phase_shape(
    choose: &[OwnedLexToken],
    skip: &[OwnedLexToken],
) -> bool {
    matches_complete_sequence(choose, CHOOSE_PHASES) && matches_complete_sequence(skip, SKIP_PHASES)
}

const SAME_CONTROLLER_SUFFIXES: &[&[&str]] = &[
    &["controlled", "by", "the", "same", "player"],
    &["controlled", "by", "same", "player"],
];
const SACRIFICE_ONE: &[&[&str]] = &[
    &[
        "their",
        "controller",
        "chooses",
        "and",
        "sacrifices",
        "one",
        "of",
        "them",
    ],
    &[
        "their",
        "controller",
        "choose",
        "and",
        "sacrifice",
        "one",
        "of",
        "them",
    ],
    &[
        "its",
        "controller",
        "chooses",
        "and",
        "sacrifices",
        "one",
        "of",
        "them",
    ],
    &[
        "that",
        "player",
        "sacrifices",
        "one",
        "of",
        "them",
        "of",
        "their",
        "choice",
    ],
    &["that", "player", "sacrifices", "one", "of", "them"],
    &[
        "that",
        "player",
        "sacrifice",
        "one",
        "of",
        "them",
        "of",
        "their",
        "choice",
    ],
];
const RETURN_OTHER: &[&[&str]] = &[
    &["return", "other", "to", "its", "owners", "hand"],
    &["return", "other", "to", "its", "owner's", "hand"],
    &["return", "other", "to", "its", "owner", "hand"],
];

fn parse_same_controller_target<'a>(input: &mut LexStream<'a>) -> WResult<Range<usize>> {
    let initial_len = input.len();
    sequence_phrase(&["choose"]).parse_next(input)?;
    let start = initial_len.saturating_sub(input.len());
    seek_sequence_phrase(input, SAME_CONTROLLER_SUFFIXES)?;
    let end = initial_len.saturating_sub(input.len());
    sequence_any_phrase(SAME_CONTROLLER_SUFFIXES).parse_next(input)?;
    finish_sequence_words(input)?;
    if start >= end {
        return Err(primitives::backtrack_err(
            "same-controller target",
            "target phrase",
        ));
    }
    Ok(start..end)
}

pub fn parse_same_controller_sacrifice_shape(
    choose: &[OwnedLexToken],
    sacrifice: &[OwnedLexToken],
) -> Option<SameControllerSacrificeShape> {
    if !matches_complete_content_sequence(sacrifice, SACRIFICE_ONE) {
        return None;
    }
    let target = crate::grammar::primitives::probe_all(
        choose,
        parse_same_controller_target,
        "same-controller-sacrifice-target",
    )?;
    Some(SameControllerSacrificeShape { target })
}

pub fn is_return_other_to_owner_hand_shape(tokens: &[OwnedLexToken]) -> bool {
    matches_complete_content_sequence(tokens, RETURN_OTHER)
}

const CAST_PREFIX: &[&[&str]] = &[&["you", "may", "cast", "target"]];
const CAST_FROM_GRAVEYARD: &[&[&str]] =
    &[&["from", "your", "graveyard"], &["from", "a", "graveyard"]];
const WITHOUT_MANA: &[&[&str]] = &[&["without", "paying", "its", "mana", "cost"]];
const THAT_SPELL_YOUR_GRAVEYARD_REPLACEMENT: &[&str] = &[
    "if",
    "that",
    "spell",
    "would",
    "be",
    "put",
    "into",
    "your",
    "graveyard",
    "exile",
    "it",
    "instead",
];
const THAT_SPELL_A_GRAVEYARD_REPLACEMENT: &[&str] = &[
    "if",
    "that",
    "spell",
    "would",
    "be",
    "put",
    "into",
    "a",
    "graveyard",
    "exile",
    "it",
    "instead",
];
const CAST_THIS_WAY_YOUR_GRAVEYARD_REPLACEMENT: &[&str] = &[
    "if",
    "an",
    "instant",
    "or",
    "sorcery",
    "spell",
    "cast",
    "this",
    "way",
    "would",
    "be",
    "put",
    "into",
    "your",
    "graveyard",
    "exile",
    "it",
    "instead",
];
const CAST_THIS_WAY_A_GRAVEYARD_REPLACEMENT: &[&str] = &[
    "if",
    "an",
    "instant",
    "or",
    "sorcery",
    "spell",
    "cast",
    "this",
    "way",
    "would",
    "be",
    "put",
    "into",
    "a",
    "graveyard",
    "exile",
    "it",
    "instead",
];
const MANA_COMPARISONS: &[&[&str]] = &[
    &["or", "less"],
    &["or", "less", "than", "or", "equal"],
    &["or", "less", "than", "or", "equal", "to"],
    &["less", "than", "or", "equal"],
    &["less", "than", "or", "equal", "to"],
];

const FILTERED_FUTURE_EXILE_THIS_TURN: &[&str] = &[
    "if",
    "a",
    "permanent",
    "you",
    "control",
    "would",
    "be",
    "put",
    "into",
    "a",
    "graveyard",
    "from",
    "the",
    "battlefield",
    "this",
    "turn",
    "exile",
    "it",
    "instead",
];
const RETURN_LINKED_AT_NEXT_END_STEP: &[&str] = &[
    "return",
    "it",
    "to",
    "the",
    "battlefield",
    "under",
    "its",
    "owner's",
    "control",
    "at",
    "the",
    "beginning",
    "of",
    "the",
    "next",
    "end",
    "step",
];
const AT_NEXT_END_STEP_RETURN_LINKED: &[&str] = &[
    "at",
    "the",
    "beginning",
    "of",
    "the",
    "next",
    "end",
    "step",
    "return",
    "it",
    "to",
    "the",
    "battlefield",
    "under",
    "its",
    "owner's",
    "control",
];

pub fn is_filtered_future_exile_return_next_end_step_shape(
    replacement: &[OwnedLexToken],
    delayed_return: &[OwnedLexToken],
) -> bool {
    matches_complete_sequence(replacement, &[FILTERED_FUTURE_EXILE_THIS_TURN])
        && matches_complete_sequence(
            delayed_return,
            &[
                RETURN_LINKED_AT_NEXT_END_STEP,
                AT_NEXT_END_STEP_RETURN_LINKED,
            ],
        )
}

fn parse_fixed_limit_word(input: &mut LexStream<'_>) -> WResult<i32> {
    let token = super::next_word(input)?;
    let word = token.parser_text();
    crate::util::decimal_amount(word)
        .or_else(|| {
            Some(match word {
                "zero" => 0,
                "one" => 1,
                "two" => 2,
                "three" => 3,
                "four" => 4,
                "five" => 5,
                "six" => 6,
                "seven" => 7,
                "eight" => 8,
                "nine" => 9,
                "ten" => 10,
                _ => return None,
            })
        })
        .ok_or_else(|| primitives::backtrack_err("mana value limit", "fixed number"))
}

fn mana_value_limit(tokens: &[OwnedLexToken]) -> Option<i32> {
    let mut input = LexStream::new(tokens);
    crate::grammar::primitives::take_leaf(&mut input, |input: &mut _| {
        seek_sequence_phrase(input, &[&["mana", "value"]])
    })?;
    crate::grammar::primitives::take_leaf(&mut input, sequence_phrase(&["mana", "value"]))?;
    let limit = crate::grammar::primitives::take_leaf(&mut input, parse_fixed_limit_word)?;
    crate::grammar::primitives::take_leaf(&mut input, sequence_any_phrase(MANA_COMPARISONS))?;
    Some(limit)
}

fn additional_cast_mana_cost(
    tokens: &[OwnedLexToken],
) -> Result<Option<crate::mana::ManaCost>, ()> {
    let Some(by_paying) = crate::slice_primitives::find_window_by(tokens, 2, |pair| {
        pair[0].is_word("by") && pair[1].is_word("paying")
    }) else {
        return Ok(None);
    };
    let cost_start = by_paying + 2;
    let Some(addition) = crate::slice_primitives::select_position(&tokens[cost_start..], |token| {
        token.is_word("in")
    })
    .map(|offset| cost_start + offset) else {
        return Err(());
    };
    if !matches_complete_content_sequence(
        &tokens[addition..],
        &[&["in", "addition", "to", "its", "other", "costs"]],
    ) {
        return Err(());
    }
    crate::grammar::leaf::parse_leaf_mana_cost_tokens(&tokens[cost_start..addition])
        .map(Some)
        .map_err(|_| ())
}

const DELAYED_DIES: &[&[&str]] = &[&["when", "that", "creature", "dies", "this", "turn"]];
const EXILE_TOP_POWER: &[&[&str]] = &[&[
    "exile", "number", "of", "cards", "from", "top", "of", "your", "library", "equal", "to", "its",
    "power",
]];
const CHOOSE_EXILED: &[&[&str]] = &[&["choose", "card", "exiled", "this", "way"]];
const PLAY_NEXT_TURN: &[&[&str]] = &[&[
    "until", "end", "of", "your", "next", "turn", "you", "may", "play", "that", "card",
]];

#[cfg(test)]
#[path = "misc_inline_tests.rs"]
mod tests;

#[path = "misc/trigger.rs"]
mod trigger_programs;
pub use trigger_programs::is_delayed_dies_exile_play_shape;
#[path = "misc/reference.rs"]
mod reference_programs;
use reference_programs::filtered_return_phrase;
pub use reference_programs::parse_return_tagged_battlefield_shape;
#[path = "misc/condition.rs"]
mod condition_programs;
use condition_programs::parse_conditional_self_animate;
pub use condition_programs::{
    parse_conditional_self_animate_tail, parse_graveyard_cast_replacement_shape,
};
#[path = "misc/resource.rs"]
mod resource_programs;
pub use resource_programs::has_life_gain_surface;
