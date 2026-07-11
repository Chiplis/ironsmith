use super::*;
use crate::runtime_backend::front_end::lexer::lex_line;

#[test]
fn parses_atomic_action_shapes() {
    let monstrosity = lex_line("Monstrosity 3.", 0).unwrap();
    assert_eq!(
        parse_monstrosity_shape(&monstrosity).map(|shape| shape.amount),
        Some(Value::Fixed(3))
    );
    let combat = lex_line("Sacrifice it at the end of combat.", 0).unwrap();
    assert_eq!(
        parse_token_end_combat_action_shape(&combat),
        Some(TokenEndCombatActionShape::Sacrifice)
    );
}

#[test]
fn parses_turn_and_phase_shapes() {
    let turn = lex_line("After that turn, that player takes an extra turn.", 0).unwrap();
    assert_eq!(
        parse_extra_turn_shape(&turn).map(|shape| shape.anchor),
        Some(ExtraTurnAnchorAst::ReferencedTurn)
    );
    let unpunctuated = lex_line("After that turn that player takes an extra turn.", 0).unwrap();
    assert_eq!(
        parse_extra_turn_shape(&unpunctuated).map(|shape| shape.anchor),
        Some(ExtraTurnAnchorAst::ReferencedTurn)
    );
    let phases = lex_line(
        "After this main phase, there is an additional combat phase followed by an additional main phase.",
        0,
    )
    .unwrap();
    assert_eq!(
        parse_additional_phases_shape(&phases).unwrap().phases.len(),
        2
    );
    let contracted = lex_line("There's an additional combat phase after this phase.", 0).unwrap();
    assert_eq!(
        parse_additional_phases_shape(&contracted).unwrap().phases,
        vec![AdditionalPhase::Combat]
    );
}

#[test]
fn parses_counter_removed_pump_shape() {
    let tokens = lex_line(
        "For each counter removed this way, this creature gets +1/+0 until end of turn.",
        0,
    )
    .unwrap();
    assert_eq!(
        parse_counter_removed_pump_shape(&tokens),
        Some(CounterRemovedPumpShape {
            power: 1,
            toughness: 0,
        })
    );
}
