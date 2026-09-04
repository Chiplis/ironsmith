use super::*;
use crate::lexer::lex_line;

fn lex_inputs(first: &str, second: &str) -> [Vec<OwnedLexToken>; 2] {
    [
        lex_line(first, 0).expect("first sentence should lex"),
        lex_line(second, 1).expect("second sentence should lex"),
    ]
}

#[test]
fn looked_players_hand_optional_free_cast_keeps_zone_owner_and_may_semantics() {
    let lexed = lex_inputs(
        "Look at that player's hand.",
        "You may cast a spell from among those cards without paying its mana cost.",
    );
    let sentences = [
        SentenceInput::from_lexed(&lexed[0]),
        SentenceInput::from_lexed(&lexed[1]),
    ];
    let effects = crate::effect_sentences::sequence_rules::try_parse_document_program(&sentences, 0)
        .map(|matched| matched.map(|matched| matched.effects))
        .expect("pair parser should not error")
        .expect("looked-hand optional cast should match");

    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::LookAtHand {
                    target:
                        TargetAst::Player(
                            PlayerFilter::DamagedPlayer
                            | PlayerFilter::IteratedPlayer
                            | PlayerFilter::Target(_)
                            | PlayerFilter::AliasedTarget(_),
                            _,
                        ),
                },
            ..
        }),
        EffectAst::MayCastMatchingSpellWithoutPayingManaCost {
            player: PlayerAst::You,
            zone_owner: PlayerAst::That,
            filter,
            zone: Zone::Hand,
            payment: ironsmith_core::MayCastMatchingSpellPayment::WithoutPayingManaCost,
        },
    ] = effects.as_slice()
    else {
        panic!("expected looked hand plus typed optional hand cast: {effects:#?}");
    };
    assert_eq!(filter.zone, Some(Zone::Hand));
    assert_eq!(filter.excluded_card_types, [CardType::Land]);
}

#[test]
fn public_two_sentence_route_keeps_looked_hand_cast_optional() {
    let tokens = lex_line(
            "Look at that player's hand. You may cast a spell from among those cards without paying its mana cost.",
            0,
        )
        .expect("look-and-cast sentence pair should lex");
    let effects = effect_sentences::parse_effect_sentences_lexed(&tokens)
        .expect("look-and-cast sentence pair should parse");

    assert!(
        matches!(
            effects.as_slice(),
            [
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::LookAtHand { .. },
                    ..
                }),
                EffectAst::MayCastMatchingSpellWithoutPayingManaCost {
                    player: PlayerAst::You,
                    zone_owner: PlayerAst::That,
                    filter,
                    zone: Zone::Hand,
                    payment: ironsmith_core::MayCastMatchingSpellPayment::WithoutPayingManaCost,
                }
            ] if filter.zone == Some(Zone::Hand)
                && filter.excluded_card_types == [CardType::Land]
        ),
        "public route must preserve the optional cast from the looked player's hand: {effects:#?}"
    );
}

#[test]
fn looked_hand_pair_does_not_claim_unrelated_or_nonoptional_casts() {
    let wrong_reference_lexed = lex_inputs(
        "Look at that player's hand.",
        "You may cast a spell from your hand without paying its mana cost.",
    );
    let wrong_reference = [
        SentenceInput::from_lexed(&wrong_reference_lexed[0]),
        SentenceInput::from_lexed(&wrong_reference_lexed[1]),
    ];
    assert!(
        crate::effect_sentences::sequence_rules::try_parse_document_program(&wrong_reference, 0)
        .map(|matched| matched.map(|matched| matched.effects))
            .unwrap()
            .is_none()
    );

    let mandatory_lexed = lex_inputs(
        "Look at that player's hand.",
        "Cast a spell from among those cards without paying its mana cost.",
    );
    let mandatory = [
        SentenceInput::from_lexed(&mandatory_lexed[0]),
        SentenceInput::from_lexed(&mandatory_lexed[1]),
    ];
    assert!(
        crate::effect_sentences::sequence_rules::try_parse_document_program(&mandatory, 0)
        .map(|matched| matched.map(|matched| matched.effects))
            .unwrap()
            .is_none()
    );
}
