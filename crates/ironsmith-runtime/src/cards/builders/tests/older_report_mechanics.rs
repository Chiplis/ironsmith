#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

fn permanent(name: &str, card_type: CardType) -> CardDefinition {
    let mut builder = CardDefinitionBuilder::new(CardId::new(), name).card_types(vec![card_type]);
    if card_type == CardType::Creature {
        builder = builder.power_toughness(PowerToughness::fixed(2, 10));
    }
    builder.build()
}

fn flying_spell(name: &str, card_type: CardType) -> CardDefinition {
    let mut builder = CardDefinitionBuilder::new(CardId::new(), name)
        .mana_cost(crate::mana::ManaCost::from_pips(vec![vec![
            crate::mana::ManaSymbol::Generic(3),
        ]]))
        .card_types(vec![card_type]);
    if card_type == CardType::Creature {
        builder = builder.power_toughness(PowerToughness::fixed(2, 2));
    }
    builder
        .with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::flying(),
        ))
        .build()
}

fn nonflying_creature_spell(name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .mana_cost(crate::mana::ManaCost::from_pips(vec![vec![
            crate::mana::ManaSymbol::Generic(3),
        ]]))
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build()
}

fn zone_by_stable_id(game: &crate::game_state::GameState, stable_id: StableId) -> Option<Zone> {
    game.find_object_by_stable_id(stable_id)
        .and_then(|id| game.object(id))
        .map(|object| object.zone)
}

fn targeted_destroy_spec(definition: &CardDefinition) -> ChooseSpec {
    fn find(effect: &crate::effect::Effect) -> Option<ChooseSpec> {
        if let Some(destroy) = effect.downcast_ref::<crate::effects::DestroyEffect>()
            && matches!(destroy.spec, ChooseSpec::Target(_))
        {
            return Some(destroy.spec.clone());
        }
        let mut found = None;
        effect.visit_child_effects(&mut |child| {
            if found.is_none() {
                found = find(child);
            }
        });
        found
    }

    definition
        .spell_effect
        .as_ref()
        .into_iter()
        .flat_map(|program| &program.segments)
        .flat_map(|segment| &segment.default_effects)
        .find_map(find)
        .expect("spell should contain a targeted destroy effect")
}

fn effective_cost(
    game: &crate::game_state::GameState,
    caster: PlayerId,
    spell: ObjectId,
) -> String {
    let object = game.object(spell).expect("spell should exist");
    let base = object.mana_cost.as_ref().expect("spell should have a cost");
    crate::decision::calculate_effective_mana_cost(game, caster, object, base).to_oracle()
}

fn enter_and_resolve_triggers(
    game: &mut crate::game_state::GameState,
    object: ObjectId,
    watched_source: ObjectId,
) -> (ObjectId, usize) {
    let entered = game
        .move_object_by_effect(object, Zone::Battlefield)
        .expect("object should enter the battlefield");
    let mut queue = crate::triggers::TriggerQueue::new();
    crate::game_loop::drain_pending_trigger_events(game, &mut queue);
    let watched_count = queue
        .entries
        .iter()
        .filter(|entry| entry.source == watched_source)
        .count();
    if !queue.entries.is_empty() {
        crate::game_loop::put_triggers_on_stack(game, &mut queue)
            .expect("triggered abilities should go on the stack");
        while !game.stack_is_empty() {
            crate::game_loop::resolve_stack_entry(game)
                .expect("triggered abilities should resolve");
        }
    }
    (entered, watched_count)
}

fn resolve_hail_storm(
    game: &mut crate::game_state::GameState,
    definition: &CardDefinition,
    controller: PlayerId,
    prevent_next_damage_from_hail: bool,
) {
    let spell = game.create_object_from_definition(definition, controller, Zone::Stack);
    let mut decisions = crate::decision::SelectFirstDecisionMaker;
    if prevent_next_damage_from_hail {
        let shield = crate::effects::PreventNextTimeDamageEffect::new(
            crate::effects::PreventNextTimeDamageSource::Target(ChooseSpec::Source),
            crate::effects::PreventNextTimeDamageTarget::AnyTarget,
        );
        let mut shield_ctx =
            crate::effects::ExecutionContext::new(spell, controller, &mut decisions);
        crate::effects::execute_effect(game, &crate::effect::Effect::new(shield), &mut shield_ctx)
            .expect("damage-prevention shield should register");
    }
    let mut ctx = crate::effects::ExecutionContext::new(spell, controller, &mut decisions);
    crate::game_loop::execute_resolution_program(
        game,
        &mut ctx,
        controller,
        spell,
        definition
            .spell_effect
            .as_ref()
            .expect("Hail Storm should have a spell program"),
        None,
        &[],
    )
    .expect("Hail Storm should resolve");
}

#[test]
fn maelstrom_pulse_destroys_every_other_same_name_permanent_without_copying_target_scope() {
    let definition = parse_oracle_card_definition("Maelstrom Pulse");
    assert_eq!(
        canonical_compiled_lines(&definition),
        [
            "Destroy target nonland permanent and all other permanents with the same name as that permanent."
        ]
    );
    let debug = format!("{:#?}", definition.spell_effect);
    assert!(
        debug.contains("SameNameAsTagged") && debug.contains("IsNotTaggedObject"),
        "Pulse's fanout must retain the target's name and exclude only that object: {debug}"
    );

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let target = game.create_object_from_definition(
        &permanent("Pulse Family", CardType::Artifact),
        bob,
        Zone::Battlefield,
    );
    let same_name_creature = game.create_object_from_definition(
        &permanent("Pulse Family", CardType::Creature),
        alice,
        Zone::Battlefield,
    );
    let same_name_land = game.create_object_from_definition(
        &permanent("Pulse Family", CardType::Land),
        bob,
        Zone::Battlefield,
    );
    let unrelated = game.create_object_from_definition(
        &permanent("Unrelated Permanent", CardType::Enchantment),
        bob,
        Zone::Battlefield,
    );
    let target_stable = game.object(target).expect("target exists").stable_id;
    let creature_stable = game
        .object(same_name_creature)
        .expect("same-name creature exists")
        .stable_id;
    let land_stable = game
        .object(same_name_land)
        .expect("same-name land exists")
        .stable_id;

    let pulse = game.create_object_from_definition(&definition, alice, Zone::Stack);
    game.push_to_stack(
        crate::game_state::StackEntry::new(pulse, alice)
            .with_targets(vec![crate::game_state::Target::Object(target)]),
    );
    crate::game_loop::resolve_stack_entry(&mut game).expect("Maelstrom Pulse should resolve");

    assert_eq!(
        zone_by_stable_id(&game, target_stable),
        Some(Zone::Graveyard)
    );
    assert_eq!(
        zone_by_stable_id(&game, creature_stable),
        Some(Zone::Graveyard),
        "same-name fanout must cross controller and permanent-type boundaries"
    );
    assert_eq!(
        zone_by_stable_id(&game, land_stable),
        Some(Zone::Graveyard),
        "nonland restricts only the target; a same-name land is still an 'other permanent'"
    );
    assert_eq!(
        game.object(unrelated).map(|object| object.zone),
        Some(Zone::Battlefield),
        "a different name must remain untouched"
    );
}

#[test]
fn maelstrom_pulse_cannot_target_a_land_and_fizzles_before_same_name_fanout() {
    let definition = parse_oracle_card_definition("Maelstrom Pulse");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let target = game.create_object_from_definition(
        &permanent("Vanishing Family", CardType::Artifact),
        bob,
        Zone::Battlefield,
    );
    let same_name = game.create_object_from_definition(
        &permanent("Vanishing Family", CardType::Creature),
        bob,
        Zone::Battlefield,
    );
    let land = game.create_object_from_definition(
        &permanent("Targeting Probe Land", CardType::Land),
        bob,
        Zone::Battlefield,
    );
    let same_name_stable = game.object(same_name).expect("same-name object").stable_id;
    let pulse = game.create_object_from_definition(&definition, alice, Zone::Stack);

    let target_spec = targeted_destroy_spec(&definition);
    let legal = crate::game_loop::compute_legal_targets(&game, &target_spec, alice, Some(pulse));
    assert!(legal.contains(&crate::game_state::Target::Object(target)));
    assert!(
        !legal.contains(&crate::game_state::Target::Object(land)),
        "Pulse's announced target must be nonland"
    );

    game.push_to_stack(
        crate::game_state::StackEntry::new(pulse, alice)
            .with_targets(vec![crate::game_state::Target::Object(target)]),
    );
    game.move_object_by_effect(target, Zone::Graveyard)
        .expect("the target should leave before resolution");
    crate::game_loop::resolve_stack_entry(&mut game).expect("Pulse should fizzle cleanly");
    assert_eq!(
        zone_by_stable_id(&game, same_name_stable),
        Some(Zone::Battlefield),
        "when its only target is illegal, Pulse must not perform the untargeted fanout"
    );
}

#[test]
fn maelstrom_pulse_fanout_continues_when_the_target_is_indestructible() {
    let definition = parse_oracle_card_definition("Maelstrom Pulse");
    let indestructible = CardDefinitionBuilder::new(CardId::new(), "Resilient Family")
        .card_types(vec![CardType::Artifact])
        .with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::indestructible(),
        ))
        .build();
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let target = game.create_object_from_definition(&indestructible, bob, Zone::Battlefield);
    let other = game.create_object_from_definition(
        &permanent("Resilient Family", CardType::Creature),
        bob,
        Zone::Battlefield,
    );
    let other_stable = game.object(other).expect("other permanent").stable_id;
    let pulse = game.create_object_from_definition(&definition, alice, Zone::Stack);
    game.push_to_stack(
        crate::game_state::StackEntry::new(pulse, alice)
            .with_targets(vec![crate::game_state::Target::Object(target)]),
    );

    crate::game_loop::resolve_stack_entry(&mut game).expect("Pulse should resolve");
    assert_eq!(
        game.object(target).map(|object| object.zone),
        Some(Zone::Battlefield),
        "the indestructible target should survive"
    );
    assert_eq!(
        zone_by_stable_id(&game, other_stable),
        Some(Zone::Graveyard),
        "one failed destruction must not stop the independent same-name destructions"
    );
}

#[test]
fn hail_storm_applies_both_overlapping_damage_sets_and_keeps_controller_scope() {
    let definition = parse_oracle_card_definition("Hail Storm");
    assert_eq!(
        canonical_compiled_lines(&definition),
        [
            "Hail Storm deals 2 damage to each attacking creature, Hail Storm deals 1 damage to you, and Hail Storm deals 1 damage to each creature you control."
        ]
    );

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let mut alice_attacks = crate::tests::test_helpers::setup_two_player_game();
    let alice_attacker = alice_attacks.create_object_from_definition(
        &permanent("Alice Attacker", CardType::Creature),
        alice,
        Zone::Battlefield,
    );
    let alice_nonattacker = alice_attacks.create_object_from_definition(
        &permanent("Alice Nonattacker", CardType::Creature),
        alice,
        Zone::Battlefield,
    );
    let bob_nonattacker = alice_attacks.create_object_from_definition(
        &permanent("Bob Nonattacker", CardType::Creature),
        bob,
        Zone::Battlefield,
    );
    alice_attacks.combat = Some(crate::combat_state::CombatState {
        attackers: vec![crate::combat_state::AttackerInfo {
            creature: alice_attacker,
            target: crate::combat_state::AttackTarget::Player(bob),
        }],
        ..Default::default()
    });
    resolve_hail_storm(&mut alice_attacks, &definition, alice, false);
    assert_eq!(
        alice_attacks.damage_on(alice_attacker),
        3,
        "a controlled attacking creature belongs to both damage sets"
    );
    assert_eq!(alice_attacks.damage_on(alice_nonattacker), 1);
    assert_eq!(alice_attacks.damage_on(bob_nonattacker), 0);
    assert_eq!(alice_attacks.player(alice).expect("Alice").life, 19);
    assert_eq!(alice_attacks.player(bob).expect("Bob").life, 20);

    let mut bob_attacks = crate::tests::test_helpers::setup_two_player_game();
    let bob_attacker = bob_attacks.create_object_from_definition(
        &permanent("Bob Attacker", CardType::Creature),
        bob,
        Zone::Battlefield,
    );
    let bob_nonattacker = bob_attacks.create_object_from_definition(
        &permanent("Bob Nonattacker", CardType::Creature),
        bob,
        Zone::Battlefield,
    );
    let alice_creature = bob_attacks.create_object_from_definition(
        &permanent("Alice Creature", CardType::Creature),
        alice,
        Zone::Battlefield,
    );
    bob_attacks.combat = Some(crate::combat_state::CombatState {
        attackers: vec![crate::combat_state::AttackerInfo {
            creature: bob_attacker,
            target: crate::combat_state::AttackTarget::Player(alice),
        }],
        ..Default::default()
    });
    resolve_hail_storm(&mut bob_attacks, &definition, alice, false);
    assert_eq!(bob_attacks.damage_on(bob_attacker), 2);
    assert_eq!(bob_attacks.damage_on(bob_nonattacker), 0);
    assert_eq!(bob_attacks.damage_on(alice_creature), 1);
    assert_eq!(bob_attacks.player(alice).expect("Alice").life, 19);
}

#[test]
fn hail_storm_damage_is_ordinary_and_preventable() {
    let definition = parse_oracle_card_definition("Hail Storm");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let attacker = game.create_object_from_definition(
        &permanent("Prevented Attacker", CardType::Creature),
        bob,
        Zone::Battlefield,
    );
    let controlled_creature = game.create_object_from_definition(
        &permanent("Later Controlled Creature", CardType::Creature),
        alice,
        Zone::Battlefield,
    );
    game.combat = Some(crate::combat_state::CombatState {
        attackers: vec![crate::combat_state::AttackerInfo {
            creature: attacker,
            target: crate::combat_state::AttackTarget::Player(alice),
        }],
        ..Default::default()
    });

    resolve_hail_storm(&mut game, &definition, alice, true);
    assert_eq!(
        game.damage_on(attacker),
        0,
        "the next damage event from Hail Storm should be preventable"
    );
    assert_eq!(
        game.player(alice).expect("Alice").life,
        19,
        "preventing the first damage action must not stop Hail's later damage instructions"
    );
    assert_eq!(
        game.damage_on(controlled_creature),
        1,
        "the final controlled-creature damage instruction must still happen"
    );
}

#[test]
fn watcher_reduces_only_flying_creature_spells_cast_by_its_current_controller() {
    let mut watcher_definition = parse_oracle_card_definition("Watcher of the Spheres");
    watcher_definition.card.power_toughness = Some(PowerToughness::fixed(2, 2));
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let watcher = game.create_object_from_definition(&watcher_definition, alice, Zone::Battlefield);
    let alice_flyer = game.create_object_from_definition(
        &flying_spell("Alice Flying Creature", CardType::Creature),
        alice,
        Zone::Hand,
    );
    let alice_ground = game.create_object_from_definition(
        &nonflying_creature_spell("Alice Ground Creature"),
        alice,
        Zone::Hand,
    );
    let alice_flying_noncreature = game.create_object_from_definition(
        &flying_spell("Alice Flying Artifact", CardType::Artifact),
        alice,
        Zone::Hand,
    );
    let bob_flyer = game.create_object_from_definition(
        &flying_spell("Bob Flying Creature", CardType::Creature),
        bob,
        Zone::Hand,
    );

    assert_eq!(effective_cost(&game, alice, alice_flyer), "{2}");
    assert_eq!(effective_cost(&game, alice, alice_ground), "{3}");
    assert_eq!(
        effective_cost(&game, alice, alice_flying_noncreature),
        "{3}"
    );
    assert_eq!(
        effective_cost(&game, bob, bob_flyer),
        "{3}",
        "an opponent's flying creature spell must not receive Alice's reduction"
    );

    game.set_current_controller(watcher, bob);
    game.refresh_continuous_state();
    assert_eq!(effective_cost(&game, alice, alice_flyer), "{3}");
    assert_eq!(
        effective_cost(&game, bob, bob_flyer),
        "{2}",
        "'you cast' must follow Watcher's current controller"
    );
}

#[test]
fn watcher_triggers_for_every_other_controlled_flyer_and_buffs_reset_at_end_of_turn() {
    let mut watcher_definition = parse_oracle_card_definition("Watcher of the Spheres");
    watcher_definition.card.power_toughness = Some(PowerToughness::fixed(2, 2));
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let watcher_in_hand =
        game.create_object_from_definition(&watcher_definition, alice, Zone::Hand);
    let watcher = game
        .move_object_by_effect(watcher_in_hand, Zone::Battlefield)
        .expect("Watcher should enter");
    let mut self_queue = crate::triggers::TriggerQueue::new();
    crate::game_loop::drain_pending_trigger_events(&mut game, &mut self_queue);
    assert!(
        self_queue
            .entries
            .iter()
            .all(|entry| entry.source != watcher),
        "'another' must prevent Watcher from triggering for its own entry"
    );

    let first = game.create_object_from_definition(
        &flying_spell("First Controlled Flyer", CardType::Creature),
        alice,
        Zone::Hand,
    );
    let (_, first_count) = enter_and_resolve_triggers(&mut game, first, watcher);
    assert_eq!(first_count, 1);
    assert_eq!(game.current_power(watcher), Some(3));
    assert_eq!(game.current_toughness(watcher), Some(3));

    let second = game.create_object_from_definition(
        &flying_spell("Second Controlled Flyer", CardType::Creature),
        alice,
        Zone::Hand,
    );
    let (_, second_count) = enter_and_resolve_triggers(&mut game, second, watcher);
    assert_eq!(
        second_count, 1,
        "the current oracle says 'whenever', so this is not limited to the first flyer each turn"
    );
    assert_eq!(game.current_power(watcher), Some(4));
    assert_eq!(game.current_toughness(watcher), Some(4));

    let ground = game.create_object_from_definition(
        &nonflying_creature_spell("Controlled Ground Creature"),
        alice,
        Zone::Hand,
    );
    let (_, ground_count) = enter_and_resolve_triggers(&mut game, ground, watcher);
    assert_eq!(ground_count, 0);
    let opponent_flyer = game.create_object_from_definition(
        &flying_spell("Opponent Flyer", CardType::Creature),
        bob,
        Zone::Hand,
    );
    let (_, opponent_count) = enter_and_resolve_triggers(&mut game, opponent_flyer, watcher);
    assert_eq!(opponent_count, 0);
    assert_eq!(game.current_power(watcher), Some(4));

    game.effect_store.continuous_effects.cleanup_end_of_turn();
    game.refresh_continuous_state();
    assert_eq!(game.current_power(watcher), Some(2));
    assert_eq!(game.current_toughness(watcher), Some(2));
}
