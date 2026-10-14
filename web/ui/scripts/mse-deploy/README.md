# MSE assets on the personal Netlify website

`pnpm build` emits a small, self-contained Netlify plugin in `dist/deploy/mse/`.
It does not download or bundle the corpus locally. Production defaults to the
same-origin `./mse/<pinned revision>/` URL; `VITE_MSE_CORPUS_URL` overrides it
(and an explicitly empty value disables MSE). Development remains opt-in.

The personal website's `netlify.toml` registers `./public/ironsmith/deploy/mse`
with `public` as the publish directory. Keep the existing build, rsync, commit,
and push workflow. The next rsync supplies the plugin referenced by that config.
Commit the website's new `netlify.toml` and `.gitignore` together with that output.
No Netlify credentials or additional npm dependencies are needed by the plugin.

Before publication, the plugin restores Netlify's build cache, verifies the pinned
corpus, and installs assets into `public/ironsmith/mse/<revision>/`. A cold build
fetches the GitHub tree and one archive, verifies each published file's Git blob
hash, and caches it for subsequent builds. Cache misses and corruption trigger a
fresh archive download. Download/validation failure stops deployment, preserving
the previous successful deploy. The first build requires network access, tar,
and space for both archive extraction and published assets.

All template families and shared packages are retained. Desktop dictionaries,
updater packages, executables and Photoshop source files are excluded. The complete
upstream index remains available for the runtime's revision checks. The installer
also generates `runtime-index.json` with font family metadata and one text bundle
per package under `packages/`. The browser downloads only requested font families
and reads scripts from package bundles, avoiding hundreds of sequential requests.
Older development corpora without this optional index still work through the
original individual-file path. This does not establish compatibility with every style.

The cache and generated corpus are ignored in the website Git repository.
`rsync --delete` is safe because Netlify reconstructs the corpus after checkout.
For a plain local production preview, configure a hosted corpus or invoke
`installCorpus` from `install.mjs` with `destination`, `cacheRoot`, and the pinned
lock. Without assets the UI retains its simple card fallback.

Test with `node --test tests/mse-deployment.test.mjs` from `web/ui`.

Real browser check: `MSE_TEST_CORPUS=/path/to/installed/corpus node tests/mse-deployed-card.browser.mjs`.
For a partial local corpus, `MSE_ASSET_ORIGIN` may point to a deployed corpus URL
(with a trailing slash) for missing images. This check renders Delighted Halfling
with the real runtime and verifies direct mouse and keyboard ability activation.
