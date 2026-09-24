# Ironsmith Frontend Audit — `web/ui`

## 1. Executive Summary

This is a real-time, peer-to-peer multiplayer card game UI (React 19 + Vite 7 + Tailwind 4, WASM game engine, PeerJS transport) — not a typical CRUD app, and the review should be read with that in mind. The codebase shows real engineering depth in the places that matter most for this domain: the Scryfall data layer has proper request de-duplication/backoff/caching, the peer-lobby logic is already split into focused modules (`connections.js`, `crypto-resync.js`, `messaging.js`), and there's a genuinely large custom test suite (282 files under `tests/`, plus a dev-only interactive workshop at `/tests/ui-audit.html`). The team has also already solved the "avoid context re-render storms" problem once — the match clock uses `useSyncExternalStore` instead of context — it just hasn't applied that pattern everywhere it's needed.

The three biggest issues, in order of impact-to-effort:

1. **One god-context drives 60 files.** `GameContext.jsx` merges the fast-changing game `state`/`dispatch` with rarely-changing settings (UI font, lobby, semantic threshold, trigger ordering) into a single memoized value. Every game action re-renders every one of the 60 files that call `useGame()`, including 2,700–3,700-line components.
2. **Zero code splitting.** Nothing in the app uses `React.lazy`/`import()`. Setup-only screens (card forge, deck browser, lobby overlay) and both full i18n locale catalogs (~4,300 lines combined) ship in the initial bundle regardless of what the user actually needs.
3. **No error boundary anywhere in `src`.** A render exception in any component — especially the several that exceed 2,500 lines — white-screens an in-progress multiplayer match with no recovery, despite the app already having the checkpoint/audit-replay infrastructure a boundary could use to recover.

Overall health: solid domain logic, thin on the React-specific performance/resilience layer. Nothing here requires a rewrite — every recommendation below is incremental and independently shippable.

---

## 2. Findings Table

| ID | Category | Severity | Location | Problem | Impact | Effort |
|----|----------|----------|----------|---------|--------|--------|
| P1 | Performance | High | `src/context/GameContext.jsx:2791-2885` | Single context merges hot state with cold settings; consumed by `useGame()` in 60 files | Full-tree re-render on every game action | M |
| P2 | Performance | High | `components/layout/TableActionControls.jsx`, `TopbarMenuSheet.jsx`, `board/DeckLoadingView.jsx`, `i18n/catalog.js` | No `React.lazy`/dynamic `import()` anywhere; both locale catalogs (~4,300 lines) always loaded | Larger initial bundle, worse cold-start parse/exec, worse LCP/INP | S–M |
| P3 | Performance | Medium | `components/board/BattlefieldRow.jsx:2615-2660` → `components/cards/GameCard.jsx:830` | Every list item gets newly-allocated inline handlers + computed `className` each parent render | Even adding `memo()` to `GameCard` wouldn't stop re-renders; wasted memoization effort elsewhere | M |
| P4 | Performance | Low | `src/lib/scryfall.js` (11 module-level `Map` caches) | No eviction/size cap on caches | Minor memory growth risk in very long sessions | S |
| L1 | Legibility | Medium | `hooks/peer-lobby/validation.js` (4,372L, one function), `messaging.js` (4,360L), `crypto-resync.js` (4,234L), `context/GameContext.jsx` (2,900L), `overlays/DecisionPopupLayer.jsx` (3,696L), `right-rail/HoverArtOverlay.jsx` (3,107L), `board/BattlefieldRow.jsx` (2,743L) | Several files/single functions exceed 2,500–4,300 lines | Hard to review, navigate, or safely modify | L |
| L2 | Legibility | Medium | repo root (no `tsconfig.json`/`jsconfig.json`) | `@types/react`, `@types/react-dom`, `@types/node` installed but nothing consumes them; plain JS/JSX with no compile-time checking | No contract checking on a large, deeply nested domain (game state, decision commands, peer messages) | S (jsconfig) / L (full TS) |
| L3 | Legibility | Low | `eslint.config.js` | Only `react-hooks` + `react-refresh`; no `max-lines`, no import ordering | Nothing would have flagged L1 automatically | S |
| R1 | Reusability | Medium | `lib/scryfall.js:232`, `i18n/cardTranslations.js:9`, `i18n/generatedTextTranslations.js:28` (+ 8 more ad hoc `import.meta.env` reads) | `baseAssetUrl()` duplicated verbatim in 3 files; base-URL/env logic reimplemented in 8 more | Drift risk if base-path logic ever needs to change | S |
| R2 | Reusability | Medium | `components/overlays/DecisionPopupLayer.jsx` | ~45 components/helpers for every decision-popup variant crammed into one file, while `components/decisions/*Decision.jsx` already splits sibling decision UIs one-file-per-kind | Inconsistent application of an already-established pattern in the same codebase | M |
| R3 | Reusability | Low | `src/lib/` (108 files, flat) | No subfolder grouping by concern | Mild discoverability cost | M |
| S1 | Scalability | High | entire `src` (no `ErrorBoundary`/`componentDidCatch`) | No error boundary anywhere | Any render exception white-screens a live match with no recovery | S–M |
| S2 | Scalability | High | `.github/workflows/` (only `publish-npm.yml`) | `pnpm lint`, `pnpm build`, and the ~282 test files are never run in CI | Large existing test investment provides no regression safety net | S |
| S3 | Scalability | Low | 12 files reading `import.meta.env` directly | No central config module | Related to R1; scattered defaults/validation | S |

---

## 3. Detailed Findings

### P1 — Monolithic `GameContext` causes tree-wide re-renders (High)

`src/context/GameContext.jsx:2791-2850` builds one `useMemo`'d value with ~50 keys — everything from `state`/`dispatch` (changes on every game action) to `uiFont`/`setUiFont`, `multiplayer`, `logEntries`, and lobby functions (change rarely). Because they're one object, the reference changes whenever **any** dependency changes, and every consumer of `useGame()` re-renders regardless of which slice it actually reads.

`grep -rl "useGame(" src` returns **60 files**, including `BattlefieldRow.jsx:906` (`const { state, cancelDecision, dispatch, loading } = useGame();`) — a 2,743-line component with heavy layout math — which re-renders in full every time, say, the player changes their UI font or a lobby message arrives.

The team already fixed this exact problem for the match clock: `matchClockStore` is a separate external store consumed via `useSyncExternalStore` (`GameContext.jsx:2898-2899`) specifically so clock ticks don't blow through the context. The same treatment hasn't been applied to the rest of the settings/lobby slice.

**Before:**
```jsx
// GameContext.jsx
const value = useMemo(() => ({
  state, dispatch, dispatchInBackground, cancelDecision, loading, game,
  uiFont, setUiFont, playerAccentOverrides, setPlayerAccentOverride,
  multiplayer, createLobby, joinLobby, leaveLobby, logEntries, pushLog,
  semanticThreshold, setSemanticThreshold, /* ...45 more keys */
}), [ /* ...50 deps... */ ]);

return <GameContext.Provider value={value}>{children}</GameContext.Provider>;
```

**After (split by change-frequency, same pattern as `matchClockStore`):**
```jsx
const gameStateValue = useMemo(
  () => ({ state, dispatch, dispatchInBackground, cancelDecision, loading, game }),
  [state, dispatch, dispatchInBackground, cancelDecision, loading, game]
);
const gameSettingsValue = useMemo(
  () => ({ uiFont, setUiFont, playerAccentOverrides, setPlayerAccentOverride, semanticThreshold, setSemanticThreshold }),
  [uiFont, setUiFont, playerAccentOverrides, setPlayerAccentOverride, semanticThreshold, setSemanticThreshold]
);
const lobbyValue = useMemo(() => ({ multiplayer, createLobby, joinLobby, leaveLobby, logEntries, pushLog }),
  [multiplayer, createLobby, joinLobby, leaveLobby, logEntries, pushLog]);

return (
  <GameStateContext.Provider value={gameStateValue}>
    <GameSettingsContext.Provider value={gameSettingsValue}>
      <LobbyContext.Provider value={lobbyValue}>{children}</LobbyContext.Provider>
    </GameSettingsContext.Provider>
  </GameStateContext.Provider>
);

// BattlefieldRow.jsx — only re-renders when state/dispatch/loading change
const { state, cancelDecision, dispatch, loading } = useGameState();
```
Do this incrementally: introduce `GameSettingsContext` and `LobbyContext` first (low risk, rarely-read values), move consumers over file by file, leave `GameStateContext` for last since it's read almost everywhere anyway.

---

### P2 — No code splitting anywhere (High)

`grep -rn "React.lazy\|lazy(\|import(" src` returns zero matches. Everything is a static `import`, including components that are never shown during actual gameplay:

- `LobbyOverlay.jsx` (1,001L) — statically imported in `Shell.jsx:21`
- `CreateCardForgeSheet.jsx` (760L) — imported in both `TableActionControls.jsx:6` and `TopbarMenuSheet.jsx:21`
- `CompetitiveDeckBrowser.jsx` → imported by `DeckLoadingView.jsx:14` → imported by `TableCore.jsx:10`

And `i18n/catalog.js:1-2` eagerly imports **both** locale catalogs regardless of active locale:
```js
import * as en from './locales/en.js';   // 1,556 lines
import * as es from './locales/es.js';   // 1,728 lines + es.rules.js (1,035 lines)
```
Every user downloads ~4,300 lines of translation strings even though only one locale is ever active per session.

**Before:**
```jsx
// TopbarMenuSheet.jsx
import CreateCardForgeSheet from "./CreateCardForgeSheet";
...
{showForge && <CreateCardForgeSheet {...props} />}
```

**After:**
```jsx
import { lazy, Suspense } from "react";
const CreateCardForgeSheet = lazy(() => import("./CreateCardForgeSheet"));
...
{showForge && (
  <Suspense fallback={null}>
    <CreateCardForgeSheet {...props} />
  </Suspense>
)}
```
Same treatment for `LobbyOverlay` and `CompetitiveDeckBrowser`. For i18n, this is a bit more work since `t()`/`getActiveLocale()` are currently synchronous — plan for a `M`-effort follow-up that loads the non-default locale on demand in `I18nProvider.setLocale` with a brief loading state, keeping `en` eager as the default.

---

### P3 — Memoization would be wasted without fixing prop identity first (Medium)

`GameCard.jsx:830` takes 15+ callback/style props. Its caller, `BattlefieldRow.jsx:2615-2660`, allocates all of them fresh on every render:

```jsx
// BattlefieldRow.jsx:2640-2653 — new closures + new array every render, per card
<GameCard
  key={card.__battlefield_layout_hold_key || card.id}
  className={[...].filter(Boolean).join(" ")}
  onClick={isLayoutHold ? undefined : ((event) => handleCardSelectionClick(event, card))}
  onMouseEnter={isLayoutHold ? undefined : ((event) => { ...; hoverCard(card.id); ... })}
  onFocus={isLayoutHold ? undefined : (() => { ... })}
  ...
/>
```
Only 3 of 131 `.jsx` files use `React.memo` (`CompetitiveDeckBrowser.jsx`, `DeckCatalogParts.jsx`, `MiniatureCardFrame.jsx` — all list-row components, not the hot gameplay path), while 54–59 files use `useMemo`/`useCallback`. That asymmetry is the tell: memoized values/callbacks only pay off when the *consumer* is wrapped in `memo` with stable props — here neither condition holds for the highest-traffic component in the tree.

Wrapping `GameCard` in `memo()` today would do nothing, because `className`, `onClick`, `onMouseEnter`, etc. are new references every render regardless of whether `card` itself changed.

**Fix, in order:**
1. Memoize the per-card handlers in `BattlefieldRow` keyed by stable identity (`useCallback` closing over `card.id`, or a stable dispatch table built once via `useMemo`), not recreated per card per render.
2. Only then wrap `export default memo(GameCard)`.

```jsx
// BattlefieldRow.jsx
const handleCardClick = useCallback((cardId) => (event) => handleCardSelectionClick(event, cardId), [handleCardSelectionClick]);
// or, cheaper: pass stable top-level handlers + cardId, let GameCard resolve behavior internally
<GameCard cardId={card.id} onSelect={handleCardSelectionClick} ... />
```
```jsx
// GameCard.jsx
export default memo(function GameCard({ cardId, onSelect, ... }) { ... });
```

---

### L1 — Several files far exceed a reviewable size (Medium)

| File | Lines | Notable structure |
|---|---|---|
| `hooks/peer-lobby/validation.js` | 4,372 | **One function**, `usePeerLobbyValidation` (line 97 → EOF) |
| `hooks/peer-lobby/messaging.js` | 4,360 | — |
| `hooks/peer-lobby/crypto-resync.js` | 4,234 | — |
| `lib/multiplayer-audit.js` | 4,083 | — |
| `components/overlays/DecisionPopupLayer.jsx` | 3,696 | ~45 components/helpers in one file |
| `right-rail/HoverArtOverlay.jsx` | 3,107 | — |
| `context/GameContext.jsx` | 2,900 | — |
| `components/board/BattlefieldRow.jsx` | 2,743 | — |

`validation.js` is the sharpest example: it's not 4,372 lines of many small functions, it's a **single hook** from line 97 to the end of the file. Its sibling files in the same folder (`connections.js`, `crypto-resync.js`, `messaging.js`, `shared.js`, `audit-material.js`, `trusted-sequencer.js`) already demonstrate the fix — the peer-lobby logic *is* split by concern at the file level. `validation.js` just needs the same split applied *inside itself* (e.g., separate concern-scoped validator functions — connection validation, crypto/resync validation, message-shape validation — composed by the hook, rather than one continuous function body).

Recommend starting with `validation.js` as the pilot since the target decomposition (mirroring its siblings) is already evident from the surrounding directory.

---

### L2 — Type packages installed but unused (Medium)

`package.json` devDependencies include `@types/node`, `@types/react`, `@types/react-dom`, but there is no `tsconfig.json`, no `jsconfig.json`, and no `typescript` package — nothing consumes these types. The app is plain JS/JSX with no compile-time contract checking over a genuinely complex domain: game state shapes, decision commands, peer-protocol messages, audit checkpoints.

Given the scale of a full TS migration isn't warranted here (respecting the "incremental, not a rewrite" constraint), the cheap first step is a `jsconfig.json` with `checkJs` for editor-level type hints on `@types/react` without touching a single source file:

```json
// jsconfig.json
{
  "compilerOptions": {
    "target": "ES2022",
    "module": "ESNext",
    "moduleResolution": "bundler",
    "jsx": "react-jsx",
    "checkJs": false,
    "paths": { "@/*": ["./src/*"] }
  },
  "include": ["src"]
}
```
This alone fixes path-alias IntelliSense (currently only configured in `vite.config.js:32-34`, not visible to the editor/TS server) and gives you `@types/react` autocomplete for free. Turning on `checkJs` per-file later (`// @ts-check`) is a good next step for the highest-risk modules — `lib/multiplayer-audit.js`, `hooks/peer-lobby/*` — without committing to a full migration.

---

### R1 — `baseAssetUrl()` duplicated verbatim (Medium)

Identical 7-line implementations exist in three files:

```js
// lib/scryfall.js:232, i18n/cardTranslations.js:9, i18n/generatedTextTranslations.js:28 — byte-for-byte identical
function baseAssetUrl() {
  const configured = typeof import.meta !== "undefined" ? import.meta.env?.BASE_URL : null;
  const base = configured || "/";
  return new URL(base, globalThis?.location?.href || "http://localhost/").href;
}
```
`i18n/cardTranslations.js:1` already imports from `lib/scryfall.js` — the fix is a one-line export change, not a new abstraction:

**Before:** three copies, drift risk if base-path logic ever changes (e.g., adding a CDN prefix).
**After:**
```js
// lib/scryfall.js
export function baseAssetUrl() { ... }

// i18n/cardTranslations.js
import { cardRouteKey, fetchScryfallLocalizedCardTranslation, baseAssetUrl } from "@/lib/scryfall";
// delete the local copy
```
Same treatment for `lib/random-game-catalog.js`, `lib/catalog-client.js`, `lib/unsupported-card-substitution.js`, which reimplement the `BASE_URL` fallback inline rather than importing.

---

### R2 — `DecisionPopupLayer.jsx` doesn't follow the codebase's own split pattern (Medium)

`components/decisions/` already splits decision UIs one file per kind: `TargetsDecision.jsx`, `SelectOptionsDecision.jsx`, `ManaPaymentDecision.jsx`. But `components/overlays/DecisionPopupLayer.jsx` (3,696 lines, ~45 components/helpers) handles *popup* rendering for every decision kind in one file. Splitting it into `overlays/decision-popups/{kind}.jsx` mirroring `components/decisions/` would make the two directories consistent and each file reviewable in isolation — same benefit as L1's `validation.js` case, applied to a component file instead of a hook.

---

### S1 — No error boundary anywhere (High)

`grep -rln "componentDidCatch\|ErrorBoundary" src` returns nothing. Given several components exceed 2,700 lines and the app is driving a live P2P match, an uncaught render error currently means a full white-screen with no way back — even though the app already has the primitives to recover: `exportSyncCheckpoint`, `exportPublicAuditCheckpoint`, and audit-transcript replay (`GameContext.jsx:2761-2775`, `e2eApi`) exist for other purposes and are exactly what a recovery UI would need.

**Add one boundary around the table, using existing recovery primitives:**
```jsx
// components/layout/TableErrorBoundary.jsx
import { Component } from "react";

export class TableErrorBoundary extends Component {
  state = { error: null };
  static getDerivedStateFromError(error) { return { error }; }
  componentDidCatch(error, info) { console.error("Table render error", error, info); }
  render() {
    if (this.state.error) {
      return <TableRecoveryScreen onRetry={() => this.setState({ error: null })} />;
    }
    return this.props.children;
  }
}
```
```jsx
// Shell.jsx
<TableErrorBoundary>
  <TableCore />
</TableErrorBoundary>
```
`TableRecoveryScreen` can offer "reload and resync from checkpoint" using the existing `exportSyncCheckpoint`/replay machinery — this is largely wiring, not new infrastructure.

---

### S2 — No CI gate despite a large test suite (High)

`.github/workflows/` contains only `publish-npm.yml`. `tests/` has ~282 files (custom `node --test` + Playwright-driven browser scenarios covering peer resync, crypto, i18n, WASM engine correctness), and `package.json` defines `lint`, `build`, and a dozen `test:*` scripts — none of which run automatically on push or PR.

**Add a workflow that runs what already exists:**
```yaml
# .github/workflows/ci.yml
name: CI
on: [push, pull_request]
jobs:
  build-and-test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: pnpm/action-setup@v4
      - uses: actions/setup-node@v4
        with: { node-version: 22, cache: pnpm }
      - run: pnpm install --frozen-lockfile
      - run: pnpm lint
      - run: pnpm build
      - run: node --test web/ui/tests/*.test.js web/ui/tests/*.test.mjs
```
Start with lint + build + the fast `node --test` suites; add the Playwright-driven scenarios (`test:p2p-browser`, `test:relay`, etc.) as a second job once you've confirmed they run headlessly in CI — some currently assume a live dev server (`vite --mode lan`), which will need a `webServer`-style startup step.

---

## 4. Recommended Target Structure

The current top-level shape (`components/ board|cards|decisions|deck|layout|left-rail|overlays|right-rail|ui`, `context/`, `hooks/`, `i18n/`, `lib/`) is reasonable for a single-screen table app — this is **not** a "switch to feature folders" recommendation. The two concrete structural changes worth making:

```
src/
  context/
    GameStateContext.jsx      # state, dispatch, loading, game (P1)
    GameSettingsContext.jsx   # uiFont, accents, thresholds, debug (P1)
    LobbyContext.jsx          # multiplayer, createLobby/joinLobby/... (P1)
    GameContext.shared.js     # keep as-is
  components/
    overlays/
      decision-popups/        # split DecisionPopupLayer.jsx by kind (R2)
        targets-popup.jsx
        select-options-popup.jsx
        mana-payment-popup.jsx
        index.jsx              # thin dispatcher, not a barrel of implementation
  hooks/
    peer-lobby/
      validation/              # split validation.js by concern (L1)
        connection-validation.js
        crypto-validation.js
        message-validation.js
        index.js
  lib/
    env.js                     # single baseAssetUrl()/import.meta.env surface (R1, S3)
```

**ESLint enforcement worth adding**, given L1/L3:
```js
// eslint.config.js
rules: {
  'no-unused-vars': ['error', { varsIgnorePattern: '^[A-Z_]' }],
  'max-lines': ['warn', { max: 800, skipBlankLines: true, skipComments: true }],
}
```
Set the threshold high enough (800) that it doesn't fire on every file today, but low enough to catch new files heading toward the 2,500+ line territory before they get there. Ratchet down over time as L1 work lands.

---

## 5. Refactor Roadmap

**Phase 0 — Quick wins (each < 1 day, ship independently)**
1. S2: Add the CI workflow running existing `lint`/`build`/`node --test` scripts.
2. R1: De-duplicate `baseAssetUrl()`, export from `lib/scryfall.js`, update 3 call sites.
3. S1: Add `TableErrorBoundary` around `TableCore` with a basic reload fallback (recovery-from-checkpoint can follow).
4. L3: Add `max-lines` ESLint rule (warn-only) + a `jsconfig.json` (L2's cheap first step).

**Phase 1 — This sprint**
5. P2: Wrap `CreateCardForgeSheet`, `LobbyOverlay`, `CompetitiveDeckBrowser` in `React.lazy`/`Suspense`.
6. P1: Split `GameContext` into `GameStateContext` + `GameSettingsContext` + `LobbyContext`; migrate consumers incrementally, settings/lobby first.
7. S1: Wire `TableErrorBoundary`'s fallback to the existing checkpoint/audit-replay machinery for real recovery, not just "reload."

**Phase 2 — Longer-term (needs design discussion, not urgent)**
8. L1: Split `hooks/peer-lobby/validation.js` into concern-scoped modules (pilot for the pattern); repeat for `messaging.js`, `crypto-resync.js`, `DecisionPopupLayer.jsx`.
9. P2 (locale splitting): move `es.js`/`es.rules.js` behind dynamic `import()` in `I18nProvider`, with a brief loading state on locale switch.
10. P3: Stabilize `GameCard` prop identity in `BattlefieldRow`, then add `memo()` — do this *after* P1, since some of the unstable props originate from the unmemoized context value.
11. L2: Selectively enable `checkJs` on the riskiest modules (`multiplayer-audit.js`, `peer-lobby/*`) rather than a full TS migration.

---

## 6. Suggested Tooling

- **`rollup-plugin-visualizer`** (or `vite-bundle-visualizer`) — nothing currently measures the bundle; add it before starting P2 so the code-splitting wins are visible and you don't fly blind.
- **`eslint-plugin-import`** (or `eslint-plugin-boundaries`) — enforce that `components/ui` primitives never import from `components/board`/`overlays`, catching accidental coupling as the codebase grows.
- **GitHub Actions CI** (S2) — the single highest-leverage addition given the test investment that already exists and isn't gated.
- **Lighthouse CI** — worth adding once P2's code-splitting lands, to track LCP/INP regressions on the cold-start path going forward. Not urgent before that, since there's little to move the needle on yet.


---
