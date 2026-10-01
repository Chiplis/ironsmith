//! Canonical paid rest-tag consumers with independent exact zone/order expectations.
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{
    DecisionMaker, GameProgress, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::{
    BooleanContext, DecisionContext, NumberContext, OrderContext, SelectObjectsContext,
    SelectOptionsContext, TargetsContext, ViewCardsContext,
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
        self.trace.push(json!({"stage":self.stage,"choice":"boolean","context":format!("{c:?}"),"selected":self.plot}));
        self.plot
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
        let selected = SelectFirstDecisionMaker.decide_options(g, c);
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
        assert!(selected.len() >= c.min.min(c.candidates.iter().filter(|o| o.legal).count()));
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
    if n == "Last One Standing" {
        let total = c["creatures"].as_u64().unwrap() as usize;
        for (name, cost) in [("Grizzly Bears", 2), ("Hill Giant", 4), ("Ornithopter", 0)]
            .into_iter()
            .take(total)
        {
            dm.stage = format!("paid_creature_{name}");
            cast_announce(&mut g, &defs[name].0, cost, &mut q, dm)?;
            finish(&mut g, &mut q, dm)?;
        }
        dm.stage = "paid_primary_spell".into();
        cast_announce(
            &mut g,
            &defs[n].0,
            c["cost"].as_u64().unwrap() as u32,
            &mut q,
            dm,
        )?;
        dm.stage = "primary_resolution".into();
        let error = finish(&mut g, &mut q, dm).err();
        let witness = ["Grizzly Bears", "Hill Giant", "Ornithopter"];
        let survivors = witness
            .iter()
            .map(|name| count(&g, name, Zone::Battlefield))
            .sum::<usize>();
        let destroyed = witness
            .iter()
            .map(|name| count(&g, name, Zone::Graveyard))
            .sum::<usize>();
        dm.trace.push(json!({"stage":"final_survivors","names":g.battlefield.iter().map(|id|g.object(*id).unwrap().name.to_string()).collect::<Vec<_>>()}));
        return Ok((
            json!({"error":null,"survivors":total.min(1),"destroyed":total.saturating_sub(1)}),
            json!({"error":error,"survivors":survivors,"destroyed":destroyed}),
        ));
    }
    let size = c["library"].as_u64().unwrap() as usize;
    let owner = if ["Bamboozle", "Sealed Fate"].contains(&n) {
        PlayerId(1)
    } else {
        alice()
    };
    let names = (0..size)
        .map(|i| format!("Library witness {i}"))
        .collect::<Vec<_>>();
    for i in (0..size).rev() {
        let d = CardDefinitionBuilder::new(CardId::new(), &names[i])
            .card_types(vec![if i % 2 == 0 {
                CardType::Sorcery
            } else {
                CardType::Land
            }])
            .build();
        g.create_object_from_definition(&d, owner, Zone::Library);
    }
    let top = g
        .player(owner)
        .unwrap()
        .library
        .iter()
        .rev()
        .copied()
        .collect::<Vec<_>>();
    assert_eq!(
        top.iter()
            .map(|id| g.object(*id).unwrap().name.to_string())
            .collect::<Vec<_>>(),
        names
    );
    if owner != alice() {
        dm.targets = vec![Target::Player(owner)];
    }
    let chosen_count = match n {
        "Bamboozle" => 2,
        "Make Your Own Luck" => usize::from(dm.plot),
        "Sealed Fate" => usize::from(dm.x > 0),
        _ => 1,
    }
    .min(size);
    dm.chosen = top.iter().copied().take(chosen_count).collect();
    dm.trace.push(json!({"stage":"fixture","library_owner":owner.0,"initial_top_first":names,"intended_chosen":names.iter().take(chosen_count).collect::<Vec<_>>(),"intended_order":if dm.reverse{"reverse remainder"}else{"original remainder"}}));
    dm.stage = "paid_primary_spell".into();
    cast_announce(
        &mut g,
        &defs[n].0,
        c["cost"].as_u64().unwrap() as u32,
        &mut q,
        dm,
    )?;
    if n == "Browse" {
        dm.stage = "primary_source_resolution".into();
        finish(&mut g, &mut q, dm)?;
        let source = find(&g, n)?;
        let action = compute_legal_actions(&g, alice()).expect("fixture has complete replacement state")
            .into_iter()
            .find(|a| matches!(a,LegalAction::ActivateAbility{source:s,..}if *s==source))
            .ok_or("Browse activation absent")?;
        dm.stage = "paid_Browse_activation".into();
        let before = g.player(alice()).unwrap().mana_pool.total();
        announce(&mut g, action, &mut q, dm)?;
        let paid = before - g.player(alice()).unwrap().mana_pool.total();
        assert_eq!(paid, 4);
        dm.trace
            .push(json!({"stage":"actual_paid_activation","paid":paid}));
    }
    dm.stage = "primary_resolution".into();
    let error = finish(&mut g, &mut q, dm).err();
    let looked = match n {
        "Browse" => 5,
        "Make Your Own Luck" => 3,
        "Bamboozle" => 4,
        "Sealed Fate" => dm.x as usize,
        _ => unreachable!(),
    }
    .min(size);
    let mut expected_zones = vec!["Library"; size];
    let mut remainder = (chosen_count..looked).collect::<Vec<_>>();
    match n {
        "Browse" => {
            if size > 0 {
                expected_zones[0] = "Hand";
            }
            for z in expected_zones.iter_mut().take(looked).skip(1) {
                *z = "Exile";
            }
        }
        "Make Your Own Luck" => {
            for (i, z) in expected_zones.iter_mut().enumerate().take(looked) {
                *z = if i == 0 && dm.plot { "Exile" } else { "Hand" };
            }
        }
        "Bamboozle" => {
            for z in expected_zones.iter_mut().take(chosen_count) {
                *z = "Graveyard";
            }
        }
        "Sealed Fate" => {
            if chosen_count > 0 {
                expected_zones[0] = "Exile";
            }
        }
        _ => unreachable!(),
    }
    let expected_library = if ["Bamboozle", "Sealed Fate"].contains(&n) {
        if dm.reverse {
            remainder.reverse();
        }
        remainder.extend(looked..size);
        remainder
            .iter()
            .map(|i| names[*i].clone())
            .collect::<Vec<_>>()
    } else {
        names
            .iter()
            .enumerate()
            .filter(|(i, _)| expected_zones[*i] == "Library")
            .map(|(_, s)| s.clone())
            .collect::<Vec<_>>()
    };
    let actual_zones = names
        .iter()
        .map(|name| {
            g.objects_in_deterministic_order()
                .into_iter()
                .find(|o| o.name == name.as_str())
                .map(|o| format!("{:?}", o.zone))
        })
        .collect::<Vec<_>>();
    let actual_library = g
        .player(owner)
        .unwrap()
        .library
        .iter()
        .rev()
        .map(|id| g.object(*id).unwrap().name.to_string())
        .collect::<Vec<_>>();
    let actual_plotted = names
        .iter()
        .map(|name| {
            g.objects_in_deterministic_order()
                .into_iter()
                .find(|o| o.name == name.as_str())
                .is_some_and(|o| g.is_plotted_by(o.id, alice()))
        })
        .collect::<Vec<_>>();
    let expected_plotted = (0..size)
        .map(|i| n == "Make Your Own Luck" && dm.plot && i == 0)
        .collect::<Vec<_>>();
    Ok((
        json!({"error":null,"zones":expected_zones,"library_top_first":expected_library,"plotted":expected_plotted}),
        json!({"error":error,"zones":actual_zones,"library_top_first":actual_library,"plotted":actual_plotted}),
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
    let input = p.join("rest-tag-inputs.json");
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
        eprintln!("REST_TAG {c}");
        let mut dm = Dm {
            targets: vec![],
            stage: "fixture".into(),
            trace: vec![],
            chosen: vec![],
            reverse: c["reverse"].as_bool().unwrap_or(false),
            x: c["x"].as_u64().unwrap_or(0) as u32,
            plot: c["plot"].as_bool().unwrap_or(false),
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
    let report = json!({"scope":"Actual paid canonical rest-tag consumers, full exact library zone/order outcomes and random-destruction survivor counts. Browse actual source cast and activation, Sealed Fate actual announced X and payment, all normal priority resolutions/SBAs.","limitations":"Mana and named inert typed library witnesses seeded. Last One Standing creatures are themselves full canonical paid casts. No engine edits; controls cover selected scenarios only.","provenance":{"binary":binary,"binary_sha256":std::env::var("AUDIT_BINARY_SHA256").unwrap(),"source_sha256":hash(&root.join("crates/ironsmith-tools/tests/runtime_rest_tag_reproductions.rs")),"runtime_source_hashes":files.iter().map(|f|json!({"path":f,"sha256":hash(&root.join(f))})).collect::<Vec<_>>(),"input_sha256":hash(&input),"unique_card_ids":true,"seed":SEED,"thread_stack_bytes":67108864,"compiled_via":"ironsmith_registry::compile_builder_to_artifact"},"rows":rows});
    std::fs::write(
        p.join("rest-tag-artifacts.json"),
        serde_json::to_string_pretty(&artifacts).unwrap(),
    )
    .unwrap();
    std::fs::write(
        p.join("rest-tag-reproductions.json"),
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
}
#[test]
#[ignore = "canonical rest-tag expected-result reporter"]
fn report_rest_tags() {
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(generate)
        .unwrap()
        .join()
        .unwrap();
}
