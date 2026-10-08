//! Source-authored model contract cases. UNRUN; no historical fixture claim.
#![cfg(feature = "serde")]
use ironsmith_core::{Condition, ConditionalEffect, EffectPredicate, ObjectCharacteristic,
    PlayerFilter, RepeatProcessPromptEffect, RepeatProcessPromptKind};

#[test]
fn historical_absent_fields_keep_previous_condition_and_prompt_semantics() {
    let ordinary = ConditionalEffect::<u32>::if_only(Condition::LifeTotalOrLess(20), vec![7]);
    let mut historical = serde_json::to_value(&ordinary).unwrap();
    historical.as_object_mut().unwrap().remove("capture_condition_result");
    let restored: ConditionalEffect<u32> = serde_json::from_value(historical).unwrap();
    assert_eq!(restored, ordinary);
    let prompt: RepeatProcessPromptEffect = serde_json::from_value(serde_json::json!({
        "kind": "MayRepeatAnyNumberOfTimes"
    })).unwrap();
    assert_eq!(prompt.decider, None);
}

#[test]
fn captured_gate_explicit_actor_and_subset_predicate_roundtrip_without_loss() {
    let gate = ConditionalEffect::<u32>::if_only(Condition::LifeTotalOrLess(20), vec![7])
        .with_condition_result(true);
    let bytes = serde_json::to_vec(&gate).unwrap();
    assert_eq!(serde_json::from_slice::<ConditionalEffect<u32>>(&bytes).unwrap(), gate);
    let prompt = RepeatProcessPromptEffect::new(RepeatProcessPromptKind::MayRepeatAnyNumberOfTimes)
        .with_decider(Some(PlayerFilter::target_opponent()));
    let bytes = serde_json::to_vec(&prompt).unwrap();
    assert_eq!(serde_json::from_slice::<RepeatProcessPromptEffect>(&bytes).unwrap(), prompt);
    let predicate = EffectPredicate::AffectedObjectsShare {
        required_count: 2, characteristic: ObjectCharacteristic::Name,
    };
    let bytes = serde_json::to_vec(&predicate).unwrap();
    assert_eq!(serde_json::from_slice::<EffectPredicate>(&bytes).unwrap(), predicate);
}
