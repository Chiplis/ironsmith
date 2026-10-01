//! Opt-in canonical mechanic sweep. Generating the report is not a card pass.
//! Checks the ninjutsu return cost through legal activation, before resolving
//! the Ninja ability or any separate triggered abilities.

use ironsmith::ability::ActivatedAbility;
use ironsmith::card::PowerToughness;
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::combat_state::{AttackTarget, AttackerInfo, CombatState};
use ironsmith::costs::CostEffect;
use ironsmith::decision::{
    GameProgress, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::DecisionContext;
use ironsmith::effects::{NinjutsuCostEffect, NinjutsuEffect};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_priority_response_with_dm};
use ironsmith::mana::ManaSymbol;
use ironsmith::mana_payment::ManaPaymentResponse;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardDefinition, CardId, CardType, GameState, Phase, PlayerId, Step, Zone};
use rusqlite::{Connection, OpenFlags};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::time::Instant;

const SEED: u64 = 0x4952_4f4e_534d_4954;

fn is_ninjutsu(ability: &ActivatedAbility) -> bool {
    ability
        .effects
        .all_effects()
        .into_iter()
        .any(|effect| effect.downcast_ref::<NinjutsuEffect>().is_some())
        && ability.mana_cost.non_mana_costs().any(|cost| {
            cost.downcast_ref::<CostEffect>()
                .is_some_and(|cost| cost.effect.downcast_ref::<NinjutsuCostEffect>().is_some())
        })
}

fn announce(game: &mut GameState, action: LegalAction) -> Result<(), String> {
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = SelectFirstDecisionMaker;
    let mut response = PriorityResponse::PriorityAction(action);
    for _ in 0..24 {
        let progress =
            apply_priority_response_with_dm(game, &mut queue, &mut state, &response, &mut dm)
                .map_err(|error| error.to_string())?;
        if !game.stack.is_empty()
            && state.pending_activation.is_none()
            && state.pending_cast.is_none()
        {
            return Ok(());
        }
        response = match progress {
            GameProgress::NeedsDecisionCtx(DecisionContext::SelectOptions(ctx)) => {
                PriorityResponse::NextCostChoice(
                    ctx.options
                        .iter()
                        .find(|option| option.legal)
                        .ok_or("fixture has no legal next cost")?
                        .index,
                )
            }
            GameProgress::NeedsDecisionCtx(DecisionContext::SelectObjects(ctx)) => {
                PriorityResponse::CardCostChoice(
                    ctx.candidates
                        .iter()
                        .find(|candidate| candidate.legal)
                        .ok_or("fixture has no legal cost object")?
                        .id,
                )
            }
            GameProgress::NeedsDecisionCtx(DecisionContext::ManaPayment(ctx))
                if ctx.plan.payable =>
            {
                PriorityResponse::ManaPaymentPlan(ManaPaymentResponse::Confirm {
                    plan_id: ctx.plan.id,
                    request_hash: ctx.plan.request_hash,
                })
            }
            other => return Err(format!("unsupported fixture decision: {other:?}")),
        };
    }
    Err("fixture exceeded 24 announcement decisions".into())
}

fn execute(definition: &CardDefinition) -> Value {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Cara".into()], 20);
    game.set_random_seed(SEED);
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(Step::DeclareBlockers);
    let body = CardDefinitionBuilder::new(CardId::new(), "Ninjutsu return-cost fixture")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    if body.card.id == definition.card.id {
        return json!({"status":"fixture_unavailable","detail":"fixture and canonical source share CardId"});
    }
    let attacker = game.create_object_from_definition(&body, alice, Zone::Battlefield);
    let attacker_stable = game.object(attacker).unwrap().stable_id;
    game.remove_summoning_sickness(attacker);
    game.tap(attacker);
    game.combat = Some(CombatState {
        attackers: vec![AttackerInfo {
            creature: attacker,
            target: AttackTarget::Player(bob),
        }],
        ..Default::default()
    });
    for symbol in [
        ManaSymbol::White,
        ManaSymbol::Blue,
        ManaSymbol::Black,
        ManaSymbol::Red,
        ManaSymbol::Green,
        ManaSymbol::Colorless,
    ] {
        game.player_mut(alice).unwrap().mana_pool.add(symbol, 12);
    }
    let own_ninjutsu = definition.abilities.iter().any(|ability| {
        matches!(
        &ability.kind, ironsmith::ability::AbilityKind::Activated(ability) if is_ninjutsu(ability))
    });
    let (source, fixture_kind) = if own_ninjutsu {
        (
            game.create_object_from_definition(definition, alice, Zone::Hand),
            "printed_ninjutsu_from_hand",
        )
    } else {
        // Catalog mentions also include a battlefield ability granting ninjutsu
        // to creature cards in hand. Exercise that actual grant on a plain card.
        game.create_object_from_definition(definition, alice, Zone::Battlefield);
        (
            game.create_object_from_definition(&body, alice, Zone::Hand),
            "battlefield_source_grants_ninjutsu_to_hand",
        )
    };
    game.refresh_continuous_state();
    let action = compute_legal_actions(&game, alice).expect("fixture has complete replacement state")
        .into_iter()
        .find(|action| match action {
            LegalAction::ActivateAbility {
                source: action_source,
                ability_index,
            } if *action_source == source => game
                .current_activated_ability(source, *ability_index)
                .is_some_and(|ability| is_ninjutsu(&ability)),
            _ => false,
        });
    let Some(action) = action else {
        return json!({"status":"fixture_unavailable","fixture_kind":fixture_kind,
            "detail":"No legal activation with both compiled NinjutsuEffect and NinjutsuCostEffect in this seeded hand/combat fixture; not classified as a semantic failure."});
    };
    let action_description = format!("{action:?}");
    if let Err(error) = announce(&mut game, action) {
        return json!({"status":"announcement_or_fixture_error","fixture_kind":fixture_kind,
                      "detail":error,"legal_action":action_description});
    }
    let current = game.find_object_by_stable_id(attacker_stable);
    let actual = json!({
        "attacker_zone":current.and_then(|id|game.object(id)).map(|object|format!("{:?}",object.zone)),
        "combat_retains_departed_attacker":game.combat.as_ref().unwrap().attackers.iter().any(|info|info.creature==attacker),
        "combat_attacker_count":game.combat.as_ref().unwrap().attackers.len(),
        "activation_on_stack":!game.stack.is_empty(),
    });
    let expected = json!({"attacker_zone":"Hand","combat_retains_departed_attacker":false,
                          "combat_attacker_count":0,"activation_on_stack":true});
    json!({"status":if actual==expected {"expected_result_observed"} else {"semantic_mismatch"},
        "outcome_category":if actual==expected {"expected_result_observed"} else {"silent_wrong_result"},
        "fixture_kind":fixture_kind,"legal_action":action_description,"expected":expected,"actual":actual,
        "scope":"Canonical printed or granted ninjutsu selected by typed compiled effect+cost; real legal activation and paid costs. Ability resolution, ETB choices, triggered abilities and commander-zone activations are not exercised."})
}

#[test]
#[ignore = "manual family audit records expected versus actual; report generation is not card correctness"]
fn report_all_catalog_ninjutsu_return_costs() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let db_path = root.join("reports/engine-status.sqlite3");
    let db = Connection::open_with_flags(&db_path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let names = db.prepare("select card_name from registry_card where lower(oracle_text) like '%ninjutsu%' order by card_name")
        .unwrap().query_map([],|row|row.get::<_,String>(0)).unwrap()
        .collect::<rusqlite::Result<Vec<_>>>().unwrap();
    let payloads = ironsmith_tools::load_card_payloads_by_names(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        &names,
    )
    .unwrap();
    let out = std::env::var_os("IR_NINJUTSU_REPORT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| root.join("reports/runtime-audit/ninjutsu-family.json"));
    std::fs::create_dir_all(out.parent().unwrap()).unwrap();
    let binary = std::env::current_exe().unwrap();
    let binary_sha256 = Sha256::digest(std::fs::read(&binary).unwrap())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let git_head = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(&root)
        .output()
        .ok()
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string());
    let mut rows = Vec::new();
    for name in &names {
        let started = Instant::now();
        let result = catch_unwind(AssertUnwindSafe(|| {
            let Some(payload) = payloads
                .get(name)
                .and_then(|payloads| payloads.iter().find(|payload| payload.name == *name))
            else {
                return json!({"status":"payload_unavailable"});
            };
            let builder = ironsmith_compiler::CardDefinitionBuilder::new(
                CardId::new(),
                payload.parse_name.as_deref().unwrap_or(&payload.name),
            );
            let (artifact, definition) = match ironsmith_registry::compile_builder_to_artifact(
                builder,
                &payload.parse_input,
                false,
            ) {
                Ok(value) => value,
                Err(error) => {
                    return json!({"status":if matches!(error,ironsmith_tools::CompilerIntegrationError::Parse(_)) {"compile_failed"} else {"materialization_failed"},"detail":error.to_string()});
                }
            };
            let mut row = execute(&definition);
            row["artifact_checksum"] = json!(artifact.payload_checksum);
            row
        }));
        let mut row = match result {
            Ok(row) => row,
            Err(error) => {
                json!({"status":"panicked","detail":error.downcast_ref::<String>().cloned()
                .or_else(||error.downcast_ref::<&str>().map(|s|s.to_string())).unwrap_or_else(||"nonstring panic".into())})
            }
        };
        row["card"] = json!(name);
        row["seed"] = json!(SEED);
        row["elapsed_seconds"] = json!(started.elapsed().as_secs_f64());
        println!("{row}");
        rows.push(row);
        let report = json!({"scope":"Every registry card whose oracle text mentions ninjutsu; current canonical payload fresh-compilation and unique CardIds. Fixture/setup limitations are not card failures.",
            "provenance":{"test_binary":binary,"test_binary_sha256":binary_sha256,"git_head_at_execution":git_head,"compiled_via":"ironsmith_registry::compile_builder_to_artifact"},
            "inventory_source":db_path,"candidate_count":names.len(),"candidates":names,
            "completed":rows.len(),"status":if rows.len()==names.len() {"complete"} else {"partial"},
            "limitations":"Bounded 24 announcement decisions. No whole-card semantics proof, no ninjutsu resolution or commander-zone activation. The shared combat cleanup path may also affect other battlefield departures during combat; that broader family is only a hypothesis here.",
            "rows":rows});
        std::fs::write(&out, serde_json::to_string_pretty(&report).unwrap()).unwrap();
    }
    println!("Ninjutsu family report: {}", out.display());
}
