//! Frozen atlas observation 24458190: canonical Lesson-trigger behavior.
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::ids::CardId;
use ironsmith::{AbilityKind, CardDefinition, CardType, GameState, PlayerId, Zone};
use ironsmith_tools::{
    ParseStatus, compile_authoritative_snapshot_from_payload, compile_definition_from_payload,
    default_cards_path, load_card_payloads_by_name,
};
fn definition() -> CardDefinition {
    let payloads = load_card_payloads_by_name(
        default_cards_path().to_str().unwrap(),
        "Aang, the Last Airbender",
    )
    .unwrap();
    assert_eq!(payloads.len(), 1);
    let snapshot = compile_authoritative_snapshot_from_payload(&payloads[0]);
    assert_eq!(
        snapshot.parse_status,
        ParseStatus::StrictCompiled,
        "{snapshot:#?}"
    );
    assert!(!snapshot.parse_lossy && !snapshot.has_unimplemented);
    let def = compile_definition_from_payload(&payloads[0]).unwrap();
    assert_eq!(
        def.abilities
            .iter()
            .filter(|a| matches!(a.kind, AbilityKind::Triggered(_)))
            .count(),
        2
    );
    def
}
#[test]
fn lesson_trigger_checks_caster_and_subtype_and_keeps_source_incarnation() {
    use ironsmith::combat_state::{AttackTarget, AttackerInfo, CombatState};
    use ironsmith::triggers::{TriggerEvent, TriggerQueue, check_triggers};
    let def = definition();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for (lesson, own, blink) in [
        (true, true, false),
        (true, false, false),
        (false, true, false),
        (true, true, true),
    ] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&def, alice, Zone::Battlefield);
        let mut fixture = CardDefinitionBuilder::new(CardId::new(), "Lesson cast fixture")
            .card_types(vec![CardType::Sorcery])
            .build();
        if lesson {
            fixture
                .card
                .subtypes
                .push(ironsmith::types::Subtype::Lesson);
        }
        let caster = if own { alice } else { bob };
        let spell = game.create_object_from_definition(&fixture, caster, Zone::Stack);
        let event = TriggerEvent::new_with_provenance(
            ironsmith::events::SpellCastEvent::new(spell, caster, Zone::Hand),
            ironsmith::provenance::ProvNodeId::default(),
        );
        let mut queue = TriggerQueue::new();
        for entry in check_triggers(&game, &event) {
            queue.add(entry);
        }
        assert_eq!(queue.entries.len(), usize::from(lesson && own));
        ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
        let retained = if blink {
            let hand = game.move_object_by_effect(source, Zone::Hand).unwrap();
            game.move_object_by_effect(hand, Zone::Battlefield).unwrap()
        } else {
            source
        };
        if lesson && own {
            ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        }
        let mut combat = CombatState::default();
        combat.attackers.push(AttackerInfo {
            creature: retained,
            target: AttackTarget::Player(bob),
        });
        ironsmith::game_loop::execute_combat_damage_step(&mut game, &combat, false);
        assert_eq!(game.player(bob).unwrap().life, 17);
        assert_eq!(
            game.player(alice).unwrap().life,
            if lesson && own && !blink { 23 } else { 20 },
            "lesson={lesson}, own={own}, blink={blink}"
        );
        let life_after_first_damage = game.player(alice).unwrap().life;
        ironsmith::turn::execute_cleanup_step(&mut game);
        ironsmith::game_loop::execute_combat_damage_step(&mut game, &combat, false);
        assert_eq!(
            game.player(alice).unwrap().life,
            life_after_first_damage,
            "lifelink expires at cleanup"
        );
        assert_eq!(game.player(bob).unwrap().life, 14);
    }
}

#[test]
fn airbend_nonland_selection_excludes_source_and_lands_and_can_decline() {
    use ironsmith::decision::DecisionMaker;
    use ironsmith::game_state::Target;
    use ironsmith::triggers::{TriggerQueue, check_triggers};
    struct Pick {
        selected: Option<ironsmith::ObjectId>,
        source: ironsmith::ObjectId,
        land: ironsmith::ObjectId,
    }
    impl DecisionMaker for Pick {
        fn decide_targets(
            &mut self,
            _: &GameState,
            ctx: &ironsmith::decisions::context::TargetsContext,
        ) -> Vec<Target> {
            let legal: Vec<_> = ctx
                .requirements
                .iter()
                .flat_map(|r| r.legal_targets.iter())
                .collect();
            assert!(!legal.contains(&&Target::Object(self.source)));
            assert!(!legal.contains(&&Target::Object(self.land)));
            if let Some(selected) = self.selected {
                assert!(legal.contains(&&Target::Object(selected)));
            }
            self.selected.map(Target::Object).into_iter().collect()
        }
    }
    let def = definition();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for decline in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let artifact = CardDefinitionBuilder::new(CardId::new(), "Airbend noncreature artifact")
            .card_types(vec![CardType::Artifact])
            .build();
        let target = game.create_object_from_definition(&artifact, bob, Zone::Battlefield);
        game.set_current_controller(target, alice);
        let stable = game.object(target).unwrap().stable_id;
        let land = CardDefinitionBuilder::new(CardId::new(), "Excluded noncreature land")
            .card_types(vec![CardType::Land])
            .build();
        let land = game.create_object_from_definition(&land, bob, Zone::Battlefield);
        let hand = game.create_object_from_definition(&def, alice, Zone::Hand);
        let source = game
            .move_object_with_etb_processing(hand, Zone::Battlefield)
            .unwrap()
            .new_id;
        let mut queue = TriggerQueue::new();
        for event in game.take_pending_trigger_events() {
            for entry in check_triggers(&game, &event) {
                queue.add(entry);
            }
        }
        assert_eq!(queue.entries.len(), 1);
        ironsmith::game_loop::put_triggers_on_stack_with_dm(
            &mut game,
            &mut queue,
            &mut Pick {
                selected: (!decline).then_some(target),
                source,
                land,
            },
        )
        .unwrap();
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        let retained = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(
            game.object(retained).unwrap().zone,
            if decline {
                Zone::Battlefield
            } else {
                Zone::Exile
            }
        );
        assert_eq!(game.object(source).unwrap().zone, Zone::Battlefield);
        assert_eq!(game.object(land).unwrap().zone, Zone::Battlefield);
        for player in [alice, bob] {
            assert_eq!(
                game.effect_store
                    .grant_registry
                    .granted_alternative_casts_for_card(&game, retained, Zone::Exile, player)
                    .len(),
                usize::from(!decline && player == bob)
            );
        }
    }
}

#[test]
fn lifelink_follows_damage_controller_after_source_control_changes() {
    use ironsmith::combat_state::{AttackTarget, AttackerInfo, CombatState};
    use ironsmith::triggers::{TriggerEvent, TriggerQueue, check_triggers};
    let def = definition();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let source = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let lesson = CardDefinitionBuilder::new(CardId::new(), "Control Lesson fixture")
        .card_types(vec![CardType::Sorcery])
        .subtypes(vec![ironsmith::types::Subtype::Lesson])
        .build();
    let spell = game.create_object_from_definition(&lesson, alice, Zone::Stack);
    let event = TriggerEvent::new_with_provenance(
        ironsmith::events::SpellCastEvent::new(spell, alice, Zone::Hand),
        ironsmith::provenance::ProvNodeId::default(),
    );
    let mut queue = TriggerQueue::new();
    for entry in check_triggers(&game, &event) {
        queue.add(entry);
    }
    assert_eq!(queue.entries.len(), 1);
    ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
    game.set_current_controller(source, bob);
    ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
    let blocker = CardDefinitionBuilder::new(CardId::new(), "Grounded blocker")
        .card_types(vec![CardType::Creature])
        .power_toughness(ironsmith::card::PowerToughness::fixed(2, 2))
        .build();
    let blocker = game.create_object_from_definition(&blocker, alice, Zone::Battlefield);
    assert!(!ironsmith::rules::combat::can_block(
        game.object(source).unwrap(),
        game.object(blocker).unwrap(),
        &game
    ));
    let mut combat = CombatState::default();
    combat.attackers.push(AttackerInfo {
        creature: source,
        target: AttackTarget::Player(alice),
    });
    ironsmith::game_loop::execute_combat_damage_step(&mut game, &combat, false);
    assert_eq!(game.player(alice).unwrap().life, 17);
    assert_eq!(
        game.player(bob).unwrap().life,
        23,
        "lifelink benefits damage source's current controller"
    );
}
