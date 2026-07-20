#![allow(unused_imports)]
use super::shard_00::*;
use super::shard_01::*;
use super::shard_02::*;
use super::shard_04::*;
use super::shard_05::*;
use super::shard_06::*;
use super::shard_07::*;
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
pub(super) fn painters_servant_strict_parse_and_compiled_text_regression() {
    let def = painters_servant_definition();
    let ids: Vec<StaticAbilityId> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();

    assert!(ids.contains(&StaticAbilityId::ChooseColorAsEnters));
    assert!(ids.contains(&StaticAbilityId::AddChosenColor));
    assert!(
        !ids.contains(&StaticAbilityId::RuleFallbackText)
            && !ids.contains(&StaticAbilityId::UnsupportedParserLine),
        "expected strict Painter's Servant static abilities, got {ids:?}"
    );

    let rendered = crate::compiled_text::unprocessed_compiled_lines(&def).join("\n");
    assert_eq!(
        rendered,
        "As this creature enters, choose a color.\nAll cards that aren't on the battlefield, spells, and permanents are the chosen color in addition to their other colors."
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn painters_servant_adds_chosen_color_to_permanents_spells_and_nonbattlefield_cards() {
    let def = painters_servant_definition();
    let red_creature = CardDefinitionBuilder::new(CardId::from_raw(391), "Red Creature")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Red]]))
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .parse_text("")
        .expect("red creature definition");
    let green_spell = CardDefinitionBuilder::new(CardId::from_raw(392), "Green Spell")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Green]]))
        .card_types(vec![CardType::Sorcery])
        .parse_text("Draw a card.")
        .expect("green spell definition");
    let colorless_artifact =
        CardDefinitionBuilder::new(CardId::from_raw(393), "Colorless Artifact")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)]]))
            .card_types(vec![CardType::Artifact])
            .parse_text("")
            .expect("colorless artifact definition");

    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let servant_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    game.set_chosen_color(servant_id, Color::Blue);
    let permanent_id = game.create_object_from_definition(&red_creature, alice, Zone::Battlefield);
    let hand_card_id = game.create_object_from_definition(&green_spell, alice, Zone::Hand);
    let graveyard_card_id =
        game.create_object_from_definition(&green_spell, alice, Zone::Graveyard);
    let exile_card_id = game.create_object_from_definition(&green_spell, alice, Zone::Exile);
    let library_card_id = game.create_object_from_definition(&green_spell, alice, Zone::Library);
    let spell_id = game.create_object_from_definition(&green_spell, alice, Zone::Stack);
    let colorless_id =
        game.create_object_from_definition(&colorless_artifact, alice, Zone::Battlefield);
    game.refresh_continuous_state();

    let permanent_colors = game.current_colors(permanent_id).expect("permanent colors");
    assert!(permanent_colors.contains(Color::Red));
    assert!(permanent_colors.contains(Color::Blue));

    let hand_card_colors = game.current_colors(hand_card_id).expect("hand card colors");
    assert!(hand_card_colors.contains(Color::Green));
    assert!(hand_card_colors.contains(Color::Blue));

    let graveyard_card_colors = game
        .current_colors(graveyard_card_id)
        .expect("graveyard card colors");
    assert!(graveyard_card_colors.contains(Color::Green));
    assert!(graveyard_card_colors.contains(Color::Blue));

    let exile_card_colors = game
        .current_colors(exile_card_id)
        .expect("exile card colors");
    assert!(exile_card_colors.contains(Color::Green));
    assert!(exile_card_colors.contains(Color::Blue));

    let library_card_colors = game
        .current_colors(library_card_id)
        .expect("library card colors");
    assert!(library_card_colors.contains(Color::Green));
    assert!(library_card_colors.contains(Color::Blue));

    let spell_colors = game.current_colors(spell_id).expect("spell colors");
    assert!(spell_colors.contains(Color::Green));
    assert!(spell_colors.contains(Color::Blue));

    let artifact_colors = game.current_colors(colorless_id).expect("artifact colors");
    assert!(artifact_colors.contains(Color::Blue));
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn painters_servant_does_not_add_color_without_a_chosen_color() {
    let def = painters_servant_definition();
    let red_creature = CardDefinitionBuilder::new(CardId::from_raw(394), "Red Creature")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Red]]))
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .parse_text("")
        .expect("red creature definition");

    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let permanent_id = game.create_object_from_definition(&red_creature, alice, Zone::Battlefield);
    let hand_card_id = game.create_object_from_definition(&red_creature, alice, Zone::Hand);
    game.refresh_continuous_state();

    let colors = game.current_colors(permanent_id).expect("permanent colors");
    assert!(colors.contains(Color::Red));
    assert!(!colors.contains(Color::Blue));

    let hand_colors = game.current_colors(hand_card_id).expect("hand card colors");
    assert!(hand_colors.contains(Color::Red));
    assert!(!hand_colors.contains(Color::Blue));
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_roaming_throne_variant_duplicates_matching_creature_triggers() {
    let throne_def = CardDefinitionBuilder::new(CardId::new(), "Roaming Throne Variant")
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .subtypes(vec![Subtype::Golem])
        .power_toughness(PowerToughness::fixed(4, 4))
        .parse_text(
            "As this creature enters, choose a creature type.\nThis creature is the chosen type in addition to its other types.\nIf a triggered ability of another creature you control of the chosen type triggers, it triggers an additional time.",
        )
        .expect("parse roaming throne trigger-doubling lines");
    let wall_def = crate::cards::definitions::wall_of_omens();

    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let throne_id =
        game.create_object_from_definition(&throne_def, alice, crate::zone::Zone::Battlefield);
    game.set_chosen_creature_type(throne_id, Subtype::Wall);

    let wall_id =
        game.create_object_from_definition(&wall_def, alice, crate::zone::Zone::Battlefield);
    let event = crate::events::RawEvent::new(
        crate::events::ZoneChangeEvent::with_cause(
            wall_id,
            crate::zone::Zone::Hand,
            crate::zone::Zone::Battlefield,
            crate::events::cause::EventCause::effect(),
            None,
        ),
        crate::provenance::ProvNodeId::default(),
    );

    let triggered = crate::triggers::check_triggers(&game, &event);
    let wall_entries = triggered
        .iter()
        .filter(|entry| entry.source == wall_id)
        .count();
    assert_eq!(
        wall_entries, 2,
        "expected Wall of Omens ETB trigger to fire twice, got {wall_entries}: {triggered:#?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_this_cost_is_reduced_by_basic_land_types_without_placeholder() {
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
            StaticAbilityId::RuleFallbackText,
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

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_basic_land_type_count_conditionals_for_you_control_tail() {
    let exact = CardDefinitionBuilder::new(CardId::from_raw(1), "Exact Domain Condition")
        .card_types(vec![CardType::Instant])
        .parse_text("If there are five basic land types among lands you control, draw a card.")
        .expect("parse exact basic-land-types conditional");
    let exact_rendered = unprocessed_compiled_lines(&exact)
        .join(" | ")
        .to_ascii_lowercase();
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
    let threshold_rendered = unprocessed_compiled_lines(&at_least)
        .join(" | ")
        .to_ascii_lowercase();
    assert!(
        threshold_rendered.contains("basic land type")
            && threshold_rendered.contains("among lands you control"),
        "expected rendered threshold conditional to mention basic land types among lands you control, got {threshold_rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_voices_from_the_void_domain_discard_counts_basic_land_types() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(3), "Voices from the Void")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(4)],
            vec![ManaSymbol::Black],
        ]))
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Domain — Target player discards a card for each basic land type among lands you control.",
        )
        .expect("Voices from the Void should parse");

    let raw = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        raw.contains("discardeffect") && raw.contains("basiclandtypesamong"),
        "expected Voices from the Void to compile into a domain discard count, got {raw}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains(
            "target player discards a card for each basic land type among lands you control"
        ),
        "expected Voices from the Void wording to keep the domain discard clause, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_atraxa_grand_unifier_uses_card_type_reveal_bundle() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(4), "Atraxa, Grand Unifier")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::Green],
            vec![ManaSymbol::White],
            vec![ManaSymbol::Blue],
            vec![ManaSymbol::Black],
        ]))
        .supertypes(vec![Supertype::Legendary])
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Phyrexian, Subtype::Angel])
        .power_toughness(PowerToughness::fixed(7, 7))
        .flying()
        .vigilance()
        .deathtouch()
        .lifelink()
        .parse_text(
            "When this creature enters, reveal the top ten cards of your library. For each card type, you may put a card of that type from among the revealed cards into your hand. Put the rest on the bottom of your library in a random order.",
        )
        .expect("Atraxa should parse");

    let debug = format!("{def:#?}");
    assert!(
        debug.contains("LookAtTopCardsEffect")
            && debug.contains("reveal: true")
            && debug.contains("PutTaggedRemainderOnLibraryBottomEffect"),
        "expected Atraxa to use one public reveal producer and an exact tagged remainder, got {debug}"
    );
    assert!(
        !debug.contains("RevealTaggedEffect"),
        "expected Atraxa not to split one public reveal into look-plus-reveal effects, got {debug}"
    );
    assert!(
        !debug.contains("zone: Graveyard"),
        "Atraxa's reveal bundle should not lower through a graveyard fallback, got {debug}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains(
            "reveal the top ten cards of your library. for each card type, you may put a card of that type from among the revealed cards into your hand. put the rest on the bottom of your library in a random order"
        ) && !rendered.contains("look at the top ten cards")
            && !rendered.contains("reveal them"),
        "expected Atraxa compiled text to preserve the reveal bundle structure, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_portcullis_exile_until_leaves_battlefield() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Portcullis")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(4)]]))
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "Whenever a creature enters, if there are two or more other creatures on the battlefield, exile that creature. Return that card to the battlefield under its owner's control when this artifact leaves the battlefield.",
        )
        .expect("Portcullis should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("if there are two or more other creatures on the battlefield"),
        "expected Portcullis condition to survive rendering, got {rendered}"
    );
    let debug = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        debug.contains("exileuntil") && debug.contains("sourceleavesbattlefield"),
        "expected Portcullis to compile into exile-until-source-leaves, got {debug}"
    );
    assert!(
        !rendered.contains("graveyard"),
        "Portcullis should no longer compile into a graveyard-return pattern, got {rendered}"
    );

    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Portcullis should have a triggered ability");
    let debug = format!("{:?}", triggered.intervening_if);
    assert!(
        triggered.intervening_if.is_some(),
        "expected Portcullis trigger to keep its battlefield-count condition, got {debug}"
    );
    assert!(
        debug.contains("ValueComparison") || debug.contains("CountComparison"),
        "expected Portcullis trigger condition to be count-based, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_damage_equal_to_thiss_power() {
    CardDefinitionBuilder::new(CardId::from_raw(1), "Power Reference")
        .parse_text("This deals damage equal to this's power to any target.")
        .expect("parse damage equal to this's power");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_characteristic_power_equal_number_of_creatures() {
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

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_characteristic_power_equal_greatest_mana_value() {
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

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_creatures_attack_this_turn_if_able_clause() {
    CardDefinitionBuilder::new(CardId::from_raw(1), "Instigate Combat")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Creatures your opponents control attack this turn if able.")
        .expect("parse creatures attack this turn if able");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_this_creature_must_be_blocked_if_able_static_line() {
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

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_double_cant_clause_from_text() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "No Win")
        .parse_text("You can't lose the game and your opponents can't win the game.")
        .expect("parse dual can't clause");

    let debug = format!("{:#?}", def.abilities);
    assert!(
        debug.contains("RuleRestriction")
            && debug.contains("LoseGame(You)")
            && debug.contains("WinGame(Opponent)"),
        "expected both typed game-result restrictions, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn everybody_lives_parses_strictly_with_player_hexproof_clause() {
    CardDefinitionBuilder::new(CardId::from_raw(1), "Everybody Lives!")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "All creatures gain hexproof and indestructible until end of turn. Players gain hexproof until end of turn. Players can't lose life this turn and players can't lose the game or win the game this turn.",
        )
        .expect("Everybody Lives! should parse strictly");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn everybody_lives_compiled_text_includes_player_hexproof_and_life_lock_clauses() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Everybody Lives!")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "All creatures gain hexproof and indestructible until end of turn. Players gain hexproof until end of turn. Players can't lose life this turn and players can't lose the game or win the game this turn.",
        )
        .expect("Everybody Lives! should parse strictly");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Players have hexproof this turn")
            && rendered.contains("Players can't lose life this turn")
            && rendered.contains("Players can't lose the game this turn")
            && rendered.contains("Players can't win the game this turn"),
        "expected Everybody Lives! compiled text to keep player clauses, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn gilded_light_parses_strictly_with_player_shroud_and_cycling() {
    CardDefinitionBuilder::new(CardId::from_raw(46_396), "Gilded Light")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::White],
        ]))
        .card_types(vec![CardType::Instant])
        .parse_text(
            "You gain shroud until end of turn. (You can't be the target of spells or abilities.)\nCycling {2} ({2}, Discard this card: Draw a card.)",
        )
        .expect("Gilded Light should parse strictly");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn gilded_light_compiled_text_includes_player_shroud_and_cycling() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(46_396), "Gilded Light")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::White],
        ]))
        .card_types(vec![CardType::Instant])
        .parse_text(
            "You gain shroud until end of turn. (You can't be the target of spells or abilities.)\nCycling {2} ({2}, Discard this card: Draw a card.)",
        )
        .expect("Gilded Light should parse strictly");

    assert_eq!(
        unprocessed_compiled_lines(&def),
        vec![
            "You gain shroud until end of turn.".to_string(),
            "Cycling {2}".to_string(),
        ],
        "expected Gilded Light compiled text to preserve player shroud and cycling"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_characteristic_pt_constant_plus_count() {
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

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_characteristic_pt_count_plus_count() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Soulless One")
        .parse_text(
            "Soulless One's power and toughness are each equal to the number of Zombies on the battlefield plus the number of Zombie cards in all graveyards.",
        )
        .expect("parse characteristic P/T count plus count");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("number of zombies on the battlefield"),
        "expected characteristic P/T zombie count wording, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_keyword_action_trigger_you_earthbend() {
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

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_keyword_action_trigger_any_player() {
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

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_keyword_action_trigger_you_surveil() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Surveil Watcher")
        .card_types(vec![CardType::Enchantment])
        .parse_text("Whenever you surveil, draw a card.")
        .expect("parse surveil trigger");

    let triggered = def
        .abilities
        .iter()
        .find_map(|a| match &a.kind {
            AbilityKind::Triggered(t) => Some(t),
            _ => None,
        })
        .expect("expected triggered ability");

    assert_eq!(triggered.trigger.display(), "Whenever you surveil");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_keyword_action_trigger_players_finish_voting() {
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

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_enters_tapped_filter_keeps_opponent_controller_constraint() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Frozen Aether Variant")
        .parse_text(
            "Artifacts, creatures, and lands your opponents control enter the battlefield tapped.",
        )
        .expect("should parse opponents-control enters tapped line");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" | ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("opponent"),
        "expected rendered line to preserve opponents controller filter, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_cohort_ability_word_prefix_keeps_cost_and_effect() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Ondu War Cleric")
        .card_types(vec![CardType::Creature])
        .parse_text("Cohort — {T}, Tap an untapped Ally you control: Target opponent loses 2 life.")
        .expect("parse cohort activated ability with label");

    let lines = unprocessed_compiled_lines(&def);
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

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_labeled_leading_condition_with_gets_and_has() {
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
        displays.iter().any(|display| display.contains("has flying")
            && display.contains("as long as you control three or more artifacts")),
        "expected conditional flying grant ability, got: {displays:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_coven_condition_uses_different_power_predicate() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Coven Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Coven — At the beginning of combat on your turn, if you control three or more creatures with different powers, this creature gains trample until end of turn.",
        )
        .expect("parse coven condition with different powers");

    let debug = format!("{:#?}", def.abilities);
    assert!(
        debug.contains("PlayerHasAtLeastWithDifferentPowers")
            || debug.contains("distinct_powers: true"),
        "expected coven predicate to require different powers, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_target_player_may_copy_this_spell_and_choose_new_targets() {
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
    let lines = unprocessed_compiled_lines(&def);
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

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_then_controller_may_copy_spell_and_choose_new_targets() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Chain of Acid Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Destroy target noncreature permanent. Then that permanent's controller may copy this spell and may choose a new target for that copy.",
        )
        .expect("parse then-controller copy-this-spell clause");

    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        (joined.contains("that object's controller may copy this spell")
            || joined.contains("that permanent's controller may copy this spell"))
            && !joined.contains("you may copy this spell"),
        "expected copy permission to stay linked to referenced controller, got {joined}"
    );
    assert!(
        joined.contains("that object's controller may copy this spell")
            || joined.contains("that permanent's controller may copy this spell"),
        "expected copy permission to stay linked to referenced controller, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_choose_new_targets_for_the_copy() {
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

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn beamsplitter_mage_parses_typed_spell_and_copy_references() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Beamsplitter Mage")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Blue],
            vec![ManaSymbol::Red],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Vedalken, Subtype::Wizard])
        .power_toughness(PowerToughness::fixed(2, 2))
        .parse_text(
            "Whenever you cast an instant or sorcery spell that targets only this creature, if you control one or more other creatures that spell could target, choose one of those creatures. Copy that spell. The copy targets the chosen creature.",
        )
        .expect("Beamsplitter Mage should parse");

    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Beamsplitter Mage should compile to a triggered ability");

    let trigger_debug = format!("{:#?}", triggered.trigger);
    assert!(
        trigger_debug.contains("targets_only_object") && trigger_debug.contains("target_count"),
        "expected trigger filter to require a spell targeting only this creature, got {trigger_debug}"
    );

    let condition_debug = format!("{:#?}", triggered.intervening_if);
    assert!(
        condition_debug.contains("could_be_targeted_by")
            && condition_debug.contains("stack_object: Tagged")
            && condition_debug.contains("triggering")
            && condition_debug.contains("other: true"),
        "expected intervening-if to ask for other creatures that spell could target, got {condition_debug}"
    );

    let choose_creature = triggered
        .effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<ChooseObjectsEffect>())
        .expect("trigger should choose one of the targetable creatures");
    assert_eq!(choose_creature.count, ChoiceCount::exactly(1));
    assert_eq!(choose_creature.filter.controller, Some(PlayerFilter::You));
    assert!(choose_creature.filter.other);
    assert_eq!(choose_creature.filter.card_types, vec![CardType::Creature]);
    assert_eq!(choose_creature.tag.as_str(), "__it__");
    let targetability = choose_creature
        .filter
        .could_be_targeted_by
        .as_ref()
        .expect("chosen creature filter should retain targetability constraint");
    assert_eq!(
        targetability.stack_object,
        ObjectRef::Tagged(crate::TagKey::from("triggering"))
    );

    let effects_debug = format!("{:#?}", triggered.effects);
    assert!(
        effects_debug.contains("CopySpellEffect")
            && effects_debug.contains("__copied_stack_object__")
            && effects_debug.contains("RetargetStackObjectEffect"),
        "expected tagged copy plus retarget effect, got {effects_debug}"
    );

    let rendered = debug_compiled_lines(&def).join("\n");
    assert!(
        rendered.contains("if you control one or more other creatures that spell could target"),
        "expected targetability condition to render with one-or-more wording, got {rendered}"
    );
    assert!(
        rendered.contains("you choose one of those creatures")
            && rendered.contains("Copy that spell")
            && rendered.contains("The copy targets the chosen creature"),
        "expected choose/copy/retarget sequence to render oracle-like text, got {rendered}"
    );
    assert!(
        !rendered.contains("tags it as") && !rendered.contains("Change a target of the copy"),
        "expected internal tags and retarget scaffolding to stay out of rendered text, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn copy_assignment_card_definition(
    name: &str,
    text: &str,
) -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .parse_text(text)
        .unwrap_or_else(|err| panic!("{name} should parse copy-assignment text: {err:?}"))
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn copy_assignment_family_parses_to_multi_copy_primitive() {
    let cases = [
        (
            "Zada, Hedron Grinder",
            "Whenever you cast an instant or sorcery spell that targets only Zada, copy that spell for each other creature you control that the spell could target. Each copy targets a different one of those creatures.",
        ),
        (
            "Ink-Treader Nephilim",
            "Whenever a player casts an instant or sorcery spell, if that spell targets only this creature, copy the spell for each other creature that spell could target. Each copy targets a different one of those creatures.",
        ),
        (
            "Mirrorwing Dragon",
            "Flying\nWhenever a player casts an instant or sorcery spell that targets only this creature, that player copies that spell for each other creature they control that the spell could target. Each copy targets a different one of those creatures.",
        ),
        (
            "Precursor Golem",
            "When this creature enters, create two 3/3 colorless Golem artifact creature tokens.\nWhenever a player casts an instant or sorcery spell that targets only a single Golem, that player copies that spell for each other Golem that spell could target. Each copy targets a different one of those Golems.",
        ),
        (
            "Radiant Performer",
            "Flash\nWhen this creature enters, if you cast it from your hand, choose target spell or ability that targets only a single permanent or player. Copy that spell or ability for each other permanent or player the spell or ability could target. Each copy targets a different one of those permanents and players.",
        ),
        (
            "Agrus Kos, Eternal Soldier",
            "Vigilance\nWhenever Agrus Kos becomes the target of an ability that targets only it, you may pay {1}{R/W}. If you do, copy that ability for each other creature you control that ability could target. Each copy targets a different one of those creatures.",
        ),
    ];

    for (name, text) in cases {
        let def = copy_assignment_card_definition(name, text);
        let debug = format!("{:#?}", def.abilities);
        assert!(
            debug.contains("CopySpellForEachTargetEffect"),
            "expected {name} to lower through CopySpellForEachTargetEffect, got {debug}"
        );
        assert!(
            debug.contains("exclude_current_targets: true"),
            "expected {name} to interpret 'other' as excluding current targets, got {debug}"
        );
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn ink_treader_nephilim_merges_targeting_condition_into_spell_trigger() {
    let def = copy_assignment_card_definition(
        "Ink-Treader Nephilim",
        "Whenever a player casts an instant or sorcery spell, if that spell targets only this creature, copy the spell for each other creature that spell could target. Each copy targets a different one of those creatures.",
    );

    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Ink-Treader Nephilim should compile to a triggered ability");

    let trigger_debug = format!("{:#?}", triggered.trigger);
    assert!(
        trigger_debug.contains("targets_only_object")
            && trigger_debug.contains("source: true")
            && trigger_debug.contains("target_count"),
        "expected targeting-only condition to become part of the spell-cast trigger, got {trigger_debug}"
    );
    assert!(
        triggered.intervening_if.is_none(),
        "targeting-only spell condition should not remain as an unevaluable intervening-if: {:#?}",
        triggered.intervening_if
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join("\n")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("instant or sorcery spell that targets only this creature"),
        "expected trigger text to retain the targeting-only spell filter, got {rendered}"
    );
    assert!(
        rendered.contains("copy that spell for each other creature that spell could target")
            && rendered.contains("each copy targets a different one of those creatures"),
        "expected copy-for-each-target text to render from the reusable effect, got {rendered}"
    );
    assert!(
        !rendered.contains("unsupported effect"),
        "copy-for-each-target should render without unsupported fallback, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn radiate_parses_chosen_spell_reference_to_multi_copy_primitive() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Radiate")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Choose target instant or sorcery spell that targets only a single permanent or player. Copy that spell for each other permanent or player the spell could target. Each copy targets a different one of those permanents and players.",
        )
        .expect("Radiate should parse");

    let debug = format!("{:#?}", def.spell_effect);
    assert!(
        debug.contains("TargetOnlyEffect") && debug.contains("CopySpellForEachTargetEffect"),
        "expected Radiate to choose a stack object and use copy-for-each target assignment, got {debug}"
    );
    assert!(
        debug.contains("Tagged(") && debug.contains("targeted_0"),
        "expected Radiate's 'that spell' to reference the chosen target spell, got {debug}"
    );
    assert!(
        debug.contains("player_filter: Some")
            && debug.contains("Any")
            && debug.contains("zone: Some")
            && debug.contains("Battlefield"),
        "expected Radiate candidates to include permanents and players, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn feather_radiant_arbiter_parses_plural_paid_copy_assignment() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Feather, Radiant Arbiter")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Red],
            vec![ManaSymbol::White],
            vec![ManaSymbol::White],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Angel])
        .power_toughness(PowerToughness::fixed(4, 3))
        .parse_text(
            "Flying, lifelink\nWhenever you cast a noncreature spell that targets only Feather, you may choose any number of other creatures that spell could target and pay {2} for each of those creatures. If you do, for each of those creatures, copy that spell. The copy targets that creature.",
        )
        .expect("Feather should parse plural paid copy assignment");

    let debug = format!("{:#?}", def.abilities);
    assert!(
        debug.contains("ChooseObjectsEffect") && debug.contains("max: None"),
        "expected Feather to choose any number of creatures, got {debug}"
    );
    assert!(
        debug.matches("ForEachTaggedEffect").count() >= 2 && debug.contains("PayManaEffect"),
        "expected Feather to pay once for each chosen creature, got {debug}"
    );
    assert!(
        debug.contains("CopySpellEffect") && debug.contains("RetargetStackObjectEffect"),
        "expected Feather to copy the triggering spell once per chosen creature and retarget each copy, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_krark_coin_flip_trigger_keeps_both_flip_branches() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Krark, the Thumbless")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Goblin, Subtype::Wizard])
        .parse_text(
            "Whenever you cast an instant or sorcery spell, flip a coin. If you lose the flip, return that spell to its owner's hand. If you win the flip, copy that spell, and you may choose new targets for the copy.\nPartner (You can have two commanders if both have partner.)",
        )
        .expect("Krark should parse");

    let debug = format!("{:?}", def.abilities);
    assert!(debug.contains("FlipCoinEffect"), "{debug}");
    assert!(debug.contains("ReturnToHandEffect"), "{debug}");
    assert!(debug.contains("CopySpellEffect"), "{debug}");
    assert!(debug.contains("ChooseNewTargetsEffect"), "{debug}");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_aberrant_mind_sorcerer_rolls_and_branch_ranges() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Aberrant Mind Sorcerer")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Psionic Spells — When this creature enters, choose target instant or sorcery card in your graveyard, then roll a d20.\n1—9 | You may put that card on top of your library.\n10—20 | Return that card to your hand.",
        )
        .expect("Aberrant Mind Sorcerer should parse");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("RollDieEffect"),
        "expected die-roll effect in parsed ability, got {debug}"
    );
    assert!(
        debug.contains("BetweenInclusive(1, 9)") && debug.contains("BetweenInclusive(10, 20)"),
        "expected numeric result branches in parsed ability, got {debug}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("roll a d20"),
        "expected d20 roll in compiled text, got {rendered}"
    );
    assert!(
        (rendered.contains("if you roll 1-9") && rendered.contains("if you roll 10-20"))
            || (rendered.contains("1—9") && rendered.contains("10—20")),
        "expected numeric roll branches in compiled text, got {rendered}"
    );
    assert!(
        (rendered.contains("you may put that card on top of")
            || rendered.contains("you may put it on top of"))
            && (rendered.contains("return that card to") || rendered.contains("return it to")),
        "expected both psionic spells outcomes in compiled text, got {rendered}"
    );
}

#[test]
pub(super) fn component_pouch_strict_parser_compiled_text_and_structure_regression() {
    let def = parse_oracle_card_definition("Component Pouch");
    let rendered = unprocessed_compiled_lines(&def);
    assert_eq!(
        rendered.join("\n"),
        "{T}, Remove a component counter from this artifact: Add two mana of different colors.\n{T}: Roll a d20.\n1—9 | Put a component counter on this artifact.\n10—20 | Put two component counters on this artifact.",
        "Component Pouch should compile back to its strict oracle text"
    );

    let debug = format!("{:#?}", def.abilities);
    let compact_debug = debug.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        compact_debug.contains("RemoveCountersEffect")
            && compact_debug.contains("Named( \"component\"")
            && compact_debug.contains("distinct_colors: true"),
        "expected Component Pouch mana ability to remove a component counter and add distinct-color mana, got {debug}"
    );
    assert!(
        compact_debug.contains("RollDieEffect")
            && compact_debug.contains("sides: 20")
            && compact_debug.contains("BetweenInclusive( 1, 9")
            && compact_debug.contains("BetweenInclusive( 10, 20"),
        "expected Component Pouch d20 branches for 1-9 and 10-20, got {debug}"
    );
    assert!(
        compact_debug.contains("PutCountersEffect")
            && compact_debug.contains("Named( \"component\"")
            && compact_debug.contains("Fixed( 2"),
        "expected Component Pouch d20 high branch to put two component counters, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_arden_angel_rolls_four_sided_die_and_returns_itself_from_graveyard() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Arden Angel")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Angel])
        .power_toughness(PowerToughness::fixed(4, 4))
        .parse_text(
            "Flying\nAt the beginning of your upkeep, if Arden Angel is in your graveyard, roll a four-sided die. If the result is 1, return Arden Angel from your graveyard to the battlefield.",
        )
        .expect("Arden Angel should parse strictly");

    let debug = format!("{:?}", def.abilities);
    assert!(debug.contains("RollDieEffect"), "{debug}");
    assert!(debug.contains("sides: 4"), "{debug}");
    assert!(debug.contains("SourceIsInZone(Graveyard)"), "{debug}");
    assert!(
        debug.contains("ReturnFromGraveyardToBattlefieldEffect")
            && debug.contains("FullName(\"Arden Angel\")")
            && !debug.contains("ChooseObjectsEffect"),
        "expected Arden Angel to return itself, not choose an Angel card, got {debug}"
    );

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("if Arden Angel is in your graveyard"),
        "expected named graveyard intervening-if condition, got {rendered}"
    );
    assert!(
        rendered.contains("roll a four-sided die") && rendered.contains("If the result is 1"),
        "expected four-sided die and exact-result branch wording, got {rendered}"
    );
    assert!(
        rendered.contains("return Arden Angel from your graveyard to the battlefield"),
        "expected named self-return wording, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_clown_car_compiled_text_keeps_odd_even_branches_and_token_identity() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Clown Car")
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .subtypes(vec![Subtype::Vehicle])
        .parse_text(
            "When this Vehicle enters, roll X six-sided dice. For each odd result, create a 1/1 white Clown Robot artifact creature token. For each even result, put a +1/+1 counter on this Vehicle.\nCrew 2",
        )
        .expect("Clown Car text should parse");

    let abilities_debug = format!("{:?}", def.abilities);
    assert!(
        abilities_debug.contains("RepeatEffects"),
        "{abilities_debug}"
    );
    assert!(
        abilities_debug.contains("RollDieEffect"),
        "{abilities_debug}"
    );
    assert!(
        abilities_debug.contains("OneOf([1, 3, 5])"),
        "{abilities_debug}"
    );
    assert!(
        abilities_debug.contains("OneOf([2, 4, 6])"),
        "{abilities_debug}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("roll") && rendered.contains("d6"),
        "expected roll clause in compiled text, got {rendered}"
    );
    assert!(
        rendered.contains("if you roll 1, 3, or 5") && rendered.contains("if you roll 2, 4, or 6"),
        "expected odd/even result branch conditions in compiled text, got {rendered}"
    );
    assert!(
        !rendered.contains("its count is one of"),
        "expected die-result branch wording, not generic count wording, got {rendered}"
    );
    assert!(
        rendered.contains("1/1")
            && rendered.contains("white")
            && rendered.contains("robot")
            && rendered.contains("artifact creature token"),
        "expected Clown Robot token creation payload in compiled text, got {rendered}"
    );
    assert!(
        rendered.contains("+1/+1 counter") && rendered.contains("this vehicle"),
        "expected even-result +1/+1 counter branch in compiled text, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_complaints_clerk_roll_one_trigger_creates_clown_robot() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Complaints Clerk")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Beast])
        .power_toughness(PowerToughness::fixed(3, 3))
        .parse_text(
            "When this creature enters, open an Attraction. (Put the top card of your Attraction deck onto the battlefield.)\nWhenever you roll a 1, create a 1/1 white Clown Robot artifact creature token.",
        )
        .expect("Complaints Clerk should parse strictly");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("PlayerRollsResult") && debug.contains("result: 1"),
        "expected exact roll-one trigger, got {debug}"
    );
    assert!(
        debug.contains("CreateToken") && debug.contains("Clown") && debug.contains("Robot"),
        "expected Clown Robot token creation payload, got {debug}"
    );

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Whenever you roll a 1"),
        "expected roll-one trigger wording, got {rendered}"
    );
    assert!(
        rendered.contains("1/1 white Clown Robot artifact creature token"),
        "expected Clown Robot token wording, got {rendered}"
    );
}

#[test]
pub(super) fn netherese_puzzle_ward_strict_parser_compiled_text_and_structure_regression() {
    let def = parse_oracle_card_definition("Netherese Puzzle-Ward");
    let rendered = unprocessed_compiled_lines(&def).join("\n");
    assert!(
        rendered.contains("At the beginning of your upkeep")
            && rendered.contains("roll a d4")
            && rendered.contains("Scry X, where X is the result"),
        "expected Focus Beam upkeep roll-and-scry text, got {rendered}"
    );
    assert!(
        rendered.contains("Whenever you roll a die's highest natural result, draw a card"),
        "expected Perfect Illumination highest-natural-result trigger text, got {rendered}"
    );

    let debug = format!("{:#?}", def.abilities);
    assert!(
        debug.contains("RollDieEffect") && debug.contains("sides: 4"),
        "expected Netherese Puzzle-Ward to roll a d4, got {debug}"
    );
    assert!(
        debug.contains("PlayerRollsHighestNaturalResult") && debug.contains("DrawCardsEffect"),
        "expected highest-natural-result die-roll trigger to draw a card, got {debug}"
    );
}

#[test]
pub(super) fn the_space_family_goblinson_strict_parser_compiled_text_and_structure_regression() {
    assert_oracle_card_parses_strict("The Space Family Goblinson");
    let def = parse_oracle_card_definition("The Space Family Goblinson");
    let rendered = unprocessed_compiled_lines(&def).join("\n");

    assert_eq!(
        rendered,
        "As long as you've rolled three or more dice this turn, The Space Family Goblinson has trample.\nWhenever you roll a die, put a +1/+1 counter on The Space Family Goblinson.",
        "The Space Family Goblinson should compile back to its strict oracle text"
    );

    let debug = format!("{:#?}", def.abilities);
    let compact_debug = debug.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        compact_debug.contains("MaxDiceRolledThisTurn(You)")
            && compact_debug.contains("GreaterThanOrEqual")
            && compact_debug.contains("Fixed(3)")
            && compact_debug.contains("PlayerRollsDieTrigger")
            && compact_debug.contains("PutCountersEffect")
            && compact_debug.contains("FullName( \"The Space Family Goblinson\"")
            && !compact_debug.contains("UnsupportedParserLine")
            && !compact_debug.contains("RuleFallbackText"),
        "expected structural dice threshold, roll-any-die trigger, and named self counter effect, got {debug}"
    );
}

#[test]
pub(super) fn the_space_family_goblinson_runtime_applies_dice_trigger_and_trample_threshold() {
    let def = parse_oracle_card_definition("The Space Family Goblinson");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let goblinson_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

    assert!(
        !game.object_has_static_ability_id(goblinson_id, StaticAbilityId::Trample),
        "The Space Family Goblinson should not have trample before three dice are rolled"
    );

    game.turn_store.turn_history.record_die_roll(alice, 2);
    game.turn_store.turn_history.record_die_roll(alice, 5);
    assert!(
        !game.object_has_static_ability_id(goblinson_id, StaticAbilityId::Trample),
        "The Space Family Goblinson should not have trample after only two dice"
    );

    game.turn_store.turn_history.record_die_roll(alice, 1);
    assert!(
        game.object_has_static_ability_id(goblinson_id, StaticAbilityId::Trample),
        "The Space Family Goblinson should have trample after you roll three dice"
    );

    let bob_roll = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::other::DieRolledEvent::new(bob, goblinson_id, 4, 6),
        crate::provenance::ProvNodeId::default(),
    );
    assert_eq!(
        resolve_triggers_for_source(&mut game, goblinson_id, &bob_roll),
        0,
        "The Space Family Goblinson should not trigger when an opponent rolls a die"
    );
    assert_eq!(
        game.counter_count(goblinson_id, crate::object::CounterType::PlusOnePlusOne),
        0,
        "opponent die rolls should not put counters on The Space Family Goblinson"
    );

    let alice_roll = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::other::DieRolledEvent::new(alice, goblinson_id, 6, 6),
        crate::provenance::ProvNodeId::default(),
    );
    assert_eq!(
        resolve_triggers_for_source(&mut game, goblinson_id, &alice_roll),
        1,
        "The Space Family Goblinson should trigger when you roll any die result"
    );
    assert_eq!(
        game.counter_count(goblinson_id, crate::object::CounterType::PlusOnePlusOne),
        1,
        "your die roll should put one +1/+1 counter on The Space Family Goblinson"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_sevinnes_reclamation_flashback_copy_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Sevinne's Reclamation")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Return target permanent card with mana value 3 or less from your graveyard to the battlefield.\nIf this spell was cast from a graveyard, you may copy this spell and may choose a new target for the copy.\nFlashback {4}{W}",
        )
        .expect("Sevinne's Reclamation text should parse");

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("ReturnFromGraveyardToBattlefieldEffect"),
        "expected graveyard return effect, got {debug}"
    );
    assert!(
        debug.contains("ThisSpellWasCastFromZone(Graveyard)"),
        "expected graveyard-cast condition, got {debug}"
    );
    assert!(
        debug.contains("CopySpellEffect"),
        "expected copy effect in flashback conditional, got {debug}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("if this spell was cast from a graveyard"),
        "expected graveyard-cast conditional in compiled text, got {rendered}"
    );
    assert!(
        rendered.contains("you may copy this spell"),
        "expected copy clause in compiled text, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn increasing_confusion_strict_parse_renders_mill_self_replacement() {
    assert_oracle_card_parses_strict("Increasing Confusion");

    let def = parse_oracle_card_definition("Increasing Confusion");
    let debug = format!("{:#?}", def.spell_effect);
    assert!(
        debug.contains("SelfReplacementBranch") && debug.contains("MillEffect"),
        "Increasing Confusion should lower the flashback clause to a mill self-replacement, got {debug}"
    );
    assert!(
        debug.contains("ThisSpellWasCastFromZone") && debug.contains("Graveyard"),
        "Increasing Confusion replacement should be gated on being cast from a graveyard, got {debug}"
    );
    assert!(
        debug.contains("XTimes(2)") || debug.contains("Scaled"),
        "Increasing Confusion replacement should mill twice the default X count, got {debug}"
    );

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Target player mills X cards"),
        "Increasing Confusion should render the base target-player mill clause, got {rendered}"
    );
    assert!(
        rendered.contains(
            "If this spell was cast from a graveyard, that player mills twice that many cards instead"
        ),
        "Increasing Confusion should render the shared-target twice-that-many replacement, got {rendered}"
    );
    assert!(
        rendered.contains("Flashback"),
        "Increasing Confusion should render flashback, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_the_sixth_doctor_copy_clause_keeps_legendary_exception() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(81_601), "The Sixth Doctor")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(4)],
            vec![ManaSymbol::Green],
            vec![ManaSymbol::Blue],
        ]))
        .card_types(vec![CardType::Creature])
        .supertypes(vec![crate::types::Supertype::Legendary])
        .subtypes(vec![crate::types::Subtype::Doctor])
        .power_toughness(crate::card::PowerToughness::fixed(3, 3))
        .parse_text(
            "Time Lord's Prerogative — Whenever you cast a historic spell, copy it, except the copy isn't legendary. This ability triggers only once each turn.",
        )
        .expect("The Sixth Doctor should parse");

    let debug = format!("{:#?}", def.abilities);
    assert!(
        debug.contains("CopySpellEffect"),
        "expected copy-spell trigger in compiled ability, got {debug}"
    );
    assert!(
        debug.contains("removed_supertypes") && debug.contains("Legendary"),
        "expected legendary removal to lower into the copy effect, got {debug}"
    );
    assert!(
        debug.contains("MaxTimesEachTurn"),
        "expected once-per-turn limiter to survive lowering, got {debug}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("copy that spell or ability, except the copy isn't legendary"),
        "expected legendary exception to survive rendering, got {rendered}"
    );
    assert!(
        rendered.contains("this ability triggers only once each turn"),
        "expected explicit once-per-turn surface to survive rendering, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_gain_keyword_ability_does_not_fall_back_to_gain_life() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Gain Keyword Variant")
        .card_types(vec![CardType::Instant])
        .parse_text("Put a +1/+1 counter on target creature you control and it gains deathtouch until end of turn.")
        .expect("parse gain-keyword line");

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("AddAbility") && debug.contains("Deathtouch"),
        "expected ability grant effect, got {debug}"
    );
    assert!(
        !debug.contains("GainLifeEffect"),
        "did not expect life-gain fallback for keyword grant, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_lose_keyword_ability_does_not_fall_back_to_lose_life() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Lose Keyword Variant")
        .card_types(vec![CardType::Instant])
        .parse_text("Target creature loses flying until end of turn.")
        .expect("parse lose-keyword line");

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("RemoveAbility") && debug.contains("Flying"),
        "expected ability removal effect, got {debug}"
    );
    assert!(
        !debug.contains("LoseLifeEffect"),
        "did not expect life-loss fallback for keyword removal, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_lose_keyword_ability_without_duration() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Lose Keyword No Duration Variant")
        .card_types(vec![CardType::Instant])
        .parse_text("Target creature loses flying.")
        .expect("parse lose-keyword line without explicit duration");

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("RemoveAbility") && debug.contains("Flying"),
        "expected flying-removal effect, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_copy_this_spell_for_each_creature_sacrificed_this_way() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Plumb Variant")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "As an additional cost to cast this spell, you may sacrifice one or more creatures. When you do, copy this spell for each creature sacrificed this way.\nYou draw a card and you lose 1 life.",
        )
        .expect("Plumb-style additional cost should parse");
    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains(
            "as an additional cost to cast this spell, you may sacrifice one or more creatures"
        ),
        "expected repeatable optional additional cost line, got {rendered}"
    );
    assert!(
        rendered.contains(
            "as an additional cost to cast this spell, you may sacrifice one or more creatures. when you do, copy this spell for each creature sacrificed this way"
        ) && !rendered.contains("optional cost 'additional'")
            && !rendered.contains("when you cast this spell"),
        "expected the structural helper trigger to render inline with its repeatable sacrifice cost, got {rendered}"
    );
    assert_eq!(
        def.optional_costs.len(),
        1,
        "expected one parser-generated optional cost"
    );
    assert!(
        def.optional_costs[0].repeatable,
        "expected sacrifice additional cost to be repeatable"
    );

    let abilities_debug = format!("{:#?}", def.abilities);
    assert!(
        abilities_debug.contains("YouCastThisSpellTrigger")
            && abilities_debug.contains("ThisSpellPaidLabel"),
        "expected cast trigger guarded by paid additional-cost condition, got {abilities_debug}"
    );

    let spell_debug = format!("{:?}", def.spell_effect);
    assert!(
        spell_debug.contains("DrawCardsEffect") && spell_debug.contains("LoseLifeEffect"),
        "expected Plumb resolution effects, got {spell_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_plumb_style_additional_cost_trigger_copies_for_each_payment() {
    use crate::ability::AbilityKind;
    use crate::cost::OptionalCostsPaid;
    use crate::effects::{ExecutionContext, execute_effect};
    use crate::game_state::StackEntry;
    use crate::object::ObjectKind;
    use crate::tests::test_helpers::setup_two_player_game;
    use crate::zone::Zone;

    let mut game = setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let def = CardDefinitionBuilder::new(CardId::new(), "Plumb Variant")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "As an additional cost to cast this spell, you may sacrifice one or more creatures. When you do, copy this spell for each creature sacrificed this way.\nYou draw a card and you lose 1 life.",
        )
        .expect("Plumb-style additional cost should parse");

    let source = game.create_object_from_definition(&def, alice, Zone::Stack);
    let mut paid = OptionalCostsPaid::from_costs(&def.optional_costs);
    paid.pay_times(0, 2);
    game.object_mut(source)
        .expect("source object exists")
        .optional_costs_paid = paid.clone();
    game.stack
        .push(StackEntry::new(source, alice).with_optional_costs_paid(paid.clone()));

    let effects = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) if triggered.trigger.display().contains("cast") => {
                Some(triggered.effects.clone())
            }
            _ => None,
        })
        .expect("expected cast trigger for parser-generated Plumb support");

    let mut ctx = ExecutionContext::new_default(source, alice).with_optional_costs_paid(paid);
    for effect in &effects {
        execute_effect(&mut game, effect, &mut ctx).expect("copy effect should resolve");
    }

    let copied_spells: Vec<_> = game
        .stack
        .iter()
        .filter_map(|entry| game.object(entry.object_id))
        .filter(|obj| game.controller_of(obj) == alice && obj.name == "Plumb Variant")
        .collect();
    assert_eq!(copied_spells.len(), 3, "expected original plus two copies");
    assert_eq!(
        copied_spells
            .iter()
            .filter(|obj| obj.kind == ObjectKind::SpellCopy)
            .count(),
        2,
        "expected copies to be created from optional-cost payment count"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_additional_cost_tap_two_untapped_creatures_and_or_lands() {
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
        .find_map(|effect| {
            effect
                .downcast_ref::<crate::effects::TaggedEffect>()
                .map(|tagged| tagged.effect.as_ref())
                .unwrap_or(effect)
                .downcast_ref::<crate::effects::TapEffect>()
        })
        .expect("expected tap cost effect");
    let (inner, count) = match &tap.target {
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

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_additional_cost_tap_four_untapped_artifacts_creatures_or_lands() {
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
        .find_map(|effect| {
            effect
                .downcast_ref::<crate::effects::TaggedEffect>()
                .map(|tagged| tagged.effect.as_ref())
                .unwrap_or(effect)
                .downcast_ref::<crate::effects::TapEffect>()
        })
        .expect("expected tap cost effect");
    let (inner, count) = match &tap.target {
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

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_target_opponent_gains_control_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Witch Engine Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("{T}: Add {B}{B}{B}{B}. Target opponent gains control of this creature. (Activate only as an instant.)")
        .expect("parse target-opponent gain-control clause");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("ChangeControllerToPlayer(Target(Opponent))"),
        "expected gain-control runtime modification to resolve target opponent, got {debug}"
    );
    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("target opponent gains control of it"),
        "expected compiled text to preserve target opponent control change, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_gain_control_each_noncommander_creature_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Subjugate the Hobbits Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Gain control of each noncommander creature with mana value 3 or less.")
        .expect("parse universal gain-control clause");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("gain control of each noncommander creature with mana value 3 or less"),
        "expected compiled text to preserve universal gain-control wording, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_create_token_for_each_creature_that_died_this_turn() {
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
    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("for each creature that died this turn"),
        "expected compiled text to preserve died-this-turn token count, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_spoils_of_blood_uses_creatures_died_this_turn_count() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Spoils of Blood Variant")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Black]]))
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Create an X/X black Horror creature token, where X is the number of creatures that died this turn.",
        )
        .expect("Spoils of Blood text should parse");

    let spell_debug = format!("{:#?}", def.spell_effect);
    assert!(
        spell_debug.contains("CreaturesDiedThisTurn"),
        "expected Spoils of Blood to use the died-this-turn value, got {spell_debug}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains(
            "create an x/x black horror creature token, where x is the number of creatures that died this turn"
        ),
        "expected compiled Spoils of Blood text to compact the token PT rendering, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_corpseweft_scaled_exiled_this_way_token_power_toughness() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Corpseweft Variant")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "{1}{B}, Exile one or more creature cards from your graveyard: Create a tapped X/X black Zombie Horror creature token, where X is twice the number of cards exiled this way.",
        )
        .expect("Corpseweft-style activated ability should parse");

    let activated = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("expected activated ability");
    let set_base_pt = activated
        .effects
        .flattened_default_effects()
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::SetBasePowerToughnessEffect>())
        .expect("expected dynamic token base power/toughness setter");

    match (
        set_base_pt.power.unhinted(),
        set_base_pt.toughness.unhinted(),
    ) {
        (
            Value::CountScaled(power_filter, power_multiplier),
            Value::CountScaled(toughness_filter, toughness_multiplier),
        ) => {
            assert_eq!((*power_multiplier, *toughness_multiplier), (2, 2));
            assert_eq!(power_filter.zone, Some(Zone::Exile));
            assert_eq!(toughness_filter.zone, Some(Zone::Exile));
            assert!(
                !power_filter.tagged_constraints.is_empty()
                    && !toughness_filter.tagged_constraints.is_empty(),
                "expected exiled-this-way tag constraints, got {set_base_pt:#?}"
            );
        }
        other => panic!("expected scaled exiled-count P/T values, got {other:#?}"),
    }

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains(
            "create a tapped x/x black zombie horror creature token, where x is twice the number of cards exiled this way"
        ),
        "expected rendered text to compact the tapped dynamic token wording, got {rendered}"
    );
    assert!(
        !rendered.contains("0/0"),
        "dynamic token rendering should hide the temporary 0/0 shell, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_smugglers_share_counts_each_qualifying_opponent() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Smuggler's Share Variant")
        .card_types(vec![CardType::Enchantment])
        .parse_text("At the beginning of each end step, draw a card for each opponent who drew two or more cards this turn, then create a Treasure token for each opponent who had two or more lands enter the battlefield under their control this turn.")
        .expect("Smuggler's Share text should parse");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("BeginningOfEndStepTrigger { player: Any }")
            && debug.contains("ForPlayersEffect")
            && debug.contains("filter: Opponent")
            && debug.contains("DrawCardsEffect")
            && debug.contains("MaxCardsDrawnThisTurn")
            && debug.contains("IteratedPlayer")
            && debug.contains("CreateTokenEffect")
            && debug.contains("LandsEnteredBattlefieldThisTurn"),
        "expected Smuggler's Share to iterate qualifying opponents for both rewards, got {debug}"
    );

    let compiled = crate::compiled_text::canonical_compiled_lines(&def).join(" ");
    let compiled_lower = compiled.to_ascii_lowercase();
    assert!(
        (compiled_lower.contains("at the beginning of each end step")
            || compiled_lower.contains("at the beginning of each player's end step"))
            && compiled_lower
                .contains("draw a card for each opponent who drew two or more cards this turn")
            && compiled_lower.contains(
                "create a Treasure token for each opponent who had two or more lands enter under their control this turn"
                    .to_ascii_lowercase()
                    .as_str()
            ),
        "expected Smuggler's Share compiled text to preserve both qualifying-opponent clauses, got {compiled}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_trigger_attacks_with_subject_filter() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Attack Filter Probe")
        .card_types(vec![CardType::Enchantment])
        .parse_text("Whenever a creature you control attacks, put a +1/+1 counter on it.")
        .expect("parse filtered attacks trigger");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("AttacksTrigger"),
        "expected filtered attacks trigger matcher, got {debug}"
    );
    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("whenever a creature")
            && !joined.contains("whenever this creature attacks"),
        "expected trigger subject to remain filtered, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_trigger_deals_combat_damage_with_subject_filter() {
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
    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        (joined.contains("whenever a vampire you control deals combat damage to a player")
            || joined.contains(
                "whenever a vampire creature you control deals combat damage to a player"
            ))
            && !joined.contains("whenever this creature deals combat damage to a player"),
        "expected trigger subject to remain filtered, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_trigger_deals_combat_damage_to_you_preserves_recipient() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Combat Damage Recipient Probe")
        .card_types(vec![CardType::Enchantment])
        .parse_text("Whenever a creature deals combat damage to you, you gain 1 life.")
        .expect("parse combat-damage recipient trigger");

    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("whenever creature deals combat damage to you")
            || joined.contains("whenever a creature deals combat damage to you"),
        "expected trigger recipient to remain 'you', got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_trigger_this_blocks_filtered_creature() {
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
    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("whenever this creature blocks creature with flying"),
        "expected trigger to include blocked-object filter, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_this_blocks_or_becomes_blocked_by_filtered_creature_delayed_destroy() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Arrogant Bloodlord Probe")
        .card_types(vec![CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(4, 4))
        .parse_text(
            "Whenever this creature blocks or becomes blocked by a creature with power 1 or less, destroy this creature at end of combat.",
        )
        .expect("parse source blocks-or-becomes-blocked trigger with blocker filter");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("ThisBlocksObjectTrigger")
            && debug.contains("ThisBecomesBlockedByObjectTrigger"),
        "expected source-scoped blocks/becomes-blocked trigger pair, got {debug}"
    );
    assert!(
        debug.contains("ScheduleDelayedTriggerEffect") && debug.contains("EndOfCombatTrigger"),
        "expected delayed end-of-combat destroy payload, got {debug}"
    );
    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("blocks or becomes blocked by a creature with power 1 or less")
            && joined.contains("destroy this creature at end of combat"),
        "expected rendered trigger/effect to preserve blocker filter and delay, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_trigger_you_discard_filtered_card() {
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

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_trigger_opponent_discards_card() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Opponent Discard Trigger Probe")
        .card_types(vec![CardType::Enchantment])
        .parse_text("Whenever an opponent discards a card, that player loses 2 life.")
        .expect("parse opponent discard trigger");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("YouDiscardCardTrigger") && debug.contains("player: Opponent"),
        "expected opponent discard trigger matcher, got {debug}"
    );
    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("whenever an opponent discards a card"),
        "expected discard trigger wording in compiled text, got {joined}"
    );
}

pub(super) fn captain_howler_test_definition() -> crate::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(616_789), "Captain Howler, Sea Scourge")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Blue],
            vec![ManaSymbol::Red],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Shark, Subtype::Pirate])
        .power_toughness(PowerToughness::fixed(5, 4))
        .parse_text(
            "Ward—{2}, Pay 2 life.\n\
             Whenever you discard one or more cards, target creature gets +2/+0 until end of turn for each card discarded this way. Whenever that creature deals combat damage to a player this turn, you draw a card.",
        )
        .expect("Captain Howler, Sea Scourge should parse strictly")
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn captain_howler_sea_scourge_parses_strictly_and_renders_discard_count_pump() {
    let def = captain_howler_test_definition();
    let debug = format!("{:#?}", def.abilities);

    assert!(
        debug.contains("YouDiscardCardTrigger") && debug.contains("one_or_more: true"),
        "expected a one-or-more discard trigger for Captain Howler, got {debug}"
    );
    assert!(
        debug.contains("ModifyPowerToughnessForEachEffect")
            || debug.contains("ModifyPowerToughnessForEach"),
        "expected Captain Howler trigger to lower to a for-each pump effect, got {debug}"
    );
    assert!(
        debug.contains("EventValue") && debug.contains("Amount"),
        "expected cards-discarded-this-way count to use trigger event amount, got {debug}"
    );

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Whenever you discard one or more cards"),
        "expected one-or-more discard trigger wording, got {rendered}"
    );
    assert!(
        rendered.contains("target creature gets +2/+0 until end of turn")
            && rendered.contains("card")
            && rendered.contains("discarded this way"),
        "expected compiled text to cover the discarded-this-way pump clause, got {rendered}"
    );
    assert!(
        rendered.contains(
            "Whenever that creature deals combat damage to a player this turn, you draw a card"
        ),
        "expected compiled text to keep the delayed trigger tied to that creature, got {rendered}"
    );
}

#[test]
pub(super) fn captain_howler_sea_scourge_discard_batch_pumps_once_for_each_discarded_card() {
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let captain = game.create_object_from_definition(
        &captain_howler_test_definition(),
        alice,
        Zone::Battlefield,
    );
    let other_creature_def = CardDefinitionBuilder::new(CardId::new(), "Other Attacker")
        .card_types(vec![CardType::Creature])
        .build();
    let other_creature =
        game.create_object_from_definition(&other_creature_def, alice, Zone::Battlefield);
    let draw_card = CardDefinitionBuilder::new(CardId::new(), "Captain Draw Card")
        .card_types(vec![CardType::Creature])
        .build();
    game.create_object_from_definition(&draw_card, alice, Zone::Library);
    for idx in 0..2 {
        let discard_card = CardDefinitionBuilder::new(CardId::new(), format!("Discard Card {idx}"))
            .card_types(vec![CardType::Creature])
            .build();
        game.create_object_from_definition(&discard_card, alice, Zone::Hand);
    }

    let discard = crate::effects::DiscardEffect::new(2, PlayerFilter::You, false);
    let mut discard_dm = crate::decision::SelectFirstDecisionMaker;
    let mut discard_ctx = crate::effects::ExecutionContext::new(captain, alice, &mut discard_dm);
    let outcome = discard
        .execute(&mut game, &mut discard_ctx)
        .expect("discard effect should resolve");

    let mut trigger_queue = crate::triggers::TriggerQueue::new();
    for event in &outcome.events {
        for trigger in crate::triggers::check_triggers(&game, event) {
            if trigger.source == captain {
                trigger_queue.add(trigger);
            }
        }
    }
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "Captain Howler should trigger once for a two-card discard batch"
    );

    let mut target_dm = crate::decision::SelectFirstDecisionMaker;
    crate::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut trigger_queue, &mut target_dm)
        .expect("Captain Howler trigger should go on the stack");
    crate::game_loop::resolve_stack_entry_with(&mut game, &mut target_dm)
        .expect("Captain Howler trigger should resolve");

    assert_eq!(
        game.calculated_power(captain),
        Some(9),
        "Captain Howler should get +4/+0 from two cards discarded in one batch"
    );

    let other_combat = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::DamageEvent::with_cause(
            other_creature,
            crate::events::DamageTarget::Player(PlayerId::from_index(1)),
            1,
            true,
            crate::events::cause::EventCause::combat_damage(other_creature),
        ),
        crate::provenance::ProvNodeId::default(),
    );
    assert_eq!(
        crate::triggers::check_delayed_triggers(&mut game, &other_combat)
            .into_iter()
            .filter(|entry| entry.source == captain)
            .count(),
        0,
        "Captain Howler's delayed draw trigger should watch only the pumped creature"
    );

    let hand_before = game.player(alice).expect("Alice exists").hand.len();
    let captain_combat = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::DamageEvent::with_cause(
            captain,
            crate::events::DamageTarget::Player(PlayerId::from_index(1)),
            9,
            true,
            crate::events::cause::EventCause::combat_damage(captain),
        ),
        crate::provenance::ProvNodeId::default(),
    );
    let mut delayed_queue = crate::triggers::TriggerQueue::new();
    let delayed_triggers = crate::triggers::check_delayed_triggers(&mut game, &captain_combat);
    let delayed_count = delayed_triggers
        .iter()
        .filter(|entry| entry.source == captain)
        .count();
    for trigger in delayed_triggers
        .into_iter()
        .filter(|entry| entry.source == captain)
    {
        delayed_queue.add(trigger);
    }
    assert_eq!(
        delayed_count, 1,
        "Captain Howler's delayed trigger should fire when the pumped creature hits a player"
    );
    crate::game_loop::put_triggers_on_stack(&mut game, &mut delayed_queue)
        .expect("Captain Howler delayed trigger should go on the stack");
    crate::game_loop::resolve_stack_entry(&mut game)
        .expect("Captain Howler delayed trigger should resolve");
    assert_eq!(
        game.player(alice).expect("Alice exists").hand.len(),
        hand_before + 1,
        "Captain Howler's delayed trigger should draw a card"
    );
}

#[test]
pub(super) fn captain_howler_sea_scourge_does_not_trigger_when_no_cards_are_discarded() {
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let captain = game.create_object_from_definition(
        &captain_howler_test_definition(),
        alice,
        Zone::Battlefield,
    );

    let discard = crate::effects::DiscardEffect::new(0, PlayerFilter::You, false);
    let mut discard_dm = crate::decision::SelectFirstDecisionMaker;
    let mut discard_ctx = crate::effects::ExecutionContext::new(captain, alice, &mut discard_dm);
    let outcome = discard
        .execute(&mut game, &mut discard_ctx)
        .expect("zero-card discard effect should resolve");

    let mut captain_triggers = 0;
    for event in &outcome.events {
        captain_triggers += crate::triggers::check_triggers(&game, event)
            .into_iter()
            .filter(|trigger| trigger.source == captain)
            .count();
    }
    assert_eq!(
        captain_triggers, 0,
        "no discarded cards should mean no trigger"
    );
    assert_eq!(game.calculated_power(captain), Some(5));
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_trigger_opponent_plays_land() {
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
    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("whenever an opponent plays a land"),
        "expected land-play trigger wording in compiled text, got {joined}"
    );
    assert!(
        joined.contains("you may put a land card from your hand onto the battlefield"),
        "expected optional land deployment effect, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_trigger_tap_swamp_for_mana() {
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

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_trigger_tap_creature_for_mana() {
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

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_trigger_one_or_more_plus_one_counters_put_on_this_creature() {
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

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_trigger_a_plus_one_counter_put_on_this_creature() {
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

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_trigger_you_put_one_or_more_minus_one_counters_on_a_creature() {
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

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_swindlers_scheme_keeps_counter_target_on_triggering_spell_after_reveal() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Swindler's Scheme")
        .card_types(vec![CardType::Enchantment])
        .parse_text("Whenever an opponent casts a spell from their hand, you may reveal the top card of your library. If it shares a card type with that spell, counter that spell and that opponent may cast the revealed card without paying its mana cost.")
        .expect("parse Swindler's Scheme variant");

    let debug = format!("{:#?}", def.abilities);
    assert!(
        debug.contains("TagTriggeringObjectEffect"),
        "expected trigger tagging in compiled ability, got {debug}"
    );
    assert!(
        debug.contains("CounterEffect")
            && debug.contains("target: Tagged")
            && debug.contains("triggering"),
        "expected counter target to stay bound to the triggering spell, got {debug}"
    );
    assert!(
        debug.contains("CastTaggedEffect") && debug.contains("__sentence_helper_revealed_l0_s0_e0"),
        "expected revealed card follow-up cast to stay bound to the revealed card, got {debug}"
    );

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Whenever an opponent casts a spell")
            && rendered.contains("reveal the top card of your library")
            && rendered.contains("counter it")
            && rendered.contains("without paying its mana cost"),
        "expected oracle-like compiled text to preserve Swindler's Scheme wording, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_nest_of_scarabs_style_trigger_and_token_amount() {
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

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_damage_trigger_with_source_subject() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Source Subject Damage Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Whenever a source deals damage to you, put a filibuster counter on this creature.",
        )
        .expect("source-subject damage trigger should parse");

    let debug = format!("{:#?}", def.abilities);
    assert!(
        debug.contains("DealsDamageTrigger")
            && debug.contains("filter: ObjectFilter")
            && debug.contains("zone: None")
            && debug.contains("damaged_player: Some(")
            && debug.contains("You")
            && debug.contains("source_surface: Source")
            && debug.contains("PutCountersEffect")
            && debug.contains("filibuster"),
        "expected a generic source-damage trigger with filibuster counter effect, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_low_scoring_damage_source_card_regressions() {
    let opponent_source_cards = [
        (
            "Awaken the Sky Tyrant",
            CardType::Enchantment,
            "Whenever a source an opponent controls deals damage to you, sacrifice Awaken the Sky Tyrant. If you do, create a 5/5 red Dragon creature token with flying.",
        ),
        (
            "Farsight Mask",
            CardType::Artifact,
            "Whenever a source an opponent controls deals damage to you, if Farsight Mask is untapped, you may draw a card.",
        ),
        (
            "Michiko Konda, Truth Seeker",
            CardType::Creature,
            "Whenever a source an opponent controls deals damage to you, that player sacrifices a permanent.",
        ),
    ];

    for (name, card_type, text) in opponent_source_cards {
        let def = CardDefinitionBuilder::new(CardId::from_raw(1), name)
            .card_types(vec![card_type])
            .parse_text(text)
            .unwrap_or_else(|error| panic!("{name} should parse: {error:?}"));
        let debug = format!("{:#?}", def.abilities);
        assert!(debug.contains("DealsDamageTrigger"), "{name}: {debug}");
        assert!(debug.contains("zone: None"), "{name}: {debug}");
        assert!(
            debug.contains("controller: Some(") && debug.contains("Opponent"),
            "{name}: {debug}"
        );
        assert!(debug.contains("damaged_player: Some("), "{name}: {debug}");
        assert!(debug.contains("source_surface: Source"), "{name}: {debug}");
    }

    let name = "Tamanoa";
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), name)
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Whenever a noncreature source you control deals damage, you gain that much life.",
        )
        .unwrap_or_else(|error| panic!("{name} should parse: {error:?}"));
    let debug = format!("{:#?}", def.abilities);
    assert!(debug.contains("DealsDamageTrigger"), "{name}: {debug}");
    assert!(debug.contains("zone: None"), "{name}: {debug}");
    assert!(
        debug.contains("controller: Some(") && debug.contains("You"),
        "{name}: {debug}"
    );
    assert!(debug.contains("excluded_card_types"), "{name}: {debug}");
    assert!(debug.contains("Creature"), "{name}: {debug}");
    assert!(debug.contains("damaged_player: None"), "{name}: {debug}");
    assert!(debug.contains("source_surface: Source"), "{name}: {debug}");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_source_has_five_or_more_counters_condition() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Source Counter Threshold Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "At the beginning of your upkeep, put a filibuster counter on this creature. Then if this creature has five or more filibuster counters on it, you win the game.",
        )
        .expect("source counter threshold condition should parse");

    let debug = format!("{:#?}", def.abilities);
    assert!(
        debug.contains("SourceHasCounterAtLeast") && debug.contains("count: 5"),
        "expected five-or-more source counter threshold, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_trigger_unknown_non_source_subject_fails() {
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

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_player_subject_attack_trigger_uses_one_or_more_creature_filter() {
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

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_player_subject_attack_with_three_or_more_uses_thresholded_mode() {
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

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_player_subject_attack_with_exactly_two_uses_exact_total_mode() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Exactly Two Attack Subject Probe")
        .card_types(vec![CardType::Creature])
        .parse_text("When you attack with exactly two creatures, draw a card.")
        .expect("player-subject attack-with exact trigger should parse");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("AttacksTrigger")
            && debug.contains("one_or_more: true")
            && debug.contains("min_total_attackers: 2")
            && debug.contains("max_total_attackers: Some(2)")
            && debug.contains("controller: Some(You)"),
        "expected exact-total one-or-more attacks trigger for creatures you control, got {debug}"
    );

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Whenever you attack with exactly two creatures"),
        "expected exact attack-count trigger rendering, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_opponent_attacks_you_trigger_uses_one_or_more_mode() {
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

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_attack_life_loss_uses_iterated_defending_player_attack_filter() {
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

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_target_creature_you_control_fights_target_creature_you_dont_control() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Prey Upon Probe")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Target creature you control fights target creature you don't control.")
        .expect("parse fight clause");

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("FightEffect"),
        "expected fight effect in spell text, got {debug}"
    );
    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("fights"),
        "expected compiled fight text, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_target_creature_deals_damage_to_itself_equal_to_its_power() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Justice Strike Variant")
        .card_types(vec![CardType::Instant])
        .parse_text("Target creature deals damage to itself equal to its power.")
        .expect("parse one-sided self damage equal-power clause");

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("PowerOf"),
        "expected power-based dynamic damage amount, got {debug}"
    );
    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("deal") && joined.contains("power"),
        "expected compiled output to keep one-sided power damage semantics, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_target_creature_you_control_deals_damage_equal_to_its_power_to_target_creature()
 {
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
    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("deal") && joined.contains("target"),
        "expected one-sided targeted power damage rendering, got {joined}"
    );
    assert!(
        !joined.contains("fights"),
        "bite-style damage must not compile as fight, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_double_target_creatures_power_until_end_of_turn() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Mr. Orfeo Probe")
        .card_types(vec![CardType::Creature])
        .parse_text("Whenever you attack, double target creature's power until end of turn.")
        .expect("parse double-target-power trigger clause");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("PowerOf"),
        "expected dynamic power-based pump amount, got {debug}"
    );

    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("double target creature's power until end of turn"),
        "expected compiled output to preserve double-power semantics, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_named_source_double_power_preserves_possessive_surface() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Casey Jones, Asphalt Hooligan")
        .card_types(vec![CardType::Creature])
        .parse_text("{4}: Double Casey Jones's power until end of turn.")
        .expect("parse named-source double-power activation");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("ShortName(\"Casey Jones\")")
            && debug.contains("PowerOf")
            && debug.contains("target: Source"),
        "expected structured named-source metadata, got {debug}"
    );
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert_eq!(
        rendered,
        "{4}: Double Casey Jones's power until end of turn."
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_unleash_fury_current_oracle_wording() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Unleash Fury Probe")
        .card_types(vec![CardType::Instant])
        .parse_text("Double the power of target creature until end of turn.")
        .expect("parse current Unleash Fury wording");

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("PowerOf"),
        "expected dynamic power-based pump amount, got {debug}"
    );

    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("target creature")
            && joined.contains("power")
            && joined.contains("until end of turn"),
        "expected compiled output to preserve current double-power wording, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_double_power_and_toughness_of_each_creature_you_control() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Zopandrel Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "At the beginning of each combat, double the power and toughness of each creature you control until end of turn.",
        )
        .expect("parse double-power-and-toughness sweep");

    let debug = format!("{:?}", def);
    assert!(
        debug.contains("ForEachObject")
            && debug.contains("PowerOf")
            && debug.contains("ToughnessOf"),
        "expected per-creature dynamic double P/T effect, got {debug}"
    );

    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("at the beginning of each combat"),
        "expected trigger text to be preserved, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_triple_target_creatures_power_and_toughness_until_end_of_turn() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Tifa Probe")
        .card_types(vec![CardType::Instant])
        .parse_text("Triple target creature's power and toughness until end of turn.")
        .expect("parse triple-target-pt clause");

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("Scaled") && debug.contains("PowerOf") && debug.contains("ToughnessOf"),
        "expected dynamic scaled triple P/T modifier, got {debug}"
    );

    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("target creature")
            && (joined.contains("twice target creature's power")
                || joined.contains("twice its power")
                || joined.contains("target creature's power"))
            && joined.contains("until end of turn"),
        "expected compiled output to preserve triple-power semantics, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_triple_power_and_toughness_of_each_creature_you_control() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Triple Sweep Probe")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "At the beginning of each combat, triple the power and toughness of each creature you control until end of turn.",
        )
        .expect("parse triple-power-and-toughness sweep");

    let debug = format!("{:?}", def);
    assert!(
        debug.contains("ForEachObject")
            && debug.contains("Scaled")
            && debug.contains("PowerOf")
            && debug.contains("ToughnessOf"),
        "expected per-creature scaled triple P/T effect, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_double_target_creatures_power_and_toughness_until_end_of_turn() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Choose Your Weapon Probe")
        .card_types(vec![CardType::Instant])
        .parse_text("Double target creature's power and toughness until end of turn.")
        .expect("parse double-target-pt clause");

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("PowerOf") && debug.contains("ToughnessOf"),
        "expected dynamic double P/T modifier, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_double_this_creatures_power_and_toughness_until_end_of_turn() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Reckless Amplimancer Probe")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .parse_text("{4}{G}: Double this creature's power and toughness until end of turn.")
        .expect("parse double-self-pt activation");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("ModifyPowerToughness")
            && debug.contains("PowerOf")
            && debug.contains("ToughnessOf")
            && debug.contains("target: Source")
            && debug.contains("this creature"),
        "expected source-relative dynamic double P/T modifier, got {debug}"
    );

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("{4}{G}: Double this creature's power and toughness until end of turn"),
        "expected compact double-self P/T rendering, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_double_target_players_life_total() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Beacon Probe")
        .card_types(vec![CardType::Instant])
        .parse_text("Double target player's life total.")
        .expect("parse double target player's life total");

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("SetLifeTotal") && debug.contains("Scaled") && debug.contains("LifeTotal"),
        "expected scaled life-total setter, got {debug}"
    );

    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("double target player's life total"),
        "expected compact double-life rendering, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_double_your_life_total() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Revenge Probe")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Double your life total.")
        .expect("parse double your life total");

    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("double your life total"),
        "expected compact self double-life rendering, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_doubling_cube_oracle_text() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Doubling Cube Probe")
        .card_types(vec![CardType::Artifact])
        .parse_text("{3}, {T}: Double the amount of each type of unspent mana you have.")
        .expect("parse Doubling Cube activation");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("DoubleManaPoolEffect"),
        "expected mana-pool doubling executor, got {debug}"
    );

    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("double the amount of each type of unspent mana you have"),
        "expected compiled output to preserve mana-doubling semantics, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_reinforce_keyword_line_from_hand() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Reinforce Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Flying\nReinforce 2—{2}{W} ({2}{W}, Discard this card: Put two +1/+1 counters on target creature.)",
        )
        .expect("reinforce line should parse as a hand activated ability");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("reinforce 2") && rendered.contains("{2}{w}"),
        "expected reinforce activation to render as a safe keyword marker, got {rendered}"
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

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_do_not_replace_keyword_named_card_reference_in_enchanted_grant_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Vigilance")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Aura])
        .parse_text("Enchant creature\nEnchanted creature has vigilance. (Attacking doesn't cause it to tap.)")
        .expect("keyword-named aura grant line should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("enchanted creature has vigilance"),
        "expected aura grant to keep vigilance keyword, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_source_deals_damage_to_target_equal_to_number_of_filter() {
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

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_put_counter_then_it_deals_damage_equal_to_its_power() {
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

    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("put a +1/+1 counter on target creature you control"),
        "expected counter clause in compiled text, got {joined}"
    );
    assert!(
        joined.contains("deals damage equal to its power to target creature an opponent controls")
            || joined.contains("deals damage equal to its power to target opponent's creature"),
        "expected follow-up damage clause in compiled text, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_lyzolda_the_blood_witch_parses_sacrificed_creature_color_branches() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Lyzolda, the Blood Witch")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::Black],
            vec![ManaSymbol::Red],
        ]))
        .supertypes(vec![Supertype::Legendary])
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Human, Subtype::Cleric])
        .power_toughness(PowerToughness::fixed(3, 1))
        .parse_text(
            "{2}, Sacrifice a creature: Lyzolda deals 2 damage to any target if the sacrificed creature was red. Draw a card if the sacrificed creature was black.",
        )
        .expect("Lyzolda, the Blood Witch should parse strictly");

    let rendered = compiled_text_lines(&def).join(" ");
    assert_eq!(
        rendered,
        "{2}, Sacrifice a creature: Lyzolda deals 2 damage to any target if the sacrificed creature was red. Draw a card if the sacrificed creature was black."
    );

    let debug = format!("{def:#?}");
    assert!(
        debug.contains("sacrifice_cost_0")
            && debug.contains("TaggedObjectMatches")
            && debug.contains("DealDamageEffect")
            && debug.contains("DrawCardsEffect"),
        "expected Lyzolda to lower both sacrificed-creature color branches, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_exile_named_source_with_time_counters() {
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

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_all_hallows_eve_countdown_from_exile() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "All Hallow's Eve")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Exile All Hallow's Eve with two scream counters on it.\n\
             At the beginning of your upkeep, if this card is exiled with a scream counter on it, remove a scream counter from it. If there are no more scream counters on it, put it into your graveyard and each player returns all creature cards from their graveyard to the battlefield.",
        )
        .expect("parse All Hallow's Eve-style exiled countdown");

    let spell_debug = format!("{:?}", def.spell_effect);
    assert!(
        spell_debug.contains("MoveToZoneEffect")
            && spell_debug.contains("target: Source")
            && spell_debug.contains("scream"),
        "expected source exile with named scream counters, got {spell_debug}"
    );

    let ability_debug = format!("{:?}", def.abilities);
    assert!(
        ability_debug.contains("BeginningOfUpkeepTrigger")
            && ability_debug.contains("SourceIsInZone(Exile)")
            && ability_debug.contains("SourceHasCounterAtLeast")
            && ability_debug.contains("RemoveCountersEffect")
            && ability_debug.contains("SourceHasNoCounter")
            && ability_debug.contains("ReturnAllToBattlefieldEffect"),
        "expected exiled countdown trigger with counter removal and return branch, got {ability_debug}"
    );
    assert!(
        ability_debug.contains("functional_zones: [Exile]"),
        "countdown trigger should function from exile, got {ability_debug}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("exile all hallow's eve with two scream counters on it"),
        "expected compact source exile-with-counters text, got {rendered}"
    );
    assert!(
        rendered.contains("if this card is exiled with a scream counter on it")
            && rendered.contains("remove a scream counter from it")
            && rendered.contains("if it has no scream counters on it")
            && rendered.contains(
                "put this spell into your graveyard and each player returns all creature cards from their graveyard"
            ),
        "expected countdown and mass return rendering, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_keyword_marker_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Marker Keywords")
        .card_types(vec![CardType::Creature])
        .parse_text("Unleash\nPhasing")
        .expect("marker keyword line should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
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

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_splice_keyword_lines_preserve_typed_subject_cost_and_static_semantics() {
    for (line, expected_label) in [
        (
            "Splice onto Arcane {W} (As you cast an Arcane spell, you may reveal this card from your hand and pay its splice cost. If you do, add this card's effects to that spell.)",
            "Splice onto Arcane {W}",
        ),
        (
            "Splice onto instant or sorcery {3}{W} (As you cast an instant or sorcery spell, you may reveal this card from your hand and pay its splice cost. If you do, add this card's effects to that spell.)",
            "Splice onto instant or sorcery {3}{W}",
        ),
    ] {
        let def = CardDefinitionBuilder::new(CardId::new(), "Keyword Probe")
            .card_types(vec![CardType::Instant])
            .parse_text(line)
            .expect("typed splice keyword line should parse");

        assert!(
            def.alternative_casts.is_empty(),
            "splice must not become an AlternativeCastingMethod: {def:#?}"
        );
        let static_ability = def
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Static(static_ability)
                    if static_ability.id() == StaticAbilityId::Splice =>
                {
                    Some(static_ability)
                }
                _ => None,
            })
            .expect("splice should lower to a typed static ability");
        assert_eq!(static_ability.display(), expected_label);
        assert!(
            static_ability.splice_spec().is_some(),
            "splice must retain its typed quality and cost: {def:#?}"
        );
        assert!(
            !format!("{def:#?}").contains("KeywordFallbackText"),
            "splice must not survive as a fallback: {def:#?}"
        );
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_marker_keyword_with_parameter_keeps_parameter_in_render() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Fabricate Probe")
        .card_types(vec![CardType::Creature])
        .parse_text("Fabricate 1")
        .expect("fabricate keyword line should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("fabricate 1")
            || (rendered.contains("choose one") && rendered.contains("servo")),
        "expected fabricate parameter in render output, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_marker_keyword_with_cost_keeps_cost_in_render() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Dash Probe")
        .card_types(vec![CardType::Creature])
        .parse_text("Dash {2}{B}")
        .expect("dash keyword line should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
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

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn compiled_lines_render_zurgo_restriction_before_dash() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Zurgo Bellstriker Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("This creature can't block creatures with power 2 or greater.\nDash {1}{R}")
        .expect("zurgo-style text should parse");

    let lines = unprocessed_compiled_lines(&def);
    assert_eq!(
        lines.first().map(String::as_str),
        Some("This creature can't block creatures with power 2 or greater.")
    );
    assert_eq!(lines.get(1).map(String::as_str), Some("Dash {1}{R}"));
}
