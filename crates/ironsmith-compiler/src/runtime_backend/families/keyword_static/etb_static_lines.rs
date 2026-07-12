use crate::runtime_backend::grammar::etb_static_lines::{
    self as etb_grammar, EntersTappedWithCountersClause, EntersWithAddedAbilitiesTail,
    EntersWithCounterConditionShape, EntersWithCounterConditionTail,
    EntersWithCounterConditionTailKind, EntersWithCounterKnownForEachKind,
    EntersWithCountersClause, EtbAggregateKind, EtbAggregateValueKind, EtbAmongMetric,
    EtbAsLongAsClause, EtbGraveyardOwner, EtbNumberOffsetOperator, EtbReferenceValueKind,
    EtbSourceStatFallback, EtbSourceStatKind, EtbTaggedManaValueReference, WhereXKnownValue,
    WhereXPlayerMetric,
};
use crate::runtime_backend::grammar::filters::parse_counter_type_from_tokens;
use crate::static_abilities::StaticAbilityId;
use ironsmith_core::ValueSurfaceHint;

const ETB_TRIGGER_INTRO_WORDS: &[&str] = &["if", "when", "whenever", "as"];
const ETB_THIS_WORD: &str = "this";

const ETB_IF_WORD: &str = "if";
const ETB_ARTICLE_WORDS: &[&str] = &["a", "an"];
const ETB_ONE_WORD: &str = "one";
const ETB_ADDITIONAL_WORD: &str = "additional";
const ETB_COUNTER_OR_COUNTERS_WORDS: &[&str] = &["counter", "counters"];
const ETB_SOURCE_TAIL_HEAD_WORDS: &[&str] = &["this", "thiss"];
const ETB_SOURCE_TAIL_NOUN_WORDS: &[&str] = &["source", "spell", "card", "creature", "permanent"];
const ETB_CONTROL_OWN_WORDS: &[&str] = &["control", "controls", "own", "owns"];
const ETB_EQUAL_WORD: &str = "equal";
const ETB_AND_WORD: &str = "and";
const ETB_AND_OR_WORDS: &[&str] = &["and", "or"];

fn etb_word_is_any(word: &str, expected: &[&str]) -> bool {
    expected.contains(&word)
}

fn etb_word_is(word: &str, expected: &str) -> bool {
    word == expected
}

fn etb_token_word_is_any(token: &OwnedLexToken, expected: &[&str]) -> bool {
    token
        .as_word()
        .is_some_and(|word| etb_word_is_any(word, expected))
}

fn etb_token_word_is(token: &OwnedLexToken, expected: &str) -> bool {
    token
        .as_word()
        .is_some_and(|word| etb_word_is(word, expected))
}

#[derive(Debug, Clone, PartialEq)]
struct EntersWithCounterKnownForEachTail {
    value: Value,
    scale_by_base_count: bool,
}

#[derive(Debug, Clone, PartialEq)]
enum EntersWithCounterPlusTail {
    Supported(Value),
    Unsupported,
}

fn is_etb_source_reference_clause(clause: LexedClause<'_>) -> bool {
    etb_grammar::parse_etb_source_reference_tokens(clause.tokens()).is_some()
}

fn starts_with_etb_source_reference(tokens: &[OwnedLexToken]) -> bool {
    is_etb_source_reference_clause(LexedClause::new(tokens))
}

fn etb_starts_with_trigger_intro_after_label(tokens: &[OwnedLexToken]) -> bool {
    let Some((_, body_tokens)) = split_em_dash_label_prefix(tokens) else {
        return false;
    };
    etb_grammar::parse_etb_trigger_intro_prefix_tokens(body_tokens).is_some()
}

pub(crate) fn parse_enters_tapped_with_counters_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    if tokens.is_empty() {
        return Ok(None);
    }
    if etb_starts_with_trigger_intro_after_label(tokens) {
        return Ok(None);
    }
    if let Some(shape) = etb_grammar::parse_enters_with_dual_for_each_counter_tokens(tokens) {
        return Ok(Some(vec![StaticAbility::enters_with_counters_value(
            shape.counter_type,
            shape.count,
        )]));
    }

    let Some(captured) = etb_grammar::parse_enters_tapped_with_counters_clause_tokens(tokens)
    else {
        return Ok(None);
    };
    let _subject_tokens = captured.subject_tokens;
    let _entry_modifier_tokens = captured.entry_modifier_tokens;
    let _counter_clause_tokens = captured.counter_clause_tokens;

    let mut counter_line_tokens = Vec::new();
    counter_line_tokens.extend_from_slice(captured.subject_tokens);
    counter_line_tokens.extend_from_slice(captured.action_tokens);
    counter_line_tokens.extend_from_slice(captured.with_tokens);
    counter_line_tokens.extend_from_slice(captured.counter_clause_tokens);

    let Some(counters) = parse_enters_with_counter_line(&counter_line_tokens)? else {
        return Ok(None);
    };

    let mut abilities = vec![StaticAbility::enters_tapped_ability()];
    abilities.extend(counters);

    Ok(Some(abilities))
}

pub(crate) fn parse_enters_with_counters_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let full_words = crate::runtime_backend::lexer::token_word_refs(tokens);
    if etb_starts_with_trigger_intro_after_label(tokens) {
        return Ok(None);
    }
    let mut condition: Option<(crate::ConditionExpr, String)> = None;
    let mut clause_tokens: Vec<OwnedLexToken> = tokens.to_vec();

    // Support leading conditional form:
    // "If <condition>, it enters with ..."
    if let Some(leading_if) =
        crate::runtime_backend::grammar::static_line_support::parse_leading_if_clause(
            &clause_tokens,
        )
    {
        let condition_tokens = trim_commas(leading_if.condition_tokens);
        if !condition_tokens.is_empty() {
            let Some(parsed) = parse_enters_with_counter_condition_clause(&condition_tokens) else {
                return Ok(None);
            };
            let display =
                crate::runtime_backend::lexer::token_word_refs(&condition_tokens).join(" ");
            condition = Some((parsed, display));
            clause_tokens = trim_commas(leading_if.remainder_tokens);
        }
    }

    let Some(captured) = etb_grammar::parse_enters_with_counters_clause_tokens(&clause_tokens)
    else {
        return Ok(None);
    };
    let _subject_tokens = captured.subject_tokens;
    let _action_tokens = captured.action_tokens;
    if captured.escaped {
        condition = Some((
            crate::ConditionExpr::ThisSpellEscaped,
            "it escaped".to_string(),
        ));
    }

    let mut added_abilities: Vec<Ability> = Vec::new();
    let mut additional_counters: Vec<(CounterType, Value)> = Vec::new();
    let mut after_with = captured.counter_clause_tokens;
    if let Some(and_with) =
        crate::runtime_backend::grammar::static_line_support::parse_and_with_delimiter(after_with)
    {
        let ability_prefix = trim_commas(&after_with[..and_with.delimiter.start]);
        if let Some(abilities) = parse_enters_with_added_abilities_prefix(&ability_prefix) {
            added_abilities.extend(abilities);
            after_with = &after_with[and_with.delimiter.end..];
        }
    }

    if let Some(choice) = etb_grammar::parse_enters_with_counter_choice_tokens(after_with) {
        if condition.is_some() || !added_abilities.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "unsupported conditional self ETB counter choice clause (clause: '{}')",
                full_words.join(" ")
            )));
        }
        return Ok(Some(vec![StaticAbility::enters_with_counter_choice(
            choice.counter_types,
            choice.count,
        )]));
    }

    let (mut count, used) = if after_with
        .first()
        .is_some_and(|token| etb_token_word_is_any(token, ETB_ARTICLE_WORDS))
        && after_with
            .get(1)
            .is_some_and(|token| etb_token_word_is(token, ETB_ADDITIONAL_WORD))
    {
        if let Some((value, value_used)) = parse_value(&after_with[2..]) {
            (value, 2 + value_used)
        } else {
            (Value::Fixed(1), 2)
        }
    } else {
        parse_value(after_with).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing counter count in self ETB counters (clause: '{}')",
                full_words.join(" ")
            ))
        })?
    };

    let counter_type = parse_counter_type_from_tokens(&after_with[used..]).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported counter type for self ETB counters (clause: '{}')",
            full_words.join(" ")
        ))
    })?;

    let counter_idx =
        crate::runtime_backend::grammar::static_line_support::parse_counter_keyword(after_with)
            .map(|counter| counter.index)
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing counter keyword for self ETB counters (clause: '{}')",
                    full_words.join(" ")
                ))
            })?;
    let mut tail = &after_with[counter_idx + 1..];
    if token_slice_first_is(tail, "on") {
        tail = &tail[1..];
    }
    if token_slice_first_is(tail, "it") {
        tail = &tail[1..];
    } else if tail
        .first()
        .is_some_and(|token| etb_token_word_is_any(token, ETB_SOURCE_TAIL_HEAD_WORDS))
    {
        tail = &tail[1..];
        if let Some(word) = tail.first().and_then(OwnedLexToken::as_word)
            && (etb_word_is_any(word, ETB_SOURCE_TAIL_NOUN_WORDS)
                || parse_card_type(word).is_some())
        {
            tail = &tail[1..];
        }
    }
    let tail = trim_commas(tail);
    let tail_facts =
        crate::runtime_backend::grammar::static_line_support::parse_counter_tail_facts(&tail);
    if tail_facts.has_words {
        let scaled_for_each_count = |dynamic: Value, base_count: &Value| match base_count {
            Value::Fixed(multiplier) => scale_dynamic_cost_modifier_value(dynamic, *multiplier),
            _ => dynamic,
        };
        if let Some(abilities) = parse_enters_with_added_abilities_tail(&tail) {
            added_abilities = abilities;
        } else if let Some(sibling_counters) =
            parse_enters_with_counter_conjunction_tail_tokens(&tail)
        {
            additional_counters = sibling_counters;
        } else if let Some(condition_tail) =
            etb_grammar::parse_enters_with_counter_condition_tail_tokens(&tail)
        {
            let parsed =
                parse_enters_with_counter_condition_clause(condition_tail.condition_tokens)
                    .ok_or_else(|| {
                        CardTextError::ParseError(format!(
                            "unsupported enters-with-counter condition (clause: '{}')",
                            full_words.join(" ")
                        ))
                    })?;
            match condition_tail.kind {
                EntersWithCounterConditionTailKind::If => {
                    let display = crate::runtime_backend::lexer::token_word_refs(
                        condition_tail.condition_tokens,
                    )
                    .join(" ");
                    condition = Some(combine_enters_with_counter_conditions(
                        condition,
                        (parsed, display),
                    ));
                }
                EntersWithCounterConditionTailKind::Unless => {
                    let display = parse_unless_enters_with_counter_condition_display(
                        condition_tail.condition_tokens,
                    )
                    .unwrap_or_else(|| {
                        format!(
                            "not {}",
                            crate::runtime_backend::lexer::token_word_refs(
                                condition_tail.condition_tokens,
                            )
                            .join(" ")
                        )
                    });
                    condition = Some(combine_enters_with_counter_conditions(
                        condition,
                        (crate::ConditionExpr::Not(Box::new(parsed)), display),
                    ));
                }
            }
        } else if let Some(plus_tail) = parse_enters_with_counter_plus_tail_tokens(&tail)? {
            match plus_tail {
                EntersWithCounterPlusTail::Supported(extra) => {
                    count = Value::Add(Box::new(count), Box::new(extra));
                }
                EntersWithCounterPlusTail::Unsupported => {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported plus-self ETB counter clause (clause: '{}')",
                        full_words.join(" ")
                    )));
                }
            }
        } else if let Some(known_tail) = parse_enters_with_counter_known_for_each_tail_tokens(&tail)
        {
            count = if known_tail.scale_by_base_count {
                scaled_for_each_count(known_tail.value, &count)
            } else {
                known_tail.value
            };
        } else if let Some(dynamic) = parse_enters_with_counter_for_each_tail_tokens(&tail)? {
            count = dynamic;
        } else if matches!(
            tail_facts.prefix,
            crate::runtime_backend::grammar::static_line_support::CounterTailPrefix::ForEach
        ) {
            return Err(CardTextError::ParseError(format!(
                "unsupported for-each self ETB counter clause (clause: '{}')",
                full_words.join(" ")
            )));
        } else if let Some(dynamic) = parse_enters_with_counter_equal_to_tail_tokens(&tail) {
            count = dynamic;
        } else if matches!(
            tail_facts.prefix,
            crate::runtime_backend::grammar::static_line_support::CounterTailPrefix::EqualTo
        ) {
            return Err(CardTextError::ParseError(format!(
                "unsupported equal-to self ETB counter clause (clause: '{}')",
                full_words.join(" ")
            )));
        } else {
            count = parse_enters_with_fallback_counter_value(&tail).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported trailing self ETB counter clause (clause: '{}')",
                    full_words.join(" ")
                ))
            })?;
        }
    }

    if let Some((condition, display)) = condition {
        let mut abilities = vec![
            StaticAbility::enters_with_counters_and_abilities_if_condition(
                counter_type,
                count,
                condition.clone(),
                display.clone(),
                added_abilities,
            ),
        ];
        for (counter_type, count) in additional_counters {
            abilities.push(
                StaticAbility::enters_with_counters_and_abilities_if_condition(
                    counter_type,
                    count,
                    condition.clone(),
                    display.clone(),
                    Vec::new(),
                ),
            );
        }
        return Ok(Some(abilities));
    }

    if !additional_counters.is_empty() {
        let mut abilities = vec![StaticAbility::enters_with_counters_value(
            counter_type,
            count,
        )];
        for (counter_type, count) in additional_counters {
            abilities.push(StaticAbility::enters_with_counters_value(
                counter_type,
                count,
            ));
        }
        return Ok(Some(abilities));
    }

    if !added_abilities.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "self ETB counter granted abilities require a condition (clause: '{}')",
            full_words.join(" ")
        )));
    }

    Ok(Some(vec![StaticAbility::enters_with_counters_value(
        counter_type,
        count,
    )]))
}

fn parse_enters_with_counter_conjunction_tail_tokens(
    tokens: &[OwnedLexToken],
) -> Option<Vec<(CounterType, Value)>> {
    let mut rest = trim_commas(tokens);
    let mut counters = Vec::new();

    while rest
        .first()
        .is_some_and(|token| etb_token_word_is(token, ETB_AND_WORD))
    {
        rest = trim_commas(&rest[1..]);
        let (count, used) = if rest
            .first()
            .is_some_and(|token| etb_token_word_is_any(token, ETB_ARTICLE_WORDS))
        {
            (Value::Fixed(1), 1)
        } else {
            parse_value(&rest)?
        };
        let counter_idx = crate::runtime_backend::grammar::primitives::locate_token_index(
            &rest[used..],
            |token| etb_token_word_is_any(token, ETB_COUNTER_OR_COUNTERS_WORDS),
        )? + used;
        let counter_type = parse_counter_type_from_tokens(&rest[used..=counter_idx])?;
        counters.push((counter_type, count));

        rest = trim_commas(&rest[counter_idx + 1..]);
        if token_slice_first_is(&rest, "on") {
            rest = trim_commas(&rest[1..]);
        }
        if token_slice_first_is(&rest, "it") {
            rest = trim_commas(&rest[1..]);
        }
    }

    (!counters.is_empty() && !rest.iter().any(|token| token.as_word().is_some()))
        .then_some(counters)
}

fn parse_enters_with_added_abilities_tail(tokens: &[OwnedLexToken]) -> Option<Vec<Ability>> {
    let tail = trim_commas(tokens);
    let parsed = etb_grammar::parse_enters_with_added_abilities_tail_tokens(&tail)?;
    let ability_tokens = match parsed {
        EntersWithAddedAbilitiesTail::CanAttackAsThoughNoDefender => {
            return Some(vec![Ability::static_ability(
                StaticAbility::can_attack_as_though_no_defender(),
            )]);
        }
        EntersWithAddedAbilitiesTail::AbilityTokens(tokens) => tokens,
    };

    let actions = parse_ability_line(ability_tokens)?;
    let mut abilities = Vec::new();
    for action in actions {
        let static_ability =
            super::static_ability_helpers::static_ability_for_keyword_action(action)?;
        abilities.push(Ability::static_ability(static_ability));
    }
    (!abilities.is_empty()).then_some(abilities)
}

fn parse_enters_with_counter_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    parse_enters_with_counters_line(tokens)
}

fn parse_enters_with_added_abilities_prefix(tokens: &[OwnedLexToken]) -> Option<Vec<Ability>> {
    let actions = parse_ability_line(tokens)?;
    let mut abilities = Vec::new();
    for action in actions {
        let static_ability =
            super::static_ability_helpers::static_ability_for_keyword_action(action)?;
        abilities.push(Ability::static_ability(static_ability));
    }
    (!abilities.is_empty()).then_some(abilities)
}

fn parse_enters_with_counter_known_for_each_tail_tokens(
    tokens: &[OwnedLexToken],
) -> Option<EntersWithCounterKnownForEachTail> {
    match etb_grammar::parse_enters_with_counter_known_for_each_tail_tokens(tokens)? {
        EntersWithCounterKnownForEachKind::CreaturesDiedThisTurn => {
            Some(EntersWithCounterKnownForEachTail {
                value: Value::CreaturesDiedThisTurn,
                scale_by_base_count: true,
            })
        }
        EntersWithCounterKnownForEachKind::ColorsOfManaSpent => {
            Some(EntersWithCounterKnownForEachTail {
                value: Value::ColorsOfManaSpentToCastThisSpell,
                scale_by_base_count: true,
            })
        }
        EntersWithCounterKnownForEachKind::ControlledCreaturesDiedThisTurn => {
            Some(EntersWithCounterKnownForEachTail {
                value: Value::CreaturesDiedThisTurnControlledBy(PlayerFilter::You),
                scale_by_base_count: true,
            })
        }
        EntersWithCounterKnownForEachKind::KickCount => Some(EntersWithCounterKnownForEachTail {
            value: Value::KickCount,
            scale_by_base_count: true,
        }),
        EntersWithCounterKnownForEachKind::LoyaltyCountersOnPlaneswalkersYouControl => {
            Some(EntersWithCounterKnownForEachTail {
                value: Value::CountersOn(
                    Box::new(ChooseSpec::All(ObjectFilter::planeswalker().you_control())),
                    Some(CounterType::Loyalty),
                ),
                scale_by_base_count: true,
            })
        }
        EntersWithCounterKnownForEachKind::MagicGamesLost => {
            Some(EntersWithCounterKnownForEachTail {
                value: Value::MagicGamesLostToOpponentsSinceLastWin,
                scale_by_base_count: false,
            })
        }
    }
}

fn parse_enters_with_counter_for_each_tail_tokens(
    tokens: &[OwnedLexToken],
) -> Result<Option<Value>, CardTextError> {
    let for_each_tokens =
        if etb_grammar::parse_enters_with_counter_for_each_payload_tokens(tokens).is_some() {
            tokens
        } else if let Some(etb_grammar::EntersWithCounterPlusTail::ForEach(for_each_tokens)) =
            etb_grammar::parse_enters_with_counter_plus_tail_tokens(tokens)
        {
            for_each_tokens
        } else {
            return Ok(None);
        };

    parse_dynamic_cost_modifier_value(for_each_tokens)
}

fn parse_enters_with_counter_equal_to_tail_tokens(tokens: &[OwnedLexToken]) -> Option<Value> {
    etb_grammar::parse_enters_with_counter_equal_to_body_tokens(tokens)?;

    parse_enters_with_counter_equal_to_value_clause(tokens)
}

fn parse_enters_with_counter_plus_tail_tokens(
    tokens: &[OwnedLexToken],
) -> Result<Option<EntersWithCounterPlusTail>, CardTextError> {
    let Some(shape) = etb_grammar::parse_enters_with_counter_plus_tail_tokens(tokens) else {
        return Ok(None);
    };
    match shape {
        etb_grammar::EntersWithCounterPlusTail::Unsupported => {
            Ok(Some(EntersWithCounterPlusTail::Unsupported))
        }
        etb_grammar::EntersWithCounterPlusTail::ForEach(for_each_tokens) => {
            let extra = parse_dynamic_cost_modifier_value(for_each_tokens)?;
            Ok(extra.map(EntersWithCounterPlusTail::Supported))
        }
    }
}

fn combine_enters_with_counter_conditions(
    existing: Option<(crate::ConditionExpr, String)>,
    next: (crate::ConditionExpr, String),
) -> (crate::ConditionExpr, String) {
    match existing {
        Some((existing_condition, existing_display)) => {
            let combined_condition =
                crate::ConditionExpr::And(Box::new(existing_condition), Box::new(next.0));
            let combined_display =
                match (existing_display.trim().is_empty(), next.1.trim().is_empty()) {
                    (true, true) => String::new(),
                    (false, true) => existing_display,
                    (true, false) => next.1,
                    (false, false) => format!("{} and {}", existing_display.trim(), next.1.trim()),
                };
            (combined_condition, combined_display)
        }
        None => next,
    }
}

fn parse_enters_with_counter_colors_mana_spent_condition_tokens(
    tokens: &[OwnedLexToken],
) -> Option<u32> {
    let EntersWithCounterConditionShape::ColorsOfManaSpent(amount_tokens) =
        etb_grammar::parse_enters_with_counter_condition_shape_tokens(tokens)?
    else {
        return None;
    };

    let (comparison, used) =
        parse_quantity_comparison_prefix(amount_tokens, false, false, "enters-with condition")
            .ok()?;
    if used != amount_tokens.len() {
        return None;
    }
    crate::runtime_backend::util::comparison_to_strict_at_least_threshold(&comparison)
}

fn parse_enters_with_counter_you_cast_spells_this_turn_condition_tokens(
    tokens: &[OwnedLexToken],
) -> Option<u32> {
    let EntersWithCounterConditionShape::YouCastSpellsThisTurn(amount_tokens) =
        etb_grammar::parse_enters_with_counter_condition_shape_tokens(tokens)?
    else {
        return None;
    };

    let (comparison, used) =
        parse_quantity_comparison_prefix(amount_tokens, false, false, "enters-with condition")
            .ok()?;
    if used != amount_tokens.len() {
        return None;
    }
    crate::runtime_backend::util::comparison_to_strict_at_least_threshold(&comparison)
}

fn parse_enters_with_counter_x_value_threshold_condition_tokens(
    tokens: &[OwnedLexToken],
) -> Option<u32> {
    let EntersWithCounterConditionShape::XValueAtLeast(amount_tokens) =
        etb_grammar::parse_enters_with_counter_condition_shape_tokens(tokens)?
    else {
        return None;
    };
    let (comparison, used) =
        parse_quantity_comparison_prefix(amount_tokens, false, false, "enters-with condition")
            .ok()?;
    if used != amount_tokens.len() {
        return None;
    }
    crate::runtime_backend::util::comparison_to_strict_at_least_threshold(&comparison)
}

fn parse_unless_enters_with_counter_condition_display(tokens: &[OwnedLexToken]) -> Option<String> {
    if let Some(amount) = parse_enters_with_counter_colors_mana_spent_condition_tokens(tokens) {
        return Some(format!(
            "fewer than {amount} colors of mana were spent to cast it"
        ));
    }
    None
}

fn parse_enters_with_counter_condition_clause(
    tokens: &[OwnedLexToken],
) -> Option<crate::ConditionExpr> {
    let condition_tokens = trim_edge_punctuation(tokens);
    if condition_tokens.is_empty() {
        return None;
    }

    if let Some(shape) =
        etb_grammar::parse_enters_with_counter_condition_shape_tokens(&condition_tokens)
    {
        match shape {
            EntersWithCounterConditionShape::AttackedThisTurn => {
                return Some(crate::ConditionExpr::AttackedThisTurn);
            }
            EntersWithCounterConditionShape::SourceWasCast => {
                return Some(crate::ConditionExpr::SourceWasCast);
            }
            EntersWithCounterConditionShape::ThisSpellWasKicked => {
                return Some(crate::ConditionExpr::ThisSpellWasKicked);
            }
            EntersWithCounterConditionShape::ThisSpellEscaped => {
                return Some(crate::ConditionExpr::ThisSpellEscaped);
            }
            EntersWithCounterConditionShape::CreatureDiedThisTurn => {
                return Some(crate::ConditionExpr::CreatureDiedThisTurn);
            }
            EntersWithCounterConditionShape::OpponentLostLifeThisTurn => {
                return Some(crate::ConditionExpr::OpponentLostLifeThisTurn);
            }
            EntersWithCounterConditionShape::PermanentLeftUnderYourControl => {
                return Some(
                    crate::ConditionExpr::PermanentLeftBattlefieldUnderYourControlThisTurn,
                );
            }
            EntersWithCounterConditionShape::NotCastOrNoManaSpent => {
                return Some(crate::ConditionExpr::Or(
                    Box::new(crate::ConditionExpr::Not(Box::new(
                        crate::ConditionExpr::SourceWasCast,
                    ))),
                    Box::new(crate::ConditionExpr::Not(Box::new(
                        crate::ConditionExpr::ManaSpentToCastThisSpellAtLeast {
                            amount: 1,
                            symbol: None,
                        },
                    ))),
                ));
            }
            EntersWithCounterConditionShape::XValueAtLeast(_)
            | EntersWithCounterConditionShape::YouCastSpellsThisTurn(_)
            | EntersWithCounterConditionShape::ColorsOfManaSpent(_) => {}
        }
    }

    if let Some(amount) =
        parse_enters_with_counter_x_value_threshold_condition_tokens(&condition_tokens)
    {
        return Some(crate::ConditionExpr::XValueAtLeast(amount));
    }

    if let Some(amount) =
        parse_enters_with_counter_you_cast_spells_this_turn_condition_tokens(&condition_tokens)
    {
        return Some(crate::ConditionExpr::PlayerCastSpellsThisTurnOrMore {
            player: PlayerFilter::You,
            count: amount,
        });
    }

    if let Some(amount) =
        parse_enters_with_counter_colors_mana_spent_condition_tokens(&condition_tokens)
    {
        return Some(crate::ConditionExpr::ColorsOfManaSpentToCastThisSpellOrMore(amount));
    }

    if let Some(amount) =
        crate::runtime_backend::grammar::filters::parse_same_color_mana_spent_to_cast_predicate(
            &condition_tokens,
        )
    {
        return Some(crate::ConditionExpr::SameColorManaSpentToCastThisSpellAtLeast(amount));
    }

    parse_static_condition_clause(&condition_tokens).ok()
}

fn parse_enters_with_counter_object_filter_tokens(
    subject_tokens: &[OwnedLexToken],
) -> Option<ObjectFilter> {
    parse_object_filter(subject_tokens, false).ok()
}

fn parse_enters_with_counter_equal_to_value_clause(tokens: &[OwnedLexToken]) -> Option<Value> {
    let trimmed = trim_edge_punctuation(tokens);
    let value_body = parse_equal_to_value_body_clause(&trimmed)?;
    if let Some(value) = parse_equal_to_mana_spent_to_cast_value(&trimmed) {
        return Some(value);
    }

    let mut where_tokens = Vec::with_capacity(trimmed.len() + 1);
    where_tokens.push(OwnedLexToken::word(
        "where".to_string(),
        TextSpan::synthetic(),
    ));
    where_tokens.push(OwnedLexToken::word("x".to_string(), TextSpan::synthetic()));
    where_tokens.push(OwnedLexToken::word("is".to_string(), TextSpan::synthetic()));
    where_tokens.extend_from_slice(value_body);

    parse_value_binding_clause(&where_tokens)
        .or_else(|| parse_equal_to_greatest_cards_drawn_this_turn_value(&trimmed))
        .or_else(|| parse_add_mana_equal_amount_value(&trimmed))
        .or_else(|| parse_equal_to_aggregate_filter_value(&trimmed))
        .or_else(|| parse_equal_to_number_of_filter_plus_or_minus_fixed_value(&trimmed))
        .or_else(|| parse_equal_to_number_of_filter_value(&trimmed))
        .or_else(|| parse_equal_to_number_of_opponents_you_have_value(&trimmed))
        .or_else(|| parse_equal_to_number_of_counters_on_reference_value(&trimmed))
        .map(|value| {
            value
                .into_unhinted()
                .with_surface_hint(ValueSurfaceHint::EqualTo)
        })
}

fn parse_equal_to_value_body_clause(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    etb_grammar::parse_equal_to_value_body_tokens(tokens)
}

fn parse_equal_to_mana_spent_to_cast_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    if etb_grammar::parse_equal_to_mana_spent_to_cast_tokens(tokens) {
        Some(Value::ManaSpentToCastThisSpell.with_surface_hint(ValueSurfaceHint::EqualTo))
    } else {
        None
    }
}

fn parse_equal_to_greatest_cards_drawn_this_turn_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    if etb_grammar::parse_equal_to_greatest_cards_drawn_this_turn_tokens(tokens) {
        Some(Value::MaxCardsDrawnThisTurn(PlayerFilter::Opponent))
    } else {
        None
    }
}

pub(crate) fn parse_value_binding_clause(tokens: &[OwnedLexToken]) -> Option<Value> {
    if !etb_grammar::parse_where_x_prefix_tokens(tokens) {
        return None;
    }
    let clause = LexedClause::new(tokens);
    let word_view = crate::runtime_backend::grammar::primitives::TokenWordView::new(tokens);
    let words = word_view.word_refs();

    if let Some(value) = parse_where_x_source_stat_value(tokens) {
        return Some(value);
    }

    if let Some(value) =
        crate::runtime_backend::front_end::grammar::values::parse_players_who_control_more_than_you_value_lexed(tokens)
    {
        return Some(value);
    }

    if let Some(value) = parse_where_x_life_gained_this_turn_value(tokens) {
        return Some(value);
    }

    if let Some(value) = parse_where_x_life_lost_this_turn_value(tokens) {
        return Some(value);
    }

    if let Some(value) = parse_where_x_opponents_dealt_combat_damage_this_turn_value(tokens) {
        return Some(value);
    }

    if let Some(value) = parse_where_x_noncombat_damage_to_opponents_value(tokens) {
        return Some(value);
    }

    if let Some(value) = etb_grammar::parse_where_x_known_value_tokens(tokens) {
        return Some(match value {
            WhereXKnownValue::ThisAbilityResolvedThisTurnCount => {
                Value::ThisAbilityResolvedThisTurnCount
            }
            WhereXKnownValue::YourLifeTotal => Value::LifeTotal(PlayerFilter::You),
            WhereXKnownValue::HalfYourLifeTotalRoundedUp => {
                Value::HalfLifeTotalRoundedUp(PlayerFilter::You)
            }
            WhereXKnownValue::HalfYourLifeTotalRoundedDown => {
                Value::HalfLifeTotalRoundedDown(PlayerFilter::You)
            }
            WhereXKnownValue::YourSpeed => Value::Speed(PlayerFilter::You),
            WhereXKnownValue::EventDamageAmount => Value::EventValue(EventValueSpec::Amount),
            WhereXKnownValue::OpponentCount => Value::CountPlayers(PlayerFilter::Opponent),
            WhereXKnownValue::PlayersBeingAttacked => Value::PlayersBeingAttacked,
            WhereXKnownValue::TargetPlayerLifeTotal | WhereXKnownValue::ThatPlayerLifeTotal => {
                Value::LifeTotal(PlayerFilter::target_player())
            }
            WhereXKnownValue::TargetPlayersLifeTotalDifference => {
                Value::LifeTotalDifference(PlayerFilter::target_player())
            }
            WhereXKnownValue::ThatPlayerSpeed => Value::Speed(PlayerFilter::target_player()),
            WhereXKnownValue::DiscardedCardManaValue => {
                Value::ManaValueOf(Box::new(ChooseSpec::Tagged(TagKey::from("discarded_cost"))))
            }
            WhereXKnownValue::RevealedCardsTotalManaValue => {
                Value::TotalManaValue(ObjectFilter::tagged(TagKey::from("__public_revealed")))
            }
            WhereXKnownValue::DraftNotedHighestNumber { card_name_tokens } => {
                Value::DraftNotedHighestNumber {
                    card_name: parser_token_word_refs(card_name_tokens).join(" "),
                }
                .with_surface_hint(ValueSurfaceHint::WhereXIs)
            }
        });
    }

    if let Some(value) = parse_where_x_is_aggregate_filter_value(tokens) {
        return Some(value);
    }

    // where X is your devotion to black
    if etb_grammar::etb_tokens_have_devotion_value_marker(tokens) {
        if let Ok(Some(value)) = parse_devotion_value_from_add_clause(tokens) {
            return Some(value);
        }
    }

    // where X is the total number of cards in all players' hands
    if etb_grammar::etb_tokens_have_all_players_hand_count_value(tokens) {
        let mut filter = ObjectFilter::default();
        filter.zone = Some(Zone::Hand);
        return Some(Value::Count(filter));
    }

    if clause.after_words(3).is_some_and(|tail| {
        etb_grammar::parse_same_name_as_triggering_spell_graveyard_value_tokens(tail.tokens())
    }) {
        return Some(Value::Count(
            ObjectFilter::default()
                .in_zone(Zone::Graveyard)
                .match_tagged(
                    TagKey::from("triggering"),
                    crate::filter::TaggedOpbjectRelation::SameNameAsTagged,
                ),
        ));
    }

    // where X is N plus the number of <objects>
    if let Some(value) = parse_where_x_is_fixed_plus_number_of_filter_value(tokens) {
        return Some(value);
    }

    // where X is N plus the sacrificed creature's mana value / power / toughness
    if let Some(value) = parse_where_x_is_fixed_plus_reference_value(tokens) {
        return Some(value);
    }

    // where X is the number of <objects> plus/minus N
    if let Some(value) = parse_where_x_is_number_of_filter_plus_or_minus_fixed_value(tokens) {
        return Some(value);
    }

    if let Some(reference) = clause
        .after_words(3)
        .and_then(|tail| etb_grammar::parse_tagged_mana_value_reference_tokens(tail.tokens()))
    {
        let tag = match reference {
            EtbTaggedManaValueReference::ExiledCard => IT_TAG,
            EtbTaggedManaValueReference::TriggeringSpell => "triggering",
        };
        return Some(Value::ManaValueOf(Box::new(ChooseSpec::Tagged(
            TagKey::from(tag),
        ))));
    }

    // where X is the number of cards in your hand
    if etb_grammar::etb_tokens_have_your_hand_count_value(tokens) {
        return Some(Value::CardsInHand(PlayerFilter::You));
    }

    // where X is the number of creatures in your party
    if etb_grammar::etb_tokens_have_your_party_size_value(tokens) {
        return Some(Value::PartySize(PlayerFilter::You));
    }

    // where X is the number of differently named <objects>
    if let Some(value) = parse_where_x_is_number_of_differently_named_filter_value(tokens) {
        return Some(value);
    }

    // where X is the number of different powers among <objects>
    if let Some(value) = parse_where_x_is_number_of_different_powers_filter_value(tokens) {
        return Some(value);
    }

    // where X is the greatest number of <objects> <player> controls
    if let Some(value) = parse_where_x_is_greatest_number_of_filter_value(tokens) {
        return Some(value);
    }

    // where X is the number of counters on that creature
    if let Some(tail) = clause.after_words(3) {
        let mut equal_prefixed = Vec::with_capacity(tail.tokens().len() + 2);
        equal_prefixed.push(OwnedLexToken::word(
            "equal".to_string(),
            TextSpan::synthetic(),
        ));
        equal_prefixed.push(OwnedLexToken::word("to".to_string(), TextSpan::synthetic()));
        equal_prefixed.extend(tail.tokens().iter().cloned());
        if let Some(value) = parse_equal_to_number_of_counters_on_reference_value(&equal_prefixed) {
            return Some(value);
        }
    }

    // where X is the number of <objects>
    if let Some(value) = parse_where_x_is_number_of_filter_value(tokens) {
        return Some(value);
    }

    if let Some(tail) = words.get(3..)
        && let Some((value, used)) = parse_value_expr_words(tail)
        && used == tail.len()
    {
        return Some(value);
    }

    None
}

pub(crate) fn parse_value_binding_clause_lexed(
    tokens: &[crate::runtime_backend::lexer::OwnedLexToken],
) -> Option<Value> {
    parse_value_binding_clause(tokens)
}

pub(crate) fn parse_where_x_source_stat_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    let parsed = etb_grammar::parse_where_x_source_stat_tokens(tokens)?;
    let reference_words =
        crate::runtime_backend::lexer::parser_token_word_refs(parsed.reference_tokens);
    if let Some(surface) = source_reference_surface_for_possessive_words(&reference_words) {
        let source =
            ChooseSpec::Source.with_surface_hint(ChooseSpecSurfaceHint::SourceReference(surface));
        return match parsed.kind {
            EtbSourceStatKind::Power => Some(Value::PowerOf(Box::new(source))),
            EtbSourceStatKind::Toughness => Some(Value::ToughnessOf(Box::new(source))),
            EtbSourceStatKind::ManaValue => Some(Value::ManaValueOf(Box::new(source))),
        };
    }

    match (parsed.fallback?, parsed.kind) {
        (EtbSourceStatFallback::Source, EtbSourceStatKind::Power) => Some(Value::SourcePower),
        (EtbSourceStatFallback::Source, EtbSourceStatKind::Toughness) => {
            Some(Value::SourceToughness)
        }
        (EtbSourceStatFallback::Source, EtbSourceStatKind::ManaValue) => {
            Some(Value::ManaValueOf(Box::new(ChooseSpec::Source)))
        }
        (EtbSourceStatFallback::TaggedObject, kind) => {
            let tagged = Box::new(ChooseSpec::Tagged(TagKey::from(IT_TAG)));
            match kind {
                EtbSourceStatKind::Power => Some(Value::PowerOf(tagged)),
                EtbSourceStatKind::Toughness => Some(Value::ToughnessOf(tagged)),
                EtbSourceStatKind::ManaValue => Some(Value::ManaValueOf(tagged)),
            }
        }
        (EtbSourceStatFallback::TriggeringSpell, EtbSourceStatKind::ManaValue) => Some(
            Value::ManaValueOf(Box::new(ChooseSpec::Tagged(TagKey::from("triggering")))),
        ),
        (EtbSourceStatFallback::TriggeringSpell, _) => None,
    }
}

fn parse_enters_with_fallback_counter_value(tail: &[OwnedLexToken]) -> Option<Value> {
    parse_value_binding_clause(tail)
        .map(|value| value.with_surface_hint(ValueSurfaceHint::WhereXIs))
}

pub(crate) fn parse_where_x_is_fixed_plus_reference_value(
    tokens: &[OwnedLexToken],
) -> Option<Value> {
    let captured = etb_grammar::parse_where_x_fixed_plus_reference_tokens(tokens)?;
    let (fixed_value, fixed_used) = parse_number(captured.fixed_tokens)?;
    if fixed_used != captured.fixed_tokens.len() {
        return None;
    }
    let fixed_value = fixed_value as i32;
    if fixed_value < 0 {
        return None;
    }

    let reference_value = {
        let it = || ChooseSpec::Tagged(TagKey::from(IT_TAG));
        match captured.reference_kind {
            EtbReferenceValueKind::SacrificedCreaturePower => Value::PowerOf(Box::new(it())),
            EtbReferenceValueKind::SacrificedCreatureToughness => {
                Value::ToughnessOf(Box::new(it()))
            }
            EtbReferenceValueKind::TaggedCreatureManaValue => Value::ManaValueOf(Box::new(it())),
        }
    };

    Some(Value::Add(
        Box::new(Value::Fixed(fixed_value)),
        Box::new(reference_value),
    ))
}

pub(crate) fn parse_where_x_life_gained_this_turn_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    if matches!(
        etb_grammar::parse_where_x_player_metric_tokens(tokens),
        Some(WhereXPlayerMetric::LifeGainedByYouThisTurn)
    ) {
        Some(Value::LifeGainedThisTurn(PlayerFilter::You))
    } else {
        None
    }
}

pub(crate) fn parse_where_x_life_lost_this_turn_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    if matches!(
        etb_grammar::parse_where_x_player_metric_tokens(tokens),
        Some(WhereXPlayerMetric::LifeLostByOpponentsThisTurn)
    ) {
        Some(Value::LifeLostThisTurn(PlayerFilter::Opponent))
    } else {
        None
    }
}

pub(crate) fn parse_where_x_opponents_dealt_combat_damage_this_turn_value(
    tokens: &[OwnedLexToken],
) -> Option<Value> {
    if matches!(
        etb_grammar::parse_where_x_player_metric_tokens(tokens),
        Some(WhereXPlayerMetric::OpponentsDealtCombatDamageThisTurn)
    ) {
        Some(Value::CountPlayers(PlayerFilter::Opponent))
    } else {
        None
    }
}

pub(crate) fn parse_where_x_noncombat_damage_to_opponents_value(
    tokens: &[OwnedLexToken],
) -> Option<Value> {
    if matches!(
        etb_grammar::parse_where_x_player_metric_tokens(tokens),
        Some(WhereXPlayerMetric::NoncombatDamageDealtToOpponentsThisTurn)
    ) {
        Some(Value::NoncombatDamageDealtToPlayersThisTurn(
            PlayerFilter::Opponent,
        ))
    } else {
        None
    }
}

pub(crate) fn parse_where_x_is_aggregate_filter_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    let parsed = etb_grammar::parse_where_x_aggregate_filter_tokens(tokens)?;

    if parsed.aggregate == EtbAggregateKind::Greatest
        && parsed.value_kind == EtbAggregateValueKind::ManaValue
    {
        if let Some(value) =
            parse_where_x_greatest_commander_mana_value_filter(parsed.filter_tokens)
        {
            return Some(value);
        }
    }

    let filter_tokens = parsed.filter_tokens;
    let filter_words = crate::runtime_backend::token_word_refs(filter_tokens);
    let has_and_graveyard = etb_grammar::etb_tokens_have_and_graveyard_marker(filter_tokens);
    let should_try_split = has_and_graveyard
        && filter_words
            .iter()
            .any(|word| etb_word_is_any(word, ETB_CONTROL_OWN_WORDS));
    let mut filter = (if should_try_split {
        let segments =
            crate::runtime_backend::grammar::primitives::split_lexed_slices_on_and(filter_tokens);
        let mut branches = Vec::new();
        for segment in segments {
            let trimmed = trim_commas(segment);
            if trimmed.is_empty() {
                return None;
            }
            branches.push(parse_object_filter_lexed(&trimmed, false).ok()?);
        }
        if branches.len() < 2 {
            return None;
        }
        let mut combined = ObjectFilter::default();
        combined.any_of = branches;
        Some(combined)
    } else {
        None
    })
    .or_else(|| parse_object_filter_lexed(filter_tokens, false).ok())?;

    if etb_grammar::etb_tokens_have_sacrificed_marker(filter_tokens) {
        if matches!(filter.zone, Some(Zone::Battlefield)) {
            filter.zone = None;
        }
        if !filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag.as_str() == IT_TAG
                && matches!(
                    constraint.relation,
                    crate::filter::TaggedOpbjectRelation::IsTaggedObject
                )
        }) {
            filter
                .tagged_constraints
                .push(crate::filter::TaggedObjectConstraint {
                    tag: TagKey::from(IT_TAG),
                    relation: crate::filter::TaggedOpbjectRelation::IsTaggedObject,
                });
        }
    }
    if filter_words
        .iter()
        .any(|word| matches!(*word, "permanent" | "permanents"))
        && filter.card_types.is_empty()
        && filter.all_card_types.is_empty()
    {
        filter.card_types = ObjectFilter::permanent_card().card_types;
    }

    match (parsed.aggregate, parsed.value_kind) {
        (EtbAggregateKind::Total, EtbAggregateValueKind::Power) => Some(Value::TotalPower(filter)),
        (EtbAggregateKind::Total, EtbAggregateValueKind::Toughness) => {
            Some(Value::TotalToughness(filter))
        }
        (EtbAggregateKind::Total, EtbAggregateValueKind::ManaValue) => {
            Some(Value::TotalManaValue(filter))
        }
        (EtbAggregateKind::Greatest, EtbAggregateValueKind::Power) => {
            Some(Value::GreatestPower(filter))
        }
        (EtbAggregateKind::Greatest, EtbAggregateValueKind::Toughness) => {
            Some(Value::GreatestToughness(filter))
        }
        (EtbAggregateKind::Greatest, EtbAggregateValueKind::ManaValue) => {
            Some(Value::GreatestManaValue(filter))
        }
    }
}

pub(crate) fn parse_where_x_greatest_commander_mana_value_filter(
    commander_tokens: &[OwnedLexToken],
) -> Option<Value> {
    if !etb_grammar::parse_commander_battlefield_or_command_zone_tokens(commander_tokens) {
        return None;
    }

    let mut battlefield_commander = ObjectFilter::default();
    battlefield_commander.zone = Some(Zone::Battlefield);
    battlefield_commander.is_commander = true;
    battlefield_commander.owner = Some(PlayerFilter::You);

    let mut command_zone_commander = battlefield_commander.clone();
    command_zone_commander.zone = Some(Zone::Command);

    let mut combined = ObjectFilter::default();
    combined.any_of = vec![battlefield_commander, command_zone_commander];

    Some(Value::GreatestManaValue(combined))
}

pub(crate) fn parse_where_x_is_number_of_differently_named_filter_value(
    tokens: &[OwnedLexToken],
) -> Option<Value> {
    let filter_tokens = etb_grammar::parse_where_x_differently_named_filter_tokens(tokens)?;
    let filter = parse_object_filter_lexed(filter_tokens, false).ok()?;
    Some(Value::DistinctNames(filter))
}

pub(crate) fn parse_where_x_is_number_of_different_powers_filter_value(
    tokens: &[OwnedLexToken],
) -> Option<Value> {
    let filter_tokens = etb_grammar::parse_where_x_different_powers_filter_tokens(tokens)?;
    let filter = parse_object_filter_lexed(filter_tokens, false).ok()?;
    Some(Value::DistinctPowers(filter))
}

pub(crate) fn parse_where_x_is_greatest_number_of_filter_value(
    tokens: &[OwnedLexToken],
) -> Option<Value> {
    let filter_tokens = etb_grammar::parse_where_x_greatest_number_filter_tokens(tokens)?;
    let filter = parse_object_filter_lexed(filter_tokens, false).ok()?;
    filter.controller.as_ref()?;
    Some(Value::GreatestCount(filter))
}

pub(crate) fn parse_where_x_is_number_of_filter_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    let captured = etb_grammar::parse_where_x_number_of_filter_tokens(tokens)?;

    if etb_grammar::etb_tokens_have_common_creature_type_value(tokens) {
        return None;
    }

    let multiplier = captured.multiplier;
    let filter_tokens = captured.filter_tokens;
    if let Some(value) = etb_grammar::parse_number_of_counters_on_source_value_tokens(filter_tokens)
    {
        return Some(value);
    }
    if let Some(value) = parse_static_abilities_among_scope_value(filter_tokens) {
        return Some(scale_where_x_number_value(value, multiplier));
    }
    if let Some(value) = parse_among_types_scope_value(filter_tokens) {
        return Some(scale_where_x_number_value(value, multiplier));
    }
    if let Some(among) = etb_grammar::parse_etb_among_scope_tokens(filter_tokens) {
        let card_types = match among.metric {
            EtbAmongMetric::CardTypesAmongCards
                if etb_grammar::etb_tokens_have_graveyard_marker(among.scope_tokens) =>
            {
                let mut filter = ObjectFilter::default().in_zone(Zone::Graveyard);
                filter.owner =
                    match etb_grammar::parse_etb_graveyard_owner_tokens(among.scope_tokens) {
                        Some(EtbGraveyardOwner::Opponent) => Some(PlayerFilter::Opponent),
                        Some(EtbGraveyardOwner::You) => Some(PlayerFilter::You),
                        None => None,
                    };
                Some(Value::CardTypesAmong(filter))
            }
            EtbAmongMetric::CardTypesAmong => {
                let filter = parse_object_filter_lexed(among.scope_tokens, false).ok()?;
                Some(Value::CardTypesAmong(filter))
            }
            _ => None,
        };
        if let Some(card_types) = card_types {
            return Some(scale_where_x_number_value(card_types, multiplier));
        }
    }
    if let Some(value) = parse_aggregate_scope_value_lexed(filter_tokens) {
        return Some(scale_where_x_number_value(value, multiplier));
    }
    if let Some(kind) = etb_grammar::parse_where_x_special_number_filter_tokens(filter_tokens) {
        let value = match kind {
            etb_grammar::WhereXSpecialNumberFilterKind::CreaturesDiedThisTurn => {
                Value::CreaturesDiedThisTurn
            }
            etb_grammar::WhereXSpecialNumberFilterKind::CommanderCastCount => {
                Value::CommanderCastCount(PlayerFilter::You)
            }
            etb_grammar::WhereXSpecialNumberFilterKind::CreaturesControlledByThosePlayers => {
                let mut filter = ObjectFilter::creature();
                filter.controller = Some(PlayerFilter::target_player());
                Value::Count(filter)
            }
        };
        return Some(scale_where_x_number_value(value, multiplier));
    }
    let filter = parse_object_filter_lexed(filter_tokens, false).ok()?;
    Some(scale_where_x_number_value(Value::Count(filter), multiplier))
}

fn parse_static_abilities_among_scope_value(filter_tokens: &[OwnedLexToken]) -> Option<Value> {
    let parsed = etb_grammar::parse_etb_static_abilities_among_scope_tokens(filter_tokens)?;
    let ability_ids = etb_grammar::parse_etb_static_ability_ids_tokens(parsed.ability_tokens)?;

    let filter = parse_object_filter_lexed(parsed.scope_tokens, false).ok()?;
    Some(Value::StaticAbilitiesAmong {
        filter,
        abilities: ability_ids,
    })
}

fn parse_among_types_scope_value(filter_tokens: &[OwnedLexToken]) -> Option<Value> {
    let parsed = etb_grammar::parse_etb_among_scope_tokens(filter_tokens)?;
    let filter = parse_object_filter_lexed(parsed.scope_tokens, false).ok()?;
    match parsed.metric {
        EtbAmongMetric::BasicLandTypesAmong => Some(Value::BasicLandTypesAmong(filter)),
        EtbAmongMetric::CreatureTypesAmong => Some(Value::CreatureTypesAmong(filter)),
        EtbAmongMetric::ColorsAmong => Some(Value::ColorsAmong(filter)),
        EtbAmongMetric::CardTypesAmongCards | EtbAmongMetric::CardTypesAmong => None,
    }
}

fn scale_where_x_number_value(value: Value, multiplier: i32) -> Value {
    if multiplier == 1 {
        return value;
    }
    match value {
        Value::Count(filter) => Value::CountScaled(filter, multiplier),
        Value::CountScaled(filter, factor) => Value::CountScaled(filter, factor * multiplier),
        other => Value::Scaled(Box::new(other), multiplier),
    }
}

pub(crate) fn parse_where_x_is_fixed_plus_number_of_filter_value(
    tokens: &[OwnedLexToken],
) -> Option<Value> {
    let captured = etb_grammar::parse_where_x_fixed_plus_number_of_filter_tokens(tokens)?;
    let (fixed_value, fixed_used) = parse_number(captured.fixed_tokens)?;
    if fixed_used != captured.fixed_tokens.len() {
        return None;
    }
    let filter_tokens = captured.filter_tokens;
    if let Some(counter_value) =
        etb_grammar::parse_number_of_counters_on_source_value_tokens(filter_tokens)
    {
        return Some(Value::Add(
            Box::new(Value::Fixed(fixed_value as i32)),
            Box::new(counter_value),
        ));
    }
    if let Some(value) = parse_aggregate_scope_value_lexed(filter_tokens) {
        return Some(Value::Add(
            Box::new(Value::Fixed(fixed_value as i32)),
            Box::new(value),
        ));
    }
    let filter = parse_object_filter(filter_tokens, false).ok()?;
    Some(Value::Add(
        Box::new(Value::Fixed(fixed_value as i32)),
        Box::new(Value::Count(filter)),
    ))
}

pub(crate) fn parse_where_x_is_number_of_filter_plus_or_minus_fixed_value(
    tokens: &[OwnedLexToken],
) -> Option<Value> {
    let captured = etb_grammar::parse_where_x_number_of_filter_offset_tokens(tokens)?;
    let filter_tokens = trim_commas(captured.filter_tokens);
    let count_value = if etb_grammar::etb_tokens_have_your_hand_count_value(&filter_tokens) {
        Value::CardsInHand(PlayerFilter::You)
    } else {
        let filter = parse_object_filter(&filter_tokens, false).ok()?;
        Value::Count(filter)
    };

    let offset_tokens = trim_commas(captured.offset_tokens);
    let (offset_value, used) = parse_number(&offset_tokens)?;
    let trailing_words = crate::runtime_backend::token_word_refs(&offset_tokens[used..]);
    if !trailing_words.is_empty() {
        return None;
    }

    let signed_offset = match captured.operator {
        EtbNumberOffsetOperator::Plus => offset_value as i32,
        EtbNumberOffsetOperator::Minus => -(offset_value as i32),
    };
    Some(Value::Add(
        Box::new(count_value),
        Box::new(Value::Fixed(signed_offset)),
    ))
}

pub(crate) fn parse_enters_tapped_for_filter_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if clause_words
        .first()
        .is_some_and(|word| etb_word_is_any(word, ETB_TRIGGER_INTRO_WORDS))
    {
        return Ok(None);
    }
    if etb_starts_with_trigger_intro_after_label(tokens) {
        return Ok(None);
    }
    if is_negated_untap_clause(&clause_words) {
        if etb_grammar::etb_tokens_have_entry_verb(tokens)
            && etb_grammar::etb_tokens_have_tapped_marker(tokens)
        {
            return Err(CardTextError::ParseError(format!(
                "unsupported mixed enters-tapped and negated-untap clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        return Ok(None);
    }
    if etb_grammar::etb_tokens_have_unless_marker(tokens) {
        return Ok(None);
    }
    let Some(entry_clause) = etb_grammar::parse_entry_filter_tokens(tokens) else {
        return Ok(None);
    };
    if !etb_grammar::etb_tokens_have_tapped_marker(entry_clause.tail_tokens) {
        return Ok(None);
    }
    if LexedClause::new(tokens)
        .token(0)
        .is_some_and(|token| etb_token_word_is(token, ETB_THIS_WORD))
    {
        return Ok(None);
    }
    if etb_grammar::etb_tokens_have_copy_marker(tokens) {
        return Err(CardTextError::ParseError(format!(
            "unsupported enters-as-copy replacement clause (clause: '{}') [rule=enters-as-copy]",
            clause_words.join(" ")
        )));
    }
    let before_enter = entry_clause.filter_tokens;
    let before_word_len = LexedClause::new(before_enter).word_len();
    let played_suffix = etb_grammar::parse_etb_played_by_opponent_suffix_tokens(before_enter);
    let controller_override = played_suffix.map(|_| PlayerFilter::Opponent);
    let filter_tokens = played_suffix
        .map(|parsed| parsed.filter_tokens)
        .unwrap_or(before_enter);
    let mut filter = match parse_object_filter(filter_tokens, false) {
        Ok(filter) => filter,
        Err(_) if played_suffix.is_none() && before_word_len > 0 => {
            return Ok(Some(StaticAbility::enters_tapped_ability()));
        }
        Err(err) => return Err(err),
    };
    if controller_override.is_none() && filter.source {
        return Ok(Some(StaticAbility::enters_tapped_ability()));
    }
    if let Some(controller) = controller_override {
        filter.controller = Some(controller);
    }
    Ok(Some(StaticAbility::enters_tapped_for_filter(filter)))
}

pub(crate) fn parse_enters_untapped_for_filter_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if clause_words
        .first()
        .is_some_and(|word| etb_word_is_any(word, ETB_TRIGGER_INTRO_WORDS))
    {
        return Ok(None);
    }
    if etb_starts_with_trigger_intro_after_label(tokens) {
        return Ok(None);
    }
    if etb_grammar::etb_tokens_have_unless_marker(tokens)
        || LexedClause::new(tokens)
            .token(0)
            .is_some_and(|token| etb_token_word_is(token, ETB_THIS_WORD))
    {
        return Ok(None);
    }

    let Some(entry_clause) = etb_grammar::parse_entry_filter_tokens(tokens) else {
        return Ok(None);
    };
    if !etb_grammar::etb_tokens_have_untapped_marker(entry_clause.tail_tokens) {
        return Ok(None);
    }

    let before_enter = entry_clause.filter_tokens;
    if before_enter.is_empty() {
        return Ok(None);
    }
    let filter = parse_object_filter(before_enter, false)?;
    Ok(Some(StaticAbility::enters_untapped_for_filter(filter)))
}

pub(crate) fn parse_as_enters_reveal_from_hand_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let trimmed = trim_edge_punctuation(tokens);
    let Some(parsed) = etb_grammar::parse_as_enters_reveal_from_hand_tokens(&trimmed) else {
        return Ok(None);
    };
    let reveal_filter_tokens = trim_edge_punctuation(parsed.reveal_filter_tokens);
    if reveal_filter_tokens.is_empty() {
        return Ok(None);
    }

    let (count, count_used) = parse_choice_count_token_prefix_consumed(&reveal_filter_tokens)
        .unwrap_or((crate::effect::ChoiceCount::exactly(1), 0));
    let filter_tokens = trim_edge_punctuation(&reveal_filter_tokens[count_used..]);
    if filter_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing reveal filter in as-enters reveal clause (clause: '{}')",
            parser_token_word_refs(tokens).join(" ")
        )));
    }
    let mut filter = parse_object_filter(&filter_tokens, false)?;
    filter.zone = Some(Zone::Hand);

    let source_subject = (!parsed.source_kind_tokens.is_empty())
        .then(|| {
            let words = crate::runtime_backend::token_word_refs(parsed.source_kind_tokens);
            if words.is_empty() {
                "this".to_string()
            } else {
                format!("this {}", words.join(" "))
            }
        })
        .unwrap_or_else(|| "this".to_string());
    let reveal_filter_text = parser_token_word_refs(&reveal_filter_tokens).join(" ");
    Ok(Some(StaticAbility::reveal_from_hand_as_enters(
        filter,
        count,
        true,
        format!("As {source_subject} enters, you may reveal {reveal_filter_text} from your hand"),
    )))
}

pub(crate) fn parse_reveal_from_hand_or_enters_tapped_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if etb_grammar::parse_as_this_land_enters_prefix_tokens(tokens).is_none() {
        return Ok(None);
    }
    if !etb_grammar::etb_tokens_have_reveal_from_hand_marker(tokens) {
        return Ok(None);
    }

    let Some(reveal_filter_tokens) = etb_grammar::parse_reveal_from_hand_filter_tokens(tokens)
    else {
        return Err(CardTextError::ParseError(format!(
            "unsupported reveal source in land ETB reveal clause (clause: '{}')",
            clause_words.join(" ")
        )));
    };
    let reveal_filter_tokens = trim_edge_punctuation(reveal_filter_tokens);
    if reveal_filter_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing reveal filter in land ETB reveal clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    let mut reveal_filter = parse_object_filter(&reveal_filter_tokens, false)?;
    reveal_filter.zone = None;
    let reveal_condition = crate::ConditionExpr::YouHaveCardInHandMatching(reveal_filter);

    // Pattern A: "... If you don't, this land enters tapped."
    if let Some(if_you_dont_tail) = etb_grammar::find_if_you_dont_tail_tokens(tokens) {
        if etb_grammar::parse_land_reveal_trailing_tapped_prefix_tokens(if_you_dont_tail).is_none()
        {
            return Err(CardTextError::ParseError(format!(
                "unsupported land ETB reveal trailing clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        parser_trace("parse_static:land-reveal-or-enter-tapped:matched", tokens);
        return Ok(Some(StaticAbility::enters_tapped_unless_condition(
            reveal_condition,
            clause_words.join(" "),
        )));
    }

    // Pattern B: "... This land enters tapped unless you revealed ... this way or you control ..."
    let condition_clause = etb_grammar::parse_enters_tapped_unless_condition_tokens(tokens);
    if condition_clause.is_none() {
        if etb_grammar::etb_tokens_have_unless_marker(tokens) {
            return Err(CardTextError::ParseError(format!(
                "unsupported land ETB reveal unless-prefix (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        return Ok(None);
    }

    let mut condition = reveal_condition;
    if let Some(condition_clause) = condition_clause
        && etb_grammar::etb_tokens_have_or_marker(condition_clause)
    {
        let Some(parsed_condition) = parse_revealed_this_way_or_control_condition(condition_clause)
        else {
            return Err(CardTextError::ParseError(format!(
                "unsupported control condition in land ETB reveal clause (clause: '{}')",
                clause_words.join(" ")
            )));
        };
        condition = parsed_condition;
    }

    parser_trace("parse_static:land-reveal-or-enter-tapped:matched", tokens);
    Ok(Some(StaticAbility::enters_tapped_unless_condition(
        condition,
        clause_words.join(" "),
    )))
}

fn parse_revealed_this_way_or_control_condition(
    condition_tokens: &[OwnedLexToken],
) -> Option<crate::ConditionExpr> {
    let parsed = etb_grammar::parse_revealed_this_way_or_control_tokens(condition_tokens)?;
    let reveal_filter_tokens = trim_edge_punctuation(parsed.reveal_filter_tokens);
    if reveal_filter_tokens.is_empty() {
        return None;
    }
    let mut reveal_filter = parse_object_filter(&reveal_filter_tokens, false).ok()?;
    reveal_filter.zone = None;

    let control_tokens = trim_edge_punctuation(parsed.control_condition_tokens);
    let control_condition = crate::runtime_backend::grammar::conditions::parse_control_condition(
        &control_tokens,
        crate::runtime_backend::grammar::conditions::ControlConditionOptions {
            allow_that_player: false,
            allow_opponent_players: false,
            allow_defending_player: false,
            bind_filter_controller_to_subject: false,
            allow_different_powers_tail: false,
            default_filter_zone: Some(Zone::Battlefield),
        },
    )?;
    if control_condition.player_filter != Some(PlayerFilter::You)
        || control_condition
            .at_least_count()
            .map_or(true, |count| count > 1)
    {
        return None;
    }

    Some(crate::ConditionExpr::Or(
        Box::new(crate::ConditionExpr::YouHaveCardInHandMatching(
            reveal_filter,
        )),
        Box::new(crate::ConditionExpr::YouControl(control_condition.filter)),
    ))
}

fn captured_enters_tapped_unless_control_quantity_static_ability(
    control_condition: &crate::runtime_backend::grammar::conditions::ControlConditionAst,
) -> Option<StaticAbility> {
    let mut filter = control_condition.filter.clone();
    filter.zone = None;

    let normalize_template = |mut filter: ObjectFilter| {
        filter.zone = None;
        filter
    };
    let other_lands = normalize_template(
        ObjectFilter::land()
            .controlled_by(PlayerFilter::You)
            .other(),
    );
    let basic_lands = normalize_template(
        ObjectFilter::land()
            .controlled_by(PlayerFilter::You)
            .with_supertype(Supertype::Basic),
    );

    match (control_condition.comparison, filter) {
        (crate::effect::Comparison::GreaterThanOrEqual(2), filter) if filter == other_lands => {
            Some(StaticAbility::enters_tapped_unless_control_two_or_more_other_lands())
        }
        (crate::effect::Comparison::LessThanOrEqual(2), filter) if filter == other_lands => {
            Some(StaticAbility::enters_tapped_unless_control_two_or_fewer_other_lands())
        }
        (crate::effect::Comparison::GreaterThanOrEqual(2), filter) if filter == basic_lands => {
            Some(StaticAbility::enters_tapped_unless_control_two_or_more_basic_lands())
        }
        _ => None,
    }
}

fn parse_enters_tapped_unless_control_quantity_static_ability(
    condition_tokens: &[OwnedLexToken],
    display: String,
) -> Option<StaticAbility> {
    let condition_words = crate::runtime_backend::lexer::token_word_refs(condition_tokens);
    let control_condition = crate::runtime_backend::grammar::conditions::parse_control_condition(
        condition_tokens,
        crate::runtime_backend::grammar::conditions::ControlConditionOptions {
            allow_that_player: false,
            allow_opponent_players: false,
            allow_defending_player: false,
            bind_filter_controller_to_subject: true,
            allow_different_powers_tail: false,
            default_filter_zone: Some(Zone::Battlefield),
        },
    )?;
    if control_condition.quantity_token_count == 0 {
        return None;
    }
    if let Some(ability) =
        captured_enters_tapped_unless_control_quantity_static_ability(&control_condition)
    {
        return Some(ability);
    }

    let mut filter = control_condition.filter;
    if filter.zone.is_none() {
        filter.zone = Some(Zone::Battlefield);
    }
    let condition = crate::ConditionExpr::CountComparison {
        count: AnthemCountExpression::MatchingFilter(filter),
        comparison: control_condition.comparison,
        display: Some(condition_words.join(" ")),
    };
    Some(StaticAbility::enters_tapped_unless_condition(
        condition, display,
    ))
}

#[cfg(test)]
mod etb_enters_tapped_with_counters_tests {
    use super::*;

    #[test]
    fn enters_tapped_with_counters_uses_capture_parser() {
        let tokens = crate::runtime_backend::lexer::lex_line(
            "this creature enters tapped with one +1/+1 counter on it.",
            0,
        )
        .expect("lex");

        let captured = etb_grammar::parse_enters_tapped_with_counters_clause_tokens(&tokens)
            .expect("capture parser should recognize tapped-with-counters clause");
        assert_eq!(
            LexedClause::new(captured.subject_tokens).word_refs(),
            ["this", "creature"]
        );
        assert!(
            LexedClause::new(captured.entry_modifier_tokens)
                .word_refs()
                .contains(&"tapped")
        );

        let abilities = parse_enters_tapped_with_counters_line(&tokens)
            .expect("parser should not error")
            .expect("enters tapped with counters should parse");
        let ids = abilities.iter().map(StaticAbility::id).collect::<Vec<_>>();

        assert_eq!(abilities.len(), 2, "expected tapped plus counters");
        assert!(ids.contains(&crate::static_abilities::StaticAbilityId::EntersTapped));
        assert!(
            ids.contains(&crate::static_abilities::StaticAbilityId::EnterWithCounters),
            "expected enters-with-counters ability, got {ids:?}"
        );
    }

    #[test]
    fn enters_with_counters_uses_capture_parser_for_normalized_subjects() {
        let tokens = crate::runtime_backend::lexer::lex_line(
            "This creature enters with one +1/+1 counter on it.",
            0,
        )
        .expect("lex");

        let captured = etb_grammar::parse_enters_with_counters_clause_tokens(&tokens)
            .expect("capture parser should normalize source subject");
        assert_eq!(
            LexedClause::new(captured.subject_tokens).word_refs(),
            ["this", "creature"]
        );

        let abilities = parse_enters_with_counters_line(&tokens)
            .expect("parser should not error")
            .expect("enters with counters should parse");
        assert_eq!(
            abilities[0].id(),
            crate::static_abilities::StaticAbilityId::EnterWithCounters
        );
    }

    #[test]
    fn colors_of_mana_spent_condition_uses_capture_parser() {
        let tokens = crate::runtime_backend::lexer::lex_line(
            "Two or more colors of mana were spent to cast it.",
            0,
        )
        .expect("lex");

        assert_eq!(
            parse_enters_with_counter_colors_mana_spent_condition_tokens(&tokens),
            Some(2)
        );
        assert_eq!(
            parse_unless_enters_with_counter_condition_display(&tokens),
            Some("fewer than 2 colors of mana were spent to cast it".to_string())
        );
        assert!(matches!(
            parse_enters_with_counter_condition_clause(&tokens),
            Some(crate::ConditionExpr::ColorsOfManaSpentToCastThisSpellOrMore(2))
        ));
    }

    #[test]
    fn you_cast_spells_this_turn_condition_uses_capture_parser() {
        for text in [
            "you've cast two or more spells this turn",
            "you have cast three or more spells this turn",
            "you cast four or more spells this turn",
        ] {
            let tokens = crate::runtime_backend::lexer::lex_line(text, 0).expect("lex");
            let expected = if text.contains("three") {
                3
            } else if text.contains("four") {
                4
            } else {
                2
            };

            assert_eq!(
                parse_enters_with_counter_you_cast_spells_this_turn_condition_tokens(&tokens),
                Some(expected),
                "{text}"
            );
            assert!(matches!(
                parse_enters_with_counter_condition_clause(&tokens),
                Some(crate::ConditionExpr::PlayerCastSpellsThisTurnOrMore {
                    player: PlayerFilter::You,
                    count
                }) if count == expected
            ));
        }
    }

    #[test]
    fn x_value_threshold_condition_uses_capture_parser() {
        for (text, expected) in [("X is 5 or more", 5), ("x is five or more", 5)] {
            let tokens = crate::runtime_backend::lexer::lex_line(text, 0).expect("lex");

            assert_eq!(
                parse_enters_with_counter_x_value_threshold_condition_tokens(&tokens),
                Some(expected),
                "{text}"
            );
            assert!(matches!(
                parse_enters_with_counter_condition_clause(&tokens),
                Some(crate::ConditionExpr::XValueAtLeast(amount)) if amount == expected
            ));
        }
    }

    #[test]
    fn plus_for_each_counter_tail_uses_capture_parser() {
        let tokens = crate::runtime_backend::lexer::lex_line(
            "plus an additional +1/+1 counter on it for each other creature you control",
            0,
        )
        .expect("lex");

        let value = parse_enters_with_counter_for_each_tail_tokens(&tokens)
            .expect("tail parser should not error")
            .expect("plus for-each tail should parse");
        let debug = format!("{value:?}");
        assert!(
            debug.contains("other: true") && debug.contains("Creature"),
            "expected other-creature dynamic counter value, got {debug}"
        );
    }

    #[test]
    fn plus_counter_tail_gate_uses_capture_parser() {
        let supported_tokens = crate::runtime_backend::lexer::lex_line(
            "plus an additional +1/+1 counter on it for each other creature you control",
            0,
        )
        .expect("lex");
        let supported = parse_enters_with_counter_plus_tail_tokens(&supported_tokens)
            .expect("plus tail parser should not error")
            .expect("plus tail should be recognized");
        assert!(
            matches!(supported, EntersWithCounterPlusTail::Supported(_)),
            "expected supported plus-for-each tail, got {supported:?}"
        );

        let unsupported_tokens =
            crate::runtime_backend::lexer::lex_line("plus a mystery counter", 0).expect("lex");
        let unsupported = parse_enters_with_counter_plus_tail_tokens(&unsupported_tokens)
            .expect("unsupported plus tail should not hard-error")
            .expect("plus tail should be recognized");
        assert!(matches!(
            unsupported,
            EntersWithCounterPlusTail::Unsupported
        ));

        let unrelated_tokens =
            crate::runtime_backend::lexer::lex_line("for each creature you control", 0)
                .expect("lex");
        assert!(
            parse_enters_with_counter_plus_tail_tokens(&unrelated_tokens)
                .expect("unrelated tail should not error")
                .is_none()
        );
    }

    #[test]
    fn for_each_counter_tail_uses_capture_parser() {
        let tokens =
            crate::runtime_backend::lexer::lex_line("for each creature card in your graveyard", 0)
                .expect("lex");

        let value = parse_enters_with_counter_for_each_tail_tokens(&tokens)
            .expect("tail parser should not error")
            .expect("for-each tail should parse");
        let debug = format!("{value:?}");
        assert!(
            debug.contains("Graveyard") && debug.contains("Creature"),
            "expected creature-card-in-graveyard dynamic value, got {debug}"
        );
    }

    #[test]
    fn equal_to_counter_tail_uses_capture_parser() {
        let tokens = crate::runtime_backend::lexer::lex_line(
            "equal to the number of creature cards in your graveyard",
            0,
        )
        .expect("lex");

        let value = parse_enters_with_counter_equal_to_tail_tokens(&tokens)
            .expect("equal-to tail should parse");
        let debug = format!("{value:?}");
        assert!(
            debug.contains("Graveyard") && debug.contains("Creature"),
            "expected creature-card-in-graveyard equal-to value, got {debug}"
        );
    }

    #[test]
    fn equal_to_mana_spent_value_uses_capture_parser() {
        for text in [
            "equal to the amount of mana spent to cast it",
            "equal to the amount of mana spent to cast this spell",
            "equal to the amount of mana spent to cast spell",
        ] {
            let tokens = crate::runtime_backend::lexer::lex_line(text, 0).expect("lex");
            let value = parse_equal_to_mana_spent_to_cast_value(&tokens)
                .unwrap_or_else(|| panic!("mana-spent value should parse: {text}"));
            let debug = format!("{value:?}");
            assert!(
                debug.contains("ManaSpentToCastThisSpell") && debug.contains("EqualTo"),
                "expected equal-to mana-spent value for {text}, got {debug}"
            );
        }

        let unrelated_tokens = crate::runtime_backend::lexer::lex_line(
            "equal to the amount of mana spent to cast that permanent",
            0,
        )
        .expect("lex");
        assert!(parse_equal_to_mana_spent_to_cast_value(&unrelated_tokens).is_none());
    }

    #[test]
    fn known_for_each_counter_tails_use_capture_parser() {
        let cases = [
            (
                "for each creature that died this turn",
                "CreaturesDiedThisTurn",
                true,
            ),
            (
                "for each color of mana spent to cast it",
                "ColorsOfManaSpentToCastThisSpell",
                true,
            ),
            (
                "for each creature that died under your control this turn",
                "CreaturesDiedThisTurnControlledBy",
                true,
            ),
            ("for each time this spell was kicked", "KickCount", true),
            (
                "for each Magic game you have lost to one of your opponents since you last won a game against them",
                "MagicGamesLostToOpponentsSinceLastWin",
                false,
            ),
        ];

        for (text, expected_debug, expected_scaled) in cases {
            let tokens = crate::runtime_backend::lexer::lex_line(text, 0).expect("lex");
            let parsed = parse_enters_with_counter_known_for_each_tail_tokens(&tokens)
                .unwrap_or_else(|| panic!("known for-each tail should parse: {text}"));
            let debug = format!("{:?}", parsed.value);
            assert!(
                debug.contains(expected_debug),
                "expected {expected_debug} value for {text}, got {debug}"
            );
            assert_eq!(
                parsed.scale_by_base_count, expected_scaled,
                "unexpected scaling flag for {text}"
            );
        }
    }

    #[test]
    fn counter_condition_tail_uses_capture_parser() {
        let if_tokens =
            crate::runtime_backend::lexer::lex_line("if you attacked this turn", 0).expect("lex");
        let if_tail = etb_grammar::parse_enters_with_counter_condition_tail_tokens(&if_tokens)
            .expect("if condition tail should parse");
        assert_eq!(if_tail.kind, EntersWithCounterConditionTailKind::If);
        assert_eq!(
            LexedClause::new(if_tail.condition_tokens).word_refs(),
            ["you", "attacked", "this", "turn"]
        );

        let unless_tokens = crate::runtime_backend::lexer::lex_line(
            "unless two or more colors of mana were spent to cast it",
            0,
        )
        .expect("lex");
        let unless_tail =
            etb_grammar::parse_enters_with_counter_condition_tail_tokens(&unless_tokens)
                .expect("unless condition tail should parse");
        assert_eq!(unless_tail.kind, EntersWithCounterConditionTailKind::Unless);
        assert_eq!(
            parse_unless_enters_with_counter_condition_display(unless_tail.condition_tokens),
            Some("fewer than 2 colors of mana were spent to cast it".to_string())
        );
    }
}

#[cfg(test)]
mod etb_control_quantity_tests {
    use super::*;

    fn parse_control_quantity_condition(text: &str) -> StaticAbility {
        let tokens = crate::runtime_backend::lexer::lex_line(text, 0).expect("lex");
        parse_enters_tapped_unless_control_quantity_static_ability(&tokens, text.to_string())
            .expect("control quantity condition should parse")
    }

    #[test]
    fn enters_tapped_unless_control_quantity_special_cases_use_capture_parser() {
        let cases = [
            (
                "you control two or more other lands",
                crate::static_abilities::StaticAbilityId::EntersTappedUnlessControlTwoOrMoreOtherLands,
            ),
            (
                "you control two or fewer other lands",
                crate::static_abilities::StaticAbilityId::EntersTappedUnlessControlTwoOrFewerOtherLands,
            ),
            (
                "you control two or more basic lands",
                crate::static_abilities::StaticAbilityId::EntersTappedUnlessControlTwoOrMoreBasicLands,
            ),
        ];

        for (text, expected_id) in cases {
            let ability = parse_control_quantity_condition(text);
            assert_eq!(ability.id(), expected_id, "{text}");
        }
    }

    #[test]
    fn enters_tapped_unless_control_quantity_generic_case_keeps_count_condition() {
        let ability = parse_control_quantity_condition("you control three or more artifacts");
        let debug = format!("{ability:?}");

        assert!(
            debug.contains("EntersTappedUnlessCondition"),
            "expected generic conditional ETB ability, got {debug}"
        );
        assert!(debug.contains("CountComparison"), "{debug}");
        assert!(debug.contains("Artifact"), "{debug}");
        assert!(debug.contains("GreaterThanOrEqual(3)"), "{debug}");
    }

    #[test]
    fn reveal_unless_revealed_or_control_disjunction_uses_capture_parser() {
        let reveal_tokens = crate::runtime_backend::lexer::lex_line(
            "As this land enters, you may reveal a Dragon card from your hand.",
            0,
        )
        .expect("lex");
        assert!(
            parse_reveal_from_hand_or_enters_tapped_line(&reveal_tokens)
                .expect("standalone reveal clause should not hard-error")
                .is_none()
        );

        let tapped_tokens = crate::runtime_backend::lexer::lex_line(
            "This land enters tapped unless you revealed a Dragon card this way or you control a Dragon.",
            0,
        )
        .expect("lex");

        let ability = parse_conditional_enters_tapped_unless_line(&tapped_tokens)
            .expect("reveal-or-control clause should parse")
            .expect("expected static ability");
        let debug = format!("{ability:?}");

        assert!(debug.contains("YouHaveCardInHandMatching"), "{debug}");
        assert!(debug.contains("YouControl"), "{debug}");
        assert!(debug.contains("Dragon"), "{debug}");
    }

    #[test]
    fn enters_tapped_unless_opponents_condition_uses_capture_parser() {
        let condition_tokens =
            crate::runtime_backend::lexer::lex_line("you have two or more opponents", 0)
                .expect("lex");
        assert!(
            parse_enters_tapped_unless_two_or_more_opponents_condition(&condition_tokens).is_some()
        );

        let wrong_amount_tokens =
            crate::runtime_backend::lexer::lex_line("you have three or more opponents", 0)
                .expect("lex");
        assert!(
            parse_enters_tapped_unless_two_or_more_opponents_condition(&wrong_amount_tokens)
                .is_none()
        );

        let line_tokens = crate::runtime_backend::lexer::lex_line(
            "This land enters tapped unless you have two or more opponents.",
            0,
        )
        .expect("lex");
        let ability = parse_conditional_enters_tapped_unless_line(&line_tokens)
            .expect("opponents condition should parse")
            .expect("expected static ability");

        assert_eq!(
            ability.id(),
            crate::static_abilities::StaticAbilityId::EntersTappedUnlessTwoOrMoreOpponents
        );
    }

    #[test]
    fn enters_tapped_unless_life_condition_uses_capture_parser() {
        let condition_tokens =
            crate::runtime_backend::lexer::lex_line("a player has 13 or less life", 0)
                .expect("lex");
        assert!(
            parse_enters_tapped_unless_a_player_has_13_or_less_life_condition(&condition_tokens)
                .is_some()
        );

        let wrong_amount_tokens =
            crate::runtime_backend::lexer::lex_line("a player has 12 or less life", 0)
                .expect("lex");
        assert!(
            parse_enters_tapped_unless_a_player_has_13_or_less_life_condition(&wrong_amount_tokens)
                .is_none()
        );

        let line_tokens = crate::runtime_backend::lexer::lex_line(
            "This land enters tapped unless a player has 13 or less life.",
            0,
        )
        .expect("lex");
        let ability = parse_conditional_enters_tapped_unless_line(&line_tokens)
            .expect("life condition should parse")
            .expect("expected static ability");

        assert_eq!(
            ability.id(),
            crate::static_abilities::StaticAbilityId::EntersTappedUnlessAPlayerHas13OrLessLife
        );
    }
}

fn parse_enters_tapped_unless_a_player_has_13_or_less_life_condition(
    condition_tokens: &[OwnedLexToken],
) -> Option<()> {
    let condition = crate::runtime_backend::grammar::conditions::parse_player_life_total_condition(
        condition_tokens,
    )?;
    if condition.player != PlayerFilter::Any {
        return None;
    }
    match condition.comparison {
        crate::effect::Comparison::LessThanOrEqual(13)
        | crate::effect::Comparison::LessThan(14) => Some(()),
        _ => None,
    }
}

fn parse_enters_tapped_unless_two_or_more_opponents_condition(
    condition_tokens: &[OwnedLexToken],
) -> Option<()> {
    let opponent_phrases: &[&[&str]] = &[&["opponents"]];
    let condition =
        crate::runtime_backend::grammar::conditions::parse_player_has_quantity_object_condition(
            condition_tokens,
            opponent_phrases,
            "enters-tapped opponents condition",
        )?;
    if condition.player != PlayerFilter::You {
        return None;
    }
    let count = crate::runtime_backend::util::comparison_to_strict_at_least_threshold(
        &condition.comparison,
    )?;
    if count == 2 { Some(()) } else { None }
}

pub(crate) fn parse_conditional_enters_tapped_unless_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if !etb_grammar::etb_tokens_have_entry_verb(tokens) {
        return Ok(None);
    }
    if !etb_grammar::etb_tokens_have_tapped_marker(tokens)
        || !etb_grammar::etb_tokens_have_unless_marker(tokens)
    {
        return Ok(None);
    }

    let Some(condition_clause) = etb_grammar::parse_enters_tapped_unless_condition_tokens(tokens)
    else {
        return Ok(None);
    };
    let condition_tokens = trim_edge_punctuation(condition_clause);
    if let Some(condition) = parse_revealed_this_way_or_control_condition(&condition_tokens) {
        return Ok(Some(StaticAbility::enters_tapped_unless_condition(
            condition,
            clause_words.join(" "),
        )));
    }
    if let Some(ability) = parse_enters_tapped_unless_control_quantity_static_ability(
        &condition_tokens,
        clause_words.join(" "),
    ) {
        return Ok(Some(ability));
    }
    if parse_enters_tapped_unless_a_player_has_13_or_less_life_condition(&condition_tokens)
        .is_some()
    {
        return Ok(Some(
            StaticAbility::enters_tapped_unless_a_player_has_13_or_less_life(),
        ));
    }
    if parse_enters_tapped_unless_two_or_more_opponents_condition(&condition_tokens).is_some() {
        return Ok(Some(
            StaticAbility::enters_tapped_unless_two_or_more_opponents(),
        ));
    }
    if etb_grammar::parse_first_three_turns_prefix_tokens(&condition_tokens).is_some() {
        return Ok(Some(StaticAbility::enters_tapped_unless_condition(
            crate::ConditionExpr::YourFirstTurnsOfTheGameOrFewer(3),
            clause_words.join(" "),
        )));
    }

    // Generic: "unless you control <object filter>" (covers Mount/Vehicle, etc.).
    if let Some(control_condition) =
        crate::runtime_backend::grammar::conditions::parse_control_condition(
            &condition_tokens,
            crate::runtime_backend::grammar::conditions::ControlConditionOptions {
                allow_that_player: false,
                allow_opponent_players: false,
                allow_defending_player: false,
                bind_filter_controller_to_subject: false,
                allow_different_powers_tail: false,
                default_filter_zone: None,
            },
        )
        && !control_condition.has_explicit_quantity()
    {
        let condition = crate::ConditionExpr::YouControl(control_condition.filter);
        return Ok(Some(StaticAbility::enters_tapped_unless_condition(
            condition,
            clause_words.join(" "),
        )));
    }

    Err(CardTextError::ParseError(format!(
        "unsupported enters tapped unless condition (clause: '{}')",
        clause_words.join(" ")
    )))
}

pub(crate) fn parse_enters_with_additional_counter_for_filter_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if let Some(ability) = parse_spell_cast_enters_with_additional_counter_for_filter_line(tokens)?
    {
        return Ok(Some(ability));
    }

    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if let Some(as_long_as) = etb_grammar::parse_etb_as_long_as_clause_tokens(tokens) {
        match as_long_as {
            EtbAsLongAsClause::ThisInYourGraveyard {
                continuation_tokens,
            } => {
                return parse_enters_with_additional_counter_for_filter_line(continuation_tokens);
            }
            EtbAsLongAsClause::Condition {
                condition_tokens,
                continuation_tokens,
            } => {
                let condition = parse_static_condition_clause(condition_tokens)?;
                let Some(ability) =
                    parse_enters_with_additional_counter_for_filter_line(continuation_tokens)?
                else {
                    return Ok(None);
                };
                return Ok(Some(ability.with_condition(condition)));
            }
        }
    }

    if !etb_grammar::etb_tokens_have_with_additional_counters(tokens) {
        return Ok(None);
    }

    let Some(entry_clause) = etb_grammar::parse_entry_filter_tokens(tokens) else {
        return Ok(None);
    };
    let subject_tokens = trim_commas(entry_clause.filter_tokens);
    if subject_tokens
        .iter()
        .any(|token| token.is_period() || token.is_colon() || token.is_semicolon())
    {
        return Ok(None);
    }

    if subject_tokens.is_empty() {
        return Ok(None);
    }
    if starts_with_etb_source_reference(&subject_tokens) {
        return Ok(None);
    }
    if etb_grammar::parse_etb_trigger_intro_prefix_tokens(&subject_tokens).is_some() {
        return Ok(None);
    }

    let Some(filter) = parse_enters_with_counter_object_filter_tokens(&subject_tokens) else {
        return Ok(None);
    };

    let and_as_idx =
        crate::runtime_backend::lexer::find_token_word_sequence_span(tokens, &["and", "as"])
            .map(|(idx, _)| idx);
    let base_tokens = and_as_idx.map_or(tokens, |idx| &tokens[..idx]);

    let additional_idx =
        crate::runtime_backend::grammar::primitives::locate_token_index(base_tokens, |token| {
            etb_token_word_is(token, ETB_ADDITIONAL_WORD)
        })
        .ok_or_else(|| {
            CardTextError::ParseError("missing 'additional' keyword for ETB counters".to_string())
        })?;
    let count = if let Some(equal_idx) =
        crate::runtime_backend::grammar::primitives::locate_token_index(base_tokens, |token| {
            etb_token_word_is(token, ETB_EQUAL_WORD)
        }) {
        let value_start = equal_idx + 2;
        let value_tokens = trim_commas(base_tokens.get(value_start..).unwrap_or_default());
        parse_value(&value_tokens)
            .map(|(value, _)| value)
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported ETB counter count value (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?
    } else if additional_idx > 0
        && let Some((parsed, _)) = parse_number(&base_tokens[additional_idx - 1..additional_idx])
    {
        Value::Fixed(parsed as i32)
    } else if let Some((parsed, _)) = parse_number(&base_tokens[additional_idx + 1..]) {
        Value::Fixed(parsed as i32)
    } else {
        Value::Fixed(1)
    };

    let counter_type = parse_counter_type_from_tokens(base_tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported counter type for ETB replacement (clause: '{}')",
            clause_words.join(" ")
        ))
    })?;

    let mut added_subtypes = Vec::new();
    if let Some(idx) = and_as_idx {
        let mut addition_tokens = tokens[idx + 1..].to_vec();
        if let Some(first) = addition_tokens.first() {
            addition_tokens[0] = OwnedLexToken::word("is".to_string(), first.span());
        }
        let Some(additions) = parse_type_color_addition_clause(&addition_tokens)? else {
            return Err(CardTextError::ParseError(format!(
                "unsupported ETB type-addition tail (clause: '{}')",
                clause_words.join(" ")
            )));
        };
        if !additions.added_colors.is_empty()
            || !additions.set_colors.is_empty()
            || !additions.card_types.is_empty()
        {
            return Err(CardTextError::ParseError(format!(
                "unsupported non-subtype ETB type addition (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        added_subtypes = additions.subtypes;
    }

    Ok(Some(
        StaticAbility::enters_with_counters_and_subtypes_for_filter(
            filter,
            counter_type,
            count,
            added_subtypes,
        ),
    ))
}

fn parse_spell_cast_enters_with_additional_counter_for_filter_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    let Some(parsed) = etb_grammar::parse_spell_cast_enters_additional_counter_tokens(tokens)
    else {
        return Ok(None);
    };
    let spell_filter_tokens = trim_edge_punctuation(parsed.spell_filter_tokens);
    let condition_tokens = trim_edge_punctuation(parsed.condition_tokens);
    let entry_tokens = trim_edge_punctuation(parsed.entry_tokens);

    let Some(condition) =
        parse_snow_mana_of_any_spell_color_spent_to_cast_it_condition(&condition_tokens)
    else {
        return Ok(None);
    };

    let Some(mut filter) = parse_enters_with_counter_object_filter_tokens(&spell_filter_tokens)
    else {
        return Ok(None);
    };
    if !matches!(filter.zone, Some(Zone::Stack)) {
        return Ok(None);
    }
    filter.zone = Some(Zone::Battlefield);
    filter.stack_kind = None;
    filter.has_mana_cost = false;
    filter.controller = Some(PlayerFilter::You);

    let counter_type = parse_counter_type_from_tokens(&entry_tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported counter type for spell-cast ETB replacement (clause: '{}')",
            words.join(" ")
        ))
    })?;
    let count = parse_additional_counter_count_from_tokens(&entry_tokens);

    Ok(Some(
        StaticAbility::enters_with_counters_and_subtypes_for_filter(
            filter,
            counter_type,
            count,
            Vec::new(),
        )
        .with_condition(condition),
    ))
}

fn parse_snow_mana_of_any_spell_color_spent_to_cast_it_condition(
    tokens: &[OwnedLexToken],
) -> Option<crate::ConditionExpr> {
    etb_grammar::parse_snow_mana_of_spell_color_condition_tokens(tokens)
        .then_some(crate::ConditionExpr::SnowManaOfAnySpellColorSpentToCastThisSpell)
}

fn parse_additional_counter_count_from_tokens(tokens: &[OwnedLexToken]) -> Value {
    let additional_idx =
        crate::runtime_backend::grammar::primitives::locate_token_index(tokens, |token| {
            etb_token_word_is(token, ETB_ADDITIONAL_WORD)
        });
    let Some(additional_idx) = additional_idx else {
        return Value::Fixed(1);
    };

    if additional_idx > 0
        && let Some((parsed, _)) = parse_number(&tokens[additional_idx - 1..additional_idx])
    {
        return Value::Fixed(parsed as i32);
    }
    if let Some((parsed, _)) = parse_number(&tokens[additional_idx + 1..]) {
        return Value::Fixed(parsed as i32);
    }
    Value::Fixed(1)
}

pub(crate) fn parse_as_enters_becomes_characteristics_for_filter_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    let Some(as_enters) = etb_grammar::parse_as_enters_tokens(tokens) else {
        return Ok(None);
    };

    let Some(becomes) =
        etb_grammar::parse_it_becomes_additional_type_tail_tokens(as_enters.tail_tokens)
    else {
        return Ok(None);
    };
    let descriptor_word_view =
        crate::runtime_backend::grammar::primitives::TokenWordView::new(becomes.descriptor_tokens);
    let descriptor_words = descriptor_word_view.word_refs();

    let mut descriptor_idx = 0usize;
    if descriptor_words
        .get(descriptor_idx)
        .is_some_and(|word| etb_word_is_any(word, ETB_ARTICLE_WORDS))
    {
        descriptor_idx += 1;
    }
    let Some(pt_word) = descriptor_words.get(descriptor_idx) else {
        return Ok(None);
    };
    let (power, toughness) = match parse_pt_modifier(pt_word) {
        Ok(parsed) => parsed,
        Err(_) => return Ok(None),
    };
    descriptor_idx += 1;

    if descriptor_idx >= descriptor_words.len() {
        return Ok(None);
    }

    let subject_tokens = trim_commas(as_enters.subject_tokens);
    let filter = parse_object_filter(&subject_tokens, false)?;

    let characteristic_words = &descriptor_words[descriptor_idx..];
    let mut card_types = Vec::new();
    let mut subtypes = Vec::new();
    for descriptor in characteristic_words.iter().copied().filter(|word| {
        !etb_word_is_any(word, ETB_ARTICLE_WORDS) && !etb_word_is(word, ETB_AND_WORD)
    }) {
        if parse_color(descriptor).is_some() {
            return Err(CardTextError::ParseError(format!(
                "unsupported color-changing as-enters characteristic replacement (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        if let Some(card_type) = parse_card_type(descriptor) {
            crate::slice_primitives::push_unique(&mut card_types, card_type);
            continue;
        }
        if let Some(subtype) = parse_subtype_flexible(descriptor) {
            crate::slice_primitives::push_unique(&mut subtypes, subtype);
            continue;
        }
        return Err(CardTextError::ParseError(format!(
            "unsupported as-enters characteristic descriptor '{}' (clause: '{}')",
            descriptor,
            clause_words.join(" ")
        )));
    }

    if card_types.is_empty() && subtypes.is_empty() {
        return Ok(None);
    }

    Ok(Some(StaticAbility::enters_with_characteristics_for_filter(
        filter, card_types, subtypes, power, toughness,
    )))
}

pub(crate) fn parse_as_enters_or_turns_face_up_pt_choice_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    let Some(as_enters) = etb_grammar::parse_as_enters_tokens(tokens) else {
        return Ok(None);
    };

    let subject_clause = LexedClause::new(as_enters.subject_tokens);
    if etb_grammar::parse_etb_self_subject_tokens(as_enters.subject_tokens).is_none() {
        return Ok(None);
    }

    if let Some(choice_tokens) =
        etb_grammar::parse_it_becomes_your_choice_of_prefix_tokens(as_enters.tail_tokens)
    {
        let choice_words = crate::runtime_backend::token_word_refs(choice_tokens);
        let options = parse_pt_choice_characteristic_options(&choice_words, &clause_words)?;
        if options.is_empty() {
            return Ok(None);
        }
        let subject = subject_clause.text();
        let display = format!(
            "As {subject} enters, it becomes your choice of {}",
            render_pt_choice_characteristic_options(&options)
        );
        return Ok(Some(
            StaticAbility::choose_power_toughness_options_as_enters_or_turns_face_up(
                options, display,
            ),
        ));
    }

    let Some(choice_tokens) = etb_grammar::parse_face_up_choice_tail_tokens(as_enters.tail_tokens)
    else {
        return Ok(None);
    };
    let choice_words = crate::runtime_backend::token_word_refs(choice_tokens);
    let [first_word, separator, second_word] = choice_words.as_slice() else {
        return Ok(None);
    };
    if *separator != "or" {
        return Ok(None);
    }

    let first = parse_pt_modifier(first_word).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported power/toughness choice '{}' (clause: '{}')",
            first_word,
            clause_words.join(" ")
        ))
    })?;
    let second = parse_pt_modifier(second_word).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported power/toughness choice '{}' (clause: '{}')",
            second_word,
            clause_words.join(" ")
        ))
    })?;

    let subject = subject_clause.text();
    let display = format!(
        "As {subject} enters or is turned face up, it becomes your choice of {}/{} or {}/{}",
        first.0, first.1, second.0, second.1
    );
    Ok(Some(
        StaticAbility::choose_power_toughness_as_enters_or_turns_face_up(
            vec![first, second],
            display,
        ),
    ))
}

fn parse_pt_choice_characteristic_options(
    words: &[&str],
    clause_words: &[&str],
) -> Result<Vec<PowerToughnessChoiceOption>, CardTextError> {
    let mut options = Vec::new();
    let mut idx = 0usize;
    while idx < words.len() {
        if words[idx] == "or" {
            idx += 1;
        }
        if matches!(words.get(idx).copied(), Some("a" | "an")) {
            idx += 1;
        }
        let Some(pt_word) = words.get(idx).copied() else {
            break;
        };
        let (power, toughness) = match parse_pt_modifier(pt_word) {
            Ok(pt) => pt,
            Err(_) if options.is_empty() => return Ok(Vec::new()),
            Err(_) => {
                return Err(CardTextError::ParseError(format!(
                    "unsupported power/toughness choice '{}' (clause: '{}')",
                    pt_word,
                    clause_words.join(" ")
                )));
            }
        };
        idx += 1;

        if !matches!(
            words.get(idx).copied(),
            Some("creature" | "permanent" | "object")
        ) {
            return Err(CardTextError::ParseError(format!(
                "unsupported power/toughness choice descriptor after '{}' (clause: '{}')",
                pt_word,
                clause_words.join(" ")
            )));
        }
        idx += 1;

        let mut abilities = Vec::new();
        if words.get(idx).copied() == Some("with") {
            idx += 1;
            let ability_start = idx;
            while idx < words.len()
                && words[idx] != "or"
                && !(matches!(words[idx], "a" | "an")
                    && words
                        .get(idx + 1)
                        .is_some_and(|next| parse_pt_modifier(next).is_ok()))
            {
                idx += 1;
            }
            abilities =
                parse_pt_choice_keyword_abilities(&words[ability_start..idx], clause_words)?;
        }

        options.push(PowerToughnessChoiceOption::with_abilities(
            power, toughness, abilities,
        ));
    }

    Ok(options)
}

fn parse_pt_choice_keyword_abilities(
    words: &[&str],
    clause_words: &[&str],
) -> Result<Vec<StaticAbility>, CardTextError> {
    if words.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing keyword ability in power/toughness choice (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let action = etb_grammar::parse_pt_choice_keyword_action_words(words);
    let Some(static_ability) = action.and_then(static_ability_for_keyword_action) else {
        return Err(CardTextError::ParseError(format!(
            "unsupported keyword ability '{}' in power/toughness choice (clause: '{}')",
            words.join(" "),
            clause_words.join(" ")
        )));
    };

    Ok(vec![static_ability])
}

fn render_pt_choice_characteristic_options(options: &[PowerToughnessChoiceOption]) -> String {
    let rendered = options
        .iter()
        .map(|option| {
            let mut text = format!("a {}/{} creature", option.power, option.toughness);
            if !option.abilities.is_empty() {
                let abilities = option
                    .abilities
                    .iter()
                    .map(|ability| ability.display().to_ascii_lowercase())
                    .collect::<Vec<_>>()
                    .join(" and ");
                text.push_str(" with ");
                text.push_str(&abilities);
            }
            text
        })
        .collect::<Vec<_>>();

    match rendered.as_slice() {
        [] => String::new(),
        [only] => only.clone(),
        [first, second] => format!("{first} or {second}"),
        _ => {
            let mut text = rendered[..rendered.len() - 1].join(", ");
            text.push_str(", or ");
            text.push_str(rendered.last().expect("nonempty options"));
            text
        }
    }
}
