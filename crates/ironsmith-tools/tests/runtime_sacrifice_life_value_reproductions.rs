//! Strict paid casts at controller, life-payment, X and sacrificed-power boundaries.
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{
    DecisionMaker, GameProgress, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::{
    BooleanContext, NumberContext, SelectOptionsContext, TargetsContext,
};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, resolve_stack_entry_with,
};
use ironsmith::game_state::Target;
use ironsmith::mana::ManaSymbol;
use ironsmith::object::ObjectKind;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{
    CardDefinition, CardId, CardType, GameState, ObjectId, PlayerId, PowerToughness, Zone,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
fn hash(path: &std::path::Path) -> String {
    Sha256::digest(std::fs::read(path).unwrap())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
struct Choices {
    target: Option<ObjectId>,
    x: u32,
    pay: bool,
    trace: Vec<Value>,
}
impl DecisionMaker for Choices {
    fn answers_player_choices(&self) -> bool {
        false
    }
    fn decide_number(&mut self, _: &GameState, c: &NumberContext) -> u32 {
        let n = if c.is_x_value {
            self.x.clamp(c.min, c.max)
        } else {
            c.min
        };
        self.trace
            .push(json!({"choice":"number","context":format!("{c:?}"),"selected":n}));
        n
    }
    fn decide_boolean(&mut self, _: &GameState, c: &BooleanContext) -> bool {
        self.trace
            .push(json!({"choice":"boolean","context":format!("{c:?}"),"selected":self.pay}));
        self.pay
    }
    fn decide_options(&mut self, g: &GameState, c: &SelectOptionsContext) -> Vec<usize> {
        let selected = SelectFirstDecisionMaker.decide_options(g, c);
        self.trace
            .push(json!({"choice":"options","context":format!("{c:?}"),"selected":selected}));
        selected
    }
    fn decide_targets(&mut self, _: &GameState, c: &TargetsContext) -> Vec<Target> {
        let selected = self
            .target
            .map(Target::Object)
            .into_iter()
            .filter(|t| c.requirements.iter().any(|r| r.legal_targets.contains(t)))
            .collect::<Vec<_>>();
        self.trace.push(json!({"choice":"targets","context":format!("{c:?}"),"selected":format!("{selected:?}")}));
        selected
    }
}
fn creature(name: &str, power: i32) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(power, 6))
        .build()
}
fn run(
    def: &CardDefinition,
    n: i32,
    other: u32,
    pay: bool,
    empty_or_illegal: bool,
) -> Result<Value, String> {
    let mercy = def.name() == "Mercy Killing";
    let mut g = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    g.set_random_seed(71757432704851);
    g.turn.turn_number = 3;
    g.turn.phase = ironsmith::Phase::FirstMain;
    g.turn.step = None;
    g.turn.active_player = PlayerId(0);
    g.turn.priority_player = Some(PlayerId(0));
    let cost = if mercy { 3 } else { n as u32 + 1 };
    g.player_mut(PlayerId(0)).unwrap().mana_pool.add(
        if mercy {
            ManaSymbol::Green
        } else {
            ManaSymbol::Black
        },
        cost,
    );
    let mut dm = Choices {
        target: None,
        x: n as u32,
        pay,
        trace: vec![],
    };
    let mut initial = vec![];
    if mercy {
        let victim = g.create_object_from_definition(
            &creature("Sacrifice power boundary target", n),
            PlayerId(other as u8),
            Zone::Battlefield,
        );
        initial.push(victim);
        dm.target = Some(victim);
        g.create_object_from_definition(
            &creature("Untargeted control", 2),
            PlayerId(0),
            Zone::Battlefield,
        );
    } else if !empty_or_illegal {
        for player in [PlayerId(0), PlayerId(1)] {
            initial.push(g.create_object_from_definition(
                &creature("Wave victim", 2),
                player,
                Zone::Battlefield,
            ));
        }
    }
    g.effect_store.pending_trigger_events.clear();
    let spell = g.create_object_from_definition(def, PlayerId(0), Zone::Hand);
    let action = compute_legal_actions(&g, PlayerId(0)).expect("fixture has complete replacement state")
        .into_iter()
        .find(|a| matches!(a,LegalAction::CastSpell{spell_id,..}if *spell_id==spell))
        .ok_or("normal paid cast unavailable")?;
    dm.trace.push(json!({"choice":"legal_cast","action":format!("{action:?}"),"initial_power":initial.iter().map(|id|g.calculated_power(*id)).collect::<Vec<_>>(),"initial_controllers":initial.iter().map(|id|g.controller_of(g.object(*id).unwrap()).index()).collect::<Vec<_>>()}));
    let mut q = TriggerQueue::new();
    let mut state = PriorityLoopState::new(g.players_in_game());
    let mut progress = apply_priority_response_with_dm(
        &mut g,
        &mut q,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        &mut dm,
    )
    .map_err(|e| e.to_string())?;
    for _ in 0..24 {
        if state.pending_cast.is_none() && !g.stack.is_empty() {
            break;
        }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else {
            return Err(format!("cast stalled:{progress:?}"));
        };
        progress = apply_decision_context_with_dm(&mut g, &mut q, &mut state, &ctx, &mut dm)
            .map_err(|e| e.to_string())?;
    }
    if g.stack.len() != 1 || g.player(PlayerId(0)).unwrap().mana_pool.total() != 0 {
        return Err("cast did not pay exact mana and complete".into());
    }
    if !mercy && g.stack[0].x_value != Some(n as u32) {
        return Err(format!("announced X mismatch:{:?}", g.stack[0].x_value));
    }
    dm.trace.push(json!({"choice":"cast_complete","paid_mana":cost,"announced_x":g.stack[0].x_value,"stack_targets":format!("{:?}",g.stack[0].targets)}));
    let announced_targets = g.stack[0].targets.len();
    let target_decision_offered = dm.trace.iter().any(|entry| entry["choice"] == "targets");
    let error = resolve_stack_entry_with(&mut g, &mut dm)
        .err()
        .map(|e| e.to_string());
    let token_counts: Vec<_> = [PlayerId(0), PlayerId(1)]
        .iter()
        .map(|p| {
            g.battlefield
                .iter()
                .filter(|id| {
                    g.object(**id).is_some_and(|o| {
                        g.controller_of(o) == *p && matches!(o.kind, ObjectKind::Token)
                    })
                })
                .count()
        })
        .collect();
    let creature_counts: Vec<_> = [PlayerId(0), PlayerId(1)]
        .iter()
        .map(|p| {
            g.battlefield
                .iter()
                .filter(|id| {
                    g.object(**id).is_some_and(|o| {
                        g.controller_of(o) == *p && o.card_types.contains(&CardType::Creature)
                    })
                })
                .count()
        })
        .collect();
    let graveyard_counts: Vec<_> = [PlayerId(0), PlayerId(1)]
        .iter()
        .map(|p| {
            g.player(*p)
                .unwrap()
                .graveyard
                .iter()
                .filter(|id| {
                    g.object(**id)
                        .is_some_and(|o| o.card_types.contains(&CardType::Creature))
                })
                .count()
        })
        .collect();
    let actual = json!({"resolution_error":error,"life":[g.player(PlayerId(0)).unwrap().life,g.player(PlayerId(1)).unwrap().life],"battlefield_creatures":creature_counts,"graveyard_creatures":graveyard_counts,"tokens":token_counts,"remaining_stack":g.stack.len(),"mana_paid":cost,"announced_targets":announced_targets,"target_decision_offered":target_decision_offered});
    Ok(
        json!({"actual":actual,"execution_trace":dm.trace,"state_evidence":{"original_target_objects":initial.iter().map(|id|format!("{:?}",g.object(*id))).collect::<Vec<_>>(),"battlefield":g.battlefield.iter().map(|id|format!("{:?}",g.object(*id))).collect::<Vec<_>>()}}),
    )
}
#[test]
#[ignore = "manual expected-result audit"]
fn report_sacrifice_life_values() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let inventory =
        root.join("reports/runtime-audit/corpus/267a16aff3b321196397d0b4/inventory.json");
    let source =
        root.join("crates/ironsmith-tools/tests/runtime_sacrifice_life_value_reproductions.rs");
    let binary = std::env::current_exe().unwrap();
    let paths = [&inventory, &source, &binary];
    let before: Vec<_> = paths
        .iter()
        .map(|p| json!({"path":p,"sha256":hash(p)}))
        .collect();
    let inv: Value = serde_json::from_slice(&std::fs::read(&inventory).unwrap()).unwrap();
    let mut compilation = vec![];
    let mut rows = vec![];
    for name in ["Killing Wave", "Mercy Killing"] {
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
        let cases: Vec<_> = if name == "Mercy Killing" {
            vec![
                (0, 0, false, false),
                (2, 0, false, false),
                (5, 0, false, false),
                (0, 1, false, false),
                (2, 1, false, false),
                (5, 1, false, false),
            ]
        } else {
            vec![
                (0, 0, true, false),
                (0, 0, false, false),
                (2, 0, true, false),
                (2, 0, false, false),
                (0, 0, true, true),
                (2, 0, false, true),
            ]
        };
        for (n, other, pay, special) in cases {
            let (mut creatures, mut graveyard, mut tokens) = (vec![0, 0], vec![0, 0], vec![0, 0]);
            let life;
            if name == "Mercy Killing" {
                creatures[0] = 1;
                if !special {
                    tokens[other as usize] = n;
                    creatures[other as usize] += n;
                    graveyard[other as usize] = 1;
                }
                life = vec![20, 20];
            } else {
                if !special {
                    if pay {
                        creatures = vec![1, 1];
                    } else {
                        graveyard = vec![1, 1];
                    }
                }
                life = vec![if !special && pay { 20 - n } else { 20 }; 2];
            }
            let expected = json!({"resolution_error":null,"life":life,"battlefield_creatures":creatures,"graveyard_creatures":graveyard,"tokens":tokens,"remaining_stack":0,"mana_paid":if name=="Mercy Killing"{3}else{n+1},"announced_targets":if name=="Mercy Killing"{1}else{0},"target_decision_offered":name=="Mercy Killing"});
            let (status, actual, trace, evidence) = match run(&def, n, other, pay, special) {
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
            rows.push(json!({"card":name,"scenario":{"x_or_power":n,"intended_target_controller":other,"pay_life_policy":pay,"empty_board":special},"status":status,"expected":expected,"actual":actual,"execution_trace":trace,"state_evidence":evidence,"artifact_checksum":artifact.payload_checksum,"scope":"Actual normal cast with exact mana and explicit X. Mercy fixtures contain an intended legal target of the recorded power/controller and configure that target policy, but the engine never offers its target decision and announces with no targets before failing. Pay-life policies are recorded even if the engine errors before offering that decision. No replacement or sacrifice-prohibition effects are present."}));
        }
    }
    let after: Vec<_> = paths
        .iter()
        .map(|p| json!({"path":p,"sha256":hash(p)}))
        .collect();
    let report = json!({"rows":rows,"compilation":compilation,"provenance":{"before":before,"after":after,"artifacts_unchanged":before==after,"strict_artifact":true,"unique_card_ids":true,"seed":71757432704851_u64}});
    std::fs::write(
        root.join("reports/runtime-audit/sacrifice-life-value-reproductions.json"),
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
    println!("wrote {} cases", report["rows"].as_array().unwrap().len());
}
