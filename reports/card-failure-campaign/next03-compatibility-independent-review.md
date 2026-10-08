# Final disposition: source-review CLEAR, UNVALIDATED / UNRUN

The final disposition below supersedes the chronological in-progress notes. Source anchor: `92168f50060c362c88d5c1ecf6477dbbffa11839`. No executable validation or measured recovery is claimed.

# NEXT03 independent compatibility source review

In progress. Review anchor: `92168f50060c362c88d5c1ecf6477dbbffa11839`, comparison base `94e075587`. Checkout was clean at entry. Scope: compiled artifact 14 / digest 9 / audit protocol 27 admission, historical signature preservation, native recovery claims, and authored boundary gates. No executable verification is authorized or performed; no production edits.

## Initial source findings

- The boundary diff changes artifact format/schema identity, retains digest 9, explicitly preserves historical protocol 26 signature support, and adds first-statement guards to direct match-start and state-resync callbacks.
- Artifact validation, JSON admission, engine materialization and registry registration are inspected separately; final conclusions follow below.
- Architecture candidly labels source-authored gates UNRUN and disclaims compiler-version authentication, consistent relabeling protection, signed engine identity and generated WASM provenance.

## Final result

**SOURCE-REVIEW CLEAR for this bounded compatibility/admission scope at `92168f50060c362c88d5c1ecf6477dbbffa11839`, compared with `94e075587a07b4ba659bcca84ded1d98605ccb82`. No blocking changed-source defect identified.** This is not compiler-cohort semantic clearance, a build/test pass, generated-artifact verification, deployment readiness, or a measured card-recovery claim. Source and executable validation remain separate.

### Admission and historical evidence

- `crates/ironsmith-compiled-artifact/src/lib.rs:345–381` checks format, schema and checksum in order; `from_json` invokes validation. Format 14 and the new declared schema are current; a refreshed checksum cannot rescue format 13 or the published v13 schema. Compiler version is not validated as provenance. The authored synthetic all-fields-rewritten case correctly exposes that a consistent relabeling is not detectable authentication.
- `crates/ironsmith-engine/src/artifact_materializer.rs:1886–1896` validates before decoding the definition; `crates/ironsmith-runtime-catalog/src/lib.rs:49–68` validates and materializes before registration. Raw `materialize_definition` has no envelope and is documented as trusted-model-only. No new stale-artifact fallback was introduced.
- `campaign_boundary_14_9_27.rs` independently compiles direct and artifact forms, captures strict parse loss, checks exact JSON roundtrip and actual materialization/registration, and asserts refusal leaves no entry installed. Its raw route uses a separately freshly compiled direct definition. Historical/synthetic provenance is not overstated.
- `web/ui/src/lib/multiplayer-audit.js:25–38,4019–4061,4390–4396` retains explicit protocol 26 signature-only support. Current replay checks strict numeric 27 on outer and nested owners before callback access, including a callback supplied with `requireEngineReplay: false`. Public digest stays 9. Canonical JSON, player/match genesis payloads, public checkpoint hashing and their domains have no changed implementation in this diff. No old Rust model reserialization or default insertion is introduced.
- Authored synthetic 26/9 signed tests preserve canonical transcript/genesis/checkpoint bytes, detect nested tampering and require genesis-hash rejection when both protocol owners are relabeled to 27. These are synthetic signed model records, not captured historical evidence.

### Current replay and peer entrypoints

- Existing `audit-replay.js:478,583,792,847` guards initialization, subsequent actions, final disclosures and full replay. It is unchanged in this delta, and imports the newly current protocol constant. The action path rechecks the retained transcript before accessing current checkpoint/game methods.
- `validation.js:4842–4849` makes the direct `applyMatchStart` strict version check its first executable statement, before runtime hints/engine access, independent of skip-genesis or accepted-genesis options.
- `messaging.js:305–316` checks outer resync and nested match before flags, session access or reset. All three transport callbacks retain strict outer version admission (`2272`, `2665`, `3055`). `shared.js:121` derives their version from the audit constant.
- Authored direct-callback tests include Trusted/Verified modes, old/missing/null/string versions, mixed owners, repeated mutated carriers, and no callback/reset/engine access on rejection. Transport and replay tests add protocol 26 and mixed-owner coverage. They are UNRUN.

### Native recovery boundaries and honest limitations

- No changes touch core persistent models, native WASM savepoint code, exact-image runtime, local recovery implementation or crypto-resync ownership. `ReferenceImports`/`ReferenceExports` announced-X tracking and `IfResultPredicate::DieValue` are compiler-only; they do not change the core wire/public projection schema. Cohort semantics remain separately reviewed.
- `exact-build-snapshot.js` still uses local image format 1, strict build/root/memory checks, layout/global/reference checks and function-table capture checks. Trusted in-session analysis has its explicit separate structural route. Compiler metadata cannot authorize an old image. These source checks are not proof that generated build identity/glue exists or is correct.
- `local-runtime-recovery.js` still filters by owning game, match, seat, signed audit anchor and prefix; recovery retries local anchors then genesis and verifies after replay. `crypto-resync.js:284–326` keeps integrity-protected local metadata, local-prefix fencing, exact-build refusal and local restoration. `messaging.js:745–838` retains signed-envelope/head consistency, suffix replay, post-restore public anchor verification, final public-head verification and rollback on failure. No incoming transcript is converted into permission for remote native-image installation.
- `matchGenesisPayload` still contains no signed engine-build hash. The architecture correctly describes runtimeVersion as an unsigned hint, and numeric protocol as a release gate rather than engine authentication.
- Only `fixtures/v3.json` exists under the compiled-artifact fixture directory; the authored v14 golden explicitly requires real later regeneration. `web/wasm_demo/pkg/engine.js` is absent. No real current artifact, historical capture, WASM glue identity or executable result was invented.

### Verification boundary

Read-only Git/source inspection and this review document were the only work performed. No builds, tests, compiler/parser/engine/browser/replay probes, formatters, corpus/analysis reruns, code generation, fixture generation or remote writes occurred. HEAD was rechecked at the same exact anchor at the end. The architecture's pending-final-review checkpoint metadata can be updated by its documentation owner only after all independent scope reviews reconcile; this report alone does not clear other cohorts.
