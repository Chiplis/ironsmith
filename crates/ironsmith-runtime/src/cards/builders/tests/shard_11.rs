#![allow(unused_imports)]
use super::shard_00::*;
use super::shard_01::*;
use super::shard_02::*;
use super::shard_03::*;
use super::shard_04::*;
use super::shard_05::*;
use super::shard_06::*;
use super::shard_07::*;
use super::shard_08::*;
use super::shard_09::*;
use super::shard_10::*;
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
pub(super) fn parse_crawlspace_attack_you_cap_line() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Crawlspace")
        .parse_text("No more than two creatures can attack you each combat.")
        .expect("crawlspace attack-you cap line should parse");
    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::MaxCreaturesCanAttackYouEachCombat),
        "expected attack-you-cap static ability, got {ids:?}"
    );
    let compiled = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        compiled.contains("no more than 2 creatures can attack you each combat"),
        "expected compiled text to include attack-you cap, got {compiled}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_opponent_loses_life_trigger_with_that_much_gain() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Life Trigger Variant")
        .parse_text("Whenever an opponent loses life, you gain that much life.")
        .expect("opponent-loses-life trigger with that-much gain should parse");
    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("whenever an opponent loses life")
            && joined.contains("you gain that much life"),
        "expected life-loss trigger and mirrored gain rendering, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_you_gain_life_trigger_with_target_opponent_loses_that_much() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Life Trigger Reverse Variant")
        .parse_text("Whenever you gain life, target opponent loses that much life.")
        .expect("you-gain-life trigger with that-much life loss should parse");
    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("whenever you gain life")
            && joined.contains("target opponent loses that much life"),
        "expected gain-life trigger and mirrored loss rendering, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn reject_event_value_life_amount_without_life_trigger() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Event Value Invalid Variant")
        .parse_text("Target opponent loses that much life.")
        .expect_err("standalone event-derived amount should fail parse");
    let message = format!("{err:?}");
    assert!(
        message.contains("event-derived amount requires a compatible trigger"),
        "expected event-value context rejection, got {message}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_damage_to_target_player_or_planeswalker() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Magmutt Variant")
        .parse_text("{T}: This creature deals 1 damage to target player or planeswalker.")
        .expect("player-or-planeswalker damage target should parse");
    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("deals 1 damage to target player or planeswalker"),
        "expected compiled damage text, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_that_much_damage_trigger_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Mogg Maniac Variant")
        .parse_text(
            "Whenever this creature is dealt damage, it deals that much damage to any target.",
        )
        .expect("that-much damage trigger clause should parse");
    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("that much damage"),
        "expected event-derived damage amount in compiled text, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_oracle_torch_the_witness_strictly_parses_and_renders_excess_damage_branch() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Torch the Witness")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::X],
            vec![ManaSymbol::Red],
        ]))
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Torch the Witness deals twice X damage to target creature. If excess damage was dealt to that creature this way, investigate.",
        )
        .expect("Torch the Witness should parse strictly");

    let debug = format!("{:#?}", def.spell_effect);
    assert!(
        debug.contains("XTimes") && debug.contains("ExcessDamageDealt"),
        "expected twice-X damage and excess-damage predicate in Torch the Witness definition, got {debug}"
    );

    let joined = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        joined.contains("Torch the Witness deals twice X damage to target creature")
            && joined.contains("If excess damage was dealt to that creature this way, investigate"),
        "expected Torch the Witness compiled text to preserve twice-X damage and excess-damage condition, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_gain_choice_of_keywords_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Gift Variant")
        .parse_text("Target creature gets +1/+1 and gains your choice of deathtouch or lifelink until end of turn.")
        .expect("gain-choice keyword clause should parse");
    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined
            .contains("target creature gets +1/+1 and gains your choice of deathtouch or lifelink"),
        "expected compact keyword-choice grant in compiled text, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_gain_choice_of_three_keywords_clause_compiles_to_mode_choice() {
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
    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("gains your choice of flying, deathtouch, or lifelink"),
        "expected compact keyword-choice activated ability text, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn assaultron_dominator_parses_counter_choice_attack_trigger() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Assaultron Dominator")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::Red],
        ]))
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .subtypes(vec![Subtype::Robot])
        .power_toughness(PowerToughness::fixed(2, 2))
        .parse_text(
            "When this creature enters, you get {E}{E} (two energy counters).\n\
             Whenever an artifact creature you control attacks, you may pay {E}. If you do, put your choice of a +1/+1, first strike, or trample counter on that creature.",
        )
        .expect("Assaultron Dominator should parse strictly");

    let ability_debug = format!("{:#?}", def.abilities);
    assert!(
        ability_debug.contains("ChooseModeEffect")
            && ability_debug.contains("PlusOnePlusOne")
            && ability_debug.contains("FirstStrike")
            && ability_debug.contains("Trample"),
        "expected modal counter choice in Assaultron Dominator, got {ability_debug}"
    );

    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains(
            "put your choice of a +1/+1, first strike, or trample counter on that creature"
        ),
        "expected compact counter-choice text for Assaultron Dominator, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_gain_choice_of_keywords_preserves_protection_qualifier() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Jodah Choice Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("{0}: Until end of turn, this creature gets -1/-1 and gains your choice of double strike, protection from red, vigilance, or shadow.")
        .expect("jodah-style choice keyword clause should parse");

    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains(
            "gains your choice of double strike, protection from red, vigilance, or shadow"
        ),
        "expected compact choice text with protection qualifier, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_search_same_name_reference_filter_in_graveyard() {
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

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_search_then_put_onto_battlefield_hides_search_tags() {
    let def = parse_oracle_card_definition("Traverse the Outlands");

    let rendered_raw = unprocessed_compiled_lines(&def).join(" | ");
    let rendered = rendered_raw.to_ascii_lowercase();
    assert!(
        rendered.contains("search your library")
            && rendered.contains("basic land")
            && rendered.contains("onto the battlefield tapped")
            && rendered.contains("then shuffle"),
        "expected compact search-to-battlefield text, got {rendered_raw}"
    );
    assert!(
        !rendered.contains("tags it as")
            && !rendered.contains("tagged object")
            && !rendered.contains(" in library"),
        "search-to-battlefield rendering should hide internal search tags, got {rendered_raw}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_search_count_by_distinct_powers() {
    let def = parse_oracle_card_definition("Celebrate the Harvest");

    let rendered_raw = unprocessed_compiled_lines(&def).join(" | ");
    let rendered = rendered_raw.to_ascii_lowercase();
    assert!(
        rendered.contains("where x is the number of different powers among creatures you control"),
        "expected distinct-power X basis in compact search text, got {rendered_raw}"
    );
    assert!(
        format!("{def:#?}").contains("DistinctPowers"),
        "expected search count value to use DistinctPowers, got {def:#?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_alternative_cost_with_return_to_hand_segment() {
    CardDefinitionBuilder::new(CardId::new(), "Borderpost Variant")
        .parse_text("You may pay {1} and return a basic land you control to its owner's hand rather than pay this spell's mana cost.")
        .expect("alternative cost with return-to-hand segment should parse");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_alternative_cost_with_return_to_hand_segment_preserves_non_mana_costs() {
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

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("return") && rendered.contains("basic land"),
        "expected return-land cost in compiled text, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_alternative_cost_with_sacrifice_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Fireblast Variant")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "You may sacrifice two Mountains rather than pay this spell's mana cost.\nFireblast Variant deals 4 damage to any target.",
        )
        .expect("alternative sacrifice cost should parse through shared payment conversion");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("sacrifice two mountains") && rendered.contains("deal 4 damage"),
        "expected sacrifice alternative cost in compiled text, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_alternative_cost_with_non_cost_effect_fails_loudly() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Impossible Alternative Cost")
        .card_types(vec![CardType::Instant])
        .parse_text("You may draw a card rather than pay this spell's mana cost.")
        .expect_err("non-cost alternative payment should fail loudly");
    let message = format!("{err:?}").to_ascii_lowercase();
    assert!(
        message.contains("draw") || message.contains("cost"),
        "expected loud non-cost alternative-cost error, got {message}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_if_you_control_no_artifacts_compiles_to_negated_player_controls() {
    let def = CardDefinitionBuilder::new(CardId::new(), "No Artifacts Probe")
        .card_types(vec![CardType::Instant])
        .parse_text("Draw two cards. If you control no artifacts, discard a card.")
        .expect("control-no-artifacts predicate should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
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

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_first_main_phase_trigger_uses_precombat_main_and_active_player() {
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

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_mill_cost_activation_line() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Mill Cost Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("{T}, Mill a card: Add {C}.")
        .expect("mill-cost activation line should parse");
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.to_ascii_lowercase().contains("mill a card"),
        "expected mill cost to be preserved in rendering, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_return_cost_activation_line() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Return Cost Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Return a Forest you control to its owner's hand: Untap target creature. Activate only once each turn.")
        .expect("return-cost activation line should parse");
    let compiled = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        compiled
            .to_ascii_lowercase()
            .contains("return a forest you control to its owner's hand"),
        "expected return cost in activated ability rendering, got {compiled}"
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

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_return_elf_cost_activation_line() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Return Elf Cost Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Return an Elf you control to its owner's hand: Untap target creature. Activate only once each turn.")
        .expect("return-elf-cost activation line should parse");
    let compiled = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        compiled
            .to_ascii_lowercase()
            .contains("return an elf you control to its owner's hand"),
        "expected return cost in activated ability rendering, got {compiled}"
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

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_equip_with_once_each_turn_restriction() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Leather Armor Variant")
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Equipment])
        .parse_text("Equip {0}.\nActivate only once each turn.")
        .expect("equip with once-each-turn restriction should parse");
    assert_eq!(
        def.abilities.len(),
        1,
        "once-per-turn restriction line should attach to equip instead of adding a fallback static ability"
    );
    let ability = def.abilities.first().expect("expected equip ability");
    let AbilityKind::Activated(activated) = &ability.kind else {
        panic!("expected equip to remain an activated ability");
    };
    let activation_debug = format!("{activated:#?}");
    assert!(
        activation_debug.contains("OncePerTurn"),
        "expected rewrite restriction line to model the once-per-turn equip restriction, got {activation_debug}"
    );
    let compiled = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        compiled.contains("equip {0}"),
        "expected equip ability in compiled output, got {compiled}"
    );
    assert!(
        compiled.contains("only once each turn"),
        "expected once-per-turn activation restriction in compiled output, got {compiled}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn elvish_refueler_strict_parser_and_compiled_text_regression() {
    let def = parse_oracle_card_definition("Elvish Refueler");
    let lines = canonical_compiled_lines(&def);

    assert!(
        lines.iter().any(|line| line
            == "During your turn, as long as you haven't activated an exhaust ability this turn, you may activate exhaust abilities as though they haven't been activated."),
        "expected Elvish Refueler exhaust permission line, got {lines:?}"
    );
    assert!(
        lines
            .iter()
            .any(|line| line == "Exhaust — {1}{G}: Put a +1/+1 counter on this creature."),
        "expected Elvish Refueler exhaust activated line, got {lines:?}"
    );
    assert!(
        def.abilities.iter().any(|ability| matches!(
            &ability.kind,
            AbilityKind::Static(static_ability)
                if static_ability.id()
                    == StaticAbilityId::ExhaustAbilitiesAsThoughUnactivatedThisTurn
        )),
        "expected Elvish Refueler static exhaust permission ability, got {:#?}",
        def.abilities
    );
    let exhaust = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) if activated.is_exhaust_ability() => Some(activated),
            _ => None,
        })
        .expect("Elvish Refueler should have an exhaust activated ability");
    let effects_debug = format!("{:#?}", exhaust.effects);
    assert!(
        effects_debug.contains("PutCountersEffect") && effects_debug.contains("PlusOnePlusOne"),
        "expected Elvish Refueler exhaust ability to put a +1/+1 counter on itself, got {effects_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_mercenary_token_with_tap_pump_ability() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Mercenary Token Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("When this creature enters, create a 1/1 red Mercenary creature token with \"{T}: Target creature you control gets +1/+0 until end of turn. Activate only as a sorcery.\"")
        .expect("mercenary token with tap-pump ability should parse");
    let compiled = unprocessed_compiled_lines(&def).join(" ");
    let lower = compiled.to_ascii_lowercase();
    assert!(
        lower.contains("mercenary creature token"),
        "expected mercenary token creation in compiled text, got {compiled}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_sleep_with_the_fishes_keeps_unblockable_token_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Sleep with the Fishes")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Aura])
        .parse_text(
            "Enchant creature\nWhen this Aura enters, tap enchanted creature and you create a 1/1 blue Fish creature token with \"This token can't be blocked.\"\nEnchanted creature doesn't untap during its controller's untap step.",
        )
        .expect("Sleep with the Fishes should parse");

    let compiled = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        compiled.contains("can't be blocked") || compiled.contains("cant be blocked"),
        "expected compiled text to keep unblockable token clause, got {compiled}"
    );
    assert!(
        compiled.contains("doesn't untap during its controller's untap step")
            || compiled.contains("doesnt untap during its controller's untap step")
            || compiled.contains("doesnt untap during its controllers untap step"),
        "expected compiled text to keep enchanted-creature untap restriction, got {compiled}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_token_becomes_tapped_damage_trigger() {
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

    let compiled = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        compiled.contains("becomes tapped") && compiled.contains("deals 1 damage to target player"),
        "expected becomes-tapped damage trigger in compiled text, got {compiled}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_survivor_token_preserves_survivor_subtype() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Survivor Token Variant")
        .parse_text("Create a 1/1 red Survivor creature token.")
        .expect("survivor token clause should parse");

    let spell_debug = format!("{:#?}", def.spell_effect);
    assert!(
        spell_debug.contains("CreateTokenEffect") && spell_debug.contains("Survivor"),
        "expected created token to keep Survivor subtype, got {spell_debug}"
    );

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Survivor creature token"),
        "expected compiled text to retain Survivor token wording, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_deathpact_style_token_activation_is_preserved() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Deathpact Angel")
        .card_types(vec![CardType::Creature])
        .parse_text("When this creature dies, create a 1/1 white and black Cleric creature token. It has \"{3}{W}{B}{B}, {T}, Sacrifice this token: Return a card named Deathpact Angel from your graveyard to the battlefield.\"")
        .expect("deathpact-style token activation should parse");
    let compiled = unprocessed_compiled_lines(&def).join(" ");
    let lower = compiled.to_ascii_lowercase();
    assert!(
        lower.contains("create a 1/1 white and black cleric creature token"),
        "expected deathpact token creation to remain in output, got {compiled}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_llanowar_mentor_token_keeps_tap_for_green_mana_ability() {
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
        activated.mana_output.as_deref() == Some(&[ManaSymbol::Green])
            && matches!(activated.mana_cost.costs(), [cost] if cost.requires_tap())
    });
    assert!(
        has_green_tap_mana,
        "expected created token to keep '{{T}}: Add {{G}}' ability, got {:#?}",
        create.token.abilities
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("{t}: add {g}"),
        "expected compiled text to show token mana ability, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_sparkspitter_token_reminder_sets_next_end_step_sacrifice() {
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

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("sacrifice")
            && rendered.contains("end step")
            && rendered.contains("trample")
            && rendered.contains("haste"),
        "expected compiled text to preserve token keywords and delayed sacrifice, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_construct_token_with_for_each_artifact_text_keeps_single_token_and_cda() {
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

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_construct_token_with_single_quoted_rules_text_keeps_cda() {
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

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_token_with_quoted_static_power_toughness_keeps_cda() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Voice Token Variant")
        .parse_text(
            "Create a green and white Elemental creature token with \"This token's power and toughness are each equal to the number of creatures you control.\"",
        )
        .expect("quoted static P/T token text should parse");

    let debug = format!("{:#?}", def.spell_effect);
    assert!(
        debug.contains("CreateTokenEffect"),
        "expected spell create-token effect, got {debug}"
    );
    assert!(
        debug.contains("CharacteristicDefiningPT") && debug.contains("Creature"),
        "expected Elemental token to keep dynamic creature-count P/T text, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_ritual_of_the_returned_keeps_token_power_toughness_followup_on_spell_effect() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Ritual of the Returned Variant")
        .parse_text(
            "Exile target creature card from your graveyard. Create a black Zombie creature token. Its power is equal to that card's power and its toughness is equal to that card's toughness.",
        )
        .expect("Ritual of the Returned text should parse");

    let spell_debug = format!("{:#?}", def.spell_effect);
    assert!(
        spell_debug.contains("CreateTokenEffect"),
        "expected Ritual to remain a spell that creates a token, got {spell_debug}"
    );
    assert!(
        spell_debug.contains("SetBasePowerToughnessEffect"),
        "expected Ritual token to get a resolved base power/toughness setter, got {spell_debug}"
    );
    assert!(
        spell_debug.contains("PowerOf")
            && spell_debug.contains("ToughnessOf")
            && spell_debug.contains("Tagged"),
        "expected Ritual token P/T to come from the tagged exiled card, got {spell_debug}"
    );
    assert!(
        def.abilities.is_empty(),
        "Ritual should not compile as a battlefield static ability, got {:#?}",
        def.abilities
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("create a black zombie creature token")
            && rendered.contains("that card's power and toughness"),
        "expected oracle-style Ritual token wording, got {rendered}"
    );
    assert!(
        !rendered.contains("0/0"),
        "expected dynamic token P/T normalization to hide the temporary 0/0 shell, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_broken_visage_keeps_destroy_no_regen_and_token_followups_on_spell_effect() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Broken Visage Variant")
        .parse_text(
            "Destroy target nonartifact attacking creature. It can't be regenerated. Create a black Spirit creature token. Its power is equal to that creature's power and its toughness is equal to that creature's toughness. Sacrifice the token at the beginning of the next end step.",
        )
        .expect("Broken Visage text should parse");

    let spell_debug = format!("{:#?}", def.spell_effect);
    assert!(
        spell_debug.contains("DestroyNoRegenerationEffect"),
        "expected Broken Visage to keep no-regeneration destroy semantics, got {spell_debug}"
    );
    assert!(
        spell_debug.contains("CreateTokenEffect"),
        "expected Broken Visage to remain a spell that creates a token, got {spell_debug}"
    );
    assert!(
        spell_debug.contains("SetBasePowerToughnessEffect"),
        "expected Broken Visage token to get a resolved base power/toughness setter, got {spell_debug}"
    );
    assert!(
        spell_debug.contains("sacrifice_at_next_end_step: true"),
        "expected Broken Visage token to be sacrificed at the next end step, got {spell_debug}"
    );
    assert!(
        def.abilities.is_empty(),
        "Broken Visage should not compile as a static ability, got {:#?}",
        def.abilities
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("destroy target nonartifact attacking creature. it can't be regenerated"),
        "expected no-regeneration destroy wording in compiled output, got {rendered}"
    );
    assert!(
        rendered.contains("create a black spirit creature token")
            && rendered.contains("that creature's power and toughness"),
        "expected dynamic Spirit token wording in compiled output, got {rendered}"
    );
    assert!(
        rendered.contains("sacrifice") && rendered.contains("beginning of the next end step"),
        "expected delayed sacrifice wording in compiled output, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_sound_the_call_token_does_not_misread_named_card_reference_as_token_name() {
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
    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("wolf creature token"),
        "token name should remain subtype-derived Wolf, got {rendered}"
    );
    assert!(
        rendered.contains("card named sound the call"),
        "expected token to keep its named-card scaling ability, got {rendered}"
    );
    assert!(
        rendered.contains("for each card named sound the call in each graveyard"),
        "expected token ability to keep each-graveyard oracle wording, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_ozox_nested_token_return_keeps_named_card_literal() {
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

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_sacrifice_all_non_ogres() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Yukora Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("When this creature leaves the battlefield, sacrifice all non-Ogre creatures you control.")
        .expect("sacrifice-all trigger should parse");
    let compiled = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        compiled
            .to_ascii_lowercase()
            .contains("sacrifice all non-ogre creatures you control"),
        "expected 'sacrifice all' rendering, got {compiled}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_mana_ability_activate_only_if_control_subtype() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Tainted Mana Variant")
        .card_types(vec![CardType::Land])
        .parse_text("{T}: Add {B}.\n{T}: Add {U}. Activate only if you control a Swamp.")
        .expect("mana ability activation condition should parse");
    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
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

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_source_entry_or_basic_land_mana_gate_for_all_five_lands() {
    for (name, colored_mana) in [
        ("Dark Fortress", "{B} or {R}"),
        ("Gathering Place", "{G} or {W}"),
        ("Gleaming Bastion", "{W} or {U}"),
        ("Hidden Lair", "{U} or {B}"),
        ("Training Compound", "{R} or {G}"),
    ] {
        let oracle = format!(
            "{{T}}: Add {{C}}.\n{{T}}: Add {colored_mana}. Activate only if this land entered this turn or if you control a basic land."
        );
        let def = CardDefinitionBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Land])
            .parse_text(&oracle)
            .unwrap_or_else(|err| panic!("{name} should parse: {err}"));

        assert_eq!(def.abilities.len(), 2, "{name}");
        let AbilityKind::Activated(colorless) = &def.abilities[0].kind else {
            panic!("{name} colorless ability should be activated");
        };
        assert!(
            colorless.activation_condition.is_none(),
            "{name} colorless ability must remain unconditional"
        );

        let AbilityKind::Activated(colored) = &def.abilities[1].kind else {
            panic!("{name} colored ability should be activated");
        };
        let Some(crate::ConditionExpr::Or(left, right)) = &colored.activation_condition else {
            panic!(
                "{name} colored ability should carry a typed disjunction, got {:?}",
                colored.activation_condition
            );
        };
        assert!(matches!(
            left.as_ref(),
            crate::ConditionExpr::ObjectEnteredBattlefieldThisTurn(filter)
                if filter.source
                    && filter.source_surface
                        == Some(SourceReferenceSurface::ThisPermanentType(
                            "this land".to_string()
                        ))
        ));
        assert!(matches!(
            right.as_ref(),
            crate::ConditionExpr::YouControl(filter)
                if filter.card_types.contains(&CardType::Land)
                    && filter
                        .supertypes
                        .contains(&crate::types::Supertype::Basic)
        ));

        let rendered = compiled_text_lines(&def);
        let expected = format!(
            "{{T}}: Add {colored_mana}. Activate only if this land entered this turn or if you control a basic land"
        );
        assert!(
            rendered
                .iter()
                .any(|line| line.trim_end_matches('.') == expected),
            "expected exact colored mana restriction for {name}, got {rendered:?}"
        );
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn source_entry_or_basic_land_gate_controls_colored_mana_legal_actions() {
    use crate::events::{EnterBattlefieldEvent, RawEvent};
    use crate::provenance::ProvNodeId;
    use crate::special_actions::{SpecialAction, can_perform_check};

    fn definition() -> CardDefinition {
        CardDefinitionBuilder::new(CardId::new(), "Dark Fortress")
            .card_types(vec![CardType::Land])
            .parse_text(
                "{T}: Add {C}.\n{T}: Add {B} or {R}. Activate only if this land entered this turn or if you control a basic land.",
            )
            .expect("representative land should parse")
    }

    fn setup() -> (crate::game_state::GameState, PlayerId, ObjectId) {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = game.create_object_from_definition(&definition(), alice, Zone::Battlefield);
        (game, alice, source)
    }

    fn can_activate(
        game: &crate::game_state::GameState,
        player: PlayerId,
        source: ObjectId,
        ability_index: usize,
    ) -> bool {
        can_perform_check(
            &SpecialAction::ActivateManaAbility {
                permanent_id: source,
                ability_index,
            },
            game,
            player,
        )
        .is_ok()
    }

    fn record_source_entry_this_turn(game: &mut crate::game_state::GameState, source: ObjectId) {
        let snapshot = crate::snapshot::ObjectSnapshot::from_object(
            game.object(source).expect("source should exist"),
            game,
        );
        let event = RawEvent::new(
            EnterBattlefieldEvent::new(source, Zone::Hand),
            ProvNodeId::default(),
        );
        game.turn_store
            .turn_history
            .record_event(&event, Some(snapshot), None);
    }

    fn add_land(
        game: &mut crate::game_state::GameState,
        controller: PlayerId,
        name: &str,
        basic: bool,
        subtypes: Vec<Subtype>,
    ) {
        let mut builder = crate::card::CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Land])
            .subtypes(subtypes);
        if basic {
            builder = builder.supertypes(vec![crate::types::Supertype::Basic]);
        }
        let card = builder.build();
        game.create_object_from_card(&card, controller, Zone::Battlefield);
    }

    let (game, alice, source) = setup();
    assert!(can_activate(&game, alice, source, 0));
    assert!(!can_activate(&game, alice, source, 1));

    let (mut game, alice, source) = setup();
    record_source_entry_this_turn(&mut game, source);
    assert!(can_activate(&game, alice, source, 1));
    game.turn_store.turn_history.clear_for_new_turn();
    assert!(!can_activate(&game, alice, source, 1));

    let (mut game, alice, source) = setup();
    add_land(&mut game, alice, "Forest", true, vec![Subtype::Forest]);
    assert!(can_activate(&game, alice, source, 1));

    let (mut game, alice, source) = setup();
    add_land(
        &mut game,
        alice,
        "Nonbasic Island",
        false,
        vec![Subtype::Island],
    );
    assert!(!can_activate(&game, alice, source, 1));

    let (mut game, alice, source) = setup();
    let bob = PlayerId::from_index(1);
    add_land(
        &mut game,
        bob,
        "Opponent Forest",
        true,
        vec![Subtype::Forest],
    );
    assert!(!can_activate(&game, alice, source, 1));
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_semicolon_keyword_line_does_not_force_comma_merge() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Semicolon Keywords Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("First strike; banding")
        .expect("semicolon-separated supported keywords should parse");
    assert!(
        def.abilities
            .iter()
            .filter(|ability| matches!(&ability.kind, AbilityKind::Static(static_ability) if static_ability.id().is_keyword()))
            .count()
            >= 2,
        "expected both semicolon-separated keywords to lower"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_giant_solifuge_keeps_keyword_structure_and_compares_cleanly() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Giant Solifuge")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Red, ManaSymbol::Green],
            vec![ManaSymbol::Red, ManaSymbol::Green],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Insect])
        .power_toughness(PowerToughness::fixed(4, 1))
        .parse_text("Trample; haste; shroud")
        .expect("Giant Solifuge text should parse");

    let static_ids = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        static_ids,
        vec![
            StaticAbilityId::Trample,
            StaticAbilityId::Haste,
            StaticAbilityId::Shroud,
        ],
        "expected Giant Solifuge to compile to its three intrinsic keywords, got {static_ids:?}"
    );

    let compiled = unprocessed_compiled_lines(&def);
    let (_oracle_coverage, _compiled_coverage, similarity, delta, mismatch) =
        crate::semantic_compare::compare_semantics_scored(
            "Trample; haste; shroud",
            &compiled,
            crate::semantic_compare::report_embedding_config(),
        );
    assert!(
        !mismatch && similarity >= 0.99 && delta == 0,
        "expected Giant Solifuge keyword-only debug text to compare cleanly, similarity={similarity}, delta={delta}, compiled={compiled:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_storage_depletion_and_ki_counters() {
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

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_land_doesnt_untap_if_has_depletion_counter() {
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

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_destroy_target_attacking_or_blocking_creature_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Divine Verdict Variant")
        .card_types(vec![CardType::Instant])
        .parse_text("Destroy target attacking or blocking creature.")
        .expect("parse destroy attacking-or-blocking clause");

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("DestroyEffect"),
        "expected destroy effect, got {debug}"
    );
    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("destroy") && rendered.contains("attacking"),
        "expected attacking/blocking destroy rendering, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_activate_only_restriction_inline_with_activated_ability() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Timed Drawer")
        .card_types(vec![CardType::Creature])
        .parse_text("{T}: Draw a card. Activate only during your turn.")
        .expect("parse activated ability with inline activation restriction");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("draw a card"),
        "expected activated ability rendering, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_mana_ability_activate_only_as_instant_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Flash Mana Source")
        .card_types(vec![CardType::Artifact])
        .parse_text("{T}: Add {R}. Activate only as an instant.")
        .expect("parse mana ability with instant-speed activation restriction");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(rendered.contains("Activate only as an instant"));
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_generic_activated_presentation_label_keeps_exact_source_surface() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Power-up Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Power-up — {5}{U}: Put three +1/+1 counters on this creature.")
        .expect("parse generic activated ability presentation label");

    let rendered = unprocessed_compiled_lines(&def);
    assert!(
        rendered
            .iter()
            .any(|line| line == "Power-up — {5}{U}: Put three +1/+1 counters on this creature."),
        "expected exact generic activated presentation label, got {rendered:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_boast_ability_keeps_mechanic_prefix() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Boastful Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Boast — {1}{R}: This creature deals 1 damage to any target.")
        .expect("parse Boast ability with prefix");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("{1}{R}:"),
        "expected Boast activation cost in debug-safe rendering, got {rendered}"
    );
    assert!(
        rendered.contains("deals 1 damage to any target"),
        "expected Boast effect rendering, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_boast_ability_with_prior_sentence_still_keeps_prefix() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Boastful Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Hagi Mob enters the battlefield tapped. Boast — {1}{R}: This creature deals 1 damage to any target.")
        .expect("parse boast ability after leading sentence with prefix");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("{1}{R}:") && rendered.contains("deals 1 damage to any target"),
        "expected Boast activation cost and effect in debug-safe rendering, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_renew_ability_keeps_mechanic_prefix() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Renew Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Renew — {2}{B}, Exile this card from your graveyard: Put a flying counter on target creature. Activate only as a sorcery.",
        )
        .expect("parse Renew ability with prefix");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Exile this creature") || rendered.contains("Exile this card"),
        "expected Renew exile cost in rendering, got {rendered}"
    );
    assert!(
        rendered.contains("Activate only as a sorcery"),
        "expected Renew timing restriction in rendering, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_binding_contract_label_into_draw_replacement_static_ability() {
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

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_gain_life_for_each_clause() {
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

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_deal_damage_equal_to_clause_without_leading_amount() {
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

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_return_multiple_targets_uses_their_owners_hands() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Return Wave")
        .card_types(vec![CardType::Instant])
        .parse_text("Return up to two target creatures to their owners' hands.")
        .expect("parse multi-return clause");

    let joined = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        joined.contains("to their owners' hands"),
        "expected plural owner-hand wording, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_put_minus_one_counter_uses_singular_counter_wording() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Scar Variant")
        .card_types(vec![CardType::Instant])
        .parse_text("Put a -1/-1 counter on target creature.")
        .expect("parse single counter clause");

    let joined = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        joined.contains("Put a -1/-1 counter on target creature"),
        "expected singular -1/-1 counter wording, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_put_counter_on_each_of_up_to_targets_uses_each_of() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Gird Variant")
        .card_types(vec![CardType::Instant])
        .parse_text("Put a +1/+1 counter on each of up to two target creatures.")
        .expect("parse counted multi-target counter clause");

    let joined = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        joined.contains("on each of up to two target creatures"),
        "expected each-of wording for counted target counters, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_put_counter_on_up_to_one_target_omits_each_of() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Yawgmoth Probe")
        .card_types(vec![CardType::Instant])
        .parse_text("Put a -1/-1 counter on up to one target creature.")
        .expect("parse optional single-target counter clause");

    let joined = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        joined.contains("Put a -1/-1 counter on up to one target creature"),
        "expected optional single-target counter wording, got {joined}"
    );
    assert!(
        !joined.contains("each of up to one target creature"),
        "optional single-target counter wording should not use each-of, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_up_to_counter_amount_on_target() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Lore Counter Variant")
        .card_types(vec![CardType::Instant])
        .parse_text("Put up to three lore counters on target Saga.")
        .expect("parse optional counter amount clause");

    let debug = format!("{:#?}", def.spell_effect);
    assert!(
        debug.contains("PutCountersEffect")
            && debug.contains("Lore")
            && debug.contains("UpTo")
            && debug.contains("amount: SurfaceHinted")
            && debug.contains("3"),
        "expected up-to counter amount to lower with an up-to amount hint, got {debug}"
    );

    let rendered = compiled_text_lines(&def).join(" ");
    assert!(
        rendered.contains("Put up to three lore counters on target Saga"),
        "expected up-to counter amount to render, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn auntie_ool_renders_blight_ward_and_triggered_control_branches() {
    let def = parse_oracle_card_definition("Auntie Ool, Cursewretch");
    let rendered = unprocessed_compiled_lines(&def).join("\n");
    assert!(
        rendered.contains("Ward—Blight 2"),
        "expected blight ward keyword, got {rendered}"
    );

    let lower = rendered.to_ascii_lowercase();
    assert!(
        lower.contains(
            "draw a card if you control that creature. if you don't control it, its controller loses 1 life"
        ),
        "expected compact triggering-creature control branches, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_token_copy_cleanup_preserves_your_next_end_step() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Token Cleanup Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Create a token that's a copy of target enchantment you control. It gains haste. Sacrifice it at the beginning of your next end step.",
        )
        .expect("parse token-copy cleanup with controller-specific end step");

    let schedule = def
        .spell_effect
        .as_ref()
        .expect("expected spell effects")
        .flattened_default_effects()
        .into_iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::ScheduleDelayedTriggerEffect>())
        .unwrap_or_else(|| {
            panic!(
                "expected delayed cleanup trigger, got {:#?}",
                def.spell_effect
            )
        });
    assert_eq!(
        schedule.trigger,
        crate::triggers::Trigger::beginning_of_end_step(PlayerFilter::You)
    );

    let rendered = compiled_text_lines(&def).join(" ");
    assert!(
        rendered.contains("At the beginning of your next end step, sacrifice it"),
        "expected rendered cleanup to preserve your next end step, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_scry_one_then_draw_uses_then() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Mentor Guidance Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "When you cast this spell, copy it if you control a planeswalker, Cleric, Druid, Shaman, Warlock, or Wizard.\nScry 1, then draw a card.",
        )
        .expect("parse mentor's guidance text");

    let joined = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        joined.contains("Scry 1, then draw a card"),
        "expected scry/draw to stay as a then-clause, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_equip_line_with_parenthetical_colon_preserves_prefix_text() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Plate Variant")
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "Equipped creature gets +3/+3 and has ward {1}. (Whenever equipped creature becomes the target of a spell or ability an opponent controls, counter it unless that player pays {1}.)\nEquip {3}. This ability costs {1} less to activate for each other Equipment you control. ({3}: Attach to target creature you control. Equip only as a sorcery.)",
        )
        .expect("parse equip line with parenthetical colon");

    let joined = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        joined.contains("Equipped creature gets +3/+3") && joined.contains("Equip {3}"),
        "expected equip prefix text to survive heading stripping, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_put_counters_sequence_on_distinct_targets() {
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

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_put_multiple_counter_types_on_single_target() {
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

    let joined = unprocessed_compiled_lines(&def).join(" ");
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

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_spells_cost_modifier_merges_second_color_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "High Seas Variant")
        .card_types(vec![CardType::Enchantment])
        .parse_text("Red creature spells and green creature spells cost {1} more to cast.")
        .expect("parse dual-color spell tax clause");

    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("red and green creature spells cost {1} more to cast"),
        "expected both spell-color qualifiers in rendered text, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_spells_cost_modifier_keeps_mana_value_qualifier() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Krosan Drover Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Creature spells you cast with mana value 6 or greater cost {2} less to cast.")
        .expect("parse mana-value-qualified creature cost reduction");

    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains(
            "creature spells you cast with mana value 6 or greater cost {2} less to cast"
        ),
        "expected mana-value qualifier in rendered cost-modifier text, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_spells_cost_modifier_keeps_power_qualifier() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Goreclaw Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Creature spells you cast with power 4 or greater cost {2} less to cast.")
        .expect("parse power-qualified creature cost reduction");

    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("creature spells you cast with power 4 or greater cost {2} less to cast"),
        "expected power qualifier in rendered cost-modifier text, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_spells_cost_modifier_target_clause_does_not_add_spell_type() {
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

    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("spells you cast that target creature cost {2} less to cast"),
        "expected rendered text to keep the target clause without adding a spell type, got {joined}"
    );
    assert!(
        !joined.contains("creature spells you cast that target creature"),
        "target clause should not render as a creature-spell restriction, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_spells_cost_modifier_keeps_shared_creature_type_with_source_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Mistform Warchief Variant")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Illusion])
        .parse_text(
            "Creature spells you cast that share a creature type with this creature cost {1} less to cast.\n\
             {T}: This creature becomes the creature type of your choice until end of turn.",
        )
        .expect("parse shared-creature-type creature spell cost reduction");

    let reduction = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => static_ability.cost_reduction(),
            _ => None,
        })
        .expect("expected CostReduction static ability");

    assert_eq!(reduction.filter.card_types, vec![CardType::Creature]);
    assert_eq!(reduction.filter.cast_by, Some(PlayerFilter::You));
    assert!(reduction.filter.shares_creature_type_with_source);

    let joined = crate::compiled_text::compiled_text_lines(&def).join("\n");
    assert!(
        joined.contains(
            "Creature spells you cast that share a creature type with this creature cost {1} less to cast."
        ) && joined.contains(
            "{T}: This creature becomes the creature type of your choice until end of turn."
        ),
        "expected shared-creature-type reduction and choice wording, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_semblance_anvil_keeps_shared_exiled_card_type_cost_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Semblance Anvil")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(3)]]))
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "Imprint — When this artifact enters, you may exile a nonland card from your hand.\n\
             Spells you cast that share a card type with the exiled card cost {2} less to cast.",
        )
        .expect("Semblance Anvil should parse strictly");

    let reduction = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => static_ability.cost_reduction(),
            _ => None,
        })
        .expect("Semblance Anvil should have a cost reduction static ability");

    assert_eq!(reduction.filter.cast_by, Some(PlayerFilter::You));
    assert!(
        reduction
            .filter
            .tagged_constraints
            .iter()
            .any(|constraint| {
                constraint.tag.as_str() == crate::tag::SOURCE_EXILED_TAG
                    && constraint.relation == crate::target::TaggedOpbjectRelation::SharesCardType
            }),
        "Semblance Anvil cost filter should require sharing a card type with the exiled card, got {:?}",
        reduction.filter
    );

    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains(
            "imprint — when this artifact enters, you may exile a nonland card from your hand"
        ),
        "expected compiled text to preserve Semblance Anvil's imprint trigger, got {joined}"
    );
    assert!(
        joined.contains(
            "spells you cast that share a card type with the exiled card cost {2} less to cast"
        ),
        "expected compiled text to preserve Semblance Anvil's shared-card-type cost clause, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn prototype_portal_strict_parser_keeps_imprint_copy_and_x_definition() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(90_401), "Prototype Portal")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(4)]]))
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "Imprint — When this artifact enters, you may exile an artifact card from your hand.\n\
             {X}, {T}: Create a token that's a copy of the exiled card. X is the mana value of that card.",
        )
        .expect("Prototype Portal should parse strictly");

    assert_eq!(
        crate::compiled_text::compiled_text_lines(&def),
        vec![
            "Imprint — When this artifact enters, you may exile an artifact card from your hand."
                .to_string(),
            "{X}, {T}: Create a token that's a copy of the exiled card. X is the mana value of that card."
                .to_string(),
        ],
    );

    let activated = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("Prototype Portal should have an activated ability");
    assert!(
        activated.mana_cost.dynamic_mana_cost().is_some(),
        "Prototype Portal's {{X}} cost should lower to a dynamic mana cost"
    );
    let ability_debug = format!("{:?}", activated);
    assert!(
        ability_debug.contains("CreateTokenCopyEffect")
            && ability_debug.contains(crate::tag::SOURCE_EXILED_TAG),
        "activated ability should create a token copy of the source-exiled card, got {ability_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_spells_cost_modifier_keeps_noncreature_qualifier() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Glowrider Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Noncreature spells cost {1} more to cast.")
        .expect("parse noncreature spell tax clause");

    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("noncreature spells cost {1} more to cast"),
        "expected noncreature qualifier in rendered cost-modifier text, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_spells_cost_modifier_supports_colored_mana_increase() {
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

    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("black spells you cast cost {b} more to cast"),
        "expected colored cost increase to render, got {joined}"
    );
}

#[test]
pub(super) fn parse_oracle_defiler_of_instinct_optional_life_cost_reduction_regression() {
    let def = parse_oracle_card_definition("Defiler of Instinct");
    let raw = format!("{def:#?}");
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();

    let reduction = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => static_ability.cost_reduction_mana_cost(),
            _ => None,
        })
        .expect("Defiler of Instinct should have a mana-symbol cost reduction");

    assert_eq!(reduction.filter.cast_by, Some(PlayerFilter::You));
    assert!(
        reduction.filter.card_types.contains(&CardType::Creature)
            && reduction.filter.card_types.contains(&CardType::Artifact)
            && reduction.filter.card_types.contains(&CardType::Enchantment)
            && reduction
                .filter
                .card_types
                .contains(&CardType::Planeswalker)
            && reduction.filter.card_types.contains(&CardType::Battle),
        "Defiler cost reduction should apply to red permanent spells, got {:?}",
        reduction.filter
    );
    assert!(
        reduction.optional_life_additional_cost.is_some(),
        "Defiler cost reduction should be gated by its optional life additional cost, got {raw}"
    );
    assert!(
        rendered_lower
            .contains("as an additional cost to cast red permanent spells, you may pay 2 life")
            && rendered_lower
                .contains("those spells cost {r} less to cast if you paid life this way")
            && rendered_lower.contains("this effect reduces only the amount of red mana you pay"),
        "Defiler compiled text should preserve the optional additional cost and gated colored reduction, got {rendered}"
    );
    assert!(
        rendered_lower.contains("whenever you cast a red permanent spell")
            && rendered_lower.contains("deals 1 damage to any target"),
        "Defiler compiled text should preserve its red-permanent cast trigger, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_spells_cost_modifier_supports_where_x_differently_named_lands() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Fungal Colossus Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "This spell costs {X} less to cast, where X is the number of differently named lands you control.",
        )
        .expect("parse where-X cost reduction clause");

    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("{x} less to cast"),
        "expected cost reduction in rendered text, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn leyline_binding_compiled_text_keeps_domain_and_opponent_controlled_target() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Leyline Binding Variant")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "Flash\nDomain — This spell costs {1} less to cast for each basic land type among lands you control.\nWhen this enchantment enters, exile target nonland permanent an opponent controls until this enchantment leaves the battlefield.",
        )
        .expect("Leyline Binding text should parse");

    assert_eq!(
        crate::compiled_text::compiled_text_lines(&def),
        vec![
            "Flash".to_string(),
            "Domain — This spell costs {1} less to cast for each basic land type among lands you control.".to_string(),
            "When this enchantment enters, exile target nonland permanent an opponent controls until this enchantment leaves the battlefield.".to_string(),
        ],
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_spells_cost_modifier_supports_extended_where_x_clauses() {
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

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn visions_flashback_commander_reduction_is_typed_and_renders_as_one_line() {
    let cases = [
        ("Visions of Dread", "{8}{B}{B}"),
        ("Visions of Duplicity", "{8}{U}{U}"),
        ("Visions of Dominance", "{8}{G}{G}"),
        ("Visions of Glory", "{8}{W}{W}"),
        ("Visions of Ruin", "{8}{R}{R}"),
    ];
    let reduction_text = "This spell costs {X} less to cast this way, where X is the greatest mana value of a commander you own on the battlefield or in the command zone.";

    for (idx, (name, flashback_cost)) in cases.into_iter().enumerate() {
        let oracle = format!("Flashback {flashback_cost}. {reduction_text}");
        let def = CardDefinitionBuilder::new(CardId::from_raw(91_000 + idx as u32), name)
            .card_types(vec![CardType::Sorcery])
            .parse_text(&oracle)
            .expect("Visions compound flashback line should parse strictly");

        assert!(matches!(
            def.alternative_casts.as_slice(),
            [crate::alternative_cast::AlternativeCastingMethod::Flashback { .. }]
        ));
        let reduction = def
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Static(static_ability) => static_ability.this_spell_cost_reduction(),
                _ => None,
            })
            .expect("Visions line should retain a typed self cost reduction");
        assert_eq!(
            reduction.alternative_cast,
            Some(crate::filter::AlternativeCastKind::Flashback)
        );
        assert_eq!(compiled_text_lines(&def), vec![oracle]);
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn cavern_hoard_dragon_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(119_956), "Cavern-Hoard Dragon")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(7)],
            vec![ManaSymbol::Red],
            vec![ManaSymbol::Red],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Dragon])
        .power_toughness(PowerToughness::fixed(6, 6))
        .parse_text(
            "This spell costs {X} less to cast, where X is the greatest number of artifacts an opponent controls.\nFlying, trample, haste\nWhenever this creature deals combat damage to a player, you create a Treasure token for each artifact that player controls.",
        )
        .expect("Cavern-Hoard Dragon should parse strictly")
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn cavern_hoard_dragon_strict_parser_regression() {
    let def = cavern_hoard_dragon_definition();

    let reduction = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => static_ability.this_spell_cost_reduction(),
            _ => None,
        })
        .expect("expected Cavern-Hoard Dragon to have a this-spell cost reduction");

    let crate::effect::Value::GreatestCount(filter) = &reduction.reduction else {
        panic!(
            "expected greatest-count cost reduction for Cavern-Hoard Dragon, got {:?}",
            reduction.reduction
        );
    };
    assert!(filter.card_types.contains(&CardType::Artifact));
    assert_eq!(filter.controller, Some(PlayerFilter::Opponent));
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn cavern_hoard_dragon_compiled_text_keeps_greatest_artifact_clause() {
    let def = cavern_hoard_dragon_definition();

    assert_eq!(
        crate::compiled_text::compiled_text_lines(&def),
        vec![
            "This spell costs {X} less to cast, where X is the greatest number of artifacts an opponent controls.".to_string(),
            "Flying, trample, haste".to_string(),
            "Whenever this creature deals combat damage to a player, create a Treasure token for each artifact that player controls.".to_string(),
        ]
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn shadow_of_mortality_strict_parser_regression() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(90_321), "Shadow of Mortality")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "If your life total is less than your starting life total, this spell costs {X} less to cast, where X is the difference.\nTrample",
        )
        .expect("Shadow of Mortality should parse through conditional this-spell reduction");

    let reduction = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => static_ability.this_spell_cost_reduction(),
            _ => None,
        })
        .expect("expected this-spell cost reduction for Shadow of Mortality");

    assert!(matches!(
        reduction.condition,
        crate::static_abilities::ThisSpellCostCondition::LifeTotalLessThanStarting
    ));
    assert!(matches!(reduction.reduction, crate::effect::Value::X));
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn shadow_of_mortality_compiled_text_keeps_life_difference_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(90_322), "Shadow of Mortality")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "If your life total is less than your starting life total, this spell costs {X} less to cast, where X is the difference.\nTrample",
        )
        .expect("Shadow of Mortality should parse for compiled text regression");

    let compiled = crate::compiled_text::compiled_text_lines(&def).join("\n");
    assert!(
        compiled.contains("This spell costs {X} less to cast")
            && compiled.contains("where X is the difference")
            && compiled.contains("if your life total is less than your starting life total"),
        "expected conditional X reduction clause with explicit difference wording in compiled text, got {compiled}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn octavia_living_thesis_definition() -> CardDefinition {
    let oracle = oracle_text_by_name()
        .get("Octavia, Living Thesis")
        .expect("Octavia, Living Thesis should exist in cards.json")
        .clone();
    CardDefinitionBuilder::new(CardId::from_raw(658_547), "Octavia, Living Thesis")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(8)],
            vec![ManaSymbol::Blue],
            vec![ManaSymbol::Blue],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Elemental, Subtype::Octopus])
        .power_toughness(PowerToughness::fixed(8, 8))
        .parse_text(oracle)
        .expect("Octavia, Living Thesis should parse strictly")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn create_octavia_graveyard_card(
    game: &mut crate::game_state::GameState,
    controller: PlayerId,
    name: &str,
    card_types: Vec<CardType>,
) {
    let card = crate::card::CardBuilder::new(CardId::new(), name)
        .card_types(card_types)
        .build();
    game.create_object_from_card(&card, controller, Zone::Graveyard);
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn octavia_living_thesis_strict_parser_and_compiled_text_regression() {
    assert_oracle_card_parses_strict("Octavia, Living Thesis");
    let def = octavia_living_thesis_definition();

    let reduction = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => static_ability.this_spell_cost_reduction(),
            _ => None,
        })
        .expect("Octavia should have a this-spell cost reduction");
    assert!(matches!(
        reduction.reduction,
        crate::effect::Value::Fixed(8)
    ));
    assert!(matches!(
        &reduction.condition,
        crate::static_abilities::ThisSpellCostCondition::YouHaveCardsOfTypesInYourGraveyardOrMore {
            count: 8,
            card_types,
        } if card_types == &vec![CardType::Instant, CardType::Sorcery]
    ));

    let rendered = crate::compiled_text::compiled_text_lines(&def);
    assert_eq!(
        rendered.first().map(String::as_str),
        Some(
            "This spell costs {8} less to cast if you have 8 or more instant and/or sorcery cards in your graveyard."
        ),
        "Octavia compiled text should preserve the conditional instant/sorcery graveyard cost reduction, got {rendered:?}"
    );
    assert!(
        rendered.iter().any(|line| line == "Ward {8}"),
        "Octavia compiled text should preserve ward, got {rendered:?}"
    );
    assert!(
        rendered.iter().any(|line| line.contains("Magecraft")
            && line.contains("instant or sorcery spell")
            && line.contains("base power and toughness 8/8 until end of turn")),
        "Octavia compiled text should preserve magecraft base-8/8 effect, got {rendered:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn octavia_living_thesis_cost_reduction_requires_matching_cards_in_your_graveyard() {
    let def = octavia_living_thesis_definition();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let octavia_id = game.create_object_from_definition(&def, alice, Zone::Hand);

    for idx in 0..7 {
        create_octavia_graveyard_card(
            &mut game,
            alice,
            &format!("Alice Instant {idx}"),
            vec![CardType::Instant],
        );
    }
    create_octavia_graveyard_card(&mut game, alice, "Alice Creature", vec![CardType::Creature]);
    for idx in 0..8 {
        create_octavia_graveyard_card(
            &mut game,
            bob,
            &format!("Bob Instant {idx}"),
            vec![CardType::Instant],
        );
    }

    let octavia = game
        .object(octavia_id)
        .expect("Octavia should be in Alice's hand");
    let base_cost = octavia.mana_cost.as_ref().expect("Octavia has a mana cost");
    assert_eq!(
        crate::decision::calculate_effective_mana_cost(&game, alice, octavia, base_cost)
            .to_oracle(),
        "{8}{U}{U}",
        "Octavia should not count nonmatching cards or an opponent's graveyard"
    );

    create_octavia_graveyard_card(&mut game, alice, "Alice Sorcery", vec![CardType::Sorcery]);
    let octavia = game
        .object(octavia_id)
        .expect("Octavia should still be in Alice's hand");
    let base_cost = octavia.mana_cost.as_ref().expect("Octavia has a mana cost");
    assert_eq!(
        crate::decision::calculate_effective_mana_cost(&game, alice, octavia, base_cost)
            .to_oracle(),
        "{U}{U}",
        "Octavia should cost {{8}} less with eight instant and/or sorcery cards in your graveyard"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_this_spell_cost_reduction_counts_creature_types_with_cap() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Valiant Changeling Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "This spell costs {1} less to cast for each creature type among creatures you control. This effect can't reduce the amount of mana this spell costs by more than {5}.",
        )
        .expect("parse capped creature-type cost reduction");

    let reduction = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => static_ability.this_spell_cost_reduction(),
            _ => None,
        })
        .expect("expected this-spell cost reduction");

    let crate::effect::Value::Min(amount, cap) = &reduction.reduction else {
        panic!(
            "expected capped reduction amount, got {:?}",
            reduction.reduction
        );
    };
    assert!(matches!(cap.as_ref(), crate::effect::Value::Fixed(5)));
    let crate::effect::Value::CreatureTypesAmong(filter) = amount.as_ref() else {
        panic!("expected creature-type count reduction, got {amount:?}");
    };
    assert_eq!(filter.zone, Some(Zone::Battlefield));
    assert_eq!(filter.controller, Some(PlayerFilter::You));
    assert!(filter.card_types.contains(&CardType::Creature));

    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("for each creature type among creatures you control")
            && joined.contains("can't reduce the amount of mana this spell costs by more than {5}"),
        "expected capped creature-type reduction text, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn debug_safe_keeps_changeling_separate_from_other_keywords() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Valiant Changeling Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "This spell costs {1} less to cast for each creature type among creatures you control. This effect can't reduce the amount of mana this spell costs by more than {5}.\nChangeling (This card is every creature type.)\nDouble strike",
        )
        .expect("parse changeling with additional keyword");

    let lines = unprocessed_compiled_lines(&def);
    assert!(
        lines.iter().any(|line| line == "Changeling"),
        "expected separate Changeling line, got {lines:?}"
    );
    assert!(
        lines.iter().any(|line| line == "Double strike"),
        "expected separate double strike line, got {lines:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_imposing_grandeur_tracks_commander_mana_value_in_battlefield_or_command_zone() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Imposing Grandeur")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Each player may discard their hand and draw cards equal to the greatest mana value of a commander they own on the battlefield or in the command zone.",
        )
        .expect("Imposing Grandeur should parse");

    let debug = format!("{:#?}", def.spell_effect);
    assert!(
        debug.contains("GreatestManaValue"),
        "expected greatest-mana-value aggregate in spell effect, got {debug}"
    );
    assert!(
        debug.contains("any_of: [") && debug.contains("Battlefield") && debug.contains("Command"),
        "expected battlefield and command-zone commander filters, got {debug}"
    );
    assert!(
        debug.contains("is_commander: true"),
        "expected commander restriction in aggregate filter, got {debug}"
    );
    assert!(
        debug.contains("owner: Some(") && debug.contains("IteratedPlayer"),
        "expected per-player ownership in aggregate filter, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_destroy_cant_be_regenerated_followup_sentence() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Wrath Tail Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Destroy target creature. It can't be regenerated.")
        .expect("parse destroy + can't be regenerated");

    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("destroy target creature. it can't be regenerated")
            || joined.contains("destroy target creature. it cant be regenerated"),
        "expected can't-be-regenerated tail to render, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_damage_cant_be_regenerated_followup_sentence() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Engulfing Flames Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Engulfing Flames deals 1 damage to target creature. It can't be regenerated this turn.")
        .expect("parse damage + can't-be-regenerated-this-turn followup");

    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
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

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_threshold_destroy_cant_be_regenerated_followup_sentence() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Toxic Stench Variant")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Target nonblack creature gets -1/-1 until end of turn. Threshold \u{2014} If there are seven or more cards in your graveyard, instead destroy that creature. It can't be regenerated.",
        )
        .expect("parse conditional destroy + can't-be-regenerated followup");

    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("destroy target nonblack creature")
            && (joined.contains("can't be regenerated") || joined.contains("cant be regenerated")),
        "expected destroy-no-regeneration conditional branch in rendered text, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_destroy_target_creature_dealt_damage_this_turn() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Siegebreaker Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Destroy target creature that was dealt damage this turn.")
        .expect("parse destroy target creature dealt-damage-this-turn clause");

    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("destroy target creature that was dealt damage this turn"),
        "expected dealt-damage restriction in rendered destroy text, got {joined}"
    );
}

pub(super) fn death_rattle_oni_definition() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(91_500), "Death-Rattle Oni")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(6)],
            vec![ManaSymbol::Black],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Demon, Subtype::Spirit])
        .power_toughness(PowerToughness::fixed(5, 4))
        .parse_text(
            "Flash\nThis spell costs {2} less to cast for each creature that died this turn.\nWhen this creature enters, destroy all other creatures that were dealt damage this turn.",
        )
        .expect("Death-Rattle Oni should parse strictly")
}

pub(super) fn death_rattle_oni_destroy_filter(
    def: &CardDefinition,
) -> &crate::target::ObjectFilter {
    def.abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => triggered
                .effects
                .flattened_default_effects()
                .iter()
                .find_map(|effect| effect.downcast_ref::<DestroyEffect>())
                .and_then(|destroy| match &destroy.spec {
                    ChooseSpec::All(filter) => Some(filter),
                    other => {
                        panic!(
                            "Death-Rattle Oni should destroy all matching creatures, got {other:?}"
                        )
                    }
                }),
            _ => None,
        })
        .expect("Death-Rattle Oni should have an enters destroy trigger")
}

pub(super) fn create_death_rattle_runtime_creature(
    game: &mut crate::game_state::GameState,
    controller: PlayerId,
    name: &str,
) -> ObjectId {
    let def = CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 3))
        .build();
    game.create_object_from_definition(&def, controller, Zone::Battlefield)
}

pub(super) fn record_death_rattle_damage(
    game: &mut crate::game_state::GameState,
    source: ObjectId,
    target: ObjectId,
) {
    let event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::DamageEvent::with_cause(
            source,
            crate::events::DamageTarget::Object(target),
            1,
            false,
            crate::events::cause::EventCause::effect(),
        ),
        crate::provenance::ProvNodeId::default(),
    );
    game.record_turn_history_event(&event);
}

pub(super) fn zone_has_named_object(
    game: &crate::game_state::GameState,
    zone: Zone,
    name: &str,
) -> bool {
    game.objects_in_zone(zone)
        .iter()
        .any(|id| game.object(*id).is_some_and(|object| object.name == name))
}

pub(super) fn glissa_sunseeker_conditional_destroy_effect(def: &CardDefinition) -> Effect {
    def.abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => activated.effects.segments[0]
                .default_effects
                .iter()
                .find(|effect| {
                    effect
                        .downcast_ref::<crate::effects::ConditionalEffect>()
                        .is_some()
                })
                .cloned(),
            _ => None,
        })
        .expect("Glissa Sunseeker should have a conditional destroy effect")
}

pub(super) fn glissa_sunseeker_target_only_effect(
    def: &CardDefinition,
) -> &crate::effects::TargetOnlyEffect {
    def.abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => activated.effects.segments[0]
                .default_effects
                .iter()
                .find_map(|effect| {
                    effect
                        .downcast_ref::<crate::effects::TaggedEffect>()
                        .and_then(|tagged| {
                            tagged
                                .effect
                                .downcast_ref::<crate::effects::TargetOnlyEffect>()
                        })
                }),
            _ => None,
        })
        .expect("Glissa Sunseeker should require an artifact target")
}

pub(super) fn create_glissa_runtime_artifact(
    game: &mut crate::game_state::GameState,
    controller: PlayerId,
    name: &str,
    mana_value: u8,
) -> ObjectId {
    let def = CardDefinitionBuilder::new(CardId::new(), name)
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(
            mana_value,
        )]]))
        .card_types(vec![CardType::Artifact])
        .build();
    game.create_object_from_definition(&def, controller, Zone::Battlefield)
}

pub(super) fn create_glissa_runtime_creature(
    game: &mut crate::game_state::GameState,
    controller: PlayerId,
    name: &str,
) -> ObjectId {
    let def = CardDefinitionBuilder::new(CardId::new(), name)
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(3)]]))
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    game.create_object_from_definition(&def, controller, Zone::Battlefield)
}

pub(super) fn resolve_glissa_conditional_destroy(
    game: &mut crate::game_state::GameState,
    glissa_id: ObjectId,
    controller: PlayerId,
    target_id: ObjectId,
) {
    let def = parse_oracle_card_definition("Glissa Sunseeker");
    let effect = glissa_sunseeker_conditional_destroy_effect(&def);
    let target_snapshot = crate::snapshot::ObjectSnapshot::from_object(
        game.object(target_id)
            .expect("Glissa Sunseeker target should exist"),
        game,
    );
    let tagged = HashMap::from([(
        crate::tag::TagKey::from("targeted_0"),
        vec![target_snapshot],
    )]);
    let mut ctx = crate::effects::ExecutionContext::new_default(glissa_id, controller)
        .with_targets(vec![crate::effects::ResolvedTarget::Object(target_id)])
        .with_tagged_objects(tagged);
    crate::effects::execute_effect(game, &effect, &mut ctx)
        .expect("Glissa Sunseeker conditional destroy should resolve");
}

#[test]
pub(super) fn glissa_sunseeker_strict_parser_and_compiled_text_regression() {
    assert_oracle_card_parses_strict("Glissa Sunseeker");
    let def = parse_oracle_card_definition("Glissa Sunseeker");
    let rendered = crate::compiled_text::compiled_text_lines(&def);
    let ability_debug = format!("{:?}", def.abilities);

    assert_eq!(
        rendered,
        vec![
            "First strike".to_string(),
            "{T}: Destroy target artifact if its mana value is equal to the amount of unspent mana you have.".to_string(),
        ],
        "Glissa Sunseeker compiled text should preserve the conditional unspent-mana destroy clause"
    );
    assert!(
        ability_debug.contains("ConditionalEffect")
            && ability_debug.contains("EqualExpr")
            && ability_debug.contains("UnspentMana"),
        "Glissa Sunseeker should structurally compare target mana value to unspent mana, got {ability_debug}"
    );
}

#[test]
pub(super) fn glissa_sunseeker_targets_artifacts_only() {
    let def = parse_oracle_card_definition("Glissa Sunseeker");
    let target_only = glissa_sunseeker_target_only_effect(&def);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let glissa_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let artifact_id = create_glissa_runtime_artifact(&mut game, bob, "Matching Artifact", 3);
    let creature_id = create_glissa_runtime_creature(&mut game, bob, "Nonartifact Creature");
    let ctx = crate::effects::ExecutionContext::new_default(glissa_id, alice);

    assert!(
        crate::effects::validate_target(
            &game,
            &crate::effects::ResolvedTarget::Object(artifact_id),
            &target_only.target,
            &ctx,
        ),
        "Glissa Sunseeker should allow artifact targets"
    );
    assert!(
        !crate::effects::validate_target(
            &game,
            &crate::effects::ResolvedTarget::Object(creature_id),
            &target_only.target,
            &ctx,
        ),
        "Glissa Sunseeker should not allow nonartifact targets"
    );
}

#[test]
pub(super) fn glissa_sunseeker_destroys_artifact_when_mana_value_equals_unspent_mana() {
    let def = parse_oracle_card_definition("Glissa Sunseeker");
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let glissa_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let artifact_id = create_glissa_runtime_artifact(&mut game, bob, "Matching Artifact", 3);
    let artifact_stable = game.object(artifact_id).expect("artifact exists").stable_id;
    game.player_mut(alice)
        .expect("Alice exists")
        .mana_pool
        .add(ManaSymbol::Green, 3);

    resolve_glissa_conditional_destroy(&mut game, glissa_id, alice, artifact_id);
    let moved_id = game
        .find_object_by_stable_id(artifact_stable)
        .expect("destroyed artifact should still be tracked");
    assert!(
        game.player(bob)
            .expect("Bob exists")
            .graveyard
            .contains(&moved_id),
        "Glissa Sunseeker should destroy the artifact when its mana value equals your unspent mana"
    );
}

#[test]
pub(super) fn glissa_sunseeker_leaves_artifact_when_unspent_mana_differs() {
    let def = parse_oracle_card_definition("Glissa Sunseeker");
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let glissa_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let artifact_id = create_glissa_runtime_artifact(&mut game, bob, "Mismatched Artifact", 3);
    game.player_mut(alice)
        .expect("Alice exists")
        .mana_pool
        .add(ManaSymbol::Green, 2);

    resolve_glissa_conditional_destroy(&mut game, glissa_id, alice, artifact_id);
    assert!(
        game.objects_in_zone(Zone::Battlefield)
            .contains(&artifact_id),
        "Glissa Sunseeker should leave the artifact on the battlefield when unspent mana differs"
    );
    assert!(
        !zone_has_named_object(&game, Zone::Graveyard, "Mismatched Artifact"),
        "the failed conditional branch should not move the artifact to a graveyard"
    );
}

#[test]
pub(super) fn death_rattle_oni_strict_parser_and_compiled_text_regression() {
    assert_oracle_card_parses_strict("Death-Rattle Oni");
    let def = parse_oracle_card_definition("Death-Rattle Oni");
    let rendered = crate::compiled_text::compiled_text_lines(&def);
    let destroy_filter = death_rattle_oni_destroy_filter(&def);

    assert_eq!(
        rendered,
        vec![
            "Flash".to_string(),
            "This spell costs {2} less to cast for each creature that died this turn.".to_string(),
            "When this creature enters, destroy all other creatures that were dealt damage this turn.".to_string(),
        ],
        "Death-Rattle Oni compiled text should preserve flash, the dynamic cost reduction, and the dealt-damage destroy clause"
    );
    assert!(
        destroy_filter.other,
        "Death-Rattle Oni destroy filter should exclude the source creature"
    );
    assert!(
        destroy_filter.was_dealt_damage_this_turn,
        "Death-Rattle Oni destroy filter should structurally require damage dealt this turn"
    );
}

#[test]
pub(super) fn death_rattle_oni_cost_reduction_counts_creatures_that_died_this_turn() {
    let def = death_rattle_oni_definition();
    let alice = PlayerId::from_index(0);
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let oni_id = game.create_object_from_definition(&def, alice, Zone::Hand);

    let oni = game
        .object(oni_id)
        .expect("Death-Rattle Oni should be in hand");
    let base_cost = oni
        .mana_cost
        .as_ref()
        .expect("Death-Rattle Oni has a mana cost");
    assert_eq!(
        crate::decision::calculate_effective_mana_cost(&game, alice, oni, base_cost).to_oracle(),
        "{6}{B}",
        "Death-Rattle Oni should not be reduced before any creatures die this turn"
    );

    let first = create_death_rattle_runtime_creature(&mut game, alice, "First Doomed Creature");
    let second = create_death_rattle_runtime_creature(&mut game, alice, "Second Doomed Creature");
    game.move_object_by_effect(first, Zone::Graveyard);
    game.move_object_by_effect(second, Zone::Graveyard);
    assert_eq!(
        game.turn_store
            .turn_history
            .total_creatures_died_this_turn(),
        2,
        "test setup should record exactly two creatures dying this turn"
    );

    let oni = game
        .object(oni_id)
        .expect("Death-Rattle Oni should remain in hand");
    let base_cost = oni
        .mana_cost
        .as_ref()
        .expect("Death-Rattle Oni has a mana cost");
    assert_eq!(
        crate::decision::calculate_effective_mana_cost(&game, alice, oni, base_cost).to_oracle(),
        "{2}{B}",
        "two dead creatures should reduce {{6}}{{B}} by {{4}}"
    );
}

#[test]
pub(super) fn death_rattle_oni_enters_destroy_trigger_destroys_only_other_damaged_creatures() {
    let def = death_rattle_oni_definition();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let oni_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let damaged_alice =
        create_death_rattle_runtime_creature(&mut game, alice, "Damaged Alice Creature");
    let damaged_bob = create_death_rattle_runtime_creature(&mut game, bob, "Damaged Bob Creature");
    let undamaged_bob =
        create_death_rattle_runtime_creature(&mut game, bob, "Undamaged Bob Creature");
    let damage_source = create_death_rattle_runtime_creature(&mut game, bob, "Damage Source");

    record_death_rattle_damage(&mut game, damage_source, damaged_alice);
    record_death_rattle_damage(&mut game, damage_source, damaged_bob);
    record_death_rattle_damage(&mut game, damage_source, oni_id);

    let enters_event = crate::events::RawEvent::new(
        crate::events::ZoneChangeEvent::with_cause(
            oni_id,
            Zone::Hand,
            Zone::Battlefield,
            crate::events::cause::EventCause::effect(),
            None,
        ),
        crate::provenance::ProvNodeId::default(),
    );
    let mut trigger_queue = crate::triggers::TriggerQueue::new();
    for entry in crate::triggers::check_triggers(&game, &enters_event) {
        trigger_queue.add(entry);
    }
    crate::game_loop::put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("Death-Rattle Oni enters trigger should go on the stack");
    assert_eq!(
        game.stack.len(),
        1,
        "Death-Rattle Oni entering should create exactly one destroy trigger"
    );

    crate::game_loop::resolve_stack_entry(&mut game)
        .expect("Death-Rattle Oni destroy trigger should resolve");

    assert!(
        zone_has_named_object(&game, Zone::Graveyard, "Damaged Alice Creature"),
        "Death-Rattle Oni should destroy a damaged creature controlled by its controller"
    );
    assert!(
        zone_has_named_object(&game, Zone::Graveyard, "Damaged Bob Creature"),
        "Death-Rattle Oni should destroy a damaged creature controlled by an opponent"
    );
    assert!(
        game.object(undamaged_bob)
            .is_some_and(|object| object.zone == Zone::Battlefield),
        "Death-Rattle Oni should not destroy undamaged creatures"
    );
    assert!(
        game.object(oni_id)
            .is_some_and(|object| object.zone == Zone::Battlefield),
        "Death-Rattle Oni should not destroy itself even if it was dealt damage this turn"
    );
}

pub(super) fn spear_of_heliod_destroy_activated_ability(
    def: &CardDefinition,
) -> &crate::ability::ActivatedAbility {
    def.abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated)
                if activated
                    .effects
                    .flattened_default_effects()
                    .iter()
                    .any(|effect| effect.downcast_ref::<DestroyEffect>().is_some()) =>
            {
                Some(activated)
            }
            _ => None,
        })
        .expect("Spear of Heliod should have a destroy activated ability")
}

pub(super) fn spear_of_heliod_destroy_filter(
    activated: &crate::ability::ActivatedAbility,
) -> &crate::target::ObjectFilter {
    let effects = activated.effects.flattened_default_effects();
    let destroy = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<DestroyEffect>())
        .expect("Spear of Heliod activated ability should destroy a target creature");
    let ChooseSpec::Target(inner) = destroy.spec.unhinted() else {
        panic!(
            "Spear destroy effect should be targeted, got {:?}",
            destroy.spec
        );
    };
    let ChooseSpec::Object(filter) = inner.unhinted() else {
        panic!("Spear destroy target should be an object filter, got {inner:?}");
    };
    filter
}

pub(super) fn create_spear_runtime_creature(
    game: &mut crate::game_state::GameState,
    controller: PlayerId,
    name: &str,
) -> ObjectId {
    let def = CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    game.create_object_from_definition(&def, controller, Zone::Battlefield)
}

pub(super) fn record_spear_player_damage(
    game: &mut crate::game_state::GameState,
    source: ObjectId,
    player: PlayerId,
) {
    let event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::DamageEvent::with_cause(
            source,
            crate::events::DamageTarget::Player(player),
            2,
            false,
            crate::events::cause::EventCause::effect(),
        ),
        crate::provenance::ProvNodeId::default(),
    );
    game.record_turn_history_event(&event);
}

#[test]
pub(super) fn spear_of_heliod_strict_parser_and_compiled_text_regression() {
    let def = parse_oracle_card_definition("Spear of Heliod");
    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    let activated = spear_of_heliod_destroy_activated_ability(&def);
    let target_filter = spear_of_heliod_destroy_filter(activated);

    assert!(
        (rendered.contains("creatures you control get +1/+1")
            || rendered.contains("each creature you control gets +1/+1"))
            && rendered.contains("destroy target creature that dealt damage to you this turn"),
        "expected Spear of Heliod compiled text to keep anthem and combat-history destroy target, got {rendered}"
    );
    assert_eq!(
        target_filter.dealt_damage_to_player_this_turn,
        Some(PlayerFilter::You),
        "Spear of Heliod target filter should structurally require damage dealt to you this turn"
    );
    assert!(
        target_filter
            .description()
            .contains("that dealt damage to you this turn"),
        "Spear of Heliod target filter description should preserve the player damage-history clause, got {}",
        target_filter.description()
    );
}

#[test]
pub(super) fn spear_of_heliod_anthem_and_damage_history_destroy_runtime() {
    let def = parse_oracle_card_definition("Spear of Heliod");
    let activated = spear_of_heliod_destroy_activated_ability(&def);
    let target_filter = spear_of_heliod_destroy_filter(activated);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let spear = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    game.remove_summoning_sickness(spear);
    let alice_creature = create_spear_runtime_creature(&mut game, alice, "Alice Hoplite");
    let bob_creature = create_spear_runtime_creature(&mut game, bob, "Bob Raider");
    let bob_other_creature = create_spear_runtime_creature(&mut game, bob, "Bob Bystander");

    assert_eq!(
        game.current_power(alice_creature),
        Some(3),
        "Spear of Heliod should give Alice's creatures +1/+1"
    );
    assert_eq!(
        game.current_toughness(alice_creature),
        Some(3),
        "Spear of Heliod should give Alice's creatures +1/+1"
    );
    assert_eq!(
        game.current_power(bob_creature),
        Some(2),
        "Spear of Heliod should not pump opposing creatures"
    );

    let filter_ctx =
        crate::effects::ExecutionContext::new_default(spear, alice).filter_context(&game);
    assert!(
        !target_filter.matches(
            game.object(bob_creature).expect("Bob Raider should exist"),
            &filter_ctx,
            &game,
        ),
        "Spear should not be able to target a creature before it deals damage to you"
    );
    record_spear_player_damage(&mut game, bob_other_creature, bob);
    let filter_ctx =
        crate::effects::ExecutionContext::new_default(spear, alice).filter_context(&game);
    assert!(
        !target_filter.matches(
            game.object(bob_other_creature)
                .expect("Bob Bystander should exist"),
            &filter_ctx,
            &game,
        ),
        "Spear should not target a creature that dealt damage to a different player"
    );

    record_spear_player_damage(&mut game, bob_creature, alice);
    let filter_ctx =
        crate::effects::ExecutionContext::new_default(spear, alice).filter_context(&game);
    assert!(
        target_filter.matches(
            game.object(bob_creature).expect("Bob Raider should exist"),
            &filter_ctx,
            &game,
        ),
        "Spear should target a creature that dealt damage to you this turn"
    );

    game.player_mut(alice)
        .expect("Alice should exist")
        .mana_pool
        .white = 3;
    crate::cost::can_pay_cost(&game, spear, alice, &activated.mana_cost).expect(
        "Spear activation cost should be payable with three white mana and an untapped source",
    );
    let mut dm = crate::decision::AutoPassDecisionMaker::default();
    crate::special_actions::pay_total_cost_with_choice(
        &mut game,
        alice,
        spear,
        &activated.mana_cost,
        crate::costs::PaymentReason::ActivateAbility,
        &mut dm,
    )
    .expect("Spear activation cost should be paid");
    assert!(game.is_tapped(spear), "Spear activation cost should tap it");
    assert_eq!(
        game.player(alice)
            .expect("Alice should exist")
            .mana_pool
            .total(),
        0,
        "Spear activation cost should spend {{1}}{{W}}{{W}}"
    );

    let mut ctx = crate::effects::ExecutionContext::new_default(spear, alice)
        .with_targets(vec![crate::effects::ResolvedTarget::Object(bob_creature)]);
    for effect in activated.effects.flattened_default_effects() {
        crate::effects::execute_effect(&mut game, effect, &mut ctx)
            .expect("Spear destroy activation should resolve");
    }
    assert!(
        game.player(bob)
            .expect("Bob should exist")
            .graveyard
            .iter()
            .any(|id| game
                .object(*id)
                .is_some_and(|object| object.name == "Bob Raider")),
        "Spear should destroy the creature that dealt damage to you this turn"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_exile_target_creature_and_target_land_sentence() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Grip Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Exile target creature and target land.")
        .expect("parse exile with two target objects");

    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("exile target creature") && joined.contains("target land"),
        "expected both exile targets in rendered text, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_destroy_target_creature_and_target_land_sentence() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Spiteful Blow Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Destroy target creature and target land.")
        .expect("parse destroy with two target objects");

    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("destroy target creature") && joined.contains("target land"),
        "expected both destroy targets in rendered text, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_destroy_up_to_one_each_target_type_sentence() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Convert Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Destroy up to one target artifact, up to one target creature, and up to one target enchantment.",
        )
        .expect("parse destroy up-to-one multi-target sentence");

    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("up to one target artifact")
            && joined.contains("up to one target creature")
            && joined.contains("up to one target enchantment"),
        "expected three up-to-one target destroy clauses, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_destroy_source_and_target_blocking_sentence() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Wall of Vipers Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("{3}: Destroy this creature and target creature it's blocking.")
        .expect("parse destroy source + target creature it's blocking");

    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        (joined.contains("destroy this creature") || joined.contains("destroy this permanent"))
            && (joined.contains("target creature its blocking")
                || joined.contains("target blocking creature")),
        "expected source + blocking target destroy effects, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_lesser_werewolf_activated_ability() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(54), "Lesser Werewolf")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "{B}: If this creature's power is 1 or more, it gets -1/-0 until end of turn and put a -0/-1 counter on target creature blocking or blocked by this creature. Activate only during the declare blockers step.",
        )
        .expect("parse Lesser Werewolf activated ability");

    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("this creature's power is 1 or more")
            || joined.contains("this creatures power is 1 or more")
            || joined.contains("this has power 1 or greater"),
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

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_destroy_target_artifact_creature_enchantment_and_land_sentence() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Decimate Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Destroy target artifact, target creature, target enchantment, and target land.",
        )
        .expect("parse four-target destroy sentence");

    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("destroy target artifact")
            && joined.contains("target creature")
            && joined.contains("target enchantment")
            && joined.contains("target land"),
        "expected four destroy targets in rendered text, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_exile_self_and_target_unless_controller_pays() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Carrionette Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "{2}{B}{B}: Exile this card and target creature unless that creature's controller pays {2}. Activate only if this card is in your graveyard.",
        )
        .expect("parse exile self + target creature unless pays");

    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("unless that creature's controller pays {2}")
            || joined.contains("unless that creatures controller pays {2}")
            || joined.contains("unless that object's controller pays {2}")
            || joined.contains("unless that objects controller pays {2}")
            || joined.contains("unless its controller pays {2}"),
        "expected unless-payment tail in rendered text, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_exile_two_graveyard_targets_for_spelltwine_pattern() {
    CardDefinitionBuilder::new(CardId::from_raw(1), "Spelltwine Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Exile target instant or sorcery card from your graveyard and target instant or sorcery card from an opponent's graveyard.",
        )
        .expect("parse dual-target exile across graveyards");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_exile_named_source_and_target_permanent() {
    CardDefinitionBuilder::new(CardId::from_raw(1), "Mangara Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("{T}: Exile Mangara and target permanent.")
        .expect("parse exile named source and target permanent");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_next_damage_redirect_to_target_creature() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Nomads en-Kor Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "{0}: The next 1 damage that would be dealt to this creature this turn is dealt to target creature you control instead.",
        )
        .expect("parse next-damage redirect clause");

    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains(
            "the next 1 damage that would be dealt to this creature this turn is dealt to target creature you control instead"
        ),
        "expected redirected-next-damage text in compiled output, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_next_time_source_damage_redirect_to_this_creature() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Shaman en-Kor Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "{1}{W}: The next time a source of your choice would deal damage to target creature this turn, that damage is dealt to this creature instead.",
        )
        .expect("parse next-time source redirect clause");

    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains(
            "the next time a source of your choice would deal damage to target creature this turn, that damage is dealt to this creature instead"
        ),
        "expected next-time source redirect text in compiled output, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn jade_monolith_parses_and_renders_source_damage_redirect_to_you() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Jade Monolith")
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "{1}: The next time a source of your choice would deal damage to target creature this turn, that source deals that damage to you instead.",
        )
        .expect("Jade Monolith should parse strictly");

    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains(
            "the next time a source of your choice would deal damage to target creature this turn, that source deals that damage to you instead"
        ),
        "expected Jade Monolith redirect text in compiled output, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn vassals_duty_parses_and_renders_protected_target_redirect_to_you() {
    let def = parse_oracle_card_definition("Vassal's Duty");

    let joined = canonical_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains(
            "the next 1 damage that would be dealt to target legendary creature you control this turn is dealt to you instead"
        ),
        "expected Vassal's Duty redirect text in compiled output, got {joined}"
    );

    let debug = format!("{:#?}", def.abilities);
    assert!(
        debug.contains("RedirectNextDamageToTargetEffect")
            && debug.contains("protected_target: Some")
            && debug.contains("destination: Controller")
            && debug.contains("Legendary"),
        "Vassal's Duty should lower to partial damage redirection from a legendary protected target to you, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn generals_regalia_parses_and_renders_redirect_to_target_creature_you_control() {
    let def = parse_oracle_card_definition("General's Regalia");

    let joined = canonical_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains(
            "the next time a source of your choice would deal damage to you this turn, that damage is dealt to target creature you control instead"
        ),
        "expected General's Regalia redirect text in compiled output, got {joined}"
    );

    let debug = format!("{:#?}", def.abilities);
    assert!(
        debug.contains("RedirectNextTimeDamageToSourceEffect")
            && debug.contains("destination: TargetObject")
            && debug.contains("destination_target: Some"),
        "General's Regalia should lower to next-time damage redirection to a target object, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn opponent_chosen_redirect_destination_remains_unsupported() {
    let err = CardDefinitionBuilder::new(CardId::from_raw(1), "Nova Pentacle Variant")
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "{3}, {T}: The next time a source of your choice would deal damage to you this turn, that damage is dealt to target creature of an opponent's choice instead.",
        )
        .expect_err("opponent-chosen destination target should remain unsupported");

    let err = format!("{err:?}");
    assert!(
        err.contains("unsupported redirected-next-time damage destination"),
        "expected unsupported redirected destination error, got {err}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn oracles_attendants_parses_and_renders_all_damage_source_redirect_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Oracle's Attendants")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "{T}: All damage that would be dealt to target creature this turn by a source of your choice is dealt to this creature instead.",
        )
        .expect("Oracle's Attendants should parse");

    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("all damage that would be dealt to target creature this turn by a source of your choice is dealt to this creature instead"),
        "expected all-damage source redirect text in compiled output, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_spells_cost_modifier_subtype_does_not_force_creature_word() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Dinosaur Cost Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Dinosaur spells you cast cost {1} less to cast.")
        .expect("parse subtype-only spell cost reduction");

    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("dinosaur spells you cast cost {1} less to cast"),
        "expected subtype-only spell description, got {joined}"
    );
    assert!(
        !joined.contains("dinosaur creature spells"),
        "did not expect redundant creature word in subtype-only spell description, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_transform_source_uses_artifact_self_reference_for_artifacts() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Mysterious Tome Variant")
        .card_types(vec![CardType::Artifact])
        .parse_text("{2}, {T}: Draw a card. Transform this artifact.")
        .expect("parse transform-this-artifact activated ability");

    let joined = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        joined.contains("Transform this artifact"),
        "expected artifact self-reference for transform source, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_choose_between_modes_as_choose_one_or_more() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Modal Variant")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Choose one or more —\n• Destroy target artifact.\n• Destroy target enchantment.\n• Destroy target land.",
        )
        .expect("parse modal choose-one-or-more clause");

    let joined = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        joined.contains("Choose one or more"),
        "expected normalized choose-one-or-more header, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn soul_transfer_parses_and_keeps_choose_both_instead_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Soul Transfer")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Choose one. If you control an artifact and an enchantment as you cast this spell, you may choose both instead.\n\
• Exile target creature or planeswalker.\n\
• Return target creature or planeswalker card from your graveyard to your hand.",
        )
        .expect("Soul Transfer should parse");

    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("choose one") && joined.contains("you may choose both instead"),
        "expected Soul Transfer choose-both conditional wording, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_each_player_create_clause_uses_each_player_creates() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Dragon Crowd")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Each player creates a 5/5 red Dragon creature token with flying.")
        .expect("parse each-player create clause");

    let joined = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        joined.contains("Each player creates a 5/5 red Dragon creature token with flying"),
        "expected each-player create compaction, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_put_counter_on_each_attacking_creature_from_for_each_form() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Fumes Variant")
        .card_types(vec![CardType::Instant])
        .parse_text("Put a -1/-1 counter on each attacking creature.")
        .expect("parse each-attacking counter clause");

    let joined = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        joined.contains("Put a -1/-1 counter on each attacking creature"),
        "expected normalized each-attacking counter wording, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn explicit_you_create_actor_surface_regressions() {
    let cases = [
        (
            "Fake Your Own Death",
            vec![CardType::Instant],
            Vec::new(),
            "Until end of turn, target creature gets +2/+0 and gains \"When this creature dies, return it to the battlefield tapped under its owner's control and you create a Treasure token.\"",
            "treasure token",
        ),
        (
            "Liliana's Reaver",
            vec![CardType::Creature],
            Vec::new(),
            "Whenever this creature deals combat damage to a player, that player discards a card and you create a tapped 2/2 black Zombie creature token.",
            "tapped 2/2 black zombie creature token",
        ),
        (
            "Nurgle's Rot",
            vec![CardType::Enchantment],
            vec![Subtype::Aura],
            "When enchanted creature dies, return this card to its owner's hand and you create a 1/3 black Demon creature token named Plaguebearer of Nurgle.",
            "1/3 black demon creature token named plaguebearer of nurgle",
        ),
        (
            "Sleep with the Fishes",
            vec![CardType::Enchantment],
            vec![Subtype::Aura],
            "When this Aura enters, tap enchanted creature and you create a 1/1 blue Fish creature token with \"This token can't be blocked.\"",
            "1/1 blue fish creature token",
        ),
    ];

    for (name, card_types, subtypes, oracle, token_phrase) in cases {
        let def = CardDefinitionBuilder::new(CardId::new(), name)
            .card_types(card_types)
            .subtypes(subtypes)
            .parse_text(oracle)
            .unwrap_or_else(|error| panic!("{name} should parse: {error:?}"));
        let compiled = unprocessed_compiled_lines(&def)
            .join(" ")
            .to_ascii_lowercase();
        assert!(
            compiled.contains("and you create") && compiled.contains(token_phrase),
            "expected {name} to preserve its explicit create actor, got {compiled}"
        );
    }

    let implicit = CardDefinitionBuilder::new(CardId::new(), "Implicit Create Control")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Create a Treasure token.")
        .expect("bare imperative create should parse");
    let compiled = unprocessed_compiled_lines(&implicit)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        compiled.contains("create a treasure token") && !compiled.contains("you create"),
        "expected bare imperative create to remain implicit, got {compiled}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn attached_pt_conjunctions_keep_both_layered_static_effects() {
    use crate::static_abilities::StaticAbilityId;

    let cases = [
        (
            "Fresh Start",
            "Flash\nEnchant creature\nEnchanted creature gets -5/-0 and loses all abilities.",
            StaticAbilityId::RemoveAllAbilitiesForFilter,
            "enchanted creature gets -5/-0 and loses all abilities",
        ),
        (
            "Duskmourn's Domination",
            "Enchant creature\nYou control enchanted creature.\nEnchanted creature gets -3/-0 and loses all abilities.",
            StaticAbilityId::RemoveAllAbilitiesForFilter,
            "enchanted creature gets -3/-0 and loses all abilities",
        ),
        (
            "Mystic Subdual",
            "Flash\nEnchant creature\nEnchanted creature gets -2/-0 and loses all abilities. (Mutating onto the creature won't give it new abilities. It can gain abilities in other ways.)",
            StaticAbilityId::RemoveAllAbilitiesForFilter,
            "enchanted creature gets -2/-0 and loses all abilities",
        ),
        (
            "Sinister Strength",
            "Enchant creature\nEnchanted creature gets +3/+1 and is black.",
            StaticAbilityId::SetColors,
            "enchanted creature gets +3/+1 and is black",
        ),
    ];

    let expected_filter = ObjectFilter::creature()
        .in_zone(Zone::Battlefield)
        .match_tagged(
            "enchanted",
            crate::target::TaggedOpbjectRelation::IsTaggedObject,
        );

    for (name, oracle, expected_secondary, expected_surface) in cases {
        let def = CardDefinitionBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Enchantment])
            .subtypes(vec![Subtype::Aura])
            .parse_text(oracle)
            .unwrap_or_else(|error| panic!("{name} should parse: {error:?}"));

        let mut anthem_filter = None;
        let mut secondary_filter = None;
        for ability in &def.abilities {
            let AbilityKind::Static(static_ability) = &ability.kind else {
                continue;
            };
            let Some(model) = static_ability.compiled_model() else {
                continue;
            };
            match &model.payload {
                ironsmith_core::StaticAbilityPayload::Anthem(anthem) => {
                    anthem_filter = anthem.filter.clone();
                }
                ironsmith_core::StaticAbilityPayload::RemoveAllAbilities(filter)
                    if expected_secondary == StaticAbilityId::RemoveAllAbilitiesForFilter =>
                {
                    secondary_filter = Some(filter.clone());
                }
                ironsmith_core::StaticAbilityPayload::SetColors { filter, .. }
                    if expected_secondary == StaticAbilityId::SetColors =>
                {
                    secondary_filter = Some(filter.clone());
                }
                _ => {}
            }
        }

        assert_eq!(
            anthem_filter.as_ref(),
            Some(&expected_filter),
            "{name} should retain its enchanted-creature anthem filter"
        );
        assert_eq!(
            secondary_filter, anthem_filter,
            "{name} should use the identical filter for both layered effects"
        );

        let compiled = unprocessed_compiled_lines(&def)
            .join(" ")
            .to_ascii_lowercase();
        assert!(
            compiled.contains(expected_surface),
            "{name} should structurally recombine both effects, got {compiled}"
        );
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn possessed_threshold_source_modifier_family_recombines() {
    let cases = [
        (
            "Possessed Aven",
            "Flying",
            "blue",
            "Flying\nThreshold — As long as there are seven or more cards in your graveyard, this creature gets +1/+1, is black, and has \"{2}{B}, {T}: Destroy target blue creature.\"",
        ),
        (
            "Possessed Barbarian",
            "First strike",
            "red",
            "First strike\nThreshold — As long as there are seven or more cards in your graveyard, this creature gets +1/+1, is black, and has \"{2}{B}, {T}: Destroy target red creature.\"",
        ),
        (
            "Possessed Centaur",
            "Trample",
            "green",
            "Trample\nThreshold — As long as there are seven or more cards in your graveyard, this creature gets +1/+1, is black, and has \"{2}{B}, {T}: Destroy target green creature.\"",
        ),
        (
            "Possessed Nomad",
            "Vigilance",
            "white",
            "Vigilance\nThreshold — As long as there are seven or more cards in your graveyard, this creature gets +1/+1, is black, and has \"{2}{B}, {T}: Destroy target white creature.\"",
        ),
    ];

    for (name, intrinsic, destroyed_color, oracle) in cases {
        let def = CardDefinitionBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Creature])
            .parse_text(oracle)
            .unwrap_or_else(|error| panic!("{name} should parse: {error:?}"));
        let expected = format!(
            "Threshold — As long as there are seven or more cards in your graveyard, this creature gets +1/+1, is black, and has \"{{2}}{{B}}, {{T}}: Destroy target {destroyed_color} creature.\""
        );
        let rendered = unprocessed_compiled_lines(&def);
        assert!(
            rendered.iter().any(|line| line == intrinsic),
            "{name} should retain its intrinsic keyword, got {rendered:#?}"
        );
        assert!(
            rendered.iter().any(|line| line == &expected),
            "{name} should retain its structural Threshold surface, got {rendered:#?}"
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
            static_ids.contains(&crate::static_abilities::StaticAbilityId::Anthem)
                && static_ids.contains(&crate::static_abilities::StaticAbilityId::SetColors)
                && static_ids.contains(
                    &crate::static_abilities::StaticAbilityId::GrantObjectAbilityForFilter
                ),
            "{name} should retain all three executable static siblings, got {static_ids:?}"
        );
    }
}
