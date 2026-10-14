# Printing-aware frame selection

`selectMsePresentation` chooses a package and native styling options. The order
is an explicit per-card `mse_style`, an explicit provider default, a recognized
printing treatment that is installed, then the general template for the live
card's layout. Automatic selection uses the session's `styles` inventory.
Explicit packages remain unrestricted, including packages outside that inventory.

`MseCardFrame` resolves metadata through the shared Scryfall printing cache using
the displayed image URL. Until it resolves, metadata on `card.printing` or the
card snapshot is usable. A resolved printing is authoritative, including absent
optional fields; stale metadata from a previous image is ignored. Only frame
selection and native border color use this metadata. Live rules, stats, names,
and face layout still come from the engine snapshot.

Recognized automatic treatments:

- Retro (`1993`/`1997`), 8th Edition (`2003`), and future frames. ABU set codes
  refine the retro choice. Release date does not override a retro reprint's frame.
- Borderless and extended art through M15 Main's `borderless art` and
  `frameless art` options, respectively.
- Showcase families for THB, IKO, ZNR, KHM, DMU, CLB, WHO, PIP, MKM, and DSK,
  only when the printing carries the showcase effect.
- Dedicated EXP, MPS, MP2, and non-Japanese STA bonus-sheet treatments.
- Old-frame planeswalkers and Sagas, and 8th Edition planeswalkers.

Unknown/ambiguous showcase designs and full-art treatments use the general
layout. Language alone cannot identify Japanese alternate-art Archives.
Multipart layouts and DFCs retain their general layout rather than being
flattened into single-face decorative packages. Adding coverage requires a
reliable metadata rule and an appropriate package; set identity alone usually
does not distinguish treatments. Package compatibility does not establish exact
visual parity for every printing.

If a recognized package is absent, its styling options are discarded and the
general MSE template is used. If that template is also absent, `SimpleCardFrame`
remains visible. The same asset-free fallback stays visible during loading and
on rendering errors (including missing fonts/assets). A rendering error does
not silently retry the same treatment with different styling.

The miniature and floating inspector share `CardFramePresentation`: MSE plus
`SimpleCardFrame`, with no scan masking, registration, or stock-frame selection.
The fallback uses the current art crop, live text/stats and CSS colors; failed
artwork leaves its art area empty. Inspector abilities are interactive directly
on the card. The fallback uses semantic buttons in its rules text; MSE uses
transparent buttons positioned from the renderer's measured text runs, including
wrapping, rotation and individual grouped mana symbols. If an available ability
cannot be mapped to measured text, the interactive fallback remains visible.
There is no separate details panel. Live counters stay on the card.
Rules keywords and non-action mana symbols retain their explanation popovers.
Abilities have no permanent border; hovering a wrapped ability tints all of its
lines together, while keyboard focus remains visible.

`GameCard` and `FloatingCardPreview` no longer prepare masks in the background.
Legacy scan libraries, registration fixtures and stock assets remain for offline
comparison/generation; they are not imported by these live card surfaces.

Checks: `mse-printing-style.test.mjs` covers selection policy;
`mse-printing-frame.browser.mjs` covers metadata propagation, printing changes,
stale requests, live text preservation, explicit overrides, and both fallback
paths. `mse-react-frame.browser.mjs` covers the shared frame lifecycle.
`card-frame-reduction.browser.mjs` replaces the old stock-upgrade, default-frame,
and masked-miniature browser suites. It checks the real inspector, fallback
visibility, live stats, miniature scaling, multipart text, ability activation,
keyboard mana choices and the absence of scan/stock dependencies.
