use std::collections::{HashMap, HashSet, VecDeque};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use ironsmith_compiled_artifact::{
    ArtifactCardId, ArtifactCardIdentity, CompiledCardArtifact, CompiledCardPayload, sha256_hex,
    wire_definition_from_serializable,
};
use ironsmith_compiler::card::LinkedFaceLayout;
use ironsmith_compiler::{CardDefinitionBuilder, CompilePolicy, CompilerFacade, ids::CardId};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CardSourceFile {
    #[serde(default)]
    canonical_name: String,
    group: CardSourceGroup,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
enum CardSourceGroup {
    Single {
        name: String,
        block: String,
        #[serde(default)]
        score: Option<f32>,
    },
    Linked {
        layout: String,
        faces: Vec<CardFaceSource>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CardFaceSource {
    name: String,
    block: String,
    #[serde(default)]
    score: Option<f32>,
}

struct CompileInput<'a> {
    name: &'a str,
    text: &'a str,
    score: Option<f32>,
    local_id: u32,
    other_face_id: Option<u32>,
    other_face_name: Option<&'a str>,
    layout: LinkedFaceLayout,
}

fn artifact_compiler_version() -> &'static str {
    static VERSION: OnceLock<String> = OnceLock::new();
    VERSION.get_or_init(|| {
        format!(
            "ironsmith-artifact-baker/{}+{}",
            env!("CARGO_PKG_VERSION"),
            option_env!("IRONSMITH_ARTIFACT_COMPILER_FINGERPRINT").unwrap_or("development")
        )
    })
}

fn compile_artifact(input: CompileInput<'_>) -> Result<CompiledCardArtifact, String> {
    let builder = CardDefinitionBuilder::new(CardId::from_raw(input.local_id), input.name);
    let mut compiled = CompilerFacade::new()
        .compile_definition(
            builder,
            input.text.to_string(),
            CompilePolicy {
                allow_unsupported: false,
            },
        )
        .map_err(|error| error.to_string())?;
    compiled.definition.card.id = CardId::from_raw(input.local_id);
    compiled.definition.card.other_face = input.other_face_id.map(CardId::from_raw);
    compiled.definition.card.other_face_name = input.other_face_name.map(str::to_string);
    compiled.definition.card.linked_face_layout = input.layout;
    let definition = wire_definition_from_serializable(&compiled.definition)
        .map_err(|error| format!("failed to encode {}: {error}", input.name))?;
    let runtime_definition =
        ironsmith_runtime_catalog::artifact_materializer::materialize_definition(
            definition.clone(),
        )
        .map_err(|error| format!("failed to materialize {}: {error}", input.name))?;
    let canonical_text = ironsmith_text::compiled_text_lines(&runtime_definition).join("\n");
    let ability_labels = ironsmith_text::ability_surface_texts(&runtime_definition);
    let mut artifact = CompiledCardArtifact::new(
        ArtifactCardIdentity {
            local_id: ArtifactCardId(input.local_id),
            name: compiled.definition.card.name.clone(),
            face_name: Some(input.name.to_string()),
            other_face: input.other_face_id.map(ArtifactCardId),
            linked_face_layout: Some(format!("{:?}", input.layout)),
        },
        CompiledCardPayload {
            definition,
            canonical_text,
            ability_labels,
        },
        artifact_compiler_version(),
        input.text.as_bytes(),
    );
    artifact.semantic_score = input.score.map(|score| score.clamp(0.0, 1.0));
    artifact.refresh_checksum();
    Ok(artifact)
}

fn compile_source(source: &CardSourceFile) -> Result<Vec<CompiledCardArtifact>, String> {
    match &source.group {
        CardSourceGroup::Single { name, block, score } => {
            if score.is_none() {
                return Err(format!(
                    "{name}: no validated strict compilation snapshot is available"
                ));
            }
            Ok(vec![compile_artifact(CompileInput {
                name,
                text: block,
                score: *score,
                local_id: 1,
                other_face_id: None,
                other_face_name: None,
                layout: LinkedFaceLayout::None,
            })?])
        }
        CardSourceGroup::Linked { layout, faces } => {
            if faces.len() != 2 {
                return Err(format!(
                    "{}: linked source must contain exactly two faces",
                    source.canonical_name
                ));
            }
            if let Some(face) = faces.iter().find(|face| face.score.is_none()) {
                return Err(format!(
                    "{}: no validated strict compilation snapshot is available",
                    face.name
                ));
            }
            let layout = if layout == "split" {
                LinkedFaceLayout::Split
            } else {
                LinkedFaceLayout::TransformLike
            };
            faces
                .iter()
                .enumerate()
                .map(|(index, face)| {
                    let other_index = usize::from(index == 0);
                    compile_artifact(CompileInput {
                        name: &face.name,
                        text: &face.block,
                        score: face.score,
                        local_id: index as u32 + 1,
                        other_face_id: Some(other_index as u32 + 1),
                        other_face_name: Some(&faces[other_index].name),
                        layout,
                    })
                })
                .collect()
        }
    }
}

fn source_cache_key(source: &CardSourceFile) -> Result<String, String> {
    serde_json::to_string(source).map_err(|error| error.to_string())
}

fn source_texts(source: &CardSourceFile) -> Vec<&str> {
    match &source.group {
        CardSourceGroup::Single { block, .. } => vec![block],
        CardSourceGroup::Linked { faces, .. } => {
            faces.iter().map(|face| face.block.as_str()).collect()
        }
    }
}

fn valid_existing_artifacts(
    payload: &Value,
    source: &CardSourceFile,
) -> Option<(Value, Vec<CompiledCardArtifact>)> {
    let value = payload.get("artifacts")?.clone();
    let artifacts: Vec<CompiledCardArtifact> = serde_json::from_value(value.clone()).ok()?;
    let texts = source_texts(source);
    if artifacts.len() != texts.len()
        || artifacts.iter().zip(texts).any(|(artifact, text)| {
            artifact.validate().is_err()
                || artifact.compiler_version != artifact_compiler_version()
                || artifact.source_checksum != sha256_hex(text.as_bytes())
        })
    {
        return None;
    }
    Some((value, artifacts))
}

fn card_asset_paths(cards_dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut paths = fs::read_dir(cards_dir)
        .map_err(|error| format!("failed to read {}: {error}", cards_dir.display()))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "json")
                && path.file_name().is_some_and(|name| name != "index.json")
        })
        .collect::<Vec<_>>();
    paths.sort();
    Ok(paths)
}

struct PendingRoute {
    path: PathBuf,
    payload: Value,
}

struct CompileJob {
    source: CardSourceFile,
    routes: Vec<PendingRoute>,
}

struct BakeSummary {
    written: usize,
    retained: usize,
    unique_sources: usize,
    failed: usize,
    workers: usize,
    scan_time: Duration,
    work_time: Duration,
    total_time: Duration,
}

fn write_compiled_route(
    mut route: PendingRoute,
    compiled: &Result<Value, String>,
) -> Result<bool, String> {
    let object = route
        .payload
        .as_object_mut()
        .ok_or_else(|| format!("card asset {} is not an object", route.path.display()))?;
    let failed = match compiled {
        Ok(artifacts) => {
            object.insert("artifacts".to_string(), artifacts.clone());
            object.remove("artifactError");
            false
        }
        Err(error) => {
            object.remove("artifacts");
            object.insert("artifactError".to_string(), Value::String(error.clone()));
            true
        }
    };
    let temporary = route.path.with_extension("json.tmp");
    fs::write(
        &temporary,
        serde_json::to_vec(&route.payload).map_err(|error| error.to_string())?,
    )
    .map_err(|error| format!("failed to write {}: {error}", temporary.display()))?;
    fs::rename(&temporary, &route.path)
        .map_err(|error| format!("failed to replace {}: {error}", route.path.display()))?;
    Ok(failed)
}

fn bake(cards_dir: &Path, requested_workers: Option<usize>) -> Result<BakeSummary, String> {
    let started = Instant::now();
    let paths = card_asset_paths(cards_dir)?;
    let mut jobs = HashMap::<String, CompileJob>::new();
    let mut source_keys = HashSet::new();
    let mut retained = 0usize;
    for path in &paths {
        let payload: Value = serde_json::from_slice(
            &fs::read(path)
                .map_err(|error| format!("failed to read {}: {error}", path.display()))?,
        )
        .map_err(|error| format!("invalid {}: {error}", path.display()))?;
        let source: CardSourceFile = serde_json::from_value(payload.clone())
            .map_err(|error| format!("invalid card source {}: {error}", path.display()))?;
        let key = source_cache_key(&source)?;
        source_keys.insert(key.clone());
        if valid_existing_artifacts(&payload, &source).is_some() {
            retained += 1;
            continue;
        }
        jobs.entry(key)
            .or_insert_with(|| CompileJob {
                source,
                routes: Vec::new(),
            })
            .routes
            .push(PendingRoute {
                path: path.clone(),
                payload,
            });
    }

    let scan_time = started.elapsed();
    let pending_routes = paths.len() - retained;
    let queue = Arc::new(Mutex::new(jobs.into_values().collect::<VecDeque<_>>()));
    let written = Arc::new(AtomicUsize::new(0));
    let failed = Arc::new(AtomicUsize::new(0));
    let fatal_error = Arc::new(Mutex::new(None::<String>));
    let worker_count = requested_workers
        .unwrap_or_else(|| {
            std::thread::available_parallelism()
                .map(|count| count.get())
                .unwrap_or(1)
        })
        .clamp(1, 64);
    let work_started = Instant::now();
    let mut workers = Vec::with_capacity(worker_count);
    for worker_index in 0..worker_count {
        let queue = Arc::clone(&queue);
        let written = Arc::clone(&written);
        let failed = Arc::clone(&failed);
        let fatal_error = Arc::clone(&fatal_error);
        workers.push(
            std::thread::Builder::new()
                .name(format!("artifact-baker-{worker_index}"))
                .stack_size(64 * 1024 * 1024)
                .spawn(move || {
                    loop {
                        if fatal_error.lock().expect("error lock poisoned").is_some() {
                            return;
                        }
                        let Some(job) = queue.lock().expect("queue lock poisoned").pop_front()
                        else {
                            return;
                        };
                        let compiled = compile_source(&job.source).and_then(|artifacts| {
                            serde_json::to_value(artifacts)
                                .map_err(|error| format!("failed to serialize artifacts: {error}"))
                        });
                        for route in job.routes {
                            match write_compiled_route(route, &compiled) {
                                Ok(route_failed) => {
                                    if route_failed {
                                        failed.fetch_add(1, Ordering::Relaxed);
                                    }
                                    let count = written.fetch_add(1, Ordering::Relaxed) + 1;
                                    if count % 500 == 0 || count == pending_routes {
                                        let elapsed = work_started.elapsed().as_secs_f64();
                                        let rate = count as f64 / elapsed.max(f64::EPSILON);
                                        println!(
                                            "processed {count}/{pending_routes} pending card routes \
                                             ({rate:.1} routes/s, {elapsed:.1}s elapsed)"
                                        );
                                    }
                                }
                                Err(error) => {
                                    *fatal_error.lock().expect("error lock poisoned") = Some(error);
                                    return;
                                }
                            }
                        }
                    }
                })
                .map_err(|error| format!("failed to start artifact worker: {error}"))?,
        );
    }
    for worker in workers {
        worker
            .join()
            .map_err(|_| "artifact baker worker panicked".to_string())?;
    }
    if let Some(error) = fatal_error.lock().expect("error lock poisoned").take() {
        return Err(error);
    }
    Ok(BakeSummary {
        written: written.load(Ordering::Relaxed),
        retained,
        unique_sources: source_keys.len(),
        failed: failed.load(Ordering::Relaxed),
        workers: worker_count,
        scan_time,
        work_time: work_started.elapsed(),
        total_time: started.elapsed(),
    })
}

fn main() -> Result<(), String> {
    let mut args = env::args_os().skip(1);
    let mut cards_dir = None;
    let mut workers = None;
    while let Some(argument) = args.next() {
        if argument == "--cards-dir" {
            cards_dir = args.next().map(PathBuf::from);
        } else if argument == "--workers" {
            let value = args
                .next()
                .ok_or_else(|| "--workers requires a value".to_string())?;
            workers = Some(
                value
                    .to_string_lossy()
                    .parse::<usize>()
                    .map_err(|_| "--workers must be a positive integer".to_string())?,
            );
        } else {
            return Err(format!("unknown argument: {}", argument.to_string_lossy()));
        }
    }
    let cards_dir = cards_dir.ok_or_else(|| "--cards-dir is required".to_string())?;
    let summary = std::thread::Builder::new()
        .name("artifact-baker".to_string())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || bake(&cards_dir, workers))
        .map_err(|error| format!("failed to start artifact baker: {error}"))?
        .join()
        .map_err(|_| "artifact baker worker panicked".to_string())??;
    println!(
        "baked {} source groups with {} workers; wrote {} routes, retained {} current routes, \
         and recorded {} failed routes in {:.3}s (scan {:.3}s, compile/write {:.3}s)",
        summary.unique_sources,
        summary.workers,
        summary.written,
        summary.retained,
        summary.failed,
        summary.total_time.as_secs_f64(),
        summary.scan_time.as_secs_f64(),
        summary.work_time.as_secs_f64(),
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_strict_snapshot_skips_parser_work() {
        let source = CardSourceFile {
            canonical_name: "Unsupported Example".to_string(),
            group: CardSourceGroup::Single {
                name: "Unsupported Example".to_string(),
                block: "Type: Sorcery\nPerform an unsupported action.".to_string(),
                score: None,
            },
        };
        let error = compile_source(&source).expect_err("unvalidated source should be skipped");
        assert!(error.contains("no validated strict compilation snapshot"));
    }

    #[test]
    fn linked_artifacts_keep_local_face_relationships() {
        let source = CardSourceFile {
            canonical_name: "Front".to_string(),
            group: CardSourceGroup::Linked {
                layout: "transform_like".to_string(),
                faces: vec![
                    CardFaceSource {
                        name: "Front".to_string(),
                        block: "Type: Creature — Human\nPower/Toughness: 1/1".to_string(),
                        score: Some(1.0),
                    },
                    CardFaceSource {
                        name: "Back".to_string(),
                        block: "Type: Creature — Wolf\nPower/Toughness: 2/2".to_string(),
                        score: Some(1.0),
                    },
                ],
            },
        };
        let artifacts = compile_source(&source).expect("linked source should compile");
        assert_eq!(artifacts.len(), 2);
        assert_eq!(artifacts[0].card.other_face, Some(ArtifactCardId(2)));
        assert_eq!(artifacts[1].card.other_face, Some(ArtifactCardId(1)));
        assert!(artifacts.iter().all(|artifact| artifact.validate().is_ok()));
    }
}
