use ironsmith::cards::{CardDefinition, builders::CardDefinitionBuilder};
use ironsmith::events::DamageTarget;
use ironsmith::events::cause::EventCause;
use ironsmith::events::processing::{
    SimultaneousDamageEvent, process_simultaneous_damage_assignments_with_event,
};
use ironsmith::ids::CardId;
use ironsmith::prevention::{PreventionShield, PreventionTarget};
use ironsmith::{CardType, GameState, PlayerId, Zone};

fn payload() -> ironsmith_tools::CardPayload {
    ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        "Frenzied Baloth",
    )
    .unwrap()
    .remove(0)
}
fn definition() -> CardDefinition {
    ironsmith_tools::compile_definition_from_payload(&payload()).unwrap()
}
fn probe(kind: CardType) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Probe")
        .card_types(vec![kind])
        .power_toughness(ironsmith::card::PowerToughness::fixed(5, 5))
        .build()
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
fn combat_only_prevention_ban_preserves_shields_and_ends_when_source_leaves() {
    let def = definition();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for affected in [alice, bob] {
        for object_target in [false, true] {
            for combat in [false, true] {
                let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                let baloth = game.create_object_from_definition(&def, alice, Zone::Battlefield);
                let source = game.create_object_from_definition(
                    &probe(CardType::Creature),
                    bob,
                    Zone::Battlefield,
                );
                let creature = game.create_object_from_definition(
                    &probe(CardType::Creature),
                    affected,
                    Zone::Battlefield,
                );
                let (target, shield_target) = if object_target {
                    (
                        DamageTarget::Object(creature),
                        PreventionTarget::Permanent(creature),
                    )
                } else {
                    (
                        DamageTarget::Player(affected),
                        PreventionTarget::Player(affected),
                    )
                };
                game.effect_store
                    .prevention_effects
                    .add_shield(PreventionShield::prevent_next_n(
                        source,
                        affected,
                        shield_target,
                        3,
                    ));
                let event = SimultaneousDamageEvent {
                    source,
                    target,
                    amount: 3,
                    is_combat: combat,
                    unpreventable: false,
                    cause: EventCause::effect(),
                    source_snapshot: None,
                };
                let result =
                    process_simultaneous_damage_assignments_with_event(&mut game, &[event.clone()]);
                assert_eq!(
                    result[0].assignments.iter().map(|a| a.amount).sum::<u32>(),
                    if combat { 3 } else { 0 },
                    "combat={combat},object={object_target}"
                );
                if combat {
                    game.move_object_by_effect(baloth, Zone::Graveyard).unwrap();
                    let result =
                        process_simultaneous_damage_assignments_with_event(&mut game, &[event]);
                    assert!(
                        result[0].assignments.is_empty(),
                        "combat prohibition does not consume shields; source leaving restores prevention"
                    );
                }
            }
        }
    }
}
#[test]
fn mixed_simultaneous_damage_allocates_shields_only_to_preventable_events() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.create_object_from_definition(&definition(), alice, Zone::Battlefield);
    let source =
        game.create_object_from_definition(&probe(CardType::Creature), bob, Zone::Battlefield);
    let other =
        game.create_object_from_definition(&probe(CardType::Creature), bob, Zone::Battlefield);
    game.effect_store
        .prevention_effects
        .add_shield(PreventionShield::prevent_next_n(
            source,
            alice,
            PreventionTarget::Player(alice),
            3,
        ));
    let event = |source, is_combat| SimultaneousDamageEvent {
        source,
        target: DamageTarget::Player(alice),
        amount: 3,
        is_combat,
        unpreventable: false,
        cause: EventCause::effect(),
        source_snapshot: None,
    };
    let results = process_simultaneous_damage_assignments_with_event(
        &mut game,
        &[event(source, true), event(other, false)],
    );
    assert_eq!(
        results[0].assignments.iter().map(|a| a.amount).sum::<u32>(),
        3
    );
    assert!(results[1].assignments.is_empty());
}
#[test]
fn self_spell_is_uncounterable_and_battlefield_grant_is_only_your_creature_spells() {
    let def = definition();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for self_spell in [false, true] {
        for controller in [alice, bob] {
            for kind in [CardType::Creature, CardType::Sorcery] {
                for source_present in [false, true] {
                    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                    let source = source_present.then(|| {
                        game.create_object_from_definition(&def, alice, Zone::Battlefield)
                    });
                    let candidate = if self_spell { def.clone() } else { probe(kind) };
                    let spell =
                        game.create_object_from_definition(&candidate, controller, Zone::Stack);
                    game.push_to_stack(ironsmith::game_state::StackEntry::new(spell, controller));
                    game.update_cant_effects();
                    let effect = ironsmith::effect::Effect::counter(
                        ironsmith::target::ChooseSpec::SpecificObject(spell),
                    );
                    let mut ctx = ironsmith::effects::EffectContext::new_default(spell, bob);
                    ironsmith::effects::execute_effect(&mut game, &effect, &mut ctx).unwrap();
                    assert_eq!(
                        !game.stack.is_empty(),
                        self_spell
                            || (source_present
                                && controller == alice
                                && kind == CardType::Creature)
                    );
                    if !self_spell && !game.stack.is_empty() {
                        game.move_object_by_effect(source.unwrap(), Zone::Graveyard)
                            .unwrap();
                        game.update_cant_effects();
                        ironsmith::effects::execute_effect(&mut game, &effect, &mut ctx).unwrap();
                        assert!(
                            game.stack.is_empty(),
                            "creature-spell protection ends with source"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn fresh_baloth_attacks_and_tramples_through_prevention() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.active_player = alice;
    game.turn.phase = ironsmith::game_state::Phase::Combat;
    let attacker = game.create_object_from_definition(&definition(), alice, Zone::Battlefield);
    let block = CardDefinitionBuilder::new(CardId::new(), "Blocker")
        .card_types(vec![CardType::Creature])
        .power_toughness(ironsmith::card::PowerToughness::fixed(1, 1))
        .build();
    let blocker = game.create_object_from_definition(&block, bob, Zone::Battlefield);
    let power = game
        .calculated_characteristics(attacker)
        .unwrap()
        .power
        .unwrap();
    assert!(power > 1);
    game.effect_store
        .prevention_effects
        .add_shield(PreventionShield::prevent_next_n(
            blocker,
            bob,
            PreventionTarget::All,
            20,
        ));
    let mut combat = ironsmith::combat_state::CombatState::default();
    ironsmith::combat_state::declare_attackers(
        &mut game,
        &mut combat,
        vec![(attacker, ironsmith::combat_state::AttackTarget::Player(bob))],
    )
    .unwrap();
    ironsmith::combat_state::declare_blockers(&game, &mut combat, vec![(blocker, attacker)])
        .unwrap();
    ironsmith::game_loop::execute_combat_damage_step(&mut game, &combat, false);
    assert_eq!(game.damage_on(blocker), 1);
    assert_eq!(game.damage_on(attacker), 1);
    assert_eq!(game.player(bob).unwrap().life, 20 - (power - 1));
}

#[test]
fn creature_abilities_remain_counterable() {
    let def = definition();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for controller in [alice, bob] {
        for triggered in [false, true] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let source = game.create_object_from_definition(&def, alice, Zone::Battlefield);
            let mut entry = ironsmith::game_state::StackEntry::ability(
                source,
                controller,
                vec![ironsmith::Effect::draw(1)],
            );
            if triggered {
                entry.triggering_event =
                    Some(ironsmith::triggers::TriggerEvent::new_with_provenance(
                        ironsmith::events::LifeGainEvent::new(controller, 1),
                        ironsmith::provenance::ProvNodeId::default(),
                    ));
            }
            game.push_to_stack(entry);
            game.update_cant_effects();
            let mut ctx = ironsmith::effects::EffectContext::new_default(source, bob);
            ironsmith::effects::execute_effect(
                &mut game,
                &ironsmith::Effect::counter(ironsmith::target::ChooseSpec::SpecificObject(source)),
                &mut ctx,
            )
            .unwrap();
            assert!(
                game.stack.is_empty(),
                "triggered={triggered},controller={controller:?}"
            );
            assert_eq!(game.object(source).unwrap().zone, Zone::Battlefield);
        }
    }
}
