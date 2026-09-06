use super::*;

fn contains_plural_retarget(effect: &EffectAst) -> bool {
    if matches!(
        effect,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Stack(StackActionAst::RetargetStackObject {
                copy_reference_plural: true,
                ..
            }),
            ..
        })
    ) {
        return true;
    }
    let mut found = false;
    for_each_nested_effects(effect, true, |nested| {
        found |= nested.iter().any(contains_plural_retarget);
    });
    found
}

#[test]
fn optional_copy_retarget_stays_inside_repeating_delayed_trigger() {
    let tokens = crate::lexer::lex_line(
            "Choose a planeswalker type. Until end of turn, whenever you activate an ability of a planeswalker of that type, copy that ability. You may choose new targets for the copies.",
            0,
        )
        .expect("repeating delayed-copy procedure should lex");
    let parsed = parse_effect_sentences_lexed(&tokens)
        .expect("repeating delayed-copy procedure should parse");
    let [EffectAst::SubjectVerb(_), delayed] = parsed.as_slice() else {
        panic!("retarget must not remain on the outer program: {parsed:#?}");
    };
    let delayed_effects = match delayed {
        EffectAst::Delayed(DelayedEffectAst::DelayedTriggerThisTurn { effects, .. })
        | EffectAst::Delayed(DelayedEffectAst::DelayedTriggerForDuration { effects, .. }) => effects,
        _ => panic!("expected a repeating delayed trigger: {delayed:#?}"),
    };
    assert!(effects_copy_a_stack_object(delayed_effects));
    assert!(delayed_effects.iter().any(contains_plural_retarget));
}

#[test]
fn retarget_does_not_attach_to_a_delayed_trigger_that_creates_no_copy() {
    let tokens = crate::lexer::lex_line(
            "Until end of turn, whenever you draw a card, draw a card. You may choose new targets for the copies.",
            0,
        )
        .expect("noncopy delayed-trigger near miss should lex");
    let parsed = parse_effect_sentences_lexed(&tokens)
        .expect("noncopy delayed-trigger near miss should parse");
    assert_eq!(parsed.len(), 2, "{parsed:#?}");
    let delayed_effects = match &parsed[0] {
        EffectAst::Delayed(DelayedEffectAst::DelayedTriggerThisTurn { effects, .. })
        | EffectAst::Delayed(DelayedEffectAst::DelayedTriggerForDuration { effects, .. }) => effects,
        effect => panic!("expected delayed near miss: {effect:#?}"),
    };
    assert!(!delayed_effects.iter().any(contains_plural_retarget));
    assert!(contains_plural_retarget(&parsed[1]));
}

#[test]
fn fixed_copy_target_stays_inside_the_optional_copy_branch() {
    let tokens = crate::lexer::lex_line("You may copy that spell. The copy targets Ivy.", 0)
        .expect("optional copy procedure should lex");
    let parsed =
        parse_effect_sentences_lexed(&tokens).expect("optional copy procedure should parse");
    let [optional] = parsed.as_slice() else {
        panic!("the fixed retarget must not remain an outer sibling: {parsed:#?}");
    };
    let optional_effects = match optional {
        EffectAst::Permissions(PermissionEffectAst::May { effects })
        | EffectAst::Permissions(PermissionEffectAst::MayByPlayer {
            player: PlayerAst::You | PlayerAst::Implicit,
            effects,
        }) => effects,
        _ => panic!("expected one optional copy owner: {optional:#?}"),
    };
    assert!(effects_copy_a_stack_object(optional_effects));
    assert!(effects_are_one_copy_retarget_followup(
        &optional_effects[optional_effects.len() - 1..]
    ));
}

#[test]
fn unconditional_copy_does_not_acquire_an_optional_owner() {
    let tokens = crate::lexer::lex_line("Copy that spell. The copy targets this creature.", 0)
        .expect("unconditional copy procedure should lex");
    let parsed =
        parse_effect_sentences_lexed(&tokens).expect("unconditional copy procedure should parse");
    assert_eq!(parsed.len(), 2, "{parsed:#?}");
    assert!(!matches!(
        parsed[0],
        EffectAst::Permissions(PermissionEffectAst::May { .. }) | EffectAst::Permissions(PermissionEffectAst::MayByPlayer { .. })
    ));
}
