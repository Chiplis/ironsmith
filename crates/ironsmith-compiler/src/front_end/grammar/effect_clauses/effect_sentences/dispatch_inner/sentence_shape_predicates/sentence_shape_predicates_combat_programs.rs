use super::*;


/// Split an explicit no-combat-damage action from a preceding action whose
/// object filter may itself contain an authored `and` list.
///
/// The general semantic chain splitter deliberately preserves conjunctions
/// inside target phrases. That is normally correct, but a complete trailing
/// `this creature assigns no combat damage this turn` clause is independently
/// grammar-proven and must not be absorbed into a broad destroy target. Both
/// arms still have to lower on their own before this route claims the line.
pub(super) fn parse_explicit_assign_no_combat_damage_followup(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if let Some(prefix) = split_leading_result_prefix_lexed(tokens)
        && let Some(effects) =
            parse_explicit_assign_no_combat_damage_followup(prefix.trailing_tokens)?
    {
        return Ok(Some(vec![match prefix.kind {
            LeadingResultPrefixKind::If => EffectAst::IfResult {
                predicate: prefix.predicate,
                effects,
            },
            LeadingResultPrefixKind::When => EffectAst::WhenResult {
                predicate: prefix.predicate,
                effects,
            },
        }]));
    }

    for (idx, token) in tokens.iter().enumerate() {
        if !token.is_word("and") {
            continue;
        }
        let first = trim_edge_punctuation(&tokens[..idx]);
        let second = trim_edge_punctuation(&tokens[idx + 1..]);
        if first.is_empty()
            || second.is_empty()
            || !matches!(
                effect_grammar::clause_dispatch_shapes::parse_assigns_no_combat_damage_shape(
                    &second,
                ),
                Some(
                    effect_grammar::clause_dispatch_shapes::AssignsNoCombatDamageShape::Supported { .. }
                )
            )
        {
            continue;
        }

        let mut effects = parse_effect_sentence_lexed(&first)?;
        let mut followup = parse_effect_sentence_lexed(&second)?;
        if effects.is_empty() || followup.is_empty() {
            continue;
        }
        effects.append(&mut followup);
        return Ok(Some(vec![EffectAst::Coordinated {
            effects,
            leading_duration: false,
            result_conjunction: false,
        }]));
    }
    Ok(None)
}


pub(super) fn parse_required_damage_fanout(tokens: &[OwnedLexToken]) -> Result<Vec<EffectAst>, CardTextError> {
    super::super::fanout_family::parse_compound_damage_fanout_sentence(tokens)?.ok_or_else(|| {
        CardTextError::ParseError("conditional damage fanout body was not recognized".to_string())
    })
}


pub(super) fn restore_authored_damage_source_surface(
    effects: &mut [EffectAst],
    surface: &crate::target::SourceReferenceSurface,
) {
    fn apply(target: &mut TargetAst, surface: &crate::target::SourceReferenceSurface) {
        match target {
            TargetAst::Source(span) => {
                *target = TargetAst::Object(
                    ObjectFilter::source_with_surface(surface.clone()),
                    None,
                    *span,
                );
            }
            TargetAst::Object(filter, _, _) if filter.source => {
                filter.source_surface = Some(surface.clone());
            }
            _ => {}
        }
    }

    for effect in effects {
        if let EffectAst::SubjectVerb(SubjectVerbEffectAst { action, .. }) = effect {
            match action {
                SubjectVerbActionAst::DealDamageEqualToPower { source, .. }
                | SubjectVerbActionAst::DealDistributedDamage { source, .. } => {
                    apply(source, surface);
                }
                _ => {}
            }
        }
        crate::model::visit::for_each_nested_effects_mut(effect, true, |nested| {
            restore_authored_damage_source_surface(nested, surface);
        });
    }
}


pub(super) fn parse_attacking_doesnt_tap_if_source_untapped(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let action_tokens = crate::token_primitives::strip_leading_if_you_do_lexed(tokens);
    let wrapped_if_result = action_tokens.len() < tokens.len();
    let action_tokens = trim_commas(action_tokens);
    let Some(shape) =
        sentence_shapes::parse_attacking_doesnt_tap_if_source_untapped_tokens(&action_tokens)
    else {
        return Ok(None);
    };
    let filter = parse_object_filter(shape.affected_tokens, false)?;
    let effects = vec![
        EffectAst::subject_verb_grant_abilities_all_dynamically_with_condition(
            filter,
            vec![crate::cards::builders::GrantedAbilityAst::KeywordAction(
                Box::new(crate::payload::KeywordAction::Vigilance),
            )],
            Until::EndOfCombat,
            crate::ConditionExpr::SourceIsUntapped,
        ),
    ];
    if wrapped_if_result {
        return Ok(Some(vec![EffectAst::IfResult {
            predicate: crate::cards::builders::IfResultPredicate::Did,
            effects,
        }]));
    }
    Ok(Some(effects))
}


pub(super) fn rebind_plural_create_followup_damage_source(effects: &mut [EffectAst]) {
    for index in 1..effects.len() {
        let previous_creates_more_than_one = matches!(
            &effects[index - 1],
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::CreateTokenWithMods { count, .. },
                ..
            }) if !matches!(count.unhinted(), Value::Fixed(1))
        );
        if !previous_creates_more_than_one {
            continue;
        }
        let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::DealDamageEqualToPower { source, .. },
            ..
        }) = &mut effects[index]
        else {
            continue;
        };
        let TargetAst::Tagged(tag, span) = source else {
            continue;
        };
        if tag.as_str() == crate::cards::builders::IT_TAG {
            // A singular `it` cannot denote a plural token result. Preserve
            // the authored pronoun span while binding the damage producer to
            // the ability's source instead of the last created token.
            *source = TargetAst::Source(*span);
        }
    }

    for effect in effects {
        crate::model::visit::for_each_nested_effects_mut(effect, true, |nested| {
            rebind_plural_create_followup_damage_source(nested);
        });
    }
}
