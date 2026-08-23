#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

fn creature(name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build()
}

fn flying_creature(name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::flying(),
        ))
        .build()
}

#[test]
fn protective_bubble_and_whispersilk_cloak_apply_shroud_and_unblockable_only_to_the_host() {
    for name in ["Protective Bubble", "Whispersilk Cloak"] {
        let definition = parse_oracle_card_definition(name);
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let host = game.create_object_from_definition(
            &creature(&format!("{name} Host")),
            alice,
            Zone::Battlefield,
        );
        let bystander = game.create_object_from_definition(
            &creature(&format!("{name} Bystander")),
            alice,
            Zone::Battlefield,
        );
        let opponent_source = game.create_object_from_definition(
            &creature(&format!("{name} Opposing Source")),
            bob,
            Zone::Battlefield,
        );
        let attachment = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        assert!(game.attach_object_to_target(
            attachment,
            crate::object::AttachmentTarget::Object(host),
        ));
        game.refresh_continuous_state();

        assert!(game.object_has_static_ability_id(host, StaticAbilityId::Shroud));
        assert!(!game.can_be_blocked(host));
        assert_eq!(
            crate::targeting::can_target_object(&game, host, opponent_source, bob),
            crate::targeting::TargetingResult::Invalid(
                crate::targeting::TargetingInvalidReason::HasShroud,
            )
        );
        assert!(!game.object_has_static_ability_id(bystander, StaticAbilityId::Shroud));
        assert!(game.can_be_blocked(bystander));
        assert!(!game.object_has_static_ability_id(attachment, StaticAbilityId::Shroud));

        assert!(game.detach_object_from_current_target(attachment));
        game.refresh_continuous_state();
        assert!(!game.object_has_static_ability_id(host, StaticAbilityId::Shroud));
        assert!(game.can_be_blocked(host));
    }
}

#[test]
fn vorrac_battlehorns_grants_trample_and_the_single_blocker_limit_only_to_its_host() {
    let definition = parse_oracle_card_definition("Vorrac Battlehorns");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let host =
        game.create_object_from_definition(&creature("Battlehorns Host"), alice, Zone::Battlefield);
    let bystander = game.create_object_from_definition(
        &creature("Battlehorns Bystander"),
        alice,
        Zone::Battlefield,
    );
    let equipment = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    assert!(
        game.attach_object_to_target(equipment, crate::object::AttachmentTarget::Object(host),)
    );

    assert!(game.object_has_static_ability_id(host, StaticAbilityId::Trample));
    assert_eq!(
        crate::rules::combat::maximum_blockers(game.object(host).expect("host"), &game),
        Some(1)
    );
    assert!(!game.object_has_static_ability_id(bystander, StaticAbilityId::Trample));
    assert_eq!(
        crate::rules::combat::maximum_blockers(game.object(bystander).expect("bystander"), &game),
        None
    );
    assert!(!game.object_has_static_ability_id(equipment, StaticAbilityId::Trample));
}

fn etb_event(game: &crate::GameState, object: ObjectId) -> crate::triggers::TriggerEvent {
    let mut snapshot = crate::snapshot::ObjectSnapshot::from_object(
        game.object(object).expect("entering object should exist"),
        game,
    );
    snapshot.zone = Zone::Stack;
    crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::zones::ZoneChangeEvent::with_cause(
            object,
            Zone::Stack,
            Zone::Battlefield,
            crate::events::cause::EventCause::effect(),
            Some(snapshot),
        ),
        crate::provenance::ProvNodeId::default(),
    )
}

#[test]
fn stop_cold_taps_and_suppresses_only_the_enchanted_permanent_until_detached() {
    let definition = parse_oracle_card_definition("Stop Cold");
    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Stop Cold should retain its Aura ETB trigger");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let host = game.create_object_from_definition(
        &flying_creature("Stop Cold Host"),
        alice,
        Zone::Battlefield,
    );
    let bystander = game.create_object_from_definition(
        &flying_creature("Stop Cold Bystander"),
        alice,
        Zone::Battlefield,
    );
    let aura = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    assert!(game.attach_object_to_target(aura, crate::object::AttachmentTarget::Object(host),));
    let mut context = crate::effects::ExecutionContext::new_default(aura, alice);
    context.triggering_event = Some(etb_event(&game, aura));
    for effect in triggered.effects.flattened_default_effects() {
        crate::effects::execute_effect(&mut game, effect, &mut context)
            .expect("Stop Cold's ETB trigger should resolve");
    }
    game.refresh_continuous_state();

    assert!(game.is_tapped(host));
    assert!(!game.object_has_static_ability_id(host, StaticAbilityId::Flying));
    assert!(!game.can_untap(host));
    assert!(game.object_has_static_ability_id(bystander, StaticAbilityId::Flying));
    assert!(game.can_untap(bystander));
    assert!(game.object_has_static_ability_id(aura, StaticAbilityId::Flash));

    assert!(game.detach_object_from_current_target(aura));
    game.refresh_continuous_state();
    assert!(game.object_has_static_ability_id(host, StaticAbilityId::Flying));
    assert!(game.can_untap(host));
}

#[test]
fn frozen_in_ice_removes_abilities_and_prevents_untapping_only_while_attached() {
    let definition = parse_oracle_card_definition("Frozen in Ice");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let host = game.create_object_from_definition(
        &flying_creature("Frozen Host"),
        alice,
        Zone::Battlefield,
    );
    let bystander = game.create_object_from_definition(
        &flying_creature("Frozen Bystander"),
        alice,
        Zone::Battlefield,
    );
    let aura = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    assert!(game.attach_object_to_target(aura, crate::object::AttachmentTarget::Object(host),));
    game.refresh_continuous_state();

    assert!(!game.object_has_static_ability_id(host, StaticAbilityId::Flying));
    assert!(!game.can_untap(host));
    assert!(game.object_has_static_ability_id(bystander, StaticAbilityId::Flying));
    assert!(game.can_untap(bystander));
    assert!(game.can_untap(aura));

    assert!(game.detach_object_from_current_target(aura));
    game.refresh_continuous_state();
    assert!(game.object_has_static_ability_id(host, StaticAbilityId::Flying));
    assert!(game.can_untap(host));
}

fn register_nafs_obligation() -> (crate::GameState, PlayerId, ObjectId) {
    let definition = parse_oracle_card_definition("Nafs Asp");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let asp = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::DamageEvent::with_cause(
            asp,
            crate::events::DamageTarget::Player(bob),
            1,
            false,
            crate::events::cause::EventCause::from_effect(asp, alice),
        ),
        crate::provenance::ProvNodeId::default(),
    );
    let matching = crate::triggers::check_triggers(&game, &event)
        .into_iter()
        .filter(|entry| entry.source == asp)
        .collect::<Vec<_>>();
    assert_eq!(matching.len(), 1, "Nafs Asp damage should trigger once");
    let mut queue = crate::triggers::TriggerQueue::new();
    for entry in matching {
        queue.add(entry);
    }
    crate::game_loop::put_triggers_on_stack(&mut game, &mut queue)
        .expect("Nafs Asp's trigger should go on the stack");
    crate::game_loop::resolve_stack_entry(&mut game)
        .expect("Nafs Asp's trigger should register its delayed obligation");
    assert_eq!(game.effect_store.delayed_triggers.len(), 1);
    assert_eq!(
        game.effect_store.delayed_triggers[0]
            .prepayment
            .as_ref()
            .map(|payment| payment.player),
        Some(bob)
    );
    (game, bob, asp)
}

fn beginning_of_draw_step(player: PlayerId) -> crate::triggers::TriggerEvent {
    crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::phase::BeginningOfDrawStepEvent::new(player),
        crate::provenance::ProvNodeId::default(),
    )
}

#[test]
fn nafs_asp_exposes_a_real_advance_payment_window_and_penalizes_only_if_unpaid() {
    let (mut unpaid, bob, asp) = register_nafs_obligation();
    unpaid
        .move_object_by_effect(asp, Zone::Graveyard)
        .expect("the source may leave before the draw step");
    unpaid.turn.turn_number += 1;
    unpaid.turn.active_player = bob;
    let entries =
        crate::triggers::check_delayed_triggers(&mut unpaid, &beginning_of_draw_step(bob));
    assert_eq!(entries.len(), 1);
    let mut queue = crate::triggers::TriggerQueue::new();
    for entry in entries {
        queue.add(entry);
    }
    crate::game_loop::put_triggers_on_stack(&mut unpaid, &mut queue)
        .expect("the unpaid delayed trigger should go on the stack");
    crate::game_loop::resolve_stack_entry(&mut unpaid)
        .expect("the unpaid delayed trigger should resolve");
    assert_eq!(unpaid.life_total(bob), 19);

    let (mut paid, bob, asp) = register_nafs_obligation();
    paid.move_object_by_effect(asp, Zone::Graveyard)
        .expect("the payment window must survive source departure");
    paid.turn.priority_player = Some(bob);
    paid.player_mut(bob)
        .expect("Bob should exist")
        .mana_pool
        .add(ManaSymbol::Colorless, 1);
    let action = crate::special_actions::SpecialAction::PayDelayedTrigger {
        delayed_trigger_index: 0,
    };
    assert!(crate::special_actions::can_perform_check(&action, &paid, bob).is_ok());
    crate::special_actions::perform(
        action,
        &mut paid,
        bob,
        &mut crate::decision::SelectFirstDecisionMaker,
    )
    .expect("Bob should be able to pay before the draw step");
    assert!(paid.effect_store.delayed_triggers.is_empty());
    paid.turn.turn_number += 1;
    paid.turn.active_player = bob;
    assert!(
        crate::triggers::check_delayed_triggers(&mut paid, &beginning_of_draw_step(bob)).is_empty()
    );
    assert_eq!(paid.life_total(bob), 20);
}
