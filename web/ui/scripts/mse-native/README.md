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
CoreText for the current process. wxWidgets' macOS AddPrivateFont implementation
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
