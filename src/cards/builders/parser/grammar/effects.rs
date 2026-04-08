use winnow::combinator::{alt, dispatch, fail, opt, peek};
use winnow::error::{ContextError, ErrMode};
use winnow::prelude::*;
use winnow::token::take_till;

use crate::cards::builders::{
    CardTextError, ChoiceCount, EffectAst, IT_TAG, LibraryBottomOrderAst, LibraryConsultModeAst,
    LibraryConsultStopRuleAst, PlayerAst, ReturnControllerAst, SubjectAst, TagKey, TargetAst,
    TextSpan,
};
use crate::effect::SearchSelectionMode;
use crate::target::PlayerFilter;
use crate::target::{ObjectFilter, TaggedObjectConstraint, TaggedOpbjectRelation};
use crate::zone::Zone;

use super::super::activation_and_restrictions::{
    parse_cant_restriction_clause, parse_cant_restrictions,
};
use super::super::effect_sentences::{
    apply_search_library_mana_constraint, extract_search_library_mana_constraint, find_verb_lexed,
    is_same_name_that_reference_words, normalize_search_library_filter, parse_effect_chain_lexed,
    parse_effect_chain_with_sentence_primitives_lexed, parse_effect_clause_lexed,
    parse_restriction_duration_lexed, parse_search_library_disjunction_filter,
    split_search_same_name_reference_filter, word_slice_mentions_nth_from_top,
    word_slice_starts_with_any, zone_slice_contains,
};
use super::super::grammar::structure::{IfClausePredicateSpec, split_if_clause_lexed};
use super::super::lexer::{
    LexStream, OwnedLexToken, TokenKind, parser_token_word_positions, parser_token_word_refs,
    split_lexed_sentences, token_word_refs,
};
use super::super::token_primitives::{
    contains_window as word_slice_contains_sequence, find_any_str_index as word_slice_find_any,
    find_index as find_token_index, find_str_index as word_slice_find,
    find_window_index as word_slice_find_sequence, rfind_index as rfind_token_index,
    slice_contains_all as word_slice_has_all, slice_contains_any as word_slice_contains_any,
    slice_contains_str as word_slice_contains, slice_ends_with as word_slice_ends_with,
    slice_starts_with as word_slice_starts_with,
};
use super::super::util::{is_article, parse_subject};
use super::super::{parse_number, parse_object_filter, parse_object_filter_lexed};
use super::super::{parse_target_phrase, span_from_tokens, trim_commas};
use super::primitives;

const CHOSEN_NAME_TAG: &str = "__chosen_name__";

fn parser_text_word_refs(tokens: &[OwnedLexToken]) -> Vec<&str> {
    parser_token_word_refs(tokens)
}

fn parser_word_token_positions(tokens: &[OwnedLexToken]) -> Vec<(usize, &str)> {
    parser_token_word_positions(tokens)
}

fn find_parser_word_position(parser_words: &[(usize, &str)], expected: &str) -> Option<usize> {
    let mut idx = 0usize;
    while idx < parser_words.len() {
        if parser_words[idx].1 == expected {
            return Some(idx);
        }
        idx += 1;
    }
    None
}

fn last_non_article_parser_word_token_idx(
    parser_words: &[(usize, &str)],
    end_exclusive: usize,
) -> Option<usize> {
    let mut idx = end_exclusive;
    while idx > 0 {
        idx -= 1;
        if !is_article(parser_words[idx].1) {
            return Some(parser_words[idx].0);
        }
    }
    None
}

fn normalize_subject_routing_word(word: &str) -> String {
    let bytes = word.as_bytes();
    if bytes.len() >= 2 && bytes[bytes.len() - 2] == b'\'' && bytes[bytes.len() - 1] == b's' {
        let stem = &word[..word.len() - 2];
        return format!("{stem}s");
    }
    if bytes.last() == Some(&b'\'') {
        return word[..word.len() - 1].to_string();
    }
    word.to_string()
}

fn subject_routing_word_refs(tokens: &[OwnedLexToken]) -> Vec<String> {
    parser_text_word_refs(tokens)
        .into_iter()
        .map(normalize_subject_routing_word)
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SearchLibrarySentenceHeadKind {
    Plain,
    DirectMay,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SearchLibrarySentenceHeadSplit<'a> {
    pub(crate) subject_tokens: &'a [OwnedLexToken],
    pub(crate) search_tokens: &'a [OwnedLexToken],
    pub(crate) sentence_has_direct_may: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SearchLibraryClauseMarkers {
    pub(crate) for_idx: usize,
    pub(crate) put_idx: Option<usize>,
    pub(crate) exile_idx: Option<usize>,
    pub(crate) reveal_idx: Option<usize>,
    pub(crate) shuffle_idx: Option<usize>,
    pub(crate) filter_boundary: usize,
    pub(crate) has_explicit_destination: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SearchLibraryFilterBoundary {
    pub(crate) filter_end: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SearchLibraryDiscardFollowupBoundary {
    pub(crate) discard_idx: usize,
    pub(crate) discard_end: usize,
    pub(crate) shuffle_idx: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SearchLibraryEffectRouting {
    pub(crate) destination: Zone,
    pub(crate) reveal: bool,
    pub(crate) shuffle: bool,
    pub(crate) face_down_exile: bool,
    pub(crate) split_battlefield_and_hand: bool,
    pub(crate) has_tapped_modifier: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SearchLibrarySubjectRouting {
    pub(crate) player: PlayerAst,
    pub(crate) search_player_target: Option<TargetAst>,
    pub(crate) forced_library_owner: Option<PlayerFilter>,
    pub(crate) search_zones_override: Option<Vec<Zone>>,
}

#[derive(Debug, Clone)]
pub(crate) struct SearchLibraryCountPrefix {
    pub(crate) count: ChoiceCount,
    pub(crate) search_mode: SearchSelectionMode,
    pub(crate) count_used: usize,
}

#[derive(Debug, Clone)]
pub(crate) enum SearchLibrarySameNameReference {
    Tagged(TagKey),
    Target(TargetAst),
    Choose { filter: ObjectFilter, tag: TagKey },
}

#[derive(Debug, Clone)]
pub(crate) struct SearchLibrarySameNameSplit {
    pub(crate) filter_tokens: Vec<OwnedLexToken>,
    pub(crate) same_name_reference: Option<SearchLibrarySameNameReference>,
}

#[derive(Debug, Clone)]
pub(crate) struct SearchLibraryLeadingPrelude<'a> {
    pub(crate) subject_tokens: &'a [OwnedLexToken],
    pub(crate) leading_effects: Vec<EffectAst>,
}

fn conditional_label_phrase<'a>(input: &mut LexStream<'a>) -> Result<(), ErrMode<ContextError>> {
    dispatch! {peek(primitives::word_parser_text);
        "adamant" => primitives::phrase(&["adamant"]),
        "addendum" => primitives::phrase(&["addendum"]),
        "ascend" => primitives::phrase(&["ascend"]),
        "battalion" => primitives::phrase(&["battalion"]),
        "delirium" => primitives::phrase(&["delirium"]),
        "domain" => primitives::phrase(&["domain"]),
        "ferocious" => primitives::phrase(&["ferocious"]),
        "formidable" => primitives::phrase(&["formidable"]),
        "hellbent" => primitives::phrase(&["hellbent"]),
        "metalcraft" => primitives::phrase(&["metalcraft"]),
        "morbid" => primitives::phrase(&["morbid"]),
        "raid" => primitives::phrase(&["raid"]),
        "revolt" => primitives::phrase(&["revolt"]),
        "spectacle" => primitives::phrase(&["spectacle"]),
        "spell" => primitives::phrase(&["spell", "mastery"]),
        "surge" => primitives::phrase(&["surge"]),
        "threshold" => primitives::phrase(&["threshold"]),
        "undergrowth" => primitives::phrase(&["undergrowth"]),
        _ => fail::<_, (), _>,
    }
    .parse_next(input)
}

fn search_library_sentence_head<'a>(
    input: &mut LexStream<'a>,
) -> Result<(&'a [OwnedLexToken], SearchLibrarySentenceHeadKind), ErrMode<ContextError>> {
    let subject_tokens = take_till(0.., |token: &OwnedLexToken| {
        token.is_word("unless")
            || token.is_word("may")
            || token.is_word("search")
            || token.is_word("searches")
    })
    .parse_next(input)?;

    alt((
        (
            primitives::kw("may"),
            alt((primitives::kw("search"), primitives::kw("searches"))),
        )
            .value((subject_tokens, SearchLibrarySentenceHeadKind::DirectMay)),
        alt((primitives::kw("search"), primitives::kw("searches")))
            .value((subject_tokens, SearchLibrarySentenceHeadKind::Plain)),
    ))
    .parse_next(input)
}

pub(crate) fn split_search_library_sentence_head_lexed(
    tokens: &[OwnedLexToken],
) -> Option<SearchLibrarySentenceHeadSplit<'_>> {
    let ((subject_tokens, head_kind), _) =
        primitives::parse_prefix(tokens, search_library_sentence_head)?;
    let search_start = subject_tokens.len()
        + match head_kind {
            SearchLibrarySentenceHeadKind::Plain => 0,
            SearchLibrarySentenceHeadKind::DirectMay => 1,
        };

    Some(SearchLibrarySentenceHeadSplit {
        subject_tokens,
        search_tokens: &tokens[search_start..],
        sentence_has_direct_may: matches!(head_kind, SearchLibrarySentenceHeadKind::DirectMay),
    })
}

fn search_library_search_verb<'a>(input: &mut LexStream<'a>) -> Result<(), ErrMode<ContextError>> {
    alt((primitives::kw("search"), primitives::kw("searches")))
        .void()
        .parse_next(input)
}

fn search_library_put_marker<'a>(input: &mut LexStream<'a>) -> Result<(), ErrMode<ContextError>> {
    alt((primitives::kw("put"), primitives::kw("puts")))
        .void()
        .parse_next(input)
}

fn search_library_reveal_marker<'a>(
    input: &mut LexStream<'a>,
) -> Result<(), ErrMode<ContextError>> {
    alt((primitives::kw("reveal"), primitives::kw("reveals")))
        .void()
        .parse_next(input)
}

fn search_library_shuffle_marker<'a>(
    input: &mut LexStream<'a>,
) -> Result<(), ErrMode<ContextError>> {
    alt((primitives::kw("shuffle"), primitives::kw("shuffles")))
        .void()
        .parse_next(input)
}

fn search_library_for_marker<'a>(input: &mut LexStream<'a>) -> Result<(), ErrMode<ContextError>> {
    primitives::kw("for").void().parse_next(input)
}

fn search_library_exile_destination_marker<'a>(
    input: &mut LexStream<'a>,
) -> Result<(), ErrMode<ContextError>> {
    (
        alt((primitives::kw("exile"), primitives::kw("exiles"))),
        alt((
            primitives::phrase(&["it"]),
            primitives::phrase(&["them"]),
            primitives::phrase(&["that", "card"]),
            primitives::phrase(&["those", "cards"]),
        )),
    )
        .void()
        .parse_next(input)
}

fn search_library_then_marker<'a>(input: &mut LexStream<'a>) -> Result<(), ErrMode<ContextError>> {
    primitives::kw("then").void().parse_next(input)
}

fn search_library_and_marker<'a>(input: &mut LexStream<'a>) -> Result<(), ErrMode<ContextError>> {
    primitives::kw("and").void().parse_next(input)
}

fn search_library_discard_marker<'a>(
    input: &mut LexStream<'a>,
) -> Result<(), ErrMode<ContextError>> {
    alt((primitives::kw("discard"), primitives::kw("discards")))
        .void()
        .parse_next(input)
}

fn search_library_reveal_or_then_marker<'a>(
    input: &mut LexStream<'a>,
) -> Result<(), ErrMode<ContextError>> {
    alt((search_library_reveal_marker, search_library_then_marker)).parse_next(input)
}

fn search_library_comma_filter_break_marker<'a>(
    input: &mut LexStream<'a>,
) -> Result<(), ErrMode<ContextError>> {
    (
        primitives::comma(),
        alt((
            search_library_put_marker,
            search_library_reveal_marker,
            search_library_then_marker,
        )),
    )
        .void()
        .parse_next(input)
}

fn search_library_with_that_name_suffix<'a>(
    input: &mut LexStream<'a>,
) -> Result<(), ErrMode<ContextError>> {
    primitives::phrase(&["with", "that", "name"])
        .void()
        .parse_next(input)
}

fn search_library_with_the_chosen_name_suffix<'a>(
    input: &mut LexStream<'a>,
) -> Result<(), ErrMode<ContextError>> {
    primitives::phrase(&["with", "the", "chosen", "name"])
        .void()
        .parse_next(input)
}

fn search_library_with_chosen_name_suffix<'a>(
    input: &mut LexStream<'a>,
) -> Result<(), ErrMode<ContextError>> {
    primitives::phrase(&["with", "chosen", "name"])
        .void()
        .parse_next(input)
}

fn strip_search_library_suffix_lexed(
    tokens: &[OwnedLexToken],
    parser: for<'a> fn(&mut LexStream<'a>) -> Result<(), ErrMode<ContextError>>,
) -> Option<Vec<OwnedLexToken>> {
    let trimmed = trim_commas(tokens);
    let mut cursor = 0usize;

    while cursor < trimmed.len() {
        let Some((_, rest)) = primitives::parse_prefix(&trimmed[cursor..], parser) else {
            cursor += 1;
            continue;
        };
        if rest.is_empty() {
            return Some(trim_commas(&trimmed[..cursor]));
        }
        cursor += 1;
    }

    None
}

fn strip_search_library_leading_count_tokens(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    let tokens = trim_commas(tokens);
    if let Some((_, rest)) = primitives::parse_prefix(&tokens, primitives::kw("exactly"))
        && let Some((_, used)) = parse_number(rest)
    {
        return trim_commas(&rest[used..]);
    }
    if let Some((_, used)) = parse_number(&tokens) {
        return trim_commas(&tokens[used..]);
    }
    tokens
}

fn is_default_search_library_card_selector(tokens: &[OwnedLexToken]) -> bool {
    let words = parser_text_word_refs(tokens)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect::<Vec<_>>();
    words.is_empty() || words.as_slice() == ["card"] || words.as_slice() == ["cards"]
}

fn find_search_library_marker_lexed(
    tokens: &[OwnedLexToken],
    parser: for<'a> fn(&mut LexStream<'a>) -> Result<(), ErrMode<ContextError>>,
) -> Option<usize> {
    let mut cursor = 0usize;

    while cursor < tokens.len() {
        if primitives::parse_prefix(&tokens[cursor..], parser).is_some() {
            return Some(cursor);
        }
        cursor += 1;
    }

    None
}

fn find_last_search_library_marker_lexed(
    tokens: &[OwnedLexToken],
    parser: for<'a> fn(&mut LexStream<'a>) -> Result<(), ErrMode<ContextError>>,
) -> Option<usize> {
    let mut cursor = 0usize;
    let mut last_match = None;

    while cursor < tokens.len() {
        if primitives::parse_prefix(&tokens[cursor..], parser).is_some() {
            last_match = Some(cursor);
        }
        cursor += 1;
    }

    last_match
}

pub(crate) fn scan_search_library_clause_markers_lexed(
    search_tokens: &[OwnedLexToken],
) -> Option<SearchLibraryClauseMarkers> {
    let for_idx = find_search_library_marker_lexed(search_tokens, search_library_search_verb)
        .and_then(|search_idx| {
            find_search_library_marker_lexed(
                &search_tokens[search_idx..],
                search_library_for_marker,
            )
            .map(|relative_for_idx| search_idx + relative_for_idx)
        })
        .unwrap_or(3);
    let put_idx = find_search_library_marker_lexed(search_tokens, search_library_put_marker);
    let exile_idx =
        find_search_library_marker_lexed(search_tokens, search_library_exile_destination_marker);
    let reveal_idx = find_search_library_marker_lexed(search_tokens, search_library_reveal_marker);
    let shuffle_idx =
        find_search_library_marker_lexed(search_tokens, search_library_shuffle_marker);
    let has_explicit_destination = put_idx.is_some() || exile_idx.is_some();
    let filter_boundary = put_idx
        .or(exile_idx)
        .or(reveal_idx)
        .or(shuffle_idx)
        .unwrap_or(search_tokens.len());

    Some(SearchLibraryClauseMarkers {
        for_idx,
        put_idx,
        exile_idx,
        reveal_idx,
        shuffle_idx,
        filter_boundary,
        has_explicit_destination,
    })
}

pub(crate) fn find_search_library_filter_boundary_lexed(
    search_tokens: &[OwnedLexToken],
    for_idx: usize,
    filter_boundary: usize,
) -> SearchLibraryFilterBoundary {
    let mut filter_end = find_search_library_marker_lexed(
        &search_tokens[for_idx + 1..filter_boundary],
        search_library_comma_filter_break_marker,
    )
    .map(|relative_idx| for_idx + 1 + relative_idx)
    .unwrap_or(filter_boundary);

    if filter_end == filter_boundary
        && let Some(idx) =
            find_search_library_marker_lexed(search_tokens, search_library_reveal_or_then_marker)
    {
        filter_end = filter_end.min(idx);
    }

    while filter_end > for_idx + 1 {
        let token = &search_tokens[filter_end - 1];
        if token.is_comma() || token.is_word("and") || token.is_word("then") {
            filter_end -= 1;
        } else {
            break;
        }
    }

    SearchLibraryFilterBoundary { filter_end }
}

pub(crate) fn find_search_library_discard_before_shuffle_followup_lexed(
    search_tokens: &[OwnedLexToken],
    put_idx: Option<usize>,
) -> Option<SearchLibraryDiscardFollowupBoundary> {
    let put_idx = put_idx?;
    let discard_idx =
        find_search_library_marker_lexed(search_tokens, search_library_discard_marker)?;
    let shuffle_idx =
        find_last_search_library_marker_lexed(search_tokens, search_library_shuffle_marker)?;
    if !(discard_idx > put_idx && discard_idx < shuffle_idx) {
        return None;
    }

    let mut discard_end = shuffle_idx;
    while discard_end > discard_idx {
        let token = &search_tokens[discard_end - 1];
        if token.is_comma() || token.is_word("then") || token.is_word("and") {
            discard_end -= 1;
            continue;
        }
        break;
    }

    Some(SearchLibraryDiscardFollowupBoundary {
        discard_idx,
        discard_end,
        shuffle_idx,
    })
}

pub(crate) fn find_search_library_trailing_life_followup_lexed<'a>(
    search_tokens: &'a [OwnedLexToken],
    start_idx: usize,
) -> Option<&'a [OwnedLexToken]> {
    let and_idx =
        find_search_library_marker_lexed(&search_tokens[start_idx..], search_library_and_marker)?;
    let and_idx = start_idx + and_idx;
    let mut trailing_start = and_idx + 1;
    let mut trailing_end = search_tokens.len();
    while trailing_start < trailing_end && search_tokens[trailing_start].is_comma() {
        trailing_start += 1;
    }
    while trailing_end > trailing_start && search_tokens[trailing_end - 1].is_comma() {
        trailing_end -= 1;
    }
    let trailing_tokens = &search_tokens[trailing_start..trailing_end];
    if trailing_tokens.is_empty() {
        return None;
    }

    let trailing_words = parser_text_word_refs(trailing_tokens);
    let starts_with_life_clause = word_slice_starts_with_any(
        &trailing_words,
        &[
            &["you", "gain"],
            &["target", "player", "gains"],
            &["target", "player", "gain"],
        ],
    );

    starts_with_life_clause.then_some(trailing_tokens)
}

pub(crate) fn derive_search_library_effect_routing_lexed(
    tokens: &[OwnedLexToken],
    search_tokens: &[OwnedLexToken],
    clause_markers: SearchLibraryClauseMarkers,
    trailing_discard_before_shuffle: bool,
) -> SearchLibraryEffectRouting {
    let words_all = parser_text_word_refs(tokens);
    let destination = if let Some(put_idx) = clause_markers.put_idx {
        let put_clause_words = parser_text_word_refs(&search_tokens[put_idx..]);
        if word_slice_contains(&put_clause_words, "graveyard") {
            Zone::Graveyard
        } else if word_slice_contains(&put_clause_words, "hand") {
            Zone::Hand
        } else if word_slice_contains(&put_clause_words, "top") {
            Zone::Library
        } else {
            Zone::Battlefield
        }
    } else {
        Zone::Exile
    };
    let reveal = clause_markers.reveal_idx.is_some();
    let face_down_exile = clause_markers.exile_idx.is_some_and(|idx| {
        word_slice_contains_sequence(
            &parser_text_word_refs(&search_tokens[idx..]),
            &["face", "down"],
        )
    });
    let shuffle = clause_markers.shuffle_idx.is_some() && !trailing_discard_before_shuffle;
    let split_battlefield_and_hand = clause_markers.put_idx.is_some()
        && word_slice_has_all(&words_all, &["battlefield", "hand", "other", "one"]);
    let has_tapped_modifier = word_slice_contains(&words_all, "tapped");

    SearchLibraryEffectRouting {
        destination,
        reveal,
        shuffle,
        face_down_exile,
        split_battlefield_and_hand,
        has_tapped_modifier,
    }
}

pub(crate) fn derive_search_library_subject_routing_lexed(
    search_tokens: &[OwnedLexToken],
    chooser: PlayerAst,
) -> Option<SearchLibrarySubjectRouting> {
    let search_word_storage = subject_routing_word_refs(search_tokens);
    let search_words = search_word_storage
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    let search_body_words = &search_words[1..];
    let mut player = chooser;
    let mut search_player_target: Option<TargetAst> = None;
    let mut forced_library_owner: Option<PlayerFilter> = None;
    let mut search_zones_override: Option<Vec<Zone>> = None;

    if word_slice_starts_with_any(
        search_body_words,
        &[&["your", "library", "for"], &["their", "library", "for"]],
    ) {
        // Keep player from parsed subject/default context.
    } else if word_slice_starts_with_any(
        search_body_words,
        &[
            &[
                "its",
                "controller",
                "graveyard",
                "hand",
                "and",
                "library",
                "for",
            ],
            &[
                "its",
                "controllers",
                "graveyard",
                "hand",
                "and",
                "library",
                "for",
            ],
        ],
    ) {
        player = PlayerAst::ItsController;
        forced_library_owner = Some(PlayerFilter::ControllerOf(crate::filter::ObjectRef::Target));
        search_zones_override = Some(vec![Zone::Graveyard, Zone::Hand, Zone::Library]);
    } else if word_slice_starts_with_any(
        search_body_words,
        &[
            &["its", "owner", "graveyard", "hand", "and", "library", "for"],
            &[
                "its",
                "owners",
                "graveyard",
                "hand",
                "and",
                "library",
                "for",
            ],
        ],
    ) {
        player = PlayerAst::ItsOwner;
        forced_library_owner = Some(PlayerFilter::OwnerOf(crate::filter::ObjectRef::Target));
        search_zones_override = Some(vec![Zone::Graveyard, Zone::Hand, Zone::Library]);
    } else if word_slice_starts_with_any(
        search_body_words,
        &[
            &[
                "target",
                "player",
                "graveyard",
                "hand",
                "and",
                "library",
                "for",
            ],
            &[
                "target",
                "players",
                "graveyard",
                "hand",
                "and",
                "library",
                "for",
            ],
        ],
    ) {
        player = PlayerAst::That;
        search_player_target = Some(TargetAst::Player(
            PlayerFilter::target_player(),
            span_from_tokens(&search_tokens[1..3]),
        ));
        forced_library_owner = Some(PlayerFilter::target_player());
        search_zones_override = Some(vec![Zone::Graveyard, Zone::Hand, Zone::Library]);
    } else if word_slice_starts_with_any(
        search_body_words,
        &[
            &[
                "target",
                "opponent",
                "graveyard",
                "hand",
                "and",
                "library",
                "for",
            ],
            &[
                "target",
                "opponents",
                "graveyard",
                "hand",
                "and",
                "library",
                "for",
            ],
        ],
    ) {
        player = PlayerAst::That;
        search_player_target = Some(TargetAst::Player(
            PlayerFilter::target_opponent(),
            span_from_tokens(&search_tokens[1..3]),
        ));
        forced_library_owner = Some(PlayerFilter::target_opponent());
        search_zones_override = Some(vec![Zone::Graveyard, Zone::Hand, Zone::Library]);
    } else if word_slice_starts_with_any(
        search_body_words,
        &[
            &["target", "player", "library", "for"],
            &["target", "players", "library", "for"],
        ],
    ) {
        player = PlayerAst::That;
        search_player_target = Some(TargetAst::Player(
            PlayerFilter::target_player(),
            span_from_tokens(&search_tokens[1..3]),
        ));
        forced_library_owner = Some(PlayerFilter::target_player());
    } else if word_slice_starts_with_any(
        search_body_words,
        &[
            &["target", "opponent", "library", "for"],
            &["target", "opponents", "library", "for"],
        ],
    ) {
        player = PlayerAst::That;
        search_player_target = Some(TargetAst::Player(
            PlayerFilter::target_opponent(),
            span_from_tokens(&search_tokens[1..3]),
        ));
        forced_library_owner = Some(PlayerFilter::target_opponent());
    } else if word_slice_starts_with_any(
        search_body_words,
        &[
            &["that", "player", "library", "for"],
            &["that", "players", "library", "for"],
        ],
    ) {
        player = PlayerAst::That;
    } else if word_slice_starts_with_any(
        search_body_words,
        &[
            &[
                "that",
                "player",
                "graveyard",
                "hand",
                "and",
                "library",
                "for",
            ],
            &[
                "that",
                "players",
                "graveyard",
                "hand",
                "and",
                "library",
                "for",
            ],
        ],
    ) {
        player = PlayerAst::That;
        search_zones_override = Some(vec![Zone::Graveyard, Zone::Hand, Zone::Library]);
    } else if word_slice_starts_with_any(
        search_body_words,
        &[
            &["its", "controller", "library", "for"],
            &["its", "controllers", "library", "for"],
        ],
    ) {
        player = PlayerAst::ItsController;
    } else if word_slice_starts_with_any(
        search_body_words,
        &[
            &["its", "owner", "library", "for"],
            &["its", "owners", "library", "for"],
        ],
    ) {
        player = PlayerAst::ItsOwner;
    } else if search_body_words.first().copied() == Some("your")
        && let Some(for_pos) = word_slice_find(search_body_words, "for")
        && for_pos > 1
    {
        let zone_words = &search_body_words[1..for_pos];
        let has_library = zone_words
            .iter()
            .any(|word| *word == "library" || *word == "libraries");
        if !has_library {
            return None;
        }

        let has_graveyard = zone_words
            .iter()
            .any(|word| *word == "graveyard" || *word == "graveyards");
        let has_hand = zone_words
            .iter()
            .any(|word| *word == "hand" || *word == "hands");
        let mut zones = Vec::new();
        if has_graveyard {
            zones.push(Zone::Graveyard);
        }
        if has_hand {
            zones.push(Zone::Hand);
        }
        if zones.is_empty() {
            return None;
        }
        zones.push(Zone::Library);
        search_zones_override = Some(zones);
    } else {
        return None;
    }

    Some(SearchLibrarySubjectRouting {
        player,
        search_player_target,
        forced_library_owner,
        search_zones_override,
    })
}

pub(crate) fn parse_search_library_count_prefix_lexed(
    count_tokens: &[OwnedLexToken],
) -> SearchLibraryCountPrefix {
    let mut count = ChoiceCount::up_to(1);
    let mut search_mode = SearchSelectionMode::Exact;
    let mut count_used = 0usize;

    if count_tokens.len() >= 2
        && count_tokens[0].is_word("any")
        && count_tokens[1].is_word("number")
    {
        count = ChoiceCount::any_number();
        search_mode = SearchSelectionMode::Optional;
        count_used = 2;
    } else if count_tokens
        .first()
        .is_some_and(|token| token.is_word("any"))
    {
        if let Some((value, used)) = parse_number(&count_tokens[1..]) {
            count = ChoiceCount::up_to(value as usize);
            search_mode = SearchSelectionMode::Optional;
            count_used = 1 + used;
        }
    } else if count_tokens.len() >= 2
        && count_tokens[0].is_word("that")
        && count_tokens[1].is_word("many")
    {
        count = ChoiceCount::any_number();
        count_used = 2;
    } else if count_tokens
        .first()
        .is_some_and(|token| token.is_word("all"))
    {
        count = ChoiceCount::any_number();
        search_mode = SearchSelectionMode::AllMatching;
        count_used = 1;
    } else if count_tokens.len() >= 2
        && count_tokens[0].is_word("up")
        && count_tokens[1].is_word("to")
    {
        if count_tokens.get(2).is_some_and(|token| token.is_word("x")) {
            count = ChoiceCount::dynamic_x();
            search_mode = SearchSelectionMode::Optional;
            count_used = 3;
        } else if let Some((value, used)) = parse_number(&count_tokens[2..]) {
            count = ChoiceCount::up_to(value as usize);
            search_mode = SearchSelectionMode::Optional;
            count_used = 2 + used;
        }
    } else if count_tokens.first().is_some_and(|token| token.is_word("x")) {
        count = ChoiceCount::dynamic_x();
        count_used = 1;
    } else if let Some((value, used)) = parse_number(count_tokens) {
        count = ChoiceCount::up_to(value as usize);
        count_used = used;
    }

    if count_used < count_tokens.len() && count_tokens[count_used].is_word("of") {
        count_used += 1;
    }

    SearchLibraryCountPrefix {
        count,
        search_mode,
        count_used,
    }
}

pub(crate) fn parse_search_library_same_name_reference_lexed(
    raw_filter_tokens: &[OwnedLexToken],
    mut filter_tokens: Vec<OwnedLexToken>,
    words_all: &[&str],
) -> Result<SearchLibrarySameNameSplit, CardTextError> {
    let mut same_name_reference: Option<SearchLibrarySameNameReference> = None;
    if let Some(base_tokens) =
        strip_search_library_suffix_lexed(raw_filter_tokens, search_library_with_that_name_suffix)
    {
        filter_tokens = base_tokens;
        same_name_reference = Some(SearchLibrarySameNameReference::Tagged(TagKey::from(
            CHOSEN_NAME_TAG,
        )));
    } else if let Some(base_tokens) = strip_search_library_suffix_lexed(
        raw_filter_tokens,
        search_library_with_the_chosen_name_suffix,
    ) {
        filter_tokens = base_tokens;
        same_name_reference = Some(SearchLibrarySameNameReference::Tagged(TagKey::from(
            CHOSEN_NAME_TAG,
        )));
    } else if let Some(base_tokens) =
        strip_search_library_suffix_lexed(raw_filter_tokens, search_library_with_chosen_name_suffix)
    {
        filter_tokens = base_tokens;
        same_name_reference = Some(SearchLibrarySameNameReference::Tagged(TagKey::from(
            CHOSEN_NAME_TAG,
        )));
    } else if let Some((base_filter_tokens, reference_tokens)) =
        split_search_same_name_reference_filter(raw_filter_tokens)
    {
        if base_filter_tokens.is_empty() || reference_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "incomplete same-name search filter in search-library sentence (clause: '{}')",
                words_all.join(" ")
            )));
        }
        filter_tokens = base_filter_tokens;
        let reference_words = token_word_refs(&reference_tokens);
        same_name_reference = if is_same_name_that_reference_words(&reference_words) {
            Some(SearchLibrarySameNameReference::Tagged(TagKey::from(IT_TAG)))
        } else if reference_words.iter().any(|word| *word == "target") {
            let target = parse_target_phrase(&reference_tokens).map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported target same-name reference in search-library sentence (clause: '{}')",
                    words_all.join(" ")
                ))
            })?;
            Some(SearchLibrarySameNameReference::Target(target))
        } else {
            let mut reference_filter_tokens = reference_tokens.clone();
            let mut other_reference = false;
            if reference_filter_tokens
                .first()
                .is_some_and(|token| token.is_word("another") || token.is_word("other"))
            {
                other_reference = true;
                reference_filter_tokens = trim_commas(&reference_filter_tokens[1..]);
            }
            let reference_filter = parse_object_filter(&reference_filter_tokens, other_reference)
                .map_err(|_| {
                    CardTextError::ParseError(format!(
                        "unsupported same-name reference filter in search-library sentence (clause: '{}')",
                        words_all.join(" ")
                    ))
                })?;
            Some(SearchLibrarySameNameReference::Choose {
                filter: reference_filter,
                tag: TagKey::from("same_name_reference"),
            })
        };
    }

    Ok(SearchLibrarySameNameSplit {
        filter_tokens,
        same_name_reference,
    })
}

pub(crate) fn parse_search_library_object_filter_lexed(
    filter_tokens: &[OwnedLexToken],
    words_all: &[&str],
) -> Result<ObjectFilter, CardTextError> {
    let filter_words = parser_text_word_refs(filter_tokens)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect::<Vec<_>>();
    let parser_words = parser_word_token_positions(filter_tokens);

    if let Some(named_idx) = find_parser_word_position(&parser_words, "named") {
        let negated_named = parser_words[..named_idx]
            .iter()
            .rev()
            .find_map(|(_, word)| (!is_article(word)).then_some(*word))
            == Some("not");
        let base_token_end = if negated_named {
            last_non_article_parser_word_token_idx(&parser_words, named_idx).unwrap_or(0)
        } else {
            parser_words[named_idx].0
        };
        let name_words = parser_words
            .iter()
            .skip(named_idx + 1)
            .map(|(_, word)| *word)
            .take_while(|word| !matches!(*word, "that" | "with"))
            .filter(|word| !is_article(word))
            .collect::<Vec<_>>();
        let name = name_words.join(" ");
        if name.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing card name in named search clause (clause: '{}')",
                words_all.join(" ")
            )));
        }
        let base_tokens =
            strip_search_library_leading_count_tokens(&filter_tokens[..base_token_end]);
        let mut base_filter = if is_default_search_library_card_selector(&base_tokens) {
            ObjectFilter::default()
        } else {
            parse_object_filter(&base_tokens, false).map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported named search filter in search-library sentence (clause: '{}')",
                    words_all.join(" ")
                ))
            })?
        };
        if negated_named {
            base_filter.excluded_name = Some(name);
        } else {
            base_filter.name = Some(name);
        }
        Ok(base_filter)
    } else if filter_words.len() == 1 && (filter_words[0] == "card" || filter_words[0] == "cards") {
        Ok(ObjectFilter::default())
    } else if word_slice_contains(&filter_words, "or") {
        parse_search_library_disjunction_filter(filter_tokens)
            .or_else(|| parse_object_filter(filter_tokens, false).ok())
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported search filter in search-library sentence (clause: '{}')",
                    words_all.join(" ")
                ))
            })
    } else {
        parse_object_filter(filter_tokens, false).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported search filter in search-library sentence (clause: '{}')",
                words_all.join(" ")
            ))
        })
    }
}

fn split_search_named_item_filters_lexed(
    filter_tokens: &[OwnedLexToken],
    words_all: &[&str],
) -> Result<Option<Vec<ObjectFilter>>, CardTextError> {
    if !filter_tokens.iter().any(|token| token.is_word("named")) {
        return Ok(None);
    }

    let mut item_starts = Vec::new();
    let mut cursor = 0usize;
    while cursor < filter_tokens.len() {
        while filter_tokens
            .get(cursor)
            .is_some_and(OwnedLexToken::is_comma)
        {
            cursor += 1;
        }
        if filter_tokens
            .get(cursor)
            .is_some_and(|token| token.is_word("and"))
        {
            cursor += 1;
            while filter_tokens
                .get(cursor)
                .is_some_and(OwnedLexToken::is_comma)
            {
                cursor += 1;
            }
        }
        if cursor >= filter_tokens.len() {
            break;
        }

        let item_start = cursor;
        if filter_tokens
            .get(cursor)
            .is_some_and(|token| token.is_word("a") || token.is_word("an"))
        {
            cursor += 1;
        }
        if !filter_tokens
            .get(cursor)
            .is_some_and(|token| token.is_word("card") || token.is_word("cards"))
            || !filter_tokens
                .get(cursor + 1)
                .is_some_and(|token| token.is_word("named"))
        {
            return Ok(None);
        }
        item_starts.push(item_start);
        cursor += 2;

        while cursor < filter_tokens.len() {
            let mut probe = cursor;
            while filter_tokens
                .get(probe)
                .is_some_and(OwnedLexToken::is_comma)
            {
                probe += 1;
            }
            if filter_tokens
                .get(probe)
                .is_some_and(|token| token.is_word("and"))
            {
                probe += 1;
                while filter_tokens
                    .get(probe)
                    .is_some_and(OwnedLexToken::is_comma)
                {
                    probe += 1;
                }
            }
            let mut phrase_probe = probe;
            if filter_tokens
                .get(phrase_probe)
                .is_some_and(|token| token.is_word("a") || token.is_word("an"))
            {
                phrase_probe += 1;
            }
            if filter_tokens
                .get(phrase_probe)
                .is_some_and(|token| token.is_word("card") || token.is_word("cards"))
                && filter_tokens
                    .get(phrase_probe + 1)
                    .is_some_and(|token| token.is_word("named"))
            {
                break;
            }
            cursor += 1;
        }
    }
    if item_starts.len() <= 1 {
        return Ok(None);
    }

    let mut filters = Vec::new();
    for (pos, start) in item_starts.iter().enumerate() {
        let end = item_starts
            .get(pos + 1)
            .copied()
            .unwrap_or(filter_tokens.len());
        let item_tokens = trim_commas(&filter_tokens[*start..end]);
        let item_filter = parse_search_library_object_filter_lexed(&item_tokens, words_all)?;
        if item_filter.name.is_none() {
            return Ok(None);
        }
        filters.push(item_filter);
    }
    Ok(Some(filters))
}

pub(crate) fn parse_search_library_leading_effect_prelude_lexed<'a>(
    subject_tokens: &'a [OwnedLexToken],
) -> Result<SearchLibraryLeadingPrelude<'a>, CardTextError> {
    if subject_tokens.is_empty() || find_verb_lexed(subject_tokens).is_none() {
        return Ok(SearchLibraryLeadingPrelude {
            subject_tokens,
            leading_effects: Vec::new(),
        });
    }

    let mut leading_tokens = trim_commas(subject_tokens);
    while leading_tokens
        .last()
        .is_some_and(|token| token.is_word("then") || token.is_word("and"))
    {
        leading_tokens.pop();
    }
    let leading_effects = if leading_tokens.is_empty() {
        Vec::new()
    } else {
        parse_effect_chain_with_sentence_primitives_lexed(&leading_tokens)?
    };

    Ok(SearchLibraryLeadingPrelude {
        subject_tokens: &[],
        leading_effects,
    })
}

pub(crate) fn search_library_has_unsupported_top_position_probe(words: &[&str]) -> bool {
    word_slice_mentions_nth_from_top(words)
        && !word_slice_contains_sequence(words, &["on", "top", "of", "library"])
}

pub(crate) fn search_library_subject_wraps_each_target_player_lexed(
    subject_tokens: &[OwnedLexToken],
) -> bool {
    matches!(
        token_word_refs(subject_tokens).as_slice(),
        ["each", "of", "them"]
    )
}

pub(crate) fn search_library_starts_with_search_verb_lexed(
    search_tokens: &[OwnedLexToken],
) -> bool {
    primitives::parse_prefix(search_tokens, search_library_search_verb).is_some()
}

pub(crate) fn cant_sentence_clause_tokens_for_restriction_scan_lexed(
    clause_tokens: &[OwnedLexToken],
) -> Vec<OwnedLexToken> {
    split_lexed_sentences(clause_tokens)
        .into_iter()
        .next()
        .unwrap_or(clause_tokens)
        .to_vec()
}

pub(crate) fn cant_sentence_has_supported_negation_gate_lexed(
    clause_tokens: &[OwnedLexToken],
) -> bool {
    let Some((neg_start, _)) = find_cant_sentence_negation_span_lexed(clause_tokens) else {
        return false;
    };

    !clause_tokens[..neg_start]
        .iter()
        .any(|token| token.is_word("and"))
}

pub(crate) fn find_cant_sentence_negation_span_lexed(
    tokens: &[OwnedLexToken],
) -> Option<(usize, usize)> {
    let mut cursor = 0usize;

    while cursor < tokens.len() {
        let token = &tokens[cursor];
        if token.is_word("can't") || token.is_word("cant") || token.is_word("cannot") {
            return Some((cursor, cursor + 1));
        }
        if token.is_word("doesn't")
            || token.is_word("doesnt")
            || token.is_word("don't")
            || token.is_word("dont")
        {
            if matches!(
                tokens.get(cursor + 1).map(|next| next.parser_text.as_str()),
                Some("control" | "controls" | "own" | "owns")
            ) {
                cursor += 1;
                continue;
            }
            return Some((cursor, cursor + 1));
        }
        if (token.is_word("does") || token.is_word("do") || token.is_word("can"))
            && tokens
                .get(cursor + 1)
                .is_some_and(|next| next.is_word("not"))
        {
            if (token.is_word("does") || token.is_word("do"))
                && matches!(
                    tokens.get(cursor + 2).map(|next| next.parser_text.as_str()),
                    Some("control" | "controls" | "own" | "owns")
                )
            {
                cursor += 1;
                continue;
            }
            return Some((cursor, cursor + 2));
        }
        cursor += 1;
    }

    None
}

fn cant_sentence_next_turn_suffix<'a>(
    input: &mut LexStream<'a>,
) -> Result<(), ErrMode<ContextError>> {
    alt((
        primitives::phrase(&["during", "that", "players", "next", "turn"]),
        primitives::phrase(&["during", "that", "player's", "next", "turn"]),
        primitives::phrase(&["during", "that", "player", "s", "next", "turn"]),
    ))
    .void()
    .parse_next(input)
}

fn cant_sentence_for_as_long_as_marker<'a>(
    input: &mut LexStream<'a>,
) -> Result<(), ErrMode<ContextError>> {
    primitives::phrase(&["for", "as", "long", "as"])
        .void()
        .parse_next(input)
}

pub(crate) fn split_cant_sentence_next_turn_prefix_lexed(
    tokens: &[OwnedLexToken],
) -> Option<Vec<OwnedLexToken>> {
    let mut cursor = 0usize;

    while cursor < tokens.len() {
        let Some((_, rest)) =
            primitives::parse_prefix(&tokens[cursor..], cant_sentence_next_turn_suffix)
        else {
            cursor += 1;
            continue;
        };
        if rest.iter().all(|token| token.is_period()) {
            return Some(tokens[..cursor].to_vec());
        }
        cursor += 1;
    }

    None
}

#[derive(Debug, Clone)]
pub(crate) struct CantSentencePreparedClause {
    pub(crate) duration: crate::effect::Until,
    pub(crate) clause_tokens: Vec<OwnedLexToken>,
}

pub(crate) fn prepare_cant_sentence_restriction_clause_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<CantSentencePreparedClause>, CardTextError> {
    let Some((duration, clause_tokens)) = parse_restriction_duration_lexed(tokens)? else {
        return Ok(None);
    };
    if clause_tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "restriction clause missing body".to_string(),
        ));
    }
    if clause_tokens
        .first()
        .is_some_and(|token| token.is_word("if"))
    {
        return Ok(None);
    }

    let clause_tokens = cant_sentence_clause_tokens_for_restriction_scan_lexed(&clause_tokens);
    if !cant_sentence_has_supported_negation_gate_lexed(&clause_tokens) {
        return Ok(None);
    }

    Ok(Some(CantSentencePreparedClause {
        duration,
        clause_tokens,
    }))
}

fn conditional_label_delimiter<'a>(input: &mut LexStream<'a>) -> Result<(), ErrMode<ContextError>> {
    alt((
        primitives::token_kind(TokenKind::Dash).void(),
        primitives::token_kind(TokenKind::EmDash).void(),
    ))
    .parse_next(input)
}

fn conditional_sentence_family_head<'a>(
    input: &mut LexStream<'a>,
) -> Result<(), ErrMode<ContextError>> {
    alt((
        primitives::phrase(&["then", "if"]),
        (
            conditional_label_phrase,
            opt(conditional_label_delimiter),
            primitives::kw("if"),
        )
            .void(),
        primitives::kw("if").void(),
    ))
    .parse_next(input)
}

pub(crate) fn split_conditional_sentence_family_head_lexed(
    tokens: &[OwnedLexToken],
) -> Option<&[OwnedLexToken]> {
    let (_, rest) = primitives::parse_prefix(tokens, conditional_sentence_family_head)?;
    let consumed = tokens.len().checked_sub(rest.len())?;
    consumed.checked_sub(1).map(|if_idx| &tokens[if_idx..])
}

pub(crate) fn parse_conditional_sentence_with_grammar_entrypoint_lexed(
    tokens: &[OwnedLexToken],
    parse_effect_chain_lexed: fn(&[OwnedLexToken]) -> Result<Vec<EffectAst>, CardTextError>,
) -> Result<Vec<EffectAst>, CardTextError> {
    let split = split_if_clause_lexed(tokens, parse_effect_chain_lexed)?;

    Ok(vec![match split.predicate {
        IfClausePredicateSpec::Conditional(predicate) => EffectAst::Conditional {
            predicate,
            if_true: split.effects,
            if_false: Vec::new(),
        },
        IfClausePredicateSpec::Result(predicate) => EffectAst::IfResult {
            predicate,
            effects: split.effects,
        },
    }])
}

#[cfg(test)]
pub(crate) fn parse_conditional_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    parse_conditional_sentence_with_grammar_entrypoint_lexed(tokens, parse_effect_chain_lexed)
}

pub(crate) fn parse_conditional_sentence_family_lexed(
    tokens: &[OwnedLexToken],
    parse_effect_chain_lexed: fn(&[OwnedLexToken]) -> Result<Vec<EffectAst>, CardTextError>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(conditional_tokens) = split_conditional_sentence_family_head_lexed(tokens) else {
        return Ok(None);
    };

    parse_conditional_sentence_with_grammar_entrypoint_lexed(
        conditional_tokens,
        parse_effect_chain_lexed,
    )
    .map(Some)
}

pub(crate) fn parse_cant_effect_sentence_with_grammar_entrypoint_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if let Some(prefix_tokens) = split_cant_sentence_next_turn_prefix_lexed(tokens) {
        let prefix_tokens = prefix_tokens.as_slice();
        if let Some(parsed) = parse_cant_restriction_clause(prefix_tokens)? {
            let next_turn_effects = match parsed.restriction {
                crate::effect::Restriction::CastSpellsMatching(player, spell_filter) => {
                    let nested = crate::effect::Restriction::cast_spells_matching(
                        PlayerFilter::Active,
                        spell_filter,
                    );
                    match player {
                        PlayerFilter::Opponent => Some(vec![EffectAst::ForEachOpponent {
                            effects: vec![EffectAst::DelayedUntilNextUpkeep {
                                player: crate::cards::builders::PlayerAst::That,
                                effects: vec![EffectAst::Cant {
                                    restriction: nested,
                                    duration: crate::effect::Until::EndOfTurn,
                                    condition: None,
                                }],
                            }],
                        }]),
                        PlayerFilter::IteratedPlayer => {
                            Some(vec![EffectAst::DelayedUntilNextUpkeep {
                                player: crate::cards::builders::PlayerAst::That,
                                effects: vec![EffectAst::Cant {
                                    restriction: nested,
                                    duration: crate::effect::Until::EndOfTurn,
                                    condition: None,
                                }],
                            }])
                        }
                        _ => None,
                    }
                }
                crate::effect::Restriction::CastMoreThanOneSpellEachTurn(player, spell_filter) => {
                    let nested = crate::effect::Restriction::CastMoreThanOneSpellEachTurn(
                        PlayerFilter::Active,
                        spell_filter,
                    );
                    match player {
                        PlayerFilter::Opponent => Some(vec![EffectAst::ForEachOpponent {
                            effects: vec![EffectAst::DelayedUntilNextUpkeep {
                                player: crate::cards::builders::PlayerAst::That,
                                effects: vec![EffectAst::Cant {
                                    restriction: nested,
                                    duration: crate::effect::Until::EndOfTurn,
                                    condition: None,
                                }],
                            }],
                        }]),
                        PlayerFilter::IteratedPlayer => {
                            Some(vec![EffectAst::DelayedUntilNextUpkeep {
                                player: crate::cards::builders::PlayerAst::That,
                                effects: vec![EffectAst::Cant {
                                    restriction: nested,
                                    duration: crate::effect::Until::EndOfTurn,
                                    condition: None,
                                }],
                            }])
                        }
                        _ => None,
                    }
                }
                _ => None,
            };

            if let Some(next_turn_effects) = next_turn_effects {
                return Ok(Some(next_turn_effects));
            }
        }
    }

    let source_tapped_duration = cant_sentence_has_source_remains_tapped_duration(tokens);
    let Some(prepared_clause) = prepare_cant_sentence_restriction_clause_lexed(tokens)? else {
        return Ok(None);
    };
    let duration = prepared_clause.duration;
    let clause_tokens = prepared_clause.clause_tokens;

    let Some(restrictions) = parse_cant_restrictions(&clause_tokens)? else {
        return Err(CardTextError::ParseError(format!(
            "unsupported restriction clause body (clause: '{}')",
            token_word_refs(&clause_tokens).join(" ")
        )));
    };

    let mut target: Option<crate::cards::builders::TargetAst> = None;
    let mut effects = Vec::new();
    for parsed in restrictions {
        if let Some(parsed_target) = parsed.target {
            if let Some(existing) = &target {
                if *existing != parsed_target {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported mixed restriction targets (clause: '{}')",
                        token_word_refs(&clause_tokens).join(" ")
                    )));
                }
            } else {
                target = Some(parsed_target);
            }
        }
        effects.push(EffectAst::Cant {
            restriction: parsed.restriction,
            duration: duration.clone(),
            condition: source_tapped_duration.then_some(crate::ConditionExpr::SourceIsTapped),
        });
    }
    if let Some(target) = target {
        effects.insert(0, EffectAst::TargetOnly { target });
    }

    Ok(Some(effects))
}

pub(crate) fn parse_cant_effect_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_cant_effect_sentence_with_grammar_entrypoint_lexed(tokens)
}

pub(crate) fn parse_search_library_sentence_with_grammar_entrypoint_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    fn has_trailing_that_player_shuffle(tokens: &[OwnedLexToken]) -> bool {
        super::primitives::words_find_phrase(tokens, &["then", "that", "player", "shuffle"])
            .is_some()
            || super::primitives::words_find_phrase(tokens, &["then", "that", "player", "shuffles"])
                .is_some()
            || super::primitives::words_find_phrase(tokens, &["that", "player", "shuffle"])
                .is_some()
            || super::primitives::words_find_phrase(tokens, &["that", "player", "shuffles"])
                .is_some()
    }

    let words_all = parser_text_word_refs(tokens);
    let Some(head_split) = split_search_library_sentence_head_lexed(tokens) else {
        return Ok(None);
    };

    let subject_prelude =
        parse_search_library_leading_effect_prelude_lexed(head_split.subject_tokens)?;
    let subject_tokens = subject_prelude.subject_tokens;
    let sentence_has_direct_may = head_split.sentence_has_direct_may;
    let mut leading_effects = subject_prelude.leading_effects;
    let wrap_each_target_player =
        search_library_subject_wraps_each_target_player_lexed(subject_tokens);
    let chooser = match parse_subject(subject_tokens) {
        SubjectAst::Player(player) => player,
        _ => PlayerAst::Implicit,
    };

    let search_tokens = head_split.search_tokens;
    if !search_library_starts_with_search_verb_lexed(search_tokens) {
        return Ok(None);
    }
    let search_words = parser_text_word_refs(search_tokens);
    if search_words.is_empty() {
        return Ok(None);
    }
    let Some(subject_routing) = derive_search_library_subject_routing_lexed(search_tokens, chooser)
    else {
        return Ok(None);
    };
    let player = subject_routing.player;
    let search_player_target = subject_routing.search_player_target;
    let forced_library_owner = subject_routing.forced_library_owner;
    let search_zones_override = subject_routing.search_zones_override;
    if search_library_has_unsupported_top_position_probe(&search_words) {
        return Err(CardTextError::ParseError(format!(
            "unsupported search-library top-position clause (clause: '{}')",
            words_all.join(" ")
        )));
    }

    let clause_markers = scan_search_library_clause_markers_lexed(search_tokens)
        .expect("grammar-owned search-library clause marker scan should produce defaults");
    let for_idx = clause_markers.for_idx;
    let put_idx = clause_markers.put_idx;
    let has_explicit_destination = clause_markers.has_explicit_destination;
    let filter_boundary = clause_markers.filter_boundary;

    let filter_end =
        find_search_library_filter_boundary_lexed(search_tokens, for_idx, filter_boundary)
            .filter_end;

    if filter_end <= for_idx + 1 {
        return Err(CardTextError::ParseError(format!(
            "missing search filter in search-library sentence (clause: '{}')",
            words_all.join(" ")
        )));
    }

    let count_tokens = &search_tokens[for_idx + 1..filter_end];
    let count_prefix = parse_search_library_count_prefix_lexed(count_tokens);
    let count = count_prefix.count;
    let search_mode = count_prefix.search_mode;
    let count_used = count_prefix.count_used;

    let filter_start = for_idx + 1 + count_used;
    if filter_start >= filter_end {
        return Err(CardTextError::ParseError(format!(
            "missing object selector in search-library sentence (clause: '{}')",
            words_all.join(" ")
        )));
    }

    let raw_filter_tokens = trim_commas(&search_tokens[filter_start..filter_end]);
    let (filter_tokens, mana_constraint) = if let Some((base_filter_tokens, mana_constraint)) =
        extract_search_library_mana_constraint(&raw_filter_tokens)
    {
        (base_filter_tokens, Some(mana_constraint))
    } else {
        (raw_filter_tokens.clone(), None)
    };
    let same_name_split = parse_search_library_same_name_reference_lexed(
        &raw_filter_tokens,
        filter_tokens,
        &words_all,
    )?;
    let filter_tokens = same_name_split.filter_tokens;
    let same_name_reference = same_name_split.same_name_reference;

    let named_filters = if count_used == 0 {
        split_search_named_item_filters_lexed(&filter_tokens, &words_all)?
    } else {
        None
    };
    let mut filter = parse_search_library_object_filter_lexed(&filter_tokens, &words_all)?;
    if let Some(same_name_tag) = same_name_reference
        .as_ref()
        .map(|reference| match reference {
            SearchLibrarySameNameReference::Tagged(tag) => tag.clone(),
            SearchLibrarySameNameReference::Target(_) => TagKey::from(IT_TAG),
            SearchLibrarySameNameReference::Choose { tag, .. } => tag.clone(),
        })
    {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: same_name_tag.clone(),
            relation: TaggedOpbjectRelation::SameNameAsTagged,
        });
    }
    if filter.owner.is_none()
        && let Some(owner) = forced_library_owner.clone()
    {
        filter.owner = Some(owner);
    }
    normalize_search_library_filter(&mut filter);
    if let Some(mana_constraint) = mana_constraint {
        apply_search_library_mana_constraint(&mut filter, mana_constraint);
    }

    let discard_before_shuffle_followup =
        find_search_library_discard_before_shuffle_followup_lexed(search_tokens, put_idx);
    let trailing_discard_before_shuffle = discard_before_shuffle_followup.is_some();
    let effect_routing = derive_search_library_effect_routing_lexed(
        tokens,
        search_tokens,
        clause_markers,
        trailing_discard_before_shuffle,
    );
    let destination = effect_routing.destination;
    let reveal = effect_routing.reveal;
    let face_down_exile = effect_routing.face_down_exile;
    let shuffle = effect_routing.shuffle;
    let split_battlefield_and_hand = effect_routing.split_battlefield_and_hand;
    let mut effects = if let Some(named_filters) = named_filters {
        let searched_tag: TagKey = "searched_named".into();
        let zones = search_zones_override.unwrap_or_else(|| vec![Zone::Library]);
        let mut sequence = Vec::new();
        for mut named_filter in named_filters {
            if named_filter.owner.is_none()
                && let Some(owner) = forced_library_owner.clone()
            {
                named_filter.owner = Some(owner);
            }
            normalize_search_library_filter(&mut named_filter);
            sequence.push(EffectAst::ChooseObjectsAcrossZones {
                filter: named_filter,
                count: ChoiceCount::exactly(1),
                player: chooser,
                tag: searched_tag.clone(),
                zones: zones.clone(),
                search_mode: Some(SearchSelectionMode::Exact),
            });
        }
        if reveal {
            sequence.push(EffectAst::RevealTagged {
                tag: searched_tag.clone(),
            });
        }
        sequence.push(EffectAst::MoveToZone {
            target: TargetAst::Tagged(searched_tag, span_from_tokens(tokens)),
            zone: destination,
            to_top: matches!(destination, Zone::Library),
            battlefield_controller: ReturnControllerAst::Preserve,
            battlefield_tapped: destination == Zone::Battlefield
                && effect_routing.has_tapped_modifier,
            attached_to: None,
        });
        if shuffle && zones.contains(&Zone::Library) {
            sequence.push(EffectAst::ShuffleLibrary { player });
        }
        sequence
    } else if !has_explicit_destination {
        let chosen_tag: TagKey = "searched".into();
        let mut sequence = vec![EffectAst::ChooseObjectsAcrossZones {
            filter,
            count,
            player: chooser,
            tag: chosen_tag.clone(),
            zones: search_zones_override.unwrap_or_else(|| vec![Zone::Library]),
            search_mode: Some(search_mode),
        }];
        if reveal {
            sequence.push(EffectAst::RevealTagged {
                tag: chosen_tag.clone(),
            });
        }
        if shuffle {
            sequence.push(EffectAst::ShuffleLibrary { player });
        }
        sequence
    } else if let Some(search_zones) = search_zones_override.clone() {
        let chosen_tag: TagKey = "searched_multi_zone".into();
        let battlefield_tapped =
            destination == Zone::Battlefield && effect_routing.has_tapped_modifier;
        let shuffle_player = PlayerAst::That;
        let mut sequence = vec![EffectAst::ChooseObjectsAcrossZones {
            filter,
            count,
            player: chooser,
            tag: chosen_tag.clone(),
            zones: search_zones.clone(),
            search_mode: Some(search_mode),
        }];
        if reveal {
            sequence.push(EffectAst::RevealTagged {
                tag: chosen_tag.clone(),
            });
        }
        if shuffle
            && destination == Zone::Library
            && zone_slice_contains(&search_zones, Zone::Library)
        {
            sequence.push(EffectAst::ShuffleLibrary {
                player: shuffle_player,
            });
        }
        sequence.push(EffectAst::ForEachTagged {
            tag: chosen_tag.clone(),
            effects: vec![EffectAst::MoveToZone {
                target: TargetAst::Tagged(chosen_tag, span_from_tokens(tokens)),
                zone: destination,
                to_top: matches!(destination, Zone::Library),
                battlefield_controller: ReturnControllerAst::Preserve,
                battlefield_tapped,
                attached_to: None,
            }],
        });
        if shuffle
            && !(destination == Zone::Library && zone_slice_contains(&search_zones, Zone::Library))
        {
            sequence.push(EffectAst::ShuffleLibrary {
                player: shuffle_player,
            });
        }
        sequence
    } else if split_battlefield_and_hand {
        let battlefield_tapped = effect_routing.has_tapped_modifier;
        vec![
            EffectAst::SearchLibrary {
                filter: filter.clone(),
                destination: Zone::Battlefield,
                chooser,
                player,
                search_mode,
                reveal,
                shuffle: false,
                count: ChoiceCount::up_to(1),
                count_value: None,
                tapped: battlefield_tapped,
            },
            EffectAst::SearchLibrary {
                filter,
                destination: Zone::Hand,
                chooser,
                player,
                search_mode,
                reveal,
                shuffle,
                count: ChoiceCount::up_to(1),
                count_value: None,
                tapped: false,
            },
        ]
    } else if destination == Zone::Exile && face_down_exile {
        let searched_tag: TagKey = "searched_face_down".into();
        let mut sequence = vec![
            EffectAst::ChooseObjectsAcrossZones {
                filter,
                count,
                player: chooser,
                tag: searched_tag.clone(),
                zones: vec![Zone::Library],
                search_mode: Some(search_mode),
            },
            EffectAst::Exile {
                target: TargetAst::Tagged(searched_tag, span_from_tokens(tokens)),
                face_down: true,
            },
        ];
        if shuffle {
            sequence.push(EffectAst::ShuffleLibrary { player });
        }
        sequence
    } else {
        let battlefield_tapped =
            destination == Zone::Battlefield && effect_routing.has_tapped_modifier;
        vec![EffectAst::SearchLibrary {
            filter,
            destination,
            chooser,
            player,
            search_mode,
            reveal,
            shuffle,
            count,
            count_value: None,
            tapped: battlefield_tapped,
        }]
    };

    if let Some(discard_followup) = discard_before_shuffle_followup {
        let discard_tokens =
            trim_commas(&search_tokens[discard_followup.discard_idx..discard_followup.discard_end]);
        if !discard_tokens.is_empty() {
            effects.push(parse_effect_clause_lexed(&discard_tokens)?);
        }
        effects.push(EffectAst::ShuffleLibrary { player });
    }

    if has_trailing_that_player_shuffle(tokens) {
        let mut rewrote_existing_shuffle = false;
        for effect in &mut effects {
            if let EffectAst::ShuffleLibrary { player } = effect
                && matches!(*player, PlayerAst::You | PlayerAst::Implicit)
            {
                *player = PlayerAst::That;
                rewrote_existing_shuffle = true;
            }
        }
        if !rewrote_existing_shuffle {
            effects.push(EffectAst::ShuffleLibrary {
                player: PlayerAst::That,
            });
        }
    }

    if let Some(target) = search_player_target {
        effects.insert(0, EffectAst::TargetOnly { target });
    }

    if let Some(trailing_tokens) = find_search_library_trailing_life_followup_lexed(
        search_tokens,
        put_idx.unwrap_or(filter_boundary),
    ) {
        let trailing_effect = parse_effect_clause_lexed(trailing_tokens)?;
        effects.push(trailing_effect);
    }

    if let Some(reference) = same_name_reference {
        match reference {
            SearchLibrarySameNameReference::Tagged(_) => {}
            SearchLibrarySameNameReference::Target(target) => {
                effects.insert(0, EffectAst::TargetOnly { target });
            }
            SearchLibrarySameNameReference::Choose { filter, tag } => {
                effects.insert(
                    0,
                    EffectAst::ChooseObjects {
                        filter,
                        count: ChoiceCount::exactly(1),
                        count_value: None,
                        player,
                        tag,
                    },
                );
            }
        }
    }

    if sentence_has_direct_may {
        effects = vec![if matches!(chooser, PlayerAst::You | PlayerAst::Implicit) {
            EffectAst::May { effects }
        } else {
            EffectAst::MayByPlayer {
                player: chooser,
                effects,
            }
        }];
    }

    if !leading_effects.is_empty() {
        leading_effects.extend(effects);
        return Ok(Some(leading_effects));
    }

    if wrap_each_target_player {
        effects = vec![EffectAst::ForEachPlayersFiltered {
            filter: PlayerFilter::target_player(),
            effects,
        }];
    }

    Ok(Some(effects))
}

pub(crate) fn parse_search_library_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_search_library_sentence_with_grammar_entrypoint_lexed(tokens)
}

pub(crate) fn cant_sentence_has_source_remains_tapped_duration(tokens: &[OwnedLexToken]) -> bool {
    let mut has_for_as_long_as = false;
    let mut has_remains = false;
    let mut has_tapped = false;
    let mut has_source_word = false;
    let mut cursor = 0usize;

    while cursor < tokens.len() {
        if !has_for_as_long_as
            && primitives::parse_prefix(&tokens[cursor..], cant_sentence_for_as_long_as_marker)
                .is_some()
        {
            has_for_as_long_as = true;
        }

        let token = &tokens[cursor];
        has_remains |= token.is_word("remains");
        has_tapped |= token.is_word("tapped");
        has_source_word |= matches!(
            token.parser_text.as_str(),
            "this" | "source" | "artifact" | "creature" | "permanent"
        );
        cursor += 1;
    }

    has_for_as_long_as && has_remains && has_tapped && has_source_word
}
