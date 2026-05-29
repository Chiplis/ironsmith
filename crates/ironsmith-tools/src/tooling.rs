use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::error::Error;
use std::fmt;
use std::fs;
use std::panic::{self, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::time::Duration;

use csv::StringRecord;
use reqwest::blocking::Client;
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::Value;
use sha2::{Digest, Sha256};

use ironsmith::card::LinkedFaceLayout;
use ironsmith::cards::{
    CardDefinition, generated_definition_has_unimplemented_content,
    generated_definition_unsupported_mechanics_message,
};
use ironsmith::compiled_text::compiled_text_lines;
use ironsmith::ids::CardId;
use ironsmith::semantic_compare::{compare_card_semantics_scored, report_embedding_config};
use ironsmith_compiler::{
    CardDefinitionBuilder as CompilerCardDefinitionBuilder, parse_loss, parse_trace,
};

pub const DEFAULT_DB_PATH: &str = "reports/engine-status.sqlite3";
pub const SCRYFALL_TAGGER_TAGS_URL: &str = "https://scryfall.com/docs/tagger-tags";
pub const TAGGER_BASE_URL: &str = "https://tagger.scryfall.com";
const DB_SCHEMA_VERSION: i64 = 11;
const FIXED_SNAPSHOT_CARD_ID: u32 = 1;
const SUPPORTED_PAPER_FORMATS: &[&str] = &[
    "commander",
    "standard",
    "modern",
    "pioneer",
    "legacy",
    "vintage",
];
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

fn read_sqlite_count(row: &rusqlite::Row<'_>) -> rusqlite::Result<usize> {
    let count = row.get::<_, i64>(0)?;
    usize::try_from(count).map_err(|err| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Integer, Box::new(err))
    })
}

fn strip_parenthetical_text(text: &str) -> String {
    text.lines()
        .map(strip_parenthetical_text_from_line)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn strip_parenthetical_text_from_line(line: &str) -> String {
    let mut stripped = String::with_capacity(line.len());
    let mut depth = 0usize;

    for ch in line.chars() {
        match ch {
            '(' => {
                if depth == 0 {
                    while stripped.ends_with([' ', '\t']) {
                        stripped.pop();
                    }
                }
                depth += 1;
            }
            ')' => {
                depth = depth.saturating_sub(1);
            }
            _ if depth == 0 => stripped.push(ch),
            _ => {}
        }
    }

    tighten_spacing(&stripped)
}

fn tighten_spacing(line: &str) -> String {
    let collapsed = line
        .replace('\u{00a0}', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let mut tightened = String::with_capacity(collapsed.len());
    for ch in collapsed.chars() {
        if matches!(ch, '.' | ',' | ';' | ':' | '!' | '?') && tightened.ends_with(' ') {
            tightened.pop();
        }
        tightened.push(ch);
    }
    tightened.trim().to_string()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CardPayload {
    pub name: String,
    pub parse_name: Option<String>,
    pub oracle_text: String,
    pub raw_oracle_text: String,
    pub metadata_lines: Vec<String>,
    pub parse_input: String,
    pub other_face_name: Option<String>,
    pub linked_face_layout: Option<LinkedFaceLayout>,
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
    pub parse_loss: parse_loss::ParseLossReport,
}

#[derive(Debug, Clone)]
pub struct CompilationSnapshot {
    pub card_name: String,
    pub oracle_text: String,
    pub raw_oracle_text: String,
    pub parse_status: ParseStatus,
    pub parse_error: Option<String>,
    pub normalized_oracle_text: String,
    pub compiled_text: Option<String>,
    pub compiled_card_definition: Option<String>,
    pub oracle_coverage: f32,
    pub compiled_coverage: f32,
    pub similarity_score: f32,
    pub line_delta: isize,
    pub semantic_mismatch: bool,
    pub has_unimplemented: bool,
    pub parse_lossy: bool,
    pub parse_loss_reasons: String,
    pub parse_loss_count: usize,
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

pub fn default_cards_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")))
        .join("cards.json")
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

pub fn postprocess_oracle_text(text: &str) -> String {
    strip_parenthetical_text(text)
}

fn normalized_oracle_source_lines(text: &str) -> Vec<String> {
    strip_parenthetical_text(text)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(normalize_canonical_oracle_line)
        .collect()
}

fn normalize_canonical_oracle_line(line: &str) -> String {
    line.replace(
        "At the beginning of each player's end step,",
        "At the beginning of each end step,",
    )
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

pub fn load_registry_cards_with_explicit_includes(
    path: &str,
    included_names: &BTreeSet<String>,
) -> Result<BTreeMap<String, RegistryCardRecord>, Box<dyn Error>> {
    load_registry_cards_with_explicit_includes_and_cards(path, included_names, Vec::new())
}

pub fn load_registry_cards_with_explicit_includes_and_cards(
    path: &str,
    included_names: &BTreeSet<String>,
    extra_cards: Vec<Value>,
) -> Result<BTreeMap<String, RegistryCardRecord>, Box<dyn Error>> {
    let raw = fs::read_to_string(path)?;
    let mut cards: Vec<Value> = serde_json::from_str(&raw)?;
    let mut included_names = included_names
        .iter()
        .map(|name| normalize_lookup_name(name))
        .collect::<BTreeSet<_>>();
    for card in extra_cards {
        if let Some(name) = supplemental_card_name(&card) {
            included_names.insert(name);
        }
        cards.push(card);
    }
    Ok(load_registry_cards_from_values_with_explicit_includes(
        cards.into_iter(),
        &included_names,
    ))
}

fn supplemental_card_name(card: &Value) -> Option<String> {
    let face = get_first_face(card);
    let name = normalize_lookup_name(&pick_field(card, face, "name")?);
    (!name.is_empty()).then_some(name)
}

pub fn load_card_by_name(path: &str, name: &str) -> Result<Option<CardPayload>, Box<dyn Error>> {
    let cards = load_canonical_cards(path)?;
    let normalized = normalize_lookup_name(name);
    Ok(cards.get(&normalized).cloned())
}

pub fn load_card_payloads_by_name(
    path: &str,
    name: &str,
) -> Result<Vec<CardPayload>, Box<dyn Error>> {
    let raw = fs::read_to_string(path)?;
    let cards: Vec<Value> = serde_json::from_str(&raw)?;
    let normalized = normalize_lookup_name(name);

    for card in &cards {
        if card
            .get("digital")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            continue;
        }

        let card_name = card
            .get("name")
            .and_then(Value::as_str)
            .map(normalize_lookup_name);
        let card_name_matches = card_name.as_deref() == Some(normalized.as_str());
        let supported = card_is_legal_in_supported_paper_format(card);
        let face_match_indexes = matching_face_indexes(card, &normalized);
        if !supported && !card_name_matches && face_match_indexes.is_empty() {
            continue;
        }

        if card_name_matches {
            if linked_face_layout_from_card(card).is_some()
                && let Some(faces) = card.get("card_faces").and_then(Value::as_array)
            {
                let payloads = (0..faces.len())
                    .filter_map(|idx| build_card_payload_for_face(card, idx))
                    .collect::<Vec<_>>();
                if !payloads.is_empty() {
                    return Ok(payloads);
                }
            }
            if let Some(record) = build_registry_card_record_with_explicit_includes(
                card,
                &BTreeSet::new(),
            ) {
                return Ok(vec![record.payload]);
            }
        }

        if !face_match_indexes.is_empty() {
            let payloads = face_match_indexes
                .into_iter()
                .filter_map(|idx| build_card_payload_for_face(card, idx))
                .collect::<Vec<_>>();
            if !payloads.is_empty() {
                return Ok(payloads);
            }
        }
    }

    Ok(Vec::new())
}

pub fn parse_card_with_fallback(name: &str, parse_input: &str) -> ParseAttempt {
    let strict_attempt = parse_card(name, parse_input, false);
    if strict_attempt.status == ParseStatus::StrictCompiled {
        return strict_attempt;
    }
    let strict_error = strict_attempt.parse_error.clone();
    let allow_attempt = parse_card(name, parse_input, true);
    if allow_attempt.status == ParseStatus::StrictCompiled {
        let mut parse_loss = allow_attempt.parse_loss;
        parse_loss.push_reason(
            "allow_unsupported_fallback",
            strict_error
                .as_deref()
                .unwrap_or("strict parse failed before allow-unsupported fallback"),
        );
        return ParseAttempt {
            status: ParseStatus::CompiledWithAllowUnsupported,
            parse_error: None,
            definition: allow_attempt.definition,
            parse_loss,
        };
    }
    ParseAttempt {
        status: ParseStatus::ParseFailed,
        parse_error: strict_error,
        definition: None,
        parse_loss: strict_attempt.parse_loss,
    }
}

pub fn compile_snapshot_from_payload(payload: &CardPayload) -> CompilationSnapshot {
    let attempt = parse_card_payload_with_fallback(payload);
    snapshot_from_attempt(payload, &attempt)
}

pub fn compile_strict_snapshot_from_payload(payload: &CardPayload) -> CompilationSnapshot {
    let attempt = parse_card_payload(payload, false);
    snapshot_from_attempt(payload, &attempt)
}

pub fn compile_authoritative_snapshot_from_payload(payload: &CardPayload) -> CompilationSnapshot {
    let attempt = parse_card_payload_with_fallback(payload);
    let unsupported_mechanics_message = attempt
        .definition
        .as_ref()
        .and_then(generated_definition_unsupported_mechanics_message);
    let mut snapshot = snapshot_from_attempt(payload, &attempt);
    if snapshot.parse_status == ParseStatus::StrictCompiled && snapshot.has_unimplemented {
        mark_authoritative_snapshot_parse_failed(
            &mut snapshot,
            unsupported_mechanics_message.unwrap_or_else(|| {
                "generated definition still contains unimplemented content".to_string()
            }),
        );
    } else if let Some(parse_error) = authoritative_semantic_marker_parse_error(&snapshot) {
        mark_authoritative_snapshot_parse_failed(&mut snapshot, parse_error);
    }
    snapshot
}

fn mark_authoritative_snapshot_parse_failed(
    snapshot: &mut CompilationSnapshot,
    parse_error: String,
) {
    snapshot.parse_status = ParseStatus::ParseFailed;
    snapshot.parse_error = Some(parse_error);
    snapshot.compiled_text = None;
    snapshot.compiled_card_definition = None;
    snapshot.oracle_coverage = 0.0;
    snapshot.compiled_coverage = 0.0;
    snapshot.similarity_score = 0.0;
    snapshot.line_delta = 0;
    snapshot.semantic_mismatch = false;
    snapshot.content_hash = snapshot.compute_content_hash();
}

fn authoritative_semantic_marker_parse_error(snapshot: &CompilationSnapshot) -> Option<String> {
    if snapshot.parse_status != ParseStatus::StrictCompiled {
        return None;
    }
    let compiled_text = snapshot.compiled_text.as_deref()?;
    let oracle = snapshot.normalized_oracle_text.to_ascii_lowercase();
    let compiled = compiled_text.to_ascii_lowercase();

    let internal_markers = [
        ("valuecomparison {", "value-comparison-debug"),
        ("tagged '", "tagged-object-reference"),
        ("tagged object '", "tagged-object-reference"),
        ("that object matches", "object-predicate-debug"),
    ];
    for (marker, label) in internal_markers {
        if compiled.contains(marker) {
            return Some(format!("compiled text contains internal marker: {label}"));
        }
    }

    let malformed_markers = [
        ("whenever target ", "malformed-whenever-target"),
        ("whenever destroy ", "malformed-whenever-destroy"),
        ("whenever reveal ", "malformed-whenever-reveal"),
        ("whenever as long as ", "malformed-whenever-as-long-as"),
        ("this spell token", "malformed-spell-token"),
        ("you may if ", "malformed-conditional-permission"),
        ("for each opponent,.", "malformed-empty-per-opponent-effect"),
        ("permanents loses", "malformed-permanents-loses"),
    ];
    for (marker, label) in malformed_markers {
        if compiled.contains(marker) {
            return Some(format!("compiled text contains malformed output: {label}"));
        }
    }

    let dropped_required_all = [
        (
            "noncreature card exiled this way",
            ["noncreature", "this way"].as_slice(),
            "noncreature-exiled-this-way",
        ),
        (
            "note one of its creature types",
            ["note"].as_slice(),
            "noted-creature-types",
        ),
        ("noted for", ["noted"].as_slice(), "noted-creature-types"),
        (
            "controlled by the same opponent",
            ["same opponent"].as_slice(),
            "same-opponent-controlled",
        ),
        (
            "that player chooses one of those creatures",
            ["chooses"].as_slice(),
            "that-player-chooses",
        ),
        (
            "deals 5 damage to that creature",
            ["deals 5 damage"].as_slice(),
            "damage-to-chosen-creature",
        ),
        (
            "target instant, sorcery, or artifact card from your graveyard",
            ["target", "from your graveyard"].as_slice(),
            "target-card-from-your-graveyard",
        ),
        (
            "spell cast this way would be put into your graveyard",
            ["cast this way", "graveyard", "exile"].as_slice(),
            "cast-this-way-replacement",
        ),
        (
            "number of equipment attached",
            ["equipment attached"].as_slice(),
            "equipment-attached-count",
        ),
        (
            "goaded for the rest of the game",
            ["goaded", "rest of the game"].as_slice(),
            "goaded-rest-of-game",
        ),
        (
            "until they exile a nonland card",
            ["until", "nonland"].as_slice(),
            "exile-until-nonland",
        ),
        (
            "nonland cards exiled this way",
            ["nonland", "exiled this way"].as_slice(),
            "nonland-cards-exiled-this-way",
        ),
        (
            "had a counter put on them this way",
            ["counter", "this way"].as_slice(),
            "counter-put-this-way",
        ),
        (
            "as you activate this ability",
            ["as you activate this ability"].as_slice(),
            "as-you-activate-this-ability",
        ),
        (
            "target creature you control deals damage equal to its power",
            [
                "target creature you control",
                "deals damage equal to its power",
            ]
            .as_slice(),
            "target-creature-power-damage-source",
        ),
    ];
    for (oracle_marker, compiled_markers, label) in dropped_required_all {
        if oracle.contains(oracle_marker)
            && !compiled_markers
                .iter()
                .all(|marker| compiled.contains(marker))
        {
            return Some(format!(
                "compiled text dropped required semantic marker: {label}"
            ));
        }
    }

    let required_marker_groups = [
        (["at random"].as_slice(), ["random"].as_slice(), "at-random"),
        (
            ["command zone"].as_slice(),
            ["command zone"].as_slice(),
            "command-zone",
        ),
        (
            ["same name"].as_slice(),
            ["same name", "that name", "different name"].as_slice(),
            "same-name",
        ),
        (["instead"].as_slice(), ["instead"].as_slice(), "instead"),
        (
            ["rather than"].as_slice(),
            ["rather than"].as_slice(),
            "rather-than",
        ),
        (
            ["as though"].as_slice(),
            ["as though"].as_slice(),
            "as-though",
        ),
        (
            ["without paying"].as_slice(),
            ["without paying"].as_slice(),
            "without-paying",
        ),
        (
            ["only once"].as_slice(),
            ["only once"].as_slice(),
            "only-once",
        ),
        (
            ["additional cost"].as_slice(),
            ["additional cost"].as_slice(),
            "additional-cost",
        ),
        (
            ["spend this mana only"].as_slice(),
            ["spend this mana only"].as_slice(),
            "spend-this-mana-only",
        ),
        (["crew"].as_slice(), ["crew"].as_slice(), "crew"),
        (
            ["roll ", "dice"].as_slice(),
            ["roll", "die", "dice"].as_slice(),
            "roll-die",
        ),
        (
            ["can't be blocked"].as_slice(),
            ["can't be blocked", "unblockable"].as_slice(),
            "cant-be-blocked",
        ),
        (
            ["card named"].as_slice(),
            ["named"].as_slice(),
            "card-named",
        ),
        (
            ["no creatures with decayed"].as_slice(),
            ["no creatures with decayed"].as_slice(),
            "no-creatures-with-decayed",
        ),
        (
            ["each basic land type"].as_slice(),
            ["each basic land type"].as_slice(),
            "each-basic-land-type",
        ),
        (
            ["more cards in hand than"].as_slice(),
            ["more cards in hand than"].as_slice(),
            "more-cards-in-hand-than",
        ),
        (
            ["each opponent loses"].as_slice(),
            ["each opponent loses"].as_slice(),
            "each-opponent-loses",
        ),
        (
            ["you gain x life", "you gain 3 life"].as_slice(),
            ["you gain"].as_slice(),
            "you-gain-life",
        ),
        (
            ["reveals their hand", "reveals that player hand"].as_slice(),
            ["reveals their hand", "reveals that player hand"].as_slice(),
            "reveals-hand",
        ),
        (
            ["second spell you cast each turn"].as_slice(),
            ["second spell"].as_slice(),
            "second-spell-each-turn",
        ),
        (
            ["put into exile from the battlefield"].as_slice(),
            ["exile"].as_slice(),
            "put-into-exile-from-battlefield",
        ),
        (
            ["attacked or blocked this turn"].as_slice(),
            ["attacked or blocked this turn"].as_slice(),
            "attacked-or-blocked-this-turn",
        ),
        (
            ["one or more cards leave your graveyard"].as_slice(),
            ["one or more cards leave your graveyard"].as_slice(),
            "cards-leave-your-graveyard",
        ),
        (
            ["one or more {e}"].as_slice(),
            ["one or more {e}", "at least one {e}"].as_slice(),
            "one-or-more-energy",
        ),
        (
            ["enter as a copy"].as_slice(),
            ["enter as a copy"].as_slice(),
            "enter-as-copy",
        ),
    ];
    for (oracle_markers, compiled_markers, label) in required_marker_groups {
        if oracle_markers.iter().any(|marker| oracle.contains(marker))
            && !compiled_markers
                .iter()
                .any(|marker| compiled.contains(marker))
        {
            return Some(format!(
                "compiled text dropped required semantic marker: {label}"
            ));
        }
    }

    let guarded_markers = [
        (
            ["shares a card type", "share a card type"].as_slice(),
            ["shares a card type", "share a card type"].as_slice(),
            "shares-a-card-type",
        ),
        (
            ["card type among", "card types among"].as_slice(),
            [
                "card type among",
                "card types among",
                "number of distinct card types in",
                "number of card types in",
            ]
            .as_slice(),
            "card-types-among",
        ),
    ];

    for (oracle_markers, compiled_markers, label) in guarded_markers {
        if oracle_markers.iter().any(|marker| oracle.contains(marker))
            && !compiled_markers
                .iter()
                .any(|marker| compiled.contains(marker))
        {
            return Some(format!(
                "compiled text dropped required semantic marker: {label}"
            ));
        }
    }

    None
}

pub fn snapshot_from_attempt(payload: &CardPayload, attempt: &ParseAttempt) -> CompilationSnapshot {
    let mut snapshot = CompilationSnapshot::from_definition_result(
        payload.parse_name.as_deref().unwrap_or(&payload.name),
        &payload.oracle_text,
        attempt.status,
        attempt.parse_error.clone(),
        attempt.definition.as_ref(),
        &attempt.parse_loss,
    );
    snapshot.card_name = payload.name.clone();
    snapshot.raw_oracle_text = payload.raw_oracle_text.clone();
    snapshot.content_hash = snapshot.compute_content_hash();
    snapshot
}

pub fn snapshot_from_payload_definition(
    payload: &CardPayload,
    definition: &CardDefinition,
) -> CompilationSnapshot {
    let mut snapshot = CompilationSnapshot::from_definition_result(
        payload.parse_name.as_deref().unwrap_or(&payload.name),
        &payload.oracle_text,
        ParseStatus::StrictCompiled,
        None,
        Some(definition),
        &parse_loss::ParseLossReport::default(),
    );
    snapshot.card_name = payload.name.clone();
    snapshot.raw_oracle_text = payload.raw_oracle_text.clone();
    snapshot.content_hash = snapshot.compute_content_hash();
    snapshot
}

impl CompilationSnapshot {
    pub fn from_definition_result(
        card_name: &str,
        oracle_text: &str,
        parse_status: ParseStatus,
        parse_error: Option<String>,
        definition: Option<&CardDefinition>,
        parse_loss: &parse_loss::ParseLossReport,
    ) -> Self {
        let stored_oracle_text = strip_parenthetical_text(oracle_text);
        let (
            normalized_oracle_text,
            compiled_text,
            compiled_card_definition,
            oracle_coverage,
            compiled_coverage,
            similarity_score,
            line_delta,
            semantic_mismatch,
            has_unimplemented,
        ) = if let Some(definition) = definition {
            let normalized_oracle = normalized_oracle_source_lines(oracle_text);
            let normalized_oracle_text = normalized_oracle.join("\n");
            let compiled = compiled_text_lines(definition);
            let compiled_text = compiled.join("\n");
            let (
                oracle_coverage,
                compiled_coverage,
                similarity_score,
                line_delta,
                semantic_mismatch,
            ) = compare_card_semantics_scored(
                card_name,
                &normalized_oracle_text,
                &compiled,
                report_embedding_config(),
            );
            (
                normalized_oracle_text,
                Some(compiled_text),
                Some(stable_compiled_definition_snapshot(definition)),
                oracle_coverage,
                compiled_coverage,
                similarity_score,
                line_delta,
                semantic_mismatch,
                generated_definition_has_unimplemented_content(definition)
                    || compiled
                        .iter()
                        .any(|line| line.to_ascii_lowercase().contains("unsupported effect")),
            )
        } else {
            (
                stored_oracle_text.clone(),
                None,
                None,
                0.0,
                0.0,
                0.0,
                0,
                false,
                false,
            )
        };

        let mut snapshot = Self {
            card_name: card_name.to_string(),
            oracle_text: stored_oracle_text,
            raw_oracle_text: oracle_text.to_string(),
            parse_status,
            parse_error: parse_error.map(|error| normalize_debug_card_ids(&error)),
            normalized_oracle_text,
            compiled_text,
            compiled_card_definition,
            oracle_coverage,
            compiled_coverage,
            similarity_score,
            line_delta,
            semantic_mismatch,
            has_unimplemented,
            parse_lossy: parse_loss.is_lossy(),
            parse_loss_reasons: parse_loss.reasons_text(),
            parse_loss_count: parse_loss.count(),
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
        hasher.update(self.raw_oracle_text.as_bytes());
        hasher.update([0]);
        hasher.update(self.parse_status.as_str().as_bytes());
        hasher.update([0]);
        hasher.update(self.parse_error.as_deref().unwrap_or("").as_bytes());
        hasher.update([0]);
        hasher.update(self.normalized_oracle_text.as_bytes());
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
        hasher.update([0]);
        hasher.update((self.parse_lossy as u8).to_string().as_bytes());
        hasher.update([0]);
        hasher.update(self.parse_loss_reasons.as_bytes());
        hasher.update([0]);
        hasher.update(self.parse_loss_count.to_string().as_bytes());
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
        conn.busy_timeout(Duration::from_secs(60))?;
        let db = Self { conn };
        db.initialize()?;
        Ok(db)
    }

    fn card_compilation_has_column(&self, column: &str) -> bool {
        self.conn
            .prepare(&format!("SELECT {column} FROM card_compilation LIMIT 0"))
            .is_ok()
    }

    fn drop_uses_pseudo_oracle_fallback_column_if_present(&self) -> Result<(), Box<dyn Error>> {
        if !self.card_compilation_has_column("uses_pseudo_oracle_fallback") {
            return Ok(());
        }

        self.conn
            .execute_batch("DROP VIEW IF EXISTS latest_card_compilation;")?;
        if self
            .conn
            .execute_batch(
                "ALTER TABLE card_compilation
                 DROP COLUMN uses_pseudo_oracle_fallback;",
            )
            .is_ok()
        {
            return Ok(());
        }

        self.conn.execute_batch(
            "ALTER TABLE card_compilation RENAME TO card_compilation_old;
             CREATE TABLE card_compilation (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                card_name TEXT NOT NULL,
                oracle_text TEXT NOT NULL,
                raw_oracle_text TEXT NOT NULL,
                parse_status TEXT NOT NULL,
                parse_error TEXT,
                compiled_text TEXT,
                unprocessed_compiled_text TEXT,
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
             INSERT INTO card_compilation (
                id,
                card_name,
                oracle_text,
                raw_oracle_text,
                parse_status,
                parse_error,
                compiled_text,
                unprocessed_compiled_text,
                compiled_card_definition,
                oracle_coverage,
                compiled_coverage,
                similarity_score,
                line_delta,
                semantic_mismatch,
                has_unimplemented,
                compiled_at,
                content_hash
             )
             SELECT
                id,
                card_name,
                oracle_text,
                raw_oracle_text,
                parse_status,
                parse_error,
                compiled_text,
                unprocessed_compiled_text,
                compiled_card_definition,
                oracle_coverage,
                compiled_coverage,
                similarity_score,
                line_delta,
                semantic_mismatch,
                has_unimplemented,
                compiled_at,
                content_hash
             FROM card_compilation_old;
             DROP TABLE card_compilation_old;",
        )?;
        Ok(())
    }

    fn rename_compilation_text_columns_if_needed(&self) -> Result<(), Box<dyn Error>> {
        if !self.card_compilation_has_column("normalized_oracle_text")
            && self.card_compilation_has_column("compiled_text")
        {
            self.conn
                .execute_batch("DROP VIEW IF EXISTS latest_card_compilation;")?;
            self.conn.execute_batch(
                "ALTER TABLE card_compilation
                 RENAME COLUMN compiled_text TO normalized_oracle_text;",
            )?;
        }

        if self.card_compilation_has_column("unprocessed_compiled_text") {
            self.conn
                .execute_batch("DROP VIEW IF EXISTS latest_card_compilation;")?;
            self.conn.execute_batch(
                "ALTER TABLE card_compilation
                 RENAME COLUMN unprocessed_compiled_text TO compiled_text;",
            )?;
        }

        if self.card_compilation_has_column("oracle_text")
            && !self.card_compilation_has_column("normalized_oracle_text")
        {
            self.conn.execute_batch(
                "ALTER TABLE card_compilation
                 ADD COLUMN normalized_oracle_text TEXT;
                 UPDATE card_compilation
                 SET normalized_oracle_text = oracle_text
                 WHERE normalized_oracle_text IS NULL;",
            )?;
        }

        if !self.card_compilation_has_column("compiled_text") {
            self.conn.execute_batch(
                "ALTER TABLE card_compilation
                 ADD COLUMN compiled_text TEXT;",
            )?;
        }

        self.conn.execute_batch(
            "UPDATE card_compilation
             SET normalized_oracle_text = oracle_text
             WHERE normalized_oracle_text IS NULL;",
        )?;

        Ok(())
    }

    fn add_parse_loss_columns_if_needed(&self) -> Result<(), Box<dyn Error>> {
        if !self.card_compilation_has_column("parse_lossy") {
            self.conn.execute_batch(
                "ALTER TABLE card_compilation
                 ADD COLUMN parse_lossy INTEGER NOT NULL DEFAULT 0;",
            )?;
        }
        if !self.card_compilation_has_column("parse_loss_reasons") {
            self.conn.execute_batch(
                "ALTER TABLE card_compilation
                 ADD COLUMN parse_loss_reasons TEXT NOT NULL DEFAULT '';",
            )?;
        }
        if !self.card_compilation_has_column("parse_loss_count") {
            self.conn.execute_batch(
                "ALTER TABLE card_compilation
                 ADD COLUMN parse_loss_count INTEGER NOT NULL DEFAULT 0;",
            )?;
        }
        Ok(())
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

        // Migration: v3 -> v4: add agent_running column to latest_card_observation
        if version == 3 {
            // Check if column already exists (idempotent)
            let has_column: bool = self
                .conn
                .prepare("SELECT agent_running FROM latest_card_observation LIMIT 0")
                .is_ok();
            if !has_column {
                self.conn.execute_batch(
                    "ALTER TABLE latest_card_observation ADD COLUMN agent_running INTEGER NOT NULL DEFAULT 0;",
                )?;
            }
        }

        if version > 0 && version < 5 {
            let card_compilation_has_raw: bool = self
                .conn
                .prepare("SELECT raw_oracle_text FROM card_compilation LIMIT 0")
                .is_ok();
            if !card_compilation_has_raw {
                self.conn.execute_batch(
                    "ALTER TABLE card_compilation ADD COLUMN raw_oracle_text TEXT NOT NULL DEFAULT '';
                     UPDATE card_compilation
                     SET raw_oracle_text = oracle_text
                     WHERE raw_oracle_text = '';",
                )?;
            }

            let registry_card_has_raw: bool = self
                .conn
                .prepare("SELECT raw_oracle_text FROM registry_card LIMIT 0")
                .is_ok();
            if !registry_card_has_raw {
                self.conn.execute_batch(
                    "ALTER TABLE registry_card ADD COLUMN raw_oracle_text TEXT NOT NULL DEFAULT '';
                     UPDATE registry_card
                     SET raw_oracle_text = oracle_text
                     WHERE raw_oracle_text = '';",
                )?;
            }
        }

        if version > 0 && version < 7 {
            let has_unprocessed_compiled_text_column: bool = self
                .conn
                .prepare("SELECT unprocessed_compiled_text FROM card_compilation LIMIT 0")
                .is_ok();
            if !has_unprocessed_compiled_text_column {
                self.conn.execute_batch(
                    "ALTER TABLE card_compilation
                     ADD COLUMN unprocessed_compiled_text TEXT;",
                )?;
            }
        }

        if version > 0 && version < 8 {
            self.drop_uses_pseudo_oracle_fallback_column_if_present()?;
        }

        if version > 0 && version < 9 {
            self.rename_compilation_text_columns_if_needed()?;
        }

        if version > 0 && version < 11 {
            let has_pr_created_column: bool = self
                .conn
                .prepare("SELECT pr_created FROM latest_card_observation LIMIT 0")
                .is_ok();
            if !has_pr_created_column {
                self.conn.execute_batch(
                    "ALTER TABLE latest_card_observation
                     ADD COLUMN pr_created INTEGER NOT NULL DEFAULT 0;",
                )?;
            }
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
            CREATE TABLE IF NOT EXISTS oracle_tag (
                tag TEXT PRIMARY KEY
            );
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
            JOIN card_compilation cc
            ON cc.id = latest.compilation_id;",
        )?;
        self.conn.execute(
            "DELETE FROM latest_card_observation
             WHERE compilation_id NOT IN (SELECT id FROM card_compilation)",
            [],
        )?;
        self.conn.execute(
            "INSERT OR IGNORE INTO latest_card_observation (card_name, compilation_id)
             SELECT card_name, MAX(id) AS compilation_id
             FROM card_compilation
             GROUP BY card_name",
            [],
        )?;
        self.add_parse_loss_columns_if_needed()?;
        self.conn.execute_batch(
            "DROP VIEW IF EXISTS latest_card_compilation;
             CREATE VIEW latest_card_compilation AS
             SELECT cc.*, latest.agent_running, latest.pr_created
             FROM latest_card_observation latest
             JOIN card_compilation cc
             ON cc.id = latest.compilation_id;",
        )?;
        self.conn
            .pragma_update(None, "user_version", DB_SCHEMA_VERSION)?;
        Ok(())
    }

    pub fn insert_snapshot_if_changed(
        &self,
        snapshot: &CompilationSnapshot,
    ) -> Result<bool, Box<dyn Error>> {
        let previous_latest_id = self
            .conn
            .query_row(
                "SELECT compilation_id
                 FROM latest_card_observation
                 WHERE card_name = ?1",
                [&snapshot.card_name],
                |row| row.get::<_, i64>(0),
            )
            .optional()?;

        self.conn.execute(
            "INSERT OR IGNORE INTO card_compilation (
                card_name,
                oracle_text,
                raw_oracle_text,
                parse_status,
                parse_error,
                normalized_oracle_text,
                compiled_text,
                compiled_card_definition,
                oracle_coverage,
                compiled_coverage,
                similarity_score,
                line_delta,
                semantic_mismatch,
                has_unimplemented,
                parse_lossy,
                parse_loss_reasons,
                parse_loss_count,
                content_hash
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)",
            params![
                snapshot.card_name,
                snapshot.oracle_text,
                snapshot.raw_oracle_text,
                snapshot.parse_status.as_str(),
                snapshot.parse_error,
                snapshot.normalized_oracle_text,
                snapshot.compiled_text,
                snapshot.compiled_card_definition,
                snapshot.oracle_coverage,
                snapshot.compiled_coverage,
                snapshot.similarity_score,
                snapshot.line_delta as i64,
                snapshot.semantic_mismatch,
                snapshot.has_unimplemented,
                snapshot.parse_lossy,
                snapshot.parse_loss_reasons,
                snapshot.parse_loss_count as i64,
                snapshot.content_hash,
            ],
        )?;
        let compilation_id: i64 = self.conn.query_row(
            "SELECT id
             FROM card_compilation
             WHERE card_name = ?1
               AND content_hash = ?2
             ORDER BY id DESC
             LIMIT 1",
            params![snapshot.card_name, snapshot.content_hash],
            |row| row.get(0),
        )?;
        self.conn.execute(
            "INSERT INTO latest_card_observation (card_name, compilation_id, agent_running)
             VALUES (?1, ?2, 0)
             ON CONFLICT(card_name) DO UPDATE SET
                 compilation_id = excluded.compilation_id",
            params![snapshot.card_name, compilation_id],
        )?;

        Ok(previous_latest_id != Some(compilation_id))
    }

    pub fn insert_snapshots_if_changed(
        &mut self,
        snapshots: &[CompilationSnapshot],
    ) -> Result<usize, Box<dyn Error>> {
        let tx = self.conn.transaction()?;
        let mut changed_count = 0usize;

        {
            let mut select_previous_latest = tx.prepare(
                "SELECT compilation_id
                 FROM latest_card_observation
                 WHERE card_name = ?1",
            )?;
            let mut insert_compilation = tx.prepare(
                "INSERT OR IGNORE INTO card_compilation (
                    card_name,
                    oracle_text,
                    raw_oracle_text,
                    parse_status,
                    parse_error,
                    normalized_oracle_text,
                    compiled_text,
                    compiled_card_definition,
                    oracle_coverage,
                    compiled_coverage,
                    similarity_score,
                    line_delta,
                    semantic_mismatch,
                    has_unimplemented,
                    parse_lossy,
                    parse_loss_reasons,
                    parse_loss_count,
                    content_hash
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)",
            )?;
            let mut select_compilation_id = tx.prepare(
                "SELECT id
                 FROM card_compilation
                 WHERE card_name = ?1
                   AND content_hash = ?2
                 ORDER BY id DESC
                 LIMIT 1",
            )?;
            let mut upsert_latest = tx.prepare(
                "INSERT INTO latest_card_observation (card_name, compilation_id, agent_running)
                 VALUES (?1, ?2, 0)
                 ON CONFLICT(card_name) DO UPDATE SET
                     compilation_id = excluded.compilation_id",
            )?;

            for snapshot in snapshots {
                let previous_latest_id = select_previous_latest
                    .query_row([snapshot.card_name.as_str()], |row| row.get::<_, i64>(0))
                    .optional()?;

                insert_compilation.execute(params![
                    snapshot.card_name.as_str(),
                    snapshot.oracle_text.as_str(),
                    snapshot.raw_oracle_text.as_str(),
                    snapshot.parse_status.as_str(),
                    snapshot.parse_error.as_deref(),
                    snapshot.normalized_oracle_text.as_str(),
                    snapshot.compiled_text.as_deref(),
                    snapshot.compiled_card_definition.as_deref(),
                    snapshot.oracle_coverage,
                    snapshot.compiled_coverage,
                    snapshot.similarity_score,
                    snapshot.line_delta as i64,
                    snapshot.semantic_mismatch,
                    snapshot.has_unimplemented,
                    snapshot.parse_lossy,
                    snapshot.parse_loss_reasons.as_str(),
                    snapshot.parse_loss_count as i64,
                    snapshot.content_hash.as_str(),
                ])?;

                let compilation_id: i64 = select_compilation_id.query_row(
                    params![snapshot.card_name.as_str(), snapshot.content_hash.as_str()],
                    |row| row.get(0),
                )?;

                upsert_latest.execute(params![snapshot.card_name.as_str(), compilation_id])?;

                if previous_latest_id != Some(compilation_id) {
                    changed_count += 1;
                }
            }
        }

        tx.commit()?;
        Ok(changed_count)
    }

    pub fn set_agent_running(&self, card_name: &str, running: bool) -> Result<(), Box<dyn Error>> {
        self.conn.execute(
            "UPDATE latest_card_observation SET agent_running = ?1 WHERE card_name = ?2",
            params![running as i32, card_name],
        )?;
        Ok(())
    }

    pub fn clear_all_agent_running(&self) -> Result<(), Box<dyn Error>> {
        self.conn
            .execute("UPDATE latest_card_observation SET agent_running = 0", [])?;
        Ok(())
    }

    pub fn set_pr_created(&self, card_name: &str, created: bool) -> Result<(), Box<dyn Error>> {
        self.conn.execute(
            "UPDATE latest_card_observation SET pr_created = ?1 WHERE card_name = ?2",
            params![created as i32, card_name],
        )?;
        Ok(())
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

    pub fn latest_strict_compiled_scores(&self) -> Result<BTreeMap<String, f32>, Box<dyn Error>> {
        let mut stmt = self.conn.prepare(
            "SELECT card_name, similarity_score
             FROM latest_card_compilation
             WHERE parse_status = ?1
             ORDER BY card_name ASC",
        )?;
        let scores = stmt
            .query_map([ParseStatus::StrictCompiled.as_str()], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, f32>(1)?))
            })?
            .collect::<Result<BTreeMap<String, f32>, _>>()?;
        Ok(scores)
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
            tx.query_row("SELECT COUNT(*) FROM oracle_tag", [], read_sqlite_count)?;
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
                    raw_oracle_text,
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
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
                ON CONFLICT(card_name) DO UPDATE SET
                    oracle_text = excluded.oracle_text,
                    raw_oracle_text = excluded.raw_oracle_text,
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
                let stored_oracle_text = strip_parenthetical_text(&row.payload.oracle_text);
                upsert.execute(params![
                    row.payload.name.as_str(),
                    stored_oracle_text,
                    row.payload.raw_oracle_text.as_str(),
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
            read_sqlite_count,
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
            read_sqlite_count,
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
            read_sqlite_count,
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
            read_sqlite_count,
        )?;

        tx.execute(
            "DELETE FROM latest_card_observation
             WHERE NOT EXISTS (
                 SELECT 1
                 FROM temp_allowed_card_name allowed
                 WHERE allowed.card_name = latest_card_observation.card_name
             )",
            [],
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
            read_sqlite_count,
        )?;
        let compilation_rows_deleted: usize = tx.query_row(
            "SELECT COUNT(*)
             FROM card_compilation
             WHERE id NOT IN (
                 SELECT compilation_id
                 FROM latest_card_observation
             )",
            [],
            read_sqlite_count,
        )?;

        tx.execute(
            "DELETE FROM card_compilation
             WHERE id NOT IN (
                 SELECT compilation_id
                 FROM latest_card_observation
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
            "SELECT card_name, oracle_text, raw_oracle_text, parse_input
             FROM registry_card
             ORDER BY card_name ASC",
        )?;
        let rows = stmt
            .query_map([], |row| {
                Ok(CardPayload {
                    name: row.get(0)?,
                    parse_name: None,
                    oracle_text: row.get(1)?,
                    raw_oracle_text: row.get(2)?,
                    metadata_lines: Vec::new(),
                    parse_input: row.get(3)?,
                    other_face_name: None,
                    linked_face_layout: None,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn registry_card_count(&self) -> Result<usize, Box<dyn Error>> {
        let count =
            self.conn
                .query_row("SELECT COUNT(*) FROM registry_card", [], read_sqlite_count)?;
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
    hasher.update(payload.raw_oracle_text.as_bytes());
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
    load_registry_cards_from_values_with_explicit_includes(cards, &BTreeSet::new())
}

fn load_registry_cards_from_values_with_explicit_includes<I>(
    cards: I,
    included_names: &BTreeSet<String>,
) -> BTreeMap<String, RegistryCardRecord>
where
    I: IntoIterator<Item = Value>,
{
    let mut out = BTreeMap::new();
    for card in cards {
        let Some(record) = build_registry_card_record_with_explicit_includes(&card, included_names)
        else {
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

fn build_registry_card_record_with_explicit_includes(
    card: &Value,
    included_names: &BTreeSet<String>,
) -> Option<RegistryCardRecord> {
    if card
        .get("digital")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return None;
    }
    let face = get_first_face(card);
    let name = normalize_lookup_name(&pick_field(card, face, "name")?);
    if name.is_empty() {
        return None;
    }
    if !card_is_legal_in_supported_paper_format(card) && !included_names.contains(&name) {
        return None;
    }
    let parse_name = face
        .and_then(|value| value.get("name"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty() && *value != name)
        .map(ToOwned::to_owned);

    let raw_oracle_text = pick_field(card, face, "oracle_text")
        .unwrap_or_default()
        .trim()
        .to_string();
    let oracle_text = strip_parenthetical_text(&raw_oracle_text);

    let mana_cost = pick_field_preferring_face(card, face, "mana_cost");
    let type_line = pick_field_preferring_face(card, face, "type_line");
    let power = pick_field_preferring_face(card, face, "power");
    let toughness = pick_field_preferring_face(card, face, "toughness");
    let loyalty = pick_field_preferring_face(card, face, "loyalty");
    let defense = pick_field_preferring_face(card, face, "defense");

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

    let parse_input = build_parse_input(&metadata_lines, &raw_oracle_text);
    let linked_face_layout = linked_face_layout_from_card(card);
    let other_face_name = linked_face_layout.and_then(|_| {
        get_second_face(card)
            .and_then(|value| value.get("name"))
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
    });
    let payload = CardPayload {
        name,
        parse_name,
        oracle_text,
        raw_oracle_text,
        metadata_lines,
        parse_input,
        other_face_name,
        linked_face_layout,
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
        parse_trace::event(format!(
            "tool snapshot parse: card=\"{}\" allow_unsupported={} lines={}",
            name,
            allow_unsupported,
            parse_input.lines().count()
        ));
        let (result, parse_loss) = parse_loss::capture(|| {
            panic::catch_unwind(AssertUnwindSafe(|| {
                ironsmith_registry::compile_builder_to_runtime_definition(
                    CompilerCardDefinitionBuilder::new(
                        CardId::from_raw(FIXED_SNAPSHOT_CARD_ID),
                        name,
                    ),
                    parse_input.to_string(),
                    allow_unsupported,
                )
            }))
        });
        match result {
            Ok(Ok(definition)) => ParseAttempt {
                status: ParseStatus::StrictCompiled,
                parse_error: None,
                definition: Some(definition),
                parse_loss,
            },
            Ok(Err(err)) => ParseAttempt {
                status: ParseStatus::ParseFailed,
                parse_error: Some(format!("{err:?}")),
                definition: None,
                parse_loss,
            },
            Err(payload) => ParseAttempt {
                status: ParseStatus::ParseFailed,
                parse_error: Some(format!("panic: {}", panic_payload_to_string(payload))),
                definition: None,
                parse_loss,
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
    sanitized.card.other_face = sanitized.card.other_face.map(|_| CardId::from_raw(2));
    normalize_debug_card_ids(&format!("{sanitized:#?}"))
}

fn normalize_debug_card_ids(snapshot: &str) -> String {
    let mut normalized = Vec::new();
    let mut lines = snapshot.lines().peekable();
    while let Some(line) = lines.next() {
        normalized.push(line.to_string());
        if !line.trim().ends_with("CardId(") {
            continue;
        }

        let Some(next_line) = lines.peek() else {
            continue;
        };
        let trimmed = next_line.trim();
        let Some(number) = trimmed.strip_suffix(',') else {
            continue;
        };
        if !number.chars().all(|ch| ch.is_ascii_digit()) {
            continue;
        }

        let indent_len = next_line.len().saturating_sub(next_line.trim_start().len());
        let indent = " ".repeat(indent_len);
        normalized.push(format!("{indent}{FIXED_SNAPSHOT_CARD_ID},"));
        lines.next();
    }
    normalize_inline_card_ids(&normalized.join("\n"))
}

fn normalize_inline_card_ids(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(idx) = rest.find("CardId(") {
        let (prefix, after_prefix) = rest.split_at(idx);
        out.push_str(prefix);

        let after_marker = &after_prefix["CardId(".len()..];
        let digit_len = after_marker
            .chars()
            .take_while(|ch| ch.is_ascii_digit())
            .map(char::len_utf8)
            .sum::<usize>();
        if digit_len == 0 || !after_marker[digit_len..].starts_with(')') {
            out.push_str("CardId(");
            rest = after_marker;
            continue;
        }

        out.push_str(&format!("CardId({FIXED_SNAPSHOT_CARD_ID})"));
        rest = &after_marker[digit_len + 1..];
    }
    out.push_str(rest);
    out
}

fn get_first_face(card: &Value) -> Option<&Value> {
    card.get("card_faces")
        .and_then(Value::as_array)
        .and_then(|faces| faces.first())
}

fn get_second_face(card: &Value) -> Option<&Value> {
    card.get("card_faces")
        .and_then(Value::as_array)
        .and_then(|faces| faces.get(1))
}

fn matching_face_indexes(card: &Value, normalized_name: &str) -> Vec<usize> {
    card.get("card_faces")
        .and_then(Value::as_array)
        .map(|faces| {
            faces
                .iter()
                .enumerate()
                .filter_map(|(idx, face)| {
                    let face_name = face.get("name").and_then(Value::as_str)?.trim();
                    (normalize_lookup_name(face_name) == normalized_name).then_some(idx)
                })
                .collect()
        })
        .unwrap_or_default()
}

fn build_card_payload_for_face(card: &Value, face_index: usize) -> Option<CardPayload> {
    let faces = card.get("card_faces")?.as_array()?;
    let face = faces.get(face_index)?;
    let name = face.get("name")?.as_str()?.trim();
    if name.is_empty() {
        return None;
    }

    let raw_oracle_text = face
        .get("oracle_text")
        .and_then(value_to_string)
        .unwrap_or_default()
        .trim()
        .to_string();
    let oracle_text = strip_parenthetical_text(&raw_oracle_text);
    let mana_cost = pick_field_preferring_face(card, Some(face), "mana_cost");
    let type_line = pick_field_preferring_face(card, Some(face), "type_line");
    let power = pick_field_preferring_face(card, Some(face), "power");
    let toughness = pick_field_preferring_face(card, Some(face), "toughness");
    let loyalty = pick_field_preferring_face(card, Some(face), "loyalty");
    let defense = pick_field_preferring_face(card, Some(face), "defense");

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

    let linked_face_layout = linked_face_layout_from_card(card);
    let other_face_name = linked_face_layout.and_then(|_| {
        faces
            .iter()
            .enumerate()
            .find(|(idx, _)| *idx != face_index)
            .and_then(|(_, other_face)| other_face.get("name"))
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
    });

    Some(CardPayload {
        name: name.to_string(),
        parse_name: None,
        oracle_text,
        raw_oracle_text: raw_oracle_text.clone(),
        metadata_lines: metadata_lines.clone(),
        parse_input: build_parse_input(&metadata_lines, &raw_oracle_text),
        other_face_name,
        linked_face_layout,
    })
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

fn pick_field_preferring_face(card: &Value, face: Option<&Value>, key: &str) -> Option<String> {
    if let Some(value) = face
        .and_then(|value| value.get(key))
        .and_then(value_to_string)
    {
        return Some(value);
    }
    card.get(key).and_then(value_to_string)
}

fn linked_face_layout_from_card(card: &Value) -> Option<LinkedFaceLayout> {
    match card.get("layout").and_then(Value::as_str).map(str::trim) {
        Some("transform") => Some(LinkedFaceLayout::TransformLike),
        Some("split") => Some(LinkedFaceLayout::Split),
        _ => None,
    }
}

fn decorate_definition_from_payload(definition: &mut CardDefinition, payload: &CardPayload) {
    let Some(layout) = payload.linked_face_layout else {
        return;
    };
    if layout == LinkedFaceLayout::None {
        return;
    }

    definition.card.linked_face_layout = layout;
    if let Some(other_face_name) = payload.other_face_name.as_ref() {
        definition.card.other_face_name = Some(other_face_name.clone());
        if definition.card.other_face.is_none() {
            definition.card.other_face = Some(CardId::new());
        }
    }
}

fn definition_from_payload(
    payload: &CardPayload,
    card_id: CardId,
) -> Result<CardDefinition, String> {
    let parse_name = payload.parse_name.as_deref().unwrap_or(&payload.name);
    let builder = CompilerCardDefinitionBuilder::new(card_id, parse_name);
    parse_trace::event(format!(
        "payload compile: card=\"{}\" input=parse_input lines={}",
        payload.name,
        payload.parse_input.lines().count()
    ));
    let mut definition = match ironsmith_registry::compile_builder_to_runtime_definition(
        builder.clone(),
        payload.parse_input.clone(),
        false,
    ) {
        Ok(definition) => definition,
        Err(parse_input_err) => {
            parse_trace::event(format!(
                "payload compile: parse_input failed: {parse_input_err}; trying oracle text"
            ));
            parse_loss::record(
                "oracle_only_fallback",
                format!("parse input failed before oracle text fallback: {parse_input_err}"),
            );
            ironsmith_registry::compile_builder_to_runtime_definition(
                builder,
                payload.oracle_text.clone(),
                false,
            )
            .map_err(|oracle_err| {
                format!("{parse_input_err}; oracle-only fallback also failed: {oracle_err}")
            })?
        }
    };
    decorate_definition_from_payload(&mut definition, payload);
    Ok(definition)
}

fn parse_card_payload(payload: &CardPayload, allow_unsupported: bool) -> ParseAttempt {
    with_allow_unsupported(allow_unsupported, || {
        let (result, parse_loss) = parse_loss::capture(|| {
            panic::catch_unwind(AssertUnwindSafe(|| {
                definition_from_payload(payload, CardId::from_raw(FIXED_SNAPSHOT_CARD_ID))
            }))
        });
        match result {
            Ok(Ok(definition)) => ParseAttempt {
                status: ParseStatus::StrictCompiled,
                parse_error: None,
                definition: Some(definition),
                parse_loss,
            },
            Ok(Err(err)) => ParseAttempt {
                status: ParseStatus::ParseFailed,
                parse_error: Some(err),
                definition: None,
                parse_loss,
            },
            Err(payload) => ParseAttempt {
                status: ParseStatus::ParseFailed,
                parse_error: Some(format!("panic: {}", panic_payload_to_string(payload))),
                definition: None,
                parse_loss,
            },
        }
    })
}

fn parse_card_payload_with_fallback(payload: &CardPayload) -> ParseAttempt {
    let strict_attempt = parse_card_payload(payload, false);
    if strict_attempt.status == ParseStatus::StrictCompiled {
        return strict_attempt;
    }
    let strict_error = strict_attempt.parse_error.clone();
    let allow_attempt = parse_card_payload(payload, true);
    if allow_attempt.status == ParseStatus::StrictCompiled {
        let mut parse_loss = allow_attempt.parse_loss;
        parse_loss.push_reason(
            "allow_unsupported_fallback",
            strict_error
                .as_deref()
                .unwrap_or("strict parse failed before allow-unsupported fallback"),
        );
        return ParseAttempt {
            status: ParseStatus::CompiledWithAllowUnsupported,
            parse_error: None,
            definition: allow_attempt.definition,
            parse_loss,
        };
    }
    ParseAttempt {
        status: ParseStatus::ParseFailed,
        parse_error: strict_error,
        definition: None,
        parse_loss: strict_attempt.parse_loss,
    }
}

pub fn compile_definition_from_payload(payload: &CardPayload) -> Result<CardDefinition, String> {
    definition_from_payload(payload, CardId::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ironsmith::compiled_text::debug_compiled_lines;
    use ironsmith::semantic_compare::report_embedding_config;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_path(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        env::temp_dir().join(format!("ironsmith-{name}-{nanos}.sqlite3"))
    }

    #[test]
    fn normalize_debug_card_ids_replaces_nested_generated_ids() {
        let snapshot = "\
CardDefinition {
    card: Card {
        id: CardId(
            1,
        ),
    },
    token: CardDefinition {
        card: Card {
            id: CardId(
                2368,
            ),
        },
    },
}";

        let normalized = normalize_debug_card_ids(snapshot);

        assert!(normalized.contains("            1,"));
        assert!(
            !normalized.contains("2368"),
            "generated nested card ids should not affect snapshot hashes"
        );
    }

    #[test]
    fn normalize_debug_card_ids_replaces_inline_generated_ids() {
        let snapshot = "parse failed: Effect(CreateTokenEffect { token: CardDefinition { card: Card { id: CardId(1520), name: \"Wizard\" } } }); oracle-only fallback also failed: CardId(1522)";

        let normalized = normalize_debug_card_ids(snapshot);

        assert!(!normalized.contains("1520"));
        assert!(!normalized.contains("1522"));
        assert_eq!(normalized.matches("CardId(1)").count(), 2);
    }

    fn lightning_bolt_payload() -> CardPayload {
        CardPayload {
            name: "Lightning Bolt".to_string(),
            parse_name: None,
            oracle_text: "Lightning Bolt deals 3 damage to any target.".to_string(),
            raw_oracle_text: "Lightning Bolt deals 3 damage to any target.".to_string(),
            metadata_lines: vec!["Mana cost: {R}".to_string(), "Type: Instant".to_string()],
            parse_input:
                "Mana cost: {R}\nType: Instant\nLightning Bolt deals 3 damage to any target."
                    .to_string(),
            other_face_name: None,
            linked_face_layout: None,
        }
    }

    fn pseudo_oracle_fallback_payload() -> CardPayload {
        CardPayload {
            name: "G'raha Tia Variant".to_string(),
            parse_name: None,
            oracle_text: "Reach\nThe Allagan Eye — Whenever one or more other creatures and/or artifacts you control die, draw a card. This ability triggers only once each turn.".to_string(),
            raw_oracle_text: "Reach\nThe Allagan Eye — Whenever one or more other creatures and/or artifacts you control die, draw a card. This ability triggers only once each turn.".to_string(),
            metadata_lines: vec![
                "Mana cost: {2}{G}".to_string(),
                "Type: Creature — Cat Wizard".to_string(),
                "Power/Toughness: 2/3".to_string(),
            ],
            parse_input: "Mana cost: {2}{G}\nType: Creature — Cat Wizard\nPower/Toughness: 2/3\nReach\nThe Allagan Eye — Whenever one or more other creatures and/or artifacts you control die, draw a card. This ability triggers only once each turn.".to_string(),
            other_face_name: None,
            linked_face_layout: None,
        }
    }

    fn stonebinders_familiar_payload() -> CardPayload {
        CardPayload {
            name: "Stonebinder's Familiar".to_string(),
            parse_name: None,
            oracle_text: "Whenever one or more cards are put into exile during your turn, put a +1/+1 counter on this creature. This ability triggers only once each turn.".to_string(),
            raw_oracle_text: "Whenever one or more cards are put into exile during your turn, put a +1/+1 counter on this creature. This ability triggers only once each turn.".to_string(),
            metadata_lines: vec![
                "Mana cost: {W}".to_string(),
                "Type: Creature - Spirit Dog".to_string(),
                "Power/Toughness: 1/1".to_string(),
            ],
            parse_input: "Mana cost: {W}\nType: Creature - Spirit Dog\nPower/Toughness: 1/1\nWhenever one or more cards are put into exile during your turn, put a +1/+1 counter on this creature. This ability triggers only once each turn.".to_string(),
            other_face_name: None,
            linked_face_layout: None,
        }
    }

    fn semantic_mismatch_payload() -> CardPayload {
        CardPayload {
            name: "Mismatch Fixture".to_string(),
            parse_name: None,
            oracle_text: "Activated abilities of creatures you control cost {2} less to activate. This effect can't reduce the mana in that cost to less than one mana.".to_string(),
            raw_oracle_text: "Activated abilities of creatures you control cost {2} less to activate. This effect can't reduce the mana in that cost to less than one mana.".to_string(),
            metadata_lines: vec!["Mana cost: {1}{W}".to_string(), "Type: Enchantment".to_string()],
            parse_input:
                "Mana cost: {1}{W}\nType: Enchantment\nActivated abilities of creatures you control cost {2} less to activate. This effect can't reduce the mana in that cost to less than one mana."
                    .to_string(),
            other_face_name: None,
            linked_face_layout: None,
        }
    }

    fn tarmogoyf_payload() -> CardPayload {
        CardPayload {
            name: "Tarmogoyf".to_string(),
            parse_name: None,
            oracle_text:
                "This creature's power is equal to the number of card types among cards in all graveyards and its toughness is equal to that number plus 1."
                    .to_string(),
            raw_oracle_text:
                "This creature's power is equal to the number of card types among cards in all graveyards and its toughness is equal to that number plus 1."
                    .to_string(),
            metadata_lines: vec![
                "Mana cost: {1}{G}".to_string(),
                "Type: Creature — Lhurgoyf".to_string(),
                "Power/Toughness: */1+*".to_string(),
            ],
            parse_input:
                "Mana cost: {1}{G}\nType: Creature — Lhurgoyf\nPower/Toughness: */1+*\nThis creature's power is equal to the number of card types among cards in all graveyards and its toughness is equal to that number plus 1."
                    .to_string(),
            other_face_name: None,
            linked_face_layout: None,
        }
    }

    fn holistic_wisdom_payload() -> CardPayload {
        CardPayload {
            name: "Holistic Wisdom".to_string(),
            parse_name: None,
            oracle_text:
                "{2}, Exile a card from your hand: Return target card from your graveyard to your hand if it shares a card type with the card exiled this way."
                    .to_string(),
            raw_oracle_text:
                "{2}, Exile a card from your hand: Return target card from your graveyard to your hand if it shares a card type with the card exiled this way."
                    .to_string(),
            metadata_lines: vec![
                "Mana cost: {1}{G}{G}".to_string(),
                "Type: Enchantment".to_string(),
            ],
            parse_input:
                "Mana cost: {1}{G}{G}\nType: Enchantment\n{2}, Exile a card from your hand: Return target card from your graveyard to your hand if it shares a card type with the card exiled this way."
                    .to_string(),
            other_face_name: None,
            linked_face_layout: None,
        }
    }

    fn battle_cry_payload() -> CardPayload {
        CardPayload {
            name: "Accorder Paladin".to_string(),
            parse_name: None,
            oracle_text: "Battle cry".to_string(),
            raw_oracle_text: "Battle cry (Whenever this creature attacks, each other attacking creature gets +1/+0 until end of turn.)".to_string(),
            metadata_lines: vec![
                "Mana cost: {1}{W}".to_string(),
                "Type: Creature — Human Knight".to_string(),
                "Power/Toughness: 3/1".to_string(),
            ],
            parse_input: "Mana cost: {1}{W}\nType: Creature — Human Knight\nPower/Toughness: 3/1\nBattle cry".to_string(),
            other_face_name: None,
            linked_face_layout: None,
        }
    }

    fn enlist_payload() -> CardPayload {
        CardPayload {
            name: "Barkweave Crusher".to_string(),
            parse_name: None,
            oracle_text: "Enlist".to_string(),
            raw_oracle_text: "Enlist (As this creature attacks, you may tap a nonattacking creature you control without summoning sickness. When you do, add its power to this creature's until end of turn.)".to_string(),
            metadata_lines: vec![
                "Mana cost: {3}{G}".to_string(),
                "Type: Creature — Elemental Warrior".to_string(),
                "Power/Toughness: 4/4".to_string(),
            ],
            parse_input: "Mana cost: {3}{G}\nType: Creature — Elemental Warrior\nPower/Toughness: 4/4\nEnlist".to_string(),
            other_face_name: None,
            linked_face_layout: None,
        }
    }

    fn rout_payload() -> CardPayload {
        CardPayload {
            name: "Rout".to_string(),
            parse_name: None,
            oracle_text: "You may cast this spell as though it had flash if you pay {2} more to cast it.\nDestroy all creatures. They can't be regenerated.".to_string(),
            raw_oracle_text: "You may cast this spell as though it had flash if you pay {2} more to cast it.\nDestroy all creatures. They can't be regenerated.".to_string(),
            metadata_lines: vec![
                "Mana cost: {3}{W}{W}".to_string(),
                "Type: Instant".to_string(),
            ],
            parse_input: "Mana cost: {3}{W}{W}\nType: Instant\nYou may cast this spell as though it had flash if you pay {2} more to cast it.\nDestroy all creatures. They can't be regenerated.".to_string(),
            other_face_name: None,
            linked_face_layout: None,
        }
    }

    fn intuition_payload() -> CardPayload {
        CardPayload {
            name: "Intuition".to_string(),
            parse_name: None,
            oracle_text: "Search your library for three cards and reveal them. Target opponent chooses one. Put that card into your hand and the rest into your graveyard. Then shuffle.".to_string(),
            raw_oracle_text: "Search your library for three cards and reveal them. Target opponent chooses one. Put that card into your hand and the rest into your graveyard. Then shuffle.".to_string(),
            metadata_lines: vec![
                "Mana cost: {2}{U}".to_string(),
                "Type: Instant".to_string(),
            ],
            parse_input: "Mana cost: {2}{U}\nType: Instant\nSearch your library for three cards and reveal them. Target opponent chooses one. Put that card into your hand and the rest into your graveyard. Then shuffle.".to_string(),
            other_face_name: None,
            linked_face_layout: None,
        }
    }

    #[test]
    fn snapshot_records_normalized_oracle_and_compiled_text() {
        let fallback_snapshot = compile_snapshot_from_payload(&pseudo_oracle_fallback_payload());
        assert_eq!(fallback_snapshot.parse_status, ParseStatus::StrictCompiled);
        assert_eq!(
            fallback_snapshot.normalized_oracle_text,
            String::from(
                "Reach\nThe Allagan Eye — Whenever one or more other creatures and/or artifacts you control die, draw a card. This ability triggers only once each turn."
            )
        );
        assert_eq!(
            fallback_snapshot.compiled_text.as_deref(),
            Some(
                "Reach\nWhenever other creature artifact you control dies, you draw a card. This ability triggers only once each turn."
            )
        );
    }

    #[test]
    fn authoritative_snapshot_keeps_semantic_mismatch_as_strict_compiled() {
        let snapshot = compile_authoritative_snapshot_from_payload(&semantic_mismatch_payload());

        assert_eq!(snapshot.parse_status, ParseStatus::StrictCompiled);
        assert!(
            snapshot.semantic_mismatch,
            "authoritative snapshots should preserve semantic mismatch flags"
        );
        assert!(
            snapshot.parse_error.is_none(),
            "semantic mismatches should not be stored as parse errors"
        );
        assert!(
            snapshot.compiled_text.is_some(),
            "semantic mismatch snapshots should keep their compiled text"
        );
    }

    #[test]
    fn snapshot_strictly_compiles_stonebinders_familiar() {
        let snapshot = compile_snapshot_from_payload(&stonebinders_familiar_payload());

        assert_eq!(snapshot.parse_status, ParseStatus::StrictCompiled);
        let compiled = snapshot
            .compiled_text
            .as_deref()
            .expect("Stonebinder's Familiar should produce compiled text");
        assert!(
            compiled.contains("put") && compiled.contains("+1/+1") && compiled.contains("counter"),
            "compiled text should keep the counter-adding trigger effect, got: {compiled}"
        );
        assert!(
            compiled.contains("Whenever one or more cards are put into exile during your turn"),
            "compiled text should keep the exile trigger card subject, got: {compiled}"
        );
        assert!(
            compiled.contains("This ability triggers only once each turn"),
            "compiled text should preserve the once-each-turn limit, got: {compiled}"
        );
    }

    #[test]
    fn authoritative_snapshot_rejects_dropped_card_types_among_marker() {
        let snapshot = compile_authoritative_snapshot_from_payload(&tarmogoyf_payload());

        assert_eq!(snapshot.parse_status, ParseStatus::ParseFailed);
        assert_eq!(
            snapshot.parse_error.as_deref(),
            Some("compiled text dropped required semantic marker: card-types-among")
        );
        assert!(snapshot.compiled_text.is_none());
    }

    #[test]
    fn authoritative_snapshot_rejects_dropped_shared_card_type_marker() {
        let snapshot = compile_authoritative_snapshot_from_payload(&holistic_wisdom_payload());

        assert_eq!(snapshot.parse_status, ParseStatus::ParseFailed);
        assert_eq!(
            snapshot.parse_error.as_deref(),
            Some("compiled text dropped required semantic marker: shares-a-card-type")
        );
        assert!(snapshot.compiled_text.is_none());
    }

    #[test]
    fn authoritative_marker_guard_rejects_internal_compiled_text() {
        let mut snapshot = compile_snapshot_from_payload(&lightning_bolt_payload());
        snapshot.compiled_text = Some(
            "This artifact is a creature as long as ValueComparison { left: CountersOnSource }."
                .to_string(),
        );
        assert_eq!(
            authoritative_semantic_marker_parse_error(&snapshot).as_deref(),
            Some("compiled text contains internal marker: value-comparison-debug")
        );

        snapshot.compiled_text = Some(
            "Put the tagged object 'searched_outside_game' into its owner's hand.".to_string(),
        );
        assert_eq!(
            authoritative_semantic_marker_parse_error(&snapshot).as_deref(),
            Some("compiled text contains internal marker: tagged-object-reference")
        );

        snapshot.compiled_text =
            Some("If that object matches attacking permanent, draw a card.".to_string());
        assert_eq!(
            authoritative_semantic_marker_parse_error(&snapshot).as_deref(),
            Some("compiled text contains internal marker: object-predicate-debug")
        );
    }

    #[test]
    fn authoritative_marker_guard_accepts_equivalent_card_types_in_wording() {
        let mut snapshot = compile_snapshot_from_payload(&lightning_bolt_payload());
        snapshot.normalized_oracle_text =
            "This creature's power is equal to the number of card types among cards in all graveyards".to_string();
        snapshot.compiled_text = Some(
            "This creature's power is equal to the number of distinct card types in all graveyards"
                .to_string(),
        );

        assert_eq!(authoritative_semantic_marker_parse_error(&snapshot), None);
    }

    #[test]
    fn authoritative_marker_guard_rejects_malformed_compiled_text() {
        let mut snapshot = compile_snapshot_from_payload(&lightning_bolt_payload());

        for (compiled, expected) in [
            (
                "Whenever Target creature gets +2/+0 until end of turn.",
                "compiled text contains malformed output: malformed-whenever-target",
            ),
            (
                "Whenever Destroy target artifact.",
                "compiled text contains malformed output: malformed-whenever-destroy",
            ),
            (
                "Whenever Reveal the top four cards of your library.",
                "compiled text contains malformed output: malformed-whenever-reveal",
            ),
            (
                "Whenever As long as this creature is paired, both creatures have flying.",
                "compiled text contains malformed output: malformed-whenever-as-long-as",
            ),
            (
                "This spell token can't block.",
                "compiled text contains malformed output: malformed-spell-token",
            ),
            (
                "You may If that card is an artifact, cast it.",
                "compiled text contains malformed output: malformed-conditional-permission",
            ),
            (
                "Whenever a creature dies, for each opponent,.",
                "compiled text contains malformed output: malformed-empty-per-opponent-effect",
            ),
            (
                "When this creature dies, permanents loses all abilities.",
                "compiled text contains malformed output: malformed-permanents-loses",
            ),
        ] {
            snapshot.compiled_text = Some(compiled.to_string());
            assert_eq!(
                authoritative_semantic_marker_parse_error(&snapshot).as_deref(),
                Some(expected)
            );
        }
    }

    #[test]
    fn authoritative_marker_guard_rejects_dropped_required_words() {
        let mut snapshot = compile_snapshot_from_payload(&lightning_bolt_payload());

        for (oracle, compiled, expected) in [
            (
                "Choose a creature at random.",
                "Choose a creature.",
                "compiled text dropped required semantic marker: at-random",
            ),
            (
                "Put your commander into your hand from the command zone.",
                "Return a commander permanent you own to its owner's hand.",
                "compiled text dropped required semantic marker: command-zone",
            ),
            (
                "Destroy all permanents with the same name as that permanent.",
                "Destroy all permanents.",
                "compiled text dropped required semantic marker: same-name",
            ),
            (
                "Exile that card instead of putting it into your graveyard.",
                "Exile that card.",
                "compiled text dropped required semantic marker: instead",
            ),
            (
                "You may pay {0} rather than pay this spell's mana cost.",
                "You may pay {0}.",
                "compiled text dropped required semantic marker: rather-than",
            ),
            (
                "Creatures can attack as though they didn't have defender.",
                "Creatures have defender.",
                "compiled text dropped required semantic marker: as-though",
            ),
            (
                "You may cast that card without paying its mana cost.",
                "You may cast that card.",
                "compiled text dropped required semantic marker: without-paying",
            ),
            (
                "This ability triggers only once each turn.",
                "This ability triggers each turn.",
                "compiled text dropped required semantic marker: only-once",
            ),
            (
                "As an additional cost to cast this spell, sacrifice a creature.",
                "Sacrifice a creature.",
                "compiled text dropped required semantic marker: additional-cost",
            ),
            (
                "Spend this mana only to cast artifact spells.",
                "Add one mana.",
                "compiled text dropped required semantic marker: spend-this-mana-only",
            ),
            (
                "It gains crew 2 until end of turn.",
                "It gains haste until end of turn.",
                "compiled text dropped required semantic marker: crew",
            ),
            (
                "Whenever you roll one or more dice, draw a card.",
                "Whenever an event happens, draw a card.",
                "compiled text dropped required semantic marker: roll-die",
            ),
            (
                "This creature can't be blocked.",
                "This creature has menace.",
                "compiled text dropped required semantic marker: cant-be-blocked",
            ),
            (
                "Search your library for a card named Lightning Bolt.",
                "Search your library for a card.",
                "compiled text dropped required semantic marker: card-named",
            ),
            (
                "At the beginning of your end step, if you control no creatures with decayed, create a Zombie token.",
                "At the beginning of your end step, if you control no creature, create a Zombie token.",
                "compiled text dropped required semantic marker: no-creatures-with-decayed",
            ),
            (
                "Put a +1/+1 counter on Pako for each noncreature card exiled this way.",
                "Put that many +1/+1 counters on Pako.",
                "compiled text dropped required semantic marker: noncreature-exiled-this-way",
            ),
            (
                "Each player chooses from the lands they control a land of each basic land type.",
                "Each player chooses a basic land.",
                "compiled text dropped required semantic marker: each-basic-land-type",
            ),
            (
                "Target player who has more cards in hand than they do may discard their hand.",
                "You discard your hand.",
                "compiled text dropped required semantic marker: more-cards-in-hand-than",
            ),
            (
                "Each opponent loses 3 life and you gain 3 life.",
                "You draw three cards.",
                "compiled text dropped required semantic marker: each-opponent-loses",
            ),
            (
                "When this artifact enters, note one of its creature types.",
                "When this artifact enters, create a token.",
                "compiled text dropped required semantic marker: noted-creature-types",
            ),
            (
                "Choose two target creatures controlled by the same opponent.",
                "Choose two target creatures.",
                "compiled text dropped required semantic marker: same-opponent-controlled",
            ),
            (
                "That player chooses one of those creatures.",
                "Choose a target creature.",
                "compiled text dropped required semantic marker: that-player-chooses",
            ),
            (
                "Trial of Agony deals 5 damage to that creature.",
                "Target creature can't block this turn.",
                "compiled text dropped required semantic marker: damage-to-chosen-creature",
            ),
            (
                "You may cast target instant, sorcery, or artifact card from your graveyard without paying its mana cost.",
                "You may choose an instant, sorcery, or artifact card. You may cast it without paying its mana cost.",
                "compiled text dropped required semantic marker: target-card-from-your-graveyard",
            ),
            (
                "If an instant or sorcery spell cast this way would be put into your graveyard, exile it instead.",
                "The next time instant or sorcery spell would go from stack into graveyard this turn, it goes to exile instead.",
                "compiled text dropped required semantic marker: cast-this-way-replacement",
            ),
            (
                "Then amass Orcs X, where X is the number of Equipment attached to Shagrat.",
                "Amass orcs the number of Equipments on the battlefield.",
                "compiled text dropped required semantic marker: equipment-attached-count",
            ),
            (
                "The token is goaded for the rest of the game.",
                "Goad that creature.",
                "compiled text dropped required semantic marker: goaded-rest-of-game",
            ),
            (
                "Each player exiles cards from the top of their library until they exile a nonland card.",
                "Each player exiles the top card of that player's library.",
                "compiled text dropped required semantic marker: exile-until-nonland",
            ),
            (
                "You may cast any number of spells from among the nonland cards exiled this way without paying their mana costs.",
                "You may cast each nonland card in exile without paying its mana cost.",
                "compiled text dropped required semantic marker: nonland-cards-exiled-this-way",
            ),
            (
                "Choose any number of permanents you control that had a counter put on them this way.",
                "You choose any number permanents you control on the battlefield.",
                "compiled text dropped required semantic marker: counter-put-this-way",
            ),
            (
                "Where X is the number of Bobbleheads you control as you activate this ability.",
                "Choose up to X target creatures you control.",
                "compiled text dropped required semantic marker: as-you-activate-this-ability",
            ),
            (
                "Target player reveals their hand.",
                "Target player discards a card.",
                "compiled text dropped required semantic marker: reveals-hand",
            ),
            (
                "The second spell you cast each turn costs {2} less to cast.",
                "Spells you cast cost {2} less to cast.",
                "compiled text dropped required semantic marker: second-spell-each-turn",
            ),
            (
                "Target creature you control deals damage equal to its power to each other creature.",
                "For each other creature, this spell deals X damage to that object, where X is this creature's power.",
                "compiled text dropped required semantic marker: target-creature-power-damage-source",
            ),
            (
                "Target creature you control deals damage equal to its power to up to one target creature you don't control.",
                "A creature you control deals damage equal to its power to up to one target creature you don't control.",
                "compiled text dropped required semantic marker: target-creature-power-damage-source",
            ),
            (
                "When this creature dies or is put into exile from the battlefield, you may put it into its owner's library.",
                "When this creature dies or leaves the battlefield, you may put it into its owner's library.",
                "compiled text dropped required semantic marker: put-into-exile-from-battlefield",
            ),
            (
                "Target opponent sacrifices a creature that attacked or blocked this turn.",
                "Target opponent sacrifices a blocked creature.",
                "compiled text dropped required semantic marker: attacked-or-blocked-this-turn",
            ),
            (
                "Whenever one or more cards leave your graveyard, this creature deals 1 damage to each opponent.",
                "Whenever a creature card in your graveyard leaves, this creature deals 1 damage.",
                "compiled text dropped required semantic marker: cards-leave-your-graveyard",
            ),
            (
                "Then you may pay one or more {E}.",
                "You may pay any amount of {E}.",
                "compiled text dropped required semantic marker: one-or-more-energy",
            ),
            (
                "You may have this creature enter as a copy of any creature on the battlefield.",
                "This creature enters with a +1/+1 counter on it.",
                "compiled text dropped required semantic marker: enter-as-copy",
            ),
        ] {
            snapshot.normalized_oracle_text = oracle.to_string();
            snapshot.compiled_text = Some(compiled.to_string());
            assert_eq!(
                authoritative_semantic_marker_parse_error(&snapshot).as_deref(),
                Some(expected)
            );
        }

        snapshot.normalized_oracle_text =
            "Destroy all permanents with the same name as that permanent.".to_string();
        snapshot.compiled_text = Some("Destroy all permanents with that name.".to_string());
        assert!(authoritative_semantic_marker_parse_error(&snapshot).is_none());

        snapshot.normalized_oracle_text =
            "Target creature you control deals damage equal to its power to target creature an opponent controls.".to_string();
        snapshot.compiled_text = Some(
            "Choose target creature you control. That creature deals damage equal to its power to target creature an opponent controls.".to_string(),
        );
        assert!(authoritative_semantic_marker_parse_error(&snapshot).is_none());

        snapshot.normalized_oracle_text =
            "This token can't be blocked until end of turn.".to_string();
        snapshot.compiled_text = Some("This token is unblockable until end of turn.".to_string());
        assert!(authoritative_semantic_marker_parse_error(&snapshot).is_none());
    }

    #[test]
    fn compiled_snapshot_applies_safe_keyword_transforms() {
        for payload in [battle_cry_payload(), enlist_payload()] {
            let snapshot = compile_snapshot_from_payload(&payload);
            assert_eq!(snapshot.parse_status, ParseStatus::StrictCompiled);
            assert_eq!(snapshot.normalized_oracle_text, payload.oracle_text);
            assert_eq!(
                snapshot.compiled_text.as_deref(),
                Some(payload.oracle_text.as_str())
            );
        }
    }

    #[test]
    fn debug_compiled_lines_keep_spell_effects_when_oracle_fallback_would_apply() {
        let definition =
            compile_definition_from_payload(&rout_payload()).expect("Rout should parse");
        let rendered = debug_compiled_lines(&definition).join("\n");

        assert!(
            rendered.contains("Destroy all creatures"),
            "expected debug compiled text to include Rout's spell effect, got {rendered:?}"
        );
        assert!(
            rendered.contains("They can't be regenerated"),
            "expected debug compiled text to include Rout's no-regeneration effect, got {rendered:?}"
        );
    }

    #[test]
    fn debug_compiled_lines_compact_intuition_divvy_without_oracle_text() {
        let definition =
            compile_definition_from_payload(&intuition_payload()).expect("Intuition should parse");
        let rendered = debug_compiled_lines(&definition).join("\n");

        assert_eq!(
            rendered,
            "Search your library for three cards and reveal them. Target opponent chooses one. Put that card into your hand and the rest into your graveyard. Then shuffle."
        );
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
        let definition = ironsmith_registry::compile_builder_to_runtime_definition(
            CompilerCardDefinitionBuilder::new(CardId::new(), "House Cartographer"),
            oracle,
            false,
        )
        .expect("house cartographer should parse");
        let compiled = debug_compiled_lines(&definition);
        let snapshot = CompilationSnapshot::from_definition_result(
            "House Cartographer",
            oracle,
            ParseStatus::StrictCompiled,
            None,
            Some(&definition),
            &parse_loss::ParseLossReport::default(),
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
            embedded_similarity >= lexical_similarity,
            "expected embedding-backed similarity to be at least lexical-only scoring, lexical={lexical_similarity}, embedded={embedded_similarity}, compiled={compiled:?}"
        );
    }

    #[test]
    fn compilation_snapshot_uses_same_normalized_text_as_default_cli_surface() {
        let oracle = "Enlist (As this creature attacks, you may tap a nonattacking creature you control without summoning sickness. When you do, add its power to this creature's until end of turn.)\nWhen this creature enters, create a 1/1 white Soldier creature token.";
        let definition = ironsmith_registry::compile_builder_to_runtime_definition(
            CompilerCardDefinitionBuilder::new(CardId::new(), "Argivian Cavalier"),
            oracle,
            false,
        )
        .expect("argivian cavalier should parse");

        let snapshot = CompilationSnapshot::from_definition_result(
            "Argivian Cavalier",
            oracle,
            ParseStatus::StrictCompiled,
            None,
            Some(&definition),
            &parse_loss::ParseLossReport::default(),
        );
        let stored_text = snapshot
            .compiled_text
            .expect("snapshot should include compiled text");

        assert!(
            stored_text.contains("Enlist"),
            "expected snapshot to keep the enlist keyword surface, got {stored_text}"
        );
        assert!(
            !stored_text.contains("enlist_attacker") && !stored_text.contains("enlisted_creature"),
            "expected snapshot to avoid raw enlist tags, got {stored_text}"
        );
    }

    #[test]
    fn compilation_snapshot_strips_parenthetical_text_from_stored_surfaces() {
        let oracle = "Flying (This creature can't be blocked except by creatures with flying or reach.)\nCycling {2} ({2}, Discard this card: Draw a card.)";
        let definition = ironsmith_registry::compile_builder_to_runtime_definition(
            CompilerCardDefinitionBuilder::new(CardId::new(), "Reminder Bird"),
            oracle,
            false,
        )
        .expect("reminder bird should parse");

        let snapshot = CompilationSnapshot::from_definition_result(
            "Reminder Bird",
            oracle,
            ParseStatus::StrictCompiled,
            None,
            Some(&definition),
            &parse_loss::ParseLossReport::default(),
        );

        assert_eq!(snapshot.oracle_text, "Flying\nCycling {2}");
        assert_eq!(snapshot.raw_oracle_text, oracle);
        assert_eq!(snapshot.normalized_oracle_text, "Flying\nCycling {2}");
        assert_eq!(
            snapshot.compiled_text.as_deref(),
            Some("Flying\nCycling {2}")
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
        assert!(
            db.connection()
                .prepare("SELECT uses_pseudo_oracle_fallback FROM latest_card_compilation LIMIT 0")
                .is_err(),
            "latest view should not expose removed pseudo-oracle fallback flag"
        );
        db.connection()
            .prepare(
                "SELECT normalized_oracle_text, compiled_text, parse_lossy, parse_loss_reasons, parse_loss_count, pr_created FROM latest_card_compilation LIMIT 0",
            )
            .expect("latest view should expose normalized oracle, compiled text, parse loss, and PR tracking columns");
        assert!(
            db.connection()
                .prepare("SELECT unprocessed_compiled_text FROM latest_card_compilation LIMIT 0")
                .is_err(),
            "latest view should not expose legacy unprocessed compiled text"
        );
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
    fn db_persists_compiled_text() {
        let path = unique_temp_path("compiled-text");
        let db = CardStatusDb::open(&path).expect("open db");
        let snapshot = compile_snapshot_from_payload(&pseudo_oracle_fallback_payload());

        assert!(db.insert_snapshot_if_changed(&snapshot).expect("insert"));

        let compiled: String = db
            .connection()
            .query_row(
                "SELECT compiled_text
                 FROM latest_card_compilation
                 WHERE card_name = ?1",
                ["G'raha Tia Variant"],
                |row| row.get(0),
            )
            .expect("stored compiled text");
        assert_eq!(
            compiled,
            "Reach\nWhenever other creature artifact you control dies, you draw a card. This ability triggers only once each turn."
        );
        let _ = fs::remove_file(path);
    }

    #[test]
    fn db_persists_parse_loss_provenance() {
        let path = unique_temp_path("parse-loss");
        let db = CardStatusDb::open(&path).expect("open db");
        let mut snapshot = compile_snapshot_from_payload(&lightning_bolt_payload());
        snapshot.parse_lossy = true;
        snapshot.parse_loss_reasons =
            "suffix_object_filter_recovery: parsed suffix fixture".to_string();
        snapshot.parse_loss_count = 1;
        snapshot.content_hash = snapshot.compute_content_hash();

        assert!(db.insert_snapshot_if_changed(&snapshot).expect("insert"));

        let (parse_lossy, parse_loss_reasons, parse_loss_count): (bool, String, i64) = db
            .connection()
            .query_row(
                "SELECT parse_lossy, parse_loss_reasons, parse_loss_count
                 FROM latest_card_compilation
                 WHERE card_name = ?1",
                ["Lightning Bolt"],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("stored parse loss provenance");
        assert!(parse_lossy);
        assert!(parse_loss_reasons.contains("suffix_object_filter_recovery"));
        assert!(parse_loss_count >= 1);
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
    fn historical_snapshot_can_become_latest_again_without_duplicate_row() {
        let path = unique_temp_path("latest-revert");
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
        assert!(
            db.insert_snapshot_if_changed(&base)
                .expect("restore original snapshot")
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
        assert_eq!(latest, 1.0);

        let latest_hash = db
            .latest_snapshot_hash("Lightning Bolt")
            .expect("fetch latest lightning hash");
        assert_eq!(latest_hash.as_deref(), Some(base.content_hash.as_str()));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn batch_insert_snapshots_if_changed_preserves_latest_and_dedupes_history() {
        let path = unique_temp_path("batch-insert");
        let mut db = CardStatusDb::open(&path).expect("open db");
        let base = compile_snapshot_from_payload(&lightning_bolt_payload());
        let mut changed = base.clone();
        changed.similarity_score = 0.5;
        changed.content_hash = changed.compute_content_hash();

        let changed_count = db
            .insert_snapshots_if_changed(&[base.clone(), changed, base.clone()])
            .expect("batch insert snapshots");
        assert_eq!(changed_count, 3);

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
        assert_eq!(latest, base.similarity_score);

        let latest_hash = db
            .latest_snapshot_hash("Lightning Bolt")
            .expect("fetch latest lightning hash");
        assert_eq!(latest_hash.as_deref(), Some(base.content_hash.as_str()));
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
            parse_name: None,
            oracle_text: "Shock deals 2 damage to any target.".to_string(),
            raw_oracle_text: "Shock deals 2 damage to any target.".to_string(),
            metadata_lines: vec!["Mana cost: {R}".to_string(), "Type: Instant".to_string()],
            parse_input: "Mana cost: {R}\nType: Instant\nShock deals 2 damage to any target."
                .to_string(),
            other_face_name: None,
            linked_face_layout: None,
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
            parse_name: None,
            oracle_text: "Shock deals 2 damage to any target.".to_string(),
            raw_oracle_text: "Shock deals 2 damage to any target.".to_string(),
            metadata_lines: vec!["Mana cost: {R}".to_string(), "Type: Instant".to_string()],
            parse_input: "Mana cost: {R}\nType: Instant\nShock deals 2 damage to any target."
                .to_string(),
            other_face_name: None,
            linked_face_layout: None,
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

    #[test]
    fn replace_registry_cards_stores_stripped_oracle_text_but_keeps_raw_parse_input() {
        let path = unique_temp_path("registry-strip");
        let mut db = CardStatusDb::open(&path).expect("open db");
        let rows = load_registry_cards_from_values(vec![serde_json::json!({
            "name": "Sky Drake",
            "oracle_text": "Flying (This creature can't be blocked except by creatures with flying or reach.)",
            "mana_cost": "{1}{U}",
            "type_line": "Creature — Drake",
            "power": "2",
            "toughness": "1"
        })]);
        let record = rows
            .get("Sky Drake")
            .expect("sky drake registry row")
            .clone();

        db.replace_registry_cards(&[record])
            .expect("replace registry row");

        let (oracle_text, raw_oracle_text, parse_input): (String, String, String) = db
            .connection()
            .query_row(
                "SELECT oracle_text, raw_oracle_text, parse_input FROM registry_card WHERE card_name = 'Sky Drake'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("query registry row");
        assert_eq!(oracle_text, "Flying");
        assert_eq!(
            raw_oracle_text,
            "Flying (This creature can't be blocked except by creatures with flying or reach.)"
        );
        assert!(parse_input.contains(
            "Flying (This creature can't be blocked except by creatures with flying or reach.)"
        ));

        let _ = fs::remove_file(path);
    }
}
