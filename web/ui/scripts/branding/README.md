# Font-aware forging wordmark

Generate a standalone SVG from an outline font. The final **h** and `IronSmit`
are extracted from the same font, with uniform scaling. No system font, browser
font loading, or font CDN is required to display the output.

From the repository root:

```sh
python3 -m venv .venv-branding
.venv-branding/bin/pip install -r web/ui/scripts/branding/requirements.txt
.venv-branding/bin/python web/ui/scripts/branding/generate.py \
  --font web/ui/src/assets/fonts/Plantin-Semibold.otf \
  --out output/branding/wordmark-plantin.svg
```

Open the SVG with `#working` appended to animate. Three strikes reshape the
anvil into the selected font's h, then hold the finished wordmark. Regenerate
when changing font; this is a build-time generator, not runtime CSS font detection.
The existing loading screen and favicon are not modified by this command.

TTF, OTF, and WOFF outline fonts are supported. For variable fonts, repeat
`--axis`, e.g. `--axis wght=700 --axis wdth=90`. Choose the actual bold/italic
font file for static faces. Bitmap-only/color-only glyphs without outlines,
missing required characters, and unknown variation axes are rejected.
Fonts with unusual ornamental h shapes may still need visual review.
Use fonts whose licensing permits outline embedding.

The generator preserves glyph counters and separate contours. It locates the
h's right shoulder, adapts the starting anvil, and scales/positions the smith
and sparks together so the strike reaches that shoulder while his feet stay
on the baseline. The mascot motion and expression come from
`wordmark-template.svg`. Reduced-motion users see the finished letter.

Run regression tests:

```sh
.venv-branding/bin/python -m unittest discover -s web/ui/scripts/branding -p 'test_*.py'
```

## Current loading-screen asset

The loading screen uses Matrix Bold. Regenerate the checked-in asset with:

```sh
.venv-branding/bin/python web/ui/scripts/branding/generate.py \
  --font web/ui/src/assets/fonts/Matrix-Bold.ttf \
  --loading-screen --out web/ui/public/theme/ironsmith-loading.svg
```

`--loading-screen` produces transparent artwork, crops excess canvas, and
removes the preview caption. Optional `--letter-spacing 0.12` adds tracking
in em. Both startup states use this asset with `#working`. The favicon remains
`ironsmith-logo.svg`, without the animation fragment.
