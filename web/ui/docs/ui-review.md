# UI consistency review

The interface uses warm charcoal surfaces, brass primary actions, cyan targeting and keyboard focus, green confirmed selections, and coral combat cues. Shared controls, dialogs, tooltips, setup screens, and decision panels derive their presentation from `src/styles/design-system.css`. Player identity and card artwork retain their own colors.

## Changes

- Unified surfaces, text contrast, spacing, corners, inputs, buttons, checkboxes, sliders, popovers, tooltips, and focus indicators. Reduced-motion preferences now cover the interface's decorative transitions.
- Centered dialogs respect caller widths, reserve space for the close control, scroll on short screens, and restore focus on dismissal. Decklists use the shared accessible dialog and wrap long names.
- Number choices reject empty, fractional, unsafe, and out-of-range values; Enter only submits valid choices. Selection controls expose pressed states, with hover visually distinct from selection. Mana plan actions remain outside the scrolling content.
- Phone landscape detection includes wider phones. Battlefield card statistics fit their lanes and focusing a fanned card no longer scrolls the scene; expanded hands scale to available space and keyboard users can dismiss overlays and return to the opener.
- Setup editors use consistent form controls. Empty deck loading and card injection are disabled. Puzzle players flow into a responsive grid with readable zone editors. Sideboarding labels identify the card and destination.
- The card forge preserves working space when switching card layouts. Lobby titles follow the active mode, and joining requires a code. The rotation gate is limited to gameplay, leaving deck, puzzle, and sideboard setup usable in portrait mode.

## Coverage and reproducible checks

Reviewed the component inventory under `src/components`: board and mobile scenes, cards, decision routing and individual decisions, layout/tool dialogs, rails and inspectors, overlays, and shared UI primitives. Source review includes conditional and remote-dependent branches; it is not a claim that every possible game state was exercised live.

Browser checks used the actual app and production components at desktop, phone landscape, and portrait widths. Checked settings, deck setup, puzzle setup, Add Card, the card forge's layout controls, create/join lobby forms, empty match verification, logs, local decklist availability, expanded hand, keyboard dismissal, and long-content dialog layout. The local WASM game and isolated fixtures were used; no real multiplayer session was created.

The development-only workshop is available at `/tests/ui-audit.html` while Vite runs. It imports production styles and components with a mock game context and records submitted commands without changing a game. It covers shared control states, number and text entry, mode/object/target selection, attackers, blockers, mana plans, panel/strip layouts, disabled interaction, and a decklist with unusually long names. It is not included as an entry in the production build.

Run the focused regression checks from the repository root:

```sh
node --test web/ui/tests/ui-input-layout.test.js web/ui/tests/decision-button-style.test.js web/ui/tests/decision-key.test.js web/ui/tests/hand-drag-intent.test.js web/ui/tests/mobile-battle-layout.test.js
```

All 28 checks passed. The production build passed. Lint on modified JS/JSX reported no errors and one existing dependency warning for `exportAuditTranscript` in `GameContext.jsx`. The optional Scryfall translation refresh was skipped when its network lookup failed; existing translation assets remained available. The build retains its existing large bundle warning.

## Verification limits

The bundled lean engine reports that custom source compilation requires `ironsmith-compiler-wasm` and compiled-artifact registration. The forge's controls, layout, and displayed error state were reviewed; successful custom-card compilation could not be verified with that engine build. Live peer negotiation, remote deck submission, and imported audit replay were not exercised. These require runtime/network scenarios beyond the visual fixtures.
