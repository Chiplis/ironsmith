//! Source-authored / UNRUN. No compiler or corpus recovery claim.
use ironsmith_compiled_artifact::WireEffect;
use ironsmith_core::{ChooseSpec, CounterEffect, CounterExileGate, CounterExilePermission, ObjectFilter};
use serde_json::json;

#[test]
fn counter_exile_permission_survives_typed_decode_and_reference_walks() {
    for gate in [CounterExileGate::AnySpell, CounterExileGate::PermanentSpell] {
        for allow_land in [false, true] {
            let model = CounterEffect::any_spell()
                .with_exile_permission(CounterExilePermission { gate, allow_land });
            let payload = serde_json::to_value(&model).unwrap();
            let decoded = super::decode("CounterEffect", payload.clone()).unwrap();
            assert_eq!(decoded.downcast_ref::<CounterEffect>(), Some(&model));
            let wire = WireEffect::new("CounterEffect", payload);
            let mut bind = |_: u32| -> Result<u32, String> {
                panic!("atomic counter permission must not invent a card reference")
            };
            let (normalized, opaque) =
                super::authored_definition_graph(&wire, &mut bind, &[]).unwrap();
            assert!(!opaque);
            assert_eq!(normalized, serde_json::to_value(&wire).unwrap());
            assert_eq!(super::remap_card_ids(&wire, &mut bind).unwrap(), normalized);
        }
    }
    let legacy = serde_json::to_value(CounterEffect::any_spell()).unwrap();
    assert!(legacy.get("exile_permission").is_none());
    let decoded = super::decode("CounterEffect", legacy.clone()).unwrap();
    assert!(decoded.downcast_ref::<CounterEffect>().unwrap().exile_permission.is_none());
    assert_eq!(serde_json::to_value(decoded.downcast_ref::<CounterEffect>().unwrap()).unwrap(), legacy);
}

#[test]
fn malformed_counter_exile_gate_fails_decode_and_card_graph_walk() {
    for permission in [
        json!({ "allow_land": false }),
        json!({ "gate": null, "allow_land": false }),
        json!({ "gate": "Permanent", "allow_land": false }),
        json!({ "gate": "PermanentSpell" }),
        json!({ "gate": "PermanentSpell", "allow_land": "false" }),
        json!({ "gate": "PermanentSpell", "allow_land": false, "stable_fallback": true }),
    ] {
        let mut payload = serde_json::to_value(CounterEffect::any_spell()).unwrap();
        payload["exile_permission"] = permission;
        assert!(super::decode("CounterEffect", payload.clone()).is_err());
        let wire = WireEffect::new("CounterEffect", payload);
        let mut bind = |id| Ok(id);
        assert!(super::remap_card_ids(&wire, &mut bind).is_err());
        assert!(super::authored_definition_graph(&wire, &mut bind, &[]).is_err());
    }
}

#[test]
fn counter_permission_rejects_dangling_and_broad_targets_but_plain_counters_keep_them() {
    let mut source_filter = ObjectFilter::spell();
    source_filter.source = true;
    let mut nested_tag_filter = ObjectFilter::spell();
    nested_tag_filter.any_of.push(ObjectFilter::default().match_tagged(
        "stale", ironsmith_core::filter_model::TaggedOpbjectRelation::IsTaggedObject));
    for target in [
        ChooseSpec::Object(source_filter),
        ChooseSpec::target(ChooseSpec::Object(nested_tag_filter)),
        ChooseSpec::Tagged("dangling".into()),
        ChooseSpec::All(ObjectFilter::spell()),
        ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature())),
    ] {
        let plain = CounterEffect::new(target);
        let legacy = serde_json::to_value(&plain).unwrap();
        assert!(super::decode("CounterEffect", legacy).is_ok(), "old counter vocabulary is unchanged");
        let marked = plain.with_exile_permission(CounterExilePermission {
            gate: CounterExileGate::PermanentSpell, allow_land: false,
        });
        assert!(!marked.exile_permission_target_is_supported());
        let payload = serde_json::to_value(marked).unwrap();
        assert!(super::decode("CounterEffect", payload.clone()).is_err());
        let wire = WireEffect::new("CounterEffect", payload);
        assert!(super::remap_card_ids(&wire, &mut |id| Ok(id)).is_err());
        assert!(super::authored_definition_graph(&wire, &mut |id| Ok(id), &[]).is_err());
    }
}
