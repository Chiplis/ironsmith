//! Paid canonical activations, actual loyalty/station costs and Saga progression.
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
use ironsmith::{
    CardDefinition, CardId, CardType, GameState, ObjectId, PlayerId, PowerToughness, Zone,
};
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
    target: Option<ObjectId>,
    trace: Vec<Value>,
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
        let a = SelectFirstDecisionMaker.decide_options(g, c);
        self.trace
            .push(json!({"decision":"options","context":format!("{c:?}"),"answer":a}));
        a
    }
    fn decide_targets(&mut self, g: &GameState, c: &TargetsContext) -> Vec<Target> {
        let a = if let Some(id) = self.target {
            c.requirements
                .iter()
                .filter(|r| r.legal_targets.contains(&Target::Object(id)))
                .map(|_| Target::Object(id))
                .collect()
        } else {
            SelectFirstDecisionMaker.decide_targets(g, c)
        };
        self.trace.push(
            json!({"decision":"targets","context":format!("{c:?}"),"answer":format!("{a:?}")}),
        );
        a
    }
}
fn basic(name: &str, kind: CardType) -> CardDefinition {
    let b = CardDefinitionBuilder::new(CardId::new(), name).card_types(vec![kind]);
    if kind == CardType::Creature {
        b.power_toughness(PowerToughness::fixed(2, 6)).build()
    } else {
        b.build()
    }
}
fn cast(
    g: &mut GameState,
    def: &CardDefinition,
    actor: PlayerId,
    q: &mut TriggerQueue,
    dm: &mut Choices,
) -> Result<u32, String> {
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
use ironsmith::CounterType;
use ironsmith::ability::AbilityKind;
use ironsmith::combat_state::{AttackTarget, CombatState};
use ironsmith::decision::AttackerDeclaration;
fn count(g: &GameState, id: ObjectId, kind: CounterType) -> u32 {
    g.object(id)
        .and_then(|o| o.counters.get(&kind))
        .copied()
        .unwrap_or(0)
}
fn activation(g: &GameState, source: ObjectId, index: usize) -> Option<LegalAction> {
    compute_legal_actions(g, PlayerId(0)).expect("fixture has complete replacement state").into_iter().find(|a| matches!(a,LegalAction::ActivateAbility{source:s,ability_index:i} if *s==source && *i==index))
}
fn announce(
    g: &mut GameState,
    q: &mut TriggerQueue,
    dm: &mut Choices,
    action: LegalAction,
) -> Result<(), String> {
    g.turn.priority_player = Some(PlayerId(0));
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
fn start_next_turn(
    g: &mut GameState,
    q: &mut TriggerQueue,
    dm: &mut Choices,
) -> Result<(), String> {
    g.next_turn();
    ironsmith::turn::execute_untap_step(g);
    ironsmith::turn::advance_step(g).map_err(|e| e.to_string())?;
    ironsmith::turn::advance_step(g).map_err(|e| e.to_string())?;
    let events = ironsmith::turn::execute_draw_step_with(g, dm).unwrap();
    for event in events {
        for t in ironsmith::triggers::check_triggers(g, &event) {
            q.add(t);
        }
    }
    resolve_all(g, q, dm)?;
    ironsmith::turn::advance_phase(g).map_err(|e| e.to_string())?;
    g.turn.priority_player = Some(g.turn.active_player);
    dm.trace.push(json!({"stage":"next_turn_main","active":g.turn.active_player.index(),"turn":g.turn.turn_number,"phase":format!("{:?}",g.turn.phase)}));
    Ok(())
}
fn next_own_turn(g: &mut GameState, q: &mut TriggerQueue, dm: &mut Choices) -> Result<(), String> {
    for _ in 0..g.players_in_game() {
        start_next_turn(g, q, dm)?;
    }
    Ok(())
}
fn hand_counts(g: &GameState) -> Vec<usize> {
    g.players.iter().map(|p| p.hand.len()).collect()
}
fn run(def: &CardDefinition, seats: usize, accept: bool, negative: bool) -> Result<Value, String> {
    let mut g = GameState::new((0..seats).map(|i| format!("Seat{i}")).collect(), 30);
    g.set_random_seed(71757432704853);
    g.turn.turn_number = 3;
    g.turn.active_player = PlayerId(0);
    g.turn.priority_player = Some(PlayerId(0));
    g.turn.phase = ironsmith::Phase::FirstMain;
    g.turn.step = None;
    let artifact = basic("Neutral library artifact", CardType::Artifact);
    for seat in 0..seats {
        let p = PlayerId(seat as u8);
        for _ in 0..40 {
            g.create_object_from_definition(&artifact, p, Zone::Library);
        }
        for _ in 0..2 {
            g.create_object_from_definition(&artifact, p, Zone::Hand);
        }
        for sym in [
            ManaSymbol::White,
            ManaSymbol::Blue,
            ManaSymbol::Black,
            ManaSymbol::Red,
            ManaSymbol::Green,
        ] {
            g.player_mut(p).unwrap().mana_pool.add(sym, 5);
        }
        g.player_mut(p)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Colorless, 12);
    }
    let mut dm = Choices {
        accept,
        target: None,
        trace: vec![],
    };
    let mut q = TriggerQueue::new();
    let victim = if def.name() == "The Death of Gwen Stacy" {
        let id = g.create_object_from_definition(
            &basic("Saga chapter one victim", CardType::Creature),
            PlayerId(1),
            Zone::Battlefield,
        );
        dm.target = Some(id);
        Some(id)
    } else {
        None
    };
    let paid = cast(&mut g, def, PlayerId(0), &mut q, &mut dm)?;
    let want = if def.name() == "The Death of Gwen Stacy" {
        3
    } else if matches!(
        def.name(),
        "Entropic Battlecruiser" | "Jace, the Living Guildpact"
    ) {
        4
    } else {
        5
    };
    if paid != want {
        return Err(format!("source paid{paid} expected{want}"));
    }
    resolve_all(&mut g, &mut q, &mut dm).map_err(|e| format!("canonical cast setup:{e}"))?;
    let source = *g
        .battlefield
        .iter()
        .find(|id| g.object(**id).is_some_and(|o| o.name == def.name()))
        .ok_or("source absent after cast")?;
    dm.target = None;
    let indexes: Vec<_> = def
        .abilities
        .iter()
        .enumerate()
        .filter(|(_, a)| matches!(a.kind, AbilityKind::Activated(_)))
        .map(|(i, _)| i)
        .collect();
    let mut expected = json!({"resolution_error":null,"remaining_stack":0});
    let mut evidence = json!({"source":source.0,"paid":paid,"activated_ability_indexes":indexes});
    let error;
    if def.name() == "The Death of Gwen Stacy" {
        if count(&g, source, CounterType::Lore) != 1 || g.battlefield.contains(&victim.unwrap()) {
            return Err("Saga chapter I failed setup".into());
        }
        if negative {
            start_next_turn(&mut g, &mut q, &mut dm)?;
        } else {
            next_own_turn(&mut g, &mut q, &mut dm)?;
        }
        let before_life: Vec<_> = g.players.iter().map(|p| p.life).collect();
        let before_hands = hand_counts(&g);
        ironsmith::game_loop::add_saga_lore_counters_with_dm(&mut g, &mut q, &mut dm).unwrap();
        evidence["lore_after_rule_action"] = json!(count(&g, source, CounterType::Lore));
        let want_lore = if negative { 1 } else { 2 };
        if count(&g, source, CounterType::Lore) != want_lore {
            return Err("Saga genuine lore progression mismatch".into());
        }
        drain(&mut g, &mut q, &mut dm)?;
        evidence["chapter_stack_count"] = json!(g.stack.len());
        expected["life"] = json!(
            before_life
                .iter()
                .map(|v| v - if !negative && !accept { 3 } else { 0 })
                .collect::<Vec<_>>()
        );
        expected["hand_counts"] = json!(
            before_hands
                .iter()
                .map(|v| v - usize::from(!negative && accept))
                .collect::<Vec<_>>()
        );
        error = resolve_all(&mut g, &mut q, &mut dm).err();
    } else if def.name() == "Jace, the Living Guildpact" {
        if count(&g, source, CounterType::Loyalty) != 5 {
            return Err("Jace initial loyalty is not five".into());
        }
        let plus = *indexes.first().ok_or("Jace no +1 ability")?;
        let ult = *indexes.last().unwrap();
        if negative {
            expected["ultimate_offered"] = json!(false);
            evidence["ultimate_offered"] = json!(activation(&g, source, ult).is_some());
            error = None;
        } else {
            for loyalty in 6..=8 {
                let a = activation(&g, source, plus).ok_or("Jace genuine +1 unavailable")?;
                announce(&mut g, &mut q, &mut dm, a)?;
                resolve_all(&mut g, &mut q, &mut dm).map_err(|e| format!("Jace +1 setup:{e}"))?;
                if count(&g, source, CounterType::Loyalty) != loyalty {
                    return Err(format!("Jace +1 loyalty {:?}", g.object(source)));
                }
                evidence[format!("loyalty_{loyalty}_turn")] = json!(g.turn.turn_number);
                next_own_turn(&mut g, &mut q, &mut dm)?;
            }
            let a = activation(&g, source, ult)
                .ok_or("Jace actual eight-loyalty ultimate unavailable")?;
            announce(&mut g, &mut q, &mut dm, a)?;
            expected["hand_counts"] = json!(
                (0..seats)
                    .map(|i| if i == 0 { 7 } else { 0 })
                    .collect::<Vec<_>>()
            );
            error = resolve_all(&mut g, &mut q, &mut dm).err();
        }
    } else if def.name() == "Entropic Battlecruiser" {
        let stationer = CardDefinitionBuilder::new(CardId::new(), "Eight power Station crew")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(8, 8))
            .build();
        let crew = g.create_object_from_definition(&stationer, PlayerId(0), Zone::Battlefield);
        let acts: Vec<_> = compute_legal_actions(&g, PlayerId(0)).expect("fixture has complete replacement state")
            .into_iter()
            .filter(|a| matches!(a,LegalAction::ActivateAbility{source:s,..}if *s==source))
            .collect();
        evidence["station_actions"] = json!(format!("{acts:?}"));
        let a = acts
            .into_iter()
            .next()
            .ok_or("Station ability unavailable")?;
        announce(&mut g, &mut q, &mut dm, a)?;
        resolve_all(&mut g, &mut q, &mut dm).map_err(|e| format!("Station setup:{e}"))?;
        evidence["crew_tapped"] = json!(g.is_tapped(crew));
        evidence["charge"] = json!(count(&g, source, CounterType::Charge));
        if !g.is_tapped(crew) || count(&g, source, CounterType::Charge) != 8 {
            return Err("Station cost/counters setup failed".into());
        }
        next_own_turn(&mut g, &mut q, &mut dm)?;
        ironsmith::turn::advance_phase(&mut g).map_err(|e| e.to_string())?;
        ironsmith::turn::advance_step(&mut g).map_err(|e| e.to_string())?;
        let mut combat = CombatState::default();
        let attacker = if negative { crew } else { source };
        ironsmith::game_loop::apply_attacker_declarations_with_dm(
            &mut g,
            &mut combat,
            &mut q,
            &[AttackerDeclaration {
                creature: attacker,
                target: AttackTarget::Player(PlayerId(1)),
            }],
            &mut dm,
        )
        .map_err(|e| format!("actual attack:{e}"))?;
        evidence["actual_attacker"] = json!(attacker.0);
        g.combat = Some(combat);
        drain(&mut g, &mut q, &mut dm)?;
        evidence["attack_trigger_count"] = json!(g.stack.len());
        error = resolve_all(&mut g, &mut q, &mut dm).err();
    } else {
        next_own_turn(&mut g, &mut q, &mut dm)?;
        let baseline = hand_counts(&g);
        evidence["hands_before_activation"] = json!(baseline);
        if negative {
            error = None;
        } else {
            let a = activation(
                &g,
                source,
                *indexes.first().ok_or("Jar activated ability missing")?,
            )
            .ok_or("Jar legal activation unavailable after untap")?;
            announce(&mut g, &mut q, &mut dm, a)?;
            if g.battlefield.contains(&source) {
                return Err("Jar actual sacrifice cost not paid".into());
            }
            evidence["sacrifice_paid"] = json!(true);
            let first_error = resolve_all(&mut g, &mut q, &mut dm).err();
            evidence["first_resolution_error"] = json!(first_error);
            evidence["hands_after_initial_resolution"] = json!(hand_counts(&g));
            if first_error.is_some() {
                error = first_error;
            } else {
                if hand_counts(&g) != vec![7; seats] {
                    return Err("Jar initial draw-seven setup mismatch".into());
                }
                for _ in 0..3 {
                    ironsmith::turn::advance_phase(&mut g).map_err(|e| e.to_string())?;
                }
                let event = ironsmith::triggers::generate_step_trigger_events(&g)
                    .ok_or("endstep event absent")?;
                for t in ironsmith::triggers::check_triggers(&g, &event) {
                    q.add(t);
                }
                evidence["registered_delayed_before_end"] =
                    json!(g.effect_store.delayed_triggers.len());
                for t in ironsmith::triggers::check_delayed_triggers(&mut g, &event) {
                    q.add(t);
                }
                drain(&mut g, &mut q, &mut dm)?;
                evidence["delayed_end_step_stack_count"] = json!(g.stack.len());
                error = resolve_all(&mut g, &mut q, &mut dm).err();
            }
        }
        expected["hand_counts"] = json!(baseline);
    }
    let mut actual = json!({"resolution_error":error,"remaining_stack":g.stack.len()});
    if expected.get("life").is_some() {
        actual["life"] = json!(g.players.iter().map(|p| p.life).collect::<Vec<_>>());
    }
    if expected.get("hand_counts").is_some() {
        actual["hand_counts"] = json!(hand_counts(&g));
    }
    if expected.get("ultimate_offered").is_some() {
        actual["ultimate_offered"] = evidence["ultimate_offered"].clone();
    }
    evidence["final_life"] = json!(g.players.iter().map(|p| p.life).collect::<Vec<_>>());
    evidence["final_hand_counts"] = json!(hand_counts(&g));
    Ok(
        json!({"expected":expected,"actual":actual,"execution_trace":dm.trace,"state_evidence":evidence}),
    )
}
#[test]
#[ignore = "audit observations, scoped assertions only"]
fn report_simultaneous_activations() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let inventory =
        root.join("reports/runtime-audit/corpus/267a16aff3b321196397d0b4/inventory.json");
    let source = root.join(
        "crates/ironsmith-tools/tests/runtime_simultaneous_activation_family_reproductions.rs",
    );
    let binary = std::env::current_exe().unwrap();
    let paths = [&inventory, &source, &binary];
    let before: Vec<_> = paths
        .iter()
        .map(|p| json!({"path":p,"sha256":hash(p)}))
        .collect();
    let inv: Value = serde_json::from_slice(&std::fs::read(&inventory).unwrap()).unwrap();
    let mut rows = vec![];
    let mut compilation = vec![];
    for name in [
        "Entropic Battlecruiser",
        "Jace, the Living Guildpact",
        "Magus of the Jar",
        "Memory Jar",
        "The Death of Gwen Stacy",
    ] {
        let p = inv["cards"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["name"] == name)
            .unwrap();
        let (artifact, def) = ironsmith_registry::compile_builder_to_artifact(
            ironsmith_compiler::CardDefinitionBuilder::new(CardId::new(), name),
            p["parse_input"].as_str().unwrap(),
            false,
        )
        .unwrap();
        compilation.push(json!({"card":name,"artifact_checksum":artifact.payload_checksum,"definition":artifact.payload.definition}));
        for (seats, accept, negative) in [
            (2, true, false),
            (2, false, false),
            (3, true, false),
            (3, false, false),
            (2, false, true),
        ] {
            let r = run(&def, seats, accept, negative);
            let (status, out) = match r {
                Ok(v) => {
                    let status = if v["actual"]["resolution_error"].is_string() {
                        "resolution_failed"
                    } else if v["expected"] == v["actual"] {
                        "expected_result_observed"
                    } else {
                        "semantic_mismatch"
                    };
                    (status, v)
                }
                Err(e) => (
                    "fixture_or_execution_error",
                    json!({"expected":null,"actual":{"error":e}}),
                ),
            };
            rows.push(json!({"card":name,"scenario":{"players":seats,"optional_policy":accept,"negative_control":negative},"status":status,"expected":out["expected"],"actual":out["actual"],"execution_trace":out["execution_trace"],"state_evidence":out["state_evidence"],"artifact_checksum":artifact.payload_checksum,"scope":"Strict canonical full input, actual paid cast, actual activation costs, production priority resolution. Jar waits for next own untap; Jace progresses actual +1 on three separate turns before ultimate; Saga II follows chapter I and real next-own-turn lore action; Spacecraft pays Station and attacks after actual next-own-turn untap. Unrelated steps do not replay all game actions. Optional policy does not prove callback reached. No whole-card correctness claim."}));
        }
    }
    let after: Vec<_> = paths
        .iter()
        .map(|p| json!({"path":p,"sha256":hash(p)}))
        .collect();
    let report = json!({"rows":rows,"compilation":compilation,"provenance":{"before":before,"after":after,"artifacts_unchanged":before==after,"strict_artifact":true,"unique_card_ids":true,"seed":71757432704853_u64}});
    std::fs::write(
        root.join("reports/runtime-audit/simultaneous-activation-family-reproductions.json"),
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
    println!("wrote {} cases", report["rows"].as_array().unwrap().len());
}
