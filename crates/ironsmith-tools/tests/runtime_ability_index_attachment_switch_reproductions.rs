//! Paid conditional-index transitions caused by costs and current-turn counter history.
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
}
impl DecisionMaker for Choices {
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
        drain(g, q, dm)?;
        if g.stack.is_empty() {
            return Ok(());
        }
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
    for p in [PlayerId(0), PlayerId(1)] {
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
    for e in ironsmith::turn::execute_draw_step_with(g, dm).unwrap() {
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
        .find(|a| matches!(a,LegalAction::ActivateAbility{source:s,..}|LegalAction::ActivateManaAbility{source:s,..}if *s==source));
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
fn run(
    def: &CardDefinition,
    defs: &std::collections::HashMap<&str, CardDefinition>,
    mode: usize,
) -> Result<Value, String> {
    use ironsmith::object::AttachmentTarget;
    use ironsmith::static_abilities::StaticAbilityId as K;
    let mut g = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    g.set_random_seed(71757432704855);
    g.turn.turn_number = 3;
    g.turn.active_player = PlayerId(0);
    g.turn.priority_player = Some(PlayerId(0));
    g.turn.phase = ironsmith::Phase::FirstMain;
    g.turn.step = None;
    mana(&mut g);
    let fodder = CardDefinitionBuilder::new(CardId::new(), "Neutral library card")
        .card_types(vec![CardType::Artifact])
        .build();
    for player in [PlayerId(0), PlayerId(1)] {
        for _ in 0..12 {
            g.create_object_from_definition(&fodder, player, Zone::Library);
        }
    }
    let mut q = TriggerQueue::new();
    let mut dm = Choices {
        accept: false,
        target: None,
        trace: vec![],
        resources: vec![],
    };
    for name in ["Glory Seeker", "Grizzly Bears"] {
        if cast(&mut g, &defs[name], PlayerId(0), &mut q, &mut dm)? != 2 {
            return Err("Wrong paid creature cost".into());
        }
        resolve_all(&mut g, &mut q, &mut dm)?;
    }
    let human = find(&g, "Glory Seeker")?;
    let bear = find(&g, "Grizzly Bears")?;
    let source_paid;
    if def.name() == "Scrounged Scythe" {
        let mut front = defs["Harvest Hand"].clone();
        let mut back = def.clone();
        front.card.other_face = Some(back.card.id);
        front.card.other_face_name = Some(back.card.name.clone());
        front.card.linked_face_layout = ironsmith::card::LinkedFaceLayout::TransformLike;
        front.card.transforming_dfc = true;
        back.card.other_face = Some(front.card.id);
        back.card.other_face_name = Some(front.card.name.clone());
        back.card.linked_face_layout = ironsmith::card::LinkedFaceLayout::TransformLike;
        back.card.transforming_dfc = true;
        g.register_linked_face_definition(&front);
        g.register_linked_face_definition(&back);
        source_paid = cast(&mut g, &front, PlayerId(0), &mut q, &mut dm)?;
        if source_paid != 3 {
            return Err("Wrong Harvest Hand cost".into());
        }
        resolve_all(&mut g, &mut q, &mut dm)?;
        let front_id = find(&g, front.name())?;
        dm.target = Some(Target::Object(front_id));
        if cast(&mut g, &defs["Murder"], PlayerId(0), &mut q, &mut dm)? != 3 {
            return Err("Wrong Murder payment".into());
        }
        resolve_all(&mut g, &mut q, &mut dm)?;
    } else {
        source_paid = cast(&mut g, def, PlayerId(0), &mut q, &mut dm)?;
        if source_paid != def.card.mana_cost.as_ref().unwrap().mana_value() {
            return Err("Wrong source payment".into());
        }
        resolve_all(&mut g, &mut q, &mut dm)?;
    }
    let source = find(&g, def.name())?;
    let cost = if def.name() == "Bladehold War-Whip" {
        5
    } else {
        2
    };
    let activation_expected = |n: i64| json!({"offered":true,"announcement_error":null,"resolution_error":null,"mana_paid":n,"remaining_stack":0});
    let mut evidence = json!({"source_cast_paid":source_paid,"mode":mode,"canonical_face_metadata_reconstructed":def.name()=="Scrounged Scythe","face_link_scope":"For Scythe only, original strict front/back definitions are registered with canonical transform links; actual paid Harvest Hand + Murder produces this face. Frozen parity compares the original strict definitions before links."});
    if def.name() == "Bladehold War-Whip" {
        let rebel = match g.object(source).unwrap().attached_to {
            Some(AttachmentTarget::Object(id)) => id,
            _ => return Err("For Mirrodin failed to attach".into()),
        };
        if g.calculated_power(rebel) != Some(2)
            || g.calculated_toughness(rebel) != Some(2)
            || !g.object_has_static_ability_id(rebel, K::DoubleStrike)
        {
            return Err("For Mirrodin Rebel characteristics wrong".into());
        }
        evidence["actual_etb_rebel"] = json!(format!("{rebel:?}"));
        if mode == 0 {
            dm.target = Some(Target::Object(rebel));
            if cast(&mut g, &defs["Unsummon"], PlayerId(0), &mut q, &mut dm)? != 1 {
                return Err("Wrong Unsummon payment".into());
            }
            resolve_all(&mut g, &mut q, &mut dm)?;
            if g.object(source)
                .unwrap()
                .attached_to
                .is_some_and(|a| match a {
                    AttachmentTarget::Object(id) => {
                        g.object(id).is_some_and(|o| o.zone == Zone::Battlefield)
                    }
                    _ => true,
                })
            {
                return Err("Removal left a current attached recipient".into());
            }
        }
    } else if mode > 0 {
        dm.target = Some(Target::Object(if mode == 2 { bear } else { human }));
        let setup = activation(&mut g, &mut q, &mut dm, source)?;
        if setup != activation_expected(cost) {
            return Err(format!("Initial Equip failed:{setup}"));
        }
    }
    if def.name() == "Summoning Materia" {
        advance_turn(&mut g, &mut q, &mut dm)?;
        advance_turn(&mut g, &mut q, &mut dm)?;
    }
    let before_attached = g
        .object(source)
        .unwrap()
        .attached_to
        .is_some_and(|a| match a {
            AttachmentTarget::Object(id) => {
                g.object(id).is_some_and(|o| o.zone == Zone::Battlefield)
            }
            _ => true,
        });
    let before_human_menace = g.object_has_static_ability_id(human, K::Menace);
    let target = if def.name() == "Scrounged Scythe" && mode == 2 {
        human
    } else {
        bear
    };
    dm.target = Some(Target::Object(target));
    let outcome = activation(&mut g, &mut q, &mut dm, source)?;
    let mut expected = json!({"activation":activation_expected(cost),"attached_before":mode>0,"attached_to_requested_target":true,"power":if def.name()=="Summoning Materia"{4}else if def.name()=="Scrounged Scythe"{3}else{2},"toughness":if def.name()=="Summoning Materia"{4}else if def.name()=="Scrounged Scythe"{3}else{2}});
    let mut actual = json!({"activation":outcome,"attached_before":before_attached,"attached_to_requested_target":g.object(source).unwrap().attached_to==Some(AttachmentTarget::Object(target)),"power":g.calculated_power(target),"toughness":g.calculated_toughness(target)});
    if def.name() == "Scrounged Scythe" {
        expected["before_human_menace"] = json!(mode == 1);
        actual["before_human_menace"] = json!(before_human_menace);
        expected["target_menace"] = json!(mode == 2);
        actual["target_menace"] = json!(g.object_has_static_ability_id(target, K::Menace));
        expected["other_menace"] = json!(false);
        actual["other_menace"] = json!(
            g.object_has_static_ability_id(if target == human { bear } else { human }, K::Menace)
        );
    } else if def.name() == "Summoning Materia" {
        expected["vigilance"] = json!(true);
        actual["vigilance"] = json!(g.object_has_static_ability_id(target, K::Vigilance));
        let a = compute_legal_actions(&g, PlayerId(0)).expect("fixture has complete replacement state")
            .into_iter()
            .find(|a| matches!(a,LegalAction::ActivateManaAbility{source:s,..}if *s==target));
        dm.trace
            .push(json!({"stage":"granted_mana_action","action":format!("{a:?}")}));
        let before = g.player(PlayerId(0)).unwrap().mana_pool.total();
        let error = if let Some(a) = a.clone() {
            announce_mana(&mut g, &mut q, &mut dm, a).err()
        } else {
            None
        };
        expected["granted_mana"] = json!({"offered":true,"error":null,"added":1,"tapped":true});
        actual["granted_mana"] = json!({"offered":a.is_some(),"error":error,"added":g.player(PlayerId(0)).unwrap().mana_pool.total() as i64-before as i64,"tapped":g.is_tapped(target)});
    } else {
        expected["double_strike"] = json!(true);
        actual["double_strike"] = json!(g.object_has_static_ability_id(target, K::DoubleStrike));
        if mode == 2 {
            dm.target = Some(Target::Object(source));
            if cast(&mut g, &defs["Disenchant"], PlayerId(0), &mut q, &mut dm)? != 2 {
                return Err("Wrong Disenchant payment".into());
            }
            resolve_all(&mut g, &mut q, &mut dm)?;
            if g.battlefield.contains(&source) {
                return Err("War-Whip removal control did not remove source".into());
            }
            expected["double_strike_after_source_removal"] = json!(false);
            actual["double_strike_after_source_removal"] =
                json!(g.object_has_static_ability_id(target, K::DoubleStrike));
        }
        dm.target = None;
        if cast(&mut g, &defs["Bonesplitter"], PlayerId(0), &mut q, &mut dm)? != 1 {
            return Err("Wrong Bonesplitter payment".into());
        }
        resolve_all(&mut g, &mut q, &mut dm)?;
        let other = find(&g, "Bonesplitter")?;
        dm.target = Some(Target::Object(human));
        let discounted = activation(&mut g, &mut q, &mut dm, other)?;
        expected["other_equipment_activation"] = activation_expected(if mode == 2 { 1 } else { 0 });
        actual["other_equipment_activation"] = discounted;
        expected["other_attached"] = json!(true);
        actual["other_attached"] =
            json!(g.object(other).unwrap().attached_to == Some(AttachmentTarget::Object(human)));
        expected["other_equipped_power"] = json!(4);
        actual["other_equipped_power"] = json!(g.calculated_power(human));
    }
    Ok(
        json!({"expected":expected,"actual":actual,"state_evidence":evidence,"execution_trace":dm.trace}),
    )
}
#[test]
#[ignore = "manual scoped conditional index state transition report"]
fn report_attachment_switch_activations() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let inventory =
        root.join("reports/runtime-audit/corpus/267a16aff3b321196397d0b4/inventory.json");
    let source = root.join(
        "crates/ironsmith-tools/tests/runtime_ability_index_attachment_switch_reproductions.rs",
    );
    let binary = std::env::current_exe().unwrap();
    let paths = [&inventory, &source, &binary];
    let before: Vec<_> = paths
        .iter()
        .map(|p| json!({"path":p,"sha256":hash(p)}))
        .collect();
    let inv: Value = serde_json::from_slice(&std::fs::read(&inventory).unwrap()).unwrap();
    let names = ["Summoning Materia", "Bladehold War-Whip"];
    let mut compile = vec![];
    let mut defs = std::collections::HashMap::new();
    for name in names.into_iter().chain([
        "Glory Seeker",
        "Grizzly Bears",
        "Unsummon",
        "Bonesplitter",
        "Harvest Hand",
        "Murder",
        "Disenchant",
    ]) {
        let p = inv["cards"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["name"] == name)
            .unwrap();
        let (artifact, def) = ironsmith_registry::compile_builder_to_artifact(
            ironsmith_compiler::CardDefinitionBuilder::new(
                CardId::new(),
                p["parse_name"].as_str().unwrap_or(name),
            ),
            p["parse_input"].as_str().unwrap(),
            false,
        )
        .unwrap();
        compile.push(json!({"card":name,"artifact_checksum":artifact.payload_checksum,"definition":artifact.payload.definition}));
        defs.insert(name, def);
    }
    let mut rows = vec![];
    for name in names {
        if std::env::var("AUDIT_STATIC_CARD").is_ok_and(|n| n != name) {
            continue;
        }
        let count = match name {
            "Bladehold War-Whip" => 3,
            _ => 2,
        };
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
                    "fixture_or_execution_error",
                    json!({"expected":null,"actual":{"error":e}}),
                ),
            };
            rows.push(json!({"card":name,"scenario":{"mode":mode},"status":status,"expected":out["expected"],"actual":out["actual"],"state_evidence":out["state_evidence"],"execution_trace":out["execution_trace"],"artifact_checksum":compile.iter().find(|c|c["card"]==name).unwrap()["artifact_checksum"],"scope":"Paid canonical source and real printed producer actions, actual turn/untap transitions as required, resource-complete advertised activation. Exact condition characteristics, costs and outcome checked; linked-face and unrelated abilities remain outside scope."}));
        }
    }
    let after: Vec<_> = paths
        .iter()
        .map(|p| json!({"path":p,"sha256":hash(p)}))
        .collect();
    let report = json!({"rows":rows,"compilation":compile,"provenance":{"before":before,"after":after,"artifacts_unchanged":before==after,"strict_artifact":true,"unique_card_ids":true,"seed":71757432704855_u64}});
    let out = std::env::var("AUDIT_STATIC_OUTPUT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            root.join("reports/runtime-audit/ability-index-attachment-switch-reproductions.json")
        });
    std::fs::write(out, serde_json::to_string_pretty(&report).unwrap()).unwrap();
    println!("wrote{}cases", report["rows"].as_array().unwrap().len());
}
