//! Ring designation, action evidence and exact-incarnation regressions.
//! Authored under the source-only campaign. These scenarios have not been run.
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{DecisionMaker, SelectFirstDecisionMaker};
use ironsmith::decisions::context::{BooleanContext, SelectObjectsContext};
use ironsmith::effects::{EffectContext, EffectExecutor, RingTemptsYouEffect};
use ironsmith::events::EventCause;
use ironsmith::game_loop::{put_triggers_on_stack_with_dm, resolve_stack_entry_with};
use ironsmith::target::ChooseSpec;
use ironsmith::triggers::{TriggerEvent, TriggerQueue};
use ironsmith::{GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/ring_bearer_references.json.fixture"
    ))
    .unwrap()
}
fn compile(name: &str, text: &str) -> [CardDefinition; 2] {
    let (artifact, direct) =
        compile_to_artifact(name, text, false).unwrap_or_else(|error| panic!("{name}: {error}"));
    let restored: CompiledCardArtifact =
        serde_json::from_slice(&serde_json::to_vec(&artifact).unwrap()).unwrap();
    restored.validate().unwrap();
    [direct, materialize_artifact(&restored).unwrap()]
}
fn definitions(name: &str) -> [CardDefinition; 2] {
    let fixture = fixtures()
        .into_iter()
        .find(|row| row["name"] == name)
        .unwrap();
    compile(name, fixture["text"].as_str().unwrap())
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.phase = ironsmith::Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(A);
    game
}
fn object(game: &mut GameState, owner: PlayerId, zone: Zone, name: &str, text: &str) -> ObjectId {
    let definition = compile_to_runtime_definition(name, text, false).unwrap();
    game.create_object_from_definition(&definition, owner, zone)
}
fn creature(game: &mut GameState, owner: PlayerId) -> ObjectId {
    object(
        game,
        owner,
        Zone::Battlefield,
        "Bearer candidate",
        "Type: Creature\nPower/Toughness: 2/2",
    )
}
fn library(game: &mut GameState, owner: PlayerId, count: usize, lands: bool) {
    for _ in 0..count {
        object(
            game,
            owner,
            Zone::Library,
            "Library card",
            if lands {
                "Type: Land"
            } else {
                "Type: Sorcery\nDraw a card."
            },
        );
    }
}
struct Decisions {
    chosen: Option<ObjectId>,
    yes: bool,
}
impl DecisionMaker for Decisions {
    fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool {
        self.yes
    }
    fn decide_objects(&mut self, _: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        if let Some(id) = self.chosen
            && ctx
                .candidates
                .iter()
                .any(|candidate| candidate.id == id && candidate.legal)
        {
            return vec![id];
        }
        ctx.candidates
            .iter()
            .filter(|candidate| candidate.legal)
            .take(ctx.min)
            .map(|candidate| candidate.id)
            .collect()
    }
}
fn pending(game: &mut GameState, dm: &mut impl DecisionMaker) -> usize {
    put_triggers_on_stack_with_dm(game, &mut TriggerQueue::new(), dm).unwrap();
    game.stack.len()
}
fn settle(game: &mut GameState, dm: &mut impl DecisionMaker) {
    pending(game, dm);
    for _ in 0..24 {
        if game.stack_is_empty() {
            return;
        }
        resolve_stack_entry_with(game, dm).unwrap();
        pending(game, dm);
    }
    panic!("Ring trigger chain did not settle");
}
fn apply(
    game: &mut GameState,
    source: ObjectId,
    actor: PlayerId,
    effect: &dyn EffectExecutor,
    dm: &mut impl DecisionMaker,
) {
    let mut context = EffectContext::new(source, actor, dm);
    let outcome = effect.execute(game, &mut context).unwrap();
    for event in outcome.events {
        game.queue_trigger_event(event.provenance(), event);
    }
}
fn tempt(game: &mut GameState, source: ObjectId, actor: PlayerId, dm: &mut impl DecisionMaker) {
    apply(game, source, actor, &RingTemptsYouEffect::you(), dm);
}
fn enter(game: &mut GameState, object: ObjectId) -> ObjectId {
    game.move_object(object, Zone::Battlefield, EventCause::effect())
        .unwrap()
}
#[test]
fn exact_complete_ring_fixtures_round_trip_all_bodies() {
    let fixtures = fixtures();
    assert_eq!(fixtures.len(), 11);
    assert_eq!(
        fixtures
            .iter()
            .filter(|row| row["proposed_complete"] == true)
            .count(),
        10
    );
    for fixture in fixtures
        .into_iter()
        .filter(|row| row["proposed_complete"] == true)
    {
        for definition in compile(
            fixture["name"].as_str().unwrap(),
            fixture["text"].as_str().unwrap(),
        ) {
            assert!(!definition.abilities.is_empty());
            assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
        }
    }
}
#[test]
fn call_requires_a_real_choice_including_reselection_and_retains_the_actual_actor() {
    for definition in definitions("Call of the Ring") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        library(&mut game, A, 6, false);
        let mut dm = Decisions {
            chosen: None,
            yes: true,
        };
        tempt(&mut game, source, A, &mut dm);
        assert_eq!(pending(&mut game, &mut dm), 0);
        assert_eq!(game.ring_temptations(A), 1);
        let bearer = creature(&mut game, A);
        dm.chosen = Some(bearer);
        for expected in [1, 2] {
            tempt(&mut game, source, A, &mut dm);
            assert_eq!(pending(&mut game, &mut dm), 1);
            settle(&mut game, &mut dm);
            assert_eq!(game.player(A).unwrap().hand.len(), expected);
            assert_eq!(game.player(A).unwrap().life, 20 - 2 * expected as i32);
        }
        creature(&mut game, B);
        tempt(&mut game, source, B, &mut dm);
        assert_eq!(pending(&mut game, &mut dm), 0);
        dm.yes = false;
        tempt(&mut game, source, A, &mut dm);
        settle(&mut game, &mut dm);
        assert_eq!(game.player(A).unwrap().hand.len(), 2);
        assert_eq!(game.player(A).unwrap().life, 16);
    }
}
#[test]
fn gandalf_uses_historical_other_choice_after_reselection_and_departure() {
    for definition in definitions("Gandalf, Friend of the Shire") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let other = creature(&mut game, A);
        library(&mut game, A, 4, false);
        let mut dm = Decisions {
            chosen: Some(source),
            yes: true,
        };
        tempt(&mut game, source, A, &mut dm);
        assert_eq!(pending(&mut game, &mut dm), 0);
        dm.chosen = Some(other);
        tempt(&mut game, source, A, &mut dm);
        assert_eq!(pending(&mut game, &mut dm), 1);
        game.set_ring_bearer(A, source);
        game.move_object(other, Zone::Graveyard, EventCause::effect())
            .unwrap();
        settle(&mut game, &mut dm);
        assert_eq!(game.player(A).unwrap().hand.len(), 1);
        assert_eq!(game.current_ring_bearer(A), Some(source));
    }
}
#[test]
fn faramir_creates_the_full_token_body_and_retains_its_death_history_end_step() {
    for definition in definitions("Faramir, Field Commander") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let other = creature(&mut game, A);
        library(&mut game, A, 3, false);
        let mut dm = Decisions {
            chosen: Some(other),
            yes: true,
        };
        tempt(&mut game, source, A, &mut dm);
        settle(&mut game, &mut dm);
        assert!(game.battlefield.iter().any(|id| {
            game.object(*id).is_some_and(|object| {
                object.name == "Human Soldier"
                    && object.kind == ironsmith::object::ObjectKind::Token
            })
        }));
        apply(
            &mut game,
            source,
            A,
            &ironsmith::effects::DestroyEffect::with_spec(ChooseSpec::SpecificObject(other)),
            &mut dm,
        );
        game.queue_trigger_event(
            Default::default(),
            TriggerEvent::new_with_provenance(
                ironsmith::events::BeginningOfEndStepEvent::new(A),
                Default::default(),
            ),
        );
        settle(&mut game, &mut dm);
        assert_eq!(game.player(A).unwrap().hand.len(), 1);
    }
}
#[test]
fn galadriel_scry_followup_reveals_and_moves_the_land_tapped() {
    for definition in definitions("Galadriel of Lothlórien") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let bearer = creature(&mut game, A);
        library(&mut game, A, 4, true);
        let mut dm = Decisions {
            chosen: Some(bearer),
            yes: true,
        };
        tempt(&mut game, source, A, &mut dm);
        settle(&mut game, &mut dm);
        let lands: Vec<_> = game
            .battlefield
            .iter()
            .copied()
            .filter(|id| {
                game.object(*id)
                    .is_some_and(|object| object.card_types.contains(&ironsmith::CardType::Land))
            })
            .collect();
        assert_eq!(lands.len(), 1);
        assert!(game.is_tapped(lands[0]));
        assert_eq!(game.player(A).unwrap().library.len(), 3);
    }
}
#[test]
fn rangers_recheck_current_designation_at_resolution_and_ignore_other_players_bearers() {
    for definition in definitions("Dúnedain Rangers") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let bearer = creature(&mut game, A);
        let opponents_bearer = creature(&mut game, B);
        game.set_ring_bearer(B, opponents_bearer);
        let mut dm = Decisions {
            chosen: Some(bearer),
            yes: true,
        };
        let land = object(&mut game, A, Zone::Hand, "Landfall land", "Type: Land");
        enter(&mut game, land);
        assert_eq!(pending(&mut game, &mut dm), 1);
        game.set_ring_bearer(A, bearer);
        settle(&mut game, &mut dm);
        assert_eq!(game.ring_temptations(A), 0);
        game.clear_ring_bearer(A);
        let land = object(&mut game, A, Zone::Hand, "Next land", "Type: Land");
        enter(&mut game, land);
        settle(&mut game, &mut dm);
        assert_eq!(game.ring_temptations(A), 1);
        assert_eq!(game.current_ring_bearer(A), Some(bearer));
        assert!(game.object(source).is_some());
    }
}
#[test]
fn elven_queen_votes_then_counters_the_current_bearer_without_adding_a_target() {
    for definition in definitions("Galadriel, Elven-Queen") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let other = object(
            &mut game,
            A,
            Zone::Hand,
            "Other Elf",
            "Type: Creature — Elf\nPower/Toughness: 2/2",
        );
        let bearer = enter(&mut game, other);
        let mut dm = Decisions {
            chosen: Some(bearer),
            yes: true,
        };
        game.queue_trigger_event(
            Default::default(),
            TriggerEvent::new_with_provenance(
                ironsmith::events::BeginningOfCombatEvent::new(A),
                Default::default(),
            ),
        );
        settle(&mut game, &mut dm);
        assert_eq!(game.ring_temptations(A), 1);
        assert_eq!(game.current_ring_bearer(A), Some(bearer));
        assert_eq!(
            game.counter_count(bearer, ironsmith::object::CounterType::PlusOnePlusOne),
            1
        );
        assert_eq!(
            game.counter_count(source, ironsmith::object::CounterType::PlusOnePlusOne),
            0
        );
    }
}
#[test]
fn saga_uses_current_bearer_power_for_each_player_and_retains_later_chapters() {
    use ironsmith::effects::PutCountersEffect;
    use ironsmith::object::CounterType;
    for definition in definitions("One Ring to Rule Them All") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let bearer = creature(&mut game, A);
        let victim = creature(&mut game, B);
        library(&mut game, A, 6, false);
        library(&mut game, B, 6, false);
        let mut dm = Decisions {
            chosen: Some(bearer),
            yes: true,
        };
        for chapter in 1..=3 {
            apply(
                &mut game,
                source,
                A,
                &PutCountersEffect::new(CounterType::Lore, 1, ChooseSpec::SpecificObject(source)),
                &mut dm,
            );
            settle(&mut game, &mut dm);
            if chapter == 1 {
                assert_eq!(game.player(A).unwrap().library.len(), 4);
                assert_eq!(game.player(B).unwrap().library.len(), 4);
            } else if chapter == 2 {
                assert!(game.object(victim).is_none());
                assert!(game.object(bearer).is_some(), "Ring-bearer is legendary");
            }
        }
        assert_eq!(game.player(B).unwrap().life, 19);
    }
}
#[test]
fn lord_protection_reads_live_designation_and_departed_source_lki() {
    for definition in definitions("Lord of the Nazgûl") {
        let mut game = game();
        let lord = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let source = creature(&mut game, B);
        let alternative = creature(&mut game, B);
        game.set_ring_bearer(B, source);
        game.refresh_continuous_state().unwrap();
        assert!(ironsmith::has_protection_from_source(&game, lord, source));
        let snapshot =
            ironsmith::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
                game.object(source).unwrap(),
                &game,
            );
        let mut dm = SelectFirstDecisionMaker;
        apply(
            &mut game,
            source,
            B,
            &ironsmith::effects::DealDamageEffect::new(1, ChooseSpec::SpecificObject(lord)),
            &mut dm,
        );
        assert_eq!(game.damage_on(lord), 0);
        game.set_ring_bearer(B, alternative);
        assert!(!ironsmith::has_protection_from_source(&game, lord, source));
        apply(
            &mut game,
            source,
            B,
            &ironsmith::effects::DealDamageEffect::new(1, ChooseSpec::SpecificObject(lord)),
            &mut dm,
        );
        assert_eq!(game.damage_on(lord), 1);
        game.set_ring_bearer(B, source);
        let snapshot =
            ironsmith::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
                game.object(source).unwrap(),
                &game,
            );
        game.move_object(source, Zone::Graveyard, EventCause::effect())
            .unwrap();
        let mut context = EffectContext::new(source, B, &mut dm).with_source_snapshot(snapshot);
        ironsmith::effects::DealDamageEffect::new(1, ChooseSpec::SpecificObject(lord))
            .execute(&mut game, &mut context)
            .unwrap();
        assert_eq!(
            game.damage_on(lord),
            1,
            "departed source uses its captured designation"
        );
    }
}
#[test]
fn frodo_block_requirement_tracks_current_designation_and_control() {
    for definition in definitions("Frodo Baggins") {
        let mut game = game();
        let card = game.create_object_from_definition(&definition, A, Zone::Hand);
        let source = enter(&mut game, card);
        let mut dm = Decisions {
            chosen: Some(source),
            yes: true,
        };
        settle(&mut game, &mut dm);
        game.refresh_continuous_state().unwrap();
        assert_eq!(game.current_ring_bearer(A), Some(source));
        assert!(game.must_be_blocked(source));
        let other = creature(&mut game, A);
        game.set_ring_bearer(A, other);
        game.refresh_continuous_state().unwrap();
        assert!(!game.must_be_blocked(source));
        game.set_ring_bearer(A, source);
        apply(
            &mut game,
            other,
            B,
            &ironsmith::effects::GainControlEffect::new(
                ChooseSpec::SpecificObject(source),
                ironsmith::effect::Until::Forever,
            ),
            &mut dm,
        );
        game.refresh_continuous_state().unwrap();
        assert_eq!(game.current_ring_bearer(A), None);
        assert!(!game.must_be_blocked(source));
    }
}

#[test]
fn sauron_delays_the_unless_test_and_never_follows_a_blinked_source() {
    use ironsmith::combat_state::{AttackTarget, CombatState};
    use ironsmith::decision::AttackerDeclaration;
    for definition in definitions("Sauron, the Necromancer") {
        // No bearer: exile. Same source designated after registration: keep.
        // Source blinks, then its new incarnation is designated: still exile.
        for mode in 0..4 {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let graveyard_card = object(
                &mut game,
                A,
                Zone::Graveyard,
                "Borrowed creature",
                "Type: Creature — Elf\nPower/Toughness: 6/6",
            );
            game.remove_summoning_sickness(source);
            game.turn.phase = ironsmith::Phase::Combat;
            game.turn.step = Some(ironsmith::game_state::Step::DeclareAttackers);
            let mut combat = CombatState::default();
            let mut queue = TriggerQueue::new();
            ironsmith::game_loop::apply_attacker_declarations(
                &mut game,
                &mut combat,
                &mut queue,
                &[AttackerDeclaration {
                    creature: source,
                    target: AttackTarget::Player(B),
                }],
            )
            .unwrap();
            game.combat = Some(combat);
            let mut dm = Decisions {
                chosen: Some(graveyard_card),
                yes: true,
            };
            put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
            settle(&mut game, &mut dm);
            let token = *game
                .battlefield
                .iter()
                .find(|id| {
                    game.object(**id).is_some_and(|object| {
                        object.name == "Borrowed creature"
                            && object.kind == ironsmith::object::ObjectKind::Token
                    })
                })
                .unwrap();
            assert!(game.is_tapped(token));
            assert!(ironsmith::combat_state::is_attacking(
                game.combat.as_ref().unwrap(),
                token
            ));
            assert_eq!(game.calculated_power(token), Some(3));
            if mode == 1 {
                game.set_ring_bearer(A, source);
            }
            if mode == 2 {
                let exiled = game
                    .move_object(source, Zone::Exile, EventCause::effect())
                    .unwrap();
                let returned = game
                    .move_object(exiled, Zone::Battlefield, EventCause::effect())
                    .unwrap();
                assert_ne!(returned, source);
                game.set_ring_bearer(A, returned);
            }
            if mode == 3 {
                game.set_ring_bearer(A, source);
                game.phase_out(source);
                assert_eq!(
                    game.current_ring_bearer(A),
                    Some(source),
                    "phasing preserves the stored designation"
                );
            }
            game.queue_trigger_event(
                Default::default(),
                TriggerEvent::new_with_provenance(
                    ironsmith::events::BeginningOfEndStepEvent::new(A),
                    Default::default(),
                ),
            );
            settle(&mut game, &mut dm);
            assert_eq!(
                game.object(token).is_some(),
                mode == 1,
                "delayed Ring condition mode {mode}"
            );
        }
    }
}

#[test]
fn current_bearer_reference_is_not_a_new_choice_and_no_bearer_does_not_stop_followup() {
    for definition in compile(
        "Current bearer reference",
        "Type: Sorcery\nPut a +1/+1 counter on your Ring-bearer. Each player mills cards equal to your Ring-bearer's power. You gain 2 life.",
    ) {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Stack);
        let mut dm = SelectFirstDecisionMaker;
        library(&mut game, A, 3, false);
        library(&mut game, B, 3, false);
        let mut context = EffectContext::new(source, A, &mut dm);
        let effects = definition.spell_effect.as_ref().unwrap();
        for effect in effects.flattened_default_effects() {
            ironsmith::effects::execute_effect(&mut game, &effect, &mut context).unwrap();
        }
        assert_eq!(game.player(A).unwrap().life, 22);
        assert_eq!(game.player(A).unwrap().library.len(), 3);
        assert_eq!(game.player(B).unwrap().library.len(), 3);
        assert!(context.targets.is_empty());
    }
}

fn cast_zero_cost_spell(game: &mut GameState, dm: &mut impl DecisionMaker) {
    use ironsmith::game_loop::{
        PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
        apply_priority_response_with_dm,
    };
    let hand = object(
        game,
        A,
        Zone::Hand,
        "Bearer cast",
        "Mana cost: {0}\nType: Sorcery\nYou gain 1 life.",
    );
    game.turn.priority_player = Some(A);
    let action = ironsmith::decision::LegalAction::CastSpell {
        spell_id: hand,
        from_zone: Zone::Hand,
        casting_method: ironsmith::alternative_cast::CastingMethod::Normal,
    };
    assert!(
        ironsmith::decision::compute_legal_actions(game, A)
            .unwrap()
            .contains(&action)
    );
    let mut state = PriorityLoopState::new(game.players.len());
    let mut queue = TriggerQueue::new();
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    )
    .unwrap();
    for _ in 0..32 {
        if state.pending_cast.is_none() && state.pending_method_selection.is_none() {
            break;
        }
        let ironsmith::GameProgress::NeedsDecisionCtx(context) = progress else {
            panic!("cast pending without a choice")
        };
        progress =
            apply_decision_context_with_dm(game, &mut queue, &mut state, &context, dm).unwrap();
    }
    assert!(state.pending_cast.is_none() && state.pending_method_selection.is_none());
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
    settle(game, dm);
}
#[test]
fn gandalf_keeps_its_real_sorcery_flash_permission_on_the_opponents_turn() {
    for definition in definitions("Gandalf, Friend of the Shire") {
        let mut game = game();
        game.create_object_from_definition(&definition, A, Zone::Battlefield);
        game.turn.active_player = B;
        game.turn.phase = ironsmith::Phase::Combat;
        game.turn.step = Some(ironsmith::game_state::Step::DeclareBlockers);
        cast_zero_cost_spell(&mut game, &mut SelectFirstDecisionMaker);
        assert_eq!(game.player(A).unwrap().life, 21);
    }
}
#[test]
fn lord_cast_body_creates_the_ninth_wraith_then_sets_the_entire_groups_base_size() {
    for definition in definitions("Lord of the Nazgûl") {
        let mut game = game();
        let lord = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        for _ in 0..7 {
            object(
                &mut game,
                A,
                Zone::Battlefield,
                "Earlier Wraith",
                "Type: Creature — Wraith\nPower/Toughness: 1/1",
            );
        }
        cast_zero_cost_spell(&mut game, &mut SelectFirstDecisionMaker);
        let wraiths: Vec<_> = game
            .battlefield
            .iter()
            .copied()
            .filter(|id| {
                game.object(*id).is_some_and(|object| {
                    object.subtypes.contains(&ironsmith::types::Subtype::Wraith)
                })
            })
            .collect();
        assert_eq!(wraiths.len(), 9);
        assert!(
            wraiths
                .iter()
                .all(|id| game.calculated_power(*id) == Some(9)
                    && game.calculated_toughness(*id) == Some(9))
        );
        ironsmith::turn::execute_cleanup_step(&mut game);
        game.refresh_continuous_state().unwrap();
        assert_eq!(game.calculated_power(lord), Some(4));
        assert!(wraiths.iter().any(|id| {
            game.object(*id)
                .is_some_and(|object| object.kind == ironsmith::object::ObjectKind::Token)
                && game.calculated_power(*id) == Some(3)
        }));
    }
}

// Known, unignored secondary-body regression. It remains outside proposed
// complete coverage until counter-choice and triggering counter-kind readers
// both preserve their full executable meaning.
#[test]
fn aragorn_complete_counter_bodies_remain_a_tracked_gap() {
    for definition in definitions("Aragorn, Company Leader") {
        assert_eq!(
            definition
                .abilities
                .iter()
                .filter(|ability| matches!(
                    ability.kind,
                    ironsmith::ability::AbilityKind::Triggered(_)
                ))
                .count(),
            2
        );
    }
}
