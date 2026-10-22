//! Source-authored only. All scenarios remain UNRUN under the deferred gate.
use ironsmith::ability::AbilityKind;
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{
    DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, put_triggers_on_stack, resolve_stack_entry,
};
use ironsmith::game_state::{Phase, StackEntry, Step, TargetAssignment};
use ironsmith::mana::ManaSymbol;
use ironsmith::special_actions::{ActionError, SpecialAction, can_perform_check};
use ironsmith::triggers::TriggerQueue;
use ironsmith::{GameState, ObjectId, PlayerId, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);

fn announce_spell(game: &mut GameState, definition: &CardDefinition, player: PlayerId) -> ObjectId {
    let hand = game.create_object_from_definition(definition, player, Zone::Hand);
    announce_existing_spell(game, hand, player)
}

fn announce_existing_spell(game: &mut GameState, hand: ObjectId, player: PlayerId) -> ObjectId {
    game.turn.priority_player = Some(player);
    let stable = game.object(hand).unwrap().stable_id;
    let action = compute_legal_actions(game, player).unwrap().into_iter()
        .find(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == hand)).unwrap();
    let mut state = PriorityLoopState::new(game.players.len());
    let mut queue = TriggerQueue::new();
    let mut dm = SelectFirstDecisionMaker;
    let mut result = apply_priority_response_with_dm(game, &mut queue, &mut state,
        &PriorityResponse::PriorityAction(action), &mut dm).unwrap();
    for _ in 0..64 {
        if !state.has_pending_action() { return game.find_object_by_stable_id(stable).unwrap(); }
        let ironsmith::GameProgress::NeedsDecisionCtx(context) = result else { panic!("{result:?}"); };
        result = apply_decision_context_with_dm(game, &mut queue, &mut state, &context, &mut dm).unwrap();
    }
    panic!("cast announcement did not finish");
}

#[test]
fn holodeck_shares_one_use_across_origins_and_land_or_spell() {
    fn can_cast(game: &GameState, id: ObjectId) -> bool {
        compute_legal_actions(game, A).unwrap().iter().any(|action|
            matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == id))
    }
    for definition in definitions("The Holodeck") {
        for first_zone in [Zone::Graveyard, Zone::OutsideGame] {
            let other_zone = if first_zone == Zone::Graveyard { Zone::OutsideGame } else { Zone::Graveyard };
            for play_land in [false, true] {
                let mut game = game();
                game.create_object_from_definition(&definition, A, Zone::Battlefield);
                let first = resource(&mut game, A, "First permanent", "Mana cost: {0}\nType: Artifact", first_zone);
                let second = resource(&mut game, A, "Second permanent", "Mana cost: {0}\nType: Artifact", other_zone);
                let first_land = land(&mut game, A, "Permission land", first_zone);
                let opponent = resource(&mut game, B, "Opponent permanent", "Mana cost: {0}\nType: Artifact", other_zone);
                let instant = resource(&mut game, A, "Nonpermanent", "Mana cost: {0}\nType: Instant\nDraw a card.", other_zone);
                assert!(can_cast(&game, first));
                assert!(can_cast(&game, second));
                assert!(can_land(&game, A, first_land));
                assert!(!can_cast(&game, opponent));
                assert!(!can_cast(&game, instant));
                turn(&mut game, B);
                assert!(!can_cast(&game, first));
                turn(&mut game, A);
                if play_land {
                    ironsmith::special_actions::perform(SpecialAction::PlayLand { card_id: first_land }, &mut game, A, &mut SelectFirstDecisionMaker).unwrap();
                } else {
                    announce_existing_spell(&mut game, first, A);
                    resolve_stack_entry(&mut game).unwrap();
                }
                assert!(!can_cast(&game, second), "one allowance across both origins");
                game.turn_store.grant_cast_uses_this_turn.clear();
                assert!(can_cast(&game, second), "allowance resets next turn");
            }
        }
    }
}

#[test]
fn marooned_blocks_beam_payment_but_not_other_returns_to_hand() {
    for definition in definitions("Marooned") {
        for ordinary_bounce in [false, true] {
            let mut game = game();
            let protected = resource(&mut game, A, "Protected creature", "Type: Creature\nPower/Toughness: 2/2", Zone::Battlefield);
            let protected_stable = game.object(protected).unwrap().stable_id;
            let beam = game.create_object_from_definition(&definitions("Open Communications")[0], A, Zone::Graveyard);
            assert!(can_cast(&game, A, beam));
            let aura = game.create_object_from_definition(&definition, B, Zone::Battlefield);
            assert!(game.attach_object_to_target(aura, ironsmith::object::AttachmentTarget::Object(protected)));
            assert!(!can_cast(&game, A, beam), "the only creature cannot pay the beam cost");
            game.phase_out(aura);
            assert!(can_cast(&game, A, beam));
            game.phase_in(aura);
            assert!(!can_cast(&game, A, beam));
            if ordinary_bounce {
                use ironsmith::effects::{EffectContext, EffectExecutor, MoveToZoneEffect};
                MoveToZoneEffect::new(ironsmith::target::ChooseSpec::SpecificObject(protected), Zone::Hand, false)
                    .execute(&mut game, &mut EffectContext::new_default(aura, B)).unwrap();
                assert_eq!(game.object(game.find_object_by_stable_id(protected_stable).unwrap()).unwrap().zone, Zone::Hand);
            } else {
                let allowed = resource(&mut game, A, "Allowed creature", "Type: Creature\nPower/Toughness: 2/2", Zone::Battlefield);
                let allowed_stable = game.object(allowed).unwrap().stable_id;
                assert!(can_cast(&game, A, beam));
                announce_existing_spell(&mut game, beam, A);
                assert_eq!(game.object(game.find_object_by_stable_id(allowed_stable).unwrap()).unwrap().zone, Zone::Hand);
                assert_eq!(game.object(protected).unwrap().zone, Zone::Battlefield);
            }
        }
    }
}

#[test]
fn q_grants_exile_activation_and_copies_the_original_card_after_departure() {
    for definition in definitions("Q, Stacking the Deck") {
        for (departed, accept) in [(false, true), (true, true), (false, false)] {
            let mut game = game();
            let card = resource(&mut game, A, "Copied permanent", "Mana cost: {0}\nType: Artifact", Zone::Library);
            let stable = game.object(card).unwrap().stable_id;
            resource(&mut game, A, "Activation payment", "Type: Land", Zone::Hand);
            let source = game.create_object_from_definition(&definition, A, Zone::Hand);
            assert!(!game.move_object_with_etb_processing_with_dm(source, Zone::Battlefield, &mut SelectFirstDecisionMaker).unwrap().pending);
            put_triggers_on_stack(&mut game, &mut TriggerQueue::new()).unwrap();
            assert!(!game.stack_is_empty(), "missing Q entry trigger: {:?}", definition.abilities);
            settle(&mut game);
            let card = game.find_object_by_stable_id(stable).unwrap();
            assert_eq!(game.object(card).unwrap().zone, Zone::Exile);
            assert!(can_activate(&game, A, card));
            turn(&mut game, B);
            assert!(!can_activate(&game, A, card));
            turn(&mut game, A);
            announce_activation(&mut game, card, &mut SelectFirstDecisionMaker);
            assert!(game.player(A).unwrap().hand.is_empty(), "exiling a hand card pays the activation cost even if the copy is declined");
            if departed { game.move_object_by_effect(card, Zone::Graveyard).unwrap(); }
            if accept {
                ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
                assert_eq!(game.stack.len(), 1);
                resolve_stack_entry(&mut game).unwrap();
            } else { resolve_stack_entry(&mut game).unwrap(); }
            let copies = game.battlefield.iter().copied().filter(|id|
                game.object(*id).is_some_and(|object| object.name.as_str() == "Copied permanent")).collect::<Vec<_>>();
            assert_eq!(copies.len(), usize::from(accept));
            for copy in copies {
                assert_eq!(game.object(copy).unwrap().kind, ironsmith::object::ObjectKind::Token);
                assert!(!can_activate(&game, A, copy), "the granted exile ability is not copiable");
            }
            let original = game.find_object_by_stable_id(stable).unwrap();
            assert_eq!(game.object(original).unwrap().zone, if departed { Zone::Graveyard } else { Zone::Exile });
        }
    }
}

#[test]
fn source_card_copy_executor_uses_native_lki_and_pays_the_copy_cost() {
    use ironsmith::effects::{CastTaggedEffect, EffectContext, EffectExecutor};
    for departed in [false, true] {
        let mut game = game();
        let source = resource(&mut game, A, "Copied source", "Mana cost: {U}\nType: Artifact", Zone::Exile);
        let snapshot = ironsmith::snapshot::ObjectSnapshot::from_object(game.object(source).unwrap(), &game);
        let original = if departed {
            let next = game.move_object_by_effect(source, Zone::Graveyard).unwrap();
            game.object_mut(next).unwrap().name = "Different incarnation".into();
            next
        } else { source };
        let before = game.player(A).unwrap().mana_pool.blue;
        let mut dm = SelectFirstDecisionMaker;
        let mut context = EffectContext::new(source, A, &mut dm);
        context.source_snapshot = Some(snapshot);
        let outcome = CastTaggedEffect::new(ironsmith_core::tag::SOURCE_OBJECT_TAG, ironsmith::target::PlayerFilter::You)
            .as_copy().execute(&mut game, &mut context).unwrap();
        assert!(outcome.status.is_success(), "{outcome:?}");
        assert_eq!(game.player(A).unwrap().mana_pool.blue, before - 1);
        assert_eq!(game.stack.len(), 1);
        let copy = game.stack[0].object_id;
        assert_eq!(game.object(copy).unwrap().name.as_str(), "Copied source");
        assert_eq!(game.object(copy).unwrap().kind, ironsmith::object::ObjectKind::Token);
        assert_eq!(game.object(original).unwrap().zone, if departed { Zone::Graveyard } else { Zone::Exile });
        resolve_stack_entry(&mut game).unwrap();
        assert!(game.battlefield.iter().any(|id| game.object(*id).is_some_and(|object|
            object.name.as_str() == "Copied source" && object.kind == ironsmith::object::ObjectKind::Token)));
    }
}

#[test]
fn spock_checks_distinct_land_names_and_draws_only_the_unplayed_top_card() {
    for definition in definitions("Commander Spock") {
        for (land_names, card_text, accept, expected) in [
            (vec!["Alpha", "Beta"], "Mana cost: {2}\nType: Artifact", true, Zone::Battlefield),
            (vec!["Alpha", "Alpha"], "Mana cost: {2}\nType: Artifact", true, Zone::Hand),
            (vec!["Alpha", "Beta"], "Mana cost: {3}\nType: Artifact", true, Zone::Hand),
            (vec!["Alpha", "Beta"], "Mana cost: {2}\nType: Artifact", false, Zone::Hand),
            (vec![], "Mana cost: {0}\nType: Artifact", true, Zone::Battlefield),
            (vec![], "Type: Land", true, Zone::Battlefield),
            (vec![], "Type: Land", false, Zone::Hand),
        ] {
            let mut game = game();
            for name in &land_names { land(&mut game, A, name, Zone::Battlefield); }
            let top = resource(&mut game, A, "Revealed card", card_text, Zone::Library);
            let stable = game.object(top).unwrap().stable_id;
            let source = game.create_object_from_definition(&definition, A, Zone::Hand);
            assert!(!game.move_object_with_etb_processing_with_dm(source, Zone::Battlefield, &mut SelectFirstDecisionMaker).unwrap().pending);
            put_triggers_on_stack(&mut game, &mut TriggerQueue::new()).unwrap();
            assert!(!game.stack_is_empty(), "missing Spock entry trigger: {:?}", definition.abilities);
            for _ in 0..20 {
                put_triggers_on_stack(&mut game, &mut TriggerQueue::new()).unwrap();
                if game.stack_is_empty() { break; }
                if accept {
                    ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
                } else {
                    resolve_stack_entry(&mut game).unwrap();
                }
            }
            assert!(game.stack_is_empty());
            let card = game.find_object_by_stable_id(stable).unwrap();
            assert_eq!(game.object(card).unwrap().zone, expected, "{land_names:?}, {card_text}, accept={accept}");
            if expected == Zone::Battlefield && card_text == "Type: Land" { assert!(game.is_tapped(card)); }
            assert_eq!(game.player(A).unwrap().hand.len(), usize::from(expected == Zone::Hand));
        }
    }
}

#[test]
fn assimilation_grammar_handles_cards_and_battlefield_creatures() {
    use ironsmith::types::{CardType, Subtype};
    for (zone, text) in [
        (Zone::Graveyard, "Type: Sorcery\nAssimilate target creature card from an opponent's graveyard."),
        (Zone::Battlefield, "Type: Sorcery\nAssimilate target creature."),
    ] {
        let definition = compile_to_runtime_definition("Assimilation probe", text, false).unwrap();
        let mut game = game();
        let target = resource(&mut game, B, "Creature", "Type: Creature — Human\nPower/Toughness: 2/2", zone);
        let stable = game.object(target).unwrap().stable_id;
        cast_target_spell(&mut game, &definition, Target::Object(target));
        let assimilated = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.controller_of_id(assimilated), Some(A));
        assert!(game.current_has_card_type(assimilated, CardType::Artifact));
        assert!(game.current_has_subtype(assimilated, Subtype::Borg));
        assert!(!game.current_has_subtype(assimilated, Subtype::Human));
        assert_eq!(game.counter_count(assimilated, ironsmith::CounterType::PlusOnePlusOne), 1);
    }
}

#[test]
fn assimilation_full_cards_survive_artifact_roundtrip() {
    for name in ["Borg Cube", "Borg Queen, Perfection Manifest", "Death Is Irrelevant", "The Borg Queen"] {
        for definition in definitions(name) {
            let text = ironsmith_text::compiled_text::compiled_text_lines(&definition).join("\n");
            assert!(text.to_lowercase().contains("assimilate"), "{name}: {text}");
        }
    }
}

#[test]
fn assimilation_real_graveyard_triggers_apply_entry_changes_and_static_bonuses() {
    for name in ["Borg Queen, Perfection Manifest", "Death Is Irrelevant"] {
        for definition in definitions(name) {
            let mut game = game();
            let target = resource(&mut game, B, "Assimilation target",
                "Type: Creature — Human\nPower/Toughness: 2/2", Zone::Graveyard);
            let stable = game.object(target).unwrap().stable_id;
            let source = game.create_object_from_definition(&definition, A, Zone::Hand);
            assert!(!game.move_object_with_etb_processing_with_dm(source, Zone::Battlefield, &mut SelectFirstDecisionMaker).unwrap().pending);
            if name == "Death Is Irrelevant" {
                game.queue_trigger_event(Default::default(), ironsmith::triggers::TriggerEvent::new_with_provenance(
                    ironsmith::events::BeginningOfEndStepEvent::new(A), Default::default()));
            }
            ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut TriggerQueue::new(), &mut TargetChoice(Target::Object(target))).unwrap();
            settle(&mut game);
            let arrived = game.find_object_by_stable_id(stable).unwrap();
            assert_eq!(game.object(arrived).unwrap().zone, Zone::Battlefield, "{name}");
            assert_eq!(game.controller_of_id(arrived), Some(A));
            assert!(game.current_has_card_type(arrived, ironsmith::types::CardType::Artifact));
            assert!(game.current_has_subtype(arrived, ironsmith::types::Subtype::Borg));
            assert!(!game.current_has_subtype(arrived, ironsmith::types::Subtype::Human));
            assert_eq!(game.counter_count(arrived, ironsmith::CounterType::PlusOnePlusOne), 1);
            if name == "Borg Queen, Perfection Manifest" {
                assert_eq!(game.current_power(arrived), Some(5));
            }
        }
    }
}

#[test]
fn assimilation_library_trigger_uses_exiled_creature_and_returns_the_rest() {
    for definition in definitions("The Borg Queen") {
        let mut game = game();
        let target = resource(&mut game, B, "Library creature",
            "Type: Creature — Human\nPower/Toughness: 2/2", Zone::Library);
        let stable = game.object(target).unwrap().stable_id;
        library(&mut game, B, 3);
        let source = game.create_object_from_definition(&definition, A, Zone::Hand);
        assert!(!game.move_object_with_etb_processing_with_dm(source, Zone::Battlefield, &mut SelectFirstDecisionMaker).unwrap().pending);
        ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut TriggerQueue::new(), &mut TargetChoice(Target::Player(B))).unwrap();
        settle(&mut game);
        let arrived = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.object(arrived).unwrap().zone, Zone::Battlefield);
        assert_eq!(game.controller_of_id(arrived), Some(A));
        assert!(game.current_has_subtype(arrived, ironsmith::types::Subtype::Borg));
        assert_eq!(game.current_power(arrived), Some(5));
        assert_eq!(game.player(B).unwrap().library.len(), 3);
        assert_eq!(game.counter_count(arrived, ironsmith::CounterType::PlusOnePlusOne), 1);
    }
}

#[test]
fn assimilation_cube_exiles_only_when_no_linked_card_can_be_assimilated() {
    for definition in definitions("Borg Cube") {
        let mut game = game();
        let target = resource(&mut game, B, "Captured creature",
            "Type: Creature — Human\nPower/Toughness: 2/2", Zone::Battlefield);
        let stable = game.object(target).unwrap().stable_id;
        let source = game.create_object_from_definition(&definition, A, Zone::Hand);
        let source_stable = game.object(source).unwrap().stable_id;
        assert!(!game.move_object_with_etb_processing_with_dm(source, Zone::Battlefield, &mut SelectFirstDecisionMaker).unwrap().pending);
        let cube = game.find_object_by_stable_id(source_stable).unwrap();
        ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut TriggerQueue::new(), &mut TargetChoice(Target::Object(target))).unwrap();
        settle(&mut game);
        let exiled = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.object(exiled).unwrap().zone, Zone::Exile);
        game.queue_trigger_event(Default::default(), ironsmith::triggers::TriggerEvent::new_with_provenance(
            ironsmith::events::CreatureAttackedEvent::new(cube, ironsmith::events::AttackEventTarget::Player(B)), Default::default()));
        // The fallback's optional target may be omitted when a linked card exists.
        settle(&mut game);
        let arrived = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.object(arrived).unwrap().zone, Zone::Battlefield);
        assert_eq!(game.controller_of_id(arrived), Some(A));
        assert!(game.current_has_subtype(arrived, ironsmith::types::Subtype::Borg));
        assert_eq!(game.counter_count(arrived, ironsmith::CounterType::PlusOnePlusOne), 1);
    }
}

#[test]
fn entry_type_changes_apply_before_replacements_and_do_not_survive_blink() {
    use ironsmith::effects::{EffectContext, EffectExecutor, MoveToZoneEffect};
    use ironsmith::types::{CardType, Subtype, SubtypeFamily, Supertype};
    use ironsmith::target::ChooseSpec;
    let mut game = game();
    let source = resource(&mut game, A, "Entry source", "Type: Artifact", Zone::Battlefield);
    resource(&mut game, A, "Typed entry rule", "Type: Enchantment\nBorg creatures you control enter tapped.", Zone::Battlefield);
    resource(&mut game, A, "Typed counter rule", "Type: Enchantment\nIf one or more +1/+1 counters would be put on an artifact creature you control, twice that many +1/+1 counters are put on it instead.", Zone::Battlefield);
    let original = resource(&mut game, B, "Original creature",
        "Type: Legendary Enchantment Creature — Shrine Human\nPower/Toughness: 2/2", Zone::Graveyard);
    let stable = game.object(original).unwrap().stable_id;
    let mut movement = MoveToZoneEffect::new(ChooseSpec::SpecificObject(original), Zone::Battlefield, false).under_you_control()
        .with_entry_counter(ironsmith_core::BattlefieldEntryCounterSpec::new(
            ironsmith::CounterType::PlusOnePlusOne, 1, ironsmith_core::BattlefieldEntryCounterSurface::Inline));
    movement.entry_type_changes = Some(ironsmith_core::effect::BattlefieldEntryTypeChanges {
        added_card_types: vec![CardType::Artifact, CardType::Creature],
        removed_subtype_families: vec![SubtypeFamily::Creature],
        added_subtypes: vec![Subtype::Borg],
    });
    let wire = serde_json::to_value(&movement).unwrap();
    let restored: MoveToZoneEffect = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(restored, movement);
    let mut legacy = wire;
    legacy.as_object_mut().unwrap().remove("entry_type_changes");
    assert!(serde_json::from_value::<MoveToZoneEffect>(legacy).unwrap().entry_type_changes.is_none());
    restored.execute(&mut game, &mut EffectContext::new_default(source, A)).unwrap();
    let arrived = game.find_object_by_stable_id(stable).unwrap();
    assert_eq!(game.controller_of_id(arrived), Some(A));
    for kind in [CardType::Enchantment, CardType::Creature, CardType::Artifact] {
        assert!(game.current_has_card_type(arrived, kind));
    }
    assert!(game.current_has_supertype(arrived, Supertype::Legendary));
    assert!(game.current_has_subtype(arrived, Subtype::Shrine));
    assert!(game.current_has_subtype(arrived, Subtype::Borg));
    assert!(!game.current_has_subtype(arrived, Subtype::Human));
    assert!(game.is_tapped(arrived), "entry replacement must see the new creature type");
    assert_eq!(game.counter_count(arrived, ironsmith::CounterType::PlusOnePlusOne), 2);
    let exiled = game.move_object_by_effect(arrived, Zone::Exile).unwrap();
    MoveToZoneEffect::new(ChooseSpec::SpecificObject(exiled), Zone::Battlefield, false)
        .execute(&mut game, &mut EffectContext::new_default(source, A)).unwrap();
    let returned = game.find_object_by_stable_id(stable).unwrap();
    assert!(game.current_has_subtype(returned, Subtype::Human));
    assert!(!game.current_has_subtype(returned, Subtype::Borg));
    assert!(!game.current_has_card_type(returned, CardType::Artifact));
}

#[test]
fn dominion_copy_preserves_counter_kinds_before_entry_replacements() {
    struct CopyChoice(bool);
    impl DecisionMaker for CopyChoice {
        fn decide_options(&mut self, _: &GameState, context: &ironsmith::decisions::context::SelectOptionsContext) -> Vec<usize> {
            let legal: Vec<_> = context.options.iter().filter(|option| option.legal).collect();
            let selected = if self.0 { legal.last() } else { legal.first() };
            selected.map(|option| vec![option.index]).unwrap_or_default()
        }
    }
    for definition in definitions("Dominion Saboteur") {
        for (copy, doubled) in [(true, false), (true, true), (false, false)] {
            let mut game = game();
            let original = resource(&mut game, B, "Copied creature",
                "Type: Legendary Artifact Creature — Golem\nPower/Toughness: 2/2\nThis creature enters with a shield counter on it.", Zone::Battlefield);
            game.add_counters(original, ironsmith::CounterType::Charge, 2);
            game.add_counters(original, ironsmith::CounterType::PlusOnePlusOne, 3);
            if doubled {
                resource(&mut game, A, "Entry doubler", "Type: Enchantment\nIf one or more counters would be put on a permanent you control, twice that many of those counters are put on it instead.", Zone::Battlefield);
            }
            let entering = game.create_object_from_definition(&definition, A, Zone::Hand);
            let stable = game.object(entering).unwrap().stable_id;
            let result = game.move_object_with_etb_processing_with_dm(entering, Zone::Battlefield, &mut CopyChoice(copy)).unwrap();
            assert!(!result.pending);
            let arrived = game.find_object_by_stable_id(stable).unwrap();
            assert_eq!(game.object(arrived).unwrap().zone, Zone::Battlefield);
            let factor = if doubled { 2 } else { 1 };
            for (kind, count) in [(ironsmith::CounterType::Charge, 2), (ironsmith::CounterType::PlusOnePlusOne, 3), (ironsmith::CounterType::Shield, 1)] {
                assert_eq!(game.counter_count(arrived, kind), if copy { count * factor } else { 0 }, "copy={copy}, doubled={doubled}, {kind:?}");
            }
            assert!(!game.object(arrived).unwrap().supertypes.contains(&ironsmith::types::Supertype::Legendary));
            assert_eq!(game.counter_count(original, ironsmith::CounterType::Charge), 2);
            assert_eq!(game.counter_count(original, ironsmith::CounterType::PlusOnePlusOne), 3);
        }
    }
}

#[test]
fn first_contact_uses_its_own_full_game_cast_position() {
    let probe = compile_to_runtime_definition("Cast probe", "Mana cost: {0}\nType: Instant\nYou gain 0 life.", false).unwrap();
    for definition in definitions("First Contact") {
        for scenario in 0..5 {
            let mut game = game();
            library(&mut game, A, 8);
            if scenario == 1 || scenario == 2 || scenario == 3 {
                announce_spell(&mut game, &probe, if scenario == 2 { B } else { A });
                settle(&mut game);
                if scenario == 3 { game.turn_store.turn_history.clear_for_new_turn(); }
            }
            let source = announce_spell(&mut game, &definition, A);
            if scenario == 4 {
                announce_spell(&mut game, &probe, A);
                resolve_stack_entry(&mut game).unwrap();
            }
            assert_eq!(ironsmith::condition_eval::evaluate_condition_resolution(&game,
                &ironsmith::effect::Condition::ThisSpellCastOrderThisGame(1),
                &ironsmith::effects::EffectContext::new_default(source, A)).unwrap(),
                matches!(scenario, 0 | 2 | 4));
            settle(&mut game);
            assert_eq!(game.player(A).unwrap().life, if matches!(scenario, 0 | 2 | 4) { 22 } else { 20 }, "scenario {scenario}");
        }
        let mut game = game();
        library(&mut game, A, 8);
        let uncast = game.create_object_from_definition(&definition, A, Zone::Stack);
        game.push_to_stack(StackEntry::new(uncast, A));
        settle(&mut game);
        assert_eq!(game.player(A).unwrap().life, 20, "an uncast copy has no cast ordinal");
    }
}

#[test]
fn kirk_title_and_named_search_survive_artifact_roundtrip() {
    for definition in definitions("Captain Kirk, Boldly Going") {
        let rendered = ironsmith_text::compiled_text::compiled_text_lines(&definition).join("\n");
        assert!(rendered.contains("named Starship Enterprise"), "{rendered}");
        assert!(rendered.contains("basic land"), "{rendered}");
    }
    for definition in definitions("Captain James T. Kirk") {
        assert!(!definition.abilities.is_empty());
    }
}

#[test]
fn temporary_counter_executor_preserves_explicit_and_legacy_actor_scope() {
    use ironsmith::effects::{EffectContext, EffectExecutor, PutCountersEffect, RegisterCounterPlacementReplacementEffect, ReplacementApplyMode};
    use ironsmith::target::{ChooseSpec, ObjectFilter, PlayerFilter};
    for actor_scope in [Some(PlayerFilter::You), None] {
        let mut game = game();
        let source = resource(&mut game, A, "Source", "Type: Artifact", Zone::Battlefield);
        let recipient = resource(&mut game, A, "Recipient", "Type: Artifact", Zone::Battlefield);
        let mut replacement = RegisterCounterPlacementReplacementEffect::new(
            ObjectFilter::permanent().you_control(), None, 1, ReplacementApplyMode::UntilEndOfTurn);
        let mut legacy = serde_json::to_value(&replacement).unwrap();
        legacy.as_object_mut().unwrap().remove("actor");
        let restored: RegisterCounterPlacementReplacementEffect = serde_json::from_value(legacy).unwrap();
        assert_eq!(restored.actor, Some(PlayerFilter::You));
        replacement.actor = actor_scope.clone();
        replacement.execute(&mut game, &mut EffectContext::new_default(source, A)).unwrap();
        for actor in [A, B] {
            let before = game.counter_count(recipient, ironsmith::CounterType::Charge);
            PutCountersEffect::new(ironsmith::CounterType::Charge, 1, ChooseSpec::SpecificObject(recipient))
                .execute(&mut game, &mut EffectContext::new_default(source, actor)).unwrap();
            assert_eq!(game.counter_count(recipient, ironsmith::CounterType::Charge) - before,
                if actor == A || actor_scope.is_none() { 2 } else { 1 });
        }
    }
}

#[test]
fn brad_temporary_counter_addition_survives_source_and_expires_at_cleanup() {
    use ironsmith::effects::{EffectContext, EffectExecutor, PutCountersEffect};
    let tap = compile_to_runtime_definition("Tap probe", "Type: Instant\nTap target creature.", false).unwrap();
    for definition in definitions("Brad Boimler, Eager Ensign") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let own = resource(&mut game, A, "Own artifact", "Type: Artifact", Zone::Battlefield);
        let opponent = resource(&mut game, B, "Opponent artifact", "Type: Artifact", Zone::Battlefield);
        let place = |game: &mut GameState, target, actor, kind, amount| {
            let before = game.counter_count(target, kind);
            let mut ctx = EffectContext::new_default(own, actor);
            PutCountersEffect::new(kind, amount, ironsmith::target::ChooseSpec::SpecificObject(target))
                .execute(game, &mut ctx).unwrap();
            game.counter_count(target, kind) - before
        };
        assert_eq!(place(&mut game, own, A, ironsmith::CounterType::Charge, 1), 1);
        cast_target_spell(&mut game, &tap, Target::Object(source));
        settle(&mut game);
        for kind in [ironsmith::CounterType::Charge, ironsmith::CounterType::Shield] {
            for actor in [A, B] {
                assert_eq!(place(&mut game, own, actor, kind, 1), 2);
                assert_eq!(place(&mut game, opponent, actor, kind, 1), 1);
            }
        }
        assert_eq!(place(&mut game, own, A, ironsmith::CounterType::Charge, 0), 0);
        game.move_object_by_effect(source, Zone::Graveyard).unwrap();
        assert_eq!(place(&mut game, own, B, ironsmith::CounterType::Charge, 1), 2);
        ironsmith::turn::execute_cleanup_step(&mut game);
        assert_eq!(place(&mut game, own, A, ironsmith::CounterType::Charge, 1), 1);
    }
}

#[test]
fn miles_return_history_checks_owner_zones_and_current_turn() {
    for definition in definitions("Miles O'Brien, Transporter Ops") {
        for (owner, controller, origin, destination, new_turn, draws) in [
            (A, A, Zone::Battlefield, Zone::Hand, false, true),
            (A, B, Zone::Battlefield, Zone::Hand, false, true),
            (B, A, Zone::Battlefield, Zone::Hand, false, false),
            (A, A, Zone::Graveyard, Zone::Hand, false, false),
            (A, A, Zone::Battlefield, Zone::Exile, false, false),
            (A, A, Zone::Battlefield, Zone::Hand, true, false),
        ] {
            let mut game = game();
            game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let moved = resource(&mut game, owner, "Moved permanent",
                "Type: Artifact Creature\nPower/Toughness: 1/1", origin);
            if origin == Zone::Battlefield { game.set_current_controller(moved, controller).unwrap(); }
            game.move_object_by_effect(moved, destination).unwrap();
            settle(&mut game);
            if new_turn { game.turn_store.turn_history.clear_for_new_turn(); }
            library(&mut game, A, 2);
            let before = game.player(A).unwrap().hand.len();
            game.queue_trigger_event(Default::default(), ironsmith::triggers::TriggerEvent::new_with_provenance(
                ironsmith::events::BeginningOfEndStepEvent::new(B), Default::default()));
            settle(&mut game);
            assert_eq!(game.player(A).unwrap().hand.len(), before + usize::from(draws),
                "owner={owner:?}, controller={controller:?}, {origin:?}->{destination:?}, new turn={new_turn}");
        }
    }
}

#[test]
fn beverly_duplicates_only_own_permanent_triggers_caused_by_own_life_gain() {
    for definition in definitions("Dr. Beverly Crusher") {
        let rendered = ironsmith_text::compiled_text::compiled_text_lines(&definition).join("\n");
        assert!(rendered.contains("on Dr. Crusher."), "{rendered}");
        for (recipient, amount, phased) in [(A, 1, false), (A, 3, false), (B, 3, false), (A, 3, true)] {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let watcher = "Type: Creature\nPower/Toughness: 1/1\nWhenever a player gains life, put a charge counter on this creature.";
            let own = resource(&mut game, A, "Own watcher", watcher, Zone::Battlefield);
            let opponent = resource(&mut game, B, "Opponent watcher", watcher, Zone::Battlefield);
            let unrelated = resource(&mut game, A, "Tap watcher",
                "Type: Creature\nPower/Toughness: 1/1\nWhenever this creature becomes tapped, put a charge counter on it.", Zone::Battlefield);
            if phased { game.phase_out(source); }
            let gain = compile_to_runtime_definition("Life gain probe",
                &format!("Type: Instant\nTarget player gains {amount} life."), false).unwrap();
            cast_player_spell(&mut game, &gain, recipient);
            settle(&mut game);
            let doubled = recipient == A && !phased;
            assert_eq!(game.counter_count(own, ironsmith::CounterType::Charge), if doubled { 2 } else { 1 });
            assert_eq!(game.counter_count(opponent, ironsmith::CounterType::Charge), 1);
            assert_eq!(game.counter_count(source, ironsmith::CounterType::PlusOnePlusOne), if doubled { 2 } else { 0 });
            let tap = compile_to_runtime_definition("Tap probe", "Type: Instant\nTap target creature.", false).unwrap();
            cast_target_spell(&mut game, &tap, Target::Object(unrelated));
            settle(&mut game);
            assert_eq!(game.counter_count(unrelated, ironsmith::CounterType::Charge), 1);
        }
    }
}

#[test]
fn beckett_counter_threshold_removes_only_the_named_counters() {
    let tap = compile_to_runtime_definition("Tap probe", "Type: Instant\nTap target creature.", false).unwrap();
    for definition in definitions("Beckett Mariner, Impetuous Ensign") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let promotion = ironsmith::CounterType::Named("promotion".into());
        game.add_counters(source, ironsmith::CounterType::Charge, 2);
        for (expected, life) in [(1, 20), (2, 20), (0, 18), (1, 18)] {
            game.untap(source);
            cast_target_spell(&mut game, &tap, Target::Object(source));
            settle(&mut game);
            assert_eq!(game.counter_count(source, promotion), expected);
            assert_eq!(game.counter_count(source, ironsmith::CounterType::Charge), 2);
            assert_eq!(game.player(B).unwrap().life, life);
            assert_eq!(game.player(A).unwrap().life, 20);
        }
    }
}

#[test]
fn guinan_counts_counters_actually_placed_by_this_activation() {
    for definition in definitions("Guinan, Ancient Advisor") {
        let rendered = ironsmith_text::compiled_text::compiled_text_lines(&definition).join("\n");
        assert!(rendered.contains("the number of +1/+1 counters put on creatures this way"), "{rendered}");
        for (entrants, double) in [(0, false), (1, false), (2, false), (1, true)] {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            game.remove_summoning_sickness(source);
            if double {
                resource(&mut game, A, "Counter replacement",
                    "Type: Enchantment\nIf you would put one or more +1/+1 counters on a creature you control, put twice that many +1/+1 counters on that creature instead.",
                    Zone::Battlefield);
            }
            game.turn_store.turn_history.clear_for_new_turn();
            let mut recipients = Vec::new();
            for _ in 0..entrants {
                let id = resource(&mut game, A, "New creature",
                    "Type: Creature — Human\nPower/Toughness: 1/1", Zone::Hand);
                recipients.push(game.move_object_by_effect(id, Zone::Battlefield).unwrap());
            }
            let opponent = resource(&mut game, B, "Opponent creature",
                "Type: Creature — Human\nPower/Toughness: 1/1", Zone::Hand);
            let opponent = game.move_object_by_effect(opponent, Zone::Battlefield).unwrap();
            library(&mut game, A, 2);
            activate(&mut game, source, None);
            settle(&mut game);
            let amount = if double { 2 } else { 1 };
            for id in recipients {
                assert_eq!(game.object(id).unwrap().counters.get(&ironsmith::CounterType::PlusOnePlusOne).copied().unwrap_or(0), amount);
            }
            assert!(game.object(source).unwrap().counters.is_empty());
            assert!(game.object(opponent).unwrap().counters.is_empty());
            assert_eq!(game.player(A).unwrap().hand.len(), usize::from(entrants * amount >= 2));
        }
    }
}

#[test]
fn gowron_uses_the_sacrificed_cost_creatures_last_known_subtype() {
    for definition in definitions("Gowron, Eager for Battle") {
        for klingon in [false, true] {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let subtype = if klingon { "Klingon" } else { "Human" };
            let fodder = resource(&mut game, A, "Sacrifice candidate",
                &format!("Type: Creature — {subtype}\nPower/Toughness: 1/1"), Zone::Battlefield);
            let stable = game.object(fodder).unwrap().stable_id;
            library(&mut game, A, 2);
            activate(&mut game, source, None);
            settle(&mut game);
            let sacrificed = game.find_object_by_stable_id(stable).unwrap();
            assert_eq!(game.object(sacrificed).unwrap().zone, Zone::Graveyard);
            assert_eq!(game.player(A).unwrap().hand.len(), usize::from(klingon));
            assert_eq!(game.player(A).unwrap().life, if klingon { 19 } else { 20 });
        }
    }
}

fn game() -> GameState {
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    turn(&mut game, A);
    for player in [A, B] {
        for mana in [
            ManaSymbol::White,
            ManaSymbol::Blue,
            ManaSymbol::Black,
            ManaSymbol::Red,
            ManaSymbol::Green,
            ManaSymbol::Colorless,
        ] {
            game.player_mut(player).unwrap().mana_pool.add(mana, 20);
        }
    }
    game
}
fn turn(game: &mut GameState, player: PlayerId) {
    game.turn.active_player = player;
    game.turn.priority_player = Some(player);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
}
fn rows() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/scoped_action_prohibitions.json.fixture"
    ))
    .unwrap()
}
fn definitions(name: &str) -> [CardDefinition; 2] {
    let row = rows().into_iter().find(|row| row["name"] == name).unwrap();
    let mut text = format!(
        "Mana cost: {}\nType: {}\n",
        row["mana_cost"].as_str().unwrap(),
        row["type_line"].as_str().unwrap()
    );
    if let (Some(p), Some(t)) = (row["power"].as_str(), row["toughness"].as_str()) {
        text.push_str(&format!("Power/Toughness: {p}/{t}\n"));
    }
    text.push_str(row["oracle_text"].as_str().unwrap());
    let (compiled, loss) =
        ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, text, false));
    let (artifact, direct) = compiled.unwrap_or_else(|error| panic!("{name}: {error}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    artifact.validate().unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    [direct, materialize_artifact(&restored).unwrap()]
}
fn resource(
    game: &mut GameState,
    player: PlayerId,
    name: &str,
    text: &str,
    zone: Zone,
) -> ObjectId {
    let definition = compile_to_runtime_definition(name, text, false).unwrap();
    game.create_object_from_definition(&definition, player, zone)
}
fn land(game: &mut GameState, player: PlayerId, name: &str, zone: Zone) -> ObjectId {
    resource(game, player, name, "Type: Land", zone)
}
fn library(game: &mut GameState, player: PlayerId, count: usize) {
    for index in 0..count {
        resource(
            game,
            player,
            &format!("Library {index}"),
            "Mana cost: {0}\nType: Artifact",
            Zone::Library,
        );
    }
}
fn can_land(game: &GameState, player: PlayerId, id: ObjectId) -> bool {
    can_perform_check(&SpecialAction::PlayLand { card_id: id }, game, player).is_ok()
}
fn can_cast(game: &GameState, player: PlayerId, id: ObjectId) -> bool {
    compute_legal_actions(game, player)
        .unwrap()
        .iter()
        .any(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == id))
}
fn can_activate(game: &GameState, player: PlayerId, id: ObjectId) -> bool {
    compute_legal_actions(game, player).unwrap().iter().any(
        |action| matches!(action, LegalAction::ActivateAbility { source, .. } if *source == id),
    )
}
fn settle(game: &mut GameState) {
    for _ in 0..20 {
        put_triggers_on_stack(game, &mut TriggerQueue::new()).unwrap();
        if game.stack_is_empty() {
            return;
        }
        resolve_stack_entry(game).unwrap();
    }
    panic!("unexpected repeated stack program");
}
struct TargetChoice(Target);
impl DecisionMaker for TargetChoice {
    fn decide_targets(
        &mut self,
        _: &GameState,
        context: &ironsmith::decisions::context::TargetsContext,
    ) -> Vec<Target> {
        assert_eq!(context.requirements.len(), 1);
        assert!(context.requirements[0].legal_targets.contains(&self.0));
        vec![self.0]
    }
}
fn activate(game: &mut GameState, source: ObjectId, target: Option<Target>) {
    let mut target_dm = TargetChoice(target.unwrap_or(Target::Player(A)));
    announce_activation(game, source, &mut target_dm);
    resolve_stack_entry(game).unwrap();
}
fn announce_activation(game: &mut GameState, source: ObjectId, dm: &mut impl DecisionMaker) {
    let action = compute_legal_actions(game, A).unwrap().into_iter().find(|action| matches!(action, LegalAction::ActivateAbility { source: candidate, .. } if *candidate == source)).unwrap();
    let mut state = PriorityLoopState::new(game.players.len());
    let mut queue = TriggerQueue::new();
    let mut result = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    )
    .unwrap();
    for _ in 0..20 {
        if state.pending_activation.is_none() {
            break;
        }
        if let ironsmith::GameProgress::NeedsDecisionCtx(context) = result {
            result =
                apply_decision_context_with_dm(game, &mut queue, &mut state, &context, dm).unwrap();
        } else {
            break;
        }
    }
    assert!(state.pending_activation.is_none());
}
fn cast_player_spell(game: &mut GameState, definition: &CardDefinition, target: PlayerId) {
    cast_target_spell(game, definition, Target::Player(target));
}
fn cast_target_spell(game: &mut GameState, definition: &CardDefinition, target: Target) {
    let source = game.create_object_from_definition(definition, A, Zone::Stack);
    let requirements = ironsmith::game_loop::extract_target_requirements_from_program_with_modes(
        game,
        definition.spell_effect.as_ref().unwrap(),
        A,
        Some(source),
        None,
    );
    assert_eq!(requirements.len(), 1);
    assert!(
        requirements[0]
            .legal_targets
            .contains(&target)
    );
    game.push_to_stack(
        StackEntry::new(source, A)
            .with_targets(vec![target])
            .with_target_assignments(vec![TargetAssignment {
                spec: requirements[0].spec.clone(),
                range: 0..1,
            }]),
    );
    resolve_stack_entry(game).unwrap();
}
#[test]
fn fourteen_full_oracle_bodies_preserve_direct_and_restored_artifacts() {
    for row in rows() {
        for definition in definitions(row["name"].as_str().unwrap()) {
            assert_eq!(definition.card.name, row["name"].as_str().unwrap());
        }
    }
}
#[test]
fn mining_bans_only_its_controller_tracks_host_lifetime_and_keeps_sacrifice_draw_cost() {
    for definition in definitions("Aggressive Mining") {
        let mut game = game();
        library(&mut game, A, 3);
        let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let own = land(&mut game, A, "Own land", Zone::Hand);
        let other = land(&mut game, B, "Other land", Zone::Hand);
        assert_eq!(
            can_perform_check(&SpecialAction::PlayLand { card_id: own }, &game, A),
            Err(ActionError::LandPlayProhibited)
        );
        turn(&mut game, B);
        assert!(can_land(&game, B, other));
        turn(&mut game, A);
        game.phase_out(host);
        assert!(can_land(&game, A, own));
        game.phase_in(host);
        assert!(!can_land(&game, A, own));
        let cost_land = land(&mut game, A, "Sacrificed land", Zone::Battlefield);
        activate(&mut game, host, None);
        assert!(game.object(cost_land).is_none());
        assert_eq!(game.player(A).unwrap().hand.len(), 3);
        land(&mut game, A, "Second cost land", Zone::Battlefield);
        assert!(
            !can_activate(&game, A, host),
            "once each turn survives the legality change"
        );
        game.move_object_by_effect(host, Zone::Graveyard).unwrap();
        assert!(can_land(&game, A, own));
    }
}
#[test]
fn miner_registration_survives_its_sacrifice_and_binds_the_announced_player_until_cleanup() {
    for definition in definitions("Pardic Miner") {
        let mut game = game();
        let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let own = land(&mut game, A, "Own", Zone::Hand);
        let other = land(&mut game, B, "Other", Zone::Hand);
        activate(&mut game, host, Some(Target::Player(B)));
        assert!(game.object(host).is_none());
        assert!(can_land(&game, A, own));
        turn(&mut game, B);
        assert!(!can_land(&game, B, other));
        let later = land(&mut game, B, "Acquired later", Zone::Hand);
        assert!(!can_land(&game, B, later));
        game.cleanup_restrictions_end_of_turn();
        assert!(can_land(&game, B, other));
        assert!(can_land(&game, B, later));
    }
}
#[test]
fn turf_wound_draws_for_its_caster_and_solfatara_delays_its_draw_until_the_next_upkeep() {
    for name in ["Turf Wound", "Solfatara"] {
        for definition in definitions(name) {
            let mut game = game();
            library(&mut game, A, 3);
            let other = land(&mut game, B, "Restricted land", Zone::Hand);
            cast_player_spell(&mut game, &definition, B);
            assert_eq!(
                game.player(A).unwrap().hand.len(),
                usize::from(name == "Turf Wound")
            );
            turn(&mut game, B);
            assert!(!can_land(&game, B, other));
            game.cleanup_restrictions_end_of_turn();
            assert!(can_land(&game, B, other));
            if name == "Solfatara" {
                game.turn.turn_number += 1;
                game.turn.phase = Phase::Beginning;
                game.turn.step = Some(Step::Upkeep);
                let mut queue = TriggerQueue::new();
                ironsmith::game_loop::generate_and_queue_step_triggers(&mut game, &mut queue);
                put_triggers_on_stack(&mut game, &mut queue).unwrap();
                settle(&mut game);
                assert_eq!(game.player(A).unwrap().hand.len(), 1);
                assert_eq!(game.player(B).unwrap().hand, vec![other]);
            }
        }
    }
}
#[test]
fn frenzy_bans_both_hand_actions_but_allows_the_actual_top_library_card() {
    for definition in definitions("Experimental Frenzy") {
        let mut game = game();
        let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let hand_land = land(&mut game, A, "Hand land", Zone::Hand);
        let hand_spell = resource(
            &mut game,
            A,
            "Hand spell",
            "Mana cost: {0}\nType: Artifact",
            Zone::Hand,
        );
        let top_land = land(&mut game, A, "Top land", Zone::Library);
        assert!(!can_land(&game, A, hand_land));
        assert!(!can_cast(&game, A, hand_spell));
        assert!(can_land(&game, A, top_land));
        game.move_object_by_effect(top_land, Zone::Graveyard)
            .unwrap();
        let top_spell = resource(
            &mut game,
            A,
            "Top spell",
            "Mana cost: {0}\nType: Artifact",
            Zone::Library,
        );
        assert!(can_cast(&game, A, top_spell));
        activate(&mut game, host, None);
        assert!(game.object(host).is_none());
        assert!(can_land(&game, A, hand_land));
        assert!(can_cast(&game, A, hand_spell));
        assert!(
            !can_cast(&game, A, top_spell),
            "the source's permission leaves with it"
        );
    }
}
#[test]
fn cornered_market_uses_current_nontoken_names_and_only_nonbasic_lands() {
    for definition in definitions("Cornered Market") {
        let mut game = game();
        game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let permanent = resource(
            &mut game,
            B,
            "Shared name",
            "Mana cost: {0}\nType: Artifact",
            Zone::Battlefield,
        );
        let spell = resource(
            &mut game,
            A,
            "Shared name",
            "Mana cost: {0}\nType: Artifact",
            Zone::Hand,
        );
        let land = land(&mut game, A, "Shared name", Zone::Hand);
        let basic = resource(&mut game, A, "Shared name", "Type: Basic Land", Zone::Hand);
        assert!(!can_cast(&game, A, spell));
        assert!(!can_land(&game, A, land));
        assert!(can_land(&game, A, basic));
        game.phase_out(permanent);
        assert!(can_cast(&game, A, spell));
        assert!(can_land(&game, A, land));
        game.phase_in(permanent);
        assert!(!can_cast(&game, A, spell));
        game.object_mut(permanent).unwrap().kind = ironsmith::object::ObjectKind::Token;
        assert!(can_cast(&game, A, spell));
        assert!(can_land(&game, A, land));
    }
}
#[test]
fn ashes_stops_both_graveyard_actions_but_not_hand_casts_and_keeps_its_death_trigger() {
    for definition in definitions("Ashes of the Abhorrent") {
        let mut game = game();
        let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let grave_spell = resource(
            &mut game,
            A,
            "Flashback probe",
            "Mana cost: {0}\nType: Sorcery\nYou gain 1 life.\nFlashback {0}",
            Zone::Graveyard,
        );
        let hand_spell = resource(
            &mut game,
            A,
            "Hand probe",
            "Mana cost: {0}\nType: Sorcery\nYou gain 1 life.",
            Zone::Hand,
        );
        let skeleton = resource(
            &mut game,
            A,
            "Reassembling Skeleton",
            "Mana cost: {1}{B}\nType: Creature — Skeleton Warrior\nPower/Toughness: 1/1\n{1}{B}: Return this card from your graveyard to the battlefield tapped.",
            Zone::Graveyard,
        );
        assert!(!can_cast(&game, A, grave_spell));
        assert!(can_cast(&game, A, hand_spell));
        assert!(!can_activate(&game, A, skeleton));
        game.phase_out(host);
        assert!(can_cast(&game, A, grave_spell));
        assert!(can_activate(&game, A, skeleton));
        game.phase_in(host);
        let victim = resource(
            &mut game,
            B,
            "Dying creature",
            "Type: Creature\nPower/Toughness: 1/1",
            Zone::Battlefield,
        );
        game.move_object_by_effect(victim, Zone::Graveyard).unwrap();
        settle(&mut game);
        assert_eq!(game.player(A).unwrap().life, 21);
        game.move_object_by_effect(host, Zone::Graveyard).unwrap();
        assert!(can_cast(&game, A, grave_spell));
        assert!(can_activate(&game, A, skeleton));
    }
}

fn enter(game: &mut GameState, old: ObjectId, dm: &mut impl DecisionMaker) -> ObjectId {
    let receipt = game
        .move_object_with_etb_processing_with_dm(old, Zone::Battlefield, dm)
        .unwrap();
    assert!(!receipt.pending);
    assert!(
        receipt.programs.is_empty(),
        "this fixture must not discard added entry programs"
    );
    receipt.original.into_result().unwrap().new_id
}
struct BlueChoice;
impl DecisionMaker for BlueChoice {
    fn decide_colors(
        &mut self,
        _: &GameState,
        context: &ironsmith::decisions::context::ColorsContext,
    ) -> Vec<ironsmith::color::Color> {
        vec![ironsmith::color::Color::Blue; context.count as usize]
    }
}
#[test]
fn iona_uses_the_actual_entry_color_choice_and_current_controller() {
    for definition in definitions("Iona, Shield of Emeria") {
        let mut game = game();
        let old = game.create_object_from_definition(&definition, A, Zone::Hand);
        let host = enter(&mut game, old, &mut BlueChoice);
        assert_eq!(game.chosen_color(host), Some(ironsmith::color::Color::Blue));
        let own_blue = resource(
            &mut game,
            A,
            "Own blue",
            "Mana cost: {U}\nType: Sorcery\nYou gain 1 life.",
            Zone::Hand,
        );
        let blue = resource(
            &mut game,
            B,
            "Blue",
            "Mana cost: {U}\nType: Sorcery\nYou gain 1 life.",
            Zone::Hand,
        );
        let green = resource(
            &mut game,
            B,
            "Green",
            "Mana cost: {G}\nType: Sorcery\nYou gain 1 life.",
            Zone::Hand,
        );
        assert!(can_cast(&game, A, own_blue));
        turn(&mut game, B);
        assert!(!can_cast(&game, B, blue));
        assert!(can_cast(&game, B, green));
        game.phase_out(host);
        assert!(can_cast(&game, B, blue));
        game.phase_in(host);
        game.set_current_controller(host, B).unwrap();
        assert!(can_cast(&game, B, blue));
        turn(&mut game, A);
        assert!(!can_cast(&game, A, own_blue));
    }
}
#[test]
fn llawan_returns_blue_opposing_creatures_and_intersects_color_and_type_for_casts() {
    for definition in definitions("Llawan, Cephalid Empress") {
        let mut game = game();
        let blue_permanent = resource(
            &mut game,
            B,
            "Blue creature",
            "Mana cost: {U}\nType: Creature\nPower/Toughness: 1/1",
            Zone::Battlefield,
        );
        let green_permanent = resource(
            &mut game,
            B,
            "Green creature",
            "Mana cost: {G}\nType: Creature\nPower/Toughness: 1/1",
            Zone::Battlefield,
        );
        let old = game.create_object_from_definition(&definition, A, Zone::Hand);
        let host = enter(&mut game, old, &mut SelectFirstDecisionMaker);
        settle(&mut game);
        assert!(game.object(blue_permanent).is_none());
        assert!(game.object(green_permanent).is_some());
        turn(&mut game, B);
        let blue = game.player(B).unwrap().hand[0];
        let blue_noncreature = resource(
            &mut game,
            B,
            "Blue artifact",
            "Mana cost: {U}\nType: Artifact",
            Zone::Hand,
        );
        let green = resource(
            &mut game,
            B,
            "Green card",
            "Mana cost: {G}\nType: Creature\nPower/Toughness: 1/1",
            Zone::Hand,
        );
        assert!(!can_cast(&game, B, blue));
        assert!(can_cast(&game, B, blue_noncreature));
        assert!(can_cast(&game, B, green));
        game.move_object_by_effect(host, Zone::Graveyard).unwrap();
        assert!(can_cast(&game, B, blue));
    }
}
#[test]
fn linked_exile_bans_follow_only_this_sources_current_exiled_card_and_current_controller() {
    for name in ["Exclusion Ritual", "Ixalan's Binding"] {
        for definition in definitions(name) {
            let mut game = game();
            let victim = resource(
                &mut game,
                B,
                "Linked name",
                "Mana cost: {0}\nType: Artifact",
                Zone::Battlefield,
            );
            let host_old = game.create_object_from_definition(&definition, A, Zone::Hand);
            let host = enter(&mut game, host_old, &mut SelectFirstDecisionMaker);
            let mut queue = TriggerQueue::new();
            let mut dm = TargetChoice(Target::Object(victim));
            ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm)
                .unwrap();
            ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
            assert!(game.object(victim).is_none());
            let linked = game.get_exiled_with_source_links(host)[0];
            let own = resource(
                &mut game,
                A,
                "Linked name",
                "Mana cost: {0}\nType: Artifact",
                Zone::Hand,
            );
            let other = resource(
                &mut game,
                B,
                "Linked name",
                "Mana cost: {0}\nType: Artifact",
                Zone::Hand,
            );
            assert_eq!(can_cast(&game, A, own), name == "Ixalan's Binding");
            turn(&mut game, B);
            assert!(!can_cast(&game, B, other));
            game.phase_out(host);
            assert!(can_cast(&game, B, other));
            game.phase_in(host);
            if name == "Ixalan's Binding" {
                game.set_current_controller(host, B).unwrap();
                assert!(can_cast(&game, B, other));
                turn(&mut game, A);
                assert!(!can_cast(&game, A, own));
                game.set_current_controller(host, A).unwrap();
                turn(&mut game, B);
            }
            game.move_object_by_effect(linked, Zone::Graveyard).unwrap();
            assert!(
                can_cast(&game, B, other),
                "a departed exiled incarnation no longer supplies a name"
            );
        }
    }
}
#[test]
fn immortalsun_blocks_only_loyalty_and_keeps_draw_cost_and_anthem_bodies() {
    for definition in definitions("The Immortal Sun") {
        let mut game = game();
        library(&mut game, A, 4);
        let walker = resource(
            &mut game,
            A,
            "Two-ability walker",
            "Mana cost: {0}\nType: Planeswalker — Jace\nLoyalty: 5\n+1: You gain 1 life.\n{0}: You gain 2 life.",
            Zone::Battlefield,
        );
        let count = |game: &GameState| {
            compute_legal_actions(game, A).unwrap().iter().filter(|action| matches!(action, LegalAction::ActivateAbility { source, .. } if *source == walker)).count()
        };
        assert_eq!(count(&game), 2);
        let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        assert_eq!(
            count(&game),
            1,
            "nonloyalty ability of the same planeswalker stays usable"
        );
        let creature = resource(
            &mut game,
            A,
            "Anthem probe",
            "Type: Creature\nPower/Toughness: 2/2",
            Zone::Battlefield,
        );
        assert_eq!(game.calculated_power(creature), Some(3));
        assert_eq!(game.calculated_toughness(creature), Some(3));
        let spell = resource(
            &mut game,
            A,
            "Reduced artifact",
            "Mana cost: {1}\nType: Artifact",
            Zone::Hand,
        );
        game.player_mut(A).unwrap().mana_pool.empty();
        assert!(can_cast(&game, A, spell));
        // The starting player skips their ordinary draw on the first turn.
        game.turn.turn_number = 2;
        game.turn.phase = Phase::Beginning;
        game.turn.step = Some(Step::Draw);
        let mut runner = ironsmith::turn_runner::TurnRunner::from_state_for_sync(
            ironsmith::turn_runner::TurnState::Draw,
        );
        let mut queue = TriggerQueue::new();
        runner.advance(&mut game, &mut queue).unwrap();
        put_triggers_on_stack(&mut game, &mut queue).unwrap();
        settle(&mut game);
        assert_eq!(
            game.player(A).unwrap().hand.len(),
            3,
            "one existing spell plus the ordinary and additional draw"
        );
        turn(&mut game, A);
        game.phase_out(host);
        assert_eq!(count(&game), 2);
        assert!(!can_cast(&game, A, spell));
        assert_eq!(game.calculated_power(creature), Some(2));
    }
}
#[test]
fn tomik_blocks_opponents_graveyard_land_plays_without_changing_permissions_or_target_roles() {
    for definition in definitions("Tomik, Distinguished Advokist") {
        let mut game = game();
        resource(
            &mut game,
            A,
            "Own graveyard permission",
            "Type: Artifact\nYou may play lands from your graveyard.",
            Zone::Battlefield,
        );
        resource(
            &mut game,
            B,
            "Other graveyard permission",
            "Type: Artifact\nYou may play lands from your graveyard.",
            Zone::Battlefield,
        );
        let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let own = land(&mut game, A, "Own grave land", Zone::Graveyard);
        let other = land(&mut game, B, "Other grave land", Zone::Graveyard);
        let hand = land(&mut game, B, "Other hand land", Zone::Hand);
        assert!(can_land(&game, A, own));
        turn(&mut game, B);
        assert!(!can_land(&game, B, other));
        assert!(can_land(&game, B, hand));
        let opponent_spell = resource(&mut game, B, "Opponent spell", "Type: Instant", Zone::Stack);
        let own_spell = resource(&mut game, A, "Own spell", "Type: Instant", Zone::Stack);
        game.refresh_continuous_state().unwrap();
        assert!(!ironsmith::targeting::can_target_object(&game, own, opponent_spell, B).is_legal());
        assert!(ironsmith::targeting::can_target_object(&game, other, own_spell, A).is_legal());
        game.phase_out(host);
        assert!(can_land(&game, B, other));
        game.phase_in(host);
        game.set_current_controller(host, B).unwrap();
        assert!(can_land(&game, B, other));
        turn(&mut game, A);
        assert!(!can_land(&game, A, own));
    }
}
#[test]
fn territorial_dispute_bans_everyones_lands_and_its_unpaid_upkeep_removes_the_rule() {
    for definition in definitions("Territorial Dispute") {
        let mut game = game();
        let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let own = land(&mut game, A, "Own land", Zone::Hand);
        let other = land(&mut game, B, "Other land", Zone::Hand);
        assert!(!can_land(&game, A, own));
        turn(&mut game, B);
        assert!(!can_land(&game, B, other));
        turn(&mut game, A);
        game.turn.phase = Phase::Beginning;
        game.turn.step = Some(Step::Upkeep);
        let mut queue = TriggerQueue::new();
        ironsmith::game_loop::generate_and_queue_step_triggers(&mut game, &mut queue);
        put_triggers_on_stack(&mut game, &mut queue).unwrap();
        settle(&mut game);
        assert!(game.object(host).is_none());
        turn(&mut game, A);
        assert!(can_land(&game, A, own));
        turn(&mut game, B);
        assert!(can_land(&game, B, other));
    }
}
#[test]
fn graveyard_activation_prohibition_also_reaches_mana_ability_sources() {
    for definition in definitions("Ashes of the Abhorrent") {
        let mut game = game();
        let mut mana = compile_to_runtime_definition(
            "Graveyard mana probe",
            "Type: Artifact\n{0}: Add {B}.",
            false,
        )
        .unwrap();
        for ability in &mut mana.abilities {
            if matches!(ability.kind, AbilityKind::Activated(_)) {
                ability.functional_zones = vec![Zone::Graveyard];
            }
        }
        let source = game.create_object_from_definition(&mana, A, Zone::Graveyard);
        let index = mana
            .abilities
            .iter()
            .position(|ability| matches!(ability.kind, AbilityKind::Activated(_)))
            .unwrap();
        let action = SpecialAction::ActivateManaAbility {
            permanent_id: source,
            ability_index: index,
        };
        game.refresh_continuous_state().unwrap();
        assert!(can_perform_check(&action, &game, A).is_ok());
        let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        game.refresh_continuous_state().unwrap();
        assert!(can_perform_check(&action, &game, A).is_err());
        game.phase_out(host);
        game.refresh_continuous_state().unwrap();
        assert!(can_perform_check(&action, &game, A).is_ok());
    }
}
#[test]
fn land_restriction_checks_the_chosen_mdfc_face_name() {
    use ironsmith::card::LinkedFaceLayout;
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::{CardId, CardType};
    for definition in definitions("Cornered Market") {
        for front_type in [CardType::Land, CardType::Sorcery] {
            let mut game = game();
            game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let front_id = CardId::new();
            let back_id = CardId::new();
            let front = CardDefinitionBuilder::new(front_id, "Unrelated front")
                .card_types(vec![front_type])
                .other_face(back_id)
                .other_face_name("Restricted back")
                .linked_face_layout(LinkedFaceLayout::TransformLike)
                .build();
            let back = CardDefinitionBuilder::new(back_id, "Restricted back")
                .card_types(vec![CardType::Land])
                .other_face(front_id)
                .other_face_name("Unrelated front")
                .linked_face_layout(LinkedFaceLayout::TransformLike)
                .build();
            game.register_linked_face_definition(&front);
            game.register_linked_face_definition(&back);
            let candidate = game.create_object_from_definition(&front, A, Zone::Hand);
            let same = land(&mut game, B, "Restricted back", Zone::Battlefield);
            let action = if front_type == CardType::Land {
                assert!(
                    can_perform_check(&SpecialAction::PlayLand { card_id: candidate }, &game, A)
                        .is_ok()
                );
                SpecialAction::PlayLandBackFace { card_id: candidate }
            } else {
                SpecialAction::PlayLand { card_id: candidate }
            };
            assert_eq!(
                can_perform_check(&action, &game, A),
                Err(ActionError::LandPlayProhibited)
            );
            game.move_object_by_effect(same, Zone::Graveyard).unwrap();
            assert!(can_perform_check(&action, &game, A).is_ok());
            assert_eq!(
                game.object(candidate).unwrap().name.as_ref(),
                "Unrelated front",
                "legality queries never commit the alternate face"
            );
        }
    }
}

#[test]
fn quark_sacrifice_prohibition_preserves_player_subtype_and_token_scopes() {
    let (artifact, direct) = compile_to_artifact("Quark, Humble Proprietor", "Mana cost: {2}{R}\nType: Legendary Creature — Ferengi Rogue\nPower/Toughness: 1/2\nYou can't sacrifice Treasure tokens.\nAt the beginning of each upkeep, create a Treasure token.", false).unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    for definition in [direct, materialize_artifact(&restored).unwrap()] {
        assert!(definition.card.subtypes.contains(&ironsmith::types::Subtype::Ferengi));
        let mut game = game();
        let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        for owner in [A, B] {
            for (subtype, token) in [("Treasure", true), ("Treasure", false), ("Clue", true)] {
                let mut resource = compile_to_runtime_definition("Sacrifice resource", &format!("Type: Artifact — {subtype}\nSacrifice this artifact: Add {{C}}."), false).unwrap();
                resource.card.is_token = token;
                let id = game.create_object_from_definition(&resource, owner, Zone::Battlefield);
                game.refresh_continuous_state().unwrap();
                let protected = owner == A && subtype == "Treasure" && token;
                assert_eq!(game.can_be_sacrificed(id), !protected);
                let action = SpecialAction::ActivateManaAbility { permanent_id: id, ability_index: 0 };
                assert_eq!(can_perform_check(&action, &game, owner).is_ok(), !protected);
            }
        }
        game.phase_out(host);
        game.refresh_continuous_state().unwrap();
        for id in game.battlefield.iter().copied().filter(|id| *id != host) {
            assert!(game.can_be_sacrificed(id));
        }
    }
}

#[test]
fn dilemma_counts_choices_instead_of_modes_and_survives_countering() {
    for (seven, sisko) in definitions("Seven of Nine").into_iter()
        .zip(definitions("Captain Benjamin Sisko"))
    {
        for mode_count in [1, 2] {
            let mut game = game();
            let seven_id = game.create_object_from_definition(&seven, A, Zone::Battlefield);
            game.create_object_from_definition(&sisko, A, Zone::Battlefield);
            for n in 0..5 { land(&mut game, A, &format!("Draw card {n}"), Zone::Library); }
            let modal = resource(&mut game, A, "Modal dilemma", &format!(
                "Mana cost: {{0}}\nType: Instant\nChoose {} —\n• You gain 1 life.\n• You gain 2 life.",
                if mode_count == 1 { "one" } else { "two" }), Zone::Hand);
            let mut queue = TriggerQueue::new();
            let mut state = PriorityLoopState::new(2);
            let mut dm = SelectFirstDecisionMaker;
            let action = compute_legal_actions(&game, A).unwrap().into_iter()
                .find(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == modal)).unwrap();
            let mut result = apply_priority_response_with_dm(&mut game, &mut queue, &mut state,
                &PriorityResponse::PriorityAction(action), &mut dm).unwrap();
            for _ in 0..64 {
                if !state.has_pending_action() { break; }
                let ironsmith::GameProgress::NeedsDecisionCtx(context) = result else { panic!("{result:?}"); };
                result = apply_decision_context_with_dm(&mut game, &mut queue, &mut state, &context, &mut dm).unwrap();
            }
            assert!(!state.has_pending_action());
            let count = ironsmith::effect::Value::TurnHistoryCount(
                ironsmith_core::TurnHistoryCount::KeywordActionsPerformed {
                    player: ironsmith::PlayerFilter::You,
                    actions: vec![ironsmith_core::KeywordActionKind::FaceDilemma],
                });
            let ctx = ironsmith::effects::EffectContext::new_default(seven_id, A);
            assert_eq!(ironsmith::effects::helpers::resolve_value(&game, &count, &ctx).unwrap(), 1);
            put_triggers_on_stack(&mut game, &mut queue).unwrap();
            assert_eq!(game.stack.len(), 2, "one draw trigger per modal choice");
            resolve_stack_entry(&mut game).unwrap();
            assert_eq!(game.player(A).unwrap().hand.len(), 1);
            // Removing the modal spell before resolution must not undo the dilemma.
            let spell = game.stack.pop().unwrap().object_id;
            game.move_object_by_effect(spell, Zone::Graveyard).unwrap();
            game.queue_trigger_event(Default::default(), ironsmith::triggers::TriggerEvent::new_with_provenance(
                ironsmith::events::BeginningOfEndStepEvent::new(A), Default::default()));
            settle(&mut game);
            assert_eq!(game.battlefield.iter().filter(|&&id|
                game.object(id).is_some_and(|object| object.kind == ironsmith::object::ObjectKind::Token)
                && game.current_has_subtype(id, ironsmith::types::Subtype::Officer)).count(), 1);
            assert_eq!(game.player(A).unwrap().life, 20, "countered modes never resolve");
        }
    }
}

#[test]
fn dilemma_records_activated_triggered_and_opponent_chosen_modes_at_announcement() {
    fn count(game: &GameState, source: ObjectId, player: PlayerId) -> i32 {
        ironsmith::effects::helpers::resolve_value(game,
            &ironsmith::effect::Value::TurnHistoryCount(ironsmith_core::TurnHistoryCount::KeywordActionsPerformed {
                player: ironsmith::PlayerFilter::Specific(player),
                actions: vec![ironsmith_core::KeywordActionKind::FaceDilemma],
            }), &ironsmith::effects::EffectContext::new_default(source, A)).unwrap()
    }
    for (name, body, kind) in [
        ("Activated modal", "Type: Artifact\n{0}: Choose two —\n• You gain 1 life.\n• You gain 2 life.", 0),
        ("Triggered modal", "Type: Artifact\nWhen this artifact enters, choose two —\n• You gain 1 life.\n• You gain 2 life.", 1),
        ("Opponent modal", "Mana cost: {0}\nType: Instant\nAn opponent chooses one —\n• You gain 1 life.\n• You gain 2 life.", 2),
    ] {
        let (artifact, direct) = compile_to_artifact(name, body, false).unwrap();
        let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
        for definition in [direct, materialize_artifact(&restored).unwrap()] {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, if kind == 0 { Zone::Battlefield } else { Zone::Hand });
            assert_eq!(count(&game, source, A), 0);
            if kind == 0 {
                announce_activation(&mut game, source, &mut SelectFirstDecisionMaker);
            } else if kind == 1 {
                assert!(!game.move_object_with_etb_processing_with_dm(source, Zone::Battlefield, &mut SelectFirstDecisionMaker).unwrap().pending);
                ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut TriggerQueue::new(), &mut SelectFirstDecisionMaker).unwrap();
            } else {
                announce_existing_spell(&mut game, source, A);
            }
            assert_eq!(count(&game, source, A), i32::from(kind != 2), "{name}");
            assert_eq!(count(&game, source, B), i32::from(kind == 2), "{name}");
            settle(&mut game);
            assert_eq!(count(&game, source, A), i32::from(kind != 2), "resolution must not count again: {name}");
            assert_eq!(count(&game, source, B), i32::from(kind == 2));
        }
    }
}

#[test]
fn dilemma_resolution_counts_the_chooser_once_and_excludes_inherited_or_internal_choices() {
    use ironsmith::decision::DecisionMaker;
    use ironsmith::decisions::SelectOptionsContext;
    use ironsmith::effect::{Effect, EffectMode, Value};
    use ironsmith::effects::{ChooseModeEffect, EffectContext, EffectExecutor};
    use ironsmith::target::PlayerFilter;
    for (authored, announced, chosen, expected) in [
        (true, false, vec![0, 1], 1),
        (true, false, vec![0, 0], 1),
        (true, false, vec![], 0),
        (false, false, vec![0, 1], 0),
        (true, true, vec![0, 1], 0),
    ] {
        struct Modes(Vec<usize>);
        impl DecisionMaker for Modes {
            fn decide_options(&mut self, _: &GameState, _: &SelectOptionsContext) -> Vec<usize> {
                self.0.clone()
            }
        }
        let mut game = game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();
        let mut dm = Modes(chosen.clone());
        let mut ctx = EffectContext::new(source, alice, &mut dm);
        if announced { ctx.chosen_modes = Some(chosen); }
        let mut effect = ChooseModeEffect::choose_up_to(
            Value::Fixed(2), Value::Fixed(0),
            vec![EffectMode::new("Gain 1 life", vec![Effect::gain_life(1)]),
                EffectMode::new("Gain 2 life", vec![Effect::gain_life(2)])],
        ).with_repeated_modes();
        effect.faces_dilemma = authored;
        if !announced { effect.chooser = Some(PlayerFilter::Specific(bob)); }
        let outcome = effect.execute(&mut game, &mut ctx).unwrap();
        let events = outcome.events.iter().filter_map(|event|
            event.downcast::<ironsmith::events::KeywordActionEvent>())
            .filter(|event| event.action == ironsmith::events::KeywordActionKind::FaceDilemma)
            .collect::<Vec<_>>();
        assert_eq!(events.len(), expected);
        if let Some(event) = events.first() { assert_eq!(event.player, bob); }
    }
}


#[test]
fn q_exiles_until_a_match_and_exhausts_an_all_land_library() {
    for definition in definitions("Q, Stacking the Deck") {
        for with_match in [false, true] {
            let mut game = game();
            for name in ["First land", "Second land", "Third land"] { land(&mut game, A, name, Zone::Library); }
            if with_match {
                resource(&mut game, A, "First possible match", "Type: Artifact", Zone::Library);
                resource(&mut game, A, "Second possible match", "Type: Artifact", Zone::Library);
            }
            let source = game.create_object_from_definition(&definition, A, Zone::Hand);
            assert!(!game.move_object_with_etb_processing_with_dm(source, Zone::Battlefield, &mut SelectFirstDecisionMaker).unwrap().pending);
            resource(&mut game, A, "Hand payment", "Type: Land", Zone::Hand);
            let mut shuffled = game.clone();
            shuffled.shuffle_player_library(A);
            let mut expected_exile = Vec::new();
            for id in shuffled.player(A).unwrap().library.iter().rev() {
                let object = shuffled.object(*id).unwrap();
                expected_exile.push(object.stable_id);
                if !object.card_types.contains(&ironsmith::types::CardType::Land) { break; }
            }
            let original_size = game.player(A).unwrap().library.len();
            settle(&mut game);
            assert_eq!(game.player(A).unwrap().library.len(), original_size - expected_exile.len());
            assert_eq!(game.exile.len(), expected_exile.len());
            for stable in expected_exile {
                let id = game.find_object_by_stable_id(stable).unwrap();
                let object = game.object(id).unwrap();
                assert_eq!(object.zone, Zone::Exile);
                assert_eq!(can_activate(&game, A, id), !object.card_types.contains(&ironsmith::types::CardType::Land));
            }
        }
    }
}


#[test]
fn sisko_casts_any_number_of_eligible_exiled_spells_and_can_decline_all() {
    struct ChooseCards(bool);
    impl DecisionMaker for ChooseCards {
        fn decide_boolean(&mut self, _game: &GameState, ctx: &ironsmith::decisions::context::BooleanContext) -> bool { self.0 && ctx.can_accept }
        fn decide_objects(&mut self, _game: &GameState, ctx: &ironsmith::decisions::context::SelectObjectsContext) -> Vec<ObjectId> {
            if !self.0 { return Vec::new(); }
            ctx.candidates.iter().filter(|candidate| candidate.legal).map(|candidate| candidate.id).take(ctx.max.unwrap_or(usize::MAX)).collect()
        }
    }
    for definition in definitions("Captain Benjamin Sisko") {
        for accept in [false, true] {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            game.remove_summoning_sickness(source);
            resource(&mut game, B, "Opposing Officer", "Type: Creature — Officer\nPower/Toughness: 2/2", Zone::Battlefield);
            let mut cards = Vec::new();
            for (name, text, eligible) in [
                ("First free spell", "Mana cost: {1}\nType: Artifact", true),
                ("Second free spell", "Mana cost: {0}\nType: Artifact", true),
                ("Too expensive", "Mana cost: {2}\nType: Artifact", false),
                ("Much too expensive", "Mana cost: {4}\nType: Artifact", false),
                ("Land", "Type: Land", false),
            ] {
                let id = resource(&mut game, A, name, text, Zone::Library);
                cards.push((game.object(id).unwrap().stable_id, eligible));
            }
            let unrelated = resource(&mut game, A, "Previously exiled", "Mana cost: {0}\nType: Artifact", Zone::Exile);
            announce_activation(&mut game, source, &mut SelectFirstDecisionMaker);
            let mana_after_activation = game.player(A).unwrap().mana_pool.clone();
            ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut ChooseCards(accept)).unwrap();
            assert_eq!(game.stack.len(), if accept { 2 } else { 0 });
            assert_eq!(game.player(A).unwrap().mana_pool, mana_after_activation);
            settle(&mut game);
            for (stable, eligible) in cards {
                let id = game.find_object_by_stable_id(stable).unwrap();
                assert_eq!(game.object(id).unwrap().zone, if accept && eligible { Zone::Battlefield } else { Zone::Exile });
            }
            assert!(game.player(A).unwrap().library.is_empty());
            assert_eq!(game.object(unrelated).unwrap().zone, Zone::Exile);
        }
    }
}
