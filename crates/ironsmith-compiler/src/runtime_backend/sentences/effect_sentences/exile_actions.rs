use super::*;
use crate::runtime_backend::lex_patterns::{LexCaptureKind, LexPattern};
use crate::runtime_backend::lexer::{
    LexedClause, word_slice_contains_phrase, word_slice_eq, word_slice_starts_with,
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
const EXILE_OWNER_EACH_OPPONENT_PHRASES: &[&[&str]] = &[
    &["each", "opponent"],
    &["each", "opponents"],
    &["each", "opponent's"],
];
const EXILE_EACH_OPPONENT_LIBRARY_PATTERN: LexPattern<'static> = LexPattern::new(&[
    LexPattern::subject(
        "owner",
        LexCaptureKind::OneOfPhrase(EXILE_OWNER_EACH_OPPONENT_PHRASES),
    ),
    LexPattern::object("zone", LexCaptureKind::OneOf(EXILE_LIBRARY_ZONE_WORDS)),
]);
const EXILE_THE_TOP_PREFIX: &[&str] = &["the", "top"];
const EXILE_WITH_THAT_NAME_PHRASE: &[&str] = &["with", "that", "name"];

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
    owner_phrases: &'static [&'static [&'static str]],
    player: OwnerPrefixPlayer,
}

#[derive(Clone, Copy)]
pub(crate) struct ParsedOwnerPrefix {
    pub(crate) player: PlayerAst,
    pub(crate) consumed_words: usize,
}

const GRAVEYARD_OWNER_PREFIXES: &[OwnerPrefixEntry] = &[
    OwnerPrefixEntry {
        owner_phrases: EXILE_OWNER_YOU_PHRASES,
        player: OwnerPrefixPlayer::Direct(PlayerAst::You),
    },
    OwnerPrefixEntry {
        owner_phrases: EXILE_OWNER_THEIR_PHRASES,
        player: OwnerPrefixPlayer::GraveyardTheir,
    },
    OwnerPrefixEntry {
        owner_phrases: EXILE_OWNER_THAT_PLAYER_PHRASES,
        player: OwnerPrefixPlayer::Direct(PlayerAst::That),
    },
    OwnerPrefixEntry {
        owner_phrases: EXILE_OWNER_TARGET_PLAYER_PHRASES,
        player: OwnerPrefixPlayer::Direct(PlayerAst::Target),
    },
    OwnerPrefixEntry {
        owner_phrases: EXILE_OWNER_TARGET_OPPONENT_PHRASES,
        player: OwnerPrefixPlayer::Direct(PlayerAst::TargetOpponent),
    },
    OwnerPrefixEntry {
        owner_phrases: EXILE_OWNER_ITS_CONTROLLER_PHRASES,
        player: OwnerPrefixPlayer::Direct(PlayerAst::ItsController),
    },
    OwnerPrefixEntry {
        owner_phrases: EXILE_OWNER_ITS_OWNER_PHRASES,
        player: OwnerPrefixPlayer::Direct(PlayerAst::ItsOwner),
    },
    OwnerPrefixEntry {
        owner_phrases: EXILE_OWNER_HIS_OR_HER_PHRASES,
        player: OwnerPrefixPlayer::GraveyardTheir,
    },
];

const LIBRARY_OWNER_PREFIXES: &[OwnerPrefixEntry] = &[
    OwnerPrefixEntry {
        owner_phrases: EXILE_OWNER_NONE_PHRASES,
        player: OwnerPrefixPlayer::LibraryDefault,
    },
    OwnerPrefixEntry {
        owner_phrases: EXILE_OWNER_YOU_PHRASES,
        player: OwnerPrefixPlayer::Direct(PlayerAst::You),
    },
    OwnerPrefixEntry {
        owner_phrases: EXILE_OWNER_THEIR_PHRASES,
        player: OwnerPrefixPlayer::LibraryTheirOrHisHer,
    },
    OwnerPrefixEntry {
        owner_phrases: EXILE_OWNER_THAT_PLAYER_PHRASES,
        player: OwnerPrefixPlayer::Direct(PlayerAst::That),
    },
    OwnerPrefixEntry {
        owner_phrases: EXILE_OWNER_TARGET_PLAYER_PHRASES,
        player: OwnerPrefixPlayer::Direct(PlayerAst::Target),
    },
    OwnerPrefixEntry {
        owner_phrases: EXILE_OWNER_TARGET_OPPONENT_PHRASES,
        player: OwnerPrefixPlayer::Direct(PlayerAst::TargetOpponent),
    },
    OwnerPrefixEntry {
        owner_phrases: EXILE_OWNER_ITS_CONTROLLER_PHRASES,
        player: OwnerPrefixPlayer::Direct(PlayerAst::ItsController),
    },
    OwnerPrefixEntry {
        owner_phrases: EXILE_OWNER_ITS_OWNER_PHRASES,
        player: OwnerPrefixPlayer::Direct(PlayerAst::ItsOwner),
    },
    OwnerPrefixEntry {
        owner_phrases: EXILE_OWNER_HIS_OR_HER_PHRASES,
        player: OwnerPrefixPlayer::LibraryTheirOrHisHer,
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
    zone_words: &'static [&'static str],
    default_player: PlayerAst,
) -> Option<ParsedOwnerPrefix> {
    let clause = LexedClause::new(tokens);
    entries.iter().find_map(|entry| {
        let zone_atom = [LexPattern::object(
            "zone",
            LexCaptureKind::OneOf(zone_words),
        )];
        let owned_zone_atoms = [
            LexPattern::subject("owner", LexCaptureKind::OneOfPhrase(entry.owner_phrases)),
            LexPattern::object("zone", LexCaptureKind::OneOf(zone_words)),
        ];
        let atoms = if entry.owner_phrases.is_empty() {
            zone_atom.as_slice()
        } else {
            owned_zone_atoms.as_slice()
        };
        let matched = LexPattern::new(atoms).match_prefix(clause)?;
        let zone_range = matched.capture_word_range("zone")?;
        Some(ParsedOwnerPrefix {
            player: owner_prefix_player(entry.player, default_player),
            consumed_words: zone_range.end,
        })
    })
}

fn parse_zone_owner_prefix_words(
    words: &[&str],
    entries: &[OwnerPrefixEntry],
    zone_words: &[&str],
    default_player: PlayerAst,
) -> Option<(PlayerAst, usize)> {
    entries.iter().find_map(|entry| {
        if entry.owner_phrases.is_empty() {
            return words.first().and_then(|word| {
                zone_words
                    .contains(word)
                    .then_some((owner_prefix_player(entry.player, default_player), 1))
            });
        }
        entry.owner_phrases.iter().find_map(|owner_phrase| {
            let zone_idx = owner_phrase.len();
            words
                .get(zone_idx)
                .is_some_and(|word| words.starts_with(owner_phrase) && zone_words.contains(word))
                .then(|| {
                    (
                        owner_prefix_player(entry.player, default_player),
                        owner_phrase.len() + 1,
                    )
                })
        })
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
    if !until_source_leaves
        && let Some(effect) = parse_exile_bottom_library_clause(tokens, subject, face_down)
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
    if !word_slice_eq(
        &crate::runtime_backend::token_word_refs(attachment_target_tokens),
        &["it"],
    ) {
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

pub(crate) fn parse_graveyard_owner_prefix(words: &[&str]) -> Option<(PlayerAst, usize)> {
    parse_zone_owner_prefix_words(
        words,
        GRAVEYARD_OWNER_PREFIXES,
        EXILE_GRAVEYARD_ZONE_WORDS,
        PlayerAst::Implicit,
    )
}

pub(crate) fn parse_graveyard_owner_prefix_lexed(
    tokens: &[OwnedLexToken],
) -> Option<ParsedOwnerPrefix> {
    parse_zone_owner_prefix_lexed(
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
    parse_zone_owner_prefix_lexed(
        tokens,
        LIBRARY_OWNER_PREFIXES,
        EXILE_LIBRARY_ZONE_WORDS,
        default_player,
    )
}

fn exile_owner_prefix_is_each_opponent_library(tokens: &[OwnedLexToken]) -> bool {
    EXILE_EACH_OPPONENT_LIBRARY_PATTERN
        .match_prefix(LexedClause::new(tokens))
        .is_some()
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
    let (count, used_after_top) = parse_value(&tokens[count_start..])?;
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
    if exile_owner_prefix_is_each_opponent_library(&owner_tokens) {
        return Some(EffectAst::ForEachOpponent {
            effects: vec![EffectAst::subject_verb_exile_top_of_library(
                PlayerAst::That,
                count,
                vec![helper_tag_for_tokens(&tokens, "exiled")],
                Vec::new(),
            )],
        });
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

    if exile_owner_prefix_is_each_opponent_library(&owner_tokens) {
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
    let owner = parse_graveyard_owner_prefix_lexed(tokens)?;
    if owner.consumed_words != LexedClause::new(tokens).word_refs().len() {
        return None;
    }

    let mut filter = ObjectFilter::default().in_zone(Zone::Graveyard);
    filter.owner = match owner.player {
        PlayerAst::You => Some(PlayerFilter::You),
        PlayerAst::That | PlayerAst::Target => Some(PlayerFilter::target_player()),
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
