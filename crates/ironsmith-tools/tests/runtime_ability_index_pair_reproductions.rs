//! Scoped paid action probes for conditional Anthem ability ordering.
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{
    DecisionMaker, GameProgress, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::{BooleanContext, SelectOptionsContext, TargetsContext};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, drain_pending_trigger_events, put_triggers_on_stack_with_dm,
};
use ironsmith::game_state::Target;
use ironsmith::mana::ManaSymbol;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{
    CardDefinition, CardId, CardType, GameState, ObjectId, PlayerId, PowerToughness, Zone,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
fn hash(p: &std::path::Path) -> String {
    Sha256::digest(std::fs::read(p).unwrap())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
struct Choices {
    accept: bool,
    target: Option<Target>,
    trace: Vec<Value>,
}
impl DecisionMaker for Choices {
    fn answers_player_choices(&self) -> bool {
        false
    }
    fn decide_boolean(&mut self, _: &GameState, c: &BooleanContext) -> bool {
        self.trace
            .push(json!({"decision":"boolean","context":format!("{c:?}"),"answer":self.accept}));
        self.accept
    }
    fn decide_options(&mut self, g: &GameState, c: &SelectOptionsContext) -> Vec<usize> {
        let a = SelectFirstDecisionMaker.decide_options(g, c);
        self.trace
            .push(json!({"decision":"options","context":format!("{c:?}"),"answer":a}));
        a
    }
    fn decide_targets(&mut self, _g: &GameState, c: &TargetsContext) -> Vec<Target> {
        let a = c
            .requirements
            .iter()
            .filter_map(|r| {
                self.target
                    .filter(|t| r.legal_targets.contains(t))
                    .or_else(|| r.legal_targets.first().copied())
            })
            .collect::<Vec<_>>();
        self.trace.push(
            json!({"decision":"targets","context":format!("{c:?}"),"answer":format!("{a:?}")}),
        );
        a
    }
}
fn basic(name: &str, kind: CardType) -> CardDefinition {
    let b = CardDefinitionBuilder::new(CardId::new(), name).card_types(vec![kind]);
    if kind == CardType::Creature {
        b.power_toughness(PowerToughness::fixed(2, 6)).build()
    } else {
        b.build()
    }
}
fn cast(
    g: &mut GameState,
    def: &CardDefinition,
    actor: PlayerId,
    q: &mut TriggerQueue,
    dm: &mut Choices,
) -> Result<u32, String> {
    eprintln!("AUDIT_STAGE cast {}", def.name());
    g.turn.priority_player = Some(actor);
    let id = g.create_object_from_definition(def, actor, Zone::Hand);
    let action = compute_legal_actions(g, actor).expect("fixture has complete replacement state")
        .into_iter()
        .find(|a| matches!(a,LegalAction::CastSpell{spell_id,..}if *spell_id==id))
        .ok_or_else(|| format!("{} normal cast unavailable", def.name()))?;
    let before = g.player(actor).unwrap().mana_pool.total();
    let mut state = PriorityLoopState::new(g.players_in_game());
    dm.trace.push(json!({"stage":"legal_cast","card":def.name(),"actor":actor.index(),"action":format!("{action:?}")}));
    let mut progress = apply_priority_response_with_dm(
        g,
        q,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    )
    .map_err(|e| e.to_string())?;
    for _ in 0..32 {
        if state.pending_cast.is_none()
            && g.stack.iter().any(|e| {
                !e.is_ability && g.object(e.object_id).is_some_and(|o| o.name == def.name())
            })
        {
            let paid = before - g.player(actor).unwrap().mana_pool.total();
            dm.trace.push(json!({"stage":"cast_complete","card":def.name(),"paid":paid,"stack":format!("{:?}",g.stack)}));
            return Ok(paid);
        }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else {
            return Err(format!("cast stalled:{progress:?}"));
        };
        progress = apply_decision_context_with_dm(g, q, &mut state, &ctx, dm)
            .map_err(|e| e.to_string())?;
    }
    Err("cast announcement bound".into())
}
fn resolve_one(g: &mut GameState, q: &mut TriggerQueue, dm: &mut Choices) -> Result<(), String> {
    let mut state = PriorityLoopState::new(g.players_in_game());
    state.reset_for_new_priority_window(g);
    // The final pass invokes the production resolver, which retains the trigger
    // queue and therefore produces dedicated ETB events as well as zone changes.
    for _ in 0..g.players_in_game() {
        let progress = apply_priority_response_with_dm(
            g,
            q,
            &mut state,
            &PriorityResponse::PriorityAction(LegalAction::PassPriority),
            dm,
        )
        .map_err(|e| e.to_string())?;
        if let GameProgress::NeedsDecisionCtx(ctx) = &progress {
            if !matches!(
                ctx,
                ironsmith::decisions::context::DecisionContext::Priority(_)
            ) {
                return Err(format!(
                    "priority resolution requires unhandled decision:{ctx:?}"
                ));
            }
        }
    }
    Ok(())
}
fn drain(g: &mut GameState, q: &mut TriggerQueue, dm: &mut Choices) -> Result<(), String> {
    drain_pending_trigger_events(g, q);
    put_triggers_on_stack_with_dm(g, q, dm).map_err(|e| e.to_string())
}
fn resolve_all(g: &mut GameState, q: &mut TriggerQueue, dm: &mut Choices) -> Result<(), String> {
    for _ in 0..24 {
        drain(g, q, dm)?;
        if g.stack.is_empty() {
            return Ok(());
        }
        resolve_one(g, q, dm)?;
    }
    Err("bounded resolution did not empty stack".into())
}
use ironsmith::combat_state::{AttackTarget, CombatState};
use ironsmith::decision::AttackerDeclaration;
fn announce(
    g: &mut GameState,
    q: &mut TriggerQueue,
    dm: &mut Choices,
    action: LegalAction,
) -> Result<(), String> {
    g.turn.priority_player = Some(PlayerId(0));
    eprintln!("AUDIT_STAGE activate {action:?}");
    let old = g.stack.len();
    let mut state = PriorityLoopState::new(g.players_in_game());
    dm.trace
        .push(json!({"stage":"activation_action","action":format!("{action:?}")}));
    let mut p = apply_priority_response_with_dm(
        g,
        q,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    )
    .map_err(|e| e.to_string())?;
    for _ in 0..32 {
        if state.pending_activation.is_none() && g.stack.len() > old {
            return Ok(());
        }
        let GameProgress::NeedsDecisionCtx(ctx) = p else {
            return Err(format!("activation stalled:{p:?}"));
        };
        p = apply_decision_context_with_dm(g, q, &mut state, &ctx, dm)
            .map_err(|e| e.to_string())?;
    }
    Err("activation bound".into())
}
fn start_next_turn(
    g: &mut GameState,
    q: &mut TriggerQueue,
    dm: &mut Choices,
) -> Result<(), String> {
    g.next_turn();
    ironsmith::turn::execute_untap_step(g);
    ironsmith::turn::advance_step(g).map_err(|e| e.to_string())?;
    ironsmith::turn::advance_step(g).map_err(|e| e.to_string())?;
    let events = ironsmith::turn::execute_draw_step_with(g, dm).unwrap();
    for event in events {
        for t in ironsmith::triggers::check_triggers(g, &event) {
            q.add(t);
        }
    }
    resolve_all(g, q, dm)?;
    ironsmith::turn::advance_phase(g).map_err(|e| e.to_string())?;
    g.turn.priority_player = Some(g.turn.active_player);
    dm.trace.push(json!({"stage":"next_turn_main","active":g.turn.active_player.index(),"turn":g.turn.turn_number,"phase":format!("{:?}",g.turn.phase)}));
    Ok(())
}
fn next_own_turn(g: &mut GameState, q: &mut TriggerQueue, dm: &mut Choices) -> Result<(), String> {
    for _ in 0..g.players_in_game() {
        start_next_turn(g, q, dm)?;
    }
    Ok(())
}

use ironsmith::Supertype;

use ironsmith::Subtype;
fn mana(g: &mut GameState) {
    for sym in [
        ManaSymbol::White,
        ManaSymbol::Blue,
        ManaSymbol::Black,
        ManaSymbol::Red,
        ManaSymbol::Green,
        ManaSymbol::Colorless,
    ] {
        g.player_mut(PlayerId(0)).unwrap().mana_pool.add(sym, 12);
    }
}
fn run(def: &CardDefinition, companion: &CardDefinition, paired: bool, _: usize) -> Result<Value,String> {
    let mut g=GameState::new(vec!["Alice".into(),"Bob".into()],20);
    g.set_random_seed(71757432704855);
    g.turn.turn_number=3;g.turn.active_player=PlayerId(0);g.turn.priority_player=Some(PlayerId(0));
    g.turn.phase=ironsmith::Phase::FirstMain;g.turn.step=None;mana(&mut g);
    let witness_def=CardDefinitionBuilder::new(CardId::new(),"Established Human recipient")
        .card_types(vec![CardType::Creature]).subtypes(vec![Subtype::Human])
        .power_toughness(PowerToughness::fixed(2,4)).build();
    let witness=g.create_object_from_definition(&witness_def,PlayerId(0),Zone::Battlefield);
    let mut dm=Choices{accept:true,target:Some(Target::Object(witness)),trace:vec![]};let mut q=TriggerQueue::new();
    let mut ids=vec![];let mut payments=vec![];
    for definition in std::iter::once(def).chain(paired.then_some(companion)) {
        eprintln!("AUDIT_STAGE cast_start {}",definition.name());
        let paid=cast(&mut g,definition,PlayerId(0),&mut q,&mut dm)?;
        if paid!=2{return Err(format!("source {} cast payment {paid}, expected2",definition.name()));}
        resolve_all(&mut g,&mut q,&mut dm)?;
        let id=*g.battlefield.iter().find(|id|g.object(**id).is_some_and(|o|o.name==definition.name())).ok_or("source did not enter")?;
        ids.push(id);payments.push(json!({"stage":"cast","card":definition.name(),"paid":paid}));
        eprintln!("AUDIT_STAGE cast_complete {}",definition.name());
    }
    let mut announcement_error=None;let mut resolution_error=None;
    for id in ids.iter().copied() {
        eprintln!("AUDIT_STAGE legal_equip {id:?}");g.turn.priority_player=Some(PlayerId(0));
        let action=compute_legal_actions(&g,PlayerId(0)).expect("fixture has complete replacement state").into_iter().find(|a|matches!(a,LegalAction::ActivateAbility{source,..}if *source==id)).ok_or("normal equip unavailable")?;
        let before=g.player(PlayerId(0)).unwrap().mana_pool.total();
        announcement_error=announce(&mut g,&mut q,&mut dm,action).err();
        if announcement_error.is_some(){break;}
        let paid=before-g.player(PlayerId(0)).unwrap().mana_pool.total();
        if paid!=2{return Err(format!("equip payment {paid}, expected2"));}
        payments.push(json!({"stage":"equip","source":id.0,"paid":paid}));
        eprintln!("AUDIT_STAGE resolve_equip {id:?}");
        resolution_error=resolve_all(&mut g,&mut q,&mut dm).err();
        if resolution_error.is_some(){break;}
        if g.object(id).unwrap().attached_to!=Some(ironsmith::object::AttachmentTarget::Object(witness)){return Err("successful equip did not attach to explicit witness".into());}
        eprintln!("AUDIT_STAGE equip_complete {id:?}");
    }
    eprintln!("AUDIT_STAGE final_characteristics");
    let first_strike=g.current_has_static_ability_id(witness,ironsmith::static_abilities::StaticAbilityId::FirstStrike);
    let deathtouch=g.current_has_static_ability_id(witness,ironsmith::static_abilities::StaticAbilityId::Deathtouch);
    let expected=json!({"announcement_error":null,"resolution_error":null,"remaining_stack":0,"power":if paired{6}else{4},"toughness":if paired{8}else{4},"first_strike":paired,"deathtouch":paired});
    let actual=json!({"announcement_error":announcement_error,"resolution_error":resolution_error,"remaining_stack":g.stack.len(),"power":g.calculated_power(witness),"toughness":g.calculated_toughness(witness),"first_strike":first_strike,"deathtouch":deathtouch});
    eprintln!("AUDIT_STAGE complete");
    Ok(json!({"expected":expected,"actual":actual,"state_evidence":{"payments":payments,"witness":witness.0,"sources":ids.iter().map(|id|id.0).collect::<Vec<_>>(),"paired":paired},"execution_trace":dm.trace}))
}
#[test]
#[ignore = "manual scoped activation-index report"]
fn report_ability_index_pair() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let inventory =
        root.join("reports/runtime-audit/corpus/267a16aff3b321196397d0b4/inventory.json");
    let candidates = root.join("reports/runtime-audit/ability-index-structural-candidates.json");
    let source =
        root.join("crates/ironsmith-tools/tests/runtime_ability_index_pair_reproductions.rs");
    let binary = std::env::current_exe().unwrap();
    let paths = [&inventory, &candidates, &source, &binary];
    let before: Vec<_> = paths
        .iter()
        .map(|p| json!({"path":p,"sha256":hash(p)}))
        .collect();
    let inv: Value = serde_json::from_slice(&std::fs::read(&inventory).unwrap()).unwrap();
    let names: Vec<String> = ["Bride's Gown", "Groom's Finery"]
    .iter()
    .map(|n| n.to_string())
    .collect();
    let mut compile = vec![];
    let mut defs = std::collections::HashMap::new();
    for name in names.iter().map(String::as_str) {
        let p = inv["cards"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["name"] == name)
            .unwrap();
        let (artifact, def) = ironsmith_registry::compile_builder_to_artifact(
            ironsmith_compiler::CardDefinitionBuilder::new(CardId::new(), name),
            p["parse_input"].as_str().unwrap(),
            false,
        )
        .unwrap();
        compile.push(json!({"card":name,"artifact_checksum":artifact.payload_checksum,"definition":artifact.payload.definition}));
        defs.insert(name.to_string(), def);
    }
    let mut rows = vec![];
    for name in names {
        if std::env::var("AUDIT_INDEX_CARD").is_ok_and(|selected|selected != name) {continue;}
        let def = &defs[&name];
        let activations = 1;
        for condition_on in [false, true] {
            if std::env::var("AUDIT_INDEX_CONDITION").is_ok_and(|selected|selected != condition_on.to_string()) {continue;}
            eprintln!("AUDIT_CASE {name} {condition_on}");
            for ordinal in 0..activations {
                let (status, out) = match run(def, &defs[if name == "Bride's Gown" {"Groom's Finery"} else {"Bride's Gown"}], condition_on, ordinal)
                {
                    Ok(v) => {
                        let status = if v["actual"]["announcement_error"].is_string() {
                            "action_or_choice_failed"
                        } else if v["actual"]["resolution_error"].is_string() {
                            "resolution_failed"
                        } else if v["actual"] == v["expected"] {
                            "expected_result_observed"
                        } else {
                            "semantic_mismatch"
                        };
                        (status, v)
                    }
                    Err(e) => (
                        "fixture_or_execution_error",
                        json!({"expected":null,"actual":{"error":e}}),
                    ),
                };
                rows.push(json!({"card":name,"scenario":{"condition_on":condition_on,"source_activation_ordinal":ordinal},"status":status,"expected":out["expected"],"actual":out["actual"],"state_evidence":out["state_evidence"],"execution_trace":out["execution_trace"],"artifact_checksum":compile.iter().find(|c|c["card"]==name).unwrap()["artifact_checksum"],"scope":"Minimal actual paid canonical Equipment casts and ordinary paid Equip resolutions, one established Human witness, no Hammer or unrelated library permanents. Paired scenario casts and equips both sources to the same creature. Exact power/toughness and keyword outcomes asserted only if the process completes."}));
            }
        }
    }
    let after: Vec<_> = paths
        .iter()
        .map(|p| json!({"path":p,"sha256":hash(p)}))
        .collect();
    let report = json!({"rows":rows,"compilation":compile,"provenance":{"before":before,"after":after,"artifacts_unchanged":before==after,"strict_artifact":true,"unique_card_ids":true,"seed":71757432704855_u64}});
    std::fs::write(
        std::env::var("AUDIT_INDEX_OUTPUT").map(std::path::PathBuf::from).unwrap_or_else(|_|root.join("reports/runtime-audit/ability-index-pair-reproductions.json")),
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
    println!("wrote{}cases", report["rows"].as_array().unwrap().len());
}
