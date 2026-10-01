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

const NAMES: [&str; 6] = [
    "Crackling Perimeter",
    "Dune Diviner",
    "Gateway Shade",
    "Hecatomb",
    "Karplusan Giant",
    "Sage of the Maze",
];
fn until_alice(g: &mut GameState) {
    next_main(g);
    for _ in 0..3 {
        if g.turn.active_player == PlayerId(0) {
            return;
        }
        next_main(g);
    }
}
fn run(
    defs: &HashMap<String, CardDefinition>,
    name: &str,
    n: usize,
    state: &str,
) -> Result<(Value, Value, Value), String> {
    let mut g = setup(3, 0);
    let mut dm = Choices {
        targets: vec![],
        names: vec![],
        x: 0,
        allow_optional: true,
        choose_untap: false,
        trace: vec![],
    };
    let mut producers = vec![];
    if name == "Hecatomb" {
        dm.names = vec!["Ornithopter".into()];
        for _ in 0..4 {
            producers.push(paid(&mut g, defs, "Ornithopter", &mut dm, 0)?);
        }
    }
    let price = match name {
        "Crackling Perimeter" => 2,
        "Karplusan Giant" => 7,
        _ => 3,
    };
    let source_cast = paid(&mut g, defs, name, &mut dm, price)?;
    let source = find(&g, name, Zone::Battlefield)?;
    if name == "Hecatomb" && count(&g, "Ornithopter", Zone::Graveyard) != 4 {
        return Err("actual Hecatomb creature payment failed".into());
    }
    let land_name = match name {
        "Dune Diviner" => "Desert",
        "Hecatomb" => "Swamp",
        "Karplusan Giant" => "Snow-Covered Forest",
        _ => "Azorius Guildgate",
    };
    let mut resources = vec![];
    if n == 0 {
        let own = play_land(&mut g, &defs["Plains"], &mut dm)?;
        producers.push(json!({"land_play":"Plains","owner":0,"id":own.0}));
        next_main(&mut g);
        let opp = play_land(&mut g, &defs[land_name], &mut dm)?;
        producers.push(json!({"land_play":land_name,"owner":1,"id":opp.0}));
        while g.turn.active_player != PlayerId(0) {
            next_main(&mut g);
        }
        until_alice(&mut g);
    } else {
        for _ in 0..n {
            let id = play_land(&mut g, &defs[land_name], &mut dm)?;
            resources.push(id);
            producers.push(json!({"land_play":land_name,"owner":0,"id":id.0}));
            until_alice(&mut g);
        }
    }
    if resources.iter().any(|id| g.is_tapped(*id)) {
        return Err("actual next-turn land untap failed".into());
    }
    let mut creature_target = None;
    if state == "creature_target" {
        next_main(&mut g);
        let e = cast(&mut g, &defs["Grizzly Bears"], 1, &mut dm)?;
        if e["mana_paid"] != 2 || !e["resolution_error"].is_null() {
            return Err("opponent target cast failed".into());
        }
        producers.push(e);
        creature_target = Some(find(&g, "Grizzly Bears", Zone::Battlefield)?);
        while g.turn.active_player != PlayerId(0) {
            next_main(&mut g);
        }
    }
    if name == "Sage of the Maze" {
        dm.targets = vec![Target::Object(source)];
        producers.push(paid(&mut g, defs, "Twiddle", &mut dm, 1)?);
        if !g.is_tapped(source) {
            return Err("Sage producer tap failed".into());
        }
    }
    if state == "tapped" {
        dm.targets = vec![Target::Object(resources[0])];
        producers.push(paid(&mut g, defs, "Twiddle", &mut dm, 1)?);
        if !g.is_tapped(resources[0]) {
            return Err("land producer tap failed".into());
        }
    }
    dm.targets = if name == "Hecatomb" {
        vec![
            creature_target
                .map(Target::Object)
                .unwrap_or(Target::Player(PlayerId(1))),
        ]
    } else {
        vec![]
    };
    dm.names = vec![land_name.into()];
    let index = match name {
        "Gateway Shade" | "Hecatomb" => 1,
        "Sage of the Maze" => 2,
        _ => 0,
    };
    let actions = compute_legal_actions(&g, PlayerId(0)).expect("fixture has complete replacement state");
    let offered=actions.iter().any(|a|matches!(a,LegalAction::ActivateAbility{source:s,ability_index}if *s==source&&*ability_index==index));
    let before = json!({"source_index":index,"source_tapped":g.is_tapped(source),"resources":resources.iter().map(|id|json!({"id":id.0,"name":g.object(*id).unwrap().name.to_string(),"tapped":g.is_tapped(*id),"controller":g.controller_of_id(*id).map(|p|p.index())})).collect::<Vec<_>>(),"other_lands":g.battlefield.iter().filter(|id|g.object(**id).unwrap().card_types.contains(&CardType::Land)&&!resources.contains(id)).map(|id|json!({"name":g.object(*id).unwrap().name.to_string(),"tapped":g.is_tapped(*id),"controller":g.controller_of_id(*id).map(|p|p.index())})).collect::<Vec<_>>(),"actions":format!("{actions:?}")});
    let valid = n > 0 && state != "tapped";
    if !valid || !offered {
        return Ok((
            json!({"activation_available":valid}),
            json!({"activation_available":offered}),
            json!({"source_cast":source_cast,"producers":producers,"before_activation":before,"activation_dispatched":false,"choice_trace":dm.trace}),
        ));
    }
    let mana = g.player(PlayerId(0)).unwrap().mana_pool.total();
    let activation = action(&mut g, source, index, false, &mut dm)
        .unwrap_or_else(|e| json!({"resolution_error":e}));
    let pt = |g: &GameState| {
        if g.object(source)
            .unwrap()
            .card_types
            .contains(&CardType::Creature)
        {
            json!([g.calculated_power(source), g.calculated_toughness(source)])
        } else {
            Value::Null
        }
    };
    let mut actual = json!({"error":activation["resolution_error"],"mana_paid":mana-g.player(PlayerId(0)).unwrap().mana_pool.total(),"source_tapped":g.is_tapped(source),"source_pt":pt(&g),"tapped_resources":resources.iter().filter(|id|g.is_tapped(**id)).count(),"untapped_resources":resources.iter().filter(|id|!g.is_tapped(**id)).count(),"life":g.players.iter().map(|p|p.life).collect::<Vec<_>>(),"target_damage":creature_target.map(|id|g.damage_on(id)),"stack_length":g.stack.len()});
    let mut life = vec![20, 20, 20];
    if name == "Crackling Perimeter" {
        life[1] -= 1;
        life[2] -= 1;
    }
    if name == "Dune Diviner" {
        life[0] += 1;
    }
    if name == "Hecatomb" && creature_target.is_none() {
        life[1] -= 1;
    }
    let expected_pt = match name {
        "Dune Diviner" => json!([2, 3]),
        "Gateway Shade" => json!([3, 3]),
        "Karplusan Giant" => json!([4, 4]),
        "Sage of the Maze" => json!([1, 3]),
        _ => Value::Null,
    };
    let mut expected = json!({"error":null,"mana_paid":if name=="Dune Diviner"{1}else{0},"source_tapped":false,"source_pt":expected_pt,"tapped_resources":1,"untapped_resources":n-1,"life":life,"target_damage":creature_target.map(|_|1),"stack_length":0});
    g.turn.phase = ironsmith::Phase::Ending;
    g.turn.step = Some(ironsmith::Step::Cleanup);
    ironsmith::turn::execute_cleanup_step(&mut g);
    actual["cleanup_source_pt"] = pt(&g);
    expected["cleanup_source_pt"] = match name {
        "Gateway Shade" => json!([1, 1]),
        "Karplusan Giant" => json!([3, 3]),
        _ => expected["source_pt"].clone(),
    };
    Ok((
        expected,
        actual,
        json!({"source_cast":source_cast,"producers":producers,"before_activation":before,"activation":activation,"activation_dispatched":true,"choice_trace":dm.trace,"scope":"Actual source casting, actual land plays and normal next-turn untap, source/land tapping via paid Twiddle, Hecatomb four actual paid creature sacrifices. Ineligible owned Plains plus untapped opponent matching land in zero-own-resource cases. Exact one land tap and immediate result/cleanup."}),
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
fn report_single_land_tap() {
    let input = std::path::PathBuf::from(std::env::var("AUDIT_RUNTIME_INVENTORY").unwrap());
    let payloads: Value = serde_json::from_slice(&std::fs::read(&input).unwrap()).unwrap();
    let mut defs = HashMap::new();
    let mut artifacts = vec![];
    for p in payloads["cards"].as_array().unwrap() {
        let n = p["name"].as_str().unwrap();
        if !NAMES.contains(&n)
            && ![
                "Plains",
                "Azorius Guildgate",
                "Desert",
                "Swamp",
                "Snow-Covered Forest",
                "Ornithopter",
                "Twiddle",
                "Grizzly Bears",
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
        let mut cases = vec![(0, "ready"), (1, "ready"), (2, "ready"), (1, "tapped")];
        if name == "Hecatomb" {
            cases.push((1, "creature_target"));
        }
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
    let r = json!({"scope":"Six single-land tap payment paths through actual land play, untap and source payment. Zero/exact/surplus/tapped, subtype/snow/controller filters and exact resulting effects with cleanup.","rows":rows,"artifacts":artifacts,"provenance":{"binary":binary,"binary_sha256":hash(&binary),"inventory_sha256":hash(&input),"source_sha256":hash(&root.join("crates/ironsmith-tools/tests/runtime_single_land_tap_reproductions.rs")),"runtime_stack_bytes":67108864}});
    std::fs::write(
        root.join("reports/runtime-audit/single-land-tap-reproductions.json"),
        serde_json::to_string_pretty(&r).unwrap(),
    )
    .unwrap();
    println!("{}", serde_json::to_string_pretty(&r).unwrap());
}
