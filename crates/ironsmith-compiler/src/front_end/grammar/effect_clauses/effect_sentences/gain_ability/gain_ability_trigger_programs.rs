use super::*;

pub(super) fn parse_granted_trigger_with_nested_token_rule(
    ability_tokens: &[OwnedLexToken],
    display: &str,
) -> Result<Option<ParsedAbility>, CardTextError> {
    let trigger_intro = clause_grammar::parse_trigger_intro_tokens(ability_tokens);
    let start_idx = trigger_intro.body_first;
    let Some(split_idx) =
        clause_grammar::parse_trigger_delimiters_tokens(ability_tokens).first_comma
    else {
        return Ok(None);
    };
    if split_idx <= start_idx || split_idx + 1 >= ability_tokens.len() {
        return Ok(None);
    }

    let trigger_tokens = &ability_tokens[start_idx..split_idx];
    let effect_tokens = trim_lexed_commas(&ability_tokens[split_idx + 1..]);
    let stripped_effect_tokens = strip_embedded_token_rules_text(effect_tokens);
    if stripped_effect_tokens.as_slice() == effect_tokens {
        return Ok(None);
    }

    // Only claim this boundary when both ordinary typed parsers succeed.
    // Otherwise the complete triggered-line grammar retains first refusal for
    // complex trigger clauses.
    let Ok(trigger) = parse_trigger_clause_lexed(trigger_tokens) else {
        return Ok(None);
    };
    let Ok(mut effects) = super::super::parse_effect_sentences_lexed(&stripped_effect_tokens)
    else {
        return Ok(None);
    };
    if !super::super::creation_handlers::attach_inline_token_granted_abilities_to_last_create(
        &mut effects,
        effect_tokens,
    ) {
        return Ok(None);
    }

    Ok(Some(parsed_triggered_ability(
        trigger,
        effects,
        vec![Zone::Battlefield],
        Some(display.to_string()),
        trigger_surface::parse_trigger_frequency_condition_tokens(ability_tokens, None),
        None,
        ReferenceImports::default(),
    )))
}

pub fn parse_granted_activated_or_triggered_ability_for_gain(
    ability_tokens: &[OwnedLexToken],
    clause_words: &[&str],
) -> Result<Option<GrantedAbilityAst>, CardTextError> {
    let ability_tokens = trim_edge_punctuation_and_quotes(ability_tokens);
    if ability_tokens.is_empty() {
        return Ok(None);
    }

    let has_colon = contains_token_kind(&ability_tokens, TokenKind::Colon);
    let looks_like_trigger = ability_tokens.first().is_some_and(|token| {
        token.kind == TokenKind::Word
            && (gain_shapes::gain_word_is_when_intro(token.parser_text())
                || (gain_shapes::gain_word_is_trigger_intro(token.parser_text())
                    && ability_tokens
                        .get(1)
                        .is_some_and(|next| next.parser_text() == THE_WORD)))
    });
    if !has_colon && !looks_like_trigger {
        return Ok(None);
    }

    let display = display_text_for_tokens(&ability_tokens);
    // Nested quoted rules use apostrophes when their enclosing granted
    // ability is already double-quoted. Normalize those standalone delimiter
    // tokens for semantic parsing so sentence splitting treats punctuation
    // inside the nested activation as part of that rule. Possessives remain
    // ordinary word tokens and are unaffected.
    let semantic_tokens = ability_tokens
        .iter()
        .map(|token| {
            if token.kind == TokenKind::Apostrophe {
                OwnedLexToken::new(TokenKind::Quote, "\"", token.span())
            } else {
                token.clone()
            }
        })
        .collect::<Vec<_>>();
    let semantic_tokens = normalize_named_granted_trigger_subject(&semantic_tokens);
    // An activated ability nested inside a triggered ability can contribute a
    // colon to the full token stream. The leading grammatical shape owns the
    // outer ability kind; only use a colon to select activation when the
    // ability itself does not begin with a trigger.
    let mut parsed_ability = if looks_like_trigger {
        if let Some(parsed) =
            parse_granted_trigger_with_nested_token_rule(&semantic_tokens, &display)?
        {
            parsed
        } else if let Some(parsed) =
            parse_granted_triggered_otherwise_ability(&semantic_tokens, &display)?
        {
            parsed
        } else {
            match parse_triggered_line_lexed(&semantic_tokens)? {
                LineAst::Triggered {
                    trigger,
                    effects,
                    max_triggers_per_turn,
                } => parsed_triggered_ability(
                    trigger,
                    effects,
                    vec![Zone::Battlefield],
                    Some(display.clone()),
                    trigger_surface::parse_trigger_frequency_condition_tokens(
                        &semantic_tokens,
                        max_triggers_per_turn,
                    ),
                    None,
                    ReferenceImports::default(),
                ),
                _ => {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported granted activated/triggered ability clause (clause: '{}')",
                        clause_words.join(" ")
                    )));
                }
            }
        }
    } else {
        let Some(parsed) = parse_activated_line(&semantic_tokens)? else {
            return Err(CardTextError::ParseError(format!(
                "unsupported granted activated/triggered ability clause (clause: '{}')",
                clause_words.join(" ")
            )));
        };
        parsed
    };

    // A generic quoted token ability can use the token's authored name as its
    // trigger subject (`When Ember dies, ...`). That route parses a complete
    // typed zone-change trigger, but unlike the ordinary triggered-line CST
    // handoff it can arrive without the leading trigger presentation. Carry
    // only the explicit first-word intro onto that already-typed trigger;
    // this keeps `When` distinct from `Whenever` without inferring frequency
    // from the matched event.
    if let crate::model::CompilerAbilityKindCore::Triggered(triggered) = parsed_ability.kind_mut()
        && !matches!(triggered.trigger, TriggerSpec::WithIntro { .. })
        && let Some(intro_surface) =
            ability_tokens
                .first()
                .and_then(|token| match token.parser_text() {
                    "when" => Some(crate::model::ast::TriggerIntroSurfaceAst::When),
                    "whenever" => Some(crate::model::ast::TriggerIntroSurfaceAst::Whenever),
                    "at" => Some(crate::model::ast::TriggerIntroSurfaceAst::At),
                    _ => None,
                })
    {
        triggered.trigger = TriggerSpec::WithIntro {
            intro: intro_surface,
            trigger: Box::new(triggered.trigger.clone()),
        };
        parsed_ability.trigger_spec = Some(Box::new(triggered.trigger.clone()));
    }

    Ok(Some(GrantedAbilityAst::ParsedObjectAbility {
        ability: Box::new(parsed_ability),
        display,
    }))
}

pub(super) fn normalize_named_granted_trigger_subject(
    tokens: &[OwnedLexToken],
) -> Vec<OwnedLexToken> {
    if !tokens
        .first()
        .is_some_and(|token| matches!(token.parser_text(), "when" | "whenever"))
    {
        return tokens.to_vec();
    }
    let Some(dies_idx) =
        crate::slice_primitives::select_position(tokens, |token| token.is_word("dies"))
    else {
        return tokens.to_vec();
    };
    if dies_idx <= 1
        || !tokens[1..dies_idx].iter().all(|token| {
            token.kind == TokenKind::Word
                && super::super::creation_handlers::is_probable_token_name_word(token.parser_text())
        })
    {
        return tokens.to_vec();
    }

    let mut normalized = Vec::with_capacity(tokens.len() - dies_idx + 3);
    normalized.push(tokens[0].clone());
    normalized.push(OwnedLexToken::word(
        "this".to_string(),
        TextSpan::synthetic(),
    ));
    normalized.push(OwnedLexToken::word(
        "token".to_string(),
        TextSpan::synthetic(),
    ));
    normalized.extend_from_slice(&tokens[dies_idx..]);
    normalized
}

pub(super) fn parse_granted_triggered_otherwise_ability(
    ability_tokens: &[OwnedLexToken],
    display: &str,
) -> Result<Option<ParsedAbility>, CardTextError> {
    let start_idx = if ability_tokens
        .first()
        .is_some_and(|token| gain_shapes::gain_word_is_trigger_intro(token.parser_text()))
    {
        1
    } else {
        0
    };
    let Some(comma_idx) = locate_token_kind(ability_tokens, TokenKind::Comma) else {
        return Ok(None);
    };
    let Some(otherwise_idx) = locate_token_word(ability_tokens, "otherwise") else {
        return Ok(None);
    };
    if otherwise_idx <= comma_idx + 1 || comma_idx <= start_idx {
        return Ok(None);
    }

    let trigger = parse_trigger_clause_lexed(&ability_tokens[start_idx..comma_idx])?;
    let true_tokens = trim_edge_punctuation(trim_lexed_commas(
        &ability_tokens[comma_idx + 1..otherwise_idx],
    ));
    let false_tokens =
        trim_edge_punctuation(trim_lexed_commas(&ability_tokens[otherwise_idx + 1..]));
    if true_tokens.is_empty() || false_tokens.is_empty() {
        return Ok(None);
    }

    let true_effect = parse_single_effect_sentence_for_granted_otherwise(&true_tokens)?;
    let mut false_effect = Some(parse_single_effect_sentence_for_granted_otherwise(
        &false_tokens,
    )?);
    let mut conditional = match true_effect {
        EffectAst::Conditional {
            predicate,
            if_true,
            if_false,
        } if if_false.is_empty() => EffectAst::Conditional {
            predicate,
            if_true,
            if_false,
        },
        EffectAst::TrailingIf { predicate, effects } => EffectAst::Conditional {
            predicate,
            if_true: effects,
            if_false: Vec::new(),
        },
        EffectAst::ControlFlow(control) => {
            let crate::model::CompilerControlFlowAst {
                semantic,
                node,
                mut programs,
                provenance,
                ..
            } = *control;
            let crate::model::control_flow::ControlFlowNodeAst::Condition {
                condition,
                consequence_program,
                alternative_program: None,
                reflexive,
            } = node
            else {
                return Ok(None);
            };
            let alternative_program = programs.len();
            programs.push(crate::model::NestedProgramAst::new(
                crate::model::NestedProgramKindAst::Alternative,
                vec![false_effect.take().expect("otherwise branch effect")],
            ));
            let control = crate::model::CompilerControlFlowAst::new(
                semantic,
                crate::model::control_flow::ControlFlowNodeAst::Condition {
                    condition,
                    consequence_program,
                    alternative_program: Some(alternative_program),
                    reflexive,
                },
                programs,
                provenance,
            )
            .map_err(|error| {
                CardTextError::InvariantViolation(format!(
                    "invalid granted triggered otherwise control flow: {error:?}"
                ))
            })?;
            EffectAst::ControlFlow(Box::new(control))
        }
        _ => return Ok(None),
    };
    if let EffectAst::Conditional { if_false, .. } = &mut conditional {
        *if_false = vec![false_effect.take().expect("otherwise branch effect")];
    }

    Ok(Some(parsed_triggered_ability(
        trigger,
        vec![conditional],
        vec![Zone::Battlefield],
        Some(display.to_string()),
        None,
        None,
        ReferenceImports::default(),
    )))
}
