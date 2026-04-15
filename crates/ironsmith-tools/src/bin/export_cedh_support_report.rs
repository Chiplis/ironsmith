use std::cmp::Reverse;
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::panic::{self, AssertUnwindSafe};
use std::time::Duration;

use ironsmith::cards::{
    CardDefinition, CardDefinitionBuilder, generated_definition_has_unimplemented_content,
};
use ironsmith::compiled_text::compiled_lines;
use ironsmith::ids::CardId;
use ironsmith::semantic_compare::compare_semantics_scored;
use ironsmith::tooling::{CardStatusDb, CompilationSnapshot, ParseStatus, default_db_path};
use rayon::prelude::*;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug)]
struct Args {
    cards_path: String,
    out_csv: String,
    top_n: usize,
    lookback_days: i32,
    concurrency: usize,
    event_limit: Option<usize>,
    db_path: String,
    no_db: bool,
}

#[derive(Debug)]
struct CardPayload {
    parse_input: String,
    oracle_text: String,
}

#[derive(Debug, Clone)]
struct RankedCard {
    rank: usize,
    name: String,
    occurrences: usize,
}

#[derive(Debug)]
struct EventDeckSummary {
    players: usize,
    decklists_with_embedded_text: usize,
    card_counts: HashMap<String, usize>,
}

#[derive(Debug)]
struct Row {
    rank: usize,
    name: String,
    occurrences: usize,
    supported: bool,
    parsed: bool,
    parse_strict: bool,
    parse_with_allow_unsupported: bool,
    status: String,
    parse_error_strict: String,
    parse_error_allow_unsupported: String,
    has_unimplemented: bool,
    semantic_mismatch: bool,
    oracle_coverage: f32,
    compiled_coverage: f32,
    similarity_score: f32,
    line_delta: isize,
    compiled_lines_count: usize,
}

#[derive(Debug, Serialize)]
struct EventFilterRequest {
    #[serde(rename = "gameFilters")]
    game_filters: Vec<GameFilter>,
    #[serde(rename = "dateFilter")]
    date_filter: String,
    #[serde(rename = "distFilter")]
    dist_filter: String,
    #[serde(rename = "zipCode")]
    zip_code: String,
}

#[derive(Debug, Serialize)]
struct GameFilter {
    game: String,
    format: String,
}

#[derive(Debug, Deserialize)]
struct EventFilterResponse {
    #[serde(rename = "currEvents", default)]
    curr_events: Vec<TopdeckEvent>,
}

#[derive(Debug, Deserialize, Clone)]
struct TopdeckEvent {
    id: String,
    name: String,
    #[serde(default)]
    players: Option<usize>,
    #[serde(rename = "startUnix", default)]
    start_unix: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct PublicPlayerEntry {
    #[serde(default)]
    decklist: Option<String>,
}

#[derive(Debug)]
enum ParseOutcome {
    Success(CardDefinition),
    Error(String),
}

fn parse_args() -> Result<Args, String> {
    let mut cards_path = "cards.json".to_string();
    let mut out_csv = "cedh_top_2000_support.csv".to_string();
    let mut top_n = 2_000usize;
    let mut lookback_days = 90i32;
    let mut concurrency = 24usize;
    let mut event_limit = None;
    let mut db_path = default_db_path().display().to_string();
    let mut no_db = false;

    let mut iter = env::args().skip(1);
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--cards" => {
                cards_path = iter
                    .next()
                    .ok_or_else(|| "--cards requires a path".to_string())?;
            }
            "--out" => {
                out_csv = iter
                    .next()
                    .ok_or_else(|| "--out requires a path".to_string())?;
            }
            "--top" => {
                let raw = iter
                    .next()
                    .ok_or_else(|| "--top requires a number".to_string())?;
                top_n = raw
                    .parse::<usize>()
                    .map_err(|err| format!("invalid --top '{raw}': {err}"))?;
            }
            "--days" => {
                let raw = iter
                    .next()
                    .ok_or_else(|| "--days requires a number".to_string())?;
                lookback_days = raw
                    .parse::<i32>()
                    .map_err(|err| format!("invalid --days '{raw}': {err}"))?;
            }
            "--concurrency" => {
                let raw = iter
                    .next()
                    .ok_or_else(|| "--concurrency requires a number".to_string())?;
                concurrency = raw
                    .parse::<usize>()
                    .map_err(|err| format!("invalid --concurrency '{raw}': {err}"))?;
            }
            "--event-limit" => {
                let raw = iter
                    .next()
                    .ok_or_else(|| "--event-limit requires a number".to_string())?;
                event_limit = Some(
                    raw.parse::<usize>()
                        .map_err(|err| format!("invalid --event-limit '{raw}': {err}"))?,
                );
            }
            "--db-path" => {
                db_path = iter
                    .next()
                    .ok_or_else(|| "--db-path requires a path".to_string())?;
            }
            "--no-db" => {
                no_db = true;
            }
            "-h" | "--help" => {
                return Err("usage: cargo run --release --bin export_cedh_support_report -- [--cards <path>] [--out <path>] [--top <n>] [--days <n>] [--concurrency <n>] [--event-limit <n>] [--db-path <path>] [--no-db]".to_string());
            }
            _ => {
                return Err(format!(
                    "unknown argument '{arg}'. expected --cards/--out/--top/--days/--concurrency/--event-limit/--db-path/--no-db"
                ));
            }
        }
    }

    if top_n == 0 {
        return Err("--top must be greater than 0".to_string());
    }
    if lookback_days <= 0 {
        return Err("--days must be greater than 0".to_string());
    }
    if concurrency == 0 {
        return Err("--concurrency must be greater than 0".to_string());
    }

    Ok(Args {
        cards_path,
        out_csv,
        top_n,
        lookback_days,
        concurrency,
        event_limit,
        db_path,
        no_db,
    })
}

fn build_client() -> Result<Client, Box<dyn std::error::Error>> {
    Ok(Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent("ironsmith/cedh-support-report")
        .build()?)
}

fn is_explicit_cedh_event(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.contains("cedh") || lower.contains("competitive edh")
}

fn fetch_cedh_events(
    client: &Client,
    lookback_days: i32,
) -> Result<Vec<TopdeckEvent>, Box<dyn std::error::Error>> {
    let request = EventFilterRequest {
        game_filters: vec![GameFilter {
            game: "Magic: The Gathering".to_string(),
            format: "EDH".to_string(),
        }],
        date_filter: format!("-{lookback_days}"),
        dist_filter: String::new(),
        zip_code: String::new(),
    };

    let response = client
        .post("https://topdeck.gg/api/event-filter")
        .json(&request)
        .send()?
        .error_for_status()?
        .json::<EventFilterResponse>()?;

    let mut unique = HashSet::new();
    let mut events = response
        .curr_events
        .into_iter()
        .filter(|event| is_explicit_cedh_event(&event.name))
        .filter(|event| unique.insert(event.id.clone()))
        .collect::<Vec<_>>();

    events.sort_by_key(|event| {
        (
            Reverse(event.players.unwrap_or_default()),
            Reverse(event.start_unix.unwrap_or_default()),
            event.id.clone(),
        )
    });

    Ok(events)
}

fn decode_embedded_decklist(raw: &str) -> String {
    raw.replace("\\n", "\n")
        .replace("\\r", "\r")
        .replace("\\t", "\t")
        .replace("\\'", "'")
}

fn extract_deck_card_names(raw: &str) -> Vec<String> {
    let decoded = decode_embedded_decklist(raw);
    decoded
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            let (qty, name) = trimmed.split_once(' ')?;
            let quantity = qty.parse::<usize>().ok()?;
            if quantity == 0 {
                return None;
            }
            let name = name.trim();
            if name.is_empty() {
                return None;
            }
            Some(name.to_string())
        })
        .collect()
}

fn fetch_public_event_deck_counts(
    client: &Client,
    event: &TopdeckEvent,
) -> Result<EventDeckSummary, Box<dyn std::error::Error + Send + Sync>> {
    let url = format!("https://topdeck.gg/PublicPData/{}", event.id);
    let payload = client
        .get(url)
        .send()?
        .error_for_status()?
        .json::<HashMap<String, PublicPlayerEntry>>()?;

    let mut players = 0usize;
    let mut decklists_with_embedded_text = 0usize;
    let mut card_counts = HashMap::<String, usize>::new();

    for entry in payload.into_values() {
        players += 1;
        let Some(decklist) = entry.decklist else {
            continue;
        };
        if !decklist.contains("Mainboard") {
            continue;
        }

        let cards = extract_deck_card_names(&decklist);
        if cards.is_empty() {
            continue;
        }

        decklists_with_embedded_text += 1;
        let unique_cards = cards.into_iter().collect::<HashSet<_>>();
        for name in unique_cards {
            *card_counts.entry(name).or_insert(0) += 1;
        }
    }

    Ok(EventDeckSummary {
        players,
        decklists_with_embedded_text,
        card_counts,
    })
}

fn build_ranked_cards(event_summaries: &[EventDeckSummary], top_n: usize) -> Vec<RankedCard> {
    let mut aggregate = HashMap::<String, usize>::new();
    for summary in event_summaries {
        for (name, count) in &summary.card_counts {
            *aggregate.entry(name.clone()).or_insert(0) += count;
        }
    }

    let mut ranked = aggregate.into_iter().collect::<Vec<_>>();
    ranked.sort_by(|(left_name, left_count), (right_name, right_count)| {
        right_count
            .cmp(left_count)
            .then_with(|| left_name.cmp(right_name))
    });

    ranked
        .into_iter()
        .take(top_n)
        .enumerate()
        .map(|(index, (name, occurrences))| RankedCard {
            rank: index + 1,
            name,
            occurrences,
        })
        .collect()
}

fn get_first_face(card: &Value) -> Option<&Value> {
    card.get("card_faces")
        .and_then(|faces| faces.as_array())
        .and_then(|faces| faces.first())
}

fn value_to_string(value: &Value) -> Option<String> {
    if value.is_null() {
        return None;
    }
    if let Some(s) = value.as_str() {
        return Some(s.to_string());
    }
    Some(value.to_string())
}

fn pick_field(card: &Value, face: Option<&Value>, key: &str) -> Option<String> {
    if let Some(v) = card.get(key).and_then(value_to_string) {
        return Some(v);
    }
    face.and_then(|f| f.get(key)).and_then(value_to_string)
}

fn build_card_payload(card: &Value) -> Option<CardPayload> {
    let face = get_first_face(card);
    let mana_cost = pick_field(card, face, "mana_cost");
    let type_line = pick_field(card, face, "type_line");
    let oracle_text = pick_field(card, face, "oracle_text").unwrap_or_default();
    let power = pick_field(card, face, "power");
    let toughness = pick_field(card, face, "toughness");
    let loyalty = pick_field(card, face, "loyalty");
    let defense = pick_field(card, face, "defense");

    let mut lines = Vec::new();
    if let Some(mana_cost) = mana_cost
        && !mana_cost.is_empty()
    {
        lines.push(format!("Mana cost: {mana_cost}"));
    }
    if let Some(type_line) = type_line
        && !type_line.is_empty()
    {
        lines.push(format!("Type: {type_line}"));
    }
    if let (Some(power), Some(toughness)) = (power, toughness)
        && !power.is_empty()
        && !toughness.is_empty()
    {
        lines.push(format!("Power/Toughness: {power}/{toughness}"));
    }
    if let Some(loyalty) = loyalty
        && !loyalty.is_empty()
    {
        lines.push(format!("Loyalty: {loyalty}"));
    }
    if let Some(defense) = defense
        && !defense.is_empty()
    {
        lines.push(format!("Defense: {defense}"));
    }
    if !oracle_text.is_empty() {
        lines.push(oracle_text.clone());
    }

    if lines.is_empty() {
        return None;
    }

    Some(CardPayload {
        parse_input: lines.join("\n"),
        oracle_text,
    })
}

fn load_cards(path: &str) -> Result<HashMap<String, CardPayload>, Box<dyn std::error::Error>> {
    let raw = fs::read_to_string(path)?;
    let cards: Vec<Value> = serde_json::from_str(&raw)?;
    let mut map = HashMap::new();
    for card in cards {
        let Some(name) = card
            .get("name")
            .and_then(|value| value.as_str())
            .map(str::to_string)
        else {
            continue;
        };
        if map.contains_key(&name) {
            continue;
        }
        if let Some(payload) = build_card_payload(&card) {
            map.insert(name, payload);
        }
    }
    Ok(map)
}

fn normalize_lookup_name(name: &str) -> String {
    let trimmed = name.trim();
    if trimmed.contains(" / ") && !trimmed.contains(" // ") {
        return trimmed.replacen(" / ", " // ", 1);
    }
    trimmed.to_string()
}

fn set_allow_unsupported(enabled: bool) {
    unsafe {
        if enabled {
            env::set_var("IRONSMITH_PARSER_ALLOW_UNSUPPORTED", "1");
        } else {
            env::remove_var("IRONSMITH_PARSER_ALLOW_UNSUPPORTED");
        }
    }
}

fn panic_payload_to_string(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(msg) = payload.downcast_ref::<&str>() {
        return (*msg).to_string();
    }
    if let Some(msg) = payload.downcast_ref::<String>() {
        return msg.clone();
    }
    "unknown panic payload".to_string()
}

fn parse_card(name: &str, parse_input: &str, allow_unsupported: bool) -> ParseOutcome {
    set_allow_unsupported(allow_unsupported);
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        CardDefinitionBuilder::new(CardId::new(), name).parse_text(parse_input.to_string())
    }));
    match result {
        Ok(Ok(definition)) => ParseOutcome::Success(definition),
        Ok(Err(err)) => ParseOutcome::Error(format!("{err:?}")),
        Err(payload) => ParseOutcome::Error(format!("panic: {}", panic_payload_to_string(payload))),
    }
}

fn build_rows(
    ranked_cards: &[RankedCard],
    cards: &HashMap<String, CardPayload>,
    db: Option<&CardStatusDb>,
) -> Result<Vec<Row>, Box<dyn std::error::Error>> {
    let original_allow_unsupported = env::var("IRONSMITH_PARSER_ALLOW_UNSUPPORTED").ok();
    let mut rows = Vec::with_capacity(ranked_cards.len());

    for ranked in ranked_cards {
        let mut row = Row {
            rank: ranked.rank,
            name: ranked.name.clone(),
            occurrences: ranked.occurrences,
            supported: false,
            parsed: false,
            parse_strict: false,
            parse_with_allow_unsupported: false,
            status: String::new(),
            parse_error_strict: String::new(),
            parse_error_allow_unsupported: String::new(),
            has_unimplemented: false,
            semantic_mismatch: false,
            oracle_coverage: 0.0,
            compiled_coverage: 0.0,
            similarity_score: 0.0,
            line_delta: 0,
            compiled_lines_count: 0,
        };

        let payload = cards
            .get(&ranked.name)
            .or_else(|| cards.get(&normalize_lookup_name(&ranked.name)));

        let Some(payload) = payload else {
            row.status = "missing_card_data".to_string();
            rows.push(row);
            continue;
        };

        let strict_result = parse_card(&ranked.name, &payload.parse_input, false);
        let snapshot = match strict_result {
            ParseOutcome::Success(definition) => {
                populate_row_from_definition(&mut row, payload, &definition);
                row.parsed = true;
                row.parse_strict = true;
                row.parse_with_allow_unsupported = true;
                build_snapshot_from_row(
                    &ranked.name,
                    payload,
                    ParseStatus::StrictCompiled,
                    None,
                    Some(&definition),
                )
            }
            ParseOutcome::Error(err) => {
                row.parse_error_strict = err.clone();
                let allow_result = parse_card(&ranked.name, &payload.parse_input, true);
                match allow_result {
                    ParseOutcome::Success(definition) => {
                        populate_row_from_definition(&mut row, payload, &definition);
                        row.parsed = true;
                        row.parse_with_allow_unsupported = true;
                        build_snapshot_from_row(
                            &ranked.name,
                            payload,
                            ParseStatus::CompiledWithAllowUnsupported,
                            None,
                            Some(&definition),
                        )
                    }
                    ParseOutcome::Error(allow_err) => {
                        row.parse_error_allow_unsupported = allow_err;
                        row.status = "does_not_parse".to_string();
                        build_snapshot_from_row(
                            &ranked.name,
                            payload,
                            ParseStatus::ParseFailed,
                            Some(err),
                            None,
                        )
                    }
                }
            }
        };

        row.supported = row.parsed && !row.has_unimplemented;
        if row.supported {
            row.status = "supported".to_string();
        } else if row.status.is_empty() {
            row.status = "unsupported".to_string();
        }

        if let Some(db) = db {
            db.insert_snapshot_if_changed(&snapshot)?;
        }
        rows.push(row);
    }

    match original_allow_unsupported {
        Some(value) => unsafe {
            env::set_var("IRONSMITH_PARSER_ALLOW_UNSUPPORTED", value);
        },
        None => set_allow_unsupported(false),
    }

    Ok(rows)
}

fn build_snapshot_from_row(
    ranked_name: &str,
    payload: &CardPayload,
    parse_status: ParseStatus,
    parse_error: Option<String>,
    definition: Option<&CardDefinition>,
) -> CompilationSnapshot {
    CompilationSnapshot::from_definition_result(
        ranked_name,
        &payload.oracle_text,
        parse_status,
        parse_error,
        definition,
    )
}

fn populate_row_from_definition(row: &mut Row, payload: &CardPayload, definition: &CardDefinition) {
    let compiled = compiled_lines(&definition);
    row.compiled_lines_count = compiled.len();
    row.has_unimplemented = generated_definition_has_unimplemented_content(&definition);
    let (oracle_cov, compiled_cov, similarity, line_delta, semantic_mismatch) =
        compare_semantics_scored(&payload.oracle_text, &compiled, None);
    row.oracle_coverage = oracle_cov;
    row.compiled_coverage = compiled_cov;
    row.similarity_score = similarity;
    row.line_delta = line_delta;
    row.semantic_mismatch = semantic_mismatch;
}

fn csv_escape(value: &str) -> String {
    let needs_quotes =
        value.contains(',') || value.contains('"') || value.contains('\n') || value.contains('\r');
    if !needs_quotes {
        return value.to_string();
    }
    let escaped = value.replace('"', "\"\"");
    format!("\"{escaped}\"")
}

fn write_csv(path: &str, rows: &[Row]) -> Result<(), Box<dyn std::error::Error>> {
    let mut out = String::new();
    out.push_str("rank,name,occurrences,supported,parsed,parse_strict,parse_with_allow_unsupported,status,parse_error_strict,parse_error_allow_unsupported,has_unimplemented,semantic_mismatch,oracle_coverage,compiled_coverage,similarity_score,line_delta,compiled_lines_count\n");

    for row in rows {
        out.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{},{},{},{:.6},{:.6},{:.6},{},{}\n",
            row.rank,
            csv_escape(&row.name),
            row.occurrences,
            row.supported,
            row.parsed,
            row.parse_strict,
            row.parse_with_allow_unsupported,
            csv_escape(&row.status),
            csv_escape(&row.parse_error_strict),
            csv_escape(&row.parse_error_allow_unsupported),
            row.has_unimplemented,
            row.semantic_mismatch,
            row.oracle_coverage,
            row.compiled_coverage,
            row.similarity_score,
            row.line_delta,
            row.compiled_lines_count,
        ));
    }

    fs::write(path, out)?;
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = parse_args().map_err(|err| {
        eprintln!("{err}");
        err
    })?;

    let client = build_client()?;
    let mut events = fetch_cedh_events(&client, args.lookback_days)?;
    if let Some(limit) = args.event_limit {
        events.truncate(limit);
    }

    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(args.concurrency)
        .build()?;

    let event_summaries = pool.install(|| {
        events
            .par_iter()
            .filter_map(
                |event| match fetch_public_event_deck_counts(&client, event) {
                    Ok(summary) if summary.decklists_with_embedded_text > 0 => Some(summary),
                    Ok(_) => None,
                    Err(err) => {
                        eprintln!(
                            "warning: failed to fetch {} ({}): {err}",
                            event.id, event.name
                        );
                        None
                    }
                },
            )
            .collect::<Vec<_>>()
    });

    let total_events_with_decklists = event_summaries.len();
    let total_embedded_decklists = event_summaries
        .iter()
        .map(|summary| summary.decklists_with_embedded_text)
        .sum::<usize>();
    let total_players_seen = event_summaries
        .iter()
        .map(|summary| summary.players)
        .sum::<usize>();

    let ranked_cards = build_ranked_cards(&event_summaries, args.top_n);
    let cards = load_cards(&args.cards_path)?;
    let db = if args.no_db {
        None
    } else {
        Some(CardStatusDb::open(&args.db_path)?)
    };
    let mut rows = build_rows(&ranked_cards, &cards, db.as_ref())?;

    rows.sort_by(|left, right| {
        left.supported.cmp(&right.supported).then_with(|| {
            if left.supported && right.supported {
                left.similarity_score
                    .total_cmp(&right.similarity_score)
                    .then_with(|| left.rank.cmp(&right.rank))
            } else {
                left.rank.cmp(&right.rank)
            }
        })
    });

    write_csv(&args.out_csv, &rows)?;

    let supported_count = rows.iter().filter(|row| row.supported).count();
    let unsupported_count = rows.len().saturating_sub(supported_count);

    println!("cEDH support report complete");
    println!(
        "- Source: TopDeck public EDH events from the last {} days with explicit cEDH naming",
        args.lookback_days
    );
    println!("- Candidate events scanned: {}", events.len());
    println!("- Events with embedded decklists: {total_events_with_decklists}");
    println!("- Players seen across fetched public payloads: {total_players_seen}");
    println!("- Embedded decklists counted: {total_embedded_decklists}");
    println!("- Ranked cards exported: {}", rows.len());
    println!("- Supported: {supported_count}");
    println!("- Unsupported: {unsupported_count}");
    println!("- CSV: {}", args.out_csv);
    if !args.no_db {
        println!("- DB: {}", args.db_path);
    }

    Ok(())
}
