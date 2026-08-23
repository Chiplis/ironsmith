#![allow(unused_imports)]
use super::shard_00::*;
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
use super::*;

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn sophina_spearsage_deserter_does_not_trigger_when_only_other_creatures_attack() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let sophina = sophina_spearsage_deserter_definition();
    let sophina_id = game.create_object_from_definition(&sophina, alice, Zone::Battlefield);
    let other_attacker = create_creature(&mut game, "Other Attacker", alice, 2, 2);
    game.remove_summoning_sickness(sophina_id);
    game.remove_summoning_sickness(other_attacker);

    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(crate::game_state::Step::DeclareAttackers);

    let mut combat = CombatState::default();
    let mut trigger_queue = TriggerQueue::new();
    let declarations = vec![AttackerDeclaration {
        creature: other_attacker,
        target: AttackTarget::Player(bob),
    }];
    apply_attacker_declarations(&mut game, &mut combat, &mut trigger_queue, &declarations)
        .expect("other creature should be able to attack without Sophina");
    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("putting non-Sophina attack triggers on stack should succeed");

    assert!(
        game.stack.is_empty(),
        "Sophina should not trigger when only another creature attacks"
    );
    assert_eq!(
        clue_tokens_controlled_by(&game, alice).len(),
        0,
        "Sophina should not investigate without its own attack trigger"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn put_torch_the_witness_on_stack(
    game: &mut GameState,
    controller: PlayerId,
    target: ObjectId,
    x_value: u32,
) {
    let def = torch_the_witness_definition();
    let spell_id = game.create_object_from_definition(&def, controller, Zone::Stack);
    game.object_mut(spell_id).expect("Torch on stack").x_value = Some(x_value);
    game.push_to_stack(
        StackEntry::new(spell_id, controller)
            .with_x(x_value)
            .with_targets(vec![Target::Object(target)]),
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn torch_the_witness_targets_only_battlefield_creatures() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let spell = torch_the_witness_definition();
    let effects = spell
        .spell_effect
        .as_ref()
        .expect("Torch should have effects");

    let creature = create_creature(&mut game, "Witness Target", bob, 2, 2);
    let artifact = game.create_object_from_card(
        &CardBuilder::new(CardId::from_raw(74_261), "Noncreature Evidence")
            .card_types(vec![CardType::Artifact])
            .build(),
        bob,
        Zone::Battlefield,
    );

    let requirements = extract_target_requirements(&game, effects, alice, None);
    assert_eq!(
        requirements.len(),
        1,
        "Torch should have one target requirement"
    );
    let legal_targets = &requirements[0].legal_targets;
    assert!(
        legal_targets.contains(&Target::Object(creature)),
        "battlefield creatures should be legal Torch targets, got {legal_targets:?}"
    );
    assert!(
        !legal_targets.contains(&Target::Object(artifact)),
        "noncreature artifacts should not be legal Torch targets, got {legal_targets:?}"
    );
    assert!(
        !legal_targets.contains(&Target::Player(bob)),
        "players should not be legal Torch targets, got {legal_targets:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn torch_the_witness_investigates_when_twice_x_deals_excess_damage() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let target = create_creature(&mut game, "Small Witness", bob, 2, 3);

    put_torch_the_witness_on_stack(&mut game, alice, target, 2);
    resolve_stack_entry(&mut game).expect("Torch the Witness should resolve");

    assert_eq!(
        clue_tokens_controlled_by(&game, alice).len(),
        1,
        "twice X should deal 4 damage to a 3-toughness creature and investigate for excess damage"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn torch_the_witness_does_not_investigate_without_excess_damage() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let target = create_creature(&mut game, "Large Witness", bob, 2, 5);

    put_torch_the_witness_on_stack(&mut game, alice, target, 2);
    resolve_stack_entry(&mut game).expect("Torch the Witness should resolve");

    assert_eq!(
        game.damage_on(target),
        4,
        "twice X should deal 4 damage when X is 2"
    );
    assert_eq!(
        clue_tokens_controlled_by(&game, alice).len(),
        0,
        "Torch should not investigate when the damage is not excess"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn put_test_cards_in_zone(
    game: &mut GameState,
    player: PlayerId,
    zone: Zone,
    count: u32,
) {
    for index in 0..count {
        let card = CardBuilder::new(
            CardId::from_raw(73_000 + index),
            format!("Test Card {index}"),
        )
        .card_types(vec![CardType::Sorcery])
        .build();
        game.create_object_from_card(&card, player, zone);
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn attack_with_toad(
    game: &mut GameState,
    toad_id: ObjectId,
    extra_attacker: Option<ObjectId>,
) -> TriggerQueue {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.turn.active_player = alice;
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(crate::game_state::Step::DeclareAttackers);

    let mut declarations = vec![AttackerDeclaration {
        creature: toad_id,
        target: AttackTarget::Player(bob),
    }];
    if let Some(creature) = extra_attacker {
        declarations.push(AttackerDeclaration {
            creature,
            target: AttackTarget::Player(bob),
        });
    }

    let mut combat = CombatState::default();
    let mut trigger_queue = TriggerQueue::new();
    apply_attacker_declarations(&mut *game, &mut combat, &mut trigger_queue, &declarations)
        .expect("Twenty-Toed Toad attack declaration should be legal");
    trigger_queue
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn ox_drover_cannot_be_blocked_by_oxen_but_can_be_blocked_by_other_creatures() {
    let can_block = |blocker_is_ox: bool| {
        let mut game = setup_game();
        let mut trigger_queue = TriggerQueue::new();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let drover = ox_drover_definition();
        let attacker = game.create_object_from_definition(&drover, alice, Zone::Battlefield);
        let blocker = if blocker_is_ox {
            let ox = CardBuilder::new(CardId::from_raw(73_951), "Ox Blocker")
                .card_types(vec![CardType::Creature])
                .subtypes(vec![Subtype::Ox])
                .power_toughness(PowerToughness::fixed(2, 4))
                .build();
            game.create_object_from_card(&ox, bob, Zone::Battlefield)
        } else {
            create_creature(&mut game, "Non-Ox Blocker", bob, 2, 2)
        };

        let mut combat = CombatState::default();
        combat.attackers.push(crate::combat_state::AttackerInfo {
            creature: attacker,
            target: AttackTarget::Player(bob),
        });
        game.update_cant_effects();

        apply_blocker_declarations(
            &mut game,
            &mut combat,
            &mut trigger_queue,
            &[BlockerDeclaration {
                blocker,
                blocking: attacker,
            }],
            bob,
        )
        .is_ok()
    };

    assert!(
        can_block(false),
        "Ox Drover should still be blockable by non-Ox creatures"
    );
    assert!(
        !can_block(true),
        "Ox Drover's Oxen restriction should reject Ox blockers"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn ox_drover_enter_trigger_targets_opponent_creates_ox_and_draws() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let drover = ox_drover_definition();
    put_test_cards_in_zone(&mut game, alice, Zone::Library, 1);
    let drover_in_hand = game.create_object_from_definition(&drover, alice, Zone::Hand);
    game.move_object_by_effect(drover_in_hand, Zone::Battlefield)
        .expect("Ox Drover should enter from hand");

    let mut trigger_queue = TriggerQueue::new();
    drain_pending_trigger_events(&mut game, &mut trigger_queue);
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "Ox Drover should trigger once when it enters"
    );

    let mut dm = SelectFirstDecisionMaker;
    put_triggers_on_stack_with_dm(&mut game, &mut trigger_queue, &mut dm)
        .expect("Ox Drover enter trigger should go on the stack");
    resolve_stack_entry(&mut game).expect("Ox Drover enter trigger should resolve");

    assert_eq!(
        ox_tokens_controlled_by(&game, bob).len(),
        1,
        "target opponent should create one 2/4 white Ox token"
    );
    assert_eq!(
        game.player(alice).expect("Alice exists").hand.len(),
        1,
        "Ox Drover's controller should draw one card"
    );
    assert_eq!(
        game.player(bob).expect("Bob exists").hand.len(),
        0,
        "the target opponent should not draw from Ox Drover's trigger"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn ox_drover_attack_trigger_creates_ox_draws_and_vigilance_keeps_it_untapped() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let drover = ox_drover_definition();
    put_test_cards_in_zone(&mut game, alice, Zone::Library, 1);
    let drover_id = game.create_object_from_definition(&drover, alice, Zone::Battlefield);
    game.remove_summoning_sickness(drover_id);
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
            creature: drover_id,
            target: AttackTarget::Player(bob),
        }],
    )
    .expect("Ox Drover should be able to attack");
    assert!(
        !game.is_tapped(drover_id),
        "vigilance should keep Ox Drover untapped as it attacks"
    );
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "Ox Drover should trigger once when it attacks"
    );

    let mut dm = SelectFirstDecisionMaker;
    put_triggers_on_stack_with_dm(&mut game, &mut trigger_queue, &mut dm)
        .expect("Ox Drover attack trigger should go on the stack");
    resolve_stack_entry(&mut game).expect("Ox Drover attack trigger should resolve");

    assert_eq!(
        ox_tokens_controlled_by(&game, bob).len(),
        1,
        "target opponent should create one 2/4 white Ox token from the attack trigger"
    );
    assert_eq!(
        game.player(alice).expect("Alice exists").hand.len(),
        1,
        "Ox Drover's controller should draw one card from the attack trigger"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn scuttling_sentinel_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(651_777), "Scuttling Sentinel")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::Green, ManaSymbol::Blue],
            vec![ManaSymbol::Green, ManaSymbol::Blue],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Crab, Subtype::Elf])
        .power_toughness(PowerToughness::fixed(3, 2))
        .parse_text(
            "Flash\nVigilance\nWhen this creature enters, put a +1/+1 counter on another target creature you control. Until end of turn, that creature becomes a blue Crab in addition to its other types and gains hexproof.",
        )
        .expect("Scuttling Sentinel should parse for runtime tests")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn blitz_leech_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(479_594), "Blitz Leech")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(5)],
            vec![ManaSymbol::Black],
        ]))
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(5, 2))
        .parse_text(
            "Flash\nWhen this creature enters, target creature an opponent controls gets -2/-2 until end of turn. Remove all counters from that creature.",
        )
        .expect("Blitz Leech should parse for runtime tests")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) struct ChooseBlitzLeechTarget {
    pub(super) chosen: ObjectId,
    pub(super) seen_legal_targets: Vec<Target>,
}

#[cfg(ironsmith_runtime_parser_tests)]
impl DecisionMaker for ChooseBlitzLeechTarget {
    fn decide_targets(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::TargetsContext,
    ) -> Vec<Target> {
        self.seen_legal_targets = ctx
            .requirements
            .first()
            .map(|requirement| requirement.legal_targets.clone())
            .unwrap_or_default();
        assert!(
            self.seen_legal_targets
                .contains(&Target::Object(self.chosen)),
            "chosen opponent creature should be a legal Blitz Leech target"
        );
        vec![Target::Object(self.chosen)]
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn blitz_leech_enter_trigger_removes_all_counters_from_opponent_creature_only() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let opponent_creature = create_creature(&mut game, "Bob's Countered Creature", bob, 4, 4);
    let own_creature = create_creature(&mut game, "Alice's Countered Creature", alice, 4, 4);
    game.add_counters(
        opponent_creature,
        crate::object::CounterType::PlusOnePlusOne,
        2,
    )
    .expect("opponent creature should receive +1/+1 counters");
    game.add_counters(opponent_creature, crate::object::CounterType::Charge, 3)
        .expect("opponent creature should receive charge counters");
    game.add_counters(own_creature, crate::object::CounterType::PlusOnePlusOne, 1)
        .expect("own creature should receive a +1/+1 counter");

    let blitz = blitz_leech_definition();
    let blitz_id = game.create_object_from_definition(&blitz, alice, Zone::Hand);
    game.move_object_by_effect(blitz_id, Zone::Battlefield)
        .expect("Blitz Leech should enter the battlefield");

    let mut trigger_queue = TriggerQueue::new();
    drain_pending_trigger_events(&mut game, &mut trigger_queue);
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "Blitz Leech should queue exactly one enters trigger"
    );

    let mut dm = ChooseBlitzLeechTarget {
        chosen: opponent_creature,
        seen_legal_targets: Vec::new(),
    };
    put_triggers_on_stack_with_dm(&mut game, &mut trigger_queue, &mut dm)
        .expect("Blitz Leech trigger should go on the stack with its target");
    assert!(
        !dm.seen_legal_targets
            .contains(&Target::Object(own_creature)),
        "Blitz Leech should not be able to target a creature its controller controls"
    );

    resolve_stack_entry(&mut game).expect("Blitz Leech trigger should resolve");

    assert_eq!(
        game.counter_count(
            opponent_creature,
            crate::object::CounterType::PlusOnePlusOne
        ),
        0,
        "Blitz Leech should remove all +1/+1 counters from the targeted creature"
    );
    assert_eq!(
        game.counter_count(opponent_creature, crate::object::CounterType::Charge),
        0,
        "Blitz Leech should remove all non-P/T counters from the targeted creature too"
    );
    assert_eq!(
        game.counter_count(own_creature, crate::object::CounterType::PlusOnePlusOne),
        1,
        "Blitz Leech should not remove counters from untargeted friendly creatures"
    );
    assert_eq!(game.calculated_power(opponent_creature), Some(2));
    assert_eq!(game.calculated_toughness(opponent_creature), Some(2));

    execute_cleanup_step(&mut game);
    game.refresh_continuous_state();

    assert_eq!(
        game.counter_count(
            opponent_creature,
            crate::object::CounterType::PlusOnePlusOne
        ),
        0,
        "removed counters should stay removed after the turn ends"
    );
    assert_eq!(game.calculated_power(opponent_creature), Some(4));
    assert_eq!(game.calculated_toughness(opponent_creature), Some(4));
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn scuttling_sentinel_enter_trigger_buffs_only_another_creature_you_control_until_eot() {
    struct ChooseSpecificCreatureDecisionMaker {
        chosen: ObjectId,
        seen_legal_targets: Vec<Target>,
    }

    impl DecisionMaker for ChooseSpecificCreatureDecisionMaker {
        fn decide_targets(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::TargetsContext,
        ) -> Vec<Target> {
            self.seen_legal_targets = ctx
                .requirements
                .first()
                .map(|requirement| requirement.legal_targets.clone())
                .unwrap_or_default();
            assert!(
                self.seen_legal_targets
                    .contains(&Target::Object(self.chosen)),
                "the chosen creature should be a legal Scuttling Sentinel trigger target"
            );
            vec![Target::Object(self.chosen)]
        }
    }

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let target_id = create_typed_creature(&mut game, "Alice's Elf", alice, vec![Subtype::Elf]);
    let opponent_creature_id =
        create_typed_creature(&mut game, "Bob's Elf", bob, vec![Subtype::Elf]);
    let sentinel = scuttling_sentinel_definition();
    let sentinel_id = game.create_object_from_definition(&sentinel, alice, Zone::Hand);
    game.move_object_by_effect(sentinel_id, Zone::Battlefield)
        .expect("Scuttling Sentinel should enter the battlefield");

    let mut trigger_queue = TriggerQueue::new();
    drain_pending_trigger_events(&mut game, &mut trigger_queue);
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "Scuttling Sentinel should trigger once when it enters"
    );

    let mut dm = ChooseSpecificCreatureDecisionMaker {
        chosen: target_id,
        seen_legal_targets: Vec::new(),
    };
    put_triggers_on_stack_with_dm(&mut game, &mut trigger_queue, &mut dm)
        .expect("Scuttling Sentinel trigger should go on the stack with a target");
    assert!(
        !dm.seen_legal_targets.contains(&Target::Object(sentinel_id)),
        "Scuttling Sentinel should not be able to target itself"
    );
    assert!(
        !dm.seen_legal_targets
            .contains(&Target::Object(opponent_creature_id)),
        "Scuttling Sentinel should not be able to target an opponent's creature"
    );

    resolve_stack_entry(&mut game).expect("Scuttling Sentinel trigger should resolve");

    assert_eq!(
        game.counter_count(target_id, crate::object::CounterType::PlusOnePlusOne),
        1,
        "the chosen creature should get a +1/+1 counter"
    );
    assert_eq!(
        game.counter_count(sentinel_id, crate::object::CounterType::PlusOnePlusOne),
        0,
        "Scuttling Sentinel should not put its counter on itself"
    );
    assert_eq!(
        game.counter_count(
            opponent_creature_id,
            crate::object::CounterType::PlusOnePlusOne
        ),
        0,
        "opposing creatures should not receive Scuttling Sentinel's counter"
    );
    assert_eq!(
        game.current_colors(target_id),
        Some(crate::color::ColorSet::BLUE),
        "the target should become blue until end of turn"
    );
    assert!(
        game.current_has_subtype(target_id, Subtype::Elf)
            && game.current_has_subtype(target_id, Subtype::Crab),
        "the target should keep its existing type and gain Crab"
    );
    assert!(
        game.object_has_static_ability_id(
            target_id,
            crate::static_abilities::StaticAbilityId::Hexproof
        ),
        "the target should gain hexproof until end of turn"
    );

    execute_cleanup_step(&mut game);
    game.refresh_continuous_state();

    assert_eq!(
        game.counter_count(target_id, crate::object::CounterType::PlusOnePlusOne),
        1,
        "the +1/+1 counter should remain after the turn ends"
    );
    assert!(
        !game.current_has_subtype(target_id, Subtype::Crab)
            && game.current_has_subtype(target_id, Subtype::Elf),
        "the temporary Crab subtype should expire while the original type remains"
    );
    assert_ne!(
        game.current_colors(target_id),
        Some(crate::color::ColorSet::BLUE),
        "the temporary blue color-setting effect should expire at end of turn"
    );
    assert!(
        !game.object_has_static_ability_id(
            target_id,
            crate::static_abilities::StaticAbilityId::Hexproof
        ),
        "temporary hexproof should expire at end of turn"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn twenty_toed_toad_static_ability_sets_maximum_hand_size_to_twenty() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let toad = twenty_toed_toad_definition();
    game.create_object_from_definition(&toad, alice, Zone::Battlefield);

    game.update_cant_effects();

    assert_eq!(game.player(alice).unwrap().max_hand_size, 20);
    assert_eq!(game.player(bob).unwrap().max_hand_size, 7);
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn trusted_advisor_static_ability_increases_only_controller_maximum_hand_size() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let advisor = trusted_advisor_definition();
    game.create_object_from_definition(&advisor, alice, Zone::Battlefield);

    game.update_cant_effects();

    assert_eq!(game.player(alice).unwrap().max_hand_size, 9);
    assert_eq!(game.player(bob).unwrap().max_hand_size, 7);
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn trusted_advisor_in_hand_does_not_increase_maximum_hand_size() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let advisor = trusted_advisor_definition();
    game.create_object_from_definition(&advisor, alice, Zone::Hand);

    game.update_cant_effects();

    assert_eq!(game.player(alice).unwrap().max_hand_size, 7);
    assert_eq!(game.player(bob).unwrap().max_hand_size, 7);
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn trusted_advisor_upkeep_returns_blue_creature_you_control_to_owners_hand() {
    struct ChooseTrustedAdvisorCreatureDecisionMaker {
        chosen: ObjectId,
        seen_legal_objects: Vec<ObjectId>,
    }

    impl DecisionMaker for ChooseTrustedAdvisorCreatureDecisionMaker {
        fn decide_targets(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::TargetsContext,
        ) -> Vec<Target> {
            panic!("Trusted Advisor should choose, not target, got {ctx:?}");
        }

        fn decide_objects(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            self.seen_legal_objects = ctx
                .candidates
                .iter()
                .filter(|candidate| candidate.legal)
                .map(|candidate| candidate.id)
                .collect();
            assert!(
                self.seen_legal_objects.contains(&self.chosen),
                "chosen blue creature you control should be legal, got {:?}",
                ctx.candidates
            );
            vec![self.chosen]
        }
    }

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let advisor = trusted_advisor_definition();
    game.create_object_from_definition(&advisor, alice, Zone::Battlefield);
    let borrowed_blue = create_colored_creature(
        &mut game,
        "Borrowed Drake",
        bob,
        Some(crate::color::ColorSet::BLUE),
    );
    game.set_current_controller(borrowed_blue, alice);
    let borrowed_stable_id = game
        .object(borrowed_blue)
        .expect("borrowed creature should exist")
        .stable_id;
    let controlled_green = create_colored_creature(
        &mut game,
        "Alice Bear",
        alice,
        Some(crate::color::ColorSet::GREEN),
    );
    let opponents_blue = create_colored_creature(
        &mut game,
        "Bob Drake",
        bob,
        Some(crate::color::ColorSet::BLUE),
    );

    game.turn.phase = Phase::Beginning;
    game.turn.step = Some(crate::game_state::Step::Upkeep);
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let mut trigger_queue = TriggerQueue::new();
    generate_and_queue_step_triggers(&mut game, &mut trigger_queue);
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "Trusted Advisor should trigger at the beginning of its controller's upkeep"
    );

    let mut dm = ChooseTrustedAdvisorCreatureDecisionMaker {
        chosen: borrowed_blue,
        seen_legal_objects: Vec::new(),
    };
    put_triggers_on_stack_with_dm(&mut game, &mut trigger_queue, &mut dm)
        .expect("Trusted Advisor upkeep trigger should go on the stack without targets");
    resolve_stack_entry_with(&mut game, &mut dm)
        .expect("Trusted Advisor upkeep trigger should resolve");

    assert!(dm.seen_legal_objects.contains(&borrowed_blue));
    assert!(!dm.seen_legal_objects.contains(&controlled_green));
    assert!(!dm.seen_legal_objects.contains(&opponents_blue));
    let returned_borrowed = game
        .find_object_by_stable_id(borrowed_stable_id)
        .expect("returned creature should still be tracked by stable id");
    assert!(
        game.player(bob)
            .is_some_and(|player| player.hand.contains(&returned_borrowed)),
        "the chosen creature should return to its owner's hand"
    );
    assert!(!game.battlefield.contains(&borrowed_blue));
    assert!(game.battlefield.contains(&controlled_green));
    assert!(game.battlefield.contains(&opponents_blue));
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn twenty_toed_toad_two_creature_attack_puts_counter_and_draws() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let toad = twenty_toed_toad_definition();
    let toad_id = game.create_object_from_definition(&toad, alice, Zone::Battlefield);
    game.remove_summoning_sickness(toad_id);
    put_test_cards_in_zone(&mut game, alice, Zone::Library, 1);

    let helper = CardBuilder::new(CardId::from_raw(73_100), "Helper Attacker")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let helper_id = game.create_object_from_card(&helper, alice, Zone::Battlefield);
    game.remove_summoning_sickness(helper_id);

    let mut trigger_queue = attack_with_toad(&mut game, toad_id, Some(helper_id));
    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("Twenty-Toed Toad attack triggers should go on stack");
    assert_eq!(
        game.stack.len(),
        2,
        "toad attacking with another creature below the win threshold should still create both attack triggers"
    );

    while !game.stack.is_empty() {
        resolve_stack_entry(&mut game).expect("Twenty-Toed Toad trigger should resolve");
    }

    assert_eq!(
        game.counter_count(toad_id, crate::object::CounterType::PlusOnePlusOne),
        1,
        "two-creature attack trigger should put a +1/+1 counter on Twenty-Toed Toad"
    );
    assert_eq!(
        game.player(alice).unwrap().hand.len(),
        1,
        "two-creature attack trigger should draw one card"
    );
    assert!(
        !game.player(bob).unwrap().has_lost,
        "Twenty-Toed Toad should not win below both twenty-card and twenty-counter thresholds"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn twenty_toed_toad_win_trigger_checks_cards_in_hand_and_counters() {
    let mut below_threshold = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let toad = twenty_toed_toad_definition();
    let toad_id = below_threshold.create_object_from_definition(&toad, alice, Zone::Battlefield);
    below_threshold.remove_summoning_sickness(toad_id);
    put_test_cards_in_zone(&mut below_threshold, alice, Zone::Hand, 19);

    let mut trigger_queue = attack_with_toad(&mut below_threshold, toad_id, None);
    put_triggers_on_stack(&mut below_threshold, &mut trigger_queue)
        .expect("Twenty-Toed Toad attack trigger should go on stack");
    assert_eq!(
        below_threshold.stack.len(),
        1,
        "Twenty-Toed Toad's win trigger should still go on stack below the resolution threshold"
    );
    while !below_threshold.stack.is_empty() {
        resolve_stack_entry(&mut below_threshold).expect("below-threshold trigger should resolve");
    }
    assert!(
        !below_threshold.player(bob).unwrap().has_lost,
        "nineteen cards and fewer than twenty counters should not satisfy Twenty-Toed Toad"
    );

    let mut cards_threshold = setup_game();
    let toad = twenty_toed_toad_definition();
    let toad_id = cards_threshold.create_object_from_definition(&toad, alice, Zone::Battlefield);
    cards_threshold.remove_summoning_sickness(toad_id);
    put_test_cards_in_zone(&mut cards_threshold, alice, Zone::Hand, 20);

    let mut trigger_queue = attack_with_toad(&mut cards_threshold, toad_id, None);
    put_triggers_on_stack(&mut cards_threshold, &mut trigger_queue)
        .expect("Twenty-Toed Toad card-threshold trigger should go on stack");
    while !cards_threshold.stack.is_empty() {
        resolve_stack_entry(&mut cards_threshold).expect("card-threshold trigger should resolve");
    }
    assert!(
        cards_threshold.player(bob).unwrap().has_lost,
        "twenty cards in hand should satisfy Twenty-Toed Toad's win trigger"
    );

    let mut counters_threshold = setup_game();
    let toad = twenty_toed_toad_definition();
    let toad_id = counters_threshold.create_object_from_definition(&toad, alice, Zone::Battlefield);
    counters_threshold.remove_summoning_sickness(toad_id);
    counters_threshold
        .add_counters(toad_id, crate::object::CounterType::PlusOnePlusOne, 20)
        .expect("test should add counters to Twenty-Toed Toad");

    let mut trigger_queue = attack_with_toad(&mut counters_threshold, toad_id, None);
    put_triggers_on_stack(&mut counters_threshold, &mut trigger_queue)
        .expect("Twenty-Toed Toad counter-threshold trigger should go on stack");
    while !counters_threshold.stack.is_empty() {
        resolve_stack_entry(&mut counters_threshold)
            .expect("counter-threshold trigger should resolve");
    }
    assert!(
        counters_threshold.player(bob).unwrap().has_lost,
        "twenty counters should satisfy Twenty-Toed Toad's win trigger"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn open_the_way_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(72_900), "Open the Way")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::X],
            vec![ManaSymbol::Green],
            vec![ManaSymbol::Green],
        ]))
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "X can't be greater than the number of players in the game.\n\
             Reveal cards from the top of your library until you reveal X land cards. Put those land cards onto the battlefield tapped and the rest on the bottom of your library in a random order.",
        )
        .expect("Open the Way should parse")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn desmond_miles_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(72_910), "Desmond Miles")
        .supertypes(vec![Supertype::Legendary])
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Human, Subtype::Assassin])
        .power_toughness(PowerToughness::fixed(1, 3))
        .parse_text(
            "Menace\nDesmond Miles gets +1/+0 for each other Assassin you control and each Assassin card in your graveyard.\nWhenever Desmond Miles deals combat damage to a player, surveil X, where X is the amount of damage it dealt to that player.",
        )
        .expect("Desmond Miles should parse")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn tom_bombadil_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(72_920), "Tom Bombadil")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::White],
            vec![ManaSymbol::Blue],
            vec![ManaSymbol::Black],
            vec![ManaSymbol::Red],
            vec![ManaSymbol::Green],
        ]))
        .supertypes(vec![Supertype::Legendary])
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::God, Subtype::Bard])
        .power_toughness(PowerToughness::fixed(4, 4))
        .from_text_with_metadata(
            "As long as there are four or more lore counters among Sagas you control, Tom Bombadil has hexproof and indestructible.\n\
             Whenever the final chapter ability of a Saga you control resolves, reveal cards from the top of your library until you reveal a Saga card. Put that card onto the battlefield and the rest on the bottom of your library in a random order. This ability triggers only once each turn.",
        )
        .expect("Tom Bombadil should parse strictly")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn battle_for_bretagard_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(72_929), "Battle for Bretagard")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::Green],
            vec![ManaSymbol::White],
        ]))
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Saga])
        .from_text_with_metadata(
            "I — Create a 1/1 white Human Warrior creature token.\n\
             II — Create a 1/1 green Elf Warrior creature token.\n\
             III — Choose any number of artifact tokens and/or creature tokens you control with different names. For each of them, create a token that's a copy of it.",
        )
        .expect("Battle for Bretagard should parse strictly")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn interface_ace_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(72_925), "Interface Ace")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::White],
        ]))
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .subtypes(vec![Subtype::Robot, Subtype::Pilot])
        .power_toughness(PowerToughness::fixed(0, 4))
        .from_text_with_metadata(
            "This creature saddles Mounts and crews Vehicles using its toughness rather than its power.\n\
             Whenever this creature becomes tapped during your turn, untap it. This ability triggers only once each turn.",
        )
        .expect("Interface Ace should parse strictly")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn merfolk_cave_diver_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(72_930), "Merfolk Cave-Diver")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Blue],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Merfolk, Subtype::Scout])
        .power_toughness(PowerToughness::fixed(2, 4))
        .parse_text(
            "Whenever a creature you control explores, this creature gets +1/+0 until end of turn and can't be blocked this turn.",
        )
        .expect("Merfolk Cave-Diver should parse")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn rise_from_the_grave_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(72_940), "Rise from the Grave")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(4)],
            vec![ManaSymbol::Black],
        ]))
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Put target creature card from a graveyard onto the battlefield under your control. That creature is a black Zombie in addition to its other colors and types.",
        )
        .expect("Rise from the Grave should parse")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn necromantic_summons_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(72_945), "Necromantic Summons")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(4)],
            vec![ManaSymbol::Black],
        ]))
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Put target creature card from a graveyard onto the battlefield under your control.\n\
             Spell mastery — If there are two or more instant and/or sorcery cards in your graveyard, that creature enters with two additional +1/+1 counters on it.",
        )
        .expect("Necromantic Summons should parse")
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn keeper_of_the_flame_activation_requires_higher_life_opponent_and_damages_that_player()
{
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.player_mut(alice).expect("Alice should exist").life = 20;
    game.player_mut(bob).expect("Bob should exist").life = 20;

    let keeper_def = keeper_of_the_flame_definition();
    let keeper_id = game.create_object_from_definition(&keeper_def, alice, Zone::Battlefield);
    game.remove_summoning_sickness(keeper_id);
    game.player_mut(alice)
        .expect("Alice should exist")
        .mana_pool
        .add(ManaSymbol::Red, 1);

    assert!(
        !crate::decision::compute_legal_actions(&game, alice)
            .into_iter()
            .any(|action| matches!(
                action,
                crate::decision::LegalAction::ActivateAbility { source, .. }
                    if source == keeper_id
            )),
        "Keeper of the Flame should not be activatable without a higher-life opponent target"
    );

    game.player_mut(bob).expect("Bob should exist").life = 21;
    let ability_index = game
        .object(keeper_id)
        .expect("Keeper of the Flame should exist")
        .abilities
        .iter()
        .position(|ability| matches!(ability.kind, AbilityKind::Activated(_)))
        .expect("Keeper of the Flame should have an activated ability");
    let activate_action = crate::decision::compute_legal_actions(&game, alice)
        .into_iter()
        .find(|action| {
            matches!(
                action,
                crate::decision::LegalAction::ActivateAbility { source, ability_index: idx }
                    if *source == keeper_id && *idx == ability_index
            )
        })
        .expect("Keeper of the Flame activation should be legal once Bob has more life");

    let mut trigger_queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = SelectFirstDecisionMaker;
    let progress = apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(activate_action),
        &mut dm,
    )
    .expect("Keeper of the Flame activation should start");
    match progress {
        crate::decision::GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::Targets(_),
        ) => {}
        other => panic!("expected target selection for Keeper of the Flame, got {other:?}"),
    }

    apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::Targets(vec![Target::Player(bob)]),
        &mut dm,
    )
    .expect("choosing the higher-life opponent should complete activation");

    game.player_mut(bob).expect("Bob should exist").life = 18;

    resolve_stack_entry(&mut game).expect("Keeper of the Flame ability should resolve");
    assert_eq!(
        game.player(bob).expect("Bob should exist").life,
        16,
        "Keeper of the Flame should remember that the chosen player had more life during activation"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn keeper_of_the_flame_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(72_950), "Keeper of the Flame")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Red], vec![ManaSymbol::Red]]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Human, Subtype::Wizard])
        .power_toughness(PowerToughness::fixed(1, 2))
        .parse_text(
            "{R}, {T}: Choose target opponent who has more life than you do as you activate this ability. This creature deals 2 damage to that player.",
        )
        .expect("Keeper of the Flame should parse")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn goblin_kites_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(72_955), "Goblin Kites")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::Red],
        ]))
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "{R}: Target creature you control with toughness 2 or less gains flying until end of turn. Flip a coin at the beginning of the next end step. If you lose the flip, sacrifice that creature.",
        )
        .expect("Goblin Kites should parse")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn create_vanilla_creature(
    game: &mut GameState,
    name: &str,
    controller: PlayerId,
    power: i32,
    toughness: i32,
) -> ObjectId {
    let card = CardBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(power, toughness))
        .build();
    game.create_object_from_card(&card, controller, Zone::Battlefield)
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn barrensteppe_siege_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(1_149_701), "Barrensteppe Siege")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::White],
            vec![ManaSymbol::Black],
        ]))
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "As this enchantment enters, choose Abzan or Mardu.\n\
             • Abzan — At the beginning of your end step, put a +1/+1 counter on each creature you control.\n\
             • Mardu — At the beginning of your end step, if a creature died under your control this turn, each opponent sacrifices a creature of their choice.",
        )
        .expect("Barrensteppe Siege should parse for runtime tests")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) struct BarrensteppeChoiceDecisionMaker {
    pub(super) option_index: usize,
}

#[cfg(ironsmith_runtime_parser_tests)]
impl DecisionMaker for BarrensteppeChoiceDecisionMaker {
    fn decide_options(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        if ctx
            .options
            .iter()
            .any(|option| option.legal && option.index == self.option_index)
        {
            vec![self.option_index]
        } else {
            Vec::new()
        }
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn enter_barrensteppe_siege_with_choice(
    game: &mut GameState,
    controller: PlayerId,
    option_index: usize,
    expected_option: &str,
) -> ObjectId {
    let hand_id = game.create_object_from_definition(
        &barrensteppe_siege_definition(),
        controller,
        Zone::Hand,
    );
    let mut dm = BarrensteppeChoiceDecisionMaker { option_index };
    let siege = game
        .move_object_with_etb_processing_with_dm(hand_id, Zone::Battlefield, &mut dm)
        .expect("Barrensteppe Siege should enter the battlefield")
        .new_id;

    assert_eq!(
        game.chosen_named_option(siege),
        Some(expected_option),
        "Barrensteppe Siege should record its as-enters named option choice"
    );
    siege
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn put_barrensteppe_end_step_triggers_on_stack(game: &mut GameState) -> usize {
    let mut trigger_queue = TriggerQueue::new();
    let event = TriggerEvent::new_with_provenance(
        crate::events::phase::BeginningOfEndStepEvent::new(game.turn.active_player),
        crate::provenance::ProvNodeId::default(),
    );
    for trigger in crate::triggers::check_triggers(game, &event) {
        trigger_queue.add(trigger);
    }
    let count = trigger_queue.entries.len();
    if count > 0 {
        put_triggers_on_stack(game, &mut trigger_queue)
            .expect("Barrensteppe Siege trigger should go on stack");
    }
    count
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn barrensteppe_siege_abzan_end_step_puts_counters_on_each_creature_you_control() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.turn.active_player = alice;

    enter_barrensteppe_siege_with_choice(&mut game, alice, 0, "abzan");
    let first = create_vanilla_creature(&mut game, "Abzan Initiate", alice, 2, 2);
    let second = create_vanilla_creature(&mut game, "Abzan Acolyte", alice, 1, 1);
    let opponent = create_vanilla_creature(&mut game, "Opponent Creature", bob, 3, 3);

    assert_eq!(put_barrensteppe_end_step_triggers_on_stack(&mut game), 1);
    resolve_stack_entry(&mut game).expect("Barrensteppe Siege Abzan trigger should resolve");

    assert_eq!(
        game.counter_count(first, crate::object::CounterType::PlusOnePlusOne),
        1,
        "first controlled creature should get a +1/+1 counter"
    );
    assert_eq!(
        game.counter_count(second, crate::object::CounterType::PlusOnePlusOne),
        1,
        "second controlled creature should get a +1/+1 counter"
    );
    assert_eq!(
        game.counter_count(opponent, crate::object::CounterType::PlusOnePlusOne),
        0,
        "opponent's creature should not get a counter"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn barrensteppe_siege_mardu_does_not_trigger_for_opponent_creature_death() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.turn.active_player = alice;

    enter_barrensteppe_siege_with_choice(&mut game, alice, 1, "mardu");
    let bob_victim = create_vanilla_creature(&mut game, "Bob Victim", bob, 1, 1);
    create_vanilla_creature(&mut game, "Bob Survivor", bob, 2, 2);

    game.move_object_by_effect(bob_victim, Zone::Graveyard)
        .expect("opponent creature should die");
    let mut pending = TriggerQueue::new();
    drain_pending_trigger_events(&mut game, &mut pending);

    assert_eq!(
        put_barrensteppe_end_step_triggers_on_stack(&mut game),
        0,
        "Mardu branch should not trigger when only an opponent's creature died"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn barrensteppe_siege_mardu_sacrifices_each_opponents_chosen_creature_after_yours_died()
{
    let mut game = setup_three_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);
    game.turn.active_player = alice;

    enter_barrensteppe_siege_with_choice(&mut game, alice, 1, "mardu");
    let alice_victim = create_vanilla_creature(&mut game, "Alice Victim", alice, 1, 1);
    let bob_creature = create_vanilla_creature(&mut game, "Bob Creature", bob, 2, 2);
    let charlie_creature = create_vanilla_creature(&mut game, "Charlie Creature", charlie, 2, 2);

    game.move_object_by_effect(alice_victim, Zone::Graveyard)
        .expect("your creature should die");
    let mut pending = TriggerQueue::new();
    drain_pending_trigger_events(&mut game, &mut pending);

    assert_eq!(put_barrensteppe_end_step_triggers_on_stack(&mut game), 1);
    let mut dm = SelectFirstDecisionMaker;
    resolve_stack_entry_with(&mut game, &mut dm)
        .expect("Barrensteppe Siege Mardu trigger should resolve");

    assert!(
        !game.battlefield.contains(&bob_creature),
        "Mardu branch should remove Bob's chosen creature from the battlefield"
    );
    assert!(
        !game.battlefield.contains(&charlie_creature),
        "Mardu branch should remove Charlie's chosen creature from the battlefield"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn mighty_servant_of_leuk_o_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(274_545), "Mighty Servant of Leuk-o")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(3)]]))
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Vehicle])
        .power_toughness(PowerToughness::fixed(6, 6))
        .parse_text(
            "Trample\n\
             Ward—Discard a card.\n\
             Whenever this Vehicle becomes crewed for the first time each turn, if it was crewed by exactly two creatures, it gains \"Whenever this creature deals combat damage to a player, draw two cards\" until end of turn.\n\
             Crew 4",
        )
        .expect("Mighty Servant of Leuk-o should parse")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) struct ChooseCrewByNameDecisionMaker {
    pub(super) names: Vec<&'static str>,
}

#[cfg(ironsmith_runtime_parser_tests)]
impl DecisionMaker for ChooseCrewByNameDecisionMaker {
    fn decide_objects(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        if ctx.description.to_ascii_lowercase().contains("crew") {
            return ctx
                .candidates
                .iter()
                .filter(|candidate| candidate.legal)
                .filter_map(|candidate| {
                    game.object(candidate.id).and_then(|object| {
                        self.names
                            .contains(&object.name.as_str())
                            .then_some(candidate.id)
                    })
                })
                .collect();
        }
        AutoPassDecisionMaker.decide_objects(game, ctx)
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn activate_mighty_servant_crew(
    game: &mut GameState,
    vehicle_id: ObjectId,
    controller: PlayerId,
    crew_names: Vec<&'static str>,
) -> TriggerQueue {
    let mut trigger_queue = TriggerQueue::new();
    let mut dm = ChooseCrewByNameDecisionMaker { names: crew_names };
    let provenance = game.provenance_graph_mut().alloc_root(
        crate::provenance::ProvenanceNodeKind::EffectExecution {
            source: vehicle_id,
            controller,
        },
    );
    let mut ctx = crate::effects::ExecutionContext::new(vehicle_id, controller, &mut dm)
        .with_provenance(provenance);
    let crew = crate::effects::CrewCostEffect::new(4);
    let outcome = crate::effects::EffectExecutor::execute(&crew, game, &mut ctx)
        .expect("Mighty Servant of Leuk-o crew cost should be payable");
    for event in outcome.events {
        game.queue_trigger_event(provenance, event);
    }
    drain_pending_trigger_events(game, &mut trigger_queue);

    trigger_queue
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn vastwood_animist_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(72_956), "Vastwood Animist")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Green],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Elf, Subtype::Shaman, Subtype::Ally])
        .power_toughness(PowerToughness::fixed(1, 1))
        .parse_text(
            "{T}: Target land you control becomes an X/X Elemental creature until end of turn, where X is the number of Allies you control. It's still a land.",
        )
        .expect("Vastwood Animist should parse for runtime tests")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn create_test_land(game: &mut GameState, name: &str, controller: PlayerId) -> ObjectId {
    let card = CardBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Land])
        .build();
    game.create_object_from_card(&card, controller, Zone::Battlefield)
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn mighty_servant_two_creature_crew_grants_damage_draw_until_cleanup() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let mighty = mighty_servant_of_leuk_o_definition();
    let vehicle_id = game.create_object_from_definition(&mighty, alice, Zone::Battlefield);
    let crew_one = create_vanilla_creature(&mut game, "Crew One", alice, 2, 2);
    let crew_two = create_vanilla_creature(&mut game, "Crew Two", alice, 2, 2);
    for idx in 0..2 {
        let card = CardBuilder::new(
            CardId::from_raw(274_600 + idx),
            format!("Draw Fodder {idx}"),
        )
        .build();
        game.create_object_from_card(&card, alice, Zone::Library);
    }

    let mut trigger_queue =
        activate_mighty_servant_crew(&mut game, vehicle_id, alice, vec!["Crew One", "Crew Two"]);
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "Mighty Servant should trigger when first crewed by exactly two creatures"
    );
    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("Mighty Servant crew-count trigger should go on the stack");
    resolve_stack_entry(&mut game).expect("Mighty Servant crew-count trigger should resolve");

    game.untap(crew_one);
    game.untap(crew_two);
    let repeated_trigger_queue =
        activate_mighty_servant_crew(&mut game, vehicle_id, alice, vec!["Crew One", "Crew Two"]);
    assert!(
        repeated_trigger_queue.entries.is_empty(),
        "Mighty Servant should not retrigger when the same two creatures crew it again that turn"
    );

    let combat_damage = TriggerEvent::new_with_provenance(
        crate::events::DamageEvent::with_cause(
            vehicle_id,
            crate::events::DamageTarget::Player(bob),
            6,
            true,
            crate::events::cause::EventCause::combat_damage(vehicle_id),
        ),
        crate::provenance::ProvNodeId::default(),
    );
    let mut damage_trigger_queue = TriggerQueue::new();
    for trigger in crate::triggers::check_triggers(&game, &combat_damage) {
        damage_trigger_queue.add(trigger);
    }
    assert_eq!(
        damage_trigger_queue.entries.len(),
        1,
        "granted combat-damage trigger should be present after exact two-creature crew"
    );

    let defending_creature = create_vanilla_creature(&mut game, "Defending Creature", bob, 2, 2);
    let creature_damage = TriggerEvent::new_with_provenance(
        crate::events::DamageEvent::with_cause(
            vehicle_id,
            crate::events::DamageTarget::Object(defending_creature),
            6,
            true,
            crate::events::cause::EventCause::combat_damage(vehicle_id),
        ),
        crate::provenance::ProvNodeId::default(),
    );
    assert!(
        crate::triggers::check_triggers(&game, &creature_damage).is_empty(),
        "Mighty Servant's granted trigger should only trigger on combat damage to a player"
    );

    let hand_before = game.player(alice).expect("alice exists").hand.len();
    put_triggers_on_stack(&mut game, &mut damage_trigger_queue)
        .expect("granted combat-damage trigger should go on the stack");
    resolve_stack_entry(&mut game).expect("granted combat-damage trigger should resolve");
    assert_eq!(
        game.player(alice).expect("alice exists").hand.len(),
        hand_before + 2,
        "Mighty Servant's granted trigger should draw two cards"
    );

    execute_cleanup_step(&mut game);
    let expired_triggers = crate::triggers::check_triggers(&game, &combat_damage);
    assert!(
        expired_triggers.is_empty(),
        "Mighty Servant's granted combat-damage trigger should expire at cleanup"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn mighty_servant_crew_condition_requires_exactly_two_on_first_crew() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let mighty = mighty_servant_of_leuk_o_definition();
    let vehicle_id = game.create_object_from_definition(&mighty, alice, Zone::Battlefield);
    create_vanilla_creature(&mut game, "Solo Crew", alice, 4, 4);
    create_vanilla_creature(&mut game, "Later Crew One", alice, 2, 2);
    create_vanilla_creature(&mut game, "Later Crew Two", alice, 2, 2);

    let solo_trigger_queue =
        activate_mighty_servant_crew(&mut game, vehicle_id, alice, vec!["Solo Crew"]);
    assert!(
        solo_trigger_queue.entries.is_empty(),
        "Mighty Servant should not trigger when first crewed by one creature"
    );

    let later_trigger_queue = activate_mighty_servant_crew(
        &mut game,
        vehicle_id,
        alice,
        vec!["Later Crew One", "Later Crew Two"],
    );
    assert!(
        later_trigger_queue.entries.is_empty(),
        "Mighty Servant should not trigger on a later crew activation even if that activation used exactly two creatures"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn mighty_servant_first_crew_by_three_creatures_does_not_grant_damage_draw() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let mighty = mighty_servant_of_leuk_o_definition();
    let vehicle_id = game.create_object_from_definition(&mighty, alice, Zone::Battlefield);
    create_vanilla_creature(&mut game, "Crew One", alice, 2, 2);
    create_vanilla_creature(&mut game, "Crew Two", alice, 1, 1);
    create_vanilla_creature(&mut game, "Crew Three", alice, 1, 1);

    let trigger_queue = activate_mighty_servant_crew(
        &mut game,
        vehicle_id,
        alice,
        vec!["Crew One", "Crew Two", "Crew Three"],
    );
    assert!(
        trigger_queue.entries.is_empty(),
        "Mighty Servant should not trigger when first crewed by three creatures"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn vastwood_animist_activation_animates_only_your_land_using_ally_count_until_end_of_turn()
 {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let animist_def = vastwood_animist_definition();
    let animist_id = game.create_object_from_definition(&animist_def, alice, Zone::Battlefield);
    game.remove_summoning_sickness(animist_id);
    let other_ally = CardBuilder::new(CardId::new(), "Ally Helper")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Ally])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    game.create_object_from_card(&other_ally, alice, Zone::Battlefield);
    let _opponent_land_id = create_test_land(&mut game, "Bob's Forest", bob);

    assert!(
        !crate::decision::compute_legal_actions(&game, alice)
            .into_iter()
            .any(|action| matches!(
                action,
                crate::decision::LegalAction::ActivateAbility { source, .. }
                    if source == animist_id
            )),
        "Vastwood Animist should not be activatable without a land you control to target"
    );

    let land_id = create_test_land(&mut game, "Alice's Forest", alice);
    assert!(!game.current_is_creature(land_id));

    let ability_index = game
        .object(animist_id)
        .expect("Vastwood Animist should exist")
        .abilities
        .iter()
        .position(|ability| matches!(ability.kind, AbilityKind::Activated(_)))
        .expect("Vastwood Animist should have an activated ability");
    let activate_action = crate::decision::compute_legal_actions(&game, alice)
        .into_iter()
        .find(|action| {
            matches!(
                action,
                crate::decision::LegalAction::ActivateAbility { source, ability_index: idx }
                    if *source == animist_id && *idx == ability_index
            )
        })
        .expect("Vastwood Animist activation should be legal with a land you control");

    let mut trigger_queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = SelectFirstDecisionMaker;
    let progress = apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(activate_action),
        &mut dm,
    )
    .expect("Vastwood Animist activation should start and pay the tap cost");
    match progress {
        crate::decision::GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::Targets(_),
        ) => {}
        other => panic!("expected target selection for Vastwood Animist, got {other:?}"),
    }

    apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::Targets(vec![Target::Object(land_id)]),
        &mut dm,
    )
    .expect("choosing a land you control should complete Vastwood Animist activation");
    assert!(
        game.is_tapped(animist_id),
        "activation should tap Vastwood Animist"
    );
    resolve_stack_entry(&mut game).expect("Vastwood Animist ability should resolve");

    assert!(game.current_is_creature(land_id));
    assert!(
        game.current_card_types(land_id).is_some_and(|types| {
            types.contains(&CardType::Land) && types.contains(&CardType::Creature)
        }),
        "animated land should remain a land and gain creature"
    );
    assert!(
        game.current_has_subtype(land_id, Subtype::Elemental),
        "animated land should become an Elemental"
    );
    assert_eq!(
        game.current_power(land_id),
        Some(2),
        "X should be the number of Allies Alice controls at resolution"
    );
    assert_eq!(game.current_toughness(land_id), Some(2));

    execute_cleanup_step(&mut game);
    game.refresh_continuous_state();

    assert!(!game.current_is_creature(land_id));
    assert_eq!(game.current_power(land_id), None);
    assert!(
        game.current_card_types(land_id).is_some_and(
            |types| types.contains(&CardType::Land) && !types.contains(&CardType::Creature)
        ),
        "animation should expire at end of turn while leaving the permanent a land"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn cho_arrim_alchemist_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(19647), "Cho-Arrim Alchemist")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::White]]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Human])
        .power_toughness(PowerToughness::fixed(1, 1))
        .parse_text(
            "{1}{W}{W}, {T}, Discard a card: The next time a source of your choice would deal damage to you this turn, prevent that damage. You gain life equal to the damage prevented this way.",
        )
        .expect("Cho-Arrim Alchemist should parse")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) struct ChooseNamedSourceDecisionMaker {
    pub(super) source_name: &'static str,
}

#[cfg(ironsmith_runtime_parser_tests)]
impl DecisionMaker for ChooseNamedSourceDecisionMaker {
    fn decide_objects(
        &mut self,
        game: &GameState,
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
            AutoPassDecisionMaker.decide_objects(game, ctx)
        }
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn cho_arrim_alchemist_activation_pays_costs_and_registers_prevention_life_followup() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let alchemist = cho_arrim_alchemist_definition();
    let alchemist_id = game.create_object_from_definition(&alchemist, alice, Zone::Battlefield);
    game.remove_summoning_sickness(alchemist_id);
    let discard_fuel = CardBuilder::new(CardId::new(), "Discard Fuel")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let discard_id = game.create_object_from_card(&discard_fuel, alice, Zone::Hand);
    let discard_stable = game
        .object(discard_id)
        .expect("discard card exists")
        .stable_id;
    let chosen_source = create_vanilla_creature(&mut game, "Chosen Damage Source", bob, 2, 2);

    game.player_mut(alice)
        .expect("Alice exists")
        .mana_pool
        .add(ManaSymbol::White, 3);

    let ability_index = game
        .object(alchemist_id)
        .expect("Cho-Arrim Alchemist should exist")
        .abilities
        .iter()
        .position(|ability| matches!(ability.kind, AbilityKind::Activated(_)))
        .expect("Cho-Arrim Alchemist should have an activated ability");
    let activate_action = crate::decision::compute_legal_actions(&game, alice)
        .into_iter()
        .find(|action| {
            matches!(
                action,
                crate::decision::LegalAction::ActivateAbility { source, ability_index: idx }
                    if *source == alchemist_id && *idx == ability_index
            )
        })
        .expect("Cho-Arrim Alchemist activation should be legal with mana and discard fuel");

    let mut trigger_queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = ChooseNamedSourceDecisionMaker {
        source_name: "Chosen Damage Source",
    };
    let progress = apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(activate_action),
        &mut dm,
    )
    .expect("Cho-Arrim Alchemist activation should pay costs and go on the stack");

    let mut progress = progress;
    let mut paid_discard = false;
    let mut cost_steps = 0;
    while cost_steps < 6 {
        cost_steps += 1;
        let next_progress = match progress {
            crate::decision::GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::SelectObjects(_),
            ) => {
                paid_discard = true;
                apply_priority_response_with_dm(
                    &mut game,
                    &mut trigger_queue,
                    &mut state,
                    &PriorityResponse::CardCostChoice(discard_id),
                    &mut dm,
                )
                .expect("discarding a card should continue paying Cho-Arrim Alchemist's cost")
            }
            crate::decision::GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::SelectOptions(cost_ctx),
            ) if cost_ctx
                .description
                .to_ascii_lowercase()
                .contains("choose the next cost") =>
            {
                let option = cost_ctx
                    .options
                    .iter()
                    .find(|option| {
                        option.legal
                            && !paid_discard
                            && option.description.to_ascii_lowercase().contains("discard")
                    })
                    .or_else(|| cost_ctx.options.iter().find(|option| option.legal))
                    .expect("Cho-Arrim Alchemist should have a payable remaining cost");
                apply_priority_response_with_dm(
                    &mut game,
                    &mut trigger_queue,
                    &mut state,
                    &PriorityResponse::NextCostChoice(option.index),
                    &mut dm,
                )
                .expect("choosing next Cho-Arrim Alchemist cost should continue activation")
            }
            other => {
                progress = other;
                break;
            }
        };
        progress = next_progress;
    }

    assert!(
        matches!(
            progress,
            crate::decision::GameProgress::Continue
                | crate::decision::GameProgress::NeedsDecisionCtx(
                    crate::decisions::context::DecisionContext::Priority(_)
                )
        ),
        "expected Cho-Arrim Alchemist activation to finish after discard cost, got {progress:?}"
    );

    assert!(
        game.is_tapped(alchemist_id),
        "activation should tap Cho-Arrim Alchemist"
    );
    let discarded_id = game
        .find_object_by_stable_id(discard_stable)
        .expect("discarded card should remain tracked");
    assert!(
        !game
            .player(alice)
            .expect("Alice exists")
            .hand
            .contains(&discarded_id),
        "activation should remove a discarded card from hand as an activation cost"
    );

    resolve_stack_entry_with(&mut game, &mut dm)
        .expect("Cho-Arrim Alchemist ability should resolve");
    let (damage, prevented) = crate::events::processing::process_damage_with_event(
        &mut game,
        chosen_source,
        crate::events::DamageTarget::Player(alice),
        2,
        false,
        crate::events::cause::EventCause::effect(),
    );
    assert_eq!(
        damage, 0,
        "chosen source damage to Alice should be prevented"
    );
    assert!(
        prevented,
        "the next matching damage event should be replaced"
    );
    assert_eq!(
        game.life_total(alice),
        22,
        "Cho-Arrim Alchemist should gain life equal to the prevented damage"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn activate_goblin_kites_targeting(
    game: &mut GameState,
    controller: PlayerId,
    kites_id: ObjectId,
    target_id: ObjectId,
) {
    let ability_index = game
        .object(kites_id)
        .expect("Goblin Kites should exist")
        .abilities
        .iter()
        .position(|ability| matches!(ability.kind, AbilityKind::Activated(_)))
        .expect("Goblin Kites should have an activated ability");
    let activate_action = crate::decision::compute_legal_actions(game, controller)
        .into_iter()
        .find(|action| {
            matches!(
                action,
                crate::decision::LegalAction::ActivateAbility { source, ability_index: idx }
                    if *source == kites_id && *idx == ability_index
            )
        })
        .expect("Goblin Kites activation should be legal");

    let mut trigger_queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = SelectFirstDecisionMaker;
    apply_priority_response_with_dm(
        game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(activate_action),
        &mut dm,
    )
    .expect("Goblin Kites activation should start");
    apply_priority_response_with_dm(
        game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::Targets(vec![Target::Object(target_id)]),
        &mut dm,
    )
    .expect("choosing Goblin Kites target should complete activation");
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn resolve_goblin_kites_delayed_trigger(game: &mut GameState) {
    let mut trigger_queue = TriggerQueue::new();
    let end_step_event = TriggerEvent::new_with_provenance(
        crate::events::phase::BeginningOfEndStepEvent::new(game.turn.active_player),
        crate::provenance::ProvNodeId::default(),
    );
    for trigger in crate::triggers::check_delayed_triggers(game, &end_step_event) {
        trigger_queue.add(trigger);
    }
    put_triggers_on_stack(game, &mut trigger_queue)
        .expect("Goblin Kites delayed trigger should go on stack");
    let mut dm = SelectFirstDecisionMaker;
    resolve_stack_entry_with(game, &mut dm).expect("Goblin Kites delayed trigger should resolve");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn goblin_kites_activation_requires_creature_you_control_with_toughness_two_or_less() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let kites_id =
        game.create_object_from_definition(&goblin_kites_definition(), alice, Zone::Battlefield);
    create_vanilla_creature(&mut game, "Too Tough", alice, 2, 3);
    create_vanilla_creature(&mut game, "Opponent's 1/1", bob, 1, 1);
    game.player_mut(alice)
        .expect("Alice should exist")
        .mana_pool
        .add(ManaSymbol::Red, 1);

    assert!(
        !crate::decision::compute_legal_actions(&game, alice)
            .into_iter()
            .any(|action| matches!(
                action,
                crate::decision::LegalAction::ActivateAbility { source, .. } if source == kites_id
            )),
        "Goblin Kites should not be activatable without a legal controlled low-toughness target"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn goblin_kites_grants_flying_and_sacrifices_target_after_losing_delayed_flip() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.set_random_seed(7);

    let kites_id =
        game.create_object_from_definition(&goblin_kites_definition(), alice, Zone::Battlefield);
    let target_id = create_vanilla_creature(&mut game, "Kited Goblin", alice, 1, 1);
    let target_stable = game
        .object(target_id)
        .expect("target should exist")
        .stable_id;
    game.player_mut(alice)
        .expect("Alice should exist")
        .mana_pool
        .add(ManaSymbol::Red, 1);

    activate_goblin_kites_targeting(&mut game, alice, kites_id, target_id);
    resolve_stack_entry(&mut game).expect("Goblin Kites ability should resolve");
    game.refresh_continuous_state();
    assert!(
        game.object_has_static_ability_id(
            target_id,
            crate::static_abilities::StaticAbilityId::Flying
        ),
        "Goblin Kites should grant flying to the targeted creature until end of turn"
    );
    assert_eq!(
        game.effect_store.delayed_triggers.len(),
        1,
        "Goblin Kites should schedule exactly one delayed end-step coin flip"
    );

    resolve_goblin_kites_delayed_trigger(&mut game);
    let moved_id = game
        .find_object_by_stable_id(target_stable)
        .expect("sacrificed target should still be tracked");
    assert!(
        game.player(alice)
            .expect("Alice should exist")
            .graveyard
            .contains(&moved_id),
        "losing the delayed Goblin Kites flip should sacrifice the targeted creature"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn goblin_kites_winning_delayed_flip_leaves_target_on_battlefield() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.set_random_seed(2);

    let kites_id =
        game.create_object_from_definition(&goblin_kites_definition(), alice, Zone::Battlefield);
    let target_id = create_vanilla_creature(&mut game, "Lucky Goblin", alice, 1, 1);
    let target_stable = game
        .object(target_id)
        .expect("target should exist")
        .stable_id;
    game.player_mut(alice)
        .expect("Alice should exist")
        .mana_pool
        .add(ManaSymbol::Red, 1);

    activate_goblin_kites_targeting(&mut game, alice, kites_id, target_id);
    resolve_stack_entry(&mut game).expect("Goblin Kites ability should resolve");
    resolve_goblin_kites_delayed_trigger(&mut game);

    let current_id = game
        .find_object_by_stable_id(target_stable)
        .expect("target should still be tracked");
    assert!(
        game.battlefield.contains(&current_id),
        "winning the delayed Goblin Kites flip should leave the targeted creature on the battlefield"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn tsabos_assassin_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(72_960), "Tsabo's Assassin")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Black],
            vec![ManaSymbol::Black],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Phyrexian, Subtype::Zombie, Subtype::Assassin])
        .power_toughness(PowerToughness::fixed(1, 1))
        .parse_text(
            "{T}: Destroy target creature if it shares a color with the most common color among all permanents or a color tied for most common. A creature destroyed this way can't be regenerated.",
        )
        .expect("Tsabo's Assassin should parse")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn resolve_tsabos_assassin_conditional_effect_targeting(
    game: &mut GameState,
    assassin_id: ObjectId,
    target_id: ObjectId,
    controller: PlayerId,
) {
    let ability_index = game
        .object(assassin_id)
        .expect("Tsabo's Assassin should exist")
        .abilities
        .iter()
        .position(|ability| matches!(ability.kind, AbilityKind::Activated(_)))
        .expect("Tsabo's Assassin should have an activated ability");
    let conditional_effect = match &game
        .object(assassin_id)
        .expect("Tsabo's Assassin should exist")
        .abilities[ability_index]
        .kind
    {
        AbilityKind::Activated(activated) => activated.effects.segments[0]
            .default_effects
            .iter()
            .find(|effect| {
                effect
                    .downcast_ref::<crate::effects::ConditionalEffect>()
                    .is_some()
            })
            .expect("Tsabo's Assassin should have a conditional destroy effect")
            .clone(),
        _ => panic!("Tsabo's Assassin ability should be activated"),
    };
    let target_snapshot = crate::snapshot::ObjectSnapshot::from_object(
        game.object(target_id)
            .expect("Tsabo's Assassin target should exist"),
        game,
    );
    let tagged = std::collections::HashMap::from([(
        crate::tag::TagKey::from("targeted_0"),
        vec![target_snapshot],
    )]);
    let mut ctx = ExecutionContext::new_default(assassin_id, controller)
        .with_targets(vec![crate::effects::ResolvedTarget::Object(target_id)])
        .with_tagged_objects(tagged);
    crate::effects::execute_effect(game, &conditional_effect, &mut ctx)
        .expect("Tsabo's Assassin conditional destroy should resolve");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn tsabos_assassin_activation_is_legal_taps_source_and_resolves_from_stack() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let assassin_id =
        game.create_object_from_definition(&tsabos_assassin_definition(), alice, Zone::Battlefield);
    game.remove_summoning_sickness(assassin_id);
    let target_id = create_colored_creature(
        &mut game,
        "Green Target",
        bob,
        Some(crate::color::ColorSet::GREEN),
    );
    let target_stable = game.object(target_id).expect("target exists").stable_id;
    let ability_index = game
        .object(assassin_id)
        .expect("Tsabo's Assassin should exist")
        .abilities
        .iter()
        .position(|ability| matches!(ability.kind, AbilityKind::Activated(_)))
        .expect("Tsabo's Assassin should have an activated ability");

    let activate_action = crate::decision::compute_legal_actions(&game, alice)
        .into_iter()
        .find(|action| {
            matches!(
                action,
                crate::decision::LegalAction::ActivateAbility { source, ability_index: idx }
                    if *source == assassin_id && *idx == ability_index
            )
        })
        .expect("Tsabo's Assassin activation should be legal");

    let mut trigger_queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = SelectFirstDecisionMaker;
    apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(activate_action),
        &mut dm,
    )
    .expect("Tsabo's Assassin activation should start");
    apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::Targets(vec![Target::Object(target_id)]),
        &mut dm,
    )
    .expect("choosing Tsabo's Assassin target should complete activation");

    assert!(
        game.is_tapped(assassin_id),
        "paying Tsabo's Assassin's activation cost should tap it"
    );

    resolve_stack_entry(&mut game).expect("Tsabo's Assassin ability should resolve");
    let graveyard_id = game
        .find_object_by_stable_id(target_stable)
        .expect("destroyed target should still be tracked by stable id");
    assert!(
        game.player(bob)
            .expect("Bob exists")
            .graveyard
            .contains(&graveyard_id),
        "resolving Tsabo's Assassin from the stack should destroy a target sharing a tied most-common color"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn tsabos_assassin_destroys_creature_sharing_most_common_permanent_color() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let assassin_id =
        game.create_object_from_definition(&tsabos_assassin_definition(), alice, Zone::Battlefield);
    game.remove_summoning_sickness(assassin_id);
    create_colored_creature(
        &mut game,
        "Blue Permanent A",
        alice,
        Some(crate::color::ColorSet::BLUE),
    );
    create_colored_creature(
        &mut game,
        "Blue Permanent B",
        bob,
        Some(crate::color::ColorSet::BLUE),
    );
    let target_id = create_colored_creature(
        &mut game,
        "Blue Target With Green Base Color",
        bob,
        Some(crate::color::ColorSet::GREEN),
    );
    game.object_mut(target_id)
        .expect("target exists")
        .color_override = Some(crate::color::ColorSet::BLUE);
    let target_stable = game.object(target_id).expect("target exists").stable_id;
    game.add_regeneration_shield(target_id, 1);
    let filter = crate::target::ObjectFilter::default().shares_most_common_permanent_color();
    assert_eq!(
        game.current_colors(target_id),
        Some(crate::color::ColorSet::BLUE),
        "test setup should make the target currently blue despite its printed green color"
    );
    assert!(
        filter.matches(
            game.object(target_id).expect("target exists"),
            &game.filter_context_for(alice, Some(assassin_id)),
            &game,
        ),
        "blue target should match the most-common permanent color predicate"
    );

    resolve_tsabos_assassin_conditional_effect_targeting(&mut game, assassin_id, target_id, alice);
    let graveyard_id = game
        .find_object_by_stable_id(target_stable)
        .expect("destroyed target should still be tracked by stable id");

    assert!(
        game.player(bob)
            .expect("Bob exists")
            .graveyard
            .contains(&graveyard_id),
        "target sharing the most common permanent color should be destroyed"
    );
    assert_eq!(
        game.regenerated_this_turn_count(target_id),
        0,
        "Tsabo's Assassin should prevent regeneration for the destroyed creature"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn tsabos_assassin_destroys_creature_sharing_tied_most_common_color() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let assassin_id =
        game.create_object_from_definition(&tsabos_assassin_definition(), alice, Zone::Battlefield);
    game.remove_summoning_sickness(assassin_id);
    create_colored_creature(
        &mut game,
        "Blue Permanent A",
        alice,
        Some(crate::color::ColorSet::BLUE),
    );
    create_colored_creature(
        &mut game,
        "Blue Permanent B",
        bob,
        Some(crate::color::ColorSet::BLUE),
    );
    create_colored_creature(
        &mut game,
        "Red Permanent",
        alice,
        Some(crate::color::ColorSet::RED),
    );
    let target_id = create_colored_creature(
        &mut game,
        "Red Target",
        bob,
        Some(crate::color::ColorSet::RED),
    );
    let target_stable = game.object(target_id).expect("target exists").stable_id;
    let filter = crate::target::ObjectFilter::default().shares_most_common_permanent_color();
    assert!(
        filter.matches(
            game.object(target_id).expect("target exists"),
            &game.filter_context_for(alice, Some(assassin_id)),
            &game,
        ),
        "red target should match the tied most-common permanent color predicate"
    );

    resolve_tsabos_assassin_conditional_effect_targeting(&mut game, assassin_id, target_id, alice);
    let graveyard_id = game
        .find_object_by_stable_id(target_stable)
        .expect("destroyed target should still be tracked by stable id");

    assert!(
        game.player(bob)
            .expect("Bob exists")
            .graveyard
            .contains(&graveyard_id),
        "target sharing a tied most-common permanent color should be destroyed"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn tsabos_assassin_does_not_destroy_nonmatching_creature() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let assassin_id =
        game.create_object_from_definition(&tsabos_assassin_definition(), alice, Zone::Battlefield);
    game.remove_summoning_sickness(assassin_id);
    create_colored_creature(
        &mut game,
        "Blue Permanent A",
        alice,
        Some(crate::color::ColorSet::BLUE),
    );
    create_colored_creature(
        &mut game,
        "Blue Permanent B",
        bob,
        Some(crate::color::ColorSet::BLUE),
    );
    let target_id = create_colored_creature(
        &mut game,
        "Green Target",
        bob,
        Some(crate::color::ColorSet::GREEN),
    );
    let filter = crate::target::ObjectFilter::default().shares_most_common_permanent_color();
    assert!(
        !filter.matches(
            game.object(target_id).expect("target exists"),
            &game.filter_context_for(alice, Some(assassin_id)),
            &game,
        ),
        "green target should not match the most-common permanent color predicate"
    );

    resolve_tsabos_assassin_conditional_effect_targeting(&mut game, assassin_id, target_id, alice);

    assert!(
        game.battlefield.contains(&target_id),
        "target that does not share a most-common or tied color should remain on the battlefield"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn explore_event_for(
    game: &GameState,
    controller: PlayerId,
    source: ObjectId,
) -> TriggerEvent {
    let snapshot = game
        .object(source)
        .map(|object| crate::snapshot::ObjectSnapshot::from_object(object, game));
    TriggerEvent::new_with_provenance(
        crate::events::KeywordActionEvent::new(
            crate::events::KeywordActionKind::Explore,
            controller,
            source,
            1,
        )
        .with_snapshot(snapshot),
        crate::provenance::ProvNodeId::default(),
    )
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn open_the_way_x_choice_is_capped_by_players_in_game() {
    let mut game = setup_three_player_game();
    let alice = PlayerId::from_index(0);
    let charlie = PlayerId::from_index(2);

    let forest = crate::cards::definitions::basic_forest();
    for _ in 0..8 {
        game.create_object_from_definition(&forest, alice, Zone::Battlefield);
    }

    let open_the_way = open_the_way_definition();
    let spell_id = game.create_object_from_definition(&open_the_way, alice, Zone::Stack);
    let mana_cost = game.object(spell_id).unwrap().mana_cost_owned();

    let (needs_x, _min_x, max_x) = compute_spell_cast_x_bounds(
        &game,
        alice,
        spell_id,
        &CastingMethod::Normal,
        mana_cost.as_ref(),
    );
    assert!(needs_x, "Open the Way should ask for X while being cast");
    assert_eq!(max_x, 3, "three players in game should cap X at 3");

    game.player_mut(charlie).unwrap().has_lost = true;
    let (_, _, max_x_after_player_lost) = compute_spell_cast_x_bounds(
        &game,
        alice,
        spell_id,
        &CastingMethod::Normal,
        mana_cost.as_ref(),
    );
    assert_eq!(
        max_x_after_player_lost, 2,
        "players no longer in the game should stop increasing Open the Way's X cap"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn desmond_miles_counts_other_assassins_and_assassin_cards_in_your_graveyard() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let desmond = desmond_miles_definition();
    let desmond_id = game.create_object_from_definition(&desmond, alice, Zone::Battlefield);
    assert_eq!(
        game.calculated_power(desmond_id),
        Some(1),
        "Desmond should not count itself as another Assassin"
    );

    let bystander = CardBuilder::new(CardId::from_raw(72_911), "Bystander")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Human])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    game.create_object_from_card(&bystander, alice, Zone::Battlefield);
    assert_eq!(
        game.calculated_power(desmond_id),
        Some(1),
        "non-Assassins you control should not increase Desmond's power"
    );

    let assassin = CardBuilder::new(CardId::from_raw(72_912), "Assassin Ally")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Assassin])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    game.create_object_from_card(&assassin, bob, Zone::Battlefield);
    game.create_object_from_card(&assassin, alice, Zone::Hand);
    assert_eq!(
        game.calculated_power(desmond_id),
        Some(1),
        "opponents' Assassins and non-graveyard Assassin cards should not count"
    );

    game.create_object_from_card(&assassin, alice, Zone::Battlefield);
    assert_eq!(
        game.calculated_power(desmond_id),
        Some(2),
        "another Assassin you control should add +1/+0"
    );

    game.create_object_from_card(&assassin, alice, Zone::Graveyard);
    assert_eq!(
        game.calculated_power(desmond_id),
        Some(3),
        "an Assassin card in your graveyard should add another +1/+0"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn desmond_miles_combat_damage_trigger_surveils_equal_to_damage_and_ignores_noncombat() {
    #[derive(Default)]
    struct RecordingSurveilDecisionMaker {
        viewed_cards: usize,
        partition_description: String,
    }

    impl DecisionMaker for RecordingSurveilDecisionMaker {
        fn view_cards(
            &mut self,
            _game: &GameState,
            _viewer: PlayerId,
            cards: &[ObjectId],
            _ctx: &crate::decisions::context::ViewCardsContext,
        ) {
            self.viewed_cards = cards.len();
        }

        fn decide_partition(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::PartitionContext,
        ) -> Vec<ObjectId> {
            self.partition_description = ctx.description.clone();
            ctx.cards
                .first()
                .map(|(id, _)| vec![*id])
                .unwrap_or_default()
        }
    }

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let desmond = desmond_miles_definition();
    let desmond_id = game.create_object_from_definition(&desmond, alice, Zone::Battlefield);
    for idx in 0..4 {
        let card = CardBuilder::new(
            CardId::from_raw(72_920 + idx),
            format!("Library Card {idx}"),
        )
        .card_types(vec![CardType::Creature])
        .build();
        game.create_object_from_card(&card, alice, Zone::Library);
    }

    let noncombat_damage = TriggerEvent::new_with_provenance(
        crate::events::DamageEvent::with_cause(
            desmond_id,
            crate::events::DamageTarget::Player(bob),
            3,
            false,
            crate::events::cause::EventCause::effect(),
        ),
        crate::provenance::ProvNodeId::default(),
    );
    assert!(
        crate::triggers::check_triggers(&game, &noncombat_damage).is_empty(),
        "Desmond should not trigger from noncombat damage"
    );

    let combat_damage = TriggerEvent::new_with_provenance(
        crate::events::DamageEvent::with_cause(
            desmond_id,
            crate::events::DamageTarget::Player(bob),
            3,
            true,
            crate::events::cause::EventCause::combat_damage(desmond_id),
        ),
        crate::provenance::ProvNodeId::default(),
    );
    let mut trigger_queue = TriggerQueue::new();
    for trigger in crate::triggers::check_triggers(&game, &combat_damage) {
        trigger_queue.add(trigger);
    }
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "combat damage should trigger once"
    );

    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("Desmond combat-damage trigger should go on the stack");
    let mut dm = RecordingSurveilDecisionMaker::default();
    resolve_stack_entry_with(&mut game, &mut dm).expect("Desmond surveil trigger should resolve");

    assert_eq!(dm.viewed_cards, 3, "surveil X should use the damage amount");
    assert_eq!(dm.partition_description, "Surveil 3");
    assert_eq!(
        game.player(alice).expect("alice exists").graveyard.len(),
        1,
        "the test decision maker should put one surveilled card into the graveyard"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn merfolk_cave_diver_triggers_on_your_creature_exploring_and_expires_at_cleanup() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let merfolk = merfolk_cave_diver_definition();
    let merfolk_id = game.create_object_from_definition(&merfolk, alice, Zone::Battlefield);
    let explorer_id = create_creature(&mut game, "Friendly Explorer", alice, 1, 1);
    let blocker_id = create_creature(&mut game, "Ground Blocker", bob, 2, 2);

    let event = explore_event_for(&game, alice, explorer_id);
    let mut trigger_queue = TriggerQueue::new();
    for trigger in crate::triggers::check_triggers(&game, &event) {
        trigger_queue.add(trigger);
    }
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "Merfolk Cave-Diver should trigger when a creature you control explores"
    );

    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("Merfolk Cave-Diver trigger should go on the stack");
    resolve_stack_entry(&mut game).expect("Merfolk Cave-Diver trigger should resolve");
    game.refresh_continuous_state();

    assert_eq!(
        game.calculated_power(merfolk_id),
        Some(3),
        "Merfolk Cave-Diver should get +1/+0 after the explore trigger resolves"
    );
    assert_eq!(
        game.calculated_toughness(merfolk_id),
        Some(4),
        "Merfolk Cave-Diver should not get a toughness bonus"
    );

    let mut combat = CombatState::default();
    combat.attackers.push(crate::combat_state::AttackerInfo {
        creature: merfolk_id,
        target: AttackTarget::Player(bob),
    });
    let mut blocker_queue = TriggerQueue::new();
    let err = apply_blocker_declarations(
        &mut game,
        &mut combat,
        &mut blocker_queue,
        &[BlockerDeclaration {
            blocker: blocker_id,
            blocking: merfolk_id,
        }],
        bob,
    )
    .expect_err("Merfolk Cave-Diver shouldn't be blockable after the trigger resolves");
    let msg = format!("{err:?}");
    assert!(
        msg.contains("InvalidBlockers"),
        "expected blocking Merfolk Cave-Diver to be rejected, got {msg}"
    );

    execute_cleanup_step(&mut game);
    game.refresh_continuous_state();
    assert_eq!(
        game.calculated_power(merfolk_id),
        Some(2),
        "Merfolk Cave-Diver's +1/+0 should expire during cleanup"
    );
    assert!(
        game.can_be_blocked(merfolk_id),
        "Merfolk Cave-Diver's can't-be-blocked restriction should expire during cleanup"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn merfolk_cave_diver_ignores_opponent_creatures_exploring() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let merfolk = merfolk_cave_diver_definition();
    let merfolk_id = game.create_object_from_definition(&merfolk, alice, Zone::Battlefield);
    let opposing_explorer_id = create_creature(&mut game, "Opposing Explorer", bob, 1, 1);

    let event = explore_event_for(&game, bob, opposing_explorer_id);
    let triggers = crate::triggers::check_triggers(&game, &event);
    assert!(
        triggers.is_empty(),
        "Merfolk Cave-Diver should not trigger when an opponent's creature explores"
    );
    assert_eq!(
        game.calculated_power(merfolk_id),
        Some(2),
        "Merfolk Cave-Diver should not be pumped by an opponent's explore event"
    );
    assert!(
        game.can_be_blocked(merfolk_id),
        "Merfolk Cave-Diver should remain blockable without a matching explore trigger"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn open_the_way_reveals_x_lands_to_battlefield_tapped_and_bottoms_rest() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let open_the_way = open_the_way_definition();
    let spell_id = game.create_object_from_definition(&open_the_way, alice, Zone::Stack);
    game.object_mut(spell_id).unwrap().x_value = Some(2);
    let spell_stable = game.object(spell_id).unwrap().stable_id;

    let forest = crate::cards::definitions::basic_forest();
    let second_land = game.create_object_from_definition(&forest, alice, Zone::Library);
    let filler = CardBuilder::new(CardId::from_raw(72_901), "Filler Spell")
        .card_types(vec![CardType::Instant])
        .build();
    let filler_id = game.create_object_from_card(&filler, alice, Zone::Library);
    let top_land = game.create_object_from_definition(&forest, alice, Zone::Library);
    let second_land_stable = game.object(second_land).unwrap().stable_id;
    let filler_stable = game.object(filler_id).unwrap().stable_id;
    let top_land_stable = game.object(top_land).unwrap().stable_id;

    game.push_to_stack(StackEntry::new(spell_id, alice).with_x(2).with_source_info(
        game.object(spell_id).unwrap().stable_id,
        "Open the Way".to_string(),
    ));
    resolve_stack_entry(&mut game).expect("Open the Way should resolve");

    for stable_id in [top_land_stable, second_land_stable] {
        let land_id = game
            .find_object_by_stable_id(stable_id)
            .expect("revealed land should still exist after changing zones");
        assert!(
            game.battlefield.contains(&land_id),
            "revealed land {land_id:?} should be on the battlefield"
        );
        assert!(
            game.is_tapped(land_id),
            "revealed land {land_id:?} should enter tapped"
        );
    }
    let filler_id = game
        .find_object_by_stable_id(filler_stable)
        .expect("filler card should still exist after bottoming");
    assert!(
        game.player(alice).unwrap().library.contains(&filler_id),
        "nonmatching revealed cards should be put on the bottom of the library"
    );
    let resolved_spell_id = game
        .find_object_by_stable_id(spell_stable)
        .expect("Open the Way should still exist after resolving");
    assert!(
        game.player(alice)
            .unwrap()
            .graveyard
            .contains(&resolved_spell_id),
        "Open the Way should move to its owner's graveyard after resolving"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn covenant_of_minds_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(416_862), "Covenant of Minds")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(4)],
            vec![ManaSymbol::Blue],
        ]))
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Reveal the top three cards of your library. Target opponent may choose to put those cards into your hand. If they don't, put those cards into your graveyard and draw five cards.",
        )
        .expect("Covenant of Minds should parse for runtime tests")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) struct CovenantOfMindsDecisionMaker {
    pub(super) accept_opponent_choice: bool,
    pub(super) expected_decider: PlayerId,
    pub(super) prompts: usize,
}

#[cfg(ironsmith_runtime_parser_tests)]
impl DecisionMaker for CovenantOfMindsDecisionMaker {
    fn decide_boolean(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::BooleanContext,
    ) -> bool {
        self.prompts += 1;
        assert_eq!(
            ctx.player, self.expected_decider,
            "Covenant of Minds choice should be made by the targeted opponent"
        );
        self.accept_opponent_choice
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn create_covenant_library_card(
    game: &mut GameState,
    owner: PlayerId,
    name: &str,
) -> ObjectId {
    let card = CardBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Instant])
        .build();
    game.create_object_from_card(&card, owner, Zone::Library)
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn setup_covenant_of_minds_stack(
    game: &mut GameState,
    alice: PlayerId,
    bob: PlayerId,
) -> (ObjectId, Vec<crate::ids::StableId>) {
    let covenant = covenant_of_minds_definition();
    let spell_id = game.create_object_from_definition(&covenant, alice, Zone::Stack);
    for idx in 0..5 {
        create_covenant_library_card(game, alice, &format!("Covenant Draw Filler {idx}"));
    }
    let mut revealed = Vec::new();
    for name in [
        "Covenant Revealed A",
        "Covenant Revealed B",
        "Covenant Revealed C",
    ] {
        let id = create_covenant_library_card(game, alice, name);
        revealed.push(game.object(id).expect("revealed card exists").stable_id);
    }

    game.push_to_stack(StackEntry::new(spell_id, alice).with_targets(vec![Target::Player(bob)]));
    (spell_id, revealed)
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn covenant_of_minds_opponent_accepts_puts_revealed_cards_into_your_hand() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let (_spell_id, revealed) = setup_covenant_of_minds_stack(&mut game, alice, bob);

    let mut dm = CovenantOfMindsDecisionMaker {
        accept_opponent_choice: true,
        expected_decider: bob,
        prompts: 0,
    };
    resolve_stack_entry_with(&mut game, &mut dm).expect("Covenant of Minds should resolve");
    assert_eq!(dm.prompts, 1, "Covenant should ask exactly one may-choice");

    for stable_id in revealed {
        let id = game
            .find_object_by_stable_id(stable_id)
            .expect("revealed card should still exist");
        assert!(
            game.player(alice).expect("alice exists").hand.contains(&id),
            "accepted opponent choice should put revealed card {id:?} into Alice's hand"
        );
    }
    assert_eq!(
        game.player(alice).expect("alice exists").hand.len(),
        3,
        "accepting should not draw the fallback five cards"
    );
    assert_eq!(
        game.player(alice).expect("alice exists").library.len(),
        5,
        "accepting should leave the five unrevealed library cards in place"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn covenant_of_minds_opponent_declines_graveyards_revealed_cards_and_draws_five() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let (_spell_id, revealed) = setup_covenant_of_minds_stack(&mut game, alice, bob);

    let mut dm = CovenantOfMindsDecisionMaker {
        accept_opponent_choice: false,
        expected_decider: bob,
        prompts: 0,
    };
    resolve_stack_entry_with(&mut game, &mut dm).expect("Covenant of Minds should resolve");
    assert_eq!(dm.prompts, 1, "Covenant should ask exactly one may-choice");

    for stable_id in revealed {
        let id = game
            .find_object_by_stable_id(stable_id)
            .expect("revealed card should still exist");
        assert!(
            game.player(alice)
                .expect("alice exists")
                .graveyard
                .contains(&id),
            "declining should put revealed card {id:?} into Alice's graveyard"
        );
    }
    assert_eq!(
        game.player(alice).expect("alice exists").hand.len(),
        5,
        "declining should draw the remaining five library cards"
    );
    assert_eq!(
        game.player(alice).expect("alice exists").library.len(),
        0,
        "declining should draw the five unrevealed library cards after moving the revealed cards"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn rise_from_the_grave_returns_creature_under_your_control_black_zombie() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let spell = rise_from_the_grave_definition();
    let spell_id = game.create_object_from_definition(&spell, alice, Zone::Stack);
    let target_card = CardBuilder::new(CardId::from_raw(72_941), "Emerald Bear")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Bear])
        .color_indicator(crate::color::ColorSet::GREEN)
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let target_id = game.create_object_from_card(&target_card, bob, Zone::Graveyard);
    let target_stable = game.object(target_id).expect("target exists").stable_id;

    game.push_to_stack(
        StackEntry::new(spell_id, alice).with_targets(vec![Target::Object(target_id)]),
    );
    resolve_stack_entry(&mut game).expect("Rise from the Grave should resolve");

    let returned_id = game
        .find_object_by_stable_id(target_stable)
        .expect("returned creature should still exist");
    assert!(
        game.battlefield.contains(&returned_id),
        "target creature card should move from graveyard to battlefield"
    );
    assert_eq!(
        game.controller_of(game.object(returned_id).expect("returned object exists")),
        alice,
        "returned creature should be under the spell controller's control"
    );
    assert_eq!(
        game.current_colors(returned_id),
        Some(crate::color::ColorSet::GREEN.union(crate::color::ColorSet::BLACK)),
        "returned creature should keep green and add black"
    );
    let subtypes = game.calculated_subtypes(returned_id);
    assert!(
        subtypes.contains(&Subtype::Bear) && subtypes.contains(&Subtype::Zombie),
        "returned creature should keep Bear and add Zombie, got {subtypes:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn rise_from_the_grave_targets_only_creature_cards_in_graveyards() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let spell = rise_from_the_grave_definition();
    let effects = spell.spell_effect.as_ref().expect("expected spell effects");

    let graveyard_creature = game.create_object_from_card(
        &CardBuilder::new(CardId::from_raw(72_942), "Graveyard Creature")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build(),
        bob,
        Zone::Graveyard,
    );
    let graveyard_artifact = game.create_object_from_card(
        &CardBuilder::new(CardId::from_raw(72_943), "Graveyard Relic")
            .card_types(vec![CardType::Artifact])
            .build(),
        bob,
        Zone::Graveyard,
    );
    let battlefield_creature = create_creature(&mut game, "Battlefield Creature", bob, 2, 2);

    let requirements = extract_target_requirements(&game, effects, alice, None);
    assert_eq!(
        requirements.len(),
        1,
        "Rise should have one target requirement"
    );
    let legal_targets = &requirements[0].legal_targets;
    assert!(
        legal_targets.contains(&Target::Object(graveyard_creature)),
        "creature cards in graveyards should be legal targets, got {legal_targets:?}"
    );
    assert!(
        !legal_targets.contains(&Target::Object(graveyard_artifact)),
        "noncreature cards in graveyards should not be legal targets, got {legal_targets:?}"
    );
    assert!(
        !legal_targets.contains(&Target::Object(battlefield_creature)),
        "battlefield creatures should not be legal targets, got {legal_targets:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn necromantic_summons_returns_creature_without_counters_below_spell_mastery() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let spell = necromantic_summons_definition();
    let spell_id = game.create_object_from_definition(&spell, alice, Zone::Stack);
    let target_card = CardBuilder::new(CardId::from_raw(72_946), "Graveyard Bear")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let target_id = game.create_object_from_card(&target_card, bob, Zone::Graveyard);
    let target_stable = game.object(target_id).expect("target exists").stable_id;

    game.push_to_stack(
        StackEntry::new(spell_id, alice).with_targets(vec![Target::Object(target_id)]),
    );
    resolve_stack_entry(&mut game).expect("Necromantic Summons should resolve");

    let returned_id = game
        .find_object_by_stable_id(target_stable)
        .expect("returned creature should still exist");
    let returned = game.object(returned_id).expect("returned creature exists");
    assert_eq!(returned.zone, Zone::Battlefield);
    assert_eq!(game.controller_of(returned), alice);
    assert_eq!(
        returned
            .counters
            .get(&crate::object::CounterType::PlusOnePlusOne)
            .copied()
            .unwrap_or(0),
        0,
        "without spell mastery the returned creature should not get +1/+1 counters"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn necromantic_summons_spell_mastery_returns_creature_with_two_counters() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let spell = necromantic_summons_definition();
    let spell_id = game.create_object_from_definition(&spell, alice, Zone::Stack);
    for (idx, card_type) in [CardType::Instant, CardType::Sorcery]
        .into_iter()
        .enumerate()
    {
        let card = CardBuilder::new(CardId::from_raw(72_947 + idx as u32), "Spell Mastery Fuel")
            .card_types(vec![card_type])
            .build();
        game.create_object_from_card(&card, alice, Zone::Graveyard);
    }
    let target_card = CardBuilder::new(CardId::from_raw(72_949), "Mastered Bear")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let target_id = game.create_object_from_card(&target_card, bob, Zone::Graveyard);
    let target_stable = game.object(target_id).expect("target exists").stable_id;

    game.push_to_stack(
        StackEntry::new(spell_id, alice).with_targets(vec![Target::Object(target_id)]),
    );
    resolve_stack_entry(&mut game).expect("Necromantic Summons should resolve");

    let returned_id = game
        .find_object_by_stable_id(target_stable)
        .expect("returned creature should still exist");
    let returned = game.object(returned_id).expect("returned creature exists");
    assert_eq!(returned.zone, Zone::Battlefield);
    assert_eq!(game.controller_of(returned), alice);
    assert_eq!(
        returned
            .counters
            .get(&crate::object::CounterType::PlusOnePlusOne)
            .copied()
            .unwrap_or(0),
        2,
        "with spell mastery the returned creature should get two +1/+1 counters"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn necromantic_summons_targets_only_creature_cards_in_graveyards() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let spell = necromantic_summons_definition();
    let effects = spell.spell_effect.as_ref().expect("expected spell effects");

    let graveyard_creature = game.create_object_from_card(
        &CardBuilder::new(CardId::from_raw(72_950), "Graveyard Creature")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build(),
        bob,
        Zone::Graveyard,
    );
    let graveyard_artifact = game.create_object_from_card(
        &CardBuilder::new(CardId::from_raw(72_951), "Graveyard Relic")
            .card_types(vec![CardType::Artifact])
            .build(),
        bob,
        Zone::Graveyard,
    );
    let battlefield_creature = create_creature(&mut game, "Battlefield Creature", bob, 2, 2);

    let requirements = extract_target_requirements(&game, effects, alice, None);
    assert_eq!(
        requirements.len(),
        1,
        "Necromantic Summons should have one target requirement"
    );
    let legal_targets = &requirements[0].legal_targets;
    assert!(
        legal_targets.contains(&Target::Object(graveyard_creature)),
        "creature cards in graveyards should be legal targets, got {legal_targets:?}"
    );
    assert!(
        !legal_targets.contains(&Target::Object(graveyard_artifact)),
        "noncreature cards in graveyards should not be legal targets, got {legal_targets:?}"
    );
    assert!(
        !legal_targets.contains(&Target::Object(battlefield_creature)),
        "battlefield creatures should not be legal targets, got {legal_targets:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn dance_of_the_manse_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(72_960), "Dance of the Manse")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::X],
            vec![ManaSymbol::White],
            vec![ManaSymbol::Blue],
        ]))
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Return up to X target artifact and/or non-Aura enchantment cards each with mana value X or less from your graveyard to the battlefield. If X is 6 or more, those permanents are 4/4 creatures in addition to their other types.",
        )
        .expect("Dance of the Manse should parse")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn dance_graveyard_card(
    id: u32,
    name: &str,
    card_types: Vec<CardType>,
    subtypes: Vec<Subtype>,
    mana_value: u8,
) -> crate::card::Card {
    CardBuilder::new(CardId::from_raw(id), name)
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(
            mana_value,
        )]]))
        .card_types(card_types)
        .subtypes(subtypes)
        .build()
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn dance_of_the_manse_compiled_text_matches_oracle_animation_clause() {
    let def = dance_of_the_manse_definition();
    let rendered = crate::runtime_display::compiled_text_lines(&def).join(" ");

    assert_eq!(
        rendered,
        "Return up to X target artifact and/or non-Aura enchantment cards each with mana value X or less from your graveyard to the battlefield. If X is 6 or more, Those permanents are 4/4 creatures in addition to their other types."
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn dance_of_the_manse_targets_only_own_graveyard_artifacts_or_non_aura_enchantments() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let spell = dance_of_the_manse_definition();
    let spell_id = game.create_object_from_definition(&spell, alice, Zone::Stack);
    game.object_mut(spell_id).unwrap().x_value = Some(6);

    let artifact = game.create_object_from_card(
        &dance_graveyard_card(
            72_961,
            "Graveyard Relic",
            vec![CardType::Artifact],
            Vec::new(),
            6,
        ),
        alice,
        Zone::Graveyard,
    );
    let enchantment = game.create_object_from_card(
        &dance_graveyard_card(
            72_962,
            "Graveyard Sigil",
            vec![CardType::Enchantment],
            Vec::new(),
            6,
        ),
        alice,
        Zone::Graveyard,
    );
    let aura = game.create_object_from_card(
        &dance_graveyard_card(
            72_963,
            "Graveyard Aura",
            vec![CardType::Enchantment],
            vec![Subtype::Aura],
            6,
        ),
        alice,
        Zone::Graveyard,
    );
    let too_expensive = game.create_object_from_card(
        &dance_graveyard_card(
            72_964,
            "Expensive Relic",
            vec![CardType::Artifact],
            Vec::new(),
            7,
        ),
        alice,
        Zone::Graveyard,
    );
    let opponents_artifact = game.create_object_from_card(
        &dance_graveyard_card(
            72_969,
            "Opponent's Relic",
            vec![CardType::Artifact],
            Vec::new(),
            2,
        ),
        bob,
        Zone::Graveyard,
    );
    let battlefield_artifact = game.create_object_from_card(
        &dance_graveyard_card(
            72_965,
            "Battlefield Relic",
            vec![CardType::Artifact],
            Vec::new(),
            2,
        ),
        alice,
        Zone::Battlefield,
    );

    let effects = game
        .object(spell_id)
        .and_then(|object| object.spell_effect.as_deref())
        .expect("Dance of the Manse should have spell effects");
    let requirements = extract_target_requirements(&game, effects, alice, Some(spell_id));
    assert_eq!(
        requirements.len(),
        1,
        "Dance should have one target requirement"
    );
    let legal_targets = &requirements[0].legal_targets;
    assert!(
        legal_targets.contains(&Target::Object(artifact)),
        "artifact should be legal"
    );
    assert!(
        legal_targets.contains(&Target::Object(enchantment)),
        "non-Aura enchantment should be legal"
    );
    assert!(
        !legal_targets.contains(&Target::Object(aura)),
        "Aura enchantment should not be legal"
    );
    assert!(
        !legal_targets.contains(&Target::Object(too_expensive)),
        "mana value greater than X should not be legal"
    );
    assert!(
        !legal_targets.contains(&Target::Object(opponents_artifact)),
        "artifact in an opponent's graveyard should not be legal"
    );
    assert!(
        !legal_targets.contains(&Target::Object(battlefield_artifact)),
        "battlefield artifact should not be legal"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn dance_of_the_manse_x_six_returns_and_animates_targets() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let spell = dance_of_the_manse_definition();
    let spell_id = game.create_object_from_definition(&spell, alice, Zone::Stack);
    let artifact = game.create_object_from_card(
        &dance_graveyard_card(
            72_966,
            "Animated Relic",
            vec![CardType::Artifact],
            Vec::new(),
            6,
        ),
        alice,
        Zone::Graveyard,
    );
    let enchantment = game.create_object_from_card(
        &dance_graveyard_card(
            72_967,
            "Animated Sigil",
            vec![CardType::Enchantment],
            Vec::new(),
            3,
        ),
        alice,
        Zone::Graveyard,
    );
    let artifact_stable = game.object(artifact).expect("artifact exists").stable_id;
    let enchantment_stable = game
        .object(enchantment)
        .expect("enchantment exists")
        .stable_id;
    game.object_mut(spell_id).unwrap().x_value = Some(6);
    let target_assignment = {
        let effects = game
            .object(spell_id)
            .and_then(|object| object.spell_effect.as_deref())
            .expect("Dance of the Manse should have spell effects");
        let requirements = extract_target_requirements(&game, effects, alice, Some(spell_id));
        assert_eq!(
            requirements.len(),
            1,
            "Dance should have one target requirement"
        );
        crate::game_state::TargetAssignment {
            spec: requirements[0].spec.clone(),
            range: 0..2,
        }
    };

    game.push_to_stack(
        StackEntry::new(spell_id, alice)
            .with_x(6)
            .with_targets(vec![Target::Object(artifact), Target::Object(enchantment)])
            .with_target_assignments(vec![target_assignment]),
    );
    resolve_stack_entry(&mut game).expect("Dance of the Manse should resolve");

    for stable_id in [artifact_stable, enchantment_stable] {
        let returned = game
            .find_object_by_stable_id(stable_id)
            .expect("returned object should still exist");
        assert!(
            game.battlefield.contains(&returned),
            "target should return to battlefield"
        );
        let card_types = game
            .current_card_types(returned)
            .expect("returned permanent should have card types");
        assert!(
            card_types.contains(&CardType::Creature),
            "returned permanent should become a creature, got {card_types:?}"
        );
        assert_eq!(game.calculated_power(returned), Some(4));
        assert_eq!(game.calculated_toughness(returned), Some(4));
    }

    let artifact_id = game.find_object_by_stable_id(artifact_stable).unwrap();
    let enchantment_id = game.find_object_by_stable_id(enchantment_stable).unwrap();
    assert!(
        game.current_card_types(artifact_id)
            .unwrap()
            .contains(&CardType::Artifact),
        "animated artifact should keep artifact type"
    );
    assert!(
        game.current_card_types(enchantment_id)
            .unwrap()
            .contains(&CardType::Enchantment),
        "animated enchantment should keep enchantment type"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn dance_of_the_manse_x_five_returns_without_animation() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let spell = dance_of_the_manse_definition();
    let spell_id = game.create_object_from_definition(&spell, alice, Zone::Stack);
    let artifact = game.create_object_from_card(
        &dance_graveyard_card(
            72_968,
            "Small Relic",
            vec![CardType::Artifact],
            Vec::new(),
            5,
        ),
        alice,
        Zone::Graveyard,
    );
    let artifact_stable = game.object(artifact).expect("artifact exists").stable_id;
    game.object_mut(spell_id).unwrap().x_value = Some(5);
    let target_assignment = {
        let effects = game
            .object(spell_id)
            .and_then(|object| object.spell_effect.as_deref())
            .expect("Dance of the Manse should have spell effects");
        let requirements = extract_target_requirements(&game, effects, alice, Some(spell_id));
        assert_eq!(
            requirements.len(),
            1,
            "Dance should have one target requirement"
        );
        crate::game_state::TargetAssignment {
            spec: requirements[0].spec.clone(),
            range: 0..1,
        }
    };

    game.push_to_stack(
        StackEntry::new(spell_id, alice)
            .with_x(5)
            .with_targets(vec![Target::Object(artifact)])
            .with_target_assignments(vec![target_assignment]),
    );
    resolve_stack_entry(&mut game).expect("Dance of the Manse should resolve");

    let returned = game
        .find_object_by_stable_id(artifact_stable)
        .expect("returned artifact should still exist");
    assert!(
        game.battlefield.contains(&returned),
        "target should return to battlefield"
    );
    let card_types = game
        .current_card_types(returned)
        .expect("returned artifact should have card types");
    assert!(card_types.contains(&CardType::Artifact));
    assert!(
        !card_types.contains(&CardType::Creature),
        "X less than 6 should not animate the returned artifact, got {card_types:?}"
    );
    assert_eq!(game.calculated_power(returned), None);
    assert_eq!(game.calculated_toughness(returned), None);
}
