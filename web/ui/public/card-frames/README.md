# Standard default card frames

`default/` contains 34 local WebP frame templates: all 32 WUBRG color subsets
(including a colorless artifact frame), plus land and colorless nonartifact
materials. Total image payload is about 650 KiB; a card loads only its selected
frame. The current card supplies the artwork, name, mana cost, type, rules and
stats. Donor flavor, symbols, art, stats and attribution are not reused.

The source printings are pinned in
`web/ui/scripts/fixtures/default-card-frame-donors.json`, including Scryfall IDs
and source URLs. Every source passed `sampleCardFrameColors` with status
`masked`. Visual inspection found residual glyphs and donor watermarks in some
source masks, so `default-card-frame-template.js` normalizes paper interiors
from sampled clean material and removes donor art and attribution. The final
bundled templates, rather than the unprocessed masks, are the reusable assets.

Selection uses live colors, then mana cost when colors are absent. Colorless
lands and nonartifact cards have their own materials. Nonstandard layouts are
excluded both by explicit layout/type and by a generated local card/face-name
catalog, so waiting for Scryfall metadata cannot temporarily flatten a Saga,
adventure, split card, or other special layout. Those cards retain the existing
renderer and fallback path.

Regenerate from the UI directory with:

```sh
node scripts/build-default-card-frames.mjs
```

This uses the existing mask renderer in Chromium and downloads the pinned scans.
For an offline rebuild, `DEFAULT_FRAME_MASKS` can point to a JSON array of
`{key, printing, style, typography}` mask results. The generator also refreshes
special-layout exclusions from the repository's `cards.json`.

Verification:

```sh
node --test tests/default-card-frames.test.js tests/default-card-frames.browser.test.js tests/card-network-independence.browser.test.js
```

The browser fixture checks every color set with live replacement art/text and
blocked Scryfall access. Contact sheets are written to
`web/ui/test-results/default-card-frames/` for visual inspection.
