use super::*;

fn draw_then_discard_unless_program(
    draw_count: Value,
    draw_player: PlayerFilter,
    discard_count: Value,
    discard_player: PlayerFilter,
    positive_condition: Condition,
) -> crate::resolution::ResolutionProgram {
    crate::resolution::ResolutionProgram::new(vec![
        crate::resolution::ResolutionSegment::from_effects(vec![Effect::new(
            crate::effects::DrawCardsEffect::new(draw_count, draw_player),
        )]),
        crate::resolution::ResolutionSegment::from_effects(vec![Effect::new(
            crate::effects::SequenceEffect::sentence_leading_then(vec![Effect::new(
                crate::effects::ConditionalEffect::new(
                    Condition::Not(Box::new(positive_condition)),
                    vec![Effect::new(crate::effects::DiscardEffect::new(
                        discard_count,
                        discard_player,
                        false,
                    ))],
                    Vec::new(),
                ),
            )]),
        )]),
    ])
}

#[test]
fn adjacent_source_sentences_render_draw_then_discard_unless_with_exact_counts() {
    let program = draw_then_discard_unless_program(
        Value::Fixed(2),
        PlayerFilter::You,
        Value::Fixed(1),
        PlayerFilter::You,
        Condition::AttackedThisTurn,
    );

    assert_eq!(
        super::super::ast_render::describe_resolution_program(&program),
        "Draw two cards. Then discard a card unless you attacked this turn"
    );
}

#[test]
fn adjacent_source_sentences_do_not_fold_when_the_discard_actor_differs() {
    let program = draw_then_discard_unless_program(
        Value::Fixed(1),
        PlayerFilter::You,
        Value::Fixed(1),
        PlayerFilter::DamagedPlayer,
        Condition::AttackedThisTurn,
    );
    let rendered = super::super::ast_render::describe_resolution_program(&program);

    assert_ne!(
        rendered,
        "Draw a card. Then discard a card unless you attacked this turn"
    );
    assert!(
        rendered.contains("that player discards a card"),
        "{rendered}"
    );
}

#[test]
fn spell_cast_condition_keeps_triggering_spell_surface_in_unless_clause() {
    let program = draw_then_discard_unless_program(
        Value::Fixed(1),
        PlayerFilter::You,
        Value::Fixed(1),
        PlayerFilter::You,
        Condition::TriggeringSpellManaSpentToCastAtLeast {
            amount: 5,
            symbol: None,
        },
    );

    assert_eq!(
        super::super::ast_render::describe_resolution_program(&program),
        "Draw a card. Then discard a card unless five or more mana was spent to cast that spell"
    );
}

#[test]
fn graveyard_threshold_condition_keeps_there_are_surface_in_unless_clause() {
    let program = draw_then_discard_unless_program(
        Value::Fixed(1),
        PlayerFilter::You,
        Value::Fixed(1),
        PlayerFilter::You,
        Condition::ValueComparison {
            left: Value::Count(
                ObjectFilter::default()
                    .in_zone(Zone::Graveyard)
                    .owned_by(PlayerFilter::You),
            ),
            operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
            right: Value::Fixed(7),
        },
    );

    assert_eq!(
        super::super::ast_render::describe_resolution_program(&program),
        "Draw a card. Then discard a card unless there are seven or more cards in your graveyard"
    );
}
