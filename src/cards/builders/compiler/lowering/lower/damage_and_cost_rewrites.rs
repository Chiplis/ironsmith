fn apply_pending_mana_restrictions(
    parsed: &mut ParsedAbility,
    restrictions: &[String],
) -> Result<(), CardTextError> {
    let AbilityKind::Activated(ability) = &mut parsed.ability.kind else {
        return Err(CardTextError::InvariantViolation(
            "rewrite activated lowering expected activated ability kind".to_string(),
        ));
    };
    for restriction in restrictions {
        apply_pending_mana_restriction(ability, restriction);
    }
    Ok(())
}

fn parse_next_spell_cost_reduction_sentence_rewrite(tokens: &[OwnedLexToken]) -> Option<EffectAst> {
    let clause_words = token_word_refs(tokens);
    if !word_refs_have_prefix(clause_words.as_slice(), &["the", "next"]) {
        return None;
    }

    let spell_idx = word_refs_find(clause_words.as_slice(), "spell")?;
    let costs_idx = word_refs_find(clause_words.as_slice(), "costs")?;
    let less_idx = word_refs_find(clause_words.as_slice(), "less")?;
    if clause_words.get(spell_idx + 1).copied() != Some("you")
        || clause_words.get(spell_idx + 2).copied() != Some("cast")
        || clause_words.get(spell_idx + 3).copied() != Some("this")
        || clause_words.get(spell_idx + 4).copied() != Some("turn")
        || clause_words.get(less_idx + 1).copied() != Some("to")
        || clause_words.get(less_idx + 2).copied() != Some("cast")
        || costs_idx <= spell_idx
    {
        return None;
    }

    let spell_token_idx = find_index(tokens, |token| token.is_word("spell"))?;
    let costs_token_idx = find_index(tokens, |token| token.is_word("costs"))?;
    let less_token_idx = find_index(tokens, |token| token.is_word("less"))?;
    if less_token_idx <= costs_token_idx + 1 {
        return None;
    }
    let spell_filter_tokens = trim_lexed_commas(&tokens[2..spell_token_idx]);
    let reduction_tokens = trim_lexed_commas(&tokens[costs_token_idx + 1..less_token_idx]);
    let filter = parse_spell_filter_with_grammar_entrypoint_lexed(spell_filter_tokens);
    let reduction_symbols = reduction_tokens
        .iter()
        .filter_map(|token| match token.kind {
            TokenKind::ManaGroup => Some(token.slice.trim_start_matches('{').trim_end_matches('}')),
            TokenKind::Word | TokenKind::Number => token.as_word(),
            TokenKind::Comma | TokenKind::Period => None,
            _ => Some(""),
        })
        .map(parse_mana_symbol)
        .collect::<Result<Vec<_>, _>>()
        .ok()?;
    if reduction_symbols.is_empty() {
        return None;
    }
    let reduction = crate::mana::ManaCost::from_symbols(reduction_symbols);

    Some(EffectAst::ReduceNextSpellCostThisTurn {
        player: crate::cards::builders::PlayerAst::You,
        filter,
        reduction,
    })
}

fn parse_each_player_and_their_creatures_damage_sentence_rewrite(
    effect_text: &str,
    tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    let normalized = effect_text
        .trim()
        .trim_end_matches('.')
        .to_ascii_lowercase();
    let matches_shape = str_contains(
        normalized.as_str(),
        " damage to each player and each creature they control",
    ) || str_contains(
        normalized.as_str(),
        " damage to each player and each creatures they control",
    ) || str_contains(
        normalized.as_str(),
        " damage to each player and each creature that player controls",
    ) || str_contains(
        normalized.as_str(),
        " damage to each player and each creatures that player controls",
    );
    if !matches_shape {
        return None;
    }
    let clause_words = token_word_refs(tokens);
    let deals_idx = find_index(clause_words.as_slice(), |word| {
        matches!(*word, "deal" | "deals")
    })?;
    let amount_start = token_index_for_word_index(tokens, deals_idx + 1)?;
    let (amount, _used) = parse_number_or_x_value_lexed(&tokens[amount_start..])?;

    let mut filter = crate::filter::ObjectFilter::default();
    filter.card_types = vec![crate::types::CardType::Creature];
    filter.controller = Some(crate::PlayerFilter::IteratedPlayer);

    Some(vec![EffectAst::ForEachPlayer {
        effects: vec![
            EffectAst::DealDamage {
                amount: amount.clone(),
                target: crate::cards::builders::TargetAst::Player(
                    crate::PlayerFilter::IteratedPlayer,
                    None,
                ),
            },
            EffectAst::DealDamageEach { amount, filter },
        ],
    }])
}

#[allow(dead_code)]
pub(crate) fn lower_rewrite_document(
    doc: RewriteSemanticDocument,
) -> Result<(CardDefinition, ParseAnnotations), CardTextError> {
    let parsed = rewrite_document_to_parsed_card_ast(doc)?;
    let ast = prepare_parsed_card_ast_for_lowering(parsed)?;
    lower_normalized_card_ast(ast)
}

#[allow(dead_code)]
pub(crate) fn lower_parsed_card_ast(
    ast: ParsedCardAst,
) -> Result<(CardDefinition, ParseAnnotations), CardTextError> {
    lower_normalized_card_ast(prepare_parsed_card_ast_for_lowering(ast)?)
}

pub(crate) fn lower_normalized_card_ast(
    ast: NormalizedCardAst,
) -> Result<(CardDefinition, ParseAnnotations), CardTextError> {
    let NormalizedCardAst {
        mut builder,
        mut annotations,
        items,
        allow_unsupported,
    } = ast;

    let mut level_abilities = Vec::new();
    let mut last_restrictable_ability: Option<usize> = None;
    let mut state = RewriteLoweredCardState::default();

    for item in items {
        match item {
            NormalizedCardItem::Line(line) => {
                rewrite_lower_line_ast(
                    &mut builder,
                    &mut state,
                    &mut annotations,
                    line,
                    allow_unsupported,
                    &mut last_restrictable_ability,
                )?;
            }
            NormalizedCardItem::Modal(modal) => {
                let abilities_before = builder.abilities.len();
                builder = rewrite_lower_parsed_modal(builder, modal, allow_unsupported)?;
                rewrite_update_last_restrictable_ability(
                    &builder,
                    abilities_before,
                    &mut last_restrictable_ability,
                );
            }
            NormalizedCardItem::LevelAbility(level) => {
                level_abilities.push(rewrite_lower_level_ability_ast(level)?);
            }
        }
    }

    if !level_abilities.is_empty() {
        builder = builder.with_level_abilities(level_abilities);
    }

    builder = rewrite_finalize_lowered_card(builder, &mut state);
    Ok((builder.build(), annotations))
}
