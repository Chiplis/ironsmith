//! Full-body native optional payment contracts. Authored, execution deferred.
use std::collections::VecDeque;
use ironsmith::card::CardBuilder;
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{DecisionMaker, SelectFirstDecisionMaker};
use ironsmith::decisions::context::{BooleanContext, ManaPaymentContext, SelectObjectsContext};
use ironsmith::effects::ExecutionError;
use ironsmith::events::{DamageEvent, DamageTarget, EventCause, EventKind};
use ironsmith::game_loop::{put_triggers_on_stack_with_dm, resolve_stack_entry_with};
use ironsmith::mana_payment::ManaPaymentResponse;
use ironsmith::replacement::{ReplacementAction, ReplacementEffect};
use ironsmith::target::ObjectFilter;
use ironsmith::triggers::{TriggerEvent, TriggerQueue, check_triggers};
use ironsmith::{CardId, CardType, Effect, GameState, ObjectId, PlayerId, Supertype, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;

const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);

fn definitions() -> [CardDefinition; 2] {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/same_name_relations.json.fixture"
    )).unwrap();
    let row = rows.iter().find(|row| row["name"] == "Canoptek Wraith").unwrap();
    let body = row["oracle_text"].as_str().unwrap();
    assert!(body.contains("If you do, choose a land you control. Then search your library"));
    let text = format!("Mana cost: {}\nType: {}\nPower/Toughness: {}/{}\n{}",
        row["mana_cost"].as_str().unwrap(), row["type_line"].as_str().unwrap(),
        row["power"].as_str().unwrap(), row["toughness"].as_str().unwrap(), body);
    let (direct, loss) = ironsmith_compiler::parse_loss::capture(|| {
        compile_to_runtime_definition("Canoptek Wraith", &text, false)
    });
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (artifact, loss) = ironsmith_compiler::parse_loss::capture(|| {
        compile_to_artifact("Canoptek Wraith", &text, false)
    });
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (artifact, _) = artifact.unwrap();
    artifact.validate().unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    restored.validate().unwrap();
    assert_eq!(restored, artifact);
    [direct.unwrap(), materialize_artifact(&restored).unwrap()]
}

fn land(game: &mut GameState, zone: Zone, name: &str, basic: bool) -> ObjectId {
    let card = CardBuilder::new(CardId::new(), name).card_types(vec![CardType::Land])
        .supertypes(if basic { vec![Supertype::Basic] } else { vec![] }).build();
    game.create_object_from_card(&card, A, zone)
}

#[derive(Default)]
struct Choices {
    decline: bool,
    cancel_mana: bool,
    pause_mana: bool,
    pause_boolean_at: Option<usize>,
    pending: bool,
    boolean_calls: usize,
    mana_calls: usize,
    object_calls: usize,
    objects: VecDeque<Vec<ObjectId>>,
}
impl DecisionMaker for Choices {
    fn awaiting_choice(&self) -> bool { self.pending }
    fn decide_boolean(&mut self, _: &GameState, ctx: &BooleanContext) -> bool {
        self.boolean_calls += 1;
        self.pending = self.pause_boolean_at == Some(self.boolean_calls);
        !self.decline && ctx.can_accept
    }
    fn decide_mana_payment(&mut self, _: &GameState, ctx: &ManaPaymentContext) -> ManaPaymentResponse {
        self.mana_calls += 1;
        self.pending = self.pause_mana;
        if self.cancel_mana { ManaPaymentResponse::Cancel } else {
            ManaPaymentResponse::Confirm { plan_id: ctx.plan.id, request_hash: ctx.plan.request_hash }
        }
    }
    fn decide_objects(&mut self, game: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        self.object_calls += 1;
        if let Some(chosen) = self.objects.pop_front() {
            for id in &chosen {
                assert!(ctx.candidates.iter().any(|candidate| candidate.id == *id && candidate.legal), "{ctx:?}");
            }
            chosen
        } else { SelectFirstDecisionMaker.decide_objects(game, ctx) }
    }
}

fn setup(definition: &CardDefinition, mana: u32, matches: usize) -> (GameState, ObjectId, Choices) {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.active_player = A;
    game.turn.phase = ironsmith::Phase::FirstMain;
    game.turn.step = None;
    game.player_mut(A).unwrap().mana_pool.colorless = mana;
    let source = game.create_object_from_definition(definition, A, Zone::Battlefield);
    let chosen = land(&mut game, Zone::Battlefield, "Chosen land", false);
    let found = (0..matches).map(|_| land(&mut game, Zone::Library, "Chosen land", true)).collect();
    land(&mut game, Zone::Library, "Chosen land", false);
    land(&mut game, Zone::Library, "Different basic", true);
    let choices = Choices { objects: VecDeque::from([vec![chosen], found]), ..Default::default() };
    game.take_pending_trigger_events();
    (game, source, choices)
}

fn queue_combat_trigger(game: &mut GameState, source: ObjectId, choices: &mut Choices) {
    let event = |combat| TriggerEvent::new(DamageEvent::with_cause(
        source, DamageTarget::Player(B), 2, combat, EventCause::from_effect(source, A),
    ), Default::default());
    assert!(check_triggers(game, &event(false)).is_empty());
    let triggers = check_triggers(game, &event(true));
    assert_eq!(triggers.len(), 1);
    let mut queue = TriggerQueue::new();
    for trigger in triggers { queue.add(trigger); }
    put_triggers_on_stack_with_dm(game, &mut queue, choices).unwrap();
    assert_eq!(game.stack.len(), 1);
}

fn count(game: &GameState, kind: EventKind) -> u32 {
    game.turn_store.turn_history.event_kind_count(kind)
}

fn assert_no_reward(game: &GameState, choices: &Choices) {
    assert_eq!(count(game, EventKind::Sacrifice), 0);
    assert_eq!(count(game, EventKind::SearchLibrary), 0);
    assert_eq!(count(game, EventKind::ShuffleLibrary), 0);
    assert_eq!(choices.object_calls, 0, "payment failure must not choose or search");
}

fn inspect_program(effect: &Effect, payment_owners: &mut usize, guarded_searches: &mut usize) {
    if let Some(optional) = effect.downcast_ref::<ironsmith::effects::MayEffect>() {
        assert!(optional.pay_as_cost, "the compound payment needs its typed transaction owner");
        *payment_owners += 1;
    }
    if let Some(conditional) = effect.downcast_ref::<ironsmith::effects::IfEffect>() {
        fn searches(effect: &Effect) -> usize {
            let mut count = usize::from(effect.downcast_ref::<ironsmith::effects::ChooseObjectsEffect>()
                .is_some_and(|choose| choose.is_search));
            effect.0.visit_child_effects(&mut |child| count += searches(child));
            count
        }
        *guarded_searches += conditional.then.iter().map(searches).sum::<usize>();
    }
    effect.0.visit_child_effects(&mut |child| inspect_program(child, payment_owners, guarded_searches));
}

#[test]
fn full_body_keeps_compound_owner_and_third_sentence_inside_success_branch() {
    for definition in definitions() {
        let mut payment_owners = 0;
        let mut guarded_searches = 0;
        for ability in &definition.abilities {
            if let ironsmith::ability::AbilityKind::Triggered(trigger) = &ability.kind {
                for effect in trigger.effects.all_effects() {
                    inspect_program(effect, &mut payment_owners, &mut guarded_searches);
                }
            }
        }
        assert_eq!(payment_owners, 1);
        assert_eq!(guarded_searches, 1, "the full authored Then-search must depend on complete payment");
    }
}

#[test]
fn full_payment_is_required_before_choice_search_and_shuffle() {
    for definition in definitions() {
        for scenario in 0..4 {
            let funded = scenario != 1;
            let (mut game, source, mut choices) = setup(&definition, if funded { 3 } else { 0 }, 2);
            let source_stable = game.object(source).unwrap().stable_id;
            let found = choices.objects[1].iter().map(|id| game.object(*id).unwrap().stable_id).collect::<Vec<_>>();
            let library = game.player(A).unwrap().library.clone();
            choices.decline = scenario == 2;
            choices.cancel_mana = scenario == 3;
            queue_combat_trigger(&mut game, source, &mut choices);
            resolve_stack_entry_with(&mut game, &mut choices).unwrap();
            assert!(game.stack_is_empty());
            let paid = scenario == 0;
            assert_eq!(game.player(A).unwrap().mana_pool.total(), if funded && !paid { 3 } else { 0 });
            let current = game.find_object_by_stable_id(source_stable).unwrap();
            assert_eq!(game.object(current).unwrap().zone, if paid { Zone::Graveyard } else { Zone::Battlefield });
            if paid {
                assert_eq!(count(&game, EventKind::Sacrifice), 1);
                assert_eq!(choices.object_calls, 2, "choose the land once, then search once");
                assert_eq!(count(&game, EventKind::SearchLibrary), 1);
                assert_eq!(count(&game, EventKind::ShuffleLibrary), 1);
                for stable in found {
                    let id = game.find_object_by_stable_id(stable).unwrap();
                    assert_eq!(game.object(id).unwrap().zone, Zone::Battlefield);
                    assert!(game.is_tapped(id));
                }
            } else {
                assert_no_reward(&game, &choices);
                assert_eq!(game.player(A).unwrap().library, library);
            }
            assert_eq!(choices.mana_calls, usize::from(scenario == 0 || scenario == 3));
        }
    }
}

#[test]
fn departed_returned_or_stolen_source_cannot_pay_and_never_spends_mana() {
    for definition in definitions() {
        for fate in 0..3 {
            let (mut game, source, mut choices) = setup(&definition, 3, 2);
            let stable = game.object(source).unwrap().stable_id;
            queue_combat_trigger(&mut game, source, &mut choices);
            match fate {
                0 => { game.move_object_by_effect(source, Zone::Graveyard).unwrap(); }
                1 => {
                    game.move_object_by_effect(source, Zone::Exile).unwrap();
                    let departed = game.find_object_by_stable_id(stable).unwrap();
                    game.move_object_by_effect(departed, Zone::Battlefield).unwrap();
                }
                _ => { game.set_current_controller(source, B).unwrap(); }
            }
            game.take_pending_trigger_events();
            let current = game.find_object_by_stable_id(stable).unwrap();
            let zone = game.object(current).unwrap().zone;
            let controller = game.current_controller(current);
            resolve_stack_entry_with(&mut game, &mut choices).unwrap();
            assert_eq!(game.player(A).unwrap().mana_pool.total(), 3);
            assert_eq!(choices.mana_calls, 0, "preflight must reject the complete cost");
            assert_eq!(game.object(current).unwrap().zone, zone);
            assert_eq!(game.current_controller(current), controller);
            assert_no_reward(&game, &choices);
        }
    }
}

#[test]
fn successful_payment_with_no_matching_basics_still_searches_and_shuffles() {
    for definition in definitions() {
        let (mut game, source, mut choices) = setup(&definition, 3, 0);
        let stable = game.object(source).unwrap().stable_id;
        let library_count = game.player(A).unwrap().library.len();
        queue_combat_trigger(&mut game, source, &mut choices);
        resolve_stack_entry_with(&mut game, &mut choices).unwrap();
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        assert_eq!(game.object(game.find_object_by_stable_id(stable).unwrap()).unwrap().zone, Zone::Graveyard);
        assert_eq!(game.player(A).unwrap().library.len(), library_count);
        assert_eq!(count(&game, EventKind::SearchLibrary), 1);
        assert_eq!(count(&game, EventKind::ShuffleLibrary), 1);
    }
}

fn add_sacrifice_replacement(game: &mut GameState, source: ObjectId, effects: Vec<Effect>) {
    game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(
        source, A,
        ironsmith::events::zones::matchers::WouldChangeZoneMatcher::new(
            ObjectFilter::specific(source), Some(Zone::Battlefield), Some(Zone::Graveyard),
        ),
        ReplacementAction::Additionally(effects),
    ));
}

#[test]
fn pending_mana_or_sacrifice_addition_replays_without_partial_cost_or_reward() {
    for definition in definitions() {
        for pause_during_sacrifice in [false, true] {
            let (mut game, source, mut choices) = setup(&definition, 3, 2);
            let stable = game.object(source).unwrap().stable_id;
            if pause_during_sacrifice {
                add_sacrifice_replacement(&mut game, source, vec![Effect::may(vec![Effect::gain_life(1)])]);
                choices.pause_boolean_at = Some(2);
            } else { choices.pause_mana = true; }
            queue_combat_trigger(&mut game, source, &mut choices);
            let stack_id = game.stack.last().unwrap().object_id;
            let library = game.player(A).unwrap().library.clone();
            resolve_stack_entry_with(&mut game, &mut choices).unwrap();
            assert!(choices.pending);
            assert_eq!(game.stack.last().unwrap().object_id, stack_id);
            assert_eq!(game.player(A).unwrap().mana_pool.total(), 3);
            assert_eq!(game.object(source).unwrap().zone, Zone::Battlefield);
            assert_eq!(game.player(A).unwrap().life, 20);
            assert_eq!(game.player(A).unwrap().library, library);
            assert_no_reward(&game, &choices);
            choices.pending = false;
            choices.pause_mana = false;
            choices.pause_boolean_at = None;
            resolve_stack_entry_with(&mut game, &mut choices).unwrap();
            assert!(game.stack_is_empty());
            assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
            assert_eq!(game.object(game.find_object_by_stable_id(stable).unwrap()).unwrap().zone, Zone::Graveyard);
            assert_eq!(game.player(A).unwrap().life, if pause_during_sacrifice { 21 } else { 20 });
            assert_eq!(count(&game, EventKind::SearchLibrary), 1);
            assert_eq!(count(&game, EventKind::ShuffleLibrary), 1);
        }
    }
}

#[test]
fn sacrifice_execution_failure_is_typed_atomic_and_retryable() {
    for definition in definitions() {
        let (mut game, source, mut choices) = setup(&definition, 3, 2);
        let stable = game.object(source).unwrap().stable_id;
        add_sacrifice_replacement(&mut game, source, vec![Effect::new(
            ironsmith::effects::CreateTokenEffect::you(
                ironsmith::cards::tokens::treasure_token_definition(), 1,
            ),
        )]);
        queue_combat_trigger(&mut game, source, &mut choices);
        game.set_token_creation_limits(ironsmith::effects::tokens::TokenCreationLimits {
            max_created_tokens: 0, ..Default::default()
        });
        let stack_id = game.stack.last().unwrap().object_id;
        let objects = game.objects_in_deterministic_order().len();
        let next_id = game.next_object_id_counter();
        let error = resolve_stack_entry_with(&mut game, &mut choices).unwrap_err();
        assert!(matches!(error, ironsmith::game_loop::GameLoopError::ExecutionFailed(
            ExecutionError::ResourceLimitExceeded { .. }
        )), "{error:?}");
        assert_eq!(game.stack.last().unwrap().object_id, stack_id);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 3);
        assert_eq!(game.object(source).unwrap().zone, Zone::Battlefield);
        assert_eq!(game.objects_in_deterministic_order().len(), objects);
        assert_eq!(game.next_object_id_counter(), next_id);
        assert_no_reward(&game, &choices);
        game.set_token_creation_limits(Default::default());
        resolve_stack_entry_with(&mut game, &mut choices).unwrap();
        assert!(game.stack_is_empty());
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        assert_eq!(game.object(game.find_object_by_stable_id(stable).unwrap()).unwrap().zone, Zone::Graveyard);
        assert_eq!(count(&game, EventKind::SearchLibrary), 1);
        assert_eq!(count(&game, EventKind::ShuffleLibrary), 1);
        assert_eq!(game.battlefield.iter().filter(|id| game.object(**id).unwrap().name == "Treasure").count(), 1);
    }
}

#[test]
fn replacement_modified_original_sacrifice_completes_the_cost() {
    for definition in definitions() {
        for mode in 0..3 {
            let (mut game, source, mut choices) = setup(&definition, 3, 2);
            let stable = game.object(source).unwrap().stable_id;
            let found = choices.objects[1].iter().map(|id| game.object(*id).unwrap().stable_id).collect::<Vec<_>>();
            let action = match mode {
                0 => ReplacementAction::Prevent,
                1 => ReplacementAction::ChangeDestination(Zone::Exile),
                _ => ReplacementAction::Instead(vec![Effect::gain_life(5)]),
            };
            let replacement = game.effect_store.replacement_effects.add_one_shot_effect(
                ReplacementEffect::with_matcher(
                    source, A,
                    ironsmith::events::zones::matchers::WouldChangeZoneMatcher::new(
                        ObjectFilter::specific(source), Some(Zone::Battlefield), Some(Zone::Graveyard),
                    ),
                    action,
                ),
            );
            queue_combat_trigger(&mut game, source, &mut choices);
            resolve_stack_entry_with(&mut game, &mut choices).unwrap();
            assert!(game.stack_is_empty());
            assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
            let current = game.find_object_by_stable_id(stable).unwrap();
            assert_eq!(game.object(current).unwrap().zone, if mode == 1 { Zone::Exile } else { Zone::Battlefield });
            assert_eq!(game.player(A).unwrap().life, if mode == 2 { 25 } else { 20 });
            assert!(game.effect_store.replacement_effects.get_effect(replacement).is_none());
            assert_eq!(count(&game, EventKind::SearchLibrary), 1);
            assert_eq!(count(&game, EventKind::ShuffleLibrary), 1);
            for stable in found {
                let id = game.find_object_by_stable_id(stable).unwrap();
                assert_eq!(game.object(id).unwrap().zone, Zone::Battlefield);
                assert!(game.is_tapped(id));
            }
        }
    }
}

#[test]
fn paid_cost_with_no_land_choice_keeps_an_empty_result_and_finishes_shuffle() {
    for definition in definitions() {
        let (mut game, source, mut choices) = setup(&definition, 3, 2);
        let stable = game.object(source).unwrap().stable_id;
        let unavailable_land = choices.objects[0][0];
        game.move_object_by_effect(unavailable_land, Zone::Exile).unwrap();
        // A stale source antecedent would incorrectly find this basic land.
        land(&mut game, Zone::Library, "Canoptek Wraith", true);
        choices.objects.clear();
        let library = game.player(A).unwrap().library.clone();
        queue_combat_trigger(&mut game, source, &mut choices);
        resolve_stack_entry_with(&mut game, &mut choices).unwrap();
        assert!(game.stack_is_empty());
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        assert_eq!(game.object(game.find_object_by_stable_id(stable).unwrap()).unwrap().zone, Zone::Graveyard);
        assert_eq!(count(&game, EventKind::SearchLibrary), 1);
        assert_eq!(count(&game, EventKind::ShuffleLibrary), 1);
        assert_eq!(game.player(A).unwrap().library.len(), library.len());
        for id in library { assert_eq!(game.object(id).unwrap().zone, Zone::Library); }
        assert_eq!(choices.object_calls, 0, "known-empty choice must not reuse the source or invent a land");
    }
}
