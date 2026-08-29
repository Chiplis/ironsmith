use crate::lexer::lex_line;

use super::*;

fn shape(text: &str) -> AbilityCandidateShape {
    parse_ability_candidate_shape(&lex_line(text, 0).expect("lex fixture"))
}

#[test]
fn classifies_source_and_target_ability_grant_candidates() {
    assert!(shape("This creature gains flying until end of turn.").simple_source_gain);
    assert!(shape("Target creature gains flying until end of turn.").simple_gain);
    assert!(!shape("You gain 3 life.").simple_gain);
    assert!(!shape("Another target creature gains haste.").simple_gain);
}

#[test]
fn rejects_source_damage_then_tagged_ability_loss_as_one_grant_candidate() {
    let shape = shape(
        "This creature deals 2 damage to target creature with flying and that creature loses flying until end of turn.",
    );
    assert!(!shape.simple_source_gain);
    assert!(!shape.simple_gain);
}

#[test]
fn rejects_independent_conditioned_gain_loss_arms_as_one_grant_candidate() {
    let shape = shape(
        "Creatures your opponents control lose flying until end of turn if {G} was spent to cast this spell, and creatures you control gain flying until end of turn if {U} was spent to cast this spell.",
    );
    assert!(!shape.simple_source_gain);
    assert!(!shape.simple_gain);
}

#[test]
fn rejects_draw_then_pump_and_gain_as_one_grant_candidate() {
    let shape = shape(
        "You draw X cards and the chosen creatures get +X/+X and gain trample until end of turn, where X is the difference between the chosen creatures' powers.",
    );
    assert!(!shape.simple_source_gain);
    assert!(!shape.simple_gain);
}
