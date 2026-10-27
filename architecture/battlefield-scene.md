# Battlefield scene layout

The battlefield uses a Three.js orthographic scene with CSS3D card objects. React
still renders each existing GameCard into its object's portal. The WebGL forge
renders scenery underneath. This preserves card artwork, tooltips, mana drawers,
keyboard navigation and native pointer hit testing without rasterizing HTML.

This is not a WebGL card-texture renderer. Card content changes can still cause
local DOM layout/paint; fitting the battlefield never changes card DOM dimensions.
Movement and resizing are compositor transforms of fixed-size faces. Browser
zoom should be checked when changing Three's CSS3D renderer, which documents
limitations for non-100% zoom.

## Ownership

- `battlefield-scene-layout.js` allocates persistent slots and calculates target
  poses from the viewport, base card dimensions and reserved insets. No card
  measurements participate in sizing. Engine grouping is authoritative.
- `battlefield-scene.js` owns the orthographic camera, CSS3D objects, current
  interpolated poses, one viewport ResizeObserver and the animation loop.
- `BattlefieldSceneCard.jsx` owns React's portal into a scene-owned element.
  Three removes the wrapper; React owns only its contents.
- `BattlefieldRow.jsx` owns game actions and feeds snapshots and preferences to
  the scene. Existing death/exile layout holds reserve their cells. The older
  grid fit path is bypassed for battlefield rows; compact non-board lists keep it.
- Forge reads projected scene bounds for card shadows and occupied areas. It
  does not observe every card resize or read card geometry back into the solver.
- Mobile lanes have fixed outer dimensions. Their scene supplies overflow
  instead of expanding the surrounding layout as cards are added.

## Invariants

1. Stable member IDs retain slots. Runtime member IDs distinguish a surviving
   group from a leaving/returning permanent. Splits receive distinct visual keys.
2. Front and resource rows face the appropriate player; no density threshold
   resets the layout. Manual placement wins over automatic placement.
3. New targets start from current displayed transforms, not old destinations.
4. Shrink immediately when required. Grow after 400 ms of unchanged demand and
   at least 10% additional room. A viewport resize can grow immediately.
5. Compress gaps and then scale to a 44 CSS-pixel card-width floor (mobile
   overrides can use 40). Crowded fields scroll; they do not shrink indefinitely.
   Native scrollbar gutters are disabled so overflow cannot resize the fitting
   viewport. Wheel, touch and focus scrolling remain available. The renderer
   clips its descendants to the calculated extent; exact-fit rounding residue
   does not create a phantom scroll range.
6. Native pointer and keyboard geometry follows the same transformed element.
   Placement uses current object poses and the solver's empty cells.
7. Local attachment groups fan behind their host with reserved cluster space.
   Cycles, unknown hosts, and attachments spanning different host groups remain
   separate selectable objects. Cross-player hosts remain in their own fields.
8. Idle scenes stop requesting frames. Reduced motion snaps to the target.
   Unmount cancels frames/timers, removes listeners and disconnects observers.

The snapshot adds public `attached_to_ids` for all members of a group and
`battle_protector` when present. An updated WASM build is required to emit those
fields; older snapshots still render normally with independent attachment cards.

## Validation

Run from `web/ui`:

```sh
node --test tests/battlefield-scene-layout.test.js
node --test tests/battlefield-scene-controller.browser.test.js
node --test tests/battlefield-scene.browser.test.js
node --test tests/battlefield-scene-responsive.browser.test.js
node --test tests/battlefield-snapshot-layout.browser.test.js tests/cast-keyboard-navigation.browser.test.js
node --test tests/multiplayer-board-fit.browser.test.js tests/battlefield-responsive-sizing.browser.test.js
```

The scene regression covers fixed DOM dimensions during resizing, continuous
motion, pointer alignment, delayed growth, cancellation of pending growth,
both appearance preferences, and operation with WebGL unavailable. Pure tests
cover group representatives/splits, blink identities, attachments/cycles,
manual placement, dense overflow and reduced motion.

During this change the existing responsive suite also failed against the old
BattlefieldRow at the phase tracker's centering assertion. The existing
multiplayer suite failed against the old row at its local/opponent size-ratio
assertion. The new responsive scene test isolates field sizing and containment
from those surrounding HUD assumptions. The forge's real-puzzle check reaches
its missing visible menu-trigger lookup after passing the card geometry checks.
