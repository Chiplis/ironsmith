use super::*;
use crate::ability::AbilityKind;
use crate::color::Color;
use crate::compiled_text::{compiled_lines, oracle_like_lines};
use crate::effects::{
    AddManaEffect, ChooseModeEffect, CreateTokenEffect, GainLifeEffect,
    ReturnFromGraveyardToHandEffect,
};
use crate::static_abilities::StaticAbilityId;
use crate::target::{ChooseSpec, PlayerFilter};
use crate::{ObjectId, PlayerId};
use std::collections::HashMap;
use std::sync::OnceLock;

#[test]
fn test_creature_with_keywords() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Test Creature")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::White],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Angel])
        .power_toughness(PowerToughness::fixed(3, 3))
        .flying()
        .vigilance()
        .build();

    assert_eq!(def.name(), "Test Creature");
    assert!(def.is_creature());
    assert_eq!(def.abilities.len(), 2);
}

#[test]
fn test_creature_with_mana_ability() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Mana Dork")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Green]]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Elf, Subtype::Druid])
        .power_toughness(PowerToughness::fixed(1, 1))
        .taps_for(ManaSymbol::Green)
        .build();

    assert!(def.is_creature());
    assert_eq!(def.abilities.len(), 1);
    assert!(def.abilities[0].is_mana_ability());
}

#[test]
fn test_spell_with_effects() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Test Bolt")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Red]]))
        .card_types(vec![CardType::Instant])
        .with_spell_effect(vec![Effect::deal_damage(3, ChooseSpec::AnyTarget)])
        .build();

    assert!(def.is_spell());
    assert!(def.spell_effect.is_some());
    assert_eq!(def.spell_effect.as_ref().unwrap().len(), 1);
}

#[test]
fn test_creature_with_etb() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "ETB Creature")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Blue],
        ]))
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .with_etb(vec![Effect::draw(1)])
        .build();

    assert_eq!(def.abilities.len(), 1);
    let ability = &def.abilities[0];
    // Check that the trigger is an ETB trigger (now using Trigger struct)
    if let AbilityKind::Triggered(t) = &ability.kind {
        assert!(t.trigger.display().contains("enters"));
    } else {
        panic!("Expected triggered ability");
    }
}

#[test]
fn test_protection_from_color() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Protected")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::White],
        ]))
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .protection_from(ColorSet::from(Color::Red))
        .build();

    assert_eq!(def.abilities.len(), 1);
}

#[test]
fn test_land_with_mana_ability() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Forest")
        .supertypes(vec![Supertype::Basic])
        .card_types(vec![CardType::Land])
        .subtypes(vec![Subtype::Forest])
        .taps_for(ManaSymbol::Green)
        .build();

    assert!(def.card.is_land());
    assert_eq!(def.abilities.len(), 1);
    assert!(def.abilities[0].is_mana_ability());
}

#[test]
fn test_complex_creature() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Complex Creature")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::Black],
            vec![ManaSymbol::Black],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Vampire])
        .power_toughness(PowerToughness::fixed(2, 3))
        .flying()
        .deathtouch()
        .lifelink()
        .build();

    assert_eq!(def.abilities.len(), 3);
    assert!(def.is_creature());
}

#[test]
fn test_builder_mentor_creates_targeted_attack_trigger() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Mentor Test")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 3))
        .mentor()
        .build();

    assert_eq!(def.abilities.len(), 1);
    let ability = &def.abilities[0];
    match &ability.kind {
        AbilityKind::Triggered(triggered) => {
            assert!(triggered.trigger.display().contains("attacks"));
            assert_eq!(triggered.choices.len(), 1);
            let choices_debug = format!("{:?}", triggered.choices);
            assert!(
                choices_debug.contains("attacking: true")
                    && choices_debug.contains("power_relative_to_source: Some(LessThanSource)"),
                "expected mentor target restriction, got {choices_debug}"
            );
        }
        _ => panic!("expected triggered ability"),
    }
}

#[test]
fn test_builder_afterlife_creates_dies_trigger_with_tokens() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Afterlife Test")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .afterlife(2)
        .build();

    assert_eq!(def.abilities.len(), 1);
    let ability = &def.abilities[0];
    match &ability.kind {
        AbilityKind::Triggered(triggered) => {
            assert!(triggered.trigger.display().contains("dies"));
            let effects_debug = format!("{:?}", triggered.effects);
            assert!(
                effects_debug.contains("CreateTokenEffect"),
                "expected token creation effect, got {effects_debug}"
            );
        }
        _ => panic!("expected triggered ability"),
    }
}

#[test]
fn test_builder_fabricate_creates_etb_modal_trigger() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Fabricate Test")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .fabricate(1)
        .build();

    assert_eq!(def.abilities.len(), 1);
    let ability = &def.abilities[0];
    match &ability.kind {
        AbilityKind::Triggered(triggered) => {
            assert!(triggered.trigger.display().contains("enters"));
            let effects_debug = format!("{:?}", triggered.effects);
            assert!(
                effects_debug.contains("ChooseModeEffect"),
                "expected modal fabricate effect, got {effects_debug}"
            );
        }
        _ => panic!("expected triggered ability"),
    }
}

#[test]
fn test_builder_soulshift_creates_dies_trigger_with_graveyard_target() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Soulshift Test")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .soulshift(3)
        .build();

    assert_eq!(def.abilities.len(), 1);
    let ability = &def.abilities[0];
    assert_eq!(ability.text.as_deref(), Some("Soulshift 3"));
    match &ability.kind {
        AbilityKind::Triggered(triggered) => {
            assert!(triggered.trigger.display().contains("dies"));
            assert_eq!(triggered.choices.len(), 1);
            let debug = format!("{:?}", triggered.effects);
            assert!(
                debug.contains("ReturnFromGraveyardToHandEffect"),
                "expected soulshift recursion effect, got {debug}"
            );
        }
        _ => panic!("expected triggered soulshift ability"),
    }
}

#[test]
fn test_builder_outlast_creates_sorcery_speed_activated_ability() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Outlast Test")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .outlast(ManaCost::from_symbols(vec![ManaSymbol::White]))
        .build();

    assert_eq!(def.abilities.len(), 1);
    let ability = &def.abilities[0];
    assert_eq!(ability.text.as_deref(), Some("Outlast {W}"));
    match &ability.kind {
        AbilityKind::Activated(activated) => {
            assert_eq!(
                activated.timing,
                crate::ability::ActivationTiming::SorcerySpeed
            );
            let cost_text = activated.mana_cost.display().to_ascii_lowercase();
            assert!(
                cost_text.contains("{w}") && cost_text.contains("{t}"),
                "expected outlast mana+tap cost, got {cost_text}"
            );
            let debug = format!("{:?}", activated.effects);
            assert!(
                debug.contains("PutCountersEffect"),
                "expected +1/+1 counter effect, got {debug}"
            );
        }
        _ => panic!("expected activated outlast ability"),
    }
}

#[test]
fn test_builder_extort_creates_spell_cast_trigger_with_optional_payment() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Extort Test")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .extort()
        .build();

    assert_eq!(def.abilities.len(), 1);
    let ability = &def.abilities[0];
    assert_eq!(ability.text.as_deref(), Some("Extort"));
    match &ability.kind {
        AbilityKind::Triggered(triggered) => {
            assert!(triggered.trigger.display().contains("you cast"));
            let debug = format!("{:?}", triggered.effects);
            assert!(
                debug.contains("PayManaEffect"),
                "expected extort payment effect, got {debug}"
            );
            assert!(
                debug.contains("ForPlayersEffect"),
                "expected extort opponent-drain loop, got {debug}"
            );
        }
        _ => panic!("expected triggered extort ability"),
    }
}

#[test]
fn test_builder_partner_creates_keyword_static_ability() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Partner Test")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .partner()
        .build();

    assert_eq!(def.abilities.len(), 1);
    let ability = &def.abilities[0];
    assert_eq!(ability.text.as_deref(), Some("Partner"));
    match &ability.kind {
        AbilityKind::Static(static_ability) => {
            assert_eq!(static_ability.id(), StaticAbilityId::Partner);
        }
        _ => panic!("expected static partner ability"),
    }
}

#[test]
fn test_builder_assist_creates_keyword_static_ability() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Assist Test")
        .card_types(vec![CardType::Sorcery])
        .assist()
        .build();

    assert_eq!(def.abilities.len(), 1);
    let ability = &def.abilities[0];
    assert_eq!(ability.text.as_deref(), Some("Assist"));
    match &ability.kind {
        AbilityKind::Static(static_ability) => {
            assert_eq!(static_ability.id(), StaticAbilityId::Assist);
        }
        _ => panic!("expected static assist ability"),
    }
}

#[test]
fn test_builder_modular_creates_enters_counter_and_dies_transfer() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Modular Test")
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .modular(2)
        .build();

    assert_eq!(def.abilities.len(), 2);
    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("EntersWithCounters"),
        "expected enters-with-counters ability, got {debug}"
    );
    assert!(
        debug.contains("ZoneChangeTrigger") && debug.contains("PutCountersEffect"),
        "expected dies transfer trigger for modular, got {debug}"
    );
}

#[test]
fn test_builder_graft_creates_enters_counter_and_etb_move_trigger() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Graft Test")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(0, 0))
        .graft(2)
        .build();

    assert_eq!(def.abilities.len(), 2);
    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("EntersWithCounters"),
        "expected enters-with-counters ability, got {debug}"
    );
    assert!(
        debug.contains("ZoneChangeTrigger") && debug.contains("MoveCountersEffect"),
        "expected graft move-counter trigger, got {debug}"
    );
}

#[test]
fn test_builder_sunburst_creature_uses_plus_one_counters() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Sunburst Creature Test")
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .power_toughness(PowerToughness::fixed(0, 0))
        .sunburst()
        .build();

    assert_eq!(def.abilities.len(), 1);
    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("EntersWithCounters"),
        "expected enters-with-counters replacement, got {debug}"
    );
    assert!(
        debug.contains("ColorsOfManaSpentToCastThisSpell"),
        "expected sunburst to scale from colors spent to cast, got {debug}"
    );
    assert!(
        debug.contains("PlusOnePlusOne"),
        "expected creature sunburst to use +1/+1 counters, got {debug}"
    );
}

#[test]
fn test_builder_sunburst_noncreature_uses_charge_counters() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Sunburst Artifact Test")
        .card_types(vec![CardType::Artifact])
        .sunburst()
        .build();

    assert_eq!(def.abilities.len(), 1);
    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("EntersWithCounters"),
        "expected enters-with-counters replacement, got {debug}"
    );
    assert!(
        debug.contains("ColorsOfManaSpentToCastThisSpell"),
        "expected sunburst to scale from colors spent to cast, got {debug}"
    );
    assert!(
        debug.contains("Charge"),
        "expected noncreature sunburst to use charge counters, got {debug}"
    );
}

#[test]
fn test_builder_arcbound_wanderer_modular_sunburst_renders_full_semantics() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Arcbound Wanderer")
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .power_toughness(PowerToughness::fixed(0, 0))
        .modular_sunburst()
        .build();

    let rendered = crate::compiled_text::compiled_lines(&def)
        .join("\n")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("modular—sunburst"),
        "expected modular sunburst keyword wording, got {rendered}"
    );
    assert!(
        rendered.contains("+1/+1 counters") && rendered.contains("target creature"),
        "expected modular death-transfer wording, got {rendered}"
    );
    assert!(
        !rendered.contains("target artifact or creature"),
        "modular target must remain artifact creature, got {rendered}"
    );
}

#[test]
fn test_builder_fading_creates_counter_upkeep_and_sacrifice_triggers() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Fading Test")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .fading(2)
        .build();

    assert_eq!(def.abilities.len(), 3);
    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("EntersWithCounters") && debug.contains("Fade"),
        "expected fading ETB fade counters, got {debug}"
    );
    assert!(
        debug.contains("BeginningOfUpkeepTrigger") && debug.contains("RemoveCountersEffect"),
        "expected fading upkeep counter removal trigger, got {debug}"
    );
    assert!(
        debug.contains("CounterRemovedFromTrigger")
            && debug.contains("SourceHasNoCounter(Fade)")
            && debug.contains("SacrificeTargetEffect"),
        "expected fading last-counter sacrifice trigger, got {debug}"
    );
}

#[test]
fn test_builder_vanishing_creates_counter_upkeep_and_sacrifice_triggers() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Vanishing Test")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .vanishing(3)
        .build();

    assert_eq!(def.abilities.len(), 3);
    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("EntersWithCounters") && debug.contains("Time"),
        "expected vanishing ETB time counters, got {debug}"
    );
    assert!(
        debug.contains("BeginningOfUpkeepTrigger") && debug.contains("RemoveCountersEffect"),
        "expected vanishing upkeep counter removal trigger, got {debug}"
    );
    assert!(
        debug.contains("CounterRemovedFromTrigger")
            && debug.contains("SourceHasNoCounter(Time)")
            && debug.contains("SacrificeTargetEffect"),
        "expected vanishing last-counter sacrifice trigger, got {debug}"
    );
}

#[test]
fn test_builder_devour_creates_etb_triggered_effect_without_marker_fallback() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Devour Test")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .devour(2)
        .build();

    assert_eq!(def.abilities.len(), 1);
    let ability = &def.abilities[0];
    assert_eq!(ability.text.as_deref(), Some("Devour 2"));
    match &ability.kind {
        AbilityKind::Triggered(triggered) => {
            assert!(triggered.trigger.display().contains("enters"));
            let debug = format!("{:?}", triggered.effects);
            assert!(
                debug.contains("DevourEffect"),
                "expected explicit devour runtime effect, got {debug}"
            );
            assert!(
                !debug.contains("KeywordMarker"),
                "devour should not lower to a keyword marker, got {debug}"
            );
        }
        _ => panic!("expected triggered devour ability"),
    }
}

#[test]
fn test_builder_bloodthirst_creates_real_static_ability() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Bloodthirst Test")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .bloodthirst(3)
        .build();

    assert_eq!(def.abilities.len(), 1);
    let ability = &def.abilities[0];
    assert_eq!(ability.text.as_deref(), Some("Bloodthirst 3"));
    let debug = format!("{ability:?}");
    assert!(
        debug.contains("Bloodthirst"),
        "expected bloodthirst static ability, got {debug}"
    );
    assert!(
        !debug.contains("KeywordMarker"),
        "bloodthirst should not lower to a keyword marker, got {debug}"
    );
}

#[test]
fn test_parse_soulshift_keyword_line_compiles_keyword_text() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Soulshift Parse Test")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Soulshift 2 (When this creature dies, you may return target Spirit card with mana value 2 or less from your graveyard to your hand.)",
        )
        .expect("soulshift keyword line should parse");

    let rendered = oracle_like_lines(&def).join(" ");
    assert!(
        rendered.contains("Soulshift 2"),
        "expected soulshift keyword render, got {rendered}"
    );
}

#[test]
fn test_parse_outlast_keyword_line_compiles_keyword_text() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Outlast Parse Test")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Outlast {W} ({W}, {T}: Put a +1/+1 counter on this creature. Activate only as a sorcery.)",
        )
        .expect("outlast keyword line should parse");

    let rendered = oracle_like_lines(&def).join(" ");
    assert!(
        rendered.contains("Outlast {W}"),
        "expected outlast keyword render, got {rendered}"
    );
}

#[test]
fn test_parse_devour_keyword_line_compiles_without_unsupported_marker() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Devour Parse Test")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Devour 2 (As this creature enters, you may sacrifice any number of creatures. This creature enters with twice that many +1/+1 counters on it.)",
        )
        .expect("devour keyword line should parse");

    let rendered = oracle_like_lines(&def).join(" ").to_ascii_lowercase();
    let debug = format!("{def:#?}");
    assert!(
        rendered.contains("devour 2"),
        "expected devour keyword render, got {rendered}"
    );
    assert!(
        !debug.contains("KeywordMarker")
            && !debug.contains("RuleTextPlaceholder")
            && !debug.contains("UnsupportedParserLine"),
        "devour parse should avoid unsupported placeholders, got {debug}"
    );
}

#[test]
fn test_parse_bloodthirst_keyword_line_compiles_without_unsupported_marker() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Bloodthirst Parse Test")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Bloodthirst 3 (If an opponent was dealt damage this turn, this creature enters with three +1/+1 counters on it.)",
        )
        .expect("bloodthirst keyword line should parse");

    let rendered = oracle_like_lines(&def).join(" ").to_ascii_lowercase();
    let debug = format!("{def:#?}");
    assert!(
        rendered.contains("bloodthirst 3"),
        "expected bloodthirst keyword render, got {rendered}"
    );
    assert!(
        !debug.contains("KeywordMarker")
            && !debug.contains("RuleTextPlaceholder")
            && !debug.contains("UnsupportedParserLine"),
        "bloodthirst parse should avoid unsupported placeholders, got {debug}"
    );
}

#[test]
fn test_parse_backup_keyword_line_compiles_to_explicit_etb_trigger() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Backup Parse Test")
        .card_types(vec![CardType::Creature])
        .parse_text("Backup 1\nFlying")
        .expect("backup keyword line should parse");

    let rendered = oracle_like_lines(&def).join(" ").to_ascii_lowercase();
    let debug = format!("{def:#?}");
    assert!(
        rendered.contains("backup 1"),
        "expected backup keyword render, got {rendered}"
    );
    assert!(
        debug.contains("BackupEffect"),
        "expected backup to lower to an explicit effect, got {debug}"
    );
    assert!(
        !debug.contains("KeywordMarker"),
        "backup parse should not leave a placeholder marker, got {debug}"
    );
}

#[test]
fn test_parse_repeated_backup_keyword_line_compiles_each_instance() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Repeated Backup Parse Test")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Backup 1, backup 1, backup 1 (When this creature enters, put a +1/+1 counter on target creature. If that's another creature, it gains the following abilities until end of turn. Each backup ability triggers separately.)\nFlying",
        )
        .expect("repeated backup keyword line should parse");

    let debug = format!("{def:#?}");
    let backup_effect_count = debug.matches("BackupEffect").count();
    assert_eq!(
        backup_effect_count, 3,
        "expected three backup triggers, got {debug}"
    );
    assert!(
        !debug.contains("KeywordFallbackText"),
        "repeated backup should not remain a fallback marker, got {debug}"
    );
}

#[test]
fn test_parse_plain_vanishing_keyword_line_compiles_without_marker() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Vanishing Parse Test")
        .card_types(vec![CardType::Enchantment])
        .parse_text("Vanishing")
        .expect("plain vanishing keyword line should parse");

    let rendered = oracle_like_lines(&def).join(" ");
    let debug = format!("{def:#?}");
    assert!(
        rendered.contains("Vanishing"),
        "expected vanishing keyword render, got {rendered}"
    );
    assert!(
        !debug.contains("KeywordMarker"),
        "plain vanishing should not remain a marker, got {debug}"
    );
}

#[test]
fn test_parse_extort_keyword_line_compiles_keyword_text() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Extort Parse Test")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Extort (Whenever you cast a spell, you may pay {W/B}. If you do, each opponent loses 1 life and you gain that much life.)",
        )
        .expect("extort keyword line should parse");

    let rendered = oracle_like_lines(&def).join(" ");
    assert!(
        rendered.contains("Extort"),
        "expected extort keyword render, got {rendered}"
    );
}

#[test]
fn test_parse_partner_keyword_line_compiles_keyword_text() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Partner Parse Test")
        .card_types(vec![CardType::Creature])
        .parse_text("Partner")
        .expect("partner keyword line should parse");

    let rendered = oracle_like_lines(&def).join(" ");
    assert!(
        rendered.contains("Partner"),
        "expected partner keyword render, got {rendered}"
    );
}

#[test]
fn test_parse_partner_with_keyword_line_fails_loudly() {
    let err = CardDefinitionBuilder::new(CardId::from_raw(1), "Partner With Parse Test")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Partner with Proud Mentor (When this creature enters, target player may put Proud Mentor into their hand from their library, then shuffle.)",
        )
        .expect_err("partner-with keyword line should fail loudly until supported");
    let message = format!("{err:?}");
    assert!(
        message.contains("unsupported partner-with keyword line")
            && message.contains("[rule=partner-with-keyword-line]"),
        "expected targeted partner-with diagnostic, got {message}"
    );
}

#[test]
fn test_parse_assist_keyword_line_compiles_keyword_text() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Assist Parse Test")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Assist")
        .expect("assist keyword line should parse");

    let rendered = oracle_like_lines(&def).join(" ");
    assert!(
        rendered.contains("Assist"),
        "expected assist keyword render, got {rendered}"
    );
}

#[test]
fn test_parse_modular_keyword_line_compiles_keyword_text() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Modular Parse Test")
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .parse_text("Modular 1 (This creature enters with a +1/+1 counter on it.)")
        .expect("modular keyword line should parse");

    let rendered = oracle_like_lines(&def).join(" ");
    assert!(
        rendered.contains("Modular 1"),
        "expected modular keyword render, got {rendered}"
    );
}

#[test]
fn test_parse_graft_keyword_line_compiles_keyword_text() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Graft Parse Test")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Graft 2 (This creature enters with two +1/+1 counters on it. Whenever another creature enters, you may move a +1/+1 counter from this creature onto it.)",
        )
        .expect("graft keyword line should parse");

    let rendered = oracle_like_lines(&def).join(" ");
    assert!(
        rendered.contains("Graft 2"),
        "expected graft keyword render, got {rendered}"
    );
}

#[test]
fn test_parse_sunburst_keyword_line_compiles_keyword_text() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Sunburst Parse Test")
        .card_types(vec![CardType::Artifact])
        .parse_text("Sunburst")
        .expect("sunburst keyword line should parse");

    let rendered = oracle_like_lines(&def).join(" ");
    assert!(
        rendered.contains("Sunburst"),
        "expected sunburst keyword render, got {rendered}"
    );
}

#[test]
fn test_parse_fading_keyword_line_compiles_keyword_text() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Fading Parse Test")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Fading 2 (This creature enters with two fade counters on it. At the beginning of your upkeep, remove a fade counter from it. If you can't, sacrifice it.)",
        )
        .expect("fading keyword line should parse");

    let rendered = oracle_like_lines(&def).join(" ");
    assert!(
        rendered.contains("Fading 2"),
        "expected fading keyword render, got {rendered}"
    );
}

#[test]
fn test_parse_cant_gain_life_from_text() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "No Life")
        .parse_text("Players can't gain life.")
        .expect("parse players can't gain life");

    let has_cant_gain = def.abilities.iter().any(|ability| {
        matches!(
            &ability.kind,
            AbilityKind::Static(ability) if ability.id() == StaticAbilityId::PlayersCantGainLife
        )
    });

    assert!(has_cant_gain);
}

#[test]
fn test_parse_deafening_silence_noncreature_cast_limit() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Deafening Silence Variant")
        .parse_text("Each player can't cast more than one noncreature spell each turn.")
        .expect("parse each-player noncreature cast limit");

    let has_rule_restriction = def.abilities.iter().any(|ability| {
        matches!(
            &ability.kind,
            AbilityKind::Static(ability) if ability.id() == StaticAbilityId::RuleRestriction
        )
    });
    assert!(
        has_rule_restriction,
        "expected a rule restriction static ability"
    );

    let rendered = oracle_like_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("each player can't cast more than one noncreature spell each turn")
            || rendered.contains("each player cant cast more than one noncreature spell each turn"),
        "expected deafening silence cast-limit text, got {rendered}"
    );
}

#[test]
fn test_parse_you_cant_cast_more_than_one_spell_each_turn() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Moderation Variant")
        .parse_text("You can't cast more than one spell each turn.")
        .expect("parse you-cant-cast-more-than-one-spell restriction");

    let rendered = oracle_like_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("you can't cast more than one spell each turn")
            || rendered.contains("you cant cast more than one spell each turn"),
        "expected player-scoped cast-limit text, got {rendered}"
    );
}

#[test]
fn test_parse_each_player_cant_cast_more_than_one_spell_each_turn() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Arcane Laboratory Variant")
        .parse_text("Each player can't cast more than one spell each turn.")
        .expect("parse each-player one-spell cast limit");

    let rendered = oracle_like_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("each player can't cast more than one spell each turn")
            || rendered.contains("each player cant cast more than one spell each turn"),
        "expected each-player cast-limit text, got {rendered}"
    );
}

#[test]
fn test_parse_players_cant_cast_more_than_one_spell_each_turn() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Rule Of Law Variant")
        .parse_text("Players can't cast more than one spell each turn.")
        .expect("parse players one-spell cast limit");

    let rendered = oracle_like_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("players can't cast more than one spell each turn")
            || rendered.contains("players cant cast more than one spell each turn"),
        "expected players cast-limit text, got {rendered}"
    );
}

#[test]
fn test_parse_canonist_style_nonartifact_cast_limit() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Canonist Variant")
        .parse_text("Each player who has cast a nonartifact spell this turn can't cast additional nonartifact spells.")
        .expect("parse canonist-style nonartifact cast limit");

    let rendered = oracle_like_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains(
            "each player who has cast a nonartifact spell this turn can't cast additional nonartifact spells"
        )
            || rendered.contains(
                "each player who has cast a nonartifact spell this turn cant cast additional nonartifact spells"
            )
            || rendered.contains("each player can't cast more than one nonartifact spell each turn")
            || rendered.contains("each player cant cast more than one nonartifact spell each turn"),
        "expected canonist-style cast-limit normalization, got {rendered}"
    );
}

#[test]
fn test_parse_your_opponents_nonartifact_cast_limit() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Lavinia Variant")
        .parse_text("Your opponents can't cast more than one nonartifact spell each turn.")
        .expect("parse your-opponents nonartifact cast limit");

    let rendered = oracle_like_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("your opponents can't cast more than one nonartifact spell each turn")
            || rendered
                .contains("your opponents cant cast more than one nonartifact spell each turn"),
        "expected your-opponents nonartifact cast-limit text, got {rendered}"
    );
}

#[test]
fn test_parse_nonphyrexian_cast_limit() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Phyrexian Censor Variant")
        .parse_text("Each player can't cast more than one non-Phyrexian spell each turn.")
        .expect("parse non-phyrexian cast limit");

    let rendered = oracle_like_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("each player can't cast more than one non-phyrexian spell each turn")
            || rendered
                .contains("each player cant cast more than one non-phyrexian spell each turn"),
        "expected non-phyrexian cast-limit text, got {rendered}"
    );
}

#[test]
fn test_parse_uncounterable_from_text() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "No Counter")
        .parse_text("This spell can't be countered.")
        .expect("parse this spell can't be countered");

    let has_uncounterable = def.abilities.iter().any(|ability| {
        matches!(
            &ability.kind,
            AbilityKind::Static(ability) if ability.id() == StaticAbilityId::CantBeCountered
        )
    });

    assert!(has_uncounterable);
}

#[test]
fn test_parse_spells_cant_be_countered_as_rule_restriction() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Global No Counter")
        .parse_text("Spells can't be countered.")
        .expect("parse spells can't be countered");

    let has_rule_restriction = def.abilities.iter().any(|ability| {
        matches!(
            &ability.kind,
            AbilityKind::Static(ability) if ability.id() == StaticAbilityId::RuleRestriction
        )
    });

    assert!(has_rule_restriction);
}

#[test]
fn test_parse_cavern_of_souls_generic_mana_usage_restriction() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Cavern of Souls")
        .card_types(vec![CardType::Land])
        .parse_text(
            "As this land enters, choose a creature type.\n{T}: Add {C}.\n{T}: Add one mana of any color. Spend this mana only to cast a creature spell of the chosen type, and that spell can't be countered.",
        )
        .expect("cavern of souls style mana restriction should parse");

    let activated = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) if !activated.mana_usage_restrictions.is_empty() => {
                Some(activated)
            }
            _ => None,
        })
        .expect("expected colored mana ability with typed spend restriction");

    assert_eq!(
        activated.mana_usage_restrictions,
        vec![crate::ability::ManaUsageRestriction::CastSpell {
            card_types: vec![CardType::Creature],
            subtype_requirement: Some(
                crate::ability::ManaUsageSubtypeRequirement::ChosenTypeOfSource,
            ),
            grant_uncounterable: true,
        }]
    );
}

#[test]
fn test_parse_nonsource_cant_block_specific_attacker_restriction() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Cowardly Rule")
        .parse_text("Cowards can't block Warriors.")
        .expect("parse cowards can't block warriors");

    let has_rule_restriction = def.abilities.iter().any(|ability| {
        matches!(
            &ability.kind,
            AbilityKind::Static(ability) if ability.id() == StaticAbilityId::RuleRestriction
        )
    });

    assert!(has_rule_restriction);
}

#[test]
fn test_parse_bare_cant_be_blocked_by_more_than_one_creature() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Bare Unblockable Limit")
        .parse_text("Can't be blocked by more than one creature.")
        .expect("parse bare cant-be-blocked-by-more-than clause");

    let has_limit = def.abilities.iter().any(|ability| {
        matches!(
            &ability.kind,
            AbilityKind::Static(ability)
                if ability.id() == StaticAbilityId::CantBeBlockedByMoreThan
        )
    });

    assert!(has_limit);
}

#[test]
fn test_parse_enchanted_creature_cant_attack_or_block() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Arrest Test")
        .card_types(vec![CardType::Enchantment])
        .parse_text("Enchant creature\nEnchanted creature can't attack or block.")
        .expect("parse enchanted creature cant attack or block");

    let rendered = oracle_like_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("enchanted creature can't attack or block")
            || rendered.contains("enchanted creature cant attack or block"),
        "expected enchanted attack/block restriction text, got {rendered}"
    );
}

#[test]
fn test_parse_enchanted_creature_cant_activate_abilities() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Arrest Plus Test")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "Enchant creature\nEnchanted creature can't attack or block, and its activated abilities can't be activated.",
        )
        .expect("parse enchanted creature activated-abilities restriction");

    let rendered = oracle_like_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("enchanted creature can't attack or block")
            && (rendered.contains("its activated abilities can't be activated")
                || rendered.contains("enchanted creature activated abilities can't be activated")),
        "expected arrest-style restriction text, got {rendered}"
    );
}

#[test]
fn test_parse_deadlock_trap_its_activated_abilities_cant_be_activated_this_turn() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Deadlock Trap Test")
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "This artifact enters tapped.\n{T}, Pay {E}: Tap target creature or planeswalker. Its activated abilities can't be activated this turn.",
        )
        .expect("parse deadlock-trap style activated-abilities clause");

    let rendered = oracle_like_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("its activated abilities can't be activated this turn")
            || rendered.contains("activated abilities of permanent can't be activated this turn")
            || rendered.contains("its activated abilities cant be activated this turn"),
        "expected deadlock-trap restriction text, got {rendered}"
    );
}

#[test]
fn test_parse_activated_abilities_with_t_in_costs_cant_be_activated() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Serra Bestiary Test")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "Enchant creature\nEnchanted creature's activated abilities with {T} in their costs can't be activated.",
        )
        .expect("parse tap-cost activated-ability restriction");

    let rendered = oracle_like_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("activated abilities with {t} in their costs can't be activated")
            || rendered.contains(
                "enchanted creatures activated abilities with t in their costs can't be activated"
            )
            || rendered.contains("activated abilities with t in their costs cant be activated"),
        "expected tap-cost activated-ability restriction text, got {rendered}"
    );
}

#[test]
fn test_parse_enchanted_permanent_cant_attack_or_block() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Bound In Gold Test")
        .card_types(vec![CardType::Enchantment])
        .parse_text("Enchant permanent\nEnchanted permanent can't attack or block.")
        .expect("parse enchanted permanent cant attack or block");

    let rendered = oracle_like_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("enchanted permanent can't attack or block")
            || rendered.contains("enchanted permanent cant attack or block"),
        "expected attached cant attack or block text, got {rendered}"
    );
}

#[test]
fn test_parse_target_creature_you_dont_control_gets_minus_two_minus_two() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Downsize Test")
        .parse_text("Target creature you don't control gets -2/-2 until end of turn.")
        .expect("parse target creature you dont control gets -2/-2");

    let rendered = oracle_like_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("target creature you don't control gets -2/-2 until end of turn")
            || rendered.contains("target creature you dont control gets -2/-2 until end of turn"),
        "expected parsed pump effect, got {rendered}"
    );
}

#[test]
fn test_parse_destination_first_return_all_to_hand_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Return To Hand Test")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Return to your hand all creature cards in your graveyard that were put there from the battlefield this turn.",
        )
        .expect("parse destination-first return-to-hand clause");

    let rendered = oracle_like_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("in your graveyard") && rendered.contains("to your hand"),
        "expected destination-first return-to-hand text, got {rendered}"
    );
}

#[test]
fn test_parse_destination_first_return_all_to_battlefield_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Return To Battlefield Test")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Return to the battlefield all permanent cards in your graveyard that were put there from the battlefield this turn.",
        )
        .expect("parse destination-first return-to-battlefield clause");

    let rendered = oracle_like_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("in your graveyard") && rendered.contains("to the battlefield"),
        "expected destination-first return-to-battlefield text, got {rendered}"
    );
}

#[test]
fn test_parse_choose_color_as_enters_for_nonland_subjects() {
    let creature_def = CardDefinitionBuilder::new(CardId::from_raw(1), "Color Creature")
        .card_types(vec![CardType::Creature])
        .parse_text("As this creature enters, choose a color.")
        .expect("parse as this creature enters choose a color");
    let enchantment_def = CardDefinitionBuilder::new(CardId::from_raw(2), "Color Enchantment")
        .card_types(vec![CardType::Enchantment])
        .parse_text("As this enchantment enters, choose a color.")
        .expect("parse as this enchantment enters choose a color");

    let creature_has = creature_def.abilities.iter().any(|ability| {
        matches!(
            &ability.kind,
            AbilityKind::Static(ability) if ability.id() == StaticAbilityId::ChooseColorAsEnters
        )
    });
    let enchantment_has = enchantment_def.abilities.iter().any(|ability| {
        matches!(
            &ability.kind,
            AbilityKind::Static(ability) if ability.id() == StaticAbilityId::ChooseColorAsEnters
        )
    });

    assert!(creature_has);
    assert!(enchantment_has);
}

#[test]
fn test_parse_choose_basic_land_type_as_enters_for_aura() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Convincing Mirage Variant")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Aura])
        .parse_text("As this aura enters, choose a basic land type.")
        .expect("parse as this aura enters choose a basic land type");

    let ids: Vec<StaticAbilityId> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(ids.contains(&StaticAbilityId::ChooseBasicLandTypeAsEnters));
    assert!(
        !ids.contains(&StaticAbilityId::RuleTextPlaceholder),
        "expected typed basic-land-type-as-enters static ability, got {ids:?}"
    );
}

#[test]
fn test_parse_enchanted_land_is_chosen_type_without_placeholder() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Phantasmal Terrain Variant")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Aura])
        .parse_text(
            "Enchant land.\nAs this aura enters, choose a basic land type.\nEnchanted land is the chosen type.",
        )
        .expect("parse chosen basic land type Aura lines");

    let ids: Vec<StaticAbilityId> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(ids.contains(&StaticAbilityId::ChooseBasicLandTypeAsEnters));
    assert!(ids.contains(&StaticAbilityId::EnchantedLandIsChosenType));
    assert!(
        !ids.contains(&StaticAbilityId::RuleTextPlaceholder)
            && !ids.contains(&StaticAbilityId::UnsupportedParserLine),
        "expected typed chosen-type Aura static abilities, got {ids:?}"
    );
}

#[test]
fn test_aura_chosen_basic_land_type_sets_enchanted_land_subtype() {
    let aura_def = CardDefinitionBuilder::new(CardId::from_raw(1), "Convincing Mirage Variant")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Aura])
        .parse_text(
            "Enchant land.\nAs this aura enters, choose a basic land type.\nEnchanted land is the chosen type.",
        )
        .expect("parse chosen basic land type Aura");

    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);

    let land_card = crate::card::CardBuilder::new(CardId::from_raw(2), "Test Forest")
        .card_types(vec![CardType::Land])
        .subtypes(vec![Subtype::Forest])
        .build();
    let land_id = game.create_object_from_card(&land_card, alice, crate::zone::Zone::Battlefield);

    let aura_id_in_hand =
        game.create_object_from_definition(&aura_def, alice, crate::zone::Zone::Hand);
    let mut dm = crate::decision::SelectFirstDecisionMaker;
    let result = game
        .move_object_with_etb_processing_with_dm(
            aura_id_in_hand,
            crate::zone::Zone::Battlefield,
            &mut dm,
        )
        .expect("aura should enter and attach to the available land");
    let aura_id = result.new_id;

    assert_eq!(
        game.chosen_basic_land_type(aura_id),
        Some(Subtype::Plains),
        "select-first decision maker should choose Plains"
    );
    assert_eq!(
        game.object(aura_id).and_then(|obj| obj.attached_to),
        Some(land_id),
        "aura should attach to the only legal land"
    );

    let land_chars = game
        .calculated_characteristics(land_id)
        .expect("land should have calculated characteristics");
    assert!(
        land_chars.subtypes.contains(&Subtype::Plains),
        "enchanted land should become the chosen type"
    );
    assert!(
        !land_chars.subtypes.contains(&Subtype::Forest),
        "set-subtype effect should replace prior land subtypes"
    );
}

#[test]
fn test_parse_this_cost_is_reduced_by_basic_land_types_without_placeholder() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Draco Variant")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(8)],
            vec![ManaSymbol::Green],
        ]))
        .card_types(vec![CardType::Creature])
        .parse_text("This cost is reduced by {2} for each basic land type among lands you control.")
        .expect("parse this-cost domain reduction line");

    let mut has_typed_reduction = false;
    for ability in &def.abilities {
        let AbilityKind::Static(static_ability) = &ability.kind else {
            continue;
        };
        assert_ne!(
            static_ability.id(),
            StaticAbilityId::RuleTextPlaceholder,
            "expected typed reduction, got placeholder static ability"
        );
        if static_ability.this_spell_cost_reduction().is_some() {
            has_typed_reduction = true;
        }
    }
    assert!(
        has_typed_reduction,
        "expected parsed this-spell cost reduction ability"
    );

    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);

    let plains = crate::card::CardBuilder::new(CardId::from_raw(2), "Test Plains")
        .card_types(vec![CardType::Land])
        .subtypes(vec![Subtype::Plains])
        .build();
    game.create_object_from_card(&plains, alice, Zone::Battlefield);

    let island = crate::card::CardBuilder::new(CardId::from_raw(3), "Test Island")
        .card_types(vec![CardType::Land])
        .subtypes(vec![Subtype::Island])
        .build();
    game.create_object_from_card(&island, alice, Zone::Battlefield);

    let swamp = crate::card::CardBuilder::new(CardId::from_raw(4), "Test Swamp")
        .card_types(vec![CardType::Land])
        .subtypes(vec![Subtype::Swamp])
        .build();
    game.create_object_from_card(&swamp, alice, Zone::Battlefield);

    let spell_id = game.create_object_from_definition(&def, alice, Zone::Hand);
    let spell = game.object(spell_id).expect("spell exists");
    let base_cost = spell.mana_cost.as_ref().expect("spell has mana cost");
    let effective = crate::decision::calculate_effective_mana_cost(&game, alice, spell, base_cost);

    assert_eq!(
        effective.to_oracle(),
        "{2}{G}",
        "expected {{2}} reduction per distinct basic land type among lands you control"
    );
}

#[test]
fn test_parse_basic_land_type_count_conditionals_for_you_control_tail() {
    let exact = CardDefinitionBuilder::new(CardId::from_raw(1), "Exact Domain Condition")
        .card_types(vec![CardType::Instant])
        .parse_text("If there are five basic land types among lands you control, draw a card.")
        .expect("parse exact basic-land-types conditional");
    let exact_rendered = compiled_lines(&exact).join(" | ").to_ascii_lowercase();
    assert!(
        exact_rendered.contains("basic land type")
            && exact_rendered.contains("among lands you control"),
        "expected rendered exact conditional to mention basic land types among lands you control, got {exact_rendered}"
    );

    let at_least = CardDefinitionBuilder::new(CardId::from_raw(2), "Threshold Domain Condition")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "If there are three or more basic land types among lands you control, draw a card.",
        )
        .expect("parse threshold basic-land-types conditional");
    let threshold_rendered = compiled_lines(&at_least).join(" | ").to_ascii_lowercase();
    assert!(
        threshold_rendered.contains("basic land type")
            && threshold_rendered.contains("among lands you control"),
        "expected rendered threshold conditional to mention basic land types among lands you control, got {threshold_rendered}"
    );
}

#[test]
fn test_parse_damage_equal_to_thiss_power() {
    CardDefinitionBuilder::new(CardId::from_raw(1), "Power Reference")
        .parse_text("This deals damage equal to this's power to any target.")
        .expect("parse damage equal to this's power");
}

#[test]
fn test_parse_characteristic_power_equal_number_of_creatures() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Power Tester")
        .parse_text("Power Tester's power is equal to the number of creatures you control.")
        .expect("parse characteristic power-only count line");

    let static_ability = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability)
                if static_ability.id() == StaticAbilityId::CharacteristicDefiningPT =>
            {
                Some(static_ability)
            }
            _ => None,
        })
        .expect("expected characteristic-defining P/T ability");

    let game = crate::game_state::GameState::new(vec!["Alice".to_string()], 20);
    let effects = static_ability.generate_effects(
        crate::ids::ObjectId::from_raw(1),
        crate::ids::PlayerId::from_index(0),
        &game,
    );
    let crate::continuous::Modification::SetPowerToughness {
        power,
        toughness,
        sublayer: _,
    } = &effects[0].modification
    else {
        panic!("expected SetPowerToughness modification");
    };

    let crate::effect::Value::Count(filter) = power else {
        panic!("expected counted power value");
    };
    assert!(filter.card_types.contains(&CardType::Creature));
    assert_eq!(filter.controller, Some(PlayerFilter::You));
    assert!(matches!(toughness, crate::effect::Value::SourceToughness));
}

#[test]
fn test_parse_characteristic_power_equal_greatest_mana_value() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Dodgy Jalopy")
        .parse_text(
            "Dodgy Jalopy's power is equal to the greatest mana value among creatures you control.",
        )
        .expect("parse characteristic power-only aggregate line");

    let static_ability = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability)
                if static_ability.id() == StaticAbilityId::CharacteristicDefiningPT =>
            {
                Some(static_ability)
            }
            _ => None,
        })
        .expect("expected characteristic-defining P/T ability");

    let game = crate::game_state::GameState::new(vec!["Alice".to_string()], 20);
    let effects = static_ability.generate_effects(
        crate::ids::ObjectId::from_raw(1),
        crate::ids::PlayerId::from_index(0),
        &game,
    );
    let crate::continuous::Modification::SetPowerToughness {
        power,
        toughness,
        sublayer: _,
    } = &effects[0].modification
    else {
        panic!("expected SetPowerToughness modification");
    };

    let crate::effect::Value::GreatestManaValue(filter) = power else {
        panic!("expected greatest mana value power");
    };
    assert!(filter.card_types.contains(&CardType::Creature));
    assert_eq!(filter.controller, Some(PlayerFilter::You));
    assert!(matches!(toughness, crate::effect::Value::SourceToughness));
}

#[test]
fn test_parse_creatures_attack_this_turn_if_able_clause() {
    CardDefinitionBuilder::new(CardId::from_raw(1), "Instigate Combat")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Creatures your opponents control attack this turn if able.")
        .expect("parse creatures attack this turn if able");
}

#[test]
fn test_parse_this_creature_must_be_blocked_if_able_static_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Forced Blocker")
        .card_types(vec![CardType::Creature])
        .parse_text("This creature must be blocked if able.")
        .expect("parse this creature must be blocked if able");

    let has_rule_restriction = def.abilities.iter().any(|ability| {
        matches!(
            &ability.kind,
            AbilityKind::Static(ability) if ability.id() == StaticAbilityId::RuleRestriction
        )
    });
    assert!(has_rule_restriction);
}

#[test]
fn test_parse_double_cant_clause_from_text() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "No Win")
        .parse_text("You can't lose the game and your opponents can't win the game.")
        .expect("parse dual can't clause");

    let has_cant_lose = def.abilities.iter().any(|ability| {
        matches!(
            &ability.kind,
            AbilityKind::Static(ability) if ability.id() == StaticAbilityId::YouCantLoseGame
        )
    });
    let has_cant_win = def.abilities.iter().any(|ability| {
        matches!(
            &ability.kind,
            AbilityKind::Static(ability) if ability.id() == StaticAbilityId::OpponentsCantWinGame
        )
    });

    assert!(has_cant_lose);
    assert!(has_cant_win);
}

#[test]
fn test_parse_characteristic_pt_constant_plus_count() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Aysen Crusader")
        .parse_text(
            "Aysen Crusader's power and toughness are each equal to 2 plus the number of Soldiers and Warriors you control.",
        )
        .expect("parse characteristic P/T constant plus count");

    let static_ability = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability)
                if static_ability.id() == StaticAbilityId::CharacteristicDefiningPT =>
            {
                Some(static_ability)
            }
            _ => None,
        })
        .expect("expected characteristic-defining P/T ability");

    let game = crate::game_state::GameState::new(vec!["Alice".to_string()], 20);
    let effects = static_ability.generate_effects(
        crate::ids::ObjectId::from_raw(1),
        crate::ids::PlayerId::from_index(0),
        &game,
    );

    let crate::continuous::Modification::SetPowerToughness {
        power,
        toughness,
        sublayer: _,
    } = &effects[0].modification
    else {
        panic!("expected SetPowerToughness modification");
    };

    let crate::effect::Value::Add(left, right) = power else {
        panic!("expected additive power value");
    };
    assert!(matches!(&**left, crate::effect::Value::Fixed(2)));
    let crate::effect::Value::Count(filter) = &**right else {
        panic!("expected count term in additive power value");
    };
    assert!(filter.subtypes.contains(&Subtype::Soldier));
    assert!(filter.subtypes.contains(&Subtype::Warrior));
    assert_eq!(filter.controller, Some(PlayerFilter::You));
    assert_eq!(power, toughness);
}

#[test]
fn test_parse_characteristic_pt_count_plus_count() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Soulless One")
        .parse_text(
            "Soulless One's power and toughness are each equal to the number of Zombies on the battlefield plus the number of Zombie cards in all graveyards.",
        )
        .expect("parse characteristic P/T count plus count");

    let rendered = oracle_like_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("number of zombies on the battlefield"),
        "expected characteristic P/T zombie count wording, got {rendered}"
    );
}

#[test]
fn test_parse_keyword_action_trigger_you_earthbend() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Earthbend Watcher")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .parse_text("Whenever you earthbend, draw a card.")
        .expect("parse keyword action trigger");

    let triggered = def
        .abilities
        .iter()
        .find_map(|a| match &a.kind {
            AbilityKind::Triggered(t) => Some(t),
            _ => None,
        })
        .expect("expected triggered ability");

    assert_eq!(triggered.trigger.display(), "Whenever you earthbend");
}

#[test]
fn test_parse_keyword_action_trigger_any_player() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Investigation Watcher")
        .card_types(vec![CardType::Enchantment])
        .parse_text("Whenever a player investigates, draw a card.")
        .expect("parse keyword action trigger");

    let triggered = def
        .abilities
        .iter()
        .find_map(|a| match &a.kind {
            AbilityKind::Triggered(t) => Some(t),
            _ => None,
        })
        .expect("expected triggered ability");

    assert_eq!(
        triggered.trigger.display(),
        "Whenever a player investigates"
    );
}

#[test]
fn test_parse_keyword_action_trigger_players_finish_voting() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Vote Watcher")
        .card_types(vec![CardType::Enchantment])
        .parse_text("Whenever players finish voting, draw a card.")
        .expect("parse keyword action trigger");

    let triggered = def
        .abilities
        .iter()
        .find_map(|a| match &a.kind {
            AbilityKind::Triggered(t) => Some(t),
            _ => None,
        })
        .expect("expected triggered ability");

    assert_eq!(
        triggered.trigger.display(),
        "Whenever players finish voting"
    );
}

#[test]
fn test_parse_enters_tapped_filter_keeps_opponent_controller_constraint() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Frozen Aether Variant")
        .parse_text(
            "Artifacts, creatures, and lands your opponents control enter the battlefield tapped.",
        )
        .expect("should parse opponents-control enters tapped line");

    let rendered = compiled_lines(&def).join(" | ").to_ascii_lowercase();
    assert!(
        rendered.contains("opponent"),
        "expected rendered line to preserve opponents controller filter, got {rendered}"
    );
}

#[test]
fn test_parse_cohort_ability_word_prefix_keeps_cost_and_effect() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Ondu War Cleric")
        .card_types(vec![CardType::Creature])
        .parse_text("Cohort — {T}, Tap an untapped Ally you control: Target opponent loses 2 life.")
        .expect("parse cohort activated ability with label");

    let lines = compiled_lines(&def);
    let joined = lines.join(" ").to_ascii_lowercase();
    assert!(
        joined.contains("untapped") && joined.contains("ally"),
        "expected untapped ally tap cost in compiled text, got {joined}"
    );
    assert!(
        joined.contains("loses 2 life"),
        "expected opponent life-loss effect in compiled text, got {joined}"
    );
}

#[test]
fn test_parse_labeled_leading_condition_with_gets_and_has() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Auriok Sunchaser Variant")
            .parse_text(
                "Metalcraft — As long as you control three or more artifacts, this creature gets +2/+2 and has flying.",
            )
            .expect("parse labeled leading condition anthem+keyword");

    let displays: Vec<String> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.display()),
            _ => None,
        })
        .collect();
    assert!(
        displays
            .iter()
            .any(|display| display.contains("this creature gets +2/+2")
                && display.contains("as long as you control three or more artifacts")),
        "expected conditional self buff ability, got: {displays:?}"
    );
    assert!(
        displays.iter().any(|display| display.contains("has Flying")
            && display.contains("as long as you control three or more artifacts")),
        "expected conditional flying grant ability, got: {displays:?}"
    );
}

#[test]
fn test_parse_coven_condition_uses_different_power_predicate() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Coven Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Coven — At the beginning of combat on your turn, if you control three or more creatures with different powers, this creature gains trample until end of turn.",
        )
        .expect("parse coven condition with different powers");

    let debug = format!("{:#?}", def.abilities);
    assert!(
        debug.contains("PlayerControlsAtLeastWithDifferentPowers"),
        "expected coven predicate to require different powers, got {debug}"
    );
}

#[test]
fn test_parse_target_player_may_copy_this_spell_and_choose_new_targets() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Reverberate Variant")
            .card_types(vec![CardType::Sorcery])
            .parse_text(
                "Target player discards two cards. That player may copy this spell and may choose a new target for that copy.",
            )
            .expect("parse targeted copy-this-spell clause");

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("DiscardEffect"),
        "expected discard effect in spell text, got {debug}"
    );
    assert!(
        debug.contains("CopySpellEffect"),
        "expected copy-spell effect in spell text, got {debug}"
    );
    let lines = compiled_lines(&def);
    let joined = lines.join(" ").to_ascii_lowercase();
    assert!(
        joined.contains("target player may copy this spell")
            && !joined.contains("you may copy this spell"),
        "expected copy permission to stay linked to targeted player, got {joined}"
    );
    assert!(
        joined.contains("copy this spell"),
        "expected copy clause to remain in render output, got {joined}"
    );
}

#[test]
fn test_parse_then_controller_may_copy_spell_and_choose_new_targets() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Chain of Acid Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Destroy target noncreature permanent. Then that permanent's controller may copy this spell and may choose a new target for that copy.",
        )
        .expect("parse then-controller copy-this-spell clause");

    let joined = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains("that object's controller may copy this spell")
            && !joined.contains("you may copy this spell"),
        "expected copy permission to stay linked to referenced controller, got {joined}"
    );
    assert!(
        joined.contains("that object's controller may copy this spell"),
        "expected copy permission to stay linked to referenced controller, got {joined}"
    );
}

#[test]
fn test_parse_choose_new_targets_for_the_copy() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Reverberate Style Variant")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Copy target instant or sorcery spell. You may choose new targets for the copy.",
        )
        .expect("parse choose-new-targets for the copy");

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("CopySpellEffect"),
        "expected copy-spell effect, got {debug}"
    );
    assert!(
        debug.contains("RetargetStackObjectEffect"),
        "expected retarget effect for the copy, got {debug}"
    );
}

#[test]
fn test_parse_gain_keyword_ability_does_not_fall_back_to_gain_life() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Gain Keyword Variant")
        .card_types(vec![CardType::Instant])
        .parse_text("Put a +1/+1 counter on target creature you control and it gains deathtouch until end of turn.")
        .expect("parse gain-keyword line");

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("AddAbility(StaticAbility(Deathtouch))"),
        "expected ability grant effect, got {debug}"
    );
    assert!(
        !debug.contains("GainLifeEffect"),
        "did not expect life-gain fallback for keyword grant, got {debug}"
    );
}

#[test]
fn test_parse_lose_keyword_ability_does_not_fall_back_to_lose_life() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Lose Keyword Variant")
        .card_types(vec![CardType::Instant])
        .parse_text("Target creature loses flying until end of turn.")
        .expect("parse lose-keyword line");

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("RemoveAbility(StaticAbility(Flying))"),
        "expected ability removal effect, got {debug}"
    );
    assert!(
        !debug.contains("LoseLifeEffect"),
        "did not expect life-loss fallback for keyword removal, got {debug}"
    );
}

#[test]
fn test_parse_lose_keyword_ability_without_duration() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Lose Keyword No Duration Variant")
        .card_types(vec![CardType::Instant])
        .parse_text("Target creature loses flying.")
        .expect("parse lose-keyword line without explicit duration");

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("RemoveAbility(StaticAbility(Flying))"),
        "expected flying-removal effect, got {debug}"
    );
}

#[test]
fn test_parse_copy_this_spell_for_each_creature_sacrificed_this_way() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Plumb Variant")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "As an additional cost to cast this spell, you may sacrifice one or more creatures. When you do, copy this spell for each creature sacrificed this way.\nDraw a card.",
        )
        .expect_err("unsupported when-you-do copy clause should fail loudly");
    let rendered = format!("{err:?}").to_ascii_lowercase();
    assert!(
        rendered.contains("unsupported triggered line") || rendered.contains("when you do"),
        "expected explicit unsupported when-you-do rejection, got {rendered}"
    );
}

#[test]
fn test_parse_additional_cost_tap_two_untapped_creatures_and_or_lands() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Fear of Exposure")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "As an additional cost to cast this spell, tap two untapped creatures and/or lands you control.\nTrample",
        )
        .expect("parse tap-two additional cost");

    let additional_costs = def.additional_non_mana_costs();
    let tap = additional_costs
        .iter()
        .filter_map(|cost| cost.effect_ref())
        .find_map(|effect| effect.downcast_ref::<crate::effects::TapEffect>())
        .expect("expected tap cost effect");
    let (inner, count) = match &tap.spec {
        ChooseSpec::WithCount(inner, count) => (inner.as_ref(), count),
        other => panic!("expected counted tap spec, got {other:?}"),
    };
    assert_eq!(count.min, 2, "expected two taps, got {count:?}");
    assert_eq!(
        count.max,
        Some(2),
        "expected exactly two taps, got {count:?}"
    );
    let filter = match inner {
        ChooseSpec::Object(filter) => filter,
        other => panic!("expected object tap filter, got {other:?}"),
    };
    assert!(
        filter.untapped,
        "expected untapped requirement, got {filter:?}"
    );
    assert!(
        filter.card_types.contains(&CardType::Creature)
            && filter.card_types.contains(&CardType::Land),
        "expected creature/land tap filter, got {filter:?}"
    );
}

#[test]
fn test_parse_additional_cost_tap_four_untapped_artifacts_creatures_or_lands() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Guardian of the Great Door")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "As an additional cost to cast this spell, tap four untapped artifacts, creatures, and/or lands you control.\nFlying",
        )
        .expect("parse tap-four additional cost");

    let additional_costs = def.additional_non_mana_costs();
    let tap = additional_costs
        .iter()
        .filter_map(|cost| cost.effect_ref())
        .find_map(|effect| effect.downcast_ref::<crate::effects::TapEffect>())
        .expect("expected tap cost effect");
    let (inner, count) = match &tap.spec {
        ChooseSpec::WithCount(inner, count) => (inner.as_ref(), count),
        other => panic!("expected counted tap spec, got {other:?}"),
    };
    assert_eq!(count.min, 4, "expected four taps, got {count:?}");
    assert_eq!(
        count.max,
        Some(4),
        "expected exactly four taps, got {count:?}"
    );
    let filter = match inner {
        ChooseSpec::Object(filter) => filter,
        other => panic!("expected object tap filter, got {other:?}"),
    };
    assert!(
        filter.untapped,
        "expected untapped requirement, got {filter:?}"
    );
    assert!(
        filter.card_types.contains(&CardType::Artifact)
            && filter.card_types.contains(&CardType::Creature)
            && filter.card_types.contains(&CardType::Land),
        "expected artifact/creature/land tap filter, got {filter:?}"
    );
}

#[test]
fn test_parse_target_opponent_gains_control_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Witch Engine Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("{T}: Add {B}{B}{B}{B}. Target opponent gains control of this creature. (Activate only as an instant.)")
        .expect("parse target-opponent gain-control clause");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("ChangeControllerToPlayer(Target(Opponent))"),
        "expected gain-control runtime modification to resolve target opponent, got {debug}"
    );
    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("target opponent gains control of this"),
        "expected compiled text to preserve target opponent control change, got {rendered}"
    );
}

#[test]
fn test_parse_gain_control_each_noncommander_creature_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Subjugate the Hobbits Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Gain control of each noncommander creature with mana value 3 or less.")
        .expect("parse universal gain-control clause");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("gain control of each noncommander creature with mana value 3 or less"),
        "expected compiled text to preserve universal gain-control wording, got {rendered}"
    );
}

#[test]
fn test_parse_create_token_for_each_creature_that_died_this_turn() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Mahadi Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "At the beginning of your end step, create a Treasure token for each creature that died this turn.",
        )
        .expect("parse died-this-turn dynamic token count");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("CreaturesDiedThisTurn"),
        "expected dynamic died-this-turn count in triggered token creation, got {debug}"
    );
    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("for each creature that died this turn"),
        "expected compiled text to preserve died-this-turn token count, got {rendered}"
    );
}

#[test]
fn test_parse_trigger_attacks_with_subject_filter() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Attack Filter Probe")
        .card_types(vec![CardType::Enchantment])
        .parse_text("Whenever a creature you control attacks, put a +1/+1 counter on it.")
        .expect("parse filtered attacks trigger");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("AttacksTrigger"),
        "expected filtered attacks trigger matcher, got {debug}"
    );
    let joined = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains("whenever a creature")
            && !joined.contains("whenever this creature attacks"),
        "expected trigger subject to remain filtered, got {joined}"
    );
}

#[test]
fn test_parse_trigger_deals_combat_damage_with_subject_filter() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Combat Damage Filter Probe")
            .card_types(vec![CardType::Enchantment])
            .parse_text(
                "Whenever a Vampire you control deals combat damage to a player, put a +1/+1 counter on it.",
            )
            .expect("parse filtered combat-damage trigger");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("DealsCombatDamageToPlayerTrigger"),
        "expected filtered combat-damage trigger matcher, got {debug}"
    );
    let joined = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        (joined.contains("whenever a vampire you control deals combat damage to a player")
            || joined.contains(
                "whenever a vampire creature you control deals combat damage to a player"
            ))
            && !joined.contains("whenever this creature deals combat damage to a player"),
        "expected trigger subject to remain filtered, got {joined}"
    );
}

#[test]
fn test_parse_trigger_deals_combat_damage_to_you_preserves_recipient() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Combat Damage Recipient Probe")
        .card_types(vec![CardType::Enchantment])
        .parse_text("Whenever a creature deals combat damage to you, you gain 1 life.")
        .expect("parse combat-damage recipient trigger");

    let joined = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains("whenever creature deals combat damage to you")
            || joined.contains("whenever a creature deals combat damage to you"),
        "expected trigger recipient to remain 'you', got {joined}"
    );
}

#[test]
fn test_parse_trigger_this_blocks_filtered_creature() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Blocker Filter Probe")
            .card_types(vec![CardType::Creature])
            .parse_text(
                "Whenever this creature blocks a creature with flying, this creature gets +2/+0 until end of turn.",
            )
            .expect("parse filtered blocks trigger");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("ThisBlocksObjectTrigger"),
        "expected dedicated blocked-object trigger matcher, got {debug}"
    );
    let joined = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains("whenever this creature blocks creature with flying"),
        "expected trigger to include blocked-object filter, got {joined}"
    );
}

#[test]
fn test_parse_trigger_you_discard_filtered_card() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Discard Trigger Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Whenever you discard a noncreature, nonland card, this creature fights up to one target creature you don't control.",
        )
        .expect("parse filtered discard trigger");

    let debug = format!("{:?}", def.abilities);
    assert!(
        !debug.contains("unimplemented_trigger"),
        "discard trigger should not fall back to custom trigger, got {debug}"
    );
    assert!(
        debug.contains("YouDiscardCardTrigger"),
        "expected dedicated discard trigger matcher, got {debug}"
    );
    assert!(
        debug.contains("excluded_card_types: [")
            && debug.contains("Creature")
            && debug.contains("Land"),
        "expected noncreature/nonland discard filter, got {debug}"
    );
}

#[test]
fn test_parse_trigger_opponent_discards_card() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Opponent Discard Trigger Probe")
        .card_types(vec![CardType::Enchantment])
        .parse_text("Whenever an opponent discards a card, that player loses 2 life.")
        .expect("parse opponent discard trigger");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("YouDiscardCardTrigger") && debug.contains("player: Opponent"),
        "expected opponent discard trigger matcher, got {debug}"
    );
    let joined = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains("whenever an opponent discards a card"),
        "expected discard trigger wording in compiled text, got {joined}"
    );
}

#[test]
fn test_parse_trigger_opponent_plays_land() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Burgeoning Probe")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "Whenever an opponent plays a land, you may put a land card from your hand onto the battlefield.",
        )
        .expect("parse opponent land-play trigger");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("PlayerPlaysLandTrigger") && debug.contains("player: Opponent"),
        "expected dedicated land-play trigger matcher, got {debug}"
    );
    let joined = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains("whenever an opponent plays a land"),
        "expected land-play trigger wording in compiled text, got {joined}"
    );
    assert!(
        joined.contains("you may put a land card from your hand onto the battlefield"),
        "expected optional land deployment effect, got {joined}"
    );
}

#[test]
fn test_parse_trigger_tap_swamp_for_mana() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Tap Swamp Trigger Probe")
        .card_types(vec![CardType::Creature])
        .parse_text("Whenever you tap a Swamp for mana, add an additional {B}.")
        .expect("parse tap-for-mana swamp trigger");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("TapForManaTrigger"),
        "expected tap-for-mana trigger matcher, got {debug}"
    );
    assert!(
        !debug.contains("unimplemented_trigger"),
        "tap-for-mana trigger should not fall back to custom trigger, got {debug}"
    );
}

#[test]
fn test_parse_trigger_tap_creature_for_mana() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Tap Creature Trigger Probe")
        .card_types(vec![CardType::Creature])
        .parse_text("Whenever you tap a creature for mana, add an additional {G}.")
        .expect("parse tap-for-mana creature trigger");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("TapForManaTrigger")
            && debug.contains("card_types: [")
            && debug.contains("Creature"),
        "expected creature-filtered tap-for-mana trigger, got {debug}"
    );
}

#[test]
fn test_parse_trigger_one_or_more_plus_one_counters_put_on_this_creature() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Counter Trigger One-Or-More Probe")
        .card_types(vec![CardType::Creature])
        .parse_text("Whenever one or more +1/+1 counters are put on this creature, draw a card.")
        .expect("parse one-or-more counter placement trigger");

    let debug = format!("{:?}", def.abilities);
    assert!(
        !debug.contains("unimplemented_trigger"),
        "counter placement trigger should not fall back to custom trigger, got {debug}"
    );
    assert!(
        debug.contains("CounterPutOnTrigger") && debug.contains("count_mode: OneOrMore"),
        "expected typed one-or-more counter placement trigger, got {debug}"
    );
}

#[test]
fn test_parse_trigger_a_plus_one_counter_put_on_this_creature() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Counter Trigger Each Probe")
        .card_types(vec![CardType::Creature])
        .parse_text("Whenever a +1/+1 counter is put on this creature, draw a card.")
        .expect("parse per-counter placement trigger");

    let debug = format!("{:?}", def.abilities);
    assert!(
        !debug.contains("unimplemented_trigger"),
        "counter placement trigger should not fall back to custom trigger, got {debug}"
    );
    assert!(
        debug.contains("CounterPutOnTrigger") && debug.contains("count_mode: Each"),
        "expected typed per-counter placement trigger, got {debug}"
    );
}

#[test]
fn test_parse_trigger_you_put_one_or_more_minus_one_counters_on_a_creature() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Counter Trigger You Put Probe")
        .card_types(vec![CardType::Enchantment])
        .parse_text("Whenever you put one or more -1/-1 counters on a creature, draw a card.")
        .expect("parse active-voice one-or-more counter placement trigger");

    let debug = format!("{:?}", def.abilities);
    assert!(
        !debug.contains("unimplemented_trigger"),
        "counter placement trigger should not fall back to custom trigger, got {debug}"
    );
    assert!(
        debug.contains("CounterPutOnTrigger")
            && debug.contains("count_mode: OneOrMore")
            && debug.contains("counter_type: Some(MinusOneMinusOne)")
            && debug.contains("source_controller: Some(You)"),
        "expected typed active-voice counter placement trigger, got {debug}"
    );
}

#[test]
fn test_parse_nest_of_scarabs_style_trigger_and_token_amount() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Nest of Scarabs Variant")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "Whenever you put one or more -1/-1 counters on a creature, create that many 1/1 black Insect creature tokens.",
        )
        .expect("parse Nest of Scarabs style trigger");

    let debug = format!("{:#?}", def.abilities);
    assert!(
        debug.contains("CounterPutOnTrigger")
            && debug.contains("source_controller: Some(")
            && debug.contains("MinusOneMinusOne"),
        "expected typed counter trigger, got {debug}"
    );
    assert!(
        debug.contains("CreateTokenEffect")
            && debug.contains("EventValue")
            && debug.contains("Amount")
            && debug.contains("Insect"),
        "expected token creation to use trigger event amount, got {debug}"
    );
}

#[test]
fn test_parse_trigger_unknown_non_source_subject_fails() {
    let err = CardDefinitionBuilder::new(CardId::from_raw(1), "Unknown Subject Probe")
        .card_types(vec![CardType::Enchantment])
        .parse_text("Whenever a creature that attacks, draw a card.")
        .expect_err("unknown non-source subject should fail");
    let message = format!("{err:?}");
    assert!(
        message.contains("unsupported trigger subject filter")
            || message.contains("unsupported trigger clause")
            || message.contains("unsupported triggered line"),
        "expected strict trigger-subject parse failure, got {message}"
    );
}

#[test]
fn test_parse_player_subject_attack_trigger_uses_one_or_more_creature_filter() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Player Attack Subject Probe")
        .card_types(vec![CardType::Enchantment])
        .parse_text("Whenever you attack, draw a card.")
        .expect("player-subject attack trigger should parse");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("AttacksTrigger")
            && debug.contains("one_or_more: true")
            && debug.contains("controller: Some(You)"),
        "expected one-or-more attacks trigger for creatures you control, got {debug}"
    );
}

#[test]
fn test_parse_player_subject_attack_with_three_or_more_uses_thresholded_mode() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Three Or More Attack Subject Probe")
        .card_types(vec![CardType::Enchantment])
        .parse_text("Whenever you attack with three or more creatures, draw a card.")
        .expect("player-subject attack-with threshold trigger should parse");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("AttacksTrigger")
            && debug.contains("one_or_more: true")
            && debug.contains("min_total_attackers: 3")
            && debug.contains("controller: Some(You)"),
        "expected thresholded one-or-more attacks trigger for creatures you control, got {debug}"
    );
}

#[test]
fn test_parse_opponent_attacks_you_trigger_uses_one_or_more_mode() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Opponent Attacks You Probe")
        .card_types(vec![CardType::Enchantment])
        .parse_text("Whenever an opponent attacks you or a planeswalker you control, draw a card.")
        .expect("opponent-attacks-you trigger should parse");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("AttacksYouTrigger")
            && debug.contains("one_or_more: true")
            && debug.contains("controller: Some(Opponent)"),
        "expected one-or-more attacks-you trigger for opponent-controlled creatures, got {debug}"
    );
}

#[test]
fn test_parse_attack_life_loss_uses_iterated_defending_player_attack_filter() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Within Range Probe")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "Whenever you attack, each opponent loses life equal to the number of creatures attacking them.",
        )
        .expect("attack life-loss trigger should parse");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("AttacksTrigger") && debug.contains("one_or_more: true"),
        "expected one-or-more attacks trigger, got {debug}"
    );
    assert!(
        debug.contains("attacking_player_or_planeswalker_controlled_by: Some(IteratedPlayer)"),
        "expected count filter to bind to iterated defending player, got {debug}"
    );
}

#[test]
fn test_parse_target_creature_you_control_fights_target_creature_you_dont_control() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Prey Upon Probe")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Target creature you control fights target creature you don't control.")
        .expect("parse fight clause");

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("FightEffect"),
        "expected fight effect in spell text, got {debug}"
    );
    let joined = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains("fights"),
        "expected compiled fight text, got {joined}"
    );
}

#[test]
fn test_parse_target_creature_deals_damage_to_itself_equal_to_its_power() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Justice Strike Variant")
        .card_types(vec![CardType::Instant])
        .parse_text("Target creature deals damage to itself equal to its power.")
        .expect("parse one-sided self damage equal-power clause");

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("PowerOf"),
        "expected power-based dynamic damage amount, got {debug}"
    );
    let joined = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains("deal") && joined.contains("power"),
        "expected compiled output to keep one-sided power damage semantics, got {joined}"
    );
}

#[test]
fn test_parse_target_creature_you_control_deals_damage_equal_to_its_power_to_target_creature() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Bite Down Variant")
            .card_types(vec![CardType::Instant])
            .parse_text(
                "Target creature you control deals damage equal to its power to target creature you don't control.",
            )
            .expect("parse one-sided bite-style power damage clause");

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("PowerOf"),
        "expected power-based dynamic damage amount, got {debug}"
    );
    let joined = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains("deal") && joined.contains("target"),
        "expected one-sided targeted power damage rendering, got {joined}"
    );
    assert!(
        !joined.contains("fights"),
        "bite-style damage must not compile as fight, got {joined}"
    );
}

#[test]
fn test_parse_double_target_creatures_power_until_end_of_turn() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Mr. Orfeo Probe")
        .card_types(vec![CardType::Creature])
        .parse_text("Whenever you attack, double target creature's power until end of turn.")
        .expect("parse double-target-power trigger clause");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("PowerOf"),
        "expected dynamic power-based pump amount, got {debug}"
    );

    let joined = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains("gets")
            && (joined.contains("its power") || joined.contains("target creature's power"))
            && joined.contains("until end of turn"),
        "expected compiled output to preserve double-power semantics, got {joined}"
    );
}

#[test]
fn test_parse_reinforce_keyword_line_from_hand() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Reinforce Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Flying\nReinforce 2—{2}{W} ({2}{W}, Discard this card: Put two +1/+1 counters on target creature.)",
        )
        .expect("reinforce line should parse as a hand activated ability");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("{2}{w}")
            && rendered.contains("discard this card")
            && rendered.contains("put two +1/+1 counters on target creature"),
        "expected reinforce activation to include mana+discard cost and counter effect, got {rendered}"
    );

    let debug = format!("{:?}", def.abilities).to_ascii_lowercase();
    assert!(
        debug.contains("functional_zones: [hand]")
            && debug.contains("plusoneplusone")
            && debug.contains("discardeffect")
            && debug.contains("source: true"),
        "expected reinforce to be a hand ability with a source-discard cost and counter effect, got {debug}"
    );
}

#[test]
fn test_do_not_replace_keyword_named_card_reference_in_enchanted_grant_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Vigilance")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Aura])
        .parse_text("Enchant creature\nEnchanted creature has vigilance. (Attacking doesn't cause it to tap.)")
        .expect("keyword-named aura grant line should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("enchanted creature has vigilance"),
        "expected aura grant to keep vigilance keyword, got {rendered}"
    );
}

#[test]
fn test_parse_source_deals_damage_to_target_equal_to_number_of_filter() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Ben-Ben Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "{T}: This creature deals damage to target attacking creature equal to the number of untapped Mountains you control.",
        )
        .expect("parse damage-to-target equal-to-count clause");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("DealDamageEffect"),
        "expected damage effect in activated ability, got {debug}"
    );
    let lower = debug.to_ascii_lowercase();
    assert!(
        lower.contains("count(")
            && lower.contains("untapped: true")
            && lower.contains("subtypes: [mountain]"),
        "expected dynamic count amount using untapped Mountains, got {debug}"
    );
}

#[test]
fn test_parse_put_counter_then_it_deals_damage_equal_to_its_power() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Knockout Maneuver Variant")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Put a +1/+1 counter on target creature you control, then it deals damage equal to its power to target creature an opponent controls.",
        )
        .expect("parse put-counter then deal-damage-equal-to-power clause");

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("PutCountersEffect"),
        "expected +1/+1 counter effect before damage clause, got {debug}"
    );
    assert!(
        debug.contains("DealDamageEffect"),
        "expected damage effect after counter clause, got {debug}"
    );

    let joined = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains("put a +1/+1 counter on target creature you control"),
        "expected counter clause in compiled text, got {joined}"
    );
    assert!(
        joined.contains("deals damage equal to its power to target creature an opponent controls"),
        "expected follow-up damage clause in compiled text, got {joined}"
    );
}

#[test]
fn test_parse_exile_named_source_with_time_counters() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Suspend Setup Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Exile Suspend Setup Variant with three time counters on it.")
        .expect("parse named-source exile with time counters");

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("MoveToZoneEffect"),
        "expected exile move-to-zone effect, got {debug}"
    );
    assert!(
        debug.contains("target: Source"),
        "expected source-targeted exile/counter effects, got {debug}"
    );
    assert!(
        debug.contains("counter_type: Time"),
        "expected time counter placement, got {debug}"
    );
}

#[test]
fn test_parse_keyword_marker_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Marker Keywords")
        .card_types(vec![CardType::Creature])
        .parse_text("Unleash\nPhasing")
        .expect("marker keyword line should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("unleash")
            || (rendered.contains("+1/+1 counter") && rendered.contains("can't block")),
        "expected unleash semantics in compiled output, got {rendered}"
    );
    assert!(
        rendered.contains("phasing"),
        "expected phasing keyword text in compiled output, got {rendered}"
    );
}

#[test]
fn test_parse_marker_keyword_with_parameter_keeps_parameter_in_render() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Fabricate Probe")
        .card_types(vec![CardType::Creature])
        .parse_text("Fabricate 1")
        .expect("fabricate keyword line should parse");

    let rendered = oracle_like_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("fabricate 1")
            || (rendered.contains("choose one") && rendered.contains("servo")),
        "expected fabricate parameter in render output, got {rendered}"
    );
}

#[test]
fn test_parse_marker_keyword_with_cost_keeps_cost_in_render() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Dash Probe")
        .card_types(vec![CardType::Creature])
        .parse_text("Dash {2}{B}")
        .expect("dash keyword line should parse");

    let rendered = oracle_like_lines(&def).join(" ");
    assert!(
        rendered.contains("Dash {2}{B}"),
        "expected dash cost in render output, got {rendered}"
    );
    assert_eq!(def.alternative_casts.len(), 1);
    match &def.alternative_casts[0] {
        AlternativeCastingMethod::Dash { cost } => {
            assert_eq!(cost.to_oracle(), "{2}{B}");
        }
        other => panic!("expected dash alternative cast, got {other:?}"),
    }
    let debug = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        !debug.contains("unsupported"),
        "dash parse should avoid unsupported placeholders, got {debug}"
    );
}

#[test]
fn test_parse_companion_marker_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Companion Probe")
        .card_types(vec![CardType::Creature])
        .parse_text("Companion each nonland card in your starting deck has a different name")
        .expect("companion marker line should parse");

    let rendered = oracle_like_lines(&def).join(" ");
    assert!(
        rendered.contains("Companion"),
        "expected companion marker in render output, got {rendered}"
    );
}

#[test]
fn test_parse_hideaway_marker_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Hideaway Probe")
        .card_types(vec![CardType::Land])
        .parse_text("Hideaway 4")
        .expect("hideaway marker line should parse");

    let rendered = oracle_like_lines(&def).join(" ");
    assert!(
        rendered.contains("Hideaway 4"),
        "expected hideaway marker in render output, got {rendered}"
    );
}

#[test]
fn test_parse_implicit_become_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Implicit Become Probe")
        .card_types(vec![CardType::Creature])
        .parse_text("It's an enchantment.")
        .expect("implicit become clause should parse");

    let debug = format!("{def:#?}");
    assert!(
        debug.contains("AddCardTypes"),
        "expected add-card-types effect for implicit become clause, got {debug}"
    );
}

#[test]
fn test_parse_split_second_keyword_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Split Second Probe")
        .card_types(vec![CardType::Instant])
        .parse_text("Split second\nDraw a card.")
        .expect("split second line should parse");

    let rendered = oracle_like_lines(&def).join(" ");
    assert!(
        rendered.contains("Split second"),
        "expected split second marker in render output, got {rendered}"
    );
}

#[test]
fn test_parse_cascade_keyword_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Cascade Probe")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Cascade\nDraw a card.")
        .expect("cascade line should parse");

    let rendered = oracle_like_lines(&def).join(" ");
    assert!(
        rendered.contains("Cascade"),
        "expected cascade keyword in render output, got {rendered}"
    );
    let debug = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        debug.contains("staticability(") && debug.contains("cascade"),
        "expected cascade static ability id, got {debug}"
    );
    assert!(
        !debug.contains("staticabilityid::keywordmarker")
            && !debug.contains("staticabilityid::ruletextplaceholder"),
        "expected cascade to compile without placeholder static abilities, got {debug}"
    );
}

#[test]
fn test_parse_riot_keyword_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Riot Probe")
        .card_types(vec![CardType::Creature])
        .parse_text("Riot")
        .expect("riot line should parse");

    let abilities_debug = format!("{:?}", def.abilities);
    assert!(
        abilities_debug.contains("ChooseModeEffect"),
        "expected riot to compile into a modal ETB trigger, got {abilities_debug}"
    );
    assert!(
        !abilities_debug.contains("StaticAbilityId::KeywordMarker")
            && !abilities_debug.contains("StaticAbilityId::RuleTextPlaceholder"),
        "riot should not remain a placeholder marker ability, got {abilities_debug}"
    );
}

#[test]
fn test_parse_unleash_keyword_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Unleash Probe")
        .card_types(vec![CardType::Creature])
        .parse_text("Unleash")
        .expect("unleash line should parse");

    let abilities_debug = format!("{:?}", def.abilities);
    assert!(
        abilities_debug.contains("ZoneChangeTrigger")
            || abilities_debug.contains("ThisEntersBattlefield"),
        "expected unleash ETB trigger, got {abilities_debug}"
    );
    assert!(
        abilities_debug.contains("Unleash"),
        "expected unleash restriction ability, got {abilities_debug}"
    );
    assert!(
        !abilities_debug.contains("StaticAbilityId::KeywordMarker")
            && !abilities_debug.contains("StaticAbilityId::RuleTextPlaceholder"),
        "unleash should not remain a placeholder marker ability, got {abilities_debug}"
    );
}

#[test]
fn test_parse_zhur_taa_goblin_riot_keyword_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Zhur-Taa Goblin")
        .card_types(vec![CardType::Creature])
        .parse_text("Riot")
        .expect("zhur-taa goblin riot line should parse");

    let abilities_debug = format!("{:?}", def.abilities);
    assert!(
        abilities_debug.contains("ChooseModeEffect"),
        "expected riot to compile into a modal ETB choice, got {abilities_debug}"
    );
    assert!(
        !abilities_debug.contains("StaticAbilityId::KeywordMarker")
            && !abilities_debug.contains("StaticAbilityId::RuleTextPlaceholder"),
        "zhur-taa goblin riot should not remain a placeholder marker ability, got {abilities_debug}"
    );
}

#[test]
fn test_parse_training_keyword_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Training Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Training (Whenever this creature attacks with another creature with greater power, put a +1/+1 counter on this creature.)",
        )
        .expect("training line should parse as typed trigger");

    let rendered = oracle_like_lines(&def).join(" ");
    assert!(
        rendered.contains(
            "Whenever this creature attacks with another creature with greater power, put a +1/+1 counter on this creature"
        ),
        "expected canonical training trigger text in render output, got {rendered}"
    );
    assert!(
        !rendered.contains("EmitKeywordActionEffect"),
        "training render should hide runtime keyword-action instrumentation, got {rendered}"
    );
}

#[test]
fn test_parse_vanishing_keyword_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Vanishing Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Vanishing 3 (This creature enters with three time counters on it. At the beginning of your upkeep, remove a time counter from it. When the last is removed, sacrifice it.)",
        )
        .expect("vanishing line should parse as explicit mechanic");

    let rendered = oracle_like_lines(&def).join(" ");
    let debug = format!("{def:#?}");
    assert!(
        rendered.contains("Vanishing 3"),
        "expected vanishing render output, got {rendered}"
    );
    assert!(
        !debug.contains("KeywordMarker"),
        "expected vanishing to avoid keyword markers, got {debug}"
    );
}

#[test]
fn oracle_like_enchant_keyword_grant_does_not_duplicate_keyword_tail() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Aura Keyword Probe")
        .card_types(vec![CardType::Enchantment])
        .parse_text("Enchant creature\nEnchanted creature gets +1/+1 and has flying.")
        .expect("aura keyword grant line should parse");

    let rendered = oracle_like_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        !rendered.contains("and flying. and flying"),
        "expected duplicate keyword tail to be collapsed, got {rendered}"
    );
}

#[test]
fn oracle_like_cycling_uses_braced_mana_symbols() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Cycling Probe")
        .card_types(vec![CardType::Creature])
        .parse_text("Cycling {2}")
        .expect("cycling line should parse");

    let rendered = oracle_like_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("cycling {2}"),
        "expected braced cycling mana cost in render output, got {rendered}"
    );
}

#[test]
fn test_parse_unearth_keyword_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Unearth Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "{T}: You may tap or untap another target permanent.\n\
Unearth {U} ({U}: Return this card from your graveyard to the battlefield. It gains haste. Exile it at the beginning of the next end step or if it would leave the battlefield. Unearth only as a sorcery.)",
        )
        .expect("unearth keyword line should parse");

    let rendered = oracle_like_lines(&def).join(" ");
    assert!(
        rendered.contains("Unearth {U}")
            || rendered.contains("UnearthEffect")
            || rendered.contains("Unearth"),
        "expected unearth keyword in render output, got {rendered}"
    );
    let debug = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        !debug.contains("staticabilityid::custom") && !debug.contains("keyword_marker"),
        "expected unearth to compile without placeholder marker static abilities, got {debug}"
    );
}

#[test]
fn test_parse_echo_keyword_line_with_mana_cost() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Echo Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Echo {2}{R} (At the beginning of your upkeep, if this came under your control since the beginning of your last upkeep, sacrifice it unless you pay its echo cost.)",
        )
        .expect("echo keyword line should parse");

    let rendered = oracle_like_lines(&def).join(" ");
    assert!(
        rendered.to_ascii_lowercase().contains("echo counter"),
        "expected echo runtime text in render output, got {rendered}"
    );

    let debug = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        !debug.contains("staticabilityid::custom") && !debug.contains("keyword_marker"),
        "expected echo to compile without placeholder marker static abilities, got {debug}"
    );
    assert!(
        debug.contains("counter_type: echo"),
        "expected echo to track an internal echo counter, got {debug}"
    );
    assert!(
        debug.contains("paymanaeffect"),
        "expected echo mana variant to include a mana payment effect, got {debug}"
    );
    assert!(
        debug.contains("withideffect"),
        "expected echo trigger to track counter removal outcome with WithIdEffect, got {debug}"
    );
}

#[test]
fn test_parse_echo_keyword_line_with_non_mana_cost() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Echo Discard Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Flying, haste\nEcho—Discard a card. (At the beginning of your upkeep, if this came under your control since the beginning of your last upkeep, sacrifice it unless you pay its echo cost.)",
        )
        .expect("echo keyword line with non-mana cost should parse");

    let rendered = oracle_like_lines(&def).join(" ");
    assert!(
        rendered.to_ascii_lowercase().contains("discard a card"),
        "expected non-mana echo payment text in render output, got {rendered}"
    );

    let debug = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        !debug.contains("staticabilityid::custom") && !debug.contains("keyword_marker"),
        "expected echo to compile without placeholder marker static abilities, got {debug}"
    );
    assert!(
        debug.contains("counter_type: echo"),
        "expected echo to track an internal echo counter, got {debug}"
    );
    assert!(
        debug.contains("unlessactioneffect"),
        "expected echo non-mana variant to use unless-action payment flow, got {debug}"
    );
    assert!(
        debug.contains("withideffect"),
        "expected echo trigger to track counter removal outcome with WithIdEffect, got {debug}"
    );
}

#[test]
fn test_parse_escape_keyword_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Escape Probe")
        .card_types(vec![CardType::Creature])
        .parse_text("Escape—{3}{B}{B}, exile four other cards from your graveyard.")
        .expect("escape keyword line should parse");

    assert_eq!(def.alternative_casts.len(), 1);
    match &def.alternative_casts[0] {
        AlternativeCastingMethod::Escape { cost, exile_count } => {
            assert_eq!(*exile_count, 4);
            let cost = cost
                .as_ref()
                .expect("escape should carry explicit mana cost");
            assert_eq!(cost.to_oracle(), "{3}{B}{B}");
        }
        other => panic!("expected escape alternative cast, got {other:?}"),
    }
}

#[test]
fn test_parse_flashback_keyword_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Flashback Probe")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Flashback {1}{U}")
        .expect("flashback keyword line should parse");

    assert_eq!(def.alternative_casts.len(), 1);
    match &def.alternative_casts[0] {
        AlternativeCastingMethod::Flashback { total_cost } => {
            let cost = total_cost
                .mana_cost()
                .expect("flashback should include mana cost");
            assert_eq!(cost.to_oracle(), "{1}{U}");
            let costs = def.alternative_casts[0].non_mana_costs();
            assert!(
                costs.is_empty(),
                "expected flashback test probe to have no extra non-mana costs, got {costs:?}"
            );
        }
        other => panic!("expected flashback alternative cast, got {other:?}"),
    }
}

#[test]
fn test_parse_bestow_keyword_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Bestow Probe")
        .card_types(vec![CardType::Enchantment, CardType::Creature])
        .subtypes(vec![Subtype::Insect])
        .power_toughness(PowerToughness::fixed(2, 2))
        .parse_text("Bestow {3}{W}\nLifelink\nEnchanted creature gets +1/+1 and has lifelink.")
        .expect("bestow keyword line should parse");

    assert_eq!(def.alternative_casts.len(), 1);
    match &def.alternative_casts[0] {
        AlternativeCastingMethod::Bestow { total_cost } => {
            let cost = total_cost
                .mana_cost()
                .expect("bestow should include mana cost");
            assert_eq!(cost.to_oracle(), "{3}{W}");
            let costs = def.alternative_casts[0].non_mana_costs();
            assert!(
                costs.is_empty(),
                "expected mana-only bestow cost for probe, got {costs:?}"
            );
        }
        other => panic!("expected bestow alternative cast, got {other:?}"),
    }

    let static_ids = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(
        !static_ids.contains(&StaticAbilityId::KeywordMarker)
            && !static_ids.contains(&StaticAbilityId::RuleTextPlaceholder)
            && !static_ids.contains(&StaticAbilityId::UnsupportedParserLine),
        "bestow line should compile without placeholder static abilities, got {static_ids:?}"
    );

    let rendered = compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Bestow {3}{W}"),
        "expected compiled text to include bestow line, got {rendered}"
    );
}

#[test]
fn test_parse_bestow_keyword_line_with_extra_cost_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Bestow Extra Cost Probe")
        .card_types(vec![CardType::Enchantment, CardType::Creature])
        .subtypes(vec![Subtype::Insect])
        .power_toughness(PowerToughness::fixed(2, 2))
        .parse_text(
            "Bestow—{R}, Collect evidence 6.\nFlying\nEnchanted creature gets +2/+2 and has flying.",
        )
        .expect("bestow line with extra clause should parse");

    assert_eq!(def.alternative_casts.len(), 1);
    match &def.alternative_casts[0] {
        AlternativeCastingMethod::Bestow { total_cost } => {
            let cost = total_cost
                .mana_cost()
                .expect("bestow should include mana cost");
            assert_eq!(cost.to_oracle(), "{R}");
        }
        other => panic!("expected bestow alternative cast, got {other:?}"),
    }

    let debug = format!("{def:?}");
    assert!(
        !debug.contains("KeywordMarker")
            && !debug.contains("RuleTextPlaceholder")
            && !debug.contains("UnsupportedParserLine"),
        "bestow extra-cost line should avoid placeholder fallback, got {debug}"
    );
}

#[test]
fn test_parse_buyback_keyword_line_compiles_to_optional_cost() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Buyback Probe")
        .card_types(vec![CardType::Instant])
        .parse_text("Buyback {3}\nDraw a card.")
        .expect("buyback keyword line should parse");

    assert_eq!(def.optional_costs.len(), 1);
    let buyback = &def.optional_costs[0];
    assert_eq!(buyback.label, "Buyback");
    assert!(buyback.returns_to_hand);
    let mana = buyback
        .cost
        .mana_cost()
        .expect("buyback should preserve mana cost");
    assert_eq!(mana.to_oracle(), "{3}");
}

#[test]
fn test_parse_kicker_keyword_line_compiles_to_optional_cost() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Kicker Probe")
        .card_types(vec![CardType::Instant])
        .parse_text("Kicker {1}{U}\nDraw a card.")
        .expect("kicker keyword line should parse");

    assert_eq!(def.optional_costs.len(), 1);
    let kicker = &def.optional_costs[0];
    assert_eq!(kicker.label, "Kicker");
    assert!(!kicker.repeatable, "kicker should not be repeatable");
    let mana = kicker
        .cost
        .mana_cost()
        .expect("kicker should preserve mana cost");
    assert_eq!(mana.to_oracle(), "{1}{U}");
}

#[test]
fn test_parse_kicker_keyword_line_with_reminder_text_strips_reminder_tail() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Kicker Reminder Probe")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Kicker {2}{R} (You may pay an additional {2}{R} as you cast this spell.)\nDraw a card.",
        )
        .expect("kicker keyword with reminder text should parse");

    assert_eq!(def.optional_costs.len(), 1);
    let kicker = &def.optional_costs[0];
    assert_eq!(kicker.label, "Kicker");
    let mana = kicker
        .cost
        .mana_cost()
        .expect("kicker should preserve mana cost");
    assert_eq!(mana.to_oracle(), "{2}{R}");
}

#[test]
fn test_parse_multikicker_and_entwine_keyword_lines_compile_to_optional_costs() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Multi Optional Probe")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Multikicker {1}{G}\nEntwine {2}\nDraw a card.")
        .expect("multikicker/entwine keyword lines should parse");

    assert_eq!(def.optional_costs.len(), 2);

    let multikicker = &def.optional_costs[0];
    assert_eq!(multikicker.label, "Multikicker");
    assert!(multikicker.repeatable, "multikicker should be repeatable");
    let mana = multikicker
        .cost
        .mana_cost()
        .expect("multikicker should preserve mana cost");
    assert_eq!(mana.to_oracle(), "{1}{G}");

    let entwine = &def.optional_costs[1];
    assert_eq!(entwine.label, "Entwine");
    assert!(!entwine.repeatable, "entwine should not be repeatable");
    let mana = entwine
        .cost
        .mana_cost()
        .expect("entwine should preserve mana cost");
    assert_eq!(mana.to_oracle(), "{2}");
}

#[test]
fn test_parse_squad_keyword_line_compiles_to_optional_cost_and_etb_copy_trigger() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Squad Test")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Soldier])
        .power_toughness(PowerToughness::fixed(3, 3))
        .parse_text("Squad {2}\nFlying")
        .expect("squad keyword line should parse");

    assert_eq!(def.optional_costs.len(), 1, "expected one squad cost");
    let squad = &def.optional_costs[0];
    assert_eq!(squad.label, "Squad");
    assert!(squad.repeatable, "squad should be repeatable");
    let mana = squad
        .cost
        .mana_cost()
        .expect("squad should preserve mana cost");
    assert_eq!(mana.to_oracle(), "{2}");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("squad {2}"),
        "expected squad optional-cost line, got {rendered}"
    );

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("CreateTokenCopyEffect") && debug.contains("TimesPaidLabel(\"Squad\")"),
        "expected squad ETB copy trigger, got {debug}"
    );
    assert!(
        !debug.contains("KeywordMarker") && !debug.contains("MarkerText(\"Squad"),
        "squad should not fall back to a marker, got {debug}"
    );
}

#[test]
fn test_squad_trigger_creates_token_copies_equal_to_times_paid() {
    use crate::ability::AbilityKind;
    use crate::cost::OptionalCostsPaid;
    use crate::executor::{ExecutionContext, execute_effect};
    use crate::object::ObjectKind;
    use crate::tests::test_helpers::setup_two_player_game;
    use crate::zone::Zone;

    let mut game = setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let def = CardDefinitionBuilder::new(CardId::new(), "Squad Test")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Soldier])
        .power_toughness(PowerToughness::fixed(3, 3))
        .parse_text("Squad {2}\nFlying")
        .expect("squad keyword line should parse");

    let source = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let mut paid = OptionalCostsPaid::from_costs(&def.optional_costs);
    paid.pay_times(0, 2);
    game.object_mut(source)
        .expect("source object exists")
        .optional_costs_paid = paid.clone();

    let effects = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) if triggered.trigger.display().contains("enters") => {
                Some(triggered.effects.clone())
            }
            _ => None,
        })
        .expect("squad ETB trigger should exist");

    let mut ctx = ExecutionContext::new_default(source, alice).with_optional_costs_paid(paid);
    for effect in &effects {
        execute_effect(&mut game, effect, &mut ctx).expect("squad effect should resolve");
    }

    let squad_objects: Vec<_> = game
        .battlefield
        .iter()
        .filter_map(|&id| game.object(id))
        .filter(|obj| obj.controller == alice && obj.name == "Squad Test")
        .collect();
    assert_eq!(
        squad_objects.len(),
        3,
        "expected original plus two squad copies"
    );

    let token_count = squad_objects
        .iter()
        .filter(|obj| obj.kind == ObjectKind::Token)
        .count();
    assert_eq!(token_count, 2, "expected two squad-created tokens");
}

#[test]
fn test_parse_scavenge_keyword_line_compiles_to_graveyard_activated_ability() {
    use crate::zone::Zone;

    let def = CardDefinitionBuilder::new(CardId::new(), "Scavenge Test")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Zombie])
        .power_toughness(PowerToughness::fixed(4, 4))
        .parse_text("Scavenge {2}{G}")
        .expect("scavenge keyword line should parse");

    let (ability, activated) = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some((ability, activated)),
            _ => None,
        })
        .expect("scavenge should compile to an activated ability");

    assert_eq!(ability.functional_zones, vec![Zone::Graveyard]);
    assert_eq!(ability.text.as_deref(), Some("Scavenge {2}{G}"));

    let debug = format!("{:?}", activated);
    assert!(
        debug.contains("SourcePower")
            && debug.contains("PlusOnePlusOne")
            && debug.contains("ExileEffect"),
        "expected scavenge cost/effect lowering, got {debug}"
    );
}

#[test]
fn test_scavenge_uses_source_snapshot_power_after_source_is_exiled() {
    use crate::ability::AbilityKind;
    use crate::executor::{ExecutionContext, ResolvedTarget, execute_effect};
    use crate::snapshot::ObjectSnapshot;
    use crate::zone::Zone;

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);

    let scavenge_def = CardDefinitionBuilder::new(CardId::new(), "Scavenge Test")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Zombie])
        .power_toughness(PowerToughness::fixed(4, 4))
        .parse_text("Scavenge {2}{G}")
        .expect("scavenge keyword line should parse");
    let source = game.create_object_from_definition(&scavenge_def, alice, Zone::Graveyard);

    let target_def = CardDefinitionBuilder::new(CardId::new(), "Target Grizzly")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Bear])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let target = game.create_object_from_definition(&target_def, alice, Zone::Battlefield);

    let effect = scavenge_def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => activated.effects.first().cloned(),
            _ => None,
        })
        .expect("scavenge activated effect should exist");

    let source_snapshot =
        ObjectSnapshot::from_object(game.object(source).expect("source object exists"), &game);
    let moved_source = game
        .move_object(source, Zone::Exile)
        .expect("source should move to exile to simulate paying scavenge");
    assert!(
        game.object(moved_source)
            .is_some_and(|obj| obj.zone == Zone::Exile),
        "source should be exiled after paying scavenge"
    );

    let mut ctx = ExecutionContext::new_default(source, alice)
        .with_targets(vec![ResolvedTarget::Object(target)])
        .with_source_snapshot(source_snapshot);
    execute_effect(&mut game, &effect, &mut ctx).expect("scavenge effect should resolve");

    let target_obj = game.object(target).expect("target creature exists");
    assert_eq!(
        target_obj
            .counters
            .get(&crate::object::CounterType::PlusOnePlusOne),
        Some(&4),
        "scavenge should use the exiled source card's power via source snapshot"
    );
}

#[test]
fn test_parse_mobilize_keyword_line_compiles_to_attack_trigger() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Mobilize Test")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Warrior])
        .power_toughness(PowerToughness::fixed(3, 3))
        .parse_text("Mobilize 2")
        .expect("mobilize keyword line should parse");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("ThisAttacksTrigger")
            && debug.contains("CreateTokenEffect")
            && debug.contains("sacrifice_at_next_end_step: true"),
        "expected mobilize attack trigger lowering, got {debug}"
    );
    assert!(
        !debug.contains("MarkerText(\"Mobilize 2\")"),
        "mobilize should not fall back to marker text, got {debug}"
    );
}

#[test]
fn test_mobilize_trigger_creates_attacking_warriors() {
    use crate::ability::AbilityKind;
    use crate::combat_state::{AttackTarget, AttackerInfo, CombatState};
    use crate::executor::{ExecutionContext, execute_effect};
    use crate::zone::Zone;

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let def = CardDefinitionBuilder::new(CardId::new(), "Mobilize Test")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Warrior])
        .power_toughness(PowerToughness::fixed(3, 3))
        .parse_text("Mobilize 2")
        .expect("mobilize keyword line should parse");
    let source = game.create_object_from_definition(&def, alice, Zone::Battlefield);

    game.combat = Some(CombatState {
        attackers: vec![AttackerInfo {
            creature: source,
            target: AttackTarget::Player(bob),
        }],
        ..CombatState::default()
    });

    let effects = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered)
                if triggered.trigger.display().contains("attacks") =>
            {
                Some(triggered.effects.clone())
            }
            _ => None,
        })
        .expect("mobilize trigger should exist");

    let mut ctx = ExecutionContext::new_default(source, alice);
    for effect in &effects {
        execute_effect(&mut game, effect, &mut ctx).expect("mobilize effect should resolve");
    }

    let warrior_ids: Vec<_> = game
        .battlefield
        .iter()
        .copied()
        .filter(|&id| {
            game.object(id)
                .is_some_and(|obj| obj.controller == alice && obj.name == "Warrior")
        })
        .collect();
    assert_eq!(warrior_ids.len(), 2, "expected two mobilize tokens");
    assert!(warrior_ids.iter().all(|&id| game.is_tapped(id)));

    let combat = game.combat.as_ref().expect("combat should exist");
    let warrior_attackers = combat
        .attackers
        .iter()
        .filter(|attacker| warrior_ids.contains(&attacker.creature))
        .count();
    assert_eq!(
        warrior_attackers, 2,
        "mobilize tokens should enter attacking"
    );
}

#[test]
fn test_parse_named_counter_types_fall_back_to_named_counter() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Named Counter Probe")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .parse_text("At the beginning of your upkeep, put a spore counter on this creature.")
        .expect("named counter types should parse");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("Named(\"spore\")"),
        "expected CounterType::Named(\"spore\") in parsed ability, got {debug}"
    );
}

#[test]
fn test_parse_plus_zero_plus_one_counter_type() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "PT Counter Probe")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Put a +0/+1 counter on target creature.")
        .expect("+0/+1 counter type should parse");

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("PlusZeroPlusOne"),
        "expected +0/+1 to map to CounterType::PlusZeroPlusOne, got {debug}"
    );
}

#[test]
fn test_parse_switch_power_toughness_until_eot() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Switch Probe")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 4))
        .parse_text("{U}: Switch this creature's power and toughness until end of turn.")
        .expect("switch P/T clause should parse");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("SwitchPowerToughness"),
        "expected continuous switch P/T modification, got {debug}"
    );
}

#[test]
fn test_parse_suspend_keyword_line_with_reminder_text_keeps_suspend_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Suspend Probe")
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "Suspend 3—{0} (Rather than cast this card from your hand, pay {0} and exile it with three time counters on it. At the beginning of your upkeep, remove a time counter. When the last is removed, you may cast it without paying its mana cost.)",
        )
        .expect("suspend keyword with reminder text should parse");

    let rendered = oracle_like_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("suspend 3—{0}") || rendered.contains("suspend 3 {0}"),
        "expected suspend keyword text in oracle-like output, got {rendered}"
    );
    assert!(
        !rendered.contains("unsupported parser line fallback"),
        "suspend keyword line should not rely on unsupported fallback marker: {rendered}"
    );
    assert_eq!(def.alternative_casts.len(), 1);
    match &def.alternative_casts[0] {
        AlternativeCastingMethod::Suspend { cost, time } => {
            assert_eq!(*time, 3);
            assert_eq!(cost.to_oracle(), "{0}");
        }
        other => panic!("expected suspend metadata, got {other:?}"),
    }
    let debug = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        !debug.contains("unsupported"),
        "suspend parse should avoid unsupported placeholders, got {debug}"
    );
}

#[test]
fn test_parse_plot_keyword_line_compiles_to_alternative_cast() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Plot Probe")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Plot {2}{R} (You may pay {2}{R} and exile this card from your hand. Cast it as a sorcery on a later turn without paying its mana cost. Plot only as a sorcery.)",
        )
        .expect("plot keyword line should parse");

    let rendered = oracle_like_lines(&def).join(" ");
    assert!(
        rendered.contains("Plot {2}{R}"),
        "expected plot cost in render output, got {rendered}"
    );
    assert_eq!(def.alternative_casts.len(), 1);
    match &def.alternative_casts[0] {
        AlternativeCastingMethod::Plot { cost } => {
            assert_eq!(cost.to_oracle(), "{2}{R}");
        }
        other => panic!("expected plot alternative cast, got {other:?}"),
    }
    let debug = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        !debug.contains("unsupported"),
        "plot parse should avoid unsupported placeholders, got {debug}"
    );
}

#[test]
fn test_parse_foretell_keyword_line_compiles_to_alternative_cast() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Foretell Probe")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Foretell {1}{U} (During your turn, you may pay {2} and exile this card from your hand face down. Cast it on a later turn for its foretell cost.)",
        )
        .expect("foretell keyword line should parse");

    let rendered = oracle_like_lines(&def).join(" ");
    assert!(
        rendered.contains("Foretell {1}{U}"),
        "expected foretell cost in render output, got {rendered}"
    );
    assert_eq!(def.alternative_casts.len(), 1);
    match &def.alternative_casts[0] {
        AlternativeCastingMethod::Foretell { cost } => {
            assert_eq!(cost.to_oracle(), "{1}{U}");
        }
        other => panic!("expected foretell alternative cast, got {other:?}"),
    }
    let debug = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        !debug.contains("unsupported"),
        "foretell parse should avoid unsupported placeholders, got {debug}"
    );
}

#[test]
fn test_parse_spectacle_keyword_line_compiles_to_alternative_cast() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Spectacle Probe")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Spectacle {R} (You may cast this spell for its spectacle cost rather than its mana cost if an opponent lost life this turn.)",
        )
        .expect("spectacle keyword line should parse");

    let rendered = oracle_like_lines(&def).join(" ");
    assert!(
        rendered.contains("Spectacle") || rendered.contains("opponent lost life this turn"),
        "expected spectacle render output, got {rendered}"
    );
    assert_eq!(def.alternative_casts.len(), 1);
    let debug = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        debug.contains("spectacle") && !debug.contains("unsupported"),
        "spectacle parse should lower to a real alternative cost, got {debug}"
    );
}

#[test]
fn test_parse_disturb_keyword_line_compiles_to_alternative_cast() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Disturb Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Disturb {1}{W} (You may cast this card from your graveyard transformed for its disturb cost.)",
        )
        .expect("disturb keyword line should parse");

    let rendered = oracle_like_lines(&def).join(" ");
    assert!(
        rendered.contains("Disturb {1}{W}"),
        "expected disturb render output, got {rendered}"
    );
    assert_eq!(def.alternative_casts.len(), 1);
    match &def.alternative_casts[0] {
        AlternativeCastingMethod::Disturb { cost } => {
            assert_eq!(cost.to_oracle(), "{1}{W}");
        }
        other => panic!("expected disturb alternative cast, got {other:?}"),
    }
    let debug = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        !debug.contains("unsupported"),
        "disturb parse should avoid unsupported placeholders, got {debug}"
    );
}

#[test]
fn test_parse_cipher_keyword_line_compiles_to_real_spell_effect() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Cipher Probe")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Draw a card.\nCipher")
        .expect("cipher keyword line should parse");

    let rendered = oracle_like_lines(&def).join(" ");
    assert!(
        rendered.contains("Cipher"),
        "expected cipher render output, got {rendered}"
    );
    let debug = format!("{def:#?}");
    assert!(
        debug.contains("CipherEffect"),
        "expected cipher to compile to a first-class effect, got {debug}"
    );
    assert!(
        !debug.contains("KeywordMarker"),
        "cipher should not remain a keyword marker, got {debug}"
    );
}

#[test]
fn test_parse_overload_keyword_line_compiles_to_alternative_cast_and_rewritten_effects() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Overload Probe")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Target creature you control gets +1/+0 until end of turn.\nOverload {1}{R} (You may cast this spell for its overload cost. If you do, change \"target\" in its text to \"each.\")",
        )
        .expect("overload keyword line should parse");

    let rendered = oracle_like_lines(&def).join(" ");
    assert!(
        rendered.contains("Overload {1}{R}"),
        "expected overload render output, got {rendered}"
    );
    assert_eq!(def.alternative_casts.len(), 1);
    match &def.alternative_casts[0] {
        AlternativeCastingMethod::Overload { cost, effects } => {
            assert_eq!(cost.to_oracle(), "{1}{R}");
            assert!(
                !effects.is_empty(),
                "expected overload to compile a rewritten effect tree"
            );
            let debug = format!("{effects:?}");
            assert!(
                !debug.contains("Target("),
                "expected overloaded effects to be non-targeted, got {debug}"
            );
        }
        other => panic!("expected overload alternative cast, got {other:?}"),
    }
    let debug = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        !debug.contains("unsupported"),
        "overload parse should avoid unsupported placeholders, got {debug}"
    );
}

#[test]
fn test_parse_triggered_explore_clause_without_fallback_marker() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Explore Trigger Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "When this creature enters, it explores. (Reveal the top card of your library. Put that card into your hand if it's a land. Otherwise, put a +1/+1 counter on this creature, then put the card back or put it into your graveyard.)",
        )
        .expect("explore trigger should parse as an explicit mechanic effect");

    let rendered = oracle_like_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("explores"),
        "expected explore text in oracle-like output, got {rendered}"
    );
    assert!(
        !rendered.contains("unsupported parser line fallback"),
        "explore trigger should not rely on unsupported fallback marker: {rendered}"
    );
}

#[test]
fn test_parse_open_attraction_clause_without_fallback_marker() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Open Attraction Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "When this creature enters, open an Attraction. (Put the top card of your Attraction deck onto the battlefield.)",
        )
        .expect("open attraction trigger should parse as an explicit mechanic effect");

    let rendered = oracle_like_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("open an attraction"),
        "expected open-attraction text in oracle-like output, got {rendered}"
    );
    assert!(
        !rendered.contains("unsupported parser line fallback"),
        "open-attraction trigger should not rely on unsupported fallback marker: {rendered}"
    );
}

#[test]
fn test_parse_adapt_activation_with_reminder_text_without_fallback_marker() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Adapt Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "{2}{U}: Adapt 2. (If this creature has no +1/+1 counters on it, put two +1/+1 counters on it.)",
        )
        .expect("adapt activated line should parse as an explicit mechanic effect");

    let rendered = oracle_like_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("adapt 2"),
        "expected adapt text in oracle-like output, got {rendered}"
    );
    assert!(
        !rendered.contains("unsupported parser line fallback"),
        "adapt activation should not rely on unsupported fallback marker: {rendered}"
    );
}

#[test]
fn test_parse_manifest_dread_trigger_without_fallback_marker() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Manifest Dread Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "When this creature dies, manifest dread. (Look at the top two cards of your library. Put one onto the battlefield face down as a 2/2 creature and the other into your graveyard. Turn it face up any time for its mana cost if it's a creature card.)",
        )
        .expect("manifest-dread trigger should parse as an explicit mechanic effect");

    let rendered = oracle_like_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("manifest dread"),
        "expected manifest-dread text in oracle-like output, got {rendered}"
    );
    assert!(
        !rendered.contains("unsupported parser line fallback"),
        "manifest-dread trigger should not rely on unsupported fallback marker: {rendered}"
    );
}

#[test]
fn parse_manifest_dread_then_multi_counter_followup_keeps_full_chain() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Manifest Door Probe")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "When you unlock this door, manifest dread, then put two +1/+1 counters and a trample counter on that creature.",
        )
        .expect("manifest-dread then counter follow-up should parse");

    let rendered = oracle_like_lines(&def).join(" ");
    let lower = rendered.to_ascii_lowercase();
    assert!(
        lower.contains("manifest dread"),
        "expected manifest dread in compiled text, got {rendered}"
    );
    assert!(
        lower.contains("+1/+1 counter"),
        "expected +1/+1 counters in compiled text, got {rendered}"
    );
    assert!(
        lower.contains("trample counter"),
        "expected trample counter in compiled text, got {rendered}"
    );
}

#[test]
fn render_trigger_uses_card_name_when_oracle_uses_name() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Name Trigger Probe")
        .card_types(vec![CardType::Creature])
        .parse_text("Whenever Name Trigger Probe attacks, draw a card.")
        .expect("name-based trigger should parse");

    let rendered = compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Whenever Name Trigger Probe attacks"),
        "expected rendered trigger to keep card name, got {rendered}"
    );
}

#[test]
fn parse_alchemy_prefixed_name_still_resolves_self_reference_triggers() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "A-Oran-Rief Ooze")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "When Oran-Rief Ooze enters, put a +1/+1 counter on target creature you control.",
        )
        .expect("alchemy-prefixed source name should normalize to self reference");

    let rendered = compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("enters, put a +1/+1 counter on target creature you control"),
        "expected enters trigger body to stay intact, got {rendered}"
    );
    assert!(
        !rendered.contains("Whenever a Ooze enters"),
        "alchemy prefix should not degrade source trigger to subtype filter: {rendered}"
    );
}

#[test]
fn parse_multiword_name_first_word_still_resolves_self_reference_triggers() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Loran of the Third Path")
        .card_types(vec![CardType::Creature])
        .parse_text("When Loran enters, destroy up to one target artifact or enchantment.")
        .expect("multiword source name shorthand should normalize to self reference");

    let rendered = compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("When Loran of the Third Path enters")
            || rendered.contains("When this creature enters"),
        "expected self-reference trigger render, got {rendered}"
    );
}

#[test]
fn test_parse_bolster_trigger_without_fallback_marker() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Bolster Trigger Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "When this creature enters, bolster 2. (Choose a creature with the least toughness among creatures you control and put two +1/+1 counters on it.)",
        )
        .expect("bolster trigger should parse as an explicit mechanic effect");

    let rendered = oracle_like_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("bolster 2"),
        "expected bolster text in oracle-like output, got {rendered}"
    );
    assert!(
        !rendered.contains("unsupported parser line fallback"),
        "bolster trigger should not rely on unsupported fallback marker: {rendered}"
    );
}

#[test]
fn test_parse_support_trigger_without_fallback_marker() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Support Trigger Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "When this creature enters, support 3. (Put a +1/+1 counter on each of up to three other target creatures.)",
        )
        .expect("support trigger should parse as an explicit mechanic effect");

    let rendered = oracle_like_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("support 3"),
        "expected support text in oracle-like output, got {rendered}"
    );
    assert!(
        !rendered.contains("unsupported parser line fallback"),
        "support trigger should not rely on unsupported fallback marker: {rendered}"
    );
}

#[test]
fn test_parse_counter_target_activated_or_triggered_ability_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Stifle Probe")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Counter target activated or triggered ability. (Mana abilities can't be targeted.)",
        )
        .expect("counter activated/triggered ability clause should parse");

    let rendered = oracle_like_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("counter target activated ability")
            && rendered.contains("triggered ability"),
        "expected counter-ability text in oracle-like output, got {rendered}"
    );
    assert!(
        !rendered.contains("unsupported parser line fallback"),
        "counter-ability clause should not rely on unsupported fallback marker: {rendered}"
    );
}

#[test]
fn test_parse_counter_target_spell_activated_or_triggered_ability_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Disallow Probe")
        .card_types(vec![CardType::Instant])
        .parse_text("Counter target spell, activated ability, or triggered ability.")
        .expect("counter spell-or-ability clause should parse");

    let rendered = oracle_like_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("counter target spell")
            && rendered.contains("activated ability")
            && rendered.contains("triggered ability"),
        "expected counter spell-or-ability text in oracle-like output, got {rendered}"
    );
    assert!(
        !rendered.contains("unsupported parser line fallback"),
        "counter spell-or-ability clause should not rely on unsupported fallback marker: {rendered}"
    );
}

#[test]
fn test_parse_counter_target_activated_ability_from_artifact_source_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Rust Probe")
        .card_types(vec![CardType::Instant])
        .parse_text("Counter target activated ability from an artifact source.")
        .expect("counter activated-ability from artifact source clause should parse");

    let rendered = oracle_like_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("counter target"),
        "expected counter text in oracle-like output, got {rendered}"
    );
    assert!(
        rendered.contains("artifact"),
        "expected artifact source constraint in oracle-like output, got {rendered}"
    );
    assert!(
        !rendered.contains("unsupported parser line fallback"),
        "counter activated-ability from artifact source should not rely on unsupported fallback marker: {rendered}"
    );
}

#[test]
fn test_parse_counter_target_ability_or_legendary_spell_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Tales End Probe")
        .card_types(vec![CardType::Instant])
        .parse_text("Counter target activated ability, triggered ability, or legendary spell.")
        .expect("counter activated/triggered ability or legendary spell clause should parse");

    let rendered = oracle_like_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("legendary spell"),
        "expected legendary spell selector in oracle-like output, got {rendered}"
    );
    assert!(
        !rendered.contains("unsupported parser line fallback"),
        "counter activated/triggered ability or legendary spell should not rely on unsupported fallback marker: {rendered}"
    );
}

#[test]
fn test_parse_counter_target_ability_or_noncreature_spell_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Louisoix Probe")
        .card_types(vec![CardType::Instant])
        .parse_text("Counter target activated ability, triggered ability, or noncreature spell.")
        .expect("counter activated/triggered ability or noncreature spell clause should parse");

    let rendered = oracle_like_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("noncreature") && rendered.contains("spell"),
        "expected noncreature spell selector in oracle-like output, got {rendered}"
    );
    assert!(
        !rendered.contains("unsupported parser line fallback"),
        "counter activated/triggered ability or noncreature spell should not rely on unsupported fallback marker: {rendered}"
    );
}

#[test]
fn test_parse_counter_up_to_one_target_activated_or_triggered_ability_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Tidebinder Probe")
        .card_types(vec![CardType::Instant])
        .parse_text("Counter up to one target activated or triggered ability.")
        .expect("counter up-to-one activated/triggered ability clause should parse");

    let rendered = oracle_like_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("up to one target"),
        "expected up-to-one target selector in oracle-like output, got {rendered}"
    );
    assert!(
        !rendered.contains("unsupported parser line fallback"),
        "counter up-to-one activated/triggered ability should not rely on unsupported fallback marker: {rendered}"
    );
}

#[test]
fn test_parse_counter_target_ability_you_dont_control_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Obstructionist Probe")
        .card_types(vec![CardType::Instant])
        .parse_text("Counter target activated or triggered ability you don't control.")
        .expect("counter activated/triggered ability you don't control clause should parse");

    let rendered = oracle_like_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("opponent's"),
        "expected controller restriction in oracle-like output, got {rendered}"
    );
    assert!(
        !rendered.contains("unsupported parser line fallback"),
        "counter activated/triggered ability you don't control should not rely on unsupported fallback marker: {rendered}"
    );
}

#[test]
fn test_parse_counter_target_activated_ability_from_permanent_source_unless_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Ayesha Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "{T}: Counter target activated ability from an artifact, creature, enchantment, or land unless that ability's controller pays {W}.",
        )
        .expect("counter activated ability from permanent source unless clause should parse");

    let rendered = oracle_like_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("unless"),
        "expected unless payment clause in oracle-like output, got {rendered}"
    );
    assert!(
        !rendered.contains("unsupported parser line fallback"),
        "counter activated ability from permanent source unless should not rely on unsupported fallback marker: {rendered}"
    );
}

#[test]
fn test_parse_counter_target_instant_or_sorcery_spell_or_ability_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Sister Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "When this creature enters, counter target instant spell, sorcery spell, activated ability, or triggered ability.",
        )
        .expect("counter instant/sorcery spell or activated/triggered ability clause should parse");

    let rendered = oracle_like_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("instant spell"),
        "expected instant-spell selector in oracle-like output, got {rendered}"
    );
    assert!(
        rendered.contains("sorcery spell"),
        "expected sorcery-spell selector in oracle-like output, got {rendered}"
    );
    assert!(
        !rendered.contains("unsupported parser line fallback"),
        "counter instant/sorcery spell or activated/triggered ability should not rely on unsupported fallback marker: {rendered}"
    );
}

#[test]
fn test_parse_prevent_all_damage_to_creatures_static_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Bubble Matrix Probe")
        .card_types(vec![CardType::Artifact])
        .parse_text("Prevent all damage that would be dealt to creatures.")
        .expect("prevent-all damage to creatures clause should parse as static ability");

    let has_prevent_all_to_creatures = def.abilities.iter().any(|ability| {
        matches!(
            &ability.kind,
            AbilityKind::Static(static_ability)
                if static_ability.id() == StaticAbilityId::PreventAllDamageDealtToCreatures
        )
    });
    assert!(
        has_prevent_all_to_creatures,
        "expected PreventAllDamageDealtToCreatures static ability in parsed card"
    );
}

#[test]
fn test_parse_prevent_all_damage_duration_before_target_order_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Sivvi Prevention Probe")
        .card_types(vec![CardType::Instant])
        .parse_text("Prevent all damage that would be dealt this turn to creatures you control.")
        .expect("prevent-all damage clause with duration-before-target order should parse");

    let spell_debug = format!("{:#?}", def.spell_effect);
    assert!(
        spell_debug.contains("PreventAllDamageEffect")
            && spell_debug.contains("PermanentsMatching"),
        "expected non-targeted prevent-all-damage effect in parsed spell text, got {spell_debug}"
    );
}

#[test]
fn test_parse_prevent_all_damage_to_explicit_target_stays_targeted() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Targeted Prevention Probe")
        .card_types(vec![CardType::Instant])
        .parse_text("Prevent all damage that would be dealt this turn to target creature.")
        .expect("targeted prevent-all damage clause should parse");

    let spell_debug = format!("{:#?}", def.spell_effect);
    assert!(
        spell_debug.contains("PreventAllDamageToTarget"),
        "expected targeted prevent-all-damage effect in parsed spell text, got {spell_debug}"
    );
}

#[test]
fn test_parse_cant_be_blocked_as_long_as_defending_player_controls_artifact_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Bouncing Beebles Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "This creature can't be blocked as long as defending player controls an artifact.",
        )
        .expect("defending-player artifact unblockable clause should parse");

    let has_conditional_unblockable = def.abilities.iter().any(|ability| {
        matches!(
            &ability.kind,
            AbilityKind::Static(static_ability)
                if static_ability.id()
                    == StaticAbilityId::CantBeBlockedAsLongAsDefendingPlayerControlsCardType
        )
    });
    assert!(
        has_conditional_unblockable,
        "expected defending-player artifact unblockable static ability in parsed card"
    );
}

#[test]
fn test_parse_cant_be_blocked_as_long_as_defending_player_controls_artifact_land_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Tanglewalker Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "This creature can't be blocked as long as defending player controls an artifact land.",
        )
        .expect("defending-player artifact-land unblockable clause should parse");

    let has_multi_type_conditional_unblockable = def.abilities.iter().any(|ability| {
        matches!(
            &ability.kind,
            AbilityKind::Static(static_ability)
                if static_ability.id()
                    == StaticAbilityId::CantBeBlockedAsLongAsDefendingPlayerControlsCardTypes
        )
    });
    assert!(
        has_multi_type_conditional_unblockable,
        "expected defending-player artifact-land unblockable static ability in parsed card"
    );
}

#[test]
fn test_parse_add_any_type_that_land_produced_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Heartbeat Probe")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "Whenever a player taps a land for mana, that player adds one mana of any type that land produced.",
        )
        .expect("land-produced mana clause should parse");

    let rendered = oracle_like_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("whenever a player taps a land for mana"),
        "expected tap-for-mana trigger text in oracle-like output, got {rendered}"
    );
    assert!(
        rendered.contains("adds one mana of any type")
            || rendered.contains("add 1 mana of any type"),
        "expected add-any-type mana text in oracle-like output, got {rendered}"
    );
    assert!(
        !rendered.contains("unsupported parser line fallback"),
        "land-produced mana clause should not rely on unsupported fallback marker: {rendered}"
    );
}

#[test]
fn test_parse_raid_conditional_with_attacked_this_turn_without_fallback() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Raid Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Raid — When this creature enters, if you attacked this turn, this creature deals 2 damage to any target.",
        )
        .expect("raid conditional should parse");

    let rendered = oracle_like_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("if you attacked this turn"),
        "expected attacked-this-turn predicate in rendered text, got {rendered}"
    );
    assert!(
        !rendered.contains("unsupported parser line fallback"),
        "raid conditional should not rely on unsupported fallback marker: {rendered}"
    );
}

#[test]
fn test_parse_x_target_lands_clause_without_fallback() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "X Untap Probe")
        .card_types(vec![CardType::Artifact])
        .parse_text("{X}, {T}: Untap X target lands.")
        .expect("x-target untap clause should parse");

    let rendered = oracle_like_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("target lands"),
        "expected target-lands wording in rendered text, got {rendered}"
    );
    assert!(
        !rendered.contains("unsupported parser line fallback"),
        "x-target untap clause should not rely on unsupported fallback marker: {rendered}"
    );
}

#[test]
fn test_parse_exile_cards_from_single_graveyard_without_fallback() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Single Graveyard Probe")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Exile up to three target cards from a single graveyard.")
        .expect("single-graveyard exile clause should parse");

    let rendered = oracle_like_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("single graveyard"),
        "expected single-graveyard wording in rendered text, got {rendered}"
    );
    assert!(
        !rendered.contains("unsupported parser line fallback"),
        "single-graveyard exile clause should not rely on unsupported fallback marker: {rendered}"
    );
}

#[test]
fn test_parse_each_of_them_gets_clause_targets_selected_objects() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Hope and Glory Probe")
        .card_types(vec![CardType::Instant])
        .parse_text("Untap two target creatures. Each of them gets +1/+1 until end of turn.")
        .expect("each-of-them gets clause should parse");

    let rendered = oracle_like_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("untap two target creatures"),
        "expected untap-target-creatures clause in rendered text, got {rendered}"
    );
    assert!(
        !rendered.contains("this spell gets +1/+1"),
        "selected creatures should be pumped, not the spell itself: {rendered}"
    );
}

#[test]
fn test_parse_return_cards_at_random_from_graveyard_to_hand() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Make a Wish Probe")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Return two cards at random from your graveyard to your hand.")
        .expect("return-at-random clause should parse");

    let rendered = oracle_like_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("return two cards at random from your graveyard to your hand"),
        "expected random graveyard return wording in rendered text, got {rendered}"
    );
}

#[test]
fn test_parse_one_word_verb_card_name_does_not_break_clause_parsing() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Regenerate")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Regenerate target creature. (The next time that creature would be destroyed this turn, instead tap it, remove it from combat, and heal all damage on it.)",
        )
        .expect("verb-named card should still parse regenerate clause");

    let rendered = oracle_like_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("regenerate target creature"),
        "expected regenerate clause in rendered text, got {rendered}"
    );
    assert!(
        !rendered.contains("unsupported parser line fallback"),
        "verb-named card should not rely on unsupported fallback marker: {rendered}"
    );
}

#[test]
fn test_render_enters_with_single_counter_uses_singular_wording() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Single Counter Probe")
        .card_types(vec![CardType::Creature])
        .parse_text("This creature enters with a +1/+1 counter on it.")
        .expect("single-counter enters clause should parse");

    let rendered = oracle_like_lines(&def).join(" ");
    assert!(
        rendered.contains("enters with a +1/+1 counter on it"),
        "expected singular enters-with-counter wording, got {rendered}"
    );
}

#[test]
fn parse_tayam_oracle_text_regression() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Tayam, Luminous Enigma")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Each other creature you control enters with an additional vigilance counter on it.\n\
             {3}, Remove three counters from among creatures you control: Mill three cards, then return a permanent card with mana value 3 or less from your graveyard to the battlefield.",
        )
        .expect("tayam oracle text should parse");

    let static_ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        static_ids.contains(&StaticAbilityId::EnterWithCountersForFilter),
        "expected ETB counter replacement static ability, got {static_ids:?}"
    );
    assert!(
        !static_ids.contains(&StaticAbilityId::RuleTextPlaceholder),
        "tayam oracle text should not fall back to placeholder static ability: {static_ids:?}"
    );

    let activated = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("expected activated ability");

    let cost_debug = format!("{:?}", activated.mana_cost);
    assert!(
        cost_debug.contains("CostEffect")
            && cost_debug.contains("RemoveAnyCountersAmongEffect")
            && cost_debug.contains("count: 3"),
        "expected effect-backed remove-three-counters-among cost, got {cost_debug}"
    );
    assert!(
        activated
            .mana_cost
            .non_mana_costs()
            .any(|cost| cost.effect_ref().is_some_and(|effect| effect
                .downcast_ref::<crate::effects::RemoveAnyCountersAmongEffect>()
                .is_some())),
        "expected Tayam activation to expose an effect-backed staged remove-counters-among cost"
    );

    let effects_debug = format!("{:?}", activated.effects);
    assert!(
        effects_debug.contains("MillEffect"),
        "expected mill effect in tayam activated ability, got {effects_debug}"
    );
    assert!(
        effects_debug.contains("ChooseObjectsEffect"),
        "expected runtime graveyard choice in tayam activated ability, got {effects_debug}"
    );
    assert!(
        effects_debug.contains("ReturnFromGraveyardToBattlefieldEffect"),
        "expected return-from-graveyard effect in tayam activated ability, got {effects_debug}"
    );
    assert!(
        effects_debug.contains("mana_value: Some(")
            && effects_debug.contains("LessThanOrEqual")
            && effects_debug.contains("Artifact")
            && effects_debug.contains("Creature")
            && effects_debug.contains("Enchantment")
            && effects_debug.contains("Land")
            && effects_debug.contains("Planeswalker"),
        "expected permanent-card mana-value<=3 filter in return effect, got {effects_debug}"
    );
}

#[test]
fn parse_enters_with_counter_if_you_attacked_this_turn_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Goblin Boarders Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("This creature enters with a +1/+1 counter on it if you attacked this turn.")
        .expect("raid enters-with-counter clause should parse");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::EnterWithCountersIfCondition),
        "expected conditional enters-with-counters ability, got {ids:?}"
    );
    assert!(
        !ids.contains(&crate::static_abilities::StaticAbilityId::RuleTextPlaceholder),
        "raid enters-with-counter should not fall back to placeholder static ability: {ids:?}"
    );

    let rendered = oracle_like_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("if you attacked this turn"),
        "expected raid condition text in rendered output, got {rendered}"
    );
}

#[test]
fn parse_enters_with_x_plus_one_counters_line_is_typed_static() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Endless One Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("This creature enters with X +1/+1 counters on it.")
        .expect("x enters-with-counters clause should parse");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::EnterWithCounters),
        "expected typed enters-with-counters static ability, got {ids:?}"
    );
    assert!(
        !ids.contains(&crate::static_abilities::StaticAbilityId::RuleTextPlaceholder),
        "x enters-with-counters should not fall back to placeholder static ability: {ids:?}"
    );
}

#[test]
fn parse_enters_with_counter_if_opponent_lost_life_is_typed_static() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Frilled Sparkshooter Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "This creature enters with a +1/+1 counter on it if an opponent lost life this turn.",
        )
        .expect("opponent-life-loss conditional enters-with-counters should parse");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::EnterWithCountersIfCondition),
        "expected conditional enters-with-counters ability, got {ids:?}"
    );
    assert!(
        !ids.contains(&crate::static_abilities::StaticAbilityId::RuleTextPlaceholder),
        "opponent-life-loss conditional variant should not use placeholder fallback: {ids:?}"
    );
}

#[test]
fn parse_enters_with_counter_if_creature_died_this_turn_is_typed_static() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Moldering Reclaimer Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "This creature enters with two +1/+1 counters on it if a creature died this turn.",
        )
        .expect("creature-died conditional enters-with-counters should parse");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::EnterWithCountersIfCondition),
        "expected conditional enters-with-counters ability, got {ids:?}"
    );
    assert!(
        !ids.contains(&crate::static_abilities::StaticAbilityId::RuleTextPlaceholder),
        "creature-died conditional variant should not use placeholder fallback: {ids:?}"
    );
}

#[test]
fn parse_enters_with_counter_if_permanent_left_under_your_control_is_typed_static() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Fountainport Charmer Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "This creature enters with two +1/+1 counters on it if a permanent left the battlefield under your control this turn.",
        )
        .expect("permanent-left conditional enters-with-counters should parse");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::EnterWithCountersIfCondition),
        "expected conditional enters-with-counters ability, got {ids:?}"
    );
    assert!(
        !ids.contains(&crate::static_abilities::StaticAbilityId::RuleTextPlaceholder),
        "permanent-left conditional variant should not use placeholder fallback: {ids:?}"
    );
}

#[test]
fn parse_enters_with_counter_for_each_creature_that_died_this_turn_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Bloodcrazed Paladin Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("This creature enters with a +1/+1 counter on it for each creature that died this turn.")
        .expect("for-each-creature-died enters-with-counter clause should parse");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("CreaturesDiedThisTurn")
            || debug.contains("for each creature that died this turn"),
        "expected creatures-died-this-turn value in static ability, got {debug}"
    );
}

#[test]
fn parse_enters_with_counter_for_each_color_of_mana_spent_to_cast_it_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Springmantle Cleric Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "This creature enters with a +1/+1 counter on it for each color of mana spent to cast it.",
        )
        .expect("spent-to-cast enters-with-counter clause should parse");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("ColorsOfManaSpentToCastThisSpell")
            || debug.contains("for each color of mana spent to cast it"),
        "expected spent-to-cast color value in static ability, got {debug}"
    );
}

#[test]
fn parse_enters_with_counter_for_each_time_it_was_kicked_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Apex Hawks Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("This creature enters with a +1/+1 counter on it for each time it was kicked.")
        .expect("for-each-time-kicked enters-with-counter clause should parse");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::EnterWithCounters),
        "expected typed enters-with-counters static ability, got {ids:?}"
    );
    assert!(
        !ids.contains(&crate::static_abilities::StaticAbilityId::RuleTextPlaceholder),
        "for-each-time-kicked variant should not use placeholder fallback: {ids:?}"
    );

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("KickCount"),
        "expected kick-count value in static ability, got {debug}"
    );
}

#[test]
fn parse_enters_with_counter_for_each_creature_card_in_your_graveyard_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Golgari Raiders Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "This creature enters with a +1/+1 counter on it for each creature card in your graveyard.",
        )
        .expect("for-each-creature-card-in-graveyard enters-with-counter clause should parse");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::EnterWithCounters),
        "expected typed enters-with-counters static ability, got {ids:?}"
    );
    assert!(
        !ids.contains(&crate::static_abilities::StaticAbilityId::RuleTextPlaceholder),
        "graveyard-count enters-with-counter variant should not use placeholder fallback: {ids:?}"
    );
}

#[test]
fn parse_enters_with_counter_equal_to_number_of_creature_cards_in_your_graveyard_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Rhizome Lurcher Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "This creature enters with a number of +1/+1 counters on it equal to the number of creature cards in your graveyard.",
        )
        .expect("equal-to-number-of-creature-cards enters-with-counter clause should parse");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::EnterWithCounters),
        "expected typed enters-with-counters static ability, got {ids:?}"
    );
    assert!(
        !ids.contains(&crate::static_abilities::StaticAbilityId::RuleTextPlaceholder),
        "equal-to-count enters-with-counter variant should not use placeholder fallback: {ids:?}"
    );
}

#[test]
fn parse_enters_with_counter_if_you_control_creature_with_power_four_or_greater_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Frontier Mastodon Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "This creature enters with a +1/+1 counter on it if you control a creature with power 4 or greater.",
        )
        .expect("control-power conditional enters-with-counter clause should parse");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::EnterWithCountersIfCondition),
        "expected conditional enters-with-counters ability, got {ids:?}"
    );
    assert!(
        !ids.contains(&crate::static_abilities::StaticAbilityId::RuleTextPlaceholder),
        "control-power conditional variant should not use placeholder fallback: {ids:?}"
    );
}

#[test]
fn parse_enters_with_counter_for_each_other_creature_and_or_artifact_you_control_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Luxknight Breacher Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "This creature enters with a +1/+1 counter on it for each other creature and/or artifact you control.",
        )
        .expect("for-each-other-creature-and-or-artifact enters-with-counter clause should parse");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::EnterWithCounters),
        "expected typed enters-with-counters static ability, got {ids:?}"
    );
    assert!(
        !ids.contains(&crate::static_abilities::StaticAbilityId::RuleTextPlaceholder),
        "for-each-other-creature-and-or-artifact variant should not use placeholder fallback: {ids:?}"
    );
}

#[test]
fn parse_enters_with_counter_if_x_is_five_or_more_additional_x_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Apocalypse Hydra Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "This creature enters with X +1/+1 counters on it. If X is 5 or more, it enters with an additional X +1/+1 counters on it.",
        )
        .expect("x-threshold additional enters-with-counters clause should parse");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::EnterWithCounters),
        "expected baseline enters-with-counters static ability, got {ids:?}"
    );
    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::EnterWithCountersIfCondition),
        "expected conditional additional enters-with-counters static ability, got {ids:?}"
    );
    assert!(
        !ids.contains(&crate::static_abilities::StaticAbilityId::RuleTextPlaceholder),
        "x-threshold additional enters-with-counters variant should not use placeholder fallback: {ids:?}"
    );

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("XValueAtLeast(\n                                                            5,\n                                                        )")
            || debug.contains("XValueAtLeast(5)"),
        "expected X-threshold condition in static ability, got {debug}"
    );
}

#[test]
fn parse_enters_with_counter_where_x_is_total_life_lost_by_opponents_this_turn_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Cryptborn Horror Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "This creature enters with X +1/+1 counters on it, where X is the total life lost by your opponents this turn.",
        )
        .expect("where-x-total-life-lost enters-with-counters clause should parse");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::EnterWithCounters),
        "expected typed enters-with-counters static ability, got {ids:?}"
    );
    assert!(
        !ids.contains(&crate::static_abilities::StaticAbilityId::RuleTextPlaceholder),
        "where-x-total-life-lost variant should not use placeholder fallback: {ids:?}"
    );

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("LifeLostThisTurn(\n                                                            Opponent,\n                                                        )")
            || debug.contains("LifeLostThisTurn(Opponent)"),
        "expected life-lost-this-turn value in static ability, got {debug}"
    );
}

#[test]
fn parse_enters_with_counter_unless_two_or_more_colors_of_mana_were_spent_to_cast_it_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Steel Exemplar Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "This creature enters with two +1/+1 counters on it unless two or more colors of mana were spent to cast it.",
        )
        .expect("unless-colors-spent enters-with-counters clause should parse");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::EnterWithCountersIfCondition),
        "expected conditional enters-with-counters static ability, got {ids:?}"
    );
    assert!(
        !ids.contains(&crate::static_abilities::StaticAbilityId::RuleTextPlaceholder),
        "unless-colors-spent variant should not use placeholder fallback: {ids:?}"
    );

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("ColorsOfManaSpentToCastThisSpellOrMore(\n                                                                    2,\n                                                                )")
            || debug.contains("ColorsOfManaSpentToCastThisSpellOrMore(2)"),
        "expected distinct-colors-spent condition in static ability, got {debug}"
    );
}

#[test]
fn parse_enters_with_counter_if_youve_cast_two_or_more_spells_this_turn_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Effortless Master Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "This creature enters with two +1/+1 counters on it if you've cast two or more spells this turn.",
        )
        .expect("cast-two-spells conditional enters-with-counters clause should parse");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::EnterWithCountersIfCondition),
        "expected conditional enters-with-counters static ability, got {ids:?}"
    );
    assert!(
        !ids.contains(&crate::static_abilities::StaticAbilityId::RuleTextPlaceholder),
        "cast-two-spells conditional variant should not use placeholder fallback: {ids:?}"
    );

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("PlayerCastSpellsThisTurnOrMore")
            && (debug.contains("count: 2")
                || debug.contains(
                    "count:\n                                                                    2"
                )),
        "expected spells-cast-this-turn threshold condition in static ability, got {debug}"
    );
}

#[test]
fn parse_enters_with_counter_equal_to_greatest_number_of_cards_an_opponent_has_drawn_this_turn_line()
 {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Thought Sponge Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "This creature enters with a number of +1/+1 counters on it equal to the greatest number of cards an opponent has drawn this turn.",
        )
        .expect("equal-to-greatest-cards-drawn enters-with-counters clause should parse");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::EnterWithCounters),
        "expected typed enters-with-counters static ability, got {ids:?}"
    );
    assert!(
        !ids.contains(&crate::static_abilities::StaticAbilityId::RuleTextPlaceholder),
        "equal-to-greatest-cards-drawn variant should not use placeholder fallback: {ids:?}"
    );

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("MaxCardsDrawnThisTurn(\n                                                            Opponent,\n                                                        )")
            || debug.contains("MaxCardsDrawnThisTurn(Opponent)"),
        "expected max-cards-drawn-this-turn value in static ability, got {debug}"
    );
}

#[test]
fn parse_enters_with_counter_plus_additional_for_each_other_creature_you_control_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Sheriff of Safe Passage Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "This creature enters with a +1/+1 counter on it plus an additional +1/+1 counter on it for each other creature you control.",
        )
        .expect("plus-additional-for-each enters-with-counters clause should parse");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::EnterWithCounters),
        "expected typed enters-with-counters static ability, got {ids:?}"
    );
    assert!(
        !ids.contains(&crate::static_abilities::StaticAbilityId::RuleTextPlaceholder),
        "plus-additional-for-each variant should not use placeholder fallback: {ids:?}"
    );

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("Add(") && debug.contains("other: true"),
        "expected additive counter value with 'other creature' filter in static ability, got {debug}"
    );
}

#[test]
fn parse_enters_with_counter_for_each_magic_game_you_lost_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Gus Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "This creature enters with a +1/+1 counter on it for each Magic game you have lost to one of your opponents since you last won a game against them.",
        )
        .expect("match-history enters-with-counters clause should parse");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::EnterWithCounters),
        "expected typed enters-with-counters static ability, got {ids:?}"
    );
    assert!(
        !ids.contains(&crate::static_abilities::StaticAbilityId::RuleTextPlaceholder),
        "match-history variant should not use placeholder fallback: {ids:?}"
    );

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("MagicGamesLostToOpponentsSinceLastWin"),
        "expected dedicated match-history counter value in static ability, got {debug}"
    );
}

#[test]
fn parse_enters_with_counter_additional_x_if_threshold_direct_clause() {
    let tokens = tokenize_line(
        "it enters with an additional x +1/+1 counters on it if x is 5 or more",
        0,
    );
    let parsed = crate::cards::builders::parse_enters_with_counters_line(&tokens)
        .expect("direct additional-x clause should not error")
        .expect("direct additional-x clause should parse as a static ability");

    assert_eq!(
        parsed.id(),
        crate::static_abilities::StaticAbilityId::EnterWithCountersIfCondition,
        "expected direct additional-x clause to compile to conditional enters-with-counters"
    );
}

#[test]
fn parse_as_this_land_enters_reveal_if_you_dont_enters_tapped_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Secluded Glen Variant")
        .card_types(vec![CardType::Land])
        .parse_text(
            "As this land enters, you may reveal a Faerie card from your hand. If you don't, this land enters tapped.",
        )
        .expect("reveal-if-you-dont land ETB clause should parse");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        ids.contains(&StaticAbilityId::EntersTappedUnlessCondition),
        "expected generic enters-tapped-unless replacement, got {ids:?}"
    );
    assert!(
        !ids.contains(&StaticAbilityId::RuleTextPlaceholder),
        "reveal-if-you-dont clause should not emit placeholder static ability: {ids:?}"
    );

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("YouHaveCardInHandMatching"),
        "expected hand-match condition in replacement ability, got {debug}"
    );
}

#[test]
fn parse_as_this_land_enters_reveal_unless_revealed_or_control_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Temple of the Dragon Queen Variant")
        .card_types(vec![CardType::Land])
        .parse_text(
            "As this land enters, you may reveal a Dragon card from your hand. This land enters tapped unless you revealed a Dragon card this way or you control a Dragon.",
        )
        .expect("reveal-unless-or-control land ETB clause should parse");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        ids.contains(&StaticAbilityId::EntersTappedUnlessCondition),
        "expected generic enters-tapped-unless replacement, got {ids:?}"
    );
    assert!(
        !ids.contains(&StaticAbilityId::RuleTextPlaceholder),
        "reveal-unless-or-control clause should not emit placeholder static ability: {ids:?}"
    );

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("YouHaveCardInHandMatching") && debug.contains("YouControl"),
        "expected OR condition combining hand-match and you-control checks, got {debug}"
    );
}

#[test]
fn test_render_sacrifice_unless_you_pay_uses_pay_verb() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Conversion Probe")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "At the beginning of your upkeep, sacrifice this enchantment unless you pay {W}{W}.",
        )
        .expect("sacrifice-unless-pay upkeep clause should parse");

    let rendered = oracle_like_lines(&def).join(" ");
    assert!(
        rendered.contains("unless you pay {W}{W}"),
        "expected 'you pay' wording in rendered text, got {rendered}"
    );
    assert!(
        !rendered.contains("you pays"),
        "renderer should never emit 'you pays', got {rendered}"
    );
}

#[test]
fn test_render_leading_unless_payment_clause_keeps_unless_structure() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Demonic Hordes Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "At the beginning of your upkeep, unless you pay {B}{B}{B}, tap this creature and sacrifice a land.",
        )
        .expect("leading-unless upkeep clause should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("unless you pay {b}{b}{b}"),
        "expected unless-payment wording, got {rendered}"
    );
    assert!(
        rendered.contains("tap this creature"),
        "expected tap effect in unless branch, got {rendered}"
    );
    assert!(
        rendered.contains("sacrifice a land"),
        "expected sacrifice effect in unless branch, got {rendered}"
    );
}

#[test]
fn parse_rhystic_study_unless_that_player_does_not_flip_to_you() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Rhystic Study")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "Whenever an opponent casts a spell, you may draw a card unless that player pays {1}.",
        )
        .expect("rhystic-study style unless-payment clause should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        !rendered.contains("unless you pay {1}"),
        "payer should not collapse to you in rhystic-style trigger, got {rendered}"
    );
    assert!(
        rendered.contains("unless that player pays {1}")
            || rendered.contains("unless they pay {1}")
            || rendered.contains("unless an opponent pays {1}"),
        "expected non-you payer in rhystic-style trigger, got {rendered}"
    );
}

#[test]
fn test_parse_creatures_without_flying_cant_attack_static_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Moat Probe")
        .card_types(vec![CardType::Enchantment])
        .parse_text("Creatures without flying can't attack.")
        .expect("creatures-without-flying restriction should parse");

    let rendered = oracle_like_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("creatures without flying can't attack"),
        "expected static restriction text in oracle-like output, got {rendered}"
    );
}

#[test]
fn test_parse_this_creature_cant_attack_alone_static_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Bonded Construct Probe")
        .card_types(vec![CardType::Artifact])
        .parse_text("This creature can't attack alone.")
        .expect("cant-attack-alone restriction should parse");

    let rendered = oracle_like_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("this creature can't attack alone"),
        "expected cant-attack-alone text in oracle-like output, got {rendered}"
    );

    let static_ids = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(
        static_ids.contains(&StaticAbilityId::RuleRestriction),
        "expected rule-restriction static ability id, got {static_ids:?}"
    );
}

#[test]
fn test_parse_this_token_cant_attack_or_block_alone_static_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Token Restriction Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "When this creature enters, create a 4/4 white Beast creature token with \"This token can't attack or block alone.\"",
        )
        .expect("token cant-attack-or-block-alone restriction should parse");

    let rendered = oracle_like_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("this token can't attack or block alone"),
        "expected token cant-attack-or-block-alone text in oracle-like output, got {rendered}"
    );

    assert!(
        rendered.contains("can't attack or block alone"),
        "expected token self-restriction text in render output, got {rendered}"
    );
}

#[test]
fn test_parse_activated_abilities_of_artifacts_cant_be_activated_static_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Collector Ouphe Probe")
        .card_types(vec![CardType::Creature])
        .parse_text("Activated abilities of artifacts can't be activated.")
        .expect("activated-abilities-of restriction should parse");

    let rendered = oracle_like_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("activated abilities of artifacts can't be activated"),
        "expected activated-abilities-of restriction text, got {rendered}"
    );

    let static_ids = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(
        static_ids.contains(&StaticAbilityId::RuleRestriction),
        "expected rule-restriction static ability id, got {static_ids:?}"
    );
}

#[test]
fn test_parse_activated_abilities_of_artifacts_and_creatures_unless_mana_static_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Damping Matrix Probe")
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "Activated abilities of artifacts and creatures can't be activated unless they're mana abilities.",
        )
        .expect("matrix-style restriction should parse");

    let rendered = oracle_like_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("activated abilities of artifacts and creatures can't be activated unless they're mana abilities"),
        "expected matrix-style restriction text, got {rendered}"
    );

    let static_ids = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(
        static_ids.contains(&StaticAbilityId::RuleRestriction),
        "expected rule-restriction static ability id, got {static_ids:?}"
    );
}

#[test]
fn test_parse_lands_dont_untap_during_controllers_steps_static_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Rising Waters Probe")
        .card_types(vec![CardType::Enchantment])
        .parse_text("Lands don't untap during their controllers' untap steps.")
        .expect("lands-dont-untap restriction should parse");

    let rendered = oracle_like_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("lands don't untap during their controllers' untap steps"),
        "expected lands-dont-untap text in oracle-like output, got {rendered}"
    );

    let static_ids = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(
        static_ids.contains(&StaticAbilityId::RuleRestriction),
        "expected rule-restriction static ability id, got {static_ids:?}"
    );
}

#[test]
fn parse_flying_only_restriction_does_not_widen_to_reach() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Treetop Restriction Probe")
        .card_types(vec![CardType::Creature])
        .parse_text("This creature can't be blocked except by creatures with flying.")
        .expect("flying-only block restriction should parse");

    let static_ids = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(
        static_ids.contains(&crate::static_abilities::StaticAbilityId::FlyingOnlyRestriction),
        "expected flying-only restriction id, got {static_ids:?}"
    );

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("except by creatures with flying"),
        "expected flying-only text in render, got {rendered}"
    );
    assert!(
        !rendered.contains("flying or reach"),
        "flying-only restriction must not widen to reach, got {rendered}"
    );
}

#[test]
fn parse_conditional_spell_cost_if_it_targets_compiles_target_filter() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Conditional Cost Probe")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "This spell costs {3} less to cast if it targets a tapped creature.\nDestroy target creature.",
        )
        .expect("conditional spell-cost clause should parse");

    let static_abilities = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(
        !static_abilities.is_empty(),
        "expected at least one static ability for conditional spell cost, got {static_abilities:?}"
    );

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("if it targets tapped creature"),
        "expected tapped-target condition in rendered cost reduction, got {rendered}"
    );
    assert!(
        !rendered.contains("spells cost {3} less to cast"),
        "unconditional cost reduction text should not be rendered, got {rendered}"
    );
}

#[test]
fn test_parse_madness_keyword_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Madness Probe")
        .card_types(vec![CardType::Instant])
        .parse_text("Madness {1}{R}")
        .expect("madness keyword line should parse");

    assert_eq!(def.alternative_casts.len(), 1);
    match &def.alternative_casts[0] {
        AlternativeCastingMethod::Madness { cost } => {
            assert_eq!(cost.to_oracle(), "{1}{R}");
        }
        other => panic!("expected madness alternative cast, got {other:?}"),
    }
}

#[test]
fn test_parse_devoid_keyword_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Devoid Probe")
        .card_types(vec![CardType::Creature])
        .parse_text("Devoid")
        .expect("devoid keyword line should parse");

    let has_make_colorless = def.abilities.iter().any(|ability| {
        matches!(
            &ability.kind,
            AbilityKind::Static(static_ability)
                if static_ability.id() == StaticAbilityId::MakeColorless
        )
    });
    assert!(
        has_make_colorless,
        "expected devoid to compile to a make-colorless static ability"
    );

    let devoid_ability = def
        .abilities
        .iter()
        .find(|ability| {
            matches!(
                &ability.kind,
                AbilityKind::Static(static_ability)
                    if static_ability.id() == StaticAbilityId::MakeColorless
            )
        })
        .expect("expected to find devoid ability");
    assert!(
        devoid_ability.functions_in(&Zone::Hand)
            && devoid_ability.functions_in(&Zone::Library)
            && devoid_ability.functions_in(&Zone::Battlefield)
            && devoid_ability.functions_in(&Zone::Stack)
            && devoid_ability.functions_in(&Zone::Graveyard)
            && devoid_ability.functions_in(&Zone::Exile)
            && devoid_ability.functions_in(&Zone::Command),
        "devoid should function in all zones"
    );

    let rendered = compiled_lines(&def).join(" | ");
    assert!(
        rendered.contains("Devoid"),
        "expected compiled text to include Devoid, got {rendered}"
    );
}

#[test]
fn test_parse_landwalk_keyword_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Swampwalk Probe")
        .card_types(vec![CardType::Creature])
        .parse_text("Swampwalk")
        .expect("swampwalk keyword line should parse");

    let has_landwalk = def.abilities.iter().any(|ability| {
        matches!(
            &ability.kind,
            AbilityKind::Static(static_ability)
                if static_ability.id() == StaticAbilityId::Landwalk
        )
    });
    assert!(has_landwalk, "expected swampwalk to compile to Landwalk");
}

#[test]
fn test_parse_cant_be_blocked_by_more_than_one_creature() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Max Blockers Probe")
        .card_types(vec![CardType::Creature])
        .parse_text("This creature can't be blocked by more than one creature.")
        .expect("max-blockers line should parse");

    let has_max_blockers = def.abilities.iter().any(|ability| {
        matches!(
            &ability.kind,
            AbilityKind::Static(static_ability)
                if static_ability.id() == StaticAbilityId::CantBeBlockedByMoreThan
        )
    });
    assert!(
        has_max_blockers,
        "expected max-blockers text to compile to static ability"
    );
}

#[test]
fn test_parse_each_creature_cant_be_blocked_by_more_than_one_creature() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Familiar Ground Probe")
        .card_types(vec![CardType::Enchantment])
        .parse_text("Each creature can't be blocked by more than one creature.")
        .expect("global max-blockers line should parse");

    let has_grant = def.abilities.iter().any(|ability| {
        matches!(
            &ability.kind,
            AbilityKind::Static(static_ability) if static_ability.grants_abilities()
        )
    });
    assert!(
        has_grant,
        "expected Familiar Ground-style line to compile to an ability-granting static ability"
    );
}

#[test]
fn test_parse_each_creature_can_block_additional_creature_each_combat() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "High Ground Probe")
        .card_types(vec![CardType::Enchantment])
        .parse_text("Each creature can block an additional creature each combat.")
        .expect("global can-block-additional line should parse");

    let has_grant = def.abilities.iter().any(|ability| {
        matches!(
            &ability.kind,
            AbilityKind::Static(static_ability) if static_ability.grants_abilities()
        )
    });
    assert!(
        has_grant,
        "expected High Ground-style line to compile to an ability-granting static ability"
    );
}

#[test]
fn test_parse_trigger_becomes_targeted_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Phantasmal Probe")
        .card_types(vec![CardType::Creature])
        .parse_text("When this creature becomes the target of a spell or ability, sacrifice it.")
        .expect("becomes-targeted trigger should parse");

    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("expected triggered ability");
    assert!(
        triggered.trigger.display().contains("becomes the target"),
        "expected becomes-targeted trigger display, got {}",
        triggered.trigger.display()
    );
    let debug = format!("{:#?}", triggered.effects);
    assert!(
        debug.contains("SacrificeTargetEffect"),
        "expected direct sacrifice-target lowering for 'sacrifice it', got {debug}"
    );
    assert!(
        !debug.contains("ChooseObjectsEffect"),
        "unexpected chooser scaffolding for 'sacrifice it': {debug}"
    );
}

#[test]
fn test_parse_assign_damage_as_unblocked_with_this_creature() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Thorn Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "You may have this creature assign its combat damage as though it weren't blocked.",
        )
        .expect("assign-as-unblocked wording with 'this creature' should parse");

    let has_assign_as_unblocked = def.abilities.iter().any(|ability| {
        matches!(
            &ability.kind,
            AbilityKind::Static(static_ability)
                if static_ability.id() == StaticAbilityId::MayAssignDamageAsUnblocked
        )
    });
    assert!(
        has_assign_as_unblocked,
        "expected static may-assign-damage-as-unblocked ability"
    );
}

#[test]
fn test_parse_first_spell_cost_modifier_marker_errors() {
    let err = CardDefinitionBuilder::new(CardId::from_raw(1), "First Spell Cost Probe")
        .card_types(vec![CardType::Enchantment])
        .parse_text("The first creature spell you cast each turn costs {2} less to cast.")
        .expect_err("first-spell cost marker should fail parsing");
    let message = format!("{err:?}").to_ascii_lowercase();
    assert!(
        message.contains("unsupported first-spell cost modifier mechanic"),
        "expected explicit unsupported first-spell marker error, got {message}"
    );
}

#[test]
fn test_parse_other_anthem_subject_keeps_other() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Other Anthem Probe")
        .card_types(vec![CardType::Creature])
        .parse_text("Other Soldier creatures you control get +0/+1")
        .expect("parse other-anthem line");

    let rendered = compiled_lines(&def).join(" | ").to_ascii_lowercase();
    assert!(
        rendered.contains("other soldier creatures you control get +0/+1"),
        "expected other-anthem text in compiled output, got {rendered}"
    );
}

#[test]
fn test_parse_other_anthem_subject_rejects_temporary() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Other Anthem Reject Probe")
        .card_types(vec![CardType::Creature])
        .parse_text("Other Soldier creatures you control get +0/+1 until end of turn")
        .expect("parse temporary other-anthem line");
    assert!(
        def.abilities.is_empty(),
        "expected temporary other-anthem line to avoid static abilities, got {:?}",
        def.abilities
    );
    assert!(
        def.spell_effect.is_some(),
        "expected temporary other-anthem line to compile as a spell effect"
    );
}

#[test]
fn test_peek_targets_player_hand() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Peek Probe")
        .card_types(vec![CardType::Instant])
        .parse_text("Look at target player's hand.\nDraw a card.")
        .expect("parse peek probe");

    let rendered = compiled_lines(&def).join(" | ").to_ascii_lowercase();
    assert!(
        rendered.contains("look at target player's hand"),
        "expected target player wording in compiled output, got {rendered}"
    );
}

#[test]
fn test_peek_targets_opponent_hand() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Opponent Peek Probe")
        .card_types(vec![CardType::Creature])
        .parse_text("When this creature enters, look at target opponent's hand.")
        .expect("parse look-at-opponent-hand trigger");

    let rendered = compiled_lines(&def).join(" | ").to_ascii_lowercase();
    assert!(
        rendered.contains("look at target opponent's hand"),
        "expected target opponent wording in compiled output, got {rendered}"
    );
}

#[test]
fn test_untap_another_target_permanent_rendering() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Untap Probe")
        .card_types(vec![CardType::Creature])
        .parse_text("{T}: Untap another target permanent.")
        .expect("parse untap probe");

    let rendered = compiled_lines(&def).join(" | ").to_ascii_lowercase();
    assert!(
        rendered.contains("another target permanent"),
        "expected 'another target permanent' in compiled output, got {rendered}"
    );
}

#[test]
fn test_counter_unless_pays_rendering() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Counter Probe")
        .card_types(vec![CardType::Instant])
        .parse_text("Counter target spell unless its controller pays {1}.")
        .expect("parse counter probe");

    let rendered = compiled_lines(&def).join(" | ").to_ascii_lowercase();
    assert!(
        rendered.contains("counter target spell unless its controller pays {1}"),
        "expected counter-unless-pays text in compiled output, got {rendered}"
    );
}

#[test]
fn test_counter_unless_pays_and_life_rendering() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Mundungu Probe")
        .card_types(vec![CardType::Creature])
        .parse_text("{T}: Counter target spell unless its controller pays {1} and 1 life.")
        .expect("parse counter-unless-pay-and-life probe");

    let rendered = compiled_lines(&def).join(" | ").to_ascii_lowercase();
    assert!(
        rendered.contains("counter target spell unless its controller pays {1} and 1 life"),
        "expected counter-unless-pay-and-life text in compiled output, got {rendered}"
    );
}

#[test]
fn test_counter_unless_pays_domain_rendering() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Evasive Action Probe")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Counter target spell unless its controller pays {1} for each basic land type among lands you control.",
        )
        .expect("parse domain counter-unless-pay probe");

    let rendered = compiled_lines(&def).join(" | ").to_ascii_lowercase();
    assert!(
        rendered.contains(
            "counter target spell unless its controller pays {1} for each basic land type among lands you control"
        ),
        "expected domain counter-unless-pay text in compiled output, got {rendered}"
    );
}

#[test]
fn test_return_target_permanent_you_both_own_and_control_rendering() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Obelisk Probe")
        .card_types(vec![CardType::Artifact])
        .parse_text("{6}, {T}: Return target permanent you both own and control to your hand.")
        .expect("parse return-own-control probe");

    let rendered = compiled_lines(&def).join(" | ").to_ascii_lowercase();
    assert!(
        rendered.contains("return target permanent you both own and control to your hand"),
        "expected own/control target restriction in compiled output, got {rendered}"
    );
}

#[test]
fn test_power_damage_exchange_rendering() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Power Exchange Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "{T}: This creature deals damage equal to its power to target creature. \
That creature deals damage equal to its power to this creature.",
        )
        .expect("parse power exchange probe");

    let rendered = compiled_lines(&def).join(" | ").to_ascii_lowercase();
    assert!(
        rendered.contains("this creature deals damage equal to its power to target creature"),
        "expected first power damage clause, got {rendered}"
    );
    assert!(
        rendered.contains("that creature deals damage equal to its power to this creature"),
        "expected reciprocal power damage clause, got {rendered}"
    );
}

#[test]
fn test_prevent_all_combat_damage_from_target_rendering() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Prevent Combat Probe")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "{2}{W}: Prevent all combat damage that would be dealt by target creature this turn.",
        )
        .expect("parse prevent combat probe");

    let rendered = compiled_lines(&def).join(" | ").to_ascii_lowercase();
    assert!(
        rendered
            .contains("prevent all combat damage that would be dealt by target creature this turn"),
        "expected prevent combat damage text, got {rendered}"
    );
}

#[test]
fn test_parse_static_prevent_all_combat_damage_to_this_creature_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Everdawn Champion Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Prevent all combat damage that would be dealt to this creature.")
        .expect("parse static prevent-all-combat-damage to this creature");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        ids.contains(&StaticAbilityId::PreventAllCombatDamageToSelf),
        "expected PreventAllCombatDamageToSelf ability id, got {ids:?}"
    );

    let rendered = oracle_like_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("prevent all combat damage that would be dealt to this creature"),
        "expected static prevent-all-combat-damage text, got {rendered}"
    );
}

#[test]
fn test_parse_modal_choose_one_that_hasnt_been_chosen_sets_mode_memory() {
    let oracle = "{2}, {T}: Choose one that hasn't been chosen —\n\
• This artifact deals 2 damage to target creature.\n\
• Tap target creature.\n\
• Sacrifice this artifact. You gain 3 life.";
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Three Bowls Probe")
        .card_types(vec![CardType::Artifact])
        .parse_text(oracle)
        .expect("parse choose-one-that-hasnt-been-chosen modal ability");

    let abilities_debug = format!("{:#?}", def.abilities);
    assert!(
        abilities_debug.contains("disallow_previously_chosen_modes: true"),
        "expected modal memory flag in compiled ability, got {abilities_debug}"
    );

    let rendered = compiled_lines(&def).join("\n").to_ascii_lowercase();
    assert!(
        rendered.contains("choose one that hasn't been chosen"),
        "expected modal heading to keep unchosen-mode clause, got {rendered}"
    );
}

#[test]
fn test_parse_modal_choose_one_that_hasnt_been_chosen_this_turn_sets_turn_scope() {
    let oracle = "Whenever another creature you control enters, choose one that hasn't been chosen this turn —\n\
• Put a +1/+1 counter on this creature.\n\
• Create a tapped Treasure token.\n\
• You gain 2 life.";
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Gala Greeters Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(oracle)
        .expect("parse this-turn choose-one-that-hasnt-been-chosen trigger");

    let abilities_debug = format!("{:#?}", def.abilities);
    assert!(
        abilities_debug.contains("disallow_previously_chosen_modes_this_turn: true"),
        "expected per-turn modal memory flag in compiled ability, got {abilities_debug}"
    );

    let rendered = compiled_lines(&def).join("\n").to_ascii_lowercase();
    assert!(
        rendered.contains("choose one that hasn't been chosen this turn"),
        "expected this-turn unchosen-mode clause in rendering, got {rendered}"
    );
}

#[test]
fn test_keyword_marker_rejects_partial_trailing_clause() {
    let err = CardDefinitionBuilder::new(CardId::from_raw(1), "Bad Unleash")
        .card_types(vec![CardType::Creature])
        .parse_text("Unleash while")
        .expect_err("trailing clause must not parse as standalone keyword");
    let message = format!("{err:?}").to_ascii_lowercase();
    assert!(
        message.contains("could not find verb in effect clause")
            || message.contains("unsupported line"),
        "expected strict parse failure for trailing keyword clause, got {message}"
    );
}

#[test]
fn test_parse_level_up_tiers_render_semantics() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Level-Up Tiers Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Level up {W} ({W}: Put a level counter on this. Level up only as a sorcery.)\n\
LEVEL 2-6\n\
3/3\n\
First strike\n\
LEVEL 7+\n\
4/4\n\
Double strike",
        )
        .expect("parse level up tier block");

    let joined = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains("level 2-6") && joined.contains("first strike"),
        "expected rendered level-2 tier details, got {joined}"
    );
    assert!(
        joined.contains("level 7+") && joined.contains("double strike"),
        "expected rendered level-7 tier details, got {joined}"
    );
}

#[test]
fn test_standalone_may_effect_does_not_emit_with_id_wrapper() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Slayer Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "When this creature enters, you may destroy target Vampire, Werewolf, or Zombie.",
        )
        .expect("parse slayer-like triggered line");

    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("expected triggered ability");

    assert_eq!(triggered.choices.len(), 1, "expected one target choice");

    let debug = format!("{:?}", triggered.effects);
    assert!(
        debug.contains("MayEffect"),
        "expected optional may wrapper, got {debug}"
    );
    assert!(
        debug.contains("DestroyEffect"),
        "expected destroy effect, got {debug}"
    );
    assert!(
        !debug.contains("WithIdEffect"),
        "standalone may should not be wrapped with WithId, got {debug}"
    );
}

#[test]
fn test_if_you_do_still_wraps_antecedent_with_with_id() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "If You Do Probe")
        .card_types(vec![CardType::Creature])
        .parse_text("When this creature enters, you may draw a card. If you do, discard a card.")
        .expect("parse if-you-do triggered line");

    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("expected triggered ability");

    assert_eq!(
        triggered.effects.len(),
        2,
        "expected antecedent and if-you-do follow-up"
    );

    let first_debug = format!("{:?}", triggered.effects[0]);
    assert!(
        first_debug.contains("WithIdEffect"),
        "if-you-do antecedent must store result id, got {first_debug}"
    );
    assert!(
        first_debug.contains("MayEffect"),
        "if-you-do antecedent should stay optional, got {first_debug}"
    );

    let second_debug = format!("{:?}", triggered.effects[1]);
    assert!(
        second_debug.contains("IfEffect"),
        "if-you-do follow-up must compile to IfEffect, got {second_debug}"
    );
}

#[test]
fn test_each_player_who_did_this_way_compiles_to_per_player_if_result() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Kwain Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("{T}: Each player may draw a card, then each player who drew a card this way gains 1 life.")
        .expect("parse each-player-who-did-this-way activated line");

    let activated = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("expected activated ability");

    let debug = format!("{:?}", activated.effects);
    assert!(
        debug.contains("ForPlayersEffect"),
        "expected per-player iteration wrapper, got {debug}"
    );
    assert!(
        debug.contains("WithIdEffect") && debug.contains("MayEffect"),
        "expected optional antecedent to be tracked per player, got {debug}"
    );
    assert!(
        debug.contains("IfEffect") && debug.contains("GainLifeEffect"),
        "expected per-player follow-up gain-life conditional, got {debug}"
    );
}

#[test]
fn test_for_each_opponent_who_does_merges_into_per_opponent_if_result() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Tempting Contract Variant")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "At the beginning of your upkeep, each opponent may create a Treasure token. For each opponent who does, you create a Treasure token.",
        )
        .expect("parse each-opponent-who-does trigger");

    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("expected triggered ability");

    let debug = format!("{:?}", triggered.effects);
    let for_players_count = debug.matches("ForPlayersEffect").count();
    assert_eq!(
        for_players_count, 1,
        "expected merged per-opponent wrapper, got {debug}"
    );
    assert!(
        debug.contains("IfEffect"),
        "expected merged follow-up to compile as IfEffect, got {debug}"
    );
    assert!(
        debug.contains("controller: IteratedPlayer"),
        "expected optional antecedent token controller to remain per-opponent, got {debug}"
    );
    assert!(
        debug.contains("controller: You"),
        "expected follow-up token creation to stay on you, got {debug}"
    );
}

#[test]
fn test_for_each_opponent_who_does_binds_implicit_followup_to_you() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Tempting Offer Variant")
        .parse_text(
            "Each opponent may create a Treasure token. For each opponent who does, create a Treasure token.",
        )
        .expect("parse each-opponent-who-does implicit follow-up");

    let spell_effects = def.spell_effect.as_ref().expect("spell effects");
    let debug = format!("{:?}", spell_effects);
    assert!(
        debug.contains("ForPlayersEffect"),
        "expected per-opponent wrapper, got {debug}"
    );
    assert!(
        debug.contains("IfEffect"),
        "expected follow-up to compile as IfEffect, got {debug}"
    );
    assert!(
        debug.contains("controller: You"),
        "expected implicit follow-up token creation to bind to you, got {debug}"
    );
}

#[test]
fn test_each_player_tagged_followups_collapse_into_single_for_players_effect() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Duskmantle Seer Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("At the beginning of your upkeep, each player reveals the top card of their library, loses life equal to that card's mana value, then puts it into their hand.")
        .expect("parse each-player tagged followups trigger");

    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("expected triggered ability");

    let debug = format!("{:?}", triggered.effects);
    assert_eq!(
        debug.matches("ForPlayersEffect").count(),
        1,
        "expected a single per-player wrapper for tagged followups, got {debug}"
    );
    assert!(
        debug.contains("RevealTopEffect"),
        "expected reveal-top effect in per-player wrapper, got {debug}"
    );
    assert!(
        debug.contains("LoseLifeEffect"),
        "expected lose-life effect in per-player wrapper, got {debug}"
    );
    assert!(
        debug.contains("MoveToZoneEffect") && debug.contains("zone: Hand"),
        "expected move-to-hand effect in per-player wrapper, got {debug}"
    );
}

#[test]
fn test_parse_trigger_without_comma() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "No Comma Trigger")
        .card_types(vec![CardType::Enchantment])
        .parse_text("At the beginning of the next end step draw a card.")
        .expect("parse trigger without comma");

    let has_triggered = def
        .abilities
        .iter()
        .any(|a| matches!(a.kind, AbilityKind::Triggered(_)));
    assert!(has_triggered, "expected triggered ability");
}

#[test]
fn test_parse_trigger_when_this_creature_is_turned_face_up() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Face-Up Trigger Probe")
        .card_types(vec![CardType::Creature])
        .parse_text("Morph {2}{U}\nWhen this creature is turned face up, draw a card.")
        .expect("parse turned-face-up trigger line");

    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("expected triggered ability");

    let effects_debug = format!("{:?}", triggered.effects);
    assert!(
        effects_debug.contains("DrawCardsEffect"),
        "expected draw effect from turned-face-up trigger, got {effects_debug}"
    );

    let compiled = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        compiled.contains("turned face up"),
        "expected turned-face-up text in compiled output, got {compiled}"
    );
}

#[test]
fn test_parse_trigger_when_face_down_permanent_is_turned_face_up() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Sumala Trigger Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Whenever a face-down permanent you control is turned face up, put a +1/+1 counter on it and a +1/+1 counter on this creature.",
        )
        .expect("parse filtered turned-face-up trigger line");

    let abilities_debug = format!("{:#?}", def.abilities);
    assert!(
        abilities_debug.contains("PermanentTurnedFaceUpTrigger"),
        "expected filtered turned-face-up trigger matcher, got {abilities_debug}"
    );
    assert!(
        !abilities_debug.contains("unimplemented_trigger"),
        "expected no custom-trigger fallback for turned-face-up filter, got {abilities_debug}"
    );

    let compiled = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        compiled.contains("whenever a face-down permanent you control is turned face up"),
        "expected turned-face-up trigger text to preserve face-down filter, got {compiled}"
    );
}

#[test]
fn test_parse_trigger_this_creature_enters_from_your_graveyard() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Phyrexian Dragon Engine")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "When this creature enters from your graveyard, you may discard your hand. If you do, draw three cards.",
        )
        .expect("parse enters-from-your-graveyard trigger");

    let debug = format!("{:#?}", def.abilities);
    assert!(
        debug.contains("from: Specific(\n                                Graveyard")
            || debug.contains("from: Specific(Graveyard)"),
        "expected trigger origin zone to be graveyard, got {debug}"
    );
}

#[test]
fn test_parse_composed_anthems_keep_independent_land_conditions() {
    let err = CardDefinitionBuilder::new(CardId::from_raw(1), "Tek Variant")
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .parse_text(
            "This creature gets +0/+2 as long as you control a Plains, has flying as long as you control an Island, gets +2/+0 as long as you control a Swamp, has first strike as long as you control a Mountain, and has trample as long as you control a Forest.",
        )
        .expect_err("composed anthems are currently unsupported");
    let message = format!("{err:?}");
    assert!(
        message.contains("multiple anthem conditions are not supported"),
        "expected explicit unsupported composed-anthem error, got {message}"
    );
}

#[test]
fn test_parse_granted_keyword_and_must_attack_keeps_both_parts() {
    let err = CardDefinitionBuilder::new(CardId::from_raw(1), "Hellraiser Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Creatures you control have haste and attack each combat if able.")
        .expect_err("granted keyword + must-attack is currently unsupported");
    let message = format!("{err:?}");
    assert!(
        message.contains("unsupported anthem subject"),
        "expected explicit unsupported anthem-subject error, got {message}"
    );
}

#[test]
fn parse_anger_graveyard_condition_with_land_control() {
    use crate::zone::Zone;

    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Anger Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Haste\nAs long as this card is in your graveyard and you control a Mountain, creatures you control have haste.",
        )
        .expect("anger-style graveyard + land-control condition should parse");

    let grant_from_graveyard = def.abilities.iter().any(|ability| {
        matches!(
            &ability.kind,
            AbilityKind::Static(static_ability)
                if static_ability.id() == StaticAbilityId::GrantAbility
        ) && ability.functional_zones.contains(&Zone::Graveyard)
            && !ability.functional_zones.contains(&Zone::Battlefield)
    });
    assert!(
        grant_from_graveyard,
        "expected anger-style grant ability to function from graveyard, got {:?}",
        def.abilities
    );
}

#[test]
fn test_parse_landwalk_as_though_clause_is_not_partially_parsed() {
    let err = CardDefinitionBuilder::new(CardId::from_raw(1), "Landwalk Override Variant")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "Creatures with islandwalk can be blocked as though they didn't have islandwalk.",
        )
        .expect_err("landwalk as-though clause should not partially parse");
    let message = format!("{err:?}");
    assert!(
        message.contains("could not find verb in effect clause") || message.contains("unsupported"),
        "expected actionable parse failure, got {message}"
    );
}

#[test]
fn test_parse_exile_up_to_one_single_disjunction_stays_single_choice() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Scrollshift Variant")
            .parse_text(
                "Exile up to one target artifact, creature, or enchantment you control, then return it to the battlefield under its owner's control.",
            )
            .expect("parse single-disjunction exile");

    let effects = def.spell_effect.as_ref().expect("spell effects");
    let debug = format!("{effects:?}");
    let choose_count = debug.matches("ChooseObjectsEffect").count();
    assert!(
        choose_count <= 1,
        "single disjunctive target should not fan out into per-type choices, got {choose_count} in {debug}"
    );
    assert!(
        debug.contains("ExileEffect") && debug.contains("MoveToZoneEffect"),
        "expected exile-then-return sequence, got {debug}"
    );
    assert!(
        debug.contains("card_types: [Artifact, Creature, Enchantment]")
            || debug.contains("card_types: [Artifact, Enchantment, Creature]")
            || debug.contains("card_types: [Creature, Artifact, Enchantment]")
            || debug.contains("card_types: [Creature, Enchantment, Artifact]")
            || debug.contains("card_types: [Enchantment, Artifact, Creature]")
            || debug.contains("card_types: [Enchantment, Creature, Artifact]"),
        "expected combined disjunctive type filter, got {debug}"
    );
}

#[test]
fn test_parse_exile_then_return_with_counter_keeps_counter_followup() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Planar Incision Variant")
        .parse_text(
            "Exile target artifact or creature, then return it to the battlefield under its owner's control with a +1/+1 counter on it.",
        )
        .expect("parse exile-then-return with counter");

    let effects = def.spell_effect.as_ref().expect("spell effects");
    let debug = format!("{effects:?}");
    assert!(
        debug.contains("MoveToZoneEffect"),
        "expected return move-to-battlefield effect, got {debug}"
    );
    assert!(
        debug.contains("PutCountersEffect") && debug.contains("PlusOnePlusOne"),
        "expected +1/+1 counter follow-up on returned object, got {debug}"
    );
}

#[test]
fn test_parse_shares_permanent_type_with_it_adds_tagged_constraint() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Cloudstone Curio Variant")
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "Whenever a nonartifact permanent you control enters, you may return another permanent you control that shares a permanent type with it to its owner's hand.",
        )
        .expect("parse shares-permanent-type clause");

    let debug = format!("{:#?}", def.abilities);
    assert!(
        debug.contains("SharesCardType"),
        "expected tagged shares-card-type constraint for 'shares a permanent type with it', got {debug}"
    );

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("shares a permanent type"),
        "expected rendered share-type restriction, got {rendered}"
    );
}

#[test]
fn test_parse_unblocked_attacking_filter_sets_unblocked() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Throatseeker Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Unblocked attacking Ninjas you control have lifelink.")
        .expect("parse unblocked-attacking static filter");

    let debug = format!("{:#?}", def.abilities);
    assert!(
        debug.contains("attacking: true"),
        "expected attacking filter flag, got {debug}"
    );
    assert!(
        debug.contains("unblocked: true"),
        "expected unblocked to map to unblocked filter flag, got {debug}"
    );
}

#[test]
fn test_parse_blocked_filter_sets_blocked() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Blocked Variant")
        .card_types(vec![CardType::Instant])
        .parse_text("Target blocked creature gains lifelink until end of turn.")
        .expect("parse blocked target filter");

    let debug = format!("{:#?}", def.spell_effect);
    assert!(
        debug.contains("blocked: true"),
        "expected blocked filter flag, got {debug}"
    );
}

#[test]
fn test_parse_lesser_mana_value_adds_tagged_lt_constraint() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Orah Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Whenever this creature or another Cleric you control dies, return target Cleric card with lesser mana value from your graveyard to the battlefield.",
        )
        .expect("parse lesser-mana-value tagged comparison");

    let debug = format!("{:#?}", def.abilities);
    assert!(
        debug.contains("ManaValueLtTagged"),
        "expected lesser mana value relation against tagged object, got {debug}"
    );
}

#[test]
fn test_parse_this_or_another_creature_dies_is_not_this_dies_only() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Blood Artist Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Whenever this creature or another creature dies, target player loses 1 life and you gain 1 life.",
        )
        .expect("parse this-or-another creature dies trigger");

    let trigger_debug = match &def.abilities[0].kind {
        AbilityKind::Triggered(triggered) => format!("{:#?}", triggered.trigger),
        _ => panic!("expected triggered ability"),
    };

    assert!(
        trigger_debug.contains("this_object: false"),
        "expected global creature-dies trigger (not this-only), got {trigger_debug}"
    );
}

#[test]
fn test_parse_equal_or_lesser_mana_value_adds_tagged_lte_constraint() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Jailbreak Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Return target permanent card in an opponent's graveyard to the battlefield under their control. When that permanent enters, return up to one target permanent card with equal or lesser mana value from your graveyard to the battlefield.",
        )
        .expect("parse equal-or-lesser mana value tagged comparison");

    let debug = format!("{:#?}", def.spell_effect);
    assert!(
        debug.contains("ManaValueLteTagged"),
        "expected equal-or-lesser mana value relation against tagged object, got {debug}"
    );
}

#[test]
fn test_render_multiple_cycling_variants_preserves_variant_names() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Cycling Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Mountaincycling {2}, forestcycling {2}.")
        .expect("parse multiple cycling variants");

    let lines = compiled_lines(&def);
    assert!(
        lines.iter().any(|line| line.contains("Mountaincycling")),
        "expected mountaincycling keyword in render, got {lines:?}"
    );
    assert!(
        lines.iter().any(|line| line.contains("Forestcycling")),
        "expected forestcycling keyword in render, got {lines:?}"
    );
}

#[test]
fn test_render_multiple_cycling_variants_with_reminder_text() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Cycling Variant Reminder")
        .card_types(vec![CardType::Creature])
        .parse_text("Mountaincycling {2}, forestcycling {2} ({2}, Discard this card: Search your library for a Mountain or Forest card, reveal it, put it into your hand, then shuffle.)")
        .expect("parse cycling variants with reminder");

    let lines = compiled_lines(&def);
    assert!(
        lines
            .iter()
            .any(|line| line.contains("Mountaincycling {2}")),
        "expected mountaincycling keyword in render, got {lines:?}"
    );
    assert!(
        lines.iter().any(|line| line.contains("Forestcycling {2}")),
        "expected forestcycling keyword in render, got {lines:?}"
    );
}

#[test]
fn test_parse_multiple_cycling_variants_merges_search_filter_subtypes() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Cycling Variant Filter")
        .card_types(vec![CardType::Creature])
        .parse_text("Mountaincycling {2}, forestcycling {2} ({2}, Discard this card: Search your library for a Mountain or Forest card, reveal it, put it into your hand, then shuffle.)")
        .expect("parse cycling variants with reminder");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("subtypes: [Mountain, Forest]")
            || debug.contains("subtypes: [Forest, Mountain]"),
        "expected merged mountain/forest cycling search filter, got {debug}"
    );
}

#[test]
fn test_render_cycling_includes_cost() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Cycling Cost Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Cycling {2}{U} ({2}{U}, Discard this card: Draw a card.)")
        .expect("parse cycling with cost");

    let lines = compiled_lines(&def);
    assert!(
        lines.iter().any(|line| line.contains("Cycling {2}{U}")),
        "expected cycling cost in render, got {lines:?}"
    );
}

#[test]
fn test_render_basic_landcycling_as_keyword_ability() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Basic Landcycling Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Target player loses 4 life and you gain 4 life.\nBasic landcycling {1}{B} ({1}{B}, Discard this card: Search your library for a basic land card, reveal it, put it into your hand, then shuffle.)",
        )
        .expect("parse basic landcycling line");

    let lines = compiled_lines(&def);
    assert!(
        lines
            .iter()
            .any(|line| line.contains("Basic landcycling {1}{B}")),
        "expected basic landcycling keyword in render, got {lines:?}"
    );
}

#[test]
fn test_parse_cycling_pay_life_keeps_keyword_ability() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Street Wraith Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Cycling—Pay 2 life. ({2}, Discard this card: Draw a card.)")
        .expect("parse life-cycling line");

    let rendered = compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Cycling—Pay 2 life"),
        "expected rendered life-cycling keyword, got {rendered}"
    );

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("LoseLifeEffect")
            && debug.contains("DiscardEffect")
            && debug.contains("EmitKeywordActionEffect")
            && debug.contains("DrawCardsEffect"),
        "expected life-cycling to remain a discard+draw activated ability, got {debug}"
    );
}

#[test]
fn test_parse_cycle_this_card_trigger_compiles() {
    use crate::zone::Zone;

    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Cycling Trigger Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Cycling {2}.\nWhenever you cycle this card, draw a card.")
        .expect("parse cycling trigger variant");

    let has_trigger = def.abilities.iter().any(|ability| {
        matches!(
            &ability.kind,
            AbilityKind::Triggered(t) if t.trigger.display() == "Whenever you cycle this card"
        ) && ability.functional_zones.contains(&Zone::Graveyard)
    });
    assert!(
        has_trigger,
        "expected source-specific cycling trigger that functions in graveyard, got {:?}",
        def.abilities
    );
}

#[test]
fn test_commander_recursion_trigger_uses_graveyard_zone_and_commander_filter() {
    use crate::zone::Zone;

    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Commander Recursion Variant")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Whenever your commander enters or attacks, you may pay {2}. If you do, return this card from your graveyard to your hand.",
        )
        .expect("parse commander recursion trigger");

    let ability = def
        .abilities
        .iter()
        .find(|ability| matches!(&ability.kind, AbilityKind::Triggered(_)))
        .expect("expected triggered ability");

    assert!(
        ability.functional_zones.contains(&Zone::Graveyard)
            && !ability.functional_zones.contains(&Zone::Battlefield),
        "expected trigger to function from graveyard only, got {:?}",
        ability.functional_zones
    );

    let trigger_debug = match &ability.kind {
        AbilityKind::Triggered(triggered) => format!("{:?}", triggered.trigger),
        _ => unreachable!("checked triggered ability above"),
    };
    assert!(
        trigger_debug.contains("AttacksTrigger") && !trigger_debug.contains("ThisAttacksTrigger"),
        "expected shared-subject attack branch, got {trigger_debug}"
    );
    let compact = trigger_debug.split_whitespace().collect::<String>();
    assert!(
        compact.contains("is_commander:true") && compact.contains("owner:Some(You"),
        "expected your-commander ownership filter on both branches, got {trigger_debug}"
    );
}

#[test]
fn test_parse_bridge_from_below_compiles_graveyard_triggers() {
    use crate::zone::Zone;

    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Bridge from Below")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "Whenever a nontoken creature is put into your graveyard from the battlefield, if this card is in your graveyard, create a 2/2 black Zombie creature token.\nWhen a creature is put into an opponent's graveyard from the battlefield, if this card is in your graveyard, exile this card.",
        )
        .expect("Bridge from Below should parse without unsupported fallback");

    assert_eq!(def.abilities.len(), 2, "expected two triggered abilities");
    assert!(
        def.abilities.iter().all(|ability| {
            matches!(&ability.kind, AbilityKind::Triggered(_))
                && ability.functional_zones == vec![Zone::Graveyard]
        }),
        "expected both Bridge triggers to function from graveyard, got {:?}",
        def.abilities
    );

    let triggers = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(triggers.len(), 2, "expected two triggered abilities");
    assert_eq!(
        triggers[0].intervening_if,
        Some(crate::ConditionExpr::SourceIsInZone(Zone::Graveyard)),
        "expected Bridge token trigger to recheck that the source is still in the graveyard"
    );
    let first_trigger_debug = format!("{:?}", triggers[0].trigger);
    assert!(
        first_trigger_debug.contains("from: Specific(Battlefield)")
            && first_trigger_debug.contains("owner: Some(You)")
            && first_trigger_debug.contains("nontoken: true"),
        "expected first Bridge trigger to watch your creature dying from the battlefield, got {first_trigger_debug}"
    );

    let rendered = compiled_lines(&def).join("\n").to_ascii_lowercase();
    assert!(
        rendered.contains("create a 2/2 black zombie creature token")
            && rendered.contains("exile this")
            && rendered.contains("an opponent")
            && rendered.contains("dies"),
        "expected both Bridge abilities in compiled text, got {rendered}"
    );
}

#[test]
fn test_return_from_graveyard_keeps_with_cycling_filter() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Sacred Excavation Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Return up to two target cards with cycling from your graveyard to your hand.")
        .expect("parse return with cycling filter");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("with cycling from your graveyard"),
        "expected rendered target filter to keep with-cycling qualifier, got {rendered}"
    );
}

#[test]
fn parse_same_name_destroy_fans_out_to_all_other_matching_objects() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Same Name Destroy Variant")
        .parse_text(
            "Destroy target artifact and all other artifacts with the same name as that artifact.",
        )
        .expect("parse same-name destroy sentence");

    let effects = def.spell_effect.expect("expected spell effects");
    let debug = format!("{effects:?}");
    assert!(
        debug.contains("DestroyEffect") && debug.matches("DestroyEffect").count() >= 2,
        "expected target destroy plus fanout destroy, got {debug}"
    );
    assert!(
        debug.contains("SameNameAsTagged"),
        "expected same-name tagged relation in fanout filter, got {debug}"
    );
    assert!(
        debug.contains("IsNotTaggedObject"),
        "expected all-other exclusion relation in fanout filter, got {debug}"
    );
}

#[test]
fn parse_same_name_exile_with_that_player_controls_keeps_controller_link() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Same Name Exile Variant")
            .parse_text(
                "Exile target creature an opponent controls with mana value 2 or less and all other creatures that player controls with the same name as that creature.",
            )
            .expect("parse same-name exile sentence");

    let effects = def.spell_effect.expect("expected spell effects");
    let debug = format!("{effects:?}");
    assert!(
        debug.contains("MoveToZoneEffect") && debug.contains("ExileEffect"),
        "expected target exile plus fanout exile-all effect, got {debug}"
    );
    assert!(
        debug.contains("SameNameAsTagged"),
        "expected same-name tagged relation in fanout filter, got {debug}"
    );
    assert!(
        debug.contains("SameControllerAsTagged"),
        "expected same-controller tagged relation in fanout filter, got {debug}"
    );
}

#[test]
fn parse_legions_end_style_reveal_and_exile_keeps_same_name_hand_graveyard_bundle() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Legion's End Variant")
        .parse_text(
            "Exile target creature an opponent controls with mana value 2 or less and all other creatures that player controls with the same name as that creature. Then that player reveals their hand and exiles all cards with that name from their hand and graveyard.",
        )
        .expect("parse legion's end style reveal+exile sentence");

    let debug = format!("{:?}", def.spell_effect).to_ascii_lowercase();
    assert!(
        debug.contains("lookathandeffect"),
        "expected reveal-hand effect for that player, got {debug}"
    );
    assert!(
        debug.contains("samenameastagged"),
        "expected same-name tagged relation in hand/graveyard exile filter, got {debug}"
    );
    assert!(
        debug.contains("zone: some(hand)") && debug.contains("zone: some(graveyard)"),
        "expected hand and graveyard zones in follow-up exile filter, got {debug}"
    );
    assert!(
        debug.contains("controllerof"),
        "expected 'that player' to resolve through controller-of tagged context, got {debug}"
    );
}

#[test]
fn parse_same_name_fanout_requires_full_reference_tail() {
    let err = CardDefinitionBuilder::new(CardId::from_raw(1), "Broken Same Name Variant")
        .parse_text("Destroy target artifact and all other artifacts with the same name as.")
        .expect_err("same-name clause without full tail should fail");
    let message = format!("{err:?}");
    assert!(
        message.contains("same-name"),
        "expected actionable same-name parse error, got {message}"
    );
}

#[test]
fn parse_same_name_target_gets_fans_out_to_tagged_filter() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Same Name Gets Variant")
            .parse_text(
                "Target creature and all other creatures with the same name as that creature get -3/-3 until end of turn.",
            )
            .expect("parse same-name gets sentence");

    let effects = def.spell_effect.expect("expected spell effects");
    let debug = format!("{effects:?}");
    assert!(
        debug.matches("ApplyContinuousEffect").count() >= 2
            && debug.contains("runtime_modifications: [ModifyPowerToughness"),
        "expected target and fanout continuous runtime modifications, got {debug}"
    );
    assert!(
        debug.contains("SameNameAsTagged") && debug.contains("IsNotTaggedObject"),
        "expected same-name all-other relations in fanout filter, got {debug}"
    );
}

#[test]
fn parse_equipped_gets_and_has_activated_grant_as_static_abilities() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Equip Activated Grant Variant")
            .parse_text(
                "Equip {1}\nEquipped creature gets +0/+3 and has \"{2}, {T}: Target player mills three cards.\"",
            )
            .expect("parse equipped activated grant line");

    assert!(
        def.spell_effect.is_none(),
        "equipped activated grant must not compile as one-shot spell effects"
    );

    let mut has_anthem = false;
    let mut has_attached_grant = false;
    for ability in &def.abilities {
        if let AbilityKind::Static(static_ability) = &ability.kind {
            if static_ability.id() == crate::static_abilities::StaticAbilityId::Anthem {
                has_anthem = true;
            }
            if static_ability.id() == crate::static_abilities::StaticAbilityId::AttachedAbilityGrant
            {
                has_attached_grant = true;
            }
        }
    }
    assert!(has_anthem, "expected equipped anthem static ability");
    assert!(
        has_attached_grant,
        "expected attached activated-ability grant static ability"
    );
}

#[test]
fn parse_equipped_activated_grant_with_unsupported_cost_errors_instead_of_partial_compile() {
    let err = CardDefinitionBuilder::new(CardId::from_raw(1), "Equip Unsupported Grant Variant")
            .parse_text(
                "Equip {5}\nEquipped creature gets +2/+1 and has \"{T}, Unattach this source: Destroy target creature.\"",
            )
            .expect_err("unsupported equipped activated cost should fail");
    let message = format!("{err:?}");
    assert!(
        message.contains("unsupported equipped activated-ability grant")
            || message.contains("unsupported activation cost segment"),
        "expected actionable equipped-grant error, got {message}"
    );
}

#[test]
fn parse_equip_cost_reduction_line_does_not_silently_compile_as_equip_keyword() {
    let err = CardDefinitionBuilder::new(CardId::from_raw(1), "Equip Cost Reduction Variant")
        .parse_text("Equip costs you pay cost {1} less.")
        .expect_err("equip-cost-reduction line should not compile as keyword equip");
    let message = format!("{err:?}");
    assert!(
        !message.to_ascii_lowercase().contains("equip"),
        "expected non-equip parse error for unsupported equip-cost-reduction form, got {message}"
    );
}

#[test]
fn parse_flashback_cost_modifiers_render_with_controller_scope() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Catalyst Stone Variant")
        .parse_text(
            "Flashback costs you pay cost {2} less.\nFlashback costs your opponents pay cost {2} more.",
        )
        .expect_err("flashback cost-modifier lines are currently unsupported");
    let rendered = format!("{err:?}").to_ascii_lowercase();
    assert!(
        rendered.contains("unsupported activation cost segment"),
        "expected explicit unsupported flashback cost-modifier error, got {rendered}"
    );
}

#[test]
fn render_during_turn_flashback_grant_keeps_mana_cost_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Return the Past Variant")
        .parse_text(
            "During your turn, each instant and sorcery card in your graveyard has flashback. Its flashback cost is equal to its mana cost.",
        )
        .expect("during-turn flashback grant should parse");
    let rendered = compiled_lines(&def).join(" ");
    assert!(
        rendered
            .to_ascii_lowercase()
            .contains("flashback cost is equal to its mana cost"),
        "expected flashback-cost sentence in rendering, got {rendered}"
    );
}

#[test]
fn render_underworld_breach_escape_grant_keeps_nonland_and_cost_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Underworld Breach Variant")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "Each nonland card in your graveyard has escape. The escape cost is equal to the card's mana cost plus exile three other cards from your graveyard.",
        )
        .expect("underworld breach-style escape grant should parse");
    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("each nonland card in your graveyard has escape"),
        "expected nonland graveyard scope in rendering, got {rendered}"
    );
    assert!(
        rendered.contains(
            "escape cost is equal to the card's mana cost plus exile three other cards from your graveyard"
        ),
        "expected escape-cost sentence in rendering, got {rendered}"
    );
}

#[test]
fn render_gain_life_equal_to_its_power_uses_possessive_wording() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Infernal Reckoning Variant")
        .card_types(vec![CardType::Instant])
        .parse_text("Exile target colorless creature. You gain life equal to its power.")
        .expect("gain-life-equal-to-power line should parse");
    let rendered = compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("gain life equal to its power"),
        "expected possessive power wording, got {rendered}"
    );
}

#[test]
fn parse_gain_life_equal_to_sacrificed_creature_toughness() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Diamond Valley Variant")
        .card_types(vec![CardType::Land])
        .parse_text("{T}, Sacrifice a creature: You gain life equal to the sacrificed creature's toughness.")
        .expect("sacrificed-creature toughness life amount should parse");

    let abilities_debug = format!("{:#?}", def.abilities);
    assert!(
        abilities_debug.contains("GainLifeEffect") && abilities_debug.contains("ToughnessOf"),
        "expected gain-life amount to bind to sacrificed creature toughness, got {abilities_debug}"
    );
}

#[test]
fn parse_gain_life_equal_to_devotion_value() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Nylea Disciple Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text("You gain life equal to your devotion to green.")
        .expect("devotion-based life gain should parse");

    let spell_debug = format!("{:#?}", def.spell_effect);
    assert!(
        spell_debug.contains("GainLifeEffect") && spell_debug.contains("Devotion"),
        "expected devotion value in life-gain amount, got {spell_debug}"
    );
}

#[test]
fn parse_gain_life_equal_to_life_lost_this_way() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Agent of Masks Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Each opponent loses 1 life. You gain life equal to the life lost this way.")
        .expect("life-lost-this-way life gain should parse");

    let spell_debug = format!("{:#?}", def.spell_effect);
    assert!(
        spell_debug.contains("GainLifeEffect")
            && (spell_debug.contains("EventValue")
                || spell_debug.contains("EffectValue(")
                || spell_debug.contains("life lost this way")),
        "expected life-gain amount to use life-lost event value, got {spell_debug}"
    );
}

#[test]
fn render_artifact_land_self_reference_prefers_land() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Artifact Land Variant")
        .card_types(vec![CardType::Artifact, CardType::Land])
        .parse_text("This land enters tapped.")
        .expect("artifact land line should parse");
    let rendered = compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("This land enters tapped"),
        "expected land self-reference wording, got {rendered}"
    );
    assert!(
        !rendered.contains("This artifact enters tapped"),
        "artifact land should not render as artifact-only self-reference: {rendered}"
    );
}

#[test]
fn parse_mana_value_or_less_keeps_comparison_and_type_conjunction() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Technomancer Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "When this creature enters, mill three cards, then return any number of artifact creature cards with total mana value 6 or less from your graveyard to the battlefield.",
        )
        .expect("technomancer line should parse");
    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        !rendered.contains("artifact or creature card"),
        "type conjunction should not degrade to union wording: {rendered}"
    );
    assert!(
        !rendered.contains("mana value 6s"),
        "comparison tokenization should not pluralize numeric threshold: {rendered}"
    );
}

#[test]
fn render_exile_from_graveyard_uses_from_preposition() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Grave Robbers Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("{B}, {T}: Exile target artifact card from a graveyard. You gain 2 life.")
        .expect("graveyard exile clause should parse");
    let rendered = compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Exile target artifact card from a graveyard")
            || rendered.contains("Exile target artifact card in a graveyard"),
        "expected from-a-graveyard wording, got {rendered}"
    );
}

#[test]
fn render_granted_activated_ability_keeps_tap_symbol() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Brawl Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Until end of turn, all creatures gain \"{T}: This creature deals damage equal to its power to target creature.\"",
        )
        .expect("grant-tap-ability clause should parse");

    let rendered = compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("{T}: this creature deals damage equal to its power to target creature"),
        "expected granted tap ability to preserve tap symbol, got {rendered}"
    );
    assert!(
        !rendered.contains("gain t this creature deals"),
        "granted tap ability should not lose the tap symbol: {rendered}"
    );
}

#[test]
fn parse_lose_life_for_each_with_multiplier_uses_scaled_count_value() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Rain of Daggers Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Destroy all creatures target opponent controls. You lose 2 life for each creature destroyed this way.",
        )
        .expect("scaled for-each life loss should parse");
    let effects = def
        .spell_effect
        .as_ref()
        .expect("spell should have compiled effects");
    let lose = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::LoseLifeEffect>())
        .expect("expected lose-life effect");
    match &lose.amount {
        Value::CountScaled(filter, multiplier) => {
            assert_eq!(*multiplier, 2, "expected fixed multiplier of two");
            assert!(
                filter.card_types.contains(&CardType::Creature),
                "expected creature count filter, got {:?}",
                filter.card_types
            );
        }
        other => panic!("expected scaled count value, got {other:?}"),
    }
}

#[test]
fn render_tap_x_artifacts_creatures_and_lands_preserves_and_or_list() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Malicious Advice Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Tap X target artifacts, creatures, and/or lands. You lose X life.")
        .expect("mixed target-type tap line should parse");
    let rendered = compiled_lines(&def).join(" ");
    let lower = rendered.to_ascii_lowercase();
    assert!(
        lower.contains("x target artifacts, creatures, and/or lands"),
        "expected artifacts/creatures/lands and-or wording, got {rendered}"
    );
}

#[test]
fn parse_destroy_then_populate_requires_supported_followup_clause() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Sundering Growth Variant")
        .parse_text("Destroy target artifact or enchantment, then populate.")
        .expect_err("destroy-then-populate should fail until populate effect support exists");
    let message = format!("{err:?}");
    assert!(
        message.contains("populate") || message.contains("could not find verb"),
        "expected actionable populate parse error, got {message}"
    );
}

#[test]
fn parse_destroy_target_one_or_more_colors() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Reach of Shadows Variant")
        .parse_text("Destroy target creature that's one or more colors.")
        .expect("one-or-more-colors target should parse");
    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("destroy target colored creature")
            || rendered.contains("destroy target creature that's one or more colors"),
        "expected colored-target rendering, got {rendered}"
    );
}

#[test]
fn parse_destroy_target_three_or_more_colors_still_fails_loudly() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Reach of Shadows Negative Variant")
        .parse_text("Destroy target creature that's three or more colors.")
        .expect_err("unsupported three-or-more-colors target should fail loudly");
    let message = format!("{err:?}");
    assert!(
        message.contains("unsupported color-count object filter"),
        "expected color-count filter parse error, got {message}"
    );
}

#[test]
fn parse_ugin_colored_permanent_target_lines() {
    CardDefinitionBuilder::new(CardId::new(), "Ugin Variant")
        .card_types(vec![CardType::Planeswalker])
        .parse_text(
            "When you cast this spell, exile up to one target permanent that's one or more colors.\nWhenever you cast a colorless spell, exile up to one target permanent that's one or more colors.",
        )
        .expect("ugin colored-permanent target lines should parse");
}

#[test]
fn parse_protection_from_spells_that_are_one_or_more_colors() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Emrakul Protection Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Protection from spells that are one or more colors.")
        .expect("colored-spell protection line should parse");
    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("protection from colored spells")
            || rendered.contains("protection from spells that are one or more colors"),
        "expected colored-spell protection wording, got {rendered}"
    );
}

#[test]
fn parse_exile_face_down_manifest_tail_fails_instead_of_partial_exile() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Ghastly Conscription Variant")
        .parse_text(
            "Exile all creature cards from target player's graveyard in a face-down pile, shuffle that pile, then manifest those cards.",
        )
        .expect_err("face-down/manifest exile tail should fail loudly when unsupported");
    let message = format!("{err:?}");
    assert!(
        message.contains("unsupported face-down/manifest exile clause")
            || message.contains("unsupported face-down clause"),
        "expected actionable face-down/manifest parse error, got {message}"
    );
}

#[test]
fn parse_return_all_dealt_damage_this_turn_fails_instead_of_broadening() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Restore the Peace Variant")
        .parse_text("Return each creature that dealt damage this turn to its owner's hand.")
        .expect_err("qualified return-all filter should fail when unsupported");
    let message = format!("{err:?}");
    assert!(
        message.contains("unsupported qualified return-all filter"),
        "expected qualified return-all parse error, got {message}"
    );
}

#[test]
fn parse_return_all_without_counter_fails_instead_of_broadening() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Wave Goodbye Variant")
        .parse_text("Return each creature without a +1/+1 counter on it to its owner's hand.")
        .expect_err("without-counter return-all filter should fail when unsupported");
    let message = format!("{err:?}");
    assert!(
        message.contains("unsupported qualified return-all filter"),
        "expected qualified return-all parse error, got {message}"
    );
}

#[test]
fn parse_sacrifice_unless_clause_fails_instead_of_ignoring_unless_tail() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Pendrell Flux Variant")
        .parse_text(
            "Enchant creature\nEnchanted creature has \"At the beginning of your upkeep, sacrifice this creature unless you pay its mana cost.\"",
        )
        .expect_err("sacrifice-unless clauses should fail loudly when unsupported");
    let message = format!("{err:?}");
    assert!(
        message.contains("unsupported sacrifice-unless clause")
            || message.contains("unsupported empty attached triggered grant clause")
            || message.contains("unsupported empty granted triggered ability clause")
            || message.contains("unsupported unless-payment mana-cost clause")
            || message.contains("unsupported trailing unless-payment clause"),
        "expected sacrifice-unless parse error, got {message}"
    );
}

#[test]
fn parse_power_or_toughness_cant_be_blocked_subject_fails_loudly() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Tetsuko Variant")
        .parse_text("Creatures you control with power or toughness 1 or less can't be blocked.")
        .expect_err("power-or-toughness unblockable subject should fail when unsupported");
    let message = format!("{err:?}");
    assert!(
        message.contains("unsupported power-or-toughness cant-be-blocked subject"),
        "expected power-or-toughness subject parse error, got {message}"
    );
}

#[test]
fn parse_target_player_gain_then_draw_carries_target_player_to_draw_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Kiss of the Amesha Variant")
        .parse_text("Target player gains 7 life and draws two cards.")
        .expect("gain-then-draw line should parse");
    let joined = crate::compiled_text::compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("target player draws two cards"),
        "expected carried target player for draw clause, got {joined}"
    );
}

#[test]
fn parse_target_player_mill_draw_lose_chain_carries_target_player_to_draw_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Atrocious Experiment Variant")
        .parse_text("Target player mills two cards, draws two cards, and loses 2 life.")
        .expect("mill-draw-lose line should parse");
    let joined = crate::compiled_text::compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("target player draws two cards"),
        "expected carried target player for chained draw clause, got {joined}"
    );
}

#[test]
fn parse_target_player_mill_then_imperative_draw_does_not_carry_target_player() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Pilfered Plans Variant")
        .parse_text("Target player mills two cards. Draw two cards.")
        .expect("mill-then-draw line should parse");
    let joined = crate::compiled_text::compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        !joined.contains("target player draws two cards"),
        "imperative draw clause should not carry target player, got {joined}"
    );
}

#[test]
fn parse_defending_player_discard_then_draws_carries_defending_player() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Robber Fly Variant")
        .parse_text(
            "Whenever this creature becomes blocked, defending player discards all cards from their hand, then draws that many cards.",
        )
        .expect("defending-player discard-then-draw line should parse");
    let joined = crate::compiled_text::compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("defending player draws that many cards"),
        "expected defending player to carry into draws clause, got {joined}"
    );
}

#[test]
fn parse_target_opponent_sacrifice_discard_lose_chain_keeps_all_predicates() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Archon Chain Variant")
        .parse_text(
            "When this creature enters, target opponent sacrifices a creature of their choice, discards a card, and loses 3 life.",
        )
        .expect("target-opponent sacrifice/discard/lose chain should parse");
    let joined = crate::compiled_text::compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("target opponent sacrifices"),
        "expected sacrifice clause in chain, got {joined}"
    );
    assert!(
        joined.contains("discards a card"),
        "expected discard clause in chain, got {joined}"
    );
    assert!(
        joined.contains("loses 3 life"),
        "expected life-loss clause in chain, got {joined}"
    );
}

#[test]
fn parse_sacrifice_all_lands_clause_as_sacrifice_all() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Overlaid Terrain Variant")
        .parse_text("As this enchantment enters, sacrifice all lands you control.")
        .expect("sacrifice-all lands clause should parse");
    let compiled = crate::compiled_text::compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        compiled.contains("sacrifice all lands") || compiled.contains("sacrifices all lands"),
        "expected sacrifice-all lands wording, got {compiled}"
    );
}

#[test]
fn render_target_player_sacrifices_and_loses_uses_oracle_like_wording() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Geth's Verdict Render Variant")
        .parse_text("Target player sacrifices a creature of their choice and loses 1 life.")
        .expect("sacrifice-then-lose line should parse");
    let joined = crate::compiled_text::oracle_like_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("target player sacrifices a creature of their choice and loses 1 life"),
        "expected oracle-like sacrifice+lose wording, got {joined}"
    );
}

#[test]
fn parse_target_opponent_sacrifice_of_their_choice_keeps_non_targeted_object_choice() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Predatory Nightstalker Variant")
        .parse_text(
            "When this creature enters, target opponent sacrifices a creature of their choice.",
        )
        .expect("opponent-sacrifice-of-their-choice line should parse");
    let joined = crate::compiled_text::compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("target opponent sacrifices a creature"),
        "expected opponent sacrifice choice wording, got {joined}"
    );
    assert!(
        !joined.contains("target creature an opponent controls"),
        "sacrifice choice should not force target-creature wording, got {joined}"
    );
}

#[test]
fn parse_all_slivers_have_activated_ability_as_static_grant() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Sliver Activated Grant Variant")
        .parse_text("All Slivers have \"{2}, Sacrifice this permanent: Draw a card.\"")
        .expect("parse sliver activated grant line");

    assert!(
        def.spell_effect.is_none(),
        "sliver activated grant must not compile as one-shot spell effects"
    );
    let has_filter_grant = def.abilities.iter().any(|ability| {
        matches!(
            &ability.kind,
            AbilityKind::Static(static_ability)
                if static_ability.id()
                    == crate::static_abilities::StaticAbilityId::GrantObjectAbilityForFilter
        )
    });
    assert!(
        has_filter_grant,
        "expected filter-based object ability grant static ability, got {:?}",
        def.abilities
    );
}

#[test]
fn parse_all_slivers_have_triggered_ability_as_static_grant() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Sliver Triggered Grant Variant")
        .parse_text("All Slivers have \"When this permanent enters, draw a card.\"")
        .expect("parse sliver triggered grant line");

    assert!(
        def.spell_effect.is_none(),
        "sliver triggered grant must not compile as one-shot spell effects"
    );
    let has_filter_grant = def.abilities.iter().any(|ability| {
        matches!(
            &ability.kind,
            AbilityKind::Static(static_ability)
                if static_ability.id()
                    == crate::static_abilities::StaticAbilityId::GrantObjectAbilityForFilter
        )
    });
    assert!(
        has_filter_grant,
        "expected filter-based object ability grant static ability, got {:?}",
        def.abilities
    );
}

#[test]
fn parse_prevent_all_combat_damage_global_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Fog Variant")
        .parse_text("Prevent all combat damage that would be dealt this turn.")
        .expect("parse basic prevent-all combat clause");

    let effects = def.spell_effect.expect("expected spell effects");
    let debug = format!("{effects:?}");
    assert!(
        debug.contains("PreventAllDamageEffect"),
        "expected prevent-all damage runtime effect, got {debug}"
    );
    assert!(
        debug.contains("combat_only: true"),
        "expected combat-only prevention filter, got {debug}"
    );
}

#[test]
fn parse_prevent_all_combat_damage_by_target_source_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Targeted Fog Variant")
        .parse_text("Prevent all combat damage that would be dealt by target creature this turn.")
        .expect("parse source-scoped prevent-all combat clause");

    let effects = def.spell_effect.expect("expected spell effects");
    let debug = format!("{effects:?}");
    assert!(
        debug.contains("PreventAllCombatDamageFromEffect"),
        "expected source-scoped combat prevention runtime effect, got {debug}"
    );
    assert!(
        debug.contains("Target(") && debug.contains("Creature"),
        "expected target creature choice for source-scoped prevention, got {debug}"
    );
}

#[test]
fn parse_prevent_all_combat_damage_to_players_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Player Fog Variant")
        .parse_text("Prevent all combat damage that would be dealt to players this turn.")
        .expect("parse players-scoped prevent-all combat clause");

    let effects = def.spell_effect.expect("expected spell effects");
    let debug = format!("{effects:?}");
    assert!(
        debug.contains("target: Players"),
        "expected prevention target scope to players, got {debug}"
    );
    assert!(
        debug.contains("combat_only: true"),
        "expected combat-only prevention filter, got {debug}"
    );
}

#[test]
fn parse_prevent_all_combat_damage_requires_supported_tail() {
    let err = CardDefinitionBuilder::new(CardId::from_raw(1), "Unsupported Fog Tail Variant")
            .parse_text("Prevent all combat damage that would be dealt this turn by creatures with power 4 or less.")
            .expect_err("unsupported prevent-all tail must fail parse");
    let message = format!("{err:?}");
    assert!(
        message.contains("unsupported prevent-all-combat-damage clause tail")
            || message.contains("unsupported prevent-all source target"),
        "expected strict prevent-all tail error, got {message}"
    );
}

#[test]
fn parse_prevent_next_damage_to_any_target_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Amulet of Kroog Variant")
        .parse_text(
            "{2}, {T}: Prevent the next 1 damage that would be dealt to any target this turn.",
        )
        .expect("parse prevent-next damage clause");

    let lines = crate::compiled_text::compiled_lines(&def);
    let activated_line = lines
        .iter()
        .find(|line| line.contains("Activated ability"))
        .expect("expected activated ability line");
    assert!(
        activated_line
            .to_ascii_lowercase()
            .contains("prevent the next 1"),
        "expected prevent-next wording in compiled output, got {activated_line}"
    );

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("PreventDamageEffect"),
        "expected runtime prevent-damage effect, got {debug}"
    );
}

#[test]
fn parse_prevent_next_damage_rejects_trailing_tail_strictly() {
    let err = CardDefinitionBuilder::new(CardId::from_raw(1), "Prevent Tail Variant")
        .parse_text(
            "Prevent the next 1 damage that would be dealt to any target this turn by red sources.",
        )
        .expect_err("unsupported trailing prevent-next damage clause should fail parse");
    let message = format!("{err:?}");
    assert!(
        message.contains("unsupported trailing prevent-next damage clause"),
        "expected strict prevent-next tail parse error, got {message}"
    );
}

#[test]
fn parse_target_opponent_chooses_creature_then_other_cant_block() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Eunuchs Variant")
            .parse_text(
                "Target opponent chooses a creature they control. Other creatures they control can't block this turn.",
            )
            .expect("target-opponent choose + cant-block sequence should parse");

    let effects = def.spell_effect.expect("expected spell effects");
    let debug = format!("{effects:?}");
    assert!(
        debug.contains("ChooseObjectsEffect"),
        "expected choose-objects effect for chosen creature, got {debug}"
    );
    assert!(
        debug.contains("CantEffect") && debug.contains("Block("),
        "expected cant-block restriction effect, got {debug}"
    );
    assert!(
        debug.contains("IsNotTaggedObject"),
        "expected other-creatures exclusion via tagged relation, got {debug}"
    );
}

#[test]
fn parse_target_opponent_chooses_creature_then_destroy_that_creature() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Imperial Edict Variant")
        .parse_text("Target opponent chooses a creature they control. Destroy that creature.")
        .expect("target-opponent choose + destroy sequence should parse");

    let effects = def.spell_effect.expect("expected spell effects");
    let debug = format!("{effects:?}");
    assert!(
        debug.contains("ChooseObjectsEffect"),
        "expected choose-objects effect for chosen creature, got {debug}"
    );
    assert!(
        debug.contains("chooser: Target(Opponent)") || debug.contains("TargetOpponent"),
        "expected chooser to remain target-opponent scoped, got {debug}"
    );
    assert!(
        debug.contains("DestroyEffect"),
        "expected follow-up destroy effect for chosen creature, got {debug}"
    );
}

#[test]
fn parse_ghoulflesh_style_anthem_and_type_color_addition() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Ghoulflesh Variant")
            .card_types(vec![CardType::Enchantment])
            .subtypes(vec![Subtype::Aura])
            .parse_text(
                "Enchant creature\nEnchanted creature gets -1/-1 and is a black Zombie in addition to its other colors and types.",
            )
            .expect("parse ghoulflesh-style aura line");

    let ids = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::Anthem),
        "expected anthem in parsed abilities, got {ids:?}"
    );
    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::AddColors),
        "expected add-colors static ability, got {ids:?}"
    );
    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::AddSubtypes),
        "expected add-subtypes static ability, got {ids:?}"
    );
}

#[test]
fn parse_ghoulflesh_style_anthem_with_other_creature_types_scope() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Ghoulflesh Creature Types Scope")
            .card_types(vec![CardType::Enchantment])
            .subtypes(vec![Subtype::Aura])
            .parse_text(
                "Enchant creature\nEnchanted creature gets -1/-1 and is a black Zombie in addition to its other creature types.",
            )
            .expect("parse ghoulflesh creature-types scope");

    let ids = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::AddSubtypes),
        "expected add-subtypes static ability for creature-types scope, got {ids:?}"
    );
}

#[test]
fn parse_all_goblins_are_black_and_are_zombies_in_addition_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Dralnu Clause Variant")
        .parse_text(
            "All Goblins are black and are Zombies in addition to their other creature types.",
        )
        .expect("parse all-goblins color and type-addition line");

    let ids = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect::<Vec<_>>();

    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::SetColors),
        "expected set-colors static ability, got {ids:?}"
    );
    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::AddSubtypes),
        "expected add-subtypes static ability, got {ids:?}"
    );
}

#[test]
fn parse_type_color_addition_rejects_unsupported_scope_words() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Unsupported Addition Scope")
            .parse_text("Enchanted creature gets -1/-1 and is a black Zombie in addition to its other abilities.")
            .expect_err("unsupported addition scope should fail parse");
    let message = format!("{err:?}");
    assert!(
        message.contains("unsupported in-addition scope in type/color clause"),
        "expected strict scope parse error, got {message}"
    );
}

#[test]
fn parse_tap_untapped_creatures_cost_preserves_tap_filter_cost() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Hand of Justice Variant")
        .parse_text("{T}, Tap three untapped white creatures you control: Destroy target creature.")
        .expect("tap-untapped-creatures cost should parse");

    let ability = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("expected activated ability");
    let debug = format!("{:?}", ability.mana_cost);
    assert!(
        debug.contains("ChooseObjectsEffect"),
        "expected choose-objects tap cost in mana cost, got {debug}"
    );
    assert!(
        debug.contains("untapped: true"),
        "expected untapped filter requirement in tap cost, got {debug}"
    );
    assert!(
        debug.contains("count: ChoiceCount { min: 3, max: Some(3)")
            && debug.contains("dynamic_x: false"),
        "expected exactly-three tap cost selection, got {debug}"
    );
}

#[test]
fn parse_exile_graveyard_cost_activated_line_preserves_followup_effect() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Zombie Scavengers Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Exile the top creature card of your graveyard: Regenerate this creature.")
        .expect("exile-graveyard cost activated ability should parse");

    let lines = crate::compiled_text::compiled_lines(&def);
    let activated_line = lines
        .iter()
        .find(|line| line.contains("Activated ability"))
        .expect("expected activated ability line");
    assert!(
        activated_line.contains("Exile"),
        "expected exile cost to remain in activated ability text, got {activated_line}"
    );
    assert!(
        activated_line.contains("Regenerate"),
        "expected post-colon regenerate effect to remain, got {activated_line}"
    );
}

#[test]
fn parse_exile_source_cost_activated_line_preserves_followup_effect() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Selfless Glyphweaver Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Exile this creature: Creatures you control gain indestructible until end of turn.",
        )
        .expect("exile-source cost activated ability should parse");

    let lines = crate::compiled_text::compiled_lines(&def);
    let activated_line = lines
        .iter()
        .find(|line| line.contains("Activated ability"))
        .expect("expected activated ability line");
    assert!(
        activated_line.contains("Exile"),
        "expected exile cost to remain in activated ability text, got {activated_line}"
    );
    assert!(
        activated_line.contains("Indestructible"),
        "expected post-colon indestructible effect to remain, got {activated_line}"
    );
}

#[test]
fn parse_exile_this_card_from_graveyard_cost_uses_source_and_graveyard_zone() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Ghoulcaller Variant")
            .card_types(vec![CardType::Creature])
            .parse_text(
                "{3}{B}, Exile this card from your graveyard: Create a 2/2 black Zombie creature token. Activate only as a sorcery.",
            )
            .expect("exile-this-card-from-graveyard cost should parse as source exile");

    let activated = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some((ability, activated)),
            _ => None,
        })
        .expect("expected activated ability");

    assert_eq!(
        activated.0.functional_zones,
        vec![Zone::Graveyard],
        "expected graveyard functional zone for self-exile from graveyard"
    );

    let mana_cost_debug = format!("{:?}", activated.1.mana_cost);
    assert!(
        mana_cost_debug.contains("ExileEffect") && mana_cost_debug.contains("Source"),
        "expected source exile in activation cost, got {mana_cost_debug}"
    );
    assert!(
        !mana_cost_debug.contains("exile_cost_0"),
        "self exile from graveyard should not route through tagged choose-cost, got {mana_cost_debug}"
    );
}

#[test]
fn parse_targeted_exile_activation_cost_fails_strictly() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Targeted Exile Cost Variant")
        .card_types(vec![CardType::Artifact])
        .parse_text("Exile target creature card from a graveyard: Draw a card.")
        .expect_err("targeted exile cost should fail parse");
    let message = format!("{err:?}");
    assert!(
        message.contains("unsupported targeted exile cost segment"),
        "expected strict targeted-exile-cost parse error, got {message}"
    );
}

#[test]
fn parse_granted_activated_ability_to_non_source_compiles_as_grant() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Quicksmith Rebel Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "When this creature enters, target artifact you control gains \"{T}: This artifact deals 2 damage to any target\" for as long as you control this creature.",
        )
        .expect("non-source granted activated ability should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("target artifact you control gains")
            && rendered.contains("deals 2 damage to any target"),
        "expected granted activated ability wording, got {rendered}"
    );
}

#[test]
fn parse_put_into_hand_with_rest_tail_fails_strictly() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Organ Hoarder Variant")
            .card_types(vec![CardType::Creature])
            .parse_text(
                "When this creature enters, look at the top three cards of your library, then put one of them into your hand and the rest into your graveyard.",
            )
            .expect_err("multi-destination put-into-hand clause should fail parse");
    let message = format!("{err:?}");
    assert!(
        message.contains("unsupported multi-destination put clause"),
        "expected strict multi-destination put parse error, got {message}"
    );
}

#[test]
fn parse_put_target_creature_on_top_of_owner_library() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Griptide Variant")
        .card_types(vec![CardType::Instant])
        .parse_text("Put target creature on top of its owner's library.")
        .expect("put-on-top-of-library clause should parse");
    let lines = crate::compiled_text::compiled_lines(&def);
    let joined = lines.join("\n").to_ascii_lowercase();
    assert!(
        joined.contains("top of") && joined.contains("library"),
        "expected top-of-library move wording, got {joined}"
    );
}

#[test]
fn parse_draw_then_put_source_on_top_of_library() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Sensei Top Variant")
        .card_types(vec![CardType::Artifact])
        .parse_text("{T}: Draw a card, then put this artifact on top of its owner's library.")
        .expect("draw-then-put-self-on-top clause should parse");
    let joined = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains("draw a card") && joined.contains("top of its owner's library"),
        "expected draw-then-put-self wording, got {joined}"
    );
}

#[test]
fn parse_draw_then_put_source_third_from_top() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Sensei Top Third Variant")
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "{T}: Draw a card, then put this artifact third from the top of its owner's library.",
        )
        .expect_err("third-from-top library-position tail remains unsupported");
    let message = format!("{err:?}").to_ascii_lowercase();
    assert!(
        message.contains("unsupported put clause"),
        "expected strict unsupported third-from-top clause, got {message}"
    );
}

#[test]
fn parse_put_target_beneath_top_x_cards() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Unexpectedly Absent Variant")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Put target nonland permanent into its owner's library just beneath the top X cards of that library.",
        )
        .expect("beneath-top-x library-position clause should parse");
    let message = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        message.contains("just beneath the top x cards"),
        "expected beneath-top-x wording, got {message}"
    );
}

#[test]
fn parse_put_target_third_from_bottom_still_fails_loudly() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Library Bottom Negative Variant")
        .card_types(vec![CardType::Instant])
        .parse_text("Put target nonland permanent into its owner's library third from the bottom.")
        .expect_err("unsupported bottom-position clause should fail parse");
    let message = format!("{err:?}");
    assert!(
        message.contains("unsupported put clause"),
        "expected strict unsupported put-clause error, got {message}"
    );
}

#[test]
fn parse_triggered_put_into_graveyard_from_anywhere() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Worldspine Trigger Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("When this creature is put into a graveyard from anywhere, shuffle it into its owner's library.")
        .expect("put-into-graveyard-from-anywhere trigger should parse");
    let joined = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains("is put into a graveyard from anywhere")
            && (joined.contains("shuffle it into its owner's library")
                || (joined.contains("put it on the bottom of its owner's library")
                    && joined.contains("shuffle your library"))),
        "expected graveyard-from-anywhere trigger wording, got {joined}"
    );
}

#[test]
fn parse_triggered_put_into_exile_from_anywhere_fails_strictly() {
    let err = CardDefinitionBuilder::new(CardId::new(), "From Anywhere Exile Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("When this creature is put into exile from anywhere, shuffle it into its owner's library.")
        .expect_err("unsupported from-anywhere exile trigger should fail parse");
    let message = format!("{err:?}");
    assert!(
        message.contains("unsupported triggered line"),
        "expected strict unsupported triggered-line error, got {message}"
    );
}

#[test]
fn parse_add_any_color_for_each_removed_counter() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Coalition Relic Variant")
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "At the beginning of your first main phase, remove all charge counters from this artifact. Add one mana of any color for each charge counter removed this way.",
        )
        .expect("dynamic removed-counter mana clause should parse");
    let joined = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains("remove the number of charge counter")
            && joined.contains("add x mana of any color"),
        "expected removed-counter mana wording, got {joined}"
    );
}

#[test]
fn parse_add_any_color_for_each_removed_counter_with_unsupported_tail_fails_strictly() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Coalition Relic Negative Variant")
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "At the beginning of your first main phase, remove all charge counters from this artifact. Add one mana of any color for each charge counter removed this way unless it's your turn.",
        )
        .expect_err("unsupported removed-counter mana tail should fail parse");
    let message = format!("{err:?}");
    assert!(
        message.contains("unsupported trailing mana clause"),
        "expected strict trailing-mana error, got {message}"
    );
}

#[test]
fn parse_starting_life_total_amount_in_trigger() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Endstone Variant")
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "At the beginning of your end step, your life total becomes half your starting life total, rounded up.",
        )
        .expect("starting-life-total amount should parse");
    let joined = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains("half your starting life total, rounded up"),
        "expected starting-life-total wording, got {joined}"
    );
}

#[test]
fn parse_starting_life_total_amount_with_extra_math_fails_strictly() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Endstone Negative Variant")
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "At the beginning of your end step, your life total becomes half your starting life total plus one.",
        )
        .expect_err("unsupported starting-life-total math should fail parse");
    let message = format!("{err:?}");
    assert!(
        message.contains("missing life total amount"),
        "expected strict missing-life-total-amount error, got {message}"
    );
}

#[test]
fn parse_mana_replacement_clause_deep_water_fails_instead_of_partial_tap() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Deep Water Variant")
            .parse_text(
                "{U}: Until end of turn, if you tap a land you control for mana, it produces {U} instead of any other type.",
            )
            .expect_err("unsupported mana replacement clause should fail parse");
    let message = format!("{err:?}");
    assert!(
        message.contains("unsupported mana replacement clause"),
        "expected strict mana replacement parse error, got {message}"
    );
}

#[test]
fn parse_mana_replacement_clause_harvest_mage_fails_instead_of_partial_tap() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Harvest Mage Variant")
            .parse_text(
                "{G}, {T}, Discard a card: Until end of turn, if you tap a land for mana, it produces one mana of a color of your choice instead of any other type and amount.",
            )
            .expect_err("unsupported mana replacement clause should fail parse");
    let message = format!("{err:?}");
    assert!(
        message.contains("unsupported mana replacement clause"),
        "expected strict mana replacement parse error, got {message}"
    );
}

#[test]
fn parse_mana_replacement_clause_with_taps_plural_fails_strictly() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Pale Moon Variant")
            .parse_text(
                "Until end of turn, if a player taps a nonbasic land for mana, it produces colorless mana instead of any other type.",
            )
            .expect_err("unsupported mana replacement clause should fail parse");
    let message = format!("{err:?}");
    assert!(
        message.contains("unsupported mana replacement clause"),
        "expected strict mana replacement parse error, got {message}"
    );
}

#[test]
fn parse_mana_trigger_additional_clause_high_tide_fails_strictly() {
    let err = CardDefinitionBuilder::new(CardId::new(), "High Tide Variant")
            .parse_text(
                "Until end of turn, whenever a player taps an Island for mana, that player adds an additional {U}.",
            )
            .expect_err("unsupported mana-triggered additional-mana clause should fail parse");
    let message = format!("{err:?}");
    assert!(
        message.contains("unsupported mana-triggered additional-mana clause"),
        "expected strict mana-triggered parse error, got {message}"
    );
}

#[test]
fn parse_add_mana_chosen_color_tail() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Thriving Mana Variant")
        .card_types(vec![CardType::Land])
        .parse_text("{T}: Add {B} or one mana of the chosen color.")
        .expect("chosen-color mana tail should parse");
    let lines = compiled_lines(&def);
    let mana_line = lines
        .iter()
        .find(|line| line.contains("Mana ability"))
        .expect("expected mana ability line");
    assert!(
        mana_line.contains("Add {B} or one mana of the chosen color"),
        "expected chosen-color mana render, got {mana_line}"
    );
}

#[test]
fn parse_metalcraft_mana_activation_condition() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Mox Opal Variant")
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "{T}: Add one mana of any color. Activate only if you control three or more artifacts.",
        )
        .expect("metalcraft mana activation condition should parse");

    let lines = compiled_lines(&def);
    let mana_line = lines
        .iter()
        .find(|line| line.contains("Mana ability"))
        .expect("expected mana ability line");
    assert!(
        mana_line.contains("Add one mana of any color"),
        "expected mana production text in compiled output, got {mana_line}"
    );
    assert!(
        mana_line.contains("Activate only if you control 3 or more artifacts")
            || mana_line.contains("Activate only if you control three or more artifacts"),
        "expected rendered activation restriction, got {mana_line}"
    );
}

#[test]
fn parse_land_count_mana_activation_condition() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Temple Variant")
        .card_types(vec![CardType::Land])
        .parse_text("{T}: Add {C}{C}. Activate only if you control five or more lands.")
        .expect("land-count mana activation condition should parse");

    let lines = compiled_lines(&def);
    let mana_line = lines
        .iter()
        .find(|line| line.contains("Mana ability"))
        .expect("expected mana ability line");
    assert!(
        mana_line.contains("Add {C}{C}"),
        "expected mana amount in compiled output, got {mana_line}"
    );
    assert!(
        mana_line.contains("Activate only if you control 5 or more lands")
            || mana_line.contains("Activate only if you control five or more lands"),
        "expected rendered activation restriction, got {mana_line}"
    );
}

#[test]
fn parse_graveyard_card_mana_activation_condition() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Elf Tomb Variant")
        .card_types(vec![CardType::Land])
        .parse_text("{T}: Add {G}{G}. Activate only if there is an Elf card in your graveyard.")
        .expect("graveyard-card mana activation condition should parse");

    let lines = compiled_lines(&def);
    let mana_line = lines
        .iter()
        .find(|line| line.contains("Mana ability"))
        .expect("expected mana ability line");
    assert!(
        mana_line.contains("Add {G}{G}"),
        "expected mana amount in compiled output, got {mana_line}"
    );
    assert!(
        mana_line.contains("Activate only if there is an elf card in your graveyard"),
        "expected rendered activation restriction, got {mana_line}"
    );
}

#[test]
fn parse_creature_power_mana_activation_condition() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Ferocious Mana Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "{T}: Add {G}{G}. Activate only if you control a creature with power 4 or greater.",
        )
        .expect("creature-power mana activation condition should parse");

    let lines = compiled_lines(&def);
    let mana_line = lines
        .iter()
        .find(|line| line.contains("Mana ability"))
        .expect("expected mana ability line");
    assert!(
        mana_line.contains("Add {G}{G}"),
        "expected mana amount in compiled output, got {mana_line}"
    );
    assert!(
        mana_line.contains("Activate only if you control a creature with power 4 or greater")
            || mana_line.contains("Activate only if you control creature with power 4 or greater"),
        "expected rendered activation restriction, got {mana_line}"
    );
}

#[test]
fn parse_total_power_mana_activation_condition() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Formidable Mana Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("{T}: Add {C}{C}{C}. Activate only if creatures you control have total power 8 or greater.")
        .expect("total-power mana activation condition should parse");

    let lines = compiled_lines(&def);
    let mana_line = lines
        .iter()
        .find(|line| line.contains("Mana ability"))
        .expect("expected mana ability line");
    assert!(
        mana_line.contains("Add {C}{C}{C}"),
        "expected mana amount in compiled output, got {mana_line}"
    );
    assert!(
        mana_line.contains("Activate only if creatures you control have total power 8 or greater"),
        "expected rendered activation restriction, got {mana_line}"
    );
}

#[test]
fn parse_inline_whenever_clause_keeps_its_controller_subject() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Noxious Assault Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Creatures you control get +2/+2 until end of turn. Whenever a creature blocks this turn, its controller gets a poison counter.",
        )
        .expect("inline whenever clause with its-controller subject should parse");

    let joined = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains("controller gets a poison counter")
            || joined.contains("that object's controller gets a poison counter"),
        "expected controller-based poison counter wording, got {joined}"
    );
    assert!(
        !joined.contains("you get 1 poison counter"),
        "did not expect implicit-you poison counter wording, got {joined}"
    );
}

#[test]
fn parse_until_end_of_turn_whenever_clause_as_temporary_grant() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Mountain Titan Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("{1}{R}{R}: Until end of turn, whenever you cast a black spell, put a +1/+1 counter on this creature.")
        .expect_err("until-end-of-turn whenever grant is currently unsupported");
    let rendered = format!("{err:?}").to_ascii_lowercase();
    assert!(
        rendered.contains("unsupported until-end-of-turn permission clause"),
        "expected explicit unsupported until-end-of-turn permission error, got {rendered}"
    );
}

#[test]
fn parse_rejects_marker_keyword_with_non_keyword_tail() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Ninjutsu Tail Variant")
        .parse_text("Ninjutsu abilities you activate cost {1} less to activate.")
        .expect_err("non-keyword ninjutsu tail should not parse as a bare keyword");
    let message = format!("{err:?}");
    assert!(
        message.contains("could not find verb")
            || message.contains("unsupported")
            || message.contains("parse"),
        "expected parse failure for non-keyword tail, got {message}"
    );
}

#[test]
fn parse_ninjutsu_keyword_line_builds_hand_activated_ability() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Ninjutsu Probe")
        .parse_text("Ninjutsu {1}{B}")
        .expect("ninjutsu keyword line should parse");

    let ability = def
        .abilities
        .iter()
        .find(|ability| matches!(ability.kind, AbilityKind::Activated(_)))
        .expect("expected activated ninjutsu ability");
    assert!(
        ability.functional_zones.contains(&Zone::Hand),
        "ninjutsu should function from hand"
    );
    let AbilityKind::Activated(activated) = &ability.kind else {
        panic!("expected activated ability");
    };
    assert_eq!(
        activated.timing,
        crate::ability::ActivationTiming::DuringCombat,
        "ninjutsu should use during-combat timing"
    );

    let cost_debug = format!("{:?}", activated.mana_cost);
    assert!(
        cost_debug.contains("NinjutsuCostEffect"),
        "expected ninjutsu return-attacker cost effect, got {cost_debug}"
    );

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        (rendered.contains("effect(ninjutsucosteffect)")
            && rendered.contains("effect(ninjutsueffect)"))
            || (rendered.contains("return an unblocked attacker")
                && rendered.contains("put this card onto the battlefield tapped and attacking")),
        "expected compiled output to include ninjutsu effect pipeline, got {rendered}"
    );
}

#[test]
fn parse_each_player_discard_then_draw_keeps_each_player_scope() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Wheel Variant")
        .parse_text("Each player discards their hand, then draws seven cards.")
        .expect("each-player discard-then-draw should parse");

    let compiled = compiled_lines(&def).join(" ");
    assert!(
        compiled.contains("Each player discards their hand, then draws 7 cards")
            || compiled.contains("Each player discards their hand, then draws seven cards"),
        "expected each-player scope to carry into draw clause, got {compiled}"
    );
}

#[test]
fn parse_non_outlaw_creature_filter_excludes_outlaw_subtypes() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Shoot Variant")
        .parse_text("Destroy target non-outlaw creature.")
        .expect("non-outlaw target filter should parse");

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("excluded_subtypes: [Assassin, Mercenary, Pirate, Rogue, Warlock]")
            || debug.contains("excluded_subtypes: [Assassin, Mercenary, Pirate, Rogue, Warlock,"),
        "expected outlaw subtype exclusions in parsed filter, got {debug}"
    );
}

#[test]
fn parse_for_each_object_subject_wraps_create_effect() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Predation Variant")
        .parse_text(
            "For each creature your opponents control, create a 4/4 green Beast creature token.",
        )
        .expect("for-each object create clause should parse");

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("ForEachObject"),
        "expected ForEachObject lowering, got {debug}"
    );
}

#[test]
fn parse_create_for_each_tail_wraps_create_effect() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Pack Variant")
        .parse_text("Create a 1/1 white Soldier creature token for each creature you control.")
        .expect("create-for-each tail should parse");

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("CreateTokenEffect") && debug.contains("count: Count("),
        "expected counted token creation based on controlled creatures, got {debug}"
    );
}

#[test]
fn parse_earthbend_then_untap_keeps_tail_effect() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Earthbend Variant")
        .parse_text("Earthbend 8, then untap that land.")
        .expect("earthbend with tail clause should parse");

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("EarthbendEffect") && debug.contains("UntapEffect"),
        "expected earthbend and untap effects, got {debug}"
    );
    assert!(
        debug.contains("earthbend_0"),
        "expected earthbend target tag to carry into tail untap, got {debug}"
    );
}

#[test]
fn parse_instead_if_control_keeps_prior_damage_target() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Steer Clear Variant")
        .parse_text(
            "Steer Clear deals 2 damage to target attacking or blocking creature. Steer Clear deals 4 damage to that creature instead if you controlled a Mount as you cast this spell.",
        )
        .expect("instead-if damage clause should parse");

    let rendered = compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Deal 4 damage to target attacking or blocking creature")
            && rendered
                .contains("Otherwise, Deal 2 damage to target attacking or blocking creature"),
        "expected instead-if render with the original creature target, got {rendered}"
    );
}

#[test]
fn parse_instead_if_control_omitted_target_reuses_prior_damage_target() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Invasive Maneuvers Variant")
        .parse_text(
            "Invasive Maneuvers deals 3 damage to target creature. It deals 5 damage instead if you control a Spacecraft.",
        )
        .expect("instead-if followup sentence should reuse prior target");

    let rendered = compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Deal 5 damage to target creature")
            && rendered.contains("Otherwise, Deal 3 damage to target creature"),
        "expected conditional to preserve the original creature target, got {rendered}"
    );
}

#[test]
fn parse_instead_if_control_omitted_target_reuses_prior_damage_target_with_or_filter() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Chandra's Triumph Variant")
        .parse_text(
            "Chandra's Triumph deals 3 damage to target creature or planeswalker an opponent controls. Chandra's Triumph deals 5 damage instead if you control a Chandra planeswalker.",
        )
        .expect("instead-if followup sentence should reuse prior target");

    let rendered = compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Deal 5 damage to target creature an opponent controls or planeswalker")
            && rendered.contains(
                "Otherwise, Deal 3 damage to target creature an opponent controls or planeswalker"
            ),
        "expected conditional to preserve the original creature-or-planeswalker target, got {rendered}"
    );
}

#[test]
fn parse_spell_line_instead_followup_merges_into_prior_spell_effect() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Galvanic Blast Variant")
        .parse_text(
            "Galvanic Blast deals 2 damage to any target.\nMetalcraft — Galvanic Blast deals 4 damage instead if you control three or more artifacts.",
        )
        .expect("metalcraft instead followup line should merge into prior spell effect");

    let rendered = compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Deal 2 damage to any target")
            && rendered
                .contains("It deals 4 damage instead if you control three or more artifacts"),
        "expected metalcraft line to replace prior damage amount and reuse target, got {rendered}"
    );
}

#[test]
fn parse_spell_line_instead_followup_merges_non_control_predicate() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Cackling Flames Variant")
        .parse_text(
            "Cackling Flames deals 3 damage to any target.\nHellbent — Cackling Flames deals 5 damage instead if you have no cards in hand.",
        )
        .expect("hellbent instead followup line should merge into prior spell effect");

    let rendered = compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Deal 3 damage to any target")
            && rendered.contains("It deals 5 damage instead if you have no cards in hand"),
        "expected hellbent line to replace prior damage amount and reuse target, got {rendered}"
    );
}

#[test]
fn parse_deal_damage_with_trailing_if_clause_emits_conditional() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Kami's Flare Variant")
        .parse_text(
            "Kami's Flare deals 3 damage to target creature or planeswalker. Kami's Flare also deals 2 damage to that permanent's controller if you control a modified creature. (Equipment, Auras you control, and counters are modifications.)",
        )
        .expect("trailing if control clause should parse");

    let rendered = compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Deal 3 damage to target creature or planeswalker")
            && rendered.contains("If you control a modified creature")
            && rendered.contains("Deal 2 damage to that object's controller"),
        "expected conditional damage followup, got {rendered}"
    );
}

#[test]
fn parse_damage_to_that_creatures_controller_targets_player() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Chandra Variant")
        .parse_text(
            "Chandra's Outrage deals 4 damage to target creature and 2 damage to that creature's controller.",
        )
        .expect("damage to that creature's controller should parse");

    let rendered = compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("that object's controller"),
        "expected controller-target damage wording, got {rendered}"
    );
}

#[test]
fn mana_ability_render_uses_colon_separator() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Mana Separator Variant")
        .card_types(vec![CardType::Land])
        .parse_text("{T}: Add {W}.")
        .expect("basic mana ability should parse");

    let line = compiled_lines(&def)
        .into_iter()
        .find(|line| line.contains("Mana ability"))
        .expect("expected mana ability line");
    assert!(
        line.contains("{T}: Add {W}") && !line.contains("{T}, Add {W}"),
        "expected colon-separated mana text, got {line}"
    );
}

#[test]
fn parse_reveal_hand_clause_with_trailing_effect_fails_strictly() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Retraced Image Variant")
            .parse_text(
                "Reveal a card in your hand, then put that card onto the battlefield if it has the same name as a permanent.",
            )
            .expect_err("partial reveal-hand parsing should fail");
    let message = format!("{err:?}");
    assert!(
        message.contains("unsupported reveal-hand clause"),
        "expected strict reveal-hand parse error, got {message}"
    );
}

#[test]
fn parse_reveal_hand_clause_with_colon_tail_fails_strictly() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Sasaya Variant")
        .parse_text(
            "Reveal your hand: If you have seven or more land cards in your hand, flip Sasaya.",
        )
        .expect_err("partial reveal-hand parsing should fail");
    let message = format!("{err:?}");
    assert!(
        message.contains("unsupported reveal-hand clause"),
        "expected strict reveal-hand parse error, got {message}"
    );
}

#[test]
fn parse_reveal_any_number_of_cards_in_your_hand_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Scent Variant")
        .parse_text("Reveal any number of red cards in your hand.")
        .expect("reveal-any-number-in-hand clause should parse");

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("ChooseObjectsEffect") && debug.contains("zone: Some(Hand)"),
        "expected choose-from-hand reveal setup, got {debug}"
    );
}

#[test]
fn parse_reveal_x_cards_in_your_hand_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Nightshade Assassin Variant")
        .parse_text("Reveal X black cards in your hand.")
        .expect("reveal-x-in-hand clause should parse");

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("ChooseObjectsEffect") && debug.contains("zone: Some(Hand)"),
        "expected x-count choose-from-hand reveal setup, got {debug}"
    );
}

#[test]
fn parse_reveal_single_card_in_your_hand_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Assembly Hall Variant")
        .parse_text("Reveal a creature card in your hand.")
        .expect("reveal-single-card-in-hand clause should parse");

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("ChooseObjectsEffect") && debug.contains("zone: Some(Hand)"),
        "expected single-card choose-from-hand reveal setup, got {debug}"
    );
}

#[test]
fn parse_reveal_top_plural_cards_clause() {
    CardDefinitionBuilder::new(CardId::new(), "Top Reveal Variant")
        .parse_text("Reveal the top five cards of your library.")
        .expect("reveal-top plural cards clause should parse");
}

#[test]
fn parse_reveal_top_card_clause_without_library_suffix() {
    CardDefinitionBuilder::new(CardId::new(), "Top Card Reveal Variant")
        .parse_text("Reveal the top card.")
        .expect("reveal top-card shorthand should parse");
}

#[test]
fn parse_reveal_top_card_then_lose_life_followup() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Dark Confidant Variant")
        .parse_text(
            "At the beginning of your upkeep, reveal the top card of your library and put that card into your hand. You lose life equal to its mana value.",
        )
        .expect("dark confidant-style reveal followup should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("reveal the top card of your library")
            && (rendered.contains("lose life equal to its mana value")
                || rendered.contains("lose life equal to that card's mana value")),
        "expected reveal and life-loss followup, got {rendered}"
    );
}

#[test]
fn parse_discard_up_to_two_then_draw_that_many() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Tersa Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "When this creature enters, discard up to two cards, then draw that many cards.",
        )
        .expect("discard-up-to-two then draw-that-many should parse");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("Discard")
            && debug.contains("Fixed(2)")
            && (debug.contains("EventValue(Amount)") || debug.contains("EffectValue(EffectId(")),
        "expected discard-count and draw-that-many lowering, got {debug}"
    );
}

#[test]
fn discard_up_to_two_permanents_then_draw_that_many_still_fails_loudly() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Tersa Negative Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "When this creature enters, discard up to two permanents, then draw that many cards.",
        )
        .expect_err("unsupported discard noun should fail loudly");

    let message = format!("{err:?}");
    assert!(
        message.contains("missing card keyword") || message.contains("unsupported discard"),
        "expected loud discard-qualifier failure, got {message}"
    );
}

#[test]
fn parse_broadside_bombardiers_boast_damage_formula() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Broadside Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Boast — Sacrifice another creature or artifact: This creature deals damage equal to 2 plus the sacrificed permanent's mana value to any target. (Activate only if this creature attacked this turn and only once each turn.)",
        )
        .expect("broadside boast damage formula should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("boast")
            && (rendered.contains("2 plus the sacrificed permanent's mana value")
                || rendered.contains("2 plus its mana value"))
            && rendered.contains("any target"),
        "expected boast damage formula rendering, got {rendered}"
    );
}

#[test]
fn parse_reveal_top_card_then_if_land_else_hand_sequence() {
    CardDefinitionBuilder::new(CardId::new(), "Nadu Variant")
        .parse_text(
            "Creatures you control have \"Whenever this creature becomes the target of a spell or ability, reveal the top card of your library. If it's a land card, put it onto the battlefield. Otherwise, put it into your hand. This ability triggers only twice each turn.\"",
        )
        .expect("nadu-style reveal top card sequence should parse");
}

#[test]
fn reveal_top_card_sequence_still_fails_loudly_for_unsupported_tail() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Unsupported Reveal Tail Variant")
        .parse_text(
            "Reveal the top card of your library and put that card into your hand. Repeat this process.",
        )
        .expect_err("unsupported repeat-this-process tail should still fail");

    let rendered = format!("{err:?}").to_ascii_lowercase();
    assert!(
        rendered.contains("repeat this process")
            || rendered.contains("could not find verb in effect clause"),
        "expected loud unsupported-tail failure, got {rendered}"
    );
    assert!(
        !rendered.contains("missing reveal count in reveal-top matching split clause"),
        "expected reveal-top helper to decline unrelated top-card text, got {rendered}"
    );
}

#[test]
fn parse_reveal_card_this_way_trigger_clause() {
    CardDefinitionBuilder::new(CardId::new(), "Primitive Etchings Variant")
        .parse_text(
            "Reveal the first card you draw each turn. Whenever you reveal a creature card this way, draw a card.",
        )
        .expect("reveal-card-this-way trigger clause should parse");
}

#[test]
fn parse_reveal_cards_in_library_clause() {
    CardDefinitionBuilder::new(CardId::new(), "Guided Passage Variant")
        .parse_text("Reveal the cards in your library.")
        .expect("reveal-all-library cards clause should parse");
}

#[test]
fn parse_target_creature_attacks_or_blocks_if_able() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Hustle Variant")
        .parse_text("Target creature attacks or blocks this turn if able.")
        .expect("target creature attacks-or-blocks clause should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("gains attacks each combat if able")
            && rendered.contains("gains blocks each combat if able"),
        "expected attack/block-if-able grants, got {rendered}"
    );
}

#[test]
fn parse_target_creature_becomes_red_and_attacks_if_able() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Incite Variant")
        .parse_text("Target creature becomes red until end of turn and attacks this turn if able.")
        .expect("incite-style color-change plus attacks-if-able clause should parse");

    let debug = format!("{:#?}", def.spell_effect);
    assert!(
        debug.contains("SetColors"),
        "expected explicit color-set effect, got {debug}"
    );
    assert!(
        debug.contains("MustAttack"),
        "expected attacks-if-able grant on the same target, got {debug}"
    );
    assert!(
        debug.contains("Tagged("),
        "expected follow-up must-attack effect to reference prior target by tag, got {debug}"
    );
    assert!(
        !debug.contains("colors: Some("),
        "should not reinterpret subject as an already-red filter, got {debug}"
    );
}

#[test]
fn parse_target_creature_can_block_any_number_fails_strictly() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Valor Variant")
        .parse_text("Target creature can block any number of creatures this turn.")
        .expect_err("unsupported target-only combat-action clause should fail");
    let message = format!("{err:?}");
    assert!(
        message.contains("unsupported target-only restriction clause"),
        "expected strict target-only restriction error, got {message}"
    );
}

#[test]
fn parse_target_creature_blocks_this_turn_if_able() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Culling Mark Variant")
        .parse_text("Target creature blocks this turn if able.")
        .expect("target creature blocks-if-able clause should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("gains blocks each combat if able"),
        "expected must-block effect, got {rendered}"
    );
}

#[test]
fn parse_each_creature_opponents_control_blocks_this_turn_if_able() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Predatory Rampage Variant")
        .parse_text("Each creature your opponents control blocks this turn if able.")
        .expect("each-creature blocks-if-able clause should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("creatures gain blocks each combat if able"),
        "expected must-block effect for filtered creatures, got {rendered}"
    );
}

#[test]
fn parse_play_that_card_from_exile_this_turn_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Play From Exile Variant")
        .parse_text(
            "Exile target card from a graveyard. You may play that card from exile this turn.",
        )
        .expect("play-that-card-from-exile clause should parse");

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("GrantPlayTaggedEffect") && debug.contains("UntilEndOfTurn"),
        "expected end-of-turn play permission effect, got {debug}"
    );
}

#[test]
fn parse_play_an_additional_land_this_turn_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Explore Variant")
        .parse_text("You may play an additional land this turn. Draw a card.")
        .expect("additional land play clause should parse");

    let debug = format!("{:?}", def.spell_effect.as_ref().expect("spell effects"));
    assert!(
        debug.contains("AdditionalLandPlaysEffect") && debug.contains("duration: EndOfTurn"),
        "expected temporary additional-land-play effect, got {debug}"
    );
}

#[test]
fn parse_newline_additional_land_this_turn_clause_stays_a_spell_effect() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Explore")
        .parse_text("You may play an additional land this turn.\nDraw a card.")
        .expect("newline additional land play clause should parse");

    let abilities_debug = format!("{:?}", def.abilities);
    let spell_debug = format!("{:?}", def.spell_effect.as_ref().expect("spell effects"));
    assert!(
        !abilities_debug.contains("AdditionalLandPlay"),
        "temporary additional land play should not become a battlefield static ability: {abilities_debug}"
    );
    assert!(
        spell_debug.contains("AdditionalLandPlaysEffect")
            && spell_debug.contains("duration: EndOfTurn"),
        "expected end-of-turn additional-land-play spell effect, got {spell_debug}"
    );
}

#[test]
fn parse_additional_land_this_turn_clause_is_not_wrapped_in_may() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Explore")
        .parse_text("You may play an additional land this turn.\nDraw a card.")
        .expect("explore-style text should parse");

    let spell_debug = format!("{:?}", def.spell_effect.as_ref().expect("spell effects"));
    assert!(
        !spell_debug.contains("MayEffect"),
        "permission-granting land-play text should not become a MayEffect: {spell_debug}"
    );
}

#[test]
fn compiled_text_keeps_additional_land_this_turn_duration() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Explore")
        .parse_text("You may play an additional land this turn.\nDraw a card.")
        .expect("explore-style text should parse");

    let rendered = oracle_like_lines(&def).join(" ");
    assert!(
        rendered.contains("You may play an additional land this turn"),
        "compiled text should keep temporary land-play duration, got {rendered}"
    );
    assert!(
        rendered.contains("Draw a card"),
        "compiled text should preserve draw effect, got {rendered}"
    );
}

#[test]
fn parse_spell_next_upkeep_trigger_stays_in_spell_effects() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Pact Variant")
        .parse_text(
            "Search your library for a green creature card, reveal it, put it into your hand, then shuffle. At the beginning of your next upkeep, pay {2}{G}{G}. If you don't, you lose the game.",
        )
        .expect("next-upkeep pact line should parse");

    let spell_debug = format!("{:?}", def.spell_effect.as_ref().expect("spell effects"));
    let ability_debug = format!("{:?}", def.abilities);
    assert!(
        spell_debug.contains("ScheduleDelayedTriggerEffect")
            && spell_debug.contains("BeginningOfUpkeepTrigger")
            && spell_debug.contains("start_next_turn: true"),
        "expected next-upkeep delayed trigger in spell effects, got {spell_debug}"
    );
    assert!(
        !ability_debug.contains("BeginningOfUpkeepTrigger"),
        "delayed next-upkeep clause should not become a printed triggered ability: {ability_debug}"
    );
}

#[test]
fn parse_fastbond_additional_land_permission_is_explicitly_unsupported() {
    let err = CardDefinitionBuilder::new(CardId::from_raw(1), "Fastbond Variant")
        .card_types(vec![CardType::Enchantment])
        .parse_text("You may play any number of lands on each of your turns.")
        .expect_err("additional land play permission should stay unsupported");

    let debug = format!("{err:?}").to_ascii_lowercase();
    assert!(
        debug.contains("unsupported additional-land-play permission clause"),
        "expected explicit additional-land-play permission error, got {debug}"
    );
}

#[test]
fn parse_for_as_long_as_play_permission_is_explicitly_unsupported() {
    let err = CardDefinitionBuilder::new(CardId::from_raw(1), "Elite Spellbinder Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Flying\nWhen this creature enters, look at target opponent's hand. You may exile a nonland card from it. For as long as that card remains exiled, its owner may play it.",
        )
        .expect_err("for-as-long-as play permission should stay unsupported");

    let debug = format!("{err:?}").to_ascii_lowercase();
    assert!(
        debug.contains("unsupported for-as-long-as play/cast permission clause"),
        "expected explicit for-as-long-as play permission error, got {debug}"
    );
}

#[test]
fn parse_temporary_free_play_permission_is_explicitly_unsupported() {
    let err = CardDefinitionBuilder::new(CardId::from_raw(1), "Golos Variant")
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .parse_text(
            "{2}{W}{U}{B}{R}{G}: Exile the top three cards of your library. You may play them this turn without paying their mana costs.",
        )
        .expect_err("temporary free play permission should stay unsupported");

    let debug = format!("{err:?}").to_ascii_lowercase();
    assert!(
        debug.contains("unsupported temporary play/cast permission clause with alternative cost"),
        "expected explicit temporary free play permission error, got {debug}"
    );
}

#[test]
fn parse_omniscience_static_free_cast_permission() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Omniscience Variant")
        .card_types(vec![CardType::Enchantment])
        .parse_text("You may cast spells from your hand without paying their mana costs.")
        .expect("Omniscience static permission should parse");

    let rendered = compiled_lines(&def).join(" ");
    assert!(
        rendered
            .to_ascii_lowercase()
            .contains("you may cast spells from your hand without paying their mana costs"),
        "expected omniscience wording in compiled output, got {rendered}"
    );

    let has_free_cast_grant = def.abilities.iter().any(|ability| {
        let AbilityKind::Static(static_ability) = &ability.kind else {
            return false;
        };
        let Some(spec) = static_ability.grant_spec() else {
            return false;
        };
        matches!(
            spec.grantable,
            crate::grant::Grantable::AlternativeCast(
                crate::alternative_cast::AlternativeCastingMethod::Composed { .. }
            )
        ) && spec.zone == Zone::Hand
    });

    assert!(
        has_free_cast_grant,
        "expected a hand free-cast grant in parsed Omniscience ability"
    );
}

#[test]
fn parse_put_land_card_from_hand_onto_battlefield_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Scout Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("{T}: You may put a land card from your hand onto the battlefield.")
        .expect("put-land-from-hand clause should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("put a land card from your hand onto the battlefield"),
        "expected put-land wording in compiled output, got {rendered}"
    );
}

#[test]
fn parse_recommission_text_parses_typed_counter_followup() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Recommission Variant")
        .parse_text(
            "Return target artifact or creature card with mana value 3 or less from your graveyard to the battlefield. If a creature enters this way, it enters with an additional +1/+1 counter on it.",
        )
        .expect("mixed return+enters-with-counters clause should parse");

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("IfEffect") && debug.contains("PutCountersEffect"),
        "expected typed conditional put-counters followup, got {debug}"
    );
    let static_ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        !static_ids.contains(&StaticAbilityId::RuleTextPlaceholder),
        "recommission followup should not emit static placeholder fallback: {static_ids:?}"
    );
}

#[test]
fn parse_teferis_time_twist_text_parses_typed_counter_followup() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Teferi Time Twist Variant")
        .parse_text(
            "Exile target permanent you control. Return that card to the battlefield under its owner's control at the beginning of the next end step. If it enters as a creature, it enters with an additional +1/+1 counter on it.",
        )
        .expect("mixed exile/return+enters-with-counters clause should parse");

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("IfEffect") && debug.contains("PutCountersEffect"),
        "expected typed delayed conditional put-counters followup, got {debug}"
    );
    let static_ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        !static_ids.contains(&StaticAbilityId::RuleTextPlaceholder),
        "time-twist followup should not emit static placeholder fallback: {static_ids:?}"
    );
}

#[test]
fn parse_named_enters_tapped_and_doesnt_untap_fails_strictly() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Grimgrin Variant")
        .parse_text("Grimgrin enters tapped and doesn't untap during your untap step.")
        .expect_err("mixed enters-tapped/negated-untap should fail parse");
    let message = format!("{err:?}");
    assert!(
        message.contains("unsupported mixed enters-tapped and negated-untap clause"),
        "expected strict mixed enters-tapped parse error, got {message}"
    );
}

#[test]
fn parse_at_trigger_intro_ignores_look_at_the_top_clause() {
    let tokens = tokenize_line("Look at the top seven cards of your library.", 0);
    assert!(
        !is_at_trigger_intro(&tokens, 1),
        "look-at clause must not be treated as trigger intro"
    );
}

#[test]
fn parse_at_trigger_intro_matches_beginning_clause() {
    let tokens = tokenize_line("At the beginning of your upkeep, draw a card.", 0);
    assert!(
        is_at_trigger_intro(&tokens, 0),
        "'at the beginning' should be treated as trigger intro"
    );
}

#[test]
fn parse_enchanted_creature_doesnt_untap_during_controller_untap_step() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Sleep Variant")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "Enchant creature\nEnchanted creature doesn't untap during its controller's untap step.",
        )
        .expect("attached negated untap clause should parse as static grant");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        ids.contains(&StaticAbilityId::AttachedAbilityGrant)
            || ids.contains(&StaticAbilityId::RuleRestriction),
        "expected attached ability grant static ability, got {ids:?}"
    );

    let compiled = compiled_lines(&def).join("\n").to_ascii_lowercase();
    assert!(
        compiled.contains("enchanted creature don't untap during their controllers' untap steps")
            || compiled
                .contains("enchanted creature doesnt untap during its controllers untap step"),
        "expected compiled text to keep attached untap restriction, got {compiled}"
    );
}

#[test]
fn parse_choose_not_to_untap_artifact_line_as_static_ability() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Endoskeleton Untap Variant")
        .card_types(vec![CardType::Artifact])
        .parse_text("You may choose not to untap this artifact during your untap step.")
        .expect("choose-not-to-untap artifact clause should parse as static ability");

    assert!(
        def.spell_effect
            .as_ref()
            .map(|effects| effects.is_empty())
            .unwrap_or(true),
        "expected no spell effects for choose-not-to-untap static line"
    );

    let static_displays: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.display()),
            _ => None,
        })
        .collect();
    let lower_static = static_displays
        .iter()
        .map(|display| display.to_ascii_lowercase())
        .collect::<Vec<_>>();

    assert!(
        lower_static.iter().any(|display| {
            display.contains("you may choose not to untap this artifact during your untap step")
        }),
        "expected choose-not-to-untap static display, got {static_displays:?}"
    );
    assert!(
        !lower_static
            .iter()
            .any(|display| display.contains("unsupported parser line fallback")),
        "expected real parser static ability, not unsupported fallback marker: {static_displays:?}"
    );

    let static_ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        static_ids.contains(&StaticAbilityId::MayChooseNotToUntapDuringUntapStep),
        "expected typed optional-untap static ability, got {static_ids:?}"
    );
    assert!(
        !static_ids.contains(&StaticAbilityId::RuleTextPlaceholder),
        "optional untap should not parse as placeholder static ability: {static_ids:?}"
    );
}

#[test]
fn parse_choose_not_to_untap_line_and_activated_line_without_spurious_untap_effect() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Endoskeleton Pair Variant")
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "You may choose not to untap this artifact during your untap step.\n{2}, {T}: Target creature gets +0/+3 for as long as this artifact remains tapped.",
        )
        .expect("endoskeleton-style untap + activated lines should parse");

    let compiled = compiled_lines(&def).join("\n").to_ascii_lowercase();
    assert!(
        compiled.contains("you may choose not to untap this artifact during your untap step"),
        "expected compiled output to retain choose-not-to-untap static line, got {compiled}"
    );
    assert!(
        !compiled.contains("spell effects: you may untap target artifact"),
        "unexpected untap-target spell effect leak in compiled output: {compiled}"
    );
}

#[test]
fn parse_untap_during_each_other_players_untap_step_as_static_ability() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Seedborn Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Untap all permanents you control during each other player's untap step.")
        .expect_err("unsupported each-other-player untap line should fail loudly");
    let compiled = format!("{err:?}").to_ascii_lowercase();
    assert!(
        compiled.contains("unsupported untap-during-each-other-players-untap-step clause"),
        "expected strict each-other-player untap rejection, got {compiled}"
    );
}

#[test]
fn parse_dark_deal_that_many_minus_one_keeps_prior_effect_reference() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Dark Deal Variant")
        .parse_text("Each player discards all the cards in their hand, then draws that many cards minus one.")
        .expect("dark deal style clause should parse");

    let debug = format!("{:#?}", def.spell_effect);
    assert!(
        debug.contains("EffectValueOffset"),
        "expected draw count to reference prior effect with offset, got {debug}"
    );
    assert!(
        debug.contains("-1"),
        "expected minus one offset in draw count, got {debug}"
    );
}

#[test]
fn parse_where_x_is_count_minus_fixed_preserves_negative_offset() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Ivory Tower Variant")
        .parse_text("At the beginning of your upkeep, you gain X life, where X is the number of cards in your hand minus 4.")
        .expect("where-x count-minus-fixed clause should parse");

    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("expected upkeep triggered ability");
    let gain = triggered
        .effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<GainLifeEffect>())
        .expect("expected gain-life effect");

    let crate::effect::Value::Add(left, right) = &gain.amount else {
        panic!("expected additive life gain amount, got {:?}", gain.amount);
    };
    assert!(
        matches!(
            left.as_ref(),
            crate::effect::Value::CardsInHand(PlayerFilter::You)
        ),
        "expected left side to count cards in hand, got {left:?}"
    );
    assert!(
        matches!(right.as_ref(), crate::effect::Value::Fixed(-4)),
        "expected minus-four offset, got {right:?}"
    );

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("minus 4"),
        "expected compiled text to preserve minus-four offset, got {rendered}"
    );
}

#[test]
fn parse_hellion_eruption_that_many_keeps_prior_effect_reference() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Hellion Eruption Variant")
        .parse_text("Sacrifice all creatures you control, then create that many 4/4 red Hellion creature tokens.")
        .expect("hellion eruption style clause should parse");

    let debug = format!("{:#?}", def.spell_effect);
    assert!(
        debug.contains("CreateTokenEffect"),
        "expected token creation effect, got {debug}"
    );
    assert!(
        debug.contains("EffectValue"),
        "expected token count to reference prior effect result, got {debug}"
    );
}

#[test]
fn parse_can_block_only_creatures_with_flying_static_line() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Cloud Djinn Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Flying\nThis creature can block only creatures with flying.")
        .expect("can-block-only-flying static clause should parse");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();

    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::CanBlockOnlyFlying),
        "expected can-block-only-flying static ability, got {ids:?}"
    );

    let compiled = crate::compiled_text::compiled_lines(&def).join("\n");
    assert!(
        compiled
            .to_ascii_lowercase()
            .contains("can block only creatures with flying"),
        "expected compiled text to keep can-block-only-flying clause, got {compiled}"
    );
}

#[test]
fn parse_cant_be_blocked_by_creatures_with_power_or_less_line() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Arlinn's Wolf Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("This creature can't be blocked by creatures with power 2 or less.")
        .expect("cant-be-blocked-by-power clause should parse");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();

    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::CantBeBlockedByPowerOrLess),
        "expected cant-be-blocked-by-power static ability, got {ids:?}"
    );

    let compiled = crate::compiled_text::compiled_lines(&def).join("\n");
    assert!(
        compiled.to_ascii_lowercase().contains("power 2 or less"),
        "expected compiled text to include power threshold, got {compiled}"
    );
}

#[test]
fn parse_cant_be_blocked_by_creatures_with_power_or_greater_line() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Amrou Kithkin Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("This creature can't be blocked by creatures with power 3 or greater.")
        .expect("cant-be-blocked-by-power clause should parse");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();

    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::CantBeBlockedByPowerOrGreater),
        "expected cant-be-blocked-by-power static ability, got {ids:?}"
    );

    let compiled = crate::compiled_text::compiled_lines(&def).join("\n");
    assert!(
        compiled.to_ascii_lowercase().contains("power 3 or greater"),
        "expected compiled text to include power threshold, got {compiled}"
    );
}

#[test]
fn parse_wandering_wolf_relative_power_blocking_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Wandering Wolf")
        .card_types(vec![CardType::Creature])
        .parse_text("Creatures with power less than this creature's power can't block it.")
        .expect("wandering wolf blocking clause should parse");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();

    assert!(
        ids.contains(
            &crate::static_abilities::StaticAbilityId::CantBeBlockedByLowerPowerThanSource
        ),
        "expected relative-power blocking static ability, got {ids:?}"
    );
    assert!(
        !ids.contains(&crate::static_abilities::StaticAbilityId::Skulk),
        "wandering wolf text must not collapse into skulk, got {ids:?}"
    );

    let compiled = crate::compiled_text::compiled_lines(&def).join("\n");
    assert!(
        compiled
            .to_ascii_lowercase()
            .contains("creatures with power less than this creature's power can't block it"),
        "expected compiled text to preserve wandering wolf wording, got {compiled}"
    );
}

#[test]
fn parse_cant_attack_unless_defending_player_controls_island_line() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Deep-Sea Serpent Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("This creature can't attack unless defending player controls an Island.")
        .expect("cant-attack-unless-defending-controls-island should parse");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();

    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::CantAttackUnlessCondition),
        "expected generic cant-attack-unless condition restriction, got {ids:?}"
    );
    assert!(
        !ids.contains(&StaticAbilityId::RuleTextPlaceholder),
        "defending-player-land-subtype restriction should not emit rule text placeholders, got {ids:?}"
    );

    let compiled = crate::compiled_text::compiled_lines(&def).join("\n");
    assert!(
        compiled
            .to_ascii_lowercase()
            .contains("defending player controls an island")
            || compiled
                .to_ascii_lowercase()
                .contains("defending player controls island"),
        "expected compiled text to include defending-player island condition, got {compiled}"
    );
}

#[test]
fn parse_cant_attack_unless_youve_cast_creature_spell_this_turn_line() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Goblin Cohort Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("This creature can't attack unless you've cast a creature spell this turn.")
        .expect("cant-attack-unless-youve-cast-creature-spell should parse");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();

    assert!(
        ids.contains(
            &crate::static_abilities::StaticAbilityId::CantAttackUnlessControllerCastCreatureSpellThisTurn
        ),
        "expected cast-creature-spell attack restriction, got {ids:?}"
    );
    assert!(
        !ids.contains(&StaticAbilityId::RuleTextPlaceholder),
        "cast-creature-spell attack restriction should not emit rule text placeholders, got {ids:?}"
    );

    let compiled = crate::compiled_text::compiled_lines(&def).join("\n");
    assert!(
        compiled
            .to_ascii_lowercase()
            .contains("can't attack unless you've cast a creature spell this turn")
            || compiled
                .to_ascii_lowercase()
                .contains("cant attack unless youve cast a creature spell this turn"),
        "expected compiled text to include cast-creature-spell condition, got {compiled}"
    );
}

#[test]
fn parse_cant_attack_unless_youve_cast_noncreature_spell_this_turn_line() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Mercurial Spelldancer Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("This creature can't attack unless you've cast a noncreature spell this turn.")
        .expect("cant-attack-unless-youve-cast-noncreature-spell should parse");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();

    assert!(
        ids.contains(
            &crate::static_abilities::StaticAbilityId::CantAttackUnlessControllerCastNonCreatureSpellThisTurn
        ),
        "expected cast-noncreature-spell attack restriction, got {ids:?}"
    );
    assert!(
        !ids.contains(&StaticAbilityId::RuleTextPlaceholder),
        "cast-noncreature-spell attack restriction should not emit rule text placeholders, got {ids:?}"
    );

    let compiled = crate::compiled_text::compiled_lines(&def).join("\n");
    assert!(
        compiled
            .to_ascii_lowercase()
            .contains("can't attack unless you've cast a noncreature spell this turn")
            || compiled
                .to_ascii_lowercase()
                .contains("cant attack unless youve cast a noncreature spell this turn"),
        "expected compiled text to include cast-noncreature-spell condition, got {compiled}"
    );
}

#[test]
fn parse_cant_attack_unless_control_more_creatures_than_defending_player_line() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Bog Hoodlums Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "This creature can't attack unless you control more creatures than defending player.",
        )
        .expect("cant-attack-unless-control-more-creatures should parse");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();

    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::CantAttackUnlessCondition),
        "expected typed cant-attack-unless condition static ability, got {ids:?}"
    );
    assert!(
        !ids.contains(&StaticAbilityId::RuleTextPlaceholder),
        "control-more-creatures restriction should not emit placeholders, got {ids:?}"
    );
}

#[test]
fn parse_cant_attack_unless_defending_player_is_poisoned_line() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Skullsnatcher Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("This creature can't attack unless defending player is poisoned.")
        .expect("cant-attack-unless-defending-player-is-poisoned should parse");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();

    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::CantAttackUnlessCondition),
        "expected typed cant-attack-unless condition static ability, got {ids:?}"
    );
    assert!(
        !ids.contains(&StaticAbilityId::RuleTextPlaceholder),
        "defending-player-poisoned restriction should not emit placeholders, got {ids:?}"
    );
}

#[test]
fn parse_cant_attack_unless_black_or_green_creature_also_attacks_line() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Goblin War Drums Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("This creature can't attack unless a black or green creature also attacks.")
        .expect("cant-attack-unless-black-or-green-creature-also-attacks should parse");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();

    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::CantAttackUnlessCondition),
        "expected typed cant-attack-unless condition static ability, got {ids:?}"
    );
    assert!(
        !ids.contains(&StaticAbilityId::RuleTextPlaceholder),
        "also-attacks restriction should not emit placeholders, got {ids:?}"
    );
}

#[test]
fn parse_cant_attack_unless_sacrifice_a_land_line() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Exalted Dragon Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("This creature can't attack unless you sacrifice a land.")
        .expect("cant-attack-unless-sacrifice-land should parse");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();

    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::CantAttackUnlessCondition),
        "expected typed cant-attack-unless condition static ability, got {ids:?}"
    );
    assert!(
        !ids.contains(&StaticAbilityId::RuleTextPlaceholder),
        "sacrifice-land restriction should not emit placeholders, got {ids:?}"
    );
}

#[test]
fn parse_cant_attack_unless_sacrifice_two_islands_line() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Leviathan Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("This creature can't attack unless you sacrifice two islands.")
        .expect("cant-attack-unless-sacrifice-two-islands should parse");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();

    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::CantAttackUnlessCondition),
        "expected typed cant-attack-unless condition static ability, got {ids:?}"
    );
    assert!(
        !ids.contains(&StaticAbilityId::RuleTextPlaceholder),
        "sacrifice-two-islands restriction should not emit placeholders, got {ids:?}"
    );
}

#[test]
fn parse_cant_attack_unless_pay_per_plus_one_plus_one_counter_line() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Phyrexian Marauder Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("This creature can't attack unless you pay {1} for each +1/+1 counter on it.")
        .expect("cant-attack-unless-pay-per-counter should parse");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();

    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::CantAttackUnlessCondition),
        "expected typed cant-attack-unless condition static ability, got {ids:?}"
    );
    assert!(
        !ids.contains(&StaticAbilityId::RuleTextPlaceholder),
        "pay-per-counter restriction should not emit placeholders, got {ids:?}"
    );
}

#[test]
fn parse_cant_attack_unless_defending_player_is_the_monarch_line() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Crown-Hunter Hireling Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("This creature can't attack unless defending player is the monarch.")
        .expect("cant-attack-unless-defending-player-is-the-monarch should parse");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();

    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::CantAttackUnlessCondition),
        "expected typed cant-attack-unless condition static ability, got {ids:?}"
    );
    assert!(
        !ids.contains(&StaticAbilityId::RuleTextPlaceholder),
        "monarch restriction should not emit placeholders, got {ids:?}"
    );
}

#[test]
fn parse_collective_restraint_domain_attack_tax_line() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Collective Restraint Variant")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "Creatures can't attack you unless their controller pays {X} for each creature they control that's attacking you, where X is the number of basic land types among lands you control.",
        )
        .expect("collective restraint domain attack tax line should parse");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();

    assert!(
        ids.contains(
            &crate::static_abilities::StaticAbilityId::CantAttackYouUnlessControllerPaysPerAttackerBasicLandTypesAmongLandsYouControl
        ),
        "expected collective-restraint attack tax static ability, got {ids:?}"
    );
    assert!(
        !ids.contains(&StaticAbilityId::RuleTextPlaceholder),
        "collective-restraint line should not emit rule text placeholders, got {ids:?}"
    );

    let compiled = crate::compiled_text::compiled_lines(&def)
        .join("\n")
        .to_ascii_lowercase();
    assert!(
        compiled.contains(
            "unless their controller pays {x} for each creature they control thats attacking you"
        ) || compiled.contains(
            "unless their controller pays {x} for each creature they control that's attacking you"
        ),
        "expected compiled text to include collective-restraint tax clause, got {compiled}"
    );
    assert!(
        compiled.contains("basic land types among lands you control"),
        "expected compiled text to include domain clause, got {compiled}"
    );
}

#[test]
fn parse_fixed_attack_tax_line() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Ghostly Prison Variant")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "Creatures can't attack you unless their controller pays {2} for each creature they control that's attacking you.",
        )
        .expect("fixed attack tax line should parse");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();

    assert!(
        ids.contains(
            &crate::static_abilities::StaticAbilityId::CantAttackYouUnlessControllerPaysPerAttacker
        ),
        "expected fixed attack-tax static ability, got {ids:?}"
    );
    assert!(
        !ids.contains(&StaticAbilityId::RuleTextPlaceholder),
        "fixed attack-tax line should not emit rule text placeholders, got {ids:?}"
    );
}

#[test]
fn parse_morph_keyword_line() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Morph Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Morph {3}{R}")
        .expect("morph keyword line should parse");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();

    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::Morph),
        "expected morph static ability, got {ids:?}"
    );

    let compiled = compiled_lines(&def).join(" ");
    assert!(
        compiled.to_ascii_lowercase().contains("morph {3}{r}"),
        "expected morph line in compiled text, got {compiled}"
    );
}

#[test]
fn parse_megamorph_keyword_line() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Megamorph Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Megamorph {5}{G}")
        .expect("megamorph keyword line should parse");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();

    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::Megamorph),
        "expected megamorph static ability, got {ids:?}"
    );

    let compiled = compiled_lines(&def).join(" ");
    assert!(
        compiled.to_ascii_lowercase().contains("megamorph {5}{g}"),
        "expected megamorph line in compiled text, got {compiled}"
    );
}

#[test]
fn parse_morph_keyword_line_with_trailing_clause_fails() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Morph Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Morph {3}{R} reveal this card")
        .expect_err("morph keyword with trailing clause should fail");

    let message = format!("{err:?}").to_ascii_lowercase();
    assert!(
        message.contains("unsupported trailing morph clause"),
        "expected trailing morph clause parse error, got {message}"
    );
}

#[test]
fn parse_banding_keyword_line() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Banding Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Banding")
        .expect("banding keyword line should parse as marker");

    let compiled = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        compiled.contains("banding"),
        "expected banding marker in compiled output, got {compiled}"
    );
}

#[test]
fn parse_umbra_armor_keyword_line_lowers_to_static_ability() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Umbra Armor Variant")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Aura])
        .parse_text("Umbra armor")
        .expect("umbra armor keyword line should parse");

    let compiled = compiled_lines(&def).join(" ");
    assert!(
        compiled.contains("Umbra armor"),
        "expected umbra armor text in compiled output, got {compiled}"
    );
    let debug = format!("{def:?}");
    assert!(
        !debug.contains("KeywordMarker") && !debug.contains("MarkerText"),
        "umbra armor should lower to a real static ability, got {debug}"
    );
}

#[test]
fn parse_filter_power_numeric_comparison_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Power Filter Variant")
        .parse_text("Destroy target creature with power 2 or less.")
        .expect("numeric power comparison should parse");

    let effects = def.spell_effect.as_ref().expect("spell effects");
    let debug = format!("{effects:?}");
    assert!(
        debug.contains("power: Some(LessThanOrEqual(2))"),
        "expected parsed power comparison constraint, got {debug}"
    );
}

#[test]
fn parse_counter_spell_with_power_or_toughness_filter() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Stern Scolding Variant")
        .parse_text("Counter target creature spell with power or toughness 2 or less.")
        .expect("power-or-toughness spell filter should parse");

    let effects = def.spell_effect.as_ref().expect("spell effects");
    let debug = format!("{effects:?}");
    assert!(
        debug.contains("any_of:")
            && debug.contains("power: Some(LessThanOrEqual(2))")
            && debug.contains("toughness: Some(LessThanOrEqual(2))"),
        "expected disjunctive power/toughness spell filter, got {debug}"
    );
}

#[test]
fn parse_filter_dynamic_power_comparison_fails_instead_of_partial_parse() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Dynamic Power Filter Variant")
        .parse_text("Exile target creature with power greater than or equal to your life total.")
        .expect_err("unsupported dynamic comparison should fail parse");
    let message = format!("{err:?}");
    assert!(
        message.contains("unsupported dynamic power comparison operand")
            || message.contains("unsupported arithmetic power comparison"),
        "expected strict dynamic-power comparison error, got {message}"
    );
}

#[test]
fn parse_return_up_to_x_target_creatures_preserves_dynamic_optional_count() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Dynamic Return Count Variant")
        .parse_text(
            "Return up to X target creatures to their owners' hands, where X is one plus the number of cards named Aether Burst in all graveyards as you cast this spell.",
        )
        .expect("up-to-X target return clause should parse");
    let message = format!("{:?}", def.spell_effect);
    assert!(
        message.contains("dynamic_x: true") && message.contains("up_to_x: true"),
        "expected optional dynamic target-count in compiled effect, got {message}"
    );
}

#[test]
fn parse_destroy_up_to_x_other_targets_fails_instead_of_partial_destroy() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Dynamic Destroy Count Variant")
            .parse_text(
                "Destroy target creature and up to X other target creatures, where X is the number of Attractions you've visited this turn.",
            )
            .expect_err("unsupported dynamic multi-target destroy should fail parse");
    let message = format!("{err:?}");
    assert!(
        message.contains("unsupported multi-target destroy clause")
            || message.contains("unsupported dynamic or missing target count after 'up to'")
            || message.contains("unsupported where-x clause"),
        "expected strict multi-target destroy parse error, got {message}"
    );
}

#[test]
fn parse_loses_all_abilities_and_becomes_effect_fails_instead_of_partial_parse() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Lose Abilities Becomes Effect Variant")
            .parse_text(
                "Until end of turn, target creature loses all abilities and becomes a blue Frog with base power and toughness 1/1.",
            )
            .expect_err("unsupported lose-all-abilities+becomes effect should fail parse");
    let message = format!("{err:?}");
    assert!(
        message.contains("unsupported loses-all-abilities with becomes clause")
            || message.contains("unsupported lose-all-abilities static becomes clause"),
        "expected strict loses-all-abilities+becomes effect parse error, got {message}"
    );
}

#[test]
fn parse_loses_all_abilities_and_becomes_static_fails_instead_of_partial_parse() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Lose Abilities Becomes Static Variant")
            .parse_text(
                "Each noncreature artifact loses all abilities and becomes an artifact creature with power and toughness each equal to its mana value.",
            )
            .expect_err("unsupported lose-all-abilities+becomes static should fail parse");
    let message = format!("{err:?}");
    assert!(
        message.contains("unsupported lose-all-abilities static becomes clause"),
        "expected strict lose-all-abilities+becomes static parse error, got {message}"
    );
}

#[test]
fn parse_each_player_exile_sacrifice_return_this_way_fails_instead_of_partial_parse() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Living Death Variant")
            .parse_text(
                "Each player exiles all creature cards from their graveyard, then sacrifices all creatures they control, then puts all cards they exiled this way onto the battlefield.",
            )
            .expect_err("unsupported each-player exile/sacrifice/return should fail parse");
    let message = format!("{err:?}");
    assert!(
        message.contains("unsupported each-player exile/sacrifice/return-this-way clause"),
        "expected strict each-player exile/sacrifice/return parse error, got {message}"
    );
}

#[test]
fn parse_combat_damage_to_creature_trigger_parses_with_damaged_creature_reference() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Combat Damage Creature Trigger Variant")
        .parse_text(
            "Whenever this creature deals combat damage to a creature, you gain 2 life unless that creature's controller pays {2}.",
        )
        .expect("combat-damage-to-creature trigger should parse");
    let joined = compiled_lines(&def).join(" ").to_lowercase();
    assert!(
        joined.contains("deals combat damage to creature"),
        "expected combat-damage-to-creature trigger text, got {joined}"
    );
    assert!(
        joined.contains("unless its controller pays {2}")
            || joined.contains("unless that object's controller pays {2}"),
        "expected damaged-creature controller reference to be preserved, got {joined}"
    );
}

#[test]
fn parse_target_player_loses_the_game_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Lose Game Target Variant")
        .parse_text("{W}{W}, {T}: Target player loses the game.")
        .expect("target-player lose-game clause should parse");
    let joined = compiled_lines(&def).join(" ").to_lowercase();
    assert!(
        joined.contains("target player loses the game"),
        "expected target-player lose-game text, got {joined}"
    );
}

#[test]
fn parse_trigger_target_opponent_creates_treasure_tokens() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Target Opponent Creates Token Variant")
        .parse_text("When this creature dies, target opponent creates two Treasure tokens.")
        .expect("target-opponent create-token trigger should parse");
    let joined = compiled_lines(&def).join(" ").to_lowercase();
    assert!(
        joined.contains("create two treasure tokens under target opponent's control"),
        "expected targeted opponent token creation text, got {joined}"
    );
}

#[test]
fn parse_trigger_target_opponent_may_draw_card() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Target Opponent May Draw Variant")
        .parse_text("At the beginning of your end step, target opponent may draw a card.")
        .expect("target-opponent may-draw trigger should parse");
    let joined = compiled_lines(&def).join(" ").to_lowercase();
    assert!(
        joined.contains("target opponent may") && joined.contains("draw"),
        "expected target-opponent may-draw text, got {joined}"
    );
}

#[test]
fn parse_trigger_it_connives_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Connive It Variant")
        .parse_text("When this creature enters, it connives.")
        .expect("it-connives trigger clause should parse");
    let joined = compiled_lines(&def).join(" ").to_lowercase();
    assert!(
        joined.contains("connive"),
        "expected connive text to be preserved, got {joined}"
    );
}

#[test]
fn parse_reveal_hand_choose_card_from_it_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Reveal Hand From It Variant")
        .parse_text(
            "Target opponent reveals their hand. You choose a nonland card from it and exile that card.",
        )
        .expect("reveal-hand choose-from-it chain should parse");
    let joined = format!("{:#?}", def.spell_effect).to_lowercase();
    assert!(
        joined.contains("lookathandeffect")
            && joined.contains("chooseobjectseffect")
            && joined.contains("exileeffect"),
        "expected reveal-hand choose-then-exile effect chain, got {joined}"
    );
}

#[test]
fn parse_trigger_target_opponent_gains_control_of_it_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Gain Control Of It Variant")
        .parse_text("When this creature enters, target opponent gains control of it.")
        .expect("gain-control-of-it trigger clause should parse");
    let joined = compiled_lines(&def).join(" ").to_lowercase();
    assert!(
        joined.contains("target opponent gains control"),
        "expected gain-control text to be preserved, got {joined}"
    );
}

#[test]
fn parse_trigger_destroy_it_then_cant_regenerate_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Destroy It No Regen Variant")
        .parse_text(
            "Whenever this creature deals combat damage to a creature, destroy it. It can't be regenerated.",
        )
        .expect("destroy-it then cant-regenerate trigger should parse");
    let joined = compiled_lines(&def).join(" ").to_lowercase();
    assert!(
        joined.contains("destroy") && joined.contains("can't be regenerated"),
        "expected destroy/no-regeneration sequence to be preserved, got {joined}"
    );
}

#[test]
fn parse_each_player_multi_step_then_clause_fails_instead_of_partial_parse() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Each Player Multi-step Variant")
            .parse_text(
                "Each player loses X life, discards X cards, sacrifices X creatures of their choice, then sacrifices X lands of their choice.",
            )
            .expect_err("unsupported each-player then-clause should fail parse");
    let message = format!("{err:?}");
    assert!(
        message.contains("unsupported multi-step each-player clause with 'then'")
            || message.contains("unsupported each-player lose/discard/sacrifice chain clause"),
        "expected strict each-player multi-step parse error, got {message}"
    );
}

#[test]
fn parse_return_transformed_clause_fails_instead_of_immediate_return() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Transformed Return Variant")
        .parse_text(
            "When this creature dies, return it to the battlefield transformed under your control.",
        )
        .expect_err("unsupported transformed return should fail parse");
    let message = format!("{err:?}");
    assert!(
        message.contains("unsupported transformed return clause")
            || message.contains("unsupported triggered line"),
        "expected strict transformed-return parse error, got {message}"
    );
}

#[test]
fn parse_return_next_upkeep_clause_fails_instead_of_immediate_return() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Next Upkeep Return Variant")
            .parse_text(
                "When this creature dies, return it to the battlefield tapped under its owner's control at the beginning of their next upkeep.",
            )
            .expect_err("unsupported delayed return timing should fail parse");
    let message = format!("{err:?}");
    assert!(
        message.contains("unsupported delayed return timing clause")
            || message.contains("unsupported triggered line"),
        "expected strict delayed-return parse error, got {message}"
    );
}

#[test]
fn parse_exile_name_and_target_supports_exiling_source_and_target() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Mangara Variant")
        .parse_text("{T}: Exile Mangara of Corondor and target permanent.")
        .expect("named-source + target exile should parse");
    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        (rendered.contains("exile this creature") || rendered.contains("exile this permanent"))
            && rendered.contains("target permanent"),
        "expected exile of source and target permanent, got {rendered}"
    );
}

#[test]
fn parse_target_opponent_exiles_card_from_their_hand_uses_hand_choice() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Skullcap Snail Variant")
        .parse_text("Target opponent exiles a card from their hand.")
        .expect("parse targeted hand exile");

    let effects = def.spell_effect.as_ref().expect("spell effects");
    let debug = format!("{effects:?}");
    assert!(
        debug.contains("TargetOnlyEffect"),
        "expected target-opponent context setup, got {debug}"
    );
    assert!(
        debug.contains("ChooseObjectsEffect"),
        "expected choose-from-hand effect, got {debug}"
    );
    assert!(
        debug.contains("filter: ObjectFilter { zone: Some(Hand)"),
        "expected choose-from-hand filter zone, got {debug}"
    );
    assert!(
        debug.contains("chooser: Target(Opponent)"),
        "expected target opponent chooser, got {debug}"
    );
    assert!(
        debug.contains("ExileEffect") && debug.contains("Tagged(TagKey(\"exiled_0\"))"),
        "expected exile of chosen tagged card, got {debug}"
    );
}

#[test]
fn parse_each_opponent_exiles_card_from_their_hand_uses_iterated_chooser() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Each Opponent Hand Exile Variant")
        .parse_text("Each opponent exiles a card from their hand.")
        .expect("parse each-opponent hand exile");

    let effects = def.spell_effect.expect("spell effects");
    let debug = format!("{effects:?}");
    assert!(
        debug.contains("ForPlayersEffect"),
        "expected foreach-opponent wrapper, got {debug}"
    );
    assert!(
        debug.contains("chooser: IteratedPlayer"),
        "expected iterated chooser for each-opponent hand exile, got {debug}"
    );
}

#[test]
fn parse_eldrazi_spawn_reminder_sentence_is_not_immediate_sacrifice() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Dread Drone Variant")
            .parse_text(
                "When this creature enters, create two 0/1 colorless Eldrazi Spawn creature tokens. They have \"Sacrifice this creature: Add {C}.\"",
            )
            .expect("parse eldrazi spawn reminder");

    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("expected triggered ability");
    assert_eq!(
        triggered.effects.len(),
        1,
        "spawn reminder must not compile as a second immediate effect"
    );

    let lines = crate::compiled_text::compiled_lines(&def);
    let joined = lines.join("\n");
    assert!(
        joined.contains("Eldrazi Spawn creature token"),
        "expected spawn token in compiled text, got {joined}"
    );
    assert!(
        !joined.contains("sacrifice"),
        "spawn reminder must not add immediate sacrifice clause, got {joined}"
    );
}

#[test]
fn parse_eldrazi_scion_reminder_sentence_is_not_immediate_sacrifice() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Scion Variant")
            .parse_text(
                "When this creature enters, create a 1/1 colorless Eldrazi Scion creature token. It has \"Sacrifice this creature: Add {C}.\"",
            )
            .expect("parse eldrazi scion reminder");

    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("expected triggered ability");
    let effects = &triggered.effects;
    assert_eq!(
        effects.len(),
        1,
        "scion reminder must not compile as a second immediate effect"
    );
    let debug = format!("{effects:?}");
    assert!(
        debug.contains("Eldrazi Scion"),
        "expected scion token creation, got {debug}"
    );
}

#[test]
fn parse_spawn_scion_mana_reminder_without_context_fails_strictly() {
    let _err = CardDefinitionBuilder::new(CardId::new(), "Standalone Spawn Reminder")
        .parse_text("They have \"Sacrifice this creature: Add {C}.\"")
        .expect_err("standalone token reminder should fail");
}

#[test]
fn parse_growth_spasm_style_spawn_reminder_stays_statement_not_static() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Growth Spasm Variant")
            .parse_text(
                "Search your library for a basic land card, put it onto the battlefield tapped, then shuffle. Create a 0/1 colorless Eldrazi Spawn creature token. It has \"Sacrifice this token: Add {C}.\"",
            )
            .expect("growth spasm line should parse as statement");

    assert!(
        def.spell_effect.is_some(),
        "expected spell effects for spell text"
    );
    assert!(
        def.abilities.is_empty(),
        "statement text must not be misclassified as static ability"
    );
}

#[test]
fn parse_convoked_connive_clause_compiles_to_tagged_connive_iteration() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Lethal Scheme Variant")
        .parse_text("Destroy target creature or planeswalker. Each creature that convoked this spell connives.")
        .expect("parse convoked connive clause");

    let effects = def.spell_effect.as_ref().expect("expected spell effects");
    let debug = format!("{effects:?}");
    assert!(
        debug.contains("convoked_this_spell"),
        "expected convoked tag reference, got {debug}"
    );
    assert!(
        debug.contains("ConniveEffect"),
        "expected connive effect in compiled spell effects, got {debug}"
    );
}

#[test]
fn parse_convoked_it_creature_etb_reference_compiles_to_tagged_filter() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Venerated Loxodon Variant")
        .parse_text(
            "Convoke\nWhen this creature enters, put a +1/+1 counter on each creature that convoked it.",
        )
        .expect("parse convoked-it reference");

    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("expected triggered ability");
    let debug = format!("{:?}", triggered.effects);
    assert!(
        debug.contains("convoked_this_spell"),
        "expected convoked tag reference in effects, got {debug}"
    );
}

#[test]
fn render_mother_of_runes_compacts_protection_choice_text() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Mother of Runes Variant")
            .parse_text(
                "{T}: Target creature you control gains protection from the color of your choice until end of turn.",
            )
            .expect("mother of runes line should parse");
    let lines = crate::compiled_text::compiled_lines(&def);
    let joined = lines.join("\n");
    assert!(
        joined.contains("gains protection from the color of your choice until end of turn"),
        "expected compact protection-choice rendering, got {joined}"
    );
}

#[test]
fn render_giver_of_runes_compacts_colorless_or_color_choice_text() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Giver of Runes Variant")
            .parse_text(
                "{T}: Another target creature you control gains protection from colorless or from the color of your choice until end of turn.",
            )
            .expect("giver of runes line should parse");
    let lines = crate::compiled_text::compiled_lines(&def);
    let joined = lines.join("\n");
    assert!(
        joined.contains(
            "gains protection from colorless or from the color of your choice until end of turn"
        ),
        "expected compact colorless-or-color protection rendering, got {joined}"
    );
}

#[test]
fn render_draw_for_each_creature_uses_oracle_like_wording() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Collective Unconscious Variant")
        .parse_text("Draw a card for each creature you control.")
        .expect("draw-for-each should parse");
    let lines = crate::compiled_text::compiled_lines(&def);
    let joined = lines.join("\n");
    assert!(
        joined.contains("draw a card for each creature you control"),
        "expected oracle-like draw-for-each wording, got {joined}"
    );
}

#[test]
fn render_draw_for_each_subtype_uses_oracle_like_wording() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Sea Gate Loremaster Variant")
        .parse_text("{T}: Draw a card for each Ally you control.")
        .expect("subtype draw-for-each should parse");
    let lines = crate::compiled_text::compiled_lines(&def);
    let joined = lines.join("\n");
    assert!(
        joined.contains("draw a card for each Ally you control"),
        "expected subtype draw-for-each wording, got {joined}"
    );
}

#[test]
fn render_create_treasure_token_uses_compact_wording() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Glittermonger Variant")
        .parse_text("{T}: Create a Treasure token.")
        .expect("treasure token creation should parse");
    let lines = crate::compiled_text::compiled_lines(&def);
    let joined = lines.join("\n");
    assert!(
        joined.contains("Create a Treasure token"),
        "expected compact treasure token wording, got {joined}"
    );
}

#[test]
fn render_create_map_token_uses_compact_wording() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Spyglass Siren Variant")
        .parse_text("When this creature enters, create a Map token.")
        .expect("map token creation should parse");
    let lines = crate::compiled_text::compiled_lines(&def);
    let joined = lines.join("\n");
    assert!(
        joined.contains("create a Map artifact token")
            || joined.contains("Create a Map artifact token"),
        "expected compact map token wording, got {joined}"
    );
    assert!(
        !joined
            .to_ascii_lowercase()
            .contains("unsupported parser line fallback"),
        "map token parsing should not rely on unsupported fallback marker, got {joined}"
    );
}

#[test]
fn render_create_lander_token_uses_compact_wording() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Galactic Wayfarer Variant")
        .parse_text("When this creature enters, create a Lander token.")
        .expect("lander token creation should parse");
    let lines = crate::compiled_text::compiled_lines(&def);
    let joined = lines.join("\n");
    assert!(
        joined.contains("create a Lander artifact token")
            || joined.contains("Create a Lander artifact token"),
        "expected compact lander token wording, got {joined}"
    );
    assert!(
        !joined
            .to_ascii_lowercase()
            .contains("unsupported parser line fallback"),
        "lander token parsing should not rely on unsupported fallback marker, got {joined}"
    );
}

#[test]
fn render_create_junk_token_uses_expected_wording() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Junk Maker Variant")
        .parse_text("When this creature enters, create a Junk token.")
        .expect("junk token creation should parse");
    let lines = crate::compiled_text::compiled_lines(&def);
    let joined = lines.join("\n");
    assert!(
        joined.contains("create a Junk artifact token")
            || joined.contains("Create a Junk artifact token"),
        "expected junk token rendering, got {joined}"
    );
    assert!(
        joined
            .to_ascii_lowercase()
            .contains("activate only as a sorcery"),
        "expected junk token rules text to include sorcery restriction, got {joined}"
    );
    assert!(
        !joined
            .to_ascii_lowercase()
            .contains("unsupported parser line fallback"),
        "junk token parsing should not rely on unsupported fallback marker, got {joined}"
    );
}

#[test]
fn parse_create_supported_role_tokens_attached_to_creature() {
    let role_names = [
        "Young Hero Role",
        "Monster Role",
        "Sorcerer Role",
        "Royal Role",
        "Cursed Role",
    ];

    for role_name in role_names {
        let text = format!("Create a {role_name} token attached to target creature you control.");
        let def = CardDefinitionBuilder::new(CardId::new(), format!("{role_name} Variant"))
            .parse_text(&text)
            .unwrap_or_else(|err| panic!("{role_name} token creation should parse: {err:?}"));
        let joined = crate::compiled_text::compiled_lines(&def).join("\n");
        assert!(
            joined
                .to_ascii_lowercase()
                .contains(&role_name.to_ascii_lowercase()),
            "expected compiled text to include role token name '{role_name}', got {joined}"
        );
        assert!(
            !joined
                .to_ascii_lowercase()
                .contains("unsupported parser line fallback"),
            "{role_name} token creation should not rely on unsupported fallback marker, got {joined}"
        );
    }
}

#[test]
fn render_create_gold_token_uses_compact_wording() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Gild Variant")
        .parse_text("Exile target creature. Create a Gold token.")
        .expect("gold token creation should parse");
    let lines = crate::compiled_text::compiled_lines(&def);
    let joined = lines.join("\n");
    assert!(
        joined.contains("Create a Gold token") || joined.contains("create a Gold token"),
        "expected compact gold token wording, got {joined}"
    );
    assert!(
        !joined
            .to_ascii_lowercase()
            .contains("unsupported parser line fallback"),
        "gold token parsing should not rely on unsupported fallback marker, got {joined}"
    );
}

#[test]
fn render_create_shard_token_includes_scry_draw_ability() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Niko Variant")
        .parse_text("When this permanent enters, create two Shard tokens.")
        .expect("shard token creation should parse");
    let lines = crate::compiled_text::compiled_lines(&def);
    let joined = lines.join("\n");
    let lower = joined.to_ascii_lowercase();
    assert!(
        lower.contains("shard"),
        "expected shard token wording in compiled text, got {joined}"
    );
    assert!(
        lower.contains("scry 1") && lower.contains("draw a card"),
        "expected shard token rules text to include scry and draw, got {joined}"
    );
    assert!(
        !lower.contains("unsupported parser line fallback"),
        "shard token parsing should not rely on unsupported fallback marker, got {joined}"
    );
}

#[test]
fn render_create_walker_token_uses_expected_characteristics() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Walker Maker Variant")
        .parse_text("Create three Walker tokens.")
        .expect("walker token creation should parse");
    let lines = crate::compiled_text::compiled_lines(&def);
    let joined = lines.join("\n");
    let lower = joined.to_ascii_lowercase();
    assert!(
        lower.contains("walker") && lower.contains("2/2") && lower.contains("zombie"),
        "expected walker token characteristics in compiled text, got {joined}"
    );
    assert!(
        !lower.contains("unsupported parser line fallback"),
        "walker token parsing should not rely on unsupported fallback marker, got {joined}"
    );
}

#[test]
fn oracle_like_lines_compact_each_opponent_discard() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Burglar Rat Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("When this creature enters, each opponent discards a card.")
        .expect("etb discard should parse");
    let lines = crate::compiled_text::oracle_like_lines(&def);
    let joined = lines.join("\n").to_ascii_lowercase();
    assert!(
        joined.contains("when this creature enters"),
        "expected source subject to stay creature-like, got {joined}"
    );
    assert!(
        joined.contains("each opponent discards a card"),
        "expected compact each-opponent discard wording, got {joined}"
    );
}

#[test]
fn oracle_like_lines_compact_you_mill_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Armored Skaab Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("When this creature enters, mill four cards.")
        .expect("etb mill should parse");
    let lines = crate::compiled_text::oracle_like_lines(&def);
    let joined = lines.join("\n").to_ascii_lowercase();
    assert!(
        joined.contains("when this creature enters"),
        "expected source subject to stay creature-like, got {joined}"
    );
    assert!(
        joined.contains("mill 4 cards")
            || joined.contains("mill four cards")
            || joined.contains("you mill four cards"),
        "expected compact mill wording without explicit 'you', got {joined}"
    );
}

#[test]
fn oracle_like_lines_compact_cant_block_this_turn() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Lambholt Harrier Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("{3}{R}: Target creature can't block this turn.")
        .expect("can't-block activated ability should parse");
    let lines = crate::compiled_text::oracle_like_lines(&def);
    let joined = lines.join("\n").to_ascii_lowercase();
    assert!(
        joined.contains("target creature can't block this turn"),
        "expected oracle-like can't-block wording, got {joined}"
    );
    assert!(
        !joined.contains("choose target creature"),
        "target-only preface should be compacted away, got {joined}"
    );
}

#[test]
fn oracle_like_lines_compact_prevent_damage_source_wording() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Ordruun Commando Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "{W}: Prevent the next 1 damage that would be dealt to this creature this turn.",
        )
        .expect("prevent damage activated ability should parse");
    let lines = crate::compiled_text::oracle_like_lines(&def);
    let joined = lines.join("\n").to_ascii_lowercase();
    assert!(
        joined.contains("prevent the next 1 damage that would be dealt to this creature this turn"),
        "expected oracle-like prevention wording, got {joined}"
    );
}

#[test]
fn oracle_like_lines_compact_lands_have_tap_for_any_color() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Joiner Adept Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Lands you control have \"{T}: Add one mana of any color.\"")
        .expect("mana-grant static ability should parse");
    let lines = crate::compiled_text::oracle_like_lines(&def);
    let joined = lines.join("\n").to_ascii_lowercase();
    assert!(
        joined.contains("lands you control have \"{t}: add one mana of any color\"")
            || joined.contains("lands you control have \"{t}: add one mana of any color.\""),
        "expected quoted tap-mana grant wording, got {joined}"
    );
}

#[test]
fn oracle_like_lines_preserve_negative_zero_toughness_delta() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Cumber Stone Variant")
        .card_types(vec![CardType::Enchantment])
        .parse_text("Creatures your opponents control get -1/-0.")
        .expect("static debuff should parse");
    let lines = crate::compiled_text::oracle_like_lines(&def);
    let joined = lines.join("\n").to_ascii_lowercase();
    assert!(
        joined.contains("get -1/-0"),
        "expected oracle-like -1/-0 rendering, got {joined}"
    );
    assert!(
        joined.contains("creatures your opponents control get -1/-0"),
        "expected oracle-like opponent-controller wording, got {joined}"
    );
}

#[test]
fn parse_destroy_target_creature_or_vehicle_uses_union_filter() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Daring Demolition Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Destroy target creature or Vehicle.")
        .expect("creature-or-vehicle targeting should parse");

    let debug = format!("{:#?}", def.spell_effect);
    assert!(
        debug.contains("DestroyEffect"),
        "expected destroy effect, got {debug}"
    );
    assert!(
        debug.contains("type_or_subtype_union: true"),
        "expected type/subtype union for creature-or-vehicle targeting, got {debug}"
    );
    assert!(
        debug.contains("card_types: [") && debug.contains("Creature"),
        "expected creature card type selector, got {debug}"
    );
    assert!(
        debug.contains("subtypes: [") && debug.contains("Vehicle"),
        "expected Vehicle subtype selector, got {debug}"
    );

    let joined = crate::compiled_text::oracle_like_lines(&def).join("\n");
    assert!(
        joined.contains("Destroy target creature or Vehicle"),
        "expected oracle-like creature-or-Vehicle rendering, got {joined}"
    );
}

#[test]
fn render_multi_sacrifice_cost_uses_compact_filter_text() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Keldon Arsonist Variant")
        .parse_text("{1}, Sacrifice two lands: Destroy target land.")
        .expect("multi-sacrifice activated cost should parse");
    let lines = crate::compiled_text::compiled_lines(&def);
    let joined = lines.join("\n").to_ascii_lowercase();
    assert!(
        joined.contains("sacrifice two lands"),
        "expected compact multi-sacrifice rendering, got {joined}"
    );
}

#[test]
fn render_multi_sacrifice_artifacts_cost_uses_compact_filter_text() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Krark-Clan Engineers Variant")
        .parse_text("{R}, Sacrifice two artifacts: Destroy target artifact.")
        .expect("multi-artifact-sacrifice activated cost should parse");
    let lines = crate::compiled_text::compiled_lines(&def);
    let joined = lines.join("\n").to_ascii_lowercase();
    assert!(
        joined.contains("sacrifice two artifacts"),
        "expected compact multi-artifact sacrifice rendering, got {joined}"
    );
}

#[test]
fn render_sacrifice_artifact_or_land_cost_uses_oracle_article() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Scrapchomper Variant")
        .parse_text("{1}{R}, {T}, Sacrifice an artifact or land: Draw a card.")
        .expect("artifact-or-land sacrifice activated cost should parse");
    let lines = crate::compiled_text::compiled_lines(&def);
    let joined = lines.join("\n").to_ascii_lowercase();
    assert!(
        joined.contains("sacrifice an artifact or land"),
        "expected oracle-like sacrifice article rendering, got {joined}"
    );
}

#[test]
fn render_return_from_graveyard_uses_from_your_graveyard() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Reanimate Variant")
        .parse_text("Return target creature card from your graveyard to the battlefield.")
        .expect("return-from-graveyard spell should parse");
    let lines = compiled_lines(&def);
    let spell_line = lines
        .iter()
        .find(|line| line.starts_with("Spell effects:"))
        .expect("expected spell effects line");
    assert!(
        spell_line.contains("Return target creature card from your graveyard to the battlefield"),
        "expected oracle-like return text, got {spell_line}"
    );
}

#[test]
fn render_return_to_hand_from_your_graveyard_uses_oracle_wording() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Raise Dead Variant")
        .parse_text("Return target creature card from your graveyard to your hand.")
        .expect("return-to-hand-from-graveyard spell should parse");
    let joined = crate::compiled_text::oracle_like_lines(&def).join("\n");
    assert!(
        joined.contains("Return target creature card from your graveyard to your hand"),
        "expected oracle-like return-to-hand wording, got {joined}"
    );
}

#[test]
fn render_graveyard_self_return_activated_uses_this_card_wording() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Sanitarium Skeleton Variant")
        .parse_text("{2}{B}: Return this card from your graveyard to your hand.")
        .expect("graveyard self-return activated ability should parse");
    let joined = crate::compiled_text::oracle_like_lines(&def).join("\n");
    assert!(
        joined.contains("{2}{B}: Return this card from your graveyard to your hand"),
        "expected oracle-like graveyard self-return wording, got {joined}"
    );
}

#[test]
fn render_enchanted_tap_untap_compacts_tag_prelude() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Freed from the Real Variant")
        .parse_text(
            "Enchant creature\n{U}: Tap enchanted creature.\n{U}: Untap enchanted creature.",
        )
        .expect("enchanted tap/untap aura should parse");
    let joined = crate::compiled_text::oracle_like_lines(&def).join("\n");
    let lower = joined.to_ascii_lowercase();
    assert!(
        (lower.contains("tap enchanted creature") || lower.contains("tap an enchanted creature"))
            && (lower.contains("untap enchanted creature")
                || lower.contains("untap an enchanted creature")),
        "expected compact enchanted tap/untap wording, got {joined}"
    );
    assert!(
        !lower.contains("tag the object attached to this source")
            && !lower.contains("the tagged object 'enchanted'"),
        "internal enchanted tag prelude should not leak into oracle-like lines: {joined}"
    );
}

#[test]
fn parse_draw_then_put_two_cards_from_hand_on_top_preserves_count() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Brainstorm Variant")
        .parse_text("Draw three cards, then put two cards from your hand on top of your library in any order.")
        .expect("draw-then-put-two-cards clause should parse");

    let rendered = oracle_like_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("draw three cards")
            && rendered.contains("put two cards from your hand on top of your library"),
        "expected draw-then-put-two-cards wording, got {rendered}"
    );
}

#[test]
fn render_each_player_puts_card_from_hand_on_top_normalizes_for_each_form() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Sadistic Augermage Variant")
        .parse_text("When this creature dies, each player puts a card from their hand on top of their library.")
        .expect("each-player hand-to-library clause should parse");
    let joined = crate::compiled_text::oracle_like_lines(&def).join("\n");
    assert!(
        joined.contains("each player puts a card from their hand on top of their library"),
        "expected normalized each-player hand-to-library wording, got {joined}"
    );
}

#[test]
fn render_all_slivers_have_regenerate_uses_quoted_ability_text() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Poultice Sliver Variant")
        .parse_text("All Slivers have \"{2}, {T}: Regenerate target Sliver.\"")
        .expect("all-slivers-regenerate line should parse");
    let joined = crate::compiled_text::oracle_like_lines(&def).join("\n");
    assert!(
        joined.contains("All Slivers have \"{2}, {T}: Regenerate target Sliver.\"")
            || joined.contains("All Slivers have \"{2}, {T}: Regenerate target sliver.\"")
            || joined.contains("All Sliver creatures have \"{2}, {T}: Regenerate target sliver.\"")
            || joined.contains("All Sliver creatures have \"{2}, {T}: Regenerate target Sliver.\""),
        "expected quoted Sliver granted ability wording, got {joined}"
    );
}

#[test]
fn render_all_slivers_have_sacrifice_add_mana_uses_quoted_ability_text() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Basal Sliver Variant")
        .parse_text("All Slivers have \"Sacrifice this permanent: Add {B}{B}.\"")
        .expect("all-slivers-sacrifice-mana line should parse");
    let joined = crate::compiled_text::oracle_like_lines(&def).join("\n");
    assert!(
        joined.contains("All Slivers have \"Sacrifice this permanent: Add {B}{B}.\"")
            || joined
                .contains("All Sliver creatures have \"Sacrifice this permanent: Add {B}{B}.\""),
        "expected quoted Sliver sacrifice-mana wording, got {joined}"
    );
}

#[test]
fn render_surveil_uses_keyword_action_wording() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Surveil Variant")
        .parse_text("Surveil 1.")
        .expect("surveil spell should parse");
    let lines = compiled_lines(&def);
    let spell_line = lines
        .iter()
        .find(|line| line.starts_with("Spell effects:"))
        .expect("expected spell effects line");
    assert!(
        spell_line.contains("Surveil 1"),
        "expected oracle-like surveil text, got {spell_line}"
    );
}

#[test]
fn render_tap_target_spirit_uses_subtype_noun() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Spirit Tapper Variant")
        .parse_text("{T}: Tap target Spirit.")
        .expect("tap target Spirit should parse");
    let joined = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains("target spirit"),
        "expected Spirit subtype noun rendering, got {joined}"
    );
    assert!(
        !joined.contains("permanent spirit"),
        "unexpected permanent noun for Spirit subtype, got {joined}"
    );
}

#[test]
fn render_tap_target_wall_uses_subtype_noun() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Wall Tapper Variant")
        .parse_text("{R}: Tap target Wall.")
        .expect("tap target Wall should parse");
    let joined = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains("target wall"),
        "expected Wall subtype noun rendering, got {joined}"
    );
    assert!(
        !joined.contains("permanent wall"),
        "unexpected permanent noun for Wall subtype, got {joined}"
    );
}

#[test]
fn render_untap_target_snow_land_includes_supertype() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Snow Untapper Variant")
        .parse_text("{T}: Untap target snow land.")
        .expect("untap target snow land should parse");
    let joined = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains("snow land"),
        "expected snow supertype rendering, got {joined}"
    );
}

#[test]
fn render_artifacts_and_lands_enter_tapped_uses_union_types() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Root Maze Variant")
        .parse_text("Artifacts and lands enter the battlefield tapped.")
        .expect("artifacts and lands enter tapped should parse");
    let joined = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains("artifacts and lands enter tapped"),
        "expected union type rendering, got {joined}"
    );
    assert!(
        !joined.contains("artifact land enter the battlefield tapped"),
        "unexpected artifact-land intersection rendering, got {joined}"
    );
}

#[test]
fn render_damage_each_creature_and_each_player_keeps_both_targets() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Steam Blast Variant")
        .parse_text("This spell deals 2 damage to each creature and each player.")
        .expect("damage each creature and each player should parse");
    let joined = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains("each player"),
        "expected player damage target in rendering, got {joined}"
    );
    assert!(
        joined.contains("each creature"),
        "expected creature damage target in rendering, got {joined}"
    );
}

#[test]
fn render_subject_with_counters_cant_be_blocked_preserves_subject_filter() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Herald Variant")
        .parse_text("Creatures you control with +1/+1 counters on them can't be blocked.")
        .expect("subject unblockable static line should parse");
    let joined = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains("creatures you control") && joined.contains("can't be blocked"),
        "expected subject + restriction rendering, got {joined}"
    );
}

#[test]
fn render_granted_counter_subject_preserves_counter_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Hagra Variant")
        .parse_text(
            "This creature enters with two +1/+1 counters on it.\nEach creature you control with a +1/+1 counter on it has menace.",
        )
        .expect("counter-qualified grant line should parse");
    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("menace"),
        "expected counter-qualified menace grant rendering, got {rendered}"
    );
    assert!(
        !rendered.contains("permanents have menace"),
        "rendering regressed to broad permanent grant: {rendered}"
    );
}

#[test]
fn render_subject_with_power_or_toughness_cant_be_blocked_preserves_filter() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Tetsuko Variant")
        .parse_text("Creatures you control with power or toughness 1 or less can't be blocked.")
        .expect_err("power/toughness unblockable static line should fail loudly");
    let joined = format!("{err:?}").to_ascii_lowercase();
    assert!(
        joined.contains("unsupported power-or-toughness cant-be-blocked subject"),
        "expected explicit unsupported parse error, got {joined}"
    );
}

#[test]
fn render_create_saproling_token_keeps_subtype() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Sprout Variant")
        .parse_text("Create a 1/1 green Saproling creature token.")
        .expect("saproling token text should parse");
    let joined = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains("saproling"),
        "expected Saproling subtype in rendering, got {joined}"
    );
}

#[test]
fn render_mount_or_vehicle_target() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Daring Mechanic Variant")
        .parse_text("{3}{W}: Put a +1/+1 counter on target Mount or Vehicle.")
        .expect("mount or vehicle target should parse");
    let joined = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains("target mount or vehicle"),
        "expected mount or vehicle target rendering, got {joined}"
    );
}

#[test]
fn render_tap_cost_ability_filter_phrase() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Magewright Stone Variant")
        .parse_text(
            "{1}, {T}: Untap target creature that has an activated ability with {T} in its cost.",
        )
        .expect("tap-cost activated-ability filter should parse");
    let joined = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains("untap target creature that has an activated ability with {t} in its cost"),
        "expected activated-ability tap-cost filter rendering, got {joined}"
    );
}

#[test]
fn render_enchanted_creatures_you_control_pluralizes() {
    let def = CardDefinitionBuilder::new(CardId::new(), "A Tale Variant")
        .parse_text("Enchanted creatures you control get +2/+2.")
        .expect("enchanted-creature anthem should parse");
    let joined = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains("enchanted creatures you control get +2/+2"),
        "expected plural enchanted creatures rendering, got {joined}"
    );
}

#[test]
fn render_allies_you_control_pluralizes() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Allied Teamwork Variant")
        .parse_text("Allies you control get +1/+1.")
        .expect("allies anthem should parse");
    let joined = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains("allies you control get +1/+1"),
        "expected plural allies rendering, got {joined}"
    );
}

#[test]
fn render_tap_or_untap_mode_compacts() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Hyperion Blacksmith Variant")
        .parse_text("{T}: You may tap or untap target artifact an opponent controls.")
        .expect("tap or untap should parse");
    let joined = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains("tap or untap target opponent's artifact")
            || joined.contains("tap or untap target artifact an opponent controls"),
        "expected compact tap/untap rendering, got {joined}"
    );
}

#[test]
fn render_tap_or_untap_mode_does_not_compact_when_targets_differ() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Deceiver Exarch Variant")
        .parse_text(
            "When this creature enters, choose one —\n• Untap target permanent you control.\n• Tap target permanent an opponent controls.",
        )
        .expect("modal tap/untap with different targets should parse");
    let joined = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        !joined.contains("tap or untap target"),
        "different tap/untap targets should not compact, got {joined}"
    );
    assert!(
        joined.contains("choose one")
            && joined.contains("untap target permanent you control")
            && joined.contains("tap target permanent an opponent controls"),
        "expected separate modal tap/untap lines, got {joined}"
    );
}

#[test]
fn oracle_like_equipped_sacrifice_uses_card_name() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Ninja's Kunai")
        .parse_text(
            "Type: Artifact — Equipment\nEquipped creature has \"{1}, {T}, Sacrifice Ninja's Kunai: Ninja's Kunai deals 3 damage to any target.\"\nEquip {1}",
        )
        .expect("ninja's kunai should parse");
    let lines = oracle_like_lines(&def).join(" ");
    assert!(
        lines.contains("Sacrifice Ninja's Kunai: Ninja's Kunai deals 3 damage to any target")
            || lines.contains("Sacrifice this: This deals 3 damage to any target"),
        "expected equipment self-reference rendering, got {lines}"
    );
}

#[allow(dead_code)]
fn assert_partial_parse_rejected(name: &str, text: &str) {
    let err = CardDefinitionBuilder::new(CardId::new(), name)
        .parse_text(text)
        .expect_err("unsupported partial-parse-prone clause should fail parse");
    let message = format!("{err:?}");
    assert!(
        message.contains("unsupported partial-parse-prone clause")
            || message.contains("unsupported standalone token mana reminder clause")
            || message.contains("could not find verb in effect clause"),
        "expected partial-parse rejection, got {message}"
    );
}

#[test]
fn render_search_library_for_card_uses_card_noun() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Search Variant")
        .parse_text(
            "Search your library for a card, reveal it, put it into your hand, then shuffle.",
        )
        .expect("search clause should parse");
    let joined = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains("search your library for a card"),
        "expected search filter to render as card, got {joined}"
    );
}

#[test]
fn parse_standalone_shuffle_clause_defaults_to_library_owner() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Shuffle Variant")
        .parse_text("Search your library for a card, put it into your hand. Shuffle.")
        .expect("standalone shuffle clause should parse");
    let joined = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains("search your library for a card")
            && joined.contains("shuffle your library"),
        "expected standalone shuffle to resolve to your library, got {joined}"
    );
}

#[test]
fn parse_search_target_player_library_and_exile_cards() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Denying Wind Variant")
        .parse_text("Search target player's library for up to seven cards and exile them. Then that player shuffles.")
        .expect("target-player search-and-exile clause should parse");

    let joined = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        (joined.contains("search target player's library for up to seven cards and exile")
            || joined.contains("search target player's library for up to 7 cards, exile"))
            && (joined.contains("then that player shuffles")
                || joined.contains("shuffle target player's library")),
        "expected search/exile/shuffle rendering, got {joined}"
    );

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("ChooseObjectsEffect")
            && debug.contains("zone: Some(Library)")
            && debug.contains("zone: Exile"),
        "expected search-from-library into exile sequence, got {debug}"
    );
}

#[test]
fn parse_search_its_controller_graveyard_hand_and_library_exiles_same_name_cards() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Quash Variant")
        .parse_text("Counter target spell. Search its controller's graveyard, hand, and library for all cards with the same name as that spell and exile them. Then that player shuffles.")
        .expect("multi-zone same-name search-and-exile clause should parse");

    let joined = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        !joined.contains("exile target player"),
        "search clause must not collapse into exile-player fallback, got {joined}"
    );
    assert!(
        joined.contains("search its controller's graveyard, hand, and library for all cards with the same name as that object and exile them")
            && joined.contains("then that player shuffles"),
        "expected compact multi-zone search rendering, got {joined}"
    );

    let debug = format!("{:?}", def.spell_effect).to_ascii_lowercase();
    assert!(
        debug.contains("additional_zones: [hand, library]")
            && (debug.contains("zone: graveyard") || debug.contains("zone: none"))
            && debug.contains("samenameastagged")
            && debug.contains("controllerof")
            && debug.contains("shufflelibraryeffect"),
        "expected same-name exile across hand/graveyard/library and controller shuffle, got {debug}"
    );
    assert_eq!(
        debug.matches("shufflelibraryeffect").count(),
        1,
        "expected exactly one shuffle, got {debug}"
    );
    assert!(
        debug.contains("shufflelibraryeffect { player: controllerof(target)")
            || debug
                .contains("shufflelibraryeffect { player: controllerof(tagged(tagkey(\"exiled_"),
        "expected shuffle to target the searched player's controller, got {debug}"
    );
}

#[test]
fn parse_choose_card_name_then_draw_for_each_card_exiled_from_hand_this_way() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Stone Brain Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Choose a card name. Search target opponent's graveyard, hand, and library for all cards with that name and exile them. Then that player shuffles, then draws a card for each card exiled from their hand this way.",
        )
        .expect("stone brain style clause should parse");

    let debug = format!("{:?}", def.spell_effect).to_ascii_lowercase();
    assert!(
        debug.contains("choosecardnameeffect")
            && debug.contains("drawforeachtaggedmatchingeffect")
            && debug.contains("shufflelibraryeffect"),
        "expected choose-name search/exile/draw lowering, got {debug}"
    );
}

#[test]
fn parse_destroy_then_search_target_opponent_library_preserves_destroy_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Life's Finale Variant")
        .parse_text("Destroy all creatures, then search target opponent's library for up to three creature cards and put them into their graveyard. Then that player shuffles.")
        .expect("destroy-then-search clause should parse");

    let joined = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains("destroy all creatures")
            && joined.contains("search target opponent's library for up to three creature")
            && joined.contains("put them into")
            && joined.contains("graveyard")
            && (joined.contains("then that player shuffles")
                || joined.contains("shuffle target opponent's library")),
        "expected destroy and search/put/shuffle chain, got {joined}"
    );
    assert!(
        !joined.contains("destroy all creatures card in an opponent's libraries"),
        "search clause should not degrade into destroy-library fallback, got {joined}"
    );
}

#[test]
fn parse_where_x_is_fixed_plus_number_of_filter_value() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Muscle Burst Variant")
        .parse_text(
            "Target creature gets +X/+X until end of turn, where X is 3 plus the number of cards named Muscle Burst in all graveyards.",
        )
        .expect("where-X fixed-plus-count gets clause should parse");

    let debug = format!("{:?}", def.spell_effect).to_ascii_lowercase();
    assert!(
        debug.contains("modifypowertoughness")
            && debug.contains("add(fixed(3), count(")
            && debug.contains("name: some(\"muscle burst\")")
            && debug.contains("graveyard"),
        "expected fixed-plus-count where-X value in compiled effect, got {debug}"
    );
}

#[test]
fn parse_search_filter_artifact_with_mana_ability_or_basic_land() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Moonsilver Key Variant")
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "{1}, {T}, Sacrifice this artifact: Search your library for an artifact card with a mana ability or a basic land card, reveal it, put it into your hand, then shuffle.",
        )
        .expect("artifact-with-mana-ability-or-basic-land search should parse");

    let abilities_debug = format!("{:?}", def.abilities);
    assert!(
        abilities_debug.contains("any_of: [ObjectFilter"),
        "expected disjunctive search filter branches, got {abilities_debug}"
    );
    assert!(
        abilities_debug.contains("ability_markers: [\"mana ability\"]")
            && abilities_debug.contains("supertypes: [Basic]")
            && abilities_debug.contains("card_types: [Land]"),
        "expected mana-ability and basic-land branch constraints, got {abilities_debug}"
    );

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("artifact with mana ability or basic land"),
        "expected disjunctive search wording, got {rendered}"
    );
}

#[test]
fn render_powerstone_token_name() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Powerstone Variant")
        .parse_text("Create a tapped Powerstone token.")
        .expect("powerstone token clause should parse");
    let joined = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains("powerstone token") && joined.contains("tapped"),
        "expected powerstone token name in compiled text, got {joined}"
    );
}

#[test]
fn parse_token_with_banding_keyword_modifier() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Errand of Duty Variant")
        .parse_text("Create a 1/1 white Knight creature token with banding.")
        .expect("token with banding modifier should parse");

    let effects = def.spell_effect.as_ref().expect("spell effects");
    let debug = format!("{effects:?}");
    assert!(
        debug.contains("KeywordFallbackText"),
        "expected created token to keep banding fallback ability text, got {debug}"
    );

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("token with banding") || rendered.contains("token with \"banding\""),
        "expected compiled text to include banding token modifier, got {rendered}"
    );
}

#[test]
fn parse_myriad_keyword_as_typed_trigger_without_keyword_marker() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Conclave Evangelist Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Myriad")
        .expect("myriad keyword should parse");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("ForPlayersEffect")
            && debug.contains("MayEffect")
            && debug.contains("CreateTokenCopyEffect")
            && !debug.contains("MyriadTokenCopiesEffect"),
        "expected composed myriad trigger effect, got {debug}"
    );
    assert!(
        !debug.contains("StaticAbilityId::KeywordMarker"),
        "myriad should not compile as keyword marker ability: {debug}"
    );
}

#[test]
fn parse_myriad_oracle_text_uses_composed_primitives() {
    // Myriad oracle text with "Exile the tokens at end of combat" clause
    // currently fails to parse due to unsupported standalone token reminder clause.
    let text = "Whenever this creature attacks, for each opponent other than defending player, you may create a token that's a copy of this creature that's tapped and attacking that player or a planeswalker they control. Exile the tokens at end of combat.";
    let result = CardDefinitionBuilder::new(CardId::new(), "Myriad Oracle Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(text);

    assert!(
        result.is_err(),
        "myriad oracle text with exile clause is not yet supported"
    );
}

#[test]
fn parse_wyrms_crossing_patrol_myriad_renders_you_as_token_creator() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Wyrm's Crossing Patrol")
        .card_types(vec![CardType::Creature])
        .parse_text("Myriad")
        .expect("wrym's crossing patrol myriad keyword should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("for each opponent other than defending player, you may create a token"),
        "expected myriad render to keep you as the token creator, got {rendered}"
    );
    assert!(
        !rendered.contains("that player may create a token"),
        "myriad render must not flip the token creator to the iterated player, got {rendered}"
    );
}

#[test]
fn parse_named_vehicle_token_with_flying_and_crew() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Lita Token Variant")
        .parse_text(
            "{3}{W}, {T}: Create a 5/5 colorless Vehicle artifact token named Zeppelin with flying and crew 3.",
        )
        .expect("named vehicle token should preserve flying and crew");

    let abilities_debug = format!("{:?}", def.abilities);
    assert!(
        abilities_debug.contains("name: \"Zeppelin\""),
        "expected created token name to be Zeppelin, got {abilities_debug}"
    );
    assert!(
        abilities_debug.contains("Flying"),
        "expected created token to keep flying, got {abilities_debug}"
    );
    assert!(
        abilities_debug.contains("crew 3"),
        "expected created token crew marker, got {abilities_debug}"
    );
}

#[test]
fn parse_damage_not_removed_during_cleanup_line() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Ancient Adamantoise Variant")
        .parse_text("Damage isn't removed from this creature during cleanup steps.")
        .expect("damage-not-removed clause should parse");
    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::DamageNotRemovedDuringCleanup),
        "expected damage-not-removed static ability, got {ids:?}"
    );
    let compiled = compiled_lines(&def).join(" ");
    assert!(
        compiled
            .to_ascii_lowercase()
            .contains("damage isn't removed from this creature during cleanup steps"),
        "expected compiled text to include damage-not-removed clause, got {compiled}"
    );
}

#[test]
fn parse_damage_redirect_to_source_line() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Ancient Adamantoise Variant")
        .parse_text("All damage that would be dealt to you and other permanents you control is dealt to this creature instead.")
        .expect("damage redirect clause should parse");
    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::RedirectDamageToSource),
        "expected damage redirect static ability, got {ids:?}"
    );
    let compiled = compiled_lines(&def).join(" ");
    assert!(
        compiled
            .to_ascii_lowercase()
            .contains("all damage that would be dealt to you and other permanents you control is dealt to this creature instead"),
        "expected compiled text to include damage redirect clause, got {compiled}"
    );
}

#[test]
fn parse_no_more_than_creatures_can_attack_or_block_each_combat_lines() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Silent Arbiter Variant")
        .parse_text(
            "No more than one creature can attack each combat.\nNo more than one creature can block each combat.",
        )
        .expect("no-more-than attack/block static lines should parse");
    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::MaxCreaturesCanAttackEachCombat),
        "expected attack-cap static ability, got {ids:?}"
    );
    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::MaxCreaturesCanBlockEachCombat),
        "expected block-cap static ability, got {ids:?}"
    );
    let compiled = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        compiled.contains("no more than 1 creature can attack each combat"),
        "expected compiled text to include attack cap, got {compiled}"
    );
    assert!(
        compiled.contains("no more than 1 creature can block each combat"),
        "expected compiled text to include block cap, got {compiled}"
    );
}

#[test]
fn parse_opponent_loses_life_trigger_with_that_much_gain() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Life Trigger Variant")
        .parse_text("Whenever an opponent loses life, you gain that much life.")
        .expect("opponent-loses-life trigger with that-much gain should parse");
    let joined = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains("whenever an opponent loses life")
            && joined.contains("you gain that much life"),
        "expected life-loss trigger and mirrored gain rendering, got {joined}"
    );
}

#[test]
fn parse_you_gain_life_trigger_with_target_opponent_loses_that_much() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Life Trigger Reverse Variant")
        .parse_text("Whenever you gain life, target opponent loses that much life.")
        .expect("you-gain-life trigger with that-much life loss should parse");
    let joined = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains("whenever you gain life")
            && joined.contains("target opponent loses that much life"),
        "expected gain-life trigger and mirrored loss rendering, got {joined}"
    );
}

#[test]
fn reject_event_value_life_amount_without_life_trigger() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Event Value Invalid Variant")
        .parse_text("Target opponent loses that much life.")
        .expect_err("standalone event-derived amount should fail parse");
    let message = format!("{err:?}");
    assert!(
        message.contains("event-derived amount requires a compatible trigger"),
        "expected event-value context rejection, got {message}"
    );
}

#[test]
fn parse_damage_to_target_player_or_planeswalker() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Magmutt Variant")
        .parse_text("{T}: This creature deals 1 damage to target player or planeswalker.")
        .expect("player-or-planeswalker damage target should parse");
    let joined = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains("deals 1 damage to target player or planeswalker"),
        "expected compiled damage text, got {joined}"
    );
}

#[test]
fn parse_that_much_damage_trigger_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Mogg Maniac Variant")
        .parse_text(
            "Whenever this creature is dealt damage, it deals that much damage to any target.",
        )
        .expect("that-much damage trigger clause should parse");
    let joined = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains("that much damage"),
        "expected event-derived damage amount in compiled text, got {joined}"
    );
}

#[test]
fn parse_gain_choice_of_keywords_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Gift Variant")
        .parse_text("Target creature gets +1/+1 and gains your choice of deathtouch or lifelink until end of turn.")
        .expect("gain-choice keyword clause should parse");
    let joined = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains("deathtouch") && joined.contains("lifelink"),
        "expected both keyword options in compiled text, got {joined}"
    );
}

#[test]
fn parse_gain_choice_of_three_keywords_clause_compiles_to_mode_choice() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Assassin Initiate Variant")
        .parse_text("{1}: This creature gains your choice of flying, deathtouch, or lifelink until end of turn.")
        .expect("three-option gain-choice keyword clause should parse");
    let abilities_debug = format!("{:#?}", def.abilities);
    assert!(
        abilities_debug.contains("ChooseModeEffect"),
        "expected three-option keyword grant to compile as a modal choice, got {abilities_debug}"
    );
    assert!(
        abilities_debug.contains("Flying")
            && abilities_debug.contains("Deathtouch")
            && abilities_debug.contains("Lifelink"),
        "expected all three keyword options to be represented, got {abilities_debug}"
    );
}

#[test]
fn parse_search_same_name_reference_filter_in_graveyard() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Frostpyre Arcanist Variant")
        .parse_text("When this creature enters, search your library for an instant or sorcery card with the same name as a card in your graveyard, reveal it, put it into your hand, then shuffle.")
        .expect("same-name reference search clause should parse");
    let abilities_debug = format!("{:#?}", def.abilities);
    assert!(
        abilities_debug.contains("SameNameAsTagged"),
        "expected same-name search to use tagged same-name constraint, got {abilities_debug}"
    );
    assert!(
        abilities_debug.contains("same_name_reference"),
        "expected same-name search to tag a reference object, got {abilities_debug}"
    );
}

#[test]
fn parse_alternative_cost_with_return_to_hand_segment() {
    CardDefinitionBuilder::new(CardId::new(), "Borderpost Variant")
        .parse_text("You may pay {1} and return a basic land you control to its owner's hand rather than pay this spell's mana cost.")
        .expect("alternative cost with return-to-hand segment should parse");
}

#[test]
fn parse_alternative_cost_with_return_to_hand_segment_preserves_non_mana_costs() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Borderpost Variant")
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "You may pay {1} and return a basic land you control to its owner's hand rather than pay this spell's mana cost.",
        )
        .expect("alternative cost with return-to-hand segment should parse");

    let alternative = def
        .alternative_casts
        .first()
        .expect("expected parsed alternative cast");
    assert!(
        alternative.is_composed_cost(),
        "expected non-mana alternative costs to be preserved"
    );

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("return") && rendered.contains("basic land"),
        "expected return-land cost in compiled text, got {rendered}"
    );
}

#[test]
fn parse_if_you_control_no_artifacts_compiles_to_negated_player_controls() {
    let def = CardDefinitionBuilder::new(CardId::new(), "No Artifacts Probe")
        .card_types(vec![CardType::Instant])
        .parse_text("Draw two cards. If you control no artifacts, discard a card.")
        .expect("control-no-artifacts predicate should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("if you control no artifac"),
        "expected negated control predicate in compiled text, got {rendered}"
    );

    let debug = format!("{:#?}", def.spell_effect);
    assert!(
        debug.contains("Not(") && debug.contains("PlayerControls"),
        "expected control-no predicate to compile to Condition::Not(PlayerControls), got {debug}"
    );
}

#[test]
fn parse_first_main_phase_trigger_uses_precombat_main_and_active_player() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Vineyard Probe")
        .card_types(vec![CardType::Enchantment])
        .parse_text("At the beginning of each player's first main phase, that player adds {G}{G}.")
        .expect("first-main-phase trigger should parse");

    let ability = def.abilities.first().expect("expected triggered ability");
    let AbilityKind::Triggered(triggered) = &ability.kind else {
        panic!("expected triggered ability");
    };
    assert!(
        triggered
            .trigger
            .display()
            .to_ascii_lowercase()
            .contains("first main phase"),
        "expected first-main-phase trigger display, got {}",
        triggered.trigger.display()
    );

    let add_mana = triggered.effects[0]
        .downcast_ref::<AddManaEffect>()
        .expect("expected add-mana effect");
    assert_eq!(
        add_mana.player,
        PlayerFilter::Active,
        "expected \"that player\" to resolve to active player"
    );
}

#[test]
fn parse_mill_cost_activation_line() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Mill Cost Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("{T}, Mill a card: Add {C}.")
        .expect("mill-cost activation line should parse");
    let rendered = oracle_like_lines(&def).join(" ");
    assert!(
        rendered.to_ascii_lowercase().contains("mill a card"),
        "expected mill cost to be preserved in rendering, got {rendered}"
    );
}

#[test]
fn parse_return_cost_activation_line() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Return Cost Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Return a Forest you control to its owner's hand: Untap target creature. Activate only once each turn.")
        .expect("return-cost activation line should parse");
    let compiled = compiled_lines(&def).join(" ");
    assert!(
        compiled.to_ascii_lowercase().contains("activated ability"),
        "expected activated ability rendering, got {compiled}"
    );
    assert!(
        compiled
            .to_ascii_lowercase()
            .contains("untap target creature"),
        "expected untap effect in activated ability, got {compiled}"
    );
    assert!(
        compiled
            .to_ascii_lowercase()
            .contains("activate only once each turn"),
        "expected once-per-turn restriction in activated ability, got {compiled}"
    );
}

#[test]
fn parse_return_elf_cost_activation_line() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Return Elf Cost Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Return an Elf you control to its owner's hand: Untap target creature. Activate only once each turn.")
        .expect("return-elf-cost activation line should parse");
    let compiled = compiled_lines(&def).join(" ");
    assert!(
        compiled.to_ascii_lowercase().contains("activated ability"),
        "expected activated ability rendering, got {compiled}"
    );
    assert!(
        compiled
            .to_ascii_lowercase()
            .contains("untap target creature"),
        "expected untap effect in activated ability, got {compiled}"
    );
    assert!(
        compiled
            .to_ascii_lowercase()
            .contains("activate only once each turn"),
        "expected once-per-turn restriction in activated ability, got {compiled}"
    );
}

#[test]
fn parse_equip_with_once_each_turn_restriction() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Leather Armor Variant")
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Equipment])
        .parse_text("Equip {0}.\nActivate only once each turn.")
        .expect("equip with once-each-turn restriction should parse");
    let compiled = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        compiled.contains("equip {0}"),
        "expected equip ability in compiled output, got {compiled}"
    );
    assert!(
        compiled.contains("only once each turn"),
        "expected once-per-turn activation restriction in compiled output, got {compiled}"
    );
}

#[test]
fn parse_mercenary_token_with_tap_pump_ability() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Mercenary Token Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("When this creature enters, create a 1/1 red Mercenary creature token with \"{T}: Target creature you control gets +1/+0 until end of turn. Activate only as a sorcery.\"")
        .expect("mercenary token with tap-pump ability should parse");
    let compiled = compiled_lines(&def).join(" ");
    let lower = compiled.to_ascii_lowercase();
    assert!(
        lower.contains("mercenary creature token"),
        "expected mercenary token creation in compiled text, got {compiled}"
    );
}

#[test]
fn parse_token_becomes_tapped_damage_trigger() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Tapped Trigger Token Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("When this creature enters, create a 1/1 red Elemental creature token with \"Whenever this token becomes tapped, it deals 1 damage to target player.\"")
        .expect("token with becomes-tapped damage trigger should parse");

    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("expected triggered ability");
    let create = triggered
        .effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<CreateTokenEffect>())
        .expect("expected token creation effect");
    assert!(
        !create.enters_tapped,
        "expected token to enter untapped, got {create:#?}"
    );

    let compiled = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        compiled.contains("becomes tapped") && compiled.contains("deals 1 damage to target player"),
        "expected becomes-tapped damage trigger in compiled text, got {compiled}"
    );
}

#[test]
fn parse_deathpact_style_token_activation_is_preserved() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Deathpact Angel")
        .card_types(vec![CardType::Creature])
        .parse_text("When this creature dies, create a 1/1 white and black Cleric creature token. It has \"{3}{W}{B}{B}, {T}, Sacrifice this token: Return a card named Deathpact Angel from your graveyard to the battlefield.\"")
        .expect("deathpact-style token activation should parse");
    let compiled = compiled_lines(&def).join(" ");
    let lower = compiled.to_ascii_lowercase();
    assert!(
        lower.contains("create a 1/1 white and black cleric creature token"),
        "expected deathpact token creation to remain in output, got {compiled}"
    );
}

#[test]
fn parse_llanowar_mentor_token_keeps_tap_for_green_mana_ability() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Llanowar Mentor Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "{G}, {T}, Discard a card: Create a 1/1 green Elf Druid creature token named Llanowar Elves. It has \"{T}: Add {G}.\"",
        )
        .expect("llanowar mentor token reminder should parse");

    let activated = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("expected activated ability");
    let create = activated
        .effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<CreateTokenEffect>())
        .expect("expected token creation effect");
    let has_green_tap_mana = create.token.abilities.iter().any(|ability| {
        let AbilityKind::Activated(activated) = &ability.kind else {
            return false;
        };
        activated.effects.iter().any(|effect| {
            effect
                .downcast_ref::<AddManaEffect>()
                .is_some_and(|add| add.mana == vec![ManaSymbol::Green])
        })
    });
    assert!(
        has_green_tap_mana,
        "expected created token to keep '{{T}}: Add {{G}}' ability, got {:#?}",
        create.token.abilities
    );

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("{t}: add {g}"),
        "expected compiled text to show token mana ability, got {rendered}"
    );
}

#[test]
fn parse_sparkspitter_token_reminder_sets_next_end_step_sacrifice() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Sparkspitter Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "{R}, {T}, Discard a card: Create a 3/1 red Elemental creature token named Spark Elemental. It has trample, haste, and \"At the beginning of the end step, sacrifice this token.\"",
        )
        .expect("sparkspitter token reminder should parse");

    let activated = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("expected activated ability");
    let create = activated
        .effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<CreateTokenEffect>())
        .expect("expected token creation effect");

    assert!(
        create.sacrifice_at_next_end_step,
        "expected token to be marked for next-end-step sacrifice, got {create:#?}"
    );

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("sacrifice")
            && rendered.contains("end step")
            && rendered.contains("trample")
            && rendered.contains("haste"),
        "expected compiled text to preserve token keywords and delayed sacrifice, got {rendered}"
    );
}

#[test]
fn parse_construct_token_with_for_each_artifact_text_keeps_single_token_and_cda() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Urza Construct Variant")
        .card_types(vec![CardType::Artifact])
        .parse_text("{2}, {T}: Create a 0/0 colorless Construct artifact creature token with \"This token gets +1/+1 for each artifact you control.\"")
        .expect("construct token with inline for-each artifact text should parse");

    let activated = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("expected activated ability");
    let create = activated
        .effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<CreateTokenEffect>())
        .expect("expected token creation effect");
    assert!(
        matches!(create.count, crate::effect::Value::Fixed(1)),
        "expected exactly one token to be created, got {:?}",
        create.count
    );
    let has_cda = create.token.abilities.iter().any(|ability| {
        matches!(
            &ability.kind,
            AbilityKind::Static(static_ability)
                if static_ability.id() == StaticAbilityId::CharacteristicDefiningPT
        )
    });
    assert!(
        has_cda,
        "expected Construct token to keep +1/+1-for-each-artifact behavior, got {:#?}",
        create.token.abilities
    );
}

#[test]
fn parse_construct_token_with_single_quoted_rules_text_keeps_cda() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Construct Quote Variant")
        .parse_text(
            "Create a 0/0 colorless Construct artifact creature token with 'This token gets +1/+1 for each artifact you control.'",
        )
        .expect("single-quoted Construct token text should parse");

    let debug = format!("{:#?}", def.spell_effect);
    assert!(
        debug.contains("CreateTokenEffect"),
        "expected spell create-token effect, got {debug}"
    );
    assert!(
        debug.contains("CharacteristicDefiningPT"),
        "expected Construct token to keep dynamic +1/+1 scaling text, got {debug}"
    );
}

#[test]
fn parse_sound_the_call_token_does_not_misread_named_card_reference_as_token_name() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Sound the Call Variant")
        .parse_text(
            "Create a 1/1 green Wolf creature token. It has \"This token gets +1/+1 for each card named Sound the Call in each graveyard.\"",
        )
        .expect("sound-the-call token reminder should parse");

    let debug = format!("{:#?}", def.spell_effect);
    assert!(
        debug.contains("CreateTokenEffect"),
        "expected spell create-token effect, got {debug}"
    );
    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("wolf creature token"),
        "token name should remain subtype-derived Wolf, got {rendered}"
    );
    assert!(
        rendered.contains("for each card named sound the call in each graveyard"),
        "expected token to keep +1/+1-for-each-named-card ability, got {rendered}"
    );
}

#[test]
fn parse_ozox_nested_token_return_keeps_named_card_literal() {
    let canonical = |name: &str| {
        name.chars()
            .filter(|ch| ch.is_ascii_alphanumeric())
            .map(|ch| ch.to_ascii_lowercase())
            .collect::<String>()
    };

    let def = CardDefinitionBuilder::new(CardId::new(), "Ozox, the Clattering King")
        .card_types(vec![CardType::Creature])
        .parse_text("Ozox can't block.\nWhen Ozox dies, create Jumblebones, a legendary 2/1 black Skeleton creature token with \"Jumblebones can't block\" and \"When Jumblebones leaves the battlefield, return target card named Ozox, the Clattering King from your graveyard to your hand.\"")
        .expect("ozox nested token return clause should parse");

    let outer_trigger = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("expected outer dies trigger");
    let create = outer_trigger
        .effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<CreateTokenEffect>())
        .expect("expected token creation effect");

    let token_trigger = create
        .token
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("expected token leaves-the-battlefield trigger");
    let return_effect = token_trigger
        .effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<ReturnFromGraveyardToHandEffect>())
        .expect("expected return-from-graveyard effect");

    let filter = match return_effect.target.base() {
        ChooseSpec::Object(filter) => filter,
        other => panic!("expected object-target choose spec, got {other:?}"),
    };
    let parsed_name = filter
        .name
        .as_deref()
        .expect("expected named-card filter on nested token trigger");
    assert_ne!(
        parsed_name.to_ascii_lowercase(),
        "this",
        "named-card filter must not collapse to 'this'"
    );
    assert_eq!(
        canonical(parsed_name),
        canonical("Ozox, the Clattering King"),
        "expected nested named filter to preserve semantic card-name identity"
    );
}

#[test]
fn render_sacrifice_all_non_ogres() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Yukora Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("When this creature leaves the battlefield, sacrifice all non-Ogre creatures you control.")
        .expect("sacrifice-all trigger should parse");
    let compiled = compiled_lines(&def).join(" ");
    assert!(
        compiled
            .to_ascii_lowercase()
            .contains("sacrifice all non-ogre creatures you control"),
        "expected 'sacrifice all' rendering, got {compiled}"
    );
}

#[test]
fn parse_mana_ability_activate_only_if_control_subtype() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Tainted Mana Variant")
        .card_types(vec![CardType::Land])
        .parse_text("{T}: Add {B}.\n{T}: Add {U}. Activate only if you control a Swamp.")
        .expect("mana ability activation condition should parse");
    let rendered = oracle_like_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("{t}: add {u}"),
        "expected mana production text in rendered output, got {rendered}"
    );
    assert!(
        rendered.contains("activate only if you control a swamp")
            || rendered.contains("activate only if you control swamp"),
        "expected rendered subtype activation restriction, got {rendered}"
    );
}

#[test]
fn parse_semicolon_keyword_line_does_not_force_comma_merge() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Semicolon Keywords Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("First strike; banding")
        .expect("semicolon keyword line should parse");
    let rendered = oracle_like_lines(&def).join("\n");
    assert!(
        rendered.contains("First strike"),
        "expected first strike keyword rendering, got {rendered}"
    );
    assert!(
        rendered.contains("Banding"),
        "expected banding keyword rendering, got {rendered}"
    );
}

#[test]
fn parse_storage_depletion_and_ki_counters() {
    CardDefinitionBuilder::new(CardId::new(), "Storage Variant")
        .parse_text("{2}, {T}: Put a storage counter on this land.")
        .expect("storage counter line should parse");
    let depletion = CardDefinitionBuilder::new(CardId::new(), "Depletion Variant")
        .parse_text("This land enters tapped with two depletion counters on it.")
        .expect("depletion counter line should parse");
    let depletion_static_ids: Vec<_> = depletion
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        depletion_static_ids.contains(&StaticAbilityId::EntersTapped),
        "expected enters-tapped static ability for tapped-with-counters line, got {depletion_static_ids:?}"
    );
    assert!(
        depletion_static_ids.contains(&StaticAbilityId::EnterWithCounters),
        "expected enters-with-counters static ability for tapped-with-counters line, got {depletion_static_ids:?}"
    );
    CardDefinitionBuilder::new(CardId::new(), "Ki Variant")
        .parse_text("Whenever you cast a Spirit or Arcane spell, you may put a ki counter on this creature.")
        .expect("ki counter line should parse");
}

#[test]
fn parse_land_doesnt_untap_if_has_depletion_counter() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Land Cap Variant")
        .parse_text(
            "This land doesn't untap during your untap step if it has a depletion counter on it.",
        )
        .expect("land-level negated untap clause should parse");
    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::DoesntUntap)
            || ids.contains(&crate::static_abilities::StaticAbilityId::GrantAbility),
        "expected doesnt-untap static ability, got {ids:?}"
    );
}

#[test]
fn parse_destroy_target_attacking_or_blocking_creature_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Divine Verdict Variant")
        .card_types(vec![CardType::Instant])
        .parse_text("Destroy target attacking or blocking creature.")
        .expect("parse destroy attacking-or-blocking clause");

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("DestroyEffect"),
        "expected destroy effect, got {debug}"
    );
    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("destroy") && rendered.contains("attacking"),
        "expected attacking/blocking destroy rendering, got {rendered}"
    );
}

#[test]
fn parse_activate_only_restriction_inline_with_activated_ability() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Timed Drawer")
        .card_types(vec![CardType::Creature])
        .parse_text("{T}: Draw a card. Activate only during your turn.")
        .expect("parse activated ability with inline activation restriction");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("draw a card"),
        "expected activated ability rendering, got {rendered}"
    );
}

#[test]
fn parse_mana_ability_activate_only_as_instant_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Flash Mana Source")
        .card_types(vec![CardType::Artifact])
        .parse_text("{T}: Add {R}. Activate only as an instant.")
        .expect("parse mana ability with instant-speed activation restriction");

    let rendered = compiled_lines(&def).join(" ");
    assert!(rendered.contains("Activate only as an instant"));
}

#[test]
fn parse_boast_ability_keeps_mechanic_prefix() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Boastful Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Boast — {1}{R}: This creature deals 1 damage to any target.")
        .expect("parse Boast ability with prefix");

    let rendered = compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Boast {1}{R}"),
        "expected Boast prefix with cost in rendering, got {rendered}"
    );
    assert!(
        rendered.contains("deals 1 damage to any target"),
        "expected Boast effect rendering, got {rendered}"
    );
}

#[test]
fn parse_boast_ability_with_prior_sentence_still_keeps_prefix() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Boastful Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Hagi Mob enters the battlefield tapped. Boast — {1}{R}: This creature deals 1 damage to any target.")
        .expect("parse boast ability after leading sentence with prefix");

    let rendered = compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Boast {1}{R}"),
        "expected Boast prefix with cost in rendering, got {rendered}"
    );
}

#[test]
fn parse_renew_ability_keeps_mechanic_prefix() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Renew Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Renew — {2}{B}, Exile this card from your graveyard: Put a flying counter on target creature. Activate only as a sorcery.",
        )
        .expect("parse Renew ability with prefix");

    let rendered = compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Renew"),
        "expected Renew prefix in rendering, got {rendered}"
    );
    assert!(
        rendered.contains("Exile this card from your graveyard"),
        "expected Renew exile cost in rendering, got {rendered}"
    );
    assert!(
        rendered.contains("Activate only as a sorcery"),
        "expected Renew timing restriction in rendering, got {rendered}"
    );
}

#[test]
fn parse_binding_contract_label_into_draw_replacement_static_ability() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Asmodeus Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Binding Contract — If you would draw a card, exile the top card of your library face down instead.",
        )
        .expect("parse binding contract static replacement line");

    let static_ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        static_ids.contains(&StaticAbilityId::DrawReplacementExileTopFaceDown),
        "expected draw replacement static ability, got {static_ids:?}"
    );
}

#[test]
fn parse_gain_life_for_each_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Life Harvest Variant")
        .card_types(vec![CardType::Instant])
        .parse_text("You gain 2 life for each creature you control.")
        .expect("parse life gain for-each clause");

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("GainLifeEffect") && debug.contains("Count"),
        "expected dynamic life gain value, got {debug}"
    );
}

#[test]
fn parse_deal_damage_equal_to_clause_without_leading_amount() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Equalized Blast Variant")
        .card_types(vec![CardType::Instant])
        .parse_text("Deal damage equal to its power to target creature.")
        .expect("parse equal-to damage clause without numeric amount");

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("PowerOf"),
        "expected power-based damage amount, got {debug}"
    );
}

#[test]
fn render_return_multiple_targets_uses_their_owners_hands() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Return Wave")
        .card_types(vec![CardType::Instant])
        .parse_text("Return up to two target creatures to their owners' hands.")
        .expect("parse multi-return clause");

    let joined = oracle_like_lines(&def).join(" ");
    assert!(
        joined.contains("to their owners' hands"),
        "expected plural owner-hand wording, got {joined}"
    );
}

#[test]
fn render_put_minus_one_counter_uses_singular_counter_wording() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Scar Variant")
        .card_types(vec![CardType::Instant])
        .parse_text("Put a -1/-1 counter on target creature.")
        .expect("parse single counter clause");

    let joined = oracle_like_lines(&def).join(" ");
    assert!(
        joined.contains("Put a -1/-1 counter on target creature"),
        "expected singular -1/-1 counter wording, got {joined}"
    );
}

#[test]
fn render_put_counter_on_each_of_up_to_targets_uses_each_of() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Gird Variant")
        .card_types(vec![CardType::Instant])
        .parse_text("Put a +1/+1 counter on each of up to two target creatures.")
        .expect("parse counted multi-target counter clause");

    let joined = oracle_like_lines(&def).join(" ");
    assert!(
        joined.contains("on each of up to two target creatures"),
        "expected each-of wording for counted target counters, got {joined}"
    );
}

#[test]
fn render_scry_one_then_draw_uses_then() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Mentor Guidance Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "When you cast this spell, copy it if you control a planeswalker, Cleric, Druid, Shaman, Warlock, or Wizard.\nScry 1, then draw a card.",
        )
        .expect("parse mentor's guidance text");

    let joined = oracle_like_lines(&def).join(" ");
    assert!(
        joined.contains("Scry 1, then draw a card"),
        "expected scry/draw to stay as a then-clause, got {joined}"
    );
}

#[test]
fn render_equip_line_with_parenthetical_colon_preserves_prefix_text() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Plate Variant")
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "Equipped creature gets +3/+3 and has ward {1}. (Whenever equipped creature becomes the target of a spell or ability an opponent controls, counter it unless that player pays {1}.)\nEquip {3}. This ability costs {1} less to activate for each other Equipment you control. ({3}: Attach to target creature you control. Equip only as a sorcery.)",
        )
        .expect("parse equip line with parenthetical colon");

    let joined = oracle_like_lines(&def).join(" ");
    assert!(
        joined.contains("Equipped creature gets +3/+3") && joined.contains("Equip {3}"),
        "expected equip prefix text to survive heading stripping, got {joined}"
    );
}

#[test]
fn parse_put_counters_sequence_on_distinct_targets() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Incremental Growth Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Put a +1/+1 counter on target creature, two +1/+1 counters on another target creature, and three +1/+1 counters on a third target creature.",
        )
        .expect("parse chained put-counters clause");

    let spell_effects = def
        .spell_effect
        .as_ref()
        .expect("expected spell effects for chained counters");
    assert_eq!(
        spell_effects.len(),
        3,
        "expected three distinct put-counters effects for the chained clause"
    );
}

#[test]
fn parse_put_multiple_counter_types_on_single_target() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Gift of the Viper Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Put a +1/+1 counter, a reach counter, and a deathtouch counter on target creature. Untap it.",
        )
        .expect("parse shared-target multi-counter clause");

    let spell_effects = def
        .spell_effect
        .as_ref()
        .expect("expected spell effects for shared-target multi-counter clause");
    assert_eq!(
        spell_effects.len(),
        4,
        "expected three put-counters effects plus untap for shared-target multi-counter clause"
    );

    let joined = oracle_like_lines(&def).join(" ");
    assert!(
        joined.contains("Put a +1/+1 counter on target creature"),
        "expected +1/+1 counter clause in rendered text, got {joined}"
    );
    assert!(
        joined.contains("Put a reach counter on target creature"),
        "expected reach counter clause in rendered text, got {joined}"
    );
    assert!(
        joined.contains("Put a deathtouch counter on target creature"),
        "expected deathtouch counter clause in rendered text, got {joined}"
    );
}

#[test]
fn parse_spells_cost_modifier_merges_second_color_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "High Seas Variant")
        .card_types(vec![CardType::Enchantment])
        .parse_text("Red creature spells and green creature spells cost {1} more to cast.")
        .expect("parse dual-color spell tax clause");

    let joined = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains("red and green creature spells cost {1} more to cast"),
        "expected both spell-color qualifiers in rendered text, got {joined}"
    );
}

#[test]
fn parse_spells_cost_modifier_keeps_mana_value_qualifier() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Krosan Drover Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Creature spells you cast with mana value 6 or greater cost {2} less to cast.")
        .expect("parse mana-value-qualified creature cost reduction");

    let joined = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains(
            "creature spells you cast with mana value 6 or greater cost {2} less to cast"
        ),
        "expected mana-value qualifier in rendered cost-modifier text, got {joined}"
    );
}

#[test]
fn parse_spells_cost_modifier_keeps_power_qualifier() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Goreclaw Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Creature spells you cast with power 4 or greater cost {2} less to cast.")
        .expect("parse power-qualified creature cost reduction");

    let joined = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains("creature spells you cast with power 4 or greater cost {2} less to cast"),
        "expected power qualifier in rendered cost-modifier text, got {joined}"
    );
}

#[test]
fn parse_spells_cost_modifier_target_clause_does_not_add_spell_type() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Killian Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Spells you cast that target a creature cost {2} less to cast.")
        .expect("parse target-qualified spell cost reduction");

    let reduction = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => static_ability.cost_reduction(),
            _ => None,
        })
        .expect("expected CostReduction static ability");

    assert!(
        reduction.filter.card_types.is_empty(),
        "target clause should not constrain the spell type, got {:?}",
        reduction.filter.card_types
    );
    assert_eq!(reduction.filter.cast_by, Some(PlayerFilter::You));
    let target_filter = reduction
        .filter
        .targets_object
        .as_deref()
        .expect("expected target object filter");
    assert!(
        target_filter.card_types.contains(&CardType::Creature),
        "expected target filter to keep creature qualifier, got {:?}",
        target_filter.card_types
    );

    let joined = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains("spells you cast that target creature cost {2} less to cast"),
        "expected rendered text to keep the target clause without adding a spell type, got {joined}"
    );
    assert!(
        !joined.contains("creature spells you cast that target creature"),
        "target clause should not render as a creature-spell restriction, got {joined}"
    );
}

#[test]
fn parse_spells_cost_modifier_keeps_noncreature_qualifier() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Glowrider Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Noncreature spells cost {1} more to cast.")
        .expect("parse noncreature spell tax clause");

    let joined = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains("noncreature spells cost {1} more to cast"),
        "expected noncreature qualifier in rendered cost-modifier text, got {joined}"
    );
}

#[test]
fn parse_spells_cost_modifier_supports_colored_mana_increase() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Derelor Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Black spells you cast cost {B} more to cast.")
        .expect("parse colored spell tax clause");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::CostIncreaseManaCost),
        "expected CostIncreaseManaCost static ability, got {ids:?}"
    );

    let joined = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains("black spells you cast cost {b} more to cast"),
        "expected colored cost increase to render, got {joined}"
    );
}

#[test]
fn parse_spells_cost_modifier_supports_where_x_differently_named_lands() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Fungal Colossus Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "This spell costs {X} less to cast, where X is the number of differently named lands you control.",
        )
        .expect("parse where-X cost reduction clause");

    let joined = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains("{x} less to cast"),
        "expected cost reduction in rendered text, got {joined}"
    );
}

#[test]
fn parse_spells_cost_modifier_supports_extended_where_x_clauses() {
    let clauses = [
        "This spell costs {X} less to cast, where X is the total power of creatures you control.",
        "This spell costs {X} less to cast, where X is the total toughness of creatures you control.",
        "This spell costs {X} less to cast, where X is the total mana value of Dragons you control.",
        "This spell costs {X} less to cast, where X is the total mana value of Dragons you control not named Earthquake Dragon.",
        "This spell costs {X} less to cast, where X is the total mana value of noncreature artifacts you control.",
        "This spell costs {X} less to cast, where X is the total mana value of noncreature enchantments you control.",
        "This spell costs {X} less to cast, where X is the total mana value of historic permanents you control.",
        "This spell costs {X} less to cast, where X is the greatest power among creatures you control.",
        "This spell costs {X} less to cast, where X is the greatest mana value among Elementals you control.",
        "This spell costs {X} less to cast this way, where X is the greatest mana value of a commander you own on the battlefield or in the command zone.",
        "This spell costs {X} less to cast, where X is the amount of life you gained this turn.",
        "Creature spells you cast cost {X} less to cast, where X is the amount of life you gained this turn.",
        "This spell costs {X} less to cast, where X is the total amount of noncombat damage dealt to your opponents this turn.",
        "Aura and Equipment spells you cast cost {X} less to cast, where X is this creature's power.",
    ];

    for (idx, clause) in clauses.iter().enumerate() {
        CardDefinitionBuilder::new(
            CardId::from_raw(90_000 + idx as u32),
            format!("Where X Extension {idx}"),
        )
        .card_types(vec![CardType::Creature])
        .parse_text(*clause)
        .expect("extended where-X spells-cost modifier should parse");
    }
}

#[test]
fn parse_destroy_cant_be_regenerated_followup_sentence() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Wrath Tail Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Destroy target creature. It can't be regenerated.")
        .expect("parse destroy + can't be regenerated");

    let joined = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains("destroy target creature. it can't be regenerated")
            || joined.contains("destroy target creature. it cant be regenerated"),
        "expected can't-be-regenerated tail to render, got {joined}"
    );
}

#[test]
fn parse_damage_cant_be_regenerated_followup_sentence() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Engulfing Flames Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Engulfing Flames deals 1 damage to target creature. It can't be regenerated this turn.")
        .expect("parse damage + can't-be-regenerated-this-turn followup");

    let joined = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains("deals 1 damage to target creature")
            || joined.contains("deal 1 damage to target creature"),
        "expected damage clause in rendered text, got {joined}"
    );
    assert!(
        joined.contains("can't be regenerated") || joined.contains("cant be regenerated"),
        "expected can't-be-regenerated clause in rendered text, got {joined}"
    );
}

#[test]
fn parse_threshold_destroy_cant_be_regenerated_followup_sentence() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Toxic Stench Variant")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Target nonblack creature gets -1/-1 until end of turn. Threshold \u{2014} If there are seven or more cards in your graveyard, instead destroy that creature. It can't be regenerated.",
        )
        .expect("parse conditional destroy + can't-be-regenerated followup");

    let joined = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains("destroy target nonblack creature")
            && (joined.contains("can't be regenerated") || joined.contains("cant be regenerated")),
        "expected destroy-no-regeneration conditional branch in rendered text, got {joined}"
    );
}

#[test]
fn parse_destroy_target_creature_dealt_damage_this_turn() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Siegebreaker Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Destroy target creature that was dealt damage this turn.")
        .expect("parse destroy target creature dealt-damage-this-turn clause");

    let joined = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains("destroy target creature that was dealt damage this turn"),
        "expected dealt-damage restriction in rendered destroy text, got {joined}"
    );
}

#[test]
fn parse_exile_target_creature_and_target_land_sentence() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Grip Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Exile target creature and target land.")
        .expect("parse exile with two target objects");

    let joined = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains("exile target creature") && joined.contains("target land"),
        "expected both exile targets in rendered text, got {joined}"
    );
}

#[test]
fn parse_destroy_target_creature_and_target_land_sentence() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Spiteful Blow Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Destroy target creature and target land.")
        .expect("parse destroy with two target objects");

    let joined = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains("destroy target creature") && joined.contains("target land"),
        "expected both destroy targets in rendered text, got {joined}"
    );
}

#[test]
fn parse_destroy_up_to_one_each_target_type_sentence() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Convert Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Destroy up to one target artifact, up to one target creature, and up to one target enchantment.",
        )
        .expect("parse destroy up-to-one multi-target sentence");

    let joined = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains("up to one target artifact")
            && joined.contains("up to one target creature")
            && joined.contains("up to one target enchantment"),
        "expected three up-to-one target destroy clauses, got {joined}"
    );
}

#[test]
fn parse_destroy_source_and_target_blocking_sentence() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Wall of Vipers Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("{3}: Destroy this creature and target creature it's blocking.")
        .expect("parse destroy source + target creature it's blocking");

    let joined = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        (joined.contains("destroy this creature") || joined.contains("destroy this permanent"))
            && (joined.contains("target creature its blocking")
                || joined.contains("target blocking creature")),
        "expected source + blocking target destroy effects, got {joined}"
    );
}

#[test]
fn parse_lesser_werewolf_activated_ability() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(54), "Lesser Werewolf")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "{B}: If this creature's power is 1 or more, it gets -1/-0 until end of turn and put a -0/-1 counter on target creature blocking or blocked by this creature. Activate only during the declare blockers step.",
        )
        .expect("parse Lesser Werewolf activated ability");

    let joined = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains("this creature's power is 1 or more")
            || joined.contains("this creatures power is 1 or more"),
        "expected source-power condition in compiled text, got {joined}"
    );
    assert!(
        joined.contains("gets -1/-0 until end of turn"),
        "expected self-shrink effect in compiled text, got {joined}"
    );
    assert!(
        joined.contains("target creature") && joined.contains("blocking"),
        "expected combat target clause in compiled text, got {joined}"
    );
    let debug = format!("{:#?}", def);
    assert!(
        !debug.contains("UnsupportedParserLine"),
        "did not expect unsupported parser line after parse, got {debug}"
    );
}

#[test]
fn parse_destroy_target_artifact_creature_enchantment_and_land_sentence() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Decimate Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Destroy target artifact, target creature, target enchantment, and target land.",
        )
        .expect("parse four-target destroy sentence");

    let joined = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains("destroy target artifact")
            && joined.contains("target creature")
            && joined.contains("target enchantment")
            && joined.contains("target land"),
        "expected four destroy targets in rendered text, got {joined}"
    );
}

#[test]
fn parse_exile_self_and_target_unless_controller_pays() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Carrionette Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "{2}{B}{B}: Exile this card and target creature unless that creature's controller pays {2}. Activate only if this card is in your graveyard.",
        )
        .expect("parse exile self + target creature unless pays");

    let joined = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains("unless that creature's controller pays {2}")
            || joined.contains("unless that creatures controller pays {2}")
            || joined.contains("unless that object's controller pays {2}")
            || joined.contains("unless that objects controller pays {2}"),
        "expected unless-payment tail in rendered text, got {joined}"
    );
}

#[test]
fn parse_exile_two_graveyard_targets_for_spelltwine_pattern() {
    CardDefinitionBuilder::new(CardId::from_raw(1), "Spelltwine Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Exile target instant or sorcery card from your graveyard and target instant or sorcery card from an opponent's graveyard.",
        )
        .expect("parse dual-target exile across graveyards");
}

#[test]
fn parse_exile_named_source_and_target_permanent() {
    CardDefinitionBuilder::new(CardId::from_raw(1), "Mangara Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("{T}: Exile Mangara and target permanent.")
        .expect("parse exile named source and target permanent");
}

#[test]
fn parse_next_damage_redirect_to_target_creature() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Nomads en-Kor Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "{0}: The next 1 damage that would be dealt to this creature this turn is dealt to target creature you control instead.",
        )
        .expect("parse next-damage redirect clause");

    let joined = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains(
            "the next 1 damage that would be dealt to this creature this turn is dealt to target creature you control instead"
        ),
        "expected redirected-next-damage text in compiled output, got {joined}"
    );
}

#[test]
fn parse_next_time_source_damage_redirect_to_this_creature() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Shaman en-Kor Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "{1}{W}: The next time a source of your choice would deal damage to target creature this turn, that damage is dealt to this creature instead.",
        )
        .expect("parse next-time source redirect clause");

    let joined = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains(
            "the next time a source of your choice would deal damage to target creature this turn, that damage is dealt to this creature instead"
        ),
        "expected next-time source redirect text in compiled output, got {joined}"
    );
}

#[test]
fn parse_spells_cost_modifier_subtype_does_not_force_creature_word() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Dinosaur Cost Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Dinosaur spells you cast cost {1} less to cast.")
        .expect("parse subtype-only spell cost reduction");

    let joined = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains("dinosaur spells you cast cost {1} less to cast"),
        "expected subtype-only spell description, got {joined}"
    );
    assert!(
        !joined.contains("dinosaur creature spells"),
        "did not expect redundant creature word in subtype-only spell description, got {joined}"
    );
}

#[test]
fn render_transform_source_uses_artifact_self_reference_for_artifacts() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Mysterious Tome Variant")
        .card_types(vec![CardType::Artifact])
        .parse_text("{2}, {T}: Draw a card. Transform this artifact.")
        .expect("parse transform-this-artifact activated ability");

    let joined = compiled_lines(&def).join(" ");
    assert!(
        joined.contains("Transform this artifact"),
        "expected artifact self-reference for transform source, got {joined}"
    );
}

#[test]
fn render_choose_between_modes_as_choose_one_or_more() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Modal Variant")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Choose one or more —\n• Destroy target artifact.\n• Destroy target enchantment.\n• Destroy target land.",
        )
        .expect("parse modal choose-one-or-more clause");

    let joined = oracle_like_lines(&def).join(" ");
    assert!(
        joined.contains("Choose one or more"),
        "expected normalized choose-one-or-more header, got {joined}"
    );
}

#[test]
fn render_each_player_create_clause_uses_each_player_creates() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Dragon Crowd")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Each player creates a 5/5 red Dragon creature token with flying.")
        .expect("parse each-player create clause");

    let joined = oracle_like_lines(&def).join(" ");
    assert!(
        joined.contains("Each player creates a 5/5 red Dragon creature token with flying"),
        "expected each-player create compaction, got {joined}"
    );
}

#[test]
fn render_put_counter_on_each_attacking_creature_from_for_each_form() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Fumes Variant")
        .card_types(vec![CardType::Instant])
        .parse_text("Put a -1/-1 counter on each attacking creature.")
        .expect("parse each-attacking counter clause");

    let joined = oracle_like_lines(&def).join(" ");
    assert!(
        joined.contains("Put a -1/-1 counter on each attacking creature"),
        "expected normalized each-attacking counter wording, got {joined}"
    );
}

#[test]
fn render_daze_style_alternative_cost_clause_is_humanized() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Daze Variant")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "You may return an Island you control to its owner's hand rather than pay this spell's mana cost.\nCounter target spell unless its controller pays {1}.",
        )
        .expect("parse daze-style alternative cost");

    let joined = oracle_like_lines(&def).join(" ");
    assert!(
        joined.contains(
            "You may return an Island you control to its owner's hand rather than pay this spell's mana cost"
        ) || joined.contains(
            "You may return a Island you control to its owner's hand rather than pay this spell's mana cost"
        ),
        "expected normalized daze-style alternative cost wording, got {joined}"
    );
}

#[test]
fn render_eldrazi_token_creation_drops_under_your_control_phrase() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Scion Caller")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Create a 1/1 colorless Eldrazi Scion creature token with \"Sacrifice this creature: Add {C}.\"",
        )
        .expect("parse eldrazi scion creation clause");

    let joined = oracle_like_lines(&def).join(" ");
    assert!(
        !joined.contains("under your control"),
        "expected eldrazi token text without explicit control suffix, got {joined}"
    );
}

#[test]
fn parse_conditional_create_token_with_quoted_comma_uses_first_comma_split() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Containment Breach")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Destroy target artifact or enchantment. If its mana value is 2 or less, create a 1/1 black and green Pest creature token with \"When this token dies, you gain 1 life.\"",
        )
        .expect("conditional token clause with quoted comma should parse");

    let joined = oracle_like_lines(&def).join(" ");
    let lower = joined.to_ascii_lowercase();
    assert!(
        lower.contains("if it matches permanent with mana value 2 or less")
            || lower.contains(
                "if the tagged object 'destroyed_0' matches permanent with mana value 2 or less"
            ),
        "expected mana value predicate to stay on destroyed target, got {joined}"
    );
    assert!(
        lower.contains("create a 1/1 black and green pest creature token"),
        "expected pest token creation in conditional true branch, got {joined}"
    );
    assert!(
        lower.contains("when this token dies, you gain 1 life"),
        "expected pest dies trigger text to be preserved, got {joined}"
    );
}

#[test]
fn parse_fatal_push_revolt_clause_keeps_permanent_left_gate() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Fatal Push Variant")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Destroy target creature if it has mana value 2 or less.\nRevolt — Destroy that creature if it has mana value 4 or less instead if a permanent left the battlefield under your control this turn.",
        )
        .expect("fatal push revolt clause should parse");

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("PermanentLeftBattlefieldUnderYourControlThisTurn"),
        "expected revolt gate to compile into a permanent-left condition, got {debug}"
    );

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("mana value 4 or less"),
        "expected revolt branch to preserve the mana value 4 threshold, got {rendered}"
    );
    assert!(
        rendered.contains("mana value 2 or less"),
        "expected base branch to preserve the mana value 2 threshold, got {rendered}"
    );
}

#[test]
fn parse_fatal_push_exposes_single_target_requirement() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Fatal Push Variant")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Destroy target creature if it has mana value 2 or less.\nRevolt — Destroy that creature if it has mana value 4 or less instead if a permanent left the battlefield under your control this turn.",
        )
        .expect("fatal push should parse");

    let Some(effects) = def.spell_effect.as_ref() else {
        panic!("fatal push should compile spell effects");
    };

    let targeting_requirements = effects
        .iter()
        .filter_map(|effect| effect.0.get_target_spec())
        .filter(|spec| match spec {
            crate::target::ChooseSpec::Target(inner)
            | crate::target::ChooseSpec::WithCount(inner, _) => matches!(
                inner.as_ref(),
                crate::target::ChooseSpec::AnyTarget
                    | crate::target::ChooseSpec::PlayerOrPlaneswalker(_)
                    | crate::target::ChooseSpec::Player(_)
                    | crate::target::ChooseSpec::Object(_)
            ),
            crate::target::ChooseSpec::AnyTarget
            | crate::target::ChooseSpec::PlayerOrPlaneswalker(_)
            | crate::target::ChooseSpec::Player(_)
            | crate::target::ChooseSpec::Object(_) => true,
            _ => false,
        })
        .count();

    assert_eq!(
        targeting_requirements, 1,
        "Fatal Push should require exactly one declared target when casting",
    );
}

#[test]
fn parse_conditional_type_list_predicate_uses_rightmost_comma_split() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Gate to the Aether")
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "At the beginning of each player's upkeep, that player reveals the top card of their library. If it's an artifact, creature, enchantment, or land card, the player may put it onto the battlefield.",
        )
        .expect("type-list conditional predicate should parse");

    let joined = oracle_like_lines(&def).join(" ");
    let lower = joined.to_ascii_lowercase();
    assert!(
        lower.contains("if it's an artifact, creature, enchantment, or land card")
            || lower.contains("matches artifact or creature or enchantment or land"),
        "expected full type-list predicate in conditional, got {joined}"
    );
    assert!(
        lower.contains("that player may put it onto the battlefield"),
        "expected true branch to keep put-it effect, got {joined}"
    );
}

#[test]
fn parse_counter_unless_where_x_fails_strictly() {
    let result = CardDefinitionBuilder::new(CardId::from_raw(1), "Rethink Variant")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Counter target spell unless its controller pays {X}, where X is its mana value.",
        );
    match result {
        Ok(def) => {
            let joined = compiled_lines(&def).join(" ").to_ascii_lowercase();
            assert!(
                joined.contains("counter target spell unless") && joined.contains("pays {x}"),
                "expected counter-unless rendering when parse succeeds, got {joined}"
            );
        }
        Err(err) => {
            let joined = format!("{err:?}");
            assert!(
                joined.contains("unsupported where-x clause")
                    || joined.contains("unsupported trailing counter-unless payment clause"),
                "expected where-x strict parse error, got {joined}"
            );
        }
    }
}

#[test]
fn parse_gain_x_plus_life_with_where_clause_binds_x_value() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "An-Havva Inn Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "You gain X plus 1 life, where X is the number of green creatures on the battlefield.",
        )
        .expect("gain-x-plus-life with where clause should parse");

    let joined = oracle_like_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains("number of green creatures")
            && (joined.contains("plus 1 life")
                || joined.contains(
                    "life equal to the number of green creatures on the battlefield plus 1"
                )),
        "expected where-x binding to remain in compiled text, got {joined}"
    );
}

#[test]
fn parse_counter_unless_plus_additional_keeps_dynamic_payment_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Spell Stutter Variant")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Counter target spell unless its controller pays {2} plus an additional {1} for each Faerie you control.",
        )
        .expect("parse counter-unless-plus-additional clause");

    let joined = oracle_like_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains("pays {2}")
            && joined.contains("plus an additional {1} for each faerie you control"),
        "expected dynamic additional payment clause to be preserved, got {joined}"
    );
}

#[test]
fn render_destroy_all_artifacts_and_enchantments_combines_split_sentence() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Purify Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Destroy all artifacts and enchantments.")
        .expect("parse destroy-all artifacts-and-enchantments clause");

    let joined = oracle_like_lines(&def).join(" ");
    assert!(
        joined.contains("Destroy all artifacts and enchantments"),
        "expected combined destroy-all wording, got {joined}"
    );
}

#[test]
fn render_activation_typed_discard_cost_keeps_card_type() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Tortured Existence Variant")
        .parse_text("{B}, Discard a creature card: Return target creature card from your graveyard to your hand.")
        .expect("typed discard activation cost should parse");

    let joined = oracle_like_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains("discard a creature card"),
        "expected typed discard activation cost wording, got {joined}"
    );
}

#[test]
fn render_activation_discard_hand_cost_keeps_full_hand_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Null Brooch Variant")
        .parse_text("{2}, {T}, Discard your hand: Counter target noncreature spell.")
        .expect("discard-hand activation cost should parse");

    let joined = oracle_like_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains("discard your hand"),
        "expected discard-your-hand activation cost wording, got {joined}"
    );
}

#[test]
fn render_activation_random_discard_cost_keeps_random_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Mage il-Vec Variant")
        .parse_text("{T}, Discard a card at random: This creature deals 1 damage to any target.")
        .expect("random discard activation cost should parse");

    let joined = oracle_like_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains("discard a card at random"),
        "expected random discard activation cost wording, got {joined}"
    );
}

#[test]
fn render_activation_return_cost_preserves_numeric_count() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Flooded Shoreline Variant")
        .parse_text("{U}{U}, Return two Islands you control to their owner's hand: Return target creature to its owner's hand.")
        .expect("counted return cost should parse");

    let joined = oracle_like_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains("return two islands you control to their owners' hands")
            || joined.contains("return two islands you control to their owner's hand"),
        "expected counted return activation cost wording, got {joined}"
    );
}

#[test]
fn parse_shard_style_or_tap_activation_cost_uses_single_tap_and_alternative_mana() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Pearl Shard Variant")
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "{3}, {T} or {W}, {T}: Prevent the next 2 damage that would be dealt to any target this turn.",
        )
        .expect("shard-style mana-or-tap activation cost should parse");

    let activated = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("expected activated ability");

    let mana = activated
        .mana_cost
        .mana_cost()
        .expect("expected mana component in activation cost");
    assert_eq!(
        mana.pips().len(),
        1,
        "expected single alternative mana pip in shard-style cost"
    );
    let alternatives = &mana.pips()[0];
    assert!(
        alternatives.contains(&ManaSymbol::Generic(3)) && alternatives.contains(&ManaSymbol::White),
        "expected {{3}} or {{W}} in same mana pip, got {alternatives:?}"
    );

    let tap_cost_count = activated
        .mana_cost
        .costs()
        .iter()
        .filter(|cost| cost.requires_tap())
        .count();
    assert_eq!(
        tap_cost_count, 1,
        "expected exactly one tap cost, got {tap_cost_count}"
    );

    let rendered = oracle_like_lines(&def).join(" ");
    assert!(
        rendered.contains("{3/W}, {T}: Prevent the next 2 damage"),
        "expected rendered shard-style cost to preserve mana-or-tap meaning, got {rendered}"
    );
}

#[test]
fn parse_delayed_return_at_end_of_combat_parses() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Kaijin Variant")
        .parse_text("Return target creature to its owner's hand at end of combat.")
        .expect("delayed end-of-combat return should parse");

    let debug = format!("{:?}", def.spell_effect.as_ref().expect("spell effects"));
    assert!(
        debug.contains("ScheduleDelayedTriggerEffect"),
        "expected delayed trigger scheduling, got {debug}"
    );
    assert!(
        debug.contains("EndOfCombatTrigger"),
        "expected end-of-combat delayed trigger, got {debug}"
    );
    assert!(
        debug.contains("ReturnToHandEffect"),
        "expected delayed return-to-hand payload, got {debug}"
    );
}

#[test]
fn parse_delayed_return_at_next_end_step_parses() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Flicker Variant")
        .parse_text(
            "Exile target creature. Return that card to the battlefield under its owner's control at the beginning of the next end step.",
        )
        .expect("next-end-step return should parse");

    let debug = format!("{:?}", def.spell_effect.as_ref().expect("spell effects"));
    assert!(
        debug.contains("ScheduleDelayedTriggerEffect"),
        "expected delayed trigger scheduling, got {debug}"
    );
    assert!(
        debug.contains("BeginningOfEndStepTrigger"),
        "expected next-end-step delayed trigger, got {debug}"
    );
    assert!(
        debug.contains("MoveToZoneEffect"),
        "expected delayed return-to-battlefield payload, got {debug}"
    );
}

#[test]
fn parse_delayed_return_at_your_next_upkeep_parses() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Upkeep Return Variant")
        .parse_text(
            "Return target creature to its owner's hand at the beginning of your next upkeep.",
        )
        .expect("next-upkeep return should parse");

    let debug = format!("{:?}", def.spell_effect.as_ref().expect("spell effects"));
    assert!(
        debug.contains("ScheduleDelayedTriggerEffect"),
        "expected delayed trigger scheduling, got {debug}"
    );
    assert!(
        debug.contains("BeginningOfUpkeepTrigger"),
        "expected beginning-of-upkeep delayed trigger, got {debug}"
    );
    assert!(
        debug.contains("start_next_turn: true"),
        "expected next-turn gate for next-upkeep trigger, got {debug}"
    );
}

#[test]
fn parse_delayed_destroy_at_end_of_combat_parses() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Basilisk Variant")
        .parse_text("Destroy target creature at end of combat.")
        .expect("delayed destroy at end of combat should parse");

    let debug = format!("{:?}", def.spell_effect.as_ref().expect("spell effects"));
    assert!(
        debug.contains("ScheduleDelayedTriggerEffect"),
        "expected delayed trigger scheduling, got {debug}"
    );
    assert!(
        debug.contains("EndOfCombatTrigger"),
        "expected end-of-combat delayed trigger, got {debug}"
    );
    assert!(
        debug.contains("DestroyEffect"),
        "expected delayed destroy payload, got {debug}"
    );
}

#[test]
fn parse_delayed_destroy_at_next_end_step_parses() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Bearer Variant")
        .parse_text("Destroy all permanents at the beginning of the next end step.")
        .expect("delayed destroy at next end step should parse");

    let debug = format!("{:?}", def.spell_effect.as_ref().expect("spell effects"));
    assert!(
        debug.contains("ScheduleDelayedTriggerEffect"),
        "expected delayed trigger scheduling, got {debug}"
    );
    assert!(
        debug.contains("BeginningOfEndStepTrigger"),
        "expected next-end-step delayed trigger, got {debug}"
    );
    assert!(
        debug.contains("DestroyEffect"),
        "expected delayed destroy payload, got {debug}"
    );
}

#[test]
fn parse_arcbond_delayed_trigger_without_unsupported_fallback_in_allow_unsupported_mode() {
    let parsed = CardDefinitionBuilder::new(CardId::from_raw(1), "Arcbond Variant")
        .card_types(vec![CardType::Instant])
        .parse_text_allow_unsupported(
            "Choose target creature. Whenever that creature is dealt damage this turn, it deals that much damage to each other creature and each player.",
        );

    let def = parsed.expect("arcbond delayed trigger should parse");
    let abilities_debug = format!("{:?}", def.abilities);
    assert!(
        !abilities_debug.contains("UnsupportedParserLine"),
        "arcbond parse should not rely on unsupported fallback marker: {abilities_debug}"
    );

    let spell_debug = format!("{:?}", def.spell_effect.as_ref().expect("spell effects"));
    assert!(
        spell_debug.contains("ScheduleDelayedTriggerEffect"),
        "expected delayed trigger scheduling, got {spell_debug}"
    );
    assert!(
        spell_debug.contains("target_tag: Some"),
        "expected delayed trigger to track a tagged watched object, got {spell_debug}"
    );
    assert!(
        spell_debug.contains("IsDealtDamageTrigger { target: Source }"),
        "expected delayed trigger to watch damage dealt to the tagged object source, got {spell_debug}"
    );
}

#[test]
fn arcbond_delayed_trigger_deals_damage_to_each_other_creature_and_each_player() {
    fn create_creature(
        game: &mut crate::game_state::GameState,
        name: &str,
        controller: PlayerId,
    ) -> ObjectId {
        let id = game.new_object_id();
        let card = crate::card::CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(2)],
                vec![ManaSymbol::Red],
            ]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(3, 3))
            .build();
        let obj =
            crate::object::Object::from_card(id, &card, controller, crate::zone::Zone::Battlefield);
        game.add_object(obj);
        id
    }

    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Arcbond Variant")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Choose target creature. Whenever that creature is dealt damage this turn, it deals that much damage to each other creature and each player.",
        )
        .expect("arcbond delayed trigger should parse");
    let spell_effects = def.spell_effect.clone().expect("spell effects");

    let mut game = crate::game_state::GameState::new(
        vec![
            "Alice".to_string(),
            "Bob".to_string(),
            "Charlie".to_string(),
        ],
        20,
    );
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);

    let chosen_creature = create_creature(&mut game, "Chosen", alice);
    let other_creature_one = create_creature(&mut game, "Other One", bob);
    let other_creature_two = create_creature(&mut game, "Other Two", charlie);

    let spell_source = game.new_object_id();
    let mut ctx =
        crate::executor::ExecutionContext::new_default(spell_source, alice).with_targets(vec![
            crate::executor::ResolvedTarget::Object(chosen_creature),
        ]);
    for effect in &spell_effects {
        crate::executor::execute_effect(&mut game, effect, &mut ctx)
            .expect("spell effect execution should succeed");
    }

    assert_eq!(
        game.delayed_triggers.len(),
        1,
        "expected one delayed trigger"
    );
    assert_eq!(
        game.delayed_triggers[0].target_objects,
        vec![chosen_creature],
        "expected delayed trigger watcher to be the chosen creature"
    );

    let damage_event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::DamageEvent::new(
            other_creature_one,
            crate::game_event::DamageTarget::Object(chosen_creature),
            3,
            false,
        ),
        crate::provenance::ProvNodeId::default(),
    );
    let delayed_entries = crate::triggers::check_delayed_triggers(&mut game, &damage_event);
    assert_eq!(
        delayed_entries.len(),
        1,
        "expected arcbond delayed trigger to fire once"
    );

    let mut trigger_queue = crate::triggers::TriggerQueue::new();
    for entry in delayed_entries {
        trigger_queue.add(entry);
    }
    crate::game_loop::put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("put delayed trigger on stack");
    assert_eq!(game.stack.len(), 1, "expected delayed trigger on stack");

    crate::game_loop::resolve_stack_entry(&mut game).expect("resolve delayed trigger");

    assert_eq!(
        game.damage_on(other_creature_one),
        3,
        "first other creature should be dealt matching damage"
    );
    assert_eq!(
        game.damage_on(other_creature_two),
        3,
        "second other creature should be dealt matching damage"
    );
    assert_eq!(
        game.damage_on(chosen_creature),
        0,
        "chosen creature should not be in the 'each other creature' fanout"
    );

    assert_eq!(game.player(alice).expect("alice should exist").life, 17);
    assert_eq!(game.player(bob).expect("bob should exist").life, 17);
    assert_eq!(game.player(charlie).expect("charlie should exist").life, 17);
}

#[test]
fn parse_counter_unless_or_mana_choice_fails_strictly() {
    let err = CardDefinitionBuilder::new(CardId::from_raw(1), "Thrull Wizard Variant")
        .parse_text("Counter target black spell unless that spell's controller pays {B} or {3}.")
        .expect_err("alternative mana unless-payment should fail strict parse");
    let debug = format!("{err:?}");
    assert!(
        debug.contains("unsupported trailing counter-unless payment clause")
            || debug.contains("unsupported trailing unless-payment clause"),
        "expected strict trailing unless-payment parse error, got {debug}"
    );
}

#[test]
fn parse_exile_it_unless_discard_creature_card_as_unless_action() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Body Snatcher Variant")
        .parse_text("When this creature enters, exile it unless you discard a creature card.")
        .expect("triggered unless-discard clause should parse");

    let abilities_debug = format!("{:?}", def.abilities);
    assert!(
        abilities_debug.contains("UnlessActionEffect"),
        "expected unless-action lowering, got {abilities_debug}"
    );
    assert!(
        abilities_debug.contains("DiscardEffect"),
        "expected discard alternative action, got {abilities_debug}"
    );
    assert!(
        abilities_debug.contains("tag: TagKey(\"triggering\")"),
        "expected triggering-object tag for 'it', got {abilities_debug}"
    );

    let joined = oracle_like_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains("unless you discard a creature card"),
        "expected unless-discard wording to render, got {joined}"
    );
}

#[test]
fn render_named_count_filter_keeps_named_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Powerstone Shard Variant")
        .parse_text("{T}: Add {C} for each artifact you control named Powerstone Shard.")
        .expect("named count filter should parse");

    let joined = oracle_like_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains("named powerstone shard"),
        "expected named count filter wording, got {joined}"
    );
}

#[test]
fn render_named_filter_preserves_articles_in_card_name() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Cleric of the Forward Order Variant")
        .parse_text(
            "When this creature enters, you gain 2 life for each creature you control named Cleric of the Forward Order.",
        )
        .expect("named filter with article should parse");

    let joined = oracle_like_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains("named cleric of the forward order"),
        "expected named filter to keep articles in card name, got {joined}"
    );
}

#[test]
fn render_nonsnow_filter_keeps_non_supertype() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Hallowed Ground Variant")
        .parse_text("{W}{W}: Return target nonsnow land you control to its owner's hand.")
        .expect("nonsnow target filter should parse");

    let joined = oracle_like_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains("nonsnow land you control"),
        "expected nonsnow target filter wording, got {joined}"
    );
}

#[test]
fn parse_semantic_guard_is_disabled_by_default() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Semantic Guard Baseline Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Flying")
        .expect("semantic guard should be opt-in by env var");

    let joined = oracle_like_lines(&def).join(" ");
    assert!(
        joined.contains("Flying"),
        "expected parsed output while semantic guard is disabled, got {joined}"
    );
}

#[test]
fn parse_shared_color_prevent_fanout_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Radiance Prevent Variant")
        .parse_text(
            "Prevent the next 1 damage that would be dealt to target creature and each other creature that shares a color with it this turn.",
        )
        .expect("shared-color prevent fanout should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("prevent the next 1 damage") && rendered.contains("target creature"),
        "expected primary prevent target clause, got {rendered}"
    );
    assert!(
        rendered.contains("shares a color with that object")
            || !rendered.contains("unsupported parser line fallback"),
        "expected shared-color clause to avoid fallback rendering, got {rendered}"
    );
}

#[test]
fn parse_shared_color_gain_ability_fanout_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Radiance Gain Variant")
        .parse_text(
            "Radiance — Target creature and each other creature that shares a color with it gain haste until end of turn.",
        )
        .expect("shared-color gain fanout should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("shares a color with that object"),
        "expected shared-color fanout filter, got {rendered}"
    );
    assert!(
        rendered.contains("haste until end of turn"),
        "expected haste grant to fanout targets, got {rendered}"
    );
}

#[test]
fn parse_shared_color_pump_fanout_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Radiance Pump Variant")
        .parse_text(
            "Radiance — Target creature and each other creature that shares a color with it get +1/+1 until end of turn.",
        )
        .expect("shared-color pump fanout should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("shares a color with that object"),
        "expected shared-color fanout filter, got {rendered}"
    );
    assert!(
        rendered.contains("+1/+1"),
        "expected +1/+1 pump to be preserved, got {rendered}"
    );
}

#[test]
fn parse_shared_color_damage_with_named_subject_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Radiance Damage Variant")
        .parse_text(
            "Radiance — Cleansing Beam deals 2 damage to target creature and each other creature that shares a color with it.",
        )
        .expect("named-subject shared-color damage fanout should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("deal 2 damage to target creature"),
        "expected primary target damage clause, got {rendered}"
    );
    assert!(
        rendered.contains("shares a color with that object"),
        "expected shared-color fanout damage clause, got {rendered}"
    );
}

#[test]
fn parse_counter_unless_then_counter_that_spell_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Counter That Spell Variant")
        .parse_text(
            "Counter target noncreature spell unless its controller pays {1}. If you control a creature with power 4 or greater, counter that spell instead.",
        )
        .expect("counter-that-spell follow-up should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("counter target noncreature spell"),
        "expected base counter clause, got {rendered}"
    );
    assert!(
        rendered.contains("if you control a creature with power 4 or greater")
            && rendered.contains("otherwise, counter target noncreature spell unless"),
        "expected conditional replacement to keep shared target semantics, got {rendered}"
    );
}

#[test]
fn parse_additional_cost_sacrificed_power_reference_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Fling")
        .parse_text(
            "As an additional cost to cast this spell, sacrifice a creature.\nFling deals damage equal to the sacrificed creature's power to any target.",
        )
        .expect("sacrificed-power follow-up should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("sacrifice")
            && rendered.contains("deals damage equal to")
            && rendered.contains("power"),
        "expected additional-cost sacrificed-power linkage, got {rendered}"
    );
}

#[test]
fn parse_named_spell_exile_self_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Burning Wish")
        .parse_text("Exile Burning Wish.")
        .expect("named self-exile clause should parse");
    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("exile"),
        "expected exile-self clause to remain present, got {rendered}"
    );
}

#[test]
fn parse_delayed_next_end_step_sentence_schedules_trigger() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Planebound Variant")
        .parse_text("{R}: You may put a planeswalker card from your hand onto the battlefield. Sacrifice it at the beginning of the next end step.")
        .expect("next-end-step delayed sacrifice should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("beginning of the next end step"),
        "expected delayed next-end-step wording, got {rendered}"
    );
}

#[test]
fn parse_delayed_next_end_step_sentence_with_named_creature_keeps_delay() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Sneak Attack Variant")
        .parse_text(
            "{R}: You may put a creature card from your hand onto the battlefield. That creature gains haste. Sacrifice the creature at the beginning of the next end step.",
        )
        .expect("named delayed next-end-step sacrifice should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("beginning of the next end step"),
        "expected delayed next-end-step wording, got {rendered}"
    );
    assert!(
        !rendered.contains("sacrifice a creature"),
        "expected delayed clause not to collapse to generic immediate sacrifice, got {rendered}"
    );
}

#[test]
fn parse_delayed_next_end_step_sentence_with_this_creature_keeps_source_reference() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Pyric Variant")
        .parse_text(
            "{R}: This creature gets +1/+0 until end of turn. Sacrifice this creature at the beginning of the next end step.",
        )
        .expect("self-referential delayed next-end-step sacrifice should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("beginning of the next end step"),
        "expected delayed next-end-step wording, got {rendered}"
    );
    assert!(
        !rendered.contains("sacrifice a creature"),
        "expected self-referential sacrifice not to collapse to generic creature, got {rendered}"
    );
}

#[test]
fn parse_object_filter_with_entered_since_last_turn_ended_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Premature Burial Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Destroy target nonblack creature that entered since your last turn ended.")
        .expect("entered-since-last-turn qualifier should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("entered since your last turn ended"),
        "expected entered-since-last-turn qualifier in rendered output, got {rendered}"
    );
}

#[test]
fn parse_when_you_do_followup_clause_as_reflexive_trigger() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Invasion Variant")
        .card_types(vec![CardType::Battle])
        .parse_text(
            "When this permanent enters, you may sacrifice an artifact or creature. When you do, exile target artifact or creature an opponent controls.",
        )
        .expect("when-you-do followup clause should parse as a reflexive trigger");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("when you do"),
        "expected reflexive followup to keep when-you-do linkage, got {rendered}"
    );

    let debug = format!("{:#?}", def.abilities);
    assert!(
        debug.contains("ReflexiveTriggerEffect"),
        "expected reflexive followup lowering, got {debug}"
    );
}

#[test]
fn parse_modal_trigger_header_keeps_prefix_effect_and_result_gate() {
    let err = CardDefinitionBuilder::new(CardId::from_raw(1), "Immard Variant")
        .parse_text(
            "Whenever this creature enters or attacks, put a charge counter on it or remove one from it. When you remove a counter this way, choose one —\n• This creature deals 4 damage to any target.\n• This creature gains lifelink and indestructible until end of turn.",
        )
        .expect_err("specific this-way result gating should fail until modeled precisely");
    let rendered = format!("{err:?}").to_ascii_lowercase();
    assert!(
        rendered.contains("this way")
            || rendered.contains("unsupported predicate")
            || rendered.contains("unsupported target phrase"),
        "expected strict this-way modal gating rejection, got {rendered}"
    );
}

#[test]
fn parse_aura_barbs_attached_target_contraction_keeps_second_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Aura Barbs Variant")
        .parse_text(
            "Each enchantment deals 2 damage to its controller, then each Aura attached to a creature deals 2 damage to the creature it's attached to.",
        )
        .expect("attached-target contraction should parse");

    let spell_debug = format!("{:?}", def.spell_effect);
    assert!(
        spell_debug.matches("ForEachObject").count() >= 2,
        "expected both for-each damage clauses, got {spell_debug}"
    );
    assert!(
        spell_debug.contains("AttachedToTaggedObject"),
        "expected second clause target to stay linked to attached object, got {spell_debug}"
    );

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("for each aura"),
        "expected rendered text to keep aura damage clause, got {rendered}"
    );
}

#[test]
fn parse_for_each_of_x_target_permanents_builds_choose_then_for_each_tagged() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Doppelgang Variant")
        .parse_text(
            "For each of X target permanents, create X tokens that are copies of that permanent.",
        )
        .expect("for-each of X target permanents should parse");

    let spell_debug = format!("{:?}", def.spell_effect);
    assert!(
        spell_debug.contains("ChooseObjectsEffect"),
        "expected explicit target choice effect, got {spell_debug}"
    );
    assert!(
        spell_debug.contains("dynamic_x: true"),
        "expected dynamic X target count, got {spell_debug}"
    );
    assert!(
        spell_debug.contains("ForEachTaggedEffect"),
        "expected per-target iteration over chosen objects, got {spell_debug}"
    );
    assert!(
        spell_debug.contains("CreateTokenCopyEffect"),
        "expected token copy follow-up effect, got {spell_debug}"
    );

    let rendered = compiled_lines(&def).join(" ");
    assert!(
        rendered.to_ascii_lowercase().contains("for each tagged")
            || rendered
                .to_ascii_lowercase()
                .contains("for each of x target permanents"),
        "expected rendered text to keep 'for each of X target permanents', got {rendered}"
    );
}

#[test]
fn parse_modal_choose_up_to_x_header_preserves_dynamic_bounds() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Dynamic Modes Variant")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Choose up to X —\n• Counter target spell.\n• Draw a card.\n• Create a Treasure token.",
        )
        .expect("choose-up-to-X modal header should parse");

    let modal = def
        .spell_effect
        .as_ref()
        .and_then(|effects| {
            effects
                .iter()
                .find_map(|effect| effect.downcast_ref::<ChooseModeEffect>())
        })
        .expect("expected choose-mode effect");
    assert!(matches!(modal.choose_count, Value::X));
    assert!(
        matches!(modal.min_choose_count, Some(Value::Fixed(0))),
        "expected zero minimum for choose-up-to-X header, got {modal:?}"
    );

    let rendered = compiled_lines(&def).join(" ");
    assert!(
        rendered.to_ascii_lowercase().contains("choose up to x"),
        "expected rendered text to keep choose-up-to-X header, got {rendered}"
    );
}

#[test]
fn parse_nonhistoric_filter_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Historic Filter Variant")
        .parse_text("Return each nonland permanent that's not historic to its owner's hand.")
        .expect("nonhistoric filter should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("not historic"),
        "expected nonhistoric clause wording, got {rendered}"
    );
}

#[test]
fn parse_same_name_damage_fanout_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Homing Lightning")
        .parse_text(
            "Homing Lightning deals 4 damage to target creature and each other creature with the same name as that creature.",
        )
        .expect("same-name damage fanout should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("deal 4 damage to target creature"),
        "expected primary targeted damage clause, got {rendered}"
    );
    assert!(
        rendered.contains("with the same name as that object"),
        "expected same-name fanout wording, got {rendered}"
    );
}

#[test]
fn parse_same_name_return_from_graveyard_fanout_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Echoing Return")
        .parse_text(
            "Return target creature card and all other cards with the same name as that card from your graveyard to your hand.",
        )
        .expect("same-name return fanout from graveyard should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("from your graveyard to your hand"),
        "expected graveyard-to-hand return destination, got {rendered}"
    );
    assert!(
        !rendered.contains("to its owner's hand"),
        "expected graveyard return wording, got {rendered}"
    );
}

#[test]
fn parse_each_of_up_to_target_damage_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Wrap in Flames")
        .parse_text(
            "Wrap in Flames deals 1 damage to each of up to three target creatures. Those creatures can't block this turn.",
        )
        .expect("each-of-up-to-target damage clause should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("up to three target creatures"),
        "expected targeted damage count wording, got {rendered}"
    );
    assert!(
        !rendered.contains("for each creature"),
        "expected targeted (not global each-creature) damage wording, got {rendered}"
    );
}

#[test]
fn parse_spell_delayed_trigger_this_turn_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Song of Blood Variant")
        .parse_text(
            "Mill four cards. Whenever a creature attacks this turn, it gets +1/+0 until end of turn for each creature card put into your graveyard this way.",
        )
        .expect("spell delayed trigger clause should parse");

    let spell_debug = format!("{:?}", def.spell_effect);
    assert!(
        spell_debug.contains("ScheduleDelayedTriggerEffect"),
        "expected delayed trigger scheduling effect, got {spell_debug}"
    );
    assert!(
        spell_debug.contains("until_end_of_turn: true"),
        "expected delayed trigger to expire at end of turn, got {spell_debug}"
    );

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("whenever creature attacks this turn"),
        "expected rendered delayed trigger wording, got {rendered}"
    );
}

#[test]
fn parse_target_player_sacrifices_artifact_and_land_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Structural Collapse")
        .parse_text(
            "Target player sacrifices an artifact and a land of their choice. Structural Collapse deals 2 damage to that player.",
        )
        .expect("artifact-and-land sacrifice clause should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("artifact") && rendered.contains("land"),
        "expected both artifact and land sacrifice wording, got {rendered}"
    );
    assert!(
        !rendered.contains("artifact or land"),
        "expected split sacrifice effects rather than artifact-or-land, got {rendered}"
    );
}

#[test]
fn parse_each_player_create_uses_each_player_controller() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Each Player Cat Variant")
        .parse_text("Each player creates a Food token.")
        .expect("each-player token creation should parse");

    let rendered = compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();
    assert!(
        rendered_lower.contains("each player creates"),
        "expected each-player create phrasing, got {rendered}"
    );
    assert!(
        rendered_lower.contains("that player's control"),
        "expected iterated-player token controller, got {rendered}"
    );
}

#[test]
fn parse_create_for_each_tail_does_not_pollute_token_name() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Tail Token Name Variant")
        .parse_text("Create a Food token for each untapped artifact you control.")
        .expect("create-for-each tail should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        !rendered.contains("food untapped artifact"),
        "expected token name to remain Food, got {rendered}"
    );
}

#[test]
fn parse_ward_pay_life_line_as_static_marker() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Ward Life Variant")
        .parse_text("Ward—Pay 3 life.")
        .expect("ward-pay-life line should parse");

    let rendered = compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Ward—Pay 3 life"),
        "expected ward-pay-life marker text, got {rendered}"
    );
    assert!(
        !rendered.to_ascii_lowercase().contains("you lose 3 life"),
        "ward-pay-life should not lower as a standalone lose-life spell effect, got {rendered}"
    );
}

#[test]
fn parse_ward_mana_and_life_line_as_static_ability() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Ward Hybrid Cost Variant")
        .parse_text("Ward—{2}, Pay 2 life.")
        .expect("ward mixed-cost line should parse");

    let rendered = compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();
    assert!(
        rendered_lower.contains("ward"),
        "expected ward text in compiled output, got {rendered}"
    );
    assert!(
        !rendered_lower.contains("you lose 2 life"),
        "ward mixed cost should not lower as standalone lose-life spell effect, got {rendered}"
    );
}

#[test]
fn parse_ward_discard_multiple_card_types_as_static_ability() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Ward Typed Discard Variant")
        .parse_text("Ward—Discard an enchantment, instant, or sorcery card.")
        .expect("ward typed-discard line should parse");

    let rendered = compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();
    assert!(
        rendered_lower.contains("ward")
            && rendered_lower.contains("discard")
            && rendered_lower.contains("enchantment")
            && rendered_lower.contains("instant")
            && rendered_lower.contains("sorcery"),
        "expected ward typed-discard wording in compiled output, got {rendered}"
    );

    let debug = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        !debug.contains("keyword_marker") && !debug.contains("staticabilityid::custom"),
        "ward typed-discard should lower to a real ward static ability, got {debug}"
    );
}

#[test]
fn render_if_they_dont_uses_negative_may_condition() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Umbilicus Variant")
        .parse_text(
            "At the beginning of each player's upkeep, that player may pay 2 life. If they don't, they return a permanent they control to its owner's hand.",
        )
        .expect("if-they-dont sentence should parse");

    let rendered = compiled_lines(&def).join(" ");
    let lower = rendered.to_ascii_lowercase();
    assert!(
        lower.contains("doesn't"),
        "expected negative may/if phrasing, got {rendered}"
    );
    assert!(
        !lower.contains("if that player does,"),
        "did-not branch should not be rendered as affirmative branch, got {rendered}"
    );
}

#[test]
fn render_cost_prefixed_each_player_draw_discard_compacts() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Lore Broker Variant")
        .parse_text("{T}: Each player draws a card, then discards a card.")
        .expect("draw-then-discard should parse");

    let rendered = compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("{T}: Each player draws a card, then discards a card."),
        "expected compact each-player draw/discard wording, got {rendered}"
    );
}

#[test]
fn render_attack_skip_untap_uses_controller_next_untap_step() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Apes Variant")
        .parse_text(
            "Whenever this creature attacks, it doesn't untap during its controller's next untap step.",
        )
        .expect("attack untap-skip line should parse");

    let rendered = compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("doesn't untap during its controller's next untap step"),
        "expected controller-next-untap-step wording, got {rendered}"
    );
}

#[test]
fn parse_combat_damage_tap_then_doesnt_untap_sentence() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Kashi Variant")
        .parse_text(
            "Whenever this creature deals combat damage to a creature, tap that creature and it doesn't untap during its controller's next untap step.",
        )
        .expect("combat-damage tap+untap-skip trigger should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("tap that creature") || rendered.contains("tap it"),
        "expected tap follow-up wording, got {rendered}"
    );
    assert!(
        rendered.contains("doesn't untap during its controller's next untap step")
            || rendered.contains("doesnt untap during its controller's next untap step")
            || rendered.contains("can't untap during its controller's next untap step")
            || rendered.contains("cant untap during its controller's next untap step"),
        "expected controller-next-untap-step wording, got {rendered}"
    );
}

#[test]
fn parse_rejects_three_dog_aura_copy_attachment_clause() {
    let err = CardDefinitionBuilder::new(CardId::from_raw(1), "Three Dog Variant")
        .parse_text(
            "Whenever you attack, you may pay {2} and sacrifice an Aura attached to this creature. When you sacrifice an Aura this way, for each other attacking creature you control, create a token that's a copy of that Aura attached to that creature.",
        )
        .expect_err("unsupported aura-copy attachment fanout should not partially parse");

    let message = format!("{err:?}").to_ascii_lowercase();
    assert!(
        message.contains("unsupported parser line")
            || message.contains("unsupported known partial parse pattern")
            || message.contains("unsupported aura-copy attachment fanout clause"),
        "expected explicit unsupported rejection, got {message}"
    );
}

#[test]
fn parse_defending_player_suffix_subject_keeps_player_binding() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Keeper Variant")
        .parse_text(
            "Whenever this creature attacks and isn't blocked, defending player loses 2 life.",
        )
        .expect("parse defending-player suffix subject");

    let joined = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains("defending player loses 2 life"),
        "expected defending-player life-loss wording, got {joined}"
    );
}

#[test]
fn parse_rejects_assigns_no_combat_damage_clause() {
    let err = CardDefinitionBuilder::new(CardId::from_raw(1), "Keeper Reject Variant")
        .parse_text(
            "Whenever this creature attacks and isn't blocked, it assigns no combat damage this turn and defending player loses 2 life.",
        )
        .expect_err("assigns-no-combat-damage clause should not partially parse");

    let message = format!("{err:?}").to_ascii_lowercase();
    assert!(
        message.contains("assigns-no-combat-damage")
            || message.contains("unsupported triggered line")
            || message.contains("unsupported parser line")
            || message.contains("unsupported known partial parse pattern"),
        "expected assigns-no-combat-damage rejection, got {message}"
    );
}

#[test]
fn parse_rejects_defending_players_choice_clause() {
    let err = CardDefinitionBuilder::new(CardId::from_raw(1), "Erithizon Reject Variant")
        .parse_text(
            "Whenever this creature attacks, put a +1/+1 counter on target creature of defending player's choice.",
        )
        .expect_err("defending player's choice clause should not partially parse");

    let message = format!("{err:?}").to_ascii_lowercase();
    assert!(
        message.contains("defending-players-choice")
            || message.contains("unsupported parser line")
            || message.contains("unsupported known partial parse pattern"),
        "expected defending player's choice rejection, got {message}"
    );
}

#[test]
fn parse_rejects_creature_token_player_planeswalker_target_clause() {
    let err = CardDefinitionBuilder::new(CardId::from_raw(1), "Coalborn Reject Variant")
        .parse_text("{2}{R}: This creature deals 1 damage to target creature token, player, or planeswalker.")
        .expect_err("creature-token/player/planeswalker target clause should not partially parse");

    let message = format!("{err:?}").to_ascii_lowercase();
    assert!(
        message.contains("creature-token/player/planeswalker")
            || message.contains("unsupported parser line")
            || message.contains("unsupported known partial parse pattern"),
        "expected creature-token/player/planeswalker rejection, got {message}"
    );
}

#[test]
fn parse_rejects_if_you_sacrifice_an_island_this_way_clause() {
    let err = CardDefinitionBuilder::new(CardId::from_raw(1), "Serendib Reject Variant")
        .parse_text(
            "At the beginning of your upkeep, sacrifice a land. If you sacrifice an Island this way, this creature deals 3 damage to you.",
        )
        .expect_err("if-you-sacrifice-an-island-this-way clause should not partially parse");

    let message = format!("{err:?}").to_ascii_lowercase();
    assert!(
        message.contains("if-you-sacrifice-an-island-this-way")
            || message.contains("if you sacrifice an island this way")
            || message.contains("unsupported triggered line")
            || message.contains("unsupported parser line")
            || message.contains("unsupported known partial parse pattern"),
        "expected island-this-way rejection, got {message}"
    );
}

#[test]
fn render_rain_of_daggers_uses_destroyed_this_way_life_loss_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Rain of Daggers Variant")
        .parse_text(
            "Destroy all creatures target opponent controls. You lose 2 life for each creature destroyed this way.",
        )
        .expect("rain-of-daggers style text should parse");

    let rendered = compiled_lines(&def).join(" ");
    let lower = rendered.to_ascii_lowercase();
    assert!(
        lower.contains("destroy all target opponent's creatures")
            || lower.contains("destroy all creatures target opponent controls"),
        "expected opponent-controls destroy-all clause, got {rendered}"
    );
    assert!(
        lower.contains("lose 2 life for each creature destroyed this way"),
        "expected destroyed-this-way life-loss clause, got {rendered}"
    );
}

#[test]
fn render_artifact_or_tapped_creature_does_not_require_tapped_artifacts() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Radiant Strike Variant")
        .parse_text("Destroy target artifact or tapped creature. You gain 3 life.")
        .expect("artifact-or-tapped-creature line should parse");

    let rendered = compiled_lines(&def).join(" ");
    let lower = rendered.to_ascii_lowercase();
    assert!(
        !lower.contains("tapped artifact or creature"),
        "expected tapped to apply only to creature side of disjunction, got {rendered}"
    );
}

#[test]
fn render_named_angel_token_keeps_explicit_pt_and_keywords() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Battle at the Helvault Variant")
        .parse_text(
            "Create Avacyn, a legendary 8/8 white Angel creature token with flying, vigilance, and indestructible.",
        )
        .expect("named angel token line should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("8/8 white angel creature token"),
        "expected explicit 8/8 angel token, got {rendered}"
    );
    assert!(
        rendered.contains("vigilance") && rendered.contains("indestructible"),
        "expected explicit vigilance and indestructible keywords, got {rendered}"
    );
}

#[test]
fn render_exile_until_clause_keeps_target_filter_without_until_tail() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Liminal Hold Variant")
        .parse_text(
            "When this enchantment enters, exile up to one target nonland permanent an opponent controls until this enchantment leaves the battlefield.",
        )
        .expect("exile-until line should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("target nonland permanent an opponent controls"),
        "expected nonland-permanent target filter, got {rendered}"
    );
}

#[test]
fn parse_look_at_target_players_hand_keeps_targeting() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Glasses Variant")
        .parse_text("{T}: Look at target player's hand.")
        .expect("target-hand look clause should parse");

    let rendered = compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Look at target player's hand."),
        "expected explicit target player hand wording, got {rendered}"
    );
}

#[test]
fn parse_each_player_who_controls_condition_wraps_conditional() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Shatter Variant")
        .parse_text(
            "Each player who controls a creature with power 4 or greater draws a card. Then destroy all creatures.",
        )
        .expect("each-player conditional clause should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("for each player, if that player controls")
            && rendered.contains("power 4 or greater"),
        "expected per-player control condition, got {rendered}"
    );
}

#[test]
fn parse_may_exile_then_return_same_object_keeps_followup_return() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Conjurer's Closet Variant")
        .parse_text(
            "At the beginning of your end step, you may exile target creature you control, then return that card to the battlefield under your control.",
        )
        .expect("may exile-then-return line should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("exile target creature you control"),
        "expected exile clause to remain, got {rendered}"
    );
    assert!(
        rendered.contains("return")
            && rendered.contains("battlefield")
            && rendered.contains("under your control"),
        "expected return-to-battlefield followup, got {rendered}"
    );
}

#[test]
fn parse_hazorets_favor_keeps_delayed_sacrifice_followup() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Hazoret's Favor Variant")
        .parse_text(
            "At the beginning of combat on your turn, you may have target creature you control get +2/+0 and gain haste until end of turn. If you do, sacrifice it at the beginning of the next end step.",
        )
        .expect("hazorets favor line should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("next end step") && rendered.contains("sacrifice"),
        "expected delayed sacrifice followup, got {rendered}"
    );
}

#[test]
fn parse_earthbend_then_earthbend_chain_keeps_both_and_life_gain() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Cracked Earth Technique Variant")
        .parse_text("Earthbend 3, then earthbend 3. You gain 3 life.")
        .expect("earthbend then earthbend line should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.matches("earthbend 3").count() >= 2,
        "expected both earthbend clauses, got {rendered}"
    );
    assert!(
        rendered.contains("gain 3 life"),
        "expected trailing life gain clause, got {rendered}"
    );
}

#[test]
fn parse_search_basic_triple_and_gain_life_keeps_all_components() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Brokers Hideout Variant")
        .parse_text(
            "When this land enters, sacrifice it. When you do, search your library for a basic Forest, Plains, or Island card, put it onto the battlefield tapped, then shuffle and you gain 1 life.",
        )
        .expect("search basic triple line should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("forest") && rendered.contains("plains") && rendered.contains("island"),
        "expected all three basic land types in search filter, got {rendered}"
    );
    assert!(
        rendered.contains("gain 1 life"),
        "expected trailing life gain clause, got {rendered}"
    );
}

#[test]
fn parse_search_put_discard_random_then_shuffle_keeps_discard_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Gamble Variant")
        .parse_text(
            "Search your library for a card, put that card into your hand, discard a card at random, then shuffle.",
        )
        .expect("search-discard-random-then-shuffle clause should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("search your library for a card"),
        "expected search-library clause to remain, got {rendered}"
    );
    assert!(
        rendered.contains("discard a card at random"),
        "expected discard-at-random clause to remain, got {rendered}"
    );
    assert!(
        rendered.contains("shuffle"),
        "expected shuffle clause to remain, got {rendered}"
    );
}

#[test]
fn parse_once_each_turn_play_from_exile_line_is_rejected() {
    let err = CardDefinitionBuilder::new(CardId::from_raw(1), "Evelyn Variant")
        .parse_text(
            "Once each turn, you may play a card from exile with a collection counter on it if it was exiled by an ability you controlled, and you may spend mana as though it were mana of any color to cast it.",
        )
        .expect_err("once-each-turn play-from-exile fallback line should be rejected");
    let debug = format!("{err:?}").to_ascii_lowercase();
    assert!(
        debug.contains("unsupported static clause"),
        "expected unsupported static clause error, got {debug}"
    );
}

#[test]
fn parse_manabond_reveal_hand_put_lands_from_it() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Manabond Variant")
        .parse_text(
            "At the beginning of your end step, you may reveal your hand and put all land cards from it onto the battlefield. If you do, discard your hand.",
        )
        .expect("manabond reveal/put-from-it clause should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("reveal your hand"),
        "expected reveal-hand rendering, got {rendered}"
    );
    assert!(
        rendered.contains("from your hand") || rendered.contains("your hand to the battlefield"),
        "expected lands to be moved from hand, got {rendered}"
    );
}

#[test]
fn render_each_player_puts_card_from_hand_on_top() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Sadistic Variant")
        .parse_text(
            "When this creature dies, each player puts a card from their hand on top of their library.",
        )
        .expect("sadistic-augermage style clause should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("each player puts a card from their hand on top of their library"),
        "expected compact each-player puts wording, got {rendered}"
    );
}

#[test]
fn parse_conditional_doesnt_untap_static_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Alirios Variant")
        .parse_text(
            "This creature doesn't untap during your untap step if you control a Reflection.",
        )
        .expect("conditional doesn't-untap line should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("doesn't untap during your untap step if you control a reflection")
            || rendered.contains("doesnt untap during your untap step if you control a reflection"),
        "expected untap condition to be preserved, got {rendered}"
    );
}

#[test]
fn parse_then_if_conditional_sentence_is_preserved() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Then If Variant")
        .parse_text(
            "Target creature gets +1/+1 until end of turn. Then if you control a creature with power 4 or greater, draw a card.",
        )
        .expect("then-if conditional sentence should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("if you control a creature with power 4 or greater")
            && rendered.contains("draw a card"),
        "expected then-if conditional to remain in compiled output, got {rendered}"
    );
}

#[test]
fn parse_additional_cost_and_trigger_when_on_same_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Additional Cost Split Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "As an additional cost to cast this spell, sacrifice a creature. When this creature enters, each opponent loses 4 life.",
        )
        .expect("additional-cost line with trailing trigger sentence should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("as an additional cost to cast this spell")
            && rendered.contains("when this creature enters"),
        "expected both additional-cost and trigger clauses, got {rendered}"
    );
    assert!(
        !rendered.contains("whenever as an additional cost"),
        "expected additional-cost clause to stay out of triggered text, got {rendered}"
    );
}

#[test]
fn parse_additional_cost_or_chain_renders_inline_or_options() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Additional Cost Choice Variant")
        .parse_text(
            "As an additional cost to cast this spell, sacrifice a creature, discard a card, or pay 4 life. Draw a card.",
        )
        .expect("additional-cost or-chain should parse as a choice");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("as an additional cost to cast this spell")
            && rendered.contains("sacrifice a creature, discard a card, or pay 4 life"),
        "expected additional cost to preserve inline or-options, got {rendered}"
    );
    assert!(
        rendered.contains("sacrifice a creature")
            && rendered.contains("discard a card")
            && (rendered.contains("pay 4 life") || rendered.contains("lose 4 life")),
        "expected all additional-cost options to remain in compiled text, got {rendered}"
    );
    assert!(
        !rendered.contains("sacrifice a creature. discard a card. pay 4 life")
            && !rendered.contains("sacrifice a creature. you discard a card. you lose 4 life"),
        "expected additional costs to remain alternatives, not cumulative mandatory costs, got {rendered}"
    );
}

#[test]
fn parse_distribute_counters_among_any_number_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Invoke Variant")
        .parse_text(
            "Return target permanent card from your graveyard to the battlefield, then distribute four +1/+1 counters among any number of creatures and/or Vehicles target player controls.",
        )
        .expect("distribute-counters clause should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("return target")
            && rendered.contains("+1/+1")
            && rendered.contains("vehicle"),
        "expected return and distributed counters clause, got {rendered}"
    );
}

#[test]
fn parse_distribute_counters_one_or_two_targets() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Elven Rite Variant")
        .parse_text("Distribute two +1/+1 counters among one or two target creatures.")
        .expect("one-or-two distributed counters clause should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("one or two")
            && rendered.contains("+1/+1")
            && rendered.contains("target creatures"),
        "expected one-or-two target distribute wording, got {rendered}"
    );
}

#[test]
fn parse_distribute_counters_one_two_or_three_targets() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Biogenic Variant")
        .parse_text("Distribute three +1/+1 counters among one, two, or three target creatures.")
        .expect("one-two-or-three distributed counters clause should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("one, two, or three")
            || rendered.contains("one or two or three")
            || rendered.contains("up to three"),
        "expected plural distributed target count, got {rendered}"
    );
}

#[test]
fn parse_distribute_then_double_counters_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Biogenic Upgrade Variant")
        .parse_text(
            "Distribute three +1/+1 counters among one, two, or three target creatures, then double the number of +1/+1 counters on each of those creatures.",
        )
        .expect("distribute-then-double counters clause should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("one, two, or three")
            || rendered.contains("one or two or three")
            || rendered.contains("up to three"),
        "expected distributed target count to remain plural, got {rendered}"
    );
    assert!(
        rendered.contains("double the number of +1/+1 counters"),
        "expected trailing double-counters clause, got {rendered}"
    );
}

#[test]
fn parse_then_that_player_discards_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Recoil Variant")
        .parse_text(
            "Return target permanent to its owner's hand. Then that player discards a card.",
        )
        .expect("then-that-player discard clause should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("that player discards a card")
            || rendered.contains("target player discards a card"),
        "expected discard to remain bound to the returned permanent's player, got {rendered}"
    );
}

#[test]
fn parse_comma_then_that_player_discards_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Dinrova Variant")
        .parse_text(
            "Return target permanent to its owner's hand, then that player discards a card.",
        )
        .expect("comma-then-that-player discard clause should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("that player discards a card")
            || rendered.contains("target player discards a card"),
        "expected discard to remain bound to the returned permanent's player, got {rendered}"
    );
}

#[test]
fn parse_comma_then_return_source_to_hand_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Cyclopean Variant")
        .card_types(vec![CardType::Artifact])
        .parse_text("{3}, {T}: Tap target creature, then return this artifact to its owner's hand.")
        .expect("comma-then return-source clause should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("tap target creature"),
        "expected tap target creature effect, got {rendered}"
    );
    assert!(
        rendered.contains("return this artifact to its owner's hand"),
        "expected return-source clause, got {rendered}"
    );
}

#[test]
fn parse_then_exile_that_players_graveyard_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Go Blank Variant")
        .parse_text("Target player discards two cards. Then exile that player's graveyard.")
        .expect("then-that-player graveyard exile clause should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("that player's graveyard")
            || rendered.contains("target player's graveyard"),
        "expected graveyard exile to remain tied to the targeted player, got {rendered}"
    );
}

#[test]
fn parse_put_counter_sequence_with_and_chain() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Trygon Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "When this creature attacks, put a +1/+1 counter on it and a +1/+1 counter on up to one other target attacking creature. That creature can't be blocked this turn.",
        )
        .expect("put-counter and-chain should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("up to one other target attacking creature")
            || rendered.contains("another attacking creature"),
        "expected second counter target to remain in trigger, got {rendered}"
    );
    assert!(
        rendered.contains("can't be blocked") || rendered.contains("cant be blocked"),
        "expected trailing block restriction to remain, got {rendered}"
    );
}

#[test]
fn parse_if_you_do_search_library_clause_keeps_full_tail_effect() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Blood Speaker Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "At the beginning of your upkeep, you may sacrifice this creature. If you do, search your library for a Demon card, reveal that card, put it into your hand, then shuffle.",
        )
        .expect("if-you-do search clause should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("if you do")
            && rendered.contains("search your library for a demon card")
            && rendered.contains("put it into your hand")
            && rendered.contains("shuffle"),
        "expected if-you-do gate plus full search/reveal/put/shuffle tail, got {rendered}"
    );
    assert!(
        rendered.contains("search your library for a demon card")
            && rendered.contains("put it into your hand")
            && rendered.contains("shuffle"),
        "expected full search/reveal/put/shuffle tail to remain after if-you-do split, got {rendered}"
    );
}

#[test]
fn parse_trigger_with_comma_separated_list_does_not_split_early() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Sram Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Whenever you cast an Aura, Equipment, or Vehicle spell, draw a card.")
        .expect("comma-separated trigger list should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("aura") && rendered.contains("equipment") && rendered.contains("vehicle"),
        "expected trigger list to include aura/equipment/vehicle, got {rendered}"
    );
}

#[test]
fn parse_trigger_with_and_or_subtype_list_keeps_effect_split_on_trigger_delimiter() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Vaan Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Whenever one or more Scouts, Pirates, and/or Rogues you control deal combat damage to a player, exile the top card of that player's library. You may cast it. If you don't, create a Treasure token.")
        .expect("and/or subtype trigger list should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("that player exiles the top card of that player's library")
            && rendered.contains("you may cast it")
            && rendered.contains("create a treasure token"),
        "expected exile/create sequence to remain on the triggered effect, got {rendered}"
    );

    let abilities_debug = format!("{:#?}", def.abilities);
    assert!(
        !abilities_debug.contains("unimplemented_trigger"),
        "expected no fallback custom trigger for and/or subtype list, got {abilities_debug}"
    );
}

#[test]
fn parse_other_mice_anthem_renders_irregular_plural() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Mabel Anthem Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Other Mice you control get +1/+1.")
        .expect("mice anthem should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("other mice you control get +1/+1"),
        "expected irregular 'mice' plural in rendered anthem, got {rendered}"
    );
    assert!(
        !rendered.contains("mouses"),
        "expected not to render as 'mouses', got {rendered}"
    );
}

#[test]
fn parse_mabel_token_preserves_colorless_and_equipment_payload() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Mabel Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "When Mabel enters, create Cragflame, a legendary colorless Equipment artifact token with \"Equipped creature gets +1/+1 and has vigilance, trample, and haste\" and equip {2}.",
        )
        .expect("Mabel token payload should parse");
    let rendered = format!("{def:?}").to_ascii_lowercase();
    assert!(
        rendered.contains("name: \"cragflame\"")
            && rendered.contains("colorless")
            && rendered.contains("equipment")
            && rendered.contains("equip {2}"),
        "expected parsed Mabel token payload, got {rendered}"
    );
}

#[test]
fn parse_that_creature_gets_and_gains_uses_single_tagged_target() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Ogre Battledriver Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Whenever another creature you control enters, that creature gets +2/+0 and gains haste until end of turn.",
        )
        .expect("that-creature gets-and-gains clause should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("that creature gets +2/+0") || rendered.contains("it gets +2/+0"),
        "expected pump to stay on the single triggering creature, got {rendered}"
    );
    assert!(
        !rendered.contains("creatures get +2/+0"),
        "expected not to broaden to all creatures, got {rendered}"
    );
}

#[test]
fn parse_the_player_may_clause_preserves_that_player_decider() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Gate Variant")
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "At the beginning of each player's upkeep, that player reveals the top card of their library. If it's an artifact, creature, enchantment, or land card, the player may put it onto the battlefield.",
        )
        .expect("the-player-may conditional clause should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("that player may put it onto the battlefield"),
        "expected 'that player may' to be preserved, got {rendered}"
    );
    assert!(
        !rendered.contains("you may put it onto the battlefield"),
        "expected decision actor not to collapse to source controller, got {rendered}"
    );
}

#[test]
fn parse_return_with_multiple_counters_on_it_sentence() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Perennation Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Return target permanent card from your graveyard to the battlefield with a hexproof counter and an indestructible counter on it.",
        )
        .expect("return-with-multiple-counters clause should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("return target")
            && rendered.contains("battlefield")
            && rendered.contains("hexproof counter")
            && rendered.contains("indestructible counter"),
        "expected returned permanent to receive both counters, got {rendered}"
    );
}

#[test]
fn parse_sacrifice_any_number_sentence_keeps_open_count() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Landslide Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Sacrifice any number of Mountains. Landslide deals that much damage to target player or planeswalker.",
        )
        .expect("sacrifice-any-number clause should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("any number"),
        "expected sacrifice count to remain open-ended, got {rendered}"
    );
}

#[test]
fn parse_gain_ability_until_next_upkeep_uses_non_end_of_turn_duration() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Erhnam Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "At the beginning of your upkeep, target non-Wall creature an opponent controls gains forestwalk until your next upkeep.",
        )
        .expect("gain-until-next-upkeep clause should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("until your next"),
        "expected duration to remain next-turn scoped, got {rendered}"
    );
    assert!(
        !rendered.contains("until end of turn"),
        "expected duration not to collapse to end of turn, got {rendered}"
    );
}

#[test]
fn parse_minion_reflector_copy_clause_keeps_haste_and_end_step_sacrifice() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Minion Reflector Variant")
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "Whenever a nontoken creature you control enters, you may pay {2}. If you do, create a token that's a copy of that creature, except it has haste and \"At the beginning of the end step, sacrifice this permanent.\"",
        )
        .expect("copy-with-inline-haste-and-end-step clause should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("copy of it"),
        "expected token-copy clause to remain present, got {rendered}"
    );
    assert!(
        rendered.contains("haste"),
        "expected haste modifier to remain present, got {rendered}"
    );
    assert!(
        rendered.contains("next end step") || rendered.contains("the end step"),
        "expected delayed end-step sacrifice clause, got {rendered}"
    );
}

#[test]
fn parse_not_dead_after_all_keeps_role_creation_and_attachment_in_granted_trigger() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Not Dead After All Variant")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Until end of turn, target creature you control gains \"When this creature dies, return it to the battlefield tapped under its owner's control, then create a Wicked Role token attached to it.\"",
        )
        .expect("wicked-role-on-return clause should parse");

    let debug = format!("{:#?}", def.spell_effect);
    assert!(
        debug.contains("CreateTokenEffect"),
        "expected granted trigger to keep Wicked Role token creation, got {debug}"
    );
    assert!(
        debug.contains("AttachObjectsEffect"),
        "expected created role token to be attached, got {debug}"
    );
}

#[test]
fn parse_face_down_target_filter_for_destroy_effect() {
    let err = CardDefinitionBuilder::new(CardId::from_raw(1), "Nosy Goblin Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("{T}, Sacrifice this creature: Destroy target face-down creature.")
        .expect_err("face-down target destroy is currently unsupported");

    let rendered = format!("{err:?}").to_ascii_lowercase();
    assert!(
        rendered.contains("unsupported face-down clause"),
        "expected explicit unsupported face-down clause error, got {rendered}"
    );
}

#[test]
fn parse_face_down_static_anthem_filter() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Secret Plans Variant")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "Face-down creatures you control get +0/+1.\nWhenever a permanent you control is turned face up, draw a card.",
        )
        .expect("face-down anthem line should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("face-down creatures you control get +0/+1"),
        "expected face-down qualifier preserved on anthem, got {rendered}"
    );
}

#[test]
fn parse_player_sacrifices_trigger_preserves_another_qualifier() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Furnace Celebration Variant")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "Whenever you sacrifice another permanent, you may pay {2}. If you do, this enchantment deals 2 damage to any target.",
        )
        .expect("player-sacrifices trigger with another qualifier should parse");

    let abilities_debug = format!("{:#?}", def.abilities);
    assert!(
        abilities_debug.contains("other: true"),
        "expected sacrifice trigger filter to keep 'another' qualifier, got {abilities_debug}"
    );

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("sacrifice another permanent"),
        "expected rendered trigger to preserve 'another permanent', got {rendered}"
    );
}

#[test]
fn parse_rhystic_lightning_unless_payment_then_reduced_damage() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Rhystic Lightning Variant")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "This spell deals 4 damage to any target unless that permanent's controller or that player pays {2}. If they do, this spell deals 2 damage to the permanent or player.",
        )
        .expect("rhystic unless-payment clause should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("unless") && rendered.contains("pays {2}"),
        "expected unless-payment branch to remain explicit, got {rendered}"
    );
    assert!(
        (rendered.contains("if they do")
            || rendered.contains("if that doesn't happen")
            || rendered.contains("if that doesnt happen"))
            && rendered.contains("deal 2 damage"),
        "expected reduced-damage paid branch to remain explicit, got {rendered}"
    );
}

#[test]
fn parse_slivercycling_grant_clause_as_static_grant_not_keyword_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Homing Grant Variant")
        .parse_text("Each Sliver card in each player's hand has slivercycling {3}.")
        .expect("slivercycling grant clause should parse as a static grant ability");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("each sliver") && rendered.contains("has slivercycling 3"),
        "expected rendered static grant clause, got {rendered}"
    );
    assert!(
        !rendered.starts_with("keyword ability 1: slivercycling {3}."),
        "expected no standalone keyword-only parse for grant clause, got {rendered}"
    );
}

#[test]
fn parse_gideon_planeswalker_predicate_keeps_subtype_constraint() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Gideon Predicate Variant")
        .card_types(vec![CardType::Instant])
        .parse_text("Exile target white creature that's attacking or blocking. If it was a Gideon planeswalker, you gain 5 life.")
        .expect("gideon-planeswalker predicate should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("gideon"),
        "expected subtype constraint to remain in rendered predicate, got {rendered}"
    );
    assert!(
        rendered.contains("planeswalker"),
        "expected planeswalker card type to remain in rendered predicate, got {rendered}"
    );
}

#[test]
fn parse_permanent_card_target_in_graveyard_sets_permanent_card_types() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Jailbreak Permanent Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Return target permanent card in an opponent's graveyard to the battlefield under their control.",
        )
        .expect("permanent-card graveyard target should parse");

    let spell_debug = format!("{:#?}", def.spell_effect);
    assert!(
        spell_debug.contains("Artifact")
            && spell_debug.contains("Creature")
            && spell_debug.contains("Enchantment")
            && spell_debug.contains("Land")
            && spell_debug.contains("Planeswalker")
            && spell_debug.contains("Battle"),
        "expected permanent card-type expansion for graveyard target, got {spell_debug}"
    );
    assert!(
        !spell_debug.contains("Instant") && !spell_debug.contains("Sorcery"),
        "expected nonpermanent types to stay excluded, got {spell_debug}"
    );
}

#[test]
fn parse_one_or_more_subject_with_attack_verb_is_not_custom_trigger() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "One Or More Attack Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Whenever one or more Phyrexians you control attack, draw a card.")
        .expect("one-or-more attack trigger should parse as attacks trigger");

    let abilities_debug = format!("{:#?}", def.abilities);
    assert!(
        !abilities_debug.contains("unimplemented_trigger"),
        "expected no fallback custom trigger for singular 'attack' wording, got {abilities_debug}"
    );
}

#[test]
fn parse_one_or_more_attack_trigger_preserves_one_or_more_compiled_text() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "One Or More Attack Render Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Whenever one or more Phyrexians you control attack, draw a card.")
        .expect("one-or-more attack trigger should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("one or more phyrexian")
            && rendered.contains("you control")
            && rendered.contains("attack"),
        "expected one-or-more attack wording to remain explicit, got {rendered}"
    );
}

#[test]
fn parse_mount_or_vehicle_attack_trigger_keeps_both_subjects() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Mount Vehicle Attack Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Whenever a Mount or Vehicle you control attacks, draw a card.")
        .expect("mount-or-vehicle attack trigger should parse");

    let abilities_debug = format!("{:#?}", def.abilities);
    assert!(
        !abilities_debug.contains("unimplemented_trigger"),
        "expected no fallback custom trigger for mount-or-vehicle attack clause, got {abilities_debug}"
    );
    assert!(
        abilities_debug.contains("Mount") && abilities_debug.contains("Vehicle"),
        "expected both subtypes in attack trigger filter, got {abilities_debug}"
    );
}

#[test]
fn parse_one_or_more_enters_trigger_uses_batch_count_mode() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "One Or More Enter Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Whenever one or more tokens you control enter, put a +1/+1 counter on this creature.",
        )
        .expect("one-or-more enters trigger should parse");

    let abilities_debug = format!("{:#?}", def.abilities);
    assert!(
        abilities_debug.contains("count_mode: OneOrMore"),
        "expected ETB trigger to compile in one-or-more mode, got {abilities_debug}"
    );
}

#[test]
fn parse_due_respect_variant_renders_permanents_enter_tapped_compactly() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Due Respect Variant")
        .card_types(vec![CardType::Instant])
        .parse_text("Permanents enter tapped this turn.\nDraw a card.")
        .expect("due-respect style line should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("permanents enter tapped"),
        "expected compact permanent enter-tapped wording, got {rendered}"
    );
    assert!(
        !rendered.contains("artifacts, creatures, enchantments, lands, planeswalkers, and battles"),
        "expected no expanded permanent type list in enter-tapped wording, got {rendered}"
    );
}

#[test]
fn parse_creatures_entering_dont_trigger_static_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Torpor Orb Variant")
        .card_types(vec![CardType::Artifact])
        .parse_text("Creatures entering don't cause abilities to trigger.")
        .expect("torpor-orb static line should parse");

    let static_ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();

    assert!(
        static_ids.contains(
            &crate::static_abilities::StaticAbilityId::CreaturesEnteringDontCauseAbilitiesToTrigger
        ),
        "expected ETB trigger suppression static ability, got {static_ids:?}"
    );
}

#[test]
fn parse_as_long_as_its_enchanted_condition_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Fledgling Osprey Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("This creature has flying as long as it's enchanted.")
        .expect("as-long-as-its-enchanted static line should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("flying"),
        "expected flying in rendered static text, got {rendered}"
    );
    assert!(
        rendered.contains("as long as this creature is enchanted"),
        "expected enchanted condition in rendered static text, got {rendered}"
    );
}

#[test]
fn parse_as_long_as_enchanted_permanent_is_a_creature_condition_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Rune of Flight Variant")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Aura])
        .parse_text(
            "Enchant permanent\nAs long as enchanted permanent is a creature, it has flying.",
        )
        .expect("enchanted-permanent creature condition line should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("has flying"),
        "expected attached keyword grant in rendered static text, got {rendered}"
    );
    assert!(
        rendered.contains("as long as enchanted permanent is a creature"),
        "expected enchanted-permanent creature condition in rendered static text, got {rendered}"
    );
}

#[test]
fn parse_each_creature_assigns_combat_damage_with_toughness_static_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Doran Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Each creature assigns combat damage equal to its toughness rather than its power.",
        )
        .expect("global toughness-combat-damage static line should parse");

    let static_ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();

    assert!(
        static_ids.contains(
            &crate::static_abilities::StaticAbilityId::CreaturesAssignCombatDamageUsingToughness
        ),
        "expected global toughness combat-damage static ability, got {static_ids:?}"
    );
}

#[test]
fn parse_each_creature_you_control_assigns_combat_damage_with_toughness_static_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Brontodon Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Each creature you control assigns combat damage equal to its toughness rather than its power.",
        )
        .expect("you-control toughness-combat-damage static line should parse");

    let static_ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();

    assert!(
        static_ids.contains(
            &crate::static_abilities::StaticAbilityId::CreaturesYouControlAssignCombatDamageUsingToughness
        ),
        "expected controller-scoped toughness combat-damage static ability, got {static_ids:?}"
    );
}

#[test]
fn parse_return_up_to_one_subtype_list_target_stays_single_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Thwart Return Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Return target creature card and up to one target Cleric, Rogue, Warrior, or Wizard creature card from your graveyard to the battlefield.",
        )
        .expect("subtype-list return clause should parse without splitting into multiple returns");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("up to one target cleric or rogue or warrior or wizard creature card"),
        "expected subtype list to remain on a single return target clause, got {rendered}"
    );
    assert!(
        !rendered.contains("return card rogue from your graveyard")
            && !rendered.contains("return card warrior from your graveyard"),
        "expected no synthetic per-subtype return clauses, got {rendered}"
    );
}

#[test]
fn parse_draw_second_card_each_turn_trigger_is_not_custom() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Second Draw Trigger Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Whenever you draw your second card each turn, target Detective can't be blocked this turn.",
        )
        .expect("second-card draw trigger should parse");

    let abilities_debug = format!("{:#?}", def.abilities);
    assert!(
        !abilities_debug.contains("unimplemented_trigger"),
        "expected no fallback custom trigger for second-card draw trigger, got {abilities_debug}"
    );
    assert!(
        abilities_debug.contains("PlayerDrawsNthCardEachTurnTrigger")
            || abilities_debug.contains("draws their second card each turn"),
        "expected nth-card draw trigger matcher, got {abilities_debug}"
    );
}

#[test]
fn parse_draw_third_card_each_turn_trigger_supports_higher_ordinals() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Third Draw Trigger Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Whenever you draw your third card each turn, draw a card.")
        .expect("third-card draw trigger should parse");

    let abilities_debug = format!("{:#?}", def.abilities);
    assert!(
        abilities_debug.contains("card_number: 3"),
        "expected third-card ordinal to compile as card_number=3, got {abilities_debug}"
    );
}

#[test]
fn parse_orcish_bowmasters_draw_exception_clause_compiles_noncustom_draw_trigger() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Orcish Bowmasters Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Flash\nWhen this creature enters and whenever an opponent draws a card except the first one they draw in each of their draw steps, this creature deals 1 damage to any target. Then amass Orcs 1.",
        )
        .expect("orcish bowmasters-style trigger should parse");

    let abilities_debug = format!("{:#?}", def.abilities);
    assert!(
        !abilities_debug.contains("unimplemented_trigger"),
        "expected no fallback custom trigger, got {abilities_debug}"
    );
    assert!(
        abilities_debug.contains("OrTrigger"),
        "expected ETB-or-draw trigger composition, got {abilities_debug}"
    );
    assert!(
        abilities_debug.contains("PlayerDrawsNthCardEachTurnTrigger")
            || abilities_debug.contains("draws their second card each turn"),
        "expected draw-exception clause to compile as typed draw trigger, got {abilities_debug}"
    );
    assert!(
        abilities_debug.contains("AmassEffect"),
        "expected triggered effect list to include AmassEffect, got {abilities_debug}"
    );
}

#[test]
fn parse_exile_top_card_of_target_library_preserves_top_card_selection() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Top Card Exile Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("{T}: Exile the top card of target player's library.")
        .expect("top-card exile should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("exile the top card of target player's library")
            || rendered.contains("target player exiles the top card of target player's library"),
        "expected top-card wording to remain explicit, got {rendered}"
    );
}

#[test]
fn parse_lose_all_abilities_except_mana_static_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Blood Sun Variant")
        .card_types(vec![CardType::Enchantment])
        .parse_text("All lands lose all abilities except mana abilities.")
        .expect("lose-all-except-mana clause should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("lose all abilities except mana abilities"),
        "expected explicit except-mana wording, got {rendered}"
    );
}

#[test]
fn parse_put_counters_equal_to_that_creatures_power() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "First Responder Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "At the beginning of your end step, you may return another creature you control to its owner's hand, then put a number of +1/+1 counters equal to that creature's power on this creature.",
        )
        .expect("dynamic +1/+1 counter count should parse");

    let abilities_debug = format!("{:#?}", def.abilities);
    assert!(
        abilities_debug.contains("PowerOf") || abilities_debug.contains("that creature's power"),
        "expected dynamic power-based counter amount, got {abilities_debug}"
    );
}

#[test]
fn parse_lose_life_equal_to_power_plus_toughness() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Phthisis Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Destroy target creature. Its controller loses life equal to its power plus its toughness.")
        .expect("power-plus-toughness life amount should parse");

    let abilities_debug = format!("{:#?}", def.spell_effect);
    assert!(
        abilities_debug.contains("Add")
            && abilities_debug.contains("PowerOf")
            && abilities_debug.contains("ToughnessOf"),
        "expected additive power+toughness life amount, got {abilities_debug}"
    );
}

#[test]
fn parse_creature_tapped_to_pay_additional_cost_targets_tap_cost_tag() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Swallow Whole Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "As an additional cost to cast this spell, tap an untapped creature you control.\nExile target tapped creature. Put a +1/+1 counter on the creature tapped to pay this spell's additional cost.",
        )
        .expect("cost-linked tapped creature reference should parse");

    let spell_debug = format!("{:#?}", def.spell_effect);
    assert!(
        spell_debug.contains("tap_cost_0"),
        "expected follow-up counter target to reference tap_cost_0, got {spell_debug}"
    );
}

#[test]
fn parse_enchanted_base_pt_and_indestructible_without_nested_grant_text() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Almost Perfect Variant")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "Enchant creature\nEnchanted creature has base power and toughness 9/10 and has indestructible.",
        )
        .expect("base P/T + keyword aura clause should parse");

    let static_ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        static_ids.contains(&StaticAbilityId::SetBasePowerToughnessForFilter),
        "expected set-base-power/toughness static ability, got {static_ids:?}"
    );
    assert_eq!(
        static_ids
            .iter()
            .filter(|id| **id == StaticAbilityId::GrantAbility)
            .count(),
        1,
        "expected exactly one keyword grant static ability, got {static_ids:?}"
    );

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("base power and toughness 9/10"),
        "expected base P/T wording in compiled output, got {rendered}"
    );
    assert!(
        rendered.contains("indestructible"),
        "expected granted indestructible wording in compiled output, got {rendered}"
    );
    assert!(
        !rendered.contains("has permanents with base power and toughness"),
        "expected no nested grant phrasing in compiled output, got {rendered}"
    );
}

#[test]
fn parse_target_creature_becomes_colorless_until_end_of_turn() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Ancient Kavu Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("{1}: Target creature becomes colorless until end of turn.")
        .expect("becomes-colorless clause should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("target creature becomes colorless until end of turn"),
        "expected make-colorless wording in compiled output, got {rendered}"
    );
}

#[test]
fn parse_target_creature_becomes_single_color_until_end_of_turn() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Swirling Spriggan Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("{1}: Target creature becomes red until end of turn.")
        .expect("becomes-color clause should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("target creature becomes red until end of turn"),
        "expected set-colors wording in compiled output, got {rendered}"
    );
}

#[test]
fn parse_target_creature_becomes_color_of_your_choice_until_end_of_turn() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Color Choice Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("{1}: Target creature becomes the color of your choice until end of turn.")
        .expect("becomes-color-of-choice clause should parse");

    let abilities_debug = format!("{:#?}", def.abilities).to_ascii_lowercase();
    assert!(
        abilities_debug.contains("becomecolorchoiceeffect"),
        "expected become-color-choice effect in activated ability, got {abilities_debug}"
    );
}

#[test]
fn parse_target_creature_becomes_color_or_colors_of_your_choice_until_end_of_turn() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Swirling Spriggan Choice Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("{G/U}: Target creature you control becomes the color or colors of your choice until end of turn.")
        .expect("becomes-color-or-colors-of-choice clause should parse");

    let abilities_debug = format!("{:#?}", def.abilities).to_ascii_lowercase();
    assert!(
        abilities_debug.contains("becomecolorchoiceeffect"),
        "expected become-color-choice effect in activated ability, got {abilities_debug}"
    );
}

#[test]
fn parse_this_creature_becomes_creature_type_of_your_choice_until_end_of_turn() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Mistform Dreamer Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "{1}: This creature becomes the creature type of your choice until end of turn.",
        )
        .expect("becomes-creature-type-of-choice clause should parse");

    let abilities_debug = format!("{:#?}", def.abilities).to_ascii_lowercase();
    assert!(
        abilities_debug.contains("becomecreaturetypechoiceeffect"),
        "expected become-creature-type-choice effect in activated ability, got {abilities_debug}"
    );
}

#[test]
fn parse_choose_creature_type_then_target_becomes_that_type() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Imagecrafter Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("{T}: Choose a creature type other than Wall. Target creature becomes that type until end of turn.")
        .expect("choose-creature-type then target becomes that type should parse");

    let abilities_debug = format!("{:#?}", def.abilities).to_ascii_lowercase();
    assert!(
        abilities_debug.contains("becomecreaturetypechoiceeffect"),
        "expected become-creature-type-choice effect in activated ability, got {abilities_debug}"
    );
}

#[test]
fn parse_choose_creature_type_then_each_creature_becomes_that_type() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Standardize Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Choose a creature type other than Wall. Each creature becomes that type until end of turn.")
        .expect("choose-creature-type then each creature becomes that type should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("becomecreaturetypechoiceeffect")
            || rendered.contains("creature type of your choice"),
        "expected become-creature-type-choice effect in sorcery text, got {rendered}"
    );
}

#[test]
fn parse_this_creature_cant_be_blocked_by_creatures_with_flying() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Gnat Alley Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("This creature can't be blocked by creatures with flying.")
        .expect("cant-be-blocked-by-flying clause should parse");

    let abilities_debug = format!("{:#?}", def.abilities).to_ascii_lowercase();
    assert!(
        abilities_debug.contains("blockspecificattacker"),
        "expected blocker restriction against fliers, got {abilities_debug}"
    );
}

#[test]
fn parse_this_creature_cant_be_blocked_except_by_black_creatures() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Dread Warlock Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("This creature can't be blocked except by black creatures.")
        .expect("cant-be-blocked-except-by-black clause should parse");

    let abilities_debug = format!("{:#?}", def.abilities).to_ascii_lowercase();
    assert!(
        abilities_debug.contains("blockspecificattacker"),
        "expected blocker restriction to nonblack blockers, got {abilities_debug}"
    );
}

#[test]
fn parse_this_creature_cant_be_blocked_by_walls() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Bog Rats Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("This creature can't be blocked by Walls.")
        .expect("cant-be-blocked-by-walls clause should parse");

    let abilities_debug = format!("{:#?}", def.abilities).to_ascii_lowercase();
    assert!(
        abilities_debug.contains("blockspecificattacker"),
        "expected blocker restriction against Walls, got {abilities_debug}"
    );
}

#[test]
fn parse_this_creature_cant_block_creatures_with_power_two_or_greater() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Brassclaw Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("This creature can't block creatures with power 2 or greater.")
        .expect("cant-block-power-threshold clause should parse");

    let abilities_debug = format!("{:#?}", def.abilities).to_ascii_lowercase();
    assert!(
        abilities_debug.contains("blockspecificattacker"),
        "expected blocker restriction by attacker power, got {abilities_debug}"
    );
}

#[test]
fn parse_creatures_without_flying_cant_block_this_turn() {
    let _def = CardDefinitionBuilder::new(CardId::from_raw(1), "Falter Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Creatures without flying can't block this turn.")
        .expect("global cant-block-this-turn clause should parse");
}

#[test]
fn parse_target_creature_cant_block_this_turn() {
    let _def = CardDefinitionBuilder::new(CardId::from_raw(1), "Blindblast Variant")
        .card_types(vec![CardType::Instant])
        .parse_text("Target creature can't block this turn.")
        .expect("target cant-block-this-turn clause should parse");
}

#[test]
fn parse_your_maximum_hand_size_reduced_static_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Thought Devourer Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Flying\nYour maximum hand size is reduced by four.")
        .expect("your maximum hand size reduction line should parse");

    let rendered = format!("{:#?}", def.abilities).to_ascii_lowercase();
    assert!(
        rendered.contains("maximum hand size is reduced by"),
        "expected maximum-hand-size reduction in rendered text, got {rendered}"
    );
}

#[test]
fn parse_each_opponents_maximum_hand_size_reduced_static_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Ivory Tower Variant")
        .card_types(vec![CardType::Artifact])
        .parse_text("Each opponent's maximum hand size is reduced by one.")
        .expect("each-opponent maximum hand size reduction line should parse");

    let rendered = format!("{:#?}", def.abilities).to_ascii_lowercase();
    assert!(
        rendered.contains("maximum hand size is reduced by"),
        "expected maximum-hand-size reduction in rendered text, got {rendered}"
    );
}

#[test]
fn parse_exile_top_x_until_end_of_your_next_turn_may_play_those_cards() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Commune with Lava Variant")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Exile the top X cards of your library. Until the end of your next turn, you may play those cards.",
        )
        .expect("exile-top then until-next-turn play-those-cards should parse");

    let spell_debug = format!("{:#?}", def.spell_effect).to_ascii_lowercase();
    assert!(
        spell_debug.contains("grantplaytaggedeffect"),
        "expected tagged play grant effect in spell text, got {spell_debug}"
    );
}

#[test]
fn parse_wrenns_resolve_exiles_top_two_cards() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Wrenn's Resolve")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Exile the top two cards of your library. Until the end of your next turn, you may play those cards.",
        )
        .expect("wrenn's resolve style clause should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("top two cards") || rendered.contains("top 2 cards"),
        "expected top-two exile rendering, got {rendered}"
    );
}

#[test]
fn parse_exile_top_card_you_may_play_that_card_this_turn() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Impulse Draw Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Exile the top card of your library. You may play that card this turn.")
        .expect("exile-top then play-that-card-this-turn should parse");

    let spell_debug = format!("{:#?}", def.spell_effect).to_ascii_lowercase();
    assert!(
        spell_debug.contains("grantplaytaggedeffect"),
        "expected end-of-turn tagged play grant, got {spell_debug}"
    );
    assert!(
        spell_debug.contains("untilendofturn"),
        "expected end-of-turn duration on tagged play grant, got {spell_debug}"
    );
}

#[test]
fn parse_target_player_may_cast_tagged_card_without_paying_mana_cost() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Cast Tagged Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Exile the top card of target player's library. That player may cast that card without paying its mana cost.",
        )
        .expect("target-player may-cast-tagged clause should parse");

    let spell_debug = format!("{:#?}", def.spell_effect).to_ascii_lowercase();
    assert!(
        spell_debug.contains("casttaggedeffect"),
        "expected cast-tagged effect in spell text, got {spell_debug}"
    );
}

#[test]
fn parse_put_the_rest_on_bottom_with_previous_put_into_hand() {
    let _def = CardDefinitionBuilder::new(CardId::from_raw(1), "Put Rest Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Look at the top three cards of your library. You may reveal a creature card from among them and put it into your hand. Put the rest on the bottom of your library in any order.",
        )
        .expect("put-the-rest-on-bottom follow-up should parse as part of put clause");
}

#[test]
fn parse_when_this_creature_becomes_blocked_may_untap_and_remove_from_combat() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Gustcloak Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Flying\nWhenever this creature becomes blocked, you may untap it and remove it from combat.")
        .expect("becomes-blocked untap-and-remove-from-combat trigger should parse");

    let abilities_debug = format!("{:#?}", def.abilities).to_ascii_lowercase();
    assert!(
        abilities_debug.contains("removefromcombateffect"),
        "expected remove-from-combat effect in triggered ability, got {abilities_debug}"
    );
}

#[test]
fn parse_you_gain_protection_from_everything_until_your_next_turn() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "The One Ring")
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "When The One Ring enters, if you cast it, you gain protection from everything until your next turn.",
        )
        .expect("player protection-from-everything trigger should parse");

    let abilities_debug = format!("{:#?}", def.abilities).to_ascii_lowercase();
    assert!(
        abilities_debug.contains("betargetedplayer"),
        "expected temporary cant-target-player restriction, got {abilities_debug}"
    );
    assert!(
        abilities_debug.contains("preventalldamagetotargeteffect"),
        "expected temporary prevent-all-damage-to-player effect, got {abilities_debug}"
    );
    assert!(
        abilities_debug.contains("sourcewascast"),
        "expected intervening-if 'you cast it' condition, got {abilities_debug}"
    );
}

#[test]
fn parse_lose_half_your_life_rounded_up_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Cruel Bargain")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Draw four cards. You lose half your life, rounded up.")
        .expect("half-life loss clause should parse");

    let spell_debug = format!("{:#?}", def.spell_effect).to_ascii_lowercase();
    assert!(
        spell_debug.contains("halflifetotalroundedup"),
        "expected half-life rounded-up value in lose-life effect, got {spell_debug}"
    );
}

#[test]
fn parse_oracle_liquimetal_coating_type_addition_render_regression() {
    let def = parse_oracle_card_definition("Liquimetal Coating");

    let raw = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        raw.contains("addcardtypes") && raw.contains("artifact"),
        "expected raw compiled definition to keep artifact type addition, got {raw}"
    );

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains(
            "target permanent becomes an artifact in addition to its other types until end of turn"
        ),
        "expected Liquimetal Coating type-addition wording, got {rendered}"
    );
    assert!(
        !rendered.contains("unsupported effect"),
        "expected Liquimetal Coating to avoid unsupported markers, got {rendered}"
    );
}

#[test]
fn parse_oracle_dispossess_typed_card_name_regression() {
    let def = parse_oracle_card_definition("Dispossess");

    let raw = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        raw.contains("choosecardnameeffect")
            && raw.contains("filter: some")
            && raw.contains("artifact"),
        "expected raw compiled definition to retain typed card-name choice, got {raw}"
    );

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("choose an artifact card name"),
        "expected Dispossess to preserve the typed name-choice clause, got {rendered}"
    );
    assert!(
        rendered.contains("search target opponent's graveyard, hand, and library")
            && (rendered.contains("with that name")
                || rendered.contains("with the same name as that object"))
            && rendered.contains("exile"),
        "expected Dispossess search-and-exile wording, got {rendered}"
    );
    assert!(
        !rendered.contains("artifact you control in the battlefield")
            && !rendered.contains("exile you"),
        "expected Dispossess to avoid battlefield-target fallback text, got {rendered}"
    );
}

#[test]
fn parse_oracle_infinite_obliteration_typed_card_name_regression() {
    let def = parse_oracle_card_definition("Infinite Obliteration");

    let raw = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        raw.contains("choosecardnameeffect")
            && raw.contains("filter: some")
            && raw.contains("creature"),
        "expected raw compiled definition to retain typed card-name choice, got {raw}"
    );

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("choose a creature card name"),
        "expected Infinite Obliteration to preserve the typed name-choice clause, got {rendered}"
    );
    assert!(
        rendered.contains("search target opponent's graveyard, hand, and library")
            && (rendered.contains("with that name")
                || rendered.contains("with the same name as that object"))
            && rendered.contains("exile"),
        "expected Infinite Obliteration search-and-exile wording, got {rendered}"
    );
    assert!(
        !rendered.contains("creature you control in the battlefield")
            && !rendered.contains("exile you"),
        "expected Infinite Obliteration to avoid battlefield-target fallback text, got {rendered}"
    );
}

#[test]
fn parse_oracle_human_frailty_targeted_subtype_regression() {
    let def = parse_oracle_card_definition("Human Frailty");

    let raw = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        raw.contains("destroyeffect") && raw.contains("human") && raw.contains("creature"),
        "expected raw compiled definition to keep Human-target destroy effect, got {raw}"
    );

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("destroy target")
            && rendered.contains("human")
            && rendered.contains("creature"),
        "expected Human Frailty to preserve Human-target destroy wording, got {rendered}"
    );
    assert!(
        !rendered.contains("destroy this spell"),
        "expected Human Frailty to avoid self-destroy fallback text, got {rendered}"
    );
}

#[test]
fn parse_oracle_barkweave_crusher_enlist_render_regression() {
    let def = parse_oracle_card_definition("Barkweave Crusher");

    let raw = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        raw.contains("enlisted_creature")
            && raw.contains("enlist_attacker")
            && raw.contains("powerof"),
        "expected raw compiled definition to retain enlist power-lifting scaffolding, got {raw}"
    );

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("another nonattacking creature you control"),
        "expected Barkweave Crusher to keep the enlist tap clause, got {rendered}"
    );
    assert!(
        rendered.contains("tap target tagged object 'enlisted_creature'")
            && rendered.contains("+1/+0 until end of turn")
            && rendered.contains("enlisted_creature''s power"),
        "expected Barkweave Crusher to preserve enlist power-lifting semantics, got {rendered}"
    );
}

#[test]
fn parse_oracle_war_report_summed_battlefield_count_regression() {
    let def = parse_oracle_card_definition("War Report");

    let raw = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        raw.contains("add(") && raw.contains("creature") && raw.contains("artifact"),
        "expected raw compiled definition to keep the summed creature-plus-artifact value, got {raw}"
    );

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("gain life equal to")
            && rendered.contains("number of creatures")
            && rendered.contains("number of artifacts")
            && rendered.contains("plus"),
        "expected War Report to preserve the summed battlefield-count wording, got {rendered}"
    );
    assert!(
        !rendered.contains("creature artifact"),
        "expected War Report to avoid collapsed creature-artifact count wording, got {rendered}"
    );
}

#[test]
fn parse_oracle_descent_into_avernus_scaling_trigger_regression() {
    let def = parse_oracle_card_definition("Descent into Avernus");

    let raw = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        raw.contains("named(")
            && raw.contains("descent")
            && raw.contains("countersonsource")
            && raw.contains("treasure"),
        "expected raw compiled definition to keep descent counters and treasure scaling, got {raw}"
    );

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("put two descent counters"),
        "expected Descent into Avernus to keep named counter wording, got {rendered}"
    );
    assert!(
        rendered.contains("treasure")
            && rendered.contains("descent counter")
            && rendered.contains("each player"),
        "expected Descent into Avernus to keep treasure scaling text, got {rendered}"
    );
    assert!(
        rendered.contains("damage")
            && rendered.contains("descent counter")
            && rendered.contains("that player"),
        "expected Descent into Avernus to keep damage scaling text, got {rendered}"
    );
    assert!(
        !rendered.contains("put two this counters")
            && !rendered.contains("for each enchantment under that player's control"),
        "expected Descent into Avernus to avoid the old fallback wording, got {rendered}"
    );
}

#[test]
fn parse_oracle_ugins_insight_where_x_tail_regression() {
    let def = parse_oracle_card_definition("Ugin's Insight");

    let raw = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        raw.contains("greatestmanavalue") && raw.contains("permanent") && raw.contains("draw"),
        "expected raw compiled definition to keep greatest-mana-value scry and draw, got {raw}"
    );

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains(
            "scry the greatest mana value among permanents you control, then draw 3 cards"
        ) || rendered.contains(
            "scry the greatest mana value among permanents you control, then draw three cards"
        ),
        "expected Ugin's Insight to preserve both scry and draw clauses, got {rendered}"
    );
    assert!(
        !rendered.eq("spell effects: scry the greatest mana value among permanents you control."),
        "expected Ugin's Insight to avoid dropping the draw clause, got {rendered}"
    );
}

#[test]
fn parse_oracle_soul_partition_exile_and_recast_regression() {
    let def = parse_oracle_card_definition("Soul Partition");

    let raw = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        raw.contains("costincrease") && raw.contains("nonland"),
        "expected raw compiled definition to keep the recast tax semantics, got {raw}"
    );

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("nonland spells your opponents cast cost {2} more to cast"),
        "expected Soul Partition to preserve the opponent recast tax, got {rendered}"
    );
    assert!(
        !rendered.contains("you may effect(grantplaytaggedeffect"),
        "expected Soul Partition to avoid raw grant-play fallback text, got {rendered}"
    );
}

#[test]
fn parse_oracle_curious_herd_targeted_artifact_count_regression() {
    let def = parse_oracle_card_definition("Curious Herd");

    let raw = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        raw.contains("targetonlyeffect")
            && raw.contains("createtokeneffect")
            && raw.contains("controller: some")
            && raw.contains("opponent")
            && raw.contains("artifact"),
        "expected raw compiled definition to target an opponent and count their artifacts, got {raw}"
    );

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains(
            "create a 3/3 green beast creature token for each artifact target opponent controls"
        ),
        "expected Curious Herd to preserve the targeted artifact-count token creation, got {rendered}"
    );
    assert!(
        !rendered.contains("number of tokens you control")
            && !rendered.contains("create x 3/3 green beast creature tokens"),
        "expected Curious Herd to avoid stale x-token fallback wording, got {rendered}"
    );
}

#[test]
fn parse_oracle_drag_to_the_bottom_domain_value_regression() {
    let def = parse_oracle_card_definition("Drag to the Bottom");

    let raw = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        raw.contains("modifypowertoughness")
            && raw.contains("scaled")
            && raw.contains("basiclandtypesamong"),
        "expected raw compiled definition to keep signed domain scaling, got {raw}"
    );

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("all creatures get -x/-x until end of turn")
            && rendered.contains(
                "where x is 1 plus the number of basic land types among lands you control"
            ),
        "expected Drag to the Bottom to preserve domain-based -X/-X wording, got {rendered}"
    );
    assert!(
        !rendered.contains("+x/+x")
            && !rendered.contains("where x is -x")
            && !rendered.contains("basic lands you control"),
        "expected Drag to the Bottom to avoid the old signed-x fallback wording, got {rendered}"
    );
}

#[test]
fn parse_oracle_lucid_dreams_card_types_in_graveyard_regression() {
    let def = parse_oracle_card_definition("Lucid Dreams");

    let raw = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        raw.contains("drawcardseffect") && raw.contains("cardtypesingraveyard"),
        "expected raw compiled definition to count card types in graveyard, got {raw}"
    );

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("draw")
            && (rendered.contains("number of distinct card types in your graveyard")
                || rendered.contains("number of card types among cards in your graveyard")),
        "expected Lucid Dreams to render a card-types-in-graveyard draw count, got {rendered}"
    );
    assert!(
        !rendered.contains("for each card in your graveyard"),
        "expected Lucid Dreams to avoid plain graveyard card-count fallback wording, got {rendered}"
    );
}

#[test]
fn parse_oracle_over_the_top_dynamic_reveal_and_distribution_regression() {
    let def = parse_oracle_card_definition("Over the Top");

    let raw = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        raw.contains("forplayerseffect")
            && raw.contains("lookattopcardseffect")
            && raw.contains("nonland")
            && raw.contains("movetozoneeffect"),
        "expected raw compiled definition to reveal top cards per player and distribute them, got {raw}"
    );

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("for each player")
            && rendered.contains("number of nonland permanents they control")
            && rendered.contains("put all permanent cards revealed this way onto the battlefield")
            && rendered.contains("rest into their graveyard"),
        "expected Over the Top to preserve dynamic reveal and battlefield/graveyard distribution, got {rendered}"
    );
    assert!(
        !rendered.contains("reveals the top card of their library")
            && !rendered.contains("return all permanent revealed this way"),
        "expected Over the Top to avoid the single-card reveal fallback wording, got {rendered}"
    );
}

#[test]
fn over_the_top_moves_nonpermanents_to_their_owners_graveyards_at_runtime() {
    use crate::card::CardBuilder;
    use crate::executor::{ExecutionContext, execute_effect};
    use crate::zone::Zone;

    let def = parse_oracle_card_definition("Over the Top");
    let effects = def.spell_effect.as_ref().expect("spell effects");

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let bear = CardBuilder::new(CardId::from_raw(30_001), "Bear")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let rock = CardBuilder::new(CardId::from_raw(30_002), "Rock")
        .card_types(vec![CardType::Artifact])
        .build();
    game.create_object_from_card(&bear, alice, Zone::Battlefield);
    game.create_object_from_card(&rock, alice, Zone::Battlefield);

    let goblin = CardBuilder::new(CardId::from_raw(30_003), "Goblin")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    game.create_object_from_card(&goblin, bob, Zone::Battlefield);

    let alice_spell = CardBuilder::new(CardId::from_raw(30_004), "Alice Spell")
        .card_types(vec![CardType::Instant])
        .build();
    let alice_perm = CardBuilder::new(CardId::from_raw(30_005), "Alice Permanent")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 3))
        .build();
    game.create_object_from_card(&alice_spell, alice, Zone::Library);
    game.create_object_from_card(&alice_perm, alice, Zone::Library);

    let bob_spell = CardBuilder::new(CardId::from_raw(30_006), "Bob Spell")
        .card_types(vec![CardType::Sorcery])
        .build();
    game.create_object_from_card(&bob_spell, bob, Zone::Library);

    let source = game.new_object_id();
    let mut ctx = ExecutionContext::new_default(source, alice);
    for effect in effects {
        execute_effect(&mut game, effect, &mut ctx).expect("execute Over the Top effect");
    }

    let alice_graveyard_names: Vec<_> = game
        .player(alice)
        .expect("alice")
        .graveyard
        .iter()
        .filter_map(|&id| game.object(id).map(|obj| obj.name.clone()))
        .collect();
    assert!(
        alice_graveyard_names
            .iter()
            .any(|name| name == "Alice Spell"),
        "expected Alice Spell in Alice's graveyard after Over the Top, got {alice_graveyard_names:?}"
    );

    let bob_graveyard_names: Vec<_> = game
        .player(bob)
        .expect("bob")
        .graveyard
        .iter()
        .filter_map(|&id| game.object(id).map(|obj| obj.name.clone()))
        .collect();
    assert!(
        bob_graveyard_names.iter().any(|name| name == "Bob Spell"),
        "expected Bob Spell in Bob's graveyard after Over the Top, got {bob_graveyard_names:?}"
    );

    let battlefield_names: Vec<_> = game
        .battlefield
        .iter()
        .filter_map(|&id| game.object(id).map(|obj| obj.name.clone()))
        .collect();
    assert!(
        battlefield_names
            .iter()
            .any(|name| name == "Alice Permanent"),
        "expected Alice Permanent on the battlefield after Over the Top, got {battlefield_names:?}"
    );
}

#[test]
fn parse_oracle_myr_landshaper_type_addition_render_regression() {
    let def = parse_oracle_card_definition("Myr Landshaper");

    let raw = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        raw.contains("addcardtypes") && raw.contains("artifact"),
        "expected raw compiled definition to keep artifact type addition, got {raw}"
    );

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains(
            "target land becomes an artifact in addition to its other types until end of turn"
        ),
        "expected Myr Landshaper type-addition wording, got {rendered}"
    );
    assert!(
        !rendered.contains("unsupported effect"),
        "expected Myr Landshaper to avoid unsupported markers, got {rendered}"
    );
}

#[test]
fn parse_oracle_kavu_recluse_fixed_basic_land_type_regression() {
    let def = parse_oracle_card_definition("Kavu Recluse");

    let raw = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        raw.contains("becomebasiclandtypechoiceeffect")
            && raw.contains("fixed_subtype")
            && raw.contains("forest"),
        "expected raw compiled definition to use fixed basic land type lowering, got {raw}"
    );

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("target land becomes")
            && rendered.contains("forest")
            && rendered.contains("until end of turn"),
        "expected Kavu Recluse basic-land-type wording, got {rendered}"
    );
    assert!(
        !rendered.contains("unsupported effect"),
        "expected Kavu Recluse to avoid unsupported markers, got {rendered}"
    );
}

#[test]
fn parse_oracle_slimy_kavu_fixed_basic_land_type_regression() {
    let def = parse_oracle_card_definition("Slimy Kavu");

    let raw = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        raw.contains("becomebasiclandtypechoiceeffect")
            && raw.contains("fixed_subtype")
            && raw.contains("swamp"),
        "expected raw compiled definition to use fixed basic land type lowering, got {raw}"
    );

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("target land becomes")
            && rendered.contains("swamp")
            && rendered.contains("until end of turn"),
        "expected Slimy Kavu basic-land-type wording, got {rendered}"
    );
    assert!(
        !rendered.contains("unsupported effect"),
        "expected Slimy Kavu to avoid unsupported markers, got {rendered}"
    );
}

#[test]
fn parse_oracle_tidal_warrior_fixed_basic_land_type_regression() {
    let def = parse_oracle_card_definition("Tidal Warrior");

    let raw = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        raw.contains("becomebasiclandtypechoiceeffect")
            && raw.contains("fixed_subtype")
            && raw.contains("island"),
        "expected raw compiled definition to use fixed basic land type lowering, got {raw}"
    );

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("target land becomes")
            && rendered.contains("island")
            && rendered.contains("until end of turn"),
        "expected Tidal Warrior basic-land-type wording, got {rendered}"
    );
    assert!(
        !rendered.contains("unsupported effect"),
        "expected Tidal Warrior to avoid unsupported markers, got {rendered}"
    );
}

#[test]
fn parse_oracle_master_biomancer_etb_mutant_regression() {
    let def = parse_oracle_card_definition("Master Biomancer");

    let raw = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        raw.contains("enterwithcountersforfilter")
            && raw.contains("power")
            && raw.contains("added_subtypes")
            && raw.contains("mutant"),
        "expected raw compiled definition to retain dynamic ETB counters and mutant subtype, got {raw}"
    );

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("each other creature you control enters with")
            && rendered.contains("additional +1/+1 counters")
            && rendered.contains("equal to this creature's power")
            && rendered.contains("mutant"),
        "expected Master Biomancer ETB mutant wording, got {rendered}"
    );
    assert!(
        !rendered.contains("enter the battlefield with counters"),
        "expected Master Biomancer to avoid generic ETB counter placeholder text, got {rendered}"
    );
}

#[test]
fn parse_oracle_skanos_dragonheart_greatest_power_regression() {
    let def = parse_oracle_card_definition("Skanos Dragonheart");

    let raw = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        raw.contains("greatestpower") && raw.contains("dragon") && raw.contains("graveyard"),
        "expected raw compiled definition to retain the greatest-power source expression, got {raw}"
    );

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("greatest power")
            && rendered.contains("other dragons you control")
            && rendered.contains("dragon cards in your graveyard"),
        "expected Skanos Dragonheart to render its greatest-power explanation, got {rendered}"
    );
}

#[test]
fn parse_oracle_accomplished_automaton_fabricate_one_regression() {
    let def = parse_oracle_card_definition("Accomplished Automaton");

    let raw = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        raw.contains("put a +1/+1 counter on this creature")
            && raw.contains("create a 1/1 colorless servo artifact creature token"),
        "expected raw compiled definition to singularize fabricate-1 mode descriptions, got {raw}"
    );

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("put a +1/+1 counter on this creature"),
        "expected Accomplished Automaton to singularize the counter mode, got {rendered}"
    );
    assert!(
        rendered.contains("create a 1/1 colorless servo artifact creature token"),
        "expected Accomplished Automaton to singularize the token mode, got {rendered}"
    );
}

#[test]
fn parse_oracle_ambitious_aetherborn_fabricate_one_regression() {
    let def = parse_oracle_card_definition("Ambitious Aetherborn");

    let raw = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        raw.contains("put a +1/+1 counter on this creature")
            && raw.contains("create a 1/1 colorless servo artifact creature token"),
        "expected raw compiled definition to singularize fabricate-1 mode descriptions, got {raw}"
    );

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("put a +1/+1 counter on this creature"),
        "expected Ambitious Aetherborn to singularize the counter mode, got {rendered}"
    );
    assert!(
        rendered.contains("create a 1/1 colorless servo artifact creature token"),
        "expected Ambitious Aetherborn to singularize the token mode, got {rendered}"
    );
}

#[test]
fn parse_oracle_glint_sleeve_artisan_fabricate_one_regression() {
    let def = parse_oracle_card_definition("Glint-Sleeve Artisan");

    let raw = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        raw.contains("put a +1/+1 counter on this creature")
            && raw.contains("create a 1/1 colorless servo artifact creature token"),
        "expected raw compiled definition to singularize fabricate-1 mode descriptions, got {raw}"
    );

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("put a +1/+1 counter on this creature"),
        "expected Glint-Sleeve Artisan to singularize the counter mode, got {rendered}"
    );
    assert!(
        rendered.contains("create a 1/1 colorless servo artifact creature token"),
        "expected Glint-Sleeve Artisan to singularize the token mode, got {rendered}"
    );
}

#[test]
fn parse_oracle_arwen_weaver_of_hope_dynamic_etb_counters_regression() {
    let def = parse_oracle_card_definition("Arwen, Weaver of Hope");

    let raw = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        raw.contains("enterwithcountersforfilter")
            && raw.contains("toughness")
            && raw.contains("other creature"),
        "expected raw compiled definition to retain toughness-based ETB counters, got {raw}"
    );

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("each other creature you control enters with")
            && rendered.contains("additional +1/+1 counters")
            && (rendered.contains("equal to arwen's toughness")
                || rendered.contains("equal to this creature's toughness")),
        "expected Arwen, Weaver of Hope to render toughness-based ETB counters, got {rendered}"
    );
    assert!(
        !rendered.contains("enter the battlefield with counters"),
        "expected Arwen, Weaver of Hope to avoid generic ETB counter placeholder text, got {rendered}"
    );
}

#[test]
fn parse_oracle_grumgully_the_generous_etb_counter_regression() {
    let def = parse_oracle_card_definition("Grumgully, the Generous");

    let raw = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        raw.contains("enterwithcountersforfilter")
            && raw.contains("excluded_subtypes")
            && raw.contains("human")
            && raw.contains("plusoneplusone"),
        "expected raw compiled definition to retain the non-Human ETB counter filter, got {raw}"
    );

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("each other")
            && rendered
                .contains("creature you control enters with an additional +1/+1 counter on it")
            && (rendered.contains("non-human") || rendered.contains("nonhuman")),
        "expected Grumgully, the Generous to render its ETB counter text, got {rendered}"
    );
    assert!(
        !rendered.contains("enter the battlefield with counters"),
        "expected Grumgully, the Generous to avoid generic ETB counter placeholder text, got {rendered}"
    );
}

#[test]
fn oracle_render_regression_named_cards_compile_cleanly() {
    let cultivator =
        oracle_like_lines(&parse_oracle_card_definition("Cultivator Colossus")).join("\n");
    assert!(
        cultivator.contains(
            "When Cultivator Colossus enters, you may put a land card from your hand onto the battlefield tapped. If you do, draw a card and repeat this process."
        ),
        "expected Cultivator Colossus repeat-process text, got {cultivator}"
    );
    assert!(
        !cultivator.to_ascii_lowercase().contains("unsupported"),
        "expected Cultivator Colossus to render without unsupported markers, got {cultivator}"
    );

    let one_ring = oracle_like_lines(&parse_oracle_card_definition("The One Ring")).join("\n");
    assert!(
        one_ring.contains("gain protection from everything until your next turn"),
        "expected The One Ring protection wording, got {one_ring}"
    );
    assert!(
        one_ring.contains("burden counter"),
        "expected The One Ring burden-counter text, got {one_ring}"
    );

    let boseiju =
        oracle_like_lines(&parse_oracle_card_definition("Boseiju, Who Endures")).join("\n");
    assert!(
        boseiju.contains("Channel")
            && boseiju.contains("Destroy target")
            && boseiju.contains("artifact")
            && boseiju.contains("enchantment")
            && boseiju.contains("land")
            && boseiju.contains(
                "This ability costs {1} less to activate for each legendary creature you control"
            ),
        "expected Boseiju channel rendering, got {boseiju}"
    );

    let hanweir =
        oracle_like_lines(&parse_oracle_card_definition("Hanweir Battlements")).join("\n");
    assert!(
        hanweir.contains("Hanweir Garrison") || hanweir.contains("hanweir garrison"),
        "expected Hanweir Battlements meld clause to compile, got {hanweir}"
    );
    assert!(
        !hanweir.to_ascii_lowercase().contains("unsupported"),
        "expected Hanweir Battlements to render without unsupported markers, got {hanweir}"
    );

    let otawara =
        oracle_like_lines(&parse_oracle_card_definition("Otawara, Soaring City")).join("\n");
    assert!(
        otawara.contains("Channel")
            && otawara.contains("Return target")
            && otawara.contains("artifact")
            && otawara.contains("creature")
            && otawara.contains("enchantment")
            && otawara.contains("planeswalker")
            && otawara.contains(
                "This ability costs {1} less to activate for each legendary creature you control"
            ),
        "expected Otawara channel rendering, got {otawara}"
    );

    let tolaria = oracle_like_lines(&parse_oracle_card_definition("Tolaria West")).join("\n");
    assert!(
        tolaria.contains("Transmute {1}{U}{U}") && tolaria.contains("mana value"),
        "expected Tolaria West transmute rendering, got {tolaria}"
    );
    assert!(
        !tolaria.contains("permanent card"),
        "expected Tolaria West to avoid placeholder search text, got {tolaria}"
    );
}

#[derive(serde::Deserialize)]
struct RegressionCardFaceJson {
    name: String,
    oracle_text: Option<String>,
}

#[derive(serde::Deserialize)]
struct RegressionCardJson {
    name: String,
    oracle_text: Option<String>,
    type_line: Option<String>,
    card_faces: Option<Vec<RegressionCardFaceJson>>,
    lang: Option<String>,
}

#[derive(Clone)]
struct RegressionOracleCardInfo {
    oracle_text: String,
    type_line: Option<String>,
}

fn oracle_card_info_by_name() -> &'static HashMap<String, RegressionOracleCardInfo> {
    static ORACLE_BY_NAME: OnceLock<HashMap<String, RegressionOracleCardInfo>> = OnceLock::new();
    ORACLE_BY_NAME.get_or_init(|| {
        let raw =
            std::fs::read_to_string("cards.json").expect("read cards.json for regression tests");
        let cards: Vec<RegressionCardJson> =
            serde_json::from_str(&raw).expect("parse cards.json for regression tests");
        let mut out = HashMap::new();
        for card in cards {
            if card.lang.as_deref().unwrap_or("en") != "en" {
                continue;
            }

            let full_name = card.name;
            let root_text = card.oracle_text.and_then(|text| {
                let trimmed = text.trim();
                (!trimmed.is_empty()).then(|| trimmed.to_string())
            });

            let mut face_entries = Vec::new();
            if let Some(faces) = card.card_faces {
                for face in faces {
                    let Some(text) = face.oracle_text.and_then(|text| {
                        let trimmed = text.trim();
                        (!trimmed.is_empty()).then(|| trimmed.to_string())
                    }) else {
                        continue;
                    };
                    face_entries.push((face.name, text));
                }
            }

            let Some(primary_text) = root_text
                .clone()
                .or_else(|| face_entries.first().map(|(_, text)| text.clone()))
            else {
                continue;
            };

            out.entry(full_name.clone())
                .or_insert(RegressionOracleCardInfo {
                    oracle_text: primary_text.clone(),
                    type_line: card.type_line.clone(),
                });
            if full_name.contains(" // ") {
                for part in full_name.split(" // ") {
                    out.entry(part.to_string())
                        .or_insert(RegressionOracleCardInfo {
                            oracle_text: primary_text.clone(),
                            type_line: card.type_line.clone(),
                        });
                }
            }
            for (face_name, face_text) in face_entries {
                out.entry(face_name).or_insert(RegressionOracleCardInfo {
                    oracle_text: face_text,
                    type_line: card.type_line.clone(),
                });
            }
        }
        out
    })
}

fn oracle_text_by_name() -> &'static HashMap<String, String> {
    static ORACLE_TEXT_BY_NAME: OnceLock<HashMap<String, String>> = OnceLock::new();
    ORACLE_TEXT_BY_NAME.get_or_init(|| {
        oracle_card_info_by_name()
            .iter()
            .map(|(name, info)| (name.clone(), info.oracle_text.clone()))
            .collect()
    })
}

fn card_types_from_type_line(type_line: &str) -> Vec<CardType> {
    type_line
        .split('—')
        .next()
        .unwrap_or(type_line)
        .split_whitespace()
        .filter_map(
            |word| match word.trim_matches(|ch: char| !ch.is_ascii_alphabetic()) {
                "Artifact" => Some(CardType::Artifact),
                "Battle" => Some(CardType::Battle),
                "Creature" => Some(CardType::Creature),
                "Enchantment" => Some(CardType::Enchantment),
                "Instant" => Some(CardType::Instant),
                "Land" => Some(CardType::Land),
                "Planeswalker" => Some(CardType::Planeswalker),
                "Sorcery" => Some(CardType::Sorcery),
                "Kindred" => Some(CardType::Kindred),
                _ => None,
            },
        )
        .collect()
}

fn parse_oracle_card_definition(name: &str) -> CardDefinition {
    let info = oracle_card_info_by_name()
        .get(name)
        .unwrap_or_else(|| panic!("missing oracle text for regression card '{name}'"));
    let mut builder = CardDefinitionBuilder::new(CardId::new(), name);
    if let Some(type_line) = info.type_line.as_deref() {
        let card_types = card_types_from_type_line(type_line);
        if !card_types.is_empty() {
            builder = builder.card_types(card_types);
        }
    }
    builder
        .parse_text(info.oracle_text.clone())
        .unwrap_or_else(|err| panic!("strict parser regression failed for '{name}': {err:?}"))
}

fn assert_oracle_card_parses_strict(name: &str) {
    let oracle = oracle_text_by_name()
        .get(name)
        .unwrap_or_else(|| panic!("missing oracle text for regression card '{name}'"))
        .clone();
    let result = CardDefinitionBuilder::new(CardId::new(), name).parse_text(oracle.clone());
    assert!(
        result.is_ok(),
        "strict parser regression failed for '{name}': {:?}\nOracle text:\n{}",
        result.err(),
        oracle
    );
}

fn assert_oracle_card_fails_strict(name: &str) {
    let oracle = oracle_text_by_name()
        .get(name)
        .unwrap_or_else(|| panic!("missing oracle text for regression card '{name}'"))
        .clone();
    let result = CardDefinitionBuilder::new(CardId::new(), name).parse_text(oracle.clone());
    assert!(
        result.is_err(),
        "strict parser regression expected failure for '{name}', but parse succeeded.\nOracle text:\n{}",
        oracle
    );
}

const STRICT_PARSE_REGRESSION_SUCCESS_CARDS: &[&str] = &[
    "Blast Zone",
    "Boseiju, Who Endures",
    "Cabal Ritual",
    "Cavern of Souls",
    "Cultivator Colossus",
    "Echoing Deeps",
    "Fatal Push",
    "Golgari Thug",
    "Grief",
    "Mox Amber",
    "Nine-Lives Familiar",
    "Otawara, Soaring City",
    "Orcish Bowmasters",
    "Pawn of Ulamog",
    "Genesis Chamber",
    "Sacrifice",
    "Sephiroth, Fabled SOLDIER",
    "Shifting Woodland",
    "Spelunking",
    "Susurian Voidborn",
    "Talon Gates of Madara",
    "The Mycosynth Gardens",
    "The Stone Brain",
    "Tolaria West",
    "Turn the Earth",
    "Unmarked Grave",
    "Vesuva",
];

const STRICT_PARSE_REGRESSION_EXPECTED_FAILURE_CARDS: &[&str] = &[
    "Clown Car",
    "Gravecrawler",
    "Hancock, Ghoulish Mayor",
    "Lake of the Dead",
    "Nykthos, Shrine to Nyx",
    "The Soul Stone",
];

macro_rules! strict_parse_card_test {
    ($test_name:ident, $card_name:expr) => {
        #[test]
        fn $test_name() {
            assert_oracle_card_parses_strict($card_name);
        }
    };
}

macro_rules! strict_parse_card_expected_fail_test {
    ($test_name:ident, $card_name:expr) => {
        #[test]
        fn $test_name() {
            assert_oracle_card_fails_strict($card_name);
        }
    };
}

strict_parse_card_test!(strict_parse_blast_zone, "Blast Zone");
strict_parse_card_test!(strict_parse_bridge_from_below, "Bridge from Below");
strict_parse_card_test!(strict_parse_cabal_ritual, "Cabal Ritual");
strict_parse_card_test!(strict_parse_cavern_of_souls, "Cavern of Souls");
strict_parse_card_expected_fail_test!(strict_parse_clown_car, "Clown Car");
strict_parse_card_test!(strict_parse_fatal_push, "Fatal Push");
strict_parse_card_test!(strict_parse_gemstone_caverns, "Gemstone Caverns");
strict_parse_card_test!(strict_parse_golgari_thug, "Golgari Thug");
strict_parse_card_expected_fail_test!(strict_parse_gravecrawler, "Gravecrawler");
strict_parse_card_test!(strict_parse_grief, "Grief");
strict_parse_card_expected_fail_test!(
    strict_parse_hancock_ghoulish_mayor,
    "Hancock, Ghoulish Mayor"
);
strict_parse_card_expected_fail_test!(strict_parse_lake_of_the_dead, "Lake of the Dead");
strict_parse_card_test!(strict_parse_mox_amber, "Mox Amber");
strict_parse_card_test!(strict_parse_nine_lives_familiar, "Nine-Lives Familiar");
strict_parse_card_expected_fail_test!(strict_parse_nykthos_shrine_to_nyx, "Nykthos, Shrine to Nyx");
strict_parse_card_test!(strict_parse_orcish_bowmasters, "Orcish Bowmasters");
strict_parse_card_test!(strict_parse_pawn_of_ulamog, "Pawn of Ulamog");
strict_parse_card_test!(strict_parse_genesis_chamber, "Genesis Chamber");
strict_parse_card_test!(strict_parse_sacrifice, "Sacrifice");
strict_parse_card_test!(
    strict_parse_sephiroth_fabled_soldier,
    "Sephiroth, Fabled SOLDIER"
);
strict_parse_card_test!(strict_parse_susurian_voidborn, "Susurian Voidborn");
strict_parse_card_test!(strict_parse_talon_gates_of_madara, "Talon Gates of Madara");
strict_parse_card_expected_fail_test!(strict_parse_the_soul_stone, "The Soul Stone");
strict_parse_card_test!(strict_parse_unmarked_grave, "Unmarked Grave");

#[test]
fn strict_parse_regression_batch_target_cards() {
    let mut failures = Vec::new();
    for &name in STRICT_PARSE_REGRESSION_SUCCESS_CARDS {
        let oracle = match oracle_text_by_name().get(name) {
            Some(text) => text.clone(),
            None => {
                failures.push(format!("{name}: missing oracle text in cards.json"));
                continue;
            }
        };
        if let Err(err) = CardDefinitionBuilder::new(CardId::new(), name).parse_text(oracle.clone())
        {
            failures.push(format!("{name}: {err:?}"));
        }
    }
    for &name in STRICT_PARSE_REGRESSION_EXPECTED_FAILURE_CARDS {
        let oracle = match oracle_text_by_name().get(name) {
            Some(text) => text.clone(),
            None => {
                failures.push(format!("{name}: missing oracle text in cards.json"));
                continue;
            }
        };
        if CardDefinitionBuilder::new(CardId::new(), name)
            .parse_text(oracle.clone())
            .is_ok()
        {
            failures.push(format!(
                "{name}: expected strict parse failure, but parse succeeded"
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "strict parse regression batch failures:\n{}",
        failures.join("\n")
    );
}

#[test]
fn strict_parse_shared_parser_regression_cards() {
    for name in [
        "Tarmogoyf",
        "Carnage Interpreter",
        "Narset, Parter of Veils",
        "Leovold, Emissary of Trest",
        "Emberwilde Captain",
        "Palace Jailer",
        "Aragorn, King of Gondor",
        "Lightning Greaves",
        "Skullclamp",
        "Eagles of the North",
        "Lórien Revealed",
        "Loran of the Third Path",
        "Phelia, Exuberant Shepherd",
        "Sage of the Skies",
    ] {
        assert_oracle_card_parses_strict(name);
    }
}

#[test]
fn parse_skyclave_apparition_where_x_uses_exiled_card_mana_value() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Skyclave Apparition Variant")
        .parse_text(
            "When this creature enters, exile up to one target nonland, nontoken permanent you don't control with mana value 4 or less.\nWhen this creature leaves the battlefield, the exiled card's owner creates an X/X blue Illusion creature token, where X is the mana value of the exiled card.",
        )
        .expect("skyclave-style where-x clause should parse");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("ManaValueOf"),
        "expected exiled-card mana value binding in lowered ability, got {debug}"
    );
}

#[test]
fn where_x_exiled_card_plus_one_still_fails_loudly() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Broken Skyclave Variant")
        .parse_text(
            "When this creature leaves the battlefield, the exiled card's owner creates an X/X blue Illusion creature token, where X is the mana value of the exiled card plus one.",
        )
        .expect_err("unsupported where-x math tail should still fail");

    let rendered = format!("{err:?}").to_ascii_lowercase();
    assert!(
        rendered.contains("unsupported where-x clause") || rendered.contains("plus one"),
        "expected loud where-x failure, got {rendered}"
    );
}

#[test]
fn parse_beza_relative_opponent_comparisons() {
    CardDefinitionBuilder::new(CardId::new(), "Beza Variant")
        .parse_text(
            "When this creature enters, create a Treasure token if an opponent controls more lands than you. You gain 4 life if an opponent has more life than you. Create two 1/1 blue Fish creature tokens if an opponent controls more creatures than you. Draw a card if an opponent has more cards in hand than you.",
        )
        .expect("beza-style relative comparisons should parse");
}

#[test]
fn parse_thieving_skydiver_equipment_followup_condition() {
    CardDefinitionBuilder::new(CardId::new(), "Thieving Skydiver Variant")
        .parse_text(
            "When this creature enters, if it was kicked, gain control of target artifact with mana value X or less. If that artifact is an Equipment, attach it to this creature.",
        )
        .expect("tagged equipment followup should parse");
}

#[test]
fn parse_currency_converter_nonland_followup_condition() {
    CardDefinitionBuilder::new(CardId::new(), "Currency Converter Variant")
        .parse_text(
            "{T}: Put a card exiled with this artifact into its owner's graveyard. If it's a land card, create a Treasure token. If it's a nonland card, create a 2/2 black Rogue creature token.",
        )
        .expect("tagged nonland-card followup should parse");
}

#[test]
fn parse_dauthi_voidwalker_void_counter_target_phrase() {
    CardDefinitionBuilder::new(CardId::new(), "Dauthi Voidwalker Variant")
        .parse_text(
            "{T}, Sacrifice this creature: Choose an exiled card an opponent owns with a void counter on it. You may play it this turn without paying its mana cost.",
        )
        .expect("tagged counter-state exiled-card choice should parse");
}

#[test]
fn parse_dauthi_voidwalker_full_text_without_parser_fallback() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Dauthi Voidwalker Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Shadow\nIf a card would be put into an opponent's graveyard from anywhere, instead exile it with a void counter on it.\n{T}, Sacrifice this creature: Choose an exiled card an opponent owns with a void counter on it. You may play it this turn without paying its mana cost.",
        )
        .expect("Dauthi Voidwalker text should parse");

    let abilities_debug = format!("{:#?}", def.abilities);
    assert!(
        !abilities_debug.contains("UnsupportedParserLine"),
        "expected full Dauthi text to avoid unsupported parser fallbacks, got {abilities_debug}"
    );
    assert!(
        abilities_debug.contains("ExileToCounteredExileInsteadOfGraveyard"),
        "expected Dauthi replacement ability to lower to a real static ability, got {abilities_debug}"
    );
    assert!(
        abilities_debug.contains("ChooseObjectsEffect")
            && abilities_debug.contains("zone: Some(Exile)"),
        "expected Dauthi activation to choose from exile, got {abilities_debug}"
    );
    assert!(
        abilities_debug.contains("GrantTaggedSpellFreeCastUntilEndOfTurnEffect"),
        "expected Dauthi activation to preserve the free-cast clause, got {abilities_debug}"
    );
}

#[test]
fn parse_vaultborn_tyrant_nontoken_followup_condition() {
    CardDefinitionBuilder::new(CardId::new(), "Vaultborn Tyrant Variant")
        .parse_text(
            "When this creature dies, if it's not a token, create a token that's a copy of it, except it's an artifact in addition to its other types.",
        )
        .expect("tagged nontoken followup should parse");
}

#[test]
fn parse_time_vault_skip_that_turn_clause() {
    CardDefinitionBuilder::new(CardId::new(), "Time Vault Variant")
        .parse_text(
            "This artifact enters tapped.\nThis artifact doesn't untap during your untap step.\nIf you would begin your turn while this artifact is tapped, you may skip that turn instead. If you do, untap this artifact.\n{T}: Take an extra turn after this one.",
        )
        .expect("time-vault skip-that-turn clause should parse");
}

#[test]
fn parse_portal_to_phyrexia_subtype_followup_sentence() {
    CardDefinitionBuilder::new(CardId::new(), "Portal to Phyrexia Variant")
        .parse_text(
            "At the beginning of your upkeep, put target creature card from a graveyard onto the battlefield under your control. It's a Phyrexian in addition to its other types.",
        )
        .expect("implicit tagged subtype followup should parse");
}

#[test]
fn parse_ghost_vacuum_base_pt_and_subtype_followup_sentence() {
    CardDefinitionBuilder::new(CardId::new(), "Ghost Vacuum Variant")
        .parse_text(
            "{6}, {T}, Sacrifice this artifact: Put each creature card exiled with this artifact onto the battlefield under your control with a flying counter on it. Each of them is a 1/1 Spirit in addition to its other types. Activate only as a sorcery.",
        )
        .expect("implicit tagged base-pt followup should parse");
}

#[test]
fn enduring_curiosity_still_fails_without_type_removal_support() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Enduring Curiosity Variant")
        .parse_text(
            "When this creature dies, if it was a creature, return it to the battlefield under its owner's control. It's an enchantment. (It's not a creature.)",
        )
        .expect_err("enduring return line must stay unsupported until type-removal semantics exist");

    let rendered = format!("{err:?}").to_ascii_lowercase();
    assert!(
        rendered.contains("could not find verb in effect clause")
            || rendered.contains("unsupported"),
        "expected loud unsupported-type-removal failure, got {rendered}"
    );
}

#[test]
fn parse_cecil_half_starting_life_threshold() {
    CardDefinitionBuilder::new(CardId::new(), "Cecil Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Deathtouch\nWhenever this creature deals damage, you lose that much life. Then if your life total is less than or equal to half your starting life total, untap this creature and transform it.",
        )
        .expect("half-starting-life threshold should parse");
}

#[test]
fn half_starting_life_threshold_with_extra_math_still_fails_loudly() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Half Starting Threshold Negative Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Whenever this creature deals damage, if your life total is less than or equal to half your starting life total plus one, untap this creature.",
        )
        .expect_err("unsupported extra math after half-starting-life threshold should fail");

    let rendered = format!("{err:?}").to_ascii_lowercase();
    assert!(
        rendered.contains("unsupported")
            || rendered.contains("could not find verb")
            || rendered.contains("unsupported predicate"),
        "expected loud failure for unsupported threshold math, got {rendered}"
    );
}

#[test]
fn parse_magda_other_dwarves_anthem() {
    CardDefinitionBuilder::new(CardId::new(), "Magda Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Other Dwarves you control get +1/+0.\nWhenever a Dwarf you control becomes tapped, create a Treasure token.\nSacrifice five Treasures: Search your library for an artifact or Dragon card, put that card onto the battlefield, then shuffle.",
        )
        .expect("Magda rules text should parse");
}

#[test]
fn parse_screaming_nemesis_any_other_target() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Screaming Nemesis Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Haste\nWhenever this creature is dealt damage, it deals that much damage to any other target. If a player is dealt damage this way, they can't gain life for the rest of the game.",
        )
        .expect("any-other-target damage followup should parse");
    let abilities_debug = format!("{:#?}", def.abilities);
    assert!(
        abilities_debug.contains("AnyOtherTarget"),
        "expected any-other-target semantics to survive lowering, got {abilities_debug}"
    );
}

#[test]
fn parse_burst_lightning_kicker_instead_clause() {
    CardDefinitionBuilder::new(CardId::new(), "Burst Lightning Variant")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Kicker {4} (You may pay an additional {4} as you cast this spell.)\nThis spell deals 2 damage to any target. If this spell was kicked, it deals 4 damage instead.",
        )
        .expect("kicker damage-instead followup should parse");
}

#[test]
fn parse_consult_the_star_charts_kicker_count_override() {
    CardDefinitionBuilder::new(CardId::new(), "Consult Variant")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Kicker {1}{U} (You may pay an additional {1}{U} as you cast this spell.)\nLook at the top X cards of your library, where X is the number of lands you control. Put one of those cards into your hand. If this spell was kicked, put two of those cards into your hand instead. Put the rest on the bottom of your library in a random order.",
        )
        .expect("look-top X kicker count override should parse");
}

#[test]
fn consult_the_star_charts_kicker_override_with_extra_tail_still_fails_loudly() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Consult Variant")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Kicker {1}{U} (You may pay an additional {1}{U} as you cast this spell.)\nLook at the top X cards of your library, where X is the number of lands you control. Put one of those cards into your hand. If this spell was kicked, put two of those cards into your hand instead this turn. Put the rest on the bottom of your library in a random order.",
        )
        .expect_err("unsupported kicked looked-card tail should fail");
    let rendered = err.to_string();
    assert!(
        rendered.contains("unsupported")
            || rendered.contains("could not parse")
            || rendered.contains("expected"),
        "expected loud failure for unsupported kicked looked-card tail, got {rendered}"
    );
}

#[test]
fn parse_planar_genesis_looked_card_fallback_sequence() {
    CardDefinitionBuilder::new(CardId::new(), "Planar Genesis Variant")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Look at the top four cards of your library. You may put a land card from among them onto the battlefield tapped. If you don't, put a card from among them into your hand. Put the rest on the bottom of your library in a random order.",
        )
        .expect("looked-card battlefield-or-hand fallback should parse");
}

#[test]
fn planar_genesis_fallback_with_extra_tail_still_fails_loudly() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Planar Genesis Variant")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Look at the top four cards of your library. You may put a land card from among them onto the battlefield tapped. If you don't, put a card from among them into your hand this turn. Put the rest on the bottom of your library in a random order.",
        )
        .expect_err("unsupported looked-card fallback tail should fail");
    let rendered = err.to_string();
    assert!(
        rendered.contains("unsupported")
            || rendered.contains("could not parse")
            || rendered.contains("expected"),
        "expected loud failure for unsupported looked-card fallback tail, got {rendered}"
    );
}

#[test]
fn parse_caustic_bronco_saddled_followup_condition() {
    CardDefinitionBuilder::new(CardId::new(), "Caustic Bronco Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Whenever this creature attacks, reveal the top card of your library and put it into your hand. You lose life equal to that card's mana value if this creature isn't saddled. Otherwise, each opponent loses that much life.\nSaddle 3 (Tap any number of other creatures you control with total power 3 or more: This Mount becomes saddled until end of turn. Saddle only as a sorcery.)",
        )
        .expect("saddled conditional reveal-life trigger should parse");
}

#[test]
fn caustic_bronco_saddled_followup_with_extra_tail_still_fails_loudly() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Caustic Bronco Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Whenever this creature attacks, reveal the top card of your library and put it into your hand. You lose life equal to that card's mana value if this creature isn't saddled this turn. Otherwise, each opponent loses that much life.\nSaddle 3 (Tap any number of other creatures you control with total power 3 or more: This Mount becomes saddled until end of turn. Saddle only as a sorcery.)",
        )
        .expect_err("unsupported saddled conditional tail should fail");
    let rendered = err.to_string();
    assert!(
        rendered.contains("unsupported")
            || rendered.contains("could not parse")
            || rendered.contains("expected"),
        "expected loud failure for unsupported saddled conditional tail, got {rendered}"
    );
}

#[test]
fn parse_minsc_and_boo_hamster_followup_condition() {
    CardDefinitionBuilder::new(CardId::new(), "Minsc Variant")
        .card_types(vec![CardType::Planeswalker])
        .parse_text(
            "When this permanent enters and at the beginning of your upkeep, you may create Boo, a legendary 1/1 red Hamster creature token with trample and haste.\n+1: Put three +1/+1 counters on up to one target creature with trample or haste.\n-2: Sacrifice a creature. When you do, this permanent deals X damage to any target, where X is that creature's power. If the sacrificed creature was a Hamster, draw X cards.",
        )
        .expect("sacrificed-creature subtype followup should parse");
}

#[test]
fn sacrificed_creature_was_hamster_with_extra_tail_still_fails_loudly() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Hamster Tail Negative Variant")
        .card_types(vec![CardType::Planeswalker])
        .parse_text(
            "-2: Sacrifice a creature. When you do, this permanent deals X damage to any target, where X is that creature's power. If the sacrificed creature was a Hamster this turn, draw X cards.",
        )
        .expect_err("unsupported sacrificed-creature predicate tail should fail");

    let rendered = format!("{err:?}").to_ascii_lowercase();
    assert!(
        rendered.contains("unsupported")
            || rendered.contains("unsupported predicate")
            || rendered.contains("could not find verb"),
        "expected loud failure for unsupported sacrificed-creature tail, got {rendered}"
    );
}
