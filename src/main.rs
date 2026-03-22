//! Ironsmith - Magic: The Gathering Rules Engine
//!
//! Interactive CLI for playing MTG with two random decks.
//!
//! ## Usage
//!
//! ```
//! ironsmith [OPTIONS]
//!
//! Options:
//!   --hand "Card1 | Card2 | ..."   Specify starting hand (can be repeated for each player)
//!   --deck "Card1 | Card2 | ..."   Specify deck contents (can be repeated for each player)
//! ```
//!
//! The first --hand/--deck is for Alice, the second for Bob.
//! Players without specified hands/decks get random ones.

use ironsmith::cards::CardDefinitionBuilder;
use ironsmith::cards::builders::CardTextError;
use ironsmith::decision::{CliDecisionMaker, DecisionRouter, init_input_manager, read_input};
use ironsmith::ids::CardId;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{
    CardDefinition, CardRegistry, CombatState, GameState, ManaSymbol, PlayerId, Zone,
    execute_turn_with,
};
use rand::seq::SliceRandom;
use serde::Deserialize;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::BufReader;
use std::path::Path;

/// Analyze mana costs of cards and return color distribution.
fn analyze_mana_colors(cards: &[&CardDefinition]) -> HashMap<ManaSymbol, u32> {
    let mut colors = HashMap::new();

    for card in cards {
        if let Some(ref cost) = card.card.mana_cost {
            for pip in cost.pips() {
                for symbol in pip {
                    match symbol {
                        ManaSymbol::White
                        | ManaSymbol::Blue
                        | ManaSymbol::Black
                        | ManaSymbol::Red
                        | ManaSymbol::Green => {
                            *colors.entry(*symbol).or_insert(0) += 1;
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    colors
}

/// Get the basic land for a color.
fn basic_land_for_color(registry: &CardRegistry, color: &ManaSymbol) -> Option<CardDefinition> {
    match color {
        ManaSymbol::White => registry.get("Plains").cloned(),
        ManaSymbol::Blue => registry.get("Island").cloned(),
        ManaSymbol::Black => registry.get("Swamp").cloned(),
        ManaSymbol::Red => registry.get("Mountain").cloned(),
        ManaSymbol::Green => registry.get("Forest").cloned(),
        _ => None,
    }
}

/// Build a random 40-card deck (23 spells + 17 lands).
fn build_random_deck(registry: &CardRegistry) -> Vec<CardDefinition> {
    use rand::prelude::IndexedRandom;

    let mut rng = rand::rng();

    // Get all non-land cards
    let non_lands: Vec<&CardDefinition> = registry.all().filter(|c| !c.card.is_land()).collect();

    if non_lands.is_empty() {
        println!("Warning: No non-land cards found!");
        return Vec::new();
    }

    // Pick 23 random spells (with replacement allowed for variety)
    let mut spells: Vec<&CardDefinition> = Vec::new();
    for _ in 0..23 {
        if let Some(card) = non_lands.choose(&mut rng) {
            spells.push(*card);
        }
    }

    // Analyze colors in the spells
    let color_counts = analyze_mana_colors(&spells);

    // Calculate total colored symbols
    let total_symbols: u32 = color_counts.values().sum();

    // Distribute 17 lands based on color ratios
    let mut deck: Vec<CardDefinition> = spells.iter().map(|c| (*c).clone()).collect();

    if total_symbols > 0 {
        for (color, count) in &color_counts {
            let land_count = ((*count as f64 / total_symbols as f64) * 17.0).round() as usize;
            if let Some(land) = basic_land_for_color(registry, color) {
                for _ in 0..land_count {
                    deck.push(land.clone());
                }
            }
        }
    }

    // Fill remaining slots with the most common land type
    while deck.len() < 40 {
        let most_common_color = color_counts
            .iter()
            .max_by_key(|(_, count)| *count)
            .map(|(color, _)| color);

        if let Some(color) = most_common_color {
            if let Some(land) = basic_land_for_color(registry, color) {
                deck.push(land);
            }
        } else {
            // No colors? Add forests as default
            if let Some(forest) = registry.get("Forest") {
                deck.push(forest.clone());
            } else {
                break;
            }
        }
    }

    // Shuffle the deck
    deck.shuffle(&mut rng);

    deck
}

/// Run a full game between two players, optionally skipping initial draws.
fn run_game_with_custom_hands(
    game: &mut GameState,
    player1_has_custom_hand: bool,
    player2_has_custom_hand: bool,
) {
    let mut decision_maker = DecisionRouter::new(Box::new(CliDecisionMaker));
    let mut combat = CombatState::default();
    let mut trigger_queue = TriggerQueue::new();

    println!("\n========================================");
    println!("         GAME START!");
    println!("========================================\n");

    // Draw initial hands (skip for players with custom hands)
    let player1 = PlayerId::from_index(0);
    let player2 = PlayerId::from_index(1);
    if !player1_has_custom_hand {
        game.draw_cards(player1, 7);
    }
    if !player2_has_custom_hand {
        game.draw_cards(player2, 7);
    }

    // Main game loop
    let mut turn_count = 0;
    let max_turns = 100; // Safety limit

    while turn_count < max_turns {
        turn_count += 1;

        // Check for game over
        let remaining: Vec<_> = game.players.iter().filter(|p| p.is_in_game()).collect();

        if remaining.len() <= 1 {
            if let Some(winner) = remaining.first() {
                println!("\n========================================");
                println!("  {} WINS!", winner.name);
                println!("========================================");
            } else {
                println!("\n========================================");
                println!("  DRAW!");
                println!("========================================");
            }
            return;
        }

        // Run a turn
        if let Err(e) =
            execute_turn_with(game, &mut combat, &mut trigger_queue, &mut decision_maker)
        {
            match e {
                ironsmith::GameLoopError::GameOver => {
                    // Check who won
                    for player in &game.players {
                        if player.is_in_game() {
                            println!("\n========================================");
                            println!("  {} WINS!", player.name);
                            println!("========================================");
                            return;
                        }
                    }
                }
                _ => {
                    println!("Error during game: {}", e);
                    return;
                }
            }
        }

        // Switch active player
        let next_player = PlayerId::from_index(
            ((game.turn.active_player.index() + 1) % game.players.len()) as u8,
        );
        game.turn.active_player = next_player;
        game.turn.priority_player = Some(next_player); // Reset priority to new active player
        game.turn.turn_number += 1;

        // Reset turn state for new player
        if let Some(player) = game.player_mut(next_player) {
            player.begin_turn();
        }
    }

    println!("Game ended due to turn limit.");
}

/// Command-line arguments for custom hands/decks.
struct GameArgs {
    /// Starting hands for each player (index 0 = Alice, etc.)
    hands: Vec<Vec<String>>,
    /// Decks for each player (index 0 = Alice, etc.)
    decks: Vec<Vec<String>>,
    /// Starting battlefield for each player (index 0 = Alice, etc.)
    battlefields: Vec<Vec<String>>,
    /// Starting graveyard for each player (index 0 = Alice, etc.)
    graveyards: Vec<Vec<String>>,
    /// Starting exile for each player (index 0 = Alice, etc.)
    exiles: Vec<Vec<String>>,
    /// Commanders for each player (index 0 = Alice, etc.)
    commanders: Vec<Vec<String>>,
    /// File to record inputs to (for creating replay tests)
    record_file: Option<String>,
    /// File to replay inputs from (for automated testing)
    replay_file: Option<String>,
    /// Whether to generate random decks/hands for players without custom ones
    random: bool,
    /// Cards to generate definitions from oracle text (meta mode)
    meta_cards: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct CardJson {
    name: String,
    oracle_text: Option<String>,
    card_faces: Option<Vec<CardFace>>,
    lang: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CardFace {
    name: Option<String>,
    oracle_text: Option<String>,
}

fn parse_card_arg(value: &str) -> Vec<String> {
    value
        .split(" | ")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn oracle_text_for_card(card: &CardJson, query: &str) -> Option<String> {
    if let Some(text) = card.oracle_text.as_ref() {
        return Some(text.clone());
    }

    let Some(faces) = card.card_faces.as_ref() else {
        return None;
    };

    if let Some(face) = faces.iter().find(|face| {
        face.name
            .as_deref()
            .map(|name| name.eq_ignore_ascii_case(query))
            .unwrap_or(false)
    }) && let Some(text) = face.oracle_text.as_ref()
    {
        return Some(text.clone());
    }

    let mut parts = Vec::new();
    for face in faces {
        if let Some(text) = face.oracle_text.as_ref() {
            parts.push(text.as_str());
        }
    }

    if parts.is_empty() {
        None
    } else {
        Some(parts.join("\n"))
    }
}

fn lookup_oracle_text(cards: &[CardJson], query: &str) -> Option<(String, String)> {
    let query = query.trim();
    if query.is_empty() {
        return None;
    }

    let is_english = |card: &&CardJson| card.lang.as_deref().unwrap_or("en") == "en";

    for card in cards.iter().filter(is_english) {
        if card.name.eq_ignore_ascii_case(query)
            && let Some(text) = oracle_text_for_card(card, query)
        {
            return Some((card.name.clone(), text));
        }
    }

    for card in cards.iter().filter(is_english) {
        if let Some(faces) = card.card_faces.as_ref()
            && faces.iter().any(|face| {
                face.name
                    .as_deref()
                    .map(|name| name.eq_ignore_ascii_case(query))
                    .unwrap_or(false)
            })
            && let Some(text) = oracle_text_for_card(card, query)
        {
            return Some((query.to_string(), text));
        }
    }

    for card in cards {
        if card.name.eq_ignore_ascii_case(query)
            && let Some(text) = oracle_text_for_card(card, query)
        {
            return Some((card.name.clone(), text));
        }
    }

    for card in cards {
        if let Some(faces) = card.card_faces.as_ref()
            && faces.iter().any(|face| {
                face.name
                    .as_deref()
                    .map(|name| name.eq_ignore_ascii_case(query))
                    .unwrap_or(false)
            })
            && let Some(text) = oracle_text_for_card(card, query)
        {
            return Some((query.to_string(), text));
        }
    }

    None
}

fn load_cards_json() -> Result<Vec<CardJson>, String> {
    let file =
        fs::File::open("cards.json").map_err(|err| format!("Failed to open cards.json: {err}"))?;
    let reader = BufReader::new(file);
    serde_json::from_reader(reader).map_err(|err| format!("Failed to parse cards.json: {err}"))
}

fn run_meta(card_names: &[String]) -> Result<(), String> {
    let cards = load_cards_json()?;

    for name in card_names {
        match lookup_oracle_text(&cards, name) {
            Some((resolved_name, text)) => {
                println!("=== {} ===", resolved_name);
                let builder = CardDefinitionBuilder::new(CardId::new(), resolved_name.clone());
                match builder.parse_text(text.as_str()) {
                    Ok(definition) => {
                        println!("{definition:#?}");
                    }
                    Err(CardTextError::ParseError(message)) => {
                        eprintln!("Failed to parse {}: {message}", resolved_name);
                    }
                    Err(CardTextError::UnsupportedLine(message)) => {
                        eprintln!("Unsupported line for {}: {message}", resolved_name);
                    }
                    Err(CardTextError::InvariantViolation(message)) => {
                        eprintln!(
                            "Parser invariant violation for {}: {message}",
                            resolved_name
                        );
                    }
                }
            }
            None => {
                eprintln!("Card not found in cards.json: '{}'", name);
            }
        }
    }

    Ok(())
}

/// Parse command-line arguments.
fn parse_args() -> GameArgs {
    let args: Vec<String> = env::args().collect();
    let mut hands: Vec<Vec<String>> = Vec::new();
    let mut decks: Vec<Vec<String>> = Vec::new();
    let mut battlefields: Vec<Vec<String>> = Vec::new();
    let mut graveyards: Vec<Vec<String>> = Vec::new();
    let mut exiles: Vec<Vec<String>> = Vec::new();
    let mut commanders: Vec<Vec<String>> = Vec::new();
    let mut record_file: Option<String> = None;
    let mut replay_file: Option<String> = None;
    let mut random: bool = false;
    let mut meta_cards: Vec<String> = Vec::new();

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--record" => {
                if i + 1 < args.len() {
                    record_file = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    eprintln!("Error: --record requires a file path");
                    i += 1;
                }
            }
            "--replay" => {
                if i + 1 < args.len() {
                    replay_file = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    eprintln!("Error: --replay requires a file path");
                    i += 1;
                }
            }
            "--hand" => {
                if i + 1 < args.len() {
                    let cards = parse_card_arg(&args[i + 1]);
                    hands.push(cards);
                    i += 2;
                } else {
                    eprintln!("Error: --hand requires a value");
                    i += 1;
                }
            }
            "--deck" => {
                if i + 1 < args.len() {
                    let cards = parse_card_arg(&args[i + 1]);
                    decks.push(cards);
                    i += 2;
                } else {
                    eprintln!("Error: --deck requires a value");
                    i += 1;
                }
            }
            "--battlefield" => {
                if i + 1 < args.len() {
                    let cards = parse_card_arg(&args[i + 1]);
                    battlefields.push(cards);
                    i += 2;
                } else {
                    eprintln!("Error: --battlefield requires a value");
                    i += 1;
                }
            }
            "--graveyard" => {
                if i + 1 < args.len() {
                    let cards = parse_card_arg(&args[i + 1]);
                    graveyards.push(cards);
                    i += 2;
                } else {
                    eprintln!("Error: --graveyard requires a value");
                    i += 1;
                }
            }
            "--exile" => {
                if i + 1 < args.len() {
                    let cards = parse_card_arg(&args[i + 1]);
                    exiles.push(cards);
                    i += 2;
                } else {
                    eprintln!("Error: --exile requires a value");
                    i += 1;
                }
            }
            "--commander" => {
                if i + 1 < args.len() {
                    let cards = parse_card_arg(&args[i + 1]);
                    commanders.push(cards);
                    i += 2;
                } else {
                    eprintln!("Error: --commander requires a value");
                    i += 1;
                }
            }
            "--meta" => {
                if i + 1 < args.len() {
                    let cards = parse_card_arg(&args[i + 1]);
                    meta_cards.extend(cards);
                    i += 2;
                } else {
                    eprintln!("Error: --meta requires a value");
                    i += 1;
                }
            }
            "--random" => {
                random = true;
                i += 1;
            }
            "--help" | "-h" => {
                println!("Ironsmith - MTG Rules Engine");
                println!();
                println!("Usage: ironsmith [OPTIONS]");
                println!();
                println!("Options:");
                println!(
                    "  --hand \"Card1 | Card2 | ...\"        Specify starting hand (repeatable)"
                );
                println!(
                    "  --deck \"Card1 | Card2 | ...\"        Specify deck contents (repeatable)"
                );
                println!(
                    "  --battlefield \"Card1 | Card2 | ...\" Specify starting battlefield (repeatable)"
                );
                println!(
                    "  --graveyard \"Card1 | Card2 | ...\"   Specify starting graveyard (repeatable)"
                );
                println!(
                    "  --exile \"Card1 | Card2 | ...\"       Specify starting exile (repeatable)"
                );
                println!(
                    "  --commander \"Card1 | Card2 | ...\"   Specify commander(s) in command zone (repeatable)"
                );
                println!(
                    "  --meta \"Card1 | Card2 | ...\"       Print generated definitions from oracle text"
                );
                println!(
                    "  --record <file>                      Record inputs to file for replay tests"
                );
                println!(
                    "                                      (default: replays/####.txt when not using --replay)"
                );
                println!(
                    "  --replay <file>                      Replay inputs from file (automated testing)"
                );
                println!(
                    "  --random                             Generate random decks/hands for unspecified players"
                );
                println!("  --help, -h                           Show this help message");
                println!();
                println!("The first instance of each option is for Alice, the second for Bob.");
                println!("Without --random, players only have the cards explicitly specified.");
                std::process::exit(0);
            }
            _ => {
                eprintln!("Unknown argument: {}", args[i]);
                i += 1;
            }
        }
    }

    GameArgs {
        hands,
        decks,
        battlefields,
        graveyards,
        exiles,
        commanders,
        record_file,
        replay_file,
        random,
        meta_cards,
    }
}

/// Parse a list of card names and look them up in the registry.
/// Returns the found cards and prints warnings for cards not found.
fn parse_card_list(registry: &CardRegistry, card_names: &[String]) -> Vec<CardDefinition> {
    let mut cards = Vec::new();
    for name in card_names {
        if let Some(card) = registry.get(name) {
            cards.push(card.clone());
        } else {
            eprintln!("Warning: Card not found: '{}'", name);
        }
    }
    cards
}

fn default_replay_path() -> Option<String> {
    let dir = Path::new("replays");
    if let Err(err) = fs::create_dir_all(dir) {
        eprintln!("Warning: failed to create replays folder: {err}");
        return None;
    }

    let mut max_id: u32 = 0;
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str())
                && let Ok(id) = stem.parse::<u32>()
                && id > max_id
            {
                max_id = id;
            }
        }
    }

    let next_id = max_id.saturating_add(1);
    let file_name = format!("{:04}.txt", next_id);
    Some(dir.join(file_name).to_string_lossy().into_owned())
}

// Loading the handwritten registry can exceed the default Windows main-thread stack.
const CLI_STACK_SIZE: usize = 64 * 1024 * 1024;

fn run_cli() {
    let args = parse_args();

    if !args.meta_cards.is_empty() {
        if let Err(err) = run_meta(&args.meta_cards) {
            eprintln!("{err}");
            std::process::exit(1);
        }
        return;
    }

    let record_file = if args.record_file.is_none() && args.replay_file.is_none() {
        default_replay_path()
    } else {
        args.record_file.clone()
    };

    // Initialize input manager for recording/replay
    init_input_manager(record_file.as_deref(), args.replay_file.as_deref());

    println!("========================================");
    println!("   Ironsmith - MTG Rules Engine");
    println!("========================================\n");

    if let Some(path) = record_file.as_deref()
        && args.replay_file.is_none()
    {
        println!("Recording replay to: {}\n", path);
    }

    // Create the card registry
    let registry = CardRegistry::with_builtin_cards();

    // Build decks for each player
    let deck1 = if !args.decks.is_empty() {
        let deck = parse_card_list(&registry, &args.decks[0]);
        println!("Alice deck ({} cards) - custom:", deck.len());
        deck
    } else if args.random {
        println!("Building random 40-card deck for Alice...");
        build_random_deck(&registry)
    } else {
        println!("Alice deck: empty (use --random for random decks)");
        Vec::new()
    };

    let deck2 = if args.decks.len() > 1 {
        let deck = parse_card_list(&registry, &args.decks[1]);
        println!("Bob deck ({} cards) - custom:", deck.len());
        deck
    } else if args.random {
        println!("Building random 40-card deck for Bob...");
        build_random_deck(&registry)
    } else {
        println!("Bob deck: empty (use --random for random decks)");
        Vec::new()
    };

    // Print deck contents
    println!("\nAlice deck ({} cards):", deck1.len());
    let mut card_counts1: HashMap<String, usize> = HashMap::new();
    for card in &deck1 {
        *card_counts1.entry(card.name().to_string()).or_insert(0) += 1;
    }
    for (name, count) in &card_counts1 {
        println!("  {}x {}", count, name);
    }

    println!("\nBob deck ({} cards):", deck2.len());
    let mut card_counts2: HashMap<String, usize> = HashMap::new();
    for card in &deck2 {
        *card_counts2.entry(card.name().to_string()).or_insert(0) += 1;
    }
    for (name, count) in &card_counts2 {
        println!("  {}x {}", count, name);
    }

    // Create the game
    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);

    let player1 = PlayerId::from_index(0);
    let player2 = PlayerId::from_index(1);

    // Parse custom hands (if specified)
    let hand1: Vec<CardDefinition> = if !args.hands.is_empty() {
        parse_card_list(&registry, &args.hands[0])
    } else {
        Vec::new()
    };

    let hand2: Vec<CardDefinition> = if args.hands.len() > 1 {
        parse_card_list(&registry, &args.hands[1])
    } else {
        Vec::new()
    };

    // Add cards to libraries (excluding hand cards for custom hands)
    for card in &deck1 {
        game.create_object_from_definition(card, player1, Zone::Library);
    }
    for card in &deck2 {
        game.create_object_from_definition(card, player2, Zone::Library);
    }

    // Add custom hand cards directly to hand
    if !hand1.is_empty() {
        println!("\nAlice starting hand ({} cards):", hand1.len());
        for card in &hand1 {
            println!("  - {}", card.name());
            game.create_object_from_definition(card, player1, Zone::Hand);
        }
    }

    if !hand2.is_empty() {
        println!("\nBob starting hand ({} cards):", hand2.len());
        for card in &hand2 {
            println!("  - {}", card.name());
            game.create_object_from_definition(card, player2, Zone::Hand);
        }
    }

    // Add custom battlefield cards
    let battlefield1: Vec<CardDefinition> = if !args.battlefields.is_empty() {
        parse_card_list(&registry, &args.battlefields[0])
    } else {
        Vec::new()
    };
    let battlefield2: Vec<CardDefinition> = if args.battlefields.len() > 1 {
        parse_card_list(&registry, &args.battlefields[1])
    } else {
        Vec::new()
    };

    if !battlefield1.is_empty() {
        println!(
            "\nAlice starting battlefield ({} cards):",
            battlefield1.len()
        );
        for card in &battlefield1 {
            println!("  - {}", card.name());
            game.create_object_from_definition(card, player1, Zone::Battlefield);
        }
    }
    if !battlefield2.is_empty() {
        println!("\nBob starting battlefield ({} cards):", battlefield2.len());
        for card in &battlefield2 {
            println!("  - {}", card.name());
            game.create_object_from_definition(card, player2, Zone::Battlefield);
        }
    }

    // Add custom graveyard cards
    let graveyard1: Vec<CardDefinition> = if !args.graveyards.is_empty() {
        parse_card_list(&registry, &args.graveyards[0])
    } else {
        Vec::new()
    };
    let graveyard2: Vec<CardDefinition> = if args.graveyards.len() > 1 {
        parse_card_list(&registry, &args.graveyards[1])
    } else {
        Vec::new()
    };

    if !graveyard1.is_empty() {
        println!("\nAlice starting graveyard ({} cards):", graveyard1.len());
        for card in &graveyard1 {
            println!("  - {}", card.name());
            game.create_object_from_definition(card, player1, Zone::Graveyard);
        }
    }
    if !graveyard2.is_empty() {
        println!("\nBob starting graveyard ({} cards):", graveyard2.len());
        for card in &graveyard2 {
            println!("  - {}", card.name());
            game.create_object_from_definition(card, player2, Zone::Graveyard);
        }
    }

    // Add custom exile cards
    let exile1: Vec<CardDefinition> = if !args.exiles.is_empty() {
        parse_card_list(&registry, &args.exiles[0])
    } else {
        Vec::new()
    };
    let exile2: Vec<CardDefinition> = if args.exiles.len() > 1 {
        parse_card_list(&registry, &args.exiles[1])
    } else {
        Vec::new()
    };

    if !exile1.is_empty() {
        println!("\nAlice starting exile ({} cards):", exile1.len());
        for card in &exile1 {
            println!("  - {}", card.name());
            game.create_object_from_definition(card, player1, Zone::Exile);
        }
    }
    if !exile2.is_empty() {
        println!("\nBob starting exile ({} cards):", exile2.len());
        for card in &exile2 {
            println!("  - {}", card.name());
            game.create_object_from_definition(card, player2, Zone::Exile);
        }
    }

    // Add commanders to command zone
    let commander1: Vec<CardDefinition> = if !args.commanders.is_empty() {
        parse_card_list(&registry, &args.commanders[0])
    } else {
        Vec::new()
    };
    let commander2: Vec<CardDefinition> = if args.commanders.len() > 1 {
        parse_card_list(&registry, &args.commanders[1])
    } else {
        Vec::new()
    };

    if !commander1.is_empty() {
        println!("\nAlice commander(s) ({} cards):", commander1.len());
        for card in &commander1 {
            println!("  - {}", card.name());
            let obj_id = game.create_object_from_definition(card, player1, Zone::Command);
            game.set_as_commander(obj_id, player1);
        }
    }
    if !commander2.is_empty() {
        println!("\nBob commander(s) ({} cards):", commander2.len());
        for card in &commander2 {
            println!("  - {}", card.name());
            let obj_id = game.create_object_from_definition(card, player2, Zone::Command);
            game.set_as_commander(obj_id, player2);
        }
    }

    println!("\nStarting game...");
    println!("Press Enter to continue...");
    let _ = read_input().unwrap_or_default();

    // Run the game (pass whether players have custom hands to skip drawing)
    run_game_with_custom_hands(&mut game, !hand1.is_empty(), !hand2.is_empty());
}

fn main() {
    let handle = std::thread::Builder::new()
        .name("ironsmith-cli".to_string())
        .stack_size(CLI_STACK_SIZE)
        .spawn(run_cli)
        .expect("failed to spawn ironsmith CLI thread");

    if let Err(payload) = handle.join() {
        std::panic::resume_unwind(payload);
    }
}
