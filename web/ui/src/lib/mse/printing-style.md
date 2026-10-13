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
general MSE template is used. If that template is also absent, the existing
renderer remains visible. The existing renderer also stays visible during
loading and on rendering errors (including missing fonts/assets). A rendering
error does not silently retry the same treatment with different styling.

Checks: `mse-printing-style.test.mjs` covers selection policy;
`mse-printing-frame.browser.mjs` covers metadata propagation, printing changes,
stale requests, live text preservation, explicit overrides, and both fallback
paths. `mse-react-frame.browser.mjs` covers the shared frame lifecycle.
