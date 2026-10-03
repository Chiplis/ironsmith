//! Game-state bindings for static dynamic modifiers. Resolution-local values
//! need a separately bound source/affected-object context before admission.
use super::*;

pub(super) fn supports_game_state_binding(value: &Value) -> bool {
    ironsmith_core::anthem_model::supports_controller_state_anthem_value(value)
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
                "TurnHistoryCount",
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
            ChooseSpec::Tagged(crate::tag::CompilerReferenceTag::It.bind())
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
