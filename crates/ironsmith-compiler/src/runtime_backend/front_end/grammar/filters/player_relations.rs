use super::*;
use crate::runtime_backend::lex_patterns::{LexCaptureKind, LexCaptureRole, LexPattern};

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

const RELATION_AXIS_PATTERN: LexPattern<'static> = LexPattern::new(&[LexPattern::action(
    "axis",
    LexCaptureKind::OneOfPhrase(&[&["power"], &["toughness"], &["mana", "value"]]),
)]);
const RELATION_VERB_PATTERN: LexPattern<'static> = LexPattern::new(&[LexPattern::action(
    "verb",
    LexCaptureKind::OneOf(&["cast", "casts", "control", "controls", "own", "owns"]),
)]);
const RELATION_SUBJECT_PATTERN: LexPattern<'static> = LexPattern::new(&[LexPattern::subject(
    "player",
    LexCaptureKind::OneOfPhrase(&[
        &["you"],
        &["opponent"],
        &["opponents"],
        &["they"],
        &["your", "team"],
        &["your", "opponents"],
        &["that", "player"],
        &["target", "player"],
        &["target", "opponent"],
        &["defending", "player"],
        &["attacking", "player"],
        &["its", "controller"],
        &["its", "controllers"],
        &["their", "controller"],
        &["their", "controllers"],
    ]),
)]);
const NEGATED_YOU_RELATION_PATTERN: LexPattern<'static> = LexPattern::new(&[
    LexPattern::optional(&[LexPattern::subject(
        "player",
        LexCaptureKind::OneOf(&["you"]),
    )]),
    LexPattern::action(
        "verb",
        LexCaptureKind::OneOfPhrase(&[
            &["dont", "control"],
            &["dont", "controls"],
            &["don't", "control"],
            &["don't", "controls"],
            &["do", "not", "control"],
            &["do", "not", "controls"],
            &["dont", "own"],
            &["dont", "owns"],
            &["don't", "own"],
            &["don't", "owns"],
            &["do", "not", "own"],
            &["do", "not", "owns"],
        ]),
    ),
]);
const CHOSEN_PLAYER_GRAVEYARD_PATTERN: LexPattern<'static> =
    LexPattern::new(&[LexPattern::object(
        "zone",
        LexCaptureKind::OneOfPhrase(&[
            &["chosen", "player", "graveyard"],
            &["chosen", "players", "graveyard"],
            &["the", "chosen", "player", "graveyard"],
            &["the", "chosen", "players", "graveyard"],
        ]),
    )]);
const JOINT_OWNER_CONTROLLER_PATTERN: LexPattern<'static> = LexPattern::new(&[LexPattern::action(
    "relation",
    LexCaptureKind::OneOfPhrase(&[
        &["both", "own", "and", "control"],
        &["both", "owns", "and", "control"],
        &["both", "own", "and", "controls"],
        &["both", "owns", "and", "controls"],
        &["both", "control", "and", "own"],
        &["both", "controls", "and", "own"],
        &["both", "control", "and", "owns"],
        &["both", "controls", "and", "owns"],
    ]),
)]);
const OWNER_OR_CONTROLLER_PATTERN: LexPattern<'static> = LexPattern::new(&[LexPattern::action(
    "relation",
    LexCaptureKind::OneOfPhrase(&[
        &["own", "or", "control"],
        &["owns", "or", "control"],
        &["own", "or", "controls"],
        &["owns", "or", "controls"],
        &["control", "or", "own"],
        &["controls", "or", "own"],
        &["control", "or", "owns"],
        &["controls", "or", "owns"],
    ]),
)]);
const PUT_THERE_FROM_BATTLEFIELD_THIS_TURN_PATTERN: LexPattern<'static> =
    LexPattern::new(&[LexPattern::action(
        "event",
        LexCaptureKind::OneOfPhrase(&[
            &[
                "that",
                "was",
                "put",
                "there",
                "from",
                "battlefield",
                "this",
                "turn",
            ],
            &[
                "that",
                "were",
                "put",
                "there",
                "from",
                "battlefield",
                "this",
                "turn",
            ],
        ]),
    )]);
const PUT_THERE_FROM_ANYWHERE_THIS_TURN_PATTERN: LexPattern<'static> =
    LexPattern::new(&[LexPattern::action(
        "event",
        LexCaptureKind::OneOfPhrase(&[
            &[
                "that", "was", "put", "there", "from", "anywhere", "this", "turn",
            ],
            &[
                "that", "were", "put", "there", "from", "anywhere", "this", "turn",
            ],
        ]),
    )]);
const GRAVEYARD_FROM_BATTLEFIELD_THIS_TURN_PATTERN: LexPattern<'static> =
    LexPattern::new(&[LexPattern::action(
        "event",
        LexCaptureKind::OneOfPhrase(&[
            &["graveyard", "from", "battlefield", "this", "turn"],
            &["graveyards", "from", "battlefield", "this", "turn"],
        ]),
    )]);
const ENTERED_BATTLEFIELD_THIS_TURN_PATTERN: LexPattern<'static> =
    LexPattern::new(&[LexPattern::action(
        "event",
        LexCaptureKind::OneOfPhrase(&[
            &[
                "entered",
                "the",
                "battlefield",
                "under",
                "your",
                "control",
                "this",
                "turn",
            ],
            &[
                "entered",
                "battlefield",
                "under",
                "your",
                "control",
                "this",
                "turn",
            ],
            &["entered", "under", "your", "control", "this", "turn"],
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
            &["entered", "under", "opponent", "control", "this", "turn"],
            &["entered", "under", "opponents", "control", "this", "turn"],
            &["entered", "the", "battlefield", "this", "turn"],
            &["entered", "battlefield", "this", "turn"],
            &["entered", "this", "turn"],
        ]),
    )]);
const DRAWN_THIS_TURN_PATTERN: LexPattern<'static> = LexPattern::new(&[LexPattern::action(
    "event",
    LexCaptureKind::OneOfPhrase(&[&["drawn", "this", "turn"]]),
)]);
const LEADING_TAGGED_REFERENCE_WORDS: &[&str] = &["that", "those", "chosen"];
const IT_OR_THEM_WORDS: &[&str] = &["it", "them"];

fn relation_captured_prefix(
    words: &[&str],
    pattern: LexPattern<'static>,
    role: LexCaptureRole,
) -> Option<(String, usize)> {
    let matched = pattern.match_prefix_word_refs(words)?;
    let capture = matched.capture_by_role(role)?;
    let captured_words = words.get(capture.word_range.clone())?;
    Some((captured_words.join(" "), matched.word_range.end))
}

fn parse_relation_axis_shape(words: &[&str]) -> Option<(SpellFilterComparisonAxis, usize)> {
    let (axis, consumed) =
        relation_captured_prefix(words, RELATION_AXIS_PATTERN, LexCaptureRole::Action)?;
    match axis.as_str() {
        "power" => Some((SpellFilterComparisonAxis::Power, consumed)),
        "toughness" => Some((SpellFilterComparisonAxis::Toughness, consumed)),
        "mana value" => Some((SpellFilterComparisonAxis::ManaValue, consumed)),
        _ => None,
    }
}

fn parse_relation_verb_shape(words: &[&str]) -> Option<(PlayerRelationVerb, usize)> {
    let (verb, consumed) =
        relation_captured_prefix(words, RELATION_VERB_PATTERN, LexCaptureRole::Action)?;
    match verb.as_str() {
        "cast" | "casts" => Some((PlayerRelationVerb::Cast, consumed)),
        "control" | "controls" => Some((PlayerRelationVerb::Control, consumed)),
        "own" | "owns" => Some((PlayerRelationVerb::Own, consumed)),
        _ => None,
    }
}

fn parse_relation_subject_shape(
    words: &[&str],
    pronoun_player_filter: &PlayerFilter,
) -> Option<(PlayerFilter, usize)> {
    let (subject, consumed) =
        relation_captured_prefix(words, RELATION_SUBJECT_PATTERN, LexCaptureRole::Subject)?;
    match subject.as_str() {
        "you" | "your team" => Some((PlayerFilter::You, consumed)),
        "opponent" | "opponents" | "your opponents" => Some((PlayerFilter::Opponent, consumed)),
        "they" => Some((pronoun_player_filter.clone(), consumed)),
        "that player" => Some((PlayerFilter::IteratedPlayer, consumed)),
        "target player" => Some((PlayerFilter::target_player(), consumed)),
        "target opponent" => Some((PlayerFilter::target_opponent(), consumed)),
        "defending player" => Some((PlayerFilter::Defending, consumed)),
        "attacking player" => Some((PlayerFilter::Attacking, consumed)),
        "its controller" | "its controllers" | "their controller" | "their controllers" => Some((
            PlayerFilter::ControllerOf(crate::filter::ObjectRef::Target),
            consumed,
        )),
        _ => None,
    }
}

fn parse_negated_you_relation_shape(words: &[&str]) -> Option<(PlayerRelationVerb, usize)> {
    let (verb, consumed) =
        relation_captured_prefix(words, NEGATED_YOU_RELATION_PATTERN, LexCaptureRole::Action)?;
    if verb.ends_with("control") || verb.ends_with("controls") {
        return Some((PlayerRelationVerb::Control, consumed));
    }
    if verb.ends_with("own") || verb.ends_with("owns") {
        return Some((PlayerRelationVerb::Own, consumed));
    }
    None
}

fn parse_chosen_player_graveyard_shape(words: &[&str]) -> Option<usize> {
    relation_captured_prefix(
        words,
        CHOSEN_PLAYER_GRAVEYARD_PATTERN,
        LexCaptureRole::Object,
    )
    .map(|(_, consumed)| consumed)
}

fn parse_joint_owner_controller_shape(words: &[&str]) -> Option<usize> {
    relation_captured_prefix(
        words,
        JOINT_OWNER_CONTROLLER_PATTERN,
        LexCaptureRole::Action,
    )
    .map(|(_, consumed)| consumed)
}

fn parse_owner_or_controller_shape(words: &[&str]) -> Option<usize> {
    relation_captured_prefix(words, OWNER_OR_CONTROLLER_PATTERN, LexCaptureRole::Action)
        .map(|(_, consumed)| consumed)
}

fn parse_relation_event_shape(words: &[&str], pattern: LexPattern<'static>) -> Option<usize> {
    relation_captured_prefix(words, pattern, LexCaptureRole::Action).map(|(_, consumed)| consumed)
}

fn parse_entered_battlefield_this_turn_shape(
    words: &[&str],
) -> Option<(Option<PlayerFilter>, usize)> {
    let (event, consumed) = relation_captured_prefix(
        words,
        ENTERED_BATTLEFIELD_THIS_TURN_PATTERN,
        LexCaptureRole::Action,
    )?;
    if event.contains("under your control") {
        return Some((Some(PlayerFilter::You), consumed));
    }
    if event.contains("under opponent control") || event.contains("under opponents control") {
        return Some((Some(PlayerFilter::Opponent), consumed));
    }
    Some((None, consumed))
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
    parse_relation_axis_shape(words)
}

pub(super) fn parse_player_relation_verb(words: &[&str]) -> Option<(PlayerRelationVerb, usize)> {
    parse_relation_verb_shape(words)
}

pub(super) fn parse_player_relation_subject(
    words: &[&str],
    pronoun_player_filter: &PlayerFilter,
) -> Option<(PlayerFilter, usize)> {
    parse_relation_subject_shape(words, pronoun_player_filter)
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
    let (verb, consumed) = parse_negated_you_relation_shape(words)?;
    match verb {
        PlayerRelationVerb::Control => filter.controller = Some(PlayerFilter::NotYou),
        PlayerRelationVerb::Own => filter.owner = Some(PlayerFilter::NotYou),
        PlayerRelationVerb::Cast => return None,
    }
    Some(consumed)
}

pub(super) fn try_apply_chosen_player_graveyard_clause(
    filter: &mut ObjectFilter,
    words: &[&str],
) -> Option<usize> {
    let consumed = parse_chosen_player_graveyard_shape(words)?;
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
    let consumed = parse_joint_owner_controller_shape(&words[subject_consumed..])?;
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
    let consumed = parse_owner_or_controller_shape(&words[subject_consumed..])?;
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
    parse_relation_event_shape(words, PUT_THERE_FROM_BATTLEFIELD_THIS_TURN_PATTERN)
}

pub(super) fn parse_put_there_from_anywhere_this_turn_words(words: &[&str]) -> Option<usize> {
    parse_relation_event_shape(words, PUT_THERE_FROM_ANYWHERE_THIS_TURN_PATTERN)
}

pub(super) fn parse_graveyard_from_battlefield_this_turn_words(words: &[&str]) -> Option<usize> {
    parse_relation_event_shape(words, GRAVEYARD_FROM_BATTLEFIELD_THIS_TURN_PATTERN)
}

pub(super) fn parse_entered_battlefield_this_turn_words(
    words: &[&str],
) -> Option<(Option<PlayerFilter>, usize)> {
    parse_entered_battlefield_this_turn_shape(words)
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
    parse_relation_event_shape(words, DRAWN_THIS_TURN_PATTERN)
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
    if all_words.len() >= 2 && LEADING_TAGGED_REFERENCE_WORDS.contains(&all_words[0]) {
        let noun_idx = if all_words.get(1).is_some_and(|word| *word == "other") {
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
        .is_some_and(|word| IT_OR_THEM_WORDS.contains(word))
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
