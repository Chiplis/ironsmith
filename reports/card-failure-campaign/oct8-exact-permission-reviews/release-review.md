# Independent coordinated release/admission source review

Reviewed exact commit a5878ad2f06b18d5ec0b146ddd7ccec1f09c0354, tree
20b43caeb664a103633343a0d8f9c37117d4b37a, relative to published-875 base
8ac862bff90d9ffbde89195187369d5ccf283a9e. Repository:
/workspace/scratch/bc560e8d90ff/ironsmith-oct8-exile-integrated.

## Decision

Bounded SOURCE CLEARANCE for the coordinated admission/projection/recovery-order
integration. No new production source blocker found within this review scope.
This is not build, test, generation, compatibility-execution, corpus, deployment,
or publication clearance. All executable gates remain UNRUN. No builds, tests,
probes, corpus, code generation, source edits or remote writes were performed.
Only read-only source/Git inspection and descriptor identity hashing were used;
this standalone review report is the only file written.

## Verified source owners

- Compiled-artifact lib.rs declares artifact17 and schema
  a4bb7964a2b6b477e4d128c655ca747453c74a9d5d1cf119ac7daf399fb840c0.
  The exact descriptor SHA-256 matches. Historical descriptor16 retains
  51232067d473dfff42849248b163193463ac225d2861f3bfe693e8d586b8f3b6.
  Git diff shows no edits to earlier descriptors, existing artifact fixtures,
  original cohort fixtures or frozen counter inputs.
- Actual Rust/WASM public_audit.rs declares digest11. Its sync_restricted_mana
  maps nested runtime effects through encode_runtime_effect into WireEffect.
  The new optional typed counter rider is therefore public JSON vocabulary,
  even without a new outer checkpoint field. Known-object identity emits
  compiled_card_text as oracleText. The new authored projection tests inspect
  all gate/domain combinations, canonical text and the fresh constructor.
- Actual multiplayer-audit.js declares signed audit31 and checkpoint11;
  peer-lobby shared imports the current audit owner. Historical30 is retained
  in signature-verification support. Manabrew vendor Cargo version remains
  3.0.0, its protocol constant derives the major, and its owner is unchanged.

## Admission, isolation and provenance limits

CompiledCardArtifact::validate still rejects wrong format, then schema, then
checksum. from_json invokes it. Runtime catalog registration validates before
materialization or registration; WASM batch materialization validates before
producing definitions and registering. No refused-envelope payload fallback was
introduced. Missing/invalid gate, absent or invalid boolean and unknown nested
rider fields fail typed decoding; decoder and graph traversal use the shared
validated_counter_effect predicate. The shared target predicate rejects
contextual, tagged, plural and non-spell shapes for marked riders.

The campaign boundary suite separately invokes direct compile and artifact
compile on full frozen raw/normalized bodies, checks each route's own roundtrip,
and never passes a refused artifact payload as direct source. CardId differences
are explicitly acknowledged. Stale16 occupied-registry retries compare the whole
prior encoded definition; permissive/strict malformed requests are followed by
fresh clean-source requests on both compiler routes. These are authored gates,
not observed passes.

Both append-conflict tests survive in counter_exile_permission_artifacts.rs:
direct_model_interpretation_rejects_contextual_counter_targets and
consistent_whole_rider_removal_or_relabeling_is_not_provenance_authentication.
The latter correctly preserves the limit: consistent unsigned rider removal or
valid gate/domain relabeling is another well-formed program, not intrinsically
malformed. Numeric versions, schema/checksum and compiler/runtime hints are not
producer authentication. Full-source semantic evidence and trusted generation
remain necessary.

## Historical signatures and recovery ordering

verifyLiveAuditTranscript enters current protocol/checkpoint checks whenever
requireEngineReplay is true OR a replay callback exists. A callback cannot bypass
those checks with requireEngineReplay:false. Historical30/digest10 remains
signature-only with original canonical bytes. Added signed synthetic controls
assert no replay callback, tamper refusal and relabeling failure; the canonical
hash control independently fixes the historical digest10 JSON string. These are
synthetic authored controls, not captured historical match evidence.

Direct applyMatchStart checks strict protocol before runtime hints or engine
access. applyStateResync checks both outer and nested protocol before flags,
resets or engine access. Both production files are unchanged from the review
base. The problematic newly added pre-resync/direct-start full-state exports
are absent. Existing replay start still exports the supplied game before
startMatch, and per-action replay rechecks current protocol/checkpoint. This
pre-existing viable-engine requirement is candidly documented, not solved by
this change. Historical/missing/string/future and mixed-owner refusal controls
retain positive source controls.

Signed genesis/envelope, local accepted prefix, suffix sequence/prefix,
match/seat anchors, final accepted-head and exact-image admission owners were
not weakened by this diff. exact-build-snapshot.js remains unchanged: version1
is not cross-build compatibility; buildId, root/memory shape, integrity,
globals/reference layout and function-table ownership remain necessary.
Remote signed transcripts do not authorize foreign native memory images.

## Native and generated-build implications

Effect::clone retains executor/model ownership via Arcs; CounterEffect's new
field participates in its typed Clone. ReplayCheckpoint and
LivePriorityContinuation retain native clone owners. RuntimeSavepoint captures
game/priority/pending replay/live continuation and runner/branch state, retaining
its existing restore/exchange semantics and monotonic shared card-ID treatment.
These facts do not establish suspended-payment, pending-answer, inactive-lane,
prepared-action, rollback or exact-image execution correctness. A changed native
struct requires genuine final-build/layout evidence despite serde defaults.

Generator source alignment is integrated, including the same CounterEffect
validation helper and decode/map validation arms, retained graph/facade owners
and external counter test inclusion. The independent generator component report
and source-gate scope distinguish source parity from actual generator output.
Genuine generation/idempotence remains UNRUN; no retained Rust is accepted as
newly generated evidence.

## Anchors, inherited limitations and mandatory holds

The compatibility document's bcc1bb0896bea0d0c30f6d97660c62784e4085aa/tree
474580a30b9749d386c61f19761e5ccf445518d6 source anchor is accurate. Only the
compatibility document and grammar/artifact review record differ from that
anchor to reviewed a5878ad. Component anchors and earlier conditional reports
are not substituted for this cumulative exact-anchor review. The documentation
properly qualifies historical generator-prerequisite reports as closed only at
source-template level. Frozen original inputs are unchanged. No support counts
or measured recovery credit increase.

Confirmed inherited evidence blockers: compiled-artifact/fixtures currently
contains only v3.json. Real current v17.json must be genuinely generated; the
required original v5.json remains missing and must not be fabricated or relabeled.
Historical16 and other historical bytes are not current golden evidence.
The stale comment mentioning v16 in the current-golden test is non-operative
wording; its actual path, expected version and failure message require v17.
Some inherited test names retain v16 wording while asserting current17; this does
not change their execution semantics or create evidence of execution.

Before release claims: separately authorize and execute genuine generation,
compiler/engine/verifier/WASM/glue/catalog/build/layout provisioning; provision
real current and historical prerequisites; run focused and aggregate admission,
codec, cache, full-card, native/continuation/payment, projection, signature,
recovery and exact-image gates on the final integrated SHA; then run exact-ID
corpus recovery and report actual results. If regeneration changes source,
review that diff and rerun dependent gates. None of this has been performed here.
