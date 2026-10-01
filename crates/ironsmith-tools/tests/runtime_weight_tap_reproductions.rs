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
        let s = if let Some(o) = c.options.iter().find(|o| {
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

const NAMES: [&str; 1] = ["Weight of Conscience"];
fn run(
    defs: &HashMap<String, CardDefinition>,
    extra: i32,
    kind: &str,
) -> Result<(Value, Value, Value), String> {
    let mut g = setup(2, 0);
    let mut dm = Choices {
        targets: vec![],
        names: vec![],
        x: 0,
        allow_optional: true,
        choose_untap: false,
        trace: vec![],
    };
    let mut producers = vec![];
    next_main(&mut g);
    let target_cast = cast(&mut g, &defs["Hill Giant"], 1, &mut dm)?;
    if target_cast["mana_paid"] != 4 || !target_cast["resolution_error"].is_null() {
        return Err("target cast failed".into());
    }
    next_main(&mut g);
    let target = find(&g, "Hill Giant", Zone::Battlefield)?;
    let target_stable = g.object(target).unwrap().stable_id;
    dm.targets = vec![Target::Object(target)];
    let source_cast = paid(&mut g, defs, NAMES[0], &mut dm, 2)?;
    let source = find(&g, NAMES[0], Zone::Battlefield)?;
    let source_stable = g.object(source).unwrap().stable_id;
    if g.object(source).unwrap().attached_to
        != Some(ironsmith::object::AttachmentTarget::Object(target))
    {
        return Err("actual Aura attachment failed".into());
    }
    let mut resources = vec![];
    dm.targets.clear();
    for i in 0..2 + extra {
        let (n, price) = match kind {
            "humans" => {
                if i == 0 {
                    ("Fugitive Wizard", 1)
                } else {
                    ("Sage of Lat-Nam", 2)
                }
            }
            "disjoint" => {
                if i == 0 {
                    ("Fugitive Wizard", 1)
                } else {
                    ("Grizzly Bears", 2)
                }
            }
            _ => ("Grizzly Bears", 2),
        };
        let before = g.battlefield.clone();
        producers.push(paid(&mut g, defs, n, &mut dm, price)?);
        resources.push(
            g.battlefield
                .iter()
                .find(|id| !before.contains(id))
                .copied()
                .ok_or("resource missing")?,
        );
    }
    if extra < 0 {
        producers.push(paid(&mut g, defs, "Bonesplitter", &mut dm, 1)?);
    }
    if kind == "tapped" {
        dm.targets = vec![Target::Object(resources[0])];
        producers.push(paid(&mut g, defs, "Twiddle", &mut dm, 1)?);
        if !g.is_tapped(resources[0]) {
            return Err("resource Twiddle failed".into());
        }
    }
    dm.targets.clear();
    dm.names = resources
        .iter()
        .map(|id| g.object(*id).unwrap().name.to_string())
        .collect();
    let actions = compute_legal_actions(&g, PlayerId(0)).expect("fixture has complete replacement state");
    let offered = actions
        .iter()
        .any(|a| matches!(a,LegalAction::ActivateAbility{source:s,ability_index:1}if *s==source));
    let before = json!({"source_index":1,"attached_to":format!("{:?}",g.object(source).unwrap().attached_to),"resources":resources.iter().map(|id|json!({"id":id.0,"name":g.object(*id).unwrap().name.to_string(),"subtypes":g.object(*id).unwrap().subtypes.iter().map(|t|format!("{t:?}")).collect::<Vec<_>>(),"sick":g.is_summoning_sick(*id),"tapped":g.is_tapped(*id)})).collect::<Vec<_>>(),"actions":format!("{actions:?}")});
    let valid = extra >= 0 && kind != "disjoint" && kind != "tapped";
    if !valid || !offered {
        return Ok((
            json!({"activation_available":valid}),
            json!({"activation_available":offered}),
            json!({"source_cast":source_cast,"target_cast":target_cast,"producers":producers,"before_activation":before,"activation_dispatched":false,"choice_trace":dm.trace}),
        ));
    }
    let mana = g.player(PlayerId(0)).unwrap().mana_pool.total();
    let activation =
        action(&mut g, source, 1, false, &mut dm).unwrap_or_else(|e| json!({"resolution_error":e}));
    let zone = |stable| {
        g.objects_in_deterministic_order()
            .into_iter()
            .find(|o| o.stable_id == stable)
            .map(|o| format!("{:?}", o.zone))
    };
    let actual = json!({"error":activation["resolution_error"],"mana_paid":mana-g.player(PlayerId(0)).unwrap().mana_pool.total(),"target_zone":zone(target_stable),"aura_zone":zone(source_stable),"tapped_resources":resources.iter().filter(|id|g.is_tapped(**id)).count(),"untapped_resources":resources.iter().filter(|id|!g.is_tapped(**id)).count(),"stack_length":g.stack.len()});
    let expected = json!({"error":null,"mana_paid":0,"target_zone":"Exile","aura_zone":"Graveyard","tapped_resources":2,"untapped_resources":extra,"stack_length":0});
    Ok((
        expected,
        actual,
        json!({"source_cast":source_cast,"target_cast":target_cast,"producers":producers,"before_activation":before,"activation":activation,"activation_dispatched":true,"choice_trace":dm.trace,"scope":"Actual paid Aura attached to opposing paid Giant, paid fresh creature resources and exact chosen pair. Normal priority resolves exile and Aura state-based graveyard movement. Disjoint creature-type costs are never dispatched."}),
    ))
}
fn hash(p: &std::path::Path) -> String {
    Sha256::digest(std::fs::read(p).unwrap())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
#[test]
#[ignore = "fixed multi-permanent tap cost audit"]
fn report_weight_tap() {
    let input = std::path::PathBuf::from(std::env::var("AUDIT_RUNTIME_INVENTORY").unwrap());
    let payloads: Value = serde_json::from_slice(&std::fs::read(&input).unwrap()).unwrap();
    let mut defs = HashMap::new();
    let mut artifacts = vec![];
    for p in payloads["cards"].as_array().unwrap() {
        let n = p["name"].as_str().unwrap();
        if !NAMES.contains(&n)
            && ![
                "Hill Giant",
                "Grizzly Bears",
                "Fugitive Wizard",
                "Sage of Lat-Nam",
                "Bonesplitter",
                "Twiddle",
            ]
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
        let cases = vec![
            (-1, "bears"),
            (0, "bears"),
            (1, "bears"),
            (-1, "humans"),
            (0, "humans"),
            (1, "humans"),
            (0, "disjoint"),
            (0, "tapped"),
        ];
        for (extra, kind) in cases {
            let (status, expected, actual, evidence) = match run(&defs, extra, kind) {
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
            rows.push(json!({"card":name,"scenario":{"extra_resources":extra,"pair_kind":kind},"status":status,"expected":expected,"actual":actual,"fixture_evidence":evidence}));
        }
    }
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let binary = std::env::current_exe().unwrap();
    let r = json!({"scope":"Weight of Conscience actual Aura attachment, shared-type pair payment and enchanted creature exile. Required-minus-one/exact/surplus, different names sharing Human, disjoint types and tapped resource controls.","rows":rows,"artifacts":artifacts,"provenance":{"binary":binary,"binary_sha256":hash(&binary),"inventory_sha256":hash(&input),"source_sha256":hash(&root.join("crates/ironsmith-tools/tests/runtime_weight_tap_reproductions.rs")),"runtime_stack_bytes":67108864}});
    std::fs::write(
        root.join("reports/runtime-audit/weight-tap-reproductions.json"),
        serde_json::to_string_pretty(&r).unwrap(),
    )
    .unwrap();
    println!("{}", serde_json::to_string_pretty(&r).unwrap());
}
