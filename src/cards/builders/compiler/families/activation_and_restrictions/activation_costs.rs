pub(crate) fn parse_cant_clauses(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    if let Some((condition, remainder)) = strip_static_restriction_condition(tokens)?
        && remainder != tokens
    {
        let Some(abilities) = parse_cant_clauses(&remainder)? else {
            return Ok(None);
        };
        let conditioned = abilities
            .into_iter()
            .map(|ability| ability.with_condition(condition.clone()).unwrap_or(ability))
            .collect::<Vec<_>>();
        return Ok(Some(conditioned));
    }

    let normalized_words_storage = normalize_cant_words(tokens);
    let normalized_words = normalized_words_storage
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    let is_direct_temporary_cast_restriction =
        contains_word_sequence(&normalized_words, &["this", "turn"])
            && !slice_contains(&normalized_words, &"unless")
            && !slice_contains(&normalized_words, &"who")
            && (slice_starts_with(&normalized_words, &["your", "opponents", "cant", "cast"])
                || slice_starts_with(&normalized_words, &["each", "opponent", "cant", "cast"])
                || slice_starts_with(&normalized_words, &["each", "player", "cant", "cast"])
                || slice_starts_with(&normalized_words, &["players", "cant", "cast"])
                || slice_starts_with(&normalized_words, &["target", "player", "cant", "cast"])
                || slice_starts_with(&normalized_words, &["you", "cant", "cast"]));
    if is_direct_temporary_cast_restriction {
        return Ok(None);
    }

    if tokens.iter().any(|token| token.is_word("and"))
        && let Some((neg_start, _)) = find_negation_span(tokens)
        && tokens[..neg_start]
            .iter()
            .any(|token| token.is_word("get") || token.is_word("gets"))
    {
        return Ok(None);
    }

    if find_negation_span(tokens).is_none() {
        return Ok(None);
    }

    if let Some(segments) = split_cant_clause_on_or(tokens) {
        let mut abilities = Vec::new();
        for segment in segments {
            let Some(ability) = parse_cant_clause(&segment)? else {
                return Err(CardTextError::ParseError(format!(
                    "unsupported cant clause segment (clause: '{}')",
                    crate::cards::builders::compiler::token_word_refs(&segment).join(" ")
                )));
            };
            abilities.push(ability);
        }
        if !abilities.is_empty() {
            return Ok(Some(abilities));
        }
    }

    if tokens.iter().any(|token| token.is_word("and")) {
        let segments = grammar::split_lexed_slices_on_and(tokens);
        if segments.is_empty() {
            return Ok(None);
        }
        let shared_subject = find_negation_span(&segments[0])
            .map(|(neg_start, _)| trim_commas(&segments[0][..neg_start]))
            .unwrap_or_default();

        let mut abilities = Vec::new();
        for (idx, segment) in segments.iter().enumerate() {
            if find_negation_span(segment).is_none() {
                continue;
            }
            let mut expanded = segment.to_vec();
            if idx > 0
                && !shared_subject.is_empty()
                && matches!(find_negation_span(segment), Some((0, _)))
            {
                let mut with_subject = shared_subject.clone();
                with_subject.extend(segment.iter().cloned());
                expanded = with_subject;
            } else if idx > 0
                && !shared_subject.is_empty()
                && starts_with_possessive_activated_ability_subject(segment)
            {
                let mut with_subject = shared_subject.clone();
                with_subject.extend(segment.iter().skip(1).cloned());
                expanded = with_subject;
            }
            let Some(ability) = parse_cant_clause(&expanded)? else {
                return Err(CardTextError::ParseError(format!(
                    "unsupported cant clause segment (clause: '{}')",
                    crate::cards::builders::compiler::token_word_refs(segment).join(" ")
                )));
            };
            abilities.push(ability);
        }

        if abilities.is_empty() {
            return Ok(None);
        }
        return Ok(Some(abilities));
    }

    parse_cant_clause(tokens).map(|ability| ability.map(|ability| vec![ability]))
}

fn split_cant_clause_on_or(tokens: &[OwnedLexToken]) -> Option<Vec<Vec<OwnedLexToken>>> {
    let (neg_start, neg_end) = find_negation_span(tokens)?;
    let subject_tokens = trim_commas(&tokens[..neg_start]);
    let remainder_tokens = trim_commas(&tokens[neg_end..]);
    let remainder_words_storage = normalize_cant_words(&remainder_tokens);
    let remainder_words = remainder_words_storage
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    if slice_starts_with(&remainder_words, &["attack", "or", "block"]) {
        return None;
    }
    let or_idx = find_index(&remainder_tokens, |token: &OwnedLexToken| {
        token.is_word("or")
    })?;
    let tail = trim_commas(&remainder_tokens[or_idx + 1..]);
    let starts_new_restriction = tail.first().is_some_and(|token| {
        token.is_word("cast")
            || token.is_word("activate")
            || token.is_word("attack")
            || token.is_word("block")
            || token.is_word("be")
    });
    if !starts_new_restriction {
        return None;
    }

    let negation_tokens = tokens[neg_start..neg_end].to_vec();
    let mut first = subject_tokens.clone();
    first.extend(negation_tokens.iter().cloned());
    first.extend(trim_commas(&remainder_tokens[..or_idx]).iter().cloned());

    let mut second = subject_tokens.clone();
    second.extend(negation_tokens.iter().cloned());
    second.extend(tail.iter().cloned());

    Some(vec![first, second])
}

pub(crate) fn parse_cant_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if let Some((condition, remainder)) = strip_static_restriction_condition(tokens)?
        && remainder != tokens
    {
        let Some(ability) = parse_cant_clause(&remainder)? else {
            return Ok(None);
        };
        if let Some(conditioned) = ability.with_condition(condition.clone()) {
            return Ok(Some(conditioned));
        }
        if let Some(parsed) = parse_cant_restriction_clause(&remainder)?
            && parsed.target.is_none()
        {
            return Ok(Some(
                StaticAbility::restriction(
                    parsed.restriction,
                    format_negated_restriction_display(tokens),
                )
                .with_condition(condition)
                .unwrap_or(ability),
            ));
        }
        return Ok(Some(ability));
    }

    let normalized_storage = normalize_cant_words(tokens);
    let normalized = normalized_storage
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();

    if let Some(rest) = slice_strip_prefix(
        &normalized,
        &[
            "creatures",
            "cant",
            "attack",
            "you",
            "unless",
            "their",
            "controller",
            "pays",
        ],
    ) && rest.get(1..)
        == Some(&[
            "for",
            "each",
            "creature",
            "they",
            "control",
            "thats",
            "attacking",
            "you",
        ])
    {
        if let Ok(amount) = rest[0].parse::<u32>() {
            return Ok(Some(
                StaticAbility::cant_attack_you_unless_controller_pays_per_attacker(amount),
            ));
        }
    }

    let is_collective_restraint_domain_attack_tax = slice_starts_with(
        &normalized,
        &[
            "creatures",
            "cant",
            "attack",
            "you",
            "unless",
            "their",
            "controller",
            "pays",
            "x",
            "for",
            "each",
            "creature",
            "they",
            "control",
            "thats",
            "attacking",
            "you",
        ],
    ) && (slice_ends_with(
        &normalized,
        &[
            "where", "x", "is", "the", "number", "of", "basic", "land", "types", "among", "lands",
            "you", "control",
        ],
    ) || slice_ends_with(
        &normalized,
        &[
            "where", "x", "is", "the", "number", "of", "basic", "land", "type", "among", "lands",
            "you", "control",
        ],
    ));
    if is_collective_restraint_domain_attack_tax {
        return Ok(Some(
            StaticAbility::cant_attack_you_unless_controller_pays_per_attacker_basic_land_types_among_lands_you_control(),
        ));
    }

    let starts_with_cant_be_blocked_by =
        slice_starts_with(
            &normalized,
            &["this", "creature", "cant", "be", "blocked", "by"],
        ) || slice_starts_with(&normalized, &["this", "cant", "be", "blocked", "by"])
            || slice_starts_with(&normalized, &["cant", "be", "blocked", "by"]);
    if starts_with_cant_be_blocked_by {
        let mut idx = if slice_starts_with(
            &normalized,
            &["this", "creature", "cant", "be", "blocked", "by"],
        ) {
            6
        } else if slice_starts_with(&normalized, &["this", "cant", "be", "blocked", "by"]) {
            5
        } else {
            4
        };
        if normalized
            .get(idx)
            .is_some_and(|word| *word == "creature" || *word == "creatures")
        {
            idx += 1;
        }
        if normalized.get(idx) == Some(&"more") && normalized.get(idx + 1) == Some(&"than") {
            let amount_word = normalized.get(idx + 2).copied().ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing blocker threshold in cant-blocked clause (clause: '{}')",
                    normalized.join(" ")
                ))
            })?;
            let amount_tokens = vec![OwnedLexToken::word(
                amount_word.to_string(),
                TextSpan::synthetic(),
            )];
            let (max_blockers, used) = parse_number(&amount_tokens).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "invalid blocker threshold in cant-blocked clause (clause: '{}')",
                    normalized.join(" ")
                ))
            })?;
            if used != 1 {
                return Err(CardTextError::ParseError(format!(
                    "invalid blocker threshold in cant-blocked clause (clause: '{}')",
                    normalized.join(" ")
                )));
            }
            let noun = normalized.get(idx + 3).copied().ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing blocker noun in cant-blocked clause (clause: '{}')",
                    normalized.join(" ")
                ))
            })?;
            if noun != "creature" && noun != "creatures" {
                return Err(CardTextError::ParseError(format!(
                    "unsupported blocker noun in cant-blocked clause (clause: '{}')",
                    normalized.join(" ")
                )));
            }
            if idx + 4 != normalized.len() {
                return Err(CardTextError::ParseError(format!(
                    "unsupported cant-be-blocked max-blockers clause tail (clause: '{}')",
                    normalized.join(" ")
                )));
            }
            return Ok(Some(StaticAbility::cant_be_blocked_by_more_than(
                max_blockers as usize,
            )));
        }
        if normalized.get(idx) == Some(&"with") && normalized.get(idx + 1) == Some(&"power") {
            let amount_word = normalized.get(idx + 2).copied().ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing power threshold in cant-blocked clause (clause: '{}')",
                    normalized.join(" ")
                ))
            })?;
            let amount_tokens = vec![OwnedLexToken::word(
                amount_word.to_string(),
                TextSpan::synthetic(),
            )];
            let (threshold, used) = parse_number(&amount_tokens).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "invalid power threshold in cant-blocked clause (clause: '{}')",
                    normalized.join(" ")
                ))
            })?;
            if used != 1 || normalized.get(idx + 3) != Some(&"or") || idx + 5 != normalized.len() {
                return Err(CardTextError::ParseError(format!(
                    "unsupported cant-be-blocked power clause tail (clause: '{}')",
                    normalized.join(" ")
                )));
            }

            return match normalized.get(idx + 4) {
                Some(&"less") => Ok(Some(StaticAbility::cant_be_blocked_by_power_or_less(
                    threshold as i32,
                ))),
                Some(&"greater") | Some(&"more") => Ok(Some(
                    StaticAbility::cant_be_blocked_by_power_or_greater(threshold as i32),
                )),
                _ => Err(CardTextError::ParseError(format!(
                    "unsupported cant-be-blocked power clause tail (clause: '{}')",
                    normalized.join(" ")
                ))),
            };
        }

        if normalized.get(idx) == Some(&"with")
            && normalized.get(idx + 1) == Some(&"flying")
            && idx + 2 == normalized.len()
        {
            return Ok(Some(StaticAbility::restriction(
                crate::effect::Restriction::block_specific_attacker(
                    ObjectFilter::creature()
                        .with_static_ability(crate::static_abilities::StaticAbilityId::Flying),
                    ObjectFilter::source(),
                ),
                "this creature can't be blocked by creatures with flying".to_string(),
            )));
        }
        if let Some(color_word) = normalized.get(idx).copied()
            && normalized
                .get(idx + 1)
                .is_some_and(|word| *word == "creature" || *word == "creatures")
            && idx + 2 == normalized.len()
            && let Some(color) = parse_color(color_word)
        {
            return Ok(Some(StaticAbility::restriction(
                crate::effect::Restriction::block_specific_attacker(
                    ObjectFilter::creature().with_colors(crate::color::ColorSet::from(color)),
                    ObjectFilter::source(),
                ),
                format!("this creature can't be blocked by {color_word} creatures"),
            )));
        }

        if normalized
            .get(idx)
            .is_some_and(|word| *word == "wall" || *word == "walls")
            && idx + 1 == normalized.len()
        {
            return Ok(Some(StaticAbility::restriction(
                crate::effect::Restriction::block_specific_attacker(
                    ObjectFilter::creature().with_subtype(Subtype::Wall),
                    ObjectFilter::source(),
                ),
                "this creature can't be blocked by walls".to_string(),
            )));
        }
    }

    let starts_with_cant_be_blocked_except_by =
        slice_starts_with(
            &normalized,
            &["this", "creature", "cant", "be", "blocked", "except", "by"],
        ) || slice_starts_with(
            &normalized,
            &["this", "cant", "be", "blocked", "except", "by"],
        ) || slice_starts_with(&normalized, &["cant", "be", "blocked", "except", "by"]);
    if starts_with_cant_be_blocked_except_by {
        let idx = if slice_starts_with(
            &normalized,
            &["this", "creature", "cant", "be", "blocked", "except", "by"],
        ) {
            7
        } else if slice_starts_with(
            &normalized,
            &["this", "cant", "be", "blocked", "except", "by"],
        ) {
            6
        } else {
            5
        };
        if let Some(color_word) = normalized.get(idx)
            && normalized
                .get(idx + 1)
                .is_some_and(|word| *word == "creature" || *word == "creatures")
            && idx + 2 == normalized.len()
            && let Some(color) = parse_color(color_word)
        {
            return Ok(Some(StaticAbility::restriction(
                crate::effect::Restriction::block_specific_attacker(
                    ObjectFilter::creature().without_colors(crate::color::ColorSet::from(color)),
                    ObjectFilter::source(),
                ),
                format!("this creature can't be blocked except by {color_word} creatures"),
            )));
        }
        if normalized.get(idx) == Some(&"artifact")
            && normalized
                .get(idx + 1)
                .is_some_and(|word| *word == "creature" || *word == "creatures")
            && idx + 2 == normalized.len()
        {
            return Ok(Some(StaticAbility::restriction(
                crate::effect::Restriction::block_specific_attacker(
                    ObjectFilter::creature().without_type(CardType::Artifact),
                    ObjectFilter::source(),
                ),
                "this creature can't be blocked except by artifact creatures".to_string(),
            )));
        }
        if normalized
            .get(idx)
            .is_some_and(|word| *word == "wall" || *word == "walls")
            && idx + 1 == normalized.len()
        {
            return Ok(Some(StaticAbility::restriction(
                crate::effect::Restriction::block_specific_attacker(
                    ObjectFilter::creature().without_subtype(Subtype::Wall),
                    ObjectFilter::source(),
                ),
                "this creature can't be blocked except by walls".to_string(),
            )));
        }
    }

    let starts_with_cant_attack_unless_defending_player = slice_starts_with(
        &normalized,
        &[
            "this",
            "creature",
            "cant",
            "attack",
            "unless",
            "defending",
            "player",
        ],
    ) || slice_starts_with(
        &normalized,
        &["this", "cant", "attack", "unless", "defending", "player"],
    );
    let cant_attack_unless_cast_creature_spell_tail = slice_ends_with(
        &normalized,
        &[
            "unless", "youve", "cast", "a", "creature", "spell", "this", "turn",
        ],
    ) || slice_ends_with(
        &normalized,
        &[
            "unless", "youve", "cast", "creature", "spell", "this", "turn",
        ],
    );
    let cant_attack_unless_cast_noncreature_spell_tail = slice_ends_with(
        &normalized,
        &[
            "unless",
            "youve",
            "cast",
            "a",
            "noncreature",
            "spell",
            "this",
            "turn",
        ],
    ) || slice_ends_with(
        &normalized,
        &[
            "unless",
            "youve",
            "cast",
            "noncreature",
            "spell",
            "this",
            "turn",
        ],
    );
    if cant_attack_unless_cast_creature_spell_tail
        && (slice_starts_with(&normalized, &["this", "creature", "cant", "attack"])
            || slice_starts_with(&normalized, &["this", "cant", "attack"]))
    {
        return Ok(Some(
            StaticAbility::cant_attack_unless_controller_cast_creature_spell_this_turn(),
        ));
    }
    if cant_attack_unless_cast_noncreature_spell_tail
        && (slice_starts_with(&normalized, &["this", "creature", "cant", "attack"])
            || slice_starts_with(&normalized, &["this", "cant", "attack"]))
    {
        return Ok(Some(
            StaticAbility::cant_attack_unless_controller_cast_noncreature_spell_this_turn(),
        ));
    }

    let starts_with_this_cant_attack_unless =
        slice_starts_with(
            &normalized,
            &["this", "creature", "cant", "attack", "unless"],
        ) || slice_starts_with(&normalized, &["this", "cant", "attack", "unless"]);
    if starts_with_this_cant_attack_unless {
        let tail = if slice_starts_with(
            &normalized,
            &["this", "creature", "cant", "attack", "unless"],
        ) {
            &normalized[5..]
        } else {
            &normalized[4..]
        };

        let static_text = format!("Can't attack unless {}", tail.join(" "));
        let static_with = |condition| {
            Ok(Some(StaticAbility::cant_attack_unless_condition(
                condition,
                static_text.clone(),
            )))
        };

        if tail
            == [
                "you",
                "control",
                "more",
                "creatures",
                "than",
                "defending",
                "player",
            ]
        {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::ControllerControlsMoreThanDefendingPlayer(
                    ObjectFilter::default().with_type(crate::types::CardType::Creature),
                ),
            );
        }
        if tail
            == [
                "you",
                "control",
                "more",
                "lands",
                "than",
                "defending",
                "player",
            ]
        {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::ControllerControlsMoreThanDefendingPlayer(
                    ObjectFilter::default().with_type(crate::types::CardType::Land),
                ),
            );
        }
        if let [
            "you",
            "control",
            "another",
            "creature",
            "with",
            "power",
            amount,
            "or",
            "greater",
        ] = tail
            && let Some(value) = parse_cardinal_u32(amount)
        {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::SourceCondition(
                    crate::ConditionExpr::YouControl(
                        ObjectFilter::creature().you_control().other().with_power(
                            crate::filter::Comparison::GreaterThanOrEqual(value as i32),
                        ),
                    ),
                ),
            );
        }
        if tail == ["you", "control", "another", "artifact"] {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::SourceCondition(
                    crate::ConditionExpr::YouControl(
                        ObjectFilter::artifact().you_control().other(),
                    ),
                ),
            );
        }
        if tail == ["you", "control", "an", "artifact"] || tail == ["you", "control", "artifact"] {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::SourceCondition(
                    crate::ConditionExpr::YouControl(ObjectFilter::artifact().you_control()),
                ),
            );
        }
        if tail == ["you", "control", "a", "knight", "or", "a", "soldier"]
            || tail == ["you", "control", "knight", "or", "soldier"]
        {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::SourceCondition(
                    crate::ConditionExpr::Or(
                        Box::new(crate::ConditionExpr::YouControl(
                            ObjectFilter::creature()
                                .you_control()
                                .with_subtype(Subtype::Knight),
                        )),
                        Box::new(crate::ConditionExpr::YouControl(
                            ObjectFilter::creature()
                                .you_control()
                                .with_subtype(Subtype::Soldier),
                        )),
                    ),
                ),
            );
        }
        if let [
            "you",
            "control",
            "a",
            "creature",
            "with",
            "power",
            amount,
            "or",
            "greater",
        ] = tail
            && let Some(value) = parse_cardinal_u32(amount)
        {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::SourceCondition(
                    crate::ConditionExpr::YouControl(
                        ObjectFilter::creature().you_control().with_power(
                            crate::filter::Comparison::GreaterThanOrEqual(value as i32),
                        ),
                    ),
                ),
            );
        }
        if tail == ["you", "control", "a", "1/1", "creature"]
            || tail == ["you", "control", "1/1", "creature"]
        {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::SourceCondition(
                    crate::ConditionExpr::YouControl(
                        ObjectFilter::creature()
                            .you_control()
                            .with_power(crate::filter::Comparison::Equal(1))
                            .with_toughness(crate::filter::Comparison::Equal(1)),
                    ),
                ),
            );
        }
        if tail == ["there", "is", "a", "mountain", "on", "the", "battlefield"]
            || tail == ["there", "is", "a", "mountain", "on", "battlefield"]
            || tail == ["there", "is", "mountain", "on", "battlefield"]
        {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::BattlefieldCountAtLeast {
                    filter: ObjectFilter::default()
                        .with_type(crate::types::CardType::Land)
                        .with_subtype(Subtype::Mountain),
                    count: 1,
                },
            );
        }
        if let [
            "there",
            "are",
            amount,
            "or",
            "more",
            "cards",
            "in",
            "your",
            "graveyard",
        ] = tail
            && let Some(value) = parse_cardinal_u32(amount)
        {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::ControllerGraveyardHasCardsAtLeast(
                    value,
                ),
            );
        }
        if let [
            "there",
            "are",
            amount,
            "or",
            "more",
            "islands",
            "on",
            "the",
            "battlefield",
        ] = tail
            && let Some(value) = parse_cardinal_u32(amount)
        {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::BattlefieldCountAtLeast {
                    filter: ObjectFilter::default()
                        .with_type(crate::types::CardType::Land)
                        .with_subtype(Subtype::Island),
                    count: value,
                },
            );
        }
        if tail == ["defending", "player", "is", "poisoned"] {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::DefendingPlayerCondition(
                    crate::static_abilities::DefendingPlayerAttackCondition::IsPoisoned,
                ),
            );
        }
        if let [
            "defending",
            "player",
            "has",
            amount,
            "or",
            "more",
            "cards",
            "in",
            "their",
            "graveyard",
        ] = tail
            && let Some(value) = parse_cardinal_u32(amount)
        {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::DefendingPlayerCondition(
                    crate::static_abilities::DefendingPlayerAttackCondition::HasCardsInGraveyardOrMore(
                        value,
                    ),
                ),
            );
        }
        if tail
            == [
                "defending",
                "player",
                "controls",
                "an",
                "enchantment",
                "or",
                "an",
                "enchanted",
                "permanent",
            ]
            || tail
                == [
                    "defending",
                    "player",
                    "controls",
                    "enchantment",
                    "or",
                    "enchanted",
                    "permanent",
                ]
        {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::DefendingPlayerCondition(
                    crate::static_abilities::DefendingPlayerAttackCondition::ControlsEnchantmentOrEnchantedPermanent,
                ),
            );
        }
        if tail == ["defending", "player", "controls", "a", "snow", "land"]
            || tail == ["defending", "player", "controls", "snow", "land"]
        {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::DefendingPlayerCondition(
                    crate::static_abilities::DefendingPlayerAttackCondition::Controls(
                        ObjectFilter::default()
                            .with_type(crate::types::CardType::Land)
                            .with_supertype(crate::types::Supertype::Snow),
                    ),
                ),
            );
        }
        if tail
            == [
                "defending",
                "player",
                "controls",
                "a",
                "creature",
                "with",
                "flying",
            ]
            || tail
                == [
                    "defending",
                    "player",
                    "controls",
                    "creature",
                    "with",
                    "flying",
                ]
        {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::DefendingPlayerCondition(
                    crate::static_abilities::DefendingPlayerAttackCondition::Controls(
                        ObjectFilter::default()
                            .with_type(crate::types::CardType::Creature)
                            .with_static_ability(crate::static_abilities::StaticAbilityId::Flying),
                    ),
                ),
            );
        }
        if tail == ["defending", "player", "controls", "a", "blue", "permanent"]
            || tail == ["defending", "player", "controls", "blue", "permanent"]
        {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::DefendingPlayerCondition(
                    crate::static_abilities::DefendingPlayerAttackCondition::Controls(
                        ObjectFilter::default().with_colors(crate::color::ColorSet::from_color(
                            crate::color::Color::Blue,
                        )),
                    ),
                ),
            );
        }
        if tail == ["at", "least", "two", "other", "creatures", "attack"] {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::AttackingGroupCondition(
                    crate::static_abilities::AttackingGroupAttackCondition::AtLeastNOtherCreaturesAttack(
                        2,
                    ),
                ),
            );
        }
        if tail
            == [
                "a", "creature", "with", "greater", "power", "also", "attacks",
            ]
        {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::AttackingGroupCondition(
                    crate::static_abilities::AttackingGroupAttackCondition::CreatureWithGreaterPowerAlsoAttacks,
                ),
            );
        }
        if tail == ["a", "black", "or", "green", "creature", "also", "attacks"] {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::AttackingGroupCondition(
                    crate::static_abilities::AttackingGroupAttackCondition::BlackOrGreenCreatureAlsoAttacks,
                ),
            );
        }
        if tail
            == [
                "an", "opponent", "has", "been", "dealt", "damage", "this", "turn",
            ]
        {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::OpponentWasDealtDamageThisTurn,
            );
        }
        if let ["you", "control", amount, "or", "more", "artifacts"] = tail
            && let Some(value) = parse_cardinal_u32(amount)
        {
            let mut filter = ObjectFilter::artifact();
            filter.zone = Some(Zone::Battlefield);
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::SourceCondition(
                    crate::ConditionExpr::PlayerControlsAtLeast {
                        player: PlayerFilter::You,
                        filter,
                        count: value,
                    },
                ),
            );
        }
        if tail == ["you", "sacrifice", "a", "land"] || tail == ["you", "sacrifice", "land"] {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::AttackCost(
                    crate::static_abilities::AttackCostCondition::SacrificePermanents {
                        filter: ObjectFilter::land(),
                        count: 1,
                    },
                ),
            );
        }
        if let ["you", "sacrifice", amount, "islands"] = tail
            && let Some(value) = parse_cardinal_u32(amount)
        {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::AttackCost(
                    crate::static_abilities::AttackCostCondition::SacrificePermanents {
                        filter: ObjectFilter::land().with_subtype(Subtype::Island),
                        count: value,
                    },
                ),
            );
        }
        if tail
            == [
                "you",
                "return",
                "an",
                "enchantment",
                "you",
                "control",
                "to",
                "its",
                "owners",
                "hand",
            ]
            || tail
                == [
                    "you",
                    "return",
                    "enchantment",
                    "you",
                    "control",
                    "to",
                    "its",
                    "owners",
                    "hand",
                ]
            || tail
                == [
                    "you",
                    "return",
                    "an",
                    "enchantment",
                    "you",
                    "control",
                    "to",
                    "its",
                    "owner",
                    "s",
                    "hand",
                ]
        {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::AttackCost(
                    crate::static_abilities::AttackCostCondition::ReturnPermanentsToOwnersHand {
                        filter: ObjectFilter::enchantment(),
                        count: 1,
                    },
                ),
            );
        }
        if tail
            == [
                "you", "pay", "1", "for", "each", "+1/+1", "counter", "on", "it",
            ]
            || tail
                == [
                    "you", "pay", "1", "for", "each", "1/1", "counter", "on", "it",
                ]
        {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::AttackCost(
                    crate::static_abilities::AttackCostCondition::PayGenericPerSourceCounter {
                        counter_type: crate::object::CounterType::PlusOnePlusOne,
                        amount_per_counter: 1,
                    },
                ),
            );
        }
        if tail == ["defending", "player", "is", "the", "monarch"]
            || tail == ["defending", "player", "is", "monarch"]
        {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::DefendingPlayerCondition(
                    crate::static_abilities::DefendingPlayerAttackCondition::IsMonarch,
                ),
            );
        }
    }

    if starts_with_cant_attack_unless_defending_player {
        let mut idx = if slice_starts_with(
            &normalized,
            &[
                "this",
                "creature",
                "cant",
                "attack",
                "unless",
                "defending",
                "player",
            ],
        ) {
            7
        } else {
            6
        };

        if !normalized
            .get(idx)
            .is_some_and(|word| *word == "control" || *word == "controls")
        {
            return Err(CardTextError::ParseError(format!(
                "unsupported cant-attack unless clause tail (clause: '{}')",
                normalized.join(" ")
            )));
        }
        idx += 1;

        if normalized
            .get(idx)
            .is_some_and(|word| *word == "a" || *word == "an" || *word == "the")
        {
            idx += 1;
        }

        let subtype_word = normalized.get(idx).copied().ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing land subtype in cant-attack unless clause (clause: '{}')",
                normalized.join(" ")
            ))
        })?;
        let subtype = parse_subtype_word(subtype_word)
            .or_else(|| str_strip_suffix(subtype_word, "s").and_then(parse_subtype_word))
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported land subtype in cant-attack unless clause (clause: '{}')",
                    normalized.join(" ")
                ))
            })?;

        if idx + 1 != normalized.len() {
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing cant-attack unless clause (clause: '{}')",
                normalized.join(" ")
            )));
        }

        return Ok(Some(StaticAbility::cant_attack_unless_condition(
            crate::static_abilities::CantAttackUnlessConditionSpec::DefendingPlayerCondition(
                crate::static_abilities::DefendingPlayerAttackCondition::Controls(
                    ObjectFilter::land().with_subtype(subtype),
                ),
            ),
            "",
        )));
    }

    if let Some((neg_start, neg_end)) = find_negation_span(tokens) {
        let subject_tokens = trim_commas(&tokens[..neg_start]);
        let remainder_tokens = trim_commas(&tokens[neg_end..]);
        let remainder_words_storage = normalize_cant_words(&remainder_tokens);
        let remainder_words = remainder_words_storage
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>();
        let subject_words = crate::cards::builders::compiler::token_word_refs(&subject_tokens);
        if (subject_words == ["this", "creature"] || subject_words == ["this"])
            && remainder_words.first() == Some(&"block")
            && remainder_words.len() > 1
        {
            let attacker_tokens = trim_commas(&remainder_tokens[1..]);
            let attacker_filter = parse_subject_object_filter(&attacker_tokens)?
                .or_else(|| parse_object_filter(&attacker_tokens, false).ok())
                .ok_or_else(|| {
                    CardTextError::ParseError(format!(
                        "unsupported blocker restriction filter (clause: '{}')",
                        normalized.join(" ")
                    ))
                })?;
            return Ok(Some(StaticAbility::restriction(
                crate::effect::Restriction::block_specific_attacker(
                    ObjectFilter::source(),
                    attacker_filter,
                ),
                format!(
                    "this creature can't block {}",
                    crate::cards::builders::compiler::token_word_refs(&attacker_tokens).join(" ")
                ),
            )));
        }
        if remainder_words.as_slice() == ["transform"] {
            let Some(filter) = parse_subject_object_filter(&subject_tokens)? else {
                return Ok(None);
            };
            let subject_text =
                crate::cards::builders::compiler::token_word_refs(&subject_tokens).join(" ");
            if subject_text.is_empty() {
                return Ok(None);
            }
            return Ok(Some(StaticAbility::restriction(
                crate::effect::Restriction::transform(filter),
                format!("{subject_text} can't transform"),
            )));
        }
    }

    if slice_starts_with(
        &normalized,
        &["your", "opponents", "cant", "cast", "spells", "with"],
    ) && normalized.len() >= 8
        && normalized[6] == "mana"
        && normalized[7] == "values"
    {
        let parity = match normalized[5] {
            "odd" => crate::filter::ParityRequirement::Odd,
            "even" => crate::filter::ParityRequirement::Even,
            _ => return Ok(None),
        };
        return Ok(Some(StaticAbility::restriction(
            crate::effect::Restriction::cast_spells_matching(
                PlayerFilter::Opponent,
                ObjectFilter::spell().with_mana_value_parity(parity),
            ),
            format_negated_restriction_display(tokens),
        )));
    }

    if slice_starts_with(
        &normalized,
        &[
            "your",
            "opponents",
            "cant",
            "block",
            "with",
            "creatures",
            "with",
        ],
    ) && normalized.len() >= 10
        && normalized[8] == "mana"
        && normalized[9] == "values"
    {
        let parity = match normalized[7] {
            "odd" => crate::filter::ParityRequirement::Odd,
            "even" => crate::filter::ParityRequirement::Even,
            _ => return Ok(None),
        };
        return Ok(Some(StaticAbility::restriction(
            crate::effect::Restriction::block(
                ObjectFilter::creature()
                    .opponent_controls()
                    .with_mana_value_parity(parity),
            ),
            format_negated_restriction_display(tokens),
        )));
    }

    if slice_starts_with(
        &normalized,
        &["this", "cant", "attack", "or", "block", "unless"],
    ) && slice_ends_with(
        &normalized,
        &["even", "number", "of", "counters", "on", "it"],
    ) {
        return Ok(Some(StaticAbility::keyword_marker(
            format_negated_restriction_display(tokens),
        )));
    }

    if (slice_starts_with(
        &normalized,
        &[
            "this", "creature", "cant", "attack", "or", "block", "unless",
        ],
    ) || slice_starts_with(
        &normalized,
        &["this", "cant", "attack", "or", "block", "unless"],
    )) && let tail = if slice_starts_with(
        &normalized,
        &[
            "this", "creature", "cant", "attack", "or", "block", "unless",
        ],
    ) {
        &normalized[7..]
    } else {
        &normalized[6..]
    } && let ["you", "control", amount, "or", "more", rest @ ..] = tail
        && !rest.is_empty()
        && let Some(count) = parse_cardinal_u32(amount)
    {
        let filter_tokens = rest
            .iter()
            .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
            .collect::<Vec<_>>();
        if let Ok(mut filter) = parse_object_filter(&filter_tokens, false) {
            if filter.zone.is_none() {
                filter.zone = Some(Zone::Battlefield);
            }
            let condition =
                crate::ConditionExpr::Not(Box::new(crate::ConditionExpr::PlayerControlsAtLeast {
                    player: PlayerFilter::You,
                    filter,
                    count,
                }));
            return Ok(Some(
                StaticAbility::restriction(
                    crate::effect::Restriction::attack_or_block(ObjectFilter::source()),
                    format_negated_restriction_display(tokens),
                )
                .with_condition(condition)
                .unwrap_or_else(|| {
                    StaticAbility::restriction(
                        crate::effect::Restriction::attack_or_block(ObjectFilter::source()),
                        format_negated_restriction_display(tokens),
                    )
                }),
            ));
        }
    }

    if slice_starts_with(&normalized, &["if", "source", "you", "control", "with"])
        && slice_contains(&normalized, &"mana")
        && slice_contains(&normalized, &"value")
        && slice_contains(&normalized, &"double")
        && normalized.last().is_some_and(|word| *word == "instead")
    {
        return Ok(Some(StaticAbility::keyword_marker(
            crate::cards::builders::compiler::token_word_refs(tokens).join(" "),
        )));
    }

    if let Some(parsed) = parse_cant_restriction_clause(tokens)?
        && parsed.target.is_none()
        && matches!(
            parsed.restriction,
            crate::effect::Restriction::GainLife(_)
                | crate::effect::Restriction::SearchLibraries(_)
                | crate::effect::Restriction::CastSpellsMatching(_, _)
                | crate::effect::Restriction::ActivateNonManaAbilities(_)
                | crate::effect::Restriction::ActivateAbilitiesOf(_)
                | crate::effect::Restriction::ActivateTapAbilitiesOf(_)
                | crate::effect::Restriction::ActivateNonManaAbilitiesOf(_)
                | crate::effect::Restriction::CastMoreThanOneSpellEachTurn(_, _)
                | crate::effect::Restriction::DrawCards(_)
                | crate::effect::Restriction::DrawExtraCards(_)
                | crate::effect::Restriction::ChangeLifeTotal(_)
                | crate::effect::Restriction::LoseGame(_)
                | crate::effect::Restriction::WinGame(_)
                | crate::effect::Restriction::PreventDamage
        )
    {
        let ability = match normalized.as_slice() {
            ["players", "cant", "gain", "life"] => StaticAbility::players_cant_gain_life(),
            ["players", "cant", "search", "libraries"] => StaticAbility::players_cant_search(),
            ["damage", "cant", "be", "prevented"] => StaticAbility::damage_cant_be_prevented(),
            ["you", "cant", "lose", "the", "game"] => StaticAbility::you_cant_lose_game(),
            ["your", "opponents", "cant", "win", "the", "game"] => {
                StaticAbility::opponents_cant_win_game()
            }
            ["your", "life", "total", "cant", "change"] => {
                StaticAbility::your_life_total_cant_change()
            }
            ["your", "opponents", "cant", "cast", "spells"] => {
                StaticAbility::opponents_cant_cast_spells()
            }
            [
                "your",
                "opponents",
                "cant",
                "draw",
                "more",
                "than",
                "one",
                "card",
                "each",
                "turn",
            ] => StaticAbility::opponents_cant_draw_extra_cards(),
            _ => StaticAbility::restriction(
                parsed.restriction,
                format_negated_restriction_display(tokens),
            ),
        };
        return Ok(Some(ability));
    }

    let ability = match normalized.as_slice() {
        ["counters", "cant", "be", "put", "on", "this", "permanent"] => {
            StaticAbility::cant_have_counters_placed()
        }
        ["this", "spell", "cant", "be", "countered"] => StaticAbility::cant_be_countered_ability(),
        ["this", "creature", "cant", "attack"] => StaticAbility::cant_attack(),
        ["this", "creature", "cant", "attack", "its", "owner"] => {
            StaticAbility::cant_attack_its_owner()
        }
        ["this", "creature", "cant", "block"] => StaticAbility::cant_block(),
        ["this", "creature", "cant", "attack", "alone"] => StaticAbility::restriction(
            crate::effect::Restriction::attack_alone(ObjectFilter::source()),
            "this creature can't attack alone".to_string(),
        ),
        ["this", "token", "cant", "attack", "alone"] => StaticAbility::restriction(
            crate::effect::Restriction::attack_alone(ObjectFilter::source()),
            "this token can't attack alone".to_string(),
        ),
        ["this", "cant", "attack", "alone"] => StaticAbility::restriction(
            crate::effect::Restriction::attack_alone(ObjectFilter::source()),
            "this can't attack alone".to_string(),
        ),
        ["this", "token", "cant", "attack"] => StaticAbility::cant_attack(),
        ["this", "token", "cant", "block"] => StaticAbility::cant_block(),
        ["this", "cant", "block"] => StaticAbility::cant_block(),
        ["this", "cant", "attack"] => StaticAbility::cant_attack(),
        ["this", "creature", "cant", "attack", "or", "block"] => StaticAbility::restriction(
            crate::effect::Restriction::attack_or_block(ObjectFilter::source()),
            "this creature can't attack or block".to_string(),
        ),
        ["this", "token", "cant", "attack", "or", "block"] => StaticAbility::restriction(
            crate::effect::Restriction::attack_or_block(ObjectFilter::source()),
            "this token can't attack or block".to_string(),
        ),
        ["this", "cant", "attack", "or", "block"] => StaticAbility::restriction(
            crate::effect::Restriction::attack_or_block(ObjectFilter::source()),
            "this can't attack or block".to_string(),
        ),
        ["this", "creature", "cant", "attack", "or", "block", "alone"] => {
            StaticAbility::restriction(
                crate::effect::Restriction::attack_or_block_alone(ObjectFilter::source()),
                "this creature can't attack or block alone".to_string(),
            )
        }
        ["this", "token", "cant", "attack", "or", "block", "alone"] => StaticAbility::restriction(
            crate::effect::Restriction::attack_or_block_alone(ObjectFilter::source()),
            "this token can't attack or block alone".to_string(),
        ),
        ["this", "cant", "attack", "or", "block", "alone"] => StaticAbility::restriction(
            crate::effect::Restriction::attack_or_block_alone(ObjectFilter::source()),
            "this can't attack or block alone".to_string(),
        ),
        ["permanents", "you", "control", "cant", "be", "sacrificed"] => {
            StaticAbility::permanents_you_control_cant_be_sacrificed()
        }
        ["this", "creature", "cant", "be", "blocked"] => StaticAbility::unblockable(),
        ["this", "creature", "cant", "be", "blocked", "this", "turn"] => {
            StaticAbility::unblockable()
        }
        ["this", "cant", "be", "blocked"] => StaticAbility::unblockable(),
        ["this", "cant", "be", "blocked", "this", "turn"] => StaticAbility::unblockable(),
        ["cant", "be", "blocked"] => StaticAbility::unblockable(),
        ["cant", "be", "blocked", "this", "turn"] => StaticAbility::unblockable(),
        _ => {
            if let Some(parsed) = parse_negated_object_restriction_clause(tokens)?
                && parsed.target.is_none()
            {
                return Ok(Some(StaticAbility::restriction(
                    parsed.restriction,
                    format_negated_restriction_display(tokens),
                )));
            }
            return Ok(None);
        }
    };

    Ok(Some(ability))
}

