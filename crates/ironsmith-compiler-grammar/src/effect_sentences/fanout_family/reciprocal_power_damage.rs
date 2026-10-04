use crate::types::CardType;
use super::*;
use crate::filter::{ObjectRef, TaggedObjectConstraint, TaggedOpbjectRelation};

/// One explicitly targeted creature damages its controller's other creatures,
/// then the captured set damages that original creature. Bind both identities
/// before either phase; no ordinary nearest-pronoun lookup can swap the sets.
pub(super) fn parse(tokens: &[OwnedLexToken]) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = trim_edge_punctuation(tokens);
    if !tokens.first().is_some_and(|token| token.is_word("target")) {
        return Ok(None);
    }
    let Some((source_end, after_header)) = find_token_word_sequence_span(
        &tokens,
        &[
            "deals", "damage", "equal", "to", "its", "power", "to", "each", "other",
        ],
    ) else {
        return Ok(None);
    };
    let Some(then_index) = tokens.iter().position(|token| token.is_word("then")) else {
        return Ok(None);
    };
    if then_index <= after_header {
        return Ok(None);
    }
    let members = trim_edge_punctuation(&tokens[after_header..then_index]);
    if crate::lexer::parser_token_word_refs(&members) != ["creature", "that", "player", "controls"]
    {
        return Ok(None);
    }
    if crate::lexer::parser_token_word_refs(&tokens[then_index + 1..])
        != [
            "each",
            "of",
            "those",
            "creatures",
            "deals",
            "damage",
            "equal",
            "to",
            "its",
            "power",
            "to",
            "that",
            "creature",
        ]
    {
        return Ok(None);
    }
    let target = parse_target_phrase(&tokens[..source_end])?;
    let TargetAst::Object(filter, _, _) = &target else {
        return Ok(None);
    };
    if !filter.card_types.contains(&CardType::Creature)
        && !filter.all_card_types.contains(&CardType::Creature)
    {
        return Ok(None);
    }
    let source_tag = crate::util::helper_tag_for_tokens(&tokens, "reciprocal_damage_source");
    let others_tag = crate::util::helper_tag_for_tokens(&tokens, "reciprocal_damage_others");
    let declare = EffectAst::TagReferenced {
        effect: Box::new(EffectAst::subject_verb_target_only(target)),
        tag: source_tag.clone(),
    };
    let mut others = ObjectFilter::creature();
    others.zone = Some(Zone::Battlefield);
    others.controller = Some(PlayerFilter::AliasedControllerOf(ObjectRef::Tagged(
        source_tag.clone().into(),
    )));
    others.tagged_constraints.push(TaggedObjectConstraint {
        tag: source_tag.clone().into(),
        relation: TaggedOpbjectRelation::IsNotTaggedObject,
    });
    let capture = EffectAst::subject_verb_tag_matching_objects(
        others,
        vec![Zone::Battlefield],
        others_tag.clone(),
    );
    let mut recipients = ObjectFilter::default();
    recipients.zone = Some(Zone::Battlefield);
    recipients.tagged_constraints.push(TaggedObjectConstraint {
        tag: others_tag.clone().into(),
        relation: TaggedOpbjectRelation::SameObjectId,
    });
    recipients.set_plural_object_noun_surface(true);
    recipients.set_set_quantifier_surface(Some(ironsmith_core::SetQuantifierSurface::Each));
    let first = EffectAst::subject_verb_damage_with_source(
        TargetAst::Tagged(source_tag.clone(), None),
        Value::SourcePower,
        TargetAst::Object(recipients, None, None),
    );
    // The second recipient is the exact original permanent. Its current type
    // is not a new restriction: the noun refers to its earlier creature role.
    let mut original = ObjectFilter::default();
    original.zone = Some(Zone::Battlefield);
    original.tagged_constraints.push(TaggedObjectConstraint {
        tag: source_tag.into(),
        relation: TaggedOpbjectRelation::SameObjectId,
    });
    original.set_singular_pronoun_reference_surface(true);
    let second = EffectAst::subject_verb(
        crate::cards::builders::SubjectVerbRoleAst::Actor,
        PlayerAst::Implicit,
        SubjectVerbActionAst::Damage(DamageActionAst::DealDamageBySources {
            sources: vec![TargetAst::Tagged(others_tag, None)],
            source_binding: ironsmith_core::DamageSourceSetBinding::CapturedIncarnations,
            amount: Value::SourcePower,
            target: TargetAst::Object(original, None, None),
        }),
    );
    Ok(Some(vec![EffectAst::CommaThen {
        effects: vec![declare, capture, first, second],
    }]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reciprocal_power_damage_captures_both_sets_before_its_two_damage_phases() {
        let text = "Target creature an opponent controls deals damage equal to its power to each other creature that player controls, then each of those creatures deals damage equal to its power to that creature.";
        let tokens = crate::lexer::lex_line(text, 0).unwrap();
        let (result, loss) = ironsmith_compiler::parse_loss::capture(|| {
            crate::effect_sentences::parse_effect_sentence_lexed(&tokens)
        });
        let effects = result.unwrap();
        assert!(!loss.is_lossy(), "{}", loss.reasons_text());
        let [EffectAst::CommaThen { effects }] = effects.as_slice() else {
            panic!("{effects:#?}")
        };
        assert_eq!(effects.len(), 4);
        assert!(matches!(
            &effects[3],
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::Damage(DamageActionAst::DealDamageBySources {
                    source_binding: ironsmith_core::DamageSourceSetBinding::CapturedIncarnations,
                    ..
                }),
                ..
            })
        ));
        for changed in [
            text.replace("that player controls", "you control"),
            text.replace(
                "its power to that creature",
                "its toughness to that creature",
            ),
            text.replace("each of those creatures", "each creature"),
        ] {
            let tokens = crate::lexer::lex_line(&changed, 0).unwrap();
            assert!(
                parse(&tokens).unwrap().is_none(),
                "different set/axis/destination needs its own owner"
            );
        }
    }
}
