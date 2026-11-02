use super::*;

pub fn parse_discarded_this_way_modifier_shape(
    tokens: &[OwnedLexToken],
) -> Option<DiscardedThisWayModifierShape> {
    let first = tokens.first()?.parser_text();
    let (power, toughness) = crate::grammar::primitives::probe_shape(
        leaf::parse_leaf_pt_modifier_values_complete(first),
    )?;
    let (Value::Fixed(power), Value::Fixed(toughness)) = (power, toughness) else {
        return None;
    };
    crate::grammar::primitives::probe_all(
        tokens.get(1..)?,
        (
            primitives::phrase(&[
                "until",
                "end",
                "of",
                "turn",
                "for",
                "each",
                "card",
                "discarded",
                "this",
                "way",
            ]),
            primitives::sentence_end(),
        )
            .void(),
        "discarded this way modifier",
    )?;
    Some(DiscardedThisWayModifierShape { power, toughness })
}

pub fn is_pronoun_library_choice_put_shape(tokens: &[OwnedLexToken]) -> bool {
    let pronoun =
        primitives::parse_prefix(tokens, alt((primitives::kw("it"), primitives::kw("them"))))
            .is_some();
    let shape = pronoun
        && crate::grammar::effects::control_copy_attach_shapes::parse_library_choice_destination_shape(tokens).is_some();
    if shape {
        // "on their choice of the top or bottom of their library": both
        // possessors name the putting player (the object's owner).
        for pair in tokens.windows(2) {
            if pair[0].parser_text() == "their"
                && matches!(pair[1].parser_text(), "choice" | "library")
            {
                crate::word_claims::claim_tokens(&pair[..1]);
            }
        }
        // "choice of the top or bottom of their library": the two "of"s
        // link the choice and the library positions read above.
        for token in tokens.iter().filter(|token| token.parser_text() == "of") {
            crate::word_claims::claim_tokens(std::slice::from_ref(token));
        }
    }
    shape
}
