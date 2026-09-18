Magic: The Gathering engine supporting automatic oracle text parsing and custom card compilation via natural language and 4-way multiplayer backed by Zero Knowledge proofs.

~26k cards supported, with more to come soon!

https://chiplis.com/ironsmith

## Competitive deck catalog

The deck browser is backed by a versioned, repository-local catalog. The browser
loads `catalog/<format>/index.json` and its search index first, then fetches an
individual `details/<deck-id>.json` only when a player selects or copies a deck.
This keeps the initial page fast and avoids making requests to MTGTop8 from a
player's browser. Catalog entries include the event, placement, card lists,
mana metadata, source URL, and collection tags such as `last-20-events`,
`last-major-events`, and `mono-color`.

The bounded synchronizer lives in `tools/deck-catalog/`. It prioritizes the
latest event collections and a small mono-colour sample, then merges new deck
IDs into the existing history without deleting older records. To run a local
dry run without writing files:

```sh
node tools/deck-catalog/sync.mjs --format modern --page 0 --events 5 --limit 24 \
  --collection-limit 12 --recent-events 20 --major-events 5 --dry-run
```

`.github/workflows/update-deck-catalog.yml` runs this bounded sync every three
days and can also be started manually with format, page, and size limits. It
validates the catalog tests before fetching, commits only changes under
`catalog/`, and relies on GitHub Actions' free quota. The scheduled job runs
from the repository's default branch; after this feature branch is merged into
`Chiplis/ironsmith`'s `main`, updates will arrive there automatically. Manual
runs are useful for testing a smaller slice before enabling a larger historical
backfill.

After changing catalog files, the Vite prebuild hook copies them to the ignored
`web/ui/public/catalog/` directory. Do not commit that generated copy.

## Run it locally

Requirements: a Rust toolchain installed through `rustup`, Python 3, Node, and `pnpm`.

```sh
./rebuild-wasm.sh
cd web/ui && pnpm install && pnpm dev
```

The first `./rebuild-wasm.sh` downloads the Scryfall card list, builds the card registry
at `reports/engine-status.sqlite3`, compiles a snapshot of every supported card, and writes
the per-card assets the browser loads from `web/ui/public/cards/`, so expect it to run for a
while. It also installs the `wasm32-unknown-unknown` target and the pinned `wasm-bindgen`
CLI if they are missing. Later runs only pick up cards the registry does not have yet, and
`./rebuild-wasm.sh --release` additionally runs the shipped optimizer over the WASM.

## Browser / npm package

Build and verify the lean `ironsmith-wasm` npm artifact with:

```sh
node scripts/build-npm-package.mjs
node scripts/verify-npm-package.mjs
```

Consumer usage and card-loading details are documented in [the package README](npm/ironsmith-wasm/README.md). Release setup is documented in [the publishing guide](npm/ironsmith-wasm/PUBLISHING.md).
