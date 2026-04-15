use super::*;

pub(crate) fn parse_become(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let Some(SubjectAst::Player(player)) = subject else {
        return Err(CardTextError::ParseError(format!(
            "unsupported become clause (clause: '{}')",
            crate::cards::builders::compiler::token_word_refs(tokens).join(" ")
        )));
    };

    let clause_words = crate::cards::builders::compiler::token_word_refs(tokens);
    if clause_words.as_slice() == ["the", "monarch"] || clause_words.as_slice() == ["monarch"] {
        return Ok(EffectAst::BecomeMonarch { player });
    }

    let amount = parse_value(tokens)
        .map(|(value, _)| value)
        .or_else(|| parse_half_starting_life_total_value(tokens, player))
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing life total amount (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
    Ok(EffectAst::SetLifeTotal { amount, player })
}

pub(crate) fn parse_switch(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    use crate::effect::Until;

    let clause_words = crate::cards::builders::compiler::token_word_refs(tokens);

    // Split off trailing duration, if present.
    let (duration, remainder) =
        if let Some((duration, remainder)) = parse_restriction_duration(tokens)? {
            (duration, remainder)
        } else {
            (Until::EndOfTurn, trim_commas(tokens).to_vec())
        };

    let Some(power_idx) = find_index(&remainder, |token| token.is_word("power")) else {
        return Err(CardTextError::ParseError(format!(
            "unsupported switch clause (clause: '{}')",
            clause_words.join(" ")
        )));
    };

    // Target phrase is everything up to "power".
    let target_tokens = &remainder[..power_idx];
    let target_words = crate::cards::builders::compiler::token_word_refs(target_tokens);
    let target = if target_words.is_empty()
        || matches!(
            target_words.as_slice(),
            ["this"]
                | ["this", "creature"]
                | ["this", "creatures"]
                | ["this", "permanent"]
                | ["it"]
        ) {
        if target_words == ["it"] {
            TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(target_tokens))
        } else {
            TargetAst::Source(span_from_tokens(target_tokens))
        }
    } else {
        parse_target_phrase(target_tokens)?
    };

    // Require "... power and toughness ..." somewhere in remainder.
    if !grammar::contains_word(&remainder, "power")
        || !grammar::contains_word(&remainder, "toughness")
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported switch clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    Ok(EffectAst::SwitchPowerToughness { target, duration })
}

pub(crate) fn parse_skip(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let clause_words = crate::cards::builders::compiler::token_word_refs(tokens);
    let (player, words) = match subject {
        Some(SubjectAst::Player(player)) => (player, clause_words),
        _ => {
            if let Some((prefix, _)) = grammar::words_match_any_prefix(tokens, YOUR_PREFIXES) {
                (PlayerAst::You, clause_words[prefix.len()..].to_vec())
            } else if let Some((prefix, _)) =
                grammar::words_match_any_prefix(tokens, THEIR_PREFIXES)
            {
                (PlayerAst::That, clause_words[prefix.len()..].to_vec())
            } else if let Some((prefix, _)) =
                grammar::words_match_any_prefix(tokens, THAT_PLAYER_PREFIXES)
            {
                (PlayerAst::That, clause_words[prefix.len()..].to_vec())
            } else if let Some((prefix, _)) =
                grammar::words_match_any_prefix(tokens, TARGET_PLAYER_PREFIXES)
            {
                (PlayerAst::Target, clause_words[prefix.len()..].to_vec())
            } else if let Some((prefix, _)) =
                grammar::words_match_any_prefix(tokens, TARGET_OPPONENT_PREFIXES)
            {
                (
                    PlayerAst::TargetOpponent,
                    clause_words[prefix.len()..].to_vec(),
                )
            } else if grammar::words_match_any_prefix(tokens, TURN_PREFIXES).is_some() {
                (PlayerAst::Implicit, clause_words)
            } else {
                return Err(CardTextError::ParseError(format!(
                    "unsupported skip clause (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
        }
    };

    let skips_next_combat_phase_this_turn = slice_contains(&words, &"combat")
        && slice_contains(&words, &"phase")
        && slice_contains(&words, &"next")
        && slice_contains(&words, &"this")
        && slice_contains(&words, &"turn");
    if skips_next_combat_phase_this_turn {
        return Ok(EffectAst::SkipNextCombatPhaseThisTurn { player });
    }
    if slice_contains(&words, &"combat")
        && (slice_contains(&words, &"phase") || slice_contains(&words, &"phases"))
        && slice_contains(&words, &"turn")
    {
        return Ok(EffectAst::SkipCombatPhases { player });
    }
    if slice_contains(&words, &"draw") && slice_contains(&words, &"step") {
        return Ok(EffectAst::SkipDrawStep { player });
    }
    if slice_contains(&words, &"turn") {
        return Ok(EffectAst::SkipTurn { player });
    }

    Err(CardTextError::ParseError(format!(
        "unsupported skip clause (clause: '{}')",
        words.join(" ")
    )))
}

pub(crate) fn parse_flip(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let player = match subject.unwrap_or(SubjectAst::This) {
        SubjectAst::Player(player) => player,
        SubjectAst::This => PlayerAst::Implicit,
    };
    if tokens.is_empty() {
        return Ok(EffectAst::Flip {
            target: TargetAst::Source(None),
        });
    }

    let target_words = crate::cards::builders::compiler::token_word_refs(tokens);
    if matches!(target_words.as_slice(), ["a", "coin"] | ["coin"]) {
        return Ok(EffectAst::FlipCoin { player });
    }
    if target_words == ["it"]
        || target_words == ["this"]
        || target_words == ["this", "creature"]
        || target_words == ["this", "permanent"]
    {
        return Ok(EffectAst::Flip {
            target: TargetAst::Source(span_from_tokens(tokens)),
        });
    }

    let target = parse_target_phrase(tokens)?;
    Ok(EffectAst::Flip { target })
}

pub(crate) fn parse_roll(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let player = match subject.unwrap_or(SubjectAst::This) {
        SubjectAst::Player(player) => player,
        SubjectAst::This => PlayerAst::Implicit,
    };
    let mut die_tokens = tokens;
    if die_tokens
        .first()
        .is_some_and(|token| token.is_word("a") || token.is_word("an"))
    {
        die_tokens = &die_tokens[1..];
    }
    let Some(die_word) = die_tokens.first().and_then(OwnedLexToken::as_word) else {
        return Err(CardTextError::ParseError(
            "roll clause missing die size".to_string(),
        ));
    };
    let die_word = die_word.to_ascii_lowercase();
    let Some(sides) = die_word
        .strip_prefix('d')
        .and_then(|sides| sides.parse::<u32>().ok())
    else {
        return Err(CardTextError::ParseError(format!(
            "unsupported roll clause (clause: '{}')",
            crate::cards::builders::compiler::token_word_refs(tokens).join(" ")
        )));
    };
    Ok(EffectAst::RollDie { player, sides })
}

pub(crate) fn parse_regenerate(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    let words = crate::cards::builders::compiler::token_word_refs(tokens);
    if matches!(words.first().copied(), Some("all" | "each")) {
        if tokens.len() < 2 {
            return Err(CardTextError::ParseError(
                "regenerate clause missing filter after each/all".to_string(),
            ));
        }
        let filter = parse_object_filter(&tokens[1..], false)?;
        return Ok(EffectAst::RegenerateAll { filter });
    }
    let target = parse_target_phrase(tokens)?;
    Ok(EffectAst::Regenerate { target })
}

pub(crate) fn parse_mill(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let clause_words = crate::cards::builders::compiler::token_word_refs(tokens);
    let starts_with_card_keyword = tokens
        .first()
        .and_then(OwnedLexToken::as_word)
        .is_some_and(|word| word == "card" || word == "cards");

    let (count, used) =
        if let Some((prefix, _)) = grammar::words_match_any_prefix(tokens, THAT_MANY_PREFIXES) {
            (Value::EventValue(EventValueSpec::Amount), prefix.len())
        } else if starts_with_card_keyword {
            if let Some((count, used_after_cards)) = parse_value(&tokens[1..]) {
                (count, 1 + used_after_cards)
            } else if let Some(count) = parse_add_mana_equal_amount_value(&tokens[1..]) {
                // Mill clauses like "cards equal to its toughness" place the amount after "cards".
                (count, tokens.len())
            } else {
                return Err(CardTextError::ParseError(format!(
                    "missing mill count (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
        } else {
            parse_value(tokens).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing mill count (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?
        };

    let rest = &tokens[used..];
    if starts_with_card_keyword {
        let trailing_words: Vec<&str> = rest.iter().filter_map(OwnedLexToken::as_word).collect();
        if !trailing_words.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing mill clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
    } else {
        if rest
            .first()
            .and_then(OwnedLexToken::as_word)
            .is_some_and(|word| word != "card" && word != "cards")
        {
            return Err(CardTextError::ParseError(
                "missing card keyword".to_string(),
            ));
        }
        let trailing_words: Vec<&str> = rest
            .iter()
            .skip(1)
            .filter_map(OwnedLexToken::as_word)
            .collect();
        if !trailing_words.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing mill clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
    }

    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);

    Ok(EffectAst::Mill { count, player })
}

pub(crate) fn parse_get(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    fn parse_pump_for_each_tail(
        tail_tokens: &[OwnedLexToken],
        subject: Option<SubjectAst>,
        power_per: i32,
        toughness_per: i32,
        clause_words: &[&str],
    ) -> Result<Option<EffectAst>, CardTextError> {
        if grammar::words_match_prefix(tail_tokens, &["until", "end", "of", "turn", "for", "each"])
            .is_none()
        {
            return Ok(None);
        }

        let count = parse_get_for_each_count_value(&tail_tokens[4..])?.ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported get-for-each filter (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
        let target = match subject {
            Some(SubjectAst::This) => TargetAst::Source(None),
            _ => {
                return Err(CardTextError::ParseError(
                    "unsupported get clause (missing subject)".to_string(),
                ));
            }
        };
        Ok(Some(EffectAst::PumpForEach {
            power_per,
            toughness_per,
            target,
            count,
            duration: Until::EndOfTurn,
        }))
    }

    let clause_words = crate::cards::builders::compiler::token_word_refs(tokens);
    if grammar::contains_word(tokens, "poison")
        && (grammar::contains_word(tokens, "counter") || grammar::contains_word(tokens, "counters"))
    {
        let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);
        let count = if matches!(
            clause_words.first().copied(),
            Some("a" | "an" | "another" | "one")
        ) {
            Value::Fixed(1)
        } else {
            parse_value(tokens)
                .map(|(value, _)| value)
                .unwrap_or(Value::Fixed(1))
        };
        return Ok(EffectAst::PoisonCounters { count, player });
    }

    let energy_count = tokens
        .iter()
        .filter(|token| {
            token.is_word("e")
                || (token.kind == TokenKind::ManaGroup
                    && token
                        .slice
                        .trim_start_matches('{')
                        .trim_end_matches('}')
                        .eq_ignore_ascii_case("e"))
        })
        .count();
    if energy_count > 0 {
        let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);
        let count = parse_add_mana_equal_amount_value(tokens)
            .or(parse_equal_to_number_of_filter_value(tokens))
            .or(parse_dynamic_cost_modifier_value(tokens)?)
            .unwrap_or(Value::Fixed(energy_count as i32));
        return Ok(EffectAst::EnergyCounters { count, player });
    }

    if clause_words.as_slice() == ["tk"] {
        let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);
        return Ok(EffectAst::EnergyCounters {
            count: Value::Fixed(1),
            player,
        });
    }

    if let Some((prefix, _)) = grammar::words_match_any_prefix(tokens, EMBLEM_WITH_PREFIXES) {
        let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);
        let text_words = &clause_words[prefix.len()..];
        if text_words.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing emblem text (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let text = if slice_starts_with(&text_words, &["at", "the", "beginning", "of"])
            && let Some(this_idx) = find_index(&text_words, |word| *word == "this")
        {
            let head = text_words[..this_idx].join(" ");
            let tail = text_words[this_idx..].join(" ");
            format!(
                "{}{}, {}.",
                head[..1].to_ascii_uppercase(),
                &head[1..],
                tail
            )
        } else {
            let joined = text_words.join(" ");
            format!("{}{}.", joined[..1].to_ascii_uppercase(), &joined[1..])
        };
        return Ok(EffectAst::CreateEmblem { player, text });
    }

    let modifier_start =
        if let Some((prefix, _)) = grammar::words_match_any_prefix(tokens, ADDITIONAL_PREFIXES) {
            prefix.len()
        } else {
            0usize
        };
    if modifier_start > 0
        && let Some(mod_token) = tokens.get(modifier_start).and_then(OwnedLexToken::as_word)
        && let Ok((power_per, toughness_per)) = parse_pt_modifier(mod_token)
    {
        let tail_tokens = tokens.get(modifier_start + 1..).unwrap_or_default();
        if let Some(effect) = parse_pump_for_each_tail(
            tail_tokens,
            subject,
            power_per,
            toughness_per,
            &clause_words,
        )? {
            return Ok(effect);
        }
    }

    if let Some(mod_token) = tokens.first().and_then(OwnedLexToken::as_word)
        && let Ok((power, toughness)) = parse_pt_modifier_values(mod_token)
    {
        if let (Value::Fixed(power_per), Value::Fixed(toughness_per)) = (&power, &toughness)
            && let Some(effect) = parse_pump_for_each_tail(
                tokens.get(1..).unwrap_or_default(),
                subject,
                *power_per,
                *toughness_per,
                &clause_words,
            )?
        {
            return Ok(effect);
        }
        let (power, toughness, duration, condition) =
            parse_get_modifier_values_with_tail(tokens, power, toughness)?;
        let target = match subject {
            Some(SubjectAst::This) => TargetAst::Source(None),
            _ => {
                return Err(CardTextError::ParseError(
                    "unsupported get clause (missing subject)".to_string(),
                ));
            }
        };
        return Ok(EffectAst::Pump {
            power,
            toughness,
            target,
            duration,
            condition,
        });
    }

    if let Some(collapsed_tokens) = collapse_leading_signed_pt_modifier_tokens(tokens)
        && let Some(mod_token) = collapsed_tokens.first().and_then(OwnedLexToken::as_word)
        && let Ok((power, toughness)) = parse_pt_modifier_values(mod_token)
    {
        if let (Value::Fixed(power_per), Value::Fixed(toughness_per)) = (&power, &toughness)
            && let Some(effect) = parse_pump_for_each_tail(
                collapsed_tokens.get(1..).unwrap_or_default(),
                subject,
                *power_per,
                *toughness_per,
                &clause_words,
            )?
        {
            return Ok(effect);
        }
        let (power, toughness, duration, condition) =
            parse_get_modifier_values_with_tail(&collapsed_tokens, power, toughness)?;
        let target = match subject {
            Some(SubjectAst::This) => TargetAst::Source(None),
            _ => {
                return Err(CardTextError::ParseError(
                    "unsupported get clause (missing subject)".to_string(),
                ));
            }
        };
        return Ok(EffectAst::Pump {
            power,
            toughness,
            target,
            duration,
            condition,
        });
    }

    Err(CardTextError::ParseError(format!(
        "unsupported get clause (clause: '{}')",
        clause_words.join(" ")
    )))
}

pub(crate) fn parse_untap(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    if tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "untap clause missing target".to_string(),
        ));
    }
    let words = crate::cards::builders::compiler::token_word_refs(tokens);
    if matches!(words.first().copied(), Some("all" | "each")) {
        let filter = parse_object_filter(&tokens[1..], false)?;
        return Ok(EffectAst::UntapAll { filter });
    }
    if words.as_slice() == ["them"] {
        let mut filter = ObjectFilter::default();
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: IT_TAG.into(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
        return Ok(EffectAst::UntapAll { filter });
    }
    let target = parse_target_phrase(tokens)?;
    Ok(EffectAst::Untap { target })
}

pub(crate) fn parse_scry(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let (count, _) = parse_value(tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing scry count (clause: '{}')",
            crate::cards::builders::compiler::token_word_refs(tokens).join(" ")
        ))
    })?;

    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);

    Ok(EffectAst::Scry { count, player })
}

pub(crate) fn parse_surveil(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let (count, _) = parse_value(tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing surveil count (clause: '{}')",
            crate::cards::builders::compiler::token_word_refs(tokens).join(" ")
        ))
    })?;

    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);

    Ok(EffectAst::Surveil { count, player })
}

pub(crate) fn parse_pay(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);
    let energy_symbol_count = tokens
        .iter()
        .filter(|token| {
            token.is_word("e")
                || (token.kind == TokenKind::ManaGroup
                    && token
                        .slice
                        .trim_start_matches('{')
                        .trim_end_matches('}')
                        .eq_ignore_ascii_case("e"))
        })
        .count();

    let clause_words = crate::cards::builders::compiler::token_word_refs(tokens);
    if grammar::words_match_any_prefix(tokens, ANY_AMOUNT_OF_PREFIXES).is_some()
        && (grammar::contains_word(tokens, "e") || energy_symbol_count > 0)
    {
        return Ok(EffectAst::PayEnergy {
            amount: Value::Fixed(0),
            player,
        });
    }
    if clause_words.len() >= 4
        && grammar::contains_word(tokens, "for")
        && grammar::contains_word(tokens, "each")
        && let Ok(symbols) = parse_mana_symbol_group(clause_words[0])
    {
        return Ok(EffectAst::PayMana {
            cost: ManaCost::from_pips(vec![symbols]),
            player,
        });
    }

    if let Some((amount, used)) = parse_value(tokens)
        && tokens.get(used).is_some_and(|token| token.is_word("life"))
    {
        return Ok(EffectAst::LoseLife { amount, player });
    }
    if let Some((amount, used)) = parse_value(tokens)
        && tokens
            .get(used)
            .is_some_and(|token| token.is_word("energy"))
    {
        return Ok(EffectAst::PayEnergy { amount, player });
    }
    if energy_symbol_count > 0 {
        let mut energy_count = 0u32;
        for token in tokens {
            if token.kind == TokenKind::ManaGroup
                && token
                    .slice
                    .trim_start_matches('{')
                    .trim_end_matches('}')
                    .eq_ignore_ascii_case("e")
            {
                energy_count += 1;
                continue;
            }
            let Some(word) = token.as_word() else {
                continue;
            };
            if is_article(word)
                || word == "and"
                || word == "or"
                || word == "energy"
                || word == "counter"
                || word == "counters"
            {
                continue;
            }
            if word == "e" {
                energy_count += 1;
                continue;
            }
            return Err(CardTextError::ParseError(format!(
                "unsupported pay clause token '{word}' (clause: '{}')",
                crate::cards::builders::compiler::token_word_refs(tokens).join(" ")
            )));
        }
        if energy_count > 0 {
            return Ok(EffectAst::PayEnergy {
                amount: Value::Fixed(energy_count as i32),
                player,
            });
        }
    }

    let pips = {
        use winnow::prelude::*;
        let mut stream = LexStream::new(tokens);
        grammar::collect_mana_pip_groups
            .parse_next(&mut stream)
            .map_err(|_| {
                CardTextError::ParseError(format!(
                    "missing payment cost (clause: '{}')",
                    crate::cards::builders::compiler::token_word_refs(tokens).join(" ")
                ))
            })?
    };

    Ok(EffectAst::PayMana {
        cost: ManaCost::from_pips(pips),
        player,
    })
}
