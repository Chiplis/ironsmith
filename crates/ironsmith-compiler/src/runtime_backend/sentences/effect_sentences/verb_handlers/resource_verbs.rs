const SOURCE_ATTACHMENT_PREFIXES: &[&[&str]] = &[
    &["this", "equipment"],
    &["this", "aura"],
    &["this", "enchantment"],
    &["this", "artifact"],
];
const ADDITIONAL_PREFIXES: &[&[&str]] = &[&["an", "additional"], &["additional"]];
const FOR_EACH_OPPONENT_WHO_PREFIXES: &[&[&str]] = &[
    &["for", "each", "opponent", "who"],
    &["for", "each", "opponents", "who"],
];
const FOR_EACH_PLAYER_WHO_PREFIXES: &[&[&str]] = &[
    &["for", "each", "player", "who"],
    &["for", "each", "players", "who"],
];
const EACH_OPPONENT_WHO_PREFIXES: &[&[&str]] =
    &[&["each", "opponent", "who"], &["each", "opponents", "who"]];
const EACH_PLAYER_WHO_PREFIXES: &[&[&str]] =
    &[&["each", "player", "who"], &["each", "players", "who"]];
const EVENT_AMOUNT_PREFIXES: &[&[&str]] = &[
    &["that", "amount", "of"],
    &["that", "much"],
    &["that", "many"],
];
const DAMAGE_TO_EACH_OPPONENT_PREFIXES: &[&[&str]] = &[&["damage", "to", "each", "opponent"]];
const EACH_OF_PREFIXES: &[&[&str]] = &[&["each", "of"]];
const YOU_CONTROL_PREFIXES: &[&[&str]] = &[&["you", "control"], &["you", "controlled"]];
const FOR_EACH_PREFIXES: &[&[&str]] = &[&["for", "each"]];
const EACH_OPPONENT_AND_EACH_PREFIXES: &[&[&str]] = &[&["each", "opponent", "and", "each"]];

const TAKE_EXTRA_TURN_AFTER_THIS_ONE_WORDS: &[&str] =
    &["an", "extra", "turn", "after", "this", "one"];
const PROLIFERATE_TRAILING_OK_PHRASES: &[&[&str]] = &[
    &["time"],
    &["times"],
    &["instead"],
    &["time", "instead"],
    &["times", "instead"],
];
const BENEATH_TOP_AMOUNT_PREFIXES: &[&[&str]] = &[&["just", "beneath", "top"], &["beneath", "top"]];
const NTH_FROM_TOP_DESTINATION_TAIL_WORDS: &[&str] = &["from", "top"];
const THAT_LIBRARY_AMOUNT_TAIL_WORDS: &[&str] = &["of", "that", "library"];
const RESOURCE_LIBRARY_WORDS: &[&str] = &["library", "libraries"];
const RESOURCE_AT_WORD: &str = "at";
const RESOURCE_ARTICLE_WORDS: &[&str] = &["the", "a", "an"];
const RESOURCE_PLAY_THOSE_EXILED_WORDS: &[&str] = &[
    "and", "play", "those", "cards", "for", "as", "long", "as", "they", "remain", "exiled",
];
const RESOURCE_TOP_WORD: &str = "top";
const RESOURCE_CARD_OR_CARDS_WORDS: &[&str] = &["card", "cards"];
const RESOURCE_AND_WORD: &str = "and";
const RESOURCE_ANY_OR_ALL_WORDS: &[&str] = &["any", "all"];
const RESOURCE_OF_WORD: &str = "of";
const RESOURCE_FROM_WORD: &str = "from";
const RESOURCE_AS_YOU_CHOOSE_WORDS: &[&str] = &["as", "you", "choose"];
const RESOURCE_INTO_WORD: &str = "into";
const NOTE_YOUR_LIFE_TOTAL_WORDS: &[&str] = &["your", "life", "total"];
const RESOURCE_THE_REST_PREFIX: &[&str] = &["the", "rest"];
const RESOURCE_ALL_OTHER_PREFIX: &[&str] = &["all", "other"];
const RESOURCE_CARDS_WORD: &str = "cards";
const RESOURCE_CARDS_WORDS: &[&str] = &[RESOURCE_CARDS_WORD];
const RESOURCE_REVEALED_OR_EXILED_WORDS: &[&str] = &["revealed", "exiled"];
const RESOURCE_UNSUPPORTED_SHUFFLE_REQUIRED_WORDS: &[&str] =
    &["graveyard", "cards", "card", "into", "from"];
const RESOURCE_IT_OR_THEM_WORDS: &[&[&str]] = &[&["it"], &["them"]];
const RESOURCE_WITH_WORD: &str = "with";
const RESOURCE_NAME_OR_NAMES_WORDS: &[&str] = &["name", "names"];
const RESOURCE_CHOSEN_NAME_TAIL_PREFIX: &[&str] = &["chosen", "for", "this"];
const RESOURCE_CHOSEN_NAME_OBJECT_NOUN_WORDS: &[&str] = &[
    "artifact",
    "card",
    "creature",
    "enchantment",
    "permanent",
    "source",
];
const RESOURCE_THIS_WAY_WORDS: &[&str] = &["this", "way"];
const RESOURCE_ALL_ABILITIES_PHRASES: &[&[&str]] =
    &[&["all", "abilities"], &["all", "other", "abilities"]];
const RESOURCE_TAKE_EXTRA_TURN_AFTER_THIS_ONE_SHAPE: LexPattern<'static> =
    LexPattern::new(&[LexPattern::action(
        "extra_turn",
        LexCaptureKind::OneOfPhrase(&[TAKE_EXTRA_TURN_AFTER_THIS_ONE_WORDS]),
    )]);
const RESOURCE_PROLIFERATE_TRAILING_OK_SHAPE: LexPattern<'static> =
    LexPattern::new(&[LexPattern::modifier(
        "trailing",
        LexCaptureKind::OneOfPhrase(PROLIFERATE_TRAILING_OK_PHRASES),
    )]);
const RESOURCE_NTH_FROM_TOP_DESTINATION_TAIL_SHAPE: LexPattern<'static> =
    LexPattern::new(&[LexPattern::modifier(
        "tail",
        LexCaptureKind::OneOfPhrase(&[NTH_FROM_TOP_DESTINATION_TAIL_WORDS]),
    )]);
const RESOURCE_BENEATH_TOP_AMOUNT_SHAPE: LexPattern<'static> = LexPattern::new(&[
    LexPattern::modifier(
        "position",
        LexCaptureKind::OneOfPhrase(BENEATH_TOP_AMOUNT_PREFIXES),
    ),
    LexPattern::amount("amount", LexCaptureKind::Rest),
]);
const RESOURCE_THAT_LIBRARY_AMOUNT_TAIL_SHAPE: LexPattern<'static> =
    LexPattern::new(&[LexPattern::modifier(
        "tail",
        LexCaptureKind::OneOfPhrase(&[THAT_LIBRARY_AMOUNT_TAIL_WORDS]),
    )]);
const RESOURCE_PLAY_THOSE_EXILED_SHAPE: LexPattern<'static> =
    LexPattern::new(&[LexPattern::action(
        "permission",
        LexCaptureKind::OneOfPhrase(&[RESOURCE_PLAY_THOSE_EXILED_WORDS]),
    )]);
const RESOURCE_NOTE_YOUR_LIFE_TOTAL_SHAPE: LexPattern<'static> =
    LexPattern::new(&[LexPattern::object(
        "life_total",
        LexCaptureKind::OneOfPhrase(&[NOTE_YOUR_LIFE_TOTAL_WORDS]),
    )]);
const RESOURCE_ALL_ABILITIES_SHAPE: LexPattern<'static> = LexPattern::new(&[LexPattern::object(
    "abilities",
    LexCaptureKind::OneOfPhrase(RESOURCE_ALL_ABILITIES_PHRASES),
)]);
const RESOURCE_IT_OR_THEM_TARGET_SHAPE: LexPattern<'static> =
    LexPattern::new(&[LexPattern::object(
        "target",
        LexCaptureKind::OneOfPhrase(RESOURCE_IT_OR_THEM_WORDS),
    )]);
const RESOURCE_AS_YOU_CHOOSE_SHAPE: LexPattern<'static> =
    LexPattern::new(&[LexPattern::modifier(
        "choice",
        LexCaptureKind::OneOfPhrase(&[RESOURCE_AS_YOU_CHOOSE_WORDS]),
    )]);
const RESOURCE_THE_REST_TARGET_SHAPE: LexPattern<'static> =
    LexPattern::new(&[LexPattern::object(
        "remainder",
        LexCaptureKind::OneOfPhrase(&[RESOURCE_THE_REST_PREFIX]),
    )]);
const RESOURCE_ALL_OTHER_REMAINDER_SHAPE: LexPattern<'static> = LexPattern::new(&[
    LexPattern::phrase(RESOURCE_ALL_OTHER_PREFIX),
    LexPattern::object("remainder", LexCaptureKind::Rest),
]);
const RESOURCE_CARDS_SHAPE: LexPattern<'static> = LexPattern::new(&[LexPattern::object(
    "cards",
    LexCaptureKind::OneOf(RESOURCE_CARDS_WORDS),
)]);
const RESOURCE_REVEALED_OR_EXILED_SHAPE: LexPattern<'static> =
    LexPattern::new(&[LexPattern::modifier(
        "source",
        LexCaptureKind::OneOf(RESOURCE_REVEALED_OR_EXILED_WORDS),
    )]);
const RESOURCE_CHOSEN_NAME_TAIL_SHAPE: LexPattern<'static> = LexPattern::new(&[
    LexPattern::object("name", LexCaptureKind::OneOf(RESOURCE_NAME_OR_NAMES_WORDS)),
    LexPattern::modifier(
        "choice",
        LexCaptureKind::OneOfPhrase(&[RESOURCE_CHOSEN_NAME_TAIL_PREFIX]),
    ),
    LexPattern::object(
        "named_object",
        LexCaptureKind::OneOf(RESOURCE_CHOSEN_NAME_OBJECT_NOUN_WORDS),
    ),
    LexPattern::tail("tail", LexCaptureKind::Rest),
]);
const RESOURCE_THIS_WAY_WORD_SHAPE: LexPattern<'static> =
    LexPattern::new(&[LexPattern::modifier(
        "this_way",
        LexCaptureKind::OneOf(RESOURCE_THIS_WAY_WORDS),
    )]);

const LOOK_YOUR_OWNER_PHRASES: &[&[&str]] = &[&["your"]];
const LOOK_EACH_PLAYER_OWNER_PHRASES: &[&[&str]] = &[&["each", "player"], &["each", "players"]];
const LOOK_THEIR_OWNER_PHRASES: &[&[&str]] = &[&["their"]];
const LOOK_THAT_PLAYER_OWNER_PHRASES: &[&[&str]] = &[&["that", "player"], &["that", "players"]];
const LOOK_TARGET_PLAYER_OWNER_PHRASES: &[&[&str]] =
    &[&["target", "player"], &["target", "players"]];
const LOOK_TARGET_OPPONENT_OWNER_PHRASES: &[&[&str]] =
    &[&["target", "opponent"], &["target", "opponents"]];
const LOOK_OPPONENT_OWNER_PHRASES: &[&[&str]] = &[&["opponent"], &["opponents"]];
const LOOK_ITS_OWNER_PHRASES: &[&[&str]] = &[&["its", "owner"], &["its", "owners"]];
const LOOK_HIS_OR_HER_OWNER_PHRASES: &[&[&str]] = &[&["his", "or", "her"]];
const LOOK_HAND_ZONE_WORDS: &[&str] = &["hand"];
const LOOK_LIBRARY_ZONE_WORDS: &[&str] = &["library"];
const LOOK_TOP_THAT_PLAYER_LIBRARY_PREFIXES: &[&[&str]] = &[
    &["the", "top", "card", "of", "that", "player", "library"],
    &["the", "top", "card", "of", "that", "players", "library"],
    &["top", "card", "of", "that", "player", "library"],
    &["top", "card", "of", "that", "players", "library"],
    &["the", "top", "card", "of", "their", "library"],
    &["top", "card", "of", "their", "library"],
];
const LOOK_TOP_THAT_PLAYER_LIBRARY_SHAPE: LexPattern<'static> =
    LexPattern::new(&[LexPattern::object(
        "library",
        LexCaptureKind::OneOfPhrase(LOOK_TOP_THAT_PLAYER_LIBRARY_PREFIXES),
    )]);

struct LookZoneOwnerEntry {
    phrases: &'static [&'static [&'static str]],
    player: PlayerAst,
}

#[derive(Clone, Copy)]
struct LookZoneOwner {
    player: PlayerAst,
    consumed_words: usize,
}

fn resource_exact_pattern_matches(
    words: &[&str],
    pattern: LexPattern<'static>,
    capture: &str,
) -> bool {
    pattern
        .match_word_refs(words)
        .and_then(|matched| matched.capture_word_range(capture))
        .is_some()
}

fn resource_prefix_pattern_capture_end(
    words: &[&str],
    pattern: LexPattern<'static>,
    capture: &str,
) -> Option<usize> {
    pattern
        .match_prefix_word_refs(words)
        .and_then(|matched| matched.capture_word_range(capture))
        .map(|range| range.end)
}

fn resource_prefix_pattern_matches(
    words: &[&str],
    pattern: LexPattern<'static>,
    capture: &str,
) -> bool {
    resource_prefix_pattern_capture_end(words, pattern, capture).is_some()
}

fn resource_find_pattern_matches(
    words: &[&str],
    pattern: LexPattern<'static>,
    capture: &str,
) -> bool {
    pattern
        .find_in_word_refs(words)
        .and_then(|matched| matched.capture_word_range(capture))
        .is_some()
}

const LOOK_HAND_OWNER_PREFIXES: &[LookZoneOwnerEntry] = &[
    LookZoneOwnerEntry {
        phrases: LOOK_YOUR_OWNER_PHRASES,
        player: PlayerAst::You,
    },
    LookZoneOwnerEntry {
        phrases: LOOK_EACH_PLAYER_OWNER_PHRASES,
        player: PlayerAst::Any,
    },
    LookZoneOwnerEntry {
        phrases: LOOK_THEIR_OWNER_PHRASES,
        player: PlayerAst::That,
    },
    LookZoneOwnerEntry {
        phrases: LOOK_THAT_PLAYER_OWNER_PHRASES,
        player: PlayerAst::That,
    },
    LookZoneOwnerEntry {
        phrases: LOOK_TARGET_PLAYER_OWNER_PHRASES,
        player: PlayerAst::Target,
    },
    LookZoneOwnerEntry {
        phrases: LOOK_TARGET_OPPONENT_OWNER_PHRASES,
        player: PlayerAst::TargetOpponent,
    },
    LookZoneOwnerEntry {
        phrases: LOOK_OPPONENT_OWNER_PHRASES,
        player: PlayerAst::Opponent,
    },
    LookZoneOwnerEntry {
        phrases: LOOK_HIS_OR_HER_OWNER_PHRASES,
        player: PlayerAst::That,
    },
];

const LOOK_LIBRARY_OWNER_PREFIXES: &[LookZoneOwnerEntry] = &[
    LookZoneOwnerEntry {
        phrases: LOOK_YOUR_OWNER_PHRASES,
        player: PlayerAst::You,
    },
    LookZoneOwnerEntry {
        phrases: LOOK_EACH_PLAYER_OWNER_PHRASES,
        player: PlayerAst::Any,
    },
    LookZoneOwnerEntry {
        phrases: LOOK_THEIR_OWNER_PHRASES,
        player: PlayerAst::That,
    },
    LookZoneOwnerEntry {
        phrases: LOOK_THAT_PLAYER_OWNER_PHRASES,
        player: PlayerAst::That,
    },
    LookZoneOwnerEntry {
        phrases: LOOK_TARGET_PLAYER_OWNER_PHRASES,
        player: PlayerAst::Target,
    },
    LookZoneOwnerEntry {
        phrases: LOOK_TARGET_OPPONENT_OWNER_PHRASES,
        player: PlayerAst::TargetOpponent,
    },
    LookZoneOwnerEntry {
        phrases: LOOK_ITS_OWNER_PHRASES,
        player: PlayerAst::ItsOwner,
    },
    LookZoneOwnerEntry {
        phrases: LOOK_HIS_OR_HER_OWNER_PHRASES,
        player: PlayerAst::That,
    },
];

const LOOK_ZONE_NOUN_WORDS: &[&str] = &["hand", "hands", "library", "libraries"];

fn parse_look_zone_owner_lexed(
    tokens: &[OwnedLexToken],
    entries: &[LookZoneOwnerEntry],
) -> Option<LookZoneOwner> {
    let words = LexedClause::new(tokens).word_refs();
    entries.iter().find_map(|entry| {
        let owner_atom = LexPattern::object(
            "owner",
            LexCaptureKind::OneOfPhrase(entry.phrases),
        );
        let zone_atom = LexPattern::object("zone", LexCaptureKind::OneOf(LOOK_ZONE_NOUN_WORDS));
        let atoms = [owner_atom, zone_atom];
        let matched = LexPattern::new(&atoms).match_prefix_word_refs(&words)?;
        matched.capture_word_range("owner")?;
        let zone_range = matched.capture_word_range("zone")?;
        Some(LookZoneOwner {
            player: entry.player,
            consumed_words: zone_range.end,
        })
    })
}

fn is_it_or_them_target(tokens: &[OwnedLexToken]) -> bool {
    resource_exact_pattern_matches(
        &crate::runtime_backend::token_word_refs(tokens),
        RESOURCE_IT_OR_THEM_TARGET_SHAPE,
        "target",
    )
}

#[derive(Clone, Copy)]
enum ResourceLibraryDestinationPlayer {
    Default,
    You,
    DefaultOrController,
    That,
    ItsOwner,
}

struct ResourceLibraryDestinationEntry {
    phrases: &'static [&'static [&'static str]],
    player: ResourceLibraryDestinationPlayer,
}

#[derive(Clone, Copy)]
struct ParsedLibraryDestination {
    player: PlayerAst,
    consumed_words: usize,
}

const SHUFFLE_LIBRARY_BARE_OWNER_PHRASES: &[&[&str]] = &[&[]];
const SHUFFLE_LIBRARY_YOUR_OWNER_PHRASES: &[&[&str]] = &[&["your"]];
const SHUFFLE_LIBRARY_THEIR_OWNER_PHRASES: &[&[&str]] = &[&["their"]];
const SHUFFLE_LIBRARY_THAT_PLAYER_OWNER_PHRASES: &[&[&str]] =
    &[&["that", "player"], &["that", "players"]];
const SHUFFLE_LIBRARY_ITS_OWNER_PHRASES: &[&[&str]] = &[&["its", "owner"], &["its", "owners"]];
const SHUFFLE_LIBRARY_HIS_OR_HER_OWNER_PHRASES: &[&[&str]] = &[&["his", "or", "her"]];
const SHUFFLE_LIBRARY_ZONE_WORDS: &[&str] = &["library", "libraries"];

const SHUFFLE_LIBRARY_DESTINATION_PREFIXES: &[ResourceLibraryDestinationEntry] = &[
    ResourceLibraryDestinationEntry {
        phrases: SHUFFLE_LIBRARY_BARE_OWNER_PHRASES,
        player: ResourceLibraryDestinationPlayer::Default,
    },
    ResourceLibraryDestinationEntry {
        phrases: SHUFFLE_LIBRARY_YOUR_OWNER_PHRASES,
        player: ResourceLibraryDestinationPlayer::You,
    },
    ResourceLibraryDestinationEntry {
        phrases: SHUFFLE_LIBRARY_THEIR_OWNER_PHRASES,
        player: ResourceLibraryDestinationPlayer::DefaultOrController,
    },
    ResourceLibraryDestinationEntry {
        phrases: SHUFFLE_LIBRARY_THAT_PLAYER_OWNER_PHRASES,
        player: ResourceLibraryDestinationPlayer::That,
    },
    ResourceLibraryDestinationEntry {
        phrases: SHUFFLE_LIBRARY_ITS_OWNER_PHRASES,
        player: ResourceLibraryDestinationPlayer::ItsOwner,
    },
    ResourceLibraryDestinationEntry {
        phrases: SHUFFLE_LIBRARY_HIS_OR_HER_OWNER_PHRASES,
        player: ResourceLibraryDestinationPlayer::DefaultOrController,
    },
];

const SHUFFLE_TAGGED_TARGET_PHRASES: &[&[&str]] =
    &[&["it"], &["them"], &["that", "card"], &["those", "cards"]];
const SHUFFLE_TARGET_INTO_THEIR_LIBRARY_SHAPE: LexPattern<'static> = LexPattern::new(&[
    LexPattern::object(
        "target",
        LexCaptureKind::OneOfPhrase(SHUFFLE_TAGGED_TARGET_PHRASES),
    ),
    LexPattern::word(RESOURCE_INTO_WORD),
    LexPattern::subject(
        "owner",
        LexCaptureKind::OneOfPhrase(SHUFFLE_LIBRARY_THEIR_OWNER_PHRASES),
    ),
    LexPattern::object("zone", LexCaptureKind::OneOf(SHUFFLE_LIBRARY_ZONE_WORDS)),
]);
const SHUFFLE_SOURCE_OWNER_PHRASES: &[&[&str]] = &[
    &["your"],
    &["their"],
    &["that", "player"],
    &["that", "players"],
    &["its", "owner"],
    &["its", "owners"],
    &["his", "or", "her"],
    &[],
];
const SHUFFLE_GRAVEYARD_ZONE_WORDS: &[&str] = &["graveyard", "graveyards"];
const SUPPORTED_SHUFFLE_SOURCE_TAIL_PHRASES: &[&[&str]] = &[
    &["from", "your", "graveyard"],
    &["from", "your", "graveyards"],
    &["from", "their", "graveyard"],
    &["from", "their", "graveyards"],
    &["from", "that", "player", "graveyard"],
    &["from", "that", "player", "graveyards"],
    &["from", "that", "players", "graveyard"],
    &["from", "that", "players", "graveyards"],
    &["from", "its", "owner", "graveyard"],
    &["from", "its", "owner", "graveyards"],
    &["from", "its", "owners", "graveyard"],
    &["from", "its", "owners", "graveyards"],
    &["from", "his", "or", "her", "graveyard"],
    &["from", "his", "or", "her", "graveyards"],
    &["from", "graveyard"],
    &["from", "graveyards"],
];

fn resource_non_article_tokens(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    tokens
        .iter()
        .filter(|token| !token_is_any_resource_word(token, RESOURCE_ARTICLE_WORDS))
        .cloned()
        .collect()
}

fn token_is_resource_word(token: &OwnedLexToken, expected: &str) -> bool {
    token.as_word().is_some_and(|word| word == expected)
}

fn token_is_any_resource_word(token: &OwnedLexToken, expected: &[&str]) -> bool {
    token.as_word().is_some_and(|word| expected.contains(&word))
}

fn word_is_any_resource_word(word: &str, expected: &[&str]) -> bool {
    expected.contains(&word)
}

fn resource_all_other_revealed_or_exiled_cards(words: &[&str]) -> bool {
    let Some(remainder_range) = RESOURCE_ALL_OTHER_REMAINDER_SHAPE
        .match_prefix_word_refs(words)
        .and_then(|matched| matched.capture_word_range("remainder"))
    else {
        return false;
    };
    let Some(remainder_words) = words.get(remainder_range) else {
        return false;
    };

    resource_find_pattern_matches(remainder_words, RESOURCE_CARDS_SHAPE, "cards")
        && resource_find_pattern_matches(
            remainder_words,
            RESOURCE_REVEALED_OR_EXILED_SHAPE,
            "source",
        )
}

fn resource_unsupported_shuffle_marker(words: &[&str]) -> bool {
    RESOURCE_UNSUPPORTED_SHUFFLE_REQUIRED_WORDS
        .iter()
        .all(|word| resource_word_pattern_matches(words, word))
}

fn resource_word_pattern_matches(words: &[&str], expected: &str) -> bool {
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

fn resource_chosen_name_tail_matches(tail: &[&str]) -> bool {
    if !tail
        .first()
        .is_some_and(|word| RESOURCE_NAME_OR_NAMES_WORDS.contains(word))
    {
        return false;
    }
    if !word_slice_starts_with(&tail[1..], RESOURCE_CHOSEN_NAME_TAIL_PREFIX) {
        return false;
    }
    let after_prefix = 1 + RESOURCE_CHOSEN_NAME_TAIL_PREFIX.len();
    if !tail
        .get(after_prefix)
        .is_some_and(|word| RESOURCE_CHOSEN_NAME_OBJECT_NOUN_WORDS.contains(word))
    {
        return false;
    }
    tail[after_prefix + 1..]
        .iter()
        .all(|word| RESOURCE_THIS_WAY_WORDS.contains(word))
}

fn default_or_controller_player(default_player: PlayerAst) -> PlayerAst {
    if matches!(default_player, PlayerAst::Implicit) {
        PlayerAst::ItsController
    } else {
        default_player
    }
}

fn resolve_library_destination_player(
    player: ResourceLibraryDestinationPlayer,
    default_player: PlayerAst,
) -> PlayerAst {
    match player {
        ResourceLibraryDestinationPlayer::Default => default_player,
        ResourceLibraryDestinationPlayer::You => PlayerAst::You,
        ResourceLibraryDestinationPlayer::DefaultOrController => {
            default_or_controller_player(default_player)
        }
        ResourceLibraryDestinationPlayer::That => PlayerAst::That,
        ResourceLibraryDestinationPlayer::ItsOwner => PlayerAst::ItsOwner,
    }
}

fn parse_library_destination_player_lexed(
    tokens: &[OwnedLexToken],
    default_player: PlayerAst,
) -> Option<ParsedLibraryDestination> {
    let words = LexedClause::new(tokens).word_refs();
    SHUFFLE_LIBRARY_DESTINATION_PREFIXES
        .iter()
        .find_map(|entry| {
            let destination_atom = LexPattern::object(
                "destination",
                LexCaptureKind::OneOfPhrase(entry.phrases),
            );
            let zone_atom =
                LexPattern::object("zone", LexCaptureKind::OneOf(SHUFFLE_LIBRARY_ZONE_WORDS));
            let atoms = [destination_atom, zone_atom];
            let matched = LexPattern::new(&atoms).match_prefix_word_refs(&words)?;
            matched.capture_word_range("destination")?;
            let zone_range = matched.capture_word_range("zone")?;
            Some(ParsedLibraryDestination {
                player: resolve_library_destination_player(entry.player, default_player),
                consumed_words: zone_range.end,
            })
        })
}

fn is_tagged_shuffle_target_lexed(tokens: &[OwnedLexToken]) -> bool {
    let atoms = [LexPattern::object(
        "target",
        LexCaptureKind::OneOfPhrase(SHUFFLE_TAGGED_TARGET_PHRASES),
    )];
    LexPattern::new(&atoms)
        .match_clause(LexedClause::new(tokens))
        .and_then(|matched| matched.capture_word_range("target"))
        .is_some()
}

fn is_tagged_shuffle_target_into_their_library_lexed(tokens: &[OwnedLexToken]) -> bool {
    SHUFFLE_TARGET_INTO_THEIR_LIBRARY_SHAPE
        .match_clause(LexedClause::new(tokens))
        .and_then(|matched| {
            matched
                .capture_word_range("target")
                .zip(matched.capture_word_range("zone"))
        })
        .is_some()
}

fn is_supported_shuffle_source_tail_lexed(tokens: &[OwnedLexToken]) -> bool {
    let normalized_tokens = resource_non_article_tokens(tokens);
    if normalized_tokens.is_empty() {
        return true;
    }

    let atoms = [LexPattern::object(
        "tail",
        LexCaptureKind::OneOfPhrase(SUPPORTED_SHUFFLE_SOURCE_TAIL_PHRASES),
    )];
    let words = LexedClause::new(&normalized_tokens).word_refs();
    LexPattern::new(&atoms)
        .match_word_refs(&words)
        .and_then(|matched| matched.capture_word_range("tail"))
        .is_some()
}

fn is_simple_library_phrase_lexed(tokens: &[OwnedLexToken]) -> bool {
    let normalized_tokens = resource_non_article_tokens(tokens);
    let Some(destination) =
        parse_library_destination_player_lexed(&normalized_tokens, PlayerAst::Implicit)
    else {
        return false;
    };
    destination.consumed_words == LexedClause::new(&normalized_tokens).word_len()
}

fn subject_verb_player_resource_effect(
    role: SubjectVerbRoleAst,
    player: PlayerAst,
    action: SubjectVerbActionAst,
) -> EffectAst {
    EffectAst::SubjectVerb(SubjectVerbEffectAst {
        subject: SubjectVerbSubjectAst { role, player },
        action,
    })
}

pub(crate) fn parse_effect_with_verb(
    verb: Verb,
    subject: Option<SubjectAst>,
    tokens: &[OwnedLexToken],
) -> Result<EffectAst, CardTextError> {
    crate::parse_trace::event(format!(
        "effect-route: subject-verb verb={verb:?} subject={}",
        if subject.is_some() {
            "explicit"
        } else {
            "implicit"
        }
    ));
    match verb {
        Verb::Add => parse_add_mana(tokens, subject),
        Verb::Move => parse_move(tokens),
        Verb::Deal => parse_deal_damage(tokens),
        Verb::Draw => parse_draw(tokens, subject),
        Verb::Counter => parse_counter(tokens),
        Verb::Destroy => parse_destroy(tokens),
        Verb::Exile => parse_exile(tokens, subject),
        Verb::Reveal => parse_reveal(tokens, subject),
        Verb::Look => parse_look(tokens, subject),
        Verb::Lose => {
            let words = crate::runtime_backend::token_word_refs(tokens);
            if resource_exact_pattern_matches(&words, RESOURCE_ALL_ABILITIES_SHAPE, "abilities")
                && matches!(subject, Some(SubjectAst::This) | None)
            {
                return Ok(EffectAst::subject_verb_remove_abilities_from_target(
                    TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(tokens)),
                    Vec::new(),
                    Until::Forever,
                ));
            }
            parse_lose_life(tokens, subject)
        }
        Verb::Gain => {
            if token_slice_first_is(tokens, "control") {
                parse_gain_control(tokens, subject)
            } else if token_slice_first_is(tokens, "gain") && token_slice_at_is(tokens, 1, "control")
            {
                parse_gain_control(&tokens[1..], subject)
            } else {
                parse_gain_life(tokens, subject)
            }
        }
        Verb::Put => {
            let has_onto = crate::runtime_backend::lexer::contains_token_word(tokens, "onto");
            let has_counter_words = crate::runtime_backend::lexer::contains_token_any_word(
                tokens,
                &["counter", "counters"],
            );

            // Prefer zone moves like "... onto the battlefield" over counter placement because
            // "counter(s)" may appear in subordinate clauses (e.g. "mana value equal to the number
            // of charge counters on this artifact").
            if has_onto {
                if let Ok(effect) = parse_put_into_hand(tokens, subject) {
                    Ok(effect)
                } else if has_counter_words {
                    parse_put_counters(tokens)
                } else {
                    parse_put_into_hand(tokens, subject)
                }
            } else if has_counter_words {
                parse_put_counters(tokens)
            } else {
                parse_put_into_hand(tokens, subject)
            }
        }
        Verb::Sacrifice => parse_sacrifice(tokens, subject, None),
        Verb::Create => parse_create(tokens, subject),
        Verb::Investigate => parse_investigate(tokens, subject),
        Verb::Incubate => parse_incubate(tokens, subject),
        Verb::Proliferate => parse_proliferate(tokens),
        Verb::Tap => parse_tap(tokens),
        Verb::Attach => parse_attach(tokens),
        Verb::Untap => parse_untap(tokens),
        Verb::Scry => parse_scry(tokens, subject),
        Verb::Discard => parse_discard(tokens, subject),
        Verb::Transform => parse_transform(tokens),
        Verb::Convert => parse_convert(tokens),
        Verb::Flip => parse_flip(tokens, subject),
        Verb::Roll => parse_roll(tokens, subject),
        Verb::Regenerate => parse_regenerate(tokens),
        Verb::Mill => parse_mill(tokens, subject),
        Verb::Get => parse_get(tokens, subject),
        Verb::Remove => parse_remove(tokens),
        Verb::Return => parse_return(tokens),
        Verb::Exchange => parse_exchange(tokens, subject),
        Verb::Become => parse_become(tokens, subject),
        Verb::Switch => parse_switch(tokens),
        Verb::Skip => parse_skip(tokens, subject),
        Verb::Surveil => parse_surveil(tokens, subject),
        Verb::Shuffle => parse_shuffle(tokens, subject),
        Verb::Reorder => parse_reorder(tokens, subject),
        Verb::Pay => parse_pay(tokens, subject),
        Verb::Take => parse_take(tokens, subject),
        Verb::Detain => parse_detain(tokens),
        Verb::Goad => parse_goad(tokens),
        Verb::Suspect => parse_suspect(tokens),
        Verb::Note => parse_note(tokens),
        Verb::End => parse_end(tokens, subject),
    }
}

fn parse_note(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    if word_slice_eq(&words, NOTE_YOUR_LIFE_TOTAL_WORDS) {
        return Ok(subject_verb_player_resource_effect(
            SubjectVerbRoleAst::Actor,
            PlayerAst::You,
            SubjectVerbActionAst::NoteLifeTotal,
        ));
    }
    Err(CardTextError::ParseError(format!(
        "unsupported note clause: '{}'",
        words.join(" ")
    )))
}

fn parse_take(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    if word_slice_eq(&words, TAKE_EXTRA_TURN_AFTER_THIS_ONE_WORDS) {
        return Ok(EffectAst::subject_verb_extra_turn_after_turn(
            extract_subject_player(subject).unwrap_or(PlayerAst::You),
            ExtraTurnAnchorAst::CurrentTurn,
        ));
    }

    Err(CardTextError::ParseError(format!(
        "unsupported take clause (clause: '{}')",
        words.join(" ")
    )))
}

fn parse_proliferate(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    if tokens.is_empty() {
        return Ok(EffectAst::subject_verb_proliferate(Value::Fixed(1)));
    }

    let (count, used) = if let Some(first) = tokens.first().and_then(OwnedLexToken::as_word) {
        match first {
            "once" => (Value::Fixed(1), 1),
            "twice" => (Value::Fixed(2), 1),
            _ => parse_value(tokens).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing proliferate count (clause: '{}')",
                    crate::runtime_backend::token_word_refs(tokens).join(" ")
                ))
            })?,
        }
    } else {
        return Err(CardTextError::ParseError(format!(
            "missing proliferate count (clause: '{}')",
            crate::runtime_backend::token_word_refs(tokens).join(" ")
        )));
    };

    let trailing = trim_commas(&tokens[used..]);
    let trailing_ok = trailing.is_empty()
        || resource_exact_pattern_matches(
            &crate::runtime_backend::token_word_refs(&trailing),
            RESOURCE_PROLIFERATE_TRAILING_OK_SHAPE,
            "trailing",
        );
    if !trailing_ok {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing proliferate clause (clause: '{}')",
            crate::runtime_backend::token_word_refs(tokens).join(" ")
        )));
    }

    Ok(EffectAst::subject_verb_proliferate(count))
}

fn parse_library_nth_from_top_destination(tokens: &[OwnedLexToken]) -> Option<Value> {
    let library_idx = find_index(tokens, |token: &OwnedLexToken| {
        token_is_any_resource_word(token, RESOURCE_LIBRARY_WORDS)
    })?;
    let tail_tokens = trim_commas(&tokens[library_idx + 1..]);
    if tail_tokens.is_empty() {
        return None;
    }

    let filtered_tail = crate::runtime_backend::util::non_article_token_word_refs(&tail_tokens);
    if let Some((position, used)) = ironsmith_core::parse_ordinal_words(&filtered_tail)
        && filtered_tail
            .get(used..)
            .is_some_and(|tail| word_slice_eq(tail, NTH_FROM_TOP_DESTINATION_TAIL_WORDS))
    {
        return Some(Value::Fixed(position as i32));
    }

    let amount_range = RESOURCE_BENEATH_TOP_AMOUNT_SHAPE
        .match_prefix_word_refs(&filtered_tail)
        .and_then(|matched| matched.capture_word_range("amount"))?;
    let amount_words = filtered_tail.get(amount_range)?;
    let (amount, used) = parse_value_expr_words(amount_words)?;
    if !amount_words
        .get(used)
        .is_some_and(|word| RESOURCE_CARD_OR_CARDS_WORDS.contains(word))
    {
        return None;
    }
    if used + 1 > amount_words.len() {
        return None;
    }
    if !resource_exact_pattern_matches(
        &amount_words[used + 1..],
        RESOURCE_THAT_LIBRARY_AMOUNT_TAIL_SHAPE,
        "tail",
    ) {
        return None;
    }

    Some(Value::Add(Box::new(amount), Box::new(Value::Fixed(1))))
}

pub(crate) fn parse_look(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    fn parse_look_tail_at_same_player(words: &[&str]) -> Option<Vec<EffectAst>> {
        let top_prefix_len = resource_prefix_pattern_capture_end(
            words,
            LOOK_TOP_THAT_PLAYER_LIBRARY_SHAPE,
            "library",
        )?;
        let mut rest = &words[top_prefix_len..];
        let mut effects = vec![EffectAst::subject_verb_look_at_top_cards(
            PlayerAst::That,
            Value::Fixed(1),
            TagKey::from(IT_TAG),
        )];

        if rest.is_empty() {
            return Some(effects);
        }
        if rest.first() == Some(&RESOURCE_AND_WORD) {
            rest = &rest[1..];
        }
        if rest
            .first()
            .is_some_and(|word| word_is_any_resource_word(word, RESOURCE_ANY_OR_ALL_WORDS))
        {
            rest = &rest[1..];
        }
        if matches!(
            rest,
            ["face", "down", "creatures", "they", "control"]
                | ["face", "down", "creature", "they", "control"]
                | ["face", "down", "creatures", "that", "player", "controls"]
                | ["face", "down", "creatures", "that", "players", "control"]
                | ["face", "down", "creature", "that", "player", "controls"]
                | ["face", "down", "creature", "that", "players", "control"]
        ) {
            effects.push(EffectAst::subject_verb_look_at_objects(
                PlayerAst::That,
                ObjectFilter::creature().face_down(),
            ));
            return Some(effects);
        }

        None
    }

    // "Look at the top N cards of your library."
    let mut clause_tokens = trim_commas(tokens);
    if clause_tokens
        .first()
        .is_some_and(|token| token_is_resource_word(token, RESOURCE_AT_WORD))
    {
        clause_tokens = trim_commas(&clause_tokens[1..]);
    }
    let clause_word_storage = TokenWordView::new(&clause_tokens).owned_words();
    let clause_words = clause_word_storage
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();

    if word_slice_eq(&clause_words, RESOURCE_PLAY_THOSE_EXILED_WORDS) {
        return Ok(
            EffectAst::subject_verb_grant_play_tagged_for_as_long_as_exiled(
                TagKey::from(IT_TAG),
                PlayerAst::You,
                true,
                false,
                false,
                None,
            ),
        );
    }

    let mut hand_tokens = clause_tokens.clone();
    while hand_tokens
        .first()
        .is_some_and(|token| token_is_any_resource_word(token, RESOURCE_ARTICLE_WORDS))
    {
        hand_tokens = hand_tokens[1..].to_vec();
    }
    let hand_word_storage = TokenWordView::new(&hand_tokens).owned_words();
    let hand_words = hand_word_storage
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    if let Some(owner) =
        parse_look_zone_owner_lexed(&hand_tokens, LOOK_HAND_OWNER_PREFIXES)
    {
        let target = match owner.player {
            PlayerAst::You => TargetAst::Player(PlayerFilter::You, None),
            PlayerAst::Opponent => TargetAst::Player(PlayerFilter::Opponent, None),
            PlayerAst::Target => TargetAst::Player(
                PlayerFilter::target_player(),
                span_from_tokens(&hand_tokens),
            ),
            PlayerAst::TargetOpponent => {
                TargetAst::Player(PlayerFilter::Opponent, span_from_tokens(&hand_tokens))
            }
            PlayerAst::That => TargetAst::Player(PlayerFilter::IteratedPlayer, None),
            PlayerAst::Any => {
                return Ok(EffectAst::ForEachPlayer {
                    effects: vec![EffectAst::subject_verb_look_at_hand(TargetAst::Player(
                        PlayerFilter::IteratedPlayer,
                        None,
                    ))],
                });
            }
            _ => {
                return Err(CardTextError::ParseError(format!(
                    "unsupported look clause (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
        };

        if owner.consumed_words < hand_words.len() {
            if let Some(mut followups) =
                parse_look_tail_at_same_player(&hand_words[owner.consumed_words..])
            {
                let mut effects = vec![EffectAst::subject_verb_look_at_hand(target)];
                effects.append(&mut followups);
                return Ok(EffectAst::Sequence { effects });
            }
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing look clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }

        return Ok(EffectAst::subject_verb_look_at_hand(target));
    }

    if let Some(filter) = match hand_words.as_slice() {
        ["target", "face", "down", "creature"] | ["target", "face", "down", "creatures"] => {
            Some(ObjectFilter::creature().face_down())
        }
        ["target", "face", "down", "permanent"] | ["target", "face", "down", "permanents"] => {
            Some(ObjectFilter::permanent().face_down())
        }
        _ => None,
    } {
        let target = TargetAst::Object(filter, span_from_tokens(&hand_tokens), None);
        return Ok(EffectAst::subject_verb_look_at_target(target));
    }

    let Some(top_idx) = find_index(&clause_tokens, |t| {
        token_is_resource_word(t, RESOURCE_TOP_WORD)
    }) else {
        return Err(CardTextError::ParseError(format!(
            "unsupported look clause (clause: '{}')",
            clause_words.join(" ")
        )));
    };
    if top_idx + 1 >= clause_tokens.len() {
        return Err(CardTextError::ParseError(format!(
            "missing look top noun (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let count_before_top = parse_value(&clause_tokens[..top_idx]).and_then(|(value, used)| {
        let mut probe = used;
        if !clause_tokens
            .get(probe)
            .and_then(OwnedLexToken::as_word)
            .is_some_and(|w| word_is_any_resource_word(w, RESOURCE_CARD_OR_CARDS_WORDS))
        {
            return None;
        }
        probe += 1;
        if clause_tokens
            .get(probe)
            .and_then(OwnedLexToken::as_word)
            .is_some_and(|w| w == "from")
        {
            probe += 1;
        }
        while clause_tokens
            .get(probe)
            .is_some_and(|t| token_is_any_resource_word(t, RESOURCE_ARTICLE_WORDS))
        {
            probe += 1;
        }
        (probe == top_idx).then_some(value)
    });

    let mut idx = top_idx + 1;
    let count = if let Some(value) = count_before_top {
        value
    } else {
        let count = if clause_tokens
            .get(idx)
            .and_then(OwnedLexToken::as_word)
            .is_some_and(|w| word_is_any_resource_word(w, RESOURCE_CARD_OR_CARDS_WORDS))
        {
            Value::Fixed(1)
        } else {
            let (value, used) = parse_value(&clause_tokens[idx..]).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing look count (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?;
            idx += used;
            value
        };

        // Consume "card(s)"
        if clause_tokens
            .get(idx)
            .and_then(OwnedLexToken::as_word)
            .is_some_and(|w| word_is_any_resource_word(w, RESOURCE_CARD_OR_CARDS_WORDS))
        {
            idx += 1;
        } else {
            return Err(CardTextError::ParseError(format!(
                "missing look card noun (clause: '{}')",
                clause_words.join(" ")
            )));
        }

        count
    };

    // Consume "of <player> library"
    if !clause_tokens
        .get(idx)
        .is_some_and(|t| token_is_resource_word(t, RESOURCE_OF_WORD))
    {
        return Err(CardTextError::ParseError(format!(
            "missing 'of' in look clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    idx += 1;
    let mut owner_tokens = &clause_tokens[idx..];
    while owner_tokens
        .first()
        .is_some_and(|t| token_is_any_resource_word(t, RESOURCE_ARTICLE_WORDS))
    {
        owner_tokens = &owner_tokens[1..];
    }
    let owner_word_storage = TokenWordView::new(owner_tokens).owned_words();
    let owner_words = owner_word_storage
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    let owner = parse_look_zone_owner_lexed(owner_tokens, LOOK_LIBRARY_OWNER_PREFIXES).or_else(|| {
        // If the clause uses a subject ("target player looks ..."), treat that as the default.
        subject.and_then(|s| match s {
            SubjectAst::Player(player) => Some(LookZoneOwner {
                player,
                consumed_words: 0,
            }),
            _ => None,
        })
    })
    .ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported look library owner (clause: '{}')",
            clause_words.join(" ")
        ))
    })?;
    // No trailing words supported for now (based on word tokens).
    if owner.consumed_words < owner_words.len() {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing look clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    if matches!(owner.player, PlayerAst::Any) {
        return Ok(EffectAst::ForEachPlayer {
            effects: vec![EffectAst::subject_verb_look_at_top_cards(
                PlayerAst::That,
                count,
                TagKey::from(IT_TAG),
            )],
        });
    }

    Ok(EffectAst::subject_verb_look_at_top_cards(
        owner.player,
        count,
        TagKey::from(IT_TAG),
    ))
}

pub(crate) fn parse_reorder(
    tokens: &[OwnedLexToken],
    _subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let clause = crate::runtime_backend::token_word_refs(tokens).join(" ");
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if clause_words.is_empty() {
        return Err(CardTextError::ParseError(
            "missing reorder target".to_string(),
        ));
    }

    let Some(owner) = parse_graveyard_owner_prefix_lexed(tokens) else {
        return Err(CardTextError::ParseError(format!(
            "unsupported reorder clause (clause: '{clause}')"
        )));
    };
    if !matches!(
        owner.player,
        PlayerAst::You | PlayerAst::That | PlayerAst::ItsController | PlayerAst::ItsOwner
    ) {
        return Err(CardTextError::ParseError(format!(
            "unsupported reorder clause (clause: '{clause}')"
        )));
    }
    let rest_token_idx = LexedClause::new(tokens)
        .words()
        .token_index_for_word_or_end(owner.consumed_words)
        .unwrap_or(tokens.len());
    let rest = trim_commas(&tokens[rest_token_idx..]);

    if !rest.is_empty()
        && !resource_exact_pattern_matches(
            &crate::runtime_backend::token_word_refs(&rest),
            RESOURCE_AS_YOU_CHOOSE_SHAPE,
            "choice",
        )
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported reorder clause tail (clause: '{clause}')"
        )));
    }

    Ok(EffectAst::subject_verb_reorder_graveyard(owner.player))
}

pub(crate) fn parse_shuffle(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);

    if tokens.is_empty() {
        // Support standalone "Shuffle." clauses. If the sentence includes an explicit player
        // subject, use it; otherwise return an implicit player that can be filled in by the
        // carry-context logic (and compiles to "you" by default).
        return Ok(subject_verb_player_resource_effect(
            SubjectVerbRoleAst::LibraryOwner,
            player,
            SubjectVerbActionAst::ShuffleLibrary,
        ));
    }

    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if let Some(into_idx) = find_index(&clause_words, |word| *word == RESOURCE_INTO_WORD) {
        let target_token_idx = LexedClause::new(tokens)
            .words()
            .token_index_for_word_index(into_idx)
            .unwrap_or(tokens.len());
        let destination_token_idx = LexedClause::new(tokens)
            .words()
            .token_index_for_word_or_end(into_idx + 1)
            .unwrap_or(tokens.len());
        let target_tokens = trim_commas(&tokens[..target_token_idx]);
        let destination_tokens =
            resource_non_article_tokens(&trim_commas(&tokens[destination_token_idx..]));
        if is_tagged_shuffle_target_lexed(&target_tokens)
            && let Some(destination) =
                parse_library_destination_player_lexed(&destination_tokens, player)
        {
            let trailing_token_idx = LexedClause::new(&destination_tokens)
                .words()
                .token_index_for_word_or_end(destination.consumed_words)
                .unwrap_or(destination_tokens.len());
            if is_supported_shuffle_source_tail_lexed(&destination_tokens[trailing_token_idx..]) {
                return Ok(EffectAst::ForEachTagged {
                    tag: TagKey::from(IT_TAG),
                    effects: vec![
                        EffectAst::subject_verb_move_to_zone(
                            TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(tokens)),
                            Zone::Library,
                            false,
                            ReturnControllerAst::Preserve,
                            false,
                            None,
                        ),
                        subject_verb_player_resource_effect(
                            SubjectVerbRoleAst::LibraryOwner,
                            destination.player,
                            SubjectVerbActionAst::ShuffleLibrary,
                        ),
                    ],
                });
            }
        }

        let target_words = crate::runtime_backend::token_word_refs(&target_tokens);
        let consult_style_remainder_shuffle =
            resource_prefix_pattern_matches(
                &target_words,
                RESOURCE_THE_REST_TARGET_SHAPE,
                "remainder",
            ) || resource_all_other_revealed_or_exiled_cards(&target_words);
        if consult_style_remainder_shuffle
            && let Some(destination) =
                parse_library_destination_player_lexed(&destination_tokens, player)
            && {
                let trailing_token_idx = LexedClause::new(&destination_tokens)
                    .words()
                    .token_index_for_word_or_end(destination.consumed_words)
                    .unwrap_or(destination_tokens.len());
                is_supported_shuffle_source_tail_lexed(&destination_tokens[trailing_token_idx..])
            }
        {
            return Ok(subject_verb_player_resource_effect(
                SubjectVerbRoleAst::LibraryOwner,
                destination.player,
                SubjectVerbActionAst::ShuffleLibrary,
            ));
        }
    }

    if matches!(player, PlayerAst::ItsOwner)
        && is_tagged_shuffle_target_into_their_library_lexed(tokens)
    {
        return Ok(EffectAst::ForEachTagged {
            tag: TagKey::from(IT_TAG),
            effects: vec![
                EffectAst::subject_verb_move_to_zone(
                    TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(tokens)),
                    Zone::Library,
                    true,
                    ReturnControllerAst::Preserve,
                    false,
                    None,
                ),
                subject_verb_player_resource_effect(
                    SubjectVerbRoleAst::LibraryOwner,
                    PlayerAst::ItsOwner,
                    SubjectVerbActionAst::ShuffleLibrary,
                ),
            ],
        });
    }
    if resource_unsupported_shuffle_marker(&clause_words) {
        return Err(CardTextError::ParseError(format!(
            "unsupported shuffle clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    if is_simple_library_phrase_lexed(tokens) {
        return Ok(subject_verb_player_resource_effect(
            SubjectVerbRoleAst::LibraryOwner,
            player,
            SubjectVerbActionAst::ShuffleLibrary,
        ));
    }

    Err(CardTextError::ParseError(format!(
        "unsupported shuffle clause (clause: '{}')",
        clause_words.join(" ")
    )))
}

pub(crate) fn parse_goad(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    let target_tokens = trim_commas(tokens);
    if target_tokens.is_empty() {
        return Err(CardTextError::ParseError("missing goad target".to_string()));
    }

    let target_words = crate::runtime_backend::token_word_refs(&target_tokens);
    if let Some(target) = parse_chosen_name_goad_target(&target_tokens, &target_words)? {
        return Ok(EffectAst::subject_verb_goad(target));
    }
    if is_it_or_them_target(&target_tokens) {
        return Ok(EffectAst::subject_verb_goad(TargetAst::Tagged(
            TagKey::from(IT_TAG),
            span_from_tokens(&target_tokens),
        )));
    }

    let target = parse_target_phrase(&target_tokens)?;
    if matches!(
        target,
        TargetAst::Player(_, _) | TargetAst::PlayerOrPlaneswalker(_, _)
    ) {
        return Err(CardTextError::ParseError(format!(
            "goad target must be a creature (clause: '{}')",
            crate::runtime_backend::token_word_refs(tokens).join(" ")
        )));
    }

    Ok(EffectAst::subject_verb_goad(target))
}

fn parse_chosen_name_goad_target(
    target_tokens: &[OwnedLexToken],
    target_words: &[&str],
) -> Result<Option<TargetAst>, CardTextError> {
    for with_word_idx in 0..target_words.len() {
        if target_words[with_word_idx] != RESOURCE_WITH_WORD {
            continue;
        }

        let Some(with_token_idx) = token_index_for_word_index(target_tokens, with_word_idx) else {
            continue;
        };
        let mut tail_tokens = &target_tokens[with_token_idx + 1..];
        while tail_tokens
            .first()
            .is_some_and(|token| token_is_any_resource_word(token, RESOURCE_ARTICLE_WORDS))
        {
            tail_tokens = &tail_tokens[1..];
        }
        let tail = crate::runtime_backend::token_word_refs(tail_tokens);
        let chosen_name_tail = resource_chosen_name_tail_matches(&tail);
        if !chosen_name_tail {
            continue;
        }

        let base_tokens = trim_commas(&target_tokens[..with_token_idx]);
        if base_tokens.is_empty() {
            continue;
        }

        let mut target = parse_target_phrase(&base_tokens)?;
        add_chosen_name_constraint_to_target(&mut target);
        return Ok(Some(target));
    }

    Ok(None)
}

fn add_chosen_name_constraint_to_target(target: &mut TargetAst) {
    match target {
        TargetAst::Object(filter, _, _) => {
            filter.tagged_constraints.push(TaggedObjectConstraint {
                tag: TagKey::from("__chosen_name__"),
                relation: TaggedOpbjectRelation::SameNameAsTagged,
            });
        }
        TargetAst::WithCount(inner, _) | TargetAst::WithCountValue(inner, _, _) => {
            add_chosen_name_constraint_to_target(inner);
        }
        _ => {}
    }
}

pub(crate) fn parse_detain(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    let target_tokens = trim_commas(tokens);
    if target_tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "missing detain target".to_string(),
        ));
    }

    if is_it_or_them_target(&target_tokens) {
        return Ok(EffectAst::subject_verb_detain(TargetAst::Tagged(
            TagKey::from(IT_TAG),
            span_from_tokens(&target_tokens),
        )));
    }

    Ok(EffectAst::subject_verb_detain(parse_target_phrase(
        &target_tokens,
    )?))
}

pub(crate) fn parse_suspect(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    let target_tokens = trim_commas(tokens);
    if target_tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "missing suspect target".to_string(),
        ));
    }

    if is_it_or_them_target(&target_tokens) {
        return Ok(EffectAst::subject_verb_suspect(TargetAst::Tagged(
            TagKey::from(IT_TAG),
            span_from_tokens(&target_tokens),
        )));
    }

    let target = parse_target_phrase(&target_tokens)?;
    if matches!(
        target,
        TargetAst::Player(_, _) | TargetAst::PlayerOrPlaneswalker(_, _)
    ) {
        return Err(CardTextError::ParseError(format!(
            "suspect target must be a creature (clause: '{}')",
            crate::runtime_backend::token_word_refs(tokens).join(" ")
        )));
    }

    Ok(EffectAst::subject_verb_suspect(target))
}
