//! Native-only bundle construction. The browser dependency uses only the
//! decoder; the Brotli encoder and filesystem builder are feature-gated.

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    Catalog, CatalogError, Chunk, Directory, Location, MAGIC, MAX_CHUNK_BYTES, MAX_DIRECTORY_BYTES,
    MAX_ROUTES, error, valid_route,
};

pub const DEFAULT_CHUNK_BYTES: usize = 4 * 1024 * 1024;
/// Fast enough for iterative rebuilds; shipped builds pass `MAX_QUALITY`.
pub const DEFAULT_QUALITY: i32 = 9;
/// Brotli's densest setting. Decoding cost does not depend on the quality.
pub const MAX_QUALITY: i32 = 11;
const WINDOW: i32 = 22;

/// Incremental deterministic builder. Documents are kept verbatim and each
/// route must be supplied once. Byte-identical aliases share one location;
/// hashes are only a candidate lookup and are followed by exact byte equality.
///
/// Chunks stay raw until [`BundleBuilder::finish`], which compresses them
/// independently on every core; the bytes do not depend on the thread count.
pub struct BundleBuilder {
    directory: Directory,
    chunks: Vec<Vec<u8>>,
    pending: Vec<u8>,
    chunk_target: usize,
    quality: i32,
    seen: HashMap<[u8; 32], Vec<Location>>,
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
        Self::with_options(index_json, chunk_target, DEFAULT_QUALITY)
    }

    pub fn with_options(
        index_json: &[u8],
        chunk_target: usize,
        quality: i32,
    ) -> Result<Self, CatalogError> {
        if chunk_target == 0 || chunk_target > MAX_CHUNK_BYTES {
            return Err(error("invalid chunk target"));
        }
        check_quality(quality)?;
        check_document(index_json)?;
        Ok(Self {
            directory: Directory {
                index: Location {
                    c: 0,
                    o: 0,
                    n: index_json.len(),
                },
                chunks: Vec::new(),
                routes: BTreeMap::new(),
            },
            chunks: vec![index_json.to_vec()],
            pending: Vec::new(),
            chunk_target,
            quality,
            seen: HashMap::new(),
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
            c: self.chunks.len(),
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
        let packed = compress_all(&self.chunks, self.quality)?;
        let mut payload = Vec::with_capacity(packed.iter().map(Vec::len).sum());
        for (raw, compressed) in self.chunks.iter().zip(&packed) {
            self.directory.chunks.push(Chunk {
                o: payload.len(),
                n: compressed.len(),
                raw: raw.len(),
            });
            payload.extend_from_slice(compressed);
        }
        let directory =
            serde_json::to_vec(&self.directory).map_err(|err| error(err.to_string()))?;
        if directory.len() > MAX_DIRECTORY_BYTES {
            return Err(error("directory too large"));
        }
        let compressed = compress_with_quality(&directory, self.quality)?;
        let mut result = Vec::with_capacity(16 + compressed.len() + payload.len());
        result.extend_from_slice(MAGIC);
        result.extend_from_slice(&(directory.len() as u32).to_le_bytes());
        result.extend_from_slice(&(compressed.len() as u32).to_le_bytes());
        result.extend_from_slice(&compressed);
        result.extend_from_slice(&payload);
        Catalog::from_bytes(&result)?;
        Ok(result)
    }

    fn flush(&mut self) -> Result<(), CatalogError> {
        if !self.pending.is_empty() {
            self.chunks.push(std::mem::take(&mut self.pending));
        }
        Ok(())
    }

    fn location_bytes(&self, location: Location) -> Result<&[u8], CatalogError> {
        let chunk = if location.c == self.chunks.len() {
            &self.pending
        } else {
            &self.chunks[location.c]
        };
        Ok(&chunk[location.o..location.o + location.n])
    }
}

/// Compress every chunk at `quality`, spreading the chunks over the available
/// cores. Each chunk is an independent Brotli stream, so the result matches a
/// serial pass byte for byte.
fn compress_all(chunks: &[Vec<u8>], quality: i32) -> Result<Vec<Vec<u8>>, CatalogError> {
    let workers = std::thread::available_parallelism()
        .map(|count| count.get())
        .unwrap_or(1)
        .min(chunks.len().max(1));
    let next = std::sync::atomic::AtomicUsize::new(0);
    let mut results: Vec<Option<Result<Vec<u8>, CatalogError>>> =
        (0..chunks.len()).map(|_| None).collect();
    let slots = std::sync::Mutex::new(&mut results);
    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| {
                loop {
                    let index = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    if index >= chunks.len() {
                        break;
                    }
                    let packed = compress_with_quality(&chunks[index], quality);
                    slots.lock().unwrap()[index] = Some(packed);
                }
            });
        }
    });
    results
        .into_iter()
        .map(|slot| slot.expect("every chunk is compressed"))
        .collect()
}

fn check_document(bytes: &[u8]) -> Result<(), CatalogError> {
    if bytes.is_empty() || bytes.len() > MAX_CHUNK_BYTES {
        return Err(error("empty or oversized JSON document"));
    }
    std::str::from_utf8(bytes).map_err(|_| error("source document is not UTF-8"))?;
    Ok(())
}

fn check_quality(quality: i32) -> Result<(), CatalogError> {
    if (0..=MAX_QUALITY).contains(&quality) {
        Ok(())
    } else {
        Err(error(format!(
            "brotli quality must be 0 through {MAX_QUALITY}"
        )))
    }
}

#[cfg(test)]
pub(crate) fn compress(bytes: &[u8]) -> Result<Vec<u8>, CatalogError> {
    compress_with_quality(bytes, DEFAULT_QUALITY)
}

fn compress_with_quality(bytes: &[u8], quality: i32) -> Result<Vec<u8>, CatalogError> {
    let params = brotli::enc::BrotliEncoderParams {
        quality,
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
    build_directory_with_quality(cards_dir, output, DEFAULT_QUALITY)
}

/// Like [`build_directory`] at an explicit Brotli quality. The quality is part
/// of the cache key, so switching it always rebuilds the bundle.
pub fn build_directory_with_quality(
    cards_dir: &Path,
    output: &Path,
    quality: i32,
) -> Result<BuildStats, CatalogError> {
    check_quality(quality)?;
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
    let source_digest = source_digest(&files, quality)?;
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
    let mut builder = BundleBuilder::with_options(&index, DEFAULT_CHUNK_BYTES, quality)?;
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
    if current_files != files || source_digest != self::source_digest(&current_files, quality)? {
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

fn source_digest(files: &[PathBuf], quality: i32) -> Result<String, CatalogError> {
    let mut digest = Sha256::new();
    digest.update(quality.to_le_bytes());
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
