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

const NAMES: [&str; 2] = ["Catapult Squad", "Templar Knight"];
fn play_land(g: &mut GameState, d: &CardDefinition, dm: &mut Choices) -> Result<ObjectId, String> {
    let hand = g.create_object_from_definition(d, PlayerId(0), Zone::Hand);
    let a = compute_legal_actions(g, PlayerId(0)).expect("fixture has complete replacement state")
        .into_iter()
        .find(|a| matches!(a,LegalAction::PlayLand{land_id}if *land_id==hand))
        .ok_or("land play missing")?;
    let mut q = TriggerQueue::new();
    let mut st = PriorityLoopState::new(g.players_in_game());
    apply_priority_response_with_dm(g, &mut q, &mut st, &PriorityResponse::PriorityAction(a), dm)
        .map_err(|e| e.to_string())?;
    finish(g, &mut q, dm)?;
    find(g, d.name(), Zone::Battlefield)
}
fn combat(
    g: &mut GameState,
    actor: u8,
    attackers: &[ObjectId],
    block: Option<ObjectId>,
    dm: &mut Choices,
) -> Result<Value, String> {
    use ironsmith::turn_runner::{TurnAction, TurnRunner, TurnState};
    let mut r = TurnRunner::from_state_for_sync(TurnState::FirstMainPriority);
    let mut q = TriggerQueue::new();
    for _ in 0..100 {
        match r.advance(g, &mut q).map_err(|e| e.to_string())? {
            TurnAction::Continue => {}
            TurnAction::RunPriority => {
                if matches!(r.state(), TurnState::DeclareBlockersPriority) {
                    finish(g, &mut q, dm)?;
                    let mut st = PriorityLoopState::new(g.players_in_game());
                    for _ in 0..3 {
                        if g.turn.priority_player == Some(PlayerId(0)) {
                            return Ok(
                                json!({"active":actor,"attackers":attackers.iter().map(|id|id.0).collect::<Vec<_>>(),"blocker":block.map(|id|id.0),"combat":format!("{:?}",g.combat),"priority":g.turn.priority_player.map(|p|p.index())}),
                            );
                        }
                        apply_priority_response_with_dm(
                            g,
                            &mut q,
                            &mut st,
                            &PriorityResponse::PriorityAction(LegalAction::PassPriority),
                            dm,
                        )
                        .map_err(|e| e.to_string())?;
                    }
                    return Err("Alice did not receive priority".into());
                }
                ironsmith::game_loop::run_priority_loop_with(
                    g,
                    &mut q,
                    &mut ironsmith::decision::AutoPassDecisionMaker,
                )
                .map_err(|e| e.to_string())?;
                r.priority_done();
            }
            TurnAction::Decision(c) => match c {
                DecisionContext::Attackers(_) => r.respond_attackers(
                    attackers
                        .iter()
                        .map(|id| ironsmith::decision::AttackerDeclaration {
                            creature: *id,
                            target: ironsmith::combat_state::AttackTarget::Player(PlayerId(
                                1 - actor,
                            )),
                        })
                        .collect(),
                ),
                DecisionContext::Blockers(c) => r.respond_blockers(
                    block
                        .map(|id| ironsmith::decision::BlockerDeclaration {
                            blocker: id,
                            blocking: attackers[0],
                        })
                        .into_iter()
                        .collect(),
                    c.player,
                ),
                c => return Err(format!("unexpected combat decision:{c:?}")),
            },
            a => return Err(format!("combat stopped:{a:?}")),
        }
    }
    Err("combat bound".into())
}
fn run(
    defs: &HashMap<String, CardDefinition>,
    name: &str,
    extra: i32,
    mode: &str,
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
    for _ in 0..4 {
        for p in [PlayerId(0), PlayerId(1)] {
            g.create_object_from_definition(&defs["Plains"], p, Zone::Library);
        }
    }
    if name == "Templar Knight" {
        for n in ["Shadowspear", "Ornithopter", "Isamaru, Hound of Konda"] {
            g.create_object_from_definition(&defs[n], PlayerId(0), Zone::Library);
        }
    }
    let white_land = if name == "Templar Knight" {
        Some(play_land(&mut g, &defs["Plains"], &mut dm)?)
    } else {
        None
    };
    let source_cast = paid(&mut g, defs, name, &mut dm, 2)?;
    let source = find(&g, name, Zone::Battlefield)?;
    let required = if name == "Templar Knight" { 5 } else { 2 };
    let total = required + extra;
    let mut resources = vec![source];
    for _ in 1..total {
        let n = if name == "Templar Knight" {
            name
        } else {
            "Benalish Hero"
        };
        let price = if name == "Templar Knight" { 2 } else { 1 };
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
    let mut attacker = None;
    if name == "Catapult Squad" {
        producers.push(paid(&mut g, defs, "Grizzly Bears", &mut dm, 2)?);
        attacker = Some(find(&g, "Grizzly Bears", Zone::Battlefield)?);
    }
    next_main(&mut g);
    let target_cast = cast(&mut g, &defs["Hill Giant"], 1, &mut dm)?;
    if target_cast["mana_paid"] != 4 || !target_cast["resolution_error"].is_null() {
        return Err("target cast failed".into());
    }
    let target = find(&g, "Hill Giant", Zone::Battlefield)?;
    next_main(&mut g);
    let blue_land = if name == "Templar Knight" {
        Some(play_land(&mut g, &defs["Island"], &mut dm)?)
    } else {
        None
    };
    let mut attackers = vec![];
    let mut actor = 0;
    let mut blocker = None;
    if name == "Templar Knight" {
        attackers = resources.clone();
        if mode == "source_idle" {
            attackers.retain(|id| *id != source);
        }
        if mode == "four_attack" {
            attackers.truncate(4);
        }
    } else if mode == "attacking_target" {
        next_main(&mut g);
        actor = 1;
        attackers = vec![target];
    } else {
        attackers = vec![attacker.unwrap()];
        if mode == "blocking_target" {
            blocker = Some(target);
        }
    }
    let combat_evidence = if mode == "no_combat" {
        Value::Null
    } else {
        combat(&mut g, actor, &attackers, blocker, &mut dm)?
    };
    if mode == "tapped_attacker" {
        let e = action(&mut g, blue_land.unwrap(), 0, true, &mut dm)?;
        if !e["resolution_error"].is_null() {
            return Err("Island mana failed".into());
        }
        producers.push(e);
        dm.targets = vec![Target::Object(attackers[0])];
        producers.push(paid(&mut g, defs, "Twiddle", &mut dm, 1)?);
        if !g.is_tapped(attackers[0]) {
            return Err("Twiddle attacker failed".into());
        }
    }
    if let Some(land) = white_land {
        let e = action(&mut g, land, 0, true, &mut dm)?;
        if !e["resolution_error"].is_null() {
            return Err("Plains mana failed".into());
        }
        producers.push(e);
    }
    let index = if name == "Templar Knight" { 1 } else { 0 };
    let actions = compute_legal_actions(&g, PlayerId(0)).expect("fixture has complete replacement state");
    let offered=actions.iter().any(|a|matches!(a,LegalAction::ActivateAbility{source:s,ability_index}if *s==source&&*ability_index==index));
    dm.targets = if name == "Catapult Squad" {
        vec![Target::Object(target)]
    } else {
        vec![]
    };
    dm.names = if name == "Templar Knight" {
        vec!["Templar Knight".into(), "Shadowspear".into()]
    } else {
        vec!["Catapult Squad".into(), "Benalish Hero".into()]
    };
    let before = json!({"source_index":index,"source_sick":g.is_summoning_sick(source),"source_tapped":g.is_tapped(source),"target_damage":g.damage_on(target),"target_tapped":g.is_tapped(target),"resources":resources.iter().map(|id|json!({"id":id.0,"name":g.object(*id).unwrap().name.to_string(),"subtypes":g.object(*id).unwrap().subtypes.iter().map(|t|format!("{t:?}")).collect::<Vec<_>>(),"tapped":g.is_tapped(*id),"attacking":g.combat.as_ref().is_some_and(|c|ironsmith::combat_state::is_attacking(c,*id))})).collect::<Vec<_>>(),"actions":format!("{actions:?}"),"combat":combat_evidence});
    if name == "Catapult Squad" && mode == "noncombat_target" {
        let abilities = g.current_abilities(source).unwrap();
        let a = match &abilities[index].kind {
            ironsmith::ability::AbilityKind::Activated(a) => a,
            _ => return Err("Catapult ability missing".into()),
        };
        let legal = a
            .effects
            .iter()
            .filter_map(|effect| effect.0.get_target_spec())
            .flat_map(|spec| {
                ironsmith::targeting::compute_legal_targets(&g, spec, PlayerId(0), Some(source))
            })
            .collect::<Vec<_>>();
        return Ok((
            json!({"activation_available":true,"opposing_noncombat_target_legal":false,"own_attacker_target_legal":true}),
            json!({"activation_available":offered,"opposing_noncombat_target_legal":legal.contains(&Target::Object(target)),"own_attacker_target_legal":legal.contains(&Target::Object(attacker.unwrap()))}),
            json!({"source_cast":source_cast,"target_cast":target_cast,"producers":producers,"before_activation":before,"activation_dispatched":false,"legal_targets":format!("{legal:?}"),"choice_trace":dm.trace,"scope":"Other own attacker makes activation available; actual compiled target spec excludes the opposing noncombat Giant. Invalid target is never submitted."}),
        ));
    }
    let valid = extra >= 0
        && if name == "Templar Knight" {
            !["no_combat", "four_attack", "tapped_attacker"].contains(&mode)
        } else {
            ["blocking_target", "attacking_target"].contains(&mode)
        };
    if !valid || !offered {
        return Ok((
            json!({"activation_available":valid}),
            json!({"activation_available":offered}),
            json!({"source_cast":source_cast,"target_cast":target_cast,"producers":producers,"before_activation":before,"activation_dispatched":false,"choice_trace":dm.trace}),
        ));
    }
    let mana = g.player(PlayerId(0)).unwrap().mana_pool.total();
    let activation = action(&mut g, source, index, false, &mut dm)
        .unwrap_or_else(|e| json!({"resolution_error":e}));
    let actual = json!({"error":activation["resolution_error"],"mana_paid":mana-g.player(PlayerId(0)).unwrap().mana_pool.total(),"tapped_resources":resources.iter().filter(|id|g.is_tapped(**id)).count(),"untapped_resources":resources.iter().filter(|id|!g.is_tapped(**id)).count(),"source_tapped":g.is_tapped(source),"target_damage":g.damage_on(target),"target_zone":format!("{:?}",g.object(target).unwrap().zone),"shadowspear_battlefield":count(&g,"Shadowspear",Zone::Battlefield),"ornithopter_library":count(&g,"Ornithopter",Zone::Library),"isamaru_library":count(&g,"Isamaru, Hound of Konda",Zone::Library),"stack_length":g.stack.len()});
    let expected = json!({"error":null,"mana_paid":if name=="Templar Knight"{1}else{0},"tapped_resources":required,"untapped_resources":extra,"source_tapped":mode!="source_idle","target_damage":if name=="Catapult Squad"{2}else{0},"target_zone":"Battlefield","shadowspear_battlefield":if name=="Templar Knight"{1}else{0},"ornithopter_library":if name=="Templar Knight"{1}else{0},"isamaru_library":if name=="Templar Knight"{1}else{0},"stack_length":0});
    Ok((
        expected,
        actual,
        json!({"source_cast":source_cast,"target_cast":target_cast,"producers":producers,"before_activation":before,"activation":activation,"activation_dispatched":true,"choice_trace":dm.trace,"scope":"Actual paid source/resources/targets, actual turn and untap, TurnRunner attacks and blocks; actual priority passes to Alice. Combat flags never injected. Exact selected tap count and immediate outcome before combat damage."}),
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
fn report_combat_tap() {
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
                "Benalish Hero",
                "Plains",
                "Island",
                "Shadowspear",
                "Ornithopter",
                "Isamaru, Hound of Konda",
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
        let cases = if name == "Templar Knight" {
            vec![
                (-1, "all_attack"),
                (0, "all_attack"),
                (1, "all_attack"),
                (0, "no_combat"),
                (0, "four_attack"),
                (1, "source_idle"),
                (0, "tapped_attacker"),
            ]
        } else {
            vec![
                (-1, "blocking_target"),
                (0, "blocking_target"),
                (1, "blocking_target"),
                (0, "attacking_target"),
                (0, "no_combat"),
                (0, "noncombat_target"),
            ]
        };
        for (extra, mode) in cases {
            let (status, expected, actual, evidence) = match run(&defs, name, extra, mode) {
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
            rows.push(json!({"card":name,"scenario":{"extra_resources":extra,"mode":mode},"status":status,"expected":expected,"actual":actual,"fixture_evidence":evidence}));
        }
    }
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let binary = std::env::current_exe().unwrap();
    let r = json!({"scope":"Catapult and Templar fixed tap costs with actual declared combat and normally aged paid resources. Exact target damage/search output and no forced invalid action.","rows":rows,"artifacts":artifacts,"provenance":{"binary":binary,"binary_sha256":hash(&binary),"inventory_sha256":hash(&input),"source_sha256":hash(&root.join("crates/ironsmith-tools/tests/runtime_combat_tap_reproductions.rs")),"runtime_stack_bytes":67108864}});
    std::fs::write(
        root.join("reports/runtime-audit/combat-tap-reproductions.json"),
        serde_json::to_string_pretty(&r).unwrap(),
    )
    .unwrap();
    println!("{}", serde_json::to_string_pretty(&r).unwrap());
}
