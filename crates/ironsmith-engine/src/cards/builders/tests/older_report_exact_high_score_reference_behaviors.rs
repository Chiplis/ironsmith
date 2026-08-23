#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

fn creature_definition(name: &str, power: i32, subtypes: Vec<Subtype>) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .subtypes(subtypes)
        .power_toughness(PowerToughness::fixed(power, power.max(1)))
        .build()
}

fn zone_by_stable_id(game: &crate::GameState, stable_id: StableId) -> Option<Zone> {
    game.find_object_by_stable_id(stable_id)
        .and_then(|id| game.object(id))
        .map(|object| object.zone)
}

fn activated_ability(
    definition: &CardDefinition,
    index: usize,
) -> &crate::ability::ActivatedAbility {
    definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .nth(index)
        .unwrap_or_else(|| panic!("missing activated ability {index} on {}", definition.name()))
}

fn triggered_ability(definition: &CardDefinition) -> &crate::ability::TriggeredAbility {
    definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing triggered ability on {}", definition.name()))
}

#[test]
fn ashiok_nightmare_weaver_minus_x_uses_only_source_linked_exact_mana_value_creatures() {
    let definition = parse_oracle_card_definition("Ashiok, Nightmare Weaver");
    let ability = activated_ability(&definition, 1);
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let ashiok = game.create_object_from_definition(&definition, alice, Zone::Battlefield);

    let exact = CardDefinitionBuilder::new(CardId::new(), "Linked Three Drop")
        .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Generic(3)]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Elf])
        .power_toughness(PowerToughness::fixed(3, 3))
        .build();
    let exact = game.create_object_from_definition(&exact, bob, Zone::Exile);
    let exact_stable = game.object(exact).expect("exact candidate").stable_id;
    game.add_exiled_with_source_link(ashiok, exact);

    let wrong_value = CardDefinitionBuilder::new(CardId::new(), "Linked Four Drop")
        .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Generic(4)]))
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(4, 4))
        .build();
    let wrong_value = game.create_object_from_definition(&wrong_value, bob, Zone::Exile);
    game.add_exiled_with_source_link(ashiok, wrong_value);

    let unlinked = CardDefinitionBuilder::new(CardId::new(), "Unlinked Three Drop")
        .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Generic(3)]))
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 3))
        .build();
    let unlinked = game.create_object_from_definition(&unlinked, bob, Zone::Exile);

    let linked_noncreature = CardDefinitionBuilder::new(CardId::new(), "Linked Three Mana Spell")
        .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Generic(3)]))
        .card_types(vec![CardType::Sorcery])
        .build();
    let linked_noncreature =
        game.create_object_from_definition(&linked_noncreature, bob, Zone::Exile);
    game.add_exiled_with_source_link(ashiok, linked_noncreature);

    let mut decisions = crate::decision::SelectFirstDecisionMaker;
    let mut context =
        crate::effects::ExecutionContext::new(ashiok, alice, &mut decisions).with_x(3);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut context,
        alice,
        ashiok,
        &ability.effects,
        None,
        &[],
    )
    .expect("Ashiok's −X should resolve");

    let entered = game
        .find_object_by_stable_id(exact_stable)
        .expect("the linked exact-value creature should retain stable identity");
    assert_eq!(
        game.object(entered).expect("returned creature").zone,
        Zone::Battlefield
    );
    assert_eq!(
        game.controller_of(game.object(entered).expect("returned creature")),
        alice
    );
    assert!(game.current_has_subtype(entered, Subtype::Elf));
    assert!(game.current_has_subtype(entered, Subtype::Nightmare));
    assert_eq!(
        game.object(wrong_value).expect("wrong value remains").zone,
        Zone::Exile
    );
    assert_eq!(
        game.object(unlinked).expect("unlinked remains").zone,
        Zone::Exile
    );
    assert_eq!(
        game.object(linked_noncreature)
            .expect("linked noncreature remains")
            .zone,
        Zone::Exile
    );
}

#[test]
fn ashiok_nightmare_weaver_ultimate_exiles_only_opponents_hands_and_graveyards() {
    let definition = parse_oracle_card_definition("Ashiok, Nightmare Weaver");
    let ability = activated_ability(&definition, 2);
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into(), "Cara".into()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let cara = PlayerId::from_index(2);
    let ashiok = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let filler = CardDefinitionBuilder::new(CardId::new(), "Ultimate Filler")
        .card_types(vec![CardType::Sorcery])
        .build();
    let alice_hand = game.create_object_from_definition(&filler, alice, Zone::Hand);
    let alice_grave = game.create_object_from_definition(&filler, alice, Zone::Graveyard);
    let bob_hand = game.create_object_from_definition(&filler, bob, Zone::Hand);
    let bob_grave = game.create_object_from_definition(&filler, bob, Zone::Graveyard);
    let cara_hand = game.create_object_from_definition(&filler, cara, Zone::Hand);
    let cara_grave = game.create_object_from_definition(&filler, cara, Zone::Graveyard);
    let tracked = [
        alice_hand,
        alice_grave,
        bob_hand,
        bob_grave,
        cara_hand,
        cara_grave,
    ]
    .map(|id| game.object(id).expect("tracked card").stable_id);

    let mut context = crate::effects::ExecutionContext::new_default(ashiok, alice);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut context,
        alice,
        ashiok,
        &ability.effects,
        None,
        &[],
    )
    .expect("Ashiok's ultimate should resolve");

    assert_eq!(zone_by_stable_id(&game, tracked[0]), Some(Zone::Hand));
    assert_eq!(zone_by_stable_id(&game, tracked[1]), Some(Zone::Graveyard));
    for stable in &tracked[2..] {
        assert_eq!(zone_by_stable_id(&game, *stable), Some(Zone::Exile));
    }
}

struct ChooseElfDecisionMaker;

impl crate::decision::DecisionMaker for ChooseElfDecisionMaker {
    fn decide_options(
        &mut self,
        _game: &crate::GameState,
        context: &crate::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        context
            .options
            .iter()
            .find(|option| option.description.eq_ignore_ascii_case("elf"))
            .map(|option| vec![option.index])
            .unwrap_or_default()
    }
}

#[test]
fn elvish_soultiller_shuffles_only_your_creature_cards_of_the_chosen_type() {
    let definition = parse_oracle_card_definition("Elvish Soultiller");
    let triggered = triggered_ability(&definition);
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let soultiller = game.create_object_from_definition(&definition, alice, Zone::Battlefield);

    let elf = creature_definition("Chosen Elf Creature", 2, vec![Subtype::Elf]);
    let goblin = creature_definition("Wrong Creature Type", 2, vec![Subtype::Goblin]);
    let kindred_elf = CardDefinitionBuilder::new(CardId::new(), "Noncreature Elf Card")
        .card_types(vec![CardType::Kindred, CardType::Instant])
        .subtypes(vec![Subtype::Elf])
        .build();
    let alice_elf = game.create_object_from_definition(&elf, alice, Zone::Graveyard);
    let alice_goblin = game.create_object_from_definition(&goblin, alice, Zone::Graveyard);
    let alice_kindred = game.create_object_from_definition(&kindred_elf, alice, Zone::Graveyard);
    let bob_elf = game.create_object_from_definition(&elf, bob, Zone::Graveyard);
    let tracked = [alice_elf, alice_goblin, alice_kindred, bob_elf]
        .map(|id| game.object(id).expect("tracked graveyard card").stable_id);

    let mut decisions = ChooseElfDecisionMaker;
    let mut context = crate::effects::ExecutionContext::new(soultiller, alice, &mut decisions);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut context,
        alice,
        soultiller,
        &triggered.effects,
        None,
        &[],
    )
    .expect("Elvish Soultiller's death trigger should resolve");

    assert_eq!(zone_by_stable_id(&game, tracked[0]), Some(Zone::Library));
    assert_eq!(zone_by_stable_id(&game, tracked[1]), Some(Zone::Graveyard));
    assert_eq!(zone_by_stable_id(&game, tracked[2]), Some(Zone::Graveyard));
    assert_eq!(zone_by_stable_id(&game, tracked[3]), Some(Zone::Graveyard));
}

fn eomer_run_attacks_with_global_maximum(enemy_power: i32) -> (usize, usize) {
    let definition = parse_oracle_card_definition("Éomer of the Riddermark");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let eomer = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    game.create_object_from_definition(
        &creature_definition("Alice Five Power", 5, Vec::new()),
        alice,
        Zone::Battlefield,
    );
    game.create_object_from_definition(
        &creature_definition("Global Rival", enemy_power, Vec::new()),
        bob,
        Zone::Battlefield,
    );
    let event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::combat::CreatureAttackedEvent::new(
            eomer,
            crate::events::combat::AttackEventTarget::Player(bob),
        ),
        crate::provenance::ProvNodeId::default(),
    );
    let entries = crate::triggers::check_triggers(&game, &event)
        .into_iter()
        .filter(|entry| entry.source == eomer)
        .collect::<Vec<_>>();
    let trigger_count = entries.len();
    let tokens_before = game
        .battlefield
        .iter()
        .filter(|id| {
            game.object(**id)
                .is_some_and(|object| object.kind == crate::object::ObjectKind::Token)
        })
        .count();

    let mut queue = crate::triggers::TriggerQueue::new();
    for entry in entries {
        queue.add(entry);
    }
    crate::game_loop::put_triggers_on_stack(&mut game, &mut queue)
        .expect("Éomer's matching trigger should go on the stack");
    while !game.stack_is_empty() {
        crate::game_loop::resolve_stack_entry(&mut game)
            .expect("Éomer's attack trigger should resolve");
    }

    let tokens_after = game
        .battlefield
        .iter()
        .filter(|id| {
            game.object(**id)
                .is_some_and(|object| object.kind == crate::object::ObjectKind::Token)
        })
        .count();
    if tokens_after > tokens_before {
        let token = game
            .battlefield
            .iter()
            .copied()
            .find(|id| {
                game.object(*id)
                    .is_some_and(|object| object.kind == crate::object::ObjectKind::Token)
            })
            .expect("Éomer should create a token");
        assert!(game.current_has_subtype(token, Subtype::Human));
        assert!(game.current_has_subtype(token, Subtype::Soldier));
        assert_eq!(game.current_power(token), Some(1));
        assert_eq!(
            game.current_colors(token),
            Some(crate::color::ColorSet::WHITE)
        );
    }
    (trigger_count, tokens_after - tokens_before)
}

#[test]
fn eomer_of_the_riddermark_uses_the_global_greatest_power_with_ties_allowed() {
    assert_eq!(
        eomer_run_attacks_with_global_maximum(5),
        (1, 1),
        "a controlled creature tied for greatest power globally should satisfy Éomer"
    );
    assert_eq!(
        eomer_run_attacks_with_global_maximum(6),
        (0, 0),
        "an opponent's strictly larger creature should stop Éomer's trigger"
    );
}

#[test]
fn hellfire_counts_only_nonblack_creatures_that_actually_die_this_way() {
    let definition = parse_oracle_card_definition("Hellfire");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let ordinary = creature_definition("Ordinary Nonblack", 2, Vec::new());
    let first = game.create_object_from_definition(&ordinary, alice, Zone::Battlefield);
    let second = game.create_object_from_definition(&ordinary, bob, Zone::Battlefield);
    let first_stable = game
        .object(first)
        .expect("first ordinary creature")
        .stable_id;
    let second_stable = game
        .object(second)
        .expect("second ordinary creature")
        .stable_id;

    let black = CardDefinitionBuilder::new(CardId::new(), "Black Survivor")
        .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Black]))
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let black = game.create_object_from_definition(&black, bob, Zone::Battlefield);

    let indestructible = CardDefinitionBuilder::new(CardId::new(), "Indestructible Survivor")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::indestructible(),
        ))
        .build();
    let indestructible =
        game.create_object_from_definition(&indestructible, bob, Zone::Battlefield);

    let finality = game.create_object_from_definition(&ordinary, bob, Zone::Battlefield);
    let finality_stable = game.object(finality).expect("finality creature").stable_id;
    game.add_counters(finality, crate::object::CounterType::Finality, 1);

    let hellfire = game.create_object_from_definition(&definition, alice, Zone::Stack);
    game.push_to_stack(crate::game_state::StackEntry::new(hellfire, alice));
    crate::game_loop::resolve_stack_entry(&mut game).expect("Hellfire should resolve");

    assert_eq!(
        zone_by_stable_id(&game, first_stable),
        Some(Zone::Graveyard)
    );
    assert_eq!(
        zone_by_stable_id(&game, second_stable),
        Some(Zone::Graveyard)
    );
    assert_eq!(zone_by_stable_id(&game, finality_stable), Some(Zone::Exile));
    assert_eq!(
        game.object(black).expect("black creature survives").zone,
        Zone::Battlefield
    );
    assert_eq!(
        game.object(indestructible)
            .expect("indestructible creature survives")
            .zone,
        Zone::Battlefield
    );
    assert_eq!(
        game.player(alice).expect("Alice exists").life,
        15,
        "two creatures died this way, so Hellfire should deal 2 + 3 damage"
    );
}

#[test]
fn idol_of_false_gods_threshold_changes_only_the_source_and_grants_real_annihilator() {
    let definition = parse_oracle_card_definition("Idol of False Gods");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let idol = game.create_object_from_definition(&definition, alice, Zone::Battlefield);

    game.add_counters(idol, crate::object::CounterType::PlusOnePlusOne, 7);
    assert!(game.current_has_card_type(idol, CardType::Artifact));
    assert!(!game.current_has_card_type(idol, CardType::Creature));
    assert!(
        game.current_abilities(idol)
            .expect("Idol has current abilities")
            .iter()
            .all(
                |ability| !matches!(&ability.kind, AbilityKind::Triggered(triggered)
                if triggered.trigger.display() == "Whenever this creature attacks")
            ),
        "seven counters must not grant annihilator"
    );

    game.add_counters(idol, crate::object::CounterType::PlusOnePlusOne, 1);
    assert!(game.current_has_card_type(idol, CardType::Artifact));
    assert!(game.current_has_card_type(idol, CardType::Creature));
    assert_eq!(game.current_power(idol), Some(8));
    assert_eq!(game.current_toughness(idol), Some(8));
    let annihilator = game
        .current_abilities(idol)
        .expect("Idol has current abilities")
        .into_iter()
        .find_map(|ability| match ability.kind {
            AbilityKind::Triggered(triggered)
                if triggered.trigger.display() == "Whenever this creature attacks" =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("eight counters should grant executable annihilator 2");

    let fodder = CardDefinitionBuilder::new(CardId::new(), "Annihilator Fodder")
        .card_types(vec![CardType::Artifact])
        .build();
    let bob_first = game.create_object_from_definition(&fodder, bob, Zone::Battlefield);
    let bob_second = game.create_object_from_definition(&fodder, bob, Zone::Battlefield);
    let bob_third = game.create_object_from_definition(&fodder, bob, Zone::Battlefield);
    let alice_fodder = game.create_object_from_definition(&fodder, alice, Zone::Battlefield);
    let bob_first_stable = game.object(bob_first).expect("first fodder").stable_id;
    let bob_second_stable = game.object(bob_second).expect("second fodder").stable_id;
    let mut context = crate::effects::ExecutionContext::new_default(idol, alice)
        .with_defending_player(bob)
        .with_targets(vec![
            crate::effects::ResolvedTarget::Object(bob_first),
            crate::effects::ResolvedTarget::Object(bob_second),
        ]);
    for effect in &annihilator.effects {
        crate::effects::execute_effect(&mut game, effect, &mut context)
            .expect("Idol's granted annihilator should resolve");
    }

    assert_eq!(
        zone_by_stable_id(&game, bob_first_stable),
        Some(Zone::Graveyard)
    );
    assert_eq!(
        zone_by_stable_id(&game, bob_second_stable),
        Some(Zone::Graveyard)
    );
    assert_eq!(
        game.object(bob_third).expect("third fodder").zone,
        Zone::Battlefield
    );
    assert_eq!(
        game.object(alice_fodder).expect("Alice fodder").zone,
        Zone::Battlefield
    );
}

fn impulsivity_etb_event(
    game: &crate::GameState,
    source: ObjectId,
) -> crate::triggers::TriggerEvent {
    let snapshot = crate::snapshot::ObjectSnapshot::from_object(
        game.object(source).expect("Impulsivity exists"),
        game,
    );
    crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::ZoneChangeEvent::with_cause(
            source,
            Zone::Stack,
            Zone::Battlefield,
            crate::events::cause::EventCause::effect(),
            Some(snapshot),
        ),
        crate::provenance::ProvNodeId::default(),
    )
}

#[test]
fn impulsivity_free_casts_the_target_graveyard_spell_and_exiles_only_that_spell() {
    let definition = parse_oracle_card_definition("Impulsivity");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let impulsivity = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let captured_definition = CardDefinitionBuilder::new(CardId::new(), "Captured Grave Spell")
        .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Generic(9)]))
        .card_types(vec![CardType::Sorcery])
        .build();
    let captured = game.create_object_from_definition(&captured_definition, bob, Zone::Graveyard);
    let captured_stable = game.object(captured).expect("captured spell").stable_id;
    let illegal_creature = game.create_object_from_definition(
        &creature_definition("Illegal Grave Creature", 2, Vec::new()),
        bob,
        Zone::Graveyard,
    );

    let event = impulsivity_etb_event(&game, impulsivity);
    let entries = crate::triggers::check_triggers(&game, &event)
        .into_iter()
        .filter(|entry| entry.source == impulsivity)
        .collect::<Vec<_>>();
    assert_eq!(entries.len(), 1);
    let mut queue = crate::triggers::TriggerQueue::new();
    for entry in entries {
        queue.add(entry);
    }
    let mut decisions = crate::decision::SelectFirstDecisionMaker;
    crate::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut decisions)
        .expect("Impulsivity's ETB trigger should go on the stack");
    crate::game_loop::resolve_stack_entry_with(&mut game, &mut decisions)
        .expect("Impulsivity should cast its selected graveyard card");

    let cast = game
        .find_object_by_stable_id(captured_stable)
        .expect("captured spell should retain stable identity");
    assert_eq!(game.object(cast).expect("cast spell").zone, Zone::Stack);
    assert_eq!(
        game.stack
            .iter()
            .find(|entry| entry.object_id == cast)
            .expect("captured spell has a stack entry")
            .controller,
        alice
    );
    assert_eq!(
        game.object(illegal_creature)
            .expect("creature remains")
            .zone,
        Zone::Graveyard
    );

    crate::game_loop::resolve_stack_entry_with(&mut game, &mut decisions)
        .expect("the captured spell should resolve");
    assert_eq!(zone_by_stable_id(&game, captured_stable), Some(Zone::Exile));

    let unrelated_definition = CardDefinitionBuilder::new(CardId::new(), "Unrelated Later Spell")
        .card_types(vec![CardType::Sorcery])
        .build();
    let unrelated = game.create_object_from_definition(&unrelated_definition, alice, Zone::Stack);
    let unrelated_stable = game.object(unrelated).expect("unrelated spell").stable_id;
    game.push_to_stack(crate::game_state::StackEntry::new(unrelated, alice));
    crate::game_loop::resolve_stack_entry(&mut game).expect("unrelated spell should resolve");
    assert_eq!(
        zone_by_stable_id(&game, unrelated_stable),
        Some(Zone::Graveyard),
        "Impulsivity's replacement must stay scoped to the spell it cast"
    );
}

#[test]
fn impulsivity_decline_leaves_the_legal_target_in_its_graveyard() {
    let definition = parse_oracle_card_definition("Impulsivity");
    let triggered = triggered_ability(&definition);
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let impulsivity = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let target_definition = CardDefinitionBuilder::new(CardId::new(), "Declined Grave Spell")
        .card_types(vec![CardType::Instant])
        .build();
    let target = game.create_object_from_definition(&target_definition, bob, Zone::Graveyard);
    let stable = game.object(target).expect("declined target").stable_id;
    let event = impulsivity_etb_event(&game, impulsivity);
    game.push_to_stack(
        crate::game_state::StackEntry::ability(impulsivity, alice, triggered.effects.clone())
            .with_targets(vec![crate::game_state::Target::Object(target)])
            .with_triggering_event(event),
    );

    let mut decisions = crate::decision::AutoPassDecisionMaker;
    crate::game_loop::resolve_stack_entry_with(&mut game, &mut decisions)
        .expect("declining Impulsivity should resolve without casting");
    assert_eq!(zone_by_stable_id(&game, stable), Some(Zone::Graveyard));
    assert!(game.stack.is_empty());
}

#[test]
fn fire_nation_archers_damages_each_opponent_and_creates_one_controller_token() {
    let definition = parse_oracle_card_definition("Fire Nation Archers");
    let activated = activated_ability(&definition, 0);
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into(), "Cara".into()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let cara = PlayerId::from_index(2);
    let archers = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let mut context = crate::effects::ExecutionContext::new_default(archers, alice);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut context,
        alice,
        archers,
        &activated.effects,
        None,
        &[],
    )
    .expect("Fire Nation Archers' activation should resolve");

    assert_eq!(game.player(alice).expect("Alice").life, 20);
    assert_eq!(game.player(bob).expect("Bob").life, 18);
    assert_eq!(game.player(cara).expect("Cara").life, 18);
    let tokens = game
        .battlefield
        .iter()
        .copied()
        .filter(|id| {
            game.object(*id).is_some_and(|object| {
                object.kind == crate::object::ObjectKind::Token
                    && game.controller_of(object) == alice
            })
        })
        .collect::<Vec<_>>();
    assert_eq!(tokens.len(), 1);
    let token = tokens[0];
    assert!(game.current_has_subtype(token, Subtype::Soldier));
    assert_eq!(game.current_power(token), Some(2));
    assert_eq!(game.current_toughness(token), Some(2));
    assert_eq!(
        game.current_colors(token),
        Some(crate::color::ColorSet::RED)
    );
}

#[test]
fn fire_nation_soldier_haste_is_executable_and_does_not_spread_to_a_bystander() {
    let definition = parse_oracle_card_definition("Fire Nation Soldier");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let soldier = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let bystander = game.create_object_from_definition(
        &creature_definition("Fresh Bystander", 3, vec![Subtype::Soldier]),
        alice,
        Zone::Battlefield,
    );
    game.set_summoning_sick(soldier);
    game.set_summoning_sick(bystander);

    assert!(
        crate::rules::combat::can_attack(game.object(soldier).expect("soldier exists"), &game),
        "the freshly controlled Fire Nation Soldier should attack through haste"
    );
    assert!(
        !crate::rules::combat::can_attack(game.object(bystander).expect("bystander exists"), &game,),
        "haste must not leak to an otherwise identical fresh bystander"
    );
}
