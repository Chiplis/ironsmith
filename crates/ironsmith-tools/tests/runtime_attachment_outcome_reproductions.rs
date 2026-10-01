//! Strict canonical Thorin casts with real Equipment and target/attachment state.
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{
    DecisionMaker, GameProgress, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::{SelectOptionsContext, TargetsContext};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, drain_pending_trigger_events, put_triggers_on_stack_with_dm,
    resolve_stack_entry_with,
};
use ironsmith::game_state::Target;
use ironsmith::mana::ManaSymbol;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{
    CardDefinition, CardId, CardType, GameState, ObjectId, PlayerId, PowerToughness, Subtype, Zone,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
fn sha(path: &std::path::Path) -> String {
    Sha256::digest(std::fs::read(path).unwrap())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
struct Choices {
    equipment: Vec<ObjectId>,
    creature: ObjectId,
    victim: ObjectId,
    damage: bool,
    trace: Vec<Value>,
}
impl DecisionMaker for Choices {
    fn answers_player_choices(&self) -> bool {
        false
    }
    fn decide_options(&mut self, g: &GameState, c: &SelectOptionsContext) -> Vec<usize> {
        let result = SelectFirstDecisionMaker.decide_options(g, c);
        self.trace
            .push(json!({"kind":"options","context":format!("{c:?}"),"chosen":result}));
        result
    }
    fn decide_targets(&mut self, _: &GameState, c: &TargetsContext) -> Vec<Target> {
        let mut result = Vec::new();
        for r in &c.requirements {
            let equipment: Vec<_> = self
                .equipment
                .iter()
                .map(|id| Target::Object(*id))
                .filter(|t| r.legal_targets.contains(t))
                .collect();
            if !equipment.is_empty() {
                result.extend(equipment);
            } else if r.legal_targets.contains(&Target::Object(self.victim)) {
                if self.damage {
                    result.push(Target::Object(self.victim));
                }
            } else if r.legal_targets.contains(&Target::Object(self.creature)) {
                result.push(Target::Object(self.creature));
            }
        }
        self.trace.push(
            json!({"kind":"targets","context":format!("{c:?}"),"chosen":format!("{result:?}")}),
        );
        result
    }
}
fn scenario(
    def: &CardDefinition,
    equipment_count: usize,
    damage: bool,
    all_targets_illegal: bool,
) -> Result<Value, String> {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.set_random_seed(71757432704848);
    game.turn.turn_number = 3;
    game.turn.active_player = PlayerId(0);
    game.turn.priority_player = Some(PlayerId(0));
    game.turn.phase = ironsmith::Phase::FirstMain;
    game.turn.step = None;
    game.player_mut(PlayerId(0))
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Red, 4);
    let creature = CardDefinitionBuilder::new(CardId::new(), "Thorin attachment recipient")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(5, 10))
        .build();
    let equipment = CardDefinitionBuilder::new(CardId::new(), "Thorin neutral Equipment")
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Equipment])
        .build();
    let recipient = game.create_object_from_definition(&creature, PlayerId(0), Zone::Battlefield);
    let victim = game.create_object_from_definition(&creature, PlayerId(1), Zone::Battlefield);
    let equips: Vec<_> = (0..equipment_count)
        .map(|_| game.create_object_from_definition(&equipment, PlayerId(0), Zone::Battlefield))
        .collect();
    game.effect_store.pending_trigger_events.clear();
    let source = game.create_object_from_definition(def, PlayerId(0), Zone::Hand);
    let action = compute_legal_actions(&game, PlayerId(0)).expect("fixture has complete replacement state")
        .into_iter()
        .find(|a| matches!(a,LegalAction::CastSpell{spell_id,..} if *spell_id==source))
        .ok_or("legal normal cast unavailable")?;
    let mut dm = Choices {
        equipment: equips.clone(),
        creature: recipient,
        victim,
        damage,
        trace: vec![json!({"kind":"legal_action","action":format!("{action:?}")})],
    };
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut progress = apply_priority_response_with_dm(
        &mut game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        &mut dm,
    )
    .map_err(|e| e.to_string())?;
    for _ in 0..24 {
        if state.pending_cast.is_none() && !game.stack.is_empty() {
            break;
        }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else {
            return Err(format!("cast announcement stalled: {progress:?}"));
        };
        progress = apply_decision_context_with_dm(&mut game, &mut queue, &mut state, &ctx, &mut dm)
            .map_err(|e| e.to_string())?;
    }
    if game.stack.len() != 1 || game.player(PlayerId(0)).unwrap().mana_pool.total() != 0 {
        return Err("normal four-mana cast did not complete".into());
    }
    resolve_stack_entry_with(&mut game, &mut dm).map_err(|e| e.to_string())?;
    if !game
        .battlefield
        .iter()
        .any(|id| game.object(*id).is_some_and(|o| o.name == def.card.name))
    {
        return Err("Thorin did not enter".into());
    }
    drain_pending_trigger_events(&mut game, &mut queue);
    put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).map_err(|e| e.to_string())?;
    dm.trace.push(json!({"kind":"etb_stack","count":game.stack.len(),"targets":game.stack.iter().map(|e|format!("{:?}",e.targets)).collect::<Vec<_>>() }));
    if game.stack.len() != 1 {
        return Err("expected one ETB trigger".into());
    }
    if all_targets_illegal {
        if equipment_count != 0 || damage {
            return Err("illegal-target control expects only recipient target".into());
        }
        game.move_object_by_effect(recipient, Zone::Hand)
            .ok_or("recipient could not leave")?;
    }
    let mut error = None;
    for _ in 0..24 {
        if game.stack.is_empty() {
            break;
        }
        if let Err(e) = resolve_stack_entry_with(&mut game, &mut dm) {
            error = Some(e.to_string());
            break;
        }
        drain_pending_trigger_events(&mut game, &mut queue);
        put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).map_err(|e| e.to_string())?;
    }
    let actual = json!({"resolution_error":error,"equipment_attached":equips.iter().filter(|id|game.object(**id).is_some_and(|o|o.attached_to==Some(ironsmith::object::AttachmentTarget::Object(recipient)))).count(),"victim_damage":game.damage_on(victim),"remaining_stack":game.stack.len(),"mana_paid":4});
    Ok(json!({"actual":actual,"trace":dm.trace}))
}
#[test]
#[ignore = "manual expected-result audit; a passing exporter is not a card pass"]
fn report_attachment_outcome() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let inventory =
        root.join("reports/runtime-audit/corpus/267a16aff3b321196397d0b4/inventory.json");
    let source =
        root.join("crates/ironsmith-tools/tests/runtime_attachment_outcome_reproductions.rs");
    let binary = std::env::current_exe().unwrap();
    let paths = [&inventory, &source, &binary];
    let before: Vec<_> = paths
        .iter()
        .map(|p| json!({"path":p,"sha256":sha(p)}))
        .collect();
    let data: Value = serde_json::from_slice(&std::fs::read(&inventory).unwrap()).unwrap();
    let p = data["cards"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["name"] == "Thorin, Mountain-king")
        .unwrap();
    let (artifact, def) = ironsmith_registry::compile_builder_to_artifact(
        ironsmith_compiler::CardDefinitionBuilder::new(CardId::new(), "Thorin, Mountain-king"),
        p["parse_input"].as_str().unwrap(),
        false,
    )
    .unwrap();
    let mut rows = Vec::new();
    for (n, damage, illegal) in [
        (0, false, false),
        (0, true, false),
        (1, false, false),
        (1, true, false),
        (2, true, false),
        (0, false, true),
    ] {
        let expected = json!({"resolution_error":null,"equipment_attached":n,"victim_damage":if n>0&&damage{5}else{0},"remaining_stack":0,"mana_paid":4});
        let (status, actual, trace) = match scenario(&def, n, damage, illegal) {
            Ok(value) => {
                let a = value["actual"].clone();
                let status = if a["resolution_error"].is_string() {
                    "resolution_failed"
                } else if a == expected {
                    "expected_result_observed"
                } else {
                    "semantic_mismatch"
                };
                (status, a, value["trace"].clone())
            }
            Err(e) => ("fixture_error", json!({"error":e}), Value::Null),
        };
        rows.push(json!({"card":"Thorin, Mountain-king","scenario":{"equipment_count":n,"choose_damage_target":damage,"recipient_leaves_before_etb_resolution":illegal},"status":status,"expected":expected,"actual":actual,"execution_trace":trace,"artifact_checksum":artifact.payload_checksum,"scope":"Actual normal four-mana cast; source enters, genuine ETB trigger targets actual neutral Equipment and own power-5 creature. Opposing 5/10 survives expected damage. Explicit zero-Equipment, no-damage-target and illegal-target branches. No attachment effect is manually executed."}));
    }
    let after: Vec<_> = paths
        .iter()
        .map(|p| json!({"path":p,"sha256":sha(p)}))
        .collect();
    let report = json!({"rows":rows,"definition":artifact.payload.definition,"provenance":{"before":before,"after":after,"artifacts_unchanged":before==after,"strict_artifact":true,"seed":71757432704848_u64,"unique_card_ids":true},"limitations":["Fixtures supply legal ordinary Equipment without stat bonuses and neutral creatures.","Recorded target choice timing may reflect an additional lowering error; this report focuses on the thrown missing-outcome error and resulting state."]});
    std::fs::write(
        root.join("reports/runtime-audit/attachment-outcome-reproductions.json"),
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
    println!("{}", serde_json::to_string(&report["rows"]).unwrap());
}
