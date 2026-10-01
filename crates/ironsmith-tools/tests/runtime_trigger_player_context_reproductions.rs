//! Paid producer actions for trigger player-context failures found in MAGE.
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{DecisionMaker, GameProgress, LegalAction, compute_legal_actions};
use ironsmith::decisions::context::{DecisionContext, SelectObjectsContext, TargetsContext};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, advance_priority_with_dm,
    apply_decision_context_with_dm, apply_priority_response_with_dm};
use ironsmith::game_state::Target;
use ironsmith::mana::ManaSymbol;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardDefinition, CardId, CardType, GameState, ObjectId, PlayerId, Zone};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

struct Choices { object: Option<ObjectId>, player: PlayerId, trace: Vec<Value> }
impl DecisionMaker for Choices {
    fn decide_targets(&mut self, _: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        let preferred = [self.object.map(Target::Object), Some(Target::Player(self.player))];
        let mut chosen = Vec::new();
        for requirement in &ctx.requirements {
            let target = preferred.iter().flatten().find(|t|requirement.legal_targets.contains(t))
                .expect("fixture target must be explicitly legal");
            chosen.push(*target);
        }
        self.trace.push(json!({"choice":"targets","context":format!("{ctx:?}"),"selected":format!("{chosen:?}")}));
        chosen
    }
    fn decide_objects(&mut self, game: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        let mut candidates:Vec<_> = ctx.candidates.iter().filter(|o|o.legal).collect();
        candidates.sort_by_key(|c|!game.object(c.id).is_some_and(|o|o.card_types.contains(&CardType::Land)));
        let selected:Vec<_> = candidates.iter().take(ctx.min).map(|o|o.id).collect();
        self.trace.push(json!({"choice":"objects","context":format!("{ctx:?}"),"selected":format!("{selected:?}")}));
        selected
    }
}
fn setup(players: usize, lands: usize) -> GameState {
    let mut g=GameState::new(["Alice","Bob","Cara"][..players].iter().map(|s|s.to_string()).collect(),20);
    g.turn.turn_number=3;
    g.turn.active_player=PlayerId(0);
    g.turn.priority_player=Some(PlayerId(0));
    g.turn.phase=ironsmith::Phase::FirstMain;
    g.turn.step=None;
    for color in [ManaSymbol::White,ManaSymbol::Blue,ManaSymbol::Black,ManaSymbol::Red,ManaSymbol::Green,ManaSymbol::Colorless] {
        g.player_mut(PlayerId(0)).unwrap().mana_pool.add(color,12);
    }
    for _ in 0..lands {
        let def=CardDefinitionBuilder::new(CardId::new(),"Trigger sacrifice land").card_types(vec![CardType::Land]).build();
        g.create_object_from_definition(&def,PlayerId(0),Zone::Battlefield);
    }
    g
}
fn cast(g: &mut GameState, def: &CardDefinition, dm: &mut Choices) -> Result<Value,String> {
    g.turn.priority_player=Some(PlayerId(0));
    let source=g.create_object_from_definition(def,PlayerId(0),Zone::Hand);
    let action=compute_legal_actions(g,PlayerId(0)).expect("fixture has complete replacement state").into_iter().find(|a|matches!(a,LegalAction::CastSpell{spell_id,..}if *spell_id==source)).ok_or("intended cast unavailable")?;
    let mana=g.player(PlayerId(0)).unwrap().mana_pool.total();
    let mut q=TriggerQueue::new();
    let mut state=PriorityLoopState::new(g.players_in_game());
    let mut progress=apply_priority_response_with_dm(g,&mut q,&mut state,&PriorityResponse::PriorityAction(action),dm).map_err(|e|e.to_string())?;
    for _ in 0..24 {
        if state.pending_cast.is_none() && !g.stack.is_empty() { break; }
        let GameProgress::NeedsDecisionCtx(ctx)=progress else { return Err(format!("announcement stopped:{progress:?}")); };
        if matches!(ctx,DecisionContext::Priority(_)) { return Err("announcement returned priority without spell".into()); }
        progress=apply_decision_context_with_dm(g,&mut q,&mut state,&ctx,dm).map_err(|e|e.to_string())?;
    }
    if g.stack.is_empty() { return Err("announcement budget".into()); }
    let mut evidence=json!({"spell":def.name(),"mana_paid":mana-g.player(PlayerId(0)).unwrap().mana_pool.total(),"announced_targets":format!("{:?}",g.stack.last().unwrap().targets),"resolution_error":null});
    for _ in 0..24 {
        if let Err(error)=advance_priority_with_dm(g,&mut q,dm) {
            evidence["resolution_error"]=json!(error.to_string());return Ok(evidence);
        }
        if g.stack.is_empty() { return Ok(evidence); }
        state.reset_for_new_priority_window(g);
        for _ in 0..g.players_in_game() {
            if let Err(error)=apply_priority_response_with_dm(g,&mut q,&mut state,&PriorityResponse::PriorityAction(LegalAction::PassPriority),dm) {
                evidence["resolution_error"]=json!(error.to_string());return Ok(evidence);
            }
        }
    }
    Err("resolution decision budget".into())
}
fn find(g: &GameState, name: &str) -> Result<ObjectId,String> {
    g.battlefield.iter().copied().find(|id|g.object(*id).is_some_and(|o|o.name==name)).ok_or(format!("{name} absent"))
}
fn lands(g: &GameState) -> usize {
    g.battlefield.iter().filter(|id|g.object(**id).is_some_and(|o|o.name=="Trigger sacrifice land")).count()
}
fn run(defs: &HashMap<String,CardDefinition>, name: &str, producer: Option<&str>, player: u8, count: usize) -> Result<(Value,Value),String> {
    let mut g=setup(3,count);
    let mut dm=Choices{object:None,player:PlayerId(player),trace:Vec::new()};
    let source_cast=cast(&mut g,&defs[name],&mut dm)?;
    if !source_cast["resolution_error"].is_null() { return Err(format!("source cast failed:{source_cast}")); }
    let source=find(&g,name)?;
    let before_life:Vec<_>=g.players.iter().map(|p|p.life).collect();
    let mut producer_cast=Value::Null;
    if let Some(spell)=producer {
        dm.object=if name=="Phyrexian Negator" && player==1 {None} else {Some(source)};
        producer_cast=cast(&mut g,&defs[spell],&mut dm)?;
    }
    let actual=json!({"resolution_error":producer_cast.get("resolution_error"),"life":g.players.iter().map(|p|p.life).collect::<Vec<_>>(),"lands":lands(&g),"source_battlefield_count":g.battlefield.iter().filter(|id|g.object(**id).is_some_and(|o|o.name==name)).count()});
    Ok((actual,json!({"source_cast":source_cast,"life_after_source_etb":before_life,"producer_cast":producer_cast,"choices":dm.trace,"initial_lands":count})))
}
fn hash(p: &std::path::Path) -> String {Sha256::digest(std::fs::read(p).unwrap()).iter().map(|b|format!("{b:02x}")).collect()}
#[test]
#[ignore = "report generator; inspect actual versus expected outcomes"]
fn report_trigger_player_context() {
    let input=std::path::PathBuf::from(std::env::var("AUDIT_RUNTIME_INVENTORY").unwrap());
    let payloads:Value=serde_json::from_slice(&std::fs::read(&input).unwrap()).unwrap();
    let names=["Phyrexian Negator","Laquatus's Champion","Soul Scourge","Shock","Lightning Bolt","Unsummon","Murder","Cloudshift"];
    let mut defs=HashMap::new();let mut artifacts=Vec::new();
    for p in payloads["cards"].as_array().unwrap() {
        let name=p["name"].as_str().unwrap();if !names.contains(&name) {continue;}
        let (artifact,def)=ironsmith_registry::compile_builder_to_artifact(ironsmith_compiler::CardDefinitionBuilder::new(CardId::new(),name),p["parse_input"].as_str().unwrap(),false).unwrap();
        artifacts.push(json!({"card":name,"artifact_checksum":artifact.payload_checksum}));defs.insert(name.to_string(),def);
    }
    let mut cases=Vec::new();
    for (spell,damage) in [("Shock",2),("Lightning Bolt",3)] {
        cases.push(("Phyrexian Negator",Some(spell),0,5,json!({"resolution_error":null,"life":[20,20,20],"lands":5-damage,"source_battlefield_count":1})));
        cases.push(("Phyrexian Negator",Some(spell),1,5,json!({"resolution_error":null,"life":[20,20-damage,20],"lands":5,"source_battlefield_count":1})));
    }
    cases.push(("Phyrexian Negator",Some("Shock"),0,0,json!({"resolution_error":null,"life":[20,20,20],"lands":0,"source_battlefield_count":0})));
    cases.push(("Phyrexian Negator",None,0,5,json!({"resolution_error":null,"life":[20,20,20],"lands":5,"source_battlefield_count":1})));
    for (name,amount) in [("Laquatus's Champion",6),("Soul Scourge",3)] {
        for player in [0,1,2] {
            let mut life=vec![20,20,20];life[player as usize]-=amount;
            cases.push((name,None,player,0,json!({"resolution_error":null,"life":life,"lands":0,"source_battlefield_count":1})));
            for spell in ["Unsummon","Murder","Cloudshift"] {
                let expected_life=if spell=="Cloudshift" {life.clone()} else {vec![20,20,20]};
                cases.push((name,Some(spell),player,0,json!({"resolution_error":null,"life":expected_life,"lands":0,"source_battlefield_count":usize::from(spell=="Cloudshift")})));
            }
        }
    }
    let mut rows=Vec::new();
    for (name,producer,player,count,expected) in cases {
        let (status,actual,evidence)=match run(&defs,name,producer,player,count) {
            Ok((actual,evidence)) => (if actual==expected {"expected_result_observed"} else if !actual["resolution_error"].is_null() {"resolution_failed"} else {"semantic_mismatch"},actual,evidence),
            Err(error) => ("fixture_or_source_cast_error",json!({"error":error}),Value::Null),
        };
        rows.push(json!({"card":name,"scenario":{"producer":producer,"target_player":player,"initial_sacrifice_lands":count},"status":status,"expected":expected,"actual":actual,"fixture_evidence":evidence}));
    }
    let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");let binary=std::env::current_exe().unwrap();
    let report=json!({"scope":"Strict canonical sources and paid Shock/Lightning Bolt/Unsummon/Murder/Cloudshift producers. Normal priority-pass resolution, three players, unique CardIds. Only named expectations verified.","rows":rows,"artifacts":artifacts,"provenance":{"inventory":input,"inventory_sha256":hash(&input),"binary":binary,"binary_sha256":hash(&binary),"source_sha256":hash(&root.join("crates/ironsmith-tools/tests/runtime_trigger_player_context_reproductions.rs"))}});
    std::fs::write(root.join("reports/runtime-audit/trigger-player-context-reproductions.json"),serde_json::to_string_pretty(&report).unwrap()).unwrap();
    println!("{}",serde_json::to_string_pretty(&report).unwrap());
}
