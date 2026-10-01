#[path = "support/canonical_linked_fixture.rs"]
mod canonical_linked_fixture;
// Paid canonical catalog-linked transitions and subsequent conditional abilities.
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
        ironsmith::game_loop::advance_priority_with_dm(g,q,dm).map_err(|e|e.to_string())?;
        drain(g, q, dm)?;
        if g.stack.is_empty() {
            return Ok(());
        }
        dm.trace.push(json!({"stage":"stack_before_resolution","entries":format!("{:?}",g.stack)}));
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
    for p in (0..g.players_in_game()).map(|n|PlayerId(n as u8)) {
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
fn play_land(g:&mut GameState,def:&CardDefinition,q:&mut TriggerQueue,dm:&mut Choices)->Result<ObjectId,String>{
 let id=g.create_object_from_definition(def,PlayerId(0),Zone::Hand);g.turn.priority_player=Some(PlayerId(0));let action=compute_legal_actions(g,PlayerId(0)).expect("fixture has complete replacement state").into_iter().find(|a|matches!(a,LegalAction::PlayLand{land_id}if *land_id==id)).ok_or("land play missing")?;let mut state=PriorityLoopState::new(g.players_in_game());apply_priority_response_with_dm(g,q,&mut state,&PriorityResponse::PriorityAction(action),dm).map_err(|e|e.to_string())?;resolve_all(g,q,dm)?;find(g,def.name())
}
fn run(def:&CardDefinition,defs:&std::collections::HashMap<&str,CardDefinition>,mode:usize)->Result<Value,String>{
 use ironsmith::static_abilities::StaticAbilityId as K;
 let (group,front_name,back_name,front_cost)=match def.name(){"Scrounged Scythe"=>("Harvest Hand // Scrounged Scythe","Harvest Hand","Scrounged Scythe",3),"Watertight Gondola"=>("Waterlogged Hulk // Watertight Gondola","Waterlogged Hulk","Watertight Gondola",1),_=>("Dion, Bahamut's Dominant // Bahamut, Warden of Light","Dion, Bahamut's Dominant","Bahamut, Warden of Light",4)};
 let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");let family=canonical_linked_fixture::LinkedFamily::from_catalog(&root,group).map_err(|e|format!("catalog linked compile failed: {e}"))?;
 let mut g=GameState::new(vec!["Alice".into(),"Bob".into()],20);family.register(&mut g);g.set_random_seed(71757432704855);g.turn.turn_number=3;g.turn.active_player=PlayerId(0);g.turn.priority_player=Some(PlayerId(0));g.turn.phase=ironsmith::Phase::FirstMain;g.turn.step=None;mana(&mut g);
 let filler=CardDefinitionBuilder::new(CardId::new(),"Neutral library artifact").card_types(vec![CardType::Artifact]).build();for p in [PlayerId(0),PlayerId(1)]{for _ in 0..12{g.create_object_from_definition(&filler,p,Zone::Library);}}
 let mut q=TriggerQueue::new();let mut dm=Choices{accept:true,target:None,trace:vec![],resources:vec![],x:0,activation_hint:None};
 let paid=cast(&mut g,&family.definitions[front_name].0,PlayerId(0),&mut q,&mut dm)?;if paid!=front_cost{return Err("linked source paid cost mismatch".into());}resolve_all(&mut g,&mut q,&mut dm)?;let source=find(&g,front_name)?;
 let good_activation=|cost:i64|json!({"offered":true,"announcement_error":null,"resolution_error":null,"mana_paid":cost,"remaining_stack":0});
 let mut evidence=json!({"source_paid":paid,"catalog_linkage":family.metadata_record,"linked_artifacts":family.linked_artifacts,"prelink_artifacts":family.unlinked_artifacts});
 let mut transition=None;let mut front_own=None;let mut front_opponent=None;
 if front_name=="Harvest Hand" {dm.target=Some(Target::Object(source));if cast(&mut g,&defs["Murder"],PlayerId(0),&mut q,&mut dm)?!=3{return Err("Murder cost".into());}resolve_all(&mut g,&mut q,&mut dm)?;}
 else if front_name=="Waterlogged Hulk"{let island=play_land(&mut g,&defs["Island"],&mut q,&mut dm)?;dm.resources=vec![island];dm.activation_hint=Some("Exile");transition=Some(activation(&mut g,&mut q,&mut dm,source)?);}
 else {front_own=Some(g.object_has_static_ability_id(source,K::Flying));advance_turn(&mut g,&mut q,&mut dm)?;front_opponent=Some(g.object_has_static_ability_id(source,K::Flying));advance_turn(&mut g,&mut q,&mut dm)?;dm.activation_hint=Some("Exile");transition=Some(activation(&mut g,&mut q,&mut dm,source)?);}
 let back=find(&g,back_name).ok();
 evidence["transition_activation"]=json!(transition);evidence["back_battlefield_id"]=json!(back.map(|id|format!("{id:?}")));evidence["source_graveyard_names"]=json!(g.player(PlayerId(0)).unwrap().graveyard.iter().map(|id|g.object(*id).unwrap().name.to_string()).collect::<Vec<_>>());evidence["source_exile_names"]=json!(g.exile.iter().map(|id|g.object(*id).unwrap().name.to_string()).collect::<Vec<_>>());
 if back.is_none(){return Ok(json!({"expected":{"back_entered":true},"actual":{"back_entered":false},"state_evidence":evidence,"execution_trace":dm.trace,"failure_stage":"front_transition","attributed_card":front_name}));}
 let back=back.unwrap();
 if front_name=="Harvest Hand"{let host_name=if mode==0{"Glory Seeker"}else{"Grizzly Bears"};if cast(&mut g,&defs[host_name],PlayerId(0),&mut q,&mut dm)?!=2{return Err("host cost".into());}resolve_all(&mut g,&mut q,&mut dm)?;let host=find(&g,host_name)?;dm.target=Some(Target::Object(host));dm.activation_hint=Some("Attach");let a=activation(&mut g,&mut q,&mut dm,back)?;let expected=json!({"activation":good_activation(2),"power":3,"toughness":3,"menace":mode==0,"attached":true});let actual=json!({"activation":a,"power":g.calculated_power(host),"toughness":g.calculated_toughness(host),"menace":g.object_has_static_ability_id(host,K::Menace),"attached":g.object(back).unwrap().attached_to==Some(ironsmith::object::AttachmentTarget::Object(host))});return Ok(json!({"expected":expected,"actual":actual,"state_evidence":evidence,"execution_trace":dm.trace}));}
 if front_name=="Waterlogged Hulk"{if transition!=Some(good_activation(4)){return Err(format!("craft announcement mismatch {transition:?}"));}if cast(&mut g,&defs["Grizzly Bears"],PlayerId(0),&mut q,&mut dm)?!=2{return Err("crew host cost".into());}resolve_all(&mut g,&mut q,&mut dm)?;let host=find(&g,"Grizzly Bears")?;for _ in 0..(if mode==0{0}else{8}){if cast(&mut g,&defs["Lotus Petal"],PlayerId(0),&mut q,&mut dm)?!=0{return Err("Petal cost".into());}resolve_all(&mut g,&mut q,&mut dm)?;let petal=find(&g,"Lotus Petal")?;dm.activation_hint=Some("AddMana");let a=activation(&mut g,&mut q,&mut dm,petal)?;if !a["offered"].as_bool().unwrap_or(false)||a["announcement_error"].is_string()||g.object(petal).is_some_and(|o|o.zone==Zone::Battlefield){return Err(format!("actual Petal sacrifice failed {a}"));}}
 let gy_permanents=g.player(PlayerId(0)).unwrap().graveyard.iter().filter(|id|g.object(**id).is_some_and(|o|o.card_types.iter().any(|t|matches!(t,CardType::Artifact|CardType::Creature|CardType::Enchantment|CardType::Land|CardType::Planeswalker|CardType::Battle)))).count();if gy_permanents!=if mode==0{0}else{8}{return Err(format!("permanent GY count {gy_permanents}"));}dm.resources=vec![host];dm.activation_hint=Some("Crew");let a=activation(&mut g,&mut q,&mut dm,back)?;let expected=json!({"activation":good_activation(0),"crewed":true,"crew_host_tapped":true,"power":4,"toughness":4,"vigilance":true,"can_be_blocked":mode==0});let actual=json!({"activation":a,"crewed":g.calculated_card_types(back).contains(&CardType::Creature),"crew_host_tapped":g.is_tapped(host),"power":g.calculated_power(back),"toughness":g.calculated_toughness(back),"vigilance":g.object_has_static_ability_id(back,K::Vigilance),"can_be_blocked":g.can_be_blocked(back)});evidence["actual_permanent_graveyard_count"]=json!(gy_permanents);return Ok(json!({"expected":expected,"actual":actual,"state_evidence":evidence,"execution_trace":dm.trace}));}
 let expected=json!({"activation":good_activation(6),"own_flying":true,"opponent_flying":false,"back_power":5,"back_toughness":5,"back_flying":true});let actual=json!({"activation":transition,"own_flying":front_own,"opponent_flying":front_opponent,"back_power":g.calculated_power(back),"back_toughness":g.calculated_toughness(back),"back_flying":g.object_has_static_ability_id(back,K::Flying)});Ok(json!({"expected":expected,"actual":actual,"state_evidence":evidence,"execution_trace":dm.trace}))
}
#[test]
#[ignore = "manual scoped conditional index state transition report"]
fn report_linked_transition_activations() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let inventory =
        root.join("reports/runtime-audit/corpus/267a16aff3b321196397d0b4/inventory.json");
    let source = root.join(
        "crates/ironsmith-tools/tests/runtime_ability_index_linked_transition_reproductions.rs",
    );
    let binary = std::env::current_exe().unwrap();
    let helper=root.join("crates/ironsmith-tools/tests/support/canonical_linked_fixture.rs");let catalog=root.join("cards.json");
    let paths = [&inventory, &source, &binary, &helper, &catalog];
    let before: Vec<_> = paths
        .iter()
        .map(|p| json!({"path":p,"sha256":hash(p)}))
        .collect();
    let inv: Value = serde_json::from_slice(&std::fs::read(&inventory).unwrap()).unwrap();
    let names = ["Scrounged Scythe", "Watertight Gondola", "Dion, Bahamut's Dominant"];
    let mut compile = vec![];
    let mut compilation_failures=vec![];
    let mut defs = std::collections::HashMap::new();
    for name in names.into_iter().chain(["Harvest Hand", "Waterlogged Hulk", "Bahamut, Warden of Light", "Murder", "Island", "Grizzly Bears", "Glory Seeker", "Lotus Petal"]) {
        let p = inv["cards"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["name"] == name)
            .unwrap();
        let result = ironsmith_registry::compile_builder_to_artifact(
            ironsmith_compiler::CardDefinitionBuilder::new(
                CardId::new(),
                p["parse_name"].as_str().unwrap_or(name),
            ),
            p["parse_input"].as_str().unwrap(),
            false,
        );
        let (artifact,def)=match result {Ok(x)=>x,Err(e)=>{compilation_failures.push(json!({"card":name,"error":e.to_string()}));continue;}};
        compile.push(json!({"card":name,"artifact_checksum":artifact.payload_checksum,"definition":artifact.payload.definition}));
        defs.insert(name, def);
    }
    let mut rows = vec![];
    for name in names {
        if std::env::var("AUDIT_STATIC_CARD").is_ok_and(|n| n != name) {
            continue;
        }
        let count = if name=="Dion, Bahamut's Dominant"{1}else{2};
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
                    if e.starts_with("catalog linked compile failed:"){"linked_face_compile_failed"}else{"fixture_or_execution_error"},
                    json!({"expected":null,"actual":{"error":e}}),
                ),
            };
            rows.push(json!({"card":name,"scenario":{"mode":mode},"status":status,"expected":out["expected"],"actual":out["actual"],"state_evidence":out["state_evidence"],"execution_trace":out["execution_trace"],"artifact_checksum":compile.iter().find(|c|c["card"]==name).unwrap()["artifact_checksum"],"failure_stage":out["failure_stage"],"attributed_card":out["attributed_card"],"scope":"Paid canonical source and real printed producer actions, actual turn/untap transitions as required, resource-complete advertised activation. Exact condition characteristics, costs and outcome checked; linked-face and unrelated abilities remain outside scope."}));
        }
    }
    let after: Vec<_> = paths
        .iter()
        .map(|p| json!({"path":p,"sha256":hash(p)}))
        .collect();
    let report = json!({"rows":rows,"compilation":compile,"compilation_failures":compilation_failures,"provenance":{"before":before,"after":after,"artifacts_unchanged":before==after,"strict_artifact":true,"unique_card_ids":true,"seed":71757432704855_u64}});
    let out = std::env::var("AUDIT_STATIC_OUTPUT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            root.join("reports/runtime-audit/ability-index-linked_transition-reproductions.json")
        });
    std::fs::write(out, serde_json::to_string_pretty(&report).unwrap()).unwrap();
    println!("wrote{}cases", report["rows"].as_array().unwrap().len());
}
