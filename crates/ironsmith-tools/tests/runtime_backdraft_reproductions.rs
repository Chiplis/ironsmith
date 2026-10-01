//! Backdraft actual spell history and chosen-player context probes.
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{DecisionMaker, GameProgress, LegalAction, compute_legal_actions};
use ironsmith::decisions::context::{DecisionContext, SelectObjectsContext, SelectOptionsContext, TargetsContext};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, advance_priority_with_dm,
    apply_decision_context_with_dm, apply_priority_response_with_dm};
use ironsmith::game_state::Target;
use ironsmith::mana::ManaSymbol;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardDefinition, CardId, CardType, GameState, ObjectId, PlayerId, Zone};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

struct Choices { object: Option<ObjectId>, player: PlayerId, chosen: PlayerId, trace: Vec<Value> }
impl DecisionMaker for Choices {
    fn answers_player_choices(&self) -> bool { true }
    fn decide_options(&mut self, g: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        let name=["Alice","Bob","Cara"][self.chosen.0 as usize];
        let selected=ctx.options.iter().find(|o|o.legal&&o.description==name).map(|o|vec![o.index]).unwrap_or_else(||ironsmith::decision::SelectFirstDecisionMaker.decide_options(g,ctx));
        self.trace.push(json!({"choice":"options","context":format!("{ctx:?}"),"selected":selected}));
        selected
    }
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
fn cast(g: &mut GameState, def: &CardDefinition, actor: u8, dm: &mut Choices) -> Result<Value,String> {
    g.turn.priority_player=Some(PlayerId(actor));
    let source=g.create_object_from_definition(def,PlayerId(actor),Zone::Hand);
    let action=compute_legal_actions(g,PlayerId(actor)).expect("fixture has complete replacement state").into_iter().find(|a|matches!(a,LegalAction::CastSpell{spell_id,..}if *spell_id==source)).ok_or("intended cast unavailable")?;
    let mana=g.player(PlayerId(actor)).unwrap().mana_pool.total();
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
    let mut evidence=json!({"spell":def.name(),"mana_paid":mana-g.player(PlayerId(actor)).unwrap().mana_pool.total(),"announced_targets":format!("{:?}",g.stack.last().unwrap().targets),"resolution_error":null});
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
fn run(defs:&HashMap<String,CardDefinition>,producer:&str,caster:u8)->Result<(Value,Value),String>{
    let mut g=setup(3,0);
    while g.turn.active_player!=PlayerId(caster){g.next_turn();}
    g.turn.phase=ironsmith::Phase::FirstMain;g.turn.step=None;
    for p in [PlayerId(0),PlayerId(1),PlayerId(2)]{
        for color in [ManaSymbol::White,ManaSymbol::Blue,ManaSymbol::Black,ManaSymbol::Red,ManaSymbol::Green,ManaSymbol::Colorless]{g.player_mut(p).unwrap().mana_pool.add(color,10);}
        for _ in 0..8{let d=CardDefinitionBuilder::new(CardId::new(),"Backdraft library witness").card_types(vec![CardType::Instant]).build();g.create_object_from_definition(&d,p,Zone::Library);}
    }
    let victim=if caster==0{1}else{0};
    let d=CardDefinitionBuilder::new(CardId::new(),"Backdraft damage witness").card_types(vec![CardType::Creature]).power_toughness(ironsmith::PowerToughness::fixed(2,6)).build();
    let creature=g.create_object_from_definition(&d,PlayerId(victim),Zone::Battlefield);
    let mut dm=Choices{object:if producer=="Flame Slash"{Some(creature)}else{None},player:PlayerId(victim),chosen:PlayerId(caster),trace:vec![]};
    let producer_cast=if producer=="None" { Value::Null } else { cast(&mut g,&defs[producer],caster,&mut dm)? };
    if !producer_cast.is_null() && !producer_cast["resolution_error"].is_null(){return Err(format!("producer failed:{producer_cast}"));}
    if producer!="None" { let expected_cost=match producer{"Lava Axe"=>5,"Divination"=>3,_=>1}; if producer_cast["mana_paid"]!=expected_cost{return Err(format!("wrong producer payment:{producer_cast}"));} }
    let life_before:Vec<_>=g.players.iter().map(|p|p.life).collect();
    dm.object=None;
    let response=cast(&mut g,&defs["Backdraft"],0,&mut dm)?;
    if response["mana_paid"]!=2{return Err(format!("wrong Backdraft payment:{response}"));}
    let actual=json!({"resolution_error":response["resolution_error"],"life":g.players.iter().map(|p|p.life).collect::<Vec<_>>()});
    Ok((actual,json!({"producer_cast":producer_cast,"backdraft_cast":response,"life_before_backdraft":life_before,"choices":dm.trace,"active_player":caster,"qualifying_sorcery_caster":if ["None","Shock"].contains(&producer){Value::Null}else{json!(caster)}})))
}
fn hash(p:&std::path::Path)->String{Sha256::digest(std::fs::read(p).unwrap()).iter().map(|b|format!("{b:02x}")).collect()}
#[test]
#[ignore="actual-history reporter, inspect expected versus actual states"]
fn report_backdraft(){
    let input=std::path::PathBuf::from(std::env::var("AUDIT_RUNTIME_INVENTORY").unwrap());let payloads:Value=serde_json::from_slice(&std::fs::read(&input).unwrap()).unwrap();let mut defs=HashMap::new();let mut artifacts=vec![];
    for p in payloads["cards"].as_array().unwrap(){let n=p["name"].as_str().unwrap();if !["Backdraft","Firebolt","Lava Spike","Flame Slash","Lava Axe","Divination","Shock"].contains(&n){continue;}let(a,d)=ironsmith_registry::compile_builder_to_artifact(ironsmith_compiler::CardDefinitionBuilder::new(CardId::new(),n),p["parse_input"].as_str().unwrap(),false).unwrap();artifacts.push(json!({"card":n,"checksum":a.payload_checksum,"definition":a.payload.definition}));defs.insert(n.to_string(),d);}
    let mut rows=vec![];
    for (producer,damage) in [("Firebolt",2),("Lava Spike",3),("Flame Slash",4),("Lava Axe",5),("Divination",0),("None",0),("Shock",2)]{for caster in [0,1]{
        let mut life=vec![20,20,20];if producer!="Shock"{life[caster as usize]-=damage/2;}if producer!="Flame Slash"{life[if caster==0{1}else{0}]-=damage;}
        let expected=json!({"resolution_error":null,"life":life});
        let(status,actual,evidence)=match run(&defs,producer,caster){Ok((a,e))=>(if a==expected{"expected_result_observed"}else if !a["resolution_error"].is_null(){"resolution_failed"}else{"semantic_mismatch"},a,e),Err(e)=>("fixture_or_producer_error",json!({"error":e}),Value::Null)};
        rows.push(json!({"card":"Backdraft","scenario":{"producer":producer,"active_player":caster,"qualifying_sorcery_caster":if ["None","Shock"].contains(&producer){Value::Null}else{json!(caster)},"damage_from_producer":damage},"status":status,"expected":expected,"actual":actual,"fixture_evidence":evidence}));
    }}
    let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");let binary=std::env::current_exe().unwrap();let r=json!({"scope":"Strict canonical paid source sorceries and paid Backdraft cast; actual damage or draw history, explicit player-choice callbacks, three players, normal priority resolution. Single qualifying sorcery removes ambiguity over spell selection; no-spell and actual instant-only branches test legal resolution when no player qualifies.","rows":rows,"artifacts":artifacts,"provenance":{"binary":binary,"binary_sha256":hash(&binary),"inventory_sha256":hash(&input),"source_sha256":hash(&root.join("crates/ironsmith-tools/tests/runtime_backdraft_reproductions.rs")),"runtime_stack_bytes":67108864}});
    std::fs::write(root.join("reports/runtime-audit/backdraft-reproductions.json"),serde_json::to_string_pretty(&r).unwrap()).unwrap();println!("{}",serde_json::to_string_pretty(&r).unwrap());
}
