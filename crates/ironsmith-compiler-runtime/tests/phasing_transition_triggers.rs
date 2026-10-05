//! Source-only phasing regression scenarios: actual phase, untap, attack,
//! targeting and zone producers, direct and restored artifacts. Unrun.
use ironsmith::ability::AbilityKind;
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{DecisionMaker, SelectFirstDecisionMaker};
use ironsmith::decisions::context::TargetsContext;
use ironsmith::effects::{EffectContext, EffectExecutor, PhaseInEffect, PhaseOutEffect};
use ironsmith::game_loop::{put_triggers_on_stack_with_dm, resolve_stack_entry_with};
use ironsmith::object::{AttachmentTarget, CounterType};
use ironsmith::target::ChooseSpec;
use ironsmith::triggers::{TriggerEvent, TriggerQueue};
use ironsmith::{GameState, ObjectId, Phase, PlayerId, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_core::{ObjectFilter, TriggerKind};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/phasing_transition_triggers.json.fixture"
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
    GameState::new(vec!["Alice".into(), "Bob".into()], 20)
}
fn resource(game: &mut GameState, owner: PlayerId, zone: Zone, text: &str) -> ObjectId {
    let definition = compile_to_runtime_definition("Phasing resource", text, false).unwrap();
    game.create_object_from_definition(&definition, owner, zone)
}
fn creature(game: &mut GameState, owner: PlayerId, spirit: bool) -> ObjectId {
    resource(
        game,
        owner,
        Zone::Battlefield,
        if spirit {
            "Type: Creature — Spirit\nPower/Toughness: 2/2"
        } else {
            "Type: Creature\nPower/Toughness: 2/2"
        },
    )
}
fn stack(game: &mut GameState, dm: &mut impl DecisionMaker) -> usize {
    put_triggers_on_stack_with_dm(game, &mut TriggerQueue::new(), dm).unwrap();
    game.stack.len()
}
fn settle(game: &mut GameState, dm: &mut impl DecisionMaker) -> usize {
    let count = stack(game, dm);
    for _ in 0..20 {
        if game.stack_is_empty() {
            return count;
        }
        resolve_stack_entry_with(game, dm).unwrap();
        stack(game, dm);
    }
    panic!("phasing did not settle")
}
fn execute(game: &mut GameState, source: ObjectId, effect: &dyn EffectExecutor) {
    let mut dm = SelectFirstDecisionMaker;
    let mut ctx = EffectContext::new(source, A, &mut dm);
    let outcome = effect.execute(game, &mut ctx).unwrap();
    for event in outcome.events {
        game.queue_trigger_event(event.provenance(), event);
    }
}
fn untap_step(game: &mut GameState) {
    game.turn.active_player = A;
    game.turn.phase = Phase::Beginning;
    game.turn.step = Some(ironsmith::game_state::Step::Untap);
    ironsmith::turn::execute_untap_step_with(game, &mut SelectFirstDecisionMaker).unwrap();
    game.turn.step = Some(ironsmith::game_state::Step::Upkeep);
}
fn counters(game: &GameState, id: ObjectId, kind: CounterType) -> u32 {
    game.object(id)
        .unwrap()
        .counters
        .get(&kind)
        .copied()
        .unwrap_or(0)
}
fn has_phasing(kind: &TriggerKind) -> bool {
    match kind {
        TriggerKind::PhasingChanged { .. } => true,
        TriggerKind::Either { left, right } => has_phasing(&left.kind) || has_phasing(&right.kind),
        _ => false,
    }
}
struct TargetChoice(ObjectId);
impl DecisionMaker for TargetChoice {
    fn decide_targets(&mut self, _: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        let target = Target::Object(self.0);
        assert!(
            ctx.requirements
                .iter()
                .any(|req| req.legal_targets.contains(&target))
        );
        vec![target]
    }
}

#[test]
fn five_exact_phasing_cards_round_trip_complete_typed_models() {
    assert_eq!(fixtures().len(), 5);
    for row in fixtures() {
        for definition in definitions(row["name"].as_str().unwrap()) {
            assert!(definition.abilities.iter().any(
                |ability| matches!(&ability.kind, AbilityKind::Triggered(triggered)
            if triggered.trigger.compiled_model().is_some_and(|model| has_phasing(&model.kind)))
            ));
        }
    }
}

#[test]
fn teferis_imp_real_untap_exchange_preserves_identity_and_resolves_both_trigger_directions() {
    for definition in definitions("Teferi's Imp") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        for zone in [Zone::Hand, Zone::Hand, Zone::Library, Zone::Library] {
            resource(&mut game, A, zone, "Type: Artifact");
        }
        let before_zone = game.player(A).unwrap().graveyard.len();
        untap_step(&mut game);
        assert!(game.is_phased_out(source));
        assert_eq!(settle(&mut game, &mut SelectFirstDecisionMaker), 1);
        assert_eq!(game.player(A).unwrap().hand.len(), 1);
        assert_eq!(game.player(A).unwrap().graveyard.len(), before_zone + 1);
        game.next_turn();
        game.next_turn();
        untap_step(&mut game);
        assert!(!game.is_phased_out(source));
        assert_eq!(settle(&mut game, &mut SelectFirstDecisionMaker), 1);
        assert_eq!(game.player(A).unwrap().hand.len(), 2);
        assert!(game.battlefield.contains(&source));
        game.phase_in(source);
        assert_eq!(settle(&mut game, &mut SelectFirstDecisionMaker), 0);
    }
}

#[test]
fn warping_wurm_phase_in_counter_and_unpaid_upkeep_use_real_bodies() {
    for definition in definitions("Warping Wurm") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        game.phase_out(source);
        settle(&mut game, &mut SelectFirstDecisionMaker);
        untap_step(&mut game);
        assert_eq!(settle(&mut game, &mut SelectFirstDecisionMaker), 1);
        assert_eq!(counters(&game, source, CounterType::PlusOnePlusOne), 1);
        game.queue_trigger_event(
            Default::default(),
            TriggerEvent::new(
                ironsmith::events::BeginningOfUpkeepEvent::new(A),
                Default::default(),
            ),
        );
        assert_eq!(settle(&mut game, &mut SelectFirstDecisionMaker), 1);
        assert!(
            game.is_phased_out(source),
            "the unpaid upkeep really phases it out"
        );
        assert_eq!(counters(&game, source, CounterType::PlusOnePlusOne), 1);
    }
}

#[test]
fn shimmering_efreet_phase_in_target_phases_out_without_changing_zone_or_attachment() {
    for definition in definitions("Shimmering Efreet") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let victim = creature(&mut game, B, false);
        let aura = resource(
            &mut game,
            B,
            Zone::Battlefield,
            "Type: Enchantment — Aura\nEnchant creature",
        );
        assert!(game.attach_object_to_target(aura, AttachmentTarget::Object(victim)));
        game.take_pending_trigger_events();
        game.phase_out(source);
        settle(&mut game, &mut SelectFirstDecisionMaker);
        untap_step(&mut game);
        assert_eq!(settle(&mut game, &mut TargetChoice(victim)), 1);
        assert!(game.is_phased_out(victim) && game.is_phased_out(aura));
        assert_eq!(
            game.object(aura).unwrap().attached_to,
            Some(AttachmentTarget::Object(victim))
        );
        assert!(game.battlefield.contains(&victim) && game.battlefield.contains(&aura));
        assert!(game.player(B).unwrap().graveyard.is_empty());
    }
}

#[test]
fn king_sees_all_simultaneous_phase_ins_and_keeps_source_distinct_from_other_controlled_spirits() {
    for definition in definitions("King of the Oathbreakers") {
        let mut game = game();
        let king = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let own_spirit = creature(&mut game, A, true);
        let other_spirit = creature(&mut game, B, true);
        let nonspirit = creature(&mut game, A, false);
        game.phase_out_simultaneously(&[king, own_spirit, other_spirit, nonspirit]);
        settle(&mut game, &mut SelectFirstDecisionMaker);
        game.phase_in_simultaneously(&[own_spirit, other_spirit, nonspirit, king]);
        assert_eq!(settle(&mut game, &mut SelectFirstDecisionMaker), 2);
        let tokens = game
            .battlefield
            .iter()
            .copied()
            .filter(|id| game.object(*id).unwrap().kind == ironsmith::object::ObjectKind::Token)
            .collect::<Vec<_>>();
        assert_eq!(tokens.len(), 2);
        for token in tokens {
            assert!(game.is_tapped(token));
            assert!(game.current_has_static_ability_id(
                token,
                ironsmith::static_abilities::StaticAbilityId::Flying
            ));
        }
    }
}

#[test]
fn war_doctor_groups_phasing_effect_and_indirect_attachments_separately_from_exile_events() {
    for definition in definitions("The War Doctor") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let one = creature(&mut game, A, false);
        let two = creature(&mut game, A, false);
        let aura = resource(
            &mut game,
            B,
            Zone::Battlefield,
            "Type: Enchantment — Aura\nEnchant creature",
        );
        assert!(game.attach_object_to_target(aura, AttachmentTarget::Object(one)));
        game.take_pending_trigger_events();
        execute(
            &mut game,
            source,
            &PhaseOutEffect::all(ObjectFilter::creature().other()),
        );
        assert!(game.is_phased_out(one) && game.is_phased_out(two) && game.is_phased_out(aura));
        assert_eq!(settle(&mut game, &mut SelectFirstDecisionMaker), 1);
        assert_eq!(counters(&game, source, CounterType::Time), 1);
        execute(
            &mut game,
            source,
            &PhaseInEffect::all(ObjectFilter::permanent()),
        );
        assert_eq!(settle(&mut game, &mut SelectFirstDecisionMaker), 0);
        assert!(!game.is_phased_out(one) && !game.is_phased_out(aura));
        // Two authored instructions remain two events, even before a drain.
        execute(
            &mut game,
            source,
            &PhaseOutEffect::with_spec(ChooseSpec::SpecificObject(one)),
        );
        execute(
            &mut game,
            source,
            &PhaseOutEffect::with_spec(ChooseSpec::SpecificObject(two)),
        );
        assert_eq!(settle(&mut game, &mut SelectFirstDecisionMaker), 2);
        assert_eq!(counters(&game, source, CounterType::Time), 3);
        let card = resource(&mut game, A, Zone::Graveyard, "Type: Artifact");
        game.move_object_by_effect(card, Zone::Exile).unwrap();
        assert_eq!(settle(&mut game, &mut SelectFirstDecisionMaker), 1);
        assert_eq!(counters(&game, source, CounterType::Time), 4);
        game.phase_out(source);
        assert_eq!(
            settle(&mut game, &mut SelectFirstDecisionMaker),
            0,
            "other excludes the source"
        );
    }
}

#[test]
fn phase_out_source_lki_and_indirect_return_use_one_common_transition() {
    for definition in definitions("The War Doctor") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let host = creature(&mut game, A, false);
        let aura = resource(
            &mut game,
            B,
            Zone::Battlefield,
            "Type: Enchantment — Aura\nEnchant creature",
        );
        assert!(game.attach_object_to_target(aura, AttachmentTarget::Object(host)));
        game.take_pending_trigger_events();
        // Both Aura and host are explicitly selected. It phases indirectly
        // and must return with A's host, not during B's next untap step.
        execute(
            &mut game,
            source,
            &PhaseOutEffect::all(ObjectFilter::permanent()),
        );
        let events = game.take_pending_trigger_events();
        assert_eq!(events.len(), 3);
        let batch = events[0].simultaneous_batch();
        assert!(batch.is_some());
        assert!(events.iter().all(|event| event.kind()
            == ironsmith::events::EventKind::PermanentPhasedOut
            && event.simultaneous_batch() == batch));
        for event in events {
            game.queue_trigger_event(event.provenance(), event);
        }
        assert_eq!(
            stack(&mut game, &mut SelectFirstDecisionMaker),
            1,
            "a phased-out observer uses source LKI"
        );
        game.phase_in(source);
        settle(&mut game, &mut SelectFirstDecisionMaker);
        assert_eq!(counters(&game, source, CounterType::Time), 1);
        game.turn.active_player = B;
        game.turn.phase = Phase::Beginning;
        game.turn.step = Some(ironsmith::game_state::Step::Untap);
        ironsmith::turn::execute_untap_step_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
        assert!(
            game.is_phased_out(host) && game.is_phased_out(aura),
            "the indirectly phased-out Aura doesn't return on its own controller's step"
        );
        untap_step(&mut game);
        assert!(!game.is_phased_out(host) && !game.is_phased_out(aura));
        assert_eq!(
            game.object(aura).unwrap().attached_to,
            Some(AttachmentTarget::Object(host))
        );
        assert_eq!(settle(&mut game, &mut SelectFirstDecisionMaker), 0);
    }
}

#[test]
fn kings_targeting_body_responds_to_an_actual_spell_before_it_resolves() {
    use ironsmith::decision::LegalAction;
    use ironsmith::game_loop::{
        PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
        apply_priority_response_with_dm,
    };
    for definition in definitions("King of the Oathbreakers") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let hand = resource(
            &mut game,
            B,
            Zone::Hand,
            "Mana cost: {0}\nType: Instant\nTarget creature gets +1/+0 until end of turn.",
        );
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.active_player = B;
        game.turn.priority_player = Some(B);
        let action = LegalAction::CastSpell {
            spell_id: hand,
            from_zone: Zone::Hand,
            casting_method: ironsmith::alternative_cast::CastingMethod::Normal,
        };
        assert!(
            ironsmith::decision::compute_legal_actions(&game, B)
                .unwrap()
                .contains(&action)
        );
        let mut queue = TriggerQueue::new();
        let mut state = PriorityLoopState::new(2);
        let mut dm = TargetChoice(source);
        let mut progress = apply_priority_response_with_dm(
            &mut game,
            &mut queue,
            &mut state,
            &PriorityResponse::PriorityAction(action),
            &mut dm,
        )
        .unwrap();
        for _ in 0..30 {
            if state.pending_cast.is_none() && state.pending_method_selection.is_none() {
                break;
            }
            let ironsmith::GameProgress::NeedsDecisionCtx(context) = progress else {
                panic!("cast still pending")
            };
            progress = apply_decision_context_with_dm(
                &mut game, &mut queue, &mut state, &context, &mut dm,
            )
            .unwrap();
        }
        assert!(state.pending_cast.is_none() && state.pending_method_selection.is_none());
        put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
        assert_eq!(game.stack.len(), 2);
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert!(game.is_phased_out(source));
        settle(&mut game, &mut dm);
        assert!(game.battlefield.contains(&source));
    }
}

#[test]
fn war_doctor_attack_uses_time_counters_and_exiles_the_creature_its_damage_would_kill() {
    use ironsmith::combat_state::{AttackTarget, CombatState};
    use ironsmith::decision::AttackerDeclaration;
    for definition in definitions("The War Doctor") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let victim = creature(&mut game, B, false);
        let stable = game.object(victim).unwrap().stable_id;
        game.add_counters(source, CounterType::Time, 2).unwrap();
        game.remove_summoning_sickness(source);
        game.turn.phase = Phase::Combat;
        game.turn.step = Some(ironsmith::game_state::Step::DeclareAttackers);
        game.mark_combat_phase_started();
        let mut queue = TriggerQueue::new();
        ironsmith::game_loop::apply_attacker_declarations(
            &mut game,
            &mut CombatState::default(),
            &mut queue,
            &[AttackerDeclaration {
                creature: source,
                target: AttackTarget::Player(B),
            }],
        )
        .unwrap();
        let mut dm = TargetChoice(victim);
        put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
        assert_eq!(game.stack.len(), 1);
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        ironsmith::game_loop::check_and_apply_sbas_with(&mut game, &mut queue, &mut dm).unwrap();
        let exiled = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.object(exiled).unwrap().zone, Zone::Exile);
        assert!(game.player(B).unwrap().graveyard.is_empty());
    }
}

#[test]
fn simultaneous_untap_exchange_does_not_let_a_newly_phased_in_source_look_back() {
    for definition in definitions("The War Doctor") {
        for doctor_was_phased_out in [false, true] {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            if doctor_was_phased_out {
                game.phase_out(source);
                settle(&mut game, &mut SelectFirstDecisionMaker);
            }
            let other = resource(
                &mut game,
                A,
                Zone::Battlefield,
                "Type: Creature\nPower/Toughness: 2/2\nPhasing",
            );
            untap_step(&mut game);
            assert!(game.is_phased_out(other));
            assert!(!game.is_phased_out(source));
            let expected = if doctor_was_phased_out { 0 } else { 1 };
            assert_eq!(settle(&mut game, &mut SelectFirstDecisionMaker), expected);
            assert_eq!(counters(&game, source, CounterType::Time), expected as u32);
        }
    }
}

#[test]
fn time_and_tide_does_not_enable_a_new_phase_out_observer() {
    // Authored and unrun: an explicit typed exchange must retain the same
    // pre-event observer set as the regular untap-step exchange.
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/phasing_transition_runtime_gaps.json.fixture"
    ))
    .unwrap();
    let row = &rows[0];
    let (artifact, direct) = compile_to_artifact(
        row["name"].as_str().unwrap(),
        row["text"].as_str().unwrap(),
        false,
    )
    .unwrap();
    let restored: CompiledCardArtifact =
        serde_json::from_slice(&serde_json::to_vec(&artifact).unwrap()).unwrap();
    for (observer, spell) in definitions("The War Doctor")
        .into_iter()
        .zip([direct, materialize_artifact(&restored).unwrap()])
    {
        let mut game = game();
        let source = game.create_object_from_definition(&observer, A, Zone::Battlefield);
        game.phase_out(source);
        settle(&mut game, &mut SelectFirstDecisionMaker);
        let other = resource(
            &mut game,
            A,
            Zone::Battlefield,
            "Type: Creature\nPower/Toughness: 2/2\nPhasing",
        );
        let spell = game.create_object_from_definition(&spell, A, Zone::Stack);
        game.push_to_stack(ironsmith::game_state::StackEntry::new(spell, A));
        resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
        assert!(!game.is_phased_out(source) && game.is_phased_out(other));
        assert_eq!(
            settle(&mut game, &mut SelectFirstDecisionMaker),
            0,
            "an observer absent before the simultaneous exchange cannot see its phase-out half"
        );
        assert_eq!(counters(&game, source, CounterType::Time), 0);
    }
}

#[test]
fn authored_sequential_phase_in_then_out_is_not_silently_made_simultaneous() {
    let text = "Mana cost: {0}\nType: Instant\nAll phased-out creatures phase in. All creatures with phasing phase out.";
    let (artifact, direct) =
        compile_to_artifact("Sequential phasing fixture", text, false).unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    for (observer, spell) in definitions("The War Doctor")
        .into_iter()
        .zip([direct, materialize_artifact(&restored).unwrap()])
    {
        assert!(
            !ironsmith_text::compiled_text_lines(&spell)
                .join(" ")
                .to_lowercase()
                .contains("simultaneously")
        );
        let mut game = game();
        let source = game.create_object_from_definition(&observer, A, Zone::Battlefield);
        let incoming = resource(
            &mut game,
            A,
            Zone::Battlefield,
            "Type: Creature\nPower/Toughness: 2/2\nPhasing",
        );
        game.phase_out(source);
        game.phase_out(incoming);
        settle(&mut game, &mut SelectFirstDecisionMaker);
        let outgoing = resource(
            &mut game,
            A,
            Zone::Battlefield,
            "Type: Creature\nPower/Toughness: 2/2\nPhasing",
        );
        let spell = game.create_object_from_definition(&spell, A, Zone::Stack);
        game.push_to_stack(ironsmith::game_state::StackEntry::new(spell, A));
        resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
        assert!(!game.is_phased_out(source));
        assert!(
            game.is_phased_out(incoming),
            "a later separate instruction may phase a just-returned permanent out again"
        );
        assert!(game.is_phased_out(outgoing));
        assert_eq!(
            settle(&mut game, &mut SelectFirstDecisionMaker),
            1,
            "the now-present observer sees the separate outgoing batch once"
        );
        assert_eq!(counters(&game, source, CounterType::Time), 1);
    }
}

#[test]
fn exchange_payload_keeps_both_filters_through_artifacts_and_legacy_defaults() {
    let text = "Mana cost: {U}\nType: Instant\nSimultaneously, all phased-out creatures phase in and all creatures with phasing phase out.";
    let (artifact, direct) = compile_to_artifact("Exchange fixture", text, false).unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    restored.validate().unwrap();
    for definition in [direct, materialize_artifact(&restored).unwrap()] {
        let effects = definition
            .spell_effect
            .as_ref()
            .unwrap()
            .flattened_default_effects();
        let exchange = effects
            .iter()
            .find_map(|effect| effect.downcast_ref::<PhaseInEffect>())
            .unwrap();
        let out = exchange
            .simultaneous_phase_out
            .as_ref()
            .expect("explicit second set must survive materialization");
        assert_eq!(out.card_types, vec![ironsmith::CardType::Creature]);
        assert!(
            out.static_abilities
                .contains(&ironsmith::static_abilities::StaticAbilityId::Phasing)
        );
        assert!(
            !effects
                .iter()
                .any(|effect| effect.downcast_ref::<PhaseOutEffect>().is_some())
        );
        assert!(
            ironsmith_text::compiled_text_lines(&definition)
                .join(" ")
                .to_lowercase()
                .contains("simultaneously")
        );
    }
    let legacy = ironsmith_core::PhaseInEffect::with_spec(ironsmith_core::ChooseSpec::all(
        ironsmith_core::ObjectFilter::creature(),
    ));
    let mut json = serde_json::to_value(legacy).unwrap();
    json.as_object_mut()
        .unwrap()
        .remove("simultaneous_phase_out");
    let restored: ironsmith_core::PhaseInEffect = serde_json::from_value(json).unwrap();
    assert!(restored.simultaneous_phase_out.is_none());
}

#[test]
fn public_compiler_rejects_unknown_simultaneous_compound_tails() {
    for oracle in [
        "Simultaneously, all phased-out creatures phase in and all creatures with phasing phase out and draw a card.",
        "Simultaneously, all phased-out creatures phase in and all creatures with phasing.",
    ] {
        let text = format!("Type: Instant\n{oracle}");
        assert!(
            compile_to_artifact("Unsupported simultaneous fixture", &text, false).is_err(),
            "a fallback must not split away simultaneity: {oracle}"
        );
    }
}
