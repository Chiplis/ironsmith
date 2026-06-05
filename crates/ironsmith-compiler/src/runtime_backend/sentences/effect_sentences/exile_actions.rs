use super::super::clause_pattern_helpers::{ClauseShape, clause_shape};
use super::*;
const EXILE_TARGET_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["target"]);
const EXILE_FROM_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["from"]);
const EXILE_INSTEAD_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["instead"]);
const EXILE_ALL_OR_EACH_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["all"], &["each"]]);
const EXILE_TOP_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["top"]);
const EXILE_CARD_OR_CARDS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["card"], &["cards"]]);
const EXILE_OF_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["of"]);
const EXILE_FACE_DOWN_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["face-down"], &["facedown"]]);
const EXILE_FACE_DOWN_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["face", "down"]);
const EXILE_HAND_OR_GRAVEYARD_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["hand"], &["hands"], &["graveyard"], &["graveyards"]]);
const EXILE_IT_REFERENCE_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["it"]);
const EXILE_GRAVEYARD_OWNER_YOU_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["your", "graveyard"]);
const EXILE_GRAVEYARD_OWNER_THEIR_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["their", "graveyard"]);
const EXILE_GRAVEYARD_OWNER_THAT_PLAYER_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["that", "player", "graveyard"],
            &["that", "players", "graveyard"]
        ]
);
const EXILE_GRAVEYARD_OWNER_TARGET_PLAYER_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["target", "player", "graveyard"],
            &["target", "players", "graveyard"]
        ]
);
const EXILE_GRAVEYARD_OWNER_TARGET_OPPONENT_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["target", "opponent", "graveyard"],
            &["target", "opponents", "graveyard"]
        ]
);
const EXILE_GRAVEYARD_OWNER_ITS_CONTROLLER_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["its", "controller", "graveyard"],
            &["its", "controllers", "graveyard"]
        ]
);
const EXILE_GRAVEYARD_OWNER_ITS_OWNER_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["its", "owner", "graveyard"],
            &["its", "owners", "graveyard"]
        ]
);
const EXILE_GRAVEYARD_OWNER_HIS_OR_HER_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["his", "or", "her", "graveyard"]);
const EXILE_LIBRARY_OWNER_DEFAULT_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["library"]);
const EXILE_LIBRARY_OWNER_YOU_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["your", "library"]);
const EXILE_LIBRARY_OWNER_THEIR_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["their", "library"]);
const EXILE_LIBRARY_OWNER_THAT_PLAYER_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["that", "player", "library"],
            &["that", "players", "library"],
            &["that", "player's", "library"]
        ]
);
const EXILE_LIBRARY_OWNER_TARGET_PLAYER_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["target", "player", "library"],
            &["target", "players", "library"],
            &["target", "player's", "library"]
        ]
);
const EXILE_LIBRARY_OWNER_TARGET_OPPONENT_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["target", "opponent", "library"],
            &["target", "opponents", "library"],
            &["target", "opponent's", "library"]
        ]
);
const EXILE_LIBRARY_OWNER_ITS_CONTROLLER_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["its", "controller", "library"],
            &["its", "controllers", "library"]
        ]
);
const EXILE_LIBRARY_OWNER_ITS_OWNER_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["its", "owner", "library"], &["its", "owners", "library"]]);
const EXILE_LIBRARY_OWNER_HIS_OR_HER_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["his", "or", "her", "library"]);
const EXILE_EACH_OPPONENT_LIBRARY_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["each", "opponent", "library"],
            &["each", "opponents", "library"],
            &["each", "opponent's", "library"]
        ]
);
const EXILE_THE_TOP_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["the", "top"]);
const EXILE_WITH_THAT_NAME_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["with", "that", "name"]]);
const EXILE_CARD_FROM_THEIR_HAND_OR_PERMANENT_THEY_CONTROL_PATTERN: ClauseShape<'static> =
    clause_shape!(
        exact_any
            & [
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
            ]
    );

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
        .is_some_and(|word| EXILE_ALL_OR_EACH_WORD_PATTERN.matches_word(word))
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
    if !face_down
        && !until_source_leaves
        && let Some(effect) = parse_exile_card_from_their_hand_or_permanent_they_control(
            tokens, subject,
        )
    {
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
            .is_some_and(|t| EXILE_TARGET_WORD_PATTERN.matches_token(t))
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
    if !EXILE_CARD_FROM_THEIR_HAND_OR_PERMANENT_THEY_CONTROL_PATTERN.matches_words(&words) {
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
    let attachment_target_words = crate::runtime_backend::token_word_refs(attachment_target_tokens);
    if !EXILE_IT_REFERENCE_PATTERN.matches_words(&attachment_target_words) {
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
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if grammar::words_match_any_prefix(tokens, ALL_CARD_PREFIXES).is_none()
        || !EXILE_WITH_THAT_NAME_PATTERN.matches_words(&clause_words)
    {
        return Ok(None);
    }

    let Some(from_idx) = find_index(&clause_words, |word| {
        EXILE_FROM_WORD_PATTERN.matches_word(word)
    }) else {
        return Ok(None);
    };
    let Some(first_zone_idx) = find_index(&clause_words[from_idx + 1..], |word| {
        EXILE_HAND_OR_GRAVEYARD_WORD_PATTERN.matches_word(word)
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
    if end > 0 && EXILE_INSTEAD_WORD_PATTERN.matches_token(&tokens[end - 1]) {
        end -= 1;
        while end > 0 && tokens[end - 1].is_comma() {
            end -= 1;
        }
    }

    if end > 0 && EXILE_FACE_DOWN_WORD_PATTERN.matches_token(&tokens[end - 1]) {
        return (&tokens[..end - 1], true);
    }

    if end >= 2
        && EXILE_FACE_DOWN_TAIL_PATTERN.matches_words(&crate::runtime_backend::token_word_refs(
            &tokens[end - 2..end],
        ))
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
    if EXILE_GRAVEYARD_OWNER_YOU_PATTERN.matches_words(words) {
        return Some((PlayerAst::You, 2));
    }
    if EXILE_GRAVEYARD_OWNER_THEIR_PATTERN.matches_words(words) {
        return Some((PlayerAst::That, 2));
    }
    if EXILE_GRAVEYARD_OWNER_THAT_PLAYER_PATTERN.matches_words(words) {
        return Some((PlayerAst::That, 3));
    }
    if EXILE_GRAVEYARD_OWNER_TARGET_PLAYER_PATTERN.matches_words(words) {
        return Some((PlayerAst::Target, 3));
    }
    if EXILE_GRAVEYARD_OWNER_TARGET_OPPONENT_PATTERN.matches_words(words) {
        return Some((PlayerAst::TargetOpponent, 3));
    }
    if EXILE_GRAVEYARD_OWNER_ITS_CONTROLLER_PATTERN.matches_words(words) {
        return Some((PlayerAst::ItsController, 3));
    }
    if EXILE_GRAVEYARD_OWNER_ITS_OWNER_PATTERN.matches_words(words) {
        return Some((PlayerAst::ItsOwner, 3));
    }
    if EXILE_GRAVEYARD_OWNER_HIS_OR_HER_PATTERN.matches_words(words) {
        return Some((PlayerAst::That, 4));
    }
    None
}

fn parse_library_owner_prefix(
    words: &[&str],
    default_player: PlayerAst,
) -> Option<(PlayerAst, usize)> {
    if EXILE_LIBRARY_OWNER_DEFAULT_PATTERN.matches_words(words) {
        return Some((default_player, 1));
    }
    if EXILE_LIBRARY_OWNER_YOU_PATTERN.matches_words(words) {
        return Some((PlayerAst::You, 2));
    }
    if EXILE_LIBRARY_OWNER_THEIR_PATTERN.matches_words(words) {
        return Some((
            if matches!(default_player, PlayerAst::Implicit) {
                PlayerAst::ItsController
            } else {
                default_player
            },
            2,
        ));
    }
    if EXILE_LIBRARY_OWNER_THAT_PLAYER_PATTERN.matches_words(words) {
        return Some((PlayerAst::That, 3));
    }
    if EXILE_LIBRARY_OWNER_TARGET_PLAYER_PATTERN.matches_words(words) {
        return Some((PlayerAst::Target, 3));
    }
    if EXILE_LIBRARY_OWNER_TARGET_OPPONENT_PATTERN.matches_words(words) {
        return Some((PlayerAst::TargetOpponent, 3));
    }
    if EXILE_LIBRARY_OWNER_ITS_CONTROLLER_PATTERN.matches_words(words) {
        return Some((PlayerAst::ItsController, 3));
    }
    if EXILE_LIBRARY_OWNER_ITS_OWNER_PATTERN.matches_words(words) {
        return Some((PlayerAst::ItsOwner, 3));
    }
    if EXILE_LIBRARY_OWNER_HIS_OR_HER_PATTERN.matches_words(words) {
        return Some((
            if matches!(default_player, PlayerAst::Implicit) {
                PlayerAst::ItsController
            } else {
                default_player
            },
            4,
        ));
    }
    None
}

pub(crate) fn parse_exile_top_library_clause(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Option<EffectAst> {
    let tokens = trim_commas(tokens);
    let words = crate::runtime_backend::token_word_refs(&tokens);
    let mut start = 0usize;
    if EXILE_THE_TOP_PREFIX_PATTERN.matches_words(&words) {
        start = 1;
    }
    if !EXILE_TOP_WORD_PATTERN.matches_word_at(&words, start) {
        return None;
    }

    let count_start = token_index_for_word_index(&tokens, start + 1)?;
    let (count, used_after_top) = parse_value(&tokens[count_start..])?;
    let after_count = trim_commas(&tokens[count_start + used_after_top..]);
    let after_count_words = crate::runtime_backend::token_word_refs(&after_count);
    if !after_count_words
        .first()
        .is_some_and(|word| EXILE_CARD_OR_CARDS_WORD_PATTERN.matches_word(word))
    {
        return None;
    }

    let after_cards_start = token_index_for_word_index(&after_count, 1)?;
    let after_cards = trim_commas(&after_count[after_cards_start..]);
    let after_cards_words = crate::runtime_backend::token_word_refs(&after_cards);
    if !after_cards_words
        .first()
        .is_some_and(|word| EXILE_OF_WORD_PATTERN.matches_word(word))
    {
        return None;
    }

    let owner_tokens = trim_commas(&after_cards[1..]);
    let owner_words = crate::runtime_backend::token_word_refs(&owner_tokens);
    if EXILE_EACH_OPPONENT_LIBRARY_PATTERN.matches_words(&owner_words) {
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
    let (player, used_words) = parse_library_owner_prefix(&owner_words, default_player)?;
    if used_words < owner_words.len() {
        return None;
    }

    Some(EffectAst::subject_verb_exile_top_of_library(
        player,
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
    let count_start_words = crate::runtime_backend::token_word_refs(&tokens[count_start..=count_start]);
    let (count, used_after_bottom) = if count_start_words
        .first()
        .is_some_and(|word| EXILE_CARD_OR_CARDS_WORD_PATTERN.matches_word(word))
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
        .is_some_and(|word| EXILE_CARD_OR_CARDS_WORD_PATTERN.matches_word(word))
    {
        return None;
    }
    let after_cards_start = token_index_for_word_index(&after_count, 1)?;
    let after_cards = trim_commas(&after_count[after_cards_start..]);
    let after_cards_words = crate::runtime_backend::token_word_refs(&after_cards);
    if !after_cards_words
        .first()
        .is_some_and(|word| EXILE_OF_WORD_PATTERN.matches_word(word))
    {
        return None;
    }
    let owner_tokens = trim_commas(&after_cards[1..]);
    let owner_words = crate::runtime_backend::token_word_refs(&owner_tokens);
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

    if EXILE_EACH_OPPONENT_LIBRARY_PATTERN.matches_words(&owner_words) {
        return Some(EffectAst::ForEachOpponent {
            effects: choose_and_exile(PlayerAst::That, tag),
        });
    }

    let default_player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);
    let (player, used_words) = parse_library_owner_prefix(&owner_words, default_player)?;
    if used_words < owner_words.len() {
        return None;
    }

    Some(EffectAst::Sequence {
        effects: choose_and_exile(player, tag),
    })
}

pub(crate) fn parse_target_player_graveyard_filter(
    tokens: &[OwnedLexToken],
) -> Option<ObjectFilter> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    let (player, consumed) = parse_graveyard_owner_prefix(&words)?;
    if consumed != words.len() {
        return None;
    }

    let mut filter = ObjectFilter::default().in_zone(Zone::Graveyard);
    filter.owner = match player {
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
