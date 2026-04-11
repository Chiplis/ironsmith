pub(super) fn parse_predicate(tokens: &[OwnedLexToken]) -> Result<PredicateAst, CardTextError> {
    let raw_words_view = GrammarFilterNormalizedWords::new(tokens);
    let raw_words = raw_words_view.to_word_refs();
    let mut filtered: Vec<&str> = raw_words
        .iter()
        .copied()
        .filter(|word| !is_article(word))
        .collect();

    if filtered.is_empty() {
        return Err(CardTextError::ParseError(
            "empty predicate in if clause".to_string(),
        ));
    }

    for (phrase, zone) in [
        (
            ["this", "card", "is", "in", "your", "hand"].as_slice(),
            Zone::Hand,
        ),
        (
            ["this", "card", "is", "in", "your", "graveyard"].as_slice(),
            Zone::Graveyard,
        ),
        (
            ["this", "card", "is", "in", "your", "library"].as_slice(),
            Zone::Library,
        ),
        (
            ["this", "card", "is", "in", "exile"].as_slice(),
            Zone::Exile,
        ),
        (
            ["this", "card", "is", "in", "the", "command", "zone"].as_slice(),
            Zone::Command,
        ),
    ] {
        if filtered.as_slice() == phrase {
            return Ok(PredicateAst::SourceIsInZone(zone));
        }
    }

    if let Some(predicate) = parse_graveyard_threshold_predicate(&filtered)? {
        return Ok(predicate);
    }

    if let Some(gets_idx) = find_index(&filtered, |word| *word == "gets")
        && gets_idx > 0
        && filtered[gets_idx + 1..] == ["more", "votes"]
    {
        return Ok(PredicateAst::VoteOptionGetsMoreVotes {
            option: filtered[..gets_idx].join(" "),
        });
    }

    if let Some(gets_idx) = find_index(&filtered, |word| *word == "gets")
        && gets_idx > 0
        && filtered[gets_idx + 1..] == ["more", "votes", "or", "vote", "is", "tied"]
    {
        return Ok(PredicateAst::VoteOptionGetsMoreVotesOrTied {
            option: filtered[..gets_idx].join(" "),
        });
    }

    if filtered.len() >= 4
        && filtered[0] == "no"
        && filtered[filtered.len() - 2..] == ["got", "votes"]
    {
        let filter_tokens = filtered[1..filtered.len() - 2]
            .iter()
            .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
            .collect::<Vec<_>>();
        let filter = parse_object_filter(&filter_tokens, false)?;
        return Ok(PredicateAst::NoVoteObjectsMatched { filter });
    }

    if let Some(attacking_idx) = find_word_slice_phrase_start(
        &filtered,
        &[
            "are",
            "attacking",
            "and",
            "you",
            "both",
            "own",
            "and",
            "control",
            "them",
        ],
    ) && let Some(and_idx) = find_meld_subject_split(&filtered[..attacking_idx])
    {
        let left_words = &filtered[..and_idx];
        let right_words = &filtered[and_idx + 1..attacking_idx];
        if !left_words.is_empty() && !right_words.is_empty() {
            let mut left_filter = parse_meld_subject_filter(left_words).map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported attacking meld predicate subject (predicate: '{}')",
                    filtered.join(" ")
                ))
            })?;
            left_filter.controller = Some(PlayerFilter::You);
            left_filter.attacking = true;

            let mut right_filter = parse_meld_subject_filter(right_words).map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported attacking meld predicate tail (predicate: '{}')",
                    filtered.join(" ")
                ))
            })?;
            right_filter.controller = Some(PlayerFilter::You);
            right_filter.attacking = true;

            return Ok(PredicateAst::And(
                Box::new(PredicateAst::PlayerControls {
                    player: PlayerAst::You,
                    filter: left_filter,
                }),
                Box::new(PredicateAst::PlayerControls {
                    player: PlayerAst::You,
                    filter: right_filter,
                }),
            ));
        }
    }

    if filtered.len() >= 8
        && filtered[0] == "you"
        && filtered[1] == "both"
        && filtered[2] == "own"
        && filtered[3] == "and"
        && (filtered[4] == "control" || filtered[4] == "controls")
        && let Some(and_idx) = find_meld_subject_split(&filtered[5..])
    {
        let and_idx = 5 + and_idx;
        if and_idx > 5 && and_idx + 1 < filtered.len() {
            let mut left_filter =
                parse_meld_subject_filter(&filtered[5..and_idx]).map_err(|_| {
                    CardTextError::ParseError(format!(
                        "unsupported own-and-control predicate subject (predicate: '{}')",
                        filtered.join(" ")
                    ))
                })?;
            left_filter.controller = Some(PlayerFilter::You);
            let mut right_filter =
                parse_meld_subject_filter(&filtered[and_idx + 1..]).map_err(|_| {
                    CardTextError::ParseError(format!(
                        "unsupported own-and-control predicate tail (predicate: '{}')",
                        filtered.join(" ")
                    ))
                })?;
            right_filter.controller = Some(PlayerFilter::You);
            return Ok(PredicateAst::And(
                Box::new(PredicateAst::PlayerControls {
                    player: PlayerAst::You,
                    filter: left_filter,
                }),
                Box::new(PredicateAst::PlayerControls {
                    player: PlayerAst::You,
                    filter: right_filter,
                }),
            ));
        }
    }

    if let Some(and_idx) = find_index(&filtered, |word| *word == "and")
        && and_idx > 0
        && and_idx + 1 < filtered.len()
    {
        let right_first = filtered.get(and_idx + 1).copied();
        if matches!(right_first, Some("have") | Some("you")) {
            let left_words = &filtered[..and_idx];
            let mut right_words = filtered[and_idx + 1..].to_vec();
            if right_words.first().copied() == Some("have") {
                right_words.insert(0, "you");
            }
            let left_tokens = left_words
                .iter()
                .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
                .collect::<Vec<_>>();
            let right_tokens = right_words
                .iter()
                .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
                .collect::<Vec<_>>();
            let left = parse_predicate(&left_tokens)?;
            let right = parse_predicate(&right_tokens)?;
            return Ok(PredicateAst::And(Box::new(left), Box::new(right)));
        }
    }

    if filtered.as_slice() == ["this", "tapped"]
        || filtered.as_slice() == ["thiss", "tapped"]
        || ((filtered.first().copied() == Some("this")
            || filtered.first().copied() == Some("thiss"))
            && filtered.last().copied() == Some("tapped"))
    {
        return Ok(PredicateAst::SourceIsTapped);
    }

    if filtered.as_slice() == ["this", "untapped"]
        || filtered.as_slice() == ["thiss", "untapped"]
        || filtered.as_slice() == ["this", "is", "untapped"]
        || filtered.as_slice() == ["this", "creature", "is", "untapped"]
        || filtered.as_slice() == ["this", "permanent", "is", "untapped"]
        || ((filtered.first().copied() == Some("this")
            || filtered.first().copied() == Some("thiss"))
            && filtered.last().copied() == Some("untapped"))
    {
        return Ok(PredicateAst::Not(Box::new(PredicateAst::SourceIsTapped)));
    }

    if filtered.as_slice() == ["this", "creature", "isnt", "saddled"]
        || filtered.as_slice() == ["this", "permanent", "isnt", "saddled"]
        || filtered.as_slice() == ["this", "isnt", "saddled"]
        || filtered.as_slice() == ["it", "isnt", "saddled"]
    {
        return Ok(PredicateAst::Not(Box::new(PredicateAst::SourceIsSaddled)));
    }

    if filtered.as_slice() == ["this", "creature", "is", "saddled"]
        || filtered.as_slice() == ["this", "permanent", "is", "saddled"]
        || filtered.as_slice() == ["this", "is", "saddled"]
        || filtered.as_slice() == ["it", "is", "saddled"]
    {
        return Ok(PredicateAst::SourceIsSaddled);
    }

    if slice_starts_with(&filtered, &["there", "are", "no"])
        && slice_contains(&filtered, &"counters")
        && contains_any_filter_phrase(&filtered, &[&["on", "this"]])
        && let Some(counters_idx) = find_index(&filtered, |word| *word == "counters")
        && counters_idx >= 4
        && let Some(counter_type) = parse_counter_type_word(filtered[counters_idx - 1])
    {
        return Ok(PredicateAst::SourceHasNoCounter(counter_type));
    }

    let source_has_counter_prefix_len = if slice_starts_with(&raw_words, &["this", "has"]) {
        Some(2)
    } else if raw_words.len() >= 3
        && raw_words[0] == "this"
        && matches!(
            raw_words[1],
            "creature"
                | "permanent"
                | "artifact"
                | "enchantment"
                | "land"
                | "planeswalker"
                | "battle"
        )
        && raw_words[2] == "has"
    {
        Some(3)
    } else {
        None
    };
    if let Some(prefix_len) = source_has_counter_prefix_len
        && raw_words.len() >= prefix_len + 4
        && raw_words[prefix_len] == "no"
        && let Some(counter_type) = parse_counter_type_word(raw_words[prefix_len + 1])
        && matches!(raw_words[prefix_len + 2], "counter" | "counters")
        && raw_words[prefix_len + 3] == "on"
        && matches!(
            raw_words.get(prefix_len + 4).copied(),
            Some("it" | "him" | "her" | "them" | "this" | "that")
        )
    {
        return Ok(PredicateAst::SourceHasNoCounter(counter_type));
    }

    let triggering_object_had_no_counter_prefix_len =
        if slice_starts_with(&raw_words, &["it", "had", "no"]) {
            Some(3)
        } else if slice_starts_with(&raw_words, &["this", "creature", "had", "no"])
            || slice_starts_with(&raw_words, &["that", "creature", "had", "no"])
            || slice_starts_with(&raw_words, &["this", "permanent", "had", "no"])
            || slice_starts_with(&raw_words, &["that", "permanent", "had", "no"])
        {
            Some(4)
        } else {
            None
        };
    if let Some(prefix_len) = triggering_object_had_no_counter_prefix_len
        && raw_words.len() >= prefix_len + 4
        && let Some(counter_type) = parse_counter_type_word(raw_words[prefix_len])
        && matches!(raw_words[prefix_len + 1], "counter" | "counters")
        && raw_words[prefix_len + 2] == "on"
        && matches!(
            raw_words[prefix_len + 3],
            "it" | "them" | "this" | "that" | "itself"
        )
    {
        return Ok(PredicateAst::TriggeringObjectHadNoCounter(counter_type));
    }

    if slice_starts_with(&raw_words, &["there", "are"])
        && raw_words.get(3).copied() == Some("or")
        && raw_words.get(4).copied() == Some("more")
        && raw_words
            .iter()
            .any(|w| *w == "counter" || *w == "counters")
    {
        if let Some((count, used)) = parse_number(&tokens[2..]) {
            let rest = &tokens[2 + used..];
            let rest_words = crate::cards::builders::compiler::token_word_refs(rest);
            if rest_words.len() >= 4
                && rest_words[0] == "or"
                && rest_words[1] == "more"
                && (rest_words[3] == "counter" || rest_words[3] == "counters")
                && let Some(counter_type) = parse_counter_type_word(rest_words[2])
            {
                return Ok(PredicateAst::SourceHasCounterAtLeast {
                    counter_type,
                    count,
                });
            }
        }
    }

    if let Some(prefix_len) = source_has_counter_prefix_len
        && raw_words.len() >= prefix_len + 6
        && let Some(count) = parse_named_number(raw_words[prefix_len])
        && raw_words[prefix_len + 1] == "or"
        && raw_words[prefix_len + 2] == "more"
        && let Some(counter_type) = parse_counter_type_word(raw_words[prefix_len + 3])
        && matches!(raw_words[prefix_len + 4], "counter" | "counters")
        && raw_words[prefix_len + 5] == "on"
        && matches!(
            raw_words.get(prefix_len + 6).copied(),
            Some("it" | "him" | "her" | "them" | "this" | "that")
        )
    {
        return Ok(PredicateAst::SourceHasCounterAtLeast {
            counter_type,
            count,
        });
    }

    if filtered.len() == 7
        && matches!(
            &filtered[..4],
            ["this", "creature", "power", "is"]
                | ["this", "creatures", "power", "is"]
                | ["this", "permanent", "power", "is"]
                | ["this", "permanents", "power", "is"]
        )
        && filtered[5] == "or"
        && filtered[6] == "more"
        && let Some(count_word) = filtered.get(4).copied()
        && let Some(count) = parse_named_number(count_word)
    {
        return Ok(PredicateAst::SourcePowerAtLeast(count));
    }

    if filtered.len() >= 10 && filtered[0] == "there" && filtered[1] == "are" {
        let mut idx = 2usize;
        if let Some(count) = parse_named_number(filtered[idx]) {
            idx += 1;
            if filtered.get(idx).copied() == Some("or")
                && filtered.get(idx + 1).copied() == Some("more")
            {
                idx += 2;
            }
            let looks_like_basic_land_type_clause = filtered.get(idx).copied() == Some("basic")
                && filtered.get(idx + 1).copied() == Some("land")
                && matches!(filtered.get(idx + 2).copied(), Some("type" | "types"))
                && filtered.get(idx + 3).copied() == Some("among")
                && matches!(filtered.get(idx + 4).copied(), Some("land" | "lands"));
            if looks_like_basic_land_type_clause {
                let tail = &filtered[idx + 5..];
                let player = if tail == ["that", "player", "controls"]
                    || tail == ["that", "player", "control"]
                    || tail == ["that", "players", "controls"]
                {
                    PlayerAst::That
                } else if tail == ["you", "control"] || tail == ["you", "controls"] {
                    PlayerAst::You
                } else {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported basic-land-types predicate tail (predicate: '{}')",
                        filtered.join(" ")
                    )));
                };

                return Ok(PredicateAst::PlayerControlsBasicLandTypesAmongLandsOrMore {
                    player,
                    count,
                });
            }
        }
    }

    if filtered.len() >= 7
        && filtered[0] == "there"
        && filtered[1] == "are"
        && let Some(count) = parse_named_number(filtered[2])
    {
        let mut idx = 3usize;
        if filtered.get(idx).copied() == Some("or")
            && filtered.get(idx + 1).copied() == Some("more")
        {
            idx += 2;
        }

        let battlefield_suffix_len =
            if slice_ends_with(&filtered[idx..], &["on", "the", "battlefield"]) {
                Some(3usize)
            } else if slice_ends_with(&filtered[idx..], &["on", "battlefield"]) {
                Some(2usize)
            } else {
                None
            };
        if let Some(battlefield_suffix_len) = battlefield_suffix_len {
            let raw_filter_words = &filtered[idx..filtered.len() - battlefield_suffix_len];
            let other = raw_filter_words
                .first()
                .is_some_and(|word| matches!(*word, "other" | "another"));
            let filter_words = if other {
                &raw_filter_words[1..]
            } else {
                raw_filter_words
            };
            if !filter_words.is_empty() {
                let filter_tokens = filter_words
                    .iter()
                    .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
                    .collect::<Vec<_>>();
                if let Ok(mut filter) = parse_object_filter(&filter_tokens, other) {
                    filter.zone = Some(Zone::Battlefield);

                    return Ok(PredicateAst::ValueComparison {
                        left: Value::Count(filter),
                        operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                        right: Value::Fixed(count as i32),
                    });
                }
            }
        }
    }

    let parse_graveyard_card_types_subject = |words: &[&str]| -> Option<PlayerAst> {
        match words {
            [first, second] if *first == "your" && *second == "graveyard" => Some(PlayerAst::You),
            [first, second, third]
                if *first == "that"
                    && (*second == "player" || *second == "players")
                    && *third == "graveyard" =>
            {
                Some(PlayerAst::That)
            }
            [first, second, third]
                if *first == "target"
                    && (*second == "player" || *second == "players")
                    && *third == "graveyard" =>
            {
                Some(PlayerAst::Target)
            }
            [first, second, third]
                if *first == "target"
                    && (*second == "opponent" || *second == "opponents")
                    && *third == "graveyard" =>
            {
                Some(PlayerAst::TargetOpponent)
            }
            [first, second]
                if (*first == "opponent" || *first == "opponents") && *second == "graveyard" =>
            {
                Some(PlayerAst::Opponent)
            }
            _ => None,
        }
    };
    if filtered.len() >= 11 {
        let (count_idx, subject_start, constrained_player) =
            if filtered[0] == "there" && filtered[1] == "are" {
                (2usize, 10usize, None)
            } else if filtered[0] == "you" && filtered[1] == "have" {
                (2usize, 10usize, Some(PlayerAst::You))
            } else {
                (usize::MAX, usize::MAX, None)
            };
        if count_idx != usize::MAX
            && filtered.get(count_idx + 1).copied() == Some("or")
            && filtered.get(count_idx + 2).copied() == Some("more")
            && filtered.get(count_idx + 3).copied() == Some("card")
            && matches!(filtered.get(count_idx + 4).copied(), Some("type" | "types"))
            && filtered.get(count_idx + 5).copied() == Some("among")
            && matches!(filtered.get(count_idx + 6).copied(), Some("card" | "cards"))
            && filtered.get(count_idx + 7).copied() == Some("in")
            && subject_start <= filtered.len()
            && let Some(count) = parse_named_number(filtered[count_idx])
            && let Some(player) = parse_graveyard_card_types_subject(&filtered[subject_start..])
            && constrained_player.map_or(true, |expected| expected == player)
        {
            return Ok(PredicateAst::PlayerHasCardTypesInGraveyardOrMore { player, count });
        }
    }

    let parse_comparison_player_subject = |words: &[&str]| -> Option<(PlayerAst, usize)> {
        match words {
            [first, second, ..] if *first == "that" && *second == "player" => {
                Some((PlayerAst::That, 2))
            }
            [first, second, ..] if *first == "target" && *second == "player" => {
                Some((PlayerAst::Target, 2))
            }
            [first, second, ..] if *first == "target" && *second == "opponent" => {
                Some((PlayerAst::TargetOpponent, 2))
            }
            [first, second, ..] if *first == "each" && *second == "opponent" => {
                Some((PlayerAst::Opponent, 2))
            }
            [first, second, ..] if (*first == "a" || *first == "any") && *second == "player" => {
                Some((PlayerAst::Any, 2))
            }
            [first, second, ..] if *first == "defending" && *second == "player" => {
                Some((PlayerAst::Defending, 2))
            }
            [first, second, ..] if *first == "attacking" && *second == "player" => {
                Some((PlayerAst::Attacking, 2))
            }
            [first, ..] if *first == "you" => Some((PlayerAst::You, 1)),
            [first, ..] if *first == "opponent" || *first == "opponents" => {
                Some((PlayerAst::Opponent, 1))
            }
            [first, second, ..] if *first == "player" && *second == "who" => {
                Some((PlayerAst::That, 1))
            }
            [first, ..] if *first == "player" => Some((PlayerAst::Any, 1)),
            _ => None,
        }
    };
    let parse_life_total_subject = |words: &[&str]| -> Option<(PlayerAst, usize)> {
        match words {
            ["your", "life", "total", ..] => Some((PlayerAst::You, 3)),
            ["their", "life", "total", ..] => Some((PlayerAst::That, 3)),
            ["that", "players", "life", "total", ..] => Some((PlayerAst::That, 4)),
            ["target", "players", "life", "total", ..] => Some((PlayerAst::Target, 4)),
            ["target", "opponents", "life", "total", ..] => Some((PlayerAst::TargetOpponent, 4)),
            ["opponents", "life", "total", ..] | ["opponent", "life", "total", ..] => {
                Some((PlayerAst::Opponent, 3))
            }
            ["defending", "players", "life", "total", ..] => Some((PlayerAst::Defending, 4)),
            ["attacking", "players", "life", "total", ..] => Some((PlayerAst::Attacking, 4)),
            _ => None,
        }
    };
    let half_starting_tail_matches = |tail: &[&str]| {
        matches!(
            tail,
            ["half", "your", "starting", "life", "total"]
                | ["half", "their", "starting", "life", "total"]
                | ["half", "that", "players", "starting", "life", "total"]
                | ["half", "target", "players", "starting", "life", "total"]
                | ["half", "target", "opponents", "starting", "life", "total"]
                | ["half", "opponents", "starting", "life", "total"]
                | ["half", "defending", "players", "starting", "life", "total"]
                | ["half", "attacking", "players", "starting", "life", "total"]
        )
    };
    if let Some((player, subject_len)) = parse_life_total_subject(&filtered)
        && filtered.get(subject_len).copied() == Some("is")
    {
        let tail = &filtered[subject_len + 1..];
        if let Some(rest) = slice_strip_prefix(tail, &["less", "than", "or", "equal", "to"])
            && half_starting_tail_matches(rest)
        {
            return Ok(PredicateAst::PlayerLifeAtMostHalfStartingLifeTotal { player });
        }
        if let Some(rest) = slice_strip_prefix(tail, &["less", "than"])
            && half_starting_tail_matches(rest)
        {
            return Ok(PredicateAst::PlayerLifeLessThanHalfStartingLifeTotal { player });
        }
    }
    if let Some((player, subject_len)) = parse_comparison_player_subject(&filtered)
        && matches!(
            filtered.get(subject_len).copied(),
            Some("control" | "controls")
        )
        && filtered.get(subject_len + 1).copied() == Some("more")
        && let Some(than_offset) = find_index(&filtered[subject_len + 2..], |word| *word == "than")
    {
        let than_idx = subject_len + 2 + than_offset;
        let tail = &filtered[than_idx..];
        if matches!(tail, ["than", "you"] | ["than", "you", "do"]) {
            let filter_tokens = filtered[subject_len + 2..than_idx]
                .iter()
                .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
                .collect::<Vec<_>>();
            if !filter_tokens.is_empty() {
                let other = filter_tokens
                    .first()
                    .is_some_and(|token| token.is_word("another") || token.is_word("other"));
                if let Ok(filter) = parse_object_filter(&filter_tokens, other)
                    && filter != ObjectFilter::default()
                {
                    return Ok(PredicateAst::PlayerControlsMoreThanYou { player, filter });
                }
            }
        }
    }

    if let Some((player, subject_len)) = parse_comparison_player_subject(&filtered)
        && filtered.get(subject_len).copied() == Some("has")
        && matches!(
            &filtered[subject_len + 1..],
            ["more", "life", "than", "you"] | ["more", "life", "than", "you", "do"]
        )
    {
        return Ok(PredicateAst::PlayerHasMoreLifeThanYou { player });
    }

    if filtered.len() >= 8
        && filtered[0] == "no"
        && matches!(filtered[1], "opponent" | "opponents")
        && filtered[2] == "has"
        && filtered[3] == "more"
        && filtered[4] == "life"
        && filtered[5] == "than"
        && let Some((player, subject_len)) = parse_comparison_player_subject(&filtered[6..])
        && subject_len + 6 == filtered.len()
    {
        return Ok(PredicateAst::PlayerHasNoOpponentWithMoreLifeThan { player });
    }

    if let Some((player, subject_len)) = parse_comparison_player_subject(&filtered)
        && filtered.get(subject_len).copied() == Some("has")
        && matches!(
            &filtered[subject_len + 1..],
            ["more", "life", "than", "each", "other", "player"]
                | ["more", "life", "than", "each", "other", "players"]
        )
    {
        return Ok(PredicateAst::PlayerHasMoreLifeThanEachOtherPlayer { player });
    }

    if let Some((player, subject_len)) = parse_comparison_player_subject(&filtered)
        && filtered.get(subject_len).copied() == Some("has")
        && matches!(
            &filtered[subject_len + 1..],
            ["more", "card", "in", "hand", "than", "you"]
                | ["more", "cards", "in", "hand", "than", "you"]
                | ["more", "card", "in", "their", "hand", "than", "you"]
                | ["more", "cards", "in", "their", "hand", "than", "you"]
                | ["more", "card", "in", "hand", "than", "you", "do"]
                | ["more", "cards", "in", "hand", "than", "you", "do"]
                | ["more", "card", "in", "their", "hand", "than", "you", "do"]
                | ["more", "cards", "in", "their", "hand", "than", "you", "do"]
        )
    {
        return Ok(PredicateAst::PlayerHasMoreCardsInHandThanYou { player });
    }

    if let Some((player, subject_len)) = parse_comparison_player_subject(&filtered)
        && filtered.get(subject_len).copied() == Some("has")
        && matches!(
            &filtered[subject_len + 1..],
            [
                "more", "card", "in", "hand", "than", "each", "other", "player"
            ] | [
                "more", "cards", "in", "hand", "than", "each", "other", "player"
            ] | [
                "more", "card", "in", "their", "hand", "than", "each", "other", "player",
            ] | [
                "more", "cards", "in", "their", "hand", "than", "each", "other", "player",
            ]
        )
    {
        return Ok(PredicateAst::PlayerHasMoreCardsInHandThanEachOtherPlayer { player });
    }

    if let Some((player, subject_len)) = parse_comparison_player_subject(&filtered)
        && filtered.get(subject_len).copied() == Some("has")
        && let Some(count_word) = filtered.get(subject_len + 1).copied()
        && let Some(count) = parse_named_number(count_word)
        && filtered.get(subject_len + 2).copied() == Some("or")
        && let Some(comp_word) = filtered.get(subject_len + 3).copied()
        && matches!(comp_word, "more" | "fewer" | "less")
        && matches!(
            filtered.get(subject_len + 4).copied(),
            Some("card" | "cards")
        )
        && filtered.get(subject_len + 5).copied() == Some("in")
        && filtered.get(subject_len + 6).copied() == Some("hand")
        && filtered.len() == subject_len + 7
    {
        return Ok(if comp_word == "more" {
            PredicateAst::PlayerCardsInHandOrMore { player, count }
        } else {
            PredicateAst::PlayerCardsInHandOrFewer { player, count }
        });
    }

    if filtered.as_slice() == ["you", "have", "no", "cards", "in", "hand"] {
        return Ok(PredicateAst::YouHaveNoCardsInHand);
    }

    if matches!(
        filtered.as_slice(),
        ["it", "your", "turn"] | ["its", "your", "turn"] | ["your", "turn"]
    ) {
        return Ok(PredicateAst::YourTurn);
    }

    if matches!(
        filtered.as_slice(),
        ["creature", "died", "this", "turn"] | ["creatures", "died", "this", "turn"]
    ) {
        return Ok(PredicateAst::CreatureDiedThisTurn);
    }

    if filtered.len() == 7
        && let Some(count) = parse_named_number(filtered[0])
        && filtered[1..] == ["or", "more", "creatures", "died", "this", "turn"]
    {
        return Ok(PredicateAst::CreatureDiedThisTurnOrMore(count));
    }

    if matches!(
        filtered.as_slice(),
        [
            "permanent",
            "left",
            "battlefield",
            "under",
            "your",
            "control",
            "this",
            "turn"
        ] | [
            "permanents",
            "left",
            "battlefield",
            "under",
            "your",
            "control",
            "this",
            "turn"
        ] | [
            "permanent",
            "you",
            "controlled",
            "left",
            "battlefield",
            "this",
            "turn"
        ] | [
            "permanents",
            "you",
            "controlled",
            "left",
            "battlefield",
            "this",
            "turn"
        ]
    ) {
        return Ok(PredicateAst::PermanentLeftBattlefieldUnderYourControlThisTurn);
    }

    if matches!(
        filtered.as_slice(),
        [
            "you",
            "had",
            "land",
            "enter",
            "battlefield",
            "under",
            "your",
            "control",
            "this",
            "turn"
        ] | [
            "you",
            "had",
            "land",
            "entered",
            "battlefield",
            "under",
            "your",
            "control",
            "this",
            "turn"
        ] | [
            "you",
            "had",
            "lands",
            "enter",
            "battlefield",
            "under",
            "your",
            "control",
            "this",
            "turn"
        ] | [
            "you",
            "had",
            "lands",
            "entered",
            "battlefield",
            "under",
            "your",
            "control",
            "this",
            "turn"
        ]
    ) {
        return Ok(PredicateAst::PlayerHadLandEnterBattlefieldThisTurn {
            player: PlayerAst::You,
        });
    }

    if filtered.len() >= 7
        && filtered[0] == "you"
        && filtered[1] == "gained"
        && let Some((count, used)) = parse_number(&tokens[2..])
        && filtered[2 + used..] == ["or", "more", "life", "this", "turn"]
    {
        return Ok(PredicateAst::PlayerGainedLifeThisTurnOrMore {
            player: PlayerAst::You,
            count: count as u32,
        });
    }

    if filtered.as_slice() == ["you", "gained", "life", "this", "turn"] {
        return Ok(PredicateAst::PlayerGainedLifeThisTurnOrMore {
            player: PlayerAst::You,
            count: 1,
        });
    }

    if filtered.as_slice() == ["you", "attacked", "this", "turn"] {
        return Ok(PredicateAst::YouAttackedThisTurn);
    }

    if filtered.len() == 9
        && filtered[0] == "you"
        && filtered[1] == "attacked"
        && filtered[2] == "with"
        && filtered[3] == "exactly"
        && matches!(filtered[5], "other" | "others")
        && matches!(filtered[6], "creature" | "creatures")
        && filtered[7] == "this"
        && filtered[8] == "combat"
        && let Some(count) = parse_named_number(filtered[4])
    {
        return Ok(PredicateAst::YouAttackedWithExactlyNOtherCreaturesThisCombat(count));
    }

    if matches!(
        filtered.as_slice(),
        [
            "this", "creature", "attacked", "or", "blocked", "this", "turn"
        ] | [
            "this",
            "permanent",
            "attacked",
            "or",
            "blocked",
            "this",
            "turn"
        ] | ["this", "attacked", "or", "blocked", "this", "turn"]
            | ["it", "attacked", "or", "blocked", "this", "turn"]
    ) {
        return Ok(PredicateAst::SourceAttackedOrBlockedThisTurn);
    }

    if filtered.as_slice() == ["you", "cast", "it"]
        || filtered.as_slice() == ["you", "cast", "this", "spell"]
    {
        return Ok(PredicateAst::SourceWasCast);
    }

    if filtered.len() >= 6
        && filtered[0] == "this"
        && filtered[1] == "spell"
        && filtered[2] == "was"
        && filtered[3] == "cast"
        && filtered[4] == "from"
    {
        let zone_words = &filtered[5..];
        let zone = if zone_words.len() == 1 {
            parse_zone_word(zone_words[0])
        } else if zone_words.len() == 2 && is_article(zone_words[0]) {
            parse_zone_word(zone_words[1])
        } else if zone_words.len() == 2 && zone_words[0] == "the" {
            parse_zone_word(zone_words[1])
        } else {
            None
        };

        if let Some(zone) = zone {
            return Ok(PredicateAst::ThisSpellWasCastFromZone(zone));
        }
    }

    if filtered.as_slice() == ["no", "spells", "were", "cast", "last", "turn"]
        || filtered.as_slice() == ["no", "spell", "was", "cast", "last", "turn"]
    {
        return Ok(PredicateAst::NoSpellsWereCastLastTurn);
    }
    if filtered.as_slice() == ["this", "spell", "was", "kicked"] {
        return Ok(PredicateAst::ThisSpellWasKicked);
    }
    if filtered.as_slice() == ["this", "spell", "was", "bargained"]
        || filtered.as_slice() == ["it", "was", "bargained"]
    {
        return Ok(PredicateAst::ThisSpellPaidLabel("Bargain".to_string()));
    }
    if filtered.len() == 4
        && matches!(filtered[0], "a" | "an")
        && parse_subtype_word(filtered[1]).is_some()
        && matches!(filtered[2], "was" | "were")
        && filtered[3] == "beheld"
    {
        return Ok(PredicateAst::ThisSpellPaidLabel("Behold".to_string()));
    }
    if filtered.len() == 3
        && parse_subtype_word(filtered[0]).is_some()
        && matches!(filtered[1], "was" | "were")
        && filtered[2] == "beheld"
    {
        return Ok(PredicateAst::ThisSpellPaidLabel("Behold".to_string()));
    }
    if filtered.as_slice() == ["gift", "was", "promised"] {
        return Ok(PredicateAst::ThisSpellPaidLabel("Gift".to_string()));
    }
    if filtered.len() == 6
        && filtered[0] == "this"
        && matches!(
            filtered[1],
            "spell's" | "card's" | "creature's" | "permanent's"
        )
        && filtered[3] == "cost"
        && filtered[4] == "was"
        && filtered[5] == "paid"
    {
        let mut chars = filtered[2].chars();
        let Some(first) = chars.next() else {
            return Err(CardTextError::ParseError(
                "missing paid-cost label in predicate".to_string(),
            ));
        };
        let label = format!(
            "{}{}",
            first.to_ascii_uppercase(),
            chars.as_str().to_ascii_lowercase()
        );
        return Ok(PredicateAst::ThisSpellPaidLabel(label));
    }
    if filtered.as_slice() == ["it", "was", "kicked"] {
        return Ok(PredicateAst::ThisSpellWasKicked);
    }
    if filtered.as_slice() == ["that", "was", "kicked"] {
        return Ok(PredicateAst::TargetWasKicked);
    }

    if filtered.as_slice() == ["you", "have", "full", "party"] {
        return Ok(PredicateAst::YouHaveFullParty);
    }
    if filtered.as_slice() == ["its", "controller", "poisoned"]
        || filtered.as_slice() == ["that", "spells", "controller", "poisoned"]
    {
        return Ok(PredicateAst::TargetSpellControllerIsPoisoned);
    }
    if filtered.as_slice() == ["no", "mana", "was", "spent", "to", "cast", "it"]
        || filtered.as_slice() == ["no", "mana", "were", "spent", "to", "cast", "it"]
        || filtered.as_slice() == ["no", "mana", "was", "spent", "to", "cast", "that", "spell"]
        || filtered.as_slice() == ["no", "mana", "were", "spent", "to", "cast", "that", "spell"]
    {
        return Ok(PredicateAst::TargetSpellNoManaSpentToCast);
    }
    if filtered.as_slice()
        == [
            "you",
            "control",
            "more",
            "creatures",
            "than",
            "that",
            "spells",
            "controller",
        ]
        || filtered.as_slice()
            == [
                "you",
                "control",
                "more",
                "creatures",
                "than",
                "its",
                "controller",
            ]
    {
        return Ok(PredicateAst::YouControlMoreCreaturesThanTargetSpellController);
    }
    if filtered.len() == 7
        && matches!(filtered[0], "w" | "u" | "b" | "r" | "g" | "c" | "s")
        && filtered[1] == "was"
        && filtered[2] == "spent"
        && filtered[3] == "to"
        && filtered[4] == "cast"
        && filtered[5] == "this"
        && filtered[6] == "spell"
        && let Ok(symbol) = parse_mana_symbol(filtered[0])
    {
        return Ok(PredicateAst::ManaSpentToCastThisSpellAtLeast {
            amount: 1,
            symbol: Some(symbol),
        });
    }

    if let Some((amount, symbol)) = parse_mana_spent_to_cast_predicate(&filtered) {
        return Ok(PredicateAst::ManaSpentToCastThisSpellAtLeast { amount, symbol });
    }

    if filtered.len() >= 5
        && matches!(
            filtered.as_slice(),
            ["this", "permanent", "attached", "to", ..]
                | ["that", "permanent", "attached", "to", ..]
                | ["this", "permanent", "is", "attached", "to", ..]
                | ["that", "permanent", "is", "attached", "to", ..]
        )
    {
        let attached_start = if filtered.get(2).copied() == Some("is") {
            5
        } else {
            4
        };
        let attached_tokens = filtered[attached_start..]
            .iter()
            .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
            .collect::<Vec<_>>();
        let mut filter = parse_object_filter(&attached_tokens, false)?;
        if filter.card_types.is_empty() {
            filter.card_types.push(CardType::Creature);
        }
        return Ok(PredicateAst::TaggedMatches(
            TagKey::from("enchanted"),
            filter,
        ));
    }

    if filtered.len() >= 4 && filtered[0] == "sacrificed" && filtered[2] == "was" {
        let sacrificed_head = filtered[1];
        let subject_card_type =
            parse_card_type(sacrificed_head).filter(|card_type| is_permanent_type(*card_type));
        let subject_is_permanent = sacrificed_head == "permanent" || subject_card_type.is_some();

        if subject_is_permanent {
            let descriptor_tokens = filtered[3..]
                .iter()
                .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
                .collect::<Vec<_>>();
            let mut filter = parse_object_filter(&descriptor_tokens, false)?;
            if filter.card_types.is_empty() {
                if let Some(card_type) = subject_card_type {
                    filter.card_types.push(card_type);
                }
            }
            if filter.zone.is_none() && sacrificed_head == "permanent" {
                filter.zone = Some(Zone::Battlefield);
            }
            return Ok(PredicateAst::ItMatches(filter));
        }
    }

    if filtered.as_slice()
        == [
            "this", "is", "fourth", "time", "this", "ability", "has", "resolved", "this", "turn",
        ]
    {
        return Ok(PredicateAst::Unmodeled(filtered.join(" ")));
    }

    if matches!(
        filtered.as_slice(),
        ["any", "of", "those", "cards", "remain", "exiled"]
            | ["those", "cards", "remain", "exiled"]
            | ["that", "card", "remains", "exiled"]
            | ["it", "remains", "exiled"]
    ) {
        return Ok(PredicateAst::TaggedMatches(
            TagKey::from(IT_TAG),
            ObjectFilter::default().in_zone(Zone::Exile),
        ));
    }

    if filtered[0] == "its" {
        filtered[0] = "it";
    }
    if filtered.len() >= 2 && filtered[0] == "it" && filtered[1] == "s" {
        filtered.remove(1);
    }

    let demonstrative_reference_len = if filtered.first().copied() == Some("it") {
        Some(1usize)
    } else if filtered.len() >= 2
        && filtered[0] == "that"
        && matches!(
            filtered[1],
            "artifact"
                | "card"
                | "creature"
                | "land"
                | "object"
                | "permanent"
                | "source"
                | "spell"
                | "token"
        )
    {
        Some(2usize)
    } else {
        None
    };

    let is_it_soulbond_paired = matches!(
        filtered.as_slice(),
        ["it", "paired", "with", "creature"]
            | ["it", "paired", "with", "another", "creature"]
            | ["it", "s", "paired", "with", "creature"]
            | ["it", "s", "paired", "with", "another", "creature"]
    );
    if is_it_soulbond_paired {
        return Ok(PredicateAst::ItIsSoulbondPaired);
    }

    if filtered.len() >= 2 {
        let tag = if slice_starts_with(&filtered, &["equipped", "creature"]) {
            Some("equipped")
        } else if slice_starts_with(&filtered, &["enchanted", "creature"]) {
            Some("enchanted")
        } else {
            None
        };
        if let Some(tag) = tag {
            let remainder = filtered[2..].to_vec();
            let tokens = remainder
                .iter()
                .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
                .collect::<Vec<_>>();
            let mut filter = parse_object_filter(&tokens, false)?;
            if filter.card_types.is_empty() {
                filter.card_types.push(CardType::Creature);
            }
            return Ok(PredicateAst::TaggedMatches(TagKey::from(tag), filter));
        }
    }

    let onto_battlefield_idx = find_word_slice_phrase_start(&filtered, &["onto", "battlefield"])
        .or_else(|| find_word_slice_phrase_start(&filtered, &["onto", "the", "battlefield"]));
    if filtered.len() >= 7
        && filtered[0] == "you"
        && filtered[1] == "put"
        && slice_ends_with(&filtered, &["this", "way"])
        && let Some(onto_idx) = onto_battlefield_idx
    {
        let filter_words = &filtered[2..onto_idx];
        let filter_tokens = filter_words
            .iter()
            .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
            .collect::<Vec<_>>();
        let mut filter = parse_object_filter(&filter_tokens, false)?;
        if filter.zone.is_none() {
            filter.zone = Some(Zone::Battlefield);
        }
        return Ok(PredicateAst::PlayerTaggedObjectMatches {
            player: PlayerAst::You,
            tag: TagKey::from(IT_TAG),
            filter,
        });
    }

    let is_it = demonstrative_reference_len == Some(1);
    let has_card = demonstrative_reference_len
        .map(|reference_len| slice_contains(&filtered[reference_len..], &"card"))
        .unwrap_or(false);

    if is_it {
        if filtered
            .get(1)
            .is_some_and(|word| *word == "has" || *word == "have")
        {
            filtered.remove(1);
        }
        if filtered.len() >= 3 && filtered[1] == "mana" && filtered[2] == "value" {
            let mana_value_tail = if filtered
                .get(3)
                .is_some_and(|word| matches!(*word, "is" | "are" | "was" | "were"))
            {
                &filtered[4..]
            } else {
                &filtered[3..]
            };
            let compares_to_colors_spent = mana_value_tail
                == [
                    "less", "than", "or", "equal", "to", "number", "of", "colors", "of", "mana",
                    "spent", "to", "cast", "this", "spell",
                ]
                || mana_value_tail
                    == [
                        "less", "than", "or", "equal", "to", "number", "of", "color", "of", "mana",
                        "spent", "to", "cast", "this", "spell",
                    ];
            if compares_to_colors_spent {
                return Ok(PredicateAst::TargetManaValueLteColorsSpentToCastThisSpell);
            }

            if let Some((cmp, _consumed)) =
                parse_filter_comparison_tokens("mana value", mana_value_tail, &filtered)?
            {
                return Ok(PredicateAst::ItMatches(ObjectFilter {
                    mana_value: Some(cmp),
                    ..Default::default()
                }));
            }
        }

        if filtered.len() >= 3 && (filtered[1] == "power" || filtered[1] == "toughness") {
            let axis = filtered[1];
            let value_tail = &filtered[2..];
            if let Some((cmp, _consumed)) =
                parse_filter_comparison_tokens(axis, value_tail, &filtered)?
            {
                let mut filter = ObjectFilter::default();
                if axis == "power" {
                    filter.power = Some(cmp);
                } else {
                    filter.toughness = Some(cmp);
                }
                return Ok(PredicateAst::ItMatches(filter));
            }
        }
    }

    if let Some(reference_len) = demonstrative_reference_len {
        let mut descriptor_words = filtered[reference_len..].to_vec();
        if descriptor_words.as_slice() == ["has", "toxic"]
            || descriptor_words.as_slice() == ["have", "toxic"]
        {
            let mut filter = ObjectFilter::default().with_ability_marker("toxic");
            if filtered.get(1).copied() == Some("creature") {
                filter.card_types.push(CardType::Creature);
            }
            return Ok(PredicateAst::ItMatches(filter));
        }
        if descriptor_words
            .first()
            .is_some_and(|word| matches!(*word, "is" | "are"))
        {
            descriptor_words.remove(0);
        }
        if slice_starts_with(&descriptor_words, &["not", "token"]) {
            descriptor_words.drain(0..2);
            descriptor_words.insert(0, "nontoken");
        }
        if !descriptor_words.is_empty() {
            let descriptor_tokens = descriptor_words
                .iter()
                .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
                .collect::<Vec<_>>();
            if let Ok(filter) = parse_object_filter_lexed(&descriptor_tokens, false)
                && filter != ObjectFilter::default()
            {
                if has_card
                    && filter.card_types.len() == 1
                    && filter.card_types[0] == CardType::Land
                    && filter.subtypes.is_empty()
                    && !filter.nontoken
                    && filter.excluded_card_types.is_empty()
                {
                    return Ok(PredicateAst::ItIsLandCard);
                }
                return Ok(PredicateAst::ItMatches(filter));
            }
        }
    }

    if filtered.len() >= 3
        && filtered[0] == "you"
        && (filtered[1] == "control" || filtered[1] == "controls")
        && (filtered[2] == "no" || filtered[2] == "neither")
    {
        let control_tokens = filtered[3..]
            .iter()
            .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
            .collect::<Vec<_>>();
        if let Ok(mut filter) = parse_object_filter(&control_tokens, false) {
            filter.controller = Some(PlayerFilter::You);
            if filtered[2] == "neither" {
                filter = filter
                    .match_tagged(TagKey::from(IT_TAG), TaggedOpbjectRelation::IsTaggedObject);
            }
            return Ok(PredicateAst::PlayerControlsNo {
                player: PlayerAst::You,
                filter,
            });
        }
    }

    let you_dont_control_filter_start = if filtered.len() >= 4
        && filtered[0] == "you"
        && matches!(filtered[1], "dont" | "don't")
        && (filtered[2] == "control" || filtered[2] == "controls")
    {
        Some(3usize)
    } else if filtered.len() >= 5
        && filtered[0] == "you"
        && filtered[1] == "do"
        && filtered[2] == "not"
        && (filtered[3] == "control" || filtered[3] == "controls")
    {
        Some(4usize)
    } else {
        None
    };
    if let Some(filter_start) = you_dont_control_filter_start {
        let control_tokens = filtered[filter_start..]
            .iter()
            .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
            .collect::<Vec<_>>();
        let other = control_tokens
            .first()
            .is_some_and(|token| token.is_word("another") || token.is_word("other"));
        if let Ok(mut filter) = parse_object_filter(&control_tokens, other) {
            filter.controller = Some(PlayerFilter::You);
            return Ok(PredicateAst::PlayerControlsNo {
                player: PlayerAst::You,
                filter,
            });
        }
    }

    if filtered.len() >= 7
        && filtered[0] == "you"
        && (filtered[1] == "control" || filtered[1] == "controls")
        && let Some(or_idx) = find_index(&filtered, |word| *word == "or")
        && or_idx > 2
    {
        let left_tokens = filtered[2..or_idx]
            .iter()
            .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
            .collect::<Vec<_>>();
        let mut right_words = filtered[or_idx + 1..].to_vec();
        if right_words.first().copied() == Some("there") {
            right_words = right_words[1..].to_vec();
        }
        if slice_contains(&right_words, &"graveyard") && slice_contains(&right_words, &"your") {
            let right_tokens = right_words
                .iter()
                .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
                .collect::<Vec<_>>();
            if let (Ok(mut control_filter), Ok(mut graveyard_filter)) = (
                parse_object_filter(&left_tokens, false),
                parse_object_filter(&right_tokens, false),
            ) {
                control_filter.controller = Some(PlayerFilter::You);
                if graveyard_filter.zone.is_none() {
                    graveyard_filter.zone = Some(Zone::Graveyard);
                }
                if graveyard_filter.owner.is_none() {
                    graveyard_filter.owner = Some(PlayerFilter::You);
                }
                return Ok(PredicateAst::PlayerControlsOrHasCardInGraveyard {
                    player: PlayerAst::You,
                    control_filter,
                    graveyard_filter,
                });
            }
        }
    }

    if filtered.len() >= 3
        && filtered[0] == "you"
        && (filtered[1] == "control" || filtered[1] == "controls")
    {
        let mut filter_start = 2usize;
        let mut min_count: Option<u32> = None;
        let mut exact_count: Option<u32> = None;
        if let Some(raw_count) = filtered.get(2)
            && let Some(parsed_count) = parse_named_number(raw_count)
            && filtered.get(3).copied() == Some("or")
            && filtered.get(4).copied() == Some("more")
        {
            min_count = Some(parsed_count);
            filter_start = 5;
        } else if filtered.get(2).copied() == Some("exactly")
            && let Some(raw_count) = filtered.get(3)
            && let Some(parsed_count) = parse_named_number(raw_count)
        {
            exact_count = Some(parsed_count);
            filter_start = 4;
        } else if filtered.get(2).copied() == Some("at")
            && filtered.get(3).copied() == Some("least")
            && let Some(raw_count) = filtered.get(4)
            && let Some(parsed_count) = parse_named_number(raw_count)
        {
            min_count = Some(parsed_count);
            filter_start = 5;
        }

        let mut control_words = filtered[filter_start..].to_vec();
        let mut requires_different_powers = false;
        if slice_ends_with(&control_words, &["with", "different", "powers"])
            || slice_ends_with(&control_words, &["with", "different", "power"])
        {
            requires_different_powers = true;
            control_words.truncate(control_words.len().saturating_sub(3));
        }
        let control_tokens = control_words
            .iter()
            .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
            .collect::<Vec<_>>();
        let other = control_tokens
            .first()
            .is_some_and(|token| token.is_word("another") || token.is_word("other"));
        if let Ok(mut filter) = parse_object_filter(&control_tokens, other) {
            filter.controller = Some(PlayerFilter::You);
            if let Some(count) = exact_count {
                return Ok(PredicateAst::PlayerControlsExactly {
                    player: PlayerAst::You,
                    filter,
                    count,
                });
            }
            if let Some(count) = min_count
                && count > 1
            {
                if requires_different_powers {
                    return Ok(PredicateAst::PlayerControlsAtLeastWithDifferentPowers {
                        player: PlayerAst::You,
                        filter,
                        count,
                    });
                }
                return Ok(PredicateAst::PlayerControlsAtLeast {
                    player: PlayerAst::You,
                    filter,
                    count,
                });
            }
            return Ok(PredicateAst::PlayerControls {
                player: PlayerAst::You,
                filter,
            });
        }
    }

    if filtered.len() >= 4
        && filtered[0] == "that"
        && (filtered[1] == "player" || filtered[1] == "players")
        && (filtered[2] == "control" || filtered[2] == "controls")
    {
        let mut filter_start = 3usize;
        let mut min_count: Option<u32> = None;
        let mut exact_count: Option<u32> = None;
        if let Some(raw_count) = filtered.get(3)
            && let Some(parsed_count) = parse_named_number(raw_count)
            && filtered.get(4).copied() == Some("or")
            && filtered.get(5).copied() == Some("more")
        {
            min_count = Some(parsed_count);
            filter_start = 6;
        } else if filtered.get(3).copied() == Some("exactly")
            && let Some(raw_count) = filtered.get(4)
            && let Some(parsed_count) = parse_named_number(raw_count)
        {
            exact_count = Some(parsed_count);
            filter_start = 5;
        } else if filtered.get(3).copied() == Some("at")
            && filtered.get(4).copied() == Some("least")
            && let Some(raw_count) = filtered.get(5)
            && let Some(parsed_count) = parse_named_number(raw_count)
        {
            min_count = Some(parsed_count);
            filter_start = 6;
        }

        let control_tokens = filtered[filter_start..]
            .iter()
            .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
            .collect::<Vec<_>>();
        let other = control_tokens
            .first()
            .is_some_and(|token| token.is_word("another") || token.is_word("other"));
        if let Ok(filter) = parse_object_filter(&control_tokens, other) {
            if let Some(count) = exact_count {
                return Ok(PredicateAst::PlayerControlsExactly {
                    player: PlayerAst::That,
                    filter,
                    count,
                });
            }
            if let Some(count) = min_count
                && count > 1
            {
                return Ok(PredicateAst::PlayerControlsAtLeast {
                    player: PlayerAst::That,
                    filter,
                    count,
                });
            }
            return Ok(PredicateAst::PlayerControls {
                player: PlayerAst::That,
                filter,
            });
        }
    }

    if filtered.as_slice() == ["you", "controlled", "that", "permanent"]
        || filtered.as_slice() == ["you", "control", "that", "permanent"]
    {
        return Ok(PredicateAst::PlayerTaggedObjectMatches {
            player: PlayerAst::You,
            tag: TagKey::from(IT_TAG),
            filter: ObjectFilter::default(),
        });
    }

    if filtered.as_slice() == ["it", "entered", "under", "your", "control"]
        || filtered.as_slice() == ["that", "card", "entered", "under", "your", "control"]
        || filtered.as_slice() == ["that", "permanent", "entered", "under", "your", "control"]
    {
        return Ok(PredicateAst::PlayerTaggedObjectEnteredBattlefieldThisTurn {
            player: PlayerAst::You,
            tag: TagKey::from(IT_TAG),
        });
    }

    if filtered.len() >= 8
        && filtered[0] == "you"
        && filtered[1] == "put"
        && slice_ends_with(&filtered, &["onto", "the", "battlefield", "this", "way"])
    {
        let filter_words = &filtered[2..filtered.len() - 5];
        let filter_tokens = filter_words
            .iter()
            .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
            .collect::<Vec<_>>();
        let filter = parse_object_filter(&filter_tokens, false)?;
        return Ok(PredicateAst::PlayerTaggedObjectMatches {
            player: PlayerAst::You,
            tag: TagKey::from(IT_TAG),
            filter,
        });
    }

    if filtered.as_slice() == ["it", "wasnt", "blocking"]
        || filtered.as_slice() == ["it", "was", "not", "blocking"]
        || filtered.as_slice() == ["that", "creature", "wasnt", "blocking"]
    {
        return Ok(PredicateAst::TaggedMatches(
            TagKey::from(IT_TAG),
            ObjectFilter {
                nonblocking: true,
                ..Default::default()
            },
        ));
    }

    if filtered.as_slice() == ["no", "creatures", "are", "on", "battlefield"] {
        return Ok(PredicateAst::PlayerControlsNo {
            player: PlayerAst::Any,
            filter: ObjectFilter::creature(),
        });
    }

    if filtered.as_slice() == ["you", "have", "citys", "blessing"]
        || filtered.as_slice() == ["you", "have", "city", "blessing"]
        || slice_starts_with(
            &filtered,
            &["you", "have", "citys", "blessing", "for", "each"],
        )
        || slice_starts_with(
            &filtered,
            &["you", "have", "city", "blessing", "for", "each"],
        )
    {
        return Ok(PredicateAst::PlayerHasCitysBlessing {
            player: PlayerAst::You,
        });
    }

    if filtered.as_slice() == ["youre", "the", "monarch"]
        || filtered.as_slice() == ["youre", "monarch"]
        || filtered.as_slice() == ["you", "are", "the", "monarch"]
        || filtered.as_slice() == ["you", "are", "monarch"]
    {
        return Ok(PredicateAst::PlayerIsMonarch {
            player: PlayerAst::You,
        });
    }

    if filtered.as_slice() == ["you", "have", "the", "initiative"]
        || filtered.as_slice() == ["you", "have", "initiative"]
    {
        return Ok(PredicateAst::PlayerHasInitiative {
            player: PlayerAst::You,
        });
    }

    if filtered.as_slice() == ["youve", "completed", "a", "dungeon"]
        || filtered.as_slice() == ["you", "have", "completed", "a", "dungeon"]
    {
        return Ok(PredicateAst::PlayerCompletedDungeon {
            player: PlayerAst::You,
            dungeon_name: None,
        });
    }

    if (slice_starts_with(&filtered, &["youve", "completed"]) && filtered.len() > 2)
        || (slice_starts_with(&filtered, &["you", "have", "completed"]) && filtered.len() > 3)
    {
        let name_start = if filtered[1] == "have" { 3 } else { 2 };
        let dungeon_name = filtered[name_start..]
            .iter()
            .map(|word| (*word).to_string())
            .collect::<Vec<_>>()
            .join(" ");
        return Ok(PredicateAst::PlayerCompletedDungeon {
            player: PlayerAst::You,
            dungeon_name: Some(dungeon_name),
        });
    }

    if (slice_starts_with(&filtered, &["you", "havent", "completed"]) && filtered.len() > 3)
        || (slice_starts_with(&filtered, &["you", "have", "not", "completed"])
            && filtered.len() > 4)
    {
        let name_start = if filtered[1] == "have" { 4 } else { 3 };
        let dungeon_name = filtered[name_start..]
            .iter()
            .map(|word| (*word).to_string())
            .collect::<Vec<_>>()
            .join(" ");
        return Ok(PredicateAst::Not(Box::new(
            PredicateAst::PlayerCompletedDungeon {
                player: PlayerAst::You,
                dungeon_name: Some(dungeon_name),
            },
        )));
    }

    if filtered.as_slice() == ["youve", "cast", "another", "spell", "this", "turn"]
        || filtered.as_slice() == ["you", "have", "cast", "another", "spell", "this", "turn"]
        || filtered.as_slice() == ["you", "cast", "another", "spell", "this", "turn"]
    {
        return Ok(PredicateAst::PlayerCastSpellsThisTurnOrMore {
            player: PlayerAst::You,
            count: 2,
        });
    }

    let spell_cast_prefix = if slice_starts_with(&filtered, &["opponent", "has", "cast"]) {
        Some((3usize, PlayerFilter::Opponent))
    } else if slice_starts_with(&filtered, &["opponents", "have", "cast"]) {
        Some((3usize, PlayerFilter::Opponent))
    } else if slice_starts_with(&filtered, &["youve", "cast"]) {
        Some((2usize, PlayerFilter::You))
    } else if slice_starts_with(&filtered, &["you", "have", "cast"]) {
        Some((3usize, PlayerFilter::You))
    } else if slice_starts_with(&filtered, &["you", "cast"]) {
        Some((2usize, PlayerFilter::You))
    } else {
        None
    };
    if let Some((prefix_len, player)) = spell_cast_prefix
        && filtered.len() > prefix_len + 2
        && filtered[filtered.len() - 2..] == ["this", "turn"]
    {
        let filter_words = &filtered[prefix_len..filtered.len() - 2];
        let filter_tokens = filter_words
            .iter()
            .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
            .collect::<Vec<_>>();
        if let Ok(filter) = parse_object_filter_lexed(&filter_tokens, false) {
            return Ok(PredicateAst::ValueComparison {
                left: Value::SpellsCastThisTurnMatching {
                    player,
                    filter,
                    exclude_source: false,
                },
                operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                right: Value::Fixed(1),
            });
        }
    }

    if filtered.len() == 5
        && filtered[0] == "x"
        && filtered[1] == "is"
        && filtered[3] == "or"
        && filtered[4] == "more"
    {
        if let Some(amount) = filtered[2]
            .parse::<i32>()
            .ok()
            .or_else(|| parse_named_number(filtered[2]).map(|n| n as i32))
        {
            return Ok(PredicateAst::ValueComparison {
                left: Value::X,
                operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                right: Value::Fixed(amount),
            });
        }
    }

    if is_unmodeled_predicate_words(&filtered) {
        return Ok(PredicateAst::Unmodeled(filtered.join(" ")));
    }

    Err(CardTextError::ParseError(format!(
        "unsupported predicate (predicate: '{}')",
        filtered.join(" ")
    )))
}
