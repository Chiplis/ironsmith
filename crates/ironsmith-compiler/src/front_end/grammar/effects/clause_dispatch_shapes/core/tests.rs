use super::*;
use crate::lexer::lex_line;

#[test]
fn parses_may_subject_verb_and_pump_subject_shapes() {
    let may = lex_line("You may draw a card.", 0).unwrap();
    assert_eq!(
        parse_leading_may_shape(&may).unwrap().actor,
        LeadingMayActorShape::Player(PlayerAst::You)
    );
    let clause = lex_line("Target creature gets +1/+1 until end of turn.", 0).unwrap();
    let split = parse_clause_subject_verb_shape(&clause).unwrap();
    assert_eq!(split.kind, chain_splitting::ChainVerbKind::Get);
    assert!(matches!(
        parse_pump_subject_shape(split.subject_tokens).unwrap().kind,
        PumpSubjectKind::DirectTarget(_)
    ));

    let tagged = lex_line("They each get +2/+2 until end of turn.", 0).unwrap();
    let tagged_direct = parse_tagged_plural_pump_shape(&tagged).unwrap();
    assert_eq!(
        TokenWordView::new(tagged_direct.subject_tokens).word_refs(),
        ["they", "each"]
    );
    let split = parse_clause_subject_verb_shape(&tagged).unwrap();
    assert!(matches!(
        parse_pump_subject_shape(split.subject_tokens).unwrap().kind,
        PumpSubjectKind::Tagged
    ));

    let tagged_object = lex_line("Them each get +2/+2 until end of turn.", 0).unwrap();
    let split = parse_clause_subject_verb_shape(&tagged_object).unwrap();
    assert!(matches!(
        parse_pump_subject_shape(split.subject_tokens).unwrap().kind,
        PumpSubjectKind::Tagged
    ));

    let chosen = lex_line("The chosen creatures get +X/+X until end of turn.", 0).unwrap();
    let split = parse_clause_subject_verb_shape(&chosen).unwrap();
    assert!(matches!(
        parse_pump_subject_shape(split.subject_tokens).unwrap().kind,
        PumpSubjectKind::DemonstrativeTarget
    ));
}

#[test]
fn causative_player_may_keeps_the_explicit_affected_player_subject() {
    let each_opponent = lex_line("You may have each opponent lose 1 life.", 0).unwrap();
    let shape = parse_leading_may_shape(&each_opponent).expect("leading may shape");
    assert_eq!(shape.actor, LeadingMayActorShape::Player(PlayerAst::You));
    assert_eq!(
        TokenWordView::new(shape.effect_tokens).word_refs(),
        ["each", "opponent", "lose", "1", "life"]
    );

    let object_causative = lex_line("You may have it deal 1 damage to any target.", 0).unwrap();
    assert_eq!(
        TokenWordView::new(
            parse_leading_may_shape(&object_causative)
                .expect("object causative")
                .effect_tokens
        )
        .word_refs(),
        ["it", "deal", "1", "damage", "to", "any", "target"]
    );
}
