const CCA_LIFE_WORD: &str = "life";
const CCA_UNLESS_WORD: &str = "unless";
const CCA_OF_WORD: &str = "of";
const CCA_THOSE_WORD: &str = "those";
const CCA_ON_WORD: &str = "on";
const CCA_THAT_WORD: &str = "that";
const CCA_ATTACHED_WORD: &str = "attached";
const CCA_THE_WORD: &str = "the";
const CCA_CHOICE_WORD: &str = "choice";
const CCA_EITHER_WORD: &str = "either";
const CCA_TOP_WORD: &str = "top";
const CCA_OR_WORD: &str = "or";
const CCA_AND_WORD: &str = "and";
const CCA_INSTEAD_WORD: &str = "instead";
const CCA_BOTTOM_WORD: &str = "bottom";
const CCA_PUT_WORD: &str = "put";
const CCA_BATTLEFIELD_WORD: &str = "battlefield";
const CCA_FROM_WORD: &str = "from";
const CCA_INTO_WORD: &str = "into";
const CCA_ATTACKING_WORD: &str = "attacking";
const CCA_TAPPED_WORD: &str = "tapped";
const CCA_PERMANENT_WORD: &str = "permanent";
const CCA_STICKER_WORD: &str = "sticker";

const CCA_THE_GAME_WORDS: &[&str] = &["the", "game"];
const CCA_LIBRARY_WORDS: &[&str] = &["library"];
const CCA_ATTACKING_WORDS: &[&str] = &[CCA_ATTACKING_WORD];
const CCA_TAPPED_WORDS: &[&str] = &[CCA_TAPPED_WORD];
const CCA_PERMANENT_WORDS: &[&str] = &[CCA_PERMANENT_WORD];
const CCA_STICKER_WORDS: &[&str] = &[CCA_STICKER_WORD];
const CCA_DURATION_START_WORDS: &[&str] = &["during", "until"];
const CCA_IT_OR_THEM_WORDS: &[&str] = &["it", "them"];
const CCA_CARD_OR_CARDS_WORDS: &[&str] = &["card", "cards"];
const CCA_HAND_OR_HANDS_WORDS: &[&str] = &["hand", "hands"];
const CCA_GRAVEYARD_OR_GRAVEYARDS_WORDS: &[&str] = &["graveyard", "graveyards"];
const CCA_LIBRARY_OR_LIBRARIES_WORDS: &[&str] = &["library", "libraries"];
const CCA_HAND_DESTINATION_SHAPE: LexPattern<'static> = LexPattern::new(&[LexPattern::object(
    "zone",
    LexCaptureKind::OneOf(CCA_HAND_OR_HANDS_WORDS),
)]);
const CCA_GRAVEYARD_DESTINATION_SHAPE: LexPattern<'static> =
    LexPattern::new(&[LexPattern::object(
        "zone",
        LexCaptureKind::OneOf(CCA_GRAVEYARD_OR_GRAVEYARDS_WORDS),
    )]);
const CCA_THE_GAME_SHAPE: LexPattern<'static> = LexPattern::new(&[LexPattern::object(
    "game",
    LexCaptureKind::OneOfPhrase(&[CCA_THE_GAME_WORDS]),
)]);
const CCA_LIBRARY_WORD_SHAPE: LexPattern<'static> = LexPattern::new(&[LexPattern::object(
    "zone",
    LexCaptureKind::OneOf(CCA_LIBRARY_WORDS),
)]);
const CCA_REST_TARGET_SHAPE: LexPattern<'static> = LexPattern::new(&[LexPattern::object(
    "rest",
    LexCaptureKind::OneOfPhrase(CCA_REST_TARGET_PHRASES),
)]);
const CCA_ATTACKING_WORD_SHAPE: LexPattern<'static> = LexPattern::new(&[LexPattern::modifier(
    "attacking",
    LexCaptureKind::OneOf(CCA_ATTACKING_WORDS),
)]);
const CCA_TAPPED_WORD_SHAPE: LexPattern<'static> = LexPattern::new(&[LexPattern::modifier(
    "tapped",
    LexCaptureKind::OneOf(CCA_TAPPED_WORDS),
)]);
const CCA_PERMANENT_WORD_SHAPE: LexPattern<'static> = LexPattern::new(&[LexPattern::object(
    "permanent",
    LexCaptureKind::OneOf(CCA_PERMANENT_WORDS),
)]);
const CCA_STICKER_WORD_SHAPE: LexPattern<'static> = LexPattern::new(&[LexPattern::object(
    "sticker",
    LexCaptureKind::OneOf(CCA_STICKER_WORDS),
)]);
const CCA_FROM_COMMAND_ZONE_SHAPE: LexPattern<'static> =
    LexPattern::new(&[LexPattern::modifier(
        "source_zone",
        LexCaptureKind::OneOfPhrase(CCA_FROM_COMMAND_ZONE_PHRASES),
    )]);
const CCA_REST_TARGET_PHRASES: &[&[&str]] = &[&["the", "rest"], &["rest"]];
const CCA_FROM_COMMAND_ZONE_PHRASES: &[&[&str]] = &[
    &["from", "the", "command", "zone"],
    &["from", "command", "zone"],
];
const CCA_AND_OR_THEN_WORDS: &[&str] = &["and", "then"];
const CCA_COMMAND_ZONE_TAIL_PREFIX: &[&str] = &["command", "zone"];
const CCA_DESTINATION_IGNORED_WORDS: &[&str] = &["and", "tapped", "attacking", "face", "down"];
const CCA_ATTACHED_TO_PREFIX: &[&str] = &["attached", "to"];
const CCA_FACE_DOWN_PREFIX: &[&str] = &["face", "down"];
const CCA_FACE_DOWN_PHRASE: &[&str] = &["face", "down"];
const CCA_CONTROL_ACTION_WORDS: &[&str] = &["control", "controls"];
const CCA_YOU_CONTROLLER_PHRASES: &[&[&str]] = &[&["your"], &["you"]];
const CCA_OWNER_CONTROLLER_PHRASES: &[&[&str]] = &[
    &["its", "owners"],
    &["its", "owner"],
    &["his", "owners"],
    &["his", "owner"],
    &["her", "owners"],
    &["her", "owner"],
    &["their", "owners"],
    &["their", "owner"],
    &["that", "players"],
    &["that", "player"],
];
const CCA_UNDER_YOU_CONTROL_SHAPE: LexPattern<'static> = LexPattern::new(&[
    LexPattern::word("under"),
    LexPattern::subject(
        "controller",
        LexCaptureKind::OneOfPhrase(CCA_YOU_CONTROLLER_PHRASES),
    ),
    LexPattern::action("action", LexCaptureKind::OneOf(CCA_CONTROL_ACTION_WORDS)),
]);
const CCA_UNDER_OWNER_CONTROL_SHAPE: LexPattern<'static> = LexPattern::new(&[
    LexPattern::word("under"),
    LexPattern::subject(
        "controller",
        LexCaptureKind::OneOfPhrase(CCA_OWNER_CONTROLLER_PHRASES),
    ),
    LexPattern::action("action", LexCaptureKind::OneOf(CCA_CONTROL_ACTION_WORDS)),
]);
const CCA_ALL_OR_EACH_WORDS: &[&str] = &["all", "each"];
const CCA_OWNER_WORDS: &[&str] = &["owner", "owners", "owner's", "owners'"];
const CCA_PLAYER_WORDS: &[&str] = &["player", "players", "player's", "players'"];
const CCA_FOR_AS_LONG_AS_PHRASE: &[&str] = &["for", "as", "long", "as"];
const CCA_YOU_CONTROL_WORDS: &[&str] = &["you", "control"];
const CCA_SOURCE_REFERENCE_WORDS: &[&str] =
    &["this", "thiss", "source", "creature", "permanent", "saga"];
const CCA_DURING_NEXT_TURN_WORDS: &[&str] = &["during", "next", "turn"];
const CCA_UNTIL_END_NEXT_TURN_WORDS: &[&str] = &["until", "end", "next", "turn"];
const CCA_UNTIL_END_TURN_WORDS: &[&str] = &["until", "end", "turn"];
const CCA_YOU_PLAYER_REFERENCE_WORDS: &[&str] = &["you", "your"];
const CCA_YOUR_WORD: &str = "your";
const CCA_YOU_WORD: &str = "you";
const CCA_THAT_PLAYER_PREFIXES: &[&[&str]] =
    &[&["their"], &["that", "player"], &["that", "players"]];
const CCA_THAT_PLAYER_REFERENCE_PHRASES: &[&[&str]] =
    &[&["their"], &["that", "player"], &["that", "players"]];
const CCA_YOU_PLAYER_REFERENCE_SHAPE: LexPattern<'static> =
    LexPattern::new(&[LexPattern::subject(
        "player",
        LexCaptureKind::OneOf(CCA_YOU_PLAYER_REFERENCE_WORDS),
    )]);
const CCA_THAT_PLAYER_REFERENCE_SHAPE: LexPattern<'static> =
    LexPattern::new(&[LexPattern::subject(
        "player",
        LexCaptureKind::OneOfPhrase(CCA_THAT_PLAYER_REFERENCE_PHRASES),
    )]);
const CCA_FROM_AMONG_PREPOSITION_PHRASES: &[&[&str]] = &[&["from", "among"]];
const CCA_HAND_LOCATION_PHRASES: &[&[&str]] = &[&["hand"], &["hands"]];
const CCA_ANY_ORDER_PHRASES: &[&[&str]] = &[&["any", "order"]];
const CCA_BACK_IN_ANY_ORDER_SHAPE: LexPattern<'static> = LexPattern::new(&[
    LexPattern::object("object", LexCaptureKind::OneOf(CCA_IT_OR_THEM_WORDS)),
    LexPattern::word("back"),
    LexPattern::optional(&[LexPattern::word("in")]),
    LexPattern::modifier("order", LexCaptureKind::OneOfPhrase(CCA_ANY_ORDER_PHRASES)),
]);
const CCA_FROM_AMONG_HAND_SHAPE: LexPattern<'static> = LexPattern::new(&[
    LexPattern::action(
        "preposition",
        LexCaptureKind::OneOfPhrase(CCA_FROM_AMONG_PREPOSITION_PHRASES),
    ),
    LexPattern::modifier(
        "owner",
        LexCaptureKind::UntilAnyPhrase(CCA_HAND_LOCATION_PHRASES),
    ),
    LexPattern::object("zone", LexCaptureKind::OneOf(CCA_HAND_OR_HANDS_WORDS)),
]);
const CCA_EXILED_WORD_PHRASE: &[&str] = &["exiled"];
const CCA_INTO_WORD_PHRASE: &[&str] = &["into"];
const CCA_PUT_ALL_EXILED_CARDS_INTO_HAND_SHAPE: LexPattern<'static> = LexPattern::new(&[
    LexPattern::word("put"),
    LexPattern::amount("count", LexCaptureKind::OneOf(CCA_ALL_OR_EACH_WORDS)),
    LexPattern::modifier(
        "before_exiled",
        LexCaptureKind::UntilPhrase(CCA_EXILED_WORD_PHRASE),
    ),
    LexPattern::word("exiled"),
    LexPattern::object("card", LexCaptureKind::OneOf(CCA_CARD_OR_CARDS_WORDS)),
    LexPattern::modifier(
        "after_card",
        LexCaptureKind::UntilPhrase(CCA_INTO_WORD_PHRASE),
    ),
    LexPattern::word("into"),
    LexPattern::modifier(
        "destination_player",
        LexCaptureKind::UntilAnyPhrase(CCA_HAND_LOCATION_PHRASES),
    ),
    LexPattern::object("zone", LexCaptureKind::OneOf(CCA_HAND_OR_HANDS_WORDS)),
]);
const CCA_OF_THEM_TARGET_PHRASES: &[&[&str]] = &[&["of", "them"], &["them"]];
const CCA_OF_TAGGED_CARDS_TARGET_PHRASES: &[&[&str]] = &[
    &["of", "them"],
    &["them"],
    &["of", "those", "card"],
    &["of", "those", "cards"],
    &["those", "card"],
    &["those", "cards"],
];
const CCA_PUT_TAGGED_INTO_HAND_SHAPE: LexPattern<'static> = LexPattern::new(&[
    LexPattern::optional(&[LexPattern::word("put")]),
    LexPattern::object("object", LexCaptureKind::OneOf(CCA_IT_OR_THEM_WORDS)),
    LexPattern::word("into"),
    LexPattern::modifier(
        "destination_player",
        LexCaptureKind::UntilAnyPhrase(CCA_HAND_LOCATION_PHRASES),
    ),
    LexPattern::object("zone", LexCaptureKind::OneOf(CCA_HAND_OR_HANDS_WORDS)),
    LexPattern::tail("rest", LexCaptureKind::Rest),
]);
const CCA_PUT_COUNTED_THEM_INTO_HAND_SHAPE: LexPattern<'static> = LexPattern::new(&[
    LexPattern::optional(&[LexPattern::word("put")]),
    LexPattern::amount(
        "count",
        LexCaptureKind::UntilAnyPhrase(CCA_OF_THEM_TARGET_PHRASES),
    ),
    LexPattern::any_phrase(CCA_OF_THEM_TARGET_PHRASES),
    LexPattern::word("into"),
    LexPattern::modifier(
        "destination_player",
        LexCaptureKind::UntilAnyPhrase(CCA_HAND_LOCATION_PHRASES),
    ),
    LexPattern::object("zone", LexCaptureKind::OneOf(CCA_HAND_OR_HANDS_WORDS)),
    LexPattern::tail("rest", LexCaptureKind::Rest),
]);
const CCA_PUT_TAGGED_ON_TOP_LIBRARY_SHAPE: LexPattern<'static> = LexPattern::new(&[
    LexPattern::optional(&[LexPattern::word("put")]),
    LexPattern::amount(
        "count",
        LexCaptureKind::UntilAnyPhrase(CCA_OF_TAGGED_CARDS_TARGET_PHRASES),
    ),
    LexPattern::any_phrase(CCA_OF_TAGGED_CARDS_TARGET_PHRASES),
    LexPattern::word("on"),
    LexPattern::optional(&[LexPattern::word("the")]),
    LexPattern::word("top"),
    LexPattern::optional(&[LexPattern::word("of")]),
    LexPattern::modifier(
        "owner",
        LexCaptureKind::UntilAnyPhrase(CCA_LIBRARY_LOCATION_PHRASES),
    ),
    LexPattern::object(
        "zone",
        LexCaptureKind::OneOf(CCA_LIBRARY_OR_LIBRARIES_WORDS),
    ),
    LexPattern::tail("rest", LexCaptureKind::Rest),
]);
const CCA_REST_LIBRARY_PREPOSITION_WORDS: &[&str] = &["on", "into", "to"];
const CCA_REST_GRAVEYARD_PREPOSITION_WORDS: &[&str] = &["into", "to"];
const CCA_LIBRARY_LOCATION_PHRASES: &[&[&str]] = &[&["library"], &["libraries"]];
const CCA_GRAVEYARD_LOCATION_PHRASES: &[&[&str]] = &[&["graveyard"], &["graveyards"]];
const CCA_REST_BOTTOM_LIBRARY_SHAPE: LexPattern<'static> = LexPattern::new(&[
    LexPattern::any_word(CCA_AND_OR_THEN_WORDS),
    LexPattern::object("rest", LexCaptureKind::OneOfPhrase(CCA_REST_TARGET_PHRASES)),
    LexPattern::modifier(
        "preposition",
        LexCaptureKind::OneOf(CCA_REST_LIBRARY_PREPOSITION_WORDS),
    ),
    LexPattern::optional(&[LexPattern::word("the")]),
    LexPattern::word("bottom"),
    LexPattern::optional(&[LexPattern::word("of")]),
    LexPattern::modifier(
        "owner",
        LexCaptureKind::UntilAnyPhrase(CCA_LIBRARY_LOCATION_PHRASES),
    ),
    LexPattern::object(
        "zone",
        LexCaptureKind::OneOf(CCA_LIBRARY_OR_LIBRARIES_WORDS),
    ),
]);
const CCA_REST_GRAVEYARD_SHAPE: LexPattern<'static> = LexPattern::new(&[
    LexPattern::any_word(CCA_AND_OR_THEN_WORDS),
    LexPattern::object("rest", LexCaptureKind::OneOfPhrase(CCA_REST_TARGET_PHRASES)),
    LexPattern::modifier(
        "preposition",
        LexCaptureKind::OneOf(CCA_REST_GRAVEYARD_PREPOSITION_WORDS),
    ),
    LexPattern::modifier(
        "owner",
        LexCaptureKind::UntilAnyPhrase(CCA_GRAVEYARD_LOCATION_PHRASES),
    ),
    LexPattern::object(
        "zone",
        LexCaptureKind::OneOf(CCA_GRAVEYARD_OR_GRAVEYARDS_WORDS),
    ),
]);
const CCA_POWER_NUMBER_WORDS: &[&str] = &["power", "number"];
const CCA_ITS_SOURCE_STAT_PHRASES: &[&[&str]] = &[
    &["its", "power"],
    &["its", "toughness"],
    &["its", "mana", "value"],
];
const CCA_THEN_SHUFFLE_YOUR_GRAVEYARD_INTO_YOUR_PHRASE: &[&str] =
    &["then", "shuffle", "your", "graveyard", "into", "your"];
const CCA_FROM_IT_PHRASE: &[&str] = &["from", "it"];
const CCA_AMONG_THEM_WORDS: &[&str] = &["among", "them"];
const CCA_BACK_ANY_ORDER_WORDS: &[&str] = &["back", "any", "order"];
const CCA_REST_TOP_BOTTOM_LIBRARY_WORDS: &[&str] = &["rest", "bottom", "library"];
const CCA_OWNER_CONTROL_TAIL_PREFIXES: &[&[&str]] = &[
    &["its", "owners", "control"],
    &["its", "owner", "control"],
    &["their", "owners", "control"],
    &["their", "owner", "control"],
    &["that", "players", "control"],
    &["that", "player", "control"],
];

fn cca_token_is(token: &OwnedLexToken, expected: &str) -> bool {
    token.as_word().is_some_and(|word| word == expected)
}

fn cca_token_is_any(token: &OwnedLexToken, choices: &[&str]) -> bool {
    token.as_word().is_some_and(|word| choices.contains(&word))
}

fn cca_words_contain_all(words: &[&str], required: &[&str]) -> bool {
    required
        .iter()
        .all(|word| cca_words_contain_word(words, word))
}

fn cca_tokens_contain_all(tokens: &[OwnedLexToken], required: &[&str]) -> bool {
    let words = crate::runtime_backend::token_word_refs(tokens);
    cca_words_contain_all(&words, required)
}

fn cca_tokens_contain_any_word(tokens: &[OwnedLexToken], choices: &[&str]) -> bool {
    let words = crate::runtime_backend::token_word_refs(tokens);
    choices
        .iter()
        .any(|word| cca_words_contain_word(&words, word))
}

fn cca_tokens_contain_phrase(tokens: &[OwnedLexToken], phrase: &[&str]) -> bool {
    let words = crate::runtime_backend::token_word_refs(tokens);
    cca_words_contain_phrase(&words, phrase)
}

fn cca_tokens_contain_any_phrase(tokens: &[OwnedLexToken], phrases: &[&[&str]]) -> bool {
    let words = crate::runtime_backend::token_word_refs(tokens);
    cca_words_contain_any_phrase(&words, phrases)
}

fn cca_tokens_start_with_any(tokens: &[OwnedLexToken], prefixes: &[&[&str]]) -> bool {
    let words = TokenWordView::new(tokens).word_refs();
    let atoms = [LexPattern::object(
        "prefix",
        LexCaptureKind::OneOfPhrase(prefixes),
    )];
    LexPattern::new(&atoms)
        .match_prefix_word_refs(&words)
        .and_then(|matched| matched.capture_word_range("prefix"))
        .is_some()
}

fn cca_words_contain_word(words: &[&str], expected: &str) -> bool {
    let expected_words = [expected];
    let atoms = [LexPattern::object(
        "word",
        LexCaptureKind::OneOf(&expected_words),
    )];
    LexPattern::new(&atoms)
        .find_in_word_refs(words)
        .and_then(|matched| matched.capture_word_range("word"))
        .is_some()
}

fn cca_words_at_is(words: &[&str], idx: usize, expected: &str) -> bool {
    let Some(tail) = words.get(idx..) else {
        return false;
    };
    let expected_words = [expected];
    let atoms = [LexPattern::object(
        "word",
        LexCaptureKind::OneOf(&expected_words),
    )];
    LexPattern::new(&atoms)
        .match_prefix_word_refs(tail)
        .and_then(|matched| matched.capture_word_range("word"))
        .is_some()
}

fn cca_words_contain_phrase(words: &[&str], phrase: &[&str]) -> bool {
    let phrase_choices = [phrase];
    let atoms = [LexPattern::object(
        "phrase",
        LexCaptureKind::OneOfPhrase(&phrase_choices),
    )];
    LexPattern::new(&atoms)
        .find_in_word_refs(words)
        .and_then(|matched| matched.capture_word_range("phrase"))
        .is_some()
}

fn cca_words_contain_any_phrase(words: &[&str], phrases: &[&[&str]]) -> bool {
    let atoms = [LexPattern::object(
        "phrase",
        LexCaptureKind::OneOfPhrase(phrases),
    )];
    LexPattern::new(&atoms)
        .find_in_word_refs(words)
        .and_then(|matched| matched.capture_word_range("phrase"))
        .is_some()
}

fn cca_words_exact_pattern_matches(
    words: &[&str],
    pattern: LexPattern<'static>,
    capture: &str,
) -> bool {
    pattern
        .match_word_refs(words)
        .and_then(|matched| matched.capture_word_range(capture))
        .is_some()
}

fn cca_words_contain_pattern(words: &[&str], pattern: LexPattern<'static>, capture: &str) -> bool {
    pattern
        .find_in_word_refs(words)
        .and_then(|matched| matched.capture_word_range(capture))
        .is_some()
}

fn cca_tokens_contain_pattern(
    tokens: &[OwnedLexToken],
    pattern: LexPattern<'static>,
    capture: &str,
) -> bool {
    let words = crate::runtime_backend::token_word_refs(tokens);
    cca_words_contain_pattern(&words, pattern, capture)
}

fn cca_capture_tokens<'a>(
    tokens: &'a [OwnedLexToken],
    matched: &crate::runtime_backend::lex_patterns::LexPatternMatch<'_>,
    capture: &str,
) -> Option<&'a [OwnedLexToken]> {
    let range = matched.capture_word_range(capture)?;
    let start = token_index_for_word_index(tokens, range.start).unwrap_or(tokens.len());
    let end = token_index_for_word_index(tokens, range.end).unwrap_or(tokens.len());
    tokens.get(start..end)
}

fn cca_destination_zone_from_tokens(tokens: &[OwnedLexToken]) -> Option<Zone> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    if word_slice_contains_any_word(&words, CCA_HAND_OR_HANDS_WORDS) {
        Some(Zone::Hand)
    } else if word_slice_contains_any_word(&words, CCA_GRAVEYARD_OR_GRAVEYARDS_WORDS) {
        Some(Zone::Graveyard)
    } else {
        None
    }
}

struct CcaBattlefieldControllerTail {
    controller: ReturnControllerAst,
    consumed_words: usize,
}

fn cca_battlefield_controller_tail(
    tokens: &[OwnedLexToken],
) -> Option<CcaBattlefieldControllerTail> {
    let words = TokenWordView::new(tokens).word_refs();
    if let Some(matched) = CCA_UNDER_YOU_CONTROL_SHAPE.match_prefix_word_refs(&words) {
        return Some(CcaBattlefieldControllerTail {
            controller: ReturnControllerAst::You,
            consumed_words: matched.word_range.end,
        });
    }
    if let Some(matched) = CCA_UNDER_OWNER_CONTROL_SHAPE.match_prefix_word_refs(&words) {
        return Some(CcaBattlefieldControllerTail {
            controller: ReturnControllerAst::Owner,
            consumed_words: matched.word_range.end,
        });
    }
    None
}

fn cca_tokens_are_you_control_source_duration(tokens: &[OwnedLexToken]) -> bool {
    if cca_tokens_contain_all(tokens, CCA_YOU_CONTROL_WORDS)
        && cca_tokens_contain_any_word(tokens, CCA_SOURCE_REFERENCE_WORDS)
    {
        return true;
    }

    let words = crate::runtime_backend::token_word_refs(tokens);
    let Some(control_idx) = words.iter().position(|word| *word == "control") else {
        return false;
    };
    let source_words = &words[control_idx + 1..];
    !source_words.is_empty()
        && (this_source_surface_for_words(source_words).is_some()
            || source_reference_surface_for_words(source_words).is_some())
}

fn parse_put_choice_count_prefix(
    tokens: &[OwnedLexToken],
    clause_words: &[&str],
) -> Result<(ChoiceCount, usize), CardTextError> {
    parse_choice_count_token_prefix_consumed(tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing put count (clause: '{}')",
            clause_words.join(" ")
        ))
    })
}

fn parse_counted_card_target_prefix(
    target_tokens: &[OwnedLexToken],
) -> Result<Option<TargetAst>, CardTextError> {
    let Some((count, used)) = parse_choice_count_token_prefix_consumed(target_tokens) else {
        return Ok(None);
    };
    if !target_tokens
        .get(used)
        .is_some_and(|token| cca_token_is_any(token, CCA_CARD_OR_CARDS_WORDS))
    {
        return Ok(None);
    }
    let inner = parse_target_phrase(&target_tokens[used..])?;
    Ok(Some(TargetAst::WithCount(Box::new(inner), count)))
}

fn cca_destination_player_from_tokens(tokens: &[OwnedLexToken], fallback: PlayerAst) -> PlayerAst {
    let words = crate::runtime_backend::token_word_refs(tokens);
    if word_slice_contains_word(&words, CCA_YOUR_WORD)
        || word_slice_contains_word(&words, CCA_YOU_WORD)
    {
        PlayerAst::You
    } else if word_slice_starts_with_any(&words, CCA_THAT_PLAYER_PREFIXES) {
        PlayerAst::That
    } else {
        fallback
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CcaRestDestination {
    BottomOfLibrary,
    Graveyard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CcaPutTaggedIntoHandShape {
    count: Option<ChoiceCount>,
    rest_destination: Option<CcaRestDestination>,
}

fn cca_rest_destination_from_tokens(tokens: &[OwnedLexToken]) -> Option<CcaRestDestination> {
    let clause_words = TokenWordView::new(tokens).word_refs();
    let words = clause_words.as_slice();
    if cca_words_contain_all(&clause_words, CCA_REST_TOP_BOTTOM_LIBRARY_WORDS)
        && CCA_REST_BOTTOM_LIBRARY_SHAPE
            .find_in_word_refs(words)
            .is_some()
    {
        return Some(CcaRestDestination::BottomOfLibrary);
    }
    if CCA_REST_GRAVEYARD_SHAPE.find_in_word_refs(words).is_some() {
        return Some(CcaRestDestination::Graveyard);
    }
    None
}

fn parse_cca_put_tagged_into_hand_shape(
    tokens: &[OwnedLexToken],
    clause_words: &[&str],
) -> Result<Option<CcaPutTaggedIntoHandShape>, CardTextError> {
    let words = TokenWordView::new(tokens).word_refs();
    if let Some(matched) = CCA_PUT_COUNTED_THEM_INTO_HAND_SHAPE.match_prefix_word_refs(&words) {
        let count_tokens = cca_capture_tokens(tokens, &matched, "count")
            .ok_or_else(|| CardTextError::ParseError("missing put count capture".to_string()))?;
        if count_tokens.is_empty() {
            return Ok(Some(CcaPutTaggedIntoHandShape {
                count: None,
                rest_destination: cca_rest_destination_from_tokens(tokens),
            }));
        }
        let (count, used) = parse_put_choice_count_prefix(count_tokens, clause_words)?;
        if used != count_tokens.len() {
            return Err(CardTextError::ParseError(format!(
                "unsupported put count prefix (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        return Ok(Some(CcaPutTaggedIntoHandShape {
            count: Some(count),
            rest_destination: cca_rest_destination_from_tokens(tokens),
        }));
    }

    if CCA_PUT_TAGGED_INTO_HAND_SHAPE
        .match_prefix_word_refs(&words)
        .is_some()
    {
        return Ok(Some(CcaPutTaggedIntoHandShape {
            count: None,
            rest_destination: cca_rest_destination_from_tokens(tokens),
        }));
    }

    Ok(None)
}

fn parse_cca_put_tagged_on_top_library_count(
    tokens: &[OwnedLexToken],
    clause_words: &[&str],
) -> Result<Option<ChoiceCount>, CardTextError> {
    if cca_rest_destination_from_tokens(tokens) != Some(CcaRestDestination::BottomOfLibrary) {
        return Ok(None);
    }

    let words = TokenWordView::new(tokens).word_refs();
    let Some(matched) = CCA_PUT_TAGGED_ON_TOP_LIBRARY_SHAPE.match_prefix_word_refs(&words) else {
        return Ok(None);
    };
    let count_tokens = cca_capture_tokens(tokens, &matched, "count")
        .ok_or_else(|| CardTextError::ParseError("missing put count capture".to_string()))?;
    let (choice_count, used) = parse_put_choice_count_prefix(count_tokens, clause_words)?;
    if used != count_tokens.len() {
        return Err(CardTextError::ParseError(format!(
            "unsupported library rearrange put count (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    Ok(Some(choice_count))
}

pub(crate) fn parse_lose_life(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);

    let clause_words = crate::runtime_backend::token_word_refs(tokens);

    if clause_words.len() == 2
        && clause_words[1] == CCA_LIFE_WORD
        && let Some((amount, _)) = parse_number(tokens)
    {
        return Ok(subject_verb_player_resource_effect(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::LoseLife {
                amount: Value::Fixed(amount as i32),
            },
        ));
    }
    if let Some(mut amount) = parse_life_equal_to_value(tokens)? {
        if matches!(player, PlayerAst::ItsController | PlayerAst::ItsOwner)
            && cca_tokens_contain_any_phrase(tokens, CCA_ITS_SOURCE_STAT_PHRASES)
        {
            amount = remap_source_stat_value_to_it(amount);
        }
        return Ok(subject_verb_player_resource_effect(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::LoseLife { amount },
        ));
    }
    if word_slice_eq(&clause_words, CCA_THE_GAME_WORDS) {
        return Ok(EffectAst::subject_verb_lose_game(player));
    }

    if let Some(amount) = parse_half_life_value(tokens, player) {
        return Ok(subject_verb_player_resource_effect(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::LoseLife { amount },
        ));
    }

    let (mut amount, used) = parse_life_amount(tokens, "life loss")?;

    let rest = &tokens[used..];
    validate_life_keyword(rest)?;
    let trailing = trim_commas(&rest[1..]);
    if !trailing.is_empty() {
        if let Some(resolved) = parse_life_amount_from_trailing(&amount, &trailing)? {
            amount = resolved;
            return Ok(subject_verb_player_resource_effect(
                SubjectVerbRoleAst::AffectedPlayer,
                player,
                SubjectVerbActionAst::LoseLife { amount },
            ));
        }
        let base_effect = subject_verb_player_resource_effect(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::LoseLife { amount },
        );
        if let Some(predicate) = parse_trailing_if_predicate_lexed(&trailing) {
            return Ok(EffectAst::Conditional {
                predicate,
                if_true: vec![base_effect],
                if_false: Vec::new(),
            });
        }
        if trailing
            .first()
            .is_some_and(|token| cca_token_is(token, CCA_UNLESS_WORD))
        {
            let mut unless_as_if_tokens = Vec::with_capacity(trailing.len() + 1);
            unless_as_if_tokens.push(OwnedLexToken::word("if".to_string(), TextSpan::synthetic()));
            unless_as_if_tokens.extend_from_slice(&trailing[1..]);
            if let Some(predicate) = parse_trailing_if_predicate_lexed(&unless_as_if_tokens) {
                return Ok(EffectAst::Conditional {
                    predicate,
                    if_true: Vec::new(),
                    if_false: vec![base_effect],
                });
            }
        }
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing life-loss clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    Ok(subject_verb_player_resource_effect(
        SubjectVerbRoleAst::AffectedPlayer,
        player,
        SubjectVerbActionAst::LoseLife { amount },
    ))
}

pub(crate) fn parse_gain_life(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);

    if let Some(mut amount) = parse_life_equal_to_value(tokens)? {
        if matches!(player, PlayerAst::ItsController | PlayerAst::ItsOwner)
            && cca_tokens_contain_any_phrase(tokens, CCA_ITS_SOURCE_STAT_PHRASES)
        {
            amount = remap_source_stat_value_to_it(amount);
        }
        return Ok(subject_verb_player_resource_effect(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::GainLife { amount },
        ));
    }

    let (mut amount, used) = parse_life_amount(tokens, "life gain")?;

    let rest = &tokens[used..];
    validate_life_keyword(rest)?;
    let trailing = trim_commas(&rest[1..]);
    if !trailing.is_empty() {
        if cca_tokens_contain_phrase(&trailing, CCA_THEN_SHUFFLE_YOUR_GRAVEYARD_INTO_YOUR_PHRASE)
            && cca_tokens_contain_pattern(&trailing, CCA_LIBRARY_WORD_SHAPE, "zone")
        {
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing life-gain shuffle-graveyard clause (clause: '{}')",
                crate::runtime_backend::token_word_refs(tokens).join(" ")
            )));
        }
        if let Some(resolved) = parse_life_amount_from_trailing(&amount, &trailing)? {
            amount = resolved;
            return Ok(subject_verb_player_resource_effect(
                SubjectVerbRoleAst::AffectedPlayer,
                player,
                SubjectVerbActionAst::GainLife { amount },
            ));
        }
        let base_effect = subject_verb_player_resource_effect(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::GainLife { amount },
        );
        if let Some(predicate) = parse_trailing_if_predicate_lexed(&trailing) {
            return Ok(EffectAst::Conditional {
                predicate,
                if_true: vec![base_effect],
                if_false: Vec::new(),
            });
        }
        if trailing
            .first()
            .is_some_and(|token| cca_token_is(token, CCA_UNLESS_WORD))
        {
            let mut unless_as_if_tokens = Vec::with_capacity(trailing.len() + 1);
            unless_as_if_tokens.push(OwnedLexToken::word("if".to_string(), TextSpan::synthetic()));
            unless_as_if_tokens.extend_from_slice(&trailing[1..]);
            if let Some(predicate) = parse_trailing_if_predicate_lexed(&unless_as_if_tokens) {
                return Ok(EffectAst::Conditional {
                    predicate,
                    if_true: Vec::new(),
                    if_false: vec![base_effect],
                });
            }
        }
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing life-gain clause (clause: '{}')",
            crate::runtime_backend::token_word_refs(tokens).join(" ")
        )));
    }

    Ok(subject_verb_player_resource_effect(
        SubjectVerbRoleAst::AffectedPlayer,
        player,
        SubjectVerbActionAst::GainLife { amount },
    ))
}

pub(crate) fn parse_gain_control(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    let has_dynamic_power_bound = cca_tokens_contain_all(tokens, CCA_POWER_NUMBER_WORDS)
        && cca_tokens_contain_phrase(tokens, CCA_YOU_CONTROL_WORDS);
    if has_dynamic_power_bound {
        return Err(CardTextError::ParseError(format!(
            "unsupported dynamic power-bound control clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let mut idx = 0;
    if token_slice_at_is(tokens, idx, "control") {
        idx += 1;
    } else {
        return Err(CardTextError::ParseError(
            "missing control keyword".to_string(),
        ));
    }

    if token_slice_at_is(tokens, idx, "of") {
        idx += 1;
    }

    let duration_idx = find_index(&tokens[idx..], |token: &OwnedLexToken| {
        cca_token_is_any(token, CCA_DURATION_START_WORDS)
    })
    .map(|offset| idx + offset)
    .or_else(|| {
        find_window_by(&tokens[idx..], 4, |window: &[OwnedLexToken]| {
            token_slice_starts_with(window, &["for", "as", "long", "as"])
        })
        .map(|offset| idx + offset)
    });

    let target_tokens = if let Some(dur_idx) = duration_idx {
        &tokens[idx..dur_idx]
    } else {
        &tokens[idx..]
    };
    let invalid_conditional_error = || {
        CardTextError::ParseError(format!(
            "unsupported conditional gain-control clause (clause: '{}')",
            clause_words.join(" ")
        ))
    };
    let (target_ast, trailing_predicate, is_unless) =
        if let Some(spec) = split_trailing_if_clause_lexed(target_tokens) {
            (
                parse_target_phrase(spec.leading_tokens)?,
                Some(spec.predicate),
                false,
            )
        } else if crate::runtime_backend::lexer::contains_token_word(target_tokens, "if") {
            return Err(invalid_conditional_error());
        } else if let Some(spec) = split_trailing_unless_clause_lexed(target_tokens) {
            (
                parse_target_phrase(spec.leading_tokens)?,
                Some(spec.predicate),
                true,
            )
        } else if crate::runtime_backend::lexer::contains_token_word(target_tokens, "unless") {
            return Err(invalid_conditional_error());
        } else {
            (parse_target_phrase(target_tokens)?, None, false)
        };
    let duration_tokens = duration_idx
        .map(|dur_idx| &tokens[dur_idx..])
        .unwrap_or(&[]);
    let duration = parse_control_duration(duration_tokens)?;
    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);
    let base_effect = match target_ast {
        TargetAst::Player(filter, _) => {
            if matches!(duration, ControlDurationAst::UntilYourNextTurnEnd) {
                return Err(CardTextError::ParseError(
                    "unsupported player-control duration until the end of your next turn"
                        .to_string(),
                ));
            }
            EffectAst::subject_verb_control_player(
                player,
                PlayerFilter::Target(Box::new(filter)),
                duration,
            )
        }
        _ => {
            let until = match duration {
                ControlDurationAst::UntilEndOfTurn => Until::EndOfTurn,
                ControlDurationAst::UntilYourNextTurnEnd => Until::YourNextTurnEnd,
                ControlDurationAst::Forever => Until::Forever,
                ControlDurationAst::AsLongAsYouControlSource => Until::YouStopControllingThis,
                ControlDurationAst::DuringNextTurn => {
                    return Err(CardTextError::ParseError(
                        "unsupported control duration for permanents".to_string(),
                    ));
                }
            };
            EffectAst::subject_verb_gain_control(player, target_ast, until)
        }
    };

    if let Some(predicate) = trailing_predicate {
        return Ok(if is_unless {
            EffectAst::Conditional {
                predicate,
                if_true: Vec::new(),
                if_false: vec![base_effect],
            }
        } else {
            EffectAst::Conditional {
                predicate,
                if_true: vec![base_effect],
                if_false: Vec::new(),
            }
        });
    }

    Ok(base_effect)
}

pub(crate) fn parse_control_duration(
    tokens: &[OwnedLexToken],
) -> Result<ControlDurationAst, CardTextError> {
    if tokens.is_empty() {
        return Ok(ControlDurationAst::Forever);
    }

    if cca_tokens_contain_phrase(tokens, CCA_FOR_AS_LONG_AS_PHRASE)
        && cca_tokens_are_you_control_source_duration(tokens)
    {
        return Ok(ControlDurationAst::AsLongAsYouControlSource);
    }

    if cca_tokens_contain_all(tokens, CCA_DURING_NEXT_TURN_WORDS) {
        return Ok(ControlDurationAst::DuringNextTurn);
    }

    if cca_tokens_contain_all(tokens, CCA_UNTIL_END_NEXT_TURN_WORDS) {
        return Ok(ControlDurationAst::UntilYourNextTurnEnd);
    }
    if cca_tokens_contain_all(tokens, CCA_UNTIL_END_TURN_WORDS) {
        return Ok(ControlDurationAst::UntilEndOfTurn);
    }

    Err(CardTextError::ParseError(
        "unsupported control duration".to_string(),
    ))
}

pub(crate) fn parse_put_into_hand(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    fn parse_put_into_hand_delayed_timing(
        tokens: &[OwnedLexToken],
    ) -> Option<DelayedReturnTimingAst> {
        let hand_idx = rfind_index(tokens, |token: &OwnedLexToken| {
            cca_token_is_any(token, CCA_HAND_OR_HANDS_WORDS)
        })?;
        let tail_tokens = trim_commas(&tokens[hand_idx + 1..]);
        let tail_words = crate::runtime_backend::token_word_refs(&tail_tokens);
        parse_delayed_return_timing_words(&tail_words)
    }

    fn force_object_targeting(target: TargetAst, span: TextSpan) -> TargetAst {
        match target {
            TargetAst::Object(filter, explicit_span, fixed_span) => {
                TargetAst::Object(filter, explicit_span.or(Some(span)), fixed_span)
            }
            TargetAst::WithCount(inner, count) => {
                TargetAst::WithCount(Box::new(force_object_targeting(*inner, span)), count)
            }
            other => other,
        }
    }

    fn expand_graveyard_or_hand_disjunction(
        mut target: TargetAst,
        target_tokens: &[OwnedLexToken],
    ) -> TargetAst {
        let target_words = crate::runtime_backend::token_word_refs(target_tokens);
        let has_graveyard =
            word_slice_contains_any_word(&target_words, CCA_GRAVEYARD_OR_GRAVEYARDS_WORDS);
        let has_hand = word_slice_contains_any_word(&target_words, CCA_HAND_OR_HANDS_WORDS);
        if !(has_graveyard && has_hand) {
            return target;
        }

        fn apply(filter: &ObjectFilter) -> ObjectFilter {
            let mut graveyard = filter.clone();
            graveyard.any_of.clear();
            graveyard.zone = Some(Zone::Graveyard);

            let mut hand = filter.clone();
            hand.any_of.clear();
            hand.zone = Some(Zone::Hand);

            let mut disjunction = ObjectFilter::default();
            disjunction.any_of = vec![graveyard, hand];
            disjunction
        }

        match &mut target {
            TargetAst::Object(filter, _, _) => {
                *filter = apply(filter);
            }
            TargetAst::WithCount(inner, _) => {
                if let TargetAst::Object(filter, _, _) = inner.as_mut() {
                    *filter = apply(filter);
                }
            }
            _ => {}
        }

        target
    }

    fn apply_source_zone_constraint(target: &mut TargetAst, zone: Zone) {
        match target {
            TargetAst::Source(span) => {
                *target = TargetAst::Object(ObjectFilter::source().in_zone(zone), *span, None);
            }
            TargetAst::Object(filter, _, _) => {
                filter.zone = Some(zone);
            }
            TargetAst::WithCount(inner, _) => apply_source_zone_constraint(inner, zone),
            _ => {}
        }
    }

    fn is_top_or_bottom_choice_destination(tokens: &[OwnedLexToken]) -> bool {
        let words = crate::runtime_backend::token_word_refs(tokens);
        let mut idx = 0usize;

        match words.get(idx).copied() {
            Some("their" | "his" | "her" | "your") => {
                idx += 1;
            }
            Some("its") => {
                idx += 1;
                if words
                    .get(idx)
                    .copied()
                    .is_some_and(|word| CCA_OWNER_WORDS.contains(&word))
                {
                    idx += 1;
                }
            }
            Some("that")
                if words
                    .get(idx + 1)
                    .copied()
                    .is_some_and(|word| CCA_PLAYER_WORDS.contains(&word)) =>
            {
                idx += 2;
            }
            Some(word) if CCA_OWNER_WORDS.contains(&word) => {
                idx += 1;
            }
            _ => {}
        }

        if !cca_words_at_is(&words, idx, CCA_CHOICE_WORD) {
            return false;
        }
        idx += 1;
        if !cca_words_at_is(&words, idx, CCA_OF_WORD) {
            return false;
        }
        idx += 1;
        if cca_words_at_is(&words, idx, CCA_EITHER_WORD) {
            idx += 1;
        }
        if cca_words_at_is(&words, idx, CCA_THE_WORD) {
            idx += 1;
        }

        let top_or_bottom = cca_words_at_is(&words, idx, CCA_TOP_WORD)
            && cca_words_at_is(&words, idx + 1, CCA_OR_WORD)
            && cca_words_at_is(&words, idx + 2, CCA_BOTTOM_WORD);
        let bottom_or_top = cca_words_at_is(&words, idx, CCA_BOTTOM_WORD)
            && cca_words_at_is(&words, idx + 1, CCA_OR_WORD)
            && cca_words_at_is(&words, idx + 2, CCA_TOP_WORD);
        if !(top_or_bottom || bottom_or_top) {
            return false;
        }
        idx += 3;
        if !cca_words_at_is(&words, idx, CCA_OF_WORD) {
            return false;
        }
        words[idx + 1..]
            .iter()
            .any(|word| CCA_LIBRARY_OR_LIBRARIES_WORDS.contains(word))
    }

    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);

    let clause_words = crate::runtime_backend::token_word_refs(tokens);

    fn parse_counted_those_cards_target(tokens: &[OwnedLexToken]) -> Option<u32> {
        let tokens = trim_commas(tokens);
        let words = crate::runtime_backend::token_word_refs(&tokens);
        if !cca_words_at_is(&words, 0, CCA_PUT_WORD) {
            return None;
        }

        let count_tokens = &tokens[1..];
        let (count, used) = parse_number(count_tokens)?;
        let mut idx = used;
        if count_tokens
            .get(idx)
            .is_some_and(|token: &OwnedLexToken| cca_token_is(token, CCA_OF_WORD))
        {
            idx += 1;
        }
        if !count_tokens
            .get(idx)
            .is_some_and(|token: &OwnedLexToken| cca_token_is(token, CCA_THOSE_WORD))
        {
            return None;
        }
        idx += 1;
        if !count_tokens
            .get(idx)
            .is_some_and(|token: &OwnedLexToken| cca_token_is_any(token, CCA_CARD_OR_CARDS_WORDS))
        {
            return None;
        }
        idx += 1;

        if idx != count_tokens.len() {
            return None;
        }
        Some(count as u32)
    }

    // "Put them/it back in any order." (typically after looking at the top cards of a library).
    if cca_words_contain_all(&clause_words, CCA_BACK_ANY_ORDER_WORDS)
        && word_slice_contains_any_word(&clause_words, CCA_IT_OR_THEM_WORDS)
    {
        return Ok(EffectAst::subject_verb_reorder_top_of_library(
            TagKey::from(IT_TAG),
        ));
    }

    let from_among_words = TokenWordView::new(tokens).word_refs();
    if CCA_FROM_AMONG_HAND_SHAPE
        .find_in_word_refs(&from_among_words)
        .is_some()
    {
        return Ok(EffectAst::subject_verb_put_some_into_hand_rest_into_graveyard(player, 1));
    }

    let put_all_exiled_words = TokenWordView::new(tokens).word_refs();
    if CCA_PUT_ALL_EXILED_CARDS_INTO_HAND_SHAPE
        .match_prefix_word_refs(&put_all_exiled_words)
        .is_some()
        && let Some(into_idx) = find_index(tokens, |token| {
            token.as_word().is_some_and(|word| word == CCA_INTO_WORD)
        })
    {
        let filter_tokens = trim_commas(&tokens[1..into_idx]);
        let filter = parse_object_filter(&filter_tokens, false)?;
        return Ok(wrap_return_with_delayed_timing(
            EffectAst::subject_verb_return_all_to_hand(filter),
            parse_put_into_hand_delayed_timing(tokens),
        ));
    }

    // "Put one of those cards on top of your library and the rest on the bottom of your library"
    if let Some(choice_count) = parse_cca_put_tagged_on_top_library_count(tokens, &clause_words)? {
        let library_owner = cca_destination_player_from_tokens(tokens, player);

        return Ok(EffectAst::subject_verb_rearrange_looked_cards_in_library(
            library_owner,
            TagKey::from(IT_TAG),
            choice_count,
        ));
    }

    if let Some(put_shape) = parse_cca_put_tagged_into_hand_shape(tokens, &clause_words)? {
        // "Put N of them into your hand and the rest on the bottom of your library in any order."
        if put_shape.rest_destination == Some(CcaRestDestination::BottomOfLibrary)
            && let Some(choice_count) = put_shape.count
        {
            let dest_player = cca_destination_player_from_tokens(tokens, player);

            return Ok(
                EffectAst::subject_verb_put_some_into_hand_rest_on_bottom_of_library_with_count(
                    dest_player,
                    choice_count,
                ),
            );
        }

        // "Put N of them into your hand and the rest into your graveyard."
        if put_shape.rest_destination == Some(CcaRestDestination::Graveyard)
            && let Some(choice_count) = put_shape.count
        {
            // The chooser is typically the player whose hand is referenced.
            let dest_player = cca_destination_player_from_tokens(tokens, player);

            return Ok(
                EffectAst::subject_verb_put_some_into_hand_rest_into_graveyard_with_count(
                    dest_player,
                    choice_count,
                ),
            );
        }

        let effect = EffectAst::subject_verb_put_into_hand(
            player,
            ObjectRefAst::Tagged(TagKey::from(IT_TAG)),
        );
        return Ok(wrap_return_with_delayed_timing(
            effect,
            parse_put_into_hand_delayed_timing(tokens),
        ));
    }

    // Support destination-first wording:
    // "Put onto the battlefield under your control all creature cards ..."
    if crate::runtime_backend::lexer::token_slice_first_is(tokens, "onto") {
        let mut idx = 1usize;
        while tokens
            .get(idx)
            .and_then(OwnedLexToken::as_word)
            .is_some_and(is_article)
        {
            idx += 1;
        }
        if !tokens
            .get(idx)
            .is_some_and(|token| cca_token_is(token, CCA_BATTLEFIELD_WORD))
        {
            return Err(CardTextError::ParseError(format!(
                "unsupported put destination after 'onto' (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        idx += 1;

        let mut battlefield_tapped = false;
        if token_slice_at_is(tokens, idx, "tapped") {
            battlefield_tapped = true;
            idx += 1;
        }

        let mut battlefield_face_down = false;
        if token_slice_starts_with(&tokens[idx..], CCA_FACE_DOWN_PREFIX) {
            battlefield_face_down = true;
            idx += 2;
        }

        let mut battlefield_controller = ReturnControllerAst::Preserve;
        if token_slice_at_is(tokens, idx, "under") {
            let controller_tokens = &tokens[idx..];
            if let Some(parsed) = cca_battlefield_controller_tail(controller_tokens) {
                battlefield_controller = parsed.controller;
                idx += parsed.consumed_words;
            }
        }

        let target_tokens = trim_commas(&tokens[idx..]);
        if target_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing target before 'onto' (clause: '{}')",
                clause_words.join(" ")
            )));
        }

        if crate::runtime_backend::lexer::token_slice_first_is(&target_tokens, "attached")
            && crate::runtime_backend::lexer::token_slice_at_is(&target_tokens, 1, "to")
        {
            let after_to = &target_tokens[2..];
            if after_to.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "missing attachment target after 'attached to' (clause: '{}')",
                    clause_words.join(" ")
                )));
            }

            let attachment_target_len =
                if crate::runtime_backend::lexer::token_slice_first_is(after_to, "it") {
                    1usize
                } else if after_to.len() >= 2
                    && cca_token_is(&after_to[0], CCA_THAT_WORD)
                    && after_to[1].as_word().is_some_and(|word| {
                        matches!(
                            word,
                            "creature" | "permanent" | "object" | "aura" | "equipment"
                        )
                    })
                {
                    2usize
                } else {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported attachment target after 'attached to' (clause: '{}')",
                        clause_words.join(" ")
                    )));
                };

            let attachment_target = parse_target_phrase(&after_to[..attachment_target_len])?;
            let object_tokens = trim_commas(&after_to[attachment_target_len..]);
            if object_tokens.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "missing object after attachment target (clause: '{}')",
                    clause_words.join(" ")
                )));
            }

            let mut object_target = parse_target_phrase(&object_tokens)?;
            object_target = expand_graveyard_or_hand_disjunction(object_target, &object_tokens);
            object_target = force_object_targeting(object_target, tokens[0].span());

            return Ok(EffectAst::subject_verb_move_to_zone(
                object_target,
                Zone::Battlefield,
                false,
                battlefield_controller,
                battlefield_tapped,
                Some(attachment_target),
            ));
        }

        if !target_tokens
            .first()
            .is_some_and(|token| cca_token_is(token, CCA_ATTACHED_WORD))
        {
            if target_tokens
                .first()
                .is_some_and(|token| cca_token_is_any(token, CCA_ALL_OR_EACH_WORDS))
            {
                let filter = parse_object_filter(&target_tokens[1..], false)?;
                return Ok(EffectAst::subject_verb_return_all_to_battlefield(
                    filter,
                    battlefield_tapped,
                    battlefield_face_down,
                    battlefield_controller,
                ));
            }
            let mut rewritten = target_tokens;
            rewritten.push(OwnedLexToken::word("onto".to_string(), tokens[0].span()));
            rewritten.extend_from_slice(&tokens[1..idx]);
            return parse_put_into_hand(&rewritten, subject);
        }
    }

    if let Some(on_idx) = find_index(tokens, |token| cca_token_is(token, CCA_ON_WORD))
        && is_top_or_bottom_choice_destination(&tokens[on_idx + 1..])
    {
        let target_tokens = trim_commas(&tokens[..on_idx]);
        if target_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing target before top-or-bottom library choice (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let target = if let Some(target) = parse_counted_card_target_prefix(&target_tokens)? {
            target
        } else {
            parse_target_phrase(&target_tokens)?
        };
        return Ok(EffectAst::subject_verb_move_to_library_top_or_bottom_choice(target));
    }

    if let Some((target_slice, after_on_top_of)) =
        super::super::grammar::primitives::split_lexed_once_on_separator(tokens, || {
            use winnow::Parser as _;
            super::super::grammar::primitives::phrase(&["on", "top", "of"]).void()
        })
    {
        let target_tokens = trim_commas(target_slice);
        if target_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing target before 'on top of' (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        if !cca_tokens_contain_pattern(after_on_top_of, CCA_LIBRARY_WORD_SHAPE, "zone") {
            return Err(CardTextError::ParseError(format!(
                "unsupported put destination after 'on top of' (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let target = if let Some(target) = parse_counted_card_target_prefix(&target_tokens)? {
            target
        } else {
            parse_target_phrase(&target_tokens)?
        };
        return Ok(EffectAst::subject_verb_move_to_zone(
            target,
            Zone::Library,
            true,
            ReturnControllerAst::Preserve,
            false,
            None,
        ));
    }

    if let Some(on_idx) = find_index(tokens, |token| cca_token_is(token, CCA_ON_WORD)) {
        let mut bottom_idx = on_idx + 1;
        if tokens
            .get(bottom_idx)
            .is_some_and(|token| cca_token_is(token, CCA_THE_WORD))
        {
            bottom_idx += 1;
        }
        if tokens
            .get(bottom_idx)
            .is_some_and(|token| cca_token_is(token, CCA_BOTTOM_WORD))
            && tokens
                .get(bottom_idx + 1)
                .is_some_and(|token| cca_token_is(token, CCA_OF_WORD))
        {
            let target_tokens = trim_commas(&tokens[..on_idx]);
            if target_tokens.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "missing target before 'on bottom of' (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            if !cca_tokens_contain_pattern(
                &tokens[bottom_idx + 2..],
                CCA_LIBRARY_WORD_SHAPE,
                "zone",
            ) {
                return Err(CardTextError::ParseError(format!(
                    "unsupported put destination after 'on bottom of' (clause: '{}')",
                    clause_words.join(" ")
                )));
            }

            let target_words = crate::runtime_backend::token_word_refs(&target_tokens);
            let is_rest_target =
                cca_words_exact_pattern_matches(&target_words, CCA_REST_TARGET_SHAPE, "rest");
            if is_rest_target {
                return Ok(EffectAst::subject_verb_put_rest_on_bottom_of_library());
            }

            let target = if let Some(target) = parse_counted_card_target_prefix(&target_tokens)? {
                target
            } else {
                parse_target_phrase(&target_tokens)?
            };

            return Ok(EffectAst::subject_verb_move_to_zone(
                target,
                Zone::Library,
                false,
                ReturnControllerAst::Preserve,
                false,
                None,
            ));
        }
    }

    if let Some((target_slice, destination_tokens)) =
        super::super::grammar::primitives::split_lexed_once_on_separator(tokens, || {
            use winnow::Parser as _;
            super::super::grammar::primitives::kw("into").void()
        })
        && !target_slice
            .iter()
            .any(|token| token.as_word().is_some_and(|word| word == "onto"))
        && !target_slice
            .iter()
            .any(|token| token.as_word().is_some_and(|word| word == CCA_BATTLEFIELD_WORD))
    {
        let target_tokens = trim_commas(target_slice);
        if target_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing target before 'into' (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let zone = if let Some(zone) = cca_destination_zone_from_tokens(destination_tokens) {
            Some(zone)
        } else if let Some(position) = parse_library_nth_from_top_destination(destination_tokens) {
            let target = parse_target_phrase(&target_tokens)?;
            return Ok(EffectAst::subject_verb_move_to_library_nth_from_top(
                target, position,
            ));
        } else {
            None
        };

        if let Some(zone) = zone {
            let delayed_hand_timing = if zone == Zone::Hand {
                parse_put_into_hand_delayed_timing(tokens)
            } else {
                None
            };
            let target_words = crate::runtime_backend::token_word_refs(&target_tokens);
            if zone == Zone::Graveyard
                && cca_words_exact_pattern_matches(&target_words, CCA_REST_TARGET_SHAPE, "rest")
            {
                return Ok(EffectAst::subject_verb_move_to_zone(
                    TargetAst::Object(ObjectFilter::tagged(TagKey::from(IT_TAG)), None, None),
                    zone,
                    false,
                    ReturnControllerAst::Preserve,
                    false,
                    None,
                ));
            }

            if zone == Zone::Hand {
                if let Some(count) = parse_counted_those_cards_target(&target_tokens)
                    && cca_rest_destination_from_tokens(destination_tokens)
                        == Some(CcaRestDestination::Graveyard)
                {
                    let dest_player = cca_destination_player_from_tokens(tokens, player);

                    return Ok(
                        EffectAst::subject_verb_put_some_into_hand_rest_into_graveyard(
                            dest_player,
                            count,
                        ),
                    );
                }

                if matches!(
                    target_words.as_slice(),
                    ["it"] | ["them"] | ["that", "card"] | ["those", "card"] | ["those", "cards"]
                ) {
                    let effect = EffectAst::subject_verb_put_into_hand(
                        player,
                        ObjectRefAst::Tagged(TagKey::from(IT_TAG)),
                    );
                    return Ok(wrap_return_with_delayed_timing(effect, delayed_hand_timing));
                }
            }

            let target = parse_target_phrase(&target_tokens)?;
            let moves_all = target_words
                .first()
                .is_some_and(|word| matches!(*word, "all" | "each"));
            let effect = if moves_all {
                EffectAst::subject_verb_move_all_to_zone(
                    target,
                    zone,
                    false,
                    ReturnControllerAst::Preserve,
                    false,
                    None,
                )
            } else {
                EffectAst::subject_verb_move_to_zone(
                    target,
                    zone,
                    false,
                    ReturnControllerAst::Preserve,
                    false,
                    None,
                )
            };
            return Ok(if zone == Zone::Hand {
                wrap_return_with_delayed_timing(effect, delayed_hand_timing)
            } else {
                effect
            });
        }
    }

    if let Some((target_slice, dest_slice)) =
        super::super::grammar::primitives::split_lexed_once_on_separator(tokens, || {
            use winnow::Parser as _;
            super::super::grammar::primitives::kw("onto").void()
        })
    {
        let target_tokens = trim_commas(target_slice);
        if target_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing target before 'onto' (clause: '{}')",
                clause_words.join(" ")
            )));
        }

        let (destination_slice, trailing_predicate) =
            if let Some(spec) = split_trailing_if_clause_lexed(dest_slice) {
                (spec.leading_tokens, Some(spec.predicate))
            } else {
                (dest_slice, None)
            };

        let destination_tokens: Vec<OwnedLexToken> = destination_slice
            .iter()
            .filter(|token| !token.as_word().is_some_and(is_article))
            .cloned()
            .collect();
        if !destination_tokens
            .first()
            .is_some_and(|token| cca_token_is(token, CCA_BATTLEFIELD_WORD))
        {
            return Err(CardTextError::ParseError(format!(
                "unsupported put destination after 'onto' (clause: '{}')",
                clause_words.join(" ")
            )));
        }

        let mut destination_tail: Vec<OwnedLexToken> = destination_tokens[1..].to_vec();
        let battlefield_attacking =
            cca_tokens_contain_pattern(&destination_tail, CCA_ATTACKING_WORD_SHAPE, "attacking");
        let battlefield_tapped =
            cca_tokens_contain_pattern(&destination_tail, CCA_TAPPED_WORD_SHAPE, "tapped");
        let battlefield_face_down =
            cca_tokens_contain_phrase(&destination_tail, CCA_FACE_DOWN_PHRASE);
        if let Some(from_idx) = find_index(&destination_tail, |token| {
            cca_token_is(token, CCA_FROM_WORD)
        }) && destination_tail.len() >= from_idx + 3
            && token_slice_starts_with(
                &destination_tail[from_idx + 1..],
                CCA_COMMAND_ZONE_TAIL_PREFIX,
            )
        {
            destination_tail.drain(from_idx..from_idx + 3);
        }
        destination_tail.retain(|token| !cca_token_is_any(token, CCA_DESTINATION_IGNORED_WORDS));

        let mut attached_to_target: Option<TargetAst> = None;
        if destination_tail
            .first()
            .is_some_and(|_| token_slice_starts_with(&destination_tail, CCA_ATTACHED_TO_PREFIX))
        {
            let attachment_target_tokens = trim_commas(&destination_tail[2..]);
            if attachment_target_tokens.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "missing attachment target after 'attached to' (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            attached_to_target = Some(parse_target_phrase(&attachment_target_tokens)?);
            destination_tail.clear();
        }

        if let Some(instead_idx) = find_index(&destination_tail, |token| {
            cca_token_is(token, CCA_INSTEAD_WORD)
        }) {
            destination_tail.truncate(instead_idx);
        }

        let rest_destination_tail = if destination_tail
            .first()
            .is_some_and(|token| cca_token_is(token, CCA_AND_WORD))
        {
            &destination_tail[1..]
        } else {
            destination_tail.as_slice()
        };
        if let Some((rest_target_slice, rest_destination_slice)) =
            super::super::grammar::primitives::split_lexed_once_on_separator(
                rest_destination_tail,
                || {
                    use winnow::Parser as _;
                    super::super::grammar::primitives::kw("into").void()
                },
            )
            && cca_destination_zone_from_tokens(rest_destination_slice) == Some(Zone::Graveyard)
        {
            let target_words = crate::runtime_backend::token_word_refs(&target_tokens);
            let primary_target = if matches!(
                target_words.as_slice(),
                ["it"] | ["them"] | ["that", "card"] | ["those", "card"] | ["those", "cards"]
            ) {
                TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(&target_tokens))
            } else {
                parse_target_phrase(&target_tokens)?
            };
            let primary_effect = EffectAst::subject_verb_move_to_zone_with_attacking(
                primary_target,
                Zone::Battlefield,
                false,
                ReturnControllerAst::Preserve,
                battlefield_tapped,
                battlefield_attacking,
                battlefield_face_down,
                attached_to_target.clone(),
            );
            let rest_target_tokens = trim_commas(rest_target_slice);
            let rest_target_words = crate::runtime_backend::token_word_refs(&rest_target_tokens);
            let rest_target = parse_target_phrase(&rest_target_tokens)?;
            let rest_effect = if rest_target_words
                .first()
                .is_some_and(|word| matches!(*word, "all" | "each"))
            {
                EffectAst::subject_verb_move_all_to_zone(
                    rest_target,
                    Zone::Graveyard,
                    false,
                    ReturnControllerAst::Preserve,
                    false,
                    None,
                )
            } else {
                EffectAst::subject_verb_move_to_zone(
                    rest_target,
                    Zone::Graveyard,
                    false,
                    ReturnControllerAst::Preserve,
                    false,
                    None,
                )
            };
            let effect = EffectAst::Sequence {
                effects: vec![primary_effect, rest_effect],
            };
            return Ok(if let Some(predicate) = trailing_predicate {
                EffectAst::Conditional {
                    predicate,
                    if_true: vec![effect],
                    if_false: Vec::new(),
                }
            } else {
                effect
            });
        }

        let parsed_control_tail = cca_battlefield_controller_tail(&destination_tail);
        let supported_control_tail = destination_tail.is_empty() || parsed_control_tail.is_some();
        if !supported_control_tail {
            return Err(CardTextError::ParseError(format!(
                "unsupported put destination after 'onto' (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let battlefield_controller = parsed_control_tail
            .map(|parsed| parsed.controller)
            .unwrap_or(ReturnControllerAst::Preserve);

        if target_tokens
            .first()
            .is_some_and(|token| cca_token_is_any(token, CCA_ALL_OR_EACH_WORDS))
        {
            let mut filter = parse_object_filter(&target_tokens[1..], false)?;
            if cca_tokens_contain_phrase(&target_tokens[1..], CCA_FROM_IT_PHRASE) {
                filter.zone = Some(Zone::Hand);
                if filter.owner.is_none() {
                    filter.owner = Some(PlayerFilter::You);
                }
                filter
                    .tagged_constraints
                    .retain(|constraint| constraint.tag.as_str() != IT_TAG);
            }
            if cca_tokens_contain_all(tokens, CCA_AMONG_THEM_WORDS) {
                filter.zone = Some(Zone::Exile);
                if filter.owner.is_none() {
                    filter.owner = Some(PlayerFilter::IteratedPlayer);
                }
                if cca_tokens_contain_pattern(tokens, CCA_PERMANENT_WORD_SHAPE, "permanent") {
                    filter.card_types = vec![
                        CardType::Artifact,
                        CardType::Creature,
                        CardType::Enchantment,
                        CardType::Land,
                        CardType::Planeswalker,
                        CardType::Battle,
                    ];
                }
            }
            let effect = EffectAst::subject_verb_return_all_to_battlefield(
                filter,
                battlefield_tapped,
                battlefield_face_down,
                battlefield_controller,
            );
            return Ok(if let Some(predicate) = trailing_predicate {
                EffectAst::Conditional {
                    predicate,
                    if_true: vec![effect],
                    if_false: Vec::new(),
                }
            } else {
                effect
            });
        }

        let target_words = crate::runtime_backend::token_word_refs(&target_tokens);
        let mut target = if matches!(
            target_words.as_slice(),
            ["it"] | ["them"] | ["that", "card"] | ["those", "card"] | ["those", "cards"]
        ) {
            TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(&target_tokens))
        } else {
            parse_target_phrase(&target_tokens)?
        };
        if let Some(filter) = crate::runtime_backend::sentences::effect_sentences::zone_counter_helpers::target_object_filter_mut(&mut target)
        {
            crate::runtime_backend::sentences::effect_sentences::zone_counter_helpers::apply_exile_subject_owner_context(filter, subject);
        }
        if cca_tokens_contain_pattern(
            destination_slice,
            CCA_FROM_COMMAND_ZONE_SHAPE,
            "source_zone",
        ) {
            apply_source_zone_constraint(&mut target, Zone::Command);
        }

        let effect = EffectAst::subject_verb_move_to_zone_with_attacking(
            target,
            Zone::Battlefield,
            false,
            battlefield_controller,
            battlefield_tapped,
            battlefield_attacking,
            battlefield_face_down,
            attached_to_target,
        );
        return Ok(if let Some(predicate) = trailing_predicate {
            EffectAst::Conditional {
                predicate,
                if_true: vec![effect],
                if_false: Vec::new(),
            }
        } else {
            effect
        });
    }

    if contains_word(tokens, CCA_STICKER_WORD) {
        return Err(CardTextError::ParseError(format!(
            "unsupported sticker clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    Err(CardTextError::ParseError(format!(
        "unsupported put clause (clause: '{}')",
        clause_words.join(" ")
    )))
}
