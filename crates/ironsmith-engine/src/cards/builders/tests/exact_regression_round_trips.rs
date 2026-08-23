#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

fn assert_exact_round_trip(name: &str, oracle: &str) {
    let definition = parse_oracle_card_definition(name);
    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        oracle,
        "{definition:#?}"
    );
}

#[test]
fn caribou_range_keeps_the_token_subtype_distinct_from_its_source_alias() {
    assert_exact_round_trip(
        "Caribou Range",
        "Enchant land you control\nEnchanted land has \"{W}{W}, {T}: Create a 0/1 white Caribou creature token.\"\nSacrifice a Caribou token: You gain 1 life.",
    );
    let debug = format!("{:#?}", parse_oracle_card_definition("Caribou Range"));
    assert!(debug.contains("Caribou"), "{debug}");
}

#[test]
fn squirrel_squatters_preserves_the_for_each_attraction_visit_surface() {
    assert_exact_round_trip(
        "Squirrel Squatters",
        "When this creature enters, open an Attraction. (Put the top card of your Attraction deck onto the battlefield.)\nWhenever this creature attacks, create a 1/1 green Squirrel creature token that's tapped and attacking for each Attraction you've visited this turn.",
    );
    let debug = format!("{:#?}", parse_oracle_card_definition("Squirrel Squatters"));
    assert!(debug.contains("AttractionsVisitedThisTurn"), "{debug}");
    assert!(debug.contains("ForEach"), "{debug}");
    assert!(debug.contains("enters_tapped: true"), "{debug}");
    assert!(debug.contains("enters_attacking: true"), "{debug}");
}

#[test]
fn apocalypse_hydra_preserves_the_typed_x_threshold_surface() {
    assert_exact_round_trip(
        "Apocalypse Hydra",
        "This creature enters with X +1/+1 counters on it. If X is 5 or more, it enters with an additional X +1/+1 counters on it.\n{1}{R}, Remove a +1/+1 counter from this creature: It deals 1 damage to any target.",
    );
    let debug = format!("{:#?}", parse_oracle_card_definition("Apocalypse Hydra"));
    assert!(
        debug.contains("XValueAtLeast") && debug.contains("5"),
        "{debug}"
    );
}

#[test]
fn unlucky_witness_surfaces_the_shared_one_play_budget() {
    assert_exact_round_trip(
        "Unlucky Witness",
        "When this creature dies, exile the top two cards of your library. Until your next end step, you may play one of those cards.",
    );
    let debug = format!("{:#?}", parse_oracle_card_definition("Unlucky Witness"));
    assert!(debug.contains("max_plays: Some"), "{debug}");
    assert!(debug.contains("cast_pool_is_plural: true"), "{debug}");
}

#[test]
fn frozen_in_ice_keeps_the_general_cant_untap_restriction() {
    assert_exact_round_trip(
        "Frozen in Ice",
        "Enchant creature\nWhen this Aura enters, tap enchanted creature.\nEnchanted creature loses all abilities and can't become untapped.",
    );
    let debug = format!("{:#?}", parse_oracle_card_definition("Frozen in Ice"));
    assert!(debug.contains("RemoveAllAbilities"), "{debug}");
    assert!(debug.contains("Untap"), "{debug}");
}

#[test]
fn elspeth_tirel_preserves_land_and_token_destroy_exceptions() {
    assert_exact_round_trip(
        "Elspeth Tirel",
        "+2: You gain 1 life for each creature you control.\n−2: Create three 1/1 white Soldier creature tokens.\n−5: Destroy all other permanents except for lands and tokens.",
    );
    let debug = format!("{:#?}", parse_oracle_card_definition("Elspeth Tirel"));
    assert!(debug.contains("nontoken: true"), "{debug}");
    assert!(debug.contains("excluded_card_types"), "{debug}");
    assert!(debug.contains("Land"), "{debug}");
}

#[test]
fn whirlwind_denial_keeps_the_complete_stack_object_domain() {
    assert_exact_round_trip(
        "Whirlwind Denial",
        "For each spell and ability your opponents control, counter it unless its controller pays {4}.",
    );
    let debug = format!("{:#?}", parse_oracle_card_definition("Whirlwind Denial"));
    assert!(debug.contains("SpellOrAbility"), "{debug}");
    assert!(debug.contains("UnlessPaysEffect"), "{debug}");
}

#[test]
fn battlefield_scrounger_uses_cards_from_your_graveyard_as_its_cost() {
    let definition = parse_oracle_card_definition("Battlefield Scrounger");
    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        "Threshold — Put three cards from your graveyard on the bottom of your library: This creature gets +3/+3 until end of turn. Activate only once each turn and only if there are seven or more cards in your graveyard.",
        "{definition:#?}"
    );

    let activated = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("Battlefield Scrounger should have an activated ability");
    let move_payment = activated
        .mana_cost
        .costs()
        .iter()
        .filter_map(|cost| cost.effect_ref())
        .find_map(|effect| effect.downcast_ref::<crate::effects::MoveToZoneEffect>())
        .expect("the three-card payment should be a typed zone move");
    assert_eq!(move_payment.zone, Zone::Library);
    assert!(!move_payment.to_top);
    assert_eq!(
        move_payment.target.count(),
        crate::effect::ChoiceCount::exactly(3)
    );
    let ChooseSpec::Object(payment_filter) = move_payment.target.base() else {
        panic!("expected an object payment choice: {move_payment:#?}");
    };
    assert_eq!(payment_filter.zone, Some(Zone::Graveyard));
    assert_eq!(payment_filter.owner, Some(PlayerFilter::You));
    assert!(
        activated
            .mana_cost
            .costs()
            .iter()
            .filter_map(|cost| cost.effect_ref())
            .all(|effect| effect
                .downcast_ref::<crate::effects::PutCountersEffect>()
                .is_none()),
        "graveyard must not survive as a named counter type: {activated:#?}"
    );
    assert_eq!(
        activated.timing,
        crate::ability::ActivationTiming::OncePerTurn
    );
    let Some(crate::ConditionExpr::PlayerHasAtLeast {
        player,
        filter,
        count,
    }) = activated.activation_condition.as_ref()
    else {
        panic!("expected an executable graveyard threshold: {activated:#?}");
    };
    assert_eq!(player, &PlayerFilter::You);
    assert_eq!(*count, 7);
    assert_eq!(filter.zone, Some(Zone::Graveyard));
    assert_eq!(filter.owner, Some(PlayerFilter::You));
}

#[test]
fn battlefield_scrounger_payment_is_owned_and_threshold_and_frequency_are_enforced() {
    fn graveyard_probe(name: &str) -> CardDefinition {
        CardDefinitionBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Instant])
            .build()
    }

    let definition = parse_oracle_card_definition("Battlefield Scrounger");
    let (ability_index, activated) = definition
        .abilities
        .iter()
        .enumerate()
        .find_map(|(index, ability)| match &ability.kind {
            AbilityKind::Activated(activated) => Some((index, activated.clone())),
            _ => None,
        })
        .expect("Battlefield Scrounger should have an activated ability");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);

    let mut bob_cards = Vec::new();
    for index in 0..7 {
        let id = game.create_object_from_definition(
            &graveyard_probe(&format!("Bob Graveyard Probe {index}")),
            bob,
            Zone::Graveyard,
        );
        bob_cards.push(game.object(id).expect("Bob's card should exist").stable_id);
    }
    let mut alice_cards = Vec::new();
    for index in 0..2 {
        let id = game.create_object_from_definition(
            &graveyard_probe(&format!("Alice Initial Graveyard Probe {index}")),
            alice,
            Zone::Graveyard,
        );
        alice_cards.push(
            game.object(id)
                .expect("Alice's card should exist")
                .stable_id,
        );
    }
    assert!(
        crate::cost::can_pay_cost(&game, source, alice, &activated.mana_cost).is_err(),
        "cards in another player's graveyard must not pay this cost"
    );
    assert!(
        !crate::decision::can_activate_ability_with_restrictions(
            &game,
            source,
            ability_index,
            &activated,
        ),
        "another player's seven cards must not satisfy your threshold"
    );

    for index in 0..5 {
        let id = game.create_object_from_definition(
            &graveyard_probe(&format!("Alice Added Graveyard Probe {index}")),
            alice,
            Zone::Graveyard,
        );
        alice_cards.push(
            game.object(id)
                .expect("Alice's card should exist")
                .stable_id,
        );
    }
    crate::cost::can_pay_cost(&game, source, alice, &activated.mana_cost)
        .expect("seven cards in Alice's graveyard should satisfy the cost");
    assert!(
        crate::decision::can_activate_ability_with_restrictions(
            &game,
            source,
            ability_index,
            &activated,
        ),
        "the owned seven-card threshold should allow the first activation"
    );

    let mut decisions = crate::decision::SelectFirstDecisionMaker;
    crate::special_actions::pay_total_cost_with_choice(
        &mut game,
        alice,
        source,
        &activated.mana_cost,
        crate::costs::PaymentReason::ActivateAbility,
        &mut decisions,
    )
    .expect("the typed three-card zone-move cost should execute");
    let moved_alice_cards = alice_cards
        .iter()
        .filter(|stable_id| {
            game.find_object_by_stable_id(**stable_id)
                .and_then(|id| game.object(id))
                .is_some_and(|object| object.zone == Zone::Library)
        })
        .count();
    assert_eq!(moved_alice_cards, 3);
    assert!(bob_cards.iter().all(|stable_id| {
        game.find_object_by_stable_id(*stable_id)
            .and_then(|id| game.object(id))
            .is_some_and(|object| object.zone == Zone::Graveyard)
    }));

    for index in 0..3 {
        game.create_object_from_definition(
            &graveyard_probe(&format!("Alice Restored Graveyard Probe {index}")),
            alice,
            Zone::Graveyard,
        );
    }
    game.record_ability_activation(source, ability_index);
    assert!(
        !crate::decision::can_activate_ability_with_restrictions(
            &game,
            source,
            ability_index,
            &activated,
        ),
        "the ability must not be activated a second time this turn"
    );
    game.next_turn();
    assert!(
        crate::decision::can_activate_ability_with_restrictions(
            &game,
            source,
            ability_index,
            &activated,
        ),
        "the per-turn limit should reset while the owned threshold remains satisfied"
    );
}

#[test]
fn chandra_fire_artisan_keeps_the_grouped_loyalty_removal_trigger() {
    assert_exact_round_trip(
        "Chandra, Fire Artisan",
        "Whenever one or more loyalty counters are removed from Chandra, she deals that much damage to target opponent or planeswalker.\n+1: Exile the top card of your library. You may play it this turn.\n−7: Exile the top seven cards of your library. You may play them this turn.",
    );
    let debug = format!(
        "{:#?}",
        parse_oracle_card_definition("Chandra, Fire Artisan")
    );
    assert!(debug.contains("CounterRemovedFromTrigger"), "{debug}");
    assert!(debug.contains("Loyalty"), "{debug}");
    assert!(debug.contains("one_or_more: true"), "{debug}");
    assert!(debug.contains("EventValue"), "{debug}");
}

#[test]
fn knight_of_the_mists_keeps_the_inline_no_regeneration_rider() {
    assert_exact_round_trip(
        "Knight of the Mists",
        "Flanking (Whenever a creature without flanking blocks this creature, the blocking creature gets -1/-1 until end of turn.)\nWhen this creature enters, you may pay {U}. If you don't, destroy target Knight and it can't be regenerated.",
    );
    let debug = format!("{:#?}", parse_oracle_card_definition("Knight of the Mists"));
    assert!(debug.contains("DestroyNoRegenerationEffect"), "{debug}");
    assert!(debug.contains("Knight"), "{debug}");
}

#[test]
fn stitcher_geralf_compacts_the_linked_mill_exile_and_dynamic_token_sequence() {
    assert_exact_round_trip(
        "Stitcher Geralf",
        "{2}{U}, {T}: Each player mills three cards. Exile up to two creature cards put into graveyards this way. Create an X/X blue Zombie creature token, where X is the total power of the cards exiled this way.",
    );
    let debug = format!("{:#?}", parse_oracle_card_definition("Stitcher Geralf"));
    assert!(debug.contains("PriorEffectMetric"), "{debug}");
    assert!(debug.contains("metric: TotalPower"), "{debug}");
    assert!(debug.contains("action: Some(\n"), "{debug}");
    assert!(debug.contains("Exiled"), "{debug}");
}

#[test]
fn indulgent_tormentor_keeps_both_payment_branches_on_the_target_opponent() {
    let definition = parse_oracle_card_definition("Indulgent Tormentor");
    assert_eq!(
        canonical_compiled_lines(&definition),
        vec![
            "Flying",
            "At the beginning of your upkeep, draw a card unless target opponent sacrifices a creature or pays 3 life.",
        ],
        "{definition:#?}"
    );

    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Indulgent Tormentor should retain its upkeep trigger");
    let unless_pays = triggered
        .effects
        .segments
        .iter()
        .flat_map(|segment| &segment.default_effects)
        .find_map(|effect| effect.downcast_ref::<crate::effects::UnlessPaysEffect>())
        .expect("sacrifice-or-life should remain one typed UnlessPays effect");
    assert_eq!(
        unless_pays.player,
        PlayerFilter::target_opponent(),
        "{unless_pays:#?}"
    );
    assert_eq!(
        unless_pays.cost.as_one_of().map(<[_]>::len),
        Some(2),
        "{unless_pays:#?}"
    );
}

#[test]
fn rakshasa_debaser_targets_only_the_defending_players_graveyard() {
    let definition = parse_oracle_card_definition("Rakshasa Debaser");
    assert_eq!(
        canonical_compiled_lines(&definition),
        vec![
            "Whenever this creature attacks, put target creature card from defending player's graveyard onto the battlefield under your control.",
            "Encore {6}{B}{B}.",
        ],
        "{definition:#?}"
    );
    let debug = format!("{definition:#?}");
    assert!(debug.contains("Defending"), "{debug}");
    assert!(debug.contains("Graveyard"), "{debug}");
}

#[test]
fn erhnam_djinn_grant_expires_at_the_next_upkeep_not_the_next_turn() {
    let definition = parse_oracle_card_definition("Erhnam Djinn");
    let rendered = canonical_compiled_lines(&definition).join("\n");
    assert!(rendered.contains("until your next upkeep"), "{rendered}");
    assert!(!rendered.contains("until your next turn"), "{rendered}");
    let debug = format!("{definition:#?}");
    assert!(debug.contains("YourNextUpkeep"), "{debug}");
    assert!(!debug.contains("YourNextTurn"), "{debug}");
}

#[test]
fn leonin_arbiter_retains_its_source_scoped_pay_to_ignore_special_action() {
    assert_exact_round_trip(
        "Leonin Arbiter",
        "Players can't search libraries. Any player may pay {2} for that player to ignore this effect until end of turn.",
    );
    let debug = format!("{:#?}", parse_oracle_card_definition("Leonin Arbiter"));
    assert!(
        debug.contains("AnyPlayerMayPayManaToIgnoreSourceEffectUntilEndOfTurn"),
        "{debug}"
    );
    assert!(debug.contains("Generic(2)"), "{debug}");
}

#[test]
fn pallimud_counts_only_tapped_lands_controlled_by_the_chosen_player() {
    let definition = parse_oracle_card_definition("Pallimud");
    let debug = format!("{definition:#?}");
    assert!(debug.contains("controller: Some(ChosenPlayer)"), "{debug}");
    assert!(debug.contains("tapped: true"), "{debug}");
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let pallimud = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    game.set_chosen_player(pallimud, bob);

    let land = CardDefinitionBuilder::new(CardId::from_raw(97_200), "Scope Probe Land")
        .card_types(vec![CardType::Land])
        .build();
    for controller in [bob, bob, alice, alice, alice] {
        let land_id = game.create_object_from_definition(&land, controller, Zone::Battlefield);
        game.tap(land_id);
    }
    let _untapped_bob_land = game.create_object_from_definition(&land, bob, Zone::Battlefield);

    assert_eq!(
        game.calculated_power(pallimud),
        Some(2),
        "Pallimud must ignore Alice's tapped lands and Bob's untapped land"
    );
}

#[test]
fn living_death_returns_each_players_own_exiled_creatures_under_that_players_control() {
    for (card_name, exchanged_type, noun) in [
        ("Living Death", CardType::Creature, "Creature"),
        ("Living End", CardType::Creature, "Creature"),
        ("Scrap Mastery", CardType::Artifact, "Artifact"),
    ] {
        let definition = parse_oracle_card_definition(card_name);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let mut game =
            crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let swap_card = |raw_id, name: String| {
            let builder = CardDefinitionBuilder::new(CardId::from_raw(raw_id), name)
                .card_types(vec![exchanged_type]);
            if exchanged_type == CardType::Creature {
                builder.power_toughness(PowerToughness::fixed(2, 2)).build()
            } else {
                builder.build()
            }
        };
        for (raw_id, owner, state) in [
            (97_210, alice, "Old"),
            (97_211, bob, "Old"),
            (97_212, alice, "Returning"),
            (97_213, bob, "Returning"),
        ] {
            let player_name = if owner == alice { "Alice" } else { "Bob" };
            let zone = if state == "Old" {
                Zone::Battlefield
            } else {
                Zone::Graveyard
            };
            game.create_object_from_definition(
                &swap_card(raw_id, format!("{player_name} {state} {noun}")),
                owner,
                zone,
            );
        }

        let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
        let mut ctx = crate::effects::ExecutionContext::new_default(source, alice);
        let program = definition
            .spell_effect
            .as_ref()
            .unwrap_or_else(|| panic!("{card_name} should have a spell program"));
        for effect in program.flattened_default_effects() {
            crate::effects::execute_effect(&mut game, effect, &mut ctx)
                .unwrap_or_else(|error| panic!("{card_name} should resolve: {error:?}"));
        }

        for (player_name, expected_controller) in [("Alice", alice), ("Bob", bob)] {
            let name = format!("{player_name} Returning {noun}");
            let object_id = game
                .objects_in_zone(Zone::Battlefield)
                .into_iter()
                .find(|object_id| {
                    game.object(*object_id)
                        .is_some_and(|object| object.name == name)
                })
                .unwrap_or_else(|| panic!("{card_name}: {name} should return"));
            assert_eq!(
                game.controller_of_id(object_id),
                Some(expected_controller),
                "{card_name}: each player must control their own returning card"
            );
        }
        for player_name in ["Alice", "Bob"] {
            let name = format!("{player_name} Old {noun}");
            assert!(
                game.objects_in_zone(Zone::Graveyard)
                    .into_iter()
                    .any(|object_id| game
                        .object(object_id)
                        .is_some_and(|object| object.name == name)),
                "{card_name}: {name} should be sacrificed"
            );
        }
    }
}

#[test]
fn arcane_artisan_retains_one_typed_player_target_across_the_result_chain() {
    assert_exact_round_trip(
        "Arcane Artisan",
        "{2}{U}, {T}: Target player draws a card, then exiles a card from their hand. If a creature card is exiled this way, that player creates a token that's a copy of that card.\nWhen this creature leaves the battlefield, exile all tokens created with it at the beginning of the next end step.",
    );
    let debug = format!("{:#?}", parse_oracle_card_definition("Arcane Artisan"));
    assert!(debug.contains("TargetOnlyEffect"), "{debug}");
}

#[test]
fn bifurcate_folds_the_typed_target_into_the_same_name_search() {
    assert_exact_round_trip(
        "Bifurcate",
        "Search your library for a permanent card with the same name as target nontoken creature, put that card onto the battlefield, then shuffle.",
    );
    let debug = format!("{:#?}", parse_oracle_card_definition("Bifurcate"));
    assert!(debug.contains("TargetOnlyEffect"), "{debug}");
    assert!(debug.contains("SameNameAsTagged"), "{debug}");
}

#[test]
fn welcome_to_the_fold_reuses_one_target_across_both_toughness_thresholds() {
    let definition = parse_oracle_card_definition("Welcome to the Fold");
    assert_eq!(
        canonical_compiled_lines(&definition),
        vec![
            "Gain control of target creature if its toughness is 2 or less. If this spell's madness cost was paid, instead gain control of that creature if its toughness is X or less.",
            "Madness {X}{U}{U}",
        ],
        "{definition:#?}"
    );
    let debug = format!("{definition:#?}");
    assert_eq!(debug.matches("TargetOnlyEffect").count(), 1, "{debug}");
    assert!(debug.contains("ThisSpellPaidLabel"), "{debug}");
    assert!(debug.contains("Madness"), "{debug}");
    assert!(debug.contains("LeadingIf"), "{debug}");
    assert!(debug.contains("ToughnessOf"), "{debug}");
    assert!(!debug.contains("SourceToughness"), "{debug}");
}

#[test]
fn overload_reuses_one_target_across_both_mana_value_thresholds() {
    let definition = parse_oracle_card_definition("Overload");
    assert_eq!(
        canonical_compiled_lines(&definition),
        vec![
            "Kicker {2}",
            "Destroy target artifact if its mana value is 2 or less. If this spell was kicked, destroy that artifact if its mana value is 5 or less instead.",
        ],
        "{definition:#?}"
    );
    let debug = format!("{definition:#?}");
    // Both threshold branches retain their own target-only wrapper, but they
    // deliberately share the same logical target tag.
    assert!(debug.contains("destroyed_0"), "{debug}");
    assert!(!debug.contains("destroyed_1"), "{debug}");
    assert!(debug.contains("SelfReplacementBranch"), "{debug}");
    assert!(debug.contains("ManaValueOf"), "{debug}");
}

#[test]
fn talus_paladin_renders_the_optional_group_grant_as_a_causative() {
    assert_exact_round_trip(
        "Talus Paladin",
        "Whenever this creature or another Ally you control enters, you may have Allies you control gain lifelink until end of turn, and you may put a +1/+1 counter on this creature.",
    );
    let debug = format!("{:#?}", parse_oracle_card_definition("Talus Paladin"));
    assert_eq!(debug.matches("MayEffect").count(), 2, "{debug}");
    assert!(debug.contains("ApplyContinuousEffect"), "{debug}");
    assert!(debug.contains("AddAbility"), "{debug}");
}

#[test]
fn soul_swindler_keeps_the_typed_attraction_visit_condition() {
    assert_exact_round_trip(
        "Soul Swindler",
        "As long as you've visited an Attraction this turn, this creature has indestructible.\nWhen this creature enters, open an Attraction. (Put the top card of your Attraction deck onto the battlefield.)",
    );
    let definition = parse_oracle_card_definition("Soul Swindler");
    let debug = format!("{definition:#?}");
    assert!(debug.contains("PlayerVisitedAttractionThisTurn"), "{debug}");
    assert!(debug.contains("Indestructible"), "{debug}");

    let mut game = crate::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    assert!(
        !game.object_has_static_ability_id(source, StaticAbilityId::Indestructible),
        "Soul Swindler must not be indestructible before its controller visits"
    );

    let bobs_visit = crate::events::RawEvent::new(
        crate::events::KeywordActionEvent::new(
            crate::events::KeywordActionKind::VisitAttraction,
            PlayerId::from_index(1),
            source,
            1,
        ),
        Default::default(),
    );
    game.turn_store
        .turn_history
        .record_event(&bobs_visit, None, None);
    assert!(
        !game.object_has_static_ability_id(source, StaticAbilityId::Indestructible),
        "another player's visit must not satisfy Soul Swindler's condition"
    );

    let opened = crate::events::RawEvent::new(
        crate::events::KeywordActionEvent::new(
            crate::events::KeywordActionKind::OpenAttraction,
            alice,
            source,
            1,
        ),
        Default::default(),
    );
    game.turn_store
        .turn_history
        .record_event(&opened, None, None);
    assert!(
        !game.object_has_static_ability_id(source, StaticAbilityId::Indestructible),
        "opening an Attraction must not satisfy Soul Swindler's visit condition"
    );

    let visited = crate::events::RawEvent::new(
        crate::events::KeywordActionEvent::new(
            crate::events::KeywordActionKind::VisitAttraction,
            alice,
            source,
            1,
        ),
        Default::default(),
    );
    game.turn_store
        .turn_history
        .record_event(&visited, None, None);
    assert!(
        game.object_has_static_ability_id(source, StaticAbilityId::Indestructible),
        "a typed visit event should turn on Soul Swindler's indestructible ability"
    );

    game.turn_store.turn_history.clear_for_new_turn();
    assert!(
        !game.object_has_static_ability_id(source, StaticAbilityId::Indestructible),
        "Soul Swindler's visit condition must reset on the next turn"
    );

    // Exercise the actual supplemental-deck/open/precombat-main pipeline. The
    // registry definition must retain the physical printing's Scryfall lights,
    // not merely the Visit ability's rendered text.
    let information_booth = parse_oracle_card_definition("Information Booth");
    assert_eq!(information_booth.card.attraction_lights, vec![5, 6]);
    game.enable_attractions(vec![(
        alice,
        crate::game_state::AttractionDeckFormat::Limited,
        vec![information_booth.clone(); 3],
    )])
    .expect("a three-card Limited Attraction deck should be legal");

    let mut open_ctx = crate::effects::ExecutionContext::new_default(source, alice);
    let open_outcome = crate::effects::execute_effect(
        &mut game,
        &crate::effect::Effect::open_attraction(),
        &mut open_ctx,
    )
    .expect("Soul Swindler's open action should execute");
    let opened_booth = match &open_outcome.value {
        crate::effect::OutcomeValue::Objects(objects) => objects.first().copied(),
        _ => None,
    }
    .expect("opening should move the top Attraction onto the battlefield");
    assert_eq!(game.face_up_attractions(), &[opened_booth]);
    assert_eq!(game.attraction_deck(alice).map(|deck| deck.len()), Some(2));
    assert_eq!(
        game.attraction_lights(opened_booth),
        Some([5, 6].as_slice())
    );

    let filler = CardDefinitionBuilder::new(CardId::from_raw(97_701), "Visit Draw Filler")
        .card_types(vec![CardType::Instant])
        .build();
    game.create_object_from_definition(&filler, alice, Zone::Library);
    let hand_before = game.player(alice).expect("Alice").hand.len();

    game.force_next_die_roll(6);
    let mut trigger_queue = crate::triggers::TriggerQueue::new();
    let mut runner = crate::turn_runner::TurnRunner::from_state_for_sync(
        crate::turn_runner::TurnState::FirstMain,
    );
    let action = runner
        .advance(&mut game, &mut trigger_queue)
        .expect("precombat main Attraction action should run");
    assert!(matches!(
        action,
        crate::turn_runner::TurnAction::RunPriority
    ));
    assert_eq!(
        trigger_queue
            .entries
            .iter()
            .filter(|entry| entry.source == opened_booth)
            .count(),
        1,
        "the lit face-up Information Booth should put exactly one Visit ability on the stack"
    );
    assert!(
        game.object_has_static_ability_id(source, StaticAbilityId::Indestructible),
        "the production visit event must immediately satisfy Soul Swindler"
    );

    let mut priority_dm = crate::decision::AutoPassDecisionMaker;
    crate::game_loop::run_priority_loop_with(&mut game, &mut trigger_queue, &mut priority_dm)
        .expect("Information Booth's Visit ability should resolve through normal priority");
    assert_eq!(
        game.player(alice).expect("Alice").hand.len(),
        hand_before + 1,
        "Information Booth's stored Visit program should draw a card"
    );
}

#[test]
fn storybook_ride_counts_actual_visits_not_controlled_attractions() {
    let storybook = parse_oracle_card_definition("Storybook Ride");
    let information_booth = parse_oracle_card_definition("Information Booth");
    assert_eq!(storybook.card.attraction_lights, vec![3, 4, 6]);
    assert_eq!(information_booth.card.attraction_lights, vec![5, 6]);
    let debug = format!("{storybook:#?}");
    assert!(debug.contains("AttractionsVisitedThisTurn"), "{debug}");
    let rendered = crate::runtime_display::compiled_text_lines(&storybook).join("\n");
    assert!(
        rendered.contains("the number of Attractions you've visited this turn"),
        "{rendered}"
    );
    assert!(
        !rendered.contains("Attractions on the battlefield"),
        "{rendered}"
    );

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = crate::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    game.enable_attractions(vec![(
        alice,
        crate::game_state::AttractionDeckFormat::Limited,
        vec![
            storybook.clone(),
            information_booth.clone(),
            information_booth,
        ],
    )])
    .expect("the mixed three-card Limited Attraction deck should be legal");

    let opener = CardDefinitionBuilder::new(CardId::from_raw(97_710), "Attraction Opener")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let opener_id = game.create_object_from_definition(&opener, alice, Zone::Battlefield);
    let mut open_ctx = crate::effects::ExecutionContext::new_default(opener_id, alice);
    for _ in 0..3 {
        let outcome = crate::effects::execute_effect(
            &mut game,
            &crate::effect::Effect::open_attraction(),
            &mut open_ctx,
        )
        .expect("each Attraction should open");
        assert!(
            matches!(outcome.value, crate::effect::OutcomeValue::Objects(ref objects) if objects.len() == 1),
            "opening should move exactly one Attraction: {outcome:#?}"
        );
    }
    assert_eq!(game.face_up_attractions().len(), 3);
    assert_eq!(game.attraction_deck(alice).map(<[_]>::len), Some(0));
    let storybook_id = game
        .face_up_attractions()
        .iter()
        .copied()
        .find(|object| {
            game.object(*object)
                .is_some_and(|object| object.name == "Storybook Ride")
        })
        .expect("Storybook Ride should be face up");

    // A visit by another player is history, but not Alice's history. The two
    // Information Booths Alice controls are also deliberately unlit on 3.
    let bobs_visit = crate::events::RawEvent::new(
        crate::events::KeywordActionEvent::new(
            crate::events::KeywordActionKind::VisitAttraction,
            bob,
            storybook_id,
            1,
        ),
        Default::default(),
    );
    game.turn_store
        .turn_history
        .record_event(&bobs_visit, None, None);

    let filler = CardDefinitionBuilder::new(CardId::from_raw(97_711), "Storybook Filler")
        .card_types(vec![CardType::Instant])
        .build();
    for _ in 0..4 {
        game.create_object_from_definition(&filler, alice, Zone::Library);
    }

    game.force_next_die_roll(3);
    let mut trigger_queue = crate::triggers::TriggerQueue::new();
    let mut runner = crate::turn_runner::TurnRunner::from_state_for_sync(
        crate::turn_runner::TurnState::FirstMain,
    );
    runner
        .advance(&mut game, &mut trigger_queue)
        .expect("the production Attraction visit should run");
    assert_eq!(
        game.turn_store
            .turn_history
            .total_attractions_visited_for_players(&[alice]),
        1,
        "only the lit Storybook Ride was visited by Alice"
    );
    assert_eq!(
        game.turn_store
            .turn_history
            .total_attractions_visited_for_players(&[bob]),
        1,
        "Bob's independent visit should remain player-scoped"
    );
    assert_eq!(
        trigger_queue
            .entries
            .iter()
            .filter(|entry| entry.source == storybook_id)
            .count(),
        1,
        "only Storybook Ride should trigger on the forced 3"
    );

    let mut priority_dm = crate::decision::AutoPassDecisionMaker;
    crate::game_loop::run_priority_loop_with(&mut game, &mut trigger_queue, &mut priority_dm)
        .expect("Storybook Ride's Visit ability should resolve");
    assert_eq!(
        game.player(alice).expect("Alice").library.len(),
        3,
        "Storybook Ride must exile one card, not all three controlled Attractions"
    );
    assert_eq!(game.exile.len(), 1);

    game.turn_store.turn_history.clear_for_new_turn();
    assert_eq!(
        game.turn_store
            .turn_history
            .total_attractions_visited_for_players(&[alice, bob]),
        0,
        "visit counts must reset with turn history"
    );

    // The complementary roll proves that every matching controlled
    // Attraction is visited in one batch, not merely the first match.
    game.force_next_die_roll(6);
    let mut simultaneous_queue = crate::triggers::TriggerQueue::new();
    assert_eq!(
        crate::game_loop::roll_to_visit_attractions(&mut game, &mut simultaneous_queue)
            .expect("the simultaneous visit roll should execute"),
        Some(6)
    );
    assert_eq!(
        game.turn_store
            .turn_history
            .total_attractions_visited_for_players(&[alice]),
        3,
        "all three Attractions have 6 lit and should be visited"
    );
    assert_eq!(
        simultaneous_queue.entries.len(),
        3,
        "each matching Attraction should contribute one Visit trigger"
    );
}

#[test]
fn archelos_keeps_complementary_conditions_on_other_permanent_entry_rules() {
    assert_exact_round_trip(
        "Archelos, Lagoon Mystic",
        "As long as Archelos is tapped, other permanents enter tapped.\nAs long as Archelos is untapped, other permanents enter untapped.",
    );
    let debug = format!(
        "{:#?}",
        parse_oracle_card_definition("Archelos, Lagoon Mystic")
    );
    assert!(debug.contains("EntersTappedForFilter"), "{debug}");
    assert!(debug.contains("EntersUntappedForFilter"), "{debug}");
    assert!(debug.contains("SourceIsTapped"), "{debug}");
    assert!(debug.contains("SourceIsUntapped"), "{debug}");
    assert!(debug.contains("other: true"), "{debug}");
    assert!(
        !debug.contains("GrantAbility"),
        "entry rules must keep native conditional replacement semantics: {debug}"
    );
}

#[test]
fn fumble_preserves_the_former_attachment_set_and_one_new_recipient() {
    assert_exact_round_trip(
        "Fumble",
        "Return target creature to its owner's hand. Gain control of all Auras and Equipment that were attached to it, then attach them to another creature.",
    );
    let debug = format!("{:#?}", parse_oracle_card_definition("Fumble"));
    assert!(
        debug.contains("WasAttachedToTaggedObject"),
        "the controlled set must come from the bounced creature's last-known attachments: {debug}"
    );
    assert!(
        debug.contains("ChangeControllerToEffectController"),
        "{debug}"
    );
    assert!(debug.contains("AttachObjectsEffect"), "{debug}");
    assert!(debug.contains("objects: All("), "{debug}");
    assert_eq!(
        debug.matches("ChooseObjectsEffect").count(),
        1,
        "all former attachments must share one chosen new recipient: {debug}"
    );
    assert!(
        !debug.contains("relation: AttachedToTaggedObject"),
        "past-tense attachment provenance must not widen to current attachment state: {debug}"
    );
}

#[test]
fn gutter_grime_keeps_one_creator_bound_token_cda() {
    assert_exact_round_trip(
        "Gutter Grime",
        "Whenever a nontoken creature you control dies, put a slime counter on this enchantment, then create a green Ooze creature token with \"This token's power and toughness are each equal to the number of slime counters on Gutter Grime.\"",
    );
    let debug = format!("{:#?}", parse_oracle_card_definition("Gutter Grime"));
    assert!(
        debug.contains("CharacteristicDefiningPt"),
        "the Ooze must carry an intrinsic dynamic P/T ability: {debug}"
    );
    assert!(
        debug.contains("FullName(") && debug.contains("\"Gutter Grime\""),
        "the CDA must retain the creating permanent's typed name reference: {debug}"
    );
    assert!(
        !debug.contains("SetBasePowerToughnessEffect"),
        "the creator-bound CDA must not also lower to an X/X fallback: {debug}"
    );
}

// Byte-exact oracle round trips for cards whose former tests pinned internal
// AST shapes / pre-merge render lines. The round trip is the durable contract:
// it fixes the decompiled text against real oracle, while the intermediate
// representation stays free to change.
#[test]
fn commander_liara_portyr_round_trips_to_oracle() {
    assert_exact_round_trip(
        "Commander Liara Portyr",
        "Whenever you attack, spells you cast from exile this turn cost {X} less to cast, where X is the number of players being attacked. Exile the top X cards of your library. Until end of turn, you may cast spells from among those exiled cards.",
    );
}

#[test]
fn communal_brewing_round_trips_to_oracle() {
    assert_exact_round_trip(
        "Communal Brewing",
        "When this enchantment enters, any number of target opponents each draw a card. Put an ingredient counter on this enchantment, then put an ingredient counter on it for each card drawn this way.\nWhenever you cast a creature spell, that creature enters with X additional +1/+1 counters on it, where X is the number of ingredient counters on this enchantment.",
    );
}

#[test]
fn dina_essence_brewer_round_trips_to_oracle() {
    assert_exact_round_trip(
        "Dina, Essence Brewer",
        "Whenever you sacrifice a creature, draw a card. This ability triggers only once each turn.\n{2}, {T}, Sacrifice another creature: You gain X life and put X +1/+1 counters on target creature you control, where X is the sacrificed creature's power.",
    );
}

#[test]
fn forge_boss_round_trips_to_oracle() {
    assert_exact_round_trip(
        "Forge Boss",
        "Whenever you sacrifice one or more other creatures, this creature deals 2 damage to each opponent. This ability triggers only once each turn.",
    );
}

#[test]
fn irresistible_prey_round_trips_to_oracle() {
    assert_exact_round_trip(
        "Irresistible Prey",
        "Target creature must be blocked this turn if able.\nDraw a card.",
    );
}

#[test]
fn kang_prime_round_trips_to_oracle() {
    assert_exact_round_trip(
        "Kang Prime",
        "Flying\nWhenever Kang Prime enters or attacks, exile cards from the top of your library until you exile a nonland card. Put two time counters on that card. If it doesn't have suspend, it gains suspend.",
    );
}

#[test]
fn lucid_dreams_round_trips_to_oracle() {
    assert_exact_round_trip(
        "Lucid Dreams",
        "Draw X cards, where X is the number of card types among cards in your graveyard.",
    );
}

#[test]
fn maskwood_nexus_round_trips_to_oracle() {
    assert_exact_round_trip(
        "Maskwood Nexus",
        "Creatures you control are every creature type. The same is true for creature spells you control and creature cards you own that aren't on the battlefield.\n{3}, {T}: Create a 2/2 blue Shapeshifter creature token with changeling.",
    );
}

#[test]
fn rakdos_the_muscle_round_trips_to_oracle() {
    assert_exact_round_trip(
        "Rakdos, the Muscle",
        "Flying, trample\nWhenever you sacrifice another creature, exile cards equal to its mana value from the top of target player's library. You may play those cards until your next end step, and mana of any type can be spent to cast them.\nSacrifice another creature: Rakdos gains indestructible until end of turn. Tap it. Activate only once each turn.",
    );
}

#[test]
fn sigarda_s_splendor_round_trips_to_oracle() {
    assert_exact_round_trip(
        "Sigarda's Splendor",
        "As this enchantment enters, note your life total.\nAt the beginning of your upkeep, draw a card if your life total is greater than or equal to the last noted life total for this enchantment. Then note your life total.\nWhenever you cast a white spell, you gain 1 life.",
    );
}

#[test]
fn soul_partition_round_trips_to_oracle() {
    assert_exact_round_trip(
        "Soul Partition",
        "Exile target nonland permanent. For as long as that card remains exiled, its owner may play it. A spell cast by an opponent this way costs {2} more to cast.",
    );
}

#[test]
fn well_of_lost_dreams_round_trips_to_oracle() {
    assert_exact_round_trip(
        "Well of Lost Dreams",
        "Whenever you gain life, you may pay {X}, where X is less than or equal to the amount of life you gained. If you do, draw X cards.",
    );
}

#[test]
fn wonderscape_sage_round_trips_to_oracle() {
    assert_exact_round_trip(
        "Wonderscape Sage",
        "Flying\n{T}, Return a land you control to its owner's hand: Draw a card. Then discard a card unless that land had a nonbasic land type.",
    );
}

#[test]
fn thunderous_velocipede_keeps_both_entry_counter_branches() {
    assert_exact_round_trip(
        "Thunderous Velocipede",
        "Trample\nEach other Vehicle and creature you control enters with an additional +1/+1 counter on it if its mana value is 4 or less. Otherwise, it enters with three additional +1/+1 counters on it.\nCrew 3",
    );
}
