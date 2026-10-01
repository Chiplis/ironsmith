//! Optional waterbend spell payments and exact resolving outcomes.
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
    optional_requested: bool,
    optional_offered: Option<bool>,
    optional_selected: bool,
    target: Option<Target>,
    trace: Vec<Value>,
    resources: Vec<ObjectId>,
    x: u32,
    activation_hint: Option<&'static str>,
}
impl DecisionMaker for Choices {
    fn decide_number(
        &mut self,
        _: &GameState,
        c: &ironsmith::decisions::context::NumberContext,
    ) -> u32 {
        let x = self.x.clamp(c.min, c.max);
        self.trace
            .push(json!({"decision":"number","context":format!("{c:?}"),"answer":x}));
        x
    }
    fn answers_player_choices(&self) -> bool {
        true
    }
    fn decide_boolean(&mut self, _: &GameState, c: &BooleanContext) -> bool {
        self.trace
            .push(json!({"decision":"boolean","context":format!("{c:?}"),"answer":self.accept}));
        self.accept
    }
    fn decide_options(&mut self, g: &GameState, c: &SelectOptionsContext) -> Vec<usize> {
        let a = if c.description.starts_with("Choose optional costs") {
            self.optional_offered = Some(c.options.iter().any(|o| o.index == 0 && o.legal));
            let choice = if self.optional_requested {
                c.options
                    .iter()
                    .filter(|o| o.index == 0 && o.legal)
                    .map(|o| o.index)
                    .collect::<Vec<_>>()
            } else {
                vec![]
            };
            self.optional_selected = !choice.is_empty();
            eprintln!(
                "AUDIT_STAGE optional offered={:?} selected={choice:?}",
                self.optional_offered
            );
            choice
        } else if let Some(o) = c.options.iter().find(|o| o.legal && o.description == "Tap") {
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
            dm.trace.push(json!({"stage":"cast_complete","card":def.name(),"paid":paid,"stack_names":g.stack.iter().filter_map(|e|g.object(e.object_id).map(|o|o.name.to_string())).collect::<Vec<_>>()}));
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
        ironsmith::game_loop::advance_priority_with_dm(g, q, dm).map_err(|e| e.to_string())?;
        drain(g, q, dm)?;
        if g.stack.is_empty() {
            return Ok(());
        }
        dm.trace
            .push(json!({"stage":"stack_before_resolution","entries":g.stack.iter().map(|e|json!({"ability":e.is_ability,"source_name":e.source_name.as_ref().map(|s|s.to_string())})).collect::<Vec<_>>()}));
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
    for p in (0..g.players_in_game()).map(|n| PlayerId(n as u8)) {
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

fn run(
    def: &CardDefinition,
    defs: &std::collections::HashMap<&str, CardDefinition>,
    mode: usize,
) -> Result<Value, String> {
    let name = def.name();
    let amount = if name == "Katara, Seeking Revenge" {
        2
    } else if name == "Ruinous Waterbending" {
        4
    } else {
        6
    };
    let (branch, count, request, kind) = if mode <= amount {
        (mode, mode, true, "funded")
    } else if mode == amount + 1 {
        (amount, amount, false, "decline")
    } else if mode == amount + 2 {
        (amount, amount - 1, true, "insufficient")
    } else {
        (amount, amount + 1, true, "surplus")
    };
    let paid_expected = request && kind != "insufficient";
    let base = if amount == 2 { 4 } else { 3 };
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
    for p in [PlayerId(0), PlayerId(1)] {
        for _ in 0..20 {
            g.create_object_from_definition(&filler, p, Zone::Library);
        }
    }
    let mut q = TriggerQueue::new();
    let mut dm = Choices {
        accept: true,
        optional_requested: false,
        optional_offered: None,
        optional_selected: false,
        target: None,
        trace: vec![],
        resources: vec![],
        x: 0,
        activation_hint: None,
    };
    let mut producers = vec![];
    let mut giant = None;
    if amount == 4 {
        let cost = cast(&mut g, &defs["Hill Giant"], PlayerId(0), &mut q, &mut dm)?;
        if cost != 4 {
            return Err("Giant paid4".into());
        }
        resolve_all(&mut g, &mut q, &mut dm)?;
        let id = find(&g, "Hill Giant")?;
        dm.target = Some(Target::Object(id));
        let cost = cast(&mut g, &defs["Twiddle"], PlayerId(0), &mut q, &mut dm)?;
        if cost != 1 {
            return Err("Twiddle paid1".into());
        }
        // Tap mode explicitly selected by this fixture's option policy below.
        resolve_all(&mut g, &mut q, &mut dm)?;
        if !g.is_tapped(id) {
            return Err("Giant must be tapped before optional payment".into());
        }
        giant = Some(id);
        producers.push(json!({"producer":"paid Giant4 + Twiddle1","id":id.0}));
    }
    if amount == 6 {
        dm.target = Some(Target::Player(PlayerId(1)));
        for _ in 0..2 {
            let cost = cast(&mut g, &defs["Shock"], PlayerId(0), &mut q, &mut dm)?;
            if cost != 1 {
                return Err("Shock paid1".into());
            }
            resolve_all(&mut g, &mut q, &mut dm)?;
        }
        if g.player(PlayerId(0)).unwrap().graveyard.len() != 2
            || g.player(PlayerId(1)).unwrap().life != 16
        {
            return Err("real graveyard producers".into());
        }
        producers.push(json!({"producer":"two paid Shock1","bob_life":16,"graveyard":2}));
    }
    let mut resources = vec![];
    for n in 0..count {
        let resource = if amount == 4 || n % 2 == 0 {
            "Shuko"
        } else {
            "Grizzly Bears"
        };
        let old = g.battlefield.clone();
        let cost = cast(&mut g, &defs[resource], PlayerId(0), &mut q, &mut dm)?;
        if cost != if resource == "Shuko" { 1 } else { 2 } {
            return Err("resource real cost".into());
        }
        resolve_all(&mut g, &mut q, &mut dm)?;
        let id = *g
            .battlefield
            .iter()
            .find(|id| !old.contains(id) && g.object(**id).is_some_and(|o| o.name == resource))
            .ok_or("paid resource absent")?;
        resources.push(id);
        producers.push(json!({"card":resource,"id":id.0,"paid":cost}));
    }
    if resources
        .iter()
        .any(|id| g.is_tapped(*id) || g.current_controller(*id) != Some(PlayerId(0)))
    {
        return Err("resource readiness".into());
    }
    let old_hand = g.player(PlayerId(0)).unwrap().hand.len();
    if old_hand != 0 {
        return Err("unexpected pre-source hand".into());
    }
    dm.target = None;
    dm.resources = resources.clone();
    dm.optional_requested = request;
    dm.optional_offered = None;
    dm.optional_selected = false;
    let colored = if amount == 2 { 1 } else { 2 };
    let color = if amount == 4 {
        ManaSymbol::Black
    } else {
        ManaSymbol::Blue
    };
    g.player_mut(PlayerId(0)).unwrap().mana_pool = Default::default();
    g.player_mut(PlayerId(0))
        .unwrap()
        .mana_pool
        .add(color, colored);
    g.player_mut(PlayerId(0)).unwrap().mana_pool.add(
        ManaSymbol::Colorless,
        (base - colored) + (amount - branch) as u32,
    );
    eprintln!(
        "AUDIT_STAGE source announcement mode{mode} branch{branch} resources{count} expected_optional{paid_expected}"
    );
    let before_cast = json!({"mana":format!("{:?}",g.player(PlayerId(0)).unwrap().mana_pool),"resources":resources.iter().map(|id|json!({"id":id.0,"name":g.object(*id).unwrap().name.to_string(),"tapped":g.is_tapped(*id),"controller":g.current_controller(*id).map(|p|p.index()),"zone":format!("{:?}",g.object(*id).unwrap().zone)})).collect::<Vec<_>>(),"source_mana_value":base,"optional_amount":amount,"base_plus_additional_capacity":g.player(PlayerId(0)).unwrap().mana_pool.total() as usize+resources.len()});
    let call = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        cast(&mut g, def, PlayerId(0), &mut q, &mut dm)
    }));
    let mut panic = None;
    let result = match call {
        Ok(r) => r,
        Err(payload) => {
            let message = payload
                .downcast_ref::<&str>()
                .map(|s| s.to_string())
                .or_else(|| payload.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "non-string native panic".into());
            panic = Some(message.clone());
            Err(format!("native panic: {message}"))
        }
    };
    let payment_taps = resources.iter().filter(|id| g.is_tapped(**id)).count();
    let actual_optional = dm.optional_selected;
    let optional_offered = dm.optional_offered;
    let mut actual = json!({"announcement_error":result.as_ref().err(),"resolution_error":null,"optional_selected":actual_optional,"mana_paid":result.as_ref().ok(),"payment_resource_taps":payment_taps,"remaining_stack":g.stack.len()});
    actual["panic"] = json!(panic);
    let mut expected = json!({"announcement_error":null,"panic":null,"resolution_error":null,"optional_selected":paid_expected,"mana_paid":base+if paid_expected{(amount-branch) as u32}else{0},"payment_resource_taps":if paid_expected{branch}else{0},"remaining_stack":0});
    let entry = g
        .stack
        .iter()
        .find(|e| !e.is_ability && g.object(e.object_id).is_some_and(|o| o.name == name));
    let announced_optional = entry.map(|e| format!("{:?}", e.optional_costs_paid));
    let announced_object_optional = entry
        .and_then(|e| g.object(e.object_id))
        .map(|o| format!("{:?}", o.optional_costs_paid));
    if result.is_ok() {
        actual["resolution_error"] = json!(resolve_all(&mut g, &mut q, &mut dm).err());
    }
    if actual["announcement_error"].is_null() && actual["resolution_error"].is_null() {
        if amount == 2 {
            actual["hand"] = json!(g.player(PlayerId(0)).unwrap().hand.len());
            expected["hand"] = json!(usize::from(paid_expected));
            actual["graveyard"] = json!(g.player(PlayerId(0)).unwrap().graveyard.len());
            expected["graveyard"] = json!(usize::from(!paid_expected));
            actual["source_battlefield"] = json!(find(&g, name).is_ok());
            expected["source_battlefield"] = json!(true);
        } else if amount == 4 {
            let id = giant.unwrap();
            actual["giant_debuff"] = json!([g.current_power(id), g.current_toughness(id)]);
            expected["giant_debuff"] = json!([Some(1), Some(1)]);
            dm.optional_requested = false;
            mana(&mut g);
            dm.target = Some(Target::Object(id));
            let cost = cast(&mut g, &defs["Shock"], PlayerId(0), &mut q, &mut dm)?;
            if cost != 1 {
                return Err("post-spell actual Shock1".into());
            }
            resolve_all(&mut g, &mut q, &mut dm)?;
            actual["affected_giant_deaths"] = json!(
                g.player(PlayerId(0))
                    .unwrap()
                    .graveyard
                    .iter()
                    .filter(|id| g.object(**id).is_some_and(|o| o.name == "Hill Giant"))
                    .count()
            );
            expected["affected_giant_deaths"] = json!(1);
            actual["life"] = json!(g.player(PlayerId(0)).unwrap().life);
            expected["life"] = json!(if paid_expected { 21 } else { 20 });
        } else {
            actual["hand"] = json!(g.player(PlayerId(0)).unwrap().hand.len());
            expected["hand"] = json!(if paid_expected { 7 } else { 2 });
            actual["library"] = json!(g.player(PlayerId(0)).unwrap().library.len());
            expected["library"] = json!(if paid_expected { 15 } else { 18 });
            actual["graveyard"] = json!(g.player(PlayerId(0)).unwrap().graveyard.len());
            expected["graveyard"] = json!(if paid_expected { 0 } else { 2 });
            actual["max_hand_size"] = json!(g.player(PlayerId(0)).unwrap().max_hand_size);
            expected["max_hand_size"] = json!(if paid_expected { i32::MAX } else { 7 });
            actual["source_exiled"] = json!(
                g.exile
                    .iter()
                    .any(|id| g.object(*id).is_some_and(|o| o.name == name))
            );
            expected["source_exiled"] = json!(true);
        }
    }
    actual["remaining_stack"] = json!(g.stack.len());
    Ok(
        json!({"expected":expected,"actual":actual,"state_evidence":{"before_source_announcement":before_cast,"amount":amount,"branch":branch,"kind":kind,"resource_count":count,"optional_offered":optional_offered,"announced_optional":announced_optional,"announced_object_optional":announced_object_optional,"producers":producers,"scope":"Actual paid resources; exact optional index0 selected only if legal. OneOf remains structured; no unavailable option forced. Taps measured before source resolution."},"execution_trace":dm.trace}),
    )
}
#[test]
#[ignore = "manual scoped conditional index state transition report"]
fn report_optional_waterbend() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let inventory =
        root.join("reports/runtime-audit/corpus/267a16aff3b321196397d0b4/inventory.json");
    let source =
        root.join("crates/ironsmith-tools/tests/runtime_optional_waterbend_reproductions.rs");
    let binary = std::env::current_exe().unwrap();
    let paths = [&inventory, &source, &binary];
    let before: Vec<_> = paths
        .iter()
        .map(|p| json!({"path":p,"sha256":hash(p)}))
        .collect();
    let inv: Value = serde_json::from_slice(&std::fs::read(&inventory).unwrap()).unwrap();
    let names = [
        "Katara, Seeking Revenge",
        "Ruinous Waterbending",
        "Spirit Water Revival",
    ];
    let mut compile = vec![];
    let mut defs = std::collections::HashMap::new();
    for name in
        names
            .into_iter()
            .chain(["Shock", "Shuko", "Grizzly Bears", "Hill Giant", "Twiddle"])
    {
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
        for mode in 0..(if name == "Katara, Seeking Revenge" {
            6
        } else if name == "Ruinous Waterbending" {
            8
        } else {
            10
        }) {
            if std::env::var("AUDIT_STATIC_MODE").is_ok_and(|n| n != mode.to_string()) {
                continue;
            }
            eprintln!("AUDIT_CASE {name} {mode}");
            let (status, out) = match run(&defs[name], &defs, mode) {
                Ok(out) => {
                    let status = if out["actual"]["panic"].is_string() {
                        "panicked"
                    } else if out["actual"]["announcement_error"].is_string() {
                        "action_or_choice_failed"
                    } else if out["actual"]["resolution_error"].is_string() {
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
            rows.push(json!({"card":name,"scenario":{"mode":mode},"status":status,"expected":out["expected"],"actual":out["actual"],"state_evidence":out["state_evidence"],"execution_trace":out["execution_trace"],"artifact_checksum":compile.iter().find(|c|c["card"]==name).unwrap()["artifact_checksum"],"scope":"Actual optional waterbend cast with legal index0, exact structured OneOf resource/mana payment, and independent printed draw/discard, debuff/death/life, or shuffle/draw/no-hand-limit outcomes."}));
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
            root.join("reports/runtime-audit/optional-waterbend-reproductions.json")
        });
    std::fs::write(out, serde_json::to_string_pretty(&report).unwrap()).unwrap();
    println!("wrote{}cases", report["rows"].as_array().unwrap().len());
}
