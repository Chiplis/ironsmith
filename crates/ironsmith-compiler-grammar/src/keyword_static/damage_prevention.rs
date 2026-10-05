use super::*;
use ironsmith_core::{PreventMatchingDamageSpec, StaticDamagePreventionAmount};

/// The event's recipient is a player, an object, or an authored union of both.
/// This reuses ordinary object filtering rather than a finite card-name table.
pub(super) fn prevention_recipient_filters(
    tokens: &[OwnedLexToken],
) -> Result<(Option<PlayerFilter>, Option<ObjectFilter>), CardTextError> {
    let words = parser_token_word_refs(tokens);
    let known = parse_damage_amount_replacement_target_filters(&words)?;
    if known.0.is_some() || known.1.is_some() {
        return Ok(known);
    }
    for (index, token) in tokens.iter().enumerate() {
        if !token.is_word("or") {
            continue;
        }
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

pub(super) fn needs_shared_recipient_allocation(tokens: &[OwnedLexToken]) -> bool {
    parser_token_word_refs(tokens)
        .windows(3)
        .any(|words| words == ["one", "or", "more"])
        || tokens.iter().any(|token| token.is_word("and/or"))
}

pub fn parse_filtered_damage_prevention_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = parser_token_word_refs(tokens);
    if words
        .windows(2)
        .any(|words| words == ["this", "turn"] || words == ["this", "combat"])
    {
        return Ok(None);
    }
    let Some(shape) = keyword_static_lines::parse_filtered_damage_prevention_tokens(tokens) else {
        return Ok(None);
    };
    // Quantified simultaneous recipients can share ONE prevention budget
    // (Cover of Winter), which needs a player-chosen batch allocation. The
    // single-recipient runtime action must not apply that budget once per target.
    if needs_shared_recipient_allocation(shape.damaged_tokens) {
        return Ok(None);
    }
    let (target_player_filter, target_object_filter) =
        prevention_recipient_filters(shape.damaged_tokens)?;
    let source_filter = damage_source_filter_from_shape(shape.source)?;
    if let Some(repeated) = shape.repeated_spell_target_tokens {
        // The repeated spell and recipient must refer to the event just read;
        // neither a different source kind nor a narrowed recipient is omitted.
        let source_words = parser_token_word_refs(shape.source.filter_tokens);
        if source_filter.zone != Some(Zone::Stack)
            || !source_words.contains(&"spell")
            || repeated.iter().any(|token| token.kind == TokenKind::Period)
            || needs_shared_recipient_allocation(repeated)
        {
            return Ok(None);
        }
        let repeated_filters = prevention_recipient_filters(repeated)?;
        if repeated_filters.0 != target_player_filter || repeated_filters.1 != target_object_filter
        {
            return Ok(None);
        }
    }
    let amount = match shape.amount {
        keyword_static_lines::FilteredPreventionAmountShape::All => {
            StaticDamagePreventionAmount::All
        }
        keyword_static_lines::FilteredPreventionAmountShape::AllBut(remaining) => {
            StaticDamagePreventionAmount::AllBut(remaining)
        }
        keyword_static_lines::FilteredPreventionAmountShape::Fixed(amount) => {
            let Ok(amount) = i32::try_from(amount) else {
                return Ok(None);
            };
            StaticDamagePreventionAmount::Amount(Value::Fixed(amount))
        }
        keyword_static_lines::FilteredPreventionAmountShape::Dynamic(tokens) => {
            let Some((value, used)) = parse_value(tokens) else {
                return Ok(None);
            };
            // A trailing sentence, optional action, or qualifier cannot disappear
            // behind a partially recognized value expression.
            if used != tokens.len() || tokens.iter().any(|token| token.kind == TokenKind::Period) {
                return Ok(None);
            }
            StaticDamagePreventionAmount::Amount(value)
        }
    };
    Ok(Some(StaticAbility::prevent_matching_damage(
        PreventMatchingDamageSpec {
            source_filter,
            target_player_filter,
            target_object_filter,
            combat_only: shape.combat_only,
            noncombat_only: shape.noncombat_only,
            maximum_damage: shape.maximum_damage,
            amount,
            display: render_token_slice(tokens),
        },
    )))
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
        let ability =
            parse("If a source would deal 3 or less damage to this creature, prevent that damage.")
                .unwrap();
        let ironsmith_core::StaticAbilityPayload::PreventMatchingDamage(spec) = ability.payload
        else {
            panic!("typed prevention payload required");
        };
        assert_eq!(spec.maximum_damage, Some(3));
        assert_eq!(spec.amount, StaticDamagePreventionAmount::All);
        assert!(spec.target_object_filter.unwrap().source);
        let ability = parse("If a source would deal damage to you or a Hero you control, prevent all but 1 of that damage.").unwrap();
        let ironsmith_core::StaticAbilityPayload::PreventMatchingDamage(spec) = ability.payload
        else {
            panic!()
        };
        assert_eq!(spec.target_player_filter, Some(PlayerFilter::You));
        assert_eq!(spec.amount, StaticDamagePreventionAmount::AllBut(1));
        assert!(
            spec.target_object_filter
                .unwrap()
                .subtypes
                .contains(&Subtype::Hero)
        );
    }

    #[test]
    fn bound_prevention_amount_is_a_complete_dynamic_value() {
        let ability = parse("If a source would deal damage to equipped creature, prevent X of that damage, where X is the number of creatures you control.").unwrap();
        let ironsmith_core::StaticAbilityPayload::PreventMatchingDamage(spec) = ability.payload
        else {
            panic!()
        };
        assert!(
            matches!(spec.amount, StaticDamagePreventionAmount::Amount(ref value) if matches!(value.unhinted(), Value::Count(_)))
        );
    }

    #[test]
    fn repeated_spell_prevention_tail_preserves_both_event_operands() {
        let ability = parse("If a spell would deal damage to a permanent or player, prevent 1 damage that spell would deal to that permanent or player.").unwrap();
        let ironsmith_core::StaticAbilityPayload::PreventMatchingDamage(spec) = ability.payload
        else {
            panic!()
        };
        assert_eq!(spec.source_filter.zone, Some(Zone::Stack));
        assert_eq!(spec.target_player_filter, Some(PlayerFilter::Any));
        assert_eq!(spec.target_object_filter, Some(ObjectFilter::permanent()));
        assert_eq!(
            spec.amount,
            StaticDamagePreventionAmount::Amount(Value::Fixed(1))
        );
        for text in [
            "If a creature would deal damage to a player, prevent 1 damage that spell would deal to that player.",
            "If a spell would deal damage to a permanent or player, prevent 1 damage that spell would deal to that player.",
            "If a spell would deal damage to a permanent or player, prevent 1 damage that spell would deal to that permanent or player. Draw a card.",
        ] {
            assert!(
                !matches!(
                    parse_filtered_damage_prevention_line(&lex_line(text, 0).unwrap()),
                    Ok(Some(_))
                ),
                "{text}"
            );
        }
    }

    #[test]
    fn subject_first_self_prevention_keeps_fixed_counter_removal() {
        use crate::grammar::attached_object_static_lines::{
            RemoveCounterPreventionAmount, parse_remove_counter_prevention_tokens,
        };
        let tokens = lex_line("If this creature would be dealt damage, prevent that damage and remove a +1/+1 counter from it.", 0).unwrap();
        let spec = parse_remove_counter_prevention_tokens(&tokens).unwrap();
        assert_eq!(spec.counter_type, CounterType::PlusOnePlusOne);
        assert_eq!(spec.amount, RemoveCounterPreventionAmount::Fixed(1));
        assert!(!spec.one_damage_per_counter);
        for text in [
            "If another creature would be dealt damage, prevent that damage and remove a +1/+1 counter from it.",
            "If this creature would be dealt damage, prevent that damage and remove a +1/+1 counter from it. Draw a card.",
        ] {
            assert!(parse_remove_counter_prevention_tokens(&lex_line(text, 0).unwrap()).is_none());
        }
    }

    #[test]
    fn counter_shapes_keep_prevention_separate_from_genuine_replacement() {
        use crate::grammar::attached_object_static_lines::{
            PutCounterPreventionSpec, parse_put_counter_prevention_tokens,
        };
        for (text, expected_prevention) in [
            (
                "If damage would be dealt to this creature, put that many +1/+1 counters on it instead.",
                false,
            ),
            (
                "If damage would be dealt to this creature, prevent that damage and put that many +1/+1 counters on it.",
                true,
            ),
        ] {
            let tokens = lex_line(text, 0).unwrap();
            let Some(PutCounterPreventionSpec::General {
                prevents_damage, ..
            }) = parse_put_counter_prevention_tokens(&tokens)
            else {
                panic!("{text}")
            };
            assert_eq!(prevents_damage, expected_prevention);
        }
    }

    #[test]
    fn counter_followups_keep_sign_and_whole_sentence_scope() {
        use crate::grammar::attached_object_static_lines::{
            PutCounterPreventionSpec, parse_put_counter_prevention_tokens,
        };
        let tokens = lex_line("If damage would be dealt to this creature, prevent that damage. Put a -1/-1 counter on this creature for each 1 damage prevented this way.", 0).unwrap();
        assert!(matches!(
            parse_put_counter_prevention_tokens(&tokens),
            Some(PutCounterPreventionSpec::PerPreventedAmount {
                counter_type: CounterType::MinusOneMinusOne
            })
        ));
        for text in [
            "If damage would be dealt to this creature, prevent that damage. Put a -1/-1 counter on that creature for each 1 damage prevented this way.",
            "If damage would be dealt to this creature, prevent that damage. Put a -1/-1 counter on this creature for each 1 damage prevented this way. Draw a card.",
        ] {
            assert!(
                parse_put_counter_prevention_tokens(&lex_line(text, 0).unwrap()).is_none(),
                "{text}"
            );
        }
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
            assert!(
                !matches!(
                    parse_filtered_damage_prevention_line(&lex_line(text, 0).unwrap()),
                    Ok(Some(_))
                ),
                "{text}"
            );
        }
    }
}
