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
    target: Option<Target>,
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
        let a = if let Some(target) = self.target {
            c.requirements
                .iter()
                .filter(|r| r.legal_targets.contains(&target))
                .map(|_| target)
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
    let events = ironsmith::turn::execute_draw_step_with(g, dm);
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
use ironsmith::Supertype;

fn run(
    def: &CardDefinition,
    producers: &std::collections::HashMap<String, CardDefinition>,
    seats: usize,
    variant: usize,
) -> Result<Value, String> {
    let mut g = GameState::new((0..seats).map(|i| format!("Seat{i}")).collect(), 30);
    g.set_random_seed(71757432704854);
    g.turn.turn_number = 3;
    g.turn.active_player = PlayerId(0);
    g.turn.priority_player = Some(PlayerId(0));
    g.turn.phase = ironsmith::Phase::FirstMain;
    g.turn.step = None;
    let artifact = basic("Remaining family neutral artifact", CardType::Artifact);
    let creature = basic("Remaining family creature", CardType::Creature);
    let land = CardDefinitionBuilder::new(CardId::new(), "Remaining family basic land")
        .card_types(vec![CardType::Land])
        .supertypes(vec![Supertype::Basic])
        .build();
    let mut creatures = vec![];
    let mut lands = vec![];
    for seat in 0..seats {
        let p = PlayerId(seat as u8);
        for _ in 0..20 {
            g.create_object_from_definition(&artifact, p, Zone::Library);
            g.create_object_from_definition(&land, p, Zone::Library);
        }
        for _ in 0..2 {
            g.create_object_from_definition(&artifact, p, Zone::Hand);
        }
        if (variant != 1 || seat == 0) && !(def.name() == "Fatal Grudge" && variant == 2) {
            let resource = if def.name() == "Fatal Grudge" && variant == 1 {
                &artifact
            } else {
                &creature
            };
            creatures.push(g.create_object_from_definition(resource, p, Zone::Battlefield));
        }
        for _ in 0..2 {
            lands.push(g.create_object_from_definition(&land, p, Zone::Battlefield));
        }
        if variant != 1 {
            g.create_object_from_definition(
                if variant == 2 { &land } else { &artifact },
                p,
                Zone::Graveyard,
            );
        }
        for sym in [
            ManaSymbol::White,
            ManaSymbol::Blue,
            ManaSymbol::Black,
            ManaSymbol::Red,
            ManaSymbol::Green,
        ] {
            g.player_mut(p).unwrap().mana_pool.add(sym, 8);
        }
        g.player_mut(p)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Colorless, 20);
    }
    let mut dm = Choices {
        accept: true,
        target: None,
        trace: vec![],
    };
    let mut q = TriggerQueue::new();
    let mut expected = json!({"resolution_error":null,"remaining_stack":0});
    let mut evidence = json!({"variant":variant});
    let error;
    if def.name() == "Fatal Grudge" {
        let hand = g.create_object_from_definition(def, PlayerId(0), Zone::Hand);
        let actions = compute_legal_actions(&g, PlayerId(0)).expect("fixture has complete replacement state");
        let offered = actions
            .iter()
            .any(|a| matches!(a,LegalAction::CastSpell{spell_id,..} if *spell_id==hand));
        let expected_offer = variant != 2;
        evidence["available_own_nonland_permanents"] = json!(
            g.battlefield
                .iter()
                .filter_map(|id| g.object(*id))
                .filter(|o| g.controller_of(o) == PlayerId(0)
                    && !o.card_types.contains(&CardType::Land))
                .map(|o| format!("{:?}", o))
                .collect::<Vec<_>>()
        );
        evidence["phase"] = json!(format!("{:?}", g.turn));
        evidence["available_mana"] =
            json!(format!("{:?}", g.player(PlayerId(0)).unwrap().mana_pool));
        evidence["offered_actions"] = json!(format!("{actions:?}"));
        expected = json!({"cast_offered":expected_offer});
        return Ok(
            json!({"expected":expected,"actual":{"cast_offered":offered},"state_evidence":evidence,"execution_trace":dm.trace}),
        );
    } else {
        let source;
        if def.name() == "Field of Ruin" {
            let hand = g.create_object_from_definition(def, PlayerId(0), Zone::Hand);
            let action = compute_legal_actions(&g, PlayerId(0)).expect("fixture has complete replacement state")
                .into_iter()
                .find(|a| matches!(a,LegalAction::PlayLand{land_id}if *land_id==hand))
                .ok_or("Field legal land play unavailable")?;
            let mut state = PriorityLoopState::new(g.players_in_game());
            apply_priority_response_with_dm(
                &mut g,
                &mut q,
                &mut state,
                &PriorityResponse::PriorityAction(action),
                &mut dm,
            )
            .map_err(|e| e.to_string())?;
        } else {
            let paid = cast(&mut g, def, PlayerId(0), &mut q, &mut dm)?;
            let want = if def.name() == "Fall of the Thran" {
                6
            } else if def.name() == "Zevlor, Elturel Exile" {
                4
            } else {
                3
            };
            if paid != want {
                return Err(format!("source paid {paid} expected {want}"));
            }
            evidence["paid"] = json!(paid);
            resolve_all(&mut g, &mut q, &mut dm).map_err(|e| format!("source setup:{e}"))?;
        }
        source = *g
            .battlefield
            .iter()
            .find(|id| g.object(**id).is_some_and(|o| o.name == def.name()))
            .ok_or("canonical source absent after cast/play")?;
        evidence["source"] = json!(source.0);
        let indexes: Vec<_> = def
            .abilities
            .iter()
            .enumerate()
            .filter(|(_, a)| matches!(a.kind, AbilityKind::Activated(_)))
            .map(|(i, _)| i)
            .collect();
        if def.name() == "Augusta, Order Returned" {
            next_own_turn(&mut g, &mut q, &mut dm)?;
            ironsmith::turn::advance_phase(&mut g).map_err(|e| e.to_string())?;
            ironsmith::turn::advance_step(&mut g).map_err(|e| e.to_string())?;
            let mut combat = CombatState::default();
            let attacker = if variant == 2 { creatures[0] } else { source };
            dm.target = Some(Target::Object(attacker));
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
            .map_err(|e| e.to_string())?;
            g.combat = Some(combat);
            drain(&mut g, &mut q, &mut dm)?;
            evidence["attack_trigger_count"] = json!(g.stack.len());
            error = resolve_all(&mut g, &mut q, &mut dm).err();
        } else if matches!(def.name(), "Fall of the Thran" | "Origin of Thor") {
            if count(&g, source, CounterType::Lore) != 1 {
                return Err("Saga entry lore not one".into());
            }
            if def.name() == "Fall of the Thran"
                && lands.iter().any(|id| g.battlefield.contains(id))
            {
                return Err("Fall actual Chapter I did not destroy seeded lands".into());
            }
            let advances = if def.name() == "Origin of Thor" { 2 } else { 1 };
            if variant == 2 {
                start_next_turn(&mut g, &mut q, &mut dm)?;
                ironsmith::game_loop::add_saga_lore_counters_with_dm(&mut g, &mut q, &mut dm).unwrap();
                error = resolve_all(&mut g, &mut q, &mut dm).err();
            } else {
                let mut final_error = None;
                for i in 0..advances {
                    next_own_turn(&mut g, &mut q, &mut dm)?;
                    dm.target = Some(Target::Object(creatures[0]));
                    ironsmith::game_loop::add_saga_lore_counters_with_dm(&mut g, &mut q, &mut dm).unwrap();
                    let lore = count(&g, source, CounterType::Lore);
                    if lore != i as u32 + 2 {
                        return Err("Saga real lore progression differs".into());
                    }
                    evidence[format!("lore_{}_turn", lore)] = json!(g.turn.turn_number);
                    drain(&mut g, &mut q, &mut dm)?;
                    let e = resolve_all(&mut g, &mut q, &mut dm).err();
                    if e.is_some() {
                        final_error = e;
                        break;
                    }
                }
                error = final_error;
            }
        } else if def.name() == "Field of Ruin" {
            let victim = g.create_object_from_definition(
                &basic("Opponent nonbasic land", CardType::Land),
                PlayerId(1),
                Zone::Battlefield,
            );
            dm.target = Some(Target::Object(victim));
            if variant == 2 {
                error = None;
            } else {
                let a = activation(&g, source, *indexes.last().ok_or("Field no activation")?)
                    .ok_or("Field normal activation unavailable")?;
                let before = g.player(PlayerId(0)).unwrap().mana_pool.total();
                announce(&mut g, &mut q, &mut dm, a)?;
                let paid = before - g.player(PlayerId(0)).unwrap().mana_pool.total();
                if paid != 2 || g.battlefield.contains(&source) {
                    return Err("Field cost payment failed".into());
                }
                evidence["activation_paid"] = json!(paid);
                evidence["source_sacrificed"] = json!(true);
                error = resolve_all(&mut g, &mut q, &mut dm).err();
                evidence["target_destroyed"] = json!(!g.battlefield.contains(&victim));
            }
        } else if def.name() == "Parallax Nexus" {
            let before_hand = g.player(PlayerId(1)).unwrap().hand.len();
            let fade = count(&g, source, CounterType::Fade);
            if fade != 5 {
                return Err(format!("Parallax initial fade {fade}"));
            }
            if variant != 1 {
                dm.target = Some(Target::Player(PlayerId(1)));
                let a = activation(&g, source, *indexes.first().ok_or("Nexus no activation")?)
                    .ok_or("Nexus legal activation unavailable")?;
                announce(&mut g, &mut q, &mut dm, a)?;
                if count(&g, source, CounterType::Fade) != 4 {
                    return Err("Nexus fade cost not paid".into());
                }
                let err = resolve_all(&mut g, &mut q, &mut dm).err();
                if let Some(e) = err {
                    return Ok(
                        json!({"expected":expected,"actual":{"resolution_error":e,"remaining_stack":g.stack.len()},"execution_trace":dm.trace,"state_evidence":evidence}),
                    );
                }
                if g.player(PlayerId(1)).unwrap().hand.len() != before_hand - 1 {
                    return Err("Nexus actual activation did not exile one card".into());
                }
            }
            if variant == 2 {
                error = None;
            } else {
                dm.target = Some(Target::Object(source));
                let bounce = producers.get("Disperse").unwrap();
                if cast(&mut g, bounce, PlayerId(0), &mut q, &mut dm)? != 2 {
                    return Err("Nexus bounce payment failed".into());
                }
                resolve_one(&mut g, &mut q, &mut dm).map_err(|e| format!("bounce producer:{e}"))?;
                if g.battlefield.contains(&source) {
                    return Err("Nexus did not leave".into());
                }
                dm.target = None;
                drain(&mut g, &mut q, &mut dm)?;
                evidence["leave_trigger_count"] = json!(g.stack.len());
                error = resolve_all(&mut g, &mut q, &mut dm).err();
                expected["bob_hand"] = json!(before_hand);
            }
        } else {
            if variant != 2 {
                let a = activation(&g, source, *indexes.first().ok_or("Zevlor no activation")?)
                    .ok_or("Zevlor legal activation unavailable")?;
                let before = g.player(PlayerId(0)).unwrap().mana_pool.total();
                announce(&mut g, &mut q, &mut dm, a)?;
                if before - g.player(PlayerId(0)).unwrap().mana_pool.total() != 2
                    || !g.is_tapped(source)
                {
                    return Err("Zevlor tap/mana costs not paid".into());
                }
                resolve_all(&mut g, &mut q, &mut dm)
                    .map_err(|e| format!("Zevlor activation:{e}"))?;
            }
            dm.target = Some(Target::Player(PlayerId(1)));
            let bolt = producers.get("Lightning Bolt").unwrap();
            if cast(&mut g, bolt, PlayerId(0), &mut q, &mut dm)? != 1 {
                return Err("Bolt payment mismatch".into());
            }
            evidence["producer_targets"] = json!(format!(
                "{:?}",
                g.stack.iter().find(|e| !e.is_ability).map(|e| &e.targets)
            ));
            drain(&mut g, &mut q, &mut dm)?;
            evidence["trigger_and_spell_stack_count"] = json!(g.stack.len());
            dm.target = None;
            error = resolve_all(&mut g, &mut q, &mut dm).err();
            expected["life"] = json!(
                (0..seats)
                    .map(|i| if i == 0 || (variant == 2 && i != 1) {
                        30
                    } else {
                        27
                    })
                    .collect::<Vec<_>>()
            );
        }
    }
    let mut actual = json!({"resolution_error":error,"remaining_stack":g.stack.len()});
    if expected.get("bob_hand").is_some() {
        actual["bob_hand"] = json!(g.player(PlayerId(1)).unwrap().hand.len());
    }
    if expected.get("life").is_some() {
        actual["life"] = json!(g.players.iter().map(|p| p.life).collect::<Vec<_>>());
    }
    evidence["final_hands"] = json!(hand_counts(&g));
    evidence["final_life"] = json!(g.players.iter().map(|p| p.life).collect::<Vec<_>>());
    Ok(
        json!({"expected":expected,"actual":actual,"execution_trace":dm.trace,"state_evidence":evidence}),
    )
}
#[test]
#[ignore = "manual scoped audit capture"]
fn report_remaining_simultaneous() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let inventory =
        root.join("reports/runtime-audit/corpus/267a16aff3b321196397d0b4/inventory.json");
    let source = root.join(
        "crates/ironsmith-tools/tests/runtime_simultaneous_remaining_family_reproductions.rs",
    );
    let binary = std::env::current_exe().unwrap();
    let paths = [&inventory, &source, &binary];
    let before: Vec<_> = paths
        .iter()
        .map(|p| json!({"path":p,"sha256":hash(p)}))
        .collect();
    let inv: Value = serde_json::from_slice(&std::fs::read(&inventory).unwrap()).unwrap();
    let mut compilation = vec![];
    let mut defs = std::collections::HashMap::new();
    for name in [
        "Augusta, Order Returned",
        "Fall of the Thran",
        "Fatal Grudge",
        "Field of Ruin",
        "Origin of Thor",
        "Parallax Nexus",
        "Zevlor, Elturel Exile",
        "Disperse",
        "Lightning Bolt",
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
        defs.insert(name.to_string(), def);
    }
    let mut rows = vec![];
    for name in [
        "Augusta, Order Returned",
        "Fall of the Thran",
        "Fatal Grudge",
        "Field of Ruin",
        "Origin of Thor",
        "Parallax Nexus",
        "Zevlor, Elturel Exile",
    ] {
        let def = &defs[name];
        for (seats, variant) in [(2, 0), (3, 0), (2, 1), (3, 1), (2, 2)] {
            let out = run(def, &defs, seats, variant);
            let (status, v) = match out {
                Ok(v) => {
                    let status = if v["actual"]["resolution_error"].is_string() {
                        "resolution_failed"
                    } else if v["actual"] == v["expected"] {
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
            rows.push(json!({"card":name,"scenario":{"players":seats,"variant":variant},"status":status,"expected":v["expected"],"actual":v["actual"],"execution_trace":v["execution_trace"],"state_evidence":v["state_evidence"],"artifact_checksum":compilation.iter().find(|c|c["card"]==name).unwrap()["artifact_checksum"],"scope":if name=="Fatal Grudge" {"Strict canonical source in hand; actual legal-action computation during own main phase with empty stack, sufficient black/red mana, and either an owned controlled creature/artifact sacrifice or no nonland resources. Missing cast choice is the tested outcome; no cast, cost payment or resolution is claimed."} else {"Strict canonical source and producers. Actual paid casts/legal land play, activation costs, attacks, Saga chapter progression, or zone-change producers with production priority resolution. Variant meanings are card-specific; traces distinguish setup/positive/negative behavior. Fixture setup failures are not promoted. No whole-card verdict."}}));
        }
    }
    let after: Vec<_> = paths
        .iter()
        .map(|p| json!({"path":p,"sha256":hash(p)}))
        .collect();
    let report = json!({"rows":rows,"compilation":compilation,"provenance":{"before":before,"after":after,"artifacts_unchanged":before==after,"strict_artifact":true,"unique_card_ids":true,"seed":71757432704854_u64}});
    std::fs::write(
        root.join("reports/runtime-audit/simultaneous-remaining-family-reproductions.json"),
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
    println!("wrote {} cases", report["rows"].as_array().unwrap().len());
}
