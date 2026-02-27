use super::*;

pub(crate) fn parse_double_counters_clause(tokens: &[Token]) -> Result<Option<EffectAst>, CardTextError> {
    let clause_words = words(tokens);
    if !clause_words.starts_with(&["double", "the", "number", "of"]) {
        return Ok(None);
    }

    let counters_idx = tokens
        .iter()
        .position(|token| token.is_word("counter") || token.is_word("counters"))
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing counters keyword (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
    if counters_idx <= 4 {
        return Err(CardTextError::ParseError(format!(
            "missing counter type (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let counter_type =
        parse_counter_type_from_tokens(&tokens[4..counters_idx]).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported counter type in double-counters clause (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;

    let on_idx = tokens[counters_idx + 1..]
        .iter()
        .position(|token| token.is_word("on"))
        .map(|offset| counters_idx + 1 + offset)
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing 'on' in double-counters clause (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;

    let mut filter_tokens = &tokens[on_idx + 1..];
    if filter_tokens
        .first()
        .is_some_and(|token| token.is_word("each") || token.is_word("all"))
    {
        filter_tokens = &filter_tokens[1..];
    }
    if filter_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing filter in double-counters clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let filter = parse_object_filter(filter_tokens, false)?;
    Ok(Some(EffectAst::DoubleCountersOnEach {
        counter_type,
        filter,
    }))
}

pub(crate) fn parse_distribute_counters_clause(tokens: &[Token]) -> Result<Option<EffectAst>, CardTextError> {
    parse_distribute_counters_sentence(tokens)
}

pub(crate) fn parse_tagged_cast_or_play_target(words: &[&str]) -> Option<(bool, usize)> {
    if words.starts_with(&["it"]) || words.starts_with(&["them"]) {
        return Some((false, 1));
    }
    if words.starts_with(&["that", "card"])
        || words.starts_with(&["those", "cards"])
        || words.starts_with(&["that", "spell"])
        || words.starts_with(&["those", "spells"])
        || words.starts_with(&["the", "card"])
        || words.starts_with(&["the", "cards"])
    {
        return Some((false, 2));
    }
    if words.starts_with(&["the", "copy"])
        || words.starts_with(&["that", "copy"])
        || words.starts_with(&["a", "copy"])
    {
        return Some((true, 2));
    }
    None
}

pub(crate) fn parse_until_end_of_turn_may_play_tagged_clause(
    tokens: &[Token],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause_words = words(tokens);
    let prefix_len = if clause_words.starts_with(&["until", "the", "end", "of", "turn"]) {
        5
    } else if clause_words.starts_with(&["until", "end", "of", "turn"]) {
        4
    } else {
        return Ok(None);
    };

    let tail = &clause_words[prefix_len..];
    if !tail.starts_with(&["you", "may", "play"]) {
        return Err(CardTextError::ParseError(format!(
            "unsupported until-end-of-turn permission clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    let target_words = &tail[3..];
    let Some((as_copy, consumed)) = parse_tagged_cast_or_play_target(target_words) else {
        return Err(CardTextError::ParseError(format!(
            "unsupported until-end-of-turn play target (clause: '{}')",
            clause_words.join(" ")
        )));
    };
    if as_copy || consumed != target_words.len() {
        return Err(CardTextError::ParseError(format!(
            "unsupported until-end-of-turn play target (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    Ok(Some(EffectAst::GrantPlayTaggedUntilEndOfTurn {
        tag: TagKey::from(IT_TAG),
        player: PlayerAst::You,
    }))
}

pub(crate) fn parse_until_your_next_turn_may_play_tagged_clause(
    tokens: &[Token],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause_words = words(tokens);
    let prefix_len =
        if clause_words.starts_with(&["until", "the", "end", "of", "your", "next", "turn"]) {
            7
        } else if clause_words.starts_with(&["until", "end", "of", "your", "next", "turn"]) {
            6
        } else {
            return Ok(None);
        };

    let tail = &clause_words[prefix_len..];
    if !tail.starts_with(&["you", "may", "play"]) {
        return Err(CardTextError::ParseError(format!(
            "unsupported until-next-turn permission clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    let target_words = &tail[3..];
    let is_supported_target = matches!(
        target_words,
        ["those", "cards"] | ["those", "card"] | ["them"] | ["it"] | ["that", "card"]
    );
    if !is_supported_target {
        return Err(CardTextError::ParseError(format!(
            "unsupported until-next-turn play target (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    Ok(Some(EffectAst::GrantPlayTaggedUntilYourNextTurn {
        tag: TagKey::from(IT_TAG),
        player: PlayerAst::You,
    }))
}

pub(crate) fn parse_cast_or_play_tagged_clause(tokens: &[Token]) -> Result<Option<EffectAst>, CardTextError> {
    let mut clause_words = words(tokens);
    while clause_words
        .first()
        .is_some_and(|word| *word == "then" || *word == "and")
    {
        clause_words.remove(0);
    }
    let Some(verb_word) = clause_words.first().copied() else {
        return Ok(None);
    };
    let is_cast = verb_word == "cast";
    let is_play = verb_word == "play";
    if !is_cast && !is_play {
        return Ok(None);
    }

    let rest = &clause_words[1..];
    let Some((as_copy, consumed)) = parse_tagged_cast_or_play_target(rest) else {
        return Ok(None);
    };
    let tail = &rest[consumed..];

    let has_this_turn_duration = tail == ["this", "turn"];
    let has_until_end_of_turn_duration = tail == ["until", "end", "of", "turn"]
        || tail == ["until", "the", "end", "of", "turn"];
    if has_this_turn_duration || has_until_end_of_turn_duration {
        return Ok(Some(EffectAst::GrantPlayTaggedUntilEndOfTurn {
            tag: TagKey::from(IT_TAG),
            player: PlayerAst::Implicit,
        }));
    }

    let without_paying_its_cost = tail == ["without", "paying", "its", "mana", "cost"]
        || tail == ["without", "paying", "their", "mana", "cost"];
    if tail.is_empty() || without_paying_its_cost {
        return Ok(Some(EffectAst::CastTagged {
            tag: TagKey::from(IT_TAG),
            allow_land: is_play,
            as_copy,
            without_paying_mana_cost: without_paying_its_cost,
        }));
    }

    Ok(None)
}

pub(crate) fn parse_prevent_next_time_damage_sentence(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause_words = words(tokens);
    if !clause_words.starts_with(&["the", "next", "time"]) {
        return Ok(None);
    }

    let Some(would_idx) = clause_words.iter().position(|w| *w == "would") else {
        return Ok(None);
    };
    if clause_words.get(would_idx + 1..would_idx + 4) != Some(["deal", "damage", "to"].as_slice()) {
        return Ok(None);
    }

    // Must be "this turn ... prevent that damage".
    let this_turn_rel = clause_words[would_idx + 4..]
        .windows(2)
        .position(|window| window == ["this", "turn"])
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported prevent-next-time damage duration (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
    let this_turn_idx = (would_idx + 4) + this_turn_rel;

    let tail = &clause_words[this_turn_idx + 2..];
    if tail != ["prevent", "that", "damage"] {
        return Ok(None);
    }

    // Parse source phrase (between "time" and "would").
    let source_words = &clause_words[3..would_idx];
    if source_words.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing prevent-next-time damage source (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let source = if source_words
        .windows(3)
        .any(|w| w == ["of", "your", "choice"])
    {
        PreventNextTimeDamageSourceAst::Choice
    } else {
        // Patterns like "a red source", "an artifact source", "black or red source".
        let mut words = source_words.to_vec();
        if words.first().is_some_and(|w| matches!(*w, "a" | "an")) {
            words.remove(0);
        }
        if words.last().copied() == Some("source") {
            words.pop();
        }
        if words.is_empty() {
            return Ok(Some(vec![EffectAst::PreventNextTimeDamage {
                source: PreventNextTimeDamageSourceAst::Filter(ObjectFilter::default()),
                target: PreventNextTimeDamageTargetAst::AnyTarget,
            }]));
        }

        let mut filter = ObjectFilter::default();
        let mut colors: Option<crate::color::ColorSet> = None;
        for w in words {
            if matches!(w, "or" | "and") {
                continue;
            }
            if let Some(color) = parse_color(w) {
                colors = Some(
                    colors
                        .unwrap_or_else(crate::color::ColorSet::new)
                        .union(color),
                );
                continue;
            }
            if let Some(card_type) = parse_card_type(w) {
                if !filter.card_types.contains(&card_type) {
                    filter.card_types.push(card_type);
                }
                continue;
            }
            if w == "shadow" {
                filter = filter.with_static_ability(StaticAbilityId::Shadow);
                continue;
            }
        }
        if let Some(colors) = colors {
            // If only colors were set, COLORLESS ORing is harmless due to contains-any semantics.
            filter.colors = Some(colors);
        }

        PreventNextTimeDamageSourceAst::Filter(filter)
    };

    // Parse target phrase (between "to" and "this turn").
    let target_words = &clause_words[would_idx + 4..this_turn_idx];
    let target = if target_words == ["you"] {
        PreventNextTimeDamageTargetAst::You
    } else if target_words == ["any", "target"] {
        PreventNextTimeDamageTargetAst::AnyTarget
    } else {
        return Err(CardTextError::ParseError(format!(
            "unsupported prevent-next-time damage target scope (clause: '{}')",
            clause_words.join(" ")
        )));
    };

    Ok(Some(vec![EffectAst::PreventNextTimeDamage {
        source,
        target,
    }]))
}

pub(crate) fn parse_redirect_next_damage_sentence(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause_words = words(tokens);
    if clause_words.starts_with(&["the", "next", "time"]) {
        let Some(would_idx) = clause_words.iter().position(|word| *word == "would") else {
            return Ok(None);
        };
        if clause_words.get(would_idx + 1..would_idx + 4)
            != Some(["deal", "damage", "to"].as_slice())
        {
            return Ok(None);
        }

        let this_turn_rel = clause_words[would_idx + 4..]
            .windows(2)
            .position(|window| window == ["this", "turn"])
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported redirected-next-time damage duration (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?;
        let this_turn_idx = (would_idx + 4) + this_turn_rel;

        let source_words = &clause_words[3..would_idx];
        if source_words.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing redirected-next-time damage source (clause: '{}')",
                clause_words.join(" ")
            )));
        }

        let source = if source_words
            .windows(3)
            .any(|window| window == ["of", "your", "choice"])
        {
            PreventNextTimeDamageSourceAst::Choice
        } else {
            let mut words = source_words.to_vec();
            if words.first().is_some_and(|word| matches!(*word, "a" | "an")) {
                words.remove(0);
            }
            if words.last().copied() == Some("source") {
                words.pop();
            }
            let mut filter = ObjectFilter::default();
            let mut colors: Option<crate::color::ColorSet> = None;
            for word in words {
                if matches!(word, "or" | "and") {
                    continue;
                }
                if let Some(color) = parse_color(word) {
                    colors = Some(
                        colors
                            .unwrap_or_else(crate::color::ColorSet::new)
                            .union(color),
                    );
                    continue;
                }
                if let Some(card_type) = parse_card_type(word) {
                    if !filter.card_types.contains(&card_type) {
                        filter.card_types.push(card_type);
                    }
                    continue;
                }
                if word == "shadow" {
                    filter = filter.with_static_ability(StaticAbilityId::Shadow);
                    continue;
                }
            }
            if let Some(colors) = colors {
                filter.colors = Some(colors);
            }
            PreventNextTimeDamageSourceAst::Filter(filter)
        };

        let target_words = &clause_words[would_idx + 4..this_turn_idx];
        if target_words.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing redirected-next-time damage target (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let target_tokens = target_words
            .iter()
            .map(|word| Token::Word((*word).to_string(), TextSpan::synthetic()))
            .collect::<Vec<_>>();
        let target = parse_target_phrase(&target_tokens)?;

        let tail = &clause_words[this_turn_idx + 2..];
        if tail.len() < 7
            || !tail.starts_with(&["that", "damage", "is", "dealt", "to"])
            || tail.last().copied() != Some("instead")
        {
            return Ok(None);
        }
        let redirect_words = &tail[5..tail.len() - 1];
        let redirects_to_source = matches!(
            redirect_words,
            ["this"] | ["it"] | ["this", "creature"] | ["this", "permanent"]
        );
        if !redirects_to_source {
            return Err(CardTextError::ParseError(format!(
                "unsupported redirected-next-time damage destination (clause: '{}')",
                clause_words.join(" ")
            )));
        }

        return Ok(Some(vec![EffectAst::RedirectNextTimeDamageToSource {
            source,
            target,
        }]));
    }

    if !clause_words.starts_with(&["the", "next"]) {
        return Ok(None);
    }

    let amount_token = Token::Word(
        clause_words.get(2).copied().unwrap_or_default().to_string(),
        TextSpan::synthetic(),
    );
    let Some((amount, amount_used)) = parse_value(&[amount_token]) else {
        return Ok(None);
    };
    if amount_used != 1 {
        return Err(CardTextError::ParseError(format!(
            "unsupported redirected-next-damage amount (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let mut idx = 3usize;
    if clause_words.get(idx..idx + 6)
        != Some(["damage", "that", "would", "be", "dealt", "to"].as_slice())
    {
        return Ok(None);
    }
    idx += 6;

    let this_turn_rel = clause_words[idx..]
        .windows(2)
        .position(|window| window == ["this", "turn"])
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported redirected-next-damage duration (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
    let this_turn_idx = idx + this_turn_rel;
    let protected_words = &clause_words[idx..this_turn_idx];
    let protects_source = matches!(
        protected_words,
        ["this"] | ["it"] | ["this", "creature"] | ["this", "permanent"]
    );
    if !protects_source {
        return Err(CardTextError::ParseError(format!(
            "unsupported redirected-next-damage protected target (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let tail = &clause_words[this_turn_idx + 2..];
    if tail.len() < 5
        || !tail.starts_with(&["is", "dealt", "to"])
        || tail.last().copied() != Some("instead")
    {
        return Ok(None);
    }

    let target_words = &tail[3..tail.len() - 1];
    if target_words.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing redirected-next-damage target (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    let target_tokens = target_words
        .iter()
        .map(|word| Token::Word((*word).to_string(), TextSpan::synthetic()))
        .collect::<Vec<_>>();
    let target = parse_target_phrase(&target_tokens)?;

    Ok(Some(vec![EffectAst::RedirectNextDamageFromSourceToTarget {
        amount,
        target,
    }]))
}

pub(crate) fn parse_prevent_next_damage_clause(tokens: &[Token]) -> Result<Option<EffectAst>, CardTextError> {
    let clause_words = words(tokens);
    if clause_words.first().copied() != Some("prevent") {
        return Ok(None);
    }

    let mut idx = 1usize;
    if clause_words.get(idx) == Some(&"the") {
        idx += 1;
    }
    if clause_words.get(idx) != Some(&"next") {
        return Ok(None);
    }
    idx += 1;

    let amount_token = Token::Word(
        clause_words
            .get(idx)
            .copied()
            .unwrap_or_default()
            .to_string(),
        TextSpan::synthetic(),
    );
    let Some((amount, amount_used)) = parse_value(&[amount_token]) else {
        return Err(CardTextError::ParseError(format!(
            "missing prevent damage amount (clause: '{}')",
            clause_words.join(" ")
        )));
    };
    idx += amount_used;

    if clause_words.get(idx) != Some(&"damage") {
        return Ok(None);
    }
    idx += 1;

    if clause_words.get(idx..idx + 4) != Some(["that", "would", "be", "dealt"].as_slice()) {
        return Err(CardTextError::ParseError(format!(
            "unsupported prevent-next damage clause tail (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    idx += 4;

    if clause_words.get(idx) != Some(&"to") {
        return Err(CardTextError::ParseError(format!(
            "unsupported prevent-next damage target scope (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    idx += 1;

    let this_turn_rel = clause_words[idx..]
        .windows(2)
        .position(|window| window == ["this", "turn"])
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported prevent-next damage duration (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
    let this_turn_idx = idx + this_turn_rel;
    if this_turn_idx + 2 != clause_words.len() {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing prevent-next damage clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let target_words = &clause_words[idx..this_turn_idx];
    if target_words.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing prevent-next damage target (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    let target_tokens = target_words
        .iter()
        .map(|word| Token::Word((*word).to_string(), TextSpan::synthetic()))
        .collect::<Vec<_>>();
    let target = parse_target_phrase(&target_tokens)?;

    Ok(Some(EffectAst::PreventDamage {
        amount,
        target,
        duration: Until::EndOfTurn,
    }))
}

pub(crate) fn parse_prevent_all_damage_clause(tokens: &[Token]) -> Result<Option<EffectAst>, CardTextError> {
    let clause_words = words(tokens);
    let prefix = [
        "prevent", "all", "damage", "that", "would", "be", "dealt", "to",
    ];
    if !clause_words.starts_with(&prefix) {
        return Ok(None);
    }
    if clause_words.len() <= prefix.len() + 1 {
        return Err(CardTextError::ParseError(format!(
            "missing prevent-all damage target (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    if clause_words[clause_words.len().saturating_sub(2)..] != ["this", "turn"] {
        return Err(CardTextError::ParseError(format!(
            "unsupported prevent-all damage duration (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let target_words = &clause_words[prefix.len()..clause_words.len() - 2];
    if target_words.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing prevent-all damage target (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let target_tokens = target_words
        .iter()
        .map(|word| Token::Word((*word).to_string(), TextSpan::synthetic()))
        .collect::<Vec<_>>();
    let target = parse_target_phrase(&target_tokens)?;

    Ok(Some(EffectAst::PreventAllDamageToTarget {
        target,
        duration: Until::EndOfTurn,
    }))
}

pub(crate) fn parse_can_attack_as_though_no_defender_clause(
    tokens: &[Token],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause_words = words(tokens);
    let Some(can_idx) = clause_words.iter().position(|word| *word == "can") else {
        return Ok(None);
    };
    let subject_words = &clause_words[..can_idx];
    let tail = &clause_words[can_idx..];
    let has_core = tail.starts_with(&["can", "attack"])
        && tail.windows(2).any(|window| window == ["as", "though"])
        && tail.contains(&"turn")
        && tail.contains(&"have")
        && tail.last().copied() == Some("defender");
    if !has_core {
        return Ok(None);
    }

    let target = if subject_words.is_empty() {
        TargetAst::Tagged(TagKey::from(IT_TAG), Some(TextSpan::synthetic()))
    } else {
        let subject_tokens = subject_words
            .iter()
            .map(|word| Token::Word((*word).to_string(), TextSpan::synthetic()))
            .collect::<Vec<_>>();
        parse_target_phrase(&subject_tokens)?
    };

    Ok(Some(EffectAst::GrantAbilitiesToTarget {
        target,
        abilities: vec![StaticAbility::can_attack_as_though_no_defender()],
        duration: Until::EndOfTurn,
    }))
}

pub(crate) fn parse_win_the_game_clause(tokens: &[Token]) -> Result<Option<EffectAst>, CardTextError> {
    let clause_words = words(tokens);
    if clause_words.len() < 4 {
        return Ok(None);
    }

    if !clause_words.starts_with(&["you", "win", "the", "game"]) {
        return Ok(None);
    }

    if clause_words.len() == 4 {
        return Ok(Some(EffectAst::WinGame {
            player: PlayerAst::You,
        }));
    }

    if clause_words.get(4).copied() != Some("if") {
        return Ok(None);
    }

    let if_tail = clause_words.get(5..).unwrap_or_default();
    if if_tail.len() < 6
        || if_tail[0] != "you"
        || if_tail[1] != "own"
        || !matches!(if_tail[2], "a" | "an" | "the")
        || if_tail[3] != "card"
        || if_tail[4] != "named"
    {
        return Ok(None);
    }

    let after_named = &if_tail[5..];
    let Some(in_idx) = after_named.iter().position(|word| *word == "in") else {
        return Ok(None);
    };
    if in_idx == 0 {
        return Ok(None);
    }

    let name_words = &after_named[..in_idx];
    let remainder = &after_named[in_idx..];

    let has_exile = remainder.contains(&"exile");
    let has_hand = remainder.contains(&"hand");
    let has_graveyard = remainder.contains(&"graveyard");
    let has_battlefield = remainder.contains(&"battlefield");
    if !(has_exile && has_hand && has_graveyard && has_battlefield) {
        return Ok(None);
    }

    let name = name_words
        .iter()
        .map(|word| title_case_token_word(word))
        .collect::<Vec<_>>()
        .join(" ");
    if name.is_empty() {
        return Ok(None);
    }

    Ok(Some(EffectAst::Conditional {
        predicate: PredicateAst::PlayerOwnsCardNamedInZones {
            player: PlayerAst::You,
            name,
            zones: vec![Zone::Exile, Zone::Hand, Zone::Graveyard, Zone::Battlefield],
        },
        if_true: vec![EffectAst::WinGame {
            player: PlayerAst::You,
        }],
        if_false: Vec::new(),
    }))
}

pub(crate) fn parse_copy_spell_clause(tokens: &[Token]) -> Result<Option<EffectAst>, CardTextError> {
    let clause_words = words(tokens);
    let Some(copy_idx) = tokens
        .iter()
        .position(|token| token.is_word("copy") || token.is_word("copies"))
    else {
        return Ok(None);
    };
    let simple_copy_reference = copy_idx == 0
        && matches!(
            clause_words.get(1).copied(),
            Some("it") | Some("this") | Some("that")
        );
    if simple_copy_reference {
        if let Some(then_idx) = tokens.iter().position(|token| token.is_word("then")) {
            let tail_tokens = trim_commas(&tokens[then_idx + 1..]);
            if let Some(spec) = parse_may_cast_it_sentence(&tail_tokens)
                && spec.as_copy
            {
                return Ok(Some(build_may_cast_tagged_effect(&spec)));
            }
        }
        let base = EffectAst::CopySpell {
            target: TargetAst::Source(None),
            count: Value::Fixed(1),
            player: PlayerAst::Implicit,
            may_choose_new_targets: false,
        };
        if let Some(if_idx) = tokens.iter().position(|token| token.is_word("if")) {
            let predicate_tokens = trim_commas(&tokens[if_idx + 1..]);
            if predicate_tokens.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "missing predicate after copy clause (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            let predicate = parse_predicate(&predicate_tokens)?;
            return Ok(Some(EffectAst::Conditional {
                predicate,
                if_true: vec![base],
                if_false: Vec::new(),
            }));
        }
        return Ok(Some(base));
    }
    if !clause_words.contains(&"spell") && !clause_words.contains(&"spells") {
        return Ok(None);
    }

    let subject = parse_subject(&tokens[..copy_idx]);
    let player = match subject {
        SubjectAst::Player(player) => player,
        SubjectAst::This => PlayerAst::Implicit,
    };

    let tail = &tokens[copy_idx + 1..];
    if tail.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing spell target in copy clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let mut split_idx = None;
    for idx in 0..tail.len() {
        if !tail[idx].is_word("and") {
            continue;
        }
        let mut after = words(&tail[idx + 1..]);
        if after.first().copied() == Some("may") {
            after.remove(0);
        }
        if after.first().copied() == Some("choose")
            && after
                .iter()
                .any(|word| *word == "target" || *word == "targets")
            && after.iter().any(|word| *word == "copy")
        {
            split_idx = Some(idx);
            break;
        }
    }

    let mut count = Value::Fixed(1);
    let mut copy_target_tail = if let Some(idx) = split_idx {
        &tail[..idx]
    } else {
        tail
    };
    if let Some(for_each_idx) = copy_target_tail
        .windows(2)
        .position(|window| window[0].is_word("for") && window[1].is_word("each"))
    {
        let count_filter_tokens = trim_commas(&copy_target_tail[for_each_idx + 2..]);
        if count_filter_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing count filter after 'for each' in copy clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let count_filter = parse_object_filter(&count_filter_tokens, false)?;
        count = Value::Count(count_filter);
        copy_target_tail = &copy_target_tail[..for_each_idx];
    }

    let copy_target_tokens = trim_commas(copy_target_tail);
    if copy_target_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing spell target in copy clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let target_words = words(&copy_target_tokens);
    let target = if target_words.as_slice() == ["this", "spell"]
        || target_words.as_slice() == ["that", "spell"]
    {
        TargetAst::Source(None)
    } else {
        parse_target_phrase(&copy_target_tokens)?
    };

    let mut may_choose_new_targets = false;
    if let Some(idx) = split_idx {
        let mut choose_words = words(&tail[idx + 1..]);
        if choose_words.first().copied() == Some("may") {
            may_choose_new_targets = true;
            choose_words.remove(0);
        }
        let has_choose = choose_words.first().copied() == Some("choose");
        let has_new = choose_words.contains(&"new");
        let has_target = choose_words
            .iter()
            .any(|word| *word == "target" || *word == "targets");
        let has_copy = choose_words.contains(&"copy");
        if !has_choose || !has_target || !has_copy {
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing copy clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        if !has_new {
            return Err(CardTextError::ParseError(format!(
                "missing 'new' in copy retarget clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
    }

    Ok(Some(EffectAst::CopySpell {
        target,
        count,
        player,
        may_choose_new_targets,
    }))
}

pub(crate) fn parse_verb_first_clause(tokens: &[Token]) -> Result<Option<EffectAst>, CardTextError> {
    let Some(Token::Word(word, _)) = tokens.first() else {
        return Ok(None);
    };

    let verb = match word.as_str() {
        "add" => Verb::Add,
        "move" => Verb::Move,
        "counter" => Verb::Counter,
        "destroy" => Verb::Destroy,
        "exile" => Verb::Exile,
        "draw" => Verb::Draw,
        "deal" => Verb::Deal,
        "sacrifice" => Verb::Sacrifice,
        "create" => Verb::Create,
        "investigate" => Verb::Investigate,
        "proliferate" => Verb::Proliferate,
        "tap" => Verb::Tap,
        "attach" => Verb::Attach,
        "untap" => Verb::Untap,
        "scry" => Verb::Scry,
        "discard" => Verb::Discard,
        "transform" => Verb::Transform,
        "regenerate" => Verb::Regenerate,
        "mill" => Verb::Mill,
        "get" => Verb::Get,
        "remove" => Verb::Remove,
        "return" => Verb::Return,
        "exchange" => Verb::Exchange,
        "become" => Verb::Become,
        "skip" => Verb::Skip,
        "surveil" => Verb::Surveil,
        "shuffle" => Verb::Shuffle,
        "pay" => Verb::Pay,
        "goad" => Verb::Goad,
        "look" => Verb::Look,
        _ => return Ok(None),
    };

    let effect = parse_effect_with_verb(verb, None, &tokens[1..])?;
    Ok(Some(effect))
}

pub(crate) fn is_simple_chosen_object_reference(tokens: &[Token]) -> bool {
    let words: Vec<&str> = words(tokens)
        .into_iter()
        .filter(|word| !is_article(word) && *word != "then")
        .collect();
    if words.is_empty() {
        return false;
    }
    if words == ["it"] || words == ["them"] {
        return true;
    }
    if has_demonstrative_object_reference(&words) {
        return true;
    }
    false
}

pub(crate) fn parse_choose_target_and_verb_clause(
    tokens: &[Token],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause_words = words(tokens);
    if !clause_words.starts_with(&["choose", "target"]) {
        return Ok(None);
    }

    let Some(and_idx) = tokens.iter().position(|token| token.is_word("and")) else {
        return Ok(None);
    };
    if and_idx <= 1 {
        return Ok(None);
    }

    let target_tokens = trim_commas(&tokens[1..and_idx]);
    if target_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing target after choose clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    if find_verb(&target_tokens).is_some() {
        return Ok(None);
    }

    let mut tail_tokens = trim_commas(&tokens[and_idx + 1..]);
    if tail_tokens
        .first()
        .is_some_and(|token| token.is_word("then"))
    {
        tail_tokens = tail_tokens[1..].to_vec();
    }
    if tail_tokens.is_empty() {
        return Ok(None);
    }

    let Some((verb, verb_idx)) = find_verb(&tail_tokens) else {
        return Ok(None);
    };
    if verb_idx != 0 {
        return Ok(None);
    }

    let rest_tokens = trim_commas(&tail_tokens[1..]);
    if !is_simple_chosen_object_reference(&rest_tokens) {
        return Ok(None);
    }

    let effect = parse_effect_with_verb(verb, None, &target_tokens)?;
    Ok(Some(effect))
}

pub(crate) fn parse_choose_target_prelude_sentence(tokens: &[Token]) -> Result<Option<EffectAst>, CardTextError> {
    let clause_words = words(tokens);
    if clause_words.first().copied() != Some("choose") {
        return Ok(None);
    }

    let target_tokens = trim_commas(&tokens[1..]);
    if target_tokens.is_empty() || !starts_with_target_indicator(&target_tokens) {
        return Ok(None);
    }
    if find_verb(&target_tokens).is_some() {
        return Ok(None);
    }

    let target = parse_target_phrase(&target_tokens)?;
    Ok(Some(EffectAst::TargetOnly { target }))
}

pub(crate) fn parse_keyword_mechanic_clause(tokens: &[Token]) -> Result<Option<EffectAst>, CardTextError> {
    if tokens.is_empty() {
        return Ok(None);
    }

    let mut start = 0usize;
    if tokens.get(start).is_some_and(|token| token.is_word("then")) {
        start += 1;
    }
    if tokens.get(start).is_some_and(|token| token.is_word("you")) {
        start += 1;
    }
    if start >= tokens.len() {
        return Ok(None);
    }

    let clause_tokens = &tokens[start..];
    let clause_words = words(clause_tokens);
    if clause_words.is_empty() {
        return Ok(None);
    }

    if clause_words.first() == Some(&"amass") {
        return Err(CardTextError::ParseError(format!(
            "unsupported amass mechanic (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    if clause_words.starts_with(&["open", "an", "attraction"])
        || clause_words.starts_with(&["opens", "an", "attraction"])
    {
        return Ok(Some(EffectAst::OpenAttraction));
    }

    if clause_words == ["manifest", "dread"] {
        return Ok(Some(EffectAst::ManifestDread));
    }

    if matches!(
        clause_words.first().copied(),
        Some("bolster" | "support" | "adapt")
    ) {
        let keyword = clause_words[0];
        let (amount, used) = parse_number(&clause_tokens[1..]).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing numeric amount for {keyword} clause (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
        if 1 + used != clause_tokens.len() {
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing {keyword} clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let effect = match keyword {
            "bolster" => EffectAst::Bolster { amount },
            "support" => EffectAst::Support { amount },
            "adapt" => EffectAst::Adapt { amount },
            _ => unreachable!(),
        };
        return Ok(Some(effect));
    }

    if clause_words.first() == Some(&"fateseal") {
        let (count, used) = parse_value(&clause_tokens[1..]).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing amount for fateseal clause (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
        if 1 + used != clause_tokens.len() {
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing fateseal clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        return Ok(Some(EffectAst::Scry {
            count,
            player: PlayerAst::Opponent,
        }));
    }

    if matches!(
        clause_words.first().copied(),
        Some("discover" | "discovers")
    ) {
        let (count, used) = parse_value(&clause_tokens[1..]).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing amount for discover clause (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
        if 1 + used != clause_tokens.len() {
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing discover clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        return Ok(Some(EffectAst::Discover {
            count,
            player: PlayerAst::You,
        }));
    }

    if matches!(clause_words.last().copied(), Some("explore" | "explores")) {
        let subject_tokens = &clause_tokens[..clause_tokens.len().saturating_sub(1)];
        let subject_words = words(subject_tokens);
        let target = if subject_words.is_empty()
            || subject_words == ["it"]
            || subject_words == ["this"]
            || subject_words == ["this", "creature"]
            || subject_words == ["this", "permanent"]
        {
            TargetAst::Source(span_from_tokens(subject_tokens))
        } else {
            parse_target_phrase(subject_tokens)?
        };
        return Ok(Some(EffectAst::Explore { target }));
    }

    Ok(None)
}

pub(crate) fn parse_connive_clause(tokens: &[Token]) -> Result<Option<EffectAst>, CardTextError> {
    let Some(connive_idx) = tokens
        .iter()
        .rposition(|token| token.is_word("connive") || token.is_word("connives"))
    else {
        return Ok(None);
    };

    // We currently only support trailing "connive/connives" clauses.
    if tokens[connive_idx + 1..]
        .iter()
        .any(|token| token.as_word().is_some())
    {
        return Ok(None);
    }

    let subject_tokens = &tokens[..connive_idx];
    if subject_tokens.is_empty() {
        return Ok(None);
    }

    let subject_words = words(subject_tokens);
    if subject_words == ["each", "creature", "that", "convoked", "this", "spell"] {
        return Ok(Some(EffectAst::ForEachTagged {
            tag: TagKey::from("convoked_this_spell"),
            effects: vec![EffectAst::ConniveIterated],
        }));
    }

    let target = parse_target_phrase(subject_tokens)?;
    Ok(Some(EffectAst::Connive { target }))
}

pub(crate) fn find_verb(tokens: &[Token]) -> Option<(Verb, usize)> {
    for (idx, token) in tokens.iter().enumerate() {
        let Some(word) = token.as_word() else {
            continue;
        };
        if matches!(word, "counter" | "counters")
            && tokens
                .get(idx + 1)
                .and_then(Token::as_word)
                .is_some_and(|next| matches!(next, "on" | "from" | "among"))
        {
            continue;
        }
        let verb = match word {
            "adds" | "add" => Verb::Add,
            "moves" | "move" => Verb::Move,
            "deals" | "deal" => Verb::Deal,
            "draws" | "draw" => Verb::Draw,
            "counters" | "counter" => Verb::Counter,
            "destroys" | "destroy" => Verb::Destroy,
            "exiles" | "exile" => Verb::Exile,
            "reveals" | "reveal" => Verb::Reveal,
            "looks" | "look" => Verb::Look,
            "loses" | "lose" => Verb::Lose,
            "gains" | "gain" => Verb::Gain,
            "puts" | "put" => Verb::Put,
            "sacrifices" | "sacrifice" => Verb::Sacrifice,
            "creates" | "create" => Verb::Create,
            "investigates" | "investigate" => Verb::Investigate,
            "proliferates" | "proliferate" => Verb::Proliferate,
            "taps" | "tap" => Verb::Tap,
            "attaches" | "attach" => Verb::Attach,
            "untaps" | "untap" => Verb::Untap,
            "scries" | "scry" => Verb::Scry,
            "discards" | "discard" => Verb::Discard,
            "transforms" | "transform" => Verb::Transform,
            "flips" | "flip" => Verb::Flip,
            "regenerates" | "regenerate" => Verb::Regenerate,
            "mills" | "mill" => Verb::Mill,
            "gets" | "get" => Verb::Get,
            "removes" | "remove" => Verb::Remove,
            "returns" | "return" => Verb::Return,
            "exchanges" | "exchange" => Verb::Exchange,
            "becomes" | "become" => Verb::Become,
            "switches" | "switch" => Verb::Switch,
            "skips" | "skip" => Verb::Skip,
            "surveils" | "surveil" => Verb::Surveil,
            "shuffles" | "shuffle" => Verb::Shuffle,
            "reorders" | "reorder" => Verb::Reorder,
            "pays" | "pay" => Verb::Pay,
            "goads" | "goad" => Verb::Goad,
            _ => continue,
        };
        return Some((verb, idx));
    }

    None
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SubjectAst {
    This,
    Player(PlayerAst),
}

pub(crate) fn parse_subject(tokens: &[Token]) -> SubjectAst {
    let words = words(tokens);
    if words.is_empty() {
        return SubjectAst::This;
    }

    let mut start = 0usize;
    if words.starts_with(&["any", "number", "of"]) {
        start = 3;
    }

    let mut slice = &words[start..];
    while slice
        .first()
        .is_some_and(|word| *word == "then" || *word == "and")
    {
        slice = &slice[1..];
    }
    if let Some(have_idx) = slice
        .iter()
        .position(|word| *word == "have" || *word == "has")
    {
        if have_idx + 1 < slice.len() {
            slice = &slice[have_idx + 1..];
        }
    }

    if slice.starts_with(&["you"]) || slice.starts_with(&["your"]) {
        return SubjectAst::Player(PlayerAst::You);
    }

    if slice.starts_with(&["target", "opponent"]) || slice.starts_with(&["target", "opponents"]) {
        return SubjectAst::Player(PlayerAst::TargetOpponent);
    }

    if slice.starts_with(&["target", "player"]) || slice.starts_with(&["target", "players"]) {
        return SubjectAst::Player(PlayerAst::Target);
    }

    if slice.starts_with(&["opponent"])
        || slice.starts_with(&["opponents"])
        || slice.starts_with(&["an", "opponent"])
    {
        return SubjectAst::Player(PlayerAst::Opponent);
    }

    if slice.starts_with(&["defending", "player"]) {
        return SubjectAst::Player(PlayerAst::Defending);
    }
    if slice.ends_with(&["defending", "player"]) {
        return SubjectAst::Player(PlayerAst::Defending);
    }
    if slice.starts_with(&["attacking", "player"])
        || slice.starts_with(&["the", "attacking", "player"])
    {
        return SubjectAst::Player(PlayerAst::Attacking);
    }
    if slice.ends_with(&["attacking", "player"]) {
        return SubjectAst::Player(PlayerAst::Attacking);
    }

    if slice.starts_with(&["that", "player"]) {
        return SubjectAst::Player(PlayerAst::That);
    }

    if slice.starts_with(&["that", "players"]) || slice.starts_with(&["their"]) {
        return SubjectAst::Player(PlayerAst::That);
    }

    // Handle possessive references like "that creature's controller" /
    // "that permanent's controller" after tokenizer apostrophe normalization.
    if slice.len() >= 3
        && slice[0] == "that"
        && (slice[2] == "controller" || slice[2] == "owner")
        && (slice[1] == "creatures"
            || slice[1] == "permanents"
            || slice[1] == "sources"
            || slice[1] == "spells")
    {
        let player = if slice[2] == "owner" {
            PlayerAst::ItsOwner
        } else {
            PlayerAst::ItsController
        };
        return SubjectAst::Player(player);
    }

    if slice.starts_with(&["its", "controller"]) {
        return SubjectAst::Player(PlayerAst::ItsController);
    }
    if slice.starts_with(&["its", "owner"]) {
        return SubjectAst::Player(PlayerAst::ItsOwner);
    }
    if slice.starts_with(&["their", "owner"]) {
        return SubjectAst::Player(PlayerAst::ItsOwner);
    }
    if slice.ends_with(&["its", "controller"]) || slice.ends_with(&["their", "controller"]) {
        return SubjectAst::Player(PlayerAst::ItsController);
    }
    if slice.ends_with(&["its", "owner"]) || slice.ends_with(&["their", "owner"]) {
        return SubjectAst::Player(PlayerAst::ItsOwner);
    }

    if slice.starts_with(&["this"]) || slice.starts_with(&["thiss"]) {
        return SubjectAst::This;
    }

    SubjectAst::This
}

