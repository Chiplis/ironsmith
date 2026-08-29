use super::*;

fn parse(raw: &str) -> ActivationCostCst {
    let tokens = lex_line(raw, 0).expect("activation-cost test surface should lex");
    parse_activation_cost_tokens(&tokens)
        .expect("activation-cost grammar should own the test surface")
}

#[test]
fn program_owns_loyalty_shorthand_and_preserves_full_cost_alternatives() {
    let loyalty = parse("[-X]");
    assert!(loyalty.is_loyalty_shorthand);
    assert!(matches!(
        loyalty.segments.as_slice(),
        [ActivationCostSegmentCst::RemoveCountersDynamic {
            counter_type: Some(CounterType::Loyalty),
            display_x: true,
            remove_all: false,
        }]
    ));

    let shard = parse("{W}, {T} or {U}, {T}");
    assert!(shard.segments.is_empty());
    assert_eq!(shard.alternative_branches.len(), 2);
    for branch in &shard.alternative_branches {
        assert!(matches!(
            branch.segments.as_slice(),
            [
                ActivationCostSegmentCst::Mana(_),
                ActivationCostSegmentCst::Tap
            ]
        ));
    }
}

#[test]
fn program_preserves_waterbend_generic_as_typed_cost_metadata() {
    let waterbend = parse("Waterbend {5}");
    assert_eq!(waterbend.waterbend_generic, Some(5));
    assert!(matches!(
        waterbend.segments.as_slice(),
        [ActivationCostSegmentCst::Mana(_)]
    ));

    assert_eq!(parse("{5}").waterbend_generic, None);
}

#[test]
fn program_preserves_named_card_commas_and_payment_alternatives() {
    let composite = parse("Discard a card named Mishra, Lost to Phyrexia, sacrifice a creature");
    assert_eq!(composite.segments.len(), 2);
    assert!(composite.alternative_branches.is_empty());

    let alternative = parse("Pay {3} or discard a card");
    assert!(alternative.segments.is_empty());
    assert_eq!(alternative.alternative_branches.len(), 2);
}

#[test]
fn token_and_raw_test_entrypoints_share_the_typed_program() {
    let raw = "{2}, {T}, sacrifice another creature";
    let tokens = lex_line(raw, 0).unwrap();
    assert_eq!(
        parse_activation_cost_tokens_rewrite(&tokens).unwrap(),
        parse_activation_cost_rewrite(raw).unwrap()
    );
}

#[test]
fn named_source_sacrifice_keeps_exact_short_name_surface() {
    let text = "{2}, Sacrifice ED-E";
    let tokens = lex_line(text, 0).expect("named-source activation cost should lex");
    let context =
        crate::parse_context::ParseContext::for_fragment("ED-E", Vec::new(), Vec::new(), text);
    let parsed = parse_activation_cost_tokens_with_context(context.view(), &tokens)
        .expect("activation-cost grammar should own the named-source surface");
    let [
        ActivationCostSegmentCst::Mana(_),
        ActivationCostSegmentCst::SacrificeSelf {
            surface: Some(surface),
        },
    ] = parsed.segments.as_slice()
    else {
        panic!("expected mana plus named-source sacrifice, got {parsed:?}");
    };
    assert_eq!(surface.display_text(), "ED-E");
}

#[test]
fn exile_creature_you_control_is_a_typed_activation_cost() {
    let parsed = parse("{6}{B}, {T}, Exile a creature you control");
    assert!(matches!(
        parsed.segments.as_slice(),
        [
            ActivationCostSegmentCst::Mana(_),
            ActivationCostSegmentCst::Tap,
            ActivationCostSegmentCst::ExileChosen { .. }
        ]
    ));
}

#[test]
fn source_bound_counter_cost_keeps_exile_it_on_that_source() {
    let parsed = parse("{T}, Remove X time counters from this artifact and exile it");
    assert!(matches!(
        parsed.segments.as_slice(),
        [
            ActivationCostSegmentCst::Tap,
            ActivationCostSegmentCst::RemoveCountersDynamic {
                counter_type: Some(CounterType::Time),
                display_x: true,
                remove_all: false,
            },
            ActivationCostSegmentCst::ExileSelf,
        ]
    ));
}

#[test]
fn source_plus_any_number_sacrifice_is_two_typed_cost_segments() {
    let parsed = parse("{T}, Sacrifice this artifact and any number of creatures you control");
    let [
        ActivationCostSegmentCst::Tap,
        ActivationCostSegmentCst::SacrificeSelf { surface: None },
        ActivationCostSegmentCst::SacrificeChosen { count, filter },
    ] = parsed.segments.as_slice()
    else {
        panic!("expected source and chosen sacrifice costs, got {parsed:?}");
    };
    assert!(count.is_any_number());
    assert_eq!(filter.card_types, [crate::types::CardType::Creature]);
    assert_eq!(filter.controller, Some(crate::target::PlayerFilter::You));
}

#[test]
fn chosen_set_then_source_sacrifice_is_two_typed_cost_segments_in_authored_order() {
    let parsed = parse("{T}, Sacrifice two lands and this artifact");
    let [
        ActivationCostSegmentCst::Tap,
        ActivationCostSegmentCst::SacrificeChosen { count, filter },
        ActivationCostSegmentCst::SacrificeSelf { surface: None },
    ] = parsed.segments.as_slice()
    else {
        panic!("expected chosen-land and source sacrifice costs, got {parsed:?}");
    };
    assert_eq!(count.min, 2);
    assert_eq!(count.max, Some(2));
    assert_eq!(filter.card_types, [crate::types::CardType::Land]);
    assert!(
        filter.any_of.is_empty(),
        "the source must not enter the land filter"
    );
}

#[test]
fn two_chosen_sacrifice_arms_remain_one_filter_union_near_miss() {
    let parsed = parse("Sacrifice two lands and artifacts");
    let [ActivationCostSegmentCst::SacrificeChosen { filter, .. }] = parsed.segments.as_slice()
    else {
        panic!("two ordinary chosen types should remain one cost: {parsed:?}");
    };
    assert!(filter.card_types.contains(&crate::types::CardType::Land));
    assert!(
        filter
            .card_types
            .contains(&crate::types::CardType::Artifact)
    );
}
