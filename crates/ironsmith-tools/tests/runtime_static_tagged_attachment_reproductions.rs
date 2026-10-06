//! Paid static tagged attachment conditions and exact characteristics/attack outcomes.
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

fn ready_next_own_turn(
    g: &mut GameState,
    q: &mut TriggerQueue,
    dm: &mut Choices,
) -> Result<(), String> {
    for _ in 0..2 {
        g.next_turn();
        ironsmith::turn::execute_untap_step(g);
        ironsmith::turn::advance_step(g).map_err(|e| e.to_string())?;
        ironsmith::turn::advance_step(g).map_err(|e| e.to_string())?;
        for e in ironsmith::turn::execute_draw_step_with(g, dm).unwrap() {
            for t in ironsmith::triggers::check_triggers(g, &e) {
                q.add(t);
            }
        }
        resolve_all(g, q, dm)?;
        ironsmith::turn::advance_phase(g).map_err(|e| e.to_string())?;
    }
    g.turn.priority_player = Some(PlayerId(0));
    mana(g);
    Ok(())
}
fn run(def: &CardDefinition, mode: usize) -> Result<Value, String> {
    use ironsmith::static_abilities::StaticAbilityId as K;
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
    let bracers = def.name() == "Bladed Bracers";
    let research = def.name() == "Combat Research";
    let chariot = def.name() == "Warmonger's Chariot";
    let attach = bracers || research || mode != 2;
    let legendary = !bracers && !chariot && mode != 0;
    let owner = if research && mode == 2 {
        PlayerId(1)
    } else {
        PlayerId(0)
    };
    let subtype = if bracers && mode == 1 {
        ironsmith::Subtype::Human
    } else if bracers && mode == 2 {
        ironsmith::Subtype::Angel
    } else {
        ironsmith::Subtype::Bear
    };
    let mut b = CardDefinitionBuilder::new(CardId::new(), "Neutral attachment recipient")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![subtype])
        .power_toughness(PowerToughness::fixed(2, 6));
    if legendary {
        b = b.supertypes(vec![ironsmith::Supertype::Legendary]);
    }
    if chariot && mode > 0 {
        b = b.defender();
    }
    let recipient = g.create_object_from_definition(&b.build(), owner, Zone::Battlefield);
    for player in [PlayerId(0), PlayerId(1)] {
        for _ in 0..4 {
            g.create_object_from_definition(
                &CardDefinitionBuilder::new(CardId::new(), "Neutral library artifact")
                    .card_types(vec![CardType::Artifact])
                    .build(),
                player,
                Zone::Library,
            );
        }
    }
    dm.target = Some(Target::Object(recipient));
    let source_cost = def.card.mana_cost.as_ref().unwrap().mana_value();
    let paid = cast(&mut g, def, PlayerId(0), &mut q, &mut dm)?;
    if paid != source_cost {
        return Err(format!("source paid{paid} expected{source_cost}"));
    }
    resolve_all(&mut g, &mut q, &mut dm)?;
    let source = find(&g, def.name())?;
    if !research && attach {
        let cost = if def.name() == "Champion's Helm" {
            1
        } else if chariot {
            3
        } else {
            2
        };
        paid_activate(&mut g, &mut q, &mut dm, source, cost)?;
    }
    let attached = g.object(source).unwrap().attached_to
        == Some(ironsmith::object::AttachmentTarget::Object(recipient));
    if attached != attach {
        return Err(format!("attachment expected{attach} actual{attached}"));
    }
    let mut expected = json!({"attached":attach,"power":2,"toughness":6,"vigilance":false,"hexproof":false,"menace":false,"trample":false,"haste":false,"ward":false,"remaining_stack":0});
    if attach {
        let (p, t) = match def.name() {
            "Bladed Bracers" => (3, 7),
            "Champion's Helm" => (4, 8),
            "Gimli's Axe" => (5, 6),
            "Hero's Heirloom" => (4, 7),
            "Warmonger's Chariot" => (4, 8),
            "Combat Research" => {
                if legendary {
                    (3, 7)
                } else {
                    (2, 6)
                }
            }
            _ => unreachable!(),
        };
        expected["power"] = json!(p);
        expected["toughness"] = json!(t);
    }
    expected["vigilance"] = json!(bracers && mode > 0);
    expected["hexproof"] = json!(def.name() == "Champion's Helm" && attach && legendary);
    expected["menace"] = json!(def.name() == "Gimli's Axe" && attach && legendary);
    expected["trample"] = json!(def.name() == "Hero's Heirloom" && attach && legendary);
    expected["haste"] = expected["trample"].clone();
    expected["ward"] = json!(research && legendary);
    let mut actual = json!({"attached":attached,"power":g.calculated_power(recipient),"toughness":g.calculated_toughness(recipient),"vigilance":g.object_has_static_ability_id(recipient,K::Vigilance),"hexproof":g.object_has_static_ability_id(recipient,K::Hexproof),"menace":g.object_has_static_ability_id(recipient,K::Menace),"trample":g.object_has_static_ability_id(recipient,K::Trample),"haste":g.object_has_static_ability_id(recipient,K::Haste),"ward":g.object_has_static_ability_id(recipient,K::Ward),"remaining_stack":g.stack.len()});
    let mut evidence = json!({"mode":mode,"source_cast_paid":paid,"recipient_owner":owner.0,"recipient_legendary":legendary,"recipient_subtype":format!("{subtype:?}"),"recipient_defender":g.object_has_static_ability_id(recipient,K::Defender),"source_abilities":format!("{:?}",g.current_abilities(source)),"recipient_abilities":format!("{:?}",g.current_abilities(recipient))});
    if bracers || chariot {
        ready_next_own_turn(&mut g, &mut q, &mut dm)?;
        if g.is_summoning_sick(recipient) {
            return Err("recipient still summoning sick after actual turn cycle".into());
        }
        ironsmith::turn::advance_phase(&mut g).map_err(|e| e.to_string())?;
        ironsmith::turn::advance_step(&mut g).map_err(|e| e.to_string())?;
        let mut combat = ironsmith::combat_state::CombatState::default();
        let target = ironsmith::combat_state::AttackTarget::Player(PlayerId(1));
        let options = ironsmith::decision::compute_legal_attackers(&g, &combat);
        let offered = options
            .iter()
            .any(|a| a.creature == recipient && a.valid_targets.contains(&target));
        let error = if offered {
            ironsmith::game_loop::apply_attacker_declarations_with_dm(
                &mut g,
                &mut combat,
                &mut q,
                &[ironsmith::decision::AttackerDeclaration {
                    creature: recipient,
                    target,
                }],
                &mut dm,
            )
            .err()
            .map(|e| e.to_string())
        } else {
            None
        };
        let attacked = ironsmith::combat_state::is_attacking(&combat, recipient);
        g.combat = Some(combat);
        resolve_all(&mut g, &mut q, &mut dm)?;
        let want_attack = !(chariot && mode == 2);
        expected["attack_offered"] = json!(want_attack);
        expected["attack_declared"] = json!(want_attack);
        expected["attack_error"] = Value::Null;
        expected["recipient_tapped"] = json!(want_attack && !(bracers && mode > 0));
        actual["attack_offered"] = json!(offered);
        actual["attack_declared"] = json!(attacked);
        actual["attack_error"] = json!(error);
        actual["recipient_tapped"] = json!(g.is_tapped(recipient));
        evidence["attack_options"] = json!(format!("{options:?}"));
        evidence["turn_after_ready_cycle"] = json!(g.turn.turn_number);
    }
    Ok(
        json!({"expected":expected,"actual":actual,"state_evidence":evidence,"execution_trace":dm.trace}),
    )
}
#[test]
#[ignore = "manual scoped static tagged attachment report"]
fn report_static_tagged_attachments() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let inventory =
        root.join("reports/runtime-audit/corpus/267a16aff3b321196397d0b4/inventory.json");
    let source =
        root.join("crates/ironsmith-tools/tests/runtime_static_tagged_attachment_reproductions.rs");
    let binary = std::env::current_exe().unwrap();
    let paths = [&inventory, &source, &binary];
    let before: Vec<_> = paths
        .iter()
        .map(|p| json!({"path":p,"sha256":hash(p)}))
        .collect();
    let inv: Value = serde_json::from_slice(&std::fs::read(&inventory).unwrap()).unwrap();
    let names = [
        "Bladed Bracers",
        "Champion's Helm",
        "Combat Research",
        "Gimli's Axe",
        "Hero's Heirloom",
        "Warmonger's Chariot",
    ];
    let mut compile = vec![];
    let mut defs = std::collections::HashMap::new();
    for name in names {
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
            let (status, out) = match run(&defs[name], mode) {
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
            rows.push(json!({"card":name,"scenario":{"mode":mode},"status":status,"expected":out["expected"],"actual":out["actual"],"state_evidence":out["state_evidence"],"execution_trace":out["execution_trace"],"artifact_checksum":compile.iter().find(|c|c["card"]==name).unwrap()["artifact_checksum"],"scope":"Actual paid canonical Aura/Equipment cast and ordinary Equip. Established vanilla2/6 recipient varies printed condition attributes. Exact P/T and granted keywords checked; Bracers/Chariot additionally use actual next-turn untap and legal attack declaration."}));
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
            root.join("reports/runtime-audit/static-tagged-attachment-reproductions.json")
        });
    std::fs::write(out, serde_json::to_string_pretty(&report).unwrap()).unwrap();
    println!("wrote{}cases", report["rows"].as_array().unwrap().len());
}
