# Bundled custom sets

The shipped catalog contains the audited MSEM and Revolution source snapshots (23,704 physical cards in 112 sets). The generated set files are lazy-loaded into the ordinary Sets workspace, which saves edits as browser-local overrides. Shipped files are never mutated by a browser edit. The custom-card search uses a small name index and fetches only matching sets.

Sources:
- MSEM: https://lackeybot.com/msem/search
- Current MSEM image routing: https://lackeybot.com/scripts/arcana_web.js
- Newer MSEM source prints: https://github.com/cajunwritescode/MSEM/tree/main/img
- MSEM source prints: https://mse-modern.com/msem2/images/
- Revolution: https://lackeybot.com/rev/search
- Revolution print catalog and images: https://escapefromlimbo.github.io/lists/all-cards.json
- Approved IronSmith rewrites: the source-specific fixtures selected by `fixtures/custom_mechanics/custom-source-endgate.json`.

Frozen source and image-index snapshots are included in `sources/`. Original printed rules remain on each face. Card and mechanic attribution records carry format, original set/name/code, collector number, credited artist, designer, source key, and source link. The VLR #126 filename casing is corrected against its repository listing. For Revolution's current image catalog, the image's artist credit is used and any snapshot credit is retained as `originalArtist`. Missing source credits remain empty; none are invented.

Rebuild from the repository root:

```
node scripts/custom-showcase/build-bundled-sets.mjs
python3 scripts/custom-showcase/cache-images.py
(cd web/ui && node scripts/build-custom-showcase-pool.mjs)
```

The cache requires Pillow. It stores complete source prints as WebP and a standard art-window derivative for compact views. It retains the source image URLs and writes an explicit failure report; failed sources must be resolved before treating the image bundle as complete. Existing cached files are reused. The original source images retain the printed credits.

The editable oracle text uses the approved custom definitions, including canonical aliases for collisions with official keywords. Variants with the same keyword remain card-scoped. The Sets mechanic editor updates only cards using the selected variant. Full printed source text is retained separately; an accepted rewrite is not treated as an unsupported mechanic merely because its wording differs.

`?showcase=custom` prepares two seats, seven random custom cards in each hand, and thirty random custom cards in each library. Both players receive Omniscience and IronSmith Mana Fixer (`{0}: Add {W}{U}{B}{R}{G}.`, i.e. `{WUBRG}`). All random cards come from the bundled sets. The build-time showcase pool selects up to three distinct, compilable single-face cards per set, prioritizing custom mechanics. The native compiler checks the approved expanded inputs during pool generation. Opening the demo prefetches this compact pool alongside WASM, samples without replacement from both sources, and places the complete deal in one transactional worker call. The pool includes typed compiled artifacts checked after JSON serialization. Every showcase entry must register from its artifact without parsing. Entries whose artifact transport is unsupported are excluded from the demo pool and recorded in `showcase-build-report.json`; their complete editable source sets remain bundled. The native empty-table staging API defers legality until the complete deal is placed. No whole sets or editor previews are loaded on this startup path. Regenerate the pool after changing source sets or mechanic expansion. The frontend production prebuild rebakes the pool against the current WASM before bundling. To rebake explicitly after rebuilding WASM, run `(cd web/ui && node scripts/bake-custom-showcase-artifacts.mjs)` to refresh artifact schema identities; stale or missing artifacts fail clearly and roll back instead of invoking a runtime compiler. Every imported card remains available in the Sets editor and custom search. Unresolved source ambiguities and explicit source exclusions stay out of the random deal. Setup restores its prior savepoint if insertion fails. This preset is a local sandbox lobby; it does not publish a multiplayer room.

Successful compilation is a startup gate, not a claim that the remaining mechanics in the earlier integration handoff have all passed gameplay tests. Those issues remain recorded in `reports/custom-mechanics/main-checkpoint/README.md`.

Profile a production build from `web/ui` with `node scripts/profile-custom-showcase.mjs`. Each run uses a fresh browser context and measures the visible table after DOMContentLoaded, without waiting for unrelated images/fonts to finish loading. Worker phase timings and main-thread CPU samples are included. Set `RUNS=5` for more samples.
