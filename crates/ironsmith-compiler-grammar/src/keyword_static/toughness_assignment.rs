use super::*;
use crate::grammar::effects::toughness_assignment as shape;

pub(super) fn parse_filtered_toughness_assignment_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let Some(verb) = shape::assignment_verb(tokens) else {
        return Ok(None);
    };
    if tokens[..verb]
        .iter()
        .any(|token| token.kind == TokenKind::Period)
    {
        return Ok(None);
    }
    let Some(no_defender) = shape::assignment_body(&tokens[verb + 1..]) else {
        return Ok(None);
    };
    // The exact historical specialist owns its three unqualified forms.
    if parse_creatures_assign_combat_damage_using_toughness_line_lexed(tokens).is_some() {
        return Ok(None);
    }
    if verb == 0
        || tokens[..verb]
            .iter()
            .any(|token| token.is_word("target") || token.is_word("until") || token.is_word("may"))
    {
        return Ok(None);
    }
    let (condition, subject_start) = parse_anthem_prefix_condition(tokens, verb)?;
    let subject_tokens = trim_commas(&tokens[subject_start..verb]);
    // A modifier followed by assignment belongs to the complete anthem
    // reader, which preserves both the P/T modifier and its granted rule.
    if subject_tokens
        .iter()
        .any(|token| token.is_word("get") || token.is_word("gets"))
    {
        return Ok(None);
    }

    let attached = (subject_start > 3 && tokens.first().is_some_and(|token| token.is_word("as")))
        .then(|| {
            let predicate_tokens = &tokens[3..subject_start];
            infer_attached_subject_filter_from_condition_tokens(predicate_tokens).or_else(|| {
                // The possessive noun normalizes to `creatures`, so the
                // ordinary `equipped creature has ...` subject probe does
                // not see this otherwise equivalent attached antecedent.
                attached_axis_condition(predicate_tokens)?;
                let tag = if predicate_tokens.first()?.is_word("equipped") {
                    crate::tag::CompilerReferenceTag::Equipped
                } else {
                    crate::tag::CompilerReferenceTag::Enchanted
                };
                Some(ObjectFilter::tagged(tag.bind()))
            })
        })
        .flatten()
        .or_else(|| infer_attached_subject_filter_from_condition_expr(condition.as_ref()));
    let subject = parse_anthem_subject_with_attached_fallback(&subject_tokens, attached.as_ref())?;
    let lower = |ability: StaticAbility| match &subject {
        AnthemSubjectAst::Source => match &condition {
            Some(condition) => StaticAbilityAst::ConditionalStaticAbility {
                ability: Box::new(StaticAbilityAst::Static(ability)),
                condition: condition.clone(),
            },
            None => StaticAbilityAst::Static(ability),
        },
        AnthemSubjectAst::Filter(filter) => StaticAbilityAst::GrantStaticAbility {
            filter: filter.clone(),
            ability: Box::new(StaticAbilityAst::Static(ability)),
            condition: condition.clone(),
        },
    };
    let mut abilities = vec![lower(
        StaticAbility::this_creature_assigns_combat_damage_using_toughness(),
    )];
    if no_defender {
        abilities.push(lower(StaticAbility::can_attack_as_though_no_defender()));
    }
    if let Some(surface) = match subject_tokens.first().and_then(OwnedLexToken::as_word) {
        Some("each") => Some(ironsmith_core::SetQuantifierSurface::Each),
        Some("all") => Some(ironsmith_core::SetQuantifierSurface::All),
        _ => None,
    } {
        for ability in &mut abilities {
            *ability = StaticAbilityAst::WithSetQuantifierSurface {
                ability: Box::new(ability.clone()),
                surface,
            };
        }
    }
    Ok(Some(abilities))
}

/// A comparison of the attached creature's own current axes is a per-object
/// relation, not a comparison against the granting Equipment's power.
pub(super) fn attached_axis_condition(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let view = AnthemNormalizedWords::new(tokens);
    let words = view.word_refs();
    if !matches!(words.first(), Some(&"equipped" | &"enchanted"))
        || !words.get(1).is_some_and(|word| {
            matches!(
                *word,
                "creature" | "creatures" | "creature's" | "creature’s"
            )
        })
    {
        return None;
    }
    let tail = words.get(2..)?;
    let tail = tail.strip_prefix(&["s"]).unwrap_or(tail);
    let relation = match tail {
        ["toughness", "is", "greater", "than", "its", "power"] => {
            ironsmith_core::PowerToughnessRelation::ToughnessGreaterThanPower
        }
        ["power", "is", "greater", "than", "its", "toughness"] => {
            ironsmith_core::PowerToughnessRelation::PowerGreaterThanToughness
        }
        _ => return None,
    };
    Some(PredicateAst::AttachedToSourceMatches(
        ObjectFilter::creature().with_power_toughness_relation(relation),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn grant(ability: &StaticAbilityAst) -> (&ObjectFilter, &Option<PredicateAst>) {
        match ability {
            StaticAbilityAst::WithSetQuantifierSurface { ability, .. } => grant(ability),
            StaticAbilityAst::GrantStaticAbility {
                filter, condition, ..
            } => (filter, condition),
            _ => panic!("expected a receiver-local grant: {ability:#?}"),
        }
    }
    #[test]
    fn filtered_assignment_reads_each_candidates_axes_and_keeps_both_arcades_permissions() {
        let lex = |text| crate::lexer::lex_line(text, 0).unwrap();
        let result = parse_filtered_toughness_assignment_line(&lex("Each creature you control with toughness greater than its power assigns combat damage equal to its toughness rather than its power.")).unwrap().unwrap();
        let (filter, condition) = grant(&result[0]);
        assert_eq!(filter.controller, Some(PlayerFilter::You));
        assert_eq!(
            filter.power_toughness_relation,
            Some(ironsmith_core::PowerToughnessRelation::ToughnessGreaterThanPower)
        );
        assert!(condition.is_none());
        let result = parse_filtered_toughness_assignment_line(&lex("Each creature you control with defender assigns combat damage equal to its toughness rather than its power and can attack as though it didn't have defender.")).unwrap().unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(grant(&result[0]).0, grant(&result[1]).0);
        assert!(
            grant(&result[0])
                .0
                .static_abilities
                .contains(&crate::static_abilities::StaticAbilityId::Defender)
        );
    }
    #[test]
    fn attached_possessive_condition_and_pronoun_both_bind_the_recipient() {
        let tokens = crate::lexer::lex_line("As long as equipped creature's toughness is greater than its power, it assigns combat damage equal to its toughness rather than its power.", 0).unwrap();
        let result = parse_filtered_toughness_assignment_line(&tokens)
            .unwrap()
            .unwrap();
        let (filter, condition) = grant(&result[0]);
        assert!(
            filter
                .tagged_constraints
                .iter()
                .any(|c| c.tag.as_str() == "equipped")
        );
        assert!(
            matches!(condition, Some(PredicateAst::AttachedToSourceMatches(filter)) if filter.power_toughness_relation == Some(ironsmith_core::PowerToughnessRelation::ToughnessGreaterThanPower))
        );
    }
    #[test]
    fn normalized_attached_ability_condition_keeps_its_pronoun_recipient() {
        let tokens = crate::lexer::lex_line("As long as there is enchanted creature with vigilance, it assigns combat damage equal to its toughness rather than its power.", 0).unwrap();
        let result = parse_filtered_toughness_assignment_line(&tokens)
            .unwrap()
            .unwrap();
        let (filter, _) = grant(&result[0]);
        assert!(
            filter
                .tagged_constraints
                .iter()
                .any(|constraint| constraint.tag.as_str() == "enchanted")
        );
    }
    #[test]
    fn static_assignment_does_not_claim_a_resolving_or_optional_choice() {
        for text in [
            "Until end of turn, target creature assigns combat damage equal to its toughness rather than its power.",
            "Each creature you control may assign combat damage equal to its toughness rather than its power.",
            "Each creature you control assigns combat damage equal to its toughness rather than its power this turn.",
        ] {
            let tokens = crate::lexer::lex_line(text, 0).unwrap();
            assert!(
                parse_filtered_toughness_assignment_line(&tokens)
                    .unwrap()
                    .is_none(),
                "{text}"
            );
        }
    }
    #[test]
    fn compound_assignment_is_owned_as_a_whole_not_a_recovered_no_defender_suffix() {
        let tokens = crate::lexer::lex_line("Each creature you control with defender assigns combat damage equal to its toughness rather than its power and can attack as though it didn't have defender.", 0).unwrap();
        assert!(crate::grammar::anthem_grants::parse_plain_no_defender_shape(&tokens).is_none());
        let (result, loss) =
            crate::parse_loss::capture(|| recognize_static_ability_ast_line_registry(&tokens));
        let ParseOutcome::Match(matched) = result else {
            panic!("compound assignment registry must resolve uniquely: {result:?}");
        };
        assert_eq!(matched.value.len(), 2);
        assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    }
}
