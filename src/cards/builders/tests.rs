use super::*;
use crate::ability::AbilityKind;
use crate::color::Color;
use crate::compiled_text::{compiled_lines, oracle_like_lines};
use crate::static_abilities::StaticAbilityId;
use crate::target::{ChooseSpec, ObjectRef, PlayerFilter};

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
fn test_parse_prowess_keyword_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Prowess Creature")
        .card_types(vec![CardType::Creature])
        .parse_text("Prowess")
        .expect("parse prowess keyword");

    let triggered = def
        .abilities
        .iter()
        .find_map(|a| match &a.kind {
            AbilityKind::Triggered(t) => Some(t),
            _ => None,
        })
        .expect("expected prowess triggered ability");

    assert_eq!(def.abilities.len(), 1);
    assert_eq!(def.abilities[0].text.as_deref(), Some("Prowess"));
    assert!(triggered.trigger.display().contains("you cast"));
    assert!(triggered.trigger.display().contains("noncreature spell"));
    assert!(
        format!("{:?}", triggered.effects[0]).contains("ModifyPowerToughnessEffect"),
        "expected pump effect, got {:?}",
        triggered.effects
    );

    let lines = compiled_lines(&def);
    assert!(
        lines
            .iter()
            .any(|line| line == "Keyword ability 1: Prowess"),
        "expected keyword rendering for prowess, got {lines:?}"
    );
}

#[test]
fn test_parse_magecraft_cast_or_copy_trigger_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Witherbloom Apprentice")
            .card_types(vec![CardType::Creature])
            .parse_text(
                "Magecraft — Whenever you cast or copy an instant or sorcery spell, each opponent loses 1 life and you gain 1 life.",
            )
            .expect("parse magecraft cast-or-copy trigger");

    let triggered = def
        .abilities
        .iter()
        .find_map(|a| match &a.kind {
            AbilityKind::Triggered(t) => Some(t),
            _ => None,
        })
        .expect("expected magecraft triggered ability");

    let trigger_text = triggered.trigger.display().to_ascii_lowercase();
    assert!(
        trigger_text.contains("you cast") && trigger_text.contains("you copy"),
        "expected trigger display to include cast and copy, got {trigger_text}"
    );
    assert!(
        trigger_text.contains("instant or sorcery"),
        "expected instant/sorcery filter in trigger display, got {trigger_text}"
    );

    let lines = compiled_lines(&def);
    let joined = lines.join(" ");
    assert!(
        joined.contains("copy"),
        "expected compiled rendering to include copy trigger, got {joined}"
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
    assert!(
        debug.contains("ChooseNewTargetsEffect"),
        "expected choose-new-targets effect in spell text, got {debug}"
    );
    let lines = compiled_lines(&def);
    let joined = lines.join(" ").to_ascii_lowercase();
    assert!(
        joined.contains("target player may copy this spell")
            && !joined.contains("you may copy this spell"),
        "expected copy permission to stay linked to targeted player, got {joined}"
    );
    assert!(
        joined.contains("target player may choose new targets")
            && !joined.contains("you may choose new targets"),
        "expected retarget permission to stay linked to targeted player, got {joined}"
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
        joined.contains("whenever a vampire you control deals combat damage to a player")
            && !joined.contains("whenever this creature deals combat damage to a player"),
        "expected trigger subject to remain filtered, got {joined}"
    );
}

#[test]
fn test_parse_trigger_this_deals_damage_to_filtered_creature_targets_damage_recipient() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Damage Recipient Filter Probe")
        .card_types(vec![CardType::Enchantment])
        .parse_text("Whenever this creature deals damage to a Vampire, destroy that creature.")
        .expect("parse filtered non-combat damage trigger");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("ThisDealsDamageToTrigger"),
        "expected dedicated damage-recipient trigger matcher, got {debug}"
    );
    let joined = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains("whenever this permanent deals damage to vampire"),
        "expected trigger to include damaged-object filter, got {joined}"
    );
    assert!(
        joined.contains("destroy it"),
        "expected effects to target tagged damaged object instead of source, got {joined}"
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
fn test_parse_trigger_unknown_non_source_subject_fails() {
    let err = CardDefinitionBuilder::new(CardId::from_raw(1), "Unknown Subject Probe")
        .card_types(vec![CardType::Enchantment])
        .parse_text("Whenever a creature that attacks, draw a card.")
        .expect_err("unknown non-source subject should fail");
    let message = format!("{err:?}");
    assert!(
        message.contains("unsupported trigger subject filter")
            || message.contains("unsupported trigger clause"),
        "expected strict trigger-subject parse failure, got {message}"
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
fn test_parse_bushido_keyword_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Samurai")
            .card_types(vec![CardType::Creature])
            .parse_text(
                "Bushido 1 (Whenever this creature blocks or becomes blocked, it gets +1/+1 until end of turn.)",
            )
            .expect("parse bushido keyword");

    let triggered = def
        .abilities
        .iter()
        .find_map(|a| match &a.kind {
            AbilityKind::Triggered(t) => Some(t),
            _ => None,
        })
        .expect("expected bushido triggered ability");

    assert_eq!(def.abilities.len(), 1);
    assert_eq!(def.abilities[0].text.as_deref(), Some("Bushido 1"));
    assert!(triggered.trigger.display().contains("blocks"));
    assert!(
        format!("{:?}", triggered.effects[0]).contains("ModifyPowerToughnessEffect"),
        "expected pump effect, got {:?}",
        triggered.effects
    );

    let lines = compiled_lines(&def);
    assert!(
        lines
            .iter()
            .any(|line| line == "Keyword ability 1: Bushido 1"),
        "expected keyword rendering for bushido, got {lines:?}"
    );
}

#[test]
fn test_parse_bloodthirst_keyword_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Bloodthirst Creature")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Bloodthirst 2 (If an opponent was dealt damage this turn, this creature enters with two +1/+1 counters on it.)",
        )
        .expect("parse bloodthirst keyword");

    let static_ability = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability),
            _ => None,
        })
        .expect("expected bloodthirst static ability");

    assert_eq!(def.abilities[0].text.as_deref(), Some("Bloodthirst 2"));
    assert_eq!(
        static_ability.id(),
        crate::static_abilities::StaticAbilityId::Bloodthirst
    );

    let lines = compiled_lines(&def);
    assert!(
        lines
            .iter()
            .any(|line| line == "Static ability 1: Bloodthirst 2"),
        "expected static ability rendering for bloodthirst, got {lines:?}"
    );
}

#[test]
fn test_parse_rampage_keyword_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Rampage Creature")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Rampage 2 (Whenever this creature becomes blocked, it gets +2/+2 until end of turn for each creature blocking it beyond the first.)",
        )
        .expect("parse rampage keyword");

    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("expected rampage triggered ability");

    assert_eq!(def.abilities[0].text.as_deref(), Some("Rampage 2"));
    assert!(
        triggered.trigger.display().contains("becomes blocked"),
        "expected becomes-blocked trigger, got {}",
        triggered.trigger.display()
    );
    let effect_debug = format!("{:?}", triggered.effects);
    assert!(
        effect_debug.contains("BlockersBeyondFirst") && effect_debug.contains("multiplier: 2"),
        "expected blocker-count event value in rampage effect, got {effect_debug}"
    );

    let lines = compiled_lines(&def);
    assert!(
        lines.iter().any(
            |line| line.contains("Triggered ability 1: Whenever this creature becomes blocked")
        ),
        "expected triggered rendering for rampage, got {lines:?}"
    );
}

#[test]
fn test_parse_exalted_keyword_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Exalted Creature")
        .card_types(vec![CardType::Creature])
        .parse_text("Exalted")
        .expect("parse exalted keyword");

    let triggered = def
        .abilities
        .iter()
        .find_map(|a| match &a.kind {
            AbilityKind::Triggered(t) => Some(t),
            _ => None,
        })
        .expect("expected exalted triggered ability");

    assert_eq!(def.abilities[0].text.as_deref(), Some("Exalted"));
    assert!(triggered.trigger.display().contains("attacks alone"));
    assert_eq!(triggered.effects.len(), 2);

    let lines = compiled_lines(&def);
    assert!(
        lines
            .iter()
            .any(|line| line == "Keyword ability 1: Exalted"),
        "expected keyword rendering for exalted, got {lines:?}"
    );
}

#[test]
fn test_parse_persist_keyword_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Persist Creature")
        .card_types(vec![CardType::Creature])
        .parse_text("Persist")
        .expect("parse persist keyword");

    assert_eq!(def.abilities[0].text.as_deref(), Some("Persist"));
    let lines = compiled_lines(&def);
    assert!(
        lines
            .iter()
            .any(|line| line == "Keyword ability 1: Persist"),
        "expected keyword rendering for persist, got {lines:?}"
    );
}

#[test]
fn test_parse_undying_keyword_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Undying Creature")
        .card_types(vec![CardType::Creature])
        .parse_text("Undying")
        .expect("parse undying keyword");

    assert_eq!(def.abilities[0].text.as_deref(), Some("Undying"));
    let lines = compiled_lines(&def);
    assert!(
        lines
            .iter()
            .any(|line| line == "Keyword ability 1: Undying"),
        "expected keyword rendering for undying, got {lines:?}"
    );
}

#[test]
fn test_parse_toxic_keyword_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Toxic Creature")
        .card_types(vec![CardType::Creature])
        .parse_text("Toxic 2")
        .expect("parse toxic keyword");

    let triggered = def
        .abilities
        .iter()
        .find_map(|a| match &a.kind {
            AbilityKind::Triggered(t) => Some(t),
            _ => None,
        })
        .expect("expected toxic triggered ability");

    assert_eq!(def.abilities[0].text.as_deref(), Some("Toxic 2"));
    assert!(
        triggered
            .trigger
            .display()
            .contains("combat damage to a player")
    );
    assert!(
        format!("{:?}", triggered.effects[0]).contains("PoisonCountersEffect"),
        "expected poison counters effect, got {:?}",
        triggered.effects
    );
    let lines = compiled_lines(&def);
    assert!(
        lines
            .iter()
            .any(|line| line == "Keyword ability 1: Toxic 2"),
        "expected toxic keyword rendering, got {lines:?}"
    );
}

#[test]
fn test_parse_storm_keyword_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Storm Spell")
        .card_types(vec![CardType::Instant])
        .parse_text("Storm")
        .expect("parse storm keyword");

    let triggered = def
        .abilities
        .iter()
        .find_map(|a| match &a.kind {
            AbilityKind::Triggered(t) => Some(t),
            _ => None,
        })
        .expect("expected storm triggered ability");

    assert_eq!(def.abilities[0].text.as_deref(), Some("Storm"));
    assert_eq!(def.abilities[0].functional_zones, vec![Zone::Stack]);
    assert!(triggered.trigger.display().contains("cast this spell"));
    assert!(
        format!("{:?}", triggered.effects[0]).contains("CopySpellEffect"),
        "expected copy spell effect wrapper, got {:?}",
        triggered.effects
    );
    assert!(
        format!("{:?}", triggered.effects[1]).contains("ChooseNewTargetsEffect"),
        "expected choose-new-targets effect, got {:?}",
        triggered.effects
    );
    let lines = compiled_lines(&def);
    assert!(
        lines.iter().any(|line| line == "Keyword ability 1: Storm"),
        "expected storm keyword rendering, got {lines:?}"
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
        rendered.contains("unleash"),
        "expected unleash marker text in compiled output, got {rendered}"
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
        rendered.contains("fabricate 1"),
        "expected fabricate parameter in render output, got {rendered}"
    );
}

#[test]
fn oracle_like_merges_adjacent_keyword_lines_into_comma_list() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Keyword Merge Probe")
        .card_types(vec![CardType::Creature])
        .parse_text("Flying, vigilance")
        .expect("keyword list should parse");

    let lines = oracle_like_lines(&def);
    assert!(
        lines.iter().any(|line| line == "Flying, vigilance"),
        "expected merged keyword list rendering, got {lines:?}"
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
fn test_parse_training_keyword_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Training Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Training (Whenever this creature attacks with another creature with greater power, put a +1/+1 counter on this creature.)",
        )
        .expect("training line should parse as marker keyword");

    let rendered = oracle_like_lines(&def).join(" ");
    assert!(
        rendered.contains("Training"),
        "expected training marker in render output, got {rendered}"
    );
}

#[test]
fn test_parse_vanishing_keyword_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Vanishing Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Vanishing 3 (This creature enters with three time counters on it. At the beginning of your upkeep, remove a time counter from it. When the last is removed, sacrifice it.)",
        )
        .expect("vanishing line should parse as marker keyword");

    let rendered = oracle_like_lines(&def).join(" ");
    assert!(
        rendered.contains("Vanishing 3"),
        "expected vanishing marker in render output, got {rendered}"
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
        .expect("unearth keyword line should parse as marker keyword");

    let rendered = oracle_like_lines(&def).join(" ");
    assert!(
        rendered.contains("Unearth {U}"),
        "expected unearth marker in render output, got {rendered}"
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
fn test_parse_suspend_keyword_line_with_reminder_text_keeps_suspend_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Suspend Probe")
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "Suspend 3—{0} (Rather than cast this card from your hand, pay {0} and exile it with three time counters on it. At the beginning of your upkeep, remove a time counter. When the last is removed, you may cast it without paying its mana cost.)",
        )
        .expect("suspend keyword with reminder text should parse");

    let rendered = oracle_like_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("suspend 3 {0}"),
        "expected suspend keyword text in oracle-like output, got {rendered}"
    );
    assert!(
        !rendered.contains("unsupported parser line fallback"),
        "suspend keyword line should not rely on unsupported fallback marker: {rendered}"
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
        rendered.contains("counter target activated or triggered ability"),
        "expected counter-ability text in oracle-like output, got {rendered}"
    );
    assert!(
        !rendered.contains("unsupported parser line fallback"),
        "counter-ability clause should not rely on unsupported fallback marker: {rendered}"
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
        rendered.contains("graveyard"),
        "expected graveyard wording in rendered text, got {rendered}"
    );
    assert!(
        !rendered.contains("unsupported parser line fallback"),
        "single-graveyard exile clause should not rely on unsupported fallback marker: {rendered}"
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
fn test_parse_quoted_granted_ability_marker_errors() {
    let err = CardDefinitionBuilder::new(CardId::from_raw(1), "Quoted Grant Probe")
        .card_types(vec![CardType::Enchantment])
        .parse_text("Other permanents you control have \"{T}: Add one mana of any color.\"")
        .expect_err("quoted granted-ability marker should fail parsing");
    let message = format!("{err:?}").to_ascii_lowercase();
    assert!(
        message.contains("unsupported quoted granted-ability clause"),
        "expected explicit unsupported quoted-grant marker error, got {message}"
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
fn test_parse_modal_choose_one_header_without_fallback() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Modal Header Probe")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Choose one -\n\
- Target opponent exiles a creature they control.\n\
- Target opponent exiles an enchantment they control.",
        )
        .expect("parse choose-one modal spell");

    let rendered = compiled_lines(&def).join(" | ").to_ascii_lowercase();
    assert!(
        rendered.contains("choose one -"),
        "expected choose-one rendering, got {rendered}"
    );
    assert!(
        !rendered.contains("unsupported parser line fallback"),
        "choose-one modal header should not hit fallback, got {rendered}"
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
fn test_parse_level_up_keyword_line_keeps_cost_in_render() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Level-Up Creature")
        .card_types(vec![CardType::Creature])
        .parse_text("Level up {2}{G}")
        .expect("parse level up keyword");

    assert_eq!(def.abilities[0].text.as_deref(), Some("Level up {2}{G}"));
    let lines = compiled_lines(&def);
    assert!(
        lines
            .iter()
            .any(|line| line == "Keyword ability 1: Level up {2}{G}"),
        "expected level-up keyword rendering with cost, got {lines:?}"
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
fn test_parse_trigger_this_or_another_enchantment_enters() {
    let tokens = tokenize_line("this creature or another enchantment you control enters", 0);
    let trigger = parse_trigger_clause(&tokens).expect("parse constellation-style trigger");
    match trigger {
        TriggerSpec::Either(left, right) => {
            assert!(
                matches!(*left, TriggerSpec::ThisEntersBattlefield),
                "expected left branch to be this-enters trigger, got {left:?}"
            );
            match *right {
                TriggerSpec::EntersBattlefield(filter) => {
                    assert!(
                        filter.card_types.contains(&CardType::Enchantment),
                        "expected enchantment filter, got {filter:?}"
                    );
                    assert_eq!(
                        filter.controller,
                        Some(PlayerFilter::You),
                        "expected you-control filter, got {filter:?}"
                    );
                    assert!(filter.other, "expected 'another' filter, got {filter:?}");
                }
                other => panic!("expected enters-battlefield branch, got {other:?}"),
            }
        }
        other => panic!("expected Either trigger, got {other:?}"),
    }
}

#[test]
fn test_parse_conditional_anthem_and_haste_keeps_pump_and_keyword() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Conditional Haste Variant")
            .card_types(vec![CardType::Creature])
            .parse_text(
                "As long as you control another multicolored permanent, this creature gets +1/+1 and has haste.",
            )
            .expect("parse conditional anthem and haste");

    let lines = compiled_lines(&def);
    assert!(
        lines.iter().any(|line| line.contains("gets +1/+1")),
        "expected self buff line, got {lines:?}"
    );
    assert!(
        lines
            .iter()
            .any(|line| line.contains("has Haste") && line.contains("multicolored permanent")),
        "expected conditional haste line, got {lines:?}"
    );
}

#[test]
fn test_parse_composed_anthems_keep_independent_land_conditions() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Tek Variant")
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .parse_text(
            "This creature gets +0/+2 as long as you control a Plains, has flying as long as you control an Island, gets +2/+0 as long as you control a Swamp, has first strike as long as you control a Mountain, and has trample as long as you control a Forest.",
        )
        .expect("parse composed anthem line");

    let static_abilities = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        static_abilities.len(),
        5,
        "expected five independent static abilities, got {static_abilities:?}"
    );

    let anthem_count = static_abilities
        .iter()
        .filter(|ability| ability.id() == StaticAbilityId::Anthem)
        .count();
    let grant_count = static_abilities
        .iter()
        .filter(|ability| ability.id() == StaticAbilityId::GrantAbility)
        .count();
    assert_eq!(anthem_count, 2, "expected two P/T anthem effects");
    assert_eq!(grant_count, 3, "expected three keyword-grant effects");

    let debug = format!("{:?}", static_abilities);
    for land in ["Plains", "Island", "Swamp", "Mountain", "Forest"] {
        assert!(
            debug.contains(&format!("subtypes: [{land}]")),
            "expected independent condition for {land}, got {debug}"
        );
    }
    assert!(
        !debug.contains("subtypes: [Plains, Island"),
        "composed static conditions must not collapse into one combined subtype filter: {debug}"
    );
}

#[test]
fn test_parse_granted_keyword_list_handles_leading_and_protection_chain() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Akroma's Memorial Variant")
            .card_types(vec![CardType::Artifact])
            .parse_text(
                "Creatures you control have flying, first strike, vigilance, trample, haste, and protection from black and from red.",
            )
            .expect("parse granted keyword list");

    let lines = compiled_lines(&def);
    assert!(
        lines
            .iter()
            .any(|line| line.contains("Protection from black")),
        "expected black protection grant, got {lines:?}"
    );
    assert!(
        lines
            .iter()
            .any(|line| line.contains("Protection from red")),
        "expected red protection grant, got {lines:?}"
    );
}

#[test]
fn test_parse_granted_keyword_and_must_attack_keeps_both_parts() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Hellraiser Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Creatures you control have haste and attack each combat if able.")
        .expect("parse granted keyword + must-attack");

    let lines = compiled_lines(&def);
    assert!(
        lines.iter().any(|line| line.contains("have Haste")),
        "expected haste grant line, got {lines:?}"
    );
    assert!(
        lines
            .iter()
            .any(|line| line.contains("Attacks each combat if able")),
        "expected must-attack grant line, got {lines:?}"
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

    let effects = def.spell_effect.expect("spell effects");
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
fn parse_same_name_return_to_hand_keeps_zone_tail() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Same Name Return Variant")
            .parse_text(
                "Return target creature card and all other cards with the same name as that card from your graveyard to your hand.",
            )
            .expect("parse same-name return sentence");

    let effects = def.spell_effect.expect("expected spell effects");
    let debug = format!("{effects:?}");
    assert!(
        debug.contains("ReturnToHandEffect") && debug.matches("ReturnToHandEffect").count() >= 2,
        "expected target return plus fanout return, got {debug}"
    );
    assert!(
        debug.contains("zone: Some(Graveyard)"),
        "expected graveyard zone tail to remain in fanout filter, got {debug}"
    );
    assert!(
        debug.contains("SameNameAsTagged"),
        "expected same-name tagged relation in fanout filter, got {debug}"
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
    let def = CardDefinitionBuilder::new(CardId::new(), "Catalyst Stone Variant")
        .parse_text(
            "Flashback costs you pay cost {2} less.\nFlashback costs your opponents pay cost {2} more.",
        )
        .expect("flashback cost-modifier lines should parse");
    let rendered = compiled_lines(&def).join("\n").to_ascii_lowercase();
    assert!(
        rendered.contains("flashback costs you pay cost {2} less"),
        "expected self flashback cost reduction rendering, got {rendered}"
    );
    assert!(
        rendered.contains("flashback costs your opponents pay cost {2} more"),
        "expected opponent flashback cost increase rendering, got {rendered}"
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
fn parse_flashback_with_life_cost_stays_keyword_not_spell_effect() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Deep Analysis Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Target player draws two cards.\nFlashback—{1}{U}, Pay 3 life.")
        .expect("flashback line with life cost should parse as keyword marker");
    let rendered = compiled_lines(&def).join("\n").to_ascii_lowercase();
    assert!(
        rendered.contains("keyword ability") && rendered.contains("flashback {1}{u}, pay 3 life"),
        "expected flashback keyword ability line with life payment, got {rendered}"
    );
    assert!(
        !rendered.contains("you lose 3 life"),
        "flashback cost should not compile as unconditional life-loss effect: {rendered}"
    );
}

#[test]
fn parse_mana_spend_trigger_clause_is_split_from_mana_production() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Scaled Nurturer Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "{T}: Add {G}. When you spend this mana to cast a Dragon creature spell, you gain 2 life.",
        )
        .expect("mana-spend rider line should parse without unconditional life gain");
    let rendered = compiled_lines(&def).join("\n").to_ascii_lowercase();
    assert!(
        rendered.contains("mana ability") && rendered.contains("{t}: add {g}"),
        "expected base mana ability text, got {rendered}"
    );
    assert!(
        rendered.contains("you spend this mana to cast a dragon creature spell"),
        "expected separate mana-spend trigger clause text, got {rendered}"
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
fn parse_its_owner_gains_life_uses_owner_of_target_player_filter() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Misfortune's Gain Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Destroy target creature. Its owner gains 4 life.")
        .expect("its-owner gain clause should parse");
    let rendered = compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Its owner gains 4 life"),
        "expected owner-scoped life gain wording, got {rendered}"
    );
    let effects = def
        .spell_effect
        .as_ref()
        .expect("spell should have compiled spell effects");
    let gain = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::GainLifeEffect>())
        .expect("expected a gain life effect");
    assert_eq!(
        gain.player,
        ChooseSpec::Player(PlayerFilter::OwnerOf(ObjectRef::Target))
    );
}

#[test]
fn render_instant_and_sorcery_self_reference_as_this_spell() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Self Exile Spell Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Exile this spell.")
        .expect("spell self-reference should parse");
    let rendered = compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Exile this spell"),
        "expected spell self-reference wording, got {rendered}"
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
        rendered.contains("Exile target artifact card from a graveyard"),
        "expected from-a-graveyard wording, got {rendered}"
    );
    assert!(
        !rendered.contains("in graveyard"),
        "graveyard wording should not use in-graveyard phrasing: {rendered}"
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
fn parse_destroy_target_one_or_more_colors_fails_instead_of_broadening_target() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Reach of Shadows Variant")
        .parse_text("Destroy target creature that's one or more colors.")
        .expect_err("one-or-more-colors target should fail loudly when unsupported");
    let message = format!("{err:?}");
    assert!(
        message.contains("unsupported color-count object filter"),
        "expected color-count filter parse error, got {message}"
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
        message.contains("unsupported face-down/manifest exile clause"),
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
        message.contains("unsupported sacrifice-unless clause"),
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
fn render_exile_all_graveyards_uses_oracle_wording() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Morningtide Variant")
        .parse_text("Exile all graveyards.")
        .expect("exile all graveyards should parse");
    let joined = crate::compiled_text::oracle_like_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("exile all graveyards"),
        "expected oracle-like graveyard wording, got {joined}"
    );
}

#[test]
fn oracle_like_preserves_another_on_enter_trigger_subject() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Alliance Variant")
        .parse_text("Whenever another creature enters under your control, you gain 1 life.")
        .expect("another-creature enter trigger should parse");
    let joined = crate::compiled_text::oracle_like_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("another creature you control enters"),
        "expected another-creature trigger wording, got {joined}"
    );
}

#[test]
fn render_draw_then_proliferate_uses_oracle_like_then_wording() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Tezzeret's Gambit Variant")
        .parse_text("Draw two cards, then proliferate.")
        .expect("draw-then-proliferate should parse");
    let joined = crate::compiled_text::oracle_like_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("draw 2 cards, then proliferate"),
        "expected draw-then-proliferate wording, got {joined}"
    );
}

#[test]
fn render_draw_then_sacrifice_permanent_uses_oracle_like_then_wording() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Perilous Research Variant")
        .parse_text("Draw two cards, then sacrifice a permanent.")
        .expect("draw-then-sacrifice should parse");
    let joined = crate::compiled_text::oracle_like_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("draw 2 cards, then sacrifice a permanent"),
        "expected draw-then-sacrifice wording, got {joined}"
    );
}

#[test]
fn parse_sacrifice_then_lose_life_carries_target_player_to_second_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Geth's Verdict Variant")
        .parse_text("Target player sacrifices a creature of their choice and loses 1 life.")
        .expect("sacrifice-then-lose line should parse");
    let joined = crate::compiled_text::compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("target player loses 1 life"),
        "expected carried target player for lose-life clause, got {joined}"
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
        joined.contains("target opponent discards a card"),
        "expected discard clause in chain, got {joined}"
    );
    assert!(
        joined.contains("target opponent loses 3 life"),
        "expected life-loss clause in chain, got {joined}"
    );
}

#[test]
fn parse_each_opponent_sacrifice_discard_lose_chain_keeps_all_predicates() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Dusk Mangler Chain Variant")
        .parse_text(
            "When this creature enters, each opponent sacrifices a creature of their choice, discards a card, and loses 4 life.",
        )
        .expect("each-opponent sacrifice/discard/lose chain should parse");
    let joined = crate::compiled_text::compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("each opponent sacrifices"),
        "expected sacrifice clause in chain, got {joined}"
    );
    assert!(
        joined.contains("each opponent discards a card"),
        "expected discard clause in chain, got {joined}"
    );
    assert!(
        joined.contains("each opponent loses 4 life"),
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
        .parse_text("When this creature enters, target opponent sacrifices a creature of their choice.")
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
        .parse_text("All Goblins are black and are Zombies in addition to their other creature types.")
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
fn parse_granted_activated_ability_to_non_source_fails_strictly() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Quicksmith Rebel Variant")
            .card_types(vec![CardType::Creature])
            .parse_text(
                "When this creature enters, target artifact you control gains \"{T}: This artifact deals 2 damage to any target\" for as long as you control this creature.",
            )
            .expect_err("unsupported non-source granted activated ability should fail parse");
    let message = format!("{err:?}");
    assert!(
        message.contains("missing life gain amount")
            || message.contains("unsupported granted activated/triggered ability clause"),
        "expected strict failure for unsupported granted activated ability target, got {message}"
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
fn parse_enters_tapped_with_choose_color_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Thriving Variant")
        .card_types(vec![CardType::Land])
        .parse_text("This land enters tapped. As it enters, choose a color other than black.")
        .expect("choose-color clause should parse");
    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::EntersTapped),
        "expected enters-tapped ability, got {ids:?}"
    );
    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::ChooseColorAsEnters),
        "expected choose-color ability, got {ids:?}"
    );
    let compiled = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        compiled.contains("this enters tapped"),
        "expected enters-tapped in compiled text, got {compiled}"
    );
    assert!(
        compiled.contains("as it enters, choose a color other than black"),
        "expected choose-color clause in compiled text, got {compiled}"
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
fn parse_add_mana_or_colors_preserves_choice() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Dual Mana Variant")
        .card_types(vec![CardType::Land])
        .parse_text("{T}: Add {R} or {G}.")
        .expect("add mana or-colors should parse");

    let lines = compiled_lines(&def);
    let mana_line = lines
        .iter()
        .find(|line| line.contains("Mana ability"))
        .expect("expected mana ability line");
    assert!(
        mana_line.contains("Add {R} or {G}"),
        "expected or-choice mana render, got {mana_line}"
    );
    assert!(
        !mana_line.contains("{R}{G}"),
        "expected not to add both colors, got {mana_line}"
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
        mana_line.contains("Activate only if you control 3 or more artifacts"),
        "expected metalcraft activation condition in compiled text, got {mana_line}"
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
        mana_line.contains("Activate only if you control 5 or more lands"),
        "expected land-count activation condition in compiled text, got {mana_line}"
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
        mana_line.contains("Activate only if there is an Elf card in your graveyard"),
        "expected graveyard-card activation condition in compiled text, got {mana_line}"
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
        joined.contains("controller gets 1 poison counter"),
        "expected controller-based poison counter wording, got {joined}"
    );
    assert!(
        !joined.contains("you get 1 poison counter"),
        "did not expect implicit-you poison counter wording, got {joined}"
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
fn parse_each_player_discard_then_draw_keeps_each_player_scope() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Wheel Variant")
        .parse_text("Each player discards their hand, then draws seven cards.")
        .expect("each-player discard-then-draw should parse");

    let compiled = compiled_lines(&def).join(" ");
    assert!(
        compiled.contains("Each player discards their hand, then draws 7 cards"),
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
        debug.contains("ForEachObject"),
        "expected ForEachObject lowering, got {debug}"
    );
}

#[test]
fn parse_regenerate_each_creature_lowers_to_all_filter() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Wrap Variant")
        .parse_text("Regenerate each creature you control.")
        .expect("regenerate each creature should parse");

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("RegenerateEffect") && debug.contains("All("),
        "expected regenerate-all lowering, got {debug}"
    );
    let compiled = compiled_lines(&def).join(" ");
    assert!(
        compiled.contains("Regenerate each creature you control"),
        "expected each-creature regenerate rendering, got {compiled}"
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
fn parse_target_creature_attacks_or_blocks_if_able_fails_strictly() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Hustle Variant")
        .parse_text("Target creature attacks or blocks this turn if able.")
        .expect_err("unsupported target-only combat-action clause should fail");
    let message = format!("{err:?}");
    assert!(
        message.contains("unsupported target-only restriction clause"),
        "expected strict target-only restriction error, got {message}"
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
fn parse_recommission_text_fails_instead_of_static_counter_only() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Recommission Variant")
            .parse_text(
                "Return target artifact or creature card with mana value 3 or less from your graveyard to the battlefield. If a creature enters this way, it enters with an additional +1/+1 counter on it.",
            )
            .expect_err("unsupported mixed return+enters-with-counters clause should fail");
    let message = format!("{err:?}");
    assert!(
        !message.is_empty(),
        "expected actionable parse error for mixed return/counter line"
    );
}

#[test]
fn parse_teferis_time_twist_text_fails_instead_of_static_counter_only() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Teferi Time Twist Variant")
            .parse_text(
                "Exile target permanent you control. Return that card to the battlefield under its owner's control at the beginning of the next end step. If it enters as a creature, it enters with an additional +1/+1 counter on it.",
            )
            .expect_err("unsupported mixed exile/return+enters-with-counters clause should fail");
    let message = format!("{err:?}");
    assert!(
        !message.is_empty(),
        "expected actionable parse error for mixed exile/return/counter line"
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
        ids.contains(&StaticAbilityId::AttachedAbilityGrant),
        "expected attached ability grant static ability, got {ids:?}"
    );

    let compiled = compiled_lines(&def).join("\n").to_ascii_lowercase();
    assert!(
        compiled.contains("enchanted creature doesnt untap during its controllers untap step"),
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
            ids.contains(
                &crate::static_abilities::StaticAbilityId::CantAttackUnlessDefendingPlayerControlsLandSubtype
            ),
            "expected defending-player-land-subtype attack restriction, got {ids:?}"
        );

    let compiled = crate::compiled_text::compiled_lines(&def).join("\n");
    assert!(
        compiled
            .to_ascii_lowercase()
            .contains("defending player controls island"),
        "expected compiled text to include defending-player island condition, got {compiled}"
    );
}

#[test]
fn parse_self_damaging_any_target_clause_with_shared_deal_verb() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Pinger Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("{T}: This creature deals 1 damage to any target and 1 damage to you.")
        .expect("shared-verb damage clause should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("deal 1 damage to any target"),
        "expected first damage target in compiled output, got {rendered}"
    );
    assert!(
        rendered.contains("deal 1 damage to you"),
        "expected self-damage follow-up in compiled output, got {rendered}"
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
fn parse_haunt_keyword_line_errors() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Haunt Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Haunt")
        .expect_err("haunt keyword line should fail parsing");
    let message = format!("{err:?}").to_ascii_lowercase();
    assert!(
        message.contains("unsupported keyword mechanic 'haunt'"),
        "expected explicit unsupported haunt error, got {message}"
    );
}

#[test]
fn parse_filter_power_numeric_comparison_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Power Filter Variant")
        .parse_text("Destroy target creature with power 2 or less.")
        .expect("numeric power comparison should parse");

    let effects = def.spell_effect.expect("spell effects");
    let debug = format!("{effects:?}");
    assert!(
        debug.contains("power: Some(LessThanOrEqual(2))"),
        "expected parsed power comparison constraint, got {debug}"
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
fn parse_destroy_unless_any_player_fails_instead_of_partial_destroy() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Destroy Unless Variant")
        .parse_text("Destroy target permanent unless any player pays 1 life.")
        .expect_err("unsupported destroy-unless should fail parse");
    let message = format!("{err:?}");
    assert!(
        message.contains("unsupported destroy-unless clause"),
        "expected strict destroy-unless parse error, got {message}"
    );
}

#[test]
fn parse_return_up_to_x_target_fails_instead_of_partial_return() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Dynamic Return Count Variant")
            .parse_text(
                "Return up to X target creatures to their owners' hands, where X is one plus the number of cards named Aether Burst in all graveyards as you cast this spell.",
            )
            .expect_err("unsupported dynamic target count should fail parse");
    let message = format!("{err:?}");
    assert!(
        message.contains("unsupported dynamic or missing target count after 'up to'")
            || message.contains("unsupported dynamic target count 'X target'"),
        "expected strict dynamic target-count parse error, got {message}"
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
        message.contains("unsupported multi-target destroy clause"),
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
fn parse_combat_damage_to_creature_trigger_fails_instead_of_broad_damage_trigger() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Combat Damage Creature Trigger Variant")
            .parse_text(
                "Whenever this creature deals combat damage to a creature, you gain 2 life unless that creature's controller pays {2}.",
            )
            .expect_err("unsupported combat-damage-to-creature trigger should fail parse");
    let message = format!("{err:?}");
    assert!(
        message.contains("unsupported combat-damage-to-creature trigger clause"),
        "expected strict combat-damage-to-creature trigger parse error, got {message}"
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
        message.contains("unsupported transformed return clause"),
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
        message.contains("unsupported delayed return timing clause"),
        "expected strict delayed-return parse error, got {message}"
    );
}

#[test]
fn parse_exile_name_and_target_fails_instead_of_exiling_only_target() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Mangara Variant")
        .parse_text("{T}: Exile Mangara of Corondor and target permanent.")
        .expect_err("unsupported multi-target exile should fail parse");
    let message = format!("{err:?}");
    assert!(
        message.contains("unsupported multi-target exile clause"),
        "expected strict multi-target exile parse error, got {message}"
    );
}

#[test]
fn parse_target_opponent_exiles_card_from_their_hand_uses_hand_choice() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Skullcap Snail Variant")
        .parse_text("Target opponent exiles a card from their hand.")
        .expect("parse targeted hand exile");

    let effects = def.spell_effect.expect("spell effects");
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
fn parse_triggered_sacrifice_this_then_destroy_that_creature_uses_triggering_object() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Grave Peril Variant")
            .parse_text("When a nonblack creature enters, sacrifice this enchantment. If you do, destroy that creature.")
            .expect("grave peril style trigger should parse");
    let lines = crate::compiled_text::compiled_lines(&def);
    let joined = lines.join("\n");
    assert!(
        joined.contains("Sacrifice this source"),
        "expected source sacrifice, got {joined}"
    );
    assert!(
        joined.contains("Destroy it"),
        "expected 'that creature' to resolve to triggering object, got {joined}"
    );
    assert!(
        !joined.contains("sacrificed_"),
        "source sacrifice must not retarget 'that creature' to sacrificed object, got {joined}"
    );
}

#[test]
fn parse_triggered_sacrifice_this_then_damage_that_creature_uses_triggering_object() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Flame-Kin War Scout Variant")
            .parse_text(
                "When another creature enters, sacrifice this creature. If you do, this creature deals 4 damage to that creature.",
            )
            .expect("flame-kin war scout style trigger should parse");
    let lines = crate::compiled_text::compiled_lines(&def);
    let joined = lines.join("\n");
    assert!(
        joined.contains("Sacrifice this source"),
        "expected source sacrifice, got {joined}"
    );
    assert!(
        joined.contains("Deal 4 damage to it"),
        "expected damage to triggering object, got {joined}"
    );
    assert!(
        !joined.contains("sacrificed_"),
        "source sacrifice must not retarget 'that creature' to sacrificed object, got {joined}"
    );
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
fn render_create_token_copy_uses_oracle_like_wording() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Myr Propagator Variant")
        .parse_text("{3}, {T}: Create a token that's a copy of this creature.")
        .expect("token copy creation should parse");
    let lines = crate::compiled_text::compiled_lines(&def);
    let joined = lines.join("\n");
    assert!(
        joined.contains("Create a token that's a copy of this source"),
        "expected oracle-like token-copy wording, got {joined}"
    );
}

#[test]
fn render_transform_source_uses_this_creature_wording() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Phyrexian Werewolf Variant")
        .parse_text("{3}{G/P}: Transform this creature. Activate only as a sorcery.")
        .expect("source transform with sorcery-speed rider should parse");
    let lines = crate::compiled_text::compiled_lines(&def);
    let joined = lines.join("\n");
    assert!(
        joined.contains("Transform this creature"),
        "expected transform wording to use 'this creature', got {joined}"
    );
    assert!(
        joined.contains("Activate only as a sorcery"),
        "expected sorcery-speed rider to remain present, got {joined}"
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
        joined.contains("mill 4 cards"),
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
fn oracle_like_lines_compact_when_you_cast_creature_spell() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Skittering Horror Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("When you cast a creature spell, sacrifice this creature.")
        .expect("cast-trigger sacrifice clause should parse");
    let lines = crate::compiled_text::oracle_like_lines(&def);
    let joined = lines.join("\n").to_ascii_lowercase();
    assert!(
        joined.contains("when you cast a creature spell"),
        "expected cast trigger phrase to include creature spell, got {joined}"
    );
    assert!(
        joined.contains("sacrifice this creature"),
        "expected source wording to normalize to creature, got {joined}"
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

    let spell_effects = def.spell_effect.as_ref().expect("expected spell effects");
    let destroy = spell_effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::DestroyEffect>())
        .expect("expected destroy effect");

    let target_filter = match &destroy.spec {
        ChooseSpec::Target(inner) => match inner.as_ref() {
            ChooseSpec::Target(inner_again) => match inner_again.as_ref() {
                ChooseSpec::Object(filter) => filter,
                other => panic!("expected object target for destroy effect, got {:?}", other),
            },
            ChooseSpec::Object(filter) => filter,
            other => panic!("expected object target for destroy effect, got {:?}", other),
        },
        other => panic!("expected targeted destroy effect, got {:?}", other),
    };

    assert!(
        target_filter.type_or_subtype_union,
        "expected type/subtype union for creature-or-vehicle targeting"
    );
    assert!(
        target_filter.card_types.contains(&CardType::Creature),
        "expected creature card type selector, got {:?}",
        target_filter.card_types
    );
    assert!(
        target_filter.subtypes.contains(&Subtype::Vehicle),
        "expected Vehicle subtype selector, got {:?}",
        target_filter.subtypes
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
        lower.contains("tap enchanted creature") && lower.contains("untap enchanted creature"),
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

    let spell_effects = def.spell_effect.as_ref().expect("expected spell effects");
    let move_to_library = spell_effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::MoveToZoneEffect>())
        .expect("expected move-to-library effect");

    assert_eq!(
        move_to_library.zone,
        crate::zone::Zone::Library,
        "expected move destination to be library"
    );
    assert!(
        move_to_library.to_top,
        "expected move destination to be top of library"
    );
    assert_eq!(
        move_to_library.target.count(),
        crate::effect::ChoiceCount::exactly(2),
        "expected exactly two cards to be moved"
    );
    let base = move_to_library.target.base();
    let filter = match base {
        ChooseSpec::Object(filter) => filter,
        other => panic!(
            "expected object filter target for move-to-library, got {:?}",
            other
        ),
    };
    assert_eq!(
        filter.zone,
        Some(crate::zone::Zone::Hand),
        "expected cards to be selected from hand"
    );
    assert_eq!(
        filter.owner,
        Some(crate::target::PlayerFilter::You),
        "expected selected cards to be from your hand"
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
fn render_for_each_object_strips_article() {
    let def = CardDefinitionBuilder::new(CardId::new(), "End Festivities Variant")
        .parse_text(
            "This spell deals 1 damage to each opponent and each creature and planeswalker they control.",
        )
        .expect("end the festivities spell should parse");
    let joined = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains(
            "deal 1 damage to each opponent and each creature and planeswalker they control"
        ) || joined.contains(
            "deal 1 damage to each opponent and each creature or planeswalker they control"
        ),
        "expected compact damage rendering, got {joined}"
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
        rendered.contains("with a +1/+1 counter on it") && rendered.contains("menace"),
        "expected counter-qualified menace grant rendering, got {rendered}"
    );
    assert!(
        !rendered.contains("permanents have menace"),
        "rendering regressed to broad permanent grant: {rendered}"
    );
}

#[test]
fn render_or_tribe_creature_subject_uses_each_creature_phrasing() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Lovisa Variant")
        .parse_text(
            "Each creature that's a Barbarian, a Warrior, or a Berserker gets +2/+2 and has haste.",
        )
        .expect("tribe anthem line should parse");
    let rendered = compiled_lines(&def).join(" ");
    let lower = rendered.to_ascii_lowercase();
    assert!(
        lower.contains("each creature that's a barbarian, a warrior, or a berserker gets +2/+2"),
        "expected normalized tribe subject in rendering, got {rendered}"
    );
    assert!(
        lower.contains("has haste"),
        "expected haste clause in rendering, got {rendered}"
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
fn render_target_creature_pump_uses_oracle_like_wording() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Antagonize Variant")
        .parse_text("Target creature gets +4/+3 until end of turn.")
        .expect("targeted pump spell should parse");
    let lines = crate::compiled_text::compiled_lines(&def);
    let joined = lines.join("\n");
    assert!(
        joined.contains("target creature gets +4/+3 until end of turn"),
        "expected oracle-like targeted pump text, got {joined}"
    );
}

#[test]
fn render_target_creature_pump_and_trample_uses_oracle_like_wording() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Awaken the Bear Variant")
        .parse_text("Target creature gets +3/+3 and gains trample until end of turn.")
        .expect("pump + trample spell should parse");
    let lines = crate::compiled_text::compiled_lines(&def);
    let joined = lines.join("\n");
    assert!(
        joined.contains("target creature gets +3/+3 and gains Trample until end of turn"),
        "expected oracle-like pump + trample rendering, got {joined}"
    );
}

#[test]
fn render_pump_all_hides_internal_tag_wrapper() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Charge Variant")
        .parse_text("Creatures you control get +1/+1 until end of turn.")
        .expect("team pump spell should parse");
    let lines = crate::compiled_text::compiled_lines(&def);
    let joined = lines.join("\n");
    assert!(
        joined.contains("creatures you control get +1/+1 until end of turn"),
        "expected oracle-like team pump rendering, got {joined}"
    );
    assert!(
        !joined.contains("Tag all affected objects"),
        "internal tag wrappers should not leak into compiled text: {joined}"
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
fn render_tapped_token_from_create_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Illustrious Historian Variant")
        .parse_text("Create a tapped 3/2 red and white Spirit creature token.")
        .expect("tapped token creation should parse");
    let joined = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        (joined.contains("create 1 3/2 red and white spirit creature token")
            || joined.contains("create 1 3/2 white and red spirit creature token"))
            && joined.contains("tapped"),
        "expected tapped token rendering, got {joined}"
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
fn render_equipped_gets_and_has_line_as_static() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Spare Dagger Variant")
        .parse_text(
            "Equipped creature gets +1/+0 and has \"Whenever this creature attacks, you may sacrifice this artifact. When you do, this creature deals 1 damage to any target.\"",
        )
        .expect("equipped creature gets/has line should parse");
    let joined = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains("equipped creature gets +1/+0") && joined.contains("equipped creature has"),
        "expected equipped gets/has rendering, got {joined}"
    );
}

#[test]
fn oracle_like_evaporate_uses_each_and_or() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Evaporate")
        .parse_text("Evaporate deals 1 damage to each white and/or blue creature.")
        .expect("evaporate line should parse");
    let lines = oracle_like_lines(&def).join(" ");
    assert!(
        lines.contains("Deal 1 damage to each white and/or blue creature"),
        "expected and/or rendering for dual-color each-target clause, got {lines}"
    );
}

#[test]
fn oracle_like_destroy_color_pair_uses_oracle_order() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Deathmark")
        .parse_text("Destroy target green or white creature.")
        .expect("deathmark line should parse");
    let lines = oracle_like_lines(&def).join(" ");
    assert!(
        lines.contains("Destroy target green or white creature"),
        "expected green-or-white ordering, got {lines}"
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
fn parse_rejects_sacrifice_any_number_of_artifacts_creatures_lands_clause() {
    assert_partial_parse_rejected(
        "Reprocess Variant",
        "Sacrifice any number of artifacts, creatures, and/or lands. Draw a card for each permanent sacrificed this way.",
    );
}

#[test]
fn parse_rejects_counter_all_other_spells_clause() {
    assert_partial_parse_rejected(
        "Swift Silence Variant",
        "Counter all other spells. Draw a card for each spell countered this way.",
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
fn render_powerstone_token_name() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Powerstone Variant")
        .parse_text("Create a tapped Powerstone token.")
        .expect("powerstone token clause should parse");
    let joined = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains("powerstone artifact token") && joined.contains("tapped"),
        "expected powerstone token name in compiled text, got {joined}"
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
fn parse_damage_to_target_opponent_or_planeswalker() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Searing Flesh Variant")
        .parse_text("This spell deals 7 damage to target opponent or planeswalker.")
        .expect("opponent-or-planeswalker damage target should parse");
    let joined = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains("deals 7 damage to target opponent or planeswalker"),
        "expected opponent-or-planeswalker targeting in compiled text, got {joined}"
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
fn parse_alternative_cost_with_return_to_hand_segment() {
    CardDefinitionBuilder::new(CardId::new(), "Borderpost Variant")
        .parse_text("You may pay {1} and return a basic land you control to its owner's hand rather than pay this spell's mana cost.")
        .expect("alternative cost with return-to-hand segment should parse");
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
}

#[test]
fn parse_monocolored_cant_block_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Monocolored Block Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Monocolored creatures can't block this turn.")
        .expect("monocolored block restriction should parse");
    let compiled = compiled_lines(&def).join(" ");
    assert!(
        compiled
            .to_ascii_lowercase()
            .contains("monocolored creatures can't block this turn"),
        "expected monocolored block restriction rendering, got {compiled}"
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
    assert!(
        lower.contains("activate only as a sorcery"),
        "expected token activated ability reminder in compiled text, got {compiled}"
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
        lower.contains("sacrifice this token")
            && lower.contains("return a card named deathpact angel from your graveyard to the battlefield"),
        "expected preserved return-from-graveyard token activation, got {compiled}"
    );
}

#[test]
fn parse_rekindling_style_token_upkeep_trigger_is_preserved() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Rekindling Phoenix")
        .card_types(vec![CardType::Creature])
        .parse_text("When this creature dies, create a 0/1 red Elemental creature token with \"At the beginning of your upkeep, sacrifice this token and return target card named Rekindling Phoenix from your graveyard to the battlefield. It gains haste until end of turn.\"")
        .expect("rekindling-style token trigger should parse");
    let compiled = compiled_lines(&def).join(" ");
    let lower = compiled.to_ascii_lowercase();
    assert!(
        lower.contains("at the beginning of your upkeep, sacrifice this token")
            && lower.contains("return target card named rekindling phoenix from your graveyard to the battlefield")
            && lower.contains("gains haste until end of turn"),
        "expected preserved upkeep return trigger on token, got {compiled}"
    );
}

#[test]
fn parse_nesting_dragon_egg_token_death_trigger_is_preserved() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Dragon Egg Token Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("When this creature enters, create a 0/2 red Dragon Egg creature token with defender and \"When this token dies, create a 2/2 red Dragon creature token with flying and '{R}: This token gets +1/+0 until end of turn.'\"")
        .expect("dragon egg token death trigger should parse");
    let compiled = compiled_lines(&def).join(" ");
    let lower = compiled.to_ascii_lowercase();
    assert!(
        lower.contains("dragon egg creature token"),
        "expected dragon egg subtype in token description, got {compiled}"
    );
    assert!(
        lower.contains("when this token dies, create")
            && lower.contains("2/2 red dragon creature token with flying")
            && lower.contains("gets +1/+0 until end of turn"),
        "expected preserved nested dragon-token death trigger, got {compiled}"
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
    let rendered = oracle_like_lines(&def).join(" ");
    assert!(
        rendered.contains("Activate only if you control a Swamp"),
        "expected subtype activation condition in rendering, got {rendered}"
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
    CardDefinitionBuilder::new(CardId::new(), "Depletion Variant")
        .parse_text("This land enters tapped with two depletion counters on it.")
        .expect("depletion counter line should parse");
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
        ids.contains(&crate::static_abilities::StaticAbilityId::DoesntUntap),
        "expected doesnt-untap static ability, got {ids:?}"
    );
}

#[test]
fn reject_singleton_partial_parse_clauses_030() {
    assert_partial_parse_rejected(
        "Kwain Variant",
        "{T}: Each player may draw a card, then each player who drew a card this way gains 1 life.",
    );
    assert_partial_parse_rejected(
        "Unified Will Variant",
        "Counter target spell if you control more creatures than that spell's controller.",
    );
    assert_partial_parse_rejected(
        "Forbidden Friendship Variant",
        "Create a 1/1 red Dinosaur creature token with haste and a 1/1 white Human Soldier creature token.",
    );
    assert_partial_parse_rejected(
        "Heat Shimmer Variant",
        "Create a token that's a copy of target creature, except it has haste and \"At the beginning of the end step, exile this token.\"",
    );
    assert_partial_parse_rejected(
        "Dream Cache Variant",
        "Draw three cards, then put two cards from your hand both on top of your library or both on the bottom of your library.",
    );
    assert_partial_parse_rejected(
        "Dark Deal Variant",
        "Each player discards all the cards in their hand, then draws that many cards minus one.",
    );
    assert_partial_parse_rejected(
        "Kjeldoran Home Guard Variant",
        "At end of combat, if this creature attacked or blocked this combat, put a -0/-1 counter on this creature and create a 0/1 white Deserter creature token.",
    );
    assert_partial_parse_rejected(
        "Unified Strike Variant",
        "Exile target attacking creature if its power is less than or equal to the number of Soldiers on the battlefield.",
    );
    assert_partial_parse_rejected(
        "Time to Reflect Variant",
        "Exile target creature that blocked or was blocked by a Zombie this turn.",
    );
    assert_partial_parse_rejected(
        "Flame Wave Variant",
        "Flame Wave deals 4 damage to target player or planeswalker and each creature that player or that planeswalker's controller controls.",
    );
    assert_partial_parse_rejected(
        "Arena Athlete Variant",
        "Heroic — Whenever you cast a spell that targets this creature, target creature an opponent controls can't block this turn.",
    );
    assert_partial_parse_rejected(
        "Chocobo Racetrack Variant",
        "Landfall — Whenever a land you control enters, create a 2/2 green Bird creature token with \"Whenever a land you control enters, this token gets +1/+0 until end of turn.\"",
    );
    assert_partial_parse_rejected("Putrid Raptor Variant", "Morph—Discard a Zombie card.");
    assert_partial_parse_rejected(
        "Hubris Variant",
        "Return target creature and all Auras attached to it to their owners' hands.",
    );
    assert_partial_parse_rejected(
        "Word of Undoing Variant",
        "Return target creature and all white Auras you own attached to it to their owners' hands.",
    );
    assert_partial_parse_rejected(
        "Hellion Eruption Variant",
        "Sacrifice all creatures you control, then create that many 4/4 red Hellion creature tokens.",
    );
    assert_partial_parse_rejected(
        "Ugin's Insight Variant",
        "Scry X, where X is the greatest mana value among permanents you control, then draw three cards.",
    );
    assert_partial_parse_rejected(
        "Stunted Growth Variant",
        "Target player chooses three cards from their hand and puts them on top of their library in any order.",
    );
    assert_partial_parse_rejected(
        "Stag Beetle Variant",
        "This creature enters with X +1/+1 counters on it, where X is the number of other creatures on the battlefield.",
    );
    assert_partial_parse_rejected(
        "Vesuva Variant",
        "You may have this land enter tapped as a copy of any land on the battlefield.",
    );
    assert_partial_parse_rejected(
        "Chocobo Racetrack Variant",
        "Landfall — Whenever a land you control enters, create a 2/2 green Bird creature token with \"Whenever a land you control enters, this token gets +1/+0 until end of turn.\"",
    );
    assert_partial_parse_rejected(
        "Dryad Arbor Variant",
        "This land isn't a spell, it's affected by summoning sickness, and it has \"{T}: Add {G}.\"",
    );
    assert_partial_parse_rejected("Waning Wurm Variant", "Vanishing 2");
    assert_partial_parse_rejected(
        "Mogg Bombers Variant",
        "When another creature enters, sacrifice this creature and it deals 3 damage to target player or planeswalker.",
    );
    assert_partial_parse_rejected(
        "Reef Worm Variant",
        "When this creature dies, create a 3/3 blue Fish creature token with \"When this token dies, create a 6/6 blue Whale creature token with 'When this token dies, create a 9/9 blue Kraken creature token.\"",
    );
    assert_partial_parse_rejected(
        "Keldon Firebombers Variant",
        "When this creature enters, each player sacrifices all lands they control except for three.",
    );
    assert_partial_parse_rejected(
        "Azra Bladeseeker Variant",
        "When this creature enters, each player on your team may discard a card, then each player who discarded a card this way draws a card.",
    );
    assert_partial_parse_rejected(
        "Boggart Trawler Variant",
        "When this creature enters, exile target player's graveyard.",
    );
    assert_partial_parse_rejected(
        "Barrow Witches Variant",
        "When this creature enters, return target Knight card from your graveyard to your hand.",
    );
    assert_partial_parse_rejected(
        "Loaming Shaman Variant",
        "When this creature enters, target player shuffles any number of target cards from their graveyard into their library.",
    );
    assert_partial_parse_rejected(
        "Invasion of Ravnica Variant",
        "When this Siege enters, exile target nonland permanent an opponent controls that isn't exactly two colors.",
    );
    assert_partial_parse_rejected(
        "Captain Vargus Wrath Variant",
        "Whenever Captain Vargus Wrath attacks, Pirates you control get +1/+1 until end of turn for each time you've cast a commander from the command zone this game.",
    );
    assert_partial_parse_rejected(
        "Carnival of Souls Variant",
        "Whenever a creature enters, you lose 1 life and add {B}.",
    );
    assert_partial_parse_rejected(
        "Skrelv's Hive Variant",
        "At the beginning of your upkeep, you lose 1 life and create a 1/1 colorless Phyrexian Mite artifact creature token with toxic 1 and \"This token can't block.\"",
    );
    assert_partial_parse_rejected("Constant Mists Variant", "Buyback—Sacrifice a land.");
    assert_partial_parse_rejected(
        "Dig Up the Body Variant",
        "Casualty 1 (As you cast this spell, you may sacrifice a creature with power 1 or greater. When you do, copy this spell.)",
    );
    assert_partial_parse_rejected(
        "Cataclysmic Prospecting Variant",
        "For each mana from a Desert spent to cast this spell, create a tapped Treasure token.",
    );
    assert_partial_parse_rejected(
        "Skittering Invasion Variant",
        "They have \"Sacrifice this token: Add {C}.\"",
    );
    assert_partial_parse_rejected(
        "Sound the Call Variant",
        "It has \"This token gets +1/+1 for each card named Sound the Call in each graveyard.\"",
    );
    assert_partial_parse_rejected(
        "All Suns' Dawn Variant",
        "For each color, return up to one target card of that color from your graveyard to your hand.",
    );
    assert_partial_parse_rejected(
        "Spark Rupture Variant",
        "Each planeswalker is a creature with power and toughness each equal to the number of loyalty counters on it.",
    );
    assert_partial_parse_rejected(
        "Ill-Gotten Gains Variant",
        "Each player discards their hand, then returns up to three cards from their graveyard to their hand.",
    );
    assert_partial_parse_rejected(
        "Declaration in Stone Variant",
        "That player investigates for each nontoken creature exiled this way.",
    );
    assert_partial_parse_rejected(
        "Legion's End Variant",
        "Then that player reveals their hand and exiles all cards with that name from their hand and graveyard.",
    );
    assert_partial_parse_rejected(
        "The Mending of Dominaria Variant",
        "Return all land cards from your graveyard to the battlefield, then shuffle your graveyard into your library.",
    );
    assert_partial_parse_rejected(
        "Archangel's Light Variant",
        "You gain 2 life for each card in your graveyard, then shuffle your graveyard into your library.",
    );
    assert_partial_parse_rejected(
        "Long-Term Plans Variant",
        "Search your library for a card, then shuffle and put that card third from the top.",
    );
    assert_partial_parse_rejected(
        "Reckless Blaze Variant",
        "Whenever a creature you control dealt damage this way dies this turn, add {R}.",
    );
    assert_partial_parse_rejected("Torens Variant", "Training");
    assert_partial_parse_rejected("Crack in Time Variant", "Vanishing 3");
    assert_partial_parse_rejected(
        "Simulacrum Synthesizer Variant",
        "Whenever another artifact you control with mana value 3 or greater enters, create a 0/0 colorless Construct artifact creature token.",
    );
    assert_partial_parse_rejected(
        "Wurmwall Sweeper Variant",
        "When this Spacecraft enters, surveil 2.",
    );
    assert_partial_parse_rejected("Patron of the Orochi Variant", "Snake offering");
    assert_partial_parse_rejected(
        "Scion of Halaster Variant",
        "The first time you would draw a card each turn, instead look at the top two cards of your library. Put one of them into your graveyard and the other back on top of your library.",
    );
    assert_partial_parse_rejected(
        "Roadside Assistance Variant",
        "Enchanted permanent gets +1/+1 and has lifelink.",
    );
    assert_partial_parse_rejected(
        "Trusty Boomerang Variant",
        "Equipped creature has \"{1}, {T}: Tap target creature.\"",
    );
    assert_partial_parse_rejected(
        "Tiller Engine Variant",
        "Whenever a land you control enters tapped, choose one — Tap target nonland permanent an opponent controls; or untap that land.",
    );
    assert_partial_parse_rejected(
        "Surge of Acclaim Variant",
        "Choose one. If you have max speed, choose both instead.",
    );
    assert_partial_parse_rejected(
        "Glamermite Variant",
        "When this creature enters, choose one — Tap target creature; or untap target creature.",
    );
    assert_partial_parse_rejected(
        "Deconstruction Hammer Variant",
        "Equipped creature gets +1/+1 and has \"{3}, {T}, Sacrifice Deconstruction Hammer: Destroy target artifact or enchantment.\"",
    );
    assert_partial_parse_rejected(
        "Pulse of the Forge Variant",
        "Pulse of the Forge deals 4 damage to target player or planeswalker. Then if that player or that planeswalker's controller has more life than you, return Pulse of the Forge to its owner's hand.",
    );
    assert_partial_parse_rejected(
        "Ezio, Brash Novice Variant",
        "As long as Ezio has two or more counters on it, it has first strike and is an Assassin in addition to its other types.",
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
fn parse_activate_only_restriction_standalone_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Timed Activator")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "{T}: Draw a card\nActivate only during your turn before attackers are declared.",
        )
        .expect("parse standalone activation restriction line");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("activation_restriction"),
        "expected activation restriction marker ability, got {debug}"
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
    assert!(
        rendered.contains("Activate only as an instant"),
        "expected instant-speed activation restriction in rendering, got {rendered}"
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
        joined.contains("Equip {3}. This ability costs {1} less to activate for each other Equipment you control"),
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
fn parse_destroy_all_with_no_counters_keeps_counter_qualifier() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Damning Verdict Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Destroy all creatures with no counters on them.")
        .expect("parse no-counters destroy-all clause");

    let joined = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains("destroy all creatures"),
        "expected destroy-all wording, got {joined}"
    );
    assert!(
        joined.contains("with no counters on them"),
        "expected no-counters qualifier in rendered text, got {joined}"
    );
}

#[test]
fn parse_creatures_with_no_counters_gets_pt_modifier() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Hazardous Conditions Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Creatures with no counters on them get -2/-2 until end of turn.")
        .expect("parse no-counters global debuff clause");

    let joined = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains("creatures with no counters on them get -2/-2 until end of turn"),
        "expected no-counters global debuff wording, got {joined}"
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
fn parse_counter_target_spell_if_it_was_kicked_keeps_condition() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Ertai's Trickery Variant")
        .card_types(vec![CardType::Instant])
        .parse_text("Counter target spell if it was kicked.")
        .expect("parse kicked-target conditional counter");

    let joined = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains("counter target spell if it was kicked"),
        "expected kicked conditional counter wording, got {joined}"
    );
}

#[test]
fn parse_counter_target_spell_thats_second_cast_this_turn_keeps_condition() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Second Guess Variant")
        .card_types(vec![CardType::Instant])
        .parse_text("Counter target spell that's the second spell cast this turn.")
        .expect("parse second-spell conditional counter");

    let joined = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains("counter target spell that's the second spell cast this turn"),
        "expected second-spell conditional counter wording, got {joined}"
    );
}

#[test]
fn parse_exile_target_creature_with_greatest_power_keeps_condition() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Topple Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Exile target creature with the greatest power among creatures on the battlefield.",
        )
        .expect("parse greatest-power exile target clause");

    let joined = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains(
            "exile target creature with the greatest power among creatures on the battlefield"
        ),
        "expected greatest-power target qualifier wording, got {joined}"
    );
}

#[test]
fn parse_destroy_all_of_creature_type_of_choice_builds_shared_subtype_filter() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Extinction Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Destroy all creatures of the creature type of your choice.")
        .expect("parse creature-type choice destroy-all clause");

    let joined = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains("shares a creature type with that object"),
        "expected shared creature-type qualifier in rendered text, got {joined}"
    );
}

#[test]
fn parse_return_targets_of_creature_type_of_choice_builds_shared_subtype_target_filter() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Selective Snare Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Return X target creatures of the creature type of your choice to their owner's hand.",
        )
        .expect("parse creature-type choice return clause");

    let joined = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains("shares a creature type with that object"),
        "expected shared creature-type qualifier in rendered text, got {joined}"
    );
}

#[test]
fn render_sliver_granted_mana_ability_uses_oracle_colon_form() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Crypt Sliver Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("All Slivers have \"{T}: Regenerate target Sliver.\"")
        .expect("parse sliver granted mana ability");

    let joined = oracle_like_lines(&def).join(" ");
    assert!(
        joined.contains("All Slivers have \"{T}: Regenerate target Sliver.\""),
        "expected sliver grant rendering with explicit mana colon, got {joined}"
    );
}

#[test]
fn render_search_library_clause_drops_you_own_wording() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Search Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Search your library for an artifact card, reveal it, put it into your hand, then shuffle.")
        .expect("parse search clause");

    let joined = oracle_like_lines(&def).join(" ");
    assert!(
        joined.contains("Search your library for an artifact card, reveal it, put it into your hand, then shuffle"),
        "expected oracle-like search wording without 'you own', got {joined}"
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
        joined.contains("Choose one or more — Destroy target artifact"),
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
fn render_each_player_put_from_graveyard_clause_uses_their_graveyard() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Exhume Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Each player puts a creature card from their graveyard onto the battlefield.")
        .expect("parse each-player put clause");

    let joined = oracle_like_lines(&def).join(" ");
    assert!(
        joined
            .contains("Each player puts a creature card from their graveyard onto the battlefield"),
        "expected each-player graveyard wording, got {joined}"
    );
}

#[test]
fn render_counter_then_token_for_controller_uses_its_controller_creates() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Offer Variant")
        .card_types(vec![CardType::Instant])
        .parse_text("Counter target noncreature spell. Its controller creates a Treasure token.")
        .expect("parse counter/create clause");

    let joined = oracle_like_lines(&def).join(" ");
    assert!(
        joined
            .contains("Counter target noncreature spell. Its controller creates a Treasure token"),
        "expected controller-token wording, got {joined}"
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
fn render_spell_damage_line_uses_card_name_subject() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Bathe in Dragonfire")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Bathe in Dragonfire deals 4 damage to target creature.")
        .expect("parse named spell damage clause");

    let joined = oracle_like_lines(&def).join(" ");
    assert!(
        joined.contains("Bathe in Dragonfire deals 4 damage to target creature"),
        "expected card-name damage subject for spell line, got {joined}"
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
        lower.contains("if the tagged object 'destroyed_0' matches permanent with mana value 2 or less"),
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
        lower.contains("matches artifact or creature or enchantment or land"),
        "expected full type-list predicate in conditional, got {joined}"
    );
    assert!(
        lower.contains("that player may put it onto the battlefield"),
        "expected true branch to keep put-it effect, got {joined}"
    );
}

#[test]
fn render_each_player_reveal_top_card_uses_reveals_wording() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Parley Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Each player reveals the top card of their library.")
        .expect("parse each-player reveal-top clause");

    let joined = oracle_like_lines(&def).join(" ");
    assert!(
        joined.contains("Each player reveals the top card of their library"),
        "expected each-player reveal wording, got {joined}"
    );
}

#[test]
fn render_spell_damage_then_lifegain_uses_and_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Lightning Helix")
        .card_types(vec![CardType::Instant])
        .parse_text("Lightning Helix deals 3 damage to any target and you gain 3 life.")
        .expect("parse damage-plus-lifegain spell");

    let joined = oracle_like_lines(&def).join(" ");
    assert!(
        joined.contains("Lightning Helix deals 3 damage to any target and you gain 3 life"),
        "expected damage-and-lifegain sentence join, got {joined}"
    );
}

#[test]
fn render_spell_dual_damage_clauses_join_when_second_not_greater() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Hungry Flames")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Hungry Flames deals 3 damage to target creature and 2 damage to target player or planeswalker.",
        )
        .expect("parse dual-damage spell");

    let joined = oracle_like_lines(&def).join(" ");
    assert!(
        joined.contains(
            "Hungry Flames deals 3 damage to target creature and deal 2 damage to any target"
        ) || joined
            .contains("Hungry Flames deals 3 damage to target creature and 2 damage to any target"),
        "expected dual-damage clauses to be joined with 'and', got {joined}"
    );
}

#[test]
fn parse_counter_unless_where_x_fails_strictly() {
    let err = CardDefinitionBuilder::new(CardId::from_raw(1), "Rethink Variant")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Counter target spell unless its controller pays {X}, where X is its mana value.",
        )
        .expect_err("where-x unless-payment should fail strict parse");
    let joined = format!("{err:?}");
    assert!(
        joined.contains("unsupported where-x clause"),
        "expected where-x strict parse error, got {joined}"
    );
}

#[test]
fn parse_counter_unless_plus_additional_stops_at_first_mana_segment() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Spell Stutter Variant")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Counter target spell unless its controller pays {2} plus an additional {1} for each Faerie you control.",
        )
        .expect("parse counter-unless-plus-additional clause");

    let joined = oracle_like_lines(&def).join(" ");
    assert!(
        joined.contains("pays {2}") && !joined.contains("{2}{1}"),
        "expected base payment segment to remain unduplicated, got {joined}"
    );
}

#[test]
fn render_counter_spirit_or_arcane_clause_orders_spell_last() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Defiance Variant")
        .card_types(vec![CardType::Instant])
        .parse_text("Counter target Spirit or Arcane spell.")
        .expect("parse spirit-or-arcane counter clause");

    let joined = oracle_like_lines(&def).join(" ");
    assert!(
        joined.contains("Counter target Spirit or Arcane spell"),
        "expected spirit-or-arcane spell ordering, got {joined}"
    );
}

#[test]
fn render_destroy_target_artifact_creature_planeswalker_uses_list_commas() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Bedevil Variant")
        .card_types(vec![CardType::Instant])
        .parse_text("Destroy target artifact, creature, or planeswalker.")
        .expect("parse three-type destroy clause");

    let joined = oracle_like_lines(&def).join(" ");
    assert!(
        joined.contains("Destroy target artifact, creature, or planeswalker"),
        "expected comma-list destroy targeting, got {joined}"
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
fn render_activation_return_cost_preserves_numeric_count() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Flooded Shoreline Variant")
        .parse_text("{U}{U}, Return two Islands you control to their owner's hand: Return target creature to its owner's hand.")
        .expect("counted return cost should parse");

    let joined = oracle_like_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains("return two islands you control to their owner's hand"),
        "expected counted return activation cost wording, got {joined}"
    );
}

#[test]
fn parse_delayed_return_at_end_of_combat_fails_strictly() {
    let err = CardDefinitionBuilder::new(CardId::from_raw(1), "Kaijin Variant")
        .parse_text("Return target creature to its owner's hand at end of combat.")
        .expect_err("delayed return timing should fail strict parse");
    assert!(
        format!("{err:?}").contains("unsupported delayed return timing clause"),
        "expected delayed return timing parse error, got {err:?}"
    );
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
fn parse_filter_with_counter_on_it_fails_strictly() {
    let err = CardDefinitionBuilder::new(CardId::from_raw(1), "Razorfin Variant")
        .parse_text("{1}{U}, {T}: Return target creature with a counter on it to its owner's hand.")
        .expect_err("counter-state object filter should fail strict parse");
    assert!(
        format!("{err:?}").contains("unsupported counter-state object filter"),
        "expected unsupported counter-state filter parse error, got {err:?}"
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
fn parse_semantic_guard_rejects_round_trip_drift_when_enabled() {
    let prev_guard = std::env::var("IRONSMITH_PARSER_SEMANTIC_GUARD").ok();
    let prev_dims = std::env::var("IRONSMITH_PARSER_SEMANTIC_DIMS").ok();
    let prev_threshold = std::env::var("IRONSMITH_PARSER_SEMANTIC_THRESHOLD").ok();
    unsafe {
        std::env::set_var("IRONSMITH_PARSER_SEMANTIC_GUARD", "1");
        std::env::set_var("IRONSMITH_PARSER_SEMANTIC_DIMS", "384");
        std::env::set_var("IRONSMITH_PARSER_SEMANTIC_THRESHOLD", "0.9");
    }

    let err = CardDefinitionBuilder::new(CardId::from_raw(1), "Dogged Hunter Variant")
        .card_types(vec![CardType::Artifact])
        .parse_text("{T}: Destroy target creature token.")
        .expect_err("semantic round-trip mismatch should fail parse");

    match prev_guard {
        Some(value) => unsafe {
            std::env::set_var("IRONSMITH_PARSER_SEMANTIC_GUARD", value);
        },
        None => unsafe {
            std::env::remove_var("IRONSMITH_PARSER_SEMANTIC_GUARD");
        },
    }
    match prev_dims {
        Some(value) => unsafe {
            std::env::set_var("IRONSMITH_PARSER_SEMANTIC_DIMS", value);
        },
        None => unsafe {
            std::env::remove_var("IRONSMITH_PARSER_SEMANTIC_DIMS");
        },
    }
    match prev_threshold {
        Some(value) => unsafe {
            std::env::set_var("IRONSMITH_PARSER_SEMANTIC_THRESHOLD", value);
        },
        None => unsafe {
            std::env::remove_var("IRONSMITH_PARSER_SEMANTIC_THRESHOLD");
        },
    }

    assert!(
        format!("{err:?}").contains("semantic round-trip mismatch"),
        "expected semantic round-trip guard error, got {err:?}"
    );
}

#[test]
fn parse_shared_color_destroy_fanout_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Radiance Destroy Variant")
        .parse_text(
            "Destroy target enchantment and each other enchantment that shares a color with it.",
        )
        .expect("shared-color destroy fanout should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("destroy target enchantment"),
        "expected primary destroy target clause, got {rendered}"
    );
    assert!(
        rendered.contains("shares a color with that object"),
        "expected shared-color fanout clause, got {rendered}"
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
        rendered.contains("prevent the next 1 damage to target creature"),
        "expected primary prevent target clause, got {rendered}"
    );
    assert!(
        rendered.contains("for each") && rendered.contains("shares a color with that object"),
        "expected shared-color prevent fanout loop, got {rendered}"
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
fn parse_when_you_do_followup_clause_as_result_conditional() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Invasion Variant")
        .card_types(vec![CardType::Battle])
        .parse_text(
            "When this permanent enters, you may sacrifice an artifact or creature. When you do, exile target artifact or creature an opponent controls.",
        )
        .expect("when-you-do followup clause should parse as dependent conditional");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("if you do"),
        "expected dependent followup to keep if-you-do linkage, got {rendered}"
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
fn parse_demonstrative_reference_keeps_that_subject() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Demonstrative Variant")
        .parse_text(
            "Target creature gets +1/+1 until end of turn. That creature can't block this turn.",
        )
        .expect("demonstrative target reference should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("that creature"),
        "expected demonstrative tagged subject wording, got {rendered}"
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
fn parse_rejects_three_dog_aura_copy_attachment_clause() {
    let err = CardDefinitionBuilder::new(CardId::from_raw(1), "Three Dog Variant")
        .parse_text(
            "Whenever you attack, you may pay {2} and sacrifice an Aura attached to this creature. When you sacrifice an Aura this way, for each other attacking creature you control, create a token that's a copy of that Aura attached to that creature.",
        )
        .expect_err("unsupported aura-copy attachment fanout should not partially parse");

    let message = format!("{err:?}").to_ascii_lowercase();
    assert!(
        message.contains("unsupported parser line")
            || message.contains("unsupported known partial parse pattern"),
        "expected explicit unsupported rejection, got {message}"
    );
}

#[test]
fn parse_rejects_if_no_mana_was_spent_counter_clause() {
    let err = CardDefinitionBuilder::new(CardId::from_raw(1), "Nix Variant")
        .parse_text("Counter target spell if no mana was spent to cast it.")
        .expect_err("no-mana-spent counter clause should not partially parse");

    let message = format!("{err:?}").to_ascii_lowercase();
    assert!(
        message.contains("unsupported parser line")
            || message.contains("unsupported known partial parse pattern"),
        "expected explicit unsupported rejection, got {message}"
    );
}

#[test]
fn parse_rejects_first_spell_during_each_opponents_turn_clause() {
    let err = CardDefinitionBuilder::new(CardId::from_raw(1), "Wavebreak Variant")
        .parse_text("Whenever you cast your first spell during each opponent's turn, draw a card.")
        .expect_err("first-spell-during-opponents-turn trigger should not partially parse");

    let message = format!("{err:?}").to_ascii_lowercase();
    assert!(
        message.contains("unsupported parser line")
            || message.contains("unsupported known partial parse pattern"),
        "expected explicit unsupported rejection, got {message}"
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
        lower.contains("destroy all creatures target opponent controls"),
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
fn render_spell_self_exile_uses_card_name() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Vivid Revival")
        .parse_text("Return up to three target multicolored cards from your graveyard to your hand. Exile this spell.")
        .expect("self-exile spell line should parse");

    let rendered = compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Exile Vivid Revival."),
        "expected card-name self-exile wording, got {rendered}"
    );
}

#[test]
fn parse_enchanted_creature_granted_trigger_keeps_static_grant() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Commander's Authority Variant")
        .parse_text(
            "Enchant creature\nEnchanted creature has \"At the beginning of your upkeep, create a 1/1 white Human creature token.\"",
        )
        .expect("enchanted granted trigger should parse as static grant");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("enchanted creature has at the beginning of your upkeep"),
        "expected attached granted trigger wording, got {rendered}"
    );
    assert!(
        !rendered.contains("spell effects: attach this permanent to target creature. create"),
        "granted trigger should not collapse into immediate spell create effect, got {rendered}"
    );
}

#[test]
fn parse_enchanted_creature_keyword_plus_granted_trigger_splits_correctly() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Cathar's Call Variant")
        .parse_text(
            "Enchant creature\nEnchanted creature has vigilance and \"At the beginning of your end step, create a 1/1 white Human creature token.\"",
        )
        .expect("keyword-plus-trigger grant should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("enchanted creature has vigilance"),
        "expected vigilance grant to remain, got {rendered}"
    );
    assert!(
        rendered.contains("enchanted creature has at the beginning of your end step"),
        "expected granted trigger clause to remain, got {rendered}"
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
        rendered.contains("if that player controls creature with power 4 or greater"),
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
fn parse_once_each_turn_play_from_exile_line_as_static_permission() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Evelyn Variant")
        .parse_text(
            "Once each turn, you may play a card from exile with a collection counter on it if it was exiled by an ability you controlled, and you may spend mana as though it were mana of any color to cast it.",
        )
        .expect("once-each-turn play-from-exile line should parse as static permission");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("once each turn, you may play a card from exile"),
        "expected play-from-exile permission text, got {rendered}"
    );
    assert!(
        rendered.contains("spend mana as though it were mana of any color to cast it"),
        "expected mana-as-any-color cast rider, got {rendered}"
    );
}

#[test]
fn parse_each_opponent_who_has_less_life_uses_conditional_you_create() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Great Unclean One Variant")
        .parse_text(
            "At the beginning of your end step, each opponent loses 2 life. Then for each opponent who has less life than you, create a 1/3 black Demon creature token.",
        )
        .expect("for-each-opponent life comparison clause should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("if that player has less life than you"),
        "expected life-comparison conditional, got {rendered}"
    );
    assert!(
        rendered.contains("under your control"),
        "expected created token controller to remain you, got {rendered}"
    );
}

#[test]
fn parse_burn_away_delayed_dies_this_turn_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Burn Away Variant")
        .parse_text(
            "Burn Away deals 6 damage to target creature. When that creature dies this turn, exile its controller's graveyard.",
        )
        .expect("burn-away delayed dies clause should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("when that creature dies this turn"),
        "expected delayed dies-this-turn trigger, got {rendered}"
    );
    assert!(
        rendered.contains("its controller's graveyard"),
        "expected graveyard exile to remain tied to that creature's controller, got {rendered}"
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
fn render_each_player_who_controls_creates() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Fade Variant")
        .parse_text(
            "Each player who controls an artifact or enchantment creates a 2/2 green Bear creature token. Then destroy all artifacts and enchantments.",
        )
        .expect("fade-style each-player creates clause should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains(
            "each player who controls an artifact or enchantment creates a 2/2 green bear creature token"
        ),
        "expected compact each-player conditional create wording, got {rendered}"
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
fn parse_create_token_for_each_color_of_mana_spent() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Sweep Variant")
        .parse_text(
            "Create a 1/1 colorless Thopter artifact creature token with flying for each color of mana spent to cast this spell.",
        )
        .expect("for-each-color converge token clause should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("for each color of mana spent to cast this spell"),
        "expected for-each-color token count wording, got {rendered}"
    );
}

#[test]
fn parse_for_each_destroyed_this_way_uses_destroyed_tagged_context() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Rampage Variant")
        .parse_text(
            "Destroy all artifacts and enchantments. For each permanent destroyed this way, its controller creates a 3/3 green Centaur creature token.",
        )
        .expect("for-each destroyed-this-way clause should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("its controller creates")
            || rendered.contains("that object's controller creates"),
        "expected created token controller to track destroyed permanent controller, got {rendered}"
    );
    assert!(
        !rendered.contains("tagged ''") && !rendered.contains("for each tagged object"),
        "expected destroyed-this-way loop to resolve to a concrete prior tag, got {rendered}"
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
fn parse_you_and_target_opponent_each_draw_sentence() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Shared Draw Variant")
        .parse_text("You and target opponent each draw three cards.")
        .expect("shared draw sentence should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("you draw three cards")
            && rendered.contains("target opponent draws three cards"),
        "expected both players to draw, got {rendered}"
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
        rendered.contains("search your library for demon")
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
fn parse_that_creature_gets_and_gains_uses_single_tagged_target() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Ogre Battledriver Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Whenever another creature you control enters, that creature gets +2/+0 and gains haste until end of turn.",
        )
        .expect("that-creature gets-and-gains clause should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("that creature gets +2/+0")
            || rendered.contains("it gets +2/+0"),
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
fn parse_nesting_dragon_inline_token_rules_remain_attached_to_created_token() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Nesting Dragon Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Landfall — Whenever a land you control enters, create a 0/2 red Dragon Egg creature token with defender and \"When this token dies, create a 2/2 red Dragon creature token with flying and '{R}: This token gets +1/+0 until end of turn.'\"",
        )
        .expect("inline token-rules create clause should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("dragon egg creature token"),
        "expected dragon egg token descriptor to remain intact, got {rendered}"
    );
    assert!(
        rendered.contains("when this token dies"),
        "expected nested token dies rule to stay attached to the created token, got {rendered}"
    );
    assert!(
        !rendered.contains("under your control. create a 2/2 red dragon creature token"),
        "expected nested rule text not to split into immediate standalone effects, got {rendered}"
    );
}
