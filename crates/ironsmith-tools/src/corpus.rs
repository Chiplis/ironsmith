//! Corpus observations have one canonical aggregate and independent face snapshots.
use crate::{CardPayload, CompilationSnapshot, ParseStatus, load_registry_cards};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::error::Error;

#[derive(Debug)]
pub struct CorpusCard {
    pub name: String,
    pub faces: Vec<CardPayload>,
}

impl CorpusCard {
    pub(crate) fn from_record(payload: CardPayload, card: &Value) -> Result<Self, Box<dyn Error>> {
        let name = payload.name.clone();
        let faces = match card.get("card_faces").and_then(Value::as_array) {
            Some(faces) if !faces.is_empty() => (0..faces.len())
                .map(|index| {
                    crate::tooling::build_card_payload_for_face(card, index)
                        .ok_or_else(|| format!("{name}: invalid face at index {index}").into())
                })
                .collect::<Result<Vec<_>, Box<dyn Error>>>()?,
            _ => vec![payload],
        };
        Ok(Self { name, faces })
    }
}

pub fn load_corpus_cards(path: &str) -> Result<Vec<CorpusCard>, Box<dyn Error>> {
    load_registry_cards(path)?
        .into_values()
        .map(|record| {
            let raw: Value = serde_json::from_str(&record.raw_card_json)?;
            CorpusCard::from_record(record.payload, &raw)
        })
        .collect()
}

/// The weakest face determines coverage. Keep each face's full diagnostics in
/// the face table, including when another face prevents a whole-card compile.
pub fn aggregate_face_snapshots(name: &str, faces: &[CompilationSnapshot]) -> CompilationSnapshot {
    assert!(
        !faces.is_empty(),
        "a corpus card must have at least one face"
    );
    let mut aggregate = faces[0].clone();
    if faces.len() == 1 && aggregate.card_name == name {
        return aggregate;
    }
    aggregate.card_name = name.to_owned();
    let join = |field: fn(&CompilationSnapshot) -> &str| {
        faces
            .iter()
            .map(|face| format!("{}:\n{}", face.card_name, field(face)))
            .collect::<Vec<_>>()
            .join("\n\n")
    };
    aggregate.oracle_text = join(|f| &f.oracle_text);
    aggregate.raw_oracle_text = join(|f| &f.raw_oracle_text);
    aggregate.normalized_oracle_text = join(|f| &f.normalized_oracle_text);
    aggregate.parse_status = if faces
        .iter()
        .any(|f| f.parse_status == ParseStatus::ParseFailed)
    {
        ParseStatus::ParseFailed
    } else if faces
        .iter()
        .any(|f| f.parse_status != ParseStatus::StrictCompiled)
    {
        ParseStatus::CompiledWithAllowUnsupported
    } else {
        ParseStatus::StrictCompiled
    };
    let errors = faces
        .iter()
        .filter_map(|f| {
            f.parse_error
                .as_ref()
                .map(|error| format!("{}: {error}", f.card_name))
        })
        .collect::<Vec<_>>();
    aggregate.parse_error = (!errors.is_empty()).then(|| errors.join("\n"));
    aggregate.compiled_text = faces
        .iter()
        .all(|f| f.compiled_text.is_some())
        .then(|| join(|f| f.compiled_text.as_deref().unwrap_or_default()));
    aggregate.compiled_card_definition = faces
        .iter()
        .all(|f| f.compiled_card_definition.is_some())
        .then(|| join(|f| f.compiled_card_definition.as_deref().unwrap_or_default()));
    aggregate.similarity_score = faces.iter().map(|f| f.similarity_score).fold(1.0, f32::min);
    aggregate.oracle_coverage = faces.iter().map(|f| f.oracle_coverage).fold(1.0, f32::min);
    aggregate.compiled_coverage = faces
        .iter()
        .map(|f| f.compiled_coverage)
        .fold(1.0, f32::min);
    aggregate.line_delta = faces.iter().map(|f| f.line_delta).sum();
    aggregate.semantic_mismatch = faces.iter().any(|f| f.semantic_mismatch);
    aggregate.has_unimplemented = faces.iter().any(|f| f.has_unimplemented);
    aggregate.parse_lossy = faces.iter().any(|f| f.parse_lossy);
    aggregate.parse_loss_count = faces.iter().map(|f| f.parse_loss_count).sum();
    aggregate.parse_loss_reasons = faces
        .iter()
        .filter(|f| !f.parse_loss_reasons.is_empty())
        .map(|f| format!("{}: {}", f.card_name, f.parse_loss_reasons))
        .collect::<Vec<_>>()
        .join("\n");
    // Hash every independent observation, including successful faces when the
    // aggregate fails. A back-face-only change must produce a new observation.
    aggregate.content_hash = Sha256::digest(
        serde_json::to_vec(&("corpus-faces-v1", name, faces)).expect("serialize face snapshots"),
    )
    .iter()
    .map(|byte| format!("{byte:02x}"))
    .collect();
    aggregate
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CardStatusDb, compile_strict_snapshot_from_payload};
    use serde_json::json;

    fn fixture() -> Value {
        json!([{
            "name": "Front // Back", "layout": "modal_dfc",
            "card_faces": [
                {"name": "Front", "type_line": "Sorcery", "mana_cost": "{U}", "oracle_text": "Draw a card."},
                {"name": "Back", "type_line": "Sorcery", "mana_cost": "{U}", "oracle_text": "Choose a word. You win the game if a sentence contains that word."}
            ]
        }])
    }

    #[test]
    fn corpus_faces_persist_failures_and_recover_without_polluting_card_counts() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("cards.json");
        std::fs::write(&source, fixture().to_string()).unwrap();
        let card = load_corpus_cards(source.to_str().unwrap())
            .unwrap()
            .remove(0);
        assert_eq!(card.faces.len(), 2);
        let mut faces = card
            .faces
            .iter()
            .map(compile_strict_snapshot_from_payload)
            .collect::<Vec<_>>();
        assert_eq!(faces[0].parse_status, ParseStatus::StrictCompiled);
        assert_eq!(faces[1].parse_status, ParseStatus::ParseFailed);
        let failed = aggregate_face_snapshots(&card.name, &faces);
        assert_eq!(failed.parse_status, ParseStatus::ParseFailed);
        assert_eq!(failed.similarity_score, 0.0);
        assert!(failed.parse_error.as_ref().unwrap().contains("Back:"));
        assert!(failed.compiled_text.is_none());
        let path = dir.path().join("status.sqlite3");
        let mut db = CardStatusDb::open(&path).unwrap();
        db.insert_corpus_snapshots_if_changed(&[(failed.clone(), faces.clone())])
            .unwrap();
        assert_eq!(
            db.insert_corpus_snapshots_if_changed(&[(failed.clone(), faces.clone())])
                .unwrap(),
            0
        );
        let conn = rusqlite::Connection::open(&path).unwrap();
        let count = |table: &str| {
            conn.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| {
                r.get::<_, i64>(0)
            })
            .unwrap()
        };
        assert_eq!(count("latest_card_compilation"), 1);
        assert_eq!(count("latest_card_face_compilation"), 2);
        let status: String = conn
            .query_row(
                "SELECT parse_status FROM latest_card_face_compilation WHERE face_index = 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(status, "parse_failed");
        faces[1] = faces[0].clone();
        faces[1].card_name = "Back".into();
        faces[1].similarity_score = 0.75;
        faces[1].semantic_mismatch = true;
        let recovered = aggregate_face_snapshots(&card.name, &faces);
        assert_eq!(recovered.parse_status, ParseStatus::StrictCompiled);
        assert_eq!(recovered.similarity_score, 0.75);
        assert!(recovered.semantic_mismatch);
        assert_ne!(recovered.content_hash, failed.content_hash);
        db.insert_corpus_snapshots_if_changed(&[(recovered, faces)])
            .unwrap();
        assert_eq!(count("card_face_compilation"), 4);
        assert_eq!(count("latest_card_face_compilation"), 2);
        db.prune_compilation_history_to_latest().unwrap();
        assert_eq!(count("card_face_compilation"), 2);
        drop(db);
        let db = CardStatusDb::open(&path).unwrap();
        assert_eq!(count("latest_card_compilation"), 1);
        drop(db);
    }

    #[test]
    fn corpus_faces_db_and_file_sources_match_for_every_layout() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("cards.json");
        let mut db = CardStatusDb::open(dir.path().join("status.sqlite3")).unwrap();
        for layout in [
            "modal_dfc",
            "transform",
            "adventure",
            "split",
            "flip",
            "prepare",
            "reversible_card",
        ] {
            let mut cards = fixture();
            cards[0]["layout"] = json!(layout);
            std::fs::write(&source, cards.to_string()).unwrap();
            let records = load_registry_cards(source.to_str().unwrap())
                .unwrap()
                .into_values()
                .collect::<Vec<_>>();
            db.replace_registry_cards(&records).unwrap();
            let from_file = load_corpus_cards(source.to_str().unwrap()).unwrap();
            let from_db = db.registry_corpus_cards().unwrap();
            assert_eq!(from_file[0].faces, from_db[0].faces, "{layout}");
            assert_eq!(from_db[0].faces.len(), 2, "{layout}");
            assert_eq!(from_db[0].faces[1].name, "Back");
        }
        let mut invalid = fixture();
        invalid[0]["card_faces"][1]["name"] = json!("");
        std::fs::write(&source, invalid.to_string()).unwrap();
        assert!(load_corpus_cards(source.to_str().unwrap()).is_err());
    }
}
