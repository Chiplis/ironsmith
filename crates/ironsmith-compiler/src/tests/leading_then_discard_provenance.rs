use super::*;

fn compile_triggered_program(text: &str) -> crate::resolution::ResolutionProgram {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Leading Then Provenance")
        .card_types(vec![CardType::Creature])
        .parse_text(text)
        .expect("the triggered draw/discard ability should compile");
    definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered.effects.clone()),
            _ => None,
        })
        .expect("the card should contain a triggered ability")
}

fn draw_and_conditional_discard(
    program: &crate::resolution::ResolutionProgram,
) -> (
    &crate::effects::DrawCardsEffect,
    &crate::effects::ConditionalEffect,
    &crate::effects::DiscardEffect,
) {
    let [draw_segment, discard_segment] = program.segments.as_slice() else {
        panic!("expected exactly two source-sentence segments: {program:#?}");
    };
    let [draw_effect] = draw_segment.default_effects.as_slice() else {
        panic!("expected a single draw effect: {draw_segment:#?}");
    };
    let draw = draw_effect
        .downcast_ref::<crate::effects::DrawCardsEffect>()
        .expect("the first source sentence should draw cards");
    let [sequence_effect] = discard_segment.default_effects.as_slice() else {
        panic!("expected a single leading-then sequence: {discard_segment:#?}");
    };
    let sequence = sequence_effect
        .downcast_ref::<crate::effects::SequenceEffect>()
        .expect("the second source sentence should retain its leading-then surface");
    assert_eq!(
        sequence.surface,
        ironsmith_core::SequenceSurface::SentenceLeadingThen
    );
    let [conditional_effect] = sequence.effects.as_slice() else {
        panic!("expected one conditional inside the leading-then sequence: {sequence:#?}");
    };
    let conditional = conditional_effect
        .downcast_ref::<crate::effects::ConditionalEffect>()
        .expect("the leading-then sequence should contain a conditional");
    let [discard_effect] = conditional.if_true.as_slice() else {
        panic!("expected one discard effect in the true branch: {conditional:#?}");
    };
    let discard = discard_effect
        .downcast_ref::<crate::effects::DiscardEffect>()
        .expect("the true branch should discard");
    (draw, conditional, discard)
}

#[test]
fn leading_then_implicit_discard_keeps_the_explicit_you_actor_after_damage_trigger() {
    let program = compile_triggered_program(
        "Whenever this creature deals combat damage to a player, draw a card. Then discard a card unless there are seven or more cards in your graveyard.",
    );
    let (draw, _, discard) = draw_and_conditional_discard(&program);

    assert_eq!(draw.player, crate::target::PlayerFilter::You);
    assert_eq!(discard.player, crate::target::PlayerFilter::You);
}

#[test]
fn spell_cast_trigger_resolution_condition_refers_to_the_triggering_spell() {
    let program = compile_triggered_program(
        "Whenever you cast an instant or sorcery spell, draw a card. Then discard a card unless five or more mana was spent to cast that spell.",
    );
    let (draw, conditional, discard) = draw_and_conditional_discard(&program);

    assert_eq!(draw.player, crate::target::PlayerFilter::You);
    assert_eq!(discard.player, crate::target::PlayerFilter::You);
    assert!(matches!(
        &conditional.condition,
        crate::effect::Condition::Not(inner)
            if matches!(
                inner.as_ref(),
                crate::effect::Condition::TriggeringSpellManaSpentToCastAtLeast {
                    amount: 5,
                    symbol: None,
                }
            )
    ));
}
