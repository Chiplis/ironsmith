use super::*;
use crate::ability::AbilityKind;

#[test]
fn for_mirrodin_adds_etb_create_and_attach_trigger() {
    let def = CardDefinitionBuilder::new(CardId::new(), "For Mirrodin Variant")
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Equipment])
        .for_mirrodin()
        .build();

    let ability = def
        .abilities
        .iter()
        .find(|ability| matches!(&ability.kind, AbilityKind::Triggered(_)))
        .expect("expected For Mirrodin ability");
    let AbilityKind::Triggered(triggered) = &ability.kind else {
        panic!("expected For Mirrodin to add a triggered ability");
    };

    let debug = format!("{triggered:?}").to_ascii_lowercase();
    assert!(
        debug.contains("createtokeneffect")
            && debug.contains("rebel")
            && debug.contains("attachtoeffect"),
        "expected For Mirrodin trigger to create Rebel token and attach equipment, got {debug}"
    );
}

#[test]
fn living_weapon_adds_etb_create_and_attach_trigger() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Living Weapon Variant")
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Equipment])
        .living_weapon()
        .build();

    let ability = def
        .abilities
        .iter()
        .find(|ability| matches!(&ability.kind, AbilityKind::Triggered(_)))
        .expect("expected Living weapon ability");
    let AbilityKind::Triggered(triggered) = &ability.kind else {
        panic!("expected Living weapon to add a triggered ability");
    };

    let debug = format!("{triggered:?}").to_ascii_lowercase();
    assert!(
        debug.contains("createtokeneffect")
            && debug.contains("phyrexian")
            && debug.contains("germ")
            && debug.contains("phyrexian germ")
            && debug.contains("attachtoeffect"),
        "expected Living weapon trigger to create Germ token and attach equipment, got {debug}"
    );
}

#[test]
fn myriad_adds_attack_trigger_with_primitive_composition() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Myriad Variant")
        .card_types(vec![CardType::Creature])
        .myriad()
        .build();

    let ability = def
        .abilities
        .iter()
        .find(|ability| matches!(&ability.kind, AbilityKind::Triggered(_)))
        .expect("expected Myriad ability");
    let AbilityKind::Triggered(triggered) = &ability.kind else {
        panic!("expected Myriad to add a triggered ability");
    };

    let debug = format!("{triggered:?}");
    assert!(
        debug.contains("ForPlayersEffect")
            && debug.contains("MayEffect")
            && debug.contains("CreateTokenCopyEffect")
            && !debug.contains("MyriadTokenCopiesEffect"),
        "expected composed myriad trigger (for-players + may + create-copy), got {debug}"
    );
}

#[test]
fn undying_keyword_uses_trigger_intervening_if() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Undying Variant")
        .card_types(vec![CardType::Creature])
        .undying()
        .build();

    let ability = def
        .abilities
        .iter()
        .find(|ability| matches!(&ability.kind, AbilityKind::Triggered(_)))
        .expect("expected Undying ability");
    let AbilityKind::Triggered(triggered) = &ability.kind else {
        panic!("expected Undying to add a triggered ability");
    };

    let debug = format!("{triggered:?}");
    assert!(
        debug.contains("TriggeringObjectHadCounters")
            && debug.contains("PlusOnePlusOne")
            && !debug.contains("KeywordAbilityTriggerKind::Undying"),
        "expected undying keyword to compile through generic trigger+condition path, got {debug}"
    );
}

#[test]
fn persist_keyword_uses_trigger_intervening_if() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Persist Variant")
        .card_types(vec![CardType::Creature])
        .persist()
        .build();

    let ability = def
        .abilities
        .iter()
        .find(|ability| matches!(&ability.kind, AbilityKind::Triggered(_)))
        .expect("expected Persist ability");
    let AbilityKind::Triggered(triggered) = &ability.kind else {
        panic!("expected Persist to add a triggered ability");
    };

    let debug = format!("{triggered:?}");
    assert!(
        debug.contains("TriggeringObjectHadCounters")
            && debug.contains("MinusOneMinusOne")
            && !debug.contains("KeywordAbilityTriggerKind::Persist"),
        "expected persist keyword to compile through generic trigger+condition path, got {debug}"
    );
}

#[test]
fn bare_vanishing_adds_decay_triggers_without_entry_counter_ability() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Bare Vanishing Variant")
        .card_types(vec![CardType::Creature])
        .vanishing(0)
        .build();

    assert_eq!(
        def.abilities
            .iter()
            .filter(|ability| matches!(&ability.kind, AbilityKind::Triggered(_)))
            .count(),
        2
    );
    assert!(
        def.abilities.iter().all(|ability| {
            !matches!(
                &ability.kind,
                AbilityKind::Static(static_ability)
                    if static_ability.id()
                        == crate::static_abilities::StaticAbilityId::EnterWithCounters
            )
        }),
        "bare Vanishing must not invent an entry counter count"
    );
    let triggered = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(format!("{triggered:?}")),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(
        triggered[0].contains("BeginningOfUpkeepTrigger")
            && triggered[0].contains("RemoveCountersEffect")
            && triggered[0].contains("Time"),
        "the first decay trigger must remove a time counter: {}",
        triggered[0]
    );
    assert!(
        triggered[1].contains("CounterRemovedFromTrigger")
            && triggered[1].contains("SourceHasNoCounter(Time)")
            && triggered[1].contains("SacrificeTargetEffect"),
        "the second decay trigger must sacrifice the counterless source: {}",
        triggered[1]
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_undying_oracle_text_with_snapshot_counter_predicate() {
    let text = "When this creature dies, if it had no +1/+1 counters on it, return it to the battlefield under its owner's control with a +1/+1 counter on it.";
    let def = CardDefinitionBuilder::new(CardId::new(), "Undying Oracle Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(text)
        .expect("undying oracle text should parse");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("TriggeringObjectHadCounters")
            && debug.contains("PlusOnePlusOne")
            && !debug.contains("UnsupportedParserLine"),
        "expected undying oracle text to compile with snapshot counter predicate, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_persist_oracle_text_with_snapshot_counter_predicate() {
    let text = "When this creature dies, if it had no -1/-1 counters on it, return it to the battlefield under its owner's control with a -1/-1 counter on it.";
    let def = CardDefinitionBuilder::new(CardId::new(), "Persist Oracle Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(text)
        .expect("persist oracle text should parse");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("TriggeringObjectHadCounters")
            && debug.contains("MinusOneMinusOne")
            && !debug.contains("UnsupportedParserLine"),
        "expected persist oracle text to compile with snapshot counter predicate, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_self_enters_with_x_counters_is_typed_static() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Self ETB X Counter Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("This creature enters with X +1/+1 counters on it.")
        .expect("self etb x counters should parse");

    let static_ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        static_ids.contains(&crate::static_abilities::StaticAbilityId::EnterWithCounters),
        "expected typed enters-with-counters static ability, got {static_ids:?}"
    );
    assert!(
        !static_ids.contains(&crate::static_abilities::StaticAbilityId::RuleFallbackText),
        "self etb x counters should not remain a placeholder static ability: {static_ids:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_self_enters_with_opponent_lost_life_is_typed_static() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Self ETB Opponent Lost Life Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "This creature enters with a +1/+1 counter on it if an opponent lost life this turn.",
        )
        .expect("self etb opponent-life-loss conditional should parse");

    let static_ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        static_ids
            .contains(&crate::static_abilities::StaticAbilityId::EnterWithCountersIfCondition),
        "expected conditional enters-with-counters ability, got {static_ids:?}"
    );
    assert!(
        !static_ids.contains(&crate::static_abilities::StaticAbilityId::RuleFallbackText),
        "self etb opponent-life-loss conditional should not remain placeholder fallback: {static_ids:?}"
    );
}
