use super::*;
use crate::lexer::lex_line;

#[test]
fn damage_unless_participant_places_counter_stays_one_typed_choice() {
    let tokens = lex_line(
            "This enchantment deals 3 damage to that player unless the player puts a -1/-1 counter on a creature they control.",
            0,
        )
        .expect("damage-unless clause should lex");
    let shape = choice_shapes::parse_unless_sentence_shape(&tokens).expect("unless sentence shape");
    let prefix =
        parse_effect_chain(shape.action_tokens).expect("damage prefix should parse independently");
    assert!(!prefix.is_empty(), "damage prefix should not be empty");
    let built = try_build_unless(
        prefix,
        SubjectVerbPrimitiveClause::new(&tokens),
        shape.unless_token,
    )
    .expect("payment tail should parse");
    assert!(
        built.is_some(),
        "payment tail should form an unless wrapper"
    );
    let parsed = parse_sentence_unless_pays(SubjectVerbPrimitiveClause::new(&tokens))
        .expect("damage-unless clause should parse")
        .expect("unless parser should claim the complete clause");
    assert!(
        matches!(parsed.as_slice(), [EffectAst::UnlessPays { .. }]),
        "expected one typed unless-payment around the damage: {parsed:#?}"
    );
}

#[test]
fn public_effect_route_keeps_trailing_unless_payment_on_tap_actions() {
    for text in [
        "Tap this creature unless you pay 1 life.",
        "Tap target creature unless its controller pays {1}.",
    ] {
        let tokens = lex_line(text, 0).expect("tap-unless text should lex");
        let parsed = crate::effect_sentences::parse_effect_sentences_lexed(&tokens)
            .expect("public effect route should parse the complete choice");
        assert!(
            matches!(parsed.as_slice(), [EffectAst::UnlessPays { .. }]),
            "expected one typed unless-payment for {text}: {parsed:#?}"
        );
    }
}

#[test]
fn multi_target_destroy_keeps_opponent_chooser_on_second_target() {
    let tokens = lex_line(
            "Destroy target nonbasic land you don't control and target nonbasic land of an opponent's choice you don't control.",
            0,
        )
        .expect("destroy pair should lex");
    let parsed = parse_sentence_destroy_multi_target(SubjectVerbPrimitiveClause::new(&tokens))
        .expect("destroy pair should parse")
        .expect("multi-target destroy rule should claim the sentence");
    let [EffectAst::Coordinated { effects, .. }] = parsed.as_slice() else {
        panic!("expected one coordinated destroy pair: {parsed:#?}");
    };
    let [_, EffectAst::Sequence { effects: chosen }] = effects.as_slice() else {
        panic!("the second destroy must retain its delegated choice: {effects:#?}");
    };
    let [
        EffectAst::SubjectVerb(target_only),
        EffectAst::SubjectVerb(destroy),
    ] = chosen.as_slice()
    else {
        panic!("expected target declaration followed by destroy: {chosen:#?}");
    };
    assert_eq!(target_only.subject.role, SubjectVerbRoleAst::Chooser);
    assert_eq!(target_only.subject.player, PlayerAst::Opponent);
    assert!(matches!(
        target_only.action,
        SubjectVerbActionAst::TargetOnly {
            explicit_declaration: true,
            ..
        }
    ));
    assert!(matches!(
        destroy.action,
        SubjectVerbActionAst::Destroy {
            target: TargetAst::Tagged(_, _),
            ..
        }
    ));
}
