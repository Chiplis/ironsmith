//! Source-authored model contracts, UNRUN. Named JSON is the artifact contract.
//! Defaulting old model fragments is not artifact migration or signed-byte repair.
#![cfg(feature = "serde")]
use ironsmith_core::{Condition, ConditionalEffect, DelayedTriggerSpec, EffectPredicate,
    ObjectCharacteristic, PlayerFilter, RepeatProcessPromptEffect, RepeatProcessPromptKind,
    Until, ContinuousDurationObject, AnthemCountExpression, CounterType};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::fmt::Debug;

fn roundtrip<T: Serialize + DeserializeOwned + PartialEq + Debug>(model: T) -> Value {
    let bytes = serde_json::to_vec(&model).unwrap();
    let restored: T = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(restored, model);
    assert_eq!(serde_json::to_vec(&restored).unwrap(), bytes);
    serde_json::from_slice(&bytes).unwrap()
}

#[test]
fn repeat_defaults_are_explicit_current_fields_without_rewriting_old_bytes() {
    let old = br#"{"condition":"YourTurn","if_true":[7],"if_false":[8],"surface":"LeadingIf"}"#;
    let baseline: ConditionalEffect<u32> = serde_json::from_slice(old).unwrap();
    assert!(!baseline.capture_condition_result);
    for capture in [false, true] {
        let model = baseline.clone().with_condition_result(capture);
        let named = roundtrip(model);
        assert_eq!(named["capture_condition_result"], capture);
        assert_eq!(named["if_true"], json!([7]));
        assert_eq!(named["if_false"], json!([8]));
        assert_ne!(serde_json::to_vec(&named).unwrap(), old.to_vec());
    }
    for malformed in [Value::Null, json!("false"), json!(0)] {
        let mut named = serde_json::to_value(&baseline).unwrap();
        named["capture_condition_result"] = malformed;
        assert!(serde_json::from_value::<ConditionalEffect<u32>>(named).is_err());
    }
    let absent: RepeatProcessPromptEffect = serde_json::from_value(json!({
        "kind": "MayRepeatAnyNumberOfTimes",
    })).unwrap();
    assert_eq!(absent.decider, None);
    for decider in [None, Some(PlayerFilter::You), Some(PlayerFilter::Opponent),
        Some(PlayerFilter::Specific(ironsmith_core::PlayerId::from_index(1)))] {
        let prompt = RepeatProcessPromptEffect::new(RepeatProcessPromptKind::MayRepeatAnyNumberOfTimes)
            .with_decider(decider.clone());
        assert_eq!(roundtrip(prompt)["decider"], serde_json::to_value(decider).unwrap());
    }
    for malformed in [json!(false), json!(0), json!("missing-player-filter")] {
        assert!(serde_json::from_value::<RepeatProcessPromptEffect>(json!({
            "kind": "MayRepeatAnyNumberOfTimes", "decider": malformed,
        })).is_err());
    }
}

#[test]
fn appended_named_models_keep_complete_nondefault_payloads_and_refuse_malformed_fields() {
    use ironsmith_core::trigger_model::PlayerAttackGrouping;
    for grouping in [PlayerAttackGrouping::Attacker, PlayerAttackGrouping::Defender,
        PlayerAttackGrouping::Pair, PlayerAttackGrouping::AttackerAnyTarget] {
        let named = roundtrip(DelayedTriggerSpec::PlayerAttackDeclaration {
            attacker: PlayerFilter::Opponent, defender: PlayerFilter::You, grouping,
        });
        assert_eq!(named["PlayerAttackDeclaration"], json!({
            "attacker": "Opponent", "defender": "You", "grouping": grouping,
        }));
        for field in ["attacker", "defender", "grouping"] {
            let mut missing = named.clone();
            missing["PlayerAttackDeclaration"].as_object_mut().unwrap().remove(field);
            assert!(serde_json::from_value::<DelayedTriggerSpec>(missing).is_err());
        }
    }
    for characteristic in [ObjectCharacteristic::Name, ObjectCharacteristic::Color] {
        for required_count in [0, 2, u32::MAX] {
            assert_eq!(roundtrip(EffectPredicate::AffectedObjectsShare {
                required_count, characteristic,
            }), json!({"AffectedObjectsShare": {"required_count": required_count,
                "characteristic": characteristic}}));
        }
    }
    for invalid in [json!(-1), json!("2"), Value::Null] {
        assert!(serde_json::from_value::<EffectPredicate>(json!({"AffectedObjectsShare": {
            "required_count": invalid, "characteristic": "Name",
        }})).is_err());
    }
    for player in [PlayerFilter::You, PlayerFilter::Opponent] {
        for kind in [CounterType::Experience, CounterType::Poison] {
            assert_eq!(roundtrip(AnthemCountExpression::PlayerCounters(player.clone(), kind)),
                json!({"PlayerCounters": [player, kind]}));
        }
        assert_eq!(roundtrip(Until::PlayersNextUntapStep { player: player.clone() }),
            json!({"PlayersNextUntapStep": {"player": player}}));
    }
    for object in [ContinuousDurationObject::Source, ContinuousDurationObject::AffectedObject,
        ContinuousDurationObject::Specific(ironsmith_core::ObjectId::from_raw(17))] {
        assert_eq!(roundtrip(Until::UntilControllersNextUntapStep { object: object.clone() }),
            json!({"UntilControllersNextUntapStep": {"object": object}}));
    }
    for malformed in [json!({"PlayersNextUntapStep": {}}),
        json!({"PlayersNextUntapStep": {"player": false}}),
        json!({"UntilControllersNextUntapStep": {}}),
        json!({"UntilControllersNextUntapStep": {"object": null}})] {
        assert!(serde_json::from_value::<Until>(malformed).is_err());
    }
    assert!(serde_json::from_value::<AnthemCountExpression>(json!({"PlayerCounters": ["You"]})).is_err());
    // Ordinary historical conditions retain their prior constructor semantics.
    assert!(!ConditionalEffect::<u32>::if_only(Condition::YourTurn, vec![]).capture_condition_result);
}
