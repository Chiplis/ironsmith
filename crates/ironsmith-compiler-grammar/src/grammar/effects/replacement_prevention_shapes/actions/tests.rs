use super::*;
use crate::lexer::lex_line;

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
            includes_this_way: true,
        })
    );

    let activation_cost_reference = lex_line(
        "For each counter removed, this creature gets +2/+0 until end of turn.",
        0,
    )
    .unwrap();
    assert_eq!(
        parse_counter_removed_pump_shape(&activation_cost_reference),
        Some(CounterRemovedPumpShape {
            power: 2,
            toughness: 0,
            includes_this_way: false,
        })
    );
}

#[test]
fn counted_turns_share_one_player_and_preserve_the_exact_anchor() {
    for (line, player, count, anchor) in [
        ("Take an extra turn after this one.", PlayerAst::You, 1, ExtraTurnAnchorAst::CurrentTurn),
        ("You take an extra turn after this one.", PlayerAst::You, 1, ExtraTurnAnchorAst::CurrentTurn),
        ("Target player takes two extra turns after this one.", PlayerAst::Target, 2, ExtraTurnAnchorAst::CurrentTurn),
        ("Target opponent takes three extra turns after this one.", PlayerAst::TargetOpponent, 3, ExtraTurnAnchorAst::CurrentTurn),
        ("The chosen player takes an extra turn after this one.", PlayerAst::Chosen, 1, ExtraTurnAnchorAst::CurrentTurn),
        ("After that turn, that player takes two extra turns.", PlayerAst::That, 2, ExtraTurnAnchorAst::ReferencedTurn),
    ] {
        let shape = parse_extra_turn_shape(&lex_line(line, 0).unwrap()).expect(line);
        assert_eq!(shape.player, player);
        assert_eq!(shape.count, count);
        assert_eq!(shape.anchor, anchor);
        let expected = EffectAst::subject_verb_extra_turn_after_turn(player, anchor);
        if count == 1 {
            assert_eq!(shape.into_effect(), expected);
        } else {
            assert_eq!(shape.into_effect(), EffectAst::ForEach(ForEachEffectAst::RepeatEffects {
                count: Value::Fixed(count), effects: vec![expected],
            }));
        }
    }
    for line in [
        "Take zero extra turns after this one.",
        "Take X extra turns after this one.",
        "Take two extra turn after this one.",
        "Take an extra turns after this one.",
        "Take two extra turns after your next turn.",
        "Take two extra turns after this one with no untap step.",
        "Target creature takes an extra turn after this one.",
    ] {
        assert!(parse_extra_turn_shape(&lex_line(line, 0).unwrap()).is_none(), "{line}");
    }
}

#[test]
fn resource_tail_uses_the_same_count_and_does_not_consume_a_rider() {
    let tokens = lex_line("two extra turns after this one", 0).unwrap();
    let shape = crate::grammar::effects::resource_shapes::parse_resource_take_extra_turn_shape(
        &tokens, PlayerAst::Target,
    ).unwrap();
    assert_eq!(shape.count, 2);
    assert_eq!(shape.player, PlayerAst::Target);
    assert_eq!(shape.anchor, ExtraTurnAnchorAst::CurrentTurn);
    let tokens = lex_line("an extra turn after this one with no combat phase", 0).unwrap();
    assert!(crate::grammar::effects::resource_shapes::parse_resource_take_extra_turn_shape(
        &tokens, PlayerAst::You,
    ).is_none());
}
