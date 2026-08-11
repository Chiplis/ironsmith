#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;
use crate::decision::{DecisionMaker, SelectFirstDecisionMaker};
use crate::game_state::Target;
use crate::object::CounterType;

fn creature(name: &str, power: i32, toughness: i32) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(power, toughness))
        .build()
}

fn permanent(name: &str, card_type: CardType) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![card_type])
        .build()
}

fn land(name: &str, subtype: Subtype) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Land])
        .subtypes(vec![subtype])
        .build()
}

fn basic_forest(name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .supertypes(vec![Supertype::Basic])
        .card_types(vec![CardType::Land])
        .subtypes(vec![Subtype::Forest])
        .build()
}

fn named_count(game: &crate::GameState, zone: Zone, name: &str) -> usize {
    game.objects_in_zone(zone)
        .into_iter()
        .filter_map(|id| game.object(id))
        .filter(|object| object.name == name)
        .count()
}

fn token_count(game: &crate::GameState, controller: PlayerId, name: &str) -> usize {
    game.objects_in_zone(Zone::Battlefield)
        .into_iter()
        .filter_map(|id| game.object(id))
        .filter(|object| {
            object.kind == crate::object::ObjectKind::Token
                && object.name == name
                && game.controller_of(object) == controller
        })
        .count()
}

fn fill_graveyard(
    game: &mut crate::GameState,
    player: PlayerId,
    count: usize,
    card_type: CardType,
) {
    for index in 0..count {
        let filler = permanent(&format!("Graveyard Filler {index}"), card_type);
        game.create_object_from_definition(&filler, player, Zone::Graveyard);
    }
}

fn fill_library(game: &mut crate::GameState, player: PlayerId, count: usize) {
    for index in 0..count {
        let filler = permanent(&format!("Library Filler {index}"), CardType::Sorcery);
        game.create_object_from_definition(&filler, player, Zone::Library);
    }
}

fn resolve_spell(
    game: &mut crate::GameState,
    definition: &CardDefinition,
    controller: PlayerId,
    targets: Vec<Target>,
    paid: Option<crate::cost::OptionalCostsPaid>,
) {
    let spell = game.create_object_from_definition(definition, controller, Zone::Stack);
    let mut entry = crate::game_state::StackEntry::new(spell, controller).with_targets(targets);
    if let Some(paid) = paid {
        game.object_mut(spell)
            .expect("spell should exist on the stack")
            .optional_costs_paid = paid.clone();
        entry = entry.with_optional_costs_paid(paid);
    }
    game.push_to_stack(entry);
    let mut decisions = SelectFirstDecisionMaker;
    crate::game_loop::resolve_stack_entry_with(game, &mut decisions)
        .expect("named spell should resolve");
}

fn triggered_ability(definition: &CardDefinition) -> &TriggeredAbility {
    definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("card should have a triggered ability")
}

fn resolve_trigger_direct(
    game: &mut crate::GameState,
    definition: &CardDefinition,
    source: ObjectId,
    controller: PlayerId,
    targets: Vec<Target>,
) {
    let triggered = triggered_ability(definition);
    let mut entry =
        crate::game_state::StackEntry::ability(source, controller, triggered.effects.clone())
            .with_targets(targets)
            .with_trigger_identity(crate::triggers::compute_trigger_identity(triggered));
    if let Some(condition) = triggered.intervening_if.clone() {
        entry = entry.with_intervening_if(condition);
    }
    game.push_to_stack(entry);
    let mut decisions = SelectFirstDecisionMaker;
    crate::game_loop::resolve_stack_entry_with(game, &mut decisions)
        .expect("named trigger should resolve");
}

#[derive(Clone, Copy)]
struct PreferTarget {
    target: Target,
}

impl DecisionMaker for PreferTarget {
    fn decide_boolean(
        &mut self,
        _game: &crate::GameState,
        _ctx: &crate::decisions::context::BooleanContext,
    ) -> bool {
        true
    }

    fn decide_objects(
        &mut self,
        _game: &crate::GameState,
        ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        let legal = ctx
            .candidates
            .iter()
            .filter(|candidate| candidate.legal)
            .map(|candidate| candidate.id)
            .collect::<Vec<_>>();
        legal.into_iter().take(ctx.max.unwrap_or(1)).collect()
    }

    fn decide_targets(
        &mut self,
        _game: &crate::GameState,
        ctx: &crate::decisions::context::TargetsContext,
    ) -> Vec<Target> {
        ctx.requirements
            .iter()
            .filter_map(|requirement| {
                requirement
                    .legal_targets
                    .iter()
                    .copied()
                    .find(|target| *target == self.target)
                    .or_else(|| requirement.legal_targets.first().copied())
            })
            .collect()
    }
}

fn resolve_landfall(
    game: &mut crate::GameState,
    source: ObjectId,
    controller: PlayerId,
    land_definition: &CardDefinition,
    preferred_target: Target,
) {
    let in_hand = game.create_object_from_definition(land_definition, controller, Zone::Hand);
    game.move_object_by_effect(in_hand, Zone::Battlefield)
        .expect("land should enter the battlefield");
    let mut queue = crate::triggers::TriggerQueue::new();
    crate::game_loop::drain_pending_trigger_events(game, &mut queue);
    queue.entries.retain(|entry| entry.source == source);
    assert_eq!(
        queue.entries.len(),
        1,
        "expected one named landfall trigger"
    );
    let mut decisions = PreferTarget {
        target: preferred_target,
    };
    crate::game_loop::put_triggers_on_stack_with_dm(game, &mut queue, &mut decisions)
        .expect("landfall trigger should go on the stack");
    if game
        .stack
        .last()
        .is_some_and(|entry| !entry.targets.is_empty())
    {
        assert!(
            game.stack
                .last()
                .is_some_and(|entry| entry.targets.contains(&preferred_target)),
            "the named landfall trigger should retain its selected target"
        );
    }
    crate::game_loop::resolve_stack_entry_with(game, &mut decisions)
        .expect("landfall trigger should resolve");
}

#[test]
fn aerith_last_ancient_returns_to_hand_below_seven_life_and_to_battlefield_at_seven() {
    for (life_gained, expected_zone) in [(1, Zone::Hand), (7, Zone::Battlefield)] {
        let definition = parse_oracle_card_definition("Aerith, Last Ancient");
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let target_name = format!("Aerith Raise Target {life_gained}");
        let target = game.create_object_from_definition(
            &creature(&target_name, 2, 2),
            alice,
            Zone::Graveyard,
        );
        let event = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::LifeGainEvent::new(alice, life_gained),
            crate::provenance::ProvNodeId::default(),
        );
        game.turn_store
            .turn_history
            .record_event(&event, None, None);

        resolve_trigger_direct(
            &mut game,
            &definition,
            source,
            alice,
            vec![Target::Object(target)],
        );
        assert_eq!(
            named_count(&game, expected_zone, &target_name),
            1,
            "Aerith's branch left counts graveyard/hand/battlefield = {}/{}/{}",
            named_count(&game, Zone::Graveyard, &target_name),
            named_count(&game, Zone::Hand, &target_name),
            named_count(&game, Zone::Battlefield, &target_name),
        );
        assert_eq!(
            named_count(
                &game,
                if expected_zone == Zone::Hand {
                    Zone::Battlefield
                } else {
                    Zone::Hand
                },
                &target_name,
            ),
            0,
            "Aerith's replacement branch must replace rather than supplement the default"
        );
    }
}

#[test]
fn akoum_hellkite_deals_one_for_other_lands_and_two_for_a_mountain() {
    for (subtype, expected_damage) in [(Subtype::Island, 1), (Subtype::Mountain, 2)] {
        let definition = parse_oracle_card_definition("Akoum Hellkite");
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        resolve_landfall(
            &mut game,
            source,
            alice,
            &land("Akoum Triggering Land", subtype),
            Target::Player(bob),
        );
        assert_eq!(
            game.player(bob).expect("Bob should exist").life,
            20 - expected_damage,
            "Akoum must inspect the triggering land, not its damage target"
        );
    }
}

#[test]
fn emeria_shepherd_returns_to_hand_for_other_lands_and_to_battlefield_for_a_plains() {
    for (subtype, expected_zone) in [
        (Subtype::Island, Zone::Hand),
        (Subtype::Plains, Zone::Battlefield),
    ] {
        let definition = parse_oracle_card_definition("Emeria Shepherd");
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let target_name = format!("Emeria Target {subtype:?}");
        let target = game.create_object_from_definition(
            &permanent(&target_name, CardType::Artifact),
            alice,
            Zone::Graveyard,
        );
        resolve_landfall(
            &mut game,
            source,
            alice,
            &land("Emeria Triggering Land", subtype),
            Target::Object(target),
        );
        assert_eq!(
            named_count(&game, expected_zone, &target_name),
            1,
            "Emeria {subtype:?} branch left counts graveyard/hand/battlefield = {}/{}/{}",
            named_count(&game, Zone::Graveyard, &target_name),
            named_count(&game, Zone::Hand, &target_name),
            named_count(&game, Zone::Battlefield, &target_name),
        );
        assert_eq!(
            named_count(
                &game,
                if expected_zone == Zone::Hand {
                    Zone::Battlefield
                } else {
                    Zone::Hand
                },
                &target_name,
            ),
            0,
            "Emeria must inspect the triggering land and choose exactly one destination"
        );
    }
}

#[test]
fn grizzly_fate_creates_two_bears_below_threshold_and_four_at_threshold() {
    for (graveyard_cards, expected) in [(6, 2), (7, 4)] {
        let definition = parse_oracle_card_definition("Grizzly Fate");
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        fill_graveyard(&mut game, alice, graveyard_cards, CardType::Sorcery);
        resolve_spell(&mut game, &definition, alice, vec![], None);
        assert_eq!(token_count(&game, alice, "Bear"), expected);
    }
}

#[test]
fn guul_draz_overseer_pumps_other_creatures_once_by_the_triggering_land_kind() {
    for (subtype, expected_power) in [(Subtype::Island, 3), (Subtype::Swamp, 4)] {
        let mut definition = parse_oracle_card_definition("Guul Draz Overseer");
        // The shared oracle-text parser helper intentionally supplies type-line
        // metadata but not printed P/T. Give the source its real 3/4 body so
        // this assertion can distinguish `other` from an accidentally broad
        // pump without mistaking missing fixture metadata for engine behavior.
        definition.card.power_toughness = Some(PowerToughness::fixed(3, 4));
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let ally = game.create_object_from_definition(
            &creature("Overseer Ally", 2, 2),
            alice,
            Zone::Battlefield,
        );
        let opponent = game.create_object_from_definition(
            &creature("Overseer Opponent", 2, 2),
            bob,
            Zone::Battlefield,
        );
        resolve_landfall(
            &mut game,
            source,
            alice,
            &land("Overseer Triggering Land", subtype),
            Target::Player(alice),
        );
        assert_eq!(game.current_power(ally), Some(expected_power));
        assert_eq!(
            game.current_power(source),
            Some(3),
            "other must exclude Overseer; battlefield copies={}, object={:?}",
            named_count(&game, Zone::Battlefield, "Guul Draz Overseer"),
            game.object(source),
        );
        assert_eq!(game.current_power(opponent), Some(2));
    }
}

#[test]
fn mirran_mettle_applies_two_without_metalcraft_and_four_with_metalcraft() {
    for (artifacts, expected_power) in [(2, 4), (3, 6)] {
        let definition = parse_oracle_card_definition("Mirran Mettle");
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        for index in 0..artifacts {
            game.create_object_from_definition(
                &permanent(&format!("Metalcraft Artifact {index}"), CardType::Artifact),
                alice,
                Zone::Battlefield,
            );
        }
        let target = game.create_object_from_definition(
            &creature("Mettle Target", 2, 2),
            alice,
            Zone::Battlefield,
        );
        resolve_spell(
            &mut game,
            &definition,
            alice,
            vec![Target::Object(target)],
            None,
        );
        assert_eq!(game.current_power(target), Some(expected_power));
        assert_eq!(game.current_toughness(target), Some(expected_power));
    }
}

#[test]
fn nissas_pilgrimage_searches_two_normally_and_three_with_spell_mastery() {
    for (spell_cards, expected_found) in [(1, 2), (2, 3)] {
        let definition = parse_oracle_card_definition("Nissa's Pilgrimage");
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        fill_graveyard(&mut game, alice, spell_cards, CardType::Instant);
        for index in 0..3 {
            game.create_object_from_definition(
                &basic_forest(&format!("Pilgrimage Forest {index}")),
                alice,
                Zone::Library,
            );
        }
        resolve_spell(&mut game, &definition, alice, vec![], None);
        let in_hand = (0..3)
            .map(|index| named_count(&game, Zone::Hand, &format!("Pilgrimage Forest {index}")))
            .sum::<usize>();
        let on_battlefield = (0..3)
            .map(|index| {
                named_count(
                    &game,
                    Zone::Battlefield,
                    &format!("Pilgrimage Forest {index}"),
                )
            })
            .sum::<usize>();
        assert_eq!(
            in_hand + on_battlefield,
            expected_found,
            "Nissa with {spell_cards} spells found hand/battlefield/library = {in_hand}/{on_battlefield}/{}",
            (0..3)
                .map(|index| named_count(
                    &game,
                    Zone::Library,
                    &format!("Pilgrimage Forest {index}")
                ))
                .sum::<usize>(),
        );
        assert_eq!(
            on_battlefield, 1,
            "exactly one found Forest should enter tapped"
        );
    }
}

#[test]
fn oran_rief_hydra_gets_one_counter_for_other_lands_and_two_for_a_forest() {
    for (subtype, expected) in [(Subtype::Island, 1), (Subtype::Forest, 2)] {
        let definition = parse_oracle_card_definition("Oran-Rief Hydra");
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        resolve_landfall(
            &mut game,
            source,
            alice,
            &land("Hydra Triggering Land", subtype),
            Target::Player(alice),
        );
        assert_eq!(
            game.counter_count(source, CounterType::PlusOnePlusOne),
            expected
        );
    }
}

#[test]
fn porcelain_zealot_gives_one_to_an_ordinary_target_and_two_to_a_toxic_target() {
    for toxic in [false, true] {
        let definition = parse_oracle_card_definition("Porcelain Zealot");
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let target_definition = if toxic {
            CardDefinitionBuilder::new(CardId::new(), "Toxic Zealot Target")
                .card_types(vec![CardType::Creature])
                .power_toughness(PowerToughness::fixed(2, 2))
                .parse_text("Toxic 1")
                .expect("toxic creature should parse")
        } else {
            creature("Ordinary Zealot Target", 2, 2)
        };
        let target =
            game.create_object_from_definition(&target_definition, alice, Zone::Battlefield);
        resolve_trigger_direct(
            &mut game,
            &definition,
            source,
            alice,
            vec![Target::Object(target)],
        );
        assert_eq!(game.current_power(target), Some(if toxic { 4 } else { 3 }));
        assert_eq!(
            game.current_toughness(target),
            Some(if toxic { 4 } else { 3 })
        );
    }
}

#[test]
fn predators_howl_creates_one_wolf_without_morbid_and_three_with_morbid() {
    for creature_died in [false, true] {
        let definition = parse_oracle_card_definition("Predator's Howl");
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        if creature_died {
            let doomed = game.create_object_from_definition(
                &creature("Morbid Fodder", 1, 1),
                alice,
                Zone::Battlefield,
            );
            game.move_object_by_effect(doomed, Zone::Graveyard)
                .expect("morbid fodder should die");
        }
        resolve_spell(&mut game, &definition, alice, vec![], None);
        assert_eq!(
            token_count(&game, alice, "Wolf"),
            if creature_died { 3 } else { 1 }
        );
    }
}

#[test]
fn reclaim_the_wastes_searches_one_normally_and_two_when_kicked() {
    let rendered = parse_oracle_card_definition("Reclaim the Wastes");
    assert_eq!(
        canonical_compiled_lines(&rendered).join("\n"),
        "Kicker {3}\nSearch your library for a basic land card, reveal it, put it into your hand, then shuffle. If this spell was kicked, search your library for two basic land cards instead of one."
    );
    for kicked in [false, true] {
        let definition = parse_oracle_card_definition("Reclaim the Wastes");
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        for index in 0..2 {
            game.create_object_from_definition(
                &basic_forest(&format!("Reclaim Forest {index}")),
                alice,
                Zone::Library,
            );
        }
        let paid = kicked.then(|| {
            let mut paid = crate::cost::OptionalCostsPaid::from_costs(&definition.optional_costs);
            paid.pay(0);
            paid
        });
        resolve_spell(&mut game, &definition, alice, vec![], paid);
        let found = (0..2)
            .map(|index| named_count(&game, Zone::Hand, &format!("Reclaim Forest {index}")))
            .sum::<usize>();
        assert_eq!(
            found,
            if kicked { 2 } else { 1 },
            "Reclaim kicked={kicked} left matching lands hand/library/battlefield = {found}/{}/{}",
            (0..2)
                .map(|index| named_count(&game, Zone::Library, &format!("Reclaim Forest {index}")))
                .sum::<usize>(),
            (0..2)
                .map(|index| named_count(
                    &game,
                    Zone::Battlefield,
                    &format!("Reclaim Forest {index}")
                ))
                .sum::<usize>(),
        );
    }
}

#[test]
fn stitch_together_returns_to_hand_below_threshold_and_to_battlefield_at_threshold() {
    for threshold in [false, true] {
        let definition = parse_oracle_card_definition("Stitch Together");
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        fill_graveyard(
            &mut game,
            alice,
            if threshold { 6 } else { 5 },
            CardType::Sorcery,
        );
        let target_name = if threshold {
            "Threshold Stitch Target"
        } else {
            "Ordinary Stitch Target"
        };
        let target = game.create_object_from_definition(
            &creature(target_name, 2, 2),
            alice,
            Zone::Graveyard,
        );
        resolve_spell(
            &mut game,
            &definition,
            alice,
            vec![Target::Object(target)],
            None,
        );
        let expected_zone = if threshold {
            Zone::Battlefield
        } else {
            Zone::Hand
        };
        assert_eq!(named_count(&game, expected_zone, target_name), 1);
        assert_eq!(
            named_count(
                &game,
                if threshold {
                    Zone::Hand
                } else {
                    Zone::Battlefield
                },
                target_name
            ),
            0,
        );
    }
}

#[test]
fn tallyman_of_nurgle_draws_and_loses_one_for_one_death_and_seven_for_seven() {
    for deaths in [1, 7] {
        let definition = parse_oracle_card_definition("Tallyman of Nurgle");
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        fill_library(&mut game, alice, 8);
        for index in 0..deaths {
            let doomed = game.create_object_from_definition(
                &creature(&format!("Tallyman Fodder {index}"), 1, 1),
                alice,
                Zone::Battlefield,
            );
            game.move_object_by_effect(doomed, Zone::Graveyard)
                .expect("Tallyman fodder should die");
        }
        resolve_trigger_direct(&mut game, &definition, source, alice, vec![]);
        assert_eq!(
            game.player(alice).expect("Alice should exist").hand.len(),
            deaths
        );
        assert_eq!(
            game.player(alice).expect("Alice should exist").life,
            20 - deaths as i32
        );
    }
}

fn resolve_tower_worker_with_partners(partners: &[CardDefinition]) -> u32 {
    let definition = parse_oracle_card_definition("Tower Worker");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    for partner in partners {
        game.create_object_from_definition(partner, alice, Zone::Battlefield);
    }
    let activated = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("Tower Worker should have a mana ability");
    game.push_to_stack(crate::game_state::StackEntry::ability(
        source,
        alice,
        activated.effects.clone(),
    ));
    crate::game_loop::resolve_stack_entry(&mut game).expect("Tower Worker ability should resolve");
    game.player(alice)
        .expect("Alice should exist")
        .mana_pool
        .total()
}

#[test]
fn tower_worker_requires_both_exact_named_workers_and_never_adds_base_plus_upgrade() {
    let mine_worker = creature("Mine Worker", 2, 2);
    let power_plant_worker = creature("Power Plant Worker", 2, 2);
    let generic_plant = CardDefinitionBuilder::new(CardId::new(), "Ordinary Plant")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Plant])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    assert_eq!(resolve_tower_worker_with_partners(&[]), 1);
    assert_eq!(
        resolve_tower_worker_with_partners(&[mine_worker.clone(), power_plant_worker]),
        3,
        "the two exact named partners should enable the replacement"
    );
    assert_eq!(
        resolve_tower_worker_with_partners(&[mine_worker, generic_plant]),
        1,
        "a generic Plant must not impersonate Power Plant Worker"
    );
}

#[test]
fn toxic_stench_shrinks_below_threshold_and_destroys_at_threshold() {
    for threshold in [false, true] {
        let definition = parse_oracle_card_definition("Toxic Stench");
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        fill_graveyard(
            &mut game,
            alice,
            if threshold { 7 } else { 6 },
            CardType::Sorcery,
        );
        let target_name = if threshold {
            "Threshold Stench Target"
        } else {
            "Ordinary Stench Target"
        };
        let target = game.create_object_from_definition(
            &creature(target_name, 3, 3),
            bob,
            Zone::Battlefield,
        );
        resolve_spell(
            &mut game,
            &definition,
            alice,
            vec![Target::Object(target)],
            None,
        );
        if threshold {
            assert_eq!(named_count(&game, Zone::Graveyard, target_name), 1);
        } else {
            assert_eq!(named_count(&game, Zone::Battlefield, target_name), 1);
            assert_eq!(game.current_toughness(target), Some(2));
        }
    }
}

#[test]
fn tragic_fall_uses_minus_three_with_a_hand_and_minus_thirteen_when_hellbent() {
    for hellbent in [false, true] {
        let definition = parse_oracle_card_definition("Tragic Fall");
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        if !hellbent {
            game.create_object_from_definition(
                &permanent("Hellbent Blocker", CardType::Sorcery),
                alice,
                Zone::Hand,
            );
        }
        let target = game.create_object_from_definition(
            &creature("Tragic Fall Target", 20, 20),
            bob,
            Zone::Battlefield,
        );
        resolve_spell(
            &mut game,
            &definition,
            alice,
            vec![Target::Object(target)],
            None,
        );
        assert_eq!(
            game.current_toughness(target),
            Some(if hellbent { 7 } else { 17 })
        );
    }
}

#[test]
fn tragic_slip_uses_minus_one_without_morbid_and_minus_thirteen_with_morbid() {
    for morbid in [false, true] {
        let definition = parse_oracle_card_definition("Tragic Slip");
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        if morbid {
            let doomed = game.create_object_from_definition(
                &creature("Slip Morbid Fodder", 1, 1),
                alice,
                Zone::Battlefield,
            );
            game.move_object_by_effect(doomed, Zone::Graveyard)
                .expect("morbid fodder should die");
        }
        let target = game.create_object_from_definition(
            &creature("Tragic Slip Target", 20, 20),
            bob,
            Zone::Battlefield,
        );
        resolve_spell(
            &mut game,
            &definition,
            alice,
            vec![Target::Object(target)],
            None,
        );
        assert_eq!(
            game.current_toughness(target),
            Some(if morbid { 7 } else { 19 })
        );
    }
}

#[test]
fn tragic_trajectory_uses_minus_two_without_void_and_minus_ten_after_a_nonland_leaves() {
    for void in [false, true] {
        let definition = parse_oracle_card_definition("Tragic Trajectory");
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let departing = if void {
            permanent("Departing Void Artifact", CardType::Artifact)
        } else {
            land("Departing Nonvoid Land", Subtype::Island)
        };
        let departing = game.create_object_from_definition(&departing, alice, Zone::Battlefield);
        game.move_object_by_effect(departing, Zone::Graveyard)
            .expect("void setup permanent should leave");
        let target = game.create_object_from_definition(
            &creature("Tragic Trajectory Target", 20, 20),
            bob,
            Zone::Battlefield,
        );
        resolve_spell(
            &mut game,
            &definition,
            alice,
            vec![Target::Object(target)],
            None,
        );
        assert_eq!(
            game.current_toughness(target),
            Some(if void { 10 } else { 18 })
        );
    }
}

#[test]
fn join_the_dead_counts_four_permanent_cards_not_four_arbitrary_graveyard_cards() {
    for (permanents, nonpermanents, expected_toughness) in [(3, 0, 15), (4, 0, 10), (0, 4, 15)] {
        let definition = parse_oracle_card_definition("Join the Dead");
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        fill_graveyard(&mut game, alice, permanents, CardType::Artifact);
        fill_graveyard(&mut game, alice, nonpermanents, CardType::Instant);
        let target = game.create_object_from_definition(
            &creature("Join the Dead Target", 20, 20),
            bob,
            Zone::Battlefield,
        );
        resolve_spell(
            &mut game,
            &definition,
            alice,
            vec![Target::Object(target)],
            None,
        );
        assert_eq!(
            game.current_toughness(target),
            Some(expected_toughness),
            "descend 4 must count permanent cards only"
        );
    }
}

#[test]
fn take_the_fall_replaces_the_pump_when_you_control_an_outlaw_and_always_draws_once() {
    for controls_outlaw in [false, true] {
        let definition = parse_oracle_card_definition("Take the Fall");
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        fill_library(&mut game, alice, 1);
        if controls_outlaw {
            let outlaw = CardDefinitionBuilder::new(CardId::new(), "Friendly Outlaw")
                .card_types(vec![CardType::Creature])
                .subtypes(vec![Subtype::Rogue])
                .power_toughness(PowerToughness::fixed(2, 2))
                .build();
            game.create_object_from_definition(&outlaw, alice, Zone::Battlefield);
        } else {
            let opposing_outlaw = CardDefinitionBuilder::new(CardId::new(), "Opposing Outlaw")
                .card_types(vec![CardType::Creature])
                .subtypes(vec![Subtype::Rogue])
                .power_toughness(PowerToughness::fixed(2, 2))
                .build();
            game.create_object_from_definition(&opposing_outlaw, bob, Zone::Battlefield);
        }
        let target = game.create_object_from_definition(
            &creature("Take the Fall Target", 10, 10),
            bob,
            Zone::Battlefield,
        );
        resolve_spell(
            &mut game,
            &definition,
            alice,
            vec![Target::Object(target)],
            None,
        );
        assert_eq!(
            game.current_power(target),
            Some(if controls_outlaw { 6 } else { 9 }),
            "the outlaw branch must replace -1/-0 with -4/-0"
        );
        assert_eq!(
            game.player(alice).expect("Alice should exist").hand.len(),
            1,
            "the common draw must happen exactly once in either branch"
        );
    }
}

#[test]
fn consult_the_star_charts_takes_one_or_kicked_two_and_puts_every_remainder_on_bottom() {
    for kicked in [false, true] {
        let definition = parse_oracle_card_definition("Consult the Star Charts");
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        for index in 0..3 {
            game.create_object_from_definition(
                &land(&format!("Consult Land {index}"), Subtype::Island),
                alice,
                Zone::Battlefield,
            );
        }
        let existing_bottom = game.create_object_from_definition(
            &permanent("Consult Existing Bottom", CardType::Sorcery),
            alice,
            Zone::Library,
        );
        for index in 0..3 {
            game.create_object_from_definition(
                &permanent(&format!("Consult Candidate {index}"), CardType::Sorcery),
                alice,
                Zone::Library,
            );
        }
        let paid = kicked.then(|| {
            let mut paid = crate::cost::OptionalCostsPaid::from_costs(&definition.optional_costs);
            paid.pay(0);
            paid
        });
        resolve_spell(&mut game, &definition, alice, vec![], paid);

        let cards_taken = (0..3)
            .map(|index| named_count(&game, Zone::Hand, &format!("Consult Candidate {index}")))
            .sum::<usize>();
        assert_eq!(cards_taken, if kicked { 2 } else { 1 });
        let candidates_left = game
            .player(alice)
            .expect("Alice should exist")
            .library
            .iter()
            .filter_map(|id| game.object(*id))
            .filter(|object| object.name.starts_with("Consult Candidate"))
            .count();
        assert_eq!(candidates_left, if kicked { 1 } else { 2 });
        assert_eq!(
            game.player(alice)
                .expect("Alice should exist")
                .library
                .last(),
            Some(&existing_bottom),
            "all unchosen looked-at cards should move below the old library bottom"
        );
    }
}
