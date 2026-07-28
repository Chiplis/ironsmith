use super::*;

fn any_number_counter_removal_sequence() -> Vec<Effect> {
    let chosen_tag = TagKey::from("counter_removal_subset");
    let choose = Effect::new(crate::effects::ChooseObjectsEffect::new(
        ObjectFilter::permanent()
            .in_zone(Zone::Battlefield)
            .controlled_by(PlayerFilter::You),
        ChoiceCount::any_number(),
        PlayerFilter::You,
        chosen_tag.clone(),
    ));
    let remove = Effect::with_id(
        7,
        Effect::new(crate::effects::RemoveCountersEffect::new(
            CounterType::Loyalty,
            1,
            ChooseSpec::Iterated,
        )),
    );
    let for_each = Effect::new(crate::effects::ForEachTaggedEffect::new(
        chosen_tag,
        vec![remove],
    ));
    vec![choose, for_each]
}

#[test]
fn choose_any_number_then_remove_from_each_recovers_oracle_surface() {
    let effects = any_number_counter_removal_sequence();
    let expected = "Remove a loyalty counter from each of any number of permanents you control";

    assert_eq!(describe_effect_list(&effects), expected);
    assert_eq!(
        describe_effect(&Effect::new(crate::effects::SequenceEffect::new(
            effects.clone(),
        ))),
        expected
    );

    let wrapped = vec![effects[0].clone(), Effect::with_id(11, effects[1].clone())];
    assert_eq!(
        describe_effect_list(&wrapped),
        expected,
        "reference-binding wrappers must not expose the internal choose/loop expansion"
    );
}

#[test]
fn counter_removed_activation_result_preserves_scaling_and_omitted_this_way() {
    let basis = Value::EffectValue(ironsmith_core::EffectId(7))
        .with_surface_hint(ironsmith_core::ValueSurfaceHint::CountersRemoved);
    let power = Value::Scaled(Box::new(basis), 2);

    assert_eq!(
        describe_dynamic_runtime_pt_with_where_x(
            "this creature",
            false,
            None,
            &power,
            &Value::Fixed(0),
            &Until::EndOfTurn,
        ),
        Some("For each counter removed, this creature gets +2/+0 until end of turn".to_string())
    );
}

#[test]
fn exile_top_play_bundle_keeps_effect_count_backreference() {
    let exile = crate::effects::ExileTopOfLibraryEffect::new(
        Value::EffectValue(ironsmith_core::EffectId(11)),
        PlayerFilter::You,
    );

    assert_eq!(
        describe_exile_top_clause(&exile, false),
        Some((
            "Exile that many cards from the top of your library".to_string(),
            false,
        ))
    );
}
