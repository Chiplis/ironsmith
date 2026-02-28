use super::*;

pub(crate) fn parse_effect_with_verb(
    verb: Verb,
    subject: Option<SubjectAst>,
    tokens: &[Token],
) -> Result<EffectAst, CardTextError> {
    match verb {
        Verb::Add => parse_add_mana(tokens, subject),
        Verb::Move => parse_move(tokens),
        Verb::Deal => parse_deal_damage(tokens),
        Verb::Draw => parse_draw(tokens, subject),
        Verb::Counter => parse_counter(tokens),
        Verb::Destroy => parse_destroy(tokens),
        Verb::Exile => parse_exile(tokens, subject),
        Verb::Reveal => parse_reveal(tokens, subject),
        Verb::Look => parse_look(tokens, subject),
        Verb::Lose => parse_lose_life(tokens, subject),
        Verb::Gain => {
            if tokens.first().is_some_and(|token| token.is_word("control")) {
                parse_gain_control(tokens, subject)
            } else {
                parse_gain_life(tokens, subject)
            }
        }
        Verb::Put => {
            let has_onto = tokens.iter().any(|token| token.is_word("onto"));
            let has_counter_words = tokens
                .iter()
                .any(|token| token.is_word("counter") || token.is_word("counters"));

            // Prefer zone moves like "... onto the battlefield" over counter placement because
            // "counter(s)" may appear in subordinate clauses (e.g. "mana value equal to the number
            // of charge counters on this artifact").
            if has_onto {
                if let Ok(effect) = parse_put_into_hand(tokens, subject) {
                    Ok(effect)
                } else if has_counter_words {
                    parse_put_counters(tokens)
                } else {
                    parse_put_into_hand(tokens, subject)
                }
            } else if has_counter_words {
                parse_put_counters(tokens)
            } else {
                parse_put_into_hand(tokens, subject)
            }
        }
        Verb::Sacrifice => parse_sacrifice(tokens, subject),
        Verb::Create => parse_create(tokens, subject),
        Verb::Investigate => parse_investigate(tokens),
        Verb::Proliferate => Ok(EffectAst::Proliferate),
        Verb::Tap => parse_tap(tokens),
        Verb::Attach => parse_attach(tokens),
        Verb::Untap => parse_untap(tokens),
        Verb::Scry => parse_scry(tokens, subject),
        Verb::Discard => parse_discard(tokens, subject),
        Verb::Transform => parse_transform(tokens),
        Verb::Flip => parse_flip(tokens),
        Verb::Regenerate => parse_regenerate(tokens),
        Verb::Mill => parse_mill(tokens, subject),
        Verb::Get => parse_get(tokens, subject),
        Verb::Remove => parse_remove(tokens),
        Verb::Return => parse_return(tokens),
        Verb::Exchange => parse_exchange(tokens),
        Verb::Become => parse_become(tokens, subject),
        Verb::Switch => parse_switch(tokens),
        Verb::Skip => parse_skip(tokens, subject),
        Verb::Surveil => parse_surveil(tokens, subject),
        Verb::Shuffle => parse_shuffle(tokens, subject),
        Verb::Reorder => parse_reorder(tokens, subject),
        Verb::Pay => parse_pay(tokens, subject),
        Verb::Goad => parse_goad(tokens),
    }
}

pub(crate) fn parse_look(
    tokens: &[Token],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    fn parse_library_owner(words: &[&str]) -> Option<(PlayerAst, usize)> {
        if words.starts_with(&["your", "library"]) {
            return Some((PlayerAst::You, 2));
        }
        if words.starts_with(&["each", "player", "library"])
            || words.starts_with(&["each", "players", "library"])
        {
            return Some((PlayerAst::Any, 3));
        }
        if words.starts_with(&["their", "library"]) {
            return Some((PlayerAst::That, 2));
        }
        if words.starts_with(&["that", "player", "library"])
            || words.starts_with(&["that", "players", "library"])
        {
            return Some((PlayerAst::That, 3));
        }
        if words.starts_with(&["target", "player", "library"])
            || words.starts_with(&["target", "players", "library"])
        {
            return Some((PlayerAst::Target, 3));
        }
        if words.starts_with(&["target", "opponent", "library"])
            || words.starts_with(&["target", "opponents", "library"])
        {
            return Some((PlayerAst::TargetOpponent, 3));
        }
        if words.starts_with(&["its", "owner", "library"])
            || words.starts_with(&["its", "owners", "library"])
        {
            return Some((PlayerAst::ItsOwner, 3));
        }
        if words.starts_with(&["his", "or", "her", "library"]) {
            return Some((PlayerAst::That, 4));
        }
        None
    }

    // "Look at the top N cards of your library."
    let mut clause_tokens = trim_commas(tokens);
    if clause_tokens
        .first()
        .is_some_and(|token| token.is_word("at"))
    {
        clause_tokens = trim_commas(&clause_tokens[1..]);
    }
    let clause_words = words(&clause_tokens);

    let Some(top_idx) = clause_tokens.iter().position(|t| t.is_word("top")) else {
        return Err(CardTextError::ParseError(format!(
            "unsupported look clause (clause: '{}')",
            clause_words.join(" ")
        )));
    };
    if top_idx + 1 >= clause_tokens.len() {
        return Err(CardTextError::ParseError(format!(
            "missing look top noun (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let mut idx = top_idx + 1;
    let count = if clause_tokens
        .get(idx)
        .and_then(Token::as_word)
        .is_some_and(|w| w == "card" || w == "cards")
    {
        Value::Fixed(1)
    } else {
        let (value, used) = parse_value(&clause_tokens[idx..]).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing look count (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
        idx += used;
        value
    };

    // Consume "card(s)"
    if clause_tokens
        .get(idx)
        .and_then(Token::as_word)
        .is_some_and(|w| w == "card" || w == "cards")
    {
        idx += 1;
    } else {
        return Err(CardTextError::ParseError(format!(
            "missing look card noun (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    // Consume "of <player> library"
    if !clause_tokens.get(idx).is_some_and(|t| t.is_word("of")) {
        return Err(CardTextError::ParseError(format!(
            "missing 'of' in look clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    idx += 1;
    let mut owner_tokens = &clause_tokens[idx..];
    while owner_tokens
        .first()
        .is_some_and(|t| t.is_word("the") || t.is_word("a") || t.is_word("an"))
    {
        owner_tokens = &owner_tokens[1..];
    }
    let owner_words = words(owner_tokens);
    let (player, used_words) = parse_library_owner(&owner_words)
        .or_else(|| {
            // If the clause uses a subject ("target player looks ..."), treat that as the default.
            subject.and_then(|s| match s {
                SubjectAst::Player(p) => Some((p, 0)),
                _ => None,
            })
        })
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported look library owner (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
    // No trailing words supported for now (based on word tokens).
    if used_words < owner_words.len() {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing look clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    if matches!(player, PlayerAst::Any) {
        return Ok(EffectAst::ForEachPlayer {
            effects: vec![EffectAst::LookAtTopCards {
                player: PlayerAst::That,
                count,
                tag: TagKey::from(IT_TAG),
            }],
        });
    }

    Ok(EffectAst::LookAtTopCards {
        player,
        count,
        tag: TagKey::from(IT_TAG),
    })
}

pub(crate) fn parse_reorder(
    tokens: &[Token],
    _subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let clause = words(tokens).join(" ");
    let clause_words = words(tokens);
    if clause_words.is_empty() {
        return Err(CardTextError::ParseError(
            "missing reorder target".to_string(),
        ));
    }

    let (player, rest) = if clause_words.starts_with(&["your", "graveyard"]) {
        (PlayerAst::You, &clause_words[2..])
    } else if clause_words.starts_with(&["their", "graveyard"]) {
        (PlayerAst::That, &clause_words[2..])
    } else if clause_words.starts_with(&["that", "player", "graveyard"])
        || clause_words.starts_with(&["that", "players", "graveyard"])
    {
        (PlayerAst::That, &clause_words[3..])
    } else if clause_words.starts_with(&["its", "controller", "graveyard"])
        || clause_words.starts_with(&["its", "controllers", "graveyard"])
    {
        (PlayerAst::ItsController, &clause_words[3..])
    } else if clause_words.starts_with(&["its", "owner", "graveyard"])
        || clause_words.starts_with(&["its", "owners", "graveyard"])
    {
        (PlayerAst::ItsOwner, &clause_words[3..])
    } else {
        return Err(CardTextError::ParseError(format!(
            "unsupported reorder clause (clause: '{clause}')"
        )));
    };

    if !rest.is_empty() && rest != ["as", "you", "choose"] {
        return Err(CardTextError::ParseError(format!(
            "unsupported reorder clause tail (clause: '{clause}')"
        )));
    }

    Ok(EffectAst::ReorderGraveyard { player })
}

pub(crate) fn parse_shuffle(
    tokens: &[Token],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    fn is_simple_library_phrase(words: &[&str]) -> bool {
        matches!(
            words,
            ["library"]
                | ["your", "library"]
                | ["their", "library"]
                | ["that", "player", "library"]
                | ["that", "players", "library"]
                | ["its", "owner", "library"]
                | ["its", "owners", "library"]
                | ["his", "or", "her", "library"]
        )
    }

    let player = match subject {
        Some(SubjectAst::Player(player)) => player,
        _ => PlayerAst::Implicit,
    };

    if tokens.is_empty() {
        // Support standalone "Shuffle." clauses. If the sentence includes an explicit player
        // subject, use it; otherwise return an implicit player that can be filled in by the
        // carry-context logic (and compiles to "you" by default).
        return Ok(EffectAst::ShuffleLibrary { player });
    }

    let clause_words = words(tokens);
    if clause_words.contains(&"graveyard")
        || clause_words.contains(&"cards")
        || clause_words.contains(&"card")
        || clause_words.contains(&"into")
        || clause_words.contains(&"from")
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported shuffle clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    if is_simple_library_phrase(&clause_words) {
        return Ok(EffectAst::ShuffleLibrary { player });
    }

    Err(CardTextError::ParseError(format!(
        "unsupported shuffle clause (clause: '{}')",
        clause_words.join(" ")
    )))
}

pub(crate) fn parse_goad(tokens: &[Token]) -> Result<EffectAst, CardTextError> {
    let target_tokens = trim_commas(tokens);
    if target_tokens.is_empty() {
        return Err(CardTextError::ParseError("missing goad target".to_string()));
    }

    let target_words = words(&target_tokens);
    if target_words.as_slice() == ["it"] || target_words.as_slice() == ["them"] {
        return Ok(EffectAst::Goad {
            target: TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(&target_tokens)),
        });
    }

    let target = parse_target_phrase(&target_tokens)?;
    if matches!(
        target,
        TargetAst::Player(_, _) | TargetAst::PlayerOrPlaneswalker(_, _)
    ) {
        return Err(CardTextError::ParseError(format!(
            "goad target must be a creature (clause: '{}')",
            words(tokens).join(" ")
        )));
    }

    Ok(EffectAst::Goad { target })
}

pub(crate) fn parse_attach_object_phrase(tokens: &[Token]) -> Result<TargetAst, CardTextError> {
    let object_words = words(tokens);
    let object_span = span_from_tokens(tokens);
    if object_words.is_empty() {
        return Err(CardTextError::ParseError(
            "missing object to attach".to_string(),
        ));
    }

    let is_source_attachment = is_source_reference_words(&object_words)
        || object_words.starts_with(&["this", "equipment"])
        || object_words.starts_with(&["this", "aura"])
        || object_words.starts_with(&["this", "enchantment"])
        || object_words.starts_with(&["this", "artifact"]);
    if is_source_attachment {
        return Ok(TargetAst::Source(object_span));
    }

    if matches!(object_words.as_slice(), ["it"] | ["them"]) {
        return Ok(TargetAst::Tagged(TagKey::from(IT_TAG), object_span));
    }

    let mut tagged_filter = ObjectFilter::default();
    if matches!(
        object_words.as_slice(),
        ["that", "equipment"] | ["those", "equipment"]
    ) {
        tagged_filter.zone = Some(Zone::Battlefield);
        tagged_filter.card_types.push(CardType::Artifact);
        tagged_filter.subtypes.push(Subtype::Equipment);
    } else if matches!(
        object_words.as_slice(),
        ["that", "aura"] | ["those", "auras"]
    ) {
        tagged_filter.zone = Some(Zone::Battlefield);
        tagged_filter.card_types.push(CardType::Enchantment);
        tagged_filter.subtypes.push(Subtype::Aura);
    } else if matches!(
        object_words.as_slice(),
        ["that", "artifact"] | ["those", "artifacts"]
    ) {
        tagged_filter.zone = Some(Zone::Battlefield);
        tagged_filter.card_types.push(CardType::Artifact);
    } else if object_words.as_slice() == ["that", "enchantment"] {
        tagged_filter.zone = Some(Zone::Battlefield);
        tagged_filter.card_types.push(CardType::Enchantment);
    }

    if tagged_filter.zone.is_some() {
        tagged_filter
            .tagged_constraints
            .push(TaggedObjectConstraint {
                tag: TagKey::from(IT_TAG),
                relation: TaggedOpbjectRelation::IsTaggedObject,
            });
        return Ok(TargetAst::Object(tagged_filter, object_span, None));
    }

    if tokens.first().is_some_and(|token| token.is_word("target"))
        && let Some(attached_idx) = tokens.iter().position(|token| token.is_word("attached"))
        && tokens
            .get(attached_idx + 1)
            .is_some_and(|token| token.is_word("to"))
    {
        let head_tokens = trim_commas(&tokens[..attached_idx]);
        if !head_tokens.is_empty() {
            return parse_target_phrase(&head_tokens);
        }
    }

    parse_target_phrase(tokens)
}

pub(crate) fn parse_attach(tokens: &[Token]) -> Result<EffectAst, CardTextError> {
    let clause_words = words(tokens);
    if tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "attach clause missing object and destination".to_string(),
        ));
    }

    if tokens.first().is_some_and(|token| token.is_word("to")) {
        let rest = trim_commas(&tokens[1..]);
        let Some(first) = rest.first() else {
            return Err(CardTextError::ParseError(format!(
                "attach clause missing object or destination (clause: '{}')",
                clause_words.join(" ")
            )));
        };
        if first.is_word("it") || first.is_word("them") {
            let target_tokens = vec![first.clone()];
            let object_tokens = trim_commas(&rest[1..]);
            if object_tokens.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "attach clause missing object or destination (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            let target = TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(&target_tokens));
            let object = parse_attach_object_phrase(&object_tokens)?;
            return Ok(EffectAst::Attach { object, target });
        }
    }

    let Some(to_idx) = tokens.iter().rposition(|token| token.is_word("to")) else {
        return Err(CardTextError::ParseError(format!(
            "attach clause missing destination (clause: '{}')",
            clause_words.join(" ")
        )));
    };
    if to_idx == 0 || to_idx + 1 >= tokens.len() {
        return Err(CardTextError::ParseError(format!(
            "attach clause missing object or destination (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let object_tokens = trim_commas(&tokens[..to_idx]);
    let target_tokens = trim_commas(&tokens[to_idx + 1..]);
    if object_tokens.is_empty() || target_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "attach clause missing object or destination (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let object = parse_attach_object_phrase(&object_tokens)?;
    let target_words = words(&target_tokens);
    let target = if matches!(target_words.as_slice(), ["it"] | ["them"]) {
        TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(&target_tokens))
    } else {
        parse_target_phrase(&target_tokens)?
    };

    Ok(EffectAst::Attach { object, target })
}

pub(crate) fn parse_deal_damage(tokens: &[Token]) -> Result<EffectAst, CardTextError> {
    let clause_words = words(tokens);
    let is_divided_as_you_choose_clause = clause_words.contains(&"divided")
        && clause_words.contains(&"choose")
        && clause_words.contains(&"among");
    if is_divided_as_you_choose_clause {
        return Err(CardTextError::ParseError(format!(
            "unsupported divided-damage distribution clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    if let Some(effect) = parse_deal_damage_equal_to_clause(tokens)? {
        return Ok(effect);
    }
    if let Some(effect) = parse_deal_damage_to_target_equal_to_clause(tokens)? {
        return Ok(effect);
    }
    if clause_words.starts_with(&["that", "much"]) {
        return parse_deal_damage_with_amount(tokens, Value::EventValue(EventValueSpec::Amount), 2);
    }

    if let Some((value, used)) = parse_value(tokens) {
        return parse_deal_damage_with_amount(tokens, value, used);
    }

    if clause_words.starts_with(&["damage", "to", "each", "opponent"])
        && clause_words.contains(&"number")
        && clause_words.contains(&"cards")
        && clause_words.contains(&"hand")
    {
        let value = Value::CardsInHand(PlayerFilter::IteratedPlayer);
        return Ok(EffectAst::ForEachOpponent {
            effects: vec![EffectAst::DealDamage {
                amount: value,
                target: TargetAst::Player(PlayerFilter::IteratedPlayer, None),
            }],
        });
    }

    Err(CardTextError::ParseError(format!(
        "missing damage amount (clause: '{}')",
        clause_words.join(" ")
    )))
}

pub(crate) fn parse_deal_damage_to_target_equal_to_clause(
    tokens: &[Token],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause_words = words(tokens);
    if !clause_words.starts_with(&["damage", "to"]) {
        return Ok(None);
    }

    let Some(equal_word_idx) = clause_words
        .windows(2)
        .position(|window| window == ["equal", "to"])
    else {
        return Ok(None);
    };
    let Some(equal_token_idx) = token_index_for_word_index(tokens, equal_word_idx) else {
        return Ok(None);
    };

    let mut target_tokens = trim_commas(&tokens[1..equal_token_idx]);
    if target_tokens
        .first()
        .is_some_and(|token| token.is_word("to"))
    {
        target_tokens.remove(0);
    }
    if target_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing damage target in equal-to clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let amount = parse_add_mana_equal_amount_value(tokens)
        .or(parse_equal_to_aggregate_filter_value(tokens))
        .or(parse_equal_to_number_of_filter_value(tokens))
        .or(parse_dynamic_cost_modifier_value(tokens)?)
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing damage amount (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
    let target_words = words(&target_tokens);
    if target_words.as_slice() == ["each", "player"]
        || target_words.as_slice() == ["each", "players"]
    {
        return Ok(Some(EffectAst::ForEachPlayer {
            effects: vec![EffectAst::DealDamage {
                amount: amount.clone(),
                target: TargetAst::Player(PlayerFilter::IteratedPlayer, None),
            }],
        }));
    }
    if target_words.as_slice() == ["each", "opponent"]
        || target_words.as_slice() == ["each", "opponents"]
        || target_words.as_slice() == ["each", "other", "player"]
        || target_words.as_slice() == ["each", "other", "players"]
    {
        return Ok(Some(EffectAst::ForEachOpponent {
            effects: vec![EffectAst::DealDamage {
                amount: amount.clone(),
                target: TargetAst::Player(PlayerFilter::IteratedPlayer, None),
            }],
        }));
    }
    let target = parse_target_phrase(&target_tokens)?;
    Ok(Some(EffectAst::DealDamage { amount, target }))
}

pub(crate) fn parse_deal_damage_equal_to_clause(
    tokens: &[Token],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause_words = words(tokens);
    if !clause_words.starts_with(&["damage", "equal", "to"]) {
        return Ok(None);
    }

    let mut target_to_idx = None;
    for idx in 3..tokens.len() {
        if !tokens[idx].is_word("to") {
            continue;
        }
        let tail_words = words(&tokens[idx + 1..]);
        if tail_words.is_empty() {
            continue;
        }
        let looks_like_target = tail_words.contains(&"target")
            || matches!(
                tail_words.first().copied(),
                Some(
                    "any"
                        | "each"
                        | "all"
                        | "it"
                        | "itself"
                        | "them"
                        | "him"
                        | "her"
                        | "that"
                        | "this"
                        | "you"
                        | "player"
                        | "opponent"
                        | "creature"
                        | "planeswalker"
                )
            );
        if looks_like_target {
            target_to_idx = Some(idx);
            break;
        }
    }

    let Some(target_to_idx) = target_to_idx else {
        return Err(CardTextError::ParseError(format!(
            "missing damage target in equal-to clause (clause: '{}')",
            clause_words.join(" ")
        )));
    };

    let amount_tokens = &tokens[..target_to_idx];
    let amount = parse_add_mana_equal_amount_value(amount_tokens)
        .or(parse_equal_to_aggregate_filter_value(amount_tokens))
        .or(parse_equal_to_number_of_filter_plus_or_minus_fixed_value(
            amount_tokens,
        ))
        .or(parse_equal_to_number_of_filter_value(amount_tokens))
        .or(parse_equal_to_number_of_opponents_you_have_value(
            amount_tokens,
        ))
        .or(parse_equal_to_number_of_counters_on_reference_value(
            amount_tokens,
        ))
        .or(parse_dynamic_cost_modifier_value(amount_tokens)?)
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing damage amount (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;

    let target_tokens = &tokens[target_to_idx + 1..];
    if target_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing damage target in equal-to clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    let mut normalized_target_tokens = target_tokens;
    let target_words = words(target_tokens);
    if target_words.starts_with(&["each", "of"]) {
        let each_of_tokens = &target_tokens[2..];
        let each_of_words = words(each_of_tokens);
        if each_of_words.iter().any(|word| *word == "target") {
            normalized_target_tokens = each_of_tokens;
        }
    }
    let normalized_target_words = words(normalized_target_tokens);
    if normalized_target_words.as_slice() == ["each", "player"]
        || normalized_target_words.as_slice() == ["each", "players"]
    {
        return Ok(Some(EffectAst::ForEachPlayer {
            effects: vec![EffectAst::DealDamage {
                amount: amount.clone(),
                target: TargetAst::Player(PlayerFilter::IteratedPlayer, None),
            }],
        }));
    }
    if normalized_target_words.as_slice() == ["each", "opponent"]
        || normalized_target_words.as_slice() == ["each", "opponents"]
        || normalized_target_words.as_slice() == ["each", "other", "player"]
        || normalized_target_words.as_slice() == ["each", "other", "players"]
    {
        return Ok(Some(EffectAst::ForEachOpponent {
            effects: vec![EffectAst::DealDamage {
                amount: amount.clone(),
                target: TargetAst::Player(PlayerFilter::IteratedPlayer, None),
            }],
        }));
    }
    let target = parse_target_phrase(normalized_target_tokens)?;
    Ok(Some(EffectAst::DealDamage { amount, target }))
}

pub(crate) fn parse_deal_damage_with_amount(
    tokens: &[Token],
    amount: Value,
    used: usize,
) -> Result<EffectAst, CardTextError> {
    let rest = &tokens[used..];
    let Some(Token::Word(word, _)) = rest.first() else {
        return Err(CardTextError::ParseError(
            "missing damage keyword".to_string(),
        ));
    };
    if word != "damage" {
        return Err(CardTextError::ParseError(
            "missing damage keyword".to_string(),
        ));
    }

    let mut target_tokens = &rest[1..];
    if target_tokens
        .first()
        .is_some_and(|token| token.is_word("to"))
    {
        target_tokens = &target_tokens[1..];
    }
    if let Some(among_idx) = target_tokens
        .iter()
        .position(|token| token.is_word("among"))
    {
        let among_tail = &target_tokens[among_idx + 1..];
        if among_tail.iter().any(|token| token.is_word("target"))
            && among_tail.iter().any(|token| {
                token.is_word("player")
                    || token.is_word("players")
                    || token.is_word("creature")
                    || token.is_word("creatures")
            })
        {
            target_tokens = among_tail;
        }
    }

    if target_tokens.iter().any(|token| token.is_word("where")) {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing where damage clause (clause: '{}')",
            words(tokens).join(" ")
        )));
    }

    if let Some(instead_idx) = target_tokens
        .iter()
        .position(|token| token.is_word("instead"))
        && target_tokens
            .get(instead_idx + 1)
            .is_some_and(|token| token.is_word("if"))
    {
        let pre_target_tokens = trim_commas(&target_tokens[..instead_idx]);
        let condition_tokens = trim_commas(&target_tokens[instead_idx + 2..]);
        let predicate =
            if let Some(predicate) = parse_instead_if_control_predicate(&condition_tokens)? {
                predicate
            } else {
                parse_predicate(&condition_tokens)?
            };
        let target = if pre_target_tokens.is_empty() {
            TargetAst::PlayerOrPlaneswalker(PlayerFilter::Any, None)
        } else {
            parse_target_phrase(&pre_target_tokens)?
        };
        return Ok(EffectAst::Conditional {
            predicate,
            if_true: vec![EffectAst::DealDamage {
                amount: amount.clone(),
                target,
            }],
            if_false: Vec::new(),
        });
    }

    if let Some(if_idx) = target_tokens.iter().position(|token| token.is_word("if")) {
        let pre_target_tokens = trim_commas(&target_tokens[..if_idx]);
        let condition_tokens = trim_commas(&target_tokens[if_idx + 1..]);
        if condition_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing if clause in damage effect (clause: '{}')",
                words(tokens).join(" ")
            )));
        }
        let predicate = parse_predicate(&condition_tokens)?;
        let target = if pre_target_tokens.is_empty() {
            // Follow-up "deals N damage if ..." clauses can omit the target and rely
            // on parser-level merge with a prior damage sentence.
            TargetAst::PlayerOrPlaneswalker(PlayerFilter::Any, None)
        } else {
            parse_target_phrase(&pre_target_tokens)?
        };
        return Ok(EffectAst::Conditional {
            predicate,
            if_true: vec![EffectAst::DealDamage { amount, target }],
            if_false: Vec::new(),
        });
    }

    let target_words = words(target_tokens);
    if target_words.starts_with(&["each", "of"]) {
        let each_of_tokens = &target_tokens[2..];
        let each_of_words = words(each_of_tokens);
        if each_of_words.iter().any(|word| *word == "target") {
            let target = parse_target_phrase(each_of_tokens)?;
            return Ok(EffectAst::DealDamage { amount, target });
        }
    }
    if target_words.as_slice() == ["each", "player"]
        || target_words.as_slice() == ["each", "players"]
    {
        return Ok(EffectAst::ForEachPlayer {
            effects: vec![EffectAst::DealDamage {
                amount: amount.clone(),
                target: TargetAst::Player(PlayerFilter::IteratedPlayer, None),
            }],
        });
    }
    if target_words.as_slice() == ["each", "opponent"]
        || target_words.as_slice() == ["each", "opponents"]
    {
        return Ok(EffectAst::ForEachOpponent {
            effects: vec![EffectAst::DealDamage {
                amount: amount.clone(),
                target: TargetAst::Player(PlayerFilter::IteratedPlayer, None),
            }],
        });
    }
    if target_words.starts_with(&["each", "opponent", "who"])
        && target_words
            .windows(2)
            .any(|window| window == ["this", "way"])
    {
        let predicate = parse_who_did_this_way_predicate(&target_tokens[2..])?;
        return Ok(EffectAst::ForEachOpponentDid {
            effects: vec![EffectAst::DealDamage {
                amount: amount.clone(),
                target: TargetAst::Player(PlayerFilter::IteratedPlayer, None),
            }],
            predicate,
        });
    }
    if target_words.starts_with(&["each", "player", "who"])
        && target_words
            .windows(2)
            .any(|window| window == ["this", "way"])
    {
        let predicate = parse_who_did_this_way_predicate(&target_tokens[2..])?;
        return Ok(EffectAst::ForEachPlayerDid {
            effects: vec![EffectAst::DealDamage {
                amount: amount.clone(),
                target: TargetAst::Player(PlayerFilter::IteratedPlayer, None),
            }],
            predicate,
        });
    }

    if matches!(target_words.first(), Some(&"each") | Some(&"all"))
        && let Some(and_each_idx) = target_words.windows(3).position(|window| {
            window == ["and", "each", "player"] || window == ["and", "each", "players"]
        })
        && and_each_idx >= 1
        && and_each_idx + 3 == target_words.len()
    {
        let filter_tokens = &target_tokens[1..and_each_idx];
        let mut filter = parse_object_filter(filter_tokens, false)?;
        if filter.controller.is_none() {
            filter.controller = Some(PlayerFilter::IteratedPlayer);
        }
        return Ok(EffectAst::ForEachPlayer {
            effects: vec![
                EffectAst::DealDamage {
                    amount: amount.clone(),
                    target: TargetAst::Player(PlayerFilter::IteratedPlayer, None),
                },
                EffectAst::DealDamageEach {
                    amount: amount.clone(),
                    filter,
                },
            ],
        });
    }

    if target_words.starts_with(&["each", "opponent", "and", "each"])
        && target_words.contains(&"creature")
        && target_words.contains(&"planeswalker")
        && (target_words
            .windows(2)
            .any(|pair| pair == ["they", "control"])
            || target_words
                .windows(3)
                .any(|triplet| triplet == ["that", "player", "controls"]))
    {
        let mut filter = ObjectFilter::default();
        filter.card_types = vec![CardType::Creature, CardType::Planeswalker];
        filter.controller = Some(PlayerFilter::IteratedPlayer);
        return Ok(EffectAst::ForEachOpponent {
            effects: vec![
                EffectAst::DealDamage {
                    amount: amount.clone(),
                    target: TargetAst::Player(PlayerFilter::IteratedPlayer, None),
                },
                EffectAst::DealDamageEach {
                    amount: amount.clone(),
                    filter,
                },
            ],
        });
    }

    if matches!(target_words.first(), Some(&"each") | Some(&"all")) {
        if target_tokens.len() < 2 {
            return Err(CardTextError::ParseError(
                "missing damage target filter after 'each'".to_string(),
            ));
        }
        let filter_tokens = &target_tokens[1..];
        let filter = parse_object_filter(filter_tokens, false)?;
        return Ok(EffectAst::DealDamageEach {
            amount: amount.clone(),
            filter,
        });
    }

    if let Some(at_idx) = target_tokens.iter().position(|token| token.is_word("at")) {
        let timing_words = words(&target_tokens[at_idx..]);
        let matches_end_of_combat = timing_words.as_slice() == ["at", "end", "of", "combat"]
            || timing_words.as_slice() == ["at", "the", "end", "of", "combat"];
        if matches_end_of_combat && at_idx >= 1 {
            let pre_target_tokens = trim_commas(&target_tokens[..at_idx]);
            if !pre_target_tokens.is_empty() {
                let target = parse_target_phrase(&pre_target_tokens)?;
                return Ok(EffectAst::DelayedUntilEndOfCombat {
                    effects: vec![EffectAst::DealDamage { amount, target }],
                });
            }
        }
    }

    let target = parse_target_phrase(&target_tokens)?;
    Ok(EffectAst::DealDamage { amount, target })
}

pub(crate) fn parse_instead_if_control_predicate(
    tokens: &[Token],
) -> Result<Option<PredicateAst>, CardTextError> {
    let clause_words = words(tokens);
    let starts_with_you_control = clause_words.starts_with(&["you", "control"])
        || clause_words.starts_with(&["you", "controlled"]);
    if !starts_with_you_control {
        return Ok(None);
    }

    let mut filter_tokens = &tokens[2..];
    let mut min_count: Option<u32> = None;
    if let Some((count, used)) = parse_number(filter_tokens)
        && count > 1
    {
        let tail = &filter_tokens[used..];
        if tail.first().is_some_and(|token| token.is_word("or"))
            && tail.get(1).is_some_and(|token| token.is_word("more"))
        {
            min_count = Some(count);
            filter_tokens = &tail[2..];
        } else if tail.first().is_some_and(|token| token.is_word("or"))
            && tail.get(1).is_some_and(|token| token.is_word("fewer"))
        {
            // Keep unsupported "or fewer" variants as plain control checks for now.
            filter_tokens = &tail[2..];
        }
    }
    if filter_tokens
        .first()
        .is_some_and(|token| token.is_word("at"))
        && filter_tokens
            .get(1)
            .is_some_and(|token| token.is_word("least"))
        && let Some((count, used)) = parse_number(&filter_tokens[2..])
        && count > 1
    {
        min_count = Some(count);
        filter_tokens = &filter_tokens[2 + used..];
    }
    let cut_markers: &[&[&str]] = &[&["as", "you", "cast", "this", "spell"], &["this", "turn"]];
    for marker in cut_markers {
        if let Some(idx) = words(filter_tokens)
            .windows(marker.len())
            .position(|window| window == *marker)
        {
            let cut_idx =
                token_index_for_word_index(filter_tokens, idx).unwrap_or(filter_tokens.len());
            filter_tokens = &filter_tokens[..cut_idx];
            break;
        }
    }
    let mut filter_tokens = trim_commas(filter_tokens);
    let filter_words = words(&filter_tokens);
    let mut requires_different_powers = false;
    if filter_words.ends_with(&["with", "different", "powers"])
        || filter_words.ends_with(&["with", "different", "power"])
    {
        requires_different_powers = true;
        let cut_word_idx = filter_words.len().saturating_sub(3);
        let cut_token_idx =
            token_index_for_word_index(&filter_tokens, cut_word_idx).unwrap_or(filter_tokens.len());
        filter_tokens = trim_commas(&filter_tokens[..cut_token_idx]);
    }
    if filter_tokens.is_empty() {
        return Ok(None);
    }

    let other = filter_tokens
        .first()
        .is_some_and(|token| token.is_word("another") || token.is_word("other"));
    let filter = parse_object_filter(&filter_tokens, other)?;
    if let Some(count) = min_count {
        if requires_different_powers {
            return Ok(Some(
                PredicateAst::PlayerControlsAtLeastWithDifferentPowers {
                    player: PlayerAst::You,
                    filter,
                    count,
                },
            ));
        }
        Ok(Some(PredicateAst::PlayerControlsAtLeast {
            player: PlayerAst::You,
            filter,
            count,
        }))
    } else {
        Ok(Some(PredicateAst::PlayerControls {
            player: PlayerAst::You,
            filter,
        }))
    }
}

pub(crate) fn parse_move(tokens: &[Token]) -> Result<EffectAst, CardTextError> {
    let clause_words = words(tokens);
    if !clause_words.starts_with(&["all", "counters", "from"]) {
        return Err(CardTextError::ParseError(format!(
            "unsupported move clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let from_idx = tokens
        .iter()
        .position(|token| token.is_word("from"))
        .unwrap_or(2);
    let onto_idx = tokens
        .iter()
        .position(|token| token.is_word("onto"))
        .or_else(|| tokens.iter().position(|token| token.is_word("to")))
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing move destination (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;

    let from_tokens = &tokens[from_idx + 1..onto_idx];
    let to_tokens = &tokens[onto_idx + 1..];
    let from = parse_target_phrase(from_tokens)?;
    let to = parse_target_phrase(to_tokens)?;

    Ok(EffectAst::MoveAllCounters { from, to })
}

pub(crate) fn parse_draw(
    tokens: &[Token],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let clause_words = words(tokens);
    let mut parsed_that_many_minus_one = false;
    let mut parsed_that_many_plus_one = false;
    let mut consumed_embedded_card_keyword = false;
    let (mut count, used) = if clause_words.starts_with(&["that", "many"]) {
        let mut value = Value::EventValue(EventValueSpec::Amount);
        let consumed = 2usize;
        let rest = &tokens[consumed..];
        if rest
            .first()
            .is_some_and(|token| token.is_word("card") || token.is_word("cards"))
        {
            let trailing = trim_commas(&rest[1..]);
            let trailing_words = words(&trailing);
            if trailing_words.as_slice() == ["minus", "one"] {
                value = Value::EventValueOffset(EventValueSpec::Amount, -1);
                parsed_that_many_minus_one = true;
            } else if trailing_words.as_slice() == ["plus", "one"] {
                value = Value::EventValueOffset(EventValueSpec::Amount, 1);
                parsed_that_many_plus_one = true;
            } else if !trailing_words.is_empty()
                && !(trailing_words
                    .windows(2)
                    .any(|window| window[0] == "for" && window[1] == "each"))
            {
                return Err(CardTextError::ParseError(format!(
                    "unsupported trailing draw clause (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
        }
        (value, consumed)
    } else if let Some(value) = parse_draw_as_many_cards_value(tokens) {
        consumed_embedded_card_keyword = true;
        (value, tokens.len())
    } else if tokens.first().is_some_and(|token| token.is_word("another"))
        && tokens
            .get(1)
            .is_some_and(|token| token.is_word("card") || token.is_word("cards"))
    {
        (Value::Fixed(1), 1)
    } else if tokens
        .first()
        .is_some_and(|token| token.is_word("card") || token.is_word("cards"))
    {
        let tail = trim_commas(&tokens[1..]);
        let value = parse_draw_card_prefixed_count_value(&tail)?.ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing draw count (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
        consumed_embedded_card_keyword = true;
        (value, tokens.len())
    } else if tokens.first().is_some_and(|token| token.is_word("up"))
        && tokens.get(1).is_some_and(|token| token.is_word("to"))
    {
        let Some((amount, used_amount)) = parse_number(&tokens[2..]) else {
            return Err(CardTextError::ParseError(format!(
                "missing draw count (clause: '{}')",
                clause_words.join(" ")
            )));
        };
        (Value::Fixed(amount as i32), 2 + used_amount)
    } else {
        parse_value(tokens).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing draw count (clause: '{}')",
                clause_words.join(" ")
            ))
        })?
    };

    let rest = &tokens[used..];
    let tail = if consumed_embedded_card_keyword {
        trim_commas(rest)
    } else {
        let mut card_word_idx = 0usize;
        if rest
            .first()
            .is_some_and(|token| token.is_word("additional"))
        {
            card_word_idx = 1;
        }
        let Some(card_word) = rest.get(card_word_idx).and_then(Token::as_word) else {
            return Err(CardTextError::ParseError(
                "missing card keyword".to_string(),
            ));
        };
        if card_word != "card" && card_word != "cards" {
            return Err(CardTextError::ParseError(
                "missing card keyword".to_string(),
            ));
        }
        trim_commas(&rest[card_word_idx + 1..])
    };
    let player = match subject {
        Some(SubjectAst::Player(player)) => player,
        _ => PlayerAst::Implicit,
    };
    let mut effect = EffectAst::Draw {
        count: count.clone(),
        player,
    };

    if !tail.is_empty() {
        let tail_words = words(&tail);
        if !((parsed_that_many_minus_one && tail_words.as_slice() == ["minus", "one"])
            || (parsed_that_many_plus_one && tail_words.as_slice() == ["plus", "one"]))
        {
            let has_for_each = tail
                .windows(2)
                .any(|window| window[0].is_word("for") && window[1].is_word("each"));
            if has_for_each {
                let dynamic = parse_dynamic_cost_modifier_value(&tail)?.ok_or_else(|| {
                    CardTextError::ParseError(format!(
                        "unsupported draw for-each clause (clause: '{}')",
                        words(tokens).join(" ")
                    ))
                })?;
                match count {
                    Value::Fixed(1) => count = dynamic,
                    _ => {
                        return Err(CardTextError::ParseError(format!(
                            "unsupported multiplied draw count (clause: '{}')",
                            words(tokens).join(" ")
                        )));
                    }
                }
                effect = EffectAst::Draw {
                    count: count.clone(),
                    player,
                };
            } else if let Some(parsed) = parse_draw_trailing_clause(&tail, effect.clone())? {
                effect = parsed;
            } else {
                return Err(CardTextError::ParseError(format!(
                    "unsupported trailing draw clause (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
        }
    }
    Ok(effect)
}

pub(crate) fn parse_draw_trailing_clause(
    tokens: &[Token],
    draw_effect: EffectAst,
) -> Result<Option<EffectAst>, CardTextError> {
    let tail_words = words(tokens);
    if tail_words.as_slice() == ["instead"] {
        return Ok(Some(draw_effect));
    }

    if let Some(timing) = parse_draw_delayed_timing_words(&tail_words) {
        return Ok(Some(wrap_return_with_delayed_timing(
            draw_effect,
            Some(timing),
        )));
    }

    if tail_words.first().copied() == Some("if") {
        let predicate_tokens = trim_commas(&tokens[1..]);
        if predicate_tokens.is_empty() {
            return Err(CardTextError::ParseError(
                "missing condition after trailing if clause".to_string(),
            ));
        }
        let predicate = parse_predicate(&predicate_tokens)?;
        return Ok(Some(EffectAst::Conditional {
            predicate,
            if_true: vec![draw_effect],
            if_false: Vec::new(),
        }));
    }

    if tail_words.first().copied() == Some("unless") {
        return try_build_unless(vec![draw_effect], tokens, 0);
    }

    Ok(None)
}

pub(crate) fn parse_draw_delayed_timing_words(words: &[&str]) -> Option<DelayedReturnTimingAst> {
    if let Some(timing) = parse_delayed_return_timing_words(words) {
        return Some(timing);
    }

    if matches!(
        words,
        ["at", "beginning", "of", "next", "turns", "upkeep"]
            | ["at", "beginning", "of", "the", "next", "turns", "upkeep"]
            | ["at", "the", "beginning", "of", "next", "turns", "upkeep"]
            | [
                "at",
                "the",
                "beginning",
                "of",
                "the",
                "next",
                "turns",
                "upkeep"
            ]
    ) {
        return Some(DelayedReturnTimingAst::NextUpkeep(PlayerAst::Any));
    }

    None
}

pub(crate) fn parse_draw_as_many_cards_value(tokens: &[Token]) -> Option<Value> {
    let clause_words = words(tokens);
    let starts_as_many = clause_words.len() >= 4
        && clause_words[0] == "as"
        && clause_words[1] == "many"
        && matches!(clause_words[2], "card" | "cards")
        && clause_words[3] == "as";
    if !starts_as_many {
        return None;
    }

    let tail = &clause_words[4..];
    let references_previous_event = tail.windows(2).any(|window| window == ["this", "way"]);
    if references_previous_event {
        return Some(Value::EventValue(EventValueSpec::Amount));
    }

    None
}

pub(crate) fn parse_draw_card_prefixed_count_value(
    tokens: &[Token],
) -> Result<Option<Value>, CardTextError> {
    if tokens.is_empty() {
        return Ok(None);
    }

    if let Some(value) = parse_draw_equal_to_value(tokens)? {
        return Ok(Some(value));
    }
    if let Some(value) = parse_dynamic_cost_modifier_value(tokens)? {
        return Ok(Some(value));
    }

    Ok(None)
}

pub(crate) fn parse_draw_equal_to_value(tokens: &[Token]) -> Result<Option<Value>, CardTextError> {
    let clause_words = words(tokens);
    if !clause_words.starts_with(&["equal", "to"]) {
        return Ok(None);
    }

    if let Some(value) = parse_devotion_value_from_add_clause(tokens)? {
        return Ok(Some(value));
    }
    if let Some(value) = parse_add_mana_equal_amount_value(tokens)
        .or_else(|| parse_equal_to_number_of_opponents_you_have_value(tokens))
        .or_else(|| parse_equal_to_number_of_counters_on_reference_value(tokens))
        .or_else(|| parse_equal_to_aggregate_filter_value(tokens))
        .or_else(|| parse_equal_to_number_of_filter_plus_or_minus_fixed_value(tokens))
        .or_else(|| parse_equal_to_number_of_filter_value(tokens))
    {
        return Ok(Some(value));
    }
    if clause_words
        .windows(2)
        .any(|window| window == ["this", "way"])
    {
        return Ok(Some(Value::EventValue(EventValueSpec::Amount)));
    }
    if let Some(value) = parse_dynamic_cost_modifier_value(tokens)? {
        return Ok(Some(value));
    }

    Ok(None)
}

pub(crate) fn parse_counter(tokens: &[Token]) -> Result<EffectAst, CardTextError> {
    if let Some(if_idx) = tokens.iter().position(|token| token.is_word("if")) {
        if if_idx == 0 || if_idx + 1 >= tokens.len() {
            return Err(CardTextError::ParseError(format!(
                "missing conditional counter target or predicate (clause: '{}')",
                words(tokens).join(" ")
            )));
        }
        let target_tokens = trim_commas(&tokens[..if_idx]);
        let predicate_tokens = trim_commas(&tokens[if_idx + 1..]);
        let target = parse_counter_target_phrase(&target_tokens)?;
        let predicate = parse_predicate(&predicate_tokens)?;
        return Ok(EffectAst::Conditional {
            predicate,
            if_true: vec![EffectAst::Counter { target }],
            if_false: Vec::new(),
        });
    }

    if let Some(unless_idx) = tokens.iter().position(|token| token.is_word("unless")) {
        let target_tokens = &tokens[..unless_idx];
        let target = parse_counter_target_phrase(target_tokens)?;

        let unless_tokens = &tokens[unless_idx + 1..];
        let pays_idx = unless_tokens
            .iter()
            .position(|token| token.is_word("pays"))
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing pays keyword (clause: '{}')",
                    words(tokens).join(" ")
                ))
            })?;

        // Parse the contiguous mana payment immediately following "pays".
        // Stop at the first non-mana word so trailing dynamic qualifiers
        // ("for each ...", "where X is ...", "plus an additional ...") do not
        // accidentally duplicate symbols.
        let mut mana = Vec::new();
        let mut trailing_start: Option<usize> = None;
        for (offset, token) in unless_tokens[pays_idx + 1..].iter().enumerate() {
            let Some(word) = token.as_word() else {
                continue;
            };
            match parse_mana_symbol(word) {
                Ok(symbol) => mana.push(symbol),
                Err(_) => {
                    trailing_start = Some(pays_idx + 1 + offset);
                    break;
                }
            }
        }

        let mut life = None;
        let mut additional_generic = None;
        if mana.is_empty() {
            let payment_tokens = trim_commas(&unless_tokens[pays_idx + 1..]);
            let payment_words = words(&payment_tokens);
            // "unless its controller pays mana equal to ..." uses a dynamic generic payment.
            if payment_words.first().copied() == Some("mana")
                && let Some(value) = parse_equal_to_aggregate_filter_value(&payment_tokens)
                    .or_else(|| parse_equal_to_number_of_filter_value(&payment_tokens))
            {
                additional_generic = Some(value);
                trailing_start = None;
            } else {
                return Err(CardTextError::ParseError(format!(
                    "missing mana cost (clause: '{}')",
                    words(tokens).join(" ")
                )));
            }
        }

        if let Some(trailing_idx) = trailing_start {
            let trailing_tokens = trim_commas(&unless_tokens[trailing_idx..]);
            let trailing_words = words(&trailing_tokens);
            if trailing_tokens
                .first()
                .is_some_and(|token| token.is_word("and"))
            {
                let life_tokens = trim_commas(&trailing_tokens[1..]);
                if let Some((amount, used)) = parse_value(&life_tokens)
                    && life_tokens
                        .get(used)
                        .is_some_and(|token| token.is_word("life"))
                    && trim_commas(&life_tokens[used + 1..]).is_empty()
                {
                    life = Some(amount);
                } else {
                    parser_trace("parse_counter:ignored-trailing-unless-payment", tokens);
                }
            } else if let Some(value) =
                parse_counter_unless_additional_generic_value(&trailing_tokens)?
            {
                additional_generic = Some(value);
            } else if trailing_words.starts_with(&["for", "each"]) {
                if let Some(dynamic) = parse_dynamic_cost_modifier_value(&trailing_tokens)? {
                    if let [ManaSymbol::Generic(multiplier)] = mana.as_slice() {
                        additional_generic =
                            Some(scale_value_multiplier(dynamic, *multiplier as i32));
                        mana.clear();
                    } else {
                        parser_trace("parse_counter:ignored-trailing-unless-payment", tokens);
                    }
                } else {
                    parser_trace("parse_counter:ignored-trailing-unless-payment", tokens);
                }
            } else if !trailing_words.is_empty() {
                parser_trace("parse_counter:ignored-trailing-unless-payment", tokens);
            }
        }

        if mana.is_empty() && life.is_none() && additional_generic.is_none() {
            return Err(CardTextError::ParseError(format!(
                "missing mana cost (clause: '{}')",
                words(tokens).join(" ")
            )));
        }

        return Ok(EffectAst::CounterUnlessPays {
            target,
            mana,
            life,
            additional_generic,
        });
    }

    let target = parse_counter_target_phrase(tokens)?;
    Ok(EffectAst::Counter { target })
}

fn parse_counter_target_phrase(tokens: &[Token]) -> Result<TargetAst, CardTextError> {
    if let Some(target) = parse_counter_ability_target_phrase(tokens)? {
        return Ok(target);
    }

    let clause_words = words(tokens);
    if clause_words.contains(&"ability")
        && (clause_words.contains(&"activated") || clause_words.contains(&"triggered"))
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported counter-ability target clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    parse_target_phrase(tokens)
}

fn parse_counter_ability_target_phrase(
    tokens: &[Token],
) -> Result<Option<TargetAst>, CardTextError> {
    let clause_tokens = trim_commas(tokens);
    let clause_words = words(&clause_tokens);
    if !clause_words.contains(&"ability")
        || (!clause_words.contains(&"activated") && !clause_words.contains(&"triggered"))
    {
        return Ok(None);
    }

    let mut idx = 0usize;
    let mut target_count: Option<ChoiceCount> = None;
    if clause_tokens
        .get(idx)
        .is_some_and(|token| token.is_word("up"))
        && clause_tokens
            .get(idx + 1)
            .is_some_and(|token| token.is_word("to"))
        && let Some((count, used)) = parse_number(&clause_tokens[idx + 2..])
    {
        target_count = Some(ChoiceCount::up_to(count as usize));
        idx += 2 + used;
    } else if let Some((count, used)) = parse_number(&clause_tokens[idx..])
        && clause_tokens
            .get(idx + used)
            .is_some_and(|token| token.is_word("target"))
    {
        target_count = Some(ChoiceCount::exactly(count as usize));
        idx += used;
    } else if let Some((count, used)) = parse_target_count_range_prefix(&clause_tokens[idx..])
        && clause_tokens
            .get(idx + used)
            .is_some_and(|token| token.is_word("target"))
    {
        target_count = Some(count);
        idx += used;
    }

    if !clause_tokens
        .get(idx)
        .is_some_and(|token| token.is_word("target"))
    {
        return Ok(None);
    }
    idx += 1;

    #[derive(Clone, Copy)]
    enum CounterTargetTerm {
        Ability,
        Spell,
    }

    let mut term_filters: Vec<(ObjectFilter, CounterTargetTerm)> = Vec::new();
    let mut list_end = clause_tokens.len();
    let mut scan = idx;
    while scan < clause_tokens.len() {
        if clause_tokens
            .get(scan)
            .is_some_and(|token| token.is_word("from"))
        {
            list_end = scan;
            break;
        }
        if clause_tokens
            .get(scan)
            .is_some_and(|token| token.is_word("you"))
            && clause_tokens
                .get(scan + 1)
                .is_some_and(|token| token.is_word("dont"))
            && clause_tokens
                .get(scan + 2)
                .is_some_and(|token| token.is_word("control"))
        {
            list_end = scan;
            break;
        }
        scan += 1;
    }

    while idx < list_end {
        let Some(word) = clause_tokens.get(idx).and_then(Token::as_word) else {
            idx += 1;
            continue;
        };
        if matches!(word, "or" | "and") {
            idx += 1;
            continue;
        }

        if word == "activated"
            && clause_tokens
                .get(idx + 1)
                .is_some_and(|token| token.is_word("or"))
            && clause_tokens
                .get(idx + 2)
                .is_some_and(|token| token.is_word("triggered"))
            && clause_tokens
                .get(idx + 3)
                .is_some_and(|token| token.is_word("ability"))
        {
            term_filters.push((
                ObjectFilter::activated_ability(),
                CounterTargetTerm::Ability,
            ));
            let mut triggered = ObjectFilter::ability();
            triggered.stack_kind = Some(crate::filter::StackObjectKind::TriggeredAbility);
            term_filters.push((triggered, CounterTargetTerm::Ability));
            idx += 4;
            continue;
        }

        if word == "triggered"
            && clause_tokens
                .get(idx + 1)
                .is_some_and(|token| token.is_word("or"))
            && clause_tokens
                .get(idx + 2)
                .is_some_and(|token| token.is_word("activated"))
            && clause_tokens
                .get(idx + 3)
                .is_some_and(|token| token.is_word("ability"))
        {
            let mut triggered = ObjectFilter::ability();
            triggered.stack_kind = Some(crate::filter::StackObjectKind::TriggeredAbility);
            term_filters.push((triggered, CounterTargetTerm::Ability));
            term_filters.push((
                ObjectFilter::activated_ability(),
                CounterTargetTerm::Ability,
            ));
            idx += 4;
            continue;
        }

        if word == "activated"
            && clause_tokens
                .get(idx + 1)
                .is_some_and(|token| token.is_word("ability"))
        {
            term_filters.push((
                ObjectFilter::activated_ability(),
                CounterTargetTerm::Ability,
            ));
            idx += 2;
            continue;
        }

        if word == "triggered"
            && clause_tokens
                .get(idx + 1)
                .is_some_and(|token| token.is_word("ability"))
        {
            let mut triggered = ObjectFilter::ability();
            triggered.stack_kind = Some(crate::filter::StackObjectKind::TriggeredAbility);
            term_filters.push((triggered, CounterTargetTerm::Ability));
            idx += 2;
            continue;
        }

        if word == "spell" {
            term_filters.push((ObjectFilter::spell(), CounterTargetTerm::Spell));
            idx += 1;
            continue;
        }

        if word == "instant"
            && clause_tokens
                .get(idx + 1)
                .is_some_and(|token| token.is_word("spell"))
        {
            term_filters.push((
                ObjectFilter::spell().with_type(CardType::Instant),
                CounterTargetTerm::Spell,
            ));
            idx += 2;
            continue;
        }

        if word == "sorcery"
            && clause_tokens
                .get(idx + 1)
                .is_some_and(|token| token.is_word("spell"))
        {
            term_filters.push((
                ObjectFilter::spell().with_type(CardType::Sorcery),
                CounterTargetTerm::Spell,
            ));
            idx += 2;
            continue;
        }

        if word == "legendary"
            && clause_tokens
                .get(idx + 1)
                .is_some_and(|token| token.is_word("spell"))
        {
            term_filters.push((
                ObjectFilter::spell().with_supertype(Supertype::Legendary),
                CounterTargetTerm::Spell,
            ));
            idx += 2;
            continue;
        }

        if word == "noncreature"
            && clause_tokens
                .get(idx + 1)
                .is_some_and(|token| token.is_word("spell"))
        {
            let mut filter = ObjectFilter::noncreature_spell().in_zone(Zone::Stack);
            filter.stack_kind = Some(crate::filter::StackObjectKind::Spell);
            term_filters.push((filter, CounterTargetTerm::Spell));
            idx += 2;
            continue;
        }

        return Ok(None);
    }

    if term_filters.is_empty() {
        return Ok(None);
    }

    let mut source_types: Vec<CardType> = Vec::new();
    let mut opponent_controlled = false;
    while idx < clause_tokens.len() {
        let Some(word) = clause_tokens.get(idx).and_then(Token::as_word) else {
            idx += 1;
            continue;
        };
        if matches!(word, "and" | "or") {
            idx += 1;
            continue;
        }
        if word == "you"
            && clause_tokens
                .get(idx + 1)
                .is_some_and(|token| token.is_word("dont"))
            && clause_tokens
                .get(idx + 2)
                .is_some_and(|token| token.is_word("control"))
        {
            opponent_controlled = true;
            idx += 3;
            continue;
        }
        if word == "from" {
            idx += 1;
            if clause_tokens
                .get(idx)
                .is_some_and(|token| matches!(token.as_word(), Some("a" | "an" | "the")))
            {
                idx += 1;
            }

            let mut parsed_type = false;
            while idx < clause_tokens.len() {
                let Some(type_word) = clause_tokens.get(idx).and_then(Token::as_word) else {
                    idx += 1;
                    continue;
                };
                if matches!(type_word, "source" | "sources") {
                    idx += 1;
                    break;
                }
                if matches!(type_word, "and" | "or") {
                    idx += 1;
                    continue;
                }
                let parsed = parse_card_type(type_word)
                    .or_else(|| type_word.strip_suffix('s').and_then(parse_card_type));
                let Some(card_type) = parsed else {
                    return Ok(None);
                };
                source_types.push(card_type);
                parsed_type = true;
                idx += 1;
            }
            if !parsed_type {
                return Ok(None);
            }
            continue;
        }

        return Ok(None);
    }

    for (filter, term) in &mut term_filters {
        if opponent_controlled {
            *filter = filter.clone().opponent_controls();
        }
        if !source_types.is_empty() && matches!(term, CounterTargetTerm::Ability) {
            for card_type in &source_types {
                *filter = filter.clone().with_type(*card_type);
            }
        }
    }

    let target_filter = if term_filters.len() == 1 {
        term_filters
            .pop()
            .map(|(filter, _)| filter)
            .expect("single term filter should be present")
    } else {
        let mut any = ObjectFilter::default();
        any.any_of = term_filters.into_iter().map(|(filter, _)| filter).collect();
        any
    };

    let target = wrap_target_count(
        TargetAst::Object(target_filter, span_from_tokens(&clause_tokens), None),
        target_count,
    );
    Ok(Some(target))
}

pub(crate) fn scale_value_multiplier(value: Value, multiplier: i32) -> Value {
    if multiplier <= 0 {
        return Value::Fixed(0);
    }
    if multiplier == 1 {
        return value;
    }
    match value {
        Value::Fixed(amount) => Value::Fixed(amount * multiplier),
        Value::Count(filter) => Value::CountScaled(filter, multiplier),
        Value::CountScaled(filter, factor) => Value::CountScaled(filter, factor * multiplier),
        other => {
            let mut result = Value::Fixed(0);
            for _ in 0..multiplier {
                result = match result {
                    Value::Fixed(0) => other.clone(),
                    _ => Value::Add(Box::new(result), Box::new(other.clone())),
                };
            }
            result
        }
    }
}

pub(crate) fn parse_counter_unless_additional_generic_value(
    tokens: &[Token],
) -> Result<Option<Value>, CardTextError> {
    if tokens.is_empty() || !tokens[0].is_word("plus") {
        return Ok(None);
    }

    let mut idx = 1usize;
    if tokens.get(idx).is_some_and(|token| token.is_word("an")) {
        idx += 1;
    }
    if !tokens
        .get(idx)
        .is_some_and(|token| token.is_word("additional"))
    {
        return Ok(None);
    }
    idx += 1;

    let symbol_word = tokens
        .get(idx)
        .and_then(Token::as_word)
        .ok_or_else(|| CardTextError::ParseError("missing additional mana symbol".to_string()))?;
    let symbol = parse_mana_symbol(symbol_word).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported additional payment symbol '{}' in counter clause",
            symbol_word
        ))
    })?;
    let multiplier = match symbol {
        ManaSymbol::Generic(amount) => amount as i32,
        _ => {
            return Err(CardTextError::ParseError(
                "unsupported nongeneric additional counter payment".to_string(),
            ));
        }
    };

    let filter_tokens = trim_commas(&tokens[idx + 1..]);
    let filter_words = words(&filter_tokens);
    if !filter_words.starts_with(&["for", "each"]) {
        return Err(CardTextError::ParseError(format!(
            "unsupported additional counter payment tail (clause: '{}')",
            words(tokens).join(" ")
        )));
    }

    let dynamic = parse_dynamic_cost_modifier_value(&filter_tokens)?.ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported additional counter payment filter (clause: '{}')",
            words(tokens).join(" ")
        ))
    })?;
    Ok(Some(scale_value_multiplier(dynamic, multiplier)))
}

pub(crate) fn parse_reveal(
    tokens: &[Token],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let player = match subject {
        Some(SubjectAst::Player(player)) => player,
        _ => PlayerAst::Implicit,
    };

    let words = words(tokens);
    // Many effects split "reveal it/that card/those cards" into a standalone clause.
    // The engine does not model hidden information, so this compiles to a semantic no-op
    // that still allows parsing and auditing to proceed.
    if matches!(
        words.as_slice(),
        ["it"]
            | ["them"]
            | ["that"]
            | ["that", "card"]
            | ["those", "cards"]
            | ["those"]
            | ["this", "card"]
            | ["this"]
    ) {
        return Ok(EffectAst::RevealTagged {
            tag: TagKey::from(IT_TAG),
        });
    }
    let reveals_from_among = words.contains(&"from")
        && words.contains(&"among")
        && (words.contains(&"them") || words.contains(&"those"));
    if reveals_from_among {
        return Ok(EffectAst::RevealTagged {
            tag: TagKey::from(IT_TAG),
        });
    }
    let reveals_outside_game = words.contains(&"outside") && words.contains(&"game");
    if reveals_outside_game {
        return Ok(EffectAst::RevealTagged {
            tag: TagKey::from(IT_TAG),
        });
    }
    let reveals_first_draw = words.starts_with(&["the", "first", "card", "you", "draw"]);
    if reveals_first_draw {
        return Ok(EffectAst::RevealTagged {
            tag: TagKey::from(IT_TAG),
        });
    }
    let reveals_card_this_way =
        (words.contains(&"card") || words.contains(&"cards")) && words.ends_with(&["this", "way"]);
    if reveals_card_this_way {
        return Ok(EffectAst::RevealTagged {
            tag: TagKey::from(IT_TAG),
        });
    }
    let reveals_conditional_it = words.first() == Some(&"it") && words.contains(&"if");
    if reveals_conditional_it {
        return Ok(EffectAst::RevealTagged {
            tag: TagKey::from(IT_TAG),
        });
    }
    if words.contains(&"hand") {
        let is_full_hand_reveal = matches!(words.as_slice(), ["your", "hand"] | ["their", "hand"])
            || words.as_slice() == ["his", "or", "her", "hand"];
        if !is_full_hand_reveal {
            if words.contains(&"from") {
                return Ok(EffectAst::RevealTagged {
                    tag: TagKey::from(IT_TAG),
                });
            }
            return Err(CardTextError::ParseError(format!(
                "unsupported reveal-hand clause (clause: '{}')",
                words.join(" ")
            )));
        }
        return Ok(EffectAst::RevealHand { player });
    }

    let has_card = words.contains(&"card") || words.contains(&"cards");
    let has_library = words.contains(&"library") || words.contains(&"libraries");
    let explicit_top_card =
        words.as_slice() == ["top", "card"] || words.as_slice() == ["the", "top", "card"];

    if !has_card || (!has_library && !explicit_top_card) {
        return Err(CardTextError::ParseError(format!(
            "unsupported reveal clause (clause: '{}')",
            words.join(" ")
        )));
    }

    Ok(EffectAst::RevealTop { player })
}

pub(crate) fn parse_life_amount(
    tokens: &[Token],
    amount_kind: &str,
) -> Result<(Value, usize), CardTextError> {
    let clause_words = words(tokens);
    if clause_words == ["that", "much", "life"] {
        // "that much life" binds to the triggering event amount.
        return Ok((Value::EventValue(EventValueSpec::Amount), 2));
    }

    parse_value(tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing {amount_kind} amount (clause: '{}')",
            clause_words.join(" ")
        ))
    })
}

pub(crate) fn parse_life_equal_to_value(tokens: &[Token]) -> Result<Option<Value>, CardTextError> {
    let clause_words = words(tokens);
    if !clause_words.starts_with(&["life", "equal", "to"]) {
        return Ok(None);
    }

    let amount_tokens = &tokens[1..];
    let amount_words = words(amount_tokens);

    if let Some(value) = parse_add_mana_equal_amount_value(amount_tokens) {
        return Ok(Some(value));
    }
    if let Some(value) = parse_devotion_value_from_add_clause(amount_tokens)? {
        return Ok(Some(value));
    }
    if let Some(value) = parse_equal_to_number_of_filter_value(amount_tokens) {
        return Ok(Some(value));
    }
    if matches!(
        amount_words.as_slice(),
        ["equal", "to", "the", "life", "lost", "this", "way"]
            | ["equal", "to", "life", "lost", "this", "way"]
            | [
                "equal", "to", "the", "amount", "of", "life", "lost", "this", "way"
            ]
            | ["equal", "to", "amount", "of", "life", "lost", "this", "way"]
    ) {
        return Ok(Some(Value::EventValue(EventValueSpec::LifeAmount)));
    }
    if let Some(value) = parse_dynamic_cost_modifier_value(amount_tokens)? {
        return Ok(Some(value));
    }

    Err(CardTextError::ParseError(format!(
        "missing life amount in equal-to clause (clause: '{}')",
        clause_words.join(" ")
    )))
}

pub(crate) fn parse_life_amount_from_trailing(
    base_amount: &Value,
    trailing: &[Token],
) -> Result<Option<Value>, CardTextError> {
    if trailing.is_empty() {
        return Ok(None);
    }

    if let Some(dynamic) = parse_dynamic_cost_modifier_value(trailing)? {
        if let Some(multiplier) = match base_amount {
            Value::Fixed(value) => Some(*value),
            Value::X => Some(1),
            _ => None,
        } {
            return Ok(Some(scale_value_multiplier(dynamic, multiplier)));
        }
    }

    if let Some(where_value) = parse_where_x_value_clause(trailing) {
        if value_contains_unbound_x(base_amount) {
            let clause = words(trailing).join(" ");
            return Ok(Some(replace_unbound_x_with_value(
                base_amount.clone(),
                &where_value,
                &clause,
            )?));
        }
        if matches!(base_amount, Value::Fixed(1)) {
            return Ok(Some(where_value));
        }
    }

    Ok(None)
}

pub(crate) fn validate_life_keyword(rest: &[Token]) -> Result<(), CardTextError> {
    if rest
        .first()
        .and_then(Token::as_word)
        .is_some_and(|word| word != "life")
    {
        return Err(CardTextError::ParseError(
            "missing life keyword".to_string(),
        ));
    }
    Ok(())
}

pub(crate) fn remap_source_stat_value_to_it(value: Value) -> Value {
    match value {
        Value::PowerOf(spec) if matches!(spec.as_ref(), ChooseSpec::Source) => {
            Value::PowerOf(Box::new(ChooseSpec::Tagged(TagKey::from(IT_TAG))))
        }
        Value::ToughnessOf(spec) if matches!(spec.as_ref(), ChooseSpec::Source) => {
            Value::ToughnessOf(Box::new(ChooseSpec::Tagged(TagKey::from(IT_TAG))))
        }
        Value::ManaValueOf(spec) if matches!(spec.as_ref(), ChooseSpec::Source) => {
            Value::ManaValueOf(Box::new(ChooseSpec::Tagged(TagKey::from(IT_TAG))))
        }
        Value::Add(left, right) => Value::Add(
            Box::new(remap_source_stat_value_to_it(*left)),
            Box::new(remap_source_stat_value_to_it(*right)),
        ),
        other => other,
    }
}

fn player_filter_for_life_reference(player: PlayerAst) -> Option<PlayerFilter> {
    match player {
        PlayerAst::You | PlayerAst::Implicit => Some(PlayerFilter::You),
        PlayerAst::Any => Some(PlayerFilter::Any),
        PlayerAst::Opponent => Some(PlayerFilter::Opponent),
        PlayerAst::Target => Some(PlayerFilter::target_player()),
        PlayerAst::TargetOpponent => Some(PlayerFilter::target_opponent()),
        PlayerAst::That => Some(PlayerFilter::IteratedPlayer),
        PlayerAst::Defending => Some(PlayerFilter::Defending),
        PlayerAst::Attacking => Some(PlayerFilter::Attacking),
        PlayerAst::ItsController | PlayerAst::ItsOwner => None,
    }
}

fn parse_half_life_value(tokens: &[Token], player: PlayerAst) -> Option<Value> {
    let clause_words = words(tokens);
    if clause_words.first().copied() != Some("half")
        || !clause_words.contains(&"life")
        || clause_words.contains(&"lost")
    {
        return None;
    }

    let player_filter = player_filter_for_life_reference(player)?;
    let rounded_down = clause_words
        .windows(2)
        .any(|window| window == ["rounded", "down"]);
    if rounded_down {
        Some(Value::HalfLifeTotalRoundedDown(player_filter))
    } else {
        Some(Value::HalfLifeTotalRoundedUp(player_filter))
    }
}

pub(crate) fn parse_lose_life(
    tokens: &[Token],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let player = match subject {
        Some(SubjectAst::Player(player)) => player,
        _ => PlayerAst::Implicit,
    };

    let clause_words = words(tokens);
    if let Some(mut amount) = parse_life_equal_to_value(tokens)? {
        if matches!(player, PlayerAst::ItsController | PlayerAst::ItsOwner)
            && (clause_words
                .windows(2)
                .any(|window| window == ["its", "power"])
                || clause_words
                    .windows(2)
                    .any(|window| window == ["its", "toughness"])
                || clause_words
                    .windows(3)
                    .any(|window| window == ["its", "mana", "value"]))
        {
            amount = remap_source_stat_value_to_it(amount);
        }
        return Ok(EffectAst::LoseLife { amount, player });
    }
    if clause_words.as_slice() == ["the", "game"] {
        return Ok(EffectAst::LoseGame { player });
    }

    if let Some(amount) = parse_half_life_value(tokens, player) {
        return Ok(EffectAst::LoseLife { amount, player });
    }

    let (mut amount, used) = parse_life_amount(tokens, "life loss")?;

    let rest = &tokens[used..];
    validate_life_keyword(rest)?;
    let trailing = trim_commas(&rest[1..]);
    if !trailing.is_empty() {
        if let Some(resolved) = parse_life_amount_from_trailing(&amount, &trailing)? {
            amount = resolved;
            return Ok(EffectAst::LoseLife { amount, player });
        }
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing life-loss clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    Ok(EffectAst::LoseLife { amount, player })
}

pub(crate) fn parse_gain_life(
    tokens: &[Token],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let player = match subject {
        Some(SubjectAst::Player(player)) => player,
        _ => PlayerAst::Implicit,
    };

    if let Some(amount) = parse_life_equal_to_value(tokens)? {
        return Ok(EffectAst::GainLife { amount, player });
    }

    let (mut amount, used) = parse_life_amount(tokens, "life gain")?;

    let rest = &tokens[used..];
    validate_life_keyword(rest)?;
    let trailing = trim_commas(&rest[1..]);
    if !trailing.is_empty() {
        let trailing_words = words(&trailing);
        if trailing_words
            .windows(6)
            .any(|window| window == ["then", "shuffle", "your", "graveyard", "into", "your"])
            && trailing_words.contains(&"library")
        {
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing life-gain shuffle-graveyard clause (clause: '{}')",
                words(tokens).join(" ")
            )));
        }
        if let Some(resolved) = parse_life_amount_from_trailing(&amount, &trailing)? {
            amount = resolved;
            return Ok(EffectAst::GainLife { amount, player });
        }
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing life-gain clause (clause: '{}')",
            words(tokens).join(" ")
        )));
    }

    Ok(EffectAst::GainLife { amount, player })
}

pub(crate) fn parse_gain_control(
    tokens: &[Token],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let clause_words = words(tokens);
    let has_dynamic_power_bound = clause_words.contains(&"power")
        && clause_words.contains(&"number")
        && clause_words
            .windows(2)
            .any(|window| window == ["you", "control"]);
    if has_dynamic_power_bound {
        return Err(CardTextError::ParseError(format!(
            "unsupported dynamic power-bound control clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let mut idx = 0;
    if tokens
        .get(idx)
        .is_some_and(|token| token.is_word("control"))
    {
        idx += 1;
    } else {
        return Err(CardTextError::ParseError(
            "missing control keyword".to_string(),
        ));
    }

    if tokens.get(idx).is_some_and(|token| token.is_word("of")) {
        idx += 1;
    }

    let duration_idx = tokens[idx..]
        .iter()
        .position(|token| token.is_word("during") || token.is_word("until"))
        .map(|offset| idx + offset)
        .or_else(|| {
            tokens[idx..]
                .windows(4)
                .position(|window| {
                    window[0].is_word("for")
                        && window[1].is_word("as")
                        && window[2].is_word("long")
                        && window[3].is_word("as")
                })
                .map(|offset| idx + offset)
        });

    let target_tokens = if let Some(dur_idx) = duration_idx {
        &tokens[idx..dur_idx]
    } else {
        &tokens[idx..]
    };
    if target_tokens
        .iter()
        .any(|token| token.is_word("if") || token.is_word("unless"))
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported conditional gain-control clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let target_ast = parse_target_phrase(target_tokens)?;
    let duration_tokens = duration_idx
        .map(|dur_idx| &tokens[dur_idx..])
        .unwrap_or(&[]);
    let duration = parse_control_duration(duration_tokens)?;
    let player = match subject {
        Some(SubjectAst::Player(player)) => player,
        _ => PlayerAst::Implicit,
    };
    match target_ast {
        TargetAst::Player(filter, _) => Ok(EffectAst::ControlPlayer {
            player: PlayerFilter::Target(Box::new(filter)),
            duration,
        }),
        _ => {
            let until = match duration {
                ControlDurationAst::UntilEndOfTurn => Until::EndOfTurn,
                ControlDurationAst::Forever => Until::Forever,
                ControlDurationAst::AsLongAsYouControlSource => Until::YouStopControllingThis,
                ControlDurationAst::DuringNextTurn => {
                    return Err(CardTextError::ParseError(
                        "unsupported control duration for permanents".to_string(),
                    ));
                }
            };
            Ok(EffectAst::GainControl {
                target: target_ast,
                player,
                duration: until,
            })
        }
    }
}

pub(crate) fn parse_control_duration(
    tokens: &[Token],
) -> Result<ControlDurationAst, CardTextError> {
    if tokens.is_empty() {
        return Ok(ControlDurationAst::Forever);
    }

    let words = words(tokens);
    let has_for_as_long_as = words
        .windows(4)
        .any(|window| window == ["for", "as", "long", "as"]);
    if has_for_as_long_as
        && words.contains(&"you")
        && words.contains(&"control")
        && (words.contains(&"this")
            || words.contains(&"thiss")
            || words.contains(&"source")
            || words.contains(&"creature")
            || words.contains(&"permanent"))
    {
        return Ok(ControlDurationAst::AsLongAsYouControlSource);
    }

    let has_during = words.contains(&"during");
    let has_next = words.contains(&"next");
    let has_turn = words.contains(&"turn");
    if has_during && has_next && has_turn {
        return Ok(ControlDurationAst::DuringNextTurn);
    }

    let has_until = words.contains(&"until");
    let has_end = words.contains(&"end");
    if has_until && has_end && has_turn {
        return Ok(ControlDurationAst::UntilEndOfTurn);
    }

    Err(CardTextError::ParseError(
        "unsupported control duration".to_string(),
    ))
}

pub(crate) fn parse_put_into_hand(
    tokens: &[Token],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    fn force_object_targeting(target: TargetAst, span: TextSpan) -> TargetAst {
        match target {
            TargetAst::Object(filter, explicit_span, fixed_span) => {
                TargetAst::Object(filter, explicit_span.or(Some(span)), fixed_span)
            }
            TargetAst::WithCount(inner, count) => {
                TargetAst::WithCount(Box::new(force_object_targeting(*inner, span)), count)
            }
            other => other,
        }
    }

    fn expand_graveyard_or_hand_disjunction(
        mut target: TargetAst,
        target_tokens: &[Token],
    ) -> TargetAst {
        let target_words = words(target_tokens);
        let has_graveyard = target_words
            .iter()
            .any(|word| matches!(*word, "graveyard" | "graveyards"));
        let has_hand = target_words
            .iter()
            .any(|word| matches!(*word, "hand" | "hands"));
        if !(has_graveyard && has_hand) {
            return target;
        }

        fn apply(filter: &ObjectFilter) -> ObjectFilter {
            let mut graveyard = filter.clone();
            graveyard.any_of.clear();
            graveyard.zone = Some(Zone::Graveyard);

            let mut hand = filter.clone();
            hand.any_of.clear();
            hand.zone = Some(Zone::Hand);

            let mut disjunction = ObjectFilter::default();
            disjunction.any_of = vec![graveyard, hand];
            disjunction
        }

        match &mut target {
            TargetAst::Object(filter, _, _) => {
                *filter = apply(filter);
            }
            TargetAst::WithCount(inner, _) => {
                if let TargetAst::Object(filter, _, _) = inner.as_mut() {
                    *filter = apply(filter);
                }
            }
            _ => {}
        }

        target
    }

    let player = match subject {
        Some(SubjectAst::Player(player)) => player,
        _ => PlayerAst::Implicit,
    };

    let clause_words = words(tokens);

    // "Put them/it back in any order." (typically after looking at the top cards of a library).
    if clause_words.contains(&"back")
        && clause_words.contains(&"any")
        && clause_words.contains(&"order")
        && matches!(clause_words.first().copied(), Some("it" | "them"))
    {
        return Ok(EffectAst::ReorderTopOfLibrary {
            tag: TagKey::from(IT_TAG),
        });
    }

    if clause_words.contains(&"from")
        && clause_words.contains(&"among")
        && clause_words.contains(&"hand")
    {
        return Ok(EffectAst::PutSomeIntoHandRestIntoGraveyard { player, count: 1 });
    }
    let has_it = clause_words.contains(&"it");
    let has_them = clause_words.contains(&"them");
    let has_hand = clause_words.contains(&"hand");
    let has_into = clause_words.contains(&"into");

    if has_hand && has_into && (has_it || has_them) {
        // "Put N of them into your hand and the rest on the bottom of your library in any order."
        if has_them
            && clause_words.contains(&"rest")
            && clause_words.contains(&"bottom")
            && clause_words.contains(&"library")
            && clause_words.iter().any(|w| *w == "and" || *w == "then")
        {
            let (count, used) = parse_number(tokens).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing put count (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?;
            let mut idx = used;
            if tokens.get(idx).is_some_and(|t| t.is_word("of")) {
                idx += 1;
            }
            if !tokens.get(idx).is_some_and(|t| t.is_word("them")) {
                return Err(CardTextError::ParseError(format!(
                    "unsupported multi-destination put clause (clause: '{}')",
                    clause_words.join(" ")
                )));
            }

            let dest_player = if clause_words.contains(&"your") {
                PlayerAst::You
            } else if clause_words.contains(&"their")
                || clause_words.starts_with(&["that", "player"])
                || clause_words.starts_with(&["that", "players"])
            {
                PlayerAst::That
            } else {
                player
            };

            return Ok(EffectAst::PutSomeIntoHandRestOnBottomOfLibrary {
                player: dest_player,
                count: count as u32,
            });
        }

        // "Put N of them into your hand and the rest into your graveyard."
        if has_them
            && clause_words.contains(&"rest")
            && clause_words.contains(&"graveyard")
            && clause_words.iter().any(|w| *w == "and" || *w == "then")
        {
            let (count, used) = parse_number(tokens).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing put count (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?;
            // Accept optional "of" before "them".
            let mut idx = used;
            if tokens.get(idx).is_some_and(|t| t.is_word("of")) {
                idx += 1;
            }
            if !tokens.get(idx).is_some_and(|t| t.is_word("them")) {
                return Err(CardTextError::ParseError(format!(
                    "unsupported multi-destination put clause (clause: '{}')",
                    clause_words.join(" ")
                )));
            }

            // The chooser is typically the player whose hand is referenced.
            let dest_player = if clause_words.contains(&"your") {
                PlayerAst::You
            } else if clause_words.contains(&"their")
                || clause_words.starts_with(&["that", "player"])
                || clause_words.starts_with(&["that", "players"])
            {
                PlayerAst::That
            } else {
                player
            };

            return Ok(EffectAst::PutSomeIntoHandRestIntoGraveyard {
                player: dest_player,
                count: count as u32,
            });
        }

        return Ok(EffectAst::PutIntoHand {
            player,
            object: ObjectRefAst::It,
        });
    }

    // Support destination-first wording:
    // "Put onto the battlefield under your control all creature cards ..."
    if tokens.first().is_some_and(|token| token.is_word("onto")) {
        let mut idx = 1usize;
        while tokens
            .get(idx)
            .and_then(Token::as_word)
            .is_some_and(is_article)
        {
            idx += 1;
        }
        if !tokens
            .get(idx)
            .is_some_and(|token| token.is_word("battlefield"))
        {
            return Err(CardTextError::ParseError(format!(
                "unsupported put destination after 'onto' (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        idx += 1;

        let mut battlefield_tapped = false;
        if tokens.get(idx).is_some_and(|token| token.is_word("tapped")) {
            battlefield_tapped = true;
            idx += 1;
        }

        let mut battlefield_controller = ReturnControllerAst::Preserve;
        if tokens.get(idx).is_some_and(|token| token.is_word("under")) {
            let tail_words = words(&tokens[idx..]);
            let consumed = if tail_words.starts_with(&["under", "your", "control"]) {
                battlefield_controller = ReturnControllerAst::You;
                Some(3usize)
            } else if tail_words.starts_with(&["under", "its", "owners", "control"])
                || tail_words.starts_with(&["under", "their", "owners", "control"])
                || tail_words.starts_with(&["under", "that", "players", "control"])
            {
                battlefield_controller = ReturnControllerAst::Owner;
                Some(4usize)
            } else {
                None
            };
            if let Some(consumed) = consumed {
                idx += consumed;
            }
        }

        let target_tokens = trim_commas(&tokens[idx..]);
        if target_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing target before 'onto' (clause: '{}')",
                clause_words.join(" ")
            )));
        }

        if target_tokens
            .first()
            .is_some_and(|token| token.is_word("attached"))
            && target_tokens
                .get(1)
                .is_some_and(|token| token.is_word("to"))
        {
            let after_to = &target_tokens[2..];
            if after_to.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "missing attachment target after 'attached to' (clause: '{}')",
                    clause_words.join(" ")
                )));
            }

            let attachment_target_len = if after_to.first().is_some_and(|token| token.is_word("it"))
            {
                1usize
            } else if after_to.len() >= 2
                && after_to[0].is_word("that")
                && after_to[1].as_word().is_some_and(|word| {
                    matches!(
                        word,
                        "creature" | "permanent" | "object" | "aura" | "equipment"
                    )
                })
            {
                2usize
            } else {
                return Err(CardTextError::ParseError(format!(
                    "unsupported attachment target after 'attached to' (clause: '{}')",
                    clause_words.join(" ")
                )));
            };

            let attachment_target = parse_target_phrase(&after_to[..attachment_target_len])?;
            let object_tokens = trim_commas(&after_to[attachment_target_len..]);
            if object_tokens.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "missing object after attachment target (clause: '{}')",
                    clause_words.join(" ")
                )));
            }

            let mut object_target = parse_target_phrase(&object_tokens)?;
            object_target = expand_graveyard_or_hand_disjunction(object_target, &object_tokens);
            object_target = force_object_targeting(object_target, tokens[0].span());

            return Ok(EffectAst::MoveToZone {
                target: object_target,
                zone: Zone::Battlefield,
                to_top: false,
                battlefield_controller,
                battlefield_tapped,
                attached_to: Some(attachment_target),
            });
        }

        if !target_tokens
            .first()
            .is_some_and(|token| token.is_word("attached"))
        {
            let mut rewritten = target_tokens;
            rewritten.push(Token::Word("onto".to_string(), tokens[0].span()));
            rewritten.extend_from_slice(&tokens[1..idx]);
            return parse_put_into_hand(&rewritten, subject);
        }
    }

    if let Some(on_idx) = tokens.iter().position(|token| token.is_word("on"))
        && tokens
            .get(on_idx + 1)
            .is_some_and(|token| token.is_word("top"))
        && tokens
            .get(on_idx + 2)
            .is_some_and(|token| token.is_word("of"))
    {
        let target_tokens = trim_commas(&tokens[..on_idx]);
        if target_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing target before 'on top of' (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let destination_words = words(&tokens[on_idx + 3..]);
        if !destination_words.contains(&"library") {
            return Err(CardTextError::ParseError(format!(
                "unsupported put destination after 'on top of' (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let target = if let Some((count, used)) = parse_number(&target_tokens)
            && target_tokens
                .get(used)
                .is_some_and(|token| token.is_word("card") || token.is_word("cards"))
        {
            let inner = parse_target_phrase(&target_tokens[used..])?;
            TargetAst::WithCount(Box::new(inner), ChoiceCount::exactly(count as usize))
        } else {
            parse_target_phrase(&target_tokens)?
        };
        return Ok(EffectAst::MoveToZone {
            target,
            zone: Zone::Library,
            to_top: true,
            battlefield_controller: ReturnControllerAst::Preserve,
            battlefield_tapped: false,
            attached_to: None,
        });
    }

    if let Some(on_idx) = tokens.iter().position(|token| token.is_word("on")) {
        let mut bottom_idx = on_idx + 1;
        if tokens
            .get(bottom_idx)
            .is_some_and(|token| token.is_word("the"))
        {
            bottom_idx += 1;
        }
        if tokens
            .get(bottom_idx)
            .is_some_and(|token| token.is_word("bottom"))
            && tokens
                .get(bottom_idx + 1)
                .is_some_and(|token| token.is_word("of"))
        {
            let target_tokens = trim_commas(&tokens[..on_idx]);
            if target_tokens.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "missing target before 'on bottom of' (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            let destination_words = words(&tokens[bottom_idx + 2..]);
            if !destination_words.contains(&"library") {
                return Err(CardTextError::ParseError(format!(
                    "unsupported put destination after 'on bottom of' (clause: '{}')",
                    clause_words.join(" ")
                )));
            }

            let target_words = words(&target_tokens);
            let is_rest_target =
                target_words.as_slice() == ["the", "rest"] || target_words.as_slice() == ["rest"];
            if is_rest_target {
                return Ok(EffectAst::PutRestOnBottomOfLibrary);
            }

            let target = if let Some((count, used)) = parse_number(&target_tokens)
                && target_tokens
                    .get(used)
                    .is_some_and(|token| token.is_word("card") || token.is_word("cards"))
            {
                let inner = parse_target_phrase(&target_tokens[used..])?;
                TargetAst::WithCount(Box::new(inner), ChoiceCount::exactly(count as usize))
            } else {
                parse_target_phrase(&target_tokens)?
            };

            return Ok(EffectAst::MoveToZone {
                target,
                zone: Zone::Library,
                to_top: false,
                battlefield_controller: ReturnControllerAst::Preserve,
                battlefield_tapped: false,
                attached_to: None,
            });
        }
    }

    if let Some(into_idx) = tokens.iter().position(|token| token.is_word("into")) {
        let target_tokens = trim_commas(&tokens[..into_idx]);
        if target_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing target before 'into' (clause: '{}')",
                clause_words.join(" ")
            )));
        }

        let destination_words: Vec<&str> = words(&tokens[into_idx + 1..])
            .into_iter()
            .filter(|word| !is_article(word))
            .collect();
        let zone = if destination_words.contains(&"hand") || destination_words.contains(&"hands") {
            Some(Zone::Hand)
        } else if destination_words.contains(&"graveyard")
            || destination_words.contains(&"graveyards")
        {
            Some(Zone::Graveyard)
        } else if destination_words.contains(&"library")
            && destination_words.ends_with(&["second", "from", "top"])
        {
            return Ok(EffectAst::MoveToLibrarySecondFromTop {
                target: parse_target_phrase(&target_tokens)?,
            });
        } else {
            None
        };

        if let Some(zone) = zone {
            let target_words = words(&target_tokens);
            if zone == Zone::Graveyard
                && matches!(target_words.as_slice(), ["the", "rest"] | ["rest"])
            {
                return Ok(EffectAst::MoveToZone {
                    target: TargetAst::Object(
                        ObjectFilter::tagged(TagKey::from(IT_TAG)),
                        None,
                        None,
                    ),
                    zone,
                    to_top: false,
                    battlefield_controller: ReturnControllerAst::Preserve,
                    battlefield_tapped: false,
                    attached_to: None,
                });
            }

            if zone == Zone::Hand {
                if matches!(
                    target_words.as_slice(),
                    ["it"] | ["them"] | ["that", "card"] | ["those", "card"] | ["those", "cards"]
                ) {
                    return Ok(EffectAst::PutIntoHand {
                        player,
                        object: ObjectRefAst::It,
                    });
                }
            }

            return Ok(EffectAst::MoveToZone {
                target: parse_target_phrase(&target_tokens)?,
                zone,
                to_top: false,
                battlefield_controller: ReturnControllerAst::Preserve,
                battlefield_tapped: false,
                attached_to: None,
            });
        }
    }

    if let Some(onto_idx) = tokens.iter().position(|token| token.is_word("onto")) {
        let target_tokens = trim_commas(&tokens[..onto_idx]);
        if target_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing target before 'onto' (clause: '{}')",
                clause_words.join(" ")
            )));
        }

        let destination_words: Vec<&str> = words(&tokens[onto_idx + 1..])
            .into_iter()
            .filter(|word| !is_article(word))
            .collect();
        if destination_words.first() != Some(&"battlefield") {
            return Err(CardTextError::ParseError(format!(
                "unsupported put destination after 'onto' (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let mut destination_tail: Vec<&str> = destination_words[1..].to_vec();
        let battlefield_tapped = destination_tail.contains(&"tapped");
        destination_tail.retain(|word| *word != "tapped");
        let supported_control_tail = destination_tail.is_empty()
            || destination_tail.as_slice() == ["under", "your", "control"]
            || destination_tail.as_slice() == ["under", "its", "owners", "control"]
            || destination_tail.as_slice() == ["under", "their", "owners", "control"]
            || destination_tail.as_slice() == ["under", "that", "players", "control"];
        if !supported_control_tail {
            return Err(CardTextError::ParseError(format!(
                "unsupported put destination after 'onto' (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let battlefield_controller = if destination_tail.as_slice() == ["under", "your", "control"]
        {
            ReturnControllerAst::You
        } else if destination_tail.as_slice() == ["under", "its", "owners", "control"]
            || destination_tail.as_slice() == ["under", "their", "owners", "control"]
            || destination_tail.as_slice() == ["under", "that", "players", "control"]
        {
            ReturnControllerAst::Owner
        } else {
            ReturnControllerAst::Preserve
        };

        if target_tokens
            .first()
            .is_some_and(|token| token.is_word("all") || token.is_word("each"))
        {
            let mut filter = parse_object_filter(&target_tokens[1..], false)?;
            let target_words = words(&target_tokens[1..]);
            if target_words
                .windows(2)
                .any(|window| window == ["from", "it"])
            {
                filter.zone = Some(Zone::Hand);
                if filter.owner.is_none() {
                    filter.owner = Some(PlayerFilter::You);
                }
                filter
                    .tagged_constraints
                    .retain(|constraint| constraint.tag.as_str() != IT_TAG);
            }
            if clause_words.contains(&"among") && clause_words.contains(&"them") {
                filter.zone = Some(Zone::Exile);
                if filter.owner.is_none() {
                    filter.owner = Some(PlayerFilter::IteratedPlayer);
                }
                if clause_words.contains(&"permanent") {
                    filter.card_types = vec![
                        CardType::Artifact,
                        CardType::Creature,
                        CardType::Enchantment,
                        CardType::Land,
                        CardType::Planeswalker,
                        CardType::Battle,
                    ];
                }
            }
            return Ok(EffectAst::ReturnAllToBattlefield {
                filter,
                tapped: battlefield_tapped,
            });
        }

        return Ok(EffectAst::MoveToZone {
            target: parse_target_phrase(&target_tokens)?,
            zone: Zone::Battlefield,
            to_top: false,
            battlefield_controller,
            battlefield_tapped,
            attached_to: None,
        });
    }

    if clause_words.contains(&"sticker") {
        return Ok(EffectAst::Investigate {
            count: Value::Fixed(0),
        });
    }

    Err(CardTextError::ParseError(format!(
        "unsupported put clause (clause: '{}')",
        clause_words.join(" ")
    )))
}
