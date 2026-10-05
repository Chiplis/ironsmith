//! Game-state bindings for static dynamic modifiers. Resolution-local values
//! need a separately bound source/affected-object context before admission.
use super::*;

pub(super) fn supports_game_state_binding(value: &Value) -> bool {
    ironsmith_core::anthem_model::supports_controller_state_anthem_value(value)
        || ironsmith_core::anthem_model::supports_scoped_reference_anthem_value(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex_line;

    #[test]
    fn static_dynamic_bindings_keep_typed_values_and_negative_signs() {
        for (text, marker) in [
            (
                "This creature gets -X/-X, where X is your life total.",
                "LifeTotal",
            ),
            (
                "Enchanted creature gets -X/-X, where X is the number of cards in your hand.",
                "CardsInHand",
            ),
            (
                "This creature gets +X/+0, where X is the greatest power among creature cards in your graveyard.",
                "GreatestPower",
            ),
            (
                "This creature gets +X/+0, where X is the amount of life you've lost this turn.",
                "LifeLostThisTurn",
            ),
            (
                "Creatures you control get +X/+X, where X is the number of cards you've drawn this turn.",
                "MaxCardsDrawnThisTurn",
            ),
        ] {
            let tokens = lex_line(text, 0).unwrap();
            let get = tokens
                .iter()
                .position(|token| token.is_any_word(&["get", "gets"]))
                .unwrap();
            let clause = parse_anthem_clause(&tokens, get, tokens.len()).unwrap();
            let AnthemValue::Dynamic(power) = clause.power else {
                panic!("{text}");
            };
            assert!(format!("{power:?}").contains(marker), "{text}: {power:?}");
            assert_eq!(
                matches!(power.unhinted(), Value::Scaled(_, -1)),
                text.contains("-X")
            );
        }
    }

    #[test]
    fn unresolved_object_and_event_operands_do_not_become_zero_valued_statics() {
        assert!(!supports_game_state_binding(&Value::ManaValueOf(Box::new(
            ChooseSpec::Tagged(crate::tag::CompilerReferenceTag::It.bind().into())
        ))));
        assert!(!supports_game_state_binding(&Value::EventValue(
            ironsmith_core::EventValueSpec::Amount
        )));
        assert!(!supports_game_state_binding(&Value::DividedRoundedDown(
            Box::new(Value::Fixed(1)),
            0
        )));
    }
}

/// In an attachment's static modifier, "its mana value" names the affected
/// host. It is not a resolution-local pronoun and must not fall back to the
/// Equipment/Aura source. Other subjects and quantities retain their readers.
pub(super) fn bind_affected_mana_value(value: Value, subject_tokens: &[OwnedLexToken]) -> Value {
    let subject = crate::lexer::parser_token_word_refs(subject_tokens);
    if !matches!(
        subject.as_slice(),
        ["equipped", "creature"] | ["enchanted", "creature" | "permanent"]
    ) {
        return value;
    }
    fn bind(value: Value) -> Value {
        match value {
            Value::SurfaceHinted { value, hints } => Value::SurfaceHinted {
                value: Box::new(bind(*value)),
                hints,
            },
            Value::ManaValueOf(spec)
                if matches!(spec.unhinted(), ChooseSpec::Tagged(tag) if tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str())
                    && matches!(spec.source_reference_surface(), Some(crate::target::SourceReferenceSurface::ThisPermanentType(surface)) if surface == "it") =>
            {
                Value::ManaValueOf(Box::new(
                    ChooseSpec::Iterated.with_surface_hints(spec.surface_hints().iter().cloned()),
                ))
            }
            other => other,
        }
    }
    bind(value)
}

#[cfg(test)]
mod scoped_tests {
    use super::*;
    #[test]
    fn attached_subject_mana_value_is_recipient_bound_but_unrelated_pronouns_are_not() {
        let tokens = crate::lexer::lex_line(
            "Equipped creature gets +X/+X, where X is its mana value.",
            0,
        )
        .unwrap();
        let get = tokens
            .iter()
            .position(|token| token.is_word("gets"))
            .unwrap();
        let clause = parse_anthem_clause(&tokens, get, tokens.len()).unwrap();
        assert!(
            matches!(clause.power, AnthemValue::Dynamic(value) if matches!(value.unhinted(), Value::ManaValueOf(spec) if matches!(spec.base(), ChooseSpec::Iterated)))
        );
        let unbound = Value::ManaValueOf(Box::new(ChooseSpec::Tagged(
            crate::tag::CompilerReferenceTag::It.key(),
        )));
        let unknown_subject = crate::lexer::lex_line("Creatures you control", 0).unwrap();
        assert_eq!(
            bind_affected_mana_value(unbound.clone(), &unknown_subject),
            unbound
        );
        assert!(!supports_game_state_binding(&unbound));
    }
    #[test]
    fn all_counters_on_the_source_are_not_counters_on_each_affected_creature() {
        let tokens = crate::lexer::lex_line("Other creatures you control get +X/+X, where X is the number of counters on this creature.", 0).unwrap();
        let get = tokens
            .iter()
            .position(|token| token.is_word("get"))
            .unwrap();
        let clause = parse_anthem_clause(&tokens, get, tokens.len()).unwrap();
        assert!(
            matches!(clause.power, AnthemValue::Dynamic(value) if matches!(value.unhinted(), Value::CountersOn(spec, None) if matches!(spec.base(), ChooseSpec::Source)))
        );
    }
}

#[cfg(test)]
#[test]
fn a_distinct_demonstrative_artifact_is_not_rebound_to_the_equipped_subject() {
    let subject = crate::lexer::lex_line("equipped creature", 0).unwrap();
    let tokens = crate::lexer::lex_line("that artifact's mana value", 0).unwrap();
    let (value, _) = crate::util::parse_value(&tokens).unwrap();
    let unchanged = bind_affected_mana_value(value.clone(), &subject);
    assert_eq!(unchanged, value);
    assert!(!supports_game_state_binding(&unchanged));
}
