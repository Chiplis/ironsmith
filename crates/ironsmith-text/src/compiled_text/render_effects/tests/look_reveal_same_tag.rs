use super::*;

fn look(count: i32, tag: TagKey) -> Effect {
    Effect::new(crate::effects::LookAtTopCardsEffect::new(
        PlayerFilter::You,
        Value::Fixed(count),
        tag,
    ))
}

fn reveal(tag: TagKey) -> Effect {
    Effect::new(crate::effects::RevealTaggedEffect::new(tag))
}

#[test]
fn adjacent_reveal_of_same_looked_collection_uses_public_reveal_surface() {
    let looked = TagKey::from("looked");
    assert_eq!(
        describe_effect_list(&[look(7, looked.clone()), reveal(looked)]),
        "Reveal the top seven cards of your library"
    );
}

#[test]
fn adjacent_reveal_with_different_tag_does_not_claim_looked_cards_were_revealed() {
    let rendered = describe_effect_list(&[
        look(7, TagKey::from("looked")),
        reveal(TagKey::from("other")),
    ]);
    assert!(rendered.contains("Look at the top seven cards"), "{rendered}");
    assert!(rendered.contains("Reveal it"), "{rendered}");
}

#[test]
fn already_public_look_does_not_consume_later_reveal() {
    let looked = TagKey::from("looked");
    let public_look = Effect::new(crate::effects::LookAtTopCardsEffect::revealing(
        PlayerFilter::You,
        Value::Fixed(1),
        looked.clone(),
    ));
    let rendered = describe_effect_list(&[public_look, reveal(looked)]);
    assert!(rendered.contains("Reveal the top card"), "{rendered}");
    assert!(rendered.contains("Reveal it"), "{rendered}");
}

#[test]
fn intervening_effect_keeps_look_and_reveal_separate() {
    let looked = TagKey::from("looked");
    let rendered = describe_effect_list(&[
        look(3, looked.clone()),
        Effect::new(crate::effects::DrawCardsEffect::you(1)),
        reveal(looked),
    ]);
    assert!(rendered.contains("Look at the top three cards"), "{rendered}");
    assert!(rendered.contains("Reveal it"), "{rendered}");
}
