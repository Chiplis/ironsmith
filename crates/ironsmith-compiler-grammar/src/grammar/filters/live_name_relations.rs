//! Live comparison-set name predicates, distinct from tagged antecedents and
//! relations among the objects selected together for a cost.
use super::*;

pub(crate) fn parse_live_name_relation(
    tokens: &[OwnedLexToken],
    other: bool,
) -> Option<Result<ObjectFilter, CardTextError>> {
    let (marker, negated, tail) = primitives::find_prefix(tokens, || {
        winnow::combinator::alt((
            primitives::phrase(&["with", "the", "same", "name", "as"]).value(false),
            primitives::any_phrase(&[
                &["that", "doesnt", "have", "the", "same", "name", "as"],
                &["that", "doesn't", "have", "the", "same", "name", "as"],
                &["that", "does", "not", "have", "the", "same", "name", "as"],
            ]).value(true),
        ))
    })?;
    if marker == 0 {
        return None;
    }
    let words = crate::lexer::token_word_refs(tail);
    // Exiled/tagged/source references remain owned by the existing reader.
    if !matches!(words.first(), Some(&"a" | &"an" | &"another")) || words.contains(&"exiled") {
        return None;
    }
    let is_hand_comparison = words == ["another", "card", "in", "their", "hand"];
    if words.starts_with(&["another", "card", "in", "their", "hand"]) && !is_hand_comparison {
        return Some(Err(CardTextError::ParseError(
            "unsupported live-name hand comparison tail".into(),
        )));
    }
    if !negated && !is_hand_comparison
        && !words
            .iter()
            .any(|word| matches!(*word, "permanent" | "permanents"))
    {
        return None;
    }
    Some((|| {
        let mut candidate =
            super::parse_object_filter_with_grammar_entrypoint_lexed(&tokens[..marker], other)?;
        let exclude_candidate = words.first() == Some(&"another");
        let mut comparison = if is_hand_comparison {
            ObjectFilter {
                zone: Some(Zone::Hand),
                owner: Some(PlayerFilter::OwnerOf(ObjectRef::FilterCandidate)),
                ..Default::default()
            }
        } else {
            let rest = if exclude_candidate { &tail[1..] } else { tail };
            super::parse_object_filter_with_grammar_entrypoint_lexed(rest, false)?
        };
        // A bare token noun has no zone in the generic filter model. Here
        // it denotes the live comparison set of permanents, not token
        // objects temporarily retained after leaving the battlefield.
        comparison.zone.get_or_insert(Zone::Battlefield);
        let characteristics = vec![crate::target::ObjectCharacteristic::Name];
        let mut relation = if negated {
            crate::target::ObjectCharacteristicRelation::shares_none(characteristics, comparison)
        } else {
            crate::target::ObjectCharacteristicRelation::shares(characteristics, comparison)
        };
        relation.exclude_candidate = exclude_candidate;
        candidate.characteristic_relations.push(relation);
        Ok(candidate)
    })())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::target::ObjectCharacteristic;
    #[test]
    fn live_name_comparisons_keep_candidate_and_comparison_qualifiers_separate() {
        for text in [
            "legendary card with the same name as a legendary permanent you control",
            "nonland cards with the same name as another card in their hand",
        ] {
            let tokens = crate::lexer::lex_line(text, 0).unwrap();
            let entry =
                super::super::parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false)
                    .unwrap();
            let facade = crate::object_filters::parse_object_filter_lexed(&tokens, false).unwrap();
            let words = crate::lexer::token_word_refs(&tokens);
            assert_eq!(entry, facade);
            assert_eq!(
                entry,
                crate::object_filters::parse_object_filter_words(&words, false).unwrap()
            );
            assert_eq!(entry.characteristic_relations.len(), 1);
            assert!(entry.controller.is_none());
            let relation = &entry.characteristic_relations[0];
            assert_eq!(relation.characteristics, [ObjectCharacteristic::Name]);
            if text.starts_with("legendary") {
                assert!(entry.supertypes.contains(&Supertype::Legendary));
                assert!(
                    relation
                        .comparison
                        .supertypes
                        .contains(&Supertype::Legendary)
                );
                assert_eq!(relation.comparison.zone, Some(Zone::Battlefield));
                assert_eq!(relation.comparison.controller, Some(PlayerFilter::You));
                assert!(!relation.exclude_candidate);
            } else {
                assert!(entry.excluded_card_types.contains(&CardType::Land));
                assert!(relation.exclude_candidate);
                assert_eq!(relation.comparison.zone, Some(Zone::Hand));
                assert_eq!(
                    relation.comparison.owner,
                    Some(PlayerFilter::OwnerOf(ObjectRef::FilterCandidate))
                );
                assert!(relation.comparison.excluded_card_types.is_empty());
            }
        }
    }
    #[test]
    fn live_name_reader_does_not_take_linked_exile_references_or_accept_unknown_tails() {
        let tokens = crate::lexer::lex_line(
            "spells with the same name as a card exiled with this permanent",
            0,
        )
        .unwrap();
        assert!(parse_live_name_relation(&tokens, false).is_none());
        for text in [
            "cards with the same name as another card in their hand banana",
            "legendary card with the same name as a legendary permanent you control banana",
        ] {
            let tokens = crate::lexer::lex_line(text, 0).unwrap();
            assert!(
                parse_live_name_relation(&tokens, false).unwrap().is_err(),
                "{text}"
            );
        }
    }

    #[test]
    fn negative_live_name_sets_preserve_polarity_and_candidate_exclusion() {
        for (text, another, token) in [
            ("nontoken creature you control that doesn't have the same name as a token you control", false, true),
            ("enchantment you control that doesn't have the same name as another permanent you control", true, false),
        ] {
            let tokens = crate::lexer::lex_line(text, 0).unwrap();
            let filter = super::super::parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
            assert_eq!(filter, crate::object_filters::parse_object_filter(&tokens, false).unwrap());
            assert_eq!(filter, crate::object_filters::parse_object_filter_lexed(&tokens, false).unwrap());
            let [relation] = filter.characteristic_relations.as_slice() else { panic!("{filter:?}"); };
            assert_eq!(relation.kind, crate::target::ObjectCharacteristicRelationKind::SharesNone);
            assert_eq!(relation.exclude_candidate, another);
            assert_eq!(relation.comparison.token, token);
            assert_eq!(relation.comparison.controller, Some(PlayerFilter::You));
            assert_eq!(relation.comparison.zone, Some(Zone::Battlefield));
            assert_eq!(filter.controller, Some(PlayerFilter::You));
            assert!(filter.tagged_constraints.is_empty());
        }
    }
}

#[cfg(test)]
mod body_reference_tests {
    use super::*;
    use crate::cards::builders::{ConditionalEffectAst, EffectAst, ForEachEffectAst, StatChangeActionAst, SubjectVerbActionAst};

    #[test]
    fn source_and_same_name_pump_has_two_scoped_recipients_and_no_target() {
        let tokens = crate::lexer::lex_line("This creature and each other creature with the same name as it get +3/+3 until end of turn.", 0).unwrap();
        let effects = crate::effect_sentences::parse_same_name_gets_fanout_sentence(&tokens).unwrap().unwrap();
        let [EffectAst::SubjectVerb(source), EffectAst::SubjectVerb(others)] = effects.as_slice() else { panic!("{effects:?}"); };
        assert!(matches!(&source.action, SubjectVerbActionAst::StatChanges(StatChangeActionAst::Pump {
            target: crate::cards::builders::TargetAst::Source(_), power: crate::effect::Value::Fixed(3), toughness: crate::effect::Value::Fixed(3), ..
        })));
        let SubjectVerbActionAst::StatChanges(StatChangeActionAst::PumpAll { filter, .. }) = &others.action else { panic!("{others:?}"); };
        assert!(filter.other);
        assert!(filter.tagged_constraints.iter().any(|constraint| constraint.tag.as_str() == crate::tag::CompilerReferenceTag::SourceObject.as_str()
            && constraint.relation == TaggedOpbjectRelation::SameNameAsTagged));
    }

    #[test]
    fn name_existence_condition_uses_candidate_exclusion_not_source_exclusion() {
        let tokens = crate::lexer::lex_line("another permanent with the same name is on the battlefield", 0).unwrap();
        let PredicateAst::ItMatches(filter) = super::super::parse_condition_predicate_lexed(&tokens).unwrap() else { panic!("name predicate"); };
        let [relation] = filter.characteristic_relations.as_slice() else { panic!("{filter:?}"); };
        assert_eq!(relation.kind, crate::target::ObjectCharacteristicRelationKind::SharesAny);
        assert!(relation.exclude_candidate);
        assert!(!relation.comparison.other);
        assert!(filter.tagged_constraints.is_empty());
    }

    #[test]
    fn opponent_cast_condition_retains_the_name_comparison_inside_per_player_history() {
        let tokens = crate::lexer::lex_line("Each opponent who cast a spell this turn with the same name as that card loses 6 life.", 0).unwrap();
        let program = crate::effect_sentences::parse_effect_chain(&tokens).unwrap();
        let [EffectAst::ForEach(ForEachEffectAst::ForEachOpponent { effects })] = program.as_slice() else { panic!("opponent loop: {program:?}"); };
        let [EffectAst::Conditionals(ConditionalEffectAst::Conditional { predicate, if_true, if_false })] = effects.as_slice() else { panic!("history gate"); };
        let PredicateAst::ValueComparison { left: Value::SpellsCastThisTurnMatching { player, filter, exclude_source }, .. } = predicate else { panic!("{predicate:?}"); };
        assert_eq!(*player, PlayerFilter::IteratedPlayer);
        assert!(!*exclude_source);
        assert!(filter.tagged_constraints.iter().any(|constraint| constraint.relation == TaggedOpbjectRelation::SameNameAsTagged));
        assert_eq!(if_true.len(), 1);
        assert!(if_false.is_empty());
    }

    #[test]
    fn definite_token_predicate_reads_the_creation_result() {
        let tokens = crate::lexer::lex_line("the token is an Aura", 0).unwrap();
        let PredicateAst::ItMatches(filter) = super::super::parse_condition_predicate_lexed(&tokens).unwrap() else { panic!("created-token predicate"); };
        assert!(filter.subtypes.contains(&Subtype::Aura));
        assert!(!filter.source);
    }
}
