# Native draw ordinal history

UNVALIDATED source increment for two already-proposed frozen identities:

- Astrologian's Planisphere, oracle_id `4dcd0583-cee0-46a6-ba43-93bca5eb0428`:
  its equipped creature's granted third-card trigger, alongside its cast trigger,
  Job select, Wizard grant, and equip ability.
- Madame Masque, oracle_id `b5b0e85f-6651-4667-829c-2253ff8b130d`:
  its second-card trigger, alongside entry connive and the complete 2/1 black
  Villain token with menace.

The exact bodies come from `fixtures/card-failure-campaign/cards-20261003.json.xz`.
The full-body regression reuses their existing `job_select_materialization`
and `lossy_metadata` fixtures. No new identity is claimed. No builds, compiler
or engine probes, tests, formatters, generated artifacts, coverage admission,
ledger-count updates, schema versions, or publication have run or changed.

## First-observed occurrence owner

The ordinal matcher no longer sums through a u32 accessor, subtracts a possibly
narrowed event amount, or saturates an inconsistent window. It counts retained
card-vector lengths in checked i64 arithmetic through the exact occurrence,
then compares the authored u32 ordinal by lossless conversion. Numbered-set
multiplicity has an explicit checked conversion at its existing u32 boundary.

The old projected history concatenates committed observations before staged
observations. That is not always physical chronology: a later replacement
original can be published while an earlier enclosing draw is still staged.
Re-enriching an older staged alias can also move its aggregate-journal position.
Arbitrary provenance IDs cannot repair this, because publication can allocate
or replace them and a caller-supplied ID is not completed-order proof.

`TurnHistory::draw_occurrences: Option<TurnEventRecords>` therefore retains the
native first-observation order. The stage/record owners append each physical
occurrence once. Clones and enclosing aggregate projections preserve RawEvent's
shared occurrence identity; promotion and enrichment replace that entry in
place. The completed-receipt refresh path updates the same owner. Removing a
staged provenance wrapper does not erase its physical draw. The ledger shares
immutable records and does not store a new numeric count or widen any scalar.

A changed drawing player or card vector for the same occurrence invalidates
chronology. A required occurrence missing from native order, or an aggregate
projection not represented by that owner, is incomplete evidence. Reconstruction
never sorts provenance IDs or treats unknown history as an empty turn.

## Lifetime, errors, and recovery

Default TurnHistory has no draw chronology. GameState::new establishes a known
empty ledger; clear_for_new_turn and the normal next-turn owner do so again at
the real boundary. An unknown ledger remains unknown when more events arrive.
A complete empty ledger for a real fresh turn is distinct from missing evidence.

The existing checked trigger-collection scope retains matcher errors instead
of accepting an empty match set. Effect execution, replacement-original capture,
and pending-event drain/stacking retain these failures through their existing
transaction owners. Public pending-drain regressions cover ordinary and
simultaneous collection returning IncompleteEvidence and restoring game/queue.

Native GameState clones, execution checkpoints, RuntimeSavepoint,
ReplayCheckpoint, and Grand Melee TurnStore lane swaps retain the ledger exactly.
Regressions cover root and inactive-lane restoration, sibling isolation, staged
versus committed inversion, alias enrichment, unknown/conflicting histories,
turn reset, wide arithmetic, and the full direct/artifact card scenarios.
All regressions are authored and unrun.

## Compatibility boundary

This increment adds one presence-bearing, native-only TurnHistory field.
TurnHistory has no serialization derive. TurnEventRecord, TurnEventRecords,
CardsDrawnEvent, compiled models, artifact wires, public snapshots, scalar
widths, and public audit version 5 are unchanged. Public audit remains an
output-only projection with no gameplay importer. Recovery continues to require
native retained state or replay; no public/legacy serialized claim can create
an authenticated draw order.

## Bounded physical producer follow-through

The reached first/nonfirst reads in DrawCardsEffect, expanded-original draw
commit, the turn-draw proposal, and both turn adapters now query whether the
native owner contains a nonempty draw for the actual player. No numeric total,
u32 sum, saturation, or widening is needed to answer that question. The query
first validates known chronology, so a missing owner cannot silently label the
next draw as first. The legacy numeric accessor remains outside these paths.

Unpublished cards in the current direct batch and already completed redirected
segments remain explicit local evidence. Redirected draws bind first status
to their actual drawer, including the observation emitted by an expanded
original. Cost feasibility preserves incomplete-evidence errors; zero-card
instructions neither consult nor manufacture history. TurnRunner preserves the
typed execution failure through its existing private-game rollback. Its unused
private PendingTurnDraw.previous_draws field is removed rather than storing a
fictional bounded count; it was never a serialized carrier.

Additional authored, unrun scenarios use the real draw producer to check
first/nonfirst batching, alias publication, native checkpoint replay, the next
turn, redirected recipients, unknown-history sequence/cost failure, and pending
commander-draw rollback followed by a complete retry. This closes the identified
producer follow-through; it is not a whole-engine audit of unrelated numeric
history accessors or draw-step counters.


The public Rust `turn::execute_draw_step` and `execute_draw_step_with` adapters
now return `Result<Vec<TriggerEvent>, ExecutionError>`. Required evidence errors
restore the same checkpoint and remain visible to ordinary callers without an
active resource meter. Existing successful repository callers explicitly unwrap
the result; TurnRunner assertions retain the precise typed error. An authored
no-meter regression exercises both public adapters, unchanged draw-step state on
failure, and recovery at the real next-turn boundary. This is a Rust source API
change only; no serialized carrier or public audit encoding changes.


Completed draw segments now stage their occurrence before an added program can
run, including when simultaneous actions or explicit matching holds defer
trigger collection. Expanded originals also receive prior local observations
when determining the actual drawer's first/nonfirst status. Authored regressions
cover both hold owners, redirects with and without prior draws, repeated added
programs, and a nested added draw retaining original/addition/later-original
order through enclosing aggregate publication, plus pending/error rollback after
an original has been staged. Staging uses the existing native
alias owner and remains inside the enclosing rollback transaction.


The plain redirected-draw branch uses the same staging owner. It first finishes
any earlier direct segment before changing the recipient, preventing a buffered
direct draw from being ordered after a later redirect. Mixed-path regressions
cover a plain redirect followed by an expanded original, and a buffered direct
draw followed by a plain redirect, under both kinds of trigger hold.
