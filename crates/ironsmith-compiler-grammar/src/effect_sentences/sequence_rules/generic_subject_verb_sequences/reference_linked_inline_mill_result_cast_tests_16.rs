use super::*;
use crate::lexer::lex_line;

fn parse_pair(second: &str) -> Option<Vec<EffectAst>> {
    let first = lex_line("Target opponent mills five cards.", 0).expect("mill sentence");
    let second = lex_line(second, 1).expect("cast sentence");
    let sentences = [
        SentenceInput::from_lexed(&first),
        SentenceInput::from_lexed(&second),
    ];
    crate::effect_sentences::sequence_rules::try_parse_document_program(&sentences, 0)
        .map(|matched| matched.map(|matched| matched.effects)).expect("pair parser")
}

#[test]
fn choice_is_optional_and_scoped_to_the_milled_result() {
    let effects = parse_pair(
        "You may cast an instant or sorcery spell from among them without paying its mana cost.",
    )
    .expect("exact mill-result cast pair");
    let mill_tag = effects.iter().find_map(|effect| match effect {
        EffectAst::TagAffected { tag, .. } => Some(tag),
        _ => None,
    });
    let Some(mill_tag) = mill_tag else {
        panic!("mill result must be tagged: {effects:#?}");
    };
    let choose = effects.iter().find_map(|effect| match effect {
        EffectAst::ChooseTaggedObjectsInZone {
            filter,
            count,
            tag,
            zone,
            ..
        } => Some((filter, count, tag, zone)),
        _ => None,
    });
    let Some((filter, count, chosen_tag, zone)) = choose else {
        panic!("milled-card choice missing: {effects:#?}");
    };
    assert_eq!(*count, ChoiceCount::up_to(1));
    assert_eq!(*zone, Zone::Graveyard);
    assert_eq!(
        filter.card_types,
        vec![CardType::Instant, CardType::Sorcery]
    );
    assert!(filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag == *mill_tag && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
    }));
    assert!(effects.iter().any(|effect| matches!(
        effect,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::CastTagged {
                tag,
                without_paying_mana_cost: true,
                ..
            },
            ..
        }) if tag == chosen_tag
    )));
}

#[test]
fn dynamic_mana_value_cap_is_shared_by_both_spell_types() {
    let effects = parse_pair(
            "You may cast an instant or sorcery spell with mana value X or less from among them without paying its mana cost.",
        )
        .expect("exact capped mill-result cast pair");
    let filter = effects.iter().find_map(|effect| match effect {
        EffectAst::ChooseTaggedObjectsInZone { filter, .. } => Some(filter),
        _ => None,
    });
    let filter = filter.expect("milled-card choice");
    assert_eq!(filter.zone, Some(Zone::Graveyard));
    assert_eq!(filter.any_of.len(), 2, "{filter:#?}");
    assert!(filter.any_of.iter().all(|branch| matches!(
        branch.mana_value.as_ref(),
        Some(crate::filter::Comparison::LessThanOrEqualExpr(value))
            if value.as_ref() == &Value::X
    )));
}

#[test]
fn ordinary_graveyard_cast_is_not_claimed_as_a_mill_result() {
    assert!(
            parse_pair(
                "You may cast an instant or sorcery spell from your graveyard without paying its mana cost."
            )
            .is_none()
        );
}
