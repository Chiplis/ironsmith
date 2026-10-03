use super::*;
use ironsmith_core::{PreventMatchingDamageSpec, StaticDamagePreventionAmount};

/// The event's recipient is a player, an object, or an authored union of both.
/// This reuses ordinary object filtering rather than a finite card-name table.
fn prevention_recipient_filters(
    tokens: &[OwnedLexToken],
) -> Result<(Option<PlayerFilter>, Option<ObjectFilter>), CardTextError> {
    let words = parser_token_word_refs(tokens);
    let known = parse_damage_amount_replacement_target_filters(&words)?;
    if known.0.is_some() || known.1.is_some() { return Ok(known); }
    for (index, token) in tokens.iter().enumerate() {
        if !token.is_word("or") { continue; }
        let left = parser_token_word_refs(&tokens[..index]);
        let (player, object) = parse_damage_amount_replacement_target_filters(&left)?;
        if player.is_some() && object.is_none() {
            let object = parse_object_filter_lexed(&tokens[index + 1..], false)?;
            return Ok((player, Some(object)));
        }
    }
    let filter = if is_source_reference_words(&words) {
        let mut filter = ObjectFilter::source();
        filter.source_surface = source_reference_surface_for_words(&words);
        filter
    } else {
        parse_object_filter_lexed(tokens, false)?
    };
    Ok((None, Some(filter)))
}

pub fn parse_filtered_damage_prevention_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(shape) = keyword_static_lines::parse_filtered_damage_prevention_tokens(tokens)
    else { return Ok(None); };
    // Quantified simultaneous recipients can share ONE prevention budget
    // (Cover of Winter), which needs a player-chosen batch allocation. The
    // single-recipient runtime action must not apply that budget once per target.
    let recipient_words = parser_token_word_refs(shape.damaged_tokens);
    if recipient_words.windows(3).any(|words| words == ["one", "or", "more"])
        || shape.damaged_tokens.iter().any(|token| token.is_word("and/or"))
    {
        return Ok(None);
    }
    let (target_player_filter, target_object_filter) =
        prevention_recipient_filters(shape.damaged_tokens)?;
    let amount = match shape.amount {
        keyword_static_lines::FilteredPreventionAmountShape::All =>
            StaticDamagePreventionAmount::All,
        keyword_static_lines::FilteredPreventionAmountShape::AllBut(remaining) =>
            StaticDamagePreventionAmount::AllBut(remaining),
        keyword_static_lines::FilteredPreventionAmountShape::Fixed(amount) => {
            let Ok(amount) = i32::try_from(amount) else { return Ok(None); };
            StaticDamagePreventionAmount::Amount(Value::Fixed(amount))
        }
        keyword_static_lines::FilteredPreventionAmountShape::Dynamic(tokens) => {
            let Some((value, used)) = parse_value(tokens) else { return Ok(None); };
            // A trailing sentence, optional action, or qualifier cannot disappear
            // behind a partially recognized value expression.
            if used != tokens.len()
                || tokens.iter().any(|token| token.kind == TokenKind::Period)
            { return Ok(None); }
            StaticDamagePreventionAmount::Amount(value)
        }
    };
    Ok(Some(StaticAbility::prevent_matching_damage(PreventMatchingDamageSpec {
        source_filter: damage_source_filter_from_shape(shape.source)?,
        target_player_filter,
        target_object_filter,
        combat_only: shape.combat_only,
        noncombat_only: shape.noncombat_only,
        maximum_damage: shape.maximum_damage,
        amount,
        display: render_token_slice(tokens),
    })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex_line;

    fn parse(text: &str) -> Option<StaticAbility> {
        parse_filtered_damage_prevention_line(&lex_line(text, 0).unwrap()).unwrap()
    }

    #[test]
    fn filtered_prevention_retains_amount_recipient_and_threshold() {
        let ability = parse("If a source would deal 3 or less damage to this creature, prevent that damage.").unwrap();
        let ironsmith_core::StaticAbilityPayload::PreventMatchingDamage(spec) = ability.payload else {
            panic!("typed prevention payload required");
        };
        assert_eq!(spec.maximum_damage, Some(3));
        assert_eq!(spec.amount, StaticDamagePreventionAmount::All);
        assert!(spec.target_object_filter.unwrap().source);
        let ability = parse("If a source would deal damage to you or a Hero you control, prevent all but 1 of that damage.").unwrap();
        let ironsmith_core::StaticAbilityPayload::PreventMatchingDamage(spec) = ability.payload else { panic!() };
        assert_eq!(spec.target_player_filter, Some(PlayerFilter::You));
        assert_eq!(spec.amount, StaticDamagePreventionAmount::AllBut(1));
        assert!(spec.target_object_filter.unwrap().subtypes.contains(&Subtype::Hero));
    }

    #[test]
    fn bound_prevention_amount_is_a_complete_dynamic_value() {
        let ability = parse("If a source would deal damage to equipped creature, prevent X of that damage, where X is the number of creatures you control.").unwrap();
        let ironsmith_core::StaticAbilityPayload::PreventMatchingDamage(spec) = ability.payload else { panic!() };
        assert!(matches!(spec.amount, StaticDamagePreventionAmount::Amount(ref value) if matches!(value.unhinted(), Value::Count(_))));
    }

    #[test]
    fn prevention_does_not_erase_optional_or_follow_up_effects() {
        for text in [
            "If a source would deal damage to a player, you may prevent 1 of that damage.",
            "If a creature would deal combat damage to you and/or one or more creatures you control, prevent X of that damage, where X is the number of age counters on this enchantment.",
            "If a source would deal damage to one or more creatures you control, prevent 1 of that damage.",
            "If a spell you control would deal damage to an opponent, prevent that damage. Create a token.",
            "If a source would deal damage to this creature, prevent that damage and draw a card.",
            "If a source would deal damage to equipped creature, prevent X of that damage, where X is the number of creatures you control. Draw a card.",
            "If a source would deal 3 or more damage to this creature, prevent that damage.",
            "If a source would deal damage to this creature this turn, prevent that damage.",
        ] {
            assert!(!matches!(parse_filtered_damage_prevention_line(&lex_line(text, 0).unwrap()), Ok(Some(_))), "{text}");
        }
    }
}
