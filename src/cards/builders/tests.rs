use super::*;
use crate::ability::AbilityKind;
use crate::color::Color;
use crate::compiled_text::compiled_lines;
use crate::static_abilities::StaticAbilityId;
use crate::target::ChooseSpec;

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
        joined.contains("whenever a permanent vampire you control deals combat damage to a player")
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
        joined.contains("whenever this permanent deals damage to permanent vampire"),
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
fn test_parse_additional_keyword_marker_lines_without_fallback() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Marker Keywords")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Unleash\n\
Extort\n\
Mentor\n\
Riot\n\
Dethrone\n\
Enlist\n\
Evolve\n\
Myriad\n\
Populate\n\
Provoke\n\
Skulk\n\
Sunburst\n\
Ravenous\n\
Undaunted\n\
Assist\n\
Cipher\n\
Partner\n\
Ingest\n\
Devoid\n\
Phasing",
        )
        .expect("parse standalone marker-style keywords");

    let rendered = compiled_lines(&def).join(" | ").to_ascii_lowercase();
    for keyword in [
        "unleash",
        "extort",
        "mentor",
        "riot",
        "dethrone",
        "enlist",
        "evolve",
        "myriad",
        "populate",
        "provoke",
        "skulk",
        "sunburst",
        "ravenous",
        "undaunted",
        "assist",
        "cipher",
        "partner",
        "ingest",
        "devoid",
        "phasing",
    ] {
        assert!(
            rendered.contains(keyword),
            "expected compiled text to include '{keyword}', got {rendered}"
        );
    }

    assert!(
        !rendered.contains("unsupported parser line fallback"),
        "standalone keyword markers should not hit fallback, got {rendered}"
    );

    let has_phasing = def.abilities.iter().any(|ability| match &ability.kind {
        AbilityKind::Static(static_ability) => static_ability.id() == StaticAbilityId::Phasing,
        _ => false,
    });
    assert!(
        has_phasing,
        "expected explicit Phasing static ability, got {:?}",
        def.abilities
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
        debug.contains("count: ChoiceCount { min: 3, max: Some(3) }"),
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
fn parse_enters_tapped_with_trailing_choose_clause_fails_strictly() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Thriving Variant")
        .card_types(vec![CardType::Land])
        .parse_text("This land enters tapped. As it enters, choose a color other than black.")
        .expect_err("unsupported trailing enters-tapped clause should fail parse");
    let message = format!("{err:?}");
    assert!(
        message.contains("unsupported trailing enters-tapped clause"),
        "expected strict trailing enters-tapped parse error, got {message}"
    );
}

#[test]
fn parse_add_mana_chosen_color_tail_fails_strictly() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Thriving Mana Variant")
        .card_types(vec![CardType::Land])
        .parse_text("{T}: Add {B} or one mana of the chosen color.")
        .expect_err("unsupported chosen-color mana tail should fail parse");
    let message = format!("{err:?}");
    assert!(
        message.contains("unsupported mana output option segment")
            || message.contains("unsupported trailing mana clause"),
        "expected strict trailing-mana parse error, got {message}"
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
fn parse_morph_keyword_line_as_marker() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Morph Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Morph {3}{R}")
        .expect("morph keyword line should parse as marker");
    let compiled = crate::compiled_text::compiled_lines(&def).join("\n");
    assert!(
        compiled.to_ascii_lowercase().contains("morph"),
        "expected compiled text to retain morph marker, got {compiled}"
    );
}

#[test]
fn parse_banding_keyword_line_as_marker() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Banding Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Banding")
        .expect("banding keyword line should parse as marker");
    let compiled = crate::compiled_text::compiled_lines(&def).join("\n");
    assert!(
        compiled.to_ascii_lowercase().contains("banding"),
        "expected compiled text to retain banding marker, got {compiled}"
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
fn render_multi_sacrifice_cost_uses_compact_filter_text() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Keldon Arsonist Variant")
        .parse_text("{1}, Sacrifice two lands: Destroy target land.")
        .expect("multi-sacrifice activated cost should parse");
    let lines = crate::compiled_text::compiled_lines(&def);
    let joined = lines.join("\n");
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
    let joined = lines.join("\n");
    assert!(
        joined.contains("sacrifice two artifacts"),
        "expected compact multi-artifact sacrifice rendering, got {joined}"
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
