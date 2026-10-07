# Next-series-04 combined source integration review

Reviewed combined source anchor `0bfe423cabf555b0d59ccf8422574b479fa84025` relative to `ff30f190c`, in `ironsmith-next-series-04`. HEAD matched the requested anchor and the checkout was clean when inspected.

## Disposition

**Bounded combined-source clear.** No concrete new integration/API/test-oracle defect was found in this combined packet. The three final scoped reviews are incorporated without reopening their entire scope. This is authored, independently reviewed, **UNRUN** source-proposal evidence, not executable gameplay validation. No build, test, probe, formatter, code generation, corpus execution, remote write, source edit, or accounting edit was performed.

## Integration checks

- The complete delta contains 13 files, all additions except a one-edge lockfile addition, two dev-dependency lines, and five test-module registration lines. There are no production behavior, model, protocol, descriptor, browser implementation, or central campaign accounting changes relative to `ff30f190c`.
- Cho-Manno test, fixture and report are byte-identical to scoped-reviewed `fd7106e0b75ce30ea6a57b6104b9c848edabfe44`.
- Revelation/Wandering Eye compiler-runtime test, snapshot/crypto test, fixture and report are byte-identical to scoped-reviewed `e7b17f36841df70ab85dfc9a0c5897252aeb705f`.
- Chittering Rats/Chimney Imp test, fixture and report are byte-identical to scoped-reviewed `25e628c833f406743df4c142af3bb4858c12b225`. The final independent library review explicitly clears the singleton correction; it is no longer pending. Combined head retains zero singleton selection callbacks, one normal surplus callback, and two pending/resume surplus callbacks, without weakening movement/privacy/order assertions.
- Compiler-runtime files are separate integration test binaries with no helper-name collision. Both have normal automatic test discovery and the existing compiler/artifact/catalog/serde dependencies. Tools likewise has automatic integration-test discovery; `autobins=false` does not disable tests.
- Tools imports its existing `ironsmith-runtime` facade, which reexports engine APIs and the runtime-catalog materializer. Registry/compiler dependencies are already present. The new tools test invokes the same public strict registry entrypoints as its scoped review.
- The snapshot source file is registered exactly once as a private child of `ui_snapshot`, behind `cfg(test)` and an explicit adjacent-file path. The actual owner is `ironsmith-web-session`: its library path points to `../ironsmith-wasm/src/lib.rs`, which imports `ui_snapshot`. The wrapper wasm crate instead points at `binding.rs`. Thus adding the two compiler dev dependencies to web-session is consistent with the test's actual owner. `autotests=false` there does not suppress this library-unit-test module.
- The lockfile adds only the web-session-to-compiler edge. Compiler-runtime was already represented in that package's dependency list through its existing build dependency, so its new dev dependency needs no second edge. This is source-level graph consistency, not Cargo resolution verification.
- All new fixture include paths resolve from their physical source files to the repository-level fixtures. Distinct fixture names avoid packet collisions. Snapshot tests retain the existing ID-counter guard and use separate game instances, so the combination introduces no new shared test-global fixture owner.
- The combined snapshot oracle retains reverse `hand_cards` display order and forward persistent-look order. The crypto oracle retains full sorted requirement equality and camelCase transport fields. Its previously reported defects are not reintroduced by integration.

## Baseline and compatibility boundary

The original scoped work used `94e075587`; the combined baseline `ff30f190c` contains intervening next-series-03 parser/source changes and its compatibility boundary. Inspection confirmed the invoked compiler-runtime public API source, native damage/zone/controller/phasing/selection paths, snapshot/crypto owner APIs, and relevant filter model are unchanged across that baseline shift. The engine materializer delta in that interval is documentation only; artifact transport advances to current format 14 without changing these constructor/validation/JSON entrypoints. The new tests generate artifacts from full source at runtime and do not import historical compiled blobs or assert an obsolete format constant.

Relative to `ff30f190c`, next04 introduces no artifact/model/protocol change. The inherited **14/9/27** boundary remains untouched; this review does not recertify or execute those inherited aggregate compatibility gates. Inherited parser changes can still affect eventual test outcomes, which source inspection cannot establish.

## Referenced scoped review records

- `/workspace/scratch/bc560e8d90ff/chomanno-independent-review.md`, final head `fd7106e0`.
- `/workspace/scratch/bc560e8d90ff/hand-visibility-independent-review.md`, final head `e7b17f36`.
- `/workspace/scratch/bc560e8d90ff/library-top-independent-review.md`, final head `25e628c8`.

Their semantic and coverage boundaries remain in force: assignment-layer prevention is not a combat-step proof; native crypto requirement generation is not browser proof acceptance; engine hand-choice privacy is not a network/UI cryptographic proof; retained snapshot cache usage is not exhaustive same-perspective cache coverage.

## Admission and eventual execution

All five bodies are source-proposal eligible within this bounded independent review. The designated accounting owner must separately reconcile identity-level proposal admission and duplicate-credit rules. No count or tracked accounting file was changed by this review. There is zero new measured recovery and zero residual reduction from this source-only packet.

When execution is authorized under the campaign threshold, the focused owners are compiler-runtime `chomanno_full_body` and `public_revealed_hand_bodies`, tools `opponent_hand_library_top_bodies`, and web-session library unit tests in `ui_snapshot::public_revealed_hand_source_tests`, followed by the appropriate combined and compatibility gates. Their compilation, execution and results remain entirely unestablished. Execution is not being imposed as a prerequisite to authored-source proposal admission.
