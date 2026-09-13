# Expanded English and Spanish frame verification

The corpus now contains 48 cards in both languages: 96 pinned printings.
Five batches of eight new cards were added after the original eight-card
benchmark. Each failing batch drove reusable fixes before another fresh batch
was selected.

The final discovery batch passed all 16 cases without exposing a new masking
or text-fitting issue in the automated checks or full-card visual review:

- Guttersnipe
- Meteor Golem
- Stitcher's Supplier
- Exclusion Mage
- Cleansing Nova
- Resplendent Angel
- Sarkhan's Unsealing
- Poison-Tip Archer

[Fresh-batch comparisons](../test-results/bilingual-batch6/index.html)
show the original scan beside the interactive render.

## Fixes driven by the additional cards

- Rules fitting now respects the full bottom padding reserved for printed
  stats. Spanish Urza exposed text that fit the outer rectangle but collided
  with the power/toughness area.
- Stats protection covers the complete bevel, measured relative to its paper
  color. Pale gray bevels no longer need to look nearly black to be detected.
  Protected pixels are excluded before glyph recognition and validation.
- Physical bottom-edge ornaments preserve their interiors and antialiasing.
  This protection is specific to a complete panel, rather than arbitrary text
  crops, and cannot expand across the entire text block.
- Set-symbol registration searches for complete wide logos, then recovers
  their connected enclosure. Paper is sampled over an area so a translated
  type line cannot turn a row of lettering into the background estimate.
- Title measurement includes the full height around the detected bar, keeping
  leading capitals joined to descenders. Agent of Treachery exposed the old
  crop dropping the first two letters.
- Known short labels can supply fewer baseline letters. First-line geometry
  also supplies typography when current Oracle wording or mixed reminder text
  prevents an exact string match with the printing.
- Font fitting measures the current flex allocation and disables transitions
  on the participating label rows and siblings, including reduced motion.
- Flavor emphasis markers render as emphasis rather than literal asterisks.

No card-name-specific renderer exceptions were added.

## Verification and limits

The final regression run passed all 96 printings with the final renderer:

| Check | English (48) | Spanish (48) |
| --- | ---: | ---: |
| Original-image fallbacks | 0 | 0 |
| English-frame retries | 0 | 3 |
| Text overflows | 0 | 0 |
| Changed reviewed art/decoration pixels | 0 | 0 |
| Browser errors | 0 | 0 |

All cases also passed the fixed residual-ink checks. See the searchable
[complete comparison gallery](../test-results/bilingual-complete/index.html).

The focused unit/browser suite passes 50 tests. ESLint reports no errors;
two unrelated hook-dependency warnings remain in HoverArtOverlay.

The benchmark now rejects missing annotations, invisible type lines, text
overflow into reserved bottom padding, original-image fallback, browser errors,
residual ink above the fixed tolerance, or changed pixels in reviewed art and
decoration bands. Three Spanish scans retry the English frame while retaining
Spanish live text: Lightning Bolt and Swamp have placeholder sources; Crucible
of Worlds has a low-resolution source that fails art registration. A separate
retry audit confirmed the latter reason. None falls back to the untouched
original image.

The annotations are independent of the renderer's current detected bounds.
Two source-region annotations were corrected during review because their broad
rectangles included the printed title-bar rail; tolerances were not loosened.
Small remaining dark pixels are screening signals, not OCR proof of surviving
letters. The fresh batch's largest region fraction was below 0.4% and visual
review found no surviving printed text.

This establishes the requested clean next batch within the tested conventional
frames. It does not establish pixel-identical typography or universal support
for alternate layouts, other languages, or every printing and viewport.

Run the entire pinned corpus from `web/ui` with `pnpm test:frame-benchmark`.
See [fixture maintenance and checks](../tests/fixtures/bilingual-frames/README.md).
