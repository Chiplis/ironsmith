//! Canonical Martyr reveal/sacrifice costs with explicit card choices and exact outcomes.
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
    amount: u32,
    actual_x: Option<u32>,
    pay_tax: bool,
    targets: Vec<Target>,
    reveal_ids: Vec<ObjectId>,
    revealed: std::collections::BTreeSet<u64>,
    trace: Vec<Value>,
}
impl DecisionMaker for Choices {
    fn answers_player_choices(&self) -> bool {
        true
    }
    fn decide_boolean(
        &mut self,
        _: &GameState,
        c: &ironsmith::decisions::context::BooleanContext,
    ) -> bool {
        let yes = if c.description.contains("prevent effect") {
            self.pay_tax
        } else {
            true
        };
        self.trace
            .push(json!({"choice":"boolean","context":format!("{c:?}"),"selected":yes}));
        yes
    }
    fn decide_number(
        &mut self,
        g: &GameState,
        c: &ironsmith::decisions::context::NumberContext,
    ) -> u32 {
        let selected = self.amount.max(c.min).min(c.max);
        if c.is_x_value {
            self.actual_x = Some(selected);
        }
        self.trace.push(json!({"choice":"number","context":format!("{c:?}"),"selected":selected,"min":c.min,"max":c.max,"requested":self.amount,"is_x_value":c.is_x_value,"source_on_battlefield":c.source.is_some_and(|id|g.object(id).is_some_and(|o|o.zone==Zone::Battlefield)),"hand":g.player(c.player).unwrap().hand.iter().map(|id|json!({"id":id.0,"name":g.object(*id).map(|o|o.name.to_string()),"colors":format!("{:?}",g.current_colors(*id))})).collect::<Vec<_>>() }));
        selected
    }
    fn decide_targets(&mut self, g: &GameState, c: &TargetsContext) -> Vec<Target> {
        let selected = if g
            .object(c.source)
            .is_some_and(|o| o.name == "Martyr of Bones")
        {
            self.targets
                .iter()
                .copied()
                .take(self.actual_x.unwrap_or(0) as usize)
                .collect()
        } else {
            self.targets.clone()
        };
        self.trace.push(json!({"choice":"targets","context":format!("{c:?}"),"selected":format!("{selected:?}"),"requirement_bounds":c.requirements.iter().map(|r|json!({"min":r.min_targets,"max":r.max_targets})).collect::<Vec<_>>() }));
        selected
    }
    fn decide_options(&mut self, g: &GameState, c: &SelectOptionsContext) -> Vec<usize> {
        let selected = c
            .options
            .iter()
            .find(|o| o.legal && o.description.to_lowercase().contains("reveal"))
            .map(|o| vec![o.index])
            .unwrap_or_else(|| ironsmith::decision::SelectFirstDecisionMaker.decide_options(g, c));
        self.trace
            .push(json!({"choice":"options","context":format!("{c:?}"),"selected":selected}));
        selected
    }
    fn decide_objects(&mut self, g: &GameState, c: &SelectObjectsContext) -> Vec<ObjectId> {
        let selected = if c.description.to_lowercase().contains("reveal") {
            self.reveal_ids
                .iter()
                .copied()
                .filter(|id| c.candidates.iter().any(|o| o.id == *id && o.legal))
                .take(c.max.unwrap_or(self.reveal_ids.len()))
                .collect()
        } else {
            ironsmith::decision::SelectFirstDecisionMaker.decide_objects(g, c)
        };
        self.trace.push(json!({"choice":"objects","context":format!("{c:?}"),"source_present":c.source.is_some_and(|id|g.object(id).is_some()),"selected":format!("{selected:?}")}));
        selected
    }
    fn view_cards(
        &mut self,
        g: &GameState,
        viewer: PlayerId,
        cards: &[ObjectId],
        c: &ironsmith::decisions::context::ViewCardsContext,
    ) {
        if c.public && c.zone == Zone::Hand {
            for id in cards {
                self.revealed.insert(id.0);
            }
        }
        self.trace.push(json!({"choice":"public_reveal","context":format!("{c:?}"),"viewer":viewer.index(),"cards":cards.iter().map(|id|json!({"id":id.0,"name":g.object(*id).map(|o|o.name.to_string()),"zone":g.object(*id).map(|o|format!("{:?}",o.zone))})).collect::<Vec<_>>() }));
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
        g.player_mut(PlayerId(0)).unwrap().mana_pool.add(color, 12);
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
    let evidence = json!({"spell":def.name(),"mana_paid":mana-g.player(PlayerId(actor)).unwrap().mana_pool.total(),"announced_targets":format!("{:?}",g.stack.last().unwrap().targets),"resolution_error":null});
    if !g.stack.last().is_some_and(|entry| {
        g.object(entry.object_id)
            .is_some_and(|o| o.name == def.name())
    }) {
        return Err("intended spell did not reach top of stack".into());
    }
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
fn activate(
    g: &mut GameState,
    source: ObjectId,
    index: usize,
    dm: &mut Choices,
) -> Result<Value, String> {
    let initial_stack_len = g.stack.len();
    g.turn.priority_player = Some(PlayerId(0));
    let action=compute_legal_actions(g,PlayerId(0)).expect("fixture has complete replacement state").into_iter().find(|a|matches!(a,LegalAction::ActivateAbility{source:s,ability_index}if *s==source&&*ability_index==index)).ok_or("intended activation unavailable")?;
    let before = g.player(PlayerId(0)).unwrap().mana_pool.total();
    let mut q = TriggerQueue::new();
    let mut st = PriorityLoopState::new(g.players_in_game());
    let mut progress = apply_priority_response_with_dm(
        g,
        &mut q,
        &mut st,
        &PriorityResponse::PriorityAction(action.clone()),
        dm,
    )
    .map_err(|e| e.to_string())?;
    for _ in 0..24 {
        if st.pending_activation.is_none() && g.stack.len() > initial_stack_len {
            break;
        }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else {
            return Err(format!("activation stopped:{progress:?}"));
        };
        if matches!(ctx, DecisionContext::Priority(_)) {
            return Err("activation returned priority without stack".into());
        }
        progress = apply_decision_context_with_dm(g, &mut q, &mut st, &ctx, dm)
            .map_err(|e| e.to_string())?;
    }
    let paid = before - g.player(PlayerId(0)).unwrap().mana_pool.total();
    let announced_x = g.stack.last().and_then(|e| e.x_value);
    let source_present_at_announcement = g
        .object(source)
        .is_some_and(|o| o.zone == Zone::Battlefield);
    let error = finish(g, &mut q, dm).err();
    Ok(
        json!({"action":format!("{action:?}"),"mana_paid":paid,"resolution_error":error,"announced_x":announced_x,"source_present_at_announcement":source_present_at_announcement}),
    )
}

const NAMES: [&str; 5] = [
    "Martyr of Ashes",
    "Martyr of Bones",
    "Martyr of Frost",
    "Martyr of Sands",
    "Martyr of Spores",
];
fn find(g: &GameState, n: &str, z: Zone) -> Result<ObjectId, String> {
    g.objects_in_deterministic_order()
        .iter()
        .find(|o| o.name == n && o.zone == z)
        .map(|o| o.id)
        .ok_or(format!("missing {n} in {z:?}"))
}
fn count(g: &GameState, n: &str, z: Zone) -> usize {
    g.objects_in_deterministic_order()
        .iter()
        .filter(|o| o.name == n && o.zone == z)
        .count()
}
fn paid(
    g: &mut GameState,
    defs: &HashMap<String, CardDefinition>,
    name: &str,
    actor: u8,
    cost: u32,
    d: &mut Choices,
    history: &mut Vec<Value>,
) -> Result<(), String> {
    let p = cast(g, &defs[name], actor, d)?;
    if p["mana_paid"] != cost || !p["resolution_error"].is_null() {
        return Err(format!("producer failed {p}"));
    }
    history.push(p);
    Ok(())
}
fn trial(
    defs: &HashMap<String, CardDefinition>,
    name: &str,
    variant: usize,
) -> Result<(Value, Value, Value), String> {
    let mut g = setup(2, 0);
    for color in [
        ManaSymbol::White,
        ManaSymbol::Blue,
        ManaSymbol::Black,
        ManaSymbol::Red,
        ManaSymbol::Green,
        ManaSymbol::Colorless,
    ] {
        g.player_mut(PlayerId(1)).unwrap().mana_pool.add(color, 20);
    }
    let effective_x = if variant == 0 || variant == 2 { 0 } else { 2 };
    let pay_tax = variant != 3;
    let wrong_color = variant == 2;
    let mut d = Choices {
        amount: if variant == 0 { 0 } else { 2 },
        pay_tax,
        actual_x: None,
        targets: vec![],
        reveal_ids: vec![],
        revealed: Default::default(),
        trace: vec![],
    };
    let mut history = vec![];
    paid(&mut g, defs, name, 0, 1, &mut d, &mut history)?;
    let source = find(&g, name, Zone::Battlefield)?;
    let mut bear = None;
    let mut giant = None;
    let mut flyer = None;
    if name == "Martyr of Ashes" {
        paid(&mut g, defs, "Grizzly Bears", 0, 2, &mut d, &mut history)?;
        bear = Some(find(&g, "Grizzly Bears", Zone::Battlefield)?);
        paid(&mut g, defs, "Ornithopter", 0, 0, &mut d, &mut history)?;
        flyer = Some(find(&g, "Ornithopter", Zone::Battlefield)?);
    }
    if name == "Martyr of Ashes" || name == "Martyr of Spores" {
        paid(&mut g, defs, "Hill Giant", 0, 4, &mut d, &mut history)?;
        giant = Some(find(&g, "Hill Giant", Zone::Battlefield)?);
    }
    if name == "Martyr of Bones" {
        for _ in 0..3 {
            d.targets = vec![Target::Player(PlayerId(0))];
            paid(&mut g, defs, "Shock", 1, 1, &mut d, &mut history)?;
        }
    }
    let resource = if wrong_color {
        "Plains"
    } else {
        match name {
            "Martyr of Ashes" => "Shock",
            "Martyr of Bones" => "Murder",
            "Martyr of Frost" => "Opt",
            "Martyr of Sands" => "Raise the Alarm",
            _ => "Giant Growth",
        }
    };
    let resources = (0..3)
        .map(|_| g.create_object_from_definition(&defs[resource], PlayerId(0), Zone::Hand))
        .collect::<Vec<_>>();
    if effective_x > 0 {
        d.reveal_ids = resources[1..].to_vec();
    }
    let chosen_reveal = d
        .reveal_ids
        .iter()
        .map(|id| id.0)
        .collect::<std::collections::BTreeSet<_>>();
    let before_life = g.player(PlayerId(0)).unwrap().life;
    let selected_bones = if effective_x == 0 {
        0
    } else if variant == 3 {
        1
    } else {
        2
    };
    if name == "Martyr of Bones" {
        d.targets = g
            .objects_in_deterministic_order()
            .iter()
            .filter(|o| o.name == "Shock" && o.zone == Zone::Graveyard && o.owner == PlayerId(1))
            .take(selected_bones)
            .map(|o| Target::Object(o.id))
            .collect();
    } else if name == "Martyr of Spores" {
        d.targets = vec![Target::Object(giant.unwrap())];
    } else if name == "Martyr of Frost" {
        d.targets = vec![Target::Player(PlayerId(0))];
        let (_, p) = announce(&mut g, &defs["Shock"], 1, &mut d)?;
        if p["mana_paid"] != 1 {
            return Err("pending Shock payment not1".into());
        }
        history.push(p);
        let spell = g.stack.last().unwrap().object_id;
        d.targets = vec![Target::Object(spell)];
    } else {
        d.targets.clear();
    }
    let bob_before = g.player(PlayerId(1)).unwrap().mana_pool.total();
    let action =
        activate(&mut g, source, 0, &mut d).map_err(|e| format!("{e};choices={:?}", d.trace))?;
    let state = json!({"error":action["resolution_error"],"activation_mana":action["mana_paid"],"announced_x":action["announced_x"],"source_on_battlefield_after_payment":action["source_present_at_announcement"],"source_graveyard":count(&g,name,Zone::Graveyard),"revealed_ids":d.revealed,"resource_hand_count":count(&g,resource,Zone::Hand),"life_delta":g.player(PlayerId(0)).unwrap().life-before_life,"bob_tax_paid":bob_before-g.player(PlayerId(1)).unwrap().mana_pool.total(),"bear_survives":bear.map(|id|g.object(id).is_some_and(|o|o.zone==Zone::Battlefield)),"giant_power":giant.and_then(|id|g.calculated_power(id)),"giant_toughness":giant.and_then(|id|g.calculated_toughness(id)),"giant_damage":giant.map(|id|g.damage_on(id)),"flyer_damage":flyer.map(|id|g.damage_on(id)),"shock_exile":if name=="Martyr of Bones"{count(&g,"Shock",Zone::Exile)}else{0},"shock_graveyard":if name=="Martyr of Bones"{count(&g,"Shock",Zone::Graveyard)}else{0}});
    let expected = json!({"error":null,"activation_mana":if name=="Martyr of Ashes"||name=="Martyr of Frost"{2}else{1},"announced_x":effective_x,"source_on_battlefield_after_payment":false,"source_graveyard":1,"revealed_ids":chosen_reveal,"resource_hand_count":3,"life_delta":if name=="Martyr of Sands"{3*effective_x as i32}else if name=="Martyr of Frost"&&pay_tax{-2}else{0},"bob_tax_paid":if name=="Martyr of Frost"&&pay_tax{effective_x}else{0},"bear_survives":bear.map(|_|effective_x==0),"giant_power":giant.map(|_|3+if name=="Martyr of Spores"{effective_x as i32}else{0}),"giant_toughness":giant.map(|_|3+if name=="Martyr of Spores"{effective_x as i32}else{0}),"giant_damage":giant.map(|_|if name=="Martyr of Ashes"{effective_x}else{0}),"flyer_damage":flyer.map(|_|0),"shock_exile":if name=="Martyr of Bones"{selected_bones}else{0},"shock_graveyard":if name=="Martyr of Bones"{3-selected_bones}else{0}});
    Ok((
        state,
        expected,
        json!({"paid_source_and_producers":history,"activation":action,"decisions":d.trace,"requested_x":if variant==0{0}else{2},"effective_x":effective_x,"wrong_color_only":wrong_color,"requested_reveal_ids":chosen_reveal,"resource_ids":resources.iter().map(|id|id.0).collect::<Vec<_>>(),"tax_accepted":pay_tax}),
    ))
}
fn hash(p: &std::path::Path) -> String {
    Sha256::digest(std::fs::read(p).unwrap())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn normalize(v: &Value, p: &str, ids: &mut Vec<String>) -> Value {
    match v {
        Value::Object(o) => {
            let mut r = serde_json::Map::new();
            for (k, v) in o {
                if k == "id"
                    && p.ends_with("/card")
                    && o.contains_key("card_types")
                    && o.contains_key("name")
                {
                    ids.push(format!("{p}/id"));
                    continue;
                }
                r.insert(k.clone(), normalize(v, &format!("{p}/{k}"), ids));
            }
            Value::Object(r)
        }
        Value::Array(a) => Value::Array(
            a.iter()
                .enumerate()
                .map(|(i, v)| normalize(v, &format!("{p}/{i}"), ids))
                .collect(),
        ),
        _ => v.clone(),
    }
}
#[test]
#[ignore = "strict paid Martyr reveal and sacrifice costs"]
fn report_martyr_costs() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let input = root.join("reports/runtime-audit/martyr-reveal-cost-frozen-inputs.json");
    let paths = [
        std::env::current_exe().unwrap(),
        input.clone(),
        root.join("crates/ironsmith-tools/tests/runtime_martyr_reveal_cost_reproductions.rs"),
    ];
    let before: Vec<_> = paths
        .iter()
        .map(|p| json!({"path":p,"sha256":hash(p)}))
        .collect();
    let payloads: Value = serde_json::from_slice(&std::fs::read(&input).unwrap()).unwrap();
    let mut defs = HashMap::new();
    let mut compilation = vec![];
    for p in payloads["cards"].as_array().unwrap() {
        let n = p["name"].as_str().unwrap();
        let (a, d) = ironsmith_registry::compile_builder_to_artifact(
            ironsmith_compiler::CardDefinitionBuilder::new(
                CardId::new(),
                p["parse_name"].as_str().unwrap_or(n),
            ),
            p["parse_input"].as_str().unwrap(),
            false,
        )
        .unwrap();
        let definition = serde_json::to_value(&a.payload.definition).unwrap();
        let (mut ids, mut fi) = (vec![], vec![]);
        let parity = normalize(&definition, "/definition", &mut ids)
            == normalize(&p["frozen_definition"], "/definition", &mut fi);
        compilation.push(json!({"card":n,"artifact_checksum":a.payload_checksum,"frozen_artifact_checksum":p["frozen_artifact_checksum"],"definition_matches_frozen_except_unique_card_ids":parity,"ignored_id_paths":ids,"definition":definition}));
        defs.insert(n.to_string(), d);
    }
    let mut rows = vec![];
    for name in NAMES {
        for variant in 0..if name == "Martyr of Frost" || name == "Martyr of Bones" {
            4
        } else {
            3
        } {
            println!("stage: {name} variant{variant}");
            let (status, actual, expected, evidence) = match trial(&defs, name, variant) {
                Ok((a, e, t)) => (
                    if a == e {
                        "expected_result_observed"
                    } else if !a["error"].is_null() {
                        "execution_failed"
                    } else {
                        "semantic_mismatch"
                    },
                    a,
                    e,
                    t,
                ),
                Err(e) => (
                    "fixture_or_announcement_error",
                    json!({"error":e}),
                    Value::Null,
                    Value::Null,
                ),
            };
            rows.push(json!({"card":name,"scenario":{"variant":variant},"status":status,"actual":actual,"expected":expected,"fixture_evidence":evidence}));
        }
    }
    let after: Vec<_> = paths
        .iter()
        .map(|p| json!({"path":p,"sha256":hash(p)}))
        .collect();
    let report = json!({"scope":"Five full canonical Martyrs paid normally then activated through actual cost decisions. X0/X2 with three matching hand cards and explicit last-two reveal selections; no matching color clamps legal X to0; Bones target response is additionally capped to the actually accepted X, ensuring intended X2 does not send illegal X0 targets; source sacrifice and count propagation checked. Frost responds to an actually paid pending Shock with pay/decline; Bones targets actual paid Shock graveyard cards and optional one-of-two selection. Exact damage/life/PT/exile/tax outcomes and revealed identities.","rows":rows,"compilation":compilation,"provenance":{"before":before,"after":after,"artifacts_unchanged":before==after}});
    std::fs::write(
        root.join("reports/runtime-audit/martyr-reveal-cost-final-execution.json"),
        serde_json::to_string_pretty(&report).unwrap() + "\n",
    )
    .unwrap();
}
