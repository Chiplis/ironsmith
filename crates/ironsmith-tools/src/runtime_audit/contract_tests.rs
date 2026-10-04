use super::*;
use serde_json::json;

fn effect(kind: &str, payload: Value) -> Value {
    json!({"kind": kind, "payload": payload})
}

fn program(effects: Vec<Value>) -> Value {
    json!({"segments": [{"default_effects": effects, "self_replacements": []}], "flattened_default_effects": effects})
}

fn definition(effects: Vec<Value>) -> Value {
    json!({"card": {"name": "Contract fixture"}, "abilities": [], "spell_effect": program(effects)})
}

fn triggered(trigger: Value, effects: Vec<Value>) -> Value {
    json!({"kind": {"Triggered": {"trigger": {"kind": trigger, "label": "fixture"}, "effects": program(effects), "choices": [], "intervening_if": null}}})
}

fn loss(amount: Value) -> Value {
    effect(
        "LoseLifeEffect",
        json!({"player": {"Player": "You"}, "amount": amount}),
    )
}

fn errors(findings: &[ContractFinding]) -> Vec<&ContractFinding> {
    findings
        .iter()
        .filter(|finding| finding.severity == "error")
        .collect()
}

#[test]
fn nested_value_player_is_checked_once_without_flattened_cache_duplication() {
    let findings = audit(&definition(vec![loss(
        json!({"Add": [{"Fixed": 1}, {"LifeTotal": "IteratedPlayer"}]}),
    )]));
    assert_eq!(errors(&findings).len(), 1, "{findings:?}");
    assert!(findings[0].path.ends_with("/amount/Add/1/LifeTotal"));
    assert!(!findings[0].path.contains("flattened_default_effects"));
}

#[test]
fn player_loop_binds_body_but_not_its_filter_or_following_effect() {
    let findings = audit(&definition(vec![
        effect(
            "ForPlayersEffect",
            json!({"filter": "IteratedPlayer", "effects": [loss(json!({"LifeTotal": "IteratedPlayer"}))]}),
        ),
        loss(json!({"LifeTotal": "IteratedPlayer"})),
    ]));
    assert_eq!(errors(&findings).len(), 2, "{findings:?}");
    assert!(
        findings
            .iter()
            .any(|finding| finding.path.ends_with("/filter"))
    );
}

#[test]
fn life_loss_trigger_supplies_player_and_amount_but_upkeep_has_no_amount() {
    let value = json!({"card": {}, "abilities": [
        triggered(json!({"PlayerLosesLife": {"player": "Opponent"}}), vec![loss(json!({"Add": [{"LifeTotal": "IteratedPlayer"}, {"EventValue": "Amount"}]}))]),
        triggered(json!({"BeginningOfUpkeep": {"player": "Any"}}), vec![loss(json!({"EventValue": "Amount"}))]),
    ]});
    let findings = audit(&value);
    assert_eq!(errors(&findings).len(), 1, "{findings:?}");
    assert!(findings[0].path.starts_with("/abilities/1/"));
}

#[test]
fn any_trigger_union_must_supply_context_in_every_branch() {
    let union = json!({"AnyOf": [
        {"kind": {"PlayerLosesLife": {"player": "Any"}}},
        {"kind": "DayNightChanged"},
    ]});
    let findings = audit(
        &json!({"card": {}, "abilities": [triggered(union, vec![loss(json!({"LifeTotal": "IteratedPlayer"}))])]}),
    );
    assert!(
        errors(&findings).is_empty(),
        "Mixed branch contracts are uncertain, not proof every branch fails"
    );
    assert!(
        findings
            .iter()
            .any(|finding| finding.code == "context_provider_unknown")
    );
}

#[test]
fn delayed_body_and_created_token_do_not_inherit_the_outer_loop() {
    let token = json!({"card": {}, "abilities": [triggered(json!("DayNightChanged"), vec![loss(json!({"LifeTotal": "IteratedPlayer"}))])]});
    let findings = audit(&definition(vec![effect(
        "ForPlayersEffect",
        json!({"filter": "Any", "effects": [
            effect("ScheduleDelayedTriggerEffect", json!({"trigger": "EndOfCombat", "effects": [loss(json!({"EventValue": "Amount"}))]})),
            effect("CreateTokenEffect", json!({"controller": "IteratedPlayer", "count": {"Fixed": 1}, "token": token})),
        ]}),
    )]));
    assert_eq!(errors(&findings).len(), 2, "{findings:?}");
}

#[test]
fn both_conditional_arms_and_self_replacement_branches_are_checked() {
    let mut value = definition(vec![effect(
        "ConditionalEffect",
        json!({"condition": "Always", "if_true": [], "if_false": [loss(json!({"LifeTotal": "IteratedPlayer"}))]}),
    )]);
    value["spell_effect"]["segments"][0]["self_replacements"] = json!([{"condition": "Always", "replacement_effects": [loss(json!({"EventValueOffset": ["Amount", 1]}))]}]);
    assert_eq!(errors(&audit(&value)).len(), 2);
}

#[test]
fn outcome_ids_are_scoped_and_only_completed_producers_dominate() {
    let missing = audit(&definition(vec![loss(
        json!({"EffectMetric": {"effect_id": 7, "metric": "Count", "source": "Outcome"}}),
    )]));
    assert!(
        missing
            .iter()
            .any(|finding| finding.code == "missing_effect_outcome")
    );
    let present = audit(&definition(vec![
        effect(
            "WithIdEffect",
            json!({"id": 7, "effect": loss(json!({"Fixed": 1}))}),
        ),
        loss(json!({"EffectValue": 7})),
    ]));
    assert!(present.is_empty(), "{present:?}");
    let optional = audit(&definition(vec![
        effect(
            "MayEffect",
            json!({"effects": [effect("WithIdEffect", json!({"id": 7, "effect": loss(json!({"Fixed": 1}))}))]}),
        ),
        loss(json!({"EffectValue": 7})),
    ]));
    assert!(
        optional
            .iter()
            .any(|finding| finding.code == "outcome_dominance")
    );
    assert!(errors(&optional).is_empty());
}

#[test]
fn unresolved_values_are_errors_even_under_an_unknown_effect_contract() {
    let findings = audit(&definition(vec![effect(
        "NewEffect",
        json!({"amount": {"PendingEffectMetric": {"metric": "Count", "source": "Outcome"}}}),
    )]));
    assert!(
        findings
            .iter()
            .any(|finding| finding.code == "unknown_effect_contract")
    );
    assert!(
        findings
            .iter()
            .any(|finding| finding.code == "unresolved_compiler_value"
                && finding.severity == "error")
    );
}

#[test]
fn if_effect_does_not_unconditionally_prove_playercounts() {
    let findings = audit(&definition(vec![
        effect(
            "WithIdEffect",
            json!({"id": 1, "effect": loss(json!({"Fixed": 1}))}),
        ),
        effect(
            "IfEffect",
            json!({"condition": 1, "predicate": "Happened", "then": [loss(json!({"LifeTotal": "IteratedPlayer"}))], "else_": []}),
        ),
    ]));
    assert!(errors(&findings).is_empty());
    assert!(
        findings
            .iter()
            .any(|finding| finding.code == "context_provider_unknown")
    );
}

#[test]
fn metadata_that_looks_like_a_reference_is_ignored() {
    let mut value = definition(vec![loss(json!({"Fixed": 1}))]);
    value["card"]["name"] = json!("IteratedPlayer");
    value["card"]["oracle_text"] = json!("PendingComparisonLeft");
    assert!(audit(&value).is_empty());
}

#[test]
fn unmodeled_trigger_and_effect_contracts_are_explicit_gaps() {
    let findings = audit(
        &json!({"card": {}, "abilities": [triggered(json!({"FutureTrigger": {}}), vec![effect("FutureEffect", json!({"player": "IteratedPlayer"}))])]}),
    );
    assert!(errors(&findings).is_empty());
    assert!(
        findings
            .iter()
            .any(|finding| finding.code == "unknown_trigger_contract")
    );
    assert!(
        findings
            .iter()
            .any(|finding| finding.code == "unknown_effect_contract")
    );
}

#[test]
fn iterated_choices_and_ability_only_values_have_distinct_requirements() {
    let findings = audit(&definition(vec![
        effect("DestroyEffect", json!({"target": "Iterated"})),
        loss(json!("ThisAbilityResolvedThisTurnCount")),
    ]));
    assert_eq!(errors(&findings).len(), 2, "{findings:?}");
    let within_object_loop = audit(&definition(vec![effect(
        "ForEachObject",
        json!({"filter": {}, "effects": [effect("DestroyEffect", json!({"target": "Iterated"}))]}),
    )]));
    assert!(within_object_loop.is_empty(), "{within_object_loop:?}");
    let ability = audit(
        &json!({"card": {}, "abilities": [triggered(json!({"BeginningOfUpkeep": {"player": "You"}}), vec![loss(json!("ThisAbilityResolvedThisTurnCount"))])]}),
    );
    assert!(ability.is_empty(), "{ability:?}");
}

#[test]
fn cast_specific_event_values_require_a_cast_event() {
    let findings = audit(&json!({"card": {}, "abilities": [
        triggered(json!({"SpellCast": {"caster": "Any", "filter": null}}), vec![loss(json!("ManaSpentToCastTriggeringObject"))]),
        triggered(json!({"BeginningOfUpkeep": {"player": "Any"}}), vec![loss(json!("ManaSpentToCastTriggeringObject"))]),
    ]}));
    assert_eq!(errors(&findings).len(), 1, "{findings:?}");
    assert!(findings[0].path.starts_with("/abilities/1/"));
}

#[test]
fn prevention_followups_receive_future_damage_amount_without_inheriting_spell_scope() {
    let findings = audit(&definition(vec![effect(
        "PreventDamageEffect",
        json!({"amount": {"Fixed": 3}, "target": "AnyTarget", "follow_up_effects": [
            loss(json!({"EventValue": "Amount"})),
            loss(json!({"EventValue": "DieResult"})),
        ]}),
    )]));
    assert_eq!(errors(&findings).len(), 1, "{findings:?}");
    assert!(findings[0].message.contains("DieResult"));
}

#[test]
fn targeted_chooser_binds_relative_filter_and_count_only_locally() {
    let findings = audit(&definition(vec![
        effect(
            "ChooseObjectsEffect",
            json!({
                "chooser": {"Target": "Any"},
                "filter": {"controller": "IteratedPlayer"},
                "count_value": {"CardsInHand": "IteratedPlayer"},
            }),
        ),
        loss(json!({"LifeTotal": "IteratedPlayer"})),
    ]));
    assert_eq!(errors(&findings).len(), 1, "{findings:?}");
    assert!(findings[0].path.contains("/default_effects/1/"));
}

#[test]
fn event_only_conditions_detect_silent_false_branches_on_activated_abilities() {
    let conditions = vec![
        json!({"TriggeringObjectHadCounters": {"counter_type": "Verse", "min_count": 4}}),
        json!("TriggeringObjectWasEnchanted"),
        json!("EvolveEnteringCreatureIsLarger"),
        json!({"TurnHistory": {"TriggeringObjectWasCastFromZone": "Hand"}}),
    ];
    let effects: Vec<_> = conditions.iter().map(|condition| effect("ConditionalEffect", json!({"condition": condition, "if_true": [loss(json!({"Fixed": 1}))], "if_false": []}))).collect();
    let findings = audit(
        &json!({"card": {}, "abilities": [{"kind": {"Activated": {"effects": program(effects.clone()), "choices": []}}}]}),
    );
    assert_eq!(errors(&findings).len(), 4, "{findings:?}");
    assert!(
        findings
            .iter()
            .all(|finding| finding.code == "condition_without_event")
    );
    let death_trigger =
        audit(&json!({"card": {}, "abilities": [triggered(json!("ThisDies"), effects)]}));
    assert!(errors(&death_trigger).is_empty(), "{death_trigger:?}");
}

#[test]
fn tagging_an_event_object_requires_event_and_object_independently() {
    let tag = || effect("TagTriggeringObjectEffect", json!({"tag": "triggering"}));
    let value = json!({"card": {}, "spell_effect": program(vec![tag()]), "abilities": [
        triggered(json!({"PlayerLosesLife": {"player": "Any"}}), vec![tag()]),
        triggered(json!({"ThisDealsDamageToPlayer": {}}), vec![tag()]),
        triggered(json!({"BeginningOfUpkeep": {"player": "You"}}), vec![tag()]),
    ]});
    let findings = audit(&value);
    assert_eq!(errors(&findings).len(), 3, "{findings:?}");
    assert!(
        !findings.iter().any(|f| f.path.starts_with("/abilities/1/")),
        "{findings:?}"
    );
}

#[test]
fn saga_chapters_supply_counter_amount_and_object_but_no_player() {
    let findings = audit(&json!({"card": {}, "abilities": [triggered(
    json!({"SagaChapter": {"chapters": [1]}}), vec![
        effect("TagTriggeringObjectEffect", json!({"tag": "triggering"})),
        loss(json!({"EventValue": "Amount"})),
        loss(json!({"LifeTotal": "IteratedPlayer"})),
    ])]}));
    assert_eq!(errors(&findings).len(), 1, "{findings:?}");
    assert!(
        !findings
            .iter()
            .any(|f| f.code == "unknown_trigger_contract"),
        "{findings:?}"
    );
}

#[test]
fn intervening_value_conditions_reconstruct_only_the_events_player_binding() {
    let condition = json!({"ValueComparison": {
        "left": {"LandsEnteredBattlefieldThisTurn": "IteratedPlayer"},
        "operator": "GreaterThanOrEqual", "right": {"Fixed": 2}
    }});
    let mut land = triggered(json!({"EntersBattlefield": {"filter": {}}}), vec![]);
    land["kind"]["Triggered"]["intervening_if"] = condition.clone();
    let mut life = triggered(json!({"PlayerLosesLife": {"player": "Any"}}), vec![]);
    life["kind"]["Triggered"]["intervening_if"] = condition;
    let findings = audit(&json!({"card": {}, "abilities": [land, life]}));
    assert_eq!(errors(&findings).len(), 1, "{findings:?}");
    assert!(
        errors(&findings)[0]
            .path
            .starts_with("/abilities/0/kind/Triggered/intervening_if/")
    );
}

#[test]
fn intervening_condition_cannot_read_an_outcome_from_later_resolution() {
    let mut ability = triggered(
        json!({"PlayerLosesLife": {"player": "Any"}}),
        vec![effect(
            "WithIdEffect",
            json!({"id": 7, "effect": loss(json!({"Fixed": 1}))}),
        )],
    );
    ability["kind"]["Triggered"]["intervening_if"] = json!({"ValueComparison": {
        "left": {"EffectValue": 7}, "operator": "GreaterThan", "right": {"Fixed": 0}
    }});
    let findings = audit(&json!({"card": {}, "abilities": [ability]}));
    assert_eq!(errors(&findings).len(), 1, "{findings:?}");
    assert_eq!(errors(&findings)[0].code, "missing_effect_outcome");
}

#[test]
fn condition_qualified_trigger_uses_trigger_time_value_context() {
    let findings = audit(&json!({"card": {}, "abilities": [triggered(
        json!({"ConditionQualified": {
            "trigger": {"kind": {"EntersBattlefield": {"filter": {}}}},
            "condition": {"ValueComparison": {
                "left": {"LandsEnteredBattlefieldThisTurn": "IteratedPlayer"},
                "operator": "GreaterThan", "right": {"Fixed": 1}
            }}
        }}), vec![]
    )]}));
    assert_eq!(errors(&findings).len(), 1, "{findings:?}");
    assert!(errors(&findings)[0].path.contains("/trigger/condition/"));
}

#[test]
fn generic_zone_change_destination_controls_the_player_contract() {
    let condition = json!({"ValueComparison": {
        "left": {"SurfaceHinted": {"hints": ["AnotherLandEnteredThisTurn"],
            "value": {"LandsEnteredBattlefieldThisTurn": "IteratedPlayer"}}},
        "operator": "GreaterThanOrEqual", "right": {"Fixed": 2}
    }});
    let mut land = triggered(
        json!({"ZoneChange": {"from": null, "to": "Battlefield", "filter": {}}}),
        vec![],
    );
    land["kind"]["Triggered"]["intervening_if"] = condition;
    let graveyard = triggered(
        json!({"ZoneChange": {"to": "Graveyard", "filter": {}}}),
        vec![loss(json!({"LifeTotal": "IteratedPlayer"}))],
    );
    let findings = audit(&json!({"card": {}, "abilities": [land, graveyard]}));
    assert_eq!(errors(&findings).len(), 1, "{findings:?}");
    assert!(errors(&findings)[0].path.contains("/intervening_if/"));
    assert!(
        findings
            .iter()
            .any(|f| f.path.starts_with("/abilities/1/") && f.severity == "coverage_gap")
    );
}

#[test]
fn tap_actor_trigger_binds_only_its_guaranteed_actor_object_and_count() {
    let actor = json!({"PlayerChangesTapState": {"player": "You", "filter": {}, "tapped": false, "one_or_more": true, "during_untap_step": "You"}});
    let value = json!({"card": {}, "abilities": [
        triggered(actor.clone(), vec![loss(json!({"Add": [{"LifeTotal": "IteratedPlayer"}, {"EventValue": "Amount"}]}))]),
        triggered(json!({"PermanentBecomesUntapped": {"filter": {}, "one_or_more": false}}), vec![loss(json!({"EventValue": "Amount"}))]),
        triggered(actor, vec![loss(json!({"EventValue": "DieResult"}))]),
        triggered(json!({"BeginningOfUpkeep": {"player": "Any"}}), vec![loss(json!({"EventValue": "Amount"}))]),
    ]});
    let findings = audit(&value);
    assert_eq!(errors(&findings).len(), 2, "{findings:?}");
    assert!(
        errors(&findings)
            .iter()
            .all(|finding| finding.path.starts_with("/abilities/2/")
                || finding.path.starts_with("/abilities/3/"))
    );
}

#[test]
fn attachment_event_does_not_claim_unrelated_event_amounts() {
    let model = json!({"AttachmentChanged": {"attachment": {}, "recipient": {}, "attached": true}});
    let value = json!({"card": {}, "abilities": [triggered(model, vec![loss(json!({"EventValue": "Amount"}))])]});
    assert_eq!(errors(&audit(&value)).len(), 1);
}

#[test]
fn caster_specific_mana_value_requires_a_cast_event_scope() {
    let value=json!({"card": {}, "abilities": [
        triggered(json!({"SpellCast": {"caster": "You", "filter": null}}), vec![loss(json!("CasterManaSpentToCastTriggeringObject"))]),
        triggered(json!({"BeginningOfUpkeep": {"player": "Any"}}), vec![loss(json!("CasterManaSpentToCastTriggeringObject"))]),
    ]});
    assert_eq!(errors(&audit(&value)).len(),1);
}

#[test]
fn permanent_lifecycle_contracts_separate_event_object_from_actor() {
    for (kind, has_player) in [("PermanentTransforms", false), ("PermanentTransformsInto", false), ("PermanentMutates", true), ("PlayerTurnsFaceUp", true)] {
        let findings = audit(&json!({"card": {}, "abilities": [triggered(json!({kind: {}}), vec![
            effect("TagTriggeringObjectEffect", json!({"tag": "triggering"})),
            loss(json!({"LifeTotal": "IteratedPlayer"})),
        ])]}));
        assert_eq!(errors(&findings).len(), usize::from(!has_player), "{kind}: {findings:?}");
        assert!(!findings.iter().any(|finding| finding.code == "unknown_trigger_contract"), "{findings:?}");
    }
}
