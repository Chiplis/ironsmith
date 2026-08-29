use super::*;
use crate::lexer::lex_line;

fn parsed_followup_target(text: &str) -> TargetAst {
    let tokens = lex_line(text, 0).expect("damage unless-payment procedure should lex");
    let effects = parse_effect_sentences_lexed(&tokens)
        .expect("damage unless-payment procedure should parse structurally");
    primary_damage_target_from_effect(
        effects
            .get(1)
            .expect("procedure should retain its result-gated followup"),
    )
    .expect("followup should contain damage")
}

#[test]
fn definite_permanent_or_player_reuses_the_prior_any_target_damage_recipient() {
    let exact = "This spell deals 4 damage to any target unless that permanent's controller or that player pays {2}. If they do, this spell deals 2 damage to the permanent or player.";
    let exact_target = parsed_followup_target(exact);
    assert!(
        matches!(
            &exact_target,
            TargetAst::Tagged(tag, None)
                if tag == &crate::tag::CompilerReferenceTag::Damaged0.key()
        ),
        "{exact_target:#?}"
    );

    let fresh = "This spell deals 4 damage to any target unless that permanent's controller or that player pays {2}. If they do, this spell deals 2 damage to a permanent or player.";
    assert!(matches!(
        parsed_followup_target(fresh),
        TargetAst::ObjectOrPlayer(_, PlayerFilter::Any, None)
    ));
}
