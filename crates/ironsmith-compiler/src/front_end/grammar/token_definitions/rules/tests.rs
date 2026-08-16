use super::*;
use crate::lexer::lex_line;

fn parse_rules(raw: &str) -> TokenRulesSurfaces {
    let tokens = lex_line(raw, 0).expect("token rules fixture should lex");
    parse_token_rules_surfaces_tokens(&tokens)
}

#[test]
fn typed_amount_shapes_parse_crew_equip_and_power_bonus() {
    assert_eq!(
        parse_token_crew_shape_words(&["vehicle", "crew", "3"]),
        Some(TokenCrewShape { amount: 3 })
    );
    assert_eq!(
        parse_token_equip_shape_words(&["equipment", "equip", "2"]),
        Some(TokenEquipShape { amount: 2 })
    );
    assert_eq!(
        parse_token_power_as_though_greater_shape_words(&[
            "as", "though", "its", "power", "were", "4", "greater"
        ]),
        Some(TokenPowerAsThoughGreaterShape { amount: 4 })
    );

    let tokens = lex_line(
        "This creature saddles Mounts and crews Vehicles as though its power were 2 greater.",
        0,
    )
    .unwrap();
    assert_eq!(
        parse_token_power_as_though_greater_shape_tokens(&tokens),
        Some(TokenPowerAsThoughGreaterShape { amount: 2 })
    );
}

#[test]
fn inline_damage_shape_requires_trigger_subject_and_each_opponent() {
    let damage = lex_line(
        "Whenever you cast a noncreature spell, this token deals 2 damage to each opponent.",
        0,
    )
    .unwrap();
    assert_eq!(
        parse_inline_noncreature_spell_damage_tokens(&damage),
        Some(InlineNoncreatureSpellDamageShape { amount: 2 })
    );
    let draw = lex_line("Whenever you cast a noncreature spell, draw a card.", 0).unwrap();
    assert!(parse_inline_noncreature_spell_damage_tokens(&draw).is_none());
}

#[test]
fn token_rules_surface_distinguishes_keyword_and_parseable_rules() {
    let keyword = parse_rules("1/1 green Splinter creature token with \"Cumulative upkeep {G}\"");
    assert!(keyword.embedded_rules.is_empty());

    let triggered =
        parse_rules("2/2 creature token with \"Whenever this token attacks, draw a card.\"");
    assert!(triggered.embedded_rules.is_empty());

    let typed = parse_rules(
        "2/2 black Alien Angel artifact creature token with \"Whenever an opponent casts a creature spell, this token isn't a creature until end of turn.\"",
    );
    assert_eq!(
        typed.embedded_rules,
        vec![TokenEmbeddedRuleShape::OpponentCastsCreatureRemoveCreatureTypeUntilEndOfTurn]
    );

    let dies_create = parse_rules(
        "1/1 green Boar creature token with \"When this token dies, create a Food token.\"",
    );
    assert_eq!(
        dies_create.embedded_rules,
        vec![TokenEmbeddedRuleShape::DiesCreateBuiltinToken {
            token: BuiltinTokenShape::Food,
            count: 1,
        }]
    );
}

#[test]
fn creature_count_shortcut_requires_the_exact_typed_count_filter() {
    let creatures = parse_rules(
        "white Gnome creature token with \"This token's power and toughness are each equal to the number of creatures you control.\"",
    );
    assert_eq!(
        creatures.embedded_rules,
        vec![TokenEmbeddedRuleShape::PowerToughnessEqualCreaturesYouControl]
    );

    let artifacts_or_creatures = parse_rules(
        "white Gnome Soldier artifact creature token with \"This token's power and toughness are each equal to the number of artifacts and/or creatures you control.\"",
    );
    assert!(
        artifacts_or_creatures.embedded_rules.is_empty(),
        "the broader typed count must be left for generic CDA parsing: {artifacts_or_creatures:#?}"
    );
}

#[test]
fn labeled_land_entry_counter_rule_is_typed_and_validates_its_self_reference() {
    let tokens = lex_line(
        "legendary 2/2 green Cat creature token named Zabu with \"Landfall — Whenever a land you control enters, put a +1/+1 counter on Zabu.\"",
        0,
    )
    .unwrap();
    let rules = parse_token_rules_surfaces_for_named_token(&tokens, Some("Zabu"));
    assert_eq!(
        rules.embedded_rules,
        vec![TokenEmbeddedRuleShape::LandEntersPutCountersOnSelf {
            counter_type: CounterType::PlusOnePlusOne,
            count: 1,
        }]
    );

    let wrong_name = parse_token_rules_surfaces_for_named_token(&tokens, Some("Another token"));
    assert!(wrong_name.embedded_rules.is_empty());

    let pronoun = lex_line(
        "2/2 creature token with \"Whenever a land enters under your control, put two charge counters on this token.\"",
        0,
    )
    .unwrap();
    assert_eq!(
        parse_token_rules_surfaces_tokens(&pronoun).embedded_rules,
        vec![TokenEmbeddedRuleShape::LandEntersPutCountersOnSelf {
            counter_type: CounterType::Charge,
            count: 2,
        }]
    );
}

#[test]
fn token_tap_mana_rule_preserves_usage_restriction() {
    let tokens = lex_line(
        "1/1 red Wizard creature token with \"{T}: Add {R}. Spend this mana only to cast a planeswalker spell.\"",
        0,
    )
    .unwrap();
    let parsed = parse_token_tap_mana_ability_tokens(&tokens)
        .expect("quoted token mana ability should parse");
    assert_eq!(parsed.mana, vec![ManaSymbol::Red]);
    assert!(matches!(
        parsed.restrictions.as_slice(),
        [crate::ability::ManaUsageRestriction::CastSpell { card_types, .. }]
            if card_types == &[crate::types::CardType::Planeswalker]
    ));
}

#[test]
fn typed_damage_trigger_rules_preserve_combat_and_recipient_semantics() {
    let poison = parse_rules(
        "1/1 Snake artifact creature token with \"Whenever this creature deals damage to a player, that player gets a poison counter.\"",
    );
    assert_eq!(
        poison.embedded_rules,
        vec![TokenEmbeddedRuleShape::DealsDamageToPlayerPutCounters {
            combat_only: false,
            counter_type: CounterType::Poison,
            count: 1,
        }]
    );

    let pronoun_followup = parse_rules(
        "It has Whenever this creature deals damage to a player, that player gets a poison counter.",
    );
    assert_eq!(pronoun_followup.embedded_rules, poison.embedded_rules);

    let loses = parse_rules(
        "1/1 Assassin creature token with \"Whenever this token deals combat damage to a player, that player loses the game.\"",
    );
    assert_eq!(
        loses.embedded_rules,
        vec![TokenEmbeddedRuleShape::DealsDamageToPlayerLoseGame { combat_only: true }]
    );

    let destroy = parse_rules(
        "1/1 Assassin creature token with \"Whenever this token deals damage to a planeswalker, destroy that planeswalker.\"",
    );
    assert_eq!(
        destroy.embedded_rules,
        vec![TokenEmbeddedRuleShape::DealsDamageToPlaneswalkerDestroy { combat_only: false }]
    );
}

#[test]
fn typed_multisentence_token_rules_preserve_costs_and_fallbacks() {
    let upkeep = parse_rules(
        "6/6 Demon creature token with \"At the beginning of your upkeep, sacrifice another creature. If you can't, this token deals 6 damage to you.\"",
    );
    assert_eq!(
        upkeep.embedded_rules,
        vec![TokenEmbeddedRuleShape::BeginningOfYourUpkeepSacrificeAnotherCreatureOrSourceDamagesYou {
            damage: 6,
        }]
    );

    let mana = parse_rules(
        "colorless artifact token named Banana with \"{T}, Sacrifice this token: Add {R} or {G}. You gain 2 life.\"",
    );
    assert_eq!(
        mana.embedded_rules,
        vec![TokenEmbeddedRuleShape::TapSacrificeAddManaOrGainLife(
            TokenTapSacrificeManaLifeShape {
                mana_options: vec![ManaSymbol::Red, ManaSymbol::Green],
                life: 2,
            }
        )]
    );

    let any_color = parse_rules(
        "colorless artifact token named Etherium Cell with \"{T}, Sacrifice this token: Add one mana of any color.\"",
    );
    assert_eq!(
        any_color.embedded_rules,
        vec![TokenEmbeddedRuleShape::TapSacrificeAddManaOfAnyColor]
    );
}
