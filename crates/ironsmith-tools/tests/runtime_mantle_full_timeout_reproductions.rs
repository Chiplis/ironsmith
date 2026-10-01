//! Small paid Mantle of the Ancients cases after imported attachment timeout.
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
        let grave:Vec<_>=c.requirements.iter().flat_map(|r|r.legal_targets.iter()).filter(|t|matches!(t,Target::Object(id)if g.object(*id).is_some_and(|o|o.zone==Zone::Graveyard))).copied().collect();
        if !grave.is_empty(){self.trace.push(json!({"graveyard_targets":format!("{grave:?}")}));return grave;}
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

#[test]
#[ignore="manual bounded paid Mantle outcome and termination check"]
fn report_mantle_full_timeout(){
 let case=std::env::var("MANTLE_CASE").unwrap_or("full-one".into());
 let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
 let paths=[std::env::current_exe().unwrap(),ironsmith_tools::default_cards_path(),root.join("crates/ironsmith-tools/tests/runtime_mantle_full_timeout_reproductions.rs")];let before:Vec<_>=paths.iter().map(|p|hash(p)).collect();
 let names=["Mantle of the Ancients","Konda's Banner","O-Naginata","Aether Tunnel","Reprobation","Indestructibility","Abundant Growth","Skylasher","Grim Guardian"].map(str::to_owned).to_vec();
 let payloads=ironsmith_tools::load_card_payloads_by_names(ironsmith_tools::default_cards_path().to_str().unwrap(),&names).unwrap();
 let mut defs=HashMap::new();let mut compilation=vec![];
 for p in payloads.into_values().flatten(){let b=ironsmith_compiler::CardDefinitionBuilder::new(CardId::new(),p.parse_name.as_deref().unwrap_or(&p.name));let(a,d)=ironsmith_registry::compile_builder_to_artifact(b,&p.parse_input,false).unwrap();compilation.push(json!({"card":p.name,"artifact_checksum":a.payload_checksum,"definition":a.payload.definition}));defs.insert(d.name().to_owned(),d);}
 let result=(||->Result<(Value,Value),String>{
  let mut g=setup_players(2);let target=g.create_object_from_definition(&defs["Skylasher"],alice(),Zone::Battlefield);let guardian=g.create_object_from_definition(&defs["Grim Guardian"],alice(),Zone::Battlefield);
  let attachments=["Konda's Banner","O-Naginata","Aether Tunnel","Reprobation","Indestructibility","Abundant Growth"];
  for name in attachments{g.create_object_from_definition(&defs[name],alice(),Zone::Graveyard);}
  let mut dm=dm(0,true);dm.target_name=Some("Skylasher".into());let start=std::time::Instant::now();let mut error=None;
  let rounds=if case=="full-two"{2}else{1};let mut per_cast=vec![];
  for n in 0..rounds{
   println!("stage: Mantle full cast {n} {case}");let(mut q,paid)=cast(&mut g,&defs["Mantle of the Ancients"],alice(),&mut dm)?;if paid!=5{return Err(format!("mana paid{paid}"));}
   println!("stage: Mantle full stack {n} {case}");error=resolve(&mut g,&mut q,&mut dm).err();println!("stage: Mantle full resolved {n} {case}");
   per_cast.push(json!({"round":n,"error":error,"power":g.calculated_power(target),"toughness":g.calculated_toughness(target),"aether_battlefield":count(&g,"Aether Tunnel",Zone::Battlefield)}));if error.is_some(){break;}
  }
  let attached_to=|id|g.battlefield.iter().filter(|obj|g.object(**obj).is_some_and(|o|o.attached_to==Some(ironsmith::object::AttachmentTarget::Object(id)))).count();
  let actual=json!({"resolution_error":error,"mantle_battlefield":count(&g,"Mantle of the Ancients",Zone::Battlefield),"banner_battlefield":count(&g,"Konda's Banner",Zone::Battlefield),"skylasher_attachments":attached_to(target),"guardian_attachments":attached_to(guardian),"aether_battlefield":count(&g,"Aether Tunnel",Zone::Battlefield),"growth_battlefield":count(&g,"Abundant Growth",Zone::Battlefield),"power":g.calculated_power(target),"toughness":g.calculated_toughness(target),"opponent_life":g.player(PlayerId::from_index(1)).unwrap().life,"stack":g.stack.len()});
  Ok((actual,json!({"choices":dm.trace,"per_cast":per_cast,"action_seconds":start.elapsed().as_secs_f64()})))
 })();
 let two=case=="full-two";let expected=json!({"resolution_error":null,"mantle_battlefield":if two{2}else{1},"banner_battlefield":1,"skylasher_attachments":if two{6}else{4},"guardian_attachments":0,"aether_battlefield":usize::from(two),"growth_battlefield":0,"power":if two{16}else{7},"toughness":if two{13}else{5},"opponent_life":if two{15}else{17},"stack":0});
 let(status,actual,diag)=match result{Ok((v,t))if v==expected=>("passed",v,t),Ok((v,t))if !v["resolution_error"].is_null()=>("confirmed_resolution_failure",v,t),Ok((v,t))=>("semantic_mismatch",v,t),Err(e)=>("execution_or_fixture_error",json!({"error":e}),Value::Null)};
 let after:Vec<_>=paths.iter().map(|p|hash(p)).collect();let report=json!({"scope":"Full imported Mantle attachment setup: real paid one/two casts on canonical Skylasher with Grim Guardian and all six actual graveyard Aura/Equipment definitions. Every graveyard target is selected legally, ordinary effect-driven attachment choice, priority and SBA. Exact oracle expectations reviewed from original imported scenario; failure requires further attribution review.","provenance":{"before":before,"after":after,"artifacts_unchanged":before==after},"compilation":compilation,"rows":[{"card":"Mantle of the Ancients","scenario":case,"status":status,"expected":expected,"actual":actual,"diagnostics":diag}]});
 let slug=case.to_lowercase().replace(' ',"-");std::fs::write(root.join(format!("reports/runtime-audit/mantle-{slug}.json")),serde_json::to_string_pretty(&report).unwrap()+"\n").unwrap();
}
