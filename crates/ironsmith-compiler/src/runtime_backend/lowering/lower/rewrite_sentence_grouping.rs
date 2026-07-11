use super::*;

pub(crate) fn condition_for_chosen_option(context: &ChosenOptionContext) -> crate::ConditionExpr {
    match context {
        ChosenOptionContext::SourceOption(label) => {
            crate::ConditionExpr::SourceChosenOption(label.clone())
        }
        ChosenOptionContext::MaxSpeed => crate::ConditionExpr::ValueComparison {
            left: crate::effect::Value::Speed(PlayerFilter::You),
            operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
            right: crate::effect::Value::Fixed(4),
        },
        ChosenOptionContext::StationThreshold(threshold) => crate::ConditionExpr::ValueComparison {
            left: crate::effect::Value::CountersOnSource(crate::CounterType::Charge),
            operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
            right: crate::effect::Value::Fixed(*threshold),
        },
        ChosenOptionContext::ControlsSubtypePermanent(subtype) => {
            let filter = ObjectFilter::permanent()
                .you_control()
                .with_subtype(*subtype);
            crate::ConditionExpr::CountComparison {
                count: crate::static_abilities::AnthemCountExpression::MatchingFilter(filter),
                comparison: crate::effect::Comparison::GreaterThanOrEqual(1),
                display: Some(format!("you control a {subtype}")),
            }
        }
        ChosenOptionContext::ControlsEitherColorPermanent { left, right } => {
            let left_name = left.name();
            let right_name = right.name();
            let left_filter = ObjectFilter::permanent()
                .you_control()
                .with_colors(ColorSet::from_color(*left));
            let right_filter = ObjectFilter::permanent()
                .you_control()
                .with_colors(ColorSet::from_color(*right));
            let left_condition = crate::ConditionExpr::CountComparison {
                count: crate::static_abilities::AnthemCountExpression::MatchingFilter(left_filter),
                comparison: crate::effect::Comparison::GreaterThanOrEqual(1),
                display: Some(format!("you control a {left_name} permanent")),
            };
            let right_condition = crate::ConditionExpr::CountComparison {
                count: crate::static_abilities::AnthemCountExpression::MatchingFilter(right_filter),
                comparison: crate::effect::Comparison::GreaterThanOrEqual(1),
                display: Some(format!("you control a {right_name} permanent")),
            };
            crate::ConditionExpr::Or(Box::new(left_condition), Box::new(right_condition))
        }
    }
}

pub(crate) fn strip_non_keyword_label_prefix_for_lowering_lexed(
    mut tokens: &[OwnedLexToken],
) -> &[OwnedLexToken] {
    if looks_like_numeric_result_prefix_lexed(tokens) {
        return tokens;
    }
    while let Some((label_tokens, body_tokens)) =
        split_statement_label_prefix_for_lowering_lexed(tokens)
    {
        if crate::runtime_backend::grammar::document_shapes::parse_preserved_keyword_label_tokens(
            label_tokens,
        )
        .is_some()
        {
            break;
        }
        tokens = body_tokens;
    }
    tokens
}

pub(crate) fn rewrite_statement_followup_intro_for_lowering_lexed(
    tokens: &[OwnedLexToken],
) -> Vec<OwnedLexToken> {
    rewrite_followup_intro_to_if_lexed(tokens)
}

pub(crate) fn rewrite_copy_exception_type_removal_for_lowering_lexed(
    tokens: &[OwnedLexToken],
) -> Vec<OwnedLexToken> {
    remove_copy_exception_type_removal_lexed(tokens)
}

pub(crate) fn looks_like_numeric_result_prefix_lexed(tokens: &[OwnedLexToken]) -> bool {
    matches!(
        tokens.first().map(|token| token.kind),
        Some(TokenKind::Number)
    ) && matches!(
        tokens.get(1).map(|token| token.kind),
        Some(TokenKind::Dash | TokenKind::EmDash)
    ) && matches!(
        tokens.get(2).map(|token| token.kind),
        Some(TokenKind::Number)
    ) && tokens
        .iter()
        .skip(3)
        .any(|token| token.kind == TokenKind::Pipe)
}

pub(crate) fn rewrite_statement_parse_sentences_for_lowering_lexed(
    tokens: &[OwnedLexToken],
) -> Vec<Vec<OwnedLexToken>> {
    split_lexed_sentences(tokens)
        .into_iter()
        .filter(|sentence_tokens| !sentence_tokens.is_empty())
        .map(strip_non_keyword_label_prefix_for_lowering_lexed)
        .map(rewrite_statement_followup_intro_for_lowering_lexed)
        .map(|tokens| rewrite_copy_exception_type_removal_for_lowering_lexed(&tokens))
        .filter(|tokens| !tokens.is_empty())
        .collect()
}

pub(crate) fn statement_sentence_contains_instead_split_for_lowering(
    tokens: &[OwnedLexToken],
) -> bool {
    lexed_tokens_contain_non_prefix_instead(tokens)
}

pub(crate) fn group_statement_sentences_for_lowering_lexed(
    sentence_tokens: Vec<Vec<OwnedLexToken>>,
    fallback_tokens: &[OwnedLexToken],
) -> Vec<Vec<OwnedLexToken>> {
    if sentence_tokens.len() <= 1 {
        let only_sentence = sentence_tokens
            .into_iter()
            .next()
            .or_else(|| {
                let fallback = strip_non_keyword_label_prefix_for_lowering_lexed(fallback_tokens);
                (!fallback.is_empty()).then(|| {
                    rewrite_copy_exception_type_removal_for_lowering_lexed(
                        &rewrite_statement_followup_intro_for_lowering_lexed(fallback),
                    )
                })
            })
            .unwrap_or_default();
        return (!only_sentence.is_empty())
            .then_some(only_sentence)
            .into_iter()
            .collect();
    }

    let split_idx = sentence_tokens
        .iter()
        .enumerate()
        .skip(1)
        .find_map(|(idx, sentence)| {
            statement_sentence_contains_instead_split_for_lowering(sentence).then_some(idx)
        });

    let Some(split_idx) = split_idx else {
        return vec![join_sentences_with_period(&sentence_tokens)];
    };

    let mut groups = Vec::new();
    if !sentence_tokens[..split_idx].is_empty() {
        groups.push(join_sentences_with_period(&sentence_tokens[..split_idx]));
    }
    if !sentence_tokens[split_idx..].is_empty() {
        groups.push(join_sentences_with_period(&sentence_tokens[split_idx..]));
    }
    groups
}

pub(crate) fn wrap_chosen_option_static_chunk(
    chunk: LineAst,
    chosen_option: Option<&ChosenOptionContext>,
) -> Result<LineAst, CardTextError> {
    let Some(context) = chosen_option else {
        return Ok(chunk);
    };
    let condition = condition_for_chosen_option(context);
    Ok(match chunk {
        LineAst::Multiple(chunks) => LineAst::Multiple(
            chunks
                .into_iter()
                .map(|chunk| wrap_chosen_option_static_chunk(chunk, chosen_option))
                .collect::<Result<Vec<_>, _>>()?,
        ),
        LineAst::StaticAbility(ability) => LineAst::StaticAbility(
            crate::cards::builders::StaticAbilityAst::ConditionalStaticAbility {
                ability: Box::new(ability),
                condition,
            },
        ),
        LineAst::StaticAbilities(abilities) => LineAst::StaticAbilities(
            abilities
                .into_iter()
                .map(
                    |ability| crate::cards::builders::StaticAbilityAst::ConditionalStaticAbility {
                        ability: Box::new(ability),
                        condition: condition.clone(),
                    },
                )
                .collect(),
        ),
        LineAst::Abilities(actions) => LineAst::StaticAbilities(
            actions
                .into_iter()
                .map(
                    |action| crate::cards::builders::StaticAbilityAst::ConditionalKeywordAction {
                        action,
                        condition: condition.clone(),
                    },
                )
                .collect(),
        ),
        LineAst::Ability(mut parsed) => {
            if let AbilityKind::Static(static_ability) = parsed.kind_mut() {
                *static_ability = static_ability
                    .clone()
                    .with_condition(condition.clone())
                    .unwrap_or_else(|| {
                        crate::static_abilities::StaticAbility::new(
                            crate::static_abilities::GrantAbility::source(static_ability.clone())
                                .with_condition(condition.clone()),
                        )
                    });
            }
            LineAst::Ability(parsed)
        }
        other => other,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::builders::{CardDefinitionBuilder, LineAst, NormalizedLine};
    use crate::ids::CardId;
    use crate::runtime_backend::RewriteKeywordLineKind;
    use crate::runtime_backend::pipeline::parse_text_to_semantic_document;
    use crate::types::CardType;

    #[test]
    fn chosen_option_conditions_consume_typed_contexts() {
        assert!(matches!(
            condition_for_chosen_option(&ChosenOptionContext::source_option("khans")),
            crate::ConditionExpr::SourceChosenOption(option) if option == "khans"
        ));
        assert!(matches!(
            condition_for_chosen_option(&ChosenOptionContext::StationThreshold(5)),
            crate::ConditionExpr::ValueComparison {
                left: crate::effect::Value::CountersOnSource(crate::CounterType::Charge),
                operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                right: crate::effect::Value::Fixed(5),
            }
        ));
        assert!(matches!(
            condition_for_chosen_option(&ChosenOptionContext::MaxSpeed),
            crate::ConditionExpr::ValueComparison {
                left: crate::effect::Value::Speed(PlayerFilter::You),
                operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                right: crate::effect::Value::Fixed(4),
            }
        ));
    }

    #[test]
    fn rewrite_activated_sentence_alignment_merges_inner_quoted_periods() {
        let effect_text = r#"target creature gains "{t}: add {g}." until end of turn. any player may activate this ability."#;
        let effect_parse_tokens = lex_line(effect_text, 0)
            .expect("rewrite lexer should classify quoted activated effect");
        let rendered_effect_text = render_token_slice(&effect_parse_tokens).trim().to_string();
        let (parsed_sentences, _) = split_text_for_parse(
            rendered_effect_text.as_str(),
            rendered_effect_text.as_str(),
            0,
        );
        let token_sentence_texts = split_lexed_sentences(&effect_parse_tokens)
            .into_iter()
            .map(|tokens| render_token_slice(tokens).trim().to_string())
            .collect::<Vec<_>>();

        let aligned = align_rewrite_activated_parse_sentences(&parsed_sentences, &effect_parse_tokens)
            .unwrap_or_else(|| {
                panic!(
                    "quoted activated sentences should align against existing token slices: parsed={parsed_sentences:?} token_sentences={token_sentence_texts:?}"
                )
            });

        assert_eq!(aligned.len(), 2);
        assert_eq!(render_token_slice(&aligned[0]).trim(), parsed_sentences[0]);
        assert_eq!(render_token_slice(&aligned[1]).trim(), parsed_sentences[1]);
    }

    #[test]
    fn rewrite_exert_followup_subject_rewrite_uses_existing_tokens() {
        let tokens = lex_line("he can't block this turn.", 0)
            .expect("rewrite lexer should classify exert followup");

        let normalized = normalize_exert_followup_source_reference_tokens(
            "Champion",
            trim_lexed_commas(&tokens),
        );

        assert_eq!(
            render_token_slice(&normalized).trim(),
            "this creature can't block this turn."
        );
    }

    #[test]
    fn rewrite_exert_keyword_lowering_reuses_token_followup_for_linked_trigger()
    -> Result<(), CardTextError> {
        let text = "you may exert champion as it attacks. when you do, he can't block this turn.";
        let tokens = lex_line(text, 0).expect("rewrite lexer should classify exert keyword line");

        let parsed = parse_keyword_line_for_test(
            super::LineInfo {
                line_index: 0,
                display_line_index: 0,
                raw_line: text.to_string(),
                source_tokens: tokens.clone(),
                normalized: NormalizedLine {
                    original: text.to_string(),
                    normalized: text.to_string(),
                    char_map: Vec::new(),
                },
                semantic_facts: Default::default(),
            },
            text,
            &tokens,
            RewriteKeywordLineKind::ExertAttack,
        )?;

        match parsed {
            LineAst::StaticAbility(ability) => {
                let debug = format!("{ability:?}");
                assert!(
                    debug.contains("exert attack") || debug.contains("ExertAttack"),
                    "{debug}"
                );
            }
            other => panic!("expected exert static ability, got {other:?}"),
        }

        Ok(())
    }

    #[test]
    fn rewrite_exert_keyword_lowering_uses_parse_tokens_when_text_is_stale()
    -> Result<(), CardTextError> {
        let token_text = "if this creature hasn't been exerted this turn, you may exert champion as it attacks. when you do, he can't block this turn.";
        let tokens =
            lex_line(token_text, 0).expect("rewrite lexer should classify exert keyword line");

        let parsed = parse_keyword_line_for_test(
            super::LineInfo {
                line_index: 0,
                display_line_index: 0,
                raw_line: "placeholder exert text".to_string(),
                source_tokens: tokens.clone(),
                normalized: NormalizedLine {
                    original: "placeholder exert text".to_string(),
                    normalized: "placeholder exert text".to_string(),
                    char_map: Vec::new(),
                },
                semantic_facts: Default::default(),
            },
            "placeholder exert text",
            &tokens,
            RewriteKeywordLineKind::ExertAttack,
        )?;

        match parsed {
            LineAst::StaticAbility(ability) => {
                let debug = format!("{ability:?}");
                assert!(
                    debug.contains("exert attack") || debug.contains("ExertAttack"),
                    "{debug}"
                );
                assert!(
                    debug.contains("only_if_not_exerted_this_turn: true") || debug.contains("true"),
                    "{debug}"
                );
            }
            other => panic!("expected exert static ability, got {other:?}"),
        }

        Ok(())
    }

    #[test]
    fn rewrite_special_triggered_burning_rune_demon_accepts_stored_parse_tokens()
    -> Result<(), CardTextError> {
        let full_text = "when this creature enters, you may search your library for exactly two cards not named burning rune demon that have different names. if you do, reveal those cards. an opponent chooses one of them. put the chosen card into your hand and the other into your graveyard, then shuffle.";
        let trigger_text = "when this creature enters";
        let effect_text = "you may search your library for exactly two cards not named burning rune demon that have different names. if you do, reveal those cards. an opponent chooses one of them. put the chosen card into your hand and the other into your graveyard, then shuffle.";
        let full_tokens =
            lex_line(full_text, 0).expect("rewrite lexer should classify burning rune demon line");
        let trigger_tokens = lex_line(trigger_text, 0)
            .expect("rewrite lexer should classify burning rune demon trigger");
        let effect_tokens = lex_line(effect_text, 0)
            .expect("rewrite lexer should classify burning rune demon effect");

        let parsed = parse_triggered_line(
            super::LineInfo {
                line_index: 0,
                display_line_index: 0,
                raw_line: full_text.to_string(),
                source_tokens: full_tokens.clone(),
                normalized: NormalizedLine {
                    original: full_text.to_string(),
                    normalized: full_text.to_string(),
                    char_map: Vec::new(),
                },
                semantic_facts: Default::default(),
            },
            full_text,
            &full_tokens,
            trigger_text,
            &trigger_tokens,
            effect_text,
            &effect_tokens,
            None,
            None,
            None,
            None,
        )?;

        let debug = format!("{parsed:?}");
        assert!(debug.contains("Triggered"), "{debug}");
        assert!(debug.contains("divvy_source"), "{debug}");
        assert!(debug.contains("divvy_chosen"), "{debug}");
        assert!(debug.contains("ShuffleLibrary"), "{debug}");

        Ok(())
    }

    #[test]
    fn rewrite_divvy_suffix_trim_reuses_first_sentence_tokens() -> Result<(), CardTextError> {
        let tokens = lex_line(
            "Exile up to five target permanent cards from your graveyard and separate them into two piles.",
            0,
        )
        .expect("rewrite lexer should classify divvy exile sentence");
        let first_sentence = split_lexed_sentences(&tokens)
            .into_iter()
            .next()
            .expect("expected first sentence tokens");
        let trimmed = strip_lexed_suffix_phrase(
            first_sentence,
            &["and", "separate", "them", "into", "two", "piles"],
        )
        .expect("expected divvy pile suffix to trim");

        assert_eq!(
            render_token_slice(trimmed).trim(),
            "Exile up to five target permanent cards from your graveyard"
        );
        assert!(matches!(
            parse_single_effect_lexed(trimmed)?,
            EffectAst::SubjectVerb(crate::runtime_backend::ast::SubjectVerbEffectAst {
                action: crate::runtime_backend::ast::SubjectVerbActionAst::Exile { .. },
                ..
            })
        ));

        Ok(())
    }

    #[test]
    fn rewrite_triggered_normalization_keeps_explicit_intervening_if_predicate()
    -> Result<(), CardTextError> {
        let builder = CardDefinitionBuilder::new(CardId::new(), "Portcullis Variant")
            .card_types(vec![CardType::Artifact]);
        let (doc, _) = parse_text_to_semantic_document(
            builder,
            "Whenever a creature enters, if there are two or more other creatures on the battlefield, exile that creature. Return that card to the battlefield under its owner's control when this artifact leaves the battlefield.".to_string(),
            false,
        )?;

        let normalized = rewrite_document_to_normalized_card_ast(doc)?;
        let parsed = normalized
            .items
            .into_iter()
            .find_map(|item| match item {
                NormalizedCardItem::Line(line) => line.chunks.into_iter().find_map(|chunk| {
                    if let NormalizedLineChunk::Ability(parsed) = chunk {
                        Some(parsed)
                    } else {
                        None
                    }
                }),
                _ => None,
            })
            .expect("expected Portcullis-style line to normalize into a triggered ability");

        let AbilityKind::Triggered(triggered) = parsed.parsed.kind() else {
            panic!(
                "expected Portcullis-style line to normalize into a triggered ability, got {:?}",
                parsed.parsed.kind()
            );
        };
        let debug = format!("{:?}", triggered.intervening_if);
        assert!(
            triggered.intervening_if.is_some(),
            "expected trigger predicate to survive normalization, got {debug}"
        );
        assert!(
            debug.contains("ValueComparison"),
            "expected battlefield-count predicate to survive normalization, got {debug}"
        );

        Ok(())
    }
}
