use super::super::super::super::lexer::lex_line;
use super::*;

#[test]
fn parses_attacking_doesnt_tap_if_source_is_untapped() {
    let tokens = lex_line(
        "Attacking doesn't cause creatures you control to tap this combat if this is untapped.",
        0,
    )
    .unwrap();
    let shape =
        parse_attacking_doesnt_tap_if_source_untapped_tokens(&tokens).expect("vigilance surface");
    assert_eq!(
        parser_token_word_refs(shape.affected_tokens),
        ["creatures", "you", "control"]
    );
}

#[test]
fn parses_typed_where_x_shapes() {
    let tokens = lex_line(
        "Target creature deals X damage to any target and X damage to itself, where X is its power.",
        0,
    )
    .unwrap();
    let damage = parse_power_damage_self_tokens(&tokens).unwrap();
    assert_eq!(
        parser_token_word_refs(damage.source_tokens),
        ["target", "creature"]
    );
    assert_eq!(
        parser_token_word_refs(damage.first_target_tokens),
        ["any", "target"]
    );

    let tokens = lex_line(
        "Draw X cards, where X is the number of cards exiled this way.",
        0,
    )
    .unwrap();
    let sentence = parse_where_x_sentence_tokens(&tokens).unwrap();
    let layout = sentence.layout(false);
    assert_eq!(
        parse_where_x_value_shape_tokens(
            layout.primary_where_tokens,
            sentence.stripped_references_target,
        ),
        Some(WhereXValueShape::PriorEffectMetric(
            PriorEffectMetricQuery::new(EffectMetricSource::AffectedObjects, EffectMetric::Count,)
                .with_filter({
                    let mut filter = ObjectFilter::default();
                    filter.set_explicit_card_noun(true);
                    filter
                })
                .with_action(ironsmith_core::PriorEffectAction::Exiled),
        ))
    );

    let tokens = lex_line(
        "Put X +1/+1 counters on it, where X is the number of charge counters removed this way.",
        0,
    )
    .unwrap();
    let sentence = parse_where_x_sentence_tokens(&tokens).unwrap();
    assert_eq!(
        parse_where_x_value_shape_tokens(
            sentence.layout(false).primary_where_tokens,
            sentence.stripped_references_target,
        ),
        Some(WhereXValueShape::RemovedCountersThisWay),
    );
}

#[test]
fn parses_chosen_objects_power_difference() {
    let tokens = lex_line(
        "where X is the difference between the chosen creatures' powers.",
        0,
    )
    .unwrap();
    assert_eq!(
        parse_where_x_value_shape_tokens(&tokens, false),
        Some(WhereXValueShape::ChosenObjectsPowerDifference {
            object_kind: "creature".to_string(),
        })
    );
}

#[test]
fn distinguishes_source_pronoun_damage_from_later_targets() {
    let tokens = lex_line(
        "It deals X damage divided as you choose among up to X target creatures, where X is its power.",
        0,
    )
    .unwrap();
    let sentence = parse_where_x_sentence_tokens(&tokens).unwrap();
    assert!(sentence.stripped_references_target);
    assert!(starts_with_source_deals_x_tokens(sentence.stripped_tokens));
    assert_eq!(
        parse_where_x_value_shape_tokens(sentence.where_tokens, false),
        Some(WhereXValueShape::ReferenceMetric {
            reference: WhereXReferenceShape::Source,
            metric: WhereXMetricShape::Power,
        })
    );
}

#[test]
fn parses_sacrificed_possessive_card_type_without_retaining_apostrophe() {
    let tokens = lex_line("where X is the sacrificed enchantment's mana value.", 0).unwrap();
    assert_eq!(
        parse_where_x_value_shape_tokens(&tokens, false),
        Some(WhereXValueShape::SacrificeCostManaValue {
            object_kind: SacrificeCostObjectKindShape::CardType(
                crate::types::CardType::Enchantment,
            ),
        })
    );

    let card_types = lex_line(
        "where X is the number of card types among cards in your graveyard.",
        0,
    )
    .unwrap();
    assert_eq!(
        parse_where_x_value_shape_tokens(&card_types, false),
        Some(WhereXValueShape::CardTypesInYourGraveyard)
    );
}

#[test]
fn parses_that_creature_possessive_mana_value_binding() {
    let tokens = lex_line("where X is that creature's mana value.", 0).unwrap();
    assert_eq!(
        parse_where_x_value_shape_tokens(&tokens, true),
        Some(WhereXValueShape::ReferenceMetric {
            reference: WhereXReferenceShape::Target,
            metric: WhereXMetricShape::ManaValue,
        })
    );
    assert_eq!(
        parse_where_x_value_shape_tokens(&tokens, false),
        Some(WhereXValueShape::ReferenceMetric {
            reference: WhereXReferenceShape::TaggedIt,
            metric: WhereXMetricShape::ManaValue,
        })
    );
}

#[test]
fn parses_typed_dispatch_and_library_shapes() {
    let delayed = lex_line(
        "At this turn's next end of combat, sacrifice that creature.",
        0,
    )
    .unwrap();
    assert!(matches!(
        parse_delayed_sentence_tokens(&delayed),
        Some(DelayedSentenceShape::EndOfCombat { .. })
    ));

    let bottom = lex_line(
        "Put two cards from a single graveyard on the bottom of their owners' library.",
        0,
    )
    .unwrap();
    assert_eq!(
        parse_single_graveyard_library_bottom_tokens(&bottom),
        Some(SingleGraveyardLibraryBottomShape { count: 2 })
    );
}

#[test]
fn parses_typed_aura_shape() {
    let tokens = lex_line(
        "It is an Aura enchantment with enchant creature you control and has 'flying' and loses all other abilities.",
        0,
    )
    .unwrap();
    let shape = parse_aura_enchantment_tokens(&tokens).unwrap();
    assert!(shape.attachment_mentions_you_control);
    assert!(shape.loses_all_abilities);
    assert_eq!(shape.granted_ability_tokens.len(), 1);
}

#[test]
fn parses_where_x_symbolic_counter_reference() {
    let tokens = lex_line("where X is the number of +1/+1 counters on it", 0).unwrap();
    assert_eq!(
        parse_where_x_value_shape_tokens(&tokens, false),
        Some(WhereXValueShape::CountersOn {
            reference: WhereXReferenceShape::Source,
            counter_type: Some(crate::object::CounterType::PlusOnePlusOne),
        })
    );
}
