# Variable Suspend bodies, reconstructed from retained source payloads

Base: `69a946ec767deda59927d63f08dd17fabff470f3` (published stage 76).
Reconstructed implementation checkpoint: `c798cf28a9460c0928acbc3714f4a240c8c551b8`.
The earlier final commit c4b22039fc23fa042ea9242e01fae6e35a52868a was lost
with its filesystem. This reconstruction does not assert that old commit
identity or byte-for-byte equality to its unavailable tree. Retained literal
source transformations were replayed against the verified base, with fresh
full-file payload and blob/tree-hash retention.

This is a four-card source proposal, with **zero verified recoveries**. No build,
compiler probe, tests, formatter or corpus execution was performed. Prior
source-review results do not substitute for review of this reconstructed diff.

## Exact bounded cohort

| Card | Oracle identity | Complete body reviewed |
| --- | --- | --- |
| Aeon Chronicler | a1399cb8-f22a-487e-b539-2db47b09a790 | Hand-size CDA; positive-X Suspend; draw once per time counter removed from this exiled card |
| Benalish Commander | 3a1e85fe-7064-4df7-bc3b-24131cd15d06 | Soldier-count CDA; positive-X Suspend; create an actual 1/1 white Soldier for every removed time counter |
| Detritivore | ba5fa5ca-f72d-49dd-9f86-e6e1292f5bf0 | Nonbasic land cards in every opponent's graveyard CDA; positive-X Suspend; target and destroy a nonbasic land per removed counter |
| Fungal Behemoth | 844f1fb4-81d2-4a2a-9de7-20e6bf37d77b | Sum of +1/+1 counters on creatures you control CDA; positive-X Suspend; optional +1/+1 counter on any target creature per removed counter |

All four names and identities were checked against the authoritative ledger as
unaddressed. Full Oracle text and metadata were regenerated from the pinned
base's cards-20261003.json.xz into fixtures/variable_suspend_bodies.json.fixture.
The separate family identity manifest is
fixtures/card-failure-campaign/variable-suspend-bodies-recovery.json.
The global coverage ledger is unchanged.

## Ownership and behavior

SuspendTime carries either a fixed count or an announced X with its minimum.
Fixed counts retain the old numeric JSON representation. Compiler semantic
keywords and core alternative-cost metadata share that type. The keyword reader
consumes the complete local cost and X restriction. The local restriction is
not a minimum on ordinary casting. Document dispatch hands the typed keyword
directly to lowering. Existing fixed-count builders share the typed builder
implementation with the compiler's variable-count route.

The special-action owner checks minimum affordability, asks for X before
payment, rejects an out-of-range answer, passes the answer through the existing
interactive payment transaction, and resolves that same X for initial counters.
Counter replacement processing and rollback remain shared with fixed Suspend.
The announcement is not retained as the later spell's X. Existing upkeep,
last-counter optional casting, and haste owners are retained.

The existing last-counter/exile grammar also handles ordinary passive
counter-removal triggers. Counter type, source identity, Exile scope and
per-counter multiplicity remain typed. Both zone-selection owners derive the
functional zone from the typed source filter. The printed body triggers have
no extra resolution-time exile condition. Their text retains the qualifier.

The four CDA bodies use shared scalar/count grammar and layer evaluation:
own hand count; controlled Soldier count; nonbasic land-card count in all
opponents' graveyards; and counter totals over controlled creatures.

The counter-total layer owner widens each counter count and uses checked
aggregation. Both direct and context CDA evaluation record out-of-range
scalars in CalculatedCharacteristics.numeric_range_error. Checked discovery
rejects provisional characteristics before publication. The Fungal boundary
scenario gives two creatures individually representable counts of 1.5 billion
and 1 billion, expects a scalar error for their 2.5 billion sum, then reduces a
count and checks later recovery.

Balanced-reminder removal retains every outside token and rejects malformed
parentheses. Grammar negatives cover executable instructions after a reminder,
unclosed groups, unmatched closing parentheses and extra closing parentheses;
a nested balanced reminder remains accepted.

## Authored, unrun scenarios

The full-card helper separately calls compile_to_runtime_definition with
parse-loss capture for the direct route. Its artifact route independently
compiles, validates, JSON-restores and materializes the transport artifact.

Scenarios cover complete CDA and trigger bodies, actual payment, invalid and
pending X, replacement doubling, pending-placement rollback, each removed
counter, wrong type/source/zone negatives, nonbasic-only targeting, multiplayer
graveyard scope, control changes, optional effects, phasing, upkeep, last-counter
casting, haste, a queued body after its source leaves exile, and Fungal's checked
aggregate range boundary.

## Explicit partials and limits

Roiling Horror receives the shared Suspend primitive but receives no coverage
credit: its exact life-total difference CDA remains outside this bounded
proposal. Watcher of Hours remains partial because "you remove" requires an
explicit removal-player matcher. Other suspended-card targets and Suspend-grant
programs remain outside this work.

All scenarios remain uncompiled and unrun. Semantic output must be checked at
the later campaign validation gate. No placeholder, rejected line or discarded
body is counted as coverage. Numeric values remain subject to checked host
representation limits rather than an arbitrary-precision claim.
