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
use super::*;

#[test]
pub(super) fn parse_additional_combat_phase_followed_by_main_phase() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Extra Combat Variant")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Red]]))
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "After this main phase, there is an additional combat phase followed by an additional main phase.",
        )
        .expect("additional combat and main phase clause should parse");

    let effect = def
        .spell_effect
        .as_ref()
        .expect("spell effects")
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::AdditionalPhasesEffect>())
        .expect("additional phases effect");
    assert_eq!(
        effect.phases,
        vec![
            crate::effects::AdditionalPhase::Combat,
            crate::effects::AdditionalPhase::Main
        ]
    );

    let rendered = crate::compiled_text::compiled_text_lines(&def).join("\n");
    assert!(
        rendered.contains(
            "After this main phase, there is an additional combat phase followed by an additional main phase"
        ),
        "expected additional phase surface, got {rendered}"
    );
}

#[test]
pub(super) fn parse_full_throttle_two_additional_combats() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Full Throttle")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "After this main phase, there are two additional combat phases.\nAt the beginning of each combat this turn, untap all creatures that attacked this turn.",
        )
        .expect("Full Throttle should parse");

    let effect = def
        .spell_effect
        .as_ref()
        .expect("spell effects")
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::AdditionalPhasesEffect>())
        .expect("additional phases effect");
    assert_eq!(
        effect.phases,
        vec![
            crate::effects::AdditionalPhase::Combat,
            crate::effects::AdditionalPhase::Combat,
        ]
    );

    let rendered = crate::compiled_text::compiled_text_lines(&def).join("\n");
    assert!(
        rendered.contains("After this main phase, there are two additional combat phases"),
        "expected Full Throttle additional-combat surface, got {rendered}"
    );
}

#[test]
pub(super) fn last_night_together_strict_parser_and_compiled_text_regression() {
    assert_oracle_card_parses_strict("Last Night Together");

    let def = parse_oracle_card_definition("Last Night Together");
    let rendered = crate::compiled_text::compiled_text_lines(&def).join(" ");
    let debug = format!("{:#?}", def.spell_effect);

    assert!(
        rendered.contains("Choose two target creatures")
            && rendered.contains("Put two +1/+1 counters on each of them")
            && rendered
                .contains("They gain vigilance, indestructible, and haste until end of turn")
            && rendered.contains("After this main phase, there is an additional combat phase")
            && rendered.contains("Only the chosen creatures can attack during that combat phase"),
        "expected Last Night Together compiled text to preserve the full chosen-creature combat restriction, got {rendered}"
    );
    assert!(
        debug.contains("AdditionalPhasesEffect")
            && debug.contains("CantEffect")
            && debug.contains("EndOfCombat"),
        "expected Last Night Together to lower to additional combat plus end-of-combat attack restriction, got {debug}"
    );
}

#[test]
pub(super) fn last_night_together_runtime_limits_attackers_to_chosen_creatures_for_that_combat() {
    let def = parse_oracle_card_definition("Last Night Together");
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    game.turn.active_player = alice;
    game.turn.phase = crate::game_state::Phase::FirstMain;
    game.turn.step = None;

    let source = game.create_object_from_definition(&def, alice, Zone::Stack);
    let chosen_one = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::from_raw(71_001), "Chosen One")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build(),
        alice,
        Zone::Battlefield,
    );
    let chosen_two = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::from_raw(71_002), "Chosen Two")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build(),
        alice,
        Zone::Battlefield,
    );
    let unchosen = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::from_raw(71_003), "Unchosen Bear")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build(),
        alice,
        Zone::Battlefield,
    );
    for creature in [chosen_one, chosen_two, unchosen] {
        game.remove_summoning_sickness(creature);
        game.tap(creature);
    }

    let effects = def
        .spell_effect
        .as_ref()
        .expect("Last Night Together should have a spell effect")
        .flattened_default_effects();
    let target_spec = effects[0]
        .0
        .get_target_spec()
        .expect("Last Night Together should start with target selection")
        .clone();
    let mut ctx = crate::effects::ExecutionContext::new_default(source, alice)
        .with_targets(vec![
            crate::effects::ResolvedTarget::Object(chosen_one),
            crate::effects::ResolvedTarget::Object(chosen_two),
        ])
        .with_target_assignments(vec![crate::game_state::TargetAssignment {
            spec: target_spec,
            range: 0..2,
        }]);
    ctx.snapshot_targets(&game);

    for effect in effects {
        crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap_or_else(|err| {
            panic!("Last Night Together effect should resolve: {err:?}; effect={effect:?}")
        });
    }

    assert_eq!(
        game.turn_store.additional_phases,
        vec![crate::game_state::Phase::Combat],
        "Last Night Together should insert an additional combat phase"
    );
    for chosen in [chosen_one, chosen_two] {
        assert!(
            !game.is_tapped(chosen),
            "chosen creatures should be untapped"
        );
        assert_eq!(
            game.counter_count(chosen, crate::object::CounterType::PlusOnePlusOne),
            2,
            "chosen creatures should get two +1/+1 counters"
        );
        assert!(game.object_has_static_ability_id(chosen, StaticAbilityId::Vigilance));
        assert!(game.object_has_static_ability_id(chosen, StaticAbilityId::Indestructible));
        assert!(game.object_has_static_ability_id(chosen, StaticAbilityId::Haste));
        assert!(
            game.can_attack(chosen),
            "chosen creatures should be allowed to attack"
        );
    }
    assert!(
        !game.can_attack(unchosen),
        "unchosen creatures should be prohibited from attacking during that combat"
    );

    crate::turn::advance_phase(&mut game).expect("advance to the inserted combat phase");
    assert_eq!(game.turn.phase, crate::game_state::Phase::Combat);
    assert!(!game.can_attack(unchosen));

    let late_unchosen = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::from_raw(71_004), "Late Unchosen Bear")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build(),
        alice,
        Zone::Battlefield,
    );
    game.remove_summoning_sickness(late_unchosen);
    game.update_cant_effects();
    assert!(
        !game.can_attack(late_unchosen),
        "creatures that enter before that combat are still not chosen and cannot attack"
    );

    game.turn.step = None;
    crate::turn::advance_phase(&mut game).expect("advance out of the restricted combat phase");
    assert!(
        game.can_attack(unchosen),
        "the chosen-creatures-only restriction should expire after that combat phase"
    );
    assert!(
        game.can_attack(late_unchosen),
        "late unchosen creatures should be able to attack after the restricted combat ends"
    );
}

#[test]
pub(super) fn parse_must_be_blocked_each_combat_this_turn_if_able() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Must Be Blocked Variant")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Red],
            vec![ManaSymbol::Green],
        ]))
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(8, 4))
        .parse_text("This creature must be blocked each combat this turn if able.")
        .expect("each-combat must-be-blocked clause should parse");

    let rendered = crate::compiled_text::compiled_text_lines(&def).join("\n");
    assert!(
        rendered.contains("must be blocked this turn if able"),
        "expected must-be-blocked surface, got {rendered}"
    );
}

#[test]
pub(super) fn parse_suspect_designation_clauses() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Suspect Variant")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::White]]))
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(4, 3))
        .parse_text(
            "Flying\nWhen this creature enters, all suspected creatures are no longer suspected.\nWhen this creature dies, you gain 3 life and suspect up to one target creature an opponent controls.",
        )
        .expect("suspect clauses should parse");

    let rendered = crate::compiled_text::compiled_text_lines(&def).join("\n");
    let rendered_lower = rendered.to_ascii_lowercase();
    assert!(
        rendered_lower.contains("all suspected creatures are no longer suspected"),
        "expected clear-suspected surface, got {rendered}"
    );
    assert!(
        rendered_lower.contains("suspect up to one target creature an opponent controls"),
        "expected suspect-target surface, got {rendered}"
    );
}

#[test]
pub(super) fn parse_suspect_it_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Self Suspect Variant")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Black]]))
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 1))
        .parse_text("When this creature enters, suspect it.")
        .expect("suspect it should parse");

    let rendered = crate::compiled_text::compiled_text_lines(&def).join("\n");
    assert!(
        rendered.contains("suspect it"),
        "expected suspect-it surface, got {rendered}"
    );
}

#[test]
pub(super) fn parse_passive_goad_designation_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Passive Goad Variant")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Black]]))
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 1))
        .parse_text(
            "When one or more Faeries you control deal combat damage to a player, that player creates a 4/2 red Pirate creature token with \"This token can't block.\" The token is goaded for the rest of the game.",
        )
        .expect("passive goad clause should parse");

    let rendered = crate::compiled_text::compiled_text_lines(&def).join("\n");
    assert!(
        rendered.to_ascii_lowercase().contains("goad that creature"),
        "expected passive goad surface, got {rendered}"
    );
}

#[test]
pub(super) fn rayami_first_of_the_fallen_parses_and_renders_blood_counter_replacement() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Rayami, First of the Fallen")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::Black],
            vec![ManaSymbol::Green],
            vec![ManaSymbol::Blue],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Vampire])
        .power_toughness(PowerToughness::fixed(5, 4))
        .parse_text(
            "If a nontoken creature would die, exile that card with a blood counter on it instead.\nAs long as an exiled creature card with a blood counter on it has flying, this has flying. The same is true for first strike, double strike, deathtouch, haste, hexproof, indestructible, lifelink, menace, protection, reach, trample, and vigilance.",
        )
        .expect("Rayami, First of the Fallen oracle text should parse");

    let rendered = crate::compiled_text::compiled_text_lines(&def)
        .join("\n")
        .to_ascii_lowercase();
    assert!(
        rendered.contains(
            "if nontoken creature would die, exile it with a blood counter on it instead"
        ),
        "expected would-die replacement text, got {rendered}"
    );
}

#[test]
pub(super) fn absolute_virtue_renders_protection_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Absolute Virtue")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::White],
            vec![ManaSymbol::White],
        ]))
        .card_types(vec![CardType::Enchantment])
        .parse_text("You have protection from each of your opponents.")
        .expect("Absolute Virtue oracle text should parse");

    let rendered = crate::compiled_text::compiled_text_lines(&def)
        .join("\n")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("you have protection from each of your opponents"),
        "expected protection clause in compiled text, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn sorin_markov_strict_parse_regression_and_compiled_text_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Sorin Markov")
        .card_types(vec![CardType::Planeswalker])
        .subtypes(vec![Subtype::Sorin])
        .loyalty(4)
        .parse_text(
            "+2: Sorin Markov deals 2 damage to any target and you gain 2 life.\n−3: Target opponent's life total becomes 10.\n−7: You control target player during that player's next turn.",
        )
        .expect("Sorin Markov should parse strictly");

    let rendered = crate::compiled_text::compiled_text_lines(&def).join("\n");
    let rendered_lower = rendered.to_ascii_lowercase();
    assert!(
        rendered_lower.contains("control target player during")
            && rendered_lower.contains("next turn"),
        "expected control-target-player-next-turn wording in compiled output, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn sorin_markov_compiles_control_player_and_life_total_effects() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Sorin Markov")
        .card_types(vec![CardType::Planeswalker])
        .subtypes(vec![Subtype::Sorin])
        .loyalty(4)
        .parse_text(
            "+2: Sorin Markov deals 2 damage to any target and you gain 2 life.\n−3: Target opponent's life total becomes 10.\n−7: You control target player during that player's next turn.",
        )
        .expect("Sorin Markov should parse strictly");

    let mut has_set_life_to_ten = false;
    let mut has_control_player_next_turn = false;
    for ability in &def.abilities {
        let crate::ability::AbilityKind::Activated(activated) = &ability.kind else {
            continue;
        };
        for effect in activated.effects.flattened_default_effects() {
            if let Some(set_life) = effect.downcast_ref::<crate::effects::SetLifeTotalEffect>()
                && set_life.amount == crate::effect::Value::Fixed(10)
            {
                has_set_life_to_ten = true;
            }
            if let Some(control_player) =
                effect.downcast_ref::<crate::effects::ControlPlayerEffect>()
                && format!("{:?}", control_player.start)
                    .to_ascii_lowercase()
                    .contains("nextturn")
            {
                has_control_player_next_turn = true;
            }
        }
    }

    assert!(
        has_set_life_to_ten,
        "expected Sorin Markov to compile a set-life-total-to-10 effect"
    );
    assert!(
        has_control_player_next_turn,
        "expected Sorin Markov to compile a control-player-during-next-turn effect"
    );
}

#[test]
pub(super) fn eruth_tormented_prophet_parses_strictly_as_draw_replacement() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Eruth, Tormented Prophet")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::Blue],
            vec![ManaSymbol::Red],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Human, Subtype::Wizard])
        .power_toughness(PowerToughness::fixed(2, 4))
        .parse_text(
            "If you would draw a card, exile the top two cards of your library instead. You may play those cards this turn.",
        )
        .expect("Eruth, Tormented Prophet oracle text should parse");
    let static_ids = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect::<Vec<_>>();

    assert!(
        static_ids.contains(&StaticAbilityId::DrawReplacementExileTopAndPlay),
        "expected Eruth replacement static ability, got {static_ids:?}"
    );
}

#[test]
pub(super) fn eruth_tormented_prophet_compiled_text_keeps_replacement_and_play_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Eruth, Tormented Prophet")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::Blue],
            vec![ManaSymbol::Red],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Human, Subtype::Wizard])
        .power_toughness(PowerToughness::fixed(2, 4))
        .parse_text(
            "If you would draw a card, exile the top two cards of your library instead. You may play those cards this turn.",
        )
        .expect("Eruth, Tormented Prophet oracle text should parse");
    let rendered = unprocessed_compiled_lines(&def)
        .join("\n")
        .to_ascii_lowercase();

    assert!(
        rendered
            .contains("if you would draw a card, exile the top 2 cards of your library instead"),
        "expected replacement clause in compiled text, got {rendered}"
    );
    assert!(
        rendered.contains("you may play those cards this turn"),
        "expected play-permission clause in compiled text, got {rendered}"
    );
}

#[test]
pub(super) fn urabrask_heretic_praetor_strict_parser_and_compiled_text_regression() {
    assert_oracle_card_parses_strict("Urabrask, Heretic Praetor");
    let def = parse_oracle_card_definition("Urabrask, Heretic Praetor");
    let rendered = unprocessed_compiled_lines(&def).join("\n");
    let ability_debug = format!("{:#?}", def.abilities);

    assert!(
        ability_debug.contains("RegisterDrawReplacementEffect"),
        "Urabrask should lower the opponent upkeep ability to a draw replacement registration, got {ability_debug}"
    );
    assert!(
        rendered.contains("the next time they would draw a card this turn, instead they exile the top card of their library"),
        "Urabrask compiled text should preserve the next-draw instead clause, got {rendered}"
    );
    assert!(
        rendered.contains("At the beginning of your upkeep, exile the top card of your library"),
        "Urabrask compiled text should preserve its controller upkeep impulse draw trigger, got {rendered}"
    );
}

#[test]
pub(super) fn aspect_of_wolf_strict_parser_and_compiled_text_regression() {
    assert_oracle_card_parses_strict("Aspect of Wolf");
    let def = parse_oracle_card_definition("Aspect of Wolf");
    let rendered = compiled_text_lines(&def).join("\n");
    let ability_debug = format!("{:#?}", def.abilities);

    assert!(
        rendered.contains("Enchant creature")
            && rendered.contains(
                "Enchanted creature gets +X/+Y, where X is half the number of Forests you control, rounded down, and Y is half the number of Forests you control, rounded up."
            ),
        "Aspect of Wolf compiled text should preserve enchant creature and both half-rounded Forest-count clauses, got {rendered}"
    );
    assert!(
        ability_debug.contains("Dynamic")
            && ability_debug.contains("HalfRoundedDown")
            && ability_debug.contains("subtypes: [Forest]"),
        "Aspect of Wolf should structurally lower to dynamic half-rounded Forest-count anthem values, got {ability_debug}"
    );
}

#[test]
pub(super) fn aspect_of_wolf_updates_enchanted_creature_from_controller_forest_count() {
    let aspect = parse_oracle_card_definition("Aspect of Wolf");
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = crate::tests::test_helpers::setup_two_player_game();

    let creature = CardDefinitionBuilder::new(CardId::new(), "Aspect Bear")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let forest = CardDefinitionBuilder::new(CardId::new(), "Regression Forest")
        .card_types(vec![CardType::Land])
        .subtypes(vec![Subtype::Forest])
        .build();

    let creature_id = game.create_object_from_definition(&creature, alice, Zone::Battlefield);
    let aspect_id = game.create_object_from_definition(&aspect, alice, Zone::Battlefield);
    assert!(
        game.attach_object_to_target(
            aspect_id,
            crate::object::AttachmentTarget::Object(creature_id),
        ),
        "Aspect of Wolf should attach to the regression creature"
    );

    game.create_object_from_definition(&forest, bob, Zone::Battlefield);
    assert_eq!(game.calculated_power(creature_id), Some(2));
    assert_eq!(game.calculated_toughness(creature_id), Some(2));

    game.create_object_from_definition(&forest, alice, Zone::Battlefield);
    assert_eq!(game.calculated_power(creature_id), Some(2));
    assert_eq!(game.calculated_toughness(creature_id), Some(3));

    game.create_object_from_definition(&forest, alice, Zone::Battlefield);
    assert_eq!(game.calculated_power(creature_id), Some(3));
    assert_eq!(game.calculated_toughness(creature_id), Some(3));

    game.create_object_from_definition(&forest, alice, Zone::Battlefield);
    assert_eq!(game.calculated_power(creature_id), Some(3));
    assert_eq!(game.calculated_toughness(creature_id), Some(4));
}

pub(super) fn add_named_library_card(
    game: &mut crate::game_state::GameState,
    player: PlayerId,
    name: &str,
) {
    let card = CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Instant])
        .build();
    game.create_object_from_definition(&card, player, Zone::Library);
}

pub(super) fn urabrask_triggered_effects<'a>(
    def: &'a crate::cards::CardDefinition,
    needle: &str,
) -> &'a crate::resolution::ResolutionProgram {
    def.abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered)
                if format!("{:#?}", triggered.effects).contains(needle) =>
            {
                Some(&triggered.effects)
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing Urabrask triggered ability containing {needle}"))
}

#[test]
pub(super) fn urabrask_controller_upkeep_exiles_top_card_and_grants_play_permission() {
    let def = parse_oracle_card_definition("Urabrask, Heretic Praetor");
    let alice = PlayerId::from_index(0);
    let mut game = crate::game_state::GameState::new(vec!["Alice".to_string()], 20);
    let source = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    add_named_library_card(&mut game, alice, "Alice Urabrask Top Card");
    let effects = urabrask_triggered_effects(&def, "GrantPlayTaggedEffect");
    let mut ctx = crate::effects::ExecutionContext::new_default(source, alice);

    for effect in effects.flattened_default_effects() {
        crate::effects::execute_effect(&mut game, effect, &mut ctx)
            .expect("Urabrask controller upkeep effect should resolve");
    }

    let alice_state = game.player(alice).expect("Alice exists");
    assert_eq!(alice_state.library.len(), 0);
    assert_eq!(alice_state.hand.len(), 0);
    assert_eq!(game.exile.len(), 1);
}

#[test]
pub(super) fn urabrask_opponent_draw_before_upkeep_replacement_is_not_replaced() {
    let def = parse_oracle_card_definition("Urabrask, Heretic Praetor");
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let source = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    add_named_library_card(&mut game, bob, "Bob Normal Draw");
    let mut ctx = crate::effects::ExecutionContext::new_default(source, alice);

    let outcome = DrawCardsEffect::new(1, PlayerFilter::Specific(bob))
        .execute(&mut game, &mut ctx)
        .expect("Bob draw before Urabrask replacement should resolve normally");

    assert_eq!(outcome.count_or_zero(), 1);
    let bob_state = game.player(bob).expect("Bob exists");
    assert_eq!(bob_state.library.len(), 0);
    assert_eq!(bob_state.hand.len(), 1);
    assert_eq!(game.exile.len(), 0);
}

#[test]
pub(super) fn urabrask_opponent_upkeep_registers_next_draw_replacement() {
    let def = parse_oracle_card_definition("Urabrask, Heretic Praetor");
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let source = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    add_named_library_card(&mut game, bob, "Bob Urabrask Exiled Card");
    add_named_library_card(&mut game, bob, "Bob Normal Followup Draw");
    let effects = urabrask_triggered_effects(&def, "RegisterDrawReplacementEffect");
    let mut ctx = crate::effects::ExecutionContext::new_default(source, alice);
    ctx.iteration.iterated_player = Some(bob);

    for effect in effects.flattened_default_effects() {
        crate::effects::execute_effect(&mut game, effect, &mut ctx)
            .expect("Urabrask opponent upkeep should register a draw replacement");
    }
    assert!(
        game.effect_store
            .replacement_effects
            .effects()
            .iter()
            .any(|replacement| {
                replacement.source == source
                    && matches!(
                        replacement.replacement,
                        crate::replacement::ReplacementAction::Instead(_)
                    )
            }),
        "Urabrask should register a one-shot draw replacement from its opponent upkeep trigger"
    );

    let outcome = DrawCardsEffect::new(1, PlayerFilter::Specific(bob))
        .execute(&mut game, &mut ctx)
        .expect("Bob draw should be replaced by Urabrask");

    assert_eq!(outcome.count_or_zero(), 0);
    let bob_state = game.player(bob).expect("Bob exists");
    assert_eq!(bob_state.library.len(), 1);
    assert_eq!(bob_state.hand.len(), 0);
    assert_eq!(game.exile.len(), 1);

    let followup_outcome = DrawCardsEffect::new(1, PlayerFilter::Specific(bob))
        .execute(&mut game, &mut ctx)
        .expect("Bob's next draw after the replaced draw should resolve normally");

    assert_eq!(followup_outcome.count_or_zero(), 1);
    let bob_state = game.player(bob).expect("Bob exists");
    assert_eq!(bob_state.library.len(), 0);
    assert_eq!(bob_state.hand.len(), 1);
    assert_eq!(game.exile.len(), 1);
}

#[test]
pub(super) fn parse_oracle_trove_tracker_regression_compiles_with_encore_keyword_line() {
    let def = parse_oracle_card_definition("Trove Tracker");
    let rendered = unprocessed_compiled_lines(&def).join("\n");

    assert!(
        rendered.contains("Encore {5}{U}{U}"),
        "expected Trove Tracker to preserve encore keyword cost line, got {rendered}"
    );
    assert!(
        rendered.contains("When this creature dies, draw a card."),
        "expected Trove Tracker death trigger to compile, got {rendered}"
    );
}

#[test]
pub(super) fn parse_oracle_knowledge_exploitation_supports_prowl_keyword_line() {
    let def = parse_oracle_card_definition("Knowledge Exploitation");
    assert!(
        !def.alternative_casts.is_empty(),
        "Knowledge Exploitation should compile with a prowl alternative cost"
    );

    let has_prowl = def.alternative_casts.iter().any(|method| {
        matches!(
            method.cast_condition(),
            Some(crate::static_abilities::ThisSpellCostCondition::YouDealtCombatDamageToPlayerWithSubtypeThisTurn(
                Subtype::Rogue
            ))
        )
    });
    assert!(
        has_prowl,
        "Knowledge Exploitation should encode Prowl with Rogue-combat-damage condition"
    );
}

#[test]
pub(super) fn knowledge_exploitation_compiled_text_keeps_prowl_and_target_opponent_library_clause()
{
    let def = parse_oracle_card_definition("Knowledge Exploitation");
    let rendered = unprocessed_compiled_lines(&def).join("\n");

    assert!(
        rendered.contains("Prowl {3}{U}"),
        "expected Knowledge Exploitation to render the prowl keyword cost, got {rendered}"
    );
    assert!(
        rendered.contains(
            "Search target opponent's library for an instant or sorcery card. You may cast that card without paying its mana cost. Then that player shuffles"
        ),
        "expected Knowledge Exploitation to keep search-target clause, got {rendered}"
    );
}

#[test]
pub(super) fn parse_oracle_loot_the_key_to_everything_strict_regression() {
    let def = parse_oracle_card_definition("Loot, the Key to Everything");
    let debug = format!("{:?}", def.abilities);

    assert!(
        debug.contains("CardTypesAmong"),
        "expected Loot trigger to bind X to CardTypesAmong, got {debug}"
    );
    assert!(
        debug.contains("controller: Some(You)")
            && debug.contains("excluded_card_types: [Land]")
            && debug.contains("other: true"),
        "expected Loot trigger to scope to other nonland permanents you control, got {debug}"
    );
}

#[test]
pub(super) fn loot_the_key_to_everything_compiled_text_keeps_card_types_among_marker() {
    let def = parse_oracle_card_definition("Loot, the Key to Everything");
    let rendered = unprocessed_compiled_lines(&def)
        .join("\n")
        .to_ascii_lowercase();

    assert!(
        rendered.contains(
            "where x is the number of card types among other nonland permanents you control"
        ),
        "expected Loot compiled text to preserve card-types-among clause, got {rendered}"
    );
    assert!(
        rendered.contains("you may play those cards this turn"),
        "expected Loot compiled text to preserve temporary play permission, got {rendered}"
    );
}

#[test]
pub(super) fn loot_the_key_to_everything_runtime_count_uses_distinct_card_types_among_other_nonlands()
 {
    let def = parse_oracle_card_definition("Loot, the Key to Everything");
    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("CardTypesAmong"),
        "expected Loot upkeep X value to use CardTypesAmong, got {debug}"
    );

    let mut game = crate::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = crate::ids::PlayerId::from_index(0);
    let source_id = game.create_object_from_definition(&def, alice, crate::zone::Zone::Battlefield);

    let sol_ring = parse_oracle_card_definition("Sol Ring");
    let llanowar_elves = parse_oracle_card_definition("Llanowar Elves");
    game.create_object_from_definition(&sol_ring, alice, crate::zone::Zone::Battlefield);
    game.create_object_from_definition(&llanowar_elves, alice, crate::zone::Zone::Battlefield);

    let count_value = crate::effect::Value::CardTypesAmong(crate::target::ObjectFilter {
        zone: Some(crate::zone::Zone::Battlefield),
        controller: Some(crate::target::PlayerFilter::You),
        excluded_card_types: vec![crate::types::CardType::Land],
        other: true,
        ..Default::default()
    });
    let ctx = crate::effects::ExecutionContext::new_default(source_id, alice);
    let resolved = crate::effects::helpers::resolve_value(&game, &count_value, &ctx)
        .expect("Loot upkeep X value should resolve");
    assert_eq!(
        resolved, 2,
        "artifact + creature should produce two card types"
    );
}

#[test]
pub(super) fn parse_oracle_overpowering_attack_supports_freerunning_keyword_line() {
    let def = parse_oracle_card_definition("Overpowering Attack");
    assert!(
        !def.alternative_casts.is_empty(),
        "Overpowering Attack should compile with a freerunning alternative cost"
    );

    let has_freerunning = def.alternative_casts.iter().any(|method| {
        method.name().eq_ignore_ascii_case("Freerunning")
            && matches!(
                method.cast_condition(),
                Some(
                    crate::static_abilities::ThisSpellCostCondition::YouDealtCombatDamageToPlayerWithSubtypeOrCommanderThisTurn(
                        Subtype::Assassin
                    )
                )
            )
    });
    assert!(
        has_freerunning,
        "Overpowering Attack should encode Freerunning with Assassin-or-commander combat damage condition"
    );
}

#[test]
pub(super) fn overpowering_attack_compiled_text_keeps_freerunning_and_extra_combat_clause() {
    let def = parse_oracle_card_definition("Overpowering Attack");
    let rendered = unprocessed_compiled_lines(&def).join("\n");

    assert!(
        rendered.contains("Freerunning {2}{R}"),
        "expected Overpowering Attack to render the freerunning keyword cost, got {rendered}"
    );
    assert!(
        rendered.contains(
            "After this main phase, there is an additional combat phase followed by an additional main phase"
        ),
        "expected Overpowering Attack to keep additional combat/main phase clause, got {rendered}"
    );
}

#[test]
pub(super) fn loot_the_key_to_everything_runtime_count_zero_without_other_nonlands() {
    let def = parse_oracle_card_definition("Loot, the Key to Everything");
    let mut game = crate::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = crate::ids::PlayerId::from_index(0);
    let source_id = game.create_object_from_definition(&def, alice, crate::zone::Zone::Battlefield);

    let count_value = crate::effect::Value::CardTypesAmong(crate::target::ObjectFilter {
        zone: Some(crate::zone::Zone::Battlefield),
        controller: Some(crate::target::PlayerFilter::You),
        excluded_card_types: vec![crate::types::CardType::Land],
        other: true,
        ..Default::default()
    });
    let ctx = crate::effects::ExecutionContext::new_default(source_id, alice);
    let resolved = crate::effects::helpers::resolve_value(&game, &count_value, &ctx)
        .expect("Loot upkeep X value should resolve");
    assert_eq!(
        resolved, 0,
        "no other nonland permanents should produce X=0"
    );
}

#[test]
pub(super) fn loot_the_key_to_everything_runtime_count_ignores_lands_and_opponents_permanents() {
    let def = parse_oracle_card_definition("Loot, the Key to Everything");
    let mut game = crate::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = crate::ids::PlayerId::from_index(0);
    let bob = crate::ids::PlayerId::from_index(1);
    let source_id = game.create_object_from_definition(&def, alice, crate::zone::Zone::Battlefield);

    let forest = parse_oracle_card_definition("Forest");
    let shivan_dragon = parse_oracle_card_definition("Shivan Dragon");
    game.create_object_from_definition(&forest, alice, crate::zone::Zone::Battlefield);
    game.create_object_from_definition(&shivan_dragon, bob, crate::zone::Zone::Battlefield);

    let count_value = crate::effect::Value::CardTypesAmong(crate::target::ObjectFilter {
        zone: Some(crate::zone::Zone::Battlefield),
        controller: Some(crate::target::PlayerFilter::You),
        excluded_card_types: vec![crate::types::CardType::Land],
        other: true,
        ..Default::default()
    });
    let ctx = crate::effects::ExecutionContext::new_default(source_id, alice);
    let resolved = crate::effects::helpers::resolve_value(&game, &count_value, &ctx)
        .expect("Loot upkeep X value should resolve");
    assert_eq!(
        resolved, 0,
        "lands and opponents' permanents should not contribute to Loot's X"
    );
}

#[test]
pub(super) fn loot_the_key_to_everything_runtime_grants_play_permission_until_end_of_turn() {
    let def = parse_oracle_card_definition("Loot, the Key to Everything");
    let debug = format!("{:?}", def.abilities);

    assert!(
        debug.contains("GrantPlayTaggedEffect"),
        "expected Loot upkeep trigger to grant play permission to exiled cards, got {debug}"
    );
    assert!(
        debug.contains("duration: UntilEndOfTurn"),
        "expected Loot play permission duration to end at end of turn, got {debug}"
    );
    assert!(
        debug.contains("allow_land: true"),
        "expected Loot play permission to allow lands as well as spells, got {debug}"
    );
}

#[test]
pub(super) fn occult_epiphany_strict_parser_and_compiled_text_regression() {
    let def = parse_oracle_card_definition("Occult Epiphany");
    let rendered = unprocessed_compiled_lines(&def).join("\n");
    let debug = format!("{:?}", def.spell_effect);

    assert!(
        debug.contains("CardTypesAmong"),
        "Occult Epiphany should count card types among discarded cards structurally, got {debug}"
    );
    assert!(
        debug.contains("discarded_0"),
        "Occult Epiphany should bind the token count to cards discarded this way, got {debug}"
    );
    assert_eq!(
        rendered,
        "Draw X cards, then discard X cards. Create a 1/1 white Spirit creature token with flying for each card type among cards discarded this way.",
        "Occult Epiphany should render the full oracle text"
    );
}

pub(super) fn create_occult_epiphany_test_card(
    game: &mut crate::GameState,
    name: &str,
    owner: PlayerId,
    card_types: Vec<CardType>,
    zone: Zone,
) -> ObjectId {
    let card = CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(card_types)
        .build();
    game.create_object_from_definition(&card, owner, zone)
}

pub(super) fn resolve_occult_epiphany_with_x(
    game: &mut crate::GameState,
    controller: PlayerId,
    x: u32,
) {
    let def = parse_oracle_card_definition("Occult Epiphany");
    let spell_id = game.create_object_from_definition(&def, controller, Zone::Stack);
    game.object_mut(spell_id)
        .expect("Occult Epiphany spell exists")
        .x_value = Some(x);
    let stable_id = game
        .object(spell_id)
        .expect("Occult Epiphany spell exists")
        .stable_id;
    game.push_to_stack(
        crate::game_state::StackEntry::new(spell_id, controller)
            .with_x(x)
            .with_source_info(stable_id, "Occult Epiphany".to_string()),
    );

    let mut dm = crate::decision::SelectFirstDecisionMaker;
    crate::game_loop::resolve_stack_entry_with(game, &mut dm)
        .expect("Occult Epiphany should resolve");
}

#[test]
pub(super) fn occult_epiphany_runtime_creates_spirits_for_distinct_discarded_card_types() {
    let mut game = crate::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    create_occult_epiphany_test_card(
        &mut game,
        "Existing Enchantment Card",
        alice,
        vec![CardType::Enchantment],
        Zone::Graveyard,
    );
    create_occult_epiphany_test_card(
        &mut game,
        "Artifact Creature Card",
        alice,
        vec![CardType::Artifact, CardType::Creature],
        Zone::Library,
    );
    create_occult_epiphany_test_card(
        &mut game,
        "Duplicate Artifact Card",
        alice,
        vec![CardType::Artifact],
        Zone::Library,
    );
    create_occult_epiphany_test_card(
        &mut game,
        "Instant Card",
        alice,
        vec![CardType::Instant],
        Zone::Library,
    );

    resolve_occult_epiphany_with_x(&mut game, alice, 3);

    assert_eq!(
        game.player(alice).expect("Alice exists").hand.len(),
        0,
        "Occult Epiphany should discard the X cards it drew"
    );
    assert_eq!(
        game.player(alice).expect("Alice exists").graveyard.len(),
        5,
        "the existing graveyard card, three discarded cards, and Occult Epiphany should end in Alice's graveyard"
    );

    let spirit_ids = game
        .objects_in_zone(Zone::Battlefield)
        .into_iter()
        .filter(|id| {
            let Some(obj) = game.object(*id) else {
                return false;
            };
            game.controller_of(obj) == alice
                && obj.card_types.contains(&CardType::Creature)
                && obj.subtypes.contains(&Subtype::Spirit)
        })
        .collect::<Vec<_>>();
    assert_eq!(
        spirit_ids.len(),
        3,
        "artifact, creature, and instant among discarded cards should create three Spirits; the existing enchantment and duplicate artifact should not add tokens"
    );

    for spirit_id in spirit_ids {
        let spirit = game.object(spirit_id).expect("Spirit token exists");
        assert_eq!(game.calculated_power(spirit_id), Some(1));
        assert_eq!(game.calculated_toughness(spirit_id), Some(1));
        assert_eq!(
            game.current_colors(spirit_id),
            Some(crate::color::ColorSet::WHITE)
        );
        assert!(
            spirit.has_static_ability_id(StaticAbilityId::Flying),
            "Occult Epiphany Spirit tokens should have flying"
        );
    }
}

#[test]
pub(super) fn occult_epiphany_runtime_x_zero_draws_discards_and_creates_no_tokens() {
    let mut game = crate::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    create_occult_epiphany_test_card(
        &mut game,
        "Library Card",
        alice,
        vec![CardType::Sorcery],
        Zone::Library,
    );

    resolve_occult_epiphany_with_x(&mut game, alice, 0);

    assert_eq!(
        game.player(alice).expect("Alice exists").hand.len(),
        0,
        "X=0 should draw no cards"
    );
    assert_eq!(
        game.player(alice).expect("Alice exists").library.len(),
        1,
        "X=0 should leave the library unchanged"
    );
    assert!(
        game.objects_in_zone(Zone::Battlefield)
            .into_iter()
            .filter_map(|id| game.object(id))
            .all(|obj| !obj.subtypes.contains(&Subtype::Spirit)),
        "X=0 should create no Spirit tokens"
    );
}

pub(super) fn fatespinner_triggered_ability(
    def: &CardDefinition,
) -> crate::ability::TriggeredAbility {
    def.abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered)
                if triggered
                    .trigger
                    .display()
                    .contains("each opponent's upkeep") =>
            {
                Some(triggered.clone())
            }
            _ => None,
        })
        .expect("Fatespinner should compile an opponent-upkeep trigger")
}

pub(super) struct ChooseFatespinnerPhase(&'static str);

impl crate::decision::DecisionMaker for ChooseFatespinnerPhase {
    fn decide_options(
        &mut self,
        _game: &crate::game_state::GameState,
        ctx: &crate::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        ctx.options
            .iter()
            .find(|option| option.description.eq_ignore_ascii_case(self.0))
            .map(|option| vec![option.index])
            .unwrap_or_else(|| vec![0])
    }
}

pub(super) fn resolve_fatespinner_upkeep_choice(
    choice: &'static str,
) -> (crate::game_state::GameState, PlayerId) {
    use crate::effects::{ExecutionContext, execute_effect};

    let def = parse_oracle_card_definition("Fatespinner");
    let triggered = fatespinner_triggered_ability(&def);
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let fatespinner_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    game.turn.active_player = bob;
    game.turn.phase = crate::game_state::Phase::Beginning;
    game.turn.step = Some(crate::game_state::Step::Upkeep);

    let event = crate::events::RawEvent::new(
        crate::events::phase::BeginningOfUpkeepEvent::new(bob),
        crate::provenance::ProvNodeId::default(),
    );
    let mut dm = ChooseFatespinnerPhase(choice);
    let mut ctx =
        ExecutionContext::new(fatespinner_id, alice, &mut dm).with_triggering_event(event);
    for effect in &triggered.effects {
        execute_effect(&mut game, effect, &mut ctx).expect("Fatespinner trigger should resolve");
    }

    (game, bob)
}

#[test]
pub(super) fn fatespinner_oracle_parses_strictly_and_renders_choice_clause() {
    assert_oracle_card_parses_strict("Fatespinner");
    let def = parse_oracle_card_definition("Fatespinner");
    let rendered = unprocessed_compiled_lines(&def).join("\n");

    assert_eq!(
        rendered,
        "At the beginning of each opponent's upkeep, that player chooses draw step, main phase, or combat phase. The player skips each instance of the chosen step or phase this turn."
    );
}

#[test]
pub(super) fn fatespinner_triggers_only_on_opponents_upkeep() {
    let def = parse_oracle_card_definition("Fatespinner");
    let triggered = fatespinner_triggered_ability(&def);
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let fatespinner_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let ctx = crate::triggers::TriggerContext::for_source(fatespinner_id, alice, &game);
    let controller_upkeep = crate::events::RawEvent::new(
        crate::events::phase::BeginningOfUpkeepEvent::new(alice),
        crate::provenance::ProvNodeId::default(),
    );
    let opponent_upkeep = crate::events::RawEvent::new(
        crate::events::phase::BeginningOfUpkeepEvent::new(bob),
        crate::provenance::ProvNodeId::default(),
    );

    assert!(
        !triggered.trigger.matches(&controller_upkeep, &ctx),
        "Fatespinner should not trigger on its controller's upkeep"
    );
    assert!(
        triggered.trigger.matches(&opponent_upkeep, &ctx),
        "Fatespinner should trigger on an opponent's upkeep"
    );
}

#[test]
pub(super) fn fatespinner_draw_step_choice_skips_only_that_players_draw_step() {
    let (game, bob) = resolve_fatespinner_upkeep_choice("draw step");

    assert!(game.turn_store.skip_next_draw_step.contains(&bob));
    assert!(!game.turn_store.skip_current_turn_main_phases.contains(&bob));
    assert!(!game.turn_store.skip_next_combat_phases.contains(&bob));
}

#[test]
pub(super) fn fatespinner_main_phase_choice_skips_each_remaining_main_phase_this_turn() {
    let (mut game, bob) = resolve_fatespinner_upkeep_choice("main phase");

    assert!(game.turn_store.skip_current_turn_main_phases.contains(&bob));
    assert!(!game.turn_store.skip_next_draw_step.contains(&bob));
    assert!(!game.turn_store.skip_next_combat_phases.contains(&bob));

    crate::turn::advance_phase(&mut game).expect("first main phase should be skipped");
    assert_eq!(game.turn.phase, crate::game_state::Phase::Combat);
    crate::turn::advance_phase(&mut game).expect("second main phase should be skipped");
    assert_eq!(game.turn.phase, crate::game_state::Phase::Ending);
}

#[test]
pub(super) fn fatespinner_combat_phase_choice_skips_only_that_players_combat_phase_this_turn() {
    let (mut game, bob) = resolve_fatespinner_upkeep_choice("combat phase");

    assert!(
        game.turn_store
            .skip_current_turn_combat_phases
            .contains(&bob)
    );
    assert!(!game.turn_store.skip_next_combat_phases.contains(&bob));
    assert!(!game.turn_store.skip_next_draw_step.contains(&bob));
    assert!(!game.turn_store.skip_current_turn_main_phases.contains(&bob));

    crate::turn::advance_phase(&mut game).expect("first main phase should happen normally");
    assert_eq!(game.turn.phase, crate::game_state::Phase::FirstMain);
    crate::turn::advance_phase(&mut game).expect("combat phase should be skipped");
    assert_eq!(game.turn.phase, crate::game_state::Phase::NextMain);

    game.turn_store
        .additional_phases
        .push(crate::game_state::Phase::Combat);
    crate::turn::advance_phase(&mut game).expect("additional combat phase should be skipped");
    assert_eq!(game.turn.phase, crate::game_state::Phase::NextMain);
}

#[test]
pub(super) fn pardic_firecat_strict_parser_and_compiled_text_regression() {
    assert_oracle_card_parses_strict("Pardic Firecat");
    let def = parse_oracle_card_definition("Pardic Firecat");
    let rendered = unprocessed_compiled_lines(&def).join("\n");

    let count_as_ability = def
        .abilities
        .iter()
        .find(|ability| {
            matches!(
                &ability.kind,
                AbilityKind::Static(static_ability)
                    if static_ability.id() == StaticAbilityId::CountAsCardNamedForSpellEffect
            )
        })
        .expect("Pardic Firecat should compile its Flame Burst graveyard count-as ability");

    assert!(
        count_as_ability.functions_in(&Zone::Graveyard),
        "Pardic Firecat's count-as ability should function in graveyards"
    );
    assert!(
        !count_as_ability.functions_in(&Zone::Battlefield),
        "Pardic Firecat's count-as ability should not function on the battlefield"
    );
    assert!(
        rendered.contains(
            "If this card is in a graveyard, effects from spells named Flame Burst count it as a card named Flame Burst."
        ),
        "expected Pardic Firecat compiled text to preserve the Flame Burst count-as clause, got {rendered}"
    );
}

pub(super) fn resolve_flame_burst_with_pardic_firecat_in_zones(
    flame_burst_source_zone: Zone,
    pardic_zone: Zone,
) -> i32 {
    let flame_burst = parse_oracle_card_definition("Flame Burst");
    let pardic_firecat = parse_oracle_card_definition("Pardic Firecat");
    let mut game = crate::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    game.create_object_from_definition(&flame_burst, alice, Zone::Graveyard);
    game.create_object_from_definition(&pardic_firecat, alice, pardic_zone);
    let source = game.create_object_from_definition(&flame_burst, alice, flame_burst_source_zone);

    let mut dm = crate::decision::AutoPassDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut dm)
        .with_targets(vec![crate::effects::ResolvedTarget::Player(bob)]);
    for effect in flame_burst
        .spell_effect
        .as_ref()
        .expect("Flame Burst should have a spell effect")
    {
        crate::effects::execute_effect(&mut game, effect, &mut ctx)
            .expect("Flame Burst damage should resolve");
    }

    game.life_total(bob)
}

#[test]
pub(super) fn pardic_firecat_in_graveyard_counts_for_flame_burst_damage() {
    let bob_life = resolve_flame_burst_with_pardic_firecat_in_zones(Zone::Stack, Zone::Graveyard);
    assert_eq!(
        bob_life, 16,
        "Flame Burst should deal 4 damage with one Flame Burst and Pardic Firecat in graveyards"
    );
}

#[test]
pub(super) fn pardic_firecat_on_battlefield_does_not_count_for_flame_burst_damage() {
    let bob_life = resolve_flame_burst_with_pardic_firecat_in_zones(Zone::Stack, Zone::Battlefield);
    assert_eq!(
        bob_life, 17,
        "Flame Burst should deal only 3 damage when Pardic Firecat is not in a graveyard"
    );
}

#[test]
pub(super) fn pardic_firecat_does_not_count_for_non_spell_flame_burst_effect_source() {
    let bob_life =
        resolve_flame_burst_with_pardic_firecat_in_zones(Zone::Battlefield, Zone::Graveyard);
    assert_eq!(
        bob_life, 17,
        "Pardic Firecat should count only for effects from spells named Flame Burst"
    );
}

#[test]
pub(super) fn captain_america_first_avenger_strict_parser_and_compiled_text_regression() {
    assert_oracle_card_parses_strict("Captain America, First Avenger");

    let def = parse_oracle_card_definition("Captain America, First Avenger");
    let rendered = compiled_text_lines(&def).join("\n");
    let ability_debug = format!("{:#?}", def.abilities);

    assert!(
        rendered.contains("Throw...")
            && rendered.contains("Unattach an Equipment from Captain America")
            && rendered.contains("that Equipment's mana value")
            && rendered.contains("divided as you choose among one, two, or three targets")
            && rendered.contains("... Catch")
            && rendered.contains("attach up to one target Equipment you control"),
        "expected Captain America's Throw/Catch labels and distributed-damage text, got {rendered}"
    );
    let activated = captain_america_throw_ability(&def);
    let damage_effect = activated
        .effects
        .flattened_default_effects()
        .iter()
        .find(|effect| captain_america_distributed_damage(effect).is_some())
        .expect("Throw should have a distributed damage effect");
    let damage = captain_america_distributed_damage(damage_effect)
        .expect("Throw should lower through a distributed damage effect");
    let target_count = damage_effect
        .0
        .get_target_count()
        .expect("distributed damage should expose target count");
    assert_eq!(target_count.min, 1);
    assert_eq!(target_count.max, Some(3));
    assert!(
        ability_debug.contains("UnattachObjectsEffect")
            && format!("{:?}", damage.amount).contains("ManaValueOf"),
        "expected Captain America to lower to unattach cost plus mana-value distributed damage, got {ability_debug}"
    );
}

pub(super) fn captain_america_distributed_damage(
    effect: &crate::effect::Effect,
) -> Option<&crate::effects::DealDistributedDamageEffect> {
    effect
        .downcast_ref::<crate::effects::DealDistributedDamageEffect>()
        .or_else(|| {
            effect
                .downcast_ref::<crate::effects::TaggedEffect>()
                .and_then(|tagged| {
                    tagged
                        .effect
                        .downcast_ref::<crate::effects::DealDistributedDamageEffect>()
                })
        })
}

pub(super) fn captain_america_throw_ability(
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
                    .any(|effect| captain_america_distributed_damage(effect).is_some()) =>
            {
                Some(activated)
            }
            _ => None,
        })
        .expect("Captain America should have its Throw activated ability")
}

pub(super) fn create_attached_test_equipment(
    game: &mut crate::game_state::GameState,
    controller: PlayerId,
    host: ObjectId,
) -> ObjectId {
    let equipment = CardDefinitionBuilder::new(CardId::from_raw(72_001), "Vibranium Shield")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(4)]]))
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Equipment])
        .build();
    let equipment_id =
        game.create_object_from_definition(&equipment, controller, Zone::Battlefield);
    game.object_mut(equipment_id)
        .expect("test Equipment should exist")
        .attached_to = Some(crate::object::AttachmentTarget::Object(host));
    game.object_mut(host)
        .expect("Captain America should exist")
        .attachments
        .push(equipment_id);
    equipment_id
}

pub(super) fn pay_captain_america_throw_costs(
    game: &mut crate::game_state::GameState,
    source: ObjectId,
    controller: PlayerId,
    activated: &crate::ability::ActivatedAbility,
    equipment: Option<ObjectId>,
) -> Result<
    std::collections::HashMap<crate::tag::TagKey, Vec<crate::snapshot::ObjectSnapshot>>,
    crate::cost::CostPaymentError,
> {
    game.player_mut(controller)
        .expect("controller should exist")
        .mana_pool
        .add(ManaSymbol::Colorless, 3);

    let mut dm = crate::decision::AutoPassDecisionMaker;
    let mut cost_ctx = crate::costs::CostContext::new(source, controller, &mut dm);
    if let Some(equipment) = equipment {
        cost_ctx.pre_chosen_cards = vec![equipment];
    }

    for cost in activated.mana_cost.costs() {
        cost.pay(game, &mut cost_ctx)?;
    }

    Ok(cost_ctx.tagged_objects)
}

#[test]
pub(super) fn captain_america_throw_unattaches_equipment_and_deals_its_mana_value_divided_damage() {
    struct DivideDamageBetweenPlayers {
        first_player: PlayerId,
        second_player: PlayerId,
    }

    impl crate::decision::DecisionMaker for DivideDamageBetweenPlayers {
        fn decide_distribute(
            &mut self,
            _game: &crate::game_state::GameState,
            ctx: &crate::decisions::context::DistributeContext,
        ) -> Vec<(crate::game_state::Target, u32)> {
            assert_eq!(
                ctx.total, 4,
                "damage total should be the Equipment's mana value"
            );
            assert_eq!(ctx.min_per_target, 1);
            vec![
                (crate::game_state::Target::Player(self.first_player), 1),
                (crate::game_state::Target::Player(self.second_player), 3),
            ]
        }
    }

    let def = parse_oracle_card_definition("Captain America, First Avenger");
    let activated = captain_america_throw_ability(&def);
    let damage_effect = activated
        .effects
        .flattened_default_effects()
        .iter()
        .find(|effect| captain_america_distributed_damage(effect).is_some())
        .expect("Throw should have a distributed damage effect");
    let target_count = damage_effect
        .0
        .get_target_count()
        .expect("distributed damage should expose target count");
    assert_eq!(target_count.min, 1);
    assert_eq!(target_count.max, Some(3));

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
    let captain_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let shield_id = create_attached_test_equipment(&mut game, alice, captain_id);

    let tagged_objects =
        pay_captain_america_throw_costs(&mut game, captain_id, alice, activated, Some(shield_id))
            .expect("Throw costs should be payable with three mana and attached Equipment");
    assert!(
        tagged_objects.values().any(|snapshots| snapshots
            .iter()
            .any(|snapshot| snapshot.name == "Vibranium Shield")),
        "paying the unattach cost should remember the Equipment for the damage amount"
    );
    assert_eq!(
        game.object(shield_id)
            .expect("Equipment should still exist")
            .attached_to,
        None,
        "paying the Throw cost should unattach the Equipment"
    );
    assert!(
        !game
            .object(captain_id)
            .expect("Captain America should still exist")
            .attachments
            .contains(&shield_id),
        "paying the Throw cost should remove the Equipment from Captain America's attachments"
    );

    let mut dm = DivideDamageBetweenPlayers {
        first_player: bob,
        second_player: charlie,
    };
    let mut ctx = crate::effects::ExecutionContext::new(captain_id, alice, &mut dm)
        .with_targets(vec![
            crate::effects::ResolvedTarget::Player(bob),
            crate::effects::ResolvedTarget::Player(charlie),
        ])
        .with_tagged_objects(tagged_objects);
    crate::effects::execute_effect(&mut game, damage_effect, &mut ctx)
        .expect("Throw distributed damage should resolve");

    assert_eq!(
        game.life_total(bob),
        19,
        "one damage should be assigned to the first player target"
    );
    assert_eq!(
        game.life_total(charlie),
        17,
        "three damage should be assigned to the second player target"
    );

    let mut game_without_equipment =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let empty_captain =
        game_without_equipment.create_object_from_definition(&def, alice, Zone::Battlefield);
    assert!(
        pay_captain_america_throw_costs(
            &mut game_without_equipment,
            empty_captain,
            alice,
            activated,
            None,
        )
        .is_err(),
        "Throw should not be payable without an Equipment attached to Captain America"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn gloomwidows_feast_definition() -> crate::cards::CardDefinition {
    parse_oracle_card_definition("Gloomwidow's Feast")
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn gloomwidows_feast_parses_strictly() {
    assert_oracle_card_parses_strict("Gloomwidow's Feast");

    let def = gloomwidows_feast_definition();
    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        !rendered.contains("unsupported predicate")
            && !rendered.contains("unsupported effect")
            && !rendered.contains("unsupported parser line fallback"),
        "Gloomwidow's Feast should parse without unsupported fallback markers, got {rendered}"
    );

    let debug = format!("{:#?}", def.spell_effect);
    assert!(
        debug.contains("DestroyEffect")
            && debug.contains("TaggedObjectMatches")
            && debug.contains("CreateTokenEffect")
            && debug.contains("Spider"),
        "Gloomwidow's Feast should lower to destroy, tagged color condition, and Spider token creation, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn gloomwidows_feast_compiled_text_preserves_blue_or_black_was_clause() {
    let def = gloomwidows_feast_definition();
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Destroy target creature with flying"),
        "Gloomwidow's Feast should render the flying target restriction, got {rendered}"
    );
    assert!(
        rendered.contains("If that creature was blue or black"),
        "Gloomwidow's Feast should render the historical blue-or-black condition, got {rendered}"
    );
    assert!(
        rendered.contains("create a 1/2 green Spider creature token with reach"),
        "Gloomwidow's Feast should render the conditional Spider token, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_of_talents_preserves_optional_same_name_search() {
    let def = parse_oracle_card_definition("Test of Talents");
    let rendered = compiled_text_lines(&def).join(" ");
    assert!(
        rendered.contains("for any number of cards with the same name")
            && !rendered.contains("for all cards with the same name"),
        "Test of Talents should preserve optional same-name search mode, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn fulgent_distraction_taps_chosen_creatures_and_unattaches_all_equipment() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Fulgent Distraction Variant")
        .parse_text("Choose two target creatures. Tap those creatures, then unattach all Equipment from them.")
        .expect("tap those creatures then unattach all Equipment should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Choose two target creatures")
            && rendered.contains("Tap those creatures, then unattach all Equipment from them"),
        "expected Fulgent-style choice carry into tap and unattach-all-equipment, got {rendered}"
    );
    let debug = format!("{:#?}", def.spell_effect);
    assert!(
        debug.contains("TapEffect") && debug.contains("UnattachObjectsEffect"),
        "expected tap plus unattach effects, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn disarm_unattaches_all_equipment_from_target_creature() {
    let def = parse_oracle_card_definition("Disarm");
    let rendered = compiled_text_lines(&def).join(" ");
    assert!(
        rendered.contains("Unattach all Equipment from target creature")
            && !rendered.contains("Equipment creature"),
        "expected Disarm to target the creature and unattach Equipment attached to it, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn boreal_outrider_snow_mana_cast_replacement_regression() {
    let def = parse_oracle_card_definition("Boreal Outrider");
    let rendered = compiled_text_lines(&def).join(" ");
    assert_eq!(
        rendered,
        "Whenever you cast a creature spell, if {S} of any of that spell's colors was spent to cast it, that creature enters with an additional +1/+1 counter on it.",
        "expected Boreal Outrider to render as a cast-spell enter-with replacement"
    );

    let debug = format!("{:#?}", def.abilities);
    assert!(
        debug.contains("EnterWithCountersForFilter")
            && debug.contains("SnowManaOfAnySpellColorSpentToCastThisSpell")
            && !debug.contains("SpellCastTrigger")
            && !debug.contains("PutCountersEffect"),
        "expected Boreal Outrider to compile to an enter-with replacement, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn enchanted_creature_tap_does_not_render_as_those_creatures() {
    let def = parse_oracle_card_definition("Burden of Guilt");
    let rendered = compiled_text_lines(&def).join(" ");
    assert!(
        rendered.contains("Tap enchanted creature")
            || rendered.contains("Tap an enchanted creature"),
        "expected Aura tap surface to refer to enchanted creature, got {rendered}"
    );
    assert!(
        !rendered.contains("Tap those creatures"),
        "enchanted-creature tap should not inherit Fulgent-style wording, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn thundermaw_hellkite_taps_creatures_damaged_by_enter_trigger() {
    let def = parse_oracle_card_definition("Thundermaw Hellkite");
    let rendered = compiled_text_lines(&def).join(" ");
    assert!(
        rendered.contains("Tap those creatures") && !rendered.contains("Tap that creature"),
        "expected Thundermaw's damage-each follow-up to tap the damaged creature set, got {rendered}"
    );

    let debug = format!("{:#?}", def.abilities);
    let tap_debug = debug
        .split("TapEffect")
        .nth(1)
        .expect("expected Thundermaw ability to include TapEffect");
    assert!(
        tap_debug.contains("damaged_0") && !tap_debug.contains("\"triggering\""),
        "expected Thundermaw tap to reference damaged creatures instead of the triggering source, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn rupture_split_damage_keeps_sacrificed_creature_power_reference() {
    let def = parse_oracle_card_definition("Rupture");
    let rendered = compiled_text_lines(&def).join(" ");
    assert!(
        rendered.contains("that creature's power")
            && rendered.contains("each creature without flying")
            && rendered.contains("each player")
            && !rendered.contains("that creature deals damage equal to its power to each player"),
        "expected Rupture's split damage to keep the sacrificed creature as the amount source, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn corpse_explosion_split_damage_keeps_exiled_card_power_reference() {
    let def = parse_oracle_card_definition("Corpse Explosion");
    let rendered = compiled_text_lines(&def).join(" ");
    assert!(
        rendered.contains("the exiled card's power")
            && rendered.contains("each creature")
            && rendered.contains("each planeswalker")
            && !rendered
                .contains("that creature deals damage equal to its power to each planeswalker"),
        "expected Corpse Explosion's split damage to keep the exiled card as the amount source, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn retched_wretch_style_return_it_then_loses_all_abilities_preserves_return() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Retched Wretch Variant")
        .parse_text("When this creature dies, if it had a -1/-1 counter on it, return it to the battlefield under its owner's control and it loses all abilities.")
        .expect("conditional return-it-and-lose-abilities trigger should parse");

    let rendered = compiled_text_lines(&def).join(" ");
    assert_eq!(
        rendered,
        "When this creature dies, if it had a -1/-1 counter on it, return it to the battlefield under its owner's control and it loses all abilities.",
        "expected Retched-style trigger to preserve oracle-like counter condition and chained effect"
    );
    let debug = format!("{:#?}", def.abilities);
    assert!(
        debug.contains("MoveToZoneEffect")
            && debug.contains("zone: Battlefield")
            && debug.contains("RemoveAllAbilities"),
        "expected return and remove-abilities effects, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn soulflayer_source_exiled_keyword_lines_merge_same_is_true() {
    let def = parse_oracle_card_definition("Soulflayer");
    let rendered = compiled_text_lines(&def).join(" ");
    assert!(
        rendered.contains("If a creature card with flying was exiled with this creature's delve ability, this creature has flying")
            && rendered.contains("The same is true for first strike")
            && !rendered.contains("This creature has first strike as long as"),
        "expected Soulflayer source-exiled keyword grants to compact with same-is-true wording, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn genesis_ultimatum_style_put_matching_battlefield_rest_hand_compacts() {
    let def = parse_oracle_card_definition("Genesis Ultimatum");
    let rendered = compiled_text_lines(&def).join(" ");
    assert!(
        rendered.contains(
            "Look at the top five cards of your library. Put any number of permanent cards from among them onto the battlefield and the rest into your hand"
        ) && rendered.contains("Exile Genesis Ultimatum")
            && !rendered.contains("Unless it's a permanent")
            && !rendered.contains("Exile this"),
        "expected Genesis-style looked-card split to compact chosen permanents and the true remainder, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn kamahls_druidic_vow_style_battlefield_rest_graveyard_compacts() {
    let def = parse_oracle_card_definition("Kamahl's Druidic Vow");
    let rendered = compiled_text_lines(&def).join(" ");
    assert!(
        rendered.contains(
            "Look at the top X cards of your library. You may put any number of land and/or legendary permanent cards with mana value X or less from among them onto the battlefield. Put the rest into your graveyard"
        ) && !rendered.contains("legendary lands")
            && !rendered.contains("Unless it's a permanent"),
        "expected Kamahl-style looked-card split to preserve union filter and true remainder, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn bitter_revelation_style_choose_cards_and_rest_graveyard_compacts() {
    assert_oracle_card_parses_strict("Bitter Revelation");

    let def = parse_oracle_card_definition("Bitter Revelation");
    let rendered = compiled_text_lines(&def).join(" ");
    assert!(
        rendered.contains(
            "Look at the top four cards of your library. Put two of them into your hand and the rest into your graveyard. You lose 2 life"
        ) && !rendered.contains("return that object")
            && !rendered.contains("Unless it's a permanent"),
        "expected Bitter Revelation to compact chosen cards and the true remainder, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn stargaze_style_choose_dynamic_cards_and_rest_graveyard_compacts() {
    assert_oracle_card_parses_strict("Stargaze");

    let def = parse_oracle_card_definition("Stargaze");
    let rendered = compiled_text_lines(&def).join(" ");
    let spell_debug = format!("{:#?}", def.spell_effect);
    assert!(
        rendered.contains(
            "Look at twice X cards from the top of your library. Put X cards from among them into your hand and the rest into your graveyard. You lose X life"
        ) && !rendered.contains("return that object")
            && !rendered.contains("Unless it's a permanent")
            && !rendered.contains("2*X"),
        "expected Stargaze to compact dynamic chosen cards and the true remainder, got {rendered}; spell effect was {spell_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn sticker_sheet_ticket_marker_rows_preserve_ticket_prefixes() {
    let info = oracle_card_info_by_name()
        .get("Happy Dead Squirrel")
        .expect("missing Happy Dead Squirrel oracle info");
    let type_line = info
        .type_line
        .as_deref()
        .expect("Happy Dead Squirrel should carry a sticker type line");
    let parse_input = format!("Type: {type_line}\n{}", info.oracle_text);

    let def = CardDefinitionBuilder::new(CardId::new(), "Happy Dead Squirrel")
        .parse_text(parse_input)
        .expect("Happy Dead Squirrel sticker sheet should parse strictly");
    let rendered = compiled_text_lines(&def).join("\n").to_ascii_lowercase();
    assert!(
        rendered.contains(
            "{tk}{tk} — {t}: add {c}{c}. spend this mana only to cast noncreature spells."
        ) && rendered.contains("{tk}{tk}{tk} — infect")
            && rendered.contains("{tk}{tk} — 3/2")
            && rendered.contains("{tk}{tk}{tk}{tk} — 4/7")
            && !rendered.contains("\ninfect\n"),
        "expected sticker-sheet rows to keep their ticket marker prefixes, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn ring_goes_south_consult_land_count_where_x_compacts() {
    let def = parse_oracle_card_definition("The Ring Goes South");
    let rendered = compiled_text_lines(&def).join(" ");
    assert!(
        rendered.contains(
            "The Ring tempts you. You reveal cards from the top of your library until you reveal X land cards, where X is the number of legendary creatures you control. Put those land cards onto the battlefield tapped and the rest on the bottom of your library in a random order"
        ) && !rendered.contains("the number of legendary creatures you control land cards"),
        "expected Ring-style consult split to preserve where-X count basis and tapped land move, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn leaf_migration_labeled_line_regression_cards_parse_strictly() {
    for name in [
        "Dr. Eggman",
        "Edge of Autumn",
        "Ka-Zar of the Savage Land",
        "Summon: Anima",
        "Summon: Bahamut",
        "Summon: Brynhildr",
        "Summon: Choco/Mog",
        "Summon: Esper Ramuh",
        "Summon: Fat Chocobo",
        "Summon: G.F. Cerberus",
        "Summon: Ixion",
        "Summon: Kujata",
        "Summon: Primal Garuda",
        "Summon: Primal Odin",
        "Summon: Shiva",
    ] {
        assert_oracle_card_parses_strict(name);
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn leaf_migration_nonmana_cycling_and_labeled_token_rule_render_truthfully() {
    let edge = parse_oracle_card_definition("Edge of Autumn");
    let edge_rendered = compiled_text_lines(&edge).join("\n");
    assert!(
        edge_rendered.contains("Cycling—Sacrifice a land")
            && !edge_rendered.contains("Cycling Exile a land")
            && !edge_rendered.contains("Sacrifice a permanent"),
        "typed nonmana cycling costs should compact to their semantic action: {edge_rendered}"
    );

    let ka_zar = parse_oracle_card_definition("Ka-Zar of the Savage Land");
    let ka_zar_rendered = compiled_text_lines(&ka_zar).join("\n");
    assert!(
        ka_zar_rendered.contains("Whenever a land you control enters, put a +1/+1 counter on"),
        "the named token's typed land-entry counter rule must survive lowering: {ka_zar_rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn looked_card_three_way_choices_lower_as_three_disjoint_candidate_selections() {
    for (name, text, middle_zone, expects_top) in [
        (
            "Three-Way Top Variant",
            "Look at the top three cards of your library. Put one of those cards into your hand, one on top of your library, and one on the bottom of your library.",
            "zone: Library",
            true,
        ),
        (
            "Three-Way Graveyard Variant",
            "Look at the top three cards of your library. Put one of those cards into your hand, one into your graveyard, and one on the bottom of your library.",
            "zone: Graveyard",
            false,
        ),
    ] {
        let def = CardDefinitionBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Instant])
            .parse_text(text)
            .expect("three-way looked-card choice should parse");
        let debug = format!("{:#?}", def.spell_effect);

        assert_eq!(
            debug.matches("ChooseObjectsEffect").count(),
            3,
            "each destination must lower from its own one-card tag: {debug}"
        );
        assert!(debug.contains("looked_candidates"), "{debug}");
        assert!(debug.contains("IsNotTaggedObject"), "{debug}");
        assert!(debug.contains("zone: Hand"), "{debug}");
        assert!(debug.contains(middle_zone), "{debug}");
        assert_eq!(debug.contains("to_top: true"), expects_top, "{debug}");
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn top_three_partition_cluster_parses_strictly_and_compiles_exactly() {
    for (name, expected) in [
        (
            "Dark Bargain",
            "Look at the top three cards of your library. Put two of them into your hand and the other into your graveyard. Dark Bargain deals 2 damage to you.",
        ),
        (
            "Moment of Truth",
            "Look at the top three cards of your library. Put one of those cards into your hand, one into your graveyard, and one on the bottom of your library.",
        ),
        (
            "Omen",
            "Look at the top three cards of your library, then put them back in any order. You may shuffle. Draw a card.",
        ),
        (
            "Ponder",
            "Look at the top three cards of your library, then put them back in any order. You may shuffle. Draw a card.",
        ),
        (
            "Telling Time",
            "Look at the top three cards of your library. Put one of those cards into your hand, one on top of your library, and one on the bottom of your library.",
        ),
    ] {
        assert_oracle_card_parses_strict(name);
        let def = parse_oracle_card_definition(name);
        let rendered = compiled_text_lines(&def).join(" ");
        assert_eq!(rendered, expected, "{name}: {def:#?}");
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn dark_bargain_lowers_to_selected_cards_plus_their_exact_complement() {
    let def = parse_oracle_card_definition("Dark Bargain");
    let debug = format!("{:#?}", def.spell_effect);

    assert!(debug.contains("LookAtTopCardsEffect"), "{debug}");
    assert!(debug.contains("ChooseObjectsEffect"), "{debug}");
    assert!(
        debug.contains("min: 2") && debug.contains("TagMatchingObjectsEffect"),
        "{debug}"
    );
    assert!(debug.contains("IsNotTaggedObject"), "{debug}");
    assert!(debug.contains("zone: Hand"), "{debug}");
    assert!(debug.contains("zone: Graveyard"), "{debug}");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn revealed_card_chooser_can_only_select_from_the_revealed_collection() {
    let beguiler = CardDefinitionBuilder::new(CardId::new(), "Revealed Trigger Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Whenever this creature deals combat damage to a player, that player reveals the top two cards of their library. You choose one of those cards and put it into their graveyard.",
        )
        .expect("triggered revealed-card choice should parse");
    let beguiler_debug = format!("{:#?}", beguiler.abilities);
    assert!(
        beguiler_debug.contains("revealed_candidates"),
        "{beguiler_debug}"
    );
    assert!(
        beguiler_debug.contains("revealed_choice"),
        "{beguiler_debug}"
    );
    assert!(
        beguiler_debug.contains("IsTaggedObject"),
        "{beguiler_debug}"
    );
    let triggered = beguiler
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("revealed-card probe should have a triggered ability");
    let revealed_choice = triggered
        .effects
        .segments
        .iter()
        .flat_map(|segment| segment.default_effects.iter())
        .find_map(|effect| effect.downcast_ref::<ChooseObjectsEffect>())
        .expect("revealed-card probe should choose from the revealed collection");
    assert_eq!(revealed_choice.zone, Some(Zone::Library));
    assert!(
        revealed_choice
            .filter
            .tagged_constraints
            .iter()
            .any(|constraint| {
                constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
                    && constraint.tag.as_str().contains("revealed_candidates")
            })
    );
    assert!(
        beguiler_debug.contains("zone: Graveyard"),
        "{beguiler_debug}"
    );

    let tome = CardDefinitionBuilder::new(CardId::new(), "Opponent Revealed Choice Variant")
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "{5}, {T}: Reveal the top three cards of your library. Target opponent chooses one of those cards. Put that card into your graveyard, then draw two cards.",
        )
        .expect("activated opponent revealed-card choice should parse");
    let tome_debug = format!("{:#?}", tome.abilities);
    assert!(tome_debug.contains("revealed_candidates"), "{tome_debug}");
    assert!(tome_debug.contains("revealed_choice"), "{tome_debug}");
    assert!(tome_debug.contains("IsTaggedObject"), "{tome_debug}");
    let activated = tome
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("revealed-card probe should have an activated ability");
    let opponent_choice = activated
        .effects
        .segments
        .iter()
        .flat_map(|segment| segment.default_effects.iter())
        .find_map(|effect| effect.downcast_ref::<ChooseObjectsEffect>())
        .expect("target opponent should choose from the revealed collection");
    assert_eq!(opponent_choice.chooser, PlayerFilter::target_opponent());
    assert!(tome_debug.contains("MoveToZoneEffect"), "{tome_debug}");
    assert!(tome_debug.contains("DrawCardsEffect"), "{tome_debug}");
}
