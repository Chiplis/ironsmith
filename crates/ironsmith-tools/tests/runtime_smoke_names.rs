//! Generic runtime smoke harness over a names file. EXPENSIVE: ignored by
//! default and does nothing unless IRONSMITH_RUNTIME_SMOKE_NAMES is set.
//!
//!   IRONSMITH_RUNTIME_SMOKE_NAMES=names.txt \
//!   [IRONSMITH_RUNTIME_SMOKE_CARDS=cards.json] \
//!   [IRONSMITH_RUNTIME_SMOKE_STRICT_COMPILE=1] \
//!   cargo test -p ironsmith-tools --test <suite> runtime_smoke_names -- --ignored --nocapture
//!
//! One card name per line (blank lines and `#` comments skipped). Each face
//! is compiled from its real Oracle payload, then (1) put onto the
//! battlefield of a three-player sandbox (when it is a permanent) followed by
//! a continuous-state refresh, SBAs, trigger draining and legal-action
//! discovery for every player, with a zone-index invariant check; and (2) run
//! through the bounded runtime-audit probes (legal casts/activations from
//! hand, battlefield and graveyard, plus injected trigger events). Panics and
//! invariant violations fail the test; compile failures are listed and only
//! fail it under STRICT_COMPILE. Passing proves no crash, not correct rules.
use ironsmith::decision::{SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::game_loop::{check_and_apply_sbas_with, drain_pending_trigger_events};
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardDefinition, GameState, ObjectId, Phase, PlayerId, Zone};
use ironsmith_tools::runtime_audit::{actions, execution};
use std::panic::{AssertUnwindSafe, catch_unwind};

fn panic_text(panic: Box<dyn std::any::Any + Send>) -> String {
    panic
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| panic.downcast_ref::<&str>().map(|text| text.to_string()))
        .unwrap_or_else(|| "non-string panic".into())
}

fn zone_invariant(game: &GameState) -> Result<(), String> {
    let mut seen = std::collections::HashSet::new();
    let mut check = |zone: Zone, ids: Vec<ObjectId>| -> Result<(), String> {
        for id in ids {
            let object = game
                .object(id)
                .ok_or_else(|| format!("{zone:?} lists missing object {id:?}"))?;
            if object.zone != zone {
                return Err(format!("{zone:?} lists {id:?} whose zone is {:?}", object.zone));
            }
            if !seen.insert(id) {
                return Err(format!("{id:?} is indexed in more than one zone"));
            }
        }
        Ok(())
    };
    check(Zone::Battlefield, game.battlefield.iter().copied().collect())?;
    for player in &game.players {
        check(Zone::Hand, player.hand.to_vec())?;
        check(Zone::Library, player.library.to_vec())?;
        check(Zone::Graveyard, player.graveyard.to_vec())?;
    }
    Ok(())
}

/// Put the permanent onto a sandbox battlefield and exercise the engine's
/// always-on machinery around it.
fn battlefield_sandbox(definition: &CardDefinition) -> Result<(), String> {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Cara".into()], 20);
    game.turn.active_player = PlayerId(0);
    game.turn.priority_player = Some(PlayerId(0));
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.turn_number = 3;
    let id = game.create_object_from_definition(definition, PlayerId(0), Zone::Battlefield);
    game.remove_summoning_sickness(id);
    game.refresh_continuous_state()
        .map_err(|error| format!("continuous refresh: {error:?}"))?;
    let mut queue = TriggerQueue::new();
    let mut dm = SelectFirstDecisionMaker;
    check_and_apply_sbas_with(&mut game, &mut queue, &mut dm)
        .map_err(|error| format!("SBA: {error}"))?;
    drain_pending_trigger_events(&mut game, &mut queue);
    let _ = game.calculated_power(id);
    let _ = game.calculated_toughness(id);
    for player in 0..3 {
        game.turn.priority_player = Some(PlayerId(player));
        compute_legal_actions(&game, PlayerId(player))
            .map_err(|error| format!("legal actions for seat {player}: {error}"))?;
    }
    zone_invariant(&game)
}

#[test]
#[ignore = "expensive: generic runtime smoke over IRONSMITH_RUNTIME_SMOKE_NAMES"]
fn runtime_smoke_names() {
    let Ok(names_path) = std::env::var("IRONSMITH_RUNTIME_SMOKE_NAMES") else {
        eprintln!("IRONSMITH_RUNTIME_SMOKE_NAMES unset; nothing to smoke");
        return;
    };
    let cards = std::env::var("IRONSMITH_RUNTIME_SMOKE_CARDS").unwrap_or_else(|_| {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../cards.json")
            .display()
            .to_string()
    });
    let strict_compile = std::env::var("IRONSMITH_RUNTIME_SMOKE_STRICT_COMPILE").is_ok();
    let names: Vec<String> = std::fs::read_to_string(&names_path)
        .unwrap_or_else(|error| panic!("read {names_path}: {error}"))
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_string)
        .collect();
    let payloads = ironsmith_tools::load_card_payloads_by_names(&cards, &names)
        .unwrap_or_else(|error| panic!("load {cards}: {error}"));

    let mut failures = Vec::new();
    let mut compile_failures = Vec::new();
    let mut smoked = 0usize;
    for name in &names {
        let Some(faces) = payloads.get(&ironsmith_tools::normalize_lookup_name(name)) else {
            compile_failures.push(format!("{name}: not found in {cards}"));
            continue;
        };
        for payload in faces {
            let face = payload.parse_name.as_deref().unwrap_or(&payload.name);
            let compiled = catch_unwind(AssertUnwindSafe(|| {
                ironsmith_tools::parse_card_definition_with_runtime_builder(
                    face,
                    payload.parse_input.clone(),
                    false,
                )
            }));
            let definition = match compiled {
                Err(panic) => {
                    failures.push(format!("{face}: compiler panicked: {}", panic_text(panic)));
                    continue;
                }
                Ok(Err(error)) => {
                    compile_failures.push(format!("{face}: {error}"));
                    continue;
                }
                Ok(Ok(definition)) => definition,
            };
            smoked += 1;
            if definition.is_permanent() {
                match catch_unwind(AssertUnwindSafe(|| battlefield_sandbox(&definition))) {
                    Ok(Ok(())) => {}
                    Ok(Err(error)) => failures.push(format!("{face}: battlefield sandbox: {error}")),
                    Err(panic) => failures.push(format!(
                        "{face}: battlefield sandbox panicked: {}",
                        panic_text(panic)
                    )),
                }
            }
            let observations = catch_unwind(AssertUnwindSafe(|| {
                let mut rows = actions::audit(&definition);
                rows.extend(execution::audit(&definition));
                rows
            }));
            match observations {
                Err(panic) => failures.push(format!("{face}: audit panicked: {}", panic_text(panic))),
                Ok(rows) => {
                    for row in rows {
                        if matches!(row.status.as_str(), "panicked" | "invariant_failed") {
                            failures.push(format!(
                                "{face}: {} {} {}: {}",
                                row.ability_path, row.scenario, row.status, row.detail
                            ));
                        }
                    }
                }
            }
        }
    }
    eprintln!(
        "runtime smoke: {} names, {smoked} faces exercised, {} compile failures, {} runtime failures",
        names.len(),
        compile_failures.len(),
        failures.len()
    );
    for line in &compile_failures {
        eprintln!("  compile: {line}");
    }
    assert!(failures.is_empty(), "runtime failures:\n{}", failures.join("\n"));
    assert!(
        !strict_compile || compile_failures.is_empty(),
        "compile failures:\n{}",
        compile_failures.join("\n")
    );
}
