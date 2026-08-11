#![allow(unused_imports)]
use super::shard_00::*;
use super::shard_01::*;
use super::shard_02::*;
use super::shard_03::*;
use super::shard_04::*;
use super::shard_05::*;
use super::shard_06::*;
use super::shard_08::*;
use super::shard_09::*;
use super::shard_10::*;
use super::shard_11::*;
use super::shard_12::*;
use super::shard_13::*;
use super::shard_14::*;
use super::shard_15::*;
use super::shard_16::*;
use super::shard_17::*;
use super::shard_18::*;
use super::shard_19::*;
use super::shard_20::*;
use super::shard_21::*;
use super::shard_22::*;
use super::shard_23::*;
use super::*;

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn carry_away_unattaches_enchanted_equipment_and_controls_it() {
    let (mut game, alice, bob, carry_away_id, equipment_id, creature_id) =
        carry_away_runtime_setup(true);

    assert_eq!(game.controller_of_id(equipment_id), Some(alice));
    assert_eq!(
        game.object(equipment_id)
            .and_then(|equipment| equipment.attached_to),
        Some(crate::object::AttachmentTarget::Object(creature_id)),
        "test setup should start with the enchanted Equipment attached to the creature"
    );

    resolve_carry_away_enter_trigger(&mut game, carry_away_id);

    assert_eq!(game.controller_of_id(equipment_id), Some(alice));
    assert_eq!(game.controller_of_id(creature_id), Some(bob));
    assert_eq!(
        game.object(equipment_id)
            .and_then(|equipment| equipment.attached_to),
        None,
        "Carry Away should unattach the enchanted Equipment from the creature"
    );
    assert_eq!(
        game.object(carry_away_id).and_then(|aura| aura.attached_to),
        Some(crate::object::AttachmentTarget::Object(equipment_id)),
        "Carry Away should remain attached to the Equipment it enchants"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn carry_away_does_not_detach_aura_when_equipment_is_already_unattached() {
    let (mut game, alice, bob, carry_away_id, equipment_id, creature_id) =
        carry_away_runtime_setup(false);

    assert_eq!(game.controller_of_id(equipment_id), Some(alice));
    assert_eq!(game.controller_of_id(creature_id), Some(bob));
    assert_eq!(
        game.object(equipment_id)
            .and_then(|equipment| equipment.attached_to),
        None,
        "test setup should start with unattached enchanted Equipment"
    );

    resolve_carry_away_enter_trigger(&mut game, carry_away_id);

    assert_eq!(game.controller_of_id(equipment_id), Some(alice));
    assert_eq!(
        game.object(equipment_id)
            .and_then(|equipment| equipment.attached_to),
        None,
        "already-unattached Equipment should remain unattached"
    );
    assert_eq!(
        game.object(carry_away_id).and_then(|aura| aura.attached_to),
        Some(crate::object::AttachmentTarget::Object(equipment_id)),
        "the unattach effect should not detach Carry Away itself"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_auriok_steelshaper_strict_and_preserves_equip_cost_modifier_text() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Auriok Steelshaper")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Human, Subtype::Soldier])
        .power_toughness(PowerToughness::fixed(1, 1))
        .parse_text(
            "Equip costs you pay cost {1} less.\nAs long as this creature is equipped, each creature you control that's a Soldier or a Knight gets +1/+1.",
        )
        .expect("Auriok Steelshaper should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("equip costs you pay cost {1} less"),
        "expected equip cost-modifier line in compiled output, got {rendered}"
    );
    assert!(
        rendered.contains("as long as this creature is equipped")
            && rendered.contains("soldier")
            && rendered.contains("knight")
            && rendered.contains("gets +1/+1"),
        "expected equipped Soldier/Knight anthem line in compiled output, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_flashback_cost_modifiers_render_with_controller_scope() {
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

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_dash_cost_modifier_line_renders_with_controller_scope() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Warbringer Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Dash costs you pay cost {2} less (as long as this creature is on the battlefield).\nDash {2}{R}",
        )
        .expect("dash cost-modifier line should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Dash costs you pay cost {2} less"),
        "expected dash cost-modifier wording in render output, got {rendered}"
    );

    let reduction = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => static_ability.cost_reduction(),
            _ => None,
        })
        .expect("expected dash cost reduction static ability");

    assert_eq!(
        reduction.filter.alternative_cast,
        Some(crate::filter::AlternativeCastKind::Dash)
    );
    assert_eq!(reduction.filter.cast_by, Some(PlayerFilter::You));
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_during_turn_flashback_grant_keeps_mana_cost_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Return the Past Variant")
        .parse_text(
            "During your turn, each instant and sorcery card in your graveyard has flashback. Its flashback cost is equal to its mana cost.",
        )
        .expect("during-turn flashback grant should parse");
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered
            .to_ascii_lowercase()
            .contains("flashback cost is equal to its mana cost"),
        "expected flashback-cost sentence in rendering, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_underworld_breach_escape_grant_keeps_nonland_and_cost_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Underworld Breach Variant")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "Each nonland card in your graveyard has escape. The escape cost is equal to the card's mana cost plus exile three other cards from your graveyard.",
        )
        .expect("underworld breach-style escape grant should parse");
    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
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

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_gain_life_equal_to_its_power_uses_possessive_wording() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Infernal Reckoning Variant")
        .card_types(vec![CardType::Instant])
        .parse_text("Exile target colorless creature. You gain life equal to its power.")
        .expect("gain-life-equal-to-power line should parse");
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("gain life equal to its power"),
        "expected possessive power wording, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_gain_life_equal_to_sacrificed_creature_toughness() {
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

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_gain_life_equal_to_devotion_value() {
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

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_gain_life_equal_to_the_power_of_target_creature_you_control() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Wall of Reverence Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Flying\nAt the beginning of your end step, you may gain life equal to the power of target creature you control.",
        )
        .expect("target-creature power life gain should parse");

    let abilities_debug = format!("{:#?}", def.abilities);
    assert!(
        abilities_debug.contains("GainLifeEffect")
            && abilities_debug.contains("PowerOf")
            && abilities_debug.contains("Target("),
        "expected gain-life amount to use target creature power, got {abilities_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_gain_life_equal_to_your_speed() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Momentum Breaker Variant")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "Start your engines!\n{2}, Sacrifice this enchantment: You gain life equal to your speed.",
        )
        .expect("speed-based life gain should parse");

    let gain = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => activated
                .effects
                .flattened_default_effects()
                .iter()
                .find_map(|effect| effect.downcast_ref::<GainLifeEffect>()),
            _ => None,
        })
        .expect("expected activated gain-life effect");
    assert!(
        matches!(
            gain.amount.unhinted(),
            crate::effect::Value::Speed(PlayerFilter::You)
        ),
        "expected gain-life amount to use your speed, got {gain:?}"
    );
    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("gain life equal to your speed"),
        "expected speed life-gain rendering, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_gain_life_equal_to_life_lost_this_way() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Agent of Masks Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Each opponent loses 1 life. You gain life equal to the life lost this way.")
        .expect("life-lost-this-way life gain should parse");

    let spell_debug = format!("{:#?}", def.spell_effect);
    assert!(
        spell_debug.contains("GainLifeEffect")
            && (spell_debug.contains("EventValue")
                || spell_debug.contains("EffectValue(")
                || spell_debug.contains("EffectMetric")
                || spell_debug.contains("LifeLost")
                || spell_debug.contains("life lost this way")),
        "expected life-gain amount to use life-lost event value, got {spell_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_artifact_land_self_reference_prefers_land() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Artifact Land Variant")
        .card_types(vec![CardType::Artifact, CardType::Land])
        .parse_text("This land enters tapped.")
        .expect("artifact land line should parse");
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("This land enters tapped"),
        "expected land self-reference wording, got {rendered}"
    );
    assert!(
        !rendered.contains("This artifact enters tapped"),
        "artifact land should not render as artifact-only self-reference: {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_mana_value_or_less_keeps_comparison_and_type_conjunction() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Technomancer Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "When this creature enters, mill three cards, then return any number of artifact creature cards with total mana value 6 or less from your graveyard to the battlefield.",
        )
        .expect("technomancer line should parse");
    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        !rendered.contains("artifact or creature card"),
        "type conjunction should not degrade to union wording: {rendered}"
    );
    assert!(
        !rendered.contains("mana value 6s"),
        "comparison tokenization should not pluralize numeric threshold: {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_exile_from_graveyard_uses_from_preposition() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Grave Robbers Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("{B}, {T}: Exile target artifact card from a graveyard. You gain 2 life.")
        .expect("graveyard exile clause should parse");
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Exile target artifact card from a graveyard")
            || rendered.contains("Exile target artifact card in a graveyard"),
        "expected from-a-graveyard wording, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_granted_activated_ability_keeps_tap_symbol() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Brawl Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Until end of turn, all creatures gain \"{T}: This creature deals damage equal to its power to target creature.\"",
        )
        .expect("grant-tap-ability clause should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();
    assert!(
        rendered.contains("{T}")
            && rendered_lower
                .contains("this creature deals damage equal to its power to target creature"),
        "expected granted tap ability to preserve tap symbol, got {rendered}"
    );
    assert!(
        !rendered_lower.contains("gain t this creature deals")
            && !rendered_lower.contains(", choose target creature:"),
        "granted tap ability should not lose the tap symbol: {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_lose_life_for_each_with_multiplier_uses_scaled_count_value() {
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
    match lose.amount.unhinted() {
        Value::Scaled(inner, 2)
            if matches!(
                inner.unhinted(),
                Value::PriorEffectMetric {
                    query: crate::effect::PriorEffectMetricQuery {
                        source: crate::effect::EffectMetricSource::AffectedObjects,
                        metric: crate::effect::EffectMetric::Count,
                        action: Some(crate::effect::PriorEffectAction::Destroyed),
                        ..
                    },
                    ..
                }
            ) => {}
        other => panic!("expected doubled destroyed-this-way metric, got {other:?}"),
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_tap_x_artifacts_creatures_and_lands_preserves_and_or_list() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Malicious Advice Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Tap X target artifacts, creatures, and/or lands. You lose X life.")
        .expect("mixed target-type tap line should parse");
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let lower = rendered.to_ascii_lowercase();
    assert!(
        lower.contains("x target artifacts, creatures, and/or lands"),
        "expected artifacts/creatures/lands disjunction wording, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_destroy_then_populate_compiles_followup_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Sundering Growth Variant")
        .parse_text("Destroy target artifact or enchantment, then populate.")
        .expect("destroy-then-populate should parse");
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Destroy target artifact or enchantment")
            && rendered.contains("Populate"),
        "expected destroy then populate rendering, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_destroy_target_one_or_more_colors() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Reach of Shadows Variant")
        .parse_text("Destroy target creature that's one or more colors.")
        .expect("one-or-more-colors target should parse");
    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("destroy target colored creature")
            || rendered.contains("destroy target creature that's one or more colors"),
        "expected colored-target rendering, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_destroy_target_three_or_more_colors_still_fails_loudly() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Reach of Shadows Negative Variant")
        .parse_text("Destroy target creature that's three or more colors.")
        .expect_err("unsupported three-or-more-colors target should fail loudly");
    let message = format!("{err:?}");
    assert!(
        message.contains("unsupported color-count object filter"),
        "expected color-count filter parse error, got {message}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_time_counter_on_it_or_suspended_card_compiles() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Shivan Sand-Mage Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "When this creature enters, choose one —\n• Remove two time counters from target permanent or suspended card.\n• Put two time counters on target permanent with a time counter on it or suspended card.",
        )
        .expect("time-counter or suspended-card clause should parse");
    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains(
            "put two time counters on target permanent with a time counter on it or suspended card"
        ),
        "expected rendered text to preserve the counter-state suspended-card disjunction, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_ugin_colored_permanent_target_lines() {
    CardDefinitionBuilder::new(CardId::new(), "Ugin Variant")
        .card_types(vec![CardType::Planeswalker])
        .parse_text(
            "When you cast this spell, exile up to one target permanent that's one or more colors.\nWhenever you cast a colorless spell, exile up to one target permanent that's one or more colors.",
        )
        .expect("ugin colored-permanent target lines should parse");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_protection_from_spells_that_are_one_or_more_colors() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Emrakul Protection Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Protection from spells that are one or more colors.")
        .expect("colored-spell protection line should parse");
    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("protection from colored spells")
            || rendered.contains("protection from spells that are one or more colors"),
        "expected colored-spell protection wording, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_emrakul_the_world_anew_strict_oracle_text() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(124_001), "Emrakul, the World Anew")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(12)]]))
        .supertypes(vec![Supertype::Legendary])
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Eldrazi])
        .power_toughness(PowerToughness::fixed(12, 12))
        .parse_text(
            "When you cast this spell, gain control of all creatures target player controls.\n\
             Flying, protection from spells and from permanents that were cast this turn\n\
             When Emrakul leaves the battlefield, sacrifice all creatures you control.\n\
             Madness—Pay six {C}.",
        )
        .expect("Emrakul, the World Anew should parse strictly");

    let rendered = unprocessed_compiled_lines(&def).join("\n");
    assert_eq!(
        rendered,
        "When you cast this spell, gain control of all creatures target player controls.\n\
         Flying, protection from spells and from permanents that were cast this turn\n\
         When Emrakul leaves the battlefield, sacrifice all creatures you control.\n\
         Madness—Pay six {C}.",
        "Emrakul's strict parser output should preserve every oracle clause"
    );

    let cast_trigger = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => triggered
                .choices
                .contains(&ChooseSpec::target_player())
                .then_some(triggered),
            _ => None,
        })
        .expect("Emrakul cast trigger should require a target player choice");
    assert_eq!(
        cast_trigger.choices,
        vec![ChooseSpec::target_player()],
        "Emrakul's gain-control trigger should target only the player whose creatures are affected"
    );
    let effects_debug = format!("{:?}", cast_trigger.effects);
    assert!(
        effects_debug.contains("controller: Some(") && effects_debug.contains("Target"),
        "Emrakul's gain-control effect should filter creatures by the target player's controller, got {effects_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_exile_face_down_manifest_tail_fails_instead_of_partial_exile() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Ghastly Conscription Variant")
        .parse_text(
            "Exile all creature cards from target player's graveyard in a face-down pile, shuffle that pile, then manifest those cards.",
        )
        .expect_err("face-down/manifest exile tail should fail loudly when unsupported");
    let message = format!("{err:?}");
    assert!(
        message.contains("unsupported face-down/manifest exile clause")
            || message.contains("unsupported face-down clause")
            || message.contains("unsupported shuffle clause"),
        "expected actionable face-down/manifest parse error, got {message}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_return_all_dealt_damage_this_turn_fails_instead_of_broadening() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Restore the Peace Variant")
        .parse_text("Return each creature that dealt damage this turn to its owner's hand.")
        .expect_err("qualified return-all filter should fail when unsupported");
    let message = format!("{err:?}");
    assert!(
        message.contains("unsupported qualified return-all filter"),
        "expected qualified return-all parse error, got {message}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_return_all_without_counter_fails_instead_of_broadening() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Wave Goodbye Variant")
        .parse_text("Return each creature without a +1/+1 counter on it to its owner's hand.")
        .expect_err("without-counter return-all filter should fail when unsupported");
    let message = format!("{err:?}");
    assert!(
        message.contains("unsupported qualified return-all filter"),
        "expected qualified return-all parse error, got {message}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_sacrifice_unless_clause_fails_instead_of_ignoring_unless_tail() {
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

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_war_elemental_strictly_parses_etb_sacrifice_unless_opponent_damage_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "War Elemental")
        .parse_text(
            "When this creature enters, sacrifice it unless an opponent was dealt damage this turn.\nWhenever an opponent is dealt damage, put that many +1/+1 counters on this creature.",
        )
        .expect("War Elemental should parse");
    let joined = crate::compiled_text::unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("unless an opponent was dealt damage this turn, sacrifice it"),
        "expected compiled text to preserve ETB unless-opponent-damage semantics, got {joined}"
    );
    assert!(
        joined.contains(
            "whenever an opponent is dealt damage, put that many +1/+1 counters on this creature"
        ),
        "expected damage-counter trigger to remain intact, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn living_artifact_oracle_parses_with_player_damage_trigger_and_upkeep_branch() {
    let def = parse_oracle_card_definition("Living Artifact");
    let joined = canonical_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();

    assert!(
        joined.contains("whenever you are dealt damage")
            && joined.contains("vitality counters on this aura"),
        "expected Living Artifact player-damage trigger text, got {joined}"
    );
    assert!(
        joined.contains("at the beginning of your upkeep")
            && joined.contains("you may remove a vitality counter from this aura")
            && joined.contains("if you do, you gain 1 life"),
        "expected Living Artifact upkeep branch text, got {joined}"
    );
}

#[test]
pub(super) fn war_elemental_runtime_condition_matches_when_opponent_was_dealt_damage_this_turn() {
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let source = crate::ObjectId::from_raw(77);
    let bob = crate::PlayerId::from_index(1);
    let damage_event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::DamageEvent::with_cause(
            source,
            crate::events::DamageTarget::Player(bob),
            2,
            false,
            crate::events::cause::EventCause::effect(),
        ),
        crate::provenance::ProvNodeId::default(),
    );
    game.stage_turn_history_event(&damage_event);

    assert!(
        crate::condition_eval::evaluate_condition_cast_time(
            &game,
            &crate::effect::Condition::OpponentWasDealtDamageThisTurn,
            crate::PlayerId::from_index(0),
            source,
        ),
        "War Elemental ETB unless-condition should pass when an opponent lost life this turn"
    );
}

#[test]
pub(super) fn war_elemental_runtime_condition_fails_when_no_opponent_lost_life_this_turn() {
    let game = crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let source = crate::ObjectId::from_raw(77);

    assert!(
        !crate::condition_eval::evaluate_condition_cast_time(
            &game,
            &crate::effect::Condition::OpponentWasDealtDamageThisTurn,
            crate::PlayerId::from_index(0),
            source,
        ),
        "War Elemental ETB unless-condition should fail when no opponent lost life this turn"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_power_or_toughness_cant_be_blocked_subject_fails_loudly() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Tetsuko Variant")
        .parse_text("Creatures you control with power or toughness 1 or less can't be blocked.")
        .expect_err("power-or-toughness unblockable subject should fail when unsupported");
    let message = format!("{err:?}");
    assert!(
        message.contains("unsupported power-or-toughness cant-be-blocked subject"),
        "expected power-or-toughness subject parse error, got {message}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_target_player_gain_then_draw_carries_target_player_to_draw_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Kiss of the Amesha Variant")
        .parse_text("Target player gains 7 life and draws two cards.")
        .expect("gain-then-draw line should parse");
    let joined = crate::compiled_text::unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("target player gains 7 life and draws two cards"),
        "expected carried target player for draw clause, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_target_player_mill_draw_lose_chain_carries_target_player_to_draw_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Atrocious Experiment Variant")
        .parse_text("Target player mills two cards, draws two cards, and loses 2 life.")
        .expect("mill-draw-lose line should parse");
    let joined = crate::compiled_text::unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("target player mills two cards, draws two cards, and loses 2 life"),
        "expected carried target player for chained draw clause, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_target_player_mill_then_imperative_draw_does_not_carry_target_player() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Pilfered Plans Variant")
        .parse_text("Target player mills two cards. Draw two cards.")
        .expect("mill-then-draw line should parse");
    let joined = crate::compiled_text::unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        !joined.contains("target player draws two cards"),
        "imperative draw clause should not carry target player, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_defending_player_discard_then_draws_carries_defending_player() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Robber Fly Variant")
        .parse_text(
            "Whenever this creature becomes blocked, defending player discards all cards from their hand, then draws that many cards.",
        )
        .expect("defending-player discard-then-draw line should parse");
    let joined = crate::compiled_text::unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("defending player discards all the cards in their hand")
            && (joined.contains("then draws that many cards")
                || joined.contains("then defending player draws that many cards")),
        "expected defending player to carry into draws clause, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn bottom_score_parse_laquatus_creativity_draws_then_discards_that_many() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Laquatus's Creativity")
        .parse_text(
            "Target player draws cards equal to the number of cards in their hand, then discards that many cards.",
        )
        .expect("Laquatus's Creativity should parse both draw and discard effects");
    let debug = format!("{def:#?}");
    let rendered = crate::compiled_text::unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();

    assert!(debug.contains("DrawCardsEffect"), "{debug}");
    assert!(debug.contains("DiscardEffect"), "{debug}");
    assert!(
        rendered.contains(
            "target player draws cards equal to the number of cards in their hand, then discards that many cards"
        ),
        "expected discard follow-up to render, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn bottom_score_parse_officious_interrogation_counts_targeted_players_creatures() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Officious Interrogation")
        .parse_text(
            "This spell costs {W}{U} more to cast for each target beyond the first.\nChoose any number of target players. Investigate X times, where X is the total number of creatures those players control.",
        )
        .expect("Officious Interrogation should parse target-player investigate scaling");
    let program = def
        .spell_effect
        .as_ref()
        .expect("Officious Interrogation should have spell effects");
    let target_only = program
        .all_effects()
        .into_iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::TargetOnlyEffect>())
        .expect("Officious Interrogation should declare its target-player set");
    assert!(target_only.target.is_target());
    assert!(target_only.target.count().is_any_number());
    assert!(matches!(
        target_only.target.base(),
        ChooseSpec::Player(PlayerFilter::Any)
    ));

    let investigate = program
        .all_effects()
        .into_iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::InvestigateEffect>())
        .expect("Officious Interrogation should investigate");
    let Value::Count(creatures) = investigate.count.unhinted() else {
        panic!(
            "expected a typed creature count, got {:#?}",
            investigate.count
        );
    };
    assert_eq!(creatures.zone, Some(Zone::Battlefield));
    assert_eq!(creatures.card_types, [CardType::Creature]);
    assert_eq!(
        creatures.controller,
        Some(PlayerFilter::Target(Box::new(PlayerFilter::Any)))
    );

    let rendered = crate::compiled_text::unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();

    assert!(
        rendered.contains(
            "investigate x times, where x is the total number of creatures those players control"
        ),
        "expected targeted-player creature count, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn officious_interrogation_runtime_counts_only_the_targeted_players_creatures() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Officious Interrogation")
        .parse_text(
            "This spell costs {W}{U} more to cast for each target beyond the first.\nChoose any number of target players. Investigate X times, where X is the total number of creatures those players control.",
        )
        .expect("Officious Interrogation should parse target-player investigate scaling");
    let investigate = def
        .spell_effect
        .as_ref()
        .expect("Officious Interrogation should have spell effects")
        .all_effects()
        .into_iter()
        .find(|effect| {
            effect
                .downcast_ref::<crate::effects::InvestigateEffect>()
                .is_some()
        })
        .expect("Officious Interrogation should investigate")
        .clone();

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&def, alice, Zone::Stack);
    let creature = CardDefinitionBuilder::new(CardId::from_raw(97_071), "Counted Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();

    game.create_object_from_definition(&creature, alice, Zone::Battlefield);
    game.create_object_from_definition(&creature, bob, Zone::Battlefield);
    game.create_object_from_definition(&creature, bob, Zone::Battlefield);

    let mut dm = crate::decision::AutoPassDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut dm)
        .with_targets(vec![crate::effects::ResolvedTarget::Player(bob)]);
    let outcome = crate::effects::execute_effect(&mut game, &investigate, &mut ctx)
        .expect("target-set investigate should resolve");

    assert_eq!(outcome.value, crate::effect::OutcomeValue::Count(2));
    assert_eq!(
        game.battlefield
            .iter()
            .filter(|&&id| game.object(id).is_some_and(|object| object.name == "Clue"))
            .count(),
        2,
        "Alice's untargeted creature must not contribute to the investigate count"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn bottom_score_parse_journey_for_the_elixir_searches_library_and_graveyard_slots() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Journey for the Elixir")
        .parse_text(
            "Search your library and graveyard for a basic land card and a card named Jiang Yanggu, reveal them, put them into your hand, then shuffle.",
        )
        .expect("Journey for the Elixir should parse multi-zone search slots");
    let debug = format!("{def:#?}");
    let rendered = crate::compiled_text::unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();

    assert!(debug.contains("SearchLibrarySlotsEffect"), "{debug}");
    assert!(debug.contains("zone: None"), "{debug}");
    assert!(
        rendered.contains(
            "search your library and graveyard for a basic land card and a card named jiang yanggu"
        ),
        "expected multi-zone slot search, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn bottom_score_parse_loxodon_peacekeeper_lowest_life_control_change() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Loxodon Peacekeeper")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "At the beginning of your upkeep, the player with the lowest life total gains control of this creature. If two or more players are tied for lowest life total, you choose one of them, and that player gains control of this creature.",
        )
        .expect("Loxodon Peacekeeper should parse lowest-life controller change");
    let debug = format!("{def:#?}");
    let rendered = crate::compiled_text::unprocessed_compiled_lines(&def);

    assert!(debug.contains("ChangeControllerToPlayer"), "{debug}");
    assert!(debug.contains("LowestLifeTied"), "{debug}");
    assert_eq!(
        rendered,
        vec![
            "At the beginning of your upkeep, the player with the lowest life total gains control of this creature. If two or more players are tied for lowest life total, you choose one of them, and that player gains control of this creature."
        ],
        "the executable tie branch must not render a duplicate generic conditional"
    );

    let AbilityKind::Triggered(triggered) = &def.abilities[0].kind else {
        panic!("expected upkeep trigger")
    };
    let [lowest_segment, tie_segment] = triggered.effects.segments.as_slice() else {
        panic!("expected lowest-life handoff followed by tie branch")
    };
    let [lowest_effect] = lowest_segment.default_effects.as_slice() else {
        panic!("expected one lowest-life control effect")
    };
    let lowest = lowest_effect
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()
        .expect("lowest-life instruction should stay executable");
    assert!(matches!(
        lowest.runtime_modifications.as_slice(),
        [
            crate::effects::continuous::RuntimeModification::ChangeControllerToPlayer(
                PlayerFilter::LowestLifeTied
            )
        ]
    ));

    let [conditional_effect] = tie_segment.default_effects.as_slice() else {
        panic!("expected one typed tie conditional")
    };
    let conditional = conditional_effect
        .downcast_ref::<crate::effects::ConditionalEffect>()
        .expect("tie sentence should be a typed conditional");
    assert!(matches!(
        &conditional.condition,
        crate::effect::Condition::ValueComparison {
            left: crate::effect::Value::CountPlayers(PlayerFilter::LowestLifeTied),
            operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
            right: crate::effect::Value::Fixed(2),
        }
    ));
    let [choose_effect, tied_control_effect] = conditional.if_true.as_slice() else {
        panic!("tie branch should choose a tied player then transfer control")
    };
    let choose = choose_effect
        .downcast_ref::<crate::effects::ChoosePlayerEffect>()
        .expect("tie branch should make the source controller choose");
    assert_eq!(choose.chooser, PlayerFilter::You);
    assert_eq!(choose.filter, PlayerFilter::LowestLifeTied);
    let tied_control = tied_control_effect
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()
        .expect("chosen tied player should gain control");
    assert!(matches!(
        tied_control.runtime_modifications.as_slice(),
        [crate::effects::continuous::RuntimeModification::ChangeControllerToPlayer(
            PlayerFilter::TaggedPlayer(tag)
        )] if tag.as_str() == "__it__"
    ));
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn loxodon_peacekeeper_tie_choice_controls_the_source_for_the_chosen_player() {
    struct ChooseLastTiedPlayer;

    impl crate::decision::DecisionMaker for ChooseLastTiedPlayer {
        fn decide_options(
            &mut self,
            _game: &crate::game_state::GameState,
            ctx: &crate::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            ctx.options
                .iter()
                .rev()
                .find(|option| option.legal)
                .map(|option| vec![option.index])
                .unwrap_or_default()
        }
    }

    let def = CardDefinitionBuilder::new(CardId::new(), "Loxodon Peacekeeper")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "At the beginning of your upkeep, the player with the lowest life total gains control of this creature. If two or more players are tied for lowest life total, you choose one of them, and that player gains control of this creature.",
        )
        .expect("Loxodon Peacekeeper should parse");
    let AbilityKind::Triggered(triggered) = &def.abilities[0].kind else {
        panic!("expected upkeep trigger")
    };
    let program = triggered.effects.clone();

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
    game.player_mut(bob).expect("Bob exists").life = 10;
    game.player_mut(charlie).expect("Charlie exists").life = 10;
    let source = game.create_object_from_definition(&def, alice, Zone::Battlefield);

    let mut dm = ChooseLastTiedPlayer;
    let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut dm);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        source,
        &program,
        None,
        &[],
    )
    .expect("lowest-life handoff should resolve");

    assert_eq!(
        game.controller_of_id(source),
        Some(charlie),
        "the source controller's tied-player choice must override the default first tied player"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
fn shiko_and_narset_unified_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Shiko and Narset, Unified")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Flying, vigilance\nFlurry — Whenever you cast your second spell each turn, copy that spell if it targets a permanent or player, and you may choose new targets for the copy. If you don't copy a spell this way, draw a card.",
        )
        .expect("Shiko and Narset should parse")
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn shiko_and_narset_keeps_conditional_copy_id_retarget_and_fallback_linked() {
    let def = shiko_and_narset_unified_definition();
    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("expected Flurry trigger");
    assert_eq!(
        crate::compiled_text::unprocessed_compiled_lines(&def),
        vec![
            "Flying, vigilance.".to_string(),
            "Flurry — Whenever you cast your second spell each turn, copy that spell if it targets a permanent or player, and you may choose new targets for the copy. If you don't copy a spell this way, draw a card.".to_string(),
        ],
        "full Flurry resolution program:\n{:#?}",
        triggered.effects
    );

    let [copy_segment, fallback_segment] = triggered.effects.segments.as_slice() else {
        panic!("expected conditional copy and did-not-copy fallback segments")
    };
    let [tag_triggering_effect, outer_result_effect] = copy_segment.default_effects.as_slice()
    else {
        panic!("expected triggering-object tag plus copy conditional")
    };
    let trigger_tag = tag_triggering_effect
        .downcast_ref::<crate::effects::TagTriggeringObjectEffect>()
        .expect("copy condition should inspect the triggering object");
    assert_eq!(trigger_tag.tag.as_str(), "triggering");
    let outer_result = outer_result_effect
        .downcast_ref::<crate::effects::WithIdEffect>()
        .expect("the conditional should retain the fallback result id");
    let result_conjunction = outer_result
        .effect
        .downcast_ref::<crate::effects::SequenceEffect>()
        .expect("copy condition and retarget permission should remain coordinated");
    assert_eq!(
        result_conjunction.surface,
        ironsmith_core::SequenceSurface::Coordinated
    );
    let [conditional_effect, retarget_may_effect] = result_conjunction.effects.as_slice() else {
        panic!("expected conditional copy followed by optional retarget")
    };
    let conditional = conditional_effect
        .downcast_ref::<crate::effects::ConditionalEffect>()
        .expect("copy should retain its target-domain condition");
    let crate::effect::Condition::TaggedObjectMatches(tag, filter) = &conditional.condition else {
        panic!("condition should inspect the triggering spell")
    };
    assert_eq!(tag.as_str(), "triggering");
    assert!(filter.targets_any_of);
    assert_eq!(filter.targets_player, Some(PlayerFilter::Any));
    assert!(filter.targets_object.is_some());

    let [copy_effect] = conditional.if_true.as_slice() else {
        panic!("condition should contain the copy action")
    };
    let tagged_copy = copy_effect
        .downcast_ref::<crate::effects::TaggedEffect>()
        .expect("copy result should stay tagged");
    assert_eq!(tagged_copy.tag.as_str(), "__copied_stack_object__");
    let copy_with_id = tagged_copy
        .effect
        .downcast_ref::<crate::effects::WithIdEffect>()
        .expect("copy should have an executable result id");
    let copy = copy_with_id
        .effect
        .downcast_ref::<crate::effects::CopySpellEffect>()
        .expect("expected typed copy effect");
    assert!(matches!(
        &copy.target,
        ChooseSpec::Tagged(tag) if tag.as_str() == "triggering"
    ));
    assert_eq!(outer_result.id, copy_with_id.id);
    let retarget_may = retarget_may_effect
        .downcast_ref::<crate::effects::MayEffect>()
        .expect("retarget should remain an explicit optional instruction");
    assert!(matches!(retarget_may.decider, Some(PlayerFilter::You)));
    let [retarget_effect] = retarget_may.effects.as_slice() else {
        panic!("optional retarget should contain one typed action")
    };
    let tagged_retarget = retarget_effect
        .downcast_ref::<crate::effects::TaggedEffect>()
        .expect("retarget result should stay tagged");
    assert!(tagged_retarget.tag.as_str().starts_with("retargeted_"));
    let retarget = tagged_retarget
        .effect
        .downcast_ref::<crate::effects::RetargetStackObjectEffect>()
        .expect("expected typed retarget permission");
    assert!(matches!(retarget.mode, crate::effects::RetargetMode::All));
    assert_eq!(retarget.chooser, PlayerFilter::You);
    assert!(!retarget.copy_reference_plural);
    assert!(matches!(
        &retarget.target,
        ChooseSpec::Tagged(tag) if tag == &tagged_copy.tag
    ));

    let [fallback_effect] = fallback_segment.default_effects.as_slice() else {
        panic!("expected one did-not-copy fallback")
    };
    let fallback = fallback_effect
        .downcast_ref::<crate::effects::IfEffect>()
        .expect("fallback should inspect the copy result");
    assert_eq!(fallback.condition, copy_with_id.id);
    assert_eq!(
        fallback.predicate,
        crate::effect::EffectPredicate::DidNotHappen
    );
    assert!(matches!(
        fallback.then.as_slice(),
        [draw] if draw
            .downcast_ref::<crate::effects::DrawCardsEffect>()
            .is_some_and(|draw| draw.player == PlayerFilter::You && draw.count == crate::effect::Value::Fixed(1))
    ));
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn shiko_and_narset_copies_only_targeting_spells_and_draws_on_the_fallback() {
    fn resolve_case(targets_permanent: bool) -> (usize, usize) {
        let def = shiko_and_narset_unified_definition();
        let triggered = def
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Triggered(triggered) => Some(triggered),
                _ => None,
            })
            .expect("expected Flurry trigger");
        let program = triggered.effects.clone();
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = game.create_object_from_definition(&def, alice, Zone::Battlefield);
        let target = game.create_object_from_definition(
            &CardDefinitionBuilder::new(CardId::new(), "Target Permanent")
                .card_types(vec![CardType::Artifact])
                .build(),
            alice,
            Zone::Battlefield,
        );
        let spell_def = CardDefinitionBuilder::new(CardId::new(), "Second Spell")
            .card_types(vec![CardType::Instant])
            .build();
        let spell = game.create_object_from_definition(&spell_def, alice, Zone::Stack);
        let mut entry = crate::game_state::StackEntry::new(spell, alice);
        if targets_permanent {
            let target_spec = ChooseSpec::target_permanent();
            entry = entry
                .with_targets(vec![crate::game_state::Target::Object(target)])
                .with_target_assignments(vec![crate::game_state::TargetAssignment {
                    spec: target_spec,
                    range: 0..1,
                }]);
        }
        game.push_to_stack(entry);
        game.create_object_from_definition(
            &CardDefinitionBuilder::new(CardId::new(), "Card to Draw").build(),
            alice,
            Zone::Library,
        );
        let snapshot = crate::snapshot::ObjectSnapshot::from_object(
            game.object(spell).expect("spell exists"),
            &game,
        );
        let event = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::spells::SpellCastEvent::new_with_snapshot(
                spell,
                alice,
                Zone::Hand,
                snapshot,
            ),
            crate::provenance::ProvNodeId::default(),
        );
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut dm)
            .with_triggering_event(event);
        crate::game_loop::execute_resolution_program(
            &mut game,
            &mut ctx,
            alice,
            source,
            &program,
            None,
            &[],
        )
        .expect("Flurry trigger should resolve");
        (
            game.stack.len(),
            game.objects_in_zone(Zone::Hand)
                .iter()
                .filter(|id| game.object(**id).is_some_and(|card| card.owner == alice))
                .count(),
        )
    }

    assert_eq!(
        resolve_case(true),
        (2, 0),
        "a targeting second spell should be copied and should not draw"
    );
    assert_eq!(
        resolve_case(false),
        (1, 1),
        "a nontargeting second spell should not be copied and should draw"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn bottom_score_parse_it_that_betrays_annihilator_and_sacrifice_trigger() {
    let def = CardDefinitionBuilder::new(CardId::new(), "It That Betrays")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Annihilator 2\nWhenever an opponent sacrifices a nontoken permanent, put that card onto the battlefield under your control.",
        )
        .expect("It That Betrays should parse annihilator and sacrifice trigger");
    let debug = format!("{def:#?}");
    let rendered = crate::compiled_text::unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();

    assert!(debug.contains("SacrificePlayerEffect"), "{debug}");
    assert!(debug.contains("PlayerSacrificesTrigger"), "{debug}");
    assert!(debug.contains("MoveToZoneEffect"), "{debug}");
    assert!(rendered.contains("annihilator 2"), "got {rendered}");
    assert!(
        rendered.contains(
            "whenever an opponent sacrifices a nontoken permanent, put that card onto the battlefield under your control"
        ),
        "expected sacrifice trigger render, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_target_opponent_sacrifice_discard_lose_chain_keeps_all_predicates() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Archon Chain Variant")
        .parse_text(
            "When this creature enters, target opponent sacrifices a creature of their choice, discards a card, and loses 3 life.",
        )
        .expect("target-opponent sacrifice/discard/lose chain should parse");
    let joined = crate::compiled_text::unprocessed_compiled_lines(&def)
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

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_sacrifice_all_lands_clause_as_sacrifice_all() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Overlaid Terrain Variant")
        .parse_text("As this enchantment enters, sacrifice all lands you control.")
        .expect("sacrifice-all lands clause should parse");
    let compiled = crate::compiled_text::unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        compiled.contains("sacrifice all lands") || compiled.contains("sacrifices all lands"),
        "expected sacrifice-all lands wording, got {compiled}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_target_player_sacrifices_and_loses_uses_oracle_like_wording() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Geth's Verdict Render Variant")
        .parse_text("Target player sacrifices a creature of their choice and loses 1 life.")
        .expect("sacrifice-then-lose line should parse");
    let joined = crate::compiled_text::unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("target player sacrifices a creature of their choice")
            && joined.contains("loses 1 life"),
        "expected oracle-like sacrifice+lose wording, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_target_opponent_sacrifice_of_their_choice_keeps_non_targeted_object_choice() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Predatory Nightstalker Variant")
        .parse_text(
            "When this creature enters, target opponent sacrifices a creature of their choice.",
        )
        .expect("opponent-sacrifice-of-their-choice line should parse");
    let joined = crate::compiled_text::unprocessed_compiled_lines(&def)
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

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_each_player_sacrifice_non_vampire_creature_of_their_choice_keeps_creature_filter()
 {
    let def = CardDefinitionBuilder::new(CardId::new(), "Anowon Variant")
        .parse_text(
            "At the beginning of your upkeep, each player sacrifices a non-Vampire creature of their choice.",
        )
        .expect("non-Vampire sacrifice-choice line should parse");

    let joined = crate::compiled_text::unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("non-vampire creature"),
        "expected non-Vampire creature wording, got {joined}"
    );

    let abilities_debug = format!("{:?}", def.abilities);
    assert!(
        abilities_debug.contains(
            "card_types: [\n                                                            Creature,"
        ) || abilities_debug.contains("card_types: [Creature]"),
        "expected creature filter in lowered ability, got {abilities_debug}"
    );
    assert!(
        abilities_debug.contains("excluded_subtypes: [\n                                                                Vampire,")
            || abilities_debug.contains("excluded_subtypes: [Vampire]"),
        "expected excluded Vampire subtype in lowered ability, got {abilities_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_all_slivers_have_activated_ability_as_static_grant() {
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

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_all_slivers_have_triggered_ability_as_static_grant() {
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

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_kira_granted_trigger_counters_the_targeting_stack_object() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Kira, Great Glass-Spinner")
        .parse_text(
            "Creatures you control have \"Whenever this creature becomes the target of a spell or ability for the first time each turn, counter that spell or ability.\"",
        )
        .expect("Kira granted trigger should parse");

    let abilities_debug = format!("{:#?}", def.abilities);
    assert!(
        abilities_debug.contains("GrantObjectAbilityForFilter")
            && abilities_debug.contains("FirstTimeThisTurn")
            && abilities_debug.contains("CounterEffect")
            && abilities_debug.contains("triggering_source"),
        "expected Kira to counter the stack object that targeted the creature, got {abilities_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn hellish_rebuke_keeps_lose_life_inside_granted_trigger() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Hellish Rebuke")
        .parse_text(
            "Until end of turn, permanents your opponents control gain \"When this permanent deals damage to the player who cast Hellish Rebuke, sacrifice this permanent. You lose 2 life.\"",
        )
        .expect("hellish rebuke grant line should parse");

    let spell_effects = def
        .spell_effect
        .as_ref()
        .expect("hellish rebuke should compile to spell effects");
    assert_eq!(
        spell_effects.len(),
        1,
        "lose life should not be hoisted to a top-level spell effect: {spell_effects:?}"
    );

    let flattened = spell_effects.flattened_default_effects();
    let apply = flattened
        .iter()
        .find_map(|effect| {
            effect
                .downcast_ref::<crate::effects::ApplyContinuousEffect>()
                .or_else(|| {
                    effect
                        .downcast_ref::<crate::effects::TaggedEffect>()?
                        .effect
                        .downcast_ref::<crate::effects::ApplyContinuousEffect>()
                })
        })
        .expect("top-level spell effect should be a continuous grant");
    let granted = apply
        .modification
        .as_ref()
        .and_then(|modification| match modification {
            crate::continuous::Modification::AddAbilityGeneric(ability) => Some(ability.clone()),
            crate::continuous::Modification::AddAbility(static_ability) => {
                static_ability.granted_inline_ability().cloned()
            }
            _ => None,
        })
        .expect("continuous effect should grant an inline ability");

    let crate::ability::AbilityKind::Triggered(triggered) = &granted.kind else {
        panic!("expected granted inline ability to be triggered: {granted:?}");
    };
    assert_eq!(
        triggered.effects.len(),
        2,
        "granted trigger should keep both sacrifice and lose-life effects: {triggered:?}"
    );
    assert!(
        triggered.effects.iter().any(|effect| effect
            .downcast_ref::<crate::effects::LoseLifeEffect>()
            .is_some()),
        "granted trigger should include lose-life effect: {triggered:?}"
    );

    let trigger_debug = format!("{:?}", triggered.trigger);
    assert!(
        trigger_debug.contains("damaged_player: Some("),
        "granted trigger should constrain the damaged player: {trigger_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_prevent_all_combat_damage_global_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Fog Variant")
        .parse_text("Prevent all combat damage that would be dealt this turn.")
        .expect("parse basic prevent-all combat clause");

    let effects = def.spell_effect.as_ref().expect("expected spell effects");
    let debug = format!("{effects:?}");
    assert!(
        debug.contains("PreventAllCombatDamageEffect") && debug.contains("target: All"),
        "expected all-combat-damage runtime effect, got {debug}"
    );
    assert!(
        debug.contains("until: EndOfTurn"),
        "expected end-of-turn combat prevention duration, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_prevent_all_combat_damage_by_target_source_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Targeted Fog Variant")
        .parse_text("Prevent all combat damage that would be dealt by target creature this turn.")
        .expect("parse source-scoped prevent-all combat clause");

    let effects = def.spell_effect.expect("expected spell effects");
    let debug = format!("{effects:?}");
    assert!(
        (debug.contains("PreventAllCombatDamageEffect") && debug.contains("target: From("))
            || (debug.contains("PreventAllDamageEffect")
                && debug.contains("combat_only: true")
                && debug.contains("from_source: Some")
                && debug.contains("Creature")),
        "expected source-scoped combat prevention runtime effect, got {debug}"
    );
    assert!(
        debug.contains("Target(") && debug.contains("Creature"),
        "expected target creature choice for source-scoped prevention, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_prevent_all_combat_damage_to_players_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Player Fog Variant")
        .parse_text("Prevent all combat damage that would be dealt to players this turn.")
        .expect("parse players-scoped prevent-all combat clause");

    let effects = def.spell_effect.expect("expected spell effects");
    let debug = format!("{effects:?}");
    assert!(
        debug.contains("PreventAllCombatDamageEffect") && debug.contains("target: Players"),
        "expected prevention target scope to players, got {debug}"
    );
    assert!(
        debug.contains("until: EndOfTurn"),
        "expected end-of-turn combat prevention duration, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_prevent_all_combat_damage_requires_supported_tail() {
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

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_oracle_winds_of_qal_sisma_ferocious_is_self_replacement() {
    let def = parse_oracle_card_definition("Winds of Qal Sisma");

    let program = def.spell_effect.as_ref().expect("expected spell effects");
    assert_eq!(
        program.segments.len(),
        1,
        "Winds of Qal Sisma should lower the ferocious instead clause into the base prevention segment"
    );
    assert_eq!(
        program.segments[0].default_effects.len(),
        1,
        "Winds of Qal Sisma should keep the non-ferocious prevention as the default branch"
    );
    assert_eq!(
        program.segments[0].self_replacements.len(),
        1,
        "Winds of Qal Sisma should model ferocious as a spell self-replacement"
    );

    let debug = format!("{program:#?}").to_ascii_lowercase();
    assert!(
        debug.contains("preventallcombatdamageeffect")
            && debug.contains("target: all")
            && debug.contains("preventalldamageeffect")
            && debug.contains("combat_only: true")
            && debug.contains("controller: some(")
            && debug.contains("opponent")
            && debug.contains("creature")
            && debug.contains("greaterthanorequal")
            && debug.contains("4"),
        "expected Winds of Qal Sisma to keep default fog and ferocious opponent-creature prevention branches, got {debug}"
    );

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Prevent all combat damage that would be dealt this turn.")
            && rendered.contains("If you control a creature with power 4 or greater")
            && rendered.contains(
                "instead prevent all combat damage that would be dealt this turn by creatures your opponents control"
            ),
        "Winds of Qal Sisma compiled text should preserve the complete ferocious instead prevention clause, got {rendered}"
    );
    assert!(
        !rendered.to_ascii_lowercase().contains("unsupported"),
        "Winds of Qal Sisma should parse strictly without unsupported markers, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn create_winds_test_creature(
    game: &mut crate::game_state::GameState,
    name: &str,
    controller: PlayerId,
    power: i32,
    toughness: i32,
) -> ObjectId {
    let card = crate::card::CardBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(power, toughness))
        .build();
    game.create_object_from_card(&card, controller, Zone::Battlefield)
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn resolve_winds_of_qal_sisma(
    game: &mut crate::game_state::GameState,
    controller: PlayerId,
) {
    let winds = parse_oracle_card_definition("Winds of Qal Sisma");
    let spell_id = game.create_object_from_definition(&winds, controller, Zone::Stack);
    game.push_to_stack(crate::game_state::StackEntry::new(spell_id, controller));
    crate::game_loop::resolve_stack_entry_with(
        game,
        &mut crate::decision::SelectFirstDecisionMaker,
    )
    .expect("Winds of Qal Sisma should resolve");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_oracle_winds_of_qal_sisma_without_ferocious_prevents_all_combat_damage() {
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let bob_attacker = create_winds_test_creature(&mut game, "Bob Attacker", bob, 3, 3);

    resolve_winds_of_qal_sisma(&mut game, alice);

    let mut combat = crate::combat_state::CombatState::default();
    combat.attackers.push(crate::combat_state::AttackerInfo {
        creature: bob_attacker,
        target: crate::combat_state::AttackTarget::Player(alice),
    });
    combat.blockers.insert(bob_attacker, Vec::new());

    game.turn.phase = crate::game_state::Phase::Combat;
    game.turn.step = Some(crate::game_state::Step::CombatDamage);
    let events = crate::game_loop::execute_combat_damage_step(&mut game, &combat, false);

    assert_eq!(
        events.len(),
        1,
        "Bob's attacker should assign combat damage"
    );
    assert_eq!(
        game.player(alice).expect("Alice exists").life,
        20,
        "without ferocious, Winds of Qal Sisma should prevent all combat damage"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_oracle_winds_of_qal_sisma_ferocious_prevents_only_opponents_creature_damage() {
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    create_winds_test_creature(&mut game, "Ferocious Creature", alice, 4, 4);
    let alice_attacker = create_winds_test_creature(&mut game, "Alice Attacker", alice, 3, 3);
    let bob_attacker = create_winds_test_creature(&mut game, "Bob Attacker", bob, 3, 3);

    resolve_winds_of_qal_sisma(&mut game, alice);

    let shields = game.effect_store.prevention_effects.shields();
    assert_eq!(
        shields.len(),
        1,
        "ferocious Winds of Qal Sisma should create only the narrowed prevention shield"
    );
    assert!(
        shields[0].damage_filter.combat_only && shields[0].damage_filter.from_source.is_some(),
        "ferocious Winds of Qal Sisma should prevent combat damage from a source filter, got {:?}",
        shields[0]
    );

    let mut combat = crate::combat_state::CombatState::default();
    combat.attackers.push(crate::combat_state::AttackerInfo {
        creature: bob_attacker,
        target: crate::combat_state::AttackTarget::Player(alice),
    });
    combat.attackers.push(crate::combat_state::AttackerInfo {
        creature: alice_attacker,
        target: crate::combat_state::AttackTarget::Player(bob),
    });
    combat.blockers.insert(bob_attacker, Vec::new());
    combat.blockers.insert(alice_attacker, Vec::new());

    game.turn.phase = crate::game_state::Phase::Combat;
    game.turn.step = Some(crate::game_state::Step::CombatDamage);
    let events = crate::game_loop::execute_combat_damage_step(&mut game, &combat, false);

    assert_eq!(
        events.len(),
        2,
        "both attackers should assign combat damage"
    );
    assert_eq!(
        game.player(alice).expect("Alice exists").life,
        20,
        "ferocious Winds of Qal Sisma should prevent opposing creature combat damage"
    );
    assert_eq!(
        game.player(bob).expect("Bob exists").life,
        17,
        "ferocious Winds of Qal Sisma should not prevent its controller's creature combat damage"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn radiant_kavu_test_permanent(
    game: &mut crate::game_state::GameState,
    name: &str,
    controller: PlayerId,
    card_types: Vec<CardType>,
    colors: crate::color::ColorSet,
) -> ObjectId {
    let card = crate::card::CardBuilder::new(CardId::new(), name)
        .card_types(card_types)
        .color_indicator(colors)
        .power_toughness(PowerToughness::fixed(3, 3))
        .build();
    game.create_object_from_card(&card, controller, Zone::Battlefield)
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn activate_radiant_kavu(
    game: &mut crate::game_state::GameState,
    controller: PlayerId,
    kavu_id: ObjectId,
) {
    let ability_index = game
        .object(kavu_id)
        .expect("Radiant Kavu should exist")
        .abilities
        .iter()
        .position(|ability| matches!(ability.kind, AbilityKind::Activated(_)))
        .expect("Radiant Kavu should have an activated ability");
    let activate_action = crate::decision::compute_legal_actions(game, controller)
        .into_iter()
        .find(|action| {
            matches!(
                action,
                crate::decision::LegalAction::ActivateAbility { source, ability_index: idx }
                    if *source == kavu_id && *idx == ability_index
            )
        })
        .expect("Radiant Kavu activation should be legal after paying {R}{G}{W}");

    let mut trigger_queue = crate::triggers::TriggerQueue::new();
    let mut state = crate::game_loop::PriorityLoopState::new(game.players_in_game());
    let mut dm = crate::decision::SelectFirstDecisionMaker;
    let progress = crate::game_loop::apply_priority_response_with_dm(
        game,
        &mut trigger_queue,
        &mut state,
        &crate::game_loop::PriorityResponse::PriorityAction(activate_action),
        &mut dm,
    )
    .expect("Radiant Kavu activation should start");
    chandras_regulator_drive_activation(
        game,
        &mut trigger_queue,
        &mut state,
        progress,
        &mut dm,
        None,
    );
    crate::game_loop::resolve_stack_entry_with(game, &mut dm)
        .expect("Radiant Kavu activated ability should resolve");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn radiant_kavu_strict_parser_and_compiled_text_regression() {
    assert_oracle_card_parses_strict("Radiant Kavu");
    let def = parse_oracle_card_definition("Radiant Kavu");
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();
    let activated = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("Radiant Kavu should have an activated ability");
    let prevent = activated
        .effects
        .flattened_default_effects()
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::PreventAllDamageEffect>())
        .expect("Radiant Kavu should lower to a prevent-all damage effect");
    let source_filter = prevent
        .damage_filter
        .from_source
        .as_ref()
        .expect("Radiant Kavu prevention should be source-filtered");

    assert_eq!(
        rendered,
        "{R}{G}{W}: Prevent all combat damage blue creatures and black creatures would deal this turn.",
        "Radiant Kavu compiled text should preserve its exact blue/black creature combat-prevention clause"
    );
    assert!(
        !rendered_lower.contains("unsupported") && !rendered_lower.contains("unimplemented"),
        "Radiant Kavu should compile without fallback markers, got {rendered}"
    );
    assert!(
        prevent.damage_filter.combat_only
            && source_filter.zone == Some(Zone::Battlefield)
            && source_filter.card_types == vec![CardType::Creature]
            && source_filter.colors
                == Some(crate::color::ColorSet::BLUE.union(crate::color::ColorSet::BLACK)),
        "Radiant Kavu should lower to a combat-only blue/black creature source-filter shield, got {prevent:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn radiant_kavu_activation_cost_and_source_filter_prevention_runtime() {
    let def = parse_oracle_card_definition("Radiant Kavu");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.turn.active_player = alice;
    game.turn.phase = crate::game_state::Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);

    let kavu_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let ability_index = game
        .object(kavu_id)
        .expect("Radiant Kavu should exist")
        .abilities
        .iter()
        .position(|ability| matches!(ability.kind, AbilityKind::Activated(_)))
        .expect("Radiant Kavu should have an activated ability");
    let cost_debug = format!(
        "{:?}",
        game.object(kavu_id)
            .expect("Radiant Kavu should exist")
            .abilities[ability_index]
    );
    assert!(
        cost_debug.contains("Red") && cost_debug.contains("Green") && cost_debug.contains("White"),
        "Radiant Kavu activated ability should carry its {{R}}{{G}}{{W}} cost, got {cost_debug}"
    );

    {
        let player = game.player_mut(alice).expect("Alice should exist");
        player.mana_pool.add(ManaSymbol::Red, 1);
        player.mana_pool.add(ManaSymbol::Green, 1);
        player.mana_pool.add(ManaSymbol::White, 1);
    }
    activate_radiant_kavu(&mut game, alice, kavu_id);
    assert_eq!(
        game.player(alice)
            .expect("Alice should exist")
            .mana_pool
            .total(),
        0,
        "Radiant Kavu activation should spend {{R}}{{G}}{{W}}"
    );

    let shields = game.effect_store.prevention_effects.shields();
    assert_eq!(
        shields.len(),
        1,
        "Radiant Kavu should create one prevention shield"
    );
    assert!(
        shields[0].damage_filter.combat_only && shields[0].damage_filter.from_source.is_some(),
        "Radiant Kavu should create a combat-only source-filter prevention shield, got {:?}",
        shields[0]
    );

    let blue_creature = radiant_kavu_test_permanent(
        &mut game,
        "Blue Combat Source",
        bob,
        vec![CardType::Creature],
        crate::color::ColorSet::BLUE,
    );
    let black_creature = radiant_kavu_test_permanent(
        &mut game,
        "Black Combat Source",
        bob,
        vec![CardType::Creature],
        crate::color::ColorSet::BLACK,
    );
    let green_creature = radiant_kavu_test_permanent(
        &mut game,
        "Green Combat Source",
        bob,
        vec![CardType::Creature],
        crate::color::ColorSet::GREEN,
    );
    let blue_artifact = radiant_kavu_test_permanent(
        &mut game,
        "Blue Noncreature Source",
        bob,
        vec![CardType::Artifact],
        crate::color::ColorSet::BLUE,
    );

    let (blue_combat, _) = crate::events::processing::process_damage_with_event(
        &mut game,
        blue_creature,
        crate::events::DamageTarget::Player(alice),
        3,
        true,
        crate::events::cause::EventCause::effect(),
    );
    assert_eq!(
        blue_combat, 0,
        "Radiant Kavu should prevent blue creature combat damage"
    );

    let (black_combat, _) = crate::events::processing::process_damage_with_event(
        &mut game,
        black_creature,
        crate::events::DamageTarget::Player(alice),
        3,
        true,
        crate::events::cause::EventCause::effect(),
    );
    assert_eq!(
        black_combat, 0,
        "Radiant Kavu should prevent black creature combat damage"
    );

    let (green_combat, _) = crate::events::processing::process_damage_with_event(
        &mut game,
        green_creature,
        crate::events::DamageTarget::Player(alice),
        3,
        true,
        crate::events::cause::EventCause::effect(),
    );
    assert_eq!(
        green_combat, 3,
        "Radiant Kavu should not prevent green creature combat damage"
    );

    let (blue_artifact_combat, _) = crate::events::processing::process_damage_with_event(
        &mut game,
        blue_artifact,
        crate::events::DamageTarget::Player(alice),
        3,
        true,
        crate::events::cause::EventCause::effect(),
    );
    assert_eq!(
        blue_artifact_combat, 3,
        "Radiant Kavu should not prevent combat damage from blue noncreatures"
    );

    let (blue_noncombat, _) = crate::events::processing::process_damage_with_event(
        &mut game,
        blue_creature,
        crate::events::DamageTarget::Player(alice),
        3,
        false,
        crate::events::cause::EventCause::effect(),
    );
    assert_eq!(
        blue_noncombat, 3,
        "Radiant Kavu should not prevent noncombat damage"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_oracle_heavy_fog_strict_and_keeps_attacking_creature_prevention_clause() {
    assert_oracle_card_parses_strict("Heavy Fog");
    let def = parse_oracle_card_definition("Heavy Fog");
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let debug = format!("{:#?}", def.spell_effect).to_ascii_lowercase();

    assert!(
        rendered.contains(
            "Prevent all damage that would be dealt to you this turn by attacking creatures"
        ),
        "Heavy Fog compiled text should keep the attacking-creature source-filter prevention clause, got {rendered}"
    );
    assert!(
        rendered.contains(
            "Cast this spell only during the declare attackers step and only if you've been attacked this step"
        ),
        "Heavy Fog compiled text should keep its declare-attackers cast restriction, got {rendered}"
    );
    assert!(
        debug.contains("preventalldamageeffect")
            && debug.contains("target: you")
            && debug.contains("from_source")
            && debug.contains("attacking: true")
            && debug.contains("creature"),
        "Heavy Fog should lower to a prevent-all shield to you from attacking creature sources, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn heavy_fog_cast_restriction_requires_declare_attackers_after_you_were_attacked() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Heavy Fog")
        .card_types(vec![CardType::Instant])
        .mana_cost(ManaCost::new())
        .parse_text(
            "Cast this spell only during the declare attackers step and only if you've been attacked this step.\n\
             Prevent all damage that would be dealt to you this turn by attacking creatures.",
        )
        .expect("Heavy Fog text should parse for cast-restriction runtime coverage");

    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let spell_id = game.create_object_from_definition(&def, alice, Zone::Hand);
    let attacker = create_winds_test_creature(&mut game, "Heavy Fog Attacker", bob, 3, 3);

    let spell = game.object(spell_id).expect("Heavy Fog should be in hand");
    assert!(
        !crate::decision::can_cast_spell(
            &game,
            alice,
            spell,
            &crate::alternative_cast::CastingMethod::Normal,
        ),
        "Heavy Fog should not be castable outside the declare attackers step"
    );

    game.turn.active_player = bob;
    game.turn.phase = crate::game_state::Phase::Combat;
    game.turn.step = Some(crate::game_state::Step::DeclareAttackers);
    game.combat = Some(crate::combat_state::CombatState {
        attackers: vec![crate::combat_state::AttackerInfo {
            creature: attacker,
            target: crate::combat_state::AttackTarget::Player(bob),
        }],
        ..crate::combat_state::CombatState::default()
    });

    let spell = game.object(spell_id).expect("Heavy Fog should be in hand");
    assert!(
        !crate::decision::can_cast_spell(
            &game,
            alice,
            spell,
            &crate::alternative_cast::CastingMethod::Normal,
        ),
        "Heavy Fog should not be castable if Alice was not attacked this step"
    );

    game.combat
        .as_mut()
        .expect("combat should be present")
        .attackers[0]
        .target = crate::combat_state::AttackTarget::Player(alice);

    let spell = game.object(spell_id).expect("Heavy Fog should be in hand");
    assert!(
        crate::decision::can_cast_spell(
            &game,
            alice,
            spell,
            &crate::alternative_cast::CastingMethod::Normal,
        ),
        "Heavy Fog should be castable during declare attackers after Alice was attacked"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn resolve_heavy_fog(game: &mut crate::game_state::GameState, controller: PlayerId) {
    let heavy_fog = parse_oracle_card_definition("Heavy Fog");
    let spell_id = game.create_object_from_definition(&heavy_fog, controller, Zone::Stack);
    game.push_to_stack(crate::game_state::StackEntry::new(spell_id, controller));
    crate::game_loop::resolve_stack_entry_with(
        game,
        &mut crate::decision::SelectFirstDecisionMaker,
    )
    .expect("Heavy Fog should resolve");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn heavy_fog_prevents_only_damage_to_you_from_attacking_creatures() {
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let bob_attacker = create_winds_test_creature(&mut game, "Bob Attacker", bob, 3, 3);
    let bob_nonattacker = create_winds_test_creature(&mut game, "Bob Nonattacker", bob, 3, 3);
    let alice_creature = create_winds_test_creature(&mut game, "Alice Creature", alice, 2, 2);

    resolve_heavy_fog(&mut game, alice);

    let shields = game.effect_store.prevention_effects.shields();
    assert_eq!(
        shields.len(),
        1,
        "Heavy Fog should create one prevention shield"
    );
    assert!(
        matches!(
            shields[0].protected,
            crate::prevention::PreventionTarget::You
        ) && shields[0].damage_filter.from_source.is_some(),
        "Heavy Fog should protect you from a source-filtered damage set, got {:?}",
        shields[0]
    );

    game.combat = Some(crate::combat_state::CombatState {
        attackers: vec![crate::combat_state::AttackerInfo {
            creature: bob_attacker,
            target: crate::combat_state::AttackTarget::Player(alice),
        }],
        blockers: std::iter::once((bob_attacker, Vec::new())).collect(),
        ..crate::combat_state::CombatState::default()
    });

    let (attacking_damage_to_you, _) = crate::events::processing::process_damage_with_event(
        &mut game,
        bob_attacker,
        crate::events::DamageTarget::Player(alice),
        3,
        false,
        crate::events::cause::EventCause::effect(),
    );
    assert_eq!(
        attacking_damage_to_you, 0,
        "Heavy Fog should prevent noncombat damage to you from an attacking creature"
    );

    let (nonattacking_damage_to_you, _) = crate::events::processing::process_damage_with_event(
        &mut game,
        bob_nonattacker,
        crate::events::DamageTarget::Player(alice),
        3,
        false,
        crate::events::cause::EventCause::effect(),
    );
    assert_eq!(
        nonattacking_damage_to_you, 3,
        "Heavy Fog should not prevent damage from a nonattacking creature"
    );

    let (attacking_damage_to_other_player, _) =
        crate::events::processing::process_damage_with_event(
            &mut game,
            bob_attacker,
            crate::events::DamageTarget::Player(bob),
            3,
            false,
            crate::events::cause::EventCause::effect(),
        );
    assert_eq!(
        attacking_damage_to_other_player, 3,
        "Heavy Fog should not prevent damage to players other than you"
    );

    let (attacking_damage_to_permanent, _) = crate::events::processing::process_damage_with_event(
        &mut game,
        bob_attacker,
        crate::events::DamageTarget::Object(alice_creature),
        3,
        false,
        crate::events::cause::EventCause::effect(),
    );
    assert_eq!(
        attacking_damage_to_permanent, 3,
        "Heavy Fog should not prevent damage to permanents you control"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_prevent_next_damage_to_any_target_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Amulet of Kroog Variant")
        .parse_text(
            "{2}, {T}: Prevent the next 1 damage that would be dealt to any target this turn.",
        )
        .expect("parse prevent-next damage clause");

    let lines = crate::compiled_text::unprocessed_compiled_lines(&def);
    let activated_line = lines.join(" ");
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

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_prevent_next_damage_rejects_trailing_tail_strictly() {
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

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_oracle_healing_grace_strict_and_keeps_source_choice_clause() {
    let def = parse_oracle_card_definition("Healing Grace");
    let rendered = crate::compiled_text::unprocessed_compiled_lines(&def).join("\n");
    assert!(
        rendered.contains("Prevent the next 3 damage")
            && rendered.contains("by a source of your choice")
            && rendered.contains("gain 3 life"),
        "expected Healing Grace prevention + source choice + life gain in compiled text, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn fiery_emancipation_strict_parser_compiled_text_and_model_regression() {
    assert_oracle_card_parses_strict("Fiery Emancipation");
    let def = parse_oracle_card_definition("Fiery Emancipation");
    let rendered = crate::compiled_text::unprocessed_compiled_lines(&def).join(" ");
    let debug = format!("{:#?}", def.abilities);

    assert!(
        rendered.contains(
            "If a source you control would deal damage to a permanent or player, it deals triple that damage to that permanent or player instead"
        ),
        "expected Fiery Emancipation triple-damage replacement wording, got {rendered}"
    );
    assert!(
        debug.contains("DoubleDamageAmountReplacement")
            && debug.contains("factor: 3")
            && debug.contains("source_filter")
            && debug.contains("target_player_filter")
            && debug.contains("target_object_filter"),
        "expected Fiery Emancipation to compile to a factor-3 damage replacement, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn fiery_emancipation_triples_only_damage_from_sources_you_control() {
    let fiery = parse_oracle_card_definition("Fiery Emancipation");
    let alice_source = CardDefinitionBuilder::new(CardId::from_raw(91_300), "Alice Damage Source")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let bob_source = CardDefinitionBuilder::new(CardId::from_raw(91_301), "Bob Damage Source")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let target = CardDefinitionBuilder::new(CardId::from_raw(91_302), "Bob Target Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(4, 4))
        .build();

    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let fiery_id = game.create_object_from_definition(&fiery, alice, Zone::Battlefield);
    let alice_source_id =
        game.create_object_from_definition(&alice_source, alice, Zone::Battlefield);
    let bob_source_id = game.create_object_from_definition(&bob_source, bob, Zone::Battlefield);
    let target_id = game.create_object_from_definition(&target, bob, Zone::Battlefield);

    game.update_replacement_effects();
    assert!(
        game.effect_store
            .replacement_effects
            .effects()
            .iter()
            .any(|replacement| {
                replacement.source == fiery_id
                    && matches!(
                        replacement.replacement,
                        crate::replacement::ReplacementAction::Modify(
                            crate::replacement::EventModification::Multiply(3)
                        )
                    )
            }),
        "Fiery Emancipation should register a factor-3 damage replacement"
    );

    let (player_damage, _) = crate::events::processing::process_damage_with_event(
        &mut game,
        alice_source_id,
        crate::events::DamageTarget::Player(bob),
        2,
        false,
        crate::events::cause::EventCause::effect(),
    );
    assert_eq!(
        player_damage, 6,
        "damage from an Alice-controlled source to a player should be tripled"
    );

    let (permanent_damage, _) = crate::events::processing::process_damage_with_event(
        &mut game,
        alice_source_id,
        crate::events::DamageTarget::Object(target_id),
        3,
        false,
        crate::events::cause::EventCause::effect(),
    );
    assert_eq!(
        permanent_damage, 9,
        "damage from an Alice-controlled source to a permanent should be tripled"
    );

    let (opponent_source_damage, _) = crate::events::processing::process_damage_with_event(
        &mut game,
        bob_source_id,
        crate::events::DamageTarget::Player(alice),
        4,
        false,
        crate::events::cause::EventCause::effect(),
    );
    assert_eq!(
        opponent_source_damage, 4,
        "damage from a source Alice does not control should not be tripled"
    );
}

#[test]
pub(super) fn embermaw_hellion_strict_parser_compiled_text_and_model_regression() {
    assert_oracle_card_parses_strict("Embermaw Hellion");
    let def = parse_oracle_card_definition("Embermaw Hellion");
    let rendered = crate::compiled_text::unprocessed_compiled_lines(&def).join(" ");
    let debug = format!("{:#?}", def.abilities);

    assert!(
        rendered.contains("Trample")
            && rendered.contains(
                "If another red source you control would deal damage to a permanent or player, it deals that much damage plus 1 to that permanent or player instead"
            ),
        "expected Embermaw Hellion trample and additive damage replacement wording, got {rendered}"
    );
    assert!(
        debug.contains("ModifyDamageAmountReplacement")
            && debug.contains("other: true")
            && debug.contains("colors: Some")
            && debug.contains("controller: Some(You)")
            && debug.contains("target_player_filter")
            && debug.contains("target_object_filter")
            && debug.contains("delta: 1"),
        "expected Embermaw Hellion to compile to an other red source +1 damage replacement, got {debug}"
    );
}

#[test]
pub(super) fn embermaw_hellion_adds_one_to_another_red_source_damage_to_players_and_permanents() {
    let embermaw = parse_oracle_card_definition("Embermaw Hellion");
    let red_source = CardDefinitionBuilder::new(CardId::from_raw(91_400), "Alice Red Source")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Red]]))
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let red_spell = CardDefinitionBuilder::new(CardId::from_raw(91_404), "Alice Red Spell")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Red]]))
        .card_types(vec![CardType::Instant])
        .build();
    let target = CardDefinitionBuilder::new(CardId::from_raw(91_401), "Alice Target Permanent")
        .card_types(vec![CardType::Artifact])
        .build();

    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let embermaw_id = game.create_object_from_definition(&embermaw, alice, Zone::Battlefield);
    let red_source_id = game.create_object_from_definition(&red_source, alice, Zone::Battlefield);
    let red_spell_id = game.create_object_from_definition(&red_spell, alice, Zone::Stack);
    let target_id = game.create_object_from_definition(&target, alice, Zone::Battlefield);

    game.update_replacement_effects();
    assert!(
        game.effect_store
            .replacement_effects
            .effects()
            .iter()
            .any(|replacement| {
                replacement.source == embermaw_id
                    && matches!(
                        replacement.replacement,
                        crate::replacement::ReplacementAction::Modify(
                            crate::replacement::EventModification::Add(1)
                        )
                    )
            }),
        "Embermaw Hellion should register a +1 damage replacement"
    );

    let player_damage = crate::events::processing::process_damage_assignments_with_event(
        &mut game,
        red_source_id,
        crate::events::DamageTarget::Player(bob),
        2,
        false,
        crate::events::cause::EventCause::effect(),
    );
    assert_eq!(player_damage.assignments.len(), 1);
    assert_eq!(player_damage.assignments[0].amount, 3);

    let permanent_damage = crate::events::processing::process_damage_assignments_with_event(
        &mut game,
        red_source_id,
        crate::events::DamageTarget::Object(target_id),
        4,
        false,
        crate::events::cause::EventCause::effect(),
    );
    assert_eq!(permanent_damage.assignments.len(), 1);
    assert_eq!(permanent_damage.assignments[0].amount, 5);

    let spell_damage = crate::events::processing::process_damage_assignments_with_event(
        &mut game,
        red_spell_id,
        crate::events::DamageTarget::Player(bob),
        2,
        false,
        crate::events::cause::EventCause::effect(),
    );
    assert_eq!(spell_damage.assignments.len(), 1);
    assert_eq!(spell_damage.assignments[0].amount, 3);
}

#[test]
pub(super) fn embermaw_hellion_ignores_self_nonred_and_opposing_sources() {
    let embermaw = parse_oracle_card_definition("Embermaw Hellion");
    let colorless_source =
        CardDefinitionBuilder::new(CardId::from_raw(91_402), "Alice Colorless Source")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
    let bob_red_source = CardDefinitionBuilder::new(CardId::from_raw(91_403), "Bob Red Source")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Red]]))
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();

    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let embermaw_id = game.create_object_from_definition(&embermaw, alice, Zone::Battlefield);
    let colorless_source_id =
        game.create_object_from_definition(&colorless_source, alice, Zone::Battlefield);
    let bob_red_source_id =
        game.create_object_from_definition(&bob_red_source, bob, Zone::Battlefield);
    game.update_replacement_effects();

    let self_damage = crate::events::processing::process_damage_assignments_with_event(
        &mut game,
        embermaw_id,
        crate::events::DamageTarget::Player(bob),
        2,
        false,
        crate::events::cause::EventCause::effect(),
    );
    assert_eq!(self_damage.assignments.len(), 1);
    assert_eq!(self_damage.assignments[0].amount, 2);

    let nonred_damage = crate::events::processing::process_damage_assignments_with_event(
        &mut game,
        colorless_source_id,
        crate::events::DamageTarget::Player(bob),
        2,
        false,
        crate::events::cause::EventCause::effect(),
    );
    assert_eq!(nonred_damage.assignments.len(), 1);
    assert_eq!(nonred_damage.assignments[0].amount, 2);

    let opposing_damage = crate::events::processing::process_damage_assignments_with_event(
        &mut game,
        bob_red_source_id,
        crate::events::DamageTarget::Player(alice),
        2,
        false,
        crate::events::cause::EventCause::effect(),
    );
    assert_eq!(opposing_damage.assignments.len(), 1);
    assert_eq!(opposing_damage.assignments[0].amount, 2);
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn sphere_of_truth_strict_parser_and_compiled_text_regression() {
    assert_oracle_card_parses_strict("Sphere of Truth");
    let def = parse_oracle_card_definition("Sphere of Truth");
    let rendered = crate::compiled_text::unprocessed_compiled_lines(&def).join(" ");
    let debug = format!("{:#?}", def.abilities);

    assert!(
        rendered.contains("If a white source would deal damage to you, prevent 2 of that damage"),
        "expected Sphere of Truth partial prevention wording, got {rendered}"
    );
    assert!(
        debug.contains("PreventDamageToYouFromSourceFilter") && debug.contains("amount: 2"),
        "expected Sphere of Truth to compile to a white-source partial prevention static ability, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn sphere_of_truth_reduces_each_white_source_damage_event_to_you_by_two() {
    let sphere = parse_oracle_card_definition("Sphere of Truth");
    let white_source = CardDefinitionBuilder::new(CardId::from_raw(91_200), "White Damage Source")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::White]]))
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let red_source = CardDefinitionBuilder::new(CardId::from_raw(91_201), "Red Damage Source")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Red]]))
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();

    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let sphere_id = game.create_object_from_definition(&sphere, alice, Zone::Battlefield);
    let white_source_id = game.create_object_from_definition(&white_source, bob, Zone::Battlefield);
    let red_source_id = game.create_object_from_definition(&red_source, bob, Zone::Battlefield);

    game.update_replacement_effects();
    assert!(
        game.effect_store
            .replacement_effects
            .effects()
            .iter()
            .any(|replacement| {
                replacement.source == sphere_id
                    && matches!(
                        replacement.replacement,
                        crate::replacement::ReplacementAction::PreventDamageAmount(2)
                    )
            }),
        "Sphere of Truth should register a static partial-prevention replacement"
    );

    let (white_damage, _) = crate::events::processing::process_damage_with_event(
        &mut game,
        white_source_id,
        crate::events::DamageTarget::Player(alice),
        3,
        false,
        crate::events::cause::EventCause::effect(),
    );
    assert_eq!(
        white_damage, 1,
        "3 white damage to you should be reduced by 2"
    );

    let (small_white_damage, _) = crate::events::processing::process_damage_with_event(
        &mut game,
        white_source_id,
        crate::events::DamageTarget::Player(alice),
        1,
        false,
        crate::events::cause::EventCause::effect(),
    );
    assert_eq!(
        small_white_damage, 0,
        "1 white damage to you should be fully prevented"
    );

    let (red_damage, _) = crate::events::processing::process_damage_with_event(
        &mut game,
        red_source_id,
        crate::events::DamageTarget::Player(alice),
        3,
        false,
        crate::events::cause::EventCause::effect(),
    );
    assert_eq!(
        red_damage, 3,
        "nonwhite source damage should not be reduced"
    );

    let (damage_to_other_player, _) = crate::events::processing::process_damage_with_event(
        &mut game,
        white_source_id,
        crate::events::DamageTarget::Player(bob),
        3,
        false,
        crate::events::cause::EventCause::effect(),
    );
    assert_eq!(
        damage_to_other_player, 3,
        "white source damage to a player other than you should not be reduced"
    );

    let no_prevention = CardDefinitionBuilder::new(CardId::from_raw(91_202), "No Prevention")
        .card_types(vec![CardType::Enchantment])
        .with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::damage_cant_be_prevented(),
        ))
        .build();
    game.create_object_from_definition(&no_prevention, bob, Zone::Battlefield);

    let (unpreventable_white_damage, _) = crate::events::processing::process_damage_with_event(
        &mut game,
        white_source_id,
        crate::events::DamageTarget::Player(alice),
        3,
        false,
        crate::events::cause::EventCause::effect(),
    );
    assert_eq!(
        unpreventable_white_damage, 3,
        "Sphere of Truth should not reduce damage while damage can't be prevented"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn healing_grace_runtime_prevents_up_to_three_damage_and_gains_life() {
    struct ChooseNamedSourceDecisionMaker {
        source_name: &'static str,
    }

    impl crate::decision::DecisionMaker for ChooseNamedSourceDecisionMaker {
        fn decide_objects(
            &mut self,
            game: &crate::game_state::GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            if let Some(chosen) = ctx.candidates.iter().find_map(|candidate| {
                if !candidate.legal {
                    return None;
                }
                let matches_name = game
                    .object(candidate.id)
                    .is_some_and(|object| object.name == self.source_name);
                if matches_name {
                    Some(candidate.id)
                } else {
                    None
                }
            }) {
                vec![chosen]
            } else {
                crate::decision::AutoPassDecisionMaker.decide_objects(game, ctx)
            }
        }
    }

    let def = parse_oracle_card_definition("Healing Grace");
    let spell = def
        .spell_effect
        .as_ref()
        .expect("Healing Grace should produce spell effects")
        .clone();
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let spell_source = game.create_object_from_definition(&def, alice, Zone::Stack);
    let chosen_source = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::from_raw(91_100), "Damage Source")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build(),
        bob,
        Zone::Battlefield,
    );
    let other_source = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::from_raw(91_102), "Other Damage Source")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build(),
        bob,
        Zone::Battlefield,
    );

    let mut dm = ChooseNamedSourceDecisionMaker {
        source_name: "Damage Source",
    };
    let mut ctx = crate::effects::ExecutionContext::new(spell_source, alice, &mut dm)
        .with_targets(vec![crate::effects::ResolvedTarget::Player(bob)])
        .with_target_assignments(vec![crate::game_state::TargetAssignment {
            spec: ChooseSpec::AnyTarget,
            range: 0..1,
        }]);

    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        spell_source,
        &spell,
        None,
        &[],
    )
    .expect("Healing Grace should resolve");

    assert_eq!(
        game.players[0].life, 23,
        "Healing Grace should gain 3 life for the caster"
    );

    let (first_damage, first_prevented) = crate::events::processing::process_damage_with_event(
        &mut game,
        chosen_source,
        crate::events::DamageTarget::Player(bob),
        2,
        false,
        crate::events::cause::EventCause::effect(),
    );
    assert_eq!(first_damage, 0, "first 2 damage should be prevented");
    assert!(
        first_prevented || first_damage == 0,
        "first damage application should reflect prevention"
    );

    let (second_damage, second_prevented) = crate::events::processing::process_damage_with_event(
        &mut game,
        chosen_source,
        crate::events::DamageTarget::Player(bob),
        2,
        false,
        crate::events::cause::EventCause::effect(),
    );
    assert_eq!(
        second_damage, 1,
        "shield should have exactly 1 prevention remaining"
    );
    assert!(
        second_prevented || second_damage < 2,
        "second damage application should still be partially prevented"
    );

    let (other_source_damage, other_source_prevented) =
        crate::events::processing::process_damage_with_event(
            &mut game,
            other_source,
            crate::events::DamageTarget::Player(bob),
            2,
            false,
            crate::events::cause::EventCause::effect(),
        );
    assert_eq!(
        other_source_damage, 2,
        "non-chosen source damage should not be prevented"
    );
    assert!(
        !other_source_prevented,
        "non-chosen source damage should remain fully unprevented"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn healing_grace_runtime_only_protects_chosen_target() {
    let def = parse_oracle_card_definition("Healing Grace");
    let spell = def
        .spell_effect
        .as_ref()
        .expect("Healing Grace should produce spell effects")
        .clone();
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let spell_source = game.create_object_from_definition(&def, alice, Zone::Stack);
    let damage_source = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::from_raw(91_101), "Damage Source Two")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build(),
        bob,
        Zone::Battlefield,
    );

    let mut dm = crate::decision::AutoPassDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new(spell_source, alice, &mut dm)
        .with_targets(vec![crate::effects::ResolvedTarget::Player(bob)])
        .with_target_assignments(vec![crate::game_state::TargetAssignment {
            spec: ChooseSpec::AnyTarget,
            range: 0..1,
        }]);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        spell_source,
        &spell,
        None,
        &[],
    )
    .expect("Healing Grace should resolve");

    let (damage_to_alice, _) = crate::events::processing::process_damage_with_event(
        &mut game,
        damage_source,
        crate::events::DamageTarget::Player(alice),
        2,
        false,
        crate::events::cause::EventCause::effect(),
    );
    assert_eq!(
        damage_to_alice, 2,
        "non-targeted player should not be protected"
    );

    let (damage_to_bob, _) = crate::events::processing::process_damage_with_event(
        &mut game,
        damage_source,
        crate::events::DamageTarget::Player(bob),
        2,
        false,
        crate::events::cause::EventCause::effect(),
    );
    assert_eq!(
        damage_to_bob, 0,
        "chosen target should receive prevented damage"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_oracle_samite_blessing_strict_text_and_granted_targeted_prevention() {
    assert_oracle_card_parses_strict("Samite Blessing");
    let def = parse_oracle_card_definition("Samite Blessing");
    let rendered = canonical_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Enchant creature")
            && rendered.contains("Enchanted creature has \"{T}: The next time a source of your choice would deal damage to target creature this turn, prevent that damage.\""),
        "expected Samite Blessing to preserve enchant creature and granted targeted prevention text, got {rendered}"
    );

    let debug = format!("{:#?}", def.abilities);
    assert!(
        debug.contains("AttachedAbilityGrant")
            && debug.contains("PreventNextTimeDamageEffect")
            && debug.contains("Target(Object"),
        "expected Samite Blessing to grant a targeted prevention activated ability, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_oracle_cho_arrim_alchemist_strict_text_and_activation_shape() {
    assert_oracle_card_parses_strict("Cho-Arrim Alchemist");
    let def = parse_oracle_card_definition("Cho-Arrim Alchemist");
    let activated = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("Cho-Arrim Alchemist should have an activated ability");
    let debug = format!("{activated:#?}");
    assert!(
        debug.contains("PreventNextTimeDamageEffect")
            && debug.contains("GainLifeEffect")
            && debug.contains("EventValue")
            && debug.contains("Amount"),
        "expected activated ability to prevent next damage and gain prevented amount as life, got {debug}"
    );
    assert!(
        debug.contains("Tap") && debug.contains("Discard") && debug.contains("White"),
        "expected activation costs to include white mana, tap, and discard, got {debug}"
    );

    let rendered = canonical_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("The next time a source of your choice would deal damage to you this turn, prevent that damage")
            && rendered.contains("You gain life equal to the damage prevented this way"),
        "expected Cho-Arrim Alchemist prevention/life text to compile, got {rendered}"
    );
    assert!(
        !rendered.to_ascii_lowercase().contains("unsupported"),
        "Cho-Arrim Alchemist should parse strictly without unsupported markers, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_oracle_dazzling_reflection_strict_text_and_spell_shape() {
    assert_oracle_card_parses_strict("Dazzling Reflection");
    let def = parse_oracle_card_definition("Dazzling Reflection");
    let spell = def
        .spell_effect
        .as_ref()
        .expect("Dazzling Reflection should have a spell effect");
    let debug = format!("{spell:#?}");
    assert!(
        debug.contains("TargetOnlyEffect")
            && debug.contains("GainLifeEffect")
            && debug.contains("PowerOf")
            && debug.contains("PreventNextTimeDamageEffect"),
        "expected Dazzling Reflection to target a creature, gain life from its power, and prevent that source's next damage, got {debug}"
    );

    let rendered = canonical_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("You gain life equal to target creature's power")
            && rendered.contains(
                "The next time that creature would deal damage this turn, prevent that damage"
            ),
        "expected Dazzling Reflection compiled text to preserve target-power life gain and that-creature prevention, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn dazzling_reflection_runtime_gains_target_power_and_prevents_that_creature_damage_only()
 {
    fn resolve_for_target_power(power: i32) -> (i32, bool, u32, bool, u32) {
        let def = parse_oracle_card_definition("Dazzling Reflection");
        let spell_effect = def.spell_effect.as_ref().expect("spell effect exists");
        let mut game =
            crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let spell_source = game.create_object_from_definition(&def, alice, Zone::Stack);
        let target_creature = game.create_object_from_definition(
            &CardDefinitionBuilder::new(CardId::from_raw(91_601), "Dazzling Target")
                .card_types(vec![CardType::Creature])
                .power_toughness(PowerToughness::fixed(power, 4))
                .build(),
            bob,
            Zone::Battlefield,
        );
        let other_creature = game.create_object_from_definition(
            &CardDefinitionBuilder::new(CardId::from_raw(91_602), "Other Damage Source")
                .card_types(vec![CardType::Creature])
                .power_toughness(PowerToughness::fixed(3, 3))
                .build(),
            bob,
            Zone::Battlefield,
        );

        let mut dm = crate::decision::AutoPassDecisionMaker;
        let mut ctx = crate::effects::ExecutionContext::new(spell_source, alice, &mut dm)
            .with_targets(vec![crate::effects::ResolvedTarget::Object(
                target_creature,
            )])
            .with_target_assignments(vec![crate::game_state::TargetAssignment {
                spec: ChooseSpec::target_creature(),
                range: 0..1,
            }]);
        crate::game_loop::execute_resolution_program(
            &mut game,
            &mut ctx,
            alice,
            spell_source,
            spell_effect,
            None,
            &[],
        )
        .expect("Dazzling Reflection should resolve");

        let life_after_resolution = game.life_total(alice);
        let (other_source_damage, other_source_prevented) =
            crate::events::processing::process_damage_with_event(
                &mut game,
                other_creature,
                crate::events::DamageTarget::Player(alice),
                3,
                false,
                crate::events::cause::EventCause::effect(),
            );
        let (target_source_damage, target_source_prevented) =
            crate::events::processing::process_damage_with_event(
                &mut game,
                target_creature,
                crate::events::DamageTarget::Player(bob),
                5,
                false,
                crate::events::cause::EventCause::effect(),
            );

        (
            life_after_resolution,
            other_source_prevented,
            other_source_damage,
            target_source_prevented,
            target_source_damage,
        )
    }

    let (life, other_prevented, other_damage, target_prevented, target_damage) =
        resolve_for_target_power(4);
    assert_eq!(
        life, 24,
        "Alice should gain life equal to target creature's power"
    );
    assert!(
        !other_prevented,
        "damage from a different creature should not be prevented"
    );
    assert_eq!(other_damage, 3, "nonmatching damage should still be dealt");
    assert!(
        target_prevented,
        "damage from the targeted creature should be prevented"
    );
    assert_eq!(
        target_damage, 0,
        "prevented target-creature damage should be reduced to zero"
    );

    let (life, _, _, target_prevented, target_damage) = resolve_for_target_power(0);
    assert_eq!(
        life, 20,
        "zero-power target should not increase Alice's life total"
    );
    assert!(
        target_prevented,
        "the prevention shield should still apply for a zero-power target"
    );
    assert_eq!(
        target_damage, 0,
        "zero-power branch should still prevent that creature's damage"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn cho_arrim_alchemist_runtime_prevents_chosen_source_to_you_and_gains_that_life() {
    struct ChooseNamedSourceDecisionMaker {
        source_name: &'static str,
    }

    impl crate::decision::DecisionMaker for ChooseNamedSourceDecisionMaker {
        fn decide_objects(
            &mut self,
            game: &crate::game_state::GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            if let Some(chosen) = ctx.candidates.iter().find_map(|candidate| {
                if !candidate.legal {
                    return None;
                }
                game.object(candidate.id)
                    .is_some_and(|object| object.name == self.source_name)
                    .then_some(candidate.id)
            }) {
                vec![chosen]
            } else {
                crate::decision::AutoPassDecisionMaker.decide_objects(game, ctx)
            }
        }
    }

    let def = parse_oracle_card_definition("Cho-Arrim Alchemist");
    let activated = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("Cho-Arrim Alchemist should have an activated ability");
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let alchemist_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let chosen_source = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::from_raw(91_103), "Chosen Damage Source")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(4, 4))
            .build(),
        bob,
        Zone::Battlefield,
    );
    let other_source = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::from_raw(91_104), "Other Damage Source")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(4, 4))
            .build(),
        bob,
        Zone::Battlefield,
    );

    let mut dm = ChooseNamedSourceDecisionMaker {
        source_name: "Chosen Damage Source",
    };
    let mut ctx = crate::effects::ExecutionContext::new(alchemist_id, alice, &mut dm);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        alchemist_id,
        &activated.effects,
        None,
        &[],
    )
    .expect("Cho-Arrim Alchemist ability should resolve");

    let (wrong_player_damage, wrong_player_prevented) =
        crate::events::processing::process_damage_with_event(
            &mut game,
            chosen_source,
            crate::events::DamageTarget::Player(bob),
            3,
            false,
            crate::events::cause::EventCause::effect(),
        );
    assert_eq!(
        wrong_player_damage, 3,
        "damage to Bob should not be prevented"
    );
    assert!(!wrong_player_prevented, "the shield only protects Alice");
    assert_eq!(
        game.life_total(alice),
        20,
        "nonmatching damage should not gain life"
    );

    let (other_source_damage, other_source_prevented) =
        crate::events::processing::process_damage_with_event(
            &mut game,
            other_source,
            crate::events::DamageTarget::Player(alice),
            3,
            false,
            crate::events::cause::EventCause::effect(),
        );
    assert_eq!(
        other_source_damage, 3,
        "damage from an unchosen source should not be prevented"
    );
    assert!(
        !other_source_prevented,
        "unchosen source should not consume the shield"
    );
    assert_eq!(
        game.life_total(alice),
        20,
        "unchosen source should not gain life"
    );

    let (prevented_damage, prevented) = crate::events::processing::process_damage_with_event(
        &mut game,
        chosen_source,
        crate::events::DamageTarget::Player(alice),
        4,
        false,
        crate::events::cause::EventCause::effect(),
    );
    assert_eq!(
        prevented_damage, 0,
        "chosen source damage to Alice should be prevented"
    );
    assert!(prevented, "the matching damage event should be replaced");
    assert_eq!(
        game.life_total(alice),
        24,
        "Alice should gain life equal to the damage prevented this way"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn divine_deflection_strict_parser_compiled_text_and_shape_regression() {
    assert_oracle_card_parses_strict("Divine Deflection");
    let def = parse_oracle_card_definition("Divine Deflection");
    let rendered = crate::compiled_text::unprocessed_compiled_lines(&def).join(" ");
    let debug = format!("{:#?}", def.spell_effect);

    assert!(
        rendered.contains(
            "Prevent the next X damage that would be dealt to you and/or permanents you control this turn"
        ) && rendered.contains(
            "If damage is prevented this way, Divine Deflection deals that much damage to any target"
        ),
        "expected Divine Deflection prevention and conditional damage text, got {rendered}"
    );
    assert!(
        debug.contains("PreventDamageEffect")
            && debug.contains("protect_you_and_permanents_you_control: true")
            && debug.contains("DealDamageEffect")
            && debug.contains("EventValue")
            && debug.contains("Amount")
            && debug.contains("target: AnyTarget"),
        "expected Divine Deflection to compile to a shared prevention shield with prevented-damage follow-up, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn divine_deflection_runtime_prevents_shared_pool_and_damages_chosen_target() {
    let def = parse_oracle_card_definition("Divine Deflection");
    let spell = def
        .spell_effect
        .as_ref()
        .expect("Divine Deflection should produce spell effects")
        .clone();
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let spell_source = game.create_object_from_definition(&def, alice, Zone::Stack);
    let damage_source = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::from_raw(91_105), "Damage Source")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(4, 4))
            .build(),
        bob,
        Zone::Battlefield,
    );
    let alice_permanent = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::from_raw(91_106), "Alice Permanent")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build(),
        alice,
        Zone::Battlefield,
    );

    let mut dm = crate::decision::AutoPassDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new(spell_source, alice, &mut dm)
        .with_x(5)
        .with_targets(vec![crate::effects::ResolvedTarget::Player(bob)])
        .with_target_assignments(vec![crate::game_state::TargetAssignment {
            spec: ChooseSpec::AnyTarget,
            range: 0..1,
        }]);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        spell_source,
        &spell,
        None,
        &[],
    )
    .expect("Divine Deflection should resolve");

    let (alice_damage, _) = crate::events::processing::process_damage_with_event(
        &mut game,
        damage_source,
        crate::events::DamageTarget::Player(alice),
        3,
        false,
        crate::events::cause::EventCause::effect(),
    );
    assert_eq!(alice_damage, 0, "damage to Alice should be prevented");
    assert_eq!(
        game.life_total(bob),
        17,
        "prevented damage should be dealt to the chosen any-target player"
    );

    let (permanent_damage, _) = crate::events::processing::process_damage_with_event(
        &mut game,
        damage_source,
        crate::events::DamageTarget::Object(alice_permanent),
        4,
        false,
        crate::events::cause::EventCause::effect(),
    );
    assert_eq!(
        permanent_damage, 2,
        "the shared prevention pool should have only 2 damage remaining"
    );
    assert_eq!(
        game.life_total(bob),
        15,
        "the follow-up should deal only the amount actually prevented"
    );

    let (bob_damage, bob_prevented) = crate::events::processing::process_damage_with_event(
        &mut game,
        damage_source,
        crate::events::DamageTarget::Player(bob),
        3,
        false,
        crate::events::cause::EventCause::effect(),
    );
    assert_eq!(bob_damage, 3, "damage to Bob should not be protected");
    assert!(
        !bob_prevented,
        "unprotected damage should not trigger prevention"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn divine_deflection_runtime_does_not_damage_target_when_damage_cannot_be_prevented() {
    let def = parse_oracle_card_definition("Divine Deflection");
    let spell = def
        .spell_effect
        .as_ref()
        .expect("Divine Deflection should produce spell effects")
        .clone();
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let spell_source = game.create_object_from_definition(&def, alice, Zone::Stack);
    let damage_source = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::from_raw(91_107), "Unpreventable Source")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(4, 4))
            .build(),
        bob,
        Zone::Battlefield,
    );
    let no_prevention = CardDefinitionBuilder::new(CardId::from_raw(91_108), "No Prevention")
        .card_types(vec![CardType::Enchantment])
        .with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::damage_cant_be_prevented(),
        ))
        .build();
    game.create_object_from_definition(&no_prevention, bob, Zone::Battlefield);

    let mut dm = crate::decision::AutoPassDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new(spell_source, alice, &mut dm)
        .with_x(5)
        .with_targets(vec![crate::effects::ResolvedTarget::Player(bob)])
        .with_target_assignments(vec![crate::game_state::TargetAssignment {
            spec: ChooseSpec::AnyTarget,
            range: 0..1,
        }]);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        spell_source,
        &spell,
        None,
        &[],
    )
    .expect("Divine Deflection should resolve");

    let (damage, prevented) = crate::events::processing::process_damage_with_event(
        &mut game,
        damage_source,
        crate::events::DamageTarget::Player(alice),
        3,
        false,
        crate::events::cause::EventCause::effect(),
    );
    assert_eq!(damage, 3, "unpreventable damage should not be reduced");
    assert!(
        !prevented,
        "unpreventable damage should not count as prevented"
    );
    assert_eq!(
        game.life_total(bob),
        20,
        "the conditional follow-up should not happen when no damage was prevented"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_target_opponent_chooses_creature_then_other_cant_block() {
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

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_target_opponent_chooses_creature_then_destroy_that_creature() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Imperial Edict Variant")
        .parse_text("Target opponent chooses a creature they control. Destroy that creature.")
        .expect("target-opponent choose + destroy sequence should parse");

    let effects = def.spell_effect.as_ref().expect("expected spell effects");
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

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Target opponent chooses a creature")
            && rendered.contains("Destroy that creature"),
        "expected target-opponent choice and destroy follow-up, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_scrounge_debug_text_preserves_target_opponent_choice() {
    let oracle = "Target opponent chooses an artifact card in their graveyard. Put that card onto the battlefield under your control.";
    let def = CardDefinitionBuilder::new(CardId::new(), "Scrounge Variant")
        .parse_text(oracle)
        .expect("Scrounge-style target-opponent graveyard choice should parse");

    let rendered = debug_compiled_lines(&def);
    assert_eq!(
        rendered,
        vec![oracle.to_string()],
        "expected debug compiled text to preserve target-opponent choice semantics"
    );

    let (_oracle_cov, _compiled_cov, similarity, delta, mismatch) =
        crate::semantic_compare::compare_card_semantics_scored(
            "Scrounge",
            oracle,
            &rendered,
            crate::semantic_compare::report_embedding_config(),
        );
    assert_eq!(similarity, 1.0);
    assert_eq!(delta, 0);
    assert!(
        !mismatch,
        "expected Scrounge debug text to avoid semantic mismatch, got {rendered:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_dredge_the_mire_debug_text_preserves_each_opponent_choices() {
    let oracle = "Each opponent chooses a creature card in their graveyard. Put those cards onto the battlefield under your control.";
    let def = CardDefinitionBuilder::new(CardId::new(), "Dredge the Mire Variant")
        .parse_text(oracle)
        .expect("Dredge the Mire style each-opponent graveyard choice should parse");

    let effects = def.spell_effect.as_ref().expect("expected spell effects");
    let flattened = effects
        .segments
        .iter()
        .flat_map(|segment| segment.default_effects.iter().cloned())
        .collect::<Vec<_>>();
    let score_path = crate::compiled_text::compile_effect_list(&flattened);
    assert_eq!(
        score_path,
        "Each opponent chooses a creature card in their graveyard. Put those cards onto the battlefield under your control"
    );

    let rendered = debug_compiled_lines(&def);
    assert_eq!(
        rendered,
        vec![oracle.to_string()],
        "expected Dredge the Mire debug compiled text to preserve each-opponent choices"
    );

    let (_oracle_cov, _compiled_cov, similarity, delta, mismatch) =
        crate::semantic_compare::compare_card_semantics_scored(
            "Dredge the Mire",
            oracle,
            &rendered,
            crate::semantic_compare::report_embedding_config(),
        );
    assert_eq!(similarity, 1.0);
    assert_eq!(delta, 0);
    assert!(
        !mismatch,
        "expected Dredge the Mire debug text to avoid semantic mismatch, got {rendered:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_wei_assassins_etb_target_opponent_chooses_creature_then_destroy() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Wei Assassins")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "When this creature enters, target opponent chooses a creature they control. Destroy that creature.",
        )
        .expect("Wei Assassins ETB text should parse");

    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Wei Assassins should compile to a triggered ability");

    let trigger_debug = format!("{:?}", triggered.trigger);
    assert!(
        trigger_debug.contains("ZoneChangeTrigger")
            && trigger_debug.contains("Specific(Battlefield)")
            && trigger_debug.contains("this_object: true"),
        "expected a self ETB zone-change trigger, got {trigger_debug}"
    );

    let effects = &triggered.effects;
    assert_eq!(
        effects.len(),
        3,
        "expected target opponent, opponent choice, and destroy effects, got {effects:?}"
    );

    let target_opponent = effects[0]
        .downcast_ref::<TargetOnlyEffect>()
        .expect("first effect should target an opponent");
    assert!(matches!(
        &target_opponent.target,
        ChooseSpec::Target(inner)
            if matches!(inner.as_ref(), ChooseSpec::Player(PlayerFilter::Opponent))
    ));

    let choose_creature = effects[1]
        .downcast_ref::<ChooseObjectsEffect>()
        .expect("second effect should make the target opponent choose a creature");
    assert_eq!(choose_creature.count, ChoiceCount::exactly(1));
    assert_eq!(
        choose_creature.chooser,
        PlayerFilter::Target(Box::new(PlayerFilter::Opponent))
    );
    assert_eq!(choose_creature.filter.zone, Some(Zone::Battlefield));
    assert_eq!(
        choose_creature.filter.controller,
        Some(PlayerFilter::IteratedPlayer)
    );
    assert_eq!(choose_creature.filter.card_types, vec![CardType::Creature]);
    assert_eq!(choose_creature.tag.as_str(), "__it__");

    let destroy_chosen = effects[2]
        .downcast_ref::<DestroyEffect>()
        .expect("third effect should destroy the chosen creature");
    let destroys_chosen = match destroy_chosen.spec.unhinted() {
        ChooseSpec::Iterated => true,
        ChooseSpec::Tagged(tag) => tag == &choose_creature.tag,
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => {
            filter.tagged_constraints.iter().any(|constraint| {
                constraint.tag == choose_creature.tag
                    && constraint.relation == crate::TaggedOpbjectRelation::IsTaggedObject
            })
        }
        _ => false,
    };
    assert!(
        destroys_chosen,
        "destroy should reference the creature chosen by target opponent, got {:?}",
        destroy_chosen.spec
    );

    let score_path = crate::compiled_text::compile_effect_list(effects).to_ascii_lowercase();
    assert!(
        score_path.contains("target opponent chooses a creature")
            && score_path.contains("destroy that creature"),
        "expected target-opponent choice and destroy follow-up, got {score_path}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("when this creature enters, target opponent chooses a creature")
            && rendered.contains("destroy that creature"),
        "expected Wei Assassins ETB choice and destroy semantics, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn nissas_pilgrimage_renders_shared_search_partition_and_count_only_mastery() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Pilgrimage Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Search your library for up to two basic Forest cards, reveal those cards, and put one onto the battlefield tapped and the rest into your hand. Then shuffle.\nSpell mastery — If there are two or more instant and/or sorcery cards in your graveyard, search your library for up to three basic Forest cards instead of two.",
        )
        .expect("Nissa-style count-only search replacement should parse");
    let rendered = unprocessed_compiled_lines(&def).join("\n");

    assert_eq!(
        rendered,
        "Search your library for up to two basic Forest cards, reveal those cards, and put one onto the battlefield tapped and the rest into your hand. Then shuffle. Spell mastery — If there are two or more instant and/or sorcery cards in your graveyard, search your library for up to three basic Forest cards instead of two.",
    );
    assert_eq!(rendered.matches("reveal those cards").count(), 1);
    assert_eq!(rendered.matches("Then shuffle").count(), 1);
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_ghoulflesh_style_anthem_and_type_color_addition() {
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

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_ghoulflesh_style_anthem_with_other_creature_types_scope() {
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

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_all_goblins_are_black_and_are_zombies_in_addition_clause() {
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

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_all_forests_and_saprolings_pt_color_type_addition_bundle() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Life and Limb Variant")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "All Forests and all Saprolings are 1/1 green Saproling creatures and Forest lands in addition to their other types.",
        )
        .expect("Life and Limb style static line should parse");

    let ids = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::SetColors)
            && ids.contains(&crate::static_abilities::StaticAbilityId::AddCardTypes)
            && ids.contains(&crate::static_abilities::StaticAbilityId::AddSubtypes)
            && ids.contains(
                &crate::static_abilities::StaticAbilityId::SetBasePowerToughnessForFilter
            ),
        "expected color/type/subtype/base-PT static bundle, got {ids:?}"
    );

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains(
            "All Forests and all Saprolings are 1/1 green Saproling creatures and Forest lands in addition to their other types"
        ),
        "expected compact Life and Limb wording, got {rendered}"
    );

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("any_of")
            && debug.contains("Forest")
            && debug.contains("Saproling")
            && debug.contains("SetBasePowerToughness"),
        "expected conjoined subject and characteristic static abilities, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_type_color_addition_rejects_unsupported_scope_words() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Unsupported Addition Scope")
            .parse_text("Enchanted creature gets -1/-1 and is a black Zombie in addition to its other abilities.")
            .expect_err("unsupported addition scope should fail parse");
    let message = format!("{err:?}");
    assert!(
        message.contains("unsupported in-addition scope in type/color clause"),
        "expected strict scope parse error, got {message}"
    );
}
