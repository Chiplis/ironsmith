# Decoder generator source alignment

Status: **source-authored, UNVALIDATED; all executable gates UNRUN**.
Base: `d0c214f6de9d8256627804e10812dd698cff7d4f`.
Branch: `repair/decoder-generator-source-20261008`.

No generator, build, test, compiler/parser/runtime probe, formatter, corpus,
remote write, version migration, cache regeneration, or release admission was
performed. Python was used only for literal source comparison/editing, without
importing the generator or parsing/executing its code. The new Python regression
file itself is UNRUN. Checked-in Rust is unchanged, not regenerated output.

## Bounded repair and ownership evidence

The inherited blocker is documented in
`reports/countered-spell-exact-permission-20261008/ARTIFACT-IMPACT.md`.
The generator's `CARD_GRAPH_SOURCE` now mirrors the retained Rust graph block
in `crates/ironsmith-artifact-effect-decoder/src/lib.rs`, including all five
existing graph tests. The narrow differences were inspected before editing:

- Restore `authored_definition_graph` with the existing `(Value, bool)` result
  and supplied `generated_definitions` slice, and all four additional Context
  fields. `remap_card_ids` keeps authored mode disabled and no supplied stamps.
- Restore typed `LinkedExileDefinition` handling: normalize only supplied
  generated stamps, preserve repeated-stamp aliases through the existing
  first-encounter ordinal map, and report every unsupplied typed stamp as
  retained. Do not invent provenance for opaque bytes or change the algorithm.
- Restore authored-only omission of named serde presentation fields. Arbitrary
  JSON keys and semantic names do not acquire this special behavior.
- Remove the obsolete template-only `RetainedCardPayload`/`card_references`
  rewrite. The retained graph has no such branch; current `WireEffect` emits
  `CompiledEffect` with `kind` and `payload` in compiled-artifact/src/lib.rs.
  This does not remove a branch from currently retained Rust execution.
- Preserve the existing typed effect dispatch, unknown-kind errors, CardId
  callback failure propagation, and namespace isolation.

The lowering caller in
`crates/ironsmith-compiler-lowering/src/lowering_impl/lower/trigger_definitions.rs`
passes generated stamps to `authored_definition_graph` and declines authored
canonicalization when its returned retained flag is true. That caller and the
underlying LinkedExileDefinition model are unchanged. No ownership redesign
was needed for this source alignment. This is fidelity to the retained owner,
not an independent proof that all inherited algorithms are correct.

## Test preservation and authored future gates

The full existing facade test block is now a raw `FACADE_TEST_SOURCE` constant
interpolated once by `write_facade`. This preserves both the representative
routing test and `source_counter_payload_decodes_and_normalizes_inside_owned_cost`
without f-string brace escaping or rewriting the tests. All five existing graph
tests remain in `CARD_GRAPH_SOURCE`, including typed retained/generated stamp
alias handling and nested presentation omission. The separately owned
`counter_exile_permission_tests.rs` file and its cfg(test) registration remain.

`write_shard`, its CounterEffect validation helper, and its decode/map arms are
unchanged from the exact requested base. Thus the new atomic rider validation
is not displaced while repairing the older graph-template drift.

Authored but **UNRUN**: `scripts/test_generate_artifact_decoder_shards.py`.
Its intended source gates read Python literals with AST, without importing or
executing the generator, and require:

1. Exact graph-template/retained-Rust block parity, including graph tests.
2. Exact facade-test/retained-Rust block parity, with both existing test names.
3. Single emission of both constants and preserved external counter test
   registration/file.
4. CounterEffect validation helper parity and retained decode/map validation
   calls in generator source and stack_event.rs.

These gates deliberately fail on even formatting drift in paired source blocks.
Future edits must update both owned source locations; they are not permission
to blindly overwrite either one. They do not validate output assembly, registry
routing completeness, Rust semantics, or full generator idempotence.

## Genuine validation still required

All items below are **UNRUN**, require the validation gate to be opened, and
must use the final integrated SHA rather than treating this source commit as
release evidence:

- Run the new source gates (`python -m unittest discover -s scripts -p
  test_generate_artifact_decoder_shards.py`), then genuinely execute the decoder
  generator in a disposable worktree. Review every resulting Rust/manifest
  diff, allowing only explicitly reviewed formatting differences. In particular
  preserve all seven in-file facade/graph tests and the external counter suite,
  both public graph APIs, the owned stamp logic, and both counter validation
  arms. Re-run the generator and require no additional output change.
- Compile and run the complete artifact-effect-decoder crate, including the
  source-counter, five graph, routing, and external counter controls. Exercise
  compiler-lowering authored-root finalization, retained stamp refusal,
  generated aliases, nested definitions, and direct/compiled artifact paths.
- Run applicable focused and aggregate checks on the final integration; genuine
  compiler/engine/WASM/glue/catalog/golden and build/layout generation remains
  separately required. A clean source parity check cannot substitute for any
  runtime or exact-build evidence.
- Keep the inherited semantic/cache/public/signed admission HOLD. No artifact,
  schema, digest, checkpoint or signed version was bumped, and no historical
  bytes were relabeled. The original frozen cohort and support totals remain
  unchanged; this grants no measured recovery or deployment credit.

The historical ARTIFACT-IMPACT report is left intact. Its specific missing
source-template alignment is addressed here; its prohibition on treating a
narrow source patch as genuine regeneration evidence remains in force.
