//! Source-authored compatibility contracts; campaign execution remains deferred.
//!
//! WireEffect payloads are opaque during ordinary artifact loading. The boundary
//! checked here is typed decoding and fresh native re-encoding, also used for
//! executable effects carried by restricted mana in public checkpoints.
use ironsmith::{Effect, effects};
use ironsmith::target::ChooseSpec;
use ironsmith_compiled_artifact::WireEffect;
use ironsmith_runtime_catalog::artifact_materializer::{encode_runtime_effect, materialize_effect};

#[test]
fn absent_counter_ceiling_stays_absent_through_typed_and_native_encoding() {
    let original = ironsmith_core::PutCountersEffect::plus_one_counters(2, ChooseSpec::Source);
    let mut legacy = serde_json::to_value(&original).unwrap();
    legacy.as_object_mut().unwrap().remove("maximum_total");
    let restored: ironsmith_core::PutCountersEffect = serde_json::from_value(legacy.clone()).unwrap();
    assert_eq!(restored.maximum_total, None);
    assert_eq!(serde_json::to_value(&restored).unwrap(), legacy);

    let expected = WireEffect::new("PutCountersEffect", legacy);
    let native = encode_runtime_effect(Effect::new(restored)).unwrap();
    assert_eq!(native, expected, "fresh native output must retain the established wire shape");
    let materialized = materialize_effect(expected.clone()).unwrap();
    assert_eq!(encode_runtime_effect(materialized).unwrap(), expected);

    let limited = original.with_maximum_total(7);
    let encoded = encode_runtime_effect(Effect::new(limited.clone())).unwrap();
    assert_eq!(encoded.payload()["maximum_total"], 7);
    let decoded: ironsmith_core::PutCountersEffect = serde_json::from_value(encoded.payload().clone()).unwrap();
    assert_eq!(decoded, limited, "authored ceilings must not be omitted");
}

#[test]
fn absent_combat_flag_stays_absent_and_fresh_native_redirection_is_encodable() {
    let original = ironsmith_core::RedirectNextTimeDamageToSourceEffect::from_source_target(ChooseSpec::Source);
    let mut legacy = serde_json::to_value(&original).unwrap();
    legacy.as_object_mut().unwrap().remove("combat_only");
    let restored: ironsmith_core::RedirectNextTimeDamageToSourceEffect = serde_json::from_value(legacy.clone()).unwrap();
    assert!(!restored.combat_only);
    assert_eq!(serde_json::to_value(&restored).unwrap(), legacy);

    let expected = WireEffect::new("RedirectNextTimeDamageToSourceEffect", legacy);
    let native = effects::RedirectNextTimeDamageToSourceEffect::from_source_target(ChooseSpec::Source);
    assert_eq!(encode_runtime_effect(Effect::new(native)).unwrap(), expected);
    assert_eq!(encode_runtime_effect(materialize_effect(expected.clone()).unwrap()).unwrap(), expected);

    // Exercise every native source and destination variant without relying on a
    // retained compiler model to hide a missing or lossy native encoder.
    for source in [
        effects::RedirectNextTimeDamageSource::Choice,
        effects::RedirectNextTimeDamageSource::Filter(Default::default()),
        effects::RedirectNextTimeDamageSource::Target(ChooseSpec::Source),
    ] {
        for destination in [
            effects::RedirectNextTimeDamageDestination::DamageSource,
            effects::RedirectNextTimeDamageDestination::SourceObject,
            effects::RedirectNextTimeDamageDestination::Controller,
            effects::RedirectNextTimeDamageDestination::SourceController,
            effects::RedirectNextTimeDamageDestination::TargetObject,
        ] {
            let mut native = effects::RedirectNextTimeDamageToSourceEffect::new(source.clone(), ChooseSpec::Source);
            native.combat_only = true;
            native.destination = destination;
            native.destination_target = (destination == effects::RedirectNextTimeDamageDestination::TargetObject).then_some(ChooseSpec::Source);
            native.all_this_turn = true;
            let encoded = encode_runtime_effect(Effect::new(native.clone())).unwrap();
            assert_eq!(encoded.payload()["combat_only"], true);
            let restored = materialize_effect(encoded.clone()).unwrap();
            assert_eq!(restored.downcast_ref::<effects::RedirectNextTimeDamageToSourceEffect>(), Some(&native));
            assert_eq!(encode_runtime_effect(restored).unwrap(), encoded);
        }
    }
}
