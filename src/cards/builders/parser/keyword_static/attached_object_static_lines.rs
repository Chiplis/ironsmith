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

fn token_words(tokens: &[OwnedLexToken]) -> Vec<&str> {
    crate::cards::builders::parser::lexer::token_word_refs(tokens)
}

fn str_ends_with_char(text: &str, suffix: char) -> bool {
    let mut chars = text.chars();
    chars.next_back().is_some_and(|ch| ch == suffix)
}

fn word_slice_starts_with(words: &[&str], prefix: &[&str]) -> bool {
    if prefix.len() > words.len() {
        return false;
    }
    for (idx, expected) in prefix.iter().enumerate() {
        if words[idx] != *expected {
            return false;
        }
    }
    true
}

fn word_slice_ends_with(words: &[&str], suffix: &[&str]) -> bool {
    if suffix.len() > words.len() {
        return false;
    }
    let start = words.len() - suffix.len();
    for (offset, expected) in suffix.iter().enumerate() {
        if words[start + offset] != *expected {
            return false;
        }
    }
    true
}

fn word_slice_contains(words: &[&str], expected: &str) -> bool {
    for word in words {
        if *word == expected {
            return true;
        }
    }
    false
}

fn word_slice_contains_sequence(words: &[&str], sequence: &[&str]) -> bool {
    if sequence.is_empty() {
        return true;
    }
    if sequence.len() > words.len() {
        return false;
    }
    for start in 0..=words.len() - sequence.len() {
        let mut matches = true;
        for (offset, expected) in sequence.iter().enumerate() {
            if words[start + offset] != *expected {
                matches = false;
                break;
            }
        }
        if matches {
            return true;
        }
    }
    false
}

fn find_word_index(words: &[&str], mut predicate: impl FnMut(&str) -> bool) -> Option<usize> {
    for (idx, word) in words.iter().enumerate() {
        if predicate(word) {
            return Some(idx);
        }
    }
    None
}

fn find_token_index(
    tokens: &[OwnedLexToken],
    mut predicate: impl FnMut(&OwnedLexToken) -> bool,
) -> Option<usize> {
    for (idx, token) in tokens.iter().enumerate() {
        if predicate(token) {
            return Some(idx);
        }
    }
    None
}

fn strip_suffix_char<'a>(word: &'a str, suffix: char) -> Option<&'a str> {
    let mut chars = word.char_indices();
    let Some((last_idx, last_char)) = chars.next_back() else {
        return None;
    };
    if last_char != suffix {
        return None;
    }
    Some(&word[..last_idx])
}

fn push_unique<T: PartialEq>(items: &mut Vec<T>, item: T) {
    for existing in items.iter() {
        if *existing == item {
            return;
        }
    }
    items.push(item);
}

pub(crate) fn display_text_for_tokens(
    tokens: &[OwnedLexToken],
    capitalize_effect_start: bool,
) -> String {
    let mut text = String::new();
    let mut needs_space = false;
    let mut in_effect_text = false;
    let mut capitalize_next_effect_word = false;
    let mut capitalize_next_cost_action = true;

    for token in tokens {
        if let Some(word) = token.as_word() {
            if needs_space && !text.is_empty() {
                text.push(' ');
            }
            let numeric_like = word
                .chars()
                .all(|ch| ch.is_ascii_digit() || matches!(ch, 'x' | 'X' | '+' | '-' | '/'));
            let mut rendered = match word {
                "t" => "{T}".to_string(),
                "q" => "{Q}".to_string(),
                _ if in_effect_text && numeric_like => word.to_string(),
                _ => crate::cards::builders::parser::util::parse_mana_symbol(word)
                    .map(|symbol| ManaCost::from_symbols(vec![symbol]).to_oracle())
                    .unwrap_or_else(|_| word.to_string()),
            };
            if !in_effect_text
                && capitalize_next_cost_action
                && matches!(
                    word,
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
        } else if matches!(
            token.kind,
            crate::cards::builders::parser::lexer::TokenKind::ManaGroup
        ) {
            let suppress_space = str_ends_with_char(text.as_str(), '}');
            if needs_space && !text.is_empty() && !suppress_space {
                text.push(' ');
            }
            text.push_str(token.slice.to_ascii_uppercase().as_str());
            needs_space = true;
            capitalize_next_cost_action = false;
        } else if token.is_colon() {
            text.push(':');
            needs_space = true;
            in_effect_text = true;
            capitalize_next_effect_word = capitalize_effect_start;
        } else if token.is_comma() {
            text.push(',');
            needs_space = true;
            if !in_effect_text {
                capitalize_next_cost_action = true;
            }
        } else if token.is_period() {
            text.push('.');
            needs_space = true;
            if in_effect_text {
                capitalize_next_effect_word = capitalize_effect_start;
            }
        } else if token.is_semicolon() {
            text.push(';');
            needs_space = true;
        }
    }

    text
}

fn parse_attached_granted_activated_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedAbility>, CardTextError> {
    let trimmed = trim_edge_punctuation(tokens);
    let rendered = display_text_for_tokens(&trimmed, false);
    if let Ok(reparsed) = crate::cards::builders::parser::lexer::lex_line(rendered.as_str(), 0) {
        if let Some(parsed) = parse_activated_line(&reparsed)? {
            return Ok(Some(parsed));
        }
    }
    parse_activated_line(&trimmed)
}

fn parse_nonstatic_keyword_action_as_object_ability(
    action: KeywordAction,
) -> Option<ParsedAbility> {
    match action {
        KeywordAction::Crew {
            amount,
            timing,
            additional_restrictions,
        } => {
            let cost = TotalCost::from_cost(crate::costs::Cost::effect(
                crate::effects::CrewCostEffect::new(amount),
            ));
            let animate = Effect::new(crate::effects::ApplyContinuousEffect::new(
                crate::continuous::EffectTarget::Source,
                crate::continuous::Modification::AddCardTypes(vec![CardType::Creature]),
                crate::effect::Until::EndOfTurn,
            ));
            Some(ParsedAbility {
                ability: Ability {
                    kind: AbilityKind::Activated(crate::ability::ActivatedAbility {
                        mana_cost: cost,
                        effects: crate::resolution::ResolutionProgram::from_effects(vec![animate]),
                        choices: Vec::new(),
                        timing,
                        additional_restrictions,
                        activation_restrictions: vec![],
                        mana_output: None,
                        activation_condition: None,
                        mana_usage_restrictions: vec![],
                    }),
                    functional_zones: vec![Zone::Battlefield],
                    text: Some(format!("Crew {amount}")),
                },
                effects_ast: None,
                reference_imports: ReferenceImports::default(),
                trigger_spec: None,
            })
        }
        _ => None,
    }
}

fn parse_attached_nonstatic_keyword_ability(
    tokens: &[OwnedLexToken],
) -> Result<Option<(ParsedAbility, String)>, CardTextError> {
    let ability_tokens = trim_edge_punctuation(tokens);
    if ability_tokens.is_empty() {
        return Ok(None);
    }

    let Some(actions) = parse_ability_line(&ability_tokens) else {
        return Ok(None);
    };
    if actions.len() != 1 {
        return Ok(None);
    }

    let action = actions.into_iter().next().expect("single action exists");
    let Some(parsed) = parse_nonstatic_keyword_action_as_object_ability(action.clone()) else {
        return Ok(None);
    };
    let display = match action {
        KeywordAction::Crew { amount, .. } => format!("Crew {amount}"),
        _ => return Ok(None),
    };
    Ok(Some((parsed, display)))
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
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let words = token_words(tokens);
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
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let line_words = token_words(tokens);
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
    let ability_head_words = token_words(&ability_tokens);
    if let Some(as_long_idx) = find_word_index(&ability_head_words, |word| word == "as")
        .filter(|idx| word_slice_starts_with(&ability_head_words[*idx..], &["as", "long", "as"]))
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

    let ability_words = token_words(&ability_tokens);
    if matches!(ability_words.as_slice(), ["landwalk", "of", "the", "chosen", "type"])
        || matches!(
            ability_words.as_slice(),
            ["snow", "landwalk", "of", "the", "chosen", "type"]
        )
    {
        let snow = ability_words.first().copied() == Some("snow");
        let display = if snow {
            format!("{subject} has snow landwalk of the chosen type")
        } else {
            format!("{subject} has landwalk of the chosen type")
        };
        return Ok(Some(vec![StaticAbilityAst::AttachedChosenLandwalkGrant {
            snow,
            display,
            condition,
        }]));
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
        let ability_text = format!(
            "{subject} has {}",
            action.display_text().to_ascii_lowercase()
        );
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
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let line_words = token_words(tokens);
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

    let Some(and_idx) = find_token_index(tokens, |token| token.is_word("and")) else {
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
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let normalized_storage = normalize_cant_words(tokens);
    let normalized = normalized_storage
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    if normalized.len() < 4 {
        return Ok(None);
    }

    let is_enchanted_creature = word_slice_starts_with(&normalized, &["enchanted", "creature"]);
    let is_enchanted_permanent =
        word_slice_starts_with(&normalized, &["enchanted", "permanent"]);
    let is_equipped_creature = word_slice_starts_with(&normalized, &["equipped", "creature"]);
    if !is_enchanted_creature && !is_enchanted_permanent && !is_equipped_creature {
        return Ok(None);
    }

    let subject_len = 2usize;
    let tail = &normalized[subject_len..];
    if !word_slice_starts_with(tail, &["cant"]) {
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

pub(crate) fn parse_attached_tap_abilities_cant_be_activated_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let normalized_storage = normalize_cant_words(tokens);
    let normalized = normalized_storage
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    let display = if normalized.as_slice()
        == [
            "enchanted",
            "creatures",
            "activated",
            "abilities",
            "with",
            "t",
            "in",
            "their",
            "costs",
            "cant",
            "be",
            "activated",
        ] {
        "enchanted creature's activated abilities with {T} in their costs can't be activated"
    } else if normalized.as_slice()
        == [
            "enchanted",
            "permanents",
            "activated",
            "abilities",
            "with",
            "t",
            "in",
            "their",
            "costs",
            "cant",
            "be",
            "activated",
        ]
    {
        "enchanted permanent's activated abilities with {T} in their costs can't be activated"
    } else if normalized.as_slice()
        == [
            "equipped",
            "creatures",
            "activated",
            "abilities",
            "with",
            "t",
            "in",
            "their",
            "costs",
            "cant",
            "be",
            "activated",
        ]
    {
        "equipped creature's activated abilities with {T} in their costs can't be activated"
    } else {
        return Ok(None);
    };

    Ok(Some(StaticAbilityAst::AttachedStaticAbilityGrant {
        ability: Box::new(StaticAbilityAst::Static(StaticAbility::restriction(
            crate::effect::Restriction::activate_tap_abilities_of(ObjectFilter::source()),
            display.to_string(),
        ))),
        display: display.to_string(),
        condition: None,
    }))
}

pub(crate) fn parse_you_control_attached_creature_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let line_words = token_words(tokens);
    if line_words.len() < 4 || !word_slice_starts_with(&line_words, &["you", "control"]) {
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
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let line_words = token_words(tokens);
    if line_words.len() < 6 {
        return Ok(None);
    }

    let Some(get_idx) = find_token_index(tokens, |token| {
        token.is_word("get") || token.is_word("gets")
    })
    else {
        return Ok(None);
    };
    let Some(and_idx) = find_token_index(tokens, |token| token.is_word("and")) else {
        return Ok(None);
    };
    if get_idx >= and_idx {
        return Ok(None);
    }

    let clause = parse_anthem_clause(tokens, get_idx, and_idx)?;
    let tail_tokens = trim_edge_punctuation(&tokens[and_idx + 1..]);
    let tail_words_storage = normalize_cant_words(&tail_tokens);
    let tail_words = tail_words_storage
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    let subject = if word_slice_contains_sequence(&line_words, &["enchanted", "permanent"]) {
        "enchanted permanent"
    } else if word_slice_contains_sequence(&line_words, &["enchanted", "creature"]) {
        "enchanted creature"
    } else if word_slice_contains_sequence(&line_words, &["equipped", "permanent"]) {
        "equipped permanent"
    } else if word_slice_contains_sequence(&line_words, &["equipped", "creature"]) {
        "equipped creature"
    } else {
        return Ok(None);
    };
    let anthem = build_anthem_static_ability(&clause);
    let granted = match tail_words.as_slice() {
        ["cant", "block"] => StaticAbilityAst::AttachedStaticAbilityGrant {
            ability: Box::new(StaticAbilityAst::Static(StaticAbility::cant_block())),
            display: format!("{subject} can't block"),
            condition: clause.condition.clone(),
        },
        ["cant", "attack"] => StaticAbilityAst::AttachedStaticAbilityGrant {
            ability: Box::new(StaticAbilityAst::Static(StaticAbility::cant_attack())),
            display: format!("{subject} can't attack"),
            condition: clause.condition.clone(),
        },
        ["cant", "attack", "or", "block"] => StaticAbilityAst::AttachedStaticAbilityGrant {
            ability: Box::new(StaticAbilityAst::Static(StaticAbility::restriction(
                crate::effect::Restriction::attack_or_block(ObjectFilter::source()),
                format!("{subject} can't attack or block"),
            ))),
            display: format!("{subject} can't attack or block"),
            condition: clause.condition.clone(),
        },
        ["cant", "be", "blocked"] => {
            return Ok(Some(vec![
                anthem.into(),
                grant_keyword_action_for_anthem_subject(&clause, KeywordAction::Unblockable),
            ]));
        }
        [lose, ..] if matches!(*lose, "lose" | "loses") => {
            let ability_tokens = trim_commas(&tail_tokens[1..]);
            if ability_tokens.is_empty() {
                return Ok(None);
            }
            let Some(actions) = parse_ability_line(&ability_tokens) else {
                return Ok(None);
            };
            reject_unimplemented_keyword_actions(&actions, &line_words.join(" "))?;
            let removed = actions
                .into_iter()
                .filter_map(|action| keyword_action_to_static_ability(action))
                .collect::<Vec<_>>();
            if removed.is_empty() {
                return Ok(None);
            }
            let mut out = vec![anthem.into()];
            for ability in removed {
                out.push(match &clause.subject {
                    AnthemSubjectAst::Source => match &clause.condition {
                        Some(condition) => StaticAbilityAst::ConditionalStaticAbility {
                            ability: Box::new(StaticAbilityAst::RemoveStaticAbility {
                                filter: ObjectFilter::source(),
                                ability: Box::new(StaticAbilityAst::Static(ability)),
                            }),
                            condition: condition.clone(),
                        },
                        None => StaticAbilityAst::RemoveStaticAbility {
                            filter: ObjectFilter::source(),
                            ability: Box::new(StaticAbilityAst::Static(ability)),
                        },
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
            return Ok(Some(out));
        }
        _ => return Ok(None),
    };
    Ok(Some(vec![anthem.into(), granted]))
}

pub(crate) fn parse_attached_type_transform_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let line_words = token_words(tokens);
    if line_words.len() < 4 {
        return Ok(None);
    }

    if !word_slice_starts_with(&line_words, &["enchanted", "creature"])
        && !word_slice_starts_with(&line_words, &["enchanted", "permanent"])
        && !word_slice_starts_with(&line_words, &["enchanted", "artifact"])
        && !word_slice_starts_with(&line_words, &["enchanted", "land"])
        && !word_slice_starts_with(&line_words, &["equipped", "creature"])
        && !word_slice_starts_with(&line_words, &["equipped", "permanent"])
    {
        return Ok(None);
    }

    let subject_token_end = token_index_for_word_index(tokens, 2).unwrap_or(tokens.len());
    let subject_tokens = trim_commas(&tokens[..subject_token_end]);
    let subject_text = token_words(&subject_tokens).join(" ");
    let filter = parse_object_filter(&subject_tokens, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported attached transform subject (clause: '{}')",
            line_words.join(" ")
        ))
    })?;

    let remainder = trim_commas(&tokens[subject_token_end..]);
    let remainder_words = token_words(&remainder);
    if !matches!(remainder_words.first().copied(), Some("is" | "are")) {
        return Ok(None);
    }

    let mut with_idx = None;
    let mut lose_idx = None;
    let mut in_quotes = false;
    for (idx, token) in remainder.iter().enumerate() {
        if token.is_quote() {
            in_quotes = !in_quotes;
            continue;
        }
        if in_quotes {
            continue;
        }
        if with_idx.is_none() && token.is_word("with") {
            with_idx = Some(idx);
            continue;
        }
        if token.is_word("lose") || token.is_word("loses") {
            lose_idx = Some(idx);
            break;
        }
    }

    let descriptor_end = with_idx.or(lose_idx).unwrap_or(remainder.len());
    let mut descriptor_tokens = trim_commas(&remainder[1..descriptor_end]).to_vec();
    while descriptor_tokens
        .first()
        .is_some_and(|token| token.is_word("a") || token.is_word("an"))
    {
        descriptor_tokens.remove(0);
    }
    let descriptor_words = token_words(&descriptor_tokens);
    if descriptor_words.is_empty() {
        return Ok(None);
    }

    let mut set_card_types = Vec::new();
    let mut add_subtypes = Vec::new();
    let mut set_colors = ColorSet::new();
    let mut make_colorless = false;
    for word in descriptor_words {
        if word == "and" {
            continue;
        }
        if word == "colorless" {
            make_colorless = true;
            continue;
        }
        if let Some(color) = parse_color(word) {
            set_colors = set_colors.union(color);
            continue;
        }
        if let Some(card_type) = parse_card_type(word) {
            push_unique(&mut set_card_types, card_type);
            continue;
        }
        if let Some(subtype) = parse_subtype_word(word)
            .or_else(|| strip_suffix_char(word, 's').and_then(parse_subtype_word))
        {
            push_unique(&mut add_subtypes, subtype);
            continue;
        }
        return Err(CardTextError::ParseError(format!(
            "unsupported attached transform descriptor '{}' (clause: '{}')",
            word,
            line_words.join(" ")
        )));
    }

    let mut out = Vec::new();
    if !set_card_types.is_empty() {
        out.push(StaticAbility::set_card_types(filter.clone(), set_card_types).into());
    }
    if !add_subtypes.is_empty() {
        out.push(StaticAbility::add_subtypes(filter.clone(), add_subtypes).into());
    }
    if !set_colors.is_empty() {
        out.push(StaticAbility::set_colors(filter.clone(), set_colors).into());
    }
    if make_colorless {
        out.push(StaticAbility::make_colorless(filter.clone()).into());
    }

    if let Some(with_idx) = with_idx {
        let ability_end = lose_idx.unwrap_or(remainder.len());
        let mut ability_tokens = trim_commas(&remainder[with_idx + 1..ability_end]).to_vec();
        while ability_tokens
            .last()
            .is_some_and(|token| token.is_word("and") || token.is_word("it"))
        {
            ability_tokens.pop();
        }
        let ability_tokens = trim_commas(&ability_tokens);
        if ability_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing attached transform granted ability (clause: '{}')",
                line_words.join(" ")
            )));
        }

        if let Some(parsed) = parse_attached_granted_activated_line(&ability_tokens)? {
            out.push(StaticAbilityAst::AttachedObjectAbilityGrant {
                ability: parsed,
                display: format!(
                    "{subject_text} has {}",
                    display_text_for_tokens(&ability_tokens, true)
                ),
                condition: None,
            });
        } else if let Some((parsed, display)) =
            parse_attached_nonstatic_keyword_ability(&ability_tokens)?
        {
            out.push(StaticAbilityAst::AttachedObjectAbilityGrant {
                ability: parsed,
                display: format!("{subject_text} has {display}"),
                condition: None,
            });
        } else {
            return Err(CardTextError::ParseError(format!(
                "unsupported attached transform granted ability (clause: '{}')",
                line_words.join(" ")
            )));
        }
    }

    if let Some(lose_idx) = lose_idx {
        let loss_tokens = trim_commas(&remainder[lose_idx..]);
        let loss_words = token_words(&loss_tokens);
        if matches!(
            loss_words.as_slice(),
            ["lose", "all", "other", "abilities"]
                | ["loses", "all", "other", "abilities"]
                | ["lose", "all", "other", "card", "types", "and", "abilities"]
                | ["loses", "all", "other", "card", "types", "and", "abilities"]
        ) {
            out.push(StaticAbility::remove_all_abilities(filter.clone()).into());
        } else if !matches!(
            loss_words.as_slice(),
            ["lose", "all", "other", "card", "types"]
                | ["loses", "all", "other", "card", "types"]
        ) {
            return Err(CardTextError::ParseError(format!(
                "unsupported attached transform loss clause (clause: '{}')",
                line_words.join(" ")
            )));
        }
    }

    if out.is_empty() {
        return Ok(None);
    }
    Ok(Some(out))
}

pub(crate) fn parse_prevent_damage_to_source_remove_counter_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let line_words = token_words(tokens);
    if line_words.len() < 12 {
        return Ok(None);
    }

    if !word_slice_starts_with(&line_words, &["if", "damage", "would", "be", "dealt", "to"]) {
        return Ok(None);
    }
    if !(word_slice_starts_with(&line_words[6..], &["this", "creature"])
        || word_slice_starts_with(&line_words[6..], &["this", "permanent"]))
    {
        return Ok(None);
    }
    if !word_slice_contains_sequence(&line_words, &["prevent", "that", "damage"]) {
        return Ok(None);
    }

    let Some(remove_word_idx) = find_word_index(&line_words, |word| word == "remove") else {
        return Ok(None);
    };
    let Some(counter_word_idx) = find_word_index(&line_words[remove_word_idx + 1..], |word| {
        matches!(word, "counter" | "counters")
    })
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
    let valid_tail = word_slice_starts_with(after_counter_words, &["from", "this", "creature"])
        || word_slice_starts_with(after_counter_words, &["from", "this", "permanent"])
        || word_slice_starts_with(after_counter_words, &["from", "it"]);
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

pub(crate) fn parse_prevent_damage_to_source_put_counters_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let line_words = token_words(tokens);
    if line_words.len() < 11 {
        return Ok(None);
    }

    let self_damage_prefix = ["if", "damage", "would", "be", "dealt", "to"];
    if word_slice_starts_with(&line_words, &self_damage_prefix) {
        let source_words_used = if word_slice_starts_with(&line_words[6..], &["this", "creature"])
            || word_slice_starts_with(&line_words[6..], &["this", "permanent"])
        {
            Some(2usize)
        } else if word_slice_starts_with(&line_words[6..], &["this"]) {
            Some(1usize)
        } else {
            None
        };
        if let Some(source_words_used) = source_words_used {
            let generic_tail = [
                "prevent",
                "that",
                "damage",
                "and",
                "put",
                "that",
                "many",
                "+1/+1",
                "counters",
                "on",
                "it",
            ];
            let generic_tail_alt = [
                "prevent",
                "that",
                "damage",
                "and",
                "put",
                "that",
                "many",
                "+1/+1",
                "counters",
                "on",
                "this",
                "creature",
            ];
            let generic_tail_pronoun = [
                "prevent",
                "that",
                "damage",
                "and",
                "put",
                "that",
                "many",
                "+1/+1",
                "counters",
                "on",
                "him",
            ];
            let instead_tail = [
                "put",
                "that",
                "many",
                "+1/+1",
                "counters",
                "on",
                "it",
                "instead",
            ];
            let instead_tail_alt = [
                "put",
                "that",
                "many",
                "+1/+1",
                "counters",
                "on",
                "this",
                "creature",
                "instead",
            ];

            let tail_word_start = 6 + source_words_used;
            let mut tail = &line_words[tail_word_start..];
            let mut condition = None;

            if tail.first().copied() == Some("while") {
                let Some(prevent_word_idx) = find_word_index(tail, |word| word == "prevent") else {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported conditional prevent-damage tail (clause: '{}')",
                        line_words.join(" ")
                    )));
                };
                if prevent_word_idx <= 1 {
                    return Err(CardTextError::ParseError(format!(
                        "missing condition in conditional prevent-damage line (clause: '{}')",
                        line_words.join(" ")
                    )));
                }

                let condition_start_word_idx = tail_word_start + 1;
                let condition_end_word_idx = tail_word_start + prevent_word_idx;
                let condition_start_token_idx =
                    token_index_for_word_index(tokens, condition_start_word_idx).ok_or_else(
                        || {
                            CardTextError::ParseError(format!(
                                "unable to map prevent-damage condition start (clause: '{}')",
                                line_words.join(" ")
                            ))
                        },
                    )?;
                let condition_end_token_idx =
                    token_index_for_word_index(tokens, condition_end_word_idx)
                        .unwrap_or(tokens.len());
                let condition_tokens =
                    trim_commas(&tokens[condition_start_token_idx..condition_end_token_idx]);
                condition = Some(parse_static_condition_clause(&condition_tokens)?);
                tail = &tail[prevent_word_idx..];
            }

            if tail == generic_tail
                || tail == generic_tail_alt
                || tail == generic_tail_pronoun
                || tail == instead_tail
                || tail == instead_tail_alt
            {
                let display = match condition {
                    Some(_) => {
                        let prefix = line_words[..tail_word_start].join(" ");
                        let effect = tail.join(" ");
                        let mut text = format!("{prefix}, {effect}");
                        if let Some(first) = text.get_mut(0..1) {
                            first.make_ascii_uppercase();
                        }
                        text
                    }
                    None => display_text_for_tokens(tokens, true),
                };
                let ability = StaticAbility::prevent_damage_to_self_put_counters_instead(
                    crate::object::CounterType::PlusOnePlusOne,
                    display,
                );
                let ast = StaticAbilityAst::Static(ability);
                return Ok(Some(match condition {
                    Some(condition) => StaticAbilityAst::ConditionalStaticAbility {
                        ability: Box::new(ast),
                        condition,
                    },
                    None => ast,
                }));
            }
        }
    }

    let noncombat_tail = [
        "prevent",
        "that",
        "damage",
        "put",
        "a",
        "+1/+1",
        "counter",
        "on",
        "this",
        "creature",
        "for",
        "each",
        "1",
        "damage",
        "prevented",
        "this",
        "way",
    ];
    if word_slice_starts_with(
        &line_words,
        &["if", "noncombat", "damage", "would", "be", "dealt", "to"],
    ) && word_slice_starts_with(&line_words[7..], &["this", "creature"])
        && line_words[9..] == noncombat_tail
    {
        return Ok(Some(StaticAbilityAst::Static(
            StaticAbility::prevent_constrained_damage_to_self_put_counters_instead(
                crate::object::CounterType::PlusOnePlusOne,
                display_text_for_tokens(tokens, true),
                None,
                Some(false),
            ),
        )));
    }

    let combat_creature_prefix = [
        "if",
        "a",
        "creature",
        "would",
        "deal",
        "combat",
        "damage",
        "to",
        "this",
        "creature",
    ];
    let combat_creature_tail = [
        "prevent",
        "that",
        "damage",
        "and",
        "put",
        "a",
        "+1/+1",
        "counter",
        "on",
        "this",
        "creature",
    ];
    if word_slice_starts_with(&line_words, &combat_creature_prefix)
        && line_words[10..] == combat_creature_tail
    {
        return Ok(Some(StaticAbilityAst::Static(
            StaticAbility::prevent_constrained_damage_to_self_put_counters_instead(
                crate::object::CounterType::PlusOnePlusOne,
                display_text_for_tokens(tokens, true),
                Some(ObjectFilter::creature()),
                Some(true),
            ),
        )));
    }

    Ok(None)
}

pub(crate) fn parse_attached_prevent_all_damage_dealt_by_attached_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let line_words = token_words(tokens);
    if line_words.len() < 6 {
        return Ok(None);
    }

    // "Prevent all damage that would be dealt by enchanted creature."
    if !word_slice_starts_with(&line_words, &["prevent", "all", "damage"]) {
        return Ok(None);
    }
    if !word_slice_ends_with(&line_words, &["by", "enchanted", "creature"]) {
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
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let line_words = token_words(tokens);
    if line_words.len() < 6 {
        return Ok(None);
    }

    let is_enchanted = word_slice_starts_with(&line_words, &["enchanted", "creature"]);
    let is_equipped = word_slice_starts_with(&line_words, &["equipped", "creature"]);
    if !is_enchanted && !is_equipped {
        return Ok(None);
    }

    let Some(has_idx) = find_token_index(tokens, |token| token.is_word("has")) else {
        return Ok(None);
    };
    if has_idx + 1 >= tokens.len() {
        return Ok(None);
    }

    let ability_tokens = trim_edge_punctuation(&tokens[has_idx + 1..]);
    if ability_tokens.is_empty() {
        return Ok(None);
    }

    let Some(and_idx) = find_token_index(&ability_tokens, |token| token.is_word("and")) else {
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
    let triggered =
        match crate::cards::builders::parser::clause_support::parse_triggered_line_lexed(
            &trigger_tokens,
        )? {
            LineAst::Triggered {
                trigger,
                effects,
                max_triggers_per_turn,
            } => parsed_triggered_ability(
                trigger,
                effects,
                vec![Zone::Battlefield],
                Some(token_words(&trigger_tokens).join(" ")),
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
    let subject_text = token_words(&tokens[..has_idx]).join(" ");
    let display = format!("{subject_text} has {}", token_words(&trigger_tokens).join(" "));
    static_abilities.push(StaticAbilityAst::AttachedObjectAbilityGrant {
        ability: triggered,
        display,
        condition: None,
    });

    Ok(Some(static_abilities))
}

pub(crate) fn parse_attached_is_legendary_gets_and_has_keywords_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let line_words = token_words(tokens);
    if line_words.len() < 10 {
        return Ok(None);
    }

    let is_enchanted = word_slice_starts_with(&line_words, &["enchanted", "creature"]);
    let is_equipped = word_slice_starts_with(&line_words, &["equipped", "creature"]);
    if !is_enchanted && !is_equipped {
        return Ok(None);
    }

    let Some(is_idx) = find_token_index(tokens, |token| token.is_word("is")) else {
        return Ok(None);
    };
    if is_idx < 2
        || !tokens
            .get(is_idx + 1)
            .is_some_and(|token| token.is_word("legendary"))
    {
        return Ok(None);
    }

    let Some(get_idx) = find_token_index(tokens, |token| {
        token.is_word("get") || token.is_word("gets")
    })
    else {
        return Ok(None);
    };
    let Some(has_idx) = find_token_index(tokens, |token| token.is_word("has")) else {
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

    let modifier_token = tokens.get(get_idx + 1).and_then(OwnedLexToken::as_word);
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
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let line_words = token_words(tokens);
    if line_words.len() < 6 {
        return Ok(None);
    }
    let is_enchanted = word_slice_starts_with(&line_words, &["enchanted", "creature"])
        || word_slice_starts_with(&line_words, &["enchanted", "permanent"]);
    let is_equipped = word_slice_starts_with(&line_words, &["equipped", "creature"])
        || word_slice_starts_with(&line_words, &["equipped", "permanent"]);
    if !is_enchanted && !is_equipped {
        return Ok(None);
    }

    let Some(get_idx) = find_token_index(tokens, |token| {
        token.is_word("get") || token.is_word("gets")
    })
    else {
        return Ok(None);
    };
    let Some(has_idx) = find_token_index(tokens, |token| token.is_word("has")) else {
        return Ok(None);
    };
    let Some(and_idx) = find_token_index(tokens, |token| token.is_word("and")) else {
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
        let granted_tokens_raw = &ability_tokens[and_idx + 1..];
        let granted_tokens = trim_edge_punctuation(granted_tokens_raw);
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

        if let Some(parsed) = parse_attached_granted_activated_line(granted_tokens_raw)? {
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

    let has_colon = ability_tokens.iter().any(|token| token.is_colon());
    if let Some(parsed) = parse_attached_granted_activated_line(&ability_tokens)? {
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
    } = crate::cards::builders::parser::clause_support::parse_triggered_line_lexed(
        &ability_tokens,
    )? {
        let parsed = parsed_triggered_ability(
            trigger,
            effects,
            vec![Zone::Battlefield],
            Some(token_words(&ability_tokens).join(" ")),
            max_triggers_per_turn.map(crate::ConditionExpr::MaxTimesEachTurn),
            ReferenceImports::default(),
        );
        if parsed_triggered_ability_is_empty(&parsed) {
            return Err(CardTextError::ParseError(format!(
                "unsupported empty attached triggered grant clause (clause: '{}')",
                line_words.join(" ")
            )));
        }
        let text = token_words(&ability_tokens).join(" ");
        let grant = grant_object_ability_for_anthem_subject(&clause, parsed, text);
        return Ok(Some(vec![anthem.into(), grant]));
    }

    Err(CardTextError::ParseError(format!(
        "unsupported attached granted ability clause (clause: '{}')",
        line_words.join(" ")
    )))
}

pub(crate) fn parse_equipped_gets_and_has_activated_ability_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let line_words = token_words(tokens);
    if line_words.len() < 4 || line_words[0] != "equipped" || line_words[1] != "creature" {
        return Ok(None);
    }

    let Some(has_idx) = find_token_index(tokens, |token| token.is_word("has")) else {
        return Ok(None);
    };
    if has_idx + 1 >= tokens.len() {
        return Ok(None);
    }
    let ability_tokens_raw = &tokens[has_idx + 1..];
    let ability_tokens = trim_edge_punctuation(ability_tokens_raw);
    if ability_tokens.is_empty() {
        return Ok(None);
    }
    let has_colon = ability_tokens.iter().any(|token| token.is_colon());
    let Some(parsed) = parse_attached_granted_activated_line(ability_tokens_raw)? else {
        if has_colon {
            return Err(CardTextError::ParseError(format!(
                "unsupported equipped activated-ability grant (clause: '{}')",
                line_words.join(" ")
            )));
        }
        return Ok(None);
    };
    let mut static_abilities = Vec::new();
    if let Some(get_idx) = find_token_index(tokens, |token| {
        token.is_word("get") || token.is_word("gets")
    })
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
            token_words(&tokens[..has_idx]).join(" "),
            display_text_for_tokens(&ability_tokens, true)
        ),
        condition: None,
    });

    Ok(Some(static_abilities))
}

pub(crate) fn parse_enchanted_has_activated_ability_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let line_words = token_words(tokens);
    if !word_slice_starts_with(&line_words, &["enchanted"])
        || !word_slice_contains(&line_words, "has")
    {
        return Ok(None);
    }

    let Some(has_idx) = find_token_index(tokens, |token| token.is_word("has")) else {
        return Ok(None);
    };
    let ability_tokens_raw = &tokens[has_idx + 1..];
    let ability_tokens = trim_edge_punctuation(ability_tokens_raw);
    if ability_tokens.is_empty() {
        return Ok(None);
    }
    let Some(parsed) = parse_attached_granted_activated_line(ability_tokens_raw)? else {
        return Ok(None);
    };

    Ok(Some(StaticAbilityAst::AttachedObjectAbilityGrant {
        ability: parsed,
        display: format!(
            "{} has {}",
            token_words(&tokens[..has_idx]).join(" "),
            display_text_for_tokens(&ability_tokens, true)
        ),
        condition: None,
    }))
}
