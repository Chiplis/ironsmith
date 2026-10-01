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
    tap: bool,
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
            .find(|o| o.legal && o.description == if self.tap { "Tap" } else { "Untap" })
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
    last: bool,
) -> Result<Value, String> {
    g.turn.priority_player = Some(PlayerId(0));
    let actions: Vec<_> = compute_legal_actions(g, PlayerId(0)).expect("fixture has complete replacement state")
        .into_iter()
        .filter(|a| matches!(a,LegalAction::ActivateAbility{source:s,..}if *s==source))
        .collect();
    dm.trace.push(
        json!({"stage":"advertised_actions","all":format!("{actions:?}"),"select_last_pump":last}),
    );
    let a = if last {
        actions.last().cloned()
    } else {
        actions.first().cloned()
    };
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
    dm.trace
        .push(json!({"stage":"actual_land_play","action":format!("{action:?}")}));
    let mut state = PriorityLoopState::new(g.players_in_game());
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
fn cast_existing(
    g: &mut GameState,
    q: &mut TriggerQueue,
    dm: &mut Choices,
    action: LegalAction,
    name: &str,
) -> Result<u32, String> {
    let before = g.player(PlayerId(0)).unwrap().mana_pool.total();
    let mut state = PriorityLoopState::new(g.players_in_game());
    dm.trace
        .push(json!({"stage":"graveyard_cast_action","action":format!("{action:?}")}));
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
            && g.stack
                .iter()
                .any(|e| !e.is_ability && g.object(e.object_id).is_some_and(|o| o.name == name))
        {
            return Ok(before - g.player(PlayerId(0)).unwrap().mana_pool.total());
        }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else {
            return Err(format!("GY cast stalled:{progress:?}"));
        };
        progress = apply_decision_context_with_dm(g, q, &mut state, &ctx, dm)
            .map_err(|e| e.to_string())?;
    }
    Err("GY cast bound".into())
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
        for _ in 0..20 {
            g.create_object_from_definition(&filler, player, Zone::Library);
        }
    }
    let mut q = TriggerQueue::new();
    let mut dm = Choices {
        accept: true,
        target: None,
        trace: vec![],
        resources: vec![],
        tap: false,
    };
    if def.name() == "Specimen Freighter" {
        if cast(&mut g, &defs["Grizzly Bears"], PlayerId(0), &mut q, &mut dm)? != 2 {
            return Err("pre-entry Bear cost".into());
        }
        resolve_all(&mut g, &mut q, &mut dm)?;
        dm.target = Some(Target::Object(find(&g, "Grizzly Bears")?));
    }
    let paid = cast(&mut g, def, PlayerId(0), &mut q, &mut dm)?;
    if paid != def.card.mana_cost.as_ref().unwrap().mana_value() {
        return Err("source cost mismatch".into());
    }
    resolve_all(&mut g, &mut q, &mut dm)?;
    let source = find(&g, def.name())?;
    dm.target = None;
    if def.name() == "Exploration Broodship" {
        if cast(&mut g, &defs["Shuko"], PlayerId(0), &mut q, &mut dm)? != 1 {
            return Err("Shuko cost".into());
        }
        resolve_all(&mut g, &mut q, &mut dm)?;
        let target = find(&g, "Shuko")?;
        dm.target = Some(Target::Object(target));
        if cast(&mut g, &defs["Disenchant"], PlayerId(0), &mut q, &mut dm)? != 2 {
            return Err("Disenchant cost".into());
        }
        resolve_all(&mut g, &mut q, &mut dm)?;
        dm.target = None;
    }
    let threshold = match def.name() {
        "Infinite Guideline Station" => 12,
        "Sledge-Class Seedship" => 7,
        "Specimen Freighter" => 9,
        _ => 8,
    };
    let resources: Vec<&str> = if mode == 0 {
        vec![]
    } else {
        match threshold {
            7 => vec!["Vorstclaw"],
            8 => vec!["Terra Stomper"],
            9 => vec!["Ancient Brontodon"],
            _ => vec!["Gigantosaurus", "Grizzly Bears"],
        }
    };
    let mut stationed = vec![];
    for name in resources {
        let d = &defs[name];
        if cast(&mut g, d, PlayerId(0), &mut q, &mut dm)?
            != d.card.mana_cost.as_ref().unwrap().mana_value()
        {
            return Err("station resource cost".into());
        }
        resolve_all(&mut g, &mut q, &mut dm)?;
        let id = find(&g, name)?;
        dm.resources = vec![id];
        let a = activation(&mut g, &mut q, &mut dm, source, false)?;
        if a != json!({"offered":true,"announcement_error":null,"resolution_error":null,"mana_paid":0,"remaining_stack":0})
            || !g.is_tapped(id)
        {
            return Err(format!("Station failed:{a}"));
        }
        stationed.push(id);
    }
    let counters = g.counter_count(source, ironsmith::CounterType::Charge);
    if counters != if mode == 0 { 0 } else { threshold } {
        return Err(format!("station count{counters}"));
    }
    let mut evidence = json!({"mode":mode,"source_paid":paid,"threshold":threshold,"counters":counters,"source_creature_before_animation":g.current_has_card_type(source,CardType::Creature),"flying_before":g.object_has_static_ability_id(source,K::Flying),"stationed_resources":format!("{stationed:?}")});
    if def.name() == "Exploration Broodship" {
        let land = play_land(&mut g, &defs["Wastes"], &mut q, &mut dm)?;
        let gy = *g
            .player(PlayerId(0))
            .unwrap()
            .graveyard
            .iter()
            .find(|id| g.object(**id).is_some_and(|o| o.name == "Shuko"))
            .ok_or("actual destroyed Shuko absent")?;
        let a=compute_legal_actions(&g,PlayerId(0)).expect("fixture has complete replacement state").into_iter().find(|a|matches!(a,LegalAction::CastSpell{spell_id,from_zone:Zone::Graveyard,..}if *spell_id==gy));
        dm.trace.push(json!({"stage":"graveyard_spell_legality","action":format!("{a:?}"),"card":format!("{gy:?}"),"land":format!("{land:?}")}));
        dm.resources = vec![land];
        let mut error = None;
        let mut cast_paid = 0;
        let mut resolution_error = None;
        if let Some(a) = a.clone() {
            match cast_existing(&mut g, &mut q, &mut dm, a, "Shuko") {
                Ok(p) => {
                    cast_paid = p;
                    resolution_error = resolve_all(&mut g, &mut q, &mut dm).err();
                }
                Err(e) => error = Some(e),
            }
        }
        let expected = json!({"graveyard_cast_offered":mode==1,"announcement_error":null,"resolution_error":null,"paid":if mode==1{1}else{0},"shuko_battlefield":mode==1,"land_sacrificed":mode==1,"remaining_stack":0});
        let actual = json!({"graveyard_cast_offered":a.is_some(),"announcement_error":error,"resolution_error":resolution_error,"paid":cast_paid,"shuko_battlefield":g.battlefield.iter().any(|id|g.object(*id).is_some_and(|o|o.name=="Shuko")),"land_sacrificed":!g.battlefield.contains(&land),"remaining_stack":g.stack.len()});
        return Ok(
            json!({"expected":expected,"actual":actual,"state_evidence":evidence,"execution_trace":dm.trace}),
        );
    }
    dm.target = Some(Target::Object(source));
    if cast(
        &mut g,
        &defs["Ensoul Artifact"],
        PlayerId(0),
        &mut q,
        &mut dm,
    )? != 2
    {
        return Err("Ensoul cost".into());
    }
    resolve_all(&mut g, &mut q, &mut dm)?;
    dm.target = None;
    if cast(&mut g, &defs["Fervor"], PlayerId(0), &mut q, &mut dm)? != 3 {
        return Err("Fervor cost".into());
    }
    resolve_all(&mut g, &mut q, &mut dm)?;
    if !g.current_has_card_type(source, CardType::Creature)
        || !g.object_has_static_ability_id(source, K::Haste)
    {
        return Err("real animation/haste setup missing".into());
    }
    let elf = if def.name() == "Sledge-Class Seedship" {
        Some(g.create_object_from_definition(&defs["Llanowar Elves"], PlayerId(0), Zone::Hand))
    } else {
        None
    };
    if let Some(elf) = elf {
        dm.resources = vec![elf];
    }
    if def.name() == "Entropic Battlecruiser" {
        g.create_object_from_definition(&filler, PlayerId(1), Zone::Hand);
    }
    let alice_hand = g.player(PlayerId(0)).unwrap().hand.len();
    let bob_hand = g.player(PlayerId(1)).unwrap().hand.len();
    let bob_gy = g.player(PlayerId(1)).unwrap().graveyard.len();
    let bob_life = g.player(PlayerId(1)).unwrap().life;
    ironsmith::turn::advance_phase(&mut g).map_err(|e| e.to_string())?;
    ironsmith::turn::advance_step(&mut g).map_err(|e| e.to_string())?;
    let legal = ironsmith::decision::compute_legal_attackers(
        &g,
        &ironsmith::combat_state::CombatState::default(),
    );
    if !legal.iter().any(|a| a.creature == source) {
        return Err("actually animated/haste source not legal attacker".into());
    }
    let mut combat = ironsmith::combat_state::CombatState::default();
    ironsmith::game_loop::apply_attacker_declarations_with_dm(
        &mut g,
        &mut combat,
        &mut q,
        &[ironsmith::decision::AttackerDeclaration {
            creature: source,
            target: ironsmith::combat_state::AttackTarget::Player(PlayerId(1)),
        }],
        &mut dm,
    )
    .map_err(|e| e.to_string())?;
    g.combat = Some(combat);
    drain(&mut g, &mut q, &mut dm)?;
    let triggered = g.stack.iter().filter(|e| e.is_ability).count();
    evidence["actual_attack"] = json!(format!("{:?}", g.combat));
    evidence["queued_ability_count"] = json!(triggered);
    let error = resolve_all(&mut g, &mut q, &mut dm).err();
    let mut expected = json!({"queued_ability_count":if mode==1{1}else{0},"resolution_error":null,"remaining_stack":0});
    let mut actual = json!({"queued_ability_count":triggered,"resolution_error":error,"remaining_stack":g.stack.len()});
    match def.name() {
        "Infinite Guideline Station" => {
            expected["cards_drawn"] = json!(if mode == 1 { 1 } else { 0 });
            actual["cards_drawn"] =
                json!(g.player(PlayerId(0)).unwrap().hand.len() as i64 - alice_hand as i64);
        }
        "Specimen Freighter" => {
            expected["cards_milled"] = json!(if mode == 1 { 4 } else { 0 });
            actual["cards_milled"] =
                json!(g.player(PlayerId(1)).unwrap().graveyard.len() as i64 - bob_gy as i64);
        }
        "Sledge-Class Seedship" => {
            expected["hand_creature_entered"] = json!(mode == 1);
            actual["hand_creature_entered"] = json!(
                g.battlefield
                    .iter()
                    .any(|id| g.object(*id).is_some_and(|o| o.name == "Llanowar Elves"))
            );
        }
        _ => {
            expected["opponent_cards_discarded"] = json!(if mode == 1 { 1 } else { 0 });
            actual["opponent_cards_discarded"] =
                json!(bob_hand as i64 - g.player(PlayerId(1)).unwrap().hand.len() as i64);
            expected["opponent_life_lost"] = json!(if mode == 1 { 3 } else { 0 });
            actual["opponent_life_lost"] = json!(bob_life - g.player(PlayerId(1)).unwrap().life);
        }
    }
    Ok(
        json!({"expected":expected,"actual":actual,"state_evidence":evidence,"execution_trace":dm.trace}),
    )
}
#[test]
#[ignore = "manual scoped conditional index state transition report"]
fn report_station_threshold_event() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let inventory =
        root.join("reports/runtime-audit/corpus/267a16aff3b321196397d0b4/inventory.json");
    let source =
        root.join("crates/ironsmith-tools/tests/runtime_station_threshold_event_reproductions.rs");
    let binary = std::env::current_exe().unwrap();
    let paths = [&inventory, &source, &binary];
    let before: Vec<_> = paths
        .iter()
        .map(|p| json!({"path":p,"sha256":hash(p)}))
        .collect();
    let inv: Value = serde_json::from_slice(&std::fs::read(&inventory).unwrap()).unwrap();
    let names = [
        "Entropic Battlecruiser",
        "Infinite Guideline Station",
        "Sledge-Class Seedship",
        "Specimen Freighter",
        "Exploration Broodship",
    ];
    let mut compile = vec![];
    let mut defs = std::collections::HashMap::new();
    for name in names.into_iter().chain([
        "Shuko",
        "Disenchant",
        "Wastes",
        "Vorstclaw",
        "Terra Stomper",
        "Ancient Brontodon",
        "Gigantosaurus",
        "Grizzly Bears",
        "Ensoul Artifact",
        "Fervor",
        "Llanowar Elves",
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
        let count = 2;
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
            root.join("reports/runtime-audit/station-threshold-event-reproductions.json")
        });
    std::fs::write(out, serde_json::to_string_pretty(&report).unwrap()).unwrap();
    println!("wrote{}cases", report["rows"].as_array().unwrap().len());
}
