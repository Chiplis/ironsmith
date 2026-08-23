#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::{oracle_text_by_name, parse_oracle_card_definition};
use super::*;

fn three_player_game() -> crate::GameState {
    crate::GameState::new(
        vec![
            "Alice".to_string(),
            "Bob".to_string(),
            "Charlie".to_string(),
        ],
        20,
    )
}

fn creature(name: &str, mana_value: u8) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(
            mana_value,
        )]]))
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 3))
        .build()
}

fn artifact(name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Artifact])
        .build()
}

fn assert_high_semantic_score(name: &str, definition: &CardDefinition) {
    let compiled = unprocessed_compiled_lines(definition);
    let (_oracle_coverage, _compiled_coverage, similarity, delta, mismatch) =
        crate::semantic_compare::compare_semantics_scored(
            &oracle_text_by_name()[name],
            &compiled,
            crate::semantic_compare::report_embedding_config(),
        );
    assert!(
        !mismatch && similarity >= 0.99,
        "{name} should remain in the high-score cohort: similarity={similarity}, delta={delta}, compiled={compiled:#?}"
    );
}

fn tagged_child(effect: &Effect) -> Option<&Effect> {
    effect
        .downcast_ref::<crate::effects::TaggedEffect>()
        .map(|tagged| tagged.effect.as_ref())
        .or_else(|| {
            effect
                .downcast_ref::<crate::effects::WithIdEffect>()
                .map(|with_id| with_id.effect.as_ref())
        })
}

fn find_goad(effect: &Effect) -> Option<&crate::effects::GoadEffect> {
    effect
        .downcast_ref::<crate::effects::GoadEffect>()
        .or_else(|| tagged_child(effect).and_then(find_goad))
}

fn find_phase_out(effect: &Effect) -> Option<&crate::effects::PhaseOutEffect> {
    effect
        .downcast_ref::<crate::effects::PhaseOutEffect>()
        .or_else(|| tagged_child(effect).and_then(find_phase_out))
}

fn find_destroy(effect: &Effect) -> Option<&crate::effects::DestroyEffect> {
    effect
        .downcast_ref::<crate::effects::DestroyEffect>()
        .or_else(|| tagged_child(effect).and_then(find_destroy))
}

fn program_goad(
    program: &crate::resolution::ResolutionProgram,
) -> Option<&crate::effects::GoadEffect> {
    program
        .flattened_default_effects()
        .iter()
        .find_map(find_goad)
}

fn program_phase_out(
    program: &crate::resolution::ResolutionProgram,
) -> Option<&crate::effects::PhaseOutEffect> {
    program
        .flattened_default_effects()
        .iter()
        .find_map(find_phase_out)
}

fn program_destroy(
    program: &crate::resolution::ResolutionProgram,
) -> Option<&crate::effects::DestroyEffect> {
    program
        .flattened_default_effects()
        .iter()
        .find_map(find_destroy)
}

fn resolve_with_targets(
    game: &mut crate::GameState,
    source: ObjectId,
    controller: PlayerId,
    program: &crate::resolution::ResolutionProgram,
    selected: Vec<crate::game_state::Target>,
    resolved: Vec<crate::effects::ResolvedTarget>,
) -> Vec<crate::decision::TargetRequirement> {
    let requirements = crate::game_loop::extract_target_requirements_from_program_with_modes(
        game,
        program,
        controller,
        Some(source),
        None,
    );
    let assignments =
        super::shard_17::target_assignments_for_requirements(&requirements, &selected);
    let mut decisions = crate::decision::SelectFirstDecisionMaker;
    let mut context = crate::effects::ExecutionContext::new(source, controller, &mut decisions)
        .with_targets(resolved)
        .with_target_assignments(assignments.clone());
    context.snapshot_targets(game);
    crate::game_loop::execute_resolution_program(
        game,
        &mut context,
        controller,
        source,
        program,
        None,
        &assignments,
    )
    .expect("the parser-backed targeted program should resolve");
    requirements
}

fn current_zone(game: &crate::GameState, stable: StableId) -> Zone {
    let current = game
        .find_object_by_stable_id(stable)
        .expect("the fixture should retain stable identity");
    game.object(current).expect("the fixture should exist").zone
}

#[test]
fn geode_rager_goads_every_creature_of_only_the_target_player() {
    let definition = parse_oracle_card_definition("Geode Rager");
    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) if program_goad(&triggered.effects).is_some() => {
                Some(triggered)
            }
            _ => None,
        })
        .expect("Geode Rager should retain its Landfall goad trigger");
    let goad = program_goad(&triggered.effects).unwrap();
    let ChooseSpec::All(filter) = goad.target.base() else {
        panic!("authored `each creature` must be a complete set: {goad:#?}");
    };
    assert!(matches!(filter.controller, Some(PlayerFilter::Target(_))));

    let mut game = three_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let bob_a =
        game.create_object_from_definition(&creature("Bob Creature A", 2), bob, Zone::Battlefield);
    let bob_b =
        game.create_object_from_definition(&creature("Bob Creature B", 3), bob, Zone::Battlefield);
    let charlie_decoy = game.create_object_from_definition(
        &creature("Charlie Decoy", 2),
        charlie,
        Zone::Battlefield,
    );
    let alice_decoy =
        game.create_object_from_definition(&creature("Alice Decoy", 2), alice, Zone::Battlefield);

    let requirements = resolve_with_targets(
        &mut game,
        source,
        alice,
        &triggered.effects,
        vec![crate::game_state::Target::Player(bob)],
        vec![crate::effects::ResolvedTarget::Player(bob)],
    );
    assert_eq!(requirements.len(), 1);
    assert!(game.is_goaded(bob_a));
    assert!(game.is_goaded(bob_b));
    assert!(!game.is_goaded(charlie_decoy));
    assert!(!game.is_goaded(alice_decoy));
    assert_high_semantic_score("Geode Rager", &definition);
}

#[test]
fn droning_bureaucrats_restricts_every_creature_with_the_announced_mana_value() {
    let definition = parse_oracle_card_definition("Droning Bureaucrats");
    let activated = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => activated
                .effects
                .flattened_default_effects()
                .iter()
                .any(|effect| {
                    matches!(
                        effect.downcast_ref::<crate::effects::CantEffect>(),
                        Some(crate::effects::CantEffect {
                            restriction: crate::effect::Restriction::AttackOrBlock(_),
                            ..
                        })
                    )
                })
                .then_some(activated),
            _ => None,
        })
        .expect("Droning Bureaucrats should retain its X restriction");
    let cant = activated
        .effects
        .flattened_default_effects()
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::CantEffect>())
        .expect("the activation should register a restriction");
    let crate::effect::Restriction::AttackOrBlock(filter) = &cant.restriction else {
        panic!("the restriction must cover both combat roles: {cant:#?}");
    };
    assert!(filter.mana_value.is_some());

    let mut game = three_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let first_match =
        game.create_object_from_definition(&creature("MV Three A", 3), bob, Zone::Battlefield);
    let second_match =
        game.create_object_from_definition(&creature("MV Three B", 3), charlie, Zone::Battlefield);
    let decoy =
        game.create_object_from_definition(&creature("MV Two Decoy", 2), bob, Zone::Battlefield);
    let mut decisions = crate::decision::AutoPassDecisionMaker;
    let mut context =
        crate::effects::ExecutionContext::new(source, alice, &mut decisions).with_x(3);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut context,
        alice,
        source,
        &activated.effects,
        None,
        &[],
    )
    .expect("Droning Bureaucrats should resolve for X=3");
    game.update_cant_effects();

    for matching in [first_match, second_match] {
        assert!(!game.can_attack(matching));
        assert!(!game.can_block(matching));
    }
    assert!(game.can_attack(decoy));
    assert!(game.can_block(decoy));
    assert_high_semantic_score("Droning Bureaucrats", &definition);
}

#[test]
fn teferi_timeless_voyager_phases_every_creature_of_only_the_target_opponent() {
    let definition = parse_oracle_card_definition("Teferi, Timeless Voyager");
    let activated = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated)
                if program_phase_out(&activated.effects).is_some() =>
            {
                Some(activated)
            }
            _ => None,
        })
        .expect("Teferi should retain his phase-out ultimate");
    let phase_out = program_phase_out(&activated.effects).unwrap();
    let ChooseSpec::All(filter) = phase_out.spec.base() else {
        panic!("authored `each creature` must phase the complete set: {phase_out:#?}");
    };
    assert!(matches!(filter.controller, Some(PlayerFilter::Target(_))));

    let mut game = three_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let bob_a =
        game.create_object_from_definition(&creature("Bob Creature A", 2), bob, Zone::Battlefield);
    let bob_b =
        game.create_object_from_definition(&creature("Bob Creature B", 3), bob, Zone::Battlefield);
    let charlie_decoy = game.create_object_from_definition(
        &creature("Charlie Decoy", 2),
        charlie,
        Zone::Battlefield,
    );
    let alice_decoy =
        game.create_object_from_definition(&creature("Alice Decoy", 2), alice, Zone::Battlefield);

    let requirements = resolve_with_targets(
        &mut game,
        source,
        alice,
        &activated.effects,
        vec![crate::game_state::Target::Player(bob)],
        vec![crate::effects::ResolvedTarget::Player(bob)],
    );
    assert_eq!(requirements.len(), 1);
    game.update_cant_effects();
    for phased in [bob_a, bob_b] {
        assert!(game.is_phased_out(phased));
        assert!(
            !game.can_phase_in(phased),
            "the persistent phase-in restriction should contain both phased objects: {:#?}",
            game.effect_store.restriction_effects
        );
    }
    for decoy in [charlie_decoy, alice_decoy] {
        assert!(!game.is_phased_out(decoy));
        assert!(game.can_phase_in(decoy));
    }
    assert_high_semantic_score("Teferi, Timeless Voyager", &definition);
}

#[test]
fn tetzimoc_destroys_only_opposing_creatures_with_prey_counters() {
    let definition = parse_oracle_card_definition("Tetzimoc, Primal Death");
    let activated = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("Tetzimoc should retain its hand activation");
    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) if program_destroy(&triggered.effects).is_some() => {
                Some(triggered)
            }
            _ => None,
        })
        .expect("Tetzimoc should retain its ETB destroy trigger");
    let destroy = program_destroy(&triggered.effects).unwrap();
    let ChooseSpec::All(filter) = destroy.spec.base() else {
        panic!("Tetzimoc must destroy the complete matching set: {destroy:#?}");
    };
    assert_eq!(filter.card_types, [CardType::Creature]);
    assert_eq!(filter.controller, Some(PlayerFilter::Opponent));
    assert!(filter.with_counter.is_some());

    let mut game = three_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);
    let source = game.create_object_from_definition(&definition, alice, Zone::Hand);
    let bob_prey =
        game.create_object_from_definition(&creature("Bob Prey", 2), bob, Zone::Battlefield);
    let charlie_prey = game.create_object_from_definition(
        &creature("Charlie Prey", 2),
        charlie,
        Zone::Battlefield,
    );
    let own_prey =
        game.create_object_from_definition(&creature("Own Prey", 2), alice, Zone::Battlefield);
    let bob_unmarked =
        game.create_object_from_definition(&creature("Bob Unmarked", 2), bob, Zone::Battlefield);
    let bob_artifact =
        game.create_object_from_definition(&artifact("Bob Prey Artifact"), bob, Zone::Battlefield);

    resolve_with_targets(
        &mut game,
        source,
        alice,
        &activated.effects,
        vec![crate::game_state::Target::Object(bob_prey)],
        vec![crate::effects::ResolvedTarget::Object(bob_prey)],
    );
    assert_eq!(game.counter_count(bob_prey, CounterType::Named("prey")), 1);
    for permanent in [charlie_prey, own_prey, bob_artifact] {
        game.add_counters(permanent, CounterType::Named("prey"), 1);
    }

    let tracked = [bob_prey, charlie_prey, own_prey, bob_unmarked, bob_artifact]
        .map(|object| game.object(object).expect("fixture exists").stable_id);
    let source = game
        .move_object(
            source,
            Zone::Battlefield,
            crate::events::cause::EventCause::effect(),
        )
        .expect("Tetzimoc should enter the battlefield");
    let mut decisions = crate::decision::AutoPassDecisionMaker;
    let mut context = crate::effects::ExecutionContext::new(source, alice, &mut decisions);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut context,
        alice,
        source,
        &triggered.effects,
        None,
        &[],
    )
    .expect("Tetzimoc's ETB trigger should resolve");

    assert_eq!(current_zone(&game, tracked[0]), Zone::Graveyard);
    assert_eq!(current_zone(&game, tracked[1]), Zone::Graveyard);
    assert_eq!(current_zone(&game, tracked[2]), Zone::Battlefield);
    assert_eq!(current_zone(&game, tracked[3]), Zone::Battlefield);
    assert_eq!(current_zone(&game, tracked[4]), Zone::Battlefield);
    assert_high_semantic_score("Tetzimoc, Primal Death", &definition);
}
