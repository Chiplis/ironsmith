# English / Spanish full-card benchmark

Run from `web/ui`:

```sh
pnpm test:frame-benchmark
```

This renders the production card-frame component for 48 English/Spanish
pairs (96 printings) from AVR, M11, DMR, M21, M20 and M19. The corpus covers
all five colors, gold and artifact frames, legendary crowns, nonbasic lands,
long creature types, activated abilities, modal bullets, mixed italic reminder
text, flavor emphasis, centered text, boxed set logos and security stamps.
Scan quality varies in both languages. Two Spanish fixtures are explicitly
marked `placeholder` by Scryfall and exercise the English-frame retry, with
Spanish live text retained.

The latest independent batch is Guttersnipe, Meteor Golem, Stitcher's Supplier,
Exclusion Mage, Cleansing Nova, Resplendent Angel, Sarkhan's Unsealing and
Poison-Tip Archer. It was selected after fixing preceding batches and passed
without another renderer change. Retain these fixtures as regression evidence;
use previously untested cards for the next discovery batch.

All Scryfall images, metadata and set symbols are pinned locally. External
requests during the benchmark are fulfilled from this corpus or blocked.
`corpus.js` identifies each printing and image status. The cache script can
populate the corpus; it preserves existing printing choices. Review new scans
and annotations if intentionally replacing fixtures.

Outputs in `test-results/bilingual-final` (or the selected output directory):

- `results.json`: source language/status, renderer mode, retry, failure reason,
  residual-ink counts, changed art/decorative pixels, and rendered text overflow.
- `index.html`: paired original/rendered full-card images for human review.
- One full-card comparison PNG per printing.

The default CLI output directory is `test-results/bilingual-frames`.
`FRAME_BENCHMARK_OUTPUT`, `FRAME_BENCHMARK_FILTER` (regular expression), and
`FRAME_MASK_DIAGNOSTICS=1` control reports and debugging. The package script
sets `FRAME_BENCHMARK_ASSERT=1`, rejecting fallback, residual ink above the
reviewed tolerance, missing annotations, invisible type lines, visible overflow,
browser errors, or changes in protected artwork/decorative regions.

`ink-regions.js` freezes reviewed scan-coordinate rectangles, independent of
future detector output. It must not be regenerated to make a failing run pass.
Watermarks, symbols, frame rails and collector text are deliberately excluded
from text-erasure counts. Separate reviewed bands check boxed set logos,
security stamps and power/toughness bevels for changes. Text-fit checks reserve
the actual bottom padding so a line cannot overlap the stats area. Artwork is compared against the actual source scan,
including the English scan when a placeholder triggers a retry.

Ink counts are screening signals, not OCR ground truth. They can miss pale
halos or damage outside the annotated regions. Review every full-card image
when changing region detection, masking, font fitting, or these fixtures.
Passing this small representative corpus does not establish support for every
printing, alternate layout, language, or viewport.
