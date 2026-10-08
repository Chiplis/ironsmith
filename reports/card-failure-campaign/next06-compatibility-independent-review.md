# NEXT06 independent compatibility source review

## Scope and disposition

Reviewed source anchor `1d4c5a6f4a89dbae2d0a28b81a4d95f9d5b178fc` against published parent `113e2e3a`. Read `architecture/card-failure-next-series-06-compatibility.md`, the native field/projection changes, compatibility delta `c02ebd3f`, and the admission/recovery owners and authored gates they depend on.

**Bounded production compatibility source review: CLEAR.** No changed-source runtime compatibility/admission blocker was identified. One authored-evidence qualification is recorded below and was reported to the integration owner for reconciliation. This is not a build/test pass, native card-semantics clearance, captured historical-match validation, generated-output verification, or deployment readiness. Every executable scenario is **UNRUN**.

## Authored-evidence qualification

At the exact reviewed anchor, `crates/ironsmith-compiler-runtime/tests/campaign_boundary_14_9_27.rs::compile_source` independently compiles direct and artifact paths, checks both parse-loss reports, round-trips the artifact JSON, materializes it, registers it, and separately materializes the freshly compiled direct definition. It does **not** compare the independently encoded direct definition against `artifact.payload.definition`. Its equality assertions prove artifact round-trip equality, not cross-route definition equality. The architecture inventory's “JSON equality” must be read narrowly or strengthened with a supported cross-route comparison, accounting for any independently allocated identities. No actual definition divergence was demonstrated. The integration owner is reconciling this qualification separately.

## Retained artifact and Manabrew boundary

- No compiled definition schema, compiler production implementation, artifact codec, core wire vocabulary, descriptor, runtime catalog implementation, or Manabrew protocol source changed against the published parent in this scope. The compiler-side repeated-static change is test-only.
- Artifact format remains 14 and schema remains `292e6db310f90613f13024fb4d135e405483443b81ef85b38f04e6755f6fdd7f`. The new `Option<BTreeSet<PlayerId>>` belongs to native `CombatState`, not the compiled definition payload. Exert selects existing `Until::PlayersNextUntapStep` / `PlayerFilter::Specific` shapes at runtime. Glittering changes are evidence-only.
- Artifact envelope validation still checks format, schema and checksum before materialization/registry admission. The authored native-fog gate calls the existing stale-format/schema/checksum rejection helper. Its direct route is independently compiled, never extracted from a refused envelope.
- Retention permits valid v14 definitions to be consumed by the corrected runtime; it does not authenticate the compiler/runtime or prove malicious consistent relabeling detectable. Runtime semantics changed, so matching current runtime outputs remain necessary.
- Manabrew's package remains 3.0.0 and its protocol major derives from that package version. No new input/output wire field is introduced there.

## Native owner and public projection

- `CombatState.last_attack_declaration_step_players` is optional ordered player evidence. Committed empty declarations become `Some(empty)`; uncommitted state is `None`. Committed surviving direct-player declarations union into the field after declaration costs/choices. Attacks at permanents and entering attacking are not converted into direct-player declaration facts.
- Both DeclareAttackers scheduler entries explicitly clear the field. Phase entry and combat termination also clear it; phase-wide melee history remains separate. The permission reader independently requires Combat/DeclareAttackers. Evidence remains owned by its combat after step exit rather than being labeled as arbitrary current-step evidence.
- `public_audit.rs` advances the genuine typed public checkpoint version from 9 to 10 and adds `lastAttackDeclarationStepPlayers` at the active checkpoint root and `SyncGrandMeleeCombat` lane owner. The shared projection iterates a BTreeSet, producing sorted u8 values. Option serialization emits explicit null versus an empty/nonempty list; no skip/default is added to the new field.
- Both public structs derive Serialize, not a new gameplay import path. No generic gameplay serializer/importer is introduced. Existing historical payloads never pass through these new types.
- Authored projection checks cover field presence, null/empty/sorted nonempty values in both owners, distinct representations, deterministic serialization and native clone restoration. Native savepoint evidence uses actual `RuntimeSavepoint::capture`, clone, exchange and restore plus pending decision game and `ReplayCheckpoint` roots. The APIs exist at the reviewed source; compilation and runtime success are unverified.

## Historical bytes and strict admission

- Current signed audit is 28; public digest is 10. Historical 27 remains explicitly supported for signature-only verification. The production multiplayer audit delta changes constants and supported-version membership only; canonical JSON, genesis/checkpoint hashing and hash domains are unchanged.
- `verifyLiveAuditTranscript` applies strict numeric outer/nested current protocol admission whenever replay is requested or a replay callback is supplied, including `requireEngineReplay: false` with a callback. Current final checkpoint admission precedes that callback. Historical signature-only records retain their original JSON; there is no default insertion, version rewrite or typed deserialize/reserialize migration.
- `audit-replay.js` rechecks strict current protocol on initialization, retained-session action application, full replay and disclosure replay. Invalid protocol fails before engine reads; invalid exported checkpoint versions fail after export and before engine start/action mutation. All imported constants now require 28/10.
- Direct `applyMatchStart` checks exact numeric current version as its first statement, before runtime hints, game/session access or skip-genesis branches. `applyStateResync` checks both outer and nested owners before flags/resets/session/engine access. Transport callbacks retain strict guards through the shared current constant.
- Synthetic signed 27/9 tests preserve canonical transcript/genesis/checkpoint bytes and hash, reject active/lane field injection and nested combat mutation, reject relabeling, and assert zero replay callbacks. These are authored genuine-signature constructions over synthetic JSON, not captured historical match evidence.
- The inherited current-export assertions that expected 8 despite parent constant 9 now follow current constant 10. Historical filenames and historical signed digest9 records remain unchanged.

## Recovery and exact-build limits

- Native `RuntimeSavepoint` and `ReplayCheckpoint` ownership continues to retain complete native game/continuation roots; the added field travels with the cloned native owner. Public digest JSON is evidence, not a substitute for executable rules state.
- Local recovery candidates remain tied to owning engine, match, seat, accepted audit anchor and prefix. Recovery restores a candidate, verifies its public signed anchor, replays suffix actions and verifies the accepted head; failed levels fall back through older local points to genesis, with failure refused rather than partial acceptance.
- Verified resync still authenticates signed action envelopes/prefixes and accepted match/head, rejects remote serialized engine payloads, and uses native local recovery or accepted-genesis replay. No new remote-image install route exists.
- Exact local images retain format1, build identity, root/memory checks, integrity checks, layout/global/reference-table checks and function-table provenance. A changed native CombatState layout requires the corresponding genuine build. Artifact14 retention and unsigned runtimeVersion do not authorize old-build images.
- Existing authored recovery/native/exact-build suites remain required. The changed WASM recovery gate now explicitly requires actual digest10 exports and the new field. No tracked declaration or synthetic fixture establishes genuine WASM/glue/build provenance.
- Missing original-v5 provisioning and genuine v14 golden limitations remain. Matching compiler/engine/verifier/glue/WASM generation, genuine artifact/catalog regeneration, executable validation and newly measured corpus results require a later authorized stage.

## Inspection boundary

Only Git/source reads and this review document were performed. No builds, tests, probes, formatters, corpus runs, code generation, generated fixtures, or remote writes occurred in this review. Checkout HEAD advanced during review to `b9a4ff81068b1c827dba90cc400610824b7fc308`; its difference from the reviewed anchor contains only the separate Glittering test/report follow-up. Reviewed compatibility production files still matched `1d4c5a6` at that inspection. No production edits or commit were made by this reviewer.

## Qualification resolved: exact follow-up `ed5de2351ec58e8e92c5d1158831c0596a44d3d1`

Independently inspected this two-file follow-up via Git source reads. **SOURCE-REVIEW CLEAR for the compatibility scope including this delta; no remaining review blocker.** It accurately narrows the inventory to independent routes plus within-route typed/JSON equality, and adds explicit typed and byte roundtrip assertions for the direct wire definition. The actual public compile calls each allocate `CardId::new()`; `wire_definition_from_serializable` and `encode_runtime_definition` retain complete definition identities. Therefore no unsupported blanket ID stripping or misleading cross-identity equality was introduced. Complete-body native semantic suites continue to exercise each separately compiled route. This closes the authored-evidence qualification above without asserting an unexecuted semantic result.

The added exert caveat correctly distinguishes typed duration payload evidence from latent restriction receipts and cutoff state omitted by public projection. Equal public digests do not establish exert tenure or engine equivalence. Native ownership, honest-peer protocol admission and exact-image build/layout constraints remain necessary; unsigned runtime version is still not authentication.

The follow-up also scopes its no-remote-write statement to work actually performed; publication authority remains a separate parent concern and was not exercised by this reviewer. The original integration checkout remained `b9a4ff81` when this separate commit was inspected, so the parent must integrate the exact reviewed follow-up before treating the qualification as resolved in its own final source. No builds/tests/probes/formatters/codegen/corpus/remote writes occurred. Every executable gate remains UNRUN.
