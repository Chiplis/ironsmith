use super::*;
use crate::runtime_backend::sentences::effect_sentences::clause_pattern_helpers::{
    ClauseShape, clause_shape,
};

pub(super) type GrammarFilterNormalizedWords<'a> = TokenWordView<'a>;

pub(super) fn push_unique_filter_value<T: Copy + PartialEq>(items: &mut Vec<T>, value: T) {
    crate::slice_primitives::push_unique(items, value);
}

#[derive(Clone, Copy)]
pub(super) enum SpellFilterComparisonAxis {
    Power,
    Toughness,
    ManaValue,
}

#[derive(Clone, Copy)]
pub(super) enum PlayerRelationVerb {
    Cast,
    Control,
    Own,
}

#[derive(Clone, Copy)]
pub(super) struct SegmentPhraseVariant {
    words: &'static [&'static str],
    drain_start_offset: usize,
}

const POWER_AXIS_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["power"]);
const TOUGHNESS_AXIS_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["toughness"]);
const MANA_VALUE_AXIS_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["mana", "value"]);
const CAST_RELATION_VERB_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["cast"], &["casts"]]);
const CONTROL_RELATION_VERB_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["control"], &["controls"]]);
const OWN_RELATION_VERB_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["own"], &["owns"]]);
const YOU_RELATION_SUBJECT_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["you"]);
const OPPONENT_RELATION_SUBJECT_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["opponent"], &["opponents"]]);
const THEY_RELATION_SUBJECT_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["they"]);
const YOUR_TEAM_RELATION_SUBJECT_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["your", "team"]);
const YOUR_OPPONENTS_RELATION_SUBJECT_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["your", "opponents"]);
const THAT_PLAYER_RELATION_SUBJECT_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["that", "player"]);
const TARGET_PLAYER_RELATION_SUBJECT_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["target", "player"]);
const TARGET_OPPONENT_RELATION_SUBJECT_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["target", "opponent"]);
const DEFENDING_PLAYER_RELATION_SUBJECT_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["defending", "player"]);
const ATTACKING_PLAYER_RELATION_SUBJECT_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["attacking", "player"]);
const TARGET_CONTROLLER_RELATION_SUBJECT_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["its", "controller"],
            &["its", "controllers"],
            &["their", "controller"],
            &["their", "controllers"],
        ]
);
const DONT_CONTROL_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["dont", "control"],
            &["dont", "controls"],
            &["don't", "control"],
            &["don't", "controls"],
        ]
);
const DONT_OWN_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["dont", "own"],
            &["dont", "owns"],
            &["don't", "own"],
            &["don't", "owns"],
        ]
);
const DO_NOT_CONTROL_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["do", "not", "control"], &["do", "not", "controls"]]);
const DO_NOT_OWN_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["do", "not", "own"], &["do", "not", "owns"]]);
const YOU_DONT_CONTROL_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["you", "dont", "control"],
            &["you", "dont", "controls"],
            &["you", "don't", "control"],
            &["you", "don't", "controls"],
        ]
);
const YOU_DONT_OWN_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["you", "dont", "own"],
            &["you", "dont", "owns"],
            &["you", "don't", "own"],
            &["you", "don't", "owns"],
        ]
);
const YOU_DO_NOT_CONTROL_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["you", "do", "not", "control"],
            &["you", "do", "not", "controls"],
        ]
);
const YOU_DO_NOT_OWN_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["you", "do", "not", "own"], &["you", "do", "not", "owns"]]);
const CHOSEN_PLAYER_GRAVEYARD_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["chosen", "player", "graveyard"],
            &["chosen", "players", "graveyard"]
        ]
);
const THE_CHOSEN_PLAYER_GRAVEYARD_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["the", "chosen", "player", "graveyard"],
            &["the", "chosen", "players", "graveyard"],
        ]
);
const BOTH_OWN_AND_CONTROL_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["both", "own", "and", "control"],
            &["both", "owns", "and", "control"],
            &["both", "own", "and", "controls"],
            &["both", "owns", "and", "controls"],
            &["both", "control", "and", "own"],
            &["both", "controls", "and", "own"],
            &["both", "control", "and", "owns"],
            &["both", "controls", "and", "owns"],
        ]
);
const OWN_OR_CONTROL_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["own", "or", "control"],
            &["owns", "or", "control"],
            &["own", "or", "controls"],
            &["owns", "or", "controls"],
            &["control", "or", "own"],
            &["controls", "or", "own"],
            &["control", "or", "owns"],
            &["controls", "or", "owns"],
        ]
);
const PUT_THERE_FROM_BATTLEFIELD_THIS_TURN_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &[
                "that",
                "was",
                "put",
                "there",
                "from",
                "battlefield",
                "this",
                "turn"
            ],
            &[
                "that",
                "were",
                "put",
                "there",
                "from",
                "battlefield",
                "this",
                "turn"
            ]
        ]
);
const PUT_THERE_FROM_ANYWHERE_THIS_TURN_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &[
                "that", "was", "put", "there", "from", "anywhere", "this", "turn"
            ],
            &[
                "that", "were", "put", "there", "from", "anywhere", "this", "turn"
            ]
        ]
);
const GRAVEYARD_FROM_BATTLEFIELD_THIS_TURN_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["graveyard", "from", "battlefield", "this", "turn"],
            &["graveyards", "from", "battlefield", "this", "turn"]
        ]
);
const ENTERED_YOUR_CONTROL_THIS_TURN_LONG_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix
        & [
            "entered",
            "the",
            "battlefield",
            "under",
            "your",
            "control",
            "this",
            "turn",
        ]
);
const ENTERED_YOUR_CONTROL_THIS_TURN_MID_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix
        & [
            "entered",
            "battlefield",
            "under",
            "your",
            "control",
            "this",
            "turn",
        ]
);
const ENTERED_YOUR_CONTROL_THIS_TURN_SHORT_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["entered", "under", "your", "control", "this", "turn"]);
const ENTERED_OPPONENT_CONTROL_THIS_TURN_LONG_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &[
                "entered",
                "the",
                "battlefield",
                "under",
                "opponent",
                "control",
                "this",
                "turn",
            ],
            &[
                "entered",
                "the",
                "battlefield",
                "under",
                "opponents",
                "control",
                "this",
                "turn",
            ],
        ]
);
const ENTERED_OPPONENT_CONTROL_THIS_TURN_MID_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &[
                "entered",
                "battlefield",
                "under",
                "opponent",
                "control",
                "this",
                "turn",
            ],
            &[
                "entered",
                "battlefield",
                "under",
                "opponents",
                "control",
                "this",
                "turn",
            ],
        ]
);
const ENTERED_OPPONENT_CONTROL_THIS_TURN_SHORT_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["entered", "under", "opponent", "control", "this", "turn"],
            &["entered", "under", "opponents", "control", "this", "turn",],
        ]
);
const ENTERED_THIS_TURN_LONG_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["entered", "the", "battlefield", "this", "turn"]);
const ENTERED_THIS_TURN_MID_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["entered", "battlefield", "this", "turn"]);
const ENTERED_THIS_TURN_SHORT_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["entered", "this", "turn"]);
const DRAWN_THIS_TURN_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["drawn", "this", "turn"]);
const OTHER_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["other"]);
const LEADING_TAGGED_REFERENCE_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["that"], &["those"], &["chosen"]]);
const IT_OR_THEM_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["it"], &["them"]]);

fn shape_prefix_consumed(
    words: &[&str],
    shape: &ClauseShape<'static>,
    consumed: usize,
) -> Option<usize> {
    shape.matches_words(words).then_some(consumed)
}

impl SpellFilterComparisonAxis {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Power => "power",
            Self::Toughness => "toughness",
            Self::ManaValue => "mana value",
        }
    }

    pub(super) fn assign(self, filter: &mut ObjectFilter, comparison: crate::target::Comparison) {
        match self {
            Self::Power => filter.power = Some(comparison),
            Self::Toughness => filter.toughness = Some(comparison),
            Self::ManaValue => filter.mana_value = Some(comparison),
        }
    }
}

pub(super) fn parse_spell_filter_comparison_axis_words(
    words: &[&str],
) -> Option<(SpellFilterComparisonAxis, usize)> {
    shape_prefix_consumed(words, &POWER_AXIS_PREFIX_PATTERN, 1)
        .map(|consumed| (SpellFilterComparisonAxis::Power, consumed))
        .or_else(|| {
            shape_prefix_consumed(words, &TOUGHNESS_AXIS_PREFIX_PATTERN, 1)
                .map(|consumed| (SpellFilterComparisonAxis::Toughness, consumed))
        })
        .or_else(|| {
            shape_prefix_consumed(words, &MANA_VALUE_AXIS_PREFIX_PATTERN, 2)
                .map(|consumed| (SpellFilterComparisonAxis::ManaValue, consumed))
        })
}

pub(super) fn parse_player_relation_verb(words: &[&str]) -> Option<(PlayerRelationVerb, usize)> {
    shape_prefix_consumed(words, &CAST_RELATION_VERB_PREFIX_PATTERN, 1)
        .map(|consumed| (PlayerRelationVerb::Cast, consumed))
        .or_else(|| {
            shape_prefix_consumed(words, &CONTROL_RELATION_VERB_PREFIX_PATTERN, 1)
                .map(|consumed| (PlayerRelationVerb::Control, consumed))
        })
        .or_else(|| {
            shape_prefix_consumed(words, &OWN_RELATION_VERB_PREFIX_PATTERN, 1)
                .map(|consumed| (PlayerRelationVerb::Own, consumed))
        })
}

pub(super) fn parse_player_relation_subject(
    words: &[&str],
    pronoun_player_filter: &PlayerFilter,
) -> Option<(PlayerFilter, usize)> {
    if let Some(consumed) = shape_prefix_consumed(words, &YOU_RELATION_SUBJECT_PREFIX_PATTERN, 1) {
        return Some((PlayerFilter::You, consumed));
    }
    if let Some(consumed) =
        shape_prefix_consumed(words, &OPPONENT_RELATION_SUBJECT_PREFIX_PATTERN, 1)
    {
        return Some((PlayerFilter::Opponent, consumed));
    }
    if let Some(consumed) = shape_prefix_consumed(words, &THEY_RELATION_SUBJECT_PREFIX_PATTERN, 1) {
        return Some((pronoun_player_filter.clone(), consumed));
    }
    if let Some(consumed) =
        shape_prefix_consumed(words, &YOUR_TEAM_RELATION_SUBJECT_PREFIX_PATTERN, 2)
    {
        return Some((PlayerFilter::You, consumed));
    }
    if let Some(consumed) =
        shape_prefix_consumed(words, &YOUR_OPPONENTS_RELATION_SUBJECT_PREFIX_PATTERN, 2)
    {
        return Some((PlayerFilter::Opponent, consumed));
    }
    if let Some(consumed) =
        shape_prefix_consumed(words, &THAT_PLAYER_RELATION_SUBJECT_PREFIX_PATTERN, 2)
    {
        return Some((PlayerFilter::IteratedPlayer, consumed));
    }
    if let Some(consumed) =
        shape_prefix_consumed(words, &TARGET_PLAYER_RELATION_SUBJECT_PREFIX_PATTERN, 2)
    {
        return Some((PlayerFilter::target_player(), consumed));
    }
    if let Some(consumed) =
        shape_prefix_consumed(words, &TARGET_OPPONENT_RELATION_SUBJECT_PREFIX_PATTERN, 2)
    {
        return Some((PlayerFilter::target_opponent(), consumed));
    }
    if let Some(consumed) =
        shape_prefix_consumed(words, &DEFENDING_PLAYER_RELATION_SUBJECT_PREFIX_PATTERN, 2)
    {
        return Some((PlayerFilter::Defending, consumed));
    }
    if let Some(consumed) =
        shape_prefix_consumed(words, &ATTACKING_PLAYER_RELATION_SUBJECT_PREFIX_PATTERN, 2)
    {
        return Some((PlayerFilter::Attacking, consumed));
    }
    if let Some(consumed) =
        shape_prefix_consumed(words, &TARGET_CONTROLLER_RELATION_SUBJECT_PREFIX_PATTERN, 2)
    {
        return Some((
            PlayerFilter::ControllerOf(crate::filter::ObjectRef::Target),
            consumed,
        ));
    }

    None
}

pub(super) fn apply_player_relation(
    filter: &mut ObjectFilter,
    player: PlayerFilter,
    verb: PlayerRelationVerb,
) {
    match verb {
        PlayerRelationVerb::Cast => filter.cast_by = Some(player),
        PlayerRelationVerb::Control => filter.controller = Some(player),
        PlayerRelationVerb::Own => filter.owner = Some(player),
    }
}

pub(super) fn try_apply_player_relation_clause(
    filter: &mut ObjectFilter,
    words: &[&str],
    pronoun_player_filter: &PlayerFilter,
) -> Option<usize> {
    let (player, subject_consumed) = parse_player_relation_subject(words, pronoun_player_filter)?;
    let (verb, verb_consumed) = parse_player_relation_verb(&words[subject_consumed..])?;

    if matches!(player, PlayerFilter::Defending | PlayerFilter::Attacking)
        && !matches!(verb, PlayerRelationVerb::Control)
    {
        return None;
    }
    if matches!(player, PlayerFilter::ControllerOf(_))
        && !matches!(verb, PlayerRelationVerb::Control)
    {
        return None;
    }

    apply_player_relation(filter, player, verb);
    Some(subject_consumed + verb_consumed)
}

pub(super) fn try_apply_negated_you_relation_clause(
    filter: &mut ObjectFilter,
    words: &[&str],
) -> Option<usize> {
    if let Some(consumed) = shape_prefix_consumed(words, &DONT_CONTROL_PREFIX_PATTERN, 2) {
        filter.controller = Some(PlayerFilter::NotYou);
        return Some(consumed);
    }
    if let Some(consumed) = shape_prefix_consumed(words, &DONT_OWN_PREFIX_PATTERN, 2) {
        filter.owner = Some(PlayerFilter::NotYou);
        return Some(consumed);
    }
    if let Some(consumed) = shape_prefix_consumed(words, &DO_NOT_CONTROL_PREFIX_PATTERN, 3) {
        filter.controller = Some(PlayerFilter::NotYou);
        return Some(consumed);
    }
    if let Some(consumed) = shape_prefix_consumed(words, &DO_NOT_OWN_PREFIX_PATTERN, 3) {
        filter.owner = Some(PlayerFilter::NotYou);
        return Some(consumed);
    }
    if let Some(consumed) = shape_prefix_consumed(words, &YOU_DONT_CONTROL_PREFIX_PATTERN, 3) {
        filter.controller = Some(PlayerFilter::NotYou);
        return Some(consumed);
    }
    if let Some(consumed) = shape_prefix_consumed(words, &YOU_DONT_OWN_PREFIX_PATTERN, 3) {
        filter.owner = Some(PlayerFilter::NotYou);
        return Some(consumed);
    }
    if let Some(consumed) = shape_prefix_consumed(words, &YOU_DO_NOT_CONTROL_PREFIX_PATTERN, 4) {
        filter.controller = Some(PlayerFilter::NotYou);
        return Some(consumed);
    }
    if let Some(consumed) = shape_prefix_consumed(words, &YOU_DO_NOT_OWN_PREFIX_PATTERN, 4) {
        filter.owner = Some(PlayerFilter::NotYou);
        return Some(consumed);
    }

    None
}

pub(super) fn try_apply_chosen_player_graveyard_clause(
    filter: &mut ObjectFilter,
    words: &[&str],
) -> Option<usize> {
    let consumed = shape_prefix_consumed(words, &CHOSEN_PLAYER_GRAVEYARD_PREFIX_PATTERN, 3)
        .or_else(|| shape_prefix_consumed(words, &THE_CHOSEN_PLAYER_GRAVEYARD_PREFIX_PATTERN, 4))?;
    filter.owner = Some(PlayerFilter::ChosenPlayer);
    filter.zone = Some(Zone::Graveyard);
    Some(consumed)
}

pub(super) fn try_apply_joint_owner_controller_clause(
    filter: &mut ObjectFilter,
    words: &[&str],
    pronoun_player_filter: &PlayerFilter,
) -> Option<usize> {
    let (player, subject_consumed) = parse_player_relation_subject(words, pronoun_player_filter)?;
    let consumed = shape_prefix_consumed(
        &words[subject_consumed..],
        &BOTH_OWN_AND_CONTROL_PREFIX_PATTERN,
        4,
    )?;
    filter.owner = Some(player.clone());
    filter.controller = Some(player);
    Some(subject_consumed + consumed)
}

pub(super) fn parse_owner_or_controller_disjunction_player(
    words: &[&str],
    pronoun_player_filter: &PlayerFilter,
) -> Option<(PlayerFilter, usize)> {
    let (player, subject_consumed) = parse_player_relation_subject(words, pronoun_player_filter)?;
    if matches!(
        player,
        PlayerFilter::Defending | PlayerFilter::Attacking | PlayerFilter::ControllerOf(_)
    ) {
        return None;
    }
    let consumed = shape_prefix_consumed(
        &words[subject_consumed..],
        &OWN_OR_CONTROL_PREFIX_PATTERN,
        3,
    )?;
    Some((player, subject_consumed + consumed))
}

pub(super) fn find_filter_prefix_consumed<F>(words: &[&str], parser: F) -> Option<(usize, usize)>
where
    F: Fn(&[&str]) -> Option<usize>,
{
    words
        .iter()
        .enumerate()
        .find_map(|(idx, _)| parser(&words[idx..]).map(|consumed| (idx, consumed)))
}

pub(super) fn drain_segment_phrase_variants(
    segment_tokens: &mut Vec<OwnedLexToken>,
    variants: &[SegmentPhraseVariant],
) {
    let segment_words_view = GrammarFilterNormalizedWords::new(segment_tokens.as_slice());
    let segment_words = segment_words_view.to_word_refs();
    let segment_match = variants.iter().find_map(|variant| {
        segment_words
            .windows(variant.words.len())
            .position(|window| window == variant.words)
            .map(|seg_start| {
                (
                    seg_start + variant.drain_start_offset,
                    seg_start + variant.words.len(),
                )
            })
    });
    if let Some((start_word_idx, end_word_idx)) = segment_match
        && let Some(start_token_idx) = segment_words_view.token_index_for_word_index(start_word_idx)
    {
        let end_token_idx = segment_words_view
            .token_index_after_words(end_word_idx)
            .unwrap_or(segment_tokens.len());
        segment_tokens.drain(start_token_idx..end_token_idx);
    }
}

pub(super) fn parse_put_there_from_battlefield_this_turn_words(words: &[&str]) -> Option<usize> {
    shape_prefix_consumed(
        words,
        &PUT_THERE_FROM_BATTLEFIELD_THIS_TURN_PREFIX_PATTERN,
        8,
    )
}

pub(super) fn parse_put_there_from_anywhere_this_turn_words(words: &[&str]) -> Option<usize> {
    shape_prefix_consumed(words, &PUT_THERE_FROM_ANYWHERE_THIS_TURN_PREFIX_PATTERN, 8)
}

pub(super) fn parse_graveyard_from_battlefield_this_turn_words(words: &[&str]) -> Option<usize> {
    shape_prefix_consumed(
        words,
        &GRAVEYARD_FROM_BATTLEFIELD_THIS_TURN_PREFIX_PATTERN,
        5,
    )
}

pub(super) fn parse_entered_battlefield_this_turn_words(
    words: &[&str],
) -> Option<(Option<PlayerFilter>, usize)> {
    if let Some(consumed) = shape_prefix_consumed(
        words,
        &ENTERED_YOUR_CONTROL_THIS_TURN_LONG_PREFIX_PATTERN,
        8,
    )
    .or_else(|| shape_prefix_consumed(words, &ENTERED_YOUR_CONTROL_THIS_TURN_MID_PREFIX_PATTERN, 7))
    .or_else(|| {
        shape_prefix_consumed(
            words,
            &ENTERED_YOUR_CONTROL_THIS_TURN_SHORT_PREFIX_PATTERN,
            6,
        )
    }) {
        return Some((Some(PlayerFilter::You), consumed));
    }
    if let Some(consumed) = shape_prefix_consumed(
        words,
        &ENTERED_OPPONENT_CONTROL_THIS_TURN_LONG_PREFIX_PATTERN,
        8,
    )
    .or_else(|| {
        shape_prefix_consumed(
            words,
            &ENTERED_OPPONENT_CONTROL_THIS_TURN_MID_PREFIX_PATTERN,
            7,
        )
    })
    .or_else(|| {
        shape_prefix_consumed(
            words,
            &ENTERED_OPPONENT_CONTROL_THIS_TURN_SHORT_PREFIX_PATTERN,
            6,
        )
    }) {
        return Some((Some(PlayerFilter::Opponent), consumed));
    }
    if let Some(consumed) = shape_prefix_consumed(words, &ENTERED_THIS_TURN_LONG_PREFIX_PATTERN, 5)
        .or_else(|| shape_prefix_consumed(words, &ENTERED_THIS_TURN_MID_PREFIX_PATTERN, 4))
        .or_else(|| shape_prefix_consumed(words, &ENTERED_THIS_TURN_SHORT_PREFIX_PATTERN, 3))
    {
        return Some((None, consumed));
    }

    None
}

pub(super) fn try_apply_put_there_from_battlefield_this_turn_clause(
    filter: &mut ObjectFilter,
    all_words: &mut Vec<&str>,
    segment_tokens: &mut Vec<OwnedLexToken>,
) -> bool {
    let Some((word_start, consumed)) = find_filter_prefix_consumed(
        all_words.as_slice(),
        parse_put_there_from_battlefield_this_turn_words,
    ) else {
        return false;
    };
    filter.entered_graveyard_this_turn = true;
    filter.entered_graveyard_from_battlefield_this_turn = true;
    all_words.drain(word_start..word_start + consumed);
    drain_segment_phrase_variants(
        segment_tokens,
        &[
            SegmentPhraseVariant {
                words: &[
                    "that",
                    "was",
                    "put",
                    "there",
                    "from",
                    "the",
                    "battlefield",
                    "this",
                    "turn",
                ],
                drain_start_offset: 0,
            },
            SegmentPhraseVariant {
                words: &[
                    "that",
                    "was",
                    "put",
                    "there",
                    "from",
                    "battlefield",
                    "this",
                    "turn",
                ],
                drain_start_offset: 0,
            },
            SegmentPhraseVariant {
                words: &[
                    "that",
                    "were",
                    "put",
                    "there",
                    "from",
                    "the",
                    "battlefield",
                    "this",
                    "turn",
                ],
                drain_start_offset: 0,
            },
            SegmentPhraseVariant {
                words: &[
                    "that",
                    "were",
                    "put",
                    "there",
                    "from",
                    "battlefield",
                    "this",
                    "turn",
                ],
                drain_start_offset: 0,
            },
        ],
    );
    true
}

pub(super) fn try_apply_put_there_from_anywhere_this_turn_clause(
    filter: &mut ObjectFilter,
    all_words: &mut Vec<&str>,
    segment_tokens: &mut Vec<OwnedLexToken>,
) -> bool {
    let Some((word_start, consumed)) = find_filter_prefix_consumed(
        all_words.as_slice(),
        parse_put_there_from_anywhere_this_turn_words,
    ) else {
        return false;
    };
    filter.entered_graveyard_this_turn = true;
    all_words.drain(word_start..word_start + consumed);
    drain_segment_phrase_variants(
        segment_tokens,
        &[
            SegmentPhraseVariant {
                words: &[
                    "that", "was", "put", "there", "from", "anywhere", "this", "turn",
                ],
                drain_start_offset: 0,
            },
            SegmentPhraseVariant {
                words: &[
                    "that", "were", "put", "there", "from", "anywhere", "this", "turn",
                ],
                drain_start_offset: 0,
            },
        ],
    );
    true
}

pub(super) fn try_apply_graveyard_from_battlefield_this_turn_clause(
    filter: &mut ObjectFilter,
    all_words: &mut Vec<&str>,
    segment_tokens: &mut Vec<OwnedLexToken>,
) -> bool {
    let Some((word_start, consumed)) = find_filter_prefix_consumed(
        all_words.as_slice(),
        parse_graveyard_from_battlefield_this_turn_words,
    ) else {
        return false;
    };
    filter.entered_graveyard_from_battlefield_this_turn = true;
    all_words.drain(word_start + 1..word_start + consumed);
    drain_segment_phrase_variants(
        segment_tokens,
        &[
            SegmentPhraseVariant {
                words: &["graveyard", "from", "the", "battlefield", "this", "turn"],
                drain_start_offset: 1,
            },
            SegmentPhraseVariant {
                words: &["graveyard", "from", "battlefield", "this", "turn"],
                drain_start_offset: 1,
            },
            SegmentPhraseVariant {
                words: &["graveyards", "from", "the", "battlefield", "this", "turn"],
                drain_start_offset: 1,
            },
            SegmentPhraseVariant {
                words: &["graveyards", "from", "battlefield", "this", "turn"],
                drain_start_offset: 1,
            },
        ],
    );
    true
}

pub(super) fn try_apply_entered_battlefield_this_turn_clause(
    filter: &mut ObjectFilter,
    all_words: &mut Vec<&str>,
    segment_tokens: &mut Vec<OwnedLexToken>,
) -> bool {
    let Some((word_start, (controller, consumed))) =
        all_words.iter().enumerate().find_map(|(idx, _)| {
            parse_entered_battlefield_this_turn_words(&all_words[idx..])
                .map(|matched| (idx, matched))
        })
    else {
        return false;
    };
    filter.entered_battlefield_this_turn = true;
    filter.entered_battlefield_controller = controller;
    filter.zone = Some(Zone::Battlefield);
    all_words.drain(word_start..word_start + consumed);
    drain_segment_phrase_variants(
        segment_tokens,
        &[
            SegmentPhraseVariant {
                words: &[
                    "entered",
                    "the",
                    "battlefield",
                    "under",
                    "your",
                    "control",
                    "this",
                    "turn",
                ],
                drain_start_offset: 0,
            },
            SegmentPhraseVariant {
                words: &[
                    "entered",
                    "battlefield",
                    "under",
                    "your",
                    "control",
                    "this",
                    "turn",
                ],
                drain_start_offset: 0,
            },
            SegmentPhraseVariant {
                words: &["entered", "under", "your", "control", "this", "turn"],
                drain_start_offset: 0,
            },
            SegmentPhraseVariant {
                words: &[
                    "entered",
                    "the",
                    "battlefield",
                    "under",
                    "opponent",
                    "control",
                    "this",
                    "turn",
                ],
                drain_start_offset: 0,
            },
            SegmentPhraseVariant {
                words: &[
                    "entered",
                    "the",
                    "battlefield",
                    "under",
                    "opponents",
                    "control",
                    "this",
                    "turn",
                ],
                drain_start_offset: 0,
            },
            SegmentPhraseVariant {
                words: &[
                    "entered",
                    "battlefield",
                    "under",
                    "opponent",
                    "control",
                    "this",
                    "turn",
                ],
                drain_start_offset: 0,
            },
            SegmentPhraseVariant {
                words: &[
                    "entered",
                    "battlefield",
                    "under",
                    "opponents",
                    "control",
                    "this",
                    "turn",
                ],
                drain_start_offset: 0,
            },
            SegmentPhraseVariant {
                words: &["entered", "under", "opponent", "control", "this", "turn"],
                drain_start_offset: 0,
            },
            SegmentPhraseVariant {
                words: &["entered", "under", "opponents", "control", "this", "turn"],
                drain_start_offset: 0,
            },
            SegmentPhraseVariant {
                words: &["entered", "the", "battlefield", "this", "turn"],
                drain_start_offset: 0,
            },
            SegmentPhraseVariant {
                words: &["entered", "battlefield", "this", "turn"],
                drain_start_offset: 0,
            },
            SegmentPhraseVariant {
                words: &["entered", "this", "turn"],
                drain_start_offset: 0,
            },
        ],
    );
    true
}

pub(super) fn parse_drawn_this_turn_words(words: &[&str]) -> Option<usize> {
    shape_prefix_consumed(words, &DRAWN_THIS_TURN_PREFIX_PATTERN, 3)
}

pub(super) fn try_apply_drawn_this_turn_clause(
    filter: &mut ObjectFilter,
    all_words: &mut Vec<&str>,
    segment_tokens: &mut Vec<OwnedLexToken>,
) -> bool {
    let Some((word_start, consumed)) =
        find_filter_prefix_consumed(all_words.as_slice(), parse_drawn_this_turn_words)
    else {
        return false;
    };
    filter.drawn_this_turn = true;
    all_words.drain(word_start..word_start + consumed);
    drain_segment_phrase_variants(
        segment_tokens,
        &[SegmentPhraseVariant {
            words: &["drawn", "this", "turn"],
            drain_start_offset: 0,
        }],
    );
    true
}

pub(super) fn push_it_tagged_object_constraint(filter: &mut ObjectFilter) {
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: TagKey::from(IT_TAG),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });
}

pub(super) fn try_apply_leading_tagged_reference_prefix(
    filter: &mut ObjectFilter,
    all_words: &mut Vec<&str>,
) -> bool {
    if all_words.len() >= 2 && LEADING_TAGGED_REFERENCE_WORD_PATTERN.matches_word(all_words[0]) {
        let noun_idx = if all_words
            .get(1)
            .is_some_and(|word| OTHER_WORD_PATTERN.matches_word(word))
        {
            2
        } else {
            1
        };
        if all_words
            .get(noun_idx)
            .is_some_and(|word| is_demonstrative_object_head(word))
        {
            push_it_tagged_object_constraint(filter);
            all_words.remove(0);
            return true;
        }
    }

    if all_words
        .first()
        .is_some_and(|word| IT_OR_THEM_WORD_PATTERN.matches_word(word))
    {
        push_it_tagged_object_constraint(filter);
        all_words.remove(0);
        return true;
    }

    false
}

pub(super) fn is_name_clause_boundary(word: &str) -> bool {
    matches!(
        word,
        "in" | "from"
            | "with"
            | "without"
            | "that"
            | "which"
            | "who"
            | "whose"
            | "under"
            | "among"
            | "on"
            | "you"
            | "your"
            | "opponent"
            | "opponents"
            | "their"
            | "its"
            | "controller"
            | "controllers"
            | "owner"
            | "owners"
    )
}

pub(super) fn find_name_clause_end(all_words: &[&str], name_start: usize) -> usize {
    let mut name_end = all_words.len();
    for idx in (name_start + 1)..all_words.len() {
        if is_name_clause_boundary(all_words[idx]) {
            name_end = idx;
            break;
        }
    }
    name_end
}

pub(super) fn extract_name_clause_text<'a, F, G>(
    all_words: &[&'a str],
    all_words_with_articles: &[&'a str],
    marker_idx: usize,
    marker_len: usize,
    map_non_article_index: &F,
    map_non_article_end: &G,
    error_label: &str,
) -> Result<(String, usize), CardTextError>
where
    F: Fn(usize) -> Option<usize>,
    G: Fn(usize) -> Option<usize>,
{
    let name_start = marker_idx + marker_len;
    let name_end = find_name_clause_end(all_words, name_start);
    let full_marker_idx = map_non_article_index(marker_idx).unwrap_or(marker_idx);
    let full_name_end = map_non_article_end(name_end).unwrap_or(name_end);
    let name_words = if full_marker_idx + marker_len <= full_name_end
        && full_name_end <= all_words_with_articles.len()
    {
        &all_words_with_articles[full_marker_idx + marker_len..full_name_end]
    } else {
        &all_words[name_start..name_end]
    };
    if name_words.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing card name in {error_label} object filter (clause: '{}')",
            all_words.join(" ")
        )));
    }

    Ok((name_words.join(" "), name_end))
}
