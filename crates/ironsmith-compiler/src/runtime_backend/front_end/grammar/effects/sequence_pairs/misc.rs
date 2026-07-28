use std::ops::Range;

use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use crate::runtime_backend::front_end::lexer::{LexStream, OwnedLexToken, TokenKind};

use super::super::super::primitives;
use super::{
    contains_sequence_phrase, contains_sequence_word, ends_content_sequence, finish_sequence_words,
    matches_complete_content_sequence, matches_complete_sequence, same_words_without_articles,
    seek_sequence_phrase, sequence_any_phrase, sequence_phrase, starts_content_sequence,
    starts_sequence,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DirectionalAdjacentPlayerControlShape {
    pub(crate) choice_object: Range<usize>,
    pub(crate) gained_object: Range<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SameControllerSacrificeShape {
    pub(crate) target: Range<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct GraveyardCastReplacementShape {
    pub(crate) until_end_of_turn: bool,
    pub(crate) without_paying_mana_cost: bool,
    pub(crate) includes_artifact: bool,
    pub(crate) artifact_first: bool,
    pub(crate) mana_value_limit: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConditionalSelfAnimateTail {
    pub(crate) effect: Range<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ReturnTaggedBattlefieldShape {
    pub(crate) tapped: bool,
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

pub(crate) fn parse_directional_adjacent_player_control_shape(
    choice: &[OwnedLexToken],
    gain: &[OwnedLexToken],
) -> Option<DirectionalAdjacentPlayerControlShape> {
    let choice_object = primitives::parse_all(
        choice,
        |input: &mut LexStream<'_>| parse_between(input, CHOICE_PREFIX, CHOICE_SUFFIX),
        "directional-control-choice",
    )
    .ok()?;
    let gained_object = primitives::parse_all(
        gain,
        |input: &mut LexStream<'_>| parse_between(input, GAIN_PREFIX, GAIN_SUFFIX),
        "directional-control-gain",
    )
    .ok()?;
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

pub(crate) fn parse_choose_then_skip_phase_shape(
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

pub(crate) fn parse_same_controller_sacrifice_shape(
    choose: &[OwnedLexToken],
    sacrifice: &[OwnedLexToken],
) -> Option<SameControllerSacrificeShape> {
    if !matches_complete_content_sequence(sacrifice, SACRIFICE_ONE) {
        return None;
    }
    let target = primitives::parse_all(
        choose,
        parse_same_controller_target,
        "same-controller-sacrifice-target",
    )
    .ok()?;
    Some(SameControllerSacrificeShape { target })
}

pub(crate) fn is_return_other_to_owner_hand_shape(tokens: &[OwnedLexToken]) -> bool {
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

pub(crate) fn is_filtered_future_exile_return_next_end_step_shape(
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
    word.parse::<i32>()
        .ok()
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
    seek_sequence_phrase(&mut input, &[&["mana", "value"]]).ok()?;
    sequence_phrase(&["mana", "value"])
        .parse_next(&mut input)
        .ok()?;
    let limit = parse_fixed_limit_word(&mut input).ok()?;
    sequence_any_phrase(MANA_COMPARISONS)
        .parse_next(&mut input)
        .ok()?;
    Some(limit)
}

pub(crate) fn parse_graveyard_cast_replacement_shape(
    cast: &[OwnedLexToken],
    replacement: &[OwnedLexToken],
) -> Option<GraveyardCastReplacementShape> {
    let (cast, until_end_of_turn) = if let Some(rest) =
        primitives::strip_lexed_prefix_phrase(cast, &["until", "end", "of", "turn"])
    {
        (rest, true)
    } else if let Some(rest) =
        primitives::strip_lexed_prefix_phrase(cast, &["until", "the", "end", "of", "turn"])
    {
        (rest, true)
    } else {
        (cast, false)
    };
    if !starts_sequence(cast, CAST_PREFIX)
        || !contains_sequence_phrase(cast, CAST_FROM_GRAVEYARD)
        || !(contains_sequence_word(cast, "instant") || contains_sequence_word(cast, "sorcery"))
        || !contains_sequence_word(cast, "card")
        || !matches_complete_sequence(
            replacement,
            &[
                THAT_SPELL_YOUR_GRAVEYARD_REPLACEMENT,
                THAT_SPELL_A_GRAVEYARD_REPLACEMENT,
                CAST_THIS_WAY_YOUR_GRAVEYARD_REPLACEMENT,
                CAST_THIS_WAY_A_GRAVEYARD_REPLACEMENT,
            ],
        )
    {
        return None;
    }
    Some(GraveyardCastReplacementShape {
        until_end_of_turn,
        without_paying_mana_cost: contains_sequence_phrase(cast, WITHOUT_MANA),
        includes_artifact: contains_sequence_word(cast, "artifact"),
        artifact_first: cast
            .iter()
            .position(|token| token.is_word("artifact"))
            .zip(cast.iter().position(|token| token.is_word("instant")))
            .is_some_and(|(artifact, instant)| artifact < instant),
        mana_value_limit: mana_value_limit(cast),
    })
}

pub(crate) fn has_life_gain_surface(tokens: &[OwnedLexToken]) -> bool {
    contains_sequence_word(tokens, "life")
        && (contains_sequence_word(tokens, "gain") || contains_sequence_word(tokens, "gains"))
}

fn parse_conditional_self_animate<'a>(
    input: &mut LexStream<'a>,
) -> WResult<ConditionalSelfAnimateTail> {
    let initial_len = input.len();
    sequence_phrase(&["if", "this"]).parse_next(input)?;
    let mut comma_at = None;
    let mut saw_isnt = false;
    let mut saw_creature = false;
    while !input.is_empty() {
        let offset = initial_len.saturating_sub(input.len());
        let token: &'a OwnedLexToken = any.parse_next(input)?;
        if token.kind == TokenKind::Comma {
            comma_at = Some(offset);
            break;
        }
        saw_isnt |= token.is_word("isnt");
        saw_creature |= token.is_word("creature");
    }
    let _comma_at = comma_at.ok_or_else(|| {
        primitives::backtrack_err("conditional self animation", "condition comma")
    })?;
    if !saw_isnt || !saw_creature {
        return Err(primitives::backtrack_err(
            "conditional self animation",
            "isn't a creature condition",
        ));
    }
    let effect_start = initial_len.saturating_sub(input.len());
    let mut tail_probe = input.clone();
    sequence_phrase(&["it"]).parse_next(&mut tail_probe)?;
    Ok(ConditionalSelfAnimateTail {
        effect: effect_start..initial_len,
    })
}

pub(crate) fn parse_conditional_self_animate_tail(
    tokens: &[OwnedLexToken],
) -> Option<ConditionalSelfAnimateTail> {
    primitives::parse_prefix(tokens, parse_conditional_self_animate).map(|(shape, _)| shape)
}

fn filtered_return_phrase(
    tokens: &[OwnedLexToken],
    expected: &'static [&'static str],
) -> Option<bool> {
    let mut input = LexStream::new(tokens);
    let mut tapped = false;
    for expected_word in expected {
        loop {
            let token = super::next_word(&mut input).ok()?;
            let word = token.parser_text();
            if matches!(word, "a" | "an" | "the") {
                continue;
            }
            if word == "tapped" {
                tapped = true;
                continue;
            }
            if word != *expected_word {
                return None;
            }
            break;
        }
    }
    while let Ok(token) = super::next_word(&mut input) {
        if token.is_word("tapped") {
            tapped = true;
        } else if !matches!(token.parser_text(), "a" | "an" | "the") {
            return None;
        }
    }
    Some(tapped)
}

pub(crate) fn parse_return_tagged_battlefield_shape(
    tokens: &[OwnedLexToken],
) -> Option<ReturnTaggedBattlefieldShape> {
    let tapped = filtered_return_phrase(tokens, &["return", "those", "cards", "to", "battlefield"])
        .or_else(|| filtered_return_phrase(tokens, &["return", "them", "to", "battlefield"]))?;
    Some(ReturnTaggedBattlefieldShape { tapped })
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

pub(crate) fn is_delayed_dies_exile_play_shape(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
) -> bool {
    if !starts_sequence(first, DELAYED_DIES) {
        return false;
    }
    let mut input = LexStream::new(first);
    let initial_len = input.len();
    let mut action_start = None;
    while !input.is_empty() {
        let parsed: WResult<&OwnedLexToken> = any.parse_next(&mut input);
        let token = match parsed {
            Ok(token) => token,
            Err(_) => return false,
        };
        if token.kind == TokenKind::Comma {
            action_start = Some(initial_len.saturating_sub(input.len()));
            break;
        }
    }
    let Some(action_start) = action_start else {
        return false;
    };
    let action = &first[action_start..];
    starts_content_sequence(action, EXILE_TOP_POWER)
        && ends_content_sequence(action, CHOOSE_EXILED)
        && matches_complete_content_sequence(second, PLAY_NEXT_TURN)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::lex_line;

    fn lex(raw: &str) -> Vec<OwnedLexToken> {
        lex_line(raw, 0).unwrap()
    }

    #[test]
    fn parses_directional_and_phase_pair_shapes() {
        let directional = parse_directional_adjacent_player_control_shape(
            &lex("Starting with you and proceeding in the chosen direction, each player chooses a creature controlled by the next player in that direction"),
            &lex("Each player gains control of the creature they chose"),
        )
        .unwrap();
        assert!(!directional.choice_object.is_empty());
        assert!(parse_choose_then_skip_phase_shape(
            &lex("That player chooses draw step, main phase, or combat phase"),
            &lex("That player skips each instance of the chosen step or phase this turn"),
        ));
    }

    #[test]
    fn parses_graveyard_cast_and_return_shapes() {
        let shape = parse_graveyard_cast_replacement_shape(
            &lex("You may cast target artifact, instant, or sorcery card with mana value three or less from your graveyard without paying its mana cost"),
            &lex("If that spell would be put into your graveyard, exile it instead"),
        )
        .unwrap();
        assert_eq!(shape.mana_value_limit, Some(3));
        assert!(shape.includes_artifact);
        assert!(shape.artifact_first);
        assert!(shape.without_paying_mana_cost);
        assert!(!shape.until_end_of_turn);
        let duration = parse_graveyard_cast_replacement_shape(
            &lex(
                "Until end of turn, you may cast target instant or sorcery card from your graveyard without paying its mana cost",
            ),
            &lex("If that spell would be put into your graveyard, exile it instead"),
        )
        .unwrap();
        assert!(duration.until_end_of_turn);
        assert!(
            parse_graveyard_cast_replacement_shape(
                &lex(
                    "You may cast target instant card from your graveyard without paying its mana cost"
                ),
                &lex("If that spell would be put into a graveyard, exile it instead"),
            )
            .is_some()
        );
        assert!(
            parse_graveyard_cast_replacement_shape(
                &lex(
                    "You may cast target instant or sorcery card from a graveyard without paying its mana cost"
                ),
                &lex("If that spell would be put into a graveyard, exile it instead"),
            )
            .is_some()
        );
        assert_eq!(
            parse_return_tagged_battlefield_shape(&lex(
                "Return those cards to the battlefield tapped"
            )),
            Some(ReturnTaggedBattlefieldShape { tapped: true })
        );
        assert!(is_filtered_future_exile_return_next_end_step_shape(
            &lex(
                "If a permanent you control would be put into a graveyard from the battlefield this turn, exile it instead"
            ),
            &lex(
                "Return it to the battlefield under its owner's control at the beginning of the next end step"
            ),
        ));
        assert!(is_filtered_future_exile_return_next_end_step_shape(
            &lex(
                "If a permanent you control would be put into a graveyard from the battlefield this turn, exile it instead"
            ),
            &lex(
                "At the beginning of the next end step, return it to the battlefield under its owner's control"
            ),
        ));
    }
}
