use super::*;
use crate::CardType;
use crate::runtime_backend::front_end::grammar::effects as effect_grammar;
use crate::runtime_backend::lexer::LexedClause;

pub(crate) fn parse_each_opponent_exiles_card_from_their_hand_or_permanent_they_control(
    tokens: &[OwnedLexToken],
) -> Option<EffectAst> {
    if let Some(shape) =
        effect_grammar::parse_each_player_exile_counted_hand_permanent_shape(tokens)
    {
        let effects = exile_iterated_hand_cards_and_permanents(
            tokens,
            crate::effect::ChoiceCount::dynamic_x(),
            Some(Value::X),
        );
        return Some(match shape.group {
            effect_grammar::EachPlayerExileGroup::Player => EffectAst::ForEachPlayer { effects },
            effect_grammar::EachPlayerExileGroup::Opponent => {
                EffectAst::ForEachOpponent { effects }
            }
        });
    }

    let shape = effect_grammar::parse_each_opponent_exile_choice_shape(tokens)?;
    parse_exile_card_from_their_hand_or_permanent_they_control(
        &shape.choice,
        Some(SubjectAst::Player(PlayerAst::Opponent)),
    )
}

fn exile_iterated_hand_cards_and_permanents(
    tokens: &[OwnedLexToken],
    count: crate::effect::ChoiceCount,
    count_value: Option<Value>,
) -> Vec<EffectAst> {
    let mut hand_card = ObjectFilter::default().in_zone(Zone::Hand);
    hand_card.owner = Some(PlayerFilter::IteratedPlayer);
    let mut permanent = ObjectFilter::permanent_card().in_zone(Zone::Battlefield);
    permanent.controller = Some(PlayerFilter::IteratedPlayer);

    let mut filter = ObjectFilter::default();
    filter.any_of = vec![hand_card, permanent];
    let tag = helper_tag_for_tokens(tokens, "exiled");
    vec![
        EffectAst::ChooseObjectsAcrossZones {
            filter,
            count,
            count_value,
            player: PlayerAst::That,
            tag: tag.clone(),
            zones: vec![Zone::Hand, Zone::Battlefield],
            search_mode: None,
        },
        EffectAst::subject_verb_exile(TargetAst::Tagged(tag, None), false),
    ]
}

fn parse_exile_card_from_their_hand_or_permanent_they_control(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Option<EffectAst> {
    if !effect_grammar::is_exile_hand_or_permanent_choice_shape(tokens) {
        return None;
    }

    let wrap_for_each_opponent = matches!(subject, Some(SubjectAst::Player(PlayerAst::Opponent)));
    let chooser = match subject {
        Some(SubjectAst::Player(PlayerAst::That | PlayerAst::Opponent)) => PlayerAst::That,
        Some(SubjectAst::This) => PlayerAst::That,
        _ => return None,
    };

    let mut effects = exile_iterated_hand_cards_and_permanents(
        tokens,
        crate::effect::ChoiceCount::exactly(1),
        None,
    );
    if let Some(EffectAst::ChooseObjectsAcrossZones { player, .. }) = effects.first_mut() {
        *player = chooser;
    }

    Some(if wrap_for_each_opponent {
        EffectAst::ForEachOpponent { effects }
    } else {
        EffectAst::Sequence { effects }
    })
}

pub(crate) use effect_grammar::ParsedExileOwnerPrefix as ParsedOwnerPrefix;

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
    if let Some(effect) =
        parse_battlefield_graveyard_exile_all_pair(tokens, subject, until_source_leaves, face_down)?
    {
        return Ok(effect);
    }
    if let Some(filter_tokens) = effect_grammar::strip_exile_all_or_each_shape(tokens) {
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
        let starts_multi_target = effect_grammar::starts_exile_multi_target_shape(after_and);
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
    let Some(shape) = effect_grammar::parse_attached_object_exile_shape(tokens) else {
        return Ok(None);
    };
    let target = parse_target_phrase(&shape.target)?;
    let attachment_filter = parse_object_filter_lexed(&shape.attachment_filter, false)?;
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
    let Some(shape) = effect_grammar::parse_same_name_hand_graveyard_exile_shape(tokens) else {
        return Ok(None);
    };
    let owner_from_subject = match subject {
        Some(SubjectAst::Player(player)) => controller_filter_for_token_player(player),
        _ => None,
    };
    let owner = match shape.owner {
        effect_grammar::SameNameExileOwnerShape::TargetPlayer => {
            Some(PlayerFilter::target_player())
        }
        effect_grammar::SameNameExileOwnerShape::TargetOpponent => {
            Some(PlayerFilter::target_opponent())
        }
        effect_grammar::SameNameExileOwnerShape::ThatPlayer => Some(PlayerFilter::IteratedPlayer),
        effect_grammar::SameNameExileOwnerShape::You => Some(PlayerFilter::You),
        effect_grammar::SameNameExileOwnerShape::TheirOrHisHer => {
            owner_from_subject.or(Some(PlayerFilter::IteratedPlayer))
        }
        effect_grammar::SameNameExileOwnerShape::FromSubject => owner_from_subject,
    };
    let Some(owner) = owner else {
        return Ok(None);
    };

    let mut filter = ObjectFilter::default();
    filter.owner = Some(owner);
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: TagKey::from(IT_TAG),
        relation: TaggedOpbjectRelation::SameNameAsTagged,
    });
    filter.any_of = [Zone::Hand, Zone::Graveyard]
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
        let next_starts_all_clause =
            effect_grammar::strip_exile_all_or_each_shape(&tokens[idx + 1..]).is_some();
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

fn parse_battlefield_graveyard_exile_all_pair(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
    until_source_leaves: bool,
    face_down: bool,
) -> Result<Option<EffectAst>, CardTextError> {
    let segments = split_exile_all_list_tail_segment(tokens.to_vec());
    let [first, second] = segments.as_slice() else {
        return Ok(None);
    };
    let mut filters = Vec::with_capacity(2);
    for segment in [first, second] {
        let Some(filter_tokens) = effect_grammar::strip_exile_all_or_each_shape(segment) else {
            return Ok(None);
        };
        let mut filter = parse_object_filter_lexed(filter_tokens, false)?;
        apply_exile_subject_owner_context(&mut filter, subject.clone());
        filters.push(filter);
    }

    let is_creature_planeswalker_limit = |filter: &ObjectFilter| {
        filter.card_types.len() == 2
            && filter.card_types.contains(&CardType::Creature)
            && filter.card_types.contains(&CardType::Planeswalker)
            && filter.mana_value.is_some()
    };
    if !filters.iter().all(is_creature_planeswalker_limit)
        || !matches!(
            (filters[0].zone, filters[1].zone),
            (Some(Zone::Battlefield), Some(Zone::Graveyard))
                | (Some(Zone::Graveyard), Some(Zone::Battlefield))
        )
        || filters[0].mana_value != filters[1].mana_value
    {
        return Ok(None);
    }

    let effects = filters
        .into_iter()
        .map(|filter| {
            if until_source_leaves {
                EffectAst::subject_verb_exile_all_until_source_leaves(
                    TargetAst::Object(filter, None, None),
                    face_down,
                )
            } else {
                EffectAst::subject_verb_exile_all(filter, face_down)
            }
        })
        .collect();
    Ok(Some(EffectAst::Sequence { effects }))
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
            if effect_grammar::strip_exile_all_or_each_shape(&segment).is_none() {
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
        let Some(filter_tokens) = effect_grammar::strip_exile_all_or_each_shape(&segment) else {
            return Ok(None);
        };
        let filter_tokens = trim_commas(filter_tokens);
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
    let shape = effect_grammar::parse_exile_face_down_suffix_shape(tokens);
    (shape.core, shape.face_down)
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
    effect_grammar::parse_exile_graveyard_owner_shape(tokens)
}

fn parse_exile_dynamic_count_from_top_library_clause(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Option<EffectAst> {
    let default_player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);
    let shape = effect_grammar::parse_exile_dynamic_top_library_shape(tokens, default_player)?;
    let effect_grammar::ExileLibraryPlayerShape::Player(player) = shape.player else {
        return None;
    };
    let tag_tokens = trim_commas(tokens);

    Some(EffectAst::subject_verb_exile_top_of_library(
        player,
        shape.count,
        vec![helper_tag_for_tokens(&tag_tokens, "exiled")],
        Vec::new(),
    ))
}

pub(crate) fn parse_exile_top_library_clause(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Option<EffectAst> {
    let default_player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);
    let shape = effect_grammar::parse_exile_top_library_shape(tokens, default_player)?;
    let tag_tokens = trim_commas(tokens);
    match shape.player {
        effect_grammar::ExileLibraryPlayerShape::EachOpponent => Some(EffectAst::ForEachOpponent {
            effects: vec![EffectAst::subject_verb_exile_top_of_library(
                PlayerAst::That,
                shape.count,
                Vec::new(),
                vec![helper_tag_for_tokens(&tag_tokens, "exiled")],
            )],
        }),
        effect_grammar::ExileLibraryPlayerShape::Player(player) => {
            Some(EffectAst::subject_verb_exile_top_of_library(
                player,
                shape.count,
                vec![helper_tag_for_tokens(&tag_tokens, "exiled")],
                Vec::new(),
            ))
        }
    }
}

fn parse_exile_bottom_library_clause(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
    face_down: bool,
) -> Option<EffectAst> {
    let default_player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);
    let shape = effect_grammar::parse_exile_bottom_library_shape(tokens, default_player)?;
    let tag_tokens = trim_commas(tokens);
    let tag = helper_tag_for_tokens(&tag_tokens, "exiled");
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

    match shape.player {
        effect_grammar::ExileLibraryPlayerShape::EachOpponent => Some(EffectAst::ForEachOpponent {
            effects: choose_and_exile(PlayerAst::That, tag),
        }),
        effect_grammar::ExileLibraryPlayerShape::Player(player) => Some(EffectAst::Sequence {
            effects: choose_and_exile(player, tag),
        }),
    }
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
