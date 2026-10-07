# Suspected designation bodies

This source-only family freezes the exact bodies and Scryfall/oracle IDs in
`fixtures/suspected_designation_bodies.json.fixture`, extracted from the campaign's
2026-10-03 corpus and e8740178 baseline diagnostics.

## Bounded implementation

- **Agency Coroner:** the sacrificed creature's designation is captured before it
  moves and tested with a typed last-known predicate. Missing retained evidence
  produces `IncompleteEvidence`, including under negation, rather than choosing
  the one-card branch. Current or blinked incarnations cannot replace paid state.
- **Airtight Alibi:** copular removal follows the enchanted-object antecedent.
  Untap, temporary hexproof, and the static +2/+2 remain. The no-suspicion rule is
  owned by the exact attached Aura and follows its attachment, phasing, and
  battlefield lifetime; removing the host's abilities does not erase that rule.
- **Deadly Complication:** modal destroy/counter bodies retain separate target
  bindings and the spell controller makes the optional removal choice. Removal
  follows an exact object ID, including when a counter replacement blinks and
  suspects a new incarnation during the same resolution.
- **Eliminate the Impossible:** investigate and the opposing-creature pump remain.
  The conditional removal uses the previously affected set and keeps the dynamic
  opponent relation. An empty suspected subset does not erase the other effects.

The appended core `BecomeSuspected` restriction and appended optional snapshot
field preserve the ordering of previously published serialized members. Grammar
recognition stays in named front-end rules; lowering consumes typed predicates,
references, and the existing `ClearSuspectedEffect`.

## Partial bodies

**Frantic Scapegoat** is not claimed complete. Its exact counted choice needs the
immutable simultaneous entering group and the actual choice actor; optional
acceptance, impossible suspicion, and no choice must remain distinct. The
unimplemented event-subset production fails explicitly instead of rebinding to
the source named by its intervening condition.

**Hot Pursuit** is not claimed complete. It needs completed player-loss history,
source-lived goad, the union of goaded/suspected recipients, and preservation of
its control, untap, and haste body. Source departure before its first trigger
resolves must still allow suspicion while preventing goad from starting.

## Authored validation, all unrun

`crates/ironsmith-compiler-runtime/tests/suspected_designation_bodies.rs` contains
full-card direct/artifact round trips and native cast/activation scenarios for the
four bounded bodies, including zero/decline, source departure, phasing, detached
Aura, target loss, paid-history blink, missing evidence, modal target coordination,
and actual replacement-addition blink with pending rollback/replay. Grammar and
engine-local scenarios cover typed subject distinctions and positive/negated
legacy snapshots.

No test, build, compiler probe, formatter, or corpus rerun has been performed.
These are proposed source recoveries, not verified compile recoveries.

Fresh source review through `29c6b383` and combined integration review at
`6afb57377` clear the four bounded bodies. The two partial bodies retain their
explicit holds, and no execution-based recovery is claimed.

## Rules sources

- [Comprehensive Rules, 2026-06-19, 701.60](https://media.wizards.com/2026/downloads/MagicCompRules%2020260619.pdf): suspicion is noncopiable designation state, grants menace/cannot-block, and cannot be applied again to an already suspected permanent.
- [Murders at Karlov Manor release notes](https://magic.wizards.com/en/news/feature/murders-at-karlov-manor-release-notes): designation persists through ability loss; Hot Pursuit's departure before resolution prevents its goad duration from starting.
