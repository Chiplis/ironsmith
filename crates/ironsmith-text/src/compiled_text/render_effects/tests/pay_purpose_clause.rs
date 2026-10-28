use super::*;

fn compile(text: &str) -> Result<crate::CardDefinition, String> {
    crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Pay Probe")
        .parse_text(format!("Type: Instant\n{text}"))
        .map_err(|error| format!("{error:?}"))
}

/// "You may pay {1} to draw a card" once compiled to the payment alone,
/// silently dropping what the payment buys. It now fails strictly, and the
/// supported "If you do" wording keeps both halves.
#[test]
fn pay_purpose_clause_never_drops_its_effect() {
    let error = compile("You may pay {1} to draw a card.").expect_err("pay-to clause must not compile to the payment alone");
    assert!(error.contains("pay {1} to draw a card"), "{error}");

    let definition = compile("You may pay {1}. If you do, draw a card.").expect("If you do wording compiles");
    let debug = format!("{:?}", definition.spell_effect);
    assert!(debug.contains("PayManaEffect") && debug.contains("DrawCardsEffect"), "{debug}");

    // Licids' special action keeps its own reading.
    compile("You may pay {W} to end this effect.").expect("Licid reading still compiles");
}
