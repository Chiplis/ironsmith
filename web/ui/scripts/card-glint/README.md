# card-glint frontend integration

`pnpm build` starts with `pnpm assets:card-glint`. It fetches the current `main`
commit from https://github.com/vishvananda/card-glint on every invocation. Network
or compilation failures stop the build; there is no silent stale-cache fallback.
The existing build → rsync → commit → push deploy command needs no changes.

The disposable checkout and compiler cache live in
`web/ui/node_modules/.cache/card-glint`. A changed upstream commit builds its
browser library and matching WASM together. Unchanged commits reuse the completed
bundle after a successful fetch. Vite embeds the commit and copies the bundle,
licenses and Magic pack notice from `public/card-glint/<commit>/` into `dist`.
Old generated commit directories are removed. Do not commit generated assets.

First build requires Git, Node/npm, Cargo and rustup. Rustup installs the upstream
pinned toolchain/wasm target; the script installs the matching wasm-bindgen CLI
inside the cache without replacing the developer's globally installed CLI.
On macOS a local linker wrapper resolves that toolchain's LLVM library.
Only the runtime library is bundled: upstream's declaration generation and
showcase website are not needed by this JavaScript application.

`pnpm dev` and `pnpm lan` refresh the renderer too. When invoking Vite directly
(e.g. browser tests), run `pnpm assets:card-glint` first on a fresh checkout.

The app uses `card-glint/magic/presentation`'s Canvas2D static-face API, sharing a
96 MiB snapshot cache and a priority queue with two loading slots. Background
preparation uses one slot, leaving capacity for foreground requests. Your visible
hand is warmed first, then all public battlefield cards, on initial load and game
state changes. New snapshots replace obsolete queued work. Shared frame/art
bitmaps use a separate 64 MiB cache, preserving upstream SVG fixes and aliases.

Build's card preview also loads `browser.js`, `magic.js` and `renderer_bg.wasm` for
card-glint's WebGL renderer. It shows the chosen finish, print effects (cardstock
relief and ink dots), printing style and frame, border and rarity; "Current
printing" keeps a real card's own set. Only animated finishes run a clock, which
pauses off-screen, in hidden tabs and under reduced motion. Without WebGL2 the
preview falls back to the static face. Choices are saved with library cards and
applied to the copy added to a game; the inspector draws that copy with the same
renderer. Every face opts into card-glint's automatic printing calibration.

Engine text, current colors/stats and selected faces remain authoritative;
Scryfall printing metadata supplies decorations and the chosen artwork.
The Magic pack loads its pinned external frame/font assets at runtime, as upstream
does; those assets are not included in the browser library bundle.

Ordinary and planeswalker ability targets use card-glint's measured text. When
available actions cannot be mapped safely on another layout, the existing
interactive SimpleCardFrame remains the fallback. Renderer/network failures also
keep that fallback usable. Legacy MSE implementation and audit fixtures remain
for reference, but the app no longer initializes it or packages its corpus.

Validation:

```
pnpm assets:card-glint
node --test tests/card-glint.test.mjs tests/card-glint.browser.test.mjs tests/card-glint-build.browser.test.mjs
pnpm build
```
