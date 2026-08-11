#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

fn creature(name: &str, controller_subtypes: Vec<Subtype>) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .subtypes(controller_subtypes)
        .power_toughness(PowerToughness::fixed(2, 2))
        .build()
}

fn activated_ability_containing<'a>(
    definition: &'a CardDefinition,
    needle: &str,
) -> &'a crate::ability::ActivatedAbility {
    definition
        .abilities
        .iter()
        .find_map(|ability| {
            let AbilityKind::Activated(activated) = &ability.kind else {
                return None;
            };
            format!("{:#?}", activated.effects)
                .contains(needle)
                .then_some(activated)
        })
        .unwrap_or_else(|| panic!("{needle} activated ability should exist: {definition:#?}"))
}

fn resolve_targeted_activation(
    game: &mut crate::game_state::GameState,
    source: ObjectId,
    controller: PlayerId,
    target: ObjectId,
    activated: &crate::ability::ActivatedAbility,
) {
    let target_spec = activated
        .choices
        .first()
        .expect("the activation should declare one object target")
        .clone();
    let mut ctx = crate::effects::ExecutionContext::new_default(source, controller)
        .with_targets(vec![crate::effects::ResolvedTarget::Object(target)])
        .with_target_assignments(vec![crate::game_state::TargetAssignment {
            spec: target_spec,
            range: 0..1,
        }]);
    ctx.snapshot_targets(game);
    for effect in activated.effects.flattened_default_effects() {
        crate::effects::execute_effect(game, effect, &mut ctx)
            .expect("the targeted activation should resolve");
    }
}

#[test]
fn alpha_authority_grants_both_restrictions_only_to_its_enchanted_creature() {
    let definition = parse_oracle_card_definition("Alpha Authority");
    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        "Enchant creature\nEnchanted creature has hexproof and can't be blocked by more than one creature."
    );

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let host = game.create_object_from_definition(
        &creature("Authority Host", Vec::new()),
        alice,
        Zone::Battlefield,
    );
    let bystander = game.create_object_from_definition(
        &creature("Authority Bystander", Vec::new()),
        alice,
        Zone::Battlefield,
    );
    let opposing_source = game.create_object_from_definition(
        &creature("Opposing Source", Vec::new()),
        bob,
        Zone::Battlefield,
    );
    let authority = game.create_object_from_definition(&definition, alice, Zone::Battlefield);

    assert!(
        game.attach_object_to_target(authority, crate::object::AttachmentTarget::Object(host),)
    );
    assert!(game.object_has_static_ability_id(host, StaticAbilityId::Hexproof));
    assert_eq!(
        crate::rules::combat::maximum_blockers(game.object(host).unwrap(), &game),
        Some(1)
    );
    assert_eq!(
        crate::targeting::can_target_object(&game, host, opposing_source, bob),
        crate::targeting::TargetingResult::Invalid(
            crate::targeting::TargetingInvalidReason::HasHexproof,
        )
    );
    assert!(!game.object_has_static_ability_id(bystander, StaticAbilityId::Hexproof));
    assert_eq!(
        crate::rules::combat::maximum_blockers(game.object(bystander).unwrap(), &game),
        None
    );
    assert!(!game.object_has_static_ability_id(authority, StaticAbilityId::Hexproof));

    assert!(game.detach_object_from_current_target(authority));
    assert!(!game.object_has_static_ability_id(host, StaticAbilityId::Hexproof));
    assert_eq!(
        crate::rules::combat::maximum_blockers(game.object(host).unwrap(), &game),
        None
    );
}

fn etb_event(
    game: &crate::game_state::GameState,
    object: ObjectId,
) -> crate::triggers::TriggerEvent {
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
fn cloak_and_dagger_matches_any_rogue_and_attaches_only_when_the_may_is_accepted() {
    let definition = parse_oracle_card_definition("Cloak and Dagger");
    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        "Equipped creature gets +2/+0 and has shroud.\nWhenever a Rogue creature enters, you may attach this Equipment to it.\nEquip {3}"
    );
    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Cloak and Dagger should retain its Rogue ETB trigger");

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let cloak = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let original_host = game.create_object_from_definition(
        &creature("Original Host", Vec::new()),
        alice,
        Zone::Battlefield,
    );
    let rogue = game.create_object_from_definition(
        &creature("Opponent Rogue", vec![Subtype::Rogue]),
        bob,
        Zone::Battlefield,
    );
    let nonrogue = game.create_object_from_definition(
        &creature("Opponent Soldier", vec![Subtype::Soldier]),
        bob,
        Zone::Battlefield,
    );
    assert!(game.attach_object_to_target(
        cloak,
        crate::object::AttachmentTarget::Object(original_host),
    ));

    let rogue_event = etb_event(&game, rogue);
    let nonrogue_event = etb_event(&game, nonrogue);
    let trigger_ctx = crate::triggers::TriggerContext::for_source(cloak, alice, &game);
    assert!(triggered.trigger.matches(&rogue_event, &trigger_ctx));
    assert!(!triggered.trigger.matches(&nonrogue_event, &trigger_ctx));

    let mut accept = crate::decision::SelectFirstDecisionMaker;
    let mut resolution = crate::effects::ExecutionContext::new_default(cloak, alice)
        .with_decision_maker(&mut accept);
    resolution.triggering_event = Some(rogue_event);
    for effect in triggered.effects.flattened_default_effects() {
        crate::effects::execute_effect(&mut game, effect, &mut resolution)
            .expect("accepted Rogue trigger should resolve");
    }

    assert_eq!(
        game.object(cloak).and_then(|object| object.attached_to),
        Some(crate::object::AttachmentTarget::Object(rogue))
    );
    assert_eq!(game.calculated_power(rogue), Some(4));
    assert!(game.object_has_static_ability_id(rogue, StaticAbilityId::Shroud));
    assert_eq!(game.calculated_power(original_host), Some(2));
    assert!(!game.object_has_static_ability_id(original_host, StaticAbilityId::Shroud));
}

#[test]
fn cloak_and_dagger_decline_keeps_its_existing_attachment() {
    let definition = parse_oracle_card_definition("Cloak and Dagger");
    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Cloak and Dagger should retain its Rogue ETB trigger");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let cloak = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let original_host = game.create_object_from_definition(
        &creature("Original Host", Vec::new()),
        alice,
        Zone::Battlefield,
    );
    let rogue = game.create_object_from_definition(
        &creature("Entering Rogue", vec![Subtype::Rogue]),
        alice,
        Zone::Battlefield,
    );
    assert!(game.attach_object_to_target(
        cloak,
        crate::object::AttachmentTarget::Object(original_host),
    ));

    let mut decline = crate::decision::AutoPassDecisionMaker;
    let mut resolution = crate::effects::ExecutionContext::new_default(cloak, alice)
        .with_decision_maker(&mut decline);
    resolution.triggering_event = Some(etb_event(&game, rogue));
    for effect in triggered.effects.flattened_default_effects() {
        crate::effects::execute_effect(&mut game, effect, &mut resolution)
            .expect("declined Rogue trigger should resolve as a no-op");
    }
    assert_eq!(
        game.object(cloak).and_then(|object| object.attached_to),
        Some(crate::object::AttachmentTarget::Object(original_host))
    );
}

#[test]
fn guidelight_matrix_activations_modify_their_targets_not_the_matrix() {
    let definition = parse_oracle_card_definition("Guidelight Matrix");
    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        "When this artifact enters, draw a card.\n{2}, {T}: Target Mount you control becomes saddled until end of turn. Activate only as a sorcery.\n{2}, {T}: Target Vehicle you control becomes an artifact creature until end of turn."
    );
    let saddle = activated_ability_containing(&definition, "BecomeSaddledUntilEotEffect");
    let animate = activated_ability_containing(&definition, "SetCardTypes");
    assert!(
        matches!(
            saddle.timing,
            crate::ability::ActivationTiming::SorcerySpeed
        ),
        "the Mount activation should retain its sorcery-speed restriction"
    );
    assert!(
        matches!(animate.timing, crate::ability::ActivationTiming::AnyTime),
        "the Vehicle activation has no authored sorcery-speed restriction"
    );

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let matrix = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let mount = game.create_object_from_definition(
        &creature("Matrix Mount", vec![Subtype::Mount]),
        alice,
        Zone::Battlefield,
    );
    let enemy_mount = game.create_object_from_definition(
        &creature("Enemy Mount", vec![Subtype::Mount]),
        bob,
        Zone::Battlefield,
    );
    let vehicle_definition = CardDefinitionBuilder::new(CardId::new(), "Matrix Vehicle")
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Vehicle])
        .build();
    let vehicle = game.create_object_from_definition(&vehicle_definition, alice, Zone::Battlefield);
    let enemy_vehicle =
        game.create_object_from_definition(&vehicle_definition, bob, Zone::Battlefield);

    let enters = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Guidelight Matrix should retain its ETB draw trigger");
    let draw_card = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::new(), "Matrix Draw")
            .card_types(vec![CardType::Instant])
            .build(),
        alice,
        Zone::Library,
    );
    let draw_stable = game.object(draw_card).unwrap().stable_id;
    let enter_event = etb_event(&game, matrix);
    assert!(enters.trigger.matches(
        &enter_event,
        &crate::triggers::TriggerContext::for_source(matrix, alice, &game),
    ));
    let mut enter_ctx = crate::effects::ExecutionContext::new_default(matrix, alice);
    enter_ctx.triggering_event = Some(enter_event);
    for effect in enters.effects.flattened_default_effects() {
        crate::effects::execute_effect(&mut game, effect, &mut enter_ctx)
            .expect("Guidelight Matrix ETB draw should resolve");
    }
    let drawn = game
        .find_object_by_stable_id(draw_stable)
        .unwrap_or(draw_card);
    assert_eq!(
        game.object(drawn).map(|object| object.zone),
        Some(Zone::Hand)
    );

    let saddle_legal = crate::game_loop::compute_legal_targets(
        &game,
        saddle.choices.first().unwrap(),
        alice,
        Some(matrix),
    );
    assert!(saddle_legal.contains(&crate::game_state::Target::Object(mount)));
    assert!(!saddle_legal.contains(&crate::game_state::Target::Object(enemy_mount)));
    let vehicle_legal = crate::game_loop::compute_legal_targets(
        &game,
        animate.choices.first().unwrap(),
        alice,
        Some(matrix),
    );
    assert!(vehicle_legal.contains(&crate::game_state::Target::Object(vehicle)));
    assert!(!vehicle_legal.contains(&crate::game_state::Target::Object(enemy_vehicle)));

    resolve_targeted_activation(&mut game, matrix, alice, mount, saddle);
    assert!(game.is_saddled(mount));
    assert!(!game.is_saddled(matrix));

    assert!(!game.current_is_creature(vehicle));
    resolve_targeted_activation(&mut game, matrix, alice, vehicle, animate);
    assert!(game.current_has_card_type(vehicle, CardType::Artifact));
    assert!(game.current_is_creature(vehicle));
    assert!(!game.current_is_creature(matrix));
    assert!(!game.current_is_creature(enemy_vehicle));

    crate::turn::execute_cleanup_step(&mut game);
    assert!(!game.current_is_creature(vehicle));
    game.next_turn();
    assert!(!game.is_saddled(mount));
}
