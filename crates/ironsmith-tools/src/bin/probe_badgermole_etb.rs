//! Reproduces the reported slow Badgermole Cub ETB and reports where the time
//! goes.
//!
//! Board under test, from a live game: Agatha's Soul Cauldron with a Llanowar
//! Elves exiled under it, so every creature with a +1/+1 counter has
//! `{T}: Add {G}`; Myr Moonvessel carrying a counter; and Badgermole Cub
//! entering, whose earthbend trigger animates a land and puts a counter on it —
//! which makes that land a *new* creature with the granted mana ability while
//! the trigger is resolving. Badgermole Cub also has "Whenever you tap a
//! creature for mana, add an additional {G}".
//!
//! Run with:
//!   cargo run --release -p ironsmith-tools --bin probe_badgermole_etb

use std::time::Instant;

use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::game_state::GameState;
use ironsmith::ids::{CardId, ObjectId, PlayerId};
use ironsmith::object::CounterType;
use ironsmith::triggers::TriggerQueue;
use ironsmith::zone::Zone;
use ironsmith_registry::card::PowerToughness;
use ironsmith_registry::cards::CardDefinition;
use ironsmith_registry::cards::builders::CardDefinitionBuilder;
use ironsmith_registry::types::{CardType, Subtype, Supertype};

#[derive(serde::Deserialize)]
struct RawCard {
    name: String,
    #[serde(default)]
    type_line: Option<String>,
    #[serde(default)]
    oracle_text: Option<String>,
    #[serde(default)]
    power: Option<String>,
    #[serde(default)]
    toughness: Option<String>,
}

fn oracle_index() -> std::collections::HashMap<String, RawCard> {
    let path = format!("{}/../../cards.json", env!("CARGO_MANIFEST_DIR"));
    let raw = std::fs::read_to_string(&path).expect("read cards.json");
    let cards: Vec<RawCard> = serde_json::from_str(&raw).expect("parse cards.json");
    let mut out = std::collections::HashMap::new();
    for card in cards {
        out.entry(card.name.clone()).or_insert(card);
    }
    out
}

fn card_types_for(type_line: &str) -> (Vec<Supertype>, Vec<CardType>, Vec<Subtype>) {
    let mut supertypes = Vec::new();
    let mut types = Vec::new();
    let head = type_line.split('—').next().unwrap_or(type_line);
    for word in head.split_whitespace() {
        match word {
            "Legendary" => supertypes.push(Supertype::Legendary),
            "Basic" => supertypes.push(Supertype::Basic),
            "Snow" => supertypes.push(Supertype::Snow),
            "Artifact" => types.push(CardType::Artifact),
            "Creature" => types.push(CardType::Creature),
            "Land" => types.push(CardType::Land),
            "Enchantment" => types.push(CardType::Enchantment),
            "Instant" => types.push(CardType::Instant),
            "Sorcery" => types.push(CardType::Sorcery),
            _ => {}
        }
    }
    (supertypes, types, Vec::new())
}

fn compile(index: &std::collections::HashMap<String, RawCard>, name: &str) -> CardDefinition {
    let card = index
        .get(name)
        .unwrap_or_else(|| panic!("card '{name}' missing from cards.json"));
    let type_line = card.type_line.clone().unwrap_or_default();
    let (supertypes, types, subtypes) = card_types_for(&type_line);
    let mut builder = CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(types)
        .supertypes(supertypes)
        .subtypes(subtypes);
    if let (Some(power), Some(toughness)) = (card.power.as_deref(), card.toughness.as_deref())
        && let (Ok(power), Ok(toughness)) = (power.parse::<i32>(), toughness.parse::<i32>())
    {
        builder = builder.power_toughness(PowerToughness::fixed(power, toughness));
    }
    let text = card.oracle_text.clone().unwrap_or_default();
    if text.trim().is_empty() {
        return builder.build();
    }
    builder
        .clone()
        .parse_text(text.clone())
        .unwrap_or_else(|err| panic!("compile '{name}' failed: {err:?}"))
}

struct Board {
    game: GameState,
    alice: PlayerId,
    forest: ObjectId,
    cub_definition: CardDefinition,
}

fn build_board(players: usize, extra_lands: usize, cauldron: bool) -> Board {
    let index = oracle_index();
    let names: Vec<String> = (0..players).map(|i| format!("P{i}")).collect::<Vec<_>>();
    let mut game = GameState::new(names, 20);
    let alice = PlayerId::from_index(0);

    // Lands, matching the reported board.
    let land_names = [
        "Forest",
        "Island",
        "Swamp",
        "Mountain",
        "Plains",
        "Tropical Island",
        "Volcanic Island",
        "City of Traitors",
    ];
    let mut forest = None;
    for (index_of, name) in land_names.iter().take(2 + extra_lands).enumerate() {
        let definition = compile(&index, name);
        let id = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        if index_of == 0 {
            forest = Some(id);
        }
    }

    // Myr Moonvessel already carries a +1/+1 counter in the reported game.
    let moonvessel = compile(&index, "Myr Moonvessel");
    let moonvessel_id = game.create_object_from_definition(&moonvessel, alice, Zone::Battlefield);
    game.add_counters(moonvessel_id, CounterType::PlusOnePlusOne, 1);

    if cauldron {
        let asc = compile(&index, "Agatha's Soul Cauldron");
        let asc_id = game.create_object_from_definition(&asc, alice, Zone::Battlefield);
        // Llanowar Elves exiled *with* the Cauldron: that link is what makes its
        // `{T}: Add {G}` reachable by every counter-bearing creature.
        let elves = compile(&index, "Llanowar Elves");
        let elves_id = game.create_object_from_definition(&elves, alice, Zone::Exile);
        game.add_exiled_with_source_link(asc_id, elves_id);
    }

    // Opponents get a few permanents so the board is not trivially small.
    for player_index in 1..players {
        let player = PlayerId::from_index(player_index as u8);
        for name in ["Forest", "Island", "Swamp"] {
            let definition = compile(&index, name);
            game.create_object_from_definition(&definition, player, Zone::Battlefield);
        }
    }

    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    game.turn.step = None;
    game.refresh_continuous_state();

    Board {
        game,
        alice,
        forest: forest.expect("forest created"),
        cub_definition: compile(&index, "Badgermole Cub"),
    }
}

fn main() {
    if std::env::var("PROBE_REPORTED").is_ok() {
        report_reported_board();
        return;
    }
    if std::env::var("PROBE_REPORTED_LOOP").is_ok() {
        let (mut game, alice, definitions) = reported_board(3);
        let mut queue = TriggerQueue::default();
        let mut dm = SelectFirstDecisionMaker;
        for definition in &definitions {
            add_with_etb(&mut game, definition, alice, &mut queue, &mut dm);
            let _ =
                ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm);
            let mut resolved = 0;
            while !game.stack.is_empty() && resolved < 6 {
                if ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).is_err() {
                    break;
                }
                resolved += 1;
            }
        }
        let started = Instant::now();
        let mut n = 0u64;
        while started.elapsed().as_secs() < 25 {
            let _ = ironsmith::decision::compute_legal_actions(&game, alice);
            n += 1;
        }
        println!("iterations={n}");
        return;
    }
    // Profiling mode: hammer the expensive case so a sampler can catch it.
    if std::env::var("PROBE_LOOP").is_ok() {
        let board = build_board(4, 6, true);
        let started = Instant::now();
        let mut iterations = 0u64;
        while started.elapsed().as_secs() < 25 {
            let _ = ironsmith::decision::compute_legal_actions(&board.game, board.alice);
            iterations += 1;
        }
        println!("loop iterations={iterations}");
        return;
    }
    println!(
        "{:>8} {:>6} {:>9} | {:>10} {:>10} {:>10} {:>10} {:>10}",
        "players",
        "lands",
        "cauldron",
        "enter_ms",
        "triggers_ms",
        "resolve_ms",
        "sba_ms",
        "legal_ms"
    );
    for cauldron in [false, true] {
        for (players, extra_lands) in [(2usize, 2usize), (4, 2), (4, 6)] {
            let mut board = build_board(players, extra_lands, cauldron);
            let mut queue = TriggerQueue::default();
            let mut dm = SelectFirstDecisionMaker;

            let started = Instant::now();
            let _cub = board.game.create_object_from_definition(
                &board.cub_definition,
                board.alice,
                Zone::Battlefield,
            );
            let enter_ms = started.elapsed().as_secs_f64() * 1000.0;

            let started = Instant::now();
            let _ = ironsmith::game_loop::put_triggers_on_stack_with_dm(
                &mut board.game,
                &mut queue,
                &mut dm,
            );
            let triggers_ms = started.elapsed().as_secs_f64() * 1000.0;

            let started = Instant::now();
            let mut resolved = 0;
            while !board.game.stack.is_empty() && resolved < 8 {
                if ironsmith::game_loop::resolve_stack_entry_with(&mut board.game, &mut dm).is_err()
                {
                    break;
                }
                resolved += 1;
            }
            let resolve_ms = started.elapsed().as_secs_f64() * 1000.0;

            let started = Instant::now();
            let _ = ironsmith::game_loop::check_and_apply_sbas_with(
                &mut board.game,
                &mut queue,
                &mut dm,
            );
            let sba_ms = started.elapsed().as_secs_f64() * 1000.0;

            let started = Instant::now();
            let actions = ironsmith::decision::compute_legal_actions(&board.game, board.alice);
            let legal_ms = started.elapsed().as_secs_f64() * 1000.0;
            let perf = ironsmith::decision::last_compute_legal_actions_perf();

            println!(
                "{:>8} {:>6} {:>9} | {:>10.1} {:>10.1} {:>10.1} {:>10.1} {:>10.1}  (actions={}, forest_is_creature={})",
                players,
                2 + extra_lands,
                cauldron,
                enter_ms,
                triggers_ms,
                resolve_ms,
                sba_ms,
                legal_ms,
                actions.len(),
                board
                    .game
                    .object(board.forest)
                    .is_some_and(|object| object.card_types.contains(&CardType::Creature)),
            );
            if let Some(perf) = perf
                && perf.total_ms > 5.0
            {
                let mut rows = vec![
                    ("derived_view", perf.derived_view_ms),
                    ("prewarm", perf.prewarm_ms),
                    ("cast_context", perf.cast_context_ms),
                    (
                        "battlefield_ability_context",
                        perf.battlefield_ability_context_ms,
                    ),
                    ("hand_casts", perf.hand_casts_ms),
                    ("battlefield_abilities", perf.battlefield_abilities_ms),
                    (
                        "  bf_ability_precheck",
                        perf.battlefield_ability_precheck_ms,
                    ),
                    (
                        "  bf_ability_target_legality",
                        perf.battlefield_ability_target_legality_ms,
                    ),
                    (
                        "  bf_ability_cost_build",
                        perf.battlefield_ability_cost_build_ms,
                    ),
                    (
                        "  bf_ability_affordability",
                        perf.battlefield_ability_affordability_ms,
                    ),
                    (
                        "non_battlefield_abilities",
                        perf.non_battlefield_abilities_ms,
                    ),
                    (
                        "compute_potential_mana",
                        perf.compute_potential_mana_with_view_ms,
                    ),
                    ("lands", perf.lands_ms),
                ];
                rows.sort_by(|a, b| b.1.total_cmp(&a.1));
                for (label, ms) in rows {
                    if ms >= 0.05 {
                        println!("        {label:32} {ms:8.2} ms");
                    }
                }
            }
        }
    }
}

/// Reproduces the exact board from the reported diagnostics journal:
/// Alice and Bob each with Omniscience, Agatha's Soul Cauldron, Yawgmoth,
/// Ornithopter, Myr Moonvessel and seven lands, four players, and Alice
/// adding Badgermole Cubs one at a time. Each Cub carries "Whenever you tap a
/// creature for mana, add an additional {G}".
fn reported_board(cubs: usize) -> (GameState, PlayerId, Vec<CardDefinition>) {
    let index = oracle_index();
    let mut game = GameState::new(
        vec![
            "Alice".into(),
            "Bob".into(),
            "Charlie".into(),
            "Diana".into(),
        ],
        20,
    );
    let alice = PlayerId::from_index(0);
    let shared = [
        "Omniscience",
        "Forest",
        "Plains",
        "Island",
        "Mountain",
        "Swamp",
        "Tropical Island",
        "Volcanic Island",
        "Yawgmoth, Thran Physician",
        "Ornithopter",
        "Myr Moonvessel",
        "Agatha's Soul Cauldron",
    ];
    for player_index in 0..2u8 {
        let player = PlayerId::from_index(player_index);
        for name in shared {
            let definition = compile(&index, name);
            game.create_object_from_definition(&definition, player, Zone::Battlefield);
        }
    }
    for player_index in 0..4u8 {
        let player = PlayerId::from_index(player_index);
        for _ in 0..2 {
            let definition = compile(&index, "Swamp");
            game.create_object_from_definition(&definition, player, Zone::Exile);
        }
        for _ in 0..5 {
            let definition = compile(&index, "Plains");
            game.create_object_from_definition(&definition, player, Zone::Graveyard);
        }
    }
    for name in [
        "Raise Dead",
        "Lightning Bolt",
        "Raise Dead",
        "Swamp",
        "Counterspell",
        "Plains",
        "Llanowar Elves",
        "Unsummon",
    ] {
        let definition = compile(&index, name);
        game.create_object_from_definition(&definition, alice, Zone::Hand);
    }

    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    game.turn.step = None;
    game.refresh_continuous_state();

    let cub = compile(&index, "Badgermole Cub");
    (game, alice, vec![cub; cubs])
}

/// Adds a permanent the way the UI's `addCardToZone` does when triggers are
/// wanted: create in the command zone, then move to the battlefield so the
/// zone-change and enter-the-battlefield triggers fire naturally.
fn add_with_etb(
    game: &mut GameState,
    definition: &CardDefinition,
    player: PlayerId,
    queue: &mut TriggerQueue,
    dm: &mut SelectFirstDecisionMaker,
) -> Option<ObjectId> {
    let temp = game.create_object_from_definition(definition, player, Zone::Command);
    let result = game.move_object_with_etb_processing_with_dm(temp, Zone::Battlefield, dm)?;
    let entered = result.new_id;
    let provenance = game
        .provenance_graph_mut()
        .alloc_root_event(ironsmith::events::EventKind::EnterBattlefield);
    let event = ironsmith::triggers::TriggerEvent::new_with_provenance(
        ironsmith::events::EnterBattlefieldEvent::new(entered, Zone::Command),
        provenance,
    );
    game.queue_trigger_event(provenance, event);
    ironsmith::game_loop::drain_pending_trigger_events(game, queue);
    Some(entered)
}

fn report_reported_board() {
    println!(
        "{:>5} | {:>10} {:>12} {:>12} {:>12}",
        "cubs", "add_ms", "triggers_ms", "resolve_ms", "legal_ms"
    );
    for cubs in 1..=3usize {
        let (mut game, alice, definitions) = reported_board(cubs);
        let mut queue = TriggerQueue::default();
        let mut dm = SelectFirstDecisionMaker;
        let mut add_ms = 0.0;
        let mut triggers_ms = 0.0;
        let mut resolve_ms = 0.0;
        for definition in &definitions {
            let started = Instant::now();
            add_with_etb(&mut game, definition, alice, &mut queue, &mut dm);
            add_ms += started.elapsed().as_secs_f64() * 1000.0;

            let started = Instant::now();
            let _ =
                ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm);
            triggers_ms += started.elapsed().as_secs_f64() * 1000.0;

            let started = Instant::now();
            let mut resolved = 0;
            while !game.stack.is_empty() && resolved < 6 {
                match ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm) {
                    Ok(()) => {}
                    Err(err) => {
                        if std::env::var("PROBE_VERBOSE").is_ok() {
                            println!("      resolve error: {err:?} (stack={})", game.stack.len());
                        }
                        break;
                    }
                }
                resolved += 1;
            }
            if std::env::var("PROBE_VERBOSE").is_ok() {
                println!(
                    "      after resolve: stack={} resolved={resolved}",
                    game.stack.len()
                );
            }
            resolve_ms += started.elapsed().as_secs_f64() * 1000.0;
        }
        let clean_before = game.continuous_state_is_clean_public();
        let started = Instant::now();
        let actions = ironsmith::decision::compute_legal_actions(&game, alice);
        let legal_ms = started.elapsed().as_secs_f64() * 1000.0;
        game.refresh_continuous_state();
        let started = Instant::now();
        let _ = ironsmith::decision::compute_legal_actions(&game, alice);
        let legal_clean_ms = started.elapsed().as_secs_f64() * 1000.0;
        game.refresh_continuous_state();
        let animated = game
            .battlefield
            .iter()
            .filter(|id| {
                game.calculated_characteristics_arc(**id).is_some_and(|c| {
                    c.card_types.contains(&CardType::Land)
                        && c.card_types.contains(&CardType::Creature)
                })
            })
            .count();
        let with_counters: u32 = game
            .battlefield
            .iter()
            .filter_map(|id| game.object(*id))
            .map(|o| o.counters.values().sum::<u32>())
            .sum();
        println!(
            "      clean_before={clean_before} legal_after_refresh={legal_clean_ms:.1} ms animated_lands={animated} with_counters={with_counters}"
        );
        println!(
            "{cubs:>5} | {add_ms:>10.1} {triggers_ms:>12.1} {resolve_ms:>12.1} {legal_ms:>12.1}   (actions={}, bf={})",
            actions.len(),
            game.battlefield.len()
        );
    }
}
