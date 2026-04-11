pub(crate) fn parse_object_filter_with_grammar_entrypoint_lexed(
    tokens: &[OwnedLexToken],
    other: bool,
) -> Result<ObjectFilter, CardTextError> {
    parse_object_filter_lexed(tokens, other)
}

fn parse_object_filter(
    tokens: &[OwnedLexToken],
    other: bool,
) -> Result<ObjectFilter, CardTextError> {
    parse_object_filter_inner(tokens, other, true)
}

fn parse_object_filter_permissive(
    tokens: &[OwnedLexToken],
    other: bool,
) -> Result<ObjectFilter, CardTextError> {
    parse_object_filter_inner(tokens, other, false)
}

fn parse_object_filter_inner(
    tokens: &[OwnedLexToken],
    other: bool,
    strict: bool,
) -> Result<ObjectFilter, CardTextError> {
    let (tokens, vote_winners_only) = trim_vote_winner_suffix(tokens);
    let mut filter = ObjectFilter::default();
    if other {
        filter.other = true;
    }

    let mut target_player: Option<PlayerFilter> = None;
    let mut target_object: Option<ObjectFilter> = None;
    let mut base_tokens: Vec<OwnedLexToken> = tokens.to_vec();
    let mut targets_idx: Option<usize> = None;
    for (idx, token) in tokens.iter().enumerate() {
        if token.is_word("targets") || token.is_word("target") {
            if idx > 0 && tokens[idx - 1].is_word("that") {
                targets_idx = Some(idx);
                break;
            }
        }
    }
    if let Some(targets_idx) = targets_idx {
        let that_idx = targets_idx - 1;
        base_tokens = tokens[..that_idx].to_vec();
        let target_tokens = &tokens[targets_idx + 1..];
        let parse_target_fragment = |fragment_tokens: &[OwnedLexToken]| -> Result<
            (Option<PlayerFilter>, Option<ObjectFilter>),
            CardTextError,
        > {
            let target_words_view = GrammarFilterNormalizedWords::new(fragment_tokens);
            let target_words = target_words_view.to_word_refs();
            if starts_with_any_filter_phrase(&target_words, &[&["you"]]) {
                return Ok((Some(PlayerFilter::You), None));
            }
            if starts_with_any_filter_phrase(&target_words, &[&["opponent"], &["opponents"]]) {
                return Ok((Some(PlayerFilter::Opponent), None));
            }
            if starts_with_any_filter_phrase(&target_words, &[&["player"], &["players"]]) {
                return Ok((Some(PlayerFilter::Any), None));
            }

            let mut target_filter_tokens = fragment_tokens;
            if target_filter_tokens
                .first()
                .is_some_and(|token| token.is_word("target"))
            {
                target_filter_tokens = &target_filter_tokens[1..];
            }
            if target_filter_tokens.is_empty() {
                return Ok((None, None));
            }
            Ok((
                None,
                Some(parse_object_filter_permissive(target_filter_tokens, false)?),
            ))
        };

        let target_words_view = GrammarFilterNormalizedWords::new(target_tokens);
        let target_words = target_words_view.to_word_refs();
        if let Some(or_word_idx) = lower_words_find_index(&target_words, |word| word == "or")
            && let Some(or_token_idx) =
                normalized_token_index_for_word_index(target_tokens, or_word_idx)
        {
            let left_tokens = trim_commas(&target_tokens[..or_token_idx]);
            let right_tokens = trim_commas(&target_tokens[or_token_idx + 1..]);
            let (left_player, left_object) = parse_target_fragment(&left_tokens)?;
            let (right_player, right_object) = parse_target_fragment(&right_tokens)?;
            target_player = left_player.or(right_player);
            target_object = left_object.or(right_object);
            if target_player.is_some() && target_object.is_some() {
                filter.targets_any_of = true;
            }
        } else {
            let (parsed_player, parsed_object) = parse_target_fragment(target_tokens)?;
            target_player = parsed_player;
            target_object = parsed_object;
        }
    }

    // Object filters should not absorb trailing duration clauses such as
    // "... until this enchantment leaves the battlefield".
    if let Some(until_token_idx) = token_find_index(&base_tokens, |token| token.is_word("until"))
        && until_token_idx > 0
    {
        base_tokens.truncate(until_token_idx);
    }

    let not_on_battlefield = strip_not_on_battlefield_phrase(&mut base_tokens);

    // "other than this/it/them ..." marks an exclusion, not an additional
    // type selector. Keep "other" but drop the self-reference tail.
    let mut idx = 0usize;
    while idx + 2 < base_tokens.len() {
        if !(base_tokens[idx].is_word("other") && base_tokens[idx + 1].is_word("than")) {
            idx += 1;
            continue;
        }

        let mut end = idx + 2;
        let starts_with_self_reference = base_tokens[end].is_word("this")
            || base_tokens[end].is_word("it")
            || base_tokens[end].is_word("them");
        if !starts_with_self_reference {
            idx += 1;
            continue;
        }
        end += 1;

        if end < base_tokens.len()
            && base_tokens[end].as_word().is_some_and(|word| {
                matches!(
                    word,
                    "artifact"
                        | "artifacts"
                        | "battle"
                        | "battles"
                        | "card"
                        | "cards"
                        | "creature"
                        | "creatures"
                        | "enchantment"
                        | "enchantments"
                        | "land"
                        | "lands"
                        | "permanent"
                        | "permanents"
                        | "planeswalker"
                        | "planeswalkers"
                        | "spell"
                        | "spells"
                        | "token"
                        | "tokens"
                )
            })
        {
            end += 1;
        }

        base_tokens.drain(idx + 1..end);
    }
    if let Some(mut disjunction) = parse_attached_reference_or_another_disjunction(&base_tokens)? {
        if target_player.is_some() || target_object.is_some() {
            disjunction = disjunction.targeting(target_player.take(), target_object.take());
        }
        return Ok(disjunction);
    }
    let mut segment_tokens = base_tokens.clone();

    let all_words_view = GrammarFilterNormalizedWords::new(&base_tokens);
    let all_words_with_articles: Vec<&str> = all_words_view
        .to_word_refs()
        .into_iter()
        .filter(|word| *word != "instead")
        .collect();

    let map_non_article_index = |non_article_idx: usize| -> Option<usize> {
        let mut seen = 0usize;
        for (idx, word) in all_words_with_articles.iter().enumerate() {
            if is_article(word) {
                continue;
            }
            if seen == non_article_idx {
                return Some(idx);
            }
            seen += 1;
        }
        None
    };

    let map_non_article_end = |non_article_end: usize| -> Option<usize> {
        let mut seen = 0usize;
        for (idx, word) in all_words_with_articles.iter().enumerate() {
            if is_article(word) {
                continue;
            }
            if seen == non_article_end {
                return Some(idx);
            }
            seen += 1;
        }
        if seen == non_article_end {
            return Some(all_words_with_articles.len());
        }
        None
    };

    let mut all_words: Vec<&str> = all_words_with_articles
        .iter()
        .copied()
        .filter(|word| !is_article(word))
        .collect();

    // "that were put there from the battlefield this turn" means the card entered
    // a graveyard from the battlefield this turn.
    try_apply_put_there_from_battlefield_this_turn_clause(
        &mut filter,
        &mut all_words,
        &mut segment_tokens,
    );

    // "legendary or Rat card" (Nashi, Moon's Legacy) is a supertype/subtype disjunction.
    // We parse it by collecting both selectors and then expanding into an `any_of` filter
    // after the normal pass so other shared qualifiers (zone/owner/etc.) are preserved.
    let legendary_or_subtype = find_word_slice_phrase_start(&all_words, &["legendary", "or"])
        .and_then(|idx| all_words.get(idx + 2).copied())
        .and_then(parse_subtype_word);

    // "in a graveyard that was put there from anywhere this turn" (Reenact the Crime)
    // means the card entered a graveyard this turn.
    try_apply_put_there_from_anywhere_this_turn_clause(
        &mut filter,
        &mut all_words,
        &mut segment_tokens,
    );

    // "... graveyard from the battlefield this turn" means the card entered a graveyard
    // from the battlefield this turn.
    try_apply_graveyard_from_battlefield_this_turn_clause(
        &mut filter,
        &mut all_words,
        &mut segment_tokens,
    );

    // "... entered the battlefield ... this turn" marks a battlefield entry this turn.
    try_apply_entered_battlefield_this_turn_clause(
        &mut filter,
        &mut all_words,
        &mut segment_tokens,
    );

    // Avoid treating reference phrases like "... with mana value equal to the number of charge
    // counters on this artifact" as additional type selectors on the filtered object.
    // (Aether Vial: "put a creature card with mana value equal to the number of charge counters
    // on this artifact from your hand onto the battlefield.")
    let _ = try_apply_mana_value_eq_counters_on_source_clause(
        &mut filter,
        &mut all_words,
        &mut segment_tokens,
    );

    try_apply_attached_exclusion_phrases(&mut filter, &mut all_words);

    let _ = try_apply_pt_literal_prefix(&mut filter, &mut all_words);

    strip_object_filter_leading_prefixes(&mut all_words);

    let _ = try_apply_not_all_colors_clause(&mut filter, &mut all_words);

    let _ = try_apply_not_exactly_two_colors_clause(&mut filter, &mut all_words);

    let _ = try_apply_leading_tagged_reference_prefix(&mut filter, &mut all_words);

    let _ = try_apply_entered_since_your_last_turn_ended_clause(&mut filter, &mut all_words);

    strip_object_filter_face_state_words(&mut filter, &mut all_words);

    if contains_any_filter_phrase(&all_words, &[&["entered", "this", "turn"]]) {
        return Err(CardTextError::ParseError(format!(
            "unsupported entered-this-turn object filter (clause: '{}')",
            all_words.join(" ")
        )));
    }
    if contains_any_filter_phrase(
        &all_words,
        &[
            &["counter", "on", "it", "or"],
            &["counter", "on", "them", "or"],
        ],
    ) {
        return Err(CardTextError::ParseError(format!(
            "unsupported counter-state object filter (clause: '{}')",
            all_words.join(" ")
        )));
    }
    strip_single_graveyard_phrase(&mut filter, &mut all_words);

    let _ = try_apply_not_named_clause(
        &mut filter,
        &mut all_words,
        &all_words_with_articles,
        &map_non_article_index,
        &map_non_article_end,
    )?;

    let _ = try_apply_named_clause(
        &mut filter,
        &mut all_words,
        &all_words_with_articles,
        &map_non_article_index,
        &map_non_article_end,
    )?;

    let _ = try_apply_color_count_phrase(&mut filter, &mut all_words)?;
    let has_power_or_toughness_clause = contains_any_filter_phrase(
        &all_words,
        &[&["power", "or", "toughness"], &["toughness", "or", "power"]],
    );
    if has_power_or_toughness_clause
        && !all_words
            .iter()
            .any(|word| matches!(*word, "spell" | "spells"))
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported power-or-toughness object filter (clause: '{}')",
            all_words.join(" ")
        )));
    }
    let reference_stage =
        apply_reference_and_tag_stage(&mut filter, &mut all_words, &mut segment_tokens);
    if reference_stage.early_return {
        return Ok(filter);
    }
    let source_linked_exile_reference = reference_stage.source_linked_exile_reference;

    let references_target_player =
        contains_any_filter_phrase(&all_words, &[&["target", "player"], &["target", "players"]]);
    let references_target_opponent = contains_any_filter_phrase(
        &all_words,
        &[&["target", "opponent"], &["target", "opponents"]],
    );
    let pronoun_player_filter = if references_target_opponent {
        PlayerFilter::target_opponent()
    } else if references_target_player {
        PlayerFilter::target_player()
    } else {
        PlayerFilter::IteratedPlayer
    };

    if let Some(attacking_filter) =
        attacking_player_filter_from_words(&all_words, &pronoun_player_filter)
    {
        filter.attacking_player_or_planeswalker_controlled_by = Some(attacking_filter);
    }

    let is_tagged_spell_reference_at = |idx: usize| {
        all_words
            .get(idx.wrapping_sub(1))
            .is_some_and(|prev| matches!(*prev, "that" | "this" | "its" | "their"))
    };
    let contains_unqualified_spell_word = all_words.iter().enumerate().any(|(idx, word)| {
        matches!(*word, "spell" | "spells") && !is_tagged_spell_reference_at(idx)
    });
    let mentions_ability_word = all_words
        .iter()
        .any(|word| matches!(*word, "ability" | "abilities"));
    if contains_unqualified_spell_word && !mentions_ability_word {
        filter.has_mana_cost = true;
    }

    if !all_words.is_empty() {
        let mut idx = 0usize;
        while idx < all_words.len() {
            let slice = &all_words[idx..];
            if let Some(consumed) =
                try_apply_joint_owner_controller_clause(&mut filter, slice, &pronoun_player_filter)
            {
                idx += consumed.max(1);
                continue;
            }
            if let Some(consumed) = try_apply_chosen_player_graveyard_clause(&mut filter, slice) {
                idx += consumed.max(1);
                continue;
            }
            if let Some(consumed) = try_apply_negated_you_relation_clause(&mut filter, slice) {
                idx += consumed.max(1);
                continue;
            }
            if let Some(consumed) =
                try_apply_player_relation_clause(&mut filter, slice, &pronoun_player_filter)
            {
                idx += consumed.max(1);
                continue;
            }
            idx += 1;
        }
    }

    let mut with_idx = 0usize;
    while with_idx + 1 < all_words.len() {
        if all_words[with_idx] != "with" {
            with_idx += 1;
            continue;
        }

        if let Some(consumed) = try_apply_with_clause_tail(&mut filter, &all_words[with_idx + 1..])
        {
            with_idx += 1 + consumed;
            continue;
        }

        with_idx += 1;
    }

    let mut has_idx = 0usize;
    while has_idx + 1 < all_words.len() {
        if !matches!(all_words[has_idx], "has" | "have") {
            has_idx += 1;
            continue;
        }
        if filter.with_counter.is_none()
            && let Some((counter_constraint, consumed)) =
                parse_filter_counter_constraint_words(&all_words[has_idx + 1..])
        {
            filter.with_counter = Some(counter_constraint);
            has_idx += 1 + consumed;
            continue;
        }
        has_idx += 1;
    }

    let mut without_idx = 0usize;
    while without_idx + 1 < all_words.len() {
        if all_words[without_idx] != "without" {
            without_idx += 1;
            continue;
        }

        if let Some(consumed) =
            try_apply_without_clause_tail(&mut filter, &all_words[without_idx + 1..])
        {
            without_idx += 1 + consumed;
            continue;
        }

        without_idx += 1;
    }

    let has_tap_activated_ability = contains_any_filter_phrase(
        &all_words,
        &[
            &[
                "has",
                "an",
                "activated",
                "ability",
                "with",
                "t",
                "in",
                "its",
                "cost",
            ],
            &[
                "has",
                "activated",
                "ability",
                "with",
                "t",
                "in",
                "its",
                "cost",
            ],
            &[
                "activated",
                "abilities",
                "with",
                "t",
                "in",
                "their",
                "costs",
            ],
        ],
    );
    if has_tap_activated_ability {
        filter.has_tap_activated_ability = true;
    }

    for idx in 0..all_words.len() {
        if let Some(zone) = parse_zone_word(all_words[idx]) {
            let is_reference_zone_for_spell = if contains_unqualified_spell_word {
                idx > 0
                    && matches!(
                        all_words[idx - 1],
                        "controller"
                            | "controllers"
                            | "owner"
                            | "owners"
                            | "its"
                            | "their"
                            | "that"
                            | "this"
                    )
            } else {
                false
            };
            if is_reference_zone_for_spell {
                continue;
            }
            if filter.zone.is_none() {
                filter.zone = Some(zone);
            }
            if idx > 0 {
                match all_words[idx - 1] {
                    "your" => {
                        filter.owner = Some(PlayerFilter::You);
                    }
                    "opponent" | "opponents" => {
                        filter.owner = Some(PlayerFilter::Opponent);
                    }
                    "their" => {
                        filter.owner = Some(pronoun_player_filter.clone());
                    }
                    _ => {}
                }
            }
            if idx > 1 {
                let owner_pair = (all_words[idx - 2], all_words[idx - 1]);
                match owner_pair {
                    ("target", "player") | ("target", "players") => {
                        filter.owner = Some(PlayerFilter::target_player());
                    }
                    ("target", "opponent") | ("target", "opponents") => {
                        filter.owner = Some(PlayerFilter::target_opponent());
                    }
                    ("that", "player") | ("that", "players") => {
                        filter.owner = Some(PlayerFilter::IteratedPlayer);
                    }
                    _ => {}
                }
            }
        }
    }

    let clause_words = all_words.clone();
    for idx in 0..all_words.len() {
        let (is_base_reference, pt_word_idx) = if idx + 4 < all_words.len()
            && all_words[idx] == "base"
            && all_words[idx + 1] == "power"
            && all_words[idx + 2] == "and"
            && all_words[idx + 3] == "toughness"
        {
            (true, idx + 4)
        } else if idx + 3 < all_words.len()
            && all_words[idx] == "power"
            && all_words[idx + 1] == "and"
            && all_words[idx + 2] == "toughness"
            && (idx == 0 || all_words[idx - 1] != "base")
        {
            (false, idx + 3)
        } else {
            continue;
        };

        if let Ok((power, toughness)) = parse_pt_modifier(all_words[pt_word_idx]) {
            filter.power = Some(crate::filter::Comparison::Equal(power));
            filter.toughness = Some(crate::filter::Comparison::Equal(toughness));
            filter.power_reference = if is_base_reference {
                crate::filter::PtReference::Base
            } else {
                crate::filter::PtReference::Effective
            };
            filter.toughness_reference = if is_base_reference {
                crate::filter::PtReference::Base
            } else {
                crate::filter::PtReference::Effective
            };
        }
    }

    let mut idx = 0usize;
    while idx < all_words.len() {
        let axis = match all_words[idx] {
            "power" => Some("power"),
            "toughness" => Some("toughness"),
            "mana" if idx + 1 < all_words.len() && all_words[idx + 1] == "value" => {
                Some("mana value")
            }
            _ => None,
        };
        let Some(axis) = axis else {
            idx += 1;
            continue;
        };
        let is_base_reference = idx > 0 && all_words[idx - 1] == "base";

        let axis_word_count = usize::from(axis == "mana value") + 1;
        let value_tokens = if idx + axis_word_count < all_words.len() {
            &all_words[idx + axis_word_count..]
        } else {
            &[]
        };
        let Some((cmp, consumed)) =
            parse_filter_comparison_tokens(axis, value_tokens, &clause_words)?
        else {
            idx += 1;
            continue;
        };

        match axis {
            "power" => {
                filter.power = Some(cmp);
                filter.power_reference = if is_base_reference {
                    crate::filter::PtReference::Base
                } else {
                    crate::filter::PtReference::Effective
                };
            }
            "toughness" => {
                filter.toughness = Some(cmp);
                filter.toughness_reference = if is_base_reference {
                    crate::filter::PtReference::Base
                } else {
                    crate::filter::PtReference::Effective
                };
            }
            "mana value" => filter.mana_value = Some(cmp),
            _ => {}
        }
        idx += axis_word_count + consumed;
    }

    apply_parity_filter_phrases(&clause_words, &mut filter);

    if contains_any_filter_phrase(
        &clause_words,
        &[&["power", "greater", "than", "its", "base", "power"]],
    ) {
        filter.power_greater_than_base_power = true;
    }

    let mut saw_permanent = false;
    let mut saw_spell = false;
    let mut saw_permanent_type = false;

    let mut saw_subtype = false;
    let mut negated_word_indices = std::collections::HashSet::new();
    let mut negated_historic_indices = std::collections::HashSet::new();
    let is_text_negation_word =
        |word: &str| matches!(word, "not" | "isnt" | "isn't" | "arent" | "aren't");
    for idx in 0..all_words.len().saturating_sub(1) {
        if all_words[idx] != "non" {
            continue;
        }
        let next = all_words[idx + 1];
        if is_outlaw_word(next) {
            push_outlaw_subtypes(&mut filter.excluded_subtypes);
            negated_word_indices.insert(idx + 1);
        }
        if let Some(card_type) = parse_card_type(next)
            && !slice_has(&filter.excluded_card_types, &card_type)
        {
            filter.excluded_card_types.push(card_type);
            negated_word_indices.insert(idx + 1);
        }
        if next == "attacking" {
            filter.nonattacking = true;
            negated_word_indices.insert(idx + 1);
        }
        if next == "blocking" {
            filter.nonblocking = true;
            negated_word_indices.insert(idx + 1);
        }
        if next == "blocked" {
            filter.unblocked = true;
            negated_word_indices.insert(idx + 1);
        }
        if next == "commander" || next == "commanders" {
            filter.noncommander = true;
            negated_word_indices.insert(idx + 1);
        }
        if let Some(color) = parse_color(next) {
            filter.excluded_colors = filter.excluded_colors.union(color);
            negated_word_indices.insert(idx + 1);
        }
        if let Some(subtype) = parse_subtype_flexible(next)
            && !slice_has(&filter.excluded_subtypes, &subtype)
        {
            filter.excluded_subtypes.push(subtype);
            negated_word_indices.insert(idx + 1);
        }
    }
    for idx in 0..all_words.len() {
        if !is_text_negation_word(all_words[idx]) {
            continue;
        }
        let mut target_idx = idx + 1;
        if target_idx >= all_words.len() {
            continue;
        }
        if is_article(all_words[target_idx]) {
            target_idx += 1;
            if target_idx >= all_words.len() {
                continue;
            }
        }

        let negated_word = all_words[target_idx];
        if negated_word == "attacking" {
            filter.nonattacking = true;
            negated_word_indices.insert(target_idx);
        }
        if negated_word == "blocking" {
            filter.nonblocking = true;
            negated_word_indices.insert(target_idx);
        }
        if negated_word == "blocked" {
            filter.unblocked = true;
            negated_word_indices.insert(target_idx);
        }
        if negated_word == "historic" {
            filter.nonhistoric = true;
            negated_historic_indices.insert(target_idx);
        }
        if negated_word == "commander" || negated_word == "commanders" {
            filter.noncommander = true;
            negated_word_indices.insert(target_idx);
        }
        if let Some(card_type) = parse_card_type(negated_word)
            && !slice_has(&filter.excluded_card_types, &card_type)
        {
            filter.excluded_card_types.push(card_type);
            negated_word_indices.insert(target_idx);
        }
        if let Some(supertype) = parse_supertype_word(negated_word)
            && !slice_has(&filter.excluded_supertypes, &supertype)
        {
            filter.excluded_supertypes.push(supertype);
            negated_word_indices.insert(target_idx);
        }
        if let Some(color) = parse_color(negated_word) {
            filter.excluded_colors = filter.excluded_colors.union(color);
            negated_word_indices.insert(target_idx);
        }
        if let Some(subtype) = parse_subtype_flexible(negated_word)
            && !slice_has(&filter.excluded_subtypes, &subtype)
        {
            filter.excluded_subtypes.push(subtype);
            negated_word_indices.insert(target_idx);
        }
    }
    for idx in 0..all_words.len().saturating_sub(1) {
        if all_words[idx] == "not" && all_words[idx + 1] == "historic" {
            filter.nonhistoric = true;
            negated_historic_indices.insert(idx + 1);
        }
    }

    for (idx, word) in all_words.iter().enumerate() {
        let is_negated_word = set_has(&negated_word_indices, &idx);
        match *word {
            "permanent" | "permanents" => saw_permanent = true,
            "spell" | "spells" => {
                if !is_tagged_spell_reference_at(idx) {
                    saw_spell = true;
                }
            }
            "token" | "tokens" => filter.token = true,
            "nontoken" => filter.nontoken = true,
            "other" => filter.other = true,
            "tapped" => filter.tapped = true,
            "untapped" => filter.untapped = true,
            "attacking" if !is_negated_word => filter.attacking = true,
            "nonattacking" => filter.nonattacking = true,
            "blocking" if !is_negated_word => filter.blocking = true,
            "nonblocking" => filter.nonblocking = true,
            "blocked" if !is_negated_word => filter.blocked = true,
            "unblocked" if !is_negated_word => filter.unblocked = true,
            "commander" | "commanders" => {
                let prev = idx.checked_sub(1).and_then(|i| all_words.get(i)).copied();
                let prev2 = idx.checked_sub(2).and_then(|i| all_words.get(i)).copied();
                let negated_by_phrase = prev.is_some_and(is_text_negation_word)
                    || (prev.is_some_and(is_article) && prev2.is_some_and(is_text_negation_word));
                if is_negated_word || negated_by_phrase {
                    filter.noncommander = true;
                } else {
                    filter.is_commander = true;
                    match prev {
                        Some("your") => filter.owner = Some(PlayerFilter::You),
                        Some("opponent") | Some("opponents") => {
                            filter.owner = Some(PlayerFilter::Opponent);
                        }
                        Some("their") => filter.owner = Some(pronoun_player_filter.clone()),
                        _ => {}
                    }
                }
            }
            "noncommander" | "noncommanders" => filter.noncommander = true,
            "nonbasic" => {
                filter = filter.without_supertype(Supertype::Basic);
            }
            "colorless" => filter.colorless = true,
            "multicolored" => filter.multicolored = true,
            "monocolored" => filter.monocolored = true,
            "nonhistoric" => filter.nonhistoric = true,
            "historic" if !set_has(&negated_historic_indices, &idx) => filter.historic = true,
            "modified" if !is_negated_word => filter.modified = true,
            _ => {}
        }

        if is_non_outlaw_word(word) {
            push_outlaw_subtypes(&mut filter.excluded_subtypes);
            continue;
        }

        if set_has(&negated_word_indices, &idx) {
            continue;
        }

        if is_outlaw_word(word) {
            push_outlaw_subtypes(&mut filter.subtypes);
            saw_subtype = true;
            continue;
        }

        if let Some(card_type) = parse_non_type(word) {
            filter.excluded_card_types.push(card_type);
        }

        if let Some(supertype) = parse_non_supertype(word)
            && !slice_has(&filter.excluded_supertypes, &supertype)
        {
            filter.excluded_supertypes.push(supertype);
        }

        if let Some(color) = parse_non_color(word) {
            filter.excluded_colors = filter.excluded_colors.union(color);
        }
        if let Some(subtype) = parse_non_subtype(word)
            && !slice_has(&filter.excluded_subtypes, &subtype)
        {
            filter.excluded_subtypes.push(subtype);
        }

        if let Some(color) = parse_color(word) {
            let existing = filter.colors.unwrap_or(ColorSet::new());
            filter.colors = Some(existing.union(color));
        }

        if let Some(supertype) = parse_supertype_word(word)
            && !slice_has(&filter.supertypes, &supertype)
        {
            filter.supertypes.push(supertype);
        }

        if let Some(card_type) = parse_card_type(word) {
            push_unique(&mut filter.card_types, card_type);
            if is_permanent_type(card_type) {
                saw_permanent_type = true;
            }
        }

        if let Some(subtype) = parse_subtype_flexible(word) {
            push_unique(&mut filter.subtypes, subtype);
            saw_subtype = true;
        }
    }
    if saw_spell && source_linked_exile_reference {
        // "spell ... exiled with this" describes a stack spell with a relation
        // to source-linked exiled cards, not a spell object in exile.
        filter.zone = Some(Zone::Stack);
    }

    let segments = split_lexed_slices_on_or(&segment_tokens);
    let mut segment_types = Vec::new();
    let mut segment_subtypes = Vec::new();
    let mut segment_marker_counts = Vec::new();
    let mut segment_words_lists: Vec<Vec<String>> = Vec::new();

    for segment in &segments {
        let segment_words_view = GrammarFilterNormalizedWords::new(segment);
        let segment_words: Vec<String> = segment_words_view
            .to_word_refs()
            .into_iter()
            .filter(|word| !is_article(word))
            .map(ToString::to_string)
            .collect();
        segment_words_lists.push(segment_words.clone());
        let mut types = Vec::new();
        let mut subtypes = Vec::new();
        for word in &segment_words {
            if let Some(card_type) = parse_card_type(word) {
                push_unique(&mut types, card_type);
            }
            if let Some(subtype) = parse_subtype_flexible(word) {
                push_unique(&mut subtypes, subtype);
            }
        }
        segment_marker_counts.push(types.len() + subtypes.len());
        if !types.is_empty() {
            segment_types.push(types);
        }
        if !subtypes.is_empty() {
            segment_subtypes.push(subtypes);
        }
    }

    if segments.len() > 1 {
        let qualifier_in_all_segments = |qualifier: &str| {
            segment_words_lists
                .iter()
                .all(|segment| segment.iter().any(|word| word == qualifier))
        };
        let shared_leading_qualifier = |qualifier: &str, opposite: &str| {
            if qualifier_in_all_segments(qualifier) {
                return true;
            }
            if all_words.iter().any(|word| *word == opposite) {
                return false;
            }
            let Some(first_segment) = segment_words_lists.first() else {
                return false;
            };
            if !first_segment.iter().any(|word| word == qualifier) {
                return false;
            }
            segment_words_lists
                .iter()
                .skip(1)
                .all(|segment| !segment.iter().any(|word| word == opposite))
        };

        if filter.tapped && !shared_leading_qualifier("tapped", "untapped") {
            filter.tapped = false;
        }
        if filter.untapped && !shared_leading_qualifier("untapped", "tapped") {
            filter.untapped = false;
        }
    }

    if segments.len() > 1 {
        let type_list_candidate = !segment_marker_counts.is_empty()
            && segment_marker_counts.iter().all(|count| *count == 1);

        if type_list_candidate {
            let mut any_types = Vec::new();
            let mut any_subtypes = Vec::new();
            for types in segment_types {
                let Some(card_type) = types.first().copied() else {
                    continue;
                };
                push_unique(&mut any_types, card_type);
            }
            for subtypes in segment_subtypes {
                let Some(subtype) = subtypes.first().copied() else {
                    continue;
                };
                push_unique(&mut any_subtypes, subtype);
            }
            if !any_types.is_empty() {
                filter.card_types = any_types;
            }
            if !any_subtypes.is_empty() {
                filter.subtypes = any_subtypes;
            }
            if !filter.card_types.is_empty() && !filter.subtypes.is_empty() {
                filter.type_or_subtype_union = true;
            }
        }
    } else if let Some(types) = segment_types.into_iter().next() {
        let has_and = contains_filter_word(&all_words, "and");
        let has_or = contains_filter_word(&all_words, "or");
        if types.len() > 1 {
            if has_and && !has_or {
                filter.card_types = types;
            } else {
                filter.all_card_types = types;
            }
        } else if types.len() == 1 {
            filter.card_types = types;
        }
    }

    let permanent_type_defaults = vec![
        CardType::Artifact,
        CardType::Creature,
        CardType::Enchantment,
        CardType::Land,
        CardType::Planeswalker,
        CardType::Battle,
    ];
    let and_segments = split_lexed_slices_on_and(&segment_tokens);
    let and_segment_words_lists: Vec<Vec<String>> = and_segments
        .iter()
        .map(|segment| {
            GrammarFilterNormalizedWords::new(segment)
                .to_word_refs()
                .into_iter()
                .filter(|word| !is_article(word))
                .map(ToString::to_string)
                .collect()
        })
        .collect();

    let segment_has_standalone_spell = |segment: &[String]| {
        let contains_spell = segment
            .iter()
            .any(|word| matches!(word.as_str(), "spell" | "spells"));
        if !contains_spell {
            return false;
        }

        !segment.iter().any(|word| {
            matches!(
                word.as_str(),
                "permanent" | "permanents" | "card" | "cards" | "source" | "sources"
            ) || parse_card_type(word).is_some()
                || parse_subtype_flexible(word).is_some()
        })
    };
    let segment_has_nonspell_permanent_head = |segment: &[String]| {
        let contains_spell = segment
            .iter()
            .any(|word| matches!(word.as_str(), "spell" | "spells"));
        if contains_spell {
            return false;
        }

        segment.iter().any(|word| {
            matches!(word.as_str(), "permanent" | "permanents")
                || parse_card_type(word).is_some_and(is_permanent_type)
                || parse_subtype_flexible(word).is_some()
        })
    };
    let segment_has_permanent_spell_head = |segment: &[String]| {
        if segment.len() < 2 {
            return false;
        }
        let mut idx = 0usize;
        while idx + 1 < segment.len() {
            let permanent = &segment[idx];
            let spell = &segment[idx + 1];
            if (permanent == "permanent" || permanent == "permanents")
                && (spell == "spell" || spell == "spells")
            {
                return true;
            }
            idx += 1;
        }
        false
    };
    let has_standalone_spell_segment = segment_words_lists
        .iter()
        .any(|segment| segment_has_standalone_spell(segment));
    let has_nonspell_permanent_segment = segment_words_lists
        .iter()
        .any(|segment| segment_has_nonspell_permanent_head(segment));
    let has_split_permanent_spell_segments = and_segment_words_lists.len() > 1
        && and_segment_words_lists
            .iter()
            .any(|segment| segment_has_permanent_spell_head(segment))
        && and_segment_words_lists
            .iter()
            .any(|segment| segment_has_nonspell_permanent_head(segment));

    if saw_spell && has_standalone_spell_segment && has_nonspell_permanent_segment {
        let mut spell_filter = filter.clone();
        spell_filter.any_of.clear();
        spell_filter.zone = Some(Zone::Stack);
        spell_filter.card_types.clear();
        spell_filter.all_card_types.clear();
        spell_filter.subtypes.clear();
        spell_filter.type_or_subtype_union = false;

        let mut permanent_filter = filter.clone();
        permanent_filter.any_of.clear();
        permanent_filter.zone = Some(Zone::Battlefield);
        permanent_filter.has_mana_cost = false;
        if permanent_filter.card_types.is_empty()
            && permanent_filter.all_card_types.is_empty()
            && permanent_filter.subtypes.is_empty()
        {
            permanent_filter.card_types = permanent_type_defaults.clone();
        }

        let mut combined_filter = ObjectFilter::default();
        combined_filter.any_of = vec![spell_filter, permanent_filter];
        filter = combined_filter;
    } else if saw_spell && saw_permanent && has_split_permanent_spell_segments {
        let mut spell_filter = filter.clone();
        spell_filter.any_of.clear();
        spell_filter.zone = Some(Zone::Stack);
        spell_filter.has_mana_cost = false;
        if spell_filter.card_types.is_empty()
            && spell_filter.all_card_types.is_empty()
            && spell_filter.subtypes.is_empty()
        {
            spell_filter.card_types = permanent_type_defaults.clone();
        }

        let mut permanent_filter = filter.clone();
        permanent_filter.any_of.clear();
        permanent_filter.zone = Some(Zone::Battlefield);
        permanent_filter.has_mana_cost = false;
        if permanent_filter.card_types.is_empty()
            && permanent_filter.all_card_types.is_empty()
            && permanent_filter.subtypes.is_empty()
        {
            permanent_filter.card_types = permanent_type_defaults.clone();
        }

        let mut combined_filter = ObjectFilter::default();
        combined_filter.any_of = vec![spell_filter, permanent_filter];
        filter = combined_filter;
    } else if saw_spell && saw_permanent {
        if filter.card_types.is_empty() && filter.all_card_types.is_empty() {
            filter.card_types = permanent_type_defaults.clone();
        }
        filter.zone = Some(Zone::Stack);
    } else {
        if saw_permanent && filter.card_types.is_empty() && filter.all_card_types.is_empty() {
            filter.card_types = permanent_type_defaults.clone();
        }
    }

    if filter.any_of.is_empty() {
        if let Some(zone) = filter.zone {
            if saw_spell && zone != Zone::Stack {
                let is_spell_origin_zone = matches!(
                    zone,
                    Zone::Hand | Zone::Graveyard | Zone::Exile | Zone::Library | Zone::Command
                );
                if !is_spell_origin_zone {
                    return Err(CardTextError::ParseError(
                        "spell targets must be on the stack".to_string(),
                    ));
                }
            }
        } else if saw_spell {
            filter.zone = Some(Zone::Stack);
        } else if saw_permanent || saw_permanent_type || saw_subtype {
            filter.zone = Some(Zone::Battlefield);
        }
    }

    if contains_unqualified_spell_word
        && filter.cast_by.is_some()
        && matches!(
            filter.zone,
            Some(Zone::Hand | Zone::Graveyard | Zone::Exile | Zone::Library | Zone::Command)
        )
    {
        filter.owner = None;
    }

    if target_player.is_some() || target_object.is_some() {
        filter = filter.targeting(target_player.take(), target_object.take());
    }

    if let Some(or_subtype) = legendary_or_subtype
        && filter.any_of.is_empty()
        && slice_has(&filter.supertypes, &Supertype::Legendary)
        && slice_has(&filter.subtypes, &or_subtype)
    {
        let mut legendary_branch = filter.clone();
        legendary_branch.any_of.clear();
        legendary_branch
            .subtypes
            .retain(|subtype| *subtype != or_subtype);

        let mut subtype_branch = filter.clone();
        subtype_branch.any_of.clear();
        subtype_branch
            .supertypes
            .retain(|supertype| *supertype != Supertype::Legendary);

        let mut disjunction = ObjectFilter::default();
        disjunction.any_of = vec![legendary_branch, subtype_branch];
        filter = disjunction;
    }

    let owner_or_controller_player = all_words.iter().enumerate().find_map(|(idx, _)| {
        parse_owner_or_controller_disjunction_player(&all_words[idx..], &pronoun_player_filter)
            .map(|(player_filter, _)| player_filter)
    });
    if let Some(player_filter) = owner_or_controller_player
        && filter.any_of.is_empty()
    {
        let mut base = filter.clone();
        base.any_of.clear();
        base.owner = None;
        base.controller = None;

        let mut owner_branch = base.clone();
        owner_branch.owner = Some(player_filter.clone());

        let mut controller_branch = base;
        controller_branch.controller = Some(player_filter);

        let mut disjunction = ObjectFilter::default();
        disjunction.any_of = vec![owner_branch, controller_branch];
        filter = disjunction;
    }

    if has_power_or_toughness_clause && saw_spell {
        let mut power_or_toughness_cmp = None;
        for idx in 0..all_words.len() {
            let (_, value_tokens) = match all_words.get(idx..) {
                Some(["power", "or", "toughness", rest @ ..])
                | Some(["toughness", "or", "power", rest @ ..]) => {
                    (crate::filter::PtReference::Effective, rest)
                }
                _ => continue,
            };
            let Some((cmp, _)) =
                parse_filter_comparison_tokens("power", value_tokens, &clause_words)?
            else {
                continue;
            };
            power_or_toughness_cmp = Some(cmp);
            break;
        }
        if let Some(cmp) = power_or_toughness_cmp {
            let mut base = filter.clone();
            base.any_of.clear();
            base.power = None;
            base.toughness = None;

            let mut power_branch = base.clone();
            power_branch.power = Some(cmp.clone());

            let mut toughness_branch = base;
            toughness_branch.toughness = Some(cmp);

            let mut disjunction = ObjectFilter::default();
            disjunction.any_of = vec![power_branch, toughness_branch];
            filter = disjunction;
        }
    }

    let has_constraints = !filter.card_types.is_empty()
        || !filter.all_card_types.is_empty()
        || !filter.supertypes.is_empty()
        || !filter.excluded_supertypes.is_empty()
        || !filter.excluded_card_types.is_empty()
        || !filter.excluded_subtypes.is_empty()
        || !filter.subtypes.is_empty()
        || filter.zone.is_some()
        || filter.controller.is_some()
        || filter.owner.is_some()
        || filter.other
        || filter.token
        || filter.nontoken
        || filter.face_down.is_some()
        || filter.tapped
        || filter.untapped
        || filter.attacking
        || filter
            .attacking_player_or_planeswalker_controlled_by
            .is_some()
        || filter.nonattacking
        || filter.blocking
        || filter.nonblocking
        || filter.blocked
        || filter.unblocked
        || filter.is_commander
        || filter.noncommander
        || !filter.excluded_colors.is_empty()
        || filter.colorless
        || filter.multicolored
        || filter.monocolored
        || filter.all_colors.is_some()
        || filter.exactly_two_colors.is_some()
        || filter.historic
        || filter.nonhistoric
        || filter.power.is_some()
        || filter.power_parity.is_some()
        || filter.toughness.is_some()
        || filter.mana_value.is_some()
        || filter.mana_value_parity.is_some()
        || filter.name.is_some()
        || filter.excluded_name.is_some()
        || filter.source
        || filter.with_counter.is_some()
        || filter.without_counter.is_some()
        || filter.total_counters_parity.is_some()
        || filter.alternative_cast.is_some()
        || !filter.static_abilities.is_empty()
        || !filter.excluded_static_abilities.is_empty()
        || !filter.ability_markers.is_empty()
        || !filter.excluded_ability_markers.is_empty()
        || !filter.tagged_constraints.is_empty()
        || filter.targets_player.is_some()
        || filter.targets_object.is_some()
        || !filter.any_of.is_empty();

    if !has_constraints {
        return Err(CardTextError::ParseError(format!(
            "unsupported target phrase (clause: '{}')",
            all_words.join(" ")
        )));
    }

    let has_object_identity = !filter.card_types.is_empty()
        || !filter.all_card_types.is_empty()
        || !filter.supertypes.is_empty()
        || !filter.excluded_supertypes.is_empty()
        || !filter.excluded_card_types.is_empty()
        || !filter.excluded_subtypes.is_empty()
        || !filter.subtypes.is_empty()
        || filter.zone.is_some()
        || filter.token
        || filter.nontoken
        || filter.face_down.is_some()
        || filter.tapped
        || filter.untapped
        || filter.attacking
        || filter
            .attacking_player_or_planeswalker_controlled_by
            .is_some()
        || filter.nonattacking
        || filter.blocking
        || filter.nonblocking
        || filter.blocked
        || filter.unblocked
        || filter.is_commander
        || filter.noncommander
        || !filter.excluded_colors.is_empty()
        || filter.colorless
        || filter.multicolored
        || filter.monocolored
        || filter.all_colors.is_some()
        || filter.exactly_two_colors.is_some()
        || filter.historic
        || filter.nonhistoric
        || filter.power.is_some()
        || filter.power_parity.is_some()
        || filter.toughness.is_some()
        || filter.mana_value.is_some()
        || filter.mana_value_parity.is_some()
        || filter.name.is_some()
        || filter.excluded_name.is_some()
        || filter.source
        || filter.with_counter.is_some()
        || filter.without_counter.is_some()
        || filter.total_counters_parity.is_some()
        || filter.alternative_cast.is_some()
        || !filter.static_abilities.is_empty()
        || !filter.excluded_static_abilities.is_empty()
        || !filter.ability_markers.is_empty()
        || !filter.excluded_ability_markers.is_empty()
        || filter.chosen_color
        || filter.chosen_creature_type
        || filter.excluded_chosen_creature_type
        || filter.colors.is_some()
        || !filter.tagged_constraints.is_empty()
        || filter.targets_player.is_some()
        || filter.targets_object.is_some()
        || !filter.any_of.is_empty();
    if !has_object_identity {
        return Err(CardTextError::ParseError(format!(
            "unsupported target phrase lacking object selector (clause: '{}')",
            all_words.join(" ")
        )));
    }

    if vote_winners_only {
        filter = filter.match_tagged(
            TagKey::from(VOTE_WINNERS_TAG),
            TaggedOpbjectRelation::IsTaggedObject,
        );
    }

    if not_on_battlefield && filter.any_of.is_empty() && !matches!(filter.zone, Some(Zone::Stack)) {
        let mut base = filter.clone();
        base.any_of.clear();
        base.zone = None;

        let mut disjunction = ObjectFilter::default();
        disjunction.any_of = [
            Zone::Hand,
            Zone::Library,
            Zone::Graveyard,
            Zone::Exile,
            Zone::Command,
        ]
        .into_iter()
        .map(|zone| {
            let mut branch = base.clone();
            branch.zone = Some(zone);
            branch
        })
        .collect();
        filter = disjunction;
    }

    // Strict mode: detect structural patterns in the input that indicate
    // unconsumed compound content (e.g. "for each card in your hand AND EACH
    // foretold card you own in exile" where the second clause was silently
    // absorbed into the first filter).
    if strict {
        let input_words_view = GrammarFilterNormalizedWords::new(&tokens);
        let input_words: Vec<&str> = input_words_view
            .to_word_refs()
            .into_iter()
            .filter(|word| !is_article(word))
            .collect();

        // "and each" / "and every" signals a compound count source when
        // the word after "each"/"every" introduces a new filter (type word,
        // zone word, etc.) rather than qualifying the current subject
        // (e.g. "and each other creature" is a subject qualifier, but
        // "and each foretold card you own in exile" is a new clause).
        for (idx, word) in input_words.iter().enumerate() {
            if *word != "and" {
                continue;
            }
            let next = input_words.get(idx + 1).copied();
            if !next.is_some_and(|n| matches!(n, "each" | "every")) {
                continue;
            }
            // "and each other" is typically a subject qualifier, allow it.
            let after_each = input_words.get(idx + 2).copied();
            if after_each.is_some_and(|w| matches!(w, "other" | "another")) {
                continue;
            }
            return Err(CardTextError::ParseError(format!(
                "object filter has unconsumed compound clause '{}' (full input: '{}')",
                input_words[idx..].join(" "),
                input_words.join(" "),
            )));
        }

        // "for each" signals a trailing iteration clause that should have
        // been split out by the caller before passing to the filter parser.
        for (idx, word) in input_words.iter().enumerate() {
            if *word == "for"
                && idx > 0
                && input_words.get(idx + 1).is_some_and(|next| *next == "each")
            {
                return Err(CardTextError::ParseError(format!(
                    "object filter has unconsumed 'for each' clause '{}' (full input: '{}')",
                    input_words[idx..].join(" "),
                    input_words.join(" "),
                )));
            }
        }
    }

    Ok(filter)
}

