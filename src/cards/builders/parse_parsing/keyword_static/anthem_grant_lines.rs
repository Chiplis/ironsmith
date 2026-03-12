pub(crate) fn parse_subject_cant_be_blocked_line(
    tokens: &[Token],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let normalized = words(tokens)
        .into_iter()
        .map(|word| if word == "cannot" { "cant" } else { word })
        .collect::<Vec<_>>();
    if normalized.len() < 4 || !normalized.ends_with(&["cant", "be", "blocked"]) {
        return Ok(None);
    }

    let tail_start = token_index_for_word_index(tokens, normalized.len() - 3).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unable to map cant-be-blocked tail (clause: '{}')",
            normalized.join(" ")
        ))
    })?;
    let subject_tokens = trim_commas(&tokens[..tail_start]);
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    if subject_tokens
        .iter()
        .any(|token| matches!(token, Token::Comma(_)) || token.is_word("and"))
    {
        return Ok(None);
    }
    let subject_words = words(&subject_tokens);
    if subject_words
        .first()
        .is_some_and(|word| *word == "this" || *word == "it")
    {
        return Ok(None);
    }
    if subject_words.iter().any(|word| {
        matches!(
            *word,
            "as" | "long"
                | "if"
                | "when"
                | "whenever"
                | "get"
                | "gets"
                | "gain"
                | "gains"
                | "have"
                | "has"
        )
    }) {
        return Ok(None);
    }
    if subject_words.windows(3).any(|window| {
        window == ["power", "or", "toughness"] || window == ["toughness", "or", "power"]
    }) {
        return Err(CardTextError::ParseError(format!(
            "unsupported power-or-toughness cant-be-blocked subject (clause: '{}')",
            words(tokens).join(" ")
        )));
    }

    let subject = parse_anthem_subject(&subject_tokens)?;
    let ability = match subject {
        AnthemSubjectAst::Source => StaticAbilityAst::KeywordAction(KeywordAction::Unblockable),
        AnthemSubjectAst::Filter(filter) => StaticAbilityAst::GrantKeywordAction {
            filter,
            action: KeywordAction::Unblockable,
            condition: None,
        },
    };
    Ok(Some(ability))
}

pub(crate) fn parse_subject_cant_be_blocked_as_long_as_condition_line(
    tokens: &[Token],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let normalized = words(tokens)
        .into_iter()
        .map(|word| if word == "cannot" { "cant" } else { word })
        .collect::<Vec<_>>();

    let Some(cant_idx) = normalized
        .windows(3)
        .position(|window| window == ["cant", "be", "blocked"])
    else {
        return Ok(None);
    };

    let tail = &normalized[cant_idx + 3..];
    if !tail.starts_with(&["as", "long", "as"]) {
        return Ok(None);
    }

    let subject_end = token_index_for_word_index(tokens, cant_idx).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unable to map cant-be-blocked subject (clause: '{}')",
            normalized.join(" ")
        ))
    })?;
    let subject_tokens = trim_commas(&tokens[..subject_end]);
    if subject_tokens.is_empty() {
        return Ok(None);
    }

    let condition_start = token_index_for_word_index(tokens, cant_idx + 6).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unable to map cant-be-blocked condition (clause: '{}')",
            normalized.join(" ")
        ))
    })?;
    let condition_tokens = trim_commas(&tokens[condition_start..]);
    if condition_tokens.is_empty() {
        return Ok(None);
    }
    let condition = parse_static_condition_clause(&condition_tokens)?;

    let subject = parse_anthem_subject(&subject_tokens)?;
    let granted = match subject {
        AnthemSubjectAst::Source => StaticAbilityAst::ConditionalKeywordAction {
            action: KeywordAction::Unblockable,
            condition,
        },
        AnthemSubjectAst::Filter(filter) => StaticAbilityAst::GrantKeywordAction {
            filter,
            action: KeywordAction::Unblockable,
            condition: Some(condition),
        },
    };
    Ok(Some(granted))
}

pub(crate) fn parse_subject_cant_be_blocked_as_long_as_defending_player_controls_card_type_line(
    tokens: &[Token],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let normalized = words(tokens)
        .into_iter()
        .map(|word| if word == "cannot" { "cant" } else { word })
        .collect::<Vec<_>>();

    let Some(cant_idx) = normalized
        .windows(3)
        .position(|window| window == ["cant", "be", "blocked"])
    else {
        return Ok(None);
    };

    let tail = &normalized[cant_idx + 3..];
    if tail.len() < 7 || !tail.starts_with(&["as", "long", "as", "defending", "player", "controls"])
    {
        return Ok(None);
    }

    let mut type_words = &tail[6..];
    if matches!(type_words.first(), Some(&"a" | &"an" | &"the")) {
        type_words = &type_words[1..];
    }
    if type_words.is_empty() {
        return Ok(None);
    }
    let mut card_types = Vec::with_capacity(type_words.len());
    for type_word in type_words {
        let Some(card_type) = parse_card_type(type_word) else {
            return Ok(None);
        };
        if !matches!(
            card_type,
            CardType::Artifact
                | CardType::Battle
                | CardType::Creature
                | CardType::Enchantment
                | CardType::Land
                | CardType::Planeswalker
        ) {
            return Ok(None);
        }
        card_types.push(card_type);
    }

    let tail_start = token_index_for_word_index(tokens, cant_idx).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unable to map cant-be-blocked conditional tail (clause: '{}')",
            normalized.join(" ")
        ))
    })?;
    let subject_tokens = trim_commas(&tokens[..tail_start]);
    if subject_tokens.is_empty() {
        return Ok(None);
    }

    let subject = parse_anthem_subject(&subject_tokens)?;
    let unblockable = if card_types.len() == 1 {
        StaticAbility::cant_be_blocked_as_long_as_defending_player_controls_card_type(card_types[0])
    } else {
        StaticAbility::cant_be_blocked_as_long_as_defending_player_controls_card_types(card_types)
    };
    let ability = match subject {
        AnthemSubjectAst::Source => StaticAbilityAst::Static(unblockable),
        AnthemSubjectAst::Filter(filter) => StaticAbilityAst::GrantStaticAbility {
            filter,
            ability: Box::new(StaticAbilityAst::Static(unblockable)),
            condition: None,
        },
    };
    Ok(Some(ability))
}

pub(crate) fn parse_granted_keyword_static_line(
    tokens: &[Token],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    fn extract_grant_spec_from_subject(
        subject_tokens: &[Token],
        grantable: crate::grant::Grantable,
    ) -> Result<Option<crate::grant::GrantSpec>, CardTextError> {
        let subject = parse_anthem_subject(subject_tokens)?;
        let AnthemSubjectAst::Filter(mut filter) = subject else {
            return Ok(None);
        };
        let zone = filter.zone.unwrap_or(Zone::Battlefield);
        filter.zone = None;
        Ok(Some(crate::grant::GrantSpec::new(grantable, filter, zone)))
    }

    fn parse_granted_escape_cost_tail(
        trailing_tokens: &[Token],
    ) -> Result<Option<u32>, CardTextError> {
        let trailing_words = words(trailing_tokens);
        let Some(prefix_len) = (match trailing_words.as_slice() {
            [
                "the",
                "escape",
                "cost",
                "is",
                "equal",
                "to",
                "the",
                "cards",
                "mana",
                "cost",
                "plus",
                ..,
            ] => Some(11usize),
            [
                "its",
                "escape",
                "cost",
                "is",
                "equal",
                "to",
                "its",
                "mana",
                "cost",
                "plus",
                ..,
            ] => Some(10usize),
            _ => None,
        }) else {
            return Ok(None);
        };

        let Some(exile_idx) = token_index_for_word_index(trailing_tokens, prefix_len) else {
            return Ok(None);
        };
        let exile_tokens = trailing_tokens.get(exile_idx..).unwrap_or_default();
        if !exile_tokens
            .first()
            .is_some_and(|token| token.is_word("exile"))
        {
            return Err(CardTextError::ParseError(format!(
                "unsupported escape cost clause (clause: '{}')",
                trailing_words.join(" ")
            )));
        }
        let Some((count, used)) = parse_number(exile_tokens.get(1..).unwrap_or_default()) else {
            return Err(CardTextError::ParseError(format!(
                "escape cost clause missing exile count (clause: '{}')",
                trailing_words.join(" ")
            )));
        };
        let tail = words(exile_tokens.get(1 + used..).unwrap_or_default());
        if tail.as_slice() != ["other", "cards", "from", "your", "graveyard"]
            && tail.as_slice() != ["other", "card", "from", "your", "graveyard"]
        {
            return Err(CardTextError::ParseError(format!(
                "unsupported escape cost clause (clause: '{}')",
                trailing_words.join(" ")
            )));
        }
        Ok(Some(count as u32))
    }

    fn parse_granted_alternative_cast_static(
        subject_tokens: &[Token],
        keyword_tokens: &[Token],
        trailing_tokens: &[Token],
        condition: Option<crate::ConditionExpr>,
    ) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
        let keyword_words = words(keyword_tokens);
        let spec = match keyword_words.as_slice() {
            ["flashback"] => {
                let trailing_words = words(trailing_tokens);
                let is_supported_flashback_tail = trailing_words
                    == [
                        "its",
                        "flashback",
                        "cost",
                        "is",
                        "equal",
                        "to",
                        "its",
                        "mana",
                        "cost",
                    ];
                if !is_supported_flashback_tail {
                    return Ok(None);
                }
                extract_grant_spec_from_subject(
                    subject_tokens,
                    crate::grant::Grantable::flashback_from_cards_mana_cost(),
                )?
            }
            ["escape"] => {
                let Some(exile_count) = parse_granted_escape_cost_tail(trailing_tokens)? else {
                    return Ok(None);
                };
                extract_grant_spec_from_subject(
                    subject_tokens,
                    crate::grant::Grantable::escape(exile_count),
                )?
            }
            _ => None,
        };

        let Some(spec) = spec else {
            return Ok(None);
        };

        let mut ability = StaticAbilityAst::Static(StaticAbility::grants(spec));
        if let Some(condition) = condition {
            ability = StaticAbilityAst::ConditionalStaticAbility {
                ability: Box::new(ability),
                condition,
            };
        }
        Ok(Some(vec![ability]))
    }

    let clause_words = words(tokens);
    if !clause_words
        .iter()
        .any(|word| *word == "have" || *word == "has")
    {
        return Ok(None);
    }

    let have_token_idx = tokens
        .iter()
        .rposition(|token| token.is_word("have") || token.is_word("has"))
        .ok_or_else(|| CardTextError::ParseError("missing granted-keyword verb".to_string()))?;
    if words(&tokens[..have_token_idx])
        .iter()
        .any(|word| *word == "get" || *word == "gets")
    {
        return Ok(None);
    }

    if words_start_with(tokens, &["as", "long", "as"]) {
        let trailing_has = tokens[have_token_idx + 1..]
            .iter()
            .any(|token| token.is_word("have") || token.is_word("has"));
        let trailing_get_or_be = tokens[have_token_idx + 1..].iter().any(|token| {
            token.is_word("get")
                || token.is_word("gets")
                || token.is_word("is")
                || token.is_word("are")
        });
        if !trailing_has && trailing_get_or_be {
            return Ok(None);
        }
    }

    let (prefix_condition, subject_start) =
        match parse_anthem_prefix_condition(tokens, have_token_idx) {
            Ok(parsed) => parsed,
            Err(_) => return Ok(None),
        };
    let subject_tokens = trim_commas(&tokens[subject_start..have_token_idx]);
    if subject_tokens.is_empty() {
        return Ok(None);
    }

    let subject_words = words(&subject_tokens);
    if subject_words.contains(&"equipped")
        || subject_words.contains(&"enchanted")
        || subject_words.contains(&"mana")
    {
        return Ok(None);
    }
    if subject_words.iter().any(|word| {
        matches!(
            *word,
            "can"
                | "cant"
                | "cannot"
                | "attack"
                | "attacks"
                | "block"
                | "blocks"
                | "blocked"
                | "blocking"
                | "cast"
                | "spell"
                | "spells"
                | "during"
                | "until"
                | "unless"
                | "when"
                | "whenever"
                | "if"
                | "though"
        )
    }) {
        return Ok(None);
    }

    let tail_tokens = trim_commas(&tokens[have_token_idx + 1..]);
    if tail_tokens.is_empty() {
        return Ok(None);
    }

    let mut tail_tokens = tail_tokens;
    let mut trailing_clause_tokens: Vec<Token> = Vec::new();
    if let Some(period_idx) = tail_tokens
        .iter()
        .position(|token| matches!(token, Token::Period(_)))
    {
        let leading = trim_commas(&tail_tokens[..period_idx]);
        let trailing = trim_commas(&tail_tokens[period_idx + 1..]);
        trailing_clause_tokens = trailing.to_vec();
        tail_tokens = leading;
    }

    let mut keyword_tokens = tail_tokens.clone();
    let mut suffix_condition = None;
    if let Some(idx) = words(&tail_tokens)
        .windows(3)
        .position(|window| window == ["as", "long", "as"])
    {
        if idx + 3 >= tail_tokens.len() {
            return Err(CardTextError::ParseError(format!(
                "missing condition after trailing 'as long as' clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        keyword_tokens = trim_commas(&tail_tokens[..idx]);
        suffix_condition = Some(parse_static_condition_clause(&tail_tokens[idx + 3..])?);
    }
    if keyword_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing granted keyword list (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let mut grants_must_attack = false;
    let keyword_words = words(&keyword_tokens);
    if let Some(and_idx) = keyword_words
        .windows(6)
        .position(|window| window == ["and", "attack", "each", "combat", "if", "able"])
        .or_else(|| {
            keyword_words
                .windows(6)
                .position(|window| window == ["and", "attacks", "each", "combat", "if", "able"])
        })
    {
        keyword_tokens = trim_commas(&keyword_tokens[..and_idx]);
        grants_must_attack = true;
    }
    if keyword_tokens.is_empty() {
        return Ok(None);
    }

    let condition = match (prefix_condition, suffix_condition) {
        (Some(_), Some(_)) => {
            return Err(CardTextError::ParseError(format!(
                "multiple static conditions are not supported in granted-keyword clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        (Some(cond), None) | (None, Some(cond)) => Some(cond),
        (None, None) => None,
    };

    if !trailing_clause_tokens.is_empty() {
        if let Some(compiled) = parse_granted_alternative_cast_static(
            &subject_tokens,
            &keyword_tokens,
            &trailing_clause_tokens,
            condition.clone(),
        )? {
            return Ok(Some(compiled));
        }

        return Err(CardTextError::ParseError(format!(
            "unsupported trailing granted-keyword clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let Some(actions) = parse_ability_line(&keyword_tokens) else {
        return Ok(None);
    };
    reject_unimplemented_keyword_actions(&actions, &clause_words.join(" "))?;
    if actions.is_empty() {
        return Ok(None);
    }

    let mapped = actions
        .into_iter()
        .filter(|action| action.lowers_to_static_ability())
        .collect::<Vec<_>>();
    if mapped.is_empty() && !grants_must_attack {
        return Ok(None);
    }

    let subject = parse_anthem_subject(&subject_tokens)?;
    let mut compiled = Vec::new();
    if grants_must_attack {
        match &subject {
            AnthemSubjectAst::Source => {
                if let Some(condition) = &condition {
                    compiled.push(StaticAbilityAst::ConditionalStaticAbility {
                        ability: Box::new(StaticAbilityAst::Static(StaticAbility::must_attack())),
                        condition: condition.clone(),
                    });
                } else {
                    compiled.push(StaticAbilityAst::Static(StaticAbility::must_attack()));
                }
            }
            AnthemSubjectAst::Filter(filter) => {
                compiled.push(StaticAbilityAst::GrantStaticAbility {
                    filter: filter.clone(),
                    ability: Box::new(StaticAbilityAst::Static(StaticAbility::must_attack())),
                    condition: condition.clone(),
                })
            }
        }
    }
    for action in mapped {
        let ast = match &subject {
            AnthemSubjectAst::Source => match &condition {
                Some(condition) => StaticAbilityAst::ConditionalKeywordAction {
                    action,
                    condition: condition.clone(),
                },
                None => StaticAbilityAst::KeywordAction(action),
            },
            AnthemSubjectAst::Filter(filter) => StaticAbilityAst::GrantKeywordAction {
                filter: filter.clone(),
                action,
                condition: condition.clone(),
            },
        };
        compiled.push(ast);
    }
    Ok(Some(compiled))
}

pub(crate) fn parse_all_creatures_lose_flying_line(
    tokens: &[Token],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let words = words(tokens);
    if words.as_slice() == ["all", "creatures", "lose", "flying"] {
        return Ok(Some(StaticAbilityAst::RemoveKeywordAction {
            filter: ObjectFilter::creature(),
            action: KeywordAction::Flying,
        }));
    }
    Ok(None)
}

pub(crate) fn parse_each_creature_cant_be_blocked_by_more_than_line(
    tokens: &[Token],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    // Familiar Ground: "Each creature can't be blocked by more than one creature."
    let clause_words = words(tokens);
    if clause_words.len() < 10 {
        return Ok(None);
    }
    let (subject_len, you_control) = if clause_words.starts_with(&[
        "each", "creature", "you", "control", "cant", "be", "blocked", "by", "more", "than",
    ]) {
        (4usize, true)
    } else if clause_words.starts_with(&[
        "each", "creature", "cant", "be", "blocked", "by", "more", "than",
    ]) {
        (2usize, false)
    } else {
        return Ok(None);
    };

    // "Each creature (you control) can't be blocked by more than <N> creature(s)"
    let amount_word_idx = subject_len + 6;
    let Some(amount_token_idx) = token_index_for_word_index(tokens, amount_word_idx) else {
        return Ok(None);
    };
    let Some((amount, used)) = parse_number(&tokens[amount_token_idx..]) else {
        return Ok(None);
    };

    // Expect "... creature(s)" after the number.
    let rest_tokens = &tokens[amount_token_idx + used..];
    let rest_words = words(rest_tokens);
    if rest_words
        .first()
        .is_some_and(|w| *w == "creature" || *w == "creatures")
    {
        let filter = if you_control {
            ObjectFilter::creature().you_control()
        } else {
            ObjectFilter::creature()
        };
        let granted = StaticAbility::cant_be_blocked_by_more_than(amount as usize);
        return Ok(Some(StaticAbilityAst::GrantStaticAbility {
            filter,
            ability: Box::new(StaticAbilityAst::Static(granted)),
            condition: None,
        }));
    }

    Ok(None)
}

pub(crate) fn parse_each_creature_can_block_additional_creature_each_combat_line(
    tokens: &[Token],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    // High Ground: "Each creature can block an additional creature each combat."
    let clause_words = words(tokens);
    if clause_words.len() < 9 {
        return Ok(None);
    }
    let (_subject_len, you_control) =
        if clause_words.starts_with(&["each", "creature", "you", "control", "can", "block"]) {
            (4usize, true)
        } else if clause_words.starts_with(&["each", "creature", "can", "block"]) {
            (2usize, false)
        } else {
            return Ok(None);
        };
    if !clause_words.ends_with(&["each", "combat"]) {
        return Ok(None);
    }
    let Some(additional_word_idx) = clause_words.iter().position(|w| *w == "additional") else {
        return Ok(None);
    };
    if additional_word_idx == 0 {
        return Ok(None);
    }

    let mut additional = 1usize;
    let prev = clause_words[additional_word_idx - 1];
    if prev != "an" {
        if let Some(prev_token_idx) = token_index_for_word_index(tokens, additional_word_idx - 1)
            && let Some((count, used)) = parse_number(&tokens[prev_token_idx..])
            && used > 0
        {
            additional = count as usize;
        }
    }

    let filter = if you_control {
        ObjectFilter::creature().you_control()
    } else {
        ObjectFilter::creature()
    };
    let granted = StaticAbility::can_block_additional_creature_each_combat(additional);
    Ok(Some(StaticAbilityAst::GrantStaticAbility {
        filter,
        ability: Box::new(StaticAbilityAst::Static(granted)),
        condition: None,
    }))
}

pub(crate) fn parse_lose_all_abilities_and_transform_base_pt_line(
    tokens: &[Token],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    fn title_case_words(words: &[&str]) -> String {
        words
            .iter()
            .map(|word| {
                let mut chars = word.chars();
                if let Some(first) = chars.next() {
                    let mut out = String::new();
                    out.extend(first.to_uppercase());
                    out.push_str(chars.as_str());
                    out
                } else {
                    String::new()
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    let words = words(tokens);
    if words.len() < 8 {
        return Ok(None);
    }

    let Some(is_idx) = words
        .iter()
        .position(|word| *word == "is" || *word == "are")
    else {
        return Ok(None);
    };
    let Some(with_idx) = words
        .windows(5)
        .position(|window| window == ["with", "base", "power", "and", "toughness"])
    else {
        return Ok(None);
    };
    if with_idx <= is_idx {
        return Ok(None);
    }

    let Some(pt_word) = words.get(with_idx + 5) else {
        return Ok(None);
    };
    let (power, toughness) = parse_pt_modifier(pt_word).map_err(|_| {
        CardTextError::ParseError(format!(
            "invalid base power/toughness value (clause: '{}')",
            words.join(" ")
        ))
    })?;

    let has_lose_all = words.windows(3).any(|window| {
        matches!(
            window,
            ["lose", "all", "abilities"] | ["loses", "all", "abilities"]
        )
    });
    if !has_lose_all {
        return Ok(None);
    }

    let subject_end = is_idx.min(
        words
            .iter()
            .position(|word| *word == "lose" || *word == "loses")
            .unwrap_or(is_idx),
    );
    if subject_end == 0 {
        return Ok(None);
    }
    let subject_tokens = trim_commas(&tokens[..subject_end]);
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    let filter = parse_object_filter(&subject_tokens, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported subject in lose-all-abilities transform clause (clause: '{}')",
            words.join(" ")
        ))
    })?;

    let mut descriptor_words = words[is_idx + 1..with_idx]
        .iter()
        .copied()
        .filter(|word| !is_article(word) && *word != "and")
        .collect::<Vec<_>>();
    if descriptor_words.is_empty() {
        return Ok(None);
    }
    if descriptor_words.first().copied() == Some("all") {
        descriptor_words.remove(0);
    }
    if descriptor_words.is_empty() {
        return Ok(None);
    }

    let mut set_colors = ColorSet::new();
    let mut set_card_types: Vec<CardType> = Vec::new();
    let mut creature_subtypes: Vec<Subtype> = Vec::new();

    for descriptor in descriptor_words {
        if let Some(color) = parse_color(descriptor) {
            set_colors = set_colors.union(color);
            continue;
        }
        if let Some(card_type) = parse_card_type(descriptor) {
            if !set_card_types.contains(&card_type) {
                set_card_types.push(card_type);
            }
            continue;
        }
        if let Some(subtype) = parse_subtype_word(descriptor)
            .or_else(|| descriptor.strip_suffix('s').and_then(parse_subtype_word))
        {
            if !creature_subtypes.contains(&subtype) {
                creature_subtypes.push(subtype);
            }
            continue;
        }
        return Err(CardTextError::ParseError(format!(
            "unsupported transform descriptor '{}' (clause: '{}')",
            descriptor,
            words.join(" ")
        )));
    }

    if !creature_subtypes.is_empty() && !set_card_types.contains(&CardType::Creature) {
        set_card_types.push(CardType::Creature);
    }

    let mut set_name: Option<String> = None;
    let tail_words = &words[with_idx + 6..];
    if let Some(named_idx) = tail_words.iter().position(|word| *word == "named") {
        let end_idx = (named_idx + 1..tail_words.len())
            .find(|idx| {
                matches!(
                    tail_words[*idx],
                    "and" | "lose" | "loses" | "with" | "it" | "that" | "those" | "this"
                )
            })
            .unwrap_or(tail_words.len());
        if end_idx > named_idx + 1 {
            set_name = Some(title_case_words(&tail_words[named_idx + 1..end_idx]));
        }
    }

    let has_except_mana = words
        .windows(3)
        .any(|window| window == ["except", "mana", "abilities"]);
    let mut abilities = vec![if has_except_mana {
        StaticAbility::remove_all_abilities_except_mana(filter.clone())
    } else {
        StaticAbility::remove_all_abilities(filter.clone())
    }];

    if !set_card_types.is_empty() {
        abilities.push(StaticAbility::set_card_types(
            filter.clone(),
            set_card_types,
        ));
    }
    if !creature_subtypes.is_empty() {
        abilities.push(StaticAbility::set_creature_subtypes(
            filter.clone(),
            creature_subtypes,
        ));
    }
    if !set_colors.is_empty() {
        abilities.push(StaticAbility::set_colors(filter.clone(), set_colors));
    }
    if let Some(name) = set_name {
        abilities.push(StaticAbility::set_name(filter.clone(), name));
    }
    abilities.push(StaticAbility::set_base_power_toughness(
        filter, power, toughness,
    ));

    Ok(Some(abilities))
}

pub(crate) fn parse_lose_all_abilities_and_base_pt_line(
    tokens: &[Token],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let words = words(tokens);
    let lose_idx = words
        .iter()
        .position(|word| *word == "lose" || *word == "loses");
    let Some(lose_idx) = lose_idx else {
        return Ok(None);
    };

    if !words[lose_idx + 1..]
        .windows(2)
        .any(|window| window == ["all", "abilities"])
    {
        return Ok(None);
    }
    if words.contains(&"becomes") {
        return Err(CardTextError::ParseError(format!(
            "unsupported lose-all-abilities static becomes clause (clause: '{}')",
            words.join(" ")
        )));
    }

    let subject_tokens = &tokens[..lose_idx];
    let filter = parse_object_filter(subject_tokens, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported subject in lose-all-abilities clause (clause: '{}')",
            words.join(" ")
        ))
    })?;

    let has_except_mana = words
        .windows(3)
        .any(|window| window == ["except", "mana", "abilities"]);
    let mut abilities = vec![if has_except_mana {
        StaticAbility::remove_all_abilities_except_mana(filter.clone())
    } else {
        StaticAbility::remove_all_abilities(filter.clone())
    }];

    let have_idx = words
        .iter()
        .position(|word| *word == "have" || *word == "has");
    if let Some(have_idx) = have_idx {
        let after_have = &words[have_idx + 1..];
        if after_have.starts_with(&["base", "power", "and", "toughness"])
            && let Some(modifier_token) = after_have.iter().find(|word| word.contains('/'))
            && let Ok((power, toughness)) = parse_pt_modifier(modifier_token)
        {
            abilities.push(StaticAbility::set_base_power_toughness(
                filter, power, toughness,
            ));
        }
    }

    Ok(Some(abilities))
}

pub(crate) fn parse_all_have_indestructible_line(
    tokens: &[Token],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let words = words(tokens);
    let have_idx = words
        .iter()
        .position(|word| *word == "have" || *word == "has");
    let Some(have_idx) = have_idx else {
        return Ok(None);
    };
    if words[..have_idx]
        .iter()
        .any(|word| *word == "get" || *word == "gets")
    {
        return Ok(None);
    }

    let have_token_idx = tokens
        .iter()
        .position(|token| token.is_word("have") || token.is_word("has"))
        .ok_or_else(|| CardTextError::ParseError("missing granted-keyword verb".to_string()))?;
    let tail = trim_commas(&tokens[have_token_idx + 1..]);
    let Some(actions) = parse_ability_line(&tail) else {
        return Ok(None);
    };
    reject_unimplemented_keyword_actions(&actions, &words.join(" "))?;
    if actions.len() != 1
        || !actions
            .first()
            .is_some_and(|action| matches!(action, KeywordAction::Indestructible))
    {
        return Ok(None);
    }

    let filter = parse_object_filter(&tokens[..have_token_idx], false)?;
    Ok(Some(StaticAbilityAst::GrantKeywordAction {
        filter,
        action: KeywordAction::Indestructible,
        condition: None,
    }))
}

#[derive(Debug, Clone)]
pub(crate) enum AnthemSubjectAst {
    Source,
    Filter(ObjectFilter),
}

#[derive(Debug, Clone)]
pub(crate) struct ParsedAnthemClause {
    pub(crate) subject: AnthemSubjectAst,
    pub(crate) power: AnthemValue,
    pub(crate) toughness: AnthemValue,
    pub(crate) condition: Option<crate::ConditionExpr>,
}

pub(crate) fn words_start_with(tokens: &[Token], expected: &[&str]) -> bool {
    words(tokens).starts_with(expected)
}

pub(crate) fn find_source_reference_start(tokens: &[Token]) -> Option<usize> {
    let mut token_indices = Vec::new();
    let mut token_words = Vec::new();
    for (idx, token) in tokens.iter().enumerate() {
        if let Some(word) = token.as_word() {
            token_indices.push(idx);
            token_words.push(word);
        }
    }

    for word_start in 0..token_words.len() {
        if is_source_reference_words(&token_words[word_start..]) {
            return token_indices.get(word_start).copied();
        }
    }
    None
}

pub(crate) fn object_filter_specificity_score(filter: &ObjectFilter) -> usize {
    let mut score = 0usize;
    score += filter.tagged_constraints.len() * 20;
    score += filter.card_types.len() * 10;
    score += filter.all_card_types.len() * 10;
    score += filter.subtypes.len() * 8;
    score += filter.excluded_subtypes.len() * 8;
    score += usize::from(filter.controller.is_some()) * 6;
    score += usize::from(filter.owner.is_some()) * 6;
    score += usize::from(filter.zone.is_some()) * 4;
    score += usize::from(filter.other) * 3;
    score += usize::from(filter.token || filter.nontoken) * 3;
    score += usize::from(filter.tapped || filter.untapped) * 2;
    score += usize::from(
        filter.attacking
            || filter.nonattacking
            || filter.blocking
            || filter.nonblocking
            || filter.blocked
            || filter.unblocked,
    ) * 2;
    score += usize::from(filter.is_commander || filter.noncommander) * 2;
    score += usize::from(filter.colorless || filter.multicolored || filter.monocolored) * 2;
    score += usize::from(filter.with_counter.is_some() || filter.without_counter.is_some()) * 4;
    score += usize::from(filter.entered_battlefield_this_turn) * 2;
    score += usize::from(filter.entered_battlefield_controller.is_some()) * 2;
    score += usize::from(filter.was_dealt_damage_this_turn) * 2;
    score += usize::from(!filter.excluded_card_types.is_empty()) * 2;
    score += usize::from(!filter.excluded_supertypes.is_empty()) * 2;
    score += usize::from(!filter.excluded_colors.is_empty()) * 2;
    score += usize::from(!filter.excluded_static_abilities.is_empty()) * 2;
    score += usize::from(!filter.excluded_ability_markers.is_empty()) * 2;
    score += usize::from(filter.colors.is_some()) * 2;
    score += usize::from(filter.power.is_some() || filter.toughness.is_some()) * 2;
    score
}

pub(crate) fn parse_best_object_filter_suffix(tokens: &[Token]) -> Option<ObjectFilter> {
    let mut best: Option<(usize, ObjectFilter)> = None;
    for start in 0..tokens.len() {
        if tokens[start].as_word().is_none() {
            continue;
        }
        let mut other = false;
        let mut candidate = &tokens[start..];
        if candidate
            .first()
            .is_some_and(|token| token.is_word("other") || token.is_word("another"))
        {
            other = true;
            candidate = &candidate[1..];
        }
        if candidate.is_empty() {
            continue;
        }
        let candidate_words = words(candidate);
        if matches!(candidate_words.as_slice(), ["it"] | ["them"]) {
            continue;
        }
        let Ok(filter) = parse_object_filter(candidate, other) else {
            continue;
        };
        let score = object_filter_specificity_score(&filter);
        if best
            .as_ref()
            .is_none_or(|(best_score, _)| score > *best_score)
        {
            best = Some((score, filter));
        }
    }
    best.map(|(_, filter)| filter)
}

fn subject_branch_looks_type_like(filter: &ObjectFilter) -> bool {
    !filter.card_types.is_empty()
        || !filter.subtypes.is_empty()
        || !filter.excluded_card_types.is_empty()
        || !filter.excluded_subtypes.is_empty()
}

fn parse_shared_suffix_and_subject_filter(tokens: &[Token]) -> Option<ObjectFilter> {
    let mut best: Option<(usize, ObjectFilter)> = None;

    for (and_idx, token) in tokens.iter().enumerate() {
        if !token.is_word("and") {
            continue;
        }

        let left_branch = trim_commas(&tokens[..and_idx]);
        let right_tail = trim_commas(&tokens[and_idx + 1..]);
        if left_branch.is_empty() || right_tail.len() < 2 {
            continue;
        }

        let Ok(left_branch_filter) = parse_object_filter(&left_branch, false) else {
            continue;
        };
        if !subject_branch_looks_type_like(&left_branch_filter) {
            continue;
        }

        for split_idx in 1..right_tail.len() {
            let right_branch = trim_commas(&right_tail[..split_idx]);
            let shared_suffix = trim_commas(&right_tail[split_idx..]);
            if right_branch.is_empty() || shared_suffix.is_empty() {
                continue;
            }

            let Some(shared_head) = shared_suffix.first().and_then(Token::as_word) else {
                continue;
            };
            if !matches!(
                shared_head,
                "you"
                    | "your"
                    | "that"
                    | "those"
                    | "with"
                    | "without"
                    | "named"
                    | "in"
                    | "from"
                    | "on"
                    | "among"
                    | "under"
                    | "during"
            ) {
                continue;
            }

            let Ok(right_branch_filter) = parse_object_filter(&right_branch, false) else {
                continue;
            };
            if !subject_branch_looks_type_like(&right_branch_filter) {
                continue;
            }

            let mut left_full = left_branch.clone();
            left_full.extend(shared_suffix.iter().cloned());
            let mut right_full = right_branch.clone();
            right_full.extend(shared_suffix.iter().cloned());

            let Ok(left_filter) = parse_object_filter(&left_full, false) else {
                continue;
            };
            let Ok(right_filter) = parse_object_filter(&right_full, false) else {
                continue;
            };
            if left_filter == right_filter {
                continue;
            }

            let mut disjunction = ObjectFilter::default();
            disjunction.any_of = vec![left_filter.clone(), right_filter.clone()];
            let score = object_filter_specificity_score(&left_filter)
                + object_filter_specificity_score(&right_filter)
                + shared_suffix.len();
            if best
                .as_ref()
                .is_none_or(|(best_score, _)| score > *best_score)
            {
                best = Some((score, disjunction));
            }
        }
    }

    best.map(|(_, filter)| filter)
}

pub(crate) fn parse_anthem_subject(tokens: &[Token]) -> Result<AnthemSubjectAst, CardTextError> {
    let subject_words = words(tokens);
    if subject_words.as_slice() == ["it"] {
        return Ok(AnthemSubjectAst::Source);
    }
    if find_source_reference_start(tokens).is_some() {
        return Ok(AnthemSubjectAst::Source);
    }
    if let Some(filter) = parse_shared_suffix_and_subject_filter(tokens) {
        return Ok(AnthemSubjectAst::Filter(filter));
    }
    parse_best_object_filter_suffix(tokens)
        .map(AnthemSubjectAst::Filter)
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported anthem subject (clause: '{}')",
                words(tokens).join(" ")
            ))
        })
}

pub(crate) fn parse_static_quantity_prefix(
    tokens: &[Token],
    allow_default_one: bool,
) -> Result<(crate::effect::Comparison, usize), CardTextError> {
    if tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "missing quantity in static condition".to_string(),
        ));
    }

    if tokens[0].is_word("no") {
        return Ok((crate::effect::Comparison::LessThanOrEqual(0), 1));
    }

    if tokens[0].is_word("exactly") {
        let (value, used) = parse_number(tokens.get(1..).unwrap_or_default()).ok_or_else(|| {
            CardTextError::ParseError("missing quantity in static condition".to_string())
        })?;
        return Ok((crate::effect::Comparison::Equal(value as i32), used + 1));
    }

    if (tokens[0].is_word("fewer") || tokens[0].is_word("less"))
        && tokens.get(1).is_some_and(|token| token.is_word("than"))
    {
        let (value, used) = parse_number(tokens.get(2..).unwrap_or_default()).ok_or_else(|| {
            CardTextError::ParseError("missing quantity in static condition".to_string())
        })?;
        return Ok((crate::effect::Comparison::LessThan(value as i32), used + 2));
    }

    if (tokens[0].is_word("more") || tokens[0].is_word("greater"))
        && tokens.get(1).is_some_and(|token| token.is_word("than"))
    {
        let (value, used) = parse_number(tokens.get(2..).unwrap_or_default()).ok_or_else(|| {
            CardTextError::ParseError("missing quantity in static condition".to_string())
        })?;
        return Ok((
            crate::effect::Comparison::GreaterThan(value as i32),
            used + 2,
        ));
    }

    if let Some((value, used)) = parse_number(tokens) {
        let value = value as i32;
        let first_word = tokens.first().and_then(Token::as_word);
        if matches!(first_word, Some("a" | "an")) {
            return Ok((crate::effect::Comparison::GreaterThanOrEqual(1), used));
        }
        if tokens.get(used).is_some_and(|token| token.is_word("or"))
            && tokens
                .get(used + 1)
                .is_some_and(|token| token.is_word("more") || token.is_word("greater"))
        {
            return Ok((
                crate::effect::Comparison::GreaterThanOrEqual(value),
                used + 2,
            ));
        }
        if tokens.get(used).is_some_and(|token| token.is_word("or"))
            && tokens
                .get(used + 1)
                .is_some_and(|token| token.is_word("less") || token.is_word("fewer"))
        {
            return Ok((crate::effect::Comparison::LessThanOrEqual(value), used + 2));
        }
        return Ok((crate::effect::Comparison::Equal(value), used));
    }

    if allow_default_one {
        return Ok((crate::effect::Comparison::GreaterThanOrEqual(1), 0));
    }

    Err(CardTextError::ParseError(
        "missing quantity in static condition".to_string(),
    ))
}

pub(crate) fn parse_permanent_card_count_filter(tokens: &[Token]) -> Option<ObjectFilter> {
    let token_words = words(tokens);
    if !token_words.starts_with(&["permanent", "card"])
        && !token_words.starts_with(&["permanent", "cards"])
    {
        return None;
    }

    let mut filter = ObjectFilter::default();
    filter.card_types = vec![
        CardType::Artifact,
        CardType::Creature,
        CardType::Enchantment,
        CardType::Land,
        CardType::Planeswalker,
        CardType::Battle,
    ];

    for (idx, word) in token_words.iter().enumerate() {
        if let Some(zone) = parse_zone_word(word) {
            filter.zone = Some(zone);
            if idx > 0 {
                match token_words[idx - 1] {
                    "your" => filter.owner = Some(PlayerFilter::You),
                    "opponent" | "opponents" => filter.owner = Some(PlayerFilter::Opponent),
                    _ => {}
                }
            }
        }
    }

    filter.zone.map(|_| filter)
}

pub(crate) fn parse_static_condition_clause(
    tokens: &[Token],
) -> Result<crate::ConditionExpr, CardTextError> {
    let tokens = trim_edge_punctuation(tokens);
    let clause_words = words(&tokens);
    if clause_words.is_empty() {
        return Err(CardTextError::ParseError(
            "missing condition clause after 'as long as'".to_string(),
        ));
    }

    if let Some(condition) = parse_cards_in_hand_static_condition(&tokens) {
        return Ok(condition);
    }

    if clause_words == ["this", "creature", "is", "equipped"]
        || clause_words == ["this", "is", "equipped"]
        || clause_words == ["it", "is", "equipped"]
        || clause_words == ["its", "equipped"]
    {
        return Ok(crate::ConditionExpr::SourceIsEquipped);
    }
    if clause_words == ["this", "creature", "is", "enchanted"]
        || clause_words == ["this", "is", "enchanted"]
        || clause_words == ["it", "is", "enchanted"]
        || clause_words == ["its", "enchanted"]
    {
        return Ok(crate::ConditionExpr::SourceIsEnchanted);
    }
    if clause_words == ["this", "creature", "is", "untapped"]
        || clause_words == ["this", "is", "untapped"]
        || clause_words == ["it", "is", "untapped"]
        || clause_words == ["its", "untapped"]
    {
        return Ok(crate::ConditionExpr::SourceIsUntapped);
    }
    if clause_words == ["this", "creature", "is", "tapped"]
        || clause_words == ["this", "permanent", "is", "tapped"]
        || clause_words == ["this", "is", "tapped"]
        || clause_words == ["it", "is", "tapped"]
        || clause_words == ["its", "tapped"]
    {
        return Ok(crate::ConditionExpr::SourceIsTapped);
    }
    if clause_words == ["this", "is", "paired", "with", "another", "creature"]
        || clause_words
            == [
                "this", "creature", "is", "paired", "with", "another", "creature",
            ]
        || clause_words == ["it", "is", "paired", "with", "another", "creature"]
    {
        return Ok(crate::ConditionExpr::SourceIsSoulbondPaired);
    }
    if clause_words == ["enchanted", "permanent", "is", "a", "creature"]
        || clause_words == ["enchanted", "permanent", "is", "creature"]
    {
        return Ok(crate::ConditionExpr::EnchantedPermanentIsCreature);
    }
    if clause_words == ["enchanted", "permanent", "is", "an", "equipment"]
        || clause_words == ["enchanted", "permanent", "is", "a", "equipment"]
        || clause_words == ["enchanted", "permanent", "is", "equipment"]
    {
        return Ok(crate::ConditionExpr::EnchantedPermanentIsEquipment);
    }
    if clause_words == ["enchanted", "permanent", "is", "a", "vehicle"]
        || clause_words == ["enchanted", "permanent", "is", "vehicle"]
    {
        return Ok(crate::ConditionExpr::EnchantedPermanentIsVehicle);
    }
    if clause_words == ["equipped", "creature", "is", "tapped"] {
        return Ok(crate::ConditionExpr::EquippedCreatureTapped);
    }
    if clause_words == ["equipped", "creature", "is", "untapped"] {
        return Ok(crate::ConditionExpr::EquippedCreatureUntapped);
    }
    if clause_words == ["it", "is", "attacking"]
        || clause_words == ["its", "attacking"]
        || clause_words == ["this", "creature", "is", "attacking"]
        || clause_words == ["this", "permanent", "is", "attacking"]
    {
        return Ok(crate::ConditionExpr::SourceIsAttacking);
    }
    if clause_words == ["it", "is", "your", "turn"] || clause_words == ["its", "your", "turn"] {
        return Ok(crate::ConditionExpr::YourTurn);
    }
    if clause_words == ["it", "is", "not", "your", "turn"]
        || clause_words == ["its", "not", "your", "turn"]
    {
        return Ok(crate::ConditionExpr::Not(Box::new(
            crate::ConditionExpr::YourTurn,
        )));
    }

    if let Some(is_idx) = clause_words
        .iter()
        .position(|word| *word == "is" || *word == "are")
    {
        let subject_words = &clause_words[..is_idx];
        let source_pronoun_subject = matches!(subject_words, ["it"] | ["its"]);
        if !subject_words.is_empty()
            && (is_source_reference_words(subject_words) || source_pronoun_subject)
        {
            let remainder_words = &clause_words[is_idx + 1..];
            if remainder_words == ["in", "your", "graveyard"]
                || remainder_words == ["in", "graveyard"]
            {
                let mut filter = ObjectFilter::source();
                filter.zone = Some(Zone::Graveyard);
                return Ok(crate::ConditionExpr::CountComparison {
                    count: AnthemCountExpression::MatchingFilter(filter),
                    comparison: crate::effect::Comparison::GreaterThanOrEqual(1),
                    display: Some(clause_words.join(" ")),
                });
            }
        }
    }

    if clause_words.starts_with(&["there", "are"]) || clause_words.starts_with(&["there", "is"]) {
        if let Some((metric, threshold)) = parse_graveyard_metric_threshold_condition(&tokens)? {
            if metric == crate::static_abilities::GraveyardCountMetric::CardTypes {
                return Ok(crate::ConditionExpr::PlayerHasCardTypesInGraveyardOrMore {
                    player: PlayerFilter::You,
                    count: threshold,
                });
            }
        }

        let quantified = &tokens[2..];
        let (comparison, used) = parse_static_quantity_prefix(quantified, false)?;
        let mut filter_tokens = &quantified[used..];
        if filter_tokens
            .first()
            .is_some_and(|token| token.is_word("card") || token.is_word("cards"))
        {
            filter_tokens = &filter_tokens[1..];
        }
        if filter_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing object phrase in static condition (clause: '{}')",
                clause_words.join(" ")
            )));
        }

        let filter = parse_permanent_card_count_filter(filter_tokens)
            .or_else(|| parse_object_filter(filter_tokens, false).ok())
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported counted object phrase in static condition (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?;
        return Ok(crate::ConditionExpr::CountComparison {
            count: AnthemCountExpression::MatchingFilter(filter),
            comparison,
            display: Some(clause_words.join(" ")),
        });
    }

    let control_prefix_len = if clause_words.starts_with(&["you", "control"])
        || clause_words.starts_with(&["you", "controls"])
        || clause_words.starts_with(&["opponent", "control"])
        || clause_words.starts_with(&["opponent", "controls"])
        || clause_words.starts_with(&["opponents", "control"])
        || clause_words.starts_with(&["opponents", "controls"])
    {
        2
    } else if clause_words.starts_with(&["an", "opponent", "control"])
        || clause_words.starts_with(&["an", "opponent", "controls"])
        || clause_words.starts_with(&["your", "opponents", "control"])
        || clause_words.starts_with(&["your", "opponents", "controls"])
    {
        3
    } else {
        0
    };
    if control_prefix_len > 0 {
        let quantified = &tokens[control_prefix_len..];
        let (comparison, used) = parse_static_quantity_prefix(quantified, true)?;
        let mut filter_tokens: Vec<Token> = tokens[..control_prefix_len].to_vec();
        filter_tokens.extend_from_slice(&quantified[used..]);
        let filter = parse_object_filter(&filter_tokens, false).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported control condition filter (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
        return Ok(crate::ConditionExpr::CountComparison {
            count: AnthemCountExpression::MatchingFilter(filter),
            comparison,
            display: Some(clause_words.join(" ")),
        });
    }

    let own_prefix_len = if clause_words.starts_with(&["you", "own"])
        || clause_words.starts_with(&["you", "owns"])
        || clause_words.starts_with(&["opponent", "own"])
        || clause_words.starts_with(&["opponent", "owns"])
        || clause_words.starts_with(&["opponents", "own"])
        || clause_words.starts_with(&["opponents", "owns"])
    {
        2
    } else if clause_words.starts_with(&["an", "opponent", "own"])
        || clause_words.starts_with(&["an", "opponent", "owns"])
        || clause_words.starts_with(&["your", "opponents", "own"])
        || clause_words.starts_with(&["your", "opponents", "owns"])
    {
        3
    } else {
        0
    };
    if own_prefix_len > 0 {
        let quantified = &tokens[own_prefix_len..];
        let (comparison, used) = parse_static_quantity_prefix(quantified, true)?;
        let mut filter_tokens: Vec<Token> = tokens[..own_prefix_len].to_vec();
        filter_tokens.extend_from_slice(&quantified[used..]);
        let filter = parse_object_filter(&filter_tokens, false).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported ownership condition filter (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
        return Ok(crate::ConditionExpr::CountComparison {
            count: AnthemCountExpression::MatchingFilter(filter),
            comparison,
            display: Some(clause_words.join(" ")),
        });
    }

    if clause_words.as_slice() == ["you", "have", "citys", "blessing"]
        || clause_words.as_slice() == ["you", "have", "city", "blessing"]
        || clause_words.as_slice() == ["you", "have", "the", "citys", "blessing"]
        || clause_words.as_slice() == ["you", "have", "the", "city", "blessing"]
    {
        return Ok(crate::ConditionExpr::PlayerHasCitysBlessing {
            player: PlayerFilter::You,
        });
    }

    let has_counter_on_source = clause_words.windows(2).any(|window| {
        matches!(
            window,
            ["on", "it"] | ["on", "this"] | ["on", "him"] | ["on", "her"]
        )
    });
    if has_counter_on_source
        && let Some(has_idx) = clause_words
            .iter()
            .position(|word| *word == "has" || *word == "have")
    {
        let subject_words = &clause_words[..has_idx];
        let source_pronoun_subject = matches!(subject_words, ["it"] | ["its"]);
        if !subject_words.is_empty()
            && (is_source_reference_words(subject_words) || source_pronoun_subject)
        {
            let quantified = &tokens[has_idx + 1..];
            let (comparison, used) = parse_static_quantity_prefix(quantified, true)?;
            let counter_tokens = &quantified[used..];
            let counter_words = words(counter_tokens);
            let Some(counter_word_idx) = counter_words
                .iter()
                .position(|word| *word == "counter" || *word == "counters")
            else {
                return Err(CardTextError::ParseError(format!(
                    "missing counter phrase in static condition (clause: '{}')",
                    clause_words.join(" ")
                )));
            };

            let counter_type = if counter_word_idx > 0 {
                parse_counter_type_word(counter_words[counter_word_idx - 1])
            } else {
                None
            };

            let tail = &counter_words[counter_word_idx + 1..];
            let on_source_tail = tail.starts_with(&["on", "it"])
                || tail.starts_with(&["on", "this"])
                || tail.starts_with(&["on", "him"])
                || tail.starts_with(&["on", "her"]);
            if !on_source_tail {
                return Err(CardTextError::ParseError(format!(
                    "unsupported source-counter condition tail (clause: '{}')",
                    clause_words.join(" ")
                )));
            }

            let mut filter = ObjectFilter::source();
            filter.with_counter = Some(match counter_type {
                Some(counter_type) => crate::filter::CounterConstraint::Typed(counter_type),
                None => crate::filter::CounterConstraint::Any,
            });
            return Ok(crate::ConditionExpr::CountComparison {
                count: AnthemCountExpression::MatchingFilter(filter),
                comparison,
                display: Some(clause_words.join(" ")),
            });
        }
    }

    if let Some(conjoined) = parse_conjoined_static_condition_clause(&tokens) {
        return Ok(conjoined);
    }

    Err(CardTextError::ParseError(format!(
        "unsupported static condition clause (clause: '{}')",
        clause_words.join(" ")
    )))
}

fn parse_conjoined_static_condition_clause(tokens: &[Token]) -> Option<crate::ConditionExpr> {
    let words = words(tokens);
    let and_positions = words
        .iter()
        .enumerate()
        .filter_map(|(idx, word)| (*word == "and").then_some(idx))
        .collect::<Vec<_>>();
    for and_word_idx in and_positions {
        let and_token_idx = token_index_for_word_index(tokens, and_word_idx)?;
        let left_tokens = trim_commas(&tokens[..and_token_idx]);
        let right_tokens = trim_commas(&tokens[and_token_idx + 1..]);
        if left_tokens.is_empty() || right_tokens.is_empty() {
            continue;
        }
        let Ok(left) = parse_static_condition_clause(&left_tokens) else {
            continue;
        };
        let right = parse_conjoined_static_condition_clause(&right_tokens)
            .or_else(|| parse_static_condition_clause(&right_tokens).ok());
        if let Some(right) = right {
            return Some(crate::ConditionExpr::And(Box::new(left), Box::new(right)));
        }
    }
    None
}

fn parse_cards_in_hand_static_condition(tokens: &[Token]) -> Option<crate::ConditionExpr> {
    let clause_words = words(tokens);
    let (player, count_start_idx) = match clause_words.as_slice() {
        ["you", "have", ..] => (PlayerFilter::You, 2usize),
        ["that", "player", "has", ..] => (PlayerFilter::Target(Box::new(PlayerFilter::Any)), 3),
        ["an", "opponent", "has", ..] => (PlayerFilter::Opponent, 3usize),
        ["opponent", "has", ..] => (PlayerFilter::Opponent, 2usize),
        _ => return None,
    };

    let count_tokens = tokens.get(count_start_idx..)?;
    let (count, used) = parse_number(count_tokens)?;
    let tail_tokens = count_tokens.get(used..)?;
    let tail_words = words(tail_tokens);
    if tail_words.as_slice() == ["or", "more", "cards", "in", "hand"]
        || tail_words.as_slice() == ["or", "more", "card", "in", "hand"]
    {
        return Some(crate::ConditionExpr::PlayerCardsInHandOrMore {
            player,
            count: count as i32,
        });
    }
    if tail_words.as_slice() == ["or", "fewer", "cards", "in", "hand"]
        || tail_words.as_slice() == ["or", "fewer", "card", "in", "hand"]
    {
        return Some(crate::ConditionExpr::PlayerCardsInHandOrFewer {
            player,
            count: count as i32,
        });
    }
    None
}

pub(crate) fn parse_anthem_for_each_expression(
    tokens: &[Token],
) -> Result<AnthemCountExpression, CardTextError> {
    let tokens = trim_edge_punctuation(tokens);
    if !words_start_with(&tokens, &["for", "each"]) {
        return Err(CardTextError::ParseError(
            "missing 'for each' in anthem scaling clause".to_string(),
        ));
    }
    let rest = &tokens[2..];
    if rest.is_empty() {
        return Err(CardTextError::ParseError(
            "missing object phrase after 'for each'".to_string(),
        ));
    }

    if words_start_with(rest, &["basic", "land", "type", "among"]) {
        let filter_tokens = &rest[4..];
        let filter = parse_object_filter(filter_tokens, false).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported domain count filter (clause: '{}')",
                words(&tokens).join(" ")
            ))
        })?;
        return Ok(AnthemCountExpression::BasicLandTypesAmong(filter));
    }

    if let Some(attached_idx) = rest.iter().position(|token| token.is_word("attached")) {
        let filter_tokens = &rest[..attached_idx];
        let tail_words = words(&rest[attached_idx + 1..]);
        let attached_to_source = tail_words == ["to", "it"]
            || tail_words == ["to", "this", "creature"]
            || tail_words == ["to", "this", "permanent"];
        if attached_to_source {
            let filter = parse_object_filter(filter_tokens, false).map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported attached-object filter in anthem scaling clause (clause: '{}')",
                    words(&tokens).join(" ")
                ))
            })?;
            return Ok(AnthemCountExpression::AttachedToSource(filter));
        }
    }

    let filter = parse_object_filter(rest, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported 'for each' filter in anthem clause (clause: '{}')",
            words(&tokens).join(" ")
        ))
    })?;
    Ok(AnthemCountExpression::MatchingFilter(filter))
}

pub(crate) fn parse_anthem_prefix_condition(
    tokens: &[Token],
    get_idx: usize,
) -> Result<(Option<crate::ConditionExpr>, usize), CardTextError> {
    if words_start_with(tokens, &["during", "turns", "other", "than", "yours"]) {
        let subject_start = tokens[..get_idx]
            .iter()
            .position(|token| matches!(token, Token::Comma(_)))
            .map(|idx| idx + 1)
            .or_else(|| find_source_reference_start(&tokens[..get_idx]))
            .unwrap_or(5);
        return Ok((
            Some(crate::ConditionExpr::Not(Box::new(
                crate::ConditionExpr::YourTurn,
            ))),
            subject_start,
        ));
    }
    if words_start_with(tokens, &["during", "your", "turn"]) {
        let subject_start = tokens[..get_idx]
            .iter()
            .position(|token| matches!(token, Token::Comma(_)))
            .map(|idx| idx + 1)
            .or_else(|| find_source_reference_start(&tokens[..get_idx]))
            .unwrap_or(3);
        return Ok((Some(crate::ConditionExpr::YourTurn), subject_start));
    }

    if words_start_with(tokens, &["as", "long", "as"]) {
        let subject_start = tokens[..get_idx]
            .iter()
            .position(|token| matches!(token, Token::Comma(_)))
            .map(|idx| idx + 1)
            .or_else(|| infer_as_long_as_subject_start(tokens, get_idx))
            .or_else(|| find_source_reference_start(&tokens[..get_idx]))
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing subject boundary in leading static condition clause (clause: '{}')",
                    words(tokens).join(" ")
                ))
            })?;
        if subject_start <= 3 {
            return Err(CardTextError::ParseError(format!(
                "missing condition after leading 'as long as' clause (clause: '{}')",
                words(tokens).join(" ")
            )));
        }
        let condition_tokens = trim_commas(&tokens[3..subject_start]);
        let condition = parse_static_condition_clause(&condition_tokens)?;
        return Ok((Some(condition), subject_start));
    }

    Ok((None, 0))
}

fn infer_as_long_as_subject_start(tokens: &[Token], action_idx: usize) -> Option<usize> {
    if action_idx <= 3 {
        return None;
    }

    let mut word_token_indices = Vec::new();
    for (idx, token) in tokens.iter().enumerate() {
        if token.as_word().is_some() {
            word_token_indices.push(idx);
        }
    }
    if word_token_indices.is_empty() {
        return None;
    }

    let action_word_idx = word_token_indices
        .iter()
        .position(|idx| *idx == action_idx)
        .unwrap_or(word_token_indices.len());
    if action_word_idx <= 3 {
        return None;
    }

    for subject_word_idx in 4..action_word_idx {
        let subject_start = word_token_indices[subject_word_idx];
        let condition_tokens = trim_commas(&tokens[3..subject_start]);
        if condition_tokens.is_empty() {
            continue;
        }
        if parse_static_condition_clause(&condition_tokens).is_err() {
            continue;
        }

        let subject_tokens = trim_commas(&tokens[subject_start..action_idx]);
        if subject_tokens.is_empty() {
            continue;
        }
        if parse_anthem_subject(&subject_tokens).is_ok() {
            return Some(subject_start);
        }
    }

    None
}

pub(crate) fn parse_anthem_clause(
    tokens: &[Token],
    get_idx: usize,
    tail_end: usize,
) -> Result<ParsedAnthemClause, CardTextError> {
    let (prefix_condition, subject_start) = parse_anthem_prefix_condition(tokens, get_idx)?;
    let subject_tokens = trim_commas(&tokens[subject_start..get_idx]);
    if subject_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing anthem subject (clause: '{}')",
            words(tokens).join(" ")
        )));
    }
    let subject = parse_anthem_subject(&subject_tokens)?;

    let mut modifier_idx = get_idx + 1;
    if tokens
        .get(modifier_idx)
        .is_some_and(|token| token.is_word("a") || token.is_word("an"))
        && tokens
            .get(modifier_idx + 1)
            .is_some_and(|token| token.is_word("additional"))
    {
        modifier_idx += 2;
    }

    let modifier_token = tokens
        .get(modifier_idx)
        .and_then(Token::as_word)
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing power/toughness modifier in anthem clause (clause: '{}')",
                words(tokens).join(" ")
            ))
        })?;
    let (raw_power, raw_toughness) = parse_pt_modifier_values(modifier_token).map_err(|_| {
        CardTextError::ParseError(format!(
            "invalid power/toughness modifier in anthem clause (clause: '{}')",
            words(tokens).join(" ")
        ))
    })?;

    let tail_tokens = trim_edge_punctuation(&tokens[modifier_idx + 1..tail_end]);
    let mut scale: Option<AnthemCountExpression> = None;
    let mut suffix_condition: Option<crate::ConditionExpr> = None;
    if !tail_tokens.is_empty() {
        if words_start_with(&tail_tokens, &["for", "each"]) {
            scale = Some(parse_anthem_for_each_expression(&tail_tokens)?);
        } else if words_start_with(&tail_tokens, &["where", "x", "is"]) {
            let x_value = parse_where_x_value_clause(&tail_tokens).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported where-x anthem clause (clause: '{}')",
                    words(tokens).join(" ")
                ))
            })?;
            scale = Some(match x_value {
                Value::Count(filter) => AnthemCountExpression::MatchingFilter(filter),
                Value::BasicLandTypesAmong(filter) => {
                    AnthemCountExpression::BasicLandTypesAmong(filter)
                }
                _ => {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported where-x anthem value (clause: '{}')",
                        words(tokens).join(" ")
                    )));
                }
            });
        } else if words_start_with(&tail_tokens, &["as", "long", "as"]) {
            suffix_condition = Some(parse_static_condition_clause(&tail_tokens[3..])?);
        } else {
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing anthem clause (clause: '{}')",
                words(tokens).join(" ")
            )));
        }
    }

    let condition = match (prefix_condition, suffix_condition) {
        (Some(_prefix), Some(_)) => {
            return Err(CardTextError::ParseError(format!(
                "multiple anthem conditions are not supported (clause: '{}')",
                words(tokens).join(" ")
            )));
        }
        (Some(condition), None) | (None, Some(condition)) => Some(condition),
        (None, None) => None,
    };

    let resolve_anthem_value = |component: Value,
                                scale_expr: Option<&AnthemCountExpression>|
     -> Result<AnthemValue, CardTextError> {
        match component {
            Value::Fixed(value) => Ok(match scale_expr {
                Some(scale_expr) => AnthemValue::scaled(value, scale_expr.clone()),
                None => AnthemValue::Fixed(value),
            }),
            Value::X => {
                if let Some(scale_expr) = scale_expr {
                    Ok(AnthemValue::scaled(1, scale_expr.clone()))
                } else {
                    Err(CardTextError::ParseError(format!(
                        "unsupported X power/toughness modifier without count expression (clause: '{}')",
                        words(tokens).join(" ")
                    )))
                }
            }
            Value::XTimes(multiplier) => {
                if let Some(scale_expr) = scale_expr {
                    Ok(AnthemValue::scaled(multiplier, scale_expr.clone()))
                } else {
                    Err(CardTextError::ParseError(format!(
                        "unsupported X power/toughness modifier without count expression (clause: '{}')",
                        words(tokens).join(" ")
                    )))
                }
            }
            _ => Err(CardTextError::ParseError(format!(
                "invalid power/toughness modifier in anthem clause (clause: '{}')",
                words(tokens).join(" ")
            ))),
        }
    };

    let power = resolve_anthem_value(raw_power, scale.as_ref())?;
    let toughness = resolve_anthem_value(raw_toughness, scale.as_ref())?;

    parser_trace_stack("parse_static:anthem-clause:matched", tokens);
    Ok(ParsedAnthemClause {
        subject,
        power,
        toughness,
        condition,
    })
}

pub(crate) fn build_anthem_static_ability(clause: &ParsedAnthemClause) -> StaticAbility {
    let mut anthem = match &clause.subject {
        AnthemSubjectAst::Source => Anthem::for_source(0, 0),
        AnthemSubjectAst::Filter(filter) => Anthem::new(filter.clone(), 0, 0),
    }
    .with_values(clause.power.clone(), clause.toughness.clone());

    if let Some(condition) = &clause.condition {
        anthem = anthem.with_condition(condition.clone());
    }

    StaticAbility::new(anthem)
}

#[derive(Debug)]
pub(crate) struct TypeColorAdditionClause {
    pub(crate) added_colors: ColorSet,
    pub(crate) set_colors: ColorSet,
    pub(crate) card_types: Vec<CardType>,
    pub(crate) subtypes: Vec<Subtype>,
}

pub(crate) fn parse_type_color_addition_clause(
    tokens: &[Token],
) -> Result<Option<TypeColorAdditionClause>, CardTextError> {
    let words = words(tokens);
    if words.len() < 7 || words.first() != Some(&"is") {
        return Ok(None);
    }

    let Some(addition_idx) = words
        .windows(5)
        .position(|window| window == ["in", "addition", "to", "its", "other"])
    else {
        return Ok(None);
    };
    if addition_idx <= 1 {
        return Ok(None);
    }

    let scope_words = &words[addition_idx + 5..];
    let mut allow_colors = false;
    let mut allow_types = false;
    let mut segment_start = 0usize;
    for idx in 0..=scope_words.len() {
        let is_boundary = idx == scope_words.len() || scope_words[idx] == "and";
        if !is_boundary {
            continue;
        }
        if segment_start == idx {
            segment_start = idx + 1;
            continue;
        }
        let segment = &scope_words[segment_start..idx];
        segment_start = idx + 1;
        if segment.len() == 1 && (segment[0] == "color" || segment[0] == "colors") {
            allow_colors = true;
            continue;
        }
        if matches!(segment.last().copied(), Some("type" | "types"))
            && segment[..segment.len() - 1]
                .iter()
                .all(|word| is_type_scope_qualifier_word(word))
        {
            allow_types = true;
            continue;
        }
        return Err(CardTextError::ParseError(format!(
            "unsupported in-addition scope in type/color clause (clause: '{}')",
            words.join(" ")
        )));
    }
    if !allow_colors && !allow_types {
        return Ok(None);
    }

    let descriptor_words = words[1..addition_idx]
        .iter()
        .copied()
        .filter(|word| !is_article(word) && *word != "and")
        .collect::<Vec<_>>();
    if descriptor_words.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing type/color descriptors in in-addition clause (clause: '{}')",
            words.join(" ")
        )));
    }

    let mut added_colors = ColorSet::new();
    let mut set_colors = ColorSet::new();
    let mut card_types = Vec::new();
    let mut subtypes = Vec::new();
    for descriptor in descriptor_words {
        if let Some(color) = parse_color(descriptor) {
            if allow_colors {
                added_colors = added_colors.union(color);
            } else if allow_types {
                // "is black Zombie in addition to its other creature types"
                // sets color while only preserving existing types.
                set_colors = set_colors.union(color);
            } else {
                return Err(CardTextError::ParseError(format!(
                    "color descriptor '{}' not allowed by in-addition scope (clause: '{}')",
                    descriptor,
                    words.join(" ")
                )));
            }
            continue;
        }

        if let Some(card_type) = parse_card_type(descriptor) {
            if allow_types {
                if !card_types.contains(&card_type) {
                    card_types.push(card_type);
                }
                continue;
            }
            return Err(CardTextError::ParseError(format!(
                "card type descriptor '{}' not allowed by in-addition scope (clause: '{}')",
                descriptor,
                words.join(" ")
            )));
        }

        if let Some(subtype) = parse_subtype_word(descriptor)
            .or_else(|| descriptor.strip_suffix('s').and_then(parse_subtype_word))
        {
            if allow_types {
                if !subtypes.contains(&subtype) {
                    subtypes.push(subtype);
                }
                continue;
            }
            return Err(CardTextError::ParseError(format!(
                "subtype descriptor '{}' not allowed by in-addition scope (clause: '{}')",
                descriptor,
                words.join(" ")
            )));
        }

        return Err(CardTextError::ParseError(format!(
            "unsupported descriptor '{}' in type/color addition clause (clause: '{}')",
            descriptor,
            words.join(" ")
        )));
    }

    if added_colors.is_empty()
        && set_colors.is_empty()
        && card_types.is_empty()
        && subtypes.is_empty()
    {
        return Err(CardTextError::ParseError(format!(
            "missing type/color additions in in-addition clause (clause: '{}')",
            words.join(" ")
        )));
    }

    Ok(Some(TypeColorAdditionClause {
        added_colors,
        set_colors,
        card_types,
        subtypes,
    }))
}

pub(crate) fn is_type_scope_qualifier_word(word: &str) -> bool {
    parse_card_type(word).is_some()
        || matches!(
            word,
            "card" | "creature" | "permanent" | "basic" | "legendary" | "snow" | "nonbasic"
        )
}

pub(crate) fn parse_soulbond_shared_line(
    tokens: &[Token],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let clause_words = words(tokens);
    if !clause_words.starts_with(&["as", "long", "as"]) {
        return Ok(None);
    }

    let Some(paired_word_idx) = clause_words
        .windows(5)
        .enumerate()
        .find_map(|(idx, window)| {
            (idx >= 3 && window == ["is", "paired", "with", "another", "creature"]).then_some(idx)
        })
    else {
        return Ok(None);
    };

    let subject_words = &clause_words[3..paired_word_idx];
    if subject_words.is_empty() {
        return Ok(None);
    }

    let source_like_subject = is_source_reference_words(subject_words)
        || matches!(subject_words, ["this"] | ["this", "creature"])
        || !subject_words.iter().any(|word| {
            matches!(
                *word,
                "enchanted" | "equipped" | "target" | "another" | "each" | "those"
            )
        });
    if !source_like_subject {
        return Ok(None);
    }

    let prefix_word_len = paired_word_idx + 5;
    let prefix_len = token_index_for_word_index(tokens, prefix_word_len).unwrap_or(tokens.len());

    let rest = trim_commas(&tokens[prefix_len..]);
    if rest.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing soulbond shared effect clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let rest_words = words(&rest);
    let pt_modifier_idx = if rest_words.starts_with(&["both", "creatures", "get"]) {
        Some(3usize)
    } else if rest_words.starts_with(&["each", "of", "those", "creatures", "gets"]) {
        Some(5usize)
    } else {
        None
    };
    if let Some(modifier_idx) = pt_modifier_idx {
        let modifier = *rest_words.get(modifier_idx).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing power/toughness modifier in soulbond clause (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
        let (power, toughness) = parse_pt_modifier(modifier).map_err(|_| {
            CardTextError::ParseError(format!(
                "invalid power/toughness modifier in soulbond clause (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
        return Ok(Some(vec![
            StaticAbility::soulbond_shared_power_toughness(power, toughness).into(),
        ]));
    }

    let ability_start = if rest_words.starts_with(&["both", "creatures", "have"]) {
        Some(3usize)
    } else if rest_words.starts_with(&["each", "of", "those", "creatures", "has"]) {
        Some(5usize)
    } else {
        None
    };
    if let Some(ability_start) = ability_start {
        let mut ability_tokens = trim_commas(&rest[ability_start..]);
        ability_tokens = trim_edge_punctuation(&ability_tokens);
        if ability_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing shared ability in soulbond clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }

        if let Some(actions) = parse_ability_line(&ability_tokens) {
            reject_unimplemented_keyword_actions(&actions, &clause_words.join(" "))?;
            let abilities: Vec<StaticAbility> = actions
                .into_iter()
                .filter_map(keyword_action_to_static_ability)
                .collect();
            if abilities.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "unsupported shared ability in soulbond clause (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            let shared = abilities
                .into_iter()
                .map(StaticAbility::soulbond_shared_ability)
                .map(StaticAbilityAst::from)
                .collect();
            return Ok(Some(shared));
        }

        if let Some(GrantedAbilityAst::ParsedObjectAbility { ability, display }) =
            parse_granted_activated_or_triggered_ability_for_gain(&ability_tokens, &clause_words)?
        {
            return Ok(Some(vec![StaticAbilityAst::SoulbondSharedObjectAbility {
                ability,
                display,
            }]));
        }

        return Err(CardTextError::ParseError(format!(
            "unsupported shared ability in soulbond clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    Err(CardTextError::ParseError(format!(
        "unsupported soulbond shared clause (clause: '{}')",
        clause_words.join(" ")
    )))
}

pub(crate) fn parse_anthem_and_type_color_addition_line(
    tokens: &[Token],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let words = words(tokens);
    if contains_until_end_of_turn(&words) {
        return Ok(None);
    }

    let get_idx = tokens
        .iter()
        .position(|token| token.is_word("get") || token.is_word("gets"));
    let Some(get_idx) = get_idx else {
        return Ok(None);
    };

    let and_idx = tokens
        .iter()
        .enumerate()
        .find_map(|(idx, token)| (idx > get_idx && token.is_word("and")).then_some(idx));
    let Some(and_idx) = and_idx else {
        return Ok(None);
    };

    let addition_tokens = &tokens[and_idx + 1..];
    let Some(additions) = parse_type_color_addition_clause(addition_tokens)? else {
        return Ok(None);
    };

    let clause = parse_anthem_clause(tokens, get_idx, and_idx)?;
    let AnthemSubjectAst::Filter(filter) = &clause.subject else {
        return Err(CardTextError::ParseError(format!(
            "unsupported source-only type/color addition clause (clause: '{}')",
            words.join(" ")
        )));
    };

    let mut result = vec![build_anthem_static_ability(&clause)];
    if !additions.set_colors.is_empty() {
        result.push(StaticAbility::set_colors(
            filter.clone(),
            additions.set_colors,
        ));
    }
    if !additions.added_colors.is_empty() {
        result.push(StaticAbility::add_colors(
            filter.clone(),
            additions.added_colors,
        ));
    }
    if !additions.card_types.is_empty() {
        result.push(StaticAbility::add_card_types(
            filter.clone(),
            additions.card_types,
        ));
    }
    if !additions.subtypes.is_empty() {
        result.push(StaticAbility::add_subtypes(
            filter.clone(),
            additions.subtypes,
        ));
    }
    Ok(Some(result))
}

pub(crate) fn parse_anthem_and_keyword_line(
    tokens: &[Token],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let clause_words = words(tokens);
    let get_idx = tokens
        .iter()
        .position(|token| token.is_word("get") || token.is_word("gets"));
    let have_idx = clause_words
        .iter()
        .position(|word| *word == "have" || *word == "has");

    let (Some(get_idx), Some(have_idx)) = (get_idx, have_idx) else {
        return Ok(None);
    };

    if have_idx <= get_idx {
        return Ok(None);
    }

    let have_token_idx = tokens.iter().enumerate().find_map(|(idx, token)| {
        (idx > get_idx && (token.is_word("have") || token.is_word("has"))).then_some(idx)
    });
    let Some(have_token_idx) = have_token_idx else {
        return Ok(None);
    };

    let pre_grant_words = words(&tokens[..have_token_idx]);
    // "until end of turn" in the pump clause indicates a one-shot effect.
    // Ignore timing text that appears only inside a quoted granted ability.
    if contains_until_end_of_turn(&pre_grant_words) {
        return Ok(None);
    }

    let mut ability_tokens = trim_commas(&tokens[have_token_idx + 1..]);
    let mut trailing_condition: Option<crate::ConditionExpr> = None;
    if let Some(as_long_idx) = words(&ability_tokens)
        .windows(3)
        .position(|window| window == ["as", "long", "as"])
    {
        let as_token_idx =
            token_index_for_word_index(&ability_tokens, as_long_idx).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unable to map trailing 'as long as' keyword condition (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?;
        let condition_start_idx = token_index_for_word_index(&ability_tokens, as_long_idx + 3)
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing condition after trailing 'as long as' keyword clause (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?;
        let ability_head = trim_commas(&ability_tokens[..as_token_idx]);
        if ability_head.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing granted keyword list before trailing condition (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let condition_tokens = trim_commas(&ability_tokens[condition_start_idx..]);
        if condition_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing condition after trailing 'as long as' keyword clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        trailing_condition = Some(parse_static_condition_clause(&condition_tokens)?);
        ability_tokens = ability_head;
    }

    let mut keyword_actions: Vec<KeywordAction> = Vec::new();
    let mut granted_activated_ability: Option<ParsedAbility> = None;
    let mut granted_activated_display: Option<String> = None;

    let and_has_idx = (0..ability_tokens.len().saturating_sub(1)).find(|idx| {
        ability_tokens[*idx].is_word("and")
            && (ability_tokens[*idx + 1].is_word("has") || ability_tokens[*idx + 1].is_word("have"))
    });
    if let Some(and_has_idx) = and_has_idx {
        let keyword_tokens = trim_commas(&ability_tokens[..and_has_idx]);
        if !keyword_tokens.is_empty() {
            if let Some(actions) = parse_ability_line(&keyword_tokens) {
                reject_unimplemented_keyword_actions(&actions, &clause_words.join(" "))?;
                keyword_actions.extend(
                    actions
                        .into_iter()
                        .filter(|action| action.lowers_to_static_ability()),
                );
            } else {
                return Ok(None);
            }
        }

        let ability_tail_tokens = trim_commas(&ability_tokens[and_has_idx + 2..]);
        if !ability_tail_tokens.is_empty() {
            let has_colon = ability_tail_tokens
                .iter()
                .any(|token| matches!(token, Token::Colon(_)));
            let Some(parsed) = parse_activated_line(&ability_tail_tokens)? else {
                if has_colon {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported granted activated ability in anthem clause (clause: '{}')",
                        clause_words.join(" ")
                    )));
                }
                return Ok(None);
            };
            let display = words(&ability_tail_tokens).join(" ");
            granted_activated_display = Some(display);
            granted_activated_ability = Some(parsed);
        }
    } else if let Some(actions) = parse_ability_line(&ability_tokens) {
        reject_unimplemented_keyword_actions(&actions, &clause_words.join(" "))?;
        keyword_actions = actions
            .into_iter()
            .filter(|action| action.lowers_to_static_ability())
            .collect();
    } else {
        return Ok(None);
    }

    if keyword_actions.is_empty() && granted_activated_ability.is_none() {
        return Ok(None);
    }

    let clause_tail_end = if have_token_idx > get_idx + 2
        && tokens
            .get(have_token_idx - 1)
            .is_some_and(|token| token.is_word("and"))
    {
        have_token_idx - 1
    } else {
        have_token_idx
    };
    let mut clause = parse_anthem_clause(tokens, get_idx, clause_tail_end)?;
    if let Some(condition) = trailing_condition {
        if clause.condition.is_some() {
            return Err(CardTextError::ParseError(format!(
                "multiple anthem conditions are not supported (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        clause.condition = Some(condition);
    }
    let mut result = vec![build_anthem_static_ability(&clause).into()];
    for action in keyword_actions {
        result.push(grant_keyword_action_for_anthem_subject(&clause, action));
    }

    if let Some(ability) = granted_activated_ability {
        result.push(grant_object_ability_for_anthem_subject(
            &clause,
            ability,
            granted_activated_display.unwrap_or_else(|| clause_words.join(" ")),
        ));
    }

    Ok(Some(result))
}

pub(crate) fn parse_protection_from_colored_spells_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = words(tokens);
    if !matches!(
        clause_words.as_slice(),
        [
            "protection",
            "from",
            "spells",
            "that",
            "are",
            "one",
            "or",
            "more",
            "colors"
        ]
    ) {
        return Ok(None);
    }

    let all_colors = crate::color::ColorSet::WHITE
        .union(crate::color::ColorSet::BLUE)
        .union(crate::color::ColorSet::BLACK)
        .union(crate::color::ColorSet::RED)
        .union(crate::color::ColorSet::GREEN);
    let mut filter = ObjectFilter::spell();
    filter.colors = Some(all_colors);
    Ok(Some(StaticAbility::protection(
        crate::ability::ProtectionFrom::Permanents(filter),
    )))
}

fn grant_for_anthem_subject(
    clause: &ParsedAnthemClause,
    ability: StaticAbility,
) -> StaticAbilityAst {
    match &clause.subject {
        AnthemSubjectAst::Source => match &clause.condition {
            Some(condition) => StaticAbilityAst::ConditionalStaticAbility {
                ability: Box::new(StaticAbilityAst::Static(ability)),
                condition: condition.clone(),
            },
            None => StaticAbilityAst::Static(ability),
        },
        AnthemSubjectAst::Filter(filter) => StaticAbilityAst::GrantStaticAbility {
            filter: filter.clone(),
            ability: Box::new(StaticAbilityAst::Static(ability)),
            condition: clause.condition.clone(),
        },
    }
}

fn grant_keyword_action_for_anthem_subject(
    clause: &ParsedAnthemClause,
    action: KeywordAction,
) -> StaticAbilityAst {
    match &clause.subject {
        AnthemSubjectAst::Source => match &clause.condition {
            Some(condition) => StaticAbilityAst::ConditionalKeywordAction {
                action,
                condition: condition.clone(),
            },
            None => StaticAbilityAst::KeywordAction(action),
        },
        AnthemSubjectAst::Filter(filter) => StaticAbilityAst::GrantKeywordAction {
            filter: filter.clone(),
            action,
            condition: clause.condition.clone(),
        },
    }
}

fn anthem_subject_filter(subject: &AnthemSubjectAst) -> ObjectFilter {
    match subject {
        AnthemSubjectAst::Source => ObjectFilter::source(),
        AnthemSubjectAst::Filter(filter) => filter.clone(),
    }
}

fn grant_object_ability_for_anthem_subject(
    clause: &ParsedAnthemClause,
    ability: ParsedAbility,
    display: String,
) -> StaticAbilityAst {
    StaticAbilityAst::GrantObjectAbility {
        filter: anthem_subject_filter(&clause.subject),
        ability,
        display,
        condition: clause.condition.clone(),
    }
}

fn parsed_ability_from_ability(ability: Ability) -> ParsedAbility {
    ParsedAbility {
        ability,
        effects_ast: None,
        reference_imports: ReferenceImports::default(),
        trigger_spec: None,
    }
}

fn parse_triggered_granted_ability(
    tokens: &[Token],
) -> Result<Option<ParsedAbility>, CardTextError> {
    let trigger_tokens = trim_edge_punctuation(tokens);
    if trigger_tokens.is_empty() {
        return Ok(None);
    }
    if !trigger_tokens
        .first()
        .is_some_and(|token| token.is_word("when") || token.is_word("whenever"))
        && !is_at_trigger_intro(&trigger_tokens, 0)
    {
        return Ok(None);
    }

    let ability = match parse_triggered_line(&trigger_tokens)? {
        LineAst::Triggered {
            trigger,
            effects,
            max_triggers_per_turn,
        } => parsed_triggered_ability(
            trigger,
            effects,
            vec![Zone::Battlefield],
            Some(words(&trigger_tokens).join(" ")),
            max_triggers_per_turn.map(crate::ConditionExpr::MaxTimesEachTurn),
            ReferenceImports::default(),
        ),
        _ => return Ok(None),
    };
    if parsed_triggered_ability_is_empty(&ability) {
        return Err(CardTextError::ParseError(format!(
            "unsupported empty triggered granted ability clause (clause: '{}')",
            words(&trigger_tokens).join(" ")
        )));
    }
    Ok(Some(ability))
}

fn split_anthem_trailing_segments_preserving_granted_abilities(
    tokens: &[Token],
) -> Vec<Vec<Token>> {
    let mut segments = Vec::new();
    let mut current = Vec::new();
    let mut preserve_commas = false;

    for token in tokens {
        if matches!(token, Token::Comma(_)) && !preserve_commas {
            if !current.is_empty() {
                segments.push(std::mem::take(&mut current));
            }
            continue;
        }

        current.push(token.clone());
        let trimmed = trim_commas(&current);
        let segment_words = words(&trimmed);
        preserve_commas = trimmed.iter().any(|token| matches!(token, Token::Colon(_)))
            || segment_words.first().is_some_and(|word| {
                matches!(*word, "when" | "whenever")
                    || (*word == "at" && segment_words.get(1).copied() == Some("the"))
            })
            || (segment_words.first().copied() == Some("and")
                && segment_words.get(1).is_some_and(|word| {
                    matches!(*word, "when" | "whenever")
                        || (*word == "at" && segment_words.get(2).copied() == Some("the"))
                }));
    }

    if !current.is_empty() {
        segments.push(current);
    }

    segments
}

fn parsed_triggered_ability_is_empty(ability: &ParsedAbility) -> bool {
    matches!(
        &ability.ability.kind,
        AbilityKind::Triggered(triggered)
            if triggered.effects.is_empty()
                && ability
                    .effects_ast
                    .as_ref()
                    .is_none_or(|effects| effects.is_empty())
    )
}

pub(crate) fn parse_anthem_with_trailing_segments_line(
    tokens: &[Token],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let clause_words = words(tokens);
    if contains_until_end_of_turn(&clause_words) {
        return Ok(None);
    }

    let Some(get_idx) = tokens
        .iter()
        .position(|token| token.is_word("get") || token.is_word("gets"))
    else {
        return Ok(None);
    };

    let mut work_tokens = tokens.to_vec();
    if work_tokens
        .get(get_idx + 1)
        .is_some_and(|token| token.is_word("a") || token.is_word("an"))
        && work_tokens
            .get(get_idx + 2)
            .is_some_and(|token| token.is_word("additional"))
    {
        work_tokens.drain(get_idx + 1..get_idx + 3);
    }

    let Some(pt_word) = work_tokens.get(get_idx + 1).and_then(Token::as_word) else {
        return Ok(None);
    };
    if parse_pt_modifier(pt_word).is_err() {
        return Ok(None);
    }

    let clause = parse_anthem_clause(&work_tokens, get_idx, get_idx + 2)?;
    let tail_tokens = trim_commas(&work_tokens[get_idx + 2..]);
    if tail_tokens.is_empty() {
        return Ok(None);
    }

    let mut extras: Vec<StaticAbilityAst> = Vec::new();
    for raw_segment in split_anthem_trailing_segments_preserving_granted_abilities(&tail_tokens) {
        let mut segment = trim_commas(&raw_segment).to_vec();
        while segment.first().is_some_and(|token| token.is_word("and")) {
            segment = trim_commas(&segment[1..]).to_vec();
        }
        if segment.is_empty() {
            continue;
        }

        let segment_words = normalize_cant_words(&segment);
        if segment_words.as_slice() == ["cant", "block"] {
            extras.push(grant_for_anthem_subject(&clause, StaticAbility::cant_block()).into());
            continue;
        }
        if segment_words.as_slice() == ["attacks", "each", "combat", "if", "able"]
            || segment_words.as_slice() == ["attack", "each", "combat", "if", "able"]
        {
            extras.push(grant_for_anthem_subject(&clause, StaticAbility::must_attack()).into());
            continue;
        }
        if segment_words.starts_with(&["cant", "be", "blocked", "by", "more", "than"]) {
            let count_tokens = &segment[6..];
            let Some((count, used)) = parse_number(count_tokens) else {
                return Ok(None);
            };
            let tail = normalize_cant_words(&count_tokens[used..]);
            if tail.as_slice() != ["creature"] && tail.as_slice() != ["creatures"] {
                return Ok(None);
            }
            extras.push(
                grant_for_anthem_subject(
                    &clause,
                    StaticAbility::cant_be_blocked_by_more_than(count as usize),
                )
                .into(),
            );
            continue;
        }
        if segment_words.len() == 2 && segment_words[0] == "is" {
            let Some(color) = parse_color(segment_words[1]) else {
                return Ok(None);
            };
            let filter = match &clause.subject {
                AnthemSubjectAst::Source => ObjectFilter::source(),
                AnthemSubjectAst::Filter(filter) => filter.clone(),
            };
            let mut set_colors = crate::static_abilities::SetColorsForFilter::new(filter, color);
            if let Some(condition) = &clause.condition {
                set_colors = set_colors.with_condition(condition.clone());
            }
            extras.push(StaticAbility::new(set_colors).into());
            continue;
        }

        if segment_words
            .first()
            .is_some_and(|word| *word == "lose" || *word == "loses")
        {
            let ability_tokens = trim_commas(&segment[1..]);
            if ability_tokens.is_empty() {
                return Ok(None);
            }
            let Some(actions) = parse_ability_line(&ability_tokens) else {
                return Ok(None);
            };
            reject_unimplemented_keyword_actions(&actions, &clause_words.join(" "))?;
            let removed = actions
                .into_iter()
                .filter_map(keyword_action_to_static_ability)
                .collect::<Vec<_>>();
            if removed.is_empty() {
                return Ok(None);
            }
            for ability in removed {
                extras.push(match &clause.subject {
                    AnthemSubjectAst::Source => StaticAbilityAst::RemoveStaticAbility {
                        filter: ObjectFilter::source(),
                        ability: Box::new(StaticAbilityAst::Static(ability)),
                    },
                    AnthemSubjectAst::Filter(filter) => StaticAbilityAst::GrantStaticAbility {
                        filter: filter.clone(),
                        ability: Box::new(StaticAbilityAst::RemoveStaticAbility {
                            filter: ObjectFilter::source(),
                            ability: Box::new(StaticAbilityAst::Static(ability)),
                        }),
                        condition: clause.condition.clone(),
                    },
                });
            }
            continue;
        }

        if segment_words
            .first()
            .is_some_and(|word| *word == "has" || *word == "have")
        {
            let mut ability_tokens = trim_commas(&segment[1..]).to_vec();
            if ability_tokens.is_empty() {
                return Ok(None);
            }

            let mut grant_must_attack = false;
            let ability_words = normalize_cant_words(&ability_tokens);
            if let Some(and_idx) = ability_words.windows(6).position(|window| {
                window == ["and", "attacks", "each", "combat", "if", "able"]
                    || window == ["and", "attack", "each", "combat", "if", "able"]
            }) {
                let Some(and_token_idx) = token_index_for_word_index(&ability_tokens, and_idx)
                else {
                    return Ok(None);
                };
                let head = trim_commas(&ability_tokens[..and_token_idx]);
                if head.is_empty() {
                    return Ok(None);
                }
                ability_tokens = head.to_vec();
                grant_must_attack = true;
            }

            let mut granted_activated: Option<ParsedAbility> = None;
            let mut granted_activated_display: Option<String> = None;
            let actions = if let Some(actions) = parse_ability_line(&ability_tokens) {
                Some(actions)
            } else if ability_tokens
                .iter()
                .any(|token| matches!(token, Token::Colon(_)))
            {
                let Some(colon_idx) = ability_tokens
                    .iter()
                    .position(|token| matches!(token, Token::Colon(_)))
                else {
                    return Ok(None);
                };
                let and_idx = (0..colon_idx)
                    .rev()
                    .find(|idx| ability_tokens[*idx].is_word("and"));
                let Some(and_idx) = and_idx else {
                    return Ok(None);
                };
                let keyword_head = trim_commas(&ability_tokens[..and_idx]);
                let activated_tail = trim_commas(&ability_tokens[and_idx + 1..]);
                if keyword_head.is_empty() || activated_tail.is_empty() {
                    return Ok(None);
                }
                let Some(actions) = parse_ability_line(&keyword_head) else {
                    return Ok(None);
                };
                let has_colon = activated_tail
                    .iter()
                    .any(|token| matches!(token, Token::Colon(_)));
                let Some(parsed) = parse_activated_line(&activated_tail)? else {
                    if has_colon {
                        return Err(CardTextError::ParseError(format!(
                            "unsupported granted activated ability in anthem clause (clause: '{}')",
                            clause_words.join(" ")
                        )));
                    }
                    return Ok(None);
                };
                let display = words(&activated_tail).join(" ");
                granted_activated_display = Some(display);
                granted_activated = Some(parsed);
                Some(actions)
            } else {
                None
            };

            if let Some(triggered) = parse_triggered_granted_ability(&ability_tokens)? {
                let display = format!(
                    "{} has {}",
                    clause_words.join(" "),
                    words(&ability_tokens).join(" ")
                );
                extras.push(grant_object_ability_for_anthem_subject(
                    &clause, triggered, display,
                ));
            } else if let Some(actions) = actions {
                reject_unimplemented_keyword_actions(&actions, &clause_words.join(" "))?;
                let granted = actions
                    .into_iter()
                    .filter_map(keyword_action_to_static_ability)
                    .collect::<Vec<_>>();
                if granted.is_empty() {
                    return Ok(None);
                }
                for ability in granted {
                    extras.push(grant_for_anthem_subject(&clause, ability).into());
                }

                if let Some(activated) = granted_activated {
                    extras.push(grant_object_ability_for_anthem_subject(
                        &clause,
                        activated,
                        granted_activated_display.unwrap_or_else(|| clause_words.join(" ")),
                    ));
                }
            } else {
                return Ok(None);
            }

            if grant_must_attack {
                extras.push(grant_for_anthem_subject(&clause, StaticAbility::must_attack()).into());
            }
            continue;
        }

        if let Some(triggered) = parse_triggered_granted_ability(&segment)? {
            let display = format!(
                "{} has {}",
                clause_words.join(" "),
                words(&segment).join(" ")
            );
            extras.push(grant_object_ability_for_anthem_subject(
                &clause, triggered, display,
            ));
            continue;
        }

        return Ok(None);
    }

    if extras.is_empty() {
        return Ok(None);
    }

    let mut result = vec![build_anthem_static_ability(&clause).into()];
    result.extend(extras);
    Ok(Some(result))
}

pub(crate) fn parse_conditional_all_creatures_able_to_block_line(
    tokens: &[Token],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let all_words = normalize_cant_words(tokens);
    if !all_words.starts_with(&["as", "long", "as"]) {
        return Ok(None);
    }

    let Some(comma_idx) = tokens
        .iter()
        .position(|token| matches!(token, Token::Comma(_)))
    else {
        return Ok(None);
    };
    if comma_idx <= 3 {
        return Ok(None);
    }

    let condition_tokens = trim_commas(&tokens[3..comma_idx]);
    if condition_tokens.is_empty() {
        return Ok(None);
    }
    let condition = parse_static_condition_clause(&condition_tokens)?;

    let remainder = trim_commas(&tokens[comma_idx + 1..]);
    let remainder_words = normalize_cant_words(&remainder);
    if remainder_words.as_slice()
        == [
            "all",
            "creatures",
            "able",
            "to",
            "block",
            "this",
            "creature",
            "do",
            "so",
        ]
        || remainder_words.as_slice()
            == [
                "all",
                "creatures",
                "able",
                "to",
                "block",
                "this",
                "do",
                "so",
            ]
    {
        return Ok(Some(StaticAbilityAst::ConditionalStaticAbility {
            ability: Box::new(StaticAbilityAst::Static(StaticAbility::must_block())),
            condition,
        }));
    }

    if remainder_words.as_slice()
        == [
            "all",
            "creatures",
            "able",
            "to",
            "block",
            "enchanted",
            "creature",
            "do",
            "so",
        ]
    {
        return Ok(Some(StaticAbilityAst::AttachedStaticAbilityGrant {
            ability: Box::new(StaticAbilityAst::Static(StaticAbility::must_block())),
            display: "enchanted creature has this creature must be blocked if able".to_string(),
            condition: Some(condition),
        }));
    }

    Ok(None)
}

pub(crate) fn parse_source_can_attack_as_though_no_defender_as_long_as_line(
    tokens: &[Token],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let normalized = words(tokens)
        .into_iter()
        .map(|word| if word == "didn't" { "didnt" } else { word })
        .collect::<Vec<_>>();
    let Some(can_idx) = normalized.windows(11).position(|window| {
        window
            == [
                "can", "attack", "as", "though", "it", "didnt", "have", "defender", "as", "long",
                "as",
            ]
    }) else {
        return Ok(None);
    };
    if can_idx == 0 {
        return Ok(None);
    }

    let subject_end = token_index_for_word_index(tokens, can_idx).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unable to map conditional no-defender subject (clause: '{}')",
            normalized.join(" ")
        ))
    })?;
    let subject_tokens = trim_commas(&tokens[..subject_end]);
    if subject_tokens.is_empty() {
        return Ok(None);
    }

    let condition_start = token_index_for_word_index(tokens, can_idx + 11).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unable to map conditional no-defender condition (clause: '{}')",
            normalized.join(" ")
        ))
    })?;
    let condition_tokens = trim_commas(&tokens[condition_start..]);
    if condition_tokens.is_empty() {
        return Ok(None);
    }
    let condition = parse_static_condition_clause(&condition_tokens)?;

    let subject = parse_anthem_subject(&subject_tokens)?;
    let granted = match subject {
        AnthemSubjectAst::Source => StaticAbilityAst::ConditionalStaticAbility {
            ability: Box::new(StaticAbilityAst::Static(
                StaticAbility::can_attack_as_though_no_defender(),
            )),
            condition,
        },
        AnthemSubjectAst::Filter(filter) => StaticAbilityAst::GrantStaticAbility {
            filter,
            ability: Box::new(StaticAbilityAst::Static(
                StaticAbility::can_attack_as_though_no_defender(),
            )),
            condition: Some(condition),
        },
    };
    Ok(Some(granted))
}

pub(crate) fn parse_as_long_as_condition_can_attack_as_though_no_defender_line(
    tokens: &[Token],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let normalized = words(tokens)
        .into_iter()
        .map(|word| if word == "didn't" { "didnt" } else { word })
        .collect::<Vec<_>>();
    if !normalized.starts_with(&["as", "long", "as"]) {
        return Ok(None);
    }

    let Some(can_idx) = normalized.windows(8).position(|window| {
        window
            == [
                "can", "attack", "as", "though", "it", "didnt", "have", "defender",
            ]
    }) else {
        return Ok(None);
    };
    let Some(comma_idx) = tokens
        .iter()
        .position(|token| matches!(token, Token::Comma(_)))
    else {
        return Ok(None);
    };
    let Some(can_token_idx) = token_index_for_word_index(tokens, can_idx) else {
        return Ok(None);
    };
    if comma_idx >= can_token_idx {
        return Ok(None);
    }

    let condition_tokens = trim_commas(&tokens[3..comma_idx]);
    if condition_tokens.is_empty() {
        return Ok(None);
    }
    let subject_tokens = trim_commas(&tokens[comma_idx + 1..can_token_idx]);
    if subject_tokens.is_empty() {
        return Ok(None);
    }

    let condition = parse_static_condition_clause(&condition_tokens)?;
    let subject = parse_anthem_subject(&subject_tokens)?;
    let granted = match subject {
        AnthemSubjectAst::Source => StaticAbilityAst::ConditionalStaticAbility {
            ability: Box::new(StaticAbilityAst::Static(
                StaticAbility::can_attack_as_though_no_defender(),
            )),
            condition,
        },
        AnthemSubjectAst::Filter(filter) => StaticAbilityAst::GrantStaticAbility {
            filter,
            ability: Box::new(StaticAbilityAst::Static(
                StaticAbility::can_attack_as_though_no_defender(),
            )),
            condition: Some(condition),
        },
    };
    Ok(Some(granted))
}

pub(crate) fn parse_gets_and_attacks_each_combat_if_able_line(
    tokens: &[Token],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let clause_words = words(tokens);
    let Some(get_idx) = tokens
        .iter()
        .position(|token| token.is_word("get") || token.is_word("gets"))
    else {
        return Ok(None);
    };
    let Some(and_idx) = tokens
        .iter()
        .enumerate()
        .find_map(|(idx, token)| (idx > get_idx && token.is_word("and")).then_some(idx))
    else {
        return Ok(None);
    };
    let Some(attack_idx) = tokens.iter().enumerate().find_map(|(idx, token)| {
        (idx > and_idx && (token.is_word("attack") || token.is_word("attacks"))).then_some(idx)
    }) else {
        return Ok(None);
    };

    let attack_tail = words(&tokens[attack_idx..]);
    if attack_tail.as_slice() != ["attacks", "each", "combat", "if", "able"]
        && attack_tail.as_slice() != ["attack", "each", "combat", "if", "able"]
    {
        return Ok(None);
    }

    let clause = parse_anthem_clause(tokens, get_idx, and_idx)?;
    let mut result = vec![build_anthem_static_ability(&clause).into()];
    result.push(grant_for_anthem_subject(
        &clause,
        StaticAbility::must_attack(),
    ));

    if result.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "failed to parse gets-and-attacks clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    Ok(Some(result))
}

pub(crate) fn parse_anthem_and_granted_ability_line(
    tokens: &[Token],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let clause_words = words(tokens);
    if contains_until_end_of_turn(&clause_words) {
        return Ok(None);
    }

    let Some(get_idx) = tokens
        .iter()
        .position(|token| token.is_word("get") || token.is_word("gets"))
    else {
        return Ok(None);
    };
    let Some(and_idx) = tokens
        .iter()
        .enumerate()
        .find_map(|(idx, token)| (idx > get_idx && token.is_word("and")).then_some(idx))
    else {
        return Ok(None);
    };
    let tail_tokens = trim_edge_punctuation(&tokens[and_idx + 1..]);
    let tail_words = words(&tail_tokens);
    let granted_ability = match tail_words.as_slice() {
        ["cant", "be", "blocked"] | ["cannot", "be", "blocked"] => StaticAbility::unblockable(),
        ["is", "every", "creature", "type"] | ["is", "every", "creature", "types"] => {
            StaticAbility::changeling()
        }
        _ => return Ok(None),
    };

    let clause = parse_anthem_clause(tokens, get_idx, and_idx)?;
    let mut result = vec![build_anthem_static_ability(&clause).into()];
    result.push(grant_for_anthem_subject(&clause, granted_ability));

    Ok(Some(result))
}

pub(crate) fn parse_anthem_line(tokens: &[Token]) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    // Targeted "gets +N/+N" text is usually a one-shot spell/ability effect,
    // not a global/static anthem.
    if words.contains(&"target") {
        return Ok(None);
    }
    // "until end of turn" indicates a temporary effect, not a permanent anthem.
    if contains_until_end_of_turn(&words) {
        return Ok(None);
    }

    let get_idx = tokens
        .iter()
        .position(|token| token.is_word("get") || token.is_word("gets"));
    let Some(get_idx) = get_idx else {
        return Ok(None);
    };
    let clause = parse_anthem_clause(tokens, get_idx, tokens.len())?;
    Ok(Some(build_anthem_static_ability(&clause)))
}

pub(crate) fn parse_has_base_power_toughness_static_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words_all = words(tokens);
    let Some(has_idx) = words_all
        .iter()
        .position(|word| *word == "has" || *word == "have")
    else {
        return Ok(None);
    };
    let subject_tokens = trim_commas(&tokens[..has_idx]);
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    let subject_words = words(&subject_tokens);
    if subject_words.contains(&"target") {
        return Ok(None);
    }
    if starts_with_until_end_of_turn(&subject_words)
        || subject_words.starts_with(&["until", "your", "next", "turn"])
    {
        return Ok(None);
    }

    let rest_words = &words_all[has_idx + 1..];
    if rest_words.len() < 5 || !rest_words.starts_with(&["base", "power", "and", "toughness"]) {
        return Ok(None);
    }
    let tail = &rest_words[5..];
    if !tail.is_empty() {
        return Ok(None);
    }

    let (power, toughness) = parse_pt_modifier(rest_words[4]).map_err(|_| {
        CardTextError::ParseError(format!(
            "invalid base power/toughness value (clause: '{}')",
            words_all.join(" ")
        ))
    })?;

    let subject = parse_anthem_subject(&subject_tokens)?;
    let filter = match subject {
        AnthemSubjectAst::Source => ObjectFilter::source(),
        AnthemSubjectAst::Filter(filter) => filter,
    };

    Ok(Some(StaticAbility::set_base_power_toughness(
        filter, power, toughness,
    )))
}

fn is_negated_creature_tail(words: &[&str]) -> bool {
    if words.is_empty() {
        return false;
    }

    let is_creature_phrase = |tail: &[&str]| {
        matches!(
            tail,
            ["creature"] | ["creatures"] | ["a", "creature"] | ["an", "creature"]
        )
    };

    let be = words[0];
    if be == "isnt" || be == "isn't" {
        return is_creature_phrase(&words[1..]);
    }

    if be == "is" || be == "are" {
        if words.get(1).copied() == Some("not") {
            return is_creature_phrase(&words[2..]);
        }
        if words.get(1).copied() == Some("no") && words.get(2).copied() == Some("longer") {
            return is_creature_phrase(&words[3..]);
        }
    }

    false
}

pub(crate) fn parse_isnt_creature_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let all_words = words(tokens);
    if all_words.len() < 3 {
        return Ok(None);
    }
    if all_words.contains(&"target") || contains_until_end_of_turn(&all_words) {
        return Ok(None);
    }

    let mut condition: Option<crate::ConditionExpr> = None;
    let clause_tokens_buf = if all_words.starts_with(&["as", "long", "as"]) {
        let Some(comma_idx) = tokens
            .iter()
            .position(|token| matches!(token, Token::Comma(_)))
        else {
            return Ok(None);
        };
        if comma_idx <= 3 {
            return Err(CardTextError::ParseError(format!(
                "missing condition after leading 'as long as' clause (clause: '{}')",
                all_words.join(" ")
            )));
        }
        let condition_tokens = trim_commas(&tokens[3..comma_idx]);
        if condition_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing condition after leading 'as long as' clause (clause: '{}')",
                all_words.join(" ")
            )));
        }
        condition = Some(parse_static_condition_clause(&condition_tokens)?);
        Some(trim_commas(&tokens[comma_idx + 1..]))
    } else {
        None
    };
    let clause_tokens = clause_tokens_buf.as_deref().unwrap_or(tokens);

    let clause_words = words(clause_tokens);
    if clause_words.len() < 3 {
        return Ok(None);
    }

    let Some(verb_word_idx) = clause_words
        .iter()
        .position(|word| matches!(*word, "isnt" | "isn't" | "is" | "are"))
    else {
        return Ok(None);
    };
    if !is_negated_creature_tail(&clause_words[verb_word_idx..]) {
        return Ok(None);
    }

    let verb_token_idx =
        token_index_for_word_index(clause_tokens, verb_word_idx).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unable to map subject in isn't-a-creature clause (clause: '{}')",
                all_words.join(" ")
            ))
        })?;
    let subject_tokens = trim_commas(&clause_tokens[..verb_token_idx]);
    if subject_tokens.is_empty() {
        return Ok(None);
    }

    let subject = parse_anthem_subject(&subject_tokens)?;
    let filter = match subject {
        AnthemSubjectAst::Source => ObjectFilter::source(),
        AnthemSubjectAst::Filter(filter) => filter,
    };

    let mut remove =
        crate::static_abilities::RemoveCardTypesForFilter::new(filter, vec![CardType::Creature]);
    if let Some(condition) = condition {
        remove = remove.with_condition(condition);
    }
    Ok(Some(StaticAbility::new(remove)))
}

pub(crate) fn parse_has_base_power_toughness_and_granted_keywords_static_line(
    tokens: &[Token],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let clause_words = words(tokens);
    if clause_words.is_empty() {
        return Ok(None);
    }

    let Some(has_idx) = tokens
        .iter()
        .position(|token| token.is_word("has") || token.is_word("have"))
    else {
        return Ok(None);
    };
    if has_idx == 0 || has_idx + 1 >= tokens.len() {
        return Ok(None);
    }

    let subject_tokens = trim_commas(&tokens[..has_idx]);
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    let subject_words = words(&subject_tokens);
    if subject_words.contains(&"target") {
        return Ok(None);
    }
    if starts_with_until_end_of_turn(&subject_words)
        || subject_words.starts_with(&["until", "your", "next", "turn"])
    {
        return Ok(None);
    }

    let rest_tokens = trim_commas(&tokens[has_idx + 1..]);
    let rest_words = words(&rest_tokens);
    if rest_words.len() < 8 || !rest_words.starts_with(&["base", "power", "and", "toughness"]) {
        return Ok(None);
    }
    let (power, toughness) = parse_pt_modifier(rest_words[4]).map_err(|_| {
        CardTextError::ParseError(format!(
            "invalid base power/toughness value (clause: '{}')",
            clause_words.join(" ")
        ))
    })?;
    if rest_words[5] != "and" {
        return Ok(None);
    }
    if !matches!(rest_words[6], "has" | "have" | "gain" | "gains") {
        return Ok(None);
    }

    let Some(ability_start_idx) = token_index_for_word_index(&rest_tokens, 7) else {
        return Err(CardTextError::ParseError(format!(
            "missing granted keyword list after base power/toughness clause (clause: '{}')",
            clause_words.join(" ")
        )));
    };
    let ability_tokens = trim_commas(&rest_tokens[ability_start_idx..]);
    if ability_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing granted keyword list after base power/toughness clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let Some(actions) = parse_ability_line(&ability_tokens) else {
        return Ok(None);
    };
    reject_unimplemented_keyword_actions(&actions, &clause_words.join(" "))?;
    let granted = actions;
    if granted.is_empty() {
        return Ok(None);
    }

    let subject = match parse_anthem_subject(&subject_tokens) {
        Ok(subject) => subject,
        Err(_) => return Ok(None),
    };

    let mut compiled = Vec::new();
    match subject {
        AnthemSubjectAst::Source => {
            compiled.push(
                StaticAbility::set_base_power_toughness(ObjectFilter::source(), power, toughness)
                    .into(),
            );
            compiled.extend(granted.into_iter().map(StaticAbilityAst::KeywordAction));
        }
        AnthemSubjectAst::Filter(filter) => {
            compiled.push(
                StaticAbility::set_base_power_toughness(filter.clone(), power, toughness).into(),
            );
            for action in granted {
                compiled.push(StaticAbilityAst::GrantKeywordAction {
                    filter: filter.clone(),
                    action,
                    condition: None,
                });
            }
        }
    }

    Ok(Some(compiled))
}

pub(crate) fn parse_filter_has_granted_ability_line(
    tokens: &[Token],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let clause_words = words(tokens);
    if clause_words.is_empty() {
        return Ok(None);
    }

    let Some(has_idx) = tokens
        .iter()
        .rposition(|token| token.is_word("has") || token.is_word("have"))
    else {
        return Ok(None);
    };
    if has_idx == 0 || has_idx + 1 >= tokens.len() {
        return Ok(None);
    }
    if tokens[..has_idx]
        .iter()
        .any(|token| token.is_word("get") || token.is_word("gets"))
    {
        return Ok(None);
    }

    let (condition, subject_start) = match parse_anthem_prefix_condition(tokens, has_idx) {
        Ok(parsed) => parsed,
        Err(_) => return Ok(None),
    };
    let subject_tokens = trim_commas(&tokens[subject_start..has_idx]);
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    let subject_words = words(&subject_tokens);
    if subject_words.iter().any(|word| {
        matches!(
            *word,
            "deal"
                | "deals"
                | "create"
                | "creates"
                | "draw"
                | "draws"
                | "destroy"
                | "destroys"
                | "exile"
                | "exiles"
                | "return"
                | "returns"
                | "sacrifice"
                | "sacrifices"
                | "put"
                | "puts"
                | "gain"
                | "gains"
                | "lose"
                | "loses"
                | "discard"
                | "discards"
                | "counter"
                | "counters"
                | "search"
                | "reveals"
                | "investigate"
                | "investigates"
        )
    }) {
        return Ok(None);
    }
    if subject_words.contains(&"may") {
        return Ok(None);
    }
    let ability_tokens = &tokens[has_idx + 1..];
    let has_colon = ability_tokens
        .iter()
        .any(|token| matches!(token, Token::Colon(_)));
    let looks_like_trigger = ability_tokens.first().is_some_and(|token| {
        token.is_word("when")
            || token.is_word("whenever")
            || (token.is_word("at")
                && ability_tokens
                    .get(1)
                    .is_some_and(|next| next.is_word("the")))
    });
    let mut granted_static: Vec<StaticAbilityAst> = Vec::new();
    let mut granted_object_abilities: Vec<ParsedAbility> = Vec::new();
    if has_colon {
        let Some(parsed) = parse_activated_line(ability_tokens)? else {
            return Err(CardTextError::ParseError(format!(
                "unsupported granted activated/triggered ability clause (clause: '{}')",
                clause_words.join(" ")
            )));
        };
        granted_object_abilities.push(parsed);
    } else if let Some(parsed) = parse_cycling_line(ability_tokens)? {
        granted_object_abilities.push(parsed);
    } else if looks_like_trigger {
        match parse_triggered_line(ability_tokens)? {
            LineAst::Triggered {
                trigger,
                effects,
                max_triggers_per_turn,
            } => {
                let parsed = parsed_triggered_ability(
                    trigger,
                    effects,
                    vec![Zone::Battlefield],
                    None,
                    max_triggers_per_turn.map(crate::ConditionExpr::MaxTimesEachTurn),
                    ReferenceImports::default(),
                );
                if parsed_triggered_ability_is_empty(&parsed) {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported empty granted triggered ability clause (clause: '{}')",
                        clause_words.join(" ")
                    )));
                }
                granted_object_abilities.push(parsed);
            }
            _ => {
                return Err(CardTextError::ParseError(format!(
                    "unsupported granted activated/triggered ability clause (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
        }
    } else if let Some(actions) = parse_ability_line(ability_tokens) {
        let [
            KeywordAction::CumulativeUpkeep {
                mana_symbols_per_counter,
                life_per_counter,
                text,
            },
        ] = actions.as_slice()
        else {
            return Ok(None);
        };
        granted_object_abilities.push(ParsedAbility {
            ability: cumulative_upkeep_granted_ability(
                mana_symbols_per_counter.clone(),
                *life_per_counter,
                text.clone(),
            ),
            effects_ast: None,
            reference_imports: ReferenceImports::default(),
            trigger_spec: None,
        });
    } else if let Some(abilities) = parse_static_ability_ast_line(ability_tokens)? {
        granted_static = abilities;
    } else {
        return Ok(None);
    }
    let subject = match parse_anthem_subject(&subject_tokens) {
        Ok(subject) => subject,
        Err(_) => return Ok(None),
    };
    let mut granted: Vec<StaticAbilityAst> = Vec::new();
    if !granted_static.is_empty() {
        for ability in granted_static {
            granted.push(match &subject {
                AnthemSubjectAst::Source => match &condition {
                    Some(condition) => StaticAbilityAst::ConditionalStaticAbility {
                        ability: Box::new(ability),
                        condition: condition.clone(),
                    },
                    None => ability,
                },
                AnthemSubjectAst::Filter(filter) => StaticAbilityAst::GrantStaticAbility {
                    filter: filter.clone(),
                    ability: Box::new(ability),
                    condition: condition.clone(),
                },
            });
        }
    }

    let attached_subject = subject_words
        .first()
        .is_some_and(|word| *word == "enchanted" || *word == "equipped");
    let filter = match &subject {
        AnthemSubjectAst::Filter(filter) => filter.clone(),
        AnthemSubjectAst::Source => ObjectFilter::source(),
    };
    for ability in granted_object_abilities {
        if attached_subject {
            granted.push(StaticAbilityAst::AttachedObjectAbilityGrant {
                ability,
                display: clause_words.join(" "),
                condition: condition.clone(),
            });
            continue;
        }
        granted.push(StaticAbilityAst::GrantObjectAbility {
            filter: filter.clone(),
            ability,
            display: clause_words.join(" "),
            condition: condition.clone(),
        });
    }
    if granted.is_empty() {
        return Ok(None);
    }
    Ok(Some(granted))
}
