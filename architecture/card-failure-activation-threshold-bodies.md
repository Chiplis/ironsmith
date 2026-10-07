# Current-turn activation thresholds

Status: source-complete proposal, **EXECUTION UNVALIDATED**. Four full frozen
bodies are retained in `fixtures/activation_threshold_bodies.json.fixture`:
Dragon Whelp, Farrelite Priest, Initiates of the Ebon Hand, and Nalathni Dragon.
No build, compilation, compiler probe, test, formatter, engine run or corpus
execution was performed. No measured coverage or campaign ledger is changed.

The shared named predicate reader recognizes the complete current-turn
activation threshold. It emits a typed turn-event predicate and appended core
condition. Lowering does not inspect names, labels, Debug output or Oracle
strings. Existing conditional and next-end-step owners preserve the written
scope: the threshold is evaluated during the original ability's resolution;
only the sacrifice instruction is delayed.

## Activation identity and completed admission

The new presence-bearing current-turn ledger reuses `AbilityOrigin`, the
existing Power-up/Exhaust acquisition namespace. Its key is the exact source
ObjectId, that origin, and a compiler-retained definition stamp, not a calculated
display index, aggregate source count, resolution ordinal, or controller. Printed slots, independently
registered grants, borrowed donor acquisitions, temporary occurrences and
counter/level grants keep their existing structural identities.

The definition stamp includes the typed face, complete typed ability definitions
(including activation costs), and authored occurrence. Caller-local CardIds are
normalized out. Generic definition metadata is hashed; no name selects behavior.
Only typed activation-history readers require a stamp. Program mapping, cloning
and instruction replacement retain it, while combining independently stamped
programs clears the ambiguous identity. A transformed or flipped source at the
same printed slot, including a borrowed donor that transforms, has a different
definition identity. Returning to the original face recovers its own earlier
history. The resolving copy uses the captured stamp even after the source or
donor changes face. A native or legacy threshold program missing its stamp fails
closed instead of being matched against the source's current definition.

Ordinary activation owners capture that origin before payment and increment
the ledger when the activation completes, before pushing its stack entry.
Countering it later does not erase an activation. Direct, pending and
payment-planner mana owners count after their costs complete and before their
immediate effects. Four stacked activations therefore each observe four when
they resolve; a fourth mana activation observes itself. Copies retain the
origin through stack-copy construction and execution projections but never
increment the ledger. Control changes preserve it. A new ObjectId does not
inherit it, and a new turn explicitly creates a complete empty ledger.

Native game/context checkpoints retain the ledger and origin. Existing
transaction owners restore cost payment, count publication, mana production
and delayed registration together after errors or pending decisions. Exhaust's
contribution cancellation removes the matching current-turn contribution as
well as its pre-existing lifetime contribution. Missing origins or absent
historical ledgers return IncompleteEvidence, including when the required
threshold would otherwise be compared against zero.

## Delay and sacrifice actor

The existing next-end-step owner retains the original source incarnation;
resolution source lookup does not follow an unrelated blink. Imperative source
sacrifice now carries PlayerFilter::You explicitly, so the controller of the
resolving delayed ability can sacrifice only a permanent they control.
Changing control before registration or before the end step does not rewrite
that actor. A copied activation may have a different delayed controller while
reading the original acquisition's activation count. The threshold is not
rechecked at the end step or after a turn reset.

Flying and Nalathni's entire Banding line remain present. Native combat owners
already validate a band with at most one nonbanding member, propagate blocking
to the whole band, and assign the banding player's damage-choice authority.
The authored full-body scenarios use those owners, including Flying exclusion
and Banding's exceptional blocking/assignment behavior.

## Current recovery boundary

At base `3b0b27470d01e0f056a6088a6baa5bea47e5195e`, commit `2511818a2`
already removed serialized gameplay recovery checkpoint exports/imports.
Older architecture notes about generic-history empty wire carriers describe the
superseded boundary. No lossy gameplay export or import is reintroduced here.
RuntimeSavepoint retains GameState and inactive Grand Melee lanes; accepted
transcript replay reconstructs activations. Public audit checkpoint 3 and
signed audit protocol 19 remain distinct digest/protocol surfaces; neither is
an executable gameplay restore representation.
`web/ui/tests/runtime-identity-origin.wasm.test.mjs` already asserts the removed
methods are absent. Public claim ObjectSnapshot serde omits acquisitions and
cannot be used to invent a resolving activation identity.

Artifact format 6 remains unchanged; the new condition is appended, preserving
previous serialized discriminants. The optional activation-definition field is
omitted when absent, and legacy absent values remain unknown. This predicate was
unsupported in historical artifacts; no old successful reader depends on a
manufactured stamp. Ordinary and mana activations without the optional stamp
still execute and are recorded with an unknown definition component; only the
new same-ability history reader requires that evidence. No regeneration or
format bump is imposed on previously supported bodies. Independent strict direct compilation and
artifact compilation/JSON rematerialization are authored for all four bodies.
Native full-body scenarios cover three versus four stacked activations,
countered activations, copied abilities and copied controllers, both actual
mana activation owners, nested spell payment, cancellation, failed payment,
pending suffix rollback, control changes, source departure and blink, turn
reset, independent grants, changed display slots, exact delayed sacrifice,
Flying, and Banding. Wasm scenarios cover root/inactive-lane native restore and
missing origins after public-claim serde. All are unrun.
