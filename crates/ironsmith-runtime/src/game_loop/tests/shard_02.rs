#![allow(unused_imports)]
use super::shard_00::*;
use super::shard_01::*;
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

#[test]
pub(super) fn lethal_damage_with_two_regeneration_shields_resolves_without_looping() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let creature = create_creature(&mut game, "Shielded Skeleton", alice, 1, 1);

    let regenerate = crate::effects::RegenerateEffect::new(
        crate::target::ChooseSpec::SpecificObject(creature),
        crate::effect::Until::EndOfTurn,
    );
    let mut ctx = crate::effects::ExecutionContext::new_default(creature, alice);
    for _ in 0..2 {
        crate::effects::execute_effect(
            &mut game,
            &crate::effect::Effect::new(regenerate.clone()),
            &mut ctx,
        )
        .expect("regeneration shield should apply");
    }

    game.mark_damage(creature, 3);
    let mut trigger_queue = TriggerQueue::new();
    check_and_apply_sbas(&mut game, &mut trigger_queue).expect("SBA processing should terminate");

    let object = game.object(creature).expect("creature should survive");
    assert_eq!(
        object.zone,
        Zone::Battlefield,
        "regeneration should save it"
    );
    assert!(game.is_tapped(creature), "regeneration taps the creature");
    assert_eq!(
        game.damage_on(creature),
        0,
        "regeneration clears marked damage"
    );

    game.mark_damage(creature, 3);
    check_and_apply_sbas(&mut game, &mut trigger_queue).expect("SBA processing should terminate");
    let object = game
        .object(creature)
        .expect("creature should survive twice");
    assert_eq!(object.zone, Zone::Battlefield);

    game.mark_damage(creature, 3);
    check_and_apply_sbas(&mut game, &mut trigger_queue).expect("SBA processing should terminate");
    assert!(
        game.object(creature)
            .is_none_or(|object| object.zone == Zone::Graveyard),
        "with no shields left, lethal damage should destroy the creature"
    );
}

#[test]
pub(super) fn regeneration_count_tracks_used_shields_until_cleanup() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let creature = create_creature(&mut game, "Regeneration Probe", alice, 1, 1);

    assert_eq!(game.regenerated_this_turn_count(creature), 0);
    assert!(!game.use_regeneration_shield(creature));
    assert_eq!(
        game.regenerated_this_turn_count(creature),
        0,
        "failed regeneration attempts must not count"
    );

    game.add_regeneration_shield(creature, 2);
    assert!(game.use_regeneration_shield(creature));
    assert!(game.use_regeneration_shield(creature));
    assert_eq!(game.regeneration_shield_count(creature), 0);
    assert_eq!(game.regenerated_this_turn_count(creature), 2);

    execute_cleanup_step(&mut game);
    assert_eq!(
        game.regenerated_this_turn_count(creature),
        0,
        "regenerated-this-turn counts expire during cleanup"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn debt_of_loyalty_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(74_542), "Debt of Loyalty")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::White],
            vec![ManaSymbol::White],
        ]))
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Regenerate target creature. You gain control of that creature if it regenerates this way.",
        )
        .expect("Debt of Loyalty should parse strictly")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn resolve_debt_of_loyalty_on_target(
    game: &mut GameState,
    source: ObjectId,
    controller: PlayerId,
    target: ObjectId,
) {
    let debt = debt_of_loyalty_definition();
    let [effect] = debt
        .spell_effect
        .as_ref()
        .expect("Debt of Loyalty should have a spell effect")
        .flattened_default_effects()
    else {
        panic!("Debt of Loyalty should lower to one regenerate effect");
    };
    let mut ctx = crate::effects::ExecutionContext::new_default(source, controller)
        .with_targets(vec![crate::effects::ResolvedTarget::Object(target)])
        .with_target_assignments(vec![crate::game_state::TargetAssignment {
            spec: crate::target::ChooseSpec::target(crate::target::ChooseSpec::creature()),
            range: 0..1,
        }]);
    crate::effects::execute_effect(game, effect, &mut ctx)
        .expect("Debt of Loyalty should create a regeneration shield");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn debt_of_loyalty_gains_control_when_target_regenerates_this_way() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source_card = CardBuilder::new(CardId::from_raw(74_543), "Debt of Loyalty")
        .card_types(vec![CardType::Instant])
        .build();
    let source = game.create_object_from_card(&source_card, alice, Zone::Stack);
    let creature = create_creature(&mut game, "Debt Target", bob, 2, 2);

    resolve_debt_of_loyalty_on_target(&mut game, source, alice, creature);
    assert_eq!(
        game.controller_of_id(creature),
        Some(bob),
        "Debt of Loyalty should not change control before the shield is used"
    );

    let mut dm = SelectFirstDecisionMaker;
    let outcome =
        crate::events::processing::process_destroy(&mut game, creature, Some(source), &mut dm);

    assert!(
        matches!(outcome, crate::events::processing::EventOutcome::Replaced),
        "the destroy event should be replaced by the regeneration shield, got {outcome:?}"
    );
    assert!(
        game.battlefield.contains(&creature),
        "the regenerated creature should remain on the battlefield"
    );
    assert!(
        game.is_tapped(creature),
        "the regenerated creature should be tapped by regeneration"
    );
    assert_eq!(
        game.controller_of_id(creature),
        Some(alice),
        "Debt of Loyalty should gain control only after the target regenerates this way"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn debt_of_loyalty_does_not_gain_control_if_regeneration_shield_is_unused() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source_card = CardBuilder::new(CardId::from_raw(74_544), "Debt of Loyalty")
        .card_types(vec![CardType::Instant])
        .build();
    let source = game.create_object_from_card(&source_card, alice, Zone::Stack);
    let creature = create_creature(&mut game, "Debt Target", bob, 2, 2);

    resolve_debt_of_loyalty_on_target(&mut game, source, alice, creature);

    assert_eq!(
        game.controller_of_id(creature),
        Some(bob),
        "Debt of Loyalty's control change is conditional on the target actually regenerating"
    );
    assert!(
        game.battlefield.contains(&creature),
        "the target should remain on the battlefield while the shield is unused"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn debt_of_loyalty_shield_is_not_deduplicated_with_plain_regeneration() {
    struct ChooseLastReplacementDecisionMaker {
        replacement_choices: usize,
    }

    impl DecisionMaker for ChooseLastReplacementDecisionMaker {
        fn decide_options(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            if ctx.description == "Choose which replacement effect to apply" {
                self.replacement_choices += 1;
                return ctx
                    .options
                    .iter()
                    .rev()
                    .find(|option| option.legal)
                    .map(|option| vec![option.index])
                    .unwrap_or_default();
            }
            ctx.options
                .iter()
                .filter(|option| option.legal)
                .map(|option| option.index)
                .take(ctx.min)
                .collect()
        }
    }

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let plain_source_card = CardBuilder::new(CardId::from_raw(74_545), "Plain Regeneration")
        .card_types(vec![CardType::Instant])
        .build();
    let plain_source = game.create_object_from_card(&plain_source_card, alice, Zone::Stack);
    let debt_source_card = CardBuilder::new(CardId::from_raw(74_546), "Debt of Loyalty")
        .card_types(vec![CardType::Instant])
        .build();
    let debt_source = game.create_object_from_card(&debt_source_card, alice, Zone::Stack);
    let creature = create_creature(&mut game, "Debt Target", bob, 2, 2);

    let plain_regeneration = Effect::regenerate(
        crate::target::ChooseSpec::SpecificObject(creature),
        Until::EndOfTurn,
    );
    let mut plain_ctx = crate::effects::ExecutionContext::new_default(plain_source, alice);
    crate::effects::execute_effect(&mut game, &plain_regeneration, &mut plain_ctx)
        .expect("plain regeneration should create the first shield");
    resolve_debt_of_loyalty_on_target(&mut game, debt_source, alice, creature);

    let mut dm = ChooseLastReplacementDecisionMaker {
        replacement_choices: 0,
    };
    let outcome = crate::events::processing::process_destroy(
        &mut game,
        creature,
        Some(plain_source),
        &mut dm,
    );

    assert!(
        matches!(outcome, crate::events::processing::EventOutcome::Replaced),
        "the destroy event should be replaced by a regeneration shield, got {outcome:?}"
    );
    assert_eq!(
        dm.replacement_choices, 1,
        "non-equivalent regeneration shields must be offered as a replacement choice"
    );
    assert_eq!(
        game.controller_of_id(creature),
        Some(alice),
        "choosing the Debt of Loyalty shield should run its control-change follow-up"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn sublime_archangel_grants_real_exalted_triggers_to_other_creatures() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let sublime = CardDefinitionBuilder::new(CardId::from_raw(72_001), "Sublime Archangel")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(4, 3))
        .parse_text("Flying\nExalted\nOther creatures you control have exalted.")
        .expect("Sublime Archangel should parse");
    game.create_object_from_definition(&sublime, alice, Zone::Battlefield);

    let attacker = CardBuilder::new(CardId::from_raw(72_002), "Llanowar Elves")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let attacker_id = game.create_object_from_card(&attacker, alice, Zone::Battlefield);
    game.remove_summoning_sickness(attacker_id);

    for id in [72_003, 72_004] {
        let creature = CardBuilder::new(CardId::from_raw(id), "Elite Vanguard")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 1))
            .build();
        game.create_object_from_card(&creature, alice, Zone::Battlefield);
    }

    game.turn.active_player = alice;
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(crate::game_state::Step::DeclareAttackers);

    let mut combat = CombatState::default();
    let mut trigger_queue = TriggerQueue::new();
    let declarations = vec![AttackerDeclaration {
        creature: attacker_id,
        target: AttackTarget::Player(bob),
    }];

    apply_attacker_declarations(&mut game, &mut combat, &mut trigger_queue, &declarations)
        .expect("single attacker declaration should be legal");

    assert_eq!(
        trigger_queue.entries.len(),
        4,
        "Sublime plus the three other creatures should each contribute an exalted trigger"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn frenzy_instances_trigger_separately_only_for_unblocked_attackers() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let frenzy_sliver = CardDefinitionBuilder::new(CardId::from_raw(72_020), "Frenzy Sliver")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Sliver])
        .power_toughness(PowerToughness::fixed(1, 1))
        .parse_text("All Sliver creatures have frenzy 1.")
        .expect("Frenzy Sliver should parse");
    game.create_object_from_definition(&frenzy_sliver, alice, Zone::Battlefield);
    game.create_object_from_definition(&frenzy_sliver, alice, Zone::Battlefield);

    let make_attacker = |game: &mut GameState, id, name| {
        let card = CardBuilder::new(CardId::from_raw(id), name)
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Sliver])
            .power_toughness(PowerToughness::fixed(1, 1))
            .build();
        let object = game.create_object_from_card(&card, alice, Zone::Battlefield);
        game.remove_summoning_sickness(object);
        object
    };
    let blocked = make_attacker(&mut game, 72_021, "Blocked Sliver");
    let unblocked = make_attacker(&mut game, 72_022, "Unblocked Sliver");
    let blocker = create_creature(&mut game, "Blocker", bob, 1, 1);

    game.turn.active_player = alice;
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(crate::game_state::Step::DeclareAttackers);
    let mut combat = CombatState::default();
    let mut trigger_queue = TriggerQueue::new();
    apply_attacker_declarations(
        &mut game,
        &mut combat,
        &mut trigger_queue,
        &[
            AttackerDeclaration {
                creature: blocked,
                target: AttackTarget::Player(bob),
            },
            AttackerDeclaration {
                creature: unblocked,
                target: AttackTarget::Player(bob),
            },
        ],
    )
    .expect("both Slivers should attack");
    apply_blocker_declarations(
        &mut game,
        &mut combat,
        &mut trigger_queue,
        &[BlockerDeclaration {
            blocker,
            blocking: blocked,
        }],
        bob,
    )
    .expect("one Sliver should be blocked");

    assert_eq!(
        trigger_queue.entries.len(),
        2,
        "two independently granted Frenzy instances should trigger for only the unblocked Sliver"
    );
    put_triggers_on_stack(&mut game, &mut trigger_queue).expect("Frenzy triggers should stack");
    while !game.stack_is_empty() {
        resolve_stack_entry(&mut game).expect("Frenzy trigger should resolve");
    }
    game.refresh_continuous_state();

    assert_eq!(game.calculated_power(blocked), Some(1));
    assert_eq!(game.calculated_power(unblocked), Some(3));
    assert_eq!(game.calculated_toughness(unblocked), Some(1));

    execute_cleanup_step(&mut game);
    game.refresh_continuous_state();
    assert_eq!(
        game.calculated_power(unblocked),
        Some(1),
        "each Frenzy bonus should expire at end of turn"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn poisonous_instances_trigger_separately_only_for_combat_damage_to_a_player() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let virulent = CardDefinitionBuilder::new(CardId::from_raw(72_030), "Virulent Sliver")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Sliver])
        .power_toughness(PowerToughness::fixed(1, 1))
        .parse_text("All Sliver creatures have poisonous 1.")
        .expect("Virulent Sliver should parse");
    game.create_object_from_definition(&virulent, alice, Zone::Battlefield);
    game.create_object_from_definition(&virulent, alice, Zone::Battlefield);

    let attacker = CardBuilder::new(CardId::from_raw(72_031), "Attacking Sliver")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Sliver])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let attacker = game.create_object_from_card(&attacker, alice, Zone::Battlefield);
    let damaged_creature = create_creature(&mut game, "Damaged Creature", bob, 1, 1);

    let event = |target, combat, cause| {
        TriggerEvent::new_with_provenance(
            crate::events::DamageEvent::with_cause(attacker, target, 1, combat, cause),
            crate::provenance::ProvNodeId::default(),
        )
    };
    let noncombat = event(
        crate::events::DamageTarget::Player(bob),
        false,
        crate::events::cause::EventCause::effect(),
    );
    let creature_combat = event(
        crate::events::DamageTarget::Object(damaged_creature),
        true,
        crate::events::cause::EventCause::combat_damage(attacker),
    );
    assert!(crate::triggers::check_triggers(&game, &noncombat).is_empty());
    assert!(crate::triggers::check_triggers(&game, &creature_combat).is_empty());

    let player_combat = event(
        crate::events::DamageTarget::Player(bob),
        true,
        crate::events::cause::EventCause::combat_damage(attacker),
    );
    let mut trigger_queue = TriggerQueue::new();
    for trigger in crate::triggers::check_triggers(&game, &player_combat) {
        trigger_queue.add(trigger);
    }
    assert_eq!(trigger_queue.entries.len(), 2);
    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("independent Poisonous triggers should stack");
    while !game.stack_is_empty() {
        resolve_stack_entry(&mut game).expect("Poisonous trigger should resolve");
    }
    assert_eq!(game.player(bob).expect("Bob exists").poison_counters, 2);

    let poison_shield = CardDefinitionBuilder::new(CardId::from_raw(72_032), "Poison Shield")
        .card_types(vec![CardType::Enchantment])
        .with_ability(Ability::static_ability(StaticAbility::restriction(
            crate::effect::Restriction::poison_counters(PlayerFilter::Specific(bob)),
            "Bob can't get poison counters".to_string(),
        )))
        .build();
    game.create_object_from_definition(&poison_shield, alice, Zone::Battlefield);
    game.refresh_continuous_state();
    for trigger in crate::triggers::check_triggers(&game, &player_combat) {
        trigger_queue.add(trigger);
    }
    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("prohibited Poisonous triggers should still stack");
    while !game.stack_is_empty() {
        resolve_stack_entry(&mut game).expect("prohibited Poisonous trigger should resolve");
    }
    assert_eq!(
        game.player(bob).expect("Bob exists").poison_counters,
        2,
        "a player who can't get poison counters should receive none"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn transfigure_activation_sacrifices_source_and_searches_by_its_lki_mana_value() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    game.turn.active_player = alice;
    game.turn.phase = Phase::FirstMain;

    let source_definition =
        CardDefinitionBuilder::new(CardId::from_raw(54_001), "LKI Transfigurer")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(4)]]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(3, 3))
            .parse_text("Transfigure {0}")
            .expect("Transfigure should parse");
    let source = game.create_object_from_definition(&source_definition, alice, Zone::Battlefield);
    game.turn.phase = Phase::Combat;
    assert!(
        crate::decision::compute_legal_actions(&game, alice)
            .iter()
            .all(|action| !matches!(
                action,
                crate::decision::LegalAction::ActivateAbility {
                    source: action_source,
                    ..
                } if *action_source == source
            )),
        "Transfigure cannot be activated outside sorcery timing"
    );
    game.turn.phase = Phase::FirstMain;

    let matching = CardDefinitionBuilder::new(CardId::from_raw(54_002), "Matching Four")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(4)]]))
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(4, 4))
        .build();
    game.create_object_from_definition(&matching, alice, Zone::Library);
    let wrong_value = CardDefinitionBuilder::new(CardId::from_raw(54_003), "Wrong Three")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(3)]]))
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 3))
        .build();
    game.create_object_from_definition(&wrong_value, alice, Zone::Library);
    let wrong_type = CardDefinitionBuilder::new(CardId::from_raw(54_004), "Wrong Relic")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(4)]]))
        .card_types(vec![CardType::Artifact])
        .build();
    game.create_object_from_definition(&wrong_type, alice, Zone::Library);

    let action = crate::decision::compute_legal_actions(&game, alice)
        .into_iter()
        .find(|action| {
            matches!(
                action,
                crate::decision::LegalAction::ActivateAbility {
                    source: action_source,
                    ..
                } if *action_source == source
            )
        })
        .expect("Transfigure should be legal in the active player's main phase");
    let mut trigger_queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = SelectFirstDecisionMaker;
    let mut progress = apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        &mut dm,
    )
    .expect("Transfigure activation should begin");

    for _ in 0..4 {
        if game.stack.len() == 1 {
            break;
        }

        progress = match progress {
            crate::decision::GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::SelectOptions(ctx),
            ) => {
                let option = ctx
                    .options
                    .iter()
                    .find(|option| {
                        option
                            .description
                            .to_ascii_lowercase()
                            .contains("sacrifice")
                    })
                    .unwrap_or_else(|| {
                        panic!("expected the Transfigure sacrifice cost, got {ctx:?}")
                    });
                apply_priority_response_with_dm(
                    &mut game,
                    &mut trigger_queue,
                    &mut state,
                    &PriorityResponse::NextCostChoice(option.index),
                    &mut dm,
                )
                .expect("choosing the Transfigure sacrifice cost should continue activation")
            }
            crate::decision::GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::SelectObjects(_),
            ) => apply_priority_response_with_dm(
                &mut game,
                &mut trigger_queue,
                &mut state,
                &PriorityResponse::SacrificeTarget(source),
                &mut dm,
            )
            .expect("selecting the Transfigure source should pay its sacrifice cost"),
            other => panic!("Transfigure activation did not advance through costs: {other:?}"),
        };
    }

    assert!(
        game.object(source)
            .is_none_or(|object| object.zone == Zone::Graveyard),
        "the source is sacrificed while paying the activation cost"
    );
    assert_eq!(
        game.stack.len(),
        1,
        "the ability should remain on the stack"
    );

    resolve_stack_entry_with_dm_and_triggers(&mut game, &mut dm, &mut trigger_queue)
        .expect("Transfigure should resolve");

    assert!(game.battlefield.iter().any(|object| {
        game.object(*object)
            .is_some_and(|object| object.name == "Matching Four")
    }));
    assert!(!game.battlefield.iter().any(|object| {
        game.object(*object)
            .is_some_and(|object| object.name == "Wrong Three" || object.name == "Wrong Relic")
    }));
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn glamdring_equipped_creature_gets_first_strike_and_graveyard_scaled_power() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let glamdring = CardDefinitionBuilder::new(CardId::from_raw(72_101), "Glamdring")
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "Equipped creature has first strike and gets +1/+0 for each instant and sorcery card in your graveyard.\nWhenever equipped creature deals combat damage to a player, you may cast an instant or sorcery spell from your hand with mana value less than or equal to that damage without paying its mana cost.\nEquip {3}",
        )
        .expect("Glamdring should parse");
    let glamdring_id = game.create_object_from_definition(&glamdring, alice, Zone::Battlefield);

    let attacker_id = create_creature(&mut game, "Bearer", alice, 2, 2);
    game.remove_summoning_sickness(attacker_id);

    let instant = CardBuilder::new(CardId::from_raw(72_102), "Shock")
        .card_types(vec![CardType::Instant])
        .build();
    game.create_object_from_card(&instant, alice, Zone::Graveyard);
    let sorcery = CardBuilder::new(CardId::from_raw(72_103), "Ponder")
        .card_types(vec![CardType::Sorcery])
        .build();
    game.create_object_from_card(&sorcery, alice, Zone::Graveyard);

    if let Some(equipment) = game.object_mut(glamdring_id) {
        equipment.attached_to = Some(crate::object::AttachmentTarget::Object(attacker_id));
    }
    if let Some(attacker) = game.object_mut(attacker_id) {
        attacker.attachments.push(glamdring_id);
    }

    assert_eq!(game.calculated_power(attacker_id), Some(4));

    let mut combat = CombatState::default();
    combat.attackers.push(crate::combat_state::AttackerInfo {
        creature: attacker_id,
        target: AttackTarget::Player(bob),
    });
    combat.blockers.insert(attacker_id, Vec::new());

    let first_strike_events = execute_combat_damage_step(&mut game, &combat, true);
    assert_eq!(
        first_strike_events.len(),
        1,
        "equipped Bearer should hit in first-strike step"
    );
    assert_eq!(
        game.player(bob).unwrap().life,
        16,
        "Bearer should deal 4 first-strike damage"
    );

    let regular_events = execute_combat_damage_step(&mut game, &combat, false);
    assert_eq!(
        regular_events.len(),
        0,
        "first strike should suppress regular damage step hit"
    );

    if let Some(equipment) = game.object_mut(glamdring_id) {
        equipment.attached_to = None;
    }
    if let Some(attacker) = game.object_mut(attacker_id) {
        attacker.attachments.clear();
    }

    assert_eq!(game.calculated_power(attacker_id), Some(2));
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn spider_man_no_more_attached_creature_becomes_citizen_defender_only() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let host = CardDefinitionBuilder::new(CardId::from_raw(72_120), "Heroic Bearer")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Human, Subtype::Advisor])
        .power_toughness(PowerToughness::fixed(4, 4))
        .parse_text("Flying\nTrample")
        .expect("host creature should parse");
    let aura = CardDefinitionBuilder::new(CardId::from_raw(72_121), "Spider-Man No More")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Aura])
        .parse_text(
            "Enchant creature\nEnchanted creature is a Citizen with base power and toughness 1/1. It has defender and loses all other abilities.",
        )
        .expect("Spider-Man No More should parse");

    let host_id = game.create_object_from_definition(&host, alice, Zone::Battlefield);
    let aura_id = game.create_object_from_definition(&aura, alice, Zone::Battlefield);
    if let Some(aura_object) = game.object_mut(aura_id) {
        aura_object.attached_to = Some(crate::object::AttachmentTarget::Object(host_id));
    }
    if let Some(host_object) = game.object_mut(host_id) {
        host_object.attachments.push(aura_id);
    }

    let characteristics = game
        .calculated_characteristics(host_id)
        .expect("enchanted creature should have calculated characteristics");
    assert_eq!(
        (characteristics.power, characteristics.toughness),
        (Some(1), Some(1)),
        "Spider-Man No More should set the enchanted creature's base power/toughness to 1/1"
    );
    assert!(
        characteristics.subtypes.contains(&Subtype::Citizen),
        "Spider-Man No More should make the enchanted creature a Citizen, got {:?}",
        characteristics.subtypes
    );
    assert!(
        !characteristics.subtypes.contains(&Subtype::Human)
            && !characteristics.subtypes.contains(&Subtype::Advisor),
        "Spider-Man No More should remove other creature types, got {:?}",
        characteristics.subtypes
    );
    assert!(
        game.object_has_ability(host_id, &StaticAbility::defender()),
        "Spider-Man No More should grant defender after removing other abilities"
    );
    assert!(
        !game.object_has_ability(host_id, &StaticAbility::flying())
            && !game.object_has_ability(host_id, &StaticAbility::trample()),
        "Spider-Man No More should remove the enchanted creature's previous abilities"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn frontier_warmonger_grants_menace_only_to_qualifying_attackers_until_cleanup() {
    let mut game = setup_three_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);

    let warmonger = CardDefinitionBuilder::new(CardId::from_raw(72_140), "Frontier Warmonger")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(4, 4))
        .parse_text("Whenever one or more creatures attack one of your opponents or a planeswalker they control, those creatures gain menace until end of turn.")
        .expect("Frontier Warmonger should parse");
    game.create_object_from_definition(&warmonger, alice, Zone::Battlefield);

    let legal_attacker_id = create_creature(&mut game, "Legal Attacker", alice, 2, 2);
    game.remove_summoning_sickness(legal_attacker_id);
    let illegal_attacker_id = create_creature(&mut game, "Illegal Attacker", bob, 2, 2);
    game.remove_summoning_sickness(illegal_attacker_id);
    let home_creature_id = create_creature(&mut game, "Home Creature", alice, 2, 2);

    assert!(
        !game.object_has_ability(legal_attacker_id, &StaticAbility::menace()),
        "attacker should not start with menace"
    );

    game.turn.active_player = bob;
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(crate::game_state::Step::DeclareAttackers);

    let mut combat = CombatState::default();
    let mut trigger_queue = TriggerQueue::new();
    let declarations = vec![AttackerDeclaration {
        creature: illegal_attacker_id,
        target: AttackTarget::Player(alice),
    }];

    apply_attacker_declarations(&mut game, &mut combat, &mut trigger_queue, &declarations)
        .expect("attacker declaration should be legal");
    put_triggers_on_stack(&mut game, &mut trigger_queue).expect("trigger should go on stack");
    assert!(
        game.stack.is_empty(),
        "attacking you should not create a Frontier Warmonger trigger"
    );

    assert!(
        !game.object_has_ability(illegal_attacker_id, &StaticAbility::menace()),
        "attacker that attacked you should not gain menace"
    );

    game.turn.active_player = alice;
    let mut combat = CombatState::default();
    let mut trigger_queue = TriggerQueue::new();
    let declarations = vec![AttackerDeclaration {
        creature: legal_attacker_id,
        target: AttackTarget::Player(charlie),
    }];
    apply_attacker_declarations(&mut game, &mut combat, &mut trigger_queue, &declarations)
        .expect("attacker declaration should be legal");
    put_triggers_on_stack(&mut game, &mut trigger_queue).expect("trigger should go on stack");
    resolve_stack_entry(&mut game).expect("trigger should resolve");

    assert!(
        game.object_has_ability(legal_attacker_id, &StaticAbility::menace()),
        "attacking one of your opponents should grant menace"
    );
    assert!(
        !game.object_has_ability(home_creature_id, &StaticAbility::menace()),
        "nonattacking creature should not gain menace"
    );

    execute_cleanup_step(&mut game);
    assert!(
        !game.object_has_ability(legal_attacker_id, &StaticAbility::menace()),
        "temporary menace should end at cleanup"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn party_dude_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(72_144), "Party Dude")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Class])
        .parse_text(
            "(Gain the next level as a sorcery to add its ability.)\n\
             When this Class enters, each player creates a Food token.\n\
             {1}{G}: Level 2\n\
             Whenever an artifact an opponent controls is put into a graveyard from the battlefield, draw a card.\n\
             {4}{G}: Level 3\n\
             Whenever one or more of your opponents are attacked, up to one target attacking creature gets +X/+X until end of turn, where X is the number of cards in your hand.",
        )
        .expect("Party Dude should parse strictly")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn put_cards_in_hand(game: &mut GameState, player: PlayerId, count: u32) {
    for index in 0..count {
        let card = CardBuilder::new(CardId::from_raw(72_200 + index), "Hand Filler").build();
        game.create_object_from_card(&card, player, Zone::Hand);
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn put_party_dude_at_level(game: &mut GameState, party_dude_id: ObjectId, level: u32) {
    let counters = level.saturating_sub(1);
    if counters > 0 {
        game.add_counters(party_dude_id, crate::object::CounterType::Level, counters);
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn party_dude_level_ability_index(
    game: &GameState,
    party_dude_id: ObjectId,
    level: u32,
) -> usize {
    game.object(party_dude_id)
        .expect("Party Dude should be on the battlefield")
        .abilities
        .iter()
        .enumerate()
        .find_map(|(index, ability)| match &ability.kind {
            AbilityKind::Activated(activated)
                if activated.additional_restrictions.iter().any(|restriction| {
                    restriction == &format!("__ironsmith_class_level:{level}")
                }) =>
            {
                Some(index)
            }
            _ => None,
        })
        .expect("Party Dude should have the requested class level ability")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn party_dude_can_activate_level(
    game: &GameState,
    party_dude_id: ObjectId,
    level: u32,
) -> bool {
    let ability_index = party_dude_level_ability_index(game, party_dude_id, level);
    crate::decision::compute_legal_actions(game, PlayerId::from_index(0))
        .iter()
        .any(|action| {
            matches!(
                action,
                crate::decision::LegalAction::ActivateAbility { source, ability_index: idx }
                    if *source == party_dude_id && *idx == ability_index
            )
        })
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn case_of_the_shattered_pact_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(72_300), "Case of the Shattered Pact")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]]))
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "When this Case enters, search your library for a basic land card, reveal it, put it into your hand, then shuffle.\n\
             To solve — There are five colors among permanents you control. (If unsolved, solve at the beginning of your end step.)\n\
             Solved — At the beginning of combat on your turn, target creature you control gains flying, double strike, and vigilance until end of turn.",
        )
        .expect("Case of the Shattered Pact should parse strictly")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn case_of_the_shattered_pact_zone_change_event(case_id: ObjectId) -> TriggerEvent {
    TriggerEvent::new_with_provenance(
        crate::events::ZoneChangeEvent::with_cause(
            case_id,
            Zone::Stack,
            Zone::Battlefield,
            crate::events::cause::EventCause::from_game_rule(),
            None,
        ),
        crate::provenance::ProvNodeId::default(),
    )
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn case_of_the_shattered_pact_end_step_event(player: PlayerId) -> TriggerEvent {
    TriggerEvent::new_with_provenance(
        crate::events::phase::BeginningOfEndStepEvent::new(player),
        crate::provenance::ProvNodeId::default(),
    )
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn case_of_the_shattered_pact_combat_event(player: PlayerId) -> TriggerEvent {
    TriggerEvent::new_with_provenance(
        crate::events::phase::BeginningOfCombatEvent::new(player),
        crate::provenance::ProvNodeId::default(),
    )
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn create_case_color_permanent(
    game: &mut GameState,
    player: PlayerId,
    name: &str,
    colors: crate::color::ColorSet,
) {
    let card = CardBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Artifact])
        .color_indicator(colors)
        .build();
    game.create_object_from_card(&card, player, Zone::Battlefield);
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn create_five_case_colors(game: &mut GameState, player: PlayerId) {
    for (name, colors) in [
        ("Case White Permanent", crate::color::ColorSet::WHITE),
        ("Case Blue Permanent", crate::color::ColorSet::BLUE),
        ("Case Black Permanent", crate::color::ColorSet::BLACK),
        ("Case Red Permanent", crate::color::ColorSet::RED),
        ("Case Green Permanent", crate::color::ColorSet::GREEN),
    ] {
        create_case_color_permanent(game, player, name, colors);
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn case_of_the_shattered_pact_enters_searches_basic_land_into_hand() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let basic_land = CardBuilder::new(CardId::from_raw(72_301), "Case Search Basic")
        .card_types(vec![CardType::Land])
        .supertypes(vec![Supertype::Basic])
        .build();
    game.create_object_from_card(&basic_land, alice, Zone::Library);
    let nonland = CardBuilder::new(CardId::from_raw(72_302), "Case Search Nonland")
        .card_types(vec![CardType::Creature])
        .build();
    game.create_object_from_card(&nonland, alice, Zone::Library);
    let case = case_of_the_shattered_pact_definition();
    let case_id = game.create_object_from_definition(&case, alice, Zone::Battlefield);

    let event = case_of_the_shattered_pact_zone_change_event(case_id);
    let mut trigger_queue = TriggerQueue::new();
    for trigger in crate::triggers::check_triggers(&game, &event) {
        if trigger.source == case_id {
            trigger_queue.add(trigger);
        }
    }
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "Case should trigger when it enters"
    );

    let mut dm = SelectFirstDecisionMaker;
    put_triggers_on_stack_with_dm(&mut game, &mut trigger_queue, &mut dm)
        .expect("Case ETB trigger should go on the stack");
    resolve_stack_entry_with(&mut game, &mut dm).expect("Case ETB trigger should resolve");

    let basic_land_in_hand = game
        .player(alice)
        .expect("alice exists")
        .hand
        .iter()
        .any(|&id| {
            game.object(id).is_some_and(|object| {
                object.name == "Case Search Basic"
                    && object.card_types.contains(&CardType::Land)
                    && object.supertypes.contains(&Supertype::Basic)
            })
        });
    assert!(
        basic_land_in_hand,
        "Case should put the searched basic land into its controller's hand"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn case_of_the_shattered_pact_solves_only_with_five_colors() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let case = case_of_the_shattered_pact_definition();
    let case_id = game.create_object_from_definition(&case, alice, Zone::Battlefield);
    create_case_color_permanent(
        &mut game,
        alice,
        "Case White Permanent",
        crate::color::ColorSet::WHITE,
    );

    let event = case_of_the_shattered_pact_end_step_event(alice);
    assert!(
        crate::triggers::check_triggers(&game, &event)
            .into_iter()
            .all(|trigger| trigger.source != case_id),
        "Case should not solve with fewer than five colors among permanents you control"
    );
    assert!(!game.is_case_solved(case_id));

    create_five_case_colors(&mut game, alice);
    let mut trigger_queue = TriggerQueue::new();
    for trigger in crate::triggers::check_triggers(&game, &event) {
        if trigger.source == case_id {
            trigger_queue.add(trigger);
        }
    }
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "Case should solve at end step with five colors"
    );

    let mut dm = AutoPassDecisionMaker;
    put_triggers_on_stack_with_dm(&mut game, &mut trigger_queue, &mut dm)
        .expect("Case solve trigger should go on the stack");
    resolve_stack_entry_with(&mut game, &mut dm).expect("Case solve trigger should resolve");

    assert!(
        game.is_case_solved(case_id),
        "Case solving should mark the Case as solved"
    );
    assert!(
        !game
            .object(case_id)
            .expect("Case should exist")
            .counters
            .contains_key(&crate::object::CounterType::Level),
        "Case solving must not use level counters"
    );
    assert!(
        crate::triggers::check_triggers(&game, &event)
            .into_iter()
            .all(|trigger| trigger.source != case_id),
        "Case should not try to solve again once already solved"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn case_of_the_shattered_pact_solved_combat_trigger_is_gated_and_grants_keywords() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let case = case_of_the_shattered_pact_definition();
    let case_id = game.create_object_from_definition(&case, alice, Zone::Battlefield);
    let target = create_creature(&mut game, "Case Target Creature", alice, 2, 2);
    let event = case_of_the_shattered_pact_combat_event(alice);

    assert!(
        crate::triggers::check_triggers(&game, &event)
            .into_iter()
            .all(|trigger| trigger.source != case_id),
        "Case solved ability should not trigger before the Case is solved"
    );

    assert!(game.solve_case(case_id));
    let mut trigger_queue = TriggerQueue::new();
    for trigger in crate::triggers::check_triggers(&game, &event) {
        if trigger.source == case_id {
            trigger_queue.add(trigger);
        }
    }
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "Solved Case should trigger at combat"
    );

    let mut dm = SelectFirstDecisionMaker;
    put_triggers_on_stack_with_dm(&mut game, &mut trigger_queue, &mut dm)
        .expect("Solved Case combat trigger should go on the stack");
    resolve_stack_entry_with(&mut game, &mut dm)
        .expect("Solved Case combat trigger should resolve");
    game.refresh_continuous_state();

    assert!(
        game.object_has_ability(target, &StaticAbility::flying()),
        "target should gain flying"
    );
    assert!(
        game.object_has_ability(target, &StaticAbility::double_strike()),
        "target should gain double strike"
    );
    assert!(
        game.object_has_ability(target, &StaticAbility::vigilance()),
        "target should gain vigilance"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn party_dude_pumps_target_attacking_creature_when_an_opponent_is_attacked() {
    let mut game = setup_three_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);

    let party_dude = party_dude_definition();
    let party_dude_id = game.create_object_from_definition(&party_dude, alice, Zone::Battlefield);
    put_party_dude_at_level(&mut game, party_dude_id, 3);
    put_cards_in_hand(&mut game, alice, 3);
    let attacker = create_creature(&mut game, "Party Attacker", bob, 2, 2);
    game.remove_summoning_sickness(attacker);
    game.turn.active_player = bob;
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(crate::game_state::Step::DeclareAttackers);

    let mut combat = CombatState::default();
    let mut trigger_queue = TriggerQueue::new();
    apply_attacker_declarations(
        &mut game,
        &mut combat,
        &mut trigger_queue,
        &[AttackerDeclaration {
            creature: attacker,
            target: AttackTarget::Player(charlie),
        }],
    )
    .expect("Bob should be able to attack Charlie");
    game.combat = Some(combat.clone());
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "Party Dude should trigger once"
    );

    let mut dm = SelectFirstDecisionMaker;
    put_triggers_on_stack_with_dm(&mut game, &mut trigger_queue, &mut dm)
        .expect("Party Dude trigger should go on the stack with a target");
    resolve_stack_entry(&mut game).expect("Party Dude trigger should resolve");
    game.refresh_continuous_state();

    assert_eq!(
        game.calculated_power(attacker),
        Some(5),
        "Party Dude should give +X/+X where X is its controller's hand size"
    );
    assert_eq!(game.calculated_toughness(attacker), Some(5));
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn party_dude_does_not_trigger_before_level_three_when_an_opponent_is_attacked() {
    for level in [1, 2] {
        let mut game = setup_three_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let charlie = PlayerId::from_index(2);

        let party_dude = party_dude_definition();
        let party_dude_id =
            game.create_object_from_definition(&party_dude, alice, Zone::Battlefield);
        put_party_dude_at_level(&mut game, party_dude_id, level);
        let attacker = create_creature(&mut game, "Early Party Attacker", bob, 2, 2);
        game.remove_summoning_sickness(attacker);
        game.turn.active_player = bob;
        game.turn.phase = Phase::Combat;
        game.turn.step = Some(crate::game_state::Step::DeclareAttackers);

        let mut combat = CombatState::default();
        let mut trigger_queue = TriggerQueue::new();
        apply_attacker_declarations(
            &mut game,
            &mut combat,
            &mut trigger_queue,
            &[AttackerDeclaration {
                creature: attacker,
                target: AttackTarget::Player(charlie),
            }],
        )
        .expect("Bob should be able to attack Charlie");

        assert!(
            trigger_queue.entries.is_empty(),
            "Party Dude should not have its level 3 attack trigger at level {level}"
        );
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn party_dude_class_level_activations_are_level_gated() {
    let mut game = setup_three_player_game();
    let alice = PlayerId::from_index(0);
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;

    let party_dude = party_dude_definition();
    let party_dude_id = game.create_object_from_definition(&party_dude, alice, Zone::Battlefield);

    assert!(party_dude_can_activate_level(&game, party_dude_id, 2));
    assert!(!party_dude_can_activate_level(&game, party_dude_id, 3));

    game.add_counters(party_dude_id, crate::object::CounterType::Level, 1);
    assert!(!party_dude_can_activate_level(&game, party_dude_id, 2));
    assert!(party_dude_can_activate_level(&game, party_dude_id, 3));

    game.add_counters(party_dude_id, crate::object::CounterType::Level, 1);
    assert!(!party_dude_can_activate_level(&game, party_dude_id, 2));
    assert!(!party_dude_can_activate_level(&game, party_dude_id, 3));
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn party_dude_does_not_trigger_when_its_controller_is_attacked() {
    let mut game = setup_three_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let party_dude = party_dude_definition();
    let party_dude_id = game.create_object_from_definition(&party_dude, alice, Zone::Battlefield);
    put_party_dude_at_level(&mut game, party_dude_id, 3);
    let attacker = create_creature(&mut game, "Uninvited Attacker", bob, 2, 2);
    game.remove_summoning_sickness(attacker);
    game.turn.active_player = bob;
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(crate::game_state::Step::DeclareAttackers);

    let mut combat = CombatState::default();
    let mut trigger_queue = TriggerQueue::new();
    apply_attacker_declarations(
        &mut game,
        &mut combat,
        &mut trigger_queue,
        &[AttackerDeclaration {
            creature: attacker,
            target: AttackTarget::Player(alice),
        }],
    )
    .expect("Bob should be able to attack Alice");
    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("empty trigger queue should process");

    assert!(
        game.stack.is_empty(),
        "Party Dude should not trigger when its controller, not an opponent, is attacked"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn kargan_dragonlord_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(18_920), "Kargan Dragonlord")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Red],
            vec![ManaSymbol::Red],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Human, Subtype::Warrior])
        .power_toughness(PowerToughness::fixed(2, 2))
        .parse_text(
            "Level up {R}\n\
             LEVEL 4-7\n\
             4/4\n\
             Flying\n\
             LEVEL 8+\n\
             8/8\n\
             Flying, trample\n\
             {R}: This creature gets +1/+0 until end of turn.",
        )
        .expect("Kargan Dragonlord should parse strictly")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn kargan_pump_ability_index(game: &GameState, kargan_id: ObjectId) -> usize {
    game.object(kargan_id)
        .expect("Kargan Dragonlord should be on the battlefield")
        .abilities
        .iter()
        .enumerate()
        .find_map(|(index, ability)| match &ability.kind {
            AbilityKind::Activated(activated)
                if activated
                    .additional_restrictions
                    .iter()
                    .any(|restriction| restriction == "__ironsmith_level_range:8:+") =>
            {
                Some(index)
            }
            _ => None,
        })
        .expect("Kargan Dragonlord should have its level-8 pump ability")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn kargan_level_up_ability_index(game: &GameState, kargan_id: ObjectId) -> usize {
    game.object(kargan_id)
        .expect("Kargan Dragonlord should be on the battlefield")
        .abilities
        .iter()
        .enumerate()
        .find_map(|(index, ability)| match &ability.kind {
            AbilityKind::Activated(activated)
                if matches!(
                    activated.timing,
                    crate::ability::ActivationTiming::SorcerySpeed
                ) && activated
                    .effects
                    .flattened_default_effects()
                    .iter()
                    .any(|effect| {
                        effect
                            .downcast_ref::<crate::effects::PutCountersEffect>()
                            .is_some_and(|put| {
                                put.counter_type == crate::object::CounterType::Level
                            })
                    }) =>
            {
                Some(index)
            }
            _ => None,
        })
        .expect("Kargan Dragonlord should have its level-up ability")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn kargan_can_activate(
    game: &GameState,
    kargan_id: ObjectId,
    ability_index: usize,
) -> bool {
    crate::decision::compute_legal_actions(game, PlayerId::from_index(0))
        .iter()
        .any(|action| {
            matches!(
                action,
                crate::decision::LegalAction::ActivateAbility { source, ability_index: idx }
                    if *source == kargan_id && *idx == ability_index
            )
        })
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn activate_kargan_ability_and_resolve(
    game: &mut GameState,
    kargan_id: ObjectId,
    ability_index: usize,
) {
    let action = crate::decision::compute_legal_actions(game, PlayerId::from_index(0))
        .into_iter()
        .find(|action| {
            matches!(
                action,
                crate::decision::LegalAction::ActivateAbility { source, ability_index: idx }
                    if *source == kargan_id && *idx == ability_index
            )
        })
        .expect("Kargan Dragonlord activation should be legal");
    let mut trigger_queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = AutoPassDecisionMaker;
    apply_priority_response_with_dm(
        game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        &mut dm,
    )
    .expect("Kargan Dragonlord activation should start");
    resolve_stack_entry_with_dm_and_triggers(game, &mut dm, &mut trigger_queue)
        .expect("Kargan Dragonlord activation should resolve");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn kargan_dragonlord_level_up_adds_counter_and_uses_sorcery_timing() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;

    let kargan = kargan_dragonlord_definition();
    let kargan_id = game.create_object_from_definition(&kargan, alice, Zone::Battlefield);
    let level_up_index = kargan_level_up_ability_index(&game, kargan_id);
    game.player_mut(alice)
        .expect("Alice should exist")
        .mana_pool
        .add(ManaSymbol::Red, 1);

    assert!(
        kargan_can_activate(&game, kargan_id, level_up_index),
        "Kargan's level-up ability should be legal in its controller's main phase"
    );
    game.turn.phase = Phase::Combat;
    assert!(
        !kargan_can_activate(&game, kargan_id, level_up_index),
        "Kargan's level-up ability should not be legal outside sorcery timing"
    );

    game.turn.phase = Phase::FirstMain;
    activate_kargan_ability_and_resolve(&mut game, kargan_id, level_up_index);

    assert_eq!(
        game.object(kargan_id)
            .expect("Kargan should exist")
            .counters
            .get(&crate::object::CounterType::Level)
            .copied(),
        Some(1),
        "Kargan's level-up activation should put one level counter on itself"
    );
    assert_eq!(
        game.player(alice)
            .expect("Alice should exist")
            .mana_pool
            .red,
        0,
        "Kargan's level-up activation should spend its red mana cost"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn kargan_dragonlord_level_eight_pump_is_gated_and_expires() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;

    let kargan = kargan_dragonlord_definition();
    let kargan_id = game.create_object_from_definition(&kargan, alice, Zone::Battlefield);
    let pump_index = kargan_pump_ability_index(&game, kargan_id);
    game.add_counters(kargan_id, crate::object::CounterType::Level, 7);
    game.player_mut(alice)
        .expect("Alice should exist")
        .mana_pool
        .add(ManaSymbol::Red, 2);

    assert!(
        !kargan_can_activate(&game, kargan_id, pump_index),
        "Kargan's pump should not be legal before it has eight level counters"
    );
    game.add_counters(kargan_id, crate::object::CounterType::Level, 1);
    assert!(
        kargan_can_activate(&game, kargan_id, pump_index),
        "Kargan's pump should become legal at eight level counters"
    );

    activate_kargan_ability_and_resolve(&mut game, kargan_id, pump_index);
    game.refresh_continuous_state();

    assert_eq!(game.calculated_power(kargan_id), Some(9));
    assert_eq!(game.calculated_toughness(kargan_id), Some(8));
    assert!(
        game.object_has_ability(kargan_id, &StaticAbility::flying())
            && game.object_has_ability(kargan_id, &StaticAbility::trample()),
        "Kargan should have its level-eight flying and trample while pumped"
    );

    execute_cleanup_step(&mut game);
    assert_eq!(game.calculated_power(kargan_id), Some(8));
    assert_eq!(game.calculated_toughness(kargan_id), Some(8));
}

#[test]
pub(super) fn guardian_of_the_ages_loses_defender_and_stops_triggering() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let guardian = CardDefinitionBuilder::new(CardId::from_raw(72_141), "Guardian of the Ages")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(7)]]))
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .subtypes(vec![Subtype::Golem])
        .power_toughness(PowerToughness::fixed(7, 7))
        .parse_text(
            "Defender\n\
             When a creature attacks you or a planeswalker you control, if this creature has defender, it loses defender and gains trample.",
        )
        .expect("Guardian of the Ages should parse");
    let guardian_id = game.create_object_from_definition(&guardian, alice, Zone::Battlefield);

    assert!(
        game.object_has_ability(guardian_id, &StaticAbility::defender()),
        "Guardian of the Ages should start with defender"
    );
    assert!(
        !game.object_has_ability(guardian_id, &StaticAbility::trample()),
        "Guardian of the Ages should not start with trample"
    );

    let first_attacker = create_creature(&mut game, "First Attacker", bob, 2, 2);
    game.remove_summoning_sickness(first_attacker);
    game.turn.active_player = bob;
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(crate::game_state::Step::DeclareAttackers);

    let mut combat = CombatState::default();
    let mut trigger_queue = TriggerQueue::new();
    apply_attacker_declarations(
        &mut game,
        &mut combat,
        &mut trigger_queue,
        &[AttackerDeclaration {
            creature: first_attacker,
            target: AttackTarget::Player(alice),
        }],
    )
    .expect("attacking Guardian's controller should be legal");
    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("Guardian of the Ages trigger should go on stack");
    assert_eq!(
        game.stack.len(),
        1,
        "Guardian should trigger while it has defender"
    );
    resolve_stack_entry(&mut game).expect("Guardian of the Ages trigger should resolve");

    assert!(
        !game.object_has_ability(guardian_id, &StaticAbility::defender()),
        "Guardian of the Ages should lose defender after its trigger resolves"
    );
    assert!(
        game.object_has_ability(guardian_id, &StaticAbility::trample()),
        "Guardian of the Ages should gain trample after its trigger resolves"
    );

    let second_attacker = create_creature(&mut game, "Second Attacker", bob, 2, 2);
    game.remove_summoning_sickness(second_attacker);
    let mut second_combat = CombatState::default();
    let mut second_trigger_queue = TriggerQueue::new();
    apply_attacker_declarations(
        &mut game,
        &mut second_combat,
        &mut second_trigger_queue,
        &[AttackerDeclaration {
            creature: second_attacker,
            target: AttackTarget::Player(alice),
        }],
    )
    .expect("a later attack should be legal");
    put_triggers_on_stack(&mut game, &mut second_trigger_queue)
        .expect("empty trigger queue should still process");
    assert!(
        game.stack.is_empty(),
        "Guardian of the Ages should not trigger once it no longer has defender"
    );
}

#[test]
pub(super) fn guardian_of_the_ages_source_defender_condition_is_not_type_gated() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let guardian = CardDefinitionBuilder::new(CardId::from_raw(72_143), "Guardian of the Ages")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(7)]]))
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "Defender\n\
             When a creature attacks you or a planeswalker you control, if this creature has defender, it loses defender and gains trample.",
        )
        .expect("Guardian of the Ages should parse");
    let guardian_id = game.create_object_from_definition(&guardian, alice, Zone::Battlefield);

    let attacker = create_creature(&mut game, "Attacker", bob, 2, 2);
    game.remove_summoning_sickness(attacker);
    game.turn.active_player = bob;
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(crate::game_state::Step::DeclareAttackers);

    let mut combat = CombatState::default();
    let mut trigger_queue = TriggerQueue::new();
    apply_attacker_declarations(
        &mut game,
        &mut combat,
        &mut trigger_queue,
        &[AttackerDeclaration {
            creature: attacker,
            target: AttackTarget::Player(alice),
        }],
    )
    .expect("attacking Guardian's controller should be legal");
    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("Guardian of the Ages trigger should go on stack");
    assert_eq!(
        game.stack.len(),
        1,
        "self-reference should check whether the source has defender, not whether it is currently a creature"
    );
    resolve_stack_entry(&mut game).expect("Guardian of the Ages trigger should resolve");

    assert!(
        !game.object_has_ability(guardian_id, &StaticAbility::defender()),
        "Guardian of the Ages should lose defender after its trigger resolves"
    );
    assert!(
        game.object_has_ability(guardian_id, &StaticAbility::trample()),
        "Guardian of the Ages should gain trample after its trigger resolves"
    );
}

#[test]
pub(super) fn guardian_of_the_ages_triggers_for_planeswalker_you_control_only() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let guardian = CardDefinitionBuilder::new(CardId::from_raw(72_144), "Guardian of the Ages")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(7)]]))
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .subtypes(vec![Subtype::Golem])
        .power_toughness(PowerToughness::fixed(7, 7))
        .parse_text(
            "Defender\n\
             When a creature attacks you or a planeswalker you control, if this creature has defender, it loses defender and gains trample.",
        )
        .expect("Guardian of the Ages should parse");
    let guardian_id = game.create_object_from_definition(&guardian, alice, Zone::Battlefield);

    let alice_planeswalker = CardBuilder::new(CardId::from_raw(72_145), "Alice Planeswalker")
        .card_types(vec![CardType::Planeswalker])
        .loyalty(4)
        .build();
    let alice_planeswalker_id =
        game.create_object_from_card(&alice_planeswalker, alice, Zone::Battlefield);
    let bob_planeswalker = CardBuilder::new(CardId::from_raw(72_146), "Bob Planeswalker")
        .card_types(vec![CardType::Planeswalker])
        .loyalty(4)
        .build();
    let bob_planeswalker_id =
        game.create_object_from_card(&bob_planeswalker, bob, Zone::Battlefield);

    let alice_attacker = create_creature(&mut game, "Alice Attacker", alice, 2, 2);
    game.remove_summoning_sickness(alice_attacker);
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
            creature: alice_attacker,
            target: AttackTarget::Planeswalker(bob_planeswalker_id),
        }],
    )
    .expect("attacking an opponent's planeswalker should be legal");
    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("empty trigger queue should process");
    assert!(
        game.stack.is_empty(),
        "Guardian of the Ages should not trigger for attacks at an opponent's planeswalker"
    );

    let bob_attacker = create_creature(&mut game, "Bob Attacker", bob, 2, 2);
    game.remove_summoning_sickness(bob_attacker);
    game.turn.active_player = bob;

    let mut combat = CombatState::default();
    let mut trigger_queue = TriggerQueue::new();
    apply_attacker_declarations(
        &mut game,
        &mut combat,
        &mut trigger_queue,
        &[AttackerDeclaration {
            creature: bob_attacker,
            target: AttackTarget::Planeswalker(alice_planeswalker_id),
        }],
    )
    .expect("attacking Guardian's controller's planeswalker should be legal");
    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("Guardian of the Ages planeswalker attack trigger should go on stack");
    assert_eq!(
        game.stack.len(),
        1,
        "Guardian should trigger for attacks at a planeswalker its controller controls"
    );
    resolve_stack_entry(&mut game).expect("Guardian of the Ages trigger should resolve");

    assert!(
        !game.object_has_ability(guardian_id, &StaticAbility::defender()),
        "Guardian of the Ages should lose defender after a planeswalker attack trigger resolves"
    );
    assert!(
        game.object_has_ability(guardian_id, &StaticAbility::trample()),
        "Guardian of the Ages should gain trample after a planeswalker attack trigger resolves"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn sharpened_pitchfork_boosts_only_humans_but_always_grants_first_strike() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let pitchfork = CardDefinitionBuilder::new(CardId::from_raw(72_111), "Sharpened Pitchfork")
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Equipment])
        .parse_text(
            "Equipped creature has first strike.\nAs long as equipped creature is a Human, it gets +1/+1.\nEquip {1}",
        )
        .expect("Sharpened Pitchfork should parse");
    let pitchfork_id = game.create_object_from_definition(&pitchfork, alice, Zone::Battlefield);

    let human_id = CardBuilder::new(CardId::from_raw(72_112), "Human Bearer")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Human])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let human_id = game.create_object_from_card(&human_id, alice, Zone::Battlefield);
    game.remove_summoning_sickness(human_id);

    let non_human_id = CardBuilder::new(CardId::from_raw(72_113), "Elf Bearer")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Elf])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let non_human_id = game.create_object_from_card(&non_human_id, alice, Zone::Battlefield);
    game.remove_summoning_sickness(non_human_id);

    if let Some(equipment) = game.object_mut(pitchfork_id) {
        equipment.attached_to = Some(crate::object::AttachmentTarget::Object(human_id));
    }
    if let Some(human) = game.object_mut(human_id) {
        human.attachments.push(pitchfork_id);
    }

    assert_eq!(game.calculated_power(human_id), Some(3));
    assert_eq!(game.calculated_toughness(human_id), Some(3));

    let mut combat = CombatState::default();
    combat.attackers.push(crate::combat_state::AttackerInfo {
        creature: human_id,
        target: AttackTarget::Player(bob),
    });
    combat.blockers.insert(human_id, Vec::new());

    let first_strike_events = execute_combat_damage_step(&mut game, &combat, true);
    assert_eq!(
        first_strike_events.len(),
        1,
        "equipped Human should deal first-strike combat damage"
    );
    assert_eq!(game.player(bob).unwrap().life, 17);

    let regular_events = execute_combat_damage_step(&mut game, &combat, false);
    assert_eq!(
        regular_events.len(),
        0,
        "first strike should suppress regular damage"
    );

    if let Some(human) = game.object_mut(human_id) {
        human.attachments.clear();
    }
    if let Some(equipment) = game.object_mut(pitchfork_id) {
        equipment.attached_to = Some(crate::object::AttachmentTarget::Object(non_human_id));
    }
    if let Some(non_human) = game.object_mut(non_human_id) {
        non_human.attachments.push(pitchfork_id);
    }

    assert_eq!(game.calculated_power(non_human_id), Some(2));
    assert_eq!(game.calculated_toughness(non_human_id), Some(2));
}

#[test]
pub(super) fn proposed_granted_emerge_cast_keeps_sacrifice_cost_on_stack_spell() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let source = CardBuilder::new(CardId::from_raw(7012), "Emerge Grant Source")
        .card_types(vec![CardType::Creature])
        .build();
    let source_id = game.create_object_from_card(&source, alice, Zone::Battlefield);
    let grant_spec = crate::grant::GrantSpec::new(
        crate::grant::Grantable::emerge_from_cards_mana_cost(),
        crate::filter::ObjectFilter {
            card_types: vec![CardType::Creature],
            ..crate::filter::ObjectFilter::default()
        },
        Zone::Hand,
    );
    game.object_mut(source_id)
        .expect("source permanent should exist")
        .abilities_mut()
        .push(Ability::static_ability(StaticAbility::grants(grant_spec)));

    let sacrifice = CardBuilder::new(CardId::from_raw(7013), "Air Elemental")
        .card_types(vec![CardType::Creature])
        .mana_cost(ManaCost::from_symbols(vec![
            ManaSymbol::Generic(3),
            ManaSymbol::Blue,
            ManaSymbol::Blue,
        ]))
        .build();
    game.create_object_from_card(&sacrifice, alice, Zone::Battlefield);

    let spell = CardBuilder::new(CardId::from_raw(7014), "Vastwood Gorger")
        .card_types(vec![CardType::Creature])
        .mana_cost(ManaCost::from_symbols(vec![
            ManaSymbol::Generic(5),
            ManaSymbol::Green,
        ]))
        .build();
    let spell_id = game.create_object_from_card(&spell, alice, Zone::Hand);

    let grants = game
        .effect_store
        .grant_registry
        .granted_alternative_casts_for_card(&game, spell_id, Zone::Hand, alice);
    assert_eq!(
        grants.len(),
        1,
        "source should grant emerge to the creature card in hand, got {grants:?}"
    );

    let casting_method = CastingMethod::PlayFrom {
        source: source_id,
        zone: Zone::Hand,
        use_alternative: Some(0),
    };
    let stack_id = super::priority_mana::propose_spell_cast(
        &mut game,
        spell_id,
        Zone::Hand,
        alice,
        &casting_method,
    )
    .expect("granted emerge spell should move to the stack");

    let stack_spell = game
        .object(stack_id)
        .expect("stack spell should exist after proposal");
    let method = stack_spell
        .cast_alternative_method
        .as_ref()
        .expect("granted emerge method should be stored on the stack spell");
    assert_eq!(method.name(), "Emerge");
    assert_eq!(method.non_mana_costs().len(), 1);

    let steps = super::priority_cast::collect_spell_cost_steps(
        &game,
        stack_id,
        alice,
        &casting_method,
        &crate::cost::OptionalCostsPaid::default(),
        &[],
        None,
    );
    assert!(
        steps.iter().any(|step| matches!(
            step,
            super::priority_state::ActivationCostStep::Sacrifice { .. }
        )),
        "granted emerge should retain its sacrifice payment step after proposal"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn everybody_lives_prevents_life_loss_and_game_win_loss_for_all_players_this_turn() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let everybody_lives = CardDefinitionBuilder::new(CardId::from_raw(72_301), "Everybody Lives!")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "All creatures gain hexproof and indestructible until end of turn. Players gain hexproof until end of turn. Players can't lose life this turn and players can't lose the game or win the game this turn.",
        )
        .expect("Everybody Lives! should parse");

    let spell_id = game.create_object_from_definition(&everybody_lives, alice, Zone::Stack);
    game.push_to_stack(
        StackEntry::new(spell_id, alice).with_source_info(
            game.object(spell_id)
                .expect("Everybody Lives! spell should exist")
                .stable_id,
            "Everybody Lives!".to_string(),
        ),
    );
    resolve_stack_entry(&mut game).expect("Everybody Lives! should resolve");

    assert!(!game.can_lose_life(alice));
    assert!(!game.can_lose_life(bob));
    assert!(!game.can_lose_game(alice));
    assert!(!game.can_lose_game(bob));
    assert!(!game.can_win_game(alice));
    assert!(!game.can_win_game(bob));

    let bob_life_before = game.player(bob).expect("bob exists").life;
    assert_eq!(
        game.lose_life(bob, 3),
        0,
        "life loss should be prevented this turn"
    );
    assert_eq!(game.player(bob).expect("bob exists").life, bob_life_before);

    let alice_life_before = game.player(alice).expect("alice exists").life;
    let mut gain_ctx = crate::effects::ExecutionContext::new_default(spell_id, alice);
    crate::effects::execute_effect(
        &mut game,
        &crate::effect::Effect::gain_life(3),
        &mut gain_ctx,
    )
    .expect("life gain effect should resolve");
    assert_eq!(
        game.player(alice).expect("alice exists").life,
        alice_life_before + 3,
        "life gain should still be allowed"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn everybody_lives_switches_player_restrictions_on_resolution() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let everybody_lives = CardDefinitionBuilder::new(CardId::from_raw(72_302), "Everybody Lives!")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "All creatures gain hexproof and indestructible until end of turn. Players gain hexproof until end of turn. Players can't lose life this turn and players can't lose the game or win the game this turn.",
        )
        .expect("Everybody Lives! should parse");

    let spell_id = game.create_object_from_definition(&everybody_lives, alice, Zone::Stack);
    game.push_to_stack(
        StackEntry::new(spell_id, alice).with_source_info(
            game.object(spell_id)
                .expect("Everybody Lives! spell should exist")
                .stable_id,
            "Everybody Lives!".to_string(),
        ),
    );
    assert!(game.can_lose_life(alice));
    assert!(game.can_lose_life(bob));
    assert!(game.can_lose_game(alice));
    assert!(game.can_lose_game(bob));
    assert!(game.can_win_game(alice));
    assert!(game.can_win_game(bob));

    resolve_stack_entry(&mut game).expect("Everybody Lives! should resolve");

    assert!(!game.can_lose_life(alice));
    assert!(!game.can_lose_life(bob));
    assert!(!game.can_lose_game(alice));
    assert!(!game.can_lose_game(bob));
    assert!(!game.can_win_game(alice));
    assert!(!game.can_win_game(bob));
}

pub(super) fn resolve_triggered_ability_from_spell_cast(
    game: &mut GameState,
    triggered: &crate::ability::TriggeredAbility,
    source_id: ObjectId,
    controller: PlayerId,
    decision_maker: &mut dyn DecisionMaker,
) {
    let spell_id = game.create_object_from_card(
        &CardBuilder::new(CardId::new(), "Magecraft Probe Spell")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Blue]]))
            .card_types(vec![CardType::Instant])
            .build(),
        controller,
        Zone::Stack,
    );
    let event = TriggerEvent::new_with_provenance(
        SpellCastEvent::new(spell_id, controller, Zone::Hand),
        crate::provenance::ProvNodeId::default(),
    );
    let mut ctx = crate::effects::ExecutionContext::new(source_id, controller, decision_maker)
        .with_triggering_event(event);

    for effect in &triggered.effects {
        crate::effects::execute_effect(game, effect, &mut ctx)
            .expect("Quandrix Apprentice trigger should resolve");
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn queue_minds_dilation_spell_cast(
    game: &mut GameState,
    caster: PlayerId,
    trigger_queue: &mut TriggerQueue,
) -> ObjectId {
    let spell_id = game.create_object_from_card(
        &CardBuilder::new(CardId::new(), "Triggering Probe Spell")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Blue]]))
            .card_types(vec![CardType::Instant])
            .build(),
        caster,
        Zone::Stack,
    );
    let event = TriggerEvent::new_with_provenance(
        SpellCastEvent::new(spell_id, caster, Zone::Hand),
        crate::provenance::ProvNodeId::default(),
    );
    queue_triggers_from_event(game, trigger_queue, event, false);
    spell_id
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn queue_nymris_spell_cast(
    game: &mut GameState,
    caster: PlayerId,
    trigger_queue: &mut TriggerQueue,
) -> ObjectId {
    let spell_id = game.create_object_from_card(
        &CardBuilder::new(CardId::new(), "Nymris Trigger Probe Spell")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Blue]]))
            .card_types(vec![CardType::Instant])
            .build(),
        caster,
        Zone::Stack,
    );
    let event = TriggerEvent::new_with_provenance(
        SpellCastEvent::new(spell_id, caster, Zone::Hand),
        crate::provenance::ProvNodeId::default(),
    );
    queue_triggers_from_event(game, trigger_queue, event, false);
    spell_id
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn queue_aligned_heart_spell_cast(
    game: &mut GameState,
    caster: PlayerId,
    card_types: Vec<CardType>,
    trigger_queue: &mut TriggerQueue,
) -> ObjectId {
    let mut builder = CardBuilder::new(CardId::new(), "Aligned Heart Trigger Probe Spell")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Blue]]))
        .card_types(card_types.clone());
    if card_types.contains(&CardType::Creature) {
        builder = builder.power_toughness(PowerToughness::fixed(1, 1));
    }
    let spell_id = game.create_object_from_card(&builder.build(), caster, Zone::Stack);
    let event = TriggerEvent::new_with_provenance(
        SpellCastEvent::new(spell_id, caster, Zone::Hand),
        crate::provenance::ProvNodeId::default(),
    );
    queue_triggers_from_event(game, trigger_queue, event, false);
    spell_id
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn aligned_heart_triggers_only_on_second_spell_and_token_prowess_branches() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;

    let aligned_heart = aligned_heart_definition();
    let source = game.create_object_from_definition(&aligned_heart, alice, Zone::Battlefield);

    let mut first_queue = TriggerQueue::new();
    queue_aligned_heart_spell_cast(&mut game, alice, vec![CardType::Instant], &mut first_queue);
    assert_eq!(
        first_queue.entries.len(),
        0,
        "Aligned Heart should not trigger for your first spell each turn"
    );

    let mut second_queue = TriggerQueue::new();
    queue_aligned_heart_spell_cast(&mut game, alice, vec![CardType::Instant], &mut second_queue);
    assert_eq!(
        second_queue.entries.len(),
        1,
        "Aligned Heart should trigger for your second spell each turn"
    );
    let mut dm = SelectFirstDecisionMaker;
    put_triggers_on_stack_with_dm(&mut game, &mut second_queue, &mut dm)
        .expect("Aligned Heart trigger should go on the stack");
    resolve_stack_entry_with(&mut game, &mut dm).expect("Aligned Heart trigger should resolve");

    assert_eq!(
        game.counter_count(source, crate::object::CounterType::Named("rally")),
        1,
        "Aligned Heart should put one rally counter on itself"
    );
    let monks = aligned_heart_monk_tokens(&game, alice);
    assert_eq!(
        monks.len(),
        1,
        "one rally counter should create one Monk token"
    );
    let monk = monks[0];
    assert_eq!(game.calculated_power(monk), Some(1));
    assert_eq!(game.calculated_toughness(monk), Some(1));

    let mut third_creature_queue = TriggerQueue::new();
    queue_aligned_heart_spell_cast(
        &mut game,
        alice,
        vec![CardType::Creature],
        &mut third_creature_queue,
    );
    assert_eq!(
        third_creature_queue.entries.len(),
        0,
        "neither Aligned Heart nor prowess should trigger from a third creature spell"
    );

    let mut prowess_queue = TriggerQueue::new();
    queue_aligned_heart_spell_cast(
        &mut game,
        alice,
        vec![CardType::Instant],
        &mut prowess_queue,
    );
    assert_eq!(
        prowess_queue.entries.len(),
        1,
        "the Monk token's prowess should trigger from a noncreature spell"
    );
    put_triggers_on_stack_with_dm(&mut game, &mut prowess_queue, &mut dm)
        .expect("Prowess trigger should go on the stack");
    resolve_stack_entry_with(&mut game, &mut dm).expect("Prowess trigger should resolve");
    assert_eq!(
        game.calculated_power(monk),
        Some(2),
        "prowess should give the Monk +1/+1 until end of turn"
    );
    assert_eq!(game.calculated_toughness(monk), Some(2));
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn aligned_heart_creates_one_monk_for_each_rally_counter_on_it() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;

    let aligned_heart = aligned_heart_definition();
    let source = game.create_object_from_definition(&aligned_heart, alice, Zone::Battlefield);
    game.add_counters(source, crate::object::CounterType::Named("rally"), 1)
        .expect("preexisting rally counter should be addable");

    let mut first_queue = TriggerQueue::new();
    queue_aligned_heart_spell_cast(&mut game, alice, vec![CardType::Instant], &mut first_queue);
    assert_eq!(first_queue.entries.len(), 0);

    let mut second_queue = TriggerQueue::new();
    queue_aligned_heart_spell_cast(&mut game, alice, vec![CardType::Instant], &mut second_queue);
    assert_eq!(second_queue.entries.len(), 1);
    let mut dm = SelectFirstDecisionMaker;
    put_triggers_on_stack_with_dm(&mut game, &mut second_queue, &mut dm)
        .expect("Aligned Heart trigger should go on the stack");
    resolve_stack_entry_with(&mut game, &mut dm).expect("Aligned Heart trigger should resolve");

    assert_eq!(
        game.counter_count(source, crate::object::CounterType::Named("rally")),
        2,
        "the trigger should add a rally counter before counting counters for tokens"
    );
    assert_eq!(
        aligned_heart_monk_tokens(&game, alice).len(),
        2,
        "two rally counters on Aligned Heart should create two Monk tokens"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn create_nymris_library_card(
    game: &mut GameState,
    owner: PlayerId,
    name: &str,
) -> crate::ids::StableId {
    let card = CardBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Instant])
        .build();
    let id = game.create_object_from_card(&card, owner, Zone::Library);
    game.object(id)
        .expect("Nymris library card exists")
        .stable_id
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn nymris_first_spell_on_opponents_turn_moves_one_top_card_to_hand_and_one_to_graveyard()
{
    let mut game = setup_three_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.turn.active_player = bob;
    game.turn.priority_player = Some(alice);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;

    let nymris = nymris_definition();
    game.create_object_from_definition(&nymris, alice, Zone::Battlefield);
    let bottom_stable = create_nymris_library_card(&mut game, alice, "Nymris Bottom Card");
    let top_one_stable = create_nymris_library_card(&mut game, alice, "Nymris Top Card One");
    let top_two_stable = create_nymris_library_card(&mut game, alice, "Nymris Top Card Two");

    let mut trigger_queue = TriggerQueue::new();
    queue_nymris_spell_cast(&mut game, alice, &mut trigger_queue);
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "Nymris should trigger for your first spell during an opponent's turn"
    );
    assert_eq!(
        trigger_queue.entries[0].ability.trigger.display(),
        "Whenever you cast your first spell during each opponent's turn"
    );

    let mut dm = SelectFirstDecisionMaker;
    put_triggers_on_stack_with_dm(&mut game, &mut trigger_queue, &mut dm)
        .expect("Nymris trigger should go on the stack");
    resolve_stack_entry_with(&mut game, &mut dm).expect("Nymris trigger should resolve");

    let top_one_id = game
        .find_object_by_stable_id(top_one_stable)
        .expect("first looked-at card should still exist");
    let top_two_id = game
        .find_object_by_stable_id(top_two_stable)
        .expect("second looked-at card should still exist");
    let hand = &game.player(alice).expect("alice exists").hand;
    let graveyard = &game.player(alice).expect("alice exists").graveyard;
    assert_eq!(
        usize::from(hand.contains(&top_one_id)) + usize::from(hand.contains(&top_two_id)),
        1,
        "exactly one of the two looked-at cards should move to Alice's hand"
    );
    assert_eq!(
        usize::from(graveyard.contains(&top_one_id)) + usize::from(graveyard.contains(&top_two_id)),
        1,
        "the other looked-at card should move to Alice's graveyard"
    );

    let bottom_id = game
        .find_object_by_stable_id(bottom_stable)
        .expect("bottom card should still exist");
    assert!(
        game.player(alice)
            .expect("alice exists")
            .library
            .contains(&bottom_id),
        "Nymris should only move the top two library cards"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn nymris_ignores_your_spell_during_your_own_turn() {
    let mut game = setup_three_player_game();
    let alice = PlayerId::from_index(0);
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;

    let nymris = nymris_definition();
    game.create_object_from_definition(&nymris, alice, Zone::Battlefield);

    let mut trigger_queue = TriggerQueue::new();
    queue_nymris_spell_cast(&mut game, alice, &mut trigger_queue);
    assert_eq!(
        trigger_queue.entries.len(),
        0,
        "Nymris should not trigger for your spell during your own turn"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn nymris_ignores_opponents_spell_during_that_opponents_turn() {
    let mut game = setup_three_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.turn.active_player = bob;
    game.turn.priority_player = Some(bob);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;

    let nymris = nymris_definition();
    game.create_object_from_definition(&nymris, alice, Zone::Battlefield);

    let mut trigger_queue = TriggerQueue::new();
    queue_nymris_spell_cast(&mut game, bob, &mut trigger_queue);
    assert_eq!(
        trigger_queue.entries.len(),
        0,
        "Nymris should not trigger for an opponent casting a spell"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn nymris_ignores_your_second_spell_during_same_opponents_turn() {
    let mut game = setup_three_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.turn.active_player = bob;
    game.turn.priority_player = Some(alice);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;

    let nymris = nymris_definition();
    game.create_object_from_definition(&nymris, alice, Zone::Battlefield);

    let mut first_queue = TriggerQueue::new();
    queue_nymris_spell_cast(&mut game, alice, &mut first_queue);
    assert_eq!(first_queue.entries.len(), 1);

    let mut second_queue = TriggerQueue::new();
    queue_nymris_spell_cast(&mut game, alice, &mut second_queue);
    assert_eq!(
        second_queue.entries.len(),
        0,
        "Nymris should trigger only for your first spell during each opponent's turn"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn minds_dilation_first_opponent_spell_exiles_that_players_top_nonland_and_casts_it() {
    let mut game = setup_three_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;

    let minds_dilation = minds_dilation_definition();
    game.create_object_from_definition(&minds_dilation, alice, Zone::Battlefield);

    let charlie_top = game.create_object_from_card(
        &CardBuilder::new(CardId::new(), "Charlie's Top Card")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)]]))
            .card_types(vec![CardType::Sorcery])
            .build(),
        charlie,
        Zone::Library,
    );
    let charlie_top_stable = game
        .object(charlie_top)
        .expect("Charlie's library card should exist")
        .stable_id;
    let bob_top = game.create_object_from_card(
        &CardBuilder::new(CardId::new(), "Bob's Exiled Spell")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]]))
            .card_types(vec![CardType::Sorcery])
            .build(),
        bob,
        Zone::Library,
    );
    let bob_top_stable = game
        .object(bob_top)
        .expect("Bob's library card should exist")
        .stable_id;

    let mut trigger_queue = TriggerQueue::new();
    queue_minds_dilation_spell_cast(&mut game, bob, &mut trigger_queue);
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "Mind's Dilation should trigger for an opponent's first spell each turn"
    );

    let mut dm = SelectFirstDecisionMaker;
    put_triggers_on_stack_with_dm(&mut game, &mut trigger_queue, &mut dm)
        .expect("Mind's Dilation trigger should go on the stack");
    resolve_stack_entry_with(&mut game, &mut dm).expect("Mind's Dilation trigger should resolve");

    let cast_id = game
        .find_object_by_stable_id(bob_top_stable)
        .expect("Bob's exiled nonland card should still exist");
    let cast_object = game
        .object(cast_id)
        .expect("Bob's exiled nonland card should be on stack");
    assert_eq!(cast_object.zone, Zone::Stack);
    assert!(
        game.stack
            .iter()
            .any(|entry| entry.object_id == cast_id && entry.controller == alice),
        "Alice should cast Bob's exiled nonland card without paying its mana cost"
    );

    let charlie_id = game
        .find_object_by_stable_id(charlie_top_stable)
        .expect("Charlie's library card should still exist");
    assert_eq!(
        game.object(charlie_id)
            .expect("Charlie's card should exist")
            .zone,
        Zone::Library,
        "Mind's Dilation should exile the triggering player's top card, not another opponent's"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn minds_dilation_declined_may_cast_leaves_nonland_card_exiled() {
    let mut game = setup_three_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;

    let minds_dilation = minds_dilation_definition();
    game.create_object_from_definition(&minds_dilation, alice, Zone::Battlefield);
    let bob_top = game.create_object_from_card(
        &CardBuilder::new(CardId::new(), "Declined Exiled Spell")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]]))
            .card_types(vec![CardType::Sorcery])
            .build(),
        bob,
        Zone::Library,
    );
    let bob_top_stable = game
        .object(bob_top)
        .expect("Bob's library card should exist")
        .stable_id;

    let mut trigger_queue = TriggerQueue::new();
    queue_minds_dilation_spell_cast(&mut game, bob, &mut trigger_queue);
    let mut dm = AutoPassDecisionMaker;
    put_triggers_on_stack_with_dm(&mut game, &mut trigger_queue, &mut dm)
        .expect("Mind's Dilation trigger should go on the stack");
    resolve_stack_entry_with(&mut game, &mut dm).expect("Mind's Dilation trigger should resolve");

    let exiled_id = game
        .find_object_by_stable_id(bob_top_stable)
        .expect("Bob's declined nonland card should still exist");
    assert_eq!(
        game.object(exiled_id)
            .expect("Bob's declined nonland card should exist")
            .zone,
        Zone::Exile,
        "declining the optional cast should leave the nonland card in exile"
    );
    assert!(
        game.stack.is_empty(),
        "declining the optional cast should not put the exiled card on the stack"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn minds_dilation_land_card_is_exiled_but_not_cast() {
    let mut game = setup_three_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;

    let minds_dilation = minds_dilation_definition();
    game.create_object_from_definition(&minds_dilation, alice, Zone::Battlefield);
    let bob_top = game.create_object_from_card(
        &CardBuilder::new(CardId::new(), "Exiled Island")
            .card_types(vec![CardType::Land])
            .subtypes(vec![Subtype::Island])
            .build(),
        bob,
        Zone::Library,
    );
    let bob_top_stable = game
        .object(bob_top)
        .expect("Bob's library land should exist")
        .stable_id;

    let mut trigger_queue = TriggerQueue::new();
    queue_minds_dilation_spell_cast(&mut game, bob, &mut trigger_queue);
    let mut dm = SelectFirstDecisionMaker;
    put_triggers_on_stack_with_dm(&mut game, &mut trigger_queue, &mut dm)
        .expect("Mind's Dilation trigger should go on the stack");
    resolve_stack_entry_with(&mut game, &mut dm).expect("Mind's Dilation trigger should resolve");

    let exiled_id = game
        .find_object_by_stable_id(bob_top_stable)
        .expect("Bob's exiled land should still exist");
    assert_eq!(
        game.object(exiled_id)
            .expect("Bob's exiled land should exist")
            .zone,
        Zone::Exile,
        "Mind's Dilation should exile a land top card"
    );
    assert!(
        game.stack.is_empty(),
        "Mind's Dilation should not cast a land card from exile"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn minds_dilation_ignores_second_spell_by_same_opponent_that_turn() {
    let mut game = setup_three_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;

    let minds_dilation = minds_dilation_definition();
    game.create_object_from_definition(&minds_dilation, alice, Zone::Battlefield);

    let mut first_queue = TriggerQueue::new();
    queue_minds_dilation_spell_cast(&mut game, bob, &mut first_queue);
    assert_eq!(first_queue.entries.len(), 1);

    let mut second_queue = TriggerQueue::new();
    queue_minds_dilation_spell_cast(&mut game, bob, &mut second_queue);
    assert_eq!(
        second_queue.entries.len(),
        0,
        "Mind's Dilation should trigger only for an opponent's first spell each turn"
    );
}

pub(super) struct DeclineOptionalTriggerTargetsDecisionMaker {
    pub(super) seen_min_targets: Option<usize>,
    pub(super) seen_max_targets: Option<Option<usize>>,
}

impl DecisionMaker for DeclineOptionalTriggerTargetsDecisionMaker {
    fn decide_targets(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::TargetsContext,
    ) -> Vec<Target> {
        let requirement = ctx
            .requirements
            .first()
            .expect("expected a single target requirement");
        self.seen_min_targets = Some(requirement.min_targets);
        self.seen_max_targets = Some(requirement.max_targets);
        Vec::new()
    }
}

#[derive(Default)]
pub(super) struct CaptureRevealDecisionMaker {
    pub(super) view_calls: Vec<(PlayerId, PlayerId, Zone, bool, Vec<ObjectId>)>,
}

impl DecisionMaker for CaptureRevealDecisionMaker {
    fn decide_objects(
        &mut self,
        _game: &GameState,
        _ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        panic!("random hand reveal should not prompt for object selection");
    }

    fn view_cards(
        &mut self,
        _game: &GameState,
        viewer: PlayerId,
        cards: &[ObjectId],
        ctx: &crate::decisions::context::ViewCardsContext,
    ) {
        self.view_calls
            .push((viewer, ctx.subject, ctx.zone, ctx.public, cards.to_vec()));
    }
}

pub(super) struct AlhammarretChoiceDecisionMaker {
    pub(super) name: &'static str,
    pub(super) view_calls: Vec<(PlayerId, PlayerId, Zone, bool, Vec<ObjectId>)>,
}

impl AlhammarretChoiceDecisionMaker {
    pub(super) fn new(name: &'static str) -> Self {
        Self {
            name,
            view_calls: Vec::new(),
        }
    }
}

impl DecisionMaker for AlhammarretChoiceDecisionMaker {
    fn decide_text(
        &mut self,
        _game: &GameState,
        _ctx: &crate::decisions::context::TextInputContext,
    ) -> String {
        self.name.to_string()
    }

    fn view_cards(
        &mut self,
        _game: &GameState,
        viewer: PlayerId,
        cards: &[ObjectId],
        ctx: &crate::decisions::context::ViewCardsContext,
    ) {
        self.view_calls
            .push((viewer, ctx.subject, ctx.zone, ctx.public, cards.to_vec()));
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn alhammarret_high_arbiter_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(91_201), "Alhammarret, High Arbiter")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(5)],
            vec![ManaSymbol::Blue],
            vec![ManaSymbol::Blue],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Sphinx])
        .power_toughness(PowerToughness::fixed(5, 5))
        .parse_text(
            "Flying\nAs Alhammarret enters, each opponent reveals their hand. You choose the name of a nonland card revealed this way.\nYour opponents can't cast spells with the chosen name (as long as this creature is on the battlefield).",
        )
        .expect("Alhammarret should parse for runtime regression")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn named_spell_definition(card_id: u32, name: &str) -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(card_id), name)
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)]]))
        .card_types(vec![CardType::Instant])
        .with_spell_effect(vec![Effect::draw(1)])
        .build()
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn named_land_definition(card_id: u32, name: &str) -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(card_id), name)
        .card_types(vec![CardType::Land])
        .build()
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn alhammarret_high_arbiter_reveals_opponents_hand_and_blocks_chosen_nonland_name() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let alhammarret = alhammarret_high_arbiter_definition();
    let lightning = named_spell_definition(91_202, "Lightning Bolt");
    let giant_growth = named_spell_definition(91_203, "Giant Growth");
    let forest = named_land_definition(91_204, "Forest");

    let alhammarret_hand = game.create_object_from_definition(&alhammarret, alice, Zone::Hand);
    let bob_lightning = game.create_object_from_definition(&lightning, bob, Zone::Hand);
    let bob_other = game.create_object_from_definition(&giant_growth, bob, Zone::Hand);
    let bob_forest = game.create_object_from_definition(&forest, bob, Zone::Hand);
    let alice_lightning = game.create_object_from_definition(&lightning, alice, Zone::Hand);

    let mut dm = AlhammarretChoiceDecisionMaker::new("Lightning Bolt");
    let result = game
        .move_object_with_etb_processing_with_dm(alhammarret_hand, Zone::Battlefield, &mut dm)
        .expect("Alhammarret should enter the battlefield");
    game.update_cant_effects();

    assert_eq!(
        game.chosen_named_option(result.new_id).map(str::to_string),
        Some("Lightning Bolt".to_string()),
        "Alhammarret should store the nonland name chosen from an opponent's revealed hand"
    );
    assert_eq!(
        dm.view_calls.len(),
        2,
        "Bob's hand should be revealed to both players"
    );
    assert!(
        dm.view_calls
            .iter()
            .all(|(_, subject, zone, public, cards)| {
                *subject == bob
                    && *zone == Zone::Hand
                    && *public
                    && cards == &vec![bob_lightning, bob_other, bob_forest]
            })
    );

    let bob_filter_debug = format!(
        "{:?}",
        game.effect_store.cant_effects.cast_filters_for_player(bob)
    );
    let alice_filter_debug = format!(
        "{:?}",
        game.effect_store
            .cant_effects
            .cast_filters_for_player(alice)
    );
    assert!(
        bob_filter_debug.contains("Lightning Bolt") && !bob_filter_debug.contains("Giant Growth"),
        "Alhammarret should add only a Bob-side cast prohibition for the chosen nonland name, got {bob_filter_debug}"
    );
    assert!(
        alice_filter_debug == "None",
        "Alhammarret should not restrict its controller, got {alice_filter_debug}"
    );
    game.move_object_by_effect(result.new_id, Zone::Graveyard)
        .expect("Alhammarret should be movable off the battlefield");
    game.update_cant_effects();
    assert!(
        game.effect_store
            .cant_effects
            .cast_filters_for_player(bob)
            .is_none(),
        "Alhammarret's chosen-name restriction should end when it leaves the battlefield"
    );
    assert!(
        game.object(bob_lightning).is_some()
            && game.object(bob_other).is_some()
            && game.object(alice_lightning).is_some(),
        "test spells should remain in hand while checking cast restrictions"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn alhammarret_high_arbiter_rejects_revealed_land_name_choice() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let alhammarret = alhammarret_high_arbiter_definition();
    let lightning = named_spell_definition(91_205, "Lightning Bolt");
    let forest = named_land_definition(91_206, "Forest");

    let alhammarret_hand = game.create_object_from_definition(&alhammarret, alice, Zone::Hand);
    let bob_lightning = game.create_object_from_definition(&lightning, bob, Zone::Hand);
    game.create_object_from_definition(&forest, bob, Zone::Hand);

    let mut dm = AlhammarretChoiceDecisionMaker::new("Forest");
    let result = game
        .move_object_with_etb_processing_with_dm(alhammarret_hand, Zone::Battlefield, &mut dm)
        .expect("Alhammarret should enter the battlefield");
    game.update_cant_effects();

    assert!(
        game.chosen_named_option(result.new_id).is_none(),
        "Alhammarret should reject land names even when revealed"
    );
    assert!(
        game.effect_store
            .cant_effects
            .cast_filters_for_player(bob)
            .is_none(),
        "without a valid nonland revealed choice, Alhammarret should not add a cast prohibition"
    );
    assert!(
        game.object(bob_lightning).is_some(),
        "Bob's Lightning Bolt should remain available in hand"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn alhammarret_high_arbiter_rejects_unrevealed_nonland_name_choice() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let alhammarret = alhammarret_high_arbiter_definition();
    let lightning = named_spell_definition(91_207, "Lightning Bolt");

    let alhammarret_hand = game.create_object_from_definition(&alhammarret, alice, Zone::Hand);
    let bob_lightning = game.create_object_from_definition(&lightning, bob, Zone::Hand);

    let mut dm = AlhammarretChoiceDecisionMaker::new("Giant Growth");
    let result = game
        .move_object_with_etb_processing_with_dm(alhammarret_hand, Zone::Battlefield, &mut dm)
        .expect("Alhammarret should enter the battlefield");
    game.update_cant_effects();

    assert!(
        game.chosen_named_option(result.new_id).is_none(),
        "Alhammarret should reject nonland names that were not revealed from an opponent's hand"
    );
    assert!(
        game.effect_store
            .cant_effects
            .cast_filters_for_player(bob)
            .is_none(),
        "without a revealed nonland choice, Alhammarret should not add a cast prohibition"
    );
    assert!(
        game.object(bob_lightning).is_some(),
        "Bob's Lightning Bolt should remain available in hand"
    );
}

#[test]
pub(super) fn test_generate_damage_triggers_emits_life_loss_for_player_damage() {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();

    let events = vec![CombatDamageEvent {
        source: ObjectId::from_raw(99),
        target: DamageEventTarget::Player(PlayerId::from_index(1)),
        amount: 3,
        life_lost: 3,
        result: DamageResult::default(),
    }];

    generate_damage_triggers(&mut game, &events, &mut trigger_queue);

    assert_eq!(
        game.trigger_event_kind_count_this_turn(EventKind::Damage),
        1
    );
    assert_eq!(
        game.trigger_event_kind_count_this_turn(EventKind::LifeLoss),
        1
    );
    assert_eq!(
        game.turn_store
            .turn_history
            .total_damage_to_player(PlayerId::from_index(1)),
        3
    );
}

#[test]
pub(super) fn test_monarch_end_step_draws_a_card() {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let bob = PlayerId::from_index(1);

    let library_card = CardBuilder::new(CardId::from_raw(9102), "Monarch Draw Test")
        .card_types(vec![CardType::Artifact])
        .build();
    game.create_object_from_card(&library_card, bob, Zone::Library);

    game.turn.active_player = bob;
    game.turn.phase = Phase::Ending;
    game.turn.step = Some(crate::game_state::Step::End);
    game.monarch = Some(bob);

    let hand_before = game.player(bob).expect("bob exists").hand.len();
    let library_before = game.player(bob).expect("bob exists").library.len();

    generate_and_queue_step_triggers(&mut game, &mut trigger_queue);

    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "the monarch should get a draw trigger at the beginning of their end step"
    );
    assert_eq!(
        trigger_queue.entries[0].source_name.as_str(),
        "The Monarch",
        "the designation trigger should have a stable synthetic source name"
    );
    assert_eq!(
        trigger_queue.entries[0].controller, bob,
        "the monarch controls the designation trigger"
    );

    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("monarch draw trigger should go on the stack");
    assert_eq!(
        game.stack.len(),
        1,
        "monarch draw trigger should use the stack"
    );

    resolve_stack_entry(&mut game).expect("monarch draw trigger should resolve");

    assert_eq!(
        game.player(bob).expect("bob exists").hand.len(),
        hand_before + 1,
        "the monarch should draw one card on their end step"
    );
    assert_eq!(
        game.player(bob).expect("bob exists").library.len(),
        library_before - 1,
        "the drawn card should leave the library"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn sarevok_deathbringer_end_step_hits_active_player_when_no_permanent_left() {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let sarevok = CardDefinitionBuilder::new(CardId::from_raw(81_001), "Sarevok, Deathbringer")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 4))
        .parse_text(
            "At the beginning of each player's end step, if no permanents left the battlefield this turn, that player loses X life, where X is Sarevok's power.\nChoose a Background (You can have a Background as a second commander.)",
        )
        .expect("Sarevok, Deathbringer should parse");
    game.create_object_from_definition(&sarevok, alice, Zone::Battlefield);

    game.turn.active_player = bob;
    game.turn.phase = Phase::Ending;
    game.turn.step = Some(crate::game_state::Step::End);

    let bob_life_before = game.player(bob).expect("bob exists").life;
    generate_and_queue_step_triggers(&mut game, &mut trigger_queue);

    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "Sarevok should trigger on each player's end step when nothing left the battlefield"
    );

    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("Sarevok trigger should go on the stack");
    resolve_stack_entry(&mut game).expect("Sarevok trigger should resolve");

    assert_eq!(
        game.player(bob).expect("bob exists").life,
        bob_life_before - 3,
        "Sarevok should make the active player lose life equal to its power"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn sarevok_deathbringer_skips_end_step_if_a_permanent_left_battlefield() {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let sarevok = CardDefinitionBuilder::new(CardId::from_raw(81_002), "Sarevok, Deathbringer")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 4))
        .parse_text(
            "At the beginning of each player's end step, if no permanents left the battlefield this turn, that player loses X life, where X is Sarevok's power.\nChoose a Background (You can have a Background as a second commander.)",
        )
        .expect("Sarevok, Deathbringer should parse");
    game.create_object_from_definition(&sarevok, alice, Zone::Battlefield);

    let departing = CardBuilder::new(CardId::from_raw(81_003), "Leaving Permanent")
        .card_types(vec![CardType::Artifact])
        .build();
    let departing_id = game.create_object_from_card(&departing, alice, Zone::Battlefield);
    let departing_snapshot = {
        let object = game
            .object(departing_id)
            .expect("departing permanent exists");
        crate::snapshot::ObjectSnapshot::from_object(object, &game)
    };
    let zone_change = crate::events::RawEvent::new(
        crate::events::ZoneChangeEvent::with_cause(
            departing_id,
            Zone::Battlefield,
            Zone::Graveyard,
            crate::events::cause::EventCause::effect(),
            Some(departing_snapshot.clone()),
        ),
        crate::provenance::ProvNodeId::default(),
    );
    game.turn_store
        .turn_history
        .record_event(&zone_change, Some(departing_snapshot), None);

    game.turn.active_player = bob;
    game.turn.phase = Phase::Ending;
    game.turn.step = Some(crate::game_state::Step::End);

    generate_and_queue_step_triggers(&mut game, &mut trigger_queue);

    assert!(
        trigger_queue.entries.is_empty(),
        "Sarevok should not trigger if any permanent left the battlefield this turn"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn dawnbreak_reclaimer_end_step_returns_both_chosen_graveyard_creatures() {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let dawnbreak = CardDefinitionBuilder::new(CardId::from_raw(81_010), "Dawnbreak Reclaimer")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(5, 5))
        .parse_text(
            "Flying\nAt the beginning of your end step, choose a creature card in an opponent's graveyard, then that player chooses a creature card in your graveyard. You may return those cards to the battlefield under their owners' control.",
        )
        .expect("Dawnbreak Reclaimer should parse");
    game.create_object_from_definition(&dawnbreak, alice, Zone::Battlefield);

    let bob_grave_creature = CardBuilder::new(CardId::from_raw(81_011), "Bob Gravebody")
        .card_types(vec![CardType::Creature])
        .build();
    let alice_grave_creature = CardBuilder::new(CardId::from_raw(81_012), "Alice Gravebody")
        .card_types(vec![CardType::Creature])
        .build();

    game.create_object_from_card(&bob_grave_creature, bob, Zone::Graveyard);
    game.create_object_from_card(&alice_grave_creature, alice, Zone::Graveyard);

    game.turn.active_player = alice;
    game.turn.phase = Phase::Ending;
    game.turn.step = Some(crate::game_state::Step::End);

    generate_and_queue_step_triggers(&mut game, &mut trigger_queue);
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "Dawnbreak Reclaimer should trigger at your end step"
    );

    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("Dawnbreak Reclaimer trigger should go on the stack");

    let mut dm = SelectFirstDecisionMaker;
    resolve_stack_entry_with_dm_and_triggers(&mut game, &mut dm, &mut trigger_queue)
        .expect("Dawnbreak Reclaimer trigger should resolve");

    let graveyard_has = |game: &GameState, player: PlayerId, name: &str| {
        game.player(player)
            .expect("player exists")
            .graveyard
            .iter()
            .any(|id| game.object(*id).is_some_and(|obj| obj.name == name))
    };
    let battlefield_has = |game: &GameState, owner: PlayerId, name: &str| {
        game.battlefield.iter().any(|id| {
            game.object(*id)
                .is_some_and(|obj| obj.owner == owner && obj.name == name)
        })
    };

    assert!(
        !graveyard_has(&game, bob, "Bob Gravebody"),
        "the opponent-chosen creature should leave their graveyard"
    );
    assert!(
        !graveyard_has(&game, alice, "Alice Gravebody"),
        "the chosen creature in your graveyard should leave your graveyard"
    );
    assert!(
        battlefield_has(&game, bob, "Bob Gravebody"),
        "the opponent-owned creature should return under its owner's control"
    );
    assert!(
        battlefield_has(&game, alice, "Alice Gravebody"),
        "your chosen creature should return under your control"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn exert_attack_choice_draws_card_and_skips_only_next_untap() {
    #[derive(Default)]
    struct AcceptBooleanDecisionMaker;

    impl DecisionMaker for AcceptBooleanDecisionMaker {
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

    let exert_probe = CardDefinitionBuilder::new(CardId::new(), "Exert Probe")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .parse_text("You may exert this creature as it attacks. When you do, draw a card.")
        .expect("exert attack line should parse");
    let source_id = game.create_object_from_definition(&exert_probe, alice, Zone::Battlefield);
    game.remove_summoning_sickness(source_id);

    let library_card = CardBuilder::new(CardId::from_raw(71_001), "Draw Target")
        .card_types(vec![CardType::Artifact])
        .build();
    game.create_object_from_card(&library_card, alice, Zone::Library);

    game.turn.active_player = alice;
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(crate::game_state::Step::DeclareAttackers);

    let hand_before = game.player(alice).expect("alice exists").hand.len();
    let mut combat = CombatState::default();
    let mut trigger_queue = TriggerQueue::new();
    let declarations = vec![AttackerDeclaration {
        creature: source_id,
        target: AttackTarget::Player(bob),
    }];

    let mut dm = AcceptBooleanDecisionMaker;
    apply_attacker_declarations_with_dm(
        &mut game,
        &mut combat,
        &mut trigger_queue,
        &declarations,
        &mut dm,
    )
    .expect("declaring an exert attacker should succeed");
    assert!(
        game.is_tapped(source_id),
        "attacking should tap the creature"
    );

    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("exert follow-up trigger should go on the stack");
    assert_eq!(
        game.stack.len(),
        1,
        "exert should queue exactly one linked trigger"
    );
    resolve_stack_entry(&mut game).expect("exert follow-up trigger should resolve");
    assert_eq!(
        game.player(alice).expect("alice exists").hand.len(),
        hand_before + 1,
        "accepting the exert prompt should resolve the linked draw trigger"
    );
    game.next_turn();
    crate::turn::execute_untap_step_with(&mut game, &mut dm);

    game.next_turn();
    crate::turn::execute_untap_step_with(&mut game, &mut dm);
    assert!(
        game.is_tapped(source_id),
        "the exerted attacker should stay tapped during its controller's next untap step"
    );

    game.next_turn();
    crate::turn::execute_untap_step_with(&mut game, &mut dm);

    game.next_turn();
    crate::turn::execute_untap_step_with(&mut game, &mut dm);
    assert!(
        !game.is_tapped(source_id),
        "the exert restriction should wear off after that untap step"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn exerting_trueheart_twins_triggers_its_own_exert_ability() {
    #[derive(Default)]
    struct AcceptBooleanDecisionMaker;

    impl DecisionMaker for AcceptBooleanDecisionMaker {
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

    let twins = CardDefinitionBuilder::new(CardId::new(), "Trueheart Twins")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(4, 4))
        .parse_text(
            "You may exert this creature as it attacks.\nWhenever you exert a creature, creatures you control get +1/+0 until end of turn.",
        )
        .expect("Trueheart Twins should parse");
    let twins_id = game.create_object_from_definition(&twins, alice, Zone::Battlefield);
    game.remove_summoning_sickness(twins_id);

    let hyena = CardBuilder::new(CardId::from_raw(71_002), "Hyena Pack")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 4))
        .build();
    let hyena_id = game.create_object_from_card(&hyena, alice, Zone::Battlefield);

    game.turn.active_player = alice;
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(crate::game_state::Step::DeclareAttackers);

    let mut combat = CombatState::default();
    let mut trigger_queue = TriggerQueue::new();
    let declarations = vec![AttackerDeclaration {
        creature: twins_id,
        target: AttackTarget::Player(bob),
    }];

    let mut dm = AcceptBooleanDecisionMaker;
    apply_attacker_declarations_with_dm(
        &mut game,
        &mut combat,
        &mut trigger_queue,
        &declarations,
        &mut dm,
    )
    .expect("declaring Trueheart Twins as an exert attacker should succeed");

    drain_pending_trigger_events(&mut game, &mut trigger_queue);
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "Trueheart Twins should trigger from its own exert event"
    );
    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("Trueheart Twins exert trigger should go on the stack");
    resolve_stack_entry(&mut game).expect("Trueheart Twins exert trigger should resolve");

    assert_eq!(game.calculated_power(twins_id), Some(5));
    assert_eq!(game.calculated_power(hyena_id), Some(4));
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn guild_artisan_grants_treasure_trigger_when_commander_attacks_tied_life_leader() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let guild_artisan = CardDefinitionBuilder::new(CardId::new(), "Guild Artisan")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::Red],
        ]))
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![crate::types::Subtype::Background])
        .parse_text(
            "Commander creatures you own have \"Whenever this creature attacks a player, if no opponent has more life than that player, you create two Treasure tokens.\"",
        )
        .expect("Guild Artisan should parse");

    let commander = CardBuilder::new(CardId::from_raw(71_020), "Guild Artisan Commander")
        .card_types(vec![CardType::Creature])
        .supertypes(vec![crate::types::Supertype::Legendary])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();

    let commander_id = game.create_object_from_card(&commander, alice, Zone::Battlefield);
    game.set_as_commander(commander_id, alice);
    game.remove_summoning_sickness(commander_id);
    game.create_object_from_definition(&guild_artisan, alice, Zone::Battlefield);
    game.refresh_continuous_state();

    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(crate::game_state::Step::DeclareAttackers);

    let mut combat = CombatState::default();
    let mut trigger_queue = TriggerQueue::new();
    let declarations = vec![AttackerDeclaration {
        creature: commander_id,
        target: AttackTarget::Player(bob),
    }];

    apply_attacker_declarations(&mut game, &mut combat, &mut trigger_queue, &declarations)
        .expect("commander attack should be legal");
    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("Guild Artisan trigger should go on the stack");
    assert_eq!(
        game.stack.len(),
        1,
        "Guild Artisan should create exactly one attack trigger"
    );

    resolve_stack_entry(&mut game).expect("Guild Artisan trigger should resolve");

    let treasure_count = game
        .battlefield
        .iter()
        .filter(|&&id| game.object(id).is_some_and(|obj| obj.name == "Treasure"))
        .count();
    assert_eq!(
        treasure_count, 2,
        "expected two Treasure tokens after resolution"
    );
}

#[test]
pub(super) fn attack_trigger_with_countered_attacker_taps_defending_creature() {
    use crate::object::{CounterType, ObjectKind};

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let scaleguard = CardDefinitionBuilder::new(CardId::new(), "Elite Scaleguard Probe")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(4)],
            vec![ManaSymbol::White],
        ]))
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 3))
        .parse_text(
            "Whenever a creature you control with a +1/+1 counter on it attacks, tap target creature defending player controls.",
        )
        .expect("attack trigger should parse");
    game.create_object_from_definition(&scaleguard, alice, Zone::Battlefield);

    let attacker = create_creature(&mut game, "Countered Attacker", alice, 2, 2);
    game.object_mut(attacker)
        .expect("attacker should exist")
        .add_counters(CounterType::PlusOnePlusOne, 1);
    game.remove_summoning_sickness(attacker);

    let defender = create_creature(&mut game, "Defending Creature", bob, 2, 4);

    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(crate::game_state::Step::DeclareAttackers);

    let mut combat = CombatState::default();
    let mut trigger_queue = TriggerQueue::new();
    let declarations = vec![AttackerDeclaration {
        creature: attacker,
        target: AttackTarget::Player(bob),
    }];

    apply_attacker_declarations(&mut game, &mut combat, &mut trigger_queue, &declarations)
        .expect("countered creature should be able to attack");
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "countered attacker should queue Elite Scaleguard-style trigger"
    );

    let mut dm = SelectFirstDecisionMaker;
    put_triggers_on_stack_with_dm(&mut game, &mut trigger_queue, &mut dm)
        .expect("targeted trigger should go on the stack");
    resolve_stack_entry(&mut game).expect("tap trigger should resolve");

    assert!(
        game.is_tapped(defender),
        "defending player's targeted creature should be tapped"
    );
}

#[test]
pub(super) fn bought_back_instant_returns_to_owners_hand_after_resolution() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let spell = CardDefinitionBuilder::new(CardId::new(), "Buyback Probe")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Green]]))
        .card_types(vec![CardType::Instant])
        .parse_text("Buyback {4}")
        .expect("buyback line should parse");
    let spell_id = game.create_object_from_definition(&spell, alice, Zone::Stack);
    let mut paid = crate::cost::OptionalCostsPaid::from_costs(&spell.optional_costs);
    paid.mark_label_paid("Buyback");

    game.push_to_stack(
        StackEntry::new(spell_id, alice)
            .with_optional_costs_paid(paid)
            .with_source_info(
                game.object(spell_id).expect("spell should exist").stable_id,
                "Buyback Probe".to_string(),
            ),
    );

    resolve_stack_entry(&mut game).expect("bought-back spell should resolve");

    assert_eq!(
        game.player(alice)
            .expect("alice should exist")
            .hand
            .iter()
            .filter(|&&id| game
                .object(id)
                .is_some_and(|obj| obj.name == "Buyback Probe"))
            .count(),
        1,
        "resolved bought-back spell should return to its owner's hand"
    );
    assert_eq!(
        game.player(alice)
            .expect("alice should exist")
            .graveyard
            .iter()
            .filter(|&&id| game
                .object(id)
                .is_some_and(|obj| obj.name == "Buyback Probe"))
            .count(),
        0,
        "resolved bought-back spell should not go to its owner's graveyard"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn clockspinning_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(80_600), "Clockspinning")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Blue]]))
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Buyback {3}
Choose a counter on target permanent or suspended card. Remove that counter from that permanent or card or put another of those counters on it.",
        )
        .expect("Clockspinning should parse for runtime tests")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn suspended_card_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(80_601), "Suspended Probe")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Blue]]))
        .card_types(vec![CardType::Instant])
        .parse_text("Suspend 3—{U}")
        .expect("suspend probe should parse for Clockspinning target tests")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) struct ClockspinningDecisionMaker {
    pub(super) mode_index: usize,
}

#[cfg(ironsmith_runtime_parser_tests)]
impl DecisionMaker for ClockspinningDecisionMaker {
    fn decide_options(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        if ctx.description == "Choose a counter kind" {
            vec![0]
        } else if ctx.description.starts_with("Choose for ") {
            vec![self.mode_index.min(ctx.options.len().saturating_sub(1))]
        } else {
            vec![0]
        }
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn clockspinning_counter_effect(
    def: &crate::cards::CardDefinition,
) -> &crate::effect::Effect {
    def.spell_effect
        .as_ref()
        .expect("Clockspinning should have spell effects")
        .flattened_default_effects()
        .into_iter()
        .find(|effect| {
            effect
                .downcast_ref::<crate::effects::ForEachCounterKindPutOrRemoveEffect>()
                .is_some()
        })
        .expect("Clockspinning should have a chosen-counter put/remove effect")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn clockspinning_target_permanent(
    game: &mut GameState,
    controller: PlayerId,
) -> ObjectId {
    let target = CardBuilder::new(CardId::from_raw(80_602), "Clockspinning Target")
        .card_types(vec![CardType::Artifact])
        .build();
    game.create_object_from_card(&target, controller, Zone::Battlefield)
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn clockspinning_targets_permanents_or_suspended_cards() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let def = clockspinning_definition();
    let effects = def
        .spell_effect
        .as_ref()
        .expect("Clockspinning spell effects");

    let countered_permanent = clockspinning_target_permanent(&mut game, alice);
    game.add_counters(countered_permanent, crate::object::CounterType::Charge, 1)
        .expect("countered permanent should accept a charge counter");
    let uncountered_permanent = clockspinning_target_permanent(&mut game, alice);
    let suspended = suspended_card_definition();
    let suspended_id = game.create_object_from_definition(&suspended, alice, Zone::Exile);
    game.add_counters(suspended_id, crate::object::CounterType::Time, 1)
        .expect("suspended card should accept a time counter");
    let uncountered_suspended = game.create_object_from_definition(&suspended, alice, Zone::Exile);
    let no_longer_suspended = game.create_object_from_definition(&suspended, alice, Zone::Exile);
    game.add_counters(no_longer_suspended, crate::object::CounterType::Charge, 1)
        .expect("exiled suspend card should accept non-time counters");

    let requirements = extract_target_requirements(&game, effects, alice, None);
    assert_eq!(
        requirements.len(),
        1,
        "Clockspinning should have one target requirement"
    );
    let legal_targets = &requirements[0].legal_targets;
    assert!(
        legal_targets.contains(&Target::Object(countered_permanent)),
        "countered permanents should be legal Clockspinning targets, got {legal_targets:?}"
    );
    assert!(
        legal_targets.contains(&Target::Object(suspended_id)),
        "suspended cards with time counters should be legal Clockspinning targets, got {legal_targets:?}"
    );
    assert!(
        legal_targets.contains(&Target::Object(uncountered_permanent)),
        "permanents without counters should still be legal Clockspinning targets, got {legal_targets:?}"
    );
    assert!(
        !legal_targets.contains(&Target::Object(uncountered_suspended)),
        "suspended cards without counters should not be legal Clockspinning targets, got {legal_targets:?}"
    );
    assert!(
        !legal_targets.contains(&Target::Object(no_longer_suspended)),
        "exiled cards with suspend but no time counter should not be legal Clockspinning targets, got {legal_targets:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn clockspinning_buyback_put_branch_returns_to_hand_and_adds_counter() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let def = clockspinning_definition();
    let spell_id = game.create_object_from_definition(&def, alice, Zone::Stack);
    let target = clockspinning_target_permanent(&mut game, alice);
    game.add_counters(target, crate::object::CounterType::Charge, 1)
        .expect("target should accept an initial charge counter");

    let mut paid = crate::cost::OptionalCostsPaid::from_costs(&def.optional_costs);
    paid.mark_label_paid("Buyback");
    game.push_to_stack(
        StackEntry::new(spell_id, alice)
            .with_targets(vec![Target::Object(target)])
            .with_optional_costs_paid(paid)
            .with_source_info(
                game.object(spell_id)
                    .expect("Clockspinning spell object")
                    .stable_id,
                "Clockspinning".to_string(),
            ),
    );

    let mut dm = ClockspinningDecisionMaker { mode_index: 0 };
    resolve_stack_entry_with(&mut game, &mut dm)
        .expect("Clockspinning should resolve its put branch");

    assert_eq!(
        game.counter_count(target, crate::object::CounterType::Charge),
        2,
        "Clockspinning put branch should add one chosen counter kind"
    );
    assert!(
        game.player(alice)
            .expect("alice exists")
            .hand
            .iter()
            .any(|&id| game
                .object(id)
                .is_some_and(|object| object.name == "Clockspinning")),
        "Clockspinning should return to hand when buyback was paid"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn clockspinning_remove_branch_removes_one_chosen_counter() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let def = clockspinning_definition();
    let spell_id = game.create_object_from_definition(&def, alice, Zone::Stack);
    let target = clockspinning_target_permanent(&mut game, alice);
    game.add_counters(target, crate::object::CounterType::Charge, 2)
        .expect("target should accept charge counters");

    let mut dm = ClockspinningDecisionMaker { mode_index: 1 };
    let ctx = crate::effects::ExecutionContext::new_default(spell_id, alice)
        .with_targets(vec![crate::effects::ResolvedTarget::Object(target)]);
    let mut ctx = ctx.with_decision_maker(&mut dm);
    crate::effects::execute_effect(&mut game, clockspinning_counter_effect(&def), &mut ctx)
        .expect("Clockspinning remove branch should execute");

    assert_eq!(
        game.counter_count(target, crate::object::CounterType::Charge),
        1,
        "Clockspinning remove branch should remove exactly one chosen counter"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn all_of_history_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(80_610), "All of History, All at Once")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Blue],
            vec![ManaSymbol::Blue],
        ]))
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Time travel. (For each suspended card you own and each permanent you control with a time counter on it, you may add or remove a time counter.)\n\
             Storm (When you cast this spell, copy it for each spell cast before it this turn.)",
        )
        .expect("All of History, All at Once should parse for runtime tests")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn all_of_history_time_travel_effect(
    def: &crate::cards::CardDefinition,
) -> &crate::effect::Effect {
    def.spell_effect
        .as_ref()
        .expect("All of History, All at Once should have spell effects")
        .flattened_default_effects()
        .into_iter()
        .find(|effect| {
            effect
                .downcast_ref::<crate::effects::ForEachCounterKindPutOrRemoveEffect>()
                .is_some_and(|effect| {
                    effect.fixed_counter_type == Some(crate::object::CounterType::Time)
                })
        })
        .expect("All of History, All at Once should have a fixed time-counter put/remove effect")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn time_travel_permanent(game: &mut GameState, controller: PlayerId) -> ObjectId {
    let permanent = CardBuilder::new(CardId::from_raw(80_611), "Time Travel Permanent")
        .card_types(vec![CardType::Artifact])
        .build();
    game.create_object_from_card(&permanent, controller, Zone::Battlefield)
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) struct TimeTravelDecisionMaker {
    pub(super) mode_index: usize,
    pub(super) prompts: usize,
}

#[cfg(ironsmith_runtime_parser_tests)]
impl DecisionMaker for TimeTravelDecisionMaker {
    fn decide_options(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        if ctx.description.starts_with("Choose for time counter") {
            self.prompts += 1;
            vec![self.mode_index.min(ctx.options.len().saturating_sub(1))]
        } else {
            vec![0]
        }
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn all_of_history_all_at_once_adds_time_counters_to_each_eligible_object() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let def = all_of_history_definition();
    let spell_id = game.create_object_from_definition(&def, alice, Zone::Stack);

    let alice_permanent = time_travel_permanent(&mut game, alice);
    game.add_counters(alice_permanent, crate::object::CounterType::Time, 1)
        .expect("eligible controlled permanent should accept a time counter");
    let bob_permanent = time_travel_permanent(&mut game, bob);
    game.add_counters(bob_permanent, crate::object::CounterType::Time, 1)
        .expect("opponent permanent should accept a time counter");

    let suspended = suspended_card_definition();
    let alice_suspended = game.create_object_from_definition(&suspended, alice, Zone::Exile);
    game.add_counters(alice_suspended, crate::object::CounterType::Time, 2)
        .expect("eligible owned suspended card should accept time counters");
    let bob_suspended = game.create_object_from_definition(&suspended, bob, Zone::Exile);
    game.add_counters(bob_suspended, crate::object::CounterType::Time, 2)
        .expect("opponent suspended card should accept time counters");
    let alice_unsuspended = game.create_object_from_definition(&suspended, alice, Zone::Exile);
    game.add_counters(alice_unsuspended, crate::object::CounterType::Charge, 1)
        .expect("ineligible exiled card should accept non-time counters");

    let mut dm = TimeTravelDecisionMaker {
        mode_index: 0,
        prompts: 0,
    };
    let mut ctx =
        crate::effects::ExecutionContext::new_default(spell_id, alice).with_decision_maker(&mut dm);
    crate::effects::execute_effect(&mut game, all_of_history_time_travel_effect(&def), &mut ctx)
        .expect("All of History, All at Once time travel add branch should execute");

    assert_eq!(
        game.counter_count(alice_permanent, crate::object::CounterType::Time),
        2,
        "time travel should add a time counter to each controlled permanent with one"
    );
    assert_eq!(
        game.counter_count(alice_suspended, crate::object::CounterType::Time),
        3,
        "time travel should add a time counter to each owned suspended card"
    );
    assert_eq!(
        game.counter_count(bob_permanent, crate::object::CounterType::Time),
        1,
        "time travel should not affect permanents controlled by opponents"
    );
    assert_eq!(
        game.counter_count(bob_suspended, crate::object::CounterType::Time),
        2,
        "time travel should not affect suspended cards owned by opponents"
    );
    assert_eq!(
        game.counter_count(alice_unsuspended, crate::object::CounterType::Charge),
        1,
        "time travel should not affect exiled cards without time counters"
    );
    assert_eq!(
        dm.prompts, 2,
        "time travel should offer one choice per eligible object"
    );
}
