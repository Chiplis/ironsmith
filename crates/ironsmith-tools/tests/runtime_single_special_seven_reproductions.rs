//! Canonical fixed multi-permanent tap costs with exact output and resource boundaries.
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{DecisionMaker, GameProgress, LegalAction, compute_legal_actions};
use ironsmith::decisions::context::{
    DecisionContext, SelectObjectsContext, SelectOptionsContext, TargetsContext,
};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, advance_priority_with_dm, apply_decision_context_with_dm,
    apply_priority_response_with_dm,
};
use ironsmith::game_state::Target;
use ironsmith::mana::ManaSymbol;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardDefinition, CardId, CardType, GameState, ObjectId, PlayerId, Zone};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

struct Choices {
    targets: Vec<Target>,
    names: Vec<String>,
    x: u32,
    allow_optional: bool,
    choose_untap: bool,
    trace: Vec<Value>,
}
impl DecisionMaker for Choices {
    fn answers_player_choices(&self) -> bool {
        true
    }
    fn decide_number(
        &mut self,
        _: &GameState,
        c: &ironsmith::decisions::context::NumberContext,
    ) -> u32 {
        let s = self.x.max(c.min).min(c.max);
        self.trace.push(json!({"choice":"number","context":format!("{c:?}"),"minimum":c.min,"maximum":c.max,"is_x_value":c.is_x_value,"requested":self.x,"selected":s}));
        s
    }
    fn decide_boolean(
        &mut self,
        _: &GameState,
        c: &ironsmith::decisions::context::BooleanContext,
    ) -> bool {
        self.trace.push(
            json!({"choice":"boolean","context":format!("{c:?}"),"selected":self.allow_optional}),
        );
        self.allow_optional
    }
    fn decide_colors(
        &mut self,
        _: &GameState,
        c: &ironsmith::decisions::context::ColorsContext,
    ) -> Vec<ironsmith::color::Color> {
        let s = vec![ironsmith::color::Color::Blue; c.count as usize];
        self.trace.push(
            json!({"choice":"colors","context":format!("{c:?}"),"selected":format!("{s:?}")}),
        );
        s
    }
    fn decide_targets(&mut self, _: &GameState, c: &TargetsContext) -> Vec<Target> {
        self.trace.push(json!({"choice":"targets","context":format!("{c:?}"),"selected":format!("{:?}",self.targets)}));
        self.targets.clone()
    }
    fn decide_options(&mut self, g: &GameState, c: &SelectOptionsContext) -> Vec<usize> {
        let s = if let Some(o)=c.options.iter().find(|o|o.legal&&o.description=="Blue"){vec![o.index]} else if let Some(o) = c.options.iter().find(|o| {
            o.legal
                && if self.choose_untap {
                    o.description.to_lowercase().contains("untap")
                } else {
                    o.description.to_lowercase().starts_with("tap ")
                }
        }) {
            vec![o.index]
        } else {
            ironsmith::decision::SelectFirstDecisionMaker.decide_options(g, c)
        };
        self.trace
            .push(json!({"choice":"options","context":format!("{c:?}"),"selected":s}));
        s
    }
    fn decide_partition(
        &mut self,
        _: &GameState,
        c: &ironsmith::decisions::context::PartitionContext,
    ) -> Vec<ObjectId> {
        let selected = c.cards.iter().map(|(id, _)| *id).collect::<Vec<_>>();
        self.trace.push(json!({"choice":"partition_to_graveyard","context":format!("{c:?}"),"selected":format!("{selected:?}")}));
        selected
    }
    fn decide_objects(&mut self, g: &GameState, c: &SelectObjectsContext) -> Vec<ObjectId> {
        let mut s = vec![];
        for name in &self.names {
            for o in &c.candidates {
                if o.legal && g.object(o.id).is_some_and(|o| o.name == *name) && !s.contains(&o.id)
                {
                    s.push(o.id)
                }
            }
        }
        s.truncate(c.max.unwrap_or(s.len()));
        if s.len() < c.min {
            for o in &c.candidates {
                if o.legal && !s.contains(&o.id) {
                    s.push(o.id);
                    if s.len() >= c.min {
                        break;
                    }
                }
            }
        }
        self.trace.push(json!({"choice":"objects","context":format!("{c:?}"),"selected":s.iter().map(|id|json!({"id":id.0,"name":g.object(*id).map(|o|o.name.to_string())})).collect::<Vec<_>>()}));
        s
    }
}
fn setup(players: usize, lands: usize) -> GameState {
    let mut g = GameState::new(
        ["Alice", "Bob", "Cara"][..players]
            .iter()
            .map(|s| s.to_string())
            .collect(),
        20,
    );
    g.turn.turn_number = 3;
    g.turn.active_player = PlayerId(0);
    g.turn.priority_player = Some(PlayerId(0));
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
        g.player_mut(PlayerId(0)).unwrap().mana_pool.add(color, 30);
    }
    for _ in 0..lands {
        let def = CardDefinitionBuilder::new(CardId::new(), "Trigger sacrifice land")
            .card_types(vec![CardType::Land])
            .build();
        g.create_object_from_definition(&def, PlayerId(0), Zone::Battlefield);
    }
    g
}
fn announce(
    g: &mut GameState,
    def: &CardDefinition,
    actor: u8,
    dm: &mut Choices,
) -> Result<(TriggerQueue, Value), String> {
    g.turn.priority_player = Some(PlayerId(actor));
    let source = g.create_object_from_definition(def, PlayerId(actor), Zone::Hand);
    let action = compute_legal_actions(g, PlayerId(actor)).expect("fixture has complete replacement state")
        .into_iter()
        .find(|a| matches!(a,LegalAction::CastSpell{spell_id,..}if *spell_id==source))
        .ok_or("intended cast unavailable")?;
    let mana = g.player(PlayerId(actor)).unwrap().mana_pool.total();
    let mut q = TriggerQueue::new();
    let mut state = PriorityLoopState::new(g.players_in_game());
    let mut progress = apply_priority_response_with_dm(
        g,
        &mut q,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    )
    .map_err(|e| e.to_string())?;
    for _ in 0..24 {
        if state.pending_cast.is_none() && !g.stack.is_empty() {
            break;
        }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else {
            return Err(format!("announcement stopped:{progress:?}"));
        };
        if matches!(ctx, DecisionContext::Priority(_)) {
            return Err("announcement returned priority without spell".into());
        }
        progress = apply_decision_context_with_dm(g, &mut q, &mut state, &ctx, dm)
            .map_err(|e| e.to_string())?;
    }
    if g.stack.is_empty() {
        return Err("announcement budget".into());
    }
    let entry = g
        .stack
        .iter()
        .find(|entry| {
            g.object(entry.object_id)
                .is_some_and(|o| o.name == def.name())
        })
        .ok_or("intended spell missing from stack")?;
    let evidence = json!({"spell":def.name(),"mana_paid":mana-g.player(PlayerId(actor)).unwrap().mana_pool.total(),"announced_targets":format!("{:?}",entry.targets),"resolution_error":null,"stack_names_at_announcement":g.stack.iter().filter_map(|e|g.object(e.object_id).map(|o|o.name.to_string())).collect::<Vec<_>>()});
    Ok((q, evidence))
}
fn cast(
    g: &mut GameState,
    def: &CardDefinition,
    actor: u8,
    dm: &mut Choices,
) -> Result<Value, String> {
    let (mut q, mut evidence) = announce(g, def, actor, dm)?;
    let mut state = PriorityLoopState::new(g.players_in_game());
    for _ in 0..24 {
        if let Err(error) = advance_priority_with_dm(g, &mut q, dm) {
            evidence["resolution_error"] = json!(error.to_string());
            return Ok(evidence);
        }
        if g.stack.is_empty() {
            return Ok(evidence);
        }
        state.reset_for_new_priority_window(g);
        for _ in 0..g.players_in_game() {
            if let Err(error) = apply_priority_response_with_dm(
                g,
                &mut q,
                &mut state,
                &PriorityResponse::PriorityAction(LegalAction::PassPriority),
                dm,
            ) {
                evidence["resolution_error"] = json!(error.to_string());
                return Ok(evidence);
            }
        }
    }
    Err("resolution decision budget".into())
}

fn finish(g: &mut GameState, q: &mut TriggerQueue, dm: &mut Choices) -> Result<(), String> {
    let mut state = PriorityLoopState::new(g.players_in_game());
    for _ in 0..24 {
        advance_priority_with_dm(g, q, dm).map_err(|e| e.to_string())?;
        if g.stack.is_empty() {
            return Ok(());
        }
        state.reset_for_new_priority_window(g);
        for _ in 0..g.players_in_game() {
            apply_priority_response_with_dm(
                g,
                q,
                &mut state,
                &PriorityResponse::PriorityAction(LegalAction::PassPriority),
                dm,
            )
            .map_err(|e| e.to_string())?;
        }
    }
    Err("finish budget".into())
}
fn find(g: &GameState, name: &str, zone: Zone) -> Result<ObjectId, String> {
    g.objects_in_deterministic_order()
        .iter()
        .find(|o| o.name == name && o.zone == zone)
        .map(|o| o.id)
        .ok_or(format!("{name} absent in {zone:?}"))
}
fn paid(
    g: &mut GameState,
    defs: &HashMap<String, CardDefinition>,
    name: &str,
    dm: &mut Choices,
    mana: u32,
) -> Result<Value, String> {
    let e = cast(g, &defs[name], 0, dm)?;
    if e["mana_paid"] != mana || !e["resolution_error"].is_null() {
        return Err(format!("{name} cast/payment mismatch:{e}"));
    }
    Ok(e)
}
fn count(g: &GameState, name: &str, zone: Zone) -> usize {
    g.objects_in_deterministic_order()
        .iter()
        .filter(|o| o.name == name && o.zone == zone)
        .count()
}
fn action(
    g: &mut GameState,
    source: ObjectId,
    index: usize,
    mana: bool,
    dm: &mut Choices,
) -> Result<Value, String> {
    let a = compute_legal_actions(g, PlayerId(0)).expect("fixture has complete replacement state")
        .into_iter()
        .find(|a| match a {
            LegalAction::ActivateAbility {
                source: s,
                ability_index,
            } => !mana && *s == source && *ability_index == index,
            LegalAction::ActivateManaAbility {
                source: s,
                ability_index,
            } => mana && *s == source && *ability_index == index,
            _ => false,
        })
        .ok_or("intended activation unavailable")?;
    let before = g.player(PlayerId(0)).unwrap().mana_pool.total();
    let mut q = TriggerQueue::new();
    let mut st = PriorityLoopState::new(g.players_in_game());
    let mut progress = apply_priority_response_with_dm(
        g,
        &mut q,
        &mut st,
        &PriorityResponse::PriorityAction(a.clone()),
        dm,
    )
    .map_err(|e| e.to_string())?;
    for _ in 0..32 {
        match progress {
            GameProgress::NeedsDecisionCtx(ref ctx)
                if !matches!(ctx, DecisionContext::Priority(_)) =>
            {
                progress = apply_decision_context_with_dm(g, &mut q, &mut st, ctx, dm)
                    .map_err(|e| e.to_string())?;
            }
            _ => {
                let taps = g
                    .battlefield
                    .iter()
                    .filter(|id| g.is_tapped(**id))
                    .map(|id| id.0)
                    .collect::<Vec<_>>();
                let err = finish(g, &mut q, dm).err();
                return Ok(
                    json!({"action":format!("{a:?}"),"mana_change":g.player(PlayerId(0)).unwrap().mana_pool.total() as i64-before as i64,"resolution_error":err,"tapped_at_announcement":taps}),
                );
            }
        }
    }
    Err("activation decision budget".into())
}

fn fund(g: &mut GameState, actor: PlayerId) {
    for c in [
        ManaSymbol::White,
        ManaSymbol::Blue,
        ManaSymbol::Black,
        ManaSymbol::Red,
        ManaSymbol::Green,
        ManaSymbol::Colorless,
    ] {
        g.player_mut(actor).unwrap().mana_pool.add(c, 30);
    }
}
fn next_main(g: &mut GameState) {
    g.next_turn();
    ironsmith::turn::execute_untap_step(g);
    g.turn.phase = ironsmith::Phase::FirstMain;
    g.turn.step = None;
    g.turn.priority_player = Some(g.turn.active_player);
    fund(g, g.turn.active_player);
}

fn play_land(g: &mut GameState, d: &CardDefinition, dm: &mut Choices) -> Result<ObjectId, String> {
    let hand = g.create_object_from_definition(d, g.turn.active_player, Zone::Hand);
    let stable=g.object(hand).unwrap().stable_id;
    let a = compute_legal_actions(g, g.turn.active_player).expect("fixture has complete replacement state")
        .into_iter()
        .find(|a| matches!(a,LegalAction::PlayLand{land_id}if *land_id==hand))
        .ok_or("land play missing")?;
    let mut q = TriggerQueue::new();
    let mut st = PriorityLoopState::new(g.players_in_game());
    apply_priority_response_with_dm(g, &mut q, &mut st, &PriorityResponse::PriorityAction(a), dm)
        .map_err(|e| e.to_string())?;
    finish(g, &mut q, dm)?;
    g.objects_in_deterministic_order().into_iter().find(|o|o.stable_id==stable&&o.zone==Zone::Battlefield).map(|o|o.id).ok_or("played land missing".into())
}


const NAMES:[&str;7]=["Aura of Dominion","Coral Reef","Earthcraft","Honor-Worn Shaku","Leyline Dowser","Radiant, Serra Archangel","Spire Mechcycle"];
fn counter(g:&GameState,id:ObjectId,name:&str)->u32{g.object(id).unwrap().counters.iter().filter(|(k,_)|k.description()==name).map(|(_,v)|*v).sum()}
fn add_paid(g:&mut GameState,defs:&HashMap<String,CardDefinition>,name:&str,dm:&mut Choices,price:u32,ev:&mut Vec<Value>)->Result<ObjectId,String>{
 let before=g.battlefield.clone();ev.push(paid(g,defs,name,dm,price)?);g.battlefield.iter().find(|id|!before.contains(id)&&g.object(**id).is_some_and(|o|o.name==name)).copied().ok_or(format!("new {name} missing"))
}
fn run(defs:&HashMap<String,CardDefinition>,name:&str,n:usize,state:&str)->Result<(Value,Value,Value),String>{
 let mut g=setup(2,0);let mut dm=Choices{targets:vec![],names:vec![],x:0,allow_optional:true,choose_untap:false,trace:vec![]};let mut producers=vec![];
 let mut host=None;
 if name=="Aura of Dominion" {host=Some(add_paid(&mut g,defs,"Grizzly Bears",&mut dm,2,&mut producers)?);dm.targets=vec![Target::Object(host.unwrap())];}
 let price=match name{"Honor-Worn Shaku"=>3,"Radiant, Serra Archangel"=>7,"Spire Mechcycle"=>5,_=>2};
 let source_cast=paid(&mut g,defs,name,&mut dm,price)?;let source=find(&g,name,Zone::Battlefield)?;
 if name=="Aura of Dominion"&&g.object(source).unwrap().attached_to!=Some(ironsmith::object::AttachmentTarget::Object(host.unwrap())) {return Err("actual Aura not attached".into());}
 if name=="Coral Reef"{host=Some(add_paid(&mut g,defs,"Grizzly Bears",&mut dm,2,&mut producers)?);if counter(&g,source,"polyp")!=4{return Err("entry polyp count mismatch".into());}}
 if name=="Earthcraft"{host=Some(play_land(&mut g,&defs["Forest"],&mut dm)?);producers.push(json!({"land_play":"Forest","id":host.unwrap().0}));producers.push(action(&mut g,host.unwrap(),0,true,&mut dm)?);if !g.is_tapped(host.unwrap()){return Err("Forest mana did not tap".into());}}
 if name=="Aura of Dominion"||name=="Honor-Worn Shaku"||name=="Leyline Dowser"{dm.targets=vec![Target::Object(host.unwrap_or(source))];producers.push(paid(&mut g,defs,"Twiddle",&mut dm,1)?);if !g.is_tapped(host.unwrap_or(source)){return Err("target tap producer failed".into());}}
 let mut resources=vec![];let mut resource_names=vec![];
 for k in 0..n{
  let (r,price)=match name{
   "Coral Reef"=>("Merfolk of the Pearl Trident",1),
   "Honor-Worn Shaku"|"Leyline Dowser"=>if k==0{("Isamaru, Hound of Konda",1)}else{("Rograkh, Son of Rohgahh",0)},
   "Radiant, Serra Archangel"=>("Suntail Hawk",1),
   "Spire Mechcycle"=>if state=="mount"||(state=="mixed"&&k==1){("Trained Arynx",2)}else{("Sky Skiff",2)},
   _=>("Grizzly Bears",2),
  };
  dm.targets.clear();resources.push(add_paid(&mut g,defs,r,&mut dm,price,&mut producers)?);resource_names.push(r.to_string());
 }
 if state=="legend_artifact"{dm.targets.clear();resources.push(add_paid(&mut g,defs,"Shadowspear",&mut dm,1,&mut producers)?);resource_names.push("Shadowspear".into());}
 if n==0&&state!="legend_artifact"{let decoy=match name{"Aura of Dominion"|"Earthcraft"=>"Mind Stone","Leyline Dowser"=>"Shadowspear",_=>"Grizzly Bears"};dm.targets.clear();producers.push(paid(&mut g,defs,decoy,&mut dm,if decoy=="Shadowspear"{1}else{2})?);}
 if state=="tapped"{dm.targets=vec![Target::Object(resources[0])];producers.push(paid(&mut g,defs,"Twiddle",&mut dm,1)?);if !g.is_tapped(resources[0]){return Err("resource tap failed".into());}}
 dm.names=resource_names;dm.targets=if name=="Earthcraft"||name=="Coral Reef"{vec![Target::Object(host.unwrap())]}else{vec![]};
 let index=match name{"Coral Reef"=>2,"Honor-Worn Shaku"|"Leyline Dowser"|"Radiant, Serra Archangel"|"Spire Mechcycle"=>1,_=>0};
 let actions=compute_legal_actions(&g,PlayerId(0)).expect("fixture has complete replacement state");let offered=actions.iter().any(|a|matches!(a,LegalAction::ActivateAbility{source:s,ability_index}if *s==source&&*ability_index==index));
 let valid=(n>0&&state!="tapped")||(state=="legend_artifact"&&name=="Honor-Worn Shaku");
 let before=json!({"actions":format!("{actions:?}"),"source_index":index,"source_tapped":g.is_tapped(source),"source_counters":format!("{:?}",g.object(source).unwrap().counters),"host":host.map(|id|json!({"id":id.0,"tapped":g.is_tapped(id)})),"resources":resources.iter().map(|id|json!({"id":id.0,"name":g.object(*id).unwrap().name.to_string(),"tapped":g.is_tapped(*id),"fresh":g.is_summoning_sick(*id)})).collect::<Vec<_>>()});
 if !valid||!offered{return Ok((json!({"activation_available":valid}),json!({"activation_available":offered}),json!({"source_cast":source_cast,"producers":producers,"before_activation":before,"activation_dispatched":false,"choice_trace":dm.trace})));}
 let mana=g.player(PlayerId(0)).unwrap().mana_pool.total();let activation=action(&mut g,source,index,false,&mut dm).unwrap_or_else(|e|json!({"resolution_error":e}));
 let mut actual=json!({"error":activation["resolution_error"],"mana_paid":mana-g.player(PlayerId(0)).unwrap().mana_pool.total(),"tapped_resources":resources.iter().filter(|id|g.is_tapped(**id)).count(),"untapped_resources":resources.iter().filter(|id|!g.is_tapped(**id)).count(),"stack_length":g.stack.len()});
 let mut expected=json!({"error":null,"mana_paid":if name=="Aura of Dominion"||name=="Coral Reef"{1}else{0},"tapped_resources":1,"untapped_resources":resources.len()-1,"stack_length":0});
 match name{
  "Aura of Dominion"|"Earthcraft"=>{actual["host_untapped"]=json!(!g.is_tapped(host.unwrap()));expected["host_untapped"]=json!(true);},
  "Coral Reef"=>{actual["polyp"]=json!(counter(&g,source,"polyp"));expected["polyp"]=json!(3);actual["target_counter"]=json!(counter(&g,host.unwrap(),"+0/+1"));expected["target_counter"]=json!(1);actual["target_pt"]=json!([g.calculated_power(host.unwrap()),g.calculated_toughness(host.unwrap())]);expected["target_pt"]=json!([2,3]);},
  "Honor-Worn Shaku"|"Leyline Dowser"=>{actual["source_untapped"]=json!(!g.is_tapped(source));expected["source_untapped"]=json!(true);},
  "Radiant, Serra Archangel"=>{
   let blue=g.create_object_from_definition(&defs["Unsummon"],PlayerId(0),Zone::Hand);
   let red=g.create_object_from_definition(&defs["Shock"],PlayerId(0),Zone::Hand);
   actual["blue_protection"]=json!(ironsmith::targeting::has_protection_from_source(&g,source,blue));expected["blue_protection"]=json!(true);
   actual["red_protection"]=json!(ironsmith::targeting::has_protection_from_source(&g,source,red));expected["red_protection"]=json!(false);
   dm.targets=vec![Target::Object(source)];producers.push(paid(&mut g,defs,"Shock",&mut dm,1)?);actual["red_damage"]=json!(g.damage_on(source));expected["red_damage"]=json!(2);
  },
  "Spire Mechcycle"=>{actual["creature"]=json!(g.calculated_card_types(source).contains(&CardType::Creature));expected["creature"]=json!(true);actual["plus"]=json!(counter(&g,source,"+1/+1"));expected["plus"]=json!(n);actual["source_pt"]=json!([g.calculated_power(source),g.calculated_toughness(source)]);expected["source_pt"]=json!([5+n,4+n]);},_=>{}
 }
 g.turn.phase=ironsmith::Phase::Ending;g.turn.step=Some(ironsmith::Step::Cleanup);ironsmith::turn::execute_cleanup_step(&mut g);
 if name=="Radiant, Serra Archangel"{
  let blue=find(&g,"Unsummon",Zone::Hand)?;actual["cleanup_blue_protection"]=json!(ironsmith::targeting::has_protection_from_source(&g,source,blue));expected["cleanup_blue_protection"]=json!(false);
  next_main(&mut g);next_main(&mut g);dm.targets=vec![Target::Object(source)];producers.push(paid(&mut g,defs,"Unsummon",&mut dm,1)?);actual["cleanup_unsummon_returns_source"]=json!(count(&g,name,Zone::Hand)==1);expected["cleanup_unsummon_returns_source"]=json!(true);
 }
 if name=="Spire Mechcycle"{
  actual["cleanup_creature"]=json!(g.calculated_card_types(source).contains(&CardType::Creature));expected["cleanup_creature"]=json!(true);
  next_main(&mut g);next_main(&mut g);actual["next_turn_creature"]=json!(g.calculated_card_types(source).contains(&CardType::Creature));expected["next_turn_creature"]=json!(true);
  actual["next_turn_plus"]=json!(counter(&g,source,"+1/+1"));expected["next_turn_plus"]=json!(n);
  actual["exhaust_repeat_available"]=json!(compute_legal_actions(&g,PlayerId(0)).expect("fixture has complete replacement state").iter().any(|a|matches!(a,LegalAction::ActivateAbility{source:s,ability_index}if *s==source&&*ability_index==index)));expected["exhaust_repeat_available"]=json!(false);
 }
 Ok((expected,actual,json!({"source_cast":source_cast,"producers":producers,"before_activation":before,"activation":activation,"activation_dispatched":true,"choice_trace":dm.trace,"scope":"Actual paid sources/resources and Twiddle tapped-state producers; real Aura attachment, Forest land play/mana activation, Coral Reef entry counters, color protection and cleanup, exhaust permanence and repeat gate."})))
}
fn hash(p: &std::path::Path) -> String {
    Sha256::digest(std::fs::read(p).unwrap())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
#[test]
#[ignore = "fixed multi-permanent tap cost audit"]
fn report_single_special_seven() {
    let input = std::path::PathBuf::from(std::env::var("AUDIT_RUNTIME_INVENTORY").unwrap());
    let payloads: Value = serde_json::from_slice(&std::fs::read(&input).unwrap()).unwrap();
    let mut defs = HashMap::new();
    let mut artifacts = vec![];
    for p in payloads["cards"].as_array().unwrap() {
        let n = p["name"].as_str().unwrap();
        if !NAMES.contains(&n)
            && !["Forest","Grizzly Bears","Twiddle","Merfolk of the Pearl Trident","Isamaru, Hound of Konda","Rograkh, Son of Rohgahh","Suntail Hawk","Sky Skiff","Trained Arynx","Shadowspear","Mind Stone","Unsummon","Shock"]
            .contains(&n)
        {
            continue;
        }
        let (a, d) = ironsmith_registry::compile_builder_to_artifact(
            ironsmith_compiler::CardDefinitionBuilder::new(
                CardId::new(),
                p["parse_name"].as_str().unwrap_or(n),
            ),
            p["parse_input"].as_str().unwrap(),
            false,
        )
        .unwrap();
        artifacts.push(
            json!({"card":n,"checksum":a.payload_checksum,"definition":a.payload.definition}),
        );
        defs.insert(n.to_string(), d);
    }
    let mut rows = vec![];
    for name in NAMES {
        let mut cases = vec![(0, "ready"), (1, "ready"), (2, "ready"), (1, "tapped")];
        if name == "Honor-Worn Shaku" || name == "Leyline Dowser" {cases.push((0,"legend_artifact"));}
        if name == "Spire Mechcycle" {cases.push((1,"mount"));cases.push((2,"mixed"));}
        for (resources, state) in cases {
            let (status, expected, actual, evidence) = match run(&defs, name, resources, state) {
                Ok((e, a, f)) => (
                    if e == a {
                        "expected_result_observed"
                    } else if !a["error"].is_null() {
                        "execution_failed"
                    } else {
                        "semantic_mismatch"
                    },
                    e,
                    a,
                    f,
                ),
                Err(e) => (
                    "fixture_or_producer_error",
                    Value::Null,
                    json!({"error":e}),
                    Value::Null,
                ),
            };
            rows.push(json!({"card":name,"scenario":{"resources":resources,"state":state},"status":status,"expected":expected,"actual":actual,"fixture_evidence":evidence}));
        }
    }
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let binary = std::env::current_exe().unwrap();
    let r = json!({"scope":"Seven remaining special single-object tap costs; actual payments and expected results with resource boundaries, cleanup and exhaust checks.","rows":rows,"artifacts":artifacts,"provenance":{"binary":binary,"binary_sha256":hash(&binary),"inventory_sha256":hash(&input),"source_sha256":hash(&root.join("crates/ironsmith-tools/tests/runtime_single_special_seven_reproductions.rs")),"runtime_stack_bytes":67108864}});
    std::fs::write(
        root.join("reports/runtime-audit/single-special-seven-reproductions.json"),
        serde_json::to_string_pretty(&r).unwrap(),
    )
    .unwrap();
    println!("{}", serde_json::to_string_pretty(&r).unwrap());
}
