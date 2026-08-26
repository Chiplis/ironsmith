use super::*;
use crate::lexer::{lex_line, render_token_slice};

#[test]
fn parses_return_destination_counters_and_timing() {
    let tokens = lex_line(
            "Return target creature card from your graveyard to the battlefield tapped under its owner's control with two +1/+1 counters and a flying counter on it at the beginning of the next end step.",
            0,
        )
        .unwrap();
    let shape = parse_return_with_counters_tokens(&tokens).unwrap();
    assert_eq!(shape.destination.controller, ReturnControllerAst::Owner);
    assert!(shape.destination.tapped);
    assert_eq!(shape.descriptors.len(), 2);
    assert_eq!(shape.descriptors[0].count, 2);
    assert_eq!(
        render_token_slice(shape.target_tokens),
        "target creature card from your graveyard"
    );
    assert!(matches!(
        shape.timing,
        Some(CounterMarkerTimingShape::NextEndStep(PlayerFilter::Any))
    ));

    let attacking = lex_line(
            "Return target creature card from your graveyard to the battlefield tapped and attacking with a finality counter on it.",
            0,
        )
        .unwrap();
    let shape = parse_return_with_counters_tokens(&attacking).unwrap();
    assert!(shape.destination.tapped);
    assert!(shape.destination.attacking);
    assert!(!shape.destination.transformed);
    assert_eq!(shape.descriptors.len(), 1);
    assert_eq!(shape.descriptors[0].counter_type, CounterType::Finality);
    assert_eq!(
        render_token_slice(shape.target_tokens),
        "target creature card from your graveyard"
    );
}

#[test]
fn parses_put_and_tagged_additional_shapes() {
    let put = lex_line(
            "Put a permanent card from among them onto the battlefield with an additional +1/+1 counter on it.",
            0,
        )
        .unwrap();
    let shape = parse_put_with_additional_tokens(&put).unwrap();
    assert!(shape.descriptor.additional);
    assert_eq!(shape.descriptor.counter_type, CounterType::PlusOnePlusOne);

    let tagged = lex_line(
        "Each of them enters with an additional -1/-1 counter on it.",
        0,
    )
    .unwrap();
    let shape = parse_tagged_enters_additional_tokens(&tagged).unwrap();
    assert_eq!(shape.descriptor.count, 1);

    let conditional = lex_line(
            "Each of them enters with an additional +1/+1 counter on it if it's a creature and an additional loyalty counter on it if it's a planeswalker.",
            0,
        )
        .unwrap();
    let shape = parse_tagged_conditional_entry_counters_tokens(&conditional).unwrap();
    assert_eq!(shape.arms.len(), 2);
    assert_eq!(shape.arms[0].object_type, CardType::Creature);
    assert_eq!(
        shape.arms[0].descriptor.counter_type,
        CounterType::PlusOnePlusOne
    );
    assert_eq!(shape.arms[1].object_type, CardType::Planeswalker);
    assert_eq!(shape.arms[1].descriptor.counter_type, CounterType::Loyalty);
}

#[test]
fn parses_counter_choice_and_counter_kind_shapes() {
    let choice = lex_line(
            "Put your choice of a flying counter, a first strike counter, or a vigilance counter on target creature.",
            0,
        )
        .unwrap();
    let shape = parse_put_counter_choice_tokens(&choice).unwrap();
    assert_eq!(
        shape.counter_types,
        vec![
            CounterType::Flying,
            CounterType::FirstStrike,
            CounterType::Vigilance
        ]
    );

    let combined = lex_line(
            "Put a +1/+1 counter and a counter from among flying, first strike, lifelink, or vigilance on it.",
            0,
        )
        .unwrap();
    let combined = parse_put_fixed_and_counter_choice_tokens(&combined).unwrap();
    assert_eq!(combined.fixed.counter_type, CounterType::PlusOnePlusOne);
    assert_eq!(combined.fixed.count, 1);
    assert_eq!(
        combined.counter_types,
        vec![
            CounterType::Flying,
            CounterType::FirstStrike,
            CounterType::Lifelink,
            CounterType::Vigilance,
        ]
    );
    assert_eq!(render_token_slice(combined.target_tokens), "it");

    let each = lex_line(
            "For each kind of counter on target permanent, put another counter of that kind on it or remove one from it.",
            0,
        )
        .unwrap();
    let shape = parse_for_each_counter_kind_tokens(&each).unwrap();
    assert_eq!(render_token_slice(shape.target_tokens), "target permanent");

    let distribution = lex_line(
        "Then for each kind of counter among creatures you control, put a counter of that kind on either of those tokens.",
        0,
    )
    .unwrap();
    let distribution = parse_counter_kind_distribution_tokens(&distribution).unwrap();
    assert_eq!(
        render_token_slice(distribution.counter_source_tokens),
        "creatures you control"
    );
    assert_eq!(
        render_token_slice(distribution.target_tokens),
        "either of those tokens"
    );

    let wrong_relation = lex_line(
        "Then for each kind of counter on creatures you control, put a counter of that kind on either of those tokens.",
        0,
    )
    .unwrap();
    assert!(parse_counter_kind_distribution_tokens(&wrong_relation).is_none());
}

#[test]
fn parses_typed_counter_sequences_and_followups() {
    let placements = lex_line(
        "Put a +1/+1 counter on target creature, a flying counter on another target creature.",
        0,
    )
    .unwrap();
    let placements = parse_counter_placement_sequence_tokens(&placements).unwrap();
    assert_eq!(placements.len(), 2);
    assert_eq!(placements[1].descriptor.counter_type, CounterType::Flying);

    let three_placements = lex_line(
            "Put a +1/+1 counter on target creature, two +1/+1 counters on another target creature, and three +1/+1 counters on a third target creature.",
            0,
        )
        .unwrap();
    let three_placements = parse_counter_placement_sequence_tokens(&three_placements).unwrap();
    assert_eq!(three_placements.len(), 3);
    assert_eq!(three_placements[2].descriptor.count, 3);
    assert_eq!(
        render_token_slice(three_placements[2].target_tokens),
        "a third target creature"
    );

    let normalized_three_placements = lex_line(
            "Put a +1/+1 counter on target creature, two +1/+1 counters on another target creature and three +1/+1 counters on a third target creature.",
            0,
        )
        .unwrap();
    assert_eq!(
        parse_counter_placement_sequence_tokens(&normalized_three_placements)
            .unwrap()
            .len(),
        3
    );

    let shared = lex_line(
        "Put a flying counter and a vigilance counter on target creature.",
        0,
    )
    .unwrap();
    let shared = parse_shared_counter_target_tokens(&shared).unwrap();
    assert_eq!(shared.descriptors.len(), 2);
    assert_eq!(render_token_slice(shared.target_tokens), "target creature");

    let followup = lex_line(
        "Put a +1/+1 counter on target creature and it gains flying until end of turn.",
        0,
    )
    .unwrap();
    let followup = parse_counter_followup_tokens(&followup).unwrap();
    assert_eq!(
        render_token_slice(followup.followup_tokens),
        "gains flying until end of turn"
    );
}
