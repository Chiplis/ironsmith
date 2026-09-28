//! An indexed, independently compressed card catalogue.
//!
//! Route payloads and the source index are preserved byte for byte. Only the
//! small directory is decoded when opening the bundle; at most four content
//! chunks are retained after lazy reads. No compiler or engine dependency is
//! needed to read the catalogue in a browser's WASM worker.

use std::collections::{BTreeMap, VecDeque};
use std::fmt;
use std::io::Read;

use serde::{Deserialize, Serialize};

#[cfg(feature = "build")]
pub mod builder;

pub(crate) const MAGIC: &[u8; 8] = b"ISCCAT01";
pub(crate) const HEADER_LEN: usize = 16;
pub(crate) const MAX_DIRECTORY_BYTES: usize = 16 * 1024 * 1024;
pub(crate) const MAX_CHUNK_BYTES: usize = 32 * 1024 * 1024;
pub(crate) const MAX_ROUTES: usize = 100_000;
pub(crate) const MAX_CHUNKS: usize = 65_536;
const CACHE_CHUNKS: usize = 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogError(pub(crate) String);

impl fmt::Display for CatalogError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "card catalog: {}", self.0)
    }
}

impl std::error::Error for CatalogError {}

pub(crate) fn error(message: impl Into<String>) -> CatalogError {
    CatalogError(message.into())
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub(crate) struct Location {
    pub c: usize,
    pub o: usize,
    pub n: usize,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct Chunk {
    pub o: usize,
    pub n: usize,
    pub raw: usize,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct Directory {
    pub index: Location,
    pub chunks: Vec<Chunk>,
    pub routes: BTreeMap<String, Location>,
}

/// Reader over borrowed bundle bytes, with a bounded lazy decoded-chunk cache.
pub struct Catalog<'a> {
    payload: &'a [u8],
    directory: Directory,
    cache: VecDeque<(usize, String)>,
}

impl<'a> Catalog<'a> {
    /// Validate the bundle header, directory and all byte ranges without
    /// decompressing any card payloads or the source index.
    pub fn from_bytes(bytes: &'a [u8]) -> Result<Self, CatalogError> {
        if bytes.len() < HEADER_LEN || bytes.get(..8) != Some(MAGIC.as_slice()) {
            return Err(error("unsupported or truncated header"));
        }
        let directory_raw = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
        let directory_len = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
        if directory_raw == 0 || directory_raw > MAX_DIRECTORY_BYTES {
            return Err(error("directory exceeds decoded size limit"));
        }
        let payload_start = HEADER_LEN
            .checked_add(directory_len)
            .filter(|end| *end <= bytes.len())
            .ok_or_else(|| error("directory extends past bundle"))?;
        let raw = decompress_exact(&bytes[HEADER_LEN..payload_start], directory_raw)?;
        let directory: Directory = serde_json::from_slice(&raw)
            .map_err(|err| error(format!("invalid directory: {err}")))?;
        let payload = &bytes[payload_start..];
        validate_directory(&directory, payload.len())?;
        Ok(Self {
            payload,
            directory,
            cache: VecDeque::new(),
        })
    }

    pub fn route_count(&self) -> usize {
        self.directory.routes.len()
    }

    /// The complete, unmodified source `index.json`, including linked groups.
    pub fn index_json(&mut self) -> Result<&str, CatalogError> {
        self.read_location(self.directory.index)
    }

    /// Return a source route document, including its compiled definitions,
    /// source text and metadata. Accepts an exact basename, optionally with a
    /// `.json` suffix; paths, URL decoding and fuzzy names are not inferred.
    pub fn route_json(&mut self, route: &str) -> Result<Option<&str>, CatalogError> {
        let location = self.directory.routes.get(route).or_else(|| {
            route
                .strip_suffix(".json")
                .and_then(|name| self.directory.routes.get(name))
        });
        let Some(location) = location.copied() else {
            return Ok(None);
        };
        self.read_location(location).map(Some)
    }

    fn read_location(&mut self, location: Location) -> Result<&str, CatalogError> {
        if let Some(position) = self.cache.iter().position(|(id, _)| *id == location.c) {
            let entry = self.cache.remove(position).unwrap();
            self.cache.push_front(entry);
        } else {
            let chunk = &self.directory.chunks[location.c];
            let raw = decompress_exact(&self.payload[chunk.o..chunk.o + chunk.n], chunk.raw)?;
            let text = String::from_utf8(raw).map_err(|_| error("chunk is not UTF-8"))?;
            self.cache.push_front((location.c, text));
            self.cache.truncate(CACHE_CHUNKS);
        }
        self.cache[0]
            .1
            .get(location.o..location.o + location.n)
            .ok_or_else(|| error("route does not lie on UTF-8 boundaries"))
    }
}

fn validate_directory(directory: &Directory, payload_len: usize) -> Result<(), CatalogError> {
    if directory.routes.len() > MAX_ROUTES
        || directory.chunks.is_empty()
        || directory.chunks.len() > MAX_CHUNKS
    {
        return Err(error("invalid route or chunk count"));
    }
    let mut end = 0usize;
    for chunk in &directory.chunks {
        if chunk.o != end || chunk.n == 0 || chunk.raw == 0 || chunk.raw > MAX_CHUNK_BYTES {
            return Err(error("invalid chunk bounds or decoded size"));
        }
        end = chunk
            .o
            .checked_add(chunk.n)
            .filter(|end| *end <= payload_len)
            .ok_or_else(|| error("chunk extends past bundle"))?;
    }
    if end != payload_len {
        return Err(error("unreferenced trailing bundle bytes"));
    }
    for location in std::iter::once(&directory.index).chain(directory.routes.values()) {
        let chunk = directory
            .chunks
            .get(location.c)
            .ok_or_else(|| error("route references missing chunk"))?;
        if location.n == 0
            || location
                .o
                .checked_add(location.n)
                .is_none_or(|end| end > chunk.raw)
        {
            return Err(error("route extends past decoded chunk"));
        }
    }
    if directory.routes.keys().any(|name| !valid_route(name)) {
        return Err(error("invalid route name"));
    }
    Ok(())
}

pub(crate) fn valid_route(name: &str) -> bool {
    !name.is_empty()
        && !name.contains(['/', '\\', '\0'])
        && name != "index"
        && name != "."
        && name != ".."
}

pub(crate) fn decompress_exact(data: &[u8], expected: usize) -> Result<Vec<u8>, CatalogError> {
    let mut decoded = Vec::with_capacity(expected.min(4 * 1024 * 1024));
    // Reading one byte past the declared length detects forged decoded lengths
    // while preventing unbounded inflation of a corrupt stream.
    brotli_decompressor::Decompressor::new(data, 4096)
        .take(expected as u64 + 1)
        .read_to_end(&mut decoded)
        .map_err(|err| error(format!("invalid compressed chunk: {err}")))?;
    if decoded.len() != expected {
        return Err(error("decoded length does not match directory"));
    }
    Ok(decoded)
}

#[cfg(all(test, feature = "build"))]
mod tests {
    use super::*;
    use crate::builder::{
        BundleBuilder, MAX_QUALITY, build_directory, build_directory_with_quality, compress,
    };
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    const INDEX: &[u8] =
        br#"{ "cards": ["front", "alias", "failed"], "linked_groups": [["front","back"]] }
"#;
    const CARD: &[u8] = br#"{"name":"Fire // Ice","oracle_text":"Source text.","scryfall":{"id":"id-1"},"compiled":{"effects":[1,2]},"faces":["front","back"]}
"#;
    const FAILED: &[u8] = br#"{"name":"Unsupported","compile_error":"unsupported effect","oracle_text":"Keep me","metadata":{"set":"TEST"}}"#;

    fn fixture() -> Vec<u8> {
        let mut builder = BundleBuilder::with_chunk_target(INDEX, 64).unwrap();
        builder.add_route("front", CARD).unwrap();
        builder.add_route("failed", FAILED).unwrap();
        builder.add_route("alias", CARD).unwrap();
        assert_eq!(builder.unique_route_bytes(), CARD.len() + FAILED.len());
        builder.finish().unwrap()
    }

    #[test]
    fn round_trip_preserves_exact_index_and_full_source_even_without_compiled_definition() {
        let bytes = fixture();
        let mut catalog = Catalog::from_bytes(&bytes).unwrap();
        assert!(catalog.cache.is_empty(), "opening must not inflate content");
        assert_eq!(catalog.route_count(), 3);
        assert_eq!(catalog.route_json("absent").unwrap(), None);
        assert!(catalog.cache.is_empty(), "a miss must not inflate content");
        assert_eq!(catalog.index_json().unwrap().as_bytes(), INDEX);
        assert_eq!(
            catalog.route_json("front").unwrap().unwrap().as_bytes(),
            CARD
        );
        assert_eq!(
            catalog
                .route_json("alias.json")
                .unwrap()
                .unwrap()
                .as_bytes(),
            CARD
        );
        assert_eq!(
            catalog.route_json("failed").unwrap().unwrap().as_bytes(),
            FAILED
        );
        assert_eq!(
            catalog.directory.routes["front"].c,
            catalog.directory.routes["alias"].c
        );
        assert_eq!(
            catalog.directory.routes["front"].o,
            catalog.directory.routes["alias"].o
        );
    }

    #[test]
    fn decoded_cache_is_lazy_bounded_and_evicted_routes_remain_exact() {
        let mut builder = BundleBuilder::with_chunk_target(b"{}", 1).unwrap();
        for i in 0..9 {
            builder
                .add_route(&format!("card-{i}"), format!("{{\"id\":{i}}}").as_bytes())
                .unwrap();
        }
        let bytes = builder.finish().unwrap();
        let mut catalog = Catalog::from_bytes(&bytes).unwrap();
        for i in 0..9 {
            assert_eq!(
                catalog.route_json(&format!("card-{i}")).unwrap().unwrap(),
                format!("{{\"id\":{i}}}")
            );
            assert!(catalog.cache.len() <= CACHE_CHUNKS);
        }
        assert!(!catalog.cache.iter().any(|(id, _)| *id == 1));
        assert_eq!(catalog.route_json("card-0").unwrap(), Some("{\"id\":0}"));
        assert_eq!(catalog.cache.len(), CACHE_CHUNKS);
        assert_eq!(catalog.index_json().unwrap(), "{}");
    }

    fn rewrite_directory(bytes: &[u8], mutate: impl FnOnce(&mut Directory)) -> Vec<u8> {
        let raw = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
        let size = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
        let mut directory: Directory =
            serde_json::from_slice(&decompress_exact(&bytes[16..16 + size], raw).unwrap()).unwrap();
        mutate(&mut directory);
        let raw = serde_json::to_vec(&directory).unwrap();
        let packed = compress(&raw).unwrap();
        let mut result = Vec::from(MAGIC.as_slice());
        result.extend_from_slice(&(raw.len() as u32).to_le_bytes());
        result.extend_from_slice(&(packed.len() as u32).to_le_bytes());
        result.extend_from_slice(&packed);
        result.extend_from_slice(&bytes[16 + size..]);
        result
    }

    #[test]
    fn rejects_truncated_headers_overflowing_offsets_and_out_of_bounds_records() {
        let bytes = fixture();
        for length in 0..16 {
            assert!(Catalog::from_bytes(&bytes[..length]).is_err());
        }
        for invalid in [
            rewrite_directory(&bytes, |dir| dir.chunks[0].o = usize::MAX),
            rewrite_directory(&bytes, |dir| dir.chunks[0].n = usize::MAX),
            rewrite_directory(&bytes, |dir| dir.index.o = usize::MAX),
            rewrite_directory(&bytes, |dir| dir.index.c = dir.chunks.len()),
            rewrite_directory(&bytes, |dir| dir.chunks[0].raw = MAX_CHUNK_BYTES + 1),
        ] {
            assert!(Catalog::from_bytes(&invalid).is_err());
        }
        let mut huge_header = bytes.clone();
        huge_header[8..12].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(Catalog::from_bytes(&huge_header).is_err());
        assert!(Catalog::from_bytes(&bytes[..bytes.len() - 1]).is_err());
    }

    #[test]
    fn declared_decoded_size_is_enforced_when_content_is_read() {
        let bytes = rewrite_directory(&fixture(), |dir| dir.chunks[0].raw += 1);
        let mut catalog = Catalog::from_bytes(&bytes).unwrap();
        assert!(
            catalog
                .index_json()
                .unwrap_err()
                .to_string()
                .contains("decoded length")
        );
        let packed = compress(&vec![b'x'; 100_000]).unwrap();
        assert!(decompress_exact(&packed, 5).is_err());
    }

    #[test]
    fn rejects_duplicate_routes_and_preserves_distinct_payloads() {
        let mut builder = BundleBuilder::new(b"{}").unwrap();
        builder
            .add_route("name", b"{\"text\":\"original\"}")
            .unwrap();
        assert!(builder.add_route("name", b"{}").is_err());
        assert!(builder.add_route("../bad", b"{}").is_err());
        builder
            .add_route("alias", b"{\"text\":\"different\"}")
            .unwrap();
        let bytes = builder.finish().unwrap();
        let mut catalog = Catalog::from_bytes(&bytes).unwrap();
        assert_eq!(
            catalog.route_json("alias").unwrap(),
            Some("{\"text\":\"different\"}")
        );
    }

    #[test]
    fn filesystem_cache_checks_content_and_output_and_build_is_deterministic() {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "ironsmith-card-catalog-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let cards = root.join("cards");
        fs::create_dir_all(&cards).unwrap();
        fs::write(cards.join("index.json"), INDEX).unwrap();
        // Creation order is deliberately unrelated to lexical route order.
        fs::write(cards.join("front.json"), CARD).unwrap();
        fs::write(cards.join("failed.json"), FAILED).unwrap();
        fs::write(cards.join("alias.json"), CARD).unwrap();
        let output = root.join("catalog.bin");
        let first = build_directory(&cards, &output).unwrap();
        let original = fs::read(&output).unwrap();
        assert!(!first.cached);
        assert_eq!(first.routes, 3);
        assert!(first.unique_bytes < first.source_bytes);
        assert!(build_directory(&cards, &output).unwrap().cached);
        fs::write(cards.join("alias.json"), CARD).unwrap();
        assert!(
            build_directory(&cards, &output).unwrap().cached,
            "mtime is not content"
        );
        fs::write(&output, b"broken").unwrap();
        assert!(!build_directory(&cards, &output).unwrap().cached);
        assert_eq!(
            fs::read(&output).unwrap(),
            original,
            "rebuild must be deterministic"
        );
        fs::write(cards.join("failed.json"), b"{\"name\":\"changed\"}").unwrap();
        assert!(!build_directory(&cards, &output).unwrap().cached);
        assert_ne!(fs::read(&output).unwrap(), original);
        let default_quality = fs::read(&output).unwrap();
        let dense = build_directory_with_quality(&cards, &output, MAX_QUALITY).unwrap();
        assert!(!dense.cached, "the brotli quality is part of the cache key");
        let dense_bytes = fs::read(&output).unwrap();
        let mut dense_catalog = Catalog::from_bytes(&dense_bytes).unwrap();
        let mut default_catalog = Catalog::from_bytes(&default_quality).unwrap();
        for route in ["front", "alias", "failed"] {
            assert_eq!(
                dense_catalog.route_json(route).unwrap().map(str::to_owned),
                default_catalog
                    .route_json(route)
                    .unwrap()
                    .map(str::to_owned),
                "quality changes the encoding, never the documents"
            );
        }
        assert!(
            build_directory_with_quality(&cards, &output, MAX_QUALITY)
                .unwrap()
                .cached
        );
        assert!(!build_directory(&cards, &output).unwrap().cached);
        assert!(build_directory_with_quality(&cards, &output, MAX_QUALITY + 1).is_err());
        fs::remove_dir_all(root).unwrap();
    }
}
