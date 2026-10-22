//! SQLite storage used by registry preflight and analysis tooling.
//!
//! The dependency boundary is intentional: this crate may depend on the
//! lightweight card-source model, but never on compiler or runtime code.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use ironsmith_card_source::{RegistryCardRecord, normalize_lookup_name, strip_parenthetical_text};
use rusqlite::{Connection, params};

pub const DEFAULT_DB_PATH: &str = "reports/engine-status.sqlite3";
const DB_SCHEMA_VERSION: i64 = 11;
// Version 12 adds per-face compilation snapshots without changing the registry
// tables. Analysis tools own that migration; new lightweight DBs remain at 11.
const MAX_COMPATIBLE_DB_SCHEMA_VERSION: i64 = 12;

#[derive(Debug)]
pub struct StatusDb {
    conn: Connection,
}

#[derive(Debug, Clone, Copy)]
pub struct RegistrySyncSummary {
    pub inserted: usize,
    pub updated: usize,
    pub unchanged: usize,
    pub deleted: usize,
}

#[derive(Debug, Clone)]
pub struct MissingRegistrySyncSummary {
    pub inserted_names: Vec<String>,
    pub unchanged: usize,
}

#[derive(Debug, Clone)]
pub struct RegistryTextSyncSummary {
    pub inserted_names: Vec<String>,
    pub updated_names: Vec<String>,
    pub unchanged: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct CardPruneSummary {
    pub distinct_cards_deleted: usize,
    pub compilation_rows_deleted: usize,
    pub tag_rows_deleted: usize,
}

pub fn default_db_path() -> PathBuf {
    PathBuf::from(DEFAULT_DB_PATH)
}

fn read_sqlite_count(row: &rusqlite::Row<'_>) -> rusqlite::Result<usize> {
    let count = row.get::<_, i64>(0)?;
    usize::try_from(count).map_err(|err| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Integer, Box::new(err))
    })
}

impl StatusDb {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, Box<dyn Error>> {
        let path = path.as_ref();
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        conn.busy_timeout(Duration::from_secs(60))?;
        let db = Self { conn };
        db.initialize()?;
        Ok(db)
    }

    fn initialize(&self) -> Result<(), Box<dyn Error>> {
        let version: i64 = self
            .conn
            .pragma_query_value(None, "user_version", |row| row.get(0))?;
        if version > MAX_COMPATIBLE_DB_SCHEMA_VERSION {
            return Err(format!(
                "engine status DB schema version {version} is newer than supported {MAX_COMPATIBLE_DB_SCHEMA_VERSION}"
            )
            .into());
        }
        if version != 0 && version < DB_SCHEMA_VERSION {
            return Err(format!(
                "engine status DB schema version {version} requires migration; run the analysis-tools migration before the lightweight registry sync"
            )
            .into());
        }
        if version > DB_SCHEMA_VERSION {
            // Do not recreate analysis views or downgrade user_version when
            // opening a compatible schema owned by the analysis tools.
            return Ok(());
        }

        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS card_compilation (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                card_name TEXT NOT NULL,
                oracle_text TEXT NOT NULL,
                raw_oracle_text TEXT NOT NULL,
                parse_status TEXT NOT NULL,
                parse_error TEXT,
                normalized_oracle_text TEXT,
                compiled_text TEXT,
                compiled_card_definition TEXT,
                oracle_coverage REAL NOT NULL,
                compiled_coverage REAL NOT NULL,
                similarity_score REAL NOT NULL,
                line_delta INTEGER NOT NULL,
                semantic_mismatch INTEGER NOT NULL,
                has_unimplemented INTEGER NOT NULL,
                parse_lossy INTEGER NOT NULL DEFAULT 0,
                parse_loss_reasons TEXT NOT NULL DEFAULT '',
                parse_loss_count INTEGER NOT NULL DEFAULT 0,
                compiled_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
                content_hash TEXT NOT NULL,
                UNIQUE(card_name, content_hash)
            );
            CREATE INDEX IF NOT EXISTS idx_card_compilation_name_compiled_at
                ON card_compilation(card_name, compiled_at DESC);
            CREATE TABLE IF NOT EXISTS latest_card_observation (
                card_name TEXT PRIMARY KEY,
                compilation_id INTEGER NOT NULL,
                agent_running INTEGER NOT NULL DEFAULT 0,
                pr_created INTEGER NOT NULL DEFAULT 0
            );
            CREATE TABLE IF NOT EXISTS card_tagging (
                card_name TEXT NOT NULL,
                tag TEXT NOT NULL,
                UNIQUE(card_name, tag)
            );
            CREATE INDEX IF NOT EXISTS idx_card_tagging_tag_card_name
                ON card_tagging(tag, card_name);
            CREATE TABLE IF NOT EXISTS oracle_tag (tag TEXT PRIMARY KEY);
            CREATE TABLE IF NOT EXISTS registry_card (
                card_name TEXT PRIMARY KEY,
                oracle_text TEXT NOT NULL,
                raw_oracle_text TEXT NOT NULL,
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
            SELECT cc.*, latest.agent_running, latest.pr_created
            FROM latest_card_observation latest
            JOIN card_compilation cc ON cc.id = latest.compilation_id;",
        )?;
        self.conn
            .pragma_update(None, "user_version", DB_SCHEMA_VERSION)?;
        Ok(())
    }

    pub fn replace_registry_cards(
        &mut self,
        rows: &[RegistryCardRecord],
    ) -> Result<RegistrySyncSummary, Box<dyn Error>> {
        let normalized_rows = normalize_rows(rows)?;
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
            .map(|row| row.name.clone())
            .collect::<BTreeSet<_>>();
        let mut summary = RegistrySyncSummary {
            inserted: 0,
            updated: 0,
            unchanged: 0,
            deleted: 0,
        };
        {
            let mut upsert = tx.prepare(&registry_upsert_sql())?;
            for row in &normalized_rows {
                match existing_hashes.get(&row.name) {
                    None => summary.inserted += 1,
                    Some(hash) if hash == &row.content_hash => summary.unchanged += 1,
                    Some(_) => summary.updated += 1,
                }
                execute_registry_write(&mut upsert, row)?;
            }
        }

        tx.execute_batch(
            "DROP TABLE IF EXISTS temp_allowed_registry_card;
             CREATE TEMP TABLE temp_allowed_registry_card (card_name TEXT PRIMARY KEY);",
        )?;
        {
            let mut insert = tx.prepare(
                "INSERT OR IGNORE INTO temp_allowed_registry_card(card_name) VALUES (?1)",
            )?;
            for name in &allowed_names {
                insert.execute([name])?;
            }
        }
        summary.deleted = tx.query_row(
            "SELECT COUNT(*) FROM registry_card WHERE NOT EXISTS (
                SELECT 1 FROM temp_allowed_registry_card allowed
                WHERE allowed.card_name = registry_card.card_name)",
            [],
            read_sqlite_count,
        )?;
        tx.execute(
            "DELETE FROM registry_card WHERE NOT EXISTS (
                SELECT 1 FROM temp_allowed_registry_card allowed
                WHERE allowed.card_name = registry_card.card_name)",
            [],
        )?;
        tx.execute("DROP TABLE temp_allowed_registry_card", [])?;
        tx.commit()?;
        Ok(summary)
    }

    pub fn insert_missing_registry_cards(
        &mut self,
        rows: &[RegistryCardRecord],
    ) -> Result<MissingRegistrySyncSummary, Box<dyn Error>> {
        let normalized_rows = normalize_rows(rows)?;
        let tx = self.conn.transaction()?;
        let mut existing_names = BTreeSet::new();
        {
            let mut stmt = tx.prepare("SELECT card_name FROM registry_card")?;
            let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
            for row in rows {
                existing_names.insert(row?);
            }
        }

        let mut inserted_names = Vec::new();
        let mut unchanged = 0usize;
        {
            let mut insert = tx.prepare(&registry_insert_sql())?;
            for row in &normalized_rows {
                if existing_names.contains(&row.name) {
                    unchanged += 1;
                    continue;
                }
                execute_registry_write(&mut insert, row)?;
                existing_names.insert(row.name.clone());
                inserted_names.push(row.name.clone());
            }
        }
        tx.commit()?;
        Ok(MissingRegistrySyncSummary {
            inserted_names,
            unchanged,
        })
    }

    /// Inserts missing cards and rewrites existing rows whose compile-relevant
    /// text changed (oracle errata, type line, P/T, ...). Never prunes. Rows
    /// that differ only in volatile Scryfall metadata (prices, image URLs) are
    /// left alone, so a fresh download doesn't rewrite the whole registry.
    pub fn sync_registry_card_text_changes(
        &mut self,
        rows: &[RegistryCardRecord],
    ) -> Result<RegistryTextSyncSummary, Box<dyn Error>> {
        let normalized_rows = normalize_rows(rows)?;
        let tx = self.conn.transaction()?;
        let mut existing = BTreeMap::new();
        {
            let mut stmt = tx.prepare(
                "SELECT card_name, oracle_text, raw_oracle_text, parse_input, mana_cost,
                        type_line, power, toughness, loyalty, defense, layout
                 FROM registry_card",
            )?;
            let rows = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    registry_text_fingerprint(
                        &row.get::<_, String>(1)?,
                        &row.get::<_, String>(2)?,
                        &row.get::<_, String>(3)?,
                        [
                            row.get::<_, Option<String>>(4)?,
                            row.get::<_, Option<String>>(5)?,
                            row.get::<_, Option<String>>(6)?,
                            row.get::<_, Option<String>>(7)?,
                            row.get::<_, Option<String>>(8)?,
                            row.get::<_, Option<String>>(9)?,
                            row.get::<_, Option<String>>(10)?,
                        ],
                    ),
                ))
            })?;
            for row in rows {
                let (name, fingerprint) = row?;
                existing.insert(name, fingerprint);
            }
        }

        let mut summary = RegistryTextSyncSummary {
            inserted_names: Vec::new(),
            updated_names: Vec::new(),
            unchanged: 0,
        };
        {
            let mut upsert = tx.prepare(&registry_upsert_sql())?;
            for row in &normalized_rows {
                let fingerprint = registry_text_fingerprint(
                    &strip_parenthetical_text(&row.oracle_text),
                    &row.raw_oracle_text,
                    &row.parse_input,
                    [
                        row.mana_cost.clone(),
                        row.type_line.clone(),
                        row.power.clone(),
                        row.toughness.clone(),
                        row.loyalty.clone(),
                        row.defense.clone(),
                        row.layout.clone(),
                    ],
                );
                match existing.get(&row.name) {
                    Some(current) if current == &fingerprint => {
                        summary.unchanged += 1;
                        continue;
                    }
                    Some(_) => summary.updated_names.push(row.name.clone()),
                    None => summary.inserted_names.push(row.name.clone()),
                }
                execute_registry_write(&mut upsert, row)?;
                existing.insert(row.name.clone(), fingerprint);
            }
        }
        tx.commit()?;
        Ok(summary)
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
             CREATE TEMP TABLE temp_allowed_card_name (card_name TEXT PRIMARY KEY);",
        )?;
        {
            let mut insert =
                tx.prepare("INSERT OR IGNORE INTO temp_allowed_card_name(card_name) VALUES (?1)")?;
            for name in &allowed_names {
                insert.execute([name])?;
            }
        }
        let compilation_rows_deleted = count_not_allowed(&tx, "card_compilation", false)?;
        let distinct_cards_deleted = count_not_allowed(&tx, "card_compilation", true)?;
        let tag_rows_deleted = count_not_allowed(&tx, "card_tagging", false)?;
        for table in [
            "latest_card_observation",
            "card_tagging",
            "card_compilation",
        ] {
            tx.execute(
                &format!(
                    "DELETE FROM {table} WHERE NOT EXISTS (
                        SELECT 1 FROM temp_allowed_card_name allowed
                        WHERE allowed.card_name = {table}.card_name)"
                ),
                [],
            )?;
        }
        tx.execute("DROP TABLE temp_allowed_card_name", [])?;
        tx.commit()?;
        Ok(CardPruneSummary {
            distinct_cards_deleted,
            compilation_rows_deleted,
            tag_rows_deleted,
        })
    }
}

/// The registry columns that card compilation reads, joined for comparison.
/// `oracle_text` is passed in its stored (reminder-stripped) form.
fn registry_text_fingerprint(
    oracle_text: &str,
    raw_oracle_text: &str,
    parse_input: &str,
    fields: [Option<String>; 7],
) -> String {
    let mut parts = vec![
        oracle_text.to_string(),
        raw_oracle_text.to_string(),
        parse_input.to_string(),
    ];
    parts.extend(fields.into_iter().map(Option::unwrap_or_default));
    parts.join("\u{0}")
}

fn normalize_rows(rows: &[RegistryCardRecord]) -> Result<Vec<RegistryCardRecord>, Box<dyn Error>> {
    let rows = rows
        .iter()
        .filter_map(|row| {
            let name = normalize_lookup_name(&row.name);
            if name.is_empty() {
                return None;
            }
            let mut row = row.clone();
            row.name = name;
            Some(row)
        })
        .collect::<Vec<_>>();
    if rows.is_empty() {
        return Err("refusing to sync registry cards with an empty row set".into());
    }
    Ok(rows)
}

fn registry_columns() -> &'static str {
    "card_name, oracle_text, raw_oracle_text, parse_input, raw_card_json,
     mana_cost, type_line, power, toughness, loyalty, defense, layout, content_hash, updated_at"
}

fn registry_insert_sql() -> String {
    format!(
        "INSERT INTO registry_card ({}) VALUES
         (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13,
          strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
        registry_columns()
    )
}

fn registry_upsert_sql() -> String {
    format!(
        "{} ON CONFLICT(card_name) DO UPDATE SET
         oracle_text=excluded.oracle_text, raw_oracle_text=excluded.raw_oracle_text,
         parse_input=excluded.parse_input, raw_card_json=excluded.raw_card_json,
         mana_cost=excluded.mana_cost, type_line=excluded.type_line,
         power=excluded.power, toughness=excluded.toughness, loyalty=excluded.loyalty,
         defense=excluded.defense, layout=excluded.layout, content_hash=excluded.content_hash,
         updated_at=strftime('%Y-%m-%dT%H:%M:%fZ', 'now')",
        registry_insert_sql()
    )
}

fn execute_registry_write(
    stmt: &mut rusqlite::Statement<'_>,
    row: &RegistryCardRecord,
) -> rusqlite::Result<usize> {
    stmt.execute(params![
        row.name,
        strip_parenthetical_text(&row.oracle_text),
        row.raw_oracle_text,
        row.parse_input,
        row.raw_card_json,
        row.mana_cost,
        row.type_line,
        row.power,
        row.toughness,
        row.loyalty,
        row.defense,
        row.layout,
        row.content_hash,
    ])
}

fn count_not_allowed(
    conn: &Connection,
    table: &str,
    distinct: bool,
) -> Result<usize, Box<dyn Error>> {
    let count = if distinct {
        "COUNT(DISTINCT card_name)"
    } else {
        "COUNT(*)"
    };
    Ok(conn.query_row(
        &format!(
            "SELECT {count} FROM {table} WHERE NOT EXISTS (
                SELECT 1 FROM temp_allowed_card_name allowed
                WHERE allowed.card_name = {table}.card_name)"
        ),
        [],
        read_sqlite_count,
    )?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compatible_schema_preserves_version_and_analysis_objects() {
        let mut db = StatusDb::open(":memory:").unwrap();
        db.conn
            .execute_batch(
                "CREATE TABLE card_face_compilation (
                compilation_id INTEGER NOT NULL,
                face_index INTEGER NOT NULL,
                snapshot TEXT NOT NULL CHECK(json_valid(snapshot)),
                PRIMARY KEY (compilation_id, face_index)
             );
             INSERT INTO card_face_compilation VALUES (1, 0, '{}');
             PRAGMA user_version = 12;",
            )
            .unwrap();
        let schema_before: String = db
            .conn
            .query_row("SELECT group_concat(sql) FROM sqlite_master", [], |row| {
                row.get(0)
            })
            .unwrap();
        db.initialize().unwrap();
        let row = RegistryCardRecord {
            name: "Test Card".into(),
            oracle_text: "Draw a card.".into(),
            raw_oracle_text: "Draw a card.".into(),
            parse_input: "Draw a card.".into(),
            raw_card_json: "{}".into(),
            mana_cost: None,
            type_line: None,
            power: None,
            toughness: None,
            loyalty: None,
            defense: None,
            layout: None,
            content_hash: "test".into(),
        };
        let result = db.sync_registry_card_text_changes(&[row]).unwrap();
        assert_eq!(result.inserted_names, ["Test Card"]);
        let version: i64 = db
            .conn
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();
        assert_eq!(version, 12);
        let schema_after: String = db
            .conn
            .query_row("SELECT group_concat(sql) FROM sqlite_master", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(schema_before, schema_after);
        let snapshot: String = db
            .conn
            .query_row("SELECT snapshot FROM card_face_compilation", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(snapshot, "{}");
    }

    #[test]
    fn unsupported_schemas_are_rejected_without_changing_version() {
        for (version, message) in [(10, "requires migration"), (13, "newer than supported 12")] {
            let db = StatusDb {
                conn: Connection::open_in_memory().unwrap(),
            };
            db.conn
                .pragma_update(None, "user_version", version)
                .unwrap();
            assert!(db.initialize().unwrap_err().to_string().contains(message));
            let after: i64 = db
                .conn
                .pragma_query_value(None, "user_version", |row| row.get(0))
                .unwrap();
            assert_eq!(after, version);
        }
    }
}
