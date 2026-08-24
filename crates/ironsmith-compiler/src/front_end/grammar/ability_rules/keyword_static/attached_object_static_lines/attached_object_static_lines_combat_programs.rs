use super::*;


pub fn lower_remove_counter_prevention_spec(
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
    let mut lowered = if spec.one_damage_per_counter {
        StaticAbility::prevent_one_damage_to_self_per_removed_counter(spec.counter_type)
    } else {
        StaticAbility::prevent_damage_to_self_remove_counter_with_follow_up(
            spec.counter_type,
            amount,
            follow_up,
        )
    };
    if spec.separate_removal_sentence {
        lowered = lowered.with_separate_counter_removal_sentence();
    }
    let ability = StaticAbilityAst::Static(lowered);
    Ok(if let Some(condition_tokens) = spec.condition_tokens {
        StaticAbilityAst::ConditionalStaticAbility {
            ability: Box::new(ability),
            condition: parse_static_condition_clause(condition_tokens)?,
        }
    } else {
        ability
    })
}


pub fn parse_prevent_damage_to_source_put_counters_line(
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
                    crate::lexer::token_word_refs(display_prefix_tokens).join(" ");
                let effect =
                    crate::lexer::token_word_refs(effect_tokens).join(" ");
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


pub fn parse_attached_prevent_all_damage_dealt_by_attached_line(
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


pub fn parse_attached_prevent_all_damage_dealt_to_and_by_attached_line(
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


pub fn parse_attached_prevent_all_combat_damage_dealt_by_attached_line(
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


pub fn parse_attached_prevent_all_damage_dealt_to_attached_line(
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
