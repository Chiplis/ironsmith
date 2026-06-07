use super::*;
use crate::runtime_backend::SubjectAst;
use crate::runtime_backend::ast::SubjectVerbActionAst;
use crate::runtime_backend::lexer::{
    render_token_slice, word_slice_contains_all_words, word_slice_contains_any_word,
    word_slice_contains_phrase, word_slice_ends_with, word_slice_ends_with_any, word_slice_eq,
    word_slice_eq_any, word_slice_starts_with, word_slice_starts_with_any,
};

pub(crate) type ActivationRestrictionCompatWords<'a> = grammar::TokenWordView<'a>;

const PRIMARY_ADD_MANA_CLAUSE_PREFIXES: &[&[&str]] = &[
    &["add"],
    &["adds"],
    &["you", "add"],
    &["that", "player", "add"],
    &["that", "player", "adds"],
    &["target", "player", "add"],
    &["target", "player", "adds"],
];
const THAT_COLOR_PREFIX: &[&str] = &["that", "color"];
const THIS_ENTERS_TAPPED_PREFIX: &[&str] = &["this"];
const ATTACKING_TRAILS: &[&[&str]] = &[&["and", "attacking"], &["attacking"]];
const THIS_COST_REDUCED_BY_PREFIX: &[&str] = &["this", "cost", "is", "reduced", "by"];
const ACTIVATED_ABILITIES_OF_PREFIX: &[&str] = &["activated", "abilities", "of"];
const LESS_TO_ACTIVATE_PREFIX: &[&str] = &["less", "to", "activate"];
const LESS_TO_ACTIVATE_WORDS: &[&str] = &["less", "to", "activate"];
const LESS_TO_ACTIVATE_IF_PREFIX: &[&str] = &["less", "to", "activate", "if"];
const IT_TARGETS_PREFIX: &[&str] = &["it", "targets"];
const LESS_TO_ACTIVATE_FOR_EACH_PREFIX: &[&str] = &["less", "to", "activate", "for", "each"];
const THIS_ABILITY_COSTS_PREFIX: &[&str] = &["this", "ability", "costs"];
const THIS_SPELL_COSTS_PREFIX: &[&str] = &["this", "spell", "costs"];
const X_COST_WORDS: &[&str] = &["x", "+x", "-x"];
const WHERE_X_IS_WORDS: &[&str] = &["where", "x", "is"];
const THAT_PLAYER_DEVOTION_OWNER_SUFFIXES: &[&[&str]] = &[
    &["that", "players"],
    &["that", "player"],
    &["that", "player's"],
    &["that", "players'"],
];
const ALL_CREATURES_ABLE_TO_BLOCK_SOURCE_WORDS: &[&[&str]] = &[
    &[
        "all",
        "creatures",
        "able",
        "to",
        "block",
        "this",
        "creature",
        "do",
        "so",
    ],
    &[
        "all",
        "creatures",
        "able",
        "to",
        "block",
        "this",
        "do",
        "so",
    ],
];
const SOURCE_MUST_BE_BLOCKED_IF_ABLE_WORDS: &[&[&str]] = &[
    &["this", "creature", "must", "be", "blocked", "if", "able"],
    &["this", "must", "be", "blocked", "if", "able"],
];

fn find_token_word_exact(tokens: &[OwnedLexToken], expected: &str) -> Option<usize> {
    find_index(tokens, |token| {
        token
            .as_word()
            .is_some_and(|_| token.parser_text() == expected)
    })
}

fn activated_words_start_with(words: &[&str], prefix: &[&str]) -> bool {
    word_slice_starts_with(words, prefix)
}

fn activated_words_start_with_any(words: &[&str], prefixes: &[&[&str]]) -> bool {
    word_slice_starts_with_any(words, prefixes)
}

fn activated_words_equal(words: &[&str], expected: &[&str]) -> bool {
    word_slice_eq(words, expected)
}

fn activated_words_equal_any(words: &[&str], expected: &[&[&str]]) -> bool {
    word_slice_eq_any(words, expected)
}

fn activated_words_contain_all(words: &[&str], required: &[&str]) -> bool {
    word_slice_contains_all_words(words, required)
}

fn activated_words_contain_any(words: &[&str], candidates: &[&str]) -> bool {
    word_slice_contains_any_word(words, candidates)
}

fn activated_words_contain_phrase(words: &[&str], phrase: &[&str]) -> bool {
    word_slice_contains_phrase(words, phrase)
}

fn activated_words_end_with(words: &[&str], suffix: &[&str]) -> bool {
    word_slice_ends_with(words, suffix)
}

fn activated_words_end_with_any(words: &[&str], suffixes: &[&[&str]]) -> bool {
    word_slice_ends_with_any(words, suffixes)
}

fn activated_word_is_any(word: &str, candidates: &[&str]) -> bool {
    candidates.iter().any(|candidate| word == *candidate)
}

pub(crate) fn joined_activation_clause_text(tokens: &[OwnedLexToken]) -> String {
    crate::runtime_backend::token_word_refs(tokens).join(" ")
}

pub(crate) fn parse_prefixed_activated_ability_label(
    tokens: &[OwnedLexToken],
    cost_start: usize,
) -> Option<String> {
    if cost_start == 0 {
        return None;
    }

    let prefix = ActivationRestrictionCompatWords::new(&tokens[..cost_start]);
    match prefix.get(prefix.len().saturating_sub(1)) {
        Some("boast") => Some("Boast".to_string()),
        Some("exhaust") => Some("Exhaust".to_string()),
        Some("renew") => Some("Renew".to_string()),
        _ => None,
    }
}

pub(crate) fn contains_granted_keyword_before_word(
    words: &ActivationRestrictionCompatWords,
    keyword_idx: usize,
) -> bool {
    (0..keyword_idx)
        .filter_map(|idx| words.get(idx))
        .any(|word| activated_word_is_any(word, &["has", "have"]))
}

pub(crate) fn find_cycling_keyword_word_index(
    words: &ActivationRestrictionCompatWords,
) -> Option<usize> {
    let mut idx = 0usize;
    while idx < words.len() {
        if words.get(idx).is_some_and(word_is_cycling_keyword_marker) {
            return Some(idx);
        }
        idx += 1;
    }
    None
}

pub(crate) fn parse_hand_keyword_activated_body_lexed(
    body_tokens: &[OwnedLexToken],
    keyword: &str,
    display_label: &str,
    clause_text: &str,
) -> Result<Option<ParsedAbility>, CardTextError> {
    if body_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "{keyword} line missing activated ability body (clause: '{clause_text}')",
        )));
    }

    let ability_tokens = trim_commas(body_tokens);
    let Some(mut parsed) = parse_activated_line_with_raw(&ability_tokens)? else {
        return Ok(None);
    };
    *parsed.text_mut() = Some(display_label.to_string());
    *parsed.functional_zones_mut() = vec![Zone::Hand];
    Ok(Some(parsed))
}

pub(crate) fn parse_activated_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedAbility>, CardTextError> {
    parse_activated_line_with_raw(tokens)
}

fn fixed_mana_symbols_from_mana_groups(tokens: &[OwnedLexToken]) -> Option<Vec<ManaSymbol>> {
    let mut mana = Vec::new();
    for token in tokens {
        match token.kind {
            TokenKind::ManaGroup => {
                let inner = token.mana_group_inner()?;
                mana.push(parse_mana_symbol(inner).ok()?);
            }
            TokenKind::Period | TokenKind::Comma => {}
            _ => return None,
        }
    }

    if mana.is_empty() { None } else { Some(mana) }
}

fn subject_allows_direct_mana_output(subject: &Option<SubjectAst>) -> bool {
    matches!(
        subject,
        None | Some(SubjectAst::Player(PlayerAst::You | PlayerAst::Implicit))
    )
}

pub(crate) fn parse_activated_line_with_raw(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedAbility>, CardTextError> {
    let Some(colon_idx) = find_index(tokens, |token| token.is_colon()) else {
        return Ok(None);
    };

    let cost_start = find_activation_cost_start(&tokens[..colon_idx]).unwrap_or(0);
    let cost_tokens = &tokens[cost_start..colon_idx];
    let effect_tokens = &tokens[colon_idx + 1..];
    if cost_tokens.is_empty() || effect_tokens.is_empty() {
        return Ok(None);
    }
    let ability_label = parse_prefixed_activated_ability_label(tokens, cost_start);
    let ability_display_text = prefixed_activated_ability_display_text(
        ability_label.as_deref(),
        cost_tokens,
        effect_tokens,
    );
    let loyalty_shorthand_cost = parse_loyalty_shorthand_activation_cost(cost_tokens);
    let mut effect_sentences = grammar::split_lexed_slices_on_period(effect_tokens);
    let functional_zones = infer_activated_functional_zones_lexed(cost_tokens, &effect_sentences);
    let mut timing = ActivationTiming::AnyTime;
    let scanned_modifiers = collect_activated_sentence_modifiers(&effect_sentences, timing.clone());
    let mana_activation_condition = scanned_modifiers.mana_activation_condition;
    let mut additional_activation_restrictions =
        scanned_modifiers.additional_activation_restrictions;
    if ability_label.as_deref() == Some("Exhaust")
        && !scanned_modifiers.has_exhaust_once_restriction
    {
        additional_activation_restrictions
            .push("Activate each exhaust ability only once.".to_string());
    }
    let mana_usage_restrictions = scanned_modifiers.mana_usage_restrictions;
    let inline_effects_ast = scanned_modifiers.inline_effects_ast;
    effect_sentences = scanned_modifiers.kept_sentences;
    timing = scanned_modifiers.timing;
    let mana_activation_condition =
        combine_mana_activation_condition(mana_activation_condition, timing.clone());
    if !effect_sentences.is_empty() {
        let primary_sentence = &effect_sentences[0];
        let x_defined_by_cost = activation_cost_mentions_x(cost_tokens);
        let effect_words = ActivationRestrictionCompatWords::new(primary_sentence);
        let primary_sentence_words = effect_words.to_word_refs();
        let is_primary_add_clause = activated_words_start_with_any(
            &primary_sentence_words,
            PRIMARY_ADD_MANA_CLAUSE_PREFIXES,
        );
        let is_primary_color_among_add_clause =
            is_for_each_color_among_add_mana_clause(primary_sentence);
        if is_primary_add_clause || is_primary_color_among_add_clause {
            let mana_cost = if let Some(cost) = &loyalty_shorthand_cost {
                cost.clone()
            } else {
                parse_activation_cost(cost_tokens)?
            };
            let reference_imports = first_sacrifice_cost_choice_tag(&mana_cost)
                .or_else(|| last_exile_cost_choice_tag(&mana_cost))
                .map(ReferenceImports::with_last_object_tag)
                .unwrap_or_default();

            let mut extra_effects_ast = inline_effects_ast.clone();
            if effect_sentences.len() > 1 {
                for sentence in &effect_sentences[1..] {
                    if sentence.is_empty() {
                        continue;
                    }
                    let ast = parse_effect_sentence_lexed(sentence)?;
                    extra_effects_ast.extend(ast);
                }
            }

            let add_token_idx = find_index(primary_sentence, |token| {
                token
                    .as_word()
                    .is_some_and(|word| matches!(word, "add" | "adds"))
            })
            .unwrap_or(0);
            let mana_tokens = if is_primary_color_among_add_clause {
                primary_sentence
            } else {
                &primary_sentence[add_token_idx + 1..]
            };
            let mana_subject = (add_token_idx > 0 && !is_primary_color_among_add_clause)
                .then(|| parse_subject(&primary_sentence[..add_token_idx]));
            let mana_words_view = ActivationRestrictionCompatWords::new(mana_tokens);
            let mana_words = mana_words_view.to_word_refs();
            let has_for_each_tail = mana_words_view.has_phrase(&["for", "each"]);
            let dynamic_amount = if has_for_each_tail {
                Some(
                    parse_dynamic_cost_modifier_value(mana_tokens)?.ok_or_else(|| {
                        CardTextError::ParseError(format!(
                            "unsupported dynamic mana amount (clause: '{}')",
                            joined_activation_clause_text(primary_sentence)
                        ))
                    })?,
                )
            } else {
                parse_devotion_value_from_add_clause(mana_tokens)?
                    .or_else(|| parse_add_mana_equal_amount_value(mana_tokens))
            };

            let has_imprinted_colors = activated_words_contain_all(&mana_words, &["exiled"])
                && activated_words_contain_any(&mana_words, &["card", "cards"])
                && activated_words_contain_any(&mana_words, &["color", "colors"]);
            let has_any_combination_mana =
                activated_words_contain_phrase(&mana_words, &["any", "combination", "of"]);
            let has_any_choice_mana = has_any_combination_mana
                || (activated_words_contain_all(&mana_words, &["any"])
                    && activated_words_contain_any(&mana_words, &["color", "type"]));
            let has_or_choice_mana = activated_words_contain_all(&mana_words, &["or"]);
            let has_chosen_color = activated_words_contain_all(&mana_words, &["chosen", "color"]);
            let uses_commander_identity = activated_words_contain_all(&mana_words, &["identity"])
                && activated_words_contain_any(&mana_words, &["commander", "commanders"]);
            let loyalty_timing = if loyalty_shorthand_cost.is_some() {
                ActivationTiming::SorcerySpeed
            } else {
                timing.clone()
            };
            let loyalty_restrictions =
                loyalty_additional_restrictions(loyalty_shorthand_cost.is_some());
            let is_loyalty_ability = loyalty_shorthand_cost.is_some();
            let build_additional_restrictions = || {
                let mut restrictions = loyalty_restrictions.clone();
                restrictions.extend(additional_activation_restrictions.clone());
                restrictions
            };
            if is_primary_color_among_add_clause
                || has_imprinted_colors
                || has_any_choice_mana
                || has_or_choice_mana
                || uses_commander_identity
                || has_chosen_color
            {
                let mut mana_ast = parse_add_mana(mana_tokens, mana_subject.clone())?;
                resolve_activated_mana_x_requirements(
                    &mut mana_ast,
                    primary_sentence,
                    x_defined_by_cost,
                )?;
                let ability = Ability {
                    kind: AbilityKind::Activated(ActivatedAbility {
                        mana_cost,
                        effects: crate::resolution::ResolutionProgram::default(),
                        choices: vec![],
                        timing: loyalty_timing.clone(),
                        is_loyalty_ability,
                        additional_restrictions: build_additional_restrictions(),
                        activation_restrictions: vec![],
                        mana_output: Some(vec![]),
                        activation_condition: mana_activation_condition.clone(),
                        mana_usage_restrictions: mana_usage_restrictions.clone(),
                    }),
                    functional_zones: functional_zones.clone(),
                };
                let mut effects_ast = vec![mana_ast];
                effects_ast.extend(extra_effects_ast);
                return Ok(Some(ParsedAbility {
                    ability: ability.into(),
                    text: ability_display_text.clone(),
                    effects_ast: Some(effects_ast),
                    reference_imports: reference_imports.clone(),
                    trigger_spec: None,
                }));
            }

            if let Some(mana) = fixed_mana_symbols_from_mana_groups(mana_tokens) {
                if dynamic_amount.is_none()
                    && extra_effects_ast.is_empty()
                    && subject_allows_direct_mana_output(&mana_subject)
                {
                    let ability = Ability {
                        kind: AbilityKind::Activated(ActivatedAbility {
                            mana_cost,
                            effects: crate::resolution::ResolutionProgram::default(),
                            choices: vec![],
                            timing: loyalty_timing.clone(),
                            is_loyalty_ability,
                            additional_restrictions: build_additional_restrictions(),
                            activation_restrictions: vec![],
                            mana_output: Some(mana),
                            activation_condition: mana_activation_condition.clone(),
                            mana_usage_restrictions: mana_usage_restrictions.clone(),
                        }),
                        functional_zones: functional_zones.clone(),
                    };
                    return Ok(Some(ParsedAbility {
                        ability: ability.into(),
                        text: ability_display_text.clone(),
                        effects_ast: None,
                        reference_imports: ReferenceImports::default(),
                        trigger_spec: None,
                    }));
                }
                let mut mana_ast = parse_add_mana(mana_tokens, mana_subject)?;
                resolve_activated_mana_x_requirements(
                    &mut mana_ast,
                    primary_sentence,
                    x_defined_by_cost,
                )?;
                let ability = Ability {
                    kind: AbilityKind::Activated(ActivatedAbility {
                        mana_cost,
                        effects: crate::resolution::ResolutionProgram::default(),
                        choices: vec![],
                        timing: loyalty_timing,
                        is_loyalty_ability,
                        additional_restrictions: build_additional_restrictions(),
                        activation_restrictions: vec![],
                        mana_output: Some(vec![]),
                        activation_condition: mana_activation_condition.clone(),
                        mana_usage_restrictions: mana_usage_restrictions.clone(),
                    }),
                    functional_zones: functional_zones.clone(),
                };
                let mut effects_ast = vec![mana_ast];
                effects_ast.extend(extra_effects_ast);
                return Ok(Some(ParsedAbility {
                    ability: ability.into(),
                    text: ability_display_text.clone(),
                    effects_ast: Some(effects_ast),
                    reference_imports: reference_imports,
                    trigger_spec: None,
                }));
            }
        }
    }

    // Generic activated ability: parse costs and effects from "<costs>: <effects>"
    let mana_cost = if let Some(cost) = &loyalty_shorthand_cost {
        cost.clone()
    } else {
        parse_activation_cost(cost_tokens)?
    };
    let effect_tokens_joined = join_sentences_with_period(
        &effect_sentences
            .iter()
            .map(|sentence| sentence.to_vec())
            .collect::<Vec<_>>(),
    );
    if effect_sentences.is_empty()
        && !additional_activation_restrictions.is_empty()
        && inline_effects_ast.is_empty()
    {
        return Ok(Some(ParsedAbility {
            ability: {
                let ability = Ability {
                    kind: AbilityKind::Activated(crate::ability::ActivatedAbility {
                        mana_cost,
                        effects: crate::resolution::ResolutionProgram::default(),
                        choices: vec![],
                        timing,
                        is_loyalty_ability: loyalty_shorthand_cost.is_some(),
                        additional_restrictions: additional_activation_restrictions,
                        activation_restrictions: vec![],
                        mana_output: None,
                        activation_condition: mana_activation_condition.clone(),
                        mana_usage_restrictions,
                    }),
                    functional_zones,
                };
                ability
            }
            .into(),
            text: ability_display_text.clone(),
            effects_ast: None,
            reference_imports: ReferenceImports::default(),
            trigger_spec: None,
        }));
    }
    let mut effects_ast = parse_effect_sentences_lexed(&effect_tokens_joined)?;
    effects_ast.extend(inline_effects_ast);
    if effects_ast.is_empty() {
        return Ok(None);
    }
    let reference_imports = first_sacrifice_cost_choice_tag(&mana_cost)
        .or_else(|| last_exile_cost_choice_tag(&mana_cost))
        .map(ReferenceImports::with_last_object_tag)
        .unwrap_or_default();
    if loyalty_shorthand_cost.is_some() {
        timing = ActivationTiming::SorcerySpeed;
        for restriction in loyalty_additional_restrictions(true) {
            let already_present = additional_activation_restrictions.iter().any(|existing| {
                let existing_lower = existing.to_ascii_lowercase();
                let restriction_lower = restriction.to_ascii_lowercase();
                existing.eq_ignore_ascii_case(restriction.as_str())
                    || (existing_lower.matches("once each turn").next().is_some()
                        && restriction_lower.matches("once each turn").next().is_some())
            });
            if !already_present {
                additional_activation_restrictions.push(restriction);
            }
        }
    }

    Ok(Some(ParsedAbility {
        ability: {
            let ability = Ability {
                kind: AbilityKind::Activated(crate::ability::ActivatedAbility {
                    mana_cost,
                    effects: crate::resolution::ResolutionProgram::default(),
                    choices: vec![],
                    timing,
                    is_loyalty_ability: loyalty_shorthand_cost.is_some(),
                    additional_restrictions: additional_activation_restrictions,
                    activation_restrictions: vec![],
                    mana_output: None,
                    activation_condition: mana_activation_condition.clone(),
                    mana_usage_restrictions,
                }),
                functional_zones,
            };
            ability
        }
        .into(),
        text: ability_display_text,
        effects_ast: Some(effects_ast),
        reference_imports,
        trigger_spec: None,
    }))
}

fn prefixed_activated_ability_display_text(
    ability_label: Option<&str>,
    cost_tokens: &[OwnedLexToken],
    effect_tokens: &[OwnedLexToken],
) -> Option<String> {
    ability_label.map(|label| {
        format!(
            "{label} — {}: {}",
            render_token_slice(cost_tokens).trim(),
            render_token_slice(effect_tokens).trim()
        )
    })
}

pub(crate) fn activation_cost_mentions_x(tokens: &[OwnedLexToken]) -> bool {
    tokens
        .iter()
        .filter_map(OwnedLexToken::as_word)
        .any(|word| {
            activated_word_is_any(word, X_COST_WORDS)
                || word
                    .split('/')
                    .any(|part| activated_word_is_any(part, X_COST_WORDS))
        })
}

pub(crate) fn resolve_activated_mana_x_requirements(
    effect: &mut EffectAst,
    sentence_tokens: &[OwnedLexToken],
    x_defined_by_cost: bool,
) -> Result<(), CardTextError> {
    let clause_word_view = ActivationRestrictionCompatWords::new(sentence_tokens);
    let clause_words = clause_word_view.to_word_refs();
    if let Some(where_idx) = clause_words
        .windows(WHERE_X_IS_WORDS.len())
        .position(|window| activated_words_equal(window, WHERE_X_IS_WORDS))
    {
        let clause = clause_words.join(" ");
        let where_token_idx =
            token_index_for_word_index(sentence_tokens, where_idx).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unable to map where-x clause in mana ability (clause: '{clause}')"
                ))
            })?;
        let where_tokens = &sentence_tokens[where_token_idx..];
        let where_value = parse_value_binding_clause_lexed(where_tokens).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported where-x clause in mana ability (clause: '{clause}')"
            ))
        })?;
        replace_unbound_x_in_effect_anywhere(effect, &where_value, &clause)?;
    }

    let x_defined_by_removed_this_way =
        activated_words_contain_phrase(&clause_words, &["this", "way"])
            && activated_words_contain_all(&clause_words, &["removed"])
            && activated_words_contain_any(&clause_words, &["counter", "counters"]);

    if mana_effect_contains_unbound_x(effect)
        && !x_defined_by_cost
        && !x_defined_by_removed_this_way
    {
        return Err(CardTextError::ParseError(format!(
            "unresolved X in mana ability without an X activation cost or where-x definition (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    Ok(())
}

pub(crate) fn mana_effect_contains_unbound_x(effect: &EffectAst) -> bool {
    match effect {
        EffectAst::SubjectVerb(subject_verb) => match &subject_verb.action {
            SubjectVerbActionAst::AddManaScaled { amount, .. }
            | SubjectVerbActionAst::AddManaAnyColor { amount, .. }
            | SubjectVerbActionAst::AddManaAnyOneColor { amount }
            | SubjectVerbActionAst::AddManaChosenColor { amount, .. }
            | SubjectVerbActionAst::AddManaFromLandCouldProduce { amount, .. }
            | SubjectVerbActionAst::AddManaCommanderIdentity { amount } => {
                value_contains_unbound_x(amount)
            }
            SubjectVerbActionAst::AddManaColorsAmong { .. } => false,
            _ => false,
        },
        _ => {
            let mut contains_unbound_x = false;
            for_each_nested_effects(effect, true, |nested| {
                if nested.iter().any(mana_effect_contains_unbound_x) {
                    contains_unbound_x = true;
                }
            });
            contains_unbound_x
        }
    }
}

pub(crate) fn parse_loyalty_shorthand_activation_cost(
    cost_tokens: &[OwnedLexToken],
) -> Option<TotalCost> {
    let cost_tokens = trim_bracketed_loyalty_cost_tokens(cost_tokens);
    let shorthand = match cost_tokens {
        [token] => parse_loyalty_shorthand_word(token.as_word()?),
        [sign, value] if sign.kind == TokenKind::Plus => {
            let amount = parse_ascii_u32(value.as_word()?)?;
            Some(LoyaltyShorthandCost::Add(amount))
        }
        [sign, value] if sign.kind == TokenKind::Dash => {
            let value = value.as_word()?;
            if activated_word_is_any(value, X_COST_WORDS) {
                Some(LoyaltyShorthandCost::RemoveX)
            } else {
                parse_ascii_u32(value).map(LoyaltyShorthandCost::Remove)
            }
        }
        _ => None,
    };
    match shorthand {
        Some(LoyaltyShorthandCost::Add(amount)) => {
            return Some(if amount == 0 {
                TotalCost::free()
            } else {
                TotalCost::from_cost(crate::costs::Cost::add_counters(
                    CounterType::Loyalty,
                    amount,
                ))
            });
        }
        Some(LoyaltyShorthandCost::RemoveX) => {
            return Some(TotalCost::from_cost(
                crate::costs::Cost::remove_any_counters_from_source(
                    Some(CounterType::Loyalty),
                    true,
                ),
            ));
        }
        Some(LoyaltyShorthandCost::Remove(amount)) => {
            return Some(TotalCost::from_cost(crate::costs::Cost::remove_counters(
                CounterType::Loyalty,
                amount,
            )));
        }
        None => {}
    }
    None
}

fn trim_bracketed_loyalty_cost_tokens(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    let mut start = 0usize;
    let mut end = tokens.len();
    if start < end && tokens[start].kind == TokenKind::LBracket {
        start += 1;
    }
    if end > start && tokens[end - 1].kind == TokenKind::RBracket {
        end -= 1;
    }
    &tokens[start..end]
}

enum LoyaltyShorthandCost {
    Add(u32),
    Remove(u32),
    RemoveX,
}

fn parse_loyalty_shorthand_word(word: &str) -> Option<LoyaltyShorthandCost> {
    let mut chars = word.chars();
    let sign = chars.next()?;
    let rest = chars.as_str();
    match sign {
        '0' if rest.is_empty() => Some(LoyaltyShorthandCost::Add(0)),
        '+' => parse_ascii_u32(rest).map(LoyaltyShorthandCost::Add),
        '-' | '−' if activated_word_is_any(rest, X_COST_WORDS) => {
            Some(LoyaltyShorthandCost::RemoveX)
        }
        '-' | '−' => parse_ascii_u32(rest).map(LoyaltyShorthandCost::Remove),
        _ => None,
    }
}

fn parse_ascii_u32(text: &str) -> Option<u32> {
    let mut value = 0u32;
    let mut consumed = false;
    for ch in text.chars() {
        let digit = ch.to_digit(10)?;
        consumed = true;
        value = value.checked_mul(10)?.checked_add(digit)?;
    }
    consumed.then_some(value)
}

pub(crate) fn loyalty_additional_restrictions(is_loyalty_shorthand: bool) -> Vec<String> {
    if !is_loyalty_shorthand {
        return Vec::new();
    }
    vec!["Activate only once each turn.".to_string()]
}

pub(crate) fn first_sacrifice_cost_choice_tag(
    mana_cost: &crate::cost::TotalCost,
) -> Option<TagKey> {
    super::super::util::find_first_sacrifice_cost_choice_tag(mana_cost)
}

pub(crate) fn last_exile_cost_choice_tag(mana_cost: &crate::cost::TotalCost) -> Option<TagKey> {
    super::super::util::find_last_exile_cost_choice_tag(mana_cost)
}

pub(crate) fn infer_activated_functional_zones(
    cost_tokens: &[OwnedLexToken],
    effect_sentences: &[Vec<OwnedLexToken>],
) -> Vec<Zone> {
    let cost_words = crate::runtime_backend::util::non_article_token_word_refs(cost_tokens);
    let effect_words_match = |f: fn(&[&str]) -> bool| {
        effect_sentences.iter().any(|sentence| {
            let clause_words = crate::runtime_backend::util::non_article_token_word_refs(sentence);
            f(&clause_words)
        })
    };
    if contains_source_from_your_graveyard_phrase(&cost_words)
        || effect_words_match(contains_source_from_your_graveyard_phrase)
    {
        vec![Zone::Graveyard]
    } else if contains_from_command_zone_phrase(&cost_words)
        || effect_words_match(contains_from_command_zone_phrase)
    {
        vec![Zone::Command]
    } else if contains_source_from_your_hand_phrase(&cost_words)
        || contains_discard_source_phrase(&cost_words)
        || effect_words_match(contains_source_from_your_hand_phrase)
    {
        vec![Zone::Hand]
    } else {
        vec![Zone::Battlefield]
    }
}

pub(crate) fn infer_activated_functional_zones_lexed(
    cost_tokens: &[OwnedLexToken],
    effect_sentences: &[&[OwnedLexToken]],
) -> Vec<Zone> {
    let cost_view = ActivationRestrictionCompatWords::new(cost_tokens);
    let cost_words = non_article_word_refs(&cost_view.to_word_refs());
    let effect_words_match = |f: fn(&[&str]) -> bool| {
        effect_sentences.iter().any(|sentence| {
            let sentence_view = ActivationRestrictionCompatWords::new(sentence);
            let clause_words = non_article_word_refs(&sentence_view.to_word_refs());
            f(&clause_words)
        })
    };
    let has_stack_only_activation_modifier = effect_sentences.iter().any(|sentence| {
        is_any_player_may_activate_sentence_lexed(sentence)
            && ActivationRestrictionCompatWords::new(sentence).has_phrase(&["on", "the", "stack"])
    });
    if has_stack_only_activation_modifier {
        vec![Zone::Stack]
    } else if contains_source_from_your_graveyard_phrase(&cost_words)
        || effect_words_match(contains_source_from_your_graveyard_phrase)
    {
        vec![Zone::Graveyard]
    } else if contains_from_command_zone_phrase(&cost_words)
        || effect_words_match(contains_from_command_zone_phrase)
    {
        vec![Zone::Command]
    } else if contains_source_from_your_hand_phrase(&cost_words)
        || contains_discard_source_phrase(&cost_words)
        || effect_words_match(contains_source_from_your_hand_phrase)
    {
        vec![Zone::Hand]
    } else {
        vec![Zone::Battlefield]
    }
}

pub(crate) fn parse_activate_only_timing(tokens: &[OwnedLexToken]) -> Option<ActivationTiming> {
    activated_sentence_parsers::parse_activate_only_timing_lexed(tokens)
}

pub(crate) fn parse_activate_only_timing_lexed(
    tokens: &[OwnedLexToken],
) -> Option<ActivationTiming> {
    activated_sentence_parsers::parse_activate_only_timing_lexed(tokens)
}

pub(crate) fn normalize_activate_only_restriction(
    tokens: &[OwnedLexToken],
    timing: &ActivationTiming,
) -> Option<String> {
    activated_sentence_parsers::normalize_activate_only_restriction(tokens, timing)
}

pub(crate) fn is_for_each_color_among_add_mana_clause(tokens: &[OwnedLexToken]) -> bool {
    let words = ActivationRestrictionCompatWords::new(tokens).to_word_refs();
    activated_words_contain_phrase(&words, &["for", "each", "color", "among"])
        && activated_words_contain_phrase(&words, &["add", "one", "mana", "of", "that", "color"])
}

pub(crate) fn flatten_mana_activation_conditions(
    condition: &crate::ConditionExpr,
    out: &mut Vec<crate::ConditionExpr>,
) {
    match condition {
        crate::ConditionExpr::And(left, right) => {
            flatten_mana_activation_conditions(left, out);
            flatten_mana_activation_conditions(right, out);
        }
        _ => out.push(condition.clone()),
    }
}

pub(crate) fn rebuild_mana_activation_conditions(
    conditions: Vec<crate::ConditionExpr>,
) -> Option<crate::ConditionExpr> {
    let mut iter = conditions.into_iter();
    let first = iter.next()?;
    Some(iter.fold(first, |acc, next| {
        crate::ConditionExpr::And(Box::new(acc), Box::new(next))
    }))
}

pub(crate) fn combine_mana_activation_condition(
    base: Option<crate::ConditionExpr>,
    timing: ActivationTiming,
) -> Option<crate::ConditionExpr> {
    if timing == ActivationTiming::AnyTime {
        return base;
    }
    merge_mana_activation_conditions(base, crate::ConditionExpr::ActivationTiming(timing))
}

pub(crate) fn merge_mana_activation_conditions(
    base: Option<crate::ConditionExpr>,
    condition: crate::ConditionExpr,
) -> Option<crate::ConditionExpr> {
    let mut conditions: Vec<crate::ConditionExpr> = Vec::new();
    if let Some(base) = base {
        flatten_mana_activation_conditions(&base, &mut conditions);
    }
    if !conditions.iter().any(|existing| *existing == condition) {
        conditions.push(condition);
    }
    rebuild_mana_activation_conditions(conditions)
}

pub(crate) fn is_activate_only_restriction_sentence(tokens: &[OwnedLexToken]) -> bool {
    activated_sentence_parsers::is_activate_only_restriction_sentence_lexed(tokens)
}

pub(crate) fn is_activate_only_restriction_sentence_lexed(tokens: &[OwnedLexToken]) -> bool {
    activated_sentence_parsers::is_activate_only_restriction_sentence_lexed(tokens)
}

pub(crate) fn is_spend_mana_restriction_sentence(tokens: &[OwnedLexToken]) -> bool {
    activated_sentence_parsers::is_spend_mana_restriction_sentence_lexed(tokens)
}

pub(crate) fn is_spend_mana_restriction_sentence_lexed(tokens: &[OwnedLexToken]) -> bool {
    activated_sentence_parsers::is_spend_mana_restriction_sentence_lexed(tokens)
}

pub(crate) fn parse_mana_usage_restriction_sentence(
    tokens: &[OwnedLexToken],
) -> Option<crate::ability::ManaUsageRestriction> {
    activated_sentence_parsers::parse_mana_usage_restriction_sentence_lexed(tokens)
}

pub(crate) fn parse_mana_usage_restriction_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> Option<crate::ability::ManaUsageRestriction> {
    activated_sentence_parsers::parse_mana_usage_restriction_sentence_lexed(tokens)
}

pub(crate) fn parse_mana_spend_bonus_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> Option<crate::ability::ManaUsageRestriction> {
    activated_sentence_parsers::parse_mana_spend_bonus_sentence_lexed(tokens)
}

pub(crate) fn is_any_player_may_activate_sentence(tokens: &[OwnedLexToken]) -> bool {
    activated_sentence_parsers::is_any_player_may_activate_sentence_lexed(tokens)
}

pub(crate) fn is_any_player_may_activate_sentence_lexed(tokens: &[OwnedLexToken]) -> bool {
    activated_sentence_parsers::is_any_player_may_activate_sentence_lexed(tokens)
}

pub(crate) fn is_trigger_only_restriction_sentence(tokens: &[OwnedLexToken]) -> bool {
    activated_sentence_parsers::is_trigger_only_restriction_sentence_lexed(tokens)
}

pub(crate) fn is_trigger_only_restriction_sentence_lexed(tokens: &[OwnedLexToken]) -> bool {
    activated_sentence_parsers::is_trigger_only_restriction_sentence_lexed(tokens)
}

pub(crate) fn parse_triggered_times_each_turn_sentence(
    sentences: &[Vec<OwnedLexToken>],
) -> Option<u32> {
    activated_sentence_parsers::parse_triggered_times_each_turn_sentence(sentences)
}

pub(crate) fn parse_triggered_times_each_turn_from_words(words: &[&str]) -> Option<u32> {
    activated_sentence_parsers::parse_triggered_times_each_turn_from_words(words)
}

pub(crate) fn parse_triggered_times_each_turn_lexed(tokens: &[OwnedLexToken]) -> Option<u32> {
    activated_sentence_parsers::parse_triggered_times_each_turn_lexed(tokens)
}

pub(crate) fn parse_named_number(word: &str) -> Option<u32> {
    parse_cardinal_u32(word)
}

pub(crate) fn parse_activation_cost(tokens: &[OwnedLexToken]) -> Result<TotalCost, CardTextError> {
    let cst = parse_activation_cost_tokens_rewrite(tokens)?;
    lower_activation_cost_cst(&cst)
}

pub(crate) fn parse_devotion_value_from_add_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<Value>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    let Some(devotion_idx) = words.iter().position(|word| *word == "devotion") else {
        return Ok(None);
    };

    let player = parse_devotion_player_from_words(&words, devotion_idx).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported devotion player in clause (clause: '{}')",
            words.join(" ")
        ))
    })?;

    let to_idx = words[devotion_idx + 1..]
        .iter()
        .position(|word| *word == "to")
        .map(|idx| devotion_idx + 1 + idx)
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing color after devotion clause (clause: '{}')",
                words.join(" ")
            ))
        })?;
    if activated_words_start_with(&words[to_idx + 1..], THAT_COLOR_PREFIX) {
        return Ok(Some(Value::DevotionToChosenColor(player)));
    }
    let color_word = words.get(to_idx + 1).copied().ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing devotion color (clause: '{}')",
            words.join(" ")
        ))
    })?;
    let color_set = parse_color(color_word).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported devotion color '{}' (clause: '{}')",
            color_word,
            words.join(" ")
        ))
    })?;
    let color = color_from_color_set(color_set).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "ambiguous devotion color '{}' (clause: '{}')",
            color_word,
            words.join(" ")
        ))
    })?;

    Ok(Some(Value::Devotion { player, color }))
}

pub(crate) fn parse_devotion_player_from_words(
    words: &[&str],
    devotion_idx: usize,
) -> Option<PlayerFilter> {
    if devotion_idx == 0 {
        return None;
    }
    let left = &words[..devotion_idx];
    if activated_words_end_with(left, &["your"]) {
        return Some(PlayerFilter::You);
    }
    if activated_words_end_with(left, &["their"]) {
        return Some(PlayerFilter::IteratedPlayer);
    }
    if activated_words_end_with_any(left, &[&["opponent"], &["opponents"]]) {
        return Some(PlayerFilter::Opponent);
    }
    if activated_words_end_with_any(left, THAT_PLAYER_DEVOTION_OWNER_SUFFIXES) {
        return Some(PlayerFilter::Target(Box::new(PlayerFilter::Any)));
    }
    None
}

pub(crate) fn color_from_color_set(colors: ColorSet) -> Option<crate::color::Color> {
    let mut found = None;
    for color in [
        crate::color::Color::White,
        crate::color::Color::Blue,
        crate::color::Color::Black,
        crate::color::Color::Red,
        crate::color::Color::Green,
    ] {
        if colors.intersection(ColorSet::from_color(color)).count() > 0 {
            if found.is_some() {
                return None;
            }
            found = Some(color);
        }
    }
    found
}

pub(crate) fn parse_activation_condition(tokens: &[OwnedLexToken]) -> Option<crate::ConditionExpr> {
    activated_sentence_parsers::parse_activation_condition_lexed(tokens)
}

pub(crate) fn parse_activation_condition_lexed(
    tokens: &[OwnedLexToken],
) -> Option<crate::ConditionExpr> {
    activated_sentence_parsers::parse_activation_condition_lexed(tokens)
}

pub(crate) fn parse_cardinal_u32(word: &str) -> Option<u32> {
    let token = OwnedLexToken::word(word.to_string(), TextSpan::synthetic());
    parse_number(&[token]).map(|(value, _)| value)
}

pub(crate) fn parse_activation_count_per_turn(words: &[&str]) -> Option<u32> {
    activated_sentence_parsers::parse_activation_count_per_turn(words)
}

pub(crate) fn parse_enters_tapped_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if clause_words.is_empty() {
        return Ok(None);
    }
    if is_negated_untap_clause(&clause_words) {
        let has_enters_tapped = activated_words_contain_all(&clause_words, &["enters", "tapped"]);
        if has_enters_tapped {
            return Err(CardTextError::ParseError(format!(
                "unsupported mixed enters-tapped and negated-untap clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        return Ok(None);
    }
    if activated_words_start_with(&clause_words, THIS_ENTERS_TAPPED_PREFIX)
        && activated_words_contain_all(&clause_words, &["enters", "tapped"])
    {
        let tapped_word_idx = clause_words
            .iter()
            .position(|word| *word == "tapped")
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing tapped keyword in enters-tapped clause (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?;
        let tapped_token_idx =
            token_index_for_word_index(tokens, tapped_word_idx).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unable to map tapped keyword in enters-tapped clause (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?;
        let trailing_words =
            crate::runtime_backend::token_word_refs(&tokens[tapped_token_idx + 1..]);
        if activated_words_equal_any(&trailing_words, ATTACKING_TRAILS) {
            return Ok(None);
        }
        if !trailing_words.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing enters-tapped clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        return Ok(Some(StaticAbility::enters_tapped_ability()));
    }
    Ok(None)
}

pub(crate) fn parse_cost_reduction_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let line_words = crate::runtime_backend::token_word_refs(tokens);
    if activated_words_start_with(&line_words, THIS_COST_REDUCED_BY_PREFIX) && line_words.len() > 6
    {
        let amount_tokens = trim_commas(&tokens[5..]);
        let parsed_amount = parse_cost_modifier_amount(&amount_tokens);
        let (amount_value, used) = parsed_amount.clone().unwrap_or((Value::Fixed(1), 0));
        let amount_fixed = if let Value::Fixed(value) = amount_value {
            value
        } else {
            1
        };
        let remaining_tokens = amount_tokens.get(used..).unwrap_or_default();
        let remaining_words = crate::runtime_backend::token_word_refs(remaining_tokens);
        if activated_words_contain_all(&remaining_words, &["for", "each"])
            && let Some(dynamic) = parse_dynamic_cost_modifier_value(remaining_tokens)?
        {
            let reduction = scale_dynamic_cost_modifier_value(dynamic, amount_fixed);
            return Ok(Some(StaticAbility::new(
                crate::static_abilities::ThisSpellCostReduction::new(
                    reduction,
                    crate::static_abilities::ThisSpellCostCondition::Always,
                ),
            )));
        }

        let amount_word = line_words[5];
        let amount_text = if amount_word.chars().all(|ch| ch.is_ascii_digit()) {
            format!("{{{amount_word}}}")
        } else {
            amount_word.to_string()
        };
        let tail = line_words[6..].join(" ");
        let text = format!("This cost is reduced by {amount_text} {tail}");
        return Err(CardTextError::ParseError(format!(
            "unsupported cost-reduction static clause (clause: '{}')",
            text
        )));
    }

    if activated_words_start_with(&line_words, ACTIVATED_ABILITIES_OF_PREFIX) {
        let Some(cost_idx) = line_words
            .iter()
            .position(|word| activated_word_is_any(*word, &["cost", "costs"]))
        else {
            return Ok(None);
        };
        if cost_idx <= 3 {
            return Ok(None);
        }
        let subject_tokens = trim_commas(&tokens[3..cost_idx]);
        if subject_tokens.is_empty() {
            return Ok(None);
        }
        let mut filter = parse_object_filter(&subject_tokens, false).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported activated-ability cost reduction subject (clause: '{}')",
                line_words.join(" ")
            ))
        })?;
        if filter.zone.is_none() {
            filter.zone = Some(Zone::Battlefield);
        }

        let amount_tokens = trim_commas(&tokens[cost_idx + 1..]);
        let Some((amount_value, used)) = parse_cost_modifier_amount(&amount_tokens) else {
            return Ok(None);
        };
        let reduction = match amount_value {
            Value::Fixed(value) if value > 0 => value as u32,
            _ => {
                return Err(CardTextError::ParseError(format!(
                    "unsupported activated-ability cost reduction amount (clause: '{}')",
                    line_words.join(" ")
                )));
            }
        };
        let tail_words = crate::runtime_backend::token_word_refs(&amount_tokens[used..]);
        if !activated_words_start_with(&tail_words, LESS_TO_ACTIVATE_PREFIX) {
            return Ok(None);
        }

        return Ok(Some(StaticAbility::reduce_activated_ability_costs(
            filter,
            reduction,
            Some(1),
        )));
    }

    if activated_words_start_with(&line_words, THIS_ABILITY_COSTS_PREFIX) {
        let amount_tokens = trim_commas(&tokens[3..]);
        let Some((amount_value, used)) = parse_cost_modifier_amount(&amount_tokens) else {
            return Ok(None);
        };
        let reduction = match amount_value {
            Value::Fixed(value) if value > 0 => value as u32,
            _ => {
                return Err(CardTextError::ParseError(format!(
                    "unsupported activated-ability cost reduction amount (clause: '{}')",
                    line_words.join(" ")
                )));
            }
        };
        let tail_tokens = trim_commas(&amount_tokens[used..]);
        let tail_words = crate::runtime_backend::token_word_refs(&tail_tokens);
        if activated_words_equal(&tail_words, LESS_TO_ACTIVATE_WORDS) {
            return Ok(Some(StaticAbility::reduce_activated_ability_costs(
                ObjectFilter::source(),
                reduction,
                None,
            )));
        }
        if activated_words_start_with(&tail_words, LESS_TO_ACTIVATE_IF_PREFIX) {
            let condition_tokens = trim_commas(&tail_tokens[4..]);
            let condition_words = crate::runtime_backend::token_word_refs(&condition_tokens);
            if activated_words_start_with(&condition_words, IT_TARGETS_PREFIX) {
                let (count, used) = parse_number(&condition_tokens[2..]).ok_or_else(|| {
                    CardTextError::ParseError(format!(
                        "unsupported activated-ability target condition count (clause: '{}')",
                        line_words.join(" ")
                    ))
                })?;
                let mut filter = parse_object_filter(&condition_tokens[2 + used..], false)
                    .map_err(|_| {
                        CardTextError::ParseError(format!(
                            "unsupported activated-ability target condition filter (clause: '{}')",
                            line_words.join(" ")
                        ))
                    })?;
                if filter.zone.is_none() {
                    filter.zone = Some(Zone::Battlefield);
                }
                return Ok(Some(
                    StaticAbility::reduce_activated_ability_costs_if_targets(
                        ObjectFilter::source(),
                        reduction,
                        crate::static_abilities::ActivatedAbilityCostCondition::TargetsExactly {
                            count: count as usize,
                            filter,
                        },
                        None,
                    ),
                ));
            }
            return Err(CardTextError::ParseError(format!(
                "unsupported activated-ability cost reduction condition (clause: '{}')",
                line_words.join(" ")
            )));
        }
        if activated_words_start_with(&tail_words, LESS_TO_ACTIVATE_FOR_EACH_PREFIX) {
            if let Some(Value::BasicLandTypesAmong(lands_filter)) =
                parse_dynamic_cost_modifier_value(&tail_tokens)?
            {
                return Ok(Some(
                    StaticAbility::reduce_activated_ability_costs_for_each_basic_land_type(
                        ObjectFilter::source(),
                        reduction,
                        lands_filter,
                        None,
                    ),
                ));
            }
            let mut per_filter = parse_object_filter(&tail_tokens[5..], false).map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported activated-ability cost reduction tail (clause: '{}')",
                    line_words.join(" ")
                ))
            })?;
            if per_filter.zone.is_none() {
                per_filter.zone = Some(Zone::Battlefield);
            }
            return Ok(Some(
                StaticAbility::reduce_activated_ability_costs_for_each(
                    ObjectFilter::source(),
                    reduction,
                    per_filter,
                    None,
                ),
            ));
        }
    }

    if !activated_words_start_with(&line_words, THIS_SPELL_COSTS_PREFIX) {
        return Ok(None);
    }

    let costs_idx = find_token_word_exact(tokens, "costs")
        .ok_or_else(|| CardTextError::ParseError("missing costs keyword".to_string()))?;
    let amount_tokens = &tokens[costs_idx + 1..];
    let parsed_amount = parse_cost_modifier_amount(amount_tokens);
    let (amount_value, used) = parsed_amount.clone().unwrap_or((Value::Fixed(1), 0));
    let amount_fixed = if let Value::Fixed(value) = amount_value {
        value
    } else {
        1
    };

    let remaining_tokens = &tokens[costs_idx + 1 + used..];
    let remaining_words: Vec<&str> = crate::runtime_backend::token_word_refs(remaining_tokens);

    if !activated_words_contain_all(&remaining_words, &["less"]) {
        return Ok(None);
    }

    if let Some(dynamic) = parse_dynamic_cost_modifier_value(remaining_tokens)? {
        let reduction =
            crate::static_abilities::CostReduction::new(ObjectFilter::default(), dynamic);
        return Ok(Some(StaticAbility::new(reduction)));
    }

    if parsed_amount.is_none() {
        return Ok(None);
    }

    let has_each = activated_words_contain_all(&remaining_words, &["each"]);
    let has_card_type = activated_words_contain_phrase(&remaining_words, &["card", "type"]);
    let has_graveyard = activated_words_contain_all(&remaining_words, &["graveyard"]);

    if has_each && has_card_type && has_graveyard {
        if amount_fixed != 1 {
            return Ok(None);
        }
        let reduction = crate::effect::Value::CardTypesInGraveyard(PlayerFilter::You);
        let cost_reduction =
            crate::static_abilities::CostReduction::new(ObjectFilter::default(), reduction);
        return Ok(Some(StaticAbility::new(cost_reduction)));
    }

    Ok(None)
}

pub(crate) fn scale_dynamic_cost_modifier_value(dynamic: Value, multiplier: i32) -> Value {
    if multiplier <= 0 {
        return Value::Fixed(0);
    }
    if multiplier == 1 {
        return dynamic;
    }
    match dynamic {
        Value::Count(filter) => Value::CountScaled(filter, multiplier),
        Value::CountScaled(filter, factor) => Value::CountScaled(filter, factor * multiplier),
        other => {
            let mut scaled = other.clone();
            for _ in 1..multiplier {
                scaled = Value::Add(Box::new(scaled), Box::new(other.clone()));
            }
            scaled
        }
    }
}

pub(crate) fn parse_all_creatures_able_to_block_source_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let words_storage = normalize_cant_words(tokens);
    let words = words_storage.iter().map(String::as_str).collect::<Vec<_>>();
    if activated_words_equal_any(&words, ALL_CREATURES_ABLE_TO_BLOCK_SOURCE_WORDS) {
        return Ok(Some(StaticAbilityAst::GrantStaticAbility {
            filter: ObjectFilter::creature(),
            ability: Box::new(StaticAbilityAst::Static(StaticAbility::must_block())),
            condition: None,
        }));
    }
    Ok(None)
}

pub(crate) fn parse_source_must_be_blocked_if_able_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words_storage = normalize_cant_words(tokens);
    let words = words_storage.iter().map(String::as_str).collect::<Vec<_>>();
    if activated_words_equal_any(&words, SOURCE_MUST_BE_BLOCKED_IF_ABLE_WORDS) {
        return Ok(Some(StaticAbility::restriction(
            crate::effect::Restriction::must_be_blocked(ObjectFilter::source()),
            "this creature must be blocked if able".to_string(),
        )));
    }
    Ok(None)
}
