#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::{oracle_text_by_name, parse_oracle_card_definition};
use super::*;

fn permanent(name: &str, card_type: CardType) -> CardDefinition {
    let builder = CardDefinitionBuilder::new(CardId::new(), name).card_types(vec![card_type]);
    if card_type == CardType::Creature {
        builder.power_toughness(PowerToughness::fixed(5, 5)).build()
    } else {
        builder.build()
    }
}

fn planeswalker(name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Planeswalker])
        .loyalty(5)
        .build()
}

fn current_zone(game: &crate::GameState, stable: StableId) -> Zone {
    let id = game
        .find_object_by_stable_id(stable)
        .expect("fixture object should retain stable identity");
    game.object(id).expect("fixture object should exist").zone
}

fn target_player_filter() -> PlayerFilter {
    PlayerFilter::Target(Box::new(PlayerFilter::Any))
}

#[test]
fn structural_collapse_uses_one_player_target_for_both_choices_and_damage() {
    let definition = parse_oracle_card_definition("Structural Collapse");
    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        oracle_text_by_name()["Structural Collapse"]
    );

    let program = definition
        .spell_effect
        .as_ref()
        .expect("Structural Collapse should retain a spell program");
    let [sacrifice_segment, damage_segment] = program.segments.as_slice() else {
        panic!("expected sacrifice and damage source sentences: {program:#?}");
    };
    let [sequence_root] = sacrifice_segment.default_effects.as_slice() else {
        panic!("expected one coordinated sacrifice root: {program:#?}");
    };
    let sequence = sequence_root
        .downcast_ref::<crate::effects::SequenceEffect>()
        .expect("the two chosen sacrifices should remain coordinated");
    let target_only = sequence
        .effects
        .iter()
        .filter_map(|effect| effect.downcast_ref::<crate::effects::TargetOnlyEffect>())
        .collect::<Vec<_>>();
    assert_eq!(target_only.len(), 1, "{sequence:#?}");
    assert_eq!(target_only[0].target, ChooseSpec::target_player());

    let choices = sequence
        .effects
        .iter()
        .filter_map(|effect| effect.downcast_ref::<crate::effects::ChooseObjectsEffect>())
        .collect::<Vec<_>>();
    assert_eq!(choices.len(), 2, "{sequence:#?}");
    for (choice, card_type) in choices.iter().zip([CardType::Artifact, CardType::Land]) {
        assert_eq!(choice.count, ChoiceCount::exactly(1));
        assert_eq!(choice.chooser, target_player_filter());
        assert_eq!(choice.filter.zone, Some(Zone::Battlefield));
        assert_eq!(choice.filter.controller, Some(target_player_filter()));
        assert_eq!(choice.filter.card_types, [card_type]);
        assert!(sequence.effects.iter().any(|effect| {
            effect
                .downcast_ref::<crate::effects::zones::SacrificePlayerEffect>()
                .is_some_and(|sacrifice| {
                    sacrifice.player == target_player_filter()
                        && sacrifice.count.unhinted() == &Value::Fixed(1)
                        && sacrifice.filter == ObjectFilter::tagged(choice.tag.clone())
                })
        }));
    }
    let [damage_root] = damage_segment.default_effects.as_slice() else {
        panic!("expected one damage effect: {program:#?}");
    };
    let mut damage = Vec::new();
    collect_damage(damage_root, false, &mut damage);
    assert_eq!(damage.len(), 1, "{damage_root:#?}");
    assert_eq!(damage[0].0.amount.unhinted(), &Value::Fixed(2));
    assert_eq!(
        damage[0].0.target,
        ChooseSpec::Player(target_player_filter())
    );

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
    let bob_artifact = game.create_object_from_definition(
        &permanent("Bob Artifact", CardType::Artifact),
        bob,
        Zone::Battlefield,
    );
    let bob_land = game.create_object_from_definition(
        &permanent("Bob Land", CardType::Land),
        bob,
        Zone::Battlefield,
    );
    let alice_artifact = game.create_object_from_definition(
        &permanent("Alice Artifact", CardType::Artifact),
        alice,
        Zone::Battlefield,
    );
    let bob_artifact_stable = game
        .object(bob_artifact)
        .expect("artifact exists")
        .stable_id;
    let bob_land_stable = game.object(bob_land).expect("land exists").stable_id;
    let alice_artifact_stable = game
        .object(alice_artifact)
        .expect("artifact exists")
        .stable_id;

    let requirements = crate::game_loop::extract_target_requirements_from_program_with_modes(
        &game,
        program,
        alice,
        Some(source),
        None,
    );
    assert_eq!(requirements.len(), 1, "{requirements:#?}");
    let selected = vec![crate::game_state::Target::Player(bob)];
    let assignments =
        super::shard_17::target_assignments_for_requirements(&requirements, &selected);
    let resolved_targets = selected
        .iter()
        .map(|target| match target {
            crate::game_state::Target::Object(id) => crate::effects::ResolvedTarget::Object(*id),
            crate::game_state::Target::Player(id) => crate::effects::ResolvedTarget::Player(*id),
        })
        .collect();
    let mut decisions = crate::decision::SelectFirstDecisionMaker;
    let mut context = crate::effects::ExecutionContext::new(source, alice, &mut decisions)
        .with_targets(resolved_targets)
        .with_target_assignments(assignments.clone());
    context.snapshot_targets(&game);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut context,
        alice,
        source,
        program,
        None,
        &assignments,
    )
    .expect("Structural Collapse should resolve with one shared player target");

    assert_eq!(current_zone(&game, bob_artifact_stable), Zone::Graveyard);
    assert_eq!(current_zone(&game, bob_land_stable), Zone::Graveyard);
    assert_eq!(
        current_zone(&game, alice_artifact_stable),
        Zone::Battlefield
    );
    assert_eq!(game.life_total(bob), 18);
}

fn collect_damage(
    effect: &Effect,
    execute_with_source: bool,
    out: &mut Vec<(crate::effects::DealDamageEffect, bool)>,
) {
    if let Some(with_source) = effect.downcast_ref::<crate::effects::ExecuteWithSourceEffect>() {
        collect_damage(&with_source.effect, true, out);
        return;
    }
    if let Some(damage) = effect.downcast_ref::<crate::effects::DealDamageEffect>() {
        out.push((damage.clone(), execute_with_source));
        return;
    }
    effect.visit_child_effects(&mut |child| {
        collect_damage(child, execute_with_source, out);
    });
}

fn activated_damage(
    ability: &crate::ability::ActivatedAbility,
) -> Vec<(crate::effects::DealDamageEffect, bool)> {
    let mut damage = Vec::new();
    for root in ability.effects.all_effects() {
        collect_damage(root, false, &mut damage);
    }
    damage
}

fn resolve_soul_activation(
    definition: &CardDefinition,
    ability: &crate::ability::ActivatedAbility,
    source_zone: Zone,
    first_target_is_planeswalker: bool,
) -> (crate::GameState, ObjectId, ObjectId) {
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(definition, alice, source_zone);
    let recipient = if first_target_is_planeswalker {
        game.create_object_from_definition(&planeswalker("Bob Walker"), bob, Zone::Battlefield)
    } else {
        ObjectId::from_raw(0)
    };
    let creature = game.create_object_from_definition(
        &permanent("Bob Creature", CardType::Creature),
        bob,
        Zone::Battlefield,
    );
    let selected = if first_target_is_planeswalker {
        vec![
            crate::game_state::Target::Object(recipient),
            crate::game_state::Target::Object(creature),
        ]
    } else {
        vec![
            crate::game_state::Target::Player(bob),
            crate::game_state::Target::Object(creature),
        ]
    };
    let requirements = crate::game_loop::extract_target_requirements_from_program_with_modes(
        &game,
        &ability.effects,
        alice,
        Some(source),
        None,
    );
    assert_eq!(requirements.len(), 2, "{requirements:#?}");
    let assignments =
        super::shard_17::target_assignments_for_requirements(&requirements, &selected);
    let resolved_targets = selected
        .iter()
        .map(|target| match target {
            crate::game_state::Target::Object(id) => crate::effects::ResolvedTarget::Object(*id),
            crate::game_state::Target::Player(id) => crate::effects::ResolvedTarget::Player(*id),
        })
        .collect();
    let mut decisions = crate::decision::SelectFirstDecisionMaker;
    let mut context = crate::effects::ExecutionContext::new(source, alice, &mut decisions)
        .with_targets(resolved_targets)
        .with_target_assignments(assignments.clone());
    context.snapshot_targets(&game);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut context,
        alice,
        source,
        &ability.effects,
        None,
        &assignments,
    )
    .expect("Soul damage pair should resolve");
    (game, recipient, creature)
}

#[test]
fn soul_of_shandalar_keeps_linked_optional_creature_damage_in_both_zones() {
    let definition = parse_oracle_card_definition("Soul of Shandalar");
    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        oracle_text_by_name()["Soul of Shandalar"]
    );
    let activated = definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => {
                Some((ability.functional_zones.clone(), activated))
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(activated.len(), 2);
    assert_eq!(activated[0].0, [Zone::Battlefield]);
    assert_eq!(activated[1].0, [Zone::Graveyard]);

    for (index, (_, ability)) in activated.iter().enumerate() {
        let damage = activated_damage(ability);
        assert_eq!(damage.len(), 2, "{ability:#?}");
        assert_eq!(damage[0].0.amount.unhinted(), &Value::Fixed(3));
        assert_eq!(
            damage[0].0.target,
            ChooseSpec::PlayerOrPlaneswalker(PlayerFilter::Any)
        );
        assert_eq!(damage[1].0.amount.unhinted(), &Value::Fixed(3));
        assert!(damage[1].0.target.is_target());
        assert_eq!(damage[1].0.target.count(), ChoiceCount::up_to(1));
        let ChooseSpec::Object(filter) = damage[1].0.target.base() else {
            panic!(
                "expected a typed optional creature target: {:#?}",
                damage[1].0
            );
        };
        assert_eq!(filter.card_types, [CardType::Creature]);
        assert_eq!(
            filter.controller,
            Some(PlayerFilter::TargetPlayerOrControllerOfTarget)
        );
        assert_eq!(damage[0].1, index == 1);
        assert_eq!(damage[1].1, index == 1);
    }

    let (battlefield_game, _, battlefield_creature) =
        resolve_soul_activation(&definition, activated[0].1, Zone::Battlefield, false);
    assert_eq!(battlefield_game.life_total(PlayerId::from_index(1)), 17);
    assert_eq!(battlefield_game.damage_on(battlefield_creature), 3);

    let (graveyard_game, planeswalker, graveyard_creature) =
        resolve_soul_activation(&definition, activated[1].1, Zone::Graveyard, true);
    assert_eq!(
        graveyard_game.counter_count(planeswalker, crate::CounterType::Loyalty),
        2
    );
    assert_eq!(graveyard_game.damage_on(graveyard_creature), 3);
}

fn graveyard_type_card(card_type: CardType, index: usize) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), format!("Graveyard Type {index}"))
        .card_types(vec![card_type])
        .build()
}

fn winter_hand_sizes(type_count: usize) -> (i32, i32, i32) {
    let definition = parse_oracle_card_definition("Winter, Misanthropic Guide");
    let mut game = crate::GameState::new(
        vec!["Alice".to_string(), "Bob".to_string(), "Cara".to_string()],
        20,
    );
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let cara = PlayerId::from_index(2);
    game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    for (index, card_type) in [
        CardType::Artifact,
        CardType::Creature,
        CardType::Enchantment,
        CardType::Land,
        CardType::Instant,
    ]
    .into_iter()
    .take(type_count)
    .enumerate()
    {
        game.create_object_from_definition(
            &graveyard_type_card(card_type, index),
            alice,
            Zone::Graveyard,
        );
    }
    game.refresh_continuous_state();
    (
        game.player(alice).expect("Alice exists").max_hand_size,
        game.player(bob).expect("Bob exists").max_hand_size,
        game.player(cara).expect("Cara exists").max_hand_size,
    )
}

#[test]
fn winter_keeps_the_typed_seven_minus_rule_and_multiplayer_threshold() {
    let definition = parse_oracle_card_definition("Winter, Misanthropic Guide");
    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        oracle_text_by_name()["Winter, Misanthropic Guide"]
    );
    let winter_rule = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => {
                static_ability.compiled_model().filter(|model| {
                    model.id
                        == Some(StaticAbilityId::MaximumHandSizeSevenMinusYourGraveyardCardTypes)
                })
            }
            _ => None,
        })
        .expect("Winter should retain its typed maximum-hand-size rule");
    assert!(matches!(
        &winter_rule.payload,
        ironsmith_core::StaticAbilityPayload::Conditional { ability, condition }
            if matches!(
                &ability.payload,
                ironsmith_core::StaticAbilityPayload::MaximumHandSizeSevenMinusYourGraveyardCardTypes {
                    player: PlayerFilter::Opponent,
                    min_card_types: 0,
                }
            ) && matches!(
                condition,
                Condition::PlayerHasCardTypesInGraveyardOrMore {
                    player: PlayerFilter::You,
                    count: 4,
                }
            )
    ));

    assert_eq!(winter_hand_sizes(3), (7, 7, 7));
    assert_eq!(winter_hand_sizes(4), (7, 3, 3));
    assert_eq!(winter_hand_sizes(5), (7, 2, 2));
}
