pub(crate) fn strip_leading_trigger_intro(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    if tokens.first().is_some_and(|token| {
        token.is_word("when") || token.is_word("whenever") || token.is_word("at")
    }) {
        &tokens[1..]
    } else {
        tokens
    }
}

pub(crate) fn split_trigger_or_index(tokens: &[OwnedLexToken]) -> Option<usize> {
    tokens.iter().enumerate().find_map(|(idx, token)| {
        if !token.is_word("or") {
            return None;
        }
        // Keep quantifiers like "one or more <subject>" intact.
        let quantifier_or = idx > 0
            && tokens.get(idx - 1).is_some_and(|prev| prev.is_word("one"))
            && tokens.get(idx + 1).is_some_and(|next| next.is_word("more"));
        let comparison_or = is_comparison_or_delimiter(tokens, idx);
        let previous_numeric = (0..idx)
            .rev()
            .find_map(|i| tokens[i].as_word())
            .is_some_and(|word| word.parse::<i32>().is_ok());
        let next_numeric = tokens
            .get(idx + 1)
            .and_then(OwnedLexToken::as_word)
            .is_some_and(|word| word.parse::<i32>().is_ok());
        let numeric_list_or = previous_numeric && next_numeric;
        let color_list_or = tokens
            .get(idx - 1)
            .and_then(OwnedLexToken::as_word)
            .is_some_and(|word| parse_color(word).is_some())
            && tokens
                .get(idx + 1)
                .and_then(OwnedLexToken::as_word)
                .is_some_and(|word| parse_color(word).is_some())
            && tokens
                .iter()
                .filter_map(OwnedLexToken::as_word)
                .any(|word| word == "spell" || word == "spells");
        let objectish_word = |word: &str| is_trigger_objectish_word(word);
        let object_list_or = tokens
            .get(idx - 1)
            .and_then(OwnedLexToken::as_word)
            .is_some_and(objectish_word)
            && tokens
                .get(idx + 1)
                .and_then(OwnedLexToken::as_word)
                .is_some_and(objectish_word);
        let and_or_list_or = tokens.get(idx - 1).is_some_and(|prev| prev.is_word("and"))
            && tokens
                .get(idx + 1)
                .and_then(OwnedLexToken::as_word)
                .is_some_and(|word| parse_color(word).is_some() || objectish_word(word));
        let previous_word = (0..idx).rev().find_map(|i| tokens[i].as_word());
        let next_word = tokens.get(idx + 1).and_then(OwnedLexToken::as_word);
        let serial_spell_list_or = tokens
            .iter()
            .filter_map(OwnedLexToken::as_word)
            .any(|word| word == "spell" || word == "spells")
            && previous_word
                .is_some_and(|word| parse_color(word).is_some() || objectish_word(word))
            && next_word.is_some_and(|word| parse_color(word).is_some() || objectish_word(word));
        let cast_or_copy_or = tokens
            .iter()
            .filter_map(OwnedLexToken::as_word)
            .any(|word| word == "spell" || word == "spells")
            && previous_word.is_some_and(|word| word == "cast" || word == "casts")
            && next_word.is_some_and(|word| word == "copy" || word == "copies");
        let spell_or_ability_or = tokens
            .get(idx - 1)
            .and_then(OwnedLexToken::as_word)
            .is_some_and(|word| word == "spell" || word == "spells")
            && tokens
                .get(idx + 1)
                .and_then(OwnedLexToken::as_word)
                .is_some_and(|word| word == "ability" || word == "abilities");
        if quantifier_or
            || comparison_or
            || numeric_list_or
            || color_list_or
            || object_list_or
            || and_or_list_or
            || serial_spell_list_or
            || cast_or_copy_or
            || spell_or_ability_or
        {
            None
        } else {
            Some(idx)
        }
    })
}

pub(crate) fn has_leading_one_or_more(tokens: &[OwnedLexToken]) -> bool {
    tokens.len() >= 3
        && tokens.first().is_some_and(|token| token.is_word("one"))
        && tokens.get(1).is_some_and(|token| token.is_word("or"))
        && tokens.get(2).is_some_and(|token| token.is_word("more"))
}

pub(crate) fn strip_leading_one_or_more(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    if has_leading_one_or_more(tokens) {
        &tokens[3..]
    } else {
        tokens
    }
}

pub(crate) fn parse_leading_or_more_quantifier(
    tokens: &[OwnedLexToken],
) -> Option<(u32, &[OwnedLexToken])> {
    let (count, used) = parse_number(tokens)?;
    if tokens
        .get(used)
        .is_some_and(|token: &OwnedLexToken| token.is_word("or"))
        && tokens
            .get(used + 1)
            .is_some_and(|token: &OwnedLexToken| token.is_word("more"))
    {
        Some((count, &tokens[used + 2..]))
    } else {
        None
    }
}

pub(crate) fn parse_trigger_clause_lexed(
    tokens: &[OwnedLexToken],
) -> Result<TriggerSpec, CardTextError> {
    fn parse_not_during_turn_suffix(words: &[&str]) -> Option<PlayerFilter> {
        match words {
            ["a", "card", "if", "it", "isnt", "that", "players", "turn"]
            | ["a", "card", "if", "its", "not", "that", "players", "turn"]
            | ["a", "card", "if", "it", "isnt", "their", "turn"]
            | ["a", "card", "if", "its", "not", "their", "turn"] => {
                Some(PlayerFilter::IteratedPlayer)
            }
            ["a", "card", "if", "it", "isnt", "your", "turn"]
            | ["a", "card", "if", "its", "not", "your", "turn"] => Some(PlayerFilter::You),
            ["a", "card", "if", "it", "isnt", "an", "opponents", "turn"]
            | ["a", "card", "if", "its", "not", "an", "opponents", "turn"]
            | ["a", "card", "if", "it", "isnt", "opponents", "turn"]
            | ["a", "card", "if", "its", "not", "opponents", "turn"] => {
                Some(PlayerFilter::Opponent)
            }
            _ => None,
        }
    }

    fn parse_enters_origin_clause_lexed(words: &[&str]) -> Option<(Zone, Option<PlayerFilter>)> {
        let tail_words = words
            .iter()
            .copied()
            .filter(|word| !is_article(word))
            .collect::<Vec<_>>();
        match tail_words.as_slice() {
            ["from", "your", "graveyard"] => Some((Zone::Graveyard, Some(PlayerFilter::You))),
            ["from", "graveyard"] => Some((Zone::Graveyard, None)),
            ["from", "your", "hand"] => Some((Zone::Hand, Some(PlayerFilter::You))),
            ["from", "hand"] => Some((Zone::Hand, None)),
            ["from", "exile"] => Some((Zone::Exile, None)),
            _ => None,
        }
    }

    fn source_trigger_subject_filter_lexed(subject_words: &[&str]) -> ObjectFilter {
        let mut filter = ObjectFilter::default();
        if subject_words.iter().any(|word| *word == "creature") {
            filter.card_types.push(CardType::Creature);
        } else if subject_words.iter().any(|word| *word == "land") {
            filter.card_types.push(CardType::Land);
        } else if subject_words.iter().any(|word| *word == "artifact") {
            filter.card_types.push(CardType::Artifact);
        } else if subject_words.iter().any(|word| *word == "enchantment") {
            filter.card_types.push(CardType::Enchantment);
        } else if subject_words.iter().any(|word| *word == "planeswalker") {
            filter.card_types.push(CardType::Planeswalker);
        } else if subject_words.iter().any(|word| *word == "battle") {
            filter.card_types.push(CardType::Battle);
        }
        filter
    }

    fn parse_damage_by_dies_trigger_lexed(
        subject_tokens: &[OwnedLexToken],
        other: bool,
        clause_words: &[&str],
    ) -> Result<Option<TriggerSpec>, CardTextError> {
        fn trim_lexed_edge_punctuation(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
            let mut start = 0usize;
            let mut end = tokens.len();
            while start < end
                && matches!(
                    tokens[start].kind,
                    TokenKind::Comma | TokenKind::Period | TokenKind::Semicolon | TokenKind::Quote
                )
            {
                start += 1;
            }
            while end > start
                && matches!(
                    tokens[end - 1].kind,
                    TokenKind::Comma | TokenKind::Period | TokenKind::Semicolon | TokenKind::Quote
                )
            {
                end -= 1;
            }
            &tokens[start..end]
        }

        fn strip_leading_articles_lexed(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
            let view = ActivationRestrictionCompatWords::new(tokens);
            if matches!(view.first(), Some("a" | "an" | "the")) {
                let start = view.token_index_for_word_index(1).unwrap_or(tokens.len());
                &tokens[start..]
            } else {
                tokens
            }
        }

        let subject_word_view = ActivationRestrictionCompatWords::new(subject_tokens);
        let subject_words = subject_word_view.to_word_refs();
        if subject_words.len() < 8
            || !slice_ends_with(&subject_words, &["this", "turn"])
            || !contains_word_sequence(&subject_words, &["dealt", "damage", "by"])
        {
            return Ok(None);
        }

        let Some(dealt_word_idx) =
            find_word_sequence_start(&subject_words, &["dealt", "damage", "by"])
        else {
            return Ok(None);
        };

        let victim_end = subject_word_view
            .token_index_for_word_index(dealt_word_idx)
            .unwrap_or(0);
        if victim_end == 0 || victim_end > subject_tokens.len() {
            return Ok(None);
        }

        let victim_tokens = trim_lexed_edge_punctuation(&subject_tokens[..victim_end]);
        let victim_tokens = strip_leading_articles_lexed(victim_tokens);
        if victim_tokens.is_empty() {
            return Ok(None);
        }

        let damager_start_word_idx = dealt_word_idx + 3;
        let this_word_idx = subject_words.len() - 2;
        let damager_start = subject_word_view
            .token_index_for_word_index(damager_start_word_idx)
            .unwrap_or(subject_tokens.len());
        let damager_end = subject_word_view
            .token_index_for_word_index(this_word_idx)
            .unwrap_or(subject_tokens.len());
        if damager_start >= damager_end || damager_end > subject_tokens.len() {
            return Ok(None);
        }

        let damager_tokens =
            trim_lexed_edge_punctuation(&subject_tokens[damager_start..damager_end]);
        let damager_word_view = ActivationRestrictionCompatWords::new(&damager_tokens);
        let damager_words = damager_word_view.to_word_refs();
        let has_named_source_words = !damager_words.is_empty()
            && !matches!(
                damager_words.first().copied(),
                Some("a" | "an" | "the" | "target" | "that" | "this" | "equipped" | "enchanted")
            )
            && !damager_words.iter().any(|word| {
                matches!(
                    *word,
                    "creature" | "creatures" | "permanent" | "permanents" | "source" | "sources"
                )
            });

        let damager = if damager_words == ["this", "creature"]
            || damager_words == ["this", "permanent"]
            || damager_words == ["this", "source"]
            || damager_words == ["this"]
            || has_named_source_words
        {
            Some(DamageBySpec::ThisCreature)
        } else if damager_words == ["equipped", "creature"] {
            Some(DamageBySpec::EquippedCreature)
        } else if damager_words == ["enchanted", "creature"] {
            Some(DamageBySpec::EnchantedCreature)
        } else {
            None
        };

        let Some(damager) = damager else {
            return Ok(None);
        };

        let victim = parse_object_filter_lexed(&victim_tokens, other).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported damaged-by trigger victim filter (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
        Ok(Some(TriggerSpec::DiesCreatureDealtDamageByThisTurn {
            victim,
            damager,
        }))
    }

    fn parse_simple_spell_activity_trigger_lexed(
        tokens: &[OwnedLexToken],
        clause_words: &[&str],
    ) -> Result<Option<TriggerSpec>, CardTextError> {
        if !slice_contains(&clause_words, &"spell") && !slice_contains(&clause_words, &"spells") {
            return Ok(None);
        }
        if slice_contains(&clause_words, &"during")
            || slice_contains(&clause_words, &"turn")
            || slice_contains(&clause_words, &"first")
            || slice_contains(&clause_words, &"second")
            || slice_contains(&clause_words, &"third")
            || slice_contains(&clause_words, &"fourth")
            || slice_contains(&clause_words, &"fifth")
            || slice_contains(&clause_words, &"sixth")
            || slice_contains(&clause_words, &"seventh")
            || slice_contains(&clause_words, &"eighth")
            || slice_contains(&clause_words, &"ninth")
            || slice_contains(&clause_words, &"tenth")
            || contains_word_sequence(&clause_words, &["other", "than"])
            || contains_word_sequence(&clause_words, &["from", "anywhere"])
        {
            return Ok(None);
        }

        let cast_idx = find_index(tokens, |token| {
            token.is_word("cast") || token.is_word("casts")
        });
        let copy_idx = find_index(tokens, |token| {
            token.is_word("copy") || token.is_word("copies")
        });
        if cast_idx.is_none() && copy_idx.is_none() {
            return Ok(None);
        }

        let actor = parse_subject_clause_player_filter(clause_words);
        let parse_filter =
            |filter_tokens: &[OwnedLexToken]| -> Result<Option<ObjectFilter>, CardTextError> {
                let filter_words = ActivationRestrictionCompatWords::new(filter_tokens);
                let filter_words = filter_words.to_word_refs();
                let is_unqualified_spell = filter_words.as_slice() == ["a", "spell"]
                    || filter_words.as_slice() == ["spell"]
                    || filter_words.as_slice() == ["spells"];
                if filter_tokens.is_empty() || is_unqualified_spell {
                    return Ok(None);
                }
                parse_object_filter_lexed(filter_tokens, false)
                    .map(Some)
                    .map_err(|err| {
                        CardTextError::ParseError(format!(
                            "unsupported spell trigger filter (clause: '{}') [{err:?}]",
                            filter_words.join(" ")
                        ))
                    })
            };

        if let (Some(cast), Some(copy)) = (cast_idx, copy_idx) {
            let (first, second, first_is_cast) = if cast < copy {
                (cast, copy, true)
            } else {
                (copy, cast, false)
            };
            let between_view = ActivationRestrictionCompatWords::new(&tokens[first + 1..second]);
            let between_words = between_view.to_word_refs();
            if between_words.as_slice() == ["or"] {
                let filter = parse_filter(tokens.get(second + 1..).unwrap_or_default())?;
                let cast_trigger = TriggerSpec::SpellCast {
                    filter: filter.clone(),
                    caster: actor.clone(),
                    during_turn: None,
                    min_spells_this_turn: None,
                    exact_spells_this_turn: None,
                    from_not_hand: false,
                };
                let copied_trigger = TriggerSpec::SpellCopied {
                    filter,
                    copier: actor,
                };
                return Ok(Some(if first_is_cast {
                    TriggerSpec::Either(Box::new(cast_trigger), Box::new(copied_trigger))
                } else {
                    TriggerSpec::Either(Box::new(copied_trigger), Box::new(cast_trigger))
                }));
            }
        }

        if let Some(cast) = cast_idx {
            let mut filter_tokens = tokens.get(cast + 1..).unwrap_or_default();
            if filter_tokens.is_empty() {
                let mut prefix_tokens = &tokens[..cast];
                while let Some(last_word) = prefix_tokens.last().and_then(OwnedLexToken::as_word) {
                    if matches!(last_word, "is" | "are" | "was" | "were" | "be" | "been") {
                        prefix_tokens = &prefix_tokens[..prefix_tokens.len() - 1];
                    } else {
                        break;
                    }
                }
                let has_spell_noun = prefix_tokens
                    .iter()
                    .any(|token| token.is_word("spell") || token.is_word("spells"));
                if has_spell_noun {
                    filter_tokens = prefix_tokens;
                }
            }
            let filter = parse_filter(filter_tokens)?;
            return Ok(Some(TriggerSpec::SpellCast {
                filter,
                caster: actor,
                during_turn: None,
                min_spells_this_turn: None,
                exact_spells_this_turn: None,
                from_not_hand: false,
            }));
        }

        if let Some(copy) = copy_idx {
            let filter = parse_filter(tokens.get(copy + 1..).unwrap_or_default())?;
            return Ok(Some(TriggerSpec::SpellCopied {
                filter,
                copier: actor,
            }));
        }

        Ok(None)
    }

    let word_view = ActivationRestrictionCompatWords::new(tokens);
    let words = word_view.to_word_refs();
    if words.is_empty() {
        return Err(CardTextError::ParseError(
            "empty trigger clause".to_string(),
        ));
    }

    if let Some(enters_idx) = find_index(tokens, |token| {
        token.is_word("enters") || token.is_word("enter")
    }) {
        let tail = &tokens[enters_idx + 1..];
        let shared_subject_or_combat_damage = tail.len() >= 6
            && tail[0].is_word("the")
            && tail[1].is_word("battlefield")
            && tail[2].is_word("or")
            && (tail[3].is_word("deal") || tail[3].is_word("deals"))
            && tail[4].is_word("combat")
            && tail[5].is_word("damage");
        if shared_subject_or_combat_damage {
            let or_idx = enters_idx + 3;
            let left_tokens = &tokens[..or_idx];
            let mut right_tokens = tokens[..enters_idx].to_vec();
            right_tokens.extend_from_slice(&tokens[or_idx + 1..]);

            if !left_tokens.is_empty()
                && let (Ok(left), Ok(right)) = (
                    parse_trigger_clause_lexed(left_tokens),
                    parse_trigger_clause_lexed(&right_tokens),
                )
            {
                return Ok(TriggerSpec::Either(Box::new(left), Box::new(right)));
            }
        }
        let shared_subject_or_attack = (tail.len() == 2
            && tail[0].is_word("or")
            && (tail[1].is_word("attack") || tail[1].is_word("attacks")))
            || (tail.len() == 4
                && tail[0].is_word("the")
                && tail[1].is_word("battlefield")
                && tail[2].is_word("or")
                && (tail[3].is_word("attack") || tail[3].is_word("attacks")));
        if shared_subject_or_attack {
            let or_idx = if tail[0].is_word("or") {
                enters_idx + 1
            } else {
                enters_idx + 3
            };
            let attack_idx = or_idx + 1;
            let left_tokens = &tokens[..or_idx];
            let mut right_tokens = tokens[..enters_idx].to_vec();
            right_tokens.push(tokens[attack_idx].clone());

            if !left_tokens.is_empty()
                && let (Ok(left), Ok(right)) = (
                    parse_trigger_clause_lexed(left_tokens),
                    parse_trigger_clause_lexed(&right_tokens),
                )
            {
                return Ok(TriggerSpec::Either(Box::new(left), Box::new(right)));
            }
        }
    }

    if let Some(or_idx) = split_trigger_or_index(tokens) {
        let left_tokens = &tokens[..or_idx];
        let right_tokens = &tokens[or_idx + 1..];
        if !left_tokens.is_empty()
            && !right_tokens.is_empty()
            && let (Ok(left), Ok(right)) = (
                parse_trigger_clause_lexed(left_tokens),
                parse_trigger_clause_lexed(right_tokens),
            )
        {
            return Ok(TriggerSpec::Either(Box::new(left), Box::new(right)));
        }
    }
    if let Some(and_idx) = find_index(tokens, |token| token.is_word("and"))
        && tokens.get(and_idx + 1).is_some_and(|token| {
            token.is_word("whenever") || token.is_word("when") || token.is_word("at")
        })
    {
        let left_tokens = strip_leading_trigger_intro(&tokens[..and_idx]);
        let right_tokens = strip_leading_trigger_intro(&tokens[and_idx + 1..]);
        if !left_tokens.is_empty()
            && !right_tokens.is_empty()
            && let (Ok(left), Ok(right)) = (
                parse_trigger_clause_lexed(left_tokens),
                parse_trigger_clause_lexed(right_tokens),
            )
        {
            return Ok(TriggerSpec::Either(Box::new(left), Box::new(right)));
        }
    }

    if words.len() >= 2
        && words.last().copied() == Some("alone")
        && matches!(
            words.get(words.len() - 2).copied(),
            Some("attack" | "attacks")
        )
    {
        let attacks_word_idx = words.len().saturating_sub(2);
        let attacks_token_idx = ActivationRestrictionCompatWords::new(tokens)
            .token_index_for_word_index(attacks_word_idx)
            .unwrap_or(tokens.len());
        let subject_tokens = &tokens[..attacks_token_idx];
        return Ok(
            match parse_attack_trigger_subject_filter_lexed(subject_tokens)? {
                Some(filter) => TriggerSpec::AttacksAlone(filter),
                None => TriggerSpec::AttacksAlone(ObjectFilter::source()),
            },
        );
    }

    if let Some(attacks_word_idx) =
        find_index(&words, |word| *word == "attack" || *word == "attacks")
    {
        let tail_words = &words[attacks_word_idx + 1..];
        if tail_words == ["you", "or", "a", "planeswalker", "you", "control"]
            || tail_words == ["you", "or", "planeswalker", "you", "control"]
        {
            let attacks_token_idx = ActivationRestrictionCompatWords::new(tokens)
                .token_index_for_word_index(attacks_word_idx)
                .unwrap_or(tokens.len());
            let subject_tokens = &tokens[..attacks_token_idx];
            let subject_filter = parse_attack_trigger_subject_filter_lexed(subject_tokens)?
                .unwrap_or_else(ObjectFilter::source);
            let player_subject = trigger_subject_player_selector_lexed(subject_tokens).is_some();
            return Ok(if player_subject {
                TriggerSpec::AttacksYouOrPlaneswalkerYouControlOneOrMore(subject_filter)
            } else {
                TriggerSpec::AttacksYouOrPlaneswalkerYouControl(subject_filter)
            });
        }
    }

    if words.len() >= 3
        && matches!(
            words.get(words.len() - 3).copied(),
            Some("attack" | "attacks")
        )
        && words.get(words.len() - 2).copied() == Some("while")
        && words.last().copied() == Some("saddled")
    {
        let attacks_word_idx = words.len().saturating_sub(3);
        let attacks_token_idx = ActivationRestrictionCompatWords::new(tokens)
            .token_index_for_word_index(attacks_word_idx)
            .unwrap_or(tokens.len());
        let subject_tokens = &tokens[..attacks_token_idx];
        return Ok(
            match parse_attack_trigger_subject_filter_lexed(subject_tokens)? {
                Some(filter) => TriggerSpec::AttacksWhileSaddled(filter),
                None => TriggerSpec::ThisAttacksWhileSaddled,
            },
        );
    }

    let is_you_cast_this_spell = contains_any_word_sequence(
        &words,
        &[&["cast", "this", "spell"], &["casts", "this", "spell"]],
    );
    if is_you_cast_this_spell && slice_contains(&words, &"you") {
        return Ok(TriggerSpec::YouCastThisSpell);
    }

    if let Some(spell_activity_trigger) = parse_simple_spell_activity_trigger_lexed(tokens, &words)?
    {
        return Ok(spell_activity_trigger);
    }
    if let Some(spell_activity_trigger) = parse_spell_activity_trigger(tokens)? {
        return Ok(spell_activity_trigger);
    }

    if let Some(play_idx) = find_index(tokens, |token| {
        token.is_word("play") || token.is_word("plays")
    }) {
        let subject_tokens = &tokens[..play_idx];
        let subject_word_view = ActivationRestrictionCompatWords::new(subject_tokens);
        let subject_words = subject_word_view.to_word_refs();
        if let Some(player) = parse_trigger_subject_player_filter(&subject_words) {
            let trimmed_object_tokens = trim_commas(&tokens[play_idx + 1..]);
            let object_tokens = strip_leading_articles(&trimmed_object_tokens);
            let object_word_view = ActivationRestrictionCompatWords::new(&object_tokens);
            let object_words = object_word_view.to_word_refs();
            if object_words
                .iter()
                .any(|word| matches!(*word, "land" | "lands"))
                && let Ok(filter) = parse_object_filter_lexed(&object_tokens, false)
            {
                return Ok(TriggerSpec::PlayerPlaysLand { player, filter });
            }
        }
    }

    if let Some(search_idx) = find_index(tokens, |token| {
        token.is_word("search") || token.is_word("searches")
    }) {
        let subject_tokens = &tokens[..search_idx];
        let subject_word_view = ActivationRestrictionCompatWords::new(subject_tokens);
        let subject_words = subject_word_view.to_word_refs();
        if let Some(player) = parse_trigger_subject_player_filter(&subject_words) {
            let searched_tokens = trim_commas(&tokens[search_idx + 1..]);
            let searched_word_view = ActivationRestrictionCompatWords::new(&searched_tokens);
            let searched_words = searched_word_view.to_word_refs();
            if slice_starts_with(&searched_words, &["their", "library"])
                || slice_starts_with(&searched_words, &["your", "library"])
                || slice_starts_with(&searched_words, &["a", "library"])
            {
                return Ok(TriggerSpec::PlayerSearchesLibrary(player));
            }
        }
    }

    if let Some(shuffle_idx) = find_index(tokens, |token| {
        token.is_word("shuffle") || token.is_word("shuffles")
    }) {
        let subject_tokens = &tokens[..shuffle_idx];
        let subject_word_view = ActivationRestrictionCompatWords::new(subject_tokens);
        let subject_words = subject_word_view.to_word_refs();
        let shuffled_tokens = trim_commas(&tokens[shuffle_idx + 1..]);
        let shuffled_word_view = ActivationRestrictionCompatWords::new(&shuffled_tokens);
        let shuffled_words = shuffled_word_view.to_word_refs();
        if slice_starts_with(&shuffled_words, &["their", "library"])
            || slice_starts_with(&shuffled_words, &["your", "library"])
            || slice_starts_with(&shuffled_words, &["a", "library"])
            || slice_starts_with(&shuffled_words, &["that", "players", "library"])
        {
            if let Some((player, caused_by_effect, source_controller_shuffles)) =
                parse_shuffle_trigger_subject(&subject_words)
            {
                return Ok(TriggerSpec::PlayerShufflesLibrary {
                    player,
                    caused_by_effect,
                    source_controller_shuffles,
                });
            }
        }
    }

    if let Some(give_idx) = find_index(tokens, |token| {
        token.is_word("give") || token.is_word("gives")
    }) {
        let subject_tokens = &tokens[..give_idx];
        let subject_word_view = ActivationRestrictionCompatWords::new(subject_tokens);
        let subject_words = subject_word_view.to_word_refs();
        if let Some(player) = parse_trigger_subject_player_filter(&subject_words) {
            let gifted_tokens = trim_commas(&tokens[give_idx + 1..]);
            let gifted_word_view = ActivationRestrictionCompatWords::new(&gifted_tokens);
            let gifted_words = gifted_word_view.to_word_refs();
            if gifted_words == ["a", "gift"] || gifted_words == ["gift"] {
                return Ok(TriggerSpec::PlayerGivesGift(player));
            }
        }
    }

    if let Some(tap_idx) = find_index(tokens, |token| {
        token.is_word("tap") || token.is_word("taps")
    }) {
        let subject_tokens = &tokens[..tap_idx];
        let subject_word_view = ActivationRestrictionCompatWords::new(subject_tokens);
        let subject_words = subject_word_view.to_word_refs();
        if let Some(player) = parse_trigger_subject_player_filter(&subject_words) {
            let after_tap = &tokens[tap_idx + 1..];
            if let Some(for_idx) = find_index(after_tap, |token| token.is_word("for"))
                && for_idx > 0
            {
                let object_tokens = trim_commas(&after_tap[..for_idx]);
                let object_tokens = strip_leading_articles(&object_tokens);
                if !object_tokens.is_empty()
                    && let Ok(filter) = parse_object_filter_lexed(&object_tokens, false)
                {
                    return Ok(TriggerSpec::PlayerTapsForMana { player, filter });
                }
            }
        }
    }

    if let Some(tapped_idx) = find_index(tokens, |token| token.is_word("tapped"))
        && tapped_idx >= 2
        && tokens
            .get(tapped_idx.wrapping_sub(1))
            .is_some_and(|token| token.is_word("is") || token.is_word("are"))
    {
        let subject_tokens = &tokens[..tapped_idx - 1];
        let after_tapped = &tokens[tapped_idx + 1..];
        if after_tapped.iter().any(|token| token.is_word("for")) {
            let object_tokens = trim_commas(subject_tokens);
            let object_tokens = strip_leading_articles(&object_tokens);
            if !object_tokens.is_empty()
                && let Ok(filter) = parse_object_filter_lexed(&object_tokens, false)
            {
                return Ok(TriggerSpec::PlayerTapsForMana {
                    player: PlayerFilter::Any,
                    filter,
                });
            }
        }
    }

    if let Some(activate_idx) =
        find_index(&words, |word| *word == "activate" || *word == "activates")
    {
        let subject_tokens = &tokens[..activate_idx];
        let subject_word_view = ActivationRestrictionCompatWords::new(subject_tokens);
        let subject_words = subject_word_view.to_word_refs();
        if let Some(activator) = parse_trigger_subject_player_filter(&subject_words) {
            let tail_words = &words[activate_idx + 1..];
            if tail_words == ["an", "ability"]
                || tail_words == ["abilities"]
                || tail_words == ["an", "ability", "that", "isnt", "a", "mana", "ability"]
                || tail_words == ["an", "ability", "that", "isn't", "a", "mana", "ability"]
                || tail_words == ["abilities", "that", "arent", "mana", "abilities"]
                || tail_words == ["abilities", "that", "aren't", "mana", "abilities"]
            {
                return Ok(TriggerSpec::AbilityActivated {
                    activator,
                    filter: ObjectFilter::default(),
                    non_mana_only: slice_contains(&tail_words, &"mana"),
                });
            }
        }
    }

    let has_deal = words.iter().any(|word| *word == "deal" || *word == "deals");
    if has_deal && slice_contains(&words, &"combat") && slice_contains(&words, &"damage") {
        if let Some(deals_idx) = find_index(tokens, |token| {
            token.is_word("deal") || token.is_word("deals")
        }) {
            let subject_tokens = &tokens[..deals_idx];
            let player_subject = trigger_subject_player_selector_lexed(subject_tokens).is_some();
            let one_or_more = has_leading_one_or_more(subject_tokens) || player_subject;
            let source_filter = parse_attack_trigger_subject_filter_lexed(subject_tokens)?;
            if let Some(damage_idx_rel) =
                find_index(&tokens[deals_idx + 1..], |token| token.is_word("damage"))
            {
                let damage_idx = deals_idx + 1 + damage_idx_rel;
                if let Some(to_idx_rel) =
                    find_index(&tokens[damage_idx + 1..], |token| token.is_word("to"))
                {
                    let to_idx = damage_idx + 1 + to_idx_rel;
                    let target_tokens = split_target_clause_before_comma(&tokens[to_idx + 1..]);
                    if target_tokens.is_empty() {
                        return Err(CardTextError::ParseError(format!(
                            "missing combat damage recipient filter in trigger clause (clause: '{}')",
                            words.join(" ")
                        )));
                    }
                    let target_word_view = ActivationRestrictionCompatWords::new(&target_tokens);
                    let target_words = target_word_view.to_word_refs();
                    if let Some(player) = parse_trigger_subject_player_filter(&target_words) {
                        return Ok(match source_filter {
                            Some(source) => {
                                if one_or_more {
                                    TriggerSpec::DealsCombatDamageToPlayerOneOrMore {
                                        source,
                                        player,
                                    }
                                } else {
                                    TriggerSpec::DealsCombatDamageToPlayer { source, player }
                                }
                            }
                            None => TriggerSpec::ThisDealsCombatDamageToPlayer,
                        });
                    }

                    let target_tokens = strip_leading_one_or_more_lexed(&target_tokens);
                    let target_filter = parse_object_filter_lexed(target_tokens, false).map_err(|_| {
                        CardTextError::ParseError(format!(
                            "unsupported combat damage recipient filter in trigger clause (clause: '{}')",
                            words.join(" ")
                        ))
                    })?;
                    return Ok(match source_filter {
                        Some(source) => TriggerSpec::DealsCombatDamageTo {
                            source,
                            target: target_filter,
                        },
                        None => TriggerSpec::ThisDealsCombatDamageTo(target_filter),
                    });
                }
            }

            return Ok(match source_filter {
                Some(filter) => TriggerSpec::DealsCombatDamage(filter),
                None => TriggerSpec::ThisDealsCombatDamage,
            });
        }
        return Ok(TriggerSpec::ThisDealsCombatDamage);
    }

    if words.as_slice() == ["this", "leaves", "the", "battlefield"]
        || (words.len() == 5
            && words.first().copied() == Some("this")
            && words.get(2).copied() == Some("leaves")
            && words.get(3).copied() == Some("the")
            && words.get(4).copied() == Some("battlefield"))
    {
        return Ok(TriggerSpec::ThisLeavesBattlefield);
    }

    if let Some(dies_word_idx) = find_index(&words, |word| *word == "dies") {
        let dies_token_idx = word_view
            .token_index_for_word_index(dies_word_idx)
            .unwrap_or(tokens.len());
        let subject_tokens = &tokens[..dies_token_idx];
        let subject_word_view = ActivationRestrictionCompatWords::new(subject_tokens);
        let subject_words = subject_word_view.to_word_refs();
        if is_source_reference_words(&subject_words)
            && words.get(dies_word_idx + 1..)
                == Some(
                    &[
                        "or",
                        "is",
                        "put",
                        "into",
                        "exile",
                        "from",
                        "the",
                        "battlefield",
                    ][..],
                )
        {
            return Ok(TriggerSpec::ThisDiesOrIsExiled);
        }
    }

    if let Some(enters_word_idx) = find_index(&words, |word| *word == "enters" || *word == "enter")
    {
        let enters_token_idx = word_view
            .token_index_for_word_index(enters_word_idx)
            .unwrap_or(tokens.len());
        if slice_ends_with(&words, &["enters", "or", "leaves", "the", "battlefield"])
            || slice_ends_with(&words, &["enter", "or", "leave", "the", "battlefield"])
        {
            let subject_tokens = &tokens[..enters_token_idx];
            if subject_tokens
                .first()
                .is_some_and(|token| token.is_word("this"))
            {
                return Ok(TriggerSpec::Either(
                    Box::new(TriggerSpec::ThisEntersBattlefield),
                    Box::new(TriggerSpec::ThisLeavesBattlefield),
                ));
            }
        }

        let enters_origin = parse_enters_origin_clause_lexed(&words[enters_word_idx + 1..]);
        if enters_word_idx == 0 {
            return Ok(if let Some((from, owner)) = enters_origin.clone() {
                TriggerSpec::ThisEntersBattlefieldFromZone {
                    subject_filter: ObjectFilter::default(),
                    from,
                    owner,
                }
            } else {
                TriggerSpec::ThisEntersBattlefield
            });
        }

        let subject_tokens = &tokens[..enters_token_idx];
        if let Some(or_idx) =
            find_index(subject_tokens, |token: &OwnedLexToken| token.is_word("or"))
        {
            let left_tokens = &subject_tokens[..or_idx];
            let mut right_tokens = &subject_tokens[or_idx + 1..];
            let left_word_view = ActivationRestrictionCompatWords::new(left_tokens);
            let left_words: Vec<&str> = left_word_view
                .to_word_refs()
                .into_iter()
                .filter(|word| !is_article(word))
                .collect();
            if is_source_reference_words(&left_words) && !right_tokens.is_empty() {
                let mut other = false;
                if right_tokens
                    .first()
                    .is_some_and(|token| token.is_word("another") || token.is_word("other"))
                {
                    other = true;
                    right_tokens = &right_tokens[1..];
                }
                let parsed_filter =
                    parse_object_filter_lexed(right_tokens, other)
                        .ok()
                        .or_else(|| {
                            parse_subtype_list_enters_trigger_filter_lexed(right_tokens, other)
                        });
                if let Some(mut filter) = parsed_filter {
                    if slice_contains(&words, &"under")
                        && slice_contains(&words, &"your")
                        && slice_contains(&words, &"control")
                    {
                        filter.controller = Some(PlayerFilter::You);
                    } else if slice_contains(&words, &"under")
                        && (slice_contains(&words, &"opponent")
                            || slice_contains(&words, &"opponents"))
                        && slice_contains(&words, &"control")
                    {
                        filter.controller = Some(PlayerFilter::Opponent);
                    }
                    let cause_filter = if contains_window(&words, &["without", "being", "played"]) {
                        Some(crate::events::cause::CauseFilter::not_type(
                            crate::events::cause::CauseType::SpecialAction,
                        ))
                    } else {
                        None
                    };
                    let right_trigger = if slice_contains(&words, &"untapped") {
                        TriggerSpec::EntersBattlefieldUntapped {
                            filter,
                            cause_filter,
                        }
                    } else if slice_contains(&words, &"tapped") {
                        TriggerSpec::EntersBattlefieldTapped {
                            filter,
                            cause_filter,
                        }
                    } else {
                        TriggerSpec::EntersBattlefield {
                            filter,
                            cause_filter,
                        }
                    };
                    return Ok(TriggerSpec::Either(
                        Box::new(TriggerSpec::ThisEntersBattlefield),
                        Box::new(right_trigger),
                    ));
                }
            }
        }
        if subject_tokens
            .first()
            .is_some_and(|token| token.is_word("this"))
        {
            let subject_word_view = ActivationRestrictionCompatWords::new(subject_tokens);
            let subject_words = subject_word_view.to_word_refs();
            return Ok(if let Some((from, owner)) = enters_origin.clone() {
                TriggerSpec::ThisEntersBattlefieldFromZone {
                    subject_filter: source_trigger_subject_filter_lexed(&subject_words),
                    from,
                    owner,
                }
            } else {
                TriggerSpec::ThisEntersBattlefield
            });
        }

        let mut filtered_subject_tokens = subject_tokens;
        let mut other = false;
        if filtered_subject_tokens
            .first()
            .is_some_and(|token| token.is_word("another") || token.is_word("other"))
        {
            other = true;
            filtered_subject_tokens = &filtered_subject_tokens[1..];
        }
        let one_or_more = ActivationRestrictionCompatWords::new(filtered_subject_tokens)
            .slice_eq(0, &["one", "or", "more"]);
        filtered_subject_tokens = strip_leading_one_or_more_lexed(filtered_subject_tokens);
        if filtered_subject_tokens
            .first()
            .is_some_and(|token| token.is_word("another") || token.is_word("other"))
        {
            other = true;
            filtered_subject_tokens = &filtered_subject_tokens[1..];
        }
        let parsed_filter = parse_object_filter_lexed(filtered_subject_tokens, other)
            .ok()
            .or_else(|| {
                parse_subtype_list_enters_trigger_filter_lexed(filtered_subject_tokens, other)
            });
        if let Some(mut filter) = parsed_filter {
            let cause_filter = if contains_window(&words, &["without", "being", "played"]) {
                Some(crate::events::cause::CauseFilter::not_type(
                    crate::events::cause::CauseType::SpecialAction,
                ))
            } else {
                None
            };
            if slice_contains(&words, &"under")
                && slice_contains(&words, &"your")
                && slice_contains(&words, &"control")
            {
                filter.controller = Some(PlayerFilter::You);
            } else if slice_contains(&words, &"under")
                && (slice_contains(&words, &"opponent") || slice_contains(&words, &"opponents"))
                && slice_contains(&words, &"control")
            {
                filter.controller = Some(PlayerFilter::Opponent);
            }
            if slice_contains(&words, &"untapped") {
                return Ok(TriggerSpec::EntersBattlefieldUntapped {
                    filter,
                    cause_filter,
                });
            }
            if slice_contains(&words, &"tapped") {
                return Ok(TriggerSpec::EntersBattlefieldTapped {
                    filter,
                    cause_filter,
                });
            }
            return Ok(if let Some((from, owner)) = enters_origin {
                TriggerSpec::EntersBattlefieldFromZone {
                    filter,
                    from,
                    owner,
                    one_or_more,
                    cause_filter,
                }
            } else if one_or_more {
                TriggerSpec::EntersBattlefieldOneOrMore {
                    filter,
                    cause_filter,
                }
            } else {
                TriggerSpec::EntersBattlefield {
                    filter,
                    cause_filter,
                }
            });
        }
    }

    for tail in [
        ["is", "put", "into", "your", "graveyard", "from", "anywhere"].as_slice(),
        [
            "are",
            "put",
            "into",
            "your",
            "graveyard",
            "from",
            "anywhere",
        ]
        .as_slice(),
        ["is", "put", "into", "your", "graveyard"].as_slice(),
        ["are", "put", "into", "your", "graveyard"].as_slice(),
    ] {
        if slice_ends_with(&words, tail) {
            let subject_word_len = words.len().saturating_sub(tail.len());
            let subject_tokens = ActivationRestrictionCompatWords::new(tokens)
                .token_index_for_word_index(subject_word_len)
                .map(|idx| &tokens[..idx])
                .unwrap_or_default();
            let subject_view = ActivationRestrictionCompatWords::new(subject_tokens);
            let subject_words = subject_view.to_word_refs();
            let mut filter = parse_object_filter_lexed(subject_tokens, false).map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported card filter in put-into-your-graveyard trigger clause (clause: '{}')",
                    words.join(" ")
                ))
            })?;
            filter.zone = None;
            filter.controller = None;
            if filter.owner.is_none() {
                filter.owner = Some(PlayerFilter::You);
            }
            if subject_words
                .iter()
                .any(|word| matches!(*word, "card" | "cards"))
            {
                filter.nontoken = true;
            }
            return Ok(TriggerSpec::PutIntoGraveyard(filter));
        }
    }

    for tail in [
        ["is", "put", "into", "a", "graveyard", "from", "anywhere"].as_slice(),
        ["are", "put", "into", "a", "graveyard", "from", "anywhere"].as_slice(),
    ] {
        if slice_ends_with(&words, tail) {
            let subject_word_len = words.len().saturating_sub(tail.len());
            let subject_tokens = ActivationRestrictionCompatWords::new(tokens)
                .token_index_for_word_index(subject_word_len)
                .map(|idx| &tokens[..idx])
                .unwrap_or_default();
            let subject_view = ActivationRestrictionCompatWords::new(subject_tokens);
            let subject_words = subject_view.to_word_refs();
            if is_source_reference_words(&subject_words) {
                return Ok(TriggerSpec::PutIntoGraveyard(ObjectFilter::source()));
            }
            if let Ok(filter) = parse_object_filter_lexed(subject_tokens, false) {
                return Ok(TriggerSpec::PutIntoGraveyard(filter));
            }
            return Err(CardTextError::ParseError(format!(
                "unsupported filter in put-into-graveyard-from-anywhere trigger clause (clause: '{}')",
                words.join(" ")
            )));
        }
    }

    for tail in [
        [
            "is",
            "put",
            "into",
            "your",
            "graveyard",
            "from",
            "the",
            "battlefield",
        ]
        .as_slice(),
        [
            "are",
            "put",
            "into",
            "your",
            "graveyard",
            "from",
            "the",
            "battlefield",
        ]
        .as_slice(),
    ] {
        if slice_ends_with(&words, tail) {
            let subject_word_len = words.len().saturating_sub(tail.len());
            let subject_tokens = ActivationRestrictionCompatWords::new(tokens)
                .token_index_for_word_index(subject_word_len)
                .map(|idx| &tokens[..idx])
                .unwrap_or_default();
            let subject_view = ActivationRestrictionCompatWords::new(subject_tokens);
            let subject_words = subject_view.to_word_refs();
            if is_source_reference_words(&subject_words) {
                return Ok(TriggerSpec::PutIntoGraveyardFromZone {
                    filter: ObjectFilter::source(),
                    from: Zone::Battlefield,
                });
            }
            let mut filter = parse_object_filter_lexed(subject_tokens, false).map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported card filter in put-into-your-graveyard-from-battlefield trigger clause (clause: '{}')",
                    words.join(" ")
                ))
            })?;
            filter.zone = None;
            filter.controller = None;
            if filter.owner.is_none() {
                filter.owner = Some(PlayerFilter::You);
            }
            if subject_words
                .iter()
                .any(|word| matches!(*word, "card" | "cards"))
            {
                filter.nontoken = true;
            }
            return Ok(TriggerSpec::PutIntoGraveyardFromZone {
                filter,
                from: Zone::Battlefield,
            });
        }
    }

    for tail in [
        [
            "is",
            "put",
            "into",
            "an",
            "opponents",
            "graveyard",
            "from",
            "the",
            "battlefield",
        ]
        .as_slice(),
        [
            "are",
            "put",
            "into",
            "an",
            "opponents",
            "graveyard",
            "from",
            "the",
            "battlefield",
        ]
        .as_slice(),
    ] {
        if slice_ends_with(&words, tail) {
            let subject_word_len = words.len().saturating_sub(tail.len());
            let subject_tokens = ActivationRestrictionCompatWords::new(tokens)
                .token_index_for_word_index(subject_word_len)
                .map(|idx| &tokens[..idx])
                .unwrap_or_default();
            let subject_view = ActivationRestrictionCompatWords::new(subject_tokens);
            let subject_words = subject_view.to_word_refs();
            if is_source_reference_words(&subject_words) {
                let mut filter = ObjectFilter::source();
                filter.owner = Some(PlayerFilter::Opponent);
                return Ok(TriggerSpec::PutIntoGraveyardFromZone {
                    filter,
                    from: Zone::Battlefield,
                });
            }
            let mut filter = parse_object_filter_lexed(subject_tokens, false).map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported filter in put-into-opponents-graveyard-from-battlefield trigger clause (clause: '{}')",
                    words.join(" ")
                ))
            })?;
            filter.zone = None;
            filter.controller = None;
            filter.owner = Some(PlayerFilter::Opponent);
            return Ok(TriggerSpec::PutIntoGraveyardFromZone {
                filter,
                from: Zone::Battlefield,
            });
        }
    }

    if let Some(put_word_idx) = find_index(&words, |word| *word == "put" || *word == "puts")
        && let Some(source_controller) = parse_trigger_subject_player_filter(&words[..put_word_idx])
        && let Some(counter_word_idx) =
            find_index(&words, |word| *word == "counter" || *word == "counters")
        && counter_word_idx > put_word_idx
        && matches!(
            words.get(counter_word_idx + 1).copied(),
            Some("on") | Some("onto")
        )
    {
        let word_view = ActivationRestrictionCompatWords::new(tokens);
        let descriptor_word_start = put_word_idx + 1;
        let descriptor_token_start = word_view
            .token_index_for_word_index(descriptor_word_start)
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing counter descriptor in trigger clause (clause: '{}')",
                    words.join(" ")
                ))
            })?;
        let descriptor_token_end = word_view
            .token_index_for_word_index(counter_word_idx)
            .unwrap_or(tokens.len());
        let descriptor_span = &tokens[descriptor_token_start..descriptor_token_end];
        let one_or_more = ActivationRestrictionCompatWords::new(descriptor_span)
            .slice_eq(0, &["one", "or", "more"]);
        let counter_descriptor_tokens = &tokens[descriptor_token_start..(descriptor_token_end + 1)];
        let counter_type = parse_counter_type_from_tokens(counter_descriptor_tokens);

        let object_word_start = counter_word_idx + 2;
        let object_token_start = word_view
            .token_index_for_word_index(object_word_start)
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing counter recipient in trigger clause (clause: '{}')",
                    words.join(" ")
                ))
            })?;
        let mut object_tokens = trim_commas(&tokens[object_token_start..]);
        let object_view = ActivationRestrictionCompatWords::new(&object_tokens);
        if matches!(object_view.first(), Some("a" | "an" | "the")) {
            let start = object_view
                .token_index_for_word_index(1)
                .unwrap_or(object_tokens.len());
            object_tokens = object_tokens[start..].to_vec();
        }
        if object_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing counter recipient in trigger clause (clause: '{}')",
                words.join(" ")
            )));
        }
        let filter = parse_object_filter_lexed(&object_tokens, false).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported counter recipient filter in trigger clause (clause: '{}')",
                words.join(" ")
            ))
        })?;

        return Ok(TriggerSpec::CounterPutOn {
            filter,
            counter_type,
            source_controller: Some(source_controller),
            one_or_more,
        });
    }

    if words.as_slice() == ["players", "finish", "voting"]
        || words.as_slice() == ["players", "finished", "voting"]
    {
        return Ok(TriggerSpec::KeywordAction {
            action: crate::events::KeywordActionKind::Vote,
            player: PlayerFilter::Any,
            source_filter: None,
        });
    }

    if words.as_slice() == ["you", "cycle", "this", "card"]
        || words.as_slice() == ["you", "cycled", "this", "card"]
    {
        return Ok(TriggerSpec::KeywordActionFromSource {
            action: crate::events::KeywordActionKind::Cycle,
            player: PlayerFilter::You,
        });
    }

    if words.as_slice() == ["you", "cycle", "or", "discard", "a", "card"]
        || words.as_slice() == ["you", "cycle", "or", "discard", "card"]
    {
        return Ok(TriggerSpec::Either(
            Box::new(TriggerSpec::KeywordAction {
                action: crate::events::KeywordActionKind::Cycle,
                player: PlayerFilter::You,
                source_filter: None,
            }),
            Box::new(TriggerSpec::PlayerDiscardsCard {
                player: PlayerFilter::You,
                filter: None,
                cause_controller: None,
                effect_like_only: false,
            }),
        ));
    }

    if words.as_slice() == ["you", "commit", "a", "crime"] {
        return Ok(TriggerSpec::KeywordAction {
            action: crate::events::KeywordActionKind::CommitCrime,
            player: PlayerFilter::You,
            source_filter: None,
        });
    }

    if words.as_slice() == ["an", "opponent", "commits", "a", "crime"]
        || words.as_slice() == ["opponent", "commits", "a", "crime"]
        || words.as_slice() == ["opponents", "commit", "a", "crime"]
    {
        return Ok(TriggerSpec::KeywordAction {
            action: crate::events::KeywordActionKind::CommitCrime,
            player: PlayerFilter::Opponent,
            source_filter: None,
        });
    }

    if words.as_slice() == ["a", "player", "commits", "a", "crime"]
        || words.as_slice() == ["a", "player", "commit", "a", "crime"]
    {
        return Ok(TriggerSpec::KeywordAction {
            action: crate::events::KeywordActionKind::CommitCrime,
            player: PlayerFilter::Any,
            source_filter: None,
        });
    }

    if words.as_slice() == ["you", "unlock", "this", "door"]
        || words.as_slice() == ["you", "unlocked", "this", "door"]
    {
        return Ok(TriggerSpec::KeywordActionFromSource {
            action: crate::events::KeywordActionKind::UnlockDoor,
            player: PlayerFilter::You,
        });
    }

    if words.len() == 3
        && words[0] == "you"
        && words[1] == "expend"
        && let Some(amount) = parse_cardinal_u32(words[2])
    {
        return Ok(TriggerSpec::Expend {
            player: PlayerFilter::You,
            amount,
        });
    }

    if words.len() == 4
        && (words.as_slice()[..3] == ["an", "opponent", "expends"]
            || words.as_slice()[..3] == ["an", "opponent", "expend"])
        && let Some(amount) = parse_cardinal_u32(words[3])
    {
        return Ok(TriggerSpec::Expend {
            player: PlayerFilter::Opponent,
            amount,
        });
    }

    if words.len() == 3
        && (words.as_slice()[..2] == ["opponent", "expends"]
            || words.as_slice()[..2] == ["opponent", "expend"])
        && let Some(amount) = parse_cardinal_u32(words[2])
    {
        return Ok(TriggerSpec::Expend {
            player: PlayerFilter::Opponent,
            amount,
        });
    }

    if words.as_slice() == ["the", "ring", "tempts", "you"] {
        return Ok(TriggerSpec::KeywordAction {
            action: crate::events::KeywordActionKind::RingTemptsYou,
            player: PlayerFilter::You,
            source_filter: None,
        });
    }

    if let Some(cycle_word_idx) = find_index(&words, |word| {
        matches!(
            crate::events::KeywordActionKind::from_trigger_word(word),
            Some(crate::events::KeywordActionKind::Cycle)
        )
    }) {
        let subject_words = &words[..cycle_word_idx];
        if let Some(player) = parse_trigger_subject_player_filter(subject_words) {
            let tail_words = &words[cycle_word_idx + 1..];
            if tail_words == ["a", "card"] || tail_words == ["card"] {
                return Ok(TriggerSpec::KeywordAction {
                    action: crate::events::KeywordActionKind::Cycle,
                    player,
                    source_filter: None,
                });
            }
            if tail_words == ["another", "card"] {
                return Ok(TriggerSpec::KeywordAction {
                    action: crate::events::KeywordActionKind::Cycle,
                    player,
                    source_filter: Some(ObjectFilter::default().other()),
                });
            }
        }
    }

    if let Some(exert_word_idx) = find_index(&words, |word| {
        matches!(
            crate::events::KeywordActionKind::from_trigger_word(word),
            Some(crate::events::KeywordActionKind::Exert)
        )
    }) {
        let subject = &words[..exert_word_idx];
        if let Some(player) = parse_trigger_subject_player_filter(subject) {
            let tail = &words[exert_word_idx + 1..];
            if tail == ["a", "creature"] || tail == ["creature"] {
                return Ok(TriggerSpec::KeywordAction {
                    action: crate::events::KeywordActionKind::Exert,
                    player,
                    source_filter: Some(ObjectFilter::creature()),
                });
            }
        }
    }

    if let Some(explore_word_idx) = find_index(&words, |word| {
        matches!(
            crate::events::KeywordActionKind::from_trigger_word(word),
            Some(crate::events::KeywordActionKind::Explore)
        )
    }) {
        let subject_tokens = &tokens[..explore_word_idx];
        if let Some(filter) = parse_trigger_subject_filter_lexed(subject_tokens)?
            && words[explore_word_idx + 1..].is_empty()
        {
            return Ok(TriggerSpec::KeywordAction {
                action: crate::events::KeywordActionKind::Explore,
                player: PlayerFilter::Any,
                source_filter: Some(filter),
            });
        }
    }

    if let Some(put_word_idx) = find_index(&words, |word| *word == "put" || *word == "puts") {
        let subject = &words[..put_word_idx];
        if let Some(player) = parse_trigger_subject_player_filter(subject) {
            let tail = &words[put_word_idx + 1..];
            let has_name_sticker = contains_word_sequence(tail, &["name", "sticker"]);
            let has_on = slice_contains(&tail, &"on");
            if has_name_sticker && has_on {
                return Ok(TriggerSpec::KeywordAction {
                    action: crate::events::KeywordActionKind::NameSticker,
                    player,
                    source_filter: None,
                });
            }
        }
    }

    if slice_ends_with(&words, &["becomes", "tapped"])
        && let Some(becomes_idx) = find_index(tokens, |token| token.is_word("becomes"))
        && tokens
            .get(becomes_idx + 1)
            .is_some_and(|token| token.is_word("tapped"))
    {
        let subject_tokens = &tokens[..becomes_idx];
        return Ok(match parse_trigger_subject_filter_lexed(subject_tokens)? {
            Some(filter) => TriggerSpec::PermanentBecomesTapped(filter),
            None => TriggerSpec::ThisBecomesTapped,
        });
    }

    if words.as_slice() == ["this", "creature", "becomes", "tapped"]
        || words.as_slice() == ["this", "becomes", "tapped"]
        || words.as_slice() == ["becomes", "tapped"]
    {
        return Ok(TriggerSpec::ThisBecomesTapped);
    }

    if words.as_slice() == ["this", "creature", "becomes", "untapped"]
        || words.as_slice() == ["this", "becomes", "untapped"]
        || words.as_slice() == ["becomes", "untapped"]
    {
        return Ok(TriggerSpec::ThisBecomesUntapped);
    }

    if words.as_slice() == ["this", "creature", "becomes", "monstrous"]
        || words.as_slice() == ["this", "permanent", "becomes", "monstrous"]
        || words.as_slice() == ["this", "becomes", "monstrous"]
        || words.as_slice() == ["becomes", "monstrous"]
    {
        return Ok(TriggerSpec::ThisBecomesMonstrous);
    }

    if words.as_slice() == ["this", "creature", "is", "turned", "face", "up"]
        || words.as_slice() == ["this", "permanent", "is", "turned", "face", "up"]
        || words.as_slice() == ["this", "is", "turned", "face", "up"]
    {
        return Ok(TriggerSpec::ThisTurnedFaceUp);
    }

    if slice_ends_with(&words, &["is", "turned", "face", "up"])
        || slice_ends_with(&words, &["are", "turned", "face", "up"])
    {
        let subject_tokens = ActivationRestrictionCompatWords::new(tokens)
            .token_index_for_word_index(words.len().saturating_sub(4))
            .map(|idx| &tokens[..idx])
            .unwrap_or_default();
        return Ok(match parse_trigger_subject_filter_lexed(subject_tokens)? {
            Some(filter) => TriggerSpec::TurnedFaceUp(filter),
            None => TriggerSpec::ThisTurnedFaceUp,
        });
    }

    if let Some(becomes_idx) = find_index(&words, |word| *word == "becomes")
        && words.get(becomes_idx + 1).copied() == Some("the")
        && words.get(becomes_idx + 2).copied() == Some("target")
        && words.get(becomes_idx + 3).copied() == Some("of")
    {
        let subject_words = &words[..becomes_idx];
        let subject_tokens = ActivationRestrictionCompatWords::new(tokens)
            .token_index_for_word_index(becomes_idx)
            .map(|idx| &tokens[..idx])
            .unwrap_or_default();
        let subject_filter = parse_trigger_subject_filter_lexed(subject_tokens)?;
        let subject_is_source =
            subject_words.is_empty() || is_source_reference_words(subject_words);
        if subject_is_source {
            let tail_word_start = becomes_idx + 4;
            let tail_words = &words[tail_word_start..];
            if let Some(source_controller) = parse_spell_or_ability_controller_tail(tail_words) {
                return Ok(TriggerSpec::BecomesTargetedBySourceController {
                    target: ObjectFilter::source(),
                    source_controller,
                });
            }
            if tail_words == ["a", "spell", "or", "ability"]
                || tail_words == ["spell", "or", "ability"]
            {
                return Ok(TriggerSpec::ThisBecomesTargeted);
            }
            if tail_words
                .last()
                .is_some_and(|word| *word == "spell" || *word == "spells")
            {
                let tail_token_start = ActivationRestrictionCompatWords::new(tokens)
                    .token_index_for_word_index(tail_word_start)
                    .unwrap_or(tokens.len());
                let spell_filter_tokens = trim_commas(&tokens[tail_token_start..]);
                let spell_filter =
                    parse_object_filter_lexed(&spell_filter_tokens, false).map_err(|_| {
                        CardTextError::ParseError(format!(
                            "unsupported spell filter in becomes-targeted trigger clause (clause: '{}')",
                            words.join(" ")
                        ))
                    })?;
                return Ok(TriggerSpec::ThisBecomesTargetedBySpell(spell_filter));
            }
        } else {
            let tail_word_start = becomes_idx + 4;
            let tail_words = &words[tail_word_start..];
            if let Some(source_controller) = parse_spell_or_ability_controller_tail(tail_words)
                && let Some(filter) = subject_filter.clone()
            {
                return Ok(TriggerSpec::BecomesTargetedBySourceController {
                    target: filter,
                    source_controller,
                });
            }
            if (tail_words == ["a", "spell", "or", "ability"]
                || tail_words == ["spell", "or", "ability"])
                && let Some(filter) = subject_filter
            {
                return Ok(TriggerSpec::BecomesTargeted(filter));
            }
        }
    }

    if slice_ends_with(&words, &["is", "dealt", "damage"])
        && words.len() >= 4
        && !slice_starts_with(&words, &["this", "creature", "is", "dealt", "damage"])
        && !slice_starts_with(&words, &["this", "is", "dealt", "damage"])
    {
        let is_word_idx = words.len().saturating_sub(3);
        let is_token_idx = ActivationRestrictionCompatWords::new(tokens)
            .token_index_for_word_index(is_word_idx)
            .unwrap_or(tokens.len());
        let subject_tokens = &tokens[..is_token_idx];
        if let Some(filter) = parse_trigger_subject_filter_lexed(subject_tokens)? {
            return Ok(TriggerSpec::IsDealtDamage(filter));
        }
    }

    if slice_starts_with(&words, &["this", "creature", "is", "dealt", "damage"])
        || slice_starts_with(&words, &["this", "is", "dealt", "damage"])
    {
        return Ok(TriggerSpec::ThisIsDealtDamage);
    }

    if (slice_starts_with(&words, &["this", "creature", "deals"])
        || slice_starts_with(&words, &["this", "permanent", "deals"])
        || slice_starts_with(&words, &["this", "deals"]))
        && let Some(deals_idx) = find_index(tokens, |token| {
            token.is_word("deal") || token.is_word("deals")
        })
        && let Some(damage_idx_rel) =
            find_index(&tokens[deals_idx + 1..], |token| token.is_word("damage"))
    {
        let damage_idx = deals_idx + 1 + damage_idx_rel;
        if let Some(to_idx_rel) = find_index(&tokens[damage_idx + 1..], |token| token.is_word("to"))
        {
            let to_idx = damage_idx + 1 + to_idx_rel;
            let amount_tokens = trim_commas(&tokens[deals_idx + 1..damage_idx]);
            if !amount_tokens
                .first()
                .is_some_and(|token| token.is_word("combat"))
            {
                let amount_view = ActivationRestrictionCompatWords::new(&amount_tokens);
                let amount_words = amount_view.to_word_refs();
                if let Some((amount, _)) =
                    parse_filter_comparison_tokens("damage amount", &amount_words, &words)?
                {
                    let target_tokens = split_target_clause_before_comma(&tokens[to_idx + 1..]);
                    let target_view = ActivationRestrictionCompatWords::new(&target_tokens);
                    let target_words = target_view.to_word_refs();
                    if let Some(player) = parse_trigger_subject_player_filter(&target_words) {
                        return Ok(TriggerSpec::ThisDealsDamageToPlayer {
                            player,
                            amount: Some(amount),
                        });
                    }
                }
            }
        }
    }

    if (slice_starts_with(&words, &["this", "creature", "deals", "damage", "to"])
        || slice_starts_with(&words, &["this", "permanent", "deals", "damage", "to"])
        || slice_starts_with(&words, &["this", "deals", "damage", "to"]))
        && let Some(to_idx) = find_index(tokens, |token| token.is_word("to"))
    {
        let target_tokens = split_target_clause_before_comma(&tokens[to_idx + 1..]);
        if target_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing damage recipient filter in trigger clause (clause: '{}')",
                words.join(" ")
            )));
        }
        let target_view = ActivationRestrictionCompatWords::new(&target_tokens);
        let target_words = target_view.to_word_refs();
        if let Some(player) = parse_trigger_subject_player_filter(&target_words) {
            return Ok(TriggerSpec::ThisDealsDamageToPlayer {
                player,
                amount: None,
            });
        }
        let target_filter = parse_object_filter_lexed(&target_tokens, false).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported damage recipient filter in trigger clause (clause: '{}')",
                words.join(" ")
            ))
        })?;
        return Ok(TriggerSpec::ThisDealsDamageTo(target_filter));
    }

    if slice_starts_with(&words, &["this", "creature", "deals", "damage"])
        || slice_starts_with(&words, &["this", "permanent", "deals", "damage"])
        || slice_starts_with(&words, &["this", "deals", "damage"])
    {
        return Ok(TriggerSpec::ThisDealsDamage);
    }

    if has_deal
        && slice_contains(&words, &"damage")
        && let Some(deals_idx) = find_index(tokens, |token| {
            token.is_word("deal") || token.is_word("deals")
        })
    {
        let subject_tokens = &tokens[..deals_idx];
        return Ok(match parse_trigger_subject_filter_lexed(subject_tokens)? {
            Some(filter) => TriggerSpec::DealsDamage(filter),
            None => TriggerSpec::ThisDealsDamage,
        });
    }

    if words.as_slice() == ["you", "gain", "life"] {
        return Ok(TriggerSpec::YouGainLife);
    }

    if words.len() >= 6
        && slice_ends_with(&words, &["during", "your", "turn"])
        && words[..words.len() - 3] == ["you", "gain", "life"]
    {
        return Ok(TriggerSpec::YouGainLifeDuringTurn(PlayerFilter::You));
    }

    if slice_ends_with(&words, &["lose", "life"]) || slice_ends_with(&words, &["loses", "life"]) {
        let subject = &words[..words.len().saturating_sub(2)];
        if let Some(player) = parse_trigger_subject_player_filter(subject) {
            return Ok(TriggerSpec::PlayerLosesLife(player));
        }
    }

    if words.len() >= 5
        && slice_ends_with(&words, &["during", "your", "turn"])
        && (slice_ends_with(&words[..words.len() - 3], &["lose", "life"])
            || slice_ends_with(&words[..words.len() - 3], &["loses", "life"]))
    {
        let subject = &words[..words.len() - 5];
        if let Some(player) = parse_trigger_subject_player_filter(subject) {
            return Ok(TriggerSpec::PlayerLosesLifeDuringTurn {
                player,
                during_turn: PlayerFilter::You,
            });
        }
    }

    if let Some(draw_word_idx) = find_index(&words, |word| *word == "draw" || *word == "draws") {
        let subject = &words[..draw_word_idx];
        if let Some(player) = parse_trigger_subject_player_filter(subject) {
            let tail = &words[draw_word_idx + 1..];
            if let Some(during_turn) = parse_not_during_turn_suffix(tail) {
                return Ok(TriggerSpec::PlayerDrawsCardNotDuringTurn {
                    player,
                    during_turn,
                });
            }
            if let Some(card_number) = parse_exact_draw_count_each_turn(tail) {
                return Ok(TriggerSpec::PlayerDrawsNthCardEachTurn {
                    player,
                    card_number,
                });
            }
        }
    }

    if slice_ends_with(&words, &["draw", "a", "card"])
        || slice_ends_with(&words, &["draws", "a", "card"])
    {
        let subject = &words[..words.len().saturating_sub(3)];
        if subject == ["you"] {
            return Ok(TriggerSpec::YouDrawCard);
        }
        if let Some(player) = parse_trigger_subject_player_filter(subject) {
            return Ok(TriggerSpec::PlayerDrawsCard(player));
        }
    }

    if words.as_slice()
        == [
            "a", "spell", "or", "ability", "an", "opponent", "controls", "causes", "you", "to",
            "discard", "this", "card",
        ]
    {
        return Ok(TriggerSpec::PlayerDiscardsCard {
            player: PlayerFilter::You,
            filter: Some(ObjectFilter::source()),
            cause_controller: Some(PlayerFilter::Opponent),
            effect_like_only: true,
        });
    }

    if let Some(discard_word_idx) =
        find_index(&words, |word| *word == "discard" || *word == "discards")
        && let Some(discard_token_idx) = ActivationRestrictionCompatWords::new(tokens)
            .token_index_for_word_index(discard_word_idx)
    {
        let subject_words = &words[..discard_word_idx];
        if let Some(player) = parse_trigger_subject_player_filter(subject_words) {
            if let Ok(filter) =
                parse_discard_trigger_card_filter(&tokens[discard_token_idx + 1..], &words)
            {
                return Ok(TriggerSpec::PlayerDiscardsCard {
                    player,
                    filter,
                    cause_controller: None,
                    effect_like_only: false,
                });
            }
        }
    }

    if let Some(reveal_word_idx) =
        find_index(&words, |word| *word == "reveal" || *word == "reveals")
        && let Some(player) = parse_trigger_subject_player_filter(&words[..reveal_word_idx])
    {
        let mut tail_tokens = trim_commas(
            &tokens[ActivationRestrictionCompatWords::new(tokens)
                .token_index_for_word_index(reveal_word_idx + 1)
                .unwrap_or(tokens.len())..],
        );
        let tail_view = ActivationRestrictionCompatWords::new(&tail_tokens);
        let tail_words = tail_view.to_word_refs();
        let from_source = slice_ends_with(&tail_words, &["this", "way"]);
        if from_source {
            let cutoff = ActivationRestrictionCompatWords::new(&tail_tokens)
                .token_index_for_word_index(tail_words.len().saturating_sub(2))
                .unwrap_or(tail_tokens.len());
            tail_tokens = trim_commas(&tail_tokens[..cutoff]);
        }
        if !tail_tokens.is_empty()
            && let Ok(mut filter) = parse_object_filter_lexed(&tail_tokens, false)
        {
            filter.zone = None;
            return Ok(TriggerSpec::PlayerRevealsCard {
                player,
                filter,
                from_source,
            });
        }
    }

    if let Some(sacrifice_word_idx) =
        find_index(&words, |word| *word == "sacrifice" || *word == "sacrifices")
        && let Some(sacrifice_token_idx) = ActivationRestrictionCompatWords::new(tokens)
            .token_index_for_word_index(sacrifice_word_idx)
    {
        let subject_words = &words[..sacrifice_word_idx];
        if let Some(player) = parse_trigger_subject_player_filter(subject_words) {
            let mut filter_tokens = &tokens[sacrifice_token_idx + 1..];
            let mut other = false;
            if filter_tokens
                .first()
                .is_some_and(|token| token.is_word("another") || token.is_word("other"))
            {
                other = true;
                filter_tokens = &filter_tokens[1..];
            }

            let filter = if filter_tokens.is_empty() {
                let mut filter = ObjectFilter::permanent();
                if other {
                    filter.other = true;
                }
                filter
            } else if filter_tokens
                .first()
                .is_some_and(|token| token.is_word("this") || token.is_word("it"))
            {
                let filter_word_view = ActivationRestrictionCompatWords::new(filter_tokens);
                let filter_words = filter_word_view.to_word_refs();
                let mut filter = ObjectFilter::source();
                if slice_contains(&filter_words, &"artifact") {
                    filter = filter.with_type(CardType::Artifact);
                } else if slice_contains(&filter_words, &"creature") {
                    filter = filter.with_type(CardType::Creature);
                } else if slice_contains(&filter_words, &"enchantment") {
                    filter = filter.with_type(CardType::Enchantment);
                } else if slice_contains(&filter_words, &"land") {
                    filter = filter.with_type(CardType::Land);
                } else if slice_contains(&filter_words, &"planeswalker") {
                    filter = filter.with_type(CardType::Planeswalker);
                }
                filter
            } else {
                parse_object_filter_lexed(filter_tokens, other).map_err(|_| {
                    CardTextError::ParseError(format!(
                        "unsupported sacrifice trigger filter (clause: '{}')",
                        words.join(" ")
                    ))
                })?
            };
            return Ok(TriggerSpec::PlayerSacrifices { player, filter });
        }
    }

    if let Some(last_word) = words.last().copied()
        && let Some(action) = crate::events::KeywordActionKind::from_trigger_word(last_word)
    {
        let subject = &words[..words.len().saturating_sub(1)];
        if is_source_reference_words(subject) {
            return Ok(TriggerSpec::KeywordActionFromSource {
                action,
                player: PlayerFilter::You,
            });
        }
        if subject.len() > 2 && is_source_reference_words(&subject[..2]) {
            let trailing_ok = subject[2..].iter().all(|word| {
                matches!(
                    *word,
                    "become" | "becomes" | "became" | "becoming" | "has" | "had"
                )
            });
            if trailing_ok {
                return Ok(TriggerSpec::KeywordActionFromSource {
                    action,
                    player: PlayerFilter::You,
                });
            }
        }
        if let Some(player) = parse_trigger_subject_player_filter(subject) {
            return Ok(TriggerSpec::KeywordAction {
                action,
                player,
                source_filter: None,
            });
        }
    }

    if words == ["you", "complete", "a", "dungeon"]
        || words == ["you", "completed", "a", "dungeon"]
        || words == ["you", "completes", "a", "dungeon"]
    {
        return Ok(TriggerSpec::KeywordAction {
            action: crate::events::KeywordActionKind::CompleteDungeon,
            player: PlayerFilter::You,
            source_filter: None,
        });
    }

    if slice_ends_with(&words, &["win", "a", "clash"])
        || slice_ends_with(&words, &["wins", "a", "clash"])
        || slice_ends_with(&words, &["won", "a", "clash"])
    {
        let subject = &words[..words.len().saturating_sub(3)];
        if let Some(player) = parse_trigger_subject_player_filter(subject) {
            return Ok(TriggerSpec::WinsClash { player });
        }
    }

    if let Some(counter_word_idx) =
        find_index(&words, |word| *word == "counter" || *word == "counters")
        && matches!(
            words.get(counter_word_idx + 1).copied(),
            Some("is") | Some("are")
        )
        && words.get(counter_word_idx + 2).copied() == Some("put")
        && matches!(
            words.get(counter_word_idx + 3).copied(),
            Some("on") | Some("onto")
        )
    {
        let word_view = ActivationRestrictionCompatWords::new(tokens);
        let one_or_more = slice_starts_with(&words, &["one", "or", "more"]);
        let descriptor_token_end = word_view
            .token_index_for_word_index(counter_word_idx)
            .unwrap_or(tokens.len());
        let counter_descriptor_tokens = &tokens[..(descriptor_token_end + 1)];
        let counter_type = parse_counter_type_from_tokens(counter_descriptor_tokens);

        let object_word_start = counter_word_idx + 4;
        let object_token_start = word_view
            .token_index_for_word_index(object_word_start)
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing counter recipient in trigger clause (clause: '{}')",
                    words.join(" ")
                ))
            })?;
        let mut object_tokens = trim_commas(&tokens[object_token_start..]);
        let object_view = ActivationRestrictionCompatWords::new(&object_tokens);
        if matches!(object_view.first(), Some("a" | "an" | "the")) {
            let start = object_view
                .token_index_for_word_index(1)
                .unwrap_or(object_tokens.len());
            object_tokens = object_tokens[start..].to_vec();
        }
        if object_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing counter recipient in trigger clause (clause: '{}')",
                words.join(" ")
            )));
        }
        let filter = parse_object_filter_lexed(&object_tokens, false).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported counter recipient filter in trigger clause (clause: '{}')",
                words.join(" ")
            ))
        })?;

        return Ok(TriggerSpec::CounterPutOn {
            filter,
            counter_type,
            source_controller: None,
            one_or_more,
        });
    }

    if let Some(attacks_word_idx) =
        find_index(&words, |word| *word == "attack" || *word == "attacks")
    {
        let tail_words = &words[attacks_word_idx + 1..];
        if tail_words == ["and", "isnt", "blocked"]
            || tail_words == ["and", "isn't", "blocked"]
            || tail_words == ["and", "is", "not", "blocked"]
        {
            let attacks_token_idx = ActivationRestrictionCompatWords::new(tokens)
                .token_index_for_word_index(attacks_word_idx)
                .unwrap_or(tokens.len());
            let subject_tokens = &tokens[..attacks_token_idx];
            return Ok(
                match parse_attack_trigger_subject_filter_lexed(subject_tokens)? {
                    Some(filter) => TriggerSpec::AttacksAndIsntBlocked(filter),
                    None => TriggerSpec::ThisAttacksAndIsntBlocked,
                },
            );
        }
    }

    if (slice_starts_with(&words, &["this", "creature", "blocks"])
        || slice_starts_with(&words, &["this", "blocks"]))
        && let Some(blocks_idx) = find_index(tokens, |token| {
            token.is_word("block") || token.is_word("blocks")
        })
    {
        let tail_tokens = trim_commas(&tokens[blocks_idx + 1..]);
        if !tail_tokens.is_empty() && !tail_tokens.first().is_some_and(|token| token.is_word("or"))
        {
            let blocked_filter = parse_object_filter_lexed(&tail_tokens, false).map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported blocked-object filter in trigger clause (clause: '{}')",
                    words.join(" ")
                ))
            })?;
            return Ok(TriggerSpec::ThisBlocksObject(blocked_filter));
        }
    }

    let words = if let Some(attacks_word_idx) =
        find_index(&words, |word| matches!(*word, "attack" | "attacks"))
    {
        let tail = &words[attacks_word_idx + 1..];
        if matches!(
            tail,
            ["a", "player"]
                | ["a", "planeswalker"]
                | ["a", "battle"]
                | ["the", "defending", "player"]
                | ["defending", "player"]
        ) {
            &words[..=attacks_word_idx]
        } else {
            &words
        }
    } else {
        &words
    };

    let last = words
        .last()
        .copied()
        .ok_or_else(|| CardTextError::ParseError("empty trigger clause".to_string()))?;

    match last {
        "attack" | "attacks" => {
            let attack_word_idx = words.len().saturating_sub(1);
            let attack_token_idx = ActivationRestrictionCompatWords::new(tokens)
                .token_index_for_word_index(attack_word_idx)
                .unwrap_or(tokens.len());
            let subject_tokens = &tokens[..attack_token_idx];
            let player_subject = trigger_subject_player_selector_lexed(subject_tokens).is_some();
            let one_or_more = ActivationRestrictionCompatWords::new(subject_tokens)
                .slice_eq(0, &["one", "or", "more"])
                || player_subject;
            Ok(
                match parse_attack_trigger_subject_filter_lexed(subject_tokens)? {
                    Some(filter) => {
                        if one_or_more {
                            TriggerSpec::AttacksOneOrMore(filter)
                        } else {
                            TriggerSpec::Attacks(filter)
                        }
                    }
                    None => TriggerSpec::ThisAttacks,
                },
            )
        }
        "block" | "blocks" => {
            let block_word_idx = words.len().saturating_sub(1);
            let block_token_idx = ActivationRestrictionCompatWords::new(tokens)
                .token_index_for_word_index(block_word_idx)
                .unwrap_or(tokens.len());
            let subject_tokens = &tokens[..block_token_idx];
            Ok(match parse_trigger_subject_filter_lexed(subject_tokens)? {
                Some(filter) => TriggerSpec::Blocks(filter),
                None => TriggerSpec::ThisBlocks,
            })
        }
        "dies" => {
            let dies_word_idx = words.len().saturating_sub(1);
            let dies_token_idx = ActivationRestrictionCompatWords::new(tokens)
                .token_index_for_word_index(dies_word_idx)
                .unwrap_or(tokens.len());
            let mut subject_tokens = &tokens[..dies_token_idx];
            if subject_tokens.is_empty() {
                return Ok(TriggerSpec::ThisDies);
            }

            if subject_tokens
                .first()
                .is_some_and(|token| token.is_word("this"))
            {
                let subject_word_view = ActivationRestrictionCompatWords::new(subject_tokens);
                let subject_words = subject_word_view.to_word_refs();
                if let Some(or_word_idx) =
                    find_word_sequence_start(&subject_words, &["or", "another"])
                {
                    let rhs_word_idx = or_word_idx + 2;
                    let rhs_token_idx = subject_word_view
                        .token_index_for_word_index(rhs_word_idx)
                        .unwrap_or(subject_tokens.len());
                    if rhs_token_idx < subject_tokens.len() {
                        let rhs_tokens = trim_edge_punctuation(&subject_tokens[rhs_token_idx..]);
                        if !rhs_tokens.is_empty()
                            && let Ok(filter) = parse_object_filter_lexed(&rhs_tokens, false)
                        {
                            return Ok(TriggerSpec::Either(
                                Box::new(TriggerSpec::ThisDies),
                                Box::new(TriggerSpec::Dies(filter)),
                            ));
                        }
                    }
                }
                if is_source_reference_words(&subject_words) {
                    return Ok(TriggerSpec::ThisDies);
                }
                return Err(CardTextError::ParseError(format!(
                    "unsupported this-prefixed dies trigger subject (clause: '{}')",
                    words.join(" ")
                )));
            }

            let subject_word_view = ActivationRestrictionCompatWords::new(subject_tokens);
            let subject_words = subject_word_view.to_word_refs();
            if subject_words.last().copied() == Some("haunts")
                && subject_words.first().copied() == Some("the")
                && subject_words.get(1).copied() == Some("creature")
            {
                return Ok(TriggerSpec::HauntedCreatureDies);
            }

            let mut other = false;
            if subject_tokens
                .first()
                .is_some_and(|token| token.is_word("another"))
            {
                other = true;
                subject_tokens = &subject_tokens[1..];
            }
            if subject_tokens.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "missing subject in dies trigger clause (clause: '{}')",
                    words.join(" ")
                )));
            }

            if let Some(damaged_by_trigger) =
                parse_damage_by_dies_trigger_lexed(subject_tokens, other, &words)?
            {
                return Ok(damaged_by_trigger);
            }

            if let Ok(filter) = parse_object_filter_lexed(subject_tokens, other) {
                return Ok(TriggerSpec::Dies(filter));
            }
            let mut normalized_subject_tokens = Vec::with_capacity(subject_tokens.len());
            let mut idx = 0usize;
            while idx < subject_tokens.len() {
                if subject_tokens[idx].is_word("and")
                    && subject_tokens
                        .get(idx + 1)
                        .is_some_and(|token| token.is_word("or"))
                {
                    idx += 1;
                    continue;
                }
                normalized_subject_tokens.push(subject_tokens[idx].clone());
                idx += 1;
            }
            if normalized_subject_tokens.len() != subject_tokens.len()
                && let Ok(filter) = parse_object_filter_lexed(&normalized_subject_tokens, other)
            {
                return Ok(TriggerSpec::Dies(filter));
            }

            Err(CardTextError::ParseError(format!(
                "unsupported dies trigger subject filter (clause: '{}')",
                words.join(" ")
            )))
        }
        _ if slice_contains(&words, &"beginning")
            && slice_contains(&words, &"end")
            && slice_contains(&words, &"step") =>
        {
            Ok(TriggerSpec::BeginningOfEndStep(
                parse_possessive_clause_player_filter(&words),
            ))
        }
        _ if slice_contains(&words, &"beginning") && slice_contains(&words, &"upkeep") => Ok(
            TriggerSpec::BeginningOfUpkeep(parse_possessive_clause_player_filter(&words)),
        ),
        _ if slice_contains(&words, &"beginning")
            && slice_contains(&words, &"draw")
            && slice_contains(&words, &"step") =>
        {
            Ok(TriggerSpec::BeginningOfDrawStep(
                parse_possessive_clause_player_filter(&words),
            ))
        }
        _ if slice_contains(&words, &"beginning") && slice_contains(&words, &"combat") => Ok(
            TriggerSpec::BeginningOfCombat(parse_possessive_clause_player_filter(&words)),
        ),
        _ if slice_contains(&words, &"beginning")
            && slice_contains(&words, &"first")
            && slice_contains(&words, &"main")
            && slice_contains(&words, &"phase") =>
        {
            Ok(TriggerSpec::BeginningOfPrecombatMain(
                parse_possessive_clause_player_filter(&words),
            ))
        }
        _ if slice_contains(&words, &"beginning")
            && slice_contains(&words, &"second")
            && slice_contains(&words, &"main")
            && slice_contains(&words, &"phase") =>
        {
            Ok(TriggerSpec::BeginningOfPostcombatMain(
                parse_possessive_clause_player_filter(&words),
            ))
        }
        _ if slice_contains(&words, &"beginning")
            && slice_contains(&words, &"precombat")
            && slice_contains(&words, &"main") =>
        {
            Ok(TriggerSpec::BeginningOfPrecombatMain(
                parse_possessive_clause_player_filter(&words),
            ))
        }
        _ if slice_contains(&words, &"beginning")
            && slice_contains(&words, &"postcombat")
            && slice_contains(&words, &"main") =>
        {
            Ok(TriggerSpec::BeginningOfPostcombatMain(
                parse_possessive_clause_player_filter(&words),
            ))
        }
        _ => Err(CardTextError::ParseError(format!(
            "unsupported trigger clause (clause: '{}')",
            words.join(" ")
        ))),
    }
}

