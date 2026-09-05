use super::*;

#[test]
fn optional_single_hand_move_keeps_entry_state_and_grant_in_may_scope() {
    let tokens = crate::lexer::lex_line(
            "You may put a creature card with mana value 3 or less from your hand onto the battlefield. It enters tapped and attacking and gains indestructible until end of turn.",
            0,
        )
        .expect("follow-up fixture should lex");
    let parsed = parse_effect_sentences_lexed(&tokens).expect("follow-up should parse");
    let [
        EffectAst::MayByPlayer {
            player: PlayerAst::You,
            effects,
        },
    ] = parsed.as_slice()
    else {
        panic!("expected one optional procedure: {parsed:#?}");
    };
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::MoveToZone {
                    battlefield_tapped: true,
                    battlefield_attacking: true,
                    ..
                },
            ..
        }),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::GrantAbilitiesToTarget {
                    target: TargetAst::Tagged(tag, _),
                    abilities,
                    duration: Until::EndOfTurn,
                    ..
                },
            ..
        }),
    ] = effects.as_slice()
    else {
        panic!("entry follow-up escaped the may branch: {effects:#?}");
    };
    assert_eq!(tag.as_str(), crate::tag::CompilerReferenceTag::It.as_str());
    assert_eq!(abilities, &[KeywordAction::Indestructible.into()]);
}

#[test]
fn entry_followup_does_not_attach_to_a_mandatory_move() {
    let mut hand_creature = ObjectFilter::creature();
    hand_creature.zone = Some(Zone::Hand);
    let mut previous = EffectAst::subject_verb_move_to_zone(
        TargetAst::WithCount(
            Box::new(TargetAst::Object(hand_creature, None, None)),
            ChoiceCount::exactly(1),
        ),
        Zone::Battlefield,
        false,
        crate::cards::builders::ReturnControllerAst::Preserve,
        false,
        None,
    );
    let grant = EffectAst::subject_verb_grant_abilities_to_target(
        TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), None),
        vec![KeywordAction::Indestructible.into()],
        Until::EndOfTurn,
    );

    assert!(!append_moved_object_entry_followup_to_optional_move(
        &mut previous,
        grant
    ));
}
