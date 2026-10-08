# Oct8 release admission: independent source review and integration closure

## Scope and result

Initial boundary reviewed: `1dde937384ddd7b048ffb0d394526837c5c8b595`, against main `1dd81cd84c62f272479f26e16d74719fff24b97b`.

Integration closure reviewed: `a5a608328efe69e7dac045a3830f55fc01b7d579` in the integrated worktree. Source anchor before documentation reconciliation: `2e0b88e63510f4cd71347dac522bca01ed245c61`, tree `148da7154b8cee72e59b0984498029d0c00b2538`.

Result: source-only clearance for the release compatibility/admission changes. No production admission blocker or authored boundary-test defect found. This does not establish executable correctness, compiler cohort semantic clearance, recovery measurements, generated-output provenance or deployment readiness. All executable checks remain **UNRUN / UNVALIDATED**.

## Full initial review

- Artifact15/schema `ce114b87adedd56e98d8472b5d4e3db436ab9408ff2dc3176a286c32132462b7` matches the descriptor bytes. Published artifact14 descriptor is unchanged and hashes to `292e6db310f90613f13024fb4d135e405483443b81ef85b38f04e6755f6fdd7f`.
- Digest10 remains in `web/ui/src/lib/multiplayer-audit.js:27` and `crates/ironsmith-wasm/src/wasm_game_impl/public_audit.rs:10`. Audit29 is declared at `multiplayer-audit.js:26`. Manabrew remains version3 through `vendor/manabrew-protocol/Cargo.toml` and its major-version constant.
- `CompiledCardArtifact::validate/from_json` (`crates/ironsmith-compiled-artifact/src/lib.rs:345–381`), `materialize_artifact` (`crates/ironsmith-engine/src/artifact_materializer.rs:1886`), and `register_compiled_artifact` (`crates/ironsmith-runtime-catalog/src/lib.rs:48`) reject incompatible format/schema/checksum before model installation. Existing registry names do not bypass admission.
- `crates/ironsmith-compiler-runtime/tests/campaign_boundary_14_9_27.rs:13` independently invokes direct and artifact compilers. Its raw `materialize_definition` input comes from a fresh direct definition, never a refused artifact. Exact comparisons are within-route, preserving independently allocated CardIds.
- Boundary tests at lines61 and210 cover refusal errors, no insertion, repeated stale registration preserving a current definition, and permissive/strict same-session alternation. Production sentence memoization resets at `document_parser/mod.rs:3703` and5350, with loss replay on hits. The legacy engine `ParseCacheKey` implementation is separately cfg-gated.
- Historical28 remains accepted for signature-only verification. `multiplayer-audit.js:32` and4031 require numeric29 on both owners whenever replay is requested, including `requireEngineReplay:false` with a callback. Signed genesis verification precedes callback invocation.
- The new historical28/digest10 test checks unchanged canonical evidence, nested checkpoint mutation, relabeling both owners, and zero callbacks. It identifies its evidence as signatures over synthetic models rather than captured historical games.
- `applyMatchStart` rejects before runtime hints/session/engine access (`validation.js:4842`). `applyStateResync` rejects an outer or nested mismatch before flags/resets (`messaging.js:305`). Transport callbacks at2272,2665 and3055 retain strict guards. Replay start, action, disclosures and full replay retain their protocol checks.
- Authored tests exercise historical28, mixed/missing/null/string/future versions and mutation between calls. Current positive controls proceed through admission to their intended next stub.
- No build authentication is claimed. Checksum-consistent relabeling remains explicitly possible in `compiled-artifact/src/lib.rs:527` and in the compatibility document.

Initial integration-only conditions were a missing separately supplied chosen fixture and stale untap/numeric author anchors in the standalone compatibility document. These were not production patch defects. Final integrated-source verification was required before closing them.

## Integration closure

- HEAD resolves exactly to `a5a608328efe69e7dac045a3830f55fc01b7d579`; the worktree was clean during review.
- The documented source anchor resolves to tree `148da7154b8cee72e59b0984498029d0c00b2538`, exactly as recorded. The only change after that anchor is the compatibility-document reconciliation.
- All boundary production/test files and descriptor bytes are unchanged from the initial reviewed boundary. The materializer, registry, replay, and peer-admission owners inspected in the initial review also have no integration drift.
- The document now names the correct final author anchors: lossy `39ad55d76d741a560d732de07536a11a74d5faa0`; chosen `0e11a69886f93516d16b5204175ce2cd7d95df67`; untap `f476216129bfa193f2a5af1a7d8362a1df07d6e4`; numeric `2e67a0c80e8b1c3d87b647ebc97fe4452cc0a835`.
- Chosen fixture is present with complete Arcane Adaptation, Leyline of Transformation and Rukarumel bodies and newline-terminated metadata. The Send to Sleep and Icy Blast rows are present in the untap fixture. The complete Loathsome Troll fixture is present separately; the boundary also retains its complete inline body.
- Chosen, untap and numeric fixture/test files compared against their supplied final author heads have no drift; the lossy probe-loss test file likewise has no drift. This bounded reconciliation does not replace their independent semantic reviews.
- The Sinister packet remains explicitly test-only HOLD, outside the semantic descriptor and without recovery credit. The preserved audit packet is identified as measuring the main baseline, not these repairs.
- Initial missing-fixture and stale-author-anchor conditions are closed. No remaining release-boundary source blocker was found.

## Deferred prerequisites and work performed

Genuine artifact15 golden and final-source compiler/engine/verifier/glue/WASM/catalog/fingerprint outputs remain deferred, as do all compiler, runtime, replay, recovery and browser validation. Missing original v5 evidence remains an explicit prerequisite. Publication still needs separate authorization.

Only source reading, Git identity/diff/status inspection, descriptor hashing and this report write were performed. No builds, tests, parser/compiler/engine/replay/browser probes, corpus runs, formatter, code generation, generated fixtures, source-code edits or remote writes were performed.
