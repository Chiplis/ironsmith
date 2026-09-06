use super::*;
use crate::lexer::lex_line;

#[test]
fn may_put_keeps_an_exact_choice_inside_the_optional_action() {
    let lexed = [
            lex_line("Look at the top seven cards of your library.", 0).unwrap(),
            lex_line(
                "You may put a permanent card with mana value 3 or less from among them onto the battlefield with a shield counter on it.",
                1,
            )
            .unwrap(),
            lex_line(
                "Put the rest on the bottom of your library in a random order.",
                2,
            )
            .unwrap(),
        ];
    let sentences = lexed
        .iter()
        .map(|tokens| SentenceInput::from_lexed(tokens))
        .collect::<Vec<_>>();
    let second_tokens = trim_commas(sentences[1].lowered());
    let action = sentence_markers::parse_leading_may_action_tokens(&second_tokens, &["put"], false)
        .expect("optional put action marker");
    let move_shape = triple_grammar::parse_looked_move_action_shape(action.tail_tokens)
        .expect("looked-card battlefield move shape");
    assert_eq!(
        move_shape.entry_counter,
        Some((1, crate::CounterType::Shield))
    );
    let counted = parse_counted_from_looked_cards_action(action.tail_tokens)
        .expect("looked-card counted action");
    assert!(counted.0.is_single());
    assert_eq!(counted.2, None);
    assert_eq!(counted.3, Zone::Battlefield);
    assert!(!counted.5 && !counted.6 && counted.7.is_none() && !counted.8);
    assert!(matches!(
        triple_grammar::parse_looked_remainder_shape(sentences[2].lowered()),
        Some(triple_grammar::LookedRemainderShape::LibraryBottom(_))
    ));
    let effects = parse_look_at_top_may_put_with_counter_then_rest_bottom(&sentences, 0)
        .unwrap()
        .expect("optional looked-card entry");
    assert!(
        matches!(
            effects.as_slice(),
            [_, EffectAst::Permissions(PermissionEffectAst::May { effects: optional }), _]
                if matches!(optional.as_slice(), [
                    EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseTaggedObjectsInZone {
                        count,
                        filter,
                        ..
                    }),
                    EffectAst::ForEach(ForEachEffectAst::ForEachTagged { .. })
                ] if count.is_single()
                    && filter.card_types.contains(&CardType::Artifact)
                    && filter.card_types.contains(&CardType::Creature))
        ),
        "{effects:#?}"
    );
}
