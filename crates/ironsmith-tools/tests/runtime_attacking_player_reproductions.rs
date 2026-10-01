//! Actual combat states for synthetic attacking-player context candidates.
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

fn finish(g: &mut GameState, q: &mut TriggerQueue, dm: &mut Choices) -> Result<(),String> {
    let mut state=PriorityLoopState::new(g.players_in_game());
    for _ in 0..24 {
        advance_priority_with_dm(g,q,dm).map_err(|e|e.to_string())?;
        if g.stack.is_empty() {return Ok(());}
        state.reset_for_new_priority_window(g);
        for _ in 0..g.players_in_game() {
            apply_priority_response_with_dm(g,q,&mut state,&PriorityResponse::PriorityAction(LegalAction::PassPriority),dm).map_err(|e|e.to_string())?;
        }
    }
    Err("resolution window budget".into())
}
fn run(defs: &HashMap<String,CardDefinition>, name: &str, attacking: u8, power: i32, combat: bool, defender: u8) -> Result<(Value,Value),String> {
    use ironsmith::combat_state::{AttackTarget,CombatState};
    use ironsmith::decision::{AttackerDeclaration,BlockerDeclaration};
    use ironsmith::game_loop::{apply_attacker_declarations_with_dm,apply_blocker_declarations,try_execute_combat_damage_step,queue_combat_damage_triggers};
    let mut g=setup(3,0);
    let mut dm=Choices{object:None,player:PlayerId(0),trace:vec![]};
    let paid=cast(&mut g,&defs[name],&mut dm)?;
    if !paid["resolution_error"].is_null(){return Err(format!("source cast failed:{paid}"));}
    let source=find(&g,name)?;
    let mut evidence=json!({"source_cast":paid,"actual_combat":combat,"attacking_player":attacking,"defending_player":defender,"power":power});
    if name=="Contested Game Ball" {
        let library=CardDefinitionBuilder::new(CardId::new(),"Game Ball draw witness").card_types(vec![CardType::Instant]).build();
        g.create_object_from_definition(&library,PlayerId(0),Zone::Library);
        let action=compute_legal_actions(&g,PlayerId(0)).expect("fixture has complete replacement state").into_iter().find(|a|matches!(a,LegalAction::ActivateAbility{source:id,..}if *id==source)).ok_or("Game Ball activation unavailable")?;
        let mana=g.player(PlayerId(0)).unwrap().mana_pool.total();
        let mut q=TriggerQueue::new();let mut state=PriorityLoopState::new(g.players_in_game());
        let mut progress=apply_priority_response_with_dm(&mut g,&mut q,&mut state,&PriorityResponse::PriorityAction(action),&mut dm).map_err(|e|e.to_string())?;
        for _ in 0..24 {
            if state.pending_activation.is_none() && !g.stack.is_empty() {break;}
            let GameProgress::NeedsDecisionCtx(ctx)=progress else {return Err(format!("Game Ball announcement stalled:{progress:?}"));};
            if matches!(ctx,DecisionContext::Priority(_)){return Err("Game Ball announcement returned priority without stack".into());}
            progress=apply_decision_context_with_dm(&mut g,&mut q,&mut state,&ctx,&mut dm).map_err(|e|e.to_string())?;
        }
        if g.stack.is_empty() || mana-g.player(PlayerId(0)).unwrap().mana_pool.total()!=2 {return Err("Game Ball activation did not reach stack with two mana paid".into());}
        finish(&mut g,&mut q,&mut dm)?;
        if !g.is_tapped(source) || g.player(PlayerId(0)).unwrap().hand.len()!=1 {return Err("Game Ball paid draw activation not completed".into());}
        evidence["source_activation"]=json!({"mana_paid":mana-g.player(PlayerId(0)).unwrap().mana_pool.total(),"tapped":g.is_tapped(source),"hand":g.player(PlayerId(0)).unwrap().hand.len()});
    }
    let error=if combat {
        let creature=CardDefinitionBuilder::new(CardId::new(),"Attacking-player witness").card_types(vec![CardType::Creature]).power_toughness(ironsmith::PowerToughness::fixed(power,6)).build();
        let attacker=g.create_object_from_definition(&creature,PlayerId(attacking),Zone::Battlefield);
        while g.turn.active_player!=PlayerId(attacking) {g.next_turn();}
        g.remove_summoning_sickness(attacker);
        g.turn.phase=ironsmith::Phase::Combat;g.turn.step=Some(ironsmith::Step::DeclareAttackers);
        let mut cs=CombatState::default();let mut q=TriggerQueue::new();
        apply_attacker_declarations_with_dm(&mut g,&mut cs,&mut q,&[AttackerDeclaration{creature:attacker,target:AttackTarget::Player(PlayerId(defender))}],&mut dm).map_err(|e|e.to_string())?;
        g.combat=Some(cs);
        finish(&mut g,&mut q,&mut dm)?;
        g.turn.step=Some(ironsmith::Step::DeclareBlockers);
        let mut cs=g.combat.take().unwrap();
        let blocks=if name=="Souls of the Faultless" {vec![BlockerDeclaration{blocker:source,blocking:attacker}]} else {vec![]};
        apply_blocker_declarations(&mut g,&mut cs,&mut q,&blocks,PlayerId(defender)).map_err(|e|e.to_string())?;
        g.turn.step=Some(ironsmith::Step::CombatDamage);
        let events=try_execute_combat_damage_step(&mut g,&cs,false).map_err(|e|e.to_string())?;
        g.combat=Some(cs);
        evidence["combat_events"]=json!(format!("{events:?}"));
        queue_combat_damage_triggers(&mut g,&events,&mut q);
        finish(&mut g,&mut q,&mut dm).err()
    }else{
        dm.object=if name=="Souls of the Faultless" {Some(source)} else {None};
        let spell=cast(&mut g,&defs["Shock"],&mut dm)?;
        let error=spell["resolution_error"].as_str().map(str::to_string);evidence["producer_spell"]=spell;error
    };
    evidence["choices"]=json!(dm.trace);
    let actual=json!({"resolution_error":error,"life":g.players.iter().map(|p|p.life).collect::<Vec<_>>(),"source_on_battlefield":g.object(source).is_some_and(|o|o.zone==Zone::Battlefield),"source_controller":g.controller_of_id(source).map(|p|p.0),"source_tapped":g.object(source).map(|_|g.is_tapped(source))});
    Ok((actual,evidence))
}
fn hash(p:&std::path::Path)->String{Sha256::digest(std::fs::read(p).unwrap()).iter().map(|b|format!("{b:02x}")).collect()}
#[test]
#[ignore="audit reporter, not a whole-card correctness test"]
fn report_attacking_player_context(){
    let input=std::path::PathBuf::from(std::env::var("AUDIT_RUNTIME_INVENTORY").unwrap());
    let payloads:Value=serde_json::from_slice(&std::fs::read(&input).unwrap()).unwrap();
    let mut defs=HashMap::new();let mut artifacts=vec![];
    for p in payloads["cards"].as_array().unwrap(){let n=p["name"].as_str().unwrap();if !["Contested Game Ball","Souls of the Faultless","Shock"].contains(&n){continue;}
        let(a,d)=ironsmith_registry::compile_builder_to_artifact(ironsmith_compiler::CardDefinitionBuilder::new(CardId::new(),n),p["parse_input"].as_str().unwrap(),false).unwrap();artifacts.push(json!({"card":n,"checksum":a.payload_checksum}));defs.insert(n.to_string(),d);
    }
    let mut rows=vec![];
    for name in ["Contested Game Ball","Souls of the Faultless"] {
        for (attacking,power,combat,defender) in [(1,0,true,0),(1,1,true,0),(1,3,true,0),(1,4,true,0),(2,2,true,0),(1,2,false,0)].into_iter().chain(if name=="Contested Game Ball" {vec![(1,2,true,2)]}else{vec![]}) {
            let mut life=vec![20,20,20];let survived=!(name=="Souls of the Faultless"&&combat&&power>=4);
            let transferred=name=="Contested Game Ball"&&combat&&power>0&&defender==0;
            if combat {if name=="Souls of the Faultless"{life[0]+=power;life[attacking as usize]-=power;}else{life[defender as usize]-=power;}} else if name=="Contested Game Ball"{life[0]-=2;}
            let expected=json!({"resolution_error":null,"life":life,"source_on_battlefield":survived,"source_controller":if survived {Some(if transferred {attacking}else{0})}else{None},"source_tapped":if survived {Some(name=="Contested Game Ball"&&!transferred)}else{None}});
            let(status,actual,evidence)=match run(&defs,name,attacking,power,combat,defender){Ok((a,e))=>(if a==expected{"expected_result_observed"}else if !a["resolution_error"].is_null(){"resolution_failed"}else{"semantic_mismatch"},a,e),Err(e)=>("fixture_or_producer_error",json!({"error":e}),Value::Null)};
            rows.push(json!({"card":name,"scenario":{"attacker":attacking,"power":power,"combat":combat,"defender":defender},"expected":expected,"actual":actual,"status":status,"fixture_evidence":evidence}));
        }
    }
    let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");let binary=std::env::current_exe().unwrap();
    let report=json!({"scope":"Strict canonical paid sources, actual Game Ball paid draw/tap activation, next-turn attacker declarations, legal blocking and normal combat damage with complete CombatState, normal priority resolution; noncombat/zero/third-player controls.","rows":rows,"artifacts":artifacts,"provenance":{"inventory_sha256":hash(&input),"binary":binary,"binary_sha256":hash(&binary),"source_sha256":hash(&root.join("crates/ironsmith-tools/tests/runtime_attacking_player_reproductions.rs")),"runtime_stack_bytes":67108864}});
    std::fs::write(root.join("reports/runtime-audit/attacking-player-context-reproductions.json"),serde_json::to_string_pretty(&report).unwrap()).unwrap();println!("{}",serde_json::to_string_pretty(&report).unwrap());
}
