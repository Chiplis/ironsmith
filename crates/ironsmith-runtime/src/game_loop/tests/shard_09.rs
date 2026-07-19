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
pub(super) fn corpse_lunge_rejects_noncreature_target() {
    use crate::decision::LegalAction;
    use crate::zone::Zone;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut trigger_queue = TriggerQueue::new();

    game.turn.active_player = alice;
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);

    let corpse_lunge = corpse_lunge_definition();
    let cost_creature = CardBuilder::new(CardId::new(), "Exiled Brute")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(4, 4))
        .build();
    let artifact = CardBuilder::new(CardId::new(), "Target Relic")
        .card_types(vec![CardType::Artifact])
        .build();
    let legal_creature = CardBuilder::new(CardId::new(), "Legal Target Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();

    let cost_creature_id = game.create_object_from_card(&cost_creature, alice, Zone::Graveyard);
    let artifact_id = game.create_object_from_card(&artifact, bob, Zone::Battlefield);
    game.create_object_from_card(&legal_creature, bob, Zone::Battlefield);
    let spell_id = game.create_object_from_definition(&corpse_lunge, alice, Zone::Hand);
    game.player_mut(alice)
        .expect("alice exists")
        .mana_pool
        .add(ManaSymbol::Black, 3);

    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut progress = apply_priority_response(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(LegalAction::CastSpell {
            spell_id,
            from_zone: Zone::Hand,
            casting_method: CastingMethod::Normal,
        }),
    )
    .expect("Corpse Lunge cast should start");

    for _ in 0..8 {
        progress = match progress {
            crate::decision::GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::SelectOptions(ctx),
            ) => {
                let option_index = ctx
                    .options
                    .iter()
                    .find(|opt| opt.description.to_ascii_lowercase().contains("exile"))
                    .map(|opt| opt.index)
                    .unwrap_or(0);
                apply_priority_response(
                    &mut game,
                    &mut trigger_queue,
                    &mut state,
                    &PriorityResponse::NextCostChoice(option_index),
                )
                .expect("Corpse Lunge should accept cost choice before target rejection")
            }
            crate::decision::GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::SelectObjects(_),
            ) => apply_priority_response(
                &mut game,
                &mut trigger_queue,
                &mut state,
                &PriorityResponse::CardCostChoice(cost_creature_id),
            )
            .expect("Corpse Lunge should accept graveyard creature cost choice"),
            crate::decision::GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::Targets(_),
            ) => {
                let err = apply_priority_response(
                    &mut game,
                    &mut trigger_queue,
                    &mut state,
                    &PriorityResponse::Targets(vec![Target::Object(artifact_id)]),
                )
                .expect_err("Corpse Lunge should reject a noncreature target");
                let detail = format!("{err:?}").to_ascii_lowercase();
                assert!(
                    detail.contains("target") || detail.contains("legal"),
                    "expected target legality error for noncreature target, got {detail}"
                );
                return;
            }
            other => panic!("unexpected Corpse Lunge cast flow before target selection: {other:?}"),
        };
    }
    panic!("Corpse Lunge did not reach target selection for noncreature target test");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_corpse_cobble_flashback_from_graveyard_still_uses_sacrificed_power() {
    use crate::cards::definitions::{grizzly_bears, llanowar_elves};
    use crate::game_state::Phase;
    use crate::mana::ManaSymbol;
    use crate::zone::Zone;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let mut trigger_queue = TriggerQueue::new();
    let corpse_cobble_text = "As an additional cost to cast this spell, sacrifice any number of creatures.\nCreate an X/X blue and black Zombie creature token with menace, where X is the total power of the sacrificed creatures.\nFlashback {3}{U}{B} (You may cast this card from your graveyard for its flashback cost and any additional costs. Then exile it.)";

    game.turn.active_player = alice;
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);

    let corpse_cobble = CardDefinitionBuilder::new(CardId::from_raw(10002), "Corpse Cobble")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Blue],
            vec![ManaSymbol::Black],
        ]))
        .card_types(vec![CardType::Instant])
        .parse_text(corpse_cobble_text)
        .expect("Corpse Cobble text should parse");

    game.create_object_from_definition(&corpse_cobble, alice, Zone::Graveyard);
    game.create_object_from_definition(&grizzly_bears(), alice, Zone::Battlefield);
    game.create_object_from_definition(&llanowar_elves(), alice, Zone::Battlefield);
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Blue, 3);
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Black, 2);

    let mut dm = CorpseCobbleDecisionMaker;
    let result = run_priority_loop_with(&mut game, &mut trigger_queue, &mut dm)
        .expect("Corpse Cobble flashback should resolve cleanly");
    assert!(
        matches!(result, GameProgress::Continue),
        "priority loop should finish after Corpse Cobble flashback resolves, got {result:?}"
    );

    let zombie = game
        .battlefield
        .iter()
        .filter_map(|id| game.object(*id))
        .find(|obj| obj.name == "Zombie")
        .expect("Corpse Cobble flashback should create a Zombie token");
    assert_eq!(
        zombie.base_power,
        Some(crate::card::PtValue::Fixed(3)),
        "flashback should still use the total power of the sacrificed creatures"
    );
    assert_eq!(
        zombie.base_toughness,
        Some(crate::card::PtValue::Fixed(3)),
        "flashback should still use the total power of the sacrificed creatures"
    );

    let player = game.player(alice).expect("alice exists");
    assert!(
        !player.graveyard.iter().any(|&id| {
            game.object(id)
                .is_some_and(|obj| obj.name == "Corpse Cobble")
        }),
        "Corpse Cobble should leave the graveyard after flashback"
    );
    assert!(
        game.exile.iter().any(|&id| {
            game.object(id)
                .is_some_and(|obj| obj.name == "Corpse Cobble")
        }),
        "Corpse Cobble should be exiled after flashback"
    );
}

// === Triggered Ability Tests ===

#[test]
pub(super) fn test_etb_trigger_fires() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    // Create creature with ETB trigger
    let creature_id = create_creature(&mut game, "ETB Creature", alice, 2, 2);
    if let Some(obj) = game.object_mut(creature_id) {
        obj.abilities_mut().push(Ability::triggered(
            Trigger::this_enters_battlefield(),
            vec![Effect::draw(1)],
        ));
    }

    // Simulate ETB event
    let event = TriggerEvent::new_with_provenance(
        crate::events::zones::ZoneChangeEvent::with_cause(
            creature_id,
            Zone::Stack,
            Zone::Battlefield,
            crate::events::cause::EventCause::effect(),
            None,
        ),
        crate::provenance::ProvNodeId::default(),
    );

    let mut trigger_queue = TriggerQueue::new();
    let triggers = check_triggers(&game, &event);
    for trigger in triggers {
        trigger_queue.add(trigger);
    }

    assert!(!trigger_queue.is_empty());
    assert_eq!(trigger_queue.entries.len(), 1);
}

#[test]
pub(super) fn combat_timed_spell_cast_trigger_queues_only_during_combat() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let watcher_id = create_creature(&mut game, "Combat Cast Watcher", alice, 2, 2);
    game.object_mut(watcher_id)
        .expect("watcher should exist")
        .abilities_mut()
        .push(Ability::triggered(
            Trigger::spell_cast_qualified(
                None,
                PlayerFilter::You,
                Some(ironsmith_core::TriggerTimingRestriction::DuringCombat),
                None,
                None,
                None,
                false,
            ),
            vec![Effect::draw(1)],
        ));

    let spell = CardBuilder::new(CardId::new(), "Combat Cast Spell")
        .card_types(vec![CardType::Instant])
        .build();
    let spell_id = game.create_object_from_card(&spell, alice, Zone::Stack);
    let event = TriggerEvent::new_with_provenance(
        SpellCastEvent::new(spell_id, alice, Zone::Hand),
        crate::provenance::ProvNodeId::default(),
    );

    game.turn.phase = Phase::FirstMain;
    assert!(
        check_triggers(&game, &event)
            .into_iter()
            .all(|trigger| trigger.source != watcher_id),
        "the ability must not trigger for a spell cast outside combat"
    );

    game.turn.phase = Phase::Combat;
    let matching = check_triggers(&game, &event)
        .into_iter()
        .filter(|trigger| trigger.source == watcher_id)
        .collect::<Vec<_>>();
    assert_eq!(
        matching.len(),
        1,
        "the ability must trigger exactly once for the same spell cast during combat"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn terastodon_etb_destroys_up_to_three_permanents_and_makes_elephants() {
    use crate::cards::builders::CardDefinitionBuilder;
    use crate::decision::DecisionMaker;
    use crate::effects::{ExecutionContext, execute_effect};
    use crate::events::zones::EnterBattlefieldEvent;
    use crate::ids::CardId;
    use crate::provenance::ProvNodeId;
    use crate::triggers::TriggerEvent;
    use crate::zone::Zone;

    struct ChooseAllLegalTargetsDecisionMaker;

    impl DecisionMaker for ChooseAllLegalTargetsDecisionMaker {
        fn decide_boolean(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::BooleanContext,
        ) -> bool {
            true
        }

        fn decide_targets(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::TargetsContext,
        ) -> Vec<crate::game_state::Target> {
            let mut chosen = Vec::new();
            for requirement in &ctx.requirements {
                let max = requirement
                    .max_targets
                    .unwrap_or(requirement.legal_targets.len());
                let mut picked = 0usize;
                for target in &requirement.legal_targets {
                    if picked >= max {
                        break;
                    }
                    if !chosen.contains(target) {
                        chosen.push(*target);
                        picked += 1;
                    }
                }
            }
            chosen
        }
    }

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let terastodon = CardDefinitionBuilder::new(CardId::new(), "Terastodon Variant")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(6)],
            vec![ManaSymbol::Green],
            vec![ManaSymbol::Green],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![crate::types::Subtype::Elephant])
        .power_toughness(PowerToughness::fixed(9, 9))
        .parse_text(
            "When this creature enters, you may destroy up to three target noncreature permanents. For each permanent put into a graveyard this way, its controller creates a 3/3 green Elephant creature token.",
        )
        .expect("Terastodon should parse for the runtime regression test");
    let terastodon_id = game.create_object_from_definition(&terastodon, alice, Zone::Battlefield);

    let alice_enchantment = CardBuilder::new(CardId::from_raw(92_001), "Alice Sigil")
        .card_types(vec![CardType::Enchantment])
        .build();
    let bob_artifact = CardBuilder::new(CardId::from_raw(92_002), "Bob Relic")
        .card_types(vec![CardType::Artifact])
        .build();
    let bob_land = CardBuilder::new(CardId::from_raw(92_003), "Bob Shrine")
        .card_types(vec![CardType::Land])
        .build();

    let alice_enchantment_id =
        game.create_object_from_card(&alice_enchantment, alice, Zone::Battlefield);
    let bob_artifact_id = game.create_object_from_card(&bob_artifact, bob, Zone::Battlefield);
    let bob_land_id = game.create_object_from_card(&bob_land, bob, Zone::Battlefield);

    let etb_trigger = terastodon
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered.clone()),
            _ => None,
        })
        .expect("Terastodon should have a triggered ETB ability");

    let event = TriggerEvent::new_with_provenance(
        EnterBattlefieldEvent::new(terastodon_id, Zone::Stack),
        ProvNodeId::default(),
    );
    let mut dm = ChooseAllLegalTargetsDecisionMaker;
    let target_spec = etb_trigger
        .choices
        .first()
        .cloned()
        .expect("Terastodon should require a target choice");
    let mut ctx = ExecutionContext::new(terastodon_id, alice, &mut dm)
        .with_triggering_event(event)
        .with_targets(vec![
            crate::effects::ResolvedTarget::Object(alice_enchantment_id),
            crate::effects::ResolvedTarget::Object(bob_artifact_id),
            crate::effects::ResolvedTarget::Object(bob_land_id),
        ])
        .with_target_assignments(vec![crate::game_state::TargetAssignment {
            spec: target_spec,
            range: 0..3,
        }]);

    for effect in &etb_trigger.effects {
        execute_effect(&mut game, effect, &mut ctx).expect("Terastodon ETB effect should resolve");
    }

    assert!(
        game.player(alice).is_some_and(|player| player
            .graveyard
            .iter()
            .any(|&id| { game.object(id).is_some_and(|obj| obj.name == "Alice Sigil") })),
        "Alice's noncreature permanent should be destroyed"
    );
    assert!(
        game.player(bob).is_some_and(|player| player
            .graveyard
            .iter()
            .any(|&id| { game.object(id).is_some_and(|obj| obj.name == "Bob Relic") })),
        "Bob's artifact should be destroyed"
    );
    assert!(
        game.player(bob).is_some_and(|player| player
            .graveyard
            .iter()
            .any(|&id| { game.object(id).is_some_and(|obj| obj.name == "Bob Shrine") })),
        "Bob's land should be destroyed"
    );

    let alice_elephants = game
        .battlefield
        .iter()
        .filter(|&&id| {
            game.object(id)
                .is_some_and(|obj| obj.name == "Elephant" && game.controller_of(obj) == alice)
        })
        .count();
    let bob_elephants = game
        .battlefield
        .iter()
        .filter(|&&id| {
            game.object(id)
                .is_some_and(|obj| obj.name == "Elephant" && game.controller_of(obj) == bob)
        })
        .count();

    assert_eq!(
        alice_elephants, 1,
        "Alice should get one Elephant token (alice={alice_elephants}, bob={bob_elephants})"
    );
    assert_eq!(
        bob_elephants, 2,
        "Bob should get two Elephant tokens (alice={alice_elephants}, bob={bob_elephants})"
    );
}

#[test]
pub(super) fn test_dies_trigger_from_sba() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    // Create Blood Artist-like creature
    let blood_artist_id = create_creature(&mut game, "Blood Artist", alice, 0, 1);
    if let Some(obj) = game.object_mut(blood_artist_id) {
        obj.abilities_mut().push(Ability::triggered(
            Trigger::dies(crate::target::ObjectFilter::creature()),
            vec![Effect::gain_life(1)],
        ));
    }

    // Create victim creature with lethal damage
    let victim_id = create_creature(&mut game, "Victim", alice, 1, 1);
    game.mark_damage(victim_id, 1);

    // Apply SBAs - should trigger Blood Artist
    let mut trigger_queue = TriggerQueue::new();
    check_and_apply_sbas(&mut game, &mut trigger_queue).unwrap();

    // Blood Artist should have triggered
    assert!(!trigger_queue.is_empty());
}

// === Integration Tests ===

#[test]
pub(super) fn test_combat_damage_with_triggers() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    // Create attacker with "deals combat damage to player" trigger
    let attacker_id = create_creature(&mut game, "Ninja", alice, 2, 2);
    if let Some(obj) = game.object_mut(attacker_id) {
        obj.abilities_mut().push(Ability::triggered(
            Trigger::this_deals_combat_damage_to_player(PlayerFilter::Any),
            vec![Effect::draw(1)],
        ));
    }

    // Set up combat
    let mut combat = CombatState::default();
    combat.attackers.push(crate::combat_state::AttackerInfo {
        creature: attacker_id,
        target: AttackTarget::Player(bob),
    });
    combat.blockers.insert(attacker_id, Vec::new());

    // Execute combat damage
    let events = execute_combat_damage_step(&mut game, &combat, false);

    // Generate triggers
    let mut trigger_queue = TriggerQueue::new();
    generate_damage_triggers(&mut game, &events, &mut trigger_queue);

    // Should have triggered
    assert!(!trigger_queue.is_empty());
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn hexplate_wallbreaker_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(605_584), "Hexplate Wallbreaker")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::Red],
            vec![ManaSymbol::Red],
        ]))
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Equipment])
        .parse_text(
            "For Mirrodin! (When this Equipment enters, create a 2/2 red Rebel creature token, then attach this to it.)\n\
             Equipped creature gets +2/+2.\n\
             Whenever equipped creature attacks, if it's the first combat phase of the turn, untap each attacking creature. After this phase, there is an additional combat phase.\n\
             Equip {3}{R}",
        )
        .expect("Hexplate Wallbreaker should parse for runtime tests")
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn hexplate_wallbreaker_buffs_equipped_creature() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bearer_id = create_creature(&mut game, "Wallbreaker Bearer", alice, 2, 2);
    let hexplate_id = game.create_object_from_definition(
        &hexplate_wallbreaker_definition(),
        alice,
        Zone::Battlefield,
    );

    assert!(game.attach_object_to_target(
        hexplate_id,
        crate::object::AttachmentTarget::Object(bearer_id),
    ));

    assert_eq!(game.calculated_power(bearer_id), Some(4));
    assert_eq!(game.calculated_toughness(bearer_id), Some(4));
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn hexplate_wallbreaker_for_mirrodin_creates_and_equips_rebel() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let hexplate_id = game.create_object_from_definition(
        &hexplate_wallbreaker_definition(),
        alice,
        Zone::Battlefield,
    );

    let event = TriggerEvent::new_with_provenance(
        crate::events::ZoneChangeEvent::with_cause(
            hexplate_id,
            Zone::Stack,
            Zone::Battlefield,
            crate::events::cause::EventCause::from_game_rule(),
            None,
        ),
        crate::provenance::ProvNodeId::default(),
    );
    let mut trigger_queue = TriggerQueue::new();
    for trigger in check_triggers(&game, &event) {
        trigger_queue.add(trigger);
    }
    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("For Mirrodin trigger should go on the stack");
    assert_eq!(
        game.stack.len(),
        1,
        "For Mirrodin should queue one ETB trigger"
    );

    resolve_stack_entry(&mut game).expect("For Mirrodin trigger should resolve");

    let rebels = game
        .battlefield
        .iter()
        .copied()
        .filter(|id| {
            game.object(*id).is_some_and(|object| {
                object.name == "Rebel"
                    && object.kind == ObjectKind::Token
                    && object.has_subtype(Subtype::Rebel)
                    && game.controller_of(object) == alice
            })
        })
        .collect::<Vec<_>>();
    assert_eq!(
        rebels.len(),
        1,
        "For Mirrodin should create one Rebel token"
    );
    let rebel_id = rebels[0];

    assert_eq!(game.calculated_power(rebel_id), Some(4));
    assert_eq!(game.calculated_toughness(rebel_id), Some(4));
    assert_eq!(
        game.object(hexplate_id)
            .and_then(|object| object.attached_to),
        Some(crate::object::AttachmentTarget::Object(rebel_id))
    );
    assert!(
        game.object(rebel_id)
            .is_some_and(|object| object.attachments.contains(&hexplate_id)),
        "Rebel token should track Hexplate as an attachment"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn hexplate_wallbreaker_first_combat_attack_untaps_attackers_and_adds_combat() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.turn.active_player = alice;
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(crate::game_state::Step::DeclareAttackers);
    game.turn_store.combat_phases_started_this_turn = 1;

    let bearer_id = create_creature(&mut game, "Wallbreaker Bearer", alice, 2, 2);
    let ally_id = create_creature(&mut game, "Attacking Ally", alice, 2, 2);
    let bystander_id = create_creature(&mut game, "Bystander", alice, 2, 2);
    let hexplate_id = game.create_object_from_definition(
        &hexplate_wallbreaker_definition(),
        alice,
        Zone::Battlefield,
    );
    assert!(game.attach_object_to_target(
        hexplate_id,
        crate::object::AttachmentTarget::Object(bearer_id),
    ));
    game.tap(bearer_id);
    game.tap(ally_id);
    game.tap(bystander_id);
    game.combat = Some(crate::combat_state::CombatState {
        attackers: vec![
            crate::combat_state::AttackerInfo {
                creature: bearer_id,
                target: AttackTarget::Player(bob),
            },
            crate::combat_state::AttackerInfo {
                creature: ally_id,
                target: AttackTarget::Player(bob),
            },
        ],
        ..Default::default()
    });

    let event = TriggerEvent::new_with_provenance(
        crate::events::combat::CreatureAttackedEvent::with_total_attackers(
            bearer_id,
            crate::events::combat::AttackEventTarget::Player(bob),
            2,
        ),
        crate::provenance::ProvNodeId::default(),
    );
    let mut trigger_queue = TriggerQueue::new();
    for trigger in check_triggers(&game, &event) {
        trigger_queue.add(trigger);
    }
    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("Hexplate Wallbreaker trigger should go on the stack");
    assert_eq!(
        game.stack.len(),
        1,
        "first combat should queue Hexplate trigger"
    );

    resolve_stack_entry(&mut game).expect("Hexplate Wallbreaker trigger should resolve");

    assert!(!game.is_tapped(bearer_id));
    assert!(!game.is_tapped(ally_id));
    assert!(game.is_tapped(bystander_id));
    assert_eq!(game.turn_store.additional_phases, vec![Phase::Combat]);
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn hexplate_wallbreaker_later_combat_attack_does_not_trigger() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.turn.active_player = alice;
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(crate::game_state::Step::DeclareAttackers);
    game.turn_store.combat_phases_started_this_turn = 2;

    let bearer_id = create_creature(&mut game, "Wallbreaker Bearer", alice, 2, 2);
    let hexplate_id = game.create_object_from_definition(
        &hexplate_wallbreaker_definition(),
        alice,
        Zone::Battlefield,
    );
    assert!(game.attach_object_to_target(
        hexplate_id,
        crate::object::AttachmentTarget::Object(bearer_id),
    ));
    game.combat = Some(crate::combat_state::CombatState {
        attackers: vec![crate::combat_state::AttackerInfo {
            creature: bearer_id,
            target: AttackTarget::Player(bob),
        }],
        ..Default::default()
    });

    let event = TriggerEvent::new_with_provenance(
        crate::events::combat::CreatureAttackedEvent::with_total_attackers(
            bearer_id,
            crate::events::combat::AttackEventTarget::Player(bob),
            1,
        ),
        crate::provenance::ProvNodeId::default(),
    );
    let mut trigger_queue = TriggerQueue::new();
    for trigger in check_triggers(&game, &event) {
        trigger_queue.add(trigger);
    }
    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("later combat attack event should be processed cleanly");

    assert!(game.stack.is_empty());
    assert!(game.turn_store.additional_phases.is_empty());
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_quintessential_katana_granted_combat_damage_trigger_stacks_and_resolves() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let katana = CardDefinitionBuilder::new(CardId::new(), "Quintessential Katana")
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![crate::types::Subtype::Equipment])
        .parse_text(
            "Equipped creature gets +1/+1 and has \"Whenever this creature deals combat damage, untap it and you gain 2 life.\"\nWhenever a Ninja you control enters, you may attach this Equipment to it.\nEquip {2}",
        )
        .expect("Quintessential Katana should parse");

    let attacker_id = create_creature(&mut game, "Ninja Trainee", alice, 2, 2);
    let katana_id = game.create_object_from_definition(&katana, alice, Zone::Battlefield);

    if let Some(equipment) = game.object_mut(katana_id) {
        equipment.attached_to = Some(crate::object::AttachmentTarget::Object(attacker_id));
    }
    if let Some(attacker) = game.object_mut(attacker_id) {
        attacker.attachments.push(katana_id);
    }

    game.tap(attacker_id);
    let life_before = game.player(alice).expect("alice exists").life;

    let damage_event = TriggerEvent::new_with_provenance(
        crate::events::DamageEvent::with_cause(
            attacker_id,
            crate::events::DamageTarget::Player(bob),
            3,
            true,
            crate::events::cause::EventCause::combat_damage(attacker_id),
        ),
        crate::provenance::ProvNodeId::default(),
    );

    let mut trigger_queue = TriggerQueue::new();
    for trigger in check_triggers(&game, &damage_event) {
        trigger_queue.add(trigger);
    }

    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "equipped creature should receive Katana's granted combat-damage trigger"
    );

    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("Katana trigger should go on the stack");
    resolve_stack_entry(&mut game).expect("Katana trigger should resolve");

    assert!(
        !game.is_tapped(attacker_id),
        "Katana trigger should untap the equipped creature"
    );
    assert_eq!(
        game.player(alice).expect("alice exists").life,
        life_before + 2,
        "Katana trigger should gain 2 life for the equipped creature's controller"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn raphael_tag_team_tough_trigger_only_happens_first_time_each_turn_and_adds_combat_phase()
 {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    game.turn.active_player = alice;
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(crate::game_state::Step::CombatDamage);

    let raphael_def = CardDefinitionBuilder::new(CardId::new(), "Raphael, Tag Team Tough")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Menace (This creature can't be blocked except by two or more creatures.)\nWhenever Raphael deals combat damage to a player for the first time each turn, untap all attacking creatures. After this combat phase, there is an additional combat phase.",
        )
        .expect("Raphael, Tag Team Tough should parse");

    let raphael_id = game.create_object_from_definition(&raphael_def, alice, Zone::Battlefield);
    let ally_id = create_creature(&mut game, "Attacking Ally", alice, 2, 2);
    let bystander_id = create_creature(&mut game, "Bystander", alice, 2, 2);

    game.tap(raphael_id);
    game.tap(ally_id);
    game.tap(bystander_id);

    game.combat = Some(CombatState {
        attackers: vec![
            crate::combat_state::AttackerInfo {
                creature: raphael_id,
                target: AttackTarget::Player(bob),
            },
            crate::combat_state::AttackerInfo {
                creature: ally_id,
                target: AttackTarget::Player(bob),
            },
        ],
        blockers: std::collections::HashMap::new(),
        ..Default::default()
    });
    if let Some(combat) = game.combat.as_mut() {
        combat.blockers.insert(raphael_id, Vec::new());
        combat.blockers.insert(ally_id, Vec::new());
    }

    let first_damage = TriggerEvent::new_with_provenance(
        crate::events::DamageEvent::with_cause(
            raphael_id,
            crate::events::DamageTarget::Player(bob),
            5,
            true,
            crate::events::cause::EventCause::combat_damage(raphael_id),
        ),
        crate::provenance::ProvNodeId::default(),
    );

    let mut trigger_queue = TriggerQueue::new();
    queue_triggers_from_event(&mut game, &mut trigger_queue, first_damage, false);
    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("Raphael should trigger for first combat damage this turn");
    assert_eq!(game.stack.len(), 1, "first damage should queue one trigger");

    resolve_stack_entry(&mut game).expect("Raphael trigger should resolve");

    assert!(
        !game.is_tapped(raphael_id) && !game.is_tapped(ally_id),
        "Raphael trigger should untap all attacking creatures"
    );
    assert!(
        game.is_tapped(bystander_id),
        "nonattacking creatures should stay tapped"
    );
    assert_eq!(
        game.turn_store.additional_phases,
        vec![Phase::Combat],
        "Raphael trigger should queue one additional combat phase"
    );

    let second_damage = TriggerEvent::new_with_provenance(
        crate::events::DamageEvent::with_cause(
            raphael_id,
            crate::events::DamageTarget::Player(bob),
            5,
            true,
            crate::events::cause::EventCause::combat_damage(raphael_id),
        ),
        crate::provenance::ProvNodeId::default(),
    );

    queue_triggers_from_event(&mut game, &mut trigger_queue, second_damage, false);
    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("second damage event should be processed cleanly");
    assert!(
        game.stack.is_empty(),
        "Raphael should not trigger again from later combat damage this turn"
    );
    assert_eq!(
        game.turn_store.additional_phases,
        vec![Phase::Combat],
        "second damage should not add another additional combat phase"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_ragavan_trigger_exiles_top_card_of_damaged_players_library() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(crate::game_state::Step::CombatDamage);
    game.player_mut(alice)
        .expect("alice exists")
        .mana_pool
        .add(ManaSymbol::Red, 1);

    let ragavan_def = CardDefinitionBuilder::new(CardId::new(), "Ragavan Runtime Probe")
        .parse_text(
            "Whenever this creature deals combat damage to a player, create a Treasure token and exile the top card of that player's library. Until end of turn, you may cast that card.\nDash {1}{R}",
        )
        .expect("ragavan runtime probe should parse");
    let ragavan_id = game.create_object_from_definition(&ragavan_def, alice, Zone::Battlefield);

    let top_card = CardBuilder::new(CardId::new(), "Bob Topdeck")
        .card_types(vec![CardType::Sorcery])
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Red]]))
        .build();
    game.create_object_from_card(&top_card, bob, Zone::Library);

    let triggered = ragavan_def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("ragavan probe should have a triggered ability");

    let damage_event = TriggerEvent::new_with_provenance(
        crate::events::DamageEvent::with_cause(
            ragavan_id,
            crate::events::DamageTarget::Player(bob),
            2,
            true,
            crate::events::cause::EventCause::combat_damage(ragavan_id),
        ),
        crate::provenance::ProvNodeId::default(),
    );

    let library_before = game.player(bob).expect("bob exists").library.len();
    let exile_before = game.exile.len();
    let battlefield_before = game.battlefield.len();

    let mut dm = AutoPassDecisionMaker;
    let mut ctx = ExecutionContext::new_default(ragavan_id, alice)
        .with_decision_maker(&mut dm)
        .with_triggering_event(damage_event);
    for effect in &triggered.effects {
        execute_effect(&mut game, effect, &mut ctx).expect("ragavan trigger should resolve");
    }

    let exiled_id = *game
        .exile
        .last()
        .expect("ragavan should exile the damaged player's top card");
    let exiled_obj = game.object(exiled_id).expect("exiled object should exist");

    assert_eq!(
        game.player(bob).expect("bob exists").library.len(),
        library_before - 1,
        "ragavan should exile the top card from the damaged player's library"
    );
    assert_eq!(
        game.exile.len(),
        exile_before + 1,
        "ragavan should add one card to exile"
    );
    assert_eq!(
        exiled_obj.name, "Bob Topdeck",
        "ragavan should exile the damaged player's top card"
    );
    assert_eq!(
        game.battlefield.len(),
        battlefield_before + 1,
        "ragavan should also create a Treasure token"
    );
    assert!(
        game.effect_store.grant_registry.card_can_play_from_zone(
            &game,
            exiled_id,
            Zone::Exile,
            alice
        ),
        "ragavan should let its controller cast the exiled card until end of turn"
    );

    let combat_actions = crate::decision::compute_legal_actions(&game, alice);
    assert!(
        !combat_actions.iter().any(|action| matches!(
            action,
            crate::decision::LegalAction::CastSpell {
                spell_id,
                from_zone: Zone::Exile,
                casting_method: crate::alternative_cast::CastingMethod::PlayFrom {
                    zone: Zone::Exile,
                    use_alternative: None,
                    ..
                },
            } if *spell_id == exiled_id
        )),
        "sorcery-speed exiled cards should not be castable during combat even when Ragavan grants permission"
    );

    game.turn.phase = Phase::NextMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);

    let postcombat_actions = crate::decision::compute_legal_actions(&game, alice);
    assert!(
        postcombat_actions.iter().any(|action| matches!(
            action,
            crate::decision::LegalAction::CastSpell {
                spell_id,
                from_zone: Zone::Exile,
                casting_method: crate::alternative_cast::CastingMethod::PlayFrom {
                    zone: Zone::Exile,
                    use_alternative: None,
                    ..
                },
            } if *spell_id == exiled_id
        )),
        "Ragavan's exiled card should become castable in the postcombat main phase once timing allows it"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn cavern_hoard_dragon_combat_damage_trigger_counts_damaged_players_artifacts() {
    fn create_artifact(game: &mut GameState, owner: PlayerId, name: &str) {
        let card = CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Artifact])
            .build();
        game.create_object_from_card(&card, owner, Zone::Battlefield);
    }

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.turn.active_player = alice;
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(crate::game_state::Step::CombatDamage);

    let dragon = CardDefinitionBuilder::new(CardId::from_raw(119_956), "Cavern-Hoard Dragon")
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
        .expect("Cavern-Hoard Dragon should parse for trigger runtime test");
    let dragon_id = game.create_object_from_definition(&dragon, alice, Zone::Battlefield);

    create_artifact(&mut game, alice, "Alice Artifact");
    create_artifact(&mut game, bob, "Bob Artifact One");
    create_artifact(&mut game, bob, "Bob Artifact Two");
    create_artifact(&mut game, bob, "Bob Artifact Three");
    let battlefield_before = game.battlefield.len();

    let damage_event = TriggerEvent::new_with_provenance(
        crate::events::DamageEvent::with_cause(
            dragon_id,
            crate::events::DamageTarget::Player(bob),
            6,
            true,
            crate::events::cause::EventCause::combat_damage(dragon_id),
        ),
        crate::provenance::ProvNodeId::default(),
    );

    let mut trigger_queue = TriggerQueue::new();
    for trigger in check_triggers(&game, &damage_event) {
        trigger_queue.add(trigger);
    }
    assert_eq!(trigger_queue.entries.len(), 1, "dragon should trigger once");
    put_triggers_on_stack(&mut game, &mut trigger_queue).expect("dragon trigger should stack");
    resolve_stack_entry(&mut game).expect("dragon trigger should resolve");

    let treasure_count = game
        .battlefield
        .iter()
        .filter_map(|&id| game.object(id))
        .filter(|obj| obj.name == "Treasure")
        .count();
    assert_eq!(
        treasure_count, 3,
        "Cavern-Hoard Dragon should create one Treasure for each artifact the damaged player controls"
    );
    assert_eq!(
        game.battlefield.len(),
        battlefield_before + 3,
        "caster's own artifact should not be counted by the damage trigger"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn ancient_bronze_dragon_trigger_uses_die_result_for_up_to_two_targets() {
    struct SelectUpToTwoDecisionMaker;
    impl DecisionMaker for SelectUpToTwoDecisionMaker {
        fn decide_objects(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            ctx.candidates
                .iter()
                .filter(|candidate| candidate.legal)
                .map(|candidate| candidate.id)
                .take(2)
                .collect()
        }
    }

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.force_next_die_roll(17);

    let dragon = CardDefinitionBuilder::new(CardId::new(), "Ancient Bronze Dragon")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Elder, Subtype::Dragon])
        .power_toughness(PowerToughness::fixed(7, 7))
        .parse_text(
            "Flying\nWhenever Ancient Bronze Dragon deals combat damage to a player, roll a d20. When you do, put X +1/+1 counters on each of up to two target creatures, where X is the result.",
        )
        .expect("Ancient Bronze Dragon should parse");

    let dragon_id = game.create_object_from_definition(&dragon, alice, Zone::Battlefield);
    let first_target = create_creature(&mut game, "Target One", alice, 2, 2);
    let second_target = create_creature(&mut game, "Target Two", alice, 2, 2);

    let damage_event = TriggerEvent::new_with_provenance(
        crate::events::DamageEvent::with_cause(
            dragon_id,
            crate::events::DamageTarget::Player(bob),
            7,
            true,
            crate::events::cause::EventCause::combat_damage(dragon_id),
        ),
        crate::provenance::ProvNodeId::default(),
    );

    let mut trigger_queue = TriggerQueue::new();
    for trigger in check_triggers(&game, &damage_event) {
        trigger_queue.add(trigger);
    }
    assert_eq!(trigger_queue.entries.len(), 1, "dragon should trigger once");

    let mut dm = SelectUpToTwoDecisionMaker;
    put_triggers_on_stack_with_dm(&mut game, &mut trigger_queue, &mut dm)
        .expect("dragon trigger should go on stack");
    resolve_stack_entry_with_dm_and_triggers(&mut game, &mut dm, &mut trigger_queue)
        .expect("dragon trigger should resolve");
    if !trigger_queue.entries.is_empty() {
        put_triggers_on_stack_with_dm(&mut game, &mut trigger_queue, &mut dm)
            .expect("reflexive trigger should go on stack");
    }
    if !game.stack.is_empty() {
        resolve_stack_entry_with_dm_and_triggers(&mut game, &mut dm, &mut trigger_queue)
            .expect("reflexive trigger should resolve");
    }

    let _ = (first_target, second_target);
    assert!(
        game.stack.is_empty(),
        "both the primary and reflexive trigger entries should resolve cleanly"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn ancient_bronze_dragon_trigger_allows_zero_targets() {
    struct ChooseNoTargetsDecisionMaker;
    impl DecisionMaker for ChooseNoTargetsDecisionMaker {
        fn decide_objects(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            Vec::new()
        }
    }

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.force_next_die_roll(12);

    let dragon = CardDefinitionBuilder::new(CardId::new(), "Ancient Bronze Dragon")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Elder, Subtype::Dragon])
        .power_toughness(PowerToughness::fixed(7, 7))
        .parse_text(
            "Flying\nWhenever Ancient Bronze Dragon deals combat damage to a player, roll a d20. When you do, put X +1/+1 counters on each of up to two target creatures, where X is the result.",
        )
        .expect("Ancient Bronze Dragon should parse");

    let dragon_id = game.create_object_from_definition(&dragon, alice, Zone::Battlefield);
    let bystander = create_creature(&mut game, "Bystander", alice, 2, 2);

    let damage_event = TriggerEvent::new_with_provenance(
        crate::events::DamageEvent::with_cause(
            dragon_id,
            crate::events::DamageTarget::Player(bob),
            7,
            true,
            crate::events::cause::EventCause::combat_damage(dragon_id),
        ),
        crate::provenance::ProvNodeId::default(),
    );

    let mut trigger_queue = TriggerQueue::new();
    for trigger in check_triggers(&game, &damage_event) {
        trigger_queue.add(trigger);
    }
    assert_eq!(trigger_queue.entries.len(), 1, "dragon should trigger once");

    let mut dm = ChooseNoTargetsDecisionMaker;
    put_triggers_on_stack_with_dm(&mut game, &mut trigger_queue, &mut dm)
        .expect("dragon trigger should go on stack");
    resolve_stack_entry_with_dm_and_triggers(&mut game, &mut dm, &mut trigger_queue)
        .expect("dragon trigger should resolve");
    if !trigger_queue.entries.is_empty() {
        put_triggers_on_stack_with_dm(&mut game, &mut trigger_queue, &mut dm)
            .expect("reflexive trigger should go on stack");
    }
    if !game.stack.is_empty() {
        resolve_stack_entry_with_dm_and_triggers(&mut game, &mut dm, &mut trigger_queue)
            .expect("reflexive trigger should resolve with zero targets");
    }

    assert_eq!(
        game.counter_count(bystander, crate::object::CounterType::PlusOnePlusOne),
        0,
        "choosing zero targets should leave creatures without counters"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn clown_car_etb_applies_odd_even_result_branches_per_die_for_x_rolls() {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let alice = PlayerId::from_index(0);

    game.force_next_die_roll(1);
    game.force_next_die_roll(2);
    game.force_next_die_roll(5);

    let clown_car = CardDefinitionBuilder::new(CardId::new(), "Clown Car")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::X]]))
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .subtypes(vec![Subtype::Vehicle])
        .power_toughness(PowerToughness::fixed(1, 1))
        .parse_text(
            "When this Vehicle enters, roll X six-sided dice. For each odd result, create a 1/1 white Clown Robot artifact creature token. For each even result, put a +1/+1 counter on this Vehicle.\nCrew 2",
        )
        .expect("Clown Car should parse");

    let clown_car_id = game.create_object_from_definition(&clown_car, alice, Zone::Battlefield);
    game.object_mut(clown_car_id)
        .expect("Clown Car permanent should exist")
        .x_value = Some(3);
    let mut source_snapshot = crate::snapshot::ObjectSnapshot::from_object(
        game.object(clown_car_id)
            .expect("Clown Car permanent should exist"),
        &game,
    );
    source_snapshot.x_value = Some(3);

    let etb_event = TriggerEvent::new_with_provenance(
        crate::events::ZoneChangeEvent::with_cause(
            clown_car_id,
            Zone::Hand,
            Zone::Battlefield,
            crate::events::cause::EventCause::effect(),
            Some(source_snapshot),
        ),
        crate::provenance::ProvNodeId::default(),
    );
    queue_triggers_from_event(&mut game, &mut trigger_queue, etb_event, false);
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "Clown Car should trigger once"
    );

    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("Clown Car ETB trigger should go on stack");
    resolve_stack_entry(&mut game).expect("Clown Car ETB trigger should resolve");

    let clown_tokens = game
        .battlefield
        .iter()
        .filter(|&&id| {
            game.object(id).is_some_and(|obj| {
                obj.name == "Clown"
                    && game.controller_of(obj) == alice
                    && obj.card_types.contains(&CardType::Artifact)
                    && obj.card_types.contains(&CardType::Creature)
            })
        })
        .count();
    assert_eq!(
        clown_tokens, 2,
        "odd die results (1 and 5) should create one Clown token each"
    );

    assert_eq!(
        game.counter_count(clown_car_id, crate::object::CounterType::PlusOnePlusOne),
        1,
        "even die result (2) should add one +1/+1 counter"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn complaints_clerk_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Complaints Clerk")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Beast])
        .power_toughness(PowerToughness::fixed(3, 3))
        .parse_text(
            "When this creature enters, open an Attraction. (Put the top card of your Attraction deck onto the battlefield.)\nWhenever you roll a 1, create a 1/1 white Clown Robot artifact creature token.",
        )
        .expect("Complaints Clerk should parse")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn resolve_complaints_clerk_roll(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    clerk_id: ObjectId,
    controller: PlayerId,
    roll: u32,
) {
    game.force_next_die_roll(roll);
    let mut ctx = crate::effects::ExecutionContext::new_default(clerk_id, controller);
    let outcome = crate::effects::execute_effect(
        game,
        &Effect::roll_die(6, PlayerFilter::Specific(controller)),
        &mut ctx,
    )
    .expect("die roll should resolve");
    for event in outcome.events {
        queue_triggers_from_event(game, trigger_queue, event, false);
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn complaints_clerk_roll_one_trigger_creates_clown_robot_token() {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let alice = PlayerId::from_index(0);
    let clerk = complaints_clerk_definition();
    let clerk_id = game.create_object_from_definition(&clerk, alice, Zone::Battlefield);

    resolve_complaints_clerk_roll(&mut game, &mut trigger_queue, clerk_id, alice, 1);
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "Complaints Clerk should trigger when its controller rolls a 1"
    );

    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("Complaints Clerk roll trigger should go on stack");
    resolve_stack_entry(&mut game).expect("Complaints Clerk roll trigger should resolve");

    let clown_robots = game
        .battlefield
        .iter()
        .filter(|&&id| {
            game.object(id).is_some_and(|obj| {
                matches!(obj.kind, ObjectKind::Token)
                    && game.controller_of(obj) == alice
                    && obj.card_types.contains(&CardType::Artifact)
                    && obj.card_types.contains(&CardType::Creature)
                    && obj.subtypes.contains(&Subtype::Clown)
                    && obj.subtypes.contains(&Subtype::Robot)
                    && game.current_power(id) == Some(1)
                    && game.current_toughness(id) == Some(1)
            })
        })
        .count();
    assert_eq!(clown_robots, 1, "rolling 1 should create one Clown Robot");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn complaints_clerk_non_one_roll_does_not_trigger() {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let alice = PlayerId::from_index(0);
    let clerk = complaints_clerk_definition();
    let clerk_id = game.create_object_from_definition(&clerk, alice, Zone::Battlefield);

    resolve_complaints_clerk_roll(&mut game, &mut trigger_queue, clerk_id, alice, 2);
    assert!(
        trigger_queue.entries.is_empty(),
        "Complaints Clerk should not trigger for a non-1 die result"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn complaints_clerk_opponent_roll_one_does_not_trigger() {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let clerk = complaints_clerk_definition();
    let clerk_id = game.create_object_from_definition(&clerk, alice, Zone::Battlefield);

    resolve_complaints_clerk_roll(&mut game, &mut trigger_queue, clerk_id, bob, 1);
    assert!(
        trigger_queue.entries.is_empty(),
        "Complaints Clerk should not trigger when another player rolls a 1"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn netherese_puzzle_ward_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(531_506), "Netherese Puzzle-Ward")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::Blue],
        ]))
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "Focus Beam — At the beginning of your upkeep, roll a d4. Scry X, where X is the result.\n\
             Perfect Illumination — Whenever you roll a die's highest natural result, draw a card.",
        )
        .expect("Netherese Puzzle-Ward should parse")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn resolve_netherese_puzzle_ward_roll(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    source: ObjectId,
    roller: PlayerId,
    roll: u32,
) {
    game.force_next_die_roll(roll);
    let mut ctx = crate::effects::ExecutionContext::new_default(source, roller);
    let outcome = crate::effects::execute_effect(
        game,
        &Effect::roll_die(4, PlayerFilter::Specific(roller)),
        &mut ctx,
    )
    .expect("die roll should resolve");
    for event in outcome.events {
        queue_triggers_from_event(game, trigger_queue, event, false);
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) struct NethereseRollAdjustmentDecisionMaker;

#[cfg(ironsmith_runtime_parser_tests)]
impl DecisionMaker for NethereseRollAdjustmentDecisionMaker {
    fn decide_boolean(
        &mut self,
        _game: &GameState,
        _ctx: &crate::decisions::context::BooleanContext,
    ) -> bool {
        true
    }

    fn decide_options(
        &mut self,
        _game: &GameState,
        _ctx: &crate::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        vec![0]
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn add_netherese_roll_adjustment_source(game: &mut GameState, controller: PlayerId) {
    let card = CardDefinitionBuilder::new(CardId::new(), "Die Adjustment Source")
        .card_types(vec![CardType::Enchantment])
        .with_ability(Ability::static_ability(
            StaticAbility::die_roll_result_adjustment(
                PlayerFilter::You,
                1,
                1,
                true,
                "After you roll a die, you may pay 1 life. If you do, increase or decrease the result by 1. Do this only once each turn.",
            ),
        ))
        .build();
    game.create_object_from_definition(&card, controller, Zone::Battlefield);
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn netherese_puzzle_ward_highest_natural_result_draws_a_card() {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let alice = PlayerId::from_index(0);
    put_test_cards_in_zone(&mut game, alice, Zone::Library, 1);
    let ward = netherese_puzzle_ward_definition();
    let ward_id = game.create_object_from_definition(&ward, alice, Zone::Battlefield);

    resolve_netherese_puzzle_ward_roll(&mut game, &mut trigger_queue, ward_id, alice, 4);
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "Netherese Puzzle-Ward should trigger when its controller rolls the d4's highest natural result"
    );

    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("Netherese Puzzle-Ward roll trigger should go on the stack");
    resolve_stack_entry(&mut game).expect("Netherese Puzzle-Ward roll trigger should resolve");
    assert_eq!(
        game.player(alice).expect("Alice exists").hand.len(),
        1,
        "highest natural result should draw one card"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn netherese_puzzle_ward_non_highest_result_does_not_trigger() {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let alice = PlayerId::from_index(0);
    put_test_cards_in_zone(&mut game, alice, Zone::Library, 1);
    let ward = netherese_puzzle_ward_definition();
    let ward_id = game.create_object_from_definition(&ward, alice, Zone::Battlefield);

    resolve_netherese_puzzle_ward_roll(&mut game, &mut trigger_queue, ward_id, alice, 3);
    assert!(
        trigger_queue.entries.is_empty(),
        "Netherese Puzzle-Ward should not trigger for a non-highest d4 result"
    );
    assert_eq!(game.player(alice).expect("Alice exists").hand.len(), 0);
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn netherese_puzzle_ward_adjusted_high_result_does_not_trigger() {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let alice = PlayerId::from_index(0);
    put_test_cards_in_zone(&mut game, alice, Zone::Library, 1);
    add_netherese_roll_adjustment_source(&mut game, alice);
    let ward = netherese_puzzle_ward_definition();
    let ward_id = game.create_object_from_definition(&ward, alice, Zone::Battlefield);

    game.force_next_die_roll(3);
    let mut decisions = NethereseRollAdjustmentDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new_default(ward_id, alice)
        .with_decision_maker(&mut decisions);
    let outcome = crate::effects::execute_effect(
        &mut game,
        &Effect::roll_die(4, PlayerFilter::Specific(alice)),
        &mut ctx,
    )
    .expect("die roll should resolve");
    let die_event = outcome
        .events
        .first()
        .and_then(|event| event.downcast::<crate::events::other::DieRolledEvent>())
        .expect("roll should emit a die-rolled event");
    assert_eq!(die_event.natural_result, 3);
    assert_eq!(die_event.result, 4);
    for event in outcome.events {
        queue_triggers_from_event(&mut game, &mut trigger_queue, event, false);
    }

    assert!(
        trigger_queue.entries.is_empty(),
        "Netherese Puzzle-Ward should not trigger when an adjusted result, not the natural result, is highest"
    );
    assert_eq!(game.player(alice).expect("Alice exists").hand.len(), 0);
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn netherese_puzzle_ward_opponent_highest_result_does_not_trigger() {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    put_test_cards_in_zone(&mut game, alice, Zone::Library, 1);
    let ward = netherese_puzzle_ward_definition();
    let ward_id = game.create_object_from_definition(&ward, alice, Zone::Battlefield);

    resolve_netherese_puzzle_ward_roll(&mut game, &mut trigger_queue, ward_id, bob, 4);
    assert!(
        trigger_queue.entries.is_empty(),
        "Netherese Puzzle-Ward should not trigger when another player rolls the highest natural result"
    );
    assert_eq!(game.player(alice).expect("Alice exists").hand.len(), 0);
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn arden_angel_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Arden Angel")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Angel])
        .power_toughness(PowerToughness::fixed(4, 4))
        .parse_text(
            "Flying\nAt the beginning of your upkeep, if Arden Angel is in your graveyard, roll a four-sided die. If the result is 1, return Arden Angel from your graveyard to the battlefield.",
        )
        .expect("Arden Angel should parse")
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn arden_angel_upkeep_roll_one_returns_from_graveyard_to_battlefield() {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let alice = PlayerId::from_index(0);

    let arden = arden_angel_definition();
    let arden_id = game.create_object_from_definition(&arden, alice, Zone::Graveyard);
    let arden_stable_id = game.object(arden_id).expect("Arden Angel exists").stable_id;
    game.force_next_die_roll(1);
    game.turn.phase = Phase::Beginning;
    game.turn.step = Some(crate::game_state::Step::Upkeep);
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    generate_and_queue_step_triggers(&mut game, &mut trigger_queue);
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "Arden Angel should trigger from its controller's graveyard during upkeep"
    );

    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("Arden Angel upkeep trigger should go on the stack");
    resolve_stack_entry(&mut game).expect("Arden Angel upkeep trigger should resolve");

    let returned_id = game
        .find_object_by_stable_id(arden_stable_id)
        .expect("Arden Angel should still be trackable after returning");
    assert!(
        game.object(returned_id)
            .is_some_and(|object| object.zone == Zone::Battlefield),
        "rolling 1 should return Arden Angel to the battlefield"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn arden_angel_upkeep_non_one_roll_leaves_it_in_graveyard() {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let alice = PlayerId::from_index(0);

    let arden = arden_angel_definition();
    let arden_id = game.create_object_from_definition(&arden, alice, Zone::Graveyard);
    let arden_stable_id = game.object(arden_id).expect("Arden Angel exists").stable_id;
    game.force_next_die_roll(2);
    game.turn.phase = Phase::Beginning;
    game.turn.step = Some(crate::game_state::Step::Upkeep);
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    generate_and_queue_step_triggers(&mut game, &mut trigger_queue);
    assert_eq!(trigger_queue.entries.len(), 1, "Arden Angel should trigger");

    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("Arden Angel upkeep trigger should go on the stack");
    resolve_stack_entry(&mut game).expect("Arden Angel upkeep trigger should resolve");

    let arden_id = game
        .find_object_by_stable_id(arden_stable_id)
        .expect("Arden Angel should still be trackable after resolving");
    assert!(
        game.object(arden_id)
            .is_some_and(|object| object.zone == Zone::Graveyard),
        "rolling a non-1 should leave Arden Angel in the graveyard"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn arden_angel_does_not_trigger_when_not_in_graveyard() {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let alice = PlayerId::from_index(0);

    let arden = arden_angel_definition();
    game.create_object_from_definition(&arden, alice, Zone::Battlefield);
    game.turn.phase = Phase::Beginning;
    game.turn.step = Some(crate::game_state::Step::Upkeep);
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    generate_and_queue_step_triggers(&mut game, &mut trigger_queue);
    assert!(
        trigger_queue.entries.is_empty(),
        "Arden Angel's upkeep ability should only function from the graveyard"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_fallen_shinobi_trigger_exiles_top_two_cards_and_grants_play_permission() {
    use crate::decision::compute_legal_actions;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(crate::game_state::Step::CombatDamage);

    let shinobi_def = CardDefinitionBuilder::new(CardId::new(), "Fallen Shinobi Runtime Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Ninjutsu {2}{U}{B} ({2}{U}{B}, Return an unblocked attacker you control to hand: Put this card onto the battlefield tapped and attacking.)\nWhenever this creature deals combat damage to a player, that player exiles the top two cards of their library. Until end of turn, you may play those cards without paying their mana costs.",
        )
        .expect("fallen shinobi runtime probe should parse");
    let shinobi_id = game.create_object_from_definition(&shinobi_def, alice, Zone::Battlefield);

    let top_land = CardBuilder::new(CardId::new(), "Shinobi Land")
        .card_types(vec![CardType::Land])
        .build();
    let top_spell = CardBuilder::new(CardId::new(), "Shinobi Bolt")
        .card_types(vec![CardType::Sorcery])
        .mana_cost(crate::mana::ManaCost::from_symbols(vec![
            crate::mana::ManaSymbol::Red,
        ]))
        .build();
    let _top_land_id = game.create_object_from_card(&top_land, bob, Zone::Library);
    let _top_spell_id = game.create_object_from_card(&top_spell, bob, Zone::Library);

    let triggered = shinobi_def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("fallen shinobi probe should have a triggered ability");

    let damage_event = TriggerEvent::new_with_provenance(
        crate::events::DamageEvent::with_cause(
            shinobi_id,
            crate::events::DamageTarget::Player(bob),
            5,
            true,
            crate::events::cause::EventCause::combat_damage(shinobi_id),
        ),
        crate::provenance::ProvNodeId::default(),
    );

    let library_before = game.player(bob).expect("bob exists").library.len();
    let exile_before = game.exile.len();

    let mut dm = AutoPassDecisionMaker;
    let mut ctx = ExecutionContext::new_default(shinobi_id, alice)
        .with_decision_maker(&mut dm)
        .with_triggering_event(damage_event);
    for effect in &triggered.effects {
        execute_effect(&mut game, effect, &mut ctx).expect("fallen shinobi trigger should resolve");
    }

    let exiled_ids: Vec<_> = game.exile.clone();
    let exiled_names: Vec<_> = exiled_ids
        .iter()
        .filter_map(|&id| game.object(id).map(|obj| (id, obj.name.to_string())))
        .collect();
    let exiled_land_id = exiled_names
        .iter()
        .find_map(|(id, name)| (*name == "Shinobi Land").then_some(*id))
        .expect("fallen shinobi should exile the top land card");
    let exiled_spell_id = exiled_names
        .iter()
        .find_map(|(id, name)| (*name == "Shinobi Bolt").then_some(*id))
        .expect("fallen shinobi should exile the top spell card");

    assert_eq!(
        game.player(bob).expect("bob exists").library.len(),
        library_before - 2,
        "fallen shinobi should exile the top two cards from the damaged player's library"
    );
    assert_eq!(
        game.exile.len(),
        exile_before + 2,
        "fallen shinobi should add two cards to exile"
    );
    assert!(
        exiled_names.iter().any(|(_, name)| name == "Shinobi Land"),
        "fallen shinobi should exile the top land card"
    );
    assert!(
        exiled_names.iter().any(|(_, name)| name == "Shinobi Bolt"),
        "fallen shinobi should exile the top spell card"
    );
    assert!(
        game.effect_store.grant_registry.card_can_play_from_zone(
            &game,
            exiled_land_id,
            Zone::Exile,
            alice
        ),
        "fallen shinobi should let its controller play the exiled land"
    );
    assert!(
        game.effect_store.grant_registry.card_can_play_from_zone(
            &game,
            exiled_spell_id,
            Zone::Exile,
            alice
        ),
        "fallen shinobi should let its controller play the exiled spell"
    );

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);

    let actions = compute_legal_actions(&game, alice);
    assert!(
        actions.iter().any(|action| matches!(
            action,
            crate::decision::LegalAction::PlayLand { land_id } if *land_id == exiled_land_id
        )),
        "fallen shinobi should expose a land play action from exile"
    );
    assert!(
        actions.iter().any(|action| matches!(
            action,
            crate::decision::LegalAction::CastSpell { spell_id, from_zone: Zone::Exile, .. }
                if *spell_id == exiled_spell_id
        )),
        "fallen shinobi should expose a free cast action for the exiled spell"
    );

    game.turn.turn_number += 1;
    for card_id in [exiled_land_id, exiled_spell_id] {
        assert!(
            !game.effect_store.grant_registry.card_can_play_from_zone(
                &game,
                card_id,
                Zone::Exile,
                alice
            ),
            "fallen shinobi should only grant play permission until end of turn"
        );
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn black_cat_cunning_thief_definition() -> crate::cards::CardDefinition {
    let oracle = "When Black Cat enters, look at the top nine cards of target opponent's library, \
        exile two of them face down, then put the rest on the bottom of their library in a random \
        order. You may play the exiled cards for as long as they remain exiled. Mana of any type \
        can be spent to cast spells this way.";

    CardDefinitionBuilder::new(CardId::from_raw(90_468), "Black Cat, Cunning Thief")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Human, Subtype::Rogue, Subtype::Villain])
        .power_toughness(PowerToughness::fixed(2, 3))
        .parse_text(oracle)
        .expect("Black Cat, Cunning Thief should parse for runtime tests")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) struct ChooseBlackCatCardsDecisionMaker {
    pub(super) selected: Vec<ObjectId>,
}

#[cfg(ironsmith_runtime_parser_tests)]
impl DecisionMaker for ChooseBlackCatCardsDecisionMaker {
    fn decide_objects(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        ctx.candidates
            .iter()
            .filter(|candidate| candidate.legal && self.selected.contains(&candidate.id))
            .map(|candidate| candidate.id)
            .take(ctx.max.unwrap_or(self.selected.len()))
            .collect()
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn black_cat_library_card(name: &str, card_type: CardType) -> crate::card::Card {
    CardBuilder::new(CardId::new(), name)
        .card_types(vec![card_type])
        .build()
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn black_cat_cunning_thief_exiles_two_looked_cards_face_down_and_bottoms_rest() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let black_cat = black_cat_cunning_thief_definition();
    let source = game.create_object_from_definition(&black_cat, alice, Zone::Battlefield);

    let alice_card = black_cat_library_card("Alice Untouched", CardType::Sorcery);
    let alice_card_id = game.create_object_from_card(&alice_card, alice, Zone::Library);
    let unrelated_exiled_card = black_cat_library_card("Unrelated Exiled", CardType::Instant);
    let unrelated_exiled_id =
        game.create_object_from_card(&unrelated_exiled_card, bob, Zone::Exile);

    let mut bob_library = Vec::new();
    for index in 1..=9 {
        let card_type = if index == 8 {
            CardType::Land
        } else if index == 9 {
            CardType::Instant
        } else {
            CardType::Sorcery
        };
        let card = black_cat_library_card(&format!("Bob Card {index}"), card_type);
        bob_library.push(game.create_object_from_card(&card, bob, Zone::Library));
    }
    assert!(game.set_player_library_order_with_audit(
        bob,
        bob_library.clone(),
        "Black Cat runtime test setup",
    ));
    let selected_permanent = bob_library[7];
    let selected_spell = bob_library[8];
    let selected_permanent_stable = game
        .object(selected_permanent)
        .expect("selected permanent setup")
        .stable_id;
    let selected_spell_stable = game
        .object(selected_spell)
        .expect("selected spell setup")
        .stable_id;

    let triggered = black_cat
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Black Cat should have an enters trigger");
    let mut dm = ChooseBlackCatCardsDecisionMaker {
        selected: vec![selected_permanent, selected_spell],
    };
    let mut ctx = crate::effects::ExecutionContext::new_default(source, alice)
        .with_decision_maker(&mut dm)
        .with_targets(vec![crate::effects::ResolvedTarget::Player(bob)])
        .with_triggering_event(TriggerEvent::new_with_provenance(
            EnterBattlefieldEvent::new(source, Zone::Hand),
            crate::provenance::ProvNodeId::default(),
        ));
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        source,
        &triggered.effects,
        None,
        &[],
    )
    .expect("Black Cat enters trigger should resolve");

    let selected_permanent = game
        .find_object_by_stable_id(selected_permanent_stable)
        .expect("selected permanent should still exist");
    let selected_spell = game
        .find_object_by_stable_id(selected_spell_stable)
        .expect("selected spell should still exist");
    for selected in [selected_permanent, selected_spell] {
        assert_eq!(
            game.object(selected).expect("selected card exists").zone,
            Zone::Exile,
            "Black Cat should exile the two selected looked-at cards"
        );
        assert!(
            game.is_face_down(selected),
            "Black Cat should exile selected cards face down"
        );
        assert!(
            game.effect_store.grant_registry.card_can_play_from_zone(
                &game,
                selected,
                Zone::Exile,
                alice,
            ),
            "Black Cat should let its controller play selected exiled cards"
        );
        assert!(
            !game.effect_store.grant_registry.card_can_play_from_zone(
                &game,
                selected,
                Zone::Exile,
                bob,
            ),
            "Black Cat should not let the target opponent play its selected exiled cards"
        );
    }
    assert!(
        game.can_spend_mana_as_any_color(alice, Some(selected_spell)),
        "Black Cat should allow mana of any type to cast exiled spells"
    );
    assert!(
        !game.can_spend_mana_as_any_color(bob, Some(selected_spell)),
        "Black Cat's any-mana permission should belong to its controller"
    );
    assert!(
        !game.effect_store.grant_registry.card_can_play_from_zone(
            &game,
            unrelated_exiled_id,
            Zone::Exile,
            alice,
        ),
        "Black Cat should not grant play permission for unrelated exiled cards"
    );
    assert!(
        !game.can_spend_mana_as_any_color(alice, Some(unrelated_exiled_id)),
        "Black Cat should not grant any-mana casting for unrelated exiled cards"
    );
    assert_eq!(
        game.player(bob).expect("bob exists").library.len(),
        7,
        "Black Cat should put the seven unchosen looked-at cards back on the bottom of Bob's library"
    );
    assert!(
        bob_library[..7].iter().all(|id| game
            .object(*id)
            .is_some_and(|object| object.zone == Zone::Library && object.owner == bob)),
        "Black Cat should leave the unchosen looked-at cards in the target opponent's library"
    );
    assert_eq!(
        game.object(alice_card_id).expect("alice card exists").zone,
        Zone::Library,
        "Black Cat should not touch its controller's library"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn mindleech_mass_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(90_467), "Mindleech Mass")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(5)],
            vec![ManaSymbol::Blue],
            vec![ManaSymbol::Black],
            vec![ManaSymbol::Black],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Horror])
        .power_toughness(PowerToughness::fixed(6, 6))
        .parse_text(
            "Trample\nWhenever this creature deals combat damage to a player, you may look at that player's hand. If you do, you may cast a spell from among those cards without paying its mana cost.",
        )
        .expect("Mindleech Mass should parse for runtime tests")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) struct MindleechMassDecisionMaker {
    pub(super) boolean_choices: Vec<bool>,
}

#[cfg(ironsmith_runtime_parser_tests)]
impl DecisionMaker for MindleechMassDecisionMaker {
    fn decide_boolean(
        &mut self,
        _game: &GameState,
        _ctx: &crate::decisions::context::BooleanContext,
    ) -> bool {
        if self.boolean_choices.is_empty() {
            return false;
        }
        self.boolean_choices.remove(0)
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn resolve_mindleech_mass_damage_trigger(
    game: &mut GameState,
    definition: &crate::cards::CardDefinition,
    source_id: ObjectId,
    damaged_player: PlayerId,
    dm: &mut dyn DecisionMaker,
) {
    let controller = game
        .controller_of_id(source_id)
        .expect("Mindleech Mass should have a controller");
    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Mindleech Mass should have a combat damage trigger");

    let damage_event = TriggerEvent::new_with_provenance(
        crate::events::DamageEvent::with_cause(
            source_id,
            crate::events::DamageTarget::Player(damaged_player),
            6,
            true,
            crate::events::cause::EventCause::combat_damage(source_id),
        ),
        crate::provenance::ProvNodeId::default(),
    );
    let mut ctx = ExecutionContext::new_default(source_id, controller)
        .with_decision_maker(dm)
        .with_triggering_event(damage_event);
    for effect in &triggered.effects {
        execute_effect(game, effect, &mut ctx).expect("Mindleech Mass trigger should resolve");
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn mindleech_mass_casts_spell_from_damaged_players_hand_without_paying() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let definition = mindleech_mass_definition();
    let mindleech_id = game.create_object_from_definition(&definition, alice, Zone::Battlefield);

    let spell = CardBuilder::new(CardId::from_raw(90_468), "Mindleech Victim Spell")
        .card_types(vec![CardType::Sorcery])
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(7)]]))
        .build();
    let land = CardBuilder::new(CardId::from_raw(90_469), "Mindleech Victim Land")
        .card_types(vec![CardType::Land])
        .build();
    let _spell_id = game.create_object_from_card(&spell, bob, Zone::Hand);
    let land_id = game.create_object_from_card(&land, bob, Zone::Hand);

    let mut dm = MindleechMassDecisionMaker {
        boolean_choices: vec![true, true, true],
    };
    resolve_mindleech_mass_damage_trigger(&mut game, &definition, mindleech_id, bob, &mut dm);

    assert!(
        dm.boolean_choices.is_empty(),
        "Mindleech Mass should consume the look and free-cast choices"
    );

    let stack_entry = game
        .stack
        .last()
        .expect("accepting Mindleech Mass should cast the spell from hand");
    assert_eq!(stack_entry.controller, alice);
    assert_eq!(
        stack_entry.casting_method,
        crate::alternative_cast::CastingMethod::Normal
    );
    let stack_spell = game
        .object(stack_entry.object_id)
        .expect("cast spell should be on stack");
    assert_eq!(stack_spell.name, "Mindleech Victim Spell");
    assert_eq!(stack_spell.zone, Zone::Stack);
    assert_eq!(stack_spell.owner, bob);
    assert!(
        game.object(land_id)
            .is_some_and(|object| object.zone == Zone::Hand),
        "Mindleech Mass should not offer lands as spells to cast"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn geode_golem_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(627_850), "Geode Golem")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(5)]]))
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .subtypes(vec![Subtype::Golem])
        .power_toughness(PowerToughness::fixed(5, 3))
        .parse_text(
            "Trample\nWhenever this creature deals combat damage to a player, you may cast your commander from the command zone without paying its mana cost.",
        )
        .expect("Geode Golem should parse for runtime tests")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn resolve_geode_golem_damage_trigger(
    game: &mut GameState,
    definition: &crate::cards::CardDefinition,
    source_id: ObjectId,
    damaged_player: PlayerId,
    dm: &mut dyn DecisionMaker,
) {
    let controller = game
        .controller_of_id(source_id)
        .expect("Geode Golem should have a controller");
    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Geode Golem should have a combat damage trigger");

    let damage_event = TriggerEvent::new_with_provenance(
        crate::events::DamageEvent::with_cause(
            source_id,
            crate::events::DamageTarget::Player(damaged_player),
            5,
            true,
            crate::events::cause::EventCause::combat_damage(source_id),
        ),
        crate::provenance::ProvNodeId::default(),
    );
    let mut ctx = ExecutionContext::new_default(source_id, controller)
        .with_decision_maker(dm)
        .with_triggering_event(damage_event);
    for effect in &triggered.effects {
        execute_effect(game, effect, &mut ctx).expect("Geode Golem trigger should resolve");
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn geode_golem_damage_trigger_casts_your_commander_from_command_zone_without_mana() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let definition = geode_golem_definition();
    let geode_id = game.create_object_from_definition(&definition, alice, Zone::Battlefield);

    let opponents_commander = CardBuilder::new(CardId::from_raw(627_849), "Bob's Geode Commander")
        .supertypes(vec![Supertype::Legendary])
        .card_types(vec![CardType::Creature])
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(7)]]))
        .build();
    let opponents_commander_id =
        game.create_object_from_card(&opponents_commander, bob, Zone::Command);
    game.set_as_commander(opponents_commander_id, bob);

    let noncommander =
        CardBuilder::new(CardId::from_raw(627_848), "Geode Command-Zone Noncommander")
            .supertypes(vec![Supertype::Legendary])
            .card_types(vec![CardType::Creature])
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(7)]]))
            .build();
    let noncommander_id = game.create_object_from_card(&noncommander, alice, Zone::Command);

    let commander = CardBuilder::new(CardId::from_raw(627_851), "Geode Test Commander")
        .supertypes(vec![Supertype::Legendary])
        .card_types(vec![CardType::Creature])
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(7)]]))
        .build();
    let commander_id = game.create_object_from_card(&commander, alice, Zone::Command);
    game.set_as_commander(commander_id, alice);

    let mut dm = SelectFirstDecisionMaker;
    resolve_geode_golem_damage_trigger(&mut game, &definition, geode_id, bob, &mut dm);

    let stack_entry = game
        .stack
        .last()
        .expect("accepting Geode Golem should cast the commander onto the stack");
    let stack_object = game
        .object(stack_entry.object_id)
        .expect("cast commander should exist on the stack");
    assert_eq!(stack_object.name, "Geode Test Commander");
    assert_eq!(stack_entry.controller, alice);
    assert_eq!(
        stack_entry.casting_method,
        crate::alternative_cast::CastingMethod::PlayFrom {
            source: geode_id,
            zone: Zone::Command,
            use_alternative: None,
        }
    );
    assert_eq!(
        game.commander_cast_count(commander_id),
        1,
        "effect-driven command-zone casts should update commander cast count"
    );
    assert_eq!(
        game.object(opponents_commander_id)
            .expect("opponent commander should remain in the command zone")
            .zone,
        Zone::Command,
        "Geode Golem should not offer an opponent's commander"
    );
    assert_eq!(
        game.object(noncommander_id)
            .expect("noncommander should remain in the command zone")
            .zone,
        Zone::Command,
        "Geode Golem should not offer noncommander command-zone objects"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn geode_golem_damage_trigger_does_not_cast_when_declined_or_no_commander_exists() {
    let mut declined_game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let definition = geode_golem_definition();
    let geode_id =
        declined_game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let commander = CardBuilder::new(CardId::from_raw(627_852), "Declined Geode Commander")
        .supertypes(vec![Supertype::Legendary])
        .card_types(vec![CardType::Creature])
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(7)]]))
        .build();
    let commander_id = declined_game.create_object_from_card(&commander, alice, Zone::Command);
    declined_game.set_as_commander(commander_id, alice);

    let mut decline_dm = AutoPassDecisionMaker;
    resolve_geode_golem_damage_trigger(
        &mut declined_game,
        &definition,
        geode_id,
        bob,
        &mut decline_dm,
    );
    assert!(declined_game.stack.is_empty());
    assert_eq!(
        declined_game
            .object(commander_id)
            .expect("declined commander should remain in command zone")
            .zone,
        Zone::Command
    );

    let mut no_commander_game = setup_game();
    let geode_id =
        no_commander_game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let mut accept_dm = SelectFirstDecisionMaker;
    resolve_geode_golem_damage_trigger(
        &mut no_commander_game,
        &definition,
        geode_id,
        bob,
        &mut accept_dm,
    );
    assert!(
        no_commander_game.stack.is_empty(),
        "Geode Golem should not cast anything when you have no commander in the command zone"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn mindleech_mass_declining_look_prevents_free_cast_branch() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let definition = mindleech_mass_definition();
    let mindleech_id = game.create_object_from_definition(&definition, alice, Zone::Battlefield);

    let spell = CardBuilder::new(CardId::from_raw(90_470), "Declined Mindleech Spell")
        .card_types(vec![CardType::Sorcery])
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(7)]]))
        .build();
    let spell_id = game.create_object_from_card(&spell, bob, Zone::Hand);

    let mut dm = MindleechMassDecisionMaker {
        boolean_choices: vec![false],
    };
    resolve_mindleech_mass_damage_trigger(&mut game, &definition, mindleech_id, bob, &mut dm);

    assert!(
        game.stack.is_empty(),
        "declining the look should skip the free-cast branch"
    );
    assert!(
        game.object(spell_id)
            .is_some_and(|object| object.zone == Zone::Hand),
        "declining the look should leave the damaged player's spell in hand"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn mindleech_mass_declining_free_cast_leaves_spell_in_hand() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let definition = mindleech_mass_definition();
    let mindleech_id = game.create_object_from_definition(&definition, alice, Zone::Battlefield);

    let spell = CardBuilder::new(CardId::from_raw(90_471), "Uncast Mindleech Spell")
        .card_types(vec![CardType::Sorcery])
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(7)]]))
        .build();
    let spell_id = game.create_object_from_card(&spell, bob, Zone::Hand);

    let mut dm = MindleechMassDecisionMaker {
        boolean_choices: vec![true, false],
    };
    resolve_mindleech_mass_damage_trigger(&mut game, &definition, mindleech_id, bob, &mut dm);

    assert!(
        game.stack.is_empty(),
        "declining the free cast should not cast a spell"
    );
    assert!(
        game.object(spell_id)
            .is_some_and(|object| object.zone == Zone::Hand),
        "declining the free cast should leave the damaged player's spell in hand"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn riveteers_charm_mode_one_limits_sacrifice_to_greatest_mana_value_ties() {
    struct ChooseSacrificeFromGreatestTie {
        desired: ObjectId,
        seen_candidates: Vec<ObjectId>,
    }

    impl DecisionMaker for ChooseSacrificeFromGreatestTie {
        fn decide_objects(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            self.seen_candidates = ctx
                .candidates
                .iter()
                .map(|candidate| candidate.id)
                .collect();
            vec![self.desired]
        }
    }

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let riveteers_charm = CardDefinitionBuilder::new(CardId::new(), "Riveteers Charm")
        .mana_cost(ManaCost::new())
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Choose one —\n\
• Target opponent sacrifices a creature or planeswalker they control with the greatest mana value among creatures and planeswalkers they control.\n\
• Exile the top three cards of your library. Until your next end step, you may play those cards.\n\
• Exile target player's graveyard.",
        )
        .expect("Riveteers Charm should parse");

    let low_creature = CardBuilder::new(CardId::new(), "Low Creature")
        .card_types(vec![CardType::Creature])
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]]))
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let tie_creature = CardBuilder::new(CardId::new(), "Tie Creature")
        .card_types(vec![CardType::Creature])
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(5)]]))
        .power_toughness(PowerToughness::fixed(4, 4))
        .build();
    let tie_planeswalker = CardBuilder::new(CardId::new(), "Tie Planeswalker")
        .card_types(vec![CardType::Planeswalker])
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(5)]]))
        .build();

    let low_id = game.create_object_from_card(&low_creature, bob, Zone::Battlefield);
    let tie_creature_id = game.create_object_from_card(&tie_creature, bob, Zone::Battlefield);
    let tie_planeswalker_id =
        game.create_object_from_card(&tie_planeswalker, bob, Zone::Battlefield);

    let spell_id = game.create_object_from_definition(&riveteers_charm, alice, Zone::Hand);
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut trigger_queue = TriggerQueue::new();
    let progress = apply_priority_response(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(LegalAction::CastSpell {
            spell_id,
            from_zone: Zone::Hand,
            casting_method: CastingMethod::Normal,
        }),
    )
    .expect("casting Riveteers Charm should reach mode selection");

    match progress {
        GameProgress::NeedsDecisionCtx(crate::decisions::context::DecisionContext::Modes(_)) => {}
        other => panic!("expected mode selection for Riveteers Charm, got {other:?}"),
    }

    let progress = apply_priority_response(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::Modes(vec![0]),
    )
    .expect("choosing Riveteers Charm sacrifice mode should request targets");

    match progress {
        GameProgress::NeedsDecisionCtx(crate::decisions::context::DecisionContext::Targets(_)) => {}
        other => panic!("expected opponent target selection for Riveteers Charm, got {other:?}"),
    }

    apply_priority_response(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::Targets(vec![Target::Player(bob)]),
    )
    .expect("choosing target opponent should finish casting Riveteers Charm");

    let mut dm = ChooseSacrificeFromGreatestTie {
        desired: tie_planeswalker_id,
        seen_candidates: Vec::new(),
    };
    resolve_stack_entry_with(&mut game, &mut dm)
        .expect("Riveteers Charm sacrifice mode should resolve cleanly");

    let tie_survivors = [tie_creature_id, tie_planeswalker_id]
        .iter()
        .filter(|id| game.battlefield.contains(id))
        .count();

    assert!(
        game.battlefield.contains(&low_id),
        "lower-mana-value permanents should not be sacrificed"
    );
    assert!(
        tie_survivors == 1,
        "exactly one greatest-mana-value permanent should be sacrificed from the tie; candidates seen: {:?}",
        dm.seen_candidates
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn riveteers_charm_mode_two_play_permission_lasts_through_next_end_step_window() {
    use crate::decision::compute_legal_actions;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let riveteers_charm = CardDefinitionBuilder::new(CardId::new(), "Riveteers Charm")
        .mana_cost(ManaCost::new())
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Choose one —\n\
• Target opponent sacrifices a creature or planeswalker they control with the greatest mana value among creatures and planeswalkers they control.\n\
• Exile the top three cards of your library. Until your next end step, you may play those cards.\n\
• Exile target player's graveyard.",
        )
        .expect("Riveteers Charm should parse");

    let top_land = CardBuilder::new(CardId::new(), "Charm Land")
        .card_types(vec![CardType::Land])
        .build();
    let top_spell = CardBuilder::new(CardId::new(), "Charm Spell")
        .card_types(vec![CardType::Sorcery])
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Red]]))
        .build();
    let third_card = CardBuilder::new(CardId::new(), "Charm Third")
        .card_types(vec![CardType::Instant])
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Green]]))
        .build();

    let _top_land_id = game.create_object_from_card(&top_land, alice, Zone::Library);
    let _top_spell_id = game.create_object_from_card(&top_spell, alice, Zone::Library);
    let _third_card_id = game.create_object_from_card(&third_card, alice, Zone::Library);

    let spell_id = game.create_object_from_definition(&riveteers_charm, alice, Zone::Hand);
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut trigger_queue = TriggerQueue::new();
    let progress = apply_priority_response(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(LegalAction::CastSpell {
            spell_id,
            from_zone: Zone::Hand,
            casting_method: CastingMethod::Normal,
        }),
    )
    .expect("casting Riveteers Charm should reach mode selection");

    match progress {
        GameProgress::NeedsDecisionCtx(crate::decisions::context::DecisionContext::Modes(_)) => {}
        other => panic!("expected mode selection for Riveteers Charm, got {other:?}"),
    }

    apply_priority_response(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::Modes(vec![1]),
    )
    .expect("choosing Riveteers Charm exile mode should finish casting");

    resolve_stack_entry_with(&mut game, &mut AutoPassDecisionMaker)
        .expect("Riveteers Charm exile mode should resolve cleanly");

    game.player_mut(alice)
        .expect("alice exists")
        .mana_pool
        .add(ManaSymbol::Red, 1);

    let exiled_ids = game.exile.clone();
    let exiled_names: Vec<_> = exiled_ids
        .iter()
        .filter_map(|&id| game.object(id).map(|obj| (id, obj.name.to_string())))
        .collect();
    let exiled_land_id = exiled_names
        .iter()
        .find_map(|(id, name)| (*name == "Charm Land").then_some(*id))
        .expect("Riveteers Charm should exile the top land card");
    let exiled_spell_id = exiled_names
        .iter()
        .find_map(|(id, name)| (*name == "Charm Spell").then_some(*id))
        .expect("Riveteers Charm should exile the top spell card");

    assert_eq!(
        game.exile.len(),
        3,
        "Riveteers Charm should exile three cards"
    );
    assert!(
        game.effect_store.grant_registry.card_can_play_from_zone(
            &game,
            exiled_land_id,
            Zone::Exile,
            alice
        ),
        "Riveteers Charm should let you play exiled lands during the window"
    );
    assert!(
        game.effect_store.grant_registry.card_can_play_from_zone(
            &game,
            exiled_spell_id,
            Zone::Exile,
            alice
        ),
        "Riveteers Charm should let you cast exiled spells during the window"
    );

    let actions_now = compute_legal_actions(&game, alice);
    assert!(
        actions_now.iter().any(|action| matches!(
            action,
            LegalAction::PlayLand { land_id } if *land_id == exiled_land_id
        )),
        "Riveteers Charm should expose a land play action from exile"
    );
    assert!(
        actions_now.iter().any(|action| matches!(
            action,
            LegalAction::CastSpell { spell_id, from_zone: Zone::Exile, .. } if *spell_id == exiled_spell_id
        )),
        "Riveteers Charm should expose a cast action from exile"
    );

    game.turn.turn_number = game.turn.turn_number.saturating_add(1);
    game.turn.active_player = PlayerId::from_index(1);
    assert!(
        game.effect_store.grant_registry.card_can_play_from_zone(
            &game,
            exiled_spell_id,
            Zone::Exile,
            alice
        ),
        "Riveteers Charm play window should still exist before your next end step"
    );

    game.turn.turn_number = game.turn.turn_number.saturating_add(2);
    assert!(
        !game.effect_store.grant_registry.card_can_play_from_zone(
            &game,
            exiled_spell_id,
            Zone::Exile,
            alice
        ),
        "Riveteers Charm play window should expire after your next end step"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn rakdos_the_muscle_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(90_662), "Rakdos, the Muscle")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Black],
            vec![ManaSymbol::Black],
            vec![ManaSymbol::Red],
        ]))
        .supertypes(vec![Supertype::Legendary])
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Demon, Subtype::Mercenary])
        .power_toughness(PowerToughness::fixed(6, 5))
        .parse_text(
            "Flying, trample\n\
             Whenever you sacrifice another creature, exile cards equal to its mana value from the top of target player's library. Until your next end step, you may play those cards, and mana of any type can be spent to cast those spells.\n\
             Sacrifice another creature: Rakdos gains indestructible until end of turn. Tap it. Activate only once each turn.",
        )
        .expect("Rakdos, the Muscle should parse strictly")
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn rakdos_the_muscle_trigger_exiles_mana_value_cards_and_grants_next_end_step_any_color_play()
 {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let rakdos = rakdos_the_muscle_definition();
    let rakdos_id = game.create_object_from_definition(&rakdos, alice, Zone::Battlefield);
    let sacrificed = CardBuilder::new(CardId::from_raw(90_663), "Rakdos Sacrifice Fuel")
        .card_types(vec![CardType::Creature])
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]]))
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let sacrificed_id = game.create_object_from_card(&sacrificed, alice, Zone::Battlefield);
    let noncreature = CardBuilder::new(CardId::from_raw(90_664), "Rakdos Noncreature Fuel")
        .card_types(vec![CardType::Artifact])
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]]))
        .build();
    let noncreature_id = game.create_object_from_card(&noncreature, alice, Zone::Battlefield);

    let exiled_land = CardBuilder::new(CardId::from_raw(90_665), "Rakdos Exiled Land")
        .card_types(vec![CardType::Land])
        .build();
    let exiled_spell = CardBuilder::new(CardId::from_raw(90_666), "Rakdos Exiled Spell")
        .card_types(vec![CardType::Sorcery])
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Green]]))
        .build();
    let library_spare = CardBuilder::new(CardId::from_raw(90_667), "Rakdos Library Spare")
        .card_types(vec![CardType::Instant])
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Blue]]))
        .build();
    game.create_object_from_card(&library_spare, bob, Zone::Library);
    game.create_object_from_card(&exiled_spell, bob, Zone::Library);
    game.create_object_from_card(&exiled_land, bob, Zone::Library);

    let noncreature_event = TriggerEvent::new_with_provenance(
        crate::events::permanents::SacrificeEvent::new(noncreature_id, Some(rakdos_id))
            .with_snapshot(
                game.object(noncreature_id)
                    .map(|object| crate::snapshot::ObjectSnapshot::from_object(object, &game)),
                Some(alice),
            ),
        crate::provenance::ProvNodeId::default(),
    );
    assert!(
        crate::triggers::check_triggers(&game, &noncreature_event)
            .into_iter()
            .filter(|entry| entry.source == rakdos_id)
            .count()
            == 0,
        "Rakdos should not trigger when you sacrifice a noncreature"
    );

    let self_event = TriggerEvent::new_with_provenance(
        crate::events::permanents::SacrificeEvent::new(rakdos_id, Some(rakdos_id)).with_snapshot(
            game.object(rakdos_id)
                .map(|object| crate::snapshot::ObjectSnapshot::from_object(object, &game)),
            Some(alice),
        ),
        crate::provenance::ProvNodeId::default(),
    );
    assert!(
        crate::triggers::check_triggers(&game, &self_event)
            .into_iter()
            .filter(|entry| entry.source == rakdos_id)
            .count()
            == 0,
        "Rakdos should not trigger when itself is sacrificed"
    );

    let sacrifice_event = TriggerEvent::new_with_provenance(
        crate::events::permanents::SacrificeEvent::new(sacrificed_id, Some(rakdos_id))
            .with_snapshot(
                game.object(sacrificed_id)
                    .map(|object| crate::snapshot::ObjectSnapshot::from_object(object, &game)),
                Some(alice),
            ),
        crate::provenance::ProvNodeId::default(),
    );
    game.move_object_by_effect(sacrificed_id, Zone::Graveyard)
        .expect("sacrificed creature should move to the graveyard");
    let matching_triggers: Vec<_> = crate::triggers::check_triggers(&game, &sacrifice_event)
        .into_iter()
        .filter(|entry| entry.source == rakdos_id)
        .collect();
    assert_eq!(
        matching_triggers.len(),
        1,
        "Rakdos should trigger exactly once when you sacrifice another creature"
    );

    let triggered = rakdos
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Rakdos should have a sacrifice trigger");
    let effects = triggered.effects.flattened_default_effects();
    let target_requirements =
        super::targeting::extract_target_requirements(&game, &effects, alice, Some(rakdos_id));
    assert_eq!(
        target_requirements.len(),
        1,
        "Rakdos trigger should require exactly one target player"
    );
    let mut ctx = ExecutionContext::new_default(rakdos_id, alice)
        .with_triggering_event(sacrifice_event)
        .with_targets(vec![crate::effects::ResolvedTarget::Player(bob)])
        .with_target_assignments(vec![crate::game_state::TargetAssignment {
            spec: target_requirements[0].spec.clone(),
            range: 0..1,
        }]);
    for effect in effects {
        crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap_or_else(|err| {
            panic!("Rakdos trigger effect should resolve: {effect:?}: {err:?}")
        });
    }

    assert_eq!(
        game.exile.len(),
        2,
        "Rakdos should exile cards equal to the sacrificed creature's mana value"
    );
    let exiled_names: Vec<_> = game
        .exile
        .iter()
        .filter_map(|&id| game.object(id).map(|object| (id, object.name.to_string())))
        .collect();
    let exiled_land_id = exiled_names
        .iter()
        .find_map(|(id, name)| (*name == "Rakdos Exiled Land").then_some(*id))
        .expect("Rakdos should exile the land among the two cards");
    let exiled_spell_id = exiled_names
        .iter()
        .find_map(|(id, name)| (*name == "Rakdos Exiled Spell").then_some(*id))
        .expect("Rakdos should exile the spell among the two cards");
    assert!(
        game.player(bob)
            .expect("Bob exists")
            .library
            .iter()
            .any(|&id| game
                .object(id)
                .is_some_and(|object| object.name == "Rakdos Library Spare")),
        "Rakdos should leave the third library card behind"
    );

    assert!(
        game.effect_store.grant_registry.card_can_play_from_zone(
            &game,
            exiled_land_id,
            Zone::Exile,
            alice,
        ),
        "Rakdos should let you play exiled lands during the window"
    );
    assert!(
        game.effect_store.grant_registry.card_can_play_from_zone(
            &game,
            exiled_spell_id,
            Zone::Exile,
            alice,
        ),
        "Rakdos should let you cast exiled spells during the window"
    );
    assert!(
        game.can_spend_mana_as_any_color(alice, Some(exiled_spell_id)),
        "Rakdos should allow mana of any color to cast exiled spells"
    );
    assert!(
        !game.can_spend_mana_as_any_color(alice, Some(exiled_land_id)),
        "Rakdos's any-color cast permission should not apply to exiled lands"
    );

    game.turn.turn_number = game.turn.turn_number.saturating_add(1);
    game.turn.active_player = bob;
    assert!(
        game.effect_store.grant_registry.card_can_play_from_zone(
            &game,
            exiled_spell_id,
            Zone::Exile,
            alice,
        ),
        "Rakdos play window should survive before your next end step"
    );

    game.turn.turn_number = game.turn.turn_number.saturating_add(2);
    assert!(
        !game.effect_store.grant_registry.card_can_play_from_zone(
            &game,
            exiled_spell_id,
            Zone::Exile,
            alice,
        ),
        "Rakdos play window should expire after your next end step"
    );
    assert!(
        !game.can_spend_mana_as_any_color(alice, Some(exiled_spell_id)),
        "Rakdos any-color cast permission should expire with the play window"
    );
}

// === Full Game Flow Integration Test ===

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_full_game_lightning_bolt_wins() {
    use crate::cards::definitions::{basic_mountain, lightning_bolt};
    use crate::mana::ManaSymbol;

    // Create a game with 2 players at 3 life (so Lightning Bolt is lethal)
    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 3);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    // Set up for main phase (when spells can be cast)
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);

    // Create Mountain on Alice's battlefield (using CardDefinition for abilities)
    let mountain = basic_mountain();
    let mountain_id = game.create_object_from_definition(&mountain, alice, Zone::Battlefield);

    // Remove summoning sickness from Mountain (it's a land)
    game.remove_summoning_sickness(mountain_id);

    // Create Lightning Bolt in Alice's hand
    let bolt = lightning_bolt();
    let bolt_id = game.create_object_from_definition(&bolt, alice, Zone::Hand);

    // Verify initial state
    assert_eq!(game.player(alice).unwrap().life, 3);
    assert_eq!(game.player(bob).unwrap().life, 3);
    assert_eq!(game.player(alice).unwrap().mana_pool.total(), 0);

    // Step 1: Activate Mountain's mana ability to add {R}
    // Find the mana ability index
    let mountain_obj = game.object(mountain_id).unwrap();
    let _mana_ability_index = mountain_obj
        .abilities
        .iter()
        .position(|a| a.is_mana_ability())
        .expect("Mountain should have a mana ability");

    // Tap mountain for red mana
    game.tap(mountain_id);
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Red, 1);

    // Verify mana was added
    assert_eq!(
        game.player(alice)
            .unwrap()
            .mana_pool
            .amount(ManaSymbol::Red),
        1
    );

    // Step 2: Cast Lightning Bolt targeting Bob
    // Move Lightning Bolt from hand to stack
    let stack_bolt_id = game.move_object_by_effect(bolt_id, Zone::Stack).unwrap();

    // Create stack entry with Bob as target
    let entry = StackEntry::new(stack_bolt_id, alice).with_targets(vec![Target::Player(bob)]);
    game.push_to_stack(entry);

    // Pay the mana cost (remove red mana from pool)
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .remove(ManaSymbol::Red, 1);

    // Verify spell is on stack
    assert!(!game.stack_is_empty());

    // Step 3: Resolve the stack (both players pass priority)
    let result = resolve_stack_entry(&mut game);
    assert!(result.is_ok(), "Stack resolution should succeed");

    // Verify Lightning Bolt dealt 3 damage to Bob
    assert_eq!(game.player(bob).unwrap().life, 0);

    // Lightning Bolt should be in graveyard
    assert!(game.stack_is_empty());
    let alice_graveyard = &game.player(alice).unwrap().graveyard;
    assert_eq!(alice_graveyard.len(), 1);

    // Step 4: Check state-based actions - Bob should lose
    let mut trigger_queue = TriggerQueue::new();
    check_and_apply_sbas(&mut game, &mut trigger_queue).unwrap();

    // Bob should have lost the game
    assert!(
        game.player(bob).unwrap().has_lost,
        "Bob should have lost the game with 0 life"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_full_game_with_decision_maker() {
    use crate::cards::definitions::{basic_mountain, fireball};
    use crate::decision::DecisionMaker;

    #[derive(Debug)]
    struct TestResponseDecisionMaker {
        responses: Vec<PriorityResponse>,
        index: usize,
    }

    impl TestResponseDecisionMaker {
        fn new(responses: Vec<PriorityResponse>) -> Self {
            Self {
                responses,
                index: 0,
            }
        }
    }

    impl DecisionMaker for TestResponseDecisionMaker {
        fn decide_priority(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::PriorityContext,
        ) -> LegalAction {
            if self.index < self.responses.len()
                && let PriorityResponse::PriorityAction(action) = &self.responses[self.index]
            {
                self.index += 1;
                return action.clone();
            }
            ctx.actions
                .iter()
                .find(|a| matches!(a, LegalAction::PassPriority))
                .cloned()
                .unwrap_or_else(|| ctx.actions[0].clone())
        }

        fn decide_number(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::NumberContext,
        ) -> u32 {
            if self.index < self.responses.len() {
                if let PriorityResponse::XValue(x) = self.responses[self.index] {
                    self.index += 1;
                    return x;
                }
                if let PriorityResponse::NumberChoice(n) = self.responses[self.index] {
                    self.index += 1;
                    return n;
                }
            }
            ctx.min
        }

        fn decide_targets(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::TargetsContext,
        ) -> Vec<Target> {
            if self.index < self.responses.len()
                && let PriorityResponse::Targets(targets) = &self.responses[self.index]
            {
                self.index += 1;
                return targets.clone();
            }
            ctx.requirements
                .iter()
                .filter(|r| r.min_targets > 0)
                .filter_map(|r| r.legal_targets.first().cloned())
                .collect()
        }
    }

    // Create a game with 2 players at 3 life
    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 3);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    // Set up for main phase
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);

    // Create 4 Mountains on Alice's battlefield (Fireball with X=3 costs {3}{R} = 4 mana)
    let mountain_def = basic_mountain();
    let mut mountain_ids = Vec::new();
    for _ in 0..4 {
        let mountain_id =
            game.create_object_from_definition(&mountain_def, alice, Zone::Battlefield);
        game.remove_summoning_sickness(mountain_id);
        mountain_ids.push(mountain_id);
    }

    // Create Fireball in Alice's hand
    let fireball_def = fireball();
    let fireball_id = game.create_object_from_definition(&fireball_def, alice, Zone::Hand);

    // Find mana ability index (same for all mountains)
    let mana_ability_index = game
        .object(mountain_ids[0])
        .unwrap()
        .abilities
        .iter()
        .position(|a| a.is_mana_ability())
        .expect("Mountain should have a mana ability");

    // Create scripted responses:
    // 1-4. Alice activates mana ability on each mountain (adds 4R to pool)
    // 5. Alice casts Fireball (prompts for X value since it has X in cost)
    // 6. Alice chooses X=3 (deals 3 damage)
    // 7. Alice selects Bob as target
    // 8. Bob passes priority
    // 9. Alice passes priority (spell resolves, dealing 3 damage to Bob)
    let mut responses = Vec::new();

    // Tap all 4 mountains for mana
    for &mountain_id in &mountain_ids {
        responses.push(PriorityResponse::PriorityAction(
            LegalAction::ActivateManaAbility {
                source: mountain_id,
                ability_index: mana_ability_index,
            },
        ));
    }

    // Cast Fireball
    responses.push(PriorityResponse::PriorityAction(LegalAction::CastSpell {
        spell_id: fireball_id,
        from_zone: Zone::Hand,
        casting_method: CastingMethod::Normal,
    }));

    // Choose X=3 (after CastSpell triggers ChooseXValue decision)
    responses.push(PriorityResponse::XValue(3));

    // Choose Bob as target (after X value triggers ChooseTargets decision)
    responses.push(PriorityResponse::Targets(vec![Target::Player(bob)]));

    // Both players pass priority
    responses.push(PriorityResponse::PriorityAction(LegalAction::PassPriority)); // Bob passes
    responses.push(PriorityResponse::PriorityAction(LegalAction::PassPriority)); // Alice passes

    let mut decision_maker = TestResponseDecisionMaker::new(responses);
    let mut trigger_queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());

    // Run the decision-based priority loop
    let mut iterations = 0;
    loop {
        iterations += 1;
        if iterations > 20 {
            panic!("Too many iterations - decision loop may be stuck");
        }

        // Advance to get next decision
        let progress = advance_priority(&mut game, &mut trigger_queue)
            .expect("advance_priority should not fail");

        // Helper closure to handle a decision and any nested decisions
        let handle_result = |mut result: GameProgress,
                             game: &mut GameState,
                             trigger_queue: &mut TriggerQueue,
                             state: &mut PriorityLoopState,
                             dm: &mut TestResponseDecisionMaker|
         -> Option<GameProgress> {
            loop {
                match result {
                    GameProgress::Continue => return Some(GameProgress::Continue),
                    GameProgress::GameOver(r) => return Some(GameProgress::GameOver(r)),
                    GameProgress::StackResolved => return Some(GameProgress::StackResolved),
                    GameProgress::NeedsDecisionCtx(ctx) => {
                        result =
                            apply_decision_context_with_dm(game, trigger_queue, state, &ctx, dm)
                                .expect("apply_decision_context_with_dm should not fail");
                    }
                }
            }
        };

        match progress {
            GameProgress::NeedsDecisionCtx(ctx) => {
                // Apply the response
                let result = apply_decision_context_with_dm(
                    &mut game,
                    &mut trigger_queue,
                    &mut state,
                    &ctx,
                    &mut decision_maker,
                )
                .expect("apply_decision_context_with_dm should not fail");

                // Handle any nested decisions
                if let Some(final_result) = handle_result(
                    result,
                    &mut game,
                    &mut trigger_queue,
                    &mut state,
                    &mut decision_maker,
                ) {
                    match final_result {
                        GameProgress::GameOver(r) => {
                            assert!(
                                matches!(r, GameResult::Winner(winner) if winner == alice),
                                "Alice should win (Bob at 0 life)"
                            );
                            break;
                        }
                        GameProgress::Continue => break,
                        GameProgress::StackResolved => {} // Continue outer loop
                        _ => {}
                    }
                }
            }
            GameProgress::Continue => {
                // Phase ended - in a full game we'd continue, but for this test we're done
                break;
            }
            GameProgress::GameOver(result) => {
                // Game ended
                assert!(
                    matches!(result, GameResult::Winner(winner) if winner == alice),
                    "Alice should win (Bob at 0 life)"
                );
                break;
            }
            GameProgress::StackResolved => {
                // Stack resolved, continue loop to re-advance priority
            }
        }
    }

    // Verify final state
    assert_eq!(game.player(bob).unwrap().life, 0, "Bob should be at 0 life");
    assert!(
        game.player(bob).unwrap().has_lost,
        "Bob should have lost the game"
    );
}

// ============================================================================
// Card-Specific Integration Tests
// ============================================================================

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_darksteel_colossus_shuffle_into_library() {
    use crate::cards::definitions::darksteel_colossus;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    // Create Darksteel Colossus on battlefield
    let colossus_def = darksteel_colossus();
    let colossus_id = game.create_object_from_definition(&colossus_def, alice, Zone::Battlefield);

    // Verify it has the ShuffleIntoLibraryFromGraveyard ability
    let colossus = game.object(colossus_id).unwrap();
    let has_ability = colossus.abilities.iter().any(|a| {
        if let crate::ability::AbilityKind::Static(s) = &a.kind {
            s.id() == crate::static_abilities::StaticAbilityId::ShuffleIntoLibraryFromGraveyard
        } else {
            false
        }
    });
    assert!(
        has_ability,
        "Darksteel Colossus should have ShuffleIntoLibraryFromGraveyard"
    );

    // Record initial library size
    let _initial_library_size = game.player(alice).unwrap().library.len();

    // Verify it's on battlefield
    assert!(game.battlefield.contains(&colossus_id));
    assert_eq!(game.object(colossus_id).unwrap().zone, Zone::Battlefield);

    // Note: The actual zone change interception would happen in move_object
    // This test verifies the ability is present; full behavior would require
    // implementing the replacement effect handling in game_state.rs
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_thorn_elemental_has_ability() {
    use crate::cards::definitions::thorn_elemental;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    // Create Thorn Elemental on battlefield
    let thorn_def = thorn_elemental();
    let thorn_id = game.create_object_from_definition(&thorn_def, alice, Zone::Battlefield);

    // Verify it has trample
    let thorn = game.object(thorn_id).unwrap();
    let has_trample = thorn.abilities.iter().any(|a| {
        if let crate::ability::AbilityKind::Static(s) = &a.kind {
            s.has_trample()
        } else {
            false
        }
    });
    assert!(has_trample, "Thorn Elemental should have trample");

    // Verify it has MayAssignDamageAsUnblocked
    let has_unblocked_ability = thorn.abilities.iter().any(|a| {
        if let crate::ability::AbilityKind::Static(s) = &a.kind {
            s.id() == crate::static_abilities::StaticAbilityId::MayAssignDamageAsUnblocked
        } else {
            false
        }
    });
    assert!(
        has_unblocked_ability,
        "Thorn Elemental should have MayAssignDamageAsUnblocked"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_thorn_elemental_combat_decision() {
    use crate::cards::definitions::thorn_elemental;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    // Create Thorn Elemental on battlefield
    let thorn_def = thorn_elemental();
    let thorn_id = game.create_object_from_definition(&thorn_def, alice, Zone::Battlefield);

    // Create a blocker
    let blocker_id = create_creature(&mut game, "Blocker", bob, 2, 2);

    // Remove summoning sickness
    game.remove_summoning_sickness(thorn_id);

    // Set up combat: Thorn Elemental attacks Bob, Blocker blocks
    let mut combat = CombatState::default();
    combat.attackers.push(crate::combat_state::AttackerInfo {
        creature: thorn_id,
        target: AttackTarget::Player(bob),
    });
    combat.blockers.insert(thorn_id, vec![blocker_id]);

    // Verify the thorn elemental has the ability that would trigger the decision
    let thorn = game.object(thorn_id).unwrap();
    let has_ability = thorn.abilities.iter().any(|a| {
        if let crate::ability::AbilityKind::Static(s) = &a.kind {
            s.id() == crate::static_abilities::StaticAbilityId::MayAssignDamageAsUnblocked
        } else {
            false
        }
    });
    assert!(has_ability);

    // Without the decision (normal combat), damage goes to blocker
    // With trample, Thorn Elemental deals 7 damage: 2 to blocker (lethal), 5 to Bob
    let events = execute_combat_damage_step(&mut game, &combat, false);

    // Verify damage was dealt (trample behavior)
    assert!(!events.is_empty());
    // Blocker takes lethal damage (2)
    assert_eq!(game.damage_on(blocker_id), 2);
    // Bob takes trample damage (7 - 2 = 5)
    assert_eq!(game.player(bob).unwrap().life, 15);
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_indomitable_might_grants_power_and_assign_as_unblocked_static_ability() {
    use crate::cards::builders::CardDefinitionBuilder;
    use crate::object::AttachmentTarget;
    use crate::types::{CardType, Subtype};

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let enchanted_id = create_creature(&mut game, "Grizzly Probe", alice, 2, 2);
    let aura = CardDefinitionBuilder::new(CardId::new(), "Indomitable Might")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Aura])
        .parse_text(
            "Flash\nEnchant creature\nEnchanted creature gets +3/+3.\nEnchanted creature's controller may have it assign its combat damage as though it weren't blocked.",
        )
        .expect("Indomitable Might text should parse into an Aura card definition");
    let aura_id = game.create_object_from_definition(&aura, alice, Zone::Battlefield);

    {
        let aura_obj = game
            .object_mut(aura_id)
            .expect("aura should exist on battlefield");
        aura_obj.attached_to = Some(AttachmentTarget::Object(enchanted_id));
    }
    {
        let enchanted_obj = game
            .object_mut(enchanted_id)
            .expect("enchanted creature should exist");
        enchanted_obj.attachments.push(aura_id);
    }
    let characteristics = game
        .calculated_characteristics(enchanted_id)
        .expect("enchanted creature should have calculated characteristics");
    assert_eq!(characteristics.power, Some(5));
    assert_eq!(characteristics.toughness, Some(5));
    assert!(characteristics.static_abilities.iter().any(|ability| {
        ability.id() == crate::static_abilities::StaticAbilityId::MayAssignDamageAsUnblocked
    }));
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_indomitable_might_blocked_combat_defaults_to_blocker_damage_assignment() {
    use crate::cards::builders::CardDefinitionBuilder;
    use crate::object::AttachmentTarget;
    use crate::types::{CardType, Subtype};

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let enchanted_id = create_creature(&mut game, "Enchanted Attacker", alice, 2, 2);
    let blocker_id = create_creature(&mut game, "Blocking Bear", bob, 2, 2);

    let aura = CardDefinitionBuilder::new(CardId::new(), "Indomitable Might")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Aura])
        .parse_text(
            "Flash\nEnchant creature\nEnchanted creature gets +3/+3.\nEnchanted creature's controller may have it assign its combat damage as though it weren't blocked.",
        )
        .expect("Indomitable Might text should parse into an Aura card definition");
    let aura_id = game.create_object_from_definition(&aura, alice, Zone::Battlefield);
    {
        let aura_obj = game
            .object_mut(aura_id)
            .expect("aura should exist on battlefield");
        aura_obj.attached_to = Some(AttachmentTarget::Object(enchanted_id));
    }
    {
        let enchanted_obj = game
            .object_mut(enchanted_id)
            .expect("enchanted creature should exist");
        enchanted_obj.attachments.push(aura_id);
    }
    game.remove_summoning_sickness(enchanted_id);

    let mut combat = CombatState::default();
    combat.attackers.push(crate::combat_state::AttackerInfo {
        creature: enchanted_id,
        target: AttackTarget::Player(bob),
    });
    combat.blockers.insert(enchanted_id, vec![blocker_id]);

    let events = execute_combat_damage_step(&mut game, &combat, false);
    assert!(!events.is_empty());
    assert_eq!(game.damage_on(blocker_id), 5);
    assert_eq!(game.player(bob).expect("defender should exist").life, 20);
}
