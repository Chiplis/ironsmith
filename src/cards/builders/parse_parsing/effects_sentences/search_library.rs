use crate::cards::builders::{
    CardTextError, CarryContext, ChoiceCount, EffectAst, IT_TAG, IfResultPredicate, PlayerAst,
    ReturnControllerAst, SubjectAst, TagKey, TargetAst, TextSpan, Token,
    apply_shuffle_subject_graveyard_owner_context, ends_with_until_end_of_turn, find_negation_span,
    find_verb, is_article, maybe_apply_carried_player, maybe_apply_carried_player_with_clause,
    parse_cant_restrictions, parse_effect_chain, parse_effect_chain_with_sentence_primitives,
    parse_effect_clause, parse_if_result_predicate, parse_number, parse_object_filter,
    parse_subject, parse_target_phrase, parse_zone_word, span_from_tokens, split_on_or,
    starts_with_until_end_of_turn, token_index_for_word_index, tokenize_line, trim_commas, words,
};
use crate::target::{ObjectFilter, PlayerFilter, TaggedObjectConstraint, TaggedOpbjectRelation};
use crate::types::{CardType, Subtype};
use crate::zone::Zone;

pub(crate) fn parse_search_library_disjunction_filter(
    filter_tokens: &[Token],
) -> Option<ObjectFilter> {
    let segments = split_on_or(filter_tokens);
    if segments.len() < 2 {
        return None;
    }

    let mut branches = Vec::new();
    for segment in segments {
        let trimmed = trim_commas(&segment);
        if trimmed.is_empty() {
            return None;
        }
        let Ok(filter) = parse_object_filter(&trimmed, false) else {
            return None;
        };
        branches.push(filter);
    }

    if branches.len() < 2 {
        return None;
    }

    let mut filter = ObjectFilter::default();
    filter.any_of = branches;
    Some(filter)
}

pub(crate) fn split_search_same_name_reference_filter(
    tokens: &[Token],
) -> Option<(Vec<Token>, Vec<Token>)> {
    let words_all = words(tokens);
    let (start_word_idx, phrase_len) = if let Some(idx) = words_all
        .windows(5)
        .position(|window| window == ["with", "the", "same", "name", "as"])
    {
        (idx, 5usize)
    } else if let Some(idx) = words_all
        .windows(4)
        .position(|window| window == ["with", "same", "name", "as"])
    {
        (idx, 4usize)
    } else {
        return None;
    };

    let start_token_idx = token_index_for_word_index(tokens, start_word_idx)?;
    let end_token_idx =
        token_index_for_word_index(tokens, start_word_idx + phrase_len).unwrap_or(tokens.len());
    let base_filter_tokens = trim_commas(&tokens[..start_token_idx]);
    let reference_tokens = trim_commas(&tokens[end_token_idx..]);
    Some((base_filter_tokens, reference_tokens))
}

pub(crate) fn is_same_name_that_reference_words(words: &[&str]) -> bool {
    matches!(
        words,
        ["that", "card"]
            | ["that", "cards"]
            | ["that", "creature"]
            | ["that", "creatures"]
            | ["that", "artifact"]
            | ["that", "artifacts"]
            | ["that", "enchantment"]
            | ["that", "enchantments"]
            | ["that", "land"]
            | ["that", "lands"]
            | ["that", "permanent"]
            | ["that", "permanents"]
            | ["that", "spell"]
            | ["that", "spells"]
            | ["that", "object"]
            | ["that", "objects"]
            | ["those", "cards"]
            | ["those", "creatures"]
            | ["those", "artifacts"]
            | ["those", "enchantments"]
            | ["those", "lands"]
            | ["those", "permanents"]
            | ["those", "spells"]
            | ["those", "objects"]
    )
}

pub(crate) fn normalize_search_library_filter(filter: &mut ObjectFilter) {
    filter.zone = None;
    if filter.subtypes.iter().any(|subtype| {
        matches!(
            subtype,
            Subtype::Plains
                | Subtype::Island
                | Subtype::Swamp
                | Subtype::Mountain
                | Subtype::Forest
                | Subtype::Desert
        )
    }) && !filter.card_types.contains(&CardType::Land)
    {
        filter.card_types.push(CardType::Land);
    }

    for nested in &mut filter.any_of {
        normalize_search_library_filter(nested);
    }
}

pub(crate) fn parse_search_library_sentence(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let words_all = words(tokens);
    let Some(search_idx) = tokens
        .iter()
        .position(|token| token.is_word("search") || token.is_word("searches"))
    else {
        return Ok(None);
    };
    if tokens[..search_idx]
        .iter()
        .any(|token| token.is_word("unless"))
    {
        return Ok(None);
    }
    // Allow "you may search ..." to parse as a search sentence, but avoid treating
    // a larger "may ... then search ..." chain as a single search clause. In those
    // cases, we need the higher-level parser to preserve conditionals like "If you do".
    let may_positions: Vec<usize> = tokens[..search_idx]
        .iter()
        .enumerate()
        .filter_map(|(idx, token)| token.is_word("may").then_some(idx))
        .collect();
    if !may_positions.is_empty() {
        let allowed_may = may_positions.len() == 1 && may_positions[0] + 1 == search_idx;
        if !allowed_may {
            return Ok(None);
        }
    }

    let mut subject_tokens = &tokens[..search_idx];
    let sentence_has_direct_may = subject_tokens
        .last()
        .is_some_and(|token| token.is_word("may"));
    if sentence_has_direct_may {
        subject_tokens = &subject_tokens[..subject_tokens.len().saturating_sub(1)];
    }
    let mut if_result_prefix = None;
    let trimmed_subject_tokens = trim_commas(subject_tokens);
    if trimmed_subject_tokens
        .first()
        .is_some_and(|token| token.is_word("if"))
    {
        let predicate_tokens = trim_commas(&trimmed_subject_tokens[1..]);
        if_result_prefix = parse_if_result_predicate(&predicate_tokens);
        if if_result_prefix.is_some() {
            subject_tokens = &[];
        }
    }
    let mut leading_effects = Vec::new();
    if !subject_tokens.is_empty() && find_verb(subject_tokens).is_some() {
        let mut leading_tokens = trim_commas(subject_tokens);
        while leading_tokens
            .last()
            .is_some_and(|token| token.is_word("then") || token.is_word("and"))
        {
            leading_tokens.pop();
        }
        if !leading_tokens.is_empty() {
            leading_effects = parse_effect_chain_with_sentence_primitives(&leading_tokens)?;
        }
        subject_tokens = &[];
    }
    let mut player = match parse_subject(subject_tokens) {
        SubjectAst::Player(player) => player,
        _ => PlayerAst::Implicit,
    };

    let search_tokens = &tokens[search_idx..];
    let search_words = words(search_tokens);
    let Some(search_verb) = search_words.first().copied() else {
        return Ok(None);
    };
    if !matches!(search_verb, "search" | "searches") {
        return Ok(None);
    }
    let search_body_words = &search_words[1..];
    let mut search_player_target: Option<TargetAst> = None;
    let mut forced_library_owner: Option<PlayerFilter> = None;
    let mut include_hand_and_graveyard_bundle = false;
    let mut nonlibrary_choice_zones: Vec<Zone> = Vec::new();
    if search_body_words.starts_with(&["your", "library", "for"])
        || search_body_words.starts_with(&["their", "library", "for"])
    {
        // Keep player from parsed subject/default context.
    } else if search_body_words.starts_with(&[
        "its",
        "controller",
        "graveyard",
        "hand",
        "and",
        "library",
        "for",
    ]) || search_body_words.starts_with(&[
        "its",
        "controllers",
        "graveyard",
        "hand",
        "and",
        "library",
        "for",
    ]) {
        player = PlayerAst::ItsController;
        forced_library_owner = Some(PlayerFilter::ControllerOf(crate::filter::ObjectRef::Target));
        include_hand_and_graveyard_bundle = true;
    } else if search_body_words.starts_with(&[
        "its",
        "owner",
        "graveyard",
        "hand",
        "and",
        "library",
        "for",
    ]) || search_body_words.starts_with(&[
        "its",
        "owners",
        "graveyard",
        "hand",
        "and",
        "library",
        "for",
    ]) {
        player = PlayerAst::ItsOwner;
        forced_library_owner = Some(PlayerFilter::OwnerOf(crate::filter::ObjectRef::Target));
        include_hand_and_graveyard_bundle = true;
    } else if search_body_words.starts_with(&["target", "player", "library", "for"])
        || search_body_words.starts_with(&["target", "players", "library", "for"])
    {
        search_player_target = Some(parse_target_phrase(&search_tokens[1..3])?);
        forced_library_owner = Some(PlayerFilter::target_player());
    } else if search_body_words.starts_with(&["target", "opponent", "library", "for"])
        || search_body_words.starts_with(&["target", "opponents", "library", "for"])
    {
        search_player_target = Some(parse_target_phrase(&search_tokens[1..3])?);
        forced_library_owner = Some(PlayerFilter::target_opponent());
    } else if search_body_words.starts_with(&["that", "player", "library", "for"])
        || search_body_words.starts_with(&["that", "players", "library", "for"])
    {
        player = PlayerAst::That;
    } else if search_body_words.starts_with(&["its", "controller", "library", "for"])
        || search_body_words.starts_with(&["its", "controllers", "library", "for"])
    {
        player = PlayerAst::ItsController;
    } else if search_body_words.starts_with(&["its", "owner", "library", "for"])
        || search_body_words.starts_with(&["its", "owners", "library", "for"])
    {
        player = PlayerAst::ItsOwner;
    } else {
        // Support "Search your graveyard, hand, and/or library ..." (and ordering variants)
        // by modeling non-library zones as optional alternatives before a library search.
        if search_body_words.first().copied() == Some("your")
            && let Some(for_pos) = search_body_words.iter().position(|word| *word == "for")
            && for_pos > 1
        {
            let zone_words = &search_body_words[1..for_pos];
            let has_library = zone_words
                .iter()
                .any(|word| *word == "library" || *word == "libraries");
            if !has_library {
                return Ok(None);
            }

            let has_graveyard = zone_words
                .iter()
                .any(|word| *word == "graveyard" || *word == "graveyards");
            let has_hand = zone_words
                .iter()
                .any(|word| *word == "hand" || *word == "hands");
            if has_graveyard {
                nonlibrary_choice_zones.push(Zone::Graveyard);
            }
            if has_hand {
                nonlibrary_choice_zones.push(Zone::Hand);
            }
            if nonlibrary_choice_zones.is_empty() {
                return Ok(None);
            }
        } else {
            return Ok(None);
        }
    }
    let mentions_nth_from_top = search_words
        .windows(4)
        .any(|window| window[1] == "from" && window[2] == "the" && window[3] == "top")
        && !search_words
            .windows(4)
            .any(|window| window == ["on", "top", "of", "library"]);
    if mentions_nth_from_top {
        return Err(CardTextError::ParseError(format!(
            "unsupported search-library top-position clause (clause: '{}')",
            words_all.join(" ")
        )));
    }

    let for_idx = search_tokens
        .iter()
        .position(|token| token.is_word("for"))
        .unwrap_or(3);
    let put_idx = search_tokens.iter().position(|token| token.is_word("put"));
    let exile_idx = search_tokens.windows(3).position(|window| {
        window[0].is_word("and")
            && (window[1].is_word("exile") || window[1].is_word("exiles"))
            && (window[2].is_word("them")
                || window[2].is_word("those")
                || window[2].is_word("thosecards"))
    });
    let Some(filter_boundary) = put_idx.or(exile_idx) else {
        return Err(CardTextError::ParseError(format!(
            "missing put-or-exile clause in search-library sentence (clause: '{}')",
            words_all.join(" ")
        )));
    };

    let filter_end = {
        let mut end = filter_boundary;
        for idx in (for_idx + 1)..filter_boundary {
            if !matches!(search_tokens[idx], Token::Comma(_)) {
                continue;
            }
            let next_word = search_tokens[idx + 1..].iter().find_map(Token::as_word);
            if matches!(next_word, Some("put" | "reveal" | "then")) {
                end = idx;
                break;
            }
        }
        if end == filter_boundary
            && let Some(idx) = search_tokens
                .iter()
                .position(|token| token.is_word("reveal") || token.is_word("then"))
        {
            end = end.min(idx);
        }
        end
    };

    if filter_end <= for_idx + 1 {
        return Err(CardTextError::ParseError(format!(
            "missing search filter in search-library sentence (clause: '{}')",
            words_all.join(" ")
        )));
    }

    let count_tokens = &search_tokens[for_idx + 1..filter_end];
    let mut count = ChoiceCount::up_to(1);
    let mut count_used = 0usize;

    if count_tokens.len() >= 2
        && count_tokens[0].is_word("any")
        && count_tokens[1].is_word("number")
    {
        count = ChoiceCount::any_number();
        count_used = 2;
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
        count_used = 1;
    } else if count_tokens.len() >= 2
        && count_tokens[0].is_word("up")
        && count_tokens[1].is_word("to")
    {
        if count_tokens.get(2).is_some_and(|token| token.is_word("x")) {
            count = ChoiceCount::dynamic_x();
            count_used = 3;
        } else if let Some((value, used)) = parse_number(&count_tokens[2..]) {
            count = ChoiceCount::up_to(value as usize);
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

    let filter_start = for_idx + 1 + count_used;
    if filter_start >= filter_end {
        return Err(CardTextError::ParseError(format!(
            "missing object selector in search-library sentence (clause: '{}')",
            words_all.join(" ")
        )));
    }

    enum SameNameReference {
        TaggedIt,
        Target(TargetAst),
        Choose { filter: ObjectFilter, tag: TagKey },
    }

    let raw_filter_tokens = trim_commas(&search_tokens[filter_start..filter_end]);
    let mut filter_tokens = raw_filter_tokens.clone();
    let mut same_name_reference: Option<SameNameReference> = None;
    if let Some((base_filter_tokens, reference_tokens)) =
        split_search_same_name_reference_filter(&raw_filter_tokens)
    {
        if base_filter_tokens.is_empty() || reference_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "incomplete same-name search filter in search-library sentence (clause: '{}')",
                words_all.join(" ")
            )));
        }
        filter_tokens = base_filter_tokens;
        let reference_words = words(&reference_tokens);
        same_name_reference = if is_same_name_that_reference_words(&reference_words) {
            Some(SameNameReference::TaggedIt)
        } else if reference_words.iter().any(|word| *word == "target") {
            let target = parse_target_phrase(&reference_tokens).map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported target same-name reference in search-library sentence (clause: '{}')",
                    words_all.join(" ")
                ))
            })?;
            Some(SameNameReference::Target(target))
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
            let reference_filter =
                parse_object_filter(&reference_filter_tokens, other_reference).map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported same-name reference filter in search-library sentence (clause: '{}')",
                    words_all.join(" ")
                ))
            })?;
            Some(SameNameReference::Choose {
                filter: reference_filter,
                tag: TagKey::from("same_name_reference"),
            })
        };
    }

    let filter_words: Vec<&str> = words(&filter_tokens)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();
    let mut filter = if let Some(named_idx) = filter_words.iter().position(|word| *word == "named")
    {
        let name = filter_words
            .iter()
            .skip(named_idx + 1)
            .copied()
            .collect::<Vec<_>>()
            .join(" ");
        if name.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing card name in named search clause (clause: '{}')",
                words_all.join(" ")
            )));
        }
        let base_words = &filter_words[..named_idx];
        let mut base_filter = if base_words.is_empty()
            || (base_words.len() == 1 && (base_words[0] == "card" || base_words[0] == "cards"))
        {
            ObjectFilter::default()
        } else {
            let base_tokens: Vec<Token> = base_words
                .iter()
                .map(|word| Token::Word((*word).to_string(), TextSpan::synthetic()))
                .collect();
            parse_object_filter(&base_tokens, false).map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported named search filter in search-library sentence (clause: '{}')",
                    words_all.join(" ")
                ))
            })?
        };
        base_filter.name = Some(name);
        base_filter
    } else if filter_words.len() == 1 && (filter_words[0] == "card" || filter_words[0] == "cards") {
        ObjectFilter::default()
    } else if filter_words.contains(&"mana")
        && filter_words.contains(&"ability")
        && filter_words.contains(&"or")
    {
        if let Some(disjunction_filter) = parse_search_library_disjunction_filter(&filter_tokens) {
            disjunction_filter
        } else {
            parse_object_filter(&filter_tokens, false).map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported search filter in search-library sentence (clause: '{}')",
                    words_all.join(" ")
                ))
            })?
        }
    } else {
        parse_object_filter(&filter_tokens, false).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported search filter in search-library sentence (clause: '{}')",
                words_all.join(" ")
            ))
        })?
    };
    if let Some(same_name_tag) = same_name_reference
        .as_ref()
        .map(|reference| match reference {
            SameNameReference::TaggedIt | SameNameReference::Target(_) => TagKey::from(IT_TAG),
            SameNameReference::Choose { tag, .. } => tag.clone(),
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

    if words_all.contains(&"mana") && words_all.contains(&"cost") {
        filter.has_mana_cost = true;
        filter.no_x_in_cost = true;
        let mut max_value: Option<u32> = None;
        for word in words_all.iter() {
            if let Ok(value) = word.parse::<u32>() {
                max_value = Some(max_value.map_or(value, |max| max.max(value)));
            }
        }
        if let Some(max_value) = max_value {
            filter.mana_value = Some(crate::filter::Comparison::LessThanOrEqual(max_value as i32));
        }
    }

    let destination = if let Some(put_idx) = put_idx {
        let put_clause_words = words(&search_tokens[put_idx..]);
        if put_clause_words.contains(&"graveyard") {
            Zone::Graveyard
        } else if put_clause_words.contains(&"hand") {
            Zone::Hand
        } else if put_clause_words.contains(&"top") {
            Zone::Library
        } else {
            Zone::Battlefield
        }
    } else {
        Zone::Exile
    };

    let reveal = words_all.contains(&"reveal");
    let trailing_discard_before_shuffle = if let Some(put_idx) = put_idx {
        let discard_idx = search_tokens
            .iter()
            .position(|token| token.is_word("discard") || token.is_word("discards"));
        let shuffle_idx = search_tokens
            .iter()
            .rposition(|token| token.is_word("shuffle") || token.is_word("shuffles"));
        matches!(
            (discard_idx, shuffle_idx),
            (Some(discard_idx), Some(shuffle_idx)) if discard_idx > put_idx && discard_idx < shuffle_idx
        )
    } else {
        false
    };
    let shuffle = (words_all.contains(&"shuffle") && !trailing_discard_before_shuffle)
        || !nonlibrary_choice_zones.is_empty();
    let split_battlefield_and_hand = put_idx.is_some()
        && words_all.contains(&"battlefield")
        && words_all.contains(&"hand")
        && words_all.contains(&"other")
        && words_all.contains(&"one");
    let zone_bundle_filter = if include_hand_and_graveyard_bundle {
        Some(filter.clone())
    } else {
        None
    };
    let mut effects = if !nonlibrary_choice_zones.is_empty() {
        let chosen_tag: TagKey = "searched_nonlibrary".into();
        let battlefield_tapped = destination == Zone::Battlefield && words_all.contains(&"tapped");

        let mut move_effects = Vec::new();
        if reveal {
            move_effects.push(EffectAst::RevealTagged {
                tag: chosen_tag.clone(),
            });
        }
        move_effects.push(EffectAst::MoveToZone {
            target: TargetAst::Tagged(chosen_tag.clone(), span_from_tokens(tokens)),
            zone: destination,
            to_top: matches!(destination, Zone::Library),
            battlefield_controller: ReturnControllerAst::Preserve,
            battlefield_tapped: battlefield_tapped,
            attached_to: None,
        });

        let mut first_filter = filter.clone();
        first_filter.zone = Some(nonlibrary_choice_zones[0]);
        if first_filter.owner.is_none() {
            first_filter.owner = Some(PlayerFilter::You);
        }

        let did_not_effects = if nonlibrary_choice_zones.len() > 1 {
            let mut second_filter = filter.clone();
            second_filter.zone = Some(nonlibrary_choice_zones[1]);
            if second_filter.owner.is_none() {
                second_filter.owner = Some(PlayerFilter::You);
            }

            vec![
                EffectAst::ChooseObjects {
                    filter: second_filter,
                    count: ChoiceCount::up_to(1),
                    player,
                    tag: chosen_tag.clone(),
                },
                EffectAst::IfResult {
                    predicate: IfResultPredicate::Did,
                    effects: move_effects.clone(),
                },
                EffectAst::IfResult {
                    predicate: IfResultPredicate::DidNot,
                    effects: vec![EffectAst::SearchLibrary {
                        filter,
                        destination,
                        player,
                        reveal,
                        shuffle,
                        count,
                        tapped: battlefield_tapped,
                    }],
                },
            ]
        } else {
            vec![EffectAst::SearchLibrary {
                filter,
                destination,
                player,
                reveal,
                shuffle,
                count,
                tapped: battlefield_tapped,
            }]
        };

        vec![
            EffectAst::ChooseObjects {
                filter: first_filter,
                count: ChoiceCount::up_to(1),
                player,
                tag: chosen_tag.clone(),
            },
            EffectAst::IfResult {
                predicate: IfResultPredicate::Did,
                effects: move_effects,
            },
            EffectAst::IfResult {
                predicate: IfResultPredicate::DidNot,
                effects: did_not_effects,
            },
        ]
    } else if split_battlefield_and_hand {
        let battlefield_tapped = words_all.contains(&"tapped");
        vec![
            EffectAst::SearchLibrary {
                filter: filter.clone(),
                destination: Zone::Battlefield,
                player,
                reveal,
                shuffle: false,
                count: ChoiceCount::up_to(1),
                tapped: battlefield_tapped,
            },
            EffectAst::SearchLibrary {
                filter,
                destination: Zone::Hand,
                player,
                reveal,
                shuffle,
                count: ChoiceCount::up_to(1),
                tapped: false,
            },
        ]
    } else {
        let battlefield_tapped = destination == Zone::Battlefield && words_all.contains(&"tapped");
        vec![EffectAst::SearchLibrary {
            filter,
            destination,
            player,
            reveal,
            shuffle,
            count,
            tapped: battlefield_tapped,
        }]
    };

    if include_hand_and_graveyard_bundle && let Some(base_filter) = zone_bundle_filter {
        for zone in [Zone::Graveyard, Zone::Hand] {
            let mut zone_filter = base_filter.clone();
            zone_filter.zone = Some(zone);
            if zone_filter.owner.is_none() {
                zone_filter.owner = forced_library_owner.clone();
            }
            effects.push(EffectAst::ExileAll {
                filter: zone_filter,
                face_down: false,
            });
        }
    }

    if trailing_discard_before_shuffle
        && let (Some(discard_idx), Some(shuffle_idx)) = (
            search_tokens
                .iter()
                .position(|token| token.is_word("discard") || token.is_word("discards")),
            search_tokens
                .iter()
                .rposition(|token| token.is_word("shuffle") || token.is_word("shuffles")),
        )
    {
        let mut discard_end = shuffle_idx;
        while discard_end > discard_idx {
            let token = &search_tokens[discard_end - 1];
            if matches!(token, Token::Comma(_)) || token.is_word("then") || token.is_word("and") {
                discard_end -= 1;
                continue;
            }
            break;
        }

        let discard_tokens = trim_commas(&search_tokens[discard_idx..discard_end]);
        if !discard_tokens.is_empty() {
            effects.push(parse_effect_clause(&discard_tokens)?);
        }
        effects.push(EffectAst::ShuffleLibrary { player });
    }

    if let Some(target) = search_player_target {
        effects.insert(0, EffectAst::TargetOnly { target });
    }

    if let Some(and_idx) = search_tokens
        .iter()
        .enumerate()
        .skip(put_idx.unwrap_or(filter_boundary))
        .find_map(|(idx, token)| token.is_word("and").then_some(idx))
    {
        let trailing_tokens = trim_commas(&search_tokens[and_idx + 1..]);
        if !trailing_tokens.is_empty() {
            let trailing_words = words(&trailing_tokens);
            let starts_with_life_clause = trailing_words.starts_with(&["you", "gain"])
                || trailing_words.starts_with(&["target", "player", "gains"])
                || trailing_words.starts_with(&["target", "player", "gain"]);
            if starts_with_life_clause {
                let trailing_effect = parse_effect_clause(&trailing_tokens)?;
                effects.push(trailing_effect);
            }
        }
    }

    if let Some(reference) = same_name_reference {
        match reference {
            SameNameReference::TaggedIt => {}
            SameNameReference::Target(target) => {
                effects.insert(0, EffectAst::TargetOnly { target });
            }
            SameNameReference::Choose { filter, tag } => {
                effects.insert(
                    0,
                    EffectAst::ChooseObjects {
                        filter,
                        count: ChoiceCount::exactly(1),
                        player,
                        tag,
                    },
                );
            }
        }
    }

    if sentence_has_direct_may {
        effects = vec![if matches!(player, PlayerAst::You | PlayerAst::Implicit) {
            EffectAst::May { effects }
        } else {
            EffectAst::MayByPlayer { player, effects }
        }];
    }
    if let Some(predicate) = if_result_prefix {
        effects = vec![EffectAst::IfResult { predicate, effects }];
    }

    if !leading_effects.is_empty() {
        leading_effects.extend(effects);
        return Ok(Some(leading_effects));
    }

    Ok(Some(effects))
}

pub(crate) fn parse_shuffle_graveyard_into_library_sentence(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if tokens.is_empty() {
        return Ok(None);
    }

    let mut clause_tokens = trim_commas(tokens);
    while clause_tokens
        .first()
        .is_some_and(|token| token.is_word("then") || token.is_word("and"))
    {
        clause_tokens.remove(0);
    }
    if clause_tokens.is_empty() {
        return Ok(None);
    }

    let clause_words = words(&clause_tokens);
    if !clause_words
        .iter()
        .any(|word| *word == "shuffle" || *word == "shuffles")
        || !clause_words.contains(&"graveyard")
        || !clause_words.contains(&"library")
    {
        return Ok(None);
    }

    let Some(shuffle_idx) = clause_tokens
        .iter()
        .position(|token| token.is_word("shuffle") || token.is_word("shuffles"))
    else {
        return Ok(None);
    };

    // Keep this primitive focused on shuffle-led clauses so we don't swallow
    // earlier effects in chains like "... then shuffle your graveyard ...".
    if shuffle_idx > 3 {
        return Ok(None);
    }

    let subject_tokens = trim_commas(&clause_tokens[..shuffle_idx]);
    let each_player_subject = {
        let subject_words = words(&subject_tokens);
        subject_words.starts_with(&["each", "player"])
            || subject_words.starts_with(&["each", "players"])
    };
    let subject = if subject_tokens.is_empty() {
        SubjectAst::Player(PlayerAst::You)
    } else if each_player_subject {
        SubjectAst::Player(PlayerAst::Implicit)
    } else {
        parse_subject(&subject_tokens)
    };
    let player = match subject {
        SubjectAst::Player(player) => player,
        SubjectAst::This => return Ok(None),
    };

    let body_tokens = trim_commas(&clause_tokens[shuffle_idx + 1..]);
    if body_tokens.is_empty() {
        return Ok(None);
    }

    let Some(into_idx) = body_tokens.iter().position(|token| token.is_word("into")) else {
        return Ok(None);
    };
    if into_idx == 0 {
        return Ok(None);
    }

    let destination_tokens = trim_commas(&body_tokens[into_idx + 1..]);
    let destination_words = words(&destination_tokens);
    if !destination_words.contains(&"library") {
        return Ok(None);
    }
    let owner_library_destination = destination_words.iter().any(|word| word.contains("owner"));
    let trailing_tokens = destination_tokens
        .iter()
        .position(|token| token.is_word("library") || token.is_word("libraries"))
        .map(|idx| trim_commas(&destination_tokens[idx + 1..]).to_vec())
        .unwrap_or_default();
    let append_trailing =
        |mut effects: Vec<EffectAst>| -> Result<Option<Vec<EffectAst>>, CardTextError> {
            if trailing_tokens.is_empty() {
                return Ok(Some(effects));
            }
            let mut trailing_effects = parse_effect_chain(&trailing_tokens)?;
            if each_player_subject {
                for effect in &mut trailing_effects {
                    maybe_apply_carried_player(effect, CarryContext::ForEachPlayer);
                }
            } else {
                for effect in &mut trailing_effects {
                    maybe_apply_carried_player_with_clause(
                        effect,
                        CarryContext::Player(player),
                        &trailing_tokens,
                    );
                }
            }
            effects.extend(trailing_effects);
            Ok(Some(effects))
        };

    let target_tokens = trim_commas(&body_tokens[..into_idx]);
    if target_tokens.is_empty() {
        return Ok(None);
    }
    let target_words = words(&target_tokens);
    if !target_words.contains(&"graveyard") {
        return Ok(None);
    }

    let has_target_selector = target_words.contains(&"target");
    if !has_target_selector {
        let mut effects = Vec::new();
        let has_source_and_graveyard_clause = target_words
            .starts_with(&["this", "artifact", "and"])
            || target_words.starts_with(&["this", "permanent", "and"])
            || target_words.starts_with(&["this", "card", "and"]);
        if has_source_and_graveyard_clause {
            effects.push(EffectAst::MoveToZone {
                target: TargetAst::Source(None),
                zone: Zone::Library,
                to_top: false,
                battlefield_controller: ReturnControllerAst::Preserve,
                battlefield_tapped: false,
                attached_to: None,
            });
            if owner_library_destination {
                effects.push(EffectAst::ShuffleLibrary {
                    player: PlayerAst::ItsOwner,
                });
            }
        }
        if each_player_subject && target_words.contains(&"hand") {
            let mut hand_filter = ObjectFilter::default();
            hand_filter.zone = Some(Zone::Hand);
            hand_filter.owner = Some(PlayerFilter::IteratedPlayer);
            effects.push(EffectAst::MoveToZone {
                target: TargetAst::Object(hand_filter, None, None),
                zone: Zone::Library,
                to_top: false,
                battlefield_controller: ReturnControllerAst::Preserve,
                battlefield_tapped: false,
                attached_to: None,
            });
        }
        effects.push(EffectAst::ShuffleGraveyardIntoLibrary { player });
        if each_player_subject {
            return append_trailing(vec![EffectAst::ForEachPlayer { effects }]);
        }
        return append_trailing(effects);
    }

    let mut target = parse_target_phrase(&target_tokens)?;
    apply_shuffle_subject_graveyard_owner_context(&mut target, subject);

    append_trailing(vec![
        EffectAst::MoveToZone {
            target,
            zone: Zone::Library,
            to_top: false,
            battlefield_controller: ReturnControllerAst::Preserve,
            battlefield_tapped: false,
            attached_to: None,
        },
        EffectAst::ShuffleLibrary { player },
    ])
}

pub(crate) fn parse_shuffle_object_into_library_sentence(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if tokens.is_empty() {
        return Ok(None);
    }

    let mut clause_tokens = trim_commas(tokens);
    while clause_tokens
        .first()
        .is_some_and(|token| token.is_word("then") || token.is_word("and"))
    {
        clause_tokens.remove(0);
    }
    if clause_tokens.is_empty() {
        return Ok(None);
    }

    let clause_words = words(&clause_tokens);
    if !clause_words
        .iter()
        .any(|word| *word == "shuffle" || *word == "shuffles")
        || !clause_words.contains(&"library")
        || clause_words.contains(&"graveyard")
    {
        return Ok(None);
    }

    let Some(shuffle_idx) = clause_tokens
        .iter()
        .position(|token| token.is_word("shuffle") || token.is_word("shuffles"))
    else {
        return Ok(None);
    };
    if shuffle_idx > 3 {
        return Ok(None);
    }

    let subject_tokens = trim_commas(&clause_tokens[..shuffle_idx]);
    let subject = if subject_tokens.is_empty() {
        SubjectAst::Player(PlayerAst::You)
    } else {
        parse_subject(&subject_tokens)
    };
    let player = match subject {
        SubjectAst::Player(player) => player,
        SubjectAst::This => return Ok(None),
    };

    let body_tokens = trim_commas(&clause_tokens[shuffle_idx + 1..]);
    let Some(into_idx) = body_tokens.iter().position(|token| token.is_word("into")) else {
        return Ok(None);
    };
    if into_idx == 0 {
        return Ok(None);
    }

    let destination_tokens = trim_commas(&body_tokens[into_idx + 1..]);
    if !words(&destination_tokens).contains(&"library") {
        return Ok(None);
    }

    let target_tokens = trim_commas(&body_tokens[..into_idx]);
    if target_tokens.is_empty() {
        return Ok(None);
    }
    let target = parse_target_phrase(&target_tokens)?;

    Ok(Some(vec![
        EffectAst::MoveToZone {
            target,
            zone: Zone::Library,
            to_top: false,
            battlefield_controller: ReturnControllerAst::Preserve,
            battlefield_tapped: false,
            attached_to: None,
        },
        EffectAst::ShuffleLibrary { player },
    ]))
}

pub(crate) fn parse_exile_hand_and_graveyard_bundle_sentence(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if tokens.is_empty() {
        return Ok(None);
    }

    let mut clause_tokens = trim_commas(tokens);
    while clause_tokens
        .first()
        .is_some_and(|token| token.is_word("then") || token.is_word("and"))
    {
        clause_tokens.remove(0);
    }
    if clause_tokens.is_empty() {
        return Ok(None);
    }

    let clause_words = words(&clause_tokens);
    if !clause_words.starts_with(&["exile", "all", "cards", "from"]) {
        return Ok(None);
    }
    if !clause_words.contains(&"hand") && !clause_words.contains(&"hands") {
        return Ok(None);
    }
    if !clause_words.contains(&"graveyard") && !clause_words.contains(&"graveyards") {
        return Ok(None);
    }

    let first_zone_idx = clause_words
        .iter()
        .position(|word| matches!(*word, "hand" | "hands" | "graveyard" | "graveyards"))
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing zone in exile hand+graveyard clause (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
    if first_zone_idx <= 4 {
        return Ok(None);
    }

    let owner_words = &clause_words[4..first_zone_idx];
    let owner = match owner_words {
        ["target", "player"] | ["target", "players"] => PlayerFilter::target_player(),
        ["target", "opponent"] | ["target", "opponents"] => PlayerFilter::target_opponent(),
        ["your"] => PlayerFilter::You,
        _ => return Ok(None),
    };

    let Some(first_zone) = parse_zone_word(clause_words[first_zone_idx]) else {
        return Ok(None);
    };
    if !matches!(first_zone, Zone::Hand | Zone::Graveyard) {
        return Ok(None);
    }

    let Some(and_word) = clause_words.get(first_zone_idx + 1) else {
        return Ok(None);
    };
    if *and_word != "and" {
        return Ok(None);
    }

    let mut second_zone_idx = first_zone_idx + 2;
    while clause_words
        .get(second_zone_idx)
        .is_some_and(|word| matches!(*word, "all" | "cards" | "from"))
    {
        second_zone_idx += 1;
    }
    let Some(second_zone_word) = clause_words.get(second_zone_idx) else {
        return Ok(None);
    };
    if clause_words.len() != second_zone_idx + 1 {
        return Ok(None);
    }
    let Some(second_zone) = parse_zone_word(second_zone_word) else {
        return Ok(None);
    };
    if !matches!(second_zone, Zone::Hand | Zone::Graveyard) || second_zone == first_zone {
        return Ok(None);
    }

    let mut first_filter = ObjectFilter::default().in_zone(first_zone);
    first_filter.owner = Some(owner.clone());
    let mut second_filter = ObjectFilter::default().in_zone(second_zone);
    second_filter.owner = Some(owner);

    Ok(Some(vec![
        EffectAst::ExileAll {
            filter: first_filter,
            face_down: false,
        },
        EffectAst::ExileAll {
            filter: second_filter,
            face_down: false,
        },
    ]))
}

pub(crate) fn parse_target_player_exiles_creature_and_graveyard_sentence(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause_tokens = trim_commas(tokens);
    let clause_words = words(&clause_tokens);
    if clause_words.len() < 8 {
        return Ok(None);
    }

    let (subject_player, subject_filter) = if clause_words.starts_with(&["target", "opponent"]) {
        (PlayerAst::TargetOpponent, PlayerFilter::target_opponent())
    } else if clause_words.starts_with(&["target", "player"]) {
        (PlayerAst::Target, PlayerFilter::target_player())
    } else {
        return Ok(None);
    };

    let verb_idx = 2usize;
    if !matches!(
        clause_words.get(verb_idx).copied(),
        Some("exile") | Some("exiles")
    ) {
        return Ok(None);
    }

    let tail_words = &clause_words[verb_idx + 1..];
    let Some(and_idx) = tail_words.iter().position(|word| *word == "and") else {
        return Ok(None);
    };
    let creature_words = &tail_words[..and_idx];
    let graveyard_words = &tail_words[and_idx + 1..];

    if graveyard_words != ["their", "graveyard"] {
        return Ok(None);
    }

    let creature_words = if creature_words.first().is_some_and(|word| is_article(word)) {
        &creature_words[1..]
    } else {
        creature_words
    };
    let creature_clause_matches = creature_words == ["creature", "they", "control"]
        || creature_words == ["creature", "that", "player", "controls"];
    if !creature_clause_matches {
        return Ok(None);
    }

    let mut creature_filter = ObjectFilter::creature();
    creature_filter.controller = Some(subject_filter.clone());

    let mut graveyard_filter = ObjectFilter::default().in_zone(Zone::Graveyard);
    graveyard_filter.owner = Some(subject_filter);

    Ok(Some(vec![
        EffectAst::ChooseObjects {
            filter: creature_filter,
            count: ChoiceCount::exactly(1),
            player: subject_player,
            tag: TagKey::from(IT_TAG),
        },
        EffectAst::Exile {
            target: TargetAst::Tagged(TagKey::from(IT_TAG), None),
            face_down: false,
        },
        EffectAst::ExileAll {
            filter: graveyard_filter,
            face_down: false,
        },
    ]))
}

pub(crate) fn parse_for_each_exiled_this_way_sentence(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let words_all = words(tokens);
    if !words_all.starts_with(&["for", "each", "permanent", "exiled", "this", "way"]) {
        return Ok(None);
    }
    if !words_all.contains(&"shares")
        || !words_all.contains(&"card")
        || !words_all.contains(&"type")
        || !words_all.contains(&"library")
        || !words_all.contains(&"battlefield")
    {
        return Ok(None);
    }

    let filter_tokens = tokenize_line("a permanent that shares a card type with it", 0);
    let filter = parse_object_filter(&filter_tokens, false)?;

    Ok(Some(vec![EffectAst::ForEachTagged {
        tag: "exiled_0".into(),
        effects: vec![EffectAst::SearchLibrary {
            filter,
            destination: Zone::Battlefield,
            player: PlayerAst::Implicit,
            reveal: true,
            shuffle: true,
            count: ChoiceCount::up_to(1),
            tapped: false,
        }],
    }]))
}

pub(crate) fn parse_each_player_put_permanent_cards_exiled_with_source_sentence(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let words_all = words(tokens);
    let starts_with_each_player_turns_face_up =
        words_all.starts_with(&["each", "player", "turns", "face", "up", "all", "cards"]);
    if !starts_with_each_player_turns_face_up {
        return Ok(None);
    }
    let has_exiled_with_this = words_all
        .windows(3)
        .any(|window| window == ["exiled", "with", "this"]);
    if !has_exiled_with_this {
        return Ok(None);
    }
    let has_puts_all_permanent_cards = words_all
        .windows(5)
        .any(|window| window == ["then", "puts", "all", "permanent", "cards"]);
    let has_among_them_onto_battlefield = words_all
        .windows(4)
        .any(|window| window == ["among", "them", "onto", "battlefield"])
        || words_all
            .windows(5)
            .any(|window| window == ["among", "them", "onto", "the", "battlefield"]);
    if !has_puts_all_permanent_cards || !has_among_them_onto_battlefield {
        return Ok(None);
    }

    let mut filter = ObjectFilter::default().in_zone(Zone::Exile);
    filter.owner = Some(PlayerFilter::IteratedPlayer);
    filter.card_types = vec![
        CardType::Artifact,
        CardType::Creature,
        CardType::Enchantment,
        CardType::Land,
        CardType::Planeswalker,
        CardType::Battle,
    ];
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: TagKey::from(crate::tag::SOURCE_EXILED_TAG),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });

    Ok(Some(vec![EffectAst::ForEachPlayer {
        effects: vec![EffectAst::ReturnAllToBattlefield {
            filter,
            tapped: false,
        }],
    }]))
}

pub(crate) fn parse_for_each_destroyed_this_way_sentence(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let words_all = words(tokens);
    if !words_all.starts_with(&["for", "each"]) {
        return Ok(None);
    }
    let refers_to_destroyed = words_all
        .windows(3)
        .any(|window| window == ["destroyed", "this", "way"]);
    let refers_to_died = words_all
        .windows(3)
        .any(|window| window == ["died", "this", "way"]);
    if !refers_to_destroyed && !refers_to_died {
        return Ok(None);
    }

    let comma_idx = tokens
        .iter()
        .position(|token| matches!(token, Token::Comma(_)))
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing comma after 'for each ... this way' clause (clause: '{}')",
                words_all.join(" ")
            ))
        })?;
    let effect_tokens = trim_commas(&tokens[comma_idx + 1..]);
    if effect_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing effect after 'for each ... this way' clause (clause: '{}')",
            words_all.join(" ")
        )));
    }
    let effects = parse_effect_chain(&effect_tokens)?;
    if effects.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "empty effect after 'for each ... this way' clause (clause: '{}')",
            words_all.join(" ")
        )));
    }

    Ok(Some(vec![EffectAst::ForEachTagged {
        tag: IT_TAG.into(),
        effects,
    }]))
}

pub(crate) fn parse_earthbend_sentence(
    tokens: &[Token],
) -> Result<Option<EffectAst>, CardTextError> {
    let words = words(tokens);
    if words.first().copied() != Some("earthbend") {
        return Ok(None);
    }

    let count_tokens = &tokens[1..];
    let (count, _) = parse_number(count_tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing earthbend count (clause: '{}')",
            words.join(" ")
        ))
    })?;

    Ok(Some(EffectAst::Earthbend { counters: count }))
}

pub(crate) fn parse_enchant_sentence(tokens: &[Token]) -> Result<Option<EffectAst>, CardTextError> {
    let words = words(tokens);
    if words.is_empty() || words[0] != "enchant" {
        return Ok(None);
    }

    let remaining = if tokens.len() > 1 { &tokens[1..] } else { &[] };
    let filter = parse_object_filter(remaining, false)?;
    Ok(Some(EffectAst::Enchant { filter }))
}

pub(crate) fn parse_cant_effect_sentence(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((duration, clause_tokens)) = parse_restriction_duration(tokens)? else {
        return Ok(None);
    };
    if clause_tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "restriction clause missing body".to_string(),
        ));
    }
    if find_negation_span(&clause_tokens).is_none() {
        return Ok(None);
    }
    // Let chain-carry handle mixed clauses like
    // "Tap that creature and it doesn't untap during its controller's next untap step."
    // If conjunction appears before the first negation, this is likely not a pure
    // cant-restriction sentence.
    if let Some((neg_start, _)) = find_negation_span(&clause_tokens)
        && clause_tokens[..neg_start]
            .iter()
            .any(|token| token.is_word("and"))
    {
        return Ok(None);
    }

    let Some(restrictions) = parse_cant_restrictions(&clause_tokens)? else {
        return Err(CardTextError::ParseError(format!(
            "unsupported restriction clause body (clause: '{}')",
            words(&clause_tokens).join(" ")
        )));
    };

    let mut target: Option<TargetAst> = None;
    let mut effects = Vec::new();
    for parsed in restrictions {
        if let Some(parsed_target) = parsed.target {
            if let Some(existing) = &target {
                if *existing != parsed_target {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported mixed restriction targets (clause: '{}')",
                        words(&clause_tokens).join(" ")
                    )));
                }
            } else {
                target = Some(parsed_target);
            }
        }
        effects.push(EffectAst::Cant {
            restriction: parsed.restriction,
            duration: duration.clone(),
        });
    }
    if let Some(target) = target {
        effects.insert(0, EffectAst::TargetOnly { target });
    }

    Ok(Some(effects))
}

pub(crate) fn parse_restriction_duration(
    tokens: &[Token],
) -> Result<Option<(crate::effect::Until, Vec<Token>)>, CardTextError> {
    use crate::effect::Until;

    let all_words = words(tokens);
    if all_words.len() < 4 {
        return Ok(None);
    }

    if starts_with_until_end_of_turn(&all_words) {
        let comma_idx = tokens
            .iter()
            .position(|token| matches!(token, Token::Comma(_)));
        let remainder = if let Some(idx) = comma_idx {
            &tokens[idx + 1..]
        } else {
            &tokens[4..]
        };
        return Ok(Some((Until::EndOfTurn, trim_commas(remainder))));
    }

    if all_words.starts_with(&["until", "your", "next", "turn"]) {
        let comma_idx = tokens
            .iter()
            .position(|token| matches!(token, Token::Comma(_)));
        let remainder = if let Some(idx) = comma_idx {
            &tokens[idx + 1..]
        } else {
            &tokens[4..]
        };
        return Ok(Some((Until::YourNextTurn, trim_commas(remainder))));
    }

    if all_words.starts_with(&["for", "as", "long", "as"]) {
        let as_long_duration = all_words.contains(&"you")
            && all_words.contains(&"control")
            && (all_words.contains(&"this")
                || all_words.contains(&"thiss")
                || all_words.contains(&"source")
                || all_words.contains(&"creature")
                || all_words.contains(&"permanent"));
        if !as_long_duration {
            return Ok(None);
        }
        let Some(comma_idx) = tokens
            .iter()
            .position(|token| matches!(token, Token::Comma(_)))
        else {
            return Err(CardTextError::ParseError(
                "missing comma after duration prefix".to_string(),
            ));
        };
        let remainder = trim_commas(&tokens[comma_idx + 1..]);
        return Ok(Some((Until::YouStopControllingThis, remainder)));
    }

    if ends_with_until_end_of_turn(&all_words) {
        let end_idx = tokens
            .iter()
            .rposition(|token| token.is_word("until"))
            .unwrap_or(tokens.len());
        let remainder = trim_commas(&tokens[..end_idx]);
        return Ok(Some((Until::EndOfTurn, remainder)));
    }

    if all_words.ends_with(&["until", "your", "next", "turn"])
        || (all_words.ends_with(&["next", "turn"]) && all_words.contains(&"until"))
    {
        let end_idx = tokens
            .iter()
            .rposition(|token| token.is_word("until"))
            .unwrap_or(tokens.len());
        let remainder = trim_commas(&tokens[..end_idx]);
        return Ok(Some((Until::YourNextTurn, remainder)));
    }

    if all_words.ends_with(&["during", "your", "next", "untap", "step"]) {
        let during_idx = tokens
            .iter()
            .rposition(|token| token.is_word("during"))
            .unwrap_or(tokens.len());
        let remainder = trim_commas(&tokens[..during_idx]);
        if !remainder.is_empty() {
            return Ok(Some((Until::ControllersNextUntapStep, remainder)));
        }
    }

    if all_words.ends_with(&["during", "its", "controller", "next", "untap", "step"])
        || all_words.ends_with(&["during", "its", "controllers", "next", "untap", "step"])
        || all_words.ends_with(&["during", "their", "controller", "next", "untap", "step"])
        || all_words.ends_with(&["during", "their", "controllers", "next", "untap", "step"])
    {
        let during_idx = tokens
            .iter()
            .rposition(|token| token.is_word("during"))
            .unwrap_or(tokens.len());
        let remainder = trim_commas(&tokens[..during_idx]);
        if !remainder.is_empty() {
            return Ok(Some((Until::ControllersNextUntapStep, remainder)));
        }
    }

    let suffix_idx = tokens.windows(4).position(|window| {
        window[0].is_word("for")
            && window[1].is_word("as")
            && window[2].is_word("long")
            && window[3].is_word("as")
    });
    if let Some(idx) = suffix_idx {
        let suffix_words = words(&tokens[idx..]);
        let as_long_duration = suffix_words.contains(&"you")
            && suffix_words.contains(&"control")
            && (suffix_words.contains(&"this")
                || suffix_words.contains(&"thiss")
                || suffix_words.contains(&"source")
                || suffix_words.contains(&"creature")
                || suffix_words.contains(&"permanent"));
        if as_long_duration {
            let remainder = trim_commas(&tokens[..idx]);
            return Ok(Some((Until::YouStopControllingThis, remainder)));
        }
    }

    let has_this_turn = all_words
        .windows(2)
        .any(|window| window == ["this", "turn"]);
    if has_this_turn {
        let mut cleaned = Vec::new();
        let mut idx = 0usize;
        while idx < tokens.len() {
            if tokens[idx].is_word("this")
                && tokens
                    .get(idx + 1)
                    .is_some_and(|token| token.is_word("turn"))
            {
                idx += 2;
                continue;
            }
            cleaned.push(tokens[idx].clone());
            idx += 1;
        }
        let remainder = trim_commas(&cleaned).to_vec();
        if !remainder.is_empty() {
            return Ok(Some((Until::EndOfTurn, remainder)));
        }
    }

    Ok(None)
}

pub(crate) fn parse_play_from_graveyard_sentence(
    tokens: &[Token],
) -> Result<Option<EffectAst>, CardTextError> {
    let line_words = words(tokens);
    if line_words.len() < 8 || !starts_with_until_end_of_turn(&line_words) {
        return Ok(None);
    }

    let comma_idx = tokens
        .iter()
        .position(|token| matches!(token, Token::Comma(_)));
    let remainder = if let Some(idx) = comma_idx {
        &tokens[idx + 1..]
    } else {
        &tokens[4..]
    };

    let remaining_words: Vec<&str> = words(remainder)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();

    let expected = [
        "you",
        "may",
        "play",
        "lands",
        "and",
        "cast",
        "spells",
        "from",
        "your",
        "graveyard",
    ];

    if remaining_words == expected {
        return Ok(Some(EffectAst::PlayFromGraveyardUntilEot {
            player: PlayerAst::You,
        }));
    }

    Ok(None)
}

pub(crate) fn parse_exile_instead_of_graveyard_sentence(
    tokens: &[Token],
) -> Result<Option<EffectAst>, CardTextError> {
    let line_words = words(tokens);
    if line_words.first().copied() != Some("if") {
        return Ok(None);
    }

    let has_graveyard_clause = line_words
        .windows(4)
        .any(|w| w == ["into", "your", "graveyard", "from"])
        || line_words
            .windows(3)
            .any(|w| w == ["your", "graveyard", "from"])
        || (line_words.contains(&"your") && line_words.contains(&"graveyard"));
    let has_would_put = line_words
        .windows(4)
        .any(|w| w == ["card", "would", "be", "put"]);
    let has_this_turn = line_words.contains(&"this") && line_words.contains(&"turn");
    if !has_graveyard_clause || !has_would_put || !has_this_turn {
        return Ok(None);
    }

    let comma_idx = tokens
        .iter()
        .position(|token| matches!(token, Token::Comma(_)));
    let remainder = if let Some(idx) = comma_idx {
        &tokens[idx + 1..]
    } else {
        return Ok(None);
    };

    let remaining_words: Vec<&str> = words(remainder)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();

    let expected = ["exile", "that", "card", "instead"];
    if remaining_words == expected {
        return Ok(Some(EffectAst::ExileInsteadOfGraveyardThisTurn {
            player: PlayerAst::You,
        }));
    }

    Ok(None)
}
