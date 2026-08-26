use super::*;
use crate::lexer::lex_line;

fn parse_pair(first: &str, second: &str) -> Vec<EffectAst> {
    let first = lex_line(first, 0).expect("consult sentence should lex");
    let second = lex_line(second, 1).expect("disposition sentence should lex");
    let sentences = [
        SentenceInput::from_lexed(&first),
        SentenceInput::from_lexed(&second),
    ];
    parse_consult_match_move_and_bottom_remainder(&sentences, 0)
        .expect("consult pair should not error")
        .expect("consult pair should parse")
}

#[test]
fn counted_chosen_type_consult_shuffles_only_the_revealed_complement() {
    let effects = parse_pair(
        "Reveal cards from the top of your library until you reveal X creature cards of the chosen type, where X is the number of creatures you control of that type",
        "Put those cards onto the battlefield, then shuffle the rest of the revealed cards into your library",
    );
    let [consult, move_matches, shuffle_remainder] = effects.as_slice() else {
        panic!("expected consult/move/remainder program: {effects:#?}");
    };

    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::ConsultTopOfLibrary {
                filter,
                stop_rule: crate::cards::builders::LibraryConsultStopRuleAst::MatchCount(stop_value),
                all_tag,
                match_tag,
                ..
            },
        ..
    }) = consult
    else {
        panic!("expected counted consult: {consult:#?}");
    };
    let Value::Count(count_filter) = stop_value.unhinted() else {
        panic!("expected a counted object filter: {stop_value:#?}");
    };
    assert!(filter.chosen_creature_type, "{filter:#?}");
    assert_eq!(count_filter.controller, Some(PlayerFilter::You));
    assert!(count_filter.chosen_creature_type, "{count_filter:#?}");

    assert!(matches!(
        move_matches,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::MoveToZone {
                target: TargetAst::Tagged(tag, _),
                zone: Zone::Battlefield,
                target_plural_surface: true,
                ..
            },
            ..
        }) if tag == match_tag
    ));

    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::ShuffleObjectsIntoLibrary {
                target, all: false, ..
            },
        ..
    }) = shuffle_remainder
    else {
        panic!("expected exact revealed remainder shuffle: {shuffle_remainder:#?}");
    };
    let TargetAst::Object(remainder, None, None) = target else {
        panic!("expected a filtered revealed complement: {target:#?}");
    };
    assert!(remainder.tagged_constraints.iter().any(|constraint| {
        constraint.tag == *all_tag && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
    }));
    assert!(remainder.tagged_constraints.iter().any(|constraint| {
        constraint.tag == *match_tag
            && constraint.relation == TaggedOpbjectRelation::IsNotTaggedObject
    }));
}

#[test]
fn ordinary_fixed_count_consult_does_not_gain_a_dynamic_count() {
    let first = lex_line(
        "Reveal cards from the top of your library until you reveal two creature cards",
        0,
    )
    .expect("ordinary consult should lex");
    let parts = parse_consult_traversal_sentence(&first)
        .expect("ordinary consult should not error")
        .expect("ordinary consult should parse");
    assert!(matches!(
        parts.effects.last(),
        Some(EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::ConsultTopOfLibrary {
                stop_rule: crate::cards::builders::LibraryConsultStopRuleAst::MatchCount(
                    Value::Fixed(2),
                ),
                ..
            },
            ..
        }))
    ));
}
