//! Exact outcomes for static conditions that reference a recipient through TargetMatches.
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
fn paid_activate(
    g: &mut GameState,
    q: &mut TriggerQueue,
    dm: &mut Choices,
    source: ObjectId,
    expected: u32,
) -> Result<(), String> {
    g.turn.priority_player = Some(PlayerId(0));
    let a = compute_legal_actions(g, PlayerId(0)).expect("fixture has complete replacement state")
        .into_iter()
        .find(|a| matches!(a,LegalAction::ActivateAbility{source:s,..}if *s==source))
        .ok_or("printed activation not offered")?;
    let before = g.player(PlayerId(0)).unwrap().mana_pool.total();
    announce(g, q, dm, a)?;
    let paid = before - g.player(PlayerId(0)).unwrap().mana_pool.total();
    if paid != expected {
        return Err(format!("activation paid{paid} expected{expected}"));
    }
    dm.trace
        .push(json!({"stage":"verified_activation_payment","mana":paid,"source":source.0}));
    resolve_all(g, q, dm)
}
fn run(def: &CardDefinition, land: &CardDefinition, mode: usize) -> Result<Value, String> {
    let mut g = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    g.set_random_seed(71757432704855);
    g.turn.turn_number = 3;
    g.turn.active_player = PlayerId(0);
    g.turn.priority_player = Some(PlayerId(0));
    g.turn.phase = ironsmith::Phase::FirstMain;
    g.turn.step = None;
    mana(&mut g);
    let mut q = TriggerQueue::new();
    let mut dm = Choices {
        accept: false,
        target: None,
        trace: vec![],
    };
    let mut evidence = json!({"mode":mode,"fixture":"Established otherwise vanilla 2/6 recipient; condition attributes set at card construction, not by engine mutation. Source actions are canonical full-input paid actions."});
    let (recipient, want_power, want_toughness, want_creature, want_attached);
    let mut source = None;
    if def.name() == "Earth Surge" {
        let hand = g.create_object_from_definition(land, PlayerId(0), Zone::Hand);
        let a = compute_legal_actions(&g, PlayerId(0)).expect("fixture has complete replacement state")
            .into_iter()
            .find(|a| matches!(a,LegalAction::PlayLand{land_id}if *land_id==hand))
            .ok_or("Mutavault legal land play unavailable")?;
        let mut state = PriorityLoopState::new(g.players_in_game());
        apply_priority_response_with_dm(
            &mut g,
            &mut q,
            &mut state,
            &PriorityResponse::PriorityAction(a.clone()),
            &mut dm,
        )
        .map_err(|e| e.to_string())?;
        dm.trace
            .push(json!({"stage":"real_land_play","action":format!("{a:?}")}));
        resolve_all(&mut g, &mut q, &mut dm)?;
        recipient = find(&g, land.name())?;
        if mode > 0 {
            let paid = cast(&mut g, def, PlayerId(0), &mut q, &mut dm)?;
            if paid != 4 {
                return Err("Earth Surge payment not4".into());
            }
            resolve_all(&mut g, &mut q, &mut dm)?;
            source = Some(find(&g, def.name())?);
            evidence["source_cast_paid"] = json!(paid);
        }
        if mode != 1 {
            paid_activate(&mut g, &mut q, &mut dm, recipient, 1)?;
        }
        want_creature = mode != 1;
        want_power = if mode == 1 {
            None
        } else {
            Some(if mode == 0 { 2 } else { 4 })
        };
        want_toughness = want_power;
        want_attached = false;
        evidence["fixture"] = json!(
            "Actual Mutavault land play and its printed paid one-mana animation; Earth Surge paid normally when present."
        );
    } else {
        let owner = if def.name() != "Toralf's Hammer" && mode == 2 {
            PlayerId(1)
        } else {
            PlayerId(0)
        };
        let detective = def.name() == "Burden of Proof" && mode != 1;
        let white = def.name() == "Favorable Destiny" && mode != 1;
        let legendary = def.name() == "Toralf's Hammer" && mode != 0;
        let mut b = CardDefinitionBuilder::new(CardId::new(), "Neutral condition recipient")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 6))
            .subtypes(vec![if detective {
                ironsmith::Subtype::Detective
            } else {
                ironsmith::Subtype::Bear
            }])
            .color_indicator(if white {
                ironsmith::color::ColorSet::WHITE
            } else {
                ironsmith::color::ColorSet::BLUE
            });
        if legendary {
            b = b.supertypes(vec![ironsmith::Supertype::Legendary]);
        }
        recipient = g.create_object_from_definition(&b.build(), owner, Zone::Battlefield);
        evidence["recipient_controller"] = json!(owner.0);
        evidence["recipient_detective"] = json!(detective);
        evidence["recipient_white"] = json!(white);
        evidence["recipient_legendary"] = json!(legendary);
        dm.target = Some(Target::Object(recipient));
        let paid = cast(&mut g, def, PlayerId(0), &mut q, &mut dm)?;
        if paid != 2 {
            return Err(format!("{} source payment{paid} expected2", def.name()));
        }
        resolve_all(&mut g, &mut q, &mut dm)?;
        let id = find(&g, def.name())?;
        source = Some(id);
        evidence["source_cast_paid"] = json!(paid);
        if def.name() == "Toralf's Hammer" && mode != 2 {
            paid_activate(&mut g, &mut q, &mut dm, id, 2)?;
        }
        want_creature = true;
        want_attached = def.name() != "Toralf's Hammer" || mode != 2;
        let (p, t) = match def.name() {
            "Burden of Proof" => {
                if mode == 0 {
                    (4, 8)
                } else {
                    (1, 1)
                }
            }
            "Favorable Destiny" => {
                if white {
                    (3, 8)
                } else {
                    (2, 6)
                }
            }
            "Toralf's Hammer" => {
                if mode == 1 {
                    (5, 6)
                } else {
                    (2, 6)
                }
            }
            _ => return Err("unexpected source".into()),
        };
        want_power = Some(p);
        want_toughness = Some(t);
    }
    let actual = json!({"recipient_power":g.calculated_power(recipient),"recipient_toughness":g.calculated_toughness(recipient),"recipient_is_creature":g.calculated_card_types(recipient).contains(&CardType::Creature),"source_attached_to_recipient":source.is_some_and(|id|g.object(id).is_some_and(|o|o.attached_to==Some(ironsmith::object::AttachmentTarget::Object(recipient)))),"remaining_stack":g.stack.len()});
    let expected = json!({"recipient_power":want_power,"recipient_toughness":want_toughness,"recipient_is_creature":want_creature,"source_attached_to_recipient":want_attached,"remaining_stack":0});
    evidence["source_id"] = json!(source.map(|s| s.0));
    evidence["recipient_id"] = json!(recipient.0);
    evidence["source_abilities"] = json!(source.map(|id| format!("{:?}", g.current_abilities(id))));
    Ok(
        json!({"expected":expected,"actual":actual,"state_evidence":evidence,"execution_trace":dm.trace}),
    )
}
#[test]
#[ignore = "manual scoped static recipient report"]
fn report_static_recipient_conditions() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let inventory =
        root.join("reports/runtime-audit/corpus/267a16aff3b321196397d0b4/inventory.json");
    let source = root
        .join("crates/ironsmith-tools/tests/runtime_static_recipient_condition_reproductions.rs");
    let binary = std::env::current_exe().unwrap();
    let paths = [&inventory, &source, &binary];
    let before: Vec<_> = paths
        .iter()
        .map(|p| json!({"path":p,"sha256":hash(p)}))
        .collect();
    let inv: Value = serde_json::from_slice(&std::fs::read(&inventory).unwrap()).unwrap();
    let names = [
        "Burden of Proof",
        "Earth Surge",
        "Favorable Destiny",
        "Toralf's Hammer",
    ];
    let mut compile = vec![];
    let mut defs = std::collections::HashMap::new();
    for name in names.iter().copied().chain(["Mutavault"]) {
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
        for mode in 0..3 {
            if std::env::var("AUDIT_STATIC_MODE").is_ok_and(|n| n != mode.to_string()) {
                continue;
            }
            eprintln!("AUDIT_CASE {name} {mode}");
            let (status, out) = match run(&defs[name], &defs["Mutavault"], mode) {
                Ok(out) => {
                    let status = if out["expected"] == out["actual"] {
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
            rows.push(json!({"card":name,"scenario":{"mode":mode},"status":status,"expected":out["expected"],"actual":out["actual"],"state_evidence":out["state_evidence"],"execution_trace":out["execution_trace"],"artifact_checksum":compile.iter().find(|c|c["card"]==name).unwrap()["artifact_checksum"],"scope":"Printed two/four-mana source cast, normal Aura target/Equip or actual Mutavault land play plus animation. Compare exact recipient P/T and attachment; this does not validate all source abilities."}));
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
            root.join("reports/runtime-audit/static-recipient-condition-reproductions.json")
        });
    std::fs::write(out, serde_json::to_string_pretty(&report).unwrap()).unwrap();
    println!("wrote{}cases", report["rows"].as_array().unwrap().len());
}
