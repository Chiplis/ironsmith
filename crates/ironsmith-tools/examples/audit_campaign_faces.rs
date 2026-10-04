//! Audit every exact source-face context without name-based corpus deduplication.
//!
//! cargo run --locked -p ironsmith-tools --example audit_campaign_faces --
//!   --cards cards.json --out faces.jsonl --expected-sha256 <source digest>
//! Add --inventory-only to verify/export payload routes without compiling.
//!
//! Each unmodified source object is isolated before calling the existing public
//! single-name loader. That loader returns ALL matching faces (unlike the batch
//! name map). This preserves both reversible faces and shared prepared spells.
//! The output is additive evidence; it never updates the canonical status DB.
use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::Path;

use ironsmith_tools::{
    CardPayload, CompilationSnapshot, compile_authoritative_snapshot_from_payload,
    load_card_payloads_by_name, normalize_lookup_name,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|byte| format!("{byte:02x}")).collect()
}

fn write_record(out: &mut impl Write, record: Value) -> Result<()> {
    serde_json::to_writer(&mut *out, &record)?;
    writeln!(out)?;
    out.flush()?;
    Ok(())
}

fn face_text(face: &Value) -> String {
    match face.get("oracle_text") {
        None | Some(Value::Null) => String::new(),
        Some(Value::String(s)) => s.trim().to_string(),
        Some(value) => value.to_string().trim().to_string(),
    }
}

fn isolated_payloads(card: &Value, path: &Path) -> Result<Vec<(usize, CardPayload)>> {
    let faces = card["card_faces"].as_array().ok_or("missing faces")?;
    fs::write(path, serde_json::to_vec(&[card])?)?;
    let mut groups: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (index, face) in faces.iter().enumerate() {
        let name = face["name"].as_str().ok_or("missing face name")?;
        if name.trim().is_empty() {
            return Err("empty face name".into());
        }
        let key = normalize_lookup_name(name);
        if card["name"].as_str().map(normalize_lookup_name).as_ref() == Some(&key) {
            return Err("face query collides with its own canonical source name".into());
        }
        groups.entry(key).or_default().push(index);
    }
    let mut out = Vec::new();
    for (name, indexes) in groups {
        let payloads = load_card_payloads_by_name(path.to_str().ok_or("non-UTF8 path")?, &name)
            .map_err(|e| e.to_string())?;
        if payloads.len() != indexes.len() {
            return Err(format!(
                "{name}: expected {} exact face payloads, got {}",
                indexes.len(),
                payloads.len()
            )
            .into());
        }
        for (index, payload) in indexes.into_iter().zip(payloads) {
            let face = &faces[index];
            if payload.name != face["name"].as_str().unwrap().trim()
                || payload.raw_oracle_text != face_text(face)
                || payload.parse_name.is_some()
            {
                return Err(format!("{name}: selected payload does not match face {index}").into());
            }
            out.push((index, payload));
        }
    }
    out.sort_by_key(|(index, _)| *index);
    Ok(out)
}

fn snapshot_json(snapshot: CompilationSnapshot) -> Value {
    let definition_hash = snapshot
        .compiled_card_definition
        .as_ref()
        .map(|s| sha256(s.as_bytes()));
    json!({
        "card_name": snapshot.card_name,
        "oracle_text": snapshot.oracle_text,
        "raw_oracle_text": snapshot.raw_oracle_text,
        "parse_status": snapshot.parse_status.as_str(),
        "parse_error": snapshot.parse_error,
        "normalized_oracle_text": snapshot.normalized_oracle_text,
        "compiled_text": snapshot.compiled_text,
        "compiled_card_definition": snapshot.compiled_card_definition,
        "compiled_card_definition_sha256": definition_hash,
        "oracle_coverage": snapshot.oracle_coverage,
        "compiled_coverage": snapshot.compiled_coverage,
        "similarity_score": snapshot.similarity_score,
        "line_delta": snapshot.line_delta,
        "semantic_mismatch": snapshot.semantic_mismatch,
        "has_unimplemented": snapshot.has_unimplemented,
        "parse_lossy": snapshot.parse_lossy,
        "parse_loss_reasons": snapshot.parse_loss_reasons,
        "parse_loss_count": snapshot.parse_loss_count,
        "content_hash": snapshot.content_hash,
    })
}

fn run() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let (mut cards_path, mut out_path, mut expected_sha) = (None, None, None);
    let mut inventory_only = false;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--cards" => cards_path = Some(args.next().ok_or("--cards requires a path")?),
            "--out" => out_path = Some(args.next().ok_or("--out requires a path")?),
            "--expected-sha256" => {
                expected_sha = Some(args.next().ok_or("--expected-sha256 requires a hash")?)
            }
            "--inventory-only" => inventory_only = true,
            _ => return Err(format!("unknown argument {arg}").into()),
        }
    }
    let cards_path = cards_path.ok_or("--cards required")?;
    let out_path = out_path.ok_or("--out required")?;
    let expected_sha = expected_sha.ok_or("--expected-sha256 required")?;
    let bytes = fs::read(&cards_path)?;
    let source_sha = sha256(&bytes);
    if source_sha != expected_sha {
        return Err("source SHA-256 does not match expected digest".into());
    }
    let cards: Vec<Value> = serde_json::from_slice(&bytes)?;
    drop(bytes);
    let expected_count: usize = cards
        .iter()
        .filter_map(|c| c["card_faces"].as_array())
        .map(Vec::len)
        .sum();
    let tempfile = tempfile::NamedTempFile::new()?;
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(out_path)?;
    let mut out = BufWriter::new(file);
    write_record(
        &mut out,
        json!({"kind": "header", "schema_version": 1,
        "source_sha256": source_sha, "source_entry_count": cards.len(),
        "face_route_count": expected_count, "inventory_only": inventory_only,
        "route": "isolated unmodified source object -> load_card_payloads_by_name -> authoritative snapshot",
        "gameplay_verified": false, "artifact_baker_verified": false}),
    )?;
    let mut count = 0;
    for (source_index, card) in cards.iter().enumerate() {
        let Some(faces) = card["card_faces"].as_array() else {
            continue;
        };
        for (face_index, payload) in isolated_payloads(card, tempfile.path())? {
            let snapshot = if inventory_only {
                Value::Null
            } else {
                snapshot_json(compile_authoritative_snapshot_from_payload(&payload))
            };
            let face = &faces[face_index];
            write_record(
                &mut out,
                json!({"kind": "face", "route_id": format!("source:{source_index}/face:{face_index}"),
                "source_index": source_index, "source_name": card["name"], "source_id": card["id"],
                "source_layout": card["layout"], "top_level_oracle_id": card["oracle_id"],
                "oracle_id": face.get("oracle_id").or(card.get("oracle_id")),
                "face_index": face_index, "face_name": face["name"],
                "payload": {"name": payload.name, "parse_name": payload.parse_name,
                    "oracle_text": payload.oracle_text, "raw_oracle_text": payload.raw_oracle_text,
                    "metadata_lines": payload.metadata_lines, "parse_input": payload.parse_input,
                    "other_face_name": payload.other_face_name,
                    "linked_face_layout": payload.linked_face_layout.map(|v| format!("{v:?}"))},
                "snapshot": snapshot}),
            )?;
            count += 1;
        }
    }
    if count != expected_count || sha256(&fs::read(cards_path)?) != source_sha {
        return Err("route membership or source bytes changed during audit".into());
    }
    write_record(
        &mut out,
        json!({"kind": "complete", "face_route_count": count, "source_sha256": source_sha}),
    )?;
    eprintln!("Completed {count} exact source-face routes; inventory_only={inventory_only}");
    Ok(())
}

fn main() -> Result<()> {
    std::thread::Builder::new()
        .name("audit-campaign-faces".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(run)?
        .join()
        .map_err(|_| "face audit worker panicked")?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_name_reversible_faces_keep_distinct_metadata() {
        let source = json!({"name": "Twin // Twin", "layout": "reversible_card", "card_faces": [
            {"name": "Twin", "type_line": "Creature", "oracle_text": "Flying", "power": "1", "toughness": "2"},
            {"name": "Twin", "type_line": "Creature", "oracle_text": "Vigilance", "power": "3", "toughness": "4"}
        ]});
        let file = tempfile::NamedTempFile::new().unwrap();
        let rows = isolated_payloads(&source, file.path()).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].0, 0);
        assert_eq!(rows[1].0, 1);
        assert!(rows[0].1.parse_input.contains("Power/Toughness: 1/2"));
        assert!(rows[1].1.parse_input.contains("Power/Toughness: 3/4"));
        assert_eq!(rows[0].1.raw_oracle_text, "Flying");
        assert_eq!(rows[1].1.raw_oracle_text, "Vigilance");
    }

    #[test]
    fn prepare_face_keeps_its_own_source_context() {
        let source = json!({"name": "Scholar // Shared", "layout": "prepare", "first_printed_set_name": "Prepared Set",
            "card_faces": [{"name": "Scholar", "oracle_text": "Flying"}, {"name": "Shared", "oracle_text": "Draw a card."}]});
        let file = tempfile::NamedTempFile::new().unwrap();
        let rows = isolated_payloads(&source, file.path()).unwrap();
        assert_eq!(rows[1].1.name, "Shared");
        assert!(
            rows[1]
                .1
                .parse_input
                .contains("First printed set: Prepared Set")
        );
    }

    #[test]
    fn ambiguous_canonical_face_name_is_rejected() {
        let source = json!({"name": "Same", "layout": "transform", "card_faces": [{"name": "Same"}, {"name": "Other"}]});
        let file = tempfile::NamedTempFile::new().unwrap();
        assert!(
            isolated_payloads(&source, file.path())
                .unwrap_err()
                .to_string()
                .contains("canonical")
        );
    }
}
