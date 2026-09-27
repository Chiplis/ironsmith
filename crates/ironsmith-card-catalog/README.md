# Embedded card catalogue

This small crate stores the existing generated card JSON files inside an engine
WASM asset without parsing the whole card collection at startup. The full source
`index.json` and each route document are preserved byte for byte, including
compiled definitions, failed-compilation source, Scryfall metadata and aliases.
Only byte-identical documents are deduplicated; hash candidates are compared
against their original bytes before a location is shared.

The native builder writes a deterministic `ISCCAT01` bundle:

- 16-byte header: magic, decoded directory length, compressed directory length.
- Brotli-compressed JSON directory with route locations and chunk byte ranges.
- Independent Brotli chunks, targeting 4 MiB before compression. Documents are
  never split between chunks. The index occupies its own chunk.

The reader decodes only the directory initially. Requests lazily decode one
chunk; a four-entry least-recently-used cache bounds retained decoded content.
The catalogue can therefore contain hundreds of megabytes of raw JSON without
requiring that full allocation or registering every card in the game engine.
All offsets and declared decoded lengths are checked before returning content.
A missing route is `Ok(None)`; malformed bundle data produces an explicit error.

```rust,ignore
let mut catalog = ironsmith_card_catalog::Catalog::from_bytes(BUNDLE)?;
let index_json = catalog.index_json()?;
let lightning_bolt = catalog.route_json("lightning-bolt")?;
```

The encoder and filesystem tools are available only with the `build` feature;
the browser uses the decoder alone:

```sh
cargo run --release -p ironsmith-card-catalog --features build \
  --bin build_card_catalog -- \
  --cards-dir web/ui/public/cards --output target/embedded-card-catalog.bin
cargo test -p ironsmith-card-catalog --features build
```

Build caching hashes sorted filenames, exact source contents, and builder source.
An output digest must also match before a cache hit is accepted, so corrupt
outputs are rebuilt. Both the bundle and its adjacent `.manifest.json` file are
replaced atomically. Concurrent changes to the card directory abort construction
rather than publishing a mixture of source versions. Brotli uses quality 9 with
a 4 MiB window as a compromise between size and repeatable build time.
