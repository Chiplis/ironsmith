//! Canonical paid rest-tag consumers with independent exact zone/order expectations.
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{
    DecisionMaker, GameProgress, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::{
    BooleanContext, DecisionContext, NumberContext, OrderContext, SelectObjectsContext,
    SelectOptionsContext, TargetsContext, TextInputContext, ViewCardsContext,
};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, advance_priority_with_dm, apply_decision_context_with_dm,
    apply_priority_response_with_dm, check_and_apply_sbas_with,
};
use ironsmith::game_state::Target;
use ironsmith::mana::ManaSymbol;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardDefinition, CardId, CardType, GameState, ObjectId, Phase, PlayerId, Zone};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
const SEED: u64 = 0x49524f4e534d4954;
fn alice() -> PlayerId {
    PlayerId(0)
}
struct Dm {
    card: String,
    targets: Vec<Target>,
    stage: String,
    trace: Vec<Value>,
    chosen: Vec<ObjectId>,
    reverse: bool,
    x: u32,
    plot: bool,
}
impl DecisionMaker for Dm {
    fn answers_player_choices(&self) -> bool {
        true
    }
    fn decide_boolean(&mut self, _: &GameState, c: &BooleanContext) -> bool {
        self.trace.push(json!({"stage":self.stage,"choice":"boolean","context":format!("{c:?}"),"selected":if self.stage=="paid_primary_spell"{false}else{self.plot}}));
        if self.stage == "paid_primary_spell" {
            false
        } else {
            self.plot
        }
    }
    fn decide_targets(&mut self, _: &GameState, c: &TargetsContext) -> Vec<Target> {
        assert_eq!(c.requirements.len(), 1);
        let r = &c.requirements[0];
        assert!(
            self.targets.len() >= r.min_targets
                && r.max_targets.is_none_or(|m| self.targets.len() <= m)
        );
        assert!(self.targets.iter().all(|t| r.legal_targets.contains(t)));
        self.trace.push(json!({"stage":self.stage,"choice":"targets","context":format!("{c:?}"),"selected":format!("{:?}",self.targets)}));
        self.targets.clone()
    }
    fn decide_options(&mut self, g: &GameState, c: &SelectOptionsContext) -> Vec<usize> {
        let selected = if self.card == "Plunge into Darkness"
            && c.options.iter().any(|o| {
                o.description
                    .to_lowercase()
                    .contains("pay any amount of life")
            }) {
            vec![
                c.options
                    .iter()
                    .find(|o| {
                        o.legal
                            && o.description
                                .to_lowercase()
                                .contains("pay any amount of life")
                    })
                    .unwrap()
                    .index,
            ]
        } else {
            SelectFirstDecisionMaker.decide_options(g, c)
        };
        self.trace.push(json!({"stage":self.stage,"choice":"options","context":format!("{c:?}"),"selected":selected}));
        selected
    }
    fn decide_number(&mut self, _: &GameState, c: &NumberContext) -> u32 {
        self.trace.push(json!({"stage":self.stage,"choice":"number","context":format!("{c:?}"),"selected":self.x}));
        self.x
    }
    fn decide_objects(&mut self, g: &GameState, c: &SelectObjectsContext) -> Vec<ObjectId> {
        let selected = self
            .chosen
            .iter()
            .copied()
            .filter(|id| c.candidates.iter().any(|o| o.legal && o.id == *id))
            .take(c.max.unwrap_or(usize::MAX))
            .collect::<Vec<_>>();
        if selected.len() < c.min.min(c.candidates.iter().filter(|o| o.legal).count()) {
            self.trace.push(json!({"stage":self.stage,"choice":"unexpected_required_object_candidates","context":format!("{c:?}"),"intended":self.chosen.iter().map(|id|id.0).collect::<Vec<_>>() }));
        }
        self.trace.push(json!({"stage":self.stage,"choice":"objects","context":format!("{c:?}"),"selected":selected.iter().map(|id|g.object(*id).unwrap().name.to_string()).collect::<Vec<_>>()}));
        selected
    }
    fn decide_order(&mut self, g: &GameState, c: &OrderContext) -> Vec<ObjectId> {
        let mut selected = c.items.iter().map(|(id, _)| *id).collect::<Vec<_>>();
        if c.description.to_lowercase().contains("library") {
            selected.sort_by_key(|id| {
                g.object(*id)
                    .map(|o| o.name.to_string())
                    .unwrap_or_default()
            });
            if self.reverse {
                selected.reverse();
            }
        }
        self.trace.push(json!({"stage":self.stage,"choice":"order","context":format!("{c:?}"),"selected":selected.iter().map(|id|g.object(*id).map(|o|o.name.to_string())).collect::<Vec<_>>()}));
        selected
    }
    fn decide_text(&mut self, _: &GameState, c: &TextInputContext) -> String {
        self.trace.push(json!({"stage":self.stage,"choice":"text","context":format!("{c:?}"),"selected":"Grizzly Bears"}));
        "Grizzly Bears".into()
    }
    fn view_cards(
        &mut self,
        g: &GameState,
        viewer: PlayerId,
        cards: &[ObjectId],
        c: &ViewCardsContext,
    ) {
        self.trace.push(json!({"stage":self.stage,"choice":"view","context":format!("{c:?}"),"viewer":viewer.0,"cards":cards.iter().map(|id|g.object(*id).unwrap().name.to_string()).collect::<Vec<_>>()}));
    }
}
fn game() -> GameState {
    let mut g = GameState::new(vec!["Alice".into(), "Bob".into(), "Cara".into()], 20);
    g.set_random_seed(SEED);
    g.turn.turn_number = 3;
    g.turn.active_player = alice();
    g.turn.priority_player = Some(alice());
    g.turn.phase = Phase::FirstMain;
    g.turn.step = None;
    for s in [
        ManaSymbol::White,
        ManaSymbol::Blue,
        ManaSymbol::Black,
        ManaSymbol::Red,
        ManaSymbol::Green,
        ManaSymbol::Colorless,
    ] {
        g.player_mut(alice()).unwrap().mana_pool.add(s, 12);
    }
    g
}
fn find(g: &GameState, name: &str) -> Result<ObjectId, String> {
    g.battlefield
        .iter()
        .copied()
        .find(|id| g.object(*id).is_some_and(|o| o.name == name))
        .ok_or(format!("fixture source {name} absent"))
}
fn count(g: &GameState, name: &str, z: Zone) -> usize {
    g.objects_in_deterministic_order()
        .into_iter()
        .filter(|o| o.name == name && o.zone == z)
        .count()
}
fn announce(
    g: &mut GameState,
    action: LegalAction,
    q: &mut TriggerQueue,
    dm: &mut Dm,
) -> Result<(), String> {
    g.turn.priority_player = Some(alice());
    let mut state = PriorityLoopState::new(g.players_in_game());
    let initial = g.stack.len();
    dm.trace
        .push(json!({"stage":dm.stage,"action":format!("{action:?}")}));
    let mut progress = apply_priority_response_with_dm(
        g,
        q,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    )
    .map_err(|e| e.to_string())?;
    for _ in 0..24 {
        if state.pending_cast.is_none()
            && state.pending_activation.is_none()
            && g.stack.len() > initial
        {
            return Ok(());
        }
        let GameProgress::NeedsDecisionCtx(c) = progress else {
            return Err(format!("announcement stopped:{progress:?}"));
        };
        if matches!(c, DecisionContext::Priority(_)) {
            return Err("announcement returned priority before ability/spell stacked".into());
        }
        progress =
            apply_decision_context_with_dm(g, q, &mut state, &c, dm).map_err(|e| e.to_string())?;
    }
    Err("announcement budget".into())
}
fn one(g: &mut GameState, q: &mut TriggerQueue, dm: &mut Dm) -> Result<(), String> {
    check_and_apply_sbas_with(g, q, dm).map_err(|e| e.to_string())?;
    advance_priority_with_dm(g, q, dm).map_err(|e| e.to_string())?;
    let mut state = PriorityLoopState::new(g.players_in_game());
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
    check_and_apply_sbas_with(g, q, dm).map_err(|e| e.to_string())?;
    advance_priority_with_dm(g, q, dm).map_err(|e| e.to_string())?;
    Ok(())
}
fn finish(g: &mut GameState, q: &mut TriggerQueue, dm: &mut Dm) -> Result<(), String> {
    for _ in 0..24 {
        check_and_apply_sbas_with(g, q, dm).map_err(|e| e.to_string())?;
        advance_priority_with_dm(g, q, dm).map_err(|e| e.to_string())?;
        if g.stack.is_empty() {
            return Ok(());
        }
        one(g, q, dm)?;
    }
    Err("resolution budget".into())
}
fn cast_announce(
    g: &mut GameState,
    d: &CardDefinition,
    cost: u32,
    q: &mut TriggerQueue,
    dm: &mut Dm,
) -> Result<(), String> {
    g.turn.priority_player = Some(alice());
    let id = g.create_object_from_definition(d, alice(), Zone::Hand);
    let a = compute_legal_actions(g, alice()).expect("fixture has complete replacement state")
        .into_iter()
        .find(|a| matches!(a,LegalAction::CastSpell{spell_id,..}if *spell_id==id))
        .ok_or("fixture source cast absent")?;
    let before = g.player(alice()).unwrap().mana_pool.total();
    announce(g, a, q, dm)?;
    let paid = before - g.player(alice()).unwrap().mana_pool.total();
    dm.trace
        .push(json!({"stage":"actual_paid_cast","card":d.name(),"paid":paid}));
    if paid != cost {
        return Err(format!(
            "fixture {} expected castcost {cost}, paid {paid}",
            d.name()
        ));
    }
    Ok(())
}

fn run(
    defs: &HashMap<String, (CardDefinition, String)>,
    c: &Value,
    dm: &mut Dm,
) -> Result<(Value, Value), String> {
    let n = c["card"].as_str().unwrap();
    let mut g = game();
    let mut q = TriggerQueue::new();
    let size = c["library"].as_u64().unwrap() as usize;
    let mut watched = vec![];
    for i in (0..size).rev() {
        let d = if n == "Desperate Research" && i < (c["matches"].as_u64().unwrap() as usize) {
            defs["Grizzly Bears"].0.clone()
        } else if n == "Loot, Exuberant Explorer" && i < 3 {
            defs[["Grizzly Bears", "Forest", "Hill Giant"][i]].0.clone()
        } else {
            CardDefinitionBuilder::new(CardId::new(), format!("Library witness {i}"))
                .card_types(vec![CardType::Sorcery])
                .build()
        };
        let id = g.create_object_from_definition(&d, alice(), Zone::Library);
        watched.push((g.object(id).unwrap().stable_id, d.name().to_string(), id));
    }
    watched.reverse();
    let names = watched
        .iter()
        .map(|(_, n, _)| n.clone())
        .collect::<Vec<_>>();
    dm.chosen = watched
        .iter()
        .take(if n == "Ancestral Knowledge" {
            c["exiles"].as_u64().unwrap() as usize
        } else {
            1
        })
        .map(|(_, _, id)| *id)
        .collect();
    if n == "Loot, Exuberant Explorer" {
        for _ in 0..2 {
            g.create_object_from_definition(&defs["Forest"].0, alice(), Zone::Battlefield);
        }
    }
    dm.trace.push(json!({"stage":"fixture","top_first":names,"selected_count":dm.chosen.len(),"accept":dm.plot,"pay_life":dm.x}));
    dm.stage = "paid_primary_spell".into();
    cast_announce(
        &mut g,
        &defs[n].0,
        c["cost"].as_u64().unwrap() as u32,
        &mut q,
        dm,
    )?;
    dm.stage = "primary_resolution".into();
    let mut error = None;
    if n == "Loot, Exuberant Explorer" {
        finish(&mut g, &mut q, dm)?;
        let source = find(&g, n)?;
        g.remove_summoning_sickness(source);
        let a = compute_legal_actions(&g, alice()).expect("fixture has complete replacement state")
            .into_iter()
            .find(|a| matches!(a,LegalAction::ActivateAbility{source:s,..}if *s==source))
            .ok_or("Loot actual activation absent")?;
        dm.stage = "paid_Loot_activation".into();
        let before = g.player(alice()).unwrap().mana_pool.total();
        announce(&mut g, a, &mut q, dm)?;
        let paid = before - g.player(alice()).unwrap().mana_pool.total();
        assert_eq!(paid, 6);
        assert!(g.is_tapped(source));
        dm.trace
            .push(json!({"stage":"paid_activation","paid":paid,"tapped":true}));
        dm.stage = "primary_resolution".into();
    }
    if error.is_none() {
        error = finish(&mut g, &mut q, dm).err();
    }
    let limit = match n {
        "Ancestral Knowledge" => 10,
        "Desperate Research" => 7,
        "Moonlight Bargain" => 5,
        "Plunge into Darkness" => dm.x as usize,
        "Sword-Point Diplomacy" => 3,
        "Loot, Exuberant Explorer" => 6,
        _ => unreachable!(),
    };
    let looked = limit.min(size);
    let mut zones = vec!["Library"; size];
    let mut expected_library = vec![];
    let mut alice_life = 20;
    let mut bob_life = 20;
    match n {
        "Ancestral Knowledge" => {
            let exiles = c["exiles"].as_u64().unwrap() as usize;
            for z in zones.iter_mut().take(exiles) {
                *z = "Exile";
            }
            let mut indices = (exiles..looked).collect::<Vec<_>>();
            if dm.reverse {
                indices.reverse();
            }
            indices.extend(looked..size);
            expected_library = indices.iter().map(|i| names[*i].clone()).collect();
        }
        "Desperate Research" => {
            for (i, z) in zones.iter_mut().enumerate().take(looked) {
                *z = if i < (c["matches"].as_u64().unwrap() as usize) {
                    "Hand"
                } else {
                    "Exile"
                };
            }
        }
        "Moonlight Bargain" => {
            for z in zones.iter_mut().take(looked) {
                *z = if dm.plot { "Hand" } else { "Graveyard" };
            }
            if dm.plot {
                alice_life -= 2 * looked as i32;
            }
        }
        "Plunge into Darkness" => {
            for (i, z) in zones.iter_mut().enumerate().take(looked) {
                *z = if i == 0 { "Hand" } else { "Exile" };
            }
            alice_life -= dm.x as i32;
        }
        "Sword-Point Diplomacy" => {
            for z in zones.iter_mut().take(looked) {
                *z = if dm.plot { "Exile" } else { "Hand" };
            }
            if dm.plot {
                bob_life -= 3 * looked as i32;
            }
        }
        "Loot, Exuberant Explorer" => {
            if dm.plot && looked > 0 {
                zones[0] = "Battlefield";
            }
            expected_library = names
                .iter()
                .skip(looked)
                .cloned()
                .chain(
                    names
                        .iter()
                        .take(looked)
                        .enumerate()
                        .filter(|(i, _)| !dm.plot || *i != 0)
                        .map(|(_, n)| n.clone()),
                )
                .collect();
        }
        _ => unreachable!(),
    }
    if !["Ancestral Knowledge", "Loot, Exuberant Explorer"].contains(&n) {
        expected_library = names
            .iter()
            .enumerate()
            .filter(|(i, _)| zones[*i] == "Library")
            .map(|(_, n)| n.clone())
            .collect();
    }
    let actual_zones = watched
        .iter()
        .map(|(stable, _, _)| {
            g.find_object_by_stable_id(*stable)
                .and_then(|id| g.object(id))
                .map(|o| format!("{:?}", o.zone))
        })
        .collect::<Vec<_>>();
    let mut actual_library = g
        .player(alice())
        .unwrap()
        .library
        .iter()
        .rev()
        .map(|id| g.object(*id).unwrap().name.to_string())
        .collect::<Vec<_>>();
    if n == "Loot, Exuberant Explorer" {
        let untouched = size.saturating_sub(looked);
        if actual_library.len() >= untouched {
            actual_library[untouched..].sort();
        }
        expected_library[untouched..].sort();
    }
    let views = dm
        .trace
        .iter()
        .filter(|t| t["choice"] == "view" && t["viewer"] == 0)
        .map(|t| t["cards"].clone())
        .collect::<Vec<_>>();
    let mut expected_views = if looked == 0 {
        vec![]
    } else {
        vec![json!(names.iter().take(looked).collect::<Vec<_>>())]
    };
    if n == "Loot, Exuberant Explorer" && dm.plot && looked > 0 {
        expected_views.push(json!([names[0]]));
    }
    let source_zones = [Zone::Battlefield, Zone::Hand, Zone::Graveyard, Zone::Exile]
        .iter()
        .map(|z| count(&g, n, *z))
        .collect::<Vec<_>>();
    let expected_source = if ["Ancestral Knowledge", "Loot, Exuberant Explorer"].contains(&n) {
        vec![1, 0, 0, 0]
    } else {
        vec![0, 0, 1, 0]
    };
    Ok((
        json!({"error":null,"zones":zones,"library_top_first_or_random_tail_sorted":expected_library,"alice_life":alice_life,"bob_life":bob_life,"alice_library_views":expected_views,"source_zones_bf_hand_gy_exile":expected_source}),
        json!({"error":error,"zones":actual_zones,"library_top_first_or_random_tail_sorted":actual_library,"alice_life":g.player(alice()).unwrap().life,"bob_life":g.player(PlayerId(1)).unwrap().life,"alice_library_views":views,"source_zones_bf_hand_gy_exile":source_zones}),
    ))
}
fn hash(p: &std::path::Path) -> String {
    Sha256::digest(std::fs::read(p).unwrap())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn generate() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let p = root.join("reports/runtime-audit");
    let input = p.join("rest-library-inputs.json");
    let data: Value = serde_json::from_slice(&std::fs::read(&input).unwrap()).unwrap();
    let mut defs = HashMap::new();
    let mut artifacts = vec![];
    let mut rows = vec![];
    for (n, v) in data["cards"].as_object().unwrap() {
        let b = ironsmith_compiler::CardDefinitionBuilder::new(
            CardId::new(),
            v["parse_name"].as_str().unwrap_or(n),
        );
        match ironsmith_registry::compile_builder_to_artifact(
            b,
            v["parse_input"].as_str().unwrap(),
            false,
        ) {
            Ok((a, d)) => {
                artifacts.push(json!({"card":n,"artifact_checksum":a.payload_checksum,"definition":a.payload.definition}));
                defs.insert(n.clone(), (d, a.payload_checksum));
            }
            Err(e) => rows
                .push(json!({"card":n,"status":"compile_failed","actual":{"error":e.to_string()}})),
        }
    }
    for c in data["cases"].as_array().unwrap() {
        let n = c["card"].as_str().unwrap();
        eprintln!("REST_LIBRARY {c}");
        let mut dm = Dm {
            card: n.into(),
            targets: vec![],
            stage: "fixture".into(),
            trace: vec![],
            chosen: vec![],
            reverse: c["reverse"].as_bool().unwrap_or(false),
            x: c["life"].as_u64().unwrap_or(0) as u32,
            plot: c["accept"].as_bool().unwrap_or(false),
        };
        let result = run(&defs, c, &mut dm);
        let (status, expected, actual) = match result {
            Ok((e, a)) => (
                if a == e {
                    "expected_result_observed"
                } else if !a["error"].is_null() {
                    "resolution_failed"
                } else {
                    "semantic_mismatch"
                },
                e,
                a,
            ),
            Err(e) => (
                "execution_or_fixture_error",
                Value::Null,
                json!({"error":e}),
            ),
        };
        rows.push(json!({"card":n,"scenario":c,"status":status,"expected":expected,"actual":actual,"artifact_checksum":defs[n].1,"execution_trace":dm.trace}));
    }
    let binary = std::env::current_exe().unwrap();
    let files = [
        "crates/ironsmith-engine/src/effects/zones/move_to_zone.rs",
        "crates/ironsmith-engine/src/effects/cards/look_at_top.rs",
        "crates/ironsmith-engine/src/effects/composition/choose_objects.rs",
    ];
    let report = json!({"scope":"Actual canonical paid casts for Ancestral Knowledge, Desperate Research, Moonlight Bargain, Plunge into Darkness, Sword-Point Diplomacy, and Loot. Normal priority and ETB resolution; Loot actual six-mana tap activation. Exact viewed cards, zones, life, source location and library order asserted from Oracle.","limitations":"Mana, library and Loot two-land resources are seeded. Loot summoning sickness is cleared. Random bottom remainder compared as a multiset after exact untouched top prefix. Choice policies specify legal intended Oracle choices; unexpected engine prompts are captured as earlier gates. No injected effect tags or direct effects.","provenance":{"binary":binary,"binary_sha256":std::env::var("AUDIT_BINARY_SHA256").unwrap(),"source_sha256":hash(&root.join("crates/ironsmith-tools/tests/runtime_rest_library_reproductions.rs")),"runtime_source_hashes":files.iter().map(|f|json!({"path":f,"sha256":hash(&root.join(f))})).collect::<Vec<_>>(),"input_sha256":hash(&input),"unique_card_ids":true,"seed":SEED,"thread_stack_bytes":67108864,"compiled_via":"ironsmith_registry::compile_builder_to_artifact"},"rows":rows});
    std::fs::write(
        p.join("rest-library-artifacts.json"),
        serde_json::to_string_pretty(&artifacts).unwrap(),
    )
    .unwrap();
    std::fs::write(
        p.join("rest-library-reproductions.json"),
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
}
#[test]
#[ignore = "canonical rest-library expected-result reporter"]
fn report_rest_library() {
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(generate)
        .unwrap()
        .join()
        .unwrap();
}
