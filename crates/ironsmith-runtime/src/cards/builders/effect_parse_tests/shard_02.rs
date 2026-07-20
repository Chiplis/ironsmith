#![allow(unused_imports)]
use super::shard_00::*;
use super::shard_01::*;
use super::*;

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_conditional_equipment_granted_static_chain() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Rune of Might Variant")
            .parse_text(
                "Enchant permanent\nAs long as enchanted permanent is an Equipment, it has \"Equipped creature gets +1/+1 and has trample.\"",
            )
            .expect("conditional granted static chain for equipment should parse");

    let displays: Vec<String> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.display()),
            _ => None,
        })
        .collect();
    assert!(
        displays.iter().any(|display| {
            (display.contains("gets +1/+1") || display.contains("get +1/+1"))
                && display.contains("as long as enchanted permanent is an equipment")
        }),
        "expected conditional granted pump static, got: {displays:?}"
    );
    assert!(
        displays.iter().any(|display| {
            (display.contains("has trample") || display.contains("have trample"))
                && display.contains("as long as enchanted permanent is an equipment")
        }),
        "expected conditional granted trample static, got: {displays:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_soulbond_shared_attack_mill_equal_to_toughness() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Imperious Mindbreaker Variant")
            .parse_text(
                "Soulbond (You may pair this creature with another unpaired creature when either enters. They remain paired for as long as you control both of them.)\nAs long as this creature is paired with another creature, each of those creatures has \"Whenever this creature attacks, each opponent mills cards equal to its toughness.\"",
            )
            .expect("soulbond shared mill-by-toughness line should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();
    assert!(
        rendered_lower.contains("mill"),
        "expected granted mill trigger in compiled text, got: {rendered}"
    );
    assert!(
        rendered_lower.contains("toughness"),
        "expected mill count to reference toughness, got: {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_doom_weaver_soulbond_shared_dies_draw_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Doom Weaver")
            .parse_text(
                "Reach\nSoulbond (You may pair this creature with another unpaired creature when either enters. They remain paired for as long as you control both of them.)\nAs long as Doom Weaver is paired with another creature, each of those creatures has \"When this creature dies, draw cards equal to its power.\"",
            )
            .expect("Doom Weaver should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();
    assert!(
        rendered_lower.contains("as long as this creature is paired with another creature"),
        "expected soulbond shared condition in compiled text, got: {rendered}"
    );
    assert!(
        rendered_lower.contains("each of those creatures has"),
        "expected soulbond shared recipient wording in compiled text, got: {rendered}"
    );
    assert!(
        rendered_lower.contains("when this creature dies")
            && rendered_lower.contains("draw cards equal to its power"),
        "expected granted dies trigger draw clause in compiled text, got: {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_soulbond_shared_copy_clause_can_lose_soulbond() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Mirage Phalanx Variant")
        .parse_text(
            "Soulbond (You may pair this creature with another unpaired creature when either enters. They remain paired for as long as you control both of them.)\nAs long as this creature is paired with another creature, each of those creatures has \"At the beginning of combat on your turn, create a token that's a copy of this creature, except it has haste and loses soulbond. Exile it at end of combat.\"",
        )
        .expect("soulbond shared copy clause should parse");
    assert!(
        format!("{def:?}").contains("SoulbondPairEffect")
            && format!("{def:?}").contains("RemoveAbilityForFilter")
            && format!("{def:?}").contains("display: \\\"Soulbond\\\""),
        "expected token-copy haste and soulbond removal semantics, got: {def:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_static_condition_this_is_equipped_variant() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Armory Veteran Variant")
        .parse_text("As long as this is equipped, it has trample.")
        .expect("this-is-equipped static condition should parse");

    let displays: Vec<String> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.display()),
            _ => None,
        })
        .collect();
    assert!(
        displays.iter().any(|display| {
            display.contains("as long as this creature is equipped")
                && display.contains("has trample")
        }),
        "expected equipped-gated trample grant, got: {displays:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_static_condition_this_creature_is_untapped_variant() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Untapped Condition Variant")
        .parse_text("As long as this creature is untapped, this creature has vigilance.")
        .expect("this-creature-is-untapped static condition should parse");

    let displays: Vec<String> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.display()),
            _ => None,
        })
        .collect();
    assert!(
        displays.iter().any(|display| {
            display.contains("as long as this creature is untapped")
                && display.contains("has vigilance")
        }),
        "expected untapped-gated vigilance grant, got: {displays:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_static_condition_you_own_card_exiled_with_counter() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Rex Condition Variant")
        .parse_text(
            "As long as you own a card exiled with a brain counter, this creature has vigilance.",
        )
        .expect("ownership-based exile counter condition should parse");

    let displays: Vec<String> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.display()),
            _ => None,
        })
        .collect();
    assert!(
        displays.iter().any(|display| {
            display.contains("as long as you own a card exiled with a brain counter")
                && display.contains("has vigilance")
        }),
        "expected ownership-gated vigilance grant, got: {displays:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_threshold_additional_anthem_keeps_condition() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Divine Sacrament Variant")
            .parse_text(
                "Threshold — White creatures get an additional +1/+1 as long as there are seven or more cards in your graveyard.",
            )
            .expect("threshold anthem with additional bonus should parse");

    assert_eq!(def.abilities.len(), 1, "expected one static ability");
    let display = match &def.abilities[0].kind {
        AbilityKind::Static(static_ability) => static_ability.display(),
        other => panic!("expected static ability, got {other:?}"),
    };
    assert!(
        display.contains("white creatures get +1/+1"),
        "expected anthem bonus to parse, got: {display}"
    );
    assert!(
        display.contains("as long as there are seven or more cards in your graveyard"),
        "expected threshold condition to be preserved, got: {display}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_threshold_enchanted_creature_has_keyword_condition() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Aboshan Variant")
            .parse_text(
                "Threshold — Enchanted creature has shroud as long as there are seven or more cards in your graveyard.",
            )
            .expect("threshold enchanted-creature keyword line should parse");

    assert_eq!(def.abilities.len(), 1, "expected one static ability");
    let display = match &def.abilities[0].kind {
        AbilityKind::Static(static_ability) => static_ability.display(),
        other => panic!("expected static ability, got {other:?}"),
    };
    assert!(
        display.contains("enchanted creature has shroud")
            && display.contains("as long as there are seven or more cards in your graveyard"),
        "expected conditional enchanted keyword grant, got: {display}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_threshold_cant_be_blocked_condition() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Cephalid Variant")
            .parse_text(
                "Threshold — This creature can't be blocked as long as there are seven or more cards in your graveyard.",
            )
            .expect("conditional cant-be-blocked line should parse");

    assert_eq!(def.abilities.len(), 1, "expected one static ability");
    let display = match &def.abilities[0].kind {
        AbilityKind::Static(static_ability) => static_ability.display(),
        other => panic!("expected static ability, got {other:?}"),
    };
    let display_lc = display.to_ascii_lowercase();
    assert!(
        display_lc.contains("can't be blocked")
            && display_lc.contains("as long as there are seven or more cards in your graveyard"),
        "expected conditional unblockable grant, got: {display}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_delirium_spell_keyword_has_hand_and_stack_zones() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Conditional Flash Variant")
            .parse_text(
                "Delirium — This spell has flash as long as there are five or more mana values among cards in your graveyard.",
            )
            .expect("conditional spell keyword line should parse");

    assert_eq!(def.abilities.len(), 1, "expected one static ability");
    let ability = &def.abilities[0];
    match &ability.kind {
        AbilityKind::Static(static_ability) => {
            assert_eq!(
                static_ability.id(),
                crate::static_abilities::StaticAbilityId::ConditionalSpellKeyword,
                "expected conditional spell keyword static ability id"
            );
        }
        other => panic!("expected static ability, got {other:?}"),
    }
    assert!(
        ability.functions_in(&Zone::Hand),
        "conditional spell keyword should function in hand"
    );
    assert!(
        ability.functions_in(&Zone::Stack),
        "conditional spell keyword should function on stack"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_delirium_can_attack_as_though_no_defender_condition() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Geist Variant")
            .parse_text(
                "Delirium — This creature can attack as though it didn't have defender as long as there are four or more card types among cards in your graveyard.",
            )
            .expect("conditional can-attack-with-defender line should parse");

    assert_eq!(def.abilities.len(), 1, "expected one static ability");
    let display = match &def.abilities[0].kind {
        AbilityKind::Static(static_ability) => static_ability.display(),
        other => panic!("expected static ability, got {other:?}"),
    };
    let display_lc = display.to_ascii_lowercase();
    assert!(
        display_lc.contains("can attack as though it didn't have defender")
            && display_lc.contains("as long as there are")
            && display_lc.contains("card types among cards in your graveyard"),
        "expected conditional defender-override grant, got: {display}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_delirium_maximum_hand_size_formula_line() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Winter Variant")
            .parse_text(
                "Delirium — As long as there are four or more card types among cards in your graveyard, each opponent's maximum hand size is equal to seven minus the number of those card types.",
            )
            .expect("conditional maximum-hand-size formula should parse");

    assert_eq!(def.abilities.len(), 1, "expected one static ability");
    let ability = match &def.abilities[0].kind {
        AbilityKind::Static(static_ability) => static_ability,
        other => panic!("expected static ability, got {other:?}"),
    };
    let debug = format!("{ability:?}");
    assert!(
        ability.id() == crate::static_abilities::StaticAbilityId::GrantAbility
            && debug.contains("MaximumHandSizeSevenMinusYourGraveyardCardTypes"),
        "expected typed grant of the max-hand-size formula ability, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_conditional_multi_keyword_grant_keeps_all_keywords() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Conditional Multi Keyword Variant")
        .parse_text(
            "As long as you control an artifact, this creature has trample and indestructible.",
        )
        .expect("parse conditional multi-keyword grant");

    let displays: Vec<String> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.display()),
            _ => None,
        })
        .collect();
    assert!(
        displays.iter().any(|display| {
            display.contains("has trample")
                && display.contains("as long as you control an artifact")
        }),
        "expected conditional trample ability, got: {displays:?}"
    );
    assert!(
        displays.iter().any(|display| {
            display.contains("has indestructible")
                && display.contains("as long as you control an artifact")
        }),
        "expected conditional indestructible ability, got: {displays:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_static_anthem_with_terminal_period() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Dead Weight Style Variant")
        .parse_text("Enchanted creature gets -2/-2.")
        .expect("terminal period should not break static anthem parsing");

    assert_eq!(def.abilities.len(), 1, "expected one static ability");
    let display = match &def.abilities[0].kind {
        AbilityKind::Static(static_ability) => static_ability.display(),
        other => panic!("expected static ability, got {other:?}"),
    };
    assert!(
        display.contains("enchanted creature gets -2/-2"),
        "expected enchanted anthem display, got: {display}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_creatures_you_control_anthem_with_terminal_period() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Simple Team Anthem Variant")
        .parse_text("Creatures you control get +1/+1.")
        .expect("terminal period should not break team anthem parsing");

    assert_eq!(def.abilities.len(), 1, "expected one static ability");
    let display = match &def.abilities[0].kind {
        AbilityKind::Static(static_ability) => static_ability.display(),
        other => panic!("expected static ability, got {other:?}"),
    };
    assert!(
        display.contains("+1/+1"),
        "expected parsed anthem modifier in display, got: {display}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_granted_keyword_and_must_attack_clause_keeps_both_parts() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Hellraiser Variant")
        .parse_text("Creatures you control have haste and attack each combat if able.")
        .expect_err(
            "granted keyword + must-attack line should fail until full anthem subject support",
        );
    let message = format!("{err:?}");
    assert!(
        message.contains("unsupported anthem subject"),
        "expected unsupported anthem-subject parse error, got {message}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_anthem_and_unblockable_static_line() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Aether Tunnel Variant")
        .parse_text("Enchanted creature gets +1/+0 and can't be blocked.")
        .expect("anthem + unblockable static line should parse");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("power: Fixed(1)") && debug.contains("toughness: Fixed(0)"),
        "expected +1/+0 anthem, got: {debug}"
    );
    assert!(
        debug.contains("Unblockable"),
        "expected granted unblockable static ability, got: {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_anthem_and_changeling_static_line() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Amorphous Axe Variant")
        .parse_text("Equipped creature gets +3/+0 and is every creature type.")
        .expect("anthem + changeling static line should parse");

    let debug = format!("{:?}", def.abilities);
    let static_ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        debug.contains("power: Fixed(3)") && debug.contains("toughness: Fixed(0)"),
        "expected +3/+0 anthem, got: {debug}"
    );
    assert_eq!(
        static_ids
            .iter()
            .filter(|id| **id == crate::static_abilities::StaticAbilityId::AddAllSubtypesOfFamily)
            .count(),
        1,
        "expected generic every-creature-type static ability, got: {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_creature_spells_are_every_creature_type_line() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Maskwood Stack Variant")
        .parse_text("Creature spells you control are every creature type.")
        .expect("stack creature-type static line should parse");

    let debug = format!("{:#?}", def.abilities);
    let compact = debug.split_whitespace().collect::<String>();
    let static_ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();

    assert_eq!(
        static_ids
            .iter()
            .filter(|id| **id == crate::static_abilities::StaticAbilityId::AddAllSubtypesOfFamily)
            .count(),
        1,
        "expected generic every-creature-type stack effect, got: {debug}"
    );
    assert!(
        compact.contains("zone:Some(Stack")
            && compact.contains("card_types:[Creature")
            && compact.contains("has_mana_cost:true"),
        "expected stack creature-spell filter, got: {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_nonbattlefield_creature_cards_are_every_creature_type_line() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Maskwood Off-Battlefield Variant")
        .parse_text(
            "Creature cards you own that aren't on the battlefield are every creature type.",
        )
        .expect("off-battlefield creature-card static line should parse");

    let debug = format!("{:#?}", def.abilities);
    let compact = debug.split_whitespace().collect::<String>();
    let static_ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();

    assert_eq!(
        static_ids
            .iter()
            .filter(|id| **id == crate::static_abilities::StaticAbilityId::AddAllSubtypesOfFamily)
            .count(),
        1,
        "expected generic every-creature-type off-battlefield effect, got: {debug}"
    );
    assert!(
        compact.contains("any_of:[")
            && compact.contains("owner:Some(You")
            && compact.contains("card_types:[Creature")
            && compact.contains("zone:Some(Hand")
            && compact.contains("zone:Some(Library")
            && compact.contains("zone:Some(Graveyard")
            && compact.contains("zone:Some(Exile")
            && compact.contains("zone:Some(Command"),
        "expected off-battlefield card filter to fan out across non-battlefield zones, got: {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_permanent_spells_are_artifacts_in_addition_to_their_other_types_line() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Encroaching Stack Variant")
        .parse_text("Permanent spells you control are artifacts in addition to their other types.")
        .expect("stack permanent-spell type-addition line should parse");

    let debug = format!("{:#?}", def.abilities);
    let compact = debug.split_whitespace().collect::<String>();
    let static_ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();

    assert_eq!(
        static_ids
            .iter()
            .filter(|id| **id == crate::static_abilities::StaticAbilityId::AddCardTypes)
            .count(),
        1,
        "expected generic card-type addition stack effect, got: {debug}"
    );
    assert!(
        compact.contains("zone:Some(Stack")
            && compact.contains("controller:Some(You")
            && compact.contains("has_mana_cost:true"),
        "expected stack permanent-spell filter, got: {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_nonbattlefield_nonland_permanent_cards_are_artifacts_in_addition_line() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Encroaching Off-Battlefield Variant")
            .parse_text(
                "Nonland permanent cards you own that aren't on the battlefield are artifacts in addition to their other types.",
            )
            .expect("off-battlefield permanent-card type-addition line should parse");

    let debug = format!("{:#?}", def.abilities);
    let compact = debug.split_whitespace().collect::<String>();
    let static_ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();

    assert_eq!(
        static_ids
            .iter()
            .filter(|id| **id == crate::static_abilities::StaticAbilityId::AddCardTypes)
            .count(),
        1,
        "expected generic card-type addition off-battlefield effect, got: {debug}"
    );
    assert!(
        compact.contains("any_of:[")
            && compact.contains("owner:Some(You")
            && compact.contains("excluded_card_types:[Land")
            && compact.contains("zone:Some(Hand")
            && compact.contains("zone:Some(Library")
            && compact.contains("zone:Some(Graveyard")
            && compact.contains("zone:Some(Exile")
            && compact.contains("zone:Some(Command"),
        "expected off-battlefield permanent-card filter to fan out across non-battlefield zones, got: {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_enchanted_permanent_doesnt_untap_static_line() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Coma Veil Variant")
            .parse_text(
                "Enchant artifact or creature.\nEnchanted permanent doesn't untap during its controller's untap step.",
            )
            .expect("enchanted permanent doesnt-untap line should parse");

    let compiled = unprocessed_compiled_lines(&def)
        .join(" | ")
        .to_ascii_lowercase();
    assert!(
        compiled.contains("enchanted permanent doesnt untap during its controllers untap step")
            || compiled
                .contains("enchanted permanent doesn't untap during its controller's untap step")
            || compiled
                .contains("enchanted permanent don't untap during their controllers' untap steps"),
        "expected compiled untap restriction text, got: {compiled}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_static_gets_rejects_unsupported_trailing_clause() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Unsupported Static Tail Variant")
        .parse_text("This creature gets +1/+1 unless you control an artifact.")
        .expect_err("unsupported static tail should fail parsing");

    let message = format!("{err:?}");
    assert!(
        message.contains("unsupported trailing anthem clause"),
        "expected trailing-clause parse error, got: {message}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_mill_then_put_from_among_into_hand_with_if_you_dont() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Ainok Wayfarer Variant")
            .parse_text(
                "When this creature enters, mill three cards. You may put a land card from among them into your hand. If you don't, put a +1/+1 counter on this creature.",
            )
            .expect("mill plus put-from-among clause should parse");

    let debug = format!("{:#?}", def).to_ascii_lowercase();
    assert!(
        debug.contains("milleffect")
            && debug.contains("chooseobjectseffect")
            && debug.contains("zone: some(")
            && debug.contains("graveyard")
            && debug.contains("putcounterseffect"),
        "expected mill -> choose-from-graveyard -> fallback-counter lowering, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_mill_then_put_from_among_into_hand() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Six Variant")
            .parse_text(
                "When this creature enters, mill three cards. You may put a land card from among them into your hand.",
            )
            .expect("mill plus put-from-among clause should parse");

    let debug = format!("{:#?}", def).to_ascii_lowercase();
    assert!(
        debug.contains("milleffect")
            && debug.contains("chooseobjectseffect")
            && debug.contains("zone: some(")
            && debug.contains("graveyard"),
        "expected mill -> choose-from-graveyard lowering, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_mill_with_trailing_clause_fails_instead_of_silently_partial_parsing() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Midnight Tilling Variant")
        .parse_text(
            "Mill four cards, then you may return a permanent card from among them to your hand.",
        )
        .expect_err("mill with trailing from-among clause should fail until supported");

    let message = format!("{err:?}");
    assert!(
        message.contains("unsupported trailing mill clause"),
        "expected strict trailing-clause mill parse error, got {message}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_fireblast_style_alternative_cost_line_from_text() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Fireblast")
            .parse_text(
                "You may sacrifice two Mountains rather than pay this spell's mana cost.\nFireblast deals 4 damage to any target.",
            )
            .expect("parse fireblast-style alternative cost");

    assert_eq!(def.alternative_casts.len(), 1);
    let alt = &def.alternative_casts[0];
    match alt {
        AlternativeCastingMethod::Composed { total_cost, .. } => {
            let mana_cost = total_cost.mana_cost();
            let costs = alt.non_mana_costs();
            assert!(mana_cost.is_none(), "fireblast alt cost should be no mana");
            let has_sacrifice = costs
                .iter()
                .filter_map(|cost| cost.effect_ref())
                .any(|effect| effect.downcast_ref::<SacrificeEffect>().is_some());
            assert!(
                has_sacrifice,
                "expected sacrifice effect in alternative cost"
            );
            let sacrifice = costs
                .iter()
                .filter_map(|cost| cost.effect_ref())
                .find_map(|effect| effect.downcast_ref::<SacrificeEffect>())
                .expect("missing sacrifice effect");
            assert_eq!(sacrifice.count, Value::Fixed(2));
        }
        other => panic!("expected Composed, got {other:?}"),
    }

    let spell_effect = def.spell_effect.expect("spell effect");
    assert!(
        spell_effect.all_effects().iter().any(|effect| {
            effect
                .downcast_ref::<crate::effects::DealDamageEffect>()
                .is_some()
                || effect.downcast_ref::<TaggedEffect>().is_some_and(|tagged| {
                    tagged
                        .effect
                        .downcast_ref::<crate::effects::DealDamageEffect>()
                        .is_some()
                })
        }),
        "expected damage spell effect"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_zero_mana_alternative_cost_line_from_text() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Trap Variant")
        .parse_text("You may pay {0} rather than pay this spell's mana cost.\nDraw a card.")
        .expect("parse zero-mana alternative cost");

    assert_eq!(def.alternative_casts.len(), 1);
    let alt = &def.alternative_casts[0];
    match alt {
        AlternativeCastingMethod::Composed { total_cost, .. } => {
            let mana = total_cost.mana_cost().expect("expected mana alt cost");
            assert_eq!(mana.to_oracle(), "{0}");
        }
        other => panic!("expected Composed, got {other:?}"),
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_if_self_free_cast_alternative_cost_line_from_text() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Sivvi Probe")
            .parse_text(
                "If an opponent controls a Mountain and you control a Plains, you may cast this spell without paying its mana cost.\nDraw a card.",
            )
            .expect("conditional self free-cast alternative cost should parse");

    assert_eq!(def.alternative_casts.len(), 1);
    let alt = &def.alternative_casts[0];
    match alt {
        AlternativeCastingMethod::Composed {
            total_cost,
            condition,
            ..
        } => {
            let mana_cost = total_cost.mana_cost();
            let costs = alt.non_mana_costs();
            assert!(
                mana_cost.is_none(),
                "conditional self free-cast should not require mana"
            );
            assert!(
                costs.is_empty(),
                "conditional self free-cast should not add extra non-mana costs"
            );
            assert!(
                condition.is_some(),
                "expected parsed cast-time condition for conditional self free-cast"
            );
            let condition = condition.as_ref().expect("condition should exist");
            let crate::static_abilities::ThisSpellCostCondition::ConditionExpr {
                condition: condition_expr,
                ..
            } = condition
            else {
                panic!("expected condition expression for conditional self free-cast");
            };
            let crate::ConditionExpr::And(left, right) = condition_expr else {
                panic!("expected conjunction for mixed-controller cost condition");
            };
            let matches_clause = |expr: &crate::ConditionExpr,
                                  controller: crate::target::PlayerFilter,
                                  subtype: Subtype| {
                let crate::ConditionExpr::CountComparison {
                    count, comparison, ..
                } = expr
                else {
                    return false;
                };
                let crate::static_abilities::AnthemCountExpression::MatchingFilter(filter) = count
                else {
                    return false;
                };
                *comparison == crate::effect::Comparison::GreaterThanOrEqual(1)
                    && filter.controller == Some(controller)
                    && filter.subtypes == vec![subtype]
            };
            let left_is_opponent_mountain = matches_clause(
                left,
                crate::target::PlayerFilter::Opponent,
                Subtype::Mountain,
            );
            let left_is_you_plains =
                matches_clause(left, crate::target::PlayerFilter::You, Subtype::Plains);
            let right_is_opponent_mountain = matches_clause(
                right,
                crate::target::PlayerFilter::Opponent,
                Subtype::Mountain,
            );
            let right_is_you_plains =
                matches_clause(right, crate::target::PlayerFilter::You, Subtype::Plains);
            assert!(
                (left_is_opponent_mountain && right_is_you_plains)
                    || (left_is_you_plains && right_is_opponent_mountain),
                "expected conjunction of opponent-controls-Mountain and you-control-Plains, got {condition_expr:?}"
            );
        }
        other => panic!("expected Composed, got {other:?}"),
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_if_conditional_rather_than_alternative_cost_line_from_text() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Sivvi Valor Probe")
            .parse_text(
                "If you control a Plains, you may tap an untapped creature you control rather than pay this spell's mana cost.\nDraw a card.",
            )
            .expect("conditional rather-than alternative cost should parse");

    assert_eq!(def.alternative_casts.len(), 1);
    let alt = &def.alternative_casts[0];
    match alt {
        AlternativeCastingMethod::Composed { condition, .. } => {
            let costs = alt.non_mana_costs();
            assert!(
                !costs.is_empty(),
                "expected non-mana costs in conditional rather-than alternative cost"
            );
            assert!(
                condition.is_some(),
                "expected parsed cast-time condition for conditional rather-than alternative cost"
            );
        }
        other => panic!("expected Composed, got {other:?}"),
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_self_free_cast_alternative_cost_line_from_text() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Free Cast Probe")
        .parse_text("You may cast this spell without paying its mana cost.\nDraw a card.")
        .expect("self free-cast alternative cost should parse");

    assert_eq!(def.alternative_casts.len(), 1);
    let alt = &def.alternative_casts[0];
    match alt {
        AlternativeCastingMethod::Composed {
            total_cost,
            condition,
            ..
        } => {
            let mana_cost = total_cost.mana_cost();
            let costs = alt.non_mana_costs();
            assert!(
                mana_cost.is_none(),
                "self free-cast should not require mana"
            );
            assert!(
                costs.is_empty(),
                "self free-cast should not add extra non-mana costs"
            );
            assert!(
                condition.is_none(),
                "unconditional self free-cast should not add a condition"
            );
        }
        other => panic!("expected Composed, got {other:?}"),
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_alternative_cost_with_trailing_clause_fails() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Treasure Alt Cost Variant")
            .parse_text(
                "You may pay {R}{G} rather than pay this spell's mana cost. Spend only mana produced by Treasures to cast it this way.",
            )
            .expect_err("alternative cost line with trailing clause should fail");

    let message = format!("{err:?}");
    assert!(
        message.contains("unsupported trailing clause after alternative cost"),
        "expected strict trailing-clause error, got {message}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_unless_any_player_pays_mana_prefix() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Rhystic Tutor Variant")
            .parse_text(
                "Unless any player pays {2}, search your library for a card, put that card into your hand, then shuffle.",
            )
            .expect("parse unless-any-player-pays prefix");

    let effects = def.spell_effect.expect("spell effects");
    let debug = format!("{effects:?}");
    assert!(
        debug.contains("UnlessPaysEffect"),
        "expected unless-pays wrapper in compiled effects, got {debug}"
    );
    assert!(
        debug.contains("player: Any"),
        "expected any-player payment choice, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_yasharn_search_uses_generic_library_slots_bundle() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Yasharn Variant")
            .card_types(vec![CardType::Creature])
            .parse_text(
                "When this creature enters, search your library for a basic Forest card and a basic Plains card, reveal those cards, put them into your hand, then shuffle.",
            )
            .expect("parse Yasharn-style bundle");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("SearchLibrarySlotsEffect"),
        "expected slot-based search effect, got {debug}"
    );
    assert!(
        debug.contains("subtypes: [Forest]") && debug.contains("subtypes: [Plains]"),
        "expected Forest and Plains slot filters, got {debug}"
    );

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
            rendered.contains(
                "search your library for a basic Forest card and a basic Plains card, reveal those cards, put them into your hand, then shuffle"
            ),
            "expected oracle-like slot-search text, got {rendered}"
        );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_gem_of_becoming_search_tracks_each_land_slot() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Gem of Becoming")
            .card_types(vec![CardType::Artifact])
            .parse_text(
                "{3}, {T}, Sacrifice this artifact: Search your library for an Island card, a Swamp card, and a Mountain card. Reveal those cards, put them into your hand, then shuffle.",
            )
            .expect("parse Gem of Becoming search");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("SearchLibrarySlotsEffect"),
        "expected slot-based search effect, got {debug}"
    );
    assert!(
        debug.contains("subtypes: [Island]")
            && debug.contains("subtypes: [Swamp]")
            && debug.contains("subtypes: [Mountain]"),
        "expected Island, Swamp, and Mountain slot filters, got {debug}"
    );

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
            rendered.contains(
                "Search your library for an Island card, a Swamp card, and a Mountain card. Reveal those cards, put them into your hand, then shuffle"
            ),
            "expected oracle-like Gem text, got {rendered}"
        );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_construct_token_with_explicit_pt_does_not_force_karnstruct_stats() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Sokenzan Smelter Variant")
            .parse_text(
                "At the beginning of combat on your turn, you may pay {1} and sacrifice an artifact. If you do, create a 3/1 red Construct artifact creature token with haste.",
            )
            .expect("parse explicit-pt construct token");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("create a 3/1 red construct artifact creature token with haste"),
        "expected explicit 3/1 haste construct token text, got {rendered}"
    );
    assert!(
        !rendered
            .contains("power and toughness are each equal to the number of artifacts you control"),
        "explicit 3/1 construct token should not be forced into karnstruct stats, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_exile_up_to_one_single_disjunction_stays_single_choice() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Scrollshift Variant")
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

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_for_each_player_who_didnt_tracks_did_not_result() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Strongarm Tactics Variant")
            .parse_text(
                "Each player discards a card. Then each player who didn't discard a creature card this way loses 4 life.",
            )
            .expect("parse each-player-who-didnt branch");

    let effects = def.spell_effect.expect("spell effects");
    let debug = format!("{effects:?}");
    assert!(
        debug.contains("PlayerTaggedObjectMatches"),
        "expected discarded-card predicate branch, got {debug}"
    );
    assert!(
        debug.contains("LoseLifeEffect"),
        "expected lose-life consequence branch, got {debug}"
    );
    assert!(
        debug.contains("card_types: [Creature]"),
        "expected discarded-card qualifier to remain creature-specific, got {debug}"
    );
    assert!(
        !debug.contains("DidNotHappen"),
        "did-not branch should not collapse into a generic result predicate, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_exile_target_player_hand_and_graveyard_bundle_sets_owner() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Identity Crisis Variant")
        .parse_text("Exile all cards from target player's hand and graveyard.")
        .expect("parse target hand+graveyard exile");

    let effects = def.spell_effect.expect("spell effects");
    let debug = format!("{effects:?}");
    assert!(
        debug.contains("zone: Some(Hand)") && debug.contains("zone: Some(Graveyard)"),
        "expected both hand and graveyard exile filters, got {debug}"
    );
    assert!(
        debug.matches("owner: Some(Target(Any))").count() >= 2,
        "expected both exile filters to track target player ownership, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_self_enters_with_counters_as_static_not_spell_effect() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Self ETB Counter Variant")
        .parse_text("This creature enters with four +1/+1 counters on it.")
        .expect("parse self enters with counters");

    assert!(
        def.spell_effect.is_none(),
        "self ETB counters should not compile as spell effect"
    );

    let has_etb_replacement = def.abilities.iter().any(|ability| {
            matches!(
                &ability.kind,
                AbilityKind::Static(static_ability)
                    if static_ability.id() == crate::static_abilities::StaticAbilityId::EnterWithCounters
            )
        });
    assert!(
        has_etb_replacement,
        "expected self ETB replacement static ability, got {:?}",
        def.abilities
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_this_artifact_enters_with_counters_and_source_remove_cost() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Ox Cart Variant")
            .card_types(vec![CardType::Artifact])
            .parse_text(
                "This artifact enters with three charge counters on it.\n{1}, {T}, Remove a charge counter from this artifact: Destroy target creature.",
            )
            .expect("parse artifact enters counters plus source remove cost");

    assert!(
        def.spell_effect.is_none(),
        "artifact ETB counters should not compile as spell effect"
    );

    let has_etb_replacement = def.abilities.iter().any(|ability| {
            matches!(
                &ability.kind,
                AbilityKind::Static(static_ability)
                    if static_ability.id() == crate::static_abilities::StaticAbilityId::EnterWithCounters
            )
        });
    assert!(
        has_etb_replacement,
        "expected ETB counters static ability, got {:?}",
        def.abilities
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
            && cost_debug.contains("RemoveCountersEffect")
            && cost_debug.contains("counter_type: Charge")
            && cost_debug.contains("target: Source"),
        "expected source-specific remove-counters effect-backed cost, got {cost_debug}"
    );
    assert!(
        !cost_debug.contains("RemoveAnyCountersAmongEffect"),
        "expected no distributed remove-counter fallback, got {cost_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_return_two_target_cards_uses_exact_target_count() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Soul Strings Count Variant")
        .parse_text("Return two target creature cards from your graveyard to your hand.")
        .expect("parse exact-count return target");

    let effects = def.spell_effect.expect("spell effects");
    let debug = format!("{effects:?}");
    assert!(
        debug.contains("ChoiceCount { min: 2, max: Some(2)") && debug.contains("dynamic_x: false"),
        "expected exact two-target choice count, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn reject_target_player_dealt_damage_by_this_turn_subject() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Wicked Akuba Subject Variant")
        .parse_text("{B}: Target player dealt damage by this creature this turn loses 1 life.")
        .expect_err(
            "combat-history player subject should fail until per-source turn history is modeled",
        );

    let message = format!("{err:?}");
    assert!(
        message.contains("unsupported combat-history player subject"),
        "expected strict combat-history subject error, got {message}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_backdraft_tracks_historical_spell_damage_choice() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Backdraft")
            .card_types(vec![CardType::Sorcery])
            .parse_text(
                "Choose a player who cast one or more sorcery spells this turn. Backdraft deals damage to that player equal to half the damage dealt by one of those sorcery spells this turn, rounded down.",
            )
            .expect("Backdraft should parse");

    let effects = def.spell_effect.expect("spell effects");
    let debug = format!("{effects:?}");
    assert!(
        debug.contains("ChoosePlayerEffect"),
        "expected qualifying-player choice, got {debug}"
    );
    assert!(
        debug.contains("ChooseSpellCastHistoryEffect"),
        "expected historical spell-choice effect, got {debug}"
    );
    assert!(
        debug.contains("DamageDealtThisTurnByTaggedSpellCast"),
        "expected spell-damage history value, got {debug}"
    );
    assert!(
        debug.contains("HalfRoundedDown"),
        "expected rounded-down half-damage value, got {debug}"
    );

    let damage = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::DealDamageEffect>())
        .expect("expected lowered damage effect");
    assert!(
        matches!(
            damage.target,
            crate::target::ChooseSpec::Player(crate::target::PlayerFilter::TaggedPlayer(_))
        ),
        "expected Backdraft damage target to remain the chosen player, got {:?}",
        damage.target
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_static_condition_equipped_creature_tapped_or_untapped() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Sword Condition Variant")
            .parse_text(
                "As long as equipped creature is tapped, tapped creatures you control get +2/+0.\nAs long as equipped creature is untapped, untapped creatures you control get +0/+2.",
            )
            .expect("parse equipped-creature tapped/untapped static conditions");

    let displays = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.display()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(
        displays
            .iter()
            .any(|display| display.contains("as long as equipped creature is tapped")),
        "missing tapped equipped-creature condition in displays: {displays:?}"
    );
    assert!(
        displays
            .iter()
            .any(|display| display.contains("as long as equipped creature is untapped")),
        "missing untapped equipped-creature condition in displays: {displays:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn sharpened_pitchfork_parses_with_human_equipped_condition() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Sharpened Pitchfork")
            .parse_text(
                "Equipped creature has first strike.\nAs long as equipped creature is a Human, it gets +1/+1.\nEquip {1}",
            )
            .expect("Sharpened Pitchfork should parse");

    let displays = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.display()),
            AbilityKind::Activated(_) => crate::ability::ability_surface_text_for_tests(ability),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(
        displays.iter().any(|display| display
            .to_ascii_lowercase()
            .contains("equipped creature has first strike")),
        "missing first-strike static grant in displays: {displays:?}"
    );
    assert!(
        displays.iter().any(|display| {
            display
                .to_ascii_lowercase()
                .contains("as long as equipped creature is a human")
        }),
        "missing human equipped-creature condition in displays: {displays:?}"
    );
    assert!(
        displays.iter().any(|display| display == "Equip {1}"),
        "missing equip activation in displays: {displays:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_static_condition_its_attacking() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Kitesail Corsair Variant")
        .parse_text("This creature has flying as long as it's attacking.")
        .expect("parse source-attacking static condition");

    let displays = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.display()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(
        displays
            .iter()
            .any(|display| display.contains("as long as this creature is attacking")),
        "missing source-attacking condition in displays: {displays:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_put_that_card_into_hand_with_prior_reference() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Put Referenced Card Into Hand Variant")
        .parse_text("Reveal the top card of your library. Put that card into your hand.")
        .expect("put that card into hand should parse");

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("MoveToZoneEffect")
            && debug.contains("zone: Hand")
            && debug.contains("Tagged"),
        "expected move-to-hand tagged effect, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_put_that_card_into_graveyard_with_prior_reference() {
    let def =
        CardDefinitionBuilder::new(CardId::new(), "Put Referenced Card Into Graveyard Variant")
            .parse_text("Reveal the top card of your library. Put that card into your graveyard.")
            .expect("put that card into graveyard should parse");

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("MoveToZoneEffect")
            && debug.contains("zone: Graveyard")
            && debug.contains("Tagged"),
        "expected move-to-graveyard tagged effect, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_put_land_from_hand_onto_battlefield_tapped() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Put Land Tapped Variant")
        .parse_text("Put a land card from your hand onto the battlefield tapped.")
        .expect("put land card from hand onto battlefield tapped should parse");

    let spell_debug = format!("{:?}", def.spell_effect);
    assert!(
        spell_debug.contains("MoveToZoneEffect")
            && spell_debug.contains("zone: Battlefield")
            && spell_debug.contains("enters_tapped: true"),
        "expected tapped battlefield move effect, got {spell_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_conditional_counter_target_spell_if_it_matches_filter() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Jaded Response Variant")
        .parse_text("Counter target spell if it shares a color with a creature you control.")
        .expect("target-filter conditional should parse without prior tagged reference");

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("ConditionalEffect")
            && (debug.contains("TargetMatches")
                || debug.contains("TaggedObjectMatches")
                || (debug.contains("PlayerControls") && debug.contains("SharesColorWithTagged"))),
        "expected conditional target-match lowering, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_conditional_instead_branch_referencing_target() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Electrostatic Bolt Variant")
            .parse_text(
                "Electrostatic Bolt deals 2 damage to target creature. If it's an artifact creature, Electrostatic Bolt deals 4 damage to it instead.",
            )
            .expect("artifact-creature conditional should parse without explicit prior tag");

    let program = def.spell_effect.as_ref().expect("spell effect");
    let debug = format!("{:?}", program);
    assert!(
        program.segments.len() == 1
            && program.segments[0].self_replacements.len() == 1
            && (debug.contains("TargetMatches") || debug.contains("TaggedObjectMatches")),
        "expected artifact-creature conditional lowering, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_conditional_kicker_target_spell_mana_value() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Prohibit Variant")
            .parse_text(
                "Kicker {2} (You may pay an additional {2} as you cast this spell.)\nCounter target spell if its mana value is 2 or less. If this spell was kicked, counter that spell if its mana value is 4 or less instead.",
            )
            .expect("kicker conditional counter spell should parse");

    let program = def.spell_effect.as_ref().expect("spell effect");
    let debug = format!("{:?}", program);
    assert!(
        program.segments.len() == 1
            && program.segments[0].self_replacements.len() == 1
            && debug.contains("TaggedObjectMatches")
            && debug.contains("LessThanOrEqual(2)")
            && debug.contains("LessThanOrEqual(4)"),
        "expected kicker conditional counter-spell lowering, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_conditional_instead_branch_for_legendary_or_enchantment_creature() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Regents Authority Variant")
            .parse_text(
                "Target creature gets +2/+2 until end of turn. If it's an enchantment creature or legendary creature, instead put a +1/+1 counter on it and it gets +1/+1 until end of turn.",
            )
            .expect("enchantment-or-legendary conditional should parse");

    let program = def.spell_effect.as_ref().expect("spell effect");
    let debug = format!("{:?}", program);
    assert!(
        program.segments.len() == 1
            && program.segments[0].self_replacements.len() == 1
            && (debug.contains("TargetMatches") || debug.contains("TaggedObjectMatches"))
            && debug.contains("PutCountersEffect"),
        "expected conditional counter-and-pump lowering, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_conditional_instead_branch_for_human_target() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Flare of Faith Variant")
            .parse_text(
                "Target creature gets +2/+2 until end of turn. If it's a Human, instead it gets +3/+3 and gains indestructible until end of turn.",
            )
            .expect("human conditional should parse");

    let program = def.spell_effect.as_ref().expect("spell effect");
    let debug = format!("{:?}", program);
    assert!(
        program.segments.len() == 1
            && program.segments[0].self_replacements.len() == 1
            && (debug.contains("TargetMatches") || debug.contains("TaggedObjectMatches"))
            && debug.contains("Indestructible"),
        "expected conditional human branch with indestructible, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_conditional_instead_branch_with_trailing_gets_instead() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Groundswell Variant")
            .parse_text(
                "Target creature gets +2/+2 until end of turn. If it's a Human, that creature gets +3/+3 until end of turn instead.",
            )
            .expect("trailing gets-instead conditional should parse");

    let program = def.spell_effect.as_ref().expect("spell effect");
    let debug = format!("{:?}", program);
    assert!(
        program.segments.len() == 1
            && program.segments[0].self_replacements.len() == 1
            && (debug.contains("TargetMatches") || debug.contains("TaggedObjectMatches"))
            && debug.contains("power: Fixed(3)")
            && debug.contains("toughness: Fixed(3)"),
        "expected +3/+3 conditional replacement branch, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_conditional_landfall_history_predicate_instead_branch() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Groundswell Landfall Variant")
            .parse_text(
                "Target creature gets +2/+2 until end of turn. If you had a land enter the battlefield under your control this turn, that creature gets +4/+4 until end of turn instead.",
            )
            .expect("landfall-history conditional should parse");

    let program = def.spell_effect.as_ref().expect("spell effect");
    let debug = format!("{:?}", program);
    assert!(
        program.segments.len() == 1
            && program.segments[0].self_replacements.len() == 1
            && debug.contains("PlayerHadLandEnterBattlefieldThisTurn")
            && debug.contains("power: Fixed(4)")
            && debug.contains("toughness: Fixed(4)"),
        "expected landfall-history conditional replacement branch, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_destination_first_put_onto_battlefield_under_your_control() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Thrilling Encore Variant")
            .parse_text(
                "Put onto the battlefield under your control all creature cards in all graveyards that were put there from the battlefield this turn.",
            )
            .expect("destination-first put clause should parse");

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("ReturnAllToBattlefieldEffect")
            && debug.contains("card_types: [Creature]")
            && debug.contains("entered_graveyard_from_battlefield_this_turn: true"),
        "expected creature graveyard-history return-all lowering, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_destination_first_put_attached_to_it_from_graveyard_or_hand() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Bruna Variant")
            .parse_text(
                "Flying, vigilance\nWhenever this creature attacks or blocks, you may attach to it any number of Auras on the battlefield and you may put onto the battlefield attached to it any number of Aura cards that could enchant it from your graveyard and/or hand.",
            )
            .expect("destination-first put-attached clause should parse");

    let debug = format!("{:?}", def);
    assert!(
        debug.contains("MoveToZoneEffect")
            && debug.contains("AttachObjectsEffect")
            && debug.contains("TagKey(\"triggering\")")
            && debug.contains("TagKey(\"moved_")
            && debug.contains("zone: Some(Graveyard)")
            && debug.contains("zone: Some(Hand)"),
        "expected move+attach lowering with triggering target and hand/graveyard disjunction, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_enchanted_creature_has_keyword_static_line() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Lance Variant")
        .parse_text("Enchant creature\nEnchanted creature has first strike.")
        .expect("enchanted-creature keyword grant should parse");

    let debug = format!("{:?}", def);
    assert!(
        debug.contains("AttachedAbilityGrant")
            && debug.contains("enchanted creature has first strike"),
        "expected attached keyword grant lowering, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_you_control_enchanted_land_static_line() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Annex Variant")
        .parse_text("Enchant land\nYou control enchanted land.")
        .expect("control enchanted land static line should parse");

    let debug = format!("{:?}", def);
    assert!(
        debug.contains("ControlAttachedPermanent"),
        "expected control-attached static lowering, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_can_block_additional_creature_this_turn_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Anurid Variant")
        .parse_text("Reach\n{1}{G}: This creature can block an additional creature this turn.")
        .expect("temporary can-block-additional clause should parse");

    let debug = format!("{:?}", def);
    assert!(
        debug.contains("CanBlockAdditionalCreatureEachCombat")
            && debug.contains("until: EndOfTurn"),
        "expected end-of-turn can-block-additional grant, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_land_type_addition_static_line() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Blanket Variant")
        .parse_text("Each land is a Swamp in addition to its other land types.")
        .expect("land type addition static line should parse");

    let debug = format!("{:?}", def);
    assert!(
        debug.contains("AddSubtypes") && debug.contains("Swamp"),
        "expected subtype-add static lowering for swamp addition, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_nightcreep_style_land_type_change_uses_basic_land_type_lowering() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Nightcreep Variant")
        .card_types(vec![CardType::Instant])
        .parse_text("Until end of turn, all creatures become black and all lands become Swamps.")
        .expect("Nightcreep-style text should parse");

    let debug = format!("{:?}", def.spell_effect);
    let rendered = crate::compiled_text::canonical_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        debug.contains("SetColors")
            && debug.contains("BecomeBasicLandTypeChoiceEffect")
            && debug.contains("fixed_subtype: Some(Swamp)")
            && debug.contains("until: EndOfTurn")
            && !debug.contains("AddSubtypes"),
        "expected Nightcreep-style spell lowering to set colors and use fixed basic land type, got {debug}"
    );
    assert!(
        rendered.contains("creature")
            && rendered.contains("becomes black")
            && rendered.contains("all lands become swamps")
            && rendered.contains("until end of turn"),
        "expected Nightcreep-style render to preserve color and land-type changes, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_lands_are_pt_creatures_still_lands_static_line() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Living Plane Variant")
        .parse_text("All lands are 1/1 creatures that are still lands.")
        .expect("lands become creatures static line should parse");

    let static_ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        static_ids.contains(&crate::static_abilities::StaticAbilityId::AddCardTypes),
        "expected AddCardTypes static ability for lands becoming creatures, got {static_ids:?}"
    );
    assert!(
        static_ids
            .contains(&crate::static_abilities::StaticAbilityId::SetBasePowerToughnessForFilter),
        "expected SetBasePowerToughnessForFilter static ability for lands becoming 1/1, got {static_ids:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_lands_become_pt_creatures_until_end_of_turn_spell_line() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Life Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "All lands you control become 1/1 creatures until end of turn. They're still lands.",
        )
        .expect("lands animation spell line should parse");

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("AddCardTypes")
            && debug.contains("SetPowerToughness")
            && debug.contains("EndOfTurn"),
        "expected animated-creature continuous effect lowering, got {debug}"
    );

    let effects = def.spell_effect.as_ref().expect("expected spell effects");
    let score_path =
        crate::compiled_text::compile_effect_list(&effects.segments[0].default_effects);
    assert!(
        score_path
            == "All lands you control become 1/1 creatures until end of turn. They're still lands"
            || score_path
                == "All lands you control become creatures with base power and toughness 1/1 until end of turn. They're still lands",
        "unexpected lands-animation surface: {score_path}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_target_artifact_becomes_artifact_creature_until_end_of_turn_spell_line() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Capenna Express Variant")
        .card_types(vec![CardType::Instant])
        .parse_text("Target artifact becomes an artifact creature until end of turn.")
        .expect("artifact animation clause should parse");

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        (debug.contains("AddCardTypes") || debug.contains("SetCardTypes"))
            && debug.contains("Artifact")
            && debug.contains("Creature")
            && debug.contains("EndOfTurn"),
        "expected artifact-creature animation lowering, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_target_creature_becomes_vampire_in_addition_to_other_types_until_eot_spell_line() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Bloodline Variant")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Target creature becomes a Vampire in addition to its other types until end of turn.",
        )
        .expect("subtype-addition animation clause should parse");

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("AddSubtypes") && debug.contains("Vampire") && debug.contains("EndOfTurn"),
        "expected subtype-addition animation lowering, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_target_land_becomes_island_until_end_of_turn_spell_line() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Twiddle Land Variant")
        .card_types(vec![CardType::Instant])
        .parse_text("Target land becomes an Island until end of turn.")
        .expect("land subtype animation clause should parse");

    let debug = format!("{:?}", def.spell_effect).to_ascii_lowercase();
    assert!(
        debug.contains("becomebasiclandtypechoiceeffect")
            && debug.contains("fixed_subtype: some")
            && debug.contains("island"),
        "expected fixed basic-land-type lowering, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_you_choose_nonland_card_from_revealed_hand_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Venarian Variant")
            .card_types(vec![CardType::Sorcery])
            .parse_text("Target player reveals their hand. You choose a nonland card with mana value X or less from it. That player discards that card.")
            .expect("you-choose-from-revealed-hand clause should parse");

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("ChooseObjectsEffect")
            && debug.contains("mana_value")
            && debug.contains("DiscardEffect"),
        "expected choose-from-hand and discard lowering, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_choose_card_type_then_reveal_and_put_matching_cards() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Alrund Variant")
            .card_types(vec![CardType::Creature])
            .parse_text("At the beginning of your end step, choose a card type, then reveal the top three cards of your library. Put all cards of the chosen type revealed this way into your hand and the rest on the bottom of your library in any order.")
            .expect("choose-card-type reveal/put sequence should parse");

    let debug = format!("{:?}", def);
    assert!(
        debug.contains("ChooseModeEffect")
            && debug.contains("LookAtTopCardsEffect")
            && debug.contains("RevealTaggedEffect"),
        "expected choose-mode reveal/put lowering for chosen card type, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_activated_ability_cost_reduction_static_line() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Training Grounds Variant")
            .card_types(vec![CardType::Enchantment])
            .parse_text(
                "Activated abilities of creatures you control cost {2} less to activate.\nThis effect can't reduce the mana in that cost to less than one mana.",
            )
            .expect("activated-ability cost reduction static line should parse");

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
            .contains(&crate::static_abilities::StaticAbilityId::ActivatedAbilityCostReduction),
        "expected activated-ability cost reduction static ability, got {static_ids:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_conditional_cycling_zero_cost_static_line() {
    let def = CardDefinitionBuilder::new(CardId::new(), "New Perspectives Variant")
            .card_types(vec![CardType::Enchantment])
            .parse_text(
                "As long as you have seven or more cards in hand, you may pay {0} rather than pay cycling costs.",
            )
            .expect("conditional cycling alternative cost should parse");

    let debug = format!("{:?}", def);
    assert!(
        debug.contains("ActivatedAbilityCostReduction")
            && debug.contains("replacement_mana_cost: Some")
            && debug.contains("cycling")
            && debug.contains("PlayerCardsInHandOrMore"),
        "expected conditional cycling zero-cost modifier, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_activated_ability_cost_increase_static_line() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Brutal Suppression Variant")
            .card_types(vec![CardType::Enchantment])
            .parse_text(
                "Activated abilities of nontoken Rebels cost an additional \"Sacrifice a land\" to activate.",
            )
            .expect("activated-ability cost increase static line should parse");

    assert!(
        def.spell_effect.is_none(),
        "expected static activated-ability tax to stay out of spell effects"
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
        static_ids
            .contains(&crate::static_abilities::StaticAbilityId::ActivatedAbilityCostIncrease),
        "expected activated-ability cost increase static ability, got {static_ids:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_spell_and_activation_tax_static_line() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Tithe Taker Variant")
            .card_types(vec![CardType::Creature])
            .parse_text(
                "During your turn, spells your opponents cast cost {1} more to cast and abilities your opponents activate cost {1} more to activate unless they're mana abilities.",
            )
            .expect("combined spell and activated-ability tax line should parse");

    let static_ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        static_ids.contains(&crate::static_abilities::StaticAbilityId::CostIncrease)
            && static_ids
                .contains(&crate::static_abilities::StaticAbilityId::ActivatedAbilityCostIncrease),
        "expected both spell and activation tax static abilities, got {static_ids:?}"
    );

    let rendered = crate::compiled_text::unprocessed_compiled_lines(&def).join(" ");
    assert!(
            rendered.contains(
                "During your turn, spells your opponents cast cost {1} more to cast and abilities your opponents activate cost {1} more to activate unless they're mana abilities"
            ),
            "expected combined conditioned tax clauses, got {rendered}"
        );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_self_activated_ability_cost_reduction_for_each_static_line() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Channel Reducer Variant")
            .parse_text(
                "{1}{G}, Discard this card: Destroy target artifact, enchantment, or nonbasic land an opponent controls.\nThis ability costs {1} less to activate for each legendary creature you control.",
            )
            .expect("self activated-ability cost reduction line should parse");

    let debug = format!("{:?}", def);
    assert!(
        debug.contains("ActivatedAbilityCostReduction")
            && debug.contains("per_matching_objects: Some")
            && debug.contains("functional_zones: [Hand]"),
        "expected self cost reduction with per-match filter and nonbattlefield zones, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_self_activated_ability_cost_reduction_for_basic_land_types() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Domain Codex Variant")
            .card_types(vec![CardType::Artifact])
            .parse_text(
                "Domain — {5}, {T}: Draw a card. This ability costs {1} less to activate for each basic land type among lands you control.",
            )
            .expect("domain activated-ability cost reduction should parse");

    let debug = format!("{:?}", def);
    assert!(
        debug.contains("ActivatedAbilityCostReduction")
            && debug.contains("per_basic_land_types_among: Some")
            && !debug.contains("per_matching_objects: Some"),
        "expected domain cost reduction to count distinct basic land types, got {debug}"
    );
    let rendered = crate::compiled_text::unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("for each basic land type among lands you control")
            && !rendered.contains("for each basic land you control"),
        "expected domain cost reduction rendering, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_enchanted_creature_gets_xx_where_x_creature_cards_in_graveyard() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Wreath Variant")
            .card_types(vec![CardType::Enchantment])
            .parse_text(
                "Enchant creature\nEnchanted creature gets +X/+X, where X is the number of creature cards in your graveyard.",
            )
            .expect("where-X enchanted-creature anthem should parse");

    let static_ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        static_ids.contains(&crate::static_abilities::StaticAbilityId::Anthem),
        "expected anthem static ability for +X/+X where-X clause, got {static_ids:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_multiple_additional_land_plays_static_line() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Azusa Variant")
        .parse_text("You may play two additional lands on each of your turns.")
        .expect("multiple additional-land-play static line should parse");

    assert!(
        def.abilities.len() == 1,
        "expected a single shared restriction-based static ability, got {:#?}",
        def.abilities
    );
    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("RuleRestriction") && debug.contains("AdditionalLandPlays(2)"),
        "expected shared additional-land-play restriction, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_counter_unless_pays_dynamic_mana_equal_value() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Repulsive Mutation Variant")
            .parse_text(
                "Put X +1/+1 counters on target creature you control. Then counter up to one target spell unless its controller pays mana equal to the greatest power among creatures you control.",
            )
            .expect("counter-unless with dynamic mana-equal payment should parse");

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("UnlessPaysEffect")
            && debug.contains("additional_generic: Some")
            && debug.contains("GreatestPower"),
        "expected dynamic greatest-power payment in counter-unless lowering, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_until_next_turn_whenever_trigger_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Dont Move Variant")
            .parse_text(
                "Destroy all tapped creatures. Until your next turn, whenever a creature becomes tapped, destroy it.",
            )
            .expect("until-next-turn triggered clause should parse");

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("DestroyEffect { spec: All(")
            && debug.contains("PermanentBecomesTappedTrigger")
            && debug.contains("ScheduleDelayedTriggerEffect")
            && debug.contains("UntilControllerNextTurn"),
        "expected destroy-all plus delayed tap trigger granting, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn reject_counter_ability_target_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Tales End Variant")
        .parse_text("Counter target activated ability, triggered ability, or legendary spell.")
        .expect("countering activated/triggered abilities and legendary spells should parse");

    let message = format!("{:?}", def.spell_effect);
    assert!(
        message.contains("CounterEffect")
            && message.contains("stack_kind: Some(ActivatedAbility)")
            && message.contains("stack_kind: Some(TriggeredAbility)")
            && message.contains("stack_kind: Some(Spell)")
            && message.contains("supertypes: [Legendary]"),
        "expected parsed counter target union for ability/spell variants, got {message}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_target_creature_cant_block_this_creature_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Duct Crawler Variant")
        .parse_text("{R}: Target creature can't block this creature this turn.")
        .expect("target creature can't block this creature should parse");

    let lines = unprocessed_compiled_lines(&def);
    let activated = lines.join(" ");
    assert!(
        activated.contains("can't block")
            && (activated.contains("this permanent this turn")
                || activated.contains("this creature this turn")),
        "expected cant-block-this-creature text in compiled line, got {activated}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_target_creature_blocks_this_creature_if_able_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Rampant Elephant Variant")
        .parse_text("{G}: Target creature blocks this creature this turn if able.")
        .expect("target creature blocks this creature should parse");

    let lines = unprocessed_compiled_lines(&def);
    let activated = lines.join(" ");
    assert!(
        activated.contains("Target creature blocks this creature this turn if able"),
        "expected targeted blocks-this-creature text in compiled line, got {activated}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_all_creatures_able_to_block_target_creature_do_so_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Alluring Scent Variant")
        .parse_text("All creatures able to block target creature this turn do so.")
        .expect("all creatures able to block target creature clause should parse");

    let lines = unprocessed_compiled_lines(&def);
    let spell = lines.join(" ");
    assert!(
        spell.contains("All creatures able to block target creature this turn do so"),
        "expected all-creatures-do-so spell text, got {spell}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_curly_apostrophe_negated_untap_clause_with_tapped_duration() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Kill Switch Apostrophe Variant")
            .parse_text(
                "{2}, {T}: Tap all other artifacts. They don’t untap during their controllers’ untap steps for as long as this artifact remains tapped.",
            )
            .expect("negated untap clause with tapped duration should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("don't untap during their controllers' untap steps")
            || rendered.contains("cant untap during their controllers' untap steps")
            || rendered.contains("doesn't untap during its controller's untap step")
            || rendered.contains("doesnt untap during its controller's untap step"),
        "expected untap-lock clause in compiled text, got {rendered}"
    );
    assert!(
        rendered.contains("while this source is tapped")
            || rendered.contains("while this permanent is tapped")
            || rendered.contains("for as long as this source remains tapped")
            || rendered.contains("for as long as this permanent remains tapped"),
        "expected tapped-duration clause in compiled text, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn create_creature_token_with_food_reminder_stays_creature_token() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Wolf Quarry Token Variant")
            .parse_text(
                "Create three 1/1 green Boar creature tokens with \"When this token dies, create a Food token.\"",
            )
            .expect("parse boar token creation with food reminder");

    let effects = def.spell_effect.expect("spell effects");
    let debug = format!("{effects:?}");
    assert!(
        debug.contains("name: \"Boar\"")
            && debug.contains("subtypes: [Boar]")
            && debug.contains("this_object: true")
            && debug.contains("name: \"Food\"")
            && debug.contains("subtypes: [Food]"),
        "expected outer created token to remain a Boar while keeping the Food trigger, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_for_each_player_put_from_graveyard_keeps_choice_non_targeted() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Exhume Variant")
        .parse_text("Each player puts a creature card from their graveyard onto the battlefield.")
        .expect("for-each player put-from-graveyard should parse");

    let joined = crate::compiled_text::unprocessed_compiled_lines(&def).join(" ");
    assert!(
        !joined.contains("target creature card in that player's graveyard"),
        "for-each choice should not become a target selection: {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_for_each_player_may_put_from_hand_keeps_choice_non_targeted() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Show and Tell Variant")
            .parse_text(
                "Each player may put an artifact, creature, enchantment, or land card from their hand onto the battlefield.",
            )
            .expect("for-each player may-put-from-hand should parse");

    let joined = crate::compiled_text::unprocessed_compiled_lines(&def).join(" ");
    assert!(
        !joined.contains("target artifact or creature or enchantment or land card"),
        "for-each choice should not force target wording: {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_unstable_experiment_draw_then_connive_preserves_draw() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Unstable Experiment Variant")
            .parse_text(
                "Target player draws a card, then up to one target creature you control connives. (Draw a card, then discard a card. If you discarded a nonland card, put a +1/+1 counter on that creature.)",
            )
            .expect("draw-then-connive sentence should parse");

    let effects = def.spell_effect.as_ref().expect("spell effects");
    let debug = format!("{effects:?}");
    assert!(
        debug.contains("DrawCardsEffect"),
        "expected DrawCardsEffect, got {debug}"
    );
    assert!(
        debug.contains("ConniveEffect"),
        "expected ConniveEffect, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_grim_captains_call_then_do_same_for_subtypes_expands_each_return() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Grim Captain's Call Variant")
            .parse_text(
                "Return a Pirate card from your graveyard to your hand, then do the same for Vampire, Dinosaur, and Merfolk.",
            )
            .expect("then-do-the-same-for subtype sentence should parse");

    let lines = unprocessed_compiled_lines(&def);
    let spell_line = lines.join(" ");
    assert!(
        spell_line.contains("Pirate")
            && spell_line.contains("Vampire")
            && spell_line.contains("Dinosaur")
            && spell_line.contains("Merfolk"),
        "expected all subtype returns in compiled output, got {spell_line}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_each_player_return_with_additional_counter_appends_counter_effect() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Pyrrhic Revival Variant")
            .parse_text(
                "Each player returns each creature card from their graveyard to the battlefield with an additional -1/-1 counter on it.",
            )
            .expect("for-each return-with-additional-counter sentence should parse");

    let effects = def.spell_effect.as_ref().expect("spell effects");
    let for_players = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<ForPlayersEffect>())
        .expect("expected ForPlayersEffect");
    let debug = format!("{for_players:?}");
    let rendered = crate::compiled_text::unprocessed_compiled_lines(&def).join(" ");
    assert!(
        debug.contains("TaggedEffect")
            && debug.contains("ReturnAllToBattlefieldEffect")
            && debug.contains("PutCountersEffect")
            && debug.contains("MinusOneMinusOne"),
        "expected tagged return + -1/-1 counter effects in for-players branch, got {debug}"
    );
    assert!(
            rendered.contains("Each player returns each creature card from their graveyard to the battlefield with an additional -1/-1 counter on it")
                || rendered.contains("Each player returns each creature card from their graveyard to the battlefield. Put a -1/-1 counter on it"),
            "expected rendered Pyrrhic Revival text to preserve the return-with-counter compaction, got {rendered}"
        );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn replay_pyrrhic_revival_returns_each_players_creatures_with_counters() {
    use crate::card::{CardBuilder, PowerToughness};
    use crate::decision::SelectFirstDecisionMaker;
    use crate::game_loop::resolve_stack_entry_with;
    use crate::game_state::{GameState, StackEntry};
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::types::CardType;
    use crate::zone::Zone;

    fn create_graveyard_creature(game: &mut GameState, name: &str, owner: PlayerId) -> ObjectId {
        game.create_object_from_card(
            &CardBuilder::new(CardId::new(), name)
                .mana_cost(ManaCost::from_pips(vec![
                    vec![ManaSymbol::Generic(1)],
                    vec![ManaSymbol::Green],
                ]))
                .card_types(vec![CardType::Creature])
                .power_toughness(PowerToughness::fixed(2, 2))
                .build(),
            owner,
            Zone::Graveyard,
        )
    }

    fn create_graveyard_artifact(game: &mut GameState, name: &str, owner: PlayerId) -> ObjectId {
        game.create_object_from_card(
            &CardBuilder::new(CardId::new(), name)
                .card_types(vec![CardType::Artifact])
                .build(),
            owner,
            Zone::Graveyard,
        )
    }

    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    create_graveyard_creature(&mut game, "Alice Revival Bear", alice);
    create_graveyard_artifact(&mut game, "Alice Spare Relic", alice);
    create_graveyard_creature(&mut game, "Bob Revival Bear", bob);
    create_graveyard_artifact(&mut game, "Bob Spare Relic", bob);

    let spell = CardDefinitionBuilder::new(CardId::new(), "Pyrrhic Revival Variant")
            .parse_text(
                "Each player returns each creature card from their graveyard to the battlefield with an additional -1/-1 counter on it.",
            )
            .expect("Pyrrhic Revival text should parse");
    let spell_id = game.create_object_from_definition(&spell, alice, Zone::Stack);
    game.stack.push(StackEntry::new(spell_id, alice));

    resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker)
        .expect("Pyrrhic Revival should resolve");

    let alice_returned = game
        .battlefield
        .iter()
        .copied()
        .find(|&id| {
            game.object(id)
                .map(|obj| obj.name == "Alice Revival Bear" && game.controller_of(obj) == alice)
                .unwrap_or(false)
        })
        .expect("Alice's creature should be on the battlefield");
    let bob_returned = game
        .battlefield
        .iter()
        .copied()
        .find(|&id| {
            game.object(id)
                .map(|obj| obj.name == "Bob Revival Bear" && game.controller_of(obj) == bob)
                .unwrap_or(false)
        })
        .expect("Bob's creature should be on the battlefield");

    assert_eq!(
        game.counter_count(alice_returned, crate::object::CounterType::MinusOneMinusOne),
        1,
        "Alice's returned creature should get exactly one -1/-1 counter"
    );
    assert_eq!(
        game.counter_count(bob_returned, crate::object::CounterType::MinusOneMinusOne),
        1,
        "Bob's returned creature should get exactly one -1/-1 counter"
    );

    assert!(
        game.player(alice)
            .expect("alice exists")
            .graveyard
            .iter()
            .any(|&id| {
                game.object(id)
                    .map(|obj| obj.name == "Alice Spare Relic")
                    .unwrap_or(false)
            }),
        "Alice's noncreature card should stay in her graveyard"
    );
    assert!(
        game.player(bob)
            .expect("bob exists")
            .graveyard
            .iter()
            .any(|&id| {
                game.object(id)
                    .map(|obj| obj.name == "Bob Spare Relic")
                    .unwrap_or(false)
            }),
        "Bob's noncreature card should stay in his graveyard"
    );
    assert!(
        game.player(alice)
            .expect("alice exists")
            .graveyard
            .iter()
            .all(|&id| {
                game.object(id)
                    .map(|obj| obj.name != "Alice Revival Bear")
                    .unwrap_or(true)
            }),
        "Alice's creature should leave the graveyard"
    );
    assert!(
        game.player(bob)
            .expect("bob exists")
            .graveyard
            .iter()
            .all(|&id| {
                game.object(id)
                    .map(|obj| obj.name != "Bob Revival Bear")
                    .unwrap_or(true)
            }),
        "Bob's creature should leave the graveyard"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_spin_into_myth_fateseal_appends_fateseal_effect() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Spin into Myth Variant")
        .parse_text("Put target creature on top of its owner's library, then fateseal 2.")
        .expect("fateseal tail should parse");

    let effects = def.spell_effect.as_ref().expect("spell effects");
    let debug = format!("{effects:?}");
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        debug.contains("MoveToZoneEffect"),
        "expected move-to-library effect, got {debug}"
    );
    assert!(
        debug.contains("FatesealEffect") && debug.contains("count: Fixed(2)"),
        "expected fateseal-2 tail, got {debug}"
    );
    assert!(
        effects
            .iter()
            .any(|effect| effect.downcast_ref::<FatesealEffect>().is_some()),
        "expected concrete FatesealEffect lowering, got {debug}"
    );
    assert!(
        rendered.contains("Fateseal 2"),
        "expected compiled text to preserve fateseal wording, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_mesmeric_sliver_retains_fateseal_keyword_action() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Mesmeric Sliver Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("All Slivers have \"When this permanent enters, you may fateseal 1.\"")
        .expect("mesmeric sliver text should parse");

    let debug = format!("{def:?}");
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        debug.contains("FatesealEffect") && rendered.to_ascii_lowercase().contains("fateseal 1"),
        "expected fateseal trigger lowering and rendering, got debug={debug} rendered={rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_adder_staff_boggart_clash_followup_stays_conditional() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Adder-Staff Boggart Variant")
            .card_types(vec![CardType::Creature])
            .parse_text(
                "When this creature enters, clash with an opponent. If you win, put a +1/+1 counter on this creature.",
            )
            .expect("clash trigger should parse");

    let debug = format!("{def:?}");
    assert!(
        debug.contains("ClashEffect") && debug.contains("IfEffect"),
        "expected clash effect with conditional follow-up, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_clash_repeat_process_spell() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Clash Repeat Variant")
            .card_types(vec![CardType::Sorcery])
            .parse_text(
                "You lose 2 life and draw two cards, then clash with an opponent. If you win, repeat this process.",
            )
            .expect("clash repeat process should parse");

    let debug = format!("{def:?}");
    assert!(
        debug.contains("RepeatProcessEffect")
            && debug.contains("LoseLifeEffect")
            && debug.contains("DrawCardsEffect")
            && debug.contains("ClashEffect"),
        "expected repeated lose/draw/clash process, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_marvo_supports_defending_player_clash_and_win_trigger() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Marvo Variant")
            .card_types(vec![CardType::Creature])
            .parse_text(
                "Whenever this creature attacks, clash with defending player.\nWhenever you win a clash, draw a card.",
            )
            .expect("marvo-style clash text should parse");

    let debug = format!("{def:?}");
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        debug.contains("WinsClashTrigger")
            && rendered
                .to_ascii_lowercase()
                .contains("clash with defending player"),
        "expected defending-player clash support and win-a-clash trigger, got debug={debug} rendered={rendered}"
    );
    assert!(
        def.abilities
            .iter()
            .any(|ability| { format!("{ability:?}").contains("WinsClashTrigger") }),
        "expected a dedicated wins-clash trigger, got {debug}"
    );
    assert!(
        def.abilities.iter().any(|ability| {
            format!("{ability:?}").contains("ClashEffect")
                && format!("{ability:?}").contains("DefendingPlayer")
        }),
        "expected defending-player clash lowering, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_amass_clause_parses_structurally() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Widespread Brutality Variant")
            .parse_text(
                "Amass Zombies 2, then the Army you amassed deals damage equal to its power to each non-Army creature.",
            )
            .expect("amass clause should parse structurally");

    let spell_debug = format!("{:#?}", def.spell_effect).to_ascii_lowercase();
    assert!(
        spell_debug.contains("amasseffect"),
        "expected amass clause to compile to AmassEffect, got {spell_debug}"
    );
    assert!(
        spell_debug.contains("dealdamageeffect"),
        "expected downstream damage effect to remain parsed, got {spell_debug}"
    );
    assert!(
        spell_debug.contains("foreachobject"),
        "expected Widespread Brutality-style follow-up to fan out per non-Army creature, got {spell_debug}"
    );
    assert!(
        spell_debug.contains("executewithsourceeffect"),
        "expected follow-up damage to execute with the amassed Army as the source, got {spell_debug}"
    );
    assert!(
        spell_debug.contains("amassed_0") && spell_debug.contains("iterated"),
        "expected follow-up to route through the amassed Army tag and per-creature iteration, got {spell_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_amass_reflexive_damage_amount_uses_amassed_army_power() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Foray of Orcs")
            .parse_text(
                "Amass Orcs 2. When you do, Foray of Orcs deals X damage to target creature an opponent controls, where X is the amassed Army's power.",
            )
            .expect("Foray of Orcs style amass follow-up should parse");

    let spell_debug = format!("{:#?}", def.spell_effect);
    assert!(
        spell_debug.contains("ReflexiveTriggerEffect"),
        "expected reflexive amass follow-up to stay modeled as a when-you-do trigger, got {spell_debug}"
    );
    assert!(
        spell_debug.contains("amassed_0")
            && spell_debug.contains("PowerOf")
            && spell_debug.contains("Tagged"),
        "expected damage amount to derive from the amassed Army, got {spell_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_amass_followup_mill_amount_uses_amassed_army_power() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Surrounded by Orcs")
        .parse_text(
            "Amass Orcs 3, then target player mills X cards, where X is the amassed Army's power.",
        )
        .expect("Surrounded by Orcs style amass follow-up should parse");

    let spell_debug = format!("{:#?}", def.spell_effect);
    assert!(
        spell_debug.contains("MillEffect"),
        "expected mill follow-up to remain parsed, got {spell_debug}"
    );
    assert!(
        spell_debug.contains("amassed_0") && spell_debug.contains("__it__"),
        "expected mill amount to reference the amassed Army through the follow-up tag, got {spell_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_amass_followup_target_bound_uses_amassed_army_power() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Grishnakh Variant")
            .card_types(vec![CardType::Creature])
            .parse_text(
                "When this creature enters, amass Orcs 2. When you do, until end of turn, gain control of target nonlegendary creature an opponent controls with power less than or equal to the amassed Army's power. Untap that creature. It gains haste until end of turn.",
            )
            .expect("Grishnakh-style amass follow-up should parse");

    let abilities_debug = format!("{:#?}", def.abilities);
    assert!(
        abilities_debug.contains("ReflexiveTriggerEffect"),
        "expected reflexive trigger follow-up after amass, got {abilities_debug}"
    );
    assert!(
        abilities_debug.contains("LessThanOrEqualExpr")
            && abilities_debug.contains("amassed_0")
            && abilities_debug.contains("PowerOf"),
        "expected target power bound to reference the amassed Army, got {abilities_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_choose_from_graveyard_then_put_under_your_control() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Scrounge Variant")
            .parse_text(
                "Target opponent chooses an artifact card in their graveyard. Put that card onto the battlefield under your control.",
            )
            .expect("choose-from-graveyard then put-under-your-control should parse");

    let effects = def.spell_effect.as_ref().expect("spell effects");
    let debug = format!("{effects:?}");
    assert!(
        debug.contains("ChooseObjectsEffect")
            && debug.contains("zone: Some(Graveyard)")
            && debug.contains("owner: Some(IteratedPlayer)"),
        "expected choose-from-graveyard effect with iterated opponent ownership, got {debug}"
    );
    assert!(
        debug.contains("MoveToZoneEffect")
            && debug.contains("zone: Battlefield")
            && debug.contains("battlefield_controller: You"),
        "expected move-to-zone follow-up under your control, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_dredge_the_mire_each_opponent_chooses_from_graveyard() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Dredge the Mire Variant")
            .parse_text(
                "Each opponent chooses a creature card in their graveyard. Put those cards onto the battlefield under your control.",
            )
            .expect("Dredge the Mire style choose-and-reanimate sequence should parse");

    let effects = def.spell_effect.as_ref().expect("spell effects");
    let debug = format!("{effects:?}");
    assert!(
        debug.contains("ForPlayersEffect")
            && debug.contains("ChooseObjectsEffect")
            && debug.contains("zone: Some(Graveyard)")
            && debug.contains("owner: Some(IteratedPlayer)")
            && debug.contains("chooser: IteratedPlayer"),
        "expected per-opponent graveyard choice lowering, got {debug}"
    );
    assert!(
        debug.contains("MoveToZoneEffect")
            && debug.contains("target: Tagged(")
            && debug.contains("__it__")
            && debug.contains("zone: Battlefield")
            && debug.contains("battlefield_controller: You"),
        "expected chosen cards to be moved onto your battlefield, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_parley_revealed_this_way_uses_tagged_nonland_filter() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Parley Variant")
            .parse_text(
                "Each player reveals the top card of their library. For each nonland card revealed this way, you create a 3/3 green Elephant creature token. Then each player draws a card.",
            )
            .expect("parley revealed-this-way sentence should parse");

    let effects = def.spell_effect.as_ref().expect("spell effects");
    let debug = format!("{effects:#?}");
    assert!(
        debug.contains("RepeatEffectsEffect")
            && debug.contains("CardsRevealedThisWay")
            && debug.contains(
                "excluded_card_types: [\n                                                Land,"
            )
            && debug.contains("CreateTokenEffect"),
        "expected nonland revealed-this-way fanout, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_cant_transform_static_clause_stays_static_restriction() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Immerwolf Restriction Variant")
            .card_types(vec![CardType::Creature])
            .parse_text(
                "Intimidate.\nEach other creature you control that's a Wolf or a Werewolf gets +1/+1.\nNon-Human Werewolves you control can't transform.",
            )
            .expect("cant-transform static clause should parse as a static restriction");

    assert!(
        def.spell_effect.is_none(),
        "expected no spell effect from static cant-transform clause, got {:?}",
        def.spell_effect
    );

    let abilities_debug = format!("{:#?}", def.abilities);
    assert!(
        abilities_debug.contains("Transform(") && abilities_debug.contains("RuleRestriction"),
        "expected static RuleRestriction with transform prohibition, got {abilities_debug}"
    );
}
