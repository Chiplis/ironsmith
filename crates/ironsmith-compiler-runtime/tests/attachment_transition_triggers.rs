//! Exact attachment-trigger fixtures through real attach/equip/Aura-entry and
//! departure producers. Source-only campaign: authored regressions are unrun.
use ironsmith::ability::AbilityKind;
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{LegalAction, SelectFirstDecisionMaker};
use ironsmith::effects::{AttachToEffect, EffectContext, EffectExecutor};
use ironsmith::events::cause::EventCause;
use ironsmith::game_loop::{put_triggers_on_stack_with_dm, resolve_stack_entry_with};
use ironsmith::object::AttachmentTarget;
use ironsmith::target::ChooseSpec;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{GameState, ObjectId, Phase, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_core::trigger_model::TriggerKind;
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/attachment_transition_triggers.json.fixture"
    ))
    .unwrap()
}
fn definitions(name: &str) -> [CardDefinition; 2] {
    let row = fixtures()
        .into_iter()
        .find(|row| row["name"] == name)
        .unwrap();
    let (artifact, direct) = compile_to_artifact(name, row["text"].as_str().unwrap(), false)
        .unwrap_or_else(|error| panic!("{name}: {error}"));
    let restored: CompiledCardArtifact =
        serde_json::from_slice(&serde_json::to_vec(&artifact).unwrap()).unwrap();
    restored.validate().unwrap();
    [direct, materialize_artifact(&restored).unwrap()]
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(A);
    game
}
fn resource(game: &mut GameState, player: PlayerId, zone: Zone, aura: bool) -> ObjectId {
    let definition = compile_to_runtime_definition(if aura { "Attachment Aura" } else { "Attachment recipient" },
        if aura { "Mana cost: {0}\nType: Enchantment — Aura\nEnchant creature\nEnchanted creature gets +0/+1." } else { "Type: Creature\nPower/Toughness: 3/3" }, false).unwrap();
    game.create_object_from_definition(&definition, player, zone)
}
fn stack(game: &mut GameState) -> usize {
    put_triggers_on_stack_with_dm(
        game,
        &mut TriggerQueue::new(),
        &mut SelectFirstDecisionMaker,
    )
    .unwrap();
    game.stack.len()
}
fn settle(game: &mut GameState) -> usize {
    let count = stack(game);
    for _ in 0..20 {
        if game.stack_is_empty() {
            return count;
        }
        resolve_stack_entry_with(game, &mut SelectFirstDecisionMaker).unwrap();
        stack(game);
    }
    panic!("attachment triggers did not settle")
}
fn attach(game: &mut GameState, attachment: ObjectId, recipient: ObjectId, actor: PlayerId) {
    let mut dm = SelectFirstDecisionMaker;
    let mut ctx = EffectContext::new(attachment, actor, &mut dm);
    let result = AttachToEffect::new(ChooseSpec::SpecificObject(recipient))
        .execute(game, &mut ctx)
        .unwrap();
    for event in result.events {
        game.queue_trigger_event(event.provenance(), event);
    }
}
fn tokens(game: &GameState) -> Vec<ObjectId> {
    game.battlefield
        .iter()
        .copied()
        .filter(|id| game.object(*id).unwrap().kind == ironsmith::object::ObjectKind::Token)
        .collect()
}
fn announce(game: &mut GameState, action: LegalAction) {
    use ironsmith::game_loop::{
        PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
        apply_priority_response_with_dm,
    };
    let mut state = PriorityLoopState::new(2);
    let mut queue = TriggerQueue::new();
    let mut dm = SelectFirstDecisionMaker;
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        &mut dm,
    )
    .unwrap();
    for _ in 0..30 {
        if state.pending_cast.is_none() && state.pending_activation.is_none() {
            break;
        }
        let ironsmith::GameProgress::NeedsDecisionCtx(context) = progress else {
            panic!("action did not finish")
        };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &context, &mut dm)
            .unwrap();
    }
    assert!(state.pending_cast.is_none() && state.pending_activation.is_none());
    put_triggers_on_stack_with_dm(game, &mut queue, &mut dm).unwrap();
    assert!(!game.stack_is_empty());
}

#[test]
fn all_eight_exact_cards_round_trip_with_distinct_attachment_and_recipient_filters() {
    assert_eq!(fixtures().len(), 8);
    for row in fixtures() {
        for definition in definitions(row["name"].as_str().unwrap()) {
            assert!(definition.abilities.iter().any(|ability| matches!(&ability.kind, AbilityKind::Triggered(triggered)
            if triggered.trigger.compiled_model().is_some_and(|model| matches!(&model.kind, TriggerKind::AttachmentChanged { .. })))));
        }
    }
}

#[test]
fn resolving_an_aura_spell_triggers_self_recipient_and_retains_token_body_abilities() {
    for (name, expected) in [("Bramble Elemental", 2), ("Brood Keeper", 1)] {
        for definition in definitions(name) {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let aura = resource(&mut game, A, Zone::Hand, true);
            let aura_stable = game.object(aura).unwrap().stable_id;
            let action = LegalAction::CastSpell {
                spell_id: aura,
                from_zone: Zone::Hand,
                casting_method: ironsmith::alternative_cast::CastingMethod::Normal,
            };
            assert!(
                ironsmith::decision::compute_legal_actions(&game, A)
                    .unwrap()
                    .contains(&action)
            );
            announce(&mut game, action);
            settle(&mut game);
            let attached = game.find_object_by_stable_id(aura_stable).unwrap();
            assert_eq!(
                game.object(attached).unwrap().attached_to,
                Some(AttachmentTarget::Object(source))
            );
            assert_eq!(tokens(&game).len(), expected);
            if name == "Brood Keeper" {
                let dragon = tokens(&game)[0];
                assert!(game.current_has_static_ability_id(
                    dragon,
                    ironsmith::static_abilities::StaticAbilityId::Flying
                ));
                game.player_mut(A)
                    .unwrap()
                    .mana_pool
                    .add(ironsmith::mana::ManaSymbol::Red, 1);
                let action = ironsmith::decision::compute_legal_actions(&game, A).unwrap().into_iter().find(|action| matches!(action, LegalAction::ActivateAbility { source, .. } if *source == dragon)).unwrap();
                announce(&mut game, action);
                settle(&mut game);
                assert_eq!(game.current_power(dragon), Some(3));
            }
        }
    }
}

#[test]
fn siona_requires_both_your_aura_and_your_recipient_at_the_attachment_event() {
    for definition in definitions("Siona, Captain of the Pyleas") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let opponent = resource(&mut game, B, Zone::Battlefield, false);
        for (aura_owner, recipient, expected) in [(B, source, 0), (A, opponent, 0), (A, source, 1)]
        {
            let aura = resource(&mut game, aura_owner, Zone::Battlefield, true);
            attach(&mut game, aura, recipient, aura_owner);
            assert_eq!(settle(&mut game), expected);
        }
        assert_eq!(tokens(&game).len(), 1);
    }
}

#[test]
fn real_equip_activation_taps_the_recipient_and_same_destination_does_not_retrigger() {
    for definition in definitions("Enormous Energy Blade") {
        let mut game = game();
        let creature = resource(&mut game, A, Zone::Battlefield, false);
        let blade = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        game.player_mut(A)
            .unwrap()
            .mana_pool
            .add(ironsmith::mana::ManaSymbol::Colorless, 2);
        let action = ironsmith::decision::compute_legal_actions(&game, A).unwrap().into_iter().find(|action| matches!(action, LegalAction::ActivateAbility { source, .. } if *source == blade)).unwrap();
        announce(&mut game, action);
        settle(&mut game);
        assert_eq!(
            game.object(blade).unwrap().attached_to,
            Some(AttachmentTarget::Object(creature))
        );
        assert!(game.is_tapped(creature) && !game.is_tapped(blade));
        assert_eq!(game.current_power(creature), Some(7));
        game.untap(creature);
        assert!(game.attach_object_to_target(blade, AttachmentTarget::Object(creature)));
        assert_eq!(settle(&mut game), 0);
        assert!(!game.is_tapped(creature));
        game.phase_out(blade);
        assert_eq!(settle(&mut game), 0, "phasing is not becoming unattached");
        assert_eq!(
            game.object(blade).unwrap().attached_to,
            Some(AttachmentTarget::Object(creature))
        );
    }
}

const UNATTACH: [&str; 4] = [
    "Captain's Hook",
    "Grafted Exoskeleton",
    "Grafted Wargear",
    "Stitcher's Graft",
];
#[test]
fn reattachment_and_equipment_departure_each_affect_the_original_recipient() {
    for name in UNATTACH {
        for definition in definitions(name) {
            for equipment_leaves in [false, true] {
                let mut game = game();
                let first = resource(&mut game, A, Zone::Battlefield, false);
                let second = resource(&mut game, A, Zone::Battlefield, false);
                let equipment =
                    game.create_object_from_definition(&definition, A, Zone::Battlefield);
                attach(&mut game, equipment, first, A);
                assert_eq!(settle(&mut game), 0);
                assert!(game.attach_object_to_target(equipment, AttachmentTarget::Object(first)));
                assert_eq!(settle(&mut game), 0);
                if equipment_leaves {
                    game.move_object(equipment, Zone::Exile, EventCause::effect())
                        .unwrap();
                } else {
                    attach(&mut game, equipment, second, A);
                }
                assert_eq!(
                    settle(&mut game),
                    1,
                    "{name}, source departure={equipment_leaves}"
                );
                assert!(!game.battlefield.contains(&first));
                assert!(game.battlefield.contains(&second));
            }
        }
    }
}

#[test]
fn unattachment_sacrifice_uses_the_trigger_controllers_authority_and_never_follows_a_blink() {
    for name in UNATTACH {
        for definition in definitions(name) {
            let mut game = game();
            let opponent = resource(&mut game, B, Zone::Battlefield, false);
            let equipment = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            attach(&mut game, equipment, opponent, A);
            settle(&mut game);
            assert!(game.detach_object_from_current_target(equipment));
            assert_eq!(settle(&mut game), 1);
            assert_eq!(
                game.battlefield.contains(&opponent),
                name != "Captain's Hook",
                "only destroy can remove an opponent's permanent"
            );

            let own = resource(&mut game, A, Zone::Battlefield, false);
            attach(&mut game, equipment, own, A);
            settle(&mut game);
            // Recipient departure produces the event too, with its pre-departure
            // snapshot. Returning the card creates a new incarnation.
            let exile = game
                .move_object(own, Zone::Exile, EventCause::effect())
                .unwrap();
            let returned = game
                .move_object(exile, Zone::Battlefield, EventCause::effect())
                .unwrap();
            assert_ne!(own, returned);
            assert_eq!(settle(&mut game), 1);
            assert!(
                game.battlefield.contains(&returned),
                "{name} must not follow a blink"
            );
            assert_eq!(
                game.object(equipment)
                    .unwrap_or_else(|| panic!(
                        "{name}: attachment itself must survive recipient blink"
                    ))
                    .attached_to,
                None
            );
        }
    }
}

#[test]
fn siona_entry_body_and_attachment_filter_snapshots_survive_later_control_changes() {
    use ironsmith::effects::MoveToZoneEffect;
    for definition in definitions("Siona, Captain of the Pyleas") {
        let mut game = game();
        let source_hand = game.create_object_from_definition(&definition, A, Zone::Hand);
        let stable = game.object(source_hand).unwrap().stable_id;
        let aura = resource(&mut game, A, Zone::Library, true);
        let aura_stable = game.object(aura).unwrap().stable_id;
        for _ in 0..6 {
            resource(&mut game, A, Zone::Library, false);
        }
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = EffectContext::new(source_hand, A, &mut dm);
        let outcome = MoveToZoneEffect::new(
            ChooseSpec::SpecificObject(source_hand),
            Zone::Battlefield,
            false,
        )
        .execute(&mut game, &mut ctx)
        .unwrap();
        for event in outcome.events {
            game.queue_trigger_event(event.provenance(), event);
        }
        assert_eq!(settle(&mut game), 1);
        let source = game.find_object_by_stable_id(stable).unwrap();
        let found_aura = game.find_object_by_stable_id(aura_stable).unwrap();
        assert!(game.player(A).unwrap().hand.contains(&found_aura));
        assert_eq!(game.player(A).unwrap().library.len(), 6);
        let recipient = resource(&mut game, A, Zone::Battlefield, false);
        let aura = resource(&mut game, A, Zone::Battlefield, true);
        attach(&mut game, aura, recipient, A);
        game.set_current_controller(aura, B).unwrap();
        game.set_current_controller(recipient, B).unwrap();
        assert_eq!(
            settle(&mut game),
            1,
            "both event filters use their attachment-time controllers"
        );
        assert_eq!(tokens(&game).len(), 1);
        assert_eq!(game.controller_of_id(source), Some(A));
    }
}

#[test]
fn stitchers_graft_attack_body_skips_exactly_the_next_controller_untap_step() {
    use ironsmith::combat_state::{AttackTarget, CombatState};
    use ironsmith::decision::AttackerDeclaration;
    for definition in definitions("Stitcher's Graft") {
        let mut game = game();
        let creature = resource(&mut game, A, Zone::Battlefield, false);
        let equipment = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        attach(&mut game, equipment, creature, A);
        settle(&mut game);
        game.remove_summoning_sickness(creature);
        game.turn.phase = Phase::Combat;
        game.turn.step = Some(ironsmith::game_state::Step::DeclareAttackers);
        game.mark_combat_phase_started();
        let mut queue = TriggerQueue::new();
        ironsmith::game_loop::apply_attacker_declarations(
            &mut game,
            &mut CombatState::default(),
            &mut queue,
            &[AttackerDeclaration {
                creature,
                target: AttackTarget::Player(B),
            }],
        )
        .unwrap();
        assert_eq!(queue.entries.len(), 1);
        put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut SelectFirstDecisionMaker)
            .unwrap();
        settle(&mut game);
        assert!(game.is_tapped(creature));
        for expected_tapped in [true, false] {
            game.next_turn();
            game.next_turn();
            game.turn.active_player = A;
            game.turn.phase = Phase::Beginning;
            game.turn.step = Some(ironsmith::game_state::Step::Untap);
            ironsmith::turn::execute_untap_step_with(&mut game, &mut SelectFirstDecisionMaker)
                .unwrap();
            assert_eq!(game.is_tapped(creature), expected_tapped);
        }
    }
}

#[test]
fn sba_unattachment_after_recipient_stops_being_a_creature_still_sacrifices_that_permanent() {
    for definition in definitions("Grafted Wargear") {
        let mut game = game();
        let creature = resource(&mut game, A, Zone::Battlefield, false);
        let equipment = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        attach(&mut game, equipment, creature, A);
        settle(&mut game);
        let modification = ironsmith::effects::ApplyContinuousEffect::new(
            ironsmith::continuous::EffectTarget::Specific(creature),
            ironsmith::continuous::Modification::SetCardTypes(vec![ironsmith::CardType::Artifact]),
            ironsmith::effect::Until::EndOfTurn,
        );
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = EffectContext::new(equipment, A, &mut dm);
        let outcome = modification.execute(&mut game, &mut ctx).unwrap();
        for event in outcome.events {
            game.queue_trigger_event(event.provenance(), event);
        }
        let mut queue = TriggerQueue::new();
        ironsmith::game_loop::check_and_apply_sbas_with(&mut game, &mut queue, &mut dm).unwrap();
        assert_eq!(
            game.object(equipment)
                .expect("attachment itself must survive recipient blink")
                .attached_to,
            None
        );
        assert_eq!(queue.entries.len(), 1);
        put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
        settle(&mut game);
        assert!(!game.battlefield.contains(&creature));
        assert!(game.battlefield.contains(&equipment));
    }
}
