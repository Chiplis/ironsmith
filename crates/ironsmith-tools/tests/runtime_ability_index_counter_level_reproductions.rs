//! Paid conditional-index transitions caused by costs and current-turn counter history.
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
use ironsmith::{CardDefinition, CardId, CardType, GameState, ObjectId, PlayerId, Zone};
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
    resources: Vec<ObjectId>,
    x: u32,
    activation_hint: Option<&'static str>,
}
impl DecisionMaker for Choices {
    fn decide_number(&mut self, _: &GameState, c: &ironsmith::decisions::context::NumberContext) -> u32 {
        let x=self.x.clamp(c.min,c.max);
        self.trace.push(json!({"decision":"number","context":format!("{c:?}"),"answer":x})); x
    }
    fn answers_player_choices(&self) -> bool {
        false
    }
    fn decide_boolean(&mut self, _: &GameState, c: &BooleanContext) -> bool {
        self.trace
            .push(json!({"decision":"boolean","context":format!("{c:?}"),"answer":self.accept}));
        self.accept
    }
    fn decide_options(&mut self, g: &GameState, c: &SelectOptionsContext) -> Vec<usize> {
        let a = if let Some(o) = c
            .options
            .iter()
            .find(|o| o.legal && o.description == "Untap")
        {
            vec![o.index]
        } else {
            SelectFirstDecisionMaker.decide_options(g, c)
        };
        self.trace
            .push(json!({"decision":"options","context":format!("{c:?}"),"answer":a}));
        a
    }
    fn decide_objects(
        &mut self,
        g: &GameState,
        c: &ironsmith::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        let preferred: Vec<_> = self
            .resources
            .iter()
            .copied()
            .filter(|id| c.candidates.iter().any(|o| o.id == *id && o.legal))
            .collect();
        let a = if preferred.len() >= c.min && !preferred.is_empty() {
            preferred
                .into_iter()
                .take(c.max.unwrap_or(self.resources.len()))
                .collect()
        } else {
            SelectFirstDecisionMaker.decide_objects(g, c)
        };
        self.trace.push(
            json!({"decision":"objects","context":format!("{c:?}"),"answer":format!("{a:?}")}),
        );
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

fn mana(g: &mut GameState) {
    for p in [PlayerId(0), PlayerId(1)] {
        for symbol in [
            ManaSymbol::White,
            ManaSymbol::Blue,
            ManaSymbol::Black,
            ManaSymbol::Red,
            ManaSymbol::Green,
            ManaSymbol::Colorless,
        ] {
            g.player_mut(p).unwrap().mana_pool.add(symbol, 12);
        }
    }
}
fn find(g: &GameState, name: &str) -> Result<ObjectId, String> {
    g.battlefield
        .iter()
        .copied()
        .find(|id| g.object(*id).is_some_and(|o| o.name == name))
        .ok_or_else(|| format!("{name} absent from battlefield"))
}

fn advance_turn(g: &mut GameState, q: &mut TriggerQueue, dm: &mut Choices) -> Result<(), String> {
    g.next_turn();
    ironsmith::turn::execute_untap_step(g);
    ironsmith::turn::advance_step(g).map_err(|e| e.to_string())?;
    ironsmith::turn::advance_step(g).map_err(|e| e.to_string())?;
    for e in ironsmith::turn::execute_draw_step_with(g, dm) {
        for t in ironsmith::triggers::check_triggers(g, &e) {
            q.add(t);
        }
    }
    resolve_all(g, q, dm)?;
    ironsmith::turn::advance_phase(g).map_err(|e| e.to_string())?;
    g.turn.priority_player = Some(PlayerId(0));
    mana(g);
    Ok(())
}
fn announce_mana(
    g: &mut GameState,
    q: &mut TriggerQueue,
    dm: &mut Choices,
    action: LegalAction,
) -> Result<(), String> {
    let mut state = PriorityLoopState::new(g.players_in_game());
    let mut progress = apply_priority_response_with_dm(
        g,
        q,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    )
    .map_err(|e| e.to_string())?;
    for _ in 0..32 {
        if let GameProgress::NeedsDecisionCtx(ctx) = progress {
            if matches!(
                ctx,
                ironsmith::decisions::context::DecisionContext::Priority(_)
            ) {
                return Ok(());
            }
            progress = apply_decision_context_with_dm(g, q, &mut state, &ctx, dm)
                .map_err(|e| e.to_string())?;
        } else {
            return Ok(());
        }
    }
    Err("mana announcement bound".into())
}

fn activation(
    g: &mut GameState,
    q: &mut TriggerQueue,
    dm: &mut Choices,
    source: ObjectId,
) -> Result<Value, String> {
    g.turn.priority_player = Some(PlayerId(0));
    let a = compute_legal_actions(g, PlayerId(0)).expect("fixture has complete replacement state")
        .into_iter()
        .find(|a| matches!(a,LegalAction::ActivateAbility{source:s,..}|LegalAction::ActivateManaAbility{source:s,..}if *s==source) && dm.activation_hint.is_none_or(|hint| match a { LegalAction::ActivateAbility{ability_index,..}|LegalAction::ActivateManaAbility{ability_index,..} => g.current_abilities(source).is_some_and(|abilities|abilities.get(*ability_index).is_some_and(|a|format!("{a:?}").contains(hint))), _=>false }));
    let before = g.player(PlayerId(0)).unwrap().mana_pool.total() as i64;
    let mut error = None;
    let mut resolution_error = None;
    if let Some(action) = a.clone() {
        dm.trace.push(json!({"stage":"advertised_source_action","action":format!("{action:?}"),"abilities":format!("{:?}",g.current_abilities(source))}));
        error = if matches!(action, LegalAction::ActivateManaAbility { .. }) {
            announce_mana(g, q, dm, action).err()
        } else {
            announce(g, q, dm, action).err()
        };
        if error.is_none() {
            resolution_error = resolve_all(g, q, dm).err();
        }
    }
    Ok(
        json!({"offered":a.is_some(),"announcement_error":error,"resolution_error":resolution_error,"mana_paid":before-g.player(PlayerId(0)).unwrap().mana_pool.total() as i64,"remaining_stack":g.stack.len()}),
    )
}
fn run(def: &CardDefinition, defs: &std::collections::HashMap<&str,CardDefinition>, mode:usize)->Result<Value,String>{
    use ironsmith::static_abilities::StaticAbilityId as K;
    use ironsmith::object::AttachmentTarget;
    let mut g=GameState::new(vec!["Alice".into(),"Bob".into()],20);
    g.set_random_seed(71757432704855);g.turn.turn_number=3;g.turn.active_player=PlayerId(0);g.turn.priority_player=Some(PlayerId(0));g.turn.phase=ironsmith::Phase::FirstMain;g.turn.step=None;mana(&mut g);
    let filler=CardDefinitionBuilder::new(CardId::new(),"Neutral library artifact").card_types(vec![CardType::Artifact]).build();
    for p in [PlayerId(0),PlayerId(1)] {for _ in 0..24{g.create_object_from_definition(&filler,p,Zone::Library);}}
    let mut q=TriggerQueue::new();let mut dm=Choices{accept:true,target:None,trace:vec![],resources:vec![],x:0,activation_hint:None};
    if def.name()=="Rock Hydra" {
        if cast(&mut g,&defs["Glorious Anthem"],PlayerId(0),&mut q,&mut dm)?!=3{return Err("Anthem payment".into());}resolve_all(&mut g,&mut q,&mut dm)?;
        dm.x=if mode==1{2}else{0};
    }
    let paid=cast(&mut g,def,PlayerId(0),&mut q,&mut dm)?;
    let expected_source=def.card.mana_cost.as_ref().unwrap().mana_value()+if def.name()=="Rock Hydra"{dm.x}else{0};
    if paid!=expected_source{return Err(format!("source paid{paid} expected{expected_source}"));}resolve_all(&mut g,&mut q,&mut dm)?;
    let source=find(&g,def.name())?;
    let mut evidence=json!({"source_paid":paid,"mode":mode});
    let expected_activation=|mana:i64|json!({"offered":true,"announcement_error":null,"resolution_error":null,"mana_paid":mana,"remaining_stack":0});
    let expected;let actual;
    match def.name(){
      "Rock Hydra"=>{
        let before=g.object(source).unwrap().counters.get(&ironsmith::CounterType::PlusOnePlusOne).copied().unwrap_or(0);
        if before!=dm.x{return Err("Hydra entry X counters mismatch".into());}
        advance_turn(&mut g,&mut q,&mut dm)?;
        g.next_turn();ironsmith::turn::execute_untap_step(&mut g);ironsmith::turn::advance_step(&mut g).map_err(|e|e.to_string())?;mana(&mut g);
        if g.turn.active_player!=PlayerId(0)||g.turn.step!=Some(ironsmith::Step::Upkeep){return Err("own actual upkeep not reached".into());}
        evidence["before_counters"]=json!(before);evidence["before_power"]=json!(g.calculated_power(source));evidence["before_abilities"]=json!(format!("{:?}",g.current_abilities(source)));
        dm.activation_hint=Some("PutCountersEffect");let a=activation(&mut g,&mut q,&mut dm,source)?;
        let after=g.object(source).unwrap().counters.get(&ironsmith::CounterType::PlusOnePlusOne).copied().unwrap_or(0);
        expected=json!({"activation":expected_activation(3),"counters":before+1,"power":before+2,"toughness":before+2});
        actual=json!({"activation":a,"counters":after,"power":g.calculated_power(source),"toughness":g.calculated_toughness(source)});
      },
      "Fighter Class"=>{
        let mut levels=vec![];
        for n in 0..mode {let a=activation(&mut g,&mut q,&mut dm,source)?;levels.push(a.clone());if a!=expected_activation(if n==0{3}else{5}){return Err(format!("Class level {n} transition {a}"));}}
        evidence["level_activations"]=json!(levels);evidence["actual_class_level"]=json!(g.class_level(source));
        for (n,cost) in [("Grizzly Bears",2),("Bonesplitter",1)]{if cast(&mut g,&defs[n],PlayerId(0),&mut q,&mut dm)?!=cost{return Err(format!("{n} cost"));}resolve_all(&mut g,&mut q,&mut dm)?;}
        let host=find(&g,"Grizzly Bears")?;let equipment=find(&g,"Bonesplitter")?;dm.target=Some(Target::Object(host));let a=activation(&mut g,&mut q,&mut dm,equipment)?;
        expected=json!({"activation":expected_activation(if mode==0{1}else{0}),"class_level":mode+1,"equipment_attached":true,"power":4,"toughness":2});
        actual=json!({"activation":a,"class_level":g.class_level(source),"equipment_attached":g.object(equipment).unwrap().attached_to==Some(AttachmentTarget::Object(host)),"power":g.calculated_power(host),"toughness":g.calculated_toughness(host)});
      },
      "Gavel of the Righteous"=>{
        if cast(&mut g,&defs["Grizzly Bears"],PlayerId(0),&mut q,&mut dm)?!=2{return Err("Bear cost".into());}resolve_all(&mut g,&mut q,&mut dm)?;
        let host=find(&g,"Grizzly Bears")?;
        if mode==1 {for n in 0..4 {
          ironsmith::turn::advance_phase(&mut g).map_err(|e|e.to_string())?;
          let e=ironsmith::triggers::generate_step_trigger_events(&g).ok_or("missing actual begin combat event")?;
          let t=ironsmith::triggers::check_triggers(&g,&e);let count=t.len();for a in t{q.add(a);}resolve_all(&mut g,&mut q,&mut dm)?;
          let counters=g.object(source).unwrap().counters.get(&ironsmith::CounterType::Charge).copied().unwrap_or(0);
          if count!=1||counters!=n+1{return Err(format!("combat trigger count {count}, charges {counters}"));}
          evidence[format!("combat_{n}")]=json!({"turn":g.turn.turn_number,"triggers":count,"charges":counters});
          advance_turn(&mut g,&mut q,&mut dm)?;advance_turn(&mut g,&mut q,&mut dm)?;
        }}
        dm.target=Some(Target::Object(host));let a=activation(&mut g,&mut q,&mut dm,source)?;
        let count=if mode==1{4}else{0};expected=json!({"activation":expected_activation(3),"charges":count,"attached":true,"power":2+count,"toughness":2+count,"double_strike":mode==1});
        actual=json!({"activation":a,"charges":g.object(source).unwrap().counters.get(&ironsmith::CounterType::Charge).copied().unwrap_or(0),"attached":g.object(source).unwrap().attached_to==Some(AttachmentTarget::Object(host)),"power":g.calculated_power(host),"toughness":g.calculated_toughness(host),"double_strike":g.object_has_static_ability_id(host,K::DoubleStrike)});
      },
      _=>{
        let host_name=if mode==1{"Wall of Wood"}else{"Grizzly Bears"};
        for (n,cost) in [(host_name,if mode==1{1}else{2}),("Fervor",3)]{if cast(&mut g,&defs[n],PlayerId(0),&mut q,&mut dm)?!=cost{return Err(format!("{n} cost"));}resolve_all(&mut g,&mut q,&mut dm)?;}
        let host=find(&g,host_name)?;dm.target=Some(Target::Object(host));let a=activation(&mut g,&mut q,&mut dm,source)?;
        ironsmith::turn::advance_phase(&mut g).map_err(|e|e.to_string())?;ironsmith::turn::advance_step(&mut g).map_err(|e|e.to_string())?;
        if !g.object_has_static_ability_id(host,K::Haste)||g.is_tapped(host){return Err("actual paid Fervor did not leave ready haste attacker".into());}
        evidence["host_haste"]=json!(g.object_has_static_ability_id(host,K::Haste));evidence["host_tapped"]=json!(g.is_tapped(host));evidence["host_defender"]=json!(g.object_has_static_ability_id(host,K::Defender));evidence["host_defender_permission"]=json!(g.object_has_static_ability_id(host,K::CanAttackAsThoughNoDefender));evidence["equipment_defender_permission"]=json!(g.object_has_static_ability_id(source,K::CanAttackAsThoughNoDefender));
        let legal=ironsmith::decision::compute_legal_attackers(&g,&ironsmith::combat_state::CombatState::default());let offered=legal.iter().any(|a|a.creature==host);
        let mut declared=false;let mut attack_error=None;
        if offered {let mut combat=ironsmith::combat_state::CombatState::default();attack_error=ironsmith::game_loop::apply_attacker_declarations_with_dm(&mut g,&mut combat,&mut q,&[ironsmith::decision::AttackerDeclaration{creature:host,target:ironsmith::combat_state::AttackTarget::Player(PlayerId(1))}],&mut dm).err().map(|e|e.to_string());declared=attack_error.is_none();g.combat=Some(combat);}
        evidence["actual_combat"]=json!(format!("{:?}",g.combat));
        expected=json!({"activation":expected_activation(3),"attached":true,"power":if mode==1{2}else{4},"toughness":if mode==1{5}else{4},"attacker_offered":true,"attack_declared":true,"attack_error":null});
        actual=json!({"activation":a,"attached":g.object(source).unwrap().attached_to==Some(AttachmentTarget::Object(host)),"power":g.calculated_power(host),"toughness":g.calculated_toughness(host),"attacker_offered":offered,"attack_declared":declared,"attack_error":attack_error});
      }
    }
    Ok(json!({"expected":expected,"actual":actual,"state_evidence":evidence,"execution_trace":dm.trace}))
}
#[test]
#[ignore = "manual scoped conditional index state transition report"]
fn report_counter_level_activations() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let inventory =
        root.join("reports/runtime-audit/corpus/267a16aff3b321196397d0b4/inventory.json");
    let source = root.join(
        "crates/ironsmith-tools/tests/runtime_ability_index_counter_level_reproductions.rs",
    );
    let binary = std::env::current_exe().unwrap();
    let paths = [&inventory, &source, &binary];
    let before: Vec<_> = paths
        .iter()
        .map(|p| json!({"path":p,"sha256":hash(p)}))
        .collect();
    let inv: Value = serde_json::from_slice(&std::fs::read(&inventory).unwrap()).unwrap();
    let names = ["Rock Hydra", "Fighter Class", "Gavel of the Righteous", "Warmonger's Chariot"];
    let mut compile = vec![];
    let mut defs = std::collections::HashMap::new();
    for name in names.into_iter().chain(["Grizzly Bears", "Wall of Wood", "Glorious Anthem", "Fervor", "Bonesplitter"]) {
        let p = inv["cards"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["name"] == name)
            .unwrap();
        let (artifact, def) = ironsmith_registry::compile_builder_to_artifact(
            ironsmith_compiler::CardDefinitionBuilder::new(
                CardId::new(),
                p["parse_name"].as_str().unwrap_or(name),
            ),
            p["parse_input"].as_str().unwrap(),
            false,
        )
        .unwrap();
        compile.push(json!({"card":name,"artifact_checksum":artifact.payload_checksum,"definition":artifact.payload.definition}));
        defs.insert(name, def);
    }
    let mut rows = vec![];
    for name in names {
        if std::env::var("AUDIT_STATIC_CARD").is_ok_and(|n| n != name) {
            continue;
        }
        let count = match name {
            "Fighter Class" => 3,
            _ => 2,
        };
        for mode in 0..count {
            if std::env::var("AUDIT_STATIC_MODE").is_ok_and(|n| n != mode.to_string()) {
                continue;
            }
            eprintln!("AUDIT_CASE {name} {mode}");
            let (status, out) = match run(&defs[name], &defs, mode) {
                Ok(out) => {
                    let status = if out["actual"]["activation"]["announcement_error"].is_string() {
                        "action_or_choice_failed"
                    } else if out["actual"]["activation"]["resolution_error"].is_string() {
                        "resolution_failed"
                    } else if out["expected"] == out["actual"] {
                        "expected_result_observed"
                    } else {
                        "semantic_mismatch"
                    };
                    (status, out)
                }
                Err(e) => (
                    "fixture_or_execution_error",
                    json!({"expected":null,"actual":{"error":e}}),
                ),
            };
            rows.push(json!({"card":name,"scenario":{"mode":mode},"status":status,"expected":out["expected"],"actual":out["actual"],"state_evidence":out["state_evidence"],"execution_trace":out["execution_trace"],"artifact_checksum":compile.iter().find(|c|c["card"]==name).unwrap()["artifact_checksum"],"scope":"Paid canonical source and real printed producer actions, actual turn/untap transitions as required, resource-complete advertised activation. Exact condition characteristics, costs and outcome checked; linked-face and unrelated abilities remain outside scope."}));
        }
    }
    let after: Vec<_> = paths
        .iter()
        .map(|p| json!({"path":p,"sha256":hash(p)}))
        .collect();
    let report = json!({"rows":rows,"compilation":compile,"provenance":{"before":before,"after":after,"artifacts_unchanged":before==after,"strict_artifact":true,"unique_card_ids":true,"seed":71757432704855_u64}});
    let out = std::env::var("AUDIT_STATIC_OUTPUT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            root.join("reports/runtime-audit/ability-index-counter-level-reproductions.json")
        });
    std::fs::write(out, serde_json::to_string_pretty(&report).unwrap()).unwrap();
    println!("wrote{}cases", report["rows"].as_array().unwrap().len());
}
