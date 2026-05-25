use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::Path;
use std::process::Command;
use std::thread;

use ironsmith_tools::{
    CardPayload, CardStatusDb, build_parse_input, compile_snapshot_from_payload,
};
use rusqlite::Connection;
use serde_json::Value;
use tempfile::tempdir;

fn write_cards_json(path: &Path) {
    fs::write(
        path,
        r#"[
  {
    "name":"Lightning Bolt",
    "oracle_text":"Lightning Bolt deals 3 damage to any target.",
    "mana_cost":"{R}",
    "type_line":"Instant"
  },
  {
    "name":"Counterspell",
    "oracle_text":"Counter target spell.",
    "mana_cost":"{U}{U}",
    "type_line":"Instant"
  }
]"#,
    )
    .expect("write cards.json");
}

fn write_cards_with_abrade_json(path: &Path) {
    fs::write(
        path,
        r#"[
  {
    "name":"Lightning Bolt",
    "oracle_text":"Lightning Bolt deals 3 damage to any target.",
    "mana_cost":"{R}",
    "type_line":"Instant"
  },
  {
    "name":"Abrade",
    "oracle_text":"Choose one — Abrade deals 3 damage to target creature; or destroy target artifact.",
    "mana_cost":"{1}{R}",
    "type_line":"Instant"
  }
]"#,
    )
    .expect("write cards.json with abrade");
}

fn write_cards_with_unsupported_json(path: &Path) {
    fs::write(
        path,
        r#"[
  {
    "name":"Unsupported Fixture",
    "oracle_text":"Choose a word. You win the game if a sentence contains that word.",
    "mana_cost":"{1}{U}",
    "type_line":"Sorcery"
  }
]"#,
    )
    .expect("write cards.json with unsupported card");
}

fn write_cards_with_semantic_mismatch_json(path: &Path) {
    fs::write(
        path,
        r#"[
  {
    "name":"Mismatch Fixture",
    "oracle_text":"Activated abilities of creatures you control cost {2} less to activate. This effect can't reduce the mana in that cost to less than one mana.",
    "mana_cost":"{1}{W}",
    "type_line":"Enchantment"
  }
]"#,
    )
    .expect("write cards.json with semantic mismatch card");
}

fn write_cards_with_parenthetical_json(path: &Path) {
    fs::write(
        path,
        r#"[
  {
    "name":"Sky Drake",
    "oracle_text":"Flying (This creature can't be blocked except by creatures with flying or reach.)",
    "mana_cost":"{1}{U}",
    "type_line":"Creature — Drake",
    "power":"2",
    "toughness":"1"
  }
]"#,
    )
    .expect("write cards.json with parenthetical reminder text");
}

fn write_cards_with_worker_stack_pressure_json(path: &Path) {
    fs::write(
        path,
        r#"[
  {
    "name":"Lightning Bolt",
    "oracle_text":"Lightning Bolt deals 3 damage to any target.",
    "mana_cost":"{R}",
    "type_line":"Instant"
  },
  {
    "name":"\"Lifetime\" Pass Holder",
    "oracle_text":"This creature enters tapped.\nWhen this creature dies, open an Attraction.\nWhenever you roll to visit your Attractions, if you roll a 6, you may return this card from your graveyard to the battlefield.",
    "mana_cost":"{B}",
    "type_line":"Creature — Zombie Guest",
    "power":"2",
    "toughness":"1"
  }
]"#,
    )
    .expect("write cards.json with worker-stack-pressure card");
}

fn query_count(db_path: &Path, sql: &str) -> i64 {
    let conn = Connection::open(db_path).expect("open sqlite db");
    conn.query_row(sql, [], |row| row.get(0))
        .expect("query count")
}

fn make_payload(name: &str, oracle_text: &str, mana_cost: &str, type_line: &str) -> CardPayload {
    let metadata_lines = vec![
        format!("Mana cost: {mana_cost}"),
        format!("Type: {type_line}"),
    ];
    CardPayload {
        name: name.to_string(),
        parse_name: None,
        oracle_text: oracle_text.to_string(),
        raw_oracle_text: oracle_text.to_string(),
        parse_input: build_parse_input(&metadata_lines, oracle_text),
        metadata_lines,
        other_face_name: None,
        linked_face_layout: None,
    }
}

fn sync_registry_db(cards_path: &Path, db_path: &Path) {
    sync_registry_db_with_args(cards_path, db_path, &[]);
}

fn sync_registry_db_with_args(cards_path: &Path, db_path: &Path, extra_args: &[&str]) {
    let mut command = Command::new(env!("CARGO_BIN_EXE_sync_registry_db"));
    command
        .arg("--cards")
        .arg(cards_path)
        .arg("--db-path")
        .arg(db_path);
    for arg in extra_args {
        command.arg(arg);
    }
    let status = command.status().expect("run sync_registry_db");
    assert!(status.success(), "sync_registry_db should succeed");
}

fn spawn_mock_tagger_server() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
    let addr = listener.local_addr().expect("server addr");

    thread::spawn(move || {
        for _ in 0..8 {
            let (mut stream, _) = listener.accept().expect("accept connection");
            let mut reader = BufReader::new(stream.try_clone().expect("clone stream"));
            let mut request_line = String::new();
            reader
                .read_line(&mut request_line)
                .expect("read request line");

            let mut content_length = 0usize;
            let mut cookie_header = None;
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).expect("read header line");
                if line == "\r\n" || line.is_empty() {
                    break;
                }
                let lowercase = line.to_ascii_lowercase();
                if let Some(value) = lowercase.strip_prefix("content-length:") {
                    content_length = value.trim().parse().expect("content-length");
                }
                if let Some((name, value)) = line.split_once(':')
                    && name.eq_ignore_ascii_case("cookie")
                {
                    cookie_header = Some(value.trim().to_string());
                }
            }

            let mut body = vec![0u8; content_length];
            if content_length > 0 {
                reader.read_exact(&mut body).expect("read request body");
            }
            let body = String::from_utf8(body).expect("utf8 request body");

            let (content_type, response_body) = if request_line.starts_with("GET / ") {
                (
                    "text/html; charset=utf-8",
                    r#"<!DOCTYPE html><html><head><meta name="csrf-token" content="test-token" /></head></html>"#
                        .to_string(),
                )
            } else if request_line.starts_with("POST /graphql ") {
                let payload: Value = serde_json::from_str(&body).expect("parse graphql payload");
                let variables = payload
                    .get("variables")
                    .expect("graphql variables")
                    .as_object()
                    .expect("variables object");
                let slug = variables
                    .get("slug")
                    .and_then(Value::as_str)
                    .expect("tag slug");
                let page = variables.get("page").and_then(Value::as_u64).unwrap_or(1);
                assert_eq!(
                    cookie_header.as_deref(),
                    Some("tagger_session=test-session"),
                    "graphql request should include the session cookie from the bootstrap GET"
                );

                (
                    "application/json",
                    match (slug, page) {
                    ("burn", 1) => serde_json::json!({
                        "data": {
                            "tagBySlug": {
                                "slug": "burn",
                                "taggings": {
                                    "page": 1,
                                    "perPage": 75,
                                    "total": 2,
                                    "results": [
                                        { "subjectName": "Lightning Bolt", "card": { "name": "Lightning Bolt" } },
                                        { "subjectName": "Missing Card", "card": { "name": "Missing Card" } }
                                    ]
                                }
                            }
                        }
                    })
                    .to_string(),
                    ("removal", 1) => serde_json::json!({
                        "data": {
                            "tagBySlug": {
                                "slug": "removal",
                                "taggings": {
                                    "page": 1,
                                    "perPage": 75,
                                    "total": 76,
                                    "results": [
                                        { "subjectName": "Abrade", "card": { "name": "Abrade" } }
                                    ]
                                }
                            }
                        }
                    })
                    .to_string(),
                    ("removal", 2) => serde_json::json!({
                        "data": {
                            "tagBySlug": {
                                "slug": "removal",
                                "taggings": {
                                    "page": 2,
                                    "perPage": 75,
                                    "total": 76,
                                    "results": [
                                        { "subjectName": "Abrade", "card": { "name": "Abrade" } }
                                    ]
                                }
                            }
                        }
                    })
                    .to_string(),
                    ("missing-tag", 1) => serde_json::json!({
                        "errors": [
                            { "message": "record not found" }
                        ]
                    })
                    .to_string(),
                    _ => panic!("unexpected graphql request for slug={slug} page={page}"),
                },
                )
            } else {
                panic!("unexpected request line: {request_line}");
            };

            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n{}\r\n{}",
                content_type,
                response_body.len(),
                if request_line.starts_with("GET / ") {
                    "Set-Cookie: tagger_session=test-session; Path=/\r\n"
                } else {
                    ""
                },
                response_body
            )
            .expect("write response");
        }
    });

    format!("http://{addr}")
}

#[test]
fn sync_card_status_db_writes_rows_by_default() {
    let dir = tempdir().expect("tempdir");
    let cards_path = dir.path().join("cards.json");
    let db_path = dir.path().join("engine-status.sqlite3");
    write_cards_json(&cards_path);

    sync_registry_db(&cards_path, &db_path);

    let status = Command::new(env!("CARGO_BIN_EXE_sync_card_status_db"))
        .arg("--db-path")
        .arg(&db_path)
        .status()
        .expect("run sync_card_status_db");
    assert!(status.success(), "sync_card_status_db should succeed");

    assert_eq!(
        query_count(&db_path, "SELECT COUNT(*) FROM card_compilation"),
        2
    );
    assert_eq!(
        query_count(&db_path, "SELECT COUNT(*) FROM latest_card_compilation"),
        2
    );
}

#[test]
fn sync_registry_db_writes_registry_rows_by_default() {
    let dir = tempdir().expect("tempdir");
    let cards_path = dir.path().join("cards.json");
    let db_path = dir.path().join("engine-status.sqlite3");
    write_cards_json(&cards_path);

    sync_registry_db(&cards_path, &db_path);

    assert_eq!(
        query_count(&db_path, "SELECT COUNT(*) FROM registry_card"),
        2
    );
}

#[test]
fn sync_registry_db_includes_pioneer_legal_cards() {
    let dir = tempdir().expect("tempdir");
    let cards_path = dir.path().join("cards.json");
    let db_path = dir.path().join("engine-status.sqlite3");

    fs::write(
        &cards_path,
        r#"[
  {
    "name":"Pioneer Fixture",
    "oracle_text":"Draw a card.",
    "mana_cost":"{U}",
    "type_line":"Sorcery",
    "legalities":{
      "standard":"not_legal",
      "modern":"not_legal",
      "pioneer":"legal",
      "legacy":"not_legal",
      "vintage":"not_legal",
      "commander":"not_legal"
    }
  },
  {
    "name":"Contract from Below",
    "oracle_text":"Discard your hand, ante the top card of your library, then draw seven cards.",
    "mana_cost":"{B}",
    "type_line":"Sorcery",
    "legalities":{
      "standard":"not_legal",
      "modern":"not_legal",
      "pioneer":"not_legal",
      "legacy":"not_legal",
      "vintage":"not_legal",
      "commander":"not_legal"
    }
  }
]"#,
    )
    .expect("write pioneer fixture cards.json");

    sync_registry_db(&cards_path, &db_path);

    assert_eq!(
        query_count(&db_path, "SELECT COUNT(*) FROM registry_card"),
        1
    );
    assert_eq!(
        query_count(
            &db_path,
            "SELECT COUNT(*) FROM registry_card WHERE card_name = 'Pioneer Fixture'"
        ),
        1
    );
}

#[test]
fn sync_registry_db_can_explicitly_include_named_cards_outside_supported_legalities() {
    let dir = tempdir().expect("tempdir");
    let cards_path = dir.path().join("cards.json");
    let db_path = dir.path().join("engine-status.sqlite3");

    fs::write(
        &cards_path,
        r#"[
  {
    "name":"Lightning Bolt",
    "oracle_text":"Lightning Bolt deals 3 damage to any target.",
    "mana_cost":"{R}",
    "type_line":"Instant",
    "legalities":{
      "standard":"not_legal",
      "modern":"legal",
      "pioneer":"not_legal",
      "legacy":"legal",
      "vintage":"legal",
      "commander":"legal"
    }
  },
  {
    "name":"Mox Sapphire",
    "oracle_text":"{T}: Add {U}.",
    "mana_cost":"{0}",
    "type_line":"Artifact",
    "legalities":{
      "standard":"not_legal",
      "modern":"not_legal",
      "pioneer":"not_legal",
      "legacy":"not_legal",
      "vintage":"restricted",
      "commander":"not_legal"
    }
  },
  {
    "name":"Contract from Below",
    "oracle_text":"Discard your hand, ante the top card of your library, then draw seven cards.",
    "mana_cost":"{B}",
    "type_line":"Sorcery",
    "legalities":{
      "standard":"not_legal",
      "modern":"not_legal",
      "pioneer":"not_legal",
      "legacy":"not_legal",
      "vintage":"not_legal",
      "commander":"not_legal"
    }
  }
]"#,
    )
    .expect("write explicit include fixture cards.json");

    sync_registry_db_with_args(&cards_path, &db_path, &["--include-card", "Mox Sapphire"]);

    assert_eq!(
        query_count(&db_path, "SELECT COUNT(*) FROM registry_card"),
        2
    );
    assert_eq!(
        query_count(
            &db_path,
            "SELECT COUNT(*) FROM registry_card WHERE card_name = 'Mox Sapphire'"
        ),
        1
    );
    assert_eq!(
        query_count(
            &db_path,
            "SELECT COUNT(*) FROM registry_card WHERE card_name = 'Contract from Below'"
        ),
        0
    );
}

#[test]
fn sync_registry_db_can_register_extra_card_json_records() {
    let dir = tempdir().expect("tempdir");
    let cards_path = dir.path().join("cards.json");
    let db_path = dir.path().join("engine-status.sqlite3");
    write_cards_json(&cards_path);

    sync_registry_db_with_args(
        &cards_path,
        &db_path,
        &[
            "--extra-card-json",
            r#"{"name":"Mox Sapphire","oracle_text":"{T}: Add {U}.","mana_cost":"{0}","type_line":"Artifact","legalities":{"standard":"not_legal","modern":"not_legal","pioneer":"not_legal","legacy":"not_legal","vintage":"restricted","commander":"not_legal"}}"#,
        ],
    );

    assert_eq!(
        query_count(
            &db_path,
            "SELECT COUNT(*) FROM registry_card WHERE card_name = 'Mox Sapphire'"
        ),
        1
    );
    assert_eq!(
        query_count(&db_path, "SELECT COUNT(*) FROM registry_card"),
        3
    );
}

#[test]
fn sync_bins_strip_parenthetical_text_from_stored_oracle_and_compiled_columns() {
    let dir = tempdir().expect("tempdir");
    let cards_path = dir.path().join("cards.json");
    let db_path = dir.path().join("engine-status.sqlite3");
    write_cards_with_parenthetical_json(&cards_path);

    sync_registry_db(&cards_path, &db_path);

    let status = Command::new(env!("CARGO_BIN_EXE_sync_card_status_db"))
        .arg("--db-path")
        .arg(&db_path)
        .status()
        .expect("run sync_card_status_db");
    assert!(status.success(), "sync_card_status_db should succeed");

    let conn = Connection::open(&db_path).expect("open sqlite db");
    let (registry_oracle, registry_parse_input): (String, String) = conn
        .query_row(
            "SELECT oracle_text, parse_input FROM registry_card WHERE card_name = 'Sky Drake'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("registry sky drake row");
    let (compiled_oracle, normalized_oracle_text, compiled_text): (String, String, String) = conn
        .query_row(
            "SELECT oracle_text, normalized_oracle_text, compiled_text FROM latest_card_compilation WHERE card_name = 'Sky Drake'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("compiled sky drake row");

    assert_eq!(registry_oracle, "Flying");
    assert_eq!(compiled_oracle, "Flying");
    assert_eq!(normalized_oracle_text, "Flying");
    assert_eq!(compiled_text, "Flying");
    assert!(registry_parse_input.contains(
        "Flying (This creature can't be blocked except by creatures with flying or reach.)"
    ));
}

#[test]
fn sync_card_status_db_configures_worker_stack_for_deep_compile_paths() {
    let dir = tempdir().expect("tempdir");
    let cards_path = dir.path().join("cards.json");
    let db_path = dir.path().join("engine-status.sqlite3");
    write_cards_with_worker_stack_pressure_json(&cards_path);

    let output = Command::new(env!("CARGO_BIN_EXE_sync_card_status_db"))
        .arg("--cards")
        .arg(&cards_path)
        .arg("--db-path")
        .arg(&db_path)
        .env_remove("RUST_MIN_STACK")
        .output()
        .expect("run sync_card_status_db");
    assert!(
        output.status.success(),
        "sync_card_status_db should configure enough worker stack for deep compile paths; stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        query_count(&db_path, "SELECT COUNT(*) FROM latest_card_compilation"),
        2
    );
    let stdout = String::from_utf8(output.stdout).expect("sync stdout utf8");
    assert!(
        stdout.contains("Rayon worker stack: 16777216 bytes"),
        "expected sync output to report configured worker stack, got {stdout}"
    );
}

#[test]
fn sync_card_status_db_prunes_cards_without_supported_format_legality() {
    let dir = tempdir().expect("tempdir");
    let cards_path = dir.path().join("cards.json");
    let db_path = dir.path().join("engine-status.sqlite3");

    fs::write(
        &cards_path,
        r#"[
  {
    "name":"Lightning Bolt",
    "oracle_text":"Lightning Bolt deals 3 damage to any target.",
    "mana_cost":"{R}",
    "type_line":"Instant",
    "legalities":{
      "standard":"not_legal",
      "modern":"legal",
      "legacy":"legal",
      "vintage":"legal",
      "commander":"legal"
    }
  },
  {
    "name":"Abrade",
    "oracle_text":"Choose one — Abrade deals 3 damage to target creature; or destroy target artifact.",
    "mana_cost":"{1}{R}",
    "type_line":"Instant",
    "legalities":{
      "standard":"not_legal",
      "modern":"legal",
      "legacy":"legal",
      "vintage":"legal",
      "commander":"legal"
    }
  }
]"#,
    )
    .expect("write initial cards.json");

    let status = Command::new(env!("CARGO_BIN_EXE_sync_card_status_db"))
        .arg("--cards")
        .arg(&cards_path)
        .arg("--db-path")
        .arg(&db_path)
        .status()
        .expect("run initial sync_card_status_db");
    assert!(
        status.success(),
        "initial sync_card_status_db should succeed"
    );
    assert_eq!(
        query_count(&db_path, "SELECT COUNT(*) FROM latest_card_compilation"),
        2
    );

    fs::write(
        &cards_path,
        r#"[
  {
    "name":"Lightning Bolt",
    "oracle_text":"Lightning Bolt deals 3 damage to any target.",
    "mana_cost":"{R}",
    "type_line":"Instant",
    "legalities":{
      "standard":"not_legal",
      "modern":"legal",
      "legacy":"legal",
      "vintage":"legal",
      "commander":"legal"
    }
  },
  {
    "name":"Contract from Below",
    "oracle_text":"Discard your hand, ante the top card of your library, then draw seven cards.",
    "mana_cost":"{B}",
    "type_line":"Sorcery",
    "legalities":{
      "standard":"not_legal",
      "modern":"not_legal",
      "legacy":"not_legal",
      "vintage":"not_legal",
      "commander":"not_legal"
    }
  }
]"#,
    )
    .expect("write updated cards.json");

    let status = Command::new(env!("CARGO_BIN_EXE_sync_card_status_db"))
        .arg("--cards")
        .arg(&cards_path)
        .arg("--db-path")
        .arg(&db_path)
        .status()
        .expect("run second sync_card_status_db");
    assert!(
        status.success(),
        "second sync_card_status_db should succeed"
    );

    assert_eq!(
        query_count(&db_path, "SELECT COUNT(*) FROM latest_card_compilation"),
        1
    );
    assert_eq!(
        query_count(
            &db_path,
            "SELECT COUNT(*) FROM latest_card_compilation WHERE card_name = 'Lightning Bolt'"
        ),
        1
    );
    assert_eq!(
        query_count(
            &db_path,
            "SELECT COUNT(*) FROM latest_card_compilation WHERE card_name IN ('Abrade', 'Contract from Below')"
        ),
        0
    );
}

#[test]
fn sync_card_status_db_supports_tag_filtered_recompile_without_pruning() {
    let dir = tempdir().expect("tempdir");
    let cards_path = dir.path().join("cards.json");
    let db_path = dir.path().join("engine-status.sqlite3");
    write_cards_with_abrade_json(&cards_path);

    {
        let conn = Connection::open(&db_path).expect("open sqlite db");
        conn.execute(
            "CREATE TABLE card_tagging (card_name TEXT NOT NULL, tag TEXT NOT NULL, UNIQUE(card_name, tag))",
            [],
        )
        .expect("create card_tagging");
        conn.execute(
            "INSERT INTO card_tagging(card_name, tag) VALUES ('Lightning Bolt', 'burn'), ('Abrade', 'removal')",
            [],
        )
        .expect("seed card_tagging");
    }

    let status = Command::new(env!("CARGO_BIN_EXE_sync_card_status_db"))
        .arg("--cards")
        .arg(&cards_path)
        .arg("--db-path")
        .arg(&db_path)
        .status()
        .expect("run initial sync_card_status_db");
    assert!(
        status.success(),
        "initial sync_card_status_db should succeed"
    );
    assert_eq!(
        query_count(&db_path, "SELECT COUNT(*) FROM latest_card_compilation"),
        2
    );

    fs::write(
        &cards_path,
        r#"[
  {
    "name":"Lightning Bolt",
    "oracle_text":"Lightning Bolt deals 4 damage to any target.",
    "mana_cost":"{R}",
    "type_line":"Instant"
  },
  {
    "name":"Abrade",
    "oracle_text":"Choose one — Abrade deals 3 damage to target creature; or destroy target artifact.",
    "mana_cost":"{1}{R}",
    "type_line":"Instant"
  }
]"#,
    )
    .expect("write updated cards.json");

    let status = Command::new(env!("CARGO_BIN_EXE_sync_card_status_db"))
        .arg("--cards")
        .arg(&cards_path)
        .arg("--db-path")
        .arg(&db_path)
        .arg("--tag")
        .arg("burn")
        .status()
        .expect("run tag-filtered sync_card_status_db");
    assert!(
        status.success(),
        "tag-filtered sync_card_status_db should succeed"
    );

    assert_eq!(
        query_count(&db_path, "SELECT COUNT(*) FROM card_compilation"),
        3
    );
    assert_eq!(
        query_count(&db_path, "SELECT COUNT(*) FROM latest_card_compilation"),
        2
    );

    let conn = Connection::open(&db_path).expect("open sqlite db");
    let lightning_oracle: String = conn
        .query_row(
            "SELECT oracle_text FROM latest_card_compilation WHERE card_name = 'Lightning Bolt'",
            [],
            |row| row.get(0),
        )
        .expect("latest lightning bolt oracle text");
    let abrade_oracle: String = conn
        .query_row(
            "SELECT oracle_text FROM latest_card_compilation WHERE card_name = 'Abrade'",
            [],
            |row| row.get(0),
        )
        .expect("latest abrade oracle text");

    assert_eq!(
        lightning_oracle,
        "Lightning Bolt deals 4 damage to any target."
    );
    assert_eq!(
        abrade_oracle,
        "Choose one — Abrade deals 3 damage to target creature; or destroy target artifact."
    );
}

#[test]
fn sync_card_status_db_reports_strict_compiled_score_summary() {
    let dir = tempdir().expect("tempdir");
    let cards_path = dir.path().join("cards.json");
    let db_path = dir.path().join("engine-status.sqlite3");
    write_cards_json(&cards_path);

    let lightning = make_payload(
        "Lightning Bolt",
        "Lightning Bolt deals 3 damage to any target.",
        "{R}",
        "Instant",
    );
    let counterspell = make_payload("Counterspell", "Counter target spell.", "{U}{U}", "Instant");

    let db = CardStatusDb::open(&db_path).expect("open sqlite db");

    let mut lightning_snapshot = compile_snapshot_from_payload(&lightning);
    lightning_snapshot.similarity_score = 0.4;
    lightning_snapshot.content_hash = "lightning-bolt-seeded-score".to_string();
    db.insert_snapshot_if_changed(&lightning_snapshot)
        .expect("seed lightning snapshot");

    let mut counterspell_snapshot = compile_snapshot_from_payload(&counterspell);
    counterspell_snapshot.similarity_score = 1.2;
    counterspell_snapshot.content_hash = "counterspell-seeded-score".to_string();
    db.insert_snapshot_if_changed(&counterspell_snapshot)
        .expect("seed counterspell snapshot");

    let output = Command::new(env!("CARGO_BIN_EXE_sync_card_status_db"))
        .arg("--cards")
        .arg(&cards_path)
        .arg("--db-path")
        .arg(&db_path)
        .output()
        .expect("run sync_card_status_db");
    assert!(
        output.status.success(),
        "sync_card_status_db should succeed"
    );

    let stdout =
        String::from_utf8(output.stdout).expect("sync_card_status_db stdout should be utf8");
    assert!(
        stdout.contains("- Strict-compiled semantic score avg before: 0.8000 across 2 cards"),
        "expected strict-compiled before average in output, got {stdout}"
    );
    assert!(
        stdout.contains("- Strict-compiled semantic score avg after: 1.0000 across 2 cards"),
        "expected strict-compiled after average in output, got {stdout}"
    );
    assert!(
        stdout.contains("- Cards with increased strict-compiled score: 1 (avg +0.6000)"),
        "expected increased-score summary in output, got {stdout}"
    );
    assert!(
        stdout.contains("- Cards with decreased strict-compiled score: 1 (avg -0.2000)"),
        "expected decreased-score summary in output, got {stdout}"
    );
}

#[test]
fn compile_oracle_text_never_interacts_with_status_db() {
    let dir = tempdir().expect("tempdir");
    let cards_path = dir.path().join("cards.json");
    let default_db_path = dir.path().join("reports").join("engine-status.sqlite3");
    write_cards_json(&cards_path);

    let status = Command::new(env!("CARGO_BIN_EXE_compile_oracle_text"))
        .current_dir(dir.path())
        .arg("--name")
        .arg("Lightning Bolt")
        .arg("--cards")
        .arg(&cards_path)
        .status()
        .expect("run authoritative compile_oracle_text");
    assert!(
        status.success(),
        "authoritative compile_oracle_text should succeed"
    );
    assert!(
        !default_db_path.exists(),
        "compile_oracle_text should not create the default status DB"
    );

    let status = Command::new(env!("CARGO_BIN_EXE_compile_oracle_text"))
        .current_dir(dir.path())
        .arg("--name")
        .arg("Lightning Bolt")
        .arg("--cards")
        .arg(&cards_path)
        .arg("--text")
        .arg("Mana cost: {R}\nType: Instant\nLightning Bolt deals 3 damage to any target.")
        .status()
        .expect("run ad hoc compile_oracle_text");
    assert!(
        status.success(),
        "ad hoc compile_oracle_text should succeed"
    );
    assert!(
        !default_db_path.exists(),
        "ad hoc compile_oracle_text should not create the default status DB"
    );
}

#[test]
fn compile_oracle_text_compare_text_outputs_only_text_and_score() {
    let dir = tempdir().expect("tempdir");
    let cards_path = dir.path().join("cards.json");
    let names_path = dir.path().join("names.txt");
    write_cards_json(&cards_path);
    fs::write(&names_path, "Lightning Bolt\nCounterspell\n").expect("write names file");

    let output = Command::new(env!("CARGO_BIN_EXE_compile_oracle_text"))
        .current_dir(dir.path())
        .arg("--names")
        .arg(&names_path)
        .arg("--cards")
        .arg(&cards_path)
        .arg("--compare-text")
        .output()
        .expect("run compile_oracle_text --compare-text");
    assert!(
        output.status.success(),
        "compare-text compile_oracle_text should succeed"
    );

    let stdout =
        String::from_utf8(output.stdout).expect("compile_oracle_text stdout should be utf8");
    assert!(stdout.contains("Name: Lightning Bolt"), "{stdout}");
    assert!(stdout.contains("Name: Counterspell"), "{stdout}");
    assert!(stdout.contains("Similarity:"), "{stdout}");
    assert!(stdout.contains("Semantic mismatch:"), "{stdout}");
    assert!(stdout.contains("Original oracle text:"), "{stdout}");
    assert!(stdout.contains("Compiled oracle text:"), "{stdout}");
    assert!(
        !stdout.contains("Compiled abilities/effects")
            && !stdout.contains("Compiled card definition:")
            && !stdout.contains("Type:"),
        "compare-text mode should not print full definitions, got {stdout}"
    );
}

#[test]
fn compile_oracle_text_strictly_compiles_aunt_may_from_workspace_cards() {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("ironsmith-tools crate should be inside workspace")
        .parent()
        .expect("workspace root should be two levels up");
    let cards_path = workspace_root.join("cards.json");
    assert!(cards_path.exists(), "expected workspace cards.json at {cards_path:?}");

    let output = Command::new(env!("CARGO_BIN_EXE_compile_oracle_text"))
        .arg("--name")
        .arg("Aunt May")
        .arg("--cards")
        .arg(&cards_path)
        .arg("--compare-text")
        .output()
        .expect("run compile_oracle_text --name Aunt May --compare-text");

    assert!(
        output.status.success(),
        "Aunt May should compile strictly, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout =
        String::from_utf8(output.stdout).expect("compile_oracle_text stdout should be utf8");
    assert!(stdout.contains("Name: Aunt May"), "{stdout}");
    assert!(
        stdout.contains("If it's a Spider, put a +1/+1 counter on it."),
        "expected Spider conditional clause in compiled comparison output, got {stdout}"
    );
}

#[test]
fn compile_oracle_text_strictly_compiles_sakashimas_will_with_choose_both_instead_clause() {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("ironsmith-tools crate should be inside workspace")
        .parent()
        .expect("workspace root should be two levels up");
    let cards_path = workspace_root.join("cards.json");
    assert!(cards_path.exists(), "expected workspace cards.json at {cards_path:?}");

    let output = Command::new(env!("CARGO_BIN_EXE_compile_oracle_text"))
        .arg("--name")
        .arg("Sakashima's Will")
        .arg("--cards")
        .arg(&cards_path)
        .arg("--compare-text")
        .output()
        .expect("run compile_oracle_text --name Sakashima's Will --compare-text");

    assert!(
        output.status.success(),
        "Sakashima's Will should compile strictly, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout =
        String::from_utf8(output.stdout).expect("compile_oracle_text stdout should be utf8");
    assert!(stdout.contains("Name: Sakashima's Will"), "{stdout}");
    assert!(
        stdout.contains("you may choose both instead."),
        "expected choose-both replacement clause in compiled comparison output, got {stdout}"
    );
    assert!(
        stdout.contains("Target opponent chooses a creature they control. You gain control of it."),
        "expected opponent-choice branch in compiled comparison output, got {stdout}"
    );
}

#[test]
fn compile_oracle_text_outputs_original_oracle_text() {
    let dir = tempdir().expect("tempdir");
    let cards_path = dir.path().join("cards.json");
    write_cards_json(&cards_path);

    let output = Command::new(env!("CARGO_BIN_EXE_compile_oracle_text"))
        .current_dir(dir.path())
        .arg("--name")
        .arg("Lightning Bolt")
        .arg("--cards")
        .arg(&cards_path)
        .output()
        .expect("run compile_oracle_text");
    assert!(
        output.status.success(),
        "compile_oracle_text should succeed"
    );

    let stdout =
        String::from_utf8(output.stdout).expect("compile_oracle_text stdout should be utf8");
    assert!(stdout.contains("Original oracle text:"), "{stdout}");
    assert!(
        stdout.contains("Lightning Bolt deals 3 damage to any target."),
        "{stdout}"
    );
    assert!(stdout.contains("Compiled abilities/effects"), "{stdout}");
}

#[test]
fn compile_oracle_text_rejects_obsolete_db_flags() {
    let dir = tempdir().expect("tempdir");
    let cards_path = dir.path().join("cards.json");
    write_cards_json(&cards_path);

    let output = Command::new(env!("CARGO_BIN_EXE_compile_oracle_text"))
        .arg("--name")
        .arg("Lightning Bolt")
        .arg("--cards")
        .arg(&cards_path)
        .arg("--db-path")
        .arg(dir.path().join("obsolete.sqlite3"))
        .output()
        .expect("run compile_oracle_text with obsolete --db-path");
    assert!(
        !output.status.success(),
        "compile_oracle_text should reject --db-path"
    );

    let stderr =
        String::from_utf8(output.stderr).expect("compile_oracle_text stderr should be utf8");
    assert!(
        stderr.contains("unknown argument '--db-path'"),
        "expected obsolete flag error, got {stderr}"
    );
}

#[test]
fn compile_oracle_text_uses_builtin_linked_face_metadata_for_transform_pairs() {
    let dir = tempdir().expect("tempdir");
    let cards_path = dir.path().join("cards.json");
    write_cards_json(&cards_path);

    let output = Command::new(env!("CARGO_BIN_EXE_compile_oracle_text"))
        .arg("--name")
        .arg("Conqueror's Galleon // Conqueror's Foothold")
        .arg("--cards")
        .arg(&cards_path)
        .output()
        .expect("run compile_oracle_text");
    assert!(
        output.status.success(),
        "compile_oracle_text should succeed for builtin transform pairs"
    );

    let stdout =
        String::from_utf8(output.stdout).expect("compile_oracle_text stdout should be utf8");
    assert!(
        stdout.contains("other_face: Some"),
        "expected builtin linked-face metadata in output, got {stdout}"
    );
    assert!(
        stdout.contains("linked_face_layout: TransformLike"),
        "expected transform-like layout in output, got {stdout}"
    );
}

#[test]
fn compile_oracle_text_trace_is_card_aware_instead_of_parser_firehose() {
    let dir = tempdir().expect("tempdir");
    let cards_path = dir.path().join("cards.json");
    write_cards_json(&cards_path);

    let output = Command::new(env!("CARGO_BIN_EXE_compile_oracle_text"))
        .arg("--name")
        .arg("Lightning Bolt")
        .arg("--cards")
        .arg(&cards_path)
        .arg("--trace")
        .output()
        .expect("run compile_oracle_text --trace");
    assert!(
        output.status.success(),
        "compile_oracle_text --trace should succeed"
    );

    let stdout =
        String::from_utf8(output.stdout).expect("compile_oracle_text stdout should be utf8");
    let stderr =
        String::from_utf8(output.stderr).expect("compile_oracle_text stderr should be utf8");

    assert!(
        stdout.contains("Compiled abilities/effects"),
        "expected normal compile output on stdout, got {stdout}"
    );
    assert!(
        stderr.contains("Trace: Lightning Bolt"),
        "expected trace heading in stderr, got {stderr}"
    );
    assert!(
        stderr.contains("line 3 parse"),
        "expected line-level parse trace in stderr, got {stderr}"
    );
    assert!(
        stderr.contains("effect sentence"),
        "expected effect sentence trace in stderr, got {stderr}"
    );
    assert!(
        !stderr.contains("OwnedLexToken")
            && !stderr.contains("backtrack")
            && !stderr.contains("> punct"),
        "trace should not expose the low-level parser firehose, got {stderr}"
    );
    assert!(
        stderr.lines().count() < 160,
        "trace should stay compact for a simple card, got {} lines:\n{stderr}",
        stderr.lines().count()
    );
}

#[test]
fn compile_oracle_text_does_not_write_parse_failed_snapshot_for_authoritative_card() {
    let dir = tempdir().expect("tempdir");
    let cards_path = dir.path().join("cards.json");
    let default_db_path = dir.path().join("reports").join("engine-status.sqlite3");
    write_cards_with_unsupported_json(&cards_path);

    let status = Command::new(env!("CARGO_BIN_EXE_compile_oracle_text"))
        .current_dir(dir.path())
        .arg("--name")
        .arg("Unsupported Fixture")
        .arg("--cards")
        .arg(&cards_path)
        .status()
        .expect("run authoritative compile_oracle_text on unsupported card");
    assert!(
        !status.success(),
        "compile_oracle_text should still fail for unsupported authoritative cards"
    );
    assert!(
        !default_db_path.exists(),
        "compile_oracle_text should not write parse failure snapshots"
    );
}

#[test]
fn sync_card_status_db_records_semantic_mismatch_status() {
    let dir = tempdir().expect("tempdir");
    let cards_path = dir.path().join("cards.json");
    let db_path = dir.path().join("authoritative.sqlite3");
    write_cards_with_semantic_mismatch_json(&cards_path);

    let status = Command::new(env!("CARGO_BIN_EXE_sync_card_status_db"))
        .arg("--cards")
        .arg(&cards_path)
        .arg("--db-path")
        .arg(&db_path)
        .status()
        .expect("run sync_card_status_db on semantic mismatch card");
    assert!(
        status.success(),
        "sync_card_status_db should succeed for compiled semantic mismatches"
    );

    let conn = Connection::open(&db_path).expect("open sqlite db");
    let (parse_status, semantic_mismatch, parse_error, compiled_text): (
        String,
        i64,
        Option<String>,
        Option<String>,
    ) = conn
        .query_row(
            "SELECT parse_status, semantic_mismatch, parse_error, compiled_text
             FROM latest_card_compilation
             WHERE card_name = 'Mismatch Fixture'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("query semantic mismatch snapshot");

    assert_eq!(parse_status, "strict_compiled");
    assert_eq!(semantic_mismatch, 1);
    assert!(parse_error.is_none());
    assert!(
        compiled_text.is_some(),
        "semantic mismatch snapshots should keep compiled text"
    );
}

#[test]
fn import_card_tags_replaces_rows_for_imported_tags() {
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("engine-status.sqlite3");
    let first_csv = dir.path().join("first.csv");
    let second_csv = dir.path().join("second.csv");

    fs::write(
        &first_csv,
        "name,matched_tags,local_card_found,local_card_name,semantic_score,parse_status,parse_error,compiled_text,oracle_text,scryfall_uri\nLightning Bolt,burn,yes,Lightning Bolt,1.0,compiled,,,Lightning Bolt deals 3 damage to any target.,https://example.com/lb\nChain Lightning,burn,yes,Chain Lightning,1.0,compiled,,,Chain Lightning deals 3 damage to any target.,https://example.com/cl\n",
    )
    .expect("write first tag csv");
    fs::write(
        &second_csv,
        "name,matched_tags,local_card_found,local_card_name,semantic_score,parse_status,parse_error,compiled_text,oracle_text,scryfall_uri\nLightning Bolt,burn,yes,Lightning Bolt,1.0,compiled,,,Lightning Bolt deals 3 damage to any target.,https://example.com/lb\nLightning Bolt,burn,yes,Lightning Bolt,1.0,compiled,,,Lightning Bolt deals 3 damage to any target.,https://example.com/lb-dup\n",
    )
    .expect("write second tag csv");

    let status = Command::new(env!("CARGO_BIN_EXE_import_card_tags"))
        .arg("--db-path")
        .arg(&db_path)
        .arg("--csv")
        .arg(&first_csv)
        .status()
        .expect("run first import");
    assert!(status.success(), "first tag import should succeed");
    assert_eq!(
        query_count(&db_path, "SELECT COUNT(*) FROM card_tagging"),
        2
    );

    let status = Command::new(env!("CARGO_BIN_EXE_import_card_tags"))
        .arg("--db-path")
        .arg(&db_path)
        .arg("--csv")
        .arg(&second_csv)
        .status()
        .expect("run second import");
    assert!(status.success(), "second tag import should succeed");
    assert_eq!(
        query_count(&db_path, "SELECT COUNT(*) FROM card_tagging"),
        1
    );
    assert_eq!(
        query_count(
            &db_path,
            "SELECT COUNT(*) FROM card_tagging WHERE tag = 'burn' AND card_name = 'Lightning Bolt'"
        ),
        1
    );
}

#[test]
fn sync_oracle_tags_replaces_functional_tag_catalog() {
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("engine-status.sqlite3");
    let html_path = dir.path().join("tagger-tags.html");

    fs::write(
        &html_path,
        r#"
            <h2>#</h2>
            <p><a href="/search?q=art%3Abolt&amp;unique=art">bolt</a></p>
            <h2># (functional)</h2>
            <p><a href="/search?q=oracletag%3Aburn">burn</a></p>
            <h2>A (functional)</h2>
            <p>
                <a href="/search?q=function%3Aanthem">anthem</a>
                <a href="/search?q=oracletag%3Aremoval">removal</a>
            </p>
        "#,
    )
    .expect("write tagger tags html");

    let status = Command::new(env!("CARGO_BIN_EXE_sync_oracle_tags"))
        .arg("--db-path")
        .arg(&db_path)
        .arg("--html")
        .arg(&html_path)
        .status()
        .expect("run sync_oracle_tags");
    assert!(status.success(), "sync_oracle_tags should succeed");

    assert_eq!(query_count(&db_path, "SELECT COUNT(*) FROM oracle_tag"), 3);
    assert_eq!(
        query_count(
            &db_path,
            "SELECT COUNT(*) FROM oracle_tag WHERE tag IN ('anthem', 'burn', 'removal')"
        ),
        3
    );
}

#[test]
fn sync_card_tagging_uses_tagger_membership_and_filters_to_local_cards() {
    let dir = tempdir().expect("tempdir");
    let cards_path = dir.path().join("cards.json");
    let db_path = dir.path().join("engine-status.sqlite3");
    write_cards_with_abrade_json(&cards_path);

    {
        let conn = Connection::open(&db_path).expect("open sqlite db");
        conn.execute("CREATE TABLE oracle_tag (tag TEXT PRIMARY KEY)", [])
            .expect("create oracle_tag");
        conn.execute("CREATE TABLE card_tagging (card_name TEXT NOT NULL, tag TEXT NOT NULL, UNIQUE(card_name, tag))", [])
            .expect("create card_tagging");
        conn.execute(
            "INSERT INTO oracle_tag(tag) VALUES ('burn'), ('removal')",
            [],
        )
        .expect("seed oracle tags");
    }

    let tagger_url = spawn_mock_tagger_server();
    let status = Command::new(env!("CARGO_BIN_EXE_sync_card_tagging"))
        .arg("--cards")
        .arg(&cards_path)
        .arg("--db-path")
        .arg(&db_path)
        .arg("--tagger-url")
        .arg(&tagger_url)
        .status()
        .expect("run sync_card_tagging");
    assert!(status.success(), "sync_card_tagging should succeed");

    assert_eq!(
        query_count(&db_path, "SELECT COUNT(*) FROM card_tagging"),
        2
    );
    assert_eq!(
        query_count(
            &db_path,
            "SELECT COUNT(*) FROM card_tagging WHERE tag = 'burn' AND card_name = 'Lightning Bolt'"
        ),
        1
    );
    assert_eq!(
        query_count(
            &db_path,
            "SELECT COUNT(*) FROM card_tagging WHERE tag = 'removal' AND card_name = 'Abrade'"
        ),
        1
    );
}

#[test]
fn sync_card_tagging_supports_start_position() {
    let dir = tempdir().expect("tempdir");
    let cards_path = dir.path().join("cards.json");
    let db_path = dir.path().join("engine-status.sqlite3");
    write_cards_with_abrade_json(&cards_path);

    {
        let conn = Connection::open(&db_path).expect("open sqlite db");
        conn.execute("CREATE TABLE oracle_tag (tag TEXT PRIMARY KEY)", [])
            .expect("create oracle_tag");
        conn.execute("CREATE TABLE card_tagging (card_name TEXT NOT NULL, tag TEXT NOT NULL, UNIQUE(card_name, tag))", [])
            .expect("create card_tagging");
        conn.execute(
            "INSERT INTO oracle_tag(tag) VALUES ('burn'), ('removal')",
            [],
        )
        .expect("seed oracle tags");
    }

    let tagger_url = spawn_mock_tagger_server();
    let status = Command::new(env!("CARGO_BIN_EXE_sync_card_tagging"))
        .arg("--cards")
        .arg(&cards_path)
        .arg("--db-path")
        .arg(&db_path)
        .arg("--tagger-url")
        .arg(&tagger_url)
        .arg("--start")
        .arg("2")
        .status()
        .expect("run sync_card_tagging with --start");
    assert!(status.success(), "sync_card_tagging --start should succeed");

    assert_eq!(
        query_count(&db_path, "SELECT COUNT(*) FROM card_tagging"),
        1
    );
    assert_eq!(
        query_count(
            &db_path,
            "SELECT COUNT(*) FROM card_tagging WHERE tag = 'burn'"
        ),
        0
    );
    assert_eq!(
        query_count(
            &db_path,
            "SELECT COUNT(*) FROM card_tagging WHERE tag = 'removal' AND card_name = 'Abrade'"
        ),
        1
    );
}

#[test]
fn sync_card_tagging_skips_failed_tags_and_continues() {
    let dir = tempdir().expect("tempdir");
    let cards_path = dir.path().join("cards.json");
    let db_path = dir.path().join("engine-status.sqlite3");
    write_cards_with_abrade_json(&cards_path);

    let tagger_url = spawn_mock_tagger_server();
    let status = Command::new(env!("CARGO_BIN_EXE_sync_card_tagging"))
        .arg("--cards")
        .arg(&cards_path)
        .arg("--db-path")
        .arg(&db_path)
        .arg("--tagger-url")
        .arg(&tagger_url)
        .arg("--tag")
        .arg("missing-tag")
        .arg("--tag")
        .arg("burn")
        .status()
        .expect("run sync_card_tagging with failing tag");
    assert!(
        status.success(),
        "sync_card_tagging should continue past per-tag failures"
    );

    assert_eq!(
        query_count(&db_path, "SELECT COUNT(*) FROM card_tagging"),
        1
    );
    assert_eq!(
        query_count(
            &db_path,
            "SELECT COUNT(*) FROM card_tagging WHERE tag = 'burn' AND card_name = 'Lightning Bolt'"
        ),
        1
    );
}
