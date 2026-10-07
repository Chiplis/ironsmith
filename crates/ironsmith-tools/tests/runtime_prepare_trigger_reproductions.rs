//! Canonical trigger preparation and paid spell cast-completion audit.

#[path = "support/canonical_linked_fixture.rs"]
mod canonical_linked_fixture;
use canonical_linked_fixture::LinkedFamily;
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
        self.trace.push(json!({"stage":self.stage,"choice":"number","context":format!("{c:?}"),"selected":self.x}));
        self.x
    }
    fn decide_objects(&mut self, g: &GameState, c: &SelectObjectsContext) -> Vec<ObjectId> {
        let selected = if (self.stage == "cast_prepared_spell"
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
    g.turn.turn_number = 3;
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
    let mut state = PriorityLoopState::new(g.players_in_game());
    dm.trace
        .push(json!({"stage":dm.stage,"action":format!("{a:?}")}));
    let mut p =
        apply_priority_response_with_dm(g, q, &mut state, &PriorityResponse::PriorityAction(a), dm)
            .map_err(|e| e.to_string())?;
    for _ in 0..32 {
        if let GameProgress::NeedsDecisionCtx(c) = p {
            if matches!(c, DecisionContext::Priority(_)) {
                return Ok(());
            }
            p = apply_decision_context_with_dm(g, q, &mut state, &c, dm)
                .map_err(|e| e.to_string())?;
        } else {
            return Ok(());
        }
    }
    Err("immediate action bound".into())
}
fn paid_land(
    g: &mut GameState,
    d: &CardDefinition,
    q: &mut TriggerQueue,
    dm: &mut Dm,
) -> Result<ironsmith::ids::StableId, String> {
    let id = g.create_object_from_definition(d, alice(), Zone::Hand);
    let s = g.object(id).unwrap().stable_id;
    let a = compute_legal_actions(g, alice()).expect("fixture has complete replacement state")
        .into_iter()
        .find(|a| matches!(a,LegalAction::PlayLand{land_id}if *land_id==id))
        .ok_or("source land play absent")?;
    dm.stage = "actual_source_land_play".into();
    immediate(g, a, q, dm)?;
    finish(g, q, dm)?;
    Ok(s)
}
fn sacrifice(
    g: &mut GameState,
    altar: ironsmith::ids::StableId,
    m: ironsmith::ids::StableId,
    q: &mut TriggerQueue,
    dm: &mut Dm,
) -> Result<(), String> {
    dm.chosen = vec![current(g, m)];
    dm.targets = vec![Target::Player(PlayerId(2))];
    let a=compute_legal_actions(g,dm.actor).expect("fixture has complete replacement state").into_iter().find(|a|matches!(a,LegalAction::ActivateAbility{source,ability_index:0,..}if *source==current(g,altar))).ok_or("Altar producer absent")?;
    announce(g, a, q, dm)?;
    finish(g, q, dm)?;
    dm.targets.clear();
    dm.chosen.clear();
    Ok(())
}
fn seed_mana(g: &mut GameState) {
    for p in [alice(), PlayerId(1)] {
        for s in [
            ManaSymbol::White,
            ManaSymbol::Blue,
            ManaSymbol::Black,
            ManaSymbol::Red,
            ManaSymbol::Green,
            ManaSymbol::Colorless,
        ] {
            g.player_mut(p).unwrap().mana_pool.add(s, 30);
        }
    }
}
fn emit_phase(g: &mut GameState, q: &mut TriggerQueue, dm: &mut Dm) -> Result<(), String> {
    if let Some(e) = ironsmith::triggers::generate_step_trigger_events(g) {
        dm.trace.push(json!({"stage":"actual_phase_event","phase":format!("{:?}",g.turn.phase),"step":format!("{:?}",g.turn.step),"active":g.turn.active_player.0,"hands":g.players.iter().map(|p|p.hand.len()).collect::<Vec<_>>(),"main_phase_ordinal":g.turn_store.main_phases_started_this_turn}));
        for t in ironsmith::triggers::check_triggers(g, &e) {
            q.add(t);
        }
        for t in ironsmith::triggers::check_delayed_triggers(g, &e) {
            q.add(t);
        }
    }
    finish(g, q, dm)
}
fn step(g: &mut GameState, q: &mut TriggerQueue, dm: &mut Dm) -> Result<(), String> {
    if g.turn.step == Some(ironsmith::Step::Untap) {
        ironsmith::turn::execute_untap_step(g);
    }
    if g.turn.step == Some(ironsmith::Step::Draw) {
        for e in ironsmith::turn::execute_draw_step_with(g, dm).unwrap() {
            for t in ironsmith::triggers::check_triggers(g, &e) {
                q.add(t);
            }
        }
        finish(g, q, dm)?;
    }
    ironsmith::turn::advance_step(g).map_err(|e| e.to_string())?;
    emit_phase(g, q, dm)
}
fn phase(g: &mut GameState, want: Phase, q: &mut TriggerQueue, dm: &mut Dm) -> Result<(), String> {
    for _ in 0..80 {
        if g.turn.phase == want {
            return Ok(());
        }
        step(g, q, dm)?;
    }
    Err("phase bound".into())
}
fn own_main(
    g: &mut GameState,
    force_next: bool,
    q: &mut TriggerQueue,
    dm: &mut Dm,
) -> Result<(), String> {
    let starting_turn = g.turn.turn_number;
    for _ in 0..120 {
        if g.turn.active_player == alice()
            && if force_next {
                g.turn.turn_number > starting_turn && g.turn.phase == Phase::FirstMain
            } else {
                matches!(g.turn.phase, Phase::FirstMain | Phase::NextMain)
            }
        {
            seed_mana(g);
            g.turn.priority_player = Some(alice());
            return Ok(());
        }
        step(g, q, dm)?;
    }
    Err("own main bound".into())
}
fn combat_producer(
    g: &mut GameState,
    attackers: Vec<ObjectId>,
    damage: bool,
    q: &mut TriggerQueue,
    dm: &mut Dm,
) -> Result<(), String> {
    phase(g, Phase::Combat, q, dm)?;
    while g.turn.step != Some(ironsmith::Step::DeclareAttackers) {
        step(g, q, dm)?;
    }
    for id in &attackers {
        g.remove_summoning_sickness(*id);
    }
    let mut combat = ironsmith::combat_state::CombatState::default();
    let declarations = attackers
        .iter()
        .map(|id| ironsmith::decision::AttackerDeclaration {
            creature: *id,
            target: ironsmith::combat_state::AttackTarget::Player(PlayerId(1)),
        })
        .collect::<Vec<_>>();
    dm.stage = "actual_attack_declarations".into();
    ironsmith::game_loop::apply_attacker_declarations_with_dm(g, &mut combat, q, &declarations, dm)
        .map_err(|e| e.to_string())?;
    g.combat = Some(combat.clone());
    finish(g, q, dm)?;
    if damage {
        step(g, q, dm)?;
        ironsmith::game_loop::apply_multiplayer_blocker_declarations(g, &mut combat, q, &[])
            .map_err(|e| e.to_string())?;
        g.combat = Some(combat.clone());
        step(g, q, dm)?;
        dm.stage = "actual_unblocked_combat_damage".into();
        let events =
            ironsmith::game_loop::execute_combat_damage_step_with_dm(g, &combat, false, dm);
        ironsmith::game_loop::queue_combat_damage_triggers(g, &events, q);
        finish(g, q, dm)?;
        dm.trace.push(json!({"stage":"combat_damage_result","bob_life":g.player(PlayerId(1)).unwrap().life,"events":format!("{events:?}")}));
    }
    Ok(())
}
fn run(
    defs: &HashMap<String, (CardDefinition, String)>,
    family: &LinkedFamily,
    c: &Value,
    dm: &mut Dm,
) -> Result<(Value, Value), String> {
    let n = c["subject"].as_str().unwrap();
    let mode = c["mode"].as_str().unwrap();
    let negative = mode == "no_prepare";
    let mut g = game();
    let mut q = TriggerQueue::new();
    family.register(&mut g);
    let mut linked = defs.clone();
    for (n, d) in &family.definitions {
        linked.insert(n.clone(), d.clone());
    }
    for p in [alice(), PlayerId(1), PlayerId(2)] {
        for _ in 0..24 {
            g.create_object_from_definition(&defs["Forest"].0, p, Zone::Library);
        }
    }
    let witness = paid_cast(
        &mut g,
        &linked,
        if n == "Yavimaya Bloomsage" {
            if negative { "Hill Giant" } else { "Craw Wurm" }
        } else {
            "Grizzly Bears"
        },
        if n == "Yavimaya Bloomsage" {
            if negative { 4 } else { 6 }
        } else {
            2
        },
        &mut q,
        dm,
    )?;
    let altar = paid_cast(&mut g, &linked, "Altar of Dementia", 2, &mut q, dm)?;
    let mut gy = vec![];
    let material_count = if [
        "Bloodline Recollector",
        "Grave Researcher",
        "Lorehold Archivist",
    ]
    .contains(&n)
    {
        if negative { 2 } else { 3 }
    } else {
        1
    };
    for _ in 0..material_count {
        gy.push(paid_cast(&mut g, &linked, "Grizzly Bears", 2, &mut q, dm)?);
    }
    if n != "Bloodline Recollector" {
        for m in &gy {
            sacrifice(&mut g, altar, *m, &mut q, dm)?;
        }
    }
    if n == "Emeritus of Truce" && !negative {
        set_actor(&mut g, dm, PlayerId(1));
        for _ in 0..2 {
            paid_cast(&mut g, &linked, "Grizzly Bears", 2, &mut q, dm)?;
        }
        set_actor(&mut g, dm, alice());
    }
    if n == "Joined Researchers" && !negative {
        for _ in 0..2 {
            g.create_object_from_definition(&defs["Forest"].0, PlayerId(1), Zone::Hand);
        }
    }
    if n == "Naktamun Lorespinner" && negative {
        for p in [alice(), PlayerId(1), PlayerId(2)] {
            for _ in 0..2 {
                g.create_object_from_definition(&defs["Forest"].0, p, Zone::Hand);
            }
        }
    }
    dm.targets = if n == "Emeritus of Truce" {
        vec![Target::Player(PlayerId(1))]
    } else {
        vec![]
    };
    let source = paid_cast(
        &mut g,
        &linked,
        n,
        c["cost"].as_u64().unwrap() as u32,
        &mut q,
        dm,
    )?;
    let sid = current(&g, source);
    dm.targets.clear();
    dm.stage = "actual_prepare_producer".into();
    match n {
        "Abigale, Poet Laureate" => {
            paid_cast(
                &mut g,
                &linked,
                if negative {
                    "Darksteel Relic"
                } else {
                    "Grizzly Bears"
                },
                if negative { 0 } else { 2 },
                &mut q,
                dm,
            )?;
        }
        "Bloodline Recollector" => {
            for m in &gy {
                sacrifice(&mut g, altar, *m, &mut q, dm)?;
            }
            phase(&mut g, Phase::Ending, &mut q, dm)?;
        }
        "Defacing Duskmage" => {
            if negative {
                let bell = paid_cast(&mut g, &linked, "Temple Bell", 3, &mut q, dm)?;
                let a=compute_legal_actions(&g,alice()).expect("fixture has complete replacement state").into_iter().find(|a|matches!(a,LegalAction::ActivateAbility{source,..}if *source==current(&g,bell))).ok_or("Bell action unavailable")?;
                announce(&mut g, a, &mut q, dm)?;
                finish(&mut g, &mut q, dm)?;
            } else {
                paid_cast(&mut g, &linked, "Vision Skeins", 2, &mut q, dm)?;
            }
        }
        "Dirgur Focusmage" => {
            paid_cast(
                &mut g,
                &linked,
                if negative { "Divination" } else { "Tidings" },
                if negative { 2 } else { 4 },
                &mut q,
                dm,
            )?;
        }
        "Eccentric Pestfinder" | "Leech Collector" | "Scheming Silvertongue" => {
            if !negative || n != "Eccentric Pestfinder" {
                dm.x = if n == "Scheming Silvertongue" && !negative {
                    2
                } else {
                    1
                };
                dm.targets = vec![Target::Player(if n == "Leech Collector" && negative {
                    PlayerId(1)
                } else {
                    alice()
                })];
                paid_cast(&mut g, &linked, "Stream of Life", dm.x + 1, &mut q, dm)?;
                dm.targets.clear();
            }
            if n == "Eccentric Pestfinder" {
                phase(&mut g, Phase::Ending, &mut q, dm)?;
            }
            if n == "Scheming Silvertongue" {
                phase(&mut g, Phase::NextMain, &mut q, dm)?;
            }
        }
        "Eiganjo Dynastorian" => {
            let witness_id = current(&g, witness);
            combat_producer(
                &mut g,
                if negative {
                    vec![witness_id]
                } else {
                    vec![sid, witness_id]
                },
                false,
                &mut q,
                dm,
            )?;
        }
        "Encouraging Aviator" => {
            let witness_id = current(&g, witness);
            combat_producer(
                &mut g,
                vec![if negative { witness_id } else { sid }],
                false,
                &mut q,
                dm,
            )?;
        }
        "Striding Shotcaller" => {
            if negative {
                dm.targets = vec![Target::Player(PlayerId(1))];
                paid_cast(&mut g, &linked, "Shock", 1, &mut q, dm)?;
                dm.targets.clear();
            } else {
                let id = current(&g, witness);
                combat_producer(&mut g, vec![id], true, &mut q, dm)?;
            }
        }
        "Emeritus of Conflict" => {
            own_main(&mut g, true, &mut q, dm)?;
            for _ in 0..if negative { 2 } else { 3 } {
                paid_cast(&mut g, &linked, "Ornithopter", 0, &mut q, dm)?;
            }
        }
        "Emeritus of Truce" | "Inspired Skypainter" => {}
        "Grave Researcher" | "Lorehold Archivist" | "Naktamun Lorespinner" => {
            own_main(&mut g, true, &mut q, dm)?;
        }
        "Scathing Shadelock" => {
            if !negative {
                own_main(&mut g, true, &mut q, dm)?;
            }
        }
        "Joined Researchers" => {
            phase(&mut g, Phase::Ending, &mut q, dm)?;
        }
        "Kirol, History Buff" => {
            if !negative {
                dm.targets = vec![Target::Object(current(&g, gy[0]))];
                paid_cast(&mut g, &linked, "Raise Dead", 1, &mut q, dm)?;
                dm.targets.clear();
            }
        }
        "Spiritcall Enthusiast" => {
            paid_cast(
                &mut g,
                &linked,
                if negative {
                    "Grizzly Bears"
                } else {
                    "Raise the Alarm"
                },
                2,
                &mut q,
                dm,
            )?;
        }
        "Tam, Observant Sequencer" => {
            if negative {
                paid_cast(&mut g, &linked, "Darksteel Relic", 0, &mut q, dm)?;
            } else {
                paid_land(&mut g, &defs["Forest"].0, &mut q, dm)?;
            }
        }
        "Yavimaya Bloomsage" => {
            dm.targets = vec![Target::Object(current(&g, witness))];
            phase(&mut g, Phase::Ending, &mut q, dm)?;
            dm.targets.clear();
        }
        _ => return Err(format!("unhandled producer {n}")),
    }
    dm.targets.clear();
    let copies = |g: &GameState| {
        g.objects_in_deterministic_order()
            .into_iter()
            .filter(|o| g.prepared_spell_source(o.id) == Some(sid))
            .map(|o| o.id)
            .collect::<Vec<_>>()
    };
    dm.trace.push(json!({"stage":"focused_producer_result","prepared":g.is_prepared(sid),"copy_count":copies(&g).len(),"phase":format!("{:?}",g.turn.phase),"life":g.players.iter().map(|p|p.life).collect::<Vec<_>>() }));
    if negative {
        return Ok((
            json!({"prepared":false,"copy_count":0}),
            json!({"prepared":g.is_prepared(sid),"copy_count":copies(&g).len()}),
        ));
    }
    if !g.is_prepared(sid) || copies(&g).len() != 1 {
        return Ok((
            json!({"prepared_after_producer":true,"copy_count":1}),
            json!({"prepared_after_producer":g.is_prepared(sid),"copy_count":copies(&g).len()}),
        ));
    }
    own_main(&mut g, false, &mut q, dm)?;
    seed_mana(&mut g);
    let copy_ids = copies(&g);
    let a=compute_legal_actions(&g,alice()).expect("fixture has complete replacement state").into_iter().find(|a|matches!(a,LegalAction::CastSpell{spell_id,from_zone:Zone::Exile,..}if copy_ids.contains(spell_id)));
    let pre = json!({"prepared":true,"copy_count":1,"cast_available":true});
    let observed = json!({"prepared":g.is_prepared(sid),"copy_count":copy_ids.len(),"cast_available":a.is_some()});
    if mode == "hold_prepared" || a.is_none() {
        return Ok((pre, observed));
    }
    let spell = c["spell"].as_str().unwrap();
    dm.targets = match spell {
        "Heroic Stanza"
        | "Swords to Plowshares"
        | "Jump"
        | "Maestro's Gift"
        | "Pack a Punch"
        | "Venomous Words"
        | "Scrollboost"
        | "Run the Play" => vec![Target::Object(current(&g, witness))],
        "Ancestral Craving" | "Braingeyser" | "Lightning Bolt" | "Secret Rendezvous"
        | "Sign in Blood" => vec![Target::Player(PlayerId(1))],
        "Reanimate" | "Restore Relic" => vec![Target::Object(current(&g, gy[0]))],
        _ => vec![],
    };
    dm.x = 1;
    dm.stage = "cast_prepared_spell".into();
    let before = g.player(alice()).unwrap().mana_pool.total();
    let error = announce(&mut g, a.unwrap(), &mut q, dm).err();
    Ok((
        json!({"before":pre,"error":null,"paid":c["spell_cost"],"prepared_after_announcement":false,"linked_copy_count":0,"spell_on_stack":true}),
        json!({"before":observed,"error":error,"paid":before-g.player(alice()).unwrap().mana_pool.total(),"prepared_after_announcement":g.is_prepared(sid),"linked_copy_count":copies(&g).len(),"spell_on_stack":g.stack.iter().any(|e|g.object(e.object_id).is_some_and(|o|o.name==spell))}),
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
    let input = p.join("prepare-trigger-inputs.json");
    let data: Value = serde_json::from_slice(&std::fs::read(&input).unwrap()).unwrap();
    let families = data["cases"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["group"].as_str().unwrap())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .map(|n| (n.to_string(), LinkedFamily::from_catalog(&root, n)))
        .collect::<HashMap<_, _>>();
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
        eprintln!("PREPARE_TRIGGER {c}");
        let mut dm = Dm {
            actor: alice(),
            targets: vec![],
            stage: "fixture".into(),
            trace: vec![],
            chosen: vec![],
            reverse: c["reverse"].as_bool().unwrap_or(false),
            x: c["x"].as_u64().unwrap_or(0) as u32,
            plot: c["plot"].as_bool().unwrap_or(false),
            option_text: String::new(),
        };
        let family = &families[c["group"].as_str().unwrap()];
        if let Err(error) = family {
            rows.push(json!({"card":n,"scenario":c,"status":"linked_face_compile_failed","expected":null,"actual":{"error":error},"artifact_checksum":defs[n].1}));
            continue;
        }
        let result = run(&defs, family.as_ref().unwrap(), c, &mut dm);
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
        "crates/ironsmith-engine/src/game_loop/priority_apply.rs",
        "crates/ironsmith-engine/src/game_loop/priority_mana.rs",
        "crates/ironsmith-engine/src/game_state/object_state_and_events.rs",
        "crates/ironsmith-engine/src/effects/mana/add_mana_of_any_one_color.rs",
        "crates/ironsmith-engine/src/effects/composition/choose_objects.rs",
    ];
    let report = json!({"scope":"All22 frozen compiled Triggered-only prepare source groups and aliases. Actual paid spell, life gain, draw, land, graveyard, token, phase and combat producers. Positive hold/cast plus applicable negative producer controls; linked compilation gates separate. Cast-completion invariant before linked spell resolution.","limitations":"No prepared-state injection. Real phase transitions use advance_step, actual untap/draw and generated phase events; actual attacker/blocker declarations and damage execution produce combat triggers. Initial mana/library, first-main ordinal1 and clearing sickness before combat are setup. Mana pool refilled only at fixture casting windows after phase drains. Spell effects after prepared cast announcement not certified; no JS binding replay.","provenance":{"binary":binary,"binary_sha256":std::env::var("AUDIT_BINARY_SHA256").unwrap(),"source_sha256":hash(&root.join("crates/ironsmith-tools/tests/runtime_prepare_trigger_reproductions.rs")),"runtime_source_hashes":files.iter().map(|f|json!({"path":f,"sha256":hash(&root.join(f))})).collect::<Vec<_>>(),"input_sha256":hash(&input),"helper_sha256":hash(&root.join("crates/ironsmith-tools/tests/support/canonical_linked_fixture.rs")),"unique_card_ids":true,"seed":SEED,"thread_stack_bytes":67108864,"compiled_via":"ironsmith_registry::compile_builder_to_artifact"},"rows":rows,"linkage_families":families.values().filter_map(|f|f.as_ref().ok()).map(|f|json!({"metadata":f.metadata_record,"unlinked_artifacts":f.unlinked_artifacts,"linked_artifacts":f.linked_artifacts})).collect::<Vec<_>>()});
    std::fs::write(
        p.join("prepare-trigger-artifacts.json"),
        serde_json::to_string_pretty(&artifacts).unwrap(),
    )
    .unwrap();
    std::fs::write(
        p.join("prepare-trigger-reproductions.json"),
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
}
#[test]
#[ignore = "canonical prepare-trigger expected-result reporter"]
fn report_prepare_trigger() {
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(generate)
        .unwrap()
        .join()
        .unwrap();
}
