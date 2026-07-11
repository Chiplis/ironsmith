use crate::ability::{PresentationKeyword, PresentationLabel};
use crate::host::{EffectAst, TriggerSpec};

type AnthemNormalizedWords<'a> = crate::runtime_backend::grammar::primitives::TokenWordView<'a>;

const EVERY_SUBTYPE_FAMILY_TAILS: &[(&[&str], crate::types::SubtypeFamily)] = &[
    (
        &["every", "creature", "type"],
        crate::types::SubtypeFamily::Creature,
    ),
    (
        &["every", "creature", "types"],
        crate::types::SubtypeFamily::Creature,
    ),
    (
        &["every", "land", "type"],
        crate::types::SubtypeFamily::Land,
    ),
    (
        &["every", "land", "types"],
        crate::types::SubtypeFamily::Land,
    ),
    (
        &["every", "artifact", "type"],
        crate::types::SubtypeFamily::Artifact,
    ),
    (
        &["every", "artifact", "types"],
        crate::types::SubtypeFamily::Artifact,
    ),
    (
        &["every", "enchantment", "type"],
        crate::types::SubtypeFamily::Enchantment,
    ),
    (
        &["every", "enchantment", "types"],
        crate::types::SubtypeFamily::Enchantment,
    ),
    (
        &["every", "spell", "type"],
        crate::types::SubtypeFamily::Spell,
    ),
    (
        &["every", "spell", "types"],
        crate::types::SubtypeFamily::Spell,
    ),
    (
        &["every", "planeswalker", "type"],
        crate::types::SubtypeFamily::Planeswalker,
    ),
    (
        &["every", "planeswalker", "types"],
        crate::types::SubtypeFamily::Planeswalker,
    ),
];
#[derive(Clone, Copy)]
enum GrantedAlternativeCastKeyword {
    Flashback,
    Blitz,
    Emerge,
    Miracle,
    Escape,
}

fn parse_granted_alternative_cast_keyword(words: &[&str]) -> Option<GrantedAlternativeCastKeyword> {
    let [keyword] = words else {
        return None;
    };
    match *keyword {
        "flashback" => Some(GrantedAlternativeCastKeyword::Flashback),
        "blitz" => Some(GrantedAlternativeCastKeyword::Blitz),
        "emerge" => Some(GrantedAlternativeCastKeyword::Emerge),
        "miracle" => Some(GrantedAlternativeCastKeyword::Miracle),
        "escape" => Some(GrantedAlternativeCastKeyword::Escape),
        _ => None,
    }
}

fn first_spell_each_turn_subject(filter_words: &[&str]) -> Option<AnthemSubjectAst> {
    anthem_grant_grammar::parse_first_spell_each_turn_subject_words(filter_words).then(|| {
        AnthemSubjectAst::Filter(
            ObjectFilter::spell()
                .cast_by(PlayerFilter::You)
                .first_spell_cast_each_turn(),
        )
    })
}

fn first_spell_each_turn_subject_tokens(
    tokens: &[OwnedLexToken],
) -> Result<Option<AnthemSubjectAst>, CardTextError> {
    let Some(parsed) = anthem_grant_grammar::parse_first_spell_each_turn_clause(tokens) else {
        return Ok(None);
    };

    let mut filter = parse_object_filter_lexed(parsed.filter_tokens, false)?;
    if filter.stack_kind != Some(crate::filter::StackObjectKind::Spell)
        && filter.zone != Some(Zone::Stack)
    {
        return Ok(None);
    }
    filter.cast_by = Some(PlayerFilter::You);
    filter.first_spell_cast_each_turn = true;
    Ok(Some(AnthemSubjectAst::Filter(filter)))
}

fn parse_cant_be_blocked_as_long_as_clause(
    tokens: &[OwnedLexToken],
) -> Option<anthem_grant_grammar::CantBeBlockedAsLongAsClause<'_>> {
    anthem_grant_grammar::parse_cant_be_blocked_as_long_as_clause(tokens)
}

fn parse_cant_be_blocked_clause(
    tokens: &[OwnedLexToken],
) -> Option<anthem_grant_grammar::CantBeBlockedClause<'_>> {
    anthem_grant_grammar::parse_cant_be_blocked_clause(tokens)
}

fn parse_keywords_and_cant_be_blocked_clause(
    tokens: &[OwnedLexToken],
) -> Option<anthem_grant_grammar::KeywordsAndCantBeBlockedClause<'_>> {
    anthem_grant_grammar::parse_keywords_and_cant_be_blocked_clause(tokens)
}

fn parse_landwalk_block_override_clause(
    tokens: &[OwnedLexToken],
) -> Option<anthem_grant_grammar::LandwalkBlockOverrideClause<'_>> {
    anthem_grant_grammar::parse_landwalk_block_override_clause(tokens)
}

fn parse_granted_escape_cost_tail_clause(
    tokens: &[OwnedLexToken],
) -> Option<anthem_grant_grammar::GrantedEscapeCostTail<'_>> {
    anthem_grant_grammar::parse_granted_escape_cost_tail_clause(tokens)
}

fn parse_granted_miracle_cost_reduction_tail_clause(
    tokens: &[OwnedLexToken],
) -> Option<anthem_grant_grammar::GrantedMiracleCostReductionTail<'_>> {
    anthem_grant_grammar::parse_granted_miracle_cost_reduction_tail_clause(tokens)
}

fn parse_cant_be_blocked_by_more_than_clause(
    tokens: &[OwnedLexToken],
) -> Option<anthem_grant_grammar::CantBeBlockedByMoreThanClause<'_>> {
    anthem_grant_grammar::parse_cant_be_blocked_by_more_than_clause(tokens)
}

fn parse_can_block_additional_creature_clause(
    tokens: &[OwnedLexToken],
) -> Option<anthem_grant_grammar::CanBlockAdditionalCreatureClause<'_>> {
    anthem_grant_grammar::parse_can_block_additional_creature_clause(tokens)
}

fn triggered_grant_effects_and_condition(
    trigger: &TriggerSpec,
    effects: &[EffectAst],
) -> Result<(Vec<EffectAst>, Option<crate::ConditionExpr>), CardTextError> {
    if let [
        EffectAst::Conditional {
            predicate,
            if_true,
            if_false,
        },
    ] = effects
        && if_false.is_empty()
    {
        let mut imports = ReferenceImports::default();
        imports.last_player_filter =
            crate::runtime_backend::compile_support::inferred_trigger_player_filter(trigger);
        let reference_env = crate::runtime_backend::reference_model::ReferenceEnv::from_imports(
            &imports, false, false, false, None,
        );
        let condition =
            crate::runtime_backend::compile_support::compile_condition_from_predicate_ast_with_env(
                predicate,
                &reference_env,
                None,
            )?;
        return Ok((if_true.clone(), Some(condition)));
    }

    Ok((effects.to_vec(), None))
}

pub(crate) fn parse_subject_cant_be_blocked_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let Some(parsed) = parse_cant_be_blocked_clause(tokens) else {
        return Ok(None);
    };
    let subject_tokens = parsed.subject_tokens;
    let subject_facts = anthem_grant_grammar::parse_cant_be_blocked_subject_facts(subject_tokens);
    if subject_facts.has_conjunction_or_comma {
        return Ok(None);
    }
    if subject_facts.starts_with_source_pronoun {
        return Ok(None);
    }
    if subject_facts.has_rejected_clause_word {
        return Ok(None);
    }
    if subject_facts.mentions_power_or_toughness {
        return Err(CardTextError::ParseError(format!(
            "unsupported power-or-toughness cant-be-blocked subject (clause: '{}')",
            crate::runtime_backend::token_word_refs(tokens).join(" ")
        )));
    }

    let subject_words = crate::runtime_backend::token_word_refs(&subject_tokens);
    let subject = first_spell_each_turn_subject(&subject_words)
        .map(Ok)
        .unwrap_or_else(|| parse_anthem_subject(&subject_tokens))?;
    let ability = match subject {
        AnthemSubjectAst::Source => StaticAbilityAst::KeywordAction(KeywordAction::Unblockable),
        AnthemSubjectAst::Filter(filter) => StaticAbilityAst::GrantKeywordAction {
            filter,
            action: KeywordAction::Unblockable,
            condition: None,
        },
    };
    Ok(Some(ability))
}

pub(crate) fn parse_subject_has_keywords_and_cant_be_blocked_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if clause_words.is_empty() {
        return Ok(None);
    }

    let Some(head) = anthem_grant_grammar::parse_has_keyword_unblockable_head(tokens) else {
        return Ok(None);
    };
    let has_idx = head.has_token;

    let ability_tokens = trim_edge_punctuation(&tokens[has_idx + 1..]);
    let Some(parsed_tail) = parse_keywords_and_cant_be_blocked_clause(&ability_tokens) else {
        return Ok(None);
    };
    let keyword_tokens = parsed_tail.keyword_tokens;
    let Some(actions) = parse_ability_line(&keyword_tokens) else {
        return Ok(None);
    };
    reject_unimplemented_keyword_actions(&actions, &clause_words.join(" "))?;
    let keyword_actions = actions
        .into_iter()
        .filter(|action| action.lowers_to_static_ability())
        .collect::<Vec<_>>();
    if keyword_actions.is_empty() {
        return Ok(None);
    }

    let (condition, subject_start) = match parse_anthem_prefix_condition(tokens, has_idx) {
        Ok(parsed) => parsed,
        Err(_) => return Ok(None),
    };
    let subject_tokens = trim_commas(&tokens[subject_start..has_idx]);
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    let subject = match parse_anthem_subject(&subject_tokens) {
        Ok(subject) => subject,
        Err(_) => return Ok(None),
    };

    let mut granted = Vec::new();
    for action in keyword_actions
        .into_iter()
        .chain(std::iter::once(KeywordAction::Unblockable))
    {
        granted.push(match &subject {
            AnthemSubjectAst::Source => match &condition {
                Some(condition) => StaticAbilityAst::ConditionalKeywordAction {
                    action,
                    condition: condition.clone(),
                },
                None => StaticAbilityAst::KeywordAction(action),
            },
            AnthemSubjectAst::Filter(filter) => StaticAbilityAst::GrantKeywordAction {
                filter: filter.clone(),
                action,
                condition: condition.clone(),
            },
        });
    }

    Ok(Some(granted))
}

pub(crate) fn parse_landwalk_as_though_block_override_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let Some(parsed) = parse_landwalk_block_override_clause(tokens) else {
        return Ok(None);
    };
    if !is_landwalk_ability_word(parsed.ability_word) {
        return Ok(None);
    }

    let AnthemSubjectAst::Filter(filter) = parse_anthem_subject(parsed.subject_tokens)? else {
        return Ok(None);
    };

    let removed = StaticAbility::keyword_marker(parsed.ability_word);
    Ok(Some(StaticAbilityAst::Static(
        StaticAbility::remove_ability(filter, removed),
    )))
}

fn is_landwalk_ability_word(word: &str) -> bool {
    matches!(
        parse_single_word_keyword_action(word),
        Some(KeywordAction::Landwalk(_))
    )
}

pub(crate) fn parse_subject_cant_be_blocked_as_long_as_condition_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let Some(parsed) = parse_cant_be_blocked_as_long_as_clause(tokens) else {
        return Ok(None);
    };
    let subject_tokens = parsed.subject_tokens;
    let condition = parse_static_condition_clause(parsed.condition_tokens)?;

    let subject_words = crate::runtime_backend::token_word_refs(&subject_tokens);
    let subject = first_spell_each_turn_subject(&subject_words)
        .map(Ok)
        .unwrap_or_else(|| parse_anthem_subject(&subject_tokens))?;
    let granted = match subject {
        AnthemSubjectAst::Source => StaticAbilityAst::ConditionalKeywordAction {
            action: KeywordAction::Unblockable,
            condition,
        },
        AnthemSubjectAst::Filter(filter) => StaticAbilityAst::GrantKeywordAction {
            filter,
            action: KeywordAction::Unblockable,
            condition: Some(condition),
        },
    };
    Ok(Some(granted))
}

fn simple_card_types_from_control_filter(mut filter: ObjectFilter) -> Option<Vec<CardType>> {
    let mut card_types = if filter.all_card_types.is_empty() {
        Vec::new()
    } else {
        std::mem::take(&mut filter.all_card_types)
    };

    if !filter.card_types.is_empty() {
        if card_types.is_empty() {
            card_types = std::mem::take(&mut filter.card_types);
        } else if filter.card_types.len() == card_types.len()
            && filter
                .card_types
                .iter()
                .all(|card_type| card_types.contains(card_type))
        {
            filter.card_types.clear();
        } else {
            return None;
        }
    }

    if card_types.is_empty()
        || !card_types.iter().all(|card_type| {
            matches!(
                card_type,
                CardType::Artifact
                    | CardType::Battle
                    | CardType::Creature
                    | CardType::Enchantment
                    | CardType::Land
                    | CardType::Planeswalker
            )
        })
    {
        return None;
    }

    filter.zone = None;
    (filter == ObjectFilter::default()).then_some(card_types)
}

fn defending_player_controlled_card_types_from_condition_tokens(
    condition_tokens: &[OwnedLexToken],
) -> Option<Vec<CardType>> {
    let condition = crate::runtime_backend::grammar::conditions::parse_control_condition(
        condition_tokens,
        crate::runtime_backend::grammar::conditions::ControlConditionOptions {
            allow_that_player: false,
            allow_opponent_players: false,
            allow_defending_player: true,
            bind_filter_controller_to_subject: false,
            allow_different_powers_tail: false,
            default_filter_zone: Some(Zone::Battlefield),
        },
    )?;

    if condition.player_filter != Some(PlayerFilter::Defending)
        || condition.requires_different_powers
        || condition.at_least_count()? > 1
    {
        return None;
    }

    simple_card_types_from_control_filter(condition.filter)
}

pub(crate) fn parse_subject_cant_be_blocked_as_long_as_defending_player_controls_card_type_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let Some(parsed) = parse_cant_be_blocked_as_long_as_clause(tokens) else {
        return Ok(None);
    };
    let Some(card_types) =
        defending_player_controlled_card_types_from_condition_tokens(parsed.condition_tokens)
    else {
        return Ok(None);
    };

    let subject_words =
        crate::runtime_backend::lexer::parser_token_word_refs(parsed.subject_tokens);
    let subject = first_spell_each_turn_subject(&subject_words)
        .map(Ok)
        .unwrap_or_else(|| parse_anthem_subject(parsed.subject_tokens))?;
    let unblockable = if card_types.len() == 1 {
        StaticAbility::cant_be_blocked_as_long_as_defending_player_controls_card_type(card_types[0])
    } else {
        StaticAbility::cant_be_blocked_as_long_as_defending_player_controls_card_types(card_types)
    };
    let ability = match subject {
        AnthemSubjectAst::Source => StaticAbilityAst::Static(unblockable),
        AnthemSubjectAst::Filter(filter) => StaticAbilityAst::GrantStaticAbility {
            filter,
            ability: Box::new(StaticAbilityAst::Static(unblockable)),
            condition: None,
        },
    };
    Ok(Some(ability))
}

pub(crate) fn parse_granted_keyword_static_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    fn extract_grant_spec_from_subject(
        subject_tokens: &[OwnedLexToken],
        grantable: crate::grant::Grantable,
    ) -> Result<Option<crate::grant::GrantSpec>, CardTextError> {
        let subject = parse_anthem_subject(subject_tokens)?;
        let AnthemSubjectAst::Filter(mut filter) = subject else {
            return Ok(None);
        };
        let zone = filter.zone.unwrap_or(Zone::Battlefield);
        filter.zone = None;
        Ok(Some(crate::grant::GrantSpec::new(grantable, filter, zone)))
    }

    fn parse_granted_escape_cost_tail(
        trailing_tokens: &[OwnedLexToken],
    ) -> Result<Option<u32>, CardTextError> {
        let trailing_word_refs = crate::runtime_backend::token_word_refs(trailing_tokens);
        let Some(parsed) = parse_granted_escape_cost_tail_clause(trailing_tokens) else {
            return Ok(None);
        };

        let Some((count, used)) = parse_number(parsed.exile_count_tokens) else {
            return Err(CardTextError::ParseError(format!(
                "escape cost clause missing exile count (clause: '{}')",
                trailing_word_refs.join(" ")
            )));
        };
        if used != parsed.exile_count_tokens.len() {
            return Err(CardTextError::ParseError(format!(
                "unsupported escape cost clause (clause: '{}')",
                trailing_word_refs.join(" ")
            )));
        }
        Ok(Some(count as u32))
    }

    fn parse_granted_miracle_cost_reduction_tail(
        trailing_tokens: &[OwnedLexToken],
    ) -> Result<Option<u32>, CardTextError> {
        let trailing_word_refs = crate::runtime_backend::token_word_refs(trailing_tokens);
        let Some(parsed) = parse_granted_miracle_cost_reduction_tail_clause(trailing_tokens) else {
            return Ok(None);
        };

        let Some((cost, used)) =
            crate::runtime_backend::front_end::shared::util::leading_mana_cost_from_tokens(
                parsed.reduction_cost_tokens,
            )
        else {
            return Err(CardTextError::ParseError(format!(
                "unsupported miracle cost reduction clause (clause: '{}')",
                trailing_word_refs.join(" ")
            )));
        };
        if used != parsed.reduction_cost_tokens.len() {
            return Err(CardTextError::ParseError(format!(
                "unsupported miracle cost reduction clause (clause: '{}')",
                trailing_word_refs.join(" ")
            )));
        }
        let generic = cost.generic_mana_total();
        if generic == 0 || cost.mana_value() != generic {
            return Err(CardTextError::ParseError(format!(
                "unsupported miracle cost reduction clause (clause: '{}')",
                trailing_word_refs.join(" ")
            )));
        }
        Ok(Some(generic))
    }

    fn parse_granted_alternative_cast_static(
        subject_tokens: &[OwnedLexToken],
        keyword_tokens: &[OwnedLexToken],
        trailing_tokens: &[OwnedLexToken],
        condition: Option<crate::ConditionExpr>,
    ) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
        let keyword_words = crate::runtime_backend::token_word_refs(keyword_tokens);
        let spec = match parse_granted_alternative_cast_keyword(&keyword_words) {
            Some(GrantedAlternativeCastKeyword::Flashback) => {
                if !anthem_grant_grammar::parse_granted_flashback_cost_equals_mana(trailing_tokens)
                {
                    return Ok(None);
                }
                extract_grant_spec_from_subject(
                    subject_tokens,
                    crate::grant::Grantable::flashback_from_cards_mana_cost(),
                )?
            }
            Some(GrantedAlternativeCastKeyword::Blitz) => {
                if !is_granted_blitz_cost_tail(trailing_tokens) {
                    return Ok(None);
                }
                return granted_blitz_abilities_from_subject(subject_tokens, condition);
            }
            Some(GrantedAlternativeCastKeyword::Emerge) => {
                if !is_granted_emerge_cost_tail(trailing_tokens) {
                    return Ok(None);
                }
                return granted_emerge_abilities_from_subject(subject_tokens, condition);
            }
            Some(GrantedAlternativeCastKeyword::Miracle) => {
                let Some(reduction) = parse_granted_miracle_cost_reduction_tail(trailing_tokens)?
                else {
                    return Ok(None);
                };
                extract_grant_spec_from_subject(
                    subject_tokens,
                    crate::grant::Grantable::miracle_from_cards_mana_cost_reduced_by(reduction),
                )?
            }
            Some(GrantedAlternativeCastKeyword::Escape) => {
                let Some(exile_count) = parse_granted_escape_cost_tail(trailing_tokens)? else {
                    return Ok(None);
                };
                extract_grant_spec_from_subject(
                    subject_tokens,
                    crate::grant::Grantable::escape(exile_count),
                )?
            }
            None => None,
        };

        let Some(spec) = spec else {
            return Ok(None);
        };

        let mut ability = StaticAbilityAst::Static(StaticAbility::grants(spec));
        if let Some(condition) = condition {
            ability = StaticAbilityAst::ConditionalStaticAbility {
                ability: Box::new(ability),
                condition,
            };
        }
        Ok(Some(vec![ability]))
    }

    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    let Some(verb_facts) = anthem_grant_grammar::parse_granted_keyword_verb_facts(tokens) else {
        return Ok(None);
    };
    let have_token_idx = verb_facts.have_token;
    if verb_facts.prefix_has_get {
        return Ok(None);
    }

    if verb_facts.starts_with_as_long_as {
        if !verb_facts.tail_has_have && verb_facts.tail_has_get_or_be {
            return Ok(None);
        }
    }

    let (prefix_condition, subject_start) =
        match parse_anthem_prefix_condition(tokens, have_token_idx) {
            Ok(parsed) => parsed,
            Err(_) => return Ok(None),
        };
    let subject_tokens = trim_commas(&tokens[subject_start..have_token_idx]);
    if subject_tokens.is_empty() {
        return Ok(None);
    }

    if anthem_grant_grammar::granted_keyword_subject_is_rejected(&subject_tokens) {
        return Ok(None);
    }

    let tail_tokens = trim_edge_punctuation(&tokens[have_token_idx + 1..]);
    if tail_tokens.is_empty() {
        return Ok(None);
    }

    let mut tail_tokens = tail_tokens;
    let mut trailing_clause_tokens: Vec<OwnedLexToken> = Vec::new();
    let tail_sentences =
        crate::runtime_backend::grammar::primitives::split_lexed_slices_on_period(&tail_tokens);
    if tail_sentences.len() > 1 {
        let leading = trim_edge_punctuation(tail_sentences[0]);
        let trailing = tail_sentences[1..]
            .iter()
            .flat_map(|sentence| trim_edge_punctuation(sentence))
            .collect::<Vec<_>>();
        trailing_clause_tokens = trailing;
        tail_tokens = leading;
    }

    let mut keyword_tokens = tail_tokens.clone();
    let mut suffix_condition = None;
    if let Some(split) = anthem_grant_grammar::split_trailing_as_long_as_clause(&tail_tokens) {
        keyword_tokens = split.keyword_tokens.to_vec();
        suffix_condition = Some(parse_static_condition_clause(split.condition_tokens)?);
    }
    if keyword_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing granted keyword list (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let mut grants_must_attack = false;
    if let Some(split) = anthem_grant_grammar::split_must_attack_keyword_tail(&keyword_tokens) {
        keyword_tokens = split.keyword_tokens.to_vec();
        grants_must_attack = true;
    }
    if keyword_tokens.is_empty() {
        return Ok(None);
    }

    let condition = match (prefix_condition, suffix_condition) {
        (Some(_), Some(_)) => {
            return Err(CardTextError::ParseError(format!(
                "multiple static conditions are not supported in granted-keyword clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        (Some(cond), None) | (None, Some(cond)) => Some(cond),
        (None, None) => None,
    };

    let keyword_kind = anthem_grant_grammar::classify_granted_keyword_tokens(&keyword_tokens);
    if keyword_kind == anthem_grant_grammar::GrantedKeywordTokenKind::Blitz
        && (trailing_clause_tokens.is_empty()
            || is_granted_blitz_cost_tail(&trailing_clause_tokens))
    {
        return granted_blitz_abilities_from_subject(&subject_tokens, condition);
    }
    if keyword_kind == anthem_grant_grammar::GrantedKeywordTokenKind::Emerge
        && (trailing_clause_tokens.is_empty()
            || is_granted_emerge_cost_tail(&trailing_clause_tokens))
    {
        return granted_emerge_abilities_from_subject(&subject_tokens, condition);
    }

    if !trailing_clause_tokens.is_empty() {
        if let Some(compiled) = parse_granted_alternative_cast_static(
            &subject_tokens,
            &keyword_tokens,
            &trailing_clause_tokens,
            condition.clone(),
        )? {
            return Ok(Some(compiled));
        }

        let ignore_keyword_reminder =
            keyword_kind == anthem_grant_grammar::GrantedKeywordTokenKind::IgnoredReminder;
        if !ignore_keyword_reminder {
            if parse_ability_line(&keyword_tokens).is_some() {
                return Err(CardTextError::ParseError(format!(
                    "unsupported trailing granted-keyword clause (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            return Ok(None);
        }
    }

    if let Some(compiled) = parse_color_filtered_keyword_grants(
        &subject_tokens,
        &keyword_tokens,
        condition.clone(),
        &clause_words.join(" "),
    )? {
        return Ok(Some(compiled));
    }

    if keyword_kind == anthem_grant_grammar::GrantedKeywordTokenKind::Exploit {
        let subject = parse_anthem_subject(&subject_tokens)?;
        return Ok(Some(vec![grant_exploit_for_anthem_subject(
            &subject, condition,
        )]));
    }

    let attached_subject_filter =
        infer_attached_subject_filter_from_condition_expr(condition.as_ref());
    let subject_words = crate::runtime_backend::token_word_refs(&subject_tokens);
    let subject = first_spell_each_turn_subject(&subject_words)
        .map(Ok)
        .unwrap_or_else(|| {
            parse_anthem_subject_with_attached_fallback(
                &subject_tokens,
                attached_subject_filter.as_ref(),
            )
        })?;

    if let Some((actions, subtypes)) = parse_keyword_and_subtype_addition_tail(&keyword_tokens)? {
        reject_unimplemented_keyword_actions(&actions, &clause_words.join(" "))?;
        let mut compiled = Vec::new();
        for action in actions {
            if !action.lowers_to_static_ability() {
                return Ok(None);
            }
            match &subject {
                AnthemSubjectAst::Source => match &condition {
                    Some(condition) => compiled.push(StaticAbilityAst::ConditionalKeywordAction {
                        action,
                        condition: condition.clone(),
                    }),
                    None => compiled.push(StaticAbilityAst::KeywordAction(action)),
                },
                AnthemSubjectAst::Filter(filter) => {
                    compiled.push(StaticAbilityAst::GrantKeywordAction {
                        filter: filter.clone(),
                        action,
                        condition: condition.clone(),
                    });
                }
            }
        }

        match &subject {
            AnthemSubjectAst::Source => {
                compiled.push(conditional_static_ability(
                    StaticAbility::add_subtypes(ObjectFilter::source(), subtypes),
                    condition,
                ));
            }
            AnthemSubjectAst::Filter(filter) => {
                compiled.push(conditional_static_ability(
                    StaticAbility::add_subtypes(filter.clone(), subtypes),
                    condition,
                ));
            }
        }
        return Ok(Some(compiled));
    }

    let Some(actions) = parse_ability_line(&keyword_tokens) else {
        return Ok(None);
    };
    reject_unimplemented_keyword_actions(&actions, &clause_words.join(" "))?;
    if actions.is_empty() {
        return Ok(None);
    }

    let grants_conspire = actions
        .iter()
        .filter(|action| matches!(action, KeywordAction::Conspire))
        .count();
    if grants_conspire > 0 {
        let mut compiled = Vec::new();
        for _ in 0..grants_conspire {
            match &subject {
                AnthemSubjectAst::Source => {
                    let ability =
                        StaticAbilityAst::Static(StaticAbility::keyword_marker("Conspire"));
                    if let Some(condition) = &condition {
                        compiled.push(StaticAbilityAst::ConditionalStaticAbility {
                            ability: Box::new(ability),
                            condition: condition.clone(),
                        });
                    } else {
                        compiled.push(ability);
                    }
                }
                AnthemSubjectAst::Filter(filter) => {
                    compiled.push(StaticAbilityAst::GrantStaticAbility {
                        filter: filter.clone(),
                        ability: Box::new(StaticAbilityAst::Static(StaticAbility::keyword_marker(
                            "Conspire",
                        ))),
                        condition: condition.clone(),
                    });
                }
            }
        }
        return Ok(Some(compiled));
    }

    let mut mapped = Vec::new();
    let mut object_ability_grants = Vec::new();
    for action in actions {
        if action.lowers_to_static_ability() {
            mapped.push(action);
        } else if let Some(granted) = granted_object_ability_for_keyword_action(&action) {
            object_ability_grants.push(granted);
        } else {
            return Ok(None);
        }
    }
    if mapped.is_empty() && object_ability_grants.is_empty() && !grants_must_attack {
        return Ok(None);
    }

    let mut compiled = Vec::new();
    if grants_must_attack {
        match &subject {
            AnthemSubjectAst::Source => {
                if let Some(condition) = &condition {
                    compiled.push(StaticAbilityAst::ConditionalStaticAbility {
                        ability: Box::new(StaticAbilityAst::Static(StaticAbility::must_attack())),
                        condition: condition.clone(),
                    });
                } else {
                    compiled.push(StaticAbilityAst::Static(StaticAbility::must_attack()));
                }
            }
            AnthemSubjectAst::Filter(filter) => {
                compiled.push(StaticAbilityAst::GrantStaticAbility {
                    filter: filter.clone(),
                    ability: Box::new(StaticAbilityAst::Static(StaticAbility::must_attack())),
                    condition: condition.clone(),
                })
            }
        }
    }
    for action in mapped {
        let ast = match &subject {
            AnthemSubjectAst::Source => match &condition {
                Some(condition) => StaticAbilityAst::ConditionalKeywordAction {
                    action,
                    condition: condition.clone(),
                },
                None => StaticAbilityAst::KeywordAction(action),
            },
            AnthemSubjectAst::Filter(filter) => StaticAbilityAst::GrantKeywordAction {
                filter: filter.clone(),
                action,
                condition: condition.clone(),
            },
        };
        compiled.push(ast);
    }
    let grant_clause = ParsedAnthemClause {
        subject,
        power: AnthemValue::Fixed(0),
        toughness: AnthemValue::Fixed(0),
        condition,
        count_uses_where_x: false,
    };
    for (ability, display) in object_ability_grants {
        compiled.push(grant_object_ability_for_anthem_subject(
            &grant_clause,
            ability,
            display,
        ));
    }
    Ok(Some(compiled))
}

pub(crate) fn parse_all_creatures_lose_flying_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    if anthem_grant_grammar::parse_all_creatures_lose_flying(tokens) {
        return Ok(Some(StaticAbilityAst::RemoveKeywordAction {
            filter: ObjectFilter::creature(),
            action: KeywordAction::Flying,
        }));
    }
    Ok(None)
}

pub(crate) fn parse_subject_loses_keywords_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let Some(parsed) = anthem_grant_grammar::parse_subject_loses_keywords_clause(tokens) else {
        return Ok(None);
    };
    let subject_tokens = parsed.subject_tokens;
    let filter = match parse_object_filter(&subject_tokens, false) {
        Ok(filter) => filter,
        Err(_) => return Ok(None),
    };

    let Some(loss_actions) = parse_ability_line(parsed.loss_tokens) else {
        return Ok(None);
    };

    let mut actions = loss_actions;
    if let Some(gain_tokens) = parsed.additional_gain_tokens {
        let Some(gain_actions) = parse_ability_line(gain_tokens) else {
            return Ok(None);
        };
        actions.extend(gain_actions);
    }

    let clause_text = crate::runtime_backend::token_word_refs(tokens).join(" ");
    reject_unimplemented_keyword_actions(&actions, &clause_text)?;

    let mut result = Vec::new();
    for action in actions {
        if !action.lowers_to_static_ability() {
            return Ok(None);
        }
        if result.iter().any(|existing| {
            matches!(
                existing,
                StaticAbilityAst::RemoveKeywordAction {
                    filter: existing_filter,
                    action: existing_action,
                } if existing_filter == &filter && existing_action == &action
            )
        }) {
            continue;
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

pub(crate) fn parse_each_creature_cant_be_blocked_by_more_than_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let Some(parsed) = parse_cant_be_blocked_by_more_than_clause(tokens) else {
        return Ok(None);
    };
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    let Some(subject) = anthem_grant_grammar::parse_each_creature_subject(parsed.subject_tokens)
    else {
        return Ok(None);
    };
    let Some((minimum_blockers, used)) = parse_greater_than_or_equal_quantity_prefix(
        parsed.blocker_threshold_tokens,
        false,
        false,
        "cant-be-blocked blocker threshold",
    )?
    else {
        return Ok(None);
    };
    if minimum_blockers == 0 || used != parsed.blocker_threshold_tokens.len() {
        return Ok(None);
    }
    let amount = minimum_blockers - 1;
    let filter_tokens = subject.filter_tokens;
    if filter_tokens.is_empty() {
        return Ok(None);
    }
    let filter = parse_object_filter(&filter_tokens, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported cant-be-blocked-by-more-than subject (clause: '{}')",
            clause_words.join(" ")
        ))
    })?;
    let granted = StaticAbility::cant_be_blocked_by_more_than(amount as usize);
    Ok(Some(StaticAbilityAst::GrantStaticAbility {
        filter,
        ability: Box::new(StaticAbilityAst::Static(granted)),
        condition: None,
    }))
}

pub(crate) fn parse_each_creature_can_block_additional_creature_each_combat_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    // High Ground: "Each creature can block an additional creature each combat."
    let Some(parsed) = parse_can_block_additional_creature_clause(tokens) else {
        return Ok(None);
    };
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    let Some(subject) = anthem_grant_grammar::parse_each_creature_subject(parsed.subject_tokens)
    else {
        return Ok(None);
    };

    let Some(additional) =
        anthem_grant_grammar::parse_additional_creature_count(parsed.additional_count_tokens)
    else {
        return Ok(None);
    };

    let filter_tokens = subject.filter_tokens;
    let filter = parse_object_filter(filter_tokens, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported can-block-additional subject (clause: '{}')",
            clause_words.join(" ")
        ))
    })?;
    let granted = StaticAbility::can_block_additional_creature_each_combat(additional);
    Ok(Some(StaticAbilityAst::GrantStaticAbility {
        filter,
        ability: Box::new(StaticAbilityAst::Static(granted)),
        condition: None,
    }))
}

pub(crate) fn parse_lose_all_abilities_and_transform_base_pt_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    fn title_case_words(words: &[&str]) -> String {
        words
            .iter()
            .map(|word| {
                let mut chars = word.chars();
                if let Some(first) = chars.next() {
                    let mut out = String::new();
                    out.extend(first.to_uppercase());
                    out.push_str(chars.as_str());
                    out
                } else {
                    String::new()
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    let word_view = TokenWordView::new(tokens);
    let words = word_view.word_refs();
    let Some(shape) = anthem_grant_grammar::parse_lose_all_transform_shape(tokens) else {
        return Ok(None);
    };
    let Some(pt_word) = words.get(shape.power_toughness_word) else {
        return Ok(None);
    };
    let (power, toughness) = parse_pt_modifier(pt_word).map_err(|_| {
        CardTextError::ParseError(format!(
            "invalid base power/toughness value (clause: '{}')",
            words.join(" ")
        ))
    })?;

    let subject_token_end = word_view
        .token_index_after_words(shape.subject_word_end)
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "invalid subject span in lose-all-abilities transform clause (clause: '{}')",
                words.join(" ")
            ))
        })?;
    let subject_tokens = trim_commas(tokens.get(..subject_token_end).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "invalid subject token span in lose-all-abilities transform clause (clause: '{}')",
            words.join(" ")
        ))
    })?);
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    let filter = parse_object_filter(&subject_tokens, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported subject in lose-all-abilities transform clause (clause: '{}')",
            words.join(" ")
        ))
    })?;

    let descriptor_span = words.get(shape.descriptor_words.clone()).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "invalid descriptor span in lose-all-abilities transform clause (clause: '{}')",
            words.join(" ")
        ))
    })?;
    let mut descriptor_words = non_article_word_refs_except(descriptor_span, &["and"]);
    if descriptor_words.is_empty() {
        return Ok(None);
    }
    if descriptor_words.first().copied() == Some("all") {
        descriptor_words.remove(0);
    }
    if descriptor_words.is_empty() {
        return Ok(None);
    }

    let mut set_colors = ColorSet::new();
    let mut set_card_types: Vec<CardType> = Vec::new();
    let mut creature_subtypes: Vec<Subtype> = Vec::new();

    for descriptor in descriptor_words {
        if let Some(color) = parse_color(descriptor) {
            set_colors = set_colors.union(color);
            continue;
        }
        if let Some(card_type) = parse_card_type(descriptor) {
            if !set_card_types.iter().any(|existing| *existing == card_type) {
                set_card_types.push(card_type);
            }
            continue;
        }
        if let Some(subtype) = parse_subtype_flexible(descriptor) {
            if !creature_subtypes
                .iter()
                .any(|existing| *existing == subtype)
            {
                creature_subtypes.push(subtype);
            }
            continue;
        }
        return Err(CardTextError::ParseError(format!(
            "unsupported transform descriptor '{}' (clause: '{}')",
            descriptor,
            words.join(" ")
        )));
    }

    if !creature_subtypes.is_empty()
        && !set_card_types
            .iter()
            .any(|existing| *existing == CardType::Creature)
    {
        set_card_types.push(CardType::Creature);
    }

    let set_name = shape
        .name_words
        .as_ref()
        .map(|range| {
            words
                .get(range.clone())
                .map(title_case_words)
                .ok_or_else(|| {
                    CardTextError::ParseError(format!(
                        "invalid name span in lose-all-abilities transform clause (clause: '{}')",
                        words.join(" ")
                    ))
                })
        })
        .transpose()?;

    let mut abilities = vec![if shape.except_mana_abilities {
        StaticAbility::remove_all_abilities_except_mana(filter.clone())
    } else {
        StaticAbility::remove_all_abilities(filter.clone())
    }];

    if !set_card_types.is_empty() {
        abilities.push(StaticAbility::set_card_types(
            filter.clone(),
            set_card_types,
        ));
    }
    if !creature_subtypes.is_empty() {
        abilities.push(StaticAbility::set_creature_subtypes(
            filter.clone(),
            creature_subtypes,
        ));
    }
    if !set_colors.is_empty() {
        abilities.push(StaticAbility::set_colors(filter.clone(), set_colors));
    }
    if let Some(name) = set_name {
        abilities.push(StaticAbility::set_name(filter.clone(), name));
    }
    abilities.push(StaticAbility::set_base_power_toughness(
        filter, power, toughness,
    ));

    Ok(Some(abilities))
}

pub(crate) fn parse_lose_all_abilities_and_base_pt_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    let Some(shape) = anthem_grant_grammar::parse_lose_all_abilities_shape(tokens) else {
        return Ok(None);
    };
    if shape.becomes {
        return Err(CardTextError::ParseError(format!(
            "unsupported lose-all-abilities static becomes clause (clause: '{}')",
            words.join(" ")
        )));
    }

    let subject_tokens = &tokens[..shape.subject_word_end];
    let filter = parse_object_filter(subject_tokens, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported subject in lose-all-abilities clause (clause: '{}')",
            words.join(" ")
        ))
    })?;

    let mut abilities = vec![if shape.except_mana_abilities {
        StaticAbility::remove_all_abilities_except_mana(filter.clone())
    } else {
        StaticAbility::remove_all_abilities(filter.clone())
    }];

    if let Some(modifier_word) = shape.base_power_toughness_word {
        if let Some(modifier_token) = words.get(modifier_word)
            && let Ok((power, toughness)) = parse_pt_modifier(modifier_token)
        {
            abilities.push(StaticAbility::set_base_power_toughness(
                filter, power, toughness,
            ));
        }
    }

    Ok(Some(abilities))
}

pub(crate) fn parse_all_have_indestructible_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    let Some(parsed) = anthem_grant_grammar::parse_indestructible_grant_clause(tokens) else {
        return Ok(None);
    };
    let Some(actions) = parse_ability_line(parsed.ability_tokens) else {
        return Ok(None);
    };
    reject_unimplemented_keyword_actions(&actions, &words.join(" "))?;
    if actions.len() != 1
        || !actions
            .first()
            .is_some_and(|action| matches!(action, KeywordAction::Indestructible))
    {
        return Ok(None);
    }

    let filter = parse_object_filter(parsed.subject_tokens, false)?;
    Ok(Some(StaticAbilityAst::GrantKeywordAction {
        filter,
        action: KeywordAction::Indestructible,
        condition: None,
    }))
}

#[derive(Debug, Clone)]
pub(crate) enum AnthemSubjectAst {
    Source,
    Filter(ObjectFilter),
}

#[derive(Debug, Clone)]
pub(crate) struct ParsedAnthemClause {
    pub(crate) subject: AnthemSubjectAst,
    pub(crate) power: AnthemValue,
    pub(crate) toughness: AnthemValue,
    pub(crate) condition: Option<crate::ConditionExpr>,
    /// Whether the scaling count was written as "where X is …" (vs "for each …")
    /// in the original oracle text. Surface hint preserved for rendering.
    pub(crate) count_uses_where_x: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AnimationSubtypeMode {
    Add,
    ReplaceCreatureTypes,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ParsedGrantedTailAst {
    pub(crate) granted_static: Vec<StaticAbilityAst>,
    pub(crate) granted_keyword_actions: Vec<KeywordAction>,
    pub(crate) granted_object_abilities: Vec<(ParsedAbility, String)>,
}

#[derive(Debug, Clone)]
pub(crate) struct StaticAnimationBundleAst {
    pub(crate) subject: AnthemSubjectAst,
    pub(crate) condition: Option<crate::ConditionExpr>,
    pub(crate) ensure_creature_type: bool,
    pub(crate) subtypes: Vec<Subtype>,
    pub(crate) subtype_mode: AnimationSubtypeMode,
    pub(crate) base_power_toughness: Option<(Value, Value)>,
    pub(crate) granted_tail: ParsedGrantedTailAst,
}

fn is_granted_blitz_cost_tail(trailing_tokens: &[OwnedLexToken]) -> bool {
    anthem_grant_grammar::parse_granted_blitz_cost_equals_mana(trailing_tokens)
}

fn is_granted_emerge_cost_tail(trailing_tokens: &[OwnedLexToken]) -> bool {
    anthem_grant_grammar::parse_granted_emerge_cost_equals_mana(trailing_tokens)
}

fn normalize_granted_alternative_spell_filter(
    mut filter: ObjectFilter,
) -> (ObjectFilter, Vec<Zone>) {
    let parsed_zone = filter.zone.unwrap_or(Zone::Battlefield);
    if parsed_zone == Zone::Stack || filter.stack_kind.is_some() {
        filter.zone = None;
        filter.stack_kind = None;
        filter.cast_by = None;
        return (filter, vec![Zone::Hand, Zone::Exile, Zone::Graveyard]);
    }

    filter.zone = None;
    (filter, vec![parsed_zone])
}

fn granted_blitz_abilities_from_subject(
    subject_tokens: &[OwnedLexToken],
    condition: Option<crate::ConditionExpr>,
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let subject = parse_anthem_subject(&subject_tokens)?;
    let AnthemSubjectAst::Filter(filter) = subject else {
        return Ok(None);
    };
    let (filter, zones) = normalize_granted_alternative_spell_filter(filter);
    let mut abilities = Vec::new();
    for zone in zones {
        let spec = crate::grant::GrantSpec::new(
            crate::grant::Grantable::blitz_from_cards_mana_cost(),
            filter.clone(),
            zone,
        );
        let mut ability = StaticAbilityAst::Static(StaticAbility::grants(spec));
        if let Some(condition) = condition.clone() {
            ability = StaticAbilityAst::ConditionalStaticAbility {
                ability: Box::new(ability),
                condition,
            };
        }
        abilities.push(ability);
    }
    Ok(Some(abilities))
}

fn granted_emerge_abilities_from_subject(
    subject_tokens: &[OwnedLexToken],
    condition: Option<crate::ConditionExpr>,
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let subject = parse_anthem_subject(subject_tokens)?;
    let AnthemSubjectAst::Filter(filter) = subject else {
        return Ok(None);
    };
    let (filter, zones) = if anthem_grant_grammar::emerge_subject_is_spell_cast(subject_tokens) {
        let mut filter = filter;
        filter.zone = None;
        filter.stack_kind = None;
        filter.cast_by = None;
        (filter, vec![Zone::Hand])
    } else {
        normalize_granted_alternative_spell_filter(filter)
    };
    let mut abilities = Vec::new();
    for zone in zones {
        if zone != Zone::Hand {
            continue;
        }
        let spec = crate::grant::GrantSpec::new(
            crate::grant::Grantable::emerge_from_cards_mana_cost(),
            filter.clone(),
            zone,
        );
        let mut ability = StaticAbilityAst::Static(StaticAbility::grants(spec));
        if let Some(condition) = condition.clone() {
            ability = StaticAbilityAst::ConditionalStaticAbility {
                ability: Box::new(ability),
                condition,
            };
        }
        abilities.push(ability);
    }
    Ok((!abilities.is_empty()).then_some(abilities))
}

fn parse_keyword_and_subtype_addition_tail(
    keyword_tokens: &[OwnedLexToken],
) -> Result<Option<(Vec<KeywordAction>, Vec<Subtype>)>, CardTextError> {
    let Some(split) = anthem_grant_grammar::split_keyword_and_type_addition(keyword_tokens) else {
        return Ok(None);
    };
    let Some(actions) = parse_ability_line(split.keyword_tokens) else {
        return Ok(None);
    };
    if actions.is_empty() {
        return Ok(None);
    }

    let Some(additions) = parse_type_color_addition_clause(split.addition_tokens)? else {
        return Ok(None);
    };
    if !additions.added_colors.is_empty()
        || !additions.set_colors.is_empty()
        || !additions.card_types.is_empty()
        || additions.subtypes.is_empty()
    {
        return Ok(None);
    }

    Ok(Some((actions, additions.subtypes)))
}

fn conditional_static_ability(
    ability: StaticAbility,
    condition: Option<crate::ConditionExpr>,
) -> StaticAbilityAst {
    let ast = StaticAbilityAst::Static(ability);
    match condition {
        Some(condition) => StaticAbilityAst::ConditionalStaticAbility {
            ability: Box::new(ast),
            condition,
        },
        None => ast,
    }
}

pub(crate) fn parse_source_counter_threshold_keyword_and_subtype_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let Some(head) = anthem_grant_grammar::parse_source_counter_threshold_head(tokens) else {
        return Ok(None);
    };
    let have_token_idx = head.have_token;
    let Ok((Some(condition), subject_start)) =
        parse_anthem_prefix_condition(tokens, have_token_idx)
    else {
        return Ok(None);
    };

    let subject_tokens = trim_commas(&tokens[subject_start..have_token_idx]);
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    let subject = parse_anthem_subject(&subject_tokens)?;

    let tail_tokens = trim_edge_punctuation(&tokens[have_token_idx + 1..]);
    let Some((actions, subtypes)) = parse_keyword_and_subtype_addition_tail(&tail_tokens)? else {
        return Ok(None);
    };

    let clause_text = crate::runtime_backend::token_word_refs(tokens).join(" ");
    reject_unimplemented_keyword_actions(&actions, &clause_text)?;

    let mut compiled = Vec::new();
    for action in actions {
        if !action.lowers_to_static_ability() {
            return Ok(None);
        }
        match &subject {
            AnthemSubjectAst::Source => compiled.push(StaticAbilityAst::ConditionalKeywordAction {
                action,
                condition: condition.clone(),
            }),
            AnthemSubjectAst::Filter(filter) => {
                compiled.push(StaticAbilityAst::GrantKeywordAction {
                    filter: filter.clone(),
                    action,
                    condition: Some(condition.clone()),
                })
            }
        }
    }

    let subtype_ability = match &subject {
        AnthemSubjectAst::Source => StaticAbility::add_subtypes(ObjectFilter::source(), subtypes),
        AnthemSubjectAst::Filter(filter) => StaticAbility::add_subtypes(filter.clone(), subtypes),
    };
    compiled.push(StaticAbilityAst::ConditionalStaticAbility {
        ability: Box::new(StaticAbilityAst::Static(subtype_ability)),
        condition,
    });

    Ok((!compiled.is_empty()).then_some(compiled))
}

pub(crate) fn find_source_reference_start(tokens: &[OwnedLexToken]) -> Option<usize> {
    let mut token_indices = Vec::new();
    let mut token_words = Vec::new();
    for (idx, token) in tokens.iter().enumerate() {
        if let Some(word) = token.as_word() {
            token_indices.push(idx);
            token_words.push(word);
        }
    }

    for word_start in 0..token_words.len() {
        if is_source_reference_words(&token_words[word_start..]) {
            return token_indices.get(word_start).copied();
        }
    }
    None
}

pub(crate) fn object_filter_specificity_score(filter: &ObjectFilter) -> usize {
    let mut score = 0usize;
    if !filter.any_of.is_empty() {
        score += 12;
        score += filter
            .any_of
            .iter()
            .map(object_filter_specificity_score)
            .sum::<usize>();
    }
    score += filter.tagged_constraints.len() * 20;
    score += filter.card_types.len() * 10;
    score += filter.all_card_types.len() * 10;
    score += filter.subtypes.len() * 8;
    score += filter.excluded_subtypes.len() * 8;
    score += usize::from(filter.controller.is_some()) * 6;
    score += usize::from(filter.owner.is_some()) * 6;
    score += usize::from(filter.zone.is_some()) * 4;
    score += usize::from(filter.other) * 3;
    score += usize::from(filter.token || filter.nontoken) * 3;
    score += usize::from(filter.tapped || filter.untapped) * 2;
    score += usize::from(
        filter.attacking
            || filter.nonattacking
            || filter.blocking
            || filter.nonblocking
            || filter.blocked
            || filter.unblocked,
    ) * 2;
    score += usize::from(filter.is_commander || filter.noncommander) * 2;
    score += usize::from(filter.colorless || filter.multicolored || filter.monocolored) * 2;
    score += usize::from(filter.with_counter.is_some() || filter.without_counter.is_some()) * 4;
    score += usize::from(filter.entered_battlefield_this_turn) * 2;
    score += usize::from(filter.entered_battlefield_controller.is_some()) * 2;
    score += usize::from(filter.was_dealt_damage_this_turn) * 2;
    score += usize::from(filter.dealt_damage_to_player_this_turn.is_some()) * 2;
    score += usize::from(!filter.excluded_card_types.is_empty()) * 2;
    score += usize::from(!filter.excluded_supertypes.is_empty()) * 2;
    score += usize::from(!filter.excluded_colors.is_empty()) * 2;
    score += usize::from(!filter.excluded_static_abilities.is_empty()) * 2;
    score += usize::from(!filter.excluded_ability_markers.is_empty()) * 2;
    score += usize::from(filter.colors.is_some()) * 2;
    score += usize::from(filter.required_colors.is_some()) * 3;
    score += usize::from(filter.sticker.is_some()) * 3;
    score += usize::from(filter.chosen_color) * 3;
    score += usize::from(filter.chosen_creature_type) * 3;
    score += usize::from(filter.excluded_chosen_creature_type) * 3;
    score += usize::from(filter.power.is_some() || filter.toughness.is_some()) * 2;
    score
}

pub(crate) fn parse_best_object_filter_suffix(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    let mut best: Option<(usize, usize, ObjectFilter)> = None;
    for start in 0..tokens.len() {
        if tokens[start].as_word().is_none() {
            continue;
        }
        let mut other = false;
        let mut candidate = &tokens[start..];
        match anthem_grant_grammar::classify_suffix_filter_head(candidate) {
            anthem_grant_grammar::SuffixFilterHead::Other => {
                other = true;
                candidate = &candidate[1..];
            }
            anthem_grant_grammar::SuffixFilterHead::Pronoun => continue,
            anthem_grant_grammar::SuffixFilterHead::Normal => {}
        }
        if candidate.is_empty() {
            continue;
        }
        let Ok(filter) = parse_object_filter(candidate, other) else {
            continue;
        };
        let score = object_filter_specificity_score(&filter);
        if best
            .as_ref()
            .is_none_or(|(best_score, _, _)| score > *best_score)
        {
            best = Some((score, start, filter));
        }
    }
    best.map(|(_, start, filter)| {
        if start > 0 {
            crate::parse_loss::record(
                "suffix_object_filter_recovery",
                format!(
                    "parsed '{}' as suffix of '{}'",
                    crate::runtime_backend::token_word_refs(&tokens[start..]).join(" "),
                    crate::runtime_backend::token_word_refs(tokens).join(" ")
                ),
            );
        }
        filter
    })
}

fn subject_branch_looks_type_like(filter: &ObjectFilter) -> bool {
    !filter.card_types.is_empty()
        || !filter.subtypes.is_empty()
        || !filter.excluded_card_types.is_empty()
        || !filter.excluded_subtypes.is_empty()
}

fn parse_shared_suffix_and_subject_filter(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    let mut best: Option<(usize, ObjectFilter)> = None;

    for candidate in anthem_grant_grammar::parse_shared_suffix_candidates(tokens) {
        let left_branch = trim_commas(&tokens[..candidate.and_token]);
        let right_branch = trim_commas(&tokens[candidate.and_token + 1..candidate.split_token]);
        let shared_suffix = trim_commas(&tokens[candidate.split_token..]);
        if left_branch.is_empty() || right_branch.is_empty() || shared_suffix.is_empty() {
            continue;
        }

        let Ok(left_branch_filter) = parse_object_filter(&left_branch, false) else {
            continue;
        };
        if !subject_branch_looks_type_like(&left_branch_filter) {
            continue;
        }

        let Ok(right_branch_filter) = parse_object_filter(&right_branch, false) else {
            continue;
        };
        if !subject_branch_looks_type_like(&right_branch_filter) {
            continue;
        }

        let mut left_full = left_branch.clone();
        left_full.extend(shared_suffix.iter().cloned());
        let mut right_full = right_branch.clone();
        right_full.extend(shared_suffix.iter().cloned());

        let Ok(left_filter) = parse_object_filter(&left_full, false) else {
            continue;
        };
        let Ok(right_filter) = parse_object_filter(&right_full, false) else {
            continue;
        };
        if left_filter == right_filter {
            continue;
        }

        let mut disjunction = ObjectFilter::default();
        disjunction.any_of = vec![left_filter.clone(), right_filter.clone()];
        let score = object_filter_specificity_score(&left_filter)
            + object_filter_specificity_score(&right_filter)
            + shared_suffix.len();
        if best
            .as_ref()
            .is_none_or(|(best_score, _)| score > *best_score)
        {
            best = Some((score, disjunction));
        }
    }

    best.map(|(_, filter)| filter)
}

pub(crate) fn parse_anthem_subject(
    tokens: &[OwnedLexToken],
) -> Result<AnthemSubjectAst, CardTextError> {
    let subject_words = crate::runtime_backend::lexer::parser_token_word_refs(tokens);
    if let Some(subject) = first_spell_each_turn_subject_tokens(tokens)? {
        return Ok(subject);
    }
    if anthem_grant_grammar::parse_first_spell_each_turn_subject_words(&subject_words) {
        return Ok(AnthemSubjectAst::Filter(
            ObjectFilter::spell()
                .cast_by(PlayerFilter::You)
                .first_spell_cast_each_turn(),
        ));
    }
    if anthem_grant_grammar::is_source_it_subject(tokens) {
        return Ok(AnthemSubjectAst::Source);
    }
    if is_source_reference_words(&subject_words) {
        return Ok(AnthemSubjectAst::Source);
    }
    if let Ok(filter) = parse_object_filter(tokens, false)
        && filter.in_combat_with_source
    {
        return Ok(AnthemSubjectAst::Filter(filter));
    }
    match anthem_grant_grammar::parse_exact_anthem_subject_grammar(tokens) {
        Some(anthem_grant_grammar::AnthemSubjectGrammarMatch::Filter(filter)) => {
            return Ok(AnthemSubjectAst::Filter(filter));
        }
        Some(anthem_grant_grammar::AnthemSubjectGrammarMatch::RejectFragment) => {
            return Err(CardTextError::ParseError(format!(
                "unsupported anthem subject (clause: '{}')",
                crate::runtime_backend::token_word_refs(tokens).join(" ")
            )));
        }
        None => {}
    }
    if let Some(filter) = parse_commander_creatures_you_own_subject(tokens) {
        return Ok(AnthemSubjectAst::Filter(filter));
    }
    if let Some(filter) = parse_enchanted_player_controls_subject(tokens)? {
        return Ok(AnthemSubjectAst::Filter(filter));
    }
    if let Some(filter) = parse_shared_suffix_and_subject_filter(tokens) {
        return Ok(AnthemSubjectAst::Filter(filter));
    }
    if let Some(filter) = parse_best_object_filter_suffix(tokens) {
        return Ok(AnthemSubjectAst::Filter(filter));
    }
    if find_source_reference_start(tokens).is_some() {
        return Ok(AnthemSubjectAst::Source);
    }
    Err(CardTextError::ParseError(format!(
        "unsupported anthem subject (clause: '{}')",
        crate::runtime_backend::token_word_refs(tokens).join(" ")
    )))
}

fn parse_commander_creatures_you_own_subject(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    anthem_grant_grammar::parse_commander_creature_subject_tokens(tokens).map(|_| {
        ObjectFilter::creature()
            .commander()
            .owned_by(PlayerFilter::You)
    })
}

fn parse_enchanted_player_controls_subject(
    tokens: &[OwnedLexToken],
) -> Result<Option<ObjectFilter>, CardTextError> {
    let Some(prefix_tokens) = anthem_grant_grammar::parse_enchanted_player_controls_prefix(tokens)
    else {
        return Ok(None);
    };
    let mut filter = parse_object_filter(prefix_tokens, false)?;
    filter.controller = Some(PlayerFilter::TaggedPlayer(crate::TagKey::from("enchanted")));
    Ok(Some(filter))
}

fn infer_attached_subject_filter_from_condition_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ObjectFilter> {
    let condition_tokens = trim_edge_punctuation(tokens);
    let subject_tokens = anthem_grant_grammar::parse_attached_condition_subject(&condition_tokens)?;
    parse_object_filter(subject_tokens, false).ok()
}

fn parse_anthem_subject_with_attached_fallback(
    tokens: &[OwnedLexToken],
    attached_subject_filter: Option<&ObjectFilter>,
) -> Result<AnthemSubjectAst, CardTextError> {
    if anthem_grant_grammar::is_source_it_subject(tokens)
        && let Some(filter) = attached_subject_filter
    {
        return Ok(AnthemSubjectAst::Filter(filter.clone()));
    }
    parse_anthem_subject(tokens)
}

fn infer_attached_subject_filter_from_condition_expr(
    condition: Option<&crate::ConditionExpr>,
) -> Option<ObjectFilter> {
    match condition {
        Some(crate::ConditionExpr::EnchantedPermanentIsCreature)
        | Some(crate::ConditionExpr::EnchantedPermanentIsLand)
        | Some(crate::ConditionExpr::EnchantedPermanentIsEquipment)
        | Some(crate::ConditionExpr::EnchantedPermanentIsVehicle) => {
            Some(ObjectFilter::tagged("enchanted"))
        }
        _ => None,
    }
}

pub(crate) fn parse_static_quantity_prefix(
    tokens: &[OwnedLexToken],
    allow_default_one: bool,
) -> Result<(crate::effect::Comparison, usize), CardTextError> {
    parse_quantity_comparison_prefix(tokens, allow_default_one, true, "static condition")
}

pub(crate) fn parse_permanent_card_count_filter(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    let facts = anthem_grant_grammar::parse_permanent_card_count_facts(tokens)?;

    let mut filter = ObjectFilter::default();
    filter.card_types = vec![
        CardType::Artifact,
        CardType::Creature,
        CardType::Enchantment,
        CardType::Land,
        CardType::Planeswalker,
        CardType::Battle,
    ];

    filter.zone = Some(facts.zone);
    filter.owner = match facts.owner {
        Some(anthem_grant_grammar::PermanentCardOwner::You) => Some(PlayerFilter::You),
        Some(anthem_grant_grammar::PermanentCardOwner::Opponent) => Some(PlayerFilter::Opponent),
        None => None,
    };
    Some(filter)
}

pub(crate) fn parse_static_condition_clause(
    tokens: &[OwnedLexToken],
) -> Result<crate::ConditionExpr, CardTextError> {
    let tokens = trim_edge_punctuation(tokens);
    let clause_word_storage = AnthemNormalizedWords::new(&tokens);
    let clause_words = clause_word_storage.word_refs();
    if clause_words.is_empty() {
        return Err(CardTextError::ParseError(
            "missing condition clause after 'as long as'".to_string(),
        ));
    }
    let display = clause_words.join(" ");

    if let Some(condition) = parse_cards_in_hand_static_condition(&tokens) {
        return Ok(condition);
    }
    if let Some(condition) = parse_life_total_static_condition(&tokens) {
        return Ok(condition);
    }

    if let Some(kind) = anthem_grant_grammar::parse_fixed_static_condition_kind(&tokens) {
        use anthem_grant_grammar::FixedStaticConditionKind;
        return match kind {
            FixedStaticConditionKind::SourceIsEquipped => {
                Ok(crate::ConditionExpr::SourceIsEquipped)
            }
            FixedStaticConditionKind::OpponentLostLifeThisTurn => {
                Ok(crate::ConditionExpr::OpponentLostLifeThisTurn)
            }
            FixedStaticConditionKind::YouDidNotCastSpellThisTurn => Ok(crate::ConditionExpr::Not(
                Box::new(crate::ConditionExpr::PlayerCastSpellsThisTurnOrMore {
                    player: PlayerFilter::You,
                    count: 1,
                }),
            )),
            FixedStaticConditionKind::YouCastSpellThisTurn => {
                Ok(crate::ConditionExpr::PlayerCastSpellsThisTurnOrMore {
                    player: PlayerFilter::You,
                    count: 1,
                })
            }
            FixedStaticConditionKind::NoCardsInYourLibrary => {
                Ok(crate::ConditionExpr::CountComparison {
                    count: AnthemCountExpression::MatchingFilter(
                        ObjectFilter::default()
                            .in_zone(Zone::Library)
                            .owned_by(PlayerFilter::You),
                    ),
                    comparison: crate::effect::Comparison::Equal(0),
                    display: Some("there are no cards in your library".to_string()),
                })
            }
            FixedStaticConditionKind::SourceIsOnBattlefield => {
                Ok(crate::ConditionExpr::SourceIsInZone(Zone::Battlefield))
            }
            FixedStaticConditionKind::SourceDevouredCreature => {
                Ok(crate::ConditionExpr::SourceDevouredCreaturesOrMore(1))
            }
            FixedStaticConditionKind::SourceIsSoulbondPaired => {
                Ok(crate::ConditionExpr::SourceIsSoulbondPaired)
            }
            FixedStaticConditionKind::SourceAttackedThisTurn => {
                Ok(crate::ConditionExpr::SourceAttackedThisTurn)
            }
            FixedStaticConditionKind::YouAttackedThisTurn => {
                Ok(crate::ConditionExpr::AttackedThisTurn)
            }
            FixedStaticConditionKind::SourceEnteredThisTurn => {
                let mut filter = ObjectFilter::source();
                filter.entered_battlefield_this_turn = true;
                Ok(crate::ConditionExpr::CountComparison {
                    count: AnthemCountExpression::MatchingFilter(filter),
                    comparison: crate::effect::Comparison::GreaterThanOrEqual(1),
                    display: Some(display.clone()),
                })
            }
            FixedStaticConditionKind::YourTurn => Ok(crate::ConditionExpr::YourTurn),
            FixedStaticConditionKind::SourcePowerEven => Err(CardTextError::ParseError(
                "unsupported source power parity condition (clause: 'this power is even')"
                    .to_string(),
            )),
            FixedStaticConditionKind::SourcePowerOdd => Err(CardTextError::ParseError(
                "unsupported source power parity condition (clause: 'this power is odd')"
                    .to_string(),
            )),
            FixedStaticConditionKind::NotYourTurn => Ok(crate::ConditionExpr::Not(Box::new(
                crate::ConditionExpr::YourTurn,
            ))),
            FixedStaticConditionKind::YourLifeAtMostHalfStarting => Ok(
                crate::ConditionExpr::PlayerLifeAtMostHalfStartingLifeTotal {
                    player: PlayerFilter::You,
                },
            ),
            FixedStaticConditionKind::YouCommittedCrimeThisTurn => {
                Ok(crate::ConditionExpr::PlayerCommittedCrimeThisTurn {
                    player: PlayerFilter::You,
                })
            }
        };
    }

    if let Some(life) = anthem_grant_grammar::parse_life_total_or_less_condition(&tokens) {
        return Ok(crate::ConditionExpr::LifeTotalOrLess(life as i32));
    }

    if let Some(condition) = parse_devotion_static_condition(&tokens)? {
        return Ok(condition);
    }

    if let Some(condition) =
        crate::runtime_backend::grammar::conditions::parse_subject_status_condition(&tokens)
            .and_then(|condition| condition.condition_expr())
    {
        return Ok(condition);
    }
    if let Some(condition) =
        crate::runtime_backend::grammar::conditions::parse_subject_descriptor_condition(&tokens)
    {
        return Ok(condition.condition_expr(display.clone()));
    }
    if let Some(condition) =
        crate::runtime_backend::grammar::conditions::parse_player_status_condition(&tokens)
    {
        return Ok(condition.condition_expr());
    }
    if let Some(count) = anthem_grant_grammar::parse_x_value_at_least_condition(&tokens) {
        return Ok(crate::ConditionExpr::XValueAtLeast(count));
    }
    if let Some(condition) =
        crate::runtime_backend::grammar::conditions::parse_player_achievement_condition(&tokens)
    {
        return Ok(condition.condition_expr());
    }
    if let Some(condition) = parse_cards_drawn_this_turn_static_condition(&tokens) {
        return Ok(condition);
    }
    if let Some(shape) = anthem_grant_grammar::parse_blocking_source_condition(&tokens) {
        return Ok(crate::ConditionExpr::CountComparison {
            count: AnthemCountExpression::BlockingSource,
            comparison: shape.comparison,
            display: Some(display.clone()),
        });
    }
    if let Some(condition) = parse_dice_rolled_this_turn_static_condition(&tokens) {
        return Ok(condition);
    }

    if anthem_grant_grammar::parse_source_in_graveyard_condition(&tokens) {
        let mut filter = ObjectFilter::source();
        filter.zone = Some(Zone::Graveyard);
        return Ok(crate::ConditionExpr::CountComparison {
            count: AnthemCountExpression::MatchingFilter(filter),
            comparison: crate::effect::Comparison::GreaterThanOrEqual(1),
            display: Some(display.clone()),
        });
    }

    if let Some(conjoined) = parse_conjoined_static_condition_clause(&tokens) {
        return Ok(conjoined);
    }

    match anthem_grant_grammar::parse_existential_condition_shape(&tokens) {
        Err(()) => {
            return Err(CardTextError::ParseError(
                "missing quantity in static condition".to_string(),
            ));
        }
        Ok(Some(shape)) => {
            use anthem_grant_grammar::ExistentialConditionTail;
            match shape.tail {
                ExistentialConditionTail::CardTypesInYourGraveyard { threshold } => {
                    return Ok(crate::ConditionExpr::PlayerHasCardTypesInGraveyardOrMore {
                        player: PlayerFilter::You,
                        count: threshold,
                    });
                }
                ExistentialConditionTail::CardsInYourGraveyard => {
                    let Some((operator, value)) =
                        crate::runtime_backend::util::comparison_to_value_comparison_operator(
                            shape.comparison,
                        )
                    else {
                        return Err(CardTextError::ParseError(format!(
                            "unsupported graveyard card-count condition (clause: '{display}')"
                        )));
                    };
                    return Ok(crate::ConditionExpr::ValueComparison {
                        left: crate::effect::Value::CardsInGraveyard(PlayerFilter::You),
                        operator,
                        right: crate::effect::Value::Fixed(value),
                    });
                }
                ExistentialConditionTail::DistinctCounterTypesAmong { filter_tokens } => {
                    let filter = parse_object_filter(filter_tokens, false).map_err(|_| {
                        CardTextError::ParseError(format!(
                            "unsupported distinct-counter-kind filter in static condition (clause: '{display}')"
                        ))
                    })?;
                    return Ok(crate::ConditionExpr::CountComparison {
                        count: AnthemCountExpression::DistinctCounterTypesAmong(filter),
                        comparison: shape.comparison,
                        display: Some(display.clone()),
                    });
                }
                ExistentialConditionTail::CountersAmong {
                    filter_tokens,
                    counter_type,
                } => {
                    let filter = parse_object_filter(filter_tokens, false).map_err(|_| {
                        CardTextError::ParseError(format!(
                            "unsupported counter-among filter in static condition (clause: '{display}')"
                        ))
                    })?;
                    return Ok(crate::ConditionExpr::CountComparison {
                        count: AnthemCountExpression::CountersAmong(filter, counter_type),
                        comparison: shape.comparison,
                        display: Some(display.clone()),
                    });
                }
                ExistentialConditionTail::SourceInGraveyard => {
                    let mut filter = ObjectFilter::source();
                    filter.zone = Some(Zone::Graveyard);
                    return Ok(crate::ConditionExpr::CountComparison {
                        count: AnthemCountExpression::MatchingFilter(filter),
                        comparison: shape.comparison,
                        display: Some(display.clone()),
                    });
                }
                ExistentialConditionTail::Generic { filter_tokens } => {
                    if filter_tokens.is_empty() {
                        return Err(CardTextError::ParseError(format!(
                            "missing object phrase in static condition (clause: '{display}')"
                        )));
                    }
                    let filter = parse_permanent_card_count_filter(filter_tokens)
                        .or_else(|| parse_object_filter(filter_tokens, false).ok())
                        .ok_or_else(|| {
                            CardTextError::ParseError(format!(
                                "unsupported counted object phrase in static condition (clause: '{display}')"
                            ))
                        })?;
                    return Ok(crate::ConditionExpr::CountComparison {
                        count: AnthemCountExpression::MatchingFilter(filter),
                        comparison: shape.comparison,
                        display: Some(display.clone()),
                    });
                }
            }
        }
        Ok(None) => {}
    }

    let count_condition_tokens =
        crate::runtime_backend::grammar::leaf::parse_leaf_static_condition_intro_prefix_tokens(
            &tokens,
        )
        .map(|parsed| parsed.rest)
        .unwrap_or(&tokens);

    if let Some(control_condition) =
        crate::runtime_backend::grammar::conditions::parse_control_condition(
            count_condition_tokens,
            crate::runtime_backend::grammar::conditions::ControlConditionOptions {
                allow_that_player: false,
                allow_opponent_players: true,
                allow_defending_player: false,
                bind_filter_controller_to_subject: true,
                allow_different_powers_tail: false,
                default_filter_zone: None,
            },
        )
    {
        return Ok(crate::ConditionExpr::CountComparison {
            count: AnthemCountExpression::MatchingFilter(control_condition.filter),
            comparison: control_condition.comparison,
            display: Some(display.clone()),
        });
    }

    if let Some(ownership_condition) =
        crate::runtime_backend::grammar::conditions::parse_ownership_condition(
            count_condition_tokens,
            crate::runtime_backend::grammar::conditions::OwnershipConditionOptions {
                allow_opponent_players: true,
                bind_filter_owner_to_subject: true,
                default_filter_zone: None,
            },
        )
    {
        return Ok(crate::ConditionExpr::CountComparison {
            count: AnthemCountExpression::MatchingFilter(ownership_condition.filter),
            comparison: ownership_condition.comparison,
            display: Some(display.clone()),
        });
    }

    if let Some(shape) = anthem_grant_grammar::parse_entered_count_condition(&tokens)
        && let Ok(filter) = parse_object_filter(shape.filter_tokens, shape.begins_with_other)
        && (filter.entered_battlefield_this_turn || filter.entered_battlefield_controller.is_some())
    {
        return Ok(crate::ConditionExpr::CountComparison {
            count: AnthemCountExpression::MatchingFilter(filter),
            comparison: shape.comparison,
            display: Some(display.clone()),
        });
    }

    match anthem_grant_grammar::parse_source_counter_condition(&tokens) {
        Ok(Some(shape)) => {
            let mut filter = ObjectFilter::source();
            filter.with_counter = Some(match shape.counter_type {
                Some(counter_type) => crate::filter::CounterConstraint::Typed(counter_type),
                None => crate::filter::CounterConstraint::Any,
            });
            return Ok(crate::ConditionExpr::CountComparison {
                count: AnthemCountExpression::MatchingFilter(filter),
                comparison: shape.comparison,
                display: Some(display.clone()),
            });
        }
        Err(anthem_grant_grammar::SourceCounterConditionError::MissingQuantity) => {
            return Err(CardTextError::ParseError(
                "missing quantity in static condition".to_string(),
            ));
        }
        Err(anthem_grant_grammar::SourceCounterConditionError::MissingCounterPhrase) => {
            return Err(CardTextError::ParseError(format!(
                "missing counter phrase in static condition (clause: '{display}')"
            )));
        }
        Err(anthem_grant_grammar::SourceCounterConditionError::UnsupportedTail) => {
            return Err(CardTextError::ParseError(format!(
                "unsupported source-counter condition tail (clause: '{display}')"
            )));
        }
        Ok(None) => {}
    }

    Err(CardTextError::ParseError(format!(
        "unsupported static condition clause (clause: '{display}')"
    )))
}

fn parse_devotion_static_condition(
    tokens: &[OwnedLexToken],
) -> Result<Option<crate::ConditionExpr>, CardTextError> {
    use anthem_grant_grammar::{DevotionConditionError, DevotionPlayerKind};

    let shape = match anthem_grant_grammar::parse_devotion_condition_shape(tokens) {
        Ok(shape) => shape,
        Err(error) => {
            let display = crate::runtime_backend::token_word_refs(tokens).join(" ");
            let message = match error {
                DevotionConditionError::UnsupportedPlayer => {
                    format!("unsupported devotion player in static condition (clause: '{display}')")
                }
                DevotionConditionError::UnsupportedColor(word) => format!(
                    "unsupported devotion color '{word}' in static condition (clause: '{display}')"
                ),
                DevotionConditionError::MissingColor => {
                    format!("missing devotion color in static condition (clause: '{display}')")
                }
                DevotionConditionError::UnsupportedComparison => format!(
                    "unsupported devotion comparison in static condition (clause: '{display}')"
                ),
                DevotionConditionError::MissingValue => format!(
                    "missing devotion comparison value in static condition (clause: '{display}')"
                ),
                DevotionConditionError::UnsupportedValue(word) => format!(
                    "unsupported devotion comparison value '{word}' in static condition (clause: '{display}')"
                ),
            };
            return Err(CardTextError::ParseError(message));
        }
    };
    let Some(shape) = shape else {
        return Ok(None);
    };

    let player = match shape.player {
        DevotionPlayerKind::You => PlayerFilter::You,
        DevotionPlayerKind::IteratedPlayer => PlayerFilter::IteratedPlayer,
        DevotionPlayerKind::Opponent => PlayerFilter::Opponent,
    };
    let mut values = shape
        .colors
        .into_iter()
        .map(|color| crate::effect::Value::Devotion {
            player: player.clone(),
            color,
        });
    let Some(mut left) = values.next() else {
        return Ok(None);
    };
    for value in values {
        left = crate::effect::Value::Add(Box::new(left), Box::new(value));
    }

    Ok(Some(crate::ConditionExpr::ValueComparison {
        left,
        operator: shape.operator,
        right: crate::effect::Value::Fixed(shape.amount as i32),
    }))
}

fn parse_conjoined_static_condition_clause(
    tokens: &[OwnedLexToken],
) -> Option<crate::ConditionExpr> {
    for split in anthem_grant_grammar::parse_conjoined_condition_splits(tokens) {
        let Ok(left) = parse_static_condition_clause(split.left_tokens) else {
            continue;
        };
        let right = parse_conjoined_static_condition_clause(split.right_tokens)
            .or_else(|| parse_static_condition_clause(split.right_tokens).ok());
        if let Some(right) = right {
            return Some(crate::ConditionExpr::And(Box::new(left), Box::new(right)));
        }
    }
    None
}

fn parse_cards_drawn_this_turn_static_condition(
    tokens: &[OwnedLexToken],
) -> Option<crate::ConditionExpr> {
    let threshold =
        crate::runtime_backend::grammar::anthem_grants::parse_cards_drawn_this_turn_threshold(
            tokens,
        )?;
    let player = anthem_turn_threshold_player_filter(threshold.player);

    Some(crate::ConditionExpr::ValueComparison {
        left: crate::effect::Value::MaxCardsDrawnThisTurn(player),
        operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
        right: crate::effect::Value::Fixed(threshold.count as i32),
    })
}

fn parse_dice_rolled_this_turn_static_condition(
    tokens: &[OwnedLexToken],
) -> Option<crate::ConditionExpr> {
    let threshold =
        crate::runtime_backend::grammar::anthem_grants::parse_dice_rolled_this_turn_threshold(
            tokens,
        )?;
    let player = anthem_turn_threshold_player_filter(threshold.player);

    Some(crate::ConditionExpr::ValueComparison {
        left: crate::effect::Value::MaxDiceRolledThisTurn(player),
        operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
        right: crate::effect::Value::Fixed(threshold.count as i32),
    })
}

fn anthem_turn_threshold_player_filter(
    player: crate::runtime_backend::grammar::anthem_grants::TurnThresholdPlayer,
) -> PlayerFilter {
    use crate::runtime_backend::grammar::anthem_grants::TurnThresholdPlayer;
    match player {
        TurnThresholdPlayer::You => PlayerFilter::You,
        TurnThresholdPlayer::Opponent => PlayerFilter::Opponent,
        TurnThresholdPlayer::Any => PlayerFilter::Any,
    }
}

fn parse_cards_in_hand_static_condition(tokens: &[OwnedLexToken]) -> Option<crate::ConditionExpr> {
    crate::runtime_backend::grammar::conditions::parse_player_cards_in_hand_condition(tokens)?
        .condition_expr()
}

fn parse_life_total_static_condition(tokens: &[OwnedLexToken]) -> Option<crate::ConditionExpr> {
    crate::runtime_backend::grammar::conditions::parse_player_life_total_condition(tokens)?
        .condition_expr()
}

fn explicit_source_counter_surface(
    source_words: &[&str],
) -> Option<crate::target::SourceReferenceSurface> {
    crate::runtime_backend::front_end::shared::util::source_reference_surface_for_words(
        source_words,
    )
    .or_else(|| {
        (source_words.len() > 1)
            .then(|| {
                crate::runtime_backend::front_end::shared::util::this_source_surface_for_words(
                    source_words,
                )
            })
            .flatten()
    })
}

fn source_counter_count_expression(
    counter_type: crate::CounterType,
    source_words: &[&str],
) -> AnthemCountExpression {
    if let Some(surface) = explicit_source_counter_surface(source_words) {
        AnthemCountExpression::CountersOnSourceWithSurface {
            counter_type,
            surface,
        }
    } else {
        AnthemCountExpression::CountersOnSource(counter_type)
    }
}

fn source_counter_count_expression_from_value(value: Value) -> Option<AnthemCountExpression> {
    match value {
        Value::CountersOnSource(counter_type) => {
            Some(AnthemCountExpression::CountersOnSource(counter_type))
        }
        Value::CountersOn(spec, Some(counter_type))
            if matches!(spec.unhinted(), crate::target::ChooseSpec::Source) =>
        {
            spec.source_reference_surface()
                .cloned()
                .map(
                    |surface| AnthemCountExpression::CountersOnSourceWithSurface {
                        counter_type,
                        surface,
                    },
                )
                .or(Some(AnthemCountExpression::CountersOnSource(counter_type)))
        }
        _ => None,
    }
}

fn parse_sticker_count_expression(tokens: &[OwnedLexToken]) -> Option<AnthemCountExpression> {
    let shape = anthem_grant_grammar::parse_sticker_count_shape(tokens)?;
    let action = match shape.kind {
        anthem_grant_grammar::StickerCountKind::Any => crate::events::KeywordActionKind::Sticker,
        anthem_grant_grammar::StickerCountKind::PowerToughness => {
            crate::events::KeywordActionKind::PowerToughnessSticker
        }
        anthem_grant_grammar::StickerCountKind::Name => {
            crate::events::KeywordActionKind::NameSticker
        }
        anthem_grant_grammar::StickerCountKind::Art => crate::events::KeywordActionKind::ArtSticker,
        anthem_grant_grammar::StickerCountKind::Ability => {
            crate::events::KeywordActionKind::AbilitySticker
        }
    };
    let source_words = crate::runtime_backend::token_word_refs(shape.source_tokens);
    let surface = (!source_words.is_empty())
        .then(|| explicit_source_counter_surface(&source_words))
        .flatten();
    Some(AnthemCountExpression::StickersOnSource {
        action,
        surface,
        max_name_letters: shape.max_name_letters,
    })
}

pub(crate) fn parse_anthem_for_each_expression(
    tokens: &[OwnedLexToken],
) -> Result<AnthemCountExpression, CardTextError> {
    let tokens = trim_edge_punctuation(tokens);
    let Some(rest) = anthem_grant_grammar::parse_for_each_rest(&tokens) else {
        return Err(CardTextError::ParseError(
            "missing 'for each' in anthem scaling clause".to_string(),
        ));
    };
    if rest.is_empty() {
        return Err(CardTextError::ParseError(
            "missing object phrase after 'for each'".to_string(),
        ));
    }

    if let Some(shape) = anthem_grant_grammar::parse_for_each_special_shape(rest) {
        match shape {
            anthem_grant_grammar::ForEachSpecialShape::AffectedAttackedThisTurn => {
                return Ok(AnthemCountExpression::AffectedAttackedThisTurn);
            }
            anthem_grant_grammar::ForEachSpecialShape::ColorsOfAffected => {
                return Ok(AnthemCountExpression::ColorsOfAffected);
            }
            anthem_grant_grammar::ForEachSpecialShape::BlockingSource => {
                return Ok(AnthemCountExpression::BlockingSource);
            }
            anthem_grant_grammar::ForEachSpecialShape::AttachedToSource { filter_tokens } => {
                let filter = parse_object_filter(filter_tokens, false).map_err(|_| {
                    CardTextError::ParseError(format!(
                        "unsupported attached-object filter in anthem scaling clause (clause: '{}')",
                        crate::runtime_backend::token_word_refs(&tokens).join(" ")
                    ))
                })?;
                return Ok(AnthemCountExpression::AttachedToSource(filter));
            }
            anthem_grant_grammar::ForEachSpecialShape::UnspentGreenManaYouHave => {
                return Ok(AnthemCountExpression::UnspentMana {
                    player: PlayerFilter::You,
                    symbol: crate::mana::ManaSymbol::Green,
                });
            }
        }
    }

    if let Some(aggregate_value) = parse_aggregate_scope_value_lexed(rest) {
        match aggregate_value {
            Value::BasicLandTypesAmong(filter) => {
                return Ok(AnthemCountExpression::BasicLandTypesAmong(filter));
            }
            Value::CreatureTypesAmong(filter) => {
                return Ok(AnthemCountExpression::CreatureTypesAmong(filter));
            }
            _ => {}
        }
    }

    if let Some(player) = parse_commander_cast_count_player(rest) {
        return Ok(AnthemCountExpression::CommanderCastCount(player));
    }

    if let Some(sticker_count) = parse_sticker_count_expression(rest) {
        return Ok(sticker_count);
    }

    if let Some(filter) = parse_compound_anthem_count_filter(rest) {
        return Ok(AnthemCountExpression::MatchingFilter(filter));
    }

    if let Some(counter_clause) =
        crate::runtime_backend::grammar::anthem_grants::parse_source_counter_count_clause(rest)
        && let Some(counter_type) = parse_counter_type_word(counter_clause.counter_type_word)
    {
        let source_words = crate::runtime_backend::token_word_refs(counter_clause.source_tokens);
        if counter_clause.starts_with_source_pronoun
            || explicit_source_counter_surface(&source_words).is_some()
        {
            return Ok(source_counter_count_expression(counter_type, &source_words));
        }
    }

    let filter = parse_object_filter(rest, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported 'for each' filter in anthem clause (clause: '{}')",
            crate::runtime_backend::token_word_refs(&tokens).join(" ")
        ))
    })?;
    Ok(AnthemCountExpression::MatchingFilter(filter))
}

fn parse_compound_anthem_count_filter(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    let segments = anthem_grant_grammar::parse_compound_count_segments(tokens)?;
    let mut branches = Vec::new();
    for segment in segments {
        let segment = trim_commas(anthem_grant_grammar::strip_each_or_every(segment));
        if segment.is_empty() {
            return None;
        }
        branches.push(parse_object_filter(&segment, false).ok()?);
    }

    if branches.len() < 2 {
        return None;
    }

    let mut combined = ObjectFilter::default();
    combined.any_of = branches;
    Some(combined)
}

pub(crate) fn parse_anthem_prefix_condition(
    tokens: &[OwnedLexToken],
    get_idx: usize,
) -> Result<(Option<crate::ConditionExpr>, usize), CardTextError> {
    let Some(shape) = anthem_grant_grammar::parse_prefix_condition_shape(tokens, get_idx) else {
        return Ok((None, 0));
    };
    if shape.kind == anthem_grant_grammar::AnthemPrefixConditionKind::DuringTurnsOtherThanYours {
        let subject_start = shape
            .comma_subject_start
            .or_else(|| find_source_reference_start(&tokens[..get_idx]))
            .unwrap_or(shape.prefix_end);
        return Ok((
            Some(crate::ConditionExpr::Not(Box::new(
                crate::ConditionExpr::YourTurn,
            ))),
            subject_start,
        ));
    }
    if shape.kind == anthem_grant_grammar::AnthemPrefixConditionKind::DuringYourTurn {
        let subject_start = shape
            .comma_subject_start
            .or_else(|| find_source_reference_start(&tokens[..get_idx]))
            .unwrap_or(shape.prefix_end);
        return Ok((
            Some(crate::ConditionExpr::ActivationTiming(
                crate::ability::ActivationTiming::DuringYourTurn,
            )),
            subject_start,
        ));
    }

    if shape.kind == anthem_grant_grammar::AnthemPrefixConditionKind::AsLongAs {
        let subject_start = shape
            .comma_subject_start
            .or_else(|| infer_as_long_as_subject_start(tokens, get_idx))
            .or_else(|| find_source_reference_start(&tokens[..get_idx]))
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing subject boundary in leading static condition clause (clause: '{}')",
                    crate::runtime_backend::token_word_refs(tokens).join(" ")
                ))
            })?;
        if subject_start <= shape.prefix_end {
            return Err(CardTextError::ParseError(format!(
                "missing condition after leading 'as long as' clause (clause: '{}')",
                crate::runtime_backend::token_word_refs(tokens).join(" ")
            )));
        }
        let condition_tokens = trim_commas(&tokens[shape.prefix_end..subject_start]);
        let condition = parse_static_condition_clause(&condition_tokens)?;
        return Ok((Some(condition), subject_start));
    }
    Ok((None, 0))
}

fn infer_as_long_as_subject_start(tokens: &[OwnedLexToken], action_idx: usize) -> Option<usize> {
    if action_idx <= 3 {
        return None;
    }

    for subject_start in anthem_grant_grammar::parse_word_token_candidates(tokens, 4, action_idx) {
        let condition_tokens = trim_commas(&tokens[3..subject_start]);
        if condition_tokens.is_empty() {
            continue;
        }
        if parse_static_condition_clause(&condition_tokens).is_err() {
            continue;
        }

        let subject_tokens = trim_commas(&tokens[subject_start..action_idx]);
        if subject_tokens.is_empty() {
            continue;
        }
        if parse_anthem_subject(&subject_tokens).is_ok() {
            return Some(subject_start);
        }
    }

    None
}

pub(crate) fn parse_anthem_clause(
    tokens: &[OwnedLexToken],
    get_idx: usize,
    tail_end: usize,
) -> Result<ParsedAnthemClause, CardTextError> {
    let (prefix_condition, subject_start) = parse_anthem_prefix_condition(tokens, get_idx)?;
    let prefix_shape = anthem_grant_grammar::parse_prefix_condition_shape(tokens, get_idx);
    let prefix_attached_subject = if let Some(shape) = prefix_shape
        && shape.kind == anthem_grant_grammar::AnthemPrefixConditionKind::AsLongAs
        && subject_start > shape.prefix_end
    {
        infer_attached_subject_filter_from_condition_tokens(
            &tokens[shape.prefix_end..subject_start],
        )
    } else {
        None
    };
    let subject_tokens = trim_commas(&tokens[subject_start..get_idx]);
    if subject_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing anthem subject (clause: '{}')",
            crate::runtime_backend::token_word_refs(tokens).join(" ")
        )));
    }

    let modifier_shape = anthem_grant_grammar::parse_modifier_shape(tokens, get_idx, tail_end)
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing power/toughness modifier in anthem clause (clause: '{}')",
                crate::runtime_backend::token_word_refs(tokens).join(" ")
            ))
        })?;
    let modifier_token = modifier_shape.modifier_word;
    let tail_tokens = trim_edge_punctuation(modifier_shape.tail_tokens);
    let mut explicit_values = None;
    let (raw_power, raw_toughness) = match parse_pt_modifier_values(modifier_token) {
        Ok(values) => values,
        Err(_) => {
            if let Some(values) = parse_dynamic_xy_anthem_values(modifier_token, &tail_tokens) {
                explicit_values = Some(values);
                (Value::Fixed(0), Value::Fixed(0))
            } else {
                return Err(CardTextError::ParseError(format!(
                    "invalid power/toughness modifier in anthem clause (clause: '{}')",
                    crate::runtime_backend::token_word_refs(tokens).join(" ")
                )));
            }
        }
    };
    let mut scale: Option<AnthemCountExpression> = None;
    let mut count_uses_where_x = false;
    let mut suffix_condition: Option<crate::ConditionExpr> = None;
    let mut suffix_attached_subject: Option<ObjectFilter> = None;
    if explicit_values.is_none() && !tail_tokens.is_empty() {
        match anthem_grant_grammar::parse_tail_shape(&tail_tokens) {
            Some(anthem_grant_grammar::AnthemTailShape::ForEach(tail)) => {
                scale = Some(parse_anthem_for_each_expression(tail)?);
            }
            Some(anthem_grant_grammar::AnthemTailShape::WhereX(tail)) => {
                count_uses_where_x = true;
                let x_value = parse_value_binding_clause(tail).ok_or_else(|| {
                    CardTextError::ParseError(format!(
                        "unsupported where-x anthem clause (clause: '{}')",
                        crate::runtime_backend::token_word_refs(tokens).join(" ")
                    ))
                })?;
                scale = Some(match x_value {
                    Value::Count(filter) => AnthemCountExpression::MatchingFilter(filter),
                    Value::GreatestManaValue(filter) => {
                        AnthemCountExpression::GreatestManaValueAmong(filter)
                    }
                    value
                        if source_counter_count_expression_from_value(value.clone()).is_some() =>
                    {
                        source_counter_count_expression_from_value(value)
                            .expect("checked source-counter expression")
                    }
                    Value::BasicLandTypesAmong(filter) => {
                        AnthemCountExpression::BasicLandTypesAmong(filter)
                    }
                    Value::CreatureTypesAmong(filter) => {
                        AnthemCountExpression::CreatureTypesAmong(filter)
                    }
                    Value::Speed(player) => AnthemCountExpression::PlayerSpeed(player),
                    _ => {
                        return Err(CardTextError::ParseError(format!(
                            "unsupported where-x anthem value (clause: '{}')",
                            crate::runtime_backend::token_word_refs(tokens).join(" ")
                        )));
                    }
                });
            }
            Some(anthem_grant_grammar::AnthemTailShape::AsLongAs { condition_tokens }) => {
                suffix_attached_subject =
                    infer_attached_subject_filter_from_condition_tokens(condition_tokens);
                suffix_condition = Some(parse_static_condition_clause(condition_tokens)?);
            }
            None => {
                return Err(CardTextError::ParseError(format!(
                    "unsupported trailing anthem clause (clause: '{}')",
                    crate::runtime_backend::token_word_refs(tokens).join(" ")
                )));
            }
        }
    }

    let attached_subject_filter = prefix_attached_subject
        .as_ref()
        .or(suffix_attached_subject.as_ref());
    let subject =
        parse_anthem_subject_with_attached_fallback(&subject_tokens, attached_subject_filter)?;

    let condition = match (prefix_condition, suffix_condition) {
        (Some(_prefix), Some(_)) => {
            return Err(CardTextError::ParseError(format!(
                "multiple anthem conditions are not supported (clause: '{}')",
                crate::runtime_backend::token_word_refs(tokens).join(" ")
            )));
        }
        (Some(condition), None) | (None, Some(condition)) => Some(condition),
        (None, None) => None,
    };

    let has_dynamic_component = matches!(raw_power, Value::X | Value::XTimes(_))
        || matches!(raw_toughness, Value::X | Value::XTimes(_));
    let scale_fixed_components = scale.is_some() && !has_dynamic_component;
    let resolve_anthem_value = |component: Value,
                                scale_expr: Option<&AnthemCountExpression>,
                                scale_fixed_components: bool|
     -> Result<AnthemValue, CardTextError> {
        match component {
            Value::Fixed(value) => Ok(match scale_expr {
                Some(scale_expr) if scale_fixed_components => {
                    AnthemValue::scaled(value, scale_expr.clone())
                }
                None => AnthemValue::Fixed(value),
                Some(_) => AnthemValue::Fixed(value),
            }),
            Value::X => {
                if let Some(scale_expr) = scale_expr {
                    Ok(AnthemValue::scaled(1, scale_expr.clone()))
                } else {
                    Err(CardTextError::ParseError(format!(
                        "unsupported X power/toughness modifier without count expression (clause: '{}')",
                        crate::runtime_backend::token_word_refs(tokens).join(" ")
                    )))
                }
            }
            Value::XTimes(multiplier) => {
                if let Some(scale_expr) = scale_expr {
                    Ok(AnthemValue::scaled(multiplier, scale_expr.clone()))
                } else {
                    Err(CardTextError::ParseError(format!(
                        "unsupported X power/toughness modifier without count expression (clause: '{}')",
                        crate::runtime_backend::token_word_refs(tokens).join(" ")
                    )))
                }
            }
            _ => Err(CardTextError::ParseError(format!(
                "invalid power/toughness modifier in anthem clause (clause: '{}')",
                crate::runtime_backend::token_word_refs(tokens).join(" ")
            ))),
        }
    };

    let (mut power, mut toughness) = if let Some((power, toughness)) = explicit_values {
        (power, toughness)
    } else {
        (
            resolve_anthem_value(raw_power, scale.as_ref(), scale_fixed_components)?,
            resolve_anthem_value(raw_toughness, scale.as_ref(), scale_fixed_components)?,
        )
    };

    // When the anthem affects multiple creatures (subject is a filter rather
    // than "this creature"), any "... on/attached to it" count expression
    // refers to the affected creature, not the anthem source. Promote those
    // source-relative expressions so the runtime evaluates the count
    // per-creature.
    if matches!(subject, AnthemSubjectAst::Filter(_)) {
        promote_attached_to_affected(&mut power);
        promote_attached_to_affected(&mut toughness);
        promote_counters_on_affected(&mut power);
        promote_counters_on_affected(&mut toughness);
    }

    parser_trace_stack("parse_static:anthem-clause:matched", tokens);
    Ok(ParsedAnthemClause {
        subject,
        power,
        toughness,
        condition,
        count_uses_where_x,
    })
}

fn parse_dynamic_xy_anthem_values(
    modifier_token: &str,
    tail_tokens: &[OwnedLexToken],
) -> Option<(AnthemValue, AnthemValue)> {
    let (power_raw, toughness_raw) = split_pt_modifier_components(modifier_token).ok()?;
    let power_var = pt_modifier_variable(power_raw)?;
    let toughness_var = pt_modifier_variable(toughness_raw)?;
    let bindings = parse_where_x_y_bindings(tail_tokens)?;
    let value_for = |var| match var {
        'x' => Some(AnthemValue::Dynamic(bindings.0.clone())),
        'y' => Some(AnthemValue::Dynamic(bindings.1.clone())),
        _ => None,
    };
    Some((value_for(power_var)?, value_for(toughness_var)?))
}

fn pt_modifier_variable(raw: &str) -> Option<char> {
    let value = strip_leading_plus_char(raw).trim();
    match value {
        "x" | "X" => Some('x'),
        "y" | "Y" => Some('y'),
        _ => None,
    }
}

fn parse_where_x_y_bindings(tokens: &[OwnedLexToken]) -> Option<(Value, Value)> {
    let shape = anthem_grant_grammar::parse_where_x_y_bindings_shape(tokens)?;
    let x_words = crate::runtime_backend::lexer::token_word_refs(shape.x_tokens);
    let y_words = crate::runtime_backend::lexer::token_word_refs(shape.y_tokens);
    let (x_value, x_used) = parse_value_expr_words(&x_words)?;
    let (y_value, y_used) = parse_value_expr_words(&y_words)?;
    (x_used == x_words.len() && y_used == y_words.len()).then_some((x_value, y_value))
}

/// When an anthem targets a filter of creatures (not just the source),
/// "attached to it" refers to the affected creature, not the source.
/// Promote `AttachedToSource` -> `AttachedToAffected` in the anthem value.
fn promote_attached_to_affected(value: &mut AnthemValue) {
    if let AnthemValue::PerCount {
        count: count @ AnthemCountExpression::AttachedToSource(_),
        ..
    } = value
    {
        // Extract the inner filter and replace with AttachedToAffected.
        let AnthemCountExpression::AttachedToSource(filter) = std::mem::replace(
            count,
            AnthemCountExpression::AttachedToAffected(ObjectFilter::default()),
        ) else {
            unreachable!()
        };
        *count = AnthemCountExpression::AttachedToAffected(filter);
    }
}

fn promote_counters_on_affected(value: &mut AnthemValue) {
    if let AnthemValue::PerCount {
        count: count @ AnthemCountExpression::CountersOnSource(_),
        ..
    } = value
    {
        let AnthemCountExpression::CountersOnSource(counter_type) = std::mem::replace(
            count,
            AnthemCountExpression::CountersOnAffected(crate::CounterType::Charge),
        ) else {
            unreachable!()
        };
        *count = AnthemCountExpression::CountersOnAffected(counter_type);
    }
}

pub(crate) fn build_anthem_static_ability(clause: &ParsedAnthemClause) -> StaticAbility {
    let mut anthem = match &clause.subject {
        AnthemSubjectAst::Source => Anthem::for_source(0, 0),
        AnthemSubjectAst::Filter(filter) => Anthem::new(filter.clone(), 0, 0),
    }
    .with_values(clause.power.clone(), clause.toughness.clone())
    .with_count_uses_where_x(clause.count_uses_where_x);

    if let Some(condition) = &clause.condition {
        anthem = anthem.with_condition(condition.clone());
    }

    StaticAbility::new(anthem)
}

#[derive(Debug)]
pub(crate) struct TypeColorAdditionClause {
    pub(crate) added_colors: ColorSet,
    pub(crate) set_colors: ColorSet,
    pub(crate) card_types: Vec<CardType>,
    pub(crate) subtypes: Vec<Subtype>,
}

pub(crate) fn parse_type_color_addition_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<TypeColorAdditionClause>, CardTextError> {
    let Some(shape) = anthem_grant_grammar::parse_type_color_addition_shape(tokens) else {
        return Ok(None);
    };
    let words = crate::runtime_backend::token_word_refs(tokens);
    let mut allow_colors = false;
    let mut allow_types = false;
    for scope in shape.scopes {
        match scope {
            anthem_grant_grammar::TypeColorScope::Colors => allow_colors = true,
            anthem_grant_grammar::TypeColorScope::Types { qualifier_tokens } => {
                let qualifiers = crate::runtime_backend::token_word_refs(qualifier_tokens);
                if qualifiers
                    .iter()
                    .all(|word| is_type_scope_qualifier_word(word))
                {
                    allow_types = true;
                } else {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported in-addition scope in type/color clause (clause: '{}')",
                        words.join(" ")
                    )));
                }
            }
            anthem_grant_grammar::TypeColorScope::Unsupported { .. } => {
                return Err(CardTextError::ParseError(format!(
                    "unsupported in-addition scope in type/color clause (clause: '{}')",
                    words.join(" ")
                )));
            }
        }
    }
    if !allow_colors && !allow_types {
        return Ok(None);
    }

    let descriptor_word_storage = crate::runtime_backend::token_word_refs(shape.descriptor_tokens);
    let descriptor_words = non_article_word_refs_except(&descriptor_word_storage, &["and"]);
    if descriptor_words.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing type/color descriptors in in-addition clause (clause: '{}')",
            words.join(" ")
        )));
    }

    let mut added_colors = ColorSet::new();
    let mut set_colors = ColorSet::new();
    let mut card_types = Vec::new();
    let mut subtypes = Vec::new();
    for descriptor in descriptor_words {
        if let Some(color) = parse_color(descriptor) {
            if allow_colors {
                added_colors = added_colors.union(color);
            } else if allow_types {
                // "is black Zombie in addition to its other creature types"
                // sets color while only preserving existing types.
                set_colors = set_colors.union(color);
            } else {
                return Err(CardTextError::ParseError(format!(
                    "color descriptor '{}' not allowed by in-addition scope (clause: '{}')",
                    descriptor,
                    words.join(" ")
                )));
            }
            continue;
        }

        if let Some(card_type) = parse_card_type(descriptor) {
            if allow_types {
                if !card_types.iter().any(|existing| *existing == card_type) {
                    card_types.push(card_type);
                }
                continue;
            }
            return Err(CardTextError::ParseError(format!(
                "card type descriptor '{}' not allowed by in-addition scope (clause: '{}')",
                descriptor,
                words.join(" ")
            )));
        }

        if let Some(subtype) = parse_subtype_flexible(descriptor) {
            if allow_types {
                if !subtypes.iter().any(|existing| *existing == subtype) {
                    subtypes.push(subtype);
                }
                continue;
            }
            return Err(CardTextError::ParseError(format!(
                "subtype descriptor '{}' not allowed by in-addition scope (clause: '{}')",
                descriptor,
                words.join(" ")
            )));
        }

        return Err(CardTextError::ParseError(format!(
            "unsupported descriptor '{}' in type/color addition clause (clause: '{}')",
            descriptor,
            words.join(" ")
        )));
    }

    if added_colors.is_empty()
        && set_colors.is_empty()
        && card_types.is_empty()
        && subtypes.is_empty()
    {
        return Err(CardTextError::ParseError(format!(
            "missing type/color additions in in-addition clause (clause: '{}')",
            words.join(" ")
        )));
    }

    Ok(Some(TypeColorAdditionClause {
        added_colors,
        set_colors,
        card_types,
        subtypes,
    }))
}

pub(crate) fn is_type_scope_qualifier_word(word: &str) -> bool {
    parse_card_type(word).is_some()
        || matches!(
            word,
            "card" | "creature" | "permanent" | "basic" | "legendary" | "snow" | "nonbasic"
        )
}

pub(crate) fn parse_soulbond_shared_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    let Some(shape) = anthem_grant_grammar::parse_soulbond_shared_shape(tokens) else {
        return Ok(None);
    };
    let subject_words = crate::runtime_backend::token_word_refs(shape.subject_tokens);
    let source_like_subject = is_source_reference_words(&subject_words)
        || shape.subject_is_source_pronoun
        || !shape.subject_has_rejected_word;
    if !source_like_subject {
        return Ok(None);
    }
    if let anthem_grant_grammar::SoulbondSharedEffect::PowerToughness { modifier_word } =
        shape.effect
    {
        let (power, toughness) = parse_pt_modifier(modifier_word).map_err(|_| {
            CardTextError::ParseError(format!(
                "invalid power/toughness modifier in soulbond clause (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
        return Ok(Some(vec![
            StaticAbility::soulbond_shared_power_toughness(power, toughness).into(),
        ]));
    }
    if let anthem_grant_grammar::SoulbondSharedEffect::Ability {
        ability_tokens,
        mills_each_opponent_by_toughness,
    } = shape.effect
    {
        if mills_each_opponent_by_toughness {
            let display = display_text_for_tokens(ability_tokens, false);
            let ability = parsed_triggered_ability(
                TriggerSpec::ThisAttacks,
                vec![EffectAst::subject_verb(
                    crate::cards::builders::SubjectVerbRoleAst::AffectedPlayer,
                    crate::cards::builders::PlayerAst::Opponent,
                    crate::cards::builders::SubjectVerbActionAst::Mill {
                        count: Value::ToughnessOf(Box::new(ChooseSpec::Source)),
                    },
                )],
                vec![Zone::Battlefield],
                Some(display.clone()),
                None,
                None,
                ReferenceImports::default(),
            );
            return Ok(Some(vec![StaticAbilityAst::SoulbondSharedObjectAbility {
                ability,
            }]));
        }

        if let Some(actions) = parse_ability_line(ability_tokens) {
            reject_unimplemented_keyword_actions(&actions, &clause_words.join(" "))?;
            let abilities: Vec<StaticAbility> = actions
                .into_iter()
                .filter_map(|action| keyword_action_to_static_ability(action))
                .collect();
            if abilities.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "unsupported shared ability in soulbond clause (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            let shared = abilities
                .into_iter()
                .map(StaticAbility::soulbond_shared_ability)
                .map(StaticAbilityAst::from)
                .collect();
            return Ok(Some(shared));
        }

        if let Some(GrantedAbilityAst::ParsedObjectAbility { ability, .. }) =
            parse_granted_activated_or_triggered_ability_for_gain(ability_tokens, &clause_words)?
        {
            return Ok(Some(vec![StaticAbilityAst::SoulbondSharedObjectAbility {
                ability,
            }]));
        }

        return Err(CardTextError::ParseError(format!(
            "unsupported shared ability in soulbond clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    Ok(None)
}

pub(crate) fn parse_anthem_and_type_color_addition_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    let Some(shape) = anthem_grant_grammar::parse_anthem_and_addition_shape(tokens) else {
        return Ok(None);
    };
    if shape.temporary {
        return Ok(None);
    }
    let Some(additions) = parse_type_color_addition_clause(shape.addition_tokens)? else {
        return Ok(None);
    };

    let clause = parse_anthem_clause(tokens, shape.get_token, shape.and_token)?;
    let AnthemSubjectAst::Filter(filter) = &clause.subject else {
        return Err(CardTextError::ParseError(format!(
            "unsupported source-only type/color addition clause (clause: '{}')",
            words.join(" ")
        )));
    };

    let mut result = vec![build_anthem_static_ability(&clause)];
    if !additions.set_colors.is_empty() {
        result.push(StaticAbility::set_colors(
            filter.clone(),
            additions.set_colors,
        ));
    }
    if !additions.added_colors.is_empty() {
        result.push(StaticAbility::add_colors(
            filter.clone(),
            additions.added_colors,
        ));
    }
    if !additions.card_types.is_empty() {
        result.push(StaticAbility::add_card_types(
            filter.clone(),
            additions.card_types,
        ));
    }
    if !additions.subtypes.is_empty() {
        result.push(StaticAbility::add_subtypes(
            filter.clone(),
            additions.subtypes,
        ));
    }
    Ok(Some(result))
}

pub(crate) fn parse_anthem_and_keyword_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    let Some(line_shape) = anthem_grant_grammar::parse_anthem_keyword_head(tokens) else {
        return Ok(None);
    };
    let get_idx = line_shape.get_token;
    let have_token_idx = line_shape.have_token;

    if line_shape.order == anthem_grant_grammar::AnthemKeywordOrder::KeywordBeforeAnthem {
        let Some(shape) =
            anthem_grant_grammar::parse_keyword_before_anthem_shape(tokens, line_shape)
        else {
            return Ok(None);
        };
        let subject = parse_anthem_subject(shape.subject_tokens)?;
        let Some(actions) = parse_ability_line(shape.keyword_tokens) else {
            return Ok(None);
        };
        reject_unimplemented_keyword_actions(&actions, &clause_words.join(" "))?;

        let mut anthem_tokens = shape.subject_tokens.to_vec();
        anthem_tokens.extend_from_slice(shape.anthem_tail_tokens);
        let Some(anthem) = parse_anthem_line(&anthem_tokens)? else {
            return Ok(None);
        };
        let mut result = vec![StaticAbilityAst::from(anthem)];
        let grant_clause = ParsedAnthemClause {
            subject,
            power: AnthemValue::Fixed(0),
            toughness: AnthemValue::Fixed(0),
            condition: None,
            count_uses_where_x: false,
        };
        for action in actions
            .into_iter()
            .filter(|action| action.lowers_to_static_ability())
        {
            result.push(grant_keyword_action_for_anthem_subject(
                &grant_clause,
                action,
            ));
        }
        return Ok(Some(result));
    }

    // "until end of turn" in the pump clause indicates a one-shot effect.
    // Ignore timing text that appears only inside a quoted granted ability.
    if line_shape.pre_grant_is_temporary {
        return Ok(None);
    }

    if let Some(color_segment) =
        anthem_grant_grammar::parse_anthem_keyword_color_segment(tokens, line_shape)
    {
        let clause = parse_anthem_clause(tokens, get_idx, color_segment.is_token)?;
        let filter = anthem_subject_filter(&clause.subject);
        let mut result = vec![build_anthem_static_ability(&clause).into()];
        let color_static = StaticAbility::set_colors(filter, color_segment.color);
        let color_ast: StaticAbilityAst = color_static.into();
        result.push(match &clause.condition {
            Some(condition) => add_static_ability_ast_condition(color_ast, condition.clone())?,
            None => color_ast,
        });

        let ability_tokens_storage = trim_edge_punctuation(&tokens[have_token_idx + 1..]);
        let ability_tokens = trim_outer_quotes(&ability_tokens_storage);
        if anthem_grant_grammar::parse_colon_tail_split(&ability_tokens).is_some() {
            let Some(parsed) = parse_activated_line(ability_tokens)? else {
                return Err(CardTextError::ParseError(format!(
                    "unsupported granted activated ability in anthem clause (clause: '{}')",
                    clause_words.join(" ")
                )));
            };
            let display = display_text_for_tokens(ability_tokens, false);
            result.push(grant_object_ability_for_anthem_subject(
                &clause, parsed, display,
            ));
            return Ok(Some(result));
        }
    }

    if let Some(compound) =
        anthem_grant_grammar::parse_anthem_keyword_compound_split(tokens, line_shape)
    {
        let first_clause = parse_anthem_clause(tokens, get_idx, compound.split_token)?;
        let mut result = vec![build_anthem_static_ability(&first_clause).into()];

        let grant_clause = if let Some(second_get_idx) = compound.second_get_token {
            let second_tokens = &tokens[compound.tail_start..];
            let second_clause = parse_anthem_clause(
                second_tokens,
                second_get_idx - compound.tail_start,
                compound.second_tail_end - compound.tail_start,
            )?;
            result.push(build_anthem_static_ability(&second_clause).into());
            second_clause
        } else {
            let subject_tokens =
                trim_edge_punctuation(&tokens[compound.tail_start..have_token_idx]);
            if subject_tokens.is_empty() {
                return Ok(None);
            }
            ParsedAnthemClause {
                subject: parse_anthem_subject(&subject_tokens)?,
                power: AnthemValue::Fixed(0),
                toughness: AnthemValue::Fixed(0),
                condition: None,
                count_uses_where_x: false,
            }
        };

        let ability_tokens = trim_edge_punctuation(&tokens[have_token_idx + 1..]);
        let Some(actions) = parse_ability_line(&ability_tokens) else {
            return Ok(None);
        };
        reject_unimplemented_keyword_actions(&actions, &clause_words.join(" "))?;
        for action in actions
            .into_iter()
            .filter(|action| action.lowers_to_static_ability())
        {
            result.push(grant_keyword_action_for_anthem_subject(
                &grant_clause,
                action,
            ));
        }
        return Ok(Some(result));
    }

    let mut ability_tokens = trim_edge_punctuation(&tokens[have_token_idx + 1..]);
    let mut trailing_condition: Option<crate::ConditionExpr> = None;
    match anthem_grant_grammar::split_anthem_keyword_trailing_condition(&ability_tokens) {
        Ok(Some(split)) => {
            trailing_condition = Some(parse_static_condition_clause(split.condition_tokens)?);
            ability_tokens = split.ability_tokens.to_vec();
        }
        Ok(None) => {}
        Err(anthem_grant_grammar::AnthemKeywordTrailingConditionError::MissingAbility) => {
            return Err(CardTextError::ParseError(format!(
                "missing granted keyword list before trailing condition (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        Err(anthem_grant_grammar::AnthemKeywordTrailingConditionError::MissingCondition) => {
            return Err(CardTextError::ParseError(format!(
                "missing condition after trailing 'as long as' keyword clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
    }
    let mut trailing_type_color_addition: Option<TypeColorAdditionClause> = None;
    if let Some(split) = anthem_grant_grammar::split_anthem_keyword_and_is(&ability_tokens) {
        if let Some(additions) = parse_type_color_addition_clause(split.tail_tokens)? {
            trailing_type_color_addition = Some(additions);
            ability_tokens = split.head_tokens.to_vec();
        }
    }

    let mut keyword_actions: Vec<KeywordAction> = Vec::new();
    let mut granted_activated_ability: Option<ParsedAbility> = None;
    let mut granted_activated_display: Option<String> = None;

    if let Some(and_has) = anthem_grant_grammar::split_anthem_keyword_and_have(&ability_tokens) {
        let keyword_tokens = and_has.head_tokens.to_vec();
        if !keyword_tokens.is_empty() {
            if let Some(actions) = parse_ability_line(&keyword_tokens) {
                reject_unimplemented_keyword_actions(&actions, &clause_words.join(" "))?;
                keyword_actions.extend(
                    actions
                        .into_iter()
                        .filter(|action| action.lowers_to_static_ability()),
                );
            } else {
                return Ok(None);
            }
        }

        let ability_tail_tokens = and_has.tail_tokens.to_vec();
        if !ability_tail_tokens.is_empty() {
            let mut handled_split_keyword_activation = false;
            if let Some(colon) = anthem_grant_grammar::parse_colon_tail_split(&ability_tail_tokens)
            {
                if let Some(split_and_idx) = colon.last_and_before_colon {
                    let trailing_keyword_tokens =
                        trim_edge_punctuation(&ability_tail_tokens[..split_and_idx]);
                    let activated_tail =
                        trim_edge_punctuation(&ability_tail_tokens[split_and_idx + 1..]);
                    if !trailing_keyword_tokens.is_empty() {
                        let Some(actions) = parse_ability_line(&trailing_keyword_tokens) else {
                            return Ok(None);
                        };
                        reject_unimplemented_keyword_actions(&actions, &clause_words.join(" "))?;
                        keyword_actions.extend(
                            actions
                                .into_iter()
                                .filter(|action| action.lowers_to_static_ability()),
                        );
                    }
                    let has_colon =
                        anthem_grant_grammar::parse_colon_tail_split(&activated_tail).is_some();
                    let Some(parsed) = parse_activated_line(&activated_tail)? else {
                        if has_colon {
                            return Err(CardTextError::ParseError(format!(
                                "unsupported granted activated ability in anthem clause (clause: '{}')",
                                clause_words.join(" ")
                            )));
                        }
                        return Ok(None);
                    };
                    let display = display_text_for_tokens(&activated_tail, false);
                    granted_activated_display = Some(display);
                    granted_activated_ability = Some(parsed);
                    handled_split_keyword_activation = true;
                }
            }
            if !handled_split_keyword_activation {
                let has_colon =
                    anthem_grant_grammar::parse_colon_tail_split(&ability_tail_tokens).is_some();
                let Some(parsed) = parse_activated_line(&ability_tail_tokens)? else {
                    if has_colon {
                        return Err(CardTextError::ParseError(format!(
                            "unsupported granted activated ability in anthem clause (clause: '{}')",
                            clause_words.join(" ")
                        )));
                    }
                    return Ok(None);
                };
                let display = display_text_for_tokens(&ability_tail_tokens, false);
                granted_activated_display = Some(display);
                granted_activated_ability = Some(parsed);
            }
        }
    } else if let Some(colon) = anthem_grant_grammar::parse_colon_tail_split(&ability_tokens) {
        let Some(and_idx) = colon.last_and_before_colon else {
            let activated_tail_storage = trim_edge_punctuation(&ability_tokens);
            let activated_tail = trim_outer_quotes(&activated_tail_storage);
            let Some(parsed) = parse_activated_line(activated_tail)? else {
                return Err(CardTextError::ParseError(format!(
                    "unsupported granted activated ability in anthem clause (clause: '{}')",
                    clause_words.join(" ")
                )));
            };
            let display = display_text_for_tokens(activated_tail, false);
            granted_activated_display = Some(display);
            granted_activated_ability = Some(parsed);
            let mut clause = parse_anthem_clause(tokens, get_idx, line_shape.clause_tail_end)?;
            if let Some(condition) = trailing_condition {
                if clause.condition.is_some() {
                    return Err(CardTextError::ParseError(format!(
                        "multiple anthem conditions are not supported (clause: '{}')",
                        clause_words.join(" ")
                    )));
                }
                clause.condition = Some(condition);
            }
            let mut result = vec![build_anthem_static_ability(&clause).into()];
            if let Some(ability) = granted_activated_ability {
                result.push(grant_object_ability_for_anthem_subject(
                    &clause,
                    ability,
                    granted_activated_display.unwrap_or_else(|| clause_words.join(" ")),
                ));
            }
            return Ok(Some(result));
        };
        let keyword_head = trim_edge_punctuation(&ability_tokens[..and_idx]);
        let activated_tail = trim_edge_punctuation(&ability_tokens[and_idx + 1..]);
        if keyword_head.is_empty() || activated_tail.is_empty() {
            return Ok(None);
        }
        let Some(actions) = parse_ability_line(&keyword_head) else {
            return Ok(None);
        };
        reject_unimplemented_keyword_actions(&actions, &clause_words.join(" "))?;
        keyword_actions = actions
            .into_iter()
            .filter(|action| action.lowers_to_static_ability())
            .collect();
        let has_colon = anthem_grant_grammar::parse_colon_tail_split(&activated_tail).is_some();
        let Some(parsed) = parse_activated_line(&activated_tail)? else {
            if has_colon {
                return Err(CardTextError::ParseError(format!(
                    "unsupported granted activated ability in anthem clause (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            return Ok(None);
        };
        let display = display_text_for_tokens(&activated_tail, false);
        granted_activated_display = Some(display);
        granted_activated_ability = Some(parsed);
    } else if let Some(GrantedAbilityAst::ParsedObjectAbility { ability, display }) =
        parse_granted_activated_or_triggered_ability_for_gain(&ability_tokens, &clause_words)?
    {
        granted_activated_display = Some(display);
        granted_activated_ability = Some(ability);
    } else if let Some(actions) = parse_ability_line(&ability_tokens) {
        reject_unimplemented_keyword_actions(&actions, &clause_words.join(" "))?;
        keyword_actions = actions
            .into_iter()
            .filter(|action| action.lowers_to_static_ability())
            .collect();
    } else {
        return Ok(None);
    }

    if keyword_actions.is_empty() && granted_activated_ability.is_none() {
        return Ok(None);
    }

    let mut clause = parse_anthem_clause(tokens, get_idx, line_shape.clause_tail_end)?;
    if let Some(condition) = trailing_condition {
        if clause.condition.is_some() {
            return Err(CardTextError::ParseError(format!(
                "multiple anthem conditions are not supported (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        clause.condition = Some(condition);
    }
    let mut result = vec![build_anthem_static_ability(&clause).into()];
    for action in keyword_actions {
        result.push(grant_keyword_action_for_anthem_subject(&clause, action));
    }
    if let Some(additions) = trailing_type_color_addition {
        push_type_color_additions_for_anthem_subject(&mut result, &clause, additions)?;
    }

    if let Some(ability) = granted_activated_ability {
        result.push(grant_object_ability_for_anthem_subject(
            &clause,
            ability,
            granted_activated_display.unwrap_or_else(|| clause_words.join(" ")),
        ));
    }

    Ok(Some(result))
}

pub(crate) fn parse_anthem_and_goaded_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    let Some(shape) = anthem_grant_grammar::parse_anthem_goaded_shape(tokens) else {
        return Ok(None);
    };

    let clause = parse_anthem_clause(tokens, shape.get_token, shape.and_token)?;
    let display_subject = attached_goaded_display_subject(&clause.subject).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported goaded anthem subject (clause: '{}')",
            clause_words.join(" ")
        ))
    })?;

    Ok(Some(vec![
        build_anthem_static_ability(&clause).into(),
        crate::static_abilities::StaticAbility::attached_goaded_by_source_controller(format!(
            "{} is goaded",
            capitalize_display_subject(&display_subject)
        ))
        .into(),
    ]))
}

fn attached_goaded_display_subject(subject: &AnthemSubjectAst) -> Option<String> {
    let AnthemSubjectAst::Filter(filter) = subject else {
        return None;
    };
    let attachment = filter.tagged_constraints.iter().find_map(|constraint| {
        if !matches!(
            constraint.relation,
            crate::filter::TaggedOpbjectRelation::IsTaggedObject
        ) {
            return None;
        }
        match constraint.tag.as_str() {
            "enchanted" => Some("enchanted"),
            "equipped" => Some("equipped"),
            _ => None,
        }
    })?;

    let noun = if filter.card_types.contains(&CardType::Creature) {
        "creature"
    } else {
        "permanent"
    };
    Some(format!("{attachment} {noun}"))
}

fn capitalize_display_subject(subject: &str) -> String {
    let mut chars = subject.chars();
    match chars.next() {
        Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
        None => String::new(),
    }
}

fn push_type_color_additions_for_anthem_subject(
    result: &mut Vec<StaticAbilityAst>,
    clause: &ParsedAnthemClause,
    additions: TypeColorAdditionClause,
) -> Result<(), CardTextError> {
    let filter = anthem_subject_filter(&clause.subject);
    let condition = clause.condition.clone();
    let mut push_static = |ability: StaticAbility| -> Result<(), CardTextError> {
        let ast: StaticAbilityAst = ability.into();
        result.push(match &condition {
            Some(condition) => add_static_ability_ast_condition(ast, condition.clone())?,
            None => ast,
        });
        Ok(())
    };

    if !additions.set_colors.is_empty() {
        push_static(StaticAbility::set_colors(
            filter.clone(),
            additions.set_colors,
        ))?;
    }
    if !additions.added_colors.is_empty() {
        push_static(StaticAbility::add_colors(
            filter.clone(),
            additions.added_colors,
        ))?;
    }
    if !additions.card_types.is_empty() {
        push_static(StaticAbility::add_card_types(
            filter.clone(),
            additions.card_types,
        ))?;
    }
    if !additions.subtypes.is_empty() {
        push_static(StaticAbility::add_subtypes(filter, additions.subtypes))?;
    }

    Ok(())
}

fn merge_static_ability_ast_conditions(
    existing: Option<crate::ConditionExpr>,
    additional: crate::ConditionExpr,
) -> crate::ConditionExpr {
    match existing {
        Some(existing) => crate::ConditionExpr::And(Box::new(existing), Box::new(additional)),
        None => additional,
    }
}

fn add_static_ability_ast_condition(
    ability: StaticAbilityAst,
    condition: crate::ConditionExpr,
) -> Result<StaticAbilityAst, CardTextError> {
    Ok(match ability {
        StaticAbilityAst::Static(_) | StaticAbilityAst::KeywordAction(_) => {
            StaticAbilityAst::ConditionalStaticAbility {
                ability: Box::new(ability),
                condition,
            }
        }
        StaticAbilityAst::ConditionalStaticAbility {
            ability,
            condition: existing,
        } => StaticAbilityAst::ConditionalStaticAbility {
            ability,
            condition: crate::ConditionExpr::And(Box::new(existing), Box::new(condition)),
        },
        StaticAbilityAst::ConditionalKeywordAction {
            action,
            condition: existing,
        } => StaticAbilityAst::ConditionalKeywordAction {
            action,
            condition: crate::ConditionExpr::And(Box::new(existing), Box::new(condition)),
        },
        StaticAbilityAst::GrantStaticAbility {
            filter,
            ability,
            condition: existing,
        } => StaticAbilityAst::GrantStaticAbility {
            filter,
            ability,
            condition: Some(merge_static_ability_ast_conditions(existing, condition)),
        },
        StaticAbilityAst::GrantKeywordAction {
            filter,
            action,
            condition: existing,
        } => StaticAbilityAst::GrantKeywordAction {
            filter,
            action,
            condition: Some(merge_static_ability_ast_conditions(existing, condition)),
        },
        StaticAbilityAst::AttachedStaticAbilityGrant {
            ability,
            display,
            condition: existing,
        } => StaticAbilityAst::AttachedStaticAbilityGrant {
            ability,
            display,
            condition: Some(merge_static_ability_ast_conditions(existing, condition)),
        },
        StaticAbilityAst::AttachedKeywordActionGrant {
            action,
            display,
            condition: existing,
        } => StaticAbilityAst::AttachedKeywordActionGrant {
            action,
            display,
            condition: Some(merge_static_ability_ast_conditions(existing, condition)),
        },
        StaticAbilityAst::AttachedChosenLandwalkGrant {
            snow,
            display,
            condition: existing,
        } => StaticAbilityAst::AttachedChosenLandwalkGrant {
            snow,
            display,
            condition: Some(merge_static_ability_ast_conditions(existing, condition)),
        },
        StaticAbilityAst::GrantObjectAbility {
            filter,
            ability,
            display,
            condition: existing,
        } => StaticAbilityAst::GrantObjectAbility {
            filter,
            ability,
            display,
            condition: Some(merge_static_ability_ast_conditions(existing, condition)),
        },
        StaticAbilityAst::AttachedObjectAbilityGrant {
            ability,
            display,
            condition: existing,
        } => StaticAbilityAst::AttachedObjectAbilityGrant {
            ability,
            display,
            condition: Some(merge_static_ability_ast_conditions(existing, condition)),
        },
        StaticAbilityAst::RemoveStaticAbility { .. }
        | StaticAbilityAst::RemoveKeywordAction { .. }
        | StaticAbilityAst::EquipmentKeywordActionsGrant { .. }
        | StaticAbilityAst::SoulbondSharedObjectAbility { .. }
        | StaticAbilityAst::AttachmentRestriction { .. } => {
            return Err(CardTextError::ParseError(
                "cannot apply leading static condition to unsupported static ability shape"
                    .to_string(),
            ));
        }
    })
}

pub(crate) fn parse_protection_from_colored_spells_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if anthem_grant_grammar::parse_colored_spell_protection_tokens(tokens).is_none() {
        return Ok(None);
    }

    let all_colors = crate::color::ColorSet::WHITE
        .union(crate::color::ColorSet::BLUE)
        .union(crate::color::ColorSet::BLACK)
        .union(crate::color::ColorSet::RED)
        .union(crate::color::ColorSet::GREEN);
    let mut filter = ObjectFilter::spell();
    filter.colors = Some(all_colors);
    Ok(Some(StaticAbility::protection(
        crate::ability::ProtectionFrom::Permanents(filter),
    )))
}

fn grant_for_anthem_subject(
    clause: &ParsedAnthemClause,
    ability: StaticAbility,
) -> StaticAbilityAst {
    match &clause.subject {
        AnthemSubjectAst::Source => match &clause.condition {
            Some(condition) => StaticAbilityAst::ConditionalStaticAbility {
                ability: Box::new(StaticAbilityAst::Static(ability)),
                condition: condition.clone(),
            },
            None => StaticAbilityAst::Static(ability),
        },
        AnthemSubjectAst::Filter(filter) => StaticAbilityAst::GrantStaticAbility {
            filter: filter.clone(),
            ability: Box::new(StaticAbilityAst::Static(ability)),
            condition: clause.condition.clone(),
        },
    }
}

fn parse_every_subtype_family_tail(words: &[&str]) -> Option<crate::types::SubtypeFamily> {
    EVERY_SUBTYPE_FAMILY_TAILS
        .iter()
        .find_map(|(phrase, family)| (*phrase == words).then_some(*family))
}

fn every_subtype_family_for_subject(
    subject: &AnthemSubjectAst,
    family: crate::types::SubtypeFamily,
    condition: Option<crate::ConditionExpr>,
) -> StaticAbilityAst {
    let base = match subject {
        AnthemSubjectAst::Source => {
            StaticAbility::add_all_subtypes_of_family(ObjectFilter::source(), family)
        }
        AnthemSubjectAst::Filter(filter) => {
            StaticAbility::add_all_subtypes_of_family(filter.clone(), family)
        }
    };

    let ability = condition
        .as_ref()
        .map(|cond| base.clone().with_condition(cond.clone()))
        .unwrap_or({
            #[cfg(not(feature = "serialization"))]
            {
                base
            }
            #[cfg(feature = "serialization")]
            {
                Some(base)
            }
        });
    #[cfg(not(feature = "serialization"))]
    {
        StaticAbilityAst::Static(ability)
    }
    #[cfg(feature = "serialization")]
    {
        StaticAbilityAst::Static(ability.expect("runtime static ability should exist"))
    }
}

fn grant_keyword_action_for_anthem_subject(
    clause: &ParsedAnthemClause,
    action: KeywordAction,
) -> StaticAbilityAst {
    match &clause.subject {
        AnthemSubjectAst::Source => match &clause.condition {
            Some(condition) => StaticAbilityAst::ConditionalKeywordAction {
                action,
                condition: condition.clone(),
            },
            None => StaticAbilityAst::KeywordAction(action),
        },
        AnthemSubjectAst::Filter(filter) => StaticAbilityAst::GrantKeywordAction {
            filter: filter.clone(),
            action,
            condition: clause.condition.clone(),
        },
    }
}

fn granted_object_ability_for_keyword_action(
    action: &KeywordAction,
) -> Option<(ParsedAbility, String)> {
    match action {
        KeywordAction::Afflict(amount) => Some((
            parsed_ability_from_ability(afflict_triggered_ability(*amount)),
            action.display_text(),
        )),
        _ => None,
    }
}

fn parse_if_its_color_tail(tokens: &[OwnedLexToken]) -> Option<ColorSet> {
    crate::runtime_backend::grammar::anthem_grants::parse_if_source_is_color(tokens)
}

fn parse_keyword_if_color_segment(
    segment: &[OwnedLexToken],
    clause_text: &str,
) -> Result<Option<(Vec<KeywordAction>, ColorSet)>, CardTextError> {
    let Some(shape) = anthem_grant_grammar::parse_keyword_if_color_shape(segment) else {
        return Ok(None);
    };
    let Some(color) = parse_if_its_color_tail(shape.color_tail_tokens) else {
        return Ok(None);
    };
    let Some(actions) = parse_ability_line(shape.keyword_tokens) else {
        return Ok(None);
    };
    reject_unimplemented_keyword_actions(&actions, clause_text)?;
    let actions = actions
        .into_iter()
        .filter(|action| action.lowers_to_static_ability())
        .collect::<Vec<_>>();
    if actions.is_empty() {
        return Ok(None);
    }
    Ok(Some((actions, color)))
}

fn color_filtered_grant_filter(mut filter: ObjectFilter, color: ColorSet) -> ObjectFilter {
    let existing = filter.colors.unwrap_or(ColorSet::new());
    filter.colors = Some(existing.union(color));
    filter
}

fn source_color_condition(color: ColorSet) -> crate::ConditionExpr {
    let mut filter = ObjectFilter::source();
    filter.colors = Some(color);
    crate::ConditionExpr::SourceMatches(filter)
}

fn append_condition(
    condition: Option<crate::ConditionExpr>,
    next: crate::ConditionExpr,
) -> crate::ConditionExpr {
    match condition {
        Some(existing) => crate::ConditionExpr::And(Box::new(existing), Box::new(next)),
        None => next,
    }
}

fn parse_color_filtered_keyword_grants(
    subject_tokens: &[OwnedLexToken],
    keyword_tokens: &[OwnedLexToken],
    condition: Option<crate::ConditionExpr>,
    clause_text: &str,
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let mut parsed_segments = Vec::new();
    for segment in anthem_grant_grammar::split_keyword_if_color_segments(keyword_tokens) {
        let Some(parsed) = parse_keyword_if_color_segment(segment, clause_text)? else {
            return Ok(None);
        };
        parsed_segments.push(parsed);
    }
    if parsed_segments.is_empty() {
        return Ok(None);
    }

    let subject = parse_anthem_subject(subject_tokens)?;
    let mut compiled = Vec::new();
    for (actions, color) in parsed_segments {
        for action in actions {
            match &subject {
                AnthemSubjectAst::Source => {
                    compiled.push(StaticAbilityAst::ConditionalKeywordAction {
                        action,
                        condition: append_condition(
                            condition.clone(),
                            source_color_condition(color),
                        ),
                    })
                }
                AnthemSubjectAst::Filter(filter) => {
                    compiled.push(StaticAbilityAst::GrantKeywordAction {
                        filter: color_filtered_grant_filter(filter.clone(), color),
                        action,
                        condition: condition.clone(),
                    });
                }
            }
        }
    }

    Ok(Some(compiled))
}

fn anthem_subject_filter(subject: &AnthemSubjectAst) -> ObjectFilter {
    match subject {
        AnthemSubjectAst::Source => ObjectFilter::source(),
        AnthemSubjectAst::Filter(filter) => filter.clone(),
    }
}

fn grant_object_ability_for_anthem_subject(
    clause: &ParsedAnthemClause,
    ability: ParsedAbility,
    display: String,
) -> StaticAbilityAst {
    if let Some(filter) = attached_object_anthem_subject_filter(&clause.subject) {
        let subject = filter.description();
        return StaticAbilityAst::AttachedObjectAbilityGrant {
            ability,
            display: format!("{subject} has {display}"),
            condition: clause.condition.clone(),
        };
    }

    StaticAbilityAst::GrantObjectAbility {
        filter: anthem_subject_filter(&clause.subject),
        ability,
        display,
        condition: clause.condition.clone(),
    }
}

fn attached_object_anthem_subject_filter(subject: &AnthemSubjectAst) -> Option<&ObjectFilter> {
    let AnthemSubjectAst::Filter(filter) = subject else {
        return None;
    };
    filter
        .tagged_constraints
        .iter()
        .any(|constraint| {
            matches!(
                constraint.relation,
                crate::filter::TaggedOpbjectRelation::IsTaggedObject
            ) && matches!(constraint.tag.as_str(), "enchanted" | "equipped")
        })
        .then_some(filter)
}

fn parsed_ability_from_ability(ability: Ability) -> ParsedAbility {
    ParsedAbility {
        ability: ability.into(),
        text: None,
        effects_ast: None,
        reference_imports: ReferenceImports::default(),
        trigger_spec: None,
    }
}

pub(crate) fn parse_equipment_you_control_have_equip_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let Some(shape) = anthem_grant_grammar::parse_equipment_equip_shape(tokens) else {
        return Ok(None);
    };
    let condition = parse_static_condition_clause(shape.condition_tokens)?;
    let total_cost = parse_activation_cost(shape.cost_tokens)?;
    let target = ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature().you_control()));
    let ability = ParsedAbility {
        ability: Ability {
            kind: AbilityKind::Activated(crate::ability::ActivatedAbility {
                mana_cost: total_cost,
                effects: crate::resolution::ResolutionProgram::from_effects(vec![
                    Effect::attach_to(target.clone()),
                ]),
                choices: vec![target],
                timing: crate::ability::ActivationTiming::SorcerySpeed,
                additional_restrictions: vec![],
                activation_restrictions: vec![],
                mana_output: None,
                activation_condition: None,
                mana_usage_restrictions: vec![],
                is_loyalty_ability: false,
            }),
            functional_zones: vec![Zone::Battlefield],
        }
        .into(),
        text: Some("Equip {0}".to_string()),
        effects_ast: None,
        reference_imports: ReferenceImports::default(),
        trigger_spec: None,
    };
    Ok(Some(vec![StaticAbilityAst::GrantObjectAbility {
        filter: ObjectFilter::default()
            .with_subtype(Subtype::Equipment)
            .you_control(),
        ability,
        display: "Equipment you control have equip {0}".to_string(),
        condition: Some(condition),
    }]))
}

fn parsed_exploit_ability() -> ParsedAbility {
    let effect_id = 0;
    let ability = Ability::triggered(
        Trigger::this_enters_battlefield(),
        vec![
            Effect::with_id(
                effect_id,
                Effect::may(vec![Effect::sacrifice(ObjectFilter::creature(), 1)]),
            ),
            Effect::if_then(
                effect_id,
                crate::effect::EffectPredicate::Happened,
                vec![Effect::emit_keyword_action_with_affected_object_memory_tag(
                    crate::events::KeywordActionKind::Exploit,
                    1,
                    crate::effect::EffectId(effect_id),
                    crate::tag::EXPLOITED_TAG,
                )],
            ),
        ],
    );
    ParsedAbility {
        ability: ability.into(),
        text: Some("Exploit".to_string()),
        effects_ast: None,
        reference_imports: ReferenceImports::default(),
        trigger_spec: Some(TriggerSpec::ThisEntersBattlefield),
    }
}

fn grant_exploit_for_anthem_subject(
    subject: &AnthemSubjectAst,
    condition: Option<crate::ConditionExpr>,
) -> StaticAbilityAst {
    StaticAbilityAst::GrantObjectAbility {
        filter: anthem_subject_filter(subject),
        ability: parsed_exploit_ability(),
        display: "exploit".to_string(),
        condition,
    }
}

fn parse_triggered_granted_ability(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedAbility>, CardTextError> {
    let trigger_tokens = trim_edge_punctuation(tokens);
    if trigger_tokens.is_empty() {
        return Ok(None);
    }
    let intro = crate::runtime_backend::grammar::clause_support::parse_trigger_intro_tokens(
        &trigger_tokens,
    );
    if intro.body_first == 0 {
        return Ok(None);
    }

    let ability = match crate::runtime_backend::clause_support::parse_triggered_line_lexed(
        &trigger_tokens,
    )? {
        LineAst::Triggered {
            trigger,
            effects,
            max_triggers_per_turn,
        } => {
            let (effects, trigger_condition) =
                triggered_grant_effects_and_condition(&trigger, &effects)?;
            let max_condition = trigger_surface::parse_trigger_frequency_condition_tokens(
                &trigger_tokens,
                max_triggers_per_turn,
            );
            let intervening_if = match (trigger_condition, max_condition) {
                (Some(left), Some(right)) => {
                    Some(crate::ConditionExpr::And(Box::new(left), Box::new(right)))
                }
                (Some(condition), None) | (None, Some(condition)) => Some(condition),
                (None, None) => None,
            };
            parsed_triggered_ability(
                trigger,
                effects,
                vec![Zone::Battlefield],
                Some(crate::runtime_backend::token_word_refs(&trigger_tokens).join(" ")),
                intervening_if,
                None,
                ReferenceImports::default(),
            )
        }
        _ => return Ok(None),
    };
    if parsed_triggered_ability_is_empty(&ability) {
        return Err(CardTextError::ParseError(format!(
            "unsupported empty triggered granted ability clause (clause: '{}')",
            crate::runtime_backend::token_word_refs(&trigger_tokens).join(" ")
        )));
    }
    Ok(Some(ability))
}

fn parsed_triggered_ability_is_empty(ability: &ParsedAbility) -> bool {
    matches!(
        ability.kind(),
        AbilityKind::Triggered(triggered)
            if triggered.effects.is_empty()
                && ability
                    .effects_ast
                    .as_ref()
                    .is_none_or(|effects| effects.is_empty())
    )
}

fn parse_granted_keyword_fragment(segment: &[OwnedLexToken]) -> Option<Vec<KeywordAction>> {
    parse_ability_line(segment).or_else(|| {
        anthem_grant_grammar::parse_unblockable_keyword_fragment_tokens(segment)
            .map(|action| vec![action])
    })
}

fn parse_granted_object_ability_segment(
    raw_segment: &[OwnedLexToken],
    clause_words: &[&str],
    attached_subject: bool,
) -> Result<Option<(ParsedAbility, String)>, CardTextError> {
    let sanitized_tokens = raw_segment
        .iter()
        .filter(|token| token.kind != TokenKind::Quote)
        .cloned()
        .collect::<Vec<_>>();
    let ability_tokens = trim_edge_punctuation(&sanitized_tokens);
    if ability_tokens.is_empty() {
        return Ok(None);
    }

    if let Some(actions) = parse_ability_line(&ability_tokens)
        && actions.len() == 1
        && let Some(granted) = nonstatic_keyword_action_as_granted_object_ability(
            actions.into_iter().next().expect("single action exists"),
        )
    {
        return Ok(Some(granted));
    }

    if attached_subject && contains_token_kind(&ability_tokens, TokenKind::Colon) {
        let Some(parsed) = parse_attached_granted_activated_line(raw_segment)? else {
            return Err(CardTextError::ParseError(format!(
                "unsupported granted activated/triggered ability clause (clause: '{}')",
                clause_words.join(" ")
            )));
        };
        return Ok(Some((
            parsed,
            display_text_for_tokens(&ability_tokens, false),
        )));
    }

    if let Some(GrantedAbilityAst::ParsedObjectAbility { ability, display }) =
        parse_granted_activated_or_triggered_ability_for_gain(&ability_tokens, clause_words)?
    {
        return Ok(Some((ability, display)));
    }

    if let Some(parsed) = parse_attached_nonstatic_keyword_ability(&ability_tokens)? {
        return Ok(Some(parsed));
    }

    if let Some(parsed) = parse_cycling_line(&ability_tokens)? {
        return Ok(Some((
            parsed,
            display_text_for_tokens(&ability_tokens, false),
        )));
    }

    if contains_token_kind(&ability_tokens, TokenKind::Colon) {
        let Some(parsed) = parse_activated_line(&ability_tokens)? else {
            return Err(CardTextError::ParseError(format!(
                "unsupported granted activated/triggered ability clause (clause: '{}')",
                clause_words.join(" ")
            )));
        };
        return Ok(Some((
            parsed,
            display_text_for_tokens(&ability_tokens, false),
        )));
    }

    Ok(None)
}

fn nonstatic_keyword_action_as_granted_object_ability(
    action: KeywordAction,
) -> Option<(ParsedAbility, String)> {
    match action {
        KeywordAction::Soulshift(amount) => {
            let ability =
                crate::CardDefinitionBuilder::new(crate::CardId::from_raw(0), "Soulshift")
                    .soulshift(amount)
                    .build()
                    .abilities
                    .into_iter()
                    .next()?;
            Some((
                parsed_ability_from_ability(ability),
                format!("Soulshift {amount}"),
            ))
        }
        KeywordAction::SoulshiftValue(value) => Some((
            parsed_ability_from_ability(
                crate::CardDefinitionBuilder::soulshift_triggered_ability_from_value(value.clone()),
            ),
            format!(
                "Soulshift X, where X is {}",
                crate::payload::describe_soulshift_value(&value)
            ),
        )),
        KeywordAction::Casualty(power) => {
            let mut creature_filter = ObjectFilter::creature().you_control();
            creature_filter.power =
                Some(crate::filter::Comparison::GreaterThanOrEqual(power as i32));
            let ability = Ability {
                kind: AbilityKind::Triggered(TriggeredAbility {
                    trigger: Trigger::you_cast_this_spell(),
                    effects: crate::resolution::ResolutionProgram::from_effects(vec![Effect::may(
                        vec![
                            Effect::sacrifice(creature_filter, 1),
                            Effect::with_id(
                                0,
                                Effect::new(crate::effects::CopySpellEffect::single(
                                    ChooseSpec::Source,
                                )),
                            ),
                            Effect::may_choose_new_targets_player(
                                crate::effect::EffectId(0),
                                PlayerFilter::You,
                            ),
                        ],
                    )]),
                    choices: Vec::new(),
                    intervening_if: None,
                    presentation_label: Some(PresentationLabel::Keyword(
                        PresentationKeyword::Casualty(power),
                    )),
                }),
                functional_zones: vec![Zone::Stack],
            };
            Some((
                ParsedAbility {
                    ability: ability.into(),
                    text: Some(format!("Casualty {power}")),
                    effects_ast: None,
                    reference_imports: ReferenceImports::default(),
                    trigger_spec: None,
                },
                format!("Casualty {power}"),
            ))
        }
        _ => None,
    }
}

pub(crate) fn parse_heterogeneous_granted_tail(
    tail_tokens: &[OwnedLexToken],
    clause_words: &[&str],
    attached_subject: bool,
) -> Result<Option<ParsedGrantedTailAst>, CardTextError> {
    let mut parsed = ParsedGrantedTailAst::default();

    for raw_segment in anthem_grant_grammar::split_trailing_grant_segments(tail_tokens) {
        let trimmed = trim_commas(&raw_segment);
        let mut segment = trim_edge_punctuation(&trimmed);
        while token_slice_first_is(&segment, "and") {
            let trimmed = trim_commas(&segment[1..]);
            segment = trim_edge_punctuation(&trimmed);
        }
        if segment.is_empty() {
            continue;
        }

        if let Some((ability, display)) =
            parse_granted_object_ability_segment(&segment, clause_words, attached_subject)?
        {
            parsed.granted_object_abilities.push((ability, display));
            continue;
        }

        if let Some(actions) = parse_granted_keyword_fragment(&segment) {
            reject_unimplemented_keyword_actions(&actions, &clause_words.join(" "))?;
            if let [KeywordAction::CumulativeUpkeep { total_cost, .. }] = actions.as_slice() {
                parsed.granted_object_abilities.push((
                    ParsedAbility {
                        ability: cumulative_upkeep_granted_ability(total_cost.clone()).into(),
                        text: Some(display_text_for_tokens(&segment, false)),
                        effects_ast: None,
                        reference_imports: ReferenceImports::default(),
                        trigger_spec: None,
                    },
                    display_text_for_tokens(&segment, false),
                ));
                continue;
            }

            let lowered = actions
                .into_iter()
                .filter(|action| action.lowers_to_static_ability())
                .collect::<Vec<_>>();
            if lowered.is_empty() {
                return Ok(None);
            }
            parsed.granted_keyword_actions.extend(lowered);
            continue;
        }

        let split_actions = split_lexed_slices_on_and(&segment)
            .into_iter()
            .map(trim_edge_punctuation)
            .filter(|part| !part.is_empty())
            .map(|part| parse_granted_keyword_fragment(&part))
            .collect::<Vec<_>>();
        if split_actions.len() > 1
            && split_actions.iter().all(|actions| {
                actions.as_ref().is_some_and(|actions| {
                    actions.iter().all(KeywordAction::lowers_to_static_ability)
                })
            })
        {
            for actions in split_actions.into_iter().flatten() {
                reject_unimplemented_keyword_actions(&actions, &clause_words.join(" "))?;
                parsed.granted_keyword_actions.extend(actions);
            }
            continue;
        }

        if let Some(marker) = parse_static_text_marker_line(&segment) {
            parsed.granted_static.push(marker.into());
            continue;
        }

        let mut segment_with_period = segment.to_vec();
        segment_with_period.push(OwnedLexToken::period(
            crate::cards::builders::TextSpan::synthetic(),
        ));
        if let Some(marker) = parse_static_text_marker_line(&segment_with_period) {
            parsed.granted_static.push(marker.into());
            continue;
        }

        if let Some(abilities) = parse_static_ability_ast_line_lexed(&segment)? {
            parsed.granted_static.extend(abilities);
            continue;
        }

        return Ok(None);
    }

    if parsed.granted_static.is_empty()
        && parsed.granted_keyword_actions.is_empty()
        && parsed.granted_object_abilities.is_empty()
    {
        return Ok(None);
    }

    Ok(Some(parsed))
}

pub(crate) fn lower_granted_tail_for_anthem_subject(
    subject: &AnthemSubjectAst,
    condition: &Option<crate::ConditionExpr>,
    granted_tail: ParsedGrantedTailAst,
) -> Vec<StaticAbilityAst> {
    let wrapper_clause = ParsedAnthemClause {
        subject: subject.clone(),
        power: AnthemValue::Fixed(0),
        toughness: AnthemValue::Fixed(0),
        condition: condition.clone(),
        count_uses_where_x: false,
    };
    let mut granted = Vec::new();
    if !granted_tail.granted_static.is_empty() {
        granted.extend(grant_static_anthem_abilities_for_subject(
            &wrapper_clause,
            granted_tail.granted_static,
        ));
    }
    for action in granted_tail.granted_keyword_actions {
        granted.push(grant_keyword_action_for_anthem_subject(
            &wrapper_clause,
            action,
        ));
    }
    for (ability, display) in granted_tail.granted_object_abilities {
        granted.push(grant_object_ability_for_anthem_subject(
            &wrapper_clause,
            ability,
            display,
        ));
    }
    granted
}

fn wrap_conditioned_animation_static_ability(
    ability: StaticAbility,
    condition: &Option<crate::ConditionExpr>,
) -> StaticAbilityAst {
    if let Some(condition) = condition {
        #[cfg(not(feature = "serialization"))]
        {
            return ability.with_condition(condition.clone()).into();
        }
        #[cfg(feature = "serialization")]
        {
            return ability
                .with_condition(condition.clone())
                .expect("runtime conditioned static ability should exist")
                .into();
        }
    }
    ability.into()
}

pub(crate) fn lower_static_animation_bundle(
    bundle: StaticAnimationBundleAst,
) -> Vec<StaticAbilityAst> {
    let filter = anthem_subject_filter(&bundle.subject);
    let mut lowered = Vec::new();

    if bundle.ensure_creature_type {
        lowered.push(wrap_conditioned_animation_static_ability(
            StaticAbility::add_card_types(filter.clone(), vec![CardType::Creature]),
            &bundle.condition,
        ));
    }
    if let Some((power, toughness)) = bundle.base_power_toughness {
        let ability = match (&power, &toughness) {
            (Value::Fixed(power), Value::Fixed(toughness)) => {
                StaticAbility::set_base_power_toughness(filter.clone(), *power, *toughness)
            }
            _ => StaticAbility::set_base_power_toughness_value(filter.clone(), power, toughness),
        };
        lowered.push(wrap_conditioned_animation_static_ability(
            ability,
            &bundle.condition,
        ));
    }
    if !bundle.subtypes.is_empty() {
        let ability = match bundle.subtype_mode {
            AnimationSubtypeMode::Add => StaticAbility::add_subtypes(filter, bundle.subtypes),
            AnimationSubtypeMode::ReplaceCreatureTypes => {
                StaticAbility::set_creature_subtypes(filter, bundle.subtypes)
            }
        };
        lowered.push(wrap_conditioned_animation_static_ability(
            ability,
            &bundle.condition,
        ));
    }

    lowered.extend(lower_granted_tail_for_anthem_subject(
        &bundle.subject,
        &bundle.condition,
        bundle.granted_tail,
    ));

    lowered
}

fn grant_static_anthem_abilities_for_subject(
    clause: &ParsedAnthemClause,
    abilities: Vec<StaticAbilityAst>,
) -> Vec<StaticAbilityAst> {
    let mut granted = Vec::new();
    for ability in abilities {
        granted.push(match &clause.subject {
            AnthemSubjectAst::Source => match &clause.condition {
                Some(condition) => StaticAbilityAst::ConditionalStaticAbility {
                    ability: Box::new(ability),
                    condition: condition.clone(),
                },
                None => ability,
            },
            AnthemSubjectAst::Filter(filter) => StaticAbilityAst::GrantStaticAbility {
                filter: filter.clone(),
                ability: Box::new(ability),
                condition: clause.condition.clone(),
            },
        });
    }
    granted
}

fn parse_continuing_anthem_granted_segment(
    clause: &ParsedAnthemClause,
    clause_words: &[&str],
    segment: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let sanitized_tokens = segment
        .iter()
        .filter(|token| token.kind != TokenKind::Quote)
        .cloned()
        .collect::<Vec<_>>();
    let ability_tokens = trim_edge_punctuation(&sanitized_tokens);
    if ability_tokens.is_empty() {
        return Ok(None);
    }

    if let Some(GrantedAbilityAst::ParsedObjectAbility { ability, display }) =
        parse_granted_activated_or_triggered_ability_for_gain(&ability_tokens, clause_words)?
    {
        return Ok(Some(vec![grant_object_ability_for_anthem_subject(
            clause, ability, display,
        )]));
    }

    if let Some(actions) = parse_ability_line(&ability_tokens) {
        reject_unimplemented_keyword_actions(&actions, &clause_words.join(" "))?;
        let granted = actions
            .into_iter()
            .filter_map(keyword_action_to_static_ability)
            .collect::<Vec<_>>();
        if granted.is_empty() {
            return Ok(None);
        }
        return Ok(Some(
            granted
                .into_iter()
                .map(|ability| grant_for_anthem_subject(clause, ability))
                .collect(),
        ));
    }

    if let Some(marker) = parse_static_text_marker_line(&ability_tokens) {
        return Ok(Some(vec![grant_for_anthem_subject(clause, marker)]));
    }

    let mut ability_tokens_with_period = ability_tokens.to_vec();
    ability_tokens_with_period.push(OwnedLexToken::period(
        crate::cards::builders::TextSpan::synthetic(),
    ));
    if let Some(amount) =
        super::grammar::abilities::parse_ward_pay_life_amount_lexed(&ability_tokens_with_period)
    {
        return Ok(Some(vec![grant_for_anthem_subject(
            clause,
            StaticAbility::ward(crate::cost::TotalCost::from_cost(crate::costs::Cost::life(
                amount,
            ))),
        )]));
    }
    if let Some(marker) = parse_static_text_marker_line(&ability_tokens_with_period) {
        return Ok(Some(vec![grant_for_anthem_subject(clause, marker)]));
    }

    if let Some(abilities) = parse_static_ability_ast_line_lexed(&ability_tokens)? {
        return Ok(Some(grant_static_anthem_abilities_for_subject(
            clause, abilities,
        )));
    }

    Ok(None)
}

pub(crate) fn parse_anthem_with_trailing_segments_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    let Some(head) = anthem_grant_grammar::parse_persistent_anthem_tail_head(tokens) else {
        return Ok(None);
    };
    let get_idx = head.get_token;
    let work_tokens = head.tokens;
    if parse_pt_modifier(&head.modifier_word).is_err() {
        return Ok(None);
    }

    let clause = parse_anthem_clause(&work_tokens, get_idx, head.tail_start)?;
    let tail_tokens = trim_commas(&work_tokens[head.tail_start..]);
    if tail_tokens.is_empty() {
        return Ok(None);
    }

    let direct_have_tail =
        anthem_grant_grammar::parse_direct_have_tail(&tail_tokens).map(|tokens| tokens.to_vec());

    if let Some(grant_tail) = direct_have_tail {
        let mut extras: Vec<StaticAbilityAst> = Vec::new();
        for raw_segment in anthem_grant_grammar::split_trailing_grant_segments(&grant_tail) {
            let trimmed = trim_commas(&raw_segment);
            let mut segment = trim_edge_punctuation(&trimmed);
            while token_slice_first_is(&segment, "and") {
                let trimmed = trim_commas(&segment[1..]);
                segment = trim_edge_punctuation(&trimmed);
            }
            if segment.is_empty() {
                continue;
            }

            if let Some(mut granted) =
                parse_continuing_anthem_granted_segment(&clause, &clause_words, &segment)?
            {
                extras.append(&mut granted);
                continue;
            }

            let segment_shape = anthem_grant_grammar::parse_continuing_segment_shape(&segment);
            if segment_shape == anthem_grant_grammar::ContinuingSegmentShape::MustAttack {
                extras.push(grant_for_anthem_subject(&clause, StaticAbility::must_attack()).into());
                continue;
            }
            if segment_shape == anthem_grant_grammar::ContinuingSegmentShape::CantAttackAlone {
                extras.push(
                    grant_for_anthem_subject(
                        &clause,
                        StaticAbility::restriction(
                            crate::effect::Restriction::attack_alone(ObjectFilter::source()),
                            "This creature can't attack alone".to_string(),
                        ),
                    )
                    .into(),
                );
                continue;
            }

            return Ok(None);
        }

        if extras.is_empty() {
            return Ok(None);
        }

        let mut result = vec![build_anthem_static_ability(&clause).into()];
        result.extend(extras);
        return Ok(Some(result));
    }

    let mut extras: Vec<StaticAbilityAst> = Vec::new();
    let mut continuing_have_clause = false;
    for raw_segment in anthem_grant_grammar::split_trailing_grant_segments(&tail_tokens) {
        let trimmed = trim_commas(&raw_segment);
        let mut segment = trim_edge_punctuation(&trimmed);
        while token_slice_first_is(&segment, "and") {
            let trimmed = trim_commas(&segment[1..]);
            segment = trim_edge_punctuation(&trimmed);
        }
        if segment.is_empty() {
            continue;
        }

        let segment_shape = anthem_grant_grammar::parse_continuing_segment_shape(&segment);
        if segment_shape == anthem_grant_grammar::ContinuingSegmentShape::CantBlock {
            extras.push(grant_for_anthem_subject(&clause, StaticAbility::cant_block()).into());
            continue;
        }
        if segment_shape == anthem_grant_grammar::ContinuingSegmentShape::CantAttackAlone {
            extras.push(
                grant_for_anthem_subject(
                    &clause,
                    StaticAbility::restriction(
                        crate::effect::Restriction::attack_alone(ObjectFilter::source()),
                        "This creature can't attack alone".to_string(),
                    ),
                )
                .into(),
            );
            continue;
        }
        if segment_shape == anthem_grant_grammar::ContinuingSegmentShape::MustAttack {
            extras.push(grant_for_anthem_subject(&clause, StaticAbility::must_attack()).into());
            continue;
        }
        if let anthem_grant_grammar::ContinuingSegmentShape::CantBeBlockedByMoreThan(count) =
            segment_shape
        {
            extras.push(
                grant_for_anthem_subject(
                    &clause,
                    StaticAbility::cant_be_blocked_by_more_than(count),
                )
                .into(),
            );
            continue;
        }
        if let anthem_grant_grammar::ContinuingSegmentShape::SetColor { color_word } = segment_shape
        {
            let Some(color) = parse_color(color_word) else {
                return Ok(None);
            };
            let filter = match &clause.subject {
                AnthemSubjectAst::Source => ObjectFilter::source(),
                AnthemSubjectAst::Filter(filter) => filter.clone(),
            };
            let mut set_colors = crate::static_abilities::SetColorsForFilter::new(filter, color);
            if let Some(condition) = &clause.condition {
                set_colors = set_colors.with_condition(condition.clone());
            }
            extras.push(StaticAbility::new(set_colors).into());
            continue;
        }

        if let anthem_grant_grammar::ContinuingSegmentShape::Lose { ability_tokens } = segment_shape
        {
            let ability_tokens = trim_edge_punctuation(ability_tokens);
            if ability_tokens.is_empty() {
                return Ok(None);
            }
            let Some(actions) = parse_ability_line(&ability_tokens) else {
                return Ok(None);
            };
            reject_unimplemented_keyword_actions(&actions, &clause_words.join(" "))?;
            let removed = actions
                .into_iter()
                .filter_map(|action| keyword_action_to_static_ability(action))
                .collect::<Vec<_>>();
            if removed.is_empty() {
                return Ok(None);
            }
            for ability in removed {
                extras.push(match &clause.subject {
                    AnthemSubjectAst::Source => StaticAbilityAst::RemoveStaticAbility {
                        filter: ObjectFilter::source(),
                        ability: Box::new(StaticAbilityAst::Static(ability)),
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
            continue;
        }

        if let anthem_grant_grammar::ContinuingSegmentShape::Have { ability_tokens } = segment_shape
        {
            let mut ability_tokens = trim_edge_punctuation(ability_tokens);
            if ability_tokens.is_empty() {
                return Ok(None);
            }

            let mut grant_must_attack = false;
            if let Some(head) = anthem_grant_grammar::strip_must_attack_suffix(&ability_tokens) {
                ability_tokens = head.to_vec();
                grant_must_attack = true;
            }

            let mut granted_activated: Option<ParsedAbility> = None;
            let mut granted_activated_display: Option<String> = None;
            let split_keyword_and_activated = if let Some(split) =
                anthem_grant_grammar::split_keyword_and_activated(&ability_tokens)
            {
                let keyword_head = trim_edge_punctuation(split.keyword_tokens);
                let activated_tail = trim_edge_punctuation(split.activated_tokens);
                if keyword_head.is_empty() || activated_tail.is_empty() {
                    return Ok(None);
                }
                let Some(actions) = parse_ability_line(&keyword_head) else {
                    return Ok(None);
                };
                let has_colon = contains_token_kind(&activated_tail, TokenKind::Colon);
                let Some(parsed) = parse_activated_line(&activated_tail)? else {
                    if has_colon {
                        return Err(CardTextError::ParseError(format!(
                            "unsupported granted activated ability in anthem clause (clause: '{}')",
                            clause_words.join(" ")
                        )));
                    }
                    return Ok(None);
                };
                let display = display_text_for_tokens(&activated_tail, false);
                granted_activated_display = Some(display);
                granted_activated = Some(parsed);
                Some(actions)
            } else {
                None
            };
            let actions = if let Some(actions) = split_keyword_and_activated {
                Some(actions)
            } else if let Some(GrantedAbilityAst::ParsedObjectAbility { ability, display }) =
                parse_granted_activated_or_triggered_ability_for_gain(
                    &ability_tokens,
                    &clause_words,
                )?
            {
                granted_activated_display = Some(display);
                granted_activated = Some(ability);
                None
            } else if let Some(actions) = parse_ability_line(&ability_tokens) {
                Some(actions)
            } else if contains_token_kind(&ability_tokens, TokenKind::Colon) {
                let Some(split) =
                    anthem_grant_grammar::split_keyword_and_activated(&ability_tokens)
                else {
                    return Ok(None);
                };
                let keyword_head = trim_edge_punctuation(split.keyword_tokens);
                let activated_tail = trim_edge_punctuation(split.activated_tokens);
                if keyword_head.is_empty() || activated_tail.is_empty() {
                    return Ok(None);
                }
                let Some(actions) = parse_ability_line(&keyword_head) else {
                    return Ok(None);
                };
                let has_colon = contains_token_kind(&activated_tail, TokenKind::Colon);
                let Some(parsed) = parse_activated_line(&activated_tail)? else {
                    if has_colon {
                        return Err(CardTextError::ParseError(format!(
                            "unsupported granted activated ability in anthem clause (clause: '{}')",
                            clause_words.join(" ")
                        )));
                    }
                    return Ok(None);
                };
                let display = display_text_for_tokens(&activated_tail, false);
                granted_activated_display = Some(display);
                granted_activated = Some(parsed);
                Some(actions)
            } else {
                None
            };

            if let Some(triggered) = parse_triggered_granted_ability(&ability_tokens)? {
                let display = format!(
                    "{} has {}",
                    clause_words.join(" "),
                    crate::runtime_backend::token_word_refs(&ability_tokens).join(" ")
                );
                extras.push(grant_object_ability_for_anthem_subject(
                    &clause, triggered, display,
                ));
            } else if let Some(actions) = actions {
                reject_unimplemented_keyword_actions(&actions, &clause_words.join(" "))?;
                let granted = actions
                    .into_iter()
                    .filter_map(|action| keyword_action_to_static_ability(action))
                    .collect::<Vec<_>>();
                if granted.is_empty() {
                    return Ok(None);
                }
                for ability in granted {
                    extras.push(grant_for_anthem_subject(&clause, ability).into());
                }

                if let Some(activated) = granted_activated {
                    extras.push(grant_object_ability_for_anthem_subject(
                        &clause,
                        activated,
                        granted_activated_display.unwrap_or_else(|| clause_words.join(" ")),
                    ));
                }
            } else {
                return Ok(None);
            }

            if grant_must_attack {
                extras.push(grant_for_anthem_subject(&clause, StaticAbility::must_attack()).into());
            }
            continuing_have_clause = true;
            continue;
        }

        if continuing_have_clause
            && let Some(mut granted) =
                parse_continuing_anthem_granted_segment(&clause, &clause_words, &segment)?
        {
            extras.append(&mut granted);
            continue;
        }

        if let Some(triggered) = parse_triggered_granted_ability(&segment)? {
            let display = format!(
                "{} has {}",
                clause_words.join(" "),
                crate::runtime_backend::token_word_refs(&segment).join(" ")
            );
            extras.push(grant_object_ability_for_anthem_subject(
                &clause, triggered, display,
            ));
            continue;
        }

        return Ok(None);
    }

    if extras.is_empty() {
        return Ok(None);
    }

    let mut result = vec![build_anthem_static_ability(&clause).into()];
    result.extend(extras);
    Ok(Some(result))
}

pub(crate) fn parse_conditional_all_creatures_able_to_block_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let Some(shape) = anthem_grant_grammar::parse_conditional_must_block_shape(tokens) else {
        return Ok(None);
    };
    let condition = parse_static_condition_clause(shape.condition_tokens)?;
    match shape.target {
        anthem_grant_grammar::ConditionalMustBlockTarget::Source => {
            Ok(Some(StaticAbilityAst::ConditionalStaticAbility {
                ability: Box::new(StaticAbilityAst::Static(StaticAbility::must_block())),
                condition,
            }))
        }
        anthem_grant_grammar::ConditionalMustBlockTarget::EnchantedCreature => {
            Ok(Some(StaticAbilityAst::AttachedStaticAbilityGrant {
                ability: Box::new(StaticAbilityAst::Static(StaticAbility::must_block())),
                display: "enchanted creature has this creature must be blocked if able".to_string(),
                condition: Some(condition),
            }))
        }
    }
}

pub(crate) fn parse_source_can_attack_as_though_no_defender_as_long_as_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let Some(shape) = anthem_grant_grammar::parse_subject_no_defender_as_long_shape(tokens) else {
        return Ok(None);
    };
    let condition = parse_static_condition_clause(shape.condition_tokens)?;

    let subject = parse_anthem_subject(shape.subject_tokens)?;
    let granted = match subject {
        AnthemSubjectAst::Source => StaticAbilityAst::ConditionalStaticAbility {
            ability: Box::new(StaticAbilityAst::Static(
                StaticAbility::can_attack_as_though_no_defender(),
            )),
            condition,
        },
        AnthemSubjectAst::Filter(filter) => StaticAbilityAst::GrantStaticAbility {
            filter,
            ability: Box::new(StaticAbilityAst::Static(
                StaticAbility::can_attack_as_though_no_defender(),
            )),
            condition: Some(condition),
        },
    };
    Ok(Some(granted))
}

pub(crate) fn parse_attached_can_attack_as_though_no_defender_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let Some(shape) = anthem_grant_grammar::parse_attached_no_defender_shape(tokens) else {
        return Ok(None);
    };

    let subject = crate::runtime_backend::token_word_refs(shape.subject_tokens).join(" ");
    Ok(Some(StaticAbilityAst::AttachedStaticAbilityGrant {
        ability: Box::new(StaticAbilityAst::Static(
            StaticAbility::can_attack_as_though_no_defender(),
        )),
        display: format!("{subject} can attack as though it didn't have defender"),
        condition: None,
    }))
}

pub(crate) fn parse_as_long_as_condition_can_attack_as_though_no_defender_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let Some(shape) = anthem_grant_grammar::parse_leading_condition_no_defender_shape(tokens)
    else {
        return Ok(None);
    };

    let condition = parse_static_condition_clause(shape.condition_tokens)?;
    let subject = parse_anthem_subject(shape.subject_tokens)?;
    let granted = match subject {
        AnthemSubjectAst::Source => StaticAbilityAst::ConditionalStaticAbility {
            ability: Box::new(StaticAbilityAst::Static(
                StaticAbility::can_attack_as_though_no_defender(),
            )),
            condition,
        },
        AnthemSubjectAst::Filter(filter) => StaticAbilityAst::GrantStaticAbility {
            filter,
            ability: Box::new(StaticAbilityAst::Static(
                StaticAbility::can_attack_as_though_no_defender(),
            )),
            condition: Some(condition),
        },
    };
    Ok(Some(granted))
}

pub(crate) fn parse_gets_and_attacks_each_combat_if_able_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    let Some(shape) = anthem_grant_grammar::parse_gets_attacks_shape(tokens) else {
        return Ok(None);
    };
    let clause = parse_anthem_clause(tokens, shape.get_token, shape.and_token)?;
    let mut result = vec![build_anthem_static_ability(&clause).into()];
    result.push(grant_for_anthem_subject(
        &clause,
        StaticAbility::must_attack(),
    ));

    if result.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "failed to parse gets-and-attacks clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    Ok(Some(result))
}

pub(crate) fn parse_anthem_and_granted_ability_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if contains_until_end_of_turn(&clause_words) {
        return Ok(None);
    }

    let Some(shape) = anthem_grant_grammar::parse_anthem_and_granted_tail(tokens) else {
        return Ok(None);
    };
    let clause = parse_anthem_clause(tokens, shape.get_token, shape.and_token)?;
    let mut result = vec![build_anthem_static_ability(&clause).into()];
    match shape.tail_kind {
        anthem_grant_grammar::AnthemGrantedTailKind::CantBeBlocked => result.push(
            grant_for_anthem_subject(&clause, StaticAbility::unblockable()),
        ),
        anthem_grant_grammar::AnthemGrantedTailKind::BeEverySubtype => {
            let family_words = crate::runtime_backend::token_word_refs(shape.family_tokens);
            let Some(family) = parse_every_subtype_family_tail(&family_words) else {
                return Ok(None);
            };
            result.push(every_subtype_family_for_subject(
                &clause.subject,
                family,
                clause.condition.clone(),
            ));
        }
    }

    Ok(Some(result))
}

pub(crate) fn parse_subject_is_every_subtype_family_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let all_words = crate::runtime_backend::token_word_refs(tokens);
    if all_words.len() < 4 || contains_until_end_of_turn(&all_words) {
        return Ok(None);
    }
    let Some(shape) = anthem_grant_grammar::parse_subject_every_subtype_shape(tokens) else {
        return Ok(None);
    };
    let family_words = crate::runtime_backend::token_word_refs(shape.family_tokens);
    let Some(family) = parse_every_subtype_family_tail(&family_words) else {
        return Ok(None);
    };
    let condition = shape
        .condition_tokens
        .map(parse_static_condition_clause)
        .transpose()?;
    let subject = parse_anthem_subject(shape.subject_tokens)?;
    Ok(Some(every_subtype_family_for_subject(
        &subject, family, condition,
    )))
}

pub(crate) fn parse_anthem_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(head) = anthem_grant_grammar::parse_anthem_modifier_head(tokens) else {
        return Ok(None);
    };
    if head.has_target || head.temporary {
        return Ok(None);
    }
    let modifier_word = tokens[head.modifier_token].as_word().unwrap_or_default();
    if parse_pt_modifier_values(modifier_word).is_err()
        && parse_dynamic_xy_anthem_values(
            modifier_word,
            &trim_edge_punctuation(tokens.get(head.modifier_token + 1..).unwrap_or_default()),
        )
        .is_none()
    {
        return Ok(None);
    }
    let clause = parse_anthem_clause(tokens, head.get_token, tokens.len())?;
    Ok(Some(build_anthem_static_ability(&clause)))
}

pub(crate) fn parse_multi_subject_anthem_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let Some(head) = anthem_grant_grammar::parse_anthem_modifier_head(tokens) else {
        return Ok(None);
    };
    if head.has_target || head.temporary {
        return Ok(None);
    }
    let get_idx = head.get_token;
    let modifier_word = tokens[head.modifier_token].as_word().unwrap_or_default();
    if parse_pt_modifier_values(modifier_word).is_err()
        && parse_dynamic_xy_anthem_values(
            modifier_word,
            &trim_edge_punctuation(tokens.get(head.modifier_token + 1..).unwrap_or_default()),
        )
        .is_none()
    {
        return Ok(None);
    }

    let Ok((_prefix_condition, subject_start)) = parse_anthem_prefix_condition(tokens, get_idx)
    else {
        return Ok(None);
    };
    let subject_tokens = trim_commas(&tokens[subject_start..get_idx]);
    let Some(segments) = anthem_grant_grammar::parse_multi_subject_segments(&subject_tokens) else {
        return Ok(None);
    };

    let mut abilities = Vec::with_capacity(segments.len());
    for segment in segments {
        let mut clause_tokens = Vec::with_capacity(tokens.len());
        clause_tokens.extend_from_slice(&tokens[..subject_start]);
        clause_tokens.extend_from_slice(segment);
        clause_tokens.extend_from_slice(&tokens[get_idx..]);
        let adjusted_get_idx = subject_start + segment.len();
        let clause =
            match parse_anthem_clause(&clause_tokens, adjusted_get_idx, clause_tokens.len()) {
                Ok(clause) => clause,
                Err(_) => return Ok(None),
            };
        abilities.push(build_anthem_static_ability(&clause));
    }

    Ok(Some(abilities))
}

pub(crate) fn parse_has_base_power_toughness_static_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(shape) = anthem_grant_grammar::parse_base_power_toughness_shape(tokens) else {
        return Ok(None);
    };
    let subject = parse_anthem_subject(shape.subject_tokens)?;
    let filter = match subject {
        AnthemSubjectAst::Source => ObjectFilter::source(),
        AnthemSubjectAst::Filter(filter) => filter,
    };

    Ok(Some(StaticAbility::set_base_power_toughness(
        filter,
        shape.power,
        shape.toughness,
    )))
}

pub(crate) fn parse_isnt_creature_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let display = crate::runtime_backend::token_word_refs(tokens).join(" ");
    let shape = match anthem_grant_grammar::parse_isnt_creature_shape(tokens) {
        Ok(Some(shape)) => shape,
        Ok(None) => return Ok(None),
        Err(anthem_grant_grammar::IsntCreatureShapeError::MissingLeadingCondition) => {
            return Err(CardTextError::ParseError(format!(
                "missing condition after leading 'as long as' clause (clause: '{display}')"
            )));
        }
        Err(anthem_grant_grammar::IsntCreatureShapeError::MissingUnlessCondition) => {
            return Err(CardTextError::ParseError(format!(
                "missing condition after trailing 'unless' clause (clause: '{display}')"
            )));
        }
    };
    let mut condition = shape
        .leading_condition_tokens
        .map(parse_static_condition_clause)
        .transpose()?;
    if let Some(unless_tokens) = shape.unless_condition_tokens {
        let unless_condition =
            crate::ConditionExpr::Not(Box::new(parse_static_condition_clause(unless_tokens)?));
        condition = Some(match condition {
            Some(existing) => {
                crate::ConditionExpr::And(Box::new(existing), Box::new(unless_condition))
            }
            None => unless_condition,
        });
    }

    let subject = parse_anthem_subject(shape.subject_tokens)?;
    let filter = match subject {
        AnthemSubjectAst::Source => ObjectFilter::source(),
        AnthemSubjectAst::Filter(filter) => filter,
    };

    let mut remove =
        crate::static_abilities::RemoveCardTypesForFilter::new(filter, vec![CardType::Creature]);
    if let Some(condition) = condition {
        remove = remove.with_condition(condition);
    }
    Ok(Some(StaticAbility::new(remove)))
}

pub(crate) fn parse_has_base_power_toughness_and_granted_keywords_static_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    let Some(shape) = anthem_grant_grammar::parse_base_power_toughness_grant_shape(tokens) else {
        return Ok(None);
    };
    let (condition, subject_start) = match parse_anthem_prefix_condition(tokens, shape.has_token) {
        Ok(parsed) => parsed,
        Err(_) => return Ok(None),
    };
    let subject_tokens = trim_commas(&tokens[subject_start..shape.has_token]);
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    let subject_facts = anthem_grant_grammar::persistent_anthem_subject_facts(&subject_tokens);
    if !subject_facts.accepted {
        return Ok(None);
    }

    let Some(actions) = parse_ability_line(shape.ability_tokens) else {
        return Ok(None);
    };
    reject_unimplemented_keyword_actions(&actions, &clause_words.join(" "))?;
    let granted = actions;
    if granted.is_empty() {
        return Ok(None);
    }

    let subject = match parse_anthem_subject(&subject_tokens) {
        Ok(subject) => subject,
        Err(_) => return Ok(None),
    };

    let mut compiled = Vec::new();
    match subject {
        AnthemSubjectAst::Source => {
            let source_filter = if subject_facts.is_this_creature {
                ObjectFilter::source().with_type(CardType::Creature)
            } else {
                ObjectFilter::source()
            };
            let set_base = StaticAbility::set_base_power_toughness(
                source_filter,
                shape.power,
                shape.toughness,
            )
            .into();
            compiled.push(if let Some(condition) = condition.clone() {
                StaticAbilityAst::ConditionalStaticAbility {
                    ability: Box::new(set_base),
                    condition,
                }
            } else {
                set_base
            });
            compiled.extend(granted.into_iter().map(|action| {
                if let Some(condition) = condition.clone() {
                    StaticAbilityAst::ConditionalKeywordAction { action, condition }
                } else {
                    StaticAbilityAst::KeywordAction(action)
                }
            }));
        }
        AnthemSubjectAst::Filter(filter) => {
            let set_base = StaticAbility::set_base_power_toughness(
                filter.clone(),
                shape.power,
                shape.toughness,
            )
            .into();
            compiled.push(if let Some(condition) = condition.clone() {
                StaticAbilityAst::ConditionalStaticAbility {
                    ability: Box::new(set_base),
                    condition,
                }
            } else {
                set_base
            });
            for action in granted {
                compiled.push(StaticAbilityAst::GrantKeywordAction {
                    filter: filter.clone(),
                    action,
                    condition: condition.clone(),
                });
            }
        }
    }

    Ok(Some(compiled))
}

pub(crate) fn parse_filter_has_granted_ability_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    let mut deferred_error: Option<CardTextError> = None;
    for candidate in anthem_grant_grammar::parse_granted_ability_candidates(tokens) {
        let has_idx = candidate.has_token;
        let (mut condition, subject_start) = match parse_anthem_prefix_condition(tokens, has_idx) {
            Ok(parsed) => parsed,
            Err(err) => {
                deferred_error.get_or_insert(err);
                continue;
            }
        };
        let subject_tokens = trim_commas(&tokens[subject_start..has_idx]);
        if subject_tokens.is_empty() {
            continue;
        }
        if let Some(split) = anthem_grant_grammar::split_type_addition_subject(&subject_tokens) {
            if let Some(additions) = parse_type_color_addition_clause(split.addition_tokens)? {
                let base_subject = match parse_anthem_subject(split.base_subject_tokens) {
                    Ok(subject) => subject,
                    Err(err) => {
                        deferred_error.get_or_insert(err);
                        continue;
                    }
                };
                let AnthemSubjectAst::Filter(filter) = &base_subject else {
                    continue;
                };
                let ability_tokens = trim_commas(&tokens[has_idx + 1..]);
                let attached_subject =
                    anthem_grant_grammar::parse_granted_subject_facts(split.base_subject_tokens)
                        .attached_subject;
                let granted_tail = match parse_heterogeneous_granted_tail(
                    &ability_tokens,
                    &clause_words,
                    attached_subject,
                ) {
                    Ok(Some(tail)) => tail,
                    Ok(None) => continue,
                    Err(err) => {
                        deferred_error.get_or_insert(err);
                        continue;
                    }
                };
                let mut result = Vec::new();
                if !additions.set_colors.is_empty() {
                    result.push(
                        StaticAbility::set_colors(filter.clone(), additions.set_colors).into(),
                    );
                }
                if !additions.added_colors.is_empty() {
                    result.push(
                        StaticAbility::add_colors(filter.clone(), additions.added_colors).into(),
                    );
                }
                if !additions.card_types.is_empty() {
                    result.push(
                        StaticAbility::add_card_types(filter.clone(), additions.card_types).into(),
                    );
                }
                if !additions.subtypes.is_empty() {
                    result.push(
                        StaticAbility::add_subtypes(filter.clone(), additions.subtypes).into(),
                    );
                }
                result.extend(lower_granted_tail_for_anthem_subject(
                    &base_subject,
                    &condition,
                    granted_tail,
                ));
                if !result.is_empty() {
                    return Ok(Some(result));
                }
            }
        }
        let subject_facts = anthem_grant_grammar::parse_granted_subject_facts(&subject_tokens);
        if subject_facts.rejected_action || subject_facts.has_may {
            continue;
        }

        let mut ability_tokens = trim_commas(&tokens[has_idx + 1..]);
        let mut condition_failed = false;
        for kind in [
            anthem_grant_grammar::GrantedAbilityConditionKind::AsLongAs,
            anthem_grant_grammar::GrantedAbilityConditionKind::If,
        ] {
            let Some(split) =
                anthem_grant_grammar::split_granted_ability_condition(&ability_tokens, kind)
            else {
                continue;
            };
            let parsed_condition = match parse_static_condition_clause(split.condition_tokens) {
                Ok(condition) => condition,
                Err(err) => {
                    deferred_error.get_or_insert(err);
                    condition_failed = true;
                    break;
                }
            };
            condition = Some(match condition {
                Some(existing) => {
                    crate::ConditionExpr::And(Box::new(existing), Box::new(parsed_condition))
                }
                None => parsed_condition,
            });
            ability_tokens = split.ability_tokens.to_vec();
        }
        if condition_failed {
            continue;
        }

        if let Some(keyword) = anthem_grant_grammar::parse_special_granted_keyword(&ability_tokens)
        {
            let parsed = match keyword {
                anthem_grant_grammar::SpecialGrantedKeyword::Blitz => {
                    granted_blitz_abilities_from_subject(&subject_tokens, condition.clone())
                }
                anthem_grant_grammar::SpecialGrantedKeyword::Emerge => {
                    granted_emerge_abilities_from_subject(&subject_tokens, condition.clone())
                }
            };
            match parsed {
                Ok(Some(grants)) => return Ok(Some(grants)),
                Ok(None) => continue,
                Err(err) => {
                    deferred_error.get_or_insert(err);
                    continue;
                }
            }
        }
        let granted_tail = match parse_heterogeneous_granted_tail(
            &ability_tokens,
            &clause_words,
            subject_facts.attached_subject,
        ) {
            Ok(Some(tail)) => tail,
            Ok(None) => continue,
            Err(err) => {
                deferred_error.get_or_insert(err);
                continue;
            }
        };
        let attached_subject_filter =
            infer_attached_subject_filter_from_condition_expr(condition.as_ref());
        let subject = match parse_anthem_subject_with_attached_fallback(
            &subject_tokens,
            attached_subject_filter.as_ref(),
        ) {
            Ok(subject) => subject,
            Err(err) => {
                deferred_error.get_or_insert(err);
                continue;
            }
        };
        let granted = lower_granted_tail_for_anthem_subject(&subject, &condition, granted_tail);
        if granted.is_empty() {
            continue;
        }
        return Ok(Some(granted));
    }

    if let Some(err) = deferred_error {
        return Err(err);
    }
    Ok(None)
}

#[test]
fn attached_object_anthem_subject_uses_tagged_constraints() {
    let enchanted = AnthemSubjectAst::Filter(ObjectFilter::tagged("enchanted"));
    assert!(attached_object_anthem_subject_filter(&enchanted).is_some());

    let equipped = AnthemSubjectAst::Filter(ObjectFilter::tagged("equipped"));
    assert!(attached_object_anthem_subject_filter(&equipped).is_some());

    let creature = AnthemSubjectAst::Filter(ObjectFilter::creature());
    assert!(attached_object_anthem_subject_filter(&creature).is_none());
}

#[test]
fn quoted_static_marker_grants_parse_for_filtered_subjects() {
    let tokens = crate::runtime_backend::lexer::lex_line(
        "Commander creatures you own have \"Room abilities of dungeons you own trigger an additional time.\"",
        0,
    )
    .expect("lex quoted static grant");
    let candidates = anthem_grant_grammar::parse_granted_ability_candidates(&tokens);
    assert_eq!(candidates.len(), 1, "expected one have-tail candidate");
    let has_token = candidates[0].has_token;
    let tail = trim_commas(&tokens[has_token + 1..]);
    let parsed_tail = parse_heterogeneous_granted_tail(
        &tail,
        &crate::runtime_backend::token_word_refs(&tokens),
        false,
    )
    .expect("parse heterogeneous quoted tail");
    assert!(parsed_tail.is_some(), "expected quoted marker tail");
    let abilities = parse_filter_has_granted_ability_line(&tokens)
        .expect("parse quoted static grant")
        .expect("quoted marker grant should be recognized");
    assert!(abilities.iter().any(|ability| matches!(
        ability,
        StaticAbilityAst::GrantStaticAbility { ability, .. }
            if matches!(ability.as_ref(), StaticAbilityAst::Static(static_ability)
                if static_ability.id() == crate::static_abilities::StaticAbilityId::DungeonRoomTriggerDuplication)
    )));
}

#[test]
fn static_turn_threshold_conditions_use_typed_anthem_grammar() {
    let drawn = crate::runtime_backend::lexer::lex_line(
        "An opponent has drawn three or more cards this turn",
        0,
    )
    .expect("draw threshold should lex");
    assert_eq!(
        parse_cards_drawn_this_turn_static_condition(&drawn),
        Some(crate::ConditionExpr::ValueComparison {
            left: crate::effect::Value::MaxCardsDrawnThisTurn(PlayerFilter::Opponent),
            operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
            right: crate::effect::Value::Fixed(3),
        })
    );

    let rolled =
        crate::runtime_backend::lexer::lex_line("You have rolled two or more dice this turn", 0)
            .expect("dice threshold should lex");
    assert_eq!(
        parse_dice_rolled_this_turn_static_condition(&rolled),
        Some(crate::ConditionExpr::ValueComparison {
            left: crate::effect::Value::MaxDiceRolledThisTurn(PlayerFilter::You),
            operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
            right: crate::effect::Value::Fixed(2),
        })
    );
}

#[test]
fn static_condition_family_consumes_typed_condition_shapes() {
    let devotion = crate::runtime_backend::lexer::lex_line(
        "Your devotion to white and blue is greater than or equal to three.",
        0,
    )
    .expect("devotion condition should lex");
    assert!(matches!(
        parse_static_condition_clause(&devotion).expect("devotion condition should parse"),
        crate::ConditionExpr::ValueComparison {
            operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
            right: crate::effect::Value::Fixed(3),
            ..
        }
    ));

    let conjoined =
        crate::runtime_backend::lexer::lex_line("It is your turn and you attacked this turn.", 0)
            .expect("conjoined condition should lex");
    let crate::ConditionExpr::And(left, right) =
        parse_static_condition_clause(&conjoined).expect("conjoined condition should parse")
    else {
        panic!("expected a typed conjunction");
    };
    assert_eq!(*left, crate::ConditionExpr::YourTurn);
    assert_eq!(*right, crate::ConditionExpr::AttackedThisTurn);

    let graveyard = crate::runtime_backend::lexer::lex_line(
        "There are four or more card types among cards in your graveyard.",
        0,
    )
    .expect("graveyard condition should lex");
    assert_eq!(
        parse_static_condition_clause(&graveyard).expect("graveyard condition should parse"),
        crate::ConditionExpr::PlayerHasCardTypesInGraveyardOrMore {
            player: PlayerFilter::You,
            count: 4,
        }
    );
}

#[test]
fn keyword_and_unblockable_tail_keeps_multiple_captured_keywords() {
    let tokens = crate::runtime_backend::lexer::lex_line(
        "This creature has flying and vigilance and can't be blocked.",
        0,
    )
    .expect("line should lex");
    let parsed = parse_subject_has_keywords_and_cant_be_blocked_line(&tokens)
        .expect("parser should not error")
        .expect("line should parse");

    assert!(matches!(
        parsed.as_slice(),
        [
            StaticAbilityAst::KeywordAction(KeywordAction::Flying),
            StaticAbilityAst::KeywordAction(KeywordAction::Vigilance),
            StaticAbilityAst::KeywordAction(KeywordAction::Unblockable),
        ]
    ));
}

#[test]
fn granted_escape_tail_captures_dynamic_exile_count() {
    let tokens = crate::runtime_backend::lexer::lex_line(
        "The escape cost is equal to the card's mana cost plus exile three other cards from your graveyard.",
        0,
    )
    .expect("line should lex");
    let parsed = parse_granted_escape_cost_tail_clause(&tokens).expect("escape tail should parse");
    let (count, used) =
        parse_number(parsed.exile_count_tokens).expect("captured count should parse");

    assert_eq!(count, 3);
    assert_eq!(used, parsed.exile_count_tokens.len());
}

#[test]
fn granted_miracle_tail_captures_dynamic_cost_reduction() {
    let tokens = crate::runtime_backend::lexer::lex_line(
        "Its miracle cost is equal to its mana cost reduced by {4}.",
        0,
    )
    .expect("line should lex");
    let parsed = parse_granted_miracle_cost_reduction_tail_clause(&tokens)
        .expect("miracle tail should parse");
    let (cost, used) =
        crate::runtime_backend::front_end::shared::util::leading_mana_cost_from_tokens(
            parsed.reduction_cost_tokens,
        )
        .expect("captured cost should parse");

    assert_eq!(cost.generic_mana_total(), 4);
    assert_eq!(used, parsed.reduction_cost_tokens.len());
}

#[test]
fn cant_be_blocked_by_more_than_clause_captures_subject_and_threshold() {
    let tokens = crate::runtime_backend::lexer::lex_line(
        "Each creature you control with a +1/+1 counter on it can't be blocked by more than one creature.",
        0,
    )
    .expect("line should lex");
    let parsed = parse_cant_be_blocked_by_more_than_clause(&tokens)
        .expect("max-blockers clause should parse");
    let subject_words =
        crate::runtime_backend::lexer::parser_token_word_refs(parsed.subject_tokens);
    let (minimum_blockers, used) = parse_greater_than_or_equal_quantity_prefix(
        parsed.blocker_threshold_tokens,
        false,
        false,
        "test blocker threshold",
    )
    .expect("threshold should parse")
    .expect("threshold should be present");

    assert_eq!(
        subject_words.as_slice(),
        &[
            "each", "creature", "you", "control", "with", "a", "+1/+1", "counter", "on", "it"
        ]
    );
    assert_eq!(minimum_blockers, 2);
    assert_eq!(used, parsed.blocker_threshold_tokens.len());
}

#[test]
fn can_block_additional_creature_clause_captures_subject_and_count() {
    let tokens = crate::runtime_backend::lexer::lex_line(
        "Each creature you control can block two additional creatures each combat.",
        0,
    )
    .expect("line should lex");
    let parsed = parse_can_block_additional_creature_clause(&tokens)
        .expect("additional-blocker clause should parse");
    let subject_words =
        crate::runtime_backend::lexer::parser_token_word_refs(parsed.subject_tokens);
    let (count, used) = parse_number(parsed.additional_count_tokens)
        .expect("captured additional blocker count should parse");

    assert_eq!(
        subject_words.as_slice(),
        &["each", "creature", "you", "control"]
    );
    assert_eq!(count, 2);
    assert_eq!(used, parsed.additional_count_tokens.len());
}

#[test]
fn landwalk_override_tail_uses_keyword_action_parser() {
    assert!(is_landwalk_ability_word("islandwalk"));
    assert!(is_landwalk_ability_word("forestwalk"));
    assert!(!is_landwalk_ability_word("planeswalk"));
    assert!(!is_landwalk_ability_word("walk"));
}
