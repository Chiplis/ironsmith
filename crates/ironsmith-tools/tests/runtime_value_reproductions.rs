//! Opt-in expected-result probes for missing runtime value context. Passing
//! this reporter means observations were saved; inspect every row's status.
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{
    DecisionMaker, GameProgress, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::{NumberContext, TargetsContext};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, resolve_stack_entry_with,
};
use ironsmith::game_state::Target;
use ironsmith::mana::ManaSymbol;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardDefinition, CardId, CardType, GameState, PlayerId, Zone};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

struct Decisions {
    x: u32,
    target_contexts: Vec<Value>,
}
impl DecisionMaker for Decisions {
    fn decide_number(&mut self, game: &GameState, ctx: &NumberContext) -> u32 {
        if ctx.is_x_value {
            self.x.clamp(ctx.min, ctx.max)
        } else {
            SelectFirstDecisionMaker.decide_number(game, ctx)
        }
    }
    fn decide_targets(&mut self, _: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        self.target_contexts.push(json!(ctx.requirements.iter().map(|r| json!({
            "minimum":r.min_targets,"maximum":r.max_targets,"legal_targets":format!("{:?}",r.legal_targets)
        })).collect::<Vec<_>>()));
        let mut targets = Vec::new();
        for requirement in &ctx.requirements {
            if requirement
                .legal_targets
                .contains(&Target::Player(PlayerId(1)))
            {
                targets.push(Target::Player(PlayerId(1)));
            } else {
                targets.extend(
                    requirement
                        .legal_targets
                        .iter()
                        .filter(|target| matches!(target, Target::Object(_)))
                        .take(
                            (self.x as usize)
                                .max(requirement.min_targets)
                                .min(requirement.max_targets.unwrap_or(usize::MAX)),
                        )
                        .copied(),
                );
            }
        }
        targets
    }
}

fn suffer(definition: &CardDefinition, x: u32) -> Result<(Value, Value), String> {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.set_random_seed(71757432704846);
    game.turn.active_player = PlayerId(0);
    game.turn.priority_player = Some(PlayerId(0));
    game.turn.phase = ironsmith::Phase::FirstMain;
    let filler = CardDefinitionBuilder::new(CardId::new(), "Value audit graveyard card")
        .card_types(vec![CardType::Land])
        .build();
    assert_ne!(definition.card.id, filler.card.id);
    for _ in 0..3 {
        game.create_object_from_definition(&filler, PlayerId(1), Zone::Graveyard);
    }
    let source = game.create_object_from_definition(definition, PlayerId(0), Zone::Hand);
    game.player_mut(PlayerId(0))
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Black, x + 1);
    let action = compute_legal_actions(&game, PlayerId(0)).expect("fixture has complete replacement state")
        .into_iter()
        .find(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == source))
        .ok_or("fixture has no legal cast")?;
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = Decisions {
        x,
        target_contexts: Vec::new(),
    };
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
            return Err(format!("fixture announcement stalled: {progress:?}"));
        };
        progress = apply_decision_context_with_dm(&mut game, &mut queue, &mut state, &ctx, &mut dm)
            .map_err(|e| e.to_string())?;
    }
    let entry = game
        .stack
        .last()
        .ok_or("fixture cast did not reach stack")?;
    if entry.x_value != Some(x) {
        return Err(format!(
            "fixture X/targets inconsistent: x={:?}, targets={:?}",
            entry.x_value, entry.targets
        ));
    }
    let announced_objects = entry
        .targets
        .iter()
        .filter(|target| matches!(target, Target::Object(_)))
        .count();
    let evidence = json!({"target_decision_contexts":dm.target_contexts,"announced_targets":format!("{:?}",entry.targets),
        "paid_mana_remaining":game.player(PlayerId(0)).unwrap().mana_pool.total(),"actual_x":entry.x_value});
    let resolution = resolve_stack_entry_with(&mut game, &mut dm);
    Ok((
        json!({"resolution_error": resolution.err().map(|e| e.to_string()),
        "exiled": game.exile.len(), "alice_life":game.player(PlayerId(0)).unwrap().life,
        "bob_life":game.player(PlayerId(1)).unwrap().life,"announced_object_targets":announced_objects}),
        evidence,
    ))
}

fn hash(path: &std::path::Path) -> String {
    Sha256::digest(std::fs::read(path).unwrap())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[test]
#[ignore = "audit reporter; passing means report generated, not card correctness"]
fn report_runtime_value_context_reproductions() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let name = "Suffer the Past";
    let payload = ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        name,
    )
    .unwrap()
    .remove(0);
    let definition = ironsmith_tools::compile_runtime_definition_from_payload(&payload).unwrap();
    let mut rows = Vec::new();
    for x in [0, 1, 2] {
        let expected = json!({"resolution_error": null, "exiled": x, "alice_life":20 + x, "bob_life":20 - x,"announced_object_targets":x});
        let (status, actual, evidence) = match suffer(&definition, x) {
            Ok((actual, evidence)) if actual == expected => {
                ("expected_result_observed", actual, evidence)
            }
            Ok((actual, evidence)) => ("semantic_mismatch", actual, evidence),
            Err(error) => (
                "fixture_or_announcement_error",
                json!({"error":error}),
                json!(null),
            ),
        };
        rows.push(json!({"card":name,"scenario":{"x":x,"bob_graveyard":3},"status":status,
            "expected":expected,"actual":actual,"fixture_evidence":evidence,"scope":"engine-offered legal cast, exact X paid, target choice contexts recorded, actual exile and life totals"}));
    }
    let executable = std::env::current_exe().unwrap();
    let report = json!({"rows":rows,"provenance":{"binary":executable,"binary_sha256":hash(&executable),"cards_sha256":hash(&root.join("cards.json"))},
        "scope":"three expected-result value-context cases; not corpus completeness"});
    let output = root.join("reports/runtime-audit/runtime-value-reproductions.json");
    std::fs::write(&output, serde_json::to_string_pretty(&report).unwrap()).unwrap();
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
}
