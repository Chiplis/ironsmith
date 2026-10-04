# Optimistic calculation in Verified multiplayer

Verified matches retain two lossless runtime branches in the engine worker.
The visible branch calculates actions provisionally; the verification branch
replays and accepts the signed transcript in sequence. Trusted matches retain
their host sequencing path. Engines without runtime branch support retain the
synchronous Verified path.

## Calculation and acceptance

A `provisional_action` announces a command, its sequence, parent ID, accepted
basis sequence, and calculated public checkpoint hash. It arrives independently
of the canonical action queue. Only the authenticated transport peer assigned
to the actor can announce or cancel its provisional action. These packets are
presentation claims, never audit evidence, signatures, or quorum votes.

The visible engine checks the current decision and calculates the command in a
runtime transaction. Public identity claims are restricted to cards the command
announces publicly. Private identities are never transported in provisional
packets. Public RNG results, encrypted shuffle epochs, and public openings may
be transported once their material is available; canonical replay independently
authenticates them on its own branch.

Unknown RNG results, shuffle outcomes, and identities needed by this seat block
calculation. Existing openings need no new exchange. A private disclosure to a
different viewer uses the same placeholders as canonical replay on a non-viewer
seat. View-window metadata itself is not a material dependency; individual card
openings bound the actual dependency.

If calculation must wait, the ordinary verification pipeline acquires the
material and calculates the action on its branch. Before payload signing and
quorum collection, its completed calculation can be copied into the visible
branch, published provisionally, and announced to peers. The originating click
then releases its interaction gate while verification continues.

Dependent choices extend the provisional suffix and can belong to either
player. Their verification jobs wait for their preceding accepted action
outside the verification queue. Once a matching prefix verifies, the visible
board stays at the newer provisional state. The accepted transcript, audit
hashes, recovery checkpoints, and end-of-match disclosures always use the
verification branch.

A conflicting command, mismatched public hash, failed verification, signed
intent cancellation, or authenticated provisional cancellation discards the
suffix and restores the latest verified branch. Dependent local submissions
are cancelled, including their protocol dependency waits. Provisional entries
are capped at 32 and have a 120-second verification deadline. Match recovery,
disputes, rematches, and lobby teardown clear provisional state and retained
branch handles.

## Runtime branches

`createRuntimeSavepoint` retains game state, live continuations, triggers,
replacement-related state, hidden audit state, viewed cards, and gameplay ID
cursors. `exchangeRuntimeSavepoint` swaps a retained branch with the active
runtime. Priority, payment, and inspector analyses and snapshot encoding caches
travel with their branch. Shared card definitions remain shared; their ID
allocator never rewinds.

A background worker call enters and leaves its branch in one ordered worker
queue task. Network waits never keep the active worker in a background branch.
`copyRuntimeSavepoint` restores the visible game without consuming the retained
verification branch. Background snapshots do not update visible snapshot
versions or priority revisions. Version 2 diagnostics journals record branch IDs,
savepoint handles, and savepoint lifetimes; a replay must route each call through
its recorded branch rather than dispatching both calculations onto one game.

## Protocol ordering

Crypto material, RNG, and action quorum requests may overtake an earlier
`apply_action` waiting in a transport action queue. Their authorization waits
for `request.seq - 1` to become accepted, then rechecks the exact next sequence,
match identity, actor, transcript hash, decision, and material authorization.
A request for the current action does not wait for that action to finish.
Responses remain out of band so they can unblock an action already in flight.
Waits expire with expected/received sequence diagnostics and are cancelled on
match recovery or lifecycle changes.

## Verification coverage

Tests cover dependent choices with delayed canonical dispatch, rejection and
rollback on both peers, cancellation of dependent submissions, missing material
and already received RNG, restricted public openings, private non-viewer
behavior, runtime branch state, gameplay ID isolation, and live casting continuations in real WASM, hidden draws
through the real worker, and real full-UI Verified peers with delayed signature
verification and matching accepted transcript hashes.

Older gameplay fixtures with 30 or 60 copies of a nonbasic card cannot start
under the current constructed deck validation. They need legal test decks
before they can provide casting/private-view regression coverage.
