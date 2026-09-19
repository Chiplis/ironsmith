Magic: The Gathering engine supporting automatic oracle text parsing and custom card compilation via natural language and 4-way multiplayer backed by Zero Knowledge proofs.

~26k cards supported, with more to come soon!

https://chiplis.com/ironsmith

## Competitive deck catalog

The deck browser is backed by a local catalog under `catalog/`. The browser
loads `catalog/<format>/index.json` and its search index first, then fetches an
individual `details/<deck-id>.json` only when a player selects or copies a deck.
This keeps the initial page fast and avoids making requests to MTGTop8 from a
player's browser. Catalog entries include the event, placement, card lists,
mana metadata, source URL, and collection tags such as `last-20-events`,
`last-major-events`, and `mono-color`.

`catalog/` is generated, not committed: it is gitignored, and so is the
`web/ui/public/catalog/` copy the Vite `prebuild` hook makes from it. A checkout
without a catalog builds and runs normally; the deck browser reports that no
catalog was downloaded and every other feature is unaffected.

The bounded synchronizer lives in `tools/deck-catalog/`. It prioritizes the
latest event collections and a small mono-colour sample, then merges new deck
IDs into the existing catalog without deleting older records, so repeated runs
accumulate history:

```sh
node tools/deck-catalog/sync.mjs --format modern --page 0 --events 5 --limit 24 \
  --collection-limit 12 --recent-events 20 --major-events 5
```

Add `--dry-run` to fetch and report without writing files, `--output <dir>` to
write somewhere other than `catalog/`, and `--format pioneer|standard` for the
other supported formats. `--page 0` takes the newest decks (Last 20 Events, Last
Major Events, and a mono-colour sample); `--page N` backfills history. The
source waits 750 ms between requests, so a three-format refresh takes a few
minutes. `tools/deck-catalog/enrich.mjs` recomputes colours and mana profiles
from decks already downloaded, and accepts `--offline`.

`tools/deck-catalog/sync-all.sh` runs that bounded refresh for Modern, Pioneer
and Standard, or for the formats named as arguments
(`./tools/deck-catalog/sync-all.sh legacy pauper`). Run it whenever you want
newer decks.

### Deploying the catalog

`pnpm build` copies `catalog/` into `web/ui/public/catalog/` and Vite emits it
as `dist/catalog/`, so whatever publishes `dist/` publishes the decks with it.
The synchronizer's own bookkeeping under `catalog/state/` is not copied: the
browser never reads it and its card-metadata cache only grows. A full refresh
and deploy is therefore:

```sh
./tools/deck-catalog/sync-all.sh \
  && ./rebuild-wasm.sh --release \
  && (cd web/ui && pnpm build) \
  && rsync -a --delete web/ui/dist/ /path/to/site/ironsmith/
```

Drop the first line to redeploy the catalog already on disk. A deployment
serving the app from a subdirectory needs no extra configuration; the catalog
is fetched relative to the page like every other asset.

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
