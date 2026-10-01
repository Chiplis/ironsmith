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



const NAMES:[&str;10]=["Daze","Ensnare","Gush","Thwart","Tidal Bore","Fieldmist Borderpost","Firewild Borderpost","Mistvein Borderpost","Veinfire Borderpost","Wildfield Borderpost"];
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
fn run(defs:&HashMap<String,CardDefinition>,name:&str,n:usize,state:&str)->Result<(Value,Value,Value),String>{
 let mut g=setup(2,0);let mut dm=Choices{targets:vec![],names:vec![],x:0,allow_optional:true,choose_untap:false,trace:vec![]};let mut producers=vec![];
 for p in 0..2 {for _ in 0..8{g.create_object_from_definition(&defs["Plains"],PlayerId(p),Zone::Library);}}
 let border=name.ends_with("Borderpost");let required=match name{"Ensnare"|"Gush"=>2,"Thwart"=>3,_=>1};
 let land=if state=="wrong_land"{if border{"Azorius Guildgate"}else{"Plains"}}else{"Island"};let mut resources=vec![];
 for _ in 0..n{let id=play_land(&mut g,&defs[land],&mut dm)?;resources.push(id);producers.push(json!({"actual_land_play":land,"id":id.0}));next_main(&mut g);next_main(&mut g);}
 let creature=if name=="Tidal Bore"||name=="Ensnare"{producers.push(paid(&mut g,defs,"Grizzly Bears",&mut dm,2)?);Some(find(&g,"Grizzly Bears",Zone::Battlefield)?)}else{None};
 if name=="Tidal Bore"{dm.targets=vec![Target::Object(creature.unwrap())];producers.push(paid(&mut g,defs,"Twiddle",&mut dm,1)?);if !g.is_tapped(creature.unwrap()){return Err("Twiddle target tap failed".into());}dm.choose_untap=true;}
 let counterspell=name=="Daze"||name=="Thwart";let mut enemy_spell=None;
 if counterspell{
  next_main(&mut g);dm.targets.clear();let (mut q,e)=announce(&mut g,&defs["Divination"],1,&mut dm)?;if e["mana_paid"]!=3{return Err("actual opponent spell payment failed".into());}producers.push(e);
  enemy_spell=Some(g.stack.last().unwrap().object_id);let mut st=PriorityLoopState::new(g.players_in_game());apply_priority_response_with_dm(&mut g,&mut q,&mut st,&PriorityResponse::PriorityAction(LegalAction::PassPriority),&mut dm).map_err(|e|e.to_string())?;
  if g.turn.priority_player!=Some(PlayerId(0)){return Err("opponent did not pass priority".into());}
  dm.targets=vec![Target::Object(enemy_spell.unwrap())];dm.allow_optional=state=="tax_paid";
 }
 let source=g.create_object_from_definition(&defs[name],PlayerId(0),Zone::Hand);let stable=g.object(source).unwrap().stable_id;
 let method=if state=="normal"{ironsmith::alternative_cast::CastingMethod::Normal}else{ironsmith::alternative_cast::CastingMethod::Alternative(0)};
 let actions=compute_legal_actions(&g,PlayerId(0)).expect("fixture has complete replacement state");let offered=actions.iter().find(|a|matches!(a,LegalAction::CastSpell{spell_id,casting_method,..}if *spell_id==source&&*casting_method==method)).cloned();
 let valid=state=="normal"||(n>=required&&state!="wrong_land");
 let before=json!({"actions":format!("{actions:?}"),"method":format!("{method:?}"),"source_in_hand":true,"resources":resources.iter().map(|id|json!({"id":id.0,"name":g.object(*id).unwrap().name.to_string(),"zone":format!("{:?}",g.object(*id).unwrap().zone),"tapped":g.is_tapped(*id)})).collect::<Vec<_>>(),"opponent_stack_spell":enemy_spell.map(|id|id.0)});
 if !valid||offered.is_none(){return Ok((json!({"intended_cast_available":valid}),json!({"intended_cast_available":offered.is_some()}),json!({"producers":producers,"before_cast":before,"cast_dispatched":false,"choice_trace":dm.trace})));}
 dm.names=vec![land.into()];if name=="Tidal Bore"{dm.targets=vec![Target::Object(creature.unwrap())];}
 let library_before=g.player(PlayerId(0)).unwrap().library.len();let bob_library_before=g.player(PlayerId(1)).unwrap().library.len();let bob_mana_before=g.player(PlayerId(1)).unwrap().mana_pool.total();
 let payment=dispatch(&mut g,offered.unwrap(),&mut dm).unwrap_or_else(|e|json!({"error":e}));
 let object=g.objects_in_deterministic_order().into_iter().find(|o|o.stable_id==stable);
 let mut actual=json!({"error":payment["error"],"mana_paid":payment["announcement"]["mana_paid"],"returned_lands":g.player(PlayerId(0)).unwrap().hand.iter().filter(|id|g.object(**id).is_some_and(|o|o.name==land)).count(),"remaining_resources":resources.iter().filter(|id|g.battlefield.contains(id)).count(),"source_zone":object.map(|o|format!("{:?}",o.zone)),"stack_length":g.stack.len()});
 let num=if state=="normal"{0}else{required};let price=if state!="normal"{if border{1}else{0}}else{match name{"Gush"=>5,"Ensnare"|"Thwart"=>4,_=>if border{3}else{2}}};
 let mut expected=json!({"error":null,"mana_paid":price,"returned_lands":num,"remaining_resources":n-num,"source_zone":if border{"Battlefield"}else{"Graveyard"},"stack_length":0});
 if border {actual["source_tapped"]=json!(object.is_some_and(|o|g.is_tapped(o.id)));expected["source_tapped"]=json!(true);}
 if name=="Gush"{actual["cards_drawn"]=json!(library_before-g.player(PlayerId(0)).unwrap().library.len());expected["cards_drawn"]=json!(2);}
 if name=="Ensnare"||name=="Tidal Bore"{actual["creature_tapped"]=json!(g.is_tapped(creature.unwrap()));expected["creature_tapped"]=json!(name=="Ensnare");}
 if counterspell{actual["opponent_cards_drawn"]=json!(bob_library_before-g.player(PlayerId(1)).unwrap().library.len());expected["opponent_cards_drawn"]=json!(if state=="tax_paid"{2}else{0});actual["opponent_tax_paid"]=json!(bob_mana_before-g.player(PlayerId(1)).unwrap().mana_pool.total());expected["opponent_tax_paid"]=json!(if state=="tax_paid"{1}else{0});actual["opponent_spell_in_graveyard"]=json!(count(&g,"Divination",Zone::Graveyard)==1);expected["opponent_spell_in_graveyard"]=json!(true);}
 Ok((expected,actual,json!({"producers":producers,"before_cast":before,"payment":payment,"cast_dispatched":true,"choice_trace":dm.trace,"scope":"Actual land plays and strict exact casting method. Opponent Divination really paid and announced; caster passes priority before response. Exact land returns, announcement mana, spell zone and immediate draw/tap/counter or ETB results. Missing-resource advertisements are never dispatched."})))
}
fn hash(p: &std::path::Path) -> String {
    Sha256::digest(std::fs::read(p).unwrap())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
#[test]
#[ignore = "fixed multi-permanent tap cost audit"]
fn report_spell_return_land() {
    let input = std::path::PathBuf::from(std::env::var("AUDIT_RUNTIME_INVENTORY").unwrap());
    let payloads: Value = serde_json::from_slice(&std::fs::read(&input).unwrap()).unwrap();
    let mut defs = HashMap::new();
    let mut artifacts = vec![];
    for p in payloads["cards"].as_array().unwrap() {
        let n = p["name"].as_str().unwrap();
        if !NAMES.contains(&n)
            && !["Island","Plains","Azorius Guildgate","Grizzly Bears","Twiddle","Divination"]
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
        let required=match name{"Ensnare"|"Gush"=>2,"Thwart"=>3,_=>1};
        let mut cases=vec![(required-1,"ready"),(required,"ready"),(required+1,"ready"),(required,"wrong_land"),(0,"normal")];
        if name=="Daze" {cases.push((1,"tax_paid"));}
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
    let r = json!({"scope":"Ten land-return alternative spell costs: strict method selection, actual land play, paid opponent spell and real priority response, resource boundaries and exact immediate outcomes.","rows":rows,"artifacts":artifacts,"provenance":{"binary":binary,"binary_sha256":hash(&binary),"inventory_sha256":hash(&input),"source_sha256":hash(&root.join("crates/ironsmith-tools/tests/runtime_spell_return_land_reproductions.rs")),"runtime_stack_bytes":67108864}});
    std::fs::write(
        root.join("reports/runtime-audit/spell-return-land-reproductions.json"),
        serde_json::to_string_pretty(&r).unwrap(),
    )
    .unwrap();
    println!("{}", serde_json::to_string_pretty(&r).unwrap());
}
