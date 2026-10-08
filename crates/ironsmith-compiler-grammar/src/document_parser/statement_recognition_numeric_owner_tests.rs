use super::*;
use crate::ids::CardId;
use crate::types::CardType;

const TROLL: &str = "{3}{G}: Roll a d20. Activate only if this card is in your graveyard.\n1—9 | Put this card on top of your library.\n10—19 | Return this card to your hand.\n20 | Return this card to the battlefield tapped.";

#[test]
fn whole_troll_body_retains_cost_restriction_and_all_result_rows_in_one_activation() {
    let preprocessed = preprocess_document(
        CardBuilder::new(CardId::new(), "Loathsome Troll").card_types(vec![CardType::Creature]),
        TROLL,
    ).unwrap();
    let recognized = recognize_document(&preprocessed, false).unwrap();
    let [RecognizedLine::Activated(ability)] = recognized.lines.as_slice() else {
        panic!("result rows escaped the activation: {:?}", recognized.lines);
    };
    assert_eq!(render_token_slice(&ability.cost_parse_tokens).replace(' ', "").to_ascii_lowercase(), "{3}{g}");
    let body = render_token_slice(&ability.effect_parse_tokens);
    for fragment in ["roll a d20", "activate only if this card is in your graveyard", "1—9", "10—19", "20 |", "battlefield tapped"] {
        assert!(body.contains(fragment), "missing {fragment}: {body}");
    }
    assert!(!body.contains("{3}"), "cost must remain outside resolving body: {body}");
}

#[test]
fn restrictions_are_transparent_to_numeric_ownership_only_inside_activations() {
    for restriction in ["Activate only as a sorcery.", "Activate only once each turn.", "Activate only if this card is in your graveyard."] {
        for label in ["", "Fortune — "] {
            let text = format!("{label}{{3}}{{G}}: Roll a d20. {restriction}\n1—9 | Draw a card.\n10—20 | You gain 2 life.\n{{T}}: Add {{G}}.");
            let preprocessed = preprocess_document(
                CardBuilder::new(CardId::new(), "Restricted roll").card_types(vec![CardType::Artifact]), &text,
            ).unwrap();
            let recognized = recognize_document(&preprocessed, false).unwrap();
            let [RecognizedLine::Activated(table), RecognizedLine::Activated(next)] = recognized.lines.as_slice() else {
                panic!("separate activation boundary lost: {:?}", recognized.lines);
            };
            let body = render_token_slice(&table.effect_parse_tokens);
            assert!(body.contains("activate only") && body.contains("1—9") && body.contains("10—20"), "{body}");
            assert!(!body.contains("add"));
            assert!(render_token_slice(&next.effect_parse_tokens).contains("add"));
        }
    }
    for head in [
        "{0}: Draw a card.",
        "{0}: Roll a d20. Draw a card.",
        "{0}: You may roll a d20.",
        "{0}: If you control a creature, roll a d20.",
        "Roll a d20.",
        "When this artifact enters, roll a d20.",
    ] {
        let text = format!("{head} Activate only once each turn.\n1—20 | Draw a card.");
        let preprocessed = preprocess_document(
            CardBuilder::new(CardId::new(), "No immediate activation roll").card_types(vec![CardType::Artifact]), &text,
        ).unwrap();
        assert!(recognize_document(&preprocessed, false).is_err(), "{text}");
    }
}
