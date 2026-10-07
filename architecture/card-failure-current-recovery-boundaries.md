# Current campaign recovery and validation boundaries

Current source proposal: prepared-main reconciliation against
`f711839e5521c258ceccca52fa6b85037496b4ba`.
This is a source inspection, not executed recovery validation.

Commit `2511818a28ddb00d7ec96e85bf345eb170b88fdb` removed serialized gameplay
recovery checkpoints. Older campaign reports describe the guards that preceded
that change; those reports remain historical evidence, not the current API.
Do not reintroduce a lossy gameplay serializer to implement a new card family.

## Three distinct compatibility surfaces

- **Compiled artifact format 11** represents card definitions and programs.
  Required regeneration and exact default/ordinal compatibility remain part of
  the deferred artifact validation gate.
- **Public audit checkpoint version 7** is a redacted digest input exported by
  `wasm_game_impl/public_audit.rs`. It is not an executable gameplay snapshot
  and cannot restore continuations, history, replacement managers or private
  state. Changes to this encoding require explicit compatibility analysis.
- **Signed audit protocol 24** governs current action/replay compatibility.
  Verifying an older transcript's signatures does not establish that replaying
  it in the current engine reproduces its former semantics or public hashes.

The earlier 6/3/19, stage93 7/3/20 and published862 8/4/21 and published863 9/5/22 and published864 10/6/23 descriptions remain
historical boundaries. The current source proposal is 11/7/24; see
[`card-failure-activation-combat-blind-compatibility.md`](card-failure-activation-combat-blind-compatibility.md)
for the concrete model/evidence changes, signature-only historical23 admission
and deferred regeneration. These are separate surfaces. In particular, digest7
does not promise serialized gameplay recovery. Existing runtime identity tests
require `exportSyncCheckpoint`, `exportRedactedSyncCheckpoint`,
`importSyncCheckpoint`, `importForeignSyncCheckpoint` and
`isReplayCheckpointBoundary` to remain absent.

## Current gameplay owners

`wasm_game_impl/runtime_savepoint.rs` captures native `GameState`, trigger queues,
priority/payment continuations, pending decision state, runtime identity
counters and `grand_melee_host_lanes`. Local analysis exchanges exact native
branches and restores their owning state. These handles are client-owned and
are never network gameplay payloads.

`web/ui/src/lib/local-runtime-recovery.js` retains bounded local native points.
Candidates must match the current game, match, seat, accepted action prefix and
audit-state hash. Recovery may try an existing local state or saved native
point, replay its suffix and verify the signed head. Failed local levels fall
back to accepted genesis and full replay; no partially verified replay is
accepted. Cross-peer recovery receives transcript evidence, not a foreign
executable gameplay snapshot. Public-audit and hidden-card metadata getters do
not create a gameplay restore representation.

## Gate for newly retained card state

New activation history, copied programs, linked ownership, private inspection
or delayed references must survive exact native root and inactive-lane saves,
branch exchange, pending/error rollback and accepted transcript replay. Missing
required evidence must produce the appropriate typed incomplete-state error;
it must not become zero, an empty program or a current-object substitute.
Retained artifact/model fields still need independent typed codec and legacy
evidence review where those representations actually exist.

Authored regressions should exercise these actual owners. Tests for obsolete
wire import/export guards are historical and should not be presented as current
coverage. The consolidated deferred validation must include native root/lane
savepoint retention, analysis cancellation, pending retries, public digest
compatibility, current protocol replay and rejection of incompatible evidence.
No builds, compiler probes, tests, browser scenarios or replay executions were
run for this documentation correction; no card coverage is added.
