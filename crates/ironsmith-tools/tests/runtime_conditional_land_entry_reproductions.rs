//! Canonical conditional-land entry and actual advertised mana dispatch audit.
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
use ironsmith::{CardDefinition, CardId, GameState, ObjectId, Phase, PlayerId, Zone};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
const SEED: u64 = 0x49524f4e534d4954;
fn alice() -> PlayerId {
    PlayerId(0)
}
struct Dm {
    actor: PlayerId,
    targets: Vec<Target>,
    stage: String,
    trace: Vec<Value>,
    chosen: Vec<ObjectId>,
    reverse: bool,
    x: u32,
    plot: bool,
    option_text: String,
    mana_color: ironsmith::color::Color,
}
impl DecisionMaker for Dm {
    fn answers_player_choices(&self) -> bool {
        true
    }
    fn decide_colors(
        &mut self,
        _: &GameState,
        c: &ironsmith::decisions::context::ColorsContext,
    ) -> Vec<ironsmith::color::Color> {
        assert!(
            c.available_colors
                .as_ref()
                .is_none_or(|colors| colors.contains(&self.mana_color))
        );
        let chosen = vec![self.mana_color; c.count as usize];
        self.trace.push(json!({"stage":self.stage,"choice":"colors","context":format!("{c:?}"),"selected":format!("{chosen:?}")}));
        chosen
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
        let selected = if !self.option_text.is_empty()
            && !c.description.contains("Confirm mana")
            && c.options
                .iter()
                .any(|o| o.legal && o.description.to_lowercase().contains(&self.option_text))
        {
            vec![
                c.options
                    .iter()
                    .find(|o| o.legal && o.description.to_lowercase().contains(&self.option_text))
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
        self.trace.push(json!({"stage":self.stage,"choice":"number","context":format!("{c:?}"),"selected":self.x.max(c.min).min(c.max)}));
        self.x.max(c.min).min(c.max)
    }
    fn decide_objects(&mut self, g: &GameState, c: &SelectObjectsContext) -> Vec<ObjectId> {
        let selected = if (self.stage.starts_with("resolve_escape")
            || self.stage == "cast_prepared_spell"
            || self.stage == "resolve_Jadzi, Steward of Fate")
            && c.min > 0
        {
            c.candidates
                .iter()
                .filter(|o| o.legal)
                .take(c.min)
                .map(|o| o.id)
                .collect::<Vec<_>>()
        } else {
            self.chosen
                .iter()
                .copied()
                .filter(|id| c.candidates.iter().any(|o| o.legal && o.id == *id))
                .take(c.max.unwrap_or(usize::MAX))
                .collect::<Vec<_>>()
        };
        assert!(
            selected.len() >= c.min.min(c.candidates.iter().filter(|o| o.legal).count()),
            "selection fixture: {c:?}, requested={:?}, selected={selected:?}",
            self.chosen
        );
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
    g.turn.turn_number = 1;
    g.turn.active_player = alice();
    g.turn.priority_player = Some(alice());
    g.turn.phase = Phase::FirstMain;
    g.turn.step = None;
    g.mark_main_phase_started();
    for s in [
        ManaSymbol::White,
        ManaSymbol::Blue,
        ManaSymbol::Black,
        ManaSymbol::Red,
        ManaSymbol::Green,
        ManaSymbol::Colorless,
    ] {
        for player in [alice(), PlayerId(1)] {
            g.player_mut(player).unwrap().mana_pool.add(s, 30);
        }
    }
    g
}
fn announce(
    g: &mut GameState,
    action: LegalAction,
    q: &mut TriggerQueue,
    dm: &mut Dm,
) -> Result<(), String> {
    g.turn.priority_player = Some(dm.actor);
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
    g.turn.priority_player = Some(dm.actor);
    let id = g.create_object_from_definition(d, dm.actor, Zone::Hand);
    let a = compute_legal_actions(g, dm.actor).expect("fixture has complete replacement state")
        .into_iter()
        .find(|a| matches!(a,LegalAction::CastSpell{spell_id,..}if *spell_id==id))
        .ok_or("fixture source cast absent")?;
    let before = g.player(dm.actor).unwrap().mana_pool.total();
    announce(g, a, q, dm)?;
    let paid = before - g.player(dm.actor).unwrap().mana_pool.total();
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

fn paid_cast(
    g: &mut GameState,
    defs: &HashMap<String, (CardDefinition, String)>,
    n: &str,
    cost: u32,
    q: &mut TriggerQueue,
    dm: &mut Dm,
) -> Result<ironsmith::ids::StableId, String> {
    let before = g
        .objects_in_deterministic_order()
        .into_iter()
        .map(|o| o.stable_id)
        .collect::<Vec<_>>();
    dm.stage = format!("paid_cast_{n}");
    let definition = &defs
        .get(n)
        .ok_or_else(|| format!("canonical helper did not compile: {n}"))?
        .0;
    cast_announce(g, definition, cost, q, dm)?;
    dm.stage = format!("resolve_{n}");
    finish(g, q, dm)?;
    g.objects_in_deterministic_order()
        .into_iter()
        .find(|o| o.name == defs[n].0.name() && !before.contains(&o.stable_id))
        .map(|o| o.stable_id)
        .ok_or(format!("cast resource {n} absent after resolution"))
}
fn current(g: &GameState, id: ironsmith::ids::StableId) -> ObjectId {
    g.find_object_by_stable_id(id)
        .expect("current resource incarnation")
}

fn set_actor(g: &mut GameState, dm: &mut Dm, p: PlayerId) {
    dm.actor = p;
    g.turn.active_player = p;
    g.turn.priority_player = Some(p);
    g.turn.phase = Phase::FirstMain;
    g.turn.step = None;
}
fn immediate(
    g: &mut GameState,
    a: LegalAction,
    q: &mut TriggerQueue,
    dm: &mut Dm,
) -> Result<(), String> {
    g.turn.priority_player = Some(dm.actor);
    let mut s = PriorityLoopState::new(g.players_in_game());
    dm.trace
        .push(json!({"stage":dm.stage,"actual_action":format!("{a:?}")}));
    let mut progress =
        apply_priority_response_with_dm(g, q, &mut s, &PriorityResponse::PriorityAction(a), dm)
            .map_err(|e| e.to_string())?;
    for _ in 0..32 {
        if let GameProgress::NeedsDecisionCtx(c) = progress {
            if matches!(c, DecisionContext::Priority(_)) {
                return Ok(());
            }
            progress =
                apply_decision_context_with_dm(g, q, &mut s, &c, dm).map_err(|e| e.to_string())?;
        } else {
            return Ok(());
        }
    }
    Err("immediate bound".into())
}
fn land(
    g: &mut GameState,
    d: &CardDefinition,
    q: &mut TriggerQueue,
    dm: &mut Dm,
) -> Result<ironsmith::ids::StableId, String> {
    let id = g.create_object_from_definition(d, dm.actor, Zone::Hand);
    let stable = g.object(id).unwrap().stable_id;
    let a = compute_legal_actions(g, dm.actor).expect("fixture has complete replacement state")
        .into_iter()
        .find(|a| matches!(a,LegalAction::PlayLand{land_id}if *land_id==id))
        .ok_or_else(|| format!("actual {} land action absent", d.name()))?;
    dm.stage = format!("actual_land_play_{}", d.name());
    immediate(g, a, q, dm)?;
    finish(g, q, dm)?;
    Ok(stable)
}
fn next_main(g: &mut GameState, q: &mut TriggerQueue, dm: &mut Dm) -> Result<(), String> {
    let initial = g.turn.turn_number;
    for _ in 0..100 {
        if g.turn.step == Some(ironsmith::Step::Untap) {
            ironsmith::turn::execute_untap_step(g);
        }
        if g.turn.step == Some(ironsmith::Step::Draw) {
            for e in ironsmith::turn::execute_draw_step_with(g, dm) {
                for t in ironsmith::triggers::check_triggers(g, &e) {
                    q.add(t)
                }
            }
            finish(g, q, dm)?;
        }
        ironsmith::turn::advance_step(g).map_err(|e| e.to_string())?;
        if let Some(e) = ironsmith::triggers::generate_step_trigger_events(g) {
            dm.trace.push(json!({"stage":"actual_step","turn":g.turn.turn_number,"active":g.turn.active_player.0,"phase":format!("{:?}",g.turn.phase),"step":format!("{:?}",g.turn.step)}));
            for t in ironsmith::triggers::check_triggers(g, &e) {
                q.add(t)
            }
            for t in ironsmith::triggers::check_delayed_triggers(g, &e) {
                q.add(t)
            }
            finish(g, q, dm)?;
        }
        if g.turn.turn_number > initial
            && g.turn.active_player == alice()
            && g.turn.phase == Phase::FirstMain
        {
            g.turn.priority_player = Some(alice());
            return Ok(());
        }
    }
    Err("next main phase bound".into())
}
fn refill(g: &mut GameState) {
    for s in [
        ManaSymbol::White,
        ManaSymbol::Blue,
        ManaSymbol::Black,
        ManaSymbol::Red,
        ManaSymbol::Green,
        ManaSymbol::Colorless,
    ] {
        g.player_mut(alice()).unwrap().mana_pool.add(s, 20);
    }
}
fn mana_actions(g: &GameState, source: ObjectId) -> Vec<LegalAction> {
    compute_legal_actions(g, alice()).expect("fixture has complete replacement state")
        .into_iter()
        .filter(|a| matches!(a,LegalAction::ActivateManaAbility{source:s,..}if *s==source))
        .collect()
}
fn pool(g: &GameState) -> Vec<u32> {
    let p = &g.player(alice()).unwrap().mana_pool;
    vec![p.white, p.blue, p.black, p.red, p.green, p.colorless]
}
fn qualifier(
    g: &mut GameState,
    defs: &HashMap<String, (CardDefinition, String)>,
    c: &Value,
    q: &mut TriggerQueue,
    dm: &mut Dm,
) -> Result<ironsmith::ids::StableId, String> {
    let n = c["qualifier"].as_str().unwrap();
    if c["qualifier_is_land"] == true {
        paid_cast(g, defs, "Explore", 2, q, dm)?;
        land(g, &defs[n].0, q, dm)
    } else {
        paid_cast(
            g,
            defs,
            n,
            c["qualifier_cost"].as_u64().unwrap() as u32,
            q,
            dm,
        )
    }
}
fn run(
    defs: &HashMap<String, (CardDefinition, String)>,
    c: &Value,
    dm: &mut Dm,
) -> Result<(Value, Value), String> {
    let n = c["card"].as_str().unwrap();
    let mode = c["mode"].as_str().unwrap();
    let mut g = game();
    let mut q = TriggerQueue::new();
    for p in [alice(), PlayerId(1), PlayerId(2)] {
        for _ in 0..30 {
            g.create_object_from_definition(&defs["Forest"].0, p, Zone::Library);
        }
    }
    let mut support = None;
    if ["present", "present_alternative", "removed", "wrong_type"].contains(&mode) {
        support = Some(qualifier(&mut g, defs, c, &mut q, dm)?);
    }
    if mode.starts_with("hand_") {
        let name = c["qualifier"].as_str().unwrap();
        let id = g.create_object_from_definition(&defs[name].0, alice(), Zone::Hand);
        dm.plot = mode == "hand_reveal";
        dm.chosen = if dm.plot { vec![id] } else { vec![] };
    }
    if mode == "own_turn" {
        for _ in 1..c["own_turn"].as_u64().unwrap() {
            next_main(&mut g, &mut q, dm)?;
        }
        refill(&mut g);
    }
    let source = land(&mut g, &defs[n].0, &mut q, dm)?;
    let sid = current(&g, source);
    let entered_tapped = g.is_tapped(sid);
    let mana_initially_offered = !mana_actions(&g, sid).is_empty();
    let expected_tapped = matches!(
        mode,
        "absent" | "wrong_type" | "gained_after_entry" | "hand_decline"
    ) || (mode == "own_turn" && c["own_turn"].as_u64().unwrap() > 3);
    dm.trace.push(json!({"stage":"entry_observation","source":n,"entered_tapped":entered_tapped,"mana_initially_offered":mana_initially_offered,"global_turn":g.turn.turn_number,"mode":mode}));
    dm.chosen.clear();
    dm.plot = false;
    if mode == "removed" {
        dm.targets = vec![Target::Object(current(&g, support.unwrap()))];
        paid_cast(&mut g, defs, "Boomerang", 2, &mut q, dm)?;
        dm.targets.clear();
        if g.object(current(&g, support.unwrap())).unwrap().zone != Zone::Hand {
            return Err("actual paid qualifier removal did not return it to hand".into());
        }
    }
    if mode == "gained_after_entry" {
        qualifier(&mut g, defs, c, &mut q, dm)?;
        if !g.is_tapped(sid) {
            return Ok((
                json!({"source_stays_tapped_after_qualifier_enters":true}),
                json!({"source_stays_tapped_after_qualifier_enters":false}),
            ));
        }
    }
    if entered_tapped {
        next_main(&mut g, &mut q, dm)?;
    }
    g.player_mut(alice()).unwrap().mana_pool = ironsmith::ManaPool::new();
    dm.stage = "actual_advertised_primary_mana_activation".into();
    let actions = mana_actions(&g, sid);
    let action = actions.first().cloned();
    let before_life = g.player(alice()).unwrap().life;
    let error = if let Some(a) = action {
        immediate(&mut g, a, &mut q, dm).err()
    } else {
        Some("advertised mana action absent after natural untap".into())
    };
    let wanted = c["mana"].as_str().unwrap();
    let mut expected_mana = vec![0; 6];
    expected_mana[["W", "U", "B", "R", "G", "C"]
        .iter()
        .position(|s| *s == wanted)
        .unwrap()] = 1;
    Ok((
        json!({"entered_tapped":expected_tapped,"mana_initially_offered":!expected_tapped,"mana_action_after_boundary":true,"activation_error":null,"source_tapped_after_activation":true,"mana":expected_mana,"life_change":0,"stack_empty":true}),
        json!({"entered_tapped":entered_tapped,"mana_initially_offered":mana_initially_offered,"mana_action_after_boundary":!actions.is_empty(),"activation_error":error,"source_tapped_after_activation":g.is_tapped(sid),"mana":pool(&g),"life_change":g.player(alice()).unwrap().life-before_life,"stack_empty":g.stack.is_empty()}),
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
    let input = p.join("conditional-land-entry-inputs.json");
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
            Err(e) => {
                rows.push(
                    json!({"card":n,"status":"compile_failed","actual":{"error":e.to_string()}}),
                );
            }
        }
    }
    for c in data["cases"].as_array().unwrap() {
        let n = c["card"].as_str().unwrap();
        eprintln!("CONDITIONAL_LAND_CASE {c}");
        let mut dm = Dm {
            actor: alice(),
            targets: vec![],
            stage: "fixture".into(),
            trace: vec![],
            chosen: vec![],
            reverse: false,
            x: 0,
            plot: false,
            option_text: String::new(),
            mana_color: match c["mana"].as_str().unwrap() {
                "W" => ironsmith::color::Color::White,
                "U" => ironsmith::color::Color::Blue,
                "B" => ironsmith::color::Color::Black,
                "R" => ironsmith::color::Color::Red,
                _ => ironsmith::color::Color::Green,
            },
        };
        let result =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run(&defs, c, &mut dm)));
        let (status, expected, actual) = match result {
            Ok(Ok((e, a))) => (
                if e == a {
                    "expected_result_observed"
                } else if !a["announcement_error"].is_null() || !a["resolution_error"].is_null() {
                    "resolution_failed"
                } else {
                    "semantic_mismatch"
                },
                e,
                a,
            ),
            Ok(Err(e)) => (
                "execution_or_fixture_error",
                Value::Null,
                json!({"error":e}),
            ),
            Err(_) => (
                "panicked",
                Value::Null,
                json!({"error":"fixture or runtime panic; inspect isolated row trace"}),
            ),
        };
        rows.push(json!({"card":n,"scenario":c,"status":status,"expected":expected,"actual":actual,"artifact_checksum":defs.get(n).map(|x|&x.1),"execution_trace":dm.trace}));
    }
    let report = json!({"scope":"Forty assigned canonical conditional-entry lands. Actual land plays, full paid qualifier spells/Explore-enabled qualifier lands, real paid Boomerang removal, actual turn transitions and advertised primary mana activations. Entry tapped-state and exact color/tap/life outcomes; no absent action forced.","limitations":"Initial mana, canonical hand/library cards and first-main entry are fixtures. Turn number starts1 and all later turns advance through normal phase functions; no condition flags are injected. Qualifiers and removal pay full printed costs. The primary advertised mana action only is checked: secondary activations and Chocobo delayed Bird effect are not certified. Tapped entries use actual next-turn untap before activation. Reveal acceptance/decline and two types of an OR are explicit where present.","provenance":{"binary":std::env::current_exe().unwrap(),"binary_sha256":std::env::var("AUDIT_BINARY_SHA256").unwrap(),"source_sha256":hash(&root.join("crates/ironsmith-tools/tests/runtime_conditional_land_entry_reproductions.rs")),"input_sha256":hash(&input),"seed":SEED,"unique_card_ids":true,"compiled_via":"ironsmith_registry::compile_builder_to_artifact"},"rows":rows});
    std::fs::write(
        p.join("conditional-land-entry-artifacts.json"),
        serde_json::to_string_pretty(&artifacts).unwrap(),
    )
    .unwrap();
    std::fs::write(
        p.join("conditional-land-entry-reproductions.json"),
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
}
#[test]
#[ignore = "canonical Escape additional-cost expected-result reporter"]
fn report_conditional_land_entry() {
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(generate)
        .unwrap()
        .join()
        .unwrap();
}
