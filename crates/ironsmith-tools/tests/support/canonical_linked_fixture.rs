//! Audit-only catalog linkage, derived from an actual cards.json face group.
//! No abilities, effects, costs, or states are synthesized here.
use ironsmith::{CardDefinition, CardId, GameState};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::HashMap, path::Path};
pub struct LinkedFamily {
    pub definitions: HashMap<String, (CardDefinition, String)>,
    pub unlinked_artifacts: Vec<Value>,
    pub linked_artifacts: Vec<Value>,
    pub metadata_record: Value,
}
fn hash(data: &[u8]) -> String {
    Sha256::digest(data)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
struct CatalogCache {
    catalog: Value,
    inventory: Value,
    catalog_sha: String,
    inventory_sha: String,
}
fn catalog_cache(root: &Path) -> Result<&'static CatalogCache, String> {
    static CACHE: std::sync::OnceLock<Result<CatalogCache, String>> = std::sync::OnceLock::new();
    CACHE
        .get_or_init(|| {
            let bytes = std::fs::read(root.join("cards.json")).map_err(|e| e.to_string())?;
            let all: Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
            let catalog = Value::Array(
                all.as_array()
                    .unwrap()
                    .iter()
                    .filter(|c| c["card_faces"].is_array())
                    .cloned()
                    .collect(),
            );
            let inv = std::fs::read(
                root.join("reports/runtime-audit/corpus/267a16aff3b321196397d0b4/inventory.json"),
            )
            .map_err(|e| e.to_string())?;
            let inventory = serde_json::from_slice(&inv).map_err(|e| e.to_string())?;
            Ok(CatalogCache {
                catalog,
                inventory,
                catalog_sha: hash(&bytes),
                inventory_sha: hash(&inv),
            })
        })
        .as_ref()
        .map_err(Clone::clone)
}
impl LinkedFamily {
    pub fn from_catalog(root: &Path, combined_name: &str) -> Result<Self, String> {
        let cache = catalog_cache(root)?;
        let catalog = &cache.catalog;
        let candidates = catalog
            .as_array()
            .unwrap()
            .iter()
            .filter(|c| c["name"] == combined_name)
            .collect::<Vec<_>>();
        let card = candidates.first().ok_or("canonical linked group missing")?;
        let faces = card["card_faces"]
            .as_array()
            .ok_or("canonical faces missing")?;
        if faces.len() != 2 {
            return Err("exactly two canonical faces required".into());
        }
        let layout = card["layout"].as_str().ok_or("layout missing")?;
        if !["transform", "prepare"].contains(&layout) {
            return Err("unsupported audit linkage layout".into());
        }
        for other in &candidates {
            if other["layout"] != card["layout"]
                || other["card_faces"]
                    .as_array()
                    .map(|fs| fs.iter().map(|f| f["name"].clone()).collect::<Vec<_>>())
                    != Some(faces.iter().map(|f| f["name"].clone()).collect())
            {
                return Err("ambiguous canonical face relationship".into());
            }
        }
        let inventory = &cache.inventory;
        let names = faces
            .iter()
            .map(|f| f["name"].as_str().unwrap())
            .collect::<Vec<_>>();
        let ids = [CardId::new(), CardId::new()];
        let mut definitions = HashMap::new();
        let mut unlinked_artifacts = vec![];
        let mut linked_artifacts = vec![];
        let mut deltas = vec![];
        for (i, name) in names.iter().enumerate() {
            let payload = inventory["cards"]
                .as_array()
                .unwrap()
                .iter()
                .find(|v| v["name"] == *name)
                .ok_or_else(|| format!("frozen canonical face missing: {name}"))?;
            // Frozen text is kept byte-for-byte; face identity/layout comes only from cards.json.
            let b = ironsmith_compiler::CardDefinitionBuilder::new(
                ids[i],
                payload["parse_name"].as_str().unwrap_or(name),
            );
            let (mut artifact, _) = ironsmith_registry::compile_builder_to_artifact(
                b,
                payload["parse_input"].as_str().unwrap(),
                false,
            )
            .map_err(|e| e.to_string())?;
            unlinked_artifacts.push(json!({"card":name,"artifact_checksum":artifact.payload_checksum,"definition":artifact.payload.definition,"input":payload}));
            let before = json!({"other_face":artifact.payload.definition.card.other_face,"other_face_name":artifact.payload.definition.card.other_face_name,"linked_face_layout":artifact.payload.definition.card.linked_face_layout,"transforming_dfc":artifact.payload.definition.card.transforming_dfc});
            // These are the same catalog metadata fields populated by compiler-wasm and external_registry.
            artifact.payload.definition.card.other_face = Some(ids[1 - i]);
            artifact.payload.definition.card.other_face_name = Some(names[1 - i].to_string());
            artifact.payload.definition.card.linked_face_layout = if layout == "prepare" {
                ironsmith::card::LinkedFaceLayout::Prepare
            } else {
                ironsmith::card::LinkedFaceLayout::TransformLike
            };
            artifact.payload.definition.card.transforming_dfc = layout == "transform";
            artifact.card.local_id.0 = ids[i].0;
            let mut other_local = artifact.card.local_id;
            other_local.0 = ids[1 - i].0;
            artifact.card.other_face = Some(other_local);
            artifact.card.linked_face_layout = Some(format!(
                "{:?}",
                artifact.payload.definition.card.linked_face_layout
            ));
            artifact.refresh_checksum();
            artifact.validate().map_err(|e| e.to_string())?;
            let def = ironsmith::artifact_materializer::materialize_artifact(&artifact)
                .map_err(|e| e.to_string())?;
            deltas.push(json!({"card":name,"before":before,"after":{"other_face":def.card.other_face,"other_face_name":def.card.other_face_name,"linked_face_layout":def.card.linked_face_layout,"transforming_dfc":def.card.transforming_dfc},"artifact_identity":{"local_id":artifact.card.local_id,"other_face":artifact.card.other_face,"linked_face_layout":artifact.card.linked_face_layout}}));
            linked_artifacts.push(json!({"card":name,"artifact_checksum":artifact.payload_checksum,"definition":artifact.payload.definition,"artifact":artifact}));
            definitions.insert(name.to_string(), (def, artifact.payload_checksum));
        }
        Ok(Self {
            definitions,
            unlinked_artifacts,
            linked_artifacts,
            metadata_record: json!({"combined_name":combined_name,"cards_json_sha256":cache.catalog_sha,"inventory_sha256":cache.inventory_sha,"catalog_record_id":card["id"],"layout":layout,"canonical_faces":faces,"metadata_deltas":deltas,"scope":"Strict frozen face inputs compiled unchanged. Four linkage metadata fields and artifact-local paired IDs derived exclusively from actual cards.json face relationship. Artifacts refreshed, validated and materialized through production runtime materializer; game registration uses production public linked-face cache. This native test does not execute JS binding or WASM batch cardinality/remapping boundary."}),
        })
    }
    pub fn register(&self, g: &mut GameState) {
        for (d, _) in self.definitions.values() {
            g.register_linked_face_definition(d);
        }
    }
}
