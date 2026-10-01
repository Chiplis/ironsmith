//! Opt-in expected-result search probes. A passing reporter saves observations;
//! it does not assert that the observed cards are correct.
use ironsmith::card::PowerToughness;
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{DecisionMaker, GameProgress, LegalAction, compute_legal_actions};
use ironsmith::decisions::context::{BooleanContext, SelectObjectsContext};
use ironsmith::effect::{ChoiceAggregateMetric, Value as EffectValue};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, drain_pending_trigger_events, put_triggers_on_stack_with_dm,
    resolve_stack_entry_with,
};
use ironsmith::mana::{ManaCost, ManaSymbol};
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardDefinition, CardId, CardType, GameState, ObjectId, PlayerId, Zone};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

const SEED: u64 = 0x5345_4152_4348;

#[derive(Default)]
struct Decisions {
    mana_values: HashMap<ObjectId, i32>,
    contexts: Vec<Value>,
    invalid: Option<String>,
}
impl DecisionMaker for Decisions {
    fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool {
        true
    }
    fn decide_objects(&mut self, game: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        let mut chosen = Vec::new();
        let mut total = 0;
        for candidate in ctx.candidates.iter().filter(|c| c.legal) {
            if chosen.len() >= ctx.max.unwrap_or(usize::MAX) {
                break;
            }
            if let Some(constraint) = &ctx.aggregate_constraint {
                if constraint.minimum.is_some()
                    || constraint.metric != ChoiceAggregateMetric::ManaValue
                {
                    self.invalid =
                        Some("unsupported aggregate constraint in fixture decision".into());
                    break;
                }
                let EffectValue::Fixed(maximum) = constraint.maximum.unhinted() else {
                    self.invalid = Some("unevaluated aggregate maximum in fixture decision".into());
                    break;
                };
                let Some(value) = self.mana_values.get(&candidate.id) else {
                    self.invalid = Some("aggregate candidate missing fixture mana value".into());
                    break;
                };
                if total + value > *maximum {
                    continue;
                }
                total += value;
            }
            chosen.push(candidate.id);
        }
        if chosen.len() < ctx.min && !ctx.allow_partial_completion {
            self.invalid = Some("fixture could not complete mandatory selection".into());
        }
        self.contexts.push(json!({
            "description":ctx.description,"min":ctx.min,"max":ctx.max,
            "aggregate_constraint":format!("{:?}",ctx.aggregate_constraint),
            "allow_partial_completion":ctx.allow_partial_completion,
            "candidates":ctx.candidates.iter().map(|c| json!({
                "id":format!("{:?}",c.id),"legal":c.legal,
                "name":game.object(c.id).map(|o|o.name.to_string()),
                "fixture_mana_value":self.mana_values.get(&c.id),
                "power":game.object(c.id).and_then(|o|o.power()),
                "toughness":game.object(c.id).and_then(|o|o.toughness())
            })).collect::<Vec<_>>(),
            "chosen":chosen.iter().map(|id|format!("{id:?}")).collect::<Vec<_>>()
        }));
        chosen
    }
}
fn setup() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.set_random_seed(SEED);
    game.turn.turn_number = 3;
    game.turn.active_player = PlayerId(0);
    game.turn.priority_player = Some(PlayerId(0));
    game.turn.phase = ironsmith::Phase::FirstMain;
    game.turn.step = None;
    game
}
fn creature(name: &str, mv: u32, power: i32, toughness: i32) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(power, toughness))
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(
            mv as u8,
        )]]))
        .build()
}
fn resolve_pending(game: &mut GameState, dm: &mut Decisions) -> Result<usize, String> {
    let mut queue = TriggerQueue::new();
    let mut resolved = 0;
    for _ in 0..16 {
        drain_pending_trigger_events(game, &mut queue);
        put_triggers_on_stack_with_dm(game, &mut queue, dm).map_err(|e| e.to_string())?;
        if game.stack.is_empty() {
            return Ok(resolved);
        }
        resolve_stack_entry_with(game, dm).map_err(|e| e.to_string())?;
        resolved += 1;
    }
    Err("fixture stack resolution budget exceeded".into())
}
fn announce(game: &mut GameState, source: ObjectId, dm: &mut Decisions) -> Result<String, String> {
    let action = compute_legal_actions(game, PlayerId(0)).expect("fixture has complete replacement state")
        .into_iter()
        .find(|action| matches!(action,LegalAction::CastSpell{spell_id,..} if *spell_id==source))
        .ok_or("fixture had no legal creature cast")?;
    let action_text = format!("{action:?}");
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    )
    .map_err(|e| e.to_string())?;
    for _ in 0..24 {
        if state.pending_cast.is_none() && !game.stack.is_empty() {
            return Ok(action_text);
        }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else {
            return Err(format!("fixture announcement stalled: {progress:?}"));
        };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, dm)
            .map_err(|e| e.to_string())?;
    }
    Err("fixture announcement budget exceeded".into())
}
fn hulk(definition: &CardDefinition, values: &[u32]) -> Result<(Value, Value), String> {
    let mut game = setup();
    let mut dm = Decisions::default();
    let mut names = HashMap::new();
    for (index, value) in values.iter().copied().enumerate() {
        let name = format!("Search audit Hulk candidate {index}");
        let card = creature(&name, value, 1, 1);
        assert_ne!(definition.card.id, card.card.id);
        let object = game.create_object_from_definition(&card, PlayerId(0), Zone::Library);
        dm.mana_values.insert(object, value as i32);
        names.insert(name, value);
    }
    let source = game.create_object_from_definition(definition, PlayerId(0), Zone::Battlefield);
    game.effect_store.pending_trigger_events.clear();
    let dead = game
        .move_object_by_effect(source, Zone::Graveyard)
        .ok_or("fixture death move failed")?;
    if game.object(dead).is_none_or(|o| o.zone != Zone::Graveyard) {
        return Err("fixture Hulk did not enter graveyard".into());
    }
    let result = resolve_pending(&mut game, &mut dm);
    if let Some(error) = dm.invalid {
        return Err(error);
    }
    let fetched: Vec<_> = game
        .battlefield
        .iter()
        .filter_map(|id| game.object(*id))
        .filter_map(|o| {
            names
                .get(o.name.as_str())
                .map(|mv| json!({"name":o.name.to_string(),"mana_value":mv}))
        })
        .collect();
    let total: u32 = fetched
        .iter()
        .map(|v| v["mana_value"].as_u64().unwrap() as u32)
        .sum();
    Ok((
        json!({"resolution_error":result.as_ref().err(),"resolved_entries":result.ok(),
        "fetched_count":fetched.len(),"fetched_total_mana_value":total,"fetched":fetched}),
        json!({"death_recorded":true,"choice_contexts":dm.contexts}),
    ))
}
fn wild_pair(
    definition: &CardDefinition,
    pt: (i32, i32),
    cast: bool,
    source_cast_this_turn: bool,
) -> Result<(Value, Value), String> {
    let mut game = setup();
    let mut dm = Decisions::default();
    let library = creature("Search audit Wild Pair candidate", 2, pt.0, pt.1);
    let entering = creature("Search audit Wild Pair entering", 3, 3, 3);
    assert_ne!(definition.card.id, library.card.id);
    assert_ne!(definition.card.id, entering.card.id);
    assert_ne!(library.card.id, entering.card.id);
    let source_action = if source_cast_this_turn {
        let source = game.create_object_from_definition(definition, PlayerId(0), Zone::Hand);
        game.player_mut(PlayerId(0))
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Green, 6);
        let action = announce(&mut game, source, &mut dm)?;
        if resolve_pending(&mut game, &mut dm)? != 1
            || !game
                .battlefield
                .iter()
                .filter_map(|id| game.object(*id))
                .any(|o| o.name == definition.name())
        {
            return Err("fixture Wild Pair did not resolve its legal hand cast".into());
        }
        Some(action)
    } else {
        game.create_object_from_definition(definition, PlayerId(0), Zone::Battlefield);
        None
    };
    let candidate = game.create_object_from_definition(&library, PlayerId(0), Zone::Library);
    dm.mana_values.insert(candidate, 2);
    let entrant = game.create_object_from_definition(&entering, PlayerId(0), Zone::Hand);
    game.effect_store.pending_trigger_events.clear();
    let action = if cast {
        game.player_mut(PlayerId(0))
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Colorless, 3);
        Some(announce(&mut game, entrant, &mut dm)?)
    } else {
        game.move_object_by_effect(entrant, Zone::Battlefield)
            .ok_or("fixture move to battlefield failed")?;
        None
    };
    let result = resolve_pending(&mut game, &mut dm);
    if let Some(error) = dm.invalid {
        return Err(error);
    }
    let entering_present = game
        .battlefield
        .iter()
        .filter_map(|id| game.object(*id))
        .any(|o| o.name == entering.name());
    if !entering_present {
        return Err("fixture entrant never reached battlefield".into());
    }
    let fetched = game
        .battlefield
        .iter()
        .filter_map(|id| game.object(*id))
        .filter(|o| o.name == library.name())
        .count();
    Ok((
        json!({"resolution_error":result.as_ref().err(),"resolved_entries":result.ok(),"fetched_count":fetched}),
        json!({"source_legal_action":source_action,"legal_action":action,"entering_on_battlefield":entering_present,"mana_remaining":game.player(PlayerId(0)).unwrap().mana_pool.total(),
            "choice_contexts":dm.contexts}),
    ))
}
fn hash(path: &std::path::Path) -> String {
    Sha256::digest(std::fs::read(path).unwrap())
        .iter()
        .map(|v| format!("{v:02x}"))
        .collect()
}
#[test]
#[ignore = "audit reporter; inspect row status, not test exit status"]
fn report_runtime_search_value_reproductions() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let binary = std::env::current_exe().unwrap();
    let source = root.join("crates/ironsmith-tools/tests/runtime_search_value_reproductions.rs");
    let cards_path = root.join("cards.json");
    let before = json!({"source_sha256":hash(&source),"binary_sha256":hash(&binary),"cards_sha256":hash(&cards_path)});
    let mut rows = Vec::new();
    for name in ["Protean Hulk", "Wild Pair"] {
        let payload =
            ironsmith_tools::load_card_payloads_by_name(cards_path.to_str().unwrap(), name)
                .unwrap()
                .remove(0);
        let definition = match ironsmith_tools::compile_runtime_definition_from_payload(&payload) {
            Ok(value) => value,
            Err(error) => {
                rows.push(json!({"card":name,"status":"compile_failed","actual":{"error":error}}));
                continue;
            }
        };
        let definition_debug_sha256: String = Sha256::digest(format!("{definition:?}").as_bytes())
            .iter()
            .map(|v| format!("{v:02x}"))
            .collect();
        if name == "Protean Hulk" {
            for values in [vec![6, 6], vec![2, 4], vec![6]] {
                let expected = json!({"resolution_error":null,"maximum_fetched_total_mana_value":6,"all_candidates_if_total_at_most_six":true});
                let (status, actual, evidence) = match hulk(&definition, &values) {
                    Ok((actual, evidence)) => {
                        let correct = actual["resolution_error"].is_null()
                            && actual["resolved_entries"] == 1
                            && actual["fetched_total_mana_value"].as_u64().unwrap() <= 6
                            && (values.iter().sum::<u32>() > 6
                                || actual["fetched_count"].as_u64() == Some(values.len() as u64));
                        (
                            if correct {
                                "expected_result_observed"
                            } else {
                                "semantic_mismatch"
                            },
                            actual,
                            evidence,
                        )
                    }
                    Err(error) => ("fixture_error", json!({"error":error}), json!(null)),
                };
                rows.push(json!({"card":name,"scenario":{"library_mana_values":values,"event":"actual Battlefield-to-Graveyard move"},"status":status,"expected":expected,"actual":actual,"fixture_evidence":evidence,"definition_debug_sha256":definition_debug_sha256,"seed":SEED,"scope":"Canonical death trigger from actual zone change; proposed selection respects engine-exposed cardinality and aggregate constraints; actual battlefield count and mana-value sum verified."}));
            }
        } else {
            for source_cast_this_turn in [false, true] {
                for (pt, cast) in [
                    ((1, 1), true),
                    ((2, 4), true),
                    ((4, 4), true),
                    ((2, 4), false),
                ] {
                    let expected = json!({"resolution_error":null,"fetched_count":usize::from(cast && pt.0+pt.1==6),"resolved_entries":if cast {2}else{0}});
                    let (status, actual, evidence) =
                        match wild_pair(&definition, pt, cast, source_cast_this_turn) {
                            Ok((actual, evidence)) => (
                                if actual == expected {
                                    "expected_result_observed"
                                } else {
                                    "semantic_mismatch"
                                },
                                actual,
                                evidence,
                            ),
                            Err(error) => (
                                "fixture_or_announcement_error",
                                json!({"error":error}),
                                json!(null),
                            ),
                        };
                    rows.push(json!({"card":name,"scenario":{"entering_power_toughness":[3,3],"library_power_toughness":[pt.0,pt.1],"cast_from_hand":cast,"wild_pair_cast_from_hand_this_turn":source_cast_this_turn},"status":status,"expected":expected,"actual":actual,"fixture_evidence":evidence,"definition_debug_sha256":definition_debug_sha256,"seed":SEED,"scope":"Canonical Wild Pair, real legal creature cast from hand with paid mana or noncast negative control; engine-generated ETB trigger, legal search candidate list and actual battlefield result."}));
                }
            }
        }
    }
    let after = json!({"source_sha256":hash(&source),"binary_sha256":hash(&binary),"cards_sha256":hash(&cards_path)});
    let report = json!({"rows":rows,"provenance":{"binary":binary,"artifacts_before":before,"artifacts_after":after,"artifacts_unchanged":before==after,"compiled_via":"compile_runtime_definition_from_payload, unique CardId per definition"},"scope":"Two search-value families, eleven expected-result scenarios; not whole-card completeness."});
    let output = root.join("reports/runtime-audit/runtime-search-value-reproductions.json");
    std::fs::write(output, serde_json::to_string_pretty(&report).unwrap()).unwrap();
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
}
