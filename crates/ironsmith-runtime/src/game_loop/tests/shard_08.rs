#![allow(unused_imports)]
use super::shard_00::*;
use super::shard_01::*;
use super::shard_02::*;
use super::shard_03::*;
use super::shard_04::*;
use super::shard_05::*;
use super::shard_06::*;
use super::shard_07::*;
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
pub(super) fn test_resolution_uses_source_lki_for_source_owner() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let source_id = create_creature(&mut game, "Departed Owner Source", alice, 2, 2);
    let entry = StackEntry::ability(
        source_id,
        alice,
        vec![Effect::gain_life_player(3, ChooseSpec::SourceOwner)],
    );
    game.push_to_stack(entry);

    game.move_object_by_effect(source_id, Zone::Graveyard)
        .expect("source should leave before its ability resolves");

    resolve_stack_entry(&mut game).expect("ability should resolve using source owner LKI");

    assert_eq!(
        game.player(alice).expect("Alice should exist").life,
        23,
        "608.2h requires source-owner lookups to use the source's last known information after it leaves"
    );
}

#[test]
pub(super) fn test_resolution_refreshes_source_lki_when_effect_moves_its_own_source() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let source_id = create_creature(&mut game, "Self-Bouncing Source", alice, 2, 2);
    let entry = StackEntry::ability(
        source_id,
        alice,
        vec![
            Effect::move_to_zone(ChooseSpec::Source, Zone::Hand, false),
            Effect::gain_life(Value::PowerOf(Box::new(ChooseSpec::Source))),
        ],
    );
    game.push_to_stack(entry);

    let anthem_card = CardBuilder::new(CardId::from_raw(9009), "Late Battlefield Anthem")
        .card_types(vec![CardType::Enchantment])
        .build();
    let anthem_id = game.create_object_from_card(&anthem_card, alice, Zone::Battlefield);
    game.object_mut(anthem_id)
        .expect("anthem should exist")
        .abilities_mut()
        .push(Ability::static_ability(StaticAbility::anthem(
            crate::filter::ObjectFilter::creature(),
            3,
            0,
        )));
    game.refresh_continuous_state();

    assert_eq!(
        game.calculated_power(source_id),
        Some(5),
        "test setup should apply the late anthem before the source moves"
    );

    resolve_stack_entry(&mut game).expect("ability should resolve using source LKI");

    assert_eq!(
        game.player(alice).expect("Alice should exist").life,
        25,
        "113.7a/608.2h require source LKI from the source's last battlefield existence, even when the resolving effect moved it"
    );
}

#[test]
pub(super) fn test_source_lki_survives_self_move_through_another_zone() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let source_id = create_creature(&mut game, "Roundtrip Source", alice, 2, 2);
    let anthem_card = CardBuilder::new(CardId::from_raw(9011), "Predeparture Anthem")
        .card_types(vec![CardType::Enchantment])
        .build();
    let anthem_id = game.create_object_from_card(&anthem_card, alice, Zone::Battlefield);
    game.object_mut(anthem_id)
        .expect("anthem should exist")
        .abilities_mut()
        .push(Ability::static_ability(StaticAbility::anthem(
            crate::filter::ObjectFilter::creature(),
            3,
            0,
        )));
    game.refresh_continuous_state();

    assert_eq!(
        game.calculated_power(source_id),
        Some(5),
        "test setup should apply the anthem before the source first leaves"
    );

    let entry = StackEntry::ability(
        source_id,
        alice,
        vec![
            Effect::move_to_zone(ChooseSpec::Source, Zone::Graveyard, false),
            Effect::move_to_zone(
                ChooseSpec::Object(crate::filter::ObjectFilter::specific(anthem_id)),
                Zone::Graveyard,
                false,
            ),
            Effect::move_to_zone(ChooseSpec::Source, Zone::Battlefield, false),
            Effect::gain_life(Value::PowerOf(Box::new(ChooseSpec::Source))),
        ],
    );
    game.push_to_stack(entry);

    resolve_stack_entry(&mut game).expect("ability should resolve using source LKI");

    assert_eq!(
        game.player(alice).expect("Alice should exist").life,
        25,
        "608.2h requires source information to use the source's last battlefield existence, not an intermediate graveyard snapshot"
    );
}

#[test]
pub(super) fn test_source_lki_refreshes_when_sacrifice_source_moves_it() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let source_id = create_creature(&mut game, "Self-Sacrifice Source", alice, 2, 2);
    let entry = StackEntry::ability(
        source_id,
        alice,
        vec![
            Effect::sacrifice_source(),
            Effect::gain_life(Value::PowerOf(Box::new(ChooseSpec::Source))),
        ],
    );
    game.push_to_stack(entry);

    let anthem_card = CardBuilder::new(CardId::from_raw(9017), "Late Sacrifice Anthem")
        .card_types(vec![CardType::Enchantment])
        .build();
    let anthem_id = game.create_object_from_card(&anthem_card, alice, Zone::Battlefield);
    game.object_mut(anthem_id)
        .expect("anthem should exist")
        .abilities_mut()
        .push(Ability::static_ability(StaticAbility::anthem(
            crate::filter::ObjectFilter::creature(),
            3,
            0,
        )));
    game.refresh_continuous_state();

    assert_eq!(
        game.calculated_power(source_id),
        Some(5),
        "test setup should apply the late anthem before the source sacrifices itself"
    );

    resolve_stack_entry(&mut game).expect("ability should resolve using source LKI");

    assert_eq!(
        game.player(alice).expect("Alice should exist").life,
        25,
        "608.2h requires source LKI from immediately before sacrifice moved it"
    );
}

#[test]
pub(super) fn test_activation_cost_source_lki_uses_state_after_prior_costs() {
    use crate::ability::{ActivatedAbility, ActivationTiming};
    use crate::cost::TotalCost;
    use crate::decision::LegalAction;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let source_id = create_creature(&mut game, "Cost-Marked Source", alice, 2, 2);
    let costs = TotalCost::from_costs(vec![
        crate::costs::Cost::add_counters(crate::object::CounterType::PlusOnePlusOne, 1),
        crate::costs::Cost::sacrifice_self(),
    ]);
    game.object_mut(source_id)
        .expect("source should exist")
        .abilities_mut()
        .push(Ability {
            kind: AbilityKind::Activated(ActivatedAbility {
                mana_cost: costs,
                effects: crate::resolution::ResolutionProgram::from_effects(vec![
                    Effect::gain_life(Value::PowerOf(Box::new(ChooseSpec::Source))),
                ]),
                choices: vec![],
                timing: ActivationTiming::AnyTime,
                additional_restrictions: vec![],
                activation_restrictions: vec![],
                mana_output: None,
                activation_condition: None,
                mana_usage_restrictions: vec![],
                is_loyalty_ability: false,
            }),
            functional_zones: vec![Zone::Battlefield],
        });

    let ability_index = game
        .object(source_id)
        .expect("source should still exist")
        .abilities
        .iter()
        .position(|ability| matches!(ability.kind, AbilityKind::Activated(_)))
        .expect("source should have an activated ability");

    let mut trigger_queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = AutoPassDecisionMaker;

    let cost_order_ctx = match apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(LegalAction::ActivateAbility {
            source: source_id,
            ability_index,
        }),
        &mut dm,
    )
    .expect("activation should begin")
    {
        crate::decision::GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::SelectOptions(ctx),
        ) => ctx,
        other => panic!("expected activation to ask for cost order, got {other:?}"),
    };

    let add_counter_cost_index = cost_order_ctx
        .options
        .iter()
        .find(|option| option.description.to_ascii_lowercase().contains("counter"))
        .map(|option| option.index)
        .expect("expected an add-counter cost option");
    apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::NextCostChoice(add_counter_cost_index),
        &mut dm,
    )
    .expect("add-counter cost should be paid before sacrifice");

    assert_eq!(
        game.stack.len(),
        1,
        "the sacrifice-self cost should finish activation and put the ability on the stack"
    );
    assert!(
        !game.battlefield.contains(&source_id),
        "the source should be sacrificed as a cost before the ability resolves"
    );

    resolve_stack_entry(&mut game).expect("ability should resolve using source LKI");

    assert_eq!(
        game.player(alice).expect("Alice should exist").life,
        23,
        "113.7a/608.2h require activated ability source LKI from immediately before the sacrifice cost moved it, after prior costs modified it"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn tainted_sigil_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Tainted Sigil")
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "{T}, Sacrifice this artifact: You gain life equal to the total life lost by all players this turn. (Damage causes loss of life.)",
        )
        .expect("Tainted Sigil should parse")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn activate_tainted_sigil_after_life_losses(
    life_losses: &[(PlayerId, u32, bool)],
) -> (GameState, crate::ids::StableId) {
    use crate::decision::LegalAction;
    use crate::decisions::context::DecisionContext;
    use crate::events::{LifeLossEvent, RawEvent};
    use crate::provenance::ProvNodeId;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    for (player_id, amount, from_damage) in life_losses {
        game.player_mut(*player_id)
            .expect("life-loss player should exist")
            .life -= *amount as i32;
        let event = RawEvent::new(
            LifeLossEvent::new(*player_id, *amount, *from_damage),
            ProvNodeId::default(),
        );
        game.record_turn_history_event(&event);
    }

    let sigil = tainted_sigil_definition();
    let sigil_id = game.create_object_from_definition(&sigil, alice, Zone::Battlefield);
    let sigil_stable_id = game
        .object(sigil_id)
        .expect("Tainted Sigil should exist")
        .stable_id;
    let ability_index = game
        .object(sigil_id)
        .expect("Tainted Sigil should exist")
        .abilities
        .iter()
        .position(|ability| matches!(ability.kind, AbilityKind::Activated(_)))
        .expect("Tainted Sigil should have an activated ability");

    let mut trigger_queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = AutoPassDecisionMaker;
    let mut progress = apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(LegalAction::ActivateAbility {
            source: sigil_id,
            ability_index,
        }),
        &mut dm,
    )
    .expect("Tainted Sigil activation should start");

    for _ in 0..4 {
        if game.stack.len() == 1 {
            resolve_stack_entry(&mut game).expect("Tainted Sigil ability should resolve");
            return (game, sigil_stable_id);
        }

        progress = match progress {
            crate::decision::GameProgress::NeedsDecisionCtx(DecisionContext::SelectOptions(
                ctx,
            )) => {
                let option = ctx
                    .options
                    .iter()
                    .find(|option| {
                        let description = option.description.to_ascii_lowercase();
                        description.contains("tap") || description.contains("sacrifice")
                    })
                    .unwrap_or_else(|| {
                        panic!("expected a tap or sacrifice cost option, got {ctx:?}")
                    });
                apply_priority_response_with_dm(
                    &mut game,
                    &mut trigger_queue,
                    &mut state,
                    &PriorityResponse::NextCostChoice(option.index),
                    &mut dm,
                )
                .expect("Tainted Sigil cost choice should apply")
            }
            crate::decision::GameProgress::NeedsDecisionCtx(DecisionContext::SelectObjects(_)) => {
                apply_priority_response_with_dm(
                    &mut game,
                    &mut trigger_queue,
                    &mut state,
                    &PriorityResponse::SacrificeTarget(sigil_id),
                    &mut dm,
                )
                .expect("Tainted Sigil sacrifice choice should apply")
            }
            other => {
                panic!("expected Tainted Sigil activation to advance through costs, got {other:?}")
            }
        };
    }

    panic!("Tainted Sigil activation did not reach the stack");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_tainted_sigil_gains_total_life_lost_by_all_players_this_turn() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let (game, sigil_stable_id) =
        activate_tainted_sigil_after_life_losses(&[(alice, 3, false), (bob, 4, true)]);
    let graveyard_sigil_id = game
        .find_object_by_stable_id(sigil_stable_id)
        .expect("Tainted Sigil should remain tracked after sacrifice");

    assert_eq!(
        game.player(alice).expect("Alice should exist").life,
        24,
        "Alice should gain the 7 total life lost by all players this turn, including damage-caused life loss"
    );
    assert_eq!(
        game.player(bob).expect("Bob should exist").life,
        16,
        "Tainted Sigil should not change the opponent's life total"
    );
    assert!(
        game.player(alice)
            .expect("Alice should exist")
            .graveyard
            .contains(&graveyard_sigil_id),
        "Tainted Sigil should be sacrificed as an activation cost"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_tainted_sigil_gains_no_life_when_no_player_lost_life_this_turn() {
    let alice = PlayerId::from_index(0);

    let (game, sigil_stable_id) = activate_tainted_sigil_after_life_losses(&[]);
    let graveyard_sigil_id = game
        .find_object_by_stable_id(sigil_stable_id)
        .expect("Tainted Sigil should remain tracked after sacrifice");

    assert_eq!(
        game.player(alice).expect("Alice should exist").life,
        20,
        "Tainted Sigil should gain 0 life when no player lost life this turn"
    );
    assert!(
        game.player(alice)
            .expect("Alice should exist")
            .graveyard
            .contains(&graveyard_sigil_id),
        "Tainted Sigil should still be sacrificed when the dynamic amount is zero"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn final_punishment_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(90_018), "Final Punishment")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::Black],
            vec![ManaSymbol::Black],
        ]))
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Target player loses life equal to the damage already dealt to that player this turn.",
        )
        .expect("Final Punishment should parse")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn record_player_damage_this_turn(
    game: &mut GameState,
    source: ObjectId,
    player: PlayerId,
    amount: u32,
    is_combat: bool,
) {
    let event = TriggerEvent::new_with_provenance(
        crate::events::DamageEvent::with_cause(
            source,
            crate::events::DamageTarget::Player(player),
            amount,
            is_combat,
            crate::events::cause::EventCause::effect(),
        ),
        crate::provenance::ProvNodeId::default(),
    );
    game.record_turn_history_event(&event);
    game.player_mut(player)
        .expect("damaged player should exist")
        .life -= amount as i32;
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn final_punishment_makes_target_player_lose_life_equal_to_all_prior_damage_this_turn() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let damage_source = ObjectId::from_raw(90_019);

    record_player_damage_this_turn(&mut game, damage_source, bob, 3, true);
    record_player_damage_this_turn(&mut game, damage_source, bob, 4, false);
    record_player_damage_this_turn(&mut game, damage_source, alice, 5, false);
    game.player_mut(bob).expect("Bob should exist").life -= 2;

    let creature = create_creature(&mut game, "Irrelevant Target", bob, 1, 1);
    let creature_damage = TriggerEvent::new_with_provenance(
        crate::events::DamageEvent::with_cause(
            damage_source,
            crate::events::DamageTarget::Object(creature),
            9,
            false,
            crate::events::cause::EventCause::effect(),
        ),
        crate::provenance::ProvNodeId::default(),
    );
    game.record_turn_history_event(&creature_damage);

    let final_punishment = final_punishment_definition();
    let spell_id = game.create_object_from_definition(&final_punishment, alice, Zone::Stack);
    game.stack.push(
        crate::game_state::StackEntry::new(spell_id, alice)
            .with_targets(vec![crate::game_state::Target::Player(bob)])
            .with_target_assignments(vec![crate::game_state::TargetAssignment {
                spec: crate::target::ChooseSpec::target_player(),
                range: 0..1,
            }]),
    );

    let bob_life_before = game.player(bob).expect("Bob should exist").life;
    resolve_stack_entry(&mut game).expect("Final Punishment should resolve");

    assert_eq!(
        game.player(bob).expect("Bob should exist").life,
        bob_life_before - 7,
        "Final Punishment should count combat and noncombat damage already dealt to its target player, and ignore other life loss or other targets"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn final_punishment_makes_target_player_lose_no_life_when_they_were_not_damaged_this_turn()
 {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let damage_source = ObjectId::from_raw(90_020);

    record_player_damage_this_turn(&mut game, damage_source, alice, 6, false);

    let final_punishment = final_punishment_definition();
    let spell_id = game.create_object_from_definition(&final_punishment, alice, Zone::Stack);
    game.stack.push(
        crate::game_state::StackEntry::new(spell_id, alice)
            .with_targets(vec![crate::game_state::Target::Player(bob)])
            .with_target_assignments(vec![crate::game_state::TargetAssignment {
                spec: crate::target::ChooseSpec::target_player(),
                range: 0..1,
            }]),
    );

    let bob_life_before = game.player(bob).expect("Bob should exist").life;
    resolve_stack_entry(&mut game).expect("Final Punishment should resolve");

    assert_eq!(
        game.player(bob).expect("Bob should exist").life,
        bob_life_before,
        "Final Punishment should use the target player's damage total, not damage dealt to another player"
    );
}

#[test]
pub(super) fn test_source_lki_refreshes_when_exile_source_moves_it() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let source_id = create_creature(&mut game, "Self-Exile Source", alice, 2, 2);
    let entry = StackEntry::ability(
        source_id,
        alice,
        vec![
            Effect::new(crate::effects::ExileEffect::with_spec(ChooseSpec::Source)),
            Effect::gain_life(Value::PowerOf(Box::new(ChooseSpec::Source))),
        ],
    );
    game.push_to_stack(entry);

    let anthem_card = CardBuilder::new(CardId::from_raw(9018), "Late Exile Anthem")
        .card_types(vec![CardType::Enchantment])
        .build();
    let anthem_id = game.create_object_from_card(&anthem_card, alice, Zone::Battlefield);
    game.object_mut(anthem_id)
        .expect("anthem should exist")
        .abilities_mut()
        .push(Ability::static_ability(StaticAbility::anthem(
            crate::filter::ObjectFilter::creature(),
            3,
            0,
        )));
    game.refresh_continuous_state();

    assert_eq!(
        game.calculated_power(source_id),
        Some(5),
        "test setup should apply the late anthem before the source exiles itself"
    );

    resolve_stack_entry(&mut game).expect("ability should resolve using source LKI");

    assert_eq!(
        game.player(alice).expect("Alice should exist").life,
        25,
        "608.2h requires source LKI from immediately before exile moved it"
    );
}

#[test]
pub(super) fn test_source_lki_refreshes_when_destroy_source_moves_it() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let source_id = create_creature(&mut game, "Self-Destroy Source", alice, 2, 2);
    let entry = StackEntry::ability(
        source_id,
        alice,
        vec![
            Effect::new(crate::effects::DestroyEffect::with_spec(ChooseSpec::Source)),
            Effect::gain_life(Value::PowerOf(Box::new(ChooseSpec::Source))),
        ],
    );
    game.push_to_stack(entry);

    let anthem_card = CardBuilder::new(CardId::from_raw(9019), "Late Destroy Anthem")
        .card_types(vec![CardType::Enchantment])
        .build();
    let anthem_id = game.create_object_from_card(&anthem_card, alice, Zone::Battlefield);
    game.object_mut(anthem_id)
        .expect("anthem should exist")
        .abilities_mut()
        .push(Ability::static_ability(StaticAbility::anthem(
            crate::filter::ObjectFilter::creature(),
            3,
            0,
        )));
    game.refresh_continuous_state();

    assert_eq!(
        game.calculated_power(source_id),
        Some(5),
        "test setup should apply the late anthem before the source destroys itself"
    );

    resolve_stack_entry(&mut game).expect("ability should resolve using source LKI");

    assert_eq!(
        game.player(alice).expect("Alice should exist").life,
        25,
        "608.2h requires source LKI from immediately before destroy moved it"
    );
}

#[test]
pub(super) fn test_source_lki_refreshes_when_return_source_to_hand_moves_it() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let source_id = create_creature(&mut game, "Self-Bounce Source", alice, 2, 2);
    let entry = StackEntry::ability(
        source_id,
        alice,
        vec![
            Effect::return_source_to_hand_as_cost(),
            Effect::gain_life(Value::PowerOf(Box::new(ChooseSpec::Source))),
        ],
    );
    game.push_to_stack(entry);

    let anthem_card = CardBuilder::new(CardId::from_raw(9020), "Late Bounce Anthem")
        .card_types(vec![CardType::Enchantment])
        .build();
    let anthem_id = game.create_object_from_card(&anthem_card, alice, Zone::Battlefield);
    game.object_mut(anthem_id)
        .expect("anthem should exist")
        .abilities_mut()
        .push(Ability::static_ability(StaticAbility::anthem(
            crate::filter::ObjectFilter::creature(),
            3,
            0,
        )));
    game.refresh_continuous_state();

    assert_eq!(
        game.calculated_power(source_id),
        Some(5),
        "test setup should apply the late anthem before the source returns to hand"
    );

    resolve_stack_entry(&mut game).expect("ability should resolve using source LKI");

    assert_eq!(
        game.player(alice).expect("Alice should exist").life,
        25,
        "608.2h requires source LKI from immediately before return-to-hand moved it"
    );
}

#[test]
pub(super) fn test_source_lki_is_from_expected_zone_not_later_zone_changes() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let source_id = create_creature(&mut game, "Buffed Departing Source", alice, 2, 2);
    let anthem_card = CardBuilder::new(CardId::from_raw(9008), "Battlefield Power Anthem")
        .card_types(vec![CardType::Enchantment])
        .build();
    let anthem_id = game.create_object_from_card(&anthem_card, alice, Zone::Battlefield);
    game.object_mut(anthem_id)
        .expect("anthem should exist")
        .abilities_mut()
        .push(Ability::static_ability(StaticAbility::anthem(
            crate::filter::ObjectFilter::creature(),
            3,
            0,
        )));
    game.refresh_continuous_state();

    assert_eq!(
        game.calculated_power(source_id),
        Some(5),
        "test setup should apply the battlefield continuous effect before the source leaves"
    );

    let entry = StackEntry::ability(
        source_id,
        alice,
        vec![Effect::gain_life(Value::PowerOf(Box::new(
            ChooseSpec::Source,
        )))],
    );
    game.push_to_stack(entry);

    let graveyard_id = game
        .move_object_by_effect(source_id, Zone::Graveyard)
        .expect("source should leave its expected battlefield zone");
    game.move_object_by_effect(graveyard_id, Zone::Exile)
        .expect("source can move again before the ability resolves");

    resolve_stack_entry(&mut game).expect("ability should resolve using source LKI");

    assert_eq!(
        game.player(alice).expect("Alice should exist").life,
        25,
        "113.7a/608.2h require source LKI from its last existence on the battlefield, not a later graveyard or exile snapshot"
    );
}

#[test]
pub(super) fn test_source_lki_ignores_returned_new_object_in_expected_zone() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let source_id = create_creature(&mut game, "Returning Departed Source", alice, 2, 2);
    let entry = StackEntry::ability(
        source_id,
        alice,
        vec![Effect::gain_life(Value::PowerOf(Box::new(
            ChooseSpec::Source,
        )))],
    );
    game.push_to_stack(entry);

    let graveyard_id = game
        .move_object_by_effect(source_id, Zone::Graveyard)
        .expect("source should leave its expected battlefield zone");
    let returned_id = game
        .move_object_by_effect(graveyard_id, Zone::Battlefield)
        .expect("source can return before the ability resolves as a new object");

    let anthem_card = CardBuilder::new(CardId::from_raw(9010), "New Object Power Anthem")
        .card_types(vec![CardType::Enchantment])
        .build();
    let anthem_id = game.create_object_from_card(&anthem_card, alice, Zone::Battlefield);
    game.object_mut(anthem_id)
        .expect("anthem should exist")
        .abilities_mut()
        .push(Ability::static_ability(StaticAbility::anthem(
            crate::filter::ObjectFilter::creature(),
            3,
            0,
        )));
    game.refresh_continuous_state();

    assert_eq!(
        game.calculated_power(returned_id),
        Some(5),
        "test setup should make the returned new object a 5-power creature"
    );

    resolve_stack_entry(&mut game).expect("ability should resolve using source LKI");

    assert_eq!(
        game.player(alice).expect("Alice should exist").life,
        22,
        "113.7a/608.2h require source LKI from the old object's last battlefield existence, not the returned new object"
    );
}

#[test]
pub(super) fn test_resolution_uses_target_lki_after_effect_moves_it_to_hidden_zone() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let source_card = CardBuilder::new(CardId::from_raw(9006), "Hidden Zone LKI Source")
        .card_types(vec![CardType::Instant])
        .build();
    let source_id = game.create_object_from_card(&source_card, alice, Zone::Stack);

    let target_id = create_creature(&mut game, "Buffed Bounce Target", alice, 2, 2);
    let anthem_card = CardBuilder::new(CardId::from_raw(9007), "Battlefield Anthem")
        .card_types(vec![CardType::Enchantment])
        .build();
    let anthem_id = game.create_object_from_card(&anthem_card, alice, Zone::Battlefield);
    game.object_mut(anthem_id)
        .expect("anthem should exist")
        .abilities_mut()
        .push(Ability::static_ability(StaticAbility::anthem(
            crate::filter::ObjectFilter::creature(),
            3,
            0,
        )));
    game.refresh_continuous_state();

    assert_eq!(
        game.calculated_power(target_id),
        Some(5),
        "test setup should make the target a 5-power creature on the battlefield"
    );

    let target_spec = ChooseSpec::target(ChooseSpec::Object(
        crate::filter::ObjectFilter::creature().in_zone(Zone::Battlefield),
    ));
    let entry = StackEntry::ability(
        source_id,
        alice,
        vec![
            Effect::move_to_zone(target_spec.clone(), Zone::Hand, false),
            Effect::gain_life(Value::PowerOf(Box::new(target_spec))),
        ],
    )
    .with_targets(vec![Target::Object(target_id)]);
    game.push_to_stack(entry);

    resolve_stack_entry(&mut game).expect("effect should resolve using target LKI");

    assert_eq!(
        game.player(alice).expect("Alice should exist").life,
        25,
        "608.2h should use the target's last known battlefield power after this effect moves it to hand"
    );
}

#[test]
pub(super) fn test_target_lki_refreshes_when_effect_modifies_target_before_moving_it() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let source_card = CardBuilder::new(CardId::from_raw(9012), "Target LKI Refresher")
        .card_types(vec![CardType::Instant])
        .build();
    let source_id = game.create_object_from_card(&source_card, alice, Zone::Stack);

    let target_id = create_creature(&mut game, "Countered Bounce Target", alice, 2, 2);
    let target_spec = ChooseSpec::target(ChooseSpec::Object(
        crate::filter::ObjectFilter::creature().in_zone(Zone::Battlefield),
    ));
    let entry = StackEntry::ability(
        source_id,
        alice,
        vec![
            Effect::put_counters(
                crate::object::CounterType::PlusOnePlusOne,
                3,
                target_spec.clone(),
            ),
            Effect::move_to_zone(target_spec.clone(), Zone::Hand, false),
            Effect::gain_life(Value::PowerOf(Box::new(target_spec))),
        ],
    )
    .with_targets(vec![Target::Object(target_id)]);
    game.push_to_stack(entry);

    resolve_stack_entry(&mut game).expect("effect should resolve using refreshed target LKI");

    assert_eq!(
        game.player(alice).expect("Alice should exist").life,
        25,
        "608.2h requires target LKI from immediately before it left, including counters added earlier in the same resolution"
    );
}

#[test]
pub(super) fn test_target_lki_refreshes_when_sacrifice_moves_target() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let source_card = CardBuilder::new(CardId::from_raw(9013), "Sacrifice LKI Refresher")
        .card_types(vec![CardType::Instant])
        .build();
    let source_id = game.create_object_from_card(&source_card, alice, Zone::Stack);

    let target_id = create_creature(&mut game, "Countered Sacrifice Target", alice, 2, 2);
    let target_spec = ChooseSpec::target(ChooseSpec::Object(
        crate::filter::ObjectFilter::creature().in_zone(Zone::Battlefield),
    ));
    let entry = StackEntry::ability(
        source_id,
        alice,
        vec![
            Effect::put_counters(
                crate::object::CounterType::PlusOnePlusOne,
                3,
                target_spec.clone(),
            ),
            Effect::new(crate::effects::SacrificeTargetEffect::new(
                target_spec.clone(),
            )),
            Effect::gain_life(Value::PowerOf(Box::new(target_spec))),
        ],
    )
    .with_targets(vec![Target::Object(target_id)]);
    game.push_to_stack(entry);

    resolve_stack_entry(&mut game).expect("effect should resolve using sacrifice target LKI");

    assert_eq!(
        game.player(alice).expect("Alice should exist").life,
        25,
        "608.2h requires target LKI from immediately before sacrifice moved it to the graveyard"
    );
}

#[test]
pub(super) fn test_target_lki_refreshes_when_destroy_moves_target() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let source_card = CardBuilder::new(CardId::from_raw(9014), "Destroy LKI Refresher")
        .card_types(vec![CardType::Instant])
        .build();
    let source_id = game.create_object_from_card(&source_card, alice, Zone::Stack);

    let target_id = create_creature(&mut game, "Countered Destroy Target", alice, 2, 2);
    let target_spec = ChooseSpec::target(ChooseSpec::Object(
        crate::filter::ObjectFilter::creature().in_zone(Zone::Battlefield),
    ));
    let entry = StackEntry::ability(
        source_id,
        alice,
        vec![
            Effect::put_counters(
                crate::object::CounterType::PlusOnePlusOne,
                3,
                target_spec.clone(),
            ),
            Effect::destroy(target_spec.clone()),
            Effect::gain_life(Value::PowerOf(Box::new(target_spec))),
        ],
    )
    .with_targets(vec![Target::Object(target_id)]);
    game.push_to_stack(entry);

    resolve_stack_entry(&mut game).expect("effect should resolve using destroy target LKI");

    assert_eq!(
        game.player(alice).expect("Alice should exist").life,
        25,
        "608.2h requires target LKI from immediately before destroy moved it to the graveyard"
    );
}

#[test]
pub(super) fn test_target_lki_refreshes_when_exile_moves_target() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let source_card = CardBuilder::new(CardId::from_raw(9015), "Exile LKI Refresher")
        .card_types(vec![CardType::Instant])
        .build();
    let source_id = game.create_object_from_card(&source_card, alice, Zone::Stack);

    let target_id = create_creature(&mut game, "Countered Exile Target", alice, 2, 2);
    let target_spec = ChooseSpec::target(ChooseSpec::Object(
        crate::filter::ObjectFilter::creature().in_zone(Zone::Battlefield),
    ));
    let entry = StackEntry::ability(
        source_id,
        alice,
        vec![
            Effect::put_counters(
                crate::object::CounterType::PlusOnePlusOne,
                3,
                target_spec.clone(),
            ),
            Effect::exile(target_spec.clone()),
            Effect::gain_life(Value::PowerOf(Box::new(target_spec))),
        ],
    )
    .with_targets(vec![Target::Object(target_id)]);
    game.push_to_stack(entry);

    resolve_stack_entry(&mut game).expect("effect should resolve using exile target LKI");

    assert_eq!(
        game.player(alice).expect("Alice should exist").life,
        25,
        "608.2h requires target LKI from immediately before exile moved it"
    );
}

#[test]
pub(super) fn test_target_lki_refreshes_when_return_to_hand_moves_target() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let source_card = CardBuilder::new(CardId::from_raw(9016), "Bounce LKI Refresher")
        .card_types(vec![CardType::Instant])
        .build();
    let source_id = game.create_object_from_card(&source_card, alice, Zone::Stack);

    let target_id = create_creature(&mut game, "Countered Bounce Target", alice, 2, 2);
    let target_spec = ChooseSpec::target(ChooseSpec::Object(
        crate::filter::ObjectFilter::creature().in_zone(Zone::Battlefield),
    ));
    let entry = StackEntry::ability(
        source_id,
        alice,
        vec![
            Effect::put_counters(
                crate::object::CounterType::PlusOnePlusOne,
                3,
                target_spec.clone(),
            ),
            Effect::new(crate::effects::ReturnToHandEffect::with_spec(
                target_spec.clone(),
            )),
            Effect::gain_life(Value::PowerOf(Box::new(target_spec))),
        ],
    )
    .with_targets(vec![Target::Object(target_id)]);
    game.push_to_stack(entry);

    resolve_stack_entry(&mut game).expect("effect should resolve using return-to-hand target LKI");

    assert_eq!(
        game.player(alice).expect("Alice should exist").life,
        25,
        "608.2h requires target LKI from immediately before return-to-hand moved it"
    );
}

#[test]
pub(super) fn test_target_lki_refreshes_when_move_to_library_moves_target() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let source_card = CardBuilder::new(CardId::from_raw(9021), "Library LKI Refresher")
        .card_types(vec![CardType::Instant])
        .build();
    let source_id = game.create_object_from_card(&source_card, alice, Zone::Stack);

    let target_id = create_creature(&mut game, "Countered Library Target", alice, 2, 2);
    let target_spec = ChooseSpec::target(ChooseSpec::Object(
        crate::filter::ObjectFilter::creature().in_zone(Zone::Battlefield),
    ));
    let entry = StackEntry::ability(
        source_id,
        alice,
        vec![
            Effect::put_counters(
                crate::object::CounterType::PlusOnePlusOne,
                3,
                target_spec.clone(),
            ),
            Effect::new(crate::effects::MoveToLibraryNthFromTopEffect::new(
                target_spec.clone(),
                Value::Fixed(1),
            )),
            Effect::gain_life(Value::PowerOf(Box::new(target_spec))),
        ],
    )
    .with_targets(vec![Target::Object(target_id)]);
    game.push_to_stack(entry);

    resolve_stack_entry(&mut game).expect("effect should resolve using library target LKI");

    assert_eq!(
        game.player(alice).expect("Alice should exist").life,
        25,
        "608.2h requires target LKI from immediately before library movement"
    );
}

#[test]
pub(super) fn test_target_lki_refreshes_when_shuffle_into_library_moves_target() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let source_card = CardBuilder::new(CardId::from_raw(9022), "Shuffle LKI Refresher")
        .card_types(vec![CardType::Instant])
        .build();
    let source_id = game.create_object_from_card(&source_card, alice, Zone::Stack);

    let target_id = create_creature(&mut game, "Countered Shuffle Target", alice, 2, 2);
    let target_spec = ChooseSpec::target(ChooseSpec::Object(
        crate::filter::ObjectFilter::creature().in_zone(Zone::Battlefield),
    ));
    let entry = StackEntry::ability(
        source_id,
        alice,
        vec![
            Effect::put_counters(
                crate::object::CounterType::PlusOnePlusOne,
                3,
                target_spec.clone(),
            ),
            Effect::shuffle_objects_into_library(target_spec.clone(), PlayerFilter::You),
            Effect::gain_life(Value::PowerOf(Box::new(target_spec))),
        ],
    )
    .with_targets(vec![Target::Object(target_id)]);
    game.push_to_stack(entry);

    resolve_stack_entry(&mut game).expect("effect should resolve using shuffle target LKI");

    assert_eq!(
        game.player(alice).expect("Alice should exist").life,
        25,
        "608.2h requires target LKI from immediately before shuffle-into-library moved it"
    );
}

#[test]
pub(super) fn optional_cost_intervening_if_trigger_uses_stack_entry_paid_state_after_source_leaves()
{
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let source_card = CardBuilder::new(CardId::from_raw(71_100), "Bargained Source")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let source = game.create_object_from_card(&source_card, alice, Zone::Battlefield);

    let optional_costs = vec![crate::cost::OptionalCost::custom(
        "Bargain",
        crate::cost::TotalCost::free(),
    )];
    let mut paid = crate::cost::OptionalCostsPaid::from_costs(&optional_costs);
    paid.pay(0);
    if let Some(obj) = game.object_mut(source) {
        obj.optional_costs = optional_costs.into();
        obj.optional_costs_paid = paid.clone();
    }
    let source_snapshot = crate::snapshot::ObjectSnapshot::from_object(
        game.object(source).expect("source should exist"),
        &game,
    );
    let event = TriggerEvent::new_with_provenance(
        EnterBattlefieldEvent::new(source, Zone::Stack),
        crate::provenance::ProvNodeId::default(),
    );
    let stack_entry = StackEntry::ability(source, alice, vec![Effect::gain_life(3)])
        .with_optional_costs_paid(paid)
        .with_source_info(source_snapshot.stable_id, source_snapshot.name.to_string())
        .with_source_snapshot(source_snapshot)
        .with_triggering_event(event)
        .with_intervening_if(crate::effect::Condition::ThisSpellPaidLabel(
            "Bargain".into(),
        ));
    game.push_to_stack(stack_entry);

    game.move_object_by_effect(source, Zone::Graveyard)
        .expect("source should leave before its trigger resolves");

    resolve_stack_entry(&mut game).expect("trigger should resolve");

    assert_eq!(
        game.player(alice).expect("Alice should exist").life,
        23,
        "intervening-if should use the trigger stack entry's captured optional-cost state"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_offspring_trigger_resolves_after_source_leaves_battlefield() {
    use crate::ability::AbilityKind;
    use crate::cards::builders::CardDefinitionBuilder;
    use crate::cost::OptionalCostsPaid;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let def = CardDefinitionBuilder::new(CardId::new(), "Offspring Trigger Test")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![crate::types::Subtype::Rabbit])
        .power_toughness(PowerToughness::fixed(3, 3))
        .parse_text("Offspring {2}\nFlying")
        .expect("offspring ability should parse");

    let source = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let mut paid = OptionalCostsPaid::from_costs(&def.optional_costs);
    paid.pay(0);
    let source_snapshot = crate::snapshot::ObjectSnapshot::from_object(
        game.object(source).expect("source should exist"),
        &game,
    );
    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) if triggered.trigger.display().contains("enters") => {
                Some(triggered.clone())
            }
            _ => None,
        })
        .expect("offspring ETB trigger should exist");
    game.object_mut(source)
        .expect("source should exist")
        .optional_costs_paid = paid.clone();

    let stack_entry = StackEntry::ability(source, alice, triggered.effects.clone())
        .with_optional_costs_paid(paid)
        .with_source_info(source_snapshot.stable_id, source_snapshot.name.to_string())
        .with_source_snapshot(source_snapshot)
        .with_intervening_if(crate::effect::Condition::ThisSpellPaidLabel(
            "Offspring".into(),
        ));
    game.push_to_stack(stack_entry);

    game.move_object_by_effect(source, Zone::Graveyard)
        .expect("source should move to graveyard before resolution");

    resolve_stack_entry(&mut game).expect("offspring trigger should resolve from LKI");

    let tokens: Vec<_> = game
        .battlefield
        .iter()
        .filter_map(|&id| game.object(id))
        .filter(|obj| game.controller_of(obj) == alice && obj.kind == ObjectKind::Token)
        .filter(|obj| obj.name == "Offspring Trigger Test")
        .collect();
    assert_eq!(tokens.len(), 1, "expected one offspring token");
    assert_eq!(tokens[0].power(), Some(1), "offspring token should be 1/1");
    assert_eq!(
        tokens[0].toughness(),
        Some(1),
        "offspring token should be 1/1"
    );
}

#[test]
pub(super) fn test_delayed_tagged_graveyard_return_resolves() {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let alice = PlayerId::from_index(0);
    let reanimator_id = create_delayed_reanimator(&mut game, alice, "Delayed Reanimator");

    let first_graveyard_id = game
        .move_object_by_effect(reanimator_id, Zone::Graveyard)
        .expect("creature should move to graveyard");
    drain_pending_trigger_events(&mut game, &mut trigger_queue);
    put_triggers_on_stack(&mut game, &mut trigger_queue).expect("put dies trigger on stack");
    while !game.stack_is_empty() {
        resolve_stack_entry(&mut game).expect("resolve dies trigger");
    }
    assert_eq!(game.effect_store.delayed_triggers.len(), 1);
    assert!(
        game.players[0].graveyard.contains(&first_graveyard_id),
        "creature should still be in graveyard before delayed trigger resolves"
    );

    let end_step_event = TriggerEvent::new_with_provenance(
        crate::events::phase::BeginningOfEndStepEvent::new(game.turn.active_player),
        crate::provenance::ProvNodeId::default(),
    );
    for trigger in crate::triggers::check_delayed_triggers(&mut game, &end_step_event) {
        trigger_queue.add(trigger);
    }
    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("put delayed end-step trigger on stack");
    while !game.stack_is_empty() {
        resolve_stack_entry(&mut game).expect("resolve delayed return");
    }

    assert!(
        game.battlefield.iter().any(|id| {
            game.object(*id)
                .is_some_and(|obj| obj.name == "Delayed Reanimator")
        }),
        "creature should return from graveyard at next end step"
    );
}

#[test]
pub(super) fn test_delayed_tagged_graveyard_return_does_not_follow_zone_hops() {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let alice = PlayerId::from_index(0);
    let reanimator_id = create_delayed_reanimator(&mut game, alice, "Delayed Reanimator");

    let first_graveyard_id = game
        .move_object_by_effect(reanimator_id, Zone::Graveyard)
        .expect("creature should move to graveyard");
    drain_pending_trigger_events(&mut game, &mut trigger_queue);
    put_triggers_on_stack(&mut game, &mut trigger_queue).expect("put dies trigger on stack");
    while !game.stack_is_empty() {
        resolve_stack_entry(&mut game).expect("resolve dies trigger");
    }
    assert_eq!(game.effect_store.delayed_triggers.len(), 1);

    let exile_id = game
        .move_object_by_effect(first_graveyard_id, Zone::Exile)
        .expect("creature should move to exile");
    let second_graveyard_id = game
        .move_object_by_effect(exile_id, Zone::Graveyard)
        .expect("creature should move back to graveyard");
    assert_ne!(second_graveyard_id, first_graveyard_id);

    let end_step_event = TriggerEvent::new_with_provenance(
        crate::events::phase::BeginningOfEndStepEvent::new(game.turn.active_player),
        crate::provenance::ProvNodeId::default(),
    );
    for trigger in crate::triggers::check_delayed_triggers(&mut game, &end_step_event) {
        trigger_queue.add(trigger);
    }
    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("put delayed end-step trigger on stack");
    while !game.stack_is_empty() {
        resolve_stack_entry(&mut game).expect("resolve delayed return");
    }

    assert!(
        game.players[0].graveyard.contains(&second_graveyard_id),
        "creature should stay in graveyard after zone-hop (original instance is lost)"
    );
    assert!(
        !game.battlefield.iter().any(|id| {
            game.object(*id)
                .is_some_and(|obj| obj.name == "Delayed Reanimator")
        }),
        "delayed return should not follow a different graveyard instance"
    );
}

pub(super) fn assert_pact_upkeep_trigger_survives_fail_to_find(
    pact_def: &crate::cards::CardDefinition,
    spell_name: &str,
) {
    struct PactDecisionMaker;

    impl DecisionMaker for PactDecisionMaker {
        fn decide_boolean(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::BooleanContext,
        ) -> bool {
            false
        }

        fn decide_objects(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            if ctx.min == 0 {
                Vec::new()
            } else {
                ctx.candidates
                    .iter()
                    .filter(|candidate| candidate.legal)
                    .map(|candidate| candidate.id)
                    .take(ctx.min)
                    .collect()
            }
        }
    }

    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let alice = PlayerId::from_index(0);
    let spell_debug = format!("{:?}", pact_def.spell_effect);
    let ability_debug = format!("{:?}", pact_def.abilities);

    let green_creature = CardBuilder::new(CardId::from_raw(91_001), "Green Probe")
        .mana_cost(crate::mana::ManaCost::from_pips(vec![vec![
            crate::mana::ManaSymbol::Green,
        ]]))
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    game.create_object_from_card(&green_creature, alice, Zone::Library);
    let pact_id = game.create_object_from_definition(pact_def, alice, Zone::Stack);
    game.stack.push(StackEntry::new(pact_id, alice));

    let mut dm = PactDecisionMaker;
    resolve_stack_entry_with(&mut game, &mut dm).expect("pact spell should resolve");

    assert_eq!(
        game.player(alice).expect("alice exists").hand.len(),
        0,
        "{spell_name} should leave the searched creature in the library when the player fails to find"
    );
    assert_eq!(
        game.effect_store.delayed_triggers.len(),
        1,
        "{spell_name} should still schedule the upkeep payment trigger after the spell resolves; spell_effect={spell_debug}; abilities={ability_debug}"
    );

    let same_turn_upkeep = TriggerEvent::new_with_provenance(
        crate::events::phase::BeginningOfUpkeepEvent::new(alice),
        crate::provenance::ProvNodeId::default(),
    );
    assert!(
        crate::triggers::check_delayed_triggers(&mut game, &same_turn_upkeep).is_empty(),
        "the pact trigger should not fire again during the turn it was created"
    );

    game.turn.turn_number += 2;
    game.turn.active_player = alice;
    let next_upkeep = TriggerEvent::new_with_provenance(
        crate::events::phase::BeginningOfUpkeepEvent::new(alice),
        crate::provenance::ProvNodeId::default(),
    );
    for trigger in crate::triggers::check_delayed_triggers(&mut game, &next_upkeep) {
        trigger_queue.add(trigger);
    }

    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "{spell_name} should fire on the next upkeep even after a fail-to-find search"
    );

    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("pact delayed trigger should be put on the stack");
    resolve_stack_entry_with(&mut game, &mut dm).expect("pact delayed trigger should resolve");

    assert!(
        game.player(alice).expect("alice exists").has_lost,
        "declining to pay for {spell_name} should make the controller lose the game"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_pact_upkeep_trigger_still_fires_after_fail_to_find() {
    let pact_def = CardDefinitionBuilder::new(CardId::from_raw(91_002), "Summoner's Pact Probe")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Search your library for a green creature card, reveal it, put it into your hand, then shuffle. At the beginning of your next upkeep, pay {2}{G}{G}. If you don't, you lose the game.",
        )
        .expect("pact runtime probe should parse");

    assert_pact_upkeep_trigger_survives_fail_to_find(&pact_def, "Summoner's Pact Probe");
}

#[cfg(feature = "generated-registry")]
#[test]
pub(super) fn test_generated_summoners_pact_upkeep_trigger_survives_fail_to_find() {
    let mut registry = crate::cards::CardRegistry::new();
    registry.ensure_cards_loaded(["Summoner's Pact"]);

    let pact_def = registry
        .get("Summoner's Pact")
        .expect("generated registry should load Summoner's Pact");

    assert_pact_upkeep_trigger_survives_fail_to_find(pact_def, "Summoner's Pact");
}

#[cfg(feature = "generated-registry")]
#[test]
pub(super) fn test_generated_phlage_is_castable_with_pool_mana_for_generic_component() {
    use crate::alternative_cast::CastingMethod;
    use crate::decision::{LegalAction, compute_legal_actions};

    let mut registry = crate::cards::CardRegistry::new();
    registry.ensure_cards_loaded(["Phlage, Titan of Fire's Fury"]);

    let phlage_def = registry
        .get("Phlage, Titan of Fire's Fury")
        .expect("generated registry should load Phlage");

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;

    let phlage_id =
        game.create_object_from_catalog_definition(phlage_def, &registry, alice, Zone::Hand);
    {
        let player = game.player_mut(alice).expect("Alice should exist");
        player.mana_pool.add(ManaSymbol::White, 1);
        player.mana_pool.add(ManaSymbol::Black, 1);
        player.mana_pool.add(ManaSymbol::Red, 1);
    }

    let actions = compute_legal_actions(&game, alice);
    assert!(
        actions.iter().any(|action| matches!(
            action,
            LegalAction::CastSpell {
                spell_id,
                from_zone: Zone::Hand,
                casting_method: CastingMethod::Normal,
            } if *spell_id == phlage_id
        )),
        "Phlage should be castable from hand with white, red, and one generic mana; actions: {actions:#?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_fatal_push_without_revolt_does_not_destroy_four_mana_target() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let fatal_push = crate::cards::CardDefinitionBuilder::new(CardId::from_raw(468), "Fatal Push")
            .card_types(vec![CardType::Instant])
            .parse_text(
                "Destroy target creature if it has mana value 2 or less.\nRevolt — Destroy that creature if it has mana value 4 or less instead if a permanent left the battlefield under your control this turn.",
            )
            .expect("fatal push definition should parse");
    let four_mana_creature = CardBuilder::new(CardId::from_raw(9003), "Four Mana Creature")
        .mana_cost(crate::mana::ManaCost::from_pips(vec![
            vec![crate::mana::ManaSymbol::Generic(2)],
            vec![crate::mana::ManaSymbol::Black],
            vec![crate::mana::ManaSymbol::Black],
        ]))
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 4))
        .build();

    let target_id = game.create_object_from_card(&four_mana_creature, bob, Zone::Battlefield);
    let fatal_push_id = game.create_object_from_definition(&fatal_push, alice, Zone::Stack);

    game.push_to_stack(
        StackEntry::new(fatal_push_id, alice).with_targets(vec![Target::Object(target_id)]),
    );

    resolve_stack_entry(&mut game).expect("fatal push should resolve");

    assert!(
        game.battlefield.contains(&target_id),
        "without revolt, Fatal Push should not destroy a mana value 4 creature"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_fatal_push_with_revolt_destroys_four_mana_target() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let fatal_push = crate::cards::CardDefinitionBuilder::new(CardId::from_raw(468), "Fatal Push")
            .card_types(vec![CardType::Instant])
            .parse_text(
                "Destroy target creature if it has mana value 2 or less.\nRevolt — Destroy that creature if it has mana value 4 or less instead if a permanent left the battlefield under your control this turn.",
            )
            .expect("fatal push definition should parse");
    let four_mana_creature = CardBuilder::new(CardId::from_raw(9004), "Four Mana Creature")
        .mana_cost(crate::mana::ManaCost::from_pips(vec![
            vec![crate::mana::ManaSymbol::Generic(2)],
            vec![crate::mana::ManaSymbol::Black],
            vec![crate::mana::ManaSymbol::Black],
        ]))
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 4))
        .build();

    let target_id = game.create_object_from_card(&four_mana_creature, bob, Zone::Battlefield);
    let fatal_push_id = game.create_object_from_definition(&fatal_push, alice, Zone::Stack);

    let revolt_permanent = CardBuilder::new(CardId::from_raw(9981), "Revolt Relic")
        .card_types(vec![CardType::Artifact])
        .build();
    let revolt_id = game.create_object_from_card(&revolt_permanent, alice, Zone::Battlefield);
    game.move_object_by_effect(revolt_id, Zone::Graveyard);

    game.push_to_stack(
        StackEntry::new(fatal_push_id, alice).with_targets(vec![Target::Object(target_id)]),
    );

    resolve_stack_entry(&mut game).expect("fatal push should resolve");

    assert!(
        !game.battlefield.contains(&target_id),
        "with revolt, Fatal Push should destroy a mana value 4 creature"
    );
    assert!(
        game.player(bob).is_some_and(|player| {
            player.graveyard.iter().any(|graveyard_id| {
                game.object(*graveyard_id)
                    .is_some_and(|object| object.name == "Four Mana Creature")
            })
        }),
        "target should be in graveyard after being destroyed"
    );
}

// === Combat Damage Tests ===

#[test]
pub(super) fn test_unblocked_attacker_deals_damage() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let attacker_id = create_creature(&mut game, "Attacker", alice, 3, 3);

    // Set up combat with attacker attacking Bob
    let mut combat = CombatState::default();
    combat.attackers.push(crate::combat_state::AttackerInfo {
        creature: attacker_id,
        target: AttackTarget::Player(bob),
    });
    combat.blockers.insert(attacker_id, Vec::new());

    // Execute combat damage
    let events = execute_combat_damage_step(&mut game, &combat, false);

    assert_eq!(events.len(), 1);
    assert_eq!(events[0].amount, 3);

    // Bob should have taken 3 damage
    assert_eq!(game.player(bob).unwrap().life, 17);
}

#[test]
pub(super) fn test_unblocked_attacker_uses_calculated_power_from_conditional_anthem() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let attacker_id = create_creature(&mut game, "Tek Test", alice, 2, 2);

    if let Some(attacker) = game.object_mut(attacker_id) {
        let swamp_condition = crate::ConditionExpr::CountComparison {
            count: crate::static_abilities::AnthemCountExpression::MatchingFilter(
                crate::filter::ObjectFilter::land()
                    .with_subtype(crate::types::Subtype::Swamp)
                    .you_control(),
            ),
            comparison: crate::effect::Comparison::GreaterThanOrEqual(1),
            display: Some("you control a Swamp".to_string()),
        };
        let anthem =
            crate::static_abilities::Anthem::for_source(2, 0).with_condition(swamp_condition);
        attacker.abilities_mut().push(Ability::static_ability(
            crate::static_abilities::StaticAbility::new(anthem),
        ));
    }

    let swamp = CardBuilder::new(CardId::new(), "Swamp")
        .card_types(vec![CardType::Land])
        .subtypes(vec![crate::types::Subtype::Swamp])
        .build();
    game.create_object_from_card(&swamp, alice, Zone::Battlefield);

    let mut combat = CombatState::default();
    combat.attackers.push(crate::combat_state::AttackerInfo {
        creature: attacker_id,
        target: AttackTarget::Player(bob),
    });
    combat.blockers.insert(attacker_id, Vec::new());

    let events = execute_combat_damage_step(&mut game, &combat, false);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].amount, 4);
    assert_eq!(game.player(bob).unwrap().life, 16);
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_earthbent_land_deals_combat_damage_using_counter_boosted_power() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let source_id = create_creature(&mut game, "Earthbender Source", alice, 1, 1);
    let land_id = game.create_object_from_definition(
        &crate::cards::definitions::basic_mountain(),
        alice,
        Zone::Battlefield,
    );

    let effect = Effect::new(crate::effects::EarthbendEffect::new(
        ChooseSpec::SpecificObject(land_id),
        8,
    ));
    let mut ctx = ExecutionContext::new_default(source_id, alice);
    execute_effect(&mut game, &effect, &mut ctx).expect("earthbend should resolve");

    assert_eq!(
        game.calculated_power(land_id),
        Some(8),
        "earthbent land should be an 8-power creature after counters"
    );
    assert_eq!(
        game.calculated_toughness(land_id),
        Some(8),
        "earthbent land should be an 8-toughness creature after counters"
    );

    let mut combat = CombatState::default();
    combat.attackers.push(crate::combat_state::AttackerInfo {
        creature: land_id,
        target: AttackTarget::Player(bob),
    });
    combat.blockers.insert(land_id, Vec::new());

    let events = execute_combat_damage_step(&mut game, &combat, false);
    assert_eq!(events.len(), 1, "earthbent land should deal combat damage");
    assert_eq!(
        events[0].amount, 8,
        "earthbent land should hit for 8 damage"
    );
    assert_eq!(
        game.player(bob).unwrap().life,
        12,
        "unblocked earthbent land should make Bob lose 8 life"
    );
}

#[test]
pub(super) fn test_unblocked_attacker_uses_toughness_for_combat_damage_when_static_applies() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let enabler = CardBuilder::new(CardId::new(), "Brontodon Enabler")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(4, 6))
        .build();
    let enabler_id = game.create_object_from_card(&enabler, alice, Zone::Battlefield);
    if let Some(object) = game.object_mut(enabler_id) {
        object.abilities_mut().push(Ability::static_ability(
                crate::static_abilities::StaticAbility::creatures_you_control_assign_combat_damage_using_toughness(),
            ));
    }

    let attacker_id = create_creature(&mut game, "Wall Fighter", alice, 0, 3);

    let mut combat = CombatState::default();
    combat.attackers.push(crate::combat_state::AttackerInfo {
        creature: attacker_id,
        target: AttackTarget::Player(bob),
    });
    combat.blockers.insert(attacker_id, Vec::new());

    let events = execute_combat_damage_step(&mut game, &combat, false);
    assert_eq!(events.len(), 1);
    assert_eq!(
        events[0].amount, 3,
        "attacker should assign combat damage equal to toughness"
    );
    assert_eq!(game.player(bob).unwrap().life, 17);
}

#[test]
pub(super) fn test_zilortha_strength_incarnate_power_sets_lethal_damage_for_your_creatures() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let zilortha = CardDefinitionBuilder::new(CardId::new(), "Zilortha, Strength Incarnate")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(7, 3))
        .parse_text(
            "Trample\n\
             Lethal damage dealt to creatures you control is determined by their power rather than their toughness.",
        )
        .expect("Zilortha, Strength Incarnate should parse for runtime test");
    let zilortha_id = game.create_object_from_definition(&zilortha, alice, Zone::Battlefield);
    assert!(game.current_has_static_ability_id(
        zilortha_id,
        crate::static_abilities::StaticAbilityId::LethalDamageToCreaturesYouControlUsesPower,
    ));

    let high_power_creature =
        create_creature(&mut game, "Alice's High-Power Creature", alice, 5, 2);
    game.mark_damage(high_power_creature, 4);
    crate::rules::state_based::apply_state_based_actions(&mut game);

    assert!(
        game.battlefield.contains(&high_power_creature),
        "Zilortha should let Alice's 5/2 survive 4 marked damage because lethal damage uses power"
    );

    game.mark_damage(high_power_creature, 1);
    assert_eq!(
        game.damage_on(high_power_creature),
        5,
        "marked damage should remain between SBA checks until cleanup"
    );
    assert!(
        crate::rules::state_based::check_state_based_actions(&game).contains(
            &crate::rules::state_based::StateBasedAction::ObjectDies(high_power_creature,)
        ),
        "Zilortha should make 5 damage lethal to Alice's 5-power creature"
    );
    crate::rules::state_based::apply_state_based_actions(&mut game);

    assert!(
        game.current_object_id_after_zone_change(high_power_creature)
            .and_then(|id| game.object(id))
            .is_some_and(|object| object.zone == Zone::Graveyard),
        "Zilortha should make Alice's 5/2 die once marked damage reaches its power"
    );
}

#[test]
pub(super) fn test_zilortha_strength_incarnate_only_changes_lethal_damage_for_controller_creatures()
{
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let zilortha = CardDefinitionBuilder::new(CardId::new(), "Zilortha, Strength Incarnate")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(7, 3))
        .parse_text(
            "Trample\n\
             Lethal damage dealt to creatures you control is determined by their power rather than their toughness.",
        )
        .expect("Zilortha, Strength Incarnate should parse for runtime test");
    let zilortha_id = game.create_object_from_definition(&zilortha, alice, Zone::Battlefield);
    assert!(game.current_has_static_ability_id(
        zilortha_id,
        crate::static_abilities::StaticAbilityId::LethalDamageToCreaturesYouControlUsesPower,
    ));

    let alice_low_power_creature =
        create_creature(&mut game, "Alice's Low-Power Creature", alice, 2, 5);
    let bob_low_power_creature = create_creature(&mut game, "Bob's Low-Power Creature", bob, 2, 5);

    game.mark_damage(alice_low_power_creature, 2);
    game.mark_damage(bob_low_power_creature, 2);
    assert!(
        crate::rules::state_based::check_state_based_actions(&game).contains(
            &crate::rules::state_based::StateBasedAction::ObjectDies(alice_low_power_creature),
        ),
        "Zilortha should make 2 damage lethal to Alice's 2-power creature"
    );
    crate::rules::state_based::apply_state_based_actions(&mut game);

    assert!(
        game.current_object_id_after_zone_change(alice_low_power_creature)
            .and_then(|id| game.object(id))
            .is_some_and(|object| object.zone == Zone::Graveyard),
        "Zilortha should make Alice's 2/5 die from 2 marked damage because lethal damage uses power"
    );
    assert!(
        game.battlefield.contains(&bob_low_power_creature),
        "Zilortha should not change lethal damage for Bob's creatures"
    );
}

#[test]
pub(super) fn test_zilortha_strength_incarnate_lethal_damage_interacts_with_deathtouch_and_trample()
{
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let zilortha = CardDefinitionBuilder::new(CardId::new(), "Zilortha, Strength Incarnate")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(7, 3))
        .parse_text(
            "Trample\n\
             Lethal damage dealt to creatures you control is determined by their power rather than their toughness.",
        )
        .expect("Zilortha, Strength Incarnate should parse for runtime test");
    game.create_object_from_definition(&zilortha, alice, Zone::Battlefield);

    let deathtouch_victim = create_creature(&mut game, "Alice's Zero-Power Creature", alice, 0, 5);
    game.mark_damage(deathtouch_victim, 1);
    game.mark_deathtouch_damage_since_sba(deathtouch_victim);
    crate::rules::state_based::apply_state_based_actions(&mut game);

    assert!(
        game.current_object_id_after_zone_change(deathtouch_victim)
            .and_then(|id| game.object(id))
            .is_some_and(|object| object.zone == Zone::Graveyard),
        "deathtouch damage should still destroy Alice's creature even when Zilortha makes its power 0 the lethal threshold"
    );

    let zero_power_victim = create_creature(
        &mut game,
        "Alice's Damaged Zero-Power Creature",
        alice,
        0,
        5,
    );
    game.mark_damage(zero_power_victim, 1);
    crate::rules::state_based::apply_state_based_actions(&mut game);

    assert!(
        game.current_object_id_after_zone_change(zero_power_victim)
            .and_then(|id| game.object(id))
            .is_some_and(|object| object.zone == Zone::Graveyard),
        "Zilortha should destroy Alice's 0-power creature once it has at least 1 damage marked"
    );

    let trampler = CardDefinitionBuilder::new(CardId::new(), "Bob's Trampler")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(5, 5))
        .parse_text("Trample")
        .expect("trample attacker should parse");
    let attacker_id = game.create_object_from_definition(&trampler, bob, Zone::Battlefield);
    let blocker_id = create_creature(&mut game, "Alice's High-Power Blocker", alice, 5, 2);

    let mut combat = CombatState::default();
    combat.attackers.push(crate::combat_state::AttackerInfo {
        creature: attacker_id,
        target: AttackTarget::Player(alice),
    });
    combat.blockers.insert(attacker_id, vec![blocker_id]);

    let events = execute_combat_damage_step(&mut game, &combat, false);
    assert_eq!(
        game.player(alice).unwrap().life,
        20,
        "Zilortha should make the trampler assign all 5 damage to Alice's 5-power blocker"
    );
    assert!(
        events.iter().any(
            |event| event.target == DamageEventTarget::Object(blocker_id) && event.amount == 5
        ),
        "combat damage should assign lethal damage by blocker power under Zilortha, got {events:?}"
    );

    let second_trampler = CardDefinitionBuilder::new(CardId::new(), "Bob's Second Trampler")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(5, 5))
        .parse_text("Trample")
        .expect("second trample attacker should parse");
    let second_attacker_id =
        game.create_object_from_definition(&second_trampler, bob, Zone::Battlefield);
    let zero_power_blocker = create_creature(&mut game, "Alice's Zero-Power Blocker", alice, 0, 5);

    let mut second_combat = CombatState::default();
    second_combat
        .attackers
        .push(crate::combat_state::AttackerInfo {
            creature: second_attacker_id,
            target: AttackTarget::Player(alice),
        });
    second_combat
        .blockers
        .insert(second_attacker_id, vec![zero_power_blocker]);

    let second_events = execute_combat_damage_step(&mut game, &second_combat, false);
    assert_eq!(
        game.player(alice).unwrap().life,
        16,
        "Zilortha should require 1 assigned damage before trampling over a 0-power blocker"
    );
    assert!(
        second_events.iter().any(|event| event.target
            == DamageEventTarget::Object(zero_power_blocker)
            && event.amount == 1),
        "combat damage should assign 1 lethal damage to a 0-power blocker under Zilortha, got {second_events:?}"
    );
}

#[test]
pub(super) fn test_blocked_attacker_deals_damage_to_blocker() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let attacker_id = create_creature(&mut game, "Attacker", alice, 3, 3);
    let blocker_id = create_creature(&mut game, "Blocker", bob, 2, 2);

    // Set up combat
    let mut combat = CombatState::default();
    combat.attackers.push(crate::combat_state::AttackerInfo {
        creature: attacker_id,
        target: AttackTarget::Player(bob),
    });
    combat.blockers.insert(attacker_id, vec![blocker_id]);

    // Execute combat damage
    let events = execute_combat_damage_step(&mut game, &combat, false);

    // Should have events for attacker->blocker and blocker->attacker
    assert!(events.len() >= 2);

    // With a single blocker and no alternative recipient, all 3 combat damage
    // is assigned to that blocker. Assignment is not capped at lethal damage.
    assert_eq!(game.damage_on(blocker_id), 3);

    // Attacker should have 2 damage
    assert_eq!(game.damage_on(attacker_id), 2);

    // Bob should not have taken damage (attacker was blocked)
    assert_eq!(game.player(bob).unwrap().life, 20);
}

#[test]
pub(super) fn test_first_strike_damage() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let attacker_id = create_creature(&mut game, "First Striker", alice, 2, 2);

    // Add first strike
    if let Some(obj) = game.object_mut(attacker_id) {
        obj.abilities_mut().push(Ability::static_ability(
            crate::static_abilities::StaticAbility::first_strike(),
        ));
    }

    // Set up combat
    let mut combat = CombatState::default();
    combat.attackers.push(crate::combat_state::AttackerInfo {
        creature: attacker_id,
        target: AttackTarget::Player(bob),
    });
    combat.blockers.insert(attacker_id, Vec::new());

    // First strike damage step - should deal damage
    let events = execute_combat_damage_step(&mut game, &combat, true);
    assert_eq!(events.len(), 1);
    assert_eq!(game.player(bob).unwrap().life, 18);

    // Regular damage step - first strike creature shouldn't deal damage again
    let events = execute_combat_damage_step(&mut game, &combat, false);
    assert_eq!(events.len(), 0);
    assert_eq!(game.player(bob).unwrap().life, 18);
}

#[test]
pub(super) fn test_lifelink_damage() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let attacker_id = create_creature(&mut game, "Lifelinker", alice, 3, 3);

    // Add lifelink
    if let Some(obj) = game.object_mut(attacker_id) {
        obj.abilities_mut().push(Ability::static_ability(
            crate::static_abilities::StaticAbility::lifelink(),
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
    let _events = execute_combat_damage_step(&mut game, &combat, false);

    // Bob took 3 damage
    assert_eq!(game.player(bob).unwrap().life, 17);

    // Alice gained 3 life
    assert_eq!(game.player(alice).unwrap().life, 23);
}

#[test]
pub(super) fn test_trample_damage() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let attacker_id = create_creature(&mut game, "Trampler", alice, 5, 5);
    let blocker_id = create_creature(&mut game, "Small Blocker", bob, 2, 2);

    // Add trample
    if let Some(obj) = game.object_mut(attacker_id) {
        obj.abilities_mut().push(Ability::static_ability(
            crate::static_abilities::StaticAbility::trample(),
        ));
    }

    // Set up combat
    let mut combat = CombatState::default();
    combat.attackers.push(crate::combat_state::AttackerInfo {
        creature: attacker_id,
        target: AttackTarget::Player(bob),
    });
    combat.blockers.insert(attacker_id, vec![blocker_id]);

    // Execute combat damage
    let events = execute_combat_damage_step(&mut game, &combat, false);

    // Should have events: attacker->blocker, attacker->player (trample), blocker->attacker
    assert!(events.len() >= 3);

    // Blocker should have 2 damage (lethal)
    assert_eq!(game.damage_on(blocker_id), 2);

    // Attacker should have 2 damage (from blocker)
    assert_eq!(game.damage_on(attacker_id), 2);

    // Bob should have taken 3 trample damage (5 power - 2 toughness = 3 excess)
    assert_eq!(game.player(bob).unwrap().life, 17);
}

// === State-Based Actions Tests ===

#[test]
pub(super) fn test_sba_creature_dies() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let creature_id = create_creature(&mut game, "Doomed", alice, 2, 2);

    // Deal lethal damage
    game.mark_damage(creature_id, 2);

    let mut trigger_queue = TriggerQueue::new();
    check_and_apply_sbas(&mut game, &mut trigger_queue).unwrap();

    // Creature should be in graveyard
    assert_eq!(game.battlefield.len(), 0);
    assert_eq!(game.player(alice).unwrap().graveyard.len(), 1);
}

#[test]
pub(super) fn test_sba_deathtouch_damage_destroys_creature() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let attacker_id = create_creature(&mut game, "Deathtoucher", alice, 1, 1);
    if let Some(obj) = game.object_mut(attacker_id) {
        obj.abilities_mut()
            .push(Ability::static_ability(StaticAbility::deathtouch()));
    }
    let victim_id = create_creature(&mut game, "Victim", bob, 3, 3);
    game.refresh_continuous_state();

    let keywords = crate::rules::damage::source_damage_keywords(&game, attacker_id, None);
    let applied = crate::rules::damage::apply_processed_damage_assignment(
        &mut game,
        attacker_id,
        crate::events::DamageTarget::Object(victim_id),
        1,
        keywords,
        crate::events::cause::EventCause::effect(),
    );
    assert!(applied.applied, "damage should be applied to the victim");

    let mut trigger_queue = TriggerQueue::new();
    check_and_apply_sbas(&mut game, &mut trigger_queue).unwrap();

    assert!(
        !game.battlefield.contains(&victim_id),
        "a creature dealt damage by a source with deathtouch should be destroyed as an SBA"
    );
    assert_eq!(
        game.player(bob).unwrap().graveyard.len(),
        1,
        "the destroyed creature should be put into its owner's graveyard"
    );
}

#[test]
pub(super) fn test_deathtouch_sba_marker_clears_after_each_check() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let attacker_id = create_creature(&mut game, "Deathtoucher", alice, 1, 1);
    if let Some(obj) = game.object_mut(attacker_id) {
        obj.abilities_mut()
            .push(Ability::static_ability(StaticAbility::deathtouch()));
    }
    let victim_id = create_creature(&mut game, "Stubborn Victim", bob, 3, 3);
    if let Some(obj) = game.object_mut(victim_id) {
        obj.abilities_mut()
            .push(Ability::static_ability(StaticAbility::indestructible()));
    }
    game.refresh_continuous_state();

    let keywords = crate::rules::damage::source_damage_keywords(&game, attacker_id, None);
    let applied = crate::rules::damage::apply_processed_damage_assignment(
        &mut game,
        attacker_id,
        crate::events::DamageTarget::Object(victim_id),
        1,
        keywords,
        crate::events::cause::EventCause::effect(),
    );
    assert!(applied.applied, "damage should be applied to the victim");

    let mut trigger_queue = TriggerQueue::new();
    check_and_apply_sbas(&mut game, &mut trigger_queue).unwrap();

    assert!(
        game.battlefield.contains(&victim_id),
        "indestructible should prevent the first deathtouch-based destruction"
    );

    if let Some(obj) = game.object_mut(victim_id) {
        obj.abilities_mut().retain(|ability| {
            !matches!(
                &ability.kind,
                AbilityKind::Static(static_ability)
                    if static_ability.id()
                        == crate::static_abilities::StaticAbilityId::Indestructible
            )
        });
    }
    game.refresh_continuous_state();

    check_and_apply_sbas(&mut game, &mut trigger_queue).unwrap();

    assert!(
        game.battlefield.contains(&victim_id),
        "old deathtouch damage should not keep destroying the creature after the next SBA check"
    );
}

#[test]
pub(super) fn test_sba_player_loses() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    // Set life to 0
    game.player_mut(alice).unwrap().life = 0;

    let mut trigger_queue = TriggerQueue::new();
    check_and_apply_sbas(&mut game, &mut trigger_queue).unwrap();

    // Alice should have lost
    assert!(game.player(alice).unwrap().has_lost);
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_state_trigger_sacrifice_fires_from_sba_scan() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let def = CardDefinitionBuilder::new(CardId::new(), "State Trigger Crocodile")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(5, 5))
        .parse_text("When you control no Swamps, sacrifice this creature.")
        .expect("state trigger card should parse");
    let creature_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

    let mut trigger_queue = TriggerQueue::new();
    check_and_apply_sbas(&mut game, &mut trigger_queue).unwrap();
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "expected state trigger to be queued when condition is true"
    );

    put_triggers_on_stack(&mut game, &mut trigger_queue).unwrap();
    resolve_stack_entry(&mut game).expect("state trigger should resolve");

    assert!(
        !game.battlefield.contains(&creature_id),
        "creature should be sacrificed once the trigger resolves"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_state_trigger_only_retriggers_after_condition_turns_false() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let def = CardDefinitionBuilder::new(CardId::new(), "State Trigger Study")
        .card_types(vec![CardType::Enchantment])
        .parse_text("When you control no Swamps, draw a card.")
        .expect("state trigger card should parse");
    game.create_object_from_definition(&def, alice, Zone::Battlefield);

    let mut trigger_queue = TriggerQueue::new();
    check_and_apply_sbas(&mut game, &mut trigger_queue).unwrap();
    assert_eq!(trigger_queue.entries.len(), 1, "condition starts true");

    trigger_queue.clear();
    check_and_apply_sbas(&mut game, &mut trigger_queue).unwrap();
    assert!(
        trigger_queue.is_empty(),
        "state trigger should not requeue while the condition remains true"
    );

    let swamp_card = CardBuilder::new(CardId::new(), "Test Swamp")
        .card_types(vec![CardType::Land])
        .subtypes(vec![crate::types::Subtype::Swamp])
        .build();
    let swamp_id = game.create_object_from_card(&swamp_card, alice, Zone::Battlefield);

    check_and_apply_sbas(&mut game, &mut trigger_queue).unwrap();
    assert!(
        trigger_queue.is_empty(),
        "controlling a Swamp should clear the active state trigger"
    );

    game.move_object(
        swamp_id,
        Zone::Graveyard,
        crate::events::cause::EventCause::effect(),
    )
    .expect("moving the swamp away should succeed");
    drain_pending_trigger_events(&mut game, &mut trigger_queue);

    check_and_apply_sbas(&mut game, &mut trigger_queue).unwrap();
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "state trigger should fire again after the condition becomes false, then true again"
    );
}

// === Priority Loop Tests ===

#[test]
pub(super) fn test_priority_loop_empty_stack() {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();

    // With empty stack and all passing, phase should end
    let mut dm = crate::decision::AutoPassDecisionMaker;
    let result = run_priority_loop_with(&mut game, &mut trigger_queue, &mut dm).unwrap();
    assert!(matches!(result, GameProgress::Continue));
}

pub(super) struct CorpseCobbleDecisionMaker;

impl DecisionMaker for CorpseCobbleDecisionMaker {
    fn decide_priority(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::PriorityContext,
    ) -> crate::decision::LegalAction {
        if let Some(action) = ctx.actions.iter().find(|action| {
            matches!(
                action,
                crate::decision::LegalAction::CastSpell { spell_id, .. }
                    if game
                        .object(*spell_id)
                        .is_some_and(|obj| obj.name == "Corpse Cobble")
            )
        }) {
            return action.clone();
        }

        if let Some(action) = ctx.actions.iter().find(|action| {
            matches!(
                action,
                crate::decision::LegalAction::ActivateManaAbility { source, .. }
                    if game.object(*source).is_some_and(|obj| {
                        matches!(obj.name.as_str(), "Island" | "Swamp")
                    })
            )
        }) {
            return action.clone();
        }

        crate::decision::LegalAction::PassPriority
    }

    fn decide_objects(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        let mut selected = Vec::new();
        for name in ["Grizzly Bears", "Llanowar Elves"] {
            if let Some(candidate) = ctx.candidates.iter().find(|candidate| {
                candidate.legal
                    && game
                        .object(candidate.id)
                        .is_some_and(|obj| obj.name == name)
            }) {
                selected.push(candidate.id);
            }
        }

        if selected.is_empty() {
            ctx.candidates
                .iter()
                .filter(|candidate| candidate.legal)
                .map(|candidate| candidate.id)
                .take(ctx.min)
                .collect()
        } else {
            selected
        }
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_corpse_cobble_sums_the_power_of_sacrificed_creatures() {
    use crate::cards::definitions::{basic_island, basic_swamp, grizzly_bears, llanowar_elves};
    use crate::game_state::Phase;
    use crate::zone::Zone;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let mut trigger_queue = TriggerQueue::new();
    let corpse_cobble_text = "As an additional cost to cast this spell, sacrifice any number of creatures.\nCreate an X/X blue and black Zombie creature token with menace, where X is the total power of the sacrificed creatures.\nFlashback {3}{U}{B} (You may cast this card from your graveyard for its flashback cost and any additional costs. Then exile it.)";

    game.turn.active_player = alice;
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);

    let corpse_cobble = CardDefinitionBuilder::new(CardId::from_raw(10001), "Corpse Cobble")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Blue],
            vec![ManaSymbol::Black],
        ]))
        .card_types(vec![CardType::Instant])
        .parse_text(corpse_cobble_text)
        .expect("Corpse Cobble text should parse");

    game.create_object_from_definition(&corpse_cobble, alice, Zone::Hand);
    game.create_object_from_definition(&basic_island(), alice, Zone::Battlefield);
    game.create_object_from_definition(&basic_swamp(), alice, Zone::Battlefield);
    game.create_object_from_definition(&grizzly_bears(), alice, Zone::Battlefield);
    game.create_object_from_definition(&llanowar_elves(), alice, Zone::Battlefield);

    let mut dm = CorpseCobbleDecisionMaker;
    let result = run_priority_loop_with(&mut game, &mut trigger_queue, &mut dm)
        .expect("Corpse Cobble cast should resolve cleanly");
    assert!(
        matches!(result, GameProgress::Continue),
        "priority loop should finish after Corpse Cobble resolves, got {result:?}"
    );

    assert!(
        !game.battlefield_has("Grizzly Bears"),
        "Grizzly Bears should have been sacrificed"
    );
    assert!(
        !game.battlefield_has("Llanowar Elves"),
        "Llanowar Elves should have been sacrificed"
    );

    let alice_graveyard = game.player(alice).expect("alice exists").graveyard.clone();
    assert!(
        alice_graveyard.iter().any(|id| {
            game.object(*id)
                .is_some_and(|obj| obj.name == "Grizzly Bears")
        }),
        "Grizzly Bears should end up in the graveyard"
    );
    assert!(
        alice_graveyard.iter().any(|id| {
            game.object(*id)
                .is_some_and(|obj| obj.name == "Llanowar Elves")
        }),
        "Llanowar Elves should end up in the graveyard"
    );

    let zombie = game
        .battlefield
        .iter()
        .filter_map(|id| game.object(*id))
        .find(|obj| obj.name == "Zombie")
        .expect("Corpse Cobble should create a Zombie token");
    assert_eq!(
        zombie.kind,
        ObjectKind::Token,
        "Corpse Cobble should create a token"
    );
    assert_eq!(
        zombie.base_power,
        Some(crate::card::PtValue::Fixed(3)),
        "Zombie token should use the total power of the sacrificed creatures"
    );
    assert_eq!(
        zombie.base_toughness,
        Some(crate::card::PtValue::Fixed(3)),
        "Zombie token should use the total power of the sacrificed creatures"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn tormented_thoughts_definition_for_runtime_tests() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(398_623), "Tormented Thoughts")
        .mana_cost(ManaCost::new())
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "As an additional cost to cast this spell, sacrifice a creature.\nTarget player discards a number of cards equal to the sacrificed creature's power.",
        )
        .expect("Tormented Thoughts should parse for runtime tests")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn tormented_thoughts_test_card(
    name: &str,
    card_types: Vec<CardType>,
) -> crate::card::Card {
    CardBuilder::new(CardId::new(), name)
        .card_types(card_types)
        .build()
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) struct TormentedThoughtsDiscardDecisionMaker {
    pub(super) discard_order: Vec<ObjectId>,
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) struct TormentedThoughtsCostDecisionMaker {
    pub(super) sacrifice_id: ObjectId,
}

#[cfg(ironsmith_runtime_parser_tests)]
impl DecisionMaker for TormentedThoughtsCostDecisionMaker {
    fn decide_objects(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        if ctx
            .candidates
            .iter()
            .any(|candidate| candidate.id == self.sacrifice_id && candidate.legal)
        {
            vec![self.sacrifice_id]
        } else {
            ctx.candidates
                .iter()
                .filter(|candidate| candidate.legal)
                .map(|candidate| candidate.id)
                .take(ctx.min)
                .collect()
        }
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
impl DecisionMaker for TormentedThoughtsDiscardDecisionMaker {
    fn decide_objects(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        let legal_ids = ctx
            .candidates
            .iter()
            .filter(|candidate| candidate.legal)
            .map(|candidate| candidate.id)
            .collect::<Vec<_>>();
        self.discard_order
            .iter()
            .copied()
            .filter(|id| legal_ids.contains(id))
            .take(ctx.min)
            .collect()
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn cast_tormented_thoughts_targeting_player(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    spell_id: ObjectId,
    target_player: PlayerId,
    sacrifice_id: ObjectId,
) {
    use crate::decision::{GameProgress, LegalAction};

    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = TormentedThoughtsCostDecisionMaker { sacrifice_id };
    let mut progress = apply_priority_response_with_dm(
        game,
        trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(LegalAction::CastSpell {
            spell_id,
            from_zone: Zone::Hand,
            casting_method: CastingMethod::Normal,
        }),
        &mut dm,
    )
    .expect("Tormented Thoughts cast should start");

    for _ in 0..5 {
        progress = match progress {
            GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::Targets(_),
            ) => apply_priority_response_with_dm(
                game,
                trigger_queue,
                &mut state,
                &PriorityResponse::Targets(vec![Target::Player(target_player)]),
                &mut dm,
            )
            .expect("Tormented Thoughts should accept target player"),
            GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::SelectOptions(ctx),
            ) => {
                let sacrifice_cost_index = ctx
                    .options
                    .iter()
                    .find(|option| {
                        option
                            .description
                            .to_ascii_lowercase()
                            .contains("sacrifice")
                    })
                    .map(|option| option.index)
                    .unwrap_or_else(|| {
                        panic!(
                            "Tormented Thoughts should offer a sacrifice cost option, got {:?}",
                            ctx.options
                        )
                    });
                apply_priority_response_with_dm(
                    game,
                    trigger_queue,
                    &mut state,
                    &PriorityResponse::NextCostChoice(sacrifice_cost_index),
                    &mut dm,
                )
                .expect("Tormented Thoughts should accept choosing the sacrifice cost")
            }
            GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::SelectObjects(ctx),
            ) => {
                assert!(
                    ctx.candidates
                        .iter()
                        .any(|candidate| candidate.id == sacrifice_id && candidate.legal),
                    "Tormented Thoughts should allow sacrificing the chosen creature as its additional cost"
                );
                apply_priority_response_with_dm(
                    game,
                    trigger_queue,
                    &mut state,
                    &PriorityResponse::CardCostChoice(sacrifice_id),
                    &mut dm,
                )
                .expect("Tormented Thoughts should accept the sacrificed creature")
            }
            GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::Priority(_),
            ) => {
                break;
            }
            other => panic!("unexpected Tormented Thoughts cast flow state: {other:?}"),
        };
    }

    assert_eq!(
        game.stack.len(),
        1,
        "Tormented Thoughts should be on the stack"
    );
    let stack_entry = game
        .stack
        .last()
        .expect("Tormented Thoughts should be stacked");
    assert_eq!(stack_entry.targets, vec![Target::Player(target_player)]);
    let sacrificed = stack_entry
        .tagged_objects
        .get(&crate::tag::TagKey::from("sacrificed_0"))
        .expect("Tormented Thoughts stack entry should remember the sacrificed creature");
    assert_eq!(sacrificed.len(), 1);
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn run_tormented_thoughts_discards_by_sacrificed_power(
    sacrificed_power: i32,
    plus_one_plus_one_counters: u32,
    target_hand_size: usize,
) {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut trigger_queue = TriggerQueue::new();

    game.turn.active_player = alice;
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);

    let tormented_thoughts = tormented_thoughts_definition_for_runtime_tests();
    let spell_id = game.create_object_from_definition(&tormented_thoughts, alice, Zone::Hand);
    let fodder = CardBuilder::new(CardId::new(), "Tormented Thoughts Fodder")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(sacrificed_power, 1))
        .build();
    let fodder_id = game.create_object_from_card(&fodder, alice, Zone::Battlefield);
    if plus_one_plus_one_counters > 0 {
        game.add_counters(
            fodder_id,
            crate::object::CounterType::PlusOnePlusOne,
            plus_one_plus_one_counters,
        );
    }
    let alice_hand_card = game.create_object_from_card(
        &tormented_thoughts_test_card("Alice Untouched Hand Card", vec![CardType::Instant]),
        alice,
        Zone::Hand,
    );
    let bob_hand = (0..target_hand_size)
        .map(|idx| {
            game.create_object_from_card(
                &tormented_thoughts_test_card(
                    &format!("Bob Tormented Card {}", idx + 1),
                    vec![CardType::Artifact],
                ),
                bob,
                Zone::Hand,
            )
        })
        .collect::<Vec<_>>();

    cast_tormented_thoughts_targeting_player(
        &mut game,
        &mut trigger_queue,
        spell_id,
        bob,
        fodder_id,
    );

    assert!(
        !game.battlefield.contains(&fodder_id)
            && player_zone_contains_named(
                &game,
                alice,
                Zone::Graveyard,
                "Tormented Thoughts Fodder"
            ),
        "Tormented Thoughts should sacrifice the creature as an additional cost"
    );
    assert!(
        game.player(alice)
            .expect("Alice exists")
            .hand
            .contains(&alice_hand_card),
        "Tormented Thoughts should not discard cards from the caster when Bob is targeted"
    );

    let mut dm = TormentedThoughtsDiscardDecisionMaker {
        discard_order: bob_hand.clone(),
    };
    resolve_stack_entry_with(&mut game, &mut dm).expect("Tormented Thoughts should resolve");

    let battlefield_power = sacrificed_power + plus_one_plus_one_counters as i32;
    let expected_discards = target_hand_size.min(battlefield_power.max(0) as usize);
    for (idx, card_id) in bob_hand.iter().enumerate() {
        let name = format!("Bob Tormented Card {}", idx + 1);
        if idx < expected_discards {
            assert!(
                !game.player(bob).expect("Bob exists").hand.contains(card_id)
                    && player_zone_contains_named(&game, bob, Zone::Graveyard, &name),
                "Tormented Thoughts should discard Bob's card {name}"
            );
        } else {
            assert!(
                game.player(bob).expect("Bob exists").hand.contains(card_id),
                "Tormented Thoughts should leave Bob's extra card {name} in hand"
            );
        }
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn tormented_thoughts_discards_cards_equal_to_sacrificed_creature_power() {
    run_tormented_thoughts_discards_by_sacrificed_power(3, 0, 4);
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn tormented_thoughts_discards_only_available_cards_when_power_exceeds_hand_size() {
    run_tormented_thoughts_discards_by_sacrificed_power(4, 0, 2);
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn tormented_thoughts_uses_sacrificed_creature_lki_power_after_counters() {
    run_tormented_thoughts_discards_by_sacrificed_power(1, 3, 4);
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn soulblast_definition_for_runtime_tests() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(130_369), "Soulblast")
        .mana_cost(ManaCost::new())
        .card_types(vec![CardType::Instant])
        .parse_text(
            "As an additional cost to cast this spell, sacrifice all creatures you control.\nSoulblast deals damage to any target equal to the total power of the sacrificed creatures.",
        )
        .expect("Soulblast should parse for runtime tests")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn cast_soulblast_targeting_player(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    spell_id: ObjectId,
    target_player: PlayerId,
) {
    use crate::decision::{GameProgress, LegalAction};

    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut progress = apply_priority_response(
        game,
        trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(LegalAction::CastSpell {
            spell_id,
            from_zone: Zone::Hand,
            casting_method: CastingMethod::Normal,
        }),
    )
    .expect("Soulblast cast should start");

    for _ in 0..4 {
        progress = match progress {
            GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::Targets(_),
            ) => apply_priority_response(
                game,
                trigger_queue,
                &mut state,
                &PriorityResponse::Targets(vec![Target::Player(target_player)]),
            )
            .expect("Soulblast should accept target player"),
            GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::Priority(_),
            ) => break,
            other => panic!("unexpected Soulblast cast flow state: {other:?}"),
        };
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn soulblast_sacrifices_controlled_creatures_and_deals_their_total_power() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut trigger_queue = TriggerQueue::new();

    game.turn.active_player = alice;
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);

    let soulblast = soulblast_definition_for_runtime_tests();
    let spell_id = game.create_object_from_definition(&soulblast, alice, Zone::Hand);
    let small = CardBuilder::new(CardId::new(), "Soulblast Fodder One")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let large = CardBuilder::new(CardId::new(), "Soulblast Fodder Two")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 3))
        .build();
    let opposing = CardBuilder::new(CardId::new(), "Bob's Untouched Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(9, 9))
        .build();

    let small_id = game.create_object_from_card(&small, alice, Zone::Battlefield);
    let large_id = game.create_object_from_card(&large, alice, Zone::Battlefield);
    let opposing_id = game.create_object_from_card(&opposing, bob, Zone::Battlefield);

    cast_soulblast_targeting_player(&mut game, &mut trigger_queue, spell_id, bob);

    assert!(
        !game.battlefield.contains(&small_id) && !game.battlefield.contains(&large_id),
        "Soulblast should sacrifice all creatures controlled by its caster as a cost"
    );
    assert!(
        game.battlefield.contains(&opposing_id),
        "Soulblast should not sacrifice creatures controlled by another player"
    );

    resolve_stack_entry(&mut game).expect("Soulblast should resolve");
    assert_eq!(
        game.player(bob).expect("Bob exists").life,
        15,
        "Soulblast should deal damage equal to the total power of the sacrificed creatures"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn soulblast_with_no_controlled_creatures_deals_zero_damage() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut trigger_queue = TriggerQueue::new();

    game.turn.active_player = alice;
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);

    let soulblast = soulblast_definition_for_runtime_tests();
    let spell_id = game.create_object_from_definition(&soulblast, alice, Zone::Hand);
    let opposing = CardBuilder::new(CardId::new(), "Bob's Only Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(4, 4))
        .build();
    let opposing_id = game.create_object_from_card(&opposing, bob, Zone::Battlefield);

    cast_soulblast_targeting_player(&mut game, &mut trigger_queue, spell_id, bob);
    assert!(
        game.battlefield.contains(&opposing_id),
        "Soulblast should not sacrifice an opponent's creature when you control none"
    );

    resolve_stack_entry(&mut game)
        .expect("Soulblast with zero sacrificed creatures should resolve");
    assert_eq!(
        game.player(bob).expect("Bob exists").life,
        20,
        "Soulblast should deal zero damage when no creatures were sacrificed"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_spoils_of_blood_creates_token_using_creatures_died_this_turn_count() {
    use crate::decision::LegalAction;
    use crate::game_loop::{PriorityLoopState, apply_priority_response, resolve_stack_entry};
    use crate::game_state::Phase;
    use crate::mana::ManaSymbol;
    use crate::triggers::TriggerQueue;
    use crate::zone::Zone;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let mut trigger_queue = TriggerQueue::new();
    let spoils_text = "Create an X/X black Horror creature token, where X is the number of creatures that died this turn.";

    game.turn.active_player = alice;
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);

    let spoils = CardDefinitionBuilder::new(CardId::from_raw(10003), "Spoils of Blood")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Black]]))
        .card_types(vec![CardType::Instant])
        .parse_text(spoils_text)
        .expect("Spoils of Blood text should parse");

    let spoils_id = game.create_object_from_definition(&spoils, alice, Zone::Hand);
    let surviving_creature = CardBuilder::new(CardId::new(), "Surviving Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let doomed_creature_1 = CardBuilder::new(CardId::new(), "Doomed Creature 1")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let doomed_creature_2 = CardBuilder::new(CardId::new(), "Doomed Creature 2")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();

    let _surviving_id = game.create_object_from_card(&surviving_creature, alice, Zone::Battlefield);
    let doomed_id_1 = game.create_object_from_card(&doomed_creature_1, alice, Zone::Battlefield);
    let doomed_id_2 = game.create_object_from_card(&doomed_creature_2, alice, Zone::Battlefield);

    game.move_object_by_effect(doomed_id_1, Zone::Graveyard);
    game.move_object_by_effect(doomed_id_2, Zone::Graveyard);
    assert_eq!(
        game.turn_store
            .turn_history
            .total_creatures_died_this_turn(),
        2,
        "the setup should leave exactly two creatures dead this turn"
    );

    game.player_mut(alice)
        .expect("alice exists")
        .mana_pool
        .add(ManaSymbol::Black, 1);

    let mut state = PriorityLoopState::new(2);
    let cast_response = crate::PriorityResponse::PriorityAction(LegalAction::CastSpell {
        spell_id: spoils_id,
        from_zone: Zone::Hand,
        casting_method: crate::alternative_cast::CastingMethod::Normal,
    });
    apply_priority_response(&mut game, &mut trigger_queue, &mut state, &cast_response)
        .expect("Spoils of Blood cast should succeed");
    assert_eq!(
        game.stack.len(),
        1,
        "Spoils of Blood should be on the stack"
    );

    resolve_stack_entry(&mut game).expect("Spoils of Blood should resolve");

    let horror = game
        .battlefield
        .iter()
        .filter_map(|id| game.object(*id))
        .find(|obj| game.controller_of(obj) == alice && obj.name == "Horror")
        .expect("Spoils of Blood should create a Horror token");

    assert_eq!(
        horror.kind,
        ObjectKind::Token,
        "expected a token on resolution"
    );
    assert_eq!(horror.power(), Some(2), "Horror token should be 2/2");
    assert_eq!(horror.toughness(), Some(2), "Horror token should be 2/2");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn necrotic_fumes_cast_exiles_paid_creature_and_target_creature() {
    use crate::decision::{GameProgress, LegalAction};

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut trigger_queue = TriggerQueue::new();

    game.turn.active_player = alice;
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);

    let necrotic_fumes = CardDefinitionBuilder::new(CardId::from_raw(100_780), "Necrotic Fumes")
        .mana_cost(ManaCost::new())
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "As an additional cost to cast this spell, exile a creature you control.\nExile target creature or planeswalker.",
        )
        .expect("Necrotic Fumes should parse");

    let cost_creature = CardBuilder::new(CardId::new(), "Cost Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let target_creature = CardBuilder::new(CardId::new(), "Target Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 3))
        .build();

    let cost_creature_id = game.create_object_from_card(&cost_creature, alice, Zone::Battlefield);
    let target_creature_id = game.create_object_from_card(&target_creature, bob, Zone::Battlefield);
    let spell_id = game.create_object_from_definition(&necrotic_fumes, alice, Zone::Hand);

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
    .expect("Necrotic Fumes cast should start");

    for _ in 0..6 {
        progress = match progress {
            GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::Targets(_),
            ) => apply_priority_response(
                &mut game,
                &mut trigger_queue,
                &mut state,
                &PriorityResponse::Targets(vec![Target::Object(target_creature_id)]),
            )
            .expect("Necrotic Fumes should accept creature target"),
            GameProgress::NeedsDecisionCtx(
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
                .expect("Necrotic Fumes should accept additional cost choice")
            }
            GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::SelectObjects(ctx),
            ) => {
                assert!(
                    ctx.candidates
                        .iter()
                        .any(|candidate| candidate.id == cost_creature_id && candidate.legal),
                    "additional cost chooser should allow exiling the controller's creature"
                );
                apply_priority_response(
                    &mut game,
                    &mut trigger_queue,
                    &mut state,
                    &PriorityResponse::CardCostChoice(cost_creature_id),
                )
                .expect("Necrotic Fumes should accept exiling chosen cost creature")
            }
            GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::Priority(_),
            ) => {
                break;
            }
            other => panic!("unexpected cast flow state for Necrotic Fumes: {other:?}"),
        };
    }

    assert_eq!(
        game.stack.len(),
        1,
        "Necrotic Fumes should be on stack after costs"
    );
    let cost_creature_exiled = game.exile.iter().any(|&id| {
        game.object(id)
            .is_some_and(|obj| obj.name == "Cost Creature" && obj.owner == alice)
    });
    assert!(
        cost_creature_exiled,
        "the additional-cost creature should be exiled while casting"
    );

    resolve_stack_entry(&mut game).expect("Necrotic Fumes should resolve");
    let target_creature_exiled = game.exile.iter().any(|&id| {
        game.object(id)
            .is_some_and(|obj| obj.name == "Target Creature" && obj.owner == bob)
    });
    assert!(
        target_creature_exiled,
        "the targeted creature should be exiled on resolution"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn necrotic_fumes_cast_exiles_target_planeswalker() {
    use crate::decision::{GameProgress, LegalAction};

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    game.turn.active_player = alice;
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);

    let necrotic_fumes = CardDefinitionBuilder::new(CardId::from_raw(100_781), "Necrotic Fumes")
        .mana_cost(ManaCost::new())
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "As an additional cost to cast this spell, exile a creature you control.\nExile target creature or planeswalker.",
        )
        .expect("Necrotic Fumes should parse");

    let cost_creature = CardBuilder::new(CardId::new(), "Cost Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let target_planeswalker = CardBuilder::new(CardId::new(), "Target Planeswalker")
        .card_types(vec![CardType::Planeswalker])
        .loyalty(4)
        .build();

    let cost_creature_id = game.create_object_from_card(&cost_creature, alice, Zone::Battlefield);
    let target_planeswalker_id =
        game.create_object_from_card(&target_planeswalker, bob, Zone::Battlefield);
    let spell_id = game.create_object_from_definition(&necrotic_fumes, alice, Zone::Hand);

    let mut trigger_queue = TriggerQueue::new();
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
    .expect("Necrotic Fumes cast should start");

    for _ in 0..6 {
        progress = match progress {
            GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::Targets(_),
            ) => apply_priority_response(
                &mut game,
                &mut trigger_queue,
                &mut state,
                &PriorityResponse::Targets(vec![Target::Object(target_planeswalker_id)]),
            )
            .expect("Necrotic Fumes should accept planeswalker target"),
            GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::SelectObjects(_),
            ) => apply_priority_response(
                &mut game,
                &mut trigger_queue,
                &mut state,
                &PriorityResponse::CardCostChoice(cost_creature_id),
            )
            .expect("Necrotic Fumes should accept cost creature choice"),
            GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::Priority(_),
            ) => {
                break;
            }
            GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::SelectOptions(ctx),
            ) => {
                let option_index = ctx.options.first().map(|opt| opt.index).unwrap_or(0);
                apply_priority_response(
                    &mut game,
                    &mut trigger_queue,
                    &mut state,
                    &PriorityResponse::NextCostChoice(option_index),
                )
                .expect("Necrotic Fumes should accept cost-step option")
            }
            other => panic!("unexpected cast flow state for Necrotic Fumes: {other:?}"),
        };
    }

    resolve_stack_entry(&mut game).expect("Necrotic Fumes should resolve against planeswalker");
    let target_planeswalker_exiled = game.exile.iter().any(|&id| {
        game.object(id)
            .is_some_and(|obj| obj.name == "Target Planeswalker" && obj.owner == bob)
    });
    assert!(
        target_planeswalker_exiled,
        "the targeted planeswalker should be exiled on resolution"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn necrotic_fumes_cost_prompt_has_no_legal_creature_without_controller_creature() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    game.turn.active_player = alice;
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);

    let necrotic_fumes = CardDefinitionBuilder::new(CardId::from_raw(100_782), "Necrotic Fumes")
        .mana_cost(ManaCost::new())
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "As an additional cost to cast this spell, exile a creature you control.\nExile target creature or planeswalker.",
        )
        .expect("Necrotic Fumes should parse");

    let target_creature = CardBuilder::new(CardId::new(), "Target Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();

    let target_creature_id = game.create_object_from_card(&target_creature, bob, Zone::Battlefield);
    let spell_id = game.create_object_from_definition(&necrotic_fumes, alice, Zone::Hand);

    let mut trigger_queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let progress = apply_priority_response(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(crate::decision::LegalAction::CastSpell {
            spell_id,
            from_zone: Zone::Hand,
            casting_method: CastingMethod::Normal,
        }),
    )
    .expect("Necrotic Fumes cast should reach cost/target prompts");

    match progress {
        crate::decision::GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::Targets(_),
        ) => {
            let err = apply_priority_response(
                &mut game,
                &mut trigger_queue,
                &mut state,
                &PriorityResponse::Targets(vec![Target::Object(target_creature_id)]),
            )
            .expect_err("Necrotic Fumes target confirmation should fail without payable cost");
            let detail = format!("{err:?}").to_ascii_lowercase();
            assert!(
                detail.contains("failed to pay deferred spell cost")
                    || detail.contains("insufficient")
                    || detail.contains("additional cost"),
                "expected a cost-payment failure when no creature can be exiled, got {detail}"
            );
        }
        other => panic!("unexpected Necrotic Fumes cast flow without cost creature: {other:?}"),
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn corpse_lunge_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(100_783), "Corpse Lunge")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Black],
        ]))
        .card_types(vec![CardType::Instant])
        .parse_text(
            "As an additional cost to cast this spell, exile a creature card from your graveyard.\nCorpse Lunge deals damage equal to the exiled card's power to target creature.",
        )
        .expect("Corpse Lunge should parse")
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn corpse_lunge_cast_exiles_graveyard_creature_and_deals_its_power() {
    use crate::decision::{GameProgress, LegalAction};
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
    let target_creature = CardBuilder::new(CardId::new(), "Target Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(5, 5))
        .build();

    let cost_creature_id = game.create_object_from_card(&cost_creature, alice, Zone::Graveyard);
    let target_creature_id = game.create_object_from_card(&target_creature, bob, Zone::Battlefield);
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

    let mut reached_priority = false;
    for _ in 0..8 {
        progress = match progress {
            GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::Targets(_),
            ) => apply_priority_response(
                &mut game,
                &mut trigger_queue,
                &mut state,
                &PriorityResponse::Targets(vec![Target::Object(target_creature_id)]),
            )
            .expect("Corpse Lunge should accept creature target"),
            GameProgress::NeedsDecisionCtx(
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
                .expect("Corpse Lunge should accept exile additional cost choice")
            }
            GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::SelectObjects(ctx),
            ) => {
                assert!(
                    ctx.candidates
                        .iter()
                        .any(|candidate| candidate.id == cost_creature_id && candidate.legal),
                    "additional cost chooser should allow exiling the controller's graveyard creature card"
                );
                apply_priority_response(
                    &mut game,
                    &mut trigger_queue,
                    &mut state,
                    &PriorityResponse::CardCostChoice(cost_creature_id),
                )
                .expect("Corpse Lunge should accept exiling chosen graveyard creature")
            }
            GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::Priority(_),
            ) => {
                reached_priority = true;
                break;
            }
            other => panic!("unexpected cast flow state for Corpse Lunge: {other:?}"),
        };
    }
    assert!(
        reached_priority,
        "Corpse Lunge should finish casting after costs"
    );
    assert_eq!(game.stack.len(), 1, "Corpse Lunge should be on the stack");
    let stack_entry = game.stack.last().expect("Corpse Lunge should be stacked");
    let exiled_snapshots = stack_entry
        .tagged_objects
        .get(&crate::tag::TagKey::from(crate::tag::SOURCE_EXILED_TAG))
        .expect("Corpse Lunge stack entry should remember the exiled cost card");
    assert_eq!(exiled_snapshots.len(), 1);
    assert_eq!(exiled_snapshots[0].name, "Exiled Brute");
    assert!(
        game.exile.iter().any(|&id| game
            .object(id)
            .is_some_and(|obj| obj.name == "Exiled Brute" && obj.owner == alice)),
        "Corpse Lunge additional cost should exile the chosen graveyard creature"
    );

    resolve_stack_entry(&mut game).expect("Corpse Lunge should resolve");
    assert_eq!(
        game.damage_on(target_creature_id),
        4,
        "Corpse Lunge should deal damage equal to the exiled creature card's power"
    );
}
