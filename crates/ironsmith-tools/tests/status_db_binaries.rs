use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::Path;
use std::process::Command;
use std::thread;

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

fn query_count(db_path: &Path, sql: &str) -> i64 {
    let conn = Connection::open(db_path).expect("open sqlite db");
    conn.query_row(sql, [], |row| row.get(0))
        .expect("query count")
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

    let status = Command::new(env!("CARGO_BIN_EXE_sync_card_status_db"))
        .arg("--cards")
        .arg(&cards_path)
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
fn compile_oracle_text_only_writes_for_authoritative_cards_and_obeys_no_db() {
    let dir = tempdir().expect("tempdir");
    let cards_path = dir.path().join("cards.json");
    let authoritative_db = dir.path().join("authoritative.sqlite3");
    let no_db_path = dir.path().join("no-db.sqlite3");
    let adhoc_db = dir.path().join("adhoc.sqlite3");
    write_cards_json(&cards_path);

    let status = Command::new(env!("CARGO_BIN_EXE_compile_oracle_text"))
        .arg("--name")
        .arg("Lightning Bolt")
        .arg("--cards")
        .arg(&cards_path)
        .arg("--db-path")
        .arg(&authoritative_db)
        .status()
        .expect("run authoritative compile_oracle_text");
    assert!(
        status.success(),
        "authoritative compile_oracle_text should succeed"
    );
    assert_eq!(
        query_count(&authoritative_db, "SELECT COUNT(*) FROM card_compilation"),
        1
    );

    let status = Command::new(env!("CARGO_BIN_EXE_compile_oracle_text"))
        .arg("--name")
        .arg("Lightning Bolt")
        .arg("--cards")
        .arg(&cards_path)
        .arg("--db-path")
        .arg(&no_db_path)
        .arg("--no-db")
        .status()
        .expect("run compile_oracle_text with --no-db");
    assert!(
        status.success(),
        "compile_oracle_text --no-db should succeed"
    );
    assert!(
        !no_db_path.exists(),
        "--no-db should prevent the database from being created"
    );

    let status = Command::new(env!("CARGO_BIN_EXE_compile_oracle_text"))
        .arg("--name")
        .arg("Lightning Bolt")
        .arg("--cards")
        .arg(&cards_path)
        .arg("--text")
        .arg("Mana cost: {R}\nType: Instant\nLightning Bolt deals 3 damage to any target.")
        .arg("--db-path")
        .arg(&adhoc_db)
        .status()
        .expect("run ad hoc compile_oracle_text");
    assert!(
        status.success(),
        "ad hoc compile_oracle_text should succeed"
    );
    assert!(
        !adhoc_db.exists(),
        "ad hoc compile_oracle_text should not create the status DB"
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
