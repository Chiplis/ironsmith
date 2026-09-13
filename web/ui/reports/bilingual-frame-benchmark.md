# English and Spanish frame benchmark — 2026-09-12

Follow-up: [expanded 96-printing verification](bilingual-frame-expansion.md).

Added an offline benchmark that renders the production preview for eight paired
English/Spanish printings. It saves full-card comparisons and measures residual
ink in frozen, reviewed regions, artwork preservation, text visibility,
overflow, fallback behavior and browser errors.

| Result | English (8) | Spanish (8) |
| --- | ---: | ---: |
| Original-image fallbacks before fixes | 1 | 4 |
| Original-image fallbacks after fixes | 0 | 0 |
| Expected English-frame retries | 0 | 2 |
| Remaining ink pixels in annotated regions | 0 | 0 |
| Changed pixels in sampled artwork interior | 0 | 0 |
| Text overflow or browser errors | 0 | 0 |

Spanish Lightning Bolt and Swamp are explicit Scryfall placeholder fixtures.
They now retry using the English scan while retaining Spanish live text.
The other six Spanish fixtures use available lower-resolution Spanish scans.

The benchmark exposed and drove reusable fixes for joined-word measurement,
dash and punctuation recognition, decorative elements falsely identified as
residual text, mask expansion beyond unregistered set symbols, the English
retry image URL, missing Spanish type translations, and font-fitting
transitions under reduced motion. No card-name-specific masking exceptions
were added.

Validation: all 16 benchmark cases passed; all type lines have positive visible
height. The 41 focused unit tests and three browser tests passed, including
normal and reduced-motion fitting. Targeted ESLint and whitespace checks passed.
All 16 final comparison images were visually reviewed.

Run from `web/ui` with `pnpm test:frame-benchmark`. Fixture maintenance and
measurement details are in
[the corpus README](../tests/fixtures/bilingual-frames/README.md).
The recorded run is available as a
[comparison gallery](../test-results/bilingual-final/index.html) and
[JSON measurements](../test-results/bilingual-final/results.json).

This is a bounded regression corpus, not proof of universal rendering support.
The checks do not establish exact typography, color matching or preservation
of every decorative pixel. Visual review still shows differences from print,
including Sigarda's English title color, text sizing and flavor separators;
those are not covered by the zero-residual-ink result. Other frame treatments,
layouts, languages and viewport sizes need their own fixtures.
