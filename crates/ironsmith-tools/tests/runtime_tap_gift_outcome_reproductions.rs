//! Strict paid-action audit of tap-payment and permanent-gift trigger outcomes.
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
        let mut chosen=vec![];
        for r in &c.requirements {
            let target=r.legal_targets.iter().find(|t|match t {
                Target::Object(id)=>self.target_name.as_ref().is_some_and(|name|g.object(*id).is_some_and(|o|o.name==*name)),
                Target::Player(p)=>p.index() as usize==self.recipient,
            }).copied().or_else(||r.legal_targets.first().copied());
            if let Some(t)=target {chosen.push(t);}
        }
        self.trace.push(json!({"targets":format!("{:?}",c.context),"selected":format!("{chosen:?}")}));
        chosen
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
fn attack(g:&mut GameState,source:ObjectId,dm:&mut ProbeDm)->Result<Option<String>,String>{
 use ironsmith::combat_state::{CombatState,AttackTarget};
 use ironsmith::decision::AttackerDeclaration;
 g.remove_summoning_sickness(source);g.turn.phase=ironsmith::Phase::Combat;g.turn.step=Some(ironsmith::Step::DeclareAttackers);
 let mut combat=CombatState::default();let mut q=TriggerQueue::new();
 ironsmith::game_loop::apply_attacker_declarations_with_dm(g,&mut combat,&mut q,&[AttackerDeclaration{creature:source,target:AttackTarget::Player(PlayerId::from_index(1))}],dm).map_err(|e|e.to_string())?;
 g.combat=Some(combat);Ok(resolve(g,&mut q,dm).err())
}
fn tap_cost(defs:&HashMap<String,CardDefinition>,name:&str,resource:u8,accept:bool)->Result<(Value,Value),String>{
 let mut g=setup_players(2);let source=established(&mut g,&defs[name],alice(),if name=="Civil Servant"{2}else{3})?;
 let mut b=CardDefinitionBuilder::new(CardId::new(),"Tap cost witness").card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(2,2));
 b=b.subtypes(vec![if name=="Civil Servant"{ironsmith::Subtype::Citizen}else{ironsmith::Subtype::Merfolk}]);
 let resource_id=if resource>0{Some(g.create_object_from_definition(&b.build(),alice(),Zone::Battlefield))}else{None};
 if resource==2 {g.tap(resource_id.unwrap());}
 let grave=witness("Reanimate witness",CardType::Creature,2,false);g.create_object_from_definition(&grave,alice(),Zone::Graveyard);
 let mut dm=dm(0,accept);dm.target_name=Some(grave.name().to_owned());
 let error=attack(&mut g,source,&mut dm)?;
 let actual=json!({"resolution_error":error,"resource_tapped":resource_id.is_some_and(|id|g.is_tapped(id)),"source_power":g.calculated_power(source),"source_lifelink":g.current_has_static_ability_id(source,ironsmith::static_abilities::StaticAbilityId::Lifelink),"returned_creatures":count(&g,grave.name(),Zone::Battlefield),"graveyard_creatures":count(&g,grave.name(),Zone::Graveyard)});
 Ok((actual,json!({"choices":dm.trace,"source_paid":if name=="Civil Servant"{2}else{3},"summoning_sickness":"removed to represent control since the turn began before legal attack declaration"})))
}
fn gift(defs:&HashMap<String,CardDefinition>,players:usize,recipient:usize,accept:bool,rounds:usize)->Result<(Value,Value),String>{
 let mut g=setup_players(players);established(&mut g,&defs["Iroh, Tea Master"],alice(),3)?;
 if count(&g,"Food",Zone::Battlefield)!=1{return Err("actual Iroh ETB did not create Food".into());}
 let mut dm=dm(0,accept);dm.recipient=recipient;
 let mut error=None;
 for round in 0..rounds{dm.target_name=Some(if round==0||!accept{"Food"}else{"Ally"}.into());error=phase(&mut g,true,&mut dm)?;if error.is_some(){break;}}
 let allies:Vec<_>=g.objects_in_deterministic_order().into_iter().filter(|o|o.zone==Zone::Battlefield&&o.name=="Ally").collect();
 let actual=json!({"resolution_error":error,"gifted_permanents":g.battlefield.iter().filter(|id|g.object(**id).is_some_and(|o|o.owner==alice())&&g.controller_of_id(**id)==Some(PlayerId::from_index(recipient as u8))).count(),"alice_ally_count":allies.iter().filter(|o|g.controller_of_id(o.id)==Some(alice())).count(),"total_ally_count":allies.len(),"alice_ally_counters":allies.iter().filter(|o|g.controller_of_id(o.id)==Some(alice())).map(|o|o.counters.get(&CounterType::PlusOnePlusOne).copied().unwrap_or(0)).sum::<u32>(),"food_controller":g.objects_in_deterministic_order().into_iter().find(|o|o.zone==Zone::Battlefield&&o.name=="Food").and_then(|o|g.controller_of_id(o.id)).map(|p|p.index())});
 Ok((actual,json!({"choices":dm.trace,"source_paid":3,"gift_resources":"First actual ETB Food, then first reflexively created Ally if present"})))
}
fn record(rows:&mut Vec<Value>,name:&str,scenario:String,expected:Value,result:Result<(Value,Value),String>,unavailable:bool){
 let(status,actual,diag)=match result{Ok((v,t))if v==expected=>("passed",v,t),Ok((v,t))if !v["resolution_error"].is_null()=>("confirmed_resolution_failure",v,t),Ok((v,t))=>("semantic_mismatch",v,t),Err(e)=>("execution_or_fixture_error",json!({"error":e}),Value::Null)};
 rows.push(json!({"card":name,"scenario":scenario,"status":if unavailable{"unavailable_optional_cost_probe"}else{status},"expected":expected,"actual":actual,"diagnostics":diag}));
}
#[test]
#[ignore="manual strict paid-cast tap/gift trigger audit; inspect report row statuses"]
fn report_tap_gift_outcome_execution(){
 let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
 let paths=[std::env::current_exe().unwrap(),ironsmith_tools::default_cards_path(),root.join("crates/ironsmith-tools/tests/runtime_tap_gift_outcome_reproductions.rs")];
 let before:Vec<_>=paths.iter().map(|p|hash(p)).collect();
 let names=["Civil Servant","Meanders Guide","Iroh, Tea Master"].map(str::to_owned).to_vec();
 let payloads=ironsmith_tools::load_card_payloads_by_names(ironsmith_tools::default_cards_path().to_str().unwrap(),&names).unwrap();
 let mut defs=HashMap::new();let mut compilation=vec![];
 for p in payloads.into_values().flatten(){let b=ironsmith_compiler::CardDefinitionBuilder::new(CardId::new(),p.parse_name.as_deref().unwrap_or(&p.name));let(a,d)=ironsmith_registry::compile_builder_to_artifact(b,&p.parse_input,false).unwrap();compilation.push(json!({"card":p.name,"artifact_checksum":a.payload_checksum,"definition":a.payload.definition}));defs.insert(d.name().to_owned(),d);}
 let mut rows=vec![];
 for name in ["Civil Servant","Meanders Guide"] {for resource in 0..3{for accept in [false,true]{
 let paid=resource==1&&accept;let civil=name=="Civil Servant";
 let expected=json!({"resolution_error":null,"resource_tapped":resource==2||paid,"source_power":if civil{2+u32::from(paid)}else{3},"source_lifelink":civil&&paid,"returned_creatures":usize::from(!civil&&paid),"graveyard_creatures":usize::from(civil||!paid)});
 record(&mut rows,name,format!("real attack, tap resource={resource} (0 absent,1 untapped,2 tapped), accept={accept}"),expected,tap_cost(&defs,name,resource,accept),accept&&resource!=1);
 }}}
 for (players,recipient,accept,rounds) in [(2,1,false,1),(2,1,true,1),(2,1,false,2),(3,1,true,1),(3,2,true,1),(3,2,false,1)]{
 let expected=json!({"resolution_error":null,"gifted_permanents":if accept{rounds}else{0},"alice_ally_count":usize::from(accept),"total_ally_count":if accept{rounds}else{0},"alice_ally_counters":if accept{rounds}else{0},"food_controller":if accept{recipient}else{0}});
 record(&mut rows,"Iroh, Tea Master",format!("players={players}, recipient={recipient}, accept={accept}, combats={rounds}"),expected,gift(&defs,players,recipient,accept,rounds),false);
 }
 let after:Vec<_>=paths.iter().map(|p|hash(p)).collect();let report=json!({"scope":"Strict paid sources, production priority passes and explicit normal SBA checks. Real legal attack declaration or generated begin-combat event. Actual available/absent/already-tapped subtype resources, accepted and declined choices. Iroh gifts his real ETB Food and subsequent real Ally; explicit recipient in two/three-player games. No injected outcome binding.","provenance":{"before":before,"after":after,"artifacts_unchanged":before==after},"compilation":compilation,"rows":rows});
 std::fs::write(root.join("reports/runtime-audit/tap-gift-outcome-execution.json"),serde_json::to_string_pretty(&report).unwrap()+"\n").unwrap();println!("wrote {} cases",report["rows"].as_array().unwrap().len());
}
