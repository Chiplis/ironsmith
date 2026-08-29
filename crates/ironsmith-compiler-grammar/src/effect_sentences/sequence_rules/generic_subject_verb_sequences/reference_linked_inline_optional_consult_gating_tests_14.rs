use super::*;
use crate::lexer::lex_line;

fn parse_effect_text(text: &str) -> Vec<EffectAst> {
    let lexed = lex_line(text, 0).expect("focused consult text should lex");
    super::super::super::super::dispatch_entry::parse_effect_sentences_lexed(&lexed)
        .expect("focused consult text should parse")
}

#[test]
fn avenging_druid_keeps_the_consult_optional_and_gates_its_disposition() {
    let parsed = parse_effect_text(
        "You may reveal cards from the top of your library until you reveal a land card. If you do, put that card onto the battlefield and put all other cards revealed this way into your graveyard.",
    );
    let [
        EffectAst::May { effects: consult },
        EffectAst::IfResult {
            predicate: IfResultPredicate::Did,
            effects: disposition,
        },
    ] = parsed.as_slice()
    else {
        panic!("expected optional consult followed by its result gate: {parsed:#?}");
    };

    let consult = format!("{consult:#?}");
    let disposition = format!("{disposition:#?}");
    assert!(consult.contains("ConsultTopOfLibrary"), "{consult}");
    assert!(disposition.contains("MoveToZone"), "{disposition}");
    assert!(disposition.contains("ForEachTagged"), "{disposition}");
}

#[test]
fn optional_consult_gates_an_unprefixed_move_and_bottom_disposition() {
    let parsed = parse_effect_text(
        "You may reveal cards from the top of your library until you reveal a creature card. Put that card into your hand and the rest on the bottom of your library in a random order.",
    );
    let [
        EffectAst::May { effects: consult },
        EffectAst::IfResult {
            predicate: IfResultPredicate::Did,
            effects: disposition,
        },
    ] = parsed.as_slice()
    else {
        panic!("expected the complete consult procedure to remain optional: {parsed:#?}");
    };

    let consult = format!("{consult:#?}");
    let disposition = format!("{disposition:#?}");
    assert!(consult.contains("ConsultTopOfLibrary"), "{consult}");
    assert!(disposition.contains("MoveToZone"), "{disposition}");
    assert!(
        disposition.contains("PutTaggedRemainderOnBottomOfLibrary"),
        "{disposition}"
    );
}

#[test]
fn foster_gates_the_complete_consult_partition_on_the_optional_payment() {
    let parsed = parse_effect_text(
        "You may pay {1}. If you do, reveal cards from the top of your library until you reveal a creature card. Put that card into your hand and the rest into your graveyard.",
    );
    let [
        EffectAst::MayByPlayer {
            effects: payment, ..
        },
        EffectAst::IfResult {
            predicate: IfResultPredicate::Did,
            effects: gated_consult,
        },
    ] = parsed.as_slice()
    else {
        panic!("expected optional payment followed by one gated consult partition: {parsed:#?}");
    };

    let payment = format!("{payment:#?}");
    let gated_consult = format!("{gated_consult:#?}");
    assert!(payment.contains("PayMana"), "{payment}");
    assert!(
        gated_consult.contains("ConsultTopOfLibrary"),
        "{gated_consult}"
    );
    assert!(gated_consult.contains("MoveToZone"), "{gated_consult}");
    assert!(gated_consult.contains("ForEachTagged"), "{gated_consult}");
}
