use super::*;
use crate::CounterType;

#[test]
fn owned_graveyard_return_keeps_x_as_an_inline_entry_counter() {
    let tokens = crate::lexer::lex_line(
            "Return target artifact or non-Aura enchantment card from your graveyard to the battlefield with X additional +1/+1 counters on it.",
            0,
        )
        .expect("dynamic return should lex");
    let effects =
        parse_return_with_dynamic_entry_counters_sentence(SubjectVerbPrimitiveClause::new(&tokens))
            .expect("dynamic return should parse")
            .expect("dynamic return shape");
    let [returned, counter] = effects.as_slice() else {
        panic!("expected return and entry-counter effects: {effects:#?}");
    };
    assert!(matches!(
        returned,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::ReturnToBattlefield {
                target: TargetAst::Object(filter, ..),
                ..
            },
            ..
        }) if filter.zone == Some(Zone::Graveyard)
            && filter.owner == Some(PlayerFilter::You)
    ));
    assert!(matches!(
        counter,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::PutCounters {
                counter_type: CounterType::PlusOnePlusOne,
                count,
                target: TargetAst::Tagged(tag, _),
                ..
            },
            ..
        }) if matches!(count.unhinted(), Value::X)
            && count.has_surface_hint(ironsmith_core::ValueSurfaceHint::InlineBattlefieldEntryCounter)
            && count.has_surface_hint(ironsmith_core::ValueSurfaceHint::AdditionalEntryCounter)
            && tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
    ));
}

#[test]
fn a_return_from_an_opponents_graveyard_is_not_rebound_to_you() {
    let tokens = crate::lexer::lex_line(
            "Return target artifact card from an opponent's graveyard to the battlefield with X additional +1/+1 counters on it.",
            0,
        )
        .expect("near miss should lex");
    assert!(
        parse_return_with_dynamic_entry_counters_sentence(SubjectVerbPrimitiveClause::new(&tokens))
            .expect("near miss should not error")
            .is_none()
    );
}
