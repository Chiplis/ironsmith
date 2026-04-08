use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::error::Error;
use std::fmt;
use std::fs;
use std::panic::{self, AssertUnwindSafe};
use std::path::{Path, PathBuf};

use csv::StringRecord;
use reqwest::blocking::Client;
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::cards::{
    CardDefinition, CardDefinitionBuilder, generated_definition_has_unimplemented_content,
};
use crate::compiled_text::canonical_compiled_lines;
use crate::ids::CardId;
use crate::semantic_compare::{compare_card_semantics_scored, report_embedding_config};

pub const DEFAULT_DB_PATH: &str = "reports/engine-status.sqlite3";
pub const SCRYFALL_TAGGER_TAGS_URL: &str = "https://scryfall.com/docs/tagger-tags";
pub const TAGGER_BASE_URL: &str = "https://tagger.scryfall.com";
const DB_SCHEMA_VERSION: i64 = 2;
const FIXED_SNAPSHOT_CARD_ID: u32 = 1;
const SUPPORTED_PAPER_FORMATS: &[&str] = &["commander", "standard", "modern", "legacy", "vintage"];
const TAGGER_FETCH_ORACLE_CARD_TAG_QUERY: &str = r#"
query FetchOracleCardTagPage($slug: String!, $type: TagType!, $page: Int) {
  tagBySlug(slug: $slug, type: $type) {
    slug
    taggings(page: $page) {
      page
      perPage
      total
      results {
        subjectName
        card {
          name
        }
      }
    }
  }
}
"#;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CardPayload {
    pub name: String,
    pub oracle_text: String,
    pub metadata_lines: Vec<String>,
    pub parse_input: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryCardRecord {
    pub payload: CardPayload,
    pub raw_card_json: String,
    pub mana_cost: Option<String>,
    pub type_line: Option<String>,
    pub power: Option<String>,
    pub toughness: Option<String>,
    pub loyalty: Option<String>,
    pub defense: Option<String>,
    pub layout: Option<String>,
    pub content_hash: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseStatus {
    StrictCompiled,
    CompiledWithAllowUnsupported,
    ParseFailed,
}

impl ParseStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::StrictCompiled => "strict_compiled",
            Self::CompiledWithAllowUnsupported => "compiled_with_allow_unsupported",
            Self::ParseFailed => "parse_failed",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ParseAttempt {
    pub status: ParseStatus,
    pub parse_error: Option<String>,
    pub definition: Option<CardDefinition>,
}

#[derive(Debug, Clone)]
pub struct CompilationSnapshot {
    pub card_name: String,
    pub oracle_text: String,
    pub parse_status: ParseStatus,
    pub parse_error: Option<String>,
    pub compiled_text: Option<String>,
    pub compiled_card_definition: Option<String>,
    pub oracle_coverage: f32,
    pub compiled_coverage: f32,
    pub similarity_score: f32,
    pub line_delta: isize,
    pub semantic_mismatch: bool,
    pub has_unimplemented: bool,
    pub content_hash: String,
}

#[derive(Debug)]
pub struct CardStatusDb {
    conn: Connection,
}

#[derive(Debug, Clone, Copy)]
pub struct TagImportSummary {
    pub tags_replaced: usize,
    pub rows_inserted: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct OracleTagSyncSummary {
    pub tags_replaced: usize,
    pub rows_inserted: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct CardPruneSummary {
    pub distinct_cards_deleted: usize,
    pub compilation_rows_deleted: usize,
    pub tag_rows_deleted: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct CompilationHistoryCleanupSummary {
    pub distinct_cards_retained: usize,
    pub compilation_rows_deleted: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct RegistrySyncSummary {
    pub inserted: usize,
    pub updated: usize,
    pub unchanged: usize,
    pub deleted: usize,
}

#[derive(Debug)]
pub struct TagImportRow {
    pub card_name: String,
    pub tag: String,
}

#[derive(Debug)]
pub struct TaggerClient {
    client: Client,
    base_url: String,
    csrf_token: String,
}

#[derive(Debug, Clone)]
pub struct TaggerTagPage {
    pub total: usize,
    pub per_page: usize,
    pub card_names: Vec<String>,
}

impl fmt::Display for ParseStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

pub fn default_db_path() -> PathBuf {
    PathBuf::from(DEFAULT_DB_PATH)
}

pub fn normalize_lookup_name(name: &str) -> String {
    let trimmed = name.trim();
    if trimmed.contains(" / ") && !trimmed.contains(" // ") {
        return trimmed.replacen(" / ", " // ", 1);
    }
    trimmed.to_string()
}

pub fn build_parse_input(metadata_lines: &[String], oracle_text: &str) -> String {
    let mut lines = metadata_lines.to_vec();
    if !oracle_text.trim().is_empty() {
        lines.push(oracle_text.trim().to_string());
    }
    lines.join("\n")
}

pub fn load_canonical_cards(path: &str) -> Result<BTreeMap<String, CardPayload>, Box<dyn Error>> {
    let raw = fs::read_to_string(path)?;
    let cards: Vec<Value> = serde_json::from_str(&raw)?;
    Ok(load_registry_cards_from_values(cards.into_iter())
        .into_iter()
        .map(|(name, record)| (name, record.payload))
        .collect())
}

pub fn load_registry_cards(
    path: &str,
) -> Result<BTreeMap<String, RegistryCardRecord>, Box<dyn Error>> {
    let raw = fs::read_to_string(path)?;
    let cards: Vec<Value> = serde_json::from_str(&raw)?;
    Ok(load_registry_cards_from_values(cards.into_iter()))
}

pub fn load_card_by_name(path: &str, name: &str) -> Result<Option<CardPayload>, Box<dyn Error>> {
    let cards = load_canonical_cards(path)?;
    let normalized = normalize_lookup_name(name);
    Ok(cards.get(&normalized).cloned())
}

pub fn parse_card_with_fallback(name: &str, parse_input: &str) -> ParseAttempt {
    match parse_card(name, parse_input, false) {
        ParseAttempt {
            status: ParseStatus::StrictCompiled,
            definition,
            ..
        } => ParseAttempt {
            status: ParseStatus::StrictCompiled,
            parse_error: None,
            definition,
        },
        strict_failure => match parse_card(name, parse_input, true) {
            ParseAttempt {
                status: ParseStatus::StrictCompiled,
                definition,
                ..
            } => ParseAttempt {
                status: ParseStatus::CompiledWithAllowUnsupported,
                parse_error: None,
                definition,
            },
            _ => ParseAttempt {
                status: ParseStatus::ParseFailed,
                parse_error: strict_failure.parse_error,
                definition: None,
            },
        },
    }
}

pub fn compile_snapshot_from_payload(payload: &CardPayload) -> CompilationSnapshot {
    let attempt = parse_card_with_fallback(&payload.name, &payload.parse_input);
    snapshot_from_attempt(payload, &attempt)
}

pub fn snapshot_from_attempt(payload: &CardPayload, attempt: &ParseAttempt) -> CompilationSnapshot {
    CompilationSnapshot::from_definition_result(
        &payload.name,
        &payload.oracle_text,
        attempt.status,
        attempt.parse_error.clone(),
        attempt.definition.as_ref(),
    )
}

impl CompilationSnapshot {
    pub fn from_definition_result(
        card_name: &str,
        oracle_text: &str,
        parse_status: ParseStatus,
        parse_error: Option<String>,
        definition: Option<&CardDefinition>,
    ) -> Self {
        let (
            compiled_text,
            compiled_card_definition,
            oracle_coverage,
            compiled_coverage,
            similarity_score,
            line_delta,
            semantic_mismatch,
            has_unimplemented,
        ) = if let Some(definition) = definition {
            let compiled = canonical_compiled_lines(definition);
            let compiled_text = compiled.join("\n");
            let (
                oracle_coverage,
                compiled_coverage,
                similarity_score,
                line_delta,
                semantic_mismatch,
            ) = compare_card_semantics_scored(
                card_name,
                oracle_text,
                &compiled,
                report_embedding_config(),
            );
            (
                Some(compiled_text),
                Some(stable_compiled_definition_snapshot(definition)),
                oracle_coverage,
                compiled_coverage,
                similarity_score,
                line_delta,
                semantic_mismatch,
                generated_definition_has_unimplemented_content(definition),
            )
        } else {
            (None, None, 0.0, 0.0, 0.0, 0, false, false)
        };

        let mut snapshot = Self {
            card_name: card_name.to_string(),
            oracle_text: oracle_text.to_string(),
            parse_status,
            parse_error,
            compiled_text,
            compiled_card_definition,
            oracle_coverage,
            compiled_coverage,
            similarity_score,
            line_delta,
            semantic_mismatch,
            has_unimplemented,
            content_hash: String::new(),
        };
        snapshot.content_hash = snapshot.compute_content_hash();
        snapshot
    }

    fn compute_content_hash(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.card_name.as_bytes());
        hasher.update([0]);
        hasher.update(self.oracle_text.as_bytes());
        hasher.update([0]);
        hasher.update(self.parse_status.as_str().as_bytes());
        hasher.update([0]);
        hasher.update(self.parse_error.as_deref().unwrap_or("").as_bytes());
        hasher.update([0]);
        hasher.update(self.compiled_text.as_deref().unwrap_or("").as_bytes());
        hasher.update([0]);
        hasher.update(
            self.compiled_card_definition
                .as_deref()
                .unwrap_or("")
                .as_bytes(),
        );
        hasher.update([0]);
        hasher.update(format!("{:.6}", self.oracle_coverage).as_bytes());
        hasher.update([0]);
        hasher.update(format!("{:.6}", self.compiled_coverage).as_bytes());
        hasher.update([0]);
        hasher.update(format!("{:.6}", self.similarity_score).as_bytes());
        hasher.update([0]);
        hasher.update(self.line_delta.to_string().as_bytes());
        hasher.update([0]);
        hasher.update((self.semantic_mismatch as u8).to_string().as_bytes());
        hasher.update([0]);
        hasher.update((self.has_unimplemented as u8).to_string().as_bytes());
        let digest = hasher.finalize();
        let mut out = String::with_capacity(digest.len() * 2);
        for byte in digest {
            out.push_str(&format!("{byte:02x}"));
        }
        out
    }
}

impl CardStatusDb {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, Box<dyn Error>> {
        let path = path.as_ref();
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        let db = Self { conn };
        db.initialize()?;
        Ok(db)
    }

    pub fn initialize(&self) -> Result<(), Box<dyn Error>> {
        let version: i64 = self
            .conn
            .pragma_query_value(None, "user_version", |row| row.get(0))?;
        if version > DB_SCHEMA_VERSION {
            return Err(format!(
                "engine status DB schema version {version} is newer than supported {DB_SCHEMA_VERSION}"
            )
            .into());
        }

        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS card_compilation (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                card_name TEXT NOT NULL,
                oracle_text TEXT NOT NULL,
                parse_status TEXT NOT NULL,
                parse_error TEXT,
                compiled_text TEXT,
                compiled_card_definition TEXT,
                oracle_coverage REAL NOT NULL,
                compiled_coverage REAL NOT NULL,
                similarity_score REAL NOT NULL,
                line_delta INTEGER NOT NULL,
                semantic_mismatch INTEGER NOT NULL,
                has_unimplemented INTEGER NOT NULL,
                compiled_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
                content_hash TEXT NOT NULL,
                UNIQUE(card_name, content_hash)
            );
            CREATE INDEX IF NOT EXISTS idx_card_compilation_name_compiled_at
                ON card_compilation(card_name, compiled_at DESC);
            CREATE TABLE IF NOT EXISTS card_tagging (
                card_name TEXT NOT NULL,
                tag TEXT NOT NULL,
                UNIQUE(card_name, tag)
            );
            CREATE INDEX IF NOT EXISTS idx_card_tagging_tag_card_name
                ON card_tagging(tag, card_name);
            CREATE TABLE IF NOT EXISTS oracle_tag (
                tag TEXT PRIMARY KEY
            );
            CREATE TABLE IF NOT EXISTS registry_card (
                card_name TEXT PRIMARY KEY,
                oracle_text TEXT NOT NULL,
                parse_input TEXT NOT NULL,
                raw_card_json TEXT NOT NULL,
                mana_cost TEXT,
                type_line TEXT,
                power TEXT,
                toughness TEXT,
                loyalty TEXT,
                defense TEXT,
                layout TEXT,
                content_hash TEXT NOT NULL,
                updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
            );
            CREATE INDEX IF NOT EXISTS idx_registry_card_content_hash
                ON registry_card(content_hash);
            DROP VIEW IF EXISTS latest_card_compilation;
            CREATE VIEW latest_card_compilation AS
            SELECT cc.*
            FROM card_compilation cc
            JOIN (
                SELECT card_name, MAX(id) AS max_id
                FROM card_compilation
                GROUP BY card_name
            ) latest
            ON latest.max_id = cc.id;",
        )?;
        self.conn
            .pragma_update(None, "user_version", DB_SCHEMA_VERSION)?;
        Ok(())
    }

    pub fn insert_snapshot_if_changed(
        &self,
        snapshot: &CompilationSnapshot,
    ) -> Result<bool, Box<dyn Error>> {
        let rows = self.conn.execute(
            "INSERT OR IGNORE INTO card_compilation (
                card_name,
                oracle_text,
                parse_status,
                parse_error,
                compiled_text,
                compiled_card_definition,
                oracle_coverage,
                compiled_coverage,
                similarity_score,
                line_delta,
                semantic_mismatch,
                has_unimplemented,
                content_hash
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                snapshot.card_name,
                snapshot.oracle_text,
                snapshot.parse_status.as_str(),
                snapshot.parse_error,
                snapshot.compiled_text,
                snapshot.compiled_card_definition,
                snapshot.oracle_coverage,
                snapshot.compiled_coverage,
                snapshot.similarity_score,
                snapshot.line_delta as i64,
                snapshot.semantic_mismatch,
                snapshot.has_unimplemented,
                snapshot.content_hash,
            ],
        )?;
        Ok(rows > 0)
    }

    pub fn latest_snapshot_hash(&self, card_name: &str) -> Result<Option<String>, Box<dyn Error>> {
        let hash = self
            .conn
            .query_row(
                "SELECT content_hash
                 FROM latest_card_compilation
                 WHERE card_name = ?1",
                [card_name],
                |row| row.get(0),
            )
            .optional()?;
        Ok(hash)
    }

    pub fn replace_tag_rows(
        &mut self,
        rows: &[TagImportRow],
    ) -> Result<TagImportSummary, Box<dyn Error>> {
        let tags = rows
            .iter()
            .map(|row| row.tag.clone())
            .collect::<BTreeSet<_>>();
        self.replace_tag_rows_for_tags(&tags.into_iter().collect::<Vec<_>>(), rows)
    }

    pub fn replace_tag_rows_for_tags(
        &mut self,
        tags: &[String],
        rows: &[TagImportRow],
    ) -> Result<TagImportSummary, Box<dyn Error>> {
        let tags = tags
            .iter()
            .map(|tag| tag.trim())
            .filter(|tag| !tag.is_empty())
            .map(ToOwned::to_owned)
            .collect::<BTreeSet<_>>();
        let tx = self.conn.transaction()?;

        {
            let mut delete = tx.prepare("DELETE FROM card_tagging WHERE tag = ?1")?;
            for tag in &tags {
                delete.execute([tag])?;
            }
        }

        let mut inserted = 0usize;
        {
            let mut insert =
                tx.prepare("INSERT OR IGNORE INTO card_tagging (card_name, tag) VALUES (?1, ?2)")?;
            for row in rows {
                inserted += insert.execute(params![row.card_name, row.tag])?;
            }
        }

        tx.commit()?;
        Ok(TagImportSummary {
            tags_replaced: tags.len(),
            rows_inserted: inserted,
        })
    }

    pub fn replace_oracle_tags(
        &mut self,
        tags: &[String],
    ) -> Result<OracleTagSyncSummary, Box<dyn Error>> {
        let tags = tags
            .iter()
            .map(|tag| tag.trim())
            .filter(|tag| !tag.is_empty())
            .map(ToOwned::to_owned)
            .collect::<BTreeSet<_>>();
        if tags.is_empty() {
            return Err("refusing to replace oracle_tag with an empty tag set".into());
        }

        let tx = self.conn.transaction()?;
        let existing_count: usize =
            tx.query_row("SELECT COUNT(*) FROM oracle_tag", [], |row| row.get(0))?;
        tx.execute("DELETE FROM oracle_tag", [])?;

        let mut inserted = 0usize;
        {
            let mut insert = tx.prepare("INSERT INTO oracle_tag (tag) VALUES (?1)")?;
            for tag in &tags {
                inserted += insert.execute([tag])?;
            }
        }

        tx.commit()?;
        Ok(OracleTagSyncSummary {
            tags_replaced: existing_count,
            rows_inserted: inserted,
        })
    }

    pub fn replace_registry_cards(
        &mut self,
        rows: &[RegistryCardRecord],
    ) -> Result<RegistrySyncSummary, Box<dyn Error>> {
        let normalized_rows = rows
            .iter()
            .filter_map(|row| {
                let normalized = normalize_lookup_name(&row.payload.name);
                if normalized.is_empty() {
                    return None;
                }
                let mut row = row.clone();
                row.payload.name = normalized;
                Some(row)
            })
            .collect::<Vec<_>>();
        if normalized_rows.is_empty() {
            return Err("refusing to replace registry cards with an empty row set".into());
        }

        let tx = self.conn.transaction()?;
        let mut existing_hashes = BTreeMap::new();
        {
            let mut stmt = tx.prepare("SELECT card_name, content_hash FROM registry_card")?;
            let rows = stmt.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?;
            for row in rows {
                let (name, hash) = row?;
                existing_hashes.insert(name, hash);
            }
        }

        let allowed_names = normalized_rows
            .iter()
            .map(|row| row.payload.name.clone())
            .collect::<BTreeSet<_>>();

        let mut inserted = 0usize;
        let mut updated = 0usize;
        let mut unchanged = 0usize;
        {
            let mut upsert = tx.prepare(
                "INSERT INTO registry_card (
                    card_name,
                    oracle_text,
                    parse_input,
                    raw_card_json,
                    mana_cost,
                    type_line,
                    power,
                    toughness,
                    loyalty,
                    defense,
                    layout,
                    content_hash,
                    updated_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
                ON CONFLICT(card_name) DO UPDATE SET
                    oracle_text = excluded.oracle_text,
                    parse_input = excluded.parse_input,
                    raw_card_json = excluded.raw_card_json,
                    mana_cost = excluded.mana_cost,
                    type_line = excluded.type_line,
                    power = excluded.power,
                    toughness = excluded.toughness,
                    loyalty = excluded.loyalty,
                    defense = excluded.defense,
                    layout = excluded.layout,
                    content_hash = excluded.content_hash,
                    updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')",
            )?;
            for row in &normalized_rows {
                match existing_hashes.get(&row.payload.name) {
                    None => inserted += 1,
                    Some(existing_hash) if existing_hash == &row.content_hash => unchanged += 1,
                    Some(_) => updated += 1,
                }
                upsert.execute(params![
                    row.payload.name.as_str(),
                    row.payload.oracle_text.as_str(),
                    row.payload.parse_input.as_str(),
                    row.raw_card_json.as_str(),
                    row.mana_cost.as_deref(),
                    row.type_line.as_deref(),
                    row.power.as_deref(),
                    row.toughness.as_deref(),
                    row.loyalty.as_deref(),
                    row.defense.as_deref(),
                    row.layout.as_deref(),
                    row.content_hash.as_str(),
                ])?;
            }
        }

        tx.execute_batch(
            "DROP TABLE IF EXISTS temp_allowed_registry_card;
             CREATE TEMP TABLE temp_allowed_registry_card (
                 card_name TEXT PRIMARY KEY
             );",
        )?;
        {
            let mut insert = tx.prepare(
                "INSERT OR IGNORE INTO temp_allowed_registry_card(card_name) VALUES (?1)",
            )?;
            for name in &allowed_names {
                insert.execute([name])?;
            }
        }
        let deleted: usize = tx.query_row(
            "SELECT COUNT(*)
             FROM registry_card
             WHERE NOT EXISTS (
                 SELECT 1
                 FROM temp_allowed_registry_card allowed
                 WHERE allowed.card_name = registry_card.card_name
             )",
            [],
            |row| row.get(0),
        )?;
        tx.execute(
            "DELETE FROM registry_card
             WHERE NOT EXISTS (
                 SELECT 1
                 FROM temp_allowed_registry_card allowed
                 WHERE allowed.card_name = registry_card.card_name
             )",
            [],
        )?;
        tx.execute("DROP TABLE temp_allowed_registry_card", [])?;
        tx.commit()?;

        Ok(RegistrySyncSummary {
            inserted,
            updated,
            unchanged,
            deleted,
        })
    }

    pub fn prune_cards_not_in_names(
        &mut self,
        allowed_names: &[String],
    ) -> Result<CardPruneSummary, Box<dyn Error>> {
        let allowed_names = allowed_names
            .iter()
            .map(|name| normalize_lookup_name(name))
            .filter(|name| !name.is_empty())
            .collect::<BTreeSet<_>>();
        if allowed_names.is_empty() {
            return Err("refusing to prune against an empty canonical card set".into());
        }

        let tx = self.conn.transaction()?;
        tx.execute_batch(
            "DROP TABLE IF EXISTS temp_allowed_card_name;
             CREATE TEMP TABLE temp_allowed_card_name (
                 card_name TEXT PRIMARY KEY
             );",
        )?;

        {
            let mut insert =
                tx.prepare("INSERT OR IGNORE INTO temp_allowed_card_name(card_name) VALUES (?1)")?;
            for name in &allowed_names {
                insert.execute([name])?;
            }
        }

        let distinct_cards_deleted: usize = tx.query_row(
            "SELECT COUNT(DISTINCT card_name)
             FROM card_compilation
             WHERE NOT EXISTS (
                 SELECT 1
                 FROM temp_allowed_card_name allowed
                 WHERE allowed.card_name = card_compilation.card_name
             )",
            [],
            |row| row.get(0),
        )?;
        let compilation_rows_deleted: usize = tx.query_row(
            "SELECT COUNT(*)
             FROM card_compilation
             WHERE NOT EXISTS (
                 SELECT 1
                 FROM temp_allowed_card_name allowed
                 WHERE allowed.card_name = card_compilation.card_name
             )",
            [],
            |row| row.get(0),
        )?;
        let tag_rows_deleted: usize = tx.query_row(
            "SELECT COUNT(*)
             FROM card_tagging
             WHERE NOT EXISTS (
                 SELECT 1
                 FROM temp_allowed_card_name allowed
                 WHERE allowed.card_name = card_tagging.card_name
             )",
            [],
            |row| row.get(0),
        )?;

        tx.execute(
            "DELETE FROM card_tagging
             WHERE NOT EXISTS (
                 SELECT 1
                 FROM temp_allowed_card_name allowed
                 WHERE allowed.card_name = card_tagging.card_name
             )",
            [],
        )?;
        tx.execute(
            "DELETE FROM card_compilation
             WHERE NOT EXISTS (
                 SELECT 1
                 FROM temp_allowed_card_name allowed
                 WHERE allowed.card_name = card_compilation.card_name
             )",
            [],
        )?;
        tx.execute("DROP TABLE temp_allowed_card_name", [])?;
        tx.commit()?;

        Ok(CardPruneSummary {
            distinct_cards_deleted,
            compilation_rows_deleted,
            tag_rows_deleted,
        })
    }

    pub fn prune_compilation_history_to_latest(
        &mut self,
    ) -> Result<CompilationHistoryCleanupSummary, Box<dyn Error>> {
        let tx = self.conn.transaction()?;
        let distinct_cards_retained: usize = tx.query_row(
            "SELECT COUNT(DISTINCT card_name) FROM card_compilation",
            [],
            |row| row.get(0),
        )?;
        let compilation_rows_deleted: usize = tx.query_row(
            "SELECT COUNT(*)
             FROM card_compilation
             WHERE id NOT IN (
                 SELECT latest_id
                 FROM (
                     SELECT MAX(id) AS latest_id
                     FROM card_compilation
                     GROUP BY card_name
                 )
             )",
            [],
            |row| row.get(0),
        )?;

        tx.execute(
            "DELETE FROM card_compilation
             WHERE id NOT IN (
                 SELECT latest_id
                 FROM (
                     SELECT MAX(id) AS latest_id
                     FROM card_compilation
                     GROUP BY card_name
                 )
             )",
            [],
        )?;
        tx.commit()?;

        Ok(CompilationHistoryCleanupSummary {
            distinct_cards_retained,
            compilation_rows_deleted,
        })
    }

    pub fn connection(&self) -> &Connection {
        &self.conn
    }

    pub fn oracle_tags(&self) -> Result<Vec<String>, Box<dyn Error>> {
        let mut stmt = self
            .conn
            .prepare("SELECT tag FROM oracle_tag ORDER BY tag ASC")?;
        let tags = stmt
            .query_map([], |row| row.get(0))?
            .collect::<Result<Vec<String>, _>>()?;
        Ok(tags)
    }

    pub fn card_names_for_tag(&self, tag: &str) -> Result<Vec<String>, Box<dyn Error>> {
        let mut stmt = self.conn.prepare(
            "SELECT card_name
             FROM card_tagging
             WHERE tag = ?1
             ORDER BY card_name ASC",
        )?;
        let names = stmt
            .query_map([tag], |row| row.get(0))?
            .collect::<Result<Vec<String>, _>>()?;
        Ok(names)
    }

    pub fn registry_card_payloads(&self) -> Result<Vec<CardPayload>, Box<dyn Error>> {
        let mut stmt = self.conn.prepare(
            "SELECT card_name, oracle_text, parse_input
             FROM registry_card
             ORDER BY card_name ASC",
        )?;
        let rows = stmt
            .query_map([], |row| {
                Ok(CardPayload {
                    name: row.get(0)?,
                    oracle_text: row.get(1)?,
                    metadata_lines: Vec::new(),
                    parse_input: row.get(2)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn registry_card_count(&self) -> Result<usize, Box<dyn Error>> {
        let count = self
            .conn
            .query_row("SELECT COUNT(*) FROM registry_card", [], |row| row.get(0))?;
        Ok(count)
    }
}

pub fn read_tag_rows_from_research_csv_paths(
    paths: &[String],
) -> Result<Vec<TagImportRow>, Box<dyn Error>> {
    let mut rows = BTreeSet::new();
    for path in paths {
        let mut reader = csv::Reader::from_path(path)?;
        let headers = reader.headers()?.clone();
        let local_found_idx = header_index(&headers, "local_card_found")?;
        let local_name_idx = header_index(&headers, "local_card_name")?;
        let matched_tags_idx = header_index(&headers, "matched_tags")?;
        for record in reader.records() {
            let record = record?;
            if !record
                .get(local_found_idx)
                .unwrap_or("")
                .eq_ignore_ascii_case("yes")
            {
                continue;
            }

            let local_name = record.get(local_name_idx).unwrap_or("").trim();
            if local_name.is_empty() {
                continue;
            }

            let matched_tags = record.get(matched_tags_idx).unwrap_or("");
            for tag in split_tags(matched_tags) {
                rows.insert((local_name.to_string(), tag));
            }
        }
    }

    Ok(rows
        .into_iter()
        .map(|(card_name, tag)| TagImportRow { card_name, tag })
        .collect())
}

pub fn fetch_functional_oracle_tags_from_url(url: &str) -> Result<Vec<String>, Box<dyn Error>> {
    let client = build_http_client()?;
    let html = client.get(url).send()?.error_for_status()?.text()?;
    read_functional_oracle_tags_from_html(&html)
}

impl TaggerClient {
    pub fn open(base_url: &str) -> Result<Self, Box<dyn Error>> {
        let client = build_http_client()?;
        let base_url = base_url.trim_end_matches('/').to_string();
        let html = client
            .get(format!("{base_url}/"))
            .send()?
            .error_for_status()?
            .text()?;
        let csrf_token = extract_meta_content(&html, "csrf-token")
            .ok_or_else(|| "missing csrf-token meta tag in Tagger HTML".to_string())?;
        Ok(Self {
            client,
            base_url,
            csrf_token,
        })
    }

    pub fn fetch_oracle_tag_page(
        &self,
        tag_slug: &str,
        page: usize,
    ) -> Result<TaggerTagPage, Box<dyn Error>> {
        let payload = serde_json::json!({
            "query": TAGGER_FETCH_ORACLE_CARD_TAG_QUERY,
            "operationName": "FetchOracleCardTagPage",
            "variables": {
                "slug": tag_slug,
                "type": "ORACLE_CARD_TAG",
                "page": page,
            }
        });
        let response: TaggerGraphqlResponse<TaggerTagBySlugData> = self
            .client
            .post(format!("{}/graphql", self.base_url))
            .header("X-CSRF-Token", &self.csrf_token)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()?
            .error_for_status()?
            .json()?;

        if let Some(error) = response.errors.and_then(|mut errors| errors.pop()) {
            return Err(format!("Tagger GraphQL error for tag '{tag_slug}': {error}").into());
        }

        let tag = response
            .data
            .and_then(|data| data.tag_by_slug)
            .ok_or_else(|| format!("Tagger did not return tag '{tag_slug}'"))?;
        let taggings = tag.taggings;
        let card_names = taggings
            .results
            .into_iter()
            .map(|result| {
                let candidate = result.subject_name.trim();
                if candidate.is_empty() {
                    result.card.name
                } else {
                    candidate.to_string()
                }
            })
            .collect();

        Ok(TaggerTagPage {
            total: taggings.total,
            per_page: taggings.per_page,
            card_names,
        })
    }
}

pub fn fetch_all_oracle_tag_card_names(
    client: &TaggerClient,
    tag_slug: &str,
) -> Result<Vec<String>, Box<dyn Error>> {
    let mut page = 1usize;
    let mut names = BTreeSet::new();

    loop {
        let result = client.fetch_oracle_tag_page(tag_slug, page)?;
        for name in result.card_names {
            let normalized = normalize_lookup_name(&name);
            if !normalized.is_empty() {
                names.insert(normalized);
            }
        }

        if result.total == 0 || page * result.per_page >= result.total {
            break;
        }
        page += 1;
    }

    Ok(names.into_iter().collect())
}

pub fn build_local_tag_rows(
    tag: &str,
    tagged_card_names: &[String],
    local_card_names: &BTreeSet<String>,
) -> Vec<TagImportRow> {
    let mut seen = BTreeSet::new();
    let mut rows = Vec::new();

    for name in tagged_card_names {
        let normalized = normalize_lookup_name(name);
        if !local_card_names.contains(&normalized) {
            continue;
        }
        if seen.insert((normalized.clone(), tag.to_string())) {
            rows.push(TagImportRow {
                card_name: normalized,
                tag: tag.to_string(),
            });
        }
    }

    rows
}

pub fn read_functional_oracle_tags_from_html(html: &str) -> Result<Vec<String>, Box<dyn Error>> {
    let mut tags = BTreeSet::new();
    let mut cursor = html;

    while let Some(start) = cursor.find("<h2") {
        cursor = &cursor[start..];
        let Some(open_end) = cursor.find('>') else {
            break;
        };
        let after_open = &cursor[open_end + 1..];
        let Some(close) = after_open.find("</h2>") else {
            break;
        };

        let heading = decode_html_entities(strip_html_tags(&after_open[..close]));
        let remainder = &after_open[close + "</h2>".len()..];
        let next_heading = remainder.find("<h2").unwrap_or(remainder.len());
        let section_html = &remainder[..next_heading];

        if heading.trim().ends_with(" (functional)") {
            collect_oracle_tag_links(section_html, &mut tags);
        }

        cursor = remainder;
    }

    if tags.is_empty() {
        return Err("no functional oracle tags found in Scryfall tag docs".into());
    }

    Ok(tags.into_iter().collect())
}

fn build_http_client() -> Result<Client, Box<dyn Error>> {
    Ok(Client::builder()
        .cookie_store(true)
        .user_agent(format!("ironsmith/{}", env!("CARGO_PKG_VERSION")))
        .build()?)
}

fn extract_meta_content(html: &str, meta_name: &str) -> Option<String> {
    let needle = format!(r#"<meta name="{meta_name}" content=""#);
    let start = html.find(&needle)? + needle.len();
    let rest = &html[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

#[derive(Debug, serde::Deserialize)]
struct TaggerGraphqlResponse<T> {
    data: Option<T>,
    errors: Option<Vec<TaggerGraphqlError>>,
}

#[derive(Debug, serde::Deserialize)]
struct TaggerGraphqlError {
    message: String,
}

impl fmt::Display for TaggerGraphqlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

#[derive(Debug, serde::Deserialize)]
struct TaggerTagBySlugData {
    #[serde(rename = "tagBySlug")]
    tag_by_slug: Option<TaggerTagBySlug>,
}

#[derive(Debug, serde::Deserialize)]
struct TaggerTagBySlug {
    taggings: TaggerTaggingsResults,
}

#[derive(Debug, serde::Deserialize)]
struct TaggerTaggingsResults {
    #[serde(rename = "perPage")]
    per_page: usize,
    total: usize,
    results: Vec<TaggerTaggingResult>,
}

#[derive(Debug, serde::Deserialize)]
struct TaggerTaggingResult {
    #[serde(rename = "subjectName")]
    subject_name: String,
    card: TaggerCardResult,
}

#[derive(Debug, serde::Deserialize)]
struct TaggerCardResult {
    name: String,
}

fn header_index(headers: &StringRecord, header: &str) -> Result<usize, Box<dyn Error>> {
    headers
        .iter()
        .position(|candidate| candidate == header)
        .ok_or_else(|| format!("CSV is missing required '{header}' header").into())
}

fn split_tags(raw: &str) -> Vec<String> {
    raw.split([',', ';', '|'])
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn collect_oracle_tag_links(section_html: &str, tags: &mut BTreeSet<String>) {
    let mut cursor = section_html;
    while let Some(start) = cursor.find("<a ") {
        cursor = &cursor[start + 3..];
        let Some(open_end) = cursor.find('>') else {
            break;
        };
        let open_tag = &cursor[..open_end];
        let after_open = &cursor[open_end + 1..];
        let Some(close) = after_open.find("</a>") else {
            break;
        };
        let anchor_text = decode_html_entities(strip_html_tags(&after_open[..close]).trim());
        if !anchor_text.is_empty() && anchor_targets_oracle_tag(open_tag) {
            tags.insert(anchor_text);
        }
        cursor = &after_open[close + "</a>".len()..];
    }
}

fn anchor_targets_oracle_tag(open_tag: &str) -> bool {
    open_tag.contains("oracletag%3A")
        || open_tag.contains("oracletag:")
        || open_tag.contains("function%3A")
        || open_tag.contains("function:")
}

fn strip_html_tags(raw: &str) -> &str {
    raw.split('<').next().unwrap_or(raw)
}

fn decode_html_entities(raw: &str) -> String {
    raw.replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
}

fn registry_card_content_hash(
    payload: &CardPayload,
    raw_card_json: &str,
    layout: Option<&str>,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(payload.name.as_bytes());
    hasher.update([0]);
    hasher.update(payload.oracle_text.as_bytes());
    hasher.update([0]);
    hasher.update(payload.parse_input.as_bytes());
    hasher.update([0]);
    hasher.update(layout.unwrap_or("").as_bytes());
    hasher.update([0]);
    hasher.update(raw_card_json.as_bytes());
    let digest = hasher.finalize();
    let mut out = String::with_capacity(digest.len() * 2);
    for byte in digest {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

fn load_registry_cards_from_values<I>(cards: I) -> BTreeMap<String, RegistryCardRecord>
where
    I: IntoIterator<Item = Value>,
{
    let mut out = BTreeMap::new();
    for card in cards {
        let Some(record) = build_registry_card_record(&card) else {
            continue;
        };
        out.entry(record.payload.name.clone()).or_insert(record);
    }
    out
}

#[cfg(test)]
fn load_canonical_cards_from_values<I>(cards: I) -> BTreeMap<String, CardPayload>
where
    I: IntoIterator<Item = Value>,
{
    load_registry_cards_from_values(cards)
        .into_iter()
        .map(|(name, record)| (name, record.payload))
        .collect()
}

fn build_registry_card_record(card: &Value) -> Option<RegistryCardRecord> {
    if card
        .get("digital")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return None;
    }
    if !card_is_legal_in_supported_paper_format(card) {
        return None;
    }

    let face = get_first_face(card);
    let name = normalize_lookup_name(&pick_field(card, face, "name")?);
    if name.is_empty() {
        return None;
    }

    let oracle_text = pick_field(card, face, "oracle_text")?.trim().to_string();
    if oracle_text.is_empty() {
        return None;
    }

    let mana_cost = pick_field(card, face, "mana_cost");
    let type_line = pick_field(card, face, "type_line");
    let power = pick_field(card, face, "power");
    let toughness = pick_field(card, face, "toughness");
    let loyalty = pick_field(card, face, "loyalty");
    let defense = pick_field(card, face, "defense");

    let mut metadata_lines = Vec::new();
    if let Some(mana_cost) = mana_cost
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        metadata_lines.push(format!("Mana cost: {}", mana_cost.trim()));
    }
    if let Some(type_line) = type_line
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        metadata_lines.push(format!("Type: {}", type_line.trim()));
    }
    if let (Some(power), Some(toughness)) = (power.as_deref(), toughness.as_deref())
        && !power.trim().is_empty()
        && !toughness.trim().is_empty()
    {
        metadata_lines.push(format!(
            "Power/Toughness: {}/{}",
            power.trim(),
            toughness.trim()
        ));
    }
    if let Some(loyalty) = loyalty.as_deref().filter(|value| !value.trim().is_empty()) {
        metadata_lines.push(format!("Loyalty: {}", loyalty.trim()));
    }
    if let Some(defense) = defense.as_deref().filter(|value| !value.trim().is_empty()) {
        metadata_lines.push(format!("Defense: {}", defense.trim()));
    }

    let parse_input = build_parse_input(&metadata_lines, &oracle_text);
    let payload = CardPayload {
        name,
        oracle_text,
        metadata_lines,
        parse_input,
    };
    let layout = card
        .get("layout")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);
    let raw_card_json = serde_json::to_string(card).ok()?;
    let content_hash = registry_card_content_hash(&payload, &raw_card_json, layout.as_deref());

    Some(RegistryCardRecord {
        payload,
        raw_card_json,
        mana_cost: mana_cost
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned),
        type_line: type_line
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned),
        power: power
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned),
        toughness: toughness
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned),
        loyalty: loyalty
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned),
        defense: defense
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned),
        layout,
        content_hash,
    })
}

fn card_is_legal_in_supported_paper_format(card: &Value) -> bool {
    let Some(legalities) = card.get("legalities").and_then(Value::as_object) else {
        return true;
    };
    if legalities.is_empty() {
        return true;
    }

    SUPPORTED_PAPER_FORMATS.iter().any(|format| {
        legalities
            .get(*format)
            .and_then(Value::as_str)
            .is_some_and(|status| status == "legal")
    })
}

fn parse_card(name: &str, parse_input: &str, allow_unsupported: bool) -> ParseAttempt {
    with_allow_unsupported(allow_unsupported, || {
        let result = panic::catch_unwind(AssertUnwindSafe(|| {
            CardDefinitionBuilder::new(CardId::from_raw(FIXED_SNAPSHOT_CARD_ID), name)
                .parse_text(parse_input.to_string())
        }));
        match result {
            Ok(Ok(definition)) => ParseAttempt {
                status: ParseStatus::StrictCompiled,
                parse_error: None,
                definition: Some(definition),
            },
            Ok(Err(err)) => ParseAttempt {
                status: ParseStatus::ParseFailed,
                parse_error: Some(format!("{err:?}")),
                definition: None,
            },
            Err(payload) => ParseAttempt {
                status: ParseStatus::ParseFailed,
                parse_error: Some(format!("panic: {}", panic_payload_to_string(payload))),
                definition: None,
            },
        }
    })
}

fn with_allow_unsupported<T>(enabled: bool, f: impl FnOnce() -> T) -> T {
    let original = env::var("IRONSMITH_PARSER_ALLOW_UNSUPPORTED").ok();
    unsafe {
        if enabled {
            env::set_var("IRONSMITH_PARSER_ALLOW_UNSUPPORTED", "1");
        } else {
            env::remove_var("IRONSMITH_PARSER_ALLOW_UNSUPPORTED");
        }
    }
    let result = f();
    match original {
        Some(value) => unsafe {
            env::set_var("IRONSMITH_PARSER_ALLOW_UNSUPPORTED", value);
        },
        None => unsafe {
            env::remove_var("IRONSMITH_PARSER_ALLOW_UNSUPPORTED");
        },
    }
    result
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

fn stable_compiled_definition_snapshot(definition: &CardDefinition) -> String {
    let mut sanitized = definition.clone();
    sanitized.card.id = CardId::from_raw(FIXED_SNAPSHOT_CARD_ID);
    sanitized.card.oracle_text.clear();
    sanitized.card.other_face = sanitized.card.other_face.map(|_| CardId::from_raw(2));
    for ability in &mut sanitized.abilities {
        ability.text = None;
    }
    format!("{sanitized:#?}")
}

fn get_first_face(card: &Value) -> Option<&Value> {
    card.get("card_faces")
        .and_then(Value::as_array)
        .and_then(|faces| faces.first())
}

fn value_to_string(value: &Value) -> Option<String> {
    if value.is_null() {
        return None;
    }
    if let Some(value) = value.as_str() {
        return Some(value.to_string());
    }
    Some(value.to_string())
}

fn pick_field(card: &Value, face: Option<&Value>, key: &str) -> Option<String> {
    if let Some(value) = card.get(key).and_then(value_to_string) {
        return Some(value);
    }
    face.and_then(|value| value.get(key))
        .and_then(value_to_string)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic_compare::{compare_semantics_scored, report_embedding_config};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_path(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        env::temp_dir().join(format!("ironsmith-{name}-{nanos}.sqlite3"))
    }

    fn lightning_bolt_payload() -> CardPayload {
        CardPayload {
            name: "Lightning Bolt".to_string(),
            oracle_text: "Lightning Bolt deals 3 damage to any target.".to_string(),
            metadata_lines: vec!["Mana cost: {R}".to_string(), "Type: Instant".to_string()],
            parse_input:
                "Mana cost: {R}\nType: Instant\nLightning Bolt deals 3 damage to any target."
                    .to_string(),
        }
    }

    #[test]
    fn canonical_loader_dedupes_by_name() {
        let cards = vec![
            serde_json::json!({
                "name": "Lightning Bolt",
                "oracle_text": "Lightning Bolt deals 3 damage to any target.",
                "mana_cost": "{R}",
                "type_line": "Instant"
            }),
            serde_json::json!({
                "name": "Lightning Bolt",
                "oracle_text": "Wrong duplicate should not win",
                "mana_cost": "{1}{R}",
                "type_line": "Sorcery"
            }),
        ];

        let loaded = load_canonical_cards_from_values(cards);
        assert_eq!(loaded.len(), 1);
        assert_eq!(
            loaded
                .get("Lightning Bolt")
                .expect("lightning bolt")
                .oracle_text,
            "Lightning Bolt deals 3 damage to any target."
        );
    }

    #[test]
    fn canonical_loader_skips_digital_cards() {
        let cards = vec![
            serde_json::json!({
                "name": "Lightning Bolt",
                "oracle_text": "Lightning Bolt deals 3 damage to any target.",
                "mana_cost": "{R}",
                "type_line": "Instant",
                "digital": false
            }),
            serde_json::json!({
                "name": "Digital Bolt",
                "oracle_text": "Conjure a card named Lightning Bolt into your hand.",
                "mana_cost": "{R}",
                "type_line": "Instant",
                "digital": true
            }),
        ];

        let loaded = load_canonical_cards_from_values(cards);
        assert!(loaded.contains_key("Lightning Bolt"));
        assert!(!loaded.contains_key("Digital Bolt"));
    }

    #[test]
    fn canonical_loader_skips_cards_without_supported_format_legality() {
        let cards = vec![
            serde_json::json!({
                "name": "Lightning Bolt",
                "oracle_text": "Lightning Bolt deals 3 damage to any target.",
                "mana_cost": "{R}",
                "type_line": "Instant",
                "legalities": {
                    "modern": "legal",
                    "legacy": "legal",
                    "vintage": "legal",
                    "commander": "legal",
                    "standard": "not_legal"
                }
            }),
            serde_json::json!({
                "name": "Contract from Below",
                "oracle_text": "Discard your hand, ante the top card of your library, then draw seven cards.",
                "mana_cost": "{B}",
                "type_line": "Sorcery",
                "legalities": {
                    "modern": "not_legal",
                    "legacy": "not_legal",
                    "vintage": "not_legal",
                    "commander": "not_legal",
                    "standard": "not_legal"
                }
            }),
            serde_json::json!({
                "name": "Fixture Without Legalities",
                "oracle_text": "Draw a card.",
                "mana_cost": "{U}",
                "type_line": "Sorcery"
            }),
        ];

        let loaded = load_canonical_cards_from_values(cards);
        assert!(loaded.contains_key("Lightning Bolt"));
        assert!(!loaded.contains_key("Contract from Below"));
        assert!(loaded.contains_key("Fixture Without Legalities"));
    }

    #[test]
    fn stable_snapshot_hash_is_repeatable() {
        let payload = lightning_bolt_payload();
        let first = compile_snapshot_from_payload(&payload);
        let second = compile_snapshot_from_payload(&payload);
        assert_eq!(first.content_hash, second.content_hash);
        assert_eq!(
            first.compiled_card_definition,
            second.compiled_card_definition
        );
    }

    #[test]
    fn compilation_snapshot_uses_embedding_backed_similarity() {
        let oracle = "Survival — At the beginning of your second main phase, if this creature is tapped, reveal cards from the top of your library until you reveal a land card. Put that card into your hand and the rest on the bottom of your library in a random order.";
        let definition = CardDefinitionBuilder::new(CardId::new(), "House Cartographer")
            .parse_text(oracle)
            .expect("house cartographer should parse");
        let compiled = canonical_compiled_lines(&definition);
        let snapshot = CompilationSnapshot::from_definition_result(
            "House Cartographer",
            oracle,
            ParseStatus::StrictCompiled,
            None,
            Some(&definition),
        );
        let (_oracle_cov, _compiled_cov, lexical_similarity, _delta, _mismatch) =
            compare_card_semantics_scored("House Cartographer", oracle, &compiled, None);
        let (_oracle_cov, _compiled_cov, embedded_similarity, _delta, _mismatch) =
            compare_card_semantics_scored(
                "House Cartographer",
                oracle,
                &compiled,
                report_embedding_config(),
            );

        assert_eq!(snapshot.similarity_score, embedded_similarity);
        assert!(
            embedded_similarity > lexical_similarity,
            "expected embedding-backed similarity to improve over lexical-only scoring, lexical={lexical_similarity}, embedded={embedded_similarity}, compiled={compiled:?}"
        );
    }

    #[test]
    fn compilation_snapshot_uses_same_normalized_text_as_default_cli_surface() {
        let oracle = "Enlist (As this creature attacks, you may tap a nonattacking creature you control without summoning sickness. When you do, add its power to this creature's until end of turn.)\nWhen this creature enters, create a 1/1 white Soldier creature token.";
        let definition = CardDefinitionBuilder::new(CardId::new(), "Argivian Cavalier")
            .parse_text(oracle)
            .expect("argivian cavalier should parse");

        let snapshot = CompilationSnapshot::from_definition_result(
            "Argivian Cavalier",
            oracle,
            ParseStatus::StrictCompiled,
            None,
            Some(&definition),
        );
        let stored_text = snapshot
            .compiled_text
            .expect("snapshot should include compiled text");

        assert!(
            stored_text.contains(
                "Whenever this creature attacks, you may tap another nonattacking creature you control. When you do, this creature gets +X/+0 until end of turn, where X is that creature's power."
            ),
            "expected normalized enlist text in snapshot, got {stored_text}"
        );
        assert!(
            !stored_text.contains("enlist_attacker") && !stored_text.contains("enlisted_creature"),
            "expected snapshot to avoid raw enlist tags, got {stored_text}"
        );
    }

    #[test]
    fn db_initialization_is_idempotent() {
        let path = unique_temp_path("init");
        let db = CardStatusDb::open(&path).expect("open db");
        db.initialize().expect("reinitialize");
        let version: i64 = db
            .connection()
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .expect("version");
        assert_eq!(version, DB_SCHEMA_VERSION);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn identical_snapshot_does_not_duplicate_rows() {
        let path = unique_temp_path("dedupe");
        let db = CardStatusDb::open(&path).expect("open db");
        let snapshot = compile_snapshot_from_payload(&lightning_bolt_payload());

        assert!(
            db.insert_snapshot_if_changed(&snapshot)
                .expect("first insert")
        );
        assert!(
            !db.insert_snapshot_if_changed(&snapshot)
                .expect("second insert")
        );

        let count: i64 = db
            .connection()
            .query_row("SELECT COUNT(*) FROM card_compilation", [], |row| {
                row.get(0)
            })
            .expect("count rows");
        assert_eq!(count, 1);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn changed_snapshot_appends_new_row_and_latest_view_tracks_it() {
        let path = unique_temp_path("latest");
        let db = CardStatusDb::open(&path).expect("open db");
        let base = compile_snapshot_from_payload(&lightning_bolt_payload());
        let mut changed = base.clone();
        changed.similarity_score = 0.5;
        changed.content_hash = changed.compute_content_hash();

        assert!(db.insert_snapshot_if_changed(&base).expect("insert base"));
        assert!(
            db.insert_snapshot_if_changed(&changed)
                .expect("insert changed")
        );

        let count: i64 = db
            .connection()
            .query_row("SELECT COUNT(*) FROM card_compilation", [], |row| {
                row.get(0)
            })
            .expect("count rows");
        assert_eq!(count, 2);

        let latest: f32 = db
            .connection()
            .query_row(
                "SELECT similarity_score FROM latest_card_compilation WHERE card_name = ?1",
                ["Lightning Bolt"],
                |row| row.get(0),
            )
            .expect("latest row");
        assert_eq!(latest, 0.5);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn tag_import_replaces_existing_rows_and_dedupes() {
        let path = unique_temp_path("tags");
        let mut db = CardStatusDb::open(&path).expect("open db");
        let first = vec![
            TagImportRow {
                card_name: "Lightning Bolt".to_string(),
                tag: "burn".to_string(),
            },
            TagImportRow {
                card_name: "Chain Lightning".to_string(),
                tag: "burn".to_string(),
            },
        ];
        db.replace_tag_rows(&first).expect("insert first tag rows");

        let second = vec![
            TagImportRow {
                card_name: "Lightning Bolt".to_string(),
                tag: "burn".to_string(),
            },
            TagImportRow {
                card_name: "Lightning Bolt".to_string(),
                tag: "burn".to_string(),
            },
        ];
        let summary = db.replace_tag_rows(&second).expect("replace tags");
        assert_eq!(summary.tags_replaced, 1);
        assert_eq!(summary.rows_inserted, 1);

        let rows: Vec<String> = {
            let mut stmt = db
                .connection()
                .prepare("SELECT card_name FROM card_tagging WHERE tag = 'burn' ORDER BY card_name")
                .expect("prepare query");
            stmt.query_map([], |row| row.get(0))
                .expect("query rows")
                .collect::<Result<_, _>>()
                .expect("collect rows")
        };
        assert_eq!(rows, vec!["Lightning Bolt".to_string()]);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn functional_oracle_tag_parser_ignores_art_sections() {
        let html = r#"
            <h2>#</h2>
            <p><a href="/search?q=art%3Abolt&amp;unique=art">bolt</a></p>
            <h2># (functional)</h2>
            <p>
                <a href="/search?q=oracletag%3Aburn">burn</a>
                <a href="/search?q=oracletag%3Aburn">burn</a>
                <a href="/search?q=oracletag%3Acard-draw">card-draw</a>
                <a href="/search?q=art%3Awrong&amp;unique=art">wrong</a>
            </p>
            <h2>A</h2>
            <p><a href="/search?q=art%3Aangel&amp;unique=art">angel</a></p>
            <h2>A (functional)</h2>
            <p><a href="/search?q=function%3Aanthem">anthem</a></p>
        "#;

        let tags = read_functional_oracle_tags_from_html(html).expect("parse oracle tags");
        assert_eq!(
            tags,
            vec![
                "anthem".to_string(),
                "burn".to_string(),
                "card-draw".to_string(),
            ]
        );
    }

    #[test]
    fn oracle_tag_sync_replaces_existing_rows() {
        let path = unique_temp_path("oracle-tags");
        let mut db = CardStatusDb::open(&path).expect("open db");

        let first = vec!["burn".to_string(), "card-draw".to_string()];
        let summary = db
            .replace_oracle_tags(&first)
            .expect("insert initial oracle tags");
        assert_eq!(summary.tags_replaced, 0);
        assert_eq!(summary.rows_inserted, 2);

        let second = vec![
            "burn".to_string(),
            "burn".to_string(),
            "removal".to_string(),
        ];
        let summary = db
            .replace_oracle_tags(&second)
            .expect("replace oracle tags");
        assert_eq!(summary.tags_replaced, 2);
        assert_eq!(summary.rows_inserted, 2);

        let tags: Vec<String> = {
            let mut stmt = db
                .connection()
                .prepare("SELECT tag FROM oracle_tag ORDER BY tag")
                .expect("prepare query");
            stmt.query_map([], |row| row.get(0))
                .expect("query rows")
                .collect::<Result<_, _>>()
                .expect("collect rows")
        };
        assert_eq!(tags, vec!["burn".to_string(), "removal".to_string()]);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn explicit_tag_replacement_clears_rows_even_when_new_rows_are_empty() {
        let path = unique_temp_path("empty-tag-replace");
        let mut db = CardStatusDb::open(&path).expect("open db");
        db.replace_tag_rows(&[TagImportRow {
            card_name: "Lightning Bolt".to_string(),
            tag: "burn".to_string(),
        }])
        .expect("seed burn row");

        let summary = db
            .replace_tag_rows_for_tags(&["burn".to_string()], &[])
            .expect("clear tag rows");
        assert_eq!(summary.tags_replaced, 1);
        assert_eq!(summary.rows_inserted, 0);

        let count: i64 = db
            .connection()
            .query_row(
                "SELECT COUNT(*) FROM card_tagging WHERE tag = 'burn'",
                [],
                |row| row.get(0),
            )
            .expect("count rows");
        assert_eq!(count, 0);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn build_local_tag_rows_filters_to_known_cards() {
        let local_cards = BTreeSet::from([
            "Abrade".to_string(),
            "Lightning Bolt".to_string(),
            "Wear // Tear".to_string(),
        ]);
        let tagged = vec![
            "Abrade".to_string(),
            "Missing Card".to_string(),
            "Wear / Tear".to_string(),
            "Abrade".to_string(),
        ];

        let rows = build_local_tag_rows("removal", &tagged, &local_cards);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].card_name, "Abrade");
        assert_eq!(rows[1].card_name, "Wear // Tear");
    }

    #[test]
    fn prune_cards_not_in_names_removes_compilations_and_tags() {
        let path = unique_temp_path("prune");
        let mut db = CardStatusDb::open(&path).expect("open db");

        let lightning = compile_snapshot_from_payload(&lightning_bolt_payload());
        let shock = compile_snapshot_from_payload(&CardPayload {
            name: "Shock".to_string(),
            oracle_text: "Shock deals 2 damage to any target.".to_string(),
            metadata_lines: vec!["Mana cost: {R}".to_string(), "Type: Instant".to_string()],
            parse_input: "Mana cost: {R}\nType: Instant\nShock deals 2 damage to any target."
                .to_string(),
        });

        db.insert_snapshot_if_changed(&lightning)
            .expect("insert lightning");
        db.insert_snapshot_if_changed(&shock).expect("insert shock");
        db.replace_tag_rows(&[
            TagImportRow {
                card_name: "Lightning Bolt".to_string(),
                tag: "burn".to_string(),
            },
            TagImportRow {
                card_name: "Shock".to_string(),
                tag: "burn".to_string(),
            },
        ])
        .expect("seed tags");

        let summary = db
            .prune_cards_not_in_names(&["Lightning Bolt".to_string()])
            .expect("prune cards");
        assert_eq!(summary.distinct_cards_deleted, 1);
        assert_eq!(summary.compilation_rows_deleted, 1);
        assert_eq!(summary.tag_rows_deleted, 1);

        let remaining_cards: Vec<String> = {
            let mut stmt = db
                .connection()
                .prepare("SELECT card_name FROM latest_card_compilation ORDER BY card_name ASC")
                .expect("prepare remaining cards query");
            stmt.query_map([], |row| row.get(0))
                .expect("query remaining cards")
                .collect::<Result<_, _>>()
                .expect("collect remaining cards")
        };
        assert_eq!(remaining_cards, vec!["Lightning Bolt".to_string()]);

        let remaining_tag_rows: i64 = db
            .connection()
            .query_row(
                "SELECT COUNT(*) FROM card_tagging WHERE card_name = 'Shock'",
                [],
                |row| row.get(0),
            )
            .expect("count remaining shock tag rows");
        assert_eq!(remaining_tag_rows, 0);

        let _ = fs::remove_file(path);
    }

    #[test]
    fn prune_compilation_history_to_latest_keeps_only_latest_snapshot_per_card() {
        let path = unique_temp_path("history-prune");
        let mut db = CardStatusDb::open(&path).expect("open db");

        let first_lightning = compile_snapshot_from_payload(&lightning_bolt_payload());
        let mut latest_lightning = first_lightning.clone();
        latest_lightning.content_hash = "lightning-bolt-v2".to_string();
        latest_lightning.similarity_score = 0.25;

        let shock = compile_snapshot_from_payload(&CardPayload {
            name: "Shock".to_string(),
            oracle_text: "Shock deals 2 damage to any target.".to_string(),
            metadata_lines: vec!["Mana cost: {R}".to_string(), "Type: Instant".to_string()],
            parse_input: "Mana cost: {R}\nType: Instant\nShock deals 2 damage to any target."
                .to_string(),
        });

        db.insert_snapshot_if_changed(&first_lightning)
            .expect("insert first lightning snapshot");
        db.insert_snapshot_if_changed(&latest_lightning)
            .expect("insert latest lightning snapshot");
        db.insert_snapshot_if_changed(&shock)
            .expect("insert shock snapshot");

        let summary = db
            .prune_compilation_history_to_latest()
            .expect("prune compilation history");
        assert_eq!(summary.distinct_cards_retained, 2);
        assert_eq!(summary.compilation_rows_deleted, 1);

        let remaining_rows: Vec<(String, String)> = {
            let mut stmt = db
                .connection()
                .prepare(
                    "SELECT card_name, content_hash
                     FROM card_compilation
                     ORDER BY card_name ASC, id ASC",
                )
                .expect("prepare remaining compilation rows query");
            stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
                .expect("query remaining compilation rows")
                .collect::<Result<_, _>>()
                .expect("collect remaining compilation rows")
        };
        assert_eq!(
            remaining_rows,
            vec![
                (
                    "Lightning Bolt".to_string(),
                    "lightning-bolt-v2".to_string()
                ),
                ("Shock".to_string(), shock.content_hash.clone()),
            ]
        );

        let latest_hash = db
            .latest_snapshot_hash("Lightning Bolt")
            .expect("fetch latest lightning hash");
        assert_eq!(latest_hash.as_deref(), Some("lightning-bolt-v2"));

        let _ = fs::remove_file(path);
    }
}
