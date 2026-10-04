//! Conditional and value-bound activation prices. The authored value/condition
//! belongs to the modifier source; the priced activation supplies its targets.
use super::*;
use ironsmith_core::ActivatedAbilityCostCondition;

fn words_are(tokens: &[OwnedLexToken], expected: &[&str]) -> bool {
    crate::lexer::parser_token_word_refs(tokens) == expected
}
fn unsupported(tokens: &[OwnedLexToken], part: &str) -> CardTextError {
    CardTextError::ParseError(format!(
        "unsupported typed activated-ability cost {part} (clause: '{}')",
        render_token_slice(tokens)
    ))
}

pub(super) fn parse(tokens: &[OwnedLexToken]) -> Result<Option<StaticAbility>, CardTextError> {
    let sentences = crate::grammar::structure::split_lexed_sentences(tokens);
    let Some(first) = sentences.first() else {
        return Ok(None);
    };
    let first = crate::util::trim_edge_punctuation_tokens(first);
    let Some(cost_index) = first
        .iter()
        .position(|token| token.is_word("cost") || token.is_word("costs"))
    else {
        return Ok(None);
    };
    let subject = &first[..cost_index];
    let subject_words = crate::lexer::parser_token_word_refs(subject);
    let (filter, this_ability, equipment_ability) = if subject_words == ["this", "ability"] {
        (ObjectFilter::source(), true, false)
    } else if subject_words.starts_with(&["activated", "abilities", "of"]) {
        (parse_object_filter(&subject[3..], false)?, false, false)
    } else if matches!(
        subject_words.as_slice(),
        ["enchanted", "artifact's", "activated", "abilities"]
            | ["enchanted", "artifacts", "activated", "abilities"]
    ) {
        let mut reference = subject[..2].to_vec();
        reference[1] = OwnedLexToken::word("artifact", reference[1].span);
        (parse_object_filter(&reference, false)?, false, false)
    } else if matches!(
        subject_words.as_slice(),
        ["this", "equipment's", "equip", "ability"] | ["this", "equipments", "equip", "ability"]
    ) {
        (ObjectFilter::source(), false, true)
    } else {
        return Ok(None);
    };
    let amount_tokens = &first[cost_index + 1..];
    let Some((amount, used)) = parse_cost_modifier_amount(amount_tokens) else {
        return Ok(None);
    };
    let remainder = &amount_tokens[used..];
    let Some(direction) = remainder.first().and_then(OwnedLexToken::as_word) else {
        return Ok(None);
    };
    if !matches!(direction, "less" | "more")
        || remainder.len() < 3
        || !words_are(&remainder[1..3], &["to", "activate"])
    {
        return Ok(None);
    }
    let tail = crate::util::trim_edge_punctuation_tokens(&remainder[3..]);
    // This extension owns dynamic surcharges printed on their own activation.
    // Cross-source/general activation taxes retain their established reader.
    if direction == "more" && !this_ability {
        return Ok(None);
    }
    let extended_subject =
        !this_ability && !subject_words.starts_with(&["activated", "abilities", "of"]);
    let extended_tail = tail
        .first()
        .is_some_and(|token| token.is_any_word(&["if", "where", "during"]));
    if direction == "less" && !extended_subject && !extended_tail {
        return Ok(None);
    }

    let mut minimum = None;
    if sentences.len() > 1 {
        if sentences.len() != 2 {
            return Err(unsupported(tokens, "trailing sentences"));
        }
        let words = crate::lexer::parser_token_word_refs(sentences[1]);
        if words
            != [
                "this", "effect", "can't", "reduce", "the", "mana", "in", "that", "cost", "to",
                "less", "than", "one", "mana",
            ]
            && words
                != [
                    "this", "effect", "cant", "reduce", "the", "mana", "in", "that", "cost", "to",
                    "less", "than", "one", "mana",
                ]
        {
            return Err(unsupported(tokens, "minimum sentence"));
        }
        minimum = Some(1);
    }
    let mut predicate = None;
    let mut target_condition = equipment_ability
        .then_some(ActivatedAbilityCostCondition::EquipAbility { targeting: None });
    let mut multiplier = None;
    let reduction = match amount.unhinted() {
        Value::Fixed(value) if *value >= 0 => *value as u32,
        Value::X => 1,
        _ => return Err(unsupported(tokens, "amount")),
    };
    if tail.first().is_some_and(|token| token.is_word("where")) {
        if !matches!(amount.unhinted(), Value::X) {
            return Err(unsupported(tokens, "unbound amount"));
        }
        multiplier = if words_are(
            tail,
            &[
                "where", "x", "is", "the", "power", "of", "the", "creature", "it", "targets",
            ],
        ) {
            Some(Value::PowerOf(Box::new(ChooseSpec::target(
                ChooseSpec::Object(ObjectFilter::creature()),
            ))))
        } else {
            let (value, loss) = crate::parse_loss::capture(|| parse_value_binding_clause_lexed(tail));
            if loss.is_lossy() { return Err(unsupported(tokens, "where-X value")); }
            value
        };
        if multiplier.is_none() {
            return Err(unsupported(tokens, "where-X value"));
        }
    } else if matches!(amount.unhinted(), Value::X) {
        return Err(unsupported(tokens, "unbound X"));
    } else if tail.first().is_some_and(|token| token.is_word("if")) {
        let condition = &tail[1..];
        if condition.len() > 2 && words_are(&condition[..2], &["it", "targets"]) {
            let target = parse_object_filter(&condition[2..], false)?;
            target_condition = Some(if equipment_ability {
                ActivatedAbilityCostCondition::EquipAbility {
                    targeting: Some(target),
                }
            } else {
                ActivatedAbilityCostCondition::TargetsExactly {
                    count: 1,
                    filter: target,
                }
            });
        } else {
            predicate = Some(parse_static_condition_clause(condition)?);
        }
    } else if words_are(tail, &["during", "your", "turn"]) {
        predicate = Some(PredicateAst::YourTurn);
    } else if tail.len() >= 2 && words_are(&tail[..2], &["for", "each"]) {
        multiplier = parse_dynamic_cost_modifier_value(tail)?;
        if multiplier.is_none() {
            return Err(unsupported(tokens, "for-each value"));
        }
    } else if !tail.is_empty() {
        return Err(unsupported(tokens, "tail"));
    }

    if direction == "more" {
        // The increase representation and scope binding are handled by the
        // same engine total-cost pipeline as fixed activation surcharges.
        return build_increase(
            filter,
            this_ability,
            target_condition,
            reduction,
            multiplier,
            predicate,
            tokens,
        );
    }
    let display_tokens = if predicate.is_some() {
        &first[..cost_index + 1 + used + 3]
    } else {
        tokens
    };
    let mut ability = StaticAbility::reduce_activated_ability_costs_with_display(
        filter,
        reduction,
        minimum,
        render_token_slice(display_tokens)
            .trim()
            .trim_end_matches('.'),
    );
    if let ironsmith_core::StaticAbilityPayload::ActivatedAbilityCostReduction {
        multiplier: slot,
        ..
    } = &mut ability.payload
    {
        *slot = multiplier;
    }
    if let Some(condition) = target_condition {
        ability = ability.with_activated_ability_cost_condition(condition);
    }
    if this_ability {
        ability = ability.with_activated_ability_cost_condition(
            ActivatedAbilityCostCondition::ThisAbility {
                ability_index: None,
            },
        );
    }
    if let Some(condition) = predicate {
        ability = ability.with_condition(condition);
    }
    Ok(Some(ability))
}

fn build_increase(
    filter: ObjectFilter,
    this_ability: bool,
    target_condition: Option<ActivatedAbilityCostCondition>,
    amount: u32,
    multiplier: Option<Value>,
    predicate: Option<PredicateAst>,
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let value = match multiplier {
        Some(value) => scale_dynamic_cost_modifier_value(value, amount as i32),
        None => Value::Fixed(amount as i32),
    };
    let cost = crate::model::CompilerCost::DynamicMana(
        ironsmith_core::DynamicManaCost::generic_equal_to(value),
    );
    let mut ability = StaticAbility::increase_activated_ability_costs(
        filter,
        ironsmith_core::TotalCost::from_costs(vec![cost]),
    );
    if let ironsmith_core::StaticAbilityPayload::ActivatedAbilityCostIncrease { display, .. } =
        &mut ability.payload
    {
        *display = Some(
            render_token_slice(tokens)
                .trim()
                .trim_end_matches('.')
                .into(),
        );
    }
    if let Some(condition) = target_condition {
        ability = ability.with_activated_ability_cost_condition(condition);
    }
    if this_ability {
        ability = ability.with_activated_ability_cost_condition(
            ActivatedAbilityCostCondition::ThisAbility {
                ability_index: None,
            },
        );
    }
    if let Some(condition) = predicate {
        ability = ability.with_condition(condition);
    }
    Ok(Some(ability))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn read(text: &str) -> StaticAbility {
        parse(&crate::lexer::lex_line(text, 0).unwrap())
            .unwrap()
            .unwrap()
    }
    #[test]
    fn predicates_values_and_source_scopes_are_typed() {
        for text in [
            "This ability costs {2} less to activate if you control a legendary creature.",
            "This ability costs {3} less to activate if there are five or more mana values among cards in your graveyard.",
            "This ability costs {1} less to activate during your turn.",
            "This ability costs {4} less to activate if an opponent controls four or more nonbasic lands.",
            "This ability costs {3} less to activate if you attacked with a Spacecraft this turn.",
        ] {
            let debug = format!("{:?}", read(text));
            assert!(
                debug.contains("Conditional") && debug.contains("ThisAbility"),
                "{text}: {debug}"
            );
        }
        for text in [
            "Activated abilities of creatures you control cost {X} less to activate, where X is this creature's power. This effect can't reduce the mana in that cost to less than one mana.",
            "This ability costs {X} less to activate, where X is the greatest power among Wurms you control.",
            "This ability costs {X} less to activate, where X is the power of the creature it targets.",
            "This ability costs {X} less to activate, where X is the number of differently named lands you control.",
        ] {
            let ability = read(text);
            assert!(
                matches!(
                    ability.payload,
                    ironsmith_core::StaticAbilityPayload::ActivatedAbilityCostReduction {
                        multiplier: Some(_),
                        ..
                    }
                ),
                "{text}: {ability:?}"
            );
        }
        let ability =
            read("This ability costs {2} less to activate if it targets a colorless creature.");
        assert!(matches!(
            ability.payload,
            ironsmith_core::StaticAbilityPayload::ActivatedAbilityCostReduction {
                condition: Some(ActivatedAbilityCostCondition::All(_)),
                ..
            }
        ));
    }
    #[test]
    fn new_readings_consume_the_complete_supported_surface() {
        for text in [
            "This ability costs {X} less to activate, where X is an unknown amount.",
            "This ability costs {2} less to activate if elephants dance.",
            "This ability costs {X} less to activate, where X is the greatest power among Wurms you control banana.",
        ] {
            assert!(
                parse(&crate::lexer::lex_line(text, 0).unwrap()).is_err(),
                "{text}"
            );
        }
    }
}
