# Prepared-action main reconciliation

Status: bounded source reconciliation reviewed; all execution remains deferred. No builds, compilation, tests, compiler or engine
probes, formatters, corpus runs, or replay execution have occurred.

The working main is `f711839e5521c258ceccca52fa6b85037496b4ba`.
The preserved campaign source is `c4c7818e5f64d60b6f7f08c674d5b53496d7023c`,
with common ancestor `d51f4ee69b500462c8d70354cec324bf6d04677c`.
The latter source tree also has a verified remote checkpoint at
`618e1f9a0a8634903703b66a5e8d2229e83a762d`; that checkpoint is explicitly
pending admission and must not be merged independently.

Compared with the previously inspected main `77760eaf8670b3cbb0f368632edc4b40762f19a7`,
the new main changes 423 paths, adding 45,872 and removing 20,025 lines. It
overlaps 161 campaign paths and initially produces 86 content conflicts.
The reconciliation retains main's prepared-action architecture and ports the
campaign's rules, retained evidence, and failure boundaries into those owners.
Keeping two competing execution implementations is not a resolution.

## Evidence and admission

The frozen corpus, baseline commit, and baseline identities remain unchanged.
Measured results remain 40 compile recoveries and 3,193 unresolved identities.
Before this port, the conservative source ledger held 1,185 unique proposals
across 1,190 entries, with 36 separately identified prior-proposal holds.

The 1,185 historical proposals were temporarily marked unported while the
owners changed. The combined review at `54a523387187b0b5a5c7faa0b8dbe7a175e12b58`
now restores their source admission. Paired native profile port
`03a183a8be25c7fb7ec590fa648ce25adbb97028` retains that integration and its own
bounded prior body evidence. The source ledger proposes 1,227 unique identities
across 1,232 entries: 1,185 readmitted, 36 exact naming holds source-addressed,
and six keyword-action bodies added. None is an executed recovery result.
The majority threshold remains 1,597 of the measured unresolved 3,193, leaving
370 additional source identities before that execution gate can open.

See `card-failure-stage97-source-admission.md` for exact cohort accounting and
remaining exclusions. The frozen baseline and all pre-port evidence are retained.

## Shared contracts to preserve

- Main's `CompletedEffectOutputs` retains an authoritative chronological
  aggregate and alternative participant/shared projections. Projections must
  not be concatenated with that aggregate to record history twice.
- Main's prepared proposals, original commits, completion freezing, original
  observation, and later completion programs remain distinct phases. No
  participant may execute additions before the appropriate original barrier.
- `ActionObjects` retain full `ObjectSnapshot` evidence. Compatibility readers
  may clone an exact snapshot; they must not rebuild historical facts from
  current objects or follow a later incarnation.
- Replacement-draw continuations retain rich output packets, observation
  synchronization, original-prefix identity, and pending-choice ownership.
  Scalar adapters may expose their aggregate without discarding the retained
  packet used by the owning continuation.
- Payment scope carries the exact payer, cause, reason, reservations, and
  inherited execution context. Checked calculation failures remain errors;
  they cannot become an ordinary inability to pay or an unpaid branch.
- Original damage/counter/movement receipts remain distinct from selected
  objects, requested budgets, replacement prefixes, and added programs.
- Native root/inactive-lane savepoints and authenticated transcript replay
  remain the recovery owners. A public audit digest is not a serialized
  gameplay checkpoint.

## Required source review

The review covers the effect kernel and composition, payment/priority, damage
and counter results, card/zone/token movement, and definition/snapshot/observer
and public interfaces. Automatically merged paths require review too: a clean
text merge does not establish compatibility with the new owners.

The final admission decision must account for executable/schema changes,
public evidence and historical signature/replay behavior. Previously published
artifact 8 / public digest 4 / audit 21 describes the prior draft, not an
automatic compatibility claim for this unfinished union. Authentic historical
fixture recovery, including the missing v5 fixture, and all actual validation
remain deferred prerequisites.

## Reconciliation checkpoints

These checkpoints are source-only. The admission record below reuses prior
full-body evidence only after this combined owner review on the new main. The coordinator preserves the original refs and copies only each lane's
owned file changes into the final reconciliation.

| Owner | Latest assembled source | Bounded review state |
| --- | --- | --- |
| Kernel/composition | `94bed23bbd64b6c17cd8a6f0452dcf6888a85fa4` | Findings source-addressed: participant Stop, reached-input preflight, optional-payment rollback, chooser binding, genuine prepared-cost phases and retained native cursors. Combined review remains. |
| Payment/priority | `cf3bdf70cca3d8d6a3280244cee27d5279c616ec` | Original typed-error findings corrected; counter payment completion preserves cause and rich outputs. Combined draw-owner check remains required. |
| Movement/token/card operations | `31067f4e8723fc7652691ae4a22808c4c8e9f7f1` | Original import, sacrifice error-precedence and Mill continuation findings corrected. Kernel preflight and retained draw integration remain dependencies. |
| Damage/counter results | `87a03be59da89075f0e9f7a1e227970110060eda` | Recipient-local ceilings, error precedence, retained draw tails and original damage Instead payloads pass the bounded source recheck. |
| Definitions/evidence/public interfaces | `b595fc21dfc9cff126a9ab70325a72eacd31584e` | Disclosure ownership, terminal errors, browser parity, exact reveal reads, ordered prevention, original damage Instead processing and canonical public receipt labels pass bounded source review. |

The optional `pay_as_cost` prepared path must advertise only components with real
prepared payment contracts. Ordinary compound payments retain their ordinary
TotalCost owner; rejection of an unsupported simultaneous payment does not
constitute source completion. Canoptek Wraith's reviewed ordinary optional
mana-plus-sacrifice route is distinct from a hypothetical simultaneous compound
payment. Counter transfer cursors likewise gain no capability merely because
Put/Remove counter primitives expose retained prepared continuations.

The final combined review must establish selection/original/freeze/observation/
completion order, actual draw-boundary suspension, exact contextual bindings,
one-time prefixes, typed failure rollback, and one authoritative history
projection. Source review is not runtime validation.


The proposed compatibility successor is documented in
[`card-failure-prepared-main-compatibility.md`](card-failure-prepared-main-compatibility.md).
Its gate and corrected public-evidence projection passed the bounded source
review at `54a52338`. Runtime/corpus validation and regeneration remain deferred.
