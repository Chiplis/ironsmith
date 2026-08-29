use super::*;

#[test]
fn spell_cast_surface_preserves_player_and_filter_boundary() {
    let words = [
        "other", "creature", "spells", "an", "opponent", "has", "cast", "this", "turn",
    ];
    let parsed = parse_spell_cast_this_turn_surface(&words).unwrap();
    assert_eq!(parsed.filter_end, 3);
    assert_eq!(parsed.player, PlayerFilter::Opponent);
    assert!(parsed.exclude_source);

    let value = parse_spells_cast_this_turn_value_words(&words).unwrap();
    let Value::SpellsCastThisTurnMatching { filter, .. } = value else {
        panic!("expected spell-cast count value");
    };
    assert_eq!(filter.card_types, vec![crate::types::CardType::Creature]);
}

#[test]
fn prior_effect_reference_excludes_chosen_type_and_color() {
    assert_eq!(
        parse_prior_effect_metric_source(&["cards", "chosen", "this", "way"]),
        Some(EffectMetricSource::ChosenObjects)
    );
    assert_eq!(parse_prior_effect_metric_source(&["chosen", "color"]), None);
}

#[test]
fn aggregate_scope_returns_semantic_value_directly() {
    let Value::ColorsAmong(filter) =
        parse_aggregate_scope_value_words(&["colors", "among", "creatures", "you", "control"])
            .unwrap()
    else {
        panic!("expected colors-among value");
    };
    assert_eq!(filter.card_types, vec![crate::types::CardType::Creature]);
    assert_eq!(filter.controller, Some(PlayerFilter::You));
}

#[test]
fn counter_aggregate_preserves_among_surface() {
    let value =
        parse_aggregate_scope_value_words(&["counters", "among", "creatures", "you", "control"])
            .expect("counter aggregate value");
    assert!(value.has_surface_hint(ironsmith_core::ValueSurfaceHint::CountersAmong));
    assert!(matches!(
        value.unhinted(),
        Value::CountersOn(spec, None)
            if matches!(spec.unhinted(), ChooseSpec::All(filter)
                if filter.card_types == vec![crate::types::CardType::Creature]
                    && filter.controller == Some(PlayerFilter::You))
    ));
}
