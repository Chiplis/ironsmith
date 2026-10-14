# Native MSE reference build

This is a validation tool, not the application renderer. Build the pinned Cajun
MSE source identified by the fixture manifest. On macOS the wrapper requires
CMake, wxWidgets (wx-config), Boost regex, pkg-config, and Hunspell.

```sh
node web/ui/scripts/mse-native/patch-macos.mjs "$MSE_SOURCE"
cmake -S web/ui/scripts/mse-native -B "$MSE_BUILD" -DMSE_SOURCE="$MSE_SOURCE" -DCMAKE_BUILD_TYPE=Release
cmake --build "$MSE_BUILD" --target magicseteditor -j 2
node web/ui/scripts/mse-native/prepare-resources.mjs "$MSE_BUILD" "$MSE_SOURCE" "$MSE_CORPUS"
node web/ui/scripts/mse-write-native-fixtures.mjs "$MSE_CORPUS" "$MSE_FIXTURES"
```

The macOS patches guard an unavailable updater API, fix a symbol-editor error
formatter, send CLI diagnostics to stderr, and register package fonts with
CoreText for the current process, including uppercase `.TTF`/`.OTF` filenames. wxWidgets' macOS AddPrivateFont implementation
only validates bundle paths; it does not register fonts. No fonts are installed
into the user's system. Layout and drawing algorithms remain unchanged. The
patcher records complete original/patched file hashes and is idempotent.

The exporter calls MSE's script API `write_image_file` through the raw CLI with
an explicit 100% zoom and `use_user_settings:false`. It does not alter saved
export settings. Keep the CLI protocol in the logs: a zero process exit alone
does not prove that a script succeeded. The exporter checks response statuses.

```sh
node web/ui/scripts/mse-native/export-fixtures.mjs "$MSE_BUILD/magicseteditor" "$MSE_FIXTURES/manifest.json" "$MSE_OUTPUT" "$MSE_SOURCE"
```

The exporter checks the source revision and patched-file hashes, removes stale
output images, records executable/input/image hashes and diagnostics, and checks
PNG dimensions. Any failed export, incorrect dimensions, or diagnostics produces
a failing exit status. A source/build match beyond those recorded hashes still
requires building the executable with the commands above.

Successful exports do not prove browser parity. Native macOS MSE uses point-sized
fonts without the 96-DPI normalization applied by its Windows/Linux branches.
The browser implementation currently follows that 96-DPI path. Preserve this
platform distinction when evaluating pixel differences; do not adjust native
layout algorithms to make screenshots agree. Fixture serialization currently
covers diagnostic scalar fields and styling, not general MSE set round-tripping.

Color parsing can be checked independently of card layout:

```sh
node web/ui/scripts/mse-native/check-colors.mjs "$MSE_BUILD/magicseteditor" "$MSE_OUTPUT/colors.json"
```

This compares every supported wx color name in lowercase and uppercase, plus
fully specified numeric and hexadecimal cases, against the actual executable.
MSE's lowercase `gold`, `silver`, and `transparent` overrides are case-sensitive.
The pinned wxWidgets 3.3.x default uses its CSS color scheme; legacy names with
spaces are separate entries. Incomplete `scanf` channel lists are excluded
because native MSE can read uninitialized channels for those malformed inputs.

For comparisons against the macOS executable, use the browser probe or populated
regression runner with `--font-dpi=72`. Both reports record this setting. Omitting
it retains the production 96-DPI default used by MSE's Windows/Linux branches.
This selects native point conversion, fourfold font-size/extent quantization,
and hhea ascent/descent/line-gap metrics for registered font files. Native-mode
text also uses a fourfold glyph mask, integer area downsampling, and quarter-pixel
placement before compositing; this applies to ordinary text as well as effects. Local fonts
without file metadata retain browser bounding-box metrics. This does not
guarantee identical rasterization or layout across every font and script. The browser font provider also accepts
`pixelsPerPoint` so hosts can select the relevant platform explicitly.

The populated fixtures include both selected faces of transforming and modal
DFCs. Their native sets contain two cards with reciprocal UID links and separate
stylesheets. The exporter uses each manifest entry's `cardIndex` (default zero).
The browser regression uses the renderer's engine-snapshot preparation path for
these fixtures, so it exercises linked-face initialization as well as drawing.
Automatic DFC planeswalkers use the mainframe style because the older
planeswalker style does not render transformation symbols.

For pack-wide browser checks, `mse-browser-audit.mjs` accepts `--card=CONTEXT.json`
and `--font-dpi=72|96` alongside `--ordered-fields`. It snapshots the renderer and
probe, records a SHA-256 of the supplied context, and counts field-script error
diagnostics as failures even if drawing completes. A populated context broadens
coverage beyond empty defaults, but one context does not cover every layout or
styling option. Native geometry expectations in the populated regression suite
are enforced only with the matching 72-DPI font profile.

Add `--session` to the pack audit to exercise browser-session initialization and
host system-font lookup, as used by the app. The default standalone probe uses
only resource fonts. Audit summaries record `browserSession`, and failure-only
rechecks record the baseline's mode separately; passing on a host with a font
installed does not establish availability on other hosts. Failure-only rechecks
also do not revalidate the baseline's previously passing styles.

Font resource discovery follows `FontRef::PreloadResourceFonts`: top-level
folders whose names contain `fonts` are searched recursively for TTF/OTF files.
The browser session and standalone template probe share this policy. Package-local
font copies are not registered over the installed resource fonts; some copies in
the pinned pack differ from their corresponding `Magic - Fonts` files. Probe
reports include the font-source policy, resource-file count, and whether host
system-font lookup was enabled. System-font availability is host-dependent.

The separate `mse-audit-fonts.mjs` file audit deliberately checks every indexed
font, including package copies, and reports native-resource counts separately.
Successful loading of every file does not establish that every family requested
by templates is present, or that text rendering matches native MSE.

When system-font lookup is enabled, the browser requests the installed regular,
bold, italic, or bold-italic face separately. A missing variant can use an already
loaded face of the same family for browser synthesis; an absent family remains
an explicit error. `node web/ui/tests/mse-local-font-faces.browser.mjs` checks
all four Verdana faces against direct local-font references, comparing both
metrics and glyph pixels. It requires Verdana to be installed on the test host
and does not establish native MSE layout parity.

The `alignment-short` and `alignment-wrapped` regression fixtures exercise
Big Text's scripted `short text only` alignment. Their 72-DPI expectations
include native alignment, content dimensions, and every line's top, height, and
width. The source queries and native set contents are recorded in
`reports/mse/scripted-alignment-native/reference.json`. Scripted alignment and
direction refresh against freshly wrapped lines during native MSE's bounded
vertical-offset iteration, before the later content-style update. These checks
establish those cases' layout behavior, not complete pixel parity.

Legacy unversioned vector symbols use native `SymbolShape::after_reading`
normalization: enforce handle constraints, measure each shape's curve bounds,
and divide positions and handles by 500 only when both maximum coordinates
are at least 100. Already-normalized shapes remain unchanged, including when
mixed with legacy shapes in a group. Versioned modern symbols retain their
coordinates.

`node web/ui/tests/mse-legacy-symbols.browser.mjs` checks pixel equality between
paired legacy and modern square, curved, and grouped symbols through both
package and local-symbol loaders. The shared sources are in
`web/ui/tests/fixtures/mse-legacy-symbols.mjs`. Native exports of the same
sources, CLI queries, set contents, executable hash, and output hashes are in
`reports/mse/legacy-symbols-native/reference.json`. Each renderer independently
produces matching legacy/modern output; this does not establish cross-renderer
pixel parity.

`node web/ui/tests/mse-info-field.browser.mjs` exercises a scripted `info` field
through the full card renderer. Info fields use plain, unwrapped text, caption
fallbacks, a background extending two units beyond the field, and a padded
alignment rectangle. Their native viewer ignores text shadow/stroke settings.
The isolated native sources and export query are recorded in
`reports/mse/info-field-native.json`; `info-field-comparison.json` records the
browser/native image comparison. This test requires installed Verdana and does
not establish exact glyph-raster parity.

`reports/mse/primary-slider-native.json` records native `primary_choice` results
for numeric, grouped, maximum, and unmatched slider values. Sliders inherit the
native choice implementation. The pinned native function returns boolean `true`
for an unmatched value because its final string literal selects `to_script(bool)`;
the compatibility runtime preserves that observable result. Matching unnamed
groups still return their empty string name.

Numeric/string `format` and formatted `to_string` share printf-style width,
precision, sign, zero-padding, alternate-base, fixed, exponential, and general
format handling. Decimal rounding uses the exact binary64 value with ties to
even, matching the pinned native reference rather than JavaScript `toFixed`.
`web/ui/tests/fixtures/mse-format-cases.mjs` contains 42 native expectations;
the CLI expressions, responses, and comparison are recorded in
`reports/mse/format-native-reference.json`. Date formatting remains separate.
Formatting rejects widths over 1,000,000 and precisions over 10,000 before
allocating output.

Plain-text conversion distinguishes `to_text` from `remove_tags`: the former
removes soft-separator contents and decodes escaped angle brackets and MSE
connection spaces, while the latter only removes markup. Fourteen native cases
cover nested separators, unfinished tags, and escape preservation in
`web/ui/tests/fixtures/mse-plain-text-cases.mjs`.

Run the numeric/string and plain-text fixtures against both runtimes with:

```sh
node web/ui/scripts/mse-native/check-script-fixtures.mjs NATIVE_EXECUTABLE NATIVE_DIRECTORY_SET OUTPUT.json
```

The command requires an existing directory set that the native executable can
open. It checks the native CLI protocol and recorded expectations as well as
JavaScript results, and records executable/set hashes. The comparison covers
these script cases only, not full scripting compatibility or rendering parity.
