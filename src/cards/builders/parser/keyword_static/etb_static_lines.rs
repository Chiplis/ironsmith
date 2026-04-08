fn etb_token_words(tokens: &[OwnedLexToken]) -> Vec<&str> {
    crate::cards::builders::parser::lexer::token_word_refs(tokens)
}

fn etb_word_slice_starts_with(words: &[&str], prefix: &[&str]) -> bool {
    crate::cards::builders::parser::token_primitives::slice_starts_with(words, prefix)
}

fn etb_word_slice_ends_with(words: &[&str], suffix: &[&str]) -> bool {
    crate::cards::builders::parser::token_primitives::slice_ends_with(words, suffix)
}

fn etb_word_slice_contains(words: &[&str], expected: &str) -> bool {
    crate::cards::builders::parser::token_primitives::slice_contains_str(words, expected)
}

fn etb_word_slice_contains_all(words: &[&str], expected: &[&str]) -> bool {
    crate::cards::builders::parser::token_primitives::slice_contains_all(words, expected)
}

fn etb_word_slice_contains_any(words: &[&str], expected: &[&str]) -> bool {
    crate::cards::builders::parser::token_primitives::slice_contains_any(words, expected)
}

fn etb_word_offset(words: &[&str], mut predicate: impl FnMut(&str) -> bool) -> Option<usize> {
    crate::cards::builders::parser::token_primitives::find_str_by(words, |word| predicate(word))
}

fn etb_find_token_index(
    tokens: &[OwnedLexToken],
    mut predicate: impl FnMut(&OwnedLexToken) -> bool,
) -> Option<usize> {
    crate::cards::builders::parser::grammar::primitives::find_token_index(tokens, |token| {
        predicate(token)
    })
}

fn etb_find_token_word_sequence_index(
    tokens: &[OwnedLexToken],
    sequence: &[&str],
) -> Option<usize> {
    crate::cards::builders::parser::token_primitives::find_window_by(
        tokens,
        sequence.len(),
        |window| {
            window.len() == sequence.len()
                && window
                    .iter()
                    .zip(sequence.iter())
                    .all(|(token, expected)| token.is_word(expected))
        },
    )
}

fn etb_find_word_sequence_index(words: &[&str], sequence: &[&str]) -> Option<usize> {
    crate::cards::builders::parser::token_primitives::find_window_index(words, sequence)
}

fn etb_has_word_sequence(words: &[&str], sequence: &[&str]) -> bool {
    etb_find_word_sequence_index(words, sequence).is_some()
}

pub(crate) fn parse_enters_tapped_with_counters_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let clause_words = etb_token_words(tokens);
    if clause_words.is_empty() {
        return Ok(None);
    }

    let enters_idx = etb_word_offset(&clause_words, |word| matches!(word, "enter" | "enters"));
    let Some(enters_idx) = enters_idx else {
        return Ok(None);
    };
    let with_idx = etb_word_offset(&clause_words, |word| word == "with");
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
    if !etb_word_slice_contains(&clause_words, "counter")
        && !etb_word_slice_contains(&clause_words, "counters")
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
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let full_words = etb_token_words(tokens);
    let mut condition: Option<(crate::ConditionExpr, String)> = None;
    let mut clause_tokens: Vec<OwnedLexToken> = tokens.to_vec();

    // Support leading conditional form:
    // "If <condition>, it enters with ..."
    if clause_tokens
        .first()
        .is_some_and(|token| token.is_word("if"))
        && let Some(comma_idx) = etb_find_token_index(&clause_tokens, |token| token.is_comma())
    {
        let condition_tokens = trim_commas(&clause_tokens[1..comma_idx]);
        if !condition_tokens.is_empty() {
            let Some(parsed) = parse_enters_with_counter_condition_clause(&condition_tokens) else {
                return Ok(None);
            };
            let display = etb_token_words(&condition_tokens).join(" ");
            condition = Some((parsed, display));
            clause_tokens = trim_commas(&clause_tokens[comma_idx + 1..]);
        }
    }

    let clause_words = etb_token_words(&clause_tokens);
    let enters_idx = etb_word_offset(&clause_words, |word| word == "enters").unwrap_or(usize::MAX);
    let Some(enter_token_idx) = token_index_for_word_index(&clause_tokens, enters_idx) else {
        return Ok(None);
    };
    if clause_tokens[..enter_token_idx]
        .iter()
        .any(|token| token.is_period() || token.is_colon() || token.is_semicolon())
    {
        return Ok(None);
    }
    let subject_words = clause_words.get(..enters_idx).unwrap_or_default();
    let source_pronoun_subject = matches!(subject_words, ["it"] | ["its"]);
    if !is_source_reference_words(subject_words) && !source_pronoun_subject {
        return Ok(None);
    }
    if !etb_word_slice_contains(&clause_words, "with")
        || (!etb_word_slice_contains(&clause_words, "counter")
            && !etb_word_slice_contains(&clause_words, "counters"))
    {
        return Ok(None);
    }

    let with_idx =
        etb_find_token_index(&clause_tokens, |token| token.is_word("with")).ok_or_else(|| {
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

    let counter_idx = etb_find_token_index(after_with, |token| {
        token.is_word("counter") || token.is_word("counters")
    })
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
        if let Some(word) = tail.first().and_then(OwnedLexToken::as_word)
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
        let tail_words = tail
            .iter()
            .filter_map(OwnedLexToken::as_word)
            .collect::<Vec<_>>();
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
            let display = etb_token_words(&condition_tokens).join(" ");
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
                .unwrap_or_else(|| format!("not {}", etb_token_words(&condition_tokens).join(" ")));
            condition = Some(combine_enters_with_counter_conditions(
                condition,
                (crate::ConditionExpr::Not(Box::new(parsed)), display),
            ));
        } else if etb_word_slice_starts_with(&tail_words, &["plus"]) {
            let for_each_idx = etb_find_token_word_sequence_index(&tail, &["for", "each"]);
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
        } else if etb_word_slice_starts_with(
            &tail_words,
            &["for", "each", "creature", "that", "died", "this", "turn"],
        ) || etb_word_slice_starts_with(
            &tail_words,
            &["for", "each", "creatures", "that", "died", "this", "turn"],
        ) {
            count = scaled_for_each_count(Value::CreaturesDiedThisTurn, &count);
        } else if etb_word_slice_starts_with(
            &tail_words,
            &[
                "for", "each", "color", "of", "mana", "spent", "to", "cast", "it",
            ],
        ) || etb_word_slice_starts_with(
            &tail_words,
            &[
                "for", "each", "colour", "of", "mana", "spent", "to", "cast", "it",
            ],
        ) {
            count = scaled_for_each_count(Value::ColorsOfManaSpentToCastThisSpell, &count);
        } else if etb_word_slice_starts_with(
            &tail_words,
            &[
                "for", "each", "creature", "that", "died", "under", "your", "control", "this",
                "turn",
            ],
        ) || etb_word_slice_starts_with(
            &tail_words,
            &[
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
            ],
        ) {
            count = scaled_for_each_count(
                Value::CreaturesDiedThisTurnControlledBy(PlayerFilter::You),
                &count,
            );
        } else if etb_word_slice_starts_with(
            &tail_words,
            &["for", "each", "time", "it", "was", "kicked"],
        ) || etb_word_slice_starts_with(
            &tail_words,
            &["for", "each", "time", "this", "spell", "was", "kicked"],
        ) {
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
        } else if etb_word_slice_starts_with(&tail_words, &["for", "each"]) {
            count = parse_dynamic_cost_modifier_value(&tail)?.ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported for-each self ETB counter clause (clause: '{}')",
                    full_words.join(" ")
                ))
            })?;
        } else if etb_word_slice_starts_with(&tail_words, &["equal", "to"]) {
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

fn parse_unless_enters_with_counter_condition_display(tokens: &[OwnedLexToken]) -> Option<String> {
    let condition_words = etb_token_words(tokens);
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

fn parse_enters_with_counter_condition_clause(
    tokens: &[OwnedLexToken],
) -> Option<crate::ConditionExpr> {
    let condition_tokens = trim_edge_punctuation(tokens);
    let condition_words = etb_token_words(&condition_tokens);
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
        let amount_tokens = [OwnedLexToken::word(
            condition_words[2].to_string(),
            TextSpan::synthetic(),
        )];
        if let Some((amount, _)) = parse_number(&amount_tokens) {
            return Some(crate::ConditionExpr::XValueAtLeast(amount));
        }
    }

    if condition_words.len() >= 7 {
        let (count_word_idx, valid_prefix) =
            if etb_word_slice_starts_with(&condition_words, &["youve", "cast"])
                || etb_word_slice_starts_with(&condition_words, &["you've", "cast"])
            {
                (2usize, true)
            } else if etb_word_slice_starts_with(&condition_words, &["you", "ve", "cast"]) {
                (3usize, true)
            } else if etb_word_slice_starts_with(&condition_words, &["you", "cast"]) {
                (2usize, true)
            } else if etb_word_slice_starts_with(&condition_words, &["you", "have", "cast"]) {
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
            let amount_tokens = [OwnedLexToken::word(
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
        let amount_tokens = [OwnedLexToken::word(
            condition_words[0].to_string(),
            TextSpan::synthetic(),
        )];
        if let Some((amount, _)) = parse_number(&amount_tokens) {
            return Some(crate::ConditionExpr::ColorsOfManaSpentToCastThisSpellOrMore(amount));
        }
    }

    // Cast-time reveal/control checks aren't yet tracked as structured state.
    if etb_word_slice_starts_with(
        &condition_words,
        &[
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
        ],
    ) {
        return Some(crate::ConditionExpr::Unmodeled(condition_words.join(" ")));
    }

    parse_static_condition_clause(&condition_tokens).ok()
}

fn parse_enters_with_counter_equal_to_value_clause(tokens: &[OwnedLexToken]) -> Option<Value> {
    let trimmed = trim_edge_punctuation(tokens);
    let words_all = crate::cards::builders::parser::token_word_refs(&trimmed);
    if !etb_word_slice_starts_with(&words_all, &["equal", "to"]) {
        return None;
    }

    if trimmed.len() < 2 {
        return None;
    }

    let mut where_tokens = Vec::with_capacity(trimmed.len() + 1);
    where_tokens.push(OwnedLexToken::word(
        "where".to_string(),
        TextSpan::synthetic(),
    ));
    where_tokens.push(OwnedLexToken::word("x".to_string(), TextSpan::synthetic()));
    where_tokens.push(OwnedLexToken::word("is".to_string(), TextSpan::synthetic()));
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

fn parse_equal_to_greatest_cards_drawn_this_turn_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    let words_all = crate::cards::builders::parser::token_word_refs(tokens);
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

pub(crate) fn parse_where_x_value_clause(tokens: &[OwnedLexToken]) -> Option<Value> {
    let word_view = crate::cards::builders::parser::grammar::primitives::TokenWordView::new(tokens);
    let words = word_view.word_refs();
    if !etb_word_slice_starts_with(&words, &["where", "x", "is"]) {
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

    if let Some(value) = parse_where_x_opponents_dealt_combat_damage_this_turn_value(tokens) {
        return Some(value);
    }

    if let Some(value) = parse_where_x_noncombat_damage_to_opponents_value(tokens) {
        return Some(value);
    }

    if let Some(value) = parse_where_x_is_aggregate_filter_value(tokens) {
        return Some(value);
    }

    // where X is your devotion to black
    if etb_word_slice_contains(&words, "devotion") {
        if let Ok(Some(value)) = parse_devotion_value_from_add_clause(tokens) {
            return Some(value);
        }
    }

    // where X is the total number of cards in all players' hands
    if etb_word_slice_contains_all(&words, &["cards", "in", "all", "players"])
        && etb_word_slice_contains_any(&words, &["hand", "hands"])
    {
        let mut filter = ObjectFilter::default();
        filter.zone = Some(Zone::Hand);
        return Some(Value::Count(filter));
    }

    // where X is N plus the number of <objects>
    if let Some(value) = parse_where_x_is_fixed_plus_number_of_filter_value(tokens) {
        return Some(value);
    }

    // where X is N plus the sacrificed creature's mana value / power / toughness
    if let Some(value) = parse_where_x_is_fixed_plus_reference_value(tokens) {
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
            | Some(["that", "spell", "mana", "value"])
            | Some(["that", "spells", "mana", "value"])
    ) {
        return Some(Value::ManaValueOf(Box::new(ChooseSpec::Tagged(
            TagKey::from(IT_TAG),
        ))));
    }

    // where X is the number of cards in your hand
    if etb_word_slice_contains_all(&words, &["cards", "in", "your"])
        && etb_word_slice_contains_any(&words, &["hand", "hands"])
    {
        return Some(Value::CardsInHand(PlayerFilter::You));
    }

    // where X is the number of creatures in your party
    if etb_word_slice_contains_all(&words, &["party", "your"])
        && etb_word_slice_contains_any(&words, &["creature", "creatures"])
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

pub(crate) fn parse_where_x_value_clause_lexed(
    tokens: &[crate::cards::builders::parser::lexer::OwnedLexToken],
) -> Option<Value> {
    parse_where_x_value_clause(tokens)
}

pub(crate) fn parse_where_x_source_stat_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    let word_view = crate::cards::builders::parser::grammar::primitives::TokenWordView::new(tokens);
    let words = word_view.word_refs();
    if !etb_word_slice_starts_with(&words, &["where", "x", "is"]) {
        return None;
    }
    let tagged_it = ChooseSpec::Tagged(TagKey::from(IT_TAG));
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
        Some(["that", "creature", "power"])
        | Some(["that", "creatures", "power"])
        | Some(["that", "object", "power"])
        | Some(["that", "objects", "power"])
        | Some(["the", "sacrificed", "creature", "power"])
        | Some(["the", "sacrificed", "creatures", "power"])
        | Some(["sacrificed", "creature", "power"])
        | Some(["sacrificed", "creatures", "power"])
        | Some(["the", "amassed", "army", "power"])
        | Some(["the", "amassed", "armys", "power"])
        | Some(["amassed", "army", "power"])
        | Some(["amassed", "armys", "power"])
        | Some(["the", "army", "you", "amassed", "power"])
        | Some(["army", "you", "amassed", "power"]) => {
            Some(Value::PowerOf(Box::new(tagged_it.clone())))
        }
        Some(["that", "creature", "toughness"])
        | Some(["that", "creatures", "toughness"])
        | Some(["that", "object", "toughness"])
        | Some(["that", "objects", "toughness"])
        | Some(["the", "sacrificed", "creature", "toughness"])
        | Some(["the", "sacrificed", "creatures", "toughness"])
        | Some(["sacrificed", "creature", "toughness"])
        | Some(["sacrificed", "creatures", "toughness"])
        | Some(["the", "amassed", "army", "toughness"])
        | Some(["the", "amassed", "armys", "toughness"])
        | Some(["amassed", "army", "toughness"])
        | Some(["amassed", "armys", "toughness"])
        | Some(["the", "army", "you", "amassed", "toughness"])
        | Some(["army", "you", "amassed", "toughness"]) => {
            Some(Value::ToughnessOf(Box::new(tagged_it.clone())))
        }
        Some(["that", "spell", "mana", "value"])
        | Some(["that", "spells", "mana", "value"])
        | Some(["that", "card", "mana", "value"])
        | Some(["that", "cards", "mana", "value"])
        | Some(["the", "sacrificed", "creature", "mana", "value"])
        | Some(["the", "sacrificed", "creatures", "mana", "value"])
        | Some(["sacrificed", "creature", "mana", "value"])
        | Some(["sacrificed", "creatures", "mana", "value"])
        | Some(["the", "amassed", "army", "mana", "value"])
        | Some(["the", "amassed", "armys", "mana", "value"])
        | Some(["amassed", "army", "mana", "value"])
        | Some(["amassed", "armys", "mana", "value"])
        | Some(["the", "mana", "value", "of", "the", "amassed", "army"])
        | Some(["the", "mana", "value", "of", "the", "amassed", "armys"])
        | Some(["mana", "value", "of", "the", "amassed", "army"])
        | Some(["mana", "value", "of", "the", "amassed", "armys"])
        | Some(
            [
                "the",
                "mana",
                "value",
                "of",
                "the",
                "army",
                "you",
                "amassed",
            ],
        )
        | Some(["mana", "value", "of", "the", "army", "you", "amassed"]) => {
            Some(Value::ManaValueOf(Box::new(tagged_it)))
        }
        _ => None,
    }
}

pub(crate) fn parse_where_x_is_fixed_plus_reference_value(
    tokens: &[OwnedLexToken],
) -> Option<Value> {
    let clause_words = crate::cards::builders::parser::token_word_refs(tokens);
    if !etb_word_slice_starts_with(&clause_words, &["where", "x", "is"]) {
        return None;
    }

    let fixed_value = parse_number_word_i32(*clause_words.get(3)?)?;
    if fixed_value < 0 {
        return None;
    }
    let plus_word_idx = 4usize;
    if clause_words.get(plus_word_idx).copied() != Some("plus") {
        return None;
    }

    let value_words = clause_words.get(plus_word_idx + 1..)?;
    let reference_value =
        if etb_word_slice_starts_with(value_words, &["the", "sacrificed", "creature", "power"])
            || etb_word_slice_starts_with(value_words, &["the", "sacrificed", "creatures", "power"])
            || etb_word_slice_starts_with(value_words, &["sacrificed", "creature", "power"])
            || etb_word_slice_starts_with(value_words, &["sacrificed", "creatures", "power"])
        {
            Value::PowerOf(Box::new(ChooseSpec::Tagged(TagKey::from(IT_TAG))))
        } else if etb_word_slice_starts_with(
            value_words,
            &["the", "sacrificed", "creature", "toughness"],
        ) || etb_word_slice_starts_with(
            value_words,
            &["the", "sacrificed", "creatures", "toughness"],
        ) || etb_word_slice_starts_with(
            value_words,
            &["sacrificed", "creature", "toughness"],
        ) || etb_word_slice_starts_with(
            value_words,
            &["sacrificed", "creatures", "toughness"],
        ) {
            Value::ToughnessOf(Box::new(ChooseSpec::Tagged(TagKey::from(IT_TAG))))
        } else if etb_word_slice_starts_with(
            value_words,
            &[
                "the",
                "mana",
                "value",
                "of",
                "the",
                "sacrificed",
                "creature",
            ],
        ) || etb_word_slice_starts_with(
            value_words,
            &[
                "the",
                "mana",
                "value",
                "of",
                "the",
                "sacrificed",
                "creatures",
            ],
        ) || etb_word_slice_starts_with(
            value_words,
            &["mana", "value", "of", "the", "sacrificed", "creature"],
        ) || etb_word_slice_starts_with(
            value_words,
            &["mana", "value", "of", "the", "sacrificed", "creatures"],
        ) || etb_word_slice_starts_with(
            value_words,
            &["the", "sacrificed", "creature", "mana", "value"],
        ) || etb_word_slice_starts_with(
            value_words,
            &["the", "sacrificed", "creatures", "mana", "value"],
        ) || etb_word_slice_starts_with(
            value_words,
            &["sacrificed", "creature", "mana", "value"],
        ) || etb_word_slice_starts_with(
            value_words,
            &["sacrificed", "creatures", "mana", "value"],
        ) {
            Value::ManaValueOf(Box::new(ChooseSpec::Tagged(TagKey::from(IT_TAG))))
        } else {
            return None;
        };

    Some(Value::Add(
        Box::new(Value::Fixed(fixed_value)),
        Box::new(reference_value),
    ))
}

pub(crate) fn parse_where_x_life_gained_this_turn_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    let words = crate::cards::builders::parser::token_word_refs(tokens);
    if !etb_word_slice_starts_with(&words, &["where", "x", "is"]) {
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

pub(crate) fn parse_where_x_life_lost_this_turn_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    let words = crate::cards::builders::parser::token_word_refs(tokens);
    if !etb_word_slice_starts_with(&words, &["where", "x", "is"]) {
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

pub(crate) fn parse_where_x_opponents_dealt_combat_damage_this_turn_value(
    tokens: &[OwnedLexToken],
) -> Option<Value> {
    let words = crate::cards::builders::parser::token_word_refs(tokens);
    if !etb_word_slice_starts_with(&words, &["where", "x", "is"]) {
        return None;
    }
    match words.get(3..) {
        Some(
            [
                "the",
                "number",
                "of",
                "opponents",
                "that",
                "were",
                "dealt",
                "combat",
                "damage",
                "this",
                "turn",
            ],
        )
        | Some(
            [
                "number",
                "of",
                "opponents",
                "that",
                "were",
                "dealt",
                "combat",
                "damage",
                "this",
                "turn",
            ],
        ) => Some(Value::CountPlayers(PlayerFilter::Opponent)),
        _ => None,
    }
}

pub(crate) fn parse_where_x_noncombat_damage_to_opponents_value(
    tokens: &[OwnedLexToken],
) -> Option<Value> {
    let words = crate::cards::builders::parser::token_word_refs(tokens);
    if !etb_word_slice_starts_with(&words, &["where", "x", "is"]) {
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

pub(crate) fn parse_where_x_is_aggregate_filter_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    let clause_words = crate::cards::builders::parser::token_word_refs(tokens);
    if !etb_word_slice_starts_with(&clause_words, &["where", "x", "is"]) {
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
    let filter_words = crate::cards::builders::parser::token_word_refs(filter_tokens);
    let should_try_split = etb_word_slice_contains_all(&filter_words, &["and", "graveyard"])
        && filter_words
            .iter()
            .any(|word| matches!(*word, "control" | "controls" | "own" | "owns"));
    let filter = (if should_try_split {
        let segments =
            crate::cards::builders::parser::grammar::primitives::split_lexed_slices_on_and(
                filter_tokens,
            );
        let mut branches = Vec::new();
        for segment in segments {
            let trimmed = trim_commas(segment);
            if trimmed.is_empty() {
                return None;
            }
            branches.push(parse_object_filter(&trimmed, false).ok()?);
        }
        if branches.len() < 2 {
            return None;
        }
        let mut combined = ObjectFilter::default();
        combined.any_of = branches;
        Some(combined)
    } else {
        None
    })
    .or_else(|| parse_object_filter(filter_tokens, false).ok())?;

    match (aggregate, value_kind) {
        ("total", "power") => Some(Value::TotalPower(filter)),
        ("total", "toughness") => Some(Value::TotalToughness(filter)),
        ("total", "mana_value") => Some(Value::TotalManaValue(filter)),
        ("greatest", "power") => Some(Value::GreatestPower(filter)),
        ("greatest", "toughness") => Some(Value::GreatestToughness(filter)),
        ("greatest", "mana_value") => Some(Value::GreatestManaValue(filter)),
        _ => None,
    }
}

pub(crate) fn parse_where_x_greatest_commander_mana_value(
    tokens: &[OwnedLexToken],
    commander_start_word_idx: usize,
) -> Option<Value> {
    let commander_start_token_idx = token_index_for_word_index(tokens, commander_start_word_idx)?;
    let commander_words =
        crate::cards::builders::parser::token_word_refs(&tokens[commander_start_token_idx..]);
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
    tokens: &[OwnedLexToken],
) -> Option<Value> {
    let clause_words = crate::cards::builders::parser::token_word_refs(tokens);
    if !etb_word_slice_starts_with(&clause_words, &["where", "x", "is"]) {
        return None;
    }

    let number_idx = etb_word_offset(&clause_words, |word| word == "number")?;
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

pub(crate) fn parse_where_x_is_number_of_filter_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    let clause_words = crate::cards::builders::parser::token_word_refs(tokens);
    if !etb_word_slice_starts_with(&clause_words, &["where", "x", "is"]) {
        return None;
    }

    if etb_word_slice_contains_all(&clause_words, &["creature", "type", "common"]) {
        return None;
    }

    let number_idx = etb_word_offset(&clause_words, |word| word == "number")?;
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
    let filter_words = crate::cards::builders::parser::token_word_refs(filter_tokens);
    if let Some(value) = parse_number_of_counters_on_source_value(&filter_words) {
        return Some(value);
    }
    if etb_word_slice_starts_with(&filter_words, &["basic", "land", "type", "among"])
        || etb_word_slice_starts_with(&filter_words, &["basic", "land", "types", "among"])
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
    if etb_word_slice_starts_with(&filter_words, &["color", "among"])
        || etb_word_slice_starts_with(&filter_words, &["colors", "among"])
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
    if (etb_word_slice_starts_with(&filter_words, &["card", "type", "among", "cards"])
        || etb_word_slice_starts_with(&filter_words, &["card", "types", "among", "cards"]))
        && etb_word_slice_contains(&filter_words, "graveyard")
    {
        let player = if etb_has_word_sequence(&filter_words, &["your", "graveyard"]) {
            PlayerFilter::You
        } else if etb_has_word_sequence(&filter_words, &["opponents", "graveyard"])
            || etb_has_word_sequence(&filter_words, &["opponent", "graveyard"])
        {
            PlayerFilter::Opponent
        } else {
            PlayerFilter::You
        };
        return Some(Value::CardTypesInGraveyard(player));
    }
    let filter = parse_object_filter(filter_tokens, false).ok()?;
    Some(Value::Count(filter))
}

fn parse_number_of_counters_on_source_value(filter_words: &[&str]) -> Option<Value> {
    let mut idx = 0usize;
    if filter_words
        .get(idx)
        .is_some_and(|word| is_article(word) || *word == "one")
    {
        idx += 1;
    }
    let counter_word = *filter_words.get(idx)?;
    let counter_type = parse_counter_type_word(counter_word).or_else(|| {
        counter_word
            .chars()
            .all(|ch| ch.is_ascii_alphabetic())
            .then_some(CounterType::Named(intern_counter_name(counter_word)))
    })?;
    idx += 1;
    if !matches!(filter_words.get(idx).copied(), Some("counter" | "counters")) {
        return None;
    }
    idx += 1;
    if filter_words.get(idx).copied() != Some("on") {
        return None;
    }
    idx += 1;
    match filter_words.get(idx..) {
        Some(["it"])
        | Some(["this"])
        | Some(["this", "card"])
        | Some(["this", "creature"])
        | Some(["this", "permanent"])
        | Some(["this", "source"])
        | Some(["this", "artifact"])
        | Some(["this", "land"])
        | Some(["this", "enchantment"])
        | Some(["thiss"])
        | Some(["thiss", "card"])
        | Some(["thiss", "creature"])
        | Some(["thiss", "permanent"])
        | Some(["thiss", "source"])
        | Some(["thiss", "artifact"])
        | Some(["this", "equipment"])
        | Some(["thiss", "land"])
        | Some(["thiss", "enchantment"])
        | Some(["thiss", "equipment"]) => Some(Value::CountersOnSource(counter_type)),
        _ => None,
    }
}

pub(crate) fn parse_where_x_is_fixed_plus_number_of_filter_value(
    tokens: &[OwnedLexToken],
) -> Option<Value> {
    let clause_words = crate::cards::builders::parser::token_word_refs(tokens);
    if !etb_word_slice_starts_with(&clause_words, &["where", "x", "is"]) {
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
    let filter_words = crate::cards::builders::parser::token_word_refs(filter_tokens);
    if etb_word_slice_starts_with(&filter_words, &["basic", "land", "type", "among"])
        || etb_word_slice_starts_with(&filter_words, &["basic", "land", "types", "among"])
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
    if etb_word_slice_starts_with(&filter_words, &["color", "among"])
        || etb_word_slice_starts_with(&filter_words, &["colors", "among"])
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
    tokens: &[OwnedLexToken],
) -> Option<Value> {
    let clause_words = crate::cards::builders::parser::token_word_refs(tokens);
    if !etb_word_slice_starts_with(&clause_words, &["where", "x", "is"]) {
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
    let operator_word_idx = etb_word_offset(&clause_words[filter_start_word_idx + 1..], |word| {
        matches!(word, "plus" | "minus")
    })
    .map(|idx| filter_start_word_idx + 1 + idx)?;
    let operator = clause_words[operator_word_idx];

    let filter_start_token_idx = token_index_for_word_index(tokens, filter_start_word_idx)?;
    let operator_token_idx = token_index_for_word_index(tokens, operator_word_idx)?;
    let filter_tokens = trim_commas(&tokens[filter_start_token_idx..operator_token_idx]);
    let filter = parse_object_filter(&filter_tokens, false).ok()?;
    let filter_words = crate::cards::builders::parser::token_word_refs(&filter_tokens);
    let count_value = if etb_word_slice_contains_all(&filter_words, &["cards", "in", "your"])
        && etb_word_slice_contains_any(&filter_words, &["hand", "hands"])
    {
        Value::CardsInHand(PlayerFilter::You)
    } else {
        Value::Count(filter)
    };

    let offset_start_token_idx = token_index_for_word_index(tokens, operator_word_idx + 1)?;
    let offset_tokens = trim_commas(&tokens[offset_start_token_idx..]);
    let (offset_value, used) = parse_number(&offset_tokens)?;
    let trailing_words = crate::cards::builders::parser::token_word_refs(&offset_tokens[used..]);
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

pub(crate) fn token_index_for_word_index(
    tokens: &[OwnedLexToken],
    word_index: usize,
) -> Option<usize> {
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
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = crate::cards::builders::parser::token_word_refs(tokens);
    if matches!(
        clause_words.first().copied(),
        Some("if" | "when" | "whenever" | "as")
    ) {
        return Ok(None);
    }
    if is_negated_untap_clause(&clause_words) {
        let has_enters_tapped = etb_word_slice_contains_any(&clause_words, &["enter", "enters"]);
        let has_tapped = etb_word_slice_contains(&clause_words, "tapped");
        if has_enters_tapped && has_tapped {
            return Err(CardTextError::ParseError(format!(
                "unsupported mixed enters-tapped and negated-untap clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        return Ok(None);
    }
    if etb_word_slice_contains(&clause_words, "unless") {
        return Ok(None);
    }
    let enter_word_idx = etb_word_offset(&clause_words, |word| matches!(word, "enter" | "enters"));
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
    if etb_word_slice_contains(&clause_words, "copy") {
        return Err(CardTextError::ParseError(format!(
            "unsupported enters-as-copy replacement clause (clause: '{}') [rule=enters-as-copy]",
            clause_words.join(" ")
        )));
    }
    let before_enter = &tokens[..enter_token_idx];
    let before_word_view =
        crate::cards::builders::parser::grammar::primitives::TokenWordView::new(before_enter);
    let before_words = before_word_view.word_refs();
    let mut controller_override: Option<PlayerFilter> = None;
    let mut filter_end = before_enter.len();
    let find_suffix_cut = |suffix_len: usize| {
        let keep_word_count = before_words.len().saturating_sub(suffix_len);
        if keep_word_count == 0 {
            0
        } else {
            before_word_view
                .token_start_indices()
                .get(keep_word_count)
                .copied()
                .unwrap_or(before_enter.len())
        }
    };
    if etb_word_slice_ends_with(&before_words, &["played", "by", "your", "opponents"]) {
        controller_override = Some(PlayerFilter::Opponent);
        filter_end = find_suffix_cut(4);
    } else if etb_word_slice_ends_with(&before_words, &["played", "by", "an", "opponent"])
        || etb_word_slice_ends_with(&before_words, &["played", "by", "a", "opponent"])
    {
        controller_override = Some(PlayerFilter::Opponent);
        filter_end = find_suffix_cut(4);
    } else if etb_word_slice_ends_with(&before_words, &["played", "by", "opponents"]) {
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
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = crate::cards::builders::parser::token_word_refs(tokens);
    if matches!(
        clause_words.first().copied(),
        Some("if" | "when" | "whenever" | "as")
    ) {
        return Ok(None);
    }
    if etb_word_slice_contains(&clause_words, "unless")
        || clause_words.first().copied() == Some("this")
    {
        return Ok(None);
    }

    let Some(enter_word_idx) =
        etb_word_offset(&clause_words, |word| matches!(word, "enter" | "enters"))
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
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = crate::cards::builders::parser::token_word_refs(tokens);
    if !etb_word_slice_starts_with(&clause_words, &["as", "this", "land", "enters"]) {
        return Ok(None);
    }
    if !etb_word_slice_contains_all(&clause_words, &["reveal", "from", "hand"]) {
        return Ok(None);
    }

    let Some(reveal_word_idx) = etb_word_offset(&clause_words, |word| word == "reveal") else {
        return Err(CardTextError::ParseError(format!(
            "missing 'reveal' keyword in land ETB reveal clause (clause: '{}')",
            clause_words.join(" ")
        )));
    };
    let Some(from_hand_word_idx) = etb_find_word_sequence_index(
        &clause_words[reveal_word_idx + 1..],
        &["from", "your", "hand"],
    )
    .map(|idx| reveal_word_idx + 1 + idx) else {
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
    if let Some(if_you_dont_idx) =
        etb_find_word_sequence_index(&clause_words, &["if", "you", "dont"])
            .or_else(|| etb_find_word_sequence_index(&clause_words, &["if", "you", "don't"]))
    {
        let trailing = &clause_words[if_you_dont_idx + 3..];
        let valid_trailing =
            etb_word_slice_starts_with(trailing, &["this", "land", "enters", "tapped"])
                || etb_word_slice_starts_with(trailing, &["this", "land", "enter", "tapped"])
                || etb_word_slice_starts_with(trailing, &["it", "enters", "tapped"])
                || etb_word_slice_starts_with(trailing, &["it", "enter", "tapped"])
                || etb_word_slice_starts_with(
                    trailing,
                    &["it", "enters", "the", "battlefield", "tapped"],
                )
                || etb_word_slice_starts_with(
                    trailing,
                    &["it", "enter", "the", "battlefield", "tapped"],
                );
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
    let Some(unless_idx) = etb_word_offset(&clause_words, |word| word == "unless") else {
        return Err(CardTextError::ParseError(format!(
            "unsupported land ETB reveal clause (expected 'if you don't' or 'unless') (clause: '{}')",
            clause_words.join(" ")
        )));
    };
    let before_unless = &clause_words[..unless_idx];
    if !etb_has_word_sequence(before_unless, &["enters", "tapped"])
        && !etb_has_word_sequence(before_unless, &["enter", "tapped"])
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported land ETB reveal unless-prefix (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let mut condition = reveal_condition;
    if let Some(or_idx_rel) = etb_word_offset(&clause_words[unless_idx + 1..], |word| word == "or")
    {
        let or_idx = unless_idx + 1 + or_idx_rel;
        let Some(control_word_idx) = etb_word_offset(&clause_words[or_idx + 1..], |word| {
            matches!(word, "control" | "controls")
        })
        .map(|idx| or_idx + 1 + idx) else {
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
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = crate::cards::builders::parser::token_word_refs(tokens);
    if !etb_word_slice_contains_any(&clause_words, &["enters", "enter"]) {
        return Ok(None);
    }
    if !etb_word_slice_contains_all(&clause_words, &["tapped", "unless"]) {
        return Ok(None);
    }

    let Some(unless_idx) = etb_find_token_index(tokens, |token| token.is_word("unless")) else {
        return Ok(None);
    };
    let condition_words =
        crate::cards::builders::parser::token_word_refs(&tokens[unless_idx + 1..]);
    if etb_word_slice_starts_with(
        &condition_words,
        &["you", "control", "two", "or", "more", "other", "lands"],
    ) {
        return Ok(Some(
            StaticAbility::enters_tapped_unless_control_two_or_more_other_lands(),
        ));
    }
    if etb_word_slice_starts_with(
        &condition_words,
        &["you", "control", "two", "or", "fewer", "other", "lands"],
    ) {
        return Ok(Some(
            StaticAbility::enters_tapped_unless_control_two_or_fewer_other_lands(),
        ));
    }
    if etb_word_slice_starts_with(
        &condition_words,
        &["you", "control", "two", "or", "more", "basic", "lands"],
    ) {
        return Ok(Some(
            StaticAbility::enters_tapped_unless_control_two_or_more_basic_lands(),
        ));
    }
    if etb_word_slice_starts_with(
        &condition_words,
        &["a", "player", "has", "13", "or", "less", "life"],
    ) || etb_word_slice_starts_with(
        &condition_words,
        &["a", "player", "has", "thirteen", "or", "less", "life"],
    ) {
        return Ok(Some(
            StaticAbility::enters_tapped_unless_a_player_has_13_or_less_life(),
        ));
    }
    if etb_word_slice_starts_with(
        &condition_words,
        &["you", "have", "two", "or", "more", "opponents"],
    ) {
        return Ok(Some(
            StaticAbility::enters_tapped_unless_two_or_more_opponents(),
        ));
    }
    if etb_word_slice_starts_with(
        &condition_words,
        &[
            "it", "s", "your", "first", "second", "or", "third", "turn", "of", "the", "game",
        ],
    ) || etb_word_slice_starts_with(
        &condition_words,
        &[
            "it's", "your", "first", "second", "or", "third", "turn", "of", "the", "game",
        ],
    ) {
        return Ok(Some(StaticAbility::enters_tapped_unless_condition(
            crate::ConditionExpr::YourFirstTurnsOfTheGameOrFewer(3),
            clause_words.join(" "),
        )));
    }

    // Generic: "unless you control <object filter>" (covers Mount/Vehicle, etc.).
    if etb_word_slice_starts_with(&condition_words, &["you", "control"])
        || etb_word_slice_starts_with(&condition_words, &["you", "controls"])
    {
        let control_idx = etb_find_token_index(&tokens[unless_idx + 1..], |token| {
            token.is_word("control") || token.is_word("controls")
        })
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
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = crate::cards::builders::parser::token_word_refs(tokens);
    let enter_word_idx = etb_word_offset(&clause_words, |word| matches!(word, "enter" | "enters"));
    let Some(enter_word_idx) = enter_word_idx else {
        return Ok(None);
    };
    let Some(enter_token_idx) = token_index_for_word_index(tokens, enter_word_idx) else {
        return Ok(None);
    };
    if tokens[..enter_token_idx]
        .iter()
        .any(|token| token.is_period() || token.is_colon() || token.is_semicolon())
    {
        return Ok(None);
    }

    let subject_tokens = trim_commas(&tokens[..enter_token_idx]);
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    let subject_words = crate::cards::builders::parser::token_word_refs(&subject_tokens);
    if is_source_reference_words(&subject_words) {
        return Ok(None);
    }
    if matches!(
        subject_words.first().copied(),
        Some("if" | "when" | "whenever" | "as" | "at")
    ) {
        return Ok(None);
    }

    if !etb_word_slice_contains_all(&clause_words, &["with", "additional"])
        || !clause_words
            .iter()
            .any(|word| *word == "counter" || *word == "counters")
    {
        return Ok(None);
    }

    let Ok(filter) = parse_object_filter(&subject_tokens, false) else {
        return Ok(None);
    };

    let and_as_idx = etb_find_token_word_sequence_index(tokens, &["and", "as"]);
    let base_tokens = and_as_idx.map_or(tokens, |idx| &tokens[..idx]);

    let additional_idx = etb_find_token_index(base_tokens, |token| token.is_word("additional"))
        .ok_or_else(|| {
            CardTextError::ParseError("missing 'additional' keyword for ETB counters".to_string())
        })?;
    let count = if let Some(equal_idx) =
        etb_find_token_index(base_tokens, |token| token.is_word("equal"))
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
        if let Some(first) = addition_tokens.first() {
            addition_tokens[0] = OwnedLexToken::word("is".to_string(), first.span());
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

    Ok(Some(
        StaticAbility::enters_with_counters_and_subtypes_for_filter(
            filter,
            counter_type,
            count,
            added_subtypes,
        ),
    ))
}
