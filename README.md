Magic: The Gathering engine supporting automatic oracle text parsing and custom card compilation via natural language and 4-way multiplayer backed by Zero Knowledge proofs.

~26k cards supported, with more to come soon!

https://chiplis.com/ironsmith

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
