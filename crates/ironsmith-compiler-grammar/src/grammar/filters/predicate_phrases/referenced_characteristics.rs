//! Point-in-time characteristics of an already bound object.
use super::*;

pub(super) fn parse_referenced_characteristic_state(
    tokens: &[OwnedLexToken],
) -> Result<Option<PredicateAst>, CardTextError> {
    let clause = LexedClause::new(tokens);
    let words = clause.word_refs();
    // "if it's at least one of the chosen colors" (Tablet of the Guilds): the
    // referenced object is any of the source's chosen colors.
    if let ["it", "is", rest @ ..] | ["its" | "it's", rest @ ..] = words.as_slice()
        && rest == ["at", "least", "one", "of", "the", "chosen", "colors"]
    {
        return Ok(Some(PredicateAst::ItMatches(ObjectFilter {
            chosen_color: true,
            ..Default::default()
        })));
    }
    if let Some(reference) = demonstrative_reference_prefix(clause)
        && words.get(reference.word_len..) == Some(&["was", "blocked", "this", "turn"][..])
    {
        let mut filter = ObjectFilter {
            was_blocked_this_turn: true,
            ..Default::default()
        };
        filter.set_demonstrative_antecedent_surface(reference.antecedent_surface);
        return Ok(Some(PredicateAst::ItMatches(filter)));
    }

    if matches!(
        words.as_slice(),
        [
            "its",
            "power",
            "was",
            "different",
            "from",
            "its",
            "base",
            "power"
        ]
    ) {
        return Ok(Some(PredicateAst::ItMatchedLastKnown(ObjectFilter {
            power_comparison_to_base: Some(crate::effect::ValueComparisonOperator::NotEqual),
            ..Default::default()
        })));
    }
    // Singular descriptor authenticates the object role of the plural-form
    // personal pronoun; do not widen arbitrary "they ..." player clauses.
    if let ["they", "were", "a" | "an", noun] = words.as_slice()
        && let Some(kind) = crate::util::parse_card_type(noun)
    {
        return Ok(Some(PredicateAst::ItMatchedLastKnown(ObjectFilter {
            card_types: vec![kind],
            ..Default::default()
        })));
    }
    if let Some((descriptor, negative, _, _, when)) = demonstrative_descriptor_filter_tokens(tokens)
        && !negative
        && let [pair] = LexedClause::new(&descriptor).word_refs().as_slice()
        && let Some((power, toughness)) = crate::util::parse_unsigned_pt_word(pair)
    {
        let mut filter = ObjectFilter {
            power: Some(crate::filter::Comparison::Equal(power)),
            toughness: Some(crate::filter::Comparison::Equal(toughness)),
            ..Default::default()
        };
        filter.set_demonstrative_antecedent_surface(demonstrative_antecedent_surface(tokens));
        return Ok(Some(demonstrative_match_predicate(filter, when)));
    }
    if let Some(index) = words
        .windows(4)
        .position(|words| words == ["was", "attached", "to", "it"])
        && index + 4 == words.len()
        && index != 0
    {
        let subject = clause
            .between_word_range(0, index)
            .expect("attachment subject");
        // The attachment relation explicitly establishes the past frame.
        // Normalize only its controller verb for the ordinary complete filter reader.
        let normalized: Vec<_> = subject
            .tokens()
            .iter()
            .map(|token| {
                if token.is_word("controlled") {
                    OwnedLexToken::synthetic_word("control")
                } else {
                    token.clone()
                }
            })
            .collect();
        let attachment = parse_object_filter_lexed(&normalized, false)?;
        return Ok(Some(PredicateAst::ItMatchedLastKnown(ObjectFilter {
            with_attached_object: Some(Box::new(attachment)),
            ..Default::default()
        })));
    }
    Ok(None)
}
