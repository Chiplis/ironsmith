use super::*;

fn parsed(text: &str) -> Vec<EffectAst> {
    let tokens = crate::lexer::lex_line(text, 0).unwrap();
    let (result, trace) = crate::parse_trace::capture(|| parse_effect_sentences_lexed(&tokens));
    result.unwrap_or_else(|error| panic!("{error:?}\n{}", trace.render()))
}

#[test]
fn sacrifice_x_followup_preserves_reflexive_trigger() {
    let effects = parsed(
        "You may sacrifice X Foods. When you do, up to X target attacking creatures each get +3/+3 and gain trample and indestructible until end of turn.",
    );
    assert!(
        matches!(
            effects.last(),
            Some(EffectAst::Conditionals(
                ConditionalEffectAst::WhenResult { .. }
            ))
        ),
        "{effects:#?}"
    );
}

#[test]
fn optional_tap_followup_keeps_pump_and_goad_in_reflexive_trigger() {
    let effects = parsed(
        "You may tap two untapped creatures you control. When you do, target creature that player controls gets +2/+2 and gains trample until end of turn. Goad that creature.",
    );
    let Some(EffectAst::Conditionals(ConditionalEffectAst::WhenResult { effects, .. })) =
        effects.last()
    else {
        panic!("missing reflexive followup: {effects:#?}");
    };
    assert!(format!("{effects:#?}").contains("Goad"), "{effects:#?}");
}

#[test]
fn optional_payment_in_otherwise_owns_reflexive_followup() {
    let effects = parsed(
        "Create a Treasure token if this is the first or second time this ability has resolved this turn. Otherwise, you may pay {X}. When you do, this creature deals that much damage to any target.",
    );
    let Some(EffectAst::Conditionals(ConditionalEffectAst::IfResult { effects, .. })) =
        effects.last()
    else {
        panic!("expected optional payment branch: {effects:#?}");
    };
    assert!(
        matches!(
            effects.last(),
            Some(EffectAst::Conditionals(
                ConditionalEffectAst::WhenResult { .. }
            ))
        ),
        "the reflexive followup must read payment: {effects:#?}"
    );
}

#[test]
fn conditional_return_with_quoted_replacement_keeps_return_and_result_gate() {
    let effects = parsed(
        "Choose target permanent card in your graveyard. You may sacrifice a permanent that shares a card type with the chosen card. If you do, return the chosen card from your graveyard to the battlefield and it gains \"If this permanent would leave the battlefield, exile it instead of putting it anywhere else.\"",
    );
    let Some(EffectAst::Conditionals(ConditionalEffectAst::IfResult { effects, .. })) =
        effects.last()
    else {
        panic!("missing result gate: {effects:#?}");
    };
    let debug = format!("{effects:#?}");
    assert!(
        debug.contains("ReturnToBattlefield") || debug.contains("MoveToZone"),
        "missing return: {debug}"
    );
    assert!(
        debug.contains("Grant") || debug.contains("RegisterZoneReplacement"),
        "missing replacement: {debug}"
    );
}
