use super::*;
use crate::types::Subtype;
use crate::{lex_line, split_lexed_sentences};

#[test]
fn later_wall_subset_reuses_the_exact_multi_target_set() {
    let tokens = lex_line(
        "Up to three target creatures can't block this turn. Destroy any of them that are Walls.",
        0,
    )
    .expect("targeted subset probe should lex");
    let split = split_lexed_sentences(&tokens);
    let sentences = split
        .iter()
        .map(|tokens| SentenceInput::from_lexed(tokens))
        .collect::<Vec<_>>();
    let effects = parse_multi_target_restriction_then_destroy_typed_subset(&sentences, 0)
        .expect("targeted subset parser should not error")
        .expect("targeted subset shape should match");

    let [
        EffectAst::TagAffected {
            effect: target_effect,
            tag: target_tag,
        },
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Cant {
                    restriction: crate::effect::Restriction::Block(restricted),
                    ..
                },
            ..
        }),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Destroy {
                    target: TargetAst::Object(destroyed, _, _),
                    ..
                },
            ..
        }),
    ] = effects.as_slice()
    else {
        panic!("expected target/restrict/subset-destroy AST: {effects:#?}");
    };
    assert!(matches!(
        target_effect.as_ref(),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::TargetOnly {
                    target: TargetAst::WithCount(_, count),
                    explicit_declaration: false,
                },
            ..
        }) if count == &ChoiceCount::up_to(3)
    ));
    assert!(restricted.tagged_constraints.iter().any(|constraint| {
        constraint.tag == *target_tag
            && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
    }));
    assert!(destroyed.subtypes.contains(&Subtype::Wall));
    assert!(destroyed.tagged_constraints.iter().any(|constraint| {
        constraint.tag == *target_tag
            && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
    }));
}
