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
    pronoun
        && ["on", "choice", "top", "bottom", "library"]
            .into_iter()
            .all(|word| primitives::find_prefix(tokens, || primitives::kw(word)).is_some())
}
