//! Canonical sources, paid producers and real phase/death/cast trigger events.
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
use ironsmith::mana::{ManaCost, ManaSymbol};
use ironsmith::target::ChooseSpec;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{
    CardDefinition, CardId, CardType, Effect, GameState, ObjectId, PlayerId, PowerToughness, Zone,
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
fn run(
    def: &CardDefinition,
    seats: usize,
    accept: bool,
    empty: bool,
    negative: bool,
    graveyard_spells: usize,
) -> Result<Value, String> {
    let mut g = GameState::new((0..seats).map(|i| format!("Seat{i}")).collect(), 30);
    g.set_random_seed(71757432704852);
    g.turn.turn_number = 3;
    g.turn.active_player = PlayerId(0);
    g.turn.priority_player = Some(PlayerId(0));
    g.turn.phase = ironsmith::Phase::FirstMain;
    g.turn.step = None;
    let artifact = basic("Simultaneous graveyard/library card", CardType::Artifact);
    let land = basic("Simultaneous land resource", CardType::Land);
    let creature = basic("Simultaneous creature resource", CardType::Creature);
    for seat in 0..seats {
        let p = PlayerId(seat as u8);
        for _ in 0..6 {
            g.create_object_from_definition(&land, p, Zone::Library);
            g.create_object_from_definition(&artifact, p, Zone::Library);
        }
        if seat > 0 {
            g.create_object_from_definition(&artifact, p, Zone::Graveyard);
        }
        if !empty {
            g.create_object_from_definition(&land, p, Zone::Hand);
            g.create_object_from_definition(&artifact, p, Zone::Hand);
            g.create_object_from_definition(&creature, p, Zone::Battlefield);
        }
        for symbol in [
            ManaSymbol::White,
            ManaSymbol::Blue,
            ManaSymbol::Black,
            ManaSymbol::Red,
            ManaSymbol::Green,
        ] {
            g.player_mut(p).unwrap().mana_pool.add(symbol, 1);
        }
        g.player_mut(p)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Colorless, 8);
    }
    for i in 0..graveyard_spells {
        g.create_object_from_definition(
            &basic(
                "Historical instant/sorcery graveyard resource",
                if i % 2 == 0 {
                    CardType::Instant
                } else {
                    CardType::Sorcery
                },
            ),
            PlayerId(0),
            Zone::Graveyard,
        );
    }
    let mut dm = Choices {
        accept,
        target: None,
        trace: vec![],
    };
    let mut q = TriggerQueue::new();
    let paid = cast(&mut g, def, PlayerId(0), &mut q, &mut dm)?;
    let expected_paid = match def.name() {
        "Consuming Aberration" | "Dr. Eggman" | "Fandaniel, Telophoroi Ascian" => 5,
        "Kynaios and Tiro of Meletis" | "Tempting Contract" => 4,
        _ => 3,
    };
    if paid != expected_paid {
        return Err(format!(
            "source payment {paid} differs from {expected_paid}"
        ));
    }
    resolve_all(&mut g, &mut q, &mut dm)
        .map_err(|e| format!("source setup cast resolution:{e}"))?;
    let source = *g
        .battlefield
        .iter()
        .find(|id| g.object(**id).is_some_and(|o| o.name == def.name()))
        .ok_or("canonical source did not remain on battlefield")?;
    dm.trace.push(json!({"stage":"source_established","source":source.0,"source_power":g.calculated_power(source),"source_toughness":g.calculated_toughness(source),"paid":paid}));
    g.effect_store.pending_trigger_events.clear();
    let producer;
    if def.name() == "Consuming Aberration" {
        let actor = if negative { PlayerId(1) } else { PlayerId(0) };
        let spell = CardDefinitionBuilder::new(CardId::new(), "Paid cast trigger producer")
            .card_types(vec![CardType::Instant])
            .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Blue]))
            .with_spell_effect(vec![Effect::gain_life(1)])
            .build();
        g.player_mut(actor)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Blue, 1);
        if cast(&mut g, &spell, actor, &mut q, &mut dm)? != 1 {
            return Err("producer did not pay one mana".into());
        }
        producer = json!({"kind":"paid_spell_cast","actor":actor.index(),"spells_cast":g.turn_store.turn_history.spells_cast_by_player(actor)});
    } else if def.name() == "Sadistic Augermage" {
        let victim = if negative {
            g.create_object_from_definition(&creature, PlayerId(0), Zone::Battlefield)
        } else {
            source
        };
        dm.target = Some(victim);
        let spell = CardDefinitionBuilder::new(CardId::new(), "Paid death trigger producer")
            .card_types(vec![CardType::Instant])
            .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Black]))
            .with_spell_effect(vec![Effect::destroy(ChooseSpec::target(
                ChooseSpec::creature(),
            ))])
            .build();
        g.player_mut(PlayerId(0))
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Black, 1);
        if cast(&mut g, &spell, PlayerId(0), &mut q, &mut dm)? != 1 {
            return Err("death producer payment mismatch".into());
        }
        resolve_one(&mut g, &mut q, &mut dm)
            .map_err(|e| format!("death producer resolution:{e}"))?;
        if !negative && g.battlefield.contains(&source) {
            return Err("canonical source did not die".into());
        }
        dm.target = None;
        producer = json!({"kind":"paid_destroy_spell","victim":victim.0,"canonical_source_left":!g.battlefield.contains(&source)});
    } else {
        let upkeep = matches!(
            def.name(),
            "Descent into Avernus" | "Orzhov Advokist" | "Tempting Contract"
        );
        let turns = if upkeep {
            if negative { 1 } else { seats }
        } else {
            usize::from(negative)
        };
        for _ in 0..turns {
            g.next_turn();
            ironsmith::turn::execute_untap_step(&mut g);
        }
        if upkeep {
            ironsmith::turn::advance_step(&mut g).map_err(|e| e.to_string())?;
            if g.turn.step != Some(ironsmith::Step::Upkeep) {
                return Err(format!("upkeep transition wrong:{:?}", g.turn));
            }
        } else {
            if turns > 0 {
                ironsmith::turn::advance_phase(&mut g).map_err(|e| e.to_string())?;
            }
            for _ in 0..3 {
                ironsmith::turn::advance_phase(&mut g).map_err(|e| e.to_string())?;
            }
            if g.turn.step != Some(ironsmith::Step::End) {
                return Err(format!("end transition wrong:{:?}", g.turn));
            }
        }
        let event = ironsmith::triggers::generate_step_trigger_events(&g)
            .ok_or("no phase event generated")?;
        for t in ironsmith::triggers::check_triggers(&g, &event) {
            q.add(t);
        }
        producer = json!({"kind":"phase_begin","active_player":g.turn.active_player.index(),"turn":g.turn.turn_number,"phase":format!("{:?}",g.turn.phase),"step":format!("{:?}",g.turn.step),"actual_turn_transitions":turns});
    }
    drain(&mut g, &mut q, &mut dm)?;
    let triggers = g
        .stack
        .iter()
        .filter(|e| e.is_ability && e.object_id == source)
        .count();
    dm.trace.push(json!({"stage":"trigger_created","producer":producer,"trigger_count":triggers,"stack":format!("{:?}",g.stack)}));
    let error = resolve_all(&mut g, &mut q, &mut dm).err();
    let mut actual =
        json!({"trigger_count":triggers,"resolution_error":error,"remaining_stack":g.stack.len()});
    if def.name() == "Kynaios and Tiro of Meletis" {
        actual["hand_counts"] = json!(
            (0..seats)
                .map(|seat| g.player(PlayerId(seat as u8)).unwrap().hand.len())
                .collect::<Vec<_>>()
        );
    }
    if def.name() == "Fandaniel, Telophoroi Ascian" {
        actual["life"] = json!(
            (0..seats)
                .map(|seat| g.player(PlayerId(seat as u8)).unwrap().life)
                .collect::<Vec<_>>()
        );
    }
    let states:Vec<_>=(0..seats).map(|seat|{let p=PlayerId(seat as u8);let player=g.player(p).unwrap();json!({"player":seat,"life":player.life,"hand":player.hand.len(),"library":player.library.len(),"graveyard":player.graveyard.len(),"battlefield":g.battlefield.iter().filter_map(|id|g.object(*id)).filter(|o|g.controller_of(o)==p).map(|o|json!({"name":o.name.to_string(),"kind":format!("{:?}",o.kind),"counters":format!("{:?}",o.counters)})).collect::<Vec<_>>()})}).collect();
    Ok(json!({"actual":actual,"execution_trace":dm.trace,"state_evidence":states}))
}
#[test]
#[ignore = "manual scoped trigger audit"]
fn report_simultaneous_trigger_family() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let inventory =
        root.join("reports/runtime-audit/corpus/267a16aff3b321196397d0b4/inventory.json");
    let source = root
        .join("crates/ironsmith-tools/tests/runtime_simultaneous_trigger_family_reproductions.rs");
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
        "Consuming Aberration",
        "Descent into Avernus",
        "Dr. Eggman",
        "Kynaios and Tiro of Meletis",
        "Orzhov Advokist",
        "Sadistic Augermage",
        "Tempting Contract",
        "Fandaniel, Telophoroi Ascian",
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
        let mut cases = vec![
            (2, true, false, false, 0),
            (2, false, false, false, 0),
            (3, true, false, false, 0),
            (3, false, false, false, 0),
            (2, false, true, false, 0),
            (2, false, false, true, 0),
        ];
        if name == "Fandaniel, Telophoroi Ascian" {
            for n in 1..=2 {
                for (seats, accept, empty) in [
                    (2, true, false),
                    (2, false, false),
                    (3, true, false),
                    (3, false, false),
                    (2, false, true),
                ] {
                    cases.push((seats, accept, empty, false, n));
                }
            }
        }
        for (seats, accept, empty, negative, graveyard_spells) in cases {
            let mut expected = json!({"trigger_count":usize::from(!negative),"resolution_error":null,"remaining_stack":0});
            if name == "Kynaios and Tiro of Meletis" {
                let base = if empty { 0 } else { 2 };
                expected["hand_counts"] = json!(
                    (0..seats)
                        .map(|seat| if negative {
                            base
                        } else if seat == 0 {
                            base + 1 - usize::from(accept && !empty)
                        } else if accept && !empty {
                            base - 1
                        } else {
                            base + 1
                        })
                        .collect::<Vec<_>>()
                );
            }
            if name == "Fandaniel, Telophoroi Ascian" {
                expected["life"] = json!(
                    (0..seats)
                        .map(|seat| if seat == 0 || negative || (accept && !empty) {
                            30
                        } else {
                            30 - 2 * graveyard_spells as i32
                        })
                        .collect::<Vec<_>>()
                );
            }
            let (status, actual, trace, state) =
                match run(&def, seats, accept, empty, negative, graveyard_spells) {
                    Ok(r) => {
                        let a = r["actual"].clone();
                        (
                            if a["resolution_error"].is_string() {
                                "resolution_failed"
                            } else if a == expected {
                                "expected_result_observed"
                            } else {
                                "semantic_mismatch"
                            },
                            a,
                            r["execution_trace"].clone(),
                            r["state_evidence"].clone(),
                        )
                    }
                    Err(e) => (
                        "fixture_or_execution_error",
                        json!({"error":e}),
                        Value::Null,
                        Value::Null,
                    ),
                };
            rows.push(json!({"card":name,"scenario":{"players":seats,"optional_policy":accept,"empty_hand_and_non_source_creature_resources":empty,"negative_trigger_control":negative,"own_graveyard_instants_and_sorceries":graveyard_spells},"status":status,"expected":expected,"actual":actual,"execution_trace":trace,"state_evidence":state,"artifact_checksum":artifact.payload_checksum,"scope":"Canonical source is actually cast and paid. Production priority passes resolve source and producer spells, retaining dedicated ETB/death trigger queue. Events come from actual paid spell casts/destruction or real turn/phase transitions and engine phase event generation. Assertions cover trigger count and exception-free completion, plus Kynaios hand counts and Fandaniel life totals. Other state is recorded without a complete semantic oracle. Optional policies do not imply a choice callback was reached."}));
        }
    }
    let after: Vec<_> = paths
        .iter()
        .map(|p| json!({"path":p,"sha256":hash(p)}))
        .collect();
    let report = json!({"rows":rows,"compilation":compilation,"provenance":{"before":before,"after":after,"artifacts_unchanged":before==after,"strict_artifact":true,"unique_card_ids":true,"seed":71757432704852_u64}});
    std::fs::write(
        root.join("reports/runtime-audit/simultaneous-trigger-family-reproductions.json"),
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
    println!("wrote {} cases", report["rows"].as_array().unwrap().len());
}
