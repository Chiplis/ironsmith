use ironsmith::cards::{CardDefinition, builders::CardDefinitionBuilder};
use ironsmith::ids::CardId;
use ironsmith::{CardType, GameState, PlayerId, Zone};

fn payload() -> ironsmith_tools::CardPayload {
    ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        "Laelia, the Blade Reforged",
    )
    .unwrap()
    .remove(0)
}
fn definition() -> CardDefinition {
    ironsmith_tools::compile_definition_from_payload(&payload()).unwrap()
}
#[test]
fn strict_snapshot_and_full_quality_gate() {
    let s = ironsmith_tools::compile_authoritative_snapshot_from_payload(&payload());
    assert_eq!(
        s.parse_status,
        ironsmith_tools::ParseStatus::StrictCompiled,
        "{:?}",
        s.parse_error
    );
    assert!(!s.parse_lossy && !s.has_unimplemented && s.parse_error.is_none());
    assert!(
        s.similarity_score >= 0.99,
        "{}: {:?}",
        s.similarity_score,
        s.compiled_text
    );
}
#[test]
fn exile_trigger_filters_owner_origin_and_counts_batches_once() {
    let def = definition();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for owner in [alice, bob] {
        for zone in [
            Zone::Library,
            Zone::Graveyard,
            Zone::Hand,
            Zone::Battlefield,
        ] {
            for count in [0, 1, 3] {
                let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                let source = game.create_object_from_definition(&def, alice, Zone::Battlefield);
                let probe = CardDefinitionBuilder::new(CardId::new(), "Exile probe")
                    .card_types(vec![CardType::Artifact])
                    .build();
                for _ in 0..count {
                    game.create_object_from_definition(&probe, owner, zone);
                }
                game.push_to_stack(ironsmith::game_state::StackEntry::ability(
                    source,
                    bob,
                    vec![ironsmith::Effect::exile_all(
                        ironsmith::target::ObjectFilter::default()
                            .in_zone(zone)
                            .named("Exile probe"),
                    )],
                ));
                ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
                let mut queue = ironsmith::triggers::TriggerQueue::new();
                ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
                let expected = usize::from(
                    owner == alice && matches!(zone, Zone::Library | Zone::Graveyard) && count > 0,
                );
                assert_eq!(
                    game.stack.len(),
                    expected,
                    "owner={owner:?}, zone={zone:?}, count={count}"
                );
                if expected != 0 {
                    ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
                }
                let counters = game
                    .object(source)
                    .unwrap()
                    .counters
                    .get(&ironsmith::object::CounterType::PlusOnePlusOne)
                    .copied()
                    .unwrap_or(0);
                assert_eq!(
                    counters, expected as u32,
                    "owner={owner:?}, zone={zone:?}, count={count}"
                );
            }
        }
    }
}

#[test]
fn attack_exiles_top_card_and_grants_normal_play_only_this_turn() {
    use ironsmith::decision::{LegalAction, compute_legal_actions};
    let def = definition();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for card_type in [CardType::Land, CardType::Sorcery] {
        for source_leaves in [false, true] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            game.turn.active_player = alice;
            game.turn.priority_player = Some(alice);
            game.turn.phase = ironsmith::game_state::Phase::Combat;
            let source = game.create_object_from_definition(&def, alice, Zone::Battlefield);
            let probe = CardDefinitionBuilder::new(CardId::new(), "Top card probe")
                .card_types(vec![card_type])
                .mana_cost(ironsmith::mana::ManaCost::from_symbols(vec![
                    ironsmith::mana::ManaSymbol::Generic(2),
                ]))
                .build();
            let card = game.create_object_from_definition(&probe, alice, Zone::Library);
            let identity = game.object(card).unwrap().stable_id;
            let mut combat = ironsmith::combat_state::CombatState::default();
            ironsmith::combat_state::declare_attackers(
                &mut game,
                &mut combat,
                vec![(source, ironsmith::combat_state::AttackTarget::Player(bob))],
            )
            .unwrap();
            let event = ironsmith::triggers::TriggerEvent::new_with_provenance(
                ironsmith::events::CreatureAttackedEvent::new(
                    source,
                    ironsmith::triggers::event::AttackEventTarget::Player(bob),
                ),
                ironsmith::provenance::ProvNodeId::default(),
            );
            let mut queue = ironsmith::triggers::TriggerQueue::new();
            for entry in ironsmith::triggers::check_triggers(&game, &event) {
                queue.add(entry);
            }
            ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
            assert_eq!(game.stack.len(), 1);
            if source_leaves {
                game.move_object_by_effect(source, Zone::Graveyard);
            }
            ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
            ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
            assert_eq!(game.stack.len(), usize::from(!source_leaves));
            if !source_leaves {
                ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
            }
            let exiled = game.find_object_by_stable_id(identity).unwrap();
            assert_eq!(game.object(exiled).unwrap().zone, Zone::Exile);
            game.player_mut(alice)
                .unwrap()
                .mana_pool
                .add(ironsmith::mana::ManaSymbol::Colorless, 5);
            let has_play = |game: &GameState, player| {
                compute_legal_actions(game, player).iter().any(|a| match a {
                    LegalAction::PlayLand { land_id } => *land_id == exiled,
                    LegalAction::CastSpell { spell_id, .. } => *spell_id == exiled,
                    _ => false,
                })
            };
            assert!(!has_play(&game, alice), "normal timing remains in combat");
            game.turn.phase = ironsmith::game_state::Phase::NextMain;
            assert!(
                has_play(&game, alice),
                "attack grants play permission even if source left"
            );
            assert!(!has_play(&game, bob));
            game.turn.turn_number += 1;
            assert!(!has_play(&game, alice), "permission expires next turn");
        }
    }
}

#[test]
fn mixed_origins_share_one_trigger_only_when_exiled_simultaneously() {
    let def = definition();
    let alice = PlayerId::from_index(0);
    for separate_instructions in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&def, alice, Zone::Battlefield);
        let probe = CardDefinitionBuilder::new(CardId::new(), "Mixed exile probe")
            .card_types(vec![CardType::Artifact])
            .build();
        for zone in [Zone::Library, Zone::Graveyard] {
            game.create_object_from_definition(&probe, alice, zone);
        }
        let filters = [Zone::Library, Zone::Graveyard].map(|zone| {
            ironsmith::target::ObjectFilter::default()
                .in_zone(zone)
                .named("Mixed exile probe")
        });
        let effects = if separate_instructions {
            filters
                .into_iter()
                .map(ironsmith::Effect::exile_all)
                .collect()
        } else {
            let mut union = ironsmith::target::ObjectFilter::default();
            union.any_of = filters.to_vec();
            vec![ironsmith::Effect::exile_all(union)]
        };
        game.push_to_stack(ironsmith::game_state::StackEntry::ability(
            source, alice, effects,
        ));
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
        assert_eq!(
            game.stack.len(),
            if separate_instructions { 2 } else { 1 },
            "separate_instructions={separate_instructions}"
        );
    }
}

#[test]
fn exile_batch_preserves_each_triggers_duplicate_abilities_and_group_snapshots() {
    use ironsmith::triggers::zone_changes::{CountMode, ZoneChangeTrigger};
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let trigger = |count| {
        ironsmith::triggers::Trigger::new(
            ZoneChangeTrigger::new()
                .from(Zone::Graveyard)
                .to(Zone::Exile)
                .filter(ironsmith::target::ObjectFilter::default().nontoken())
                .count(count),
        )
    };
    let grouped = ironsmith::Ability::triggered(
        trigger(CountMode::OneOrMore),
        vec![ironsmith::Effect::gain_life(1)],
    );
    let each = ironsmith::Ability::triggered(
        trigger(CountMode::Each),
        vec![ironsmith::Effect::gain_life(2)],
    );
    let watcher = CardDefinitionBuilder::new(CardId::new(), "Batch watcher")
        .card_types(vec![CardType::Artifact])
        .with_ability(grouped.clone())
        .with_ability(grouped)
        .with_ability(each)
        .build();
    let source = game.create_object_from_definition(&watcher, alice, Zone::Battlefield);
    let probe = CardDefinitionBuilder::new(CardId::new(), "Batch object")
        .card_types(vec![CardType::Artifact])
        .build();
    for _ in 0..3 {
        game.create_object_from_definition(&probe, alice, Zone::Graveyard);
    }
    let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
    ironsmith::effects::execute_effect(
        &mut game,
        &ironsmith::Effect::exile_all(
            ironsmith::target::ObjectFilter::default().in_zone(Zone::Graveyard),
        ),
        &mut ironsmith::effects::EffectContext::new(source, alice, &mut dm),
    )
    .unwrap();
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    ironsmith::game_loop::drain_pending_trigger_events(&mut game, &mut queue);
    assert_eq!(
        queue.entries.len(),
        5,
        "two identical grouped abilities plus three each-card triggers"
    );
    let mut grouped_count = 0;
    for entry in &queue.entries {
        let matcher = entry
            .ability
            .trigger
            .downcast_ref::<ZoneChangeTrigger>()
            .unwrap();
        if matcher.count_mode == CountMode::OneOrMore {
            grouped_count += 1;
            assert_eq!(entry.event_value_amount, Some(3));
            assert_eq!(
                entry
                    .tagged_objects
                    .get(&ironsmith::tag::TagKey::from(
                        ironsmith_core::ZONE_CHANGE_GROUP_TAG
                    ))
                    .expect("captured matched batch snapshots")
                    .len(),
                3
            );
        }
    }
    assert_eq!(grouped_count, 2);
}
