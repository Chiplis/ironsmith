use super::*;
use crate::runtime_backend::lexer::{parser_token_word_positions, parser_token_word_refs};
use ironsmith_core::Value;
use winnow::error::{ContextError, ErrMode, ModalResult};

#[path = "search_library/duration_shapes.rs"]
mod duration_shapes;
#[path = "search_library/exile_shapes.rs"]
mod exile_shapes;
#[path = "search_library/shuffle_shapes.rs"]
mod shuffle_shapes;

pub(crate) use duration_shapes::*;
pub(crate) use exile_shapes::*;
pub(crate) use shuffle_shapes::*;

#[path = "search_library/same_name_references.rs"]
mod same_name_references;
pub(crate) use same_name_references::*;

const CHOSEN_NAME_TAG: &str = "__chosen_name__";
const EACH_OF_THEM_SUBJECT: &[&str] = &["each", "of", "them"];
const DIFFERENT_NAMES_CLAUSES: &[&[&str]] = &[
    &["with", "different", "names"],
    &["that", "have", "different", "names"],
];
const LIFE_FOLLOWUP_PREFIXES: &[&[&str]] = &[
    &["you", "gain"],
    &["target", "player", "gains"],
    &["target", "player", "gain"],
];
const FACE_DOWN_PHRASE: &[&str] = &["face", "down"];
const BATTLEFIELD_HAND_OTHER_ONE_MARKER_WORDS: &[&str] = &["battlefield", "hand", "other", "one"];
const YOUR_OR_THEIR_LIBRARY_FOR_PREFIX_PATTERN: &[&[&str]] =
    &[&["your", "library", "for"], &["their", "library", "for"]];
const YOUR_OR_THEIR_LIBRARY_GRAVEYARD_FOR_PREFIX_PATTERN: &[&[&str]] = &[
    &["your", "library", "and/or", "graveyard", "for"],
    &["their", "library", "and/or", "graveyard", "for"],
    &["your", "library", "and", "graveyard", "for"],
    &["their", "library", "and", "graveyard", "for"],
    &["your", "library", "and", "or", "graveyard", "for"],
    &["their", "library", "and", "or", "graveyard", "for"],
    &["your", "graveyard", "and/or", "library", "for"],
    &["their", "graveyard", "and/or", "library", "for"],
    &["your", "graveyard", "and", "library", "for"],
    &["their", "graveyard", "and", "library", "for"],
    &["your", "graveyard", "and", "or", "library", "for"],
    &["their", "graveyard", "and", "or", "library", "for"],
];
const CONTROLLER_GRAVEYARD_HAND_LIBRARY_FOR_PREFIX_PATTERN: &[&[&str]] = &[
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
];
const CONTROLLER_SUFFIX_GRAVEYARD_HAND_LIBRARY_FOR_PREFIX_PATTERN: &[&[&str]] = &[
    &[
        "the",
        "graveyard",
        "hand",
        "and",
        "library",
        "of",
        "that",
        "spells",
        "controller",
        "for",
    ],
    &[
        "graveyard",
        "hand",
        "and",
        "library",
        "of",
        "that",
        "spells",
        "controller",
        "for",
    ],
    &[
        "the",
        "graveyard",
        "hand",
        "and",
        "library",
        "of",
        "that",
        "objects",
        "controller",
        "for",
    ],
    &[
        "graveyard",
        "hand",
        "and",
        "library",
        "of",
        "that",
        "objects",
        "controller",
        "for",
    ],
];
const OWNER_GRAVEYARD_HAND_LIBRARY_FOR_PREFIX_PATTERN: &[&[&str]] = &[
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
];
const TARGET_PLAYER_GRAVEYARD_HAND_LIBRARY_FOR_PREFIX_PATTERN: &[&[&str]] = &[
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
];
const TARGET_OPPONENT_GRAVEYARD_HAND_LIBRARY_FOR_PREFIX_PATTERN: &[&[&str]] = &[
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
];
const TARGET_PLAYER_LIBRARY_FOR_PREFIX_PATTERN: &[&[&str]] = &[
    &["target", "player", "library", "for"],
    &["target", "players", "library", "for"],
];
const TARGET_OPPONENT_LIBRARY_FOR_PREFIX_PATTERN: &[&[&str]] = &[
    &["target", "opponent", "library", "for"],
    &["target", "opponents", "library", "for"],
];
const THAT_PLAYER_LIBRARY_FOR_PREFIX_PATTERN: &[&[&str]] = &[
    &["that", "player", "library", "for"],
    &["that", "players", "library", "for"],
];
const THAT_PLAYER_GRAVEYARD_HAND_LIBRARY_FOR_PREFIX_PATTERN: &[&[&str]] = &[
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
];
const CONTROLLER_LIBRARY_FOR_PREFIX_PATTERN: &[&[&str]] = &[
    &["its", "controller", "library", "for"],
    &["its", "controllers", "library", "for"],
];
const OWNER_LIBRARY_FOR_PREFIX_PATTERN: &[&[&str]] = &[
    &["its", "owner", "library", "for"],
    &["its", "owners", "library", "for"],
];
const ON_TOP_OF_LIBRARY_PHRASE: &[&str] = &["on", "top", "of", "library"];
const FROM_THE_TOP_PREFIX: &[&str] = &["from", "the", "top"];
const BASIC_LAND_TYPE_SEARCH_SELECTOR: &[&str] =
    &["land", "card", "of", "each", "basic", "land", "type"];
const ANY_NUMBER_PREFIX: &[&str] = &["any", "number"];
const UP_TO_PREFIX: &[&str] = &["up", "to"];
const UP_TO_X_PREFIX: &[&str] = &["up", "to", "x"];

fn search_library_token_is_any_word(token: &OwnedLexToken, words: &[&str]) -> bool {
    token.as_word().is_some_and(|_| {
        let text = token.parser_text();
        search_library_words_have_word(words, text)
    })
}

fn search_library_words_have_word(words: &[&str], expected: &str) -> bool {
    let mut idx = 0usize;
    while idx < words.len() {
        if words[idx] == expected {
            return true;
        }
        idx += 1;
    }
    false
}

fn search_library_words_equal_any(words: &[&str], phrases: &[&[&str]]) -> bool {
    let mut idx = 0usize;
    while idx < phrases.len() {
        if search_word_stream_eq_phrase(words, phrases[idx]) {
            return true;
        }
        idx += 1;
    }
    false
}

fn dynamic_search_word_phrase<'phrase, 'input>(
    phrase: &'phrase [&'phrase str],
) -> impl Parser<&'input [&'input str], (), ErrMode<ContextError>> + 'phrase {
    move |input: &mut &'input [&'input str]| {
        if phrase.is_empty() || input.len() < phrase.len() {
            return Err(primitives::backtrack_err(
                "search word phrase",
                "non-empty matching phrase",
            ));
        }
        let (candidate, rest) = input.split_at(phrase.len());
        if candidate
            .iter()
            .copied()
            .zip(phrase.iter().copied())
            .all(|(actual, expected)| actual == expected)
        {
            *input = rest;
            Ok(())
        } else {
            Err(primitives::backtrack_err(
                "search word phrase",
                "matching phrase",
            ))
        }
    }
}

fn search_word_stream_starts_with_any(words: &[&str], phrases: &[&[&str]]) -> bool {
    for phrase in phrases {
        let mut input = words;
        if dynamic_search_word_phrase(phrase)
            .parse_next(&mut input)
            .is_ok()
        {
            return true;
        }
    }
    false
}

fn search_word_stream_matches_at_some_offset(words: &[&str], phrase: &[&str]) -> bool {
    for start in 0..=words.len() {
        let mut input = &words[start..];
        if dynamic_search_word_phrase(phrase)
            .parse_next(&mut input)
            .is_ok()
        {
            return true;
        }
    }
    false
}

fn search_word_stream_eq_phrase(words: &[&str], phrase: &[&str]) -> bool {
    if words.len() != phrase.len() {
        return false;
    }
    let mut input = words;
    dynamic_search_word_phrase(phrase)
        .parse_next(&mut input)
        .is_ok()
        && input.is_empty()
}

fn search_library_words_contain_all(words: &[&str], required: &[&str]) -> bool {
    required
        .iter()
        .all(|required_word| words.iter().any(|word| word == required_word))
}

fn search_library_words_are_default_card_selector(words: &[&str]) -> bool {
    words.is_empty()
        || search_word_stream_eq_phrase(words, &["card"])
        || search_word_stream_eq_phrase(words, &["cards"])
}

fn search_library_prefix_len(
    tokens: &[OwnedLexToken],
    phrase: &'static [&'static str],
) -> Option<usize> {
    primitives::parse_prefix(tokens, primitives::phrase(phrase))
        .map(|(_, rest)| tokens.len().saturating_sub(rest.len()))
}

fn search_library_word_index(words: &[&str], expected: &str) -> Option<usize> {
    let mut idx = 0usize;
    while idx < words.len() {
        if words[idx] == expected {
            return Some(idx);
        }
        idx += 1;
    }
    None
}

pub(crate) fn last_non_article_parser_word_token_idx(
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

pub(crate) fn normalize_subject_routing_word(word: &str) -> String {
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

pub(crate) fn subject_routing_word_refs(tokens: &[OwnedLexToken]) -> Vec<String> {
    parser_token_word_refs(tokens)
        .into_iter()
        .map(normalize_subject_routing_word)
        .collect()
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

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SearchLibraryEffectRouting {
    pub(crate) destination: Zone,
    pub(crate) reveal: bool,
    pub(crate) shuffle: bool,
    pub(crate) face_down_exile: bool,
    pub(crate) split_battlefield_and_hand: bool,
    pub(crate) has_tapped_modifier: bool,
    pub(crate) library_position_from_top: Option<Value>,
    pub(crate) result_reference_surface: crate::effect::SearchResultReferenceSurface,
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
    pub(crate) count_value: Option<Value>,
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
    pub(crate) same_name_relation: TaggedOpbjectRelation,
    pub(crate) same_name_antecedent_surface: Option<ironsmith_core::SameNameAntecedentSurface>,
}

#[derive(Debug, Clone)]
pub(crate) struct SearchLibraryLeadingPrelude<'a> {
    pub(crate) subject_tokens: &'a [OwnedLexToken],
    pub(crate) leading_effects: Vec<EffectAst>,
}

pub(crate) fn conditional_label_phrase<'a>(
    input: &mut LexStream<'a>,
) -> Result<(), ErrMode<ContextError>> {
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
        "radiance" => primitives::phrase(&["radiance"]),
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

pub(crate) fn split_search_library_sentence_head_lexed(
    tokens: &[OwnedLexToken],
) -> Option<SearchLibrarySentenceHeadSplit<'_>> {
    let mut inside_quotes = false;

    for (idx, token) in tokens.iter().enumerate() {
        if token.is_quote() {
            inside_quotes = !inside_quotes;
            continue;
        }
        if inside_quotes {
            continue;
        }
        if search_library_token_is_any_word(token, &["unless"]) {
            return None;
        }
        if search_library_token_is_any_word(token, &["may"]) {
            if tokens
                .get(idx + 1)
                .is_some_and(|next| search_library_token_is_any_word(next, &["search", "searches"]))
            {
                return Some(SearchLibrarySentenceHeadSplit {
                    subject_tokens: &tokens[..idx],
                    search_tokens: &tokens[idx + 1..],
                    sentence_has_direct_may: true,
                });
            }
            return None;
        }
        if search_library_token_is_any_word(token, &["search", "searches"]) {
            return Some(SearchLibrarySentenceHeadSplit {
                subject_tokens: &tokens[..idx],
                search_tokens: &tokens[idx..],
                sentence_has_direct_may: false,
            });
        }
    }

    None
}

pub(crate) fn search_library_search_verb<'a>(
    input: &mut LexStream<'a>,
) -> Result<(), ErrMode<ContextError>> {
    alt((primitives::kw("search"), primitives::kw("searches")))
        .void()
        .parse_next(input)
}

pub(crate) fn search_library_put_marker<'a>(
    input: &mut LexStream<'a>,
) -> Result<(), ErrMode<ContextError>> {
    alt((primitives::kw("put"), primitives::kw("puts")))
        .void()
        .parse_next(input)
}

pub(crate) fn search_library_reveal_marker<'a>(
    input: &mut LexStream<'a>,
) -> Result<(), ErrMode<ContextError>> {
    alt((primitives::kw("reveal"), primitives::kw("reveals")))
        .void()
        .parse_next(input)
}

pub(crate) fn search_library_shuffle_marker<'a>(
    input: &mut LexStream<'a>,
) -> Result<(), ErrMode<ContextError>> {
    alt((primitives::kw("shuffle"), primitives::kw("shuffles")))
        .void()
        .parse_next(input)
}

pub(crate) fn search_library_for_marker<'a>(
    input: &mut LexStream<'a>,
) -> Result<(), ErrMode<ContextError>> {
    primitives::kw("for").void().parse_next(input)
}

pub(crate) fn search_library_exile_destination_marker<'a>(
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

pub(crate) fn search_library_then_marker<'a>(
    input: &mut LexStream<'a>,
) -> Result<(), ErrMode<ContextError>> {
    primitives::kw("then").void().parse_next(input)
}

pub(crate) fn search_library_and_marker<'a>(
    input: &mut LexStream<'a>,
) -> Result<(), ErrMode<ContextError>> {
    primitives::kw("and").void().parse_next(input)
}

pub(crate) fn search_library_discard_marker<'a>(
    input: &mut LexStream<'a>,
) -> Result<(), ErrMode<ContextError>> {
    alt((primitives::kw("discard"), primitives::kw("discards")))
        .void()
        .parse_next(input)
}

pub(crate) fn search_library_reveal_or_then_marker<'a>(
    input: &mut LexStream<'a>,
) -> Result<(), ErrMode<ContextError>> {
    alt((search_library_reveal_marker, search_library_then_marker)).parse_next(input)
}

pub(crate) fn search_library_comma_filter_break_marker<'a>(
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

pub(crate) fn search_library_with_that_name_suffix<'a>(
    input: &mut LexStream<'a>,
) -> Result<(), ErrMode<ContextError>> {
    primitives::phrase(&["with", "that", "name"])
        .void()
        .parse_next(input)
}

pub(crate) fn search_library_with_the_chosen_name_suffix<'a>(
    input: &mut LexStream<'a>,
) -> Result<(), ErrMode<ContextError>> {
    primitives::phrase(&["with", "the", "chosen", "name"])
        .void()
        .parse_next(input)
}

pub(crate) fn search_library_with_chosen_name_suffix<'a>(
    input: &mut LexStream<'a>,
) -> Result<(), ErrMode<ContextError>> {
    primitives::phrase(&["with", "chosen", "name"])
        .void()
        .parse_next(input)
}

pub(crate) fn strip_search_library_suffix_lexed(
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

pub(crate) fn strip_search_library_leading_count_tokens(
    tokens: &[OwnedLexToken],
) -> Vec<OwnedLexToken> {
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

pub(crate) fn strip_search_library_different_names_clause_lexed(
    tokens: &[OwnedLexToken],
) -> (Vec<OwnedLexToken>, bool) {
    let mut cursor = 0usize;
    while cursor < tokens.len() {
        for pattern_len in [3usize, 4usize] {
            if cursor + pattern_len <= tokens.len()
                && search_library_words_equal_any(
                    &parser_token_word_refs(&tokens[cursor..cursor + pattern_len]),
                    DIFFERENT_NAMES_CLAUSES,
                )
            {
                let mut stripped = Vec::with_capacity(tokens.len() - pattern_len);
                stripped.extend_from_slice(&tokens[..cursor]);
                stripped.extend_from_slice(&tokens[cursor + pattern_len..]);
                return (trim_commas(&stripped), true);
            }
        }
        cursor += 1;
    }

    (trim_commas(tokens), false)
}

fn strip_search_library_color_count_phrase_lexed(
    tokens: &[OwnedLexToken],
) -> Option<(Vec<OwnedLexToken>, crate::filter::Comparison)> {
    let trimmed = trim_commas(tokens);
    let patterns: [&[&str]; 4] = [
        &["thats", "exactly", "that", "many", "colors", "plus"],
        &["thats", "that", "many", "colors", "plus"],
        &["exactly", "that", "many", "colors", "plus"],
        &["that", "many", "colors", "plus"],
    ];

    for pattern in patterns {
        let mut cursor = 0usize;
        while cursor < trimmed.len() {
            let Some((_, rest)) =
                primitives::parse_prefix(&trimmed[cursor..], primitives::phrase(pattern))
            else {
                cursor += 1;
                continue;
            };
            let rest = trim_commas(rest);
            let Some((count, consumed)) = parse_number(&rest) else {
                cursor += 1;
                continue;
            };
            let mut stripped = trim_commas(&trimmed[..cursor]).to_vec();
            stripped.extend_from_slice(&trim_commas(&rest[consumed..]));

            let colors_expr = crate::effect::Value::ColorsAmong(
                crate::target::ObjectFilter::tagged(crate::cards::builders::IT_TAG),
            );
            let comparison =
                crate::filter::Comparison::EqualExpr(Box::new(crate::effect::Value::Add(
                    Box::new(colors_expr),
                    Box::new(crate::effect::Value::Fixed(count as i32)),
                )));
            return Some((stripped, comparison));
        }
    }

    None
}

pub(crate) fn is_default_search_library_card_selector(tokens: &[OwnedLexToken]) -> bool {
    let parser_words = parser_token_word_refs(tokens);
    let words = crate::runtime_backend::util::non_article_word_refs(&parser_words);
    search_library_words_are_default_card_selector(&words)
}

pub(crate) fn parse_search_library_basic_land_type_slots_lexed(
    tokens: &[OwnedLexToken],
) -> Option<Vec<SearchLibrarySlotAst>> {
    let parser_words = parser_token_word_refs(tokens);
    let words = crate::runtime_backend::util::non_article_word_refs(&parser_words);
    if !search_word_stream_eq_phrase(&words, BASIC_LAND_TYPE_SEARCH_SELECTOR) {
        return None;
    }

    Some(
        [
            Subtype::Plains,
            Subtype::Island,
            Subtype::Swamp,
            Subtype::Mountain,
            Subtype::Forest,
        ]
        .into_iter()
        .map(|subtype| SearchLibrarySlotAst {
            filter: ObjectFilter::default()
                .in_zone(Zone::Library)
                .with_type(CardType::Land)
                .with_subtype(subtype),
            optional: true,
        })
        .collect(),
    )
}

pub(crate) fn find_search_library_marker_lexed(
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

pub(crate) fn find_last_search_library_marker_lexed(
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
        if token.is_comma() || search_library_token_is_any_word(token, &["and", "then"]) {
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
        if token.is_comma() || search_library_token_is_any_word(token, &["and", "then"]) {
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

    let trailing_words = parser_token_word_refs(trailing_tokens);
    let starts_with_life_clause =
        search_word_stream_starts_with_any(&trailing_words, LIFE_FOLLOWUP_PREFIXES);

    starts_with_life_clause.then_some(trailing_tokens)
}

pub(crate) fn find_search_library_trailing_create_followup_lexed<'a>(
    search_tokens: &'a [OwnedLexToken],
    start_idx: usize,
) -> Option<&'a [OwnedLexToken]> {
    let marker_idx = find_search_library_marker_lexed(
        &search_tokens[start_idx..],
        |input: &mut LexStream<'_>| {
            let _ = (
                alt((
                    super::super::primitives::kw("then"),
                    super::super::primitives::kw("and"),
                )),
                super::super::primitives::kw("create"),
            )
                .parse_next(input)?;
            Ok(())
        },
    )?;
    let mut trailing_start = start_idx + marker_idx;
    if search_tokens
        .get(trailing_start)
        .is_some_and(|token| search_library_token_is_any_word(token, &["and", "then"]))
    {
        trailing_start += 1;
    }
    let mut trailing_end = search_tokens.len();
    if let Some(shuffle_idx) = find_search_library_marker_lexed(
        &search_tokens[trailing_start..],
        search_library_shuffle_marker,
    ) {
        trailing_end = trailing_start + shuffle_idx;
    }
    while trailing_start < trailing_end && search_tokens[trailing_start].is_comma() {
        trailing_start += 1;
    }
    while trailing_end > trailing_start {
        let token = &search_tokens[trailing_end - 1];
        if token.is_comma() || search_library_token_is_any_word(token, &["and", "then"]) {
            trailing_end -= 1;
            continue;
        }
        break;
    }
    let trailing_tokens = &search_tokens[trailing_start..trailing_end];
    (!trailing_tokens.is_empty()
        && trailing_tokens
            .first()
            .is_some_and(|token| search_library_token_is_any_word(token, &["create"])))
    .then_some(trailing_tokens)
}

pub(crate) fn derive_search_library_effect_routing_lexed(
    tokens: &[OwnedLexToken],
    search_tokens: &[OwnedLexToken],
    clause_markers: SearchLibraryClauseMarkers,
    trailing_discard_before_shuffle: bool,
) -> SearchLibraryEffectRouting {
    let words_all = parser_token_word_refs(tokens);
    let put_clause_words = clause_markers
        .put_idx
        .map(|put_idx| parser_token_word_refs(&search_tokens[put_idx..]));
    let destination = if let Some(put_clause_words) = put_clause_words.as_ref() {
        if search_library_words_have_word(put_clause_words, "graveyard") {
            Zone::Graveyard
        } else if search_library_words_have_word(put_clause_words, "hand") {
            Zone::Hand
        } else if search_library_words_have_word(put_clause_words, "top") {
            Zone::Library
        } else {
            Zone::Battlefield
        }
    } else {
        Zone::Exile
    };
    let reveal = clause_markers.reveal_idx.is_some();
    let face_down_exile = clause_markers.exile_idx.is_some_and(|idx| {
        search_word_stream_matches_at_some_offset(
            &parser_token_word_refs(&search_tokens[idx..]),
            FACE_DOWN_PHRASE,
        )
    });
    let shuffle = clause_markers.shuffle_idx.is_some() && !trailing_discard_before_shuffle;
    let split_battlefield_and_hand = clause_markers.put_idx.is_some()
        && search_library_words_contain_all(&words_all, BATTLEFIELD_HAND_OTHER_ONE_MARKER_WORDS);
    let has_tapped_modifier = search_library_words_have_word(&words_all, "tapped");

    SearchLibraryEffectRouting {
        destination,
        reveal,
        shuffle,
        face_down_exile,
        split_battlefield_and_hand,
        has_tapped_modifier,
        library_position_from_top: put_clause_words
            .as_ref()
            .and_then(|words| search_library_put_position_from_top_words(words)),
        result_reference_surface: if words_all
            .windows(3)
            .any(|words| words == ["put", "the", "card"])
        {
            crate::effect::SearchResultReferenceSurface::TheCard
        } else {
            crate::effect::SearchResultReferenceSurface::ThatCard
        },
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

    if search_word_stream_starts_with_any(
        search_body_words,
        YOUR_OR_THEIR_LIBRARY_GRAVEYARD_FOR_PREFIX_PATTERN,
    ) {
        forced_library_owner = Some(match chooser {
            PlayerAst::Target => PlayerFilter::target_player(),
            PlayerAst::TargetOpponent => PlayerFilter::target_opponent(),
            PlayerAst::Opponent => PlayerFilter::Opponent,
            PlayerAst::NotYou => PlayerFilter::NotYou,
            PlayerAst::That | PlayerAst::Any => PlayerFilter::IteratedPlayer,
            PlayerAst::ThatPlayerOrTargetController => {
                PlayerFilter::TargetPlayerOrControllerOfTarget
            }
            PlayerAst::ItsController => {
                PlayerFilter::ControllerOf(crate::filter::ObjectRef::Target)
            }
            PlayerAst::ItsOwner => PlayerFilter::OwnerOf(crate::filter::ObjectRef::Target),
            _ => PlayerFilter::You,
        });
        search_zones_override = Some(vec![Zone::Library, Zone::Graveyard]);
    } else if search_word_stream_starts_with_any(
        search_body_words,
        YOUR_OR_THEIR_LIBRARY_FOR_PREFIX_PATTERN,
    ) {
        if search_body_words.first() == Some(&"their") {
            forced_library_owner = Some(match chooser {
                PlayerAst::Target => PlayerFilter::target_player(),
                PlayerAst::TargetOpponent => PlayerFilter::target_opponent(),
                PlayerAst::Opponent => PlayerFilter::Opponent,
                PlayerAst::NotYou => PlayerFilter::NotYou,
                PlayerAst::That | PlayerAst::Any => PlayerFilter::IteratedPlayer,
                PlayerAst::ThatPlayerOrTargetController => {
                    PlayerFilter::TargetPlayerOrControllerOfTarget
                }
                PlayerAst::ItsController => {
                    PlayerFilter::ControllerOf(crate::filter::ObjectRef::Target)
                }
                PlayerAst::ItsOwner => PlayerFilter::OwnerOf(crate::filter::ObjectRef::Target),
                _ => PlayerFilter::You,
            });
        }
    } else if search_word_stream_starts_with_any(
        search_body_words,
        CONTROLLER_GRAVEYARD_HAND_LIBRARY_FOR_PREFIX_PATTERN,
    ) || search_word_stream_starts_with_any(
        search_body_words,
        CONTROLLER_SUFFIX_GRAVEYARD_HAND_LIBRARY_FOR_PREFIX_PATTERN,
    ) {
        player = PlayerAst::ItsController;
        forced_library_owner = Some(PlayerFilter::ControllerOf(crate::filter::ObjectRef::Target));
        search_zones_override = Some(vec![Zone::Graveyard, Zone::Hand, Zone::Library]);
    } else if search_word_stream_starts_with_any(
        search_body_words,
        OWNER_GRAVEYARD_HAND_LIBRARY_FOR_PREFIX_PATTERN,
    ) {
        player = PlayerAst::ItsOwner;
        forced_library_owner = Some(PlayerFilter::OwnerOf(crate::filter::ObjectRef::Target));
        search_zones_override = Some(vec![Zone::Graveyard, Zone::Hand, Zone::Library]);
    } else if search_word_stream_starts_with_any(
        search_body_words,
        TARGET_PLAYER_GRAVEYARD_HAND_LIBRARY_FOR_PREFIX_PATTERN,
    ) {
        player = PlayerAst::That;
        search_player_target = Some(TargetAst::Player(
            PlayerFilter::target_player(),
            span_from_tokens(&search_tokens[1..3]),
        ));
        forced_library_owner = Some(PlayerFilter::target_player());
        search_zones_override = Some(vec![Zone::Graveyard, Zone::Hand, Zone::Library]);
    } else if search_word_stream_starts_with_any(
        search_body_words,
        TARGET_OPPONENT_GRAVEYARD_HAND_LIBRARY_FOR_PREFIX_PATTERN,
    ) {
        player = PlayerAst::That;
        search_player_target = Some(TargetAst::Player(
            PlayerFilter::target_opponent(),
            span_from_tokens(&search_tokens[1..3]),
        ));
        forced_library_owner = Some(PlayerFilter::target_opponent());
        search_zones_override = Some(vec![Zone::Graveyard, Zone::Hand, Zone::Library]);
    } else if search_word_stream_starts_with_any(
        search_body_words,
        TARGET_PLAYER_LIBRARY_FOR_PREFIX_PATTERN,
    ) {
        player = PlayerAst::That;
        search_player_target = Some(TargetAst::Player(
            PlayerFilter::target_player(),
            span_from_tokens(&search_tokens[1..3]),
        ));
        forced_library_owner = Some(PlayerFilter::target_player());
    } else if search_word_stream_starts_with_any(
        search_body_words,
        TARGET_OPPONENT_LIBRARY_FOR_PREFIX_PATTERN,
    ) {
        player = PlayerAst::That;
        search_player_target = Some(TargetAst::Player(
            PlayerFilter::target_opponent(),
            span_from_tokens(&search_tokens[1..3]),
        ));
        forced_library_owner = Some(PlayerFilter::target_opponent());
    } else if search_word_stream_starts_with_any(
        search_body_words,
        THAT_PLAYER_LIBRARY_FOR_PREFIX_PATTERN,
    ) {
        player = PlayerAst::That;
        // Keep this as a discourse-level player reference. Lowering binds the
        // library filter to the resolved `That` subject: a preceding target in
        // ordinary spell text, or IteratedPlayer only inside a real loop.
    } else if search_word_stream_starts_with_any(
        search_body_words,
        THAT_PLAYER_GRAVEYARD_HAND_LIBRARY_FOR_PREFIX_PATTERN,
    ) {
        player = PlayerAst::That;
        search_zones_override = Some(vec![Zone::Graveyard, Zone::Hand, Zone::Library]);
    } else if search_word_stream_starts_with_any(
        search_body_words,
        CONTROLLER_LIBRARY_FOR_PREFIX_PATTERN,
    ) {
        player = PlayerAst::ItsController;
    } else if search_word_stream_starts_with_any(
        search_body_words,
        OWNER_LIBRARY_FOR_PREFIX_PATTERN,
    ) {
        player = PlayerAst::ItsOwner;
    } else if search_body_words
        .first()
        .is_some_and(|word| *word == "your")
        && let Some(for_pos) = search_library_word_index(search_body_words, "for")
        && for_pos > 1
    {
        let zone_words = &search_body_words[1..for_pos];
        let mut zones = Vec::new();
        let mut saw_library = false;
        let mut saw_graveyard = false;
        let mut saw_hand = false;
        let mut saw_outside_game = false;
        for word in zone_words {
            match *word {
                "graveyard" | "graveyards" if !saw_graveyard => {
                    zones.push(Zone::Graveyard);
                    saw_graveyard = true;
                }
                "hand" | "hands" if !saw_hand => {
                    zones.push(Zone::Hand);
                    saw_hand = true;
                }
                "library" | "libraries" if !saw_library => {
                    zones.push(Zone::Library);
                    saw_library = true;
                }
                "outside"
                    if !saw_outside_game && search_library_words_have_word(zone_words, "game") =>
                {
                    zones.push(Zone::OutsideGame);
                    saw_outside_game = true;
                }
                _ => {}
            }
        }
        if !saw_library || zones.is_empty() {
            return None;
        }
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
    let mut count = ChoiceCount::exactly(1);
    let mut search_mode = SearchSelectionMode::Exact;
    let mut count_used = 0usize;
    let mut count_value = None;

    if count_tokens
        .first()
        .is_some_and(|token| search_library_token_is_any_word(token, &["any"]))
        && search_library_prefix_len(count_tokens, ANY_NUMBER_PREFIX).is_none()
    {
        if let Some((value, used)) = parse_number(&count_tokens[1..]) {
            count = ChoiceCount::up_to(value as usize);
            search_mode = SearchSelectionMode::Optional;
            count_used = 1 + used;
        }
    } else if search_library_prefix_len(count_tokens, THAT_MANY_PREFIX).is_some() {
        count = ChoiceCount::dynamic_x();
        count_value = Some(Value::EventValue(crate::effect::EventValueSpec::Amount));
        count_used = 2;
    } else if search_library_prefix_len(count_tokens, UP_TO_X_PREFIX).is_some() {
        count = ChoiceCount::up_to_dynamic_x();
        search_mode = SearchSelectionMode::Optional;
        count_used = 3;
    } else if token_slice_first_is(count_tokens, "all") {
        count = ChoiceCount::any_number();
        search_mode = SearchSelectionMode::AllMatching;
        count_used = 1;
    } else if search_library_prefix_len(count_tokens.get(2..).unwrap_or(&[]), THAT_MANY_PREFIX)
        .is_some()
        && search_library_prefix_len(count_tokens, UP_TO_PREFIX).is_some()
    {
        count = ChoiceCount::up_to_dynamic_x();
        search_mode = SearchSelectionMode::Optional;
        count_value = Some(Value::EventValue(crate::effect::EventValueSpec::Amount));
        count_used = 4;
    } else if token_slice_first_is(count_tokens, "exactly") {
        if let Some((value, used)) = parse_number(&count_tokens[1..]) {
            count = ChoiceCount::exactly(value as usize);
            count_used = 1 + used;
        }
    } else if let Some((parsed_count, used)) =
        parse_choice_count_token_prefix_consumed(count_tokens)
    {
        let is_optional_count = parsed_count.is_any_number()
            || parsed_count.is_up_to_dynamic_x()
            || (parsed_count.min == 0 && parsed_count.max.is_some() && !parsed_count.dynamic_x);
        count = parsed_count;
        if is_optional_count {
            search_mode = SearchSelectionMode::Optional;
        }
        count_used = used;
    }

    if token_slice_at_is(count_tokens, count_used, "of") {
        count_used += 1;
    }

    SearchLibraryCountPrefix {
        count,
        search_mode,
        count_used,
        count_value,
    }
}

pub(crate) fn parse_search_library_same_name_reference_lexed(
    raw_filter_tokens: &[OwnedLexToken],
    mut filter_tokens: Vec<OwnedLexToken>,
    clause_display: &str,
) -> Result<SearchLibrarySameNameSplit, CardTextError> {
    let mut same_name_reference: Option<SearchLibrarySameNameReference> = None;
    let mut same_name_relation = TaggedOpbjectRelation::SameNameAsTagged;
    let mut same_name_antecedent_surface = None;
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
    } else if let Some((base_filter_tokens, reference_tokens, relation)) =
        split_search_same_name_reference_filter(raw_filter_tokens)
            .map(|(base_filter_tokens, reference_tokens)| {
                (
                    base_filter_tokens,
                    reference_tokens,
                    TaggedOpbjectRelation::SameNameAsTagged,
                )
            })
            .or_else(|| {
                split_search_different_name_reference_filter(raw_filter_tokens).map(
                    |(base_filter_tokens, reference_tokens)| {
                        (
                            base_filter_tokens,
                            reference_tokens,
                            TaggedOpbjectRelation::DifferentNameFromTagged,
                        )
                    },
                )
            })
    {
        if base_filter_tokens.is_empty() || reference_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "incomplete same-name search filter in search-library sentence (clause: '{}')",
                clause_display
            )));
        }
        filter_tokens = base_filter_tokens;
        same_name_relation = relation;
        let reference_words = token_word_refs(&reference_tokens);
        same_name_antecedent_surface = same_name_antecedent_surface_words(&reference_words);
        let source_exiled_reference = primitives::parse_all(
            &reference_tokens,
            (
                primitives::any_phrase(&[&["the", "exiled", "card"], &["the", "exiled", "cards"]]),
                primitives::sentence_end(),
            )
                .void(),
            "source-exiled same-name reference",
        )
        .is_ok();
        same_name_reference = if source_exiled_reference {
            Some(SearchLibrarySameNameReference::Tagged(TagKey::from(
                crate::tag::SOURCE_EXILED_TAG,
            )))
        } else if is_same_name_that_reference_words(&reference_words) {
            Some(SearchLibrarySameNameReference::Tagged(TagKey::from(IT_TAG)))
        } else if search_library_words_have_word(&reference_words, "target") {
            let target = parse_target_phrase(&reference_tokens).map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported target same-name reference in search-library sentence (clause: '{}')",
                    clause_display
                ))
            })?;
            Some(SearchLibrarySameNameReference::Target(target))
        } else {
            let mut reference_filter_tokens = reference_tokens.clone();
            let mut other_reference = false;
            if reference_filter_tokens
                .first()
                .is_some_and(|token| search_library_token_is_any_word(token, &["another", "other"]))
            {
                other_reference = true;
                reference_filter_tokens = trim_commas(&reference_filter_tokens[1..]);
            }
            let reference_filter = parse_object_filter(&reference_filter_tokens, other_reference)
                .map_err(|_| {
                    CardTextError::ParseError(format!(
                        "unsupported same-name reference filter in search-library sentence (clause: '{}')",
                        clause_display
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
        same_name_relation,
        same_name_antecedent_surface,
    })
}

pub(crate) fn parse_search_library_object_filter_lexed(
    filter_tokens: &[OwnedLexToken],
    clause_display: &str,
) -> Result<ObjectFilter, CardTextError> {
    let (filter_tokens, color_count) = if let Some((stripped, color_count)) =
        strip_search_library_color_count_phrase_lexed(filter_tokens)
    {
        (stripped, Some(color_count))
    } else {
        (filter_tokens.to_vec(), None)
    };
    let (filter_tokens, distinct_names) =
        strip_search_library_different_names_clause_lexed(&filter_tokens);
    let raw_filter_words = parser_token_word_refs(&filter_tokens);
    let filter_words = crate::runtime_backend::util::non_article_word_refs(&raw_filter_words);
    let parser_words = parser_token_word_positions(&filter_tokens);

    let parser_word_refs = parser_words
        .iter()
        .map(|(_, word)| *word)
        .collect::<Vec<_>>();
    if let Some(named_idx) = search_library_word_index(&parser_word_refs, "named") {
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
            .collect::<Vec<_>>();
        let name = name_words.join(" ");
        if name.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing card name in named search clause (clause: '{}')",
                clause_display
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
                    clause_display
                ))
            })?
        };
        if negated_named {
            base_filter.excluded_name = Some(name);
        } else {
            base_filter.name = Some(name);
        }
        if let Some(color_count) = color_count {
            base_filter.color_count = Some(color_count);
        }
        base_filter.distinct_names |= distinct_names;
        Ok(base_filter)
    } else if search_library_words_are_default_card_selector(&filter_words) {
        let mut filter = ObjectFilter::default();
        if let Some(color_count) = color_count {
            filter.color_count = Some(color_count);
        }
        filter.distinct_names |= distinct_names;
        Ok(filter)
    } else if search_library_words_have_word(&filter_words, "or")
        || search_library_words_have_word(&filter_words, "and/or")
    {
        let mut filter = parse_search_library_disjunction_filter(&filter_tokens)
            .or_else(|| parse_object_filter(&filter_tokens, false).ok())
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported search filter in search-library sentence (clause: '{}')",
                    clause_display
                ))
            })?;
        if let Some(color_count) = color_count {
            filter.color_count = Some(color_count);
        }
        filter.distinct_names |= distinct_names;
        Ok(filter)
    } else {
        let mut filter = parse_object_filter(&filter_tokens, false).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported search filter in search-library sentence (clause: '{}')",
                clause_display
            ))
        })?;
        if let Some(color_count) = color_count {
            filter.color_count = Some(color_count);
        }
        filter.distinct_names |= distinct_names;
        Ok(filter)
    }
}

pub(crate) fn split_search_named_item_filters_lexed(
    filter_tokens: &[OwnedLexToken],
    clause_display: &str,
) -> Result<Option<Vec<ObjectFilter>>, CardTextError> {
    if !crate::runtime_backend::lexer::contains_token_word(filter_tokens, "named") {
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
            .is_some_and(|token| search_library_token_is_any_word(token, &["and"]))
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
            .is_some_and(|token| search_library_token_is_any_word(token, &["a", "an"]))
        {
            cursor += 1;
        }
        if !filter_tokens
            .get(cursor)
            .is_some_and(|token| search_library_token_is_any_word(token, &["card", "cards"]))
            || !filter_tokens
                .get(cursor + 1)
                .is_some_and(|token| search_library_token_is_any_word(token, &["named"]))
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
                .is_some_and(|token| search_library_token_is_any_word(token, &["and"]))
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
                .is_some_and(|token| search_library_token_is_any_word(token, &["a", "an"]))
            {
                phrase_probe += 1;
            }
            if filter_tokens
                .get(phrase_probe)
                .is_some_and(|token| search_library_token_is_any_word(token, &["card", "cards"]))
                && filter_tokens
                    .get(phrase_probe + 1)
                    .is_some_and(|token| search_library_token_is_any_word(token, &["named"]))
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
        let item_filter = parse_search_library_object_filter_lexed(&item_tokens, clause_display)?;
        if item_filter.name.is_none() {
            return Ok(None);
        }
        filters.push(item_filter);
    }
    Ok(Some(filters))
}

pub(crate) fn parse_search_library_leading_effect_prelude_lexed<'a>(
    subject_tokens: &'a [OwnedLexToken],
    subject_starts_effect_lexed: fn(&[OwnedLexToken]) -> bool,
    parse_leading_effects_lexed: fn(&[OwnedLexToken]) -> Result<Vec<EffectAst>, CardTextError>,
) -> Result<SearchLibraryLeadingPrelude<'a>, CardTextError> {
    let typed_effect_head = super::chain_splitting::find_chain_verb_tokens(subject_tokens)
        .is_some()
        || super::chain_splitting::has_extended_effect_head_tokens(subject_tokens);
    if subject_tokens.is_empty()
        || (!subject_starts_effect_lexed(subject_tokens) && !typed_effect_head)
    {
        return Ok(SearchLibraryLeadingPrelude {
            subject_tokens,
            leading_effects: Vec::new(),
        });
    }

    let mut leading_tokens = trim_commas(subject_tokens);
    while leading_tokens
        .last()
        .is_some_and(|token| search_library_token_is_any_word(token, &["and", "then"]))
    {
        leading_tokens.pop();
    }
    let leading_effects = if leading_tokens.is_empty() {
        Vec::new()
    } else {
        parse_leading_effects_lexed(&leading_tokens)?
    };

    Ok(SearchLibraryLeadingPrelude {
        subject_tokens: &[],
        leading_effects,
    })
}

pub(crate) fn search_library_has_unsupported_top_position_probe(words: &[&str]) -> bool {
    word_slice_mentions_nth_from_top(words)
        && !search_word_stream_matches_at_some_offset(words, ON_TOP_OF_LIBRARY_PHRASE)
        && search_library_put_position_from_top_words(words).is_none()
}

pub(crate) fn search_library_has_unsupported_top_position_probe_lexed(
    tokens: &[OwnedLexToken],
) -> bool {
    let words = parser_token_word_refs(tokens);
    search_library_has_unsupported_top_position_probe(&words)
}

pub(crate) fn search_library_put_position_from_top_words(words: &[&str]) -> Option<Value> {
    let mut idx = 0usize;
    while idx < words.len() {
        let Some((position, used)) = ironsmith_core::parse_ordinal_words(&words[idx..]) else {
            idx += 1;
            continue;
        };
        if idx + used + 2 < words.len()
            && words[idx + used..].starts_with(FROM_THE_TOP_PREFIX)
            && words[..idx]
                .iter()
                .any(|word| matches!(*word, "put" | "puts"))
        {
            return Some(Value::Fixed(position as i32));
        }
        idx += 1;
    }
    None
}

pub(crate) fn search_library_subject_wraps_each_target_player_lexed(
    subject_tokens: &[OwnedLexToken],
) -> bool {
    token_word_refs(subject_tokens).as_slice() == EACH_OF_THEM_SUBJECT
}

pub(crate) fn search_library_subject_player_iteration_filter_lexed(
    subject_tokens: &[OwnedLexToken],
) -> Option<PlayerFilter> {
    let words = token_word_refs(subject_tokens)
        .into_iter()
        .map(|word| word.to_ascii_lowercase())
        .collect::<Vec<_>>();
    match words
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["each", "player"] | ["each", "players"] => Some(PlayerFilter::Any),
        ["each", "opponent"] | ["each", "opponents"] => Some(PlayerFilter::Opponent),
        ["each", "other", "player"] | ["each", "other", "players"] => Some(PlayerFilter::NotYou),
        _ => None,
    }
}

pub(crate) fn parse_search_library_iterated_object_subject_lexed(
    subject_tokens: &[OwnedLexToken],
) -> Result<Option<ObjectFilter>, CardTextError> {
    const PLAYER_OR_OPPONENT_PREFIXES: &[&[&str]] = &[
        &["player"],
        &["players"],
        &["opponent"],
        &["opponents"],
        &["target", "player"],
        &["target", "players"],
        &["target", "opponent"],
        &["target", "opponents"],
    ];

    if subject_tokens.is_empty() {
        return Ok(None);
    }
    if token_word_refs(subject_tokens).as_slice() == EACH_OF_THEM_SUBJECT {
        return Ok(None);
    }

    let mut filter_tokens = if let Some((_, rest)) =
        primitives::strip_lexed_prefix_phrases(subject_tokens, &[&["for", "each"]])
    {
        rest
    } else if let Some((_, rest)) =
        primitives::strip_lexed_prefix_phrases(subject_tokens, &[&["each"]])
    {
        rest
    } else {
        return Ok(None);
    };

    if filter_tokens
        .first()
        .is_some_and(|token| search_library_token_is_any_word(token, &["of"]))
    {
        filter_tokens = &filter_tokens[1..];
    }

    let filter_tokens = trim_commas(filter_tokens);
    if filter_tokens.is_empty() {
        return Ok(None);
    }

    if primitives::strip_lexed_prefix_phrases(&filter_tokens, PLAYER_OR_OPPONENT_PREFIXES).is_some()
    {
        return Ok(None);
    }

    Ok(Some(parse_object_filter_lexed(&filter_tokens, false)?))
}

pub(crate) fn search_library_starts_with_search_verb_lexed(
    search_tokens: &[OwnedLexToken],
) -> bool {
    primitives::parse_prefix(search_tokens, search_library_search_verb).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::lex_line;

    #[test]
    fn prior_or_trigger_amount_drives_that_many_search_counts() {
        for (surface, expected_used, up_to) in
            [("that many", 2, false), ("up to that many", 4, true)]
        {
            let tokens = lex_line(surface, 0).expect("count surface should lex");
            let parsed = parse_search_library_count_prefix_lexed(&tokens);

            assert_eq!(parsed.count_used, expected_used);
            assert_eq!(parsed.count.up_to_x, up_to);
            assert_eq!(
                parsed.count_value,
                Some(Value::EventValue(crate::effect::EventValueSpec::Amount))
            );
        }
    }

    #[test]
    fn that_players_library_remains_a_contextual_player_reference() {
        let tokens = lex_line("search that player's library for a card", 0)
            .expect("search surface should lex");
        let routing = derive_search_library_subject_routing_lexed(&tokens, PlayerAst::Implicit)
            .expect("that-player search should route");

        assert_eq!(routing.player, PlayerAst::That);
        assert_eq!(routing.forced_library_owner, None);
        assert_eq!(routing.search_player_target, None);
    }
}
