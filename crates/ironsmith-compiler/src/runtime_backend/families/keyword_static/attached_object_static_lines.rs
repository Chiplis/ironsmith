const ATTACHED_BASE_POWER_TOUGHNESS_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["base", "power", "and", "toughness"]);
const ATTACHED_PRESERVE_OTHER_TYPES_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["in", "addition", "to", "its", "other", "types"],
            &["in", "addition", "to", "their", "other", "types"],
        ]
);
const ATTACHED_EQUIPPED_CREATURE_HAS_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["equipped", "creature", "has"]);
const ATTACHED_EQUIPPED_CREATURE_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["equipped", "creature"]);
const ATTACHED_OBJECT_EXACT_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["enchanted", "creature"],
            &["enchanted", "permanent"],
            &["enchanted", "land"],
            &["enchanted", "artifact"],
            &["equipped", "creature"],
            &["equipped", "permanent"],
        ]
);
const ATTACHED_CREATURE_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["enchanted", "creature"], &["equipped", "creature"]]);
const ATTACHED_OBJECT_TYPE_TRANSFORM_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["enchanted", "creature"],
            &["enchanted", "permanent"],
            &["enchanted", "artifact"],
            &["enchanted", "land"],
            &["equipped", "creature"],
            &["equipped", "permanent"],
        ]
);
const ATTACHED_IS_OR_ARE_WORDS: &[&str] = &["is", "are"];
const ATTACHED_WITH_WORD: &str = "with";
const ATTACHED_AND_WORD: &str = "and";
const ATTACHED_REMOVE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["remove"]);
const ATTACHED_PREVENT_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["prevent"]);
const ATTACHED_COUNTER_OR_COUNTERS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["counter"], &["counters"]]);
const ATTACHED_COLORLESS_WORD: &str = "colorless";
const ATTACHED_ARTICLE_WORDS: &[&str] = &["a", "an"];
const ATTACHED_ENCHANTED_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["enchanted"]);
const ATTACHED_SNOW_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["snow"]);
const ATTACHED_WHILE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["while"]);
const ATTACHED_AND_OR_IT_WORDS: &[&str] = &["and", "it"];
const ATTACHED_LOSE_OR_LOSES_WORDS: &[&str] = &["lose", "loses"];
const ATTACHED_GET_OR_GETS_WORDS: &[&str] = &["get", "gets"];
const ATTACHED_WHEN_OR_WHENEVER_WORDS: &[&str] = &["when", "whenever"];
const ATTACHED_HAS_WORD: &str = "has";
const ATTACHED_IS_WORD: &str = "is";
const ATTACHED_YOU_CONTROL_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["you", "control"]);
const ATTACHED_EQUIPPED_SUBJECT_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["equipped", "creature"], &["equipped", "permanent"]]);
const ATTACHED_LEGENDARY_WORD: &str = "legendary";
const ATTACHED_ENCHANTED_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["enchanted"]);
const ATTACHED_CANT_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["cant"]);
const ATTACHED_CANT_ATTACK_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["cant", "attack"]);
const ATTACHED_CANT_BLOCK_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["cant", "block"]);
const ATTACHED_CANT_ATTACK_OR_BLOCK_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["cant", "attack", "or", "block"]);
const ATTACHED_CANT_BE_BLOCKED_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["cant", "be", "blocked"]);
const ATTACHED_AS_LONG_AS_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["as", "long", "as"]);
const ATTACHED_ENCHANTED_SUBJECT_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["enchanted", "creature"], &["enchanted", "permanent"]]);
const ATTACHED_ENCHANTED_CREATURE_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["enchanted", "creature"]);
const ATTACHED_ENCHANTED_PERMANENT_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["enchanted", "permanent"]);
const ATTACHED_ENCHANTED_CREATURE_CONTAINS_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["enchanted", "creature"]]);
const ATTACHED_ENCHANTED_PERMANENT_CONTAINS_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["enchanted", "permanent"]]);
const ATTACHED_EQUIPPED_CREATURE_CONTAINS_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["equipped", "creature"]]);
const ATTACHED_EQUIPPED_PERMANENT_CONTAINS_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["equipped", "permanent"]]);
const ATTACHED_ALL_CREATURES_ABLE_TO_BLOCK_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["all", "creatures", "able", "to", "block"]);
const ATTACHED_DO_SO_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(suffix & ["do", "so"]);
const ATTACHED_IF_DAMAGE_TO_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["if", "damage", "would", "be", "dealt", "to"]);
const ATTACHED_THIS_SOURCE_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["this", "creature"], &["this", "permanent"]]);
const ATTACHED_THIS_CREATURE_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["this", "creature"]);
const ATTACHED_THIS_WORD_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["this"]);
const ATTACHED_PREVENT_THAT_DAMAGE_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["prevent", "that", "damage"]]);
const ATTACHED_REMOVE_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["from", "this", "creature"],
            &["from", "this", "permanent"],
            &["from", "it"],
        ]
);
const ATTACHED_IF_NONCOMBAT_DAMAGE_TO_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["if", "noncombat", "damage", "would", "be", "dealt", "to"]);
const ATTACHED_NONCOMBAT_COUNTER_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    exact
        & [
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
        ]
);
const ATTACHED_IF_CREATURE_COMBAT_DAMAGE_TO_THIS_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix
        & [
            "if", "a", "creature", "would", "deal", "combat", "damage", "to", "this", "creature",
        ]
);
const ATTACHED_COMBAT_COUNTER_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    exact
        & [
            "prevent", "that", "damage", "and", "put", "a", "+1/+1", "counter", "on", "this",
            "creature",
        ]
);
const ATTACHED_PREVENT_ALL_DAMAGE_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["prevent", "all", "damage"]);
const ATTACHED_PREVENT_ALL_COMBAT_DAMAGE_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["prevent", "all", "combat", "damage"]);
const ATTACHED_BY_ENCHANTED_CREATURE_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(suffix & ["by", "enchanted", "creature"]);
const ATTACHED_PREVENT_ALL_DAMAGE_TO_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix
        & [
            "prevent", "all", "damage", "that", "would", "be", "dealt", "to"
        ]
);
const ATTACHED_TO_ENCHANTED_CREATURE_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(suffix & ["to", "enchanted", "creature"]);

fn attached_shape_matches_words<'a>(words: &[&str], shape: ClauseShape<'a>) -> bool {
    shape.matches_word_slice(words)
}

fn attached_word_is_any(word: &str, expected: &[&str]) -> bool {
    expected.contains(&word)
}

fn attached_word_is(word: &str, expected: &str) -> bool {
    attached_word_is_any(word, &[expected])
}

fn attached_word_at_is(words: &[&str], idx: usize, expected: &str) -> bool {
    words
        .get(idx)
        .is_some_and(|word| attached_word_is(word, expected))
}

fn attached_token_word_is_any(token: &OwnedLexToken, expected: &[&str]) -> bool {
    token
        .as_word()
        .is_some_and(|word| attached_word_is_any(word, expected))
}

fn attached_token_word_is(token: &OwnedLexToken, expected: &str) -> bool {
    attached_token_word_is_any(token, &[expected])
}

fn attached_find_prefix_shape_start(words: &[&str], shape: &ClauseShape<'static>) -> Option<usize> {
    (0..words.len()).find(|&idx| attached_shape_matches_words(&words[idx..], *shape))
}

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
            presentation_label: None,
        }),
        functional_zones: vec![Zone::Battlefield],
    }
}

fn parse_attached_with_base_power_toughness_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<(i32, i32, bool)>, CardTextError> {
    let words = crate::runtime_backend::lexer::token_word_refs(tokens);
    if words.len() < 5
        || !attached_shape_matches_words(&words, ATTACHED_BASE_POWER_TOUGHNESS_PREFIX_PATTERN)
    {
        return Ok(None);
    }

    let (power, toughness) = parse_pt_modifier(words[4]).map_err(|_| {
        CardTextError::ParseError(format!(
            "invalid attached transform base power/toughness value (clause: '{}')",
            words.join(" ")
        ))
    })?;

    let trailing = &words[5..];
    let preserve_other_types =
        attached_shape_matches_words(trailing, ATTACHED_PRESERVE_OTHER_TYPES_TAIL_PATTERN);
    if !trailing.is_empty() && !preserve_other_types {
        return Ok(None);
    }

    Ok(Some((power, toughness, preserve_other_types)))
}

fn parse_attached_transform_loss_removes_all_abilities(tokens: &[OwnedLexToken]) -> bool {
    let loss_words = crate::runtime_backend::lexer::token_word_refs(tokens);
    matches!(
        loss_words.as_slice(),
        ["lose", "all", "other", "abilities"]
            | ["loses", "all", "other", "abilities"]
            | ["lose", "all", "other", "card", "types", "and", "abilities"]
            | ["loses", "all", "other", "card", "types", "and", "abilities"]
    )
}

fn split_attached_transform_base_pt_and_keyword_grant(
    tokens: &[OwnedLexToken],
) -> Option<(Vec<OwnedLexToken>, Vec<OwnedLexToken>)> {
    for idx in 1..tokens.len() {
        let Some(word) = tokens[idx].as_word() else {
            continue;
        };
        if !matches!(word, "has" | "have") {
            continue;
        }

        let prev_word = tokens[..idx]
            .iter()
            .rev()
            .find_map(OwnedLexToken::as_word)?;
        if !matches!(prev_word, "it" | "they" | "and") {
            continue;
        }

        let prev_idx = tokens[..idx]
            .iter()
            .rposition(|token| token.as_word().is_some())?;
        let base_end = if matches!(prev_word, "it" | "they" | "and") {
            prev_idx
        } else {
            idx
        };
        let base_tokens = trim_edge_punctuation(&tokens[..base_end]);
        let keyword_tokens = trim_edge_punctuation(&tokens[idx + 1..]);
        if base_tokens.is_empty() || keyword_tokens.is_empty() {
            continue;
        }

        return Some((base_tokens, keyword_tokens));
    }

    None
}

pub(crate) fn display_text_for_tokens(
    tokens: &[OwnedLexToken],
    capitalize_effect_start: bool,
) -> String {
    let mut text = String::new();
    let mut needs_space = false;
    let mut in_effect_text = false;
    let mut in_loyalty_cost = false;
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
                _ if in_loyalty_cost || (in_effect_text && numeric_like) => word.to_string(),
                _ => crate::runtime_backend::util::parse_mana_symbol(word)
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
            crate::runtime_backend::lexer::TokenKind::ManaGroup
        ) {
            let suppress_space =
                crate::runtime_backend::token_primitives::str_ends_with_char(text.as_str(), '}');
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
            in_loyalty_cost = false;
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
        } else if token.kind == crate::runtime_backend::lexer::TokenKind::LBracket {
            if needs_space && !text.is_empty() {
                text.push(' ');
            }
            text.push('[');
            needs_space = false;
            in_loyalty_cost = true;
        } else if token.kind == crate::runtime_backend::lexer::TokenKind::RBracket {
            text.push(']');
            needs_space = false;
            in_loyalty_cost = false;
        } else if in_loyalty_cost && token.kind == crate::runtime_backend::lexer::TokenKind::Plus {
            text.push('+');
            needs_space = false;
        } else if in_loyalty_cost && token.kind == crate::runtime_backend::lexer::TokenKind::Dash {
            text.push('-');
            needs_space = false;
        }
    }

    text
}

fn parse_attached_granted_activated_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedAbility>, CardTextError> {
    let trimmed = trim_edge_punctuation(tokens);
    let rendered = display_text_for_tokens(&trimmed, false);
    if let Ok(reparsed) = crate::runtime_backend::lexer::lex_line(rendered.as_str(), 0) {
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
                        is_loyalty_ability: false,
                    }),
                    functional_zones: vec![Zone::Battlefield],
                }
                .into(),
                text: Some(format!("Crew {amount}")),
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

pub(crate) fn cumulative_upkeep_granted_ability(total_cost: TotalCost) -> Ability {
    let payment_effects = crate::costs::total_cost_to_payment_effects(&total_cost);

    Ability {
        kind: AbilityKind::Triggered(TriggeredAbility {
            trigger: Trigger::beginning_of_upkeep(PlayerFilter::You),
            effects: crate::resolution::ResolutionProgram::from_effects(vec![
                Effect::put_counters_on_source(CounterType::Age, 1),
                Effect::cumulative_upkeep(
                    payment_effects,
                    PlayerFilter::You,
                    vec![Effect::sacrifice_source()],
                ),
            ]),
            choices: vec![],
            intervening_if: None,
            presentation_label: None,
        }),
        functional_zones: vec![Zone::Battlefield],
    }
}

pub(crate) fn parse_equipped_creature_has_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let words = crate::runtime_backend::lexer::token_word_refs(tokens);
    let clause_text = words.join(" ");
    if words.len() < 4
        || !attached_shape_matches_words(&words, ATTACHED_EQUIPPED_CREATURE_HAS_PREFIX_PATTERN)
    {
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
        if let KeywordAction::CumulativeUpkeep { total_cost, text } = action {
            extra_grants.push(StaticAbilityAst::AttachedObjectAbilityGrant {
                ability: parsed_ability_from_ability(cumulative_upkeep_granted_ability(total_cost)),
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
    let line_words = crate::runtime_backend::lexer::parser_token_word_refs(tokens);
    let clause_text = line_words.join(" ");
    if line_words.len() < 4 || !ATTACHED_ENCHANTED_WORD_PATTERN.matches_first_word(&line_words) {
        return Ok(None);
    }
    let subject = match line_words.get(1).copied() {
        Some("creature") => "enchanted creature",
        Some("permanent") => "enchanted permanent",
        _ => return Ok(None),
    };
    if !line_words
        .get(2)
        .is_some_and(|word| attached_word_is(word, ATTACHED_HAS_WORD))
    {
        return Ok(None);
    }

    let mut ability_tokens = trim_edge_punctuation(&tokens[3..]);
    if ability_tokens.is_empty() {
        return Ok(None);
    }

    let mut condition: Option<crate::ConditionExpr> = None;
    let ability_head_words = crate::runtime_backend::lexer::token_word_refs(&ability_tokens);
    if let Some(as_long_idx) =
        attached_find_prefix_shape_start(&ability_head_words, &ATTACHED_AS_LONG_AS_PREFIX_PATTERN)
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

    let ability_words = crate::runtime_backend::lexer::token_word_refs(&ability_tokens);
    if matches!(
        ability_words.as_slice(),
        ["landwalk", "of", "the", "chosen", "type"]
    ) || matches!(
        ability_words.as_slice(),
        ["snow", "landwalk", "of", "the", "chosen", "type"]
    ) {
        let snow = ATTACHED_SNOW_WORD_PATTERN.matches_first_word(&ability_words);
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
        if let KeywordAction::CumulativeUpkeep { total_cost, text } = action {
            let ability_text = format!("{subject} has {}", text.to_ascii_lowercase());
            out.push(StaticAbilityAst::AttachedObjectAbilityGrant {
                ability: parsed_ability_from_ability(cumulative_upkeep_granted_ability(total_cost)),
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
    let line_words = crate::runtime_backend::lexer::token_word_refs(tokens);
    if line_words.len() < 7 {
        return Ok(None);
    }

    let is_enchanted =
        attached_shape_matches_words(&line_words, ATTACHED_ENCHANTED_SUBJECT_PREFIX_PATTERN);
    let is_equipped =
        attached_shape_matches_words(&line_words, ATTACHED_EQUIPPED_CREATURE_PREFIX_PATTERN);
    if !is_enchanted && !is_equipped {
        return Ok(None);
    }
    if !attached_word_at_is(&line_words, 2, ATTACHED_HAS_WORD) {
        return Ok(None);
    }

    let Some(and_idx) =
        crate::runtime_backend::grammar::primitives::find_token_index(tokens, |token| {
            attached_token_word_is(token, ATTACHED_AND_WORD)
        })
    else {
        return Ok(None);
    };
    if and_idx <= 3
        || !tokens
            .get(and_idx + 1)
            .is_some_and(|token| attached_token_word_is_any(token, ATTACHED_LOSE_OR_LOSES_WORDS))
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

    let is_enchanted_creature =
        attached_shape_matches_words(&normalized, ATTACHED_ENCHANTED_CREATURE_PREFIX_PATTERN);
    let is_enchanted_permanent =
        attached_shape_matches_words(&normalized, ATTACHED_ENCHANTED_PERMANENT_PREFIX_PATTERN);
    let is_equipped_creature =
        attached_shape_matches_words(&normalized, ATTACHED_EQUIPPED_CREATURE_PREFIX_PATTERN);
    if !is_enchanted_creature && !is_enchanted_permanent && !is_equipped_creature {
        return Ok(None);
    }

    let subject_len = 2usize;
    let tail = &normalized[subject_len..];
    if !attached_shape_matches_words(tail, ATTACHED_CANT_PREFIX_PATTERN) {
        return Ok(None);
    }

    let subject = if is_equipped_creature {
        "equipped creature"
    } else if is_enchanted_permanent {
        "enchanted permanent"
    } else {
        "enchanted creature"
    };

    let (restriction, display) = if attached_shape_matches_words(tail, ATTACHED_CANT_ATTACK_PATTERN)
    {
        (
            crate::effect::Restriction::attack(ObjectFilter::source()),
            format!("{subject} can't attack"),
        )
    } else if attached_shape_matches_words(tail, ATTACHED_CANT_BLOCK_PATTERN) {
        (
            crate::effect::Restriction::block(ObjectFilter::source()),
            format!("{subject} can't block"),
        )
    } else if attached_shape_matches_words(tail, ATTACHED_CANT_ATTACK_OR_BLOCK_PATTERN) {
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

pub(crate) fn parse_attached_all_creatures_able_to_block_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let normalized_storage = normalize_cant_words(tokens);
    let normalized = normalized_storage
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    if normalized.len() < 9 {
        return Ok(None);
    }

    if !attached_shape_matches_words(
        &normalized,
        ATTACHED_ALL_CREATURES_ABLE_TO_BLOCK_PREFIX_PATTERN,
    ) {
        return Ok(None);
    }
    if !attached_shape_matches_words(&normalized, ATTACHED_DO_SO_TAIL_PATTERN) {
        return Ok(None);
    }

    let subject = match &normalized[5..normalized.len() - 2] {
        ["enchanted", "creature"] => "enchanted creature",
        ["enchanted", "permanent"] => "enchanted permanent",
        ["equipped", "creature"] => "equipped creature",
        ["equipped", "permanent"] => "equipped permanent",
        _ => return Ok(None),
    };

    let display = format!("All creatures able to block {subject} do so");
    Ok(Some(StaticAbilityAst::AttachedStaticAbilityGrant {
        ability: Box::new(StaticAbilityAst::Static(StaticAbility::restriction(
            crate::effect::Restriction::must_block_specific_attacker(
                ObjectFilter::creature(),
                ObjectFilter::source(),
            ),
            display.clone(),
        ))),
        display,
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
    let line_words = crate::runtime_backend::lexer::token_word_refs(tokens);
    if line_words.len() < 4
        || !attached_shape_matches_words(&line_words, ATTACHED_YOU_CONTROL_PREFIX_PATTERN)
    {
        return Ok(None);
    }

    let tail = &line_words[2..];
    let is_attached_subject = attached_shape_matches_words(tail, ATTACHED_OBJECT_EXACT_PATTERN);
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
    let line_words = crate::runtime_backend::lexer::token_word_refs(tokens);
    if line_words.len() < 6 {
        return Ok(None);
    }

    let Some(get_idx) =
        crate::runtime_backend::grammar::primitives::find_token_index(tokens, |token| {
            attached_token_word_is_any(token, ATTACHED_GET_OR_GETS_WORDS)
        })
    else {
        return Ok(None);
    };
    let Some(and_idx) =
        crate::runtime_backend::grammar::primitives::find_token_index(tokens, |token| {
            attached_token_word_is(token, ATTACHED_AND_WORD)
        })
    else {
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
    let subject =
        if attached_shape_matches_words(&line_words, ATTACHED_ENCHANTED_PERMANENT_CONTAINS_PATTERN)
        {
            "enchanted permanent"
        } else if attached_shape_matches_words(
            &line_words,
            ATTACHED_ENCHANTED_CREATURE_CONTAINS_PATTERN,
        ) {
            "enchanted creature"
        } else if attached_shape_matches_words(
            &line_words,
            ATTACHED_EQUIPPED_PERMANENT_CONTAINS_PATTERN,
        ) {
            "equipped permanent"
        } else if attached_shape_matches_words(
            &line_words,
            ATTACHED_EQUIPPED_CREATURE_CONTAINS_PATTERN,
        ) {
            "equipped creature"
        } else {
            return Ok(None);
        };
    let anthem = build_anthem_static_ability(&clause);
    let granted = if attached_shape_matches_words(&tail_words, ATTACHED_CANT_BLOCK_PATTERN) {
        StaticAbilityAst::AttachedStaticAbilityGrant {
            ability: Box::new(StaticAbilityAst::Static(StaticAbility::cant_block())),
            display: format!("{subject} can't block"),
            condition: clause.condition.clone(),
        }
    } else if attached_shape_matches_words(&tail_words, ATTACHED_CANT_ATTACK_PATTERN) {
        StaticAbilityAst::AttachedStaticAbilityGrant {
            ability: Box::new(StaticAbilityAst::Static(StaticAbility::cant_attack())),
            display: format!("{subject} can't attack"),
            condition: clause.condition.clone(),
        }
    } else if attached_shape_matches_words(&tail_words, ATTACHED_CANT_ATTACK_OR_BLOCK_PATTERN) {
        StaticAbilityAst::AttachedStaticAbilityGrant {
            ability: Box::new(StaticAbilityAst::Static(StaticAbility::restriction(
                crate::effect::Restriction::attack_or_block(ObjectFilter::source()),
                format!("{subject} can't attack or block"),
            ))),
            display: format!("{subject} can't attack or block"),
            condition: clause.condition.clone(),
        }
    } else if attached_shape_matches_words(&tail_words, ATTACHED_CANT_BE_BLOCKED_PATTERN) {
        return Ok(Some(vec![
            anthem.into(),
            grant_keyword_action_for_anthem_subject(&clause, KeywordAction::Unblockable),
        ]));
    } else if tail_words
        .first()
        .is_some_and(|word| attached_word_is_any(word, ATTACHED_LOSE_OR_LOSES_WORDS))
    {
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
    } else {
        return Ok(None);
    };
    Ok(Some(vec![anthem.into(), granted]))
}

pub(crate) fn parse_attached_type_transform_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let line_words = crate::runtime_backend::lexer::token_word_refs(tokens);
    if line_words.len() < 4 {
        return Ok(None);
    }

    if !attached_shape_matches_words(&line_words, ATTACHED_OBJECT_TYPE_TRANSFORM_PREFIX_PATTERN) {
        return Ok(None);
    }

    let clause = LexedClause::new(tokens);
    let subject_tokens = trim_commas(
        clause
            .before_word(2)
            .unwrap_or_else(|| clause.before(tokens.len()))
            .tokens(),
    );
    let subject_text = crate::runtime_backend::lexer::token_word_refs(&subject_tokens).join(" ");
    let filter = parse_object_filter(&subject_tokens, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported attached transform subject (clause: '{}')",
            line_words.join(" ")
        ))
    })?;

    let remainder = trim_commas(
        clause
            .from_word(2)
            .unwrap_or_else(|| clause.from(tokens.len()))
            .tokens(),
    );
    let remainder_words = crate::runtime_backend::lexer::token_word_refs(&remainder);
    if !remainder_words
        .first()
        .is_some_and(|word| attached_word_is_any(word, ATTACHED_IS_OR_ARE_WORDS))
    {
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
        if with_idx.is_none() && attached_token_word_is(token, ATTACHED_WITH_WORD) {
            with_idx = Some(idx);
            continue;
        }
        if attached_token_word_is_any(token, ATTACHED_LOSE_OR_LOSES_WORDS) {
            lose_idx = Some(idx);
            break;
        }
    }

    let descriptor_end = with_idx.or(lose_idx).unwrap_or(remainder.len());
    let mut descriptor_tokens = trim_commas(&remainder[1..descriptor_end]).to_vec();
    while descriptor_tokens
        .first()
        .is_some_and(|token| attached_token_word_is_any(token, ATTACHED_ARTICLE_WORDS))
    {
        descriptor_tokens.remove(0);
    }
    let descriptor_words = crate::runtime_backend::lexer::token_word_refs(&descriptor_tokens);
    if descriptor_words.is_empty() {
        return Ok(None);
    }

    let mut set_card_types = Vec::new();
    let mut add_subtypes = Vec::new();
    let mut set_colors = ColorSet::new();
    let mut make_colorless = false;
    for word in descriptor_words {
        if attached_word_is(word, ATTACHED_AND_WORD) {
            continue;
        }
        if attached_word_is(word, ATTACHED_COLORLESS_WORD) {
            make_colorless = true;
            continue;
        }
        if let Some(color) = parse_color(word) {
            set_colors = set_colors.union(color);
            continue;
        }
        if let Some(card_type) = parse_card_type(word) {
            crate::slice_primitives::push_unique(&mut set_card_types, card_type);
            continue;
        }
        if let Some(subtype) = parse_subtype_word(word).or_else(|| {
            crate::string_primitives::strip_suffix_char(word, 's').and_then(parse_subtype_word)
        }) {
            crate::slice_primitives::push_unique(&mut add_subtypes, subtype);
            continue;
        }
        return Err(CardTextError::ParseError(format!(
            "unsupported attached transform descriptor '{}' (clause: '{}')",
            word,
            line_words.join(" ")
        )));
    }

    let mut out = Vec::new();
    let mut preserve_other_types = false;
    let mut loss_consumed = false;

    if let Some(with_idx) = with_idx {
        let ability_end = lose_idx.unwrap_or(remainder.len());
        let mut ability_tokens = trim_commas(&remainder[with_idx + 1..ability_end]).to_vec();
        while ability_tokens
            .last()
            .is_some_and(|token| attached_token_word_is_any(token, ATTACHED_AND_OR_IT_WORDS))
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

        if let Some((base_tokens, keyword_tokens)) =
            split_attached_transform_base_pt_and_keyword_grant(&ability_tokens)
        {
            let Some((power, toughness, with_preserve_other_types)) =
                parse_attached_with_base_power_toughness_clause(&base_tokens)?
            else {
                return Err(CardTextError::ParseError(format!(
                    "unsupported attached transform granted ability (clause: '{}')",
                    line_words.join(" ")
                )));
            };
            preserve_other_types = with_preserve_other_types;
            out.push(
                StaticAbility::set_base_power_toughness(filter.clone(), power, toughness).into(),
            );

            if let Some(lose_idx) = lose_idx {
                let loss_tokens = trim_commas(&remainder[lose_idx..]);
                if parse_attached_transform_loss_removes_all_abilities(&loss_tokens) {
                    out.push(StaticAbility::remove_all_abilities(filter.clone()).into());
                    loss_consumed = true;
                }
            }

            let Some(actions) = parse_ability_line(&keyword_tokens) else {
                return Err(CardTextError::ParseError(format!(
                    "unsupported attached transform granted ability (clause: '{}')",
                    line_words.join(" ")
                )));
            };
            for action in actions {
                reject_unimplemented_keyword_actions(
                    std::slice::from_ref(&action),
                    &line_words.join(" "),
                )?;
                if !action.lowers_to_static_ability() {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported attached transform granted ability (clause: '{}')",
                        line_words.join(" ")
                    )));
                }
                out.push(StaticAbilityAst::AttachedKeywordActionGrant {
                    display: format!(
                        "{subject_text} has {}",
                        action.display_text().to_ascii_lowercase()
                    ),
                    action,
                    condition: None,
                });
            }
        } else if let Some((power, toughness, with_preserve_other_types)) =
            parse_attached_with_base_power_toughness_clause(&ability_tokens)?
        {
            preserve_other_types = with_preserve_other_types;
            out.push(
                StaticAbility::set_base_power_toughness(filter.clone(), power, toughness).into(),
            );
        } else if let Some(parsed) = parse_attached_granted_activated_line(&ability_tokens)? {
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

    let descriptor_has_card_types = !set_card_types.is_empty();
    if descriptor_has_card_types {
        if preserve_other_types {
            out.push(StaticAbility::add_card_types(filter.clone(), set_card_types).into());
        } else {
            out.push(StaticAbility::set_card_types(filter.clone(), set_card_types).into());
        }
    }
    if !add_subtypes.is_empty() {
        if !preserve_other_types
            && !descriptor_has_card_types
            && add_subtypes
                .iter()
                .all(crate::types::Subtype::is_creature_type)
            && attached_shape_matches_words(&line_words, ATTACHED_ENCHANTED_CREATURE_PREFIX_PATTERN)
        {
            out.push(StaticAbility::set_creature_subtypes(filter.clone(), add_subtypes).into());
        } else {
            out.push(StaticAbility::add_subtypes(filter.clone(), add_subtypes).into());
        }
    }
    if !set_colors.is_empty() {
        out.push(StaticAbility::set_colors(filter.clone(), set_colors).into());
    }
    if make_colorless {
        out.push(StaticAbility::make_colorless(filter.clone()).into());
    }

    if let Some(lose_idx) = lose_idx
        && !loss_consumed
    {
        let loss_tokens = trim_commas(&remainder[lose_idx..]);
        let loss_words = crate::runtime_backend::lexer::token_word_refs(&loss_tokens);
        if parse_attached_transform_loss_removes_all_abilities(&loss_tokens) {
            out.push(StaticAbility::remove_all_abilities(filter.clone()).into());
        } else if !matches!(
            loss_words.as_slice(),
            ["lose", "all", "other", "card", "types"] | ["loses", "all", "other", "card", "types"]
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
    let line_words = crate::runtime_backend::lexer::token_word_refs(tokens);
    if line_words.len() < 12 {
        return Ok(None);
    }

    if !attached_shape_matches_words(&line_words, ATTACHED_IF_DAMAGE_TO_PREFIX_PATTERN) {
        return Ok(None);
    }
    if !attached_shape_matches_words(&line_words[6..], ATTACHED_THIS_SOURCE_PREFIX_PATTERN) {
        return Ok(None);
    }
    if !attached_shape_matches_words(&line_words, ATTACHED_PREVENT_THAT_DAMAGE_PATTERN) {
        return Ok(None);
    }

    let Some(remove_word_idx) = ATTACHED_REMOVE_WORD_PATTERN.find_word(&line_words) else {
        return Ok(None);
    };
    let Some(counter_word_idx) = ATTACHED_COUNTER_OR_COUNTERS_WORD_PATTERN
        .find_word(&line_words[remove_word_idx + 1..])
        .map(|idx| remove_word_idx + 1 + idx)
    else {
        return Ok(None);
    };

    let Some(descriptor_clause) =
        LexedClause::new(tokens).between_word_range(remove_word_idx + 1, counter_word_idx + 1)
    else {
        return Err(CardTextError::ParseError(format!(
            "unable to map counter descriptor in prevent-damage line (clause: '{}')",
            line_words.join(" ")
        )));
    };

    let mut descriptor_tokens = trim_commas(descriptor_clause.tokens());
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
        .is_some_and(|token| attached_token_word_is_any(token, ATTACHED_ARTICLE_WORDS))
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
    let valid_tail =
        attached_shape_matches_words(after_counter_words, ATTACHED_REMOVE_TAIL_PATTERN);
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
    let line_words = crate::runtime_backend::lexer::token_word_refs(tokens);
    if line_words.len() < 11 {
        return Ok(None);
    }

    if attached_shape_matches_words(&line_words, ATTACHED_IF_DAMAGE_TO_PREFIX_PATTERN) {
        let source_words_used = if attached_shape_matches_words(
            &line_words[6..],
            ATTACHED_THIS_SOURCE_PREFIX_PATTERN,
        ) {
            Some(2usize)
        } else if attached_shape_matches_words(&line_words[6..], ATTACHED_THIS_WORD_PATTERN) {
            Some(1usize)
        } else {
            let prevent_idx = ATTACHED_PREVENT_WORD_PATTERN.find_word(&line_words[6..]);
            let source_end = prevent_idx.map(|idx| {
                ATTACHED_WHILE_WORD_PATTERN
                    .find_word(&line_words[6..6 + idx])
                    .unwrap_or(idx)
            });
            source_end.filter(|source_end| {
                *source_end > 0
                    && crate::runtime_backend::front_end::shared::util::source_reference_surface_for_words(
                        &line_words[6..6 + *source_end],
                    )
                    .is_some()
            })
        };
        if let Some(source_words_used) = source_words_used {
            let generic_tail = [
                "prevent", "that", "damage", "and", "put", "that", "many", "+1/+1", "counters",
                "on", "it",
            ];
            let generic_tail_alt = [
                "prevent", "that", "damage", "and", "put", "that", "many", "+1/+1", "counters",
                "on", "this", "creature",
            ];
            let generic_tail_pronoun = [
                "prevent", "that", "damage", "and", "put", "that", "many", "+1/+1", "counters",
                "on", "him",
            ];
            let instead_tail = [
                "put", "that", "many", "+1/+1", "counters", "on", "it", "instead",
            ];
            let instead_tail_alt = [
                "put", "that", "many", "+1/+1", "counters", "on", "this", "creature", "instead",
            ];

            let tail_word_start = 6 + source_words_used;
            let mut tail = &line_words[tail_word_start..];
            let mut condition = None;

            if ATTACHED_WHILE_WORD_PATTERN.matches_first_word(tail) {
                let Some(prevent_word_idx) = ATTACHED_PREVENT_WORD_PATTERN.find_word(tail) else {
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

    if attached_shape_matches_words(&line_words, ATTACHED_IF_NONCOMBAT_DAMAGE_TO_PREFIX_PATTERN)
        && attached_shape_matches_words(&line_words[7..], ATTACHED_THIS_CREATURE_PREFIX_PATTERN)
        && attached_shape_matches_words(&line_words[9..], ATTACHED_NONCOMBAT_COUNTER_TAIL_PATTERN)
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

    if attached_shape_matches_words(
        &line_words,
        ATTACHED_IF_CREATURE_COMBAT_DAMAGE_TO_THIS_PREFIX_PATTERN,
    ) && attached_shape_matches_words(&line_words[10..], ATTACHED_COMBAT_COUNTER_TAIL_PATTERN)
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
    let line_words = crate::runtime_backend::lexer::token_word_refs(tokens);
    if line_words.len() < 6 {
        return Ok(None);
    }

    // "Prevent all damage that would be dealt by enchanted creature."
    if !attached_shape_matches_words(&line_words, ATTACHED_PREVENT_ALL_DAMAGE_PREFIX_PATTERN) {
        return Ok(None);
    }
    if !attached_shape_matches_words(&line_words, ATTACHED_BY_ENCHANTED_CREATURE_TAIL_PATTERN) {
        return Ok(None);
    }

    let display = "prevent all damage that would be dealt by enchanted creature".to_string();
    Ok(Some(StaticAbilityAst::AttachedStaticAbilityGrant {
        ability: Box::new(StaticAbilityAst::Static(StaticAbility::new(
            crate::static_abilities::PREVENT_ALL_DAMAGE_DEALT_BY_THIS_PERMANENT,
        ))),
        display,
        condition: None,
    }))
}

pub(crate) fn parse_attached_prevent_all_combat_damage_dealt_by_attached_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let line_words = crate::runtime_backend::lexer::token_word_refs(tokens);
    if line_words.len() < 7 {
        return Ok(None);
    }

    // "Prevent all combat damage that would be dealt by enchanted creature."
    if !attached_shape_matches_words(
        &line_words,
        ATTACHED_PREVENT_ALL_COMBAT_DAMAGE_PREFIX_PATTERN,
    ) {
        return Ok(None);
    }
    if !attached_shape_matches_words(&line_words, ATTACHED_BY_ENCHANTED_CREATURE_TAIL_PATTERN) {
        return Ok(None);
    }

    let display = "prevent all combat damage that would be dealt by enchanted creature".to_string();
    Ok(Some(StaticAbilityAst::AttachedStaticAbilityGrant {
        ability: Box::new(StaticAbilityAst::Static(StaticAbility::new(
            crate::static_abilities::PREVENT_ALL_COMBAT_DAMAGE_DEALT_BY_THIS_PERMANENT,
        ))),
        display,
        condition: None,
    }))
}

pub(crate) fn parse_attached_prevent_all_damage_dealt_to_attached_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let line_words = crate::runtime_backend::lexer::parser_token_word_refs(tokens);
    if line_words.len() < 6 {
        return Ok(None);
    }

    // "Prevent all damage that would be dealt to enchanted creature."
    if !attached_shape_matches_words(&line_words, ATTACHED_PREVENT_ALL_DAMAGE_TO_PREFIX_PATTERN) {
        return Ok(None);
    }
    if !attached_shape_matches_words(&line_words, ATTACHED_TO_ENCHANTED_CREATURE_TAIL_PATTERN) {
        return Ok(None);
    }

    let display = "prevent all damage that would be dealt to enchanted creature".to_string();
    Ok(Some(StaticAbilityAst::AttachedStaticAbilityGrant {
        ability: Box::new(StaticAbilityAst::Static(StaticAbility::new(
            crate::static_abilities::StaticAbilityId::PreventAllDamageToSelf,
        ))),
        display,
        condition: None,
    }))
}

pub(crate) fn parse_attached_has_keywords_and_triggered_ability_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let line_words = crate::runtime_backend::lexer::token_word_refs(tokens);
    if line_words.len() < 6 {
        return Ok(None);
    }

    if !attached_shape_matches_words(&line_words, ATTACHED_CREATURE_PREFIX_PATTERN) {
        return Ok(None);
    }
    let is_equipped =
        attached_shape_matches_words(&line_words, ATTACHED_EQUIPPED_CREATURE_PREFIX_PATTERN);

    let Some(has_idx) =
        crate::runtime_backend::grammar::primitives::find_token_index(tokens, |token| {
            attached_token_word_is(token, ATTACHED_HAS_WORD)
        })
    else {
        return Ok(None);
    };
    if has_idx + 1 >= tokens.len() {
        return Ok(None);
    }

    let ability_tokens = trim_edge_punctuation(&tokens[has_idx + 1..]);
    if ability_tokens.is_empty() {
        return Ok(None);
    }

    let Some(and_idx) =
        crate::runtime_backend::grammar::primitives::find_token_index(&ability_tokens, |token| {
            attached_token_word_is(token, ATTACHED_AND_WORD)
        })
    else {
        return Ok(None);
    };
    if and_idx == 0 || and_idx + 1 >= ability_tokens.len() {
        return Ok(None);
    }

    let trigger_starts = ability_tokens
        .get(and_idx + 1)
        .is_some_and(|token| attached_token_word_is_any(token, ATTACHED_WHEN_OR_WHENEVER_WORDS))
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
    let triggered = match crate::runtime_backend::clause_support::parse_triggered_line_lexed(
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
            Some(crate::runtime_backend::lexer::token_word_refs(&trigger_tokens).join(" ")),
            crate::runtime_backend::trigger_frequency_condition(
                Some(&crate::runtime_backend::lexer::token_word_refs(&trigger_tokens).join(" ")),
                max_triggers_per_turn,
            ),
            None,
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
    let subject_text = crate::runtime_backend::lexer::token_word_refs(&tokens[..has_idx]).join(" ");
    let display = format!(
        "{subject_text} has {}",
        crate::runtime_backend::lexer::token_word_refs(&trigger_tokens).join(" ")
    );
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
    let line_words = crate::runtime_backend::lexer::token_word_refs(tokens);
    if line_words.len() < 10 {
        return Ok(None);
    }

    if !attached_shape_matches_words(&line_words, ATTACHED_CREATURE_PREFIX_PATTERN) {
        return Ok(None);
    }

    let Some(is_idx) =
        crate::runtime_backend::grammar::primitives::find_token_index(tokens, |token| {
            attached_token_word_is(token, ATTACHED_IS_WORD)
        })
    else {
        return Ok(None);
    };
    if is_idx < 2
        || !tokens
            .get(is_idx + 1)
            .is_some_and(|token| attached_token_word_is(token, ATTACHED_LEGENDARY_WORD))
    {
        return Ok(None);
    }

    let Some(get_idx) =
        crate::runtime_backend::grammar::primitives::find_token_index(tokens, |token| {
            attached_token_word_is_any(token, ATTACHED_GET_OR_GETS_WORDS)
        })
    else {
        return Ok(None);
    };
    let Some(has_idx) =
        crate::runtime_backend::grammar::primitives::find_token_index(tokens, |token| {
            attached_token_word_is(token, ATTACHED_HAS_WORD)
        })
    else {
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
    let line_words = crate::runtime_backend::lexer::token_word_refs(tokens);
    if line_words.len() < 6 {
        return Ok(None);
    }
    let is_enchanted =
        attached_shape_matches_words(&line_words, ATTACHED_ENCHANTED_SUBJECT_PREFIX_PATTERN);
    let is_equipped =
        attached_shape_matches_words(&line_words, ATTACHED_EQUIPPED_SUBJECT_PREFIX_PATTERN);
    if !is_enchanted && !is_equipped {
        return Ok(None);
    }

    let Some(get_idx) =
        crate::runtime_backend::grammar::primitives::find_token_index(tokens, |token| {
            attached_token_word_is_any(token, ATTACHED_GET_OR_GETS_WORDS)
        })
    else {
        return Ok(None);
    };
    let Some(has_idx) =
        crate::runtime_backend::grammar::primitives::find_token_index(tokens, |token| {
            attached_token_word_is(token, ATTACHED_HAS_WORD)
        })
    else {
        return Ok(None);
    };
    let Some(and_idx) =
        crate::runtime_backend::grammar::primitives::find_token_index(tokens, |token| {
            attached_token_word_is(token, ATTACHED_AND_WORD)
        })
    else {
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
        .filter_map(|(idx, token)| attached_token_word_is(token, ATTACHED_AND_WORD).then_some(idx))
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

    let has_colon = contains_token_kind(&ability_tokens, TokenKind::Colon);
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

    if token_slice_first_is_any(&ability_tokens, &["when", "whenever", "at"])
        && let LineAst::Triggered {
            trigger,
            effects,
            max_triggers_per_turn,
        } = crate::runtime_backend::clause_support::parse_triggered_line_lexed(&ability_tokens)?
    {
        let parsed = parsed_triggered_ability(
            trigger,
            effects,
            vec![Zone::Battlefield],
            Some(crate::runtime_backend::lexer::token_word_refs(&ability_tokens).join(" ")),
            crate::runtime_backend::trigger_frequency_condition(
                Some(&crate::runtime_backend::lexer::token_word_refs(&ability_tokens).join(" ")),
                max_triggers_per_turn,
            ),
            None,
            ReferenceImports::default(),
        );
        if parsed_triggered_ability_is_empty(&parsed) {
            return Err(CardTextError::ParseError(format!(
                "unsupported empty attached triggered grant clause (clause: '{}')",
                line_words.join(" ")
            )));
        }
        let text = crate::runtime_backend::lexer::token_word_refs(&ability_tokens).join(" ");
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
    let line_words = crate::runtime_backend::lexer::token_word_refs(tokens);
    if line_words.len() < 4
        || !attached_shape_matches_words(&line_words, ATTACHED_EQUIPPED_CREATURE_PREFIX_PATTERN)
    {
        return Ok(None);
    }

    let Some(has_idx) =
        crate::runtime_backend::grammar::primitives::find_token_index(tokens, |token| {
            attached_token_word_is(token, ATTACHED_HAS_WORD)
        })
    else {
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
    let has_colon = contains_token_kind(&ability_tokens, TokenKind::Colon);
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
    if let Some(get_idx) =
        crate::runtime_backend::grammar::primitives::find_token_index(tokens, |token| {
            attached_token_word_is_any(token, ATTACHED_GET_OR_GETS_WORDS)
        })
        && get_idx < has_idx
    {
        let clause_tail_end = if has_idx > get_idx + 2
            && tokens
                .get(has_idx - 1)
                .is_some_and(|token| attached_token_word_is(token, ATTACHED_AND_WORD))
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
            crate::runtime_backend::lexer::token_word_refs(&tokens[..has_idx]).join(" "),
            display_text_for_tokens(&ability_tokens, true)
        ),
        condition: None,
    });

    Ok(Some(static_abilities))
}

pub(crate) fn parse_enchanted_has_activated_ability_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let line_words = crate::runtime_backend::lexer::token_word_refs(tokens);
    if !attached_shape_matches_words(&line_words, ATTACHED_ENCHANTED_PREFIX_PATTERN)
        || !line_words
            .iter()
            .any(|word| attached_word_is(word, ATTACHED_HAS_WORD))
    {
        return Ok(None);
    }

    let Some(has_idx) =
        crate::runtime_backend::grammar::primitives::find_token_index(tokens, |token| {
            attached_token_word_is(token, ATTACHED_HAS_WORD)
        })
    else {
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
            crate::runtime_backend::lexer::token_word_refs(&tokens[..has_idx]).join(" "),
            display_text_for_tokens(&ability_tokens, true)
        ),
        condition: None,
    }))
}
