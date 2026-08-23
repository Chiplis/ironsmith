#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

fn nested_counter_effect(effect: &crate::effect::Effect) -> Option<crate::effects::CounterEffect> {
    if let Some(counter) = effect.downcast_ref::<crate::effects::CounterEffect>() {
        return Some(counter.clone());
    }
    let mut found = None;
    effect.visit_child_effects(&mut |child| {
        if found.is_none() {
            found = nested_counter_effect(child);
        }
    });
    found
}

fn counter_from_program(
    program: &crate::resolution::ResolutionProgram,
) -> crate::effects::CounterEffect {
    program
        .segments
        .iter()
        .flat_map(|segment| &segment.default_effects)
        .find_map(nested_counter_effect)
        .expect("resolution program should contain a counter effect")
}

fn counter_ability(definition: &crate::cards::CardDefinition) -> &crate::ability::ActivatedAbility {
    definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated)
                if activated
                    .effects
                    .segments
                    .iter()
                    .flat_map(|segment| &segment.default_effects)
                    .any(|effect| nested_counter_effect(effect).is_some()) =>
            {
                Some(activated)
            }
            _ => None,
        })
        .expect("card should have an activated counter ability")
}

fn simple_permanent(name: &str, card_type: CardType, power: Option<i32>) -> CardDefinition {
    let mut builder = CardDefinitionBuilder::new(CardId::new(), name).card_types(vec![card_type]);
    if let Some(power) = power {
        builder = builder.power_toughness(PowerToughness::fixed(power, power));
    }
    builder.build()
}

fn push_targeting_spell(
    game: &mut crate::game_state::GameState,
    controller: PlayerId,
    name: &str,
    targets: Vec<crate::game_state::Target>,
) -> ObjectId {
    let definition = CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Instant])
        .build();
    let spell = game.create_object_from_definition(&definition, controller, Zone::Stack);
    game.push_to_stack(crate::game_state::StackEntry::new(spell, controller).with_targets(targets));
    spell
}

fn push_targeting_ability(
    game: &mut crate::game_state::GameState,
    controller: PlayerId,
    name: &str,
    targets: Vec<crate::game_state::Target>,
) -> ObjectId {
    let definition = simple_permanent(name, CardType::Artifact, None);
    let source = game.create_object_from_definition(&definition, controller, Zone::Battlefield);
    game.push_to_stack(
        crate::game_state::StackEntry::ability(
            source,
            controller,
            vec![crate::effect::Effect::draw(1)],
        )
        .with_targets(targets),
    );
    source
}

fn legal_counter_targets(
    game: &crate::game_state::GameState,
    counter: &crate::effects::CounterEffect,
    controller: PlayerId,
    source: ObjectId,
) -> Vec<crate::game_state::Target> {
    crate::game_loop::compute_legal_targets(game, &counter.target, controller, Some(source))
}

#[test]
fn named_counter_cards_keep_an_any_matching_target_relation() {
    let not = parse_oracle_card_definition("Not of This World");
    let escort = parse_oracle_card_definition("Diplomatic Escort");
    let siren = parse_oracle_card_definition("Siren Stormtamer");

    let not_counter = counter_from_program(not.spell_effect.as_ref().expect("Not spell effect"));
    let escort_counter = counter_from_program(&counter_ability(&escort).effects);
    let siren_counter = counter_from_program(&counter_ability(&siren).effects);

    for (name, counter) in [
        ("Not of This World", &not_counter),
        ("Diplomatic Escort", &escort_counter),
        ("Siren Stormtamer", &siren_counter),
    ] {
        let ChooseSpec::Object(filter) = counter.target.base() else {
            panic!("{name} should target a stack object: {:?}", counter.target);
        };
        assert_eq!(
            filter.any_of.len(),
            2,
            "{name} should target spells or abilities"
        );
        assert!(
            filter.any_of.iter().all(|branch| {
                branch.targets_only_player.is_none()
                    && branch.targets_only_object.is_none()
                    && (branch.targets_player.is_some() || branch.targets_object.is_some())
            }),
            "{name} must require at least one matching target, not require every target to match: {filter:#?}"
        );
    }

    let ChooseSpec::Object(siren_filter) = siren_counter.target.base() else {
        unreachable!();
    };
    assert!(
        siren_filter.any_of.iter().all(|branch| {
            branch.targets_player == Some(PlayerFilter::You)
                && branch.targets_object.is_some()
                && branch.targets_any_of
        }),
        "Siren must retain both 'you' and 'a creature you control': {siren_filter:#?}"
    );

    for (name, definition) in [
        ("Not of This World", &not),
        ("Diplomatic Escort", &escort),
        ("Siren Stormtamer", &siren),
    ] {
        let rendered = compiled_text_lines(definition).join("\n");
        assert!(
            !rendered.contains("targets only"),
            "{name} must not invent an all-targets restriction: {rendered}"
        );
    }
    assert!(
        compiled_text_lines(&siren)
            .join("\n")
            .contains("targets you or a creature you control"),
        "Siren's compiled text must keep its player target branch"
    );
}

#[test]
fn fixed_named_counter_cards_clear_the_strict_semantic_floor_truthfully() {
    for (name, oracle) in [
        (
            "Not of This World",
            "Counter target spell or ability that targets a permanent you control.\nThis spell costs {7} less to cast if it targets a spell or ability that targets a creature you control with power 7 or greater.",
        ),
        (
            "Diplomatic Escort",
            "{U}, {T}, Discard a card: Counter target spell or ability that targets a creature.",
        ),
        (
            "Siren Stormtamer",
            "Flying\n{U}, Sacrifice this creature: Counter target spell or ability that targets you or a creature you control.",
        ),
    ] {
        let definition = parse_oracle_card_definition(name);
        let compiled = compiled_text_lines(&definition);
        let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
            crate::semantic_compare::compare_card_semantics_scored(
                name,
                oracle,
                &compiled,
                Some(crate::semantic_compare::EmbeddingConfig {
                    dims: 384,
                    mismatch_threshold: 0.99,
                }),
            );
        println!("{name}: similarity={similarity:.4}, mismatch={mismatch}");
        assert!(
            similarity >= 0.99 && !mismatch,
            "{name} should clear the strict floor only with truthful target semantics; compiled={compiled:#?}, similarity={similarity}, mismatch={mismatch}"
        );
    }
}

#[test]
fn not_of_this_world_accepts_mixed_target_spells_and_abilities_in_scope() {
    let definition = parse_oracle_card_definition("Not of This World");
    let counter = counter_from_program(definition.spell_effect.as_ref().expect("spell effect"));
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&definition, alice, Zone::Hand);
    let owned_creature = game.create_object_from_definition(
        &simple_permanent("Alice Creature", CardType::Creature, Some(3)),
        alice,
        Zone::Battlefield,
    );
    let owned_noncreature = game.create_object_from_definition(
        &simple_permanent("Alice Artifact", CardType::Artifact, None),
        alice,
        Zone::Battlefield,
    );
    let opposing_permanent = game.create_object_from_definition(
        &simple_permanent("Bob Artifact", CardType::Artifact, None),
        bob,
        Zone::Battlefield,
    );

    let mixed_spell = push_targeting_spell(
        &mut game,
        bob,
        "Mixed Target Spell",
        vec![
            crate::game_state::Target::Object(owned_creature),
            crate::game_state::Target::Object(opposing_permanent),
        ],
    );
    let mixed_ability = push_targeting_ability(
        &mut game,
        bob,
        "Mixed Target Ability",
        vec![
            crate::game_state::Target::Object(owned_creature),
            crate::game_state::Target::Player(bob),
        ],
    );
    let owned_noncreature_spell = push_targeting_spell(
        &mut game,
        bob,
        "Targets Alice Artifact",
        vec![crate::game_state::Target::Object(owned_noncreature)],
    );
    let out_of_scope = push_targeting_spell(
        &mut game,
        bob,
        "Opponent Only Spell",
        vec![crate::game_state::Target::Object(opposing_permanent)],
    );
    let legal = legal_counter_targets(&game, &counter, alice, source);

    assert!(legal.contains(&crate::game_state::Target::Object(mixed_spell)));
    assert!(legal.contains(&crate::game_state::Target::Object(mixed_ability)));
    assert!(legal.contains(&crate::game_state::Target::Object(owned_noncreature_spell)));
    assert!(!legal.contains(&crate::game_state::Target::Object(out_of_scope)));
}

#[test]
fn not_of_this_world_counters_a_mixed_target_spell_on_resolution() {
    let definition = parse_oracle_card_definition("Not of This World");
    let counter = counter_from_program(definition.spell_effect.as_ref().expect("spell effect"));
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let owned = game.create_object_from_definition(
        &simple_permanent("Alice Permanent", CardType::Artifact, None),
        alice,
        Zone::Battlefield,
    );
    let opposing = game.create_object_from_definition(
        &simple_permanent("Bob Permanent", CardType::Artifact, None),
        bob,
        Zone::Battlefield,
    );
    let target_spell = push_targeting_spell(
        &mut game,
        bob,
        "Mixed Target Spell",
        vec![
            crate::game_state::Target::Object(owned),
            crate::game_state::Target::Object(opposing),
        ],
    );
    let target_stable = game.object(target_spell).expect("target spell").stable_id;
    let not = game.create_object_from_definition(&definition, alice, Zone::Stack);
    game.push_to_stack(
        crate::game_state::StackEntry::new(not, alice)
            .with_targets(vec![crate::game_state::Target::Object(target_spell)])
            .with_target_assignments(vec![crate::game_state::TargetAssignment {
                spec: counter.target.clone(),
                range: 0..1,
            }]),
    );

    crate::game_loop::resolve_stack_entry(&mut game).expect("Not of This World should resolve");
    let target_after = game
        .find_object_by_stable_id(target_stable)
        .expect("countered target remains tracked");
    assert_eq!(
        game.object(target_after).expect("countered target").zone,
        Zone::Graveyard
    );
}

#[test]
fn diplomatic_escort_and_siren_keep_spell_ability_and_mixed_target_scope() {
    let escort = parse_oracle_card_definition("Diplomatic Escort");
    let siren = parse_oracle_card_definition("Siren Stormtamer");
    let escort_counter = counter_from_program(&counter_ability(&escort).effects);
    let siren_counter = counter_from_program(&counter_ability(&siren).effects);
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let escort_source = game.create_object_from_definition(&escort, alice, Zone::Battlefield);
    let siren_source = game.create_object_from_definition(&siren, alice, Zone::Battlefield);
    let alice_creature = game.create_object_from_definition(
        &simple_permanent("Alice Creature", CardType::Creature, Some(1)),
        alice,
        Zone::Battlefield,
    );
    let bob_creature = game.create_object_from_definition(
        &simple_permanent("Bob Creature", CardType::Creature, Some(1)),
        bob,
        Zone::Battlefield,
    );
    let bob_artifact = game.create_object_from_definition(
        &simple_permanent("Bob Artifact", CardType::Artifact, None),
        bob,
        Zone::Battlefield,
    );

    let creature_plus_player_spell = push_targeting_spell(
        &mut game,
        bob,
        "Creature Plus Player Spell",
        vec![
            crate::game_state::Target::Object(bob_creature),
            crate::game_state::Target::Player(bob),
        ],
    );
    let creature_plus_artifact_ability = push_targeting_ability(
        &mut game,
        bob,
        "Creature Plus Artifact Ability",
        vec![
            crate::game_state::Target::Object(bob_creature),
            crate::game_state::Target::Object(bob_artifact),
        ],
    );
    let no_creature_spell = push_targeting_spell(
        &mut game,
        bob,
        "No Creature Spell",
        vec![crate::game_state::Target::Object(bob_artifact)],
    );
    let targets_alice_plus_other = push_targeting_spell(
        &mut game,
        bob,
        "Targets Alice Plus Other",
        vec![
            crate::game_state::Target::Player(alice),
            crate::game_state::Target::Object(bob_artifact),
        ],
    );
    let targets_alice_creature_plus_other = push_targeting_ability(
        &mut game,
        bob,
        "Targets Alice Creature Plus Other",
        vec![
            crate::game_state::Target::Object(alice_creature),
            crate::game_state::Target::Object(bob_artifact),
        ],
    );
    let targets_bob_creature_only = push_targeting_spell(
        &mut game,
        bob,
        "Targets Bob Creature",
        vec![crate::game_state::Target::Object(bob_creature)],
    );

    let escort_legal = legal_counter_targets(&game, &escort_counter, alice, escort_source);
    assert!(escort_legal.contains(&crate::game_state::Target::Object(
        creature_plus_player_spell
    )));
    assert!(escort_legal.contains(&crate::game_state::Target::Object(
        creature_plus_artifact_ability
    )));
    assert!(!escort_legal.contains(&crate::game_state::Target::Object(no_creature_spell)));

    let siren_legal = legal_counter_targets(&game, &siren_counter, alice, siren_source);
    assert!(siren_legal.contains(&crate::game_state::Target::Object(targets_alice_plus_other)));
    assert!(siren_legal.contains(&crate::game_state::Target::Object(
        targets_alice_creature_plus_other
    )));
    assert!(!siren_legal.contains(&crate::game_state::Target::Object(
        targets_bob_creature_only
    )));
}

#[test]
fn not_of_this_world_reduction_uses_the_chosen_targets_power_and_real_payment() {
    let mut definition = parse_oracle_card_definition("Not of This World");
    // The shared oracle regression helper intentionally supplies type line and
    // rules text only; add the printed mana cost for this payment scenario.
    definition.card.mana_cost = Some(crate::mana::ManaCost::from_symbols(vec![
        crate::mana::ManaSymbol::Generic(7),
    ]));
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let not = game.create_object_from_definition(&definition, alice, Zone::Hand);
    let strong = game.create_object_from_definition(
        &simple_permanent("Power Seven", CardType::Creature, Some(7)),
        alice,
        Zone::Battlefield,
    );
    let weak = game.create_object_from_definition(
        &simple_permanent("Power Six", CardType::Creature, Some(6)),
        alice,
        Zone::Battlefield,
    );
    let owned_noncreature = game.create_object_from_definition(
        &simple_permanent("Owned Artifact", CardType::Artifact, None),
        alice,
        Zone::Battlefield,
    );
    let strong_target_spell = push_targeting_spell(
        &mut game,
        bob,
        "Targets Power Seven",
        vec![crate::game_state::Target::Object(strong)],
    );
    let weak_target_spell = push_targeting_spell(
        &mut game,
        bob,
        "Targets Power Six",
        vec![crate::game_state::Target::Object(weak)],
    );
    let strong_target_ability = push_targeting_ability(
        &mut game,
        bob,
        "Ability Targets Power Seven",
        vec![crate::game_state::Target::Object(strong)],
    );
    let noncreature_target_spell = push_targeting_spell(
        &mut game,
        bob,
        "Targets Owned Artifact",
        vec![crate::game_state::Target::Object(owned_noncreature)],
    );
    let object = game.object(not).expect("Not exists");
    let base = definition
        .card
        .mana_cost
        .as_ref()
        .expect("Not has mana cost");
    let reduced = crate::decision::calculate_effective_mana_cost_with_chosen_targets(
        &game,
        alice,
        object,
        base,
        &[crate::game_state::Target::Object(strong_target_spell)],
    );
    let unreduced = crate::decision::calculate_effective_mana_cost_with_chosen_targets(
        &game,
        alice,
        object,
        base,
        &[crate::game_state::Target::Object(weak_target_spell)],
    );
    let reduced_for_ability = crate::decision::calculate_effective_mana_cost_with_chosen_targets(
        &game,
        alice,
        object,
        base,
        &[crate::game_state::Target::Object(strong_target_ability)],
    );
    let unreduced_for_noncreature =
        crate::decision::calculate_effective_mana_cost_with_chosen_targets(
            &game,
            alice,
            object,
            base,
            &[crate::game_state::Target::Object(noncreature_target_spell)],
        );
    assert_eq!(
        reduced.mana_value(),
        0,
        "power 7 target should remove all of {{7}}"
    );
    assert_eq!(
        reduced_for_ability.mana_value(),
        0,
        "an ability targeting the controlled power-7 creature should also remove all of {{7}}"
    );
    assert_eq!(
        unreduced.to_oracle(),
        "{7}",
        "power 6 target should keep the full cost"
    );
    assert_eq!(
        unreduced_for_noncreature.to_oracle(),
        "{7}",
        "a spell targeting a controlled noncreature permanent is counterable but gets no reduction"
    );

    assert!(
        game.player(alice)
            .expect("Alice")
            .mana_pool
            .can_pay(&reduced, 0)
    );
    assert!(
        !game
            .player(alice)
            .expect("Alice")
            .mana_pool
            .can_pay(&unreduced, 0)
    );
    game.player_mut(alice)
        .expect("Alice")
        .mana_pool
        .add(crate::mana::ManaSymbol::Colorless, 7);
    assert!(
        game.player(alice)
            .expect("Alice")
            .mana_pool
            .can_pay(&unreduced, 0)
    );
    assert!(
        game.player_mut(alice)
            .expect("Alice")
            .mana_pool
            .try_pay(&unreduced, 0),
        "seven mana should pay the unreduced cost"
    );
    assert_eq!(game.player(alice).expect("Alice").mana_pool.total(), 0);
}

#[test]
fn diplomatic_escort_pays_blue_tap_and_discard_then_counters_on_resolution() {
    let definition = parse_oracle_card_definition("Diplomatic Escort");
    let activated = counter_ability(&definition);
    let counter = counter_from_program(&activated.effects);
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    game.remove_summoning_sickness(source);
    let discard = game.create_object_from_definition(
        &simple_permanent("Discard Card", CardType::Artifact, None),
        alice,
        Zone::Hand,
    );
    let discard_stable = game.object(discard).expect("discard card").stable_id;
    let creature = game.create_object_from_definition(
        &simple_permanent("Target Creature", CardType::Creature, Some(2)),
        bob,
        Zone::Battlefield,
    );
    let other = game.create_object_from_definition(
        &simple_permanent("Other Target", CardType::Artifact, None),
        bob,
        Zone::Battlefield,
    );
    let target_spell = push_targeting_spell(
        &mut game,
        bob,
        "Mixed Target Spell",
        vec![
            crate::game_state::Target::Object(creature),
            crate::game_state::Target::Object(other),
        ],
    );
    let target_stable = game.object(target_spell).expect("target spell").stable_id;

    assert!(crate::cost::can_pay_cost(&game, source, alice, &activated.mana_cost).is_err());
    game.player_mut(alice)
        .expect("Alice")
        .mana_pool
        .add(crate::mana::ManaSymbol::Blue, 1);
    crate::cost::can_pay_cost(&game, source, alice, &activated.mana_cost)
        .expect("blue mana, untapped source, and a hand card should pay the activation");
    crate::special_actions::pay_total_cost_with_choice(
        &mut game,
        alice,
        source,
        &activated.mana_cost,
        crate::costs::PaymentReason::ActivateAbility,
        &mut crate::decision::SelectFirstDecisionMaker,
    )
    .expect("Diplomatic Escort activation cost should be paid");
    assert_eq!(game.player(alice).expect("Alice").mana_pool.total(), 0);
    assert!(
        game.is_tapped(source),
        "tap cost should tap Diplomatic Escort"
    );
    let discarded_after = game
        .find_object_by_stable_id(discard_stable)
        .expect("discard remains tracked across its zone change");
    assert_eq!(
        game.object(discarded_after).expect("discarded card").zone,
        Zone::Graveyard
    );

    game.push_to_stack(
        crate::game_state::StackEntry::ability(source, alice, activated.effects.clone())
            .with_targets(vec![crate::game_state::Target::Object(target_spell)])
            .with_target_assignments(vec![crate::game_state::TargetAssignment {
                spec: counter.target.clone(),
                range: 0..1,
            }]),
    );
    crate::game_loop::resolve_stack_entry(&mut game)
        .expect("Diplomatic Escort's counter ability should resolve");
    let target_after = game
        .find_object_by_stable_id(target_stable)
        .expect("countered spell remains tracked");
    assert_eq!(
        game.object(target_after).expect("countered spell").zone,
        Zone::Graveyard
    );

    let target_ability = push_targeting_ability(
        &mut game,
        bob,
        "Mixed Target Ability",
        vec![
            crate::game_state::Target::Object(creature),
            crate::game_state::Target::Object(other),
        ],
    );
    game.push_to_stack(
        crate::game_state::StackEntry::ability(source, alice, activated.effects.clone())
            .with_targets(vec![crate::game_state::Target::Object(target_ability)])
            .with_target_assignments(vec![crate::game_state::TargetAssignment {
                spec: counter.target,
                range: 0..1,
            }]),
    );
    crate::game_loop::resolve_stack_entry(&mut game)
        .expect("Diplomatic Escort should counter an ability too");
    assert!(
        !game
            .stack
            .iter()
            .any(|entry| entry.object_id == target_ability),
        "the targeted ability should disappear from the stack"
    );
    assert_eq!(
        game.object(target_ability)
            .expect("ability source remains")
            .zone,
        Zone::Battlefield
    );
}
