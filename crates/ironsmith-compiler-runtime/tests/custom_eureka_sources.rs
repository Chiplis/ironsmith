//! Full raw Eureka family and source-defined token choices, without runtime name dispatch.
#[path = "custom_final_next_wave_support.rs"]
mod source;
use ironsmith::decision::{
    DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::{BooleanContext, SelectOptionsContext};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm,
};
use ironsmith::{CardDefinition, GameProgress, GameState, ObjectId, PlayerId, Zone};
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
fn fixture() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../../../fixtures/custom_mechanics/msem-eureka-complete.json"
    ))
    .unwrap()
}
fn definitions(key: &str) -> Vec<CardDefinition> {
    let data = fixture();
    let card = data["mechanics"][0]["representative_cards"]
        .as_array()
        .unwrap()
        .iter()
        .find(|card| card["source_card_key"] == key)
        .unwrap();
    assert!(
        card["executable_text"].is_string(),
        "parent must materialize the complete source fixture first: {key}"
    );
    source::definitions(card).map(Vec::from)
        .unwrap_or_else(|error| panic!("strict complete source {key}: {error}"))
}
struct Choices {
    mode: usize,
    mode_choices: usize,
    pay: bool,
}
impl DecisionMaker for Choices {
    fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool {
        self.pay
    }
    fn decide_targets(
        &mut self,
        game: &GameState,
        context: &ironsmith::decisions::context::TargetsContext,
    ) -> Vec<ironsmith::Target> {
        if !context.requirements.is_empty()
            && context.requirements.iter().all(|requirement| {
                requirement
                    .legal_targets
                    .contains(&ironsmith::Target::Player(B))
            })
        {
            vec![ironsmith::Target::Player(B)]
        } else {
            SelectFirstDecisionMaker.decide_targets(game, context)
        }
    }
    fn decide_options(&mut self, game: &GameState, context: &SelectOptionsContext) -> Vec<usize> {
        if context.options.len() == 3
            && context
                .options
                .iter()
                .all(|option| option.description.to_ascii_lowercase().contains("create"))
        {
            self.mode_choices += 1;
            return vec![context.options[self.mode].index];
        }
        SelectFirstDecisionMaker.decide_options(game, context)
    }
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    game.turn.step = None;
    game
}
fn research(game: &GameState) -> u32 {
    game.player(A)
        .unwrap()
        .counter_count(ironsmith::CounterType::Named("research".into()))
}
fn set_research(game: &mut GameState, count: u32) {
    game.player_mut(A)
        .unwrap()
        .other_counters
        .insert(ironsmith::CounterType::Named("research".into()), count);
}
fn put(game: &mut GameState, dm: &mut Choices) {
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    ironsmith::game_loop::drain_pending_trigger_events_with_dm(game, &mut queue, dm).unwrap();
    ironsmith::game_loop::put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
}
fn event(game: &mut GameState, dm: &mut Choices, draw: bool) {
    game.turn.active_player = if draw { A } else { B };
    game.turn.priority_player = Some(A);
    let event = if draw {
        ironsmith::triggers::TriggerEvent::new(
            ironsmith::events::BeginningOfDrawStepEvent::new(A),
            Default::default(),
        )
    } else {
        ironsmith::triggers::TriggerEvent::new(
            ironsmith::events::BeginningOfEndStepEvent::new(B),
            Default::default(),
        )
    };
    game.queue_trigger_event(Default::default(), event);
    put(game, dm);
}
fn settle(game: &mut GameState, dm: &mut Choices) {
    for _ in 0..16 {
        put(game, dm);
        if game.stack.is_empty() {
            return;
        }
        ironsmith::game_loop::resolve_stack_entry_with(game, dm).unwrap();
    }
    panic!("stack did not settle");
}
fn announce(game: &mut GameState, action: LegalAction, dm: &mut Choices) {
    let mut state = PriorityLoopState::new(2);
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    )
    .unwrap();
    for _ in 0..64 {
        if !state.has_pending_action() {
            ironsmith::game_loop::put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
            return;
        }
        let GameProgress::NeedsDecisionCtx(context) = progress else {
            panic!("{progress:?}")
        };
        progress =
            apply_decision_context_with_dm(game, &mut queue, &mut state, &context, dm).unwrap();
    }
    panic!("announcement stalled");
}
fn free_cast(game: &mut GameState, dm: &mut Choices, resolve: bool) {
    game.turn.priority_player = Some(A);
    let definition = ironsmith_compiler_runtime::compile_to_runtime_definition(
        "Response spell",
        "Mana cost: {0}\nType: Instant\nYou gain 1 life.",
        false,
    )
    .unwrap();
    let spell = game.create_object_from_definition(&definition, A, Zone::Hand);
    let action = compute_legal_actions(game, A)
        .unwrap()
        .into_iter()
        .find(|action| matches!(action,LegalAction::CastSpell{spell_id,..}if *spell_id==spell))
        .unwrap();
    announce(game, action, dm);
    if resolve {
        ironsmith::game_loop::resolve_stack_entry_with(game, dm).unwrap();
    }
}
fn token_ids(game: &GameState) -> Vec<ObjectId> {
    game.battlefield
        .iter()
        .copied()
        .filter(|id| game.object(*id).unwrap().kind == ironsmith::object::ObjectKind::Token)
        .collect()
}
#[test]
fn every_source_is_accounted_for_with_seven_whole_cards_and_two_exact_rule_dependencies() {
    let data=fixture();let cards=data["mechanics"][0]["representative_cards"].as_array().unwrap();
    assert_eq!(cards.len(),9);let mut complete=0;let mut blocked=std::collections::BTreeSet::new();
    let ledger:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/rule-boundaries.json")).unwrap();
    for card in cards{
        assert!(!card["original_text"].as_str().unwrap().is_empty());assert!(!card["source_faces"].as_array().unwrap().is_empty());
        if card["source_dependencies"].as_array().unwrap().is_empty(){
            let variants=definitions(card["source_card_key"].as_str().unwrap());assert_eq!(variants.len(),2);
            let native=ironsmith_runtime_catalog::artifact_materializer::encode_runtime_definition(variants[0].clone()).unwrap();
            let artifact=ironsmith_runtime_catalog::artifact_materializer::encode_runtime_definition(variants[1].clone()).unwrap();
            assert_eq!(serde_json::to_value(native).unwrap(),serde_json::to_value(artifact).unwrap());complete+=1;
        }else{
            assert_eq!(card["whole_source_status"],"dependent_on_excluded_new_rules");
            for dependency in card["whole_source_rule_boundaries"].as_array().unwrap(){
                let id=dependency["boundary_id"].as_str().unwrap();let entry=ledger["entries"].as_array().unwrap().iter().find(|entry|entry["id"]==id).expect("dependency must be an existing exact rule boundary");
                assert_eq!(entry["classification"],"unsupported_requires_new_rules");blocked.insert(id.to_owned());
            }
        }
    }
    assert_eq!(complete,7);assert_eq!(blocked, ["msem:rules:66:702.219.:Archive".to_owned(),"msem:sections:1:741.:Discovery Cards".to_owned()].into_iter().collect());
}
#[test]
fn eureka_ability_itself_runs_on_all_nine_source_clauses_independently_of_companion_rules(){
    let data=fixture();let cards=data["mechanics"][0]["representative_cards"].as_array().unwrap();
    for card in cards{
        let clause=&card["independent_eureka_clause"];assert_eq!(clause["claim_scope"],"eureka_ability_only_not_whole_card");
        let start=clause["source_span"]["start"].as_u64().unwrap()as usize;let end=clause["source_span"]["end"].as_u64().unwrap()as usize;
        let exact=card["original_text"].as_str().unwrap().chars().skip(start).take(end-start).collect::<String>();assert_eq!(exact,clause["source_span"]["text"].as_str().unwrap());
        // The isolated ability still creates source-supplied prototypes. Its
        // claim scope omits companion rules, not its executable dependencies.
        let mut admitted_clause = clause.clone();
        for field in ["custom_token_definitions", "custom_tokens"] {
            if let Some(value) = card.get(field) {
                admitted_clause[field] = value.clone();
            }
        }
        let variants=source::definitions(&admitted_clause).unwrap();
        let native=ironsmith_runtime_catalog::artifact_materializer::encode_runtime_definition(variants[0].clone()).unwrap();let artifact=ironsmith_runtime_catalog::artifact_materializer::encode_runtime_definition(variants[1].clone()).unwrap();assert_eq!(serde_json::to_value(native).unwrap(),serde_json::to_value(artifact).unwrap());
        for definition in variants{for mode in 0..3{
            let mut game=game();let mut dm=Choices{mode,mode_choices:0,pay:true};game.create_object_from_definition(&definition,A,Zone::Battlefield);set_research(&mut game,8);event(&mut game,&mut dm,false);settle(&mut game,&mut dm);assert_eq!(research(&game),0);assert_eq!(dm.mode_choices,1);assert_eq!(token_ids(&game).len(),1);
        }}
    }
}
#[test]
fn resolution_time_counter_gate_and_all_three_choices_survive_transport() {
    for definition in definitions("Avid Researcher_DOA") {
        for mode in 0..3 {
            for (initial, before, response, reward, remaining) in [
                (0, false, false, false, 0),
                (7, false, false, false, 7),
                (7, true, false, true, 0),
                (7, false, true, true, 0),
                (8, false, false, true, 0),
                (10, true, false, true, 0),
            ] {
                let mut game = game();
                let mut dm = Choices {
                    mode,
                    mode_choices: 0,
                    pay: true,
                };
                game.create_object_from_definition(&definition, A, Zone::Battlefield);
                set_research(&mut game, initial);
                game.player_mut(A).unwrap().energy_counters = 4;
                game.player_mut(B)
                    .unwrap()
                    .other_counters
                    .insert(ironsmith::CounterType::Named("research".into()), 12);
                if before {
                    free_cast(&mut game, &mut dm, true);
                }
                event(&mut game, &mut dm, false);
                assert_eq!(
                    game.stack.len(),
                    1,
                    "trigger exists even with no spell cast yet"
                );
                assert_eq!(dm.mode_choices, 0, "token choice waits for resolution");
                if response {
                    free_cast(&mut game, &mut dm, true);
                }
                settle(&mut game, &mut dm);
                assert_eq!(research(&game), remaining);
                assert_eq!(game.player(A).unwrap().energy_counters, 4);
                assert_eq!(
                    game.player(B)
                        .unwrap()
                        .counter_count(ironsmith::CounterType::Named("research".into())),
                    12
                );
                let tokens = token_ids(&game);
                assert_eq!(tokens.len(), usize::from(reward));
                assert_eq!(dm.mode_choices, usize::from(reward));
                if reward {
                    let token = game.object(tokens[0]).unwrap();
                    assert_eq!(
                        token.name.as_ref(),
                        [
                            "Cultivating Spheres",
                            "Mirror of Possibilities",
                            "Thrice-Folded Lotus"
                        ][mode]
                    );
                    assert!(token.card_types.contains(&ironsmith::CardType::Artifact));
                    assert_eq!(token.colors(), ironsmith::ColorSet::COLORLESS);
                    assert!(
                        token
                            .subtypes
                            .iter()
                            .any(|subtype| subtype.to_string().eq_ignore_ascii_case("Masterpiece"))
                    );
                    assert_eq!(token.abilities.len(), [2, 1, 2][mode]);
                }
            }
        }
    }
}
#[test]
fn multiple_sources_use_shared_player_counters_but_independent_end_step_triggers() {
    for definition in definitions("Avid Researcher_DOA") {
        let mut game = game();
        let mut dm = Choices {
            mode: 0,
            mode_choices: 0,
            pay: true,
        };
        for _ in 0..2 {
            game.create_object_from_definition(&definition, A, Zone::Battlefield);
        }
        set_research(&mut game, 7);
        free_cast(&mut game, &mut dm, true);
        event(&mut game, &mut dm, false);
        assert_eq!(game.stack.len(), 2);
        settle(&mut game, &mut dm);
        assert_eq!(research(&game), 1);
        assert_eq!(token_ids(&game).len(), 1);
        event(&mut game, &mut dm, false);
        settle(&mut game, &mut dm);
        assert_eq!(
            research(&game),
            3,
            "each additional end step triggers again"
        );
    }
}
#[test]
fn supplied_masterpiece_tokens_keep_their_real_draw_land_mana_and_copy_programs() {
    for definition in definitions("Avid Researcher_DOA") {
        for mode in 0..3 {
            let mut game = game();
            let mut dm = Choices {
                mode,
                mode_choices: 0,
                pay: true,
            };
            game.create_object_from_definition(&definition, A, Zone::Battlefield);
            set_research(&mut game, 8);
            event(&mut game, &mut dm, false);
            settle(&mut game, &mut dm);
            let token = token_ids(&game)[0];
            if mode == 0 {
                game.refresh_continuous_state().unwrap();
                assert_eq!(game.player(A).unwrap().land_plays_per_turn, 2);
                let card = ironsmith_compiler_runtime::compile_to_runtime_definition(
                    "Draw witness",
                    "Type: Land",
                    false,
                )
                .unwrap();
                let card = game.create_object_from_definition(&card, A, Zone::Library);
                let stable = game.object(card).unwrap().stable_id;
                event(&mut game, &mut dm, true);
                settle(&mut game, &mut dm);
                assert_eq!(
                    game.object(game.find_object_by_stable_id(stable).unwrap())
                        .unwrap()
                        .zone,
                    Zone::Hand
                );
            } else if mode == 2 {
                let action=compute_legal_actions(&game,A).unwrap().into_iter().find(|action|matches!(action,LegalAction::ActivateAbility{source,..}if *source==token)).unwrap();
                announce(&mut game, action, &mut dm);
                assert!(game.is_tapped(token));
                assert_eq!(game.player(A).unwrap().mana_pool.total(), 3);
                let pool = &game.player(A).unwrap().mana_pool;
                assert_eq!(pool.colorless, 0);
                assert_eq!(
                    [pool.white, pool.blue, pool.black, pool.red, pool.green]
                        .into_iter()
                        .filter(|count| *count > 0)
                        .count(),
                    1,
                    "three mana must all share one chosen color"
                );
                game.untap(token);
                let damage=compute_legal_actions(&game,A).unwrap().into_iter().find(|action|matches!(action,LegalAction::ActivateAbility{source,ability_index}if *source==token && *ability_index==1)).unwrap();
                announce(&mut game, damage, &mut dm);
                assert!(game.is_tapped(token));
                assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
                settle(&mut game, &mut dm);
                assert_eq!(game.player(B).unwrap().life, 17);
            } else {
                for paid in [false, true] {
                    dm.pay = paid;
                    game.player_mut(A)
                        .unwrap()
                        .mana_pool
                        .add(ironsmith::mana::ManaSymbol::Colorless, 3);
                    let life = game.player(A).unwrap().life;
                    free_cast(&mut game, &mut dm, false);
                    settle(&mut game, &mut dm);
                    assert_eq!(
                        game.player(A).unwrap().life,
                        life + if paid { 2 } else { 1 }
                    );
                }
            }
        }
    }
}

#[test]
fn missing_formal_token_blueprint_cannot_fall_back_to_an_empty_named_artifact() {
    let data = fixture();
    let mut card = data["mechanics"][0]["representative_cards"][0].clone();
    assert!(card["executable_text"].is_string());
    card["custom_token_definitions"]
        .as_array_mut()
        .unwrap()
        .pop();
    assert!(source::definitions(&card).is_err());
}
#[test]
fn trigger_uses_its_locked_controller_even_after_source_control_changes_or_departure() {
    for definition in definitions("Avid Researcher_DOA") {
        for departed in [false, true] {
            let mut game = game();
            let mut dm = Choices {
                mode: 0,
                mode_choices: 0,
                pay: true,
            };
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            set_research(&mut game, 8);
            game.player_mut(B)
                .unwrap()
                .other_counters
                .insert(ironsmith::CounterType::Named("research".into()), 12);
            event(&mut game, &mut dm, false);
            assert_eq!(game.stack.len(), 1);
            if departed {
                ironsmith::effects::execute_effect(
                    &mut game,
                    &ironsmith::Effect::new(ironsmith::effects::MoveToZoneEffect::new(
                        ironsmith::target::ChooseSpec::SpecificObject(source),
                        Zone::Graveyard,
                        false,
                    )),
                    &mut ironsmith::effects::EffectContext::new_default(source, A),
                )
                .unwrap();
            } else {
                game.set_current_controller(source, B).unwrap();
            }
            settle(&mut game, &mut dm);
            assert_eq!(research(&game), 0);
            assert_eq!(
                game.player(B)
                    .unwrap()
                    .counter_count(ironsmith::CounterType::Named("research".into())),
                12
            );
            let tokens = token_ids(&game);
            assert_eq!(tokens.len(), 1);
            assert_eq!(game.controller_of(game.object(tokens[0]).unwrap()), A);
        }
    }
}

#[test]
fn every_standalone_source_runs_its_real_end_step_program_and_each_formal_token_choice() {
    let data = fixture();
    let sources = data["mechanics"][0]["representative_cards"]
        .as_array()
        .unwrap();
    let ordinary = sources
        .iter()
        .filter(|card| card["source_dependencies"].as_array().unwrap().is_empty())
        .collect::<Vec<_>>();
    assert_eq!(
        ordinary.len(),
        7,
        "Only these seven sources have no excluded companion-rule dependency; all nine Eureka clauses are separately required"
    );
    for card in ordinary {
        for definition in definitions(card["source_card_key"].as_str().unwrap()) {
            for mode in 0..3 {
                let mut game = game();
                let mut dm = Choices {
                    mode,
                    mode_choices: 0,
                    pay: true,
                };
                game.create_object_from_definition(&definition, A, Zone::Battlefield);
                set_research(&mut game, 8);
                event(&mut game, &mut dm, false);
                assert_eq!(dm.mode_choices, 0);
                settle(&mut game, &mut dm);
                assert_eq!(research(&game), 0, "{}", card["name"]);
                assert_eq!(dm.mode_choices, 1, "{}", card["name"]);
                let tokens = token_ids(&game);
                assert_eq!(tokens.len(), 1, "{}", card["name"]);
                assert_eq!(
                    game.object(tokens[0]).unwrap().name.as_ref(),
                    [
                        "Cultivating Spheres",
                        "Mirror of Possibilities",
                        "Thrice-Folded Lotus"
                    ][mode]
                );
            }
        }
    }
}
