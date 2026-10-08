# Atomic counter exile permission: artifact and rendering source review

Status: **source-authored, UNVALIDATED; all executable gates UNRUN**.
Worktree base: `041e1d0b`. No builds, tests, compiler/parser/runtime probes,
formatters, corpus work, generator execution, generated evidence, publication,
schema/cache version changes, or release admission occurred in this work.
The frozen three-card inputs and original HOLD remain unchanged. Synthetic
controls below confer no measured recovery credit.

## Typed transport and rejection

- `ironsmith-core::CounterEffect` retains the existing kind and target field;
  the optional `exile_permission` is absent for legacy counters. The nested
  gate and play/cast boolean are required; unknown nested fields are rejected.
  Price, actor, lifetime and exact receipt ownership are intrinsic to the new
  atomic carrier. There is no independent output tag or permission consumer.
- `ironsmith-artifact-effect-decoder/src/stack_event.rs` already owns both
  CounterEffect decoding and typed card-graph traversal. Its new
  `validated_counter_effect` runs the core target-contract predicate before
  either path accepts the rider. Malformed gates, missing nested fields,
  plural selectors, stale/dangling tags and nonspell target domains cannot
  become a plain-counter fallback. Absent riders keep the previous vocabulary.
- `ironsmith-engine/src/effect_model_interpreter.rs` clones the exact core
  CounterEffect into the runtime, including the rider. The runtime owner has
  added the same target-contract check here for the direct compiler path.
- `ironsmith-engine/src/artifact_materializer.rs` is also compiled by
  `ironsmith-runtime-catalog`. `encode_runtime_definition` maps every nested
  effect through `encode_runtime_effect`; native CounterEffect is already in
  `with_native_direct_effect_types`. Materialized effects retain their full
  canonical wire model through `with_serialized_model`, and re-encoding uses
  that model only while it still names the same executor. No new kind-table
  entry or lossy conversion is required.
- `ironsmith-compiled-artifact/src/lib.rs` transports the payload as
  `WireEffect { kind, payload }`. `validate` and `from_json` check the
  format/schema/checksum envelope, not nested executable correctness.
  `materialize_artifact` checks that envelope again before typed decoding.
  A checksummed malformed nested rider can pass envelope validation and must
  then fail materialization. The authored tests distinguish these two gates.
- A complete dropped rider is intentionally indistinguishable from a legacy
  plain counter after an attacker or buggy producer recomputes a checksum.
  Likewise changing PermanentSpell to AnySpell produces another valid typed
  payload. No checksum is source authentication. Direct/compiled expected
  semantics, full-body assertions and a later source/cache boundary must catch
  those substitutions. The tests assert exact rider equality across roundtrips
  and refuse an unacknowledged deletion with its old checksum; they do not
  claim impossible provenance protection for a rechecksummed plain counter.
- `authored_definition_graph` and `remap_card_ids` reach the same typed
  CounterEffect map arm. The rider contains no CardId or TagKey; both its gate
  and land domain survive normalization unchanged. Tests exercise both walks.
  Rejecting a Tagged rider target closes the dangling-consumer construction;
  this does not add a universal verifier for unrelated old tagged effects.

## Cached definitions and native retention

`CardRegistryArtifactExt::register_compiled_artifact`, WASM
`external_registry.rs::materialize_compiled_artifact_batch`, and the artifact
materializer still validate admission before installation. Compiler-runtime's
artifact route serializes the fresh compiled definition and materializes its
envelope; the direct route converts a separately compiled definition. This
patch adds no raw-definition fallback for rejected envelopes and does not
extract a rejected cached payload. Its synthetic direct-artifact suite creates
native definitions without invoking any grammar or compiler route.

`Effect::clone` retains its executor/model Arcs. `StackEntry` and `GameState`
derive Clone and retain ResolutionPrograms. WASM `RuntimeSavepoint` captures
the game, priority state, pending decisions, replay checkpoints and live
continuations; `ReplayCheckpoint`/`LivePriorityContinuation` also preserve
their native owners. The new field therefore travels with the native effect,
not via a public checkpoint reconstruction. Authored tests retain a counter
inside a cloned stack program and exercise cloned post-grant game branches.
Full WASM savepoint/rollback/suspended-decision execution remains unrun and is
still a required later gate.

This changes an executable native struct and program meaning. Exact local
images are not structurally migrated by serde defaults. The independent
`web/ui/src/lib/exact-build-snapshot.js` format1 contract checks buildId,
memory/root shape, integrity, globals/reference-table layout and function-table
stability. `analysis-seed.js` obtains layout/buildId from generated engine glue.
Fresh genuine build/layout evidence and exact-image tests are required; an
old-build image cannot be made current by relabeling metadata.

## Renderer and observable public projection

`ironsmith-text` renders the complete atomic body from the typed gate and
land domain while retaining the original counter target. PermanentSpell
qualifies the replacement sentence, never the initial target. The surface
preserves free price and the entire while-exiled lifetime. Every production
CounterEffect downcast in the renderer was inspected. Plain-counter
compactions now decline marked effects, including conditional/tag-target,
unless-pays, counter-and-damage, replacement/suspend, counter/search,
self-replacement and cross-segment forms. Native trigger, conditional,
sequence and unless wrapper controls are authored.

There are two observable routes beyond artifact payload bytes:

1. Compiler-runtime `attach_rendered_presentation` stores compiled text;
   artifact materialization restores its canonical text. Object creation uses
   `Object::compiled_display_text`, and public audit
   `public_audit_known_object_identity` emits `compiled_card_text` as
   `oracle_text`.
2. WASM `public_audit.rs::sync_restricted_mana` maps arbitrary nested
   restriction effects through `encode_runtime_effect` into WireEffect.
   A counter rider placed in that existing carrier adds nested typed JSON.
   The new test runs that same mapping and requires legacy/AnySpell/
   PermanentSpell/play variants to remain distinct. It is not a full WASM
   checkpoint or digest test.

Therefore unchanged top-level checkpoint fields do **not** establish unchanged
public payload vocabulary. Artifact16/schema and signed audit30 are inherited
and untouched. A later coordinated boundary must explicitly admit this new
typed meaning and decide the successor public digest version; digest10 cannot
be asserted unchanged merely because the native savepoint mechanism is still
Clone. Historical artifact/signature/checkpoint bytes must remain historical.
No new version numbers or regenerated goldens are proposed as already accepted.

## Generator source and inherited regeneration blocker

`scripts/generate_artifact_decoder_shards.py::write_shard` has been source-edited
to emit the same CounterEffect validation helper and both decode/map arms.
Its `write_facade` template also registers the separately stored
`counter_exile_permission_tests.rs`. No generator was run, and checked-in
edits are not claimed to be generated output or proof of regeneration parity.

The inherited full-facade generator is stale beyond this cohort. Its embedded
`CARD_GRAPH_SOURCE` supplies only `Context.bind` and `remap_card_ids`, whereas
the current decoder also owns `authored_definition_graph`,
`Context.authored_definition`, `retained_definition`, `generated_definitions`
and `normalized_definitions`. Current serialization distinguishes authored
LinkedExileDefinition identities from opaque retained identities, normalizes
only supplied generated stamps and reports retained stamps to its caller.
The embedded template lacks that logic and its current graph tests. A full
regeneration would also overwrite the pre-existing in-file source-counter
test. `ironsmith-compiler-lowering/src/lowering_impl/lower/trigger_definitions.rs`
calls `authored_definition_graph` and uses its retained-definition result to
decline inappropriate canonicalization; replacing the current facade with
the stale template would remove that public API and its ownership behavior.

**Full decoder regeneration remains on HOLD until the generator's inherited
facade/graph template is reconciled with the current owned source.** This is
not fixed by the narrow CounterEffect arm template changes. No unrelated
retained-definition implementation or inherited boundary was rewritten here.

## Authored controls and remaining admission work

- Decoder: legacy omission/bytes; both gates and play/cast domains; malformed
  or missing nested gate/boolean; unknown fields; typed graph walks; dangling,
  plural and nonspell rider targets; legacy target-domain preservation.
- Compiler-runtime direct artifacts: envelope roundtrip and typed equality;
  malformed nested materialization refusal; rejected-envelope no-fallback;
  native definition/stack clones; full trigger surface; nested public effect
  projection; success/protected/permanent-gate matrix; actor isolation; exact
  post-exile IDs; priced spell domain versus land-only PlayFrom; actual free
  cast with mandatory life cost, source departure, turn passage, sorcery
  timing, unaffordable cost and no grant revival on a new incarnation.
- Renderer: all gate/verb combinations; unchanged plain counter; original
  target surface; conditional/unless/sequence retention; specialized plain
  compactor refusal.

All are **UNRUN**, including syntax/type checking. These are bounded synthetic
controls; the primary frozen complete-card compiler/runtime witnesses remain
separately owned. Before support or release credit, integrate and review all
workers' source, authorize and run focused plus aggregate gates, reconcile
the generator blocker, decide the coordinated semantic/cache/public/signed
boundary, and genuinely regenerate compiler/engine/WASM/glue/catalog/golden
and build/layout evidence on the final integrated SHA. No measured recoveries,
supported-total changes or deployment readiness are claimed.
