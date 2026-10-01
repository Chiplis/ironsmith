//! Actual activation announcements before/after authored temporal boundaries.
use ironsmith::ability::AbilityKind;
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::combat_state::{AttackTarget, CombatState};
use ironsmith::decision::{
    AttackerDeclaration, GameProgress, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_attacker_declarations_with_dm,
    apply_blocker_declarations, apply_decision_context_with_dm, apply_priority_response_with_dm,
    execute_combat_damage_step,
};
use ironsmith::game_state::{Phase, Step};
use ironsmith::mana::{ManaCost, ManaSymbol};
use ironsmith::triggers::TriggerQueue;
use ironsmith::{
    CardDefinition, CardId, CardType, CounterType, GameState, ObjectId, PlayerId, PowerToughness,
    Subtype, Supertype, Zone,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
fn hash(p: &std::path::Path) -> String {
    Sha256::digest(std::fs::read(p).unwrap())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn resource(name: &str, kind: CardType, legendary: bool) -> CardDefinition {
    let mut b = CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![kind])
        .mana_cost(ManaCost::from_symbols(vec![
            if legendary {
                ManaSymbol::Black
            } else {
                ManaSymbol::White
            },
            ManaSymbol::Green,
        ]));
    if kind == CardType::Creature {
        b = b.power_toughness(PowerToughness::fixed(2, 20));
    }
    if kind == CardType::Land {
        b = b.subtypes(vec![Subtype::Island]);
    }
    if legendary {
        b = b.supertypes(vec![Supertype::Legendary]);
    }
    b.build()
}
fn move_to_time(g: &mut GameState, time: &str, attacker: ObjectId) -> Result<Value, String> {
    let mut trace = Vec::new();
    if time == "own_upkeep" || time == "opponent_upkeep" {
        g.turn.phase = Phase::Beginning;
        g.turn.step = Some(Step::Upkeep);
        if time == "opponent_upkeep" {
            g.turn.active_player = PlayerId(1);
        }
        return Ok(json!({"phase_fixture":"already in upkeep priority window"}));
    }
    if time == "opponent_first_main" {
        g.turn.active_player = PlayerId(1);
        return Ok(json!({"phase_fixture":"opponent first main"}));
    }
    if time == "first_main" {
        return Ok(json!({"phase_fixture":"first main before any combat"}));
    }
    ironsmith::turn::advance_phase(g).map_err(|e| e.to_string())?;
    assert_eq!(g.turn.step, Some(Step::BeginCombat));
    g.combat = Some(CombatState::default());
    trace.push(json!({"phase":"begin_combat","combat_phases":g.turn_store.combat_phases_started_this_turn}));
    if time == "begin_combat" {
        return Ok(json!(trace));
    }
    ironsmith::turn::advance_step(g).map_err(|e| e.to_string())?;
    assert_eq!(g.turn.step, Some(Step::DeclareAttackers));
    let mut combat = g.combat.take().unwrap();
    let mut q = TriggerQueue::new();
    let mut dm = SelectFirstDecisionMaker;
    apply_attacker_declarations_with_dm(
        g,
        &mut combat,
        &mut q,
        &[AttackerDeclaration {
            creature: attacker,
            target: AttackTarget::Player(PlayerId(1)),
        }],
        &mut dm,
    )
    .map_err(|e| e.to_string())?;
    trace.push(json!({"phase":"attackers_declared","attacker":attacker.0,"attacker_count":combat.attackers.len()}));
    g.combat = Some(combat);
    if time == "declare_attackers" {
        return Ok(json!(trace));
    }
    ironsmith::turn::advance_step(g).map_err(|e| e.to_string())?;
    assert_eq!(g.turn.step, Some(Step::DeclareBlockers));
    let mut combat = g.combat.take().unwrap();
    apply_blocker_declarations(g, &mut combat, &mut q, &[], PlayerId(1))
        .map_err(|e| e.to_string())?;
    g.combat = Some(combat);
    trace.push(json!({"phase":"blockers_declared","blocker_count":0}));
    if time == "declare_blockers" {
        return Ok(json!(trace));
    }
    ironsmith::turn::advance_step(g).map_err(|e| e.to_string())?;
    assert_eq!(g.turn.step, Some(Step::CombatDamage));
    let combat = g.combat.take().unwrap();
    let damage = execute_combat_damage_step(g, &combat, false);
    trace.push(json!({"phase":"combat_damage_done","events":format!("{damage:?}")}));
    g.combat = Some(combat);
    if time == "combat_damage" {
        return Ok(json!(trace));
    }
    ironsmith::turn::advance_step(g).map_err(|e| e.to_string())?;
    assert_eq!(g.turn.step, Some(Step::EndCombat));
    trace.push(json!({"phase":"end_combat"}));
    if time == "end_combat" {
        return Ok(json!(trace));
    }
    ironsmith::turn::advance_step(g).map_err(|e| e.to_string())?;
    assert_eq!(g.turn.phase, Phase::NextMain);
    g.combat = None;
    trace.push(
        json!({"phase":"next_main","main_phases":g.turn_store.main_phases_started_this_turn}),
    );
    Ok(json!(trace))
}
fn run(def: &CardDefinition, index: usize, time: &str) -> Result<(Value, Value), String> {
    let mut g = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    g.set_random_seed(71757432704849);
    g.turn.turn_number = 3;
    g.turn.active_player = PlayerId(0);
    g.turn.priority_player = Some(PlayerId(0));
    g.turn.phase = Phase::FirstMain;
    g.turn.step = None;
    g.mark_main_phase_started();
    let source = g.create_object_from_definition(def, PlayerId(0), Zone::Battlefield);
    g.remove_summoning_sickness(source);
    if def.name() == "Armageddon Clock" {
        g.add_counters(source, CounterType::Doom, 1);
    }
    if def.name() == "Infinite Hourglass" {
        g.add_counters(source, CounterType::Time, 1);
    }
    let neutral = resource("Temporal neutral creature", CardType::Creature, false);
    let land = resource("Temporal Island", CardType::Land, false);
    let legendary = resource(
        "Temporal legendary graveyard creature",
        CardType::Creature,
        true,
    );
    let island_attacker = time.starts_with("island_attacker:");
    let step_time = time.strip_prefix("island_attacker:").unwrap_or(time);
    let attacker = if island_attacker {
        let mut island = resource(
            "Temporal Island creature attacker",
            CardType::Creature,
            false,
        );
        island.card.card_types = vec![CardType::Land, CardType::Creature];
        island.card.subtypes = vec![Subtype::Island];
        g.create_object_from_definition(&island, PlayerId(0), Zone::Battlefield)
    } else {
        g.create_object_from_definition(&neutral, PlayerId(0), Zone::Battlefield)
    };
    g.remove_summoning_sickness(attacker);
    for seat in [PlayerId(0), PlayerId(1)] {
        let target = g.create_object_from_definition(&neutral, seat, Zone::Battlefield);
        g.remove_summoning_sickness(target);
        g.tap(target);
        g.create_object_from_definition(&land, seat, Zone::Battlefield);
        g.create_object_from_definition(&legendary, seat, Zone::Graveyard);
        for _ in 0..2 {
            g.create_object_from_definition(&neutral, seat, Zone::Hand);
        }
        for _ in 0..8 {
            g.create_object_from_definition(&neutral, seat, Zone::Library);
        }
    }
    g.effect_store.pending_trigger_events.clear();
    let history = move_to_time(&mut g, step_time, attacker)?;
    g.turn.priority_player = Some(PlayerId(0));
    for symbol in [
        ManaSymbol::White,
        ManaSymbol::Blue,
        ManaSymbol::Black,
        ManaSymbol::Red,
        ManaSymbol::Green,
        ManaSymbol::Colorless,
    ] {
        g.player_mut(PlayerId(0)).unwrap().mana_pool.add(symbol, 12);
    }
    let mut evidence = json!({"island_creature_attacker":island_attacker,"ability_index":index,"phase":format!("{:?}",g.turn.phase),"step":format!("{:?}",g.turn.step),"active_player":g.turn.active_player.index(),"history":history,"turn_history":format!("{:?}",g.turn_store.turn_history),"combat":format!("{:?}",g.combat),"combat_phases_started":g.turn_store.combat_phases_started_this_turn,"main_phases_started":g.turn_store.main_phases_started_this_turn,"source_tapped":g.is_tapped(source),"mana_available":g.player(PlayerId(0)).unwrap().mana_pool.total()});
    let action=compute_legal_actions(&g,PlayerId(0)).expect("fixture has complete replacement state").into_iter().find(|a|matches!(a,LegalAction::ActivateAbility{source:s,ability_index} if *s==source&&*ability_index==index));
    let offered = action.is_some();
    let mut announced = false;
    if let Some(action) = action {
        evidence["action"] = json!(format!("{action:?}"));
        let mut q = TriggerQueue::new();
        let mut state = PriorityLoopState::new(g.players_in_game());
        let mut dm = SelectFirstDecisionMaker;
        let mut progress = apply_priority_response_with_dm(
            &mut g,
            &mut q,
            &mut state,
            &PriorityResponse::PriorityAction(action),
            &mut dm,
        )
        .map_err(|e| format!("announcement:{e}"))?;
        for _ in 0..24 {
            if state.pending_activation.is_none() && !g.stack.is_empty() {
                announced = true;
                break;
            }
            let GameProgress::NeedsDecisionCtx(ctx) = progress else {
                return Err(format!("announcement stalled:{progress:?}"));
            };
            progress = apply_decision_context_with_dm(&mut g, &mut q, &mut state, &ctx, &mut dm)
                .map_err(|e| format!("announcement:{e}"))?;
        }
        if !announced {
            return Err("announcement bound".into());
        }
        evidence["stack_targets"] = json!(format!("{:?}", g.stack.last().unwrap().targets));
        evidence["mana_spent"] = json!(72 - g.player(PlayerId(0)).unwrap().mana_pool.total());
        evidence["source_tapped_after"] = json!(g.is_tapped(source));
    }
    Ok((
        json!({"activation_offered":offered,"activation_announced":announced}),
        evidence,
    ))
}
#[test]
#[ignore = "audit observations, no whole-card correctness assertion"]
fn report_temporal_activation() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let inventory =
        root.join("reports/runtime-audit/corpus/267a16aff3b321196397d0b4/inventory.json");
    let selection = root.join("reports/runtime-audit/activation-restriction-candidates.json");
    let source =
        root.join("crates/ironsmith-tools/tests/runtime_temporal_activation_reproductions.rs");
    let binary = std::env::current_exe().unwrap();
    let paths = [&inventory, &selection, &source, &binary];
    let before: Vec<_> = paths
        .iter()
        .map(|p| json!({"path":p,"sha256":hash(p)}))
        .collect();
    let data: Value = serde_json::from_slice(&std::fs::read(&inventory).unwrap()).unwrap();
    let candidates: Value = serde_json::from_slice(&std::fs::read(&selection).unwrap()).unwrap();
    let mut rows = Vec::new();
    let mut compilation = Vec::new();
    for candidate in candidates["rows"].as_array().unwrap() {
        let texts: Vec<_> = candidate["authored_restrictions"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(Value::as_str)
            .collect();
        let text = texts.join(" ");
        let kind = if text.contains("before attackers") {
            "attackers"
        } else if text.contains("before blockers") {
            "blockers"
        } else if text.contains("before the combat damage") {
            "damage"
        } else if text.contains("before the end of combat") {
            "end_combat"
        } else if text.contains("only during any upkeep") {
            "upkeep"
        } else {
            continue;
        };
        let name = candidate["card"].as_str().unwrap();
        let p = data["cards"]
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
        let index=def.abilities.iter().position(|a|matches!(&a.kind,AbilityKind::Activated(a) if a.additional_restrictions.iter().any(|r|text.contains(r.as_str())))).unwrap();
        compilation.push(json!({"card":name,"artifact_checksum":artifact.payload_checksum,"definition":artifact.payload.definition}));
        let cases: Vec<(&str, bool)> = match kind {
            "attackers" => vec![
                ("own_upkeep", true),
                ("first_main", true),
                ("begin_combat", true),
                ("declare_attackers", false),
                ("next_main", false),
                ("opponent_first_main", false),
            ],
            "blockers" => vec![
                ("first_main", true),
                ("begin_combat", true),
                ("declare_attackers", true),
                ("declare_blockers", false),
                ("combat_damage", false),
                ("next_main", false),
            ],
            "damage" => vec![
                ("first_main", true),
                ("begin_combat", true),
                ("declare_blockers", true),
                ("combat_damage", false),
                ("next_main", false),
            ],
            "end_combat" => vec![
                ("declare_attackers", true),
                ("declare_blockers", true),
                ("combat_damage", true),
                ("end_combat", false),
                ("island_attacker:declare_attackers", true),
                ("island_attacker:declare_blockers", true),
                ("island_attacker:combat_damage", true),
                ("island_attacker:end_combat", false),
            ],
            _ => vec![
                ("own_upkeep", true),
                ("opponent_upkeep", true),
                ("first_main", false),
                ("begin_combat", false),
                ("next_main", false),
            ],
        };
        for (time, legal) in cases {
            let expected = json!({"activation_offered":legal,"activation_announced":legal});
            let (status, actual, evidence) = match run(&def, index, time) {
                Ok((a, e)) => (
                    if a == expected {
                        "expected_result_observed"
                    } else {
                        "semantic_mismatch"
                    },
                    a,
                    e,
                ),
                Err(e) => (
                    "fixture_or_execution_error",
                    json!({"error":e}),
                    Value::Null,
                ),
            };
            rows.push(json!({"card":name,"scenario":time,"authored_restriction":text,"status":status,"expected":expected,"actual":actual,"fixture_evidence":evidence,"artifact_checksum":artifact.payload_checksum,"scope":"Established source permanent with summoning sickness removed, legal target and cost resources. Observe normal legal-action discovery and completed activation announcement. Combat cases advance engine phases, declare a real attacker, declare no blockers, and execute damage before later windows; no additional combat. Ability resolution is outside this timing test."}));
        }
    }
    let after: Vec<_> = paths
        .iter()
        .map(|p| json!({"path":p,"sha256":hash(p)}))
        .collect();
    let report = json!({"rows":rows,"compilation":compilation,"provenance":{"before":before,"after":after,"artifacts_unchanged":before==after,"strict_artifact":true,"unique_card_ids":true,"seed":71757432704849_u64},"rules_review":"temporal-activation-rules.json","limitations":["Historical established source permanents are seeded directly; source spells are not cast in this gate audit.","An announced ability is not resolved. This tests its availability and payment, not its resulting effects.","Additional/skipped combats, control changes, and noncontroller activation permissions are not covered."]});
    std::fs::write(
        root.join("reports/runtime-audit/temporal-activation-reproductions.json"),
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
    println!(
        "wrote {} temporal cases",
        report["rows"].as_array().unwrap().len()
    );
}
