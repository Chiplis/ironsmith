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



fn dispatch(g:&mut GameState,a:LegalAction,dm:&mut Choices)->Result<Value,String>{
 let mana=g.player(PlayerId(0)).unwrap().mana_pool.total();let mut q=TriggerQueue::new();let mut st=PriorityLoopState::new(g.players_in_game());
 let mut progress=apply_priority_response_with_dm(g,&mut q,&mut st,&PriorityResponse::PriorityAction(a.clone()),dm).map_err(|e|e.to_string())?;
 for _ in 0..40{
  if let GameProgress::NeedsDecisionCtx(ref ctx)=progress {if !matches!(ctx,DecisionContext::Priority(_)){progress=apply_decision_context_with_dm(g,&mut q,&mut st,ctx,dm).map_err(|e|e.to_string())?;continue;}}
  let announced=json!({"stack":g.stack.iter().map(|e|json!({"name":g.object(e.object_id).map(|o|o.name.to_string()),"targets":format!("{:?}",e.targets)})).collect::<Vec<_>>(),"mana_paid":mana-g.player(PlayerId(0)).unwrap().mana_pool.total(),"alice_hand":g.player(PlayerId(0)).unwrap().hand.iter().filter_map(|id|g.object(*id).map(|o|o.name.to_string())).collect::<Vec<_>>()});
  let error=finish(g,&mut q,dm).err();return Ok(json!({"action":format!("{a:?}"),"announcement":announced,"error":error}));
 }
 Err("casting decision budget".into())
}


const NAMES:[&str;7]=["Angelic Favor","Lashknife","Orim's Cure","Ramosian Rally","Zahid, Djinn of the Lamp","Sephara, Sky's Blade","The Lady of Otaria"];
fn begin_combat(g:&mut GameState,dm:&mut Choices)->Result<ironsmith::turn_runner::TurnRunner,String>{
 use ironsmith::turn_runner::{TurnAction,TurnRunner,TurnState};let mut r=TurnRunner::from_state_for_sync(TurnState::FirstMainPriority);let mut q=TriggerQueue::new();
 for _ in 0..20{match r.advance(g,&mut q).map_err(|e|e.to_string())?{TurnAction::Continue=>{},TurnAction::RunPriority=>{if matches!(r.state(),TurnState::BeginCombatPriority){finish(g,&mut q,dm)?;return Ok(r);}ironsmith::game_loop::run_priority_loop_with(g,&mut q,&mut ironsmith::decision::AutoPassDecisionMaker).map_err(|e|e.to_string())?;r.priority_done();},a=>return Err(format!("unexpected begincombat:{a:?}"))}}
 Err("combat entry budget".into())
}
fn end_step(g:&mut GameState,mut r:ironsmith::turn_runner::TurnRunner,dm:&mut Choices)->Result<(),String>{
 use ironsmith::turn_runner::{TurnAction,TurnState};let mut q=TriggerQueue::new();r.priority_done();
 for _ in 0..100{match r.advance(g,&mut q).map_err(|e|e.to_string())?{TurnAction::Continue=>{},TurnAction::RunPriority=>{finish(g,&mut q,dm)?;if matches!(r.state(),TurnState::EndStepPriority){return Ok(());}ironsmith::game_loop::run_priority_loop_with(g,&mut q,&mut ironsmith::decision::AutoPassDecisionMaker).map_err(|e|e.to_string())?;r.priority_done();},TurnAction::Decision(DecisionContext::Attackers(_))=>r.respond_attackers(vec![]),TurnAction::Decision(DecisionContext::Blockers(c))=>r.respond_blockers(vec![],c.player),a=>return Err(format!("unexpected endstep:{a:?}"))}}
 Err("endstep budget".into())
}
fn run(defs:&HashMap<String,CardDefinition>,name:&str,n:usize,state:&str)->Result<(Value,Value,Value),String>{
 let mut g=setup(2,0);let mut dm=Choices{targets:vec![],names:vec![],x:0,allow_optional:true,choose_untap:false,trace:vec![]};let mut producers=vec![];let mut resources=vec![];let mut mana_lands=vec![];
 let plains_gate=matches!(name,"Angelic Favor"|"Lashknife"|"Orim's Cure"|"Ramosian Rally");let required=match name{"Sephara, Sky's Blade"=>4,"The Lady of Otaria"=>3,_=>1};
 if plains_gate{let count=if name=="Angelic Favor"&&state=="normal"{4}else{1};for _ in 0..count{let land=if state=="no_plains"{"Island"}else{"Plains"};let id=play_land(&mut g,&defs[land],&mut dm)?;mana_lands.push(id);producers.push(json!({"land_play":land,"id":id.0}));next_main(&mut g);next_main(&mut g);}}
 let resource_name=if state=="wrong_type"{if name=="Zahid, Djinn of the Lamp"{"Grizzly Bears"}else{"Mind Stone"}}else{match name{"Zahid, Djinn of the Lamp"=>"Ornithopter","Sephara, Sky's Blade"=>"Suntail Hawk","The Lady of Otaria"=>"Dwarven Trader",_=>"Grizzly Bears"}};
 for _ in 0..n{let before=g.battlefield.clone();let price=match resource_name{"Ornithopter"=>0,"Suntail Hawk"|"Dwarven Trader"=>1,_=>2};producers.push(paid(&mut g,defs,resource_name,&mut dm,price)?);let id=g.battlefield.iter().find(|id|!before.contains(id)&&g.object(**id).is_some_and(|o|o.name==resource_name)).copied().ok_or("paid resource missing")?;resources.push(id);}
 let host=if name=="Lashknife"{producers.push(paid(&mut g,defs,"Hill Giant",&mut dm,4)?);Some(find(&g,"Hill Giant",Zone::Battlefield)?)}else{None};
 if name=="Lashknife"{dm.targets=vec![Target::Object(host.unwrap())];producers.push(paid(&mut g,defs,"Twiddle",&mut dm,1)?);if !g.is_tapped(host.unwrap()){return Err("host tap failed".into());}}
 if state=="tapped"{dm.targets=vec![Target::Object(resources[0])];producers.push(paid(&mut g,defs,"Twiddle",&mut dm,1)?);if !g.is_tapped(resources[0]){return Err("resource tap failed".into());}}
 let mut runner=None;if name=="Angelic Favor"&&state!="outside_combat"{runner=Some(begin_combat(&mut g,&mut dm)?);if state=="normal"{for id in &mana_lands{producers.push(action(&mut g,*id,0,true,&mut dm)?);}}}
 dm.targets=if let Some(id)=host{vec![Target::Object(id)]}else if name=="Orim's Cure"{vec![Target::Player(PlayerId(0))]}else{vec![]};dm.names=vec![resource_name.into()];
 let source=g.create_object_from_definition(&defs[name],PlayerId(0),Zone::Hand);let stable=g.object(source).unwrap().stable_id;let method=if state=="normal"{ironsmith::alternative_cast::CastingMethod::Normal}else{ironsmith::alternative_cast::CastingMethod::Alternative(0)};
 let actions=compute_legal_actions(&g,PlayerId(0)).expect("fixture has complete replacement state");let offered=actions.iter().find(|a|matches!(a,LegalAction::CastSpell{spell_id,casting_method,..}if *spell_id==source&&*casting_method==method)).cloned();let valid=state=="normal"||(n>=required&&!["tapped","wrong_type","no_plains","outside_combat"].contains(&state));
 let before=json!({"actions":format!("{actions:?}"),"phase":format!("{:?}",g.turn.phase),"method":format!("{method:?}"),"resources":resources.iter().map(|id|json!({"id":id.0,"tapped":g.is_tapped(*id),"fresh":g.is_summoning_sick(*id)})).collect::<Vec<_>>()});
 if !valid||offered.is_none(){return Ok((json!({"intended_cast_available":valid}),json!({"intended_cast_available":offered.is_some()}),json!({"producers":producers,"before_cast":before,"cast_dispatched":false,"choice_trace":dm.trace})));}
 let payment=dispatch(&mut g,offered.unwrap(),&mut dm).unwrap_or_else(|e|json!({"error":e}));let object=g.objects_in_deterministic_order().into_iter().find(|o|o.stable_id==stable).cloned();let taps=if state=="normal"{0}else{required};let price=if state=="normal"{match name{"Lashknife"|"Orim's Cure"=>2,"Zahid, Djinn of the Lamp"=>6,"Sephara, Sky's Blade"=>7,"The Lady of Otaria"=>5,_=>4}}else{match name{"Zahid, Djinn of the Lamp"=>4,"Sephara, Sky's Blade"=>1,_=>0}};
 let permanent=matches!(name,"Lashknife"|"Zahid, Djinn of the Lamp"|"Sephara, Sky's Blade"|"The Lady of Otaria");
 let mut actual=json!({"error":payment["error"],"mana_paid":payment["announcement"]["mana_paid"],"tapped_resources":resources.iter().filter(|id|g.is_tapped(**id)).count(),"source_zone":object.as_ref().map(|o|format!("{:?}",o.zone)),"stack_length":g.stack.len()});let mut expected=json!({"error":null,"mana_paid":price,"tapped_resources":taps,"source_zone":if permanent{"Battlefield"}else{"Graveyard"},"stack_length":0});
 let static_id=ironsmith::static_abilities::StaticAbilityId::FirstStrike;
 if name=="Lashknife"{actual["host_first_strike"]=json!(g.object_has_static_ability_id(host.unwrap(),static_id));expected["host_first_strike"]=json!(true);actual["attached"]=json!(object.as_ref().is_some_and(|o|o.attached_to==Some(ironsmith::object::AttachmentTarget::Object(host.unwrap()))));expected["attached"]=json!(true);}
 if name=="Ramosian Rally"{actual["resource_pt"]=json!(resources.iter().map(|id|[g.calculated_power(*id),g.calculated_toughness(*id)]).collect::<Vec<_>>());expected["resource_pt"]=json!(vec![[3,3];n]);}
 if name=="Orim's Cure"{dm.targets=vec![Target::Player(PlayerId(0))];let mut life=vec![];for _ in 0..3{producers.push(paid(&mut g,defs,"Shock",&mut dm,1)?);life.push(g.player(PlayerId(0)).unwrap().life);}actual["life_after_three_shocks"]=json!(life);expected["life_after_three_shocks"]=json!([20,20,18]);}
 if name=="Angelic Favor"{let ids=g.battlefield.iter().filter(|id|g.object(**id).unwrap().kind==ironsmith::object::ObjectKind::Token).copied().collect::<Vec<_>>();actual["angel_token"]=json!(ids.len()==1&&g.calculated_power(ids[0])==Some(4)&&g.calculated_toughness(ids[0])==Some(4)&&g.current_colors(ids[0])==Some(ironsmith::color::ColorSet::WHITE)&&g.calculated_subtypes(ids[0]).contains(&ironsmith::Subtype::Angel)&&g.object_has_static_ability_id(ids[0],ironsmith::static_abilities::StaticAbilityId::Flying));expected["angel_token"]=json!(true);end_step(&mut g,runner.take().unwrap(),&mut dm)?;actual["end_step_token_absent"]=json!(ids.iter().all(|id|!g.battlefield.contains(id)));expected["end_step_token_absent"]=json!(true);}
 if permanent&&name!="Lashknife"{actual["source_pt"]=object.as_ref().map(|o|json!([g.calculated_power(o.id),g.calculated_toughness(o.id)])).unwrap_or(Value::Null);expected["source_pt"]=match name{"Zahid, Djinn of the Lamp"=>json!([5,6]),"Sephara, Sky's Blade"=>json!([7,7]),_=>json!([5,5])};}
 if name=="Sephara, Sky's Blade"{actual["resources_indestructible"]=json!(resources.iter().filter(|id|g.object_has_static_ability_id(**id,ironsmith::static_abilities::StaticAbilityId::Indestructible)).count());expected["resources_indestructible"]=json!(n);}
 if name=="Ramosian Rally"{g.turn.phase=ironsmith::Phase::Ending;g.turn.step=Some(ironsmith::Step::Cleanup);ironsmith::turn::execute_cleanup_step(&mut g);actual["cleanup_pt"]=json!(resources.iter().map(|id|[g.calculated_power(*id),g.calculated_toughness(*id)]).collect::<Vec<_>>());expected["cleanup_pt"]=json!(vec![[2,2];n]);}
 Ok((expected,actual,json!({"producers":producers,"before_cast":before,"payment":payment,"cast_dispatched":true,"choice_trace":dm.trace,"scope":"Strict alternative methods and normal-price controls, actual land plays and paid resource creatures/artifacts, true combat/end-step transitions for Favor, exact taps and immediate expected results/cleanup. Unpayable advertised actions are not dispatched."})))
}
fn hash(p: &std::path::Path) -> String {
    Sha256::digest(std::fs::read(p).unwrap())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
#[test]
#[ignore = "fixed multi-permanent tap cost audit"]
fn report_spell_alt_tap() {
    let input = std::path::PathBuf::from(std::env::var("AUDIT_RUNTIME_INVENTORY").unwrap());
    let payloads: Value = serde_json::from_slice(&std::fs::read(&input).unwrap()).unwrap();
    let mut defs = HashMap::new();
    let mut artifacts = vec![];
    for p in payloads["cards"].as_array().unwrap() {
        let n = p["name"].as_str().unwrap();
        if !NAMES.contains(&n)
            && !["Island","Plains","Grizzly Bears","Hill Giant","Mind Stone","Ornithopter","Suntail Hawk","Dwarven Trader","Twiddle","Shock"]
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
        let required=match name{"Sephara, Sky's Blade"=>4,"The Lady of Otaria"=>3,_=>1};let mut cases=vec![(required-1,"ready"),(required,"ready"),(required+1,"ready"),(required,"tapped"),(required,"wrong_type"),(0,"normal")];if ["Angelic Favor","Lashknife","Orim's Cure","Ramosian Rally"].contains(&name){cases.push((required,"no_plains"));}if name=="Angelic Favor"{cases.push((1,"outside_combat"));}
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
    let r = json!({"scope":"Seven alternative tap spell costs with paid resource/land producers, exact method selection, resource/condition boundaries and normal-payment controls.","rows":rows,"artifacts":artifacts,"provenance":{"binary":binary,"binary_sha256":hash(&binary),"inventory_sha256":hash(&input),"source_sha256":hash(&root.join("crates/ironsmith-tools/tests/runtime_spell_alt_tap_reproductions.rs")),"runtime_stack_bytes":67108864}});
    std::fs::write(
        root.join("reports/runtime-audit/spell-alt-tap-reproductions.json"),
        serde_json::to_string_pretty(&r).unwrap(),
    )
    .unwrap();
    println!("{}", serde_json::to_string_pretty(&r).unwrap());
}
