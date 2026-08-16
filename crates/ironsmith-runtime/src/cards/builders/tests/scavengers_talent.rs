#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;
use crate::decision::{
    AutoPassDecisionMaker, DecisionMaker, LegalAction, SelectFirstDecisionMaker,
};
use crate::effects::{ExecutionContext, SacrificeEffect};
use crate::mana::ManaSymbol;
use crate::snapshot::ObjectSnapshot;

const COMPILED_TEXT: &str = "(Gain the next level as a sorcery to add its ability.)\nWhenever one or more creatures you control die, create a Food token. This ability triggers only once each turn.\n{1}{B}: Level 2\nWhenever you sacrifice a permanent, target player mills two cards.\n{2}{B}: Level 3\nAt the beginning of your end step, you may sacrifice three other nonland permanents. If you do, you return a creature card from your graveyard to the battlefield with a finality counter on it.";

struct TargetBob {
    bob: PlayerId,
}

impl DecisionMaker for TargetBob {
    fn decide_targets(
        &mut self,
        _game: &crate::GameState,
        ctx: &crate::decisions::context::TargetsContext,
    ) -> Vec<crate::game_state::Target> {
        ctx.requirements
            .iter()
            .filter_map(|requirement| {
                requirement
                    .legal_targets
                    .iter()
                    .find_map(|target| match target {
                        crate::game_state::Target::Player(player) if *player == self.bob => {
                            Some(*target)
                        }
                        _ => None,
                    })
            })
            .take(1)
            .collect()
    }
}

fn creature(name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build()
}

fn artifact(name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Artifact])
        .build()
}

fn land(name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Land])
        .build()
}

fn fill_library(game: &mut crate::GameState, player: PlayerId, count: usize) {
    for index in 0..count {
        let card = CardDefinitionBuilder::new(CardId::new(), format!("Library {index}"))
            .card_types(vec![CardType::Sorcery])
            .build();
        game.create_object_from_definition(&card, player, Zone::Library);
    }
}

fn sacrifice_specific(
    game: &mut crate::GameState,
    source: ObjectId,
    player: PlayerId,
    permanent: ObjectId,
) {
    let mut ctx = ExecutionContext::new_default(source, player);
    let outcome = SacrificeEffect::you(ObjectFilter::specific(permanent), 1)
        .execute(game, &mut ctx)
        .expect("the chosen permanent should be sacrificed");
    game.effect_store
        .pending_trigger_events
        .extend(outcome.events);
}

fn source_trigger_entries(
    game: &mut crate::GameState,
    source: ObjectId,
) -> Vec<crate::triggers::TriggeredAbilityEntry> {
    let mut pending = crate::triggers::TriggerQueue::new();
    crate::game_loop::drain_pending_trigger_events(game, &mut pending);
    pending
        .entries
        .into_iter()
        .filter(|entry| entry.source == source)
        .collect()
}

fn resolve_entries(
    game: &mut crate::GameState,
    entries: Vec<crate::triggers::TriggeredAbilityEntry>,
    decisions: &mut dyn DecisionMaker,
) {
    let mut queue = crate::triggers::TriggerQueue::new();
    for entry in entries {
        queue.add(entry);
    }
    if queue.entries.is_empty() {
        return;
    }
    crate::game_loop::put_triggers_on_stack_with_dm(game, &mut queue, decisions)
        .expect("Scavenger's Talent triggers should go on the stack");
    while !game.stack_is_empty() {
        crate::game_loop::resolve_stack_entry_with(game, decisions)
            .expect("Scavenger's Talent trigger should resolve");
    }
}

fn food_count(game: &crate::GameState, controller: PlayerId) -> usize {
    game.battlefield
        .iter()
        .copied()
        .filter(|id| game.current_controller(*id) == Some(controller))
        .filter(|id| game.calculated_subtypes(*id).contains(&Subtype::Food))
        .count()
}

fn class_ability(
    definition: &CardDefinition,
    level: u32,
) -> (usize, &crate::ability::ActivatedAbility) {
    definition
        .abilities
        .iter()
        .enumerate()
        .find_map(|(index, ability)| match &ability.kind {
            AbilityKind::Activated(activated)
                if activated.additional_restrictions.iter().any(|restriction| {
                    restriction == &format!("__ironsmith_class_level:{level}")
                }) =>
            {
                Some((index, activated))
            }
            _ => None,
        })
        .expect("Scavenger's Talent should have the requested class activation")
}

fn can_activate(game: &crate::GameState, source: ObjectId, player: PlayerId, index: usize) -> bool {
    crate::decision::compute_legal_actions(game, player)
        .iter()
        .any(|action| matches!(action, LegalAction::ActivateAbility { source: id, ability_index } if *id == source && *ability_index == index))
}

fn pay_and_resolve_class_ability(
    game: &mut crate::GameState,
    source: ObjectId,
    player: PlayerId,
    activated: &crate::ability::ActivatedAbility,
) {
    let snapshot = ObjectSnapshot::from_object(
        game.object(source).expect("Scavenger's Talent exists"),
        game,
    );
    let mut decisions = SelectFirstDecisionMaker;
    let mut ctx =
        ExecutionContext::new(source, player, &mut decisions).with_source_snapshot(snapshot);
    crate::special_actions::pay_total_cost_with_choice_in_context(
        game,
        player,
        source,
        &activated.mana_cost,
        crate::costs::PaymentReason::ActivateAbility,
        &mut ctx,
    )
    .expect("the class level cost should be payable");
    crate::game_loop::execute_resolution_program(
        game,
        &mut ctx,
        player,
        source,
        &activated.effects,
        None,
        &[],
    )
    .expect("the class level ability should resolve");
}

#[test]
fn scavengers_talent_named_definition_keeps_level_gates_and_all_three_behaviors() {
    let definition = parse_oracle_card_definition("Scavenger's Talent");
    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        COMPILED_TEXT
    );
    let debug = format!("{:#?}", definition.abilities);
    assert!(debug.contains("MaxTimesEachTurn"), "{debug}");
    assert!(debug.contains("PlayerSacrificesTrigger"), "{debug}");
    assert!(debug.contains("BeginningOfEndStepTrigger"), "{debug}");
    assert!(debug.contains("Finality"), "{debug}");
    assert!(debug.contains("__ironsmith_class_level:2"), "{debug}");
    assert!(debug.contains("__ironsmith_class_level:3"), "{debug}");
}

#[test]
fn scavengers_talent_class_activations_are_paid_in_order_and_then_locked() {
    let definition = parse_oracle_card_definition("Scavenger's Talent");
    let (level_two_index, level_two) = class_ability(&definition, 2);
    let (level_three_index, level_three) = class_ability(&definition, 3);
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = crate::game_state::Phase::FirstMain;
    game.turn.step = None;
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    game.player_mut(alice)
        .expect("Alice")
        .mana_pool
        .add(ManaSymbol::Black, 2);
    game.player_mut(alice)
        .expect("Alice")
        .mana_pool
        .add(ManaSymbol::Colorless, 3);

    assert!(can_activate(&game, source, alice, level_two_index));
    assert!(!can_activate(&game, source, alice, level_three_index));
    pay_and_resolve_class_ability(&mut game, source, alice, level_two);
    assert_eq!(
        game.counter_count(source, crate::object::CounterType::Level),
        1
    );
    assert_eq!(game.player(alice).expect("Alice").mana_pool.total(), 3);
    assert!(!can_activate(&game, source, alice, level_two_index));
    assert!(can_activate(&game, source, alice, level_three_index));

    pay_and_resolve_class_ability(&mut game, source, alice, level_three);
    assert_eq!(
        game.counter_count(source, crate::object::CounterType::Level),
        2
    );
    assert_eq!(game.player(alice).expect("Alice").mana_pool.total(), 0);
    assert!(!can_activate(&game, source, alice, level_two_index));
    assert!(!can_activate(&game, source, alice, level_three_index));
}

#[test]
fn scavengers_talent_food_trigger_is_controller_only_and_once_each_turn() {
    let definition = parse_oracle_card_definition("Scavenger's Talent");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);

    let bob_creature =
        game.create_object_from_definition(&creature("Bob Victim"), bob, Zone::Battlefield);
    game.move_object_by_effect(bob_creature, Zone::Graveyard)
        .expect("Bob creature dies");
    assert!(source_trigger_entries(&mut game, source).is_empty());

    for (index, expected) in [(0, 1), (1, 1)] {
        let victim = game.create_object_from_definition(
            &creature(&format!("Alice Victim {index}")),
            alice,
            Zone::Battlefield,
        );
        game.move_object_by_effect(victim, Zone::Graveyard)
            .expect("Alice creature dies");
        let entries = source_trigger_entries(&mut game, source);
        if index == 0 {
            assert_eq!(entries.len(), 1);
            resolve_entries(&mut game, entries, &mut SelectFirstDecisionMaker);
        } else {
            assert!(entries.is_empty());
        }
        assert_eq!(food_count(&game, alice), expected);
    }

    game.next_turn();
    let next_turn_victim =
        game.create_object_from_definition(&creature("Next Turn Victim"), alice, Zone::Battlefield);
    game.move_object_by_effect(next_turn_victim, Zone::Graveyard)
        .expect("next-turn creature dies");
    let entries = source_trigger_entries(&mut game, source);
    assert_eq!(entries.len(), 1);
    resolve_entries(&mut game, entries, &mut SelectFirstDecisionMaker);
    assert_eq!(food_count(&game, alice), 2);
}

#[test]
fn scavengers_talent_level_two_mills_only_after_its_controller_sacrifices() {
    let definition = parse_oracle_card_definition("Scavenger's Talent");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    fill_library(&mut game, alice, 3);
    fill_library(&mut game, bob, 3);

    let before_level =
        game.create_object_from_definition(&artifact("Before Level"), alice, Zone::Battlefield);
    sacrifice_specific(&mut game, source, alice, before_level);
    assert!(source_trigger_entries(&mut game, source).is_empty());

    game.add_counters(source, crate::object::CounterType::Level, 1);
    let opponent_permanent =
        game.create_object_from_definition(&artifact("Bob Sacrifice"), bob, Zone::Battlefield);
    sacrifice_specific(&mut game, source, bob, opponent_permanent);
    assert!(
        source_trigger_entries(&mut game, source).is_empty(),
        "level two must ignore an opponent's sacrifice"
    );
    let after_level =
        game.create_object_from_definition(&artifact("After Level"), alice, Zone::Battlefield);
    sacrifice_specific(&mut game, source, alice, after_level);
    let entries = source_trigger_entries(&mut game, source);
    assert_eq!(entries.len(), 1);
    resolve_entries(&mut game, entries, &mut TargetBob { bob });
    assert_eq!(
        game.player(bob).expect("Bob").graveyard.len(),
        3,
        "Bob's earlier sacrificed artifact plus exactly two milled cards should be in the graveyard"
    );
    assert_eq!(game.player(bob).expect("Bob").library.len(), 1);
    assert_eq!(game.player(alice).expect("Alice").library.len(), 3);
}

#[test]
fn scavengers_talent_level_three_is_optional_and_sacrifices_three_other_nonlands_for_a_final_return()
 {
    let definition = parse_oracle_card_definition("Scavenger's Talent");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    game.add_counters(source, crate::object::CounterType::Level, 2);
    let land =
        game.create_object_from_definition(&land("Protected Land"), alice, Zone::Battlefield);
    let sacrifices = (0..3)
        .map(|index| {
            game.create_object_from_definition(
                &artifact(&format!("Fodder {index}")),
                alice,
                Zone::Battlefield,
            )
        })
        .collect::<Vec<_>>();
    let grave_creature =
        game.create_object_from_definition(&creature("Return Me"), alice, Zone::Graveyard);
    let grave_stable = game
        .object(grave_creature)
        .expect("grave creature")
        .stable_id;
    let event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::phase::BeginningOfEndStepEvent::new(alice),
        crate::provenance::ProvNodeId::default(),
    );

    let matching = crate::triggers::check_triggers(&game, &event)
        .into_iter()
        .filter(|entry| entry.source == source)
        .collect::<Vec<_>>();
    assert_eq!(matching.len(), 1);
    resolve_entries(&mut game, matching, &mut AutoPassDecisionMaker);
    assert!(sacrifices.iter().all(|id| game.battlefield.contains(id)));
    assert_eq!(
        game.object(grave_creature).expect("declined return").zone,
        Zone::Graveyard
    );

    let matching = crate::triggers::check_triggers(&game, &event)
        .into_iter()
        .filter(|entry| entry.source == source)
        .collect::<Vec<_>>();
    assert_eq!(matching.len(), 1);
    resolve_entries(&mut game, matching, &mut SelectFirstDecisionMaker);
    assert!(
        game.battlefield.contains(&source),
        "the 'other' filter must preserve the Class"
    );
    assert!(
        game.battlefield.contains(&land),
        "the nonland filter must preserve the land"
    );
    assert!(sacrifices.iter().all(|id| !game.battlefield.contains(id)));
    let returned = game
        .find_object_by_stable_id(grave_stable)
        .expect("returned creature remains tracked");
    assert_eq!(
        game.object(returned).expect("returned creature").zone,
        Zone::Battlefield
    );
    assert_eq!(
        game.counter_count(returned, crate::object::CounterType::Finality),
        1
    );
}
