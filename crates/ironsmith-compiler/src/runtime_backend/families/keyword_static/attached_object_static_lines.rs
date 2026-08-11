fn split_attached_keyword_condition_suffix(
    ability_tokens: &[OwnedLexToken],
) -> Result<(Vec<OwnedLexToken>, Option<crate::ConditionExpr>), CardTextError> {
    let ability_tokens = trim_edge_punctuation(ability_tokens);
    let parsed = attached_grammar::split_attached_condition_suffix_tokens(&ability_tokens);
    let condition = match parsed {
        attached_grammar::AttachedConditionSuffix::None { .. } => None,
        attached_grammar::AttachedConditionSuffix::Clause {
            condition_tokens, ..
        } => Some(parse_static_condition_clause(condition_tokens)?),
        attached_grammar::AttachedConditionSuffix::YourTurn { .. } => {
            Some(crate::ConditionExpr::YourTurn)
        }
        attached_grammar::AttachedConditionSuffix::OtherTurns { .. } => Some(
            crate::ConditionExpr::Not(Box::new(crate::ConditionExpr::YourTurn)),
        ),
    };
    Ok((trim_edge_punctuation(parsed.ability_tokens()), condition))
}

fn explicit_attached_subject_tokens(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    if let Some(parsed) = attached_grammar::parse_attached_transform_tokens(tokens) {
        return Some(parsed.subject_tokens);
    }
    if let Some(parsed) = attached_grammar::parse_attached_has_tokens(tokens) {
        return Some(parsed.subject_tokens);
    }
    if attached_grammar::parse_attached_combat_restriction_tokens(tokens).is_some() {
        // Every currently modeled attached subject is the adjective/noun pair
        // `enchanted|equipped creature|permanent|land|artifact|equipment`.
        return tokens.get(..2);
    }
    None
}

fn parse_attached_loses_all_abilities_and_has_line(
    tokens: &[OwnedLexToken],
    subject_tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    if !matches!(
        words.get(..5),
        Some(["loses", "all", "abilities", "and", "has"])
    ) {
        return Ok(None);
    }
    let has_idx = tokens
        .iter()
        .position(|token| token.is_word("has"))
        .expect("matched attached ability grant");
    let mut grant_tokens = subject_tokens.to_vec();
    grant_tokens.extend_from_slice(&tokens[has_idx..]);
    let Some(mut grants) = parse_filter_has_granted_ability_line(&grant_tokens)? else {
        return Ok(None);
    };
    let filter = parse_object_filter(subject_tokens, false)?;
    let mut abilities = vec![StaticAbility::remove_all_abilities(filter).into()];
    abilities.append(&mut grants);
    Ok(Some(abilities))
}

fn parse_attached_combat_restriction_and_loses_all_abilities_line(
    tokens: &[OwnedLexToken],
    subject_tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let Some(loss_idx) = tokens.iter().position(|token| token.is_word("loses")) else {
        return Ok(None);
    };
    let Some(and_idx) = (0..loss_idx).rev().find(|idx| tokens[*idx].is_word("and")) else {
        return Ok(None);
    };
    if crate::runtime_backend::token_word_refs(&tokens[loss_idx..]) != ["loses", "all", "abilities"]
    {
        return Ok(None);
    }

    let mut restriction_tokens = subject_tokens.to_vec();
    restriction_tokens.extend_from_slice(trim_edge_punctuation(&tokens[..and_idx]).as_slice());
    let Some(restriction) = parse_attached_cant_attack_or_block_line(&restriction_tokens)? else {
        return Ok(None);
    };
    let filter = parse_object_filter(subject_tokens, false)?;
    Ok(Some(vec![
        restriction,
        StaticAbility::remove_all_abilities(filter).into(),
    ]))
}

pub(crate) fn parse_attached_conditional_loses_all_abilities_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let Some(comma_idx) = tokens.iter().position(|token| token.kind == TokenKind::Comma) else {
        return Ok(None);
    };
    let words = crate::runtime_backend::token_word_refs(tokens);
    if !words.starts_with(&["as", "long", "as", "enchanted"])
        && !words.starts_with(&["as", "long", "as", "equipped"])
    {
        return Ok(None);
    }
    let tail_words = crate::runtime_backend::token_word_refs(&tokens[comma_idx + 1..]);
    if tail_words != ["it", "loses", "all", "abilities"] {
        return Ok(None);
    }
    let condition_tokens = trim_edge_punctuation(&tokens[3..comma_idx]);
    let condition = parse_static_condition_clause(&condition_tokens)?;
    if !matches!(condition, crate::ConditionExpr::AttachedToSourceMatches(_)) {
        return Ok(None);
    }
    let subject_words = crate::runtime_backend::token_word_refs(&condition_tokens);
    let subject = subject_words
        .get(..2)
        .map(|words| words.join(" "))
        .unwrap_or_else(|| "attached permanent".to_string());
    Ok(Some(vec![StaticAbilityAst::AttachedStaticAbilityGrant {
        ability: Box::new(StaticAbilityAst::Static(
            StaticAbility::remove_all_abilities(ObjectFilter::source()),
        )),
        display: format!("{subject} loses all abilities"),
        condition: Some(condition),
    }]))
}

/// Carry an explicit attached-object subject into a following `It ...`
/// sentence before ordinary sentence splitting. The reconstructed token slice
/// is routed through the same typed attached-object parsers as an explicit
/// subject, so the pronoun cannot silently fall back to the Aura itself or to
/// every permanent.
pub(crate) fn parse_carried_attached_subject_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let sentences = crate::runtime_backend::lexer::split_lexed_sentences(tokens);
    let [first, second] = sentences.as_slice() else {
        return Ok(None);
    };
    let Some(subject_tokens) = explicit_attached_subject_tokens(first) else {
        return Ok(None);
    };
    let Some((pronoun, continuation)) = second.split_first() else {
        return Ok(None);
    };
    if !pronoun.is_word("it") {
        return Ok(None);
    }
    let continuation = trim_edge_punctuation(continuation);
    let Some(mut abilities) = parse_static_ability_ast_line_lexed_single(first)? else {
        return Ok(None);
    };

    let parsed_continuation = if let Some(parsed) =
        parse_attached_loses_all_abilities_and_has_line(&continuation, subject_tokens)?
    {
        Some(parsed)
    } else {
        parse_attached_combat_restriction_and_loses_all_abilities_line(
            &continuation,
            subject_tokens,
        )?
    };
    let Some(mut continuation_abilities) = parsed_continuation else {
        return Ok(None);
    };
    abilities.append(&mut continuation_abilities);
    Ok(Some(abilities))
}

fn negate_attached_keyword_condition(condition: crate::ConditionExpr) -> crate::ConditionExpr {
    match condition {
        crate::ConditionExpr::Not(inner) => *inner,
        other => crate::ConditionExpr::Not(Box::new(other)),
    }
}

fn parse_attached_keyword_action_grants(
    subject: &str,
    ability_tokens: &[OwnedLexToken],
    condition: Option<crate::ConditionExpr>,
    clause_text: &str,
    prefer_equipment_grant_for_unconditional_equipped: bool,
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let Some(actions) = parse_ability_line(ability_tokens) else {
        return Ok(None);
    };

    let mut actions_to_grant = Vec::new();
    let mut out = Vec::new();
    for action in actions {
        reject_unimplemented_keyword_actions(std::slice::from_ref(&action), clause_text)?;
        if let KeywordAction::Annihilator(amount) = action {
            out.push(StaticAbilityAst::AttachedObjectAbilityGrant {
                ability: parsed_ability_from_ability(annihilator_granted_ability(amount)),
                display: format!("{subject} has annihilator {amount}"),
                condition: condition.clone(),
            });
            continue;
        }
        if let KeywordAction::CumulativeUpkeep { total_cost, text } = action {
            out.push(StaticAbilityAst::AttachedObjectAbilityGrant {
                ability: parsed_ability_from_ability(cumulative_upkeep_granted_ability(total_cost)),
                display: format!("{subject} has {}", text.to_ascii_lowercase()),
                condition: condition.clone(),
            });
            continue;
        }
        if action.lowers_to_static_ability() {
            actions_to_grant.push(action);
        }
    }

    if actions_to_grant.is_empty() && out.is_empty() {
        return Ok(None);
    }

    if prefer_equipment_grant_for_unconditional_equipped && condition.is_none() {
        if !actions_to_grant.is_empty() {
            out.insert(
                0,
                StaticAbilityAst::EquipmentKeywordActionsGrant {
                    actions: actions_to_grant,
                },
            );
        }
    } else {
        for action in actions_to_grant {
            let display = format!(
                "{subject} has {}",
                action.display_text().to_ascii_lowercase()
            );
            out.push(StaticAbilityAst::AttachedKeywordActionGrant {
                action,
                display,
                condition: condition.clone(),
                protection_does_not_remove_controlled_attachments: false,
            });
        }
    }

    Ok(Some(out))
}

fn parse_attached_has_keyword_condition_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<(String, crate::ConditionExpr, Vec<StaticAbilityAst>)>, CardTextError> {
    let Some(has) = attached_grammar::parse_attached_has_tokens(tokens) else {
        return Ok(None);
    };
    if !matches!(
        has.subject,
        attached_grammar::AttachedSubject::EquippedCreature
            | attached_grammar::AttachedSubject::EnchantedCreature
            | attached_grammar::AttachedSubject::EnchantedPermanent
    ) {
        return Ok(None);
    }
    let subject = has.subject.display();

    let ability_tokens = trim_edge_punctuation(has.ability_tokens);
    if ability_tokens.is_empty() {
        return Ok(None);
    }
    let (ability_tokens, condition) = split_attached_keyword_condition_suffix(&ability_tokens)?;
    let Some(condition) = condition else {
        return Ok(None);
    };
    let clause_text = crate::runtime_backend::lexer::render_token_slice(tokens);
    let Some(grants) = parse_attached_keyword_action_grants(
        subject,
        &ability_tokens,
        Some(condition.clone()),
        &clause_text,
        false,
    )?
    else {
        return Ok(None);
    };
    Ok(Some((subject.to_string(), condition, grants)))
}

fn parse_attached_otherwise_has_keyword_sentence(
    tokens: &[OwnedLexToken],
    subject: &str,
    condition: crate::ConditionExpr,
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let Some(clause) =
        crate::runtime_backend::grammar::static_line_support::parse_otherwise_ability_clause(
            tokens,
        )
    else {
        return Ok(None);
    };
    let ability_tokens = trim_edge_punctuation(clause.ability_tokens);
    if ability_tokens.is_empty() {
        return Ok(None);
    }
    let clause_text = crate::runtime_backend::lexer::token_word_refs(tokens).join(" ");
    parse_attached_keyword_action_grants(
        subject,
        &ability_tokens,
        Some(condition),
        &clause_text,
        false,
    )
}

pub(crate) fn parse_attached_conditional_keyword_otherwise_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let sentences = crate::runtime_backend::lexer::split_lexed_sentences(tokens);
    let [first, second] = sentences.as_slice() else {
        return Ok(None);
    };

    let Some((subject, condition, mut grants)) =
        parse_attached_has_keyword_condition_sentence(first)?
    else {
        return Ok(None);
    };
    let otherwise_condition = negate_attached_keyword_condition(condition);
    let Some(mut otherwise_grants) =
        parse_attached_otherwise_has_keyword_sentence(second, &subject, otherwise_condition)?
    else {
        return Ok(None);
    };
    grants.append(&mut otherwise_grants);
    Ok(Some(grants))
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
    Ok(
        attached_grammar::parse_attached_base_power_toughness_tokens(tokens)?
            .map(|spec| (spec.power, spec.toughness, spec.preserve_other_types)),
    )
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
    let mut last_rendered_as_mana_symbol = false;

    for token in tokens {
        if let Some(word) = token.as_word() {
            if needs_space && !text.is_empty() {
                text.push(' ');
            }
            let numeric_like = word
                .chars()
                .all(|ch| ch.is_ascii_digit() || matches!(ch, 'x' | 'X' | '+' | '-' | '/'));
            let (mut rendered, rendered_as_mana_symbol) = match word {
                "t" => ("{T}".to_string(), true),
                "q" => ("{Q}".to_string(), true),
                _ if in_loyalty_cost || (in_effect_text && numeric_like) => {
                    (word.to_string(), false)
                }
                _ => match crate::runtime_backend::util::parse_mana_symbol(word) {
                    Ok(symbol) => (ManaCost::from_symbols(vec![symbol]).to_oracle(), true),
                    Err(_) => (word.to_string(), false),
                },
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
            last_rendered_as_mana_symbol = rendered_as_mana_symbol;
        } else if matches!(
            token.kind,
            crate::runtime_backend::lexer::TokenKind::ManaGroup
        ) {
            if needs_space && !text.is_empty() && !last_rendered_as_mana_symbol {
                text.push(' ');
            }
            text.push_str(token.slice.to_ascii_uppercase().as_str());
            needs_space = true;
            capitalize_next_cost_action = false;
            last_rendered_as_mana_symbol = true;
        } else if token.is_colon() {
            text.push(':');
            needs_space = true;
            in_effect_text = true;
            in_loyalty_cost = false;
            capitalize_next_effect_word = capitalize_effect_start;
            last_rendered_as_mana_symbol = false;
        } else if token.is_comma() {
            text.push(',');
            needs_space = true;
            if !in_effect_text {
                capitalize_next_cost_action = true;
            }
            last_rendered_as_mana_symbol = false;
        } else if token.is_period() {
            text.push('.');
            needs_space = true;
            if in_effect_text {
                capitalize_next_effect_word = capitalize_effect_start;
            }
            last_rendered_as_mana_symbol = false;
        } else if token.is_semicolon() {
            text.push(';');
            needs_space = true;
            last_rendered_as_mana_symbol = false;
        } else if token.kind == crate::runtime_backend::lexer::TokenKind::LBracket {
            if needs_space && !text.is_empty() {
                text.push(' ');
            }
            text.push('[');
            needs_space = false;
            in_loyalty_cost = true;
            last_rendered_as_mana_symbol = false;
        } else if token.kind == crate::runtime_backend::lexer::TokenKind::RBracket {
            text.push(']');
            needs_space = false;
            in_loyalty_cost = false;
            last_rendered_as_mana_symbol = false;
        } else if in_loyalty_cost && token.kind == crate::runtime_backend::lexer::TokenKind::Plus {
            text.push('+');
            needs_space = false;
            last_rendered_as_mana_symbol = false;
        } else if in_loyalty_cost && token.kind == crate::runtime_backend::lexer::TokenKind::Dash {
            text.push('-');
            needs_space = false;
            last_rendered_as_mana_symbol = false;
        }
    }

    text
}

#[cfg(test)]
#[path = "attached_object_static_lines/attached_static_line_migration_tests.rs"]
mod attached_static_line_migration_tests;

fn parse_attached_granted_activated_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedAbility>, CardTextError> {
    let trimmed = trim_edge_punctuation(tokens);
    let Some(source_name) =
        crate::runtime_backend::front_end::shared::util::current_source_reference_name()
    else {
        return parse_activated_line(&trimmed);
    };
    crate::runtime_backend::front_end::shared::util::with_source_reference_context(
        &source_name,
        || parse_activated_line(&trimmed),
    )
}

pub(crate) fn parse_attached_land_ability_reset_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let Some(shape) = attached_grammar::parse_attached_land_ability_reset_tokens(tokens) else {
        return Ok(None);
    };
    let filter = parse_object_filter(shape.subject_tokens, false)?;
    let line_text = crate::runtime_backend::lexer::render_token_slice(tokens);
    let mut abilities = vec![
        StaticAbility::set_land_subtypes(filter.clone(), Vec::new()).into(),
        StaticAbility::remove_all_abilities(filter).into(),
    ];

    for ability_tokens in shape.granted_abilities {
        let Some(parsed) = parse_attached_granted_activated_line(ability_tokens)? else {
            return Err(CardTextError::ParseError(format!(
                "unsupported attached land granted ability (clause: '{}')",
                line_text
            )));
        };
        abilities.push(StaticAbilityAst::AttachedObjectAbilityGrant {
            ability: parsed,
            display: format!(
                "enchanted land has {}",
                display_text_for_tokens(ability_tokens, true)
            ),
            condition: None,
        });
    }

    Ok(Some(abilities))
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
    let Some(has) = attached_grammar::parse_equipped_creature_has_tokens(tokens) else {
        return Ok(None);
    };
    let clause_text = crate::runtime_backend::lexer::render_token_slice(tokens);

    let ability_tokens = trim_edge_punctuation(has.ability_tokens);
    if ability_tokens.is_empty() {
        return Ok(None);
    }
    let (ability_tokens, condition) = split_attached_keyword_condition_suffix(&ability_tokens)?;
    parse_attached_keyword_action_grants(
        "equipped creature",
        &ability_tokens,
        condition,
        &clause_text,
        true,
    )
}

pub(crate) fn parse_enchanted_creature_has_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let tokens = super::grammar::line_families::parse_visible_line_tokens(tokens);
    let Some(has) = attached_grammar::parse_enchanted_has_tokens(tokens) else {
        return Ok(None);
    };
    let clause_text = crate::runtime_backend::lexer::render_token_slice(tokens);
    let subject = has.subject.display();
    const PROTECTION_ATTACHMENT_EXCEPTION: &[&str] = &[
        "this",
        "effect",
        "doesn't",
        "remove",
        "auras",
        "and",
        "equipment",
        "you",
        "control",
        "that",
        "are",
        "already",
        "attached",
        "to",
        "it",
    ];
    const PROTECTION_ATTACHMENT_EXCEPTION_ASCII: &[&str] = &[
        "this",
        "effect",
        "doesnt",
        "remove",
        "auras",
        "and",
        "equipment",
        "you",
        "control",
        "that",
        "are",
        "already",
        "attached",
        "to",
        "it",
    ];
    let line_words = crate::runtime_backend::lexer::parser_token_word_refs(tokens);
    let protection_attachment_exception = line_words
        .windows(PROTECTION_ATTACHMENT_EXCEPTION.len())
        .any(|window| {
            window == PROTECTION_ATTACHMENT_EXCEPTION
                || window == PROTECTION_ATTACHMENT_EXCEPTION_ASCII
        });

    let mut ability_tokens = trim_edge_punctuation(has.ability_tokens);
    if ability_tokens.is_empty() {
        return Ok(None);
    }

    let mut condition: Option<crate::ConditionExpr> = None;
    let (parsed_ability_tokens, parsed_condition) =
        split_attached_keyword_condition_suffix(&ability_tokens)?;
    if parsed_condition.is_some() {
        condition = parsed_condition;
        ability_tokens = parsed_ability_tokens;
    }

    if let Some(snow) = attached_grammar::parse_chosen_landwalk_tokens(&ability_tokens) {
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

    // A single `has` clause may grant ordinary keywords followed by a quoted
    // activated ability. Parse those heterogeneous halves independently.
    for split in attached_grammar::parse_attached_ability_splits_tokens(&ability_tokens)
        .into_iter()
        .rev()
    {
        let keyword_tokens = trim_edge_punctuation(split.keyword_tokens);
        let activated_tokens = trim_edge_punctuation(split.granted_tokens);
        let Some(mut grants) = parse_attached_keyword_action_grants(
            subject,
            &keyword_tokens,
            condition.clone(),
            &clause_text,
            false,
        )?
        else {
            continue;
        };
        let Some(parsed) = parse_attached_granted_activated_line(&activated_tokens)? else {
            continue;
        };
        grants.push(StaticAbilityAst::AttachedObjectAbilityGrant {
            ability: parsed,
            display: format!(
                "{subject} has {}",
                display_text_for_tokens(&activated_tokens, true)
            ),
            condition: condition.clone(),
        });
        return Ok(Some(grants));
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
        let preserves_controlled_attachments = protection_attachment_exception
            && matches!(action, KeywordAction::ProtectionFromChosenColor);
        let ability_text = if preserves_controlled_attachments {
            format!(
                "{ability_text}. This effect doesn't remove Auras and Equipment you control that are already attached to it"
            )
        } else {
            ability_text
        };
        out.push(StaticAbilityAst::AttachedKeywordActionGrant {
            action,
            display: ability_text,
            condition: condition.clone(),
            protection_does_not_remove_controlled_attachments: preserves_controlled_attachments,
        });
    }

    if out.is_empty() {
        return Ok(None);
    }
    Ok(Some(out))
}

/// Keep both halves of an attached-object keyword-plus-goaded clause as
/// continuous attachment semantics.
///
/// Without this rule, the generic effect parser folds the granted keywords
/// into the object filter of a one-shot goad effect. That changes
/// "has indestructible and is goaded" into "goad each creature that already
/// has indestructible."
pub(crate) fn parse_attached_has_keywords_and_is_goaded_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let tokens = super::grammar::line_families::parse_visible_line_tokens(tokens);
    let Some(has) = attached_grammar::parse_attached_has_tokens(tokens) else {
        return Ok(None);
    };
    let Some(and_index) = has.ability_tokens.windows(3).position(|window| {
        window[0].is_word("and")
            && matches!(window[1].parser_text.as_str(), "is" | "are")
            && window[2].is_word("goaded")
    }) else {
        return Ok(None);
    };
    if !trim_edge_punctuation(&has.ability_tokens[and_index + 3..]).is_empty() {
        return Ok(None);
    }

    let granted_tokens = trim_edge_punctuation(&has.ability_tokens[..and_index]);
    if granted_tokens.is_empty() {
        return Ok(None);
    }
    let subject = has.subject.display();
    let clause_text = crate::runtime_backend::lexer::render_token_slice(tokens);
    let Some(mut grants) = parse_attached_keyword_action_grants(
        subject,
        &granted_tokens,
        None,
        &clause_text,
        has.subject.is_equipped(),
    )?
    else {
        return Ok(None);
    };
    grants.push(
        crate::static_abilities::StaticAbility::attached_goaded_by_source_controller(format!(
            "{} is goaded",
            capitalize_display_subject(subject)
        ))
        .into(),
    );
    Ok(Some(grants))
}

/// Parse the old-frame attached-object restriction whose controller may take a
/// special action to ignore that restriction for the turn.
///
/// This is deliberately one typed rule for the complete two-sentence shape.
/// Parsing the second sentence as a spell-resolution `MayEffect` would make
/// the sacrifice happen when the Aura resolves and would lose the
/// "ignore ... until end of turn" semantics entirely.
pub(crate) fn parse_attached_restrictions_with_ignore_special_action_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let sentences = crate::runtime_backend::lexer::split_lexed_sentences(tokens);
    let [restrictions, special_action] = sentences.as_slice() else {
        return Ok(None);
    };

    let restriction_words = crate::runtime_backend::lexer::parser_token_word_refs(restrictions);
    let attached_noun = match restriction_words.as_slice() {
        [
            "enchanted",
            "creature",
            "cant",
            "attack",
            "or",
            "block",
            "and",
            "its",
            "activated",
            "abilities",
            "cant",
            "be",
            "activated",
        ] => "creature",
        [
            "enchanted",
            "permanent",
            "cant",
            "attack",
            "or",
            "block",
            "and",
            "its",
            "activated",
            "abilities",
            "cant",
            "be",
            "activated",
        ] => "permanent",
        _ => return Ok(None),
    };

    let special_action_words =
        crate::runtime_backend::lexer::parser_token_word_refs(special_action);
    let expected_special_action = [
        "that",
        match attached_noun {
            "creature" => "creatures",
            "permanent" => "permanents",
            _ => unreachable!("the complete grammar above owns the attached noun"),
        },
        "controller",
        "may",
        "sacrifice",
        "a",
        "permanent",
        "of",
        "their",
        "choice",
        "for",
        "that",
        "player",
        "to",
        "ignore",
        "this",
        "effect",
        "until",
        "end",
        "of",
        "turn",
    ];
    if special_action_words.as_slice() != expected_special_action {
        return Ok(None);
    }

    let subject = format!("enchanted {attached_noun}");
    let attached_filter = match attached_noun {
        "creature" => ObjectFilter::creature(),
        "permanent" => ObjectFilter::permanent_card().in_zone(Zone::Battlefield),
        _ => unreachable!("the complete grammar above owns the attached noun"),
    }
    .match_tagged(
        crate::tag::TagKey::from("enchanted"),
        crate::filter::TaggedOpbjectRelation::IsTaggedObject,
    );
    let combat_display = format!("{subject} can't attack or block");
    let combat_restriction = StaticAbilityAst::Static(StaticAbility::restriction(
        crate::effect::Restriction::attack_or_block(attached_filter.clone()),
        combat_display,
    ));
    let activation_display = format!("{subject} activated abilities can't be activated");
    let activation_restriction = StaticAbilityAst::Static(StaticAbility::restriction(
        crate::effect::Restriction::activate_abilities_of(attached_filter),
        activation_display,
    ));
    let special_action_display = format!(
        "That {attached_noun}'s controller may sacrifice a permanent of their choice for that player to ignore this effect until end of turn"
    );
    let ignore_special_action =
        StaticAbility::attached_controller_may_sacrifice_permanent_to_ignore_source_effect_until_end_of_turn(
            special_action_display,
        )
        .into();

    Ok(Some(vec![
        combat_restriction,
        activation_restriction,
        ignore_special_action,
    ]))
}

pub(crate) fn parse_attached_has_and_loses_keywords_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let Some(parsed) = attached_grammar::parse_attached_has_and_loses_tokens(tokens) else {
        return Ok(None);
    };
    let grant_tokens = trim_edge_punctuation(parsed.grant_tokens);
    let lose_tokens = trim_edge_punctuation(parsed.lose_tokens);
    if grant_tokens.is_empty() || lose_tokens.is_empty() {
        return Ok(None);
    }

    let Some(granted_actions) = parse_ability_line(&grant_tokens) else {
        return Ok(None);
    };
    let Some(removed_actions) = parse_ability_line(&lose_tokens) else {
        return Ok(None);
    };

    let clause_text = crate::runtime_backend::lexer::render_token_slice(tokens);
    let filter = parse_object_filter(parsed.subject_tokens, false)?;
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
            mode: ironsmith_core::AbilityLossMode::Lose,
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
    let Some(parsed) = attached_grammar::parse_attached_combat_restriction_tokens(tokens) else {
        return Ok(None);
    };
    let subject = parsed.subject.display();

    let (restriction, display) = match parsed.kind {
        attached_grammar::AttachedCombatRestrictionKind::CantAttack => (
            crate::effect::Restriction::attack(ObjectFilter::source()),
            format!("{subject} can't attack"),
        ),
        attached_grammar::AttachedCombatRestrictionKind::CantBlock => (
            crate::effect::Restriction::block(ObjectFilter::source()),
            format!("{subject} can't block"),
        ),
        attached_grammar::AttachedCombatRestrictionKind::CantAttackOrBlock => (
            crate::effect::Restriction::attack_or_block(ObjectFilter::source()),
            format!("{subject} can't attack or block"),
        ),
        attached_grammar::AttachedCombatRestrictionKind::CantBeBlocked => return Ok(None),
    };

    Ok(Some(StaticAbilityAst::AttachedStaticAbilityGrant {
        ability: Box::new(StaticAbilityAst::Static(StaticAbility::restriction(
            restriction,
            display.clone(),
        ))),
        display,
        condition: None,
    }))
}

pub(crate) fn parse_attached_all_creatures_able_to_block_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let Some(subject) = attached_grammar::parse_all_creatures_block_attached_tokens(tokens) else {
        return Ok(None);
    };
    let subject = subject.display();
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
    let Some(subject) = attached_grammar::parse_attached_tap_ability_restriction_tokens(tokens)
    else {
        return Ok(None);
    };
    let display = format!(
        "{}'s activated abilities with {{T}} in their costs can't be activated",
        subject.display()
    );

    Ok(Some(StaticAbilityAst::AttachedStaticAbilityGrant {
        ability: Box::new(StaticAbilityAst::Static(StaticAbility::restriction(
            crate::effect::Restriction::activate_tap_abilities_of(ObjectFilter::source()),
            display.clone(),
        ))),
        display,
        condition: None,
    }))
}

pub(crate) fn parse_you_control_attached_creature_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if attached_grammar::parse_you_control_attached_tokens(tokens).is_none() {
        return Ok(None);
    }

    Ok(Some(StaticAbility::control_attached_permanent(
        crate::runtime_backend::lexer::render_token_slice(tokens),
    )))
}

pub(crate) fn parse_attached_gets_and_cant_block_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let Some(parsed) = attached_grammar::parse_attached_gets_tail_tokens(tokens) else {
        return Ok(None);
    };
    let line_text = crate::runtime_backend::lexer::render_token_slice(tokens);
    let clause = parse_anthem_clause(tokens, parsed.get_token, parsed.and_token)?;
    let subject = parsed.subject.display();
    let anthem = build_anthem_static_ability(&clause);
    let granted = match parsed.tail {
        attached_grammar::AttachedGetsTailKind::Restriction(
            attached_grammar::AttachedCombatRestrictionKind::CantBlock,
        ) => StaticAbilityAst::AttachedStaticAbilityGrant {
            ability: Box::new(StaticAbilityAst::Static(StaticAbility::cant_block())),
            display: format!("{subject} can't block"),
            condition: clause.condition.clone(),
        },
        attached_grammar::AttachedGetsTailKind::Restriction(
            attached_grammar::AttachedCombatRestrictionKind::CantAttack,
        ) => StaticAbilityAst::AttachedStaticAbilityGrant {
            ability: Box::new(StaticAbilityAst::Static(StaticAbility::cant_attack())),
            display: format!("{subject} can't attack"),
            condition: clause.condition.clone(),
        },
        attached_grammar::AttachedGetsTailKind::Restriction(
            attached_grammar::AttachedCombatRestrictionKind::CantAttackOrBlock,
        ) => StaticAbilityAst::AttachedStaticAbilityGrant {
            ability: Box::new(StaticAbilityAst::Static(StaticAbility::restriction(
                crate::effect::Restriction::attack_or_block(ObjectFilter::source()),
                format!("{subject} can't attack or block"),
            ))),
            display: format!("{subject} can't attack or block"),
            condition: clause.condition.clone(),
        },
        attached_grammar::AttachedGetsTailKind::Restriction(
            attached_grammar::AttachedCombatRestrictionKind::CantBeBlocked,
        ) => {
            return Ok(Some(vec![
                anthem.into(),
                grant_keyword_action_for_anthem_subject(&clause, KeywordAction::Unblockable),
            ]));
        }
        attached_grammar::AttachedGetsTailKind::Loses(ability_tokens) => {
            let ability_tokens = trim_commas(ability_tokens);
            if ability_tokens.is_empty() {
                return Ok(None);
            }
            if crate::runtime_backend::lexer::token_word_refs(&ability_tokens).as_slice()
                == ["all", "abilities"]
            {
                let filter = match &clause.subject {
                    AnthemSubjectAst::Source => ObjectFilter::source(),
                    AnthemSubjectAst::Filter(filter) => filter.clone(),
                };
                return Ok(Some(vec![
                    anthem.into(),
                    StaticAbility::remove_all_abilities(filter).into(),
                ]));
            }
            let Some(actions) = parse_ability_line(&ability_tokens) else {
                return Ok(None);
            };
            reject_unimplemented_keyword_actions(&actions, &line_text)?;
            if actions.is_empty()
                || actions
                    .iter()
                    .any(|action| !action.lowers_to_static_ability())
            {
                return Ok(None);
            }
            let mut out = vec![anthem.into()];
            for action in actions {
                out.push(remove_keyword_action_for_anthem_subject(&clause, action));
            }
            return Ok(Some(out));
        }
    };
    Ok(Some(vec![anthem.into(), granted]))
}

pub(crate) fn parse_attached_type_transform_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let Some(parsed) = attached_grammar::parse_attached_transform_tokens(tokens) else {
        return Ok(None);
    };
    let line_text = crate::runtime_backend::lexer::render_token_slice(tokens);
    let subject_text = parsed.subject.display();
    let filter = parse_object_filter(parsed.subject_tokens, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported attached transform subject (clause: '{}')",
            line_text
        ))
    })?;
    let descriptor_words = crate::runtime_backend::lexer::token_word_refs(parsed.descriptor_tokens);
    if descriptor_words.is_empty() {
        return Ok(None);
    }

    let mut set_card_types = Vec::new();
    let mut add_subtypes = Vec::new();
    let mut set_colors = ColorSet::new();
    let mut make_colorless = false;
    for word in descriptor_words {
        match word {
            "and" => continue,
            "colorless" => {
                make_colorless = true;
                continue;
            }
            _ => {}
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
            word, line_text
        )));
    }

    let mut out = Vec::new();
    let mut preserve_other_types = false;
    let mut loss_consumed = false;

    if let Some(ability_tokens) = parsed.ability_tokens {
        let ability_tokens = trim_commas(ability_tokens);
        if ability_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing attached transform granted ability (clause: '{}')",
                line_text
            )));
        }

        if let Some(split) =
            attached_grammar::split_attached_base_pt_keyword_tokens(&ability_tokens)
        {
            let Some((power, toughness, with_preserve_other_types)) =
                parse_attached_with_base_power_toughness_clause(split.base_tokens)?
            else {
                return Err(CardTextError::ParseError(format!(
                    "unsupported attached transform granted ability (clause: '{}')",
                    line_text
                )));
            };
            preserve_other_types = with_preserve_other_types;
            out.push(
                StaticAbility::set_base_power_toughness(filter.clone(), power, toughness).into(),
            );

            if parsed.loss == Some(attached_grammar::AttachedTransformLossKind::AllAbilities) {
                out.push(StaticAbility::remove_all_abilities(filter.clone()).into());
                loss_consumed = true;
            }

            let Some(actions) = parse_ability_line(split.keyword_tokens) else {
                return Err(CardTextError::ParseError(format!(
                    "unsupported attached transform granted ability (clause: '{}')",
                    line_text
                )));
            };
            for action in actions {
                reject_unimplemented_keyword_actions(std::slice::from_ref(&action), &line_text)?;
                if !action.lowers_to_static_ability() {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported attached transform granted ability (clause: '{}')",
                        line_text
                    )));
                }
                out.push(StaticAbilityAst::AttachedKeywordActionGrant {
                    display: format!(
                        "{subject_text} has {}",
                        action.display_text().to_ascii_lowercase()
                    ),
                    action,
                    condition: None,
                    protection_does_not_remove_controlled_attachments: false,
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
                line_text
            )));
        }
    }

    let descriptor_has_card_types = !set_card_types.is_empty();
    let descriptor_sets_land_type = set_card_types.contains(&CardType::Land);
    if descriptor_has_card_types {
        if preserve_other_types {
            out.push(StaticAbility::add_card_types(filter.clone(), set_card_types).into());
        } else {
            out.push(
                StaticAbility::set_card_types_with_surface(
                    filter.clone(),
                    set_card_types,
                    line_text.clone(),
                )
                .into(),
            );
        }
    }
    if !add_subtypes.is_empty() {
        if !preserve_other_types
            && add_subtypes
                .iter()
                .all(crate::types::Subtype::is_land_subtype)
            && (descriptor_sets_land_type
                || parsed.subject == attached_grammar::AttachedSubject::EnchantedLand)
        {
            out.push(StaticAbility::set_land_subtypes(filter.clone(), add_subtypes).into());
        } else if !preserve_other_types
            && !descriptor_has_card_types
            && add_subtypes
                .iter()
                .all(crate::types::Subtype::is_creature_type)
            && parsed.subject == attached_grammar::AttachedSubject::EnchantedCreature
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

    if parsed.loss == Some(attached_grammar::AttachedTransformLossKind::AllAbilities)
        && !loss_consumed
    {
        out.push(StaticAbility::remove_all_abilities(filter.clone()).into());
    }

    if out.is_empty() {
        return Ok(None);
    }
    Ok(Some(out))
}

pub(crate) fn parse_prevent_damage_to_source_remove_counter_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let Some(spec) = attached_grammar::parse_remove_counter_prevention_tokens(tokens) else {
        return Ok(None);
    };
    lower_remove_counter_prevention_spec(spec).map(Some)
}

pub(crate) fn lower_remove_counter_prevention_spec(
    spec: attached_grammar::RemoveCounterPreventionSpec<'_>,
) -> Result<StaticAbilityAst, CardTextError> {
    let amount = match spec.amount {
        attached_grammar::RemoveCounterPreventionAmount::Fixed(amount) => {
            Value::Fixed(amount as i32)
        }
        attached_grammar::RemoveCounterPreventionAmount::DamageAmount => {
            Value::EventValue(EventValueSpec::Amount)
        }
    };
    let follow_up = spec.follow_up.map(|follow_up| {
        crate::static_abilities::CounterRemovalFollowUp::EachPlayerGetsCounters {
            counter_type: follow_up.counter_type,
            counters_per_removed: follow_up.counters_per_removed,
        }
    });
    let ability = StaticAbilityAst::Static(if spec.one_damage_per_counter {
        StaticAbility::prevent_one_damage_to_self_per_removed_counter(spec.counter_type)
    } else {
        StaticAbility::prevent_damage_to_self_remove_counter_with_follow_up(
            spec.counter_type,
            amount,
            follow_up,
        )
    });
    Ok(if let Some(condition_tokens) = spec.condition_tokens {
        StaticAbilityAst::ConditionalStaticAbility {
            ability: Box::new(ability),
            condition: parse_static_condition_clause(condition_tokens)?,
        }
    } else {
        ability
    })
}

pub(crate) fn parse_prevent_damage_to_source_put_counters_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let Some(parsed) = attached_grammar::parse_put_counter_prevention_tokens(tokens) else {
        return Ok(None);
    };
    let display = display_text_for_tokens(tokens, true);
    Ok(Some(match parsed {
        attached_grammar::PutCounterPreventionSpec::General {
            condition_tokens,
            display_prefix_tokens,
            effect_tokens,
        } => {
            let display = if condition_tokens.is_some() {
                let prefix =
                    crate::runtime_backend::lexer::token_word_refs(display_prefix_tokens).join(" ");
                let effect =
                    crate::runtime_backend::lexer::token_word_refs(effect_tokens).join(" ");
                let mut text = format!("{prefix}, {effect}");
                if let Some(first) = text.get_mut(0..1) {
                    first.make_ascii_uppercase();
                }
                text
            } else {
                display
            };
            let ability = StaticAbility::prevent_damage_to_self_put_counters_instead(
                crate::object::CounterType::PlusOnePlusOne,
                display,
            );
            let ast = StaticAbilityAst::Static(ability);
            if let Some(condition_tokens) = condition_tokens {
                StaticAbilityAst::ConditionalStaticAbility {
                    ability: Box::new(ast),
                    condition: parse_static_condition_clause(condition_tokens)?,
                }
            } else {
                ast
            }
        }
        attached_grammar::PutCounterPreventionSpec::Noncombat => StaticAbilityAst::Static(
            StaticAbility::prevent_constrained_damage_to_self_put_counters_instead(
                crate::object::CounterType::PlusOnePlusOne,
                display,
                None,
                Some(false),
            ),
        ),
        attached_grammar::PutCounterPreventionSpec::CreatureCombat => StaticAbilityAst::Static(
            StaticAbility::prevent_constrained_damage_to_self_put_counters_instead(
                crate::object::CounterType::PlusOnePlusOne,
                display,
                Some(ObjectFilter::creature()),
                Some(true),
            ),
        ),
    }))
}

pub(crate) fn parse_attached_prevent_all_damage_dealt_by_attached_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    if attached_grammar::parse_attached_prevent_all_tokens(tokens)
        != Some(attached_grammar::AttachedPreventAllKind::DamageDealtBy)
    {
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

pub(crate) fn parse_attached_prevent_all_damage_dealt_to_and_by_attached_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    if attached_grammar::parse_attached_prevent_all_tokens(tokens)
        != Some(attached_grammar::AttachedPreventAllKind::DamageDealtToAndBy)
    {
        return Ok(None);
    }
    let display =
        "prevent all damage that would be dealt to and dealt by enchanted creature".to_string();
    Ok(Some(StaticAbilityAst::AttachedStaticAbilityGrant {
        ability: Box::new(StaticAbilityAst::Static(
            StaticAbility::prevent_all_damage_dealt_to_and_by_this_permanent(),
        )),
        display,
        condition: None,
    }))
}

pub(crate) fn parse_attached_prevent_all_combat_damage_dealt_by_attached_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    if attached_grammar::parse_attached_prevent_all_tokens(tokens)
        != Some(attached_grammar::AttachedPreventAllKind::CombatDamageDealtBy)
    {
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
    if attached_grammar::parse_attached_prevent_all_tokens(tokens)
        != Some(attached_grammar::AttachedPreventAllKind::DamageDealtTo)
    {
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
    let Some(parsed) = attached_grammar::parse_attached_keywords_and_trigger_tokens(tokens) else {
        return Ok(None);
    };
    let clause_text = crate::runtime_backend::lexer::render_token_slice(tokens);
    let keyword_tokens = trim_edge_punctuation(parsed.keyword_tokens);
    let trigger_tokens = trim_edge_punctuation(parsed.trigger_tokens);
    let Some(actions) = parse_ability_line(&keyword_tokens) else {
        return Ok(None);
    };

    let mut keyword_actions = Vec::new();
    let mut extra_grants: Vec<StaticAbilityAst> = Vec::new();
    for action in actions {
        reject_unimplemented_keyword_actions(std::slice::from_ref(&action), &clause_text)?;
        if let KeywordAction::Annihilator(amount) = action {
            extra_grants.push(StaticAbilityAst::AttachedObjectAbilityGrant {
                ability: parsed_ability_from_ability(annihilator_granted_ability(amount)),
                display: format!("{} has annihilator {amount}", parsed.subject.display()),
                condition: None,
            });
        } else if action.lowers_to_static_ability() {
            keyword_actions.push(action);
        }
    }
    if keyword_actions.is_empty() && extra_grants.is_empty() {
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
            trigger_surface::parse_trigger_frequency_condition_tokens(
                &trigger_tokens,
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

    let subject = match parse_anthem_subject(parsed.subject_tokens) {
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
    let display = format!(
        "{} has {}",
        parsed.subject.display(),
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
    let Some(parsed) = attached_grammar::parse_attached_legendary_gets_has_tokens(tokens) else {
        return Ok(None);
    };
    let filter = parse_object_filter(parsed.subject_tokens, false)?;
    let Some(modifier_token) = parsed.modifier_token.as_word() else {
        return Ok(None);
    };
    let (power, toughness) = match parse_pt_modifier(modifier_token) {
        Ok(value) => value,
        Err(_) => return Ok(None),
    };
    let Some(actions) = parse_ability_line(parsed.keyword_tokens) else {
        return Ok(None);
    };

    let clause_text = crate::runtime_backend::lexer::render_token_slice(tokens);
    let mut out = Vec::new();
    out.push(StaticAbility::add_supertypes(filter.clone(), vec![Supertype::Legendary]).into());

    let anthem_clause = ParsedAnthemClause {
        subject: AnthemSubjectAst::Filter(filter.clone()),
        power: AnthemValue::Fixed(power),
        toughness: AnthemValue::Fixed(toughness),
        condition: None,
        count_uses_where_x: false,
        additional_surface: false,
        set_quantifier_surface: None,
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

    Ok(Some(out))
}

pub(crate) fn parse_attached_gets_and_has_ability_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let Some(shape) = attached_grammar::parse_attached_gets_and_has_tokens(tokens) else {
        return Ok(None);
    };
    let line_text = crate::runtime_backend::lexer::render_token_slice(tokens);
    let clause = parse_anthem_clause(tokens, shape.get_token, shape.and_token)?;
    let anthem = build_anthem_static_ability(&clause);
    let ability_tokens = trim_edge_punctuation(shape.ability_tokens);

    if let anthem_grant_grammar::ContinuingSegmentShape::Lose {
        ability_tokens: loss_tokens,
    } = anthem_grant_grammar::parse_continuing_segment_shape(&ability_tokens)
    {
        let loss_tokens = trim_edge_punctuation(loss_tokens);
        let Some(actions) = parse_ability_line(&loss_tokens) else {
            return Ok(None);
        };
        reject_unimplemented_keyword_actions(&actions, &line_text)?;
        if actions.is_empty()
            || actions
                .iter()
                .any(|action| !action.lowers_to_static_ability())
        {
            return Ok(None);
        }
        let mut out = vec![anthem.into()];
        out.extend(
            actions
                .into_iter()
                .map(|action| remove_keyword_action_for_anthem_subject(&clause, action)),
        );
        return Ok(Some(out));
    }

    if let Some(actions) = parse_ability_line(&ability_tokens) {
        reject_unimplemented_keyword_actions(&actions, &line_text)?;
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

    for split in attached_grammar::parse_attached_ability_splits_tokens(&ability_tokens)
        .into_iter()
        .rev()
    {
        let Some(actions) = parse_ability_line(split.keyword_tokens) else {
            continue;
        };
        reject_unimplemented_keyword_actions(&actions, &line_text)?;
        let keyword_actions = actions
            .into_iter()
            .filter(|action| action.lowers_to_static_ability())
            .collect::<Vec<_>>();
        if keyword_actions.is_empty() {
            continue;
        }

        if let Some(parsed) = parse_attached_granted_activated_line(split.granted_tokens)? {
            let mut out = vec![anthem.clone().into()];
            for action in keyword_actions {
                out.push(grant_keyword_action_for_anthem_subject(&clause, action));
            }
            let display = display_text_for_tokens(split.granted_tokens, false);
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
            line_text
        )));
    }

    if attached_grammar::parse_trigger_intro_tokens(&ability_tokens)
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
            trigger_surface::parse_trigger_frequency_condition_tokens(
                &ability_tokens,
                max_triggers_per_turn,
            ),
            None,
            ReferenceImports::default(),
        );
        if parsed_triggered_ability_is_empty(&parsed) {
            return Err(CardTextError::ParseError(format!(
                "unsupported empty attached triggered grant clause (clause: '{}')",
                line_text
            )));
        }
        let text = crate::runtime_backend::lexer::token_word_refs(&ability_tokens).join(" ");
        let grant = grant_object_ability_for_anthem_subject(&clause, parsed, text);
        return Ok(Some(vec![anthem.into(), grant]));
    }

    Err(CardTextError::ParseError(format!(
        "unsupported attached granted ability clause (clause: '{}')",
        line_text
    )))
}

pub(crate) fn parse_equipped_gets_and_has_activated_ability_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let Some(shape) = attached_grammar::parse_equipped_activated_grant_tokens(tokens) else {
        return Ok(None);
    };
    let line_text = crate::runtime_backend::lexer::render_token_slice(tokens);
    let ability_tokens_raw = shape.ability_tokens;
    let ability_tokens = trim_edge_punctuation(ability_tokens_raw);
    let has_colon = contains_token_kind(&ability_tokens, TokenKind::Colon);
    let Some(parsed) = parse_attached_granted_activated_line(ability_tokens_raw)? else {
        if has_colon {
            return Err(CardTextError::ParseError(format!(
                "unsupported equipped activated-ability grant (clause: '{}')",
                line_text
            )));
        }
        return Ok(None);
    };

    let mut static_abilities = Vec::new();
    if let Some((get_token, anthem_end)) = shape.anthem_bounds {
        let clause = parse_anthem_clause(tokens, get_token, anthem_end)?;
        static_abilities.push(build_anthem_static_ability(&clause).into());
    }
    static_abilities.push(StaticAbilityAst::AttachedObjectAbilityGrant {
        ability: parsed,
        display: format!(
            "{} has {}",
            crate::runtime_backend::lexer::token_word_refs(&tokens[..shape.has_token]).join(" "),
            display_text_for_tokens(&ability_tokens, true)
        ),
        condition: None,
    });

    Ok(Some(static_abilities))
}

pub(crate) fn parse_enchanted_has_activated_ability_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let Some(shape) = attached_grammar::parse_attached_has_tokens(tokens) else {
        return Ok(None);
    };
    if shape.subject.is_equipped() {
        return Ok(None);
    }
    let ability_tokens_raw = shape.ability_tokens;
    let ability_tokens = trim_edge_punctuation(ability_tokens_raw);

    // A mixed `has vigilance and "{W}, {T}: ..."` clause is not one
    // activated ability.  The permissive activated-line parser can recover the
    // quoted colon tail from that larger slice, so prove that no leading
    // keyword grant would be discarded before letting this early rule claim
    // the line.  The later attached-object rule lowers both halves.
    for split in attached_grammar::parse_attached_ability_splits_tokens(&ability_tokens) {
        if parse_ability_line(split.keyword_tokens).is_some()
            && parse_attached_granted_activated_line(split.granted_tokens)?.is_some()
        {
            return Ok(None);
        }
    }

    let Some(parsed) = parse_attached_granted_activated_line(ability_tokens_raw)? else {
        return Ok(None);
    };

    Ok(Some(StaticAbilityAst::AttachedObjectAbilityGrant {
        ability: parsed,
        display: format!(
            "{} has {}",
            shape.subject.display(),
            display_text_for_tokens(&ability_tokens, true)
        ),
        condition: None,
    }))
}
