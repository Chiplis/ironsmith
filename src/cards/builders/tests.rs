use super::*;
use crate::ability::AbilityKind;
use crate::color::Color;
use crate::compiled_text::{compiled_lines, oracle_like_lines};
use crate::effects::{
    AddManaEffect, CreateTokenEffect, GainLifeEffect, ReturnFromGraveyardToHandEffect,
};
use crate::static_abilities::StaticAbilityId;
use crate::target::{ChooseSpec, PlayerFilter};

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
    let crate::effect::Value::Count(first_filter) = &**left else {
        panic!("expected first count term");
    };
    let crate::effect::Value::Count(second_filter) = &**right else {
        panic!("expected second count term");
    };

    assert!(first_filter.subtypes.contains(&Subtype::Zombie));
    assert_eq!(first_filter.zone, Some(crate::zone::Zone::Battlefield));
    assert!(second_filter.subtypes.contains(&Subtype::Zombie));
    assert_eq!(second_filter.zone, Some(crate::zone::Zone::Graveyard));
    assert_eq!(power, toughness);
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
        joined.contains("that object's controller may choose new targets")
            && !joined.contains("you may choose new targets"),
        "expected retarget permission to stay linked to referenced controller, got {joined}"
    );
}

#[test]
fn test_parse_copy_this_spell_for_each_creature_sacrificed_this_way() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Plumb Variant")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "As an additional cost to cast this spell, you may sacrifice one or more creatures. When you do, copy this spell for each creature sacrificed this way.\nDraw a card.",
        )
        .expect("parse copy-this-spell for each sacrificed creature");

    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("expected when-you-do triggered ability");
    let debug = format!("{:?}", triggered.effects);
    assert!(
        debug.contains("CopySpellEffect") && debug.contains("target: Source"),
        "expected copy-this-spell target to remain source, got {debug}"
    );
    assert!(
        debug.contains("count: Count(")
            && debug.contains("Creature")
            && debug.contains("\"__it__\""),
        "expected count to track sacrificed creatures this way, got {debug}"
    );
    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains(
            "additional cost to cast this spell, you may sacrifice one or more creatures"
        ),
        "expected one-or-more sacrifice wording in compiled text, got {rendered}"
    );
    assert!(
        rendered.contains("copy this spell for each creature"),
        "expected counted copy wording in compiled text, got {rendered}"
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

    let tap = def
        .cost_effects
        .iter()
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

    let tap = def
        .cost_effects
        .iter()
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
        joined.contains("whenever a vampire you control deals combat damage to a player")
            && !joined.contains("whenever this creature deals combat damage to a player"),
        "expected trigger subject to remain filtered, got {joined}"
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
        joined.contains("whenever an opponent discard a card"),
        "expected discard trigger wording in compiled text, got {joined}"
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
fn test_parse_flashback_keyword_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Flashback Probe")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Flashback {1}{U}")
        .expect("flashback keyword line should parse");

    assert_eq!(def.alternative_casts.len(), 1);
    match &def.alternative_casts[0] {
        AlternativeCastingMethod::Flashback { cost } => {
            assert_eq!(cost.to_oracle(), "{1}{U}");
        }
        other => panic!("expected flashback alternative cast, got {other:?}"),
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
        rendered.contains("suspend 3—{0}") || rendered.contains("suspend 3 {0}"),
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
    let cost_reduction_count = static_abilities
        .iter()
        .filter(|id| **id == crate::static_abilities::StaticAbilityId::CostReduction)
        .count();
    assert_eq!(
        cost_reduction_count, 1,
        "expected exactly one cost reduction static ability, got {static_abilities:?}"
    );

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("target tapped creature"),
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
        debug.contains("DiscardEffect") && debug.contains("DrawCardsEffect"),
        "expected life-cycling to remain a discard+draw activated ability, got {debug}"
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
        joined.contains("target opponent discards a card"),
        "expected discard clause in chain, got {joined}"
    );
    assert!(
        joined.contains("target opponent loses 3 life"),
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
        mana_line.contains("Activate only if you control 3 or more artifacts"),
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
        mana_line.contains("Activate only if you control 5 or more lands"),
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
        mana_line.contains("Activate only if there is an Elf card in your graveyard"),
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
        mana_line.contains("Activate only if you control a creature with power 4 or greater"),
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
        joined.contains("controller gets 1 poison counter"),
        "expected controller-based poison counter wording, got {joined}"
    );
    assert!(
        !joined.contains("you get 1 poison counter"),
        "did not expect implicit-you poison counter wording, got {joined}"
    );
}

#[test]
fn parse_until_end_of_turn_whenever_clause_as_temporary_grant() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Mountain Titan Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("{1}{R}{R}: Until end of turn, whenever you cast a black spell, put a +1/+1 counter on this creature.")
        .expect("until-end-of-turn whenever clause should parse as temporary granted trigger");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("whenever you cast a black spell"),
        "expected cast trigger wording in compiled text, got {rendered}"
    );
    assert!(
        rendered.contains("until end of turn"),
        "expected temporary duration wording in compiled text, got {rendered}"
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
        rendered.contains("Deal 2 damage to target attacking or blocking creature")
            && rendered.contains("It deals 4 damage instead if you control a Mount"),
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
        rendered.contains("Deal 3 damage to target creature")
            && rendered.contains("It deals 5 damage instead if you control a Spacecraft"),
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
        rendered.contains("Deal 3 damage to target creature an opponent controls or planeswalker")
            && rendered.contains("It deals 5 damage instead if you control"),
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
fn parse_untap_during_each_other_players_untap_step_as_static_ability() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Seedborn Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Untap all permanents you control during each other player's untap step.")
        .expect("each-other-player untap line should parse as static ability");

    assert!(
        def.spell_effect
            .as_ref()
            .map(|effects| effects.is_empty())
            .unwrap_or(true),
        "expected no spell effects for untap-during-each-other-player static line"
    );

    let compiled = compiled_lines(&def).join("\n").to_ascii_lowercase();
    assert!(
        compiled.contains("untap all permanents you control during each other players untap step"),
        "expected compiled output to keep each-other-player untap wording, got {compiled}"
    );
    assert!(
        !compiled.contains("spell effects: untap all another permanent you control"),
        "unexpected statement-path untap rendering leak in compiled output: {compiled}"
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
fn parse_search_target_player_library_and_exile_cards() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Denying Wind Variant")
        .parse_text("Search target player's library for up to seven cards and exile them. Then that player shuffles.")
        .expect("target-player search-and-exile clause should parse");

    let joined = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains("search target player's library for up to seven cards and exile")
            && joined.contains("shuffle target player's library"),
        "expected search/exile/shuffle rendering, got {joined}"
    );

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("ChooseObjectsEffect")
            && debug.contains("zone: Library")
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

    let debug = format!("{:?}", def.spell_effect).to_ascii_lowercase();
    assert!(
        debug.contains("zone: library")
            && debug.contains("zone: some(graveyard)")
            && debug.contains("zone: some(hand)")
            && debug.contains("samenameastagged")
            && debug.contains("controllerof")
            && debug.contains("shufflelibraryeffect"),
        "expected same-name exile across hand/graveyard/library and controller shuffle, got {debug}"
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
            && joined.contains("put them into target opponent's graveyard")
            && joined.contains("shuffle target opponent's library"),
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
        debug.contains("add(fixed(3), count(")
            && debug.contains("name: some(\"muscle burst\")")
            && debug.contains("zone: some(graveyard)"),
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
        abilities_debug.contains("custom_static_markers: [\"mana ability\"]")
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
        joined.contains("powerstone artifact token") && joined.contains("tapped"),
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
        debug.contains("custom_id: \"banding\""),
        "expected created token to keep banding marker ability, got {debug}"
    );

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("token with banding"),
        "expected compiled text to include banding token modifier, got {rendered}"
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
fn parse_alternative_cost_with_return_to_hand_segment_preserves_cost_effects() {
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
        alternative.uses_composed_cost_effects(),
        "expected non-mana alternative cost effects to be preserved"
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
fn parse_deathpact_style_token_activation_is_preserved() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Deathpact Angel")
        .card_types(vec![CardType::Creature])
        .parse_text("When this creature dies, create a 1/1 white and black Cleric creature token. It has \"{3}{W}{B}{B}, {T}, Sacrifice this token: Return a card named Deathpact Angel from your graveyard to the battlefield.\"")
        .expect("deathpact-style token activation should parse");
    let compiled = compiled_lines(&def).join(" ");
    let lower = compiled.to_ascii_lowercase();
    assert!(
        lower.contains("sacrifice this token")
            && lower.contains(
                "return a card named deathpact angel from your graveyard to the battlefield"
            ),
        "expected preserved return-from-graveyard token activation, got {compiled}"
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

    let create = def
        .spell_effect
        .as_ref()
        .and_then(|effects| {
            effects
                .iter()
                .find_map(|effect| effect.downcast_ref::<CreateTokenEffect>())
        })
        .expect("expected spell create-token effect");
    let has_cda = create.token.abilities.iter().any(|ability| {
        matches!(
            &ability.kind,
            AbilityKind::Static(static_ability)
                if static_ability.id() == StaticAbilityId::CharacteristicDefiningPT
        )
    });
    assert!(
        has_cda,
        "expected Construct token to keep dynamic +1/+1 scaling text, got {:#?}",
        create.token.abilities
    );
}

#[test]
fn parse_sound_the_call_token_does_not_misread_named_card_reference_as_token_name() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Sound the Call Variant")
        .parse_text(
            "Create a 1/1 green Wolf creature token. It has \"This token gets +1/+1 for each card named Sound the Call in each graveyard.\"",
        )
        .expect("sound-the-call token reminder should parse");

    let create = def
        .spell_effect
        .as_ref()
        .and_then(|effects| {
            effects
                .iter()
                .find_map(|effect| effect.downcast_ref::<CreateTokenEffect>())
        })
        .expect("expected spell create-token effect");
    assert_eq!(
        create.token.name(),
        "Wolf",
        "token name should remain subtype-derived Wolf, not reminder-card-name text"
    );
    let has_scaling = create.token.abilities.iter().any(|ability| {
        matches!(
            &ability.kind,
            AbilityKind::Static(static_ability) if static_ability.id() == StaticAbilityId::Anthem
        )
    });
    assert!(
        has_scaling,
        "expected token to keep +1/+1-for-each-named-card ability, got {:#?}",
        create.token.abilities
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
        rendered.contains("activate only if you control a swamp"),
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
        ids.contains(&crate::static_abilities::StaticAbilityId::DoesntUntap),
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
    assert!(!rendered.contains("Activate only as an instant"));
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
        lower.contains(
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
fn parse_gain_x_plus_life_with_where_clause_binds_x_value() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "An-Havva Inn Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "You gain X plus 1 life, where X is the number of green creatures on the battlefield.",
        )
        .expect("gain-x-plus-life with where clause should parse");

    let joined = oracle_like_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains("where x is the number of green creatures"),
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
fn parse_modal_trigger_header_keeps_prefix_effect_and_result_gate() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Immard Variant")
        .parse_text(
            "Whenever this creature enters or attacks, put a charge counter on it or remove one from it. When you remove a counter this way, choose one —\n• This creature deals 4 damage to any target.\n• This creature gains lifelink and indestructible until end of turn.",
        )
        .expect("modal triggered header should keep prefix effect and conditional gate");

    let abilities_debug = format!("{:?}", def.abilities);
    assert!(
        abilities_debug.contains("PutCountersEffect")
            && abilities_debug.contains("RemoveCountersEffect"),
        "expected put/remove counters effect before modal branch, got {abilities_debug}"
    );
    assert!(
        abilities_debug.contains("IfEffect") && abilities_debug.contains("ChooseModeEffect"),
        "expected modal branch to be gated by prior effect result, got {abilities_debug}"
    );

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("put a charge counter on it or remove one from it"),
        "expected put/remove prefix wording to remain, got {rendered}"
    );
    assert!(
        (rendered.contains("if you do") || rendered.contains("when you remove"))
            && rendered.contains("choose one"),
        "expected conditional choose-one followup wording, got {rendered}"
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
        rendered
            .to_ascii_lowercase()
            .contains("for each of x target permanents"),
        "expected rendered text to keep 'for each of X target permanents', got {rendered}"
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
fn parse_defending_player_suffix_subject_keeps_player_binding() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Keeper Variant")
        .parse_text(
            "Whenever this creature attacks and isn't blocked, it assigns no combat damage this turn and defending player loses 2 life.",
        )
        .expect("parse defending-player suffix subject");

    let joined = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        joined.contains("defending player loses 2 life"),
        "expected defending-player life-loss wording, got {joined}"
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
fn parse_trigger_with_and_or_subtype_list_keeps_effect_split_on_trigger_delimiter() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Vaan Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Whenever one or more Scouts, Pirates, and/or Rogues you control deal combat damage to a player, exile the top card of that player's library. You may cast it. If you don't, create a Treasure token.")
        .expect("and/or subtype trigger list should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("exile the top card of that player's library")
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
        .expect("mabel token payload should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("colorless equipment artifact token"),
        "expected explicit colorless equipment token rendering, got {rendered}"
    );
    assert!(
        rendered.contains("equipped creature gets +1/+1")
            && rendered.contains("vigilance")
            && rendered.contains("trample")
            && rendered.contains("haste"),
        "expected equipped creature granted stats/keywords in token text, got {rendered}"
    );
    assert!(
        rendered.contains("equip {2}") || rendered.contains("equip 2"),
        "expected equip payload to remain on token, got {rendered}"
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
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Nosy Goblin Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("{T}, Sacrifice this creature: Destroy target face-down creature.")
        .expect("face-down target destroy ability should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("destroy target face-down creature"),
        "expected face-down target qualifier in compiled text, got {rendered}"
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
        rendered.contains("one or more phyrexian you control attack"),
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
fn parse_exile_top_card_of_target_library_preserves_top_card_selection() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Top Card Exile Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("{T}: Exile the top card of target player's library.")
        .expect("top-card exile should parse");

    let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("exile the top card of target player's library"),
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
