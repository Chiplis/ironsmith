use super::*;
use crate::runtime_backend::lex_patterns::{LexCaptureKind, LexPattern};
use crate::runtime_backend::lexer::{
    LexedClause, word_slice_contains_phrase, word_slice_eq, word_slice_starts_with,
    word_slice_starts_with_any,
};
const EXILE_ALL_OR_EACH_WORDS: &[&str] = &["all", "each"];
const EXILE_CARD_OR_CARDS_WORDS: &[&str] = &["card", "cards"];
const EXILE_FACE_DOWN_WORDS: &[&str] = &["face-down", "facedown"];
const EXILE_FACE_DOWN_TAIL: &[&str] = &["face", "down"];
const EXILE_HAND_OR_GRAVEYARD_WORDS: &[&str] = &["hand", "hands", "graveyard", "graveyards"];
const EXILE_GRAVEYARD_ZONE_WORDS: &[&str] = &["graveyard", "graveyards"];
const EXILE_LIBRARY_ZONE_WORDS: &[&str] = &["library", "libraries"];
const EXILE_OWNER_NONE_PHRASES: &[&[&str]] = &[];
const EXILE_OWNER_YOU_PHRASES: &[&[&str]] = &[&["your"]];
const EXILE_OWNER_THEIR_PHRASES: &[&[&str]] = &[&["their"]];
const EXILE_OWNER_THAT_PLAYER_PHRASES: &[&[&str]] = &[
    &["that", "player"],
    &["that", "players"],
    &["that", "player's"],
];
const EXILE_OWNER_TARGET_PLAYER_PHRASES: &[&[&str]] = &[
    &["target", "player"],
    &["target", "players"],
    &["target", "player's"],
];
const EXILE_OWNER_TARGET_OPPONENT_PHRASES: &[&[&str]] = &[
    &["target", "opponent"],
    &["target", "opponents"],
    &["target", "opponent's"],
];
const EXILE_OWNER_ITS_CONTROLLER_PHRASES: &[&[&str]] =
    &[&["its", "controller"], &["its", "controllers"]];
const EXILE_OWNER_ITS_OWNER_PHRASES: &[&[&str]] = &[&["its", "owner"], &["its", "owners"]];
const EXILE_OWNER_HIS_OR_HER_PHRASES: &[&[&str]] = &[&["his", "or", "her"]];
const EXILE_EACH_OPPONENT_LIBRARY_PREFIXES: &[&[&str]] = &[
    &["each", "opponent", "library"],
    &["each", "opponents", "library"],
    &["each", "opponent's", "library"],
    &["each", "opponent", "libraries"],
    &["each", "opponents", "libraries"],
    &["each", "opponent's", "libraries"],
];
const EXILE_THE_TOP_PREFIX: &[&str] = &["the", "top"];
const EXILE_WITH_THAT_NAME_PHRASE: &[&str] = &["with", "that", "name"];
const EXILE_CARD_FROM_THEIR_HAND_OR_PERMANENT_THEY_CONTROL_PHRASES: &[&[&str]] = &[
    &[
        "a",
        "card",
        "from",
        "their",
        "hand",
        "or",
        "a",
        "permanent",
        "they",
        "control",
    ],
    &[
        "a",
        "card",
        "from",
        "their",
        "hand",
        "or",
        "permanent",
        "they",
        "control",
    ],
    &[
        "card",
        "from",
        "their",
        "hand",
        "or",
        "a",
        "permanent",
        "they",
        "control",
    ],
    &[
        "card",
        "from",
        "their",
        "hand",
        "or",
        "permanent",
        "they",
        "control",
    ],
];

pub(crate) fn parse_each_opponent_exiles_card_from_their_hand_or_permanent_they_control(
    tokens: &[OwnedLexToken],
) -> Option<EffectAst> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    if words.len() < 4
        || words[0] != "each"
        || !matches!(words[1], "opponent" | "opponents")
        || !matches!(words[2], "exile" | "exiles")
    {
        return None;
    }
    let target_start = token_index_for_word_index(tokens, 3)?;
    parse_exile_card_from_their_hand_or_permanent_they_control(
        &tokens[target_start..],
        Some(SubjectAst::Player(PlayerAst::Opponent)),
    )
}

fn parse_exile_card_from_their_hand_or_permanent_they_control(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Option<EffectAst> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    if !EXILE_CARD_FROM_THEIR_HAND_OR_PERMANENT_THEY_CONTROL_PHRASES
        .iter()
        .any(|phrase| word_slice_eq(&words, phrase))
    {
        return None;
    }

    let wrap_for_each_opponent = matches!(subject, Some(SubjectAst::Player(PlayerAst::Opponent)));
    let chooser = match subject {
        Some(SubjectAst::Player(PlayerAst::That | PlayerAst::Opponent)) => PlayerAst::That,
        _ => return None,
    };

    let mut hand_card = ObjectFilter::default().in_zone(Zone::Hand);
    hand_card.owner = Some(PlayerFilter::IteratedPlayer);
    let mut permanent = ObjectFilter::permanent_card().in_zone(Zone::Battlefield);
    permanent.controller = Some(PlayerFilter::IteratedPlayer);

    let mut filter = ObjectFilter::default();
    filter.any_of = vec![hand_card, permanent];
    let tag = helper_tag_for_tokens(tokens, "exiled");
    let effects = vec![
        EffectAst::ChooseObjectsAcrossZones {
            filter,
            count: crate::effect::ChoiceCount::exactly(1),
            count_value: None,
            player: chooser,
            tag: tag.clone(),
            zones: vec![Zone::Hand, Zone::Battlefield],
            search_mode: None,
        },
        EffectAst::subject_verb_exile(TargetAst::Tagged(tag, None), false),
    ];

    Some(if wrap_for_each_opponent {
        EffectAst::ForEachOpponent { effects }
    } else {
        EffectAst::Sequence { effects }
    })
}

fn exile_token_is_word(token: &OwnedLexToken, expected: &str) -> bool {
    token.as_word().is_some_and(|word| word == expected)
}

fn exile_token_is_any_word(token: &OwnedLexToken, expected: &[&str]) -> bool {
    token.as_word().is_some_and(|word| expected.contains(&word))
}

#[derive(Clone, Copy)]
enum OwnerPrefixPlayer {
    Direct(PlayerAst),
    GraveyardTheir,
    LibraryDefault,
    LibraryTheirOrHisHer,
}

struct OwnerPrefixEntry {
    phrases: &'static [&'static [&'static str]],
    player: OwnerPrefixPlayer,
}

#[derive(Clone, Copy)]
pub(crate) struct ParsedOwnerPrefix {
    pub(crate) player: PlayerAst,
    pub(crate) consumed_words: usize,
}

const GRAVEYARD_OWNER_PREFIXES: &[OwnerPrefixEntry] = &[
    OwnerPrefixEntry {
        phrases: EXILE_OWNER_YOU_PHRASES,
        player: OwnerPrefixPlayer::Direct(PlayerAst::You),
    },
    OwnerPrefixEntry {
        phrases: EXILE_OWNER_THEIR_PHRASES,
        player: OwnerPrefixPlayer::GraveyardTheir,
    },
    OwnerPrefixEntry {
        phrases: EXILE_OWNER_THAT_PLAYER_PHRASES,
        player: OwnerPrefixPlayer::Direct(PlayerAst::That),
    },
    OwnerPrefixEntry {
        phrases: EXILE_OWNER_TARGET_PLAYER_PHRASES,
        player: OwnerPrefixPlayer::Direct(PlayerAst::Target),
    },
    OwnerPrefixEntry {
        phrases: EXILE_OWNER_TARGET_OPPONENT_PHRASES,
        player: OwnerPrefixPlayer::Direct(PlayerAst::TargetOpponent),
    },
    OwnerPrefixEntry {
        phrases: EXILE_OWNER_ITS_CONTROLLER_PHRASES,
        player: OwnerPrefixPlayer::Direct(PlayerAst::ItsController),
    },
    OwnerPrefixEntry {
        phrases: EXILE_OWNER_ITS_OWNER_PHRASES,
        player: OwnerPrefixPlayer::Direct(PlayerAst::ItsOwner),
    },
    OwnerPrefixEntry {
        phrases: EXILE_OWNER_HIS_OR_HER_PHRASES,
        player: OwnerPrefixPlayer::GraveyardTheir,
    },
];

const LIBRARY_OWNER_PREFIXES: &[OwnerPrefixEntry] = &[
    OwnerPrefixEntry {
        phrases: EXILE_OWNER_YOU_PHRASES,
        player: OwnerPrefixPlayer::Direct(PlayerAst::You),
    },
    OwnerPrefixEntry {
        phrases: EXILE_OWNER_THEIR_PHRASES,
        player: OwnerPrefixPlayer::LibraryTheirOrHisHer,
    },
    OwnerPrefixEntry {
        phrases: EXILE_OWNER_THAT_PLAYER_PHRASES,
        player: OwnerPrefixPlayer::Direct(PlayerAst::That),
    },
    OwnerPrefixEntry {
        phrases: EXILE_OWNER_TARGET_PLAYER_PHRASES,
        player: OwnerPrefixPlayer::Direct(PlayerAst::Target),
    },
    OwnerPrefixEntry {
        phrases: EXILE_OWNER_TARGET_OPPONENT_PHRASES,
        player: OwnerPrefixPlayer::Direct(PlayerAst::TargetOpponent),
    },
    OwnerPrefixEntry {
        phrases: EXILE_OWNER_ITS_CONTROLLER_PHRASES,
        player: OwnerPrefixPlayer::Direct(PlayerAst::ItsController),
    },
    OwnerPrefixEntry {
        phrases: EXILE_OWNER_ITS_OWNER_PHRASES,
        player: OwnerPrefixPlayer::Direct(PlayerAst::ItsOwner),
    },
    OwnerPrefixEntry {
        phrases: EXILE_OWNER_HIS_OR_HER_PHRASES,
        player: OwnerPrefixPlayer::LibraryTheirOrHisHer,
    },
    // Fallback: no explicit owner phrase. Must be tried last so an explicit owner
    // phrase wins; the empty-phrase entry always "matches" with zero consumed words.
    OwnerPrefixEntry {
        phrases: EXILE_OWNER_NONE_PHRASES,
        player: OwnerPrefixPlayer::LibraryDefault,
    },
];

fn owner_prefix_player(spec: OwnerPrefixPlayer, default_player: PlayerAst) -> PlayerAst {
    match spec {
        OwnerPrefixPlayer::Direct(player) => player,
        OwnerPrefixPlayer::GraveyardTheir => PlayerAst::That,
        OwnerPrefixPlayer::LibraryDefault => default_player,
        OwnerPrefixPlayer::LibraryTheirOrHisHer => {
            if matches!(default_player, PlayerAst::Implicit) {
                PlayerAst::ItsController
            } else {
                default_player
            }
        }
    }
}

fn parse_zone_owner_prefix_lexed(
    tokens: &[OwnedLexToken],
    entries: &[OwnerPrefixEntry],
    default_player: PlayerAst,
) -> Option<ParsedOwnerPrefix> {
    let clause = LexedClause::new(tokens);
    entries.iter().find_map(|entry| {
        let owner_atoms = [LexPattern::object(
            "owner",
            LexCaptureKind::OneOfPhrase(entry.phrases),
        )];
        let owner_range = if entry.phrases.is_empty() {
            0..0
        } else {
            let matched = LexPattern::new(&owner_atoms).match_prefix(clause)?;
            matched.capture_word_range("owner")?
        };
        Some(ParsedOwnerPrefix {
            player: owner_prefix_player(entry.player, default_player),
            consumed_words: owner_range.end,
        })
    })
}

/// `parse_zone_owner_prefix_lexed` reports `consumed_words` as the owner-phrase word
/// count only. The public zone-specific wrappers below restore the historical contract
/// where `consumed_words` also covers the trailing zone word: they require the owner
/// phrase to be immediately followed by exactly one matching zone word and bump the
/// count by one, so every caller (here and in resource verbs) keeps its original
/// "consumed up to and including the zone word" semantics.
fn parse_zone_owner_prefix_through_zone(
    tokens: &[OwnedLexToken],
    entries: &[OwnerPrefixEntry],
    zone_words: &[&str],
    default_player: PlayerAst,
) -> Option<ParsedOwnerPrefix> {
    let owner = parse_zone_owner_prefix_lexed(tokens, entries, default_player)?;
    let words = crate::runtime_backend::token_word_refs(tokens);
    if !words
        .get(owner.consumed_words)
        .is_some_and(|word| zone_words.contains(word))
    {
        return None;
    }
    Some(ParsedOwnerPrefix {
        player: owner.player,
        consumed_words: owner.consumed_words + 1,
    })
}

pub(crate) fn parse_exile(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let (tokens, until_source_leaves) = split_until_source_leaves_tail(tokens);
    let (tokens, face_down) = split_exile_face_down_suffix(tokens);
    let tokens = split_exile_graveyard_replacement_suffix(tokens);
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if grammar::contains_word(tokens, "unless") {
        return Err(CardTextError::ParseError(format!(
            "unsupported exile-unless clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    let has_face_down_manifest_tail = (grammar::contains_word(tokens, "face-down")
        || grammar::contains_word(tokens, "facedown")
        || grammar::contains_word(tokens, "manifest")
        || grammar::contains_word(tokens, "pile"))
        && grammar::contains_word(tokens, "then");
    if has_face_down_manifest_tail {
        return Err(CardTextError::ParseError(format!(
            "unsupported face-down/manifest exile clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    if let Some(effect) = parse_same_name_exile_hand_and_graveyard_clause(
        tokens,
        subject,
        until_source_leaves,
        face_down,
    )? {
        return Ok(effect);
    }
    if !face_down
        && !until_source_leaves
        && let Some(effect) =
            parse_exile_card_from_their_hand_or_permanent_they_control(tokens, subject)
    {
        return Ok(effect);
    }
    if clause_words
        .first()
        .is_some_and(|word| EXILE_ALL_OR_EACH_WORDS.contains(word))
    {
        let filter_tokens = &tokens[1..];
        let mut filter = parse_object_filter_lexed(filter_tokens, false)?;
        apply_exile_subject_owner_context(&mut filter, subject);
        return Ok(if until_source_leaves {
            EffectAst::subject_verb_exile_all_until_source_leaves(
                TargetAst::Object(filter, None, None),
                face_down,
            )
        } else {
            EffectAst::subject_verb_exile_all(filter, face_down)
        });
    }
    if let Some(filter) = parse_target_player_graveyard_filter(tokens) {
        return Ok(if until_source_leaves {
            EffectAst::subject_verb_exile_until_source_leaves(
                TargetAst::Object(filter, None, None),
                face_down,
            )
        } else {
            EffectAst::subject_verb_exile_all(filter, face_down)
        });
    }
    if let Some(effect) =
        parse_mixed_target_and_all_exile_list(tokens, subject, until_source_leaves, face_down)?
    {
        return Ok(effect);
    }
    if !until_source_leaves
        && let Some(effect) = parse_exile_bottom_library_clause(tokens, subject, face_down)
    {
        return Ok(effect);
    }
    if !face_down
        && !until_source_leaves
        && let Some(effect) = parse_exile_dynamic_count_from_top_library_clause(tokens, subject)
    {
        return Ok(effect);
    }
    if !face_down
        && !until_source_leaves
        && let Some(effect) = parse_exile_top_library_clause(tokens, subject)
    {
        return Ok(effect);
    }

    if grammar::contains_word(tokens, "dealt")
        && grammar::contains_word(tokens, "damage")
        && grammar::contains_word(tokens, "turn")
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported combat-history exile clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    let has_until_total_mana_value = grammar::contains_word(tokens, "until")
        && grammar::contains_word(tokens, "exiled")
        && grammar::contains_word(tokens, "total")
        && grammar::contains_word(tokens, "mana")
        && grammar::contains_word(tokens, "value");
    if has_until_total_mana_value {
        return Err(CardTextError::ParseError(format!(
            "unsupported iterative exile-total clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    if let Some(effect) = parse_attached_object_exile_bundle(tokens, face_down)? {
        return Ok(effect);
    }
    let has_same_name_token_bundle = grammar::contains_word(tokens, "and")
        && grammar::contains_word(tokens, "tokens")
        && grammar::contains_word(tokens, "same")
        && grammar::contains_word(tokens, "name");
    if has_same_name_token_bundle {
        return Err(CardTextError::ParseError(format!(
            "unsupported same-name token exile bundle (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    if let Some((before_and, after_and)) =
        crate::runtime_backend::grammar::primitives::split_lexed_once_on_separator(tokens, || {
            use winnow::Parser as _;
            crate::runtime_backend::grammar::primitives::kw("and").void()
        })
        && !before_and.is_empty()
    {
        let starts_multi_target = after_and
            .first()
            .is_some_and(|token| exile_token_is_word(token, "target"))
            || (crate::runtime_backend::grammar::primitives::strip_lexed_prefix_phrase(
                after_and,
                &["up", "to"],
            )
            .is_some()
                && crate::runtime_backend::grammar::primitives::contains_word(after_and, "target"));
        if starts_multi_target {
            return Err(CardTextError::ParseError(format!(
                "unsupported multi-target exile clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
    }

    if let Some(spec) = split_trailing_if_clause_lexed(tokens) {
        let mut target = parse_target_phrase(spec.leading_tokens)?;
        apply_exile_subject_hand_owner_context(&mut target, subject);
        return Ok(EffectAst::Conditional {
            predicate: spec.predicate,
            if_true: vec![if until_source_leaves {
                EffectAst::subject_verb_exile_until_source_leaves(target, face_down)
            } else {
                EffectAst::subject_verb_exile(target, face_down)
            }],
            if_false: Vec::new(),
        });
    } else if grammar::contains_word(tokens, "if") {
        return Err(CardTextError::ParseError(format!(
            "unsupported conditional exile clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let mut target = parse_target_phrase(tokens)?;
    apply_exile_subject_hand_owner_context(&mut target, subject);
    Ok(if until_source_leaves {
        EffectAst::subject_verb_exile_until_source_leaves(target, face_down)
    } else {
        EffectAst::subject_verb_exile(target, face_down)
    })
}

#[rustfmt::skip]
fn parse_attached_object_exile_bundle(
    tokens: &[OwnedLexToken],
    face_down: bool,
) -> Result<Option<EffectAst>, CardTextError> {
    let Some((target_tokens, attached_tokens)) =
        crate::runtime_backend::grammar::primitives::split_lexed_once_on_separator(tokens, || {
            use winnow::Parser as _;
            crate::runtime_backend::grammar::primitives::kw("and").void()
        })
    else {
        return Ok(None);
    };
    let Some(attached_tokens) =
        crate::runtime_backend::grammar::primitives::strip_lexed_prefix_phrase(
            attached_tokens,
            &["all"],
        )
    else {
        return Ok(None);
    };
    let Some(attached_idx) = attached_tokens
        .iter()
        .position(|token| token.as_word().is_some_and(|word| word == "attached"))
    else {
        return Ok(None);
    };
    if !attached_tokens
        .get(attached_idx + 1)
        .is_some_and(|token| token.as_word().is_some_and(|word| word == "to"))
    {
        return Ok(None);
    }
    let attachment_filter_tokens = &attached_tokens[..attached_idx];
    let attachment_target_tokens = &attached_tokens[attached_idx + 2..];
    if target_tokens.is_empty()
        || attachment_filter_tokens.is_empty()
        || attachment_target_tokens.is_empty()
    {
        return Ok(None);
    }
    if !word_slice_eq(&crate::runtime_backend::token_word_refs(attachment_target_tokens), &["it"]) {
        return Ok(None);
    }

    let target = parse_target_phrase(target_tokens)?;
    let attachment_filter = parse_object_filter_lexed(attachment_filter_tokens, false)?;
    Ok(Some(EffectAst::subject_verb_exile_all_attached_to(
        attachment_filter,
        target,
        face_down,
    )))
}

pub(crate) fn parse_same_name_exile_hand_and_graveyard_clause(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
    until_source_leaves: bool,
    face_down: bool,
) -> Result<Option<EffectAst>, CardTextError> {
    if grammar::words_match_any_prefix(tokens, ALL_CARD_PREFIXES).is_none()
        || !word_slice_contains_phrase(
            &crate::runtime_backend::token_word_refs(tokens),
            EXILE_WITH_THAT_NAME_PHRASE,
        )
    {
        return Ok(None);
    }
    let clause_words = crate::runtime_backend::token_word_refs(tokens);

    let Some(from_idx) = find_index(&clause_words, |word| *word == "from") else {
        return Ok(None);
    };
    let Some(first_zone_idx) = find_index(&clause_words[from_idx + 1..], |word| {
        EXILE_HAND_OR_GRAVEYARD_WORDS.contains(word)
    })
    .map(|offset| from_idx + 1 + offset) else {
        return Ok(None);
    };

    let owner_words = &clause_words[from_idx + 1..first_zone_idx];
    let owner_from_subject = match subject {
        Some(SubjectAst::Player(player)) => controller_filter_for_token_player(player),
        _ => None,
    };
    let owner = match owner_words {
        ["target", "player"] | ["target", "players"] => Some(PlayerFilter::target_player()),
        ["target", "opponent"] | ["target", "opponents"] => Some(PlayerFilter::target_opponent()),
        ["that", "player"] | ["that", "players"] => Some(PlayerFilter::IteratedPlayer),
        ["your"] => Some(PlayerFilter::You),
        ["their"] | ["his", "or", "her"] => {
            owner_from_subject.or(Some(PlayerFilter::IteratedPlayer))
        }
        [] => owner_from_subject,
        _ => return Ok(None),
    };
    let Some(owner) = owner else {
        return Ok(None);
    };

    let mut zones = Vec::new();
    for word in &clause_words[first_zone_idx..] {
        let Some(zone) = parse_zone_word(word) else {
            continue;
        };
        if !matches!(zone, Zone::Hand | Zone::Graveyard) || slice_contains(&zones, &zone) {
            continue;
        }
        zones.push(zone);
    }
    if zones.len() != 2
        || !slice_contains(&zones, &Zone::Hand)
        || !slice_contains(&zones, &Zone::Graveyard)
    {
        return Ok(None);
    }

    let mut filter = ObjectFilter::default();
    filter.owner = Some(owner);
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: TagKey::from(IT_TAG),
        relation: TaggedOpbjectRelation::SameNameAsTagged,
    });
    filter.any_of = zones
        .into_iter()
        .map(|zone| ObjectFilter::default().in_zone(zone))
        .collect();

    Ok(Some(if until_source_leaves {
        EffectAst::subject_verb_exile_until_source_leaves(
            TargetAst::Object(filter, None, None),
            face_down,
        )
    } else {
        EffectAst::subject_verb_exile_all(filter, face_down)
    }))
}

fn strip_exile_list_segment_leading_conjunction(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    let trimmed = trim_commas(tokens);
    if trimmed
        .first()
        .is_some_and(|token| token.as_word().is_some_and(|word| word == "and"))
    {
        trim_commas(&trimmed[1..])
    } else {
        trimmed
    }
}

fn split_exile_all_list_tail_segment(tokens: Vec<OwnedLexToken>) -> Vec<Vec<OwnedLexToken>> {
    let mut parts = Vec::new();
    let mut start = 0usize;
    let mut idx = 0usize;
    while idx + 1 < tokens.len() {
        let is_and = tokens[idx].as_word().is_some_and(|word| word == "and");
        let next_starts_all_clause = tokens[idx + 1]
            .as_word()
            .is_some_and(|word| EXILE_ALL_OR_EACH_WORDS.contains(&word));
        if is_and && next_starts_all_clause {
            let part = trim_commas(&tokens[start..idx]);
            if !part.is_empty() {
                parts.push(part);
            }
            start = idx + 1;
            idx += 1;
        }
        idx += 1;
    }
    let part = trim_commas(&tokens[start..]);
    if !part.is_empty() {
        parts.push(part);
    }
    parts
}

fn parse_mixed_target_and_all_exile_list(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
    until_source_leaves: bool,
    face_down: bool,
) -> Result<Option<EffectAst>, CardTextError> {
    let segments = crate::runtime_backend::grammar::primitives::split_lexed_slices_on_comma(tokens);
    if segments.len() < 2 {
        return Ok(None);
    }
    let first_segment = trim_commas(segments[0]);
    if first_segment.is_empty() {
        return Ok(None);
    }
    let mut all_segments = Vec::new();
    for segment in segments.iter().skip(1) {
        let segment = strip_exile_list_segment_leading_conjunction(segment);
        for segment in split_exile_all_list_tail_segment(segment) {
            if segment.is_empty() {
                return Ok(None);
            }
            let Some(first_word) = segment.first().and_then(|token| token.as_word()) else {
                return Ok(None);
            };
            if !EXILE_ALL_OR_EACH_WORDS.contains(&first_word) {
                return Ok(None);
            }
            all_segments.push(segment);
        }
    }

    let mut effects = Vec::new();
    let mut target = parse_target_phrase(&first_segment)?;
    apply_exile_subject_hand_owner_context(&mut target, subject);
    effects.push(if until_source_leaves {
        EffectAst::subject_verb_exile_until_source_leaves(target, face_down)
    } else {
        EffectAst::subject_verb_exile(target, face_down)
    });

    for segment in all_segments {
        let filter_tokens = trim_commas(&segment[1..]);
        if filter_tokens.is_empty() {
            return Ok(None);
        }
        let mut filter = parse_object_filter_lexed(&filter_tokens, false)?;
        apply_exile_subject_owner_context(&mut filter, subject);
        effects.push(if until_source_leaves {
            EffectAst::subject_verb_exile_all_until_source_leaves(
                TargetAst::Object(filter, None, None),
                face_down,
            )
        } else {
            EffectAst::subject_verb_exile_all(filter, face_down)
        });
    }

    Ok(Some(EffectAst::Sequence { effects }))
}

pub(crate) fn split_exile_face_down_suffix(tokens: &[OwnedLexToken]) -> (&[OwnedLexToken], bool) {
    if tokens.is_empty() {
        return (tokens, false);
    }

    let mut end = tokens.len();
    while end > 0 && tokens[end - 1].is_comma() {
        end -= 1;
    }
    if end > 0 && exile_token_is_word(&tokens[end - 1], "instead") {
        end -= 1;
        while end > 0 && tokens[end - 1].is_comma() {
            end -= 1;
        }
    }

    if end > 0 && exile_token_is_any_word(&tokens[end - 1], EXILE_FACE_DOWN_WORDS) {
        return (&tokens[..end - 1], true);
    }

    if end >= 2
        && word_slice_eq(
            &crate::runtime_backend::token_word_refs(&tokens[end - 2..end]),
            EXILE_FACE_DOWN_TAIL,
        )
    {
        return (&tokens[..end - 2], true);
    }

    (tokens, false)
}

pub(crate) fn split_exile_graveyard_replacement_suffix(
    tokens: &[OwnedLexToken],
) -> &[OwnedLexToken] {
    use crate::runtime_backend::grammar::primitives as grammar;

    let Some((main_slice, tail_slice)) = grammar::split_lexed_once_on_separator(tokens, || {
        use winnow::Parser as _;
        grammar::kw("instead").void()
    }) else {
        return tokens;
    };
    if main_slice.is_empty() {
        return tokens;
    }

    let is_graveyard_replacement =
        grammar::strip_lexed_prefix_phrase(tail_slice, &["of", "putting"]).is_some()
            && (grammar::contains_word(tail_slice, "graveyard")
                || grammar::contains_word(tail_slice, "graveyards"));
    if is_graveyard_replacement {
        main_slice
    } else {
        tokens
    }
}

pub(crate) fn parse_graveyard_owner_prefix_lexed(
    tokens: &[OwnedLexToken],
) -> Option<ParsedOwnerPrefix> {
    parse_zone_owner_prefix_through_zone(
        tokens,
        GRAVEYARD_OWNER_PREFIXES,
        EXILE_GRAVEYARD_ZONE_WORDS,
        PlayerAst::Implicit,
    )
}

fn parse_library_owner_prefix_lexed(
    tokens: &[OwnedLexToken],
    default_player: PlayerAst,
) -> Option<ParsedOwnerPrefix> {
    parse_zone_owner_prefix_through_zone(
        tokens,
        LIBRARY_OWNER_PREFIXES,
        EXILE_LIBRARY_ZONE_WORDS,
        default_player,
    )
}

fn parse_exile_dynamic_count_from_top_library_clause(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Option<EffectAst> {
    let tokens = trim_commas(tokens);
    let words = crate::runtime_backend::token_word_refs(&tokens);
    let starts_with_that_many_cards = words.len() >= 3
        && words[0] == "that"
        && words[1] == "many"
        && EXILE_CARD_OR_CARDS_WORDS.contains(&words[2]);
    if !starts_with_that_many_cards
        && !words
            .first()
            .is_some_and(|word| EXILE_CARD_OR_CARDS_WORDS.contains(word))
    {
        return None;
    }

    let (count, from_word_idx) = if words.len() >= 4
        && words[0] == "that"
        && words[1] == "many"
        && EXILE_CARD_OR_CARDS_WORDS.contains(&words[2])
        && words[3] == "from"
    {
        (Value::EventValue(EventValueSpec::Amount), 3)
    } else {
        let from_word_idx = find_index(&words, |word| **word == *"from")?;
        if from_word_idx <= 1 {
            return None;
        }
        let count_start = token_index_for_word_index(&tokens, 1)?;
        let from_token_idx = token_index_for_word_index(&tokens, from_word_idx)?;
        let count_tokens = trim_commas(&tokens[count_start..from_token_idx]);
        let count = crate::runtime_backend::front_end::grammar::values::parse_add_mana_equal_amount_value_lexed(
            &count_tokens,
        )?;
        (count, from_word_idx)
    };

    let after_from = &words[from_word_idx + 1..];
    let owner_word_idx = if after_from.len() >= 3
        && after_from[0] == "the"
        && after_from[1] == "top"
        && after_from[2] == "of"
    {
        from_word_idx + 1 + 3
    } else if after_from.len() >= 2 && after_from[0] == "top" && after_from[1] == "of" {
        from_word_idx + 1 + 2
    } else {
        return None;
    };

    let owner_start = token_index_for_word_index(&tokens, owner_word_idx)?;
    let owner_tokens = trim_commas(&tokens[owner_start..]);
    let default_player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);
    let owner = parse_library_owner_prefix_lexed(&owner_tokens, default_player)?;
    if owner.consumed_words < crate::runtime_backend::token_word_refs(&owner_tokens).len() {
        return None;
    }

    Some(EffectAst::subject_verb_exile_top_of_library(
        owner.player,
        count,
        vec![helper_tag_for_tokens(&tokens, "exiled")],
        Vec::new(),
    ))
}

pub(crate) fn parse_exile_top_library_clause(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Option<EffectAst> {
    let tokens = trim_commas(tokens);
    let words = crate::runtime_backend::token_word_refs(&tokens);
    let mut start = 0usize;
    if word_slice_starts_with(&words, EXILE_THE_TOP_PREFIX) {
        start = 1;
    }
    if !words.get(start).is_some_and(|word| *word == "top") {
        return None;
    }

    let count_start = token_index_for_word_index(&tokens, start + 1)?;
    let count_start_words =
        crate::runtime_backend::token_word_refs(&tokens[count_start..=count_start]);
    let (count, used_after_top, count_was_implicit) = if count_start_words
        .first()
        .is_some_and(|word| EXILE_CARD_OR_CARDS_WORDS.contains(word))
    {
        (Value::Fixed(1), 0, true)
    } else {
        let (count, used_after_top) = parse_value(&tokens[count_start..])?;
        (count, used_after_top, false)
    };
    let after_count = trim_commas(&tokens[count_start + used_after_top..]);
    let after_count_words = crate::runtime_backend::token_word_refs(&after_count);
    if !after_count_words
        .first()
        .is_some_and(|word| EXILE_CARD_OR_CARDS_WORDS.contains(word))
    {
        return None;
    }

    let after_cards_start = token_index_for_word_index(&after_count, 1)?;
    let after_cards = trim_commas(&after_count[after_cards_start..]);
    let after_cards_words = crate::runtime_backend::token_word_refs(&after_cards);
    if !after_cards_words.first().is_some_and(|word| *word == "of") {
        return None;
    }

    let owner_tokens = trim_commas(&after_cards[1..]);
    if word_slice_starts_with_any(
        &crate::runtime_backend::token_word_refs(&owner_tokens),
        EXILE_EACH_OPPONENT_LIBRARY_PREFIXES,
    ) {
        return Some(EffectAst::ForEachOpponent {
            effects: vec![EffectAst::subject_verb_exile_top_of_library(
                PlayerAst::That,
                count,
                Vec::new(),
                vec![helper_tag_for_tokens(&tokens, "exiled")],
            )],
        });
    }

    // An implicit single-card count ("exile the top card of <owner> library") with a
    // named owner belongs to the dedicated exile-top-then-cast/play parser (e.g. Mind's
    // Dilation, Urabrask). Only the each-opponent shape above keeps the implicit count.
    if count_was_implicit {
        return None;
    }

    let default_player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);
    let owner = parse_library_owner_prefix_lexed(&owner_tokens, default_player)?;
    if owner.consumed_words < LexedClause::new(&owner_tokens).word_refs().len() {
        return None;
    }

    Some(EffectAst::subject_verb_exile_top_of_library(
        owner.player,
        count,
        vec![helper_tag_for_tokens(&tokens, "exiled")],
        Vec::new(),
    ))
}

fn parse_exile_bottom_library_clause(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
    face_down: bool,
) -> Option<EffectAst> {
    let tokens = trim_commas(tokens);
    let words = crate::runtime_backend::token_word_refs(&tokens);
    let prefix = ["the", "bottom"];
    let mut start = 0usize;
    if words.starts_with(&prefix) {
        start = 1;
    }
    if !words.get(start).is_some_and(|word| *word == "bottom") {
        return None;
    }
    let count_start = token_index_for_word_index(&tokens, start + 1)?;
    let count_start_words =
        crate::runtime_backend::token_word_refs(&tokens[count_start..=count_start]);
    let (count, used_after_bottom) = if count_start_words
        .first()
        .is_some_and(|word| EXILE_CARD_OR_CARDS_WORDS.contains(word))
    {
        (Value::Fixed(1), 0)
    } else {
        parse_value(&tokens[count_start..])?
    };
    if count != Value::Fixed(1) {
        return None;
    }
    let after_count = trim_commas(&tokens[count_start + used_after_bottom..]);
    let after_count_words = crate::runtime_backend::token_word_refs(&after_count);
    if !after_count_words
        .first()
        .is_some_and(|word| EXILE_CARD_OR_CARDS_WORDS.contains(word))
    {
        return None;
    }
    let after_cards_start = token_index_for_word_index(&after_count, 1)?;
    let after_cards = trim_commas(&after_count[after_cards_start..]);
    let after_cards_words = crate::runtime_backend::token_word_refs(&after_cards);
    if !after_cards_words.first().is_some_and(|word| *word == "of") {
        return None;
    }
    let owner_tokens = trim_commas(&after_cards[1..]);
    let tag = helper_tag_for_tokens(&tokens, "exiled");
    let mut filter = ObjectFilter::default();
    filter.zone = Some(Zone::Library);

    let choose_and_exile = |player: PlayerAst, tag: TagKey| {
        vec![
            EffectAst::ChooseObjectsBottomOfLibrary {
                filter: filter.clone(),
                count: crate::effect::ChoiceCount::exactly(1),
                count_value: None,
                player,
                tag: tag.clone(),
            },
            EffectAst::subject_verb_exile(TargetAst::Tagged(tag, None), face_down),
        ]
    };

    if word_slice_starts_with_any(
        &crate::runtime_backend::token_word_refs(&owner_tokens),
        EXILE_EACH_OPPONENT_LIBRARY_PREFIXES,
    ) {
        return Some(EffectAst::ForEachOpponent {
            effects: choose_and_exile(PlayerAst::That, tag),
        });
    }

    let default_player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);
    let owner = parse_library_owner_prefix_lexed(&owner_tokens, default_player)?;
    if owner.consumed_words < LexedClause::new(&owner_tokens).word_refs().len() {
        return None;
    }

    Some(EffectAst::Sequence {
        effects: choose_and_exile(owner.player, tag),
    })
}

pub(crate) fn parse_target_player_graveyard_filter(
    tokens: &[OwnedLexToken],
) -> Option<ObjectFilter> {
    let tokens = trim_commas(tokens);
    let owner = parse_graveyard_owner_prefix_lexed(&tokens)?;
    if owner.consumed_words != LexedClause::new(&tokens).word_refs().len() {
        return None;
    }

    let mut filter = ObjectFilter::default().in_zone(Zone::Graveyard);
    filter.owner = match owner.player {
        PlayerAst::You => Some(PlayerFilter::You),
        PlayerAst::That => Some(PlayerFilter::IteratedPlayer),
        PlayerAst::Target => Some(PlayerFilter::target_player()),
        PlayerAst::TargetOpponent => Some(PlayerFilter::Target(Box::new(PlayerFilter::Opponent))),
        PlayerAst::ItsController => Some(PlayerFilter::ControllerOf(
            crate::filter::ObjectRef::tagged("triggering"),
        )),
        PlayerAst::ItsOwner => Some(PlayerFilter::OwnerOf(crate::filter::ObjectRef::tagged(
            "triggering",
        ))),
        _ => None,
    };
    filter.owner.as_ref()?;
    Some(filter)
}
