use super::*;
use crate::lexer::lex_line;

#[test]
fn optional_cast_chooses_from_the_exact_target_opponents_revealed_hand() {
    let first = lex_line("Target opponent reveals their hand.", 0).unwrap();
    let second = lex_line(
            "You may cast an instant or sorcery spell from among those cards without paying its mana cost.",
            1,
        )
        .unwrap();
    let sentences = [
        SentenceInput::from_lexed(&first),
        SentenceInput::from_lexed(&second),
    ];
    let effects = crate::effect_sentences::sequence_rules::try_parse_document_program(&sentences, 0)
        .map(|matched| matched.map(|matched| matched.effects))
        .expect("pair parser should not error")
        .expect("revealed-hand optional cast should match");

    let [_, EffectAst::May { effects: optional }] = effects.as_slice() else {
        panic!("expected reveal plus one optional program: {effects:#?}");
    };
    let [
        EffectAst::ChooseTaggedObjectsInZone {
            filter,
            count,
            player: PlayerAst::You,
            tag: chosen_tag,
            zone: Zone::Hand,
        },
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::CastTagged {
                    tag: cast_tag,
                    player: PlayerAst::You,
                    allow_land: false,
                    as_copy: false,
                    copy_cast_reminder_surface: false,
                    copy_instruction_surface: None,
                    without_paying_mana_cost: true,
                    additional_mana_cost: None,
                    cost_reduction: None,
                    mana_spend_mode: ironsmith_core::value_model::ManaSpendMode::Normal,
                },
            ..
        }),
    ] = optional.as_slice()
    else {
        panic!("expected exact choice/cast optional program: {optional:#?}");
    };
    assert_eq!(*count, ChoiceCount::exactly(1));
    assert_eq!(chosen_tag, cast_tag);
    assert_eq!(filter.zone, Some(Zone::Hand));
    assert_eq!(
        filter.owner,
        Some(PlayerFilter::AliasedTarget(Box::new(
            PlayerFilter::Opponent
        )))
    );
    assert_eq!(filter.card_types, [CardType::Instant, CardType::Sorcery]);
    assert!(filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag.as_str() == crate::tag::CompilerReferenceTag::RevealedThisWay.as_str()
            && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
    }));
}
