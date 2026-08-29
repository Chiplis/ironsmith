use super::*;
use crate::lexer::lex_line;

#[test]
fn splits_control_target_duration_and_delay() {
    let tokens = lex_line(
        "control of target creature until end of turn at the end of combat",
        0,
    )
    .unwrap();
    let shape = parse_gain_control_clause_shape(&tokens).unwrap();
    assert_eq!(
        TokenWordView::new(shape.target_tokens).to_word_refs(),
        vec!["target", "creature"]
    );
    assert!(shape.delayed_until_end_of_combat);
    assert_eq!(
        parse_control_duration_shape(shape.duration_tokens),
        Some(ControlDurationAst::UntilEndOfTurn)
    );

    let tapped = lex_line(
        "for as long as you control this creature and this creature remains tapped",
        0,
    )
    .unwrap();
    assert_eq!(
        parse_permanent_control_duration_shape(&tapped)
            .unwrap()
            .until,
        Until::ForAsLongAs(ironsmith_core::ContinuousDurationPredicate::all([
            ironsmith_core::ContinuousDurationPredicate::ObjectControlledBy {
                object: ironsmith_core::ContinuousDurationObject::Source,
                player: ironsmith_core::ContinuousDurationPlayer::EffectController,
            },
            ironsmith_core::ContinuousDurationPredicate::ObjectTapped(
                ironsmith_core::ContinuousDurationObject::Source,
            ),
        ]))
    );
}

#[test]
fn parses_named_source_compound_control_duration_with_surface() {
    let text =
        "for as long as you control Rubinia Soulsinger and Rubinia Soulsinger remains tapped";
    let tokens = lex_line(text, 0).unwrap();
    let context = crate::parse_context::ParseContext::for_fragment(
        "Rubinia Soulsinger",
        Vec::new(),
        Vec::new(),
        text,
    );
    let shape =
        parse_permanent_control_duration_shape_with_context(context.view(), &tokens).unwrap();
    assert!(matches!(shape.until, Until::ForAsLongAs(_)));
    assert_eq!(
        shape.source_surface,
        Some(SourceReferenceSurface::FullName(
            "Rubinia Soulsinger".to_string()
        ))
    );
}

#[test]
fn parses_typed_latched_control_duration_predicates() {
    use ironsmith_core::{
        ContinuousDurationObject as ObjectRef, ContinuousDurationPlayer as PlayerRef,
        ContinuousDurationPredicate as Predicate,
    };

    let parse = |text| {
        let tokens = lex_line(text, 0).unwrap();
        parse_permanent_control_duration_shape(&tokens)
            .expect("predicate-bearing duration should parse")
            .until
    };
    assert_eq!(
        parse("for as long as it has a shield counter on it"),
        Until::ForAsLongAs(Predicate::affected_object_has_counter(
            crate::object::CounterType::Shield,
        ))
    );
    assert_eq!(
        parse("for as long as that creature is enchanted"),
        Until::ForAsLongAs(Predicate::ObjectIsEnchanted(ObjectRef::AffectedObject,))
    );
    assert_eq!(
        parse("for as long as they're the monarch"),
        Until::ForAsLongAs(Predicate::PlayerIsMonarch(PlayerRef::ControllerOf(
            ObjectRef::AffectedObject
        ),))
    );
    assert_eq!(
        parse("for as long as that Aura is attached to it"),
        Until::ForAsLongAs(Predicate::ObjectAttachedTo {
            attachment: ObjectRef::Tagged(crate::tag::CompilerReferenceTag::Triggering.key()),
            attached_to: ObjectRef::AffectedObject,
        })
    );
    assert_eq!(
        parse(
            "for as long as this creature remains tapped and that creature's power remains less than or equal to this creature's power",
        ),
        Until::ForAsLongAs(Predicate::all([
            Predicate::ObjectTapped(ObjectRef::Source),
            Predicate::ObjectPowerAtMostObject {
                lesser: ObjectRef::AffectedObject,
                greater: ObjectRef::Source,
            },
        ]))
    );
}
