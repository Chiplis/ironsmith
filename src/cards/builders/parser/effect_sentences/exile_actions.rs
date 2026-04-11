use super::*;

pub(crate) fn parse_exile(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let (tokens, until_source_leaves) = split_until_source_leaves_tail(tokens);
    let (tokens, face_down) = split_exile_face_down_suffix(tokens);
    let tokens = split_exile_graveyard_replacement_suffix(tokens);
    let clause_words = crate::cards::builders::parser::token_word_refs(tokens);
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
    if matches!(clause_words.first().copied(), Some("all" | "each")) {
        let filter_tokens = &tokens[1..];
        let mut filter = parse_object_filter(filter_tokens, false)?;
        apply_exile_subject_owner_context(&mut filter, subject);
        return Ok(if until_source_leaves {
            EffectAst::ExileUntilSourceLeaves {
                target: TargetAst::Object(filter, None, None),
                face_down,
            }
        } else {
            EffectAst::ExileAll { filter, face_down }
        });
    }
    if let Some(filter) = parse_target_player_graveyard_filter(tokens) {
        return Ok(if until_source_leaves {
            EffectAst::ExileUntilSourceLeaves {
                target: TargetAst::Object(filter, None, None),
                face_down,
            }
        } else {
            EffectAst::ExileAll { filter, face_down }
        });
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
    let has_attached_bundle = grammar::contains_word(tokens, "and")
        && grammar::contains_word(tokens, "all")
        && grammar::contains_word(tokens, "attached");
    if has_attached_bundle {
        return Err(CardTextError::ParseError(format!(
            "unsupported attached-object exile bundle (clause: '{}')",
            clause_words.join(" ")
        )));
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
        crate::cards::builders::parser::grammar::primitives::split_lexed_once_on_separator(
            tokens,
            || {
                use winnow::Parser as _;
                crate::cards::builders::parser::grammar::primitives::kw("and").void()
            },
        )
        && !before_and.is_empty()
    {
        let starts_multi_target = after_and.first().is_some_and(|t| t.is_word("target"))
            || (crate::cards::builders::parser::grammar::primitives::strip_lexed_prefix_phrase(
                after_and,
                &["up", "to"],
            )
            .is_some()
                && crate::cards::builders::parser::grammar::primitives::contains_word(
                    after_and, "target",
                ));
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
                EffectAst::ExileUntilSourceLeaves { target, face_down }
            } else {
                EffectAst::Exile { target, face_down }
            }],
            if_false: Vec::new(),
        });
    } else if tokens.iter().any(|token| token.is_word("if")) {
        return Err(CardTextError::ParseError(format!(
            "unsupported conditional exile clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let mut target = parse_target_phrase(tokens)?;
    apply_exile_subject_hand_owner_context(&mut target, subject);
    Ok(if until_source_leaves {
        EffectAst::ExileUntilSourceLeaves { target, face_down }
    } else {
        EffectAst::Exile { target, face_down }
    })
}

pub(crate) fn parse_same_name_exile_hand_and_graveyard_clause(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
    until_source_leaves: bool,
    face_down: bool,
) -> Result<Option<EffectAst>, CardTextError> {
    let clause_words = crate::cards::builders::parser::token_word_refs(tokens);
    if grammar::words_match_any_prefix(tokens, ALL_CARD_PREFIXES).is_none()
        || grammar::words_find_phrase(tokens, &["with", "that", "name"]).is_none()
    {
        return Ok(None);
    }

    let Some(from_idx) = find_index(&clause_words, |word| *word == "from") else {
        return Ok(None);
    };
    let Some(first_zone_idx) = find_index(&clause_words[from_idx + 1..], |word| {
        matches!(*word, "hand" | "hands" | "graveyard" | "graveyards")
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
        EffectAst::ExileUntilSourceLeaves {
            target: TargetAst::Object(filter, None, None),
            face_down,
        }
    } else {
        EffectAst::ExileAll { filter, face_down }
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
    if end > 0 && tokens[end - 1].is_word("instead") {
        end -= 1;
        while end > 0 && tokens[end - 1].is_comma() {
            end -= 1;
        }
    }

    if end > 0 && (tokens[end - 1].is_word("face-down") || tokens[end - 1].is_word("facedown")) {
        return (&tokens[..end - 1], true);
    }

    if end >= 2 && tokens[end - 2].is_word("face") && tokens[end - 1].is_word("down") {
        return (&tokens[..end - 2], true);
    }

    (tokens, false)
}

pub(crate) fn split_exile_graveyard_replacement_suffix(
    tokens: &[OwnedLexToken],
) -> &[OwnedLexToken] {
    use crate::cards::builders::parser::grammar::primitives as grammar;

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
    if slice_starts_with(&words, &["your", "graveyard"]) {
        return Some((PlayerAst::You, 2));
    }
    if slice_starts_with(&words, &["their", "graveyard"]) {
        return Some((PlayerAst::That, 2));
    }
    if slice_starts_with(&words, &["that", "player", "graveyard"])
        || slice_starts_with(&words, &["that", "players", "graveyard"])
    {
        return Some((PlayerAst::That, 3));
    }
    if slice_starts_with(&words, &["target", "player", "graveyard"])
        || slice_starts_with(&words, &["target", "players", "graveyard"])
    {
        return Some((PlayerAst::Target, 3));
    }
    if slice_starts_with(&words, &["target", "opponent", "graveyard"])
        || slice_starts_with(&words, &["target", "opponents", "graveyard"])
    {
        return Some((PlayerAst::TargetOpponent, 3));
    }
    if slice_starts_with(&words, &["its", "controller", "graveyard"])
        || slice_starts_with(&words, &["its", "controllers", "graveyard"])
    {
        return Some((PlayerAst::ItsController, 3));
    }
    if slice_starts_with(&words, &["its", "owner", "graveyard"])
        || slice_starts_with(&words, &["its", "owners", "graveyard"])
    {
        return Some((PlayerAst::ItsOwner, 3));
    }
    if slice_starts_with(&words, &["his", "or", "her", "graveyard"]) {
        return Some((PlayerAst::That, 4));
    }
    None
}

fn parse_library_owner_prefix(
    words: &[&str],
    default_player: PlayerAst,
) -> Option<(PlayerAst, usize)> {
    if slice_starts_with(&words, &["library"]) {
        return Some((default_player, 1));
    }
    if slice_starts_with(&words, &["your", "library"]) {
        return Some((PlayerAst::You, 2));
    }
    if slice_starts_with(&words, &["their", "library"]) {
        return Some((
            if matches!(default_player, PlayerAst::Implicit) {
                PlayerAst::ItsController
            } else {
                default_player
            },
            2,
        ));
    }
    if slice_starts_with(&words, &["that", "player", "library"])
        || slice_starts_with(&words, &["that", "players", "library"])
    {
        return Some((PlayerAst::That, 3));
    }
    if slice_starts_with(&words, &["target", "player", "library"])
        || slice_starts_with(&words, &["target", "players", "library"])
    {
        return Some((PlayerAst::Target, 3));
    }
    if slice_starts_with(&words, &["target", "opponent", "library"])
        || slice_starts_with(&words, &["target", "opponents", "library"])
    {
        return Some((PlayerAst::TargetOpponent, 3));
    }
    if slice_starts_with(&words, &["its", "controller", "library"])
        || slice_starts_with(&words, &["its", "controllers", "library"])
    {
        return Some((PlayerAst::ItsController, 3));
    }
    if slice_starts_with(&words, &["its", "owner", "library"])
        || slice_starts_with(&words, &["its", "owners", "library"])
    {
        return Some((PlayerAst::ItsOwner, 3));
    }
    if slice_starts_with(&words, &["his", "or", "her", "library"]) {
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
    let words = crate::cards::builders::parser::token_word_refs(&tokens);
    let mut start = 0usize;
    if words.first().copied() == Some("the") {
        start = 1;
    }
    if words.get(start).copied() != Some("top") {
        return None;
    }

    let count_start = token_index_for_word_index(&tokens, start + 1)?;
    let (count, used_after_top) = parse_value(&tokens[count_start..])?;
    let after_count = trim_commas(&tokens[count_start + used_after_top..]);
    let after_count_words = crate::cards::builders::parser::token_word_refs(&after_count);
    if !matches!(after_count_words.first().copied(), Some("card" | "cards")) {
        return None;
    }

    let after_cards_start = token_index_for_word_index(&after_count, 1)?;
    let after_cards = trim_commas(&after_count[after_cards_start..]);
    let after_cards_words = crate::cards::builders::parser::token_word_refs(&after_cards);
    if after_cards_words.first().copied() != Some("of") {
        return None;
    }

    let owner_tokens = trim_commas(&after_cards[1..]);
    let owner_words = crate::cards::builders::parser::token_word_refs(&owner_tokens);
    let default_player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);
    let (player, used_words) = parse_library_owner_prefix(&owner_words, default_player)?;
    if used_words < owner_words.len() {
        return None;
    }

    Some(EffectAst::ExileTopOfLibrary {
        count,
        player,
        tags: vec![helper_tag_for_tokens(&tokens, "exiled")],
        accumulated_tags: Vec::new(),
    })
}

pub(crate) fn parse_target_player_graveyard_filter(
    tokens: &[OwnedLexToken],
) -> Option<ObjectFilter> {
    let words = crate::cards::builders::parser::token_word_refs(tokens);
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
