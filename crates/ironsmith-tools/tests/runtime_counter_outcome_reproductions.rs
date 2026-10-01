//! Strict paid-action audit of counter-dependent trigger outcomes; JSON contains verdicts.
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{
    DecisionMaker, GameProgress, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::{
    BooleanContext, DecisionContext, NumberContext, SelectObjectsContext, SelectOptionsContext,
    TargetsContext,
};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, drain_pending_trigger_events, put_triggers_on_stack_with_dm,
};
use ironsmith::game_state::Target;
use ironsmith::mana::{ManaCost, ManaSymbol};
use ironsmith::object::CounterType;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{
    CardDefinition, CardId, CardType, Effect, GameState, ObjectId, PlayerId, PowerToughness, Zone,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::Read;
fn hash(path: &std::path::Path) -> Value {
    let mut file = std::fs::File::open(path).unwrap();
    let mut digest = Sha256::new();
    let mut buffer = [0; 65536];
    loop {
        let count = file.read(&mut buffer).unwrap();
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    let sha256: String = digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    json!({"path": path.display().to_string(), "sha256": sha256})
}

fn alice() -> PlayerId {
    PlayerId::from_index(0)
}
fn setup_players(players: usize) -> GameState {
    let mut g = GameState::new(
        ["Alice", "Bob", "Cara"][..players]
            .iter()
            .map(|s| s.to_string())
            .collect(),
        20,
    );
    g.turn.active_player = alice();
    g.turn.priority_player = Some(alice());
    g.turn.phase = ironsmith::Phase::FirstMain;
    g.turn.step = None;
    for color in [
        ManaSymbol::White,
        ManaSymbol::Blue,
        ManaSymbol::Black,
        ManaSymbol::Red,
        ManaSymbol::Green,
        ManaSymbol::Colorless,
    ] {
        for p in (0..players).map(|p| PlayerId::from_index(p as u8)) {
            g.player_mut(p).unwrap().mana_pool.add(color, 30);
        }
    }
    g
}

struct ProbeDm {
    x: u32,
    accept: bool,
    target_name: Option<String>,
    recipient: usize,
    trace: Vec<Value>,
}
impl DecisionMaker for ProbeDm {
    fn answers_player_choices(&self) -> bool {
        true
    }
    fn decide_boolean(&mut self, _: &GameState, c: &BooleanContext) -> bool {
        self.trace
            .push(json!({"boolean":c.description,"player":c.player.index(),"answer":self.accept}));
        self.accept
    }
    fn decide_number(&mut self, _: &GameState, c: &NumberContext) -> u32 {
        if c.is_x_value { self.x } else { c.min }
    }
    fn decide_objects(&mut self, g: &GameState, c: &SelectObjectsContext) -> Vec<ObjectId> {
        let selected = SelectFirstDecisionMaker.decide_objects(g, c);
        self.trace.push(json!({"objects":c.description,"player":c.player.index(),"offered":c.candidates.iter().map(|v|json!({"id":v.id.0,"legal":v.legal,"controller":g.controller_of_id(v.id).map(|p|p.index())})).collect::<Vec<_>>(),"selected":selected.iter().map(|v|v.0).collect::<Vec<_>>()}));
        selected
    }
    fn decide_options(&mut self, g: &GameState, c: &SelectOptionsContext) -> Vec<usize> {
        let recipient = ["Alice", "Bob", "Cara"][self.recipient];
        let selected = c
            .options
            .iter()
            .find(|o| o.legal && o.description == recipient)
            .map(|o| vec![o.index])
            .unwrap_or_else(|| SelectFirstDecisionMaker.decide_options(g, c));
        self.trace.push(json!({"options":c.description,"player":c.player.index(),"offered":c.options.iter().map(|o|o.description.clone()).collect::<Vec<_>>(),"selected":selected}));
        selected
    }
    fn decide_targets(&mut self, g: &GameState, c: &TargetsContext) -> Vec<Target> {
        if self.target_name.as_deref()==Some("PLAYER_ALICE") { return vec![Target::Player(alice())]; }
        if let Some(name) = &self.target_name {
            if c.requirements.len() == 1 {
                if let Some(t)=c.requirements[0].legal_targets.iter().find(|t|matches!(t,Target::Object(id)if g.object(*id).is_some_and(|o|o.name==*name))){return vec![*t];}
            }
        }
        SelectFirstDecisionMaker.decide_targets(g, c)
    }
}
fn resolve(game: &mut GameState, queue: &mut TriggerQueue, dm: &mut ProbeDm) -> Result<(), String> {
    for _ in 0..32 {
        ironsmith::game_loop::check_and_apply_sbas_with(game,queue,dm).map_err(|e|e.to_string())?;
        drain_pending_trigger_events(game, queue);
        put_triggers_on_stack_with_dm(game, queue, dm).map_err(|e| e.to_string())?;
        if game.stack.is_empty() {
            return Ok(());
        }
        let mut priority = PriorityLoopState::new(game.players_in_game());
        priority.reset_for_new_priority_window(game);
        for _ in 0..game.players_in_game() {
            apply_priority_response_with_dm(
                game,
                queue,
                &mut priority,
                &PriorityResponse::PriorityAction(LegalAction::PassPriority),
                dm,
            )
            .map_err(|e| e.to_string())?;
        }
    }
    Err("fixture priority resolution bound exceeded".into())
}
fn perform(
    game: &mut GameState,
    source: ObjectId,
    ability: Option<usize>,
    dm: &mut ProbeDm,
) -> Result<(TriggerQueue, u32), String> {
    let actor = game
        .controller_of_id(source)
        .ok_or("missing action source")?;
    let initial_stack_len = game.stack.len();
    game.turn.priority_player = Some(actor);
    let action = compute_legal_actions(game, actor).expect("fixture has complete replacement state")
        .into_iter()
        .find(|a| match a {
            LegalAction::CastSpell { spell_id, .. } => ability.is_none() && *spell_id == source,
            LegalAction::ActivateAbility {
                source: id,
                ability_index,
            } => *id == source && Some(*ability_index) == ability,
            _ => false,
        })
        .ok_or("fixture intended legal action missing")?;
    let mana = game.player(actor).unwrap().mana_pool.total();
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
        if game.stack.len() > initial_stack_len {
            return Ok((queue, mana - game.player(actor).unwrap().mana_pool.total()));
        }
        progress = match progress {
            GameProgress::NeedsDecisionCtx(DecisionContext::SelectOptions(ref c))
                if c.description.starts_with("Choose optional costs") =>
            {
                apply_priority_response_with_dm(
                    game,
                    &mut queue,
                    &mut state,
                    &PriorityResponse::OptionalCosts(vec![]),
                    dm,
                )
                .map_err(|e| e.to_string())?
            }
            GameProgress::NeedsDecisionCtx(ref c) if !matches!(c, DecisionContext::Priority(_)) => {
                apply_decision_context_with_dm(game, &mut queue, &mut state, c, dm)
                    .map_err(|e| e.to_string())?
            }
            other => return Err(format!("fixture action failed to reach stack: {other:?}")),
        };
    }
    Err("fixture action decision bound exceeded".into())
}

fn dm(x: u32, accept: bool) -> ProbeDm {
    ProbeDm {
        x,
        accept,
        target_name: None,
        recipient: 1,
        trace: vec![],
    }
}
fn count(g: &GameState, name: &str, zone: Zone) -> usize {
    g.objects_in_deterministic_order()
        .into_iter()
        .filter(|o| o.zone == zone && o.name.contains(name))
        .count()
}
fn find(g: &GameState, name: &str, zone: Zone) -> Result<ObjectId, String> {
    g.objects_in_deterministic_order()
        .into_iter()
        .find(|o| o.zone == zone && o.name == name)
        .map(|o| o.id)
        .ok_or(format!("fixture {name} missing in {zone:?}"))
}
fn cast(
    g: &mut GameState,
    def: &CardDefinition,
    player: PlayerId,
    dm: &mut ProbeDm,
) -> Result<(TriggerQueue, u32), String> {
    let id = g.create_object_from_definition(def, player, Zone::Hand);
    perform(g, id, None, dm)
}
fn established(
    g: &mut GameState,
    def: &CardDefinition,
    player: PlayerId,
    expected: u32,
) -> Result<ObjectId, String> {
    let mut dm = dm(0, false);
    let (mut q, paid) = cast(g, def, player, &mut dm)?;
    if paid != expected {
        return Err(format!(
            "setup {} paid{paid} expected{expected}",
            def.name()
        ));
    }
    resolve(g, &mut q, &mut dm)?;
    find(g, def.name(), Zone::Battlefield)
}
fn witness(name: &str, kind: CardType, cost: u8, with_x: bool) -> CardDefinition {
    let mut pips = vec![ManaSymbol::Generic(cost)];
    if with_x {
        pips.push(ManaSymbol::X);
    }
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![kind])
        .mana_cost(ManaCost::from_symbols(pips))
        .power_toughness(PowerToughness::fixed(3, 3))
        .with_spell_effect(vec![Effect::gain_life(1)])
        .build()
}

fn phase(g: &mut GameState, combat: bool, dm: &mut ProbeDm) -> Result<Option<String>, String> {
    g.turn.phase = if combat { ironsmith::Phase::Combat } else { ironsmith::Phase::Beginning };
    g.turn.step = Some(if combat { ironsmith::Step::BeginCombat } else { ironsmith::Step::Upkeep });
    let event=ironsmith::triggers::generate_step_trigger_events(g).ok_or("fixture phase event missing")?;
    let mut q=TriggerQueue::new();
    for trigger in ironsmith::triggers::check_triggers(g,&event) { q.add(trigger); }
    put_triggers_on_stack_with_dm(g,&mut q,dm).map_err(|e|e.to_string())?;
    Ok(resolve(g,&mut q,dm).err())
}
fn counter(g:&GameState,id:ObjectId,typ:CounterType)->u32 { g.object(id).filter(|o|o.zone==Zone::Battlefield).map(|o|o.counters.iter().filter(|(k,_)|k.description()==typ.description()).map(|(_,n)|*n).sum()).unwrap_or(0) }
fn counter_type(name:&str)->CounterType { match name {"Cocoon"=>CounterType::Pupa,"Living Artifact"=>CounterType::Vitality,"Parallax Dementia"|"Rusting Golem"=>CounterType::Fade,_=>CounterType::PlusOnePlusOne} }
fn scenario(defs:&HashMap<String,CardDefinition>,name:&str,rounds:usize,accept:bool,x:u32,anthemed:bool,damage:usize)->Result<(Value,Value),String>{
    let mut g=setup_players(2);
    if anthemed { established(&mut g,&defs["Glorious Anthem"],alice(),3)?; }
    let target=CardDefinitionBuilder::new(CardId::new(),"Counter outcome witness").card_types(vec![CardType::Artifact,CardType::Creature]).power_toughness(PowerToughness::fixed(6,6)).build();
    let target_id=g.create_object_from_definition(&target,alice(),Zone::Battlefield);
    let mut dm=dm(x,accept);dm.target_name=Some(target.name().to_owned());
    let (mut q,paid)=cast(&mut g,&defs[name],alice(),&mut dm)?;
    let cost=match name {"Guiding Hydra"=>1+x,"Cocoon"|"Living Artifact"=>1,"Parallax Dementia"=>2,"Rusting Golem"=>4,"Magmasaur"=>5,_=>6};
    if paid!=cost {return Err(format!("fixture paid{paid}, expected{cost}"));}
    resolve(&mut g,&mut q,&mut dm)?;
    let source=find(&g,name,Zone::Battlefield)?;
    if matches!(name,"Cocoon"|"Living Artifact"|"Parallax Dementia") && g.object(source).unwrap().attached_to!=Some(ironsmith::object::AttachmentTarget::Object(target_id)){return Err("fixture aura not attached".into());}
    if damage>0 {dm.target_name=Some("PLAYER_ALICE".into());let(mut q,p)=cast(&mut g,&defs["Shock"],alice(),&mut dm)?;if p!=1{return Err("Shock payment".into());}resolve(&mut g,&mut q,&mut dm)?;}
    let initial=json!({"source_counters":counter(&g,source,counter_type(name)),"life":g.player(alice()).unwrap().life,"source_paid":paid,"target_tapped":g.is_tapped(target_id)});
    dm.trace.clear();let mut error=None;let mut events=vec![];
    for round in 0..rounds {
        if name=="Magmasaur"&&rounds==6 {dm.accept=round<5;}
        if name=="Noosegraf Mob" {
            g.turn.phase=ironsmith::Phase::FirstMain;g.turn.step=None;
            let spell=witness(&format!("Noosegraf trigger spell {round}"),CardType::Sorcery,1,false);
            let(mut q,p)=cast(&mut g,&spell,alice(),&mut dm)?;if p!=1{return Err("spell payment".into());}
            error=resolve(&mut g,&mut q,&mut dm).err();
        }else {error=phase(&mut g,name=="Guiding Hydra",&mut dm)?;}
        events.push(json!({"round":round,"source_counters":counter(&g,source,counter_type(name)),"source_battlefield":count(&g,name,Zone::Battlefield),"error":error}));
        if error.is_some(){break;}
    }
    let actual=json!({"resolution_error":error,"source_battlefield":count(&g,name,Zone::Battlefield),"source_graveyard":count(&g,name,Zone::Graveyard),"source_counters":counter(&g,source,counter_type(name)),"source_power":g.calculated_power(source).filter(|_|count(&g,name,Zone::Battlefield)>0),"target_battlefield":count(&g,target.name(),Zone::Battlefield),"target_plus_counters":counter(&g,target_id,CounterType::PlusOnePlusOne),"target_flying":g.object_has_static_ability_id(target_id,ironsmith::static_abilities::StaticAbilityId::Flying),"target_damage":g.damage_on(target_id),"zombie_tokens":count(&g,"Zombie",Zone::Battlefield),"life":g.player(alice()).unwrap().life,"bob_life":g.player(PlayerId::from_index(1)).unwrap().life});
    Ok((actual,json!({"initial":initial,"events":events,"choices":dm.trace})))
}
#[test]
#[ignore="manual strict paid-cast counter trigger audit; inspect report row statuses"]
fn report_counter_outcome_execution(){
 let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
 let paths=[std::env::current_exe().unwrap(),ironsmith_tools::default_cards_path(),root.join("crates/ironsmith-tools/tests/runtime_counter_outcome_reproductions.rs")];
 let before:Vec<_>=paths.iter().map(|p|hash(p)).collect();
 let names=["Cocoon","Guiding Hydra","Living Artifact","Magmasaur","Noosegraf Mob","Parallax Dementia","Rusting Golem","Glorious Anthem","Shock"].map(str::to_owned).to_vec();
 let payloads=ironsmith_tools::load_card_payloads_by_names(ironsmith_tools::default_cards_path().to_str().unwrap(),&names).unwrap();
 let mut defs=HashMap::new();let mut compilation=vec![];
 for p in payloads.into_values().flatten(){let b=ironsmith_compiler::CardDefinitionBuilder::new(CardId::new(),p.parse_name.as_deref().unwrap_or(&p.name));let(a,d)=ironsmith_registry::compile_builder_to_artifact(b,&p.parse_input,false).unwrap();compilation.push(json!({"card":p.name,"artifact_checksum":a.payload_checksum,"definition":a.payload.definition}));defs.insert(d.name().to_owned(),d);}
 let mut rows=vec![];
 for (name,rounds,accept,x,anthemed,damage) in [
  ("Cocoon",1,true,0,false,0),("Cocoon",3,true,0,false,0),("Cocoon",4,true,0,false,0),
  ("Guiding Hydra",1,true,2,false,0),("Guiding Hydra",1,false,2,false,0),("Guiding Hydra",1,true,1,false,0),("Guiding Hydra",1,false,0,true,0),("Guiding Hydra",1,true,0,true,0),
  ("Living Artifact",1,true,0,false,1),("Living Artifact",1,false,0,false,1),("Living Artifact",1,false,0,false,0),("Living Artifact",1,true,0,false,0),("Living Artifact",3,false,0,false,1),
  ("Magmasaur",1,true,0,false,0),("Magmasaur",1,false,0,false,0),("Magmasaur",5,true,0,true,0),("Magmasaur",6,false,0,true,0),
  ("Noosegraf Mob",1,true,0,false,0),("Noosegraf Mob",5,true,0,false,0),("Noosegraf Mob",6,true,0,true,0),
  ("Parallax Dementia",1,true,0,false,0),("Parallax Dementia",2,true,0,false,0),
  ("Rusting Golem",1,true,0,false,0),("Rusting Golem",5,true,0,false,0),("Rusting Golem",6,true,0,true,0),
 ]{
  let mut expected=json!({"resolution_error":null,"source_battlefield":1,"source_graveyard":0,"source_counters":0,"source_power":null,"target_battlefield":1,"target_plus_counters":0,"target_flying":false,"target_damage":0,"zombie_tokens":0,"life":20,"bob_life":20});
  match name{
   "Cocoon"=>{if rounds<=3{expected["source_counters"]=json!(3-rounds);}else{expected["source_battlefield"]=json!(0);expected["source_graveyard"]=json!(1);expected["target_plus_counters"]=json!(1);expected["target_flying"]=json!(true);}},
   "Guiding Hydra"=>{let removed=u32::from(accept&&x>0);expected["source_counters"]=json!(x-removed);expected["target_plus_counters"]=json!(removed);if x==removed&&!anthemed{expected["source_battlefield"]=json!(0);expected["source_graveyard"]=json!(1);}else{expected["source_power"]=json!(1+x-removed+u32::from(anthemed));}},
   "Living Artifact"=>{let initial=if damage>0{2}else{0};let removed=usize::from(accept&&initial>0).min(initial);expected["source_counters"]=json!(initial-removed);expected["life"]=json!(20-initial+removed);},
   "Magmasaur"=>{if accept {expected["source_counters"]=json!(5-rounds.min(5));expected["source_power"]=json!(5-rounds.min(5)+usize::from(anthemed));}else{expected["source_battlefield"]=json!(0);expected["source_graveyard"]=json!(1);expected["target_damage"]=json!(if rounds==6{0}else{5});expected["life"]=json!(if rounds==6{20}else{15});expected["bob_life"]=json!(if rounds==6{20}else{15});}},
   "Noosegraf Mob"=>{expected["source_counters"]=json!(5-rounds.min(5));expected["zombie_tokens"]=json!(rounds.min(5));expected["life"]=json!(20+rounds);if rounds>=5&&!anthemed{expected["source_battlefield"]=json!(0);expected["source_graveyard"]=json!(1);}else{expected["source_power"]=json!(5-rounds.min(5)+usize::from(anthemed));}},
   "Parallax Dementia"=>{if rounds>=2{expected["source_battlefield"]=json!(0);expected["source_graveyard"]=json!(1);expected["target_battlefield"]=json!(0);}},
   "Rusting Golem"=>{expected["source_counters"]=json!(5-rounds.min(5));if rounds>=6||rounds>=5&&!anthemed{expected["source_battlefield"]=json!(0);expected["source_graveyard"]=json!(1);}else{expected["source_power"]=json!(5-rounds+usize::from(anthemed));}},_=>unreachable!()
  }
  let(status,actual,diag)=match scenario(&defs,name,rounds,accept,x,anthemed,damage){Ok((v,t))if v==expected=>("passed",v,t),Ok((v,t))if !v["resolution_error"].is_null()=>("confirmed_resolution_failure",v,t),Ok((v,t))=>("semantic_mismatch",v,t),Err(e)=>("execution_or_fixture_error",json!({"error":e}),Value::Null)};
  let unavailable=accept&&(name=="Living Artifact"&&damage==0||name=="Guiding Hydra"&&x==0);
  rows.push(json!({"card":name,"scenario":format!("rounds={rounds}, accept={accept}, cast_X={x}, GloriousAnthem={anthemed}, Shock={damage}"),"status":if unavailable{"unavailable_optional_cost_probe"}else{status},"expected":expected,"actual":actual,"diagnostics":diag}));
 }
 let after:Vec<_>=paths.iter().map(|p|hash(p)).collect();let report=json!({"scope":"Strict paid casts and production priority resolution including ETB/SBA. Canonical Glorious Anthem keeps exhausted zero-counter creatures alive where needed. Repeated real step events and actual paid spell-cast events. Living Artifact counters produced by actual paid Shock targeting controller. No direct effect dispatch or injected outcome state; phase position and neutral target permanents are seeded.","provenance":{"before":before,"after":after,"artifacts_unchanged":before==after},"compilation":compilation,"rows":rows});
 std::fs::write(root.join("reports/runtime-audit/counter-outcome-execution.json"),serde_json::to_string_pretty(&report).unwrap()+"\n").unwrap();println!("wrote {} cases",report["rows"].as_array().unwrap().len());
}
