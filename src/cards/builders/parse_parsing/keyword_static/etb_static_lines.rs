pub(crate) fn parse_enters_tapped_with_counters_line(
    tokens: &[Token],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let clause_words = words(tokens);
    if clause_words.is_empty() {
        return Ok(None);
    }

    let enters_idx = clause_words
        .iter()
        .position(|word| *word == "enter" || *word == "enters");
    let Some(enters_idx) = enters_idx else {
        return Ok(None);
    };
    let with_idx = clause_words.iter().position(|word| *word == "with");
    let Some(with_idx) = with_idx else {
        return Ok(None);
    };
    if with_idx <= enters_idx {
        return Ok(None);
    }

    let tapped_between = clause_words[enters_idx + 1..with_idx]
        .iter()
        .any(|word| *word == "tapped");
    if !tapped_between {
        return Ok(None);
    }
    if !clause_words
        .iter()
        .any(|word| *word == "counter" || *word == "counters")
    {
        return Ok(None);
    }
    if !is_source_reference_words(&clause_words[..enters_idx]) {
        return Ok(None);
    }

    let Some(counters) = parse_enters_with_counters_line(tokens)? else {
        return Ok(None);
    };

    Ok(Some(vec![StaticAbility::enters_tapped_ability(), counters]))
}

pub(crate) fn parse_enters_with_counters_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let full_words = words(tokens);
    let mut condition: Option<(crate::ConditionExpr, String)> = None;
    let mut clause_tokens: Vec<Token> = tokens.to_vec();

    // Support leading conditional form:
    // "If <condition>, it enters with ..."
    if clause_tokens
        .first()
        .is_some_and(|token| token.is_word("if"))
        && let Some(comma_idx) = clause_tokens
            .iter()
            .position(|token| matches!(token, Token::Comma(_)))
    {
        let condition_tokens = trim_commas(&clause_tokens[1..comma_idx]);
        if !condition_tokens.is_empty() {
            let Some(parsed) = parse_enters_with_counter_condition_clause(&condition_tokens) else {
                return Ok(None);
            };
            let display = words(&condition_tokens).join(" ");
            condition = Some((parsed, display));
            clause_tokens = trim_commas(&clause_tokens[comma_idx + 1..]);
        }
    }

    let clause_words = words(&clause_tokens);
    let enters_idx = clause_words
        .iter()
        .position(|word| *word == "enters")
        .unwrap_or(usize::MAX);
    let Some(enter_token_idx) = token_index_for_word_index(&clause_tokens, enters_idx) else {
        return Ok(None);
    };
    if clause_tokens[..enter_token_idx].iter().any(|token| {
        matches!(
            token,
            Token::Period(_) | Token::Colon(_) | Token::Semicolon(_)
        )
    }) {
        return Ok(None);
    }
    let subject_words = clause_words.get(..enters_idx).unwrap_or_default();
    let source_pronoun_subject = matches!(subject_words, ["it"] | ["its"]);
    if !is_source_reference_words(subject_words) && !source_pronoun_subject {
        return Ok(None);
    }
    if !clause_words.contains(&"with")
        || !clause_words
            .iter()
            .any(|word| *word == "counter" || *word == "counters")
    {
        return Ok(None);
    }

    let with_idx = clause_tokens
        .iter()
        .position(|token| token.is_word("with"))
        .ok_or_else(|| {
            CardTextError::ParseError("missing 'with' in enters-with-counters clause".to_string())
        })?;
    let after_with = &clause_tokens[with_idx + 1..];
    let (mut count, used) = if after_with
        .first()
        .is_some_and(|token| token.is_word("a") || token.is_word("an"))
        && after_with
            .get(1)
            .is_some_and(|token| token.is_word("additional"))
    {
        if let Some((value, value_used)) = parse_value(&after_with[2..]) {
            (value, 2 + value_used)
        } else {
            (Value::Fixed(1), 2)
        }
    } else {
        parse_value(after_with).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing counter count in self ETB counters (clause: '{}')",
                full_words.join(" ")
            ))
        })?
    };

    let counter_type = parse_counter_type_from_tokens(&after_with[used..]).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported counter type for self ETB counters (clause: '{}')",
            full_words.join(" ")
        ))
    })?;

    let counter_idx = after_with
        .iter()
        .position(|token| token.is_word("counter") || token.is_word("counters"))
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing counter keyword for self ETB counters (clause: '{}')",
                full_words.join(" ")
            ))
        })?;
    let mut tail = &after_with[counter_idx + 1..];
    if tail.first().is_some_and(|token| token.is_word("on")) {
        tail = &tail[1..];
    }
    if tail.first().is_some_and(|token| token.is_word("it")) {
        tail = &tail[1..];
    } else if tail
        .first()
        .is_some_and(|token| token.is_word("this") || token.is_word("thiss"))
    {
        tail = &tail[1..];
        if let Some(word) = tail.first().and_then(Token::as_word)
            && (matches!(word, "source" | "spell" | "card")
                || word == "creature"
                || word == "permanent"
                || parse_card_type(word).is_some())
        {
            tail = &tail[1..];
        }
    }
    let tail = trim_commas(tail);
    let tail_has_words = tail.iter().any(|token| token.as_word().is_some());
    if tail_has_words {
        let tail_words = tail.iter().filter_map(Token::as_word).collect::<Vec<_>>();
        let scaled_for_each_count = |dynamic: Value, base_count: &Value| match base_count {
            Value::Fixed(multiplier) => scale_dynamic_cost_modifier_value(dynamic, *multiplier),
            _ => dynamic,
        };
        if tail_words.first().copied() == Some("if") {
            let condition_tokens = trim_commas(&tail[1..]);
            let parsed =
                parse_enters_with_counter_condition_clause(&condition_tokens).ok_or_else(|| {
                    CardTextError::ParseError(format!(
                        "unsupported enters-with-counter condition (clause: '{}')",
                        full_words.join(" ")
                    ))
                })?;
            let display = words(&condition_tokens).join(" ");
            condition = Some(combine_enters_with_counter_conditions(
                condition,
                (parsed, display),
            ));
        } else if tail_words.first().copied() == Some("unless") {
            let condition_tokens = trim_commas(&tail[1..]);
            let parsed =
                parse_enters_with_counter_condition_clause(&condition_tokens).ok_or_else(|| {
                    CardTextError::ParseError(format!(
                        "unsupported enters-with-counter unless condition (clause: '{}')",
                        full_words.join(" ")
                    ))
                })?;
            let display = parse_unless_enters_with_counter_condition_display(&condition_tokens)
                .unwrap_or_else(|| format!("not {}", words(&condition_tokens).join(" ")));
            condition = Some(combine_enters_with_counter_conditions(
                condition,
                (crate::ConditionExpr::Not(Box::new(parsed)), display),
            ));
        } else if tail_words.starts_with(&["plus"]) {
            let for_each_idx = tail
                .windows(2)
                .position(|window| window[0].is_word("for") && window[1].is_word("each"));
            if let Some(for_each_idx) = for_each_idx {
                let extra =
                    parse_dynamic_cost_modifier_value(&tail[for_each_idx..])?.ok_or_else(|| {
                        CardTextError::ParseError(format!(
                            "unsupported additional self ETB counter clause (clause: '{}')",
                            full_words.join(" ")
                        ))
                    })?;
                count = Value::Add(Box::new(count), Box::new(extra));
            } else {
                return Err(CardTextError::ParseError(format!(
                    "unsupported plus-self ETB counter clause (clause: '{}')",
                    full_words.join(" ")
                )));
            }
        } else if tail_words
            .starts_with(&["for", "each", "creature", "that", "died", "this", "turn"])
            || tail_words.starts_with(&["for", "each", "creatures", "that", "died", "this", "turn"])
        {
            count = scaled_for_each_count(Value::CreaturesDiedThisTurn, &count);
        } else if tail_words.starts_with(&[
            "for", "each", "color", "of", "mana", "spent", "to", "cast", "it",
        ]) || tail_words.starts_with(&[
            "for", "each", "colour", "of", "mana", "spent", "to", "cast", "it",
        ]) {
            count = scaled_for_each_count(Value::ColorsOfManaSpentToCastThisSpell, &count);
        } else if tail_words.starts_with(&[
            "for", "each", "creature", "that", "died", "under", "your", "control", "this", "turn",
        ]) || tail_words.starts_with(&[
            "for",
            "each",
            "creatures",
            "that",
            "died",
            "under",
            "your",
            "control",
            "this",
            "turn",
        ]) {
            count = scaled_for_each_count(
                Value::CreaturesDiedThisTurnControlledBy(PlayerFilter::You),
                &count,
            );
        } else if tail_words.starts_with(&["for", "each", "time", "it", "was", "kicked"])
            || tail_words.starts_with(&["for", "each", "time", "this", "spell", "was", "kicked"])
        {
            count = scaled_for_each_count(Value::KickCount, &count);
        } else if tail_words
            == [
                "for",
                "each",
                "magic",
                "game",
                "you",
                "have",
                "lost",
                "to",
                "one",
                "of",
                "your",
                "opponents",
                "since",
                "you",
                "last",
                "won",
                "a",
                "game",
                "against",
                "them",
            ]
            || tail_words
                == [
                    "for",
                    "each",
                    "magic",
                    "games",
                    "you",
                    "have",
                    "lost",
                    "to",
                    "one",
                    "of",
                    "your",
                    "opponents",
                    "since",
                    "you",
                    "last",
                    "won",
                    "a",
                    "game",
                    "against",
                    "them",
                ]
        {
            count = Value::MagicGamesLostToOpponentsSinceLastWin;
        } else if tail_words.starts_with(&["for", "each"]) {
            count = parse_dynamic_cost_modifier_value(&tail)?.ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported for-each self ETB counter clause (clause: '{}')",
                    full_words.join(" ")
                ))
            })?;
        } else if tail_words.starts_with(&["equal", "to"]) {
            count = parse_enters_with_counter_equal_to_value_clause(&tail).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported equal-to self ETB counter clause (clause: '{}')",
                    full_words.join(" ")
                ))
            })?;
        } else {
            count = parse_where_x_value_clause(&tail).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported trailing self ETB counter clause (clause: '{}')",
                    full_words.join(" ")
                ))
            })?;
        }
    }

    if let Some((condition, display)) = condition {
        return Ok(Some(StaticAbility::enters_with_counters_if_condition(
            counter_type,
            count,
            condition,
            display,
        )));
    }

    Ok(Some(StaticAbility::enters_with_counters_value(
        counter_type,
        count,
    )))
}

fn combine_enters_with_counter_conditions(
    existing: Option<(crate::ConditionExpr, String)>,
    next: (crate::ConditionExpr, String),
) -> (crate::ConditionExpr, String) {
    match existing {
        Some((existing_condition, existing_display)) => {
            let combined_condition =
                crate::ConditionExpr::And(Box::new(existing_condition), Box::new(next.0));
            let combined_display =
                match (existing_display.trim().is_empty(), next.1.trim().is_empty()) {
                    (true, true) => String::new(),
                    (false, true) => existing_display,
                    (true, false) => next.1,
                    (false, false) => format!("{} and {}", existing_display.trim(), next.1.trim()),
                };
            (combined_condition, combined_display)
        }
        None => next,
    }
}

fn parse_unless_enters_with_counter_condition_display(tokens: &[Token]) -> Option<String> {
    let condition_words = words(tokens);
    if condition_words.len() >= 11
        && condition_words.get(1).copied() == Some("or")
        && condition_words.get(2).copied() == Some("more")
        && matches!(condition_words.get(3).copied(), Some("color" | "colors"))
        && condition_words.get(4).copied() == Some("of")
        && condition_words.get(5).copied() == Some("mana")
        && matches!(condition_words.get(6).copied(), Some("was" | "were"))
        && condition_words.get(7).copied() == Some("spent")
        && condition_words.get(8).copied() == Some("to")
        && condition_words.get(9).copied() == Some("cast")
        && (condition_words.get(10).copied() == Some("it")
            || condition_words.get(10).copied() == Some("this"))
    {
        let amount = condition_words.first().copied().unwrap_or("1");
        return Some(format!(
            "fewer than {amount} colors of mana were spent to cast it"
        ));
    }
    None
}

fn parse_enters_with_counter_condition_clause(tokens: &[Token]) -> Option<crate::ConditionExpr> {
    let condition_tokens = trim_edge_punctuation(tokens);
    let condition_words = words(&condition_tokens);
    if condition_words.is_empty() {
        return None;
    }

    if condition_words == ["you", "attacked", "this", "turn"]
        || condition_words == ["youve", "attacked", "this", "turn"]
    {
        return Some(crate::ConditionExpr::AttackedThisTurn);
    }
    if condition_words == ["you", "cast", "it"]
        || condition_words == ["you", "cast", "this"]
        || condition_words == ["you", "cast", "this", "spell"]
    {
        return Some(crate::ConditionExpr::SourceWasCast);
    }
    if condition_words == ["a", "creature", "died", "this", "turn"]
        || condition_words == ["one", "or", "more", "creatures", "died", "this", "turn"]
    {
        return Some(crate::ConditionExpr::CreatureDiedThisTurn);
    }
    if condition_words == ["an", "opponent", "lost", "life", "this", "turn"]
        || condition_words
            == [
                "one",
                "or",
                "more",
                "opponents",
                "lost",
                "life",
                "this",
                "turn",
            ]
    {
        return Some(crate::ConditionExpr::OpponentLostLifeThisTurn);
    }
    if condition_words
        == [
            "a",
            "permanent",
            "left",
            "the",
            "battlefield",
            "under",
            "your",
            "control",
            "this",
            "turn",
        ]
        || condition_words
            == [
                "one",
                "or",
                "more",
                "permanents",
                "left",
                "the",
                "battlefield",
                "under",
                "your",
                "control",
                "this",
                "turn",
            ]
    {
        return Some(crate::ConditionExpr::PermanentLeftBattlefieldUnderYourControlThisTurn);
    }
    if condition_words
        == [
            "it", "wasnt", "cast", "or", "no", "mana", "was", "spent", "to", "cast", "it",
        ]
    {
        return Some(crate::ConditionExpr::Or(
            Box::new(crate::ConditionExpr::Not(Box::new(
                crate::ConditionExpr::SourceWasCast,
            ))),
            Box::new(crate::ConditionExpr::Not(Box::new(
                crate::ConditionExpr::ManaSpentToCastThisSpellAtLeast {
                    amount: 1,
                    symbol: None,
                },
            ))),
        ));
    }

    if condition_words.len() == 5
        && condition_words[0] == "x"
        && condition_words[1] == "is"
        && condition_words[3] == "or"
        && condition_words[4] == "more"
    {
        let amount_tokens = [Token::Word(
            condition_words[2].to_string(),
            TextSpan::synthetic(),
        )];
        if let Some((amount, _)) = parse_number(&amount_tokens) {
            return Some(crate::ConditionExpr::XValueAtLeast(amount));
        }
    }

    if condition_words.len() >= 7 {
        let (count_word_idx, valid_prefix) = if condition_words.starts_with(&["youve", "cast"]) {
            (2usize, true)
        } else if condition_words.starts_with(&["you", "cast"]) {
            (2usize, true)
        } else if condition_words.starts_with(&["you", "have", "cast"]) {
            (3usize, true)
        } else {
            (0usize, false)
        };
        if valid_prefix
            && condition_words.get(count_word_idx + 1).copied() == Some("or")
            && condition_words.get(count_word_idx + 2).copied() == Some("more")
            && matches!(
                condition_words.get(count_word_idx + 3).copied(),
                Some("spell" | "spells")
            )
            && condition_words.get(count_word_idx + 4).copied() == Some("this")
            && condition_words.get(count_word_idx + 5).copied() == Some("turn")
        {
            let amount_tokens = [Token::Word(
                condition_words[count_word_idx].to_string(),
                TextSpan::synthetic(),
            )];
            if let Some((amount, _)) = parse_number(&amount_tokens) {
                return Some(crate::ConditionExpr::PlayerCastSpellsThisTurnOrMore {
                    player: PlayerFilter::You,
                    count: amount,
                });
            }
        }
    }

    if condition_words.len() >= 11
        && condition_words.get(1).copied() == Some("or")
        && condition_words.get(2).copied() == Some("more")
        && matches!(condition_words.get(3).copied(), Some("color" | "colors"))
        && condition_words.get(4).copied() == Some("of")
        && condition_words.get(5).copied() == Some("mana")
        && matches!(condition_words.get(6).copied(), Some("was" | "were"))
        && condition_words.get(7).copied() == Some("spent")
        && condition_words.get(8).copied() == Some("to")
        && condition_words.get(9).copied() == Some("cast")
        && (condition_words.get(10).copied() == Some("it")
            || (condition_words.get(10).copied() == Some("this")
                && condition_words.get(11).copied() == Some("spell")))
    {
        let amount_tokens = [Token::Word(
            condition_words[0].to_string(),
            TextSpan::synthetic(),
        )];
        if let Some((amount, _)) = parse_number(&amount_tokens) {
            return Some(crate::ConditionExpr::ColorsOfManaSpentToCastThisSpellOrMore(amount));
        }
    }

    // Cast-time reveal/control checks aren't yet tracked as structured state.
    if condition_words.starts_with(&[
        "you",
        "revealed",
        "a",
        "dragon",
        "card",
        "or",
        "controlled",
        "a",
        "dragon",
        "as",
        "you",
        "cast",
        "this",
        "spell",
    ]) {
        return Some(crate::ConditionExpr::Unmodeled(condition_words.join(" ")));
    }

    parse_static_condition_clause(&condition_tokens).ok()
}

fn parse_enters_with_counter_equal_to_value_clause(tokens: &[Token]) -> Option<Value> {
    let trimmed = trim_edge_punctuation(tokens);
    let words_all = words(&trimmed);
    if !words_all.starts_with(&["equal", "to"]) {
        return None;
    }

    if trimmed.len() < 2 {
        return None;
    }

    let mut where_tokens = Vec::with_capacity(trimmed.len() + 1);
    where_tokens.push(Token::Word("where".to_string(), TextSpan::synthetic()));
    where_tokens.push(Token::Word("x".to_string(), TextSpan::synthetic()));
    where_tokens.push(Token::Word("is".to_string(), TextSpan::synthetic()));
    where_tokens.extend_from_slice(&trimmed[2..]);

    parse_where_x_value_clause(&where_tokens)
        .or_else(|| parse_equal_to_greatest_cards_drawn_this_turn_value(&trimmed))
        .or_else(|| parse_add_mana_equal_amount_value(&trimmed))
        .or_else(|| parse_equal_to_aggregate_filter_value(&trimmed))
        .or_else(|| parse_equal_to_number_of_filter_plus_or_minus_fixed_value(&trimmed))
        .or_else(|| parse_equal_to_number_of_filter_value(&trimmed))
        .or_else(|| parse_equal_to_number_of_opponents_you_have_value(&trimmed))
        .or_else(|| parse_equal_to_number_of_counters_on_reference_value(&trimmed))
}

fn parse_equal_to_greatest_cards_drawn_this_turn_value(tokens: &[Token]) -> Option<Value> {
    let words_all = words(tokens);
    if words_all
        == [
            "equal", "to", "the", "greatest", "number", "of", "cards", "an", "opponent", "has",
            "drawn", "this", "turn",
        ]
        || words_all
            == [
                "equal", "to", "greatest", "number", "of", "cards", "an", "opponent", "has",
                "drawn", "this", "turn",
            ]
    {
        return Some(Value::MaxCardsDrawnThisTurn(PlayerFilter::Opponent));
    }
    None
}

pub(crate) fn parse_where_x_value_clause(tokens: &[Token]) -> Option<Value> {
    let words = words(tokens);
    if !words.starts_with(&["where", "x", "is"]) {
        return None;
    }

    if let Some(value) = parse_where_x_source_stat_value(tokens) {
        return Some(value);
    }

    if let Some(value) = parse_where_x_life_gained_this_turn_value(tokens) {
        return Some(value);
    }

    if let Some(value) = parse_where_x_life_lost_this_turn_value(tokens) {
        return Some(value);
    }

    if let Some(value) = parse_where_x_noncombat_damage_to_opponents_value(tokens) {
        return Some(value);
    }

    if let Some(value) = parse_where_x_is_aggregate_filter_value(tokens) {
        return Some(value);
    }

    // where X is your devotion to black
    if words.contains(&"devotion") {
        if let Ok(Some(value)) = parse_devotion_value_from_add_clause(tokens) {
            return Some(value);
        }
    }

    // where X is the total number of cards in all players' hands
    if words.contains(&"cards")
        && words.contains(&"in")
        && words.contains(&"all")
        && words.contains(&"players")
        && (words.contains(&"hand") || words.contains(&"hands"))
    {
        let mut filter = ObjectFilter::default();
        filter.zone = Some(Zone::Hand);
        return Some(Value::Count(filter));
    }

    // where X is N plus the number of <objects>
    if let Some(value) = parse_where_x_is_fixed_plus_number_of_filter_value(tokens) {
        return Some(value);
    }

    // where X is the number of <objects> plus/minus N
    if let Some(value) = parse_where_x_is_number_of_filter_plus_or_minus_fixed_value(tokens) {
        return Some(value);
    }

    if matches!(
        words.get(3..),
        Some(["the", "mana", "value", "of", "the", "exiled", "card"])
            | Some(["the", "exiled", "card", "mana", "value"])
            | Some(["the", "exiled", "cards", "mana", "value"])
    ) {
        return Some(Value::ManaValueOf(Box::new(ChooseSpec::Tagged(
            TagKey::from(IT_TAG),
        ))));
    }

    // where X is the number of cards in your hand
    if words.contains(&"cards")
        && words.contains(&"in")
        && words.contains(&"your")
        && (words.contains(&"hand") || words.contains(&"hands"))
    {
        return Some(Value::CardsInHand(PlayerFilter::You));
    }

    // where X is the number of creatures in your party
    if words.contains(&"party")
        && words.contains(&"your")
        && (words.contains(&"creature") || words.contains(&"creatures"))
    {
        return Some(Value::PartySize(PlayerFilter::You));
    }

    // where X is the number of differently named <objects>
    if let Some(value) = parse_where_x_is_number_of_differently_named_filter_value(tokens) {
        return Some(value);
    }

    // where X is the number of <objects>
    if let Some(value) = parse_where_x_is_number_of_filter_value(tokens) {
        return Some(value);
    }

    None
}

pub(crate) fn parse_where_x_source_stat_value(tokens: &[Token]) -> Option<Value> {
    let words = words(tokens);
    if !words.starts_with(&["where", "x", "is"]) {
        return None;
    }
    match words.get(3..) {
        Some(["this", "power"])
        | Some(["thiss", "power"])
        | Some(["this", "creature", "power"])
        | Some(["thiss", "creature", "power"])
        | Some(["this", "creatures", "power"])
        | Some(["thiss", "creatures", "power"])
        | Some(["its", "power"]) => Some(Value::SourcePower),
        Some(["this", "toughness"])
        | Some(["thiss", "toughness"])
        | Some(["this", "creature", "toughness"])
        | Some(["thiss", "creature", "toughness"])
        | Some(["this", "creatures", "toughness"])
        | Some(["thiss", "creatures", "toughness"])
        | Some(["its", "toughness"]) => Some(Value::SourceToughness),
        Some(["this", "mana", "value"])
        | Some(["thiss", "mana", "value"])
        | Some(["this", "creature", "mana", "value"])
        | Some(["thiss", "creature", "mana", "value"])
        | Some(["this", "creatures", "mana", "value"])
        | Some(["thiss", "creatures", "mana", "value"])
        | Some(["its", "mana", "value"]) => Some(Value::ManaValueOf(Box::new(ChooseSpec::Source))),
        _ => None,
    }
}

pub(crate) fn parse_where_x_life_gained_this_turn_value(tokens: &[Token]) -> Option<Value> {
    let words = words(tokens);
    if !words.starts_with(&["where", "x", "is"]) {
        return None;
    }
    match words.get(3..) {
        Some(
            [
                "the",
                "amount",
                "of",
                "life",
                "you",
                "gained",
                "this",
                "turn",
            ],
        )
        | Some(["amount", "of", "life", "you", "gained", "this", "turn"]) => {
            Some(Value::LifeGainedThisTurn(PlayerFilter::You))
        }
        Some(
            [
                "the",
                "amount",
                "of",
                "life",
                "youve",
                "gained",
                "this",
                "turn",
            ],
        )
        | Some(["amount", "of", "life", "youve", "gained", "this", "turn"]) => {
            Some(Value::LifeGainedThisTurn(PlayerFilter::You))
        }
        _ => None,
    }
}

pub(crate) fn parse_where_x_life_lost_this_turn_value(tokens: &[Token]) -> Option<Value> {
    let words = words(tokens);
    if !words.starts_with(&["where", "x", "is"]) {
        return None;
    }
    match words.get(3..) {
        Some(
            [
                "the",
                "total",
                "life",
                "lost",
                "by",
                "your",
                "opponents",
                "this",
                "turn",
            ],
        )
        | Some(
            [
                "total",
                "life",
                "lost",
                "by",
                "your",
                "opponents",
                "this",
                "turn",
            ],
        ) => Some(Value::LifeLostThisTurn(PlayerFilter::Opponent)),
        _ => None,
    }
}

pub(crate) fn parse_where_x_noncombat_damage_to_opponents_value(tokens: &[Token]) -> Option<Value> {
    let words = words(tokens);
    if !words.starts_with(&["where", "x", "is"]) {
        return None;
    }
    match words.get(3..) {
        Some(
            [
                "the",
                "total",
                "amount",
                "of",
                "noncombat",
                "damage",
                "dealt",
                "to",
                "your",
                "opponents",
                "this",
                "turn",
            ],
        )
        | Some(
            [
                "total",
                "amount",
                "of",
                "noncombat",
                "damage",
                "dealt",
                "to",
                "your",
                "opponents",
                "this",
                "turn",
            ],
        ) => Some(Value::NoncombatDamageDealtToPlayersThisTurn(
            PlayerFilter::Opponent,
        )),
        _ => None,
    }
}

pub(crate) fn parse_where_x_is_aggregate_filter_value(tokens: &[Token]) -> Option<Value> {
    let clause_words = words(tokens);
    if !clause_words.starts_with(&["where", "x", "is"]) {
        return None;
    }

    let mut idx = 3usize;
    if clause_words.get(idx).copied() == Some("the") {
        idx += 1;
    }
    let aggregate = match clause_words.get(idx).copied() {
        Some("total") => "total",
        Some("greatest") => "greatest",
        _ => return None,
    };
    idx += 1;

    let value_kind = if clause_words.get(idx).copied() == Some("power") {
        idx += 1;
        "power"
    } else if clause_words.get(idx).copied() == Some("toughness") {
        idx += 1;
        "toughness"
    } else if clause_words.get(idx).copied() == Some("mana")
        && clause_words.get(idx + 1).copied() == Some("value")
    {
        idx += 2;
        "mana_value"
    } else {
        return None;
    };

    if !matches!(clause_words.get(idx).copied(), Some("of" | "among")) {
        return None;
    }
    idx += 1;

    if aggregate == "greatest" && value_kind == "mana_value" {
        if let Some(value) = parse_where_x_greatest_commander_mana_value(tokens, idx) {
            return Some(value);
        }
    }

    let object_start_token_idx = token_index_for_word_index(tokens, idx)?;
    let filter_tokens = &tokens[object_start_token_idx..];
    let filter = parse_object_filter(filter_tokens, false).ok()?;

    match (aggregate, value_kind) {
        ("total", "power") => Some(Value::TotalPower(filter)),
        ("total", "toughness") => Some(Value::TotalToughness(filter)),
        ("total", "mana_value") => Some(Value::TotalManaValue(filter)),
        ("greatest", "power") => Some(Value::GreatestPower(filter)),
        ("greatest", "mana_value") => Some(Value::GreatestManaValue(filter)),
        _ => None,
    }
}

pub(crate) fn parse_where_x_greatest_commander_mana_value(
    tokens: &[Token],
    commander_start_word_idx: usize,
) -> Option<Value> {
    let commander_start_token_idx = token_index_for_word_index(tokens, commander_start_word_idx)?;
    let commander_words = words(&tokens[commander_start_token_idx..]);
    let normalized: Vec<&str> = commander_words
        .iter()
        .copied()
        .filter(|word| !is_article(word))
        .collect();
    if normalized
        != [
            "commander",
            "you",
            "own",
            "on",
            "battlefield",
            "or",
            "in",
            "command",
            "zone",
        ]
    {
        return None;
    }

    let mut battlefield_commander = ObjectFilter::default();
    battlefield_commander.zone = Some(Zone::Battlefield);
    battlefield_commander.is_commander = true;
    battlefield_commander.owner = Some(PlayerFilter::You);

    let mut command_zone_commander = battlefield_commander.clone();
    command_zone_commander.zone = Some(Zone::Command);

    let mut combined = ObjectFilter::default();
    combined.any_of = vec![battlefield_commander, command_zone_commander];

    Some(Value::GreatestManaValue(combined))
}

pub(crate) fn parse_where_x_is_number_of_differently_named_filter_value(
    tokens: &[Token],
) -> Option<Value> {
    let clause_words = words(tokens);
    if !clause_words.starts_with(&["where", "x", "is"]) {
        return None;
    }

    let number_idx = clause_words.iter().position(|word| *word == "number")?;
    if clause_words.get(number_idx + 1).copied() != Some("of") {
        return None;
    }
    if clause_words.get(number_idx + 2).copied() != Some("differently") {
        return None;
    }
    if clause_words.get(number_idx + 3).copied() != Some("named") {
        return None;
    }

    let object_start_word_idx = number_idx + 4;
    let object_start_token_idx = token_index_for_word_index(tokens, object_start_word_idx)?;
    let filter_tokens = &tokens[object_start_token_idx..];
    let filter = parse_object_filter(filter_tokens, false).ok()?;
    Some(Value::DistinctNames(filter))
}

pub(crate) fn parse_where_x_is_number_of_filter_value(tokens: &[Token]) -> Option<Value> {
    let clause_words = words(tokens);
    if !clause_words.starts_with(&["where", "x", "is"]) {
        return None;
    }

    if clause_words.contains(&"creature")
        && clause_words.contains(&"type")
        && clause_words.contains(&"common")
    {
        return None;
    }

    let number_idx = clause_words.iter().position(|word| *word == "number")?;
    if clause_words.get(number_idx + 1).copied() != Some("of") {
        return None;
    }

    let object_start_word_idx = number_idx + 2;
    let mut seen_words = 0usize;
    let mut object_start_token_idx = None;
    for (idx, token) in tokens.iter().enumerate() {
        if token.as_word().is_none() {
            continue;
        }
        if seen_words == object_start_word_idx {
            object_start_token_idx = Some(idx);
            break;
        }
        seen_words += 1;
    }
    let object_start_token_idx = object_start_token_idx?;
    let filter_tokens = &tokens[object_start_token_idx..];
    let filter_words = words(filter_tokens);
    if filter_words.starts_with(&["basic", "land", "type", "among"])
        || filter_words.starts_with(&["basic", "land", "types", "among"])
    {
        let mut scope_tokens = &filter_tokens[4..];
        if scope_tokens
            .first()
            .is_some_and(|token| token.is_word("the"))
        {
            scope_tokens = &scope_tokens[1..];
        }
        let scope_filter = parse_object_filter(scope_tokens, false).ok()?;
        return Some(Value::BasicLandTypesAmong(scope_filter));
    }
    if filter_words.starts_with(&["color", "among"])
        || filter_words.starts_with(&["colors", "among"])
    {
        let mut scope_tokens = &filter_tokens[2..];
        if scope_tokens
            .first()
            .is_some_and(|token| token.is_word("the"))
        {
            scope_tokens = &scope_tokens[1..];
        }
        let scope_filter = parse_object_filter(scope_tokens, false).ok()?;
        return Some(Value::ColorsAmong(scope_filter));
    }
    let filter = parse_object_filter(filter_tokens, false).ok()?;
    Some(Value::Count(filter))
}

pub(crate) fn parse_where_x_is_fixed_plus_number_of_filter_value(
    tokens: &[Token],
) -> Option<Value> {
    let clause_words = words(tokens);
    if !clause_words.starts_with(&["where", "x", "is"]) {
        return None;
    }

    let value_start_idx = token_index_for_word_index(tokens, 3)?;
    let (fixed_value, fixed_used) = parse_number(&tokens[value_start_idx..])?;
    let plus_word_idx = 3 + fixed_used;
    if clause_words.get(plus_word_idx).copied() != Some("plus") {
        return None;
    }

    let mut number_word_idx = plus_word_idx + 1;
    if clause_words.get(number_word_idx).copied() == Some("the") {
        number_word_idx += 1;
    }
    if clause_words.get(number_word_idx).copied() != Some("number")
        || clause_words.get(number_word_idx + 1).copied() != Some("of")
    {
        return None;
    }

    let filter_start_idx = token_index_for_word_index(tokens, number_word_idx + 2)?;
    let filter_tokens = &tokens[filter_start_idx..];
    let filter_words = words(filter_tokens);
    if filter_words.starts_with(&["basic", "land", "type", "among"])
        || filter_words.starts_with(&["basic", "land", "types", "among"])
    {
        let mut scope_tokens = &filter_tokens[4..];
        if scope_tokens
            .first()
            .is_some_and(|token| token.is_word("the"))
        {
            scope_tokens = &scope_tokens[1..];
        }
        let scope_filter = parse_object_filter(scope_tokens, false).ok()?;
        return Some(Value::Add(
            Box::new(Value::Fixed(fixed_value as i32)),
            Box::new(Value::BasicLandTypesAmong(scope_filter)),
        ));
    }
    if filter_words.starts_with(&["color", "among"])
        || filter_words.starts_with(&["colors", "among"])
    {
        let mut scope_tokens = &filter_tokens[2..];
        if scope_tokens
            .first()
            .is_some_and(|token| token.is_word("the"))
        {
            scope_tokens = &scope_tokens[1..];
        }
        let scope_filter = parse_object_filter(scope_tokens, false).ok()?;
        return Some(Value::Add(
            Box::new(Value::Fixed(fixed_value as i32)),
            Box::new(Value::ColorsAmong(scope_filter)),
        ));
    }
    let filter = parse_object_filter(filter_tokens, false).ok()?;
    Some(Value::Add(
        Box::new(Value::Fixed(fixed_value as i32)),
        Box::new(Value::Count(filter)),
    ))
}

pub(crate) fn parse_where_x_is_number_of_filter_plus_or_minus_fixed_value(
    tokens: &[Token],
) -> Option<Value> {
    let clause_words = words(tokens);
    if !clause_words.starts_with(&["where", "x", "is"]) {
        return None;
    }

    let mut number_word_idx = 3usize;
    if clause_words.get(number_word_idx).copied() == Some("the") {
        number_word_idx += 1;
    }
    if clause_words.get(number_word_idx).copied() != Some("number")
        || clause_words.get(number_word_idx + 1).copied() != Some("of")
    {
        return None;
    }

    let filter_start_word_idx = number_word_idx + 2;
    let operator_word_idx = (filter_start_word_idx + 1..clause_words.len())
        .find(|idx| matches!(clause_words[*idx], "plus" | "minus"))?;
    let operator = clause_words[operator_word_idx];

    let filter_start_token_idx = token_index_for_word_index(tokens, filter_start_word_idx)?;
    let operator_token_idx = token_index_for_word_index(tokens, operator_word_idx)?;
    let filter_tokens = trim_commas(&tokens[filter_start_token_idx..operator_token_idx]);
    let filter = parse_object_filter(&filter_tokens, false).ok()?;
    let filter_words = words(&filter_tokens);
    let count_value = if filter_words.contains(&"cards")
        && filter_words.contains(&"in")
        && filter_words.contains(&"your")
        && (filter_words.contains(&"hand") || filter_words.contains(&"hands"))
    {
        Value::CardsInHand(PlayerFilter::You)
    } else {
        Value::Count(filter)
    };

    let offset_start_token_idx = token_index_for_word_index(tokens, operator_word_idx + 1)?;
    let offset_tokens = trim_commas(&tokens[offset_start_token_idx..]);
    let (offset_value, used) = parse_number(&offset_tokens)?;
    let trailing_words = words(&offset_tokens[used..]);
    if !trailing_words.is_empty() {
        return None;
    }

    let signed_offset = if operator == "minus" {
        -(offset_value as i32)
    } else {
        offset_value as i32
    };
    Some(Value::Add(
        Box::new(count_value),
        Box::new(Value::Fixed(signed_offset)),
    ))
}

pub(crate) fn token_index_for_word_index(tokens: &[Token], word_index: usize) -> Option<usize> {
    let mut seen_words = 0usize;
    for (idx, token) in tokens.iter().enumerate() {
        if token.as_word().is_none() {
            continue;
        }
        if seen_words == word_index {
            return Some(idx);
        }
        seen_words += 1;
    }
    None
}

pub(crate) fn parse_enters_tapped_for_filter_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = words(tokens);
    if is_negated_untap_clause(&clause_words) {
        let has_enters_tapped = clause_words.contains(&"enter") || clause_words.contains(&"enters");
        let has_tapped = clause_words.contains(&"tapped");
        if has_enters_tapped && has_tapped {
            return Err(CardTextError::ParseError(format!(
                "unsupported mixed enters-tapped and negated-untap clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        return Ok(None);
    }
    if clause_words.contains(&"unless") {
        return Ok(None);
    }
    let enter_word_idx = clause_words
        .iter()
        .position(|word| *word == "enter" || *word == "enters");
    let Some(enter_word_idx) = enter_word_idx else {
        return Ok(None);
    };
    let Some(enter_token_idx) = token_index_for_word_index(tokens, enter_word_idx) else {
        return Ok(None);
    };
    if !clause_words
        .iter()
        .skip(enter_word_idx + 1)
        .any(|word| *word == "tapped")
    {
        return Ok(None);
    }
    if clause_words.first().copied() == Some("this") {
        return Ok(None);
    }
    if clause_words.contains(&"copy") {
        return Err(CardTextError::ParseError(format!(
            "unsupported enters-as-copy replacement clause (clause: '{}') [rule=enters-as-copy]",
            clause_words.join(" ")
        )));
    }
    let before_enter = &tokens[..enter_token_idx];
    let before_words = words(before_enter);
    let mut controller_override: Option<PlayerFilter> = None;
    let mut filter_end = before_enter.len();
    let find_suffix_cut = |suffix_len: usize| {
        token_index_for_word_index(before_enter, before_words.len().saturating_sub(suffix_len))
            .unwrap_or(before_enter.len())
    };
    if before_words.ends_with(&["played", "by", "your", "opponents"]) {
        controller_override = Some(PlayerFilter::Opponent);
        filter_end = find_suffix_cut(4);
    } else if before_words.ends_with(&["played", "by", "an", "opponent"])
        || before_words.ends_with(&["played", "by", "a", "opponent"])
    {
        controller_override = Some(PlayerFilter::Opponent);
        filter_end = find_suffix_cut(4);
    } else if before_words.ends_with(&["played", "by", "opponents"]) {
        controller_override = Some(PlayerFilter::Opponent);
        filter_end = find_suffix_cut(3);
    }
    let mut filter = parse_object_filter(&before_enter[..filter_end], false)?;
    if let Some(controller) = controller_override {
        filter.controller = Some(controller);
    }
    Ok(Some(StaticAbility::enters_tapped_for_filter(filter)))
}

pub(crate) fn parse_enters_untapped_for_filter_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = words(tokens);
    if clause_words.contains(&"unless") || clause_words.first().copied() == Some("this") {
        return Ok(None);
    }

    let Some(enter_word_idx) = clause_words
        .iter()
        .position(|word| *word == "enter" || *word == "enters")
    else {
        return Ok(None);
    };
    let Some(enter_token_idx) = token_index_for_word_index(tokens, enter_word_idx) else {
        return Ok(None);
    };
    if !clause_words
        .iter()
        .skip(enter_word_idx + 1)
        .any(|word| *word == "untapped")
    {
        return Ok(None);
    }

    let before_enter = &tokens[..enter_token_idx];
    if before_enter.is_empty() {
        return Ok(None);
    }
    let filter = parse_object_filter(before_enter, false)?;
    Ok(Some(StaticAbility::enters_untapped_for_filter(filter)))
}

pub(crate) fn parse_reveal_from_hand_or_enters_tapped_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = words(tokens);
    if !clause_words.starts_with(&["as", "this", "land", "enters"]) {
        return Ok(None);
    }
    if !clause_words.contains(&"reveal")
        || !clause_words.contains(&"from")
        || !clause_words.contains(&"hand")
    {
        return Ok(None);
    }

    let Some(reveal_word_idx) = clause_words.iter().position(|word| *word == "reveal") else {
        return Err(CardTextError::ParseError(format!(
            "missing 'reveal' keyword in land ETB reveal clause (clause: '{}')",
            clause_words.join(" ")
        )));
    };
    let Some(from_hand_word_idx) = (reveal_word_idx + 1..clause_words.len().saturating_sub(2))
        .find(|idx| {
            clause_words[*idx] == "from"
                && clause_words[*idx + 1] == "your"
                && clause_words[*idx + 2] == "hand"
        })
    else {
        return Err(CardTextError::ParseError(format!(
            "unsupported reveal source in land ETB reveal clause (clause: '{}')",
            clause_words.join(" ")
        )));
    };
    let Some(reveal_filter_start_token_idx) =
        token_index_for_word_index(tokens, reveal_word_idx + 1)
    else {
        return Err(CardTextError::ParseError(format!(
            "missing reveal filter start in land ETB reveal clause (clause: '{}')",
            clause_words.join(" ")
        )));
    };
    let Some(reveal_filter_end_token_idx) = token_index_for_word_index(tokens, from_hand_word_idx)
    else {
        return Err(CardTextError::ParseError(format!(
            "missing reveal filter end in land ETB reveal clause (clause: '{}')",
            clause_words.join(" ")
        )));
    };
    let reveal_filter_tokens =
        trim_edge_punctuation(&tokens[reveal_filter_start_token_idx..reveal_filter_end_token_idx]);
    if reveal_filter_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing reveal filter in land ETB reveal clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    let reveal_filter = parse_object_filter(&reveal_filter_tokens, false)?;
    let reveal_condition = crate::ConditionExpr::YouHaveCardInHandMatching(reveal_filter);

    // Pattern A: "... If you don't, this land enters tapped."
    if let Some(if_you_dont_idx) = clause_words
        .windows(3)
        .position(|window| window == ["if", "you", "dont"])
    {
        let trailing = &clause_words[if_you_dont_idx + 3..];
        let valid_trailing = trailing.starts_with(&["this", "land", "enters", "tapped"])
            || trailing.starts_with(&["this", "land", "enter", "tapped"])
            || trailing.starts_with(&["it", "enters", "tapped"])
            || trailing.starts_with(&["it", "enter", "tapped"])
            || trailing.starts_with(&["it", "enters", "the", "battlefield", "tapped"])
            || trailing.starts_with(&["it", "enter", "the", "battlefield", "tapped"]);
        if !valid_trailing {
            return Err(CardTextError::ParseError(format!(
                "unsupported land ETB reveal trailing clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        parser_trace("parse_static:land-reveal-or-enter-tapped:matched", tokens);
        return Ok(Some(StaticAbility::enters_tapped_unless_condition(
            reveal_condition,
            clause_words.join(" "),
        )));
    }

    // Pattern B: "... This land enters tapped unless you revealed ... this way or you control ..."
    let Some(unless_idx) = clause_words.iter().position(|word| *word == "unless") else {
        return Err(CardTextError::ParseError(format!(
            "unsupported land ETB reveal clause (expected 'if you don't' or 'unless') (clause: '{}')",
            clause_words.join(" ")
        )));
    };
    let before_unless = &clause_words[..unless_idx];
    if !before_unless
        .windows(2)
        .any(|window| window == ["enters", "tapped"] || window == ["enter", "tapped"])
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported land ETB reveal unless-prefix (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let mut condition = reveal_condition;
    if let Some(or_idx_rel) = clause_words[unless_idx + 1..]
        .iter()
        .position(|word| *word == "or")
    {
        let or_idx = unless_idx + 1 + or_idx_rel;
        let Some(control_word_idx) = (or_idx + 1..clause_words.len())
            .find(|idx| clause_words[*idx] == "control" || clause_words[*idx] == "controls")
        else {
            return Err(CardTextError::ParseError(format!(
                "unsupported land ETB reveal disjunction (clause: '{}')",
                clause_words.join(" ")
            )));
        };
        let Some(control_filter_start_token_idx) =
            token_index_for_word_index(tokens, control_word_idx + 1)
        else {
            return Err(CardTextError::ParseError(format!(
                "missing control filter in land ETB reveal clause (clause: '{}')",
                clause_words.join(" ")
            )));
        };
        let control_filter_tokens =
            trim_edge_punctuation(&tokens[control_filter_start_token_idx..]);
        if control_filter_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing control filter in land ETB reveal clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let control_filter = parse_object_filter(&control_filter_tokens, false)?;
        condition = crate::ConditionExpr::Or(
            Box::new(condition),
            Box::new(crate::ConditionExpr::YouControl(control_filter)),
        );
    }

    parser_trace("parse_static:land-reveal-or-enter-tapped:matched", tokens);
    Ok(Some(StaticAbility::enters_tapped_unless_condition(
        condition,
        clause_words.join(" "),
    )))
}

pub(crate) fn parse_conditional_enters_tapped_unless_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = words(tokens);
    if !clause_words.contains(&"enters") && !clause_words.contains(&"enter") {
        return Ok(None);
    }
    if !clause_words.contains(&"tapped") || !clause_words.contains(&"unless") {
        return Ok(None);
    }

    let Some(unless_idx) = tokens.iter().position(|token| token.is_word("unless")) else {
        return Ok(None);
    };
    let condition_words = words(&tokens[unless_idx + 1..]);
    if condition_words.starts_with(&["you", "control", "two", "or", "more", "other", "lands"]) {
        return Ok(Some(
            StaticAbility::enters_tapped_unless_control_two_or_more_other_lands(),
        ));
    }
    if condition_words.starts_with(&["you", "control", "two", "or", "fewer", "other", "lands"]) {
        return Ok(Some(
            StaticAbility::enters_tapped_unless_control_two_or_fewer_other_lands(),
        ));
    }
    if condition_words.starts_with(&["you", "control", "two", "or", "more", "basic", "lands"]) {
        return Ok(Some(
            StaticAbility::enters_tapped_unless_control_two_or_more_basic_lands(),
        ));
    }
    if condition_words.starts_with(&["a", "player", "has", "13", "or", "less", "life"])
        || condition_words.starts_with(&["a", "player", "has", "thirteen", "or", "less", "life"])
    {
        return Ok(Some(
            StaticAbility::enters_tapped_unless_a_player_has_13_or_less_life(),
        ));
    }
    if condition_words.starts_with(&["you", "have", "two", "or", "more", "opponents"]) {
        return Ok(Some(
            StaticAbility::enters_tapped_unless_two_or_more_opponents(),
        ));
    }

    // Generic: "unless you control <object filter>" (covers Mount/Vehicle, etc.).
    if condition_words.starts_with(&["you", "control"])
        || condition_words.starts_with(&["you", "controls"])
    {
        let control_idx = tokens[unless_idx + 1..]
            .iter()
            .position(|token| token.is_word("control") || token.is_word("controls"))
            .map(|idx| unless_idx + 1 + idx)
            .unwrap_or(unless_idx + 1);
        let filter_tokens = trim_edge_punctuation(&tokens[control_idx + 1..]);
        if !filter_tokens.is_empty() {
            if let Ok(filter) = parse_object_filter(&filter_tokens, false) {
                let condition = crate::ConditionExpr::YouControl(filter);
                return Ok(Some(StaticAbility::enters_tapped_unless_condition(
                    condition,
                    clause_words.join(" "),
                )));
            }
        }
    }

    Err(CardTextError::ParseError(format!(
        "unsupported enters tapped unless condition (clause: '{}')",
        clause_words.join(" ")
    )))
}

pub(crate) fn parse_enters_with_additional_counter_for_filter_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = words(tokens);
    let enter_word_idx = clause_words
        .iter()
        .position(|word| *word == "enter" || *word == "enters");
    let Some(enter_word_idx) = enter_word_idx else {
        return Ok(None);
    };
    let Some(enter_token_idx) = token_index_for_word_index(tokens, enter_word_idx) else {
        return Ok(None);
    };
    if tokens[..enter_token_idx].iter().any(|token| {
        matches!(
            token,
            Token::Period(_) | Token::Colon(_) | Token::Semicolon(_)
        )
    }) {
        return Ok(None);
    }

    let subject_tokens = trim_commas(&tokens[..enter_token_idx]);
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    let subject_words = words(&subject_tokens);
    if is_source_reference_words(&subject_words) {
        return Ok(None);
    }
    if matches!(
        subject_words.first().copied(),
        Some("if" | "when" | "whenever" | "as" | "at")
    ) {
        return Ok(None);
    }

    if !clause_words.contains(&"with")
        || !clause_words.contains(&"additional")
        || !clause_words
            .iter()
            .any(|word| *word == "counter" || *word == "counters")
    {
        return Ok(None);
    }

    let Ok(filter) = parse_object_filter(&subject_tokens, false) else {
        return Ok(None);
    };

    let and_as_idx = tokens
        .windows(2)
        .position(|window| window[0].is_word("and") && window[1].is_word("as"));
    let base_tokens = and_as_idx.map_or(tokens, |idx| &tokens[..idx]);

    let additional_idx = base_tokens
        .iter()
        .position(|token| token.is_word("additional"))
        .ok_or_else(|| {
            CardTextError::ParseError("missing 'additional' keyword for ETB counters".to_string())
        })?;
    let count = if let Some(equal_idx) = base_tokens.iter().position(|token| token.is_word("equal"))
    {
        let value_start = equal_idx + 2;
        let value_tokens = trim_commas(base_tokens.get(value_start..).unwrap_or_default());
        parse_value(&value_tokens)
            .map(|(value, _)| value)
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported ETB counter count value (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?
    } else if additional_idx > 0
        && let Some((parsed, _)) = parse_number(&base_tokens[additional_idx - 1..additional_idx])
    {
        Value::Fixed(parsed as i32)
    } else if let Some((parsed, _)) = parse_number(&base_tokens[additional_idx + 1..]) {
        Value::Fixed(parsed as i32)
    } else {
        Value::Fixed(1)
    };

    let counter_type = parse_counter_type_from_tokens(base_tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported counter type for ETB replacement (clause: '{}')",
            clause_words.join(" ")
        ))
    })?;

    let mut added_subtypes = Vec::new();
    if let Some(idx) = and_as_idx {
        let mut addition_tokens = tokens[idx + 1..].to_vec();
        if let Some(Token::Word(_, span)) = addition_tokens.first() {
            addition_tokens[0] = Token::Word("is".to_string(), *span);
        }
        let Some(additions) = parse_type_color_addition_clause(&addition_tokens)? else {
            return Err(CardTextError::ParseError(format!(
                "unsupported ETB type-addition tail (clause: '{}')",
                clause_words.join(" ")
            )));
        };
        if !additions.added_colors.is_empty()
            || !additions.set_colors.is_empty()
            || !additions.card_types.is_empty()
        {
            return Err(CardTextError::ParseError(format!(
                "unsupported non-subtype ETB type addition (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        added_subtypes = additions.subtypes;
    }

    Ok(Some(StaticAbility::enters_with_counters_and_subtypes_for_filter(
        filter,
        counter_type,
        count,
        added_subtypes,
    )))
}
