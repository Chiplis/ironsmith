use super::*;

fn leaves_filter(trigger: &crate::cards::builders::TriggerSpec) -> Option<&ObjectFilter> {
    match trigger {
        crate::cards::builders::TriggerSpec::WithIntro { trigger, .. } => leaves_filter(trigger),
        crate::cards::builders::TriggerSpec::LeavesBattlefield(filter) => Some(filter),
        _ => None,
    }
}

#[test]
fn targeted_creature_leave_watcher_reuses_delayed_target_choice() {
    let lexed = crate::lexer::lex_line(
            "Whenever target creature deals combat damage to a non-Wall creature this turn, destroy that non-Wall creature. When the targeted creature leaves the battlefield this turn, sacrifice this artifact.",
            0,
        )
        .expect("linked delayed triggers should lex");
    let parsed =
        parse_effect_sentences_lexed(&lexed).expect("linked delayed triggers should parse");
    let chosen_tag = parsed
        .iter()
        .find_map(|effect| match effect {
            EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects { tag, .. }) => Some(tag),
            _ => None,
        })
        .expect("the target creature should be selected at resolution");
    let leave_filter = parsed
        .iter()
        .find_map(|effect| match effect {
            EffectAst::Delayed(DelayedEffectAst::DelayedTriggerThisTurn { trigger, .. }) => leaves_filter(trigger),
            _ => None,
        })
        .expect("the later sentence should register a leave watcher");

    assert!(
        leave_filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag == **chosen_tag
                && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
        }),
        "the leave watcher must be restricted to the chosen target: {parsed:#?}"
    );
}
