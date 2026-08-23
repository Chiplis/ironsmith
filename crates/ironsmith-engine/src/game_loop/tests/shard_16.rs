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
use super::shard_17::*;
use super::*;

#[cfg(ironsmith_runtime_parser_tests)]
#[derive(Default)]
struct FractionalSacrificeDecisionMaker {
    decisions: Vec<(PlayerId, Vec<PlayerId>, usize, Option<usize>)>,
}

#[cfg(ironsmith_runtime_parser_tests)]
impl DecisionMaker for FractionalSacrificeDecisionMaker {
    fn decide_objects(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        let legal = ctx
            .candidates
            .iter()
            .filter(|candidate| candidate.legal)
            .map(|candidate| candidate.id)
            .collect::<Vec<_>>();
        let legal_owners = legal
            .iter()
            .filter_map(|id| game.object(*id).map(|object| object.owner))
            .collect();
        self.decisions
            .push((ctx.player, legal_owners, ctx.min, ctx.max));
        legal.into_iter().take(ctx.max.unwrap_or(ctx.min)).collect()
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
fn fractional_test_permanent(
    game: &mut GameState,
    id: u32,
    name: impl Into<String>,
    controller: PlayerId,
    card_types: Vec<CardType>,
    subtypes: Vec<Subtype>,
) -> ObjectId {
    let card = CardBuilder::new(CardId::from_raw(id), name.into())
        .card_types(card_types)
        .subtypes(subtypes)
        .build();
    game.create_object_from_card(&card, controller, Zone::Battlefield)
}

#[cfg(ironsmith_runtime_parser_tests)]
fn controlled_battlefield_count(game: &GameState, controller: PlayerId) -> usize {
    game.battlefield
        .iter()
        .filter(|&&id| {
            game.object(id)
                .is_some_and(|object| game.controller_of(object) == controller)
        })
        .count()
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn curse_of_the_cabal_targets_only_that_players_permanents_and_rounds_down() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let definition = CardDefinitionBuilder::new(CardId::from_raw(72_970), "Curse of the Cabal")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Target player sacrifices half the permanents they control of their choice, rounded down.",
        )
        .expect("Curse of the Cabal's fractional sacrifice should parse");
    let source = game.create_object_from_definition(&definition, alice, Zone::Stack);

    for index in 0..2 {
        fractional_test_permanent(
            &mut game,
            72_971 + index,
            format!("Alice Curse Permanent {index}"),
            alice,
            vec![CardType::Artifact],
            Vec::new(),
        );
    }
    for index in 0..5 {
        fractional_test_permanent(
            &mut game,
            72_980 + index,
            format!("Bob Curse Permanent {index}"),
            bob,
            vec![CardType::Enchantment],
            Vec::new(),
        );
    }

    let mut dm = FractionalSacrificeDecisionMaker::default();
    let mut ctx = ExecutionContext::new(source, alice, &mut dm)
        .with_targets(vec![ResolvedTarget::Player(bob)])
        .with_target_assignments(vec![crate::game_state::TargetAssignment {
            spec: ChooseSpec::target_player(),
            range: 0..1,
        }]);
    execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        source,
        definition
            .spell_effect
            .as_ref()
            .expect("Curse should have a spell program"),
        None,
        &[],
    )
    .expect("Curse of the Cabal should resolve");

    assert_eq!(controlled_battlefield_count(&game, alice), 2);
    assert_eq!(
        controlled_battlefield_count(&game, bob),
        3,
        "five permanents should require exactly two sacrifices, rounded down"
    );
    let [(chooser, legal_owners, min, max)] = dm.decisions.as_slice() else {
        panic!(
            "Curse should make exactly one sacrifice choice: {:?}",
            dm.decisions
        );
    };
    assert_eq!(*chooser, bob, "the targeted player must make the choice");
    assert_eq!((*min, *max), (2, Some(2)));
    assert!(legal_owners.iter().all(|owner| *owner == bob));
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn tectonic_split_additional_cost_sacrifices_half_the_casters_lands_rounded_up() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let definition = CardDefinitionBuilder::new(CardId::from_raw(73_000), "Tectonic Split")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "As an additional cost to cast this spell, sacrifice half the lands you control, rounded up.",
        )
        .expect("Tectonic Split's additional cost should parse");
    let spell = game.create_object_from_definition(&definition, alice, Zone::Hand);

    for index in 0..5 {
        fractional_test_permanent(
            &mut game,
            73_001 + index,
            format!("Alice Tectonic Land {index}"),
            alice,
            vec![CardType::Land],
            Vec::new(),
        );
    }
    for index in 0..3 {
        fractional_test_permanent(
            &mut game,
            73_010 + index,
            format!("Bob Tectonic Land {index}"),
            bob,
            vec![CardType::Land],
            Vec::new(),
        );
    }

    crate::cost::can_pay_cost_with_reason(
        &game,
        spell,
        alice,
        &definition.additional_cost,
        crate::costs::PaymentReason::CastSpell,
    )
    .expect("ordinary affordability checks must understand the choice-backed sacrifice");

    let mut dm = FractionalSacrificeDecisionMaker::default();
    let mut ctx = ExecutionContext::new(spell, alice, &mut dm);
    crate::special_actions::pay_total_cost_with_choice_in_context(
        &mut game,
        alice,
        spell,
        &definition.additional_cost,
        crate::costs::PaymentReason::CastSpell,
        &mut ctx,
    )
    .expect("Tectonic Split's dynamic additional cost should be payable");

    assert_eq!(
        controlled_battlefield_count(&game, alice),
        2,
        "five lands should require exactly three sacrifices, rounded up"
    );
    assert_eq!(controlled_battlefield_count(&game, bob), 3);
    let [(chooser, legal_owners, min, max)] = dm.decisions.as_slice() else {
        panic!(
            "Tectonic Split should make exactly one sacrifice choice: {:?}",
            dm.decisions
        );
    };
    assert_eq!(*chooser, alice);
    assert_eq!((*min, *max), (3, Some(3)));
    assert_eq!(legal_owners, &vec![alice; 5]);
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn zodiark_each_player_sacrifices_their_own_non_gods_rounded_down() {
    let mut game = setup_three_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);
    let definition = CardDefinitionBuilder::new(CardId::from_raw(73_020), "Zodiark, Umbral God")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::God])
        .parse_text(
            "When this creature enters, each player sacrifices half the non-God creatures they control of their choice, rounded down.",
        )
        .expect("Zodiark's enters trigger should parse");
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);

    for (controller, count, base) in [(alice, 3, 73_021), (bob, 5, 73_030), (charlie, 2, 73_040)] {
        for index in 0..count {
            fractional_test_permanent(
                &mut game,
                base + index,
                format!("Zodiark Non-God {base}-{index}"),
                controller,
                vec![CardType::Creature],
                vec![Subtype::Human],
            );
        }
    }
    fractional_test_permanent(
        &mut game,
        73_050,
        "Bob God Sentinel",
        bob,
        vec![CardType::Creature],
        vec![Subtype::God],
    );

    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Zodiark should have an enters trigger");
    let mut dm = FractionalSacrificeDecisionMaker::default();
    let mut ctx = ExecutionContext::new(source, alice, &mut dm);
    execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        source,
        &triggered.effects,
        None,
        &[],
    )
    .expect("Zodiark's sacrifice trigger should resolve");

    let non_god_count = |game: &GameState, controller: PlayerId| {
        game.battlefield
            .iter()
            .filter(|&&id| {
                game.object(id).is_some_and(|object| {
                    game.controller_of(object) == controller
                        && object.card_types.contains(&CardType::Creature)
                        && !object.subtypes.contains(&Subtype::God)
                })
            })
            .count()
    };
    assert_eq!(non_god_count(&game, alice), 2);
    assert_eq!(non_god_count(&game, bob), 3);
    assert_eq!(non_god_count(&game, charlie), 1);
    assert!(game.battlefield.iter().any(|&id| {
        game.object(id)
            .is_some_and(|object| object.name == "Bob God Sentinel")
    }));
    assert_eq!(
        dm.decisions.len(),
        3,
        "each player must choose independently"
    );
    assert_eq!(
        dm.decisions
            .iter()
            .map(|(player, _, min, max)| (*player, *min, *max))
            .collect::<Vec<_>>(),
        vec![
            (alice, 1, Some(1)),
            (bob, 2, Some(2)),
            (charlie, 1, Some(1)),
        ]
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn unlucky_witness_playing_one_exiled_card_exhausts_the_shared_permission() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;

    let witness = CardDefinitionBuilder::new(CardId::from_raw(73_060), "Unlucky Witness")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .parse_text(
            "When this creature dies, exile the top two cards of your library. Until your next end step, you may play one of those cards.",
        )
        .expect("Unlucky Witness should parse");
    let source = game.create_object_from_definition(&witness, alice, Zone::Battlefield);
    let triggered = witness
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Unlucky Witness should have a dies trigger");
    let grant_max_plays = triggered
        .effects
        .flattened_default_effects()
        .iter()
        .find_map(|effect| {
            effect
                .downcast_ref::<crate::effects::GrantPlayTaggedEffect>()
                .map(|grant| grant.max_plays)
        })
        .expect("Unlucky Witness trigger should contain a tagged play grant");
    assert_eq!(grant_max_plays, Some(1));

    let card = |id, name| {
        CardDefinitionBuilder::new(CardId::from_raw(id), name)
            .card_types(vec![CardType::Instant])
            .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Blue]))
            .build()
    };
    let bottom =
        game.create_object_from_definition(&card(73_061, "Witness Bottom"), alice, Zone::Library);
    let first =
        game.create_object_from_definition(&card(73_062, "Witness First"), alice, Zone::Library);
    let second =
        game.create_object_from_definition(&card(73_063, "Witness Second"), alice, Zone::Library);
    let first_stable = game.object(first).unwrap().stable_id;
    let second_stable = game.object(second).unwrap().stable_id;
    assert!(game.set_player_library_order_with_audit(
        alice,
        vec![bottom, first, second],
        "Unlucky Witness shared permission test",
    ));
    game.player_mut(alice)
        .expect("Alice exists")
        .mana_pool
        .add(ManaSymbol::Blue, 2);

    let mut ctx = ExecutionContext::new_default(source, alice);
    execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        source,
        &triggered.effects,
        None,
        &[],
    )
    .expect("Unlucky Witness trigger should resolve");

    let first_exiled = game
        .find_object_by_stable_id(first_stable)
        .expect("first exiled card should remain tracked");
    let second_exiled = game
        .find_object_by_stable_id(second_stable)
        .expect("second exiled card should remain tracked");
    assert_eq!(game.object(first_exiled).unwrap().zone, Zone::Exile);
    assert_eq!(game.object(second_exiled).unwrap().zone, Zone::Exile);

    let actions = crate::decision::compute_legal_actions(&game, alice);
    let first_method = actions
        .iter()
        .find_map(|action| match action {
            LegalAction::CastSpell {
                spell_id,
                from_zone: Zone::Exile,
                casting_method,
            } if *spell_id == first_exiled => Some(casting_method.clone()),
            _ => None,
        })
        .expect("the first exiled card should initially be castable");
    assert!(actions.iter().any(|action| matches!(
        action,
        LegalAction::CastSpell {
            spell_id,
            from_zone: Zone::Exile,
            ..
        } if *spell_id == second_exiled
    )));

    super::priority_mana::propose_spell_cast(
        &mut game,
        first_exiled,
        Zone::Exile,
        alice,
        &first_method,
    )
    .expect("first exiled card should be proposed through the grant");

    let after_first = crate::decision::compute_legal_actions(&game, alice);
    assert!(
        !after_first.iter().any(|action| matches!(
            action,
            LegalAction::CastSpell {
                spell_id,
                from_zone: Zone::Exile,
                ..
            } if *spell_id == second_exiled
        )),
        "casting either exiled card must exhaust Unlucky Witness's shared one-play permission"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn rivals_duel_rejects_targets_that_share_a_creature_type_and_fights_a_legal_pair() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let duel = CardDefinitionBuilder::new(CardId::from_raw(73_070), "Rivals' Duel")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Choose two target creatures that share no creature types. Those creatures fight each other.",
        )
        .expect("Rivals' Duel should parse");
    let source = game.create_object_from_definition(&duel, alice, Zone::Stack);

    let creature = |id, name, subtypes, power, toughness| {
        CardBuilder::new(CardId::from_raw(id), name)
            .card_types(vec![CardType::Creature])
            .subtypes(subtypes)
            .power_toughness(PowerToughness::fixed(power, toughness))
            .build()
    };
    let human_warrior = game.create_object_from_card(
        &creature(
            73_071,
            "Human Warrior",
            vec![Subtype::Human, Subtype::Warrior],
            2,
            5,
        ),
        alice,
        Zone::Battlefield,
    );
    let human_cleric = game.create_object_from_card(
        &creature(
            73_072,
            "Human Cleric",
            vec![Subtype::Human, Subtype::Cleric],
            3,
            5,
        ),
        alice,
        Zone::Battlefield,
    );
    let goblin_rogue = game.create_object_from_card(
        &creature(
            73_073,
            "Goblin Rogue",
            vec![Subtype::Goblin, Subtype::Rogue],
            4,
            5,
        ),
        alice,
        Zone::Battlefield,
    );

    let program = duel
        .spell_effect
        .as_ref()
        .expect("Rivals' Duel should have a spell program");
    let requirements = super::targeting::extract_target_requirements(
        &game,
        program.flattened_default_effects(),
        alice,
        Some(source),
    );
    assert_eq!(requirements.len(), 1);
    let contexts = requirements
        .iter()
        .map(
            |requirement| crate::decisions::context::TargetRequirementContext {
                description: requirement.description.clone(),
                legal_targets: requirement.legal_targets.clone(),
                legal_target_sets: requirement.legal_target_sets.clone(),
                aggregate_constraint: requirement.aggregate_constraint.clone(),
                min_targets: requirement.min_targets,
                max_targets: requirement.max_targets,
                distinct_player_group: requirement.distinct_player_group,
            },
        )
        .collect::<Vec<_>>();
    assert!(
        !crate::targeting::validate_flat_target_assignment(
            &contexts,
            &[Target::Object(human_warrior), Target::Object(human_cleric)],
        ),
        "two Humans must not be announceable as Rivals' Duel's targets"
    );
    assert!(crate::targeting::validate_flat_target_assignment(
        &contexts,
        &[Target::Object(human_warrior), Target::Object(goblin_rogue)],
    ));

    let mut ctx = ExecutionContext::new_default(source, alice)
        .with_targets(vec![
            ResolvedTarget::Object(human_warrior),
            ResolvedTarget::Object(goblin_rogue),
        ])
        .with_target_assignments(vec![crate::game_state::TargetAssignment {
            spec: requirements[0].spec.clone(),
            range: 0..2,
        }]);
    execute_resolution_program(&mut game, &mut ctx, alice, source, program, None, &[])
        .expect("Rivals' Duel should resolve with a legal pair");
    assert_eq!(game.damage_on(human_warrior), 4);
    assert_eq!(game.damage_on(goblin_rogue), 2);
    assert_eq!(game.damage_on(human_cleric), 0);
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn yawgmoths_will_plays_a_land_and_casts_a_spell_from_graveyard_then_expires_at_cleanup()
{
    use crate::events::processing::ZoneChangeOutcome;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let will = CardDefinitionBuilder::new(CardId::from_raw(73_080), "Yawgmoth's Will")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Black],
        ]))
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Until end of turn, you may play lands and cast spells from your graveyard.\nIf a card would be put into your graveyard from anywhere this turn, exile that card instead.",
        )
        .expect("Yawgmoth's Will should parse");
    let source = game.create_object_from_definition(&will, alice, Zone::Stack);
    let source_stable = game.object(source).expect("Will should exist").stable_id;

    let land = |id, name| {
        CardBuilder::new(CardId::from_raw(id), name)
            .card_types(vec![CardType::Land])
            .build()
    };
    let instant = |id, name| {
        CardBuilder::new(CardId::from_raw(id), name)
            .mana_cost(ManaCost::new())
            .card_types(vec![CardType::Instant])
            .build()
    };
    let replay_land =
        game.create_object_from_card(&land(73_081, "Will Replay Land"), alice, Zone::Graveyard);
    let expiry_land =
        game.create_object_from_card(&land(73_082, "Will Expiry Land"), alice, Zone::Graveyard);
    let replay_spell = game.create_object_from_card(
        &instant(73_083, "Will Replay Spell"),
        alice,
        Zone::Graveyard,
    );
    let expiry_spell = game.create_object_from_card(
        &instant(73_084, "Will Expiry Spell"),
        alice,
        Zone::Graveyard,
    );

    let program = will
        .spell_effect
        .as_ref()
        .expect("Yawgmoth's Will should have a spell program");
    let mut ctx = ExecutionContext::new_default(source, alice);
    execute_resolution_program(&mut game, &mut ctx, alice, source, program, None, &[])
        .expect("Yawgmoth's Will should resolve");
    assert_eq!(
        game.effect_store
            .replacement_effects
            .until_end_of_turn_effects_snapshot()
            .len(),
        1,
        "Yawgmoth's Will should register its cleanup-scoped replacement: {program:#?}"
    );

    let mut dm = SelectFirstDecisionMaker;
    let source_cause = crate::events::cause::EventCause::from_effect(source, alice);
    let source_move = crate::events::processing::process_zone_change(
        &mut game,
        source,
        Zone::Stack,
        Zone::Graveyard,
        source_cause.clone(),
        &mut dm,
    );
    assert!(
        matches!(source_move, ZoneChangeOutcome::Proceed(Zone::Exile)),
        "Yawgmoth's Will should replace its own graveyard move, got {source_move:?}"
    );
    game.move_object(source, Zone::Exile, source_cause)
        .expect("the redirected Yawgmoth's Will move should succeed");
    let exiled_source = game
        .find_object_by_stable_id(source_stable)
        .expect("Yawgmoth's Will should remain tracked after resolution");
    assert_eq!(
        game.object(exiled_source).expect("Will should exist").zone,
        Zone::Exile,
        "Yawgmoth's Will must exile itself through its own replacement effect"
    );

    let actions = crate::decision::compute_legal_actions(&game, alice);
    assert!(actions.iter().any(|action| matches!(
        action,
        LegalAction::PlayLand { land_id } if *land_id == replay_land
    )));
    let replay_method = actions
        .iter()
        .find_map(|action| match action {
            LegalAction::CastSpell {
                spell_id,
                from_zone: Zone::Graveyard,
                casting_method,
            } if *spell_id == replay_spell => Some(casting_method.clone()),
            _ => None,
        })
        .expect("Yawgmoth's Will should let Alice cast a graveyard spell");

    crate::special_actions::perform(
        crate::special_actions::SpecialAction::PlayLand {
            card_id: replay_land,
        },
        &mut game,
        alice,
        &mut dm,
    )
    .expect("Yawgmoth's Will should let Alice play a graveyard land");
    assert!(game.object(replay_land).is_none());
    assert!(game.battlefield.iter().any(|&id| {
        game.object(id)
            .is_some_and(|object| object.name == "Will Replay Land")
    }));

    let stack_spell = super::priority_mana::propose_spell_cast(
        &mut game,
        replay_spell,
        Zone::Graveyard,
        alice,
        &replay_method,
    )
    .expect("Yawgmoth's Will should authorize the graveyard spell proposal");
    assert_eq!(
        game.object(stack_spell)
            .expect("proposed spell should exist")
            .zone,
        Zone::Stack
    );

    assert!(game.effect_store.grant_registry.card_can_play_from_zone(
        &game,
        expiry_land,
        Zone::Graveyard,
        alice,
    ));
    assert!(game.effect_store.grant_registry.card_can_play_from_zone(
        &game,
        expiry_spell,
        Zone::Graveyard,
        alice,
    ));

    crate::turn::execute_cleanup_step(&mut game);
    assert!(
        !game.effect_store.grant_registry.card_can_play_from_zone(
            &game,
            expiry_land,
            Zone::Graveyard,
            alice,
        ),
        "the graveyard land permission must end during cleanup"
    );
    assert!(
        !game.effect_store.grant_registry.card_can_play_from_zone(
            &game,
            expiry_spell,
            Zone::Graveyard,
            alice,
        ),
        "the graveyard spell permission must end during cleanup"
    );

    let post_cleanup_card = game.create_object_from_card(
        &CardBuilder::new(CardId::from_raw(73_085), "Post-Cleanup Creature")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(1, 1))
            .build(),
        alice,
        Zone::Battlefield,
    );
    let post_cleanup_stable = game
        .object(post_cleanup_card)
        .expect("post-cleanup creature should exist")
        .stable_id;
    let post_cleanup_cause = crate::events::cause::EventCause::from_sba();
    let post_cleanup_move = crate::events::processing::process_zone_change(
        &mut game,
        post_cleanup_card,
        Zone::Battlefield,
        Zone::Graveyard,
        post_cleanup_cause.clone(),
        &mut dm,
    );
    assert!(matches!(
        post_cleanup_move,
        ZoneChangeOutcome::Proceed(Zone::Graveyard)
    ));
    game.move_object(post_cleanup_card, Zone::Graveyard, post_cleanup_cause)
        .expect("the ordinary post-cleanup graveyard move should succeed");
    let graveyard_card = game
        .find_object_by_stable_id(post_cleanup_stable)
        .expect("post-cleanup creature should remain tracked");
    assert_eq!(
        game.object(graveyard_card)
            .expect("post-cleanup creature should exist")
            .zone,
        Zone::Graveyard,
        "the exile replacement must also end during cleanup"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn varolz_grants_recipient_specific_scavenge_costs_and_the_ability_resolves() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let grant = CardDefinitionBuilder::new(CardId::new(), "Dynamic Scavenge Grant")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .parse_text(
            "Each creature card in your graveyard has scavenge. The scavenge cost is equal to its mana cost.",
        )
        .expect("recipient-derived scavenge grant should parse");
    game.create_object_from_definition(&grant, alice, Zone::Battlefield);

    let scavenge_card = CardBuilder::new(CardId::new(), "Four-Power Scavenger")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Green],
        ]))
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(4, 4))
        .build();
    let source = game.create_object_from_card(&scavenge_card, alice, Zone::Graveyard);
    let source_stable_id = game.object(source).expect("source exists").stable_id;
    let no_mana_card = CardBuilder::new(CardId::new(), "Mana-Costless Scavenger")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 3))
        .build();
    let no_mana_source = game.create_object_from_card(&no_mana_card, alice, Zone::Graveyard);
    let target_card = CardBuilder::new(CardId::new(), "Scavenge Target")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let target = game.create_object_from_card(&target_card, alice, Zone::Battlefield);

    let calculated = game
        .calculated_characteristics(source)
        .expect("graveyard recipient should have calculated characteristics");
    let (ability_index, activated) = calculated
        .abilities
        .iter()
        .enumerate()
        .find_map(|(index, ability)| match &ability.kind {
            AbilityKind::Activated(activated) if ability.functions_in(&Zone::Graveyard) => {
                Some((index, activated.clone()))
            }
            _ => None,
        })
        .expect("the creature card should receive a graveyard scavenge ability");
    assert!(
        game.calculated_characteristics(no_mana_source)
            .expect("mana-costless recipient should still be calculable")
            .abilities
            .iter()
            .any(|ability| matches!(ability.kind, AbilityKind::Activated(_))),
        "the grant is structural even though a missing mana cost makes activation unpayable"
    );

    {
        let player = game.player_mut(alice).expect("Alice exists");
        player.mana_pool.add(ManaSymbol::Colorless, 2);
        player.mana_pool.add(ManaSymbol::Green, 1);
    }
    let legal = compute_legal_actions(&game, alice);
    assert!(
        legal.iter().any(|action| matches!(
            action,
            LegalAction::ActivateAbility { source: candidate, ability_index: index }
                if *candidate == source && *index == ability_index
        )),
        "{{2}}{{G}} must be resolved from this recipient's own mana cost"
    );
    assert!(
        !legal.iter().any(|action| matches!(
            action,
            LegalAction::ActivateAbility { source: candidate, .. }
                if *candidate == no_mana_source
        )),
        "a recipient with no mana cost must not expose a payable scavenge action"
    );

    let snapshot = ObjectSnapshot::from_object(
        game.object(source).expect("source exists before paying"),
        &game,
    );
    let mut decision_maker = SelectFirstDecisionMaker;
    let mut ctx = ExecutionContext::new(source, alice, &mut decision_maker)
        .with_targets(vec![ResolvedTarget::Object(target)])
        .with_source_snapshot(snapshot);
    crate::special_actions::pay_total_cost_with_choice_in_context(
        &mut game,
        alice,
        source,
        &activated.mana_cost,
        crate::costs::PaymentReason::ActivateAbility,
        &mut ctx,
    )
    .expect("the recipient-derived colored mana cost and exile-self cost should be payable");
    for effect in activated.effects.flattened_default_effects() {
        execute_effect(&mut game, effect, &mut ctx).expect("scavenge effect should resolve");
    }

    let moved_source = game
        .find_object_by_stable_id(source_stable_id)
        .and_then(|id| game.object(id))
        .expect("paid scavenge source should still exist in exile");
    assert_eq!(moved_source.zone, Zone::Exile);
    assert_eq!(
        game.counter_count(target, CounterType::PlusOnePlusOne),
        4,
        "the granted ability must use the exiled recipient's own power"
    );
    assert_eq!(
        game.player(alice).expect("Alice exists").mana_pool.total(),
        0,
        "the recipient's exact {{2}}{{G}} mana cost should be paid"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn tron_mana_replacements_require_each_full_compound_urzas_subtype() {
    fn mana_produced(text: &str, companion_subtypes: &[Vec<Subtype>]) -> u32 {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let definition = CardDefinitionBuilder::new(CardId::new(), "Tron Runtime Probe")
            .card_types(vec![CardType::Land])
            .parse_text(text)
            .expect("Tron conditional mana ability should parse");
        let program = definition
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Activated(activated) => Some(activated.effects.clone()),
                _ => None,
            })
            .expect("Tron land should have an activated mana program");
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);

        for (index, subtypes) in companion_subtypes.iter().enumerate() {
            let companion = CardBuilder::new(
                CardId::from_raw(91_100 + index as u32),
                format!("Tron Companion {index}"),
            )
            .card_types(vec![CardType::Land])
            .subtypes(subtypes.clone())
            .build();
            game.create_object_from_card(&companion, alice, Zone::Battlefield);
        }

        let mut decision_maker = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(source, alice, &mut decision_maker);
        execute_resolution_program(&mut game, &mut ctx, alice, source, &program, None, &[])
            .expect("Tron mana program should execute");
        game.player(alice).expect("Alice exists").mana_pool.total()
    }

    let tower = "{T}: Add {C}. If you control an Urza's Mine and an Urza's Power-Plant, add {C}{C}{C} instead.";
    assert_eq!(
        mana_produced(
            tower,
            &[
                vec![Subtype::Urzas, Subtype::Mine],
                vec![Subtype::Urzas, Subtype::Plant],
            ],
        ),
        1,
        "an Urza's Plant is not an Urza's Power-Plant"
    );
    assert_eq!(
        mana_produced(
            tower,
            &[
                vec![Subtype::Urzas, Subtype::Mine],
                vec![Subtype::Urzas, Subtype::PowerPlant],
            ],
        ),
        3
    );

    let mine = "{T}: Add {C}. If you control an Urza's Power-Plant and an Urza's Tower, add {C}{C} instead.";
    assert_eq!(
        mana_produced(
            mine,
            &[
                vec![Subtype::Urzas, Subtype::PowerPlant],
                vec![Subtype::Urzas],
            ],
        ),
        1,
        "the Urza's subtype alone is not an Urza's Tower"
    );
    assert_eq!(
        mana_produced(
            mine,
            &[
                vec![Subtype::Urzas, Subtype::PowerPlant],
                vec![Subtype::Urzas, Subtype::Tower],
            ],
        ),
        2
    );

    let power_plant =
        "{T}: Add {C}. If you control an Urza's Mine and an Urza's Tower, add {C}{C} instead.";
    assert_eq!(
        mana_produced(
            power_plant,
            &[vec![Subtype::Urzas], vec![Subtype::Urzas, Subtype::Tower],],
        ),
        1,
        "the Urza's subtype alone is not an Urza's Mine"
    );
    assert_eq!(
        mana_produced(
            power_plant,
            &[
                vec![Subtype::Urzas, Subtype::Mine],
                vec![Subtype::Urzas, Subtype::Tower],
            ],
        ),
        2
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn raid_and_threshold_damage_replacements_bypass_prevention() {
    fn resolve_damage_probe(name: &str, text: &str, graveyard_cards: usize, attacked: bool) -> i32 {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let definition = CardDefinitionBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Sorcery])
            .parse_text(text)
            .expect("conditional unpreventable damage spell should parse");
        let spell = game.create_object_from_definition(&definition, alice, Zone::Stack);
        for index in 0..graveyard_cards {
            let fodder = CardBuilder::new(
                CardId::from_raw(91_200 + index as u32),
                format!("Threshold Fodder {index}"),
            )
            .card_types(vec![CardType::Creature])
            .build();
            game.create_object_from_card(&fodder, alice, Zone::Graveyard);
        }
        if attacked {
            game.turn_store
                .turn_history
                .players_attacked_this_turn
                .insert(alice);
        }

        let mut decision_maker = SelectFirstDecisionMaker;
        {
            let shield = crate::effects::PreventNextTimeDamageEffect::new(
                crate::effects::PreventNextTimeDamageSource::Filter(
                    crate::target::ObjectFilter::default(),
                ),
                crate::effects::PreventNextTimeDamageTarget::AnyTarget,
            );
            let mut shield_ctx = ExecutionContext::new(spell, alice, &mut decision_maker);
            execute_effect(&mut game, &Effect::new(shield), &mut shield_ctx)
                .expect("damage-prevention shield should register");
        }
        {
            let mut ctx = ExecutionContext::new(spell, alice, &mut decision_maker)
                .with_targets(vec![ResolvedTarget::Player(bob)]);
            execute_resolution_program(
                &mut game,
                &mut ctx,
                alice,
                spell,
                definition
                    .spell_effect
                    .as_ref()
                    .expect("damage spell should have a resolution program"),
                None,
                &[],
            )
            .expect("conditional damage spell should resolve");
        }
        game.player(bob).expect("Bob exists").life
    }

    assert_eq!(
        resolve_damage_probe(
            "Arrow Storm",
            "Arrow Storm deals 4 damage to any target. Raid — If you attacked this turn, instead Arrow Storm deals 5 damage to that permanent or player and the damage can't be prevented.",
            0,
            true,
        ),
        15,
        "Arrow Storm's raid branch must deal all 5 damage through prevention"
    );
    assert_eq!(
        resolve_damage_probe(
            "Lightning Surge",
            "Lightning Surge deals 4 damage to any target. Threshold — If there are seven or more cards in your graveyard, instead Lightning Surge deals 6 damage to that permanent or player and the damage can't be prevented.",
            7,
            false,
        ),
        14,
        "Lightning Surge's threshold branch must deal all 6 damage through prevention"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn throw_from_the_saddle_replaces_only_the_pump_then_deals_power_damage() {
    fn resolve_throw(is_mount: bool) -> (u32, Option<i32>, u32) {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let definition = CardDefinitionBuilder::new(CardId::new(), "Throw from the Saddle")
            .card_types(vec![CardType::Sorcery])
            .parse_text(
                "Target creature you control gets +1/+1 until end of turn. Put a +1/+1 counter on it instead if it's a Mount. Then it deals damage equal to its power to target creature you don't control.",
            )
            .expect("Throw from the Saddle should parse");
        let spell = game.create_object_from_definition(&definition, alice, Zone::Stack);
        let mut subtypes = Vec::new();
        if is_mount {
            subtypes.push(Subtype::Mount);
        }
        let rider = CardBuilder::new(CardId::new(), "Throw Rider")
            .card_types(vec![CardType::Creature])
            .subtypes(subtypes)
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let rider = game.create_object_from_card(&rider, alice, Zone::Battlefield);
        let victim = CardBuilder::new(CardId::new(), "Throw Victim")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(0, 10))
            .build();
        let victim = game.create_object_from_card(&victim, bob, Zone::Battlefield);

        let mut decision_maker = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(spell, alice, &mut decision_maker).with_targets(vec![
            ResolvedTarget::Object(rider),
            ResolvedTarget::Object(victim),
        ]);
        execute_resolution_program(
            &mut game,
            &mut ctx,
            alice,
            spell,
            definition
                .spell_effect
                .as_ref()
                .expect("Throw should have a spell program"),
            None,
            &[],
        )
        .expect("Throw from the Saddle should resolve");

        (
            game.counter_count(rider, CounterType::PlusOnePlusOne),
            game.calculated_power(rider),
            game.damage_on(victim),
        )
    }

    assert_eq!(resolve_throw(false), (0, Some(3), 3));
    assert_eq!(
        resolve_throw(true),
        (1, Some(3), 3),
        "a Mount gets a permanent counter instead of the temporary pump, then deals damage"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn nissas_pilgrimage_preserves_partition_and_spell_mastery_changes_only_search_count() {
    fn resolve_pilgrimage(spell_mastery: bool) -> (usize, usize, usize, bool) {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let definition = CardDefinitionBuilder::new(CardId::new(), "Nissa's Pilgrimage")
            .card_types(vec![CardType::Sorcery])
            .parse_text(
                "Search your library for up to two basic Forest cards, reveal those cards, and put one onto the battlefield tapped and the rest into your hand. Then shuffle.\nSpell mastery — If there are two or more instant and/or sorcery cards in your graveyard, search your library for up to three basic Forest cards instead of two.",
            )
            .expect("Nissa's Pilgrimage should parse");
        let spell = game.create_object_from_definition(&definition, alice, Zone::Stack);
        for index in 0..3 {
            let forest = CardBuilder::new(CardId::from_raw(91_300 + index), "Pilgrimage Forest")
                .supertypes(vec![Supertype::Basic])
                .card_types(vec![CardType::Land])
                .subtypes(vec![Subtype::Forest])
                .build();
            game.create_object_from_card(&forest, alice, Zone::Library);
        }
        if spell_mastery {
            for (index, card_type) in [CardType::Instant, CardType::Sorcery]
                .into_iter()
                .enumerate()
            {
                let card = CardBuilder::new(
                    CardId::from_raw(91_310 + index as u32),
                    format!("Mastery Card {index}"),
                )
                .card_types(vec![card_type])
                .build();
                game.create_object_from_card(&card, alice, Zone::Graveyard);
            }
        }

        let mut decision_maker = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(spell, alice, &mut decision_maker);
        execute_resolution_program(
            &mut game,
            &mut ctx,
            alice,
            spell,
            definition
                .spell_effect
                .as_ref()
                .expect("Pilgrimage should have a spell program"),
            None,
            &[],
        )
        .expect("Nissa's Pilgrimage should resolve");

        let forests_in = |zone| count_named_objects_in_zone(&game, zone, "Pilgrimage Forest");
        let battlefield_forests = game
            .objects_in_zone(Zone::Battlefield)
            .into_iter()
            .filter(|id| {
                game.object(*id)
                    .is_some_and(|object| object.name == "Pilgrimage Forest")
            })
            .collect::<Vec<_>>();
        (
            forests_in(Zone::Battlefield),
            forests_in(Zone::Hand),
            forests_in(Zone::Library),
            battlefield_forests
                .first()
                .is_some_and(|forest| game.is_tapped(*forest)),
        )
    }

    assert_eq!(resolve_pilgrimage(false), (1, 1, 1, true));
    assert_eq!(
        resolve_pilgrimage(true),
        (1, 2, 0, true),
        "spell mastery should select three while preserving one tapped battlefield card and the rest-to-hand partition"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn bartel_runeaxe_strict_parser_and_compiled_text_regression() {
    let def = bartel_runeaxe_definition();
    let rendered = crate::runtime_display::unprocessed_compiled_lines(&def).join("\n");

    assert!(
        rendered.contains("Vigilance"),
        "Bartel Runeaxe should render vigilance, got {rendered}"
    );
    assert!(
        rendered.contains("Bartel Runeaxe can't be the target of Aura spells."),
        "Bartel Runeaxe should render its Aura-spell targeting restriction, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn bartel_runeaxe_blocks_aura_spell_targets_but_not_other_spells_and_has_vigilance() {
    use crate::target::{ChooseSpec, ObjectFilter};
    use crate::targeting::{
        TargetingInvalidReason, TargetingResult, can_target_object, compute_legal_targets,
    };

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let bartel = bartel_runeaxe_definition();
    let bartel_id = game.create_object_from_definition(&bartel, alice, Zone::Battlefield);
    game.update_cant_effects();

    let aura_spell = CardDefinitionBuilder::new(CardId::new(), "Targeting Aura")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Aura])
        .build();
    let aura_spell_id = game.create_object_from_definition(&aura_spell, bob, Zone::Stack);
    game.push_to_stack(StackEntry::new(aura_spell_id, bob));
    assert!(
        matches!(
            can_target_object(&game, bartel_id, aura_spell_id, bob),
            TargetingResult::Invalid(TargetingInvalidReason::CantBeTargeted)
        ),
        "an opponent's Aura spell should not be able to target Bartel Runeaxe"
    );

    let friendly_aura_spell_id =
        game.create_object_from_definition(&aura_spell, alice, Zone::Stack);
    game.push_to_stack(StackEntry::new(friendly_aura_spell_id, alice));
    assert!(
        matches!(
            can_target_object(&game, bartel_id, friendly_aura_spell_id, alice),
            TargetingResult::Invalid(TargetingInvalidReason::CantBeTargeted)
        ),
        "Bartel Runeaxe's Aura-spell restriction should not depend on controller"
    );

    let non_aura_spell = CardDefinitionBuilder::new(CardId::new(), "Targeting Enchantment")
        .card_types(vec![CardType::Enchantment])
        .build();
    let non_aura_spell_id = game.create_object_from_definition(&non_aura_spell, bob, Zone::Stack);
    game.push_to_stack(StackEntry::new(non_aura_spell_id, bob));
    assert!(
        can_target_object(&game, bartel_id, non_aura_spell_id, bob).is_legal(),
        "a non-Aura spell should be able to target Bartel Runeaxe"
    );

    let legal_targets_from_aura = compute_legal_targets(
        &game,
        &ChooseSpec::Target(Box::new(ChooseSpec::Object(ObjectFilter::creature()))),
        bob,
        Some(aura_spell_id),
    );
    assert!(
        !legal_targets_from_aura.contains(&Target::Object(bartel_id)),
        "Bartel Runeaxe should not appear in legal target lists for Aura spells"
    );

    let legal_targets_from_non_aura = compute_legal_targets(
        &game,
        &ChooseSpec::Target(Box::new(ChooseSpec::Object(ObjectFilter::creature()))),
        bob,
        Some(non_aura_spell_id),
    );
    assert!(
        legal_targets_from_non_aura.contains(&Target::Object(bartel_id)),
        "Bartel Runeaxe should appear in legal target lists for non-Aura spells"
    );

    game.remove_summoning_sickness(bartel_id);
    game.turn.active_player = alice;
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(crate::game_state::Step::DeclareAttackers);
    let mut combat = CombatState::default();
    let mut trigger_queue = TriggerQueue::new();
    apply_attacker_declarations(
        &mut game,
        &mut combat,
        &mut trigger_queue,
        &[AttackerDeclaration {
            creature: bartel_id,
            target: AttackTarget::Player(bob),
        }],
    )
    .expect("Bartel Runeaxe should be able to attack");
    assert!(
        !game.is_tapped(bartel_id),
        "vigilance should keep Bartel Runeaxe untapped as it attacks"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn cultivator_colossus_etb_only_asks_may_once_per_land_put() {
    use crate::cards::definitions::{basic_forest, grizzly_bears};
    use crate::effects::{ExecutionContext, execute_effect};
    use crate::ids::ObjectId;

    #[derive(Default)]
    struct CountCultivatorChoices {
        boolean_calls: usize,
        object_calls: usize,
    }

    impl DecisionMaker for CountCultivatorChoices {
        fn decide_boolean(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::BooleanContext,
        ) -> bool {
            self.boolean_calls += 1;
            self.boolean_calls <= 2
        }

        fn decide_objects(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            self.object_calls += 1;
            ctx.candidates
                .iter()
                .filter(|candidate| candidate.legal)
                .map(|candidate| candidate.id)
                .take(1)
                .collect()
        }
    }

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let cultivator = CardDefinitionBuilder::new(CardId::new(), "Cultivator Colossus")
            .card_types(vec![CardType::Creature])
            .parse_text(
                "When this creature enters, you may put a land card from your hand onto the battlefield tapped. If you do, draw a card and repeat this process.",
            )
            .expect("Cultivator Colossus ETB text should parse");
    let rendered = crate::runtime_display::unprocessed_compiled_lines(&cultivator)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
            rendered.contains(
                "when this creature enters, you may put a land card from your hand onto the battlefield tapped. if you do, draw a card and repeat this process"
            ),
            "compiled text should preserve Cultivator Colossus wording, got {rendered}"
        );
    let source_id = game.create_object_from_definition(&cultivator, alice, Zone::Battlefield);
    game.create_object_from_definition(&basic_forest(), alice, Zone::Hand);
    game.create_object_from_definition(&basic_forest(), alice, Zone::Hand);
    game.create_object_from_definition(&grizzly_bears(), alice, Zone::Library);
    game.create_object_from_definition(&grizzly_bears(), alice, Zone::Library);

    let triggered = cultivator
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Cultivator Colossus should have an ETB trigger");

    let mut dm = CountCultivatorChoices::default();
    let mut ctx = ExecutionContext::new_default(source_id, alice).with_decision_maker(&mut dm);

    for effect in &triggered.effects {
        execute_effect(&mut game, effect, &mut ctx).expect("Cultivator ETB should resolve");
    }

    assert_eq!(
        dm.object_calls, 1,
        "the second land should be auto-selected once only one legal choice remains"
    );
    assert_eq!(
        dm.boolean_calls, 3,
        "two accepted iterations should require two yes decisions and one final no"
    );
    let battlefield_forest_count = game
        .battlefield
        .iter()
        .filter_map(|&id| game.object(id))
        .filter(|obj| game.controller_of(obj) == alice && obj.name == "Forest")
        .count();
    assert_eq!(
        battlefield_forest_count, 2,
        "Cultivator Colossus should still put both lands onto the battlefield"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn voices_from_the_void_discards_one_card_per_basic_land_type() {
    use crate::cards::definitions::{basic_forest, basic_island, basic_swamp};
    use crate::game_loop::resolve_stack_entry_with_dm_and_triggers;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);

    let voices = CardDefinitionBuilder::new(CardId::new(), "Voices from the Void")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(4)],
            vec![ManaSymbol::Black],
        ]))
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Domain — Target player discards a card for each basic land type among lands you control.",
        )
        .expect("Voices from the Void should parse");
    let spell_id = game.create_object_from_definition(&voices, alice, Zone::Stack);
    game.push_to_stack(StackEntry::new(spell_id, alice).with_targets(vec![Target::Player(bob)]));

    game.create_object_from_definition(&basic_forest(), alice, Zone::Battlefield);
    game.create_object_from_definition(&basic_island(), alice, Zone::Battlefield);
    game.create_object_from_definition(&basic_swamp(), alice, Zone::Battlefield);

    for index in 0..4 {
        let filler = CardBuilder::new(
            CardId::from_raw(93_000 + index as u32),
            format!("Voices Filler {index}"),
        )
        .card_types(vec![CardType::Artifact])
        .build();
        game.create_object_from_card(&filler, bob, Zone::Hand);
    }

    let hand_before = game.player(bob).expect("bob exists").hand.len();
    let mut trigger_queue = TriggerQueue::new();
    let mut dm = SelectFirstDecisionMaker;

    resolve_stack_entry_with_dm_and_triggers(&mut game, &mut dm, &mut trigger_queue)
        .expect("Voices from the Void should resolve");

    assert_eq!(
        game.player(bob).expect("bob exists").hand.len(),
        hand_before - 3,
        "Voices from the Void should make Bob discard once per basic land type Alice controls"
    );
    assert_eq!(
        game.player(bob).expect("bob exists").graveyard.len(),
        3,
        "the discard count should match the three basic land types on Alice's battlefield"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn careful_consideration_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Careful Consideration")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Blue],
            vec![ManaSymbol::Blue],
        ]))
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Target player draws four cards, then discards three cards. If you cast this spell during your main phase, instead that player draws four cards, then discards two cards.",
        )
        .expect("Careful Consideration should parse")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn resolve_careful_consideration_targeting_bob(main_phase: bool) -> GameState {
    use crate::cost::OptionalCostsPaid;
    use crate::game_loop::resolve_stack_entry_with_dm_and_triggers;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let def = careful_consideration_definition();
    let spell_id = game.create_object_from_definition(&def, alice, Zone::Stack);

    put_test_cards_in_zone(&mut game, bob, Zone::Library, 4);
    put_test_cards_in_zone(&mut game, bob, Zone::Hand, 3);

    let mut entry = StackEntry::new(spell_id, alice).with_targets(vec![Target::Player(bob)]);
    if main_phase {
        let mut paid = OptionalCostsPaid::default();
        paid.mark_label_paid("CastDuringYourMainPhase");
        game.object_mut(spell_id)
            .expect("Careful Consideration spell should exist")
            .optional_costs_paid = paid.clone();
        entry = entry.with_optional_costs_paid(paid);
    }
    game.push_to_stack(entry);

    let mut trigger_queue = TriggerQueue::new();
    let mut dm = SelectFirstDecisionMaker;
    resolve_stack_entry_with_dm_and_triggers(&mut game, &mut dm, &mut trigger_queue)
        .expect("Careful Consideration should resolve");
    game
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn careful_consideration_non_main_phase_target_draws_four_discards_three() {
    let game = resolve_careful_consideration_targeting_bob(false);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    assert_eq!(
        game.player(bob).expect("bob exists").hand.len(),
        4,
        "Bob should net one card after drawing four and discarding three"
    );
    assert_eq!(
        game.player(bob).expect("bob exists").graveyard.len(),
        3,
        "Bob should discard three cards outside Alice's main phase"
    );
    assert_eq!(
        game.player(alice).expect("alice exists").hand.len(),
        0,
        "Careful Consideration should affect the targeted player, not its controller"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn careful_consideration_main_phase_replacement_discards_two_instead() {
    let game = resolve_careful_consideration_targeting_bob(true);
    let bob = PlayerId::from_index(1);

    assert_eq!(
        game.player(bob).expect("bob exists").hand.len(),
        5,
        "Bob should net two cards when the main-phase replacement applies"
    );
    assert_eq!(
        game.player(bob).expect("bob exists").graveyard.len(),
        2,
        "the main-phase branch should discard two cards instead of three"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn tromp_the_domains_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Tromp the Domains")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(5)],
            vec![ManaSymbol::Green],
        ]))
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Domain — Until end of turn, creatures you control gain trample and get +1/+1 for each basic land type among lands you control.",
        )
        .expect("Tromp the Domains should parse")
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn tromp_the_domains_grants_trample_and_distinct_domain_pump_until_cleanup() {
    use crate::cards::definitions::{basic_forest, basic_island};
    use crate::game_loop::resolve_stack_entry_with_dm_and_triggers;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);

    let tromp = tromp_the_domains_definition();
    let spell_id = game.create_object_from_definition(&tromp, alice, Zone::Stack);
    game.push_to_stack(StackEntry::new(spell_id, alice));

    game.create_object_from_definition(&basic_forest(), alice, Zone::Battlefield);
    game.create_object_from_definition(&basic_forest(), alice, Zone::Battlefield);
    game.create_object_from_definition(&basic_island(), alice, Zone::Battlefield);

    let alice_creature = create_creature(&mut game, "Tromping Bear", alice, 2, 2);
    let bob_creature = create_creature(&mut game, "Untromped Bear", bob, 2, 2);

    let mut trigger_queue = TriggerQueue::new();
    let mut dm = SelectFirstDecisionMaker;

    resolve_stack_entry_with_dm_and_triggers(&mut game, &mut dm, &mut trigger_queue)
        .expect("Tromp the Domains should resolve");
    game.refresh_continuous_state();

    assert_eq!(
        game.calculated_power(alice_creature),
        Some(4),
        "Tromp should count distinct basic land types, not total lands"
    );
    assert_eq!(game.calculated_toughness(alice_creature), Some(4));
    assert!(
        game.current_has_static_ability_id(
            alice_creature,
            crate::static_abilities::StaticAbilityId::Trample,
        ),
        "Tromp should grant trample to creatures you control"
    );
    assert_eq!(
        game.calculated_power(bob_creature),
        Some(2),
        "Tromp should not pump opponents' creatures"
    );
    assert!(
        !game.current_has_static_ability_id(
            bob_creature,
            crate::static_abilities::StaticAbilityId::Trample,
        ),
        "Tromp should not grant trample to opponents' creatures"
    );

    execute_cleanup_step(&mut game);
    game.refresh_continuous_state();

    assert_eq!(game.calculated_power(alice_creature), Some(2));
    assert_eq!(game.calculated_toughness(alice_creature), Some(2));
    assert!(
        !game.current_has_static_ability_id(
            alice_creature,
            crate::static_abilities::StaticAbilityId::Trample,
        ),
        "Tromp's effects should expire during cleanup"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn atraxa_grand_unifier_puts_one_card_per_type_into_hand_and_bottoms_the_rest() {
    use crate::effects::ExecutionContext;
    use crate::types::Supertype;

    fn build_library_card(id: u32, name: &str, card_type: CardType) -> crate::card::Card {
        let mut builder = CardBuilder::new(CardId::from_raw(id), name).card_types(vec![card_type]);
        match card_type {
            CardType::Creature => {
                builder = builder.power_toughness(PowerToughness::fixed(1, 1));
            }
            CardType::Planeswalker => {
                builder = builder.loyalty(4);
            }
            CardType::Battle => {
                builder = builder.defense(3);
            }
            _ => {}
        }
        builder.build()
    }

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let def = CardDefinitionBuilder::new(CardId::from_raw(91_000), "Atraxa, Grand Unifier")
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

    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Atraxa should have a triggered ability");

    let source_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

    game.create_object_from_card(
        &build_library_card(91_001, "Bottom Artifact", CardType::Artifact),
        alice,
        Zone::Library,
    );
    game.create_object_from_card(
        &build_library_card(91_002, "Bottom Land", CardType::Land),
        alice,
        Zone::Library,
    );
    game.create_object_from_card(
        &build_library_card(91_003, "Top Sorcery", CardType::Sorcery),
        alice,
        Zone::Library,
    );
    game.create_object_from_card(
        &build_library_card(91_004, "Top Planeswalker", CardType::Planeswalker),
        alice,
        Zone::Library,
    );
    game.create_object_from_card(
        &build_library_card(91_005, "Top Land", CardType::Land),
        alice,
        Zone::Library,
    );
    game.create_object_from_card(
        &build_library_card(91_006, "Top Instant", CardType::Instant),
        alice,
        Zone::Library,
    );
    game.create_object_from_card(
        &build_library_card(91_007, "Top Enchantment", CardType::Enchantment),
        alice,
        Zone::Library,
    );
    game.create_object_from_card(
        &build_library_card(91_008, "Top Creature", CardType::Creature),
        alice,
        Zone::Library,
    );
    game.create_object_from_card(
        &build_library_card(91_009, "Top Battle", CardType::Battle),
        alice,
        Zone::Library,
    );
    game.create_object_from_card(
        &build_library_card(91_010, "Top Artifact", CardType::Artifact),
        alice,
        Zone::Library,
    );

    let mut dm = SelectFirstDecisionMaker;
    let mut ctx = ExecutionContext::new(source_id, alice, &mut dm);
    for effect in &triggered.effects {
        crate::effects::execute_effect(&mut game, effect, &mut ctx)
            .expect("Atraxa ETB effect should resolve");
    }

    assert_eq!(
        game.player(alice).expect("alice exists").hand.len(),
        8,
        "Atraxa should put one card of each standard card type into hand"
    );
    assert!(
        game.player(alice)
            .expect("alice exists")
            .graveyard
            .is_empty(),
        "Atraxa should put the unrevealed remainder on the library bottom, not into the graveyard"
    );
    assert_eq!(
        game.player(alice).expect("alice exists").library.len(),
        2,
        "two duplicate cards should remain on the bottom of the library"
    );
    for card_type in [
        CardType::Artifact,
        CardType::Battle,
        CardType::Creature,
        CardType::Enchantment,
        CardType::Instant,
        CardType::Land,
        CardType::Planeswalker,
        CardType::Sorcery,
    ] {
        assert_eq!(
            game.player(alice)
                .expect("alice exists")
                .hand
                .iter()
                .filter(|&&id| game
                    .object(id)
                    .is_some_and(|obj| obj.has_card_type(card_type)))
                .count(),
            1,
            "Atraxa should place exactly one {card_type:?} card into hand"
        );
    }
    assert!(
        game.object(source_id)
            .is_some_and(|obj| obj.zone == Zone::Battlefield),
        "Atraxa itself should stay on the battlefield after its trigger resolves"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn covert_technician_combat_damage_trigger_puts_only_artifact_with_mana_value_up_to_damage()
 {
    use crate::effects::{ExecutionContext, execute_effect};
    use crate::ids::ObjectId;

    struct ChooseArtifactDecisionMaker {
        accept_may: bool,
        chosen: Option<ObjectId>,
    }

    impl DecisionMaker for ChooseArtifactDecisionMaker {
        fn decide_boolean(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::BooleanContext,
        ) -> bool {
            self.accept_may
        }

        fn decide_objects(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            let choice = ctx
                .candidates
                .iter()
                .find(|candidate| candidate.legal)
                .map(|candidate| candidate.id);
            self.chosen = choice;
            choice.into_iter().collect()
        }
    }

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let covert_technician = CardDefinitionBuilder::new(CardId::new(), "Covert Technician")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::Blue],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![crate::types::Subtype::Vedalken, crate::types::Subtype::Ninja])
        .power_toughness(PowerToughness::fixed(2, 4))
        .parse_text(
            "Ninjutsu {1}{U} ({1}{U}, Return an unblocked attacker you control to hand: Put this card onto the battlefield from your hand tapped and attacking.)\nWhenever Covert Technician deals combat damage to a player, you may put an artifact card with mana value less than or equal to that damage from your hand onto the battlefield.",
        )
        .expect("Covert Technician should parse");

    let rendered = crate::runtime_display::unprocessed_compiled_lines(&covert_technician)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains(
            "you may put an artifact card with mana value less than or equal to that damage from your hand onto the battlefield"
        ),
        "compiled text should keep the dynamic damage mana-value gate, got {rendered}"
    );

    let technician_id =
        game.create_object_from_definition(&covert_technician, alice, Zone::Battlefield);

    let legal_artifact = CardBuilder::new(CardId::new(), "Legal Bauble")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]]))
        .card_types(vec![CardType::Artifact])
        .build();
    let legal_artifact_id = game.create_object_from_card(&legal_artifact, alice, Zone::Hand);

    let expensive_artifact = CardBuilder::new(CardId::new(), "Expensive Golem")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(4)]]))
        .card_types(vec![CardType::Artifact])
        .build();
    let expensive_artifact_id =
        game.create_object_from_card(&expensive_artifact, alice, Zone::Hand);

    let triggered = covert_technician
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Covert Technician should have a combat-damage trigger");

    let mut yes_dm = ChooseArtifactDecisionMaker {
        accept_may: true,
        chosen: None,
    };
    let mut yes_ctx = ExecutionContext::new_default(technician_id, alice)
        .with_decision_maker(&mut yes_dm)
        .with_event_value_amount(2);
    for effect in &triggered.effects {
        execute_effect(&mut game, effect, &mut yes_ctx)
            .expect("Covert Technician trigger should resolve at damage=2");
    }

    assert!(
        game.player(alice)
            .expect("alice exists")
            .hand
            .contains(&expensive_artifact_id),
        "artifact above damage-based mana value cap must stay in hand"
    );
    assert!(
        game.player(alice)
            .expect("alice exists")
            .hand
            .contains(&legal_artifact_id)
            || game.battlefield.contains(&legal_artifact_id),
        "resolving the trigger must keep the legal artifact in a valid zone"
    );

    let mut game = setup_game();
    let technician_id =
        game.create_object_from_definition(&covert_technician, alice, Zone::Battlefield);
    let legal_artifact_id = game.create_object_from_card(&legal_artifact, alice, Zone::Hand);
    let mut no_dm = ChooseArtifactDecisionMaker {
        accept_may: false,
        chosen: None,
    };
    let mut no_ctx = ExecutionContext::new_default(technician_id, alice)
        .with_decision_maker(&mut no_dm)
        .with_event_value_amount(3);
    for effect in &triggered.effects {
        execute_effect(&mut game, effect, &mut no_ctx)
            .expect("declined Covert Technician trigger should still resolve cleanly");
    }
    assert!(
        game.player(alice)
            .expect("alice exists")
            .hand
            .contains(&legal_artifact_id),
        "declining the optional trigger should keep the artifact in hand"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn splinters_technique_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(80_488), "Splinter's Technique")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::Black],
        ]))
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Sneak {1}{B} (You may cast this spell for {1}{B} if you also return an unblocked attacker you control to hand during the declare blockers step.)\n\
             Search your library for a card, put that card into your hand, then shuffle.",
        )
        .expect("Splinter's Technique should parse for runtime tests")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn set_up_splinters_technique_sneak_game(
    step: Step,
    include_unblocked_attacker: bool,
) -> (GameState, PlayerId, ObjectId, Option<ObjectId>) {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(step);
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.player_mut(alice)
        .expect("alice exists")
        .mana_pool
        .add(ManaSymbol::Black, 2);

    let spell_id =
        game.create_object_from_definition(&splinters_technique_definition(), alice, Zone::Hand);
    let attacker_id = if include_unblocked_attacker {
        let attacker = CardBuilder::new(CardId::from_raw(80_489), "Sneak Attacker")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let attacker_id = game.create_object_from_card(&attacker, alice, Zone::Battlefield);
        game.combat = Some(crate::combat_state::CombatState {
            attackers: vec![crate::combat_state::AttackerInfo {
                creature: attacker_id,
                target: AttackTarget::Player(bob),
            }],
            ..Default::default()
        });
        Some(attacker_id)
    } else {
        game.combat = Some(Default::default());
        None
    };

    (game, alice, spell_id, attacker_id)
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn splinters_technique_sneak_cast_is_legal_only_with_unblocked_attacker_during_declare_blockers()
 {
    use crate::alternative_cast::CastingMethod;
    use crate::decision::{LegalAction, compute_legal_actions};

    let (game, alice, spell_id, _) =
        set_up_splinters_technique_sneak_game(Step::DeclareBlockers, true);
    let actions = compute_legal_actions(&game, alice);
    assert!(
        actions.iter().any(|action| matches!(
            action,
            LegalAction::CastSpell {
                spell_id: candidate,
                from_zone: Zone::Hand,
                casting_method: CastingMethod::Alternative(0),
            } if *candidate == spell_id
        )),
        "Splinter's Technique should be sneak-castable during declare blockers with an unblocked attacker"
    );

    let (game, alice, spell_id, _) =
        set_up_splinters_technique_sneak_game(Step::DeclareBlockers, false);
    let actions = compute_legal_actions(&game, alice);
    assert!(
        !actions.iter().any(|action| matches!(
            action,
            LegalAction::CastSpell {
                spell_id: candidate,
                casting_method: CastingMethod::Alternative(0),
                ..
            } if *candidate == spell_id
        )),
        "Splinter's Technique should not be sneak-castable without an unblocked attacker"
    );

    let (game, alice, spell_id, _) =
        set_up_splinters_technique_sneak_game(Step::CombatDamage, true);
    let actions = compute_legal_actions(&game, alice);
    assert!(
        !actions.iter().any(|action| matches!(
            action,
            LegalAction::CastSpell {
                spell_id: candidate,
                casting_method: CastingMethod::Alternative(0),
                ..
            } if *candidate == spell_id
        )),
        "Splinter's Technique sneak timing should end after the declare blockers step"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn splinters_technique_sneak_cast_returns_attacker_and_searches_library() {
    let (mut game, alice, spell_id, attacker_id) =
        set_up_splinters_technique_sneak_game(Step::DeclareBlockers, true);
    let attacker_id = attacker_id.expect("attacker should exist");
    let library_card = CardBuilder::new(CardId::from_raw(80_490), "Library Prize")
        .card_types(vec![CardType::Artifact])
        .build();
    game.create_object_from_card(&library_card, alice, Zone::Library);

    let total_cost = game
        .object(spell_id)
        .and_then(|object| object.alternative_casts[0].total_cost().cloned())
        .expect("Splinter's Technique should have a sneak total cost");
    let spell_effect = game
        .object(spell_id)
        .and_then(|object| object.spell_effect_owned())
        .expect("Splinter's Technique should have a spell effect");
    let mut decision_maker = SelectFirstDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new(spell_id, alice, &mut decision_maker);
    crate::special_actions::pay_total_cost_with_choice_in_context(
        &mut game,
        alice,
        spell_id,
        &total_cost,
        crate::costs::PaymentReason::CastSpell,
        &mut ctx,
    )
    .expect("paying Splinter's Technique sneak cost should succeed");

    assert!(
        game.player(alice)
            .expect("alice exists")
            .hand
            .iter()
            .any(|id| game
                .object(*id)
                .is_some_and(|object| object.name == "Sneak Attacker")),
        "paying sneak should return the unblocked attacker to hand"
    );
    assert!(
        !game.battlefield.contains(&attacker_id),
        "paying sneak should remove the attacker from the battlefield"
    );

    execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        spell_id,
        &spell_effect,
        None,
        &[],
    )
    .expect("Splinter's Technique tutor effect should resolve");
    assert!(
        game.player(alice)
            .expect("alice exists")
            .hand
            .iter()
            .any(|id| game
                .object(*id)
                .is_some_and(|object| object.name == "Library Prize")),
        "Splinter's Technique should put the searched library card into hand"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn kitsunes_technique_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(80_492), "Kitsune's Technique")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(4)],
            vec![ManaSymbol::Blue],
            vec![ManaSymbol::Blue],
        ]))
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Sneak {1}{U} (You may cast this spell for {1}{U} if you also return an unblocked attacker you control to hand during the declare blockers step.)\n\
             Target opponent mills half their library, rounded up.",
        )
        .expect("Kitsune's Technique should parse for runtime tests")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn set_up_kitsunes_technique_sneak_game(
    step: Step,
    include_unblocked_attacker: bool,
) -> (GameState, PlayerId, PlayerId, ObjectId, Option<ObjectId>) {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(step);
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.player_mut(alice)
        .expect("alice exists")
        .mana_pool
        .add(ManaSymbol::Blue, 2);

    let spell_id =
        game.create_object_from_definition(&kitsunes_technique_definition(), alice, Zone::Hand);
    let attacker_id = if include_unblocked_attacker {
        let attacker = CardBuilder::new(CardId::from_raw(80_493), "Kitsune Sneak Attacker")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let attacker_id = game.create_object_from_card(&attacker, alice, Zone::Battlefield);
        game.combat = Some(crate::combat_state::CombatState {
            attackers: vec![crate::combat_state::AttackerInfo {
                creature: attacker_id,
                target: AttackTarget::Player(bob),
            }],
            ..Default::default()
        });
        Some(attacker_id)
    } else {
        game.combat = Some(Default::default());
        None
    };

    (game, alice, bob, spell_id, attacker_id)
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn add_kitsune_library_cards(game: &mut GameState, owner: PlayerId, count: u32) {
    for index in 0..count {
        let card = CardBuilder::new(
            CardId::from_raw(80_500 + index),
            format!("Kitsune Library Card {index}"),
        )
        .card_types(vec![CardType::Sorcery])
        .build();
        game.create_object_from_card(&card, owner, Zone::Library);
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn resolve_kitsunes_technique_targeting_bob(
    game: &mut GameState,
    alice: PlayerId,
    bob: PlayerId,
    spell_id: ObjectId,
    spell_effect: &crate::resolution::ResolutionProgram,
) {
    let mut ctx = crate::effects::ExecutionContext::new_default(spell_id, alice)
        .with_targets(vec![crate::effects::ResolvedTarget::Player(bob)])
        .with_target_assignments(vec![crate::game_state::TargetAssignment {
            spec: ChooseSpec::target_opponent(),
            range: 0..1,
        }]);
    for effect in spell_effect.flattened_default_effects() {
        crate::effects::execute_effect(game, effect, &mut ctx)
            .expect("Kitsune's Technique effect should resolve");
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn kitsunes_technique_sneak_cast_is_legal_only_with_unblocked_attacker_during_declare_blockers()
 {
    use crate::alternative_cast::CastingMethod;
    use crate::decision::{LegalAction, compute_legal_actions};

    let (game, alice, _, spell_id, _) =
        set_up_kitsunes_technique_sneak_game(Step::DeclareBlockers, true);
    let actions = compute_legal_actions(&game, alice);
    assert!(
        actions.iter().any(|action| matches!(
            action,
            LegalAction::CastSpell {
                spell_id: candidate,
                from_zone: Zone::Hand,
                casting_method: CastingMethod::Alternative(0),
            } if *candidate == spell_id
        )),
        "Kitsune's Technique should be sneak-castable during declare blockers with an unblocked attacker"
    );

    let (game, alice, _, spell_id, _) =
        set_up_kitsunes_technique_sneak_game(Step::DeclareBlockers, false);
    let actions = compute_legal_actions(&game, alice);
    assert!(
        !actions.iter().any(|action| matches!(
            action,
            LegalAction::CastSpell {
                spell_id: candidate,
                casting_method: CastingMethod::Alternative(0),
                ..
            } if *candidate == spell_id
        )),
        "Kitsune's Technique should not be sneak-castable without an unblocked attacker"
    );

    let (game, alice, _, spell_id, _) =
        set_up_kitsunes_technique_sneak_game(Step::CombatDamage, true);
    let actions = compute_legal_actions(&game, alice);
    assert!(
        !actions.iter().any(|action| matches!(
            action,
            LegalAction::CastSpell {
                spell_id: candidate,
                casting_method: CastingMethod::Alternative(0),
                ..
            } if *candidate == spell_id
        )),
        "Kitsune's Technique sneak timing should end after the declare blockers step"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn kitsunes_technique_sneak_cost_returns_attacker_and_mills_odd_library_rounded_up() {
    let (mut game, alice, bob, spell_id, attacker_id) =
        set_up_kitsunes_technique_sneak_game(Step::DeclareBlockers, true);
    let attacker_id = attacker_id.expect("attacker should exist");
    add_kitsune_library_cards(&mut game, bob, 5);

    let total_cost = game
        .object(spell_id)
        .and_then(|object| object.alternative_casts[0].total_cost().cloned())
        .expect("Kitsune's Technique should have a sneak total cost");
    let spell_effect = game
        .object(spell_id)
        .and_then(|object| object.spell_effect_owned())
        .expect("Kitsune's Technique should have a spell effect");
    let mut decision_maker = SelectFirstDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new(spell_id, alice, &mut decision_maker);
    crate::special_actions::pay_total_cost_with_choice_in_context(
        &mut game,
        alice,
        spell_id,
        &total_cost,
        crate::costs::PaymentReason::CastSpell,
        &mut ctx,
    )
    .expect("paying Kitsune's Technique sneak cost should succeed");

    assert!(
        game.player(alice)
            .expect("alice exists")
            .hand
            .iter()
            .any(|id| game
                .object(*id)
                .is_some_and(|object| object.name == "Kitsune Sneak Attacker")),
        "paying sneak should return the unblocked attacker to hand"
    );
    assert!(
        !game.battlefield.contains(&attacker_id),
        "paying sneak should remove the attacker from the battlefield"
    );

    resolve_kitsunes_technique_targeting_bob(&mut game, alice, bob, spell_id, &spell_effect);
    assert_eq!(
        game.player(bob).expect("bob exists").graveyard.len(),
        3,
        "five-card library should mill three cards when rounded up"
    );
    assert_eq!(
        game.player(bob).expect("bob exists").library.len(),
        2,
        "Kitsune's Technique should leave the un-milled half in Bob's library"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn kitsunes_technique_even_library_does_not_round_past_half() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let def = kitsunes_technique_definition();
    let spell_id = game.create_object_from_definition(&def, alice, Zone::Stack);
    add_kitsune_library_cards(&mut game, bob, 4);
    let spell_effect = def
        .spell_effect
        .as_ref()
        .expect("Kitsune's Technique should have a spell effect");

    resolve_kitsunes_technique_targeting_bob(&mut game, alice, bob, spell_id, spell_effect);
    assert_eq!(
        game.player(bob).expect("bob exists").graveyard.len(),
        2,
        "four-card library should mill exactly two cards"
    );
    assert_eq!(
        game.player(bob).expect("bob exists").library.len(),
        2,
        "rounded-up half should not mill an extra card for an even library size"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn elektra_sneak_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(80_491), "Elektra, Daughter of the Hand")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Black],
            vec![ManaSymbol::Black],
        ]))
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 3))
        .parse_text(
            "Sneak {1}{B}{B} (You may cast this spell for {1}{B}{B} if you also return an \
             unblocked attacker you control to hand during the declare blockers step. She enters \
             tapped and attacking.)\n\
             When Elektra enters, destroy target creature an opponent controls with power 3 or less.",
        )
        .expect("Elektra should parse for permanent Sneak runtime tests")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn sneak_permanent_probe_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(80_494), "Sneak Permanent Probe")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 3))
        .parse_text(
            "Sneak {0} (You may cast this spell for {0} if you also return an unblocked \
             attacker you control to hand during the declare blockers step. It enters tapped \
             and attacking.)",
        )
        .expect("zero-cost permanent Sneak probe should parse")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn set_up_permanent_sneak_game(
    definition: &crate::cards::CardDefinition,
) -> (GameState, PlayerId, PlayerId, ObjectId, ObjectId) {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(Step::DeclareBlockers);
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let spell_id = game.create_object_from_definition(definition, alice, Zone::Hand);
    let attacker = CardBuilder::new(CardId::from_raw(80_495), "Permanent Sneak Attacker")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let attacker_id = game.create_object_from_card(&attacker, alice, Zone::Battlefield);
    game.combat = Some(crate::combat_state::CombatState {
        attackers: vec![crate::combat_state::AttackerInfo {
            creature: attacker_id,
            target: AttackTarget::Player(bob),
        }],
        ..Default::default()
    });

    (game, alice, bob, spell_id, attacker_id)
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn finish_cast_to_stack(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    mut progress: crate::decision::GameProgress,
    decision_maker: &mut SelectFirstDecisionMaker,
    spell_name: &str,
) {
    for _ in 0..16 {
        if stack_contains_named_object(game, spell_name) {
            return;
        }
        progress = match progress {
            crate::decision::GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::SelectOptions(ctx),
            ) => {
                let choice = ctx
                    .options
                    .iter()
                    .find(|option| option.legal)
                    .expect("cast flow should have a legal option")
                    .index;
                let description = ctx.description.to_ascii_lowercase();
                assert!(description.starts_with("choose the next cost to pay"));
                let response = PriorityResponse::NextCostChoice(choice);
                apply_priority_response_with_dm(
                    game,
                    trigger_queue,
                    state,
                    &response,
                    decision_maker,
                )
                .expect("cast flow option should be accepted")
            }
            crate::decision::GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::ManaPayment(ctx),
            ) => apply_priority_response_with_dm(
                game,
                trigger_queue,
                state,
                &PriorityResponse::ManaPaymentPlan(
                    crate::mana_payment::ManaPaymentResponse::Confirm {
                        plan_id: ctx.plan.id,
                        request_hash: ctx.plan.request_hash,
                    },
                ),
                decision_maker,
            )
            .expect("cast mana plan should be accepted"),
            crate::decision::GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::SelectObjects(ctx),
            ) => {
                let choice = ctx
                    .candidates
                    .iter()
                    .find(|candidate| candidate.legal)
                    .expect("cast flow should have a legal object choice")
                    .id;
                apply_priority_response_with_dm(
                    game,
                    trigger_queue,
                    state,
                    &PriorityResponse::CardCostChoice(choice),
                    decision_maker,
                )
                .expect("cast flow object choice should be accepted")
            }
            crate::decision::GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::Priority(_),
            )
            | crate::decision::GameProgress::Continue => {
                if stack_contains_named_object(game, spell_name) {
                    return;
                }
                panic!("cast flow returned before {spell_name} reached the stack");
            }
            other => panic!("unexpected cast flow state for {spell_name}: {other:?}"),
        };
    }
    panic!("cast flow did not put {spell_name} on the stack");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn elektra_sneak_cast_is_legal_during_declare_blockers_with_unblocked_attacker() {
    use crate::alternative_cast::CastingMethod;
    use crate::decision::{LegalAction, compute_legal_actions};

    let definition = elektra_sneak_definition();
    let (mut game, alice, bob, spell_id, _) = set_up_permanent_sneak_game(&definition);
    game.player_mut(alice)
        .expect("alice exists")
        .mana_pool
        .add(ManaSymbol::Black, 3);
    let target = CardBuilder::new(CardId::from_raw(80_496), "Elektra Target")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 3))
        .build();
    game.create_object_from_card(&target, bob, Zone::Battlefield);

    let actions = compute_legal_actions(&game, alice);
    assert!(
        actions.iter().any(|action| matches!(
            action,
            LegalAction::CastSpell {
                spell_id: candidate,
                from_zone: Zone::Hand,
                casting_method: CastingMethod::Alternative(0),
            } if *candidate == spell_id
        )),
        "Elektra should be offered as a Sneak cast during declare blockers with an unblocked attacker"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn permanent_sneak_cast_returns_attacker_and_enters_tapped_and_attacking_same_player() {
    use crate::alternative_cast::CastingMethod;
    use crate::decision::{LegalAction, compute_legal_actions};

    let definition = sneak_permanent_probe_definition();
    let (mut game, alice, bob, spell_id, attacker_id) = set_up_permanent_sneak_game(&definition);
    let actions = compute_legal_actions(&game, alice);
    let cast_action = actions
        .into_iter()
        .find(|action| {
            matches!(
                action,
                LegalAction::CastSpell {
                    spell_id: candidate,
                    from_zone: Zone::Hand,
                    casting_method: CastingMethod::Alternative(0),
                } if *candidate == spell_id
            )
        })
        .expect("permanent Sneak spell should be castable");

    let mut trigger_queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut decision_maker = SelectFirstDecisionMaker;
    let progress = apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(cast_action),
        &mut decision_maker,
    )
    .expect("permanent Sneak cast should finish cost payment and put the spell on the stack");
    finish_cast_to_stack(
        &mut game,
        &mut trigger_queue,
        &mut state,
        progress,
        &mut decision_maker,
        "Sneak Permanent Probe",
    );

    assert!(
        game.player(alice)
            .expect("alice exists")
            .hand
            .iter()
            .any(|id| game
                .object(*id)
                .is_some_and(|object| object.name == "Permanent Sneak Attacker")),
        "paying Sneak should return the unblocked attacker to hand"
    );
    assert!(
        !game.battlefield.contains(&attacker_id),
        "paying Sneak should remove the original attacker from the battlefield"
    );
    assert!(
        stack_contains_named_object(&game, "Sneak Permanent Probe"),
        "Sneak permanent should be on the stack after casting"
    );

    resolve_stack_entry_with(&mut game, &mut decision_maker)
        .expect("Sneak permanent should resolve");
    let sneaked_id = game
        .battlefield
        .iter()
        .copied()
        .find(|id| {
            game.object(*id)
                .is_some_and(|object| object.name == "Sneak Permanent Probe")
        })
        .expect("Sneak permanent should enter the battlefield");
    assert!(
        game.is_tapped(sneaked_id),
        "Sneak permanent should enter tapped"
    );
    let combat = game.combat.as_ref().expect("combat should remain active");
    let sneaked_attacker = combat
        .attackers
        .iter()
        .find(|attacker| attacker.creature == sneaked_id)
        .expect("Sneak permanent should enter attacking");
    assert_eq!(sneaked_attacker.target, AttackTarget::Player(bob));
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn quandrix_apprentice_magecraft_puts_only_a_looked_land_into_hand() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let def = CardDefinitionBuilder::new(CardId::from_raw(91_100), "Quandrix Apprentice")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Green], vec![ManaSymbol::Blue]]))
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .parse_text(
            "Magecraft — Whenever you cast or copy an instant or sorcery spell, look at the top three cards of your library. You may reveal a land card from among them and put that card into your hand. Put the rest on the bottom of your library in any order.",
        )
        .expect("Quandrix Apprentice should parse");

    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Quandrix Apprentice should have a triggered ability");

    let source_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    game.create_object_from_card(
        &CardBuilder::new(CardId::from_raw(91_101), "Library Bottom Sentinel")
            .card_types(vec![CardType::Artifact])
            .build(),
        alice,
        Zone::Library,
    );
    game.create_object_from_card(
        &CardBuilder::new(CardId::from_raw(91_102), "Looked Instant")
            .card_types(vec![CardType::Instant])
            .build(),
        alice,
        Zone::Library,
    );
    game.create_object_from_card(
        &CardBuilder::new(CardId::from_raw(91_103), "Quandrix Campus")
            .card_types(vec![CardType::Land])
            .build(),
        alice,
        Zone::Library,
    );
    game.create_object_from_card(
        &CardBuilder::new(CardId::from_raw(91_104), "Looked Creature")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(1, 1))
            .build(),
        alice,
        Zone::Library,
    );

    let mut dm = SelectFirstDecisionMaker;
    resolve_triggered_ability_from_spell_cast(&mut game, triggered, source_id, alice, &mut dm);

    let hand_names: Vec<_> = game
        .player(alice)
        .expect("alice exists")
        .hand
        .iter()
        .filter_map(|&id| game.object(id).map(|obj| obj.name.to_string()))
        .collect();
    assert_eq!(
        hand_names,
        vec!["Quandrix Campus".to_string()],
        "Quandrix Apprentice should put only the looked land into hand"
    );

    let library_names: Vec<_> = game
        .player(alice)
        .expect("alice exists")
        .library
        .iter()
        .filter_map(|&id| game.object(id).map(|obj| obj.name.to_string()))
        .collect();
    assert_eq!(
        library_names.len(),
        3,
        "Quandrix Apprentice should leave the unchosen cards in the library"
    );
    assert!(
        library_names.contains(&"Library Bottom Sentinel".to_string())
            && library_names.contains(&"Looked Instant".to_string())
            && library_names.contains(&"Looked Creature".to_string()),
        "Quandrix Apprentice should keep the nonland looked cards plus the unseen card in the library, got {library_names:?}"
    );
    assert!(
        game.player(alice)
            .expect("alice exists")
            .graveyard
            .is_empty(),
        "Quandrix Apprentice should bottom the other looked cards instead of milling them"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn quandrix_apprentice_magecraft_can_decline_the_land_pick() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let def = CardDefinitionBuilder::new(CardId::from_raw(91_110), "Quandrix Apprentice")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Green], vec![ManaSymbol::Blue]]))
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .parse_text(
            "Magecraft — Whenever you cast or copy an instant or sorcery spell, look at the top three cards of your library. You may reveal a land card from among them and put that card into your hand. Put the rest on the bottom of your library in any order.",
        )
        .expect("Quandrix Apprentice should parse");

    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Quandrix Apprentice should have a triggered ability");

    let source_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    game.create_object_from_card(
        &CardBuilder::new(CardId::from_raw(91_111), "Looked Land")
            .card_types(vec![CardType::Land])
            .build(),
        alice,
        Zone::Library,
    );
    game.create_object_from_card(
        &CardBuilder::new(CardId::from_raw(91_112), "Looked Instant")
            .card_types(vec![CardType::Instant])
            .build(),
        alice,
        Zone::Library,
    );
    game.create_object_from_card(
        &CardBuilder::new(CardId::from_raw(91_113), "Looked Artifact")
            .card_types(vec![CardType::Artifact])
            .build(),
        alice,
        Zone::Library,
    );

    let mut dm = AutoPassDecisionMaker;
    resolve_triggered_ability_from_spell_cast(&mut game, triggered, source_id, alice, &mut dm);

    assert!(
        game.player(alice).expect("alice exists").hand.is_empty(),
        "declining Quandrix Apprentice should leave all looked cards out of hand"
    );

    let library_names: Vec<_> = game
        .player(alice)
        .expect("alice exists")
        .library
        .iter()
        .filter_map(|&id| game.object(id).map(|obj| obj.name.to_string()))
        .collect();
    assert_eq!(
        library_names.len(),
        3,
        "declining Quandrix Apprentice should keep all three looked cards in the library"
    );
    assert!(
        library_names.contains(&"Looked Land".to_string())
            && library_names.contains(&"Looked Instant".to_string())
            && library_names.contains(&"Looked Artifact".to_string()),
        "declining Quandrix Apprentice should keep every looked card in the library, got {library_names:?}"
    );
    assert!(
        game.player(alice)
            .expect("alice exists")
            .graveyard
            .is_empty(),
        "declining Quandrix Apprentice should still bottom the looked cards instead of milling them"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn see_the_truth_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(91_120), "See the Truth")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::Blue],
        ]))
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Look at the top three cards of your library. Put one of those cards into your hand and the rest on the bottom of your library in any order. If this spell was cast from anywhere other than your hand, put each of those cards into your hand instead.",
        )
        .expect("See the Truth should parse for runtime tests")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn add_see_the_truth_library_cards(game: &mut GameState, player: PlayerId) {
    for (idx, name) in ["Alpine Grizzly", "Bear Cub", "Centaur Courser"]
        .into_iter()
        .enumerate()
    {
        game.create_object_from_card(
            &CardBuilder::new(CardId::from_raw(91_121 + idx as u32), name)
                .card_types(vec![CardType::Creature])
                .power_toughness(PowerToughness::fixed(2, 2))
                .build(),
            player,
            Zone::Library,
        );
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn resolve_see_the_truth_with_casting_method(
    casting_method: crate::alternative_cast::CastingMethod,
) -> GameState {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let def = see_the_truth_definition();
    add_see_the_truth_library_cards(&mut game, alice);

    let spell_id = game.create_object_from_definition(&def, alice, Zone::Stack);
    let mut entry = crate::game_state::StackEntry::new(spell_id, alice);
    entry.casting_method = casting_method;
    game.push_to_stack(entry);

    let mut dm = SelectFirstDecisionMaker;
    resolve_stack_entry_with(&mut game, &mut dm).expect("See the Truth should resolve");
    game
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn see_the_truth_cast_from_hand_puts_one_looked_card_into_hand() {
    let game =
        resolve_see_the_truth_with_casting_method(crate::alternative_cast::CastingMethod::Normal);
    let alice = PlayerId::from_index(0);
    let player = game.player(alice).expect("alice exists");

    assert_eq!(
        player.hand.len(),
        1,
        "See the Truth cast normally from hand should put exactly one looked card into hand"
    );
    assert_eq!(
        player.library.len(),
        2,
        "See the Truth cast from hand should leave the other two looked cards in the library"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn see_the_truth_cast_from_exile_puts_each_looked_card_into_hand() {
    let game = resolve_see_the_truth_with_casting_method(
        crate::alternative_cast::CastingMethod::PlayFrom {
            source: ObjectId::from_raw(91_130),
            zone: Zone::Exile,
            use_alternative: None,
        },
    );
    let alice = PlayerId::from_index(0);
    let player = game.player(alice).expect("alice exists");
    let mut hand_names: Vec<_> = player
        .hand
        .iter()
        .filter_map(|&id| game.object(id).map(|obj| obj.name.to_string()))
        .collect();
    hand_names.sort();

    assert_eq!(
        hand_names,
        vec![
            "Alpine Grizzly".to_string(),
            "Bear Cub".to_string(),
            "Centaur Courser".to_string(),
        ],
        "See the Truth cast from exile should put each looked card into hand"
    );
    assert!(
        player.library.is_empty(),
        "the non-hand replacement should not leave looked cards on the bottom of the library"
    );
}

// ============================================================================
// Saga Integration Tests
// ============================================================================

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn tom_bombadil_strict_parser_and_compiled_text_regression() {
    let def = tom_bombadil_definition();
    let rendered = crate::runtime_display::canonical_compiled_lines(&def).join("\n");

    assert!(
        rendered.contains(
            "As long as there are four or more lore counters among sagas you control, Tom Bombadil has hexproof and indestructible."
        ),
        "Tom Bombadil should render the lore-counter static ability with the named legendary source, got:\n{rendered}"
    );
    assert!(
        rendered.contains(
            "Whenever the final chapter ability of a Saga you control resolves, reveal cards from the top of your library until you reveal a Saga card. Put that card onto the battlefield and the rest on the bottom of your library in a random order. This ability triggers only once each turn."
        ),
        "Tom Bombadil should render the final-Saga-chapter trigger and once-per-turn cap, got:\n{rendered}"
    );

    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Tom Bombadil should have a final chapter trigger");
    let final_chapter = triggered
        .trigger
        .downcast_ref::<crate::triggers::other::FinalChapterAbilityResolvedTrigger>()
        .expect("Tom Bombadil should use the final chapter ability resolved trigger");
    assert_eq!(final_chapter.filter.subtypes, vec![Subtype::Saga]);
    assert_eq!(final_chapter.filter.controller, Some(PlayerFilter::You));
    assert!(matches!(
        triggered.intervening_if,
        Some(crate::ConditionExpr::MaxTimesEachTurn(1))
    ));
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn battle_for_bretagard_chapters_create_and_copy_distinct_named_tokens() {
    struct SelectAllObjects;

    impl DecisionMaker for SelectAllObjects {
        fn decide_objects(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            ctx.candidates
                .iter()
                .filter(|candidate| candidate.legal)
                .map(|candidate| candidate.id)
                .collect()
        }
    }

    fn count_controlled_named_tokens(game: &GameState, controller: PlayerId, name: &str) -> usize {
        game.battlefield
            .iter()
            .filter(|&&id| {
                game.object(id).is_some_and(|object| {
                    object.name == name
                        && matches!(object.kind, ObjectKind::Token)
                        && game.controller_of(object) == controller
                })
            })
            .count()
    }

    fn resolve_next_chapter(
        game: &mut GameState,
        trigger_queue: &mut TriggerQueue,
        saga_id: ObjectId,
        dm: &mut SelectAllObjects,
    ) {
        add_lore_counter_and_check_chapters(game, saga_id, trigger_queue);
        put_triggers_on_stack_with_dm(game, trigger_queue, dm)
            .expect("Battle for Bretagard chapter trigger should go on the stack");
        resolve_stack_entry_with_dm_and_triggers(game, dm, trigger_queue)
            .expect("Battle for Bretagard chapter trigger should resolve");
    }

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut trigger_queue = TriggerQueue::new();
    let mut dm = SelectAllObjects;

    let battle_id = game.create_object_from_definition(
        &battle_for_bretagard_definition(),
        alice,
        Zone::Battlefield,
    );

    resolve_next_chapter(&mut game, &mut trigger_queue, battle_id, &mut dm);
    assert_eq!(
        count_controlled_named_tokens(&game, alice, "Human Warrior"),
        1
    );

    resolve_next_chapter(&mut game, &mut trigger_queue, battle_id, &mut dm);
    assert_eq!(
        count_controlled_named_tokens(&game, alice, "Elf Warrior"),
        1
    );

    let treasure = CardDefinitionBuilder::new(CardId::from_raw(72_930), "Treasure")
        .token()
        .card_types(vec![CardType::Artifact])
        .build();
    for controller in [alice, alice, bob] {
        let treasure_id =
            game.create_object_from_definition(&treasure, controller, Zone::Battlefield);
        game.object_mut(treasure_id)
            .expect("Treasure token should exist")
            .kind = ObjectKind::Token;
    }
    create_creature(&mut game, "Nontoken Guard", alice, 2, 2);

    resolve_next_chapter(&mut game, &mut trigger_queue, battle_id, &mut dm);

    assert_eq!(
        count_controlled_named_tokens(&game, alice, "Human Warrior"),
        2,
        "chapter III should copy the Human token chosen from chapter I"
    );
    assert_eq!(
        count_controlled_named_tokens(&game, alice, "Elf Warrior"),
        2,
        "chapter III should copy the Elf token chosen from chapter II"
    );
    assert_eq!(
        count_controlled_named_tokens(&game, alice, "Treasure"),
        3,
        "chapter III should copy only one of two same-named Treasure tokens"
    );
    assert_eq!(
        count_controlled_named_tokens(&game, bob, "Treasure"),
        1,
        "chapter III should not choose or copy opponents' tokens"
    );
    assert!(
        game.battlefield.iter().all(|&id| {
            game.object(id).is_none_or(|object| {
                object.name != "Nontoken Guard" || !matches!(object.kind, ObjectKind::Token)
            })
        }),
        "the nontoken creature should not be copied by the token-only choice"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn interface_ace_tap_trigger_untaps_once_and_only_during_your_turn() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let ace_id =
        game.create_object_from_definition(&interface_ace_definition(), alice, Zone::Battlefield);
    game.turn.active_player = alice;

    game.tap(ace_id);
    let first_tap = TriggerEvent::new_with_provenance(
        crate::events::PermanentTappedEvent::new(ace_id),
        crate::provenance::ProvNodeId::default(),
    );
    queue_triggers_from_event(&mut game, &mut trigger_queue, first_tap, false);
    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("Interface Ace should trigger when tapped during its controller's turn");
    assert_eq!(
        game.stack.len(),
        1,
        "first tap during your turn should trigger"
    );
    resolve_stack_entry(&mut game).expect("Interface Ace untap trigger should resolve");
    assert!(
        !game.is_tapped(ace_id),
        "Interface Ace trigger should untap it after the first tap"
    );

    game.tap(ace_id);
    let second_tap = TriggerEvent::new_with_provenance(
        crate::events::PermanentTappedEvent::new(ace_id),
        crate::provenance::ProvNodeId::default(),
    );
    queue_triggers_from_event(&mut game, &mut trigger_queue, second_tap, false);
    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("second same-turn Interface Ace tap should be processed cleanly");
    assert!(
        game.stack.is_empty(),
        "Interface Ace should not trigger a second time in the same turn"
    );
    assert!(
        game.is_tapped(ace_id),
        "second same-turn tap should remain tapped because the once-per-turn trigger is capped"
    );

    let mut opponent_turn_game = setup_game();
    let mut opponent_turn_queue = TriggerQueue::new();
    let opponent_turn_ace = opponent_turn_game.create_object_from_definition(
        &interface_ace_definition(),
        alice,
        Zone::Battlefield,
    );
    opponent_turn_game.turn.active_player = bob;
    opponent_turn_game.tap(opponent_turn_ace);
    let opponent_turn_tap = TriggerEvent::new_with_provenance(
        crate::events::PermanentTappedEvent::new(opponent_turn_ace),
        crate::provenance::ProvNodeId::default(),
    );
    queue_triggers_from_event(
        &mut opponent_turn_game,
        &mut opponent_turn_queue,
        opponent_turn_tap,
        false,
    );
    put_triggers_on_stack(&mut opponent_turn_game, &mut opponent_turn_queue)
        .expect("opponent-turn Interface Ace tap should be processed cleanly");
    assert!(
        opponent_turn_game.stack.is_empty(),
        "Interface Ace should not trigger when tapped during an opponent's turn"
    );
    assert!(
        opponent_turn_game.is_tapped(opponent_turn_ace),
        "opponent-turn tap should stay tapped because the trigger condition is not met"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn king_macar_untap_trigger_creates_gold_only_when_optional_exile_happens() {
    fn king_macar_definition() -> crate::cards::CardDefinition {
        CardDefinitionBuilder::new(CardId::from_raw(72_931), "King Macar, the Gold-Cursed")
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(2)],
                vec![ManaSymbol::Black],
                vec![ManaSymbol::Black],
            ]))
            .supertypes(vec![Supertype::Legendary])
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Human, Subtype::Noble])
            .power_toughness(PowerToughness::fixed(2, 3))
            .parse_text(
                "Whenever this creature becomes untapped, you may exile target creature. If you do, create a Gold token.",
            )
            .expect("King Macar's inspired ability should parse")
    }

    fn gold_count(game: &GameState, controller: PlayerId) -> usize {
        game.battlefield
            .iter()
            .filter_map(|&id| game.object(id))
            .filter(|object| {
                object.name == "Gold"
                    && object.kind == ObjectKind::Token
                    && game.controller_of(object) == controller
            })
            .count()
    }

    fn queue_untap_trigger(
        game: &mut GameState,
        trigger_queue: &mut TriggerQueue,
        source: ObjectId,
    ) {
        game.tap(source);
        game.untap(source);
        let event = TriggerEvent::new_with_provenance(
            crate::events::PermanentUntappedEvent::new(source),
            crate::provenance::ProvNodeId::default(),
        );
        queue_triggers_from_event(game, trigger_queue, event, false);
    }

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let target_card = CardBuilder::new(CardId::from_raw(72_932), "Inspired Exile Target")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();

    let mut accept_game = setup_game();
    let accept_target = accept_game.create_object_from_card(&target_card, bob, Zone::Battlefield);
    let accept_target_stable = accept_game
        .object(accept_target)
        .expect("exile target should exist")
        .stable_id;
    let accept_macar = accept_game.create_object_from_definition(
        &king_macar_definition(),
        alice,
        Zone::Battlefield,
    );
    let mut accept_queue = TriggerQueue::new();
    queue_untap_trigger(&mut accept_game, &mut accept_queue, accept_macar);
    let mut accept_dm = SelectFirstDecisionMaker;
    put_triggers_on_stack_with_dm(&mut accept_game, &mut accept_queue, &mut accept_dm)
        .expect("King Macar's untap trigger should go on the stack");
    assert_eq!(
        accept_game
            .stack
            .last()
            .map(|entry| entry.targets.as_slice()),
        Some(&[Target::Object(accept_target)][..]),
        "the inspired ability should choose its creature target while going on the stack"
    );
    resolve_stack_entry_with(&mut accept_game, &mut accept_dm)
        .expect("accepted King Macar trigger should resolve");
    let exiled_target = accept_game
        .find_object_by_stable_id(accept_target_stable)
        .and_then(|id| accept_game.object(id));
    assert!(
        exiled_target.is_some_and(|object| object.zone == Zone::Exile),
        "accepting the optional effect should exile the target"
    );
    assert_eq!(
        gold_count(&accept_game, alice),
        1,
        "a Gold token should be created after the optional exile happens"
    );

    let mut decline_game = setup_game();
    let decline_target = decline_game.create_object_from_card(&target_card, bob, Zone::Battlefield);
    let decline_macar = decline_game.create_object_from_definition(
        &king_macar_definition(),
        alice,
        Zone::Battlefield,
    );
    let mut decline_queue = TriggerQueue::new();
    queue_untap_trigger(&mut decline_game, &mut decline_queue, decline_macar);
    let mut target_dm = SelectFirstDecisionMaker;
    put_triggers_on_stack_with_dm(&mut decline_game, &mut decline_queue, &mut target_dm)
        .expect("declined King Macar trigger should still choose its target on the stack");
    let mut decline_dm = AutoPassDecisionMaker;
    resolve_stack_entry_with(&mut decline_game, &mut decline_dm)
        .expect("declined King Macar trigger should resolve cleanly");
    assert!(
        decline_game
            .object(decline_target)
            .is_some_and(|object| object.zone == Zone::Battlefield),
        "declining the optional exile should leave the target on the battlefield"
    );
    assert_eq!(
        gold_count(&decline_game, alice),
        0,
        "the conditional Gold effect must not run when the optional exile is declined"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn sothera_links_opponent_exile_and_reanimates_only_if_intervening_if_still_holds() {
    fn sothera_definition() -> crate::cards::CardDefinition {
        CardDefinitionBuilder::new(CardId::from_raw(72_941), "Sothera, the Supervoid")
            .supertypes(vec![Supertype::Legendary])
            .card_types(vec![CardType::Enchantment])
            .parse_text(
                "Whenever a creature you control dies, each opponent chooses a creature they control and exiles it.\n\
                 At the beginning of your end step, if a player controls no creatures, sacrifice Sothera, then put a creature card exiled with it onto the battlefield under your control with two additional +1/+1 counters on it.",
            )
            .expect("Sothera should parse for its gameplay regression")
    }

    fn end_step_event(player: PlayerId) -> TriggerEvent {
        TriggerEvent::new_with_provenance(
            crate::events::phase::BeginningOfEndStepEvent::new(player),
            crate::provenance::ProvNodeId::default(),
        )
    }

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);
    let creature = CardBuilder::new(CardId::from_raw(72_942), "Sothera Exile Target")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();

    let mut game = setup_three_player_game();
    game.turn.active_player = alice;
    let sothera =
        game.create_object_from_definition(&sothera_definition(), alice, Zone::Battlefield);
    let alice_victim = game.create_object_from_card(&creature, alice, Zone::Battlefield);
    game.create_object_from_card(&creature, alice, Zone::Battlefield);
    let bob_target = game.create_object_from_card(&creature, bob, Zone::Battlefield);
    let bob_target_stable = game
        .object(bob_target)
        .expect("Bob's creature should exist")
        .stable_id;
    let charlie_target = game.create_object_from_card(&creature, charlie, Zone::Battlefield);
    let charlie_target_stable = game
        .object(charlie_target)
        .expect("Charlie's creature should exist")
        .stable_id;

    assert!(
        crate::triggers::check_triggers(&game, &end_step_event(alice))
            .into_iter()
            .all(|trigger| trigger.source != sothera),
        "Sothera must not trigger while every player controls a creature"
    );

    game.move_object_by_effect(alice_victim, Zone::Graveyard)
        .expect("Alice's creature should die");
    let mut death_queue = TriggerQueue::new();
    drain_pending_trigger_events(&mut game, &mut death_queue);
    assert_eq!(
        death_queue
            .entries
            .iter()
            .filter(|entry| entry.source == sothera)
            .count(),
        1,
        "Sothera should trigger once when its controller's creature dies"
    );
    let mut dm = SelectFirstDecisionMaker;
    put_triggers_on_stack_with_dm(&mut game, &mut death_queue, &mut dm)
        .expect("Sothera's dies trigger should go on the stack");
    resolve_stack_entry_with(&mut game, &mut dm)
        .expect("Sothera's dies trigger should exile Bob's chosen creature");

    let exiled_target = game
        .find_object_by_stable_id(bob_target_stable)
        .expect("Bob's chosen creature should still exist in exile");
    assert_eq!(
        game.object(exiled_target).map(|object| object.zone),
        Some(Zone::Exile),
        "the creature chosen by Bob should be exiled"
    );
    let charlie_exiled_target = game
        .find_object_by_stable_id(charlie_target_stable)
        .expect("Charlie's chosen creature should still exist in exile");
    assert_eq!(
        game.object(charlie_exiled_target).map(|object| object.zone),
        Some(Zone::Exile),
        "the creature chosen by Charlie should be exiled"
    );
    let linked = game.get_exiled_with_source_links(sothera);
    assert!(
        linked.len() == 2
            && linked.contains(&exiled_target)
            && linked.contains(&charlie_exiled_target),
        "the first trigger must exile and link one chosen creature from each opponent: {linked:?}"
    );

    let mut recheck_game = game.clone();
    let mut recheck_queue = TriggerQueue::new();
    queue_triggers_from_event(
        &mut recheck_game,
        &mut recheck_queue,
        end_step_event(alice),
        false,
    );
    put_triggers_on_stack_with_dm(&mut recheck_game, &mut recheck_queue, &mut dm)
        .expect("Sothera's conditionally triggered ability should go on the stack");
    recheck_game.create_object_from_card(&creature, bob, Zone::Battlefield);
    recheck_game.create_object_from_card(&creature, charlie, Zone::Battlefield);
    let recheck_entry = recheck_game
        .stack
        .last()
        .expect("Sothera's conditional trigger should be on the stack");
    let recheck_condition = recheck_entry
        .intervening_if
        .as_ref()
        .expect("Sothera's stack entry must carry its intervening-if condition");
    assert!(
        !crate::triggers::verify_intervening_if(
            &recheck_game,
            recheck_condition,
            recheck_entry.controller,
            recheck_entry
                .triggering_event
                .as_ref()
                .expect("Sothera's stack entry must retain the end-step event"),
            recheck_entry.object_id,
            None,
            Some(&recheck_entry.optional_costs_paid),
        ),
        "Sothera's intervening-if condition should become false once every player controls a creature: {recheck_condition:#?}"
    );
    resolve_stack_entry_with(&mut recheck_game, &mut dm)
        .expect("a failed intervening-if recheck should resolve as a no-op");
    assert!(
        recheck_game
            .object(sothera)
            .is_some_and(|object| object.zone == Zone::Battlefield),
        "Sothera must not be sacrificed when the condition stops being true before resolution"
    );
    let recheck_exiled = recheck_game
        .find_object_by_stable_id(bob_target_stable)
        .expect("the linked card should remain present after the failed recheck");
    assert_eq!(
        recheck_game
            .object(recheck_exiled)
            .map(|object| object.zone),
        Some(Zone::Exile),
        "the linked card must stay exiled after the intervening-if fails"
    );

    let mut end_step_queue = TriggerQueue::new();
    queue_triggers_from_event(&mut game, &mut end_step_queue, end_step_event(alice), false);
    assert_eq!(
        end_step_queue
            .entries
            .iter()
            .filter(|entry| entry.source == sothera)
            .count(),
        1,
        "Sothera should trigger when Bob controls no creatures"
    );
    put_triggers_on_stack_with_dm(&mut game, &mut end_step_queue, &mut dm)
        .expect("Sothera's end-step trigger should go on the stack");
    resolve_stack_entry_with(&mut game, &mut dm)
        .expect("Sothera's end-step trigger should resolve");

    assert!(
        game.objects_in_zone(Zone::Graveyard)
            .iter()
            .filter_map(|id| game.object(*id))
            .any(|object| object.name == "Sothera, the Supervoid"),
        "Sothera should sacrifice itself"
    );
    let bob_linked = game
        .find_object_by_stable_id(bob_target_stable)
        .expect("Bob's linked creature should still exist");
    let charlie_linked = game
        .find_object_by_stable_id(charlie_target_stable)
        .expect("Charlie's linked creature should still exist");
    let returned_candidates = [bob_linked, charlie_linked]
        .into_iter()
        .filter(|id| {
            game.object(*id)
                .is_some_and(|object| object.zone == Zone::Battlefield)
        })
        .collect::<Vec<_>>();
    assert_eq!(
        returned_candidates.len(),
        1,
        "exactly one of Sothera's linked creature cards should return"
    );
    let returned = returned_candidates[0];
    let returned_object = game
        .object(returned)
        .expect("returned creature should exist");
    assert_eq!(
        game.controller_of(returned_object),
        alice,
        "the linked creature should enter under Sothera's controller's control"
    );
    assert_eq!(
        game.counter_count(returned, crate::object::CounterType::PlusOnePlusOne),
        2,
        "the returned creature should enter with exactly two additional +1/+1 counters"
    );
    let still_exiled = if returned == bob_linked {
        game.object(charlie_linked)
    } else {
        game.object(bob_linked)
    };
    assert!(
        still_exiled.is_some_and(|object| object.zone == Zone::Exile),
        "the linked creature card not chosen to return should remain in exile"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn ripples_of_potential_phases_only_controlled_permanents_proliferated_this_way() {
    struct RipplesDecisionMaker {
        proliferate_permanents: Vec<ObjectId>,
        phase_candidates: Vec<ObjectId>,
    }

    impl DecisionMaker for RipplesDecisionMaker {
        fn decide_proliferate(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::ProliferateContext,
        ) -> crate::decisions::specs::ProliferateResponse {
            crate::decisions::specs::ProliferateResponse {
                permanents: self.proliferate_permanents.clone(),
                players: Vec::new(),
            }
        }

        fn decide_objects(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            self.phase_candidates = ctx
                .candidates
                .iter()
                .filter(|candidate| candidate.legal)
                .map(|candidate| candidate.id)
                .collect();
            self.phase_candidates.clone()
        }
    }

    fn counter_count(game: &GameState, object: ObjectId) -> u32 {
        game.object(object)
            .and_then(|object| {
                object
                    .counters
                    .get(&crate::object::CounterType::PlusOnePlusOne)
                    .copied()
            })
            .unwrap_or(0)
    }

    let definition = CardDefinitionBuilder::new(CardId::from_raw(72_951), "Ripples of Potential")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Proliferate, then choose any number of permanents you control that had a counter put on them this way. Those permanents phase out.",
        )
        .expect("Ripples of Potential should parse for its gameplay regression");
    let spell_effect = definition
        .spell_effect
        .clone()
        .expect("Ripples of Potential should have a spell effect");
    let creature = CardBuilder::new(CardId::from_raw(72_952), "Countered Permanent")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let selected = game.create_object_from_card(&creature, alice, Zone::Battlefield);
    let skipped = game.create_object_from_card(&creature, alice, Zone::Battlefield);
    let opponents = game.create_object_from_card(&creature, bob, Zone::Battlefield);
    for object in [selected, skipped, opponents] {
        game.add_counters(object, crate::object::CounterType::PlusOnePlusOne, 1)
            .expect("the test permanent should receive its initial counter");
    }
    let spell = game.create_object_from_definition(&definition, alice, Zone::Stack);
    let mut dm = RipplesDecisionMaker {
        proliferate_permanents: vec![selected, opponents],
        phase_candidates: Vec::new(),
    };
    let mut ctx = crate::effects::ExecutionContext::new(spell, alice, &mut dm);
    execute_resolution_program(&mut game, &mut ctx, alice, spell, &spell_effect, None, &[])
        .expect("Ripples of Potential should resolve");

    assert_eq!(
        dm.phase_candidates,
        vec![selected],
        "the phase-out choice must contain only controlled permanents that actually received a counter from this proliferate action"
    );
    assert_eq!(counter_count(&game, selected), 2);
    assert!(game.is_phased_out(selected));
    assert_eq!(
        counter_count(&game, skipped),
        1,
        "a countered permanent omitted from proliferate must not become eligible to phase out"
    );
    assert!(!game.is_phased_out(skipped));
    assert_eq!(counter_count(&game, opponents), 2);
    assert!(
        !game.is_phased_out(opponents),
        "a proliferated permanent an opponent controls is outside Ripples' later choice"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn lightning_reflexes_schedules_sacrifice_only_for_non_sorcery_timing_cast() {
    use crate::cost::OptionalCostsPaid;

    fn timing_probe_definition() -> crate::cards::CardDefinition {
        CardDefinitionBuilder::new(CardId::from_raw(72_961), "Lightning Reflexes Timing Probe")
            .card_types(vec![CardType::Enchantment])
            .parse_text(
                "You may cast this spell as though it had flash. If you cast it any time a sorcery couldn't have been cast, the controller of the permanent it becomes sacrifices it at the beginning of the next cleanup step.",
            )
            .expect("Lightning Reflexes timing family should parse")
    }

    fn execute_spell_program(
        game: &mut GameState,
        definition: &crate::cards::CardDefinition,
        controller: PlayerId,
        paid: OptionalCostsPaid,
        was_cast: bool,
    ) -> ObjectId {
        let spell = game.create_object_from_definition(definition, controller, Zone::Stack);
        game.object_mut(spell)
            .expect("timing probe spell should exist")
            .optional_costs_paid = paid.clone();
        if was_cast {
            game.record_turn_history_event(&TriggerEvent::new_with_provenance(
                crate::events::spells::SpellCastEvent::new(spell, controller, Zone::Hand),
                crate::provenance::ProvNodeId::default(),
            ));
        }
        let program = definition
            .spell_effect
            .as_ref()
            .expect("timing probe should have a conditional spell effect");
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = crate::effects::ExecutionContext::new(spell, controller, &mut dm)
            .with_optional_costs_paid(paid);
        execute_resolution_program(game, &mut ctx, controller, spell, program, None, &[])
            .expect("timing probe spell program should execute");
        game.move_object_by_effect(spell, Zone::Battlefield)
            .expect("timing probe should become a permanent")
    }

    let definition = timing_probe_definition();
    assert!(
        definition.abilities.iter().any(|ability| {
            matches!(&ability.kind, AbilityKind::Static(static_ability)
                if static_ability.id() == crate::static_abilities::StaticAbilityId::Flash)
        }),
        "the compound line must still grant flash"
    );

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut on_time_game = setup_game();
    let mut on_time_paid = OptionalCostsPaid::default();
    on_time_paid.mark_cast_at_sorcery_timing();
    let on_time_permanent =
        execute_spell_program(&mut on_time_game, &definition, alice, on_time_paid, true);
    assert!(
        on_time_game.effect_store.delayed_triggers.is_empty(),
        "a cast made when a sorcery could have been cast must not schedule a sacrifice"
    );
    assert_eq!(
        on_time_game
            .object(on_time_permanent)
            .map(|object| object.zone),
        Some(Zone::Battlefield)
    );

    let mut off_time_game = setup_game();
    let off_time_permanent = execute_spell_program(
        &mut off_time_game,
        &definition,
        alice,
        OptionalCostsPaid::default(),
        true,
    );
    assert_eq!(
        off_time_game.effect_store.delayed_triggers.len(),
        1,
        "a cast made outside sorcery timing should arm exactly one cleanup sacrifice"
    );
    let off_time_stable_id = off_time_game
        .object(off_time_permanent)
        .expect("off-time permanent should exist")
        .stable_id;

    let end_step = TriggerEvent::new_with_provenance(
        crate::events::phase::BeginningOfEndStepEvent::new(alice),
        crate::provenance::ProvNodeId::default(),
    );
    assert!(
        crate::triggers::check_delayed_triggers(&mut off_time_game, &end_step).is_empty(),
        "the delayed sacrifice must wait past the end step"
    );

    // The Oracle instruction follows the resulting permanent and uses its
    // current controller. A control change must not detach the delayed action.
    off_time_game.set_current_controller(off_time_permanent, bob);
    let cleanup = TriggerEvent::new_with_provenance(
        crate::events::phase::BeginningOfCleanupStepEvent::new(alice),
        crate::provenance::ProvNodeId::default(),
    );
    let mut cleanup_queue = TriggerQueue::new();
    for trigger in crate::triggers::check_delayed_triggers(&mut off_time_game, &cleanup) {
        cleanup_queue.add(trigger);
    }
    assert_eq!(cleanup_queue.entries.len(), 1);
    put_triggers_on_stack(&mut off_time_game, &mut cleanup_queue)
        .expect("cleanup sacrifice should go on the stack");
    resolve_stack_entry(&mut off_time_game).expect("cleanup sacrifice should resolve");

    let current = off_time_game
        .find_object_by_stable_id(off_time_stable_id)
        .unwrap_or(off_time_permanent);
    assert_eq!(
        off_time_game.object(current).map(|object| object.zone),
        Some(Zone::Graveyard),
        "the resulting permanent should be sacrificed at the next cleanup step even after changing control"
    );

    let mut copy_game = setup_game();
    let copied_permanent = execute_spell_program(
        &mut copy_game,
        &definition,
        alice,
        OptionalCostsPaid::default(),
        false,
    );
    assert!(copy_game.effect_store.delayed_triggers.is_empty());
    assert_eq!(
        copy_game.object(copied_permanent).map(|object| object.zone),
        Some(Zone::Battlefield),
        "a spell copy that was never cast must not satisfy Lightning Reflexes' cast-time condition"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn tom_bombadil_counts_lore_counters_among_sagas_you_control_for_keywords() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let tom_id =
        game.create_object_from_definition(&tom_bombadil_definition(), alice, Zone::Battlefield);
    let saga_def = CardDefinitionBuilder::new(CardId::from_raw(72_921), "Lore Counter Probe")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Saga])
        .build();
    let alice_sagas = (0..4)
        .map(|_| game.create_object_from_definition(&saga_def, alice, Zone::Battlefield))
        .collect::<Vec<_>>();
    let bob_saga = game.create_object_from_definition(&saga_def, bob, Zone::Battlefield);
    game.object_mut(bob_saga)
        .expect("opponent Saga exists")
        .add_counters(CounterType::Lore, 4);

    assert!(
        !game.object_has_static_ability_id(
            tom_id,
            crate::static_abilities::StaticAbilityId::Hexproof
        ) && !game.object_has_static_ability_id(
            tom_id,
            crate::static_abilities::StaticAbilityId::Indestructible
        ),
        "Tom Bombadil should not count zero-counter Sagas or an opponent's lore counters"
    );

    game.object_mut(alice_sagas[0])
        .expect("first Saga exists")
        .add_counters(CounterType::Lore, 3);
    assert!(
        !game.object_has_static_ability_id(
            tom_id,
            crate::static_abilities::StaticAbilityId::Hexproof
        ) && !game.object_has_static_ability_id(
            tom_id,
            crate::static_abilities::StaticAbilityId::Indestructible
        ),
        "Tom Bombadil should require at least four controlled lore counters among Sagas"
    );

    game.object_mut(alice_sagas[1])
        .expect("second Saga exists")
        .add_counters(CounterType::Lore, 1);
    assert!(
        game.object_has_static_ability_id(
            tom_id,
            crate::static_abilities::StaticAbilityId::Hexproof
        ) && game.object_has_static_ability_id(
            tom_id,
            crate::static_abilities::StaticAbilityId::Indestructible
        ),
        "Tom Bombadil should gain both keywords from four lore counters distributed among controlled Sagas"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn tom_bombadil_final_chapter_trigger_matches_once_each_turn() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let tom_id =
        game.create_object_from_definition(&tom_bombadil_definition(), alice, Zone::Battlefield);
    let saga_def = CardDefinitionBuilder::new(CardId::from_raw(72_922), "Final Chapter Probe")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Saga])
        .build();
    let alice_saga = game.create_object_from_definition(&saga_def, alice, Zone::Battlefield);
    let bob_saga = game.create_object_from_definition(&saga_def, bob, Zone::Battlefield);

    let non_final_event = TriggerEvent::new_with_provenance(
        crate::events::other::ChapterAbilityResolvedEvent::new(alice_saga, alice, false),
        crate::provenance::ProvNodeId::default(),
    );
    assert!(
        check_triggers(&game, &non_final_event).is_empty(),
        "Tom Bombadil should not trigger from a non-final Saga chapter ability"
    );

    let opponent_event = TriggerEvent::new_with_provenance(
        crate::events::other::ChapterAbilityResolvedEvent::new(bob_saga, bob, true),
        crate::provenance::ProvNodeId::default(),
    );
    assert!(
        check_triggers(&game, &opponent_event).is_empty(),
        "Tom Bombadil should not trigger from an opponent's Saga"
    );

    let final_event = TriggerEvent::new_with_provenance(
        crate::events::other::ChapterAbilityResolvedEvent::new(alice_saga, alice, true),
        crate::provenance::ProvNodeId::default(),
    );
    let mut trigger_queue = TriggerQueue::new();
    for trigger in check_triggers(&game, &final_event) {
        trigger_queue.add(trigger);
    }
    assert_eq!(trigger_queue.entries.len(), 1);
    assert_eq!(trigger_queue.entries[0].source, tom_id);

    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("Tom Bombadil trigger should stack");
    assert_eq!(game.stack.len(), 1);
    assert!(
        check_triggers(&game, &final_event).is_empty(),
        "Tom Bombadil's final-chapter ability should trigger only once each turn"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn tom_bombadil_final_chapter_trigger_uses_source_lki_after_saga_moves() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let tom_id =
        game.create_object_from_definition(&tom_bombadil_definition(), alice, Zone::Battlefield);
    let saga_def = CardDefinitionBuilder::new(CardId::from_raw(72_926), "Departing Saga")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Saga])
        .build();
    let saga_id = game.create_object_from_definition(&saga_def, alice, Zone::Battlefield);
    let snapshot = crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
        game.object(saga_id).expect("Saga should exist"),
        &game,
    );

    game.move_object_by_effect(saga_id, Zone::Graveyard)
        .expect("Saga should move to graveyard");

    let final_event = TriggerEvent::new_with_provenance(
        crate::events::other::ChapterAbilityResolvedEvent::new(saga_id, alice, true),
        crate::provenance::ProvNodeId::default(),
    )
    .with_source_snapshot(snapshot);
    let triggers = check_triggers(&game, &final_event);

    assert_eq!(triggers.len(), 1);
    assert_eq!(triggers[0].source, tom_id);
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn tom_bombadil_final_chapter_resolution_reveals_until_saga() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let tom_def = tom_bombadil_definition();
    let tom_id = game.create_object_from_definition(&tom_def, alice, Zone::Battlefield);
    let triggered = tom_def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Tom Bombadil should have a final chapter trigger");

    let bottom_def = CardDefinitionBuilder::new(CardId::from_raw(72_923), "Unrevealed Bottom")
        .card_types(vec![CardType::Artifact])
        .build();
    let top_non_saga_def = CardDefinitionBuilder::new(CardId::from_raw(72_924), "Top Non-Saga")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let saga_def = CardDefinitionBuilder::new(CardId::from_raw(72_925), "Revealed Saga")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Saga])
        .build();
    let bottom_id = game.create_object_from_definition(&bottom_def, alice, Zone::Library);
    let saga_id = game.create_object_from_definition(&saga_def, alice, Zone::Library);
    let top_non_saga_id =
        game.create_object_from_definition(&top_non_saga_def, alice, Zone::Library);
    let saga_stable_id = game.object(saga_id).expect("Saga exists").stable_id;
    assert!(game.set_player_library_order_with_audit(
        alice,
        vec![bottom_id, saga_id, top_non_saga_id],
        "Tom Bombadil test library order",
    ));

    let mut ctx = crate::effects::ExecutionContext::new_default(tom_id, alice);
    execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        tom_id,
        &triggered.effects,
        None,
        &[],
    )
    .expect("Tom Bombadil final chapter ability should resolve");

    let moved_saga_id = game
        .find_object_by_stable_id(saga_stable_id)
        .expect("revealed Saga should still exist");
    assert_eq!(
        game.object(moved_saga_id).expect("moved Saga exists").zone,
        Zone::Battlefield,
        "Tom Bombadil should put the revealed Saga onto the battlefield"
    );
    assert_eq!(
        game.object(top_non_saga_id)
            .expect("non-Saga should still exist")
            .zone,
        Zone::Library,
        "Tom Bombadil should leave revealed non-Saga cards in the library"
    );
    assert_eq!(
        game.player(alice)
            .expect("Alice exists")
            .library
            .first()
            .copied(),
        Some(top_non_saga_id),
        "the revealed non-Saga remainder should be on the bottom of the library"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_saga_etb_adds_lore_counter() {
    use crate::cards::definitions::the_birth_of_meletis;

    // Test that a saga entering the battlefield gets its initial lore counter
    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let mut trigger_queue = TriggerQueue::new();

    // Put saga directly on battlefield (simulating resolution)
    let saga_def = the_birth_of_meletis();
    let saga_id = game.create_object_from_definition(&saga_def, alice, Zone::Battlefield);

    // Add initial lore counter and check chapters (what resolve_stack_entry_full does)
    add_lore_counter_and_check_chapters(&mut game, saga_id, &mut trigger_queue);

    // Verify saga has 1 lore counter
    let saga = game.object(saga_id).unwrap();
    let lore_count = saga.counters.get(&CounterType::Lore).copied().unwrap_or(0);
    assert_eq!(lore_count, 1, "Saga should have 1 lore counter after ETB");

    // Verify chapter 1 trigger is queued
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "Chapter 1 trigger should be in queue"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_saga_chapter_one_leaving_stack_does_not_sacrifice() {
    use crate::cards::definitions::the_birth_of_meletis;

    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let mut trigger_queue = TriggerQueue::new();
    let saga_def = the_birth_of_meletis();
    let saga_id = game.create_object_from_definition(&saga_def, alice, Zone::Battlefield);
    let mut dm = SelectFirstDecisionMaker;

    handle_saga_enters_battlefield(&mut game, saga_id, &mut trigger_queue, &mut dm);
    assert_eq!(
        game.object(saga_id)
            .unwrap()
            .counters
            .get(&CounterType::Lore)
            .copied()
            .unwrap_or(0),
        1
    );
    assert_eq!(trigger_queue.entries.len(), 1);

    put_triggers_on_stack(&mut game, &mut trigger_queue).unwrap();
    game.pop_from_stack();
    check_and_apply_sbas(&mut game, &mut trigger_queue).unwrap();
    assert!(
        game.battlefield.contains(&saga_id),
        "A non-final chapter leaving the stack must not sacrifice the Saga"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_read_ahead_enters_with_choice_and_skips_lower_chapters() {
    struct ChooseSecondOption;

    impl DecisionMaker for ChooseSecondOption {
        fn decide_options(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            if ctx
                .options
                .iter()
                .any(|option| option.legal && option.index == 1)
            {
                vec![1]
            } else {
                ctx.options
                    .iter()
                    .filter(|option| option.legal)
                    .map(|option| option.index)
                    .take(ctx.min)
                    .collect()
            }
        }
    }

    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let mut trigger_queue = TriggerQueue::new();
    let saga_def = CardDefinitionBuilder::new(CardId::from_raw(991_001), "Read Ahead Probe")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Saga])
        .parse_text(
            "Read ahead (Choose a chapter and start with that many lore counters. Add one after your draw step. Skipped chapters don't trigger. Sacrifice after III.)\n\
             I — You gain 1 life.\n\
             II — You gain 2 life.\n\
             III — You gain 3 life.",
        )
        .expect("read ahead Saga should parse");
    let hand_id = game.create_object_from_definition(&saga_def, alice, Zone::Hand);
    let mut dm = ChooseSecondOption;
    let enters = game
        .move_object_with_etb_processing_with_dm(hand_id, Zone::Battlefield, &mut dm)
        .expect("Saga should enter");
    handle_saga_enters_battlefield(&mut game, enters.new_id, &mut trigger_queue, &mut dm);

    assert_eq!(
        game.object(enters.new_id)
            .unwrap()
            .counters
            .get(&CounterType::Lore)
            .copied()
            .unwrap_or(0),
        2
    );
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "Read ahead choosing chapter II should skip chapter I on the entry turn"
    );
    let chapters = trigger_queue.entries[0]
        .ability
        .trigger
        .saga_chapters()
        .expect("queued trigger should be a chapter trigger");
    assert_eq!(chapters, &[2]);
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn scroll_of_isildur_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(993_001), "Scroll of Isildur")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Blue],
        ]))
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Saga])
        .parse_text(
            "(As this Saga enters and after your draw step, add a lore counter. Sacrifice after III.)\n\
             I — Gain control of up to one target artifact for as long as you control this Saga. The Ring tempts you.\n\
             II — Tap up to two target creatures. Put a stun counter on each of them.\n\
             III — Draw a card for each tapped creature target opponent controls.",
        )
        .expect("Scroll of Isildur should parse for runtime tests")
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn scroll_of_isildur_chapter_one_steals_artifact_until_saga_not_controlled_and_tempts_ring()
 {
    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut trigger_queue = TriggerQueue::new();
    let mut dm = SelectFirstDecisionMaker;

    let scroll_id = game.create_object_from_definition(
        &scroll_of_isildur_definition(),
        alice,
        Zone::Battlefield,
    );
    let relic = CardBuilder::new(CardId::from_raw(993_002), "Bob's Relic")
        .card_types(vec![CardType::Artifact])
        .build();
    let relic_id = game.create_object_from_card(&relic, bob, Zone::Battlefield);
    let bearer = CardBuilder::new(CardId::from_raw(993_003), "Ring Candidate")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let bearer_id = game.create_object_from_card(&bearer, alice, Zone::Battlefield);

    add_lore_counter_and_check_chapters(&mut game, scroll_id, &mut trigger_queue);
    put_triggers_on_stack_with_dm(&mut game, &mut trigger_queue, &mut dm)
        .expect("Scroll of Isildur chapter I should go on the stack with an artifact target");
    resolve_stack_entry_with_dm_and_triggers(&mut game, &mut dm, &mut trigger_queue)
        .expect("Scroll of Isildur chapter I should resolve");

    assert_eq!(
        game.current_controller(relic_id),
        Some(alice),
        "chapter I should give Alice control of the targeted artifact"
    );
    assert_eq!(
        game.ring_temptations(alice),
        1,
        "chapter I should make the Ring tempt Alice"
    );
    assert_eq!(
        game.current_ring_bearer(alice),
        Some(bearer_id),
        "chapter I should choose Alice's only creature as Ring-bearer"
    );

    game.set_current_controller(scroll_id, bob);
    game.refresh_continuous_state();
    assert_eq!(
        game.current_controller(relic_id),
        Some(bob),
        "the control-changing effect should expire once Alice no longer controls the Saga"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn scroll_of_isildur_chapters_two_and_three_target_tap_stun_and_draw_count() {
    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut trigger_queue = TriggerQueue::new();
    let mut dm = SelectFirstDecisionMaker;

    let scroll_id = game.create_object_from_definition(
        &scroll_of_isildur_definition(),
        alice,
        Zone::Battlefield,
    );
    game.object_mut(scroll_id)
        .expect("Scroll of Isildur should exist")
        .add_counters(crate::object::CounterType::Lore, 1);

    let creature = |id, name| {
        CardBuilder::new(CardId::from_raw(id), name)
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build()
    };
    let bob_first = game.create_object_from_card(
        &creature(993_004, "First Bob Creature"),
        bob,
        Zone::Battlefield,
    );
    let bob_second = game.create_object_from_card(
        &creature(993_005, "Second Bob Creature"),
        bob,
        Zone::Battlefield,
    );
    let bob_third = game.create_object_from_card(
        &creature(993_006, "Third Bob Creature"),
        bob,
        Zone::Battlefield,
    );
    let alice_creature = game.create_object_from_card(
        &creature(993_007, "Alice Creature"),
        alice,
        Zone::Battlefield,
    );
    for idx in 0..4 {
        let card =
            CardBuilder::new(CardId::from_raw(993_010 + idx), format!("Draw Card {idx}")).build();
        game.create_object_from_card(&card, alice, Zone::Library);
    }
    let initial_hand_size = game.player(alice).expect("Alice exists").hand.len();

    add_lore_counter_and_check_chapters(&mut game, scroll_id, &mut trigger_queue);
    put_triggers_on_stack_with_dm(&mut game, &mut trigger_queue, &mut dm)
        .expect("Scroll of Isildur chapter II should go on the stack with creature targets");
    resolve_stack_entry_with_dm_and_triggers(&mut game, &mut dm, &mut trigger_queue)
        .expect("Scroll of Isildur chapter II should resolve");

    assert!(
        game.is_tapped(bob_first),
        "chapter II should tap the first chosen creature"
    );
    assert!(
        game.is_tapped(bob_second),
        "chapter II should tap the second chosen creature"
    );
    assert!(
        !game.is_tapped(bob_third),
        "chapter II is up to two targets and should not tap a third creature"
    );
    assert!(
        !game.is_tapped(alice_creature),
        "chapter II should only affect the two targets chosen by the decision maker"
    );
    assert_eq!(
        game.counter_count(bob_first, crate::object::CounterType::Stun),
        1,
        "chapter II should put a stun counter on the first tapped target"
    );
    assert_eq!(
        game.counter_count(bob_second, crate::object::CounterType::Stun),
        1,
        "chapter II should put a stun counter on the second tapped target"
    );
    assert_eq!(
        game.counter_count(bob_third, crate::object::CounterType::Stun),
        0,
        "chapter II should not put a stun counter on an unchosen creature"
    );

    game.tap(alice_creature);
    add_lore_counter_and_check_chapters(&mut game, scroll_id, &mut trigger_queue);
    put_triggers_on_stack_with_dm(&mut game, &mut trigger_queue, &mut dm)
        .expect("Scroll of Isildur chapter III should go on the stack");
    resolve_stack_entry_with_dm_and_triggers(&mut game, &mut dm, &mut trigger_queue)
        .expect("Scroll of Isildur chapter III should resolve");

    assert_eq!(
        game.player(alice).expect("Alice exists").hand.len(),
        initial_hand_size + 2,
        "chapter III should draw for tapped creatures the target opponent controls, not Alice's tapped creature"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn the_aesir_escape_valhalla_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(992_001), "The Aesir Escape Valhalla")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Green],
        ]))
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Saga])
        .parse_text(
            "I — Exile a permanent card from your graveyard. You gain life equal to its mana value.\n\
             II — Put a number of +1/+1 counters on target creature you control equal to the mana value of the exiled card.\n\
             III — Return this Saga and the exiled card to their owner's hand.",
        )
        .expect("The Aesir Escape Valhalla should parse")
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn the_aesir_escape_valhalla_chapters_use_exiled_card_mana_value_and_return_pair() {
    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut trigger_queue = TriggerQueue::new();
    let mut dm = SelectFirstDecisionMaker;

    let saga_def = the_aesir_escape_valhalla_definition();
    let saga_id = game.create_object_from_definition(&saga_def, alice, Zone::Battlefield);
    let saga_stable_id = game.object(saga_id).expect("saga exists").stable_id;

    let exiled_card = CardBuilder::new(CardId::from_raw(992_002), "Valhalla Relic")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(4)]]))
        .card_types(vec![CardType::Artifact])
        .build();
    let graveyard_id = game.create_object_from_card(&exiled_card, alice, Zone::Graveyard);
    let exiled_stable_id = game
        .object(graveyard_id)
        .expect("graveyard card exists")
        .stable_id;

    let target_creature = CardBuilder::new(CardId::from_raw(992_003), "Small Ally")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)]]))
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let target_id = game.create_object_from_card(&target_creature, alice, Zone::Battlefield);
    let opponent_creature = CardBuilder::new(CardId::from_raw(992_004), "Opposing Ally")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(7)]]))
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let opponent_target_id =
        game.create_object_from_card(&opponent_creature, bob, Zone::Battlefield);

    add_lore_counter_and_check_chapters(&mut game, saga_id, &mut trigger_queue);
    put_triggers_on_stack_with_dm(&mut game, &mut trigger_queue, &mut dm)
        .expect("chapter I should go on the stack");
    resolve_stack_entry_with_dm_and_triggers(&mut game, &mut dm, &mut trigger_queue)
        .expect("chapter I should resolve");

    let exiled_id = game
        .find_object_by_stable_id(exiled_stable_id)
        .expect("exiled card should still exist");
    assert_eq!(
        game.object(exiled_id).expect("exiled card exists").zone,
        Zone::Exile
    );
    assert_eq!(
        game.player(alice).expect("alice exists").life,
        24,
        "chapter I should gain life equal to the exiled card's mana value"
    );

    add_lore_counter_and_check_chapters(&mut game, saga_id, &mut trigger_queue);
    put_triggers_on_stack_with_dm(&mut game, &mut trigger_queue, &mut dm)
        .expect("chapter II should go on the stack with a legal controlled target");
    resolve_stack_entry_with_dm_and_triggers(&mut game, &mut dm, &mut trigger_queue)
        .expect("chapter II should resolve");

    assert_eq!(
        game.counter_count(target_id, CounterType::PlusOnePlusOne),
        4,
        "chapter II should use the exiled card's mana value, not the target creature's mana value"
    );
    assert_eq!(
        game.counter_count(opponent_target_id, CounterType::PlusOnePlusOne),
        0,
        "chapter II's target restriction should not put counters on an opponent's creature"
    );

    add_lore_counter_and_check_chapters(&mut game, saga_id, &mut trigger_queue);
    put_triggers_on_stack_with_dm(&mut game, &mut trigger_queue, &mut dm)
        .expect("chapter III should go on the stack");
    resolve_stack_entry_with_dm_and_triggers(&mut game, &mut dm, &mut trigger_queue)
        .expect("chapter III should resolve");

    let returned_saga_id = game
        .find_object_by_stable_id(saga_stable_id)
        .expect("saga should still exist after returning");
    let returned_exiled_id = game
        .find_object_by_stable_id(exiled_stable_id)
        .expect("exiled card should still exist after returning");
    assert_eq!(
        game.object(returned_saga_id)
            .expect("returned saga exists")
            .zone,
        Zone::Hand,
        "chapter III should return this Saga to its owner's hand"
    );
    assert_eq!(
        game.object(returned_exiled_id)
            .expect("returned exiled card exists")
            .zone,
        Zone::Hand,
        "chapter III should return the exiled card to its owner's hand"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn the_aesir_escape_valhalla_chapter_two_requires_a_creature_you_control_target() {
    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut trigger_queue = TriggerQueue::new();
    let mut dm = SelectFirstDecisionMaker;

    let saga_def = the_aesir_escape_valhalla_definition();
    let saga_id = game.create_object_from_definition(&saga_def, alice, Zone::Battlefield);

    let exiled_card = CardBuilder::new(CardId::from_raw(992_005), "Valhalla Relic")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(4)]]))
        .card_types(vec![CardType::Artifact])
        .build();
    game.create_object_from_card(&exiled_card, alice, Zone::Graveyard);

    let opponent_creature = CardBuilder::new(CardId::from_raw(992_006), "Opposing Ally")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(7)]]))
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let opponent_target_id =
        game.create_object_from_card(&opponent_creature, bob, Zone::Battlefield);

    add_lore_counter_and_check_chapters(&mut game, saga_id, &mut trigger_queue);
    put_triggers_on_stack_with_dm(&mut game, &mut trigger_queue, &mut dm)
        .expect("chapter I should go on the stack");
    resolve_stack_entry_with_dm_and_triggers(&mut game, &mut dm, &mut trigger_queue)
        .expect("chapter I should resolve");

    add_lore_counter_and_check_chapters(&mut game, saga_id, &mut trigger_queue);
    put_triggers_on_stack_with_dm(&mut game, &mut trigger_queue, &mut dm)
        .expect("chapter II target selection should complete even with no legal target");

    assert_eq!(
        game.stack.len(),
        0,
        "chapter II should not go on the stack without a target creature you control"
    );
    assert_eq!(
        game.counter_count(opponent_target_id, CounterType::PlusOnePlusOne),
        0,
        "chapter II must not target an opponent's creature to satisfy its target requirement"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn the_aesir_escape_valhalla_without_exiled_card_adds_no_counters() {
    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let mut trigger_queue = TriggerQueue::new();
    let mut dm = SelectFirstDecisionMaker;

    let saga_def = the_aesir_escape_valhalla_definition();
    let saga_id = game.create_object_from_definition(&saga_def, alice, Zone::Battlefield);
    let saga_stable_id = game.object(saga_id).expect("saga exists").stable_id;

    let target_creature = CardBuilder::new(CardId::from_raw(992_007), "Small Ally")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)]]))
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let target_id = game.create_object_from_card(&target_creature, alice, Zone::Battlefield);

    add_lore_counter_and_check_chapters(&mut game, saga_id, &mut trigger_queue);
    put_triggers_on_stack_with_dm(&mut game, &mut trigger_queue, &mut dm)
        .expect("chapter I should go on the stack");
    resolve_stack_entry_with_dm_and_triggers(&mut game, &mut dm, &mut trigger_queue)
        .expect("chapter I should resolve with no graveyard permanent to exile");

    assert_eq!(
        game.player(alice).expect("alice exists").life,
        20,
        "chapter I should not gain life when no card was exiled"
    );
    assert!(game.exile.is_empty(), "chapter I should not exile a card");

    add_lore_counter_and_check_chapters(&mut game, saga_id, &mut trigger_queue);
    put_triggers_on_stack_with_dm(&mut game, &mut trigger_queue, &mut dm)
        .expect("chapter II should go on the stack with a legal target");
    resolve_stack_entry_with_dm_and_triggers(&mut game, &mut dm, &mut trigger_queue)
        .expect("chapter II should resolve without an exiled-card amount");

    assert_eq!(
        game.counter_count(target_id, CounterType::PlusOnePlusOne),
        0,
        "chapter II should add no counters when chapter I did not exile a card"
    );

    add_lore_counter_and_check_chapters(&mut game, saga_id, &mut trigger_queue);
    put_triggers_on_stack_with_dm(&mut game, &mut trigger_queue, &mut dm)
        .expect("chapter III should go on the stack");
    resolve_stack_entry_with_dm_and_triggers(&mut game, &mut dm, &mut trigger_queue)
        .expect("chapter III should resolve without an exiled card");

    let returned_saga_id = game
        .find_object_by_stable_id(saga_stable_id)
        .expect("saga should still exist after returning");
    assert_eq!(
        game.object(returned_saga_id)
            .expect("returned saga exists")
            .zone,
        Zone::Hand,
        "chapter III should still return this Saga to its owner's hand"
    );
    assert!(
        game.exile.is_empty(),
        "chapter III should not leave or create an exiled-card object"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_saga_precombat_main_adds_lore_counter() {
    use crate::cards::definitions::the_birth_of_meletis;

    // Test that sagas get a lore counter at precombat main phase
    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let mut trigger_queue = TriggerQueue::new();

    // Put saga on battlefield with 1 lore counter already (simulating after ETB)
    let saga_def = the_birth_of_meletis();
    let saga_id = game.create_object_from_definition(&saga_def, alice, Zone::Battlefield);
    game.object_mut(saga_id)
        .unwrap()
        .add_counters(CounterType::Lore, 1);

    // Simulate precombat main phase - add lore counters to sagas
    add_saga_lore_counters(&mut game, &mut trigger_queue);

    // Verify saga now has 2 lore counters
    let saga = game.object(saga_id).unwrap();
    let lore_count = saga.counters.get(&CounterType::Lore).copied().unwrap_or(0);
    assert_eq!(
        lore_count, 2,
        "Saga should have 2 lore counters after precombat main"
    );

    // Verify chapter 2 trigger is queued (threshold crossed from 1 to 2)
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "Chapter 2 trigger should be in queue"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_urzas_saga_keeps_chapter_one_mana_ability_after_chapter_two() {
    use crate::ability::AbilityKind;
    use crate::cards::definitions::urzas_saga;

    fn granted_activated_counts(game: &GameState, saga_id: ObjectId) -> (usize, usize) {
        let abilities = game
            .current_abilities(saga_id)
            .expect("Urza's Saga should have current abilities");
        abilities
            .iter()
            .filter_map(|ability| match &ability.kind {
                AbilityKind::Activated(activated) => Some(activated.is_mana_ability()),
                _ => None,
            })
            .fold((0usize, 0usize), |(mana, non_mana), is_mana| {
                if is_mana {
                    (mana + 1, non_mana)
                } else {
                    (mana, non_mana + 1)
                }
            })
    }

    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    game.turn.active_player = alice;
    let mut trigger_queue = TriggerQueue::new();
    let mut dm = SelectFirstDecisionMaker;

    let saga_def = urzas_saga();
    let saga_id = game.create_object_from_definition(&saga_def, alice, Zone::Battlefield);

    handle_saga_enters_battlefield(&mut game, saga_id, &mut trigger_queue, &mut dm);
    put_triggers_on_stack(&mut game, &mut trigger_queue).unwrap();
    resolve_stack_entry(&mut game).expect("Urza's Saga chapter I should resolve");

    assert_eq!(
        granted_activated_counts(&game, saga_id),
        (1, 0),
        "chapter I should grant the colorless mana ability"
    );

    game.effect_store.continuous_effects.cleanup_end_of_turn();
    game.refresh_continuous_state();
    assert_eq!(
        granted_activated_counts(&game, saga_id),
        (1, 0),
        "Urza's Saga chapter I grant has no until-end-of-turn duration"
    );

    add_saga_lore_counters(&mut game, &mut trigger_queue);
    put_triggers_on_stack(&mut game, &mut trigger_queue).unwrap();
    resolve_stack_entry(&mut game).expect("Urza's Saga chapter II should resolve");

    assert_eq!(
        granted_activated_counts(&game, saga_id),
        (1, 1),
        "after chapter II, Urza's Saga should have both the chapter I mana ability and chapter II token ability"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_saga_final_chapter_waits_for_pending_and_stacked_chapter_ability() {
    use crate::cards::definitions::the_birth_of_meletis;
    use crate::rules::state_based::{
        StateBasedAction, StateBasedActionContext, check_state_based_actions_with_context,
    };

    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let mut trigger_queue = TriggerQueue::new();

    let saga_def = the_birth_of_meletis();
    let saga_id = game.create_object_from_definition(&saga_def, alice, Zone::Battlefield);
    game.object_mut(saga_id)
        .unwrap()
        .add_counters(CounterType::Lore, 2);

    add_saga_lore_counters(&mut game, &mut trigger_queue);
    assert_eq!(trigger_queue.entries.len(), 1);

    game.refresh_continuous_state();
    let view = crate::derived_view::DerivedGameView::from_refreshed_state(&game);
    let context = StateBasedActionContext::from_trigger_queue(&trigger_queue);
    let pending_sbas = check_state_based_actions_with_context(&game, &view, &context);
    assert!(
        !pending_sbas
            .iter()
            .any(|sba| matches!(sba, StateBasedAction::SagaSacrifice(id) if *id == saga_id)),
        "Saga should not be sacrificed while its final chapter ability is still pending"
    );
    drop(view);

    put_triggers_on_stack(&mut game, &mut trigger_queue).unwrap();
    assert_eq!(game.stack.len(), 1);
    check_and_apply_sbas(&mut game, &mut trigger_queue).unwrap();
    assert!(
        game.battlefield.contains(&saga_id),
        "Saga should not be sacrificed while its final chapter ability is on the stack"
    );
    game.pop_from_stack();
    check_and_apply_sbas(&mut game, &mut trigger_queue).unwrap();
    assert!(
        !game.battlefield.contains(&saga_id),
        "Saga should be sacrificed once the final chapter ability has left the stack"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_saga_full_lifecycle() {
    use crate::cards::definitions::the_birth_of_meletis;

    // Test the full saga lifecycle: ETB -> chapter triggers -> sacrifice
    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let mut trigger_queue = TriggerQueue::new();

    // Create saga and simulate entering battlefield
    let saga_def = the_birth_of_meletis();
    let saga_id = game.create_object_from_definition(&saga_def, alice, Zone::Battlefield);

    // Add initial lore counter and check chapters
    add_lore_counter_and_check_chapters(&mut game, saga_id, &mut trigger_queue);

    // Verify: 1 lore counter, chapter 1 triggered
    let saga = game.object(saga_id).unwrap();
    assert_eq!(
        saga.counters.get(&CounterType::Lore).copied().unwrap_or(0),
        1
    );
    assert_eq!(trigger_queue.entries.len(), 1);

    // Clear trigger queue (simulating triggers going on stack and resolving)
    trigger_queue.clear();

    // Simulate turn 2 - add lore counter at precombat main
    add_saga_lore_counters(&mut game, &mut trigger_queue);

    // Verify: 2 lore counters, chapter 2 triggered
    let saga = game.object(saga_id).unwrap();
    assert_eq!(
        saga.counters.get(&CounterType::Lore).copied().unwrap_or(0),
        2
    );
    assert_eq!(trigger_queue.entries.len(), 1);

    // Clear trigger queue
    trigger_queue.clear();

    // Simulate turn 3 - add lore counter at precombat main (final chapter)
    add_saga_lore_counters(&mut game, &mut trigger_queue);

    // Verify: 3 lore counters, chapter 3 triggered
    let saga = game.object(saga_id).unwrap();
    assert_eq!(
        saga.counters.get(&CounterType::Lore).copied().unwrap_or(0),
        3
    );
    assert_eq!(trigger_queue.entries.len(), 1);

    put_triggers_on_stack(&mut game, &mut trigger_queue).unwrap();
    assert_eq!(game.stack.len(), 1);
    game.pop_from_stack();

    // Apply SBAs - saga should be sacrificed
    check_and_apply_sbas(&mut game, &mut trigger_queue).unwrap();

    // Verify saga is no longer on battlefield
    assert!(
        !game.battlefield.contains(&saga_id),
        "Saga should no longer be on battlefield"
    );

    // Verify saga is in graveyard (note: zone change creates new object ID per rule 400.7)
    let alice_player = game.player(alice).unwrap();
    let saga_in_graveyard = alice_player.graveyard.iter().any(|&id| {
        game.object(id)
            .map(|o| o.name == "The Birth of Meletis")
            .unwrap_or(false)
    });
    assert!(
        saga_in_graveyard,
        "Saga should be in graveyard after sacrifice"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_saga_survives_when_lore_counter_removed() {
    use crate::cards::definitions::{hex_parasite, ornithopter, urzas_saga};
    use crate::effects::execute_effect;

    // Test that removing a lore counter from a saga at its final chapter prevents sacrifice
    // This simulates: Urza's Saga with 2 counters, gets 3rd counter (final chapter),
    // respond with Hex Parasite to remove a counter, saga survives
    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let mut trigger_queue = TriggerQueue::new();

    // Put Urza's Saga on battlefield with 2 lore counters
    let saga_def = urzas_saga();
    let saga_id = game.create_object_from_definition(&saga_def, alice, Zone::Battlefield);
    game.object_mut(saga_id)
        .unwrap()
        .add_counters(CounterType::Lore, 2);

    // Put Hex Parasite on battlefield (not summoning sick for this test)
    let parasite_def = hex_parasite();
    let parasite_id = game.create_object_from_definition(&parasite_def, alice, Zone::Battlefield);
    game.remove_summoning_sickness(parasite_id);

    // Put Ornithopter in library (for Urza's Saga to find)
    let ornithopter_def = ornithopter();
    let _ornithopter_id =
        game.create_object_from_definition(&ornithopter_def, alice, Zone::Library);

    // Verify initial state
    assert_eq!(
        game.object(saga_id)
            .unwrap()
            .counters
            .get(&CounterType::Lore)
            .copied()
            .unwrap_or(0),
        2,
        "Saga should start with 2 lore counters"
    );

    // Simulate precombat main phase - saga gets 3rd lore counter (final chapter)
    add_saga_lore_counters(&mut game, &mut trigger_queue);

    // Verify saga now has 3 lore counters and chapter 3 triggered
    let saga = game.object(saga_id).unwrap();
    assert_eq!(
        saga.counters.get(&CounterType::Lore).copied().unwrap_or(0),
        3,
        "Saga should have 3 lore counters"
    );
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "Chapter 3 trigger should be in queue"
    );

    // The chapter 3 trigger is now in the queue, but BEFORE it resolves,
    // we respond by activating Hex Parasite to remove a lore counter.
    // (In a real game, the trigger would go on the stack, and we'd respond)

    // Simulate Hex Parasite's ability: remove 1 lore counter from Urza's Saga
    // (Paying 2 life for the phyrexian black mana)
    let remove_effect = Effect::remove_counters(
        CounterType::Lore,
        1, // Remove 1 counter (X=1)
        ChooseSpec::SpecificObject(saga_id),
    );
    let mut ctx = ExecutionContext::new_default(parasite_id, alice)
        .with_x(1)
        .with_targets(vec![crate::effects::ResolvedTarget::Object(saga_id)]);
    let result = execute_effect(&mut game, &remove_effect, &mut ctx);
    assert!(result.is_ok(), "Counter removal should succeed");

    // Pay the life cost (2 life for phyrexian black)
    game.player_mut(alice).unwrap().life -= 2;

    // Verify saga now has 2 lore counters (not 3)
    let saga = game.object(saga_id).unwrap();
    assert_eq!(
        saga.counters.get(&CounterType::Lore).copied().unwrap_or(0),
        2,
        "Saga should have 2 lore counters after Hex Parasite"
    );

    // Now the chapter 3 trigger resolves - search for artifact with MV 0 or 1
    // For this test, we'll manually resolve it
    // Create a decision maker that selects the ornithopter
    struct SelectOrnithopterDecisionMaker;
    impl DecisionMaker for SelectOrnithopterDecisionMaker {
        fn decide_objects(
            &mut self,
            game: &GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            // Find ornithopter in candidates
            ctx.candidates
                .iter()
                .filter(|c| c.legal)
                .find(|c| {
                    game.object(c.id)
                        .map(|o| o.name == "Ornithopter")
                        .unwrap_or(false)
                })
                .map(|c| vec![c.id])
                .unwrap_or_default()
        }
    }

    let search_effect = Effect::search_library(
        crate::target::ObjectFilter {
            card_types: vec![CardType::Artifact],
            mana_value: Some(crate::target::Comparison::LessThanOrEqual(1)),
            ..Default::default()
        },
        Zone::Battlefield,
        crate::target::PlayerFilter::You,
        false,
    );
    let mut dm = SelectOrnithopterDecisionMaker;
    let mut ctx = ExecutionContext::new_default(saga_id, alice).with_decision_maker(&mut dm);
    let result = execute_effect(&mut game, &search_effect, &mut ctx);
    assert!(result.is_ok(), "Search should succeed");

    let saga = game.object(saga_id).unwrap();
    assert_eq!(
        saga.counters.get(&CounterType::Lore).copied().unwrap_or(0),
        2,
        "Saga should still have only 2 lore counters"
    );

    // Now check SBAs - the saga should NOT be sacrificed because it doesn't have
    // enough lore counters (need 3, only has 2)
    check_and_apply_sbas(&mut game, &mut trigger_queue).unwrap();

    // Verify saga is STILL on the battlefield
    assert!(
        game.battlefield.contains(&saga_id),
        "Saga should STILL be on battlefield - it survived because lore counter was removed!"
    );

    // Verify Ornithopter is on the battlefield (it was fetched)
    let ornithopter_on_battlefield = game.battlefield.iter().any(|&id| {
        game.object(id)
            .map(|o| o.name == "Ornithopter")
            .unwrap_or(false)
    });
    assert!(
        ornithopter_on_battlefield,
        "Ornithopter should be on battlefield (fetched by Urza's Saga)"
    );

    // Verify Hex Parasite is still on battlefield
    assert!(
        game.battlefield.contains(&parasite_id),
        "Hex Parasite should still be on battlefield"
    );

    // Verify Alice paid 2 life
    assert_eq!(
        game.player(alice).unwrap().life,
        18,
        "Alice should have 18 life (paid 2 for Hex Parasite)"
    );

    // Final summary of board state
    println!("Board state after Hex Parasite saves Urza's Saga:");
    println!("- Urza's Saga: on battlefield with 2 lore counters");
    println!("- Hex Parasite: on battlefield");
    println!("- Ornithopter: on battlefield (fetched)");
    println!("- Alice's life: 18");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_saga_chapter_triggers_again_after_counter_removed() {
    use crate::cards::definitions::urzas_saga;

    // Test scenario: Hex Parasite + Urza's Saga
    // 1. Urza's Saga has 2 lore counters
    // 2. Precombat main: lore counter added (now 3), Chapter III triggers
    // 3. In response: remove a lore counter (now 2)
    // 4. Chapter III resolves (saga survives because 2 < 3)
    // 5. NEXT TURN: lore counter added (now 3), Chapter III should trigger AGAIN
    // 6. Chapter III resolves, saga gets sacrificed
    //
    // This tests MTG Rule 714.2c: chapters can trigger multiple times if the
    // threshold is crossed multiple times (e.g., by removing and re-adding counters).

    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let mut trigger_queue = TriggerQueue::new();

    // Put Urza's Saga on battlefield with 2 lore counters
    let saga_def = urzas_saga();
    let saga_id = game.create_object_from_definition(&saga_def, alice, Zone::Battlefield);
    game.object_mut(saga_id)
        .unwrap()
        .add_counters(CounterType::Lore, 2);

    // Set active player to Alice (needed for add_saga_lore_counters)
    game.turn.active_player = alice;

    // --- TURN 1: Precombat main phase ---
    // Add lore counter (2 -> 3), Chapter III triggers
    add_saga_lore_counters(&mut game, &mut trigger_queue);

    assert_eq!(
        game.object(saga_id)
            .unwrap()
            .counters
            .get(&CounterType::Lore)
            .copied()
            .unwrap_or(0),
        3,
        "Turn 1: Saga should have 3 lore counters after precombat main"
    );
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "Turn 1: Chapter III should have triggered"
    );

    // Simulate responding with Hex Parasite: remove 1 lore counter
    game.object_mut(saga_id)
        .unwrap()
        .remove_counters(CounterType::Lore, 1);

    assert_eq!(
        game.object(saga_id)
            .unwrap()
            .counters
            .get(&CounterType::Lore)
            .copied()
            .unwrap_or(0),
        2,
        "Turn 1: Saga should have 2 lore counters after Hex Parasite"
    );

    put_triggers_on_stack(&mut game, &mut trigger_queue).unwrap();
    game.pop_from_stack();

    // Check SBAs - saga should survive because 2 < 3
    check_and_apply_sbas(&mut game, &mut trigger_queue).unwrap();
    assert!(
        game.battlefield.contains(&saga_id),
        "Turn 1: Saga should survive - only has 2 lore counters"
    );

    // --- TURN 2: Precombat main phase ---
    // Add lore counter (2 -> 3), Chapter III should trigger AGAIN!
    // This is the key test: the threshold crossing logic should allow re-triggering
    add_saga_lore_counters(&mut game, &mut trigger_queue);

    assert_eq!(
        game.object(saga_id)
            .unwrap()
            .counters
            .get(&CounterType::Lore)
            .copied()
            .unwrap_or(0),
        3,
        "Turn 2: Saga should have 3 lore counters"
    );
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "Turn 2: Chapter III should have triggered AGAIN (threshold crossed again)"
    );

    put_triggers_on_stack(&mut game, &mut trigger_queue).unwrap();
    game.pop_from_stack();

    // Check SBAs - saga should now be sacrificed because 3 >= 3
    check_and_apply_sbas(&mut game, &mut trigger_queue).unwrap();
    assert!(
        !game.battlefield.contains(&saga_id),
        "Turn 2: Saga should be sacrificed - has 3 lore counters"
    );

    println!("Test passed: Chapter III triggered twice after counter manipulation!");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_urzas_saga_excludes_x_cost_artifacts() {
    use crate::cards::definitions::{everflowing_chalice, ornithopter, urzas_saga};
    use crate::effects::execute_effect;
    use crate::target::FilterContext;

    // Test that Urza's Saga's search filter correctly excludes X-cost artifacts
    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);

    // Put Urza's Saga on battlefield
    let saga_def = urzas_saga();
    let saga_id = game.create_object_from_definition(&saga_def, alice, Zone::Battlefield);

    // Put Everflowing Chalice in library (has X in its cost)
    let chalice_def = everflowing_chalice();
    let chalice_id = game.create_object_from_definition(&chalice_def, alice, Zone::Library);

    // Put Ornithopter in library (no X in cost, mana value 0)
    let ornithopter_def = ornithopter();
    let _ornithopter_id =
        game.create_object_from_definition(&ornithopter_def, alice, Zone::Library);

    // Create the filter from Urza's Saga chapter III
    let filter = crate::target::ObjectFilter {
        card_types: vec![CardType::Artifact],
        mana_value: Some(crate::target::Comparison::LessThanOrEqual(1)),
        has_mana_cost: true,
        no_x_in_cost: true,
        ..Default::default()
    };

    let ctx = FilterContext::new(alice).with_source(saga_id);

    // Verify Everflowing Chalice does NOT match (has X in cost)
    let chalice_obj = game.object(chalice_id).unwrap();
    assert!(
        !filter.matches(chalice_obj, &ctx, &game),
        "Everflowing Chalice should NOT match - has X in cost"
    );

    // Verify Ornithopter DOES match (mana value 0, no X, has mana cost)
    let ornithopter_obj = game
        .player(alice)
        .unwrap()
        .library
        .iter()
        .find_map(|&id| {
            let obj = game.object(id)?;
            if obj.name == "Ornithopter" {
                Some(obj)
            } else {
                None
            }
        })
        .unwrap();
    assert!(
        filter.matches(ornithopter_obj, &ctx, &game),
        "Ornithopter SHOULD match - mana value 0, no X, has mana cost"
    );

    // Now test the full search effect
    struct SelectFirstMatchDecisionMaker;
    impl DecisionMaker for SelectFirstMatchDecisionMaker {
        fn decide_objects(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            ctx.candidates
                .iter()
                .filter(|c| c.legal)
                .map(|c| c.id)
                .take(1)
                .collect()
        }
    }

    let search_effect = Effect::search_library(
        filter,
        Zone::Battlefield,
        crate::target::PlayerFilter::You,
        false,
    );

    let mut dm = SelectFirstMatchDecisionMaker;
    let mut ctx = ExecutionContext::new_default(saga_id, alice).with_decision_maker(&mut dm);
    let result = execute_effect(&mut game, &search_effect, &mut ctx);
    assert!(result.is_ok(), "Search should succeed");

    // Verify Ornithopter is on battlefield (should have been selected)
    let ornithopter_on_battlefield = game.battlefield.iter().any(|&id| {
        game.object(id)
            .map(|o| o.name == "Ornithopter")
            .unwrap_or(false)
    });
    assert!(
        ornithopter_on_battlefield,
        "Ornithopter should be on battlefield"
    );

    // Verify Everflowing Chalice is NOT on battlefield (should not have been searchable)
    let chalice_on_battlefield = game.battlefield.iter().any(|&id| {
        game.object(id)
            .map(|o| o.name == "Everflowing Chalice")
            .unwrap_or(false)
    });
    assert!(
        !chalice_on_battlefield,
        "Everflowing Chalice should NOT be on battlefield - has X in cost"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_hex_parasite_pump_effect() {
    use crate::cards::definitions::{hex_parasite, the_birth_of_meletis};
    use crate::effects::execute_effect;

    // Test that Hex Parasite gets +1/+0 for each counter removed
    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);

    // Put a saga on battlefield with some lore counters
    let saga_def = the_birth_of_meletis();
    let saga_id = game.create_object_from_definition(&saga_def, alice, Zone::Battlefield);
    game.object_mut(saga_id)
        .unwrap()
        .add_counters(CounterType::Lore, 2);

    // Put Hex Parasite on battlefield
    let parasite_def = hex_parasite();
    let parasite_id = game.create_object_from_definition(&parasite_def, alice, Zone::Battlefield);
    game.remove_summoning_sickness(parasite_id);

    // Verify initial state - Hex Parasite is 1/1
    let parasite = game.object(parasite_id).unwrap();
    assert_eq!(parasite.power().unwrap(), 1, "Hex Parasite base power is 1");
    assert_eq!(
        parasite.toughness().unwrap(),
        1,
        "Hex Parasite base toughness is 1"
    );

    // Execute the counter removal + pump effect sequence
    // First, remove 2 counters (X=2)
    let remove_effect = Effect::with_id(
        0,
        Effect::remove_counters(
            CounterType::Lore,
            2,
            crate::target::ChooseSpec::SpecificObject(saga_id),
        ),
    );

    let mut ctx = ExecutionContext::new_default(parasite_id, alice)
        .with_x(2)
        .with_targets(vec![crate::effects::ResolvedTarget::Object(saga_id)]);
    let result = execute_effect(&mut game, &remove_effect, &mut ctx);
    assert!(result.is_ok(), "Counter removal should succeed");

    // Check that 2 counters were removed
    assert_eq!(
        result.unwrap().as_count().unwrap_or(0),
        2,
        "Should have removed 2 counters"
    );

    // Now execute the pump effect (which uses the stored result)
    let pump_effect = Effect::if_then(
        crate::effect::EffectId(0),
        crate::effect::EffectPredicate::Happened,
        vec![Effect::pump(
            Value::EffectValue(crate::effect::EffectId(0)),
            Value::Fixed(0),
            crate::target::ChooseSpec::Source,
            crate::effect::Until::EndOfTurn,
        )],
    );

    let result = execute_effect(&mut game, &pump_effect, &mut ctx);
    assert!(result.is_ok(), "Pump effect should succeed");

    // Verify the continuous effect was added
    let effects = game
        .effect_store
        .continuous_effects
        .effects_for_object(parasite_id);
    assert!(
        !effects.is_empty(),
        "Should have a continuous effect on Hex Parasite"
    );

    // Verify the effect is a +2/+0 modification
    let pump_effect = effects.iter().find(|e| {
        matches!(
            &e.modification,
            crate::continuous::Modification::ModifyPowerToughness {
                power: 2,
                toughness: 0
            }
        )
    });
    assert!(
        pump_effect.is_some(),
        "Should have a +2/+0 continuous effect"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_remove_up_to_counters_player_choice() {
    use crate::cards::definitions::the_birth_of_meletis;
    use crate::effects::execute_effect;

    // Test that RemoveUpToCounters allows player to choose how many counters to remove
    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);

    // Put a saga on battlefield with 3 lore counters
    let saga_def = the_birth_of_meletis();
    let saga_id = game.create_object_from_definition(&saga_def, alice, Zone::Battlefield);
    game.object_mut(saga_id)
        .unwrap()
        .add_counters(CounterType::Lore, 3);

    // Create a decision maker that chooses to remove only 1 counter (not the max)
    struct ChooseOneDecisionMaker;
    impl DecisionMaker for ChooseOneDecisionMaker {
        fn decide_number(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::NumberContext,
        ) -> u32 {
            // Verify the range is correct (0 to 3, since X=5 but only 3 available)
            assert_eq!(ctx.min, 0, "Min should be 0 for 'up to' effect");
            assert_eq!(ctx.max, 3, "Max should be 3 (number available)");
            // Choose to remove only 1 counter
            1
        }
    }

    let source_id = game.new_object_id();
    let mut dm = ChooseOneDecisionMaker;
    let mut ctx = ExecutionContext::new_default(source_id, alice)
        .with_x(5) // Pay X=5, but only 3 counters available
        .with_targets(vec![crate::effects::ResolvedTarget::Object(saga_id)])
        .with_decision_maker(&mut dm);

    // Use RemoveUpToCounters - player should be able to choose 0-3
    let effect = Effect::remove_up_to_counters(
        CounterType::Lore,
        Value::X,
        crate::target::ChooseSpec::SpecificObject(saga_id),
    );

    let result = execute_effect(&mut game, &effect, &mut ctx);
    assert!(result.is_ok(), "Effect should succeed");

    // Verify only 1 counter was removed (player's choice)
    let removed = result.unwrap().as_count().unwrap_or(0);
    assert_eq!(
        removed, 1,
        "Should have removed exactly 1 counter (player's choice)"
    );

    // Verify saga still has 2 lore counters
    let saga = game.object(saga_id).unwrap();
    assert_eq!(
        saga.counters.get(&CounterType::Lore).copied().unwrap_or(0),
        2,
        "Saga should have 2 lore counters remaining"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn medomais_prophecy_chapter_three_triggers_only_for_first_named_cast() {
    fn cast_event(spell: ObjectId, caster: PlayerId) -> TriggerEvent {
        TriggerEvent::new_with_provenance(
            crate::events::spells::SpellCastEvent::new(spell, caster, Zone::Hand),
            crate::provenance::ProvNodeId::default(),
        )
    }

    let definition = CardDefinitionBuilder::new(CardId::from_raw(73_500), "Medomai's Prophecy")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::Blue],
        ]))
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Saga])
        .parse_text(
            "(As this Saga enters and after your draw step, add a lore counter. Sacrifice after IV.)\n\
             I — Scry 2.\n\
             II — Choose a card name.\n\
             III — When you cast a spell with the chosen name for the first time this turn, draw two cards.\n\
             IV — Look at the top card of each player's library.",
        )
        .expect("Medomai's Prophecy should parse for its gameplay regression");
    let matching_spell = CardBuilder::new(CardId::from_raw(73_501), "Prophecy Match")
        .card_types(vec![CardType::Instant])
        .build();
    let nonmatching_spell = CardBuilder::new(CardId::from_raw(73_502), "Prophecy Decoy")
        .card_types(vec![CardType::Instant])
        .build();

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    game.turn.active_player = alice;
    put_test_cards_in_zone(&mut game, alice, Zone::Library, 4);

    let saga = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    // Chapter II persists this value on the Saga. Set that prior chapter's
    // result directly so this regression can isolate chapter III's timing.
    game.set_chosen_named_option(saga, matching_spell.name.to_string());
    game.object_mut(saga)
        .expect("Medomai's Prophecy should exist")
        .add_counters(CounterType::Lore, 2);

    let mut chapter_queue = TriggerQueue::new();
    add_saga_lore_counters(&mut game, &mut chapter_queue);
    assert_eq!(chapter_queue.entries.len(), 1);
    assert_eq!(
        chapter_queue.entries[0].ability.trigger.saga_chapters(),
        Some(&[3][..]),
        "adding the third lore counter should queue chapter III"
    );
    put_triggers_on_stack(&mut game, &mut chapter_queue)
        .expect("Medomai's Prophecy chapter III should go on the stack");
    resolve_stack_entry(&mut game).expect("Medomai's Prophecy chapter III should resolve");

    assert_eq!(game.effect_store.delayed_triggers.len(), 1);
    assert!(
        game.effect_store.delayed_triggers[0].one_shot,
        "chapter III must schedule a one-shot registration"
    );

    let decoy = game.create_object_from_card(&nonmatching_spell, alice, Zone::Stack);
    assert!(
        crate::triggers::check_delayed_triggers(&mut game, &cast_event(decoy, alice)).is_empty(),
        "a nonmatching cast must not trigger chapter III"
    );
    assert_eq!(
        game.effect_store.delayed_triggers.len(),
        1,
        "a nonmatching cast must not consume the first-matching-cast registration"
    );

    let hand_before = game.player(alice).expect("Alice should exist").hand.len();
    let first_match = game.create_object_from_card(&matching_spell, alice, Zone::Stack);
    let mut first_match_queue = TriggerQueue::new();
    for trigger in
        crate::triggers::check_delayed_triggers(&mut game, &cast_event(first_match, alice))
    {
        first_match_queue.add(trigger);
    }
    assert_eq!(
        first_match_queue.entries.len(),
        1,
        "the first spell with the chosen name should trigger chapter III"
    );
    assert!(
        game.effect_store.delayed_triggers.is_empty(),
        "the first matching cast should consume the one-shot registration"
    );
    put_triggers_on_stack(&mut game, &mut first_match_queue)
        .expect("the first matching cast trigger should go on the stack");
    resolve_stack_entry(&mut game).expect("the first matching cast trigger should resolve");
    assert_eq!(
        game.player(alice).expect("Alice should exist").hand.len(),
        hand_before + 2,
        "the first matching cast should draw exactly two cards"
    );

    let second_match = game.create_object_from_card(&matching_spell, alice, Zone::Stack);
    assert!(
        crate::triggers::check_delayed_triggers(&mut game, &cast_event(second_match, alice))
            .is_empty(),
        "a second matching cast in the same turn must not trigger again"
    );
    assert_eq!(
        game.player(alice).expect("Alice should exist").hand.len(),
        hand_before + 2,
        "the second matching cast must not draw more cards"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn porcuparrot_damage_uses_source_mutation_count_not_battlefield_creatures() {
    let definition = CardDefinitionBuilder::new(CardId::from_raw(73_510), "Porcuparrot")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::Red],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Bird, Subtype::Beast])
        .power_toughness(PowerToughness::fixed(3, 4))
        .parse_text(
            "Mutate {2}{R}\n\
             {T}: This creature deals X damage to any target, where X is the number of times this creature has mutated.",
        )
        .expect("Porcuparrot should parse for its gameplay regression");
    let activated = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated.clone()),
            _ => None,
        })
        .expect("Porcuparrot should have its tap ability");

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    game.mark_mutated(source);
    game.mark_mutated(source);

    let decoy = CardBuilder::new(CardId::from_raw(73_511), "Mutation Count Decoy")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    for controller in [alice, alice, bob, bob, bob] {
        game.create_object_from_card(&decoy, controller, Zone::Battlefield);
    }
    assert_eq!(game.mutation_count(source), 2);
    assert_eq!(
        game.battlefield
            .iter()
            .filter(|id| game
                .object(**id)
                .is_some_and(|object| { object.card_types.contains(&CardType::Creature) }))
            .count(),
        6,
        "the battlefield count must visibly differ from the mutation count"
    );

    let life_before = game.player(bob).expect("Bob should exist").life;
    let mut dm = SelectFirstDecisionMaker;
    let mut ctx = ExecutionContext::new(source, alice, &mut dm)
        .with_targets(vec![ResolvedTarget::Player(bob)]);
    execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        source,
        &activated.effects,
        None,
        &[],
    )
    .expect("Porcuparrot's activated ability should resolve");

    assert_eq!(
        game.player(bob).expect("Bob should exist").life,
        life_before - 2,
        "Porcuparrot should deal damage equal to its two mutations, not six creatures"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn tardis_requires_a_time_lord_and_grants_cascade_then_planeswalks() {
    fn tardis_definition() -> crate::cards::CardDefinition {
        CardDefinitionBuilder::new(CardId::from_raw(73_520), "TARDIS")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]]))
            .card_types(vec![CardType::Artifact])
            .subtypes(vec![Subtype::Vehicle])
            .power_toughness(PowerToughness::fixed(2, 4))
            .parse_text(
                "Flying\n\
                 Whenever this Vehicle attacks, if you control a Time Lord, the next spell you cast this turn has cascade and you may planeswalk.\n\
                 Crew 2",
            )
            .expect("TARDIS should parse for its gameplay regression")
    }

    fn attack_event(tardis: ObjectId, defender: PlayerId) -> TriggerEvent {
        TriggerEvent::new_with_provenance(
            crate::events::combat::CreatureAttackedEvent::new(
                tardis,
                crate::triggers::AttackEventTarget::Player(defender),
            ),
            crate::provenance::ProvNodeId::default(),
        )
    }

    fn has_cascade(game: &GameState, spell: ObjectId) -> bool {
        game.object(spell).is_some_and(|object| {
            object.abilities.iter().any(|ability| {
                matches!(
                    &ability.kind,
                    AbilityKind::Static(static_ability)
                        if static_ability.id()
                            == crate::static_abilities::StaticAbilityId::Cascade
                )
            })
        })
    }

    struct AcceptMayDecisionMaker;
    impl DecisionMaker for AcceptMayDecisionMaker {
        fn decide_boolean(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::BooleanContext,
        ) -> bool {
            true
        }
    }

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.turn.active_player = alice;
    let tardis = game.create_object_from_definition(&tardis_definition(), alice, Zone::Battlefield);
    let event = attack_event(tardis, bob);

    let ordinary_doctor = CardBuilder::new(CardId::from_raw(73_521), "Arcade Gannon")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Human, Subtype::Doctor])
        .power_toughness(PowerToughness::fixed(2, 3))
        .build();
    let doctor = game.create_object_from_card(&ordinary_doctor, alice, Zone::Battlefield);
    assert!(
        crate::triggers::check_triggers(&game, &event)
            .into_iter()
            .all(|trigger| trigger.source != tardis),
        "a Doctor that is not a Time Lord must not satisfy TARDIS's condition"
    );
    game.move_object_by_effect(doctor, Zone::Graveyard)
        .expect("the non-Time-Lord Doctor should leave the battlefield");

    // Susan is a Time Lord without the unrelated Doctor subtype.
    let susan_foreman = CardBuilder::new(CardId::from_raw(73_522), "Susan Foreman")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::TimeLord])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let susan = game.create_object_from_card(&susan_foreman, alice, Zone::Battlefield);
    let triggers = crate::triggers::check_triggers(&game, &event)
        .into_iter()
        .filter(|trigger| trigger.source == tardis)
        .collect::<Vec<_>>();
    assert_eq!(
        triggers.len(),
        1,
        "a non-Doctor Time Lord must satisfy TARDIS's condition"
    );

    let current_plane =
        CardDefinitionBuilder::new(CardId::from_raw(73_523), "TARDIS Starting Plane").build();
    let next_plane =
        CardDefinitionBuilder::new(CardId::from_raw(73_524), "TARDIS Destination").build();
    let current_plane_id = game.create_object_from_definition(&current_plane, alice, Zone::Command);
    let next_plane_id = game.create_object_from_definition(&next_plane, alice, Zone::Command);
    game.planechase = Some(crate::game_state::PlanechaseState {
        decks: std::collections::HashMap::from([(alice, vec![next_plane_id, current_plane_id])]),
        communal_deck: None,
        deck_owners: std::collections::HashMap::from([
            (current_plane_id, alice),
            (next_plane_id, alice),
        ]),
        card_kinds: std::collections::HashMap::from([
            (current_plane_id, crate::game_state::PlanarCardKind::Plane),
            (next_plane_id, crate::game_state::PlanarCardKind::Plane),
        ]),
        face_up: Vec::new(),
        planar_controller: alice,
        planar_controllers: std::collections::HashSet::from([alice]),
        face_up_controllers: std::collections::HashMap::new(),
        voluntary_rolls_this_turn: std::collections::HashMap::new(),
        planeswalk_count: 0,
    });
    game.reveal_starting_plane()
        .expect("the TARDIS fixture should reveal its starting plane");

    let mut trigger_queue = TriggerQueue::new();
    for trigger in triggers {
        trigger_queue.add(trigger);
    }
    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("TARDIS's attack trigger should go on the stack");

    let mut lost_support_game = game.clone();
    lost_support_game
        .move_object_by_effect(susan, Zone::Graveyard)
        .expect("Susan should leave before the intervening-if recheck");
    let mut accept_may = AcceptMayDecisionMaker;
    resolve_stack_entry_with(&mut lost_support_game, &mut accept_may)
        .expect("a failed TARDIS intervening-if recheck should resolve as a no-op");
    assert!(
        lost_support_game
            .effect_store
            .temporary_spell_ability_grants
            .is_empty(),
        "TARDIS must not grant cascade after its Time Lord leaves before resolution"
    );
    assert_eq!(lost_support_game.planeswalk_count(), Some(0));

    resolve_stack_entry_with(&mut game, &mut accept_may)
        .expect("TARDIS's attack trigger should resolve");
    assert_eq!(game.planeswalk_count(), Some(1));
    assert_eq!(
        game.effect_store.temporary_spell_ability_grants.len(),
        1,
        "TARDIS should register one next-spell cascade grant"
    );

    let spell_card = CardBuilder::new(CardId::from_raw(73_525), "TARDIS Spell")
        .card_types(vec![CardType::Instant])
        .build();
    let bobs_spell = game.create_object_from_card(&spell_card, bob, Zone::Stack);
    game.apply_temporary_spell_ability_grants_for_cast_proposal(bobs_spell, bob);
    assert!(!has_cascade(&game, bobs_spell));
    assert_eq!(
        game.effect_store.temporary_spell_ability_grants[0].remaining_uses, 1,
        "another player's spell must not consume Alice's grant"
    );

    let alices_first_spell = game.create_object_from_card(&spell_card, alice, Zone::Stack);
    game.apply_temporary_spell_ability_grants_for_cast_proposal(alices_first_spell, alice);
    assert!(has_cascade(&game, alices_first_spell));
    assert_eq!(
        game.effect_store.temporary_spell_ability_grants[0].remaining_uses, 0,
        "Alice's next spell should consume TARDIS's one-shot grant"
    );

    let alices_second_spell = game.create_object_from_card(&spell_card, alice, Zone::Stack);
    game.apply_temporary_spell_ability_grants_for_cast_proposal(alices_second_spell, alice);
    assert!(
        !has_cascade(&game, alices_second_spell),
        "TARDIS must not grant cascade to a second spell"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn susan_foreman_reorders_the_planar_deck_before_planeswalking_once() {
    struct ChooseNamedPlanarCard {
        name: &'static str,
        viewed: Vec<ObjectId>,
    }

    impl DecisionMaker for ChooseNamedPlanarCard {
        fn view_cards(
            &mut self,
            _game: &GameState,
            _viewer: PlayerId,
            cards: &[ObjectId],
            _ctx: &crate::decisions::context::ViewCardsContext,
        ) {
            self.viewed = cards.to_vec();
        }

        fn decide_objects(
            &mut self,
            game: &GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            ctx.candidates
                .iter()
                .filter_map(|candidate| {
                    game.object(candidate.id)
                        .filter(|object| object.name == self.name)
                        .map(|_| candidate.id)
                })
                .collect()
        }
    }

    let susan_definition = CardDefinitionBuilder::new(CardId::from_raw(73_530), "Susan Foreman")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::Green],
        ]))
        .supertypes(vec![Supertype::Legendary])
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::TimeLord])
        .power_toughness(PowerToughness::fixed(1, 1))
        .parse_text(
            "If you would planeswalk, instead look at the top two cards of your planar deck, put one on the bottom of your planar deck and the other on top, then planeswalk.\n\
             {T}: Add {G}.\n\
             Doctor's companion (You can have two commanders if the other is the Doctor.)",
        )
        .expect("Susan Foreman's exact rules text should parse");
    let susan_debug = format!("{susan_definition:#?}");
    assert!(
        susan_debug.contains("KeywordActionReplacement"),
        "{susan_debug}"
    );
    assert!(
        susan_debug.contains("ReorderTopPlanarDeckEffect"),
        "{susan_debug}"
    );

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let current = CardDefinitionBuilder::new(CardId::from_raw(73_531), "Susan Starting Plane")
        .card_types(vec![CardType::Plane])
        .build();
    let top_choice = CardDefinitionBuilder::new(CardId::from_raw(73_532), "Susan Top Choice")
        .card_types(vec![CardType::Plane])
        .build();
    let second_choice = CardDefinitionBuilder::new(CardId::from_raw(73_533), "Susan Second Choice")
        .card_types(vec![CardType::Plane])
        .build();
    let current_id = game.create_object_from_definition(&current, alice, Zone::Command);
    let top_choice_id = game.create_object_from_definition(&top_choice, alice, Zone::Command);
    let second_choice_id = game.create_object_from_definition(&second_choice, alice, Zone::Command);
    let current_stable = game.object(current_id).expect("current plane").stable_id;
    game.planechase = Some(crate::game_state::PlanechaseState {
        decks: std::collections::HashMap::from([(
            alice,
            vec![second_choice_id, top_choice_id, current_id],
        )]),
        communal_deck: None,
        deck_owners: std::collections::HashMap::from([
            (current_id, alice),
            (top_choice_id, alice),
            (second_choice_id, alice),
        ]),
        card_kinds: std::collections::HashMap::from([
            (current_id, crate::game_state::PlanarCardKind::Plane),
            (top_choice_id, crate::game_state::PlanarCardKind::Plane),
            (second_choice_id, crate::game_state::PlanarCardKind::Plane),
        ]),
        face_up: Vec::new(),
        planar_controller: alice,
        planar_controllers: std::collections::HashSet::from([alice]),
        face_up_controllers: std::collections::HashMap::new(),
        voluntary_rolls_this_turn: std::collections::HashMap::new(),
        planeswalk_count: 0,
    });
    assert_eq!(
        game.reveal_starting_plane().expect("starting plane"),
        current_id
    );
    game.create_object_from_definition(&susan_definition, alice, Zone::Battlefield);

    let mut dm = ChooseNamedPlanarCard {
        name: "Susan Top Choice",
        viewed: Vec::new(),
    };
    let mut ctx = ExecutionContext::new(current_id, alice, &mut dm);
    let outcome = crate::effects::execute_effect(
        &mut game,
        &Effect::emit_keyword_action(crate::events::KeywordActionKind::Planeswalk, 1),
        &mut ctx,
    )
    .expect("Susan's replacement planeswalk should resolve");

    assert_eq!(outcome.as_count(), Some(1));
    assert_eq!(dm.viewed, vec![top_choice_id, second_choice_id]);
    assert_eq!(
        game.planeswalk_count(),
        Some(1),
        "replacement must not recurse"
    );
    assert_eq!(
        game.face_up_planar_objects(),
        &[second_choice_id],
        "the unchosen card should be the plane turned face up"
    );
    let remaining = game.planar_deck(alice).expect("remaining planar deck");
    assert_eq!(remaining.len(), 2);
    assert_eq!(remaining[1], top_choice_id);
    assert_eq!(
        game.object(remaining[0])
            .expect("recycled old plane")
            .stable_id,
        current_stable,
        "the old face-up plane moves to the bottom after Susan's chosen card"
    );
    let planeswalk_events = game
        .take_pending_trigger_events()
        .into_iter()
        .filter_map(|event| {
            event
                .downcast::<crate::events::KeywordActionEvent>()
                .cloned()
        })
        .filter(|event| event.action == crate::events::KeywordActionKind::Planeswalk)
        .count();
    assert_eq!(
        planeswalk_events, 1,
        "only the inner planeswalk should happen"
    );
}
