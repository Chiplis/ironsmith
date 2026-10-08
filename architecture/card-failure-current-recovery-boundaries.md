# Current campaign recovery and validation boundaries

Current source proposal: first recovered-body stack on main
`5cc46c1fa41edb235aacd8e7567ad4ab2f12b7a1`, with a coordinated12/8/25 boundary,
including source-counter causes/receipts
and static Aura prevention. This combined successor is still unpublished.
This is source inspection, not executed recovery validation. The separate
current-main corpus refresh is not execution of this compatibility gate.

Commit `2511818a28ddb00d7ec96e85bf345eb170b88fdb` removed serialized gameplay
recovery checkpoints. The current main's exact local same-build memory images are
preserved user architecture and a distinct mechanism. Do not reintroduce a lossy
generic gameplay serializer or remove exact images when adding a card family.

## Three distinct compatibility surfaces

- **Compiled artifact format12** represents card definitions and programs.
  Prevention model fields, subtype construction zones, explicit creature-choice
  family, exact source/controller semantics, canonical source-counter transport,
  fixed-prevention ownership and
  Aura-owned live attachment semantics need regeneration and codec review.
- **Public audit checkpoint version8** is a redacted digest input exported by
  `wasm_game_impl/public_audit.rs`. Its typed restricted-mana programs expose the
  changed models. It cannot restore continuations, history, replacement managers
  or private gameplay state.
- **Signed audit protocol25** governs current action/replay compatibility,
  including changed current-main receipt/payment/Stop/decision and reference
  outcomes, requesting counter causes, nominal/physical payment results and Aura
  source/controller/lifetime behavior. Signed genesis has no exact engine-build
  gate; protocol/digest admission must refuse old24/7 replay, including callbacks
  supplied with `requireEngineReplay=false`. Verifying older signatures does not
  establish engine equivalence.

Earlier6/3/19, stage93 7/3/20, published862 8/4/21, published863 9/5/22,
published864 10/6/23 and published865/current-main11/7/24 remain historical
boundaries. The prior unpublished stage10012/8/25 fingerprint is not an admitted
historical release. See `card-failure-next-series-01-compatibility.md` for the fresh
successor, original24/7 signature-only retention, enum positions and deferred
regeneration. Public digest8 is not serialized gameplay recovery. Runtime identity
tests still require `exportSyncCheckpoint`, `exportRedactedSyncCheckpoint`,
`importSyncCheckpoint`, `importForeignSyncCheckpoint` and
`isReplayCheckpointBoundary` to remain absent.

## Current gameplay owners

`wasm_game_impl/runtime_savepoint.rs` captures native `GameState`, trigger queues,
priority/payment continuations, pending decision state, runtime identity counters
and `grand_melee_host_lanes`. Local analysis exchanges exact native branches and
restores their owning state. These handles are client-owned and never network
gameplay payloads. Current EffectContext and borrowed BranchScope APIs are kept.

The existing `captureLocal`/`restoreLocal` paths copy actual same-build WASM memory,
globals and reference tables under the foreground worker queue. Trusted analysis
seeds retain build/root/layout and supported reference/function-table checks while
avoiding persisted integrity hashing. They are exact local images, separate from
public evidence and authenticated transcript recovery. A peer cannot supply a
trusted analysis seed.

Persisted exact-instance points retain integrity and match/seat/prefix checks.
`web/ui/src/lib/local-runtime-recovery.js` retains bounded native points matching
the current game, match, seat, accepted action prefix and audit-state hash.
Authenticated recovery verifies genesis/envelopes, signatures and accepted-prefix
authority, can restore an already verified local anchor and replay its suffix,
and verifies the signed/public resulting head. Accepted genesis and full replay
remain fallback when valid local anchors cannot recover state. No partially
verified replay is accepted. Cross-peer recovery receives authenticated transcript
evidence; public-audit/hidden-card getters provide no gameplay importer.

The source checkout still lacks generated `web/wasm_demo/pkg/engine.js` and verified
provenance of its exact-build exports. Imported consumer names do not prove
`exactSnapshotBuildId`, `exactSnapshotLayout` or matching glue. The real generation
path/build/layout must be verified when that execution is allowed. The schema gate
invents no substitute glue or build identity.

## Gate for newly retained card state

Activation history, copied programs, linked ownership, prevention filters/chosen
colors, private inspection and delayed exact references must survive native root
and inactive-lane saves, branch exchange, pending/error rollback and authenticated
replay. Source properties use current live characteristics and exact departed-source
LKI; stable identity cannot replace a departed incarnation. Source-relative
attachments require a live, unphased battlefield owner and its current exact host
before LKI. Requesting payment cause, successful nominal acceptance, physical
child removals and energy's legacy nominal aggregate must retain their separate
ownership across original/completion phases and rollback. Missing required
evidence produces the appropriate typed incomplete-state error, never zero, an
empty program or a current-object substitute. Artifact/model fields need separate
codec and historical-evidence review.

Authored regressions exercise actual owners. Obsolete wire importer/exporter
coverage stays historical. Deferred validation includes native root/lane retention,
analysis cancellation and same-build seed reset, pending retries, public digest8,
protocol25 replay, verified local anchors, fallback and incompatible-evidence
refusal. Missing original v5 and ungenerated current v12 artifact goldens remain
separate prerequisites. No builds, tests, compiler/browser probes, formatters,
code generation or replay execution ran in this lane; no ledger or measured count
is changed here. Later repeat, plural-choice and Until work is outside this gate.
