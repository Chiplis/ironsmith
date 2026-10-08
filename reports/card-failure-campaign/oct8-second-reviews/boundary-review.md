# Independent source review: second Oct8 admission boundary

Reviewed commit: d1e0e1f49a3ab562b6ec445ed815eae6e54b1fdc
Baseline: d65bd6564569a38132ae107cbe209af81c87f4f6
Checkout: /workspace/scratch/bc560e8d90ff/ironsmith-oct8-second-boundary
Review date: 2026-10-08 UTC

## Disposition

No new production or test-source blocker found in the boundary diff. Source-review acceptance is conditional on integration of the separately reviewed repair candidates and original fixtures, resolution of their outstanding semantic/test findings, and the explicitly deferred executable/generation gates. This is not standalone build, runtime, recovery, deployment, or whole-card approval. All executable gates remain UNRUN.

I read the actual admission implementations, compiler routes/cache ownership, historical-signature verifier, direct peer entry points, replay entry points, and authored tests. I used read-only Git/source inspection and descriptor identity hashing only. No builds, tests, probes, corpus runs, code generation, source edits, or remote writes occurred. This report is the only deliverable written.

## Release identities and preservation

- FORMAT_VERSION is 16; ENGINE_SCHEMA_HASH is 51232067d473dfff42849248b163193463ac225d2861f3bfe693e8d586b8f3b6 (compiled-artifact/src/lib.rs:16-18).
- SHA-256 of architecture/oct8-second-cardrepair-schema.descriptor matches that declaration. Its final byte is not a newline.
- Artifact15 descriptor SHA-256 remains ce114b87adedd56e98d8472b5d4e3db436ab9408ff2dc3176a286c32132462b7. Baseline and reviewed commits both reference Git blob fe9b9a9db0f2c2b3ea2a1389c06630ac8c91eca7 for architecture/oct8-cardrepair-schema.descriptor, proving unchanged tracked bytes. Its final byte is not a newline. A new authored preservation test checks the exact predecessor hash.
- CURRENT_AUDIT_PROTOCOL_VERSION is 30; historical29 is added to the supported signature-verification set. CURRENT_PUBLIC_AUDIT_CHECKPOINT_VERSION remains 10.
- Manabrew remains 3, with its protocol source unchanged between the two commits. No model, enum, native-layout, public-projection, or payment owner is changed by this boundary diff.
- No historical golden, descriptor, catalog, or transcript is renamed/migrated/relabelled by the production changes. Synthetic relabelled test objects are explicitly identified as negative/admission-limit tests.

## Actual artifact admission

CompiledCardArtifact::validate (compiled-artifact/src/lib.rs:345) checks format, schema, then checksum. from_json (:376) deserializes then calls validate. Recomputing a checksum cannot evade either preceding gate.

materialize_artifact (engine/src/artifact_materializer.rs:1886, re-exported by runtime-catalog) validates before cloning/materializing the payload. CardRegistryArtifactExt::register_compiled_artifact (runtime-catalog/src/lib.rs:49) validates before materialization, checks the card-name match, then registers. Rejection cannot replace an existing registry entry through these paths. materialize_definition remains an explicitly raw definition route; the new tests do not extract its input from a rejected envelope.

campaign_boundary_14_9_27.rs:61-117 authors the complete negative matrix: stale11/12/13/14/15, current format with predecessor15 and other old schema hashes, and altered canonical text without refreshed checksum. validate/from_json/materialization/registry are all exercised; materialization and registration error owners are matched precisely, and rejected names remain absent. The existing occupied-registry cycle (:211-242) now uses stale15 with the old15 schema and fresh checksum; two attempts must preserve the full encoded current definition.

The synthetic authentication-limit test (compiled-artifact/src/lib.rs:535) remains honest: changing format/schema without refreshing checksum fails, but consistently rewritten metadata/checksum and arbitrary compiler_version claims pass. The numeric release gate is neither a signature nor authenticated source/build provenance.

## Independent compilation, caches, and six-body fixture wiring

compile_source (campaign_boundary_14_9_27.rs:13-46) separately calls compile_to_runtime_definition and compile_to_artifact under strict parse-loss capture. The actual APIs each allocate a fresh CardId and enter the compiler facade (compiler-runtime/src/lib.rs:660 onward). The helper tests exact artifact typed/JSON roundtrips, materializes and registers the artifact, and independently encodes/decodes/materializes the direct fresh definition. It deliberately does not erase CardIds or claim cross-route structural equality. Semantic correctness belongs to the dedicated cohort suites.

Production grammar recognition resets sentence_memo at document boundaries (compiler-grammar/src/document_parser/mod.rs:3703,5350). Memo hits replay both parse-loss and minted-reference evidence (sentence_memo.rs:105 onward). The old engine builder ParseCacheKey/backend is cfg(ironsmith_runtime_parser_tests)-gated and includes builder context, source text, and allow_unsupported. The boundary adds no fictitious production cache or cache migration.

The new second-cohort same-name cycle (:271 onward) makes permissive then strict calls for unsupported subjects/counts through both public routes, requires error or loss, then invokes clean fresh compilation and refusal matrices twice. Permissive calls are intentionally not asserted successful: this is isolation coverage, not support credit.

The full-body test (:249 onward) correctly refers to these candidate fixtures and shapes, verified by reading those Git objects:

1. 7e4bc0c: fixtures/source_must_be_blocked.json.fixture is an array containing Anzrag, the Quake-Mole; Glorfindel, Dauntless Rescuer; Loathsome Catoblepas. Their complete source can be reconstructed from mana_cost, type_line, power/toughness, oracle_text by source().
2. c33c973f: fixtures/temporary_additional_land_caps.json.fixture is an array containing Summer Bloom and Journey of Discovery. Both have complete text fields; Journey includes search/reveal/hand/shuffle, the additional-land mode, and Entwine.
3. d3380a3c (inheriting 5d87bb62): fixtures/titania_alternative_cost.json.fixture is an object with a card member. The boundary selects that member, asserts Titania, Rugged Rumbler, and reconstructs full mana/type/PT/oracle source including both additional casting cost and Ward. The fixture's separately stored top-level text is not required because all source fields exist in card.

These fixtures intentionally are absent from this isolated boundary checkout. Their include_str! dependencies are known integration prerequisites, not an observed compiler/runtime failure and not a reason to fabricate replacement bytes. The current v16 golden and original historical v5 evidence are also explicit provisioning conditions; the checkout currently retains v3 only. No execution was attempted.

## Historical29 signature integrity

verifyLiveAuditTranscript checks assertCurrentAuditReplayProtocol before any replay callback whenever requireEngineReplay is true OR a callback exists, even with requireEngineReplay:false (multiplayer-audit.js:4020 onward). Both transcript and match must strictly equal numeric30. Signature-only historical29 remains supported without a callback.

The historical29/digest10 test (:3592 onward) uses a helper that creates at least two player keys, player genesis signatures, and signed host genesis even when supplied players is empty. Thus these are genuine signatures over synthetic model-shaped JSON, not captured historical gameplay. It verifies valid signature-only output with engineReplay:null; historical replay requests and default replay refusal; nested checkpoint null-to-array mutation refusal; and both version owners relabelled to30 failing genesis payload hash validation before callback execution. It counts zero callbacks and rechecks canonical transcript, genesis, checkpoint bytes, and checkpoint hash. The original protocol28 test's data/hash semantics are retained; only current-version rejection expectations change. This does not prove build authentication or compatibility of old runtime semantics.

## Peer and replay matrices

- Shared peer PROTOCOL_VERSION aliases CURRENT_AUDIT_PROTOCOL_VERSION. All three transport callbacks strictly check it before dispatch (messaging.js:2272,2665,3055). The transport test adds29 across chat, match_start, state_resync, apply_action, signed recovery, and crypto-material messages, with current30 positive controls.
- applyMatchStart checks numeric version before runtime hint, engine, or session reads (validation.js:4842 onward). Direct callback tests cover Trusted/Verified, skipGenesisVerification, verifiedResyncReplay, missing carriers, and mutation after a current payload reached its next stub.
- applyStateResync checks both outer and nested match version before flags, resets, session, or engine access (messaging.js:305 onward). Tests cover historical29 mixed/both owners, absent/null/string/future values, both security modes, unchanged false flags on rejection, and current positive controls reaching session access.
- startAuditTranscriptReplayWithGame, replayAuditTranscriptWithGame, and verifyEndOfMatchDisclosuresWithGame all invoke the strict current gate before engine access (audit-replay.js:478,847,792). applyAuditReplayActionWithGame rechecks the initialized session's current transcript and clears failed sessions (:583).
- The replay matrix adds29 to old/mixed-owner cases, callback-with-replay-disabled cases, and mutated session actions. Existing digest10 and failed-initialization/action controls remain. These are source-authored assertions, not executed results.

## Conditions before final integration clearance

The architecture document correctly makes this boundary contingent on the repair stacks. The source-subject candidate still needs its pending raw-subject correction; temporary-land candidate still needs its reveal/shuffle follow-up; Ward d3380a3c still needs its follow-up independent review. The boundary document currently cites the Ward production parent 5d87bb62, while d3380a3c adds tests/report changes; final integrated provenance should name the accepted final heads for all cohorts after those follow-ups. That is a documentation/integration condition, not a newly found boundary logic defect.

After authorization and final integration, regenerate genuine current-source outputs/golden/catalog/fingerprints with build/layout evidence and execute the compiler/cohort/aggregate/artifact/protocol/peer/recovery/native/WASM/browser gates specified in the compatibility document. Do not infer recovery or readiness from this source review or from the older measured 1dd81cd refresh. Sinister remains outside this contract with its inherited test-only HOLD.

## Integrated closure review at 1a00afa3 (2026-10-08 UTC)

Exact integrated target: 1a00afa3838fcd6600db2abfc3bb3c16240c1a7c, inspected in /workspace/scratch/bc560e8d90ff/ironsmith-oct8-next-integrated with a clean working tree. This follow-up scope is boundary preservation, original fixture integration, descriptor identities, and documentary consistency. It does not substitute a new independent semantic review of the three repair stacks or any executable validation.

### Verified closure

- The boundary cherry-pick 110e27b07 and integrated target preserve every reviewed boundary production/test/descriptor file byte-for-byte from d1e0e1f. The explicit comparison covered compiled-artifact lib.rs, all three campaign boundary tests, multiplayer-audit.js, all four affected audit/peer test files, and oct8-second-cardrepair-schema.descriptor. Only the compatibility document has subsequent changes among boundary files.
- The original compatibility document at 110e27b07 is identical to d1e0e1f's document; final target 1a00afa3 changes only that document relative to 5becc5f98897bcc14662a1cfd404bda50bf3b494.
- The documented combined anchor 5becc5f98897bcc14662a1cfd404bda50bf3b494 resolves to exact tree 001ec6349d830d60f96f266ed339d37092f755f9 as claimed.
- The integrated source-block fixture is blob 86ef3d007b020dfdf84580a7966214c62e0c3ee0, identical to final author e46e0124a8ba6264179b2a5ba1de6f10750263fe.
- The integrated temporary-land fixture is blob bc93e7075b216c476e277ca9cf8c412df37db5b1, identical to final author 2f9b5c1cf0bcc44661d9d64e678c0e2c48bf1552.
- The integrated Titania fixture is blob a34bbf1e752c4ba9c36bc22f0d406c90658c421c, identical to final author d3380a3c23883188990600299076cdaa6915ad74.
- Accordingly the three missing-fixture integration prerequisites in the original isolated-boundary review are closed for this exact integrated target. The six-body test wiring reviewed above now has the exact original referenced fixture bytes on disk. This says nothing about whether those tests pass.
- Integrated artifact15 descriptor remains blob fe9b9a9db0f2c2b3ea2a1389c06630ac8c91eca7 / SHA-256 ce114b87adedd56e98d8472b5d4e3db436ab9408ff2dc3176a286c32132462b7. Integrated artifact16 descriptor SHA-256 remains 51232067d473dfff42849248b163193463ac225d2861f3bfe693e8d586b8f3b6. There is no boundary drift.
- The document now pins all three final author heads rather than the older incomplete candidates and preserves UNVALIDATED/all executable gates UNRUN, lack of authenticated build provenance, and lack of new measured recovery/support totals. The source-anchor and final-head corrections requested in the original review are present.

### Documentary issue found

architecture/cardrepair-oct8-second-compatibility.md:69-75 still says the three fixtures “intentionally do not exist on this isolated boundary branch” and describes source/fixture integration as an outstanding prerequisite. At 1a00afa3 that is stale and conflicts with both the newly documented integrated anchor and the actual fixture blobs. Update this paragraph to distinguish the original isolated d1e0e1f/110e27b07 boundary proposal from the now integrated source/fixture state, while retaining that executable/deployment clearance is absent. This is a documentation closure issue, not a production or runtime finding. It was reported promptly to the coordinator.

### Remaining limits

No new boundary production/test-source blocker was identified. This exact target has boundary/fixture integration closure with the one documentary correction above outstanding. The artifact directory still contains v3 only: genuinely generated current v16 and original historical v5 evidence remain deferred provisioning requirements. All builds, tests, probes, corpus runs, code generation, generated outputs, and remote writes remain UNRUN/unperformed in this review. The earlier pending component semantic findings are owned by the final component reports and were not independently re-adjudicated in this bounded closure pass.

## Narrow documentation closure at 041e1d0b (2026-10-08 UTC)

Confirmed exact target 041e1d0b1af01ad82612359d55e8614a97756926 in the integrated checkout, with clean working tree. The previously stale fixture-absence paragraph now states that all three original fixtures are present at the combined source anchor, their Git blobs match final author branches, and repairs plus dedicated semantic/runtime suites are integrated. It explicitly limits this to source integration, not a passing build or executable validation. This closes the sole documentation issue from my 1a00afa3 closure pass.

The diff from 1a00afa3838fcd6600db2abfc3bb3c16240c1a7c contains that compatibility-document edit plus seven report/evidence additions; there are no changes to executable boundary/source/test/fixture/descriptor files. Those additional report/evidence artifacts were outside this requested narrow closure scope and were not independently audited here. The previously reviewed boundary and fixture conclusions therefore carry forward without claiming review of the new evidence package.

Disposition: the boundary-preservation, fixture-identity, descriptor, and documentation closure scope is complete at this exact target. No execution or broader evidence audit performed. All executable gates remain UNRUN and the original deferred validation/provisioning limits remain in force.
