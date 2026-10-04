//! Live comparison-set name predicates, distinct from tagged antecedents and
//! relations among the objects selected together for a cost.
use super::*;

pub(crate) fn parse_live_name_relation(
    tokens: &[OwnedLexToken],
    other: bool,
) -> Option<Result<ObjectFilter, CardTextError>> {
    let marker = tokens.windows(5).position(|window| {
        window
            .iter()
            .zip(["with", "the", "same", "name", "as"])
            .all(|(token, word)| token.is_word(word))
    })?;
    if marker == 0 {
        return None;
    }
    let tail = &tokens[marker + 5..];
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
    if !is_hand_comparison
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
        let comparison = if is_hand_comparison {
            ObjectFilter {
                zone: Some(Zone::Hand),
                owner: Some(PlayerFilter::OwnerOf(ObjectRef::FilterCandidate)),
                ..Default::default()
            }
        } else {
            let rest = if exclude_candidate { &tail[1..] } else { tail };
            super::parse_object_filter_with_grammar_entrypoint_lexed(rest, false)?
        };
        let mut relation = crate::target::ObjectCharacteristicRelation::shares(
            vec![crate::target::ObjectCharacteristic::Name],
            comparison,
        );
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
}
