//! Complete static predicate tails retain their stat change, scope and all secondary bodies.
//! Authored only; campaign compilation and execution remain deferred.
use ironsmith::ability::AbilityKind;
use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::cards::CardDefinition;
use ironsmith::color::ColorSet;
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::effects::{AttachToEffect, EffectContext, EffectExecutor};
use ironsmith::game_loop::{
    extract_target_requirements_from_program_with_modes, put_triggers_on_stack_with_dm,
    resolve_stack_entry_with,
};
use ironsmith::game_state::{StackEntry, TargetAssignment};
use ironsmith::object::CounterType;
use ironsmith::resolution::ResolutionProgram;
use ironsmith::static_abilities::StaticAbility;
use ironsmith::target::ChooseSpec;
use ironsmith::triggers::{TriggerEvent, TriggerQueue, check_triggers};
use ironsmith::{
    CardId, CardType, GameState, ObjectId, PlayerId, Subtype, Supertype, Target, Zone,
};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::compile_to_artifact;
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;
const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);
fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/complete_anthem_tails.json.fixture"
    ))
    .unwrap()
}
fn definitions(name: &str) -> [CardDefinition; 2] {
    let row = fixtures()
        .into_iter()
        .find(|row| row["name"] == name)
        .unwrap();
    let mut text = format!(
        "Mana cost: {}\nType: {}\n",
        row["mana_cost"].as_str().unwrap(),
        row["type_line"].as_str().unwrap()
    );
    if let (Some(p), Some(t)) = (row["power"].as_str(), row["toughness"].as_str()) {
        text.push_str(&format!("Power/Toughness: {p}/{t}\n"));
    }
    if let Some(n) = row["loyalty"].as_str() {
        text.push_str(&format!("Loyalty: {n}\n"));
    }
    text.push_str(row["oracle_text"].as_str().unwrap());
    let (result, loss) =
        ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, &text, false));
    let (artifact, direct) = result.unwrap_or_else(|error| panic!("{name}: {error}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    restored.validate().unwrap();
    assert_eq!(artifact, restored);
    [direct, materialize_artifact(&restored).unwrap()]
}
fn game() -> GameState {
    GameState::new(vec!["Alice".into(), "Bob".into()], 20)
}

fn creature(
    game: &mut GameState,
    owner: PlayerId,
    power: i32,
    toughness: i32,
    abilities: Vec<StaticAbility>,
) -> ObjectId {
    let mut definition =
        ironsmith::cards::builders::CardDefinitionBuilder::new(CardId::new(), "Legality witness")
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Elf])
            .power_toughness(PowerToughness::fixed(power, toughness));
    for ability in abilities {
        definition = definition.with_ability(ironsmith::ability::Ability::static_ability(ability));
    }
    let id = game.create_object_from_definition(&definition.build(), owner, Zone::Battlefield);
    game.remove_summoning_sickness(id);
    id
}
fn attach(game: &mut GameState, definition: &CardDefinition, recipient: ObjectId) -> ObjectId {
    let host = game.create_object_from_definition(definition, A, Zone::Battlefield);
    AttachToEffect::new(ChooseSpec::SpecificObject(recipient))
        .execute(game, &mut EffectContext::new_default(host, A))
        .unwrap();
    game.refresh_continuous_state().unwrap();
    host
}
fn settle(game: &mut GameState) {
    let mut queue = TriggerQueue::new();
    for _ in 0..16 {
        put_triggers_on_stack_with_dm(game, &mut queue, &mut SelectFirstDecisionMaker).unwrap();
        if game.stack_is_empty() {
            game.refresh_continuous_state().unwrap();
            return;
        }
        resolve_stack_entry_with(game, &mut SelectFirstDecisionMaker).unwrap();
    }
    panic!("bounded scenario failed to settle");
}
fn enter(game: &mut GameState, definition: &CardDefinition) -> ObjectId {
    let host = game.create_object_from_definition(definition, A, Zone::Battlefield);
    game.take_pending_trigger_events();
    let event = TriggerEvent::new_with_provenance(
        ironsmith::events::ZoneChangeEvent::with_cause(
            host,
            Zone::Stack,
            Zone::Battlefield,
            ironsmith::events::cause::EventCause::from_game_rule(),
            None,
        ),
        Default::default(),
    );
    let mut queue = TriggerQueue::new();
    for trigger in check_triggers(game, &event) {
        queue.add(trigger);
    }
    put_triggers_on_stack_with_dm(game, &mut queue, &mut SelectFirstDecisionMaker).unwrap();
    while !game.stack_is_empty() {
        resolve_stack_entry_with(game, &mut SelectFirstDecisionMaker).unwrap();
    }
    settle(game);
    host
}
fn blocked(game: &GameState, attacker: ObjectId, blocker: ObjectId) -> bool {
    ironsmith::rules::combat::can_block(
        game.object(attacker).unwrap(),
        game.object(blocker).unwrap(),
        game,
    )
}
#[test]
fn seventeen_full_frozen_bodies_round_trip_strictly_without_loss() {
    assert_eq!(fixtures().len(), 17);
    for card in fixtures() {
        for definition in definitions(card["name"].as_str().unwrap()) {
            assert_eq!(definition.card.name, card["name"]);
        }
    }
}
#[test]
fn attached_stats_and_blocker_predicates_move_together_and_disappear_on_source_exit() {
    for (name, boost, flying_allowed, ordinary_allowed) in [
        ("Dust Corona", 2, false, true),
        ("Skyblinder Staff", 1, false, true),
        ("Treetop Bracers", 1, true, false),
    ] {
        for definition in definitions(name) {
            let mut game = game();
            let first = creature(&mut game, A, 2, 4, vec![]);
            let second = creature(&mut game, A, 2, 4, vec![]);
            let flyer = creature(&mut game, B, 4, 4, vec![StaticAbility::flying()]);
            let ground = creature(&mut game, B, 4, 4, vec![]);
            let host = attach(&mut game, &definition, first);
            assert_eq!(game.current_power(first), Some(2 + boost));
            assert_eq!(blocked(&game, first, flyer), flying_allowed);
            assert_eq!(blocked(&game, first, ground), ordinary_allowed);
            AttachToEffect::new(ChooseSpec::SpecificObject(second))
                .execute(&mut game, &mut EffectContext::new_default(host, A))
                .unwrap();
            game.refresh_continuous_state().unwrap();
            assert_eq!(game.current_power(first), Some(2));
            assert!(blocked(&game, first, flyer) && blocked(&game, first, ground));
            assert_eq!(game.current_power(second), Some(2 + boost));
            assert_eq!(blocked(&game, second, flyer), flying_allowed);
            game.move_object_by_effect(host, Zone::Graveyard).unwrap();
            game.refresh_continuous_state().unwrap();
            assert_eq!(game.current_power(second), Some(2));
            assert!(blocked(&game, second, flyer) && blocked(&game, second, ground));
        }
    }
}
#[test]
fn power_threshold_uses_current_blocker_power_and_cagemail_actually_forbids_attack() {
    for definition in definitions("Saddle of the Cavalier") {
        let mut game = game();
        let attacker = creature(&mut game, A, 2, 4, vec![]);
        let blocker = creature(&mut game, B, 3, 4, vec![]);
        attach(&mut game, &definition, attacker);
        assert_eq!(game.current_power(attacker), Some(5));
        assert!(!blocked(&game, attacker, blocker));
        game.add_counters(blocker, CounterType::PlusOnePlusOne, 1)
            .unwrap();
        game.refresh_continuous_state().unwrap();
        assert!(blocked(&game, attacker, blocker));
    }
    for definition in definitions("Cagemail") {
        let mut game = game();
        let attacker = creature(&mut game, A, 2, 4, vec![]);
        let host = attach(&mut game, &definition, attacker);
        assert_eq!(game.current_power(attacker), Some(4));
        assert!(!ironsmith::rules::combat::can_attack(
            game.object(attacker).unwrap(),
            &game
        ));
        game.move_object_by_effect(host, Zone::Graveyard).unwrap();
        game.refresh_continuous_state().unwrap();
        assert!(ironsmith::rules::combat::can_attack(
            game.object(attacker).unwrap(),
            &game
        ));
    }
}
#[test]
fn toughness_damage_is_real_and_the_old_power_is_not_overwritten() {
    for name in ["Gauntlets of Light", "Treefolk Umbra"] {
        for definition in definitions(name) {
            let mut game = game();
            let attacker = creature(&mut game, A, 1, 3, vec![]);
            attach(&mut game, &definition, attacker);
            assert_eq!(game.current_power(attacker), Some(1));
            assert_eq!(game.current_toughness(attacker), Some(5));
            game.turn.active_player = A;
            game.turn.phase = ironsmith::game_state::Phase::Combat;
            let mut combat = ironsmith::combat_state::CombatState::default();
            ironsmith::combat_state::declare_attackers(
                &mut game,
                &mut combat,
                vec![(attacker, ironsmith::combat_state::AttackTarget::Player(B))],
            )
            .unwrap();
            game.combat = Some(combat.clone());
            ironsmith::game_loop::execute_combat_damage_step(&mut game, &combat, false);
            assert_eq!(game.player(B).unwrap().life, 15);
        }
    }
}
#[test]
fn goad_is_supplied_by_aura_controller_and_not_its_enchanted_creature_controller() {
    for (name, boost) in [("Ghoulish Impetus", 1), ("Predatory Impetus", 3)] {
        for definition in definitions(name) {
            let mut game = game();
            let recipient = creature(&mut game, B, 2, 4, vec![]);
            let host = attach(&mut game, &definition, recipient);
            assert_eq!(game.current_power(recipient), Some(2 + boost));
            assert_eq!(
                game.active_goaders_for(recipient),
                [A].into_iter().collect()
            );
            if name == "Predatory Impetus" {
                assert!(game.must_be_blocked(recipient));
            } else {
                assert!(game.object_has_ability(recipient, &StaticAbility::deathtouch()));
            }
            ironsmith::effects::ApplyContinuousEffect::with_spec(
                ChooseSpec::SpecificObject(recipient),
                ironsmith::continuous::Modification::RemoveAllAbilities,
                ironsmith::effect::Until::EndOfTurn,
            )
            .execute(&mut game, &mut EffectContext::new_default(host, A))
            .unwrap();
            game.refresh_continuous_state().unwrap();
            assert_eq!(
                game.active_goaders_for(recipient),
                [A].into_iter().collect(),
                "goad is a designation, not an ability on the recipient"
            );
            game.set_current_controller(host, B).unwrap();
            game.refresh_continuous_state().unwrap();
            assert_eq!(
                game.active_goaders_for(recipient),
                [B].into_iter().collect()
            );
            game.phase_out(host);
            game.refresh_continuous_state().unwrap();
            assert!(game.active_goaders_for(recipient).is_empty());
            assert_eq!(game.current_power(recipient), Some(2));
            game.phase_in(host);
            game.refresh_continuous_state().unwrap();
            assert_eq!(game.current_power(recipient), Some(2 + boost));
        }
    }
}
#[test]
fn deity_color_gates_guard_both_anthem_and_block_requirement() {
    for definition in definitions("Gift of the Deity") {
        let mut game = game();
        let recipient = creature(&mut game, A, 2, 4, vec![]);
        let blocker = creature(&mut game, B, 3, 5, vec![]);
        let host = attach(&mut game, &definition, recipient);
        assert_eq!(game.current_power(recipient), Some(2));
        assert!(!game.must_block_attacker(blocker, recipient));
        ironsmith::effects::ApplyContinuousEffect::with_spec(
            ChooseSpec::SpecificObject(recipient),
            ironsmith::continuous::Modification::SetColors(ColorSet::BLACK.union(ColorSet::GREEN)),
            ironsmith::effect::Until::EndOfTurn,
        )
        .execute(&mut game, &mut EffectContext::new_default(host, A))
        .unwrap();
        game.refresh_continuous_state().unwrap();
        assert_eq!(game.current_power(recipient), Some(4));
        assert!(game.object_has_ability(recipient, &StaticAbility::deathtouch()));
        assert!(game.must_block_attacker(blocker, recipient));
        ironsmith::turn::execute_cleanup_step(&mut game);
        game.refresh_continuous_state().unwrap();
        assert_eq!(game.current_power(recipient), Some(2));
        assert!(!game.must_block_attacker(blocker, recipient));
    }
}
#[test]
fn grouped_graveyard_count_divides_before_scaling_and_tracks_actual_zone_changes() {
    for definition in definitions("Dark Matter Manipulator") {
        let mut game = game();
        let body = CardBuilder::new(CardId::new(), "Graveyard witness")
            .card_types(vec![CardType::Artifact])
            .build();
        for _ in 0..10 {
            game.create_object_from_card(&body, A, Zone::Library);
        }
        let host = enter(&mut game, &definition);
        assert_eq!(game.player(A).unwrap().graveyard.len(), 3);
        let base = game.object(host).unwrap().base_power.clone();
        let printed = match base {
            Some(ironsmith::card::PtValue::Fixed(value)) => value,
            _ => panic!("fixed fixture power"),
        };
        for total in 3..=14 {
            if total > 3 {
                game.create_object_from_card(&body, A, Zone::Graveyard);
            }
            game.refresh_continuous_state().unwrap();
            assert_eq!(game.current_power(host), Some(printed + 2 * (total / 7)));
        }
        let gone = game.player(A).unwrap().graveyard[0];
        game.move_object_by_effect(gone, Zone::Exile).unwrap();
        game.refresh_continuous_state().unwrap();
        assert_eq!(game.current_power(host), Some(printed + 2));
    }
}
#[test]
fn all_creature_types_are_live_and_do_not_erase_existing_types_or_conditions() {
    for definition in definitions("Stalactite Dagger") {
        let mut game = game();
        let equipment = enter(&mut game, &definition);
        let token = game
            .battlefield
            .iter()
            .copied()
            .find(|id| game.object(*id).unwrap().kind == ironsmith::object::ObjectKind::Token)
            .unwrap();
        assert!(game.current_has_subtype(token, Subtype::Dragon));
        let recipient = creature(&mut game, A, 2, 4, vec![]);
        AttachToEffect::new(ChooseSpec::SpecificObject(recipient))
            .execute(&mut game, &mut EffectContext::new_default(equipment, A))
            .unwrap();
        game.refresh_continuous_state().unwrap();
        assert!(game.current_has_subtype(recipient, Subtype::Elf));
        assert!(game.current_has_subtype(recipient, Subtype::Dragon));
        assert_eq!(game.current_power(recipient), Some(3));
        game.move_object_by_effect(equipment, Zone::Graveyard)
            .unwrap();
        game.refresh_continuous_state().unwrap();
        assert!(!game.current_has_subtype(recipient, Subtype::Dragon));
    }
    for definition in definitions("Undercover Skrull") {
        let mut game = game();
        let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        assert!(!game.current_has_subtype(host, Subtype::Dragon));
        let body = CardBuilder::new(CardId::new(), "Graveyard creature")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(1, 1))
            .build();
        for _ in 0..2 {
            game.create_object_from_card(&body, A, Zone::Graveyard);
        }
        game.refresh_continuous_state().unwrap();
        assert!(game.current_has_subtype(host, Subtype::Dragon));
        let gone = game.player(A).unwrap().graveyard[0];
        game.move_object_by_effect(gone, Zone::Exile).unwrap();
        game.refresh_continuous_state().unwrap();
        assert!(!game.current_has_subtype(host, Subtype::Dragon));
    }
}

#[test]
fn magemark_uses_the_live_enchanted_set_and_each_auras_controller() {
    for definition in definitions("Infiltrator's Magemark") {
        let mut game = game();
        let yours = creature(&mut game, A, 2, 4, vec![]);
        let plain = creature(&mut game, A, 2, 4, vec![]);
        let theirs = creature(&mut game, B, 2, 4, vec![]);
        let defender = creature(&mut game, B, 3, 5, vec![StaticAbility::defender()]);
        let ordinary = creature(&mut game, B, 3, 5, vec![]);
        let first = attach(&mut game, &definition, yours);
        assert_eq!(game.current_power(yours), Some(3));
        assert_eq!(game.current_power(plain), Some(2));
        assert!(blocked(&game, yours, defender));
        assert!(!blocked(&game, yours, ordinary));
        attach(&mut game, &definition, theirs);
        assert_eq!(game.current_power(yours), Some(4));
        assert_eq!(game.current_power(theirs), Some(2));
        game.set_current_controller(first, B).unwrap();
        game.refresh_continuous_state().unwrap();
        assert_eq!(game.current_power(yours), Some(3));
        assert_eq!(game.current_power(theirs), Some(3));
    }
}
#[test]
fn wall_crawl_keeps_entry_token_life_and_static_blocker_predicate() {
    for definition in definitions("Wall Crawl") {
        let mut game = game();
        let host = enter(&mut game, &definition);
        let spider = game
            .battlefield
            .iter()
            .copied()
            .find(|id| *id != host && game.current_has_subtype(*id, Subtype::Spider))
            .unwrap();
        assert_eq!(game.player(A).unwrap().life, 21);
        assert_eq!(game.current_power(spider), Some(3));
        assert_eq!(game.current_toughness(spider), Some(2));
        assert!(game.object_has_ability(spider, &StaticAbility::reach()));
        let wall = creature(&mut game, B, 3, 5, vec![StaticAbility::defender()]);
        let ordinary = creature(&mut game, B, 3, 5, vec![]);
        assert!(!blocked(&game, spider, wall));
        assert!(blocked(&game, spider, ordinary));
        game.move_object_by_effect(host, Zone::Graveyard).unwrap();
        game.refresh_continuous_state().unwrap();
        assert!(blocked(&game, spider, wall));
        assert_eq!(game.current_power(spider), Some(2));
    }
}
#[test]
fn dancers_quoted_commander_anthem_belongs_to_the_receiving_creature() {
    for definition in definitions("Dancer's Chakrams") {
        let mut game = game();
        let equipment = enter(&mut game, &definition);
        assert!(
            matches!(
                game.object(equipment).unwrap().attached_to,
                Some(ironsmith::object::AttachmentTarget::Object(_))
            ),
            "actual Job select must attach its created Hero"
        );
        let recipient = creature(&mut game, B, 2, 4, vec![]);
        let other_b = creature(&mut game, B, 2, 4, vec![]);
        let other_a = creature(&mut game, A, 2, 4, vec![]);
        for id in [recipient, other_b, other_a] {
            game.set_as_commander(id, game.object(id).unwrap().owner);
        }
        AttachToEffect::new(ChooseSpec::SpecificObject(recipient))
            .execute(&mut game, &mut EffectContext::new_default(equipment, A))
            .unwrap();
        game.refresh_continuous_state().unwrap();
        assert_eq!(
            game.current_power(recipient),
            Some(4),
            "other excludes the receiver from its own nested anthem"
        );
        assert_eq!(game.current_power(other_b), Some(4));
        assert_eq!(game.current_power(other_a), Some(2));
        assert!(game.object_has_ability(other_b, &StaticAbility::lifelink()));
        assert!(game.current_has_subtype(recipient, Subtype::Performer));
        game.set_current_controller(recipient, A).unwrap();
        game.refresh_continuous_state().unwrap();
        assert_eq!(game.current_power(other_b), Some(2));
        assert_eq!(game.current_power(other_a), Some(4));
        game.move_object_by_effect(equipment, Zone::Graveyard)
            .unwrap();
        game.refresh_continuous_state().unwrap();
        assert!(!game.current_has_subtype(recipient, Subtype::Performer));
        assert_eq!(game.current_power(other_a), Some(2));
    }
}
#[test]
fn ninjas_full_grant_keeps_damage_player_across_the_receivers_draw_and_discard() {
    for definition in definitions("Ninja's Blades") {
        let mut game = game();
        let equipment = enter(&mut game, &definition);
        let recipient = creature(&mut game, B, 2, 4, vec![]);
        let hand = CardBuilder::new(CardId::new(), "Discarded mana-value witness")
            .mana_cost(ironsmith::mana::ManaCost::from_pips(vec![vec![
                ironsmith::mana::ManaSymbol::Generic(3),
            ]]))
            .card_types(vec![CardType::Artifact])
            .build();
        let discard = game.create_object_from_card(&hand, B, Zone::Hand);
        game.create_object_from_card(
            &CardBuilder::new(CardId::new(), "Drawn land")
                .card_types(vec![CardType::Land])
                .build(),
            B,
            Zone::Library,
        );
        AttachToEffect::new(ChooseSpec::SpecificObject(recipient))
            .execute(&mut game, &mut EffectContext::new_default(equipment, A))
            .unwrap();
        game.refresh_continuous_state().unwrap();
        assert!(game.current_has_subtype(recipient, Subtype::Ninja));
        assert_eq!(game.current_power(recipient), Some(3));
        let outcome = ironsmith::effects::DealDamageEffect::new(1, ChooseSpec::SpecificPlayer(A))
            .with_combat(true)
            .execute(&mut game, &mut EffectContext::new_default(recipient, B))
            .unwrap();
        for event in outcome.events {
            game.queue_trigger_event(Default::default(), event);
        }
        settle(&mut game);
        assert_eq!(game.player(A).unwrap().life, 16);
        assert_eq!(game.player(B).unwrap().life, 20);
        assert_eq!(game.player(B).unwrap().hand.len(), 1);
        assert!(!game.player(B).unwrap().hand.contains(&discard));
        assert_eq!(game.player(A).unwrap().hand.len(), 0);
    }
}
#[test]
fn treefolk_umbra_still_replaces_destruction_and_gauntlets_retains_untap_ability() {
    for definition in definitions("Treefolk Umbra") {
        let mut game = game();
        let recipient = creature(&mut game, A, 2, 4, vec![]);
        let aura = attach(&mut game, &definition, recipient);
        ironsmith::effects::DestroyEffect::with_spec(ChooseSpec::SpecificObject(recipient))
            .execute(&mut game, &mut EffectContext::new_default(aura, B))
            .unwrap();
        game.refresh_continuous_state().unwrap();
        assert!(game.battlefield.contains(&recipient));
        assert!(!game.battlefield.contains(&aura));
        assert_eq!(game.current_toughness(recipient), Some(4));
    }
    for definition in definitions("Gauntlets of Light") {
        let mut game = game();
        let recipient = creature(&mut game, A, 2, 4, vec![]);
        attach(&mut game, &definition, recipient);
        game.tap(recipient);
        let program = game
            .current_abilities(recipient)
            .unwrap()
            .into_iter()
            .find_map(|ability| match ability.kind {
                AbilityKind::Activated(ability) => Some(ability.effects),
                _ => None,
            })
            .unwrap();
        game.push_to_stack(StackEntry::ability(recipient, A, program));
        resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
        assert!(!game.is_tapped(recipient));
    }
}

#[test]
fn ghoulish_delayed_return_retains_only_the_authorized_graveyard_incarnation() {
    // This secondary-program gate is deliberately unignored even though the
    // card remains partial pending exact cross-zone source-identity review.
    for definition in definitions("Ghoulish Impetus") {
        for extra_zone_change in [false, true] {
            let mut game = game();
            let victim = creature(&mut game, B, 2, 4, vec![]);
            creature(&mut game, A, 2, 4, vec![]);
            let aura = attach(&mut game, &definition, victim);
            let stable = game.object(aura).unwrap().stable_id;
            game.take_pending_trigger_events();
            let outcome =
                ironsmith::effects::DestroyEffect::with_spec(ChooseSpec::SpecificObject(victim))
                    .execute(&mut game, &mut EffectContext::new_default(aura, A))
                    .unwrap();
            for event in outcome.events {
                game.queue_trigger_event(Default::default(), event);
            }
            let mut queue = TriggerQueue::new();
            ironsmith::game_loop::check_and_apply_sbas(&mut game, &mut queue).unwrap();
            put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut SelectFirstDecisionMaker)
                .unwrap();
            assert!(!game.stack_is_empty());
            let graveyard_aura = game.find_object_by_stable_id(stable).unwrap();
            assert_eq!(game.object(graveyard_aura).unwrap().zone, Zone::Graveyard);
            if extra_zone_change {
                let exiled = game
                    .move_object_by_effect(graveyard_aura, Zone::Exile)
                    .unwrap();
                game.move_object_by_effect(exiled, Zone::Graveyard).unwrap();
            }
            while !game.stack_is_empty() {
                resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
            }
            let mut runner = ironsmith::turn_runner::TurnRunner::from_state_for_sync(
                ironsmith::turn_runner::TurnState::EndStep,
            );
            runner.advance(&mut game, &mut queue).unwrap();
            put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut SelectFirstDecisionMaker)
                .unwrap();
            while !game.stack_is_empty() {
                resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
            }
            let current = game.find_object_by_stable_id(stable).unwrap();
            assert_eq!(
                game.object(current).unwrap().zone == Zone::Battlefield,
                !extra_zone_change
            );
            if !extra_zone_change {
                assert!(game.object(current).unwrap().attached_to.is_some());
            }
        }
    }
}
