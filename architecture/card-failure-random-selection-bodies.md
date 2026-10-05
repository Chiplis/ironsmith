# Random selection bodies: bounded source checkpoint

Source base: `eb3122bd9d36866cea111a2cfc76e245d4a3792e`.
Frozen inputs: `fixtures/card-failure-campaign/cards-20261003.json.xz` and
`baseline-e8740178.snapshot.json.gz`. The dedicated
`fixtures/random_selection_bodies.json.fixture` preserves all eight original
card objects and complete Oracle bodies, including every body retained for further owner work.
No coverage matrix or frozen baseline is changed.

## Current source disposition

Bounded source review has cleared Moldgraf Monstrosity, Tomb Tyrant, Singe-Mind
Ogre (including its public-opening correction), Kheru Lich Lord, Sinister Waltz,
and Nebuchadnezzar. These are source-proposed full bodies, not executed or
validated corpus results. Goblin Test Pilot and Witch Hunt remain held for
announcement-time random target commitment/replay ownership. The sections below
retain the individual checkpoint traces and superseded holds.

## Completed source work (not execution-validated)

- **Moldgraf Monstrosity**: preserve the recognized random selection on a
  two-card graveyard return; choose from the resolving pool without replacement,
  after the source-exile instruction. Retain its trample body and death trigger.
- **Tomb Tyrant**: preserve the recognized random selection on the Zombie
  graveyard return. Retain the separate anthem, mana/tap/sacrifice costs, active
  turn restriction and minimum graveyard Zombie condition.
- **Singe-Mind Ogre**: current source already has the typed random hand choice,
  public reveal and exact revealed-card mana-value follow-up. This checkpoint
  adds frozen-full-body direct/artifact coverage and execution assertions for
  zero, one and several hand cards; no new implementation is claimed for it.

The return shape reader removed `at random` into `ReturnClauseShape.random`,
but the battlefield branch ignored that flag. Only the return-to-hand branch
used it. The fix carries the flag in `TargetAst`'s existing `ChoiceCount`, not
in a render label or a new whole-card recipe. Lowering uses the existing
resolution-time `ChooseObjectsEffect` for random graveyard counts and tags its
chosen objects for the native return operation.

The plural choice renderer independently omitted the random flag. It now
renders existing typed metadata for fixed and runtime counts, and the exact
adjacent graveyard choose/return compactor accepts a fixed random count. These
changes do not suppress semantic quality checks.

Native direct random selection now clamps X to available candidates and treats
an empty random pool as a completed empty choice. It continues using
`GameState::shuffle_slice`, the existing transcript-seed authority and
`HiddenInfoOperation::FairRandom` path; no local RNG or caller-picked result was
added. Existing native return and choice checkpoints remain their transaction owners.
For the compiled choose/return pair, the whole resolution-program checkpoint
(`execute_resolution_program_with_trigger_matching_typed`) and enclosing stack
checkpoint restore the pre-choice state on pending/error. A full direct/artifact
regression suspends an as-enters choice after the random draw and checks that
the queued seed, random counter, object IDs, graveyard and stack all roll back
together before a successful retry.
No random authority, hidden replay foundation, coin event or resource-budget
owner is modified.

## Authored regression coverage

- Local return-clause AST cardinality, ownership, zone, subtype and non-target
  distinction.
- Direct and serialized/rematerialized full cards for the three bodies above.
- Real death-trigger/source-exile then return, including empty and insufficient
  graveyards, owner/type filtering and distinct selections.
- Tomb Tyrant's live anthem, actual announcement costs, activation restrictions,
  and resolution-time random pool (including the sacrificed creature).
- Ogre's public reveal to both players, hand preservation and exact selected
  mana value, including an empty hand.
- Transcript authority overriding different local random seeds.
- Whole compiled-program pending entry and retry, covering both the random
  choice and linked return under the actual stack/resolution checkpoint.
- Native random return with fixed and X counts, empty/insufficient sets, pending
  entry choices, post-draw errors, game/RNG/ID rollback, and a successful retry
  retaining the same queued transcript seed.

All tests are authored and **unrun**. No build, compiler probe, formatter,
corpus execution or test command was run. Only source inspection, fixture
extraction, edits and `git diff --check` were performed. Full-card behavior and
strict quality status require the deferred campaign validation pass.

## Held full bodies at the initial checkpoint

- **Goblin Test Pilot**: a random *legal target* must be selected during ability
  announcement. Ordinary target legality and later resolution must retain that
  selected target; rerolling during resolution is incorrect. A `ChoiceCount`
  flag alone does not supply the missing announcement owner.
- **Witch Hunt**: same announcement-time random target requirement for its
  end-step trigger, plus retention of the life-gain prohibition and upkeep
  damage. Do not replace its target with a resolving opponent choice.
- **Sinister Waltz**: three announced legal graveyard targets must first be
  revalidated, then partitioned into up to two distinct randomly returned cards
  and the exact surviving remainder. Its remainder cannot be reselected from
  the whole graveyard or include a now-illegal target.
- **Nebuchadnezzar**: the chosen name, paid X, randomly revealed hand subset,
  publicly opened hidden identities and name-matching discard of only that
  revealed subset need a complete joined execution/replay trace. Its turn-only
  activation restriction remains part of the held body.
- **Kheru Lich Lord**: the random return clause benefits from the generic fix,
  but full-body credit is held for the payment branch, flying/trample/haste on
  the exact returned incarnation, controller-owned next end step, leave-zone
  replacement, and stale/source-left/changed-controller rider expiry checks.

These are genuine retained implementation work, not rejected-shape placeholders
or successful coverage claims. The random target owner and the resolving-set
partition are deliberately not conflated with the existing random object-choice
primitive.

## Kheru follow-up checkpoint (source reviewed, execution unvalidated)

Source inspection found the remaining rider owners already present:
`future_zone_replacement_from_sentence_tokens` binds the leave-zone rider to the
last returned object and Persistent duration; lowering emits
`RegisterZoneReplacementEffect` with Resolution lifetime. Registration freezes
the specific returned object ID, so it survives the source's departure without
following a new incarnation. The generic delayed scheduler captures the
resolution controller and pins tagged objects to their current incarnation.
The ability grant has no authored duration and must persist if the delayed
exile is countered.

A separate full-body direct/artifact module now asserts payment/decline/empty
pool, exact returned-card grants, foreign graveyard exclusion, source departure,
changed returned-card controller, next end step ownership, all leave destinations,
blinked incarnation expiry, unrelated permanents and a countered delayed exile.
No new production rider encoding or lifetime exception was added. Bounded
source review cleared checkpoint `6693c17db`; Kheru is source-proposed and no
longer held. This supersedes its initial hold above. Execution and strict
quality validation remain deferred; no successful test or corpus result is
claimed.

## Surviving-target random partition checkpoint (unrun, awaiting review)

The named `declared-graveyard-random-return-complement` pair rule reads the
announcement and its completing partition together. It emits existing typed
primitives: a tagged explicit target declaration, random selection from that
surviving set, a captured set difference, and the two zone moves. Capturing the
complement before movement ensures a prevented or redirected selected return
never becomes a member of “the other.” No card-name test or bespoke whole-card
AST/runtime operation is introduced.

The native explicit target declaration previously enforced its original count
again at resolution. It now follows the existing synthetic declaration rule:
announcement requires the full authored count, but only surviving legal targets
populate the result (CR 608.2b). Zero survivors still invalidate a required
explicit declaration, and the stack owner handles the all-targets-illegal case.

The text compactor verifies all five operations and exact pool/subset/complement
links before presenting the combined partition. Full Sinister Waltz direct and
artifact tests author a real three-target cast, zero through three lost targets,
fresh graveyard incarnations, untargeted/foreign exclusions, and prevented
returns retaining the originally selected subset outside the complement.
Local grammar, native declaration and structural-render tests are authored.
Sinister remains uncredited pending bounded review and deferred execution.

Sinister review refinements: the reader verifies complete token ownership of
its completing statement and rejects extra symbols/instructions; TargetOnly
preserves typed resolver failures rather than converting them to illegal-target
outcomes. The final complement move carries the existing `SameObjectId`
relationship and its original graveyard. This excludes a new incarnation even
if a replacement-added effect moves the complement during the same resolution.
Authored cases cover both an exile destination and a blink back to the graveyard.

Bounded source review cleared Sinister through `79e2d98b6`, superseding its
initial hold. The exact complement is lowered as `ChooseSpec::All` so zero
surviving members complete without error. Sinister is source-proposed with
execution and strict quality validation still deferred. No runtime or corpus
success is claimed.

## Tagged public opening correction (unrun, awaiting review)

Ogre's earlier source clearance is temporarily held for a newly traced hidden
peer seam. The random generic-hand choice is identity-free, so it does not open
cards through `settle_hidden_hand_random_pool`. The common execute-effect
preflight settles `All` filters/TagMatchingObjects, not RevealTagged; the latter
previously issued only view callbacks and carried pre-opening snapshots.

RevealTagged now uses the existing mandatory owner-answered Public opening for
exactly its selected private IDs, suspends before publishing callbacks/receipts,
rejects an unopened peer placeholder, refreshes both its original tag and
PUBLIC_REVEALED_TAG, and rolls back context/state on pending/error. Each revealed
card receives a distinct event provenance. It no longer claims to be a read-only
simultaneous preparation because public disclosure changes the information state.

Native owner/peer tests cover a pending opening, exact-set privacy, snapshot
refresh and a missing peer opening. A full Ogre direct/artifact real-entry test
covers the owner's mandatory opening on owner and peer engines, random authority
rollback/retry and mana-value evaluation only after the selected identity opens.
This adds no protocol, RNG authority or model/wire variant.

## Counted random hand reveal and named subset checkpoint (unrun, awaiting review)

The random hand-reveal grammar previously required an article and always
constructed a one-card count. It now retains fixed/X counts, requires a complete
unqualified card descriptor rather than silently dropping qualifiers/symbols,
and passes the actual count to the existing random ChooseObjects operation.

The named `random-hand-reveal-named-subset-discard` pair rule preserves the
following all-discard as `Value::Count` of exactly the same filter used by
Discard: original revealed ObjectIds in the hand plus the independently stored
ChosenName relationship. The generic selected-discard owner now recognizes the
shared membership relations (including SameObjectId), preserving the existing
no-second-choice path for complete selected sets. Source-persistent names never
replace the resolution's current name tag.

A structural renderer checks both identity relations and matching count/player
before compacting the named reveal/discard. The generic random hand renderer
retains fixed and X counts, placing the random modifier after the card noun.
Full Nebuchadnezzar independent direct/artifact cases author real X payment and
tap costs, own-turn restrictions, distinct random samples, 0/1/2/oversized X,
empty hands, source departure, repeated activations with different names,
matching versus unmatching revealed names, and hidden owner/peer Public-opening
suspension/retry using the same transcript random seed. This checkpoint is
source work only and does not itself promote Nebuchadnezzar from its hold.

RevealTagged's simultaneous support is retained through an immutable proposal:
preparation captures each already-selected set without opening or publishing it;
commit runs the normal opening/reveal transaction only after the outer owner has
collected all proposals. No read-only preparation bypass remains. A native
proposal contract asserts the fixed per-player sets, no preparation disclosure,
correct resulting memories and the combined public tag.

The new mandatory Public selection retains the existing cost-payment disclosure
classification: `payment_disclosure_transaction.rs` uses the active payment
subject plus SelectObjects/Public and actual hand candidates for its commit and
retry journal. The opening uses that same Public decision route and source;
no separate disclosure journal, cancellation exception or WASM route is added.

Bounded review cleared the RevealTagged correction through `25a269046` (with
source-neutral test baseline follow-up `7e5b67a45`). This supersedes Ogre's
information-opening hold and restores its source-proposed disposition after
additive integration. Full execution remains deferred. The native owner now
preserves simultaneous capability and the existing payment-disclosure path.

Bounded source review cleared Nebuchadnezzar `88e48751e` together with the
reviewed RevealTagged correction. This supersedes its initial hold. Its random
subset, independent chosen name, original Hand/ObjectId membership, complete
discard and paid-X body are source-proposed; execution remains deferred.
