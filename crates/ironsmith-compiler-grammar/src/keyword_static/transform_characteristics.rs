use super::*;
use ironsmith_core::static_ability_model::TransformCharacteristicAdjustment as Adjustment;

pub(super) fn parse(tokens: &[OwnedLexToken]) -> Option<StaticAbility> {
    let tokens = trim_edge_punctuation_tokens(tokens);
    let words = parser_token_word_refs(tokens);
    if !words.starts_with(&["as", "this"]) { return None; }
    let transforms = tokens.iter().position(|token| token.is_word("transforms"))?;
    if !tokens.get(transforms + 1)?.is_word("into") { return None; }
    for comma in transforms + 2..tokens.len() {
        if !tokens[comma].is_comma() { continue; }
        let tail = &tokens[comma + 1..];
        let words = parser_token_word_refs(tail);
        let adjustment = if words.len() >= 7 && words[..2] == ["counters", "on"]
            && matches!(words[2], "it" | "her" | "him" | "them")
            && words[3] == "become" && words[words.len()-2..] == ["counters", "instead"] {
            let kind = crate::grammar::filters::parse_counter_type_words(
                // This shared reader consumes the counter noun as well as
                // its descriptor. Keep `counters`, dropping only `instead`.
                &words[4..words.len()-1],
            )?;
            Adjustment::CounterKind(kind)
        } else if words.len() == 13
            && words[..7] == ["for", "each", "color", "of", "mana", "spent", "to"]
            && words[8..] == ["it", "it", "becomes", "that", "color"] {
            Adjustment::ColorsFromNamedActivationPayment(words[7].to_string())
        } else { continue; };
        let destination = render_token_slice(&tokens[transforms+2..comma]);
        if destination.is_empty() { return None; }
        let subject = render_token_slice(&tokens[1..transforms]);
        crate::word_claims::claim_tokens(tokens);
        return Some(StaticAbility::as_transforms_effect_program(
            ironsmith_core::resolution_model::ResolutionProgram::default(),
            subject, destination, None,
        ).with_transform_characteristics(adjustment));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn counter_kind_and_opaque_payment_label_are_atomic_static_programs() {
        for (text, destination, expected) in [
            (
                "As this permanent transforms into Radiant, the Proud, counters on them become loyalty counters instead.",
                "Radiant, the Proud",
                Adjustment::CounterKind(ironsmith_core::CounterType::Loyalty),
            ),
            (
                "As this creature transforms into Living Mechanism, for each color of mana spent to awaken it, it becomes that color.",
                "Living Mechanism",
                Adjustment::ColorsFromNamedActivationPayment("awaken".into()),
            ),
            (
                "As this creature transforms into Radiant, the Proud, for each color of mana spent to attain it, it becomes that color.",
                "Radiant, the Proud",
                Adjustment::ColorsFromNamedActivationPayment("attain".into()),
            ),
        ] {
            let tokens = crate::lexer::lex_line(text, 0).unwrap();
            let ability = parse(&tokens).unwrap_or_else(|| panic!("{text}"));
            let ironsmith_core::StaticAbilityPayload::AsEntersEffectProgram {
                program, transforms_into, transform_characteristics, ..
            } = ability.payload else { panic!("atomic transform program required"); };
            assert_eq!(transforms_into.as_deref(), Some(destination), "destination commas belong to the literal name");
            assert_eq!(transform_characteristics, Some(expected));
            assert!(program.segments.is_empty(), "characteristic adjustment must not become a later resolving program");
        }
        let tokens = crate::lexer::lex_line("Whenever this transforms, put a loyalty counter on it.", 0).unwrap();
        assert!(parse(&tokens).is_none());
    }
}
