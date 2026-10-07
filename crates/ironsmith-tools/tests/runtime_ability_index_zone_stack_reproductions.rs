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
    resolve_only_one: bool,
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
            resolution_error = if dm.resolve_only_one {
                resolve_one(g, q, dm).err()
            } else {
                resolve_all(g, q, dm).err()
            };
        }
    }
    Ok(
        json!({"offered":a.is_some(),"announcement_error":error,"resolution_error":resolution_error,"mana_paid":before-g.player(PlayerId(0)).unwrap().mana_pool.total() as i64,"remaining_stack":g.stack.len()}),
    )
}
fn play_land(
    g: &mut GameState,
    def: &CardDefinition,
    q: &mut TriggerQueue,
    dm: &mut Choices,
) -> Result<ObjectId, String> {
    let id = g.create_object_from_definition(def, PlayerId(0), Zone::Hand);
    let action = compute_legal_actions(g, PlayerId(0)).expect("fixture has complete replacement state")
        .into_iter()
        .find(|a| matches!(a,LegalAction::PlayLand{land_id,..}if *land_id==id))
        .ok_or("land play missing")?;
    let mut state = PriorityLoopState::new(g.players_in_game());
    dm.trace
        .push(json!({"stage":"actual_land_play","action":format!("{action:?}")}));
    apply_priority_response_with_dm(
        g,
        q,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    )
    .map_err(|e| e.to_string())?;
    find(g, def.name())
}
fn run(
    def: &CardDefinition,
    defs: &std::collections::HashMap<&str, CardDefinition>,
    mode: usize,
) -> Result<Value, String> {
    use ironsmith::static_abilities::StaticAbilityId as K;
    let mut g = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    g.set_random_seed(71757432704855);
    g.turn.turn_number = 3;
    g.turn.active_player = PlayerId(0);
    g.turn.priority_player = Some(PlayerId(0));
    g.turn.phase = ironsmith::Phase::FirstMain;
    g.turn.step = None;
    mana(&mut g);
    let filler = CardDefinitionBuilder::new(CardId::new(), "Neutral library artifact")
        .card_types(vec![CardType::Artifact])
        .build();
    for player in [PlayerId(0), PlayerId(1)] {
        for _ in 0..12 {
            g.create_object_from_definition(&filler, player, Zone::Library);
        }
    }
    let mut q = TriggerQueue::new();
    let mut dm = Choices {
        accept: true,
        target: None,
        trace: vec![],
        resources: vec![],
        resolve_only_one: false,
    };
    if def.name() == "Colossal Rattlewurm" && mode == 1 {
        play_land(&mut g, &defs["Desert"], &mut q, &mut dm)?;
    }
    let source_paid = cast(&mut g, def, PlayerId(0), &mut q, &mut dm)?;
    if source_paid != def.card.mana_cost.as_ref().unwrap().mana_value() {
        return Err("source payment mismatch".into());
    }
    resolve_all(&mut g, &mut q, &mut dm)?;
    let source = find(&g, def.name())?;
    let mut evidence = json!({"mode":mode,"source_paid":source_paid});
    let activation_expected = |cost: i64| json!({"offered":true,"announcement_error":null,"resolution_error":null,"mana_paid":cost,"remaining_stack":0});
    let expected;
    let actual;
    match def.name() {
        "Colossal Rattlewurm" => {
            let initial_flash = g.object_has_static_ability_id(source, K::Flash);
            dm.target = Some(Target::Object(source));
            if cast(&mut g, &defs["Murder"], PlayerId(0), &mut q, &mut dm)? != 3 {
                return Err("Murder payment".into());
            }
            resolve_all(&mut g, &mut q, &mut dm)?;
            let gy = *g
                .player(PlayerId(0))
                .unwrap()
                .graveyard
                .iter()
                .find(|id| g.object(**id).is_some_and(|o| o.name == def.name()))
                .ok_or("real dead Wurm absent")?;
            let library_land =
                g.create_object_from_definition(&defs["Desert"], PlayerId(0), Zone::Library);
            dm.target = None;
            dm.resources = vec![library_land];
            let activation = activation(&mut g, &mut q, &mut dm, gy)?;
            let lands: Vec<_> = g
                .battlefield
                .iter()
                .copied()
                .filter(|id| g.object(*id).is_some_and(|o| o.name == "Desert"))
                .collect();
            expected = json!({"activation":activation_expected(2),"initial_flash":mode==1,"source_exiled":true,"controlled_deserts":mode+1,"tapped_deserts":1});
            actual = json!({"activation":activation,"initial_flash":initial_flash,"source_exiled":g.exile.iter().any(|id|g.object(*id).is_some_and(|o|o.name==def.name())),"controlled_deserts":lands.len(),"tapped_deserts":lands.iter().filter(|id|g.is_tapped(**id)).count()});
        }
        "Fugitive Droid" => {
            let recipient = if mode >= 4 {
                if cast(&mut g, &defs["Grizzly Bears"], PlayerId(0), &mut q, &mut dm)? != 2 {
                    return Err("target creature payment".into());
                }
                resolve_all(&mut g, &mut q, &mut dm)?;
                find(&g, "Grizzly Bears")?
            } else {
                if cast(&mut g, &defs["Ornithopter"], PlayerId(0), &mut q, &mut dm)? != 0 {
                    return Err("target artifact payment".into());
                }
                resolve_all(&mut g, &mut q, &mut dm)?;
                find(&g, "Ornithopter")?
            };
            if mode == 0 || mode == 3 {
                advance_turn(&mut g, &mut q, &mut dm)?;
                advance_turn(&mut g, &mut q, &mut dm)?;
            }
            let spell_actor = if matches!(mode, 2 | 3 | 4) {
                PlayerId(0)
            } else {
                PlayerId(1)
            };
            let before_block = g.can_be_blocked(source);
            g.turn.priority_player = Some(PlayerId(0));
            let mut state = PriorityLoopState::new(g.players_in_game());
            apply_priority_response_with_dm(
                &mut g,
                &mut q,
                &mut state,
                &PriorityResponse::PriorityAction(LegalAction::PassPriority),
                &mut dm,
            )
            .map_err(|e| e.to_string())?;
            dm.target = Some(Target::Object(recipient));
            if cast(&mut g, &defs["Shock"], spell_actor, &mut q, &mut dm)? != 1 {
                return Err("opposing Shock payment".into());
            }
            let spell = g
                .stack
                .iter()
                .find(|e| !e.is_ability && g.object(e.object_id).is_some_and(|o| o.name == "Shock"))
                .ok_or("Shock absent from stack")?
                .object_id;
            evidence["spell_caster"] = json!(spell_actor.index());
            evidence["target_recipient"] = json!(format!("{recipient:?}"));
            evidence["target_recipient_types"] =
                json!(format!("{:?}", g.calculated_card_types(recipient)));
            evidence["opposing_spell_targets"] = json!(format!(
                "{:?}",
                g.stack
                    .iter()
                    .find(|e| e.object_id == spell)
                    .unwrap()
                    .targets
            ));
            g.turn.priority_player = Some(PlayerId(0));
            dm.target = Some(Target::Object(spell));
            dm.resolve_only_one = true;
            let activation = activation(&mut g, &mut q, &mut dm, source)?;
            expected = json!({"activation":activation_expected(1),"before_can_be_blocked":mode==0||mode==3,"original_shock_still_on_stack":false,"source_in_graveyard":true,"shock_in_graveyard":true,"alice_life":20});
            actual = json!({"activation":activation,"before_can_be_blocked":before_block,"original_shock_still_on_stack":g.stack.iter().any(|e|e.object_id==spell),"source_in_graveyard":g.player(PlayerId(0)).unwrap().graveyard.iter().any(|id|g.object(*id).is_some_and(|o|o.name==def.name())),"shock_in_graveyard":g.player(spell_actor).unwrap().graveyard.iter().any(|id|g.object(*id).is_some_and(|o|o.name=="Shock")),"alice_life":g.player(PlayerId(0)).unwrap().life});
        }
        _ => {
            let name = if mode == 1 {
                "Trained Arynx"
            } else {
                "Grizzly Bears"
            };
            if cast(&mut g, &defs[name], PlayerId(0), &mut q, &mut dm)? != 2 {
                return Err("crew resource payment".into());
            }
            resolve_all(&mut g, &mut q, &mut dm)?;
            let resource = find(&g, name)?;
            dm.resources = vec![resource];
            let before_haste = g.object_has_static_ability_id(source, K::Haste);
            let activation = activation(&mut g, &mut q, &mut dm, source)?;
            expected = json!({"activation":activation_expected(0),"before_haste":mode==1,"after_haste":mode==1,"resource_tapped":true,"source_creature":true,"power":3,"toughness":3});
            actual = json!({"activation":activation,"before_haste":before_haste,"after_haste":g.object_has_static_ability_id(source,K::Haste),"resource_tapped":g.is_tapped(resource),"source_creature":g.current_has_card_type(source,CardType::Creature),"power":g.calculated_power(source),"toughness":g.calculated_toughness(source)});
        }
    }
    Ok(
        json!({"expected":expected,"actual":actual,"state_evidence":evidence,"execution_trace":dm.trace}),
    )
}
#[test]
#[ignore = "manual scoped conditional index state transition report"]
fn report_zone_stack_activations() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let inventory =
        root.join("reports/runtime-audit/corpus/267a16aff3b321196397d0b4/inventory.json");
    let source =
        root.join("crates/ironsmith-tools/tests/runtime_ability_index_zone_stack_reproductions.rs");
    let binary = std::env::current_exe().unwrap();
    let paths = [&inventory, &source, &binary];
    let before: Vec<_> = paths
        .iter()
        .map(|p| json!({"path":p,"sha256":hash(p)}))
        .collect();
    let inv: Value = serde_json::from_slice(&std::fs::read(&inventory).unwrap()).unwrap();
    let names = ["Colossal Rattlewurm", "Fugitive Droid", "Mobile Homestead"];
    let mut compile = vec![];
    let mut defs = std::collections::HashMap::new();
    for name in names.into_iter().chain([
        "Desert",
        "Murder",
        "Shock",
        "Grizzly Bears",
        "Trained Arynx",
        "Ornithopter",
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
        let count = if name == "Fugitive Droid" { 6 } else { 2 };
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
            root.join("reports/runtime-audit/ability-index-zone-stack-reproductions.json")
        });
    std::fs::write(out, serde_json::to_string_pretty(&report).unwrap()).unwrap();
    println!("wrote{}cases", report["rows"].as_array().unwrap().len());
}
