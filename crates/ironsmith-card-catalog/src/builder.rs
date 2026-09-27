//! Native-only bundle construction. The browser dependency uses only the
//! decoder; the Brotli encoder and filesystem builder are feature-gated.

use std::collections::{BTreeMap, HashMap, VecDeque};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    Catalog, CatalogError, Chunk, Directory, Location, MAGIC, MAX_CHUNK_BYTES, MAX_DIRECTORY_BYTES,
    MAX_ROUTES, decompress_exact, error, valid_route,
};

pub const DEFAULT_CHUNK_BYTES: usize = 4 * 1024 * 1024;
const QUALITY: i32 = 9;
const WINDOW: i32 = 22;

/// Incremental deterministic builder. Documents are kept verbatim and each
/// route must be supplied once. Byte-identical aliases share one location;
/// hashes are only a candidate lookup and are followed by exact byte equality.
pub struct BundleBuilder {
    directory: Directory,
    payload: Vec<u8>,
    pending: Vec<u8>,
    chunk_target: usize,
    seen: HashMap<[u8; 32], Vec<Location>>,
    compare_cache: VecDeque<(usize, Vec<u8>)>,
    raw_route_bytes: usize,
    unique_route_bytes: usize,
}

impl BundleBuilder {
    pub fn new(index_json: &[u8]) -> Result<Self, CatalogError> {
        Self::with_chunk_target(index_json, DEFAULT_CHUNK_BYTES)
    }

    /// Small targets are useful for testing cache eviction without a large
    /// fixture. Large single documents remain whole in a dedicated chunk.
    pub fn with_chunk_target(index_json: &[u8], chunk_target: usize) -> Result<Self, CatalogError> {
        if chunk_target == 0 || chunk_target > MAX_CHUNK_BYTES {
            return Err(error("invalid chunk target"));
        }
        check_document(index_json)?;
        let packed = compress(index_json)?;
        Ok(Self {
            directory: Directory {
                index: Location {
                    c: 0,
                    o: 0,
                    n: index_json.len(),
                },
                chunks: vec![Chunk {
                    o: 0,
                    n: packed.len(),
                    raw: index_json.len(),
                }],
                routes: BTreeMap::new(),
            },
            payload: packed,
            pending: Vec::new(),
            chunk_target,
            seen: HashMap::new(),
            compare_cache: VecDeque::new(),
            raw_route_bytes: 0,
            unique_route_bytes: 0,
        })
    }

    pub fn add_route(&mut self, route: &str, document: &[u8]) -> Result<(), CatalogError> {
        if !valid_route(route)
            || self.directory.routes.contains_key(route)
            || self.directory.routes.len() >= MAX_ROUTES
        {
            return Err(error(format!("invalid or duplicate route: {route}")));
        }
        check_document(document)?;
        let digest: [u8; 32] = Sha256::digest(document).into();
        if let Some(candidates) = self.seen.get(&digest).cloned() {
            for location in candidates {
                if self.location_bytes(location)? == document {
                    self.directory.routes.insert(route.into(), location);
                    self.raw_route_bytes += document.len();
                    return Ok(());
                }
            }
        }
        if !self.pending.is_empty() && self.pending.len() + document.len() > self.chunk_target {
            self.flush()?;
        }
        let location = Location {
            c: self.directory.chunks.len(),
            o: self.pending.len(),
            n: document.len(),
        };
        self.pending.extend_from_slice(document);
        self.directory.routes.insert(route.into(), location);
        self.seen.entry(digest).or_default().push(location);
        self.raw_route_bytes += document.len();
        self.unique_route_bytes += document.len();
        Ok(())
    }

    pub fn route_count(&self) -> usize {
        self.directory.routes.len()
    }
    pub fn raw_route_bytes(&self) -> usize {
        self.raw_route_bytes
    }
    pub fn unique_route_bytes(&self) -> usize {
        self.unique_route_bytes
    }

    pub fn finish(mut self) -> Result<Vec<u8>, CatalogError> {
        self.flush()?;
        let directory =
            serde_json::to_vec(&self.directory).map_err(|err| error(err.to_string()))?;
        if directory.len() > MAX_DIRECTORY_BYTES {
            return Err(error("directory too large"));
        }
        let compressed = compress(&directory)?;
        let mut result = Vec::with_capacity(16 + compressed.len() + self.payload.len());
        result.extend_from_slice(MAGIC);
        result.extend_from_slice(&(directory.len() as u32).to_le_bytes());
        result.extend_from_slice(&(compressed.len() as u32).to_le_bytes());
        result.extend_from_slice(&compressed);
        result.extend_from_slice(&self.payload);
        Catalog::from_bytes(&result)?;
        Ok(result)
    }

    fn flush(&mut self) -> Result<(), CatalogError> {
        if self.pending.is_empty() {
            return Ok(());
        }
        let compressed = compress(&self.pending)?;
        self.directory.chunks.push(Chunk {
            o: self.payload.len(),
            n: compressed.len(),
            raw: self.pending.len(),
        });
        self.payload.extend_from_slice(&compressed);
        self.pending.clear();
        Ok(())
    }

    fn location_bytes(&mut self, location: Location) -> Result<&[u8], CatalogError> {
        if location.c == self.directory.chunks.len() {
            return Ok(&self.pending[location.o..location.o + location.n]);
        }
        if let Some(position) = self
            .compare_cache
            .iter()
            .position(|(id, _)| *id == location.c)
        {
            let entry = self.compare_cache.remove(position).unwrap();
            self.compare_cache.push_front(entry);
        } else {
            let chunk = &self.directory.chunks[location.c];
            let raw = decompress_exact(&self.payload[chunk.o..chunk.o + chunk.n], chunk.raw)?;
            self.compare_cache.push_front((location.c, raw));
            self.compare_cache.truncate(4);
        }
        Ok(&self.compare_cache[0].1[location.o..location.o + location.n])
    }
}

fn check_document(bytes: &[u8]) -> Result<(), CatalogError> {
    if bytes.is_empty() || bytes.len() > MAX_CHUNK_BYTES {
        return Err(error("empty or oversized JSON document"));
    }
    std::str::from_utf8(bytes).map_err(|_| error("source document is not UTF-8"))?;
    Ok(())
}

pub(crate) fn compress(bytes: &[u8]) -> Result<Vec<u8>, CatalogError> {
    let params = brotli::enc::BrotliEncoderParams {
        quality: QUALITY,
        lgwin: WINDOW,
        ..Default::default()
    };
    let mut output = Vec::new();
    brotli::BrotliCompress(&mut std::io::Cursor::new(bytes), &mut output, &params)
        .map_err(|err| error(format!("compression failed: {err}")))?;
    Ok(output)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildStats {
    pub routes: usize,
    pub source_bytes: usize,
    pub unique_bytes: usize,
    pub bundle_bytes: usize,
    pub cached: bool,
}

#[derive(Serialize, Deserialize)]
struct Manifest {
    source_digest: String,
    bundle_digest: String,
    stats: BuildStats,
}

/// Build every JSON route in a directory. Both the exact input contents and
/// the existing output are hashed before accepting a cached build. File mtimes
/// do not affect output bytes or cache validity.
pub fn build_directory(cards_dir: &Path, output: &Path) -> Result<BuildStats, CatalogError> {
    let mut files: Vec<PathBuf> = fs::read_dir(cards_dir)
        .map_err(|err| error(format!("read {}: {err}", cards_dir.display())))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<_, _>>()
        .map_err(|err| error(err.to_string()))?;
    files.retain(|path| path.extension().is_some_and(|ext| ext == "json") && path.is_file());
    files.sort_by(|left, right| left.file_name().cmp(&right.file_name()));
    if !files
        .iter()
        .any(|path| path.file_name().is_some_and(|name| name == "index.json"))
    {
        return Err(error("cards directory is missing index.json"));
    }
    let source_digest = source_digest(&files)?;
    let manifest_path = PathBuf::from(format!("{}.manifest.json", output.display()));
    if let Ok(raw) = fs::read(&manifest_path)
        && let Ok(manifest) = serde_json::from_slice::<Manifest>(&raw)
        && manifest.source_digest == source_digest
        && let Ok(bytes) = fs::read(output)
        && hex_digest(&bytes) == manifest.bundle_digest
        && Catalog::from_bytes(&bytes).is_ok()
    {
        return Ok(BuildStats {
            cached: true,
            ..manifest.stats
        });
    }
    let index = fs::read(cards_dir.join("index.json")).map_err(|err| error(err.to_string()))?;
    let mut builder = BundleBuilder::new(&index)?;
    for path in files
        .iter()
        .filter(|path| path.file_name().is_none_or(|name| name != "index.json"))
    {
        let route = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .ok_or_else(|| error("non-UTF8 route filename"))?;
        let bytes =
            fs::read(path).map_err(|err| error(format!("read {}: {err}", path.display())))?;
        builder.add_route(route, &bytes)?;
        if builder.route_count().is_multiple_of(5000) {
            eprintln!("packed {} card routes", builder.route_count());
        }
    }
    let routes = builder.route_count();
    let source_bytes = builder.raw_route_bytes() + index.len();
    let unique_bytes = builder.unique_route_bytes() + index.len();
    let bytes = builder.finish()?;
    // Reject inputs edited while the bundle was being built. This also keeps
    // the content manifest accurate during concurrent card-cache generation.
    let mut current_files: Vec<PathBuf> = fs::read_dir(cards_dir)
        .map_err(|err| error(err.to_string()))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<_, _>>()
        .map_err(|err| error(err.to_string()))?;
    current_files
        .retain(|path| path.extension().is_some_and(|ext| ext == "json") && path.is_file());
    current_files.sort_by(|left, right| left.file_name().cmp(&right.file_name()));
    if current_files != files || source_digest != self::source_digest(&current_files)? {
        return Err(error(
            "card source files changed during catalogue construction; retry the build",
        ));
    }
    let stats = BuildStats {
        routes,
        source_bytes,
        unique_bytes,
        bundle_bytes: bytes.len(),
        cached: false,
    };
    let manifest = Manifest {
        source_digest,
        bundle_digest: hex_digest(&bytes),
        stats: stats.clone(),
    };
    atomic_write(output, &bytes)?;
    atomic_write(
        &manifest_path,
        &serde_json::to_vec_pretty(&manifest).map_err(|err| error(err.to_string()))?,
    )?;
    Ok(stats)
}

fn source_digest(files: &[PathBuf]) -> Result<String, CatalogError> {
    let mut digest = Sha256::new();
    digest.update(include_bytes!("builder.rs"));
    digest.update(include_bytes!("lib.rs"));
    digest.update(include_bytes!("../Cargo.toml"));
    let mut buffer = [0u8; 64 * 1024];
    for path in files {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| error("non-UTF8 source filename"))?;
        let mut file = fs::File::open(path).map_err(|err| error(err.to_string()))?;
        let length = file.metadata().map_err(|err| error(err.to_string()))?.len();
        digest.update((name.len() as u64).to_le_bytes());
        digest.update(name.as_bytes());
        digest.update(length.to_le_bytes());
        loop {
            let read = file
                .read(&mut buffer)
                .map_err(|err| error(err.to_string()))?;
            if read == 0 {
                break;
            }
            digest.update(&buffer[..read]);
        }
    }
    Ok(hex_bytes(&digest.finalize()))
}

fn hex_digest(bytes: &[u8]) -> String {
    hex_bytes(&Sha256::digest(bytes))
}

fn hex_bytes(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut result = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        result.push(HEX[(byte >> 4) as usize] as char);
        result.push(HEX[(byte & 15) as usize] as char);
    }
    result
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), CatalogError> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).map_err(|err| error(err.to_string()))?;
    }
    let temporary = PathBuf::from(format!("{}.tmp-{}", path.display(), std::process::id()));
    let result = (|| {
        let mut file = fs::File::create(&temporary).map_err(|err| error(err.to_string()))?;
        file.write_all(bytes)
            .map_err(|err| error(err.to_string()))?;
        file.sync_all().map_err(|err| error(err.to_string()))?;
        fs::rename(&temporary, path).map_err(|err| error(err.to_string()))
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}
