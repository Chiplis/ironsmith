//! Current-main structural compatibility cases, source-authored and UNRUN.
//! Named JSON is the model contract; defaulting is never artifact admission.
#![cfg(feature = "serde")]

use ironsmith_core::{
    CardType, ChooseSpec, Color, DamageFilter, ObjectFilter, ObjectId,
    PreventAllDamageToTargetEffect, PreventDamageEffect, Until,
};
use serde_json::{Value, json};

fn restricted_filter() -> DamageFilter {
    let mut filter = DamageFilter::combat();
    filter.from_source = Some(ObjectFilter::creature());
    filter.from_colors = Some(vec![Color::Red, Color::Blue]);
    filter.from_card_types = Some(vec![CardType::Creature]);
    filter.from_specific_source = Some(ObjectId::from_raw(17));
    filter.excluded_specific_source = Some(ObjectId::from_raw(23));
    filter.resolved_permanent_source = Some(ObjectId::from_raw(29));
    filter
}

#[test]
fn finite_legacy_filter_defaults_all_but_current_json_retains_every_restriction() {
    let baseline = PreventDamageEffect::<Value>::new(4.into(), ChooseSpec::Source, Until::EndOfTurn);
    let current_default = serde_json::to_value(&baseline).unwrap();
    let mut legacy = current_default.clone();
    assert_eq!(legacy.as_object_mut().unwrap().remove("damage_filter"),
        Some(serde_json::to_value(DamageFilter::all()).unwrap()));
    let restored: PreventDamageEffect<Value> = serde_json::from_value(legacy.clone()).unwrap();
    assert_eq!(restored, baseline);
    assert_eq!(serde_json::to_value(restored).unwrap(), current_default);
    assert_ne!(legacy, current_default, "defaulting is not byte-preserving migration");

    let filtered = baseline.with_filter(restricted_filter())
        .with_follow_up_effects(vec![json!({"retained_child": 7})])
        .with_source_of_your_choice().protecting_you_and_permanents_you_control();
    let bytes = serde_json::to_vec(&filtered).unwrap();
    let restored: PreventDamageEffect<Value> = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(restored, filtered);
    assert_eq!(serde_json::to_vec(&restored).unwrap(), bytes);
    for invalid in [Value::Null, json!(false), json!("all"), json!({})] {
        let mut broken = current_default.clone();
        broken["damage_filter"] = invalid;
        assert!(serde_json::from_value::<PreventDamageEffect<Value>>(broken).is_err());
    }
}

#[test]
fn unlimited_legacy_defaults_do_not_erase_existing_combat_only_or_child_program() {
    let baseline = PreventAllDamageToTargetEffect::new(ChooseSpec::Source, Until::EndOfTurn)
        .combat_only().with_follow_up_effects(vec![json!({"retained_child": 9})]);
    let current_default = serde_json::to_value(&baseline).unwrap();
    let mut legacy = current_default.clone();
    assert!(legacy.as_object_mut().unwrap().remove("damage_filter").is_some());
    assert_eq!(legacy.as_object_mut().unwrap().remove("source_color_of_your_choice"), Some(json!(false)));
    let restored: PreventAllDamageToTargetEffect<Value> = serde_json::from_value(legacy.clone()).unwrap();
    assert_eq!(restored, baseline);
    assert!(restored.combat_only);
    assert_eq!(restored.damage_filter, DamageFilter::all());
    assert_eq!(serde_json::to_value(restored).unwrap(), current_default);
    assert_ne!(legacy, current_default);

    for choose_color in [false, true] {
        let mut filtered = baseline.clone().with_filter(restricted_filter());
        filtered.source_color_of_your_choice = choose_color;
        let bytes = serde_json::to_vec(&filtered).unwrap();
        let encoded: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(encoded["source_color_of_your_choice"], choose_color);
        let restored: PreventAllDamageToTargetEffect<Value> = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(restored, filtered);
        assert_eq!(serde_json::to_vec(&restored).unwrap(), bytes);
    }
    for invalid in [Value::Null, json!(0), json!("false")] {
        let mut broken = current_default.clone();
        broken["source_color_of_your_choice"] = invalid;
        assert!(serde_json::from_value::<PreventAllDamageToTargetEffect<Value>>(broken).is_err());
    }
    for invalid in [Value::Null, json!(false), json!("all"), json!({})] {
        let mut broken = current_default.clone();
        broken["damage_filter"] = invalid;
        assert!(serde_json::from_value::<PreventAllDamageToTargetEffect<Value>>(broken).is_err());
    }
}

#[test]
fn source_counter_bridge_preserves_the_complete_published_payload_bytes() {
    use ironsmith_core::{CounterType, RemoveAnyCountersFromSourceEffect};
    // Source-authored contract from the published core struct, not a captured
    // historical artifact. The runtime re-export adds no serialized owner flag.
    for (counter_type, encoded_type) in [
        (None, "null"),
        (Some(CounterType::PlusOnePlusOne), "\"PlusOnePlusOne\""),
        (Some(CounterType::Named("hour".into())), "{\"Named\":\"hour\"}"),
    ] {
        for display_x in [false, true] {
            for remove_all in [false, true] {
                let bytes = format!("{{\"counter_type\":{encoded_type},\"display_x\":{display_x},\"remove_all\":{remove_all}}}");
                let model = RemoveAnyCountersFromSourceEffect { counter_type, display_x, remove_all };
                assert_eq!(serde_json::to_string(&model).unwrap(), bytes);
                let restored: RemoveAnyCountersFromSourceEffect = serde_json::from_str(&bytes).unwrap();
                assert_eq!(restored, model);
                assert_eq!(serde_json::to_string(&restored).unwrap(), bytes);
                if remove_all { assert!(restored.cost_display().starts_with("Remove all ")); }
            }
        }
    }
    for malformed in [
        json!({"counter_type": false, "display_x": false, "remove_all": false}),
        json!({"counter_type": null, "display_x": "false", "remove_all": false}),
        json!({"counter_type": null, "display_x": false, "remove_all": 0}),
        json!({"counter_type": null, "display_x": false}),
    ] {
        assert!(serde_json::from_value::<RemoveAnyCountersFromSourceEffect>(malformed).is_err());
    }
}
