use super::*;
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::static_abilities::StaticAbilityId;
use ironsmith::types::Subtype;

fn five_players() -> GameState {
    GameState::new((0..5).map(|i| format!("Player {i}")).collect(), 20)
}

#[test]
fn reversing_turn_order_preserves_left_and_right() {
    let mut game = five_players();
    game.turn_store.turn_order.reverse();
    assert_eq!(
        game.closest_in_game_player_to_left_matching(A, |_| true),
        Some(PlayerId(1))
    );
    assert_eq!(
        game.closest_in_game_player_to_right_matching(A, |_| true),
        Some(PlayerId(4))
    );
}

#[test]
fn range_applies_to_battle_protectors_and_cards_in_graveyards() {
    let mut game = five_players();
    game.enable_limited_range_of_influence((0..5).map(PlayerId).collect(), vec![1; 5])
        .unwrap();
    let siege = put(&mut game, "invasion-of-zendikar", A, Zone::Battlefield);
    assert_eq!(
        game.legal_battle_protectors(siege),
        [PlayerId(1), PlayerId(4)]
    );
    let borrower = ironsmith_compiler_runtime::compile_to_runtime_definition(
        "Borrower", "Type: Creature — Shapeshifter\nPower/Toughness: 2/2\nAs long as a creature card with flying is in a graveyard, this creature has flying. The same is true for haste.", false,
    ).unwrap();
    let borrower = game.create_object_from_definition(&borrower, A, Zone::Battlefield);
    put(&mut game, "wind-drake", PlayerId(2), Zone::Graveyard);
    assert!(!game.current_has_static_ability_id(borrower, StaticAbilityId::Flying));
    put(&mut game, "wind-drake", PlayerId(1), Zone::Graveyard);
    assert!(game.current_has_static_ability_id(borrower, StaticAbilityId::Flying));
}

#[test]
fn printed_toughness_and_day_ability_are_evaluated_under_dress_down() {
    let mut game = game();
    put(&mut game, "dress-down", A, Zone::Battlefield);
    let shifter = enter_from_hand(&mut game, "shapeshifter", &mut SelectFirstDecisionMaker);
    assert_eq!(game.current_toughness(shifter), Some(7));
    enter_from_hand(&mut game, "gavony-dawnguard", &mut SelectFirstDecisionMaker);
    assert!(!game.has_day_night);
}

#[test]
fn nonbasic_land_types_include_planet_in_every_zone() {
    for zone in [
        Zone::Hand,
        Zone::Graveyard,
        Zone::Battlefield,
        Zone::Exile,
        Zone::Library,
    ] {
        let mut game = game();
        let land = put(&mut game, "planar-nexus", A, zone);
        assert!(game.current_has_subtype(land, Subtype::Planet));
        assert!(!game.current_has_subtype(land, Subtype::Satrapy));
    }
}

#[test]
fn setting_every_players_life_applies_once_to_the_archenemys_opposing_team() {
    let mut game = GameState::new((0..4).map(|i| format!("Player {i}")).collect(), 40);
    let deck = (0..10)
        .map(|i| {
            ironsmith::CardDefinition::new(
                ironsmith::card::CardBuilder::new(CardId::new(), format!("Scheme {i}"))
                    .card_types(vec![CardType::Scheme])
                    .build(),
            )
        })
        .collect();
    game.enable_archenemy(
        ironsmith::game_state::ArchenemyVariant::Commander,
        vec![(A, deck)],
    )
    .unwrap();
    resolve_spell(
        &mut game,
        "worldfire",
        vec![],
        &mut SelectFirstDecisionMaker,
    );
    for player in 0..4 {
        assert_eq!(game.player(PlayerId(player)).unwrap().life, 1);
    }
}

fn resolve_first_trigger(game: &mut GameState, source: ObjectId, target: Option<ObjectId>) {
    let trigger = game
        .object(source)
        .unwrap()
        .abilities
        .iter()
        .find_map(|ability| {
            if let AbilityKind::Triggered(trigger) = &ability.kind {
                Some(trigger.clone())
            } else {
                None
            }
        })
        .unwrap();
    let mut entry = ironsmith::StackEntry::ability(source, A, trigger.effects);
    if let Some(target) = target {
        entry.targets.push(ironsmith::Target::Object(target));
        entry
            .target_assignments
            .push(ironsmith::game_state::TargetAssignment {
                spec: trigger.choices[0].clone(),
                range: 0..1,
            });
    }
    game.push_to_stack(entry);
    ironsmith::game_loop::resolve_stack_entry_with(game, &mut SelectFirstDecisionMaker).unwrap();
}

fn qualified_hexproof_count(game: &GameState, recipient: ObjectId) -> usize {
    game.current_abilities(recipient).unwrap().iter().filter(|ability|
        matches!(&ability.kind, AbilityKind::Static(ability) if ability.id() == StaticAbilityId::HexproofFrom)
    ).count()
}

#[test]
fn static_and_resolving_grants_keep_the_donors_keyword_variants() {
    let mut game = game();
    let knight = put(&mut game, "knight-of-grace", A, Zone::Battlefield);
    let myriarch = put(&mut game, "majestic-myriarch", A, Zone::Battlefield);
    resolve_first_trigger(&mut game, myriarch, None);
    let donor = game.move_object_by_effect(knight, Zone::Graveyard).unwrap();
    let stable = game.object(donor).unwrap().stable_id;
    assert_eq!(qualified_hexproof_count(&game, myriarch), 1);
    let scavengers = put(&mut game, "urborg-scavengers", A, Zone::Battlefield);
    resolve_first_trigger(&mut game, scavengers, Some(donor));
    let exiled = game.find_object_by_stable_id(stable).unwrap();
    assert_eq!(game.object(exiled).unwrap().zone, Zone::Exile);
    assert_eq!(qualified_hexproof_count(&game, scavengers), 1);
    game.move_object_by_effect(exiled, Zone::Graveyard).unwrap();
    assert_eq!(qualified_hexproof_count(&game, scavengers), 0);
}

#[test]
fn kathril_counts_each_distinct_qualified_keyword_counter() {
    let mut game = game();
    put(&mut game, "knight-of-grace", A, Zone::Graveyard);
    put(&mut game, "knight-of-malice", A, Zone::Graveyard);
    let kathril = put(&mut game, "kathril-aspect-warper", A, Zone::Battlefield);
    resolve_first_trigger(&mut game, kathril, None);
    assert_eq!(qualified_hexproof_count(&game, kathril), 2);
    assert_eq!(game.counter_count(kathril, CounterType::PlusOnePlusOne), 3);
}

#[test]
fn a_double_slash_can_belong_to_a_single_oracle_name() {
    let game = game();
    let choice =
        ironsmith::decisions::context::TextInputContext::new(A, None, "Choose a card name");
    assert!(choice.allows_card_name(&game, "SP//dr, Piloted by Peni"));
    assert!(choice.allows_card_name(&game, "Ice"));
    assert!(!choice.allows_card_name(&game, "Fire // Ice"));
}

fn resolve_for_objects(
    game: &mut GameState,
    route: &str,
    targets: &[ObjectId],
    dm: &mut impl DecisionMaker,
) {
    let spell = put(game, route, A, Zone::Stack);
    let mut entry = ironsmith::StackEntry::new(spell, A);
    let requirements = ironsmith::game_loop::extract_target_requirements_from_program_with_modes(
        game,
        game.object(spell).unwrap().spell_effect.as_deref().unwrap(),
        A,
        Some(spell),
        None,
    );
    assert_eq!(requirements.len(), targets.len());
    for (i, (requirement, target)) in requirements
        .into_iter()
        .zip(targets.iter().copied())
        .enumerate()
    {
        let target = ironsmith::Target::Object(target);
        assert!(requirement.legal_targets.contains(&target));
        entry.targets.push(target);
        entry
            .target_assignments
            .push(ironsmith::game_state::TargetAssignment {
                spec: requirement.spec,
                range: i..i + 1,
            });
    }
    game.push_to_stack(entry);
    ironsmith::game_loop::resolve_stack_entry_with(game, dm).unwrap();
}

#[test]
fn a_stolen_cauldron_grants_its_linked_card_to_the_new_controllers_creature() {
    let mut game = game();
    let opponent = PlayerId(1);
    let cauldron = put(
        &mut game,
        "agatha-s-soul-cauldron",
        opponent,
        Zone::Battlefield,
    );
    let donor = put(&mut game, "leaden-myr", opponent, Zone::Exile);
    game.add_exiled_with_source_link(cauldron, donor);
    let recipient = creature(&mut game, "Recipient", 2, 2);
    game.add_counters(recipient, CounterType::PlusOnePlusOne, 1)
        .unwrap();
    let has_activation = |game: &GameState| {
        game.current_abilities(recipient)
            .unwrap()
            .iter()
            .any(|a| matches!(a.kind, AbilityKind::Activated(_)))
    };
    assert!(!has_activation(&game));
    resolve_for_objects(
        &mut game,
        "confiscate",
        &[cauldron],
        &mut SelectFirstDecisionMaker,
    );
    assert_eq!(game.controller_of_id(cauldron), Some(A));
    assert!(has_activation(&game));
}

#[test]
fn marvin_stops_borrowing_from_a_creature_when_spy_kit_gives_it_that_name() {
    let mut game = game();
    let marvin = put(&mut game, "marvin-murderous-mimic", A, Zone::Battlefield);
    put(&mut game, "leaden-myr", A, Zone::Battlefield);
    let kit = put(&mut game, "spy-kit", A, Zone::Battlefield);
    let activations = |game: &GameState| {
        game.current_abilities(marvin)
            .unwrap()
            .iter()
            .filter(|ability| matches!(ability.kind, AbilityKind::Activated(_)))
            .count()
    };
    assert_eq!(activations(&game), 1);
    assert!(game.attach_object_to_target(kit, ironsmith::object::AttachmentTarget::Object(marvin)));
    assert_eq!(activations(&game), 0);
}

#[test]
fn psychic_paper_applies_a_creature_name_from_outside_the_game() {
    struct ChooseName;
    impl DecisionMaker for ChooseName {
        fn decide_text(
            &mut self,
            game: &GameState,
            choice: &ironsmith::decisions::context::TextInputContext,
        ) -> String {
            let name = "Ravager of the Fells";
            assert!(choice.allows_card_name(game, name));
            name.into()
        }
    }
    let mut game = game();
    let paper = put(&mut game, "psychic-paper", A, Zone::Battlefield);
    let wearer = put(&mut game, "grizzly-bears", A, Zone::Battlefield);
    resolve_for_objects(
        &mut game,
        "magnetic-theft",
        &[paper, wearer],
        &mut ChooseName,
    );
    assert_eq!(
        game.current_name(wearer).as_deref(),
        Some("Ravager of the Fells")
    );
}

#[test]
fn storm_of_souls_returns_grist_past_containment_priest() {
    let mut game = game();
    put(
        &mut game,
        "containment-priest",
        PlayerId(1),
        Zone::Battlefield,
    );
    let grist = put(&mut game, "grist-the-hunger-tide", A, Zone::Graveyard);
    let stable = game.object(grist).unwrap().stable_id;
    assert!(game.current_has_card_type(grist, CardType::Creature));
    resolve_spell(
        &mut game,
        "storm-of-souls",
        vec![],
        &mut SelectFirstDecisionMaker,
    );
    let returned = game.find_object_by_stable_id(stable).unwrap();
    assert_eq!(game.object(returned).unwrap().zone, Zone::Battlefield);
    assert!(game.current_has_card_type(returned, CardType::Planeswalker));
    assert!(!game.current_has_card_type(returned, CardType::Creature));
    assert_eq!(game.counter_count(returned, CounterType::Loyalty), 3);
}

#[test]
fn each_keyword_grant_gives_one_copy_of_each_matching_variant() {
    for zone in [Zone::Graveyard, Zone::Battlefield] {
        let mut game = game();
        let recipient = if zone == Zone::Graveyard {
            let definition = ironsmith_compiler_runtime::compile_to_runtime_definition(
                "Borrower",
                "Type: Creature — Shapeshifter\nPower/Toughness: 2/2\nAs long as a creature card with flying is in a graveyard, this creature has flying. The same is true for hexproof.",
                false,
            ).unwrap();
            game.create_object_from_definition(&definition, A, Zone::Battlefield)
        } else {
            put(&mut game, "majestic-myriarch", A, Zone::Battlefield)
        };
        for donor in ["knight-of-grace", "garruk-s-harbinger", "knight-of-malice"] {
            put(&mut game, donor, A, zone);
        }
        if zone == Zone::Battlefield {
            resolve_first_trigger(&mut game, recipient, None);
        }
        assert_eq!(qualified_hexproof_count(&game, recipient), 2, "{zone:?}");
        if zone == Zone::Battlefield {
            resolve_first_trigger(&mut game, recipient, None);
            assert_eq!(qualified_hexproof_count(&game, recipient), 4);
        }
    }
}

struct ChangeColorWord(&'static str, &'static str);

impl DecisionMaker for ChangeColorWord {
    fn decide_options(&mut self, _: &GameState, options: &SelectOptionsContext) -> Vec<usize> {
        let label = match options.description.as_str() {
            "Choose which kind of word to change" => "Color word",
            "Choose the word to replace" => self.0,
            "Choose the replacement word" => self.1,
            other => panic!("Unexpected Mind Bend choice {other}"),
        };
        vec![
            options
                .options
                .iter()
                .find(|option| option.legal && option.description.eq_ignore_ascii_case(label))
                .unwrap()
                .index,
        ]
    }
}

fn opponent_can_target(game: &GameState, spell: &str, target: ObjectId) -> bool {
    let mut game = game.clone();
    let opponent = PlayerId(1);
    // Keep the spell castable even when the creature under test is protected.
    let control = put(&mut game, "grizzly-bears", opponent, Zone::Battlefield);
    let spell = put(&mut game, spell, opponent, Zone::Stack);
    let program = game.current_spell_program(spell).unwrap();
    let requirements = ironsmith::game_loop::extract_target_requirements_from_program_with_modes(
        &game,
        &program,
        opponent,
        Some(spell),
        None,
    );
    assert_eq!(requirements.len(), 1);
    assert!(
        requirements[0]
            .legal_targets
            .contains(&ironsmith::Target::Object(control))
    );
    requirements[0]
        .legal_targets
        .contains(&ironsmith::Target::Object(target))
}

#[test]
fn text_changed_copies_supply_both_variants_to_resolving_and_static_grants() {
    for (recipient, donor, controller, old_word, new_word, first_spell, second_spell) in [
        (
            "majestic-myriarch",
            "knight-of-grace",
            A,
            "Black",
            "White",
            "murder",
            "swords-to-plowshares",
        ),
        (
            "escaped-shapeshifter",
            "black-knight",
            PlayerId(1),
            "White",
            "Red",
            "swords-to-plowshares",
            "lightning-bolt",
        ),
    ] {
        let mut game = game();
        let definition = card(donor);
        game.create_object_from_definition(&definition, controller, Zone::Battlefield);
        let altered =
            game.create_object_from_definition(&definition, controller, Zone::Battlefield);
        resolve_for_objects(
            &mut game,
            "mind-bend",
            &[altered],
            &mut ChangeColorWord(old_word, new_word),
        );
        let source = put(&mut game, recipient, A, Zone::Battlefield);
        if recipient == "majestic-myriarch" {
            resolve_first_trigger(&mut game, source, None);
        }
        for spell in [first_spell, second_spell] {
            assert!(
                !opponent_can_target(&game, spell, source),
                "{spell} targeted {recipient}"
            );
        }
    }
}

#[test]
fn inheriting_all_abilities_preserves_each_donors_occurrences() {
    use ironsmith::ability::Ability;
    use ironsmith::static_abilities::{CopyStaticAbilityVariants, StaticAbility};
    let mut game = game();
    let definition = card("knight-of-grace");
    for _ in 0..2 {
        game.create_object_from_definition(&definition, A, Zone::Battlefield);
    }
    let mut inherit = CopyStaticAbilityVariants::new(
        ironsmith::target::ObjectFilter::creature().named("Knight of Grace"),
        vec![],
        "All abilities of creatures named Knight of Grace".into(),
    );
    inherit.copy_all = true;
    let borrower = CardDefinitionBuilder::new(CardId::new(), "Borrower")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .with_ability(Ability::static_ability(
            StaticAbility::copy_static_ability_variants(inherit),
        ))
        .build();
    let borrower = game.create_object_from_definition(&borrower, A, Zone::Battlefield);
    assert_eq!(qualified_hexproof_count(&game, borrower), 2);
}

#[test]
fn each_color_hexproof_supplies_five_counters_and_has_no_printed_color_word_to_change() {
    let mut game = game();
    let donor = put(&mut game, "breaker-of-creation", A, Zone::Battlefield);
    resolve_for_objects(
        &mut game,
        "mind-bend",
        &[donor],
        &mut ChangeColorWord("Black", "White"),
    );
    assert_eq!(qualified_hexproof_count(&game, donor), 5);
    assert!(!opponent_can_target(&game, "murder", donor));
    game.move_object_by_effect(donor, Zone::Graveyard).unwrap();
    let kathril = put(&mut game, "kathril-aspect-warper", A, Zone::Battlefield);
    resolve_first_trigger(&mut game, kathril, None);
    assert_eq!(qualified_hexproof_count(&game, kathril), 5);
    assert_eq!(game.counter_count(kathril, CounterType::PlusOnePlusOne), 5);
}

#[test]
fn tam_uses_each_donors_colors_when_the_myriarch_borrows_hexproof() {
    let mut game = game();
    let tam = put(&mut game, "tam-mindful-first-year", A, Zone::Battlefield);
    let strix = put(&mut game, "baleful-strix", A, Zone::Battlefield);
    let thopter = put(&mut game, "ornithopter", A, Zone::Battlefield);
    let myriarch = put(&mut game, "majestic-myriarch", A, Zone::Battlefield);
    assert_eq!(qualified_hexproof_count(&game, strix), 2);
    assert_eq!(qualified_hexproof_count(&game, thopter), 0);
    resolve_first_trigger(&mut game, myriarch, None);
    game.move_object_by_effect(tam, Zone::Exile).unwrap();
    assert_eq!(qualified_hexproof_count(&game, myriarch), 3);
    assert!(!opponent_can_target(&game, "murder", myriarch));
    assert!(!opponent_can_target(&game, "pongify", myriarch));
    assert!(opponent_can_target(&game, "lightning-bolt", myriarch));
}

#[test]
fn concerted_effort_copies_the_five_colors_of_iridescent_angels_protection() {
    let mut game = game();
    let angel = put(&mut game, "iridescent-angel", A, Zone::Battlefield);
    let recipient = put(&mut game, "grizzly-bears", A, Zone::Battlefield);
    let enchantment = put(&mut game, "concerted-effort", A, Zone::Battlefield);
    resolve_first_trigger(&mut game, enchantment, None);
    game.move_object_by_effect(angel, Zone::Graveyard).unwrap();
    let count = game.current_abilities(recipient).unwrap().iter().filter(|ability|
        matches!(&ability.kind, AbilityKind::Static(keyword) if keyword.id() == StaticAbilityId::Protection)).count();
    assert_eq!(count, 5);
    assert!(!opponent_can_target(&game, "lightning-bolt", recipient));
}

#[test]
fn indominus_draws_five_for_discarding_breaker_of_creation() {
    use ironsmith::decisions::context::SelectObjectsContext;
    struct Discard(ObjectId);
    impl DecisionMaker for Discard {
        fn decide_objects(&mut self, _: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
            assert!(
                ctx.candidates
                    .iter()
                    .any(|card| card.legal && card.id == self.0)
            );
            vec![self.0]
        }
    }
    let mut game = game();
    let breaker = put(&mut game, "breaker-of-creation", A, Zone::Hand);
    for _ in 0..6 {
        put(&mut game, "forest", A, Zone::Library);
    }
    let rex = enter_from_hand(&mut game, "indominus-rex-alpha", &mut Discard(breaker));
    assert!(game.player(A).unwrap().hand.is_empty());
    assert_eq!(qualified_hexproof_count(&game, rex), 5);
    let mut triggers = ironsmith::TriggerQueue::new();
    ironsmith::game_loop::try_drain_pending_trigger_events(&mut game, &mut triggers).unwrap();
    ironsmith::game_loop::put_triggers_on_stack_with_dm(
        &mut game,
        &mut triggers,
        &mut SelectFirstDecisionMaker,
    )
    .unwrap();
    assert_eq!(game.stack.last().unwrap().object_id, rex);
    ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker)
        .unwrap();
    assert_eq!(game.player(A).unwrap().hand.len(), 5);
}

#[test]
fn concerted_effort_keeps_pledges_colors_after_the_aura_leaves() {
    use ironsmith::effects::AttachToEffect;
    let mut game = game();
    let bear = put(&mut game, "grizzly-bears", A, Zone::Battlefield);
    let strix = put(&mut game, "baleful-strix", A, Zone::Battlefield);
    let aura = put(&mut game, "pledge-of-loyalty", A, Zone::Battlefield);
    AttachToEffect::new(ironsmith::target::ChooseSpec::SpecificObject(bear))
        .execute(
            &mut game,
            &mut EffectContext::new(aura, A, &mut SelectFirstDecisionMaker),
        )
        .unwrap();
    let recipient = put(&mut game, "ornithopter", A, Zone::Battlefield);
    let effort = put(&mut game, "concerted-effort", A, Zone::Battlefield);
    resolve_first_trigger(&mut game, effort, None);
    game.move_object_by_effect(aura, Zone::Graveyard).unwrap();
    game.move_object_by_effect(strix, Zone::Exile).unwrap();
    for spell in ["murder", "pongify", "swords-to-plowshares"] {
        assert!(!opponent_can_target(&game, spell, recipient), "{spell}");
    }
    assert!(opponent_can_target(&game, "lightning-bolt", recipient));
}

#[test]
fn protection_from_a_resolved_color_set_grants_one_ability_per_color() {
    use ironsmith::{Ability, ColorSet, StaticAbility, ability::ProtectionFrom};
    for (colors, expected) in [
        (ColorSet::COLORLESS, 0),
        (ColorSet::WHITE, 1),
        (ColorSet::WHITE.union(ColorSet::BLUE), 2),
    ] {
        let mut game = game();
        // Samite Elder and Commander's Plate both materialize the relevant colors this way.
        let definition = CardDefinitionBuilder::new(CardId::new(), "Protected creature")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(1, 1))
            .with_ability(Ability::static_ability(StaticAbility::protection(
                ProtectionFrom::Color(colors),
            )))
            .build();
        let id = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let abilities = game.current_abilities(id).unwrap();
        let count = abilities.iter().filter(|ability|
            matches!(&ability.kind, AbilityKind::Static(keyword) if keyword.id() == StaticAbilityId::Protection)
        ).count();
        assert_eq!(count, expected);
    }
}

#[test]
fn a_siege_retains_an_opposing_protector_outside_its_new_controllers_range() {
    let controller = PlayerId(3);
    let protector = PlayerId(1);
    for range in [0, 1] {
        let mut g = five_players();
        let mut ranges = vec![1; 5];
        ranges[3] = range;
        g.enable_limited_range_of_influence((0..5).map(PlayerId).collect(), ranges)
            .unwrap();
        let battle = put(&mut g, "invasion-of-zendikar", A, Zone::Battlefield);
        assert!(g.set_battle_protector(battle, protector));
        g.set_current_controller(battle, controller).unwrap();
        assert!(!g.legal_battle_protectors(battle).contains(&protector));
        ironsmith::game_loop::check_and_apply_sbas(&mut g, &mut ironsmith::TriggerQueue::new())
            .unwrap();
        assert_eq!(g.object(battle).unwrap().zone, Zone::Battlefield);
        assert_eq!(g.battle_protector(battle), Some(protector));
    }
}

#[test]
fn greatest_mana_value_filters_rank_copies_for_static_and_resolving_effects() {
    let mut g = game();
    let opponent = PlayerId(1);
    let copy = put(&mut g, "ornithopter", opponent, Zone::Battlefield);
    let sphere = put(&mut g, "myr-battlesphere", A, Zone::Battlefield);
    resolve_spell(
        &mut g,
        "mirrorweave",
        vec![ironsmith::Target::Object(sphere)],
        &mut SelectFirstDecisionMaker,
    );
    g.move_object(
        sphere,
        Zone::Graveyard,
        ironsmith::events::cause::EventCause::effect(),
    )
    .unwrap();
    let dinosaur = put(&mut g, "colossal-dreadmaw", opponent, Zone::Battlefield);
    let favor = put(&mut g, "favor-of-the-mighty", A, Zone::Battlefield);
    assert!(ironsmith::targeting::has_protection_from_source(
        &g, copy, favor
    ));
    assert!(!ironsmith::targeting::has_protection_from_source(
        &g, dinosaur, favor
    ));

    resolve_spell(
        &mut g,
        "soul-shatter",
        vec![],
        &mut SelectFirstDecisionMaker,
    );
    assert!(g.object(copy).is_none());
    assert_eq!(g.object(dinosaur).unwrap().zone, Zone::Battlefield);
    assert!(ironsmith::targeting::has_protection_from_source(
        &g, dinosaur, favor
    ));
}

#[test]
fn mana_value_limits_on_targets_and_sweepers_use_the_copied_cost() {
    let opponent = PlayerId(1);
    for (underlying, copied, cheap) in [
        ("ornithopter", "myr-battlesphere", false),
        ("myr-battlesphere", "ornithopter", true),
    ] {
        let mut g = game();
        let permanent = put(&mut g, underlying, opponent, Zone::Battlefield);
        let model = put(&mut g, copied, A, Zone::Battlefield);
        resolve_spell(
            &mut g,
            "mirrorweave",
            vec![ironsmith::Target::Object(model)],
            &mut SelectFirstDecisionMaker,
        );

        let mut casting = g.clone();
        let ordinary = put(&mut casting, "grizzly-bears", opponent, Zone::Battlefield);
        let decay = put(&mut casting, "abrupt-decay", A, Zone::Stack);
        let program = casting
            .object(decay)
            .unwrap()
            .spell_effect
            .as_deref()
            .unwrap();
        let requirements =
            ironsmith::game_loop::extract_target_requirements_from_program_with_modes(
                &casting,
                program,
                A,
                Some(decay),
                None,
            );
        assert_eq!(requirements.len(), 1);
        assert!(
            requirements[0]
                .legal_targets
                .contains(&ironsmith::Target::Object(ordinary))
        );
        for id in [permanent, model] {
            assert_eq!(
                requirements[0]
                    .legal_targets
                    .contains(&ironsmith::Target::Object(id)),
                cheap
            );
        }
        resolve_spell(
            &mut g,
            "ritual-of-soot",
            vec![],
            &mut SelectFirstDecisionMaker,
        );
        assert_eq!(g.object(permanent).is_none(), cheap);
        assert_eq!(g.object(model).is_none(), cheap);
    }
}
