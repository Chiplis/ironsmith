# Random selection bodies: bounded source checkpoint

Source base: `eb3122bd9d36866cea111a2cfc76e245d4a3792e`.
Frozen inputs: `fixtures/card-failure-campaign/cards-20261003.json.xz` and
`baseline-e8740178.snapshot.json.gz`. The dedicated
`fixtures/random_selection_bodies.json.fixture` preserves all eight original
card objects and complete Oracle bodies, including every body retained for further owner work.
No coverage matrix or frozen baseline is changed.

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
