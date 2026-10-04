use super::*;
use crate::grammar::blocking_permissions::{BlockingCapacity, parse_blocking_capacity_line};

/// A capacity rule, scoped to its actual source or recipient. Explicit
/// temporary rules belong to effect grammar instead.
pub fn parse_blocking_capacity_static_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let Some(shape) = parse_blocking_capacity_line(tokens) else {
        return Ok(None);
    };
    let clause = shape.clause;
    if clause.this_turn || clause.subject_tokens.is_empty() {
        return Ok(None);
    }
    let words = crate::lexer::parser_token_word_refs(clause.subject_tokens);
    if words.iter().any(|word| *word == "target") {
        return Ok(None);
    }
    let permission = StaticAbilityAst::Static(
        if let (BlockingCapacity::Additional(count), Some(tokens)) =
            (clause.capacity, clause.for_each_tokens)
        {
            StaticAbility::can_block_additional_for_each(count, parse_object_filter(tokens, false)?)
        } else {
            match clause.capacity {
                BlockingCapacity::AnyNumber => StaticAbility::can_block_any_number(),
                BlockingCapacity::Additional(count) => {
                    StaticAbility::can_block_additional_creature_each_combat(count as usize)
                }
            }
        },
    );
    let condition = shape
        .condition_tokens
        .map(parse_static_condition_clause)
        .transpose()?;
    let mut result = if let Some(preceding) = shape.preceding_tokens {
        let Some(abilities) = parse_static_ability_ast_line_lexed(preceding)? else {
            return Ok(None);
        };
        abilities
    } else {
        Vec::new()
    };
    if matches!(words.as_slice(), ["enchanted" | "equipped", "creature"]) {
        let mut rule = match clause.capacity {
            BlockingCapacity::AnyNumber => "can block any number of creatures".to_string(),
            BlockingCapacity::Additional(1) => {
                "can block an additional creature each combat".to_string()
            }
            BlockingCapacity::Additional(count) => {
                format!("can block {count} additional creatures each combat")
            }
        };
        if let Some(tokens) = clause.for_each_tokens {
            rule.push_str(&format!(
                " for each {}",
                crate::lexer::render_token_slice(tokens)
            ));
        }
        result.push(StaticAbilityAst::AttachedStaticAbilityGrant {
            ability: Box::new(permission),
            display: format!("{} {rule}", words.join(" ")),
            condition,
        });
        return Ok(Some(result));
    }
    let attached_subject = shape
        .condition_tokens
        .and_then(infer_attached_subject_filter_from_condition_tokens);
    result.push(
        match parse_anthem_subject_with_attached_fallback(
            clause.subject_tokens,
            attached_subject.as_ref(),
        )? {
            AnthemSubjectAst::Source => {
                if let Some(condition) = condition {
                    StaticAbilityAst::ConditionalStaticAbility {
                        ability: Box::new(permission),
                        condition,
                    }
                } else {
                    permission
                }
            }
            AnthemSubjectAst::Filter(filter) => StaticAbilityAst::GrantStaticAbility {
                filter,
                ability: Box::new(permission),
                condition,
            },
        },
    );
    Ok(Some(result))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex_line;

    #[test]
    fn compound_static_capacity_preserves_all_siblings_and_conditions() {
        for (text, count, marker) in [
            (
                "Equipped creature gets +0/+3 and can block an additional creature each combat.",
                2,
                "CanBlockAdditionalCreatureEachCombat",
            ),
            (
                "Enchanted creature gets +2/+2, has vigilance, and can block an additional creature each combat.",
                3,
                "Vigilance",
            ),
            (
                "As long as this creature is monstrous, it has reach and can block an additional ninety-nine creatures each combat.",
                2,
                "SourceIsMonstrous",
            ),
        ] {
            let parsed = parse_blocking_capacity_static_line(&lex_line(text, 0).unwrap())
                .unwrap()
                .expect(text);
            assert_eq!(parsed.len(), count, "{text}: {parsed:?}");
            assert!(format!("{parsed:?}").contains(marker), "{text}: {parsed:?}");
        }
        let tokens = lex_line(
            "Target creature can block any number of creatures this turn.",
            0,
        )
        .unwrap();
        assert!(
            parse_blocking_capacity_static_line(&tokens)
                .unwrap()
                .is_none(),
            "temporary grants must not become permanent statics"
        );
    }
}

#[cfg(test)]
mod counted_capacity_tests {
    use super::*;
    use crate::lexer::lex_line;

    #[test]
    fn counted_capacity_retains_attached_filter_and_scale() {
        let text = "This creature can block an additional creature each combat for each Equipment attached to this creature.";
        let parsed = parse_blocking_capacity_static_line(&lex_line(text, 0).unwrap())
            .unwrap()
            .unwrap();
        let [StaticAbilityAst::Static(ability)] = parsed.as_slice() else {
            panic!("{parsed:?}");
        };
        let ironsmith_core::StaticAbilityPayload::CanBlockAdditionalForEach { additional, filter } =
            &ability.payload
        else {
            panic!("{ability:?}");
        };
        assert_eq!(*additional, 1);
        assert!(filter.subtypes.contains(&crate::types::Subtype::Equipment));
        assert!(filter.attached_to_object.is_some());
    }
}
