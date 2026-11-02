# MSEM / Revolution main checkpoint

The user requested merging the current work into main before quota exhaustion on 2026-11-02, explicitly deferring remaining issues. This is a work-in-progress checkpoint, not a claim that every custom mechanic works.

## Integrated work

The authoritative applied-patch ledger is wave15-integrated-patches.json (historical filename). SIXTH and the final ready SEVENTH batch extend the previous five coherent batches. Existing workspace work was already on main; no remote push is implied.

Worker patch recovery is worker-patches.tar.gz. Paths inside match their original workspace paths. These include superseded and unapplied candidates: never apply the archive wholesale. Consult the applied ledger and exact hashes first. Unfinished Dewshaper/COST work is preserved separately with its dependency and policy notes.

## Verification at the cutoff

- Original bounded baseline: 223 targets, 4,751 test executions, 4,320 passed, 409 failures, 22 ignored. Later repairs require reruns; baseline failures are not automatically cleared by integration.
- Native runtime library check passed before the final batches.
- Full source preparation: 4,610/4,610 records materialized, with two explicit unresolved source ambiguities. Subsequent targeted preparations passed 356/356 and 364/364.
- Frontend union: 161/162 passed, then the remaining stale Attain display assertion was fixed and its focused rerun passed.
- Eight selected unit packages compiled successfully before SIXTH. Selected checks then had 341 passes and 30 failures, plus inventory problems. Compiler/decoder follow-up checks added ten passes and two failures; runtime unit check passed. All 219 core tests with serde passed. Full syntax control: 29 passed.
- Failed unit logs and exact results are retained here. Final ready patches address many of these but remain unverified until rerun.
- Two required lowering tests were found in an unwired legacy file; final ready patch relocates only those tests into a live module.
- Eight native session filters belong to ironsmith-web-session, not the ironsmith-wasm compatibility facade. Their build was stopped at the user's cutoff; they remain unexecuted.
- Final schema refresh 8dad79b7 applied with hash 3535ad1d61837fa1fb4977a9cd4a556c4fa93d126829cfc5282675e18259ffcd. Current golden regenerated; final artifact suite passed 31/31 (final-artifact-tests.log). Historical goldens retained.

## Resume priorities

1. Use selected-unit-plan.json with exact package ownership; require nonzero matches. Rerun final repaired tests, including the newly wired lowering tests and compiler-wasm checks.
2. Two diagnostic-only engine failures remain: replacement-child reveal ZoneChange observation, and described entry projected characteristics. Read their new diagnostics before changing semantics.
3. Six parser repairs were not ready at cutoff: characteristic-gain distinct event unions; timed unpreventability falling through static admission; named-payment label apostrophe normalization; derived flashback companion ownership; complete Foretell/Plot token capitalization; unsupported mixed player/creature grant falling through tolerant suffix parsing.
4. Finish Dewshaper and counter-cost producer provenance. Preserve original Cost cause separately from selected producer receipt. Matching requires controller and qualifying class on the same causal Effect atom; this is a disclosed inference, not a custom-author ruling. Do not treat frozen partial patches as a complete implementation.
5. Refresh generated suites and current schema bindings after source changes; regenerate only the current golden, retaining historical fixtures. Rerun native source/gameplay targets (300 at SIXTH) with the bounded runner and root-owned target directory /tmp/ironsmith-goal-native.
6. Close remaining baseline failures and scope inventory gaps before claiming completion, including Wymi copy characteristics/Icingdeath, variable-resource X route, and source-proof coverage. The scope audit records additional finite gaps.

## Scope decisions retained

Official keyword collisions remain forbidden. Function, Lucid and Chant casting rules that move cards before targets are chosen remain excluded. Tunu's unspecified initial conjure destination and Glitch's unspecified counter quantity remain unresolved by explicit user instruction. Preserve all other recorded intrinsic/new-game-rule exclusions; do not silently exclude ordinary supported-rule mechanics.

Root owns builds/tests/materialization; workers author scratch patches and tests. No fresh tests should be called passed merely because the patch applies.
