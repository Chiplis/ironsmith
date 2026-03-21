pub(crate) fn annihilator_granted_ability(amount: u32) -> Ability {
    Ability {
        kind: AbilityKind::Triggered(TriggeredAbility {
            trigger: Trigger::this_attacks(),
            effects: crate::resolution::ResolutionProgram::from_effects(vec![
                Effect::sacrifice_player(
                    ObjectFilter::permanent(),
                    Value::Fixed(amount as i32),
                    PlayerFilter::Defending,
                ),
            ]),
            choices: vec![],
            intervening_if: None,
        }),
        functional_zones: vec![Zone::Battlefield],
        text: Some(format!("Annihilator {amount}")),
    }
}

fn scale_value_by_factor(base: Value, factor: u32) -> Option<Value> {
    if factor == 0 {
        return None;
    }

    let mut value = base.clone();
    for _ in 1..factor {
        value = Value::Add(Box::new(value), Box::new(base.clone()));
    }
    Some(value)
}

pub(crate) fn display_text_for_tokens(
    tokens: &[Token],
    capitalize_effect_start: bool,
) -> String {
    let mut text = String::new();
    let mut needs_space = false;
    let mut in_effect_text = false;
    let mut capitalize_next_effect_word = false;
    let mut capitalize_next_cost_action = true;

    for token in tokens {
        match token {
            Token::Word(word, _) => {
                if needs_space && !text.is_empty() {
                    text.push(' ');
                }
                let numeric_like = word
                    .chars()
                    .all(|ch| ch.is_ascii_digit() || matches!(ch, 'x' | 'X' | '+' | '-' | '/'));
                let mut rendered = match word.as_str() {
                    "t" => "{T}".to_string(),
                    "q" => "{Q}".to_string(),
                    _ if in_effect_text && numeric_like => word.clone(),
                    _ => crate::cards::builders::parse_mana_symbol(word)
                        .map(|symbol| ManaCost::from_symbols(vec![symbol]).to_oracle())
                        .unwrap_or_else(|_| word.clone()),
                };
                if !in_effect_text
                    && capitalize_next_cost_action
                    && matches!(
                        word.as_str(),
                        "sacrifice" | "discard" | "exile" | "remove" | "reveal" | "pay"
                    )
                {
                    if let Some(first) = rendered.get_mut(0..1) {
                        first.make_ascii_uppercase();
                    }
                }
                if capitalize_next_effect_word {
                    if let Some(first) = rendered.get_mut(0..1) {
                        first.make_ascii_uppercase();
                    }
                    capitalize_next_effect_word = false;
                }
                text.push_str(&rendered);
                needs_space = true;
                capitalize_next_cost_action = false;
            }
            Token::Colon(_) => {
                text.push(':');
                needs_space = true;
                in_effect_text = true;
                capitalize_next_effect_word = capitalize_effect_start;
            }
            Token::Comma(_) => {
                text.push(',');
                needs_space = true;
                if !in_effect_text {
                    capitalize_next_cost_action = true;
                }
            }
            Token::Period(_) => {
                text.push('.');
                needs_space = true;
                if in_effect_text {
                    capitalize_next_effect_word = capitalize_effect_start;
                }
            }
            Token::Semicolon(_) => {
                text.push(';');
                needs_space = true;
            }
            Token::Quote(_) => {}
        }
    }

    text
}

pub(crate) fn cumulative_upkeep_granted_ability(
    mana_symbols_per_counter: Vec<ManaSymbol>,
    life_per_counter: u32,
    text: String,
) -> Ability {
    let age_count = Value::CountersOnSource(CounterType::Age);
    let life = scale_value_by_factor(age_count.clone(), life_per_counter);
    let mana_multiplier = if mana_symbols_per_counter.is_empty() {
        None
    } else {
        Some(age_count)
    };

    Ability {
        kind: AbilityKind::Triggered(TriggeredAbility {
            trigger: Trigger::beginning_of_upkeep(PlayerFilter::You),
            effects: crate::resolution::ResolutionProgram::from_effects(vec![
                Effect::put_counters_on_source(CounterType::Age, 1),
                Effect::unless_pays_with_life_additional_and_multiplier(
                    vec![Effect::sacrifice_source()],
                    PlayerFilter::You,
                    mana_symbols_per_counter,
                    life,
                    None,
                    mana_multiplier,
                ),
            ]),
            choices: vec![],
            intervening_if: None,
        }),
        functional_zones: vec![Zone::Battlefield],
        text: Some(text),
    }
}

pub(crate) fn parse_equipped_creature_has_line(
    tokens: &[Token],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let words = words(tokens);
    let clause_text = words.join(" ");
    if words.len() < 4 || words[0] != "equipped" || words[1] != "creature" || words[2] != "has" {
        return Ok(None);
    }

    let ability_tokens = trim_edge_punctuation(&tokens[3..]);
    if ability_tokens.is_empty() {
        return Ok(None);
    }

    let mut actions_to_grant = Vec::new();
    let mut extra_grants: Vec<StaticAbilityAst> = Vec::new();
    let Some(actions) = parse_ability_line(&ability_tokens) else {
        return Ok(None);
    };
    for action in actions {
        reject_unimplemented_keyword_actions(std::slice::from_ref(&action), &clause_text)?;
        if let KeywordAction::Annihilator(amount) = action {
            extra_grants.push(StaticAbilityAst::AttachedObjectAbilityGrant {
                ability: parsed_ability_from_ability(annihilator_granted_ability(amount)),
                display: format!("equipped creature has annihilator {amount}"),
                condition: None,
            });
            continue;
        }
        if let KeywordAction::CumulativeUpkeep {
            mana_symbols_per_counter,
            life_per_counter,
            text,
        } = action
        {
            extra_grants.push(StaticAbilityAst::AttachedObjectAbilityGrant {
                ability: parsed_ability_from_ability(cumulative_upkeep_granted_ability(
                    mana_symbols_per_counter,
                    life_per_counter,
                    text.clone(),
                )),
                display: format!("equipped creature has {}", text.to_ascii_lowercase()),
                condition: None,
            });
            continue;
        }
        if action.lowers_to_static_ability() {
            actions_to_grant.push(action);
        }
    }

    if actions_to_grant.is_empty() && extra_grants.is_empty() {
        return Ok(None);
    }

    let mut out = Vec::new();
    if !actions_to_grant.is_empty() {
        out.push(StaticAbilityAst::EquipmentKeywordActionsGrant {
            actions: actions_to_grant,
        });
    }
    out.extend(extra_grants);
    Ok(Some(out))
}

pub(crate) fn parse_enchanted_creature_has_line(
    tokens: &[Token],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let line_words = words(tokens);
    let clause_text = line_words.join(" ");
    if line_words.len() < 4 || line_words.first().copied() != Some("enchanted") {
        return Ok(None);
    }
    let subject = match line_words.get(1).copied() {
        Some("creature") => "enchanted creature",
        Some("permanent") => "enchanted permanent",
        _ => return Ok(None),
    };
    if line_words.get(2).copied() != Some("has") {
        return Ok(None);
    }

    let mut ability_tokens = trim_edge_punctuation(&tokens[3..]);
    if ability_tokens.is_empty() {
        return Ok(None);
    }

    let mut condition: Option<crate::ConditionExpr> = None;
    if let Some(as_long_idx) = words(&ability_tokens)
        .windows(3)
        .position(|window| window == ["as", "long", "as"])
    {
        let Some(as_long_token_idx) = token_index_for_word_index(&ability_tokens, as_long_idx)
        else {
            return Ok(None);
        };
        let Some(condition_start_idx) =
            token_index_for_word_index(&ability_tokens, as_long_idx + 3)
        else {
            return Ok(None);
        };
        let ability_head = trim_edge_punctuation(&ability_tokens[..as_long_token_idx]);
        if ability_head.is_empty() {
            return Ok(None);
        }
        let condition_tokens = trim_edge_punctuation(&ability_tokens[condition_start_idx..]);
        if condition_tokens.is_empty() {
            return Ok(None);
        }
        condition = Some(parse_static_condition_clause(&condition_tokens)?);
        ability_tokens = ability_head;
    }

    let Some(actions) = parse_ability_line(&ability_tokens) else {
        return Ok(None);
    };
    let mut out = Vec::new();
    for action in actions {
        reject_unimplemented_keyword_actions(std::slice::from_ref(&action), &clause_text)?;
        if let KeywordAction::Annihilator(amount) = action {
            out.push(StaticAbilityAst::AttachedObjectAbilityGrant {
                ability: parsed_ability_from_ability(annihilator_granted_ability(amount)),
                display: format!("{subject} has annihilator {amount}"),
                condition: condition.clone(),
            });
            continue;
        }
        if let KeywordAction::CumulativeUpkeep {
            mana_symbols_per_counter,
            life_per_counter,
            text,
        } = action
        {
            let ability_text = format!("{subject} has {}", text.to_ascii_lowercase());
            out.push(StaticAbilityAst::AttachedObjectAbilityGrant {
                ability: parsed_ability_from_ability(cumulative_upkeep_granted_ability(
                    mana_symbols_per_counter,
                    life_per_counter,
                    text,
                )),
                display: ability_text,
                condition: condition.clone(),
            });
            continue;
        }

        if !action.lowers_to_static_ability() {
            continue;
        }
        let ability_text = format!("{subject} has {}", action.display_text().to_ascii_lowercase());
        out.push(StaticAbilityAst::AttachedKeywordActionGrant {
            action,
            display: ability_text,
            condition: condition.clone(),
        });
    }

    if out.is_empty() {
        return Ok(None);
    }
    Ok(Some(out))
}

pub(crate) fn parse_attached_has_and_loses_keywords_line(
    tokens: &[Token],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let line_words = words(tokens);
    if line_words.len() < 7 {
        return Ok(None);
    }

    let is_enchanted = matches!(
        line_words.get(..2),
        Some(["enchanted", "creature"] | ["enchanted", "permanent"])
    );
    let is_equipped = matches!(line_words.get(..2), Some(["equipped", "creature"]));
    if !is_enchanted && !is_equipped {
        return Ok(None);
    }
    if line_words.get(2).copied() != Some("has") {
        return Ok(None);
    }

    let Some(and_idx) = tokens.iter().position(|token| token.is_word("and")) else {
        return Ok(None);
    };
    if and_idx <= 3
        || !tokens
            .get(and_idx + 1)
            .is_some_and(|token| token.is_word("lose") || token.is_word("loses"))
    {
        return Ok(None);
    }

    let grant_tokens = trim_edge_punctuation(&tokens[3..and_idx]);
    let lose_tokens = trim_edge_punctuation(&tokens[and_idx + 2..]);
    if grant_tokens.is_empty() || lose_tokens.is_empty() {
        return Ok(None);
    }

    let Some(granted_actions) = parse_ability_line(&grant_tokens) else {
        return Ok(None);
    };
    let Some(removed_actions) = parse_ability_line(&lose_tokens) else {
        return Ok(None);
    };

    let clause_text = line_words.join(" ");
    let filter = parse_object_filter(&tokens[..2], false)?;
    let mut result = Vec::new();

    for action in granted_actions {
        reject_unimplemented_keyword_actions(std::slice::from_ref(&action), &clause_text)?;
        if !action.lowers_to_static_ability() {
            return Ok(None);
        }
        result.push(StaticAbilityAst::GrantKeywordAction {
            filter: filter.clone(),
            action,
            condition: None,
        });
    }

    for action in removed_actions {
        reject_unimplemented_keyword_actions(std::slice::from_ref(&action), &clause_text)?;
        if !action.lowers_to_static_ability() {
            return Ok(None);
        }
        result.push(StaticAbilityAst::RemoveKeywordAction {
            filter: filter.clone(),
            action,
        });
    }

    if result.is_empty() {
        return Ok(None);
    }
    Ok(Some(result))
}

pub(crate) fn parse_attached_cant_attack_or_block_line(
    tokens: &[Token],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let normalized = normalize_cant_words(tokens);
    if normalized.len() < 4 {
        return Ok(None);
    }

    let is_enchanted_creature = normalized.starts_with(&["enchanted", "creature"]);
    let is_enchanted_permanent = normalized.starts_with(&["enchanted", "permanent"]);
    let is_equipped_creature = normalized.starts_with(&["equipped", "creature"]);
    if !is_enchanted_creature && !is_enchanted_permanent && !is_equipped_creature {
        return Ok(None);
    }

    let subject_len = 2usize;
    let tail = &normalized[subject_len..];
    if !tail.starts_with(&["cant"]) {
        return Ok(None);
    }

    let subject = if is_equipped_creature {
        "equipped creature"
    } else if is_enchanted_permanent {
        "enchanted permanent"
    } else {
        "enchanted creature"
    };

    let (restriction, display) = if tail == ["cant", "attack"] {
        (
            crate::effect::Restriction::attack(ObjectFilter::source()),
            format!("{subject} can't attack"),
        )
    } else if tail == ["cant", "block"] {
        (
            crate::effect::Restriction::block(ObjectFilter::source()),
            format!("{subject} can't block"),
        )
    } else if tail == ["cant", "attack", "or", "block"] {
        (
            crate::effect::Restriction::attack_or_block(ObjectFilter::source()),
            format!("{subject} can't attack or block"),
        )
    } else {
        return Ok(None);
    };

    Ok(Some(StaticAbilityAst::AttachedStaticAbilityGrant {
        ability: Box::new(StaticAbilityAst::Static(StaticAbility::restriction(
            restriction,
            display.clone(),
        ))),
        display: normalized.join(" "),
        condition: None,
    }))
}

pub(crate) fn parse_you_control_attached_creature_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let line_words = words(tokens);
    if line_words.len() < 4 || !line_words.starts_with(&["you", "control"]) {
        return Ok(None);
    }

    let tail = &line_words[2..];
    let is_attached_subject = matches!(
        tail,
        ["enchanted", "creature"]
            | ["enchanted", "permanent"]
            | ["enchanted", "land"]
            | ["enchanted", "artifact"]
            | ["equipped", "creature"]
            | ["equipped", "permanent"]
    );
    if !is_attached_subject {
        return Ok(None);
    }

    Ok(Some(StaticAbility::control_attached_permanent(
        line_words.join(" "),
    )))
}

pub(crate) fn parse_attached_gets_and_cant_block_line(
    tokens: &[Token],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let line_words = words(tokens);
    if line_words.len() < 6 {
        return Ok(None);
    }

    let is_enchanted = line_words.starts_with(&["enchanted", "creature"]);
    let is_equipped = line_words.starts_with(&["equipped", "creature"]);
    if !is_enchanted && !is_equipped {
        return Ok(None);
    }

    let Some(get_idx) = tokens
        .iter()
        .position(|token| token.is_word("get") || token.is_word("gets"))
    else {
        return Ok(None);
    };
    let Some(and_idx) = tokens.iter().position(|token| token.is_word("and")) else {
        return Ok(None);
    };
    if get_idx >= and_idx {
        return Ok(None);
    }

    let tail_tokens = trim_edge_punctuation(&tokens[and_idx + 1..]);
    let tail_words = normalize_cant_words(&tail_tokens);
    let subject = if is_enchanted {
        "enchanted creature"
    } else {
        "equipped creature"
    };
    let anthem = build_anthem_static_ability(&parse_anthem_clause(tokens, get_idx, and_idx)?);
    let granted = match tail_words.as_slice() {
        ["cant", "block"] => StaticAbilityAst::AttachedStaticAbilityGrant {
            ability: Box::new(StaticAbilityAst::Static(StaticAbility::restriction(
                crate::effect::Restriction::block(ObjectFilter::source()),
                format!("{subject} can't block"),
            ))),
            display: format!("{subject} can't block"),
            condition: None,
        },
        ["cant", "attack"] => StaticAbilityAst::AttachedStaticAbilityGrant {
            ability: Box::new(StaticAbilityAst::Static(StaticAbility::restriction(
                crate::effect::Restriction::attack(ObjectFilter::source()),
                format!("{subject} can't attack"),
            ))),
            display: format!("{subject} can't attack"),
            condition: None,
        },
        ["cant", "attack", "or", "block"] => StaticAbilityAst::AttachedStaticAbilityGrant {
            ability: Box::new(StaticAbilityAst::Static(StaticAbility::restriction(
                crate::effect::Restriction::attack_or_block(ObjectFilter::source()),
                format!("{subject} can't attack or block"),
            ))),
            display: format!("{subject} can't attack or block"),
            condition: None,
        },
        ["cant", "be", "blocked"] => StaticAbilityAst::AttachedStaticAbilityGrant {
            ability: Box::new(StaticAbilityAst::KeywordAction(KeywordAction::Unblockable)),
            display: format!("{subject} can't be blocked"),
            condition: None,
        },
        _ => return Ok(None),
    };
    Ok(Some(vec![anthem.into(), granted]))
}

pub(crate) fn parse_prevent_damage_to_source_remove_counter_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let line_words = words(tokens);
    if line_words.len() < 12 {
        return Ok(None);
    }

    if !line_words.starts_with(&["if", "damage", "would", "be", "dealt", "to"]) {
        return Ok(None);
    }
    if !(line_words[6..].starts_with(&["this", "creature"])
        || line_words[6..].starts_with(&["this", "permanent"]))
    {
        return Ok(None);
    }
    if !line_words
        .windows(3)
        .any(|window| window == ["prevent", "that", "damage"])
    {
        return Ok(None);
    }

    let Some(remove_word_idx) = line_words.iter().position(|word| *word == "remove") else {
        return Ok(None);
    };
    let Some(counter_word_idx) = line_words[remove_word_idx + 1..]
        .iter()
        .position(|word| *word == "counter" || *word == "counters")
        .map(|idx| remove_word_idx + 1 + idx)
    else {
        return Ok(None);
    };

    let remove_token_idx =
        token_index_for_word_index(tokens, remove_word_idx).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unable to map remove clause in prevent-damage line (clause: '{}')",
                line_words.join(" ")
            ))
        })?;
    let counter_token_idx =
        token_index_for_word_index(tokens, counter_word_idx).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unable to map counter clause in prevent-damage line (clause: '{}')",
                line_words.join(" ")
            ))
        })?;

    let mut descriptor_tokens = trim_commas(&tokens[remove_token_idx + 1..=counter_token_idx]);
    if descriptor_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing counter descriptor in prevent-damage line (clause: '{}')",
            line_words.join(" ")
        )));
    }

    let (amount, used) = parse_number(&descriptor_tokens).unwrap_or((1, 0));
    descriptor_tokens = descriptor_tokens[used..].to_vec();
    while descriptor_tokens
        .first()
        .is_some_and(|token| token.is_word("a") || token.is_word("an"))
    {
        descriptor_tokens.remove(0);
    }

    let counter_type = parse_counter_type_from_tokens(&descriptor_tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported counter type in prevent-damage line (clause: '{}')",
            line_words.join(" ")
        ))
    })?;

    let after_counter_words = line_words.get(counter_word_idx + 1..).unwrap_or_default();
    let valid_tail = after_counter_words.starts_with(&["from", "this", "creature"])
        || after_counter_words.starts_with(&["from", "this", "permanent"])
        || after_counter_words.starts_with(&["from", "it"]);
    if !valid_tail {
        return Err(CardTextError::ParseError(format!(
            "unsupported prevent-damage remove tail (clause: '{}')",
            line_words.join(" ")
        )));
    }

    Ok(Some(StaticAbility::prevent_damage_to_self_remove_counter(
        counter_type,
        amount,
    )))
}

pub(crate) fn parse_attached_prevent_all_damage_dealt_by_attached_line(
    tokens: &[Token],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let line_words = words(tokens);
    if line_words.len() < 6 {
        return Ok(None);
    }

    // "Prevent all damage that would be dealt by enchanted creature."
    if !line_words.starts_with(&["prevent", "all", "damage"]) {
        return Ok(None);
    }
    if !line_words.ends_with(&["by", "enchanted", "creature"]) {
        return Ok(None);
    }

    let display = "prevent all damage that would be dealt by enchanted creature".to_string();
    Ok(Some(StaticAbilityAst::AttachedStaticAbilityGrant {
        ability: Box::new(StaticAbilityAst::Static(StaticAbility::new(
            crate::static_abilities::PreventAllDamageDealtByThisPermanent,
        ))),
        display,
        condition: None,
    }))
}

pub(crate) fn parse_attached_has_keywords_and_triggered_ability_line(
    tokens: &[Token],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let line_words = words(tokens);
    if line_words.len() < 6 {
        return Ok(None);
    }

    let is_enchanted = line_words.starts_with(&["enchanted", "creature"]);
    let is_equipped = line_words.starts_with(&["equipped", "creature"]);
    if !is_enchanted && !is_equipped {
        return Ok(None);
    }

    let Some(has_idx) = tokens.iter().position(|token| token.is_word("has")) else {
        return Ok(None);
    };
    if has_idx + 1 >= tokens.len() {
        return Ok(None);
    }

    let ability_tokens = trim_edge_punctuation(&tokens[has_idx + 1..]);
    if ability_tokens.is_empty() {
        return Ok(None);
    }

    let Some(and_idx) = ability_tokens.iter().position(|token| token.is_word("and")) else {
        return Ok(None);
    };
    if and_idx == 0 || and_idx + 1 >= ability_tokens.len() {
        return Ok(None);
    }

    let trigger_starts = ability_tokens
        .get(and_idx + 1)
        .is_some_and(|token| token.is_word("when") || token.is_word("whenever"))
        || is_at_trigger_intro(&ability_tokens, and_idx + 1);
    if !trigger_starts {
        return Ok(None);
    }

    let keyword_tokens = trim_edge_punctuation(&ability_tokens[..and_idx]);
    if keyword_tokens.is_empty() {
        return Ok(None);
    }

    let clause_text = line_words.join(" ");
    let mut keyword_actions = Vec::new();
    let mut extra_grants: Vec<StaticAbilityAst> = Vec::new();
    let Some(actions) = parse_ability_line(&keyword_tokens) else {
        return Ok(None);
    };
    for action in actions {
        reject_unimplemented_keyword_actions(std::slice::from_ref(&action), &clause_text)?;
        if let KeywordAction::Annihilator(amount) = action {
            extra_grants.push(StaticAbilityAst::AttachedObjectAbilityGrant {
                ability: parsed_ability_from_ability(annihilator_granted_ability(amount)),
                display: format!(
                    "{} has annihilator {amount}",
                    if is_equipped {
                        "equipped creature"
                    } else {
                        "enchanted creature"
                    }
                ),
                condition: None,
            });
        } else if action.lowers_to_static_ability() {
            keyword_actions.push(action);
        }
    }
    if keyword_actions.is_empty() && extra_grants.is_empty() {
        return Ok(None);
    }

    let trigger_tokens = trim_edge_punctuation(&ability_tokens[and_idx + 1..]);
    if trigger_tokens.is_empty() {
        return Ok(None);
    }
    let triggered = match parse_triggered_line(&trigger_tokens)? {
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
        _ => {
            return Err(CardTextError::ParseError(format!(
                "unsupported attached triggered grant clause (clause: '{}')",
                clause_text
            )));
        }
    };
    if parsed_triggered_ability_is_empty(&triggered) {
        return Err(CardTextError::ParseError(format!(
            "unsupported empty attached triggered grant clause (clause: '{}')",
            clause_text
        )));
    }

    let subject = match parse_anthem_subject(&tokens[..has_idx]) {
        Ok(subject) => subject,
        Err(_) => return Ok(None),
    };
    let filter = match subject {
        AnthemSubjectAst::Filter(filter) => filter,
        AnthemSubjectAst::Source => ObjectFilter::source(),
    };

    let mut static_abilities = Vec::new();
    for action in keyword_actions {
        static_abilities.push(StaticAbilityAst::GrantKeywordAction {
            filter: filter.clone(),
            action,
            condition: None,
        });
    }
    static_abilities.extend(extra_grants);
    let subject_text = words(&tokens[..has_idx]).join(" ");
    let display = format!("{subject_text} has {}", words(&trigger_tokens).join(" "));
    static_abilities.push(StaticAbilityAst::AttachedObjectAbilityGrant {
        ability: triggered,
        display,
        condition: None,
    });

    Ok(Some(static_abilities))
}

pub(crate) fn parse_attached_is_legendary_gets_and_has_keywords_line(
    tokens: &[Token],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let line_words = words(tokens);
    if line_words.len() < 10 {
        return Ok(None);
    }

    let is_enchanted = line_words.starts_with(&["enchanted", "creature"]);
    let is_equipped = line_words.starts_with(&["equipped", "creature"]);
    if !is_enchanted && !is_equipped {
        return Ok(None);
    }

    let Some(is_idx) = tokens.iter().position(|token| token.is_word("is")) else {
        return Ok(None);
    };
    if is_idx < 2
        || !tokens
            .get(is_idx + 1)
            .is_some_and(|token| token.is_word("legendary"))
    {
        return Ok(None);
    }

    let Some(get_idx) = tokens
        .iter()
        .position(|token| token.is_word("get") || token.is_word("gets"))
    else {
        return Ok(None);
    };
    let Some(has_idx) = tokens.iter().position(|token| token.is_word("has")) else {
        return Ok(None);
    };
    if !(is_idx < get_idx && get_idx + 1 < tokens.len() && get_idx < has_idx) {
        return Ok(None);
    }

    let subject_tokens = trim_commas(&tokens[..is_idx]);
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    let filter = parse_object_filter(&subject_tokens, false)?;

    let modifier_token = tokens.get(get_idx + 1).and_then(Token::as_word);
    let Some(modifier_token) = modifier_token else {
        return Ok(None);
    };
    let (power, toughness) = match parse_pt_modifier(modifier_token) {
        Ok(value) => value,
        Err(_) => return Ok(None),
    };

    let keyword_tokens = trim_edge_punctuation(&tokens[has_idx + 1..]);
    if keyword_tokens.is_empty() {
        return Ok(None);
    }
    let Some(actions) = parse_ability_line(&keyword_tokens) else {
        return Ok(None);
    };

    let clause_text = line_words.join(" ");
    let mut out = Vec::new();
    out.push(StaticAbility::add_supertypes(filter.clone(), vec![Supertype::Legendary]).into());

    let anthem_clause = ParsedAnthemClause {
        subject: AnthemSubjectAst::Filter(filter.clone()),
        power: AnthemValue::Fixed(power),
        toughness: AnthemValue::Fixed(toughness),
        condition: None,
    };
    out.push(build_anthem_static_ability(&anthem_clause).into());

    for action in actions {
        reject_unimplemented_keyword_actions(std::slice::from_ref(&action), &clause_text)?;
        if action.lowers_to_static_ability() {
            out.push(StaticAbilityAst::GrantKeywordAction {
                filter: filter.clone(),
                action,
                condition: None,
            });
        }
    }

    if out.is_empty() {
        return Ok(None);
    }
    Ok(Some(out))
}

pub(crate) fn parse_attached_gets_and_has_ability_line(
    tokens: &[Token],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let line_words = words(tokens);
    if line_words.len() < 6 {
        return Ok(None);
    }
    let is_enchanted = line_words.starts_with(&["enchanted", "creature"]);
    let is_equipped = line_words.starts_with(&["equipped", "creature"]);
    if !is_enchanted && !is_equipped {
        return Ok(None);
    }

    let Some(get_idx) = tokens
        .iter()
        .position(|token| token.is_word("get") || token.is_word("gets"))
    else {
        return Ok(None);
    };
    let Some(has_idx) = tokens.iter().position(|token| token.is_word("has")) else {
        return Ok(None);
    };
    let Some(and_idx) = tokens.iter().position(|token| token.is_word("and")) else {
        return Ok(None);
    };
    if !(get_idx < and_idx && and_idx < has_idx) {
        return Ok(None);
    }

    let clause = parse_anthem_clause(tokens, get_idx, and_idx)?;
    let anthem = build_anthem_static_ability(&clause);

    let ability_tokens = trim_edge_punctuation(&tokens[has_idx + 1..]);
    if ability_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing attached ability after 'has' (clause: '{}')",
            line_words.join(" ")
        )));
    }

    if let Some(actions) = parse_ability_line(&ability_tokens) {
        reject_unimplemented_keyword_actions(&actions, &line_words.join(" "))?;
        let mut out = vec![anthem.clone().into()];
        let mut granted_any = false;
        for action in actions {
            if action.lowers_to_static_ability() {
                out.push(grant_keyword_action_for_anthem_subject(&clause, action));
                granted_any = true;
            }
        }
        if granted_any {
            return Ok(Some(out));
        }
    }

    for and_idx in ability_tokens
        .iter()
        .enumerate()
        .filter_map(|(idx, token)| token.is_word("and").then_some(idx))
        .rev()
    {
        let keyword_tokens = trim_edge_punctuation(&ability_tokens[..and_idx]);
        let granted_tokens = trim_edge_punctuation(&ability_tokens[and_idx + 1..]);
        if keyword_tokens.is_empty() || granted_tokens.is_empty() {
            continue;
        }

        let Some(actions) = parse_ability_line(&keyword_tokens) else {
            continue;
        };
        reject_unimplemented_keyword_actions(&actions, &line_words.join(" "))?;
        let keyword_actions = actions
            .into_iter()
            .filter(|action| action.lowers_to_static_ability())
            .collect::<Vec<_>>();
        if keyword_actions.is_empty() {
            continue;
        }

        if let Some(parsed) = parse_activated_line(&granted_tokens)? {
            let mut out = vec![anthem.clone().into()];
            for action in keyword_actions {
                out.push(grant_keyword_action_for_anthem_subject(&clause, action));
            }
            let display = display_text_for_tokens(&granted_tokens, false);
            let grant = grant_object_ability_for_anthem_subject(&clause, parsed, display);
            out.push(grant);
            return Ok(Some(out));
        }
    }

    let has_colon = ability_tokens
        .iter()
        .any(|token| matches!(token, Token::Colon(_)));
    if let Some(parsed) = parse_activated_line(&ability_tokens)? {
        let display = display_text_for_tokens(&ability_tokens, false);
        let grant = grant_object_ability_for_anthem_subject(&clause, parsed, display);
        return Ok(Some(vec![anthem.into(), grant]));
    }
    if has_colon {
        return Err(CardTextError::ParseError(format!(
            "unsupported attached activated-ability grant (clause: '{}')",
            line_words.join(" ")
        )));
    }

    if ability_tokens.first().is_some_and(|token| {
        token.is_word("when") || token.is_word("whenever") || token.is_word("at")
    }) && let LineAst::Triggered {
        trigger,
        effects,
        max_triggers_per_turn,
    } = parse_triggered_line(&ability_tokens)?
    {
        let parsed = parsed_triggered_ability(
            trigger,
            effects,
            vec![Zone::Battlefield],
            Some(words(&ability_tokens).join(" ")),
            max_triggers_per_turn.map(crate::ConditionExpr::MaxTimesEachTurn),
            ReferenceImports::default(),
        );
        if parsed_triggered_ability_is_empty(&parsed) {
            return Err(CardTextError::ParseError(format!(
                "unsupported empty attached triggered grant clause (clause: '{}')",
                line_words.join(" ")
            )));
        }
        let text = words(&ability_tokens).join(" ");
        let grant = grant_object_ability_for_anthem_subject(&clause, parsed, text);
        return Ok(Some(vec![anthem.into(), grant]));
    }

    Err(CardTextError::ParseError(format!(
        "unsupported attached granted ability clause (clause: '{}')",
        line_words.join(" ")
    )))
}

pub(crate) fn parse_equipped_gets_and_has_activated_ability_line(
    tokens: &[Token],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let line_words = words(tokens);
    if line_words.len() < 4 || line_words[0] != "equipped" || line_words[1] != "creature" {
        return Ok(None);
    }

    let Some(has_idx) = tokens.iter().position(|token| token.is_word("has")) else {
        return Ok(None);
    };
    if has_idx + 1 >= tokens.len() {
        return Ok(None);
    }
    let ability_tokens = trim_edge_punctuation(&tokens[has_idx + 1..]);
    if ability_tokens.is_empty() {
        return Ok(None);
    }
    let has_colon = ability_tokens
        .iter()
        .any(|token| matches!(token, Token::Colon(_)));
    let Some(parsed) = parse_activated_line(&ability_tokens)? else {
        if has_colon {
            return Err(CardTextError::ParseError(format!(
                "unsupported equipped activated-ability grant (clause: '{}')",
                line_words.join(" ")
            )));
        }
        return Ok(None);
    };
    let mut static_abilities = Vec::new();
    if let Some(get_idx) = tokens
        .iter()
        .position(|token| token.is_word("get") || token.is_word("gets"))
        && get_idx < has_idx
    {
        let clause_tail_end = if has_idx > get_idx + 2
            && tokens
                .get(has_idx - 1)
                .is_some_and(|token| token.is_word("and"))
        {
            has_idx - 1
        } else {
            has_idx
        };
        let clause = parse_anthem_clause(tokens, get_idx, clause_tail_end)?;
        static_abilities.push(build_anthem_static_ability(&clause).into());
    }

    static_abilities.push(StaticAbilityAst::AttachedObjectAbilityGrant {
        ability: parsed,
        display: format!(
            "{} has {}",
            words(&tokens[..has_idx]).join(" "),
            display_text_for_tokens(&ability_tokens, true)
        ),
        condition: None,
    });

    Ok(Some(static_abilities))
}


pub(crate) fn parse_enchanted_has_activated_ability_line(
    tokens: &[Token],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let token_words = words(tokens);
    if !token_words.starts_with(&["enchanted"]) || !token_words.contains(&"has") {
        return Ok(None);
    }

    let Some(has_idx) = tokens.iter().position(|token| token.is_word("has")) else {
        return Ok(None);
    };
    let ability_tokens = trim_edge_punctuation(&tokens[has_idx + 1..]);
    if ability_tokens.is_empty() {
        return Ok(None);
    }
    let Some(parsed) = parse_activated_line(&ability_tokens)? else {
        return Ok(None);
    };

    Ok(Some(StaticAbilityAst::AttachedObjectAbilityGrant {
        ability: parsed,
        display: format!(
            "{} has {}",
            words(&tokens[..has_idx]).join(" "),
            display_text_for_tokens(&ability_tokens, true)
        ),
        condition: None,
    }))
}
