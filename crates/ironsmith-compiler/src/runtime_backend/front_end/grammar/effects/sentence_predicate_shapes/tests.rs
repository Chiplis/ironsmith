use super::super::super::super::lexer::lex_line;
use super::*;

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
        Some(WhereXValueShape::PriorEffectMetric {
            source: EffectMetricSource::AffectedObjects,
            metric: EffectMetric::Count,
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
