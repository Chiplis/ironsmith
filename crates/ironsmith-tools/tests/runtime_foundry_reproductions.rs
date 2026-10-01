//! Opt-in report for a canonical total-mana-value activation and its controls.
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{DecisionMaker, GameProgress, LegalAction, compute_legal_actions};
use ironsmith::decisions::context::{NumberContext, SelectObjectsContext, TargetsContext};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, resolve_stack_entry_with,
};
use ironsmith::game_state::Target;
use ironsmith::mana::{ManaCost, ManaSymbol};
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardDefinition, CardId, CardType, GameState, ObjectId, PlayerId, Zone};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
const SEED: u64 = 0x464F_554E_4452;
struct Decisions {
    x: u32,
    traces: Vec<Value>,
}
impl DecisionMaker for Decisions {
    fn decide_number(&mut self, _: &GameState, ctx: &NumberContext) -> u32 {
        let chosen = self.x.clamp(ctx.min, ctx.max);
        self.traces.push(
            json!({"kind":"number","x":ctx.is_x_value,"min":ctx.min,"max":ctx.max,"chosen":chosen}),
        );
        chosen
    }
    fn decide_objects(&mut self, game: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        let result: Vec<_> = ctx
            .candidates
            .iter()
            .filter(|o| o.legal)
            .take(ctx.max.unwrap_or(usize::MAX))
            .map(|o| o.id)
            .collect();
        self.traces.push(json!({"kind":"objects","min":ctx.min,"max":ctx.max,"aggregate":format!("{:?}",ctx.aggregate_constraint),
            "candidates":ctx.candidates.iter().map(|o|json!({"id":format!("{:?}",o.id),"name":game.object(o.id).map(|c|c.name.to_string()),"legal":o.legal})).collect::<Vec<_>>(),
            "chosen":format!("{result:?}")}));
        result
    }
    fn decide_targets(&mut self, _: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        let mut targets = Vec::new();
        for requirement in &ctx.requirements {
            targets.extend(
                requirement
                    .legal_targets
                    .iter()
                    .take(
                        requirement
                            .min_targets
                            .max(1)
                            .min(requirement.max_targets.unwrap_or(usize::MAX)),
                    )
                    .copied(),
            );
        }
        self.traces.push(json!({"kind":"targets","requirements":ctx.requirements.iter().map(|r|json!({"min":r.min_targets,"max":r.max_targets,"legal":format!("{:?}",r.legal_targets)})).collect::<Vec<_>>(),"chosen":format!("{targets:?}")}));
        targets
    }
}
fn artifact(name: &str, mv: u32) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Artifact])
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(
            mv as u8,
        )]]))
        .build()
}
fn scenario(definition: &CardDefinition, values: &[u32], target_value: u32) -> (Value, Value) {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.set_random_seed(SEED);
    game.turn.turn_number = 3;
    game.turn.active_player = PlayerId(0);
    game.turn.priority_player = Some(PlayerId(0));
    game.turn.phase = ironsmith::Phase::FirstMain;
    game.turn.step = None;
    let source = game.create_object_from_definition(definition, PlayerId(0), Zone::Battlefield);
    game.remove_summoning_sickness(source);
    for (i, mv) in values.iter().copied().enumerate() {
        let card = artifact(&format!("Foundry audit cost {i}"), mv);
        assert_ne!(definition.card.id, card.card.id);
        game.create_object_from_definition(&card, PlayerId(0), Zone::Battlefield);
    }
    let target_card = artifact("Foundry audit graveyard target", target_value);
    assert_ne!(definition.card.id, target_card.card.id);
    game.create_object_from_definition(&target_card, PlayerId(0), Zone::Graveyard);
    game.player_mut(PlayerId(0))
        .unwrap()
        .mana_pool
        .add(ManaSymbol::White, 3);
    game.effect_store.pending_trigger_events.clear();
    let actions = compute_legal_actions(&game, PlayerId(0)).expect("fixture has complete replacement state");
    let available: Vec<_> = actions
        .iter()
        .filter(|a| matches!(a,LegalAction::ActivateAbility{source:id,..} if *id==source))
        .cloned()
        .collect();
    let mut dm = Decisions {
        x: values.iter().sum(),
        traces: Vec::new(),
    };
    let mut announcement_error = None;
    let mut resolution_error = None;
    let mut stack_reached = false;
    if let Some(action) = available.first().cloned() {
        let mut queue = TriggerQueue::new();
        let mut state = PriorityLoopState::new(game.players_in_game());
        let progress = apply_priority_response_with_dm(
            &mut game,
            &mut queue,
            &mut state,
            &PriorityResponse::PriorityAction(action),
            &mut dm,
        );
        match progress {
            Err(error) => announcement_error = Some(error.to_string()),
            Ok(mut progress) => {
                for _ in 0..24 {
                    if state.pending_activation.is_none() && !game.stack.is_empty() {
                        stack_reached = true;
                        break;
                    }
                    let GameProgress::NeedsDecisionCtx(ctx) = progress else {
                        announcement_error =
                            Some(format!("announcement did not complete: {progress:?}"));
                        break;
                    };
                    match apply_decision_context_with_dm(
                        &mut game, &mut queue, &mut state, &ctx, &mut dm,
                    ) {
                        Ok(next) => progress = next,
                        Err(error) => {
                            announcement_error = Some(error.to_string());
                            break;
                        }
                    }
                }
                if !stack_reached && announcement_error.is_none() {
                    announcement_error = Some("bounded announcement did not reach stack".into());
                }
                if stack_reached {
                    resolution_error = resolve_stack_entry_with(&mut game, &mut dm)
                        .err()
                        .map(|e| e.to_string());
                }
            }
        }
    }
    let returned = game
        .battlefield
        .iter()
        .filter_map(|id| game.object(*id))
        .filter(|o| o.name == target_card.name())
        .count();
    let exiled = game
        .exile
        .iter()
        .filter_map(|id| game.object(*id))
        .filter(|o| o.name.starts_with("Foundry audit cost "))
        .count();
    let actual = json!({"activation_offered":!available.is_empty(),"reached_stack":stack_reached,"returned_target":returned==1,"exiled_cost_cards":exiled,
        "announcement_error":announcement_error,"resolution_error":resolution_error});
    let evidence = json!({"source_actions":available.iter().map(|a|format!("{a:?}")).collect::<Vec<_>>(),"decision_trace":dm.traces,
        "mana_remaining":game.player(PlayerId(0)).unwrap().mana_pool.total(),"source_tapped":game.is_tapped(source),"source_untapped_initially":true,"source_summoning_sickness_removed":true,"empty_stack_initially":true,
        "fixture_controlled_other_artifact_mana_values":values,"fixture_graveyard_artifact_mana_value":target_value});
    (actual, evidence)
}
fn hash(path: &std::path::Path) -> String {
    Sha256::digest(std::fs::read(path).unwrap())
        .iter()
        .map(|v| format!("{v:02x}"))
        .collect()
}
#[test]
#[ignore = "audit reporter; inspect every expected/actual status"]
fn report_runtime_foundry_reproductions() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let binary = std::env::current_exe().unwrap();
    let source = root.join("crates/ironsmith-tools/tests/runtime_foundry_reproductions.rs");
    let cards = root.join("cards.json");
    let before = json!({"binary_sha256":hash(&binary),"source_sha256":hash(&source),"cards_sha256":hash(&cards)});
    let name = "Fabrication Foundry";
    let payload = ironsmith_tools::load_card_payloads_by_name(cards.to_str().unwrap(), name)
        .unwrap()
        .remove(0);
    let definition = ironsmith_tools::compile_runtime_definition_from_payload(&payload).unwrap();
    let mut rows = Vec::new();
    for (values, target) in [(vec![2, 3], 5), (vec![2], 2), (vec![2], 3), (vec![2, 3], 6)] {
        let legal = target <= values.iter().sum::<u32>();
        let expected = json!({"activation_offered":legal,"returned_target":legal,"maximum_cost_mana_value":values.iter().sum::<u32>(),"resolution_error":null});
        let (actual, evidence) = scenario(&definition, &values, target);
        let correct = actual["activation_offered"] == legal
            && actual["returned_target"] == legal
            && actual["resolution_error"].is_null()
            && actual["announcement_error"].is_null();
        rows.push(json!({"card":name,"scenario":{"other_artifact_mana_values":values,"graveyard_target_mana_value":target},"expected":expected,"actual":actual,"fixture_evidence":evidence,
            "status":if correct {"expected_result_observed"}else{"semantic_mismatch"},"seed":SEED,"scope":"Canonical legal-action discovery in a state with three mana, untapped source, summoning-sickness bookkeeping removed, own main phase and empty stack. Activation and return are attempted only if offered; decision_trace and reached_stack record which stages actually occurred."}));
    }
    let after = json!({"binary_sha256":hash(&binary),"source_sha256":hash(&source),"cards_sha256":hash(&cards)});
    let report = json!({"rows":rows,"provenance":{"binary":binary,"artifacts_before":before,"artifacts_after":after,"artifacts_unchanged":before==after},"scope":"Total-value cost and target legality, two positive and two insufficient-value control states; not whole-card correctness."});
    std::fs::write(
        root.join("reports/runtime-audit/foundry-value-reproductions.json"),
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
}
