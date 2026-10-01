use super::*;
use crate::cards::builders::ConditionalEffectAst;
use crate::cards::builders::ForEachEffectAst;

fn parse_required_fight_effect(tokens: &[OwnedLexToken]) -> Result<Vec<EffectAst>, CardTextError> {
    let Some(effect) = super::super::clause_primitives::parse_fight_clause(tokens)? else {
        return Err(CardTextError::ParseError(
            "conditional fight consequence did not contain a fight clause".to_string(),
        ));
    };
    Ok(vec![effect])
}

fn parse_conditional_fight_effect(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((_, consequence_tokens)) =
        crate::grammar::primitives::split_lexed_once_on_comma(tokens)
    else {
        return Ok(None);
    };
    if crate::grammar::effects::clause_primitive_shapes::parse_fight_shape(consequence_tokens)
        .is_none()
    {
        return Ok(None);
    }
    crate::grammar::effects::parse_conditional_sentence_with_grammar_entrypoint_lexed(
        tokens,
        parse_required_fight_effect,
    )
    .map(Some)
}

#[inline(never)]
fn parse_quantified_player_shared_resource_chain(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(scope) =
        crate::grammar::effects::chain_carry::parse_leading_chain_scope_tokens(tokens)
    else {
        return Ok(None);
    };
    let segments = crate::grammar::primitives::split_lexed_slices_on_and(tokens);
    if segments.len() < 2 {
        return Ok(None);
    }
    let mut effects = Vec::with_capacity(segments.len());
    for (index, segment) in segments.into_iter().enumerate() {
        let Some((verb, verb_idx)) = super::super::find_verb(segment) else {
            return Ok(None);
        };
        if !matches!(verb, Verb::Draw | Verb::Gain) || (index > 0 && verb_idx != 0) {
            return Ok(None);
        }
        let body = segment.get(verb_idx + 1..).unwrap_or_default();
        effects.push(super::super::verb_handlers::parse_effect_with_verb(
            verb,
            Some(SubjectAst::Player(PlayerAst::That)),
            body,
        )?);
    }
    let effects = vec![EffectAst::Coordinated {
        effects,
        leading_duration: false,
        result_conjunction: false,
    }];
    Ok(Some(vec![match scope {
        crate::grammar::effects::chain_carry::ChainPlayerScope::EachOpponent => {
            EffectAst::ForEach(ForEachEffectAst::ForEachOpponent { effects })
        }
        crate::grammar::effects::chain_carry::ChainPlayerScope::EachPlayer => {
            EffectAst::ForEach(ForEachEffectAst::ForEachPlayer { effects })
        }
    }]))
}

#[inline(never)]
fn parse_granted_trigger_simple_sentence_sequence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentences = crate::lexer::split_lexed_sentences(tokens);
    if sentences.len() < 2 {
        return Ok(None);
    }
    let mut effects = Vec::with_capacity(sentences.len());
    for sentence in sentences {
        let sentence = crate::util::trim_edge_punctuation_tokens(sentence);
        if sentence
            .first()
            .is_some_and(|token| token.is_word("sacrifice"))
        {
            effects.push(super::super::zone_handlers::parse_sacrifice(
                sentence, None, None,
            )?);
            continue;
        }
        let Some((verb, verb_idx)) = super::super::find_verb(sentence) else {
            return Ok(None);
        };
        if !matches!(verb, Verb::Draw | Verb::Gain | Verb::Lose)
            || verb_idx != 1
            || !sentence.first().is_some_and(|token| token.is_word("you"))
        {
            return Ok(None);
        }
        effects.push(super::super::verb_handlers::parse_effect_with_verb(
            verb,
            Some(SubjectAst::Player(PlayerAst::You)),
            sentence.get(verb_idx + 1..).unwrap_or_default(),
        )?);
    }
    Ok(Some(effects))
}

#[inline(never)]
fn parse_granted_optional_tap_effect(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) =
        crate::grammar::effects::clause_dispatch_shapes::parse_leading_may_shape(tokens)
    else {
        return Ok(None);
    };
    let Some((verb, target_tokens)) = shape.effect_tokens.split_first() else {
        return Ok(None);
    };
    if !verb.is_word("tap") || target_tokens.is_empty() {
        return Ok(None);
    }
    let tap = super::super::zone_handlers::parse_tap(target_tokens)?;
    Ok(Some(vec![match shape.actor {
        crate::grammar::effects::clause_dispatch_shapes::LeadingMayActorShape::Player(player) => {
            EffectAst::Permissions(PermissionEffectAst::MayByPlayer {
                player,
                effects: vec![tap],
            })
        }
        crate::grammar::effects::clause_dispatch_shapes::LeadingMayActorShape::Implicit => {
            EffectAst::Permissions(PermissionEffectAst::May { effects: vec![tap] })
        }
    }]))
}

#[inline(never)]
fn parse_granted_simple_trailing_unless(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(unless_idx) =
        crate::slice_primitives::select_position(tokens, |token| token.is_word("unless"))
    else {
        return Ok(None);
    };
    if unless_idx == 0 || unless_idx + 1 >= tokens.len() {
        return Ok(None);
    }
    let leading_tokens = crate::util::trim_edge_punctuation_tokens(&tokens[..unless_idx]);
    if leading_tokens.len() < 3
        || !leading_tokens[0].is_word("you")
        || !leading_tokens[1].is_word("lose")
    {
        return Ok(None);
    }
    let effect = super::super::verb_handlers::parse_effect_with_verb(
        Verb::Lose,
        Some(SubjectAst::Player(PlayerAst::You)),
        &leading_tokens[2..],
    )?;
    let Some(unless) = super::super::subject_verb_primitives::try_build_simple_unless_pays(
        vec![effect],
        &tokens[unless_idx + 1..],
    )?
    else {
        return Ok(None);
    };
    Ok(Some(vec![unless]))
}

#[inline(never)]
fn parse_granted_trigger_pump_if_monarch_otherwise(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentences = crate::lexer::split_lexed_sentences(tokens);
    let [conditional_tokens, otherwise_tokens] = sentences.as_slice() else {
        return Ok(None);
    };
    let Some(trailing_if) =
        crate::grammar::structure::split_trailing_if_clause_lexed(conditional_tokens)
    else {
        return Ok(None);
    };
    if !matches!(
        trailing_if.predicate,
        PredicateAst::Player(PlayerPredicateAst::PlayerIsMonarch { .. })
    ) {
        return Ok(None);
    }
    let Some(shape) =
        crate::grammar::effects::clause_dispatch_shapes::parse_clause_subject_verb_shape(
            trailing_if.leading_tokens,
        )
    else {
        return Ok(None);
    };
    if shape.kind != crate::grammar::effects::chain_splitting::ChainVerbKind::Get {
        return Ok(None);
    }
    let Some(pump) = super::super::parse_get_pump_clause(
        shape.subject_tokens,
        shape.action_tokens,
        trailing_if.leading_tokens,
    )?
    else {
        return Ok(None);
    };
    let otherwise_words = crate::lexer::token_word_refs(otherwise_tokens);
    if !crate::word_primitives::parse_any_sequence_complete(
        &otherwise_words,
        &[
            &["otherwise", "you", "become", "the", "monarch"],
            &["otherwise", "you", "become", "monarch"],
        ],
    ) {
        return Ok(None);
    }
    Ok(Some(vec![EffectAst::Conditionals(
        ConditionalEffectAst::Conditional {
            predicate: trailing_if.predicate,
            if_true: vec![pump],
            if_false: vec![EffectAst::subject_verb_become_monarch(PlayerAst::You)],
        },
    )]))
}

pub(super) fn parse_granted_trigger_with_nested_token_rule(
    ability_tokens: &[OwnedLexToken],
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

    // This candidate exists only when both ordinary typed parsers prove the
    // stripped trigger/effect boundary and the token rule reattaches.
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
        trigger_surface::parse_trigger_frequency_condition_tokens(ability_tokens, None),
        None,
        ReferenceImports::default(),
    )))
}

#[inline(never)]
fn parse_granted_composable_event_trigger(
    ability_tokens: &[OwnedLexToken],
) -> Result<Option<ParsedAbility>, CardTextError> {
    let trigger_intro = clause_grammar::parse_trigger_intro_tokens(ability_tokens);
    let Some(split_idx) =
        clause_grammar::parse_trigger_delimiters_tokens(ability_tokens).first_comma
    else {
        return Ok(None);
    };
    if split_idx <= trigger_intro.body_first || split_idx + 1 >= ability_tokens.len() {
        return Ok(None);
    }
    let trigger_tokens = &ability_tokens[trigger_intro.body_first..split_idx];
    let trigger = parse_trigger_clause_lexed(trigger_tokens)?;
    if !matches!(
        trigger,
        TriggerSpec::ThisAttacks
            | TriggerSpec::ThisBlocksObject { .. }
            | TriggerSpec::ThisEntersBattlefield { .. }
            | TriggerSpec::ThisEntersBattlefieldWithSurface { .. }
            | TriggerSpec::ThisEntersBattlefieldFromZone { .. }
            | TriggerSpec::ThisLeavesBattlefield
            | TriggerSpec::ThisLeavesBattlefieldWithSurface(_)
            | TriggerSpec::ThisDies
            | TriggerSpec::ThisDealsDamageToPlayer { .. }
            | TriggerSpec::ThisIsDealtDamage
            | TriggerSpec::BeginningOfUpkeep(_)
            | TriggerSpec::BeginningOfEndStep(_)
            | TriggerSpec::BeginningOfTheEndStep
            | TriggerSpec::BeginningOfMonarchEndStep
    ) {
        return Ok(None);
    }
    let effect_tokens = trim_lexed_commas(&ability_tokens[split_idx + 1..]);
    // "Whenever ..., if <condition>, A. B." is an intervening-if over the
    // whole multi-sentence body (Agent of the Shadow Thieves); the complete
    // triggered-line reader keeps that scope, this sentence composer doesn't.
    if effect_tokens.first().is_some_and(|token| token.is_word("if"))
        && crate::lexer::split_lexed_sentences(effect_tokens)
            .iter()
            .filter(|sentence| !sentence.is_empty())
            .count()
            > 1
    {
        return Ok(None);
    }
    // "it deals 1 damage to target player or planeswalker and you gain 1
    // life" is a damage clause coordinated with a second player-subject
    // clause. The single-clause damage reader would read the whole tail as
    // the damage recipient (dropping the second clause), so leave it to the
    // coordinated effect-chain parser below.
    let effect_words = crate::lexer::token_word_refs(effect_tokens);
    let coordinates_player_subject_clause = crate::word_primitives::any_sequence_occurs(
        &effect_words,
        &[
            &["and", "you"],
            &["and", "its", "controller"],
            &["and", "that", "player"],
        ],
    );
    let effects = if !coordinates_player_subject_clause
        && let Some(effect) =
            super::super::clause_primitives::parse_anaphoric_object_deals_damage_clause(
                effect_tokens,
            )?
    {
        vec![effect]
    } else if let Some(effects) = super::super::parse_complete_create_statement(effect_tokens)? {
        effects
    } else if effect_tokens
        .first()
        .is_some_and(|token| token.is_word("put"))
        && effect_tokens
            .iter()
            .any(|token| token.is_word("counter") || token.is_word("counters"))
    {
        vec![super::super::zone_counter_helpers::parse_put_counters(
            effect_tokens,
        )?]
    } else if let Some(effects) = parse_granted_trigger_pump_if_monarch_otherwise(effect_tokens)? {
        effects
    } else if let Some(effects) = parse_granted_trigger_simple_sentence_sequence(effect_tokens)? {
        effects
    } else if let Some(effects) = parse_granted_optional_tap_effect(effect_tokens)? {
        effects
    } else if let Some(effects) = parse_granted_simple_trailing_unless(effect_tokens)? {
        effects
    } else if effect_tokens
        .first()
        .is_some_and(|token| token.is_word("sacrifice"))
        && crate::lexer::split_lexed_sentences(effect_tokens).len() == 1
    {
        vec![super::super::zone_handlers::parse_sacrifice(
            effect_tokens,
            None,
            None,
        )?]
    } else if let Some(effects) = parse_quantified_player_shared_resource_chain(effect_tokens)? {
        effects
    } else if let Some(effects) =
        super::super::parse_complete_simple_subject_verb_sentence(effect_tokens)?
    {
        vec![effects]
    } else if let Some(effects) = parse_conditional_fight_effect(effect_tokens)? {
        effects
    } else if let Some(effects) =
        super::super::parse_complete_composable_fight_program(effect_tokens)?
    {
        effects
    } else if effect_tokens
        .iter()
        .any(|token| token.kind == TokenKind::Period)
    {
        super::super::parse_effect_sentences_lexed(effect_tokens)?
    } else {
        super::super::parse_effect_sentence_lexed(effect_tokens)?
    };
    Ok(Some(parsed_triggered_ability(
        trigger,
        effects,
        vec![Zone::Battlefield],
        trigger_surface::parse_trigger_frequency_condition_tokens(ability_tokens, None),
        None,
        ReferenceImports::default(),
    )))
}

fn recognize_granted_trigger_ability(
    tokens: &[OwnedLexToken],
) -> crate::recognition::ParseOutcome<ParsedAbility> {
    use crate::recognition::{ParseDiagnostic, ParseOutcome, RuleId};
    use crate::registry::{
        HeadDiscriminator, RegistryCandidate, RegistryRuleMetadata, resolve_registry_candidates,
    };

    let span = span_from_tokens(tokens);
    let mut candidates = Vec::new();
    let mut diagnostics = Vec::new();
    let mut add =
        |id: &'static str, result: Result<Option<ParsedAbility>, CardTextError>| match result {
            Ok(Some(value)) => {
                if candidates
                    .iter()
                    .any(|candidate: &RegistryCandidate<ParsedAbility>| candidate.value == value)
                {
                    return;
                }
                candidates.push(RegistryCandidate::new(
                    RegistryRuleMetadata::distinct(RuleId::new(id), HeadDiscriminator::grammar(id)),
                    value,
                    span,
                ));
            }
            Ok(None) => {}
            Err(error) => diagnostics.push(ParseDiagnostic::from_card_text_error(
                RuleId::new(id),
                span,
                error,
            )),
        };

    add(
        "granted-trigger-nested-token-rule",
        parse_granted_trigger_with_nested_token_rule(tokens),
    );
    let otherwise = parse_granted_triggered_otherwise_ability(tokens);
    let joined_otherwise = matches!(&otherwise, Ok(Some(_)));
    add("granted-trigger-otherwise", otherwise);
    add(
        "granted-trigger-composable-event",
        parse_granted_composable_event_trigger(tokens),
    );
    add(
        "granted-trigger-complete-line",
        (|| {
            // The specialized reader proves the otherwise clause belongs to
            // the preceding condition. The generic sentence reader can leave
            // it as a separate effect, so it must not compete on that input.
            if joined_otherwise {
                return Ok(None);
            }
            let LineAst::Triggered {
                trigger,
                effects,
                max_triggers_per_turn,
            } = parse_triggered_line_lexed(tokens)?
            else {
                return Ok(None);
            };
            let frequency = trigger_surface::parse_trigger_frequency_condition_tokens(
                tokens,
                max_triggers_per_turn,
            );
            // A leading "if" right after the trigger comma that governs the
            // whole body is the ability's intervening-if (CR 603.4).
            let leads_with_if = tokens
                .iter()
                .position(|token| token.is_comma())
                .and_then(|comma| tokens.get(comma + 1))
                .is_some_and(|token| token.is_word("if"));
            let (effects, intervening_if) = match effects.as_slice() {
                [
                    EffectAst::Conditionals(ConditionalEffectAst::Conditional {
                        predicate,
                        if_true,
                        if_false,
                    }),
                ] if leads_with_if
                    && if_false.is_empty()
                    // A source condition ("if Cosima is exiled") is what
                    // binds the body's "it" to the source; keep it in place.
                    && !predicate.establishes_source_object_antecedent() =>
                {
                    let intervening_if = match frequency {
                        Some(right) => Some(crate::cards::builders::PredicateAst::And(
                            Box::new(predicate.clone()),
                            Box::new(right),
                        )),
                        None => Some(predicate.clone()),
                    };
                    (if_true.clone(), intervening_if)
                }
                _ => (effects, frequency),
            };
            Ok(Some(parsed_triggered_ability(
                trigger,
                effects,
                vec![Zone::Battlefield],
                intervening_if,
                None,
                ReferenceImports::default(),
            )))
        })(),
    );

    resolve_registry_candidates(
        RuleId::new("granted-trigger-ability-registry"),
        candidates,
        diagnostics,
    )
    .map(|matched| matched.value)
}

fn apply_granted_trigger_intro_surface(
    parsed_ability: &mut ParsedAbility,
    ability_tokens: &[OwnedLexToken],
) {
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
}

#[inline(never)]
pub fn parse_granted_activated_or_triggered_ability_for_gain(
    ability_tokens: &[OwnedLexToken],
    clause_words: &[&str],
) -> Result<Option<GrantedAbilityAst>, CardTextError> {
    let trimmed = trim_edge_punctuation_and_quotes(ability_tokens);
    let direct_trigger_subject = match (trimmed.first(), trimmed.get(1)) {
        (Some(intro), Some(subject)) if matches!(intro.parser_text(), "when" | "whenever") => {
            subject.is_word("this")
        }
        (Some(intro), Some(article)) if intro.is_word("at") => article.is_word("the"),
        _ => false,
    };
    if direct_trigger_subject
        && !trimmed
            .iter()
            .any(|token| matches!(token.kind, TokenKind::Apostrophe | TokenKind::Quote))
    {
        let display = display_text_for_tokens(&trimmed);
        if let Some(mut parsed) = parse_granted_composable_event_trigger(&trimmed)? {
            apply_granted_trigger_intro_surface(&mut parsed, &trimmed);
            return Ok(Some(GrantedAbilityAst::ParsedObjectAbility {
                ability: Box::new(parsed),
                display,
            }));
        }
    }
    parse_granted_activated_or_triggered_ability_for_gain_remaining(ability_tokens, clause_words)
}

#[inline(never)]
fn parse_granted_activated_or_triggered_ability_for_gain_remaining(
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
        match recognize_granted_trigger_ability(&semantic_tokens) {
            crate::recognition::ParseOutcome::Match(matched) => matched.value,
            crate::recognition::ParseOutcome::NoMatch => {
                return Err(CardTextError::ParseError(format!(
                    "unsupported granted activated/triggered ability clause (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            crate::recognition::ParseOutcome::Error(diagnostic) => {
                return Err(diagnostic.into_card_text_error());
            }
        }
    } else {
        let Some(mut parsed) = parse_activated_line(&semantic_tokens)? else {
            return Err(CardTextError::ParseError(format!(
                "unsupported granted activated/triggered ability clause (clause: '{}')",
                clause_words.join(" ")
            )));
        };
        apply_quoted_activation_restrictions(&mut parsed, &semantic_tokens);
        parsed
    };

    // A generic quoted token ability can use the token's authored name as its
    // trigger subject (`When Ember dies, ...`). That route parses a complete
    // typed zone-change trigger, but unlike the ordinary triggered-line CST
    // handoff it can arrive without the leading trigger presentation. Carry
    // only the explicit first-word intro onto that already-typed trigger;
    // this keeps `When` distinct from `Whenever` without inferring frequency
    // from the matched event.
    apply_granted_trigger_intro_surface(&mut parsed_ability, &ability_tokens);

    Ok(Some(GrantedAbilityAst::ParsedObjectAbility {
        ability: Box::new(parsed_ability),
        display,
    }))
}

/// A printed activated ability has its "Activate only ..." sentences lowered
/// from the line's pending restrictions; a quoted granted ability ("{0}:
/// Untap this creature. Activate only once.", Touch of Vitae) has no such
/// line, so its restriction sentences are read here into the same typed
/// timing and conditions (CR 602.5b).
fn apply_quoted_activation_restrictions(parsed: &mut ParsedAbility, tokens: &[OwnedLexToken]) {
    let ironsmith_core::AbilityKind::Activated(activated) = &mut parsed.ability.kind else {
        return;
    };
    fn contains_conjunct(condition: &PredicateAst, needle: &PredicateAst) -> bool {
        match condition {
            PredicateAst::And(left, right) => {
                contains_conjunct(left, needle) || contains_conjunct(right, needle)
            }
            other => other == needle,
        }
    }
    fn conjuncts(condition: PredicateAst, out: &mut Vec<PredicateAst>) {
        match condition {
            PredicateAst::And(left, right) => {
                conjuncts(*left, out);
                conjuncts(*right, out);
            }
            other => out.push(other),
        }
    }
    let Some(colon) = tokens
        .iter()
        .position(|token| token.kind == TokenKind::Colon)
    else {
        return;
    };
    for sentence in ironsmith_grammar_common::primitives::split_lexed_slices_on_period(&tokens[colon + 1..]) {
        let Some(restriction) =
            crate::grammar::restriction_facts::parse_activation_restriction_tokens(sentence)
        else {
            continue;
        };
        if let Some(timing) = restriction.timing
            && timing != activated.timing
        {
            if activated.timing == crate::ability::ActivationTiming::AnyTime {
                activated.timing = timing;
            } else {
                let timing_condition = PredicateAst::ActivationTiming(timing);
                if !activated
                    .activation_condition
                    .as_ref()
                    .is_some_and(|existing| contains_conjunct(existing, &timing_condition))
                {
                    activated.activation_condition =
                        Some(match activated.activation_condition.take() {
                            Some(existing) => {
                                PredicateAst::And(Box::new(existing), Box::new(timing_condition))
                            }
                            None => timing_condition,
                        });
                }
            }
        }
        let mut stated = Vec::new();
        if let Some(condition) = restriction.condition {
            conjuncts(condition, &mut stated);
        }
        for condition in stated {
            if activated
                .activation_condition
                .as_ref()
                .is_some_and(|existing| contains_conjunct(existing, &condition))
            {
                continue;
            }
            activated.activation_condition = Some(match activated.activation_condition.take() {
                Some(existing) => PredicateAst::And(Box::new(existing), Box::new(condition)),
                None => condition,
            });
        }
    }
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
    // "Reveal the top card of your library. If it's a land card, put it onto
    // the battlefield. Otherwise, ...": this repair reads the consequence as
    // one sentence. Several sentences before "otherwise" belong to the
    // complete-line parser, which pairs the fallback with its own condition.
    if true_tokens
        .iter()
        .any(|token| token.kind == TokenKind::Period)
    {
        return Ok(None);
    }

    // The complete-line parser owns a canonical condition with both branches.
    // This older repair is needed only when sentence parsing has not joined
    // the trailing otherwise clause; emitting a second legacy Conditional
    // would conflict with the canonical postcondition solely by AST shape.
    if let Ok(effects) =
        crate::effect_sentences::parse_effect_sentences_lexed(&ability_tokens[comma_idx + 1..])
        && let [EffectAst::ControlFlow(control)] = effects.as_slice()
        && matches!(
            control.node,
            crate::model::control_flow::ControlFlowNodeAst::Condition {
                alternative_program: Some(_),
                ..
            }
        )
    {
        return Ok(None);
    }

    let true_effect = match crate::effect_sentences::parse_effect_sentences_lexed(&true_tokens) {
        Ok(mut effects)
            if effects.len() == 1 && matches!(&effects[0], EffectAst::ControlFlow(_)) =>
        {
            effects.remove(0)
        }
        _ => parse_single_effect_sentence_for_granted_otherwise(&true_tokens)?,
    };
    let mut false_effect = Some(parse_single_effect_sentence_for_granted_otherwise(
        &false_tokens,
    )?);
    let mut conditional = match true_effect {
        EffectAst::Conditionals(ConditionalEffectAst::Conditional {
            predicate,
            if_true,
            if_false,
        }) if if_false.is_empty() => EffectAst::Conditionals(ConditionalEffectAst::Conditional {
            predicate,
            if_true,
            if_false,
        }),
        EffectAst::Conditionals(ConditionalEffectAst::TrailingIf { predicate, effects }) => {
            use crate::model::control_flow::{
                ConditionPositionAst, ControlConditionAst, ControlFlowNodeAst,
                ControlFlowSemanticAst, ControlPredicateAst,
            };
            let control = crate::model::CompilerControlFlowAst::new(
                ControlFlowSemanticAst::ControlFlow,
                ControlFlowNodeAst::Condition {
                    condition: ControlConditionAst {
                        position: ConditionPositionAst::Postcondition,
                        predicate: ControlPredicateAst::State(predicate),
                        negated_surface: false,
                        provenance: None,
                    },
                    consequence_program: 0,
                    alternative_program: Some(1),
                    reflexive: false,
                },
                vec![
                    crate::model::NestedProgramAst::new(
                        crate::model::NestedProgramKindAst::Consequence,
                        effects,
                    ),
                    crate::model::NestedProgramAst::new(
                        crate::model::NestedProgramKindAst::Alternative,
                        vec![false_effect.take().expect("otherwise branch effect")],
                    ),
                ],
                None,
            )
            .map_err(|error| {
                CardTextError::InvariantViolation(format!(
                    "invalid trailing otherwise control flow: {error:?}"
                ))
            })?;
            EffectAst::ControlFlow(Box::new(control))
        }
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
    if let EffectAst::Conditionals(ConditionalEffectAst::Conditional { if_false, .. }) =
        &mut conditional
    {
        *if_false = vec![false_effect.take().expect("otherwise branch effect")];
    }

    Ok(Some(parsed_triggered_ability(
        trigger,
        vec![conditional],
        vec![Zone::Battlefield],
        None,
        None,
        ReferenceImports::default(),
    )))
}
