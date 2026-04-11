use winnow::Parser;
use winnow::combinator::alt;
use winnow::error::{ContextError, ErrMode};

use super::super::activation_and_restrictions::parse_named_number;
use super::super::effect_sentences::{conditionals::parse_subtype_word, parse_supertype_word};
use super::super::keyword_static::parse_pt_modifier;
use super::super::lexer::OwnedLexToken;
use super::super::object_filters::{
    apply_parity_filter_phrases, find_word_slice_phrase_start, lower_words_find_index,
    normalized_token_index_after_words, normalized_token_index_for_word_index,
    parse_attached_reference_or_another_disjunction, parse_filter_face_state_words,
    parse_object_filter_lexed, push_unique, set_has, slice_has, strip_not_on_battlefield_phrase,
    token_find_index, trim_vote_winner_suffix,
};
use super::super::token_primitives::{
    find_index, rfind_index, slice_contains, slice_ends_with, slice_starts_with, slice_strip_prefix,
};
use super::super::util::{
    apply_filter_keyword_constraint, is_article, is_demonstrative_object_head, is_non_outlaw_word,
    is_outlaw_word, is_permanent_type, is_source_reference_words, parse_alternative_cast_words,
    parse_card_type, parse_color, parse_counter_type_word, parse_filter_counter_constraint_words,
    parse_filter_keyword_constraint_words, parse_mana_symbol_word_flexible, parse_non_color,
    parse_non_subtype, parse_non_supertype, parse_non_type, parse_number, parse_subtype_flexible,
    parse_unsigned_pt_word, parse_zone_word, push_outlaw_subtypes, trim_commas,
};
use super::super::value_helpers::parse_filter_comparison_tokens;
use super::primitives::{self, TokenWordView, split_lexed_slices_on_and, split_lexed_slices_on_or};
use super::values::parse_mana_symbol;
use crate::cards::TextSpan;
use crate::cards::builders::{CardTextError, IT_TAG, PlayerAst, PredicateAst, TagKey};
use crate::color::{Color, ColorSet};
use crate::effect::Value;
use crate::effects::VOTE_WINNERS_TAG;
use crate::filter::TaggedObjectConstraint;
use crate::mana::ManaSymbol;
use crate::target::{ObjectFilter, PlayerFilter, TaggedOpbjectRelation};
use crate::types::{CardType, Supertype};
use crate::zone::Zone;

type GrammarFilterNormalizedWords<'a> = TokenWordView<'a>;

type FilterWordInput<'a> = primitives::WordSliceInput<'a>;

fn synth_words_as_tokens(words: &[&str]) -> Vec<OwnedLexToken> {
    words
        .iter()
        .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
        .collect()
}

fn push_unique_filter_value<T: Copy + PartialEq>(items: &mut Vec<T>, value: T) {
    if !items.iter().any(|item| *item == value) {
        items.push(value);
    }
}

fn parse_filter_prefix_words<'a, O>(
    words: &'a [&'a str],
    mut parser: impl Parser<FilterWordInput<'a>, O, ErrMode<ContextError>>,
) -> Option<(O, usize)> {
    let mut input = words;
    let parsed = parser.parse_next(&mut input).ok()?;
    Some((parsed, words.len().saturating_sub(input.len())))
}

#[derive(Clone, Copy)]
enum SpellFilterComparisonAxis {
    Power,
    Toughness,
    ManaValue,
}

#[derive(Clone, Copy)]
enum PlayerRelationVerb {
    Cast,
    Control,
    Own,
}

#[derive(Clone, Copy)]
struct SegmentPhraseVariant {
    words: &'static [&'static str],
    drain_start_offset: usize,
}

impl SpellFilterComparisonAxis {
    fn as_str(self) -> &'static str {
        match self {
            Self::Power => "power",
            Self::Toughness => "toughness",
            Self::ManaValue => "mana value",
        }
    }

    fn assign(self, filter: &mut ObjectFilter, comparison: crate::target::Comparison) {
        match self {
            Self::Power => filter.power = Some(comparison),
            Self::Toughness => filter.toughness = Some(comparison),
            Self::ManaValue => filter.mana_value = Some(comparison),
        }
    }
}

fn parse_spell_filter_comparison_axis_words(
    words: &[&str],
) -> Option<(SpellFilterComparisonAxis, usize)> {
    parse_filter_prefix_words(
        words,
        alt((
            primitives::word_slice_eq("power").value(SpellFilterComparisonAxis::Power),
            primitives::word_slice_eq("toughness").value(SpellFilterComparisonAxis::Toughness),
            (
                primitives::word_slice_eq("mana"),
                primitives::word_slice_eq("value"),
            )
                .value(SpellFilterComparisonAxis::ManaValue),
        )),
    )
}

fn parse_player_relation_verb(words: &[&str]) -> Option<(PlayerRelationVerb, usize)> {
    parse_filter_prefix_words(
        words,
        alt((
            alt((
                primitives::word_slice_eq("cast"),
                primitives::word_slice_eq("casts"),
            ))
            .value(PlayerRelationVerb::Cast),
            alt((
                primitives::word_slice_eq("control"),
                primitives::word_slice_eq("controls"),
            ))
            .value(PlayerRelationVerb::Control),
            alt((
                primitives::word_slice_eq("own"),
                primitives::word_slice_eq("owns"),
            ))
            .value(PlayerRelationVerb::Own),
        )),
    )
}

fn parse_player_relation_subject(
    words: &[&str],
    pronoun_player_filter: &PlayerFilter,
) -> Option<(PlayerFilter, usize)> {
    if let Some((_, consumed)) = parse_filter_prefix_words(words, primitives::word_slice_eq("you"))
    {
        return Some((PlayerFilter::You, consumed));
    }
    if let Some((_, consumed)) = parse_filter_prefix_words(
        words,
        alt((
            primitives::word_slice_eq("opponent"),
            primitives::word_slice_eq("opponents"),
        )),
    ) {
        return Some((PlayerFilter::Opponent, consumed));
    }
    if let Some((_, consumed)) = parse_filter_prefix_words(words, primitives::word_slice_eq("they"))
    {
        return Some((pronoun_player_filter.clone(), consumed));
    }
    if let Some((_, consumed)) = parse_filter_prefix_words(
        words,
        (
            primitives::word_slice_eq("your"),
            primitives::word_slice_eq("team"),
        ),
    ) {
        return Some((PlayerFilter::You, consumed));
    }
    if let Some((_, consumed)) = parse_filter_prefix_words(
        words,
        (
            primitives::word_slice_eq("your"),
            primitives::word_slice_eq("opponents"),
        ),
    ) {
        return Some((PlayerFilter::Opponent, consumed));
    }
    if let Some((_, consumed)) = parse_filter_prefix_words(
        words,
        (
            primitives::word_slice_eq("that"),
            primitives::word_slice_eq("player"),
        ),
    ) {
        return Some((PlayerFilter::IteratedPlayer, consumed));
    }
    if let Some((_, consumed)) = parse_filter_prefix_words(
        words,
        (
            primitives::word_slice_eq("target"),
            primitives::word_slice_eq("player"),
        ),
    ) {
        return Some((PlayerFilter::target_player(), consumed));
    }
    if let Some((_, consumed)) = parse_filter_prefix_words(
        words,
        (
            primitives::word_slice_eq("target"),
            primitives::word_slice_eq("opponent"),
        ),
    ) {
        return Some((PlayerFilter::target_opponent(), consumed));
    }
    if let Some((_, consumed)) = parse_filter_prefix_words(
        words,
        (
            primitives::word_slice_eq("defending"),
            primitives::word_slice_eq("player"),
        ),
    ) {
        return Some((PlayerFilter::Defending, consumed));
    }
    if let Some((_, consumed)) = parse_filter_prefix_words(
        words,
        (
            primitives::word_slice_eq("attacking"),
            primitives::word_slice_eq("player"),
        ),
    ) {
        return Some((PlayerFilter::Attacking, consumed));
    }
    if let Some((_, consumed)) = parse_filter_prefix_words(
        words,
        (
            alt((
                primitives::word_slice_eq("its"),
                primitives::word_slice_eq("their"),
            )),
            alt((
                primitives::word_slice_eq("controller"),
                primitives::word_slice_eq("controllers"),
            )),
        ),
    ) {
        return Some((
            PlayerFilter::ControllerOf(crate::filter::ObjectRef::Target),
            consumed,
        ));
    }

    None
}

fn apply_player_relation(
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

fn try_apply_player_relation_clause(
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

fn try_apply_negated_you_relation_clause(
    filter: &mut ObjectFilter,
    words: &[&str],
) -> Option<usize> {
    if let Some((_, consumed)) = parse_filter_prefix_words(
        words,
        (
            primitives::word_slice_eq("you"),
            alt((
                primitives::word_slice_eq("dont"),
                primitives::word_slice_eq("don't"),
            )),
            alt((
                primitives::word_slice_eq("control"),
                primitives::word_slice_eq("controls"),
            )),
        ),
    ) {
        filter.controller = Some(PlayerFilter::NotYou);
        return Some(consumed);
    }
    if let Some((_, consumed)) = parse_filter_prefix_words(
        words,
        (
            primitives::word_slice_eq("you"),
            alt((
                primitives::word_slice_eq("dont"),
                primitives::word_slice_eq("don't"),
            )),
            alt((
                primitives::word_slice_eq("own"),
                primitives::word_slice_eq("owns"),
            )),
        ),
    ) {
        filter.owner = Some(PlayerFilter::NotYou);
        return Some(consumed);
    }
    if let Some((_, consumed)) = parse_filter_prefix_words(
        words,
        (
            primitives::word_slice_eq("you"),
            primitives::word_slice_eq("do"),
            primitives::word_slice_eq("not"),
            alt((
                primitives::word_slice_eq("control"),
                primitives::word_slice_eq("controls"),
            )),
        ),
    ) {
        filter.controller = Some(PlayerFilter::NotYou);
        return Some(consumed);
    }
    if let Some((_, consumed)) = parse_filter_prefix_words(
        words,
        (
            primitives::word_slice_eq("you"),
            primitives::word_slice_eq("do"),
            primitives::word_slice_eq("not"),
            alt((
                primitives::word_slice_eq("own"),
                primitives::word_slice_eq("owns"),
            )),
        ),
    ) {
        filter.owner = Some(PlayerFilter::NotYou);
        return Some(consumed);
    }

    None
}

fn try_apply_chosen_player_graveyard_clause(
    filter: &mut ObjectFilter,
    words: &[&str],
) -> Option<usize> {
    if let Some((_, consumed)) = parse_filter_prefix_words(
        words,
        alt((
            (
                primitives::word_slice_eq("chosen"),
                alt((
                    primitives::word_slice_eq("player"),
                    primitives::word_slice_eq("players"),
                )),
                primitives::word_slice_eq("graveyard"),
            )
                .void(),
            (
                primitives::word_slice_eq("the"),
                primitives::word_slice_eq("chosen"),
                alt((
                    primitives::word_slice_eq("player"),
                    primitives::word_slice_eq("players"),
                )),
                primitives::word_slice_eq("graveyard"),
            )
                .void(),
        )),
    ) {
        filter.owner = Some(PlayerFilter::ChosenPlayer);
        filter.zone = Some(Zone::Graveyard);
        return Some(consumed);
    }

    None
}

fn try_apply_joint_owner_controller_clause(
    filter: &mut ObjectFilter,
    words: &[&str],
    pronoun_player_filter: &PlayerFilter,
) -> Option<usize> {
    let (player, subject_consumed) = parse_player_relation_subject(words, pronoun_player_filter)?;
    let (_, consumed) = parse_filter_prefix_words(
        &words[subject_consumed..],
        alt((
            (
                primitives::word_slice_eq("both"),
                alt((
                    primitives::word_slice_eq("own"),
                    primitives::word_slice_eq("owns"),
                )),
                primitives::word_slice_eq("and"),
                alt((
                    primitives::word_slice_eq("control"),
                    primitives::word_slice_eq("controls"),
                )),
            ),
            (
                primitives::word_slice_eq("both"),
                alt((
                    primitives::word_slice_eq("control"),
                    primitives::word_slice_eq("controls"),
                )),
                primitives::word_slice_eq("and"),
                alt((
                    primitives::word_slice_eq("own"),
                    primitives::word_slice_eq("owns"),
                )),
            ),
        )),
    )?;
    filter.owner = Some(player.clone());
    filter.controller = Some(player);
    Some(subject_consumed + consumed)
}

fn parse_owner_or_controller_disjunction_player(
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
    let (_, consumed) = parse_filter_prefix_words(
        &words[subject_consumed..],
        alt((
            (
                alt((
                    primitives::word_slice_eq("own"),
                    primitives::word_slice_eq("owns"),
                )),
                primitives::word_slice_eq("or"),
                alt((
                    primitives::word_slice_eq("control"),
                    primitives::word_slice_eq("controls"),
                )),
            ),
            (
                alt((
                    primitives::word_slice_eq("control"),
                    primitives::word_slice_eq("controls"),
                )),
                primitives::word_slice_eq("or"),
                alt((
                    primitives::word_slice_eq("own"),
                    primitives::word_slice_eq("owns"),
                )),
            ),
        )),
    )?;
    Some((player, subject_consumed + consumed))
}

fn find_filter_prefix_consumed<F>(words: &[&str], parser: F) -> Option<(usize, usize)>
where
    F: Fn(&[&str]) -> Option<usize>,
{
    words
        .iter()
        .enumerate()
        .find_map(|(idx, _)| parser(&words[idx..]).map(|consumed| (idx, consumed)))
}

fn drain_segment_phrase_variants(
    segment_tokens: &mut Vec<OwnedLexToken>,
    variants: &[SegmentPhraseVariant],
) {
    let segment_words_view = GrammarFilterNormalizedWords::new(segment_tokens.as_slice());
    let segment_words = segment_words_view.to_word_refs();
    let segment_match = variants.iter().find_map(|variant| {
        find_word_slice_phrase_start(&segment_words, variant.words).map(|seg_start| {
            (
                seg_start + variant.drain_start_offset,
                seg_start + variant.words.len(),
            )
        })
    });
    if let Some((start_word_idx, end_word_idx)) = segment_match
        && let Some(start_token_idx) =
            normalized_token_index_for_word_index(segment_tokens.as_slice(), start_word_idx)
    {
        let end_token_idx =
            normalized_token_index_after_words(segment_tokens.as_slice(), end_word_idx)
                .unwrap_or(segment_tokens.len());
        segment_tokens.drain(start_token_idx..end_token_idx);
    }
}

fn parse_put_there_from_battlefield_this_turn_words(words: &[&str]) -> Option<usize> {
    parse_filter_prefix_words(
        words,
        (
            primitives::word_slice_eq("that"),
            alt((
                primitives::word_slice_eq("was"),
                primitives::word_slice_eq("were"),
            )),
            primitives::word_slice_eq("put"),
            primitives::word_slice_eq("there"),
            primitives::word_slice_eq("from"),
            primitives::word_slice_eq("battlefield"),
            primitives::word_slice_eq("this"),
            primitives::word_slice_eq("turn"),
        )
            .void(),
    )
    .map(|(_, consumed)| consumed)
}

fn parse_put_there_from_anywhere_this_turn_words(words: &[&str]) -> Option<usize> {
    parse_filter_prefix_words(
        words,
        (
            primitives::word_slice_eq("that"),
            alt((
                primitives::word_slice_eq("was"),
                primitives::word_slice_eq("were"),
            )),
            primitives::word_slice_eq("put"),
            primitives::word_slice_eq("there"),
            primitives::word_slice_eq("from"),
            primitives::word_slice_eq("anywhere"),
            primitives::word_slice_eq("this"),
            primitives::word_slice_eq("turn"),
        )
            .void(),
    )
    .map(|(_, consumed)| consumed)
}

fn parse_graveyard_from_battlefield_this_turn_words(words: &[&str]) -> Option<usize> {
    parse_filter_prefix_words(
        words,
        (
            alt((
                primitives::word_slice_eq("graveyard"),
                primitives::word_slice_eq("graveyards"),
            )),
            primitives::word_slice_eq("from"),
            primitives::word_slice_eq("battlefield"),
            primitives::word_slice_eq("this"),
            primitives::word_slice_eq("turn"),
        )
            .void(),
    )
    .map(|(_, consumed)| consumed)
}

fn parse_entered_battlefield_this_turn_words(
    words: &[&str],
) -> Option<(Option<PlayerFilter>, usize)> {
    if let Some((_, consumed)) = parse_filter_prefix_words(
        words,
        alt((
            (
                primitives::word_slice_eq("entered"),
                primitives::word_slice_eq("the"),
                primitives::word_slice_eq("battlefield"),
                primitives::word_slice_eq("under"),
                primitives::word_slice_eq("your"),
                primitives::word_slice_eq("control"),
                primitives::word_slice_eq("this"),
                primitives::word_slice_eq("turn"),
            )
                .void(),
            (
                primitives::word_slice_eq("entered"),
                primitives::word_slice_eq("battlefield"),
                primitives::word_slice_eq("under"),
                primitives::word_slice_eq("your"),
                primitives::word_slice_eq("control"),
                primitives::word_slice_eq("this"),
                primitives::word_slice_eq("turn"),
            )
                .void(),
        )),
    ) {
        return Some((Some(PlayerFilter::You), consumed));
    }
    if let Some((_, consumed)) = parse_filter_prefix_words(
        words,
        alt((
            (
                primitives::word_slice_eq("entered"),
                primitives::word_slice_eq("the"),
                primitives::word_slice_eq("battlefield"),
                primitives::word_slice_eq("under"),
                alt((
                    primitives::word_slice_eq("opponent"),
                    primitives::word_slice_eq("opponents"),
                )),
                primitives::word_slice_eq("control"),
                primitives::word_slice_eq("this"),
                primitives::word_slice_eq("turn"),
            )
                .void(),
            (
                primitives::word_slice_eq("entered"),
                primitives::word_slice_eq("battlefield"),
                primitives::word_slice_eq("under"),
                alt((
                    primitives::word_slice_eq("opponent"),
                    primitives::word_slice_eq("opponents"),
                )),
                primitives::word_slice_eq("control"),
                primitives::word_slice_eq("this"),
                primitives::word_slice_eq("turn"),
            )
                .void(),
        )),
    ) {
        return Some((Some(PlayerFilter::Opponent), consumed));
    }
    if let Some((_, consumed)) = parse_filter_prefix_words(
        words,
        alt((
            (
                primitives::word_slice_eq("entered"),
                primitives::word_slice_eq("the"),
                primitives::word_slice_eq("battlefield"),
                primitives::word_slice_eq("this"),
                primitives::word_slice_eq("turn"),
            )
                .void(),
            (
                primitives::word_slice_eq("entered"),
                primitives::word_slice_eq("battlefield"),
                primitives::word_slice_eq("this"),
                primitives::word_slice_eq("turn"),
            )
                .void(),
        )),
    ) {
        return Some((None, consumed));
    }

    None
}

fn try_apply_put_there_from_battlefield_this_turn_clause(
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

fn try_apply_put_there_from_anywhere_this_turn_clause(
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

fn try_apply_graveyard_from_battlefield_this_turn_clause(
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

fn try_apply_entered_battlefield_this_turn_clause(
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
                words: &["entered", "the", "battlefield", "this", "turn"],
                drain_start_offset: 0,
            },
            SegmentPhraseVariant {
                words: &["entered", "battlefield", "this", "turn"],
                drain_start_offset: 0,
            },
        ],
    );
    true
}

fn push_it_tagged_object_constraint(filter: &mut ObjectFilter) {
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: TagKey::from(IT_TAG),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });
}

fn try_apply_leading_tagged_reference_prefix(
    filter: &mut ObjectFilter,
    all_words: &mut Vec<&str>,
) -> bool {
    if all_words.len() >= 2 && matches!(all_words[0], "that" | "those" | "chosen") {
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
        .is_some_and(|word| matches!(*word, "it" | "them"))
    {
        push_it_tagged_object_constraint(filter);
        all_words.remove(0);
        return true;
    }

    false
}

fn is_name_clause_boundary(word: &str) -> bool {
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

fn find_name_clause_end(all_words: &[&str], name_start: usize) -> usize {
    let mut name_end = all_words.len();
    for idx in (name_start + 1)..all_words.len() {
        if is_name_clause_boundary(all_words[idx]) {
            name_end = idx;
            break;
        }
    }
    name_end
}

fn extract_name_clause_text<'a, F, G>(
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

fn remove_word_range(words: &mut Vec<&str>, start: usize, end: usize) {
    let mut remaining = Vec::with_capacity(words.len());
    remaining.extend_from_slice(&words[..start]);
    remaining.extend_from_slice(&words[end..]);
    *words = remaining;
}

fn try_apply_not_named_clause<'a, F, G>(
    filter: &mut ObjectFilter,
    all_words: &mut Vec<&'a str>,
    all_words_with_articles: &[&'a str],
    map_non_article_index: &F,
    map_non_article_end: &G,
) -> Result<bool, CardTextError>
where
    F: Fn(usize) -> Option<usize>,
    G: Fn(usize) -> Option<usize>,
{
    let Some(not_named_idx) = find_word_slice_phrase_start(all_words.as_slice(), &["not", "named"])
    else {
        return Ok(false);
    };
    let (name, name_end) = extract_name_clause_text(
        all_words.as_slice(),
        all_words_with_articles,
        not_named_idx,
        2,
        map_non_article_index,
        map_non_article_end,
        "not-named",
    )?;
    filter.excluded_name = Some(name);
    remove_word_range(all_words, not_named_idx, name_end);
    Ok(true)
}

fn try_apply_named_clause<'a, F, G>(
    filter: &mut ObjectFilter,
    all_words: &mut Vec<&'a str>,
    all_words_with_articles: &[&'a str],
    map_non_article_index: &F,
    map_non_article_end: &G,
) -> Result<bool, CardTextError>
where
    F: Fn(usize) -> Option<usize>,
    G: Fn(usize) -> Option<usize>,
{
    let Some(named_idx) = lower_words_find_index(all_words.as_slice(), |word| word == "named")
    else {
        return Ok(false);
    };
    let (name, name_end) = extract_name_clause_text(
        all_words.as_slice(),
        all_words_with_articles,
        named_idx,
        1,
        map_non_article_index,
        map_non_article_end,
        "named",
    )?;
    filter.name = Some(name);
    remove_word_range(all_words, named_idx, name_end);
    Ok(true)
}

fn parse_entered_since_your_last_turn_ended_words(words: &[&str]) -> Option<usize> {
    if let Some((_, consumed)) = parse_filter_prefix_words(
        words,
        (
            primitives::word_slice_eq("that"),
            primitives::word_slice_eq("entered"),
            primitives::word_slice_eq("since"),
            primitives::word_slice_eq("your"),
            primitives::word_slice_eq("last"),
            primitives::word_slice_eq("turn"),
            primitives::word_slice_eq("ended"),
        )
            .void(),
    ) {
        return Some(consumed);
    }
    parse_filter_prefix_words(
        words,
        (
            primitives::word_slice_eq("entered"),
            primitives::word_slice_eq("since"),
            primitives::word_slice_eq("your"),
            primitives::word_slice_eq("last"),
            primitives::word_slice_eq("turn"),
            primitives::word_slice_eq("ended"),
        )
            .void(),
    )
    .map(|(_, consumed)| consumed)
}

fn try_apply_entered_since_your_last_turn_ended_clause(
    filter: &mut ObjectFilter,
    all_words: &mut Vec<&str>,
) -> bool {
    let Some((idx, consumed)) = find_filter_prefix_consumed(
        all_words.as_slice(),
        parse_entered_since_your_last_turn_ended_words,
    ) else {
        return false;
    };
    filter.entered_since_your_last_turn_ended = true;
    all_words.drain(idx..idx + consumed);
    true
}

fn strip_object_filter_face_state_words(filter: &mut ObjectFilter, all_words: &mut Vec<&str>) {
    let mut idx = 0usize;
    while idx < all_words.len() {
        let Some((face_down, consumed)) = parse_filter_face_state_words(&all_words[idx..]) else {
            idx += 1;
            continue;
        };
        filter.face_down = Some(face_down);
        all_words.drain(idx..idx + consumed);
    }
}

fn strip_single_graveyard_phrase(filter: &mut ObjectFilter, all_words: &mut Vec<&str>) {
    while let Some(idx) =
        find_word_slice_phrase_start(all_words.as_slice(), &["single", "graveyard"])
    {
        filter.single_graveyard = true;
        all_words.remove(idx);
    }
}

fn parse_color_count_phrase_words(words: &[&str]) -> Option<(&'static str, usize)> {
    parse_filter_prefix_words(
        words,
        (
            alt((
                primitives::word_slice_eq("one").value("one"),
                primitives::word_slice_eq("two").value("two"),
                primitives::word_slice_eq("three").value("three"),
                primitives::word_slice_eq("four").value("four"),
                primitives::word_slice_eq("five").value("five"),
            )),
            primitives::word_slice_eq("or"),
            primitives::word_slice_eq("more"),
            alt((
                primitives::word_slice_eq("color"),
                primitives::word_slice_eq("colors"),
            )),
        )
            .map(|(count, _, _, _)| count),
    )
}

fn try_apply_color_count_phrase(
    filter: &mut ObjectFilter,
    all_words: &mut Vec<&str>,
) -> Result<bool, CardTextError> {
    let Some((color_count_idx, (count_word, consumed))) =
        all_words.iter().enumerate().find_map(|(idx, _)| {
            parse_color_count_phrase_words(&all_words[idx..]).map(|matched| (idx, matched))
        })
    else {
        return Ok(false);
    };

    if count_word == "one" {
        let any_color: ColorSet = Color::ALL.into_iter().collect();
        filter.colors = Some(any_color);
        all_words.drain(color_count_idx..color_count_idx + consumed);
        return Ok(true);
    }

    Err(CardTextError::ParseError(format!(
        "unsupported color-count object filter (clause: '{}')",
        all_words.join(" ")
    )))
}

fn try_apply_pt_literal_prefix(filter: &mut ObjectFilter, all_words: &mut Vec<&str>) -> bool {
    let Some((power, toughness)) = all_words
        .first()
        .and_then(|word| parse_unsigned_pt_word(word))
    else {
        return false;
    };
    filter.power = Some(crate::filter::Comparison::Equal(power));
    filter.toughness = Some(crate::filter::Comparison::Equal(toughness));
    all_words.remove(0);
    true
}

fn parse_not_all_colors_words(words: &[&str]) -> Option<usize> {
    if let Some((_, consumed)) = parse_filter_prefix_words(
        words,
        (
            primitives::word_slice_eq("that"),
            primitives::word_slice_eq("isnt"),
            primitives::word_slice_eq("all"),
            primitives::word_slice_eq("colors"),
        )
            .void(),
    ) {
        return Some(consumed);
    }
    parse_filter_prefix_words(
        words,
        (
            primitives::word_slice_eq("isnt"),
            primitives::word_slice_eq("all"),
            primitives::word_slice_eq("colors"),
        )
            .void(),
    )
    .map(|(_, consumed)| consumed)
}

fn try_apply_not_all_colors_clause(filter: &mut ObjectFilter, all_words: &mut Vec<&str>) -> bool {
    let Some((idx, consumed)) =
        find_filter_prefix_consumed(all_words.as_slice(), parse_not_all_colors_words)
    else {
        return false;
    };
    filter.all_colors = Some(false);
    all_words.drain(idx..idx + consumed);
    true
}

fn parse_not_exactly_two_colors_words(words: &[&str]) -> Option<usize> {
    if let Some((_, consumed)) = parse_filter_prefix_words(
        words,
        (
            primitives::word_slice_eq("that"),
            primitives::word_slice_eq("isnt"),
            primitives::word_slice_eq("exactly"),
            primitives::word_slice_eq("two"),
            primitives::word_slice_eq("colors"),
        )
            .void(),
    ) {
        return Some(consumed);
    }
    parse_filter_prefix_words(
        words,
        (
            primitives::word_slice_eq("isnt"),
            primitives::word_slice_eq("exactly"),
            primitives::word_slice_eq("two"),
            primitives::word_slice_eq("colors"),
        )
            .void(),
    )
    .map(|(_, consumed)| consumed)
}

fn try_apply_not_exactly_two_colors_clause(
    filter: &mut ObjectFilter,
    all_words: &mut Vec<&str>,
) -> bool {
    let Some((idx, consumed)) =
        find_filter_prefix_consumed(all_words.as_slice(), parse_not_exactly_two_colors_words)
    else {
        return false;
    };
    filter.exactly_two_colors = Some(false);
    all_words.drain(idx..idx + consumed);
    true
}

fn parse_mana_value_eq_counters_on_source_words(
    words: &[&str],
) -> Option<(crate::object::CounterType, usize)> {
    let window = words.get(..12)?;
    if window[0] != "with"
        || window[1] != "mana"
        || window[2] != "value"
        || window[3] != "equal"
        || window[4] != "to"
        || window[5] != "number"
        || window[6] != "of"
        || !matches!(window[8], "counter" | "counters")
        || window[9] != "on"
        || window[10] != "this"
        || window[11] != "artifact"
    {
        return None;
    }
    let counter_type = parse_counter_type_word(window[7])?;
    Some((counter_type, 12))
}

fn try_apply_mana_value_eq_counters_on_source_clause(
    filter: &mut ObjectFilter,
    all_words: &mut Vec<&str>,
    segment_tokens: &mut Vec<OwnedLexToken>,
) -> bool {
    let Some((idx, (counter_type, consumed))) =
        all_words.iter().enumerate().find_map(|(idx, _)| {
            parse_mana_value_eq_counters_on_source_words(&all_words[idx..])
                .map(|matched| (idx, matched))
        })
    else {
        return false;
    };
    filter.mana_value_eq_counters_on_source = Some(counter_type);
    all_words.drain(idx..idx + consumed);

    let segment_words_view = GrammarFilterNormalizedWords::new(segment_tokens.as_slice());
    let segment_words = segment_words_view.to_word_refs();
    let segment_match = find_mana_value_equal_counter_phrase_bounds(&segment_words);
    if let Some((start_word_idx, end_word_idx)) = segment_match
        && let Some(start_token_idx) =
            normalized_token_index_for_word_index(segment_tokens.as_slice(), start_word_idx)
    {
        let end_token_idx =
            normalized_token_index_after_words(segment_tokens.as_slice(), end_word_idx)
                .unwrap_or(segment_tokens.len());
        if start_token_idx < end_token_idx && end_token_idx <= segment_tokens.len() {
            segment_tokens.drain(start_token_idx..end_token_idx);
        }
    }

    true
}

fn try_apply_attached_exclusion_phrases(filter: &mut ObjectFilter, all_words: &mut Vec<&str>) {
    let mut idx = 0usize;
    while idx + 2 < all_words.len() {
        if all_words[idx] != "other" || all_words[idx + 1] != "than" {
            idx += 1;
            continue;
        }

        let Some(tag) = (match all_words.get(idx + 2).copied() {
            Some("enchanted") => Some(TagKey::from("enchanted")),
            Some("equipped") => Some(TagKey::from("equipped")),
            _ => None,
        }) else {
            idx += 1;
            continue;
        };

        let mut drain_end = idx + 3;
        if all_words
            .get(drain_end)
            .is_some_and(|word| is_demonstrative_object_head(word))
        {
            drain_end += 1;
        }
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag,
            relation: TaggedOpbjectRelation::IsNotTaggedObject,
        });
        all_words.drain(idx..drain_end);
    }
}

fn strip_object_filter_leading_prefixes(all_words: &mut Vec<&str>) {
    while all_words.len() >= 2 && all_words[0] == "one" && all_words[1] == "of" {
        all_words.drain(0..2);
    }
    while all_words.len() >= 3
        && all_words[0] == "different"
        && all_words[1] == "one"
        && all_words[2] == "of"
    {
        all_words.drain(0..3);
    }
    while all_words
        .first()
        .is_some_and(|word| matches!(*word, "of" | "from"))
    {
        all_words.remove(0);
    }
}

fn parse_spell_filter_power_or_toughness_words(words: &[&str]) -> Option<usize> {
    parse_filter_prefix_words(
        words,
        (
            primitives::word_slice_eq("power"),
            primitives::word_slice_eq("or"),
            primitives::word_slice_eq("toughness"),
        ),
    )
    .map(|(_, consumed)| consumed)
}

fn apply_spell_filter_word_atoms(filter: &mut ObjectFilter, words: &[&str]) {
    let mut idx = 0usize;
    while idx < words.len() {
        if let Some((kind, consumed)) = parse_alternative_cast_words(&words[idx..]) {
            filter.alternative_cast = Some(kind);
            idx += consumed;
            continue;
        }

        let word = words[idx];
        if let Some((face_down, consumed)) = parse_filter_face_state_words(&words[idx..]) {
            filter.face_down = Some(face_down);
            idx += consumed;
            continue;
        }
        if let Some(card_type) = parse_card_type(word) {
            push_unique_filter_value(&mut filter.card_types, card_type);
        }
        if let Some(card_type) = parse_non_type(word) {
            push_unique_filter_value(&mut filter.excluded_card_types, card_type);
        }
        if let Some(subtype) = parse_subtype_flexible(word) {
            push_unique_filter_value(&mut filter.subtypes, subtype);
        }
        if let Some(color) = parse_color(word) {
            let existing = filter.colors.unwrap_or(ColorSet::new());
            filter.colors = Some(existing.union(color));
        }
        idx += 1;
    }
}

fn apply_spell_filter_comparisons(
    filter: &mut ObjectFilter,
    words: &[&str],
    clause_words: &[&str],
) {
    let mut cmp_idx = 0usize;
    while cmp_idx < words.len() {
        let Some((axis, axis_word_count)) =
            parse_spell_filter_comparison_axis_words(&words[cmp_idx..])
        else {
            cmp_idx += 1;
            continue;
        };

        let value_tokens = if cmp_idx + axis_word_count < words.len() {
            &words[cmp_idx + axis_word_count..]
        } else {
            &[]
        };
        let parsed = parse_filter_comparison_tokens(axis.as_str(), value_tokens, clause_words)
            .ok()
            .flatten();
        let Some((cmp, consumed)) = parsed else {
            cmp_idx += 1;
            continue;
        };

        axis.assign(filter, cmp);
        cmp_idx += axis_word_count + consumed;
    }
}

fn build_spell_filter_power_or_toughness_disjunction(
    filter: &ObjectFilter,
    words: &[&str],
    clause_words: &[&str],
) -> Option<ObjectFilter> {
    for idx in 0..words.len() {
        let Some(consumed) = parse_spell_filter_power_or_toughness_words(&words[idx..]) else {
            continue;
        };
        let value_tokens = if idx + consumed < words.len() {
            &words[idx + consumed..]
        } else {
            &[]
        };
        let Some((cmp, _)) = parse_filter_comparison_tokens("power", value_tokens, clause_words)
            .ok()
            .flatten()
        else {
            continue;
        };

        let mut base = filter.clone();
        base.any_of.clear();
        base.power = None;
        base.toughness = None;

        let mut power_branch = base.clone();
        power_branch.power = Some(cmp.clone());

        let mut toughness_branch = base;
        toughness_branch.toughness = Some(cmp);

        let mut disjunction = ObjectFilter::default();
        disjunction.any_of = vec![power_branch, toughness_branch];
        return Some(disjunction);
    }

    None
}

fn parse_spell_filter_from_words(words: &[&str]) -> ObjectFilter {
    let mut filter = ObjectFilter::default();

    apply_spell_filter_word_atoms(&mut filter, words);
    apply_spell_filter_comparisons(&mut filter, words, words);
    apply_spell_filter_parity_phrases(words, &mut filter);

    build_spell_filter_power_or_toughness_disjunction(&filter, words, words).unwrap_or(filter)
}

fn parse_with_no_abilities_words(words: &[&str]) -> Option<usize> {
    parse_filter_prefix_words(
        words,
        (
            primitives::word_slice_eq("no"),
            alt((
                primitives::word_slice_eq("ability"),
                primitives::word_slice_eq("abilities"),
            )),
        ),
    )
    .map(|(_, consumed)| consumed)
}

fn try_apply_with_clause_tail(filter: &mut ObjectFilter, words: &[&str]) -> Option<usize> {
    if let Some(consumed) = parse_with_no_abilities_words(words) {
        filter.no_abilities = true;
        return Some(consumed);
    }

    if let Some((_, no_consumed)) =
        parse_filter_prefix_words(words, primitives::word_slice_eq("no"))
        && let Some((counter_constraint, consumed)) =
            parse_filter_counter_constraint_words(&words[no_consumed..])
    {
        filter.without_counter = Some(counter_constraint);
        return Some(no_consumed + consumed);
    }

    if let Some((kind, consumed)) = parse_alternative_cast_words(words) {
        filter.alternative_cast = Some(kind);
        return Some(consumed);
    }
    if let Some((counter_constraint, consumed)) = parse_filter_counter_constraint_words(words) {
        filter.with_counter = Some(counter_constraint);
        return Some(consumed);
    }

    if let Some((constraint, consumed)) = parse_filter_keyword_constraint_words(words) {
        if let Some((_, or_consumed)) =
            parse_filter_prefix_words(&words[consumed..], primitives::word_slice_eq("or"))
            && let Some((rhs_constraint, rhs_consumed)) =
                parse_filter_keyword_constraint_words(&words[consumed + or_consumed..])
        {
            let mut left = ObjectFilter::default();
            apply_filter_keyword_constraint(&mut left, constraint, false);
            let mut right = ObjectFilter::default();
            apply_filter_keyword_constraint(&mut right, rhs_constraint, false);
            filter.any_of = vec![left, right];
            return Some(consumed + or_consumed + rhs_consumed);
        }

        apply_filter_keyword_constraint(filter, constraint, false);
        return Some(consumed);
    }

    None
}

fn try_apply_without_clause_tail(filter: &mut ObjectFilter, words: &[&str]) -> Option<usize> {
    if let Some((constraint, consumed)) = parse_filter_keyword_constraint_words(words) {
        apply_filter_keyword_constraint(filter, constraint, true);
        return Some(consumed);
    }
    if let Some((counter_constraint, consumed)) = parse_filter_counter_constraint_words(words) {
        filter.without_counter = Some(counter_constraint);
        return Some(consumed);
    }

    None
}

fn apply_spell_filter_parity_phrases(words: &[&str], filter: &mut ObjectFilter) {
    for (parity, phrases) in [
        (
            crate::filter::ParityRequirement::Odd,
            &[
                &["odd", "mana", "value"][..],
                &["odd", "mana", "values"][..],
            ][..],
        ),
        (
            crate::filter::ParityRequirement::Even,
            &[
                &["even", "mana", "value"][..],
                &["even", "mana", "values"][..],
            ][..],
        ),
    ] {
        if phrases
            .iter()
            .any(|phrase| find_word_slice_phrase_start(words, phrase).is_some())
        {
            filter.mana_value_parity = Some(parity);
        }
    }

    for (parity, phrases) in [
        (
            crate::filter::ParityRequirement::Odd,
            &[&["odd", "power"][..]][..],
        ),
        (
            crate::filter::ParityRequirement::Even,
            &[&["even", "power"][..]][..],
        ),
    ] {
        if phrases
            .iter()
            .any(|phrase| find_word_slice_phrase_start(words, phrase).is_some())
        {
            filter.power_parity = Some(parity);
        }
    }
}

fn contains_any_filter_phrase(words: &[&str], phrases: &[&[&str]]) -> bool {
    phrases
        .iter()
        .any(|phrase| find_word_slice_phrase_start(words, phrase).is_some())
}

fn find_any_filter_phrase_start(words: &[&str], phrases: &[&[&str]]) -> Option<usize> {
    phrases
        .iter()
        .find_map(|phrase| find_word_slice_phrase_start(words, phrase))
}

fn find_mana_value_equal_counter_phrase_bounds(words: &[&str]) -> Option<(usize, usize)> {
    (0..words.len()).find_map(|idx| {
        let tail = &words[idx..];
        if tail.len() >= 13
            && find_word_slice_phrase_start(
                tail,
                &[
                    "with", "mana", "value", "equal", "to", "the", "number", "of",
                ],
            ) == Some(0)
            && parse_counter_type_word(tail[8]).is_some()
            && matches!(tail[9], "counter" | "counters")
            && tail[10] == "on"
            && tail[11] == "this"
            && tail[12] == "artifact"
        {
            return Some((idx, idx + 13));
        }
        if tail.len() >= 12
            && find_word_slice_phrase_start(
                tail,
                &["with", "mana", "value", "equal", "to", "number", "of"],
            ) == Some(0)
            && parse_counter_type_word(tail[7]).is_some()
            && matches!(tail[8], "counter" | "counters")
            && tail[9] == "on"
            && tail[10] == "this"
            && tail[11] == "artifact"
        {
            return Some((idx, idx + 12));
        }
        None
    })
}

fn contains_filter_word(words: &[&str], word: &str) -> bool {
    find_word_slice_phrase_start(words, &[word]).is_some()
}

fn starts_with_any_filter_phrase(words: &[&str], phrases: &[&[&str]]) -> bool {
    phrases
        .iter()
        .any(|phrase| find_word_slice_phrase_start(words, phrase) == Some(0))
}

fn attacking_player_filter_from_words(
    words: &[&str],
    pronoun_player_filter: &PlayerFilter,
) -> Option<PlayerFilter> {
    if contains_any_filter_phrase(
        words,
        &[
            &["attacking", "that", "player"],
            &["attacking", "that", "players"],
        ],
    ) {
        return Some(PlayerFilter::IteratedPlayer);
    }
    if contains_any_filter_phrase(
        words,
        &[
            &["attacking", "defending", "player"],
            &["attacking", "defending", "players"],
        ],
    ) {
        return Some(PlayerFilter::Defending);
    }
    if contains_any_filter_phrase(
        words,
        &[
            &["attacking", "target", "player"],
            &["attacking", "target", "players"],
        ],
    ) {
        return Some(PlayerFilter::target_player());
    }
    if contains_any_filter_phrase(
        words,
        &[
            &["attacking", "target", "opponent"],
            &["attacking", "target", "opponents"],
        ],
    ) {
        return Some(PlayerFilter::target_opponent());
    }
    if contains_any_filter_phrase(words, &[&["attacking", "you"]]) {
        return Some(PlayerFilter::You);
    }
    if contains_any_filter_phrase(words, &[&["attacking", "them"]]) {
        return Some(pronoun_player_filter.clone());
    }
    if contains_any_filter_phrase(
        words,
        &[&["attacking", "opponent"], &["attacking", "opponents"]],
    ) {
        return Some(PlayerFilter::Opponent);
    }

    None
}

struct ReferenceTagStageResult {
    source_linked_exile_reference: bool,
    early_return: bool,
}

fn apply_reference_and_tag_stage(
    filter: &mut ObjectFilter,
    all_words: &mut Vec<&str>,
    segment_tokens: &mut Vec<OwnedLexToken>,
) -> ReferenceTagStageResult {
    if all_words.first().is_some_and(|word| *word == "equipped") {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: TagKey::from("equipped"),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
        all_words.remove(0);
    } else if all_words.first().is_some_and(|word| *word == "enchanted") {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: TagKey::from("enchanted"),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
        all_words.remove(0);
    }

    if is_source_reference_words(all_words) {
        filter.source = true;
    }

    if let Some(its_attached_idx) =
        find_word_slice_phrase_start(all_words, &["its", "attached", "to"])
    {
        let mut normalized = Vec::with_capacity(all_words.len() + 1);
        normalized.extend_from_slice(&all_words[..its_attached_idx]);
        normalized.extend(["attached", "to", "it"]);
        normalized.extend_from_slice(&all_words[its_attached_idx + 3..]);
        *all_words = normalized;
    }

    if let Some(attached_idx) = lower_words_find_index(all_words, |word| word == "attached")
        && all_words.get(attached_idx + 1) == Some(&"to")
    {
        let attached_to_words = &all_words[attached_idx + 2..];
        let references_it = starts_with_any_filter_phrase(
            attached_to_words,
            &[
                &["it"],
                &["that", "object"],
                &["that", "creature"],
                &["that", "permanent"],
                &["that", "equipment"],
                &["that", "aura"],
            ],
        );
        if references_it {
            let trim_start = if attached_idx >= 2
                && all_words[attached_idx - 2] == "that"
                && matches!(all_words[attached_idx - 1], "were" | "was" | "is" | "are")
            {
                attached_idx - 2
            } else {
                attached_idx
            };
            all_words.truncate(trim_start);
            filter.tagged_constraints.push(TaggedObjectConstraint {
                tag: IT_TAG.into(),
                relation: TaggedOpbjectRelation::AttachedToTaggedObject,
            });
        }
    }

    if let Some(relation_idx) = find_any_filter_phrase_start(
        all_words,
        &[
            &["blocking", "or", "blocked", "by", "this", "creature"],
            &["blocking", "or", "blocked", "by", "this", "permanent"],
            &["blocking", "or", "blocked", "by", "this", "source"],
        ],
    ) {
        filter.in_combat_with_source = true;
        all_words.truncate(relation_idx);
    }

    let starts_with_exiled_card =
        starts_with_any_filter_phrase(all_words, &[&["exiled", "card"], &["exiled", "cards"]]);
    if starts_with_exiled_card {
        filter.zone.get_or_insert(Zone::Exile);
    }
    let has_exiled_with_phrase =
        find_word_slice_phrase_start(all_words, &["exiled", "with"]).is_some();
    let owner_only_tail_after_exiled_cards = starts_with_exiled_card
        && all_words
            .iter()
            .skip(2)
            .all(|word| matches!(*word, "you" | "your" | "they" | "their" | "own" | "owns"));
    let is_source_linked_exile_reference = has_exiled_with_phrase
        || (starts_with_exiled_card
            && (all_words.len() == 2 || owner_only_tail_after_exiled_cards));
    let mut source_linked_exile_reference = false;
    if is_source_linked_exile_reference {
        source_linked_exile_reference = true;
        filter.zone = Some(Zone::Exile);
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: TagKey::from(crate::tag::SOURCE_EXILED_TAG),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
        if let Some(exiled_with_idx) = find_word_slice_phrase_start(all_words, &["exiled", "with"])
        {
            let mut reference_end = exiled_with_idx + 2;
            if all_words
                .get(reference_end)
                .is_some_and(|word| matches!(*word, "this" | "that" | "the" | "it" | "them"))
            {
                reference_end += 1;
            }
            if all_words.get(reference_end).is_some_and(|word| {
                matches!(
                    *word,
                    "artifact" | "creature" | "permanent" | "card" | "spell" | "source"
                )
            }) {
                reference_end += 1;
            }
            if reference_end > exiled_with_idx + 1 {
                all_words.drain(exiled_with_idx + 1..reference_end);
            }
        }
        let segment_words_view = GrammarFilterNormalizedWords::new(segment_tokens.as_slice());
        let segment_words = segment_words_view.to_word_refs();
        if let Some(exiled_with_idx) =
            find_word_slice_phrase_start(&segment_words, &["exiled", "with"])
            && let Some(exiled_with_token_idx) =
                normalized_token_index_for_word_index(segment_tokens.as_slice(), exiled_with_idx)
        {
            let mut reference_end = exiled_with_token_idx + 2;
            if segment_tokens.get(reference_end).is_some_and(|token| {
                token.is_word("this")
                    || token.is_word("that")
                    || token.is_word("the")
                    || token.is_word("it")
                    || token.is_word("them")
            }) {
                reference_end += 1;
            }
            if segment_tokens.get(reference_end).is_some_and(|token| {
                token.is_word("artifact")
                    || token.is_word("creature")
                    || token.is_word("permanent")
                    || token.is_word("card")
                    || token.is_word("spell")
                    || token.is_word("source")
            }) {
                reference_end += 1;
            }
            if reference_end > exiled_with_idx + 1 {
                segment_tokens.drain(exiled_with_token_idx + 1..reference_end);
            }
        }
    }

    if all_words
        .first()
        .is_some_and(|word| *word == "it" || *word == "them")
    {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: IT_TAG.into(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
        if all_words.len() == 1 {
            return ReferenceTagStageResult {
                source_linked_exile_reference,
                early_return: true,
            };
        }
        all_words.remove(0);
    }

    let has_share_card_type = (contains_filter_word(all_words, "share")
        || contains_filter_word(all_words, "shares"))
        && (contains_filter_word(all_words, "card")
            || contains_filter_word(all_words, "permanent"))
        && contains_filter_word(all_words, "type")
        && contains_filter_word(all_words, "it");
    let has_share_color = contains_filter_word(all_words, "shares")
        && contains_filter_word(all_words, "color")
        && contains_filter_word(all_words, "it");
    let has_same_mana_value =
        contains_any_filter_phrase(all_words, &[&["same", "mana", "value", "as"]]);
    let has_equal_or_lesser_mana_value =
        contains_any_filter_phrase(all_words, &[&["equal", "or", "lesser", "mana", "value"]]);
    let has_lte_mana_value_as_tagged = contains_any_filter_phrase(
        all_words,
        &[
            &[
                "equal", "or", "lesser", "mana", "value", "than", "that", "spell",
            ],
            &[
                "equal", "or", "lesser", "mana", "value", "than", "that", "card",
            ],
            &[
                "equal", "or", "lesser", "mana", "value", "than", "that", "object",
            ],
            &[
                "less", "than", "or", "equal", "to", "that", "spells", "mana", "value",
            ],
            &[
                "less", "than", "or", "equal", "to", "that", "cards", "mana", "value",
            ],
            &[
                "less", "than", "or", "equal", "to", "that", "objects", "mana", "value",
            ],
        ],
    ) || has_equal_or_lesser_mana_value;
    let has_lt_mana_value_as_tagged =
        contains_any_filter_phrase(all_words, &[&["lesser", "mana", "value"]])
            && !has_equal_or_lesser_mana_value;
    let references_sacrifice_cost_object = contains_any_filter_phrase(
        all_words,
        &[
            &["the", "sacrificed", "creature"],
            &["the", "sacrificed", "artifact"],
            &["the", "sacrificed", "permanent"],
            &["a", "sacrificed", "creature"],
            &["a", "sacrificed", "artifact"],
            &["a", "sacrificed", "permanent"],
            &["sacrificed", "creature"],
            &["sacrificed", "artifact"],
            &["sacrificed", "permanent"],
        ],
    );
    let references_it_for_mana_value = all_words.iter().any(|word| matches!(*word, "it" | "its"))
        || contains_any_filter_phrase(
            all_words,
            &[
                &["that", "object"],
                &["that", "creature"],
                &["that", "artifact"],
                &["that", "permanent"],
                &["that", "spell"],
                &["that", "card"],
            ],
        );
    let has_same_name_as_tagged_object = contains_any_filter_phrase(
        all_words,
        &[
            &["same", "name", "as", "that", "spell"],
            &["same", "name", "as", "that", "card"],
            &["same", "name", "as", "that", "object"],
            &["same", "name", "as", "that", "creature"],
            &["same", "name", "as", "that", "permanent"],
        ],
    );

    if has_share_card_type {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: IT_TAG.into(),
            relation: TaggedOpbjectRelation::SharesCardType,
        });
    }
    if has_share_color {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: IT_TAG.into(),
            relation: TaggedOpbjectRelation::SharesColorWithTagged,
        });
    }
    if has_same_mana_value && references_sacrifice_cost_object {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: TagKey::from("sacrifice_cost_0"),
            relation: TaggedOpbjectRelation::SameManaValueAsTagged,
        });
    } else if has_same_mana_value && references_it_for_mana_value {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: IT_TAG.into(),
            relation: TaggedOpbjectRelation::SameManaValueAsTagged,
        });
    }
    if has_lte_mana_value_as_tagged
        && (references_it_for_mana_value || has_equal_or_lesser_mana_value)
    {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: IT_TAG.into(),
            relation: TaggedOpbjectRelation::ManaValueLteTagged,
        });
    }
    if has_lt_mana_value_as_tagged {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: IT_TAG.into(),
            relation: TaggedOpbjectRelation::ManaValueLtTagged,
        });
    }
    if has_same_name_as_tagged_object {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: IT_TAG.into(),
            relation: TaggedOpbjectRelation::SameNameAsTagged,
        });
    }

    if contains_any_filter_phrase(
        all_words,
        &[
            &["that", "convoked", "this", "spell"],
            &["that", "convoked", "it"],
        ],
    ) {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: TagKey::from("convoked_this_spell"),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
    }
    if contains_any_filter_phrase(all_words, &[&["that", "crewed", "it", "this", "turn"]]) {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: TagKey::from("crewed_it_this_turn"),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
    }
    if contains_any_filter_phrase(all_words, &[&["that", "saddled", "it", "this", "turn"]]) {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: TagKey::from("saddled_it_this_turn"),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
    }
    if contains_any_filter_phrase(
        all_words,
        &[
            &["army", "you", "amassed"],
            &["amassed", "army"],
            &["amassed", "armys"],
        ],
    ) {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: IT_TAG.into(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
    }
    if contains_any_filter_phrase(
        all_words,
        &[
            &["exiled", "this", "way"],
            &["destroyed", "this", "way"],
            &["sacrificed", "this", "way"],
            &["revealed", "this", "way"],
            &["discarded", "this", "way"],
            &["milled", "this", "way"],
        ],
    ) {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: IT_TAG.into(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
    }

    ReferenceTagStageResult {
        source_linked_exile_reference,
        early_return: false,
    }
}

pub(crate) fn parse_object_filter_with_grammar_entrypoint_lexed(
    tokens: &[OwnedLexToken],
    other: bool,
) -> Result<ObjectFilter, CardTextError> {
    parse_object_filter_lexed(tokens, other)
}

fn parse_object_filter(
    tokens: &[OwnedLexToken],
    other: bool,
) -> Result<ObjectFilter, CardTextError> {
    parse_object_filter_inner(tokens, other, true)
}

fn parse_object_filter_permissive(
    tokens: &[OwnedLexToken],
    other: bool,
) -> Result<ObjectFilter, CardTextError> {
    parse_object_filter_inner(tokens, other, false)
}

fn parse_object_filter_inner(
    tokens: &[OwnedLexToken],
    other: bool,
    strict: bool,
) -> Result<ObjectFilter, CardTextError> {
    let (tokens, vote_winners_only) = trim_vote_winner_suffix(tokens);
    let mut filter = ObjectFilter::default();
    if other {
        filter.other = true;
    }

    let mut target_player: Option<PlayerFilter> = None;
    let mut target_object: Option<ObjectFilter> = None;
    let mut base_tokens: Vec<OwnedLexToken> = tokens.to_vec();
    let mut targets_idx: Option<usize> = None;
    for (idx, token) in tokens.iter().enumerate() {
        if token.is_word("targets") || token.is_word("target") {
            if idx > 0 && tokens[idx - 1].is_word("that") {
                targets_idx = Some(idx);
                break;
            }
        }
    }
    if let Some(targets_idx) = targets_idx {
        let that_idx = targets_idx - 1;
        base_tokens = tokens[..that_idx].to_vec();
        let target_tokens = &tokens[targets_idx + 1..];
        let parse_target_fragment = |fragment_tokens: &[OwnedLexToken]| -> Result<
            (Option<PlayerFilter>, Option<ObjectFilter>),
            CardTextError,
        > {
            let target_words_view = GrammarFilterNormalizedWords::new(fragment_tokens);
            let target_words = target_words_view.to_word_refs();
            if starts_with_any_filter_phrase(&target_words, &[&["you"]]) {
                return Ok((Some(PlayerFilter::You), None));
            }
            if starts_with_any_filter_phrase(&target_words, &[&["opponent"], &["opponents"]]) {
                return Ok((Some(PlayerFilter::Opponent), None));
            }
            if starts_with_any_filter_phrase(&target_words, &[&["player"], &["players"]]) {
                return Ok((Some(PlayerFilter::Any), None));
            }

            let mut target_filter_tokens = fragment_tokens;
            if target_filter_tokens
                .first()
                .is_some_and(|token| token.is_word("target"))
            {
                target_filter_tokens = &target_filter_tokens[1..];
            }
            if target_filter_tokens.is_empty() {
                return Ok((None, None));
            }
            Ok((
                None,
                Some(parse_object_filter_permissive(target_filter_tokens, false)?),
            ))
        };

        let target_words_view = GrammarFilterNormalizedWords::new(target_tokens);
        let target_words = target_words_view.to_word_refs();
        if let Some(or_word_idx) = lower_words_find_index(&target_words, |word| word == "or")
            && let Some(or_token_idx) =
                normalized_token_index_for_word_index(target_tokens, or_word_idx)
        {
            let left_tokens = trim_commas(&target_tokens[..or_token_idx]);
            let right_tokens = trim_commas(&target_tokens[or_token_idx + 1..]);
            let (left_player, left_object) = parse_target_fragment(&left_tokens)?;
            let (right_player, right_object) = parse_target_fragment(&right_tokens)?;
            target_player = left_player.or(right_player);
            target_object = left_object.or(right_object);
            if target_player.is_some() && target_object.is_some() {
                filter.targets_any_of = true;
            }
        } else {
            let (parsed_player, parsed_object) = parse_target_fragment(target_tokens)?;
            target_player = parsed_player;
            target_object = parsed_object;
        }
    }

    // Object filters should not absorb trailing duration clauses such as
    // "... until this enchantment leaves the battlefield".
    if let Some(until_token_idx) = token_find_index(&base_tokens, |token| token.is_word("until"))
        && until_token_idx > 0
    {
        base_tokens.truncate(until_token_idx);
    }

    let not_on_battlefield = strip_not_on_battlefield_phrase(&mut base_tokens);

    // "other than this/it/them ..." marks an exclusion, not an additional
    // type selector. Keep "other" but drop the self-reference tail.
    let mut idx = 0usize;
    while idx + 2 < base_tokens.len() {
        if !(base_tokens[idx].is_word("other") && base_tokens[idx + 1].is_word("than")) {
            idx += 1;
            continue;
        }

        let mut end = idx + 2;
        let starts_with_self_reference = base_tokens[end].is_word("this")
            || base_tokens[end].is_word("it")
            || base_tokens[end].is_word("them");
        if !starts_with_self_reference {
            idx += 1;
            continue;
        }
        end += 1;

        if end < base_tokens.len()
            && base_tokens[end].as_word().is_some_and(|word| {
                matches!(
                    word,
                    "artifact"
                        | "artifacts"
                        | "battle"
                        | "battles"
                        | "card"
                        | "cards"
                        | "creature"
                        | "creatures"
                        | "enchantment"
                        | "enchantments"
                        | "land"
                        | "lands"
                        | "permanent"
                        | "permanents"
                        | "planeswalker"
                        | "planeswalkers"
                        | "spell"
                        | "spells"
                        | "token"
                        | "tokens"
                )
            })
        {
            end += 1;
        }

        base_tokens.drain(idx + 1..end);
    }
    if let Some(mut disjunction) = parse_attached_reference_or_another_disjunction(&base_tokens)? {
        if target_player.is_some() || target_object.is_some() {
            disjunction = disjunction.targeting(target_player.take(), target_object.take());
        }
        return Ok(disjunction);
    }
    let mut segment_tokens = base_tokens.clone();

    let all_words_view = GrammarFilterNormalizedWords::new(&base_tokens);
    let all_words_with_articles: Vec<&str> = all_words_view
        .to_word_refs()
        .into_iter()
        .filter(|word| *word != "instead")
        .collect();

    let map_non_article_index = |non_article_idx: usize| -> Option<usize> {
        let mut seen = 0usize;
        for (idx, word) in all_words_with_articles.iter().enumerate() {
            if is_article(word) {
                continue;
            }
            if seen == non_article_idx {
                return Some(idx);
            }
            seen += 1;
        }
        None
    };

    let map_non_article_end = |non_article_end: usize| -> Option<usize> {
        let mut seen = 0usize;
        for (idx, word) in all_words_with_articles.iter().enumerate() {
            if is_article(word) {
                continue;
            }
            if seen == non_article_end {
                return Some(idx);
            }
            seen += 1;
        }
        if seen == non_article_end {
            return Some(all_words_with_articles.len());
        }
        None
    };

    let mut all_words: Vec<&str> = all_words_with_articles
        .iter()
        .copied()
        .filter(|word| !is_article(word))
        .collect();

    // "that were put there from the battlefield this turn" means the card entered
    // a graveyard from the battlefield this turn.
    try_apply_put_there_from_battlefield_this_turn_clause(
        &mut filter,
        &mut all_words,
        &mut segment_tokens,
    );

    // "legendary or Rat card" (Nashi, Moon's Legacy) is a supertype/subtype disjunction.
    // We parse it by collecting both selectors and then expanding into an `any_of` filter
    // after the normal pass so other shared qualifiers (zone/owner/etc.) are preserved.
    let legendary_or_subtype = find_word_slice_phrase_start(&all_words, &["legendary", "or"])
        .and_then(|idx| all_words.get(idx + 2).copied())
        .and_then(parse_subtype_word);

    // "in a graveyard that was put there from anywhere this turn" (Reenact the Crime)
    // means the card entered a graveyard this turn.
    try_apply_put_there_from_anywhere_this_turn_clause(
        &mut filter,
        &mut all_words,
        &mut segment_tokens,
    );

    // "... graveyard from the battlefield this turn" means the card entered a graveyard
    // from the battlefield this turn.
    try_apply_graveyard_from_battlefield_this_turn_clause(
        &mut filter,
        &mut all_words,
        &mut segment_tokens,
    );

    // "... entered the battlefield ... this turn" marks a battlefield entry this turn.
    try_apply_entered_battlefield_this_turn_clause(
        &mut filter,
        &mut all_words,
        &mut segment_tokens,
    );

    // Avoid treating reference phrases like "... with mana value equal to the number of charge
    // counters on this artifact" as additional type selectors on the filtered object.
    // (Aether Vial: "put a creature card with mana value equal to the number of charge counters
    // on this artifact from your hand onto the battlefield.")
    let _ = try_apply_mana_value_eq_counters_on_source_clause(
        &mut filter,
        &mut all_words,
        &mut segment_tokens,
    );

    try_apply_attached_exclusion_phrases(&mut filter, &mut all_words);

    let _ = try_apply_pt_literal_prefix(&mut filter, &mut all_words);

    strip_object_filter_leading_prefixes(&mut all_words);

    let _ = try_apply_not_all_colors_clause(&mut filter, &mut all_words);

    let _ = try_apply_not_exactly_two_colors_clause(&mut filter, &mut all_words);

    let _ = try_apply_leading_tagged_reference_prefix(&mut filter, &mut all_words);

    let _ = try_apply_entered_since_your_last_turn_ended_clause(&mut filter, &mut all_words);

    strip_object_filter_face_state_words(&mut filter, &mut all_words);

    if contains_any_filter_phrase(&all_words, &[&["entered", "this", "turn"]]) {
        return Err(CardTextError::ParseError(format!(
            "unsupported entered-this-turn object filter (clause: '{}')",
            all_words.join(" ")
        )));
    }
    if contains_any_filter_phrase(
        &all_words,
        &[
            &["counter", "on", "it", "or"],
            &["counter", "on", "them", "or"],
        ],
    ) {
        return Err(CardTextError::ParseError(format!(
            "unsupported counter-state object filter (clause: '{}')",
            all_words.join(" ")
        )));
    }
    strip_single_graveyard_phrase(&mut filter, &mut all_words);

    let _ = try_apply_not_named_clause(
        &mut filter,
        &mut all_words,
        &all_words_with_articles,
        &map_non_article_index,
        &map_non_article_end,
    )?;

    let _ = try_apply_named_clause(
        &mut filter,
        &mut all_words,
        &all_words_with_articles,
        &map_non_article_index,
        &map_non_article_end,
    )?;

    let _ = try_apply_color_count_phrase(&mut filter, &mut all_words)?;
    let has_power_or_toughness_clause = contains_any_filter_phrase(
        &all_words,
        &[&["power", "or", "toughness"], &["toughness", "or", "power"]],
    );
    if has_power_or_toughness_clause
        && !all_words
            .iter()
            .any(|word| matches!(*word, "spell" | "spells"))
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported power-or-toughness object filter (clause: '{}')",
            all_words.join(" ")
        )));
    }
    let reference_stage =
        apply_reference_and_tag_stage(&mut filter, &mut all_words, &mut segment_tokens);
    if reference_stage.early_return {
        return Ok(filter);
    }
    let source_linked_exile_reference = reference_stage.source_linked_exile_reference;

    let references_target_player =
        contains_any_filter_phrase(&all_words, &[&["target", "player"], &["target", "players"]]);
    let references_target_opponent = contains_any_filter_phrase(
        &all_words,
        &[&["target", "opponent"], &["target", "opponents"]],
    );
    let pronoun_player_filter = if references_target_opponent {
        PlayerFilter::target_opponent()
    } else if references_target_player {
        PlayerFilter::target_player()
    } else {
        PlayerFilter::IteratedPlayer
    };

    if let Some(attacking_filter) =
        attacking_player_filter_from_words(&all_words, &pronoun_player_filter)
    {
        filter.attacking_player_or_planeswalker_controlled_by = Some(attacking_filter);
    }

    let is_tagged_spell_reference_at = |idx: usize| {
        all_words
            .get(idx.wrapping_sub(1))
            .is_some_and(|prev| matches!(*prev, "that" | "this" | "its" | "their"))
    };
    let contains_unqualified_spell_word = all_words.iter().enumerate().any(|(idx, word)| {
        matches!(*word, "spell" | "spells") && !is_tagged_spell_reference_at(idx)
    });
    let mentions_ability_word = all_words
        .iter()
        .any(|word| matches!(*word, "ability" | "abilities"));
    if contains_unqualified_spell_word && !mentions_ability_word {
        filter.has_mana_cost = true;
    }

    if !all_words.is_empty() {
        let mut idx = 0usize;
        while idx < all_words.len() {
            let slice = &all_words[idx..];
            if let Some(consumed) =
                try_apply_joint_owner_controller_clause(&mut filter, slice, &pronoun_player_filter)
            {
                idx += consumed.max(1);
                continue;
            }
            if let Some(consumed) = try_apply_chosen_player_graveyard_clause(&mut filter, slice) {
                idx += consumed.max(1);
                continue;
            }
            if let Some(consumed) = try_apply_negated_you_relation_clause(&mut filter, slice) {
                idx += consumed.max(1);
                continue;
            }
            if let Some(consumed) =
                try_apply_player_relation_clause(&mut filter, slice, &pronoun_player_filter)
            {
                idx += consumed.max(1);
                continue;
            }
            idx += 1;
        }
    }

    let mut with_idx = 0usize;
    while with_idx + 1 < all_words.len() {
        if all_words[with_idx] != "with" {
            with_idx += 1;
            continue;
        }

        if let Some(consumed) = try_apply_with_clause_tail(&mut filter, &all_words[with_idx + 1..])
        {
            with_idx += 1 + consumed;
            continue;
        }

        with_idx += 1;
    }

    let mut has_idx = 0usize;
    while has_idx + 1 < all_words.len() {
        if !matches!(all_words[has_idx], "has" | "have") {
            has_idx += 1;
            continue;
        }
        if filter.with_counter.is_none()
            && let Some((counter_constraint, consumed)) =
                parse_filter_counter_constraint_words(&all_words[has_idx + 1..])
        {
            filter.with_counter = Some(counter_constraint);
            has_idx += 1 + consumed;
            continue;
        }
        has_idx += 1;
    }

    let mut without_idx = 0usize;
    while without_idx + 1 < all_words.len() {
        if all_words[without_idx] != "without" {
            without_idx += 1;
            continue;
        }

        if let Some(consumed) =
            try_apply_without_clause_tail(&mut filter, &all_words[without_idx + 1..])
        {
            without_idx += 1 + consumed;
            continue;
        }

        without_idx += 1;
    }

    let has_tap_activated_ability = contains_any_filter_phrase(
        &all_words,
        &[
            &[
                "has",
                "an",
                "activated",
                "ability",
                "with",
                "t",
                "in",
                "its",
                "cost",
            ],
            &[
                "has",
                "activated",
                "ability",
                "with",
                "t",
                "in",
                "its",
                "cost",
            ],
            &[
                "activated",
                "abilities",
                "with",
                "t",
                "in",
                "their",
                "costs",
            ],
        ],
    );
    if has_tap_activated_ability {
        filter.has_tap_activated_ability = true;
    }

    for idx in 0..all_words.len() {
        if let Some(zone) = parse_zone_word(all_words[idx]) {
            let is_reference_zone_for_spell = if contains_unqualified_spell_word {
                idx > 0
                    && matches!(
                        all_words[idx - 1],
                        "controller"
                            | "controllers"
                            | "owner"
                            | "owners"
                            | "its"
                            | "their"
                            | "that"
                            | "this"
                    )
            } else {
                false
            };
            if is_reference_zone_for_spell {
                continue;
            }
            if filter.zone.is_none() {
                filter.zone = Some(zone);
            }
            if idx > 0 {
                match all_words[idx - 1] {
                    "your" => {
                        filter.owner = Some(PlayerFilter::You);
                    }
                    "opponent" | "opponents" => {
                        filter.owner = Some(PlayerFilter::Opponent);
                    }
                    "their" => {
                        filter.owner = Some(pronoun_player_filter.clone());
                    }
                    _ => {}
                }
            }
            if idx > 1 {
                let owner_pair = (all_words[idx - 2], all_words[idx - 1]);
                match owner_pair {
                    ("target", "player") | ("target", "players") => {
                        filter.owner = Some(PlayerFilter::target_player());
                    }
                    ("target", "opponent") | ("target", "opponents") => {
                        filter.owner = Some(PlayerFilter::target_opponent());
                    }
                    ("that", "player") | ("that", "players") => {
                        filter.owner = Some(PlayerFilter::IteratedPlayer);
                    }
                    _ => {}
                }
            }
        }
    }

    let clause_words = all_words.clone();
    for idx in 0..all_words.len() {
        let (is_base_reference, pt_word_idx) = if idx + 4 < all_words.len()
            && all_words[idx] == "base"
            && all_words[idx + 1] == "power"
            && all_words[idx + 2] == "and"
            && all_words[idx + 3] == "toughness"
        {
            (true, idx + 4)
        } else if idx + 3 < all_words.len()
            && all_words[idx] == "power"
            && all_words[idx + 1] == "and"
            && all_words[idx + 2] == "toughness"
            && (idx == 0 || all_words[idx - 1] != "base")
        {
            (false, idx + 3)
        } else {
            continue;
        };

        if let Ok((power, toughness)) = parse_pt_modifier(all_words[pt_word_idx]) {
            filter.power = Some(crate::filter::Comparison::Equal(power));
            filter.toughness = Some(crate::filter::Comparison::Equal(toughness));
            filter.power_reference = if is_base_reference {
                crate::filter::PtReference::Base
            } else {
                crate::filter::PtReference::Effective
            };
            filter.toughness_reference = if is_base_reference {
                crate::filter::PtReference::Base
            } else {
                crate::filter::PtReference::Effective
            };
        }
    }

    let mut idx = 0usize;
    while idx < all_words.len() {
        let axis = match all_words[idx] {
            "power" => Some("power"),
            "toughness" => Some("toughness"),
            "mana" if idx + 1 < all_words.len() && all_words[idx + 1] == "value" => {
                Some("mana value")
            }
            _ => None,
        };
        let Some(axis) = axis else {
            idx += 1;
            continue;
        };
        let is_base_reference = idx > 0 && all_words[idx - 1] == "base";

        let axis_word_count = usize::from(axis == "mana value") + 1;
        let value_tokens = if idx + axis_word_count < all_words.len() {
            &all_words[idx + axis_word_count..]
        } else {
            &[]
        };
        let Some((cmp, consumed)) =
            parse_filter_comparison_tokens(axis, value_tokens, &clause_words)?
        else {
            idx += 1;
            continue;
        };

        match axis {
            "power" => {
                filter.power = Some(cmp);
                filter.power_reference = if is_base_reference {
                    crate::filter::PtReference::Base
                } else {
                    crate::filter::PtReference::Effective
                };
            }
            "toughness" => {
                filter.toughness = Some(cmp);
                filter.toughness_reference = if is_base_reference {
                    crate::filter::PtReference::Base
                } else {
                    crate::filter::PtReference::Effective
                };
            }
            "mana value" => filter.mana_value = Some(cmp),
            _ => {}
        }
        idx += axis_word_count + consumed;
    }

    apply_parity_filter_phrases(&clause_words, &mut filter);

    if contains_any_filter_phrase(
        &clause_words,
        &[&["power", "greater", "than", "its", "base", "power"]],
    ) {
        filter.power_greater_than_base_power = true;
    }

    let mut saw_permanent = false;
    let mut saw_spell = false;
    let mut saw_permanent_type = false;

    let mut saw_subtype = false;
    let mut negated_word_indices = std::collections::HashSet::new();
    let mut negated_historic_indices = std::collections::HashSet::new();
    let is_text_negation_word =
        |word: &str| matches!(word, "not" | "isnt" | "isn't" | "arent" | "aren't");
    for idx in 0..all_words.len().saturating_sub(1) {
        if all_words[idx] != "non" {
            continue;
        }
        let next = all_words[idx + 1];
        if is_outlaw_word(next) {
            push_outlaw_subtypes(&mut filter.excluded_subtypes);
            negated_word_indices.insert(idx + 1);
        }
        if let Some(card_type) = parse_card_type(next)
            && !slice_has(&filter.excluded_card_types, &card_type)
        {
            filter.excluded_card_types.push(card_type);
            negated_word_indices.insert(idx + 1);
        }
        if next == "attacking" {
            filter.nonattacking = true;
            negated_word_indices.insert(idx + 1);
        }
        if next == "blocking" {
            filter.nonblocking = true;
            negated_word_indices.insert(idx + 1);
        }
        if next == "blocked" {
            filter.unblocked = true;
            negated_word_indices.insert(idx + 1);
        }
        if next == "commander" || next == "commanders" {
            filter.noncommander = true;
            negated_word_indices.insert(idx + 1);
        }
        if let Some(color) = parse_color(next) {
            filter.excluded_colors = filter.excluded_colors.union(color);
            negated_word_indices.insert(idx + 1);
        }
        if let Some(subtype) = parse_subtype_flexible(next)
            && !slice_has(&filter.excluded_subtypes, &subtype)
        {
            filter.excluded_subtypes.push(subtype);
            negated_word_indices.insert(idx + 1);
        }
    }
    for idx in 0..all_words.len() {
        if !is_text_negation_word(all_words[idx]) {
            continue;
        }
        let mut target_idx = idx + 1;
        if target_idx >= all_words.len() {
            continue;
        }
        if is_article(all_words[target_idx]) {
            target_idx += 1;
            if target_idx >= all_words.len() {
                continue;
            }
        }

        let negated_word = all_words[target_idx];
        if negated_word == "attacking" {
            filter.nonattacking = true;
            negated_word_indices.insert(target_idx);
        }
        if negated_word == "blocking" {
            filter.nonblocking = true;
            negated_word_indices.insert(target_idx);
        }
        if negated_word == "blocked" {
            filter.unblocked = true;
            negated_word_indices.insert(target_idx);
        }
        if negated_word == "historic" {
            filter.nonhistoric = true;
            negated_historic_indices.insert(target_idx);
        }
        if negated_word == "commander" || negated_word == "commanders" {
            filter.noncommander = true;
            negated_word_indices.insert(target_idx);
        }
        if let Some(card_type) = parse_card_type(negated_word)
            && !slice_has(&filter.excluded_card_types, &card_type)
        {
            filter.excluded_card_types.push(card_type);
            negated_word_indices.insert(target_idx);
        }
        if let Some(supertype) = parse_supertype_word(negated_word)
            && !slice_has(&filter.excluded_supertypes, &supertype)
        {
            filter.excluded_supertypes.push(supertype);
            negated_word_indices.insert(target_idx);
        }
        if let Some(color) = parse_color(negated_word) {
            filter.excluded_colors = filter.excluded_colors.union(color);
            negated_word_indices.insert(target_idx);
        }
        if let Some(subtype) = parse_subtype_flexible(negated_word)
            && !slice_has(&filter.excluded_subtypes, &subtype)
        {
            filter.excluded_subtypes.push(subtype);
            negated_word_indices.insert(target_idx);
        }
    }
    for idx in 0..all_words.len().saturating_sub(1) {
        if all_words[idx] == "not" && all_words[idx + 1] == "historic" {
            filter.nonhistoric = true;
            negated_historic_indices.insert(idx + 1);
        }
    }

    for (idx, word) in all_words.iter().enumerate() {
        let is_negated_word = set_has(&negated_word_indices, &idx);
        match *word {
            "permanent" | "permanents" => saw_permanent = true,
            "spell" | "spells" => {
                if !is_tagged_spell_reference_at(idx) {
                    saw_spell = true;
                }
            }
            "token" | "tokens" => filter.token = true,
            "nontoken" => filter.nontoken = true,
            "other" => filter.other = true,
            "tapped" => filter.tapped = true,
            "untapped" => filter.untapped = true,
            "attacking" if !is_negated_word => filter.attacking = true,
            "nonattacking" => filter.nonattacking = true,
            "blocking" if !is_negated_word => filter.blocking = true,
            "nonblocking" => filter.nonblocking = true,
            "blocked" if !is_negated_word => filter.blocked = true,
            "unblocked" if !is_negated_word => filter.unblocked = true,
            "commander" | "commanders" => {
                let prev = idx.checked_sub(1).and_then(|i| all_words.get(i)).copied();
                let prev2 = idx.checked_sub(2).and_then(|i| all_words.get(i)).copied();
                let negated_by_phrase = prev.is_some_and(is_text_negation_word)
                    || (prev.is_some_and(is_article) && prev2.is_some_and(is_text_negation_word));
                if is_negated_word || negated_by_phrase {
                    filter.noncommander = true;
                } else {
                    filter.is_commander = true;
                    match prev {
                        Some("your") => filter.owner = Some(PlayerFilter::You),
                        Some("opponent") | Some("opponents") => {
                            filter.owner = Some(PlayerFilter::Opponent);
                        }
                        Some("their") => filter.owner = Some(pronoun_player_filter.clone()),
                        _ => {}
                    }
                }
            }
            "noncommander" | "noncommanders" => filter.noncommander = true,
            "nonbasic" => {
                filter = filter.without_supertype(Supertype::Basic);
            }
            "colorless" => filter.colorless = true,
            "multicolored" => filter.multicolored = true,
            "monocolored" => filter.monocolored = true,
            "nonhistoric" => filter.nonhistoric = true,
            "historic" if !set_has(&negated_historic_indices, &idx) => filter.historic = true,
            "modified" if !is_negated_word => filter.modified = true,
            _ => {}
        }

        if is_non_outlaw_word(word) {
            push_outlaw_subtypes(&mut filter.excluded_subtypes);
            continue;
        }

        if set_has(&negated_word_indices, &idx) {
            continue;
        }

        if is_outlaw_word(word) {
            push_outlaw_subtypes(&mut filter.subtypes);
            saw_subtype = true;
            continue;
        }

        if let Some(card_type) = parse_non_type(word) {
            filter.excluded_card_types.push(card_type);
        }

        if let Some(supertype) = parse_non_supertype(word)
            && !slice_has(&filter.excluded_supertypes, &supertype)
        {
            filter.excluded_supertypes.push(supertype);
        }

        if let Some(color) = parse_non_color(word) {
            filter.excluded_colors = filter.excluded_colors.union(color);
        }
        if let Some(subtype) = parse_non_subtype(word)
            && !slice_has(&filter.excluded_subtypes, &subtype)
        {
            filter.excluded_subtypes.push(subtype);
        }

        if let Some(color) = parse_color(word) {
            let existing = filter.colors.unwrap_or(ColorSet::new());
            filter.colors = Some(existing.union(color));
        }

        if let Some(supertype) = parse_supertype_word(word)
            && !slice_has(&filter.supertypes, &supertype)
        {
            filter.supertypes.push(supertype);
        }

        if let Some(card_type) = parse_card_type(word) {
            push_unique(&mut filter.card_types, card_type);
            if is_permanent_type(card_type) {
                saw_permanent_type = true;
            }
        }

        if let Some(subtype) = parse_subtype_flexible(word) {
            push_unique(&mut filter.subtypes, subtype);
            saw_subtype = true;
        }
    }
    if saw_spell && source_linked_exile_reference {
        // "spell ... exiled with this" describes a stack spell with a relation
        // to source-linked exiled cards, not a spell object in exile.
        filter.zone = Some(Zone::Stack);
    }

    let segments = split_lexed_slices_on_or(&segment_tokens);
    let mut segment_types = Vec::new();
    let mut segment_subtypes = Vec::new();
    let mut segment_marker_counts = Vec::new();
    let mut segment_words_lists: Vec<Vec<String>> = Vec::new();

    for segment in &segments {
        let segment_words_view = GrammarFilterNormalizedWords::new(segment);
        let segment_words: Vec<String> = segment_words_view
            .to_word_refs()
            .into_iter()
            .filter(|word| !is_article(word))
            .map(ToString::to_string)
            .collect();
        segment_words_lists.push(segment_words.clone());
        let mut types = Vec::new();
        let mut subtypes = Vec::new();
        for word in &segment_words {
            if let Some(card_type) = parse_card_type(word) {
                push_unique(&mut types, card_type);
            }
            if let Some(subtype) = parse_subtype_flexible(word) {
                push_unique(&mut subtypes, subtype);
            }
        }
        segment_marker_counts.push(types.len() + subtypes.len());
        if !types.is_empty() {
            segment_types.push(types);
        }
        if !subtypes.is_empty() {
            segment_subtypes.push(subtypes);
        }
    }

    if segments.len() > 1 {
        let qualifier_in_all_segments = |qualifier: &str| {
            segment_words_lists
                .iter()
                .all(|segment| segment.iter().any(|word| word == qualifier))
        };
        let shared_leading_qualifier = |qualifier: &str, opposite: &str| {
            if qualifier_in_all_segments(qualifier) {
                return true;
            }
            if all_words.iter().any(|word| *word == opposite) {
                return false;
            }
            let Some(first_segment) = segment_words_lists.first() else {
                return false;
            };
            if !first_segment.iter().any(|word| word == qualifier) {
                return false;
            }
            segment_words_lists
                .iter()
                .skip(1)
                .all(|segment| !segment.iter().any(|word| word == opposite))
        };

        if filter.tapped && !shared_leading_qualifier("tapped", "untapped") {
            filter.tapped = false;
        }
        if filter.untapped && !shared_leading_qualifier("untapped", "tapped") {
            filter.untapped = false;
        }
    }

    if segments.len() > 1 {
        let type_list_candidate = !segment_marker_counts.is_empty()
            && segment_marker_counts.iter().all(|count| *count == 1);

        if type_list_candidate {
            let mut any_types = Vec::new();
            let mut any_subtypes = Vec::new();
            for types in segment_types {
                let Some(card_type) = types.first().copied() else {
                    continue;
                };
                push_unique(&mut any_types, card_type);
            }
            for subtypes in segment_subtypes {
                let Some(subtype) = subtypes.first().copied() else {
                    continue;
                };
                push_unique(&mut any_subtypes, subtype);
            }
            if !any_types.is_empty() {
                filter.card_types = any_types;
            }
            if !any_subtypes.is_empty() {
                filter.subtypes = any_subtypes;
            }
            if !filter.card_types.is_empty() && !filter.subtypes.is_empty() {
                filter.type_or_subtype_union = true;
            }
        }
    } else if let Some(types) = segment_types.into_iter().next() {
        let has_and = contains_filter_word(&all_words, "and");
        let has_or = contains_filter_word(&all_words, "or");
        if types.len() > 1 {
            if has_and && !has_or {
                filter.card_types = types;
            } else {
                filter.all_card_types = types;
            }
        } else if types.len() == 1 {
            filter.card_types = types;
        }
    }

    let permanent_type_defaults = vec![
        CardType::Artifact,
        CardType::Creature,
        CardType::Enchantment,
        CardType::Land,
        CardType::Planeswalker,
        CardType::Battle,
    ];
    let and_segments = split_lexed_slices_on_and(&segment_tokens);
    let and_segment_words_lists: Vec<Vec<String>> = and_segments
        .iter()
        .map(|segment| {
            GrammarFilterNormalizedWords::new(segment)
                .to_word_refs()
                .into_iter()
                .filter(|word| !is_article(word))
                .map(ToString::to_string)
                .collect()
        })
        .collect();

    let segment_has_standalone_spell = |segment: &[String]| {
        let contains_spell = segment
            .iter()
            .any(|word| matches!(word.as_str(), "spell" | "spells"));
        if !contains_spell {
            return false;
        }

        !segment.iter().any(|word| {
            matches!(
                word.as_str(),
                "permanent" | "permanents" | "card" | "cards" | "source" | "sources"
            ) || parse_card_type(word).is_some()
                || parse_subtype_flexible(word).is_some()
        })
    };
    let segment_has_nonspell_permanent_head = |segment: &[String]| {
        let contains_spell = segment
            .iter()
            .any(|word| matches!(word.as_str(), "spell" | "spells"));
        if contains_spell {
            return false;
        }

        segment.iter().any(|word| {
            matches!(word.as_str(), "permanent" | "permanents")
                || parse_card_type(word).is_some_and(is_permanent_type)
                || parse_subtype_flexible(word).is_some()
        })
    };
    let segment_has_permanent_spell_head = |segment: &[String]| {
        if segment.len() < 2 {
            return false;
        }
        let mut idx = 0usize;
        while idx + 1 < segment.len() {
            let permanent = &segment[idx];
            let spell = &segment[idx + 1];
            if (permanent == "permanent" || permanent == "permanents")
                && (spell == "spell" || spell == "spells")
            {
                return true;
            }
            idx += 1;
        }
        false
    };
    let has_standalone_spell_segment = segment_words_lists
        .iter()
        .any(|segment| segment_has_standalone_spell(segment));
    let has_nonspell_permanent_segment = segment_words_lists
        .iter()
        .any(|segment| segment_has_nonspell_permanent_head(segment));
    let has_split_permanent_spell_segments = and_segment_words_lists.len() > 1
        && and_segment_words_lists
            .iter()
            .any(|segment| segment_has_permanent_spell_head(segment))
        && and_segment_words_lists
            .iter()
            .any(|segment| segment_has_nonspell_permanent_head(segment));

    if saw_spell && has_standalone_spell_segment && has_nonspell_permanent_segment {
        let mut spell_filter = filter.clone();
        spell_filter.any_of.clear();
        spell_filter.zone = Some(Zone::Stack);
        spell_filter.card_types.clear();
        spell_filter.all_card_types.clear();
        spell_filter.subtypes.clear();
        spell_filter.type_or_subtype_union = false;

        let mut permanent_filter = filter.clone();
        permanent_filter.any_of.clear();
        permanent_filter.zone = Some(Zone::Battlefield);
        permanent_filter.has_mana_cost = false;
        if permanent_filter.card_types.is_empty()
            && permanent_filter.all_card_types.is_empty()
            && permanent_filter.subtypes.is_empty()
        {
            permanent_filter.card_types = permanent_type_defaults.clone();
        }

        let mut combined_filter = ObjectFilter::default();
        combined_filter.any_of = vec![spell_filter, permanent_filter];
        filter = combined_filter;
    } else if saw_spell && saw_permanent && has_split_permanent_spell_segments {
        let mut spell_filter = filter.clone();
        spell_filter.any_of.clear();
        spell_filter.zone = Some(Zone::Stack);
        spell_filter.has_mana_cost = false;
        if spell_filter.card_types.is_empty()
            && spell_filter.all_card_types.is_empty()
            && spell_filter.subtypes.is_empty()
        {
            spell_filter.card_types = permanent_type_defaults.clone();
        }

        let mut permanent_filter = filter.clone();
        permanent_filter.any_of.clear();
        permanent_filter.zone = Some(Zone::Battlefield);
        permanent_filter.has_mana_cost = false;
        if permanent_filter.card_types.is_empty()
            && permanent_filter.all_card_types.is_empty()
            && permanent_filter.subtypes.is_empty()
        {
            permanent_filter.card_types = permanent_type_defaults.clone();
        }

        let mut combined_filter = ObjectFilter::default();
        combined_filter.any_of = vec![spell_filter, permanent_filter];
        filter = combined_filter;
    } else if saw_spell && saw_permanent {
        if filter.card_types.is_empty() && filter.all_card_types.is_empty() {
            filter.card_types = permanent_type_defaults.clone();
        }
        filter.zone = Some(Zone::Stack);
    } else {
        if saw_permanent && filter.card_types.is_empty() && filter.all_card_types.is_empty() {
            filter.card_types = permanent_type_defaults.clone();
        }
    }

    if filter.any_of.is_empty() {
        if let Some(zone) = filter.zone {
            if saw_spell && zone != Zone::Stack {
                let is_spell_origin_zone = matches!(
                    zone,
                    Zone::Hand | Zone::Graveyard | Zone::Exile | Zone::Library | Zone::Command
                );
                if !is_spell_origin_zone {
                    return Err(CardTextError::ParseError(
                        "spell targets must be on the stack".to_string(),
                    ));
                }
            }
        } else if saw_spell {
            filter.zone = Some(Zone::Stack);
        } else if saw_permanent || saw_permanent_type || saw_subtype {
            filter.zone = Some(Zone::Battlefield);
        }
    }

    if contains_unqualified_spell_word
        && filter.cast_by.is_some()
        && matches!(
            filter.zone,
            Some(Zone::Hand | Zone::Graveyard | Zone::Exile | Zone::Library | Zone::Command)
        )
    {
        filter.owner = None;
    }

    if target_player.is_some() || target_object.is_some() {
        filter = filter.targeting(target_player.take(), target_object.take());
    }

    if let Some(or_subtype) = legendary_or_subtype
        && filter.any_of.is_empty()
        && slice_has(&filter.supertypes, &Supertype::Legendary)
        && slice_has(&filter.subtypes, &or_subtype)
    {
        let mut legendary_branch = filter.clone();
        legendary_branch.any_of.clear();
        legendary_branch
            .subtypes
            .retain(|subtype| *subtype != or_subtype);

        let mut subtype_branch = filter.clone();
        subtype_branch.any_of.clear();
        subtype_branch
            .supertypes
            .retain(|supertype| *supertype != Supertype::Legendary);

        let mut disjunction = ObjectFilter::default();
        disjunction.any_of = vec![legendary_branch, subtype_branch];
        filter = disjunction;
    }

    let owner_or_controller_player = all_words.iter().enumerate().find_map(|(idx, _)| {
        parse_owner_or_controller_disjunction_player(&all_words[idx..], &pronoun_player_filter)
            .map(|(player_filter, _)| player_filter)
    });
    if let Some(player_filter) = owner_or_controller_player
        && filter.any_of.is_empty()
    {
        let mut base = filter.clone();
        base.any_of.clear();
        base.owner = None;
        base.controller = None;

        let mut owner_branch = base.clone();
        owner_branch.owner = Some(player_filter.clone());

        let mut controller_branch = base;
        controller_branch.controller = Some(player_filter);

        let mut disjunction = ObjectFilter::default();
        disjunction.any_of = vec![owner_branch, controller_branch];
        filter = disjunction;
    }

    if has_power_or_toughness_clause && saw_spell {
        let mut power_or_toughness_cmp = None;
        for idx in 0..all_words.len() {
            let (_, value_tokens) = match all_words.get(idx..) {
                Some(["power", "or", "toughness", rest @ ..])
                | Some(["toughness", "or", "power", rest @ ..]) => {
                    (crate::filter::PtReference::Effective, rest)
                }
                _ => continue,
            };
            let Some((cmp, _)) =
                parse_filter_comparison_tokens("power", value_tokens, &clause_words)?
            else {
                continue;
            };
            power_or_toughness_cmp = Some(cmp);
            break;
        }
        if let Some(cmp) = power_or_toughness_cmp {
            let mut base = filter.clone();
            base.any_of.clear();
            base.power = None;
            base.toughness = None;

            let mut power_branch = base.clone();
            power_branch.power = Some(cmp.clone());

            let mut toughness_branch = base;
            toughness_branch.toughness = Some(cmp);

            let mut disjunction = ObjectFilter::default();
            disjunction.any_of = vec![power_branch, toughness_branch];
            filter = disjunction;
        }
    }

    let has_constraints = !filter.card_types.is_empty()
        || !filter.all_card_types.is_empty()
        || !filter.supertypes.is_empty()
        || !filter.excluded_supertypes.is_empty()
        || !filter.excluded_card_types.is_empty()
        || !filter.excluded_subtypes.is_empty()
        || !filter.subtypes.is_empty()
        || filter.zone.is_some()
        || filter.controller.is_some()
        || filter.owner.is_some()
        || filter.other
        || filter.token
        || filter.nontoken
        || filter.face_down.is_some()
        || filter.tapped
        || filter.untapped
        || filter.attacking
        || filter
            .attacking_player_or_planeswalker_controlled_by
            .is_some()
        || filter.nonattacking
        || filter.blocking
        || filter.nonblocking
        || filter.blocked
        || filter.unblocked
        || filter.is_commander
        || filter.noncommander
        || !filter.excluded_colors.is_empty()
        || filter.colorless
        || filter.multicolored
        || filter.monocolored
        || filter.all_colors.is_some()
        || filter.exactly_two_colors.is_some()
        || filter.historic
        || filter.nonhistoric
        || filter.power.is_some()
        || filter.power_parity.is_some()
        || filter.toughness.is_some()
        || filter.mana_value.is_some()
        || filter.mana_value_parity.is_some()
        || filter.name.is_some()
        || filter.excluded_name.is_some()
        || filter.source
        || filter.with_counter.is_some()
        || filter.without_counter.is_some()
        || filter.total_counters_parity.is_some()
        || filter.alternative_cast.is_some()
        || !filter.static_abilities.is_empty()
        || !filter.excluded_static_abilities.is_empty()
        || !filter.ability_markers.is_empty()
        || !filter.excluded_ability_markers.is_empty()
        || !filter.tagged_constraints.is_empty()
        || filter.targets_player.is_some()
        || filter.targets_object.is_some()
        || !filter.any_of.is_empty();

    if !has_constraints {
        return Err(CardTextError::ParseError(format!(
            "unsupported target phrase (clause: '{}')",
            all_words.join(" ")
        )));
    }

    let has_object_identity = !filter.card_types.is_empty()
        || !filter.all_card_types.is_empty()
        || !filter.supertypes.is_empty()
        || !filter.excluded_supertypes.is_empty()
        || !filter.excluded_card_types.is_empty()
        || !filter.excluded_subtypes.is_empty()
        || !filter.subtypes.is_empty()
        || filter.zone.is_some()
        || filter.token
        || filter.nontoken
        || filter.face_down.is_some()
        || filter.tapped
        || filter.untapped
        || filter.attacking
        || filter
            .attacking_player_or_planeswalker_controlled_by
            .is_some()
        || filter.nonattacking
        || filter.blocking
        || filter.nonblocking
        || filter.blocked
        || filter.unblocked
        || filter.is_commander
        || filter.noncommander
        || !filter.excluded_colors.is_empty()
        || filter.colorless
        || filter.multicolored
        || filter.monocolored
        || filter.all_colors.is_some()
        || filter.exactly_two_colors.is_some()
        || filter.historic
        || filter.nonhistoric
        || filter.power.is_some()
        || filter.power_parity.is_some()
        || filter.toughness.is_some()
        || filter.mana_value.is_some()
        || filter.mana_value_parity.is_some()
        || filter.name.is_some()
        || filter.excluded_name.is_some()
        || filter.source
        || filter.with_counter.is_some()
        || filter.without_counter.is_some()
        || filter.total_counters_parity.is_some()
        || filter.alternative_cast.is_some()
        || !filter.static_abilities.is_empty()
        || !filter.excluded_static_abilities.is_empty()
        || !filter.ability_markers.is_empty()
        || !filter.excluded_ability_markers.is_empty()
        || filter.chosen_color
        || filter.chosen_creature_type
        || filter.excluded_chosen_creature_type
        || filter.colors.is_some()
        || !filter.tagged_constraints.is_empty()
        || filter.targets_player.is_some()
        || filter.targets_object.is_some()
        || !filter.any_of.is_empty();
    if !has_object_identity {
        return Err(CardTextError::ParseError(format!(
            "unsupported target phrase lacking object selector (clause: '{}')",
            all_words.join(" ")
        )));
    }

    if vote_winners_only {
        filter = filter.match_tagged(
            TagKey::from(VOTE_WINNERS_TAG),
            TaggedOpbjectRelation::IsTaggedObject,
        );
    }

    if not_on_battlefield && filter.any_of.is_empty() && !matches!(filter.zone, Some(Zone::Stack)) {
        let mut base = filter.clone();
        base.any_of.clear();
        base.zone = None;

        let mut disjunction = ObjectFilter::default();
        disjunction.any_of = [
            Zone::Hand,
            Zone::Library,
            Zone::Graveyard,
            Zone::Exile,
            Zone::Command,
        ]
        .into_iter()
        .map(|zone| {
            let mut branch = base.clone();
            branch.zone = Some(zone);
            branch
        })
        .collect();
        filter = disjunction;
    }

    // Strict mode: detect structural patterns in the input that indicate
    // unconsumed compound content (e.g. "for each card in your hand AND EACH
    // foretold card you own in exile" where the second clause was silently
    // absorbed into the first filter).
    if strict {
        let input_words_view = GrammarFilterNormalizedWords::new(&tokens);
        let input_words: Vec<&str> = input_words_view
            .to_word_refs()
            .into_iter()
            .filter(|word| !is_article(word))
            .collect();

        // "and each" / "and every" signals a compound count source when
        // the word after "each"/"every" introduces a new filter (type word,
        // zone word, etc.) rather than qualifying the current subject
        // (e.g. "and each other creature" is a subject qualifier, but
        // "and each foretold card you own in exile" is a new clause).
        for (idx, word) in input_words.iter().enumerate() {
            if *word != "and" {
                continue;
            }
            let next = input_words.get(idx + 1).copied();
            if !next.is_some_and(|n| matches!(n, "each" | "every")) {
                continue;
            }
            // "and each other" is typically a subject qualifier, allow it.
            let after_each = input_words.get(idx + 2).copied();
            if after_each.is_some_and(|w| matches!(w, "other" | "another")) {
                continue;
            }
            return Err(CardTextError::ParseError(format!(
                "object filter has unconsumed compound clause '{}' (full input: '{}')",
                input_words[idx..].join(" "),
                input_words.join(" "),
            )));
        }

        // "for each" signals a trailing iteration clause that should have
        // been split out by the caller before passing to the filter parser.
        for (idx, word) in input_words.iter().enumerate() {
            if *word == "for"
                && idx > 0
                && input_words.get(idx + 1).is_some_and(|next| *next == "each")
            {
                return Err(CardTextError::ParseError(format!(
                    "object filter has unconsumed 'for each' clause '{}' (full input: '{}')",
                    input_words[idx..].join(" "),
                    input_words.join(" "),
                )));
            }
        }
    }

    Ok(filter)
}

pub(crate) fn parse_object_filter_with_grammar_entrypoint(
    tokens: &[OwnedLexToken],
    other: bool,
) -> Result<ObjectFilter, CardTextError> {
    parse_object_filter(tokens, other)
}

pub(crate) fn parse_spell_filter_with_grammar_entrypoint_lexed(
    tokens: &[OwnedLexToken],
) -> ObjectFilter {
    let words_view = GrammarFilterNormalizedWords::new(tokens);
    let words: Vec<&str> = words_view
        .to_word_refs()
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();

    parse_spell_filter_from_words(&words)
}

pub(crate) fn parse_spell_filter_with_grammar_entrypoint(tokens: &[OwnedLexToken]) -> ObjectFilter {
    let words: Vec<&str> = crate::cards::builders::parser::token_word_refs(tokens)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();

    parse_spell_filter_from_words(&words)
}

fn parse_meld_subject_filter(words: &[&str]) -> Result<ObjectFilter, CardTextError> {
    if words.is_empty() {
        return Err(CardTextError::ParseError(
            "missing meld predicate subject".to_string(),
        ));
    }
    if is_source_reference_words(words) {
        return Ok(ObjectFilter::source());
    }

    let tokens = synth_words_as_tokens(words);
    parse_object_filter(&tokens, false)
        .or_else(|_| Ok(ObjectFilter::default().named(words.join(" "))))
}

fn is_plausible_meld_subject_start(word: &str) -> bool {
    matches!(
        word,
        "a" | "an"
            | "another"
            | "this"
            | "that"
            | "source"
            | "artifact"
            | "battle"
            | "card"
            | "creature"
            | "enchantment"
            | "land"
            | "nonland"
            | "permanent"
            | "planeswalker"
    )
}

fn find_meld_subject_split(words: &[&str]) -> Option<usize> {
    words
        .iter()
        .enumerate()
        .find_map(|(idx, word)| {
            (*word == "and"
                && words
                    .get(idx + 1)
                    .is_some_and(|next| is_plausible_meld_subject_start(next)))
            .then_some(idx)
        })
        .or_else(|| find_index(words, |word| *word == "and"))
}

pub(super) fn parse_predicate(tokens: &[OwnedLexToken]) -> Result<PredicateAst, CardTextError> {
    let raw_words_view = GrammarFilterNormalizedWords::new(tokens);
    let raw_words = raw_words_view.to_word_refs();
    let mut filtered: Vec<&str> = raw_words
        .iter()
        .copied()
        .filter(|word| !is_article(word))
        .collect();

    if filtered.is_empty() {
        return Err(CardTextError::ParseError(
            "empty predicate in if clause".to_string(),
        ));
    }

    for (phrase, zone) in [
        (
            ["this", "card", "is", "in", "your", "hand"].as_slice(),
            Zone::Hand,
        ),
        (
            ["this", "card", "is", "in", "your", "graveyard"].as_slice(),
            Zone::Graveyard,
        ),
        (
            ["this", "card", "is", "in", "your", "library"].as_slice(),
            Zone::Library,
        ),
        (
            ["this", "card", "is", "in", "exile"].as_slice(),
            Zone::Exile,
        ),
        (
            ["this", "card", "is", "in", "the", "command", "zone"].as_slice(),
            Zone::Command,
        ),
    ] {
        if filtered.as_slice() == phrase {
            return Ok(PredicateAst::SourceIsInZone(zone));
        }
    }

    if let Some(predicate) = parse_graveyard_threshold_predicate(&filtered)? {
        return Ok(predicate);
    }

    if let Some(gets_idx) = find_index(&filtered, |word| *word == "gets")
        && gets_idx > 0
        && filtered[gets_idx + 1..] == ["more", "votes"]
    {
        return Ok(PredicateAst::VoteOptionGetsMoreVotes {
            option: filtered[..gets_idx].join(" "),
        });
    }

    if let Some(gets_idx) = find_index(&filtered, |word| *word == "gets")
        && gets_idx > 0
        && filtered[gets_idx + 1..] == ["more", "votes", "or", "vote", "is", "tied"]
    {
        return Ok(PredicateAst::VoteOptionGetsMoreVotesOrTied {
            option: filtered[..gets_idx].join(" "),
        });
    }

    if filtered.len() >= 4
        && filtered[0] == "no"
        && filtered[filtered.len() - 2..] == ["got", "votes"]
    {
        let filter_tokens = filtered[1..filtered.len() - 2]
            .iter()
            .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
            .collect::<Vec<_>>();
        let filter = parse_object_filter(&filter_tokens, false)?;
        return Ok(PredicateAst::NoVoteObjectsMatched { filter });
    }

    if let Some(attacking_idx) = find_word_slice_phrase_start(
        &filtered,
        &[
            "are",
            "attacking",
            "and",
            "you",
            "both",
            "own",
            "and",
            "control",
            "them",
        ],
    ) && let Some(and_idx) = find_meld_subject_split(&filtered[..attacking_idx])
    {
        let left_words = &filtered[..and_idx];
        let right_words = &filtered[and_idx + 1..attacking_idx];
        if !left_words.is_empty() && !right_words.is_empty() {
            let mut left_filter = parse_meld_subject_filter(left_words).map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported attacking meld predicate subject (predicate: '{}')",
                    filtered.join(" ")
                ))
            })?;
            left_filter.controller = Some(PlayerFilter::You);
            left_filter.attacking = true;

            let mut right_filter = parse_meld_subject_filter(right_words).map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported attacking meld predicate tail (predicate: '{}')",
                    filtered.join(" ")
                ))
            })?;
            right_filter.controller = Some(PlayerFilter::You);
            right_filter.attacking = true;

            return Ok(PredicateAst::And(
                Box::new(PredicateAst::PlayerControls {
                    player: PlayerAst::You,
                    filter: left_filter,
                }),
                Box::new(PredicateAst::PlayerControls {
                    player: PlayerAst::You,
                    filter: right_filter,
                }),
            ));
        }
    }

    if filtered.len() >= 8
        && filtered[0] == "you"
        && filtered[1] == "both"
        && filtered[2] == "own"
        && filtered[3] == "and"
        && (filtered[4] == "control" || filtered[4] == "controls")
        && let Some(and_idx) = find_meld_subject_split(&filtered[5..])
    {
        let and_idx = 5 + and_idx;
        if and_idx > 5 && and_idx + 1 < filtered.len() {
            let mut left_filter =
                parse_meld_subject_filter(&filtered[5..and_idx]).map_err(|_| {
                    CardTextError::ParseError(format!(
                        "unsupported own-and-control predicate subject (predicate: '{}')",
                        filtered.join(" ")
                    ))
                })?;
            left_filter.controller = Some(PlayerFilter::You);
            let mut right_filter =
                parse_meld_subject_filter(&filtered[and_idx + 1..]).map_err(|_| {
                    CardTextError::ParseError(format!(
                        "unsupported own-and-control predicate tail (predicate: '{}')",
                        filtered.join(" ")
                    ))
                })?;
            right_filter.controller = Some(PlayerFilter::You);
            return Ok(PredicateAst::And(
                Box::new(PredicateAst::PlayerControls {
                    player: PlayerAst::You,
                    filter: left_filter,
                }),
                Box::new(PredicateAst::PlayerControls {
                    player: PlayerAst::You,
                    filter: right_filter,
                }),
            ));
        }
    }

    if let Some(and_idx) = find_index(&filtered, |word| *word == "and")
        && and_idx > 0
        && and_idx + 1 < filtered.len()
    {
        let right_first = filtered.get(and_idx + 1).copied();
        if matches!(right_first, Some("have") | Some("you")) {
            let left_words = &filtered[..and_idx];
            let mut right_words = filtered[and_idx + 1..].to_vec();
            if right_words.first().copied() == Some("have") {
                right_words.insert(0, "you");
            }
            let left_tokens = left_words
                .iter()
                .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
                .collect::<Vec<_>>();
            let right_tokens = right_words
                .iter()
                .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
                .collect::<Vec<_>>();
            let left = parse_predicate(&left_tokens)?;
            let right = parse_predicate(&right_tokens)?;
            return Ok(PredicateAst::And(Box::new(left), Box::new(right)));
        }
    }

    if filtered.as_slice() == ["this", "tapped"]
        || filtered.as_slice() == ["thiss", "tapped"]
        || ((filtered.first().copied() == Some("this")
            || filtered.first().copied() == Some("thiss"))
            && filtered.last().copied() == Some("tapped"))
    {
        return Ok(PredicateAst::SourceIsTapped);
    }

    if filtered.as_slice() == ["this", "untapped"]
        || filtered.as_slice() == ["thiss", "untapped"]
        || filtered.as_slice() == ["this", "is", "untapped"]
        || filtered.as_slice() == ["this", "creature", "is", "untapped"]
        || filtered.as_slice() == ["this", "permanent", "is", "untapped"]
        || ((filtered.first().copied() == Some("this")
            || filtered.first().copied() == Some("thiss"))
            && filtered.last().copied() == Some("untapped"))
    {
        return Ok(PredicateAst::Not(Box::new(PredicateAst::SourceIsTapped)));
    }

    if filtered.as_slice() == ["this", "creature", "isnt", "saddled"]
        || filtered.as_slice() == ["this", "permanent", "isnt", "saddled"]
        || filtered.as_slice() == ["this", "isnt", "saddled"]
        || filtered.as_slice() == ["it", "isnt", "saddled"]
    {
        return Ok(PredicateAst::Not(Box::new(PredicateAst::SourceIsSaddled)));
    }

    if filtered.as_slice() == ["this", "creature", "is", "saddled"]
        || filtered.as_slice() == ["this", "permanent", "is", "saddled"]
        || filtered.as_slice() == ["this", "is", "saddled"]
        || filtered.as_slice() == ["it", "is", "saddled"]
    {
        return Ok(PredicateAst::SourceIsSaddled);
    }

    if slice_starts_with(&filtered, &["there", "are", "no"])
        && slice_contains(&filtered, &"counters")
        && contains_any_filter_phrase(&filtered, &[&["on", "this"]])
        && let Some(counters_idx) = find_index(&filtered, |word| *word == "counters")
        && counters_idx >= 4
        && let Some(counter_type) = parse_counter_type_word(filtered[counters_idx - 1])
    {
        return Ok(PredicateAst::SourceHasNoCounter(counter_type));
    }

    let source_has_counter_prefix_len = if slice_starts_with(&raw_words, &["this", "has"]) {
        Some(2)
    } else if raw_words.len() >= 3
        && raw_words[0] == "this"
        && matches!(
            raw_words[1],
            "creature"
                | "permanent"
                | "artifact"
                | "enchantment"
                | "land"
                | "planeswalker"
                | "battle"
        )
        && raw_words[2] == "has"
    {
        Some(3)
    } else {
        None
    };
    if let Some(prefix_len) = source_has_counter_prefix_len
        && raw_words.len() >= prefix_len + 4
        && raw_words[prefix_len] == "no"
        && let Some(counter_type) = parse_counter_type_word(raw_words[prefix_len + 1])
        && matches!(raw_words[prefix_len + 2], "counter" | "counters")
        && raw_words[prefix_len + 3] == "on"
        && matches!(
            raw_words.get(prefix_len + 4).copied(),
            Some("it" | "him" | "her" | "them" | "this" | "that")
        )
    {
        return Ok(PredicateAst::SourceHasNoCounter(counter_type));
    }

    let triggering_object_had_no_counter_prefix_len =
        if slice_starts_with(&raw_words, &["it", "had", "no"]) {
            Some(3)
        } else if slice_starts_with(&raw_words, &["this", "creature", "had", "no"])
            || slice_starts_with(&raw_words, &["that", "creature", "had", "no"])
            || slice_starts_with(&raw_words, &["this", "permanent", "had", "no"])
            || slice_starts_with(&raw_words, &["that", "permanent", "had", "no"])
        {
            Some(4)
        } else {
            None
        };
    if let Some(prefix_len) = triggering_object_had_no_counter_prefix_len
        && raw_words.len() >= prefix_len + 4
        && let Some(counter_type) = parse_counter_type_word(raw_words[prefix_len])
        && matches!(raw_words[prefix_len + 1], "counter" | "counters")
        && raw_words[prefix_len + 2] == "on"
        && matches!(
            raw_words[prefix_len + 3],
            "it" | "them" | "this" | "that" | "itself"
        )
    {
        return Ok(PredicateAst::TriggeringObjectHadNoCounter(counter_type));
    }

    if slice_starts_with(&raw_words, &["there", "are"])
        && raw_words.get(3).copied() == Some("or")
        && raw_words.get(4).copied() == Some("more")
        && raw_words
            .iter()
            .any(|w| *w == "counter" || *w == "counters")
    {
        if let Some((count, used)) = parse_number(&tokens[2..]) {
            let rest = &tokens[2 + used..];
            let rest_words = crate::cards::builders::parser::token_word_refs(rest);
            if rest_words.len() >= 4
                && rest_words[0] == "or"
                && rest_words[1] == "more"
                && (rest_words[3] == "counter" || rest_words[3] == "counters")
                && let Some(counter_type) = parse_counter_type_word(rest_words[2])
            {
                return Ok(PredicateAst::SourceHasCounterAtLeast {
                    counter_type,
                    count,
                });
            }
        }
    }

    if let Some(prefix_len) = source_has_counter_prefix_len
        && raw_words.len() >= prefix_len + 6
        && let Some(count) = parse_named_number(raw_words[prefix_len])
        && raw_words[prefix_len + 1] == "or"
        && raw_words[prefix_len + 2] == "more"
        && let Some(counter_type) = parse_counter_type_word(raw_words[prefix_len + 3])
        && matches!(raw_words[prefix_len + 4], "counter" | "counters")
        && raw_words[prefix_len + 5] == "on"
        && matches!(
            raw_words.get(prefix_len + 6).copied(),
            Some("it" | "him" | "her" | "them" | "this" | "that")
        )
    {
        return Ok(PredicateAst::SourceHasCounterAtLeast {
            counter_type,
            count,
        });
    }

    if filtered.len() == 7
        && matches!(
            &filtered[..4],
            ["this", "creature", "power", "is"]
                | ["this", "creatures", "power", "is"]
                | ["this", "permanent", "power", "is"]
                | ["this", "permanents", "power", "is"]
        )
        && filtered[5] == "or"
        && filtered[6] == "more"
        && let Some(count_word) = filtered.get(4).copied()
        && let Some(count) = parse_named_number(count_word)
    {
        return Ok(PredicateAst::SourcePowerAtLeast(count));
    }

    if filtered.len() >= 10 && filtered[0] == "there" && filtered[1] == "are" {
        let mut idx = 2usize;
        if let Some(count) = parse_named_number(filtered[idx]) {
            idx += 1;
            if filtered.get(idx).copied() == Some("or")
                && filtered.get(idx + 1).copied() == Some("more")
            {
                idx += 2;
            }
            let looks_like_basic_land_type_clause = filtered.get(idx).copied() == Some("basic")
                && filtered.get(idx + 1).copied() == Some("land")
                && matches!(filtered.get(idx + 2).copied(), Some("type" | "types"))
                && filtered.get(idx + 3).copied() == Some("among")
                && matches!(filtered.get(idx + 4).copied(), Some("land" | "lands"));
            if looks_like_basic_land_type_clause {
                let tail = &filtered[idx + 5..];
                let player = if tail == ["that", "player", "controls"]
                    || tail == ["that", "player", "control"]
                    || tail == ["that", "players", "controls"]
                {
                    PlayerAst::That
                } else if tail == ["you", "control"] || tail == ["you", "controls"] {
                    PlayerAst::You
                } else {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported basic-land-types predicate tail (predicate: '{}')",
                        filtered.join(" ")
                    )));
                };

                return Ok(PredicateAst::PlayerControlsBasicLandTypesAmongLandsOrMore {
                    player,
                    count,
                });
            }
        }
    }

    if filtered.len() >= 7
        && filtered[0] == "there"
        && filtered[1] == "are"
        && let Some(count) = parse_named_number(filtered[2])
    {
        let mut idx = 3usize;
        if filtered.get(idx).copied() == Some("or")
            && filtered.get(idx + 1).copied() == Some("more")
        {
            idx += 2;
        }

        let battlefield_suffix_len =
            if slice_ends_with(&filtered[idx..], &["on", "the", "battlefield"]) {
                Some(3usize)
            } else if slice_ends_with(&filtered[idx..], &["on", "battlefield"]) {
                Some(2usize)
            } else {
                None
            };
        if let Some(battlefield_suffix_len) = battlefield_suffix_len {
            let raw_filter_words = &filtered[idx..filtered.len() - battlefield_suffix_len];
            let other = raw_filter_words
                .first()
                .is_some_and(|word| matches!(*word, "other" | "another"));
            let filter_words = if other {
                &raw_filter_words[1..]
            } else {
                raw_filter_words
            };
            if !filter_words.is_empty() {
                let filter_tokens = filter_words
                    .iter()
                    .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
                    .collect::<Vec<_>>();
                if let Ok(mut filter) = parse_object_filter(&filter_tokens, other) {
                    filter.zone = Some(Zone::Battlefield);

                    return Ok(PredicateAst::ValueComparison {
                        left: Value::Count(filter),
                        operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                        right: Value::Fixed(count as i32),
                    });
                }
            }
        }
    }

    let parse_graveyard_card_types_subject = |words: &[&str]| -> Option<PlayerAst> {
        match words {
            [first, second] if *first == "your" && *second == "graveyard" => Some(PlayerAst::You),
            [first, second, third]
                if *first == "that"
                    && (*second == "player" || *second == "players")
                    && *third == "graveyard" =>
            {
                Some(PlayerAst::That)
            }
            [first, second, third]
                if *first == "target"
                    && (*second == "player" || *second == "players")
                    && *third == "graveyard" =>
            {
                Some(PlayerAst::Target)
            }
            [first, second, third]
                if *first == "target"
                    && (*second == "opponent" || *second == "opponents")
                    && *third == "graveyard" =>
            {
                Some(PlayerAst::TargetOpponent)
            }
            [first, second]
                if (*first == "opponent" || *first == "opponents") && *second == "graveyard" =>
            {
                Some(PlayerAst::Opponent)
            }
            _ => None,
        }
    };
    if filtered.len() >= 11 {
        let (count_idx, subject_start, constrained_player) =
            if filtered[0] == "there" && filtered[1] == "are" {
                (2usize, 10usize, None)
            } else if filtered[0] == "you" && filtered[1] == "have" {
                (2usize, 10usize, Some(PlayerAst::You))
            } else {
                (usize::MAX, usize::MAX, None)
            };
        if count_idx != usize::MAX
            && filtered.get(count_idx + 1).copied() == Some("or")
            && filtered.get(count_idx + 2).copied() == Some("more")
            && filtered.get(count_idx + 3).copied() == Some("card")
            && matches!(filtered.get(count_idx + 4).copied(), Some("type" | "types"))
            && filtered.get(count_idx + 5).copied() == Some("among")
            && matches!(filtered.get(count_idx + 6).copied(), Some("card" | "cards"))
            && filtered.get(count_idx + 7).copied() == Some("in")
            && subject_start <= filtered.len()
            && let Some(count) = parse_named_number(filtered[count_idx])
            && let Some(player) = parse_graveyard_card_types_subject(&filtered[subject_start..])
            && constrained_player.map_or(true, |expected| expected == player)
        {
            return Ok(PredicateAst::PlayerHasCardTypesInGraveyardOrMore { player, count });
        }
    }

    let parse_comparison_player_subject = |words: &[&str]| -> Option<(PlayerAst, usize)> {
        match words {
            [first, second, ..] if *first == "that" && *second == "player" => {
                Some((PlayerAst::That, 2))
            }
            [first, second, ..] if *first == "target" && *second == "player" => {
                Some((PlayerAst::Target, 2))
            }
            [first, second, ..] if *first == "target" && *second == "opponent" => {
                Some((PlayerAst::TargetOpponent, 2))
            }
            [first, second, ..] if *first == "each" && *second == "opponent" => {
                Some((PlayerAst::Opponent, 2))
            }
            [first, second, ..] if (*first == "a" || *first == "any") && *second == "player" => {
                Some((PlayerAst::Any, 2))
            }
            [first, second, ..] if *first == "defending" && *second == "player" => {
                Some((PlayerAst::Defending, 2))
            }
            [first, second, ..] if *first == "attacking" && *second == "player" => {
                Some((PlayerAst::Attacking, 2))
            }
            [first, ..] if *first == "you" => Some((PlayerAst::You, 1)),
            [first, ..] if *first == "opponent" || *first == "opponents" => {
                Some((PlayerAst::Opponent, 1))
            }
            [first, second, ..] if *first == "player" && *second == "who" => {
                Some((PlayerAst::That, 1))
            }
            [first, ..] if *first == "player" => Some((PlayerAst::Any, 1)),
            _ => None,
        }
    };
    let parse_life_total_subject = |words: &[&str]| -> Option<(PlayerAst, usize)> {
        match words {
            ["your", "life", "total", ..] => Some((PlayerAst::You, 3)),
            ["their", "life", "total", ..] => Some((PlayerAst::That, 3)),
            ["that", "players", "life", "total", ..] => Some((PlayerAst::That, 4)),
            ["target", "players", "life", "total", ..] => Some((PlayerAst::Target, 4)),
            ["target", "opponents", "life", "total", ..] => Some((PlayerAst::TargetOpponent, 4)),
            ["opponents", "life", "total", ..] | ["opponent", "life", "total", ..] => {
                Some((PlayerAst::Opponent, 3))
            }
            ["defending", "players", "life", "total", ..] => Some((PlayerAst::Defending, 4)),
            ["attacking", "players", "life", "total", ..] => Some((PlayerAst::Attacking, 4)),
            _ => None,
        }
    };
    let half_starting_tail_matches = |tail: &[&str]| {
        matches!(
            tail,
            ["half", "your", "starting", "life", "total"]
                | ["half", "their", "starting", "life", "total"]
                | ["half", "that", "players", "starting", "life", "total"]
                | ["half", "target", "players", "starting", "life", "total"]
                | ["half", "target", "opponents", "starting", "life", "total"]
                | ["half", "opponents", "starting", "life", "total"]
                | ["half", "defending", "players", "starting", "life", "total"]
                | ["half", "attacking", "players", "starting", "life", "total"]
        )
    };
    if let Some((player, subject_len)) = parse_life_total_subject(&filtered)
        && filtered.get(subject_len).copied() == Some("is")
    {
        let tail = &filtered[subject_len + 1..];
        if let Some(rest) = slice_strip_prefix(tail, &["less", "than", "or", "equal", "to"])
            && half_starting_tail_matches(rest)
        {
            return Ok(PredicateAst::PlayerLifeAtMostHalfStartingLifeTotal { player });
        }
        if let Some(rest) = slice_strip_prefix(tail, &["less", "than"])
            && half_starting_tail_matches(rest)
        {
            return Ok(PredicateAst::PlayerLifeLessThanHalfStartingLifeTotal { player });
        }
    }
    if let Some((player, subject_len)) = parse_comparison_player_subject(&filtered)
        && matches!(
            filtered.get(subject_len).copied(),
            Some("control" | "controls")
        )
        && filtered.get(subject_len + 1).copied() == Some("more")
        && let Some(than_offset) = find_index(&filtered[subject_len + 2..], |word| *word == "than")
    {
        let than_idx = subject_len + 2 + than_offset;
        let tail = &filtered[than_idx..];
        if matches!(tail, ["than", "you"] | ["than", "you", "do"]) {
            let filter_tokens = filtered[subject_len + 2..than_idx]
                .iter()
                .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
                .collect::<Vec<_>>();
            if !filter_tokens.is_empty() {
                let other = filter_tokens
                    .first()
                    .is_some_and(|token| token.is_word("another") || token.is_word("other"));
                if let Ok(filter) = parse_object_filter(&filter_tokens, other)
                    && filter != ObjectFilter::default()
                {
                    return Ok(PredicateAst::PlayerControlsMoreThanYou { player, filter });
                }
            }
        }
    }

    if let Some((player, subject_len)) = parse_comparison_player_subject(&filtered)
        && filtered.get(subject_len).copied() == Some("has")
        && matches!(
            &filtered[subject_len + 1..],
            ["more", "life", "than", "you"] | ["more", "life", "than", "you", "do"]
        )
    {
        return Ok(PredicateAst::PlayerHasMoreLifeThanYou { player });
    }

    if filtered.len() >= 8
        && filtered[0] == "no"
        && matches!(filtered[1], "opponent" | "opponents")
        && filtered[2] == "has"
        && filtered[3] == "more"
        && filtered[4] == "life"
        && filtered[5] == "than"
        && let Some((player, subject_len)) = parse_comparison_player_subject(&filtered[6..])
        && subject_len + 6 == filtered.len()
    {
        return Ok(PredicateAst::PlayerHasNoOpponentWithMoreLifeThan { player });
    }

    if let Some((player, subject_len)) = parse_comparison_player_subject(&filtered)
        && filtered.get(subject_len).copied() == Some("has")
        && matches!(
            &filtered[subject_len + 1..],
            ["more", "life", "than", "each", "other", "player"]
                | ["more", "life", "than", "each", "other", "players"]
        )
    {
        return Ok(PredicateAst::PlayerHasMoreLifeThanEachOtherPlayer { player });
    }

    if let Some((player, subject_len)) = parse_comparison_player_subject(&filtered)
        && filtered.get(subject_len).copied() == Some("has")
        && matches!(
            &filtered[subject_len + 1..],
            ["more", "card", "in", "hand", "than", "you"]
                | ["more", "cards", "in", "hand", "than", "you"]
                | ["more", "card", "in", "their", "hand", "than", "you"]
                | ["more", "cards", "in", "their", "hand", "than", "you"]
                | ["more", "card", "in", "hand", "than", "you", "do"]
                | ["more", "cards", "in", "hand", "than", "you", "do"]
                | ["more", "card", "in", "their", "hand", "than", "you", "do"]
                | ["more", "cards", "in", "their", "hand", "than", "you", "do"]
        )
    {
        return Ok(PredicateAst::PlayerHasMoreCardsInHandThanYou { player });
    }

    if let Some((player, subject_len)) = parse_comparison_player_subject(&filtered)
        && filtered.get(subject_len).copied() == Some("has")
        && matches!(
            &filtered[subject_len + 1..],
            [
                "more", "card", "in", "hand", "than", "each", "other", "player"
            ] | [
                "more", "cards", "in", "hand", "than", "each", "other", "player"
            ] | [
                "more", "card", "in", "their", "hand", "than", "each", "other", "player",
            ] | [
                "more", "cards", "in", "their", "hand", "than", "each", "other", "player",
            ]
        )
    {
        return Ok(PredicateAst::PlayerHasMoreCardsInHandThanEachOtherPlayer { player });
    }

    if let Some((player, subject_len)) = parse_comparison_player_subject(&filtered)
        && filtered.get(subject_len).copied() == Some("has")
        && let Some(count_word) = filtered.get(subject_len + 1).copied()
        && let Some(count) = parse_named_number(count_word)
        && filtered.get(subject_len + 2).copied() == Some("or")
        && let Some(comp_word) = filtered.get(subject_len + 3).copied()
        && matches!(comp_word, "more" | "fewer" | "less")
        && matches!(
            filtered.get(subject_len + 4).copied(),
            Some("card" | "cards")
        )
        && filtered.get(subject_len + 5).copied() == Some("in")
        && filtered.get(subject_len + 6).copied() == Some("hand")
        && filtered.len() == subject_len + 7
    {
        return Ok(if comp_word == "more" {
            PredicateAst::PlayerCardsInHandOrMore { player, count }
        } else {
            PredicateAst::PlayerCardsInHandOrFewer { player, count }
        });
    }

    if filtered.as_slice() == ["you", "have", "no", "cards", "in", "hand"] {
        return Ok(PredicateAst::YouHaveNoCardsInHand);
    }

    if matches!(
        filtered.as_slice(),
        ["it", "your", "turn"] | ["its", "your", "turn"] | ["your", "turn"]
    ) {
        return Ok(PredicateAst::YourTurn);
    }

    if matches!(
        filtered.as_slice(),
        ["creature", "died", "this", "turn"] | ["creatures", "died", "this", "turn"]
    ) {
        return Ok(PredicateAst::CreatureDiedThisTurn);
    }

    if filtered.len() == 7
        && let Some(count) = parse_named_number(filtered[0])
        && filtered[1..] == ["or", "more", "creatures", "died", "this", "turn"]
    {
        return Ok(PredicateAst::CreatureDiedThisTurnOrMore(count));
    }

    if matches!(
        filtered.as_slice(),
        [
            "permanent",
            "left",
            "battlefield",
            "under",
            "your",
            "control",
            "this",
            "turn"
        ] | [
            "permanents",
            "left",
            "battlefield",
            "under",
            "your",
            "control",
            "this",
            "turn"
        ] | [
            "permanent",
            "you",
            "controlled",
            "left",
            "battlefield",
            "this",
            "turn"
        ] | [
            "permanents",
            "you",
            "controlled",
            "left",
            "battlefield",
            "this",
            "turn"
        ]
    ) {
        return Ok(PredicateAst::PermanentLeftBattlefieldUnderYourControlThisTurn);
    }

    if matches!(
        filtered.as_slice(),
        [
            "you",
            "had",
            "land",
            "enter",
            "battlefield",
            "under",
            "your",
            "control",
            "this",
            "turn"
        ] | [
            "you",
            "had",
            "land",
            "entered",
            "battlefield",
            "under",
            "your",
            "control",
            "this",
            "turn"
        ] | [
            "you",
            "had",
            "lands",
            "enter",
            "battlefield",
            "under",
            "your",
            "control",
            "this",
            "turn"
        ] | [
            "you",
            "had",
            "lands",
            "entered",
            "battlefield",
            "under",
            "your",
            "control",
            "this",
            "turn"
        ]
    ) {
        return Ok(PredicateAst::PlayerHadLandEnterBattlefieldThisTurn {
            player: PlayerAst::You,
        });
    }

    if filtered.len() >= 7
        && filtered[0] == "you"
        && filtered[1] == "gained"
        && let Some((count, used)) = parse_number(&tokens[2..])
        && filtered[2 + used..] == ["or", "more", "life", "this", "turn"]
    {
        return Ok(PredicateAst::PlayerGainedLifeThisTurnOrMore {
            player: PlayerAst::You,
            count: count as u32,
        });
    }

    if filtered.as_slice() == ["you", "gained", "life", "this", "turn"] {
        return Ok(PredicateAst::PlayerGainedLifeThisTurnOrMore {
            player: PlayerAst::You,
            count: 1,
        });
    }

    if filtered.as_slice() == ["you", "attacked", "this", "turn"] {
        return Ok(PredicateAst::YouAttackedThisTurn);
    }

    if filtered.len() == 9
        && filtered[0] == "you"
        && filtered[1] == "attacked"
        && filtered[2] == "with"
        && filtered[3] == "exactly"
        && matches!(filtered[5], "other" | "others")
        && matches!(filtered[6], "creature" | "creatures")
        && filtered[7] == "this"
        && filtered[8] == "combat"
        && let Some(count) = parse_named_number(filtered[4])
    {
        return Ok(PredicateAst::YouAttackedWithExactlyNOtherCreaturesThisCombat(count));
    }

    if matches!(
        filtered.as_slice(),
        [
            "this", "creature", "attacked", "or", "blocked", "this", "turn"
        ] | [
            "this",
            "permanent",
            "attacked",
            "or",
            "blocked",
            "this",
            "turn"
        ] | ["this", "attacked", "or", "blocked", "this", "turn"]
            | ["it", "attacked", "or", "blocked", "this", "turn"]
    ) {
        return Ok(PredicateAst::SourceAttackedOrBlockedThisTurn);
    }

    if filtered.as_slice() == ["you", "cast", "it"]
        || filtered.as_slice() == ["you", "cast", "this", "spell"]
    {
        return Ok(PredicateAst::SourceWasCast);
    }

    if filtered.len() >= 6
        && filtered[0] == "this"
        && filtered[1] == "spell"
        && filtered[2] == "was"
        && filtered[3] == "cast"
        && filtered[4] == "from"
    {
        let zone_words = &filtered[5..];
        let zone = if zone_words.len() == 1 {
            parse_zone_word(zone_words[0])
        } else if zone_words.len() == 2 && is_article(zone_words[0]) {
            parse_zone_word(zone_words[1])
        } else if zone_words.len() == 2 && zone_words[0] == "the" {
            parse_zone_word(zone_words[1])
        } else {
            None
        };

        if let Some(zone) = zone {
            return Ok(PredicateAst::ThisSpellWasCastFromZone(zone));
        }
    }

    if filtered.as_slice() == ["no", "spells", "were", "cast", "last", "turn"]
        || filtered.as_slice() == ["no", "spell", "was", "cast", "last", "turn"]
    {
        return Ok(PredicateAst::NoSpellsWereCastLastTurn);
    }
    if filtered.as_slice() == ["this", "spell", "was", "kicked"] {
        return Ok(PredicateAst::ThisSpellWasKicked);
    }
    if filtered.as_slice() == ["this", "spell", "was", "bargained"]
        || filtered.as_slice() == ["it", "was", "bargained"]
    {
        return Ok(PredicateAst::ThisSpellPaidLabel("Bargain".to_string()));
    }
    if filtered.len() == 4
        && matches!(filtered[0], "a" | "an")
        && parse_subtype_word(filtered[1]).is_some()
        && matches!(filtered[2], "was" | "were")
        && filtered[3] == "beheld"
    {
        return Ok(PredicateAst::ThisSpellPaidLabel("Behold".to_string()));
    }
    if filtered.len() == 3
        && parse_subtype_word(filtered[0]).is_some()
        && matches!(filtered[1], "was" | "were")
        && filtered[2] == "beheld"
    {
        return Ok(PredicateAst::ThisSpellPaidLabel("Behold".to_string()));
    }
    if filtered.as_slice() == ["gift", "was", "promised"] {
        return Ok(PredicateAst::ThisSpellPaidLabel("Gift".to_string()));
    }
    if filtered.len() == 6
        && filtered[0] == "this"
        && matches!(
            filtered[1],
            "spell's" | "card's" | "creature's" | "permanent's"
        )
        && filtered[3] == "cost"
        && filtered[4] == "was"
        && filtered[5] == "paid"
    {
        let mut chars = filtered[2].chars();
        let Some(first) = chars.next() else {
            return Err(CardTextError::ParseError(
                "missing paid-cost label in predicate".to_string(),
            ));
        };
        let label = format!(
            "{}{}",
            first.to_ascii_uppercase(),
            chars.as_str().to_ascii_lowercase()
        );
        return Ok(PredicateAst::ThisSpellPaidLabel(label));
    }
    if filtered.as_slice() == ["it", "was", "kicked"] {
        return Ok(PredicateAst::ThisSpellWasKicked);
    }
    if filtered.as_slice() == ["that", "was", "kicked"] {
        return Ok(PredicateAst::TargetWasKicked);
    }

    if filtered.as_slice() == ["you", "have", "full", "party"] {
        return Ok(PredicateAst::YouHaveFullParty);
    }
    if filtered.as_slice() == ["its", "controller", "poisoned"]
        || filtered.as_slice() == ["that", "spells", "controller", "poisoned"]
    {
        return Ok(PredicateAst::TargetSpellControllerIsPoisoned);
    }
    if filtered.as_slice() == ["no", "mana", "was", "spent", "to", "cast", "it"]
        || filtered.as_slice() == ["no", "mana", "were", "spent", "to", "cast", "it"]
        || filtered.as_slice() == ["no", "mana", "was", "spent", "to", "cast", "that", "spell"]
        || filtered.as_slice() == ["no", "mana", "were", "spent", "to", "cast", "that", "spell"]
    {
        return Ok(PredicateAst::TargetSpellNoManaSpentToCast);
    }
    if filtered.as_slice()
        == [
            "you",
            "control",
            "more",
            "creatures",
            "than",
            "that",
            "spells",
            "controller",
        ]
        || filtered.as_slice()
            == [
                "you",
                "control",
                "more",
                "creatures",
                "than",
                "its",
                "controller",
            ]
    {
        return Ok(PredicateAst::YouControlMoreCreaturesThanTargetSpellController);
    }
    if filtered.len() == 7
        && matches!(filtered[0], "w" | "u" | "b" | "r" | "g" | "c" | "s")
        && filtered[1] == "was"
        && filtered[2] == "spent"
        && filtered[3] == "to"
        && filtered[4] == "cast"
        && filtered[5] == "this"
        && filtered[6] == "spell"
        && let Ok(symbol) = parse_mana_symbol(filtered[0])
    {
        return Ok(PredicateAst::ManaSpentToCastThisSpellAtLeast {
            amount: 1,
            symbol: Some(symbol),
        });
    }

    if let Some((amount, symbol)) = parse_mana_spent_to_cast_predicate(&filtered) {
        return Ok(PredicateAst::ManaSpentToCastThisSpellAtLeast { amount, symbol });
    }

    if filtered.len() >= 5
        && matches!(
            filtered.as_slice(),
            ["this", "permanent", "attached", "to", ..]
                | ["that", "permanent", "attached", "to", ..]
                | ["this", "permanent", "is", "attached", "to", ..]
                | ["that", "permanent", "is", "attached", "to", ..]
        )
    {
        let attached_start = if filtered.get(2).copied() == Some("is") {
            5
        } else {
            4
        };
        let attached_tokens = filtered[attached_start..]
            .iter()
            .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
            .collect::<Vec<_>>();
        let mut filter = parse_object_filter(&attached_tokens, false)?;
        if filter.card_types.is_empty() {
            filter.card_types.push(CardType::Creature);
        }
        return Ok(PredicateAst::TaggedMatches(
            TagKey::from("enchanted"),
            filter,
        ));
    }

    if filtered.len() >= 4 && filtered[0] == "sacrificed" && filtered[2] == "was" {
        let sacrificed_head = filtered[1];
        let subject_card_type =
            parse_card_type(sacrificed_head).filter(|card_type| is_permanent_type(*card_type));
        let subject_is_permanent = sacrificed_head == "permanent" || subject_card_type.is_some();

        if subject_is_permanent {
            let descriptor_tokens = filtered[3..]
                .iter()
                .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
                .collect::<Vec<_>>();
            let mut filter = parse_object_filter(&descriptor_tokens, false)?;
            if filter.card_types.is_empty() {
                if let Some(card_type) = subject_card_type {
                    filter.card_types.push(card_type);
                }
            }
            if filter.zone.is_none() && sacrificed_head == "permanent" {
                filter.zone = Some(Zone::Battlefield);
            }
            return Ok(PredicateAst::ItMatches(filter));
        }
    }

    if filtered.as_slice()
        == [
            "this", "is", "fourth", "time", "this", "ability", "has", "resolved", "this", "turn",
        ]
    {
        return Ok(PredicateAst::Unmodeled(filtered.join(" ")));
    }

    if matches!(
        filtered.as_slice(),
        ["any", "of", "those", "cards", "remain", "exiled"]
            | ["those", "cards", "remain", "exiled"]
            | ["that", "card", "remains", "exiled"]
            | ["it", "remains", "exiled"]
    ) {
        return Ok(PredicateAst::TaggedMatches(
            TagKey::from(IT_TAG),
            ObjectFilter::default().in_zone(Zone::Exile),
        ));
    }

    if filtered[0] == "its" {
        filtered[0] = "it";
    }
    if filtered.len() >= 2 && filtered[0] == "it" && filtered[1] == "s" {
        filtered.remove(1);
    }

    let demonstrative_reference_len = if filtered.first().copied() == Some("it") {
        Some(1usize)
    } else if filtered.len() >= 2
        && filtered[0] == "that"
        && matches!(
            filtered[1],
            "artifact"
                | "card"
                | "creature"
                | "land"
                | "object"
                | "permanent"
                | "source"
                | "spell"
                | "token"
        )
    {
        Some(2usize)
    } else {
        None
    };

    let is_it_soulbond_paired = matches!(
        filtered.as_slice(),
        ["it", "paired", "with", "creature"]
            | ["it", "paired", "with", "another", "creature"]
            | ["it", "s", "paired", "with", "creature"]
            | ["it", "s", "paired", "with", "another", "creature"]
    );
    if is_it_soulbond_paired {
        return Ok(PredicateAst::ItIsSoulbondPaired);
    }

    if filtered.len() >= 2 {
        let tag = if slice_starts_with(&filtered, &["equipped", "creature"]) {
            Some("equipped")
        } else if slice_starts_with(&filtered, &["enchanted", "creature"]) {
            Some("enchanted")
        } else {
            None
        };
        if let Some(tag) = tag {
            let remainder = filtered[2..].to_vec();
            let tokens = remainder
                .iter()
                .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
                .collect::<Vec<_>>();
            let mut filter = parse_object_filter(&tokens, false)?;
            if filter.card_types.is_empty() {
                filter.card_types.push(CardType::Creature);
            }
            return Ok(PredicateAst::TaggedMatches(TagKey::from(tag), filter));
        }
    }

    let onto_battlefield_idx = find_word_slice_phrase_start(&filtered, &["onto", "battlefield"])
        .or_else(|| find_word_slice_phrase_start(&filtered, &["onto", "the", "battlefield"]));
    if filtered.len() >= 7
        && filtered[0] == "you"
        && filtered[1] == "put"
        && slice_ends_with(&filtered, &["this", "way"])
        && let Some(onto_idx) = onto_battlefield_idx
    {
        let filter_words = &filtered[2..onto_idx];
        let filter_tokens = filter_words
            .iter()
            .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
            .collect::<Vec<_>>();
        let mut filter = parse_object_filter(&filter_tokens, false)?;
        if filter.zone.is_none() {
            filter.zone = Some(Zone::Battlefield);
        }
        return Ok(PredicateAst::PlayerTaggedObjectMatches {
            player: PlayerAst::You,
            tag: TagKey::from(IT_TAG),
            filter,
        });
    }

    let is_it = demonstrative_reference_len == Some(1);
    let has_card = demonstrative_reference_len
        .map(|reference_len| slice_contains(&filtered[reference_len..], &"card"))
        .unwrap_or(false);

    if is_it {
        if filtered
            .get(1)
            .is_some_and(|word| *word == "has" || *word == "have")
        {
            filtered.remove(1);
        }
        if filtered.len() >= 3 && filtered[1] == "mana" && filtered[2] == "value" {
            let mana_value_tail = if filtered
                .get(3)
                .is_some_and(|word| matches!(*word, "is" | "are" | "was" | "were"))
            {
                &filtered[4..]
            } else {
                &filtered[3..]
            };
            let compares_to_colors_spent = mana_value_tail
                == [
                    "less", "than", "or", "equal", "to", "number", "of", "colors", "of", "mana",
                    "spent", "to", "cast", "this", "spell",
                ]
                || mana_value_tail
                    == [
                        "less", "than", "or", "equal", "to", "number", "of", "color", "of", "mana",
                        "spent", "to", "cast", "this", "spell",
                    ];
            if compares_to_colors_spent {
                return Ok(PredicateAst::TargetManaValueLteColorsSpentToCastThisSpell);
            }

            if let Some((cmp, _consumed)) =
                parse_filter_comparison_tokens("mana value", mana_value_tail, &filtered)?
            {
                return Ok(PredicateAst::ItMatches(ObjectFilter {
                    mana_value: Some(cmp),
                    ..Default::default()
                }));
            }
        }

        if filtered.len() >= 3 && (filtered[1] == "power" || filtered[1] == "toughness") {
            let axis = filtered[1];
            let value_tail = &filtered[2..];
            if let Some((cmp, _consumed)) =
                parse_filter_comparison_tokens(axis, value_tail, &filtered)?
            {
                let mut filter = ObjectFilter::default();
                if axis == "power" {
                    filter.power = Some(cmp);
                } else {
                    filter.toughness = Some(cmp);
                }
                return Ok(PredicateAst::ItMatches(filter));
            }
        }
    }

    if let Some(reference_len) = demonstrative_reference_len {
        let mut descriptor_words = filtered[reference_len..].to_vec();
        if descriptor_words.as_slice() == ["has", "toxic"]
            || descriptor_words.as_slice() == ["have", "toxic"]
        {
            let mut filter = ObjectFilter::default().with_ability_marker("toxic");
            if filtered.get(1).copied() == Some("creature") {
                filter.card_types.push(CardType::Creature);
            }
            return Ok(PredicateAst::ItMatches(filter));
        }
        if descriptor_words
            .first()
            .is_some_and(|word| matches!(*word, "is" | "are"))
        {
            descriptor_words.remove(0);
        }
        if slice_starts_with(&descriptor_words, &["not", "token"]) {
            descriptor_words.drain(0..2);
            descriptor_words.insert(0, "nontoken");
        }
        if !descriptor_words.is_empty() {
            let descriptor_tokens = descriptor_words
                .iter()
                .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
                .collect::<Vec<_>>();
            if let Ok(filter) = parse_object_filter_lexed(&descriptor_tokens, false)
                && filter != ObjectFilter::default()
            {
                if has_card
                    && filter.card_types.len() == 1
                    && filter.card_types[0] == CardType::Land
                    && filter.subtypes.is_empty()
                    && !filter.nontoken
                    && filter.excluded_card_types.is_empty()
                {
                    return Ok(PredicateAst::ItIsLandCard);
                }
                return Ok(PredicateAst::ItMatches(filter));
            }
        }
    }

    if filtered.len() >= 3
        && filtered[0] == "you"
        && (filtered[1] == "control" || filtered[1] == "controls")
        && (filtered[2] == "no" || filtered[2] == "neither")
    {
        let control_tokens = filtered[3..]
            .iter()
            .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
            .collect::<Vec<_>>();
        if let Ok(mut filter) = parse_object_filter(&control_tokens, false) {
            filter.controller = Some(PlayerFilter::You);
            if filtered[2] == "neither" {
                filter = filter
                    .match_tagged(TagKey::from(IT_TAG), TaggedOpbjectRelation::IsTaggedObject);
            }
            return Ok(PredicateAst::PlayerControlsNo {
                player: PlayerAst::You,
                filter,
            });
        }
    }

    let you_dont_control_filter_start = if filtered.len() >= 4
        && filtered[0] == "you"
        && matches!(filtered[1], "dont" | "don't")
        && (filtered[2] == "control" || filtered[2] == "controls")
    {
        Some(3usize)
    } else if filtered.len() >= 5
        && filtered[0] == "you"
        && filtered[1] == "do"
        && filtered[2] == "not"
        && (filtered[3] == "control" || filtered[3] == "controls")
    {
        Some(4usize)
    } else {
        None
    };
    if let Some(filter_start) = you_dont_control_filter_start {
        let control_tokens = filtered[filter_start..]
            .iter()
            .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
            .collect::<Vec<_>>();
        let other = control_tokens
            .first()
            .is_some_and(|token| token.is_word("another") || token.is_word("other"));
        if let Ok(mut filter) = parse_object_filter(&control_tokens, other) {
            filter.controller = Some(PlayerFilter::You);
            return Ok(PredicateAst::PlayerControlsNo {
                player: PlayerAst::You,
                filter,
            });
        }
    }

    if filtered.len() >= 7
        && filtered[0] == "you"
        && (filtered[1] == "control" || filtered[1] == "controls")
        && let Some(or_idx) = find_index(&filtered, |word| *word == "or")
        && or_idx > 2
    {
        let left_tokens = filtered[2..or_idx]
            .iter()
            .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
            .collect::<Vec<_>>();
        let mut right_words = filtered[or_idx + 1..].to_vec();
        if right_words.first().copied() == Some("there") {
            right_words = right_words[1..].to_vec();
        }
        if slice_contains(&right_words, &"graveyard") && slice_contains(&right_words, &"your") {
            let right_tokens = right_words
                .iter()
                .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
                .collect::<Vec<_>>();
            if let (Ok(mut control_filter), Ok(mut graveyard_filter)) = (
                parse_object_filter(&left_tokens, false),
                parse_object_filter(&right_tokens, false),
            ) {
                control_filter.controller = Some(PlayerFilter::You);
                if graveyard_filter.zone.is_none() {
                    graveyard_filter.zone = Some(Zone::Graveyard);
                }
                if graveyard_filter.owner.is_none() {
                    graveyard_filter.owner = Some(PlayerFilter::You);
                }
                return Ok(PredicateAst::PlayerControlsOrHasCardInGraveyard {
                    player: PlayerAst::You,
                    control_filter,
                    graveyard_filter,
                });
            }
        }
    }

    if filtered.len() >= 3
        && filtered[0] == "you"
        && (filtered[1] == "control" || filtered[1] == "controls")
    {
        let mut filter_start = 2usize;
        let mut min_count: Option<u32> = None;
        let mut exact_count: Option<u32> = None;
        if let Some(raw_count) = filtered.get(2)
            && let Some(parsed_count) = parse_named_number(raw_count)
            && filtered.get(3).copied() == Some("or")
            && filtered.get(4).copied() == Some("more")
        {
            min_count = Some(parsed_count);
            filter_start = 5;
        } else if filtered.get(2).copied() == Some("exactly")
            && let Some(raw_count) = filtered.get(3)
            && let Some(parsed_count) = parse_named_number(raw_count)
        {
            exact_count = Some(parsed_count);
            filter_start = 4;
        } else if filtered.get(2).copied() == Some("at")
            && filtered.get(3).copied() == Some("least")
            && let Some(raw_count) = filtered.get(4)
            && let Some(parsed_count) = parse_named_number(raw_count)
        {
            min_count = Some(parsed_count);
            filter_start = 5;
        }

        let mut control_words = filtered[filter_start..].to_vec();
        let mut requires_different_powers = false;
        if slice_ends_with(&control_words, &["with", "different", "powers"])
            || slice_ends_with(&control_words, &["with", "different", "power"])
        {
            requires_different_powers = true;
            control_words.truncate(control_words.len().saturating_sub(3));
        }
        let control_tokens = control_words
            .iter()
            .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
            .collect::<Vec<_>>();
        let other = control_tokens
            .first()
            .is_some_and(|token| token.is_word("another") || token.is_word("other"));
        if let Ok(mut filter) = parse_object_filter(&control_tokens, other) {
            filter.controller = Some(PlayerFilter::You);
            if let Some(count) = exact_count {
                return Ok(PredicateAst::PlayerControlsExactly {
                    player: PlayerAst::You,
                    filter,
                    count,
                });
            }
            if let Some(count) = min_count
                && count > 1
            {
                if requires_different_powers {
                    return Ok(PredicateAst::PlayerControlsAtLeastWithDifferentPowers {
                        player: PlayerAst::You,
                        filter,
                        count,
                    });
                }
                return Ok(PredicateAst::PlayerControlsAtLeast {
                    player: PlayerAst::You,
                    filter,
                    count,
                });
            }
            return Ok(PredicateAst::PlayerControls {
                player: PlayerAst::You,
                filter,
            });
        }
    }

    if filtered.len() >= 4
        && filtered[0] == "that"
        && (filtered[1] == "player" || filtered[1] == "players")
        && (filtered[2] == "control" || filtered[2] == "controls")
    {
        let mut filter_start = 3usize;
        let mut min_count: Option<u32> = None;
        let mut exact_count: Option<u32> = None;
        if let Some(raw_count) = filtered.get(3)
            && let Some(parsed_count) = parse_named_number(raw_count)
            && filtered.get(4).copied() == Some("or")
            && filtered.get(5).copied() == Some("more")
        {
            min_count = Some(parsed_count);
            filter_start = 6;
        } else if filtered.get(3).copied() == Some("exactly")
            && let Some(raw_count) = filtered.get(4)
            && let Some(parsed_count) = parse_named_number(raw_count)
        {
            exact_count = Some(parsed_count);
            filter_start = 5;
        } else if filtered.get(3).copied() == Some("at")
            && filtered.get(4).copied() == Some("least")
            && let Some(raw_count) = filtered.get(5)
            && let Some(parsed_count) = parse_named_number(raw_count)
        {
            min_count = Some(parsed_count);
            filter_start = 6;
        }

        let control_tokens = filtered[filter_start..]
            .iter()
            .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
            .collect::<Vec<_>>();
        let other = control_tokens
            .first()
            .is_some_and(|token| token.is_word("another") || token.is_word("other"));
        if let Ok(filter) = parse_object_filter(&control_tokens, other) {
            if let Some(count) = exact_count {
                return Ok(PredicateAst::PlayerControlsExactly {
                    player: PlayerAst::That,
                    filter,
                    count,
                });
            }
            if let Some(count) = min_count
                && count > 1
            {
                return Ok(PredicateAst::PlayerControlsAtLeast {
                    player: PlayerAst::That,
                    filter,
                    count,
                });
            }
            return Ok(PredicateAst::PlayerControls {
                player: PlayerAst::That,
                filter,
            });
        }
    }

    if filtered.as_slice() == ["you", "controlled", "that", "permanent"]
        || filtered.as_slice() == ["you", "control", "that", "permanent"]
    {
        return Ok(PredicateAst::PlayerTaggedObjectMatches {
            player: PlayerAst::You,
            tag: TagKey::from(IT_TAG),
            filter: ObjectFilter::default(),
        });
    }

    if filtered.as_slice() == ["it", "entered", "under", "your", "control"]
        || filtered.as_slice() == ["that", "card", "entered", "under", "your", "control"]
        || filtered.as_slice() == ["that", "permanent", "entered", "under", "your", "control"]
    {
        return Ok(PredicateAst::PlayerTaggedObjectEnteredBattlefieldThisTurn {
            player: PlayerAst::You,
            tag: TagKey::from(IT_TAG),
        });
    }

    if filtered.len() >= 8
        && filtered[0] == "you"
        && filtered[1] == "put"
        && slice_ends_with(&filtered, &["onto", "the", "battlefield", "this", "way"])
    {
        let filter_words = &filtered[2..filtered.len() - 5];
        let filter_tokens = filter_words
            .iter()
            .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
            .collect::<Vec<_>>();
        let filter = parse_object_filter(&filter_tokens, false)?;
        return Ok(PredicateAst::PlayerTaggedObjectMatches {
            player: PlayerAst::You,
            tag: TagKey::from(IT_TAG),
            filter,
        });
    }

    if filtered.as_slice() == ["it", "wasnt", "blocking"]
        || filtered.as_slice() == ["it", "was", "not", "blocking"]
        || filtered.as_slice() == ["that", "creature", "wasnt", "blocking"]
    {
        return Ok(PredicateAst::TaggedMatches(
            TagKey::from(IT_TAG),
            ObjectFilter {
                nonblocking: true,
                ..Default::default()
            },
        ));
    }

    if filtered.as_slice() == ["no", "creatures", "are", "on", "battlefield"] {
        return Ok(PredicateAst::PlayerControlsNo {
            player: PlayerAst::Any,
            filter: ObjectFilter::creature(),
        });
    }

    if filtered.as_slice() == ["you", "have", "citys", "blessing"]
        || filtered.as_slice() == ["you", "have", "city", "blessing"]
        || slice_starts_with(
            &filtered,
            &["you", "have", "citys", "blessing", "for", "each"],
        )
        || slice_starts_with(
            &filtered,
            &["you", "have", "city", "blessing", "for", "each"],
        )
    {
        return Ok(PredicateAst::PlayerHasCitysBlessing {
            player: PlayerAst::You,
        });
    }

    if filtered.as_slice() == ["youre", "the", "monarch"]
        || filtered.as_slice() == ["youre", "monarch"]
        || filtered.as_slice() == ["you", "are", "the", "monarch"]
        || filtered.as_slice() == ["you", "are", "monarch"]
    {
        return Ok(PredicateAst::PlayerIsMonarch {
            player: PlayerAst::You,
        });
    }

    if filtered.as_slice() == ["you", "have", "the", "initiative"]
        || filtered.as_slice() == ["you", "have", "initiative"]
    {
        return Ok(PredicateAst::PlayerHasInitiative {
            player: PlayerAst::You,
        });
    }

    if filtered.as_slice() == ["youve", "completed", "a", "dungeon"]
        || filtered.as_slice() == ["you", "have", "completed", "a", "dungeon"]
    {
        return Ok(PredicateAst::PlayerCompletedDungeon {
            player: PlayerAst::You,
            dungeon_name: None,
        });
    }

    if (slice_starts_with(&filtered, &["youve", "completed"]) && filtered.len() > 2)
        || (slice_starts_with(&filtered, &["you", "have", "completed"]) && filtered.len() > 3)
    {
        let name_start = if filtered[1] == "have" { 3 } else { 2 };
        let dungeon_name = filtered[name_start..]
            .iter()
            .map(|word| (*word).to_string())
            .collect::<Vec<_>>()
            .join(" ");
        return Ok(PredicateAst::PlayerCompletedDungeon {
            player: PlayerAst::You,
            dungeon_name: Some(dungeon_name),
        });
    }

    if (slice_starts_with(&filtered, &["you", "havent", "completed"]) && filtered.len() > 3)
        || (slice_starts_with(&filtered, &["you", "have", "not", "completed"])
            && filtered.len() > 4)
    {
        let name_start = if filtered[1] == "have" { 4 } else { 3 };
        let dungeon_name = filtered[name_start..]
            .iter()
            .map(|word| (*word).to_string())
            .collect::<Vec<_>>()
            .join(" ");
        return Ok(PredicateAst::Not(Box::new(
            PredicateAst::PlayerCompletedDungeon {
                player: PlayerAst::You,
                dungeon_name: Some(dungeon_name),
            },
        )));
    }

    if filtered.as_slice() == ["youve", "cast", "another", "spell", "this", "turn"]
        || filtered.as_slice() == ["you", "have", "cast", "another", "spell", "this", "turn"]
        || filtered.as_slice() == ["you", "cast", "another", "spell", "this", "turn"]
    {
        return Ok(PredicateAst::PlayerCastSpellsThisTurnOrMore {
            player: PlayerAst::You,
            count: 2,
        });
    }

    let spell_cast_prefix = if slice_starts_with(&filtered, &["opponent", "has", "cast"]) {
        Some((3usize, PlayerFilter::Opponent))
    } else if slice_starts_with(&filtered, &["opponents", "have", "cast"]) {
        Some((3usize, PlayerFilter::Opponent))
    } else if slice_starts_with(&filtered, &["youve", "cast"]) {
        Some((2usize, PlayerFilter::You))
    } else if slice_starts_with(&filtered, &["you", "have", "cast"]) {
        Some((3usize, PlayerFilter::You))
    } else if slice_starts_with(&filtered, &["you", "cast"]) {
        Some((2usize, PlayerFilter::You))
    } else {
        None
    };
    if let Some((prefix_len, player)) = spell_cast_prefix
        && filtered.len() > prefix_len + 2
        && filtered[filtered.len() - 2..] == ["this", "turn"]
    {
        let filter_words = &filtered[prefix_len..filtered.len() - 2];
        let filter_tokens = filter_words
            .iter()
            .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
            .collect::<Vec<_>>();
        if let Ok(filter) = parse_object_filter_lexed(&filter_tokens, false) {
            return Ok(PredicateAst::ValueComparison {
                left: Value::SpellsCastThisTurnMatching {
                    player,
                    filter,
                    exclude_source: false,
                },
                operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                right: Value::Fixed(1),
            });
        }
    }

    if filtered.len() == 5
        && filtered[0] == "x"
        && filtered[1] == "is"
        && filtered[3] == "or"
        && filtered[4] == "more"
    {
        if let Some(amount) = filtered[2]
            .parse::<i32>()
            .ok()
            .or_else(|| parse_named_number(filtered[2]).map(|n| n as i32))
        {
            return Ok(PredicateAst::ValueComparison {
                left: Value::X,
                operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                right: Value::Fixed(amount),
            });
        }
    }

    let unsupported_unmodeled = filtered.as_slice() == ["you", "gained", "life", "this", "turn"]
        || filtered.as_slice() == ["you", "dont", "cast", "it"]
        || filtered.as_slice() == ["it", "has", "odd", "number", "of", "counters", "on", "it"]
        || filtered.as_slice() == ["it", "has", "even", "number", "of", "counters", "on", "it"]
        || filtered.as_slice() == ["opponent", "lost", "life", "this", "turn"]
        || filtered.as_slice() == ["opponents", "lost", "life", "this", "turn"]
        || filtered.as_slice() == ["an", "opponent", "lost", "life", "this", "turn"]
        || filtered.as_slice() == ["this", "card", "in", "your", "graveyard"]
        || filtered.as_slice() == ["this", "artifact", "untapped"]
        || filtered.as_slice() == ["this", "has", "luck", "counter", "on", "it"]
        || filtered.as_slice() == ["it", "had", "revival", "counter", "on", "it"]
        || filtered.as_slice() == ["that", "creature", "would", "die", "this", "turn"]
        || filtered.as_slice()
            == [
                "this", "second", "time", "this", "ability", "has", "resolved", "this", "turn",
            ]
        || filtered.as_slice()
            == [
                "this", "fourth", "time", "this", "ability", "has", "resolved", "this", "turn",
            ]
        || filtered.as_slice()
            == [
                "this",
                "fourth",
                "time",
                "this",
                "ability",
                "has",
                "triggered",
                "this",
                "turn",
            ]
        || filtered.as_slice()
            == [
                "this",
                "ability",
                "has",
                "been",
                "activated",
                "four",
                "or",
                "more",
                "times",
                "this",
                "turn",
            ]
        || filtered.as_slice() == ["it", "first", "combat", "phase", "of", "turn"]
        || filtered.as_slice()
            == [
                "you", "would", "begin", "your", "turn", "while", "this", "artifact", "is",
                "tapped",
            ]
        || filtered.as_slice() == ["player", "is", "dealt", "damage", "this", "way"]
        || filtered.as_slice()
            == [
                "two",
                "or",
                "more",
                "creatures",
                "are",
                "tied",
                "for",
                "least",
                "power",
            ]
        || filtered.as_slice()
            == [
                "card",
                "would",
                "be",
                "put",
                "into",
                "opponents",
                "graveyard",
                "from",
                "anywhere",
            ]
        || filtered.as_slice() == ["the", "number", "is", "odd"]
        || filtered.as_slice() == ["the", "number", "is", "even"]
        || filtered.as_slice() == ["number", "is", "odd"]
        || filtered.as_slice() == ["number", "is", "even"]
        || filtered.as_slice() == ["the", "number", "of", "permanents", "is", "odd"]
        || filtered.as_slice() == ["the", "number", "of", "permanents", "is", "even"]
        || filtered.as_slice() == ["number", "of", "permanents", "is", "odd"]
        || filtered.as_slice() == ["number", "of", "permanents", "is", "even"];
    if unsupported_unmodeled {
        return Ok(PredicateAst::Unmodeled(filtered.join(" ")));
    }

    Err(CardTextError::ParseError(format!(
        "unsupported predicate (predicate: '{}')",
        filtered.join(" ")
    )))
}

fn parse_graveyard_threshold_predicate(
    filtered: &[&str],
) -> Result<Option<PredicateAst>, CardTextError> {
    let (count, tail_start, constrained_player) = if filtered.len() >= 5
        && filtered[0] == "there"
        && filtered[1] == "are"
        && filtered[3] == "or"
        && filtered[4] == "more"
    {
        let Some(count) = parse_named_number(filtered[2]) else {
            return Ok(None);
        };
        (count, 5usize, None)
    } else if filtered.len() >= 5
        && filtered[0] == "you"
        && filtered[1] == "have"
        && filtered[3] == "or"
        && filtered[4] == "more"
    {
        let Some(count) = parse_named_number(filtered[2]) else {
            return Ok(None);
        };
        (count, 5usize, Some(PlayerAst::You))
    } else {
        return Ok(None);
    };

    let tail = &filtered[tail_start..];
    let Some(in_idx) = rfind_index(tail, |word| *word == "in") else {
        return Ok(None);
    };
    if in_idx == 0 || in_idx + 1 >= tail.len() {
        return Ok(None);
    }

    let graveyard_owner_words = &tail[in_idx + 1..];
    let player = match graveyard_owner_words {
        ["your", "graveyard"] => PlayerAst::You,
        ["that", "player", "graveyard"] | ["that", "players", "graveyard"] => PlayerAst::That,
        ["target", "player", "graveyard"] | ["target", "players", "graveyard"] => PlayerAst::Target,
        ["target", "opponent", "graveyard"] | ["target", "opponents", "graveyard"] => {
            PlayerAst::TargetOpponent
        }
        ["opponent", "graveyard"] | ["opponents", "graveyard"] => PlayerAst::Opponent,
        _ => return Ok(None),
    };
    if constrained_player.is_some_and(|expected| expected != player) {
        return Ok(None);
    }

    let raw_filter_words = &tail[..in_idx];
    if raw_filter_words.is_empty()
        || slice_contains(raw_filter_words, &"type")
        || slice_contains(raw_filter_words, &"types")
    {
        return Ok(None);
    }

    let mut normalized_filter_words = Vec::with_capacity(raw_filter_words.len());
    for (idx, word) in raw_filter_words.iter().enumerate() {
        if *word == "and"
            && raw_filter_words
                .get(idx + 1)
                .is_some_and(|next| *next == "or")
        {
            continue;
        }
        normalized_filter_words.push(*word);
    }
    if normalized_filter_words.is_empty() {
        return Ok(None);
    }

    let mut filter = if matches!(normalized_filter_words.as_slice(), ["card"] | ["cards"]) {
        ObjectFilter::default()
    } else {
        let filter_tokens = normalized_filter_words
            .iter()
            .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
            .collect::<Vec<_>>();
        let Ok(filter) = parse_object_filter(&filter_tokens, false) else {
            return Ok(None);
        };
        filter
    };
    filter.zone = Some(Zone::Graveyard);

    Ok(Some(PredicateAst::PlayerControlsAtLeast {
        player,
        filter,
        count,
    }))
}

fn parse_mana_spent_to_cast_predicate(words: &[&str]) -> Option<(u32, Option<ManaSymbol>)> {
    if words.len() < 10 || words[0] != "at" || words[1] != "least" {
        return None;
    }

    let amount_tokens = vec![OwnedLexToken::word(
        words[2].to_string(),
        TextSpan::synthetic(),
    )];
    let (amount, _) = parse_number(&amount_tokens)?;

    let mut idx = 3;
    if words.get(idx).copied() == Some("of") {
        idx += 1;
    }

    let symbol = if let Some(word) = words.get(idx).copied() {
        if let Some(parsed) = parse_mana_symbol_word(word) {
            idx += 1;
            Some(parsed)
        } else {
            None
        }
    } else {
        None
    };

    let tail = &words[idx..];
    let canonical_tail = ["mana", "was", "spent", "to", "cast", "this", "spell"];
    let plural_tail = ["mana", "were", "spent", "to", "cast", "this", "spell"];
    if tail == canonical_tail || tail == plural_tail {
        return Some((amount, symbol));
    }

    None
}

fn parse_mana_symbol_word(word: &str) -> Option<ManaSymbol> {
    parse_mana_symbol_word_flexible(word)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::builders::parser::lexer::lex_line;
    use crate::static_abilities::StaticAbilityId;

    #[test]
    fn parse_object_filter_lexed_handles_with_keyword_disjunction() {
        let tokens = lex_line("creatures with flying or reach", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert_eq!(filter.any_of.len(), 2);
        assert_eq!(
            filter.any_of[0].static_abilities,
            vec![StaticAbilityId::Flying]
        );
        assert_eq!(
            filter.any_of[1].static_abilities,
            vec![StaticAbilityId::Reach]
        );
    }

    #[test]
    fn parse_object_filter_lexed_handles_without_keyword_clause() {
        let tokens = lex_line("creatures without flying", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert_eq!(
            filter.excluded_static_abilities,
            vec![StaticAbilityId::Flying]
        );
    }

    #[test]
    fn parse_object_filter_lexed_handles_with_no_abilities_clause() {
        let tokens = lex_line("creatures with no abilities", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert!(filter.no_abilities);
    }

    #[test]
    fn parse_object_filter_lexed_handles_joint_owner_controller_clause() {
        let tokens = lex_line("permanents you both own and control", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.owner, Some(PlayerFilter::You));
        assert_eq!(filter.controller, Some(PlayerFilter::You));
        assert_eq!(filter.zone, Some(Zone::Battlefield));
    }

    #[test]
    fn parse_object_filter_lexed_handles_chosen_player_graveyard_clause() {
        let tokens = lex_line("artifact card in the chosen player's graveyard", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.card_types, vec![CardType::Artifact]);
        assert_eq!(filter.owner, Some(PlayerFilter::ChosenPlayer));
        assert_eq!(filter.zone, Some(Zone::Graveyard));
    }

    #[test]
    fn parse_object_filter_lexed_handles_owner_or_controller_disjunction() {
        let tokens = lex_line("artifacts target opponent owns or controls", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.any_of.len(), 2);
        assert_eq!(filter.any_of[0].card_types, vec![CardType::Artifact]);
        assert_eq!(
            filter.any_of[0].owner,
            Some(PlayerFilter::target_opponent())
        );
        assert_eq!(filter.any_of[0].controller, None);
        assert_eq!(filter.any_of[1].card_types, vec![CardType::Artifact]);
        assert_eq!(filter.any_of[1].owner, None);
        assert_eq!(
            filter.any_of[1].controller,
            Some(PlayerFilter::target_opponent())
        );
    }

    #[test]
    fn parse_object_filter_lexed_handles_target_player_reference_clause() {
        let tokens = lex_line("spell that targets player", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.targets_player, Some(PlayerFilter::Any));
    }

    #[test]
    fn parse_object_filter_lexed_handles_attacking_target_opponent_clause() {
        let tokens = lex_line("creature attacking target opponent", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert_eq!(
            filter.attacking_player_or_planeswalker_controlled_by,
            Some(PlayerFilter::target_opponent())
        );
    }

    #[test]
    fn temporal_graveyard_from_battlefield_phrase_parser_matches() {
        assert_eq!(
            parse_graveyard_from_battlefield_this_turn_words(&[
                "graveyard",
                "from",
                "battlefield",
                "this",
                "turn",
            ]),
            Some(5)
        );
    }

    #[test]
    fn parse_object_filter_lexed_handles_put_there_from_anywhere_this_turn_clause() {
        let tokens = lex_line(
            "creature cards in a graveyard that were put there from anywhere this turn",
            0,
        )
        .unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert_eq!(filter.zone, Some(Zone::Graveyard));
        assert!(filter.entered_graveyard_this_turn);
    }

    #[test]
    fn parse_object_filter_lexed_handles_entered_battlefield_this_turn_clause() {
        let tokens = lex_line(
            "creatures that entered the battlefield under your control this turn",
            0,
        )
        .unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert_eq!(filter.zone, Some(Zone::Battlefield));
        assert!(filter.entered_battlefield_this_turn);
        assert_eq!(
            filter.entered_battlefield_controller,
            Some(PlayerFilter::You)
        );
    }

    #[test]
    fn parse_object_filter_lexed_handles_named_clause_with_trailing_zone() {
        let tokens = lex_line("artifact card named Sol Ring from your graveyard", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.card_types, vec![CardType::Artifact]);
        assert_eq!(filter.name.as_deref(), Some("sol ring"));
        assert_eq!(filter.owner, Some(PlayerFilter::You));
        assert_eq!(filter.zone, Some(Zone::Graveyard));
    }

    #[test]
    fn parse_object_filter_lexed_handles_not_named_clause_with_trailing_zone() {
        let tokens = lex_line("artifact card not named Sol Ring from your graveyard", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.card_types, vec![CardType::Artifact]);
        assert_eq!(filter.excluded_name.as_deref(), Some("sol ring"));
        assert_eq!(filter.owner, Some(PlayerFilter::You));
        assert_eq!(filter.zone, Some(Zone::Graveyard));
    }

    #[test]
    fn parse_object_filter_lexed_handles_tagged_reference_prefix() {
        let tokens = lex_line("that creature", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert_eq!(
            filter.tagged_constraints,
            vec![TaggedObjectConstraint {
                tag: TagKey::from(IT_TAG),
                relation: TaggedOpbjectRelation::IsTaggedObject,
            }]
        );
    }

    #[test]
    fn parse_object_filter_lexed_handles_entered_since_your_last_turn_ended_clause() {
        let tokens = lex_line("creatures that entered since your last turn ended", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert!(filter.entered_since_your_last_turn_ended);
    }

    #[test]
    fn parse_object_filter_lexed_handles_split_face_state_words() {
        let tokens = lex_line("face up creature cards", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert_eq!(filter.face_down, Some(false));
    }

    #[test]
    fn parse_object_filter_lexed_handles_single_graveyard_phrase() {
        let tokens = lex_line("creature cards in a single graveyard", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert_eq!(filter.zone, Some(Zone::Graveyard));
        assert!(filter.single_graveyard);
    }

    #[test]
    fn parse_object_filter_lexed_handles_one_or_more_colors_phrase() {
        let tokens = lex_line("creatures of one or more colors", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        let any_color: ColorSet = Color::ALL.into_iter().collect();
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert_eq!(filter.colors, Some(any_color));
    }

    #[test]
    fn parse_object_filter_lexed_handles_mana_value_eq_counters_on_source_clause() {
        let tokens = lex_line(
            "creature card with mana value equal to the number of charge counters on this artifact",
            0,
        )
        .unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert_eq!(
            filter.mana_value_eq_counters_on_source,
            Some(crate::object::CounterType::Charge)
        );
    }

    #[test]
    fn parse_object_filter_lexed_handles_attached_exclusion_phrase() {
        let tokens = lex_line("creatures other than enchanted creature", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert!(filter.tagged_constraints.iter().any(|constraint| {
            *constraint
                == TaggedObjectConstraint {
                    tag: TagKey::from("enchanted"),
                    relation: TaggedOpbjectRelation::IsNotTaggedObject,
                }
        }));
    }

    #[test]
    fn parse_object_filter_lexed_handles_different_one_of_prefix() {
        let tokens = lex_line("different one of those creatures", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert!(filter.tagged_constraints.iter().any(|constraint| {
            *constraint
                == TaggedObjectConstraint {
                    tag: TagKey::from(IT_TAG),
                    relation: TaggedOpbjectRelation::IsTaggedObject,
                }
        }));
    }

    #[test]
    fn parse_object_filter_lexed_handles_pt_literal_prefix() {
        let tokens = lex_line("2/2 creature token", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert_eq!(filter.power, Some(crate::filter::Comparison::Equal(2)));
        assert_eq!(filter.toughness, Some(crate::filter::Comparison::Equal(2)));
    }

    #[test]
    fn parse_object_filter_lexed_handles_not_all_colors_clause() {
        let tokens = lex_line("creature that isnt all colors", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert_eq!(filter.all_colors, Some(false));
    }

    #[test]
    fn parse_object_filter_lexed_handles_not_exactly_two_colors_clause() {
        let tokens = lex_line("creature that isnt exactly two colors", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert_eq!(filter.exactly_two_colors, Some(false));
    }

    #[test]
    fn parse_object_filter_lexed_handles_attached_to_tagged_reference() {
        let tokens = lex_line("creature attached to it", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert!(filter.tagged_constraints.iter().any(|constraint| {
            *constraint
                == TaggedObjectConstraint {
                    tag: TagKey::from(IT_TAG),
                    relation: TaggedOpbjectRelation::AttachedToTaggedObject,
                }
        }));
    }

    #[test]
    fn parse_object_filter_lexed_handles_its_attached_to_reference_alias() {
        let tokens = lex_line("creature its attached to", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert!(filter.tagged_constraints.iter().any(|constraint| {
            *constraint
                == TaggedObjectConstraint {
                    tag: TagKey::from(IT_TAG),
                    relation: TaggedOpbjectRelation::AttachedToTaggedObject,
                }
        }));
    }

    #[test]
    fn parse_object_filter_lexed_handles_source_linked_exile_reference() {
        let tokens = lex_line("spell exiled with this", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.zone, Some(Zone::Stack));
        assert!(filter.tagged_constraints.iter().any(|constraint| {
            *constraint
                == TaggedObjectConstraint {
                    tag: TagKey::from(crate::tag::SOURCE_EXILED_TAG),
                    relation: TaggedOpbjectRelation::IsTaggedObject,
                }
        }));
    }

    #[test]
    fn parse_object_filter_lexed_handles_same_mana_value_as_sacrificed_reference() {
        let tokens = lex_line(
            "creature with same mana value as the sacrificed creature",
            0,
        )
        .unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert!(filter.tagged_constraints.iter().any(|constraint| {
            *constraint
                == TaggedObjectConstraint {
                    tag: TagKey::from("sacrifice_cost_0"),
                    relation: TaggedOpbjectRelation::SameManaValueAsTagged,
                }
        }));
    }

    #[test]
    fn parse_object_filter_lexed_handles_same_name_as_tagged_reference() {
        let tokens = lex_line("creature with same name as that creature", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert!(filter.tagged_constraints.iter().any(|constraint| {
            *constraint
                == TaggedObjectConstraint {
                    tag: TagKey::from(IT_TAG),
                    relation: TaggedOpbjectRelation::SameNameAsTagged,
                }
        }));
    }

    #[test]
    fn parse_object_filter_lexed_handles_tap_activated_ability_phrase() {
        let tokens = lex_line(
            "creature with activated abilities with {T} in their costs",
            0,
        )
        .unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert!(filter.has_tap_activated_ability);
    }

    #[test]
    fn parse_object_filter_lexed_handles_convoked_it_reference() {
        let tokens = lex_line("creature that convoked it", 0).unwrap();

        let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert!(filter.tagged_constraints.iter().any(|constraint| {
            *constraint
                == TaggedObjectConstraint {
                    tag: TagKey::from("convoked_this_spell"),
                    relation: TaggedOpbjectRelation::IsTaggedObject,
                }
        }));
    }

    #[test]
    fn parse_spell_filter_lexed_handles_split_face_state_words() {
        let tokens = lex_line("Face up noncreature spells", 0).unwrap();

        let filter = parse_spell_filter_with_grammar_entrypoint_lexed(&tokens);
        assert_eq!(filter.face_down, Some(false));
        assert_eq!(filter.excluded_card_types, vec![CardType::Creature]);
    }

    #[test]
    fn parse_spell_filter_raw_handles_hyphenated_face_state_words() {
        let tokens = lex_line("face-down noncreature spells", 0).unwrap();

        let filter = parse_spell_filter_with_grammar_entrypoint(&tokens);
        assert_eq!(filter.face_down, Some(true));
        assert_eq!(filter.excluded_card_types, vec![CardType::Creature]);
    }

    #[test]
    fn parse_spell_filter_lexed_builds_power_or_toughness_disjunction() {
        let tokens = lex_line("creature spells with power or toughness 2 or less", 0).unwrap();

        let filter = parse_spell_filter_with_grammar_entrypoint_lexed(&tokens);
        assert_eq!(filter.any_of.len(), 2);
        assert_eq!(filter.any_of[0].card_types, vec![CardType::Creature]);
        assert!(filter.any_of[0].power.is_some());
        assert!(filter.any_of[0].toughness.is_none());
        assert_eq!(filter.any_of[1].card_types, vec![CardType::Creature]);
        assert!(filter.any_of[1].power.is_none());
        assert!(filter.any_of[1].toughness.is_some());
    }

    #[test]
    fn parse_spell_filter_lexed_handles_even_mana_value_phrase() {
        let tokens = lex_line("even mana value spells", 0).unwrap();

        let filter = parse_spell_filter_with_grammar_entrypoint_lexed(&tokens);
        assert_eq!(
            filter.mana_value_parity,
            Some(crate::filter::ParityRequirement::Even)
        );
    }
}
