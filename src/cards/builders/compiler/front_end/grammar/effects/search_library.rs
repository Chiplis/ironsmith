use super::*;

const CHOSEN_NAME_TAG: &str = "__chosen_name__";

pub(crate) fn parser_text_word_refs(tokens: &[OwnedLexToken]) -> Vec<&str> {
    parser_token_word_refs(tokens)
}

pub(crate) fn parser_word_token_positions(tokens: &[OwnedLexToken]) -> Vec<(usize, &str)> {
    parser_token_word_positions(tokens)
}

pub(crate) fn find_parser_word_position(
    parser_words: &[(usize, &str)],
    expected: &str,
) -> Option<usize> {
    let mut idx = 0usize;
    while idx < parser_words.len() {
        if parser_words[idx].1 == expected {
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
    parser_text_word_refs(tokens)
        .into_iter()
        .map(normalize_subject_routing_word)
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SearchLibrarySentenceHeadKind {
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

pub(crate) fn search_library_sentence_head<'a>(
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
    let words = parser_text_word_refs(tokens)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect::<Vec<_>>();
    words.is_empty() || words.as_slice() == ["card"] || words.as_slice() == ["cards"]
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
    let (filter_tokens, color_count) = if let Some((stripped, color_count)) =
        strip_search_library_color_count_phrase_lexed(filter_tokens)
    {
        (stripped, Some(color_count))
    } else {
        (filter_tokens.to_vec(), None)
    };
    let filter_words = parser_text_word_refs(&filter_tokens)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect::<Vec<_>>();
    let parser_words = parser_word_token_positions(&filter_tokens);

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
        if let Some(color_count) = color_count {
            base_filter.color_count = Some(color_count);
        }
        Ok(base_filter)
    } else if filter_words.len() == 1 && (filter_words[0] == "card" || filter_words[0] == "cards") {
        let mut filter = ObjectFilter::default();
        if let Some(color_count) = color_count {
            filter.color_count = Some(color_count);
        }
        Ok(filter)
    } else if word_slice_contains(&filter_words, "or") {
        let mut filter = parse_search_library_disjunction_filter(&filter_tokens)
            .or_else(|| parse_object_filter(&filter_tokens, false).ok())
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported search filter in search-library sentence (clause: '{}')",
                    words_all.join(" ")
                ))
            })?;
        if let Some(color_count) = color_count {
            filter.color_count = Some(color_count);
        }
        Ok(filter)
    } else {
        let mut filter = parse_object_filter(&filter_tokens, false).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported search filter in search-library sentence (clause: '{}')",
                words_all.join(" ")
            ))
        })?;
        if let Some(color_count) = color_count {
            filter.color_count = Some(color_count);
        }
        Ok(filter)
    }
}

pub(crate) fn split_search_named_item_filters_lexed(
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
    subject_starts_effect_lexed: fn(&[OwnedLexToken]) -> bool,
    parse_leading_effects_lexed: fn(&[OwnedLexToken]) -> Result<Vec<EffectAst>, CardTextError>,
) -> Result<SearchLibraryLeadingPrelude<'a>, CardTextError> {
    if subject_tokens.is_empty() || !subject_starts_effect_lexed(subject_tokens) {
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
        parse_leading_effects_lexed(&leading_tokens)?
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
    if matches!(
        token_word_refs(subject_tokens).as_slice(),
        ["each", "of", "them"]
    ) {
        return Ok(None);
    }

    let mut filter_tokens =
        if let Some(rest) = primitives::words_match_prefix(subject_tokens, &["for", "each"]) {
            rest
        } else if let Some(rest) = primitives::words_match_prefix(subject_tokens, &["each"]) {
            rest
        } else {
            return Ok(None);
        };

    if filter_tokens
        .first()
        .is_some_and(|token| token.is_word("of"))
    {
        filter_tokens = &filter_tokens[1..];
    }

    let filter_tokens = trim_commas(filter_tokens);
    if filter_tokens.is_empty() {
        return Ok(None);
    }

    if primitives::words_match_any_prefix(&filter_tokens, PLAYER_OR_OPPONENT_PREFIXES).is_some() {
        return Ok(None);
    }

    Ok(Some(parse_object_filter_lexed(&filter_tokens, false)?))
}

pub(crate) fn search_library_starts_with_search_verb_lexed(
    search_tokens: &[OwnedLexToken],
) -> bool {
    primitives::parse_prefix(search_tokens, search_library_search_verb).is_some()
}
