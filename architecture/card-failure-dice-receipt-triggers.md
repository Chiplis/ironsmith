# Qualified die-roll receipts

UNVALIDATED source work; no builds, compiler probes or tests executed.

Six exact frozen identities are retained in `fixtures/dice_receipt_triggers.json.fixture`:
Atomwheel Acrobats, Critical Hit, Resolute Veggiesaur, Monoxa, Midway Manager,
Farideh, Devil’s Chosen, and Vexing Puzzlebox. All six are proposed complete after bounded independent source review through
`c7f209cc7`. They remain unvalidated; authored scenarios have not run.

## Typed scope

Use the existing numeric and natural die results, adding result comparisons and
completed turn ordinals. Numeric result conditions belong to the triggering
receipt, not a later roll or all earlier rolls. One-or-more dice bodies retain
the full same-player batch for a total or any-result predicate; individual-die
triggers retain the specific die result. A natural 20 remains different from a
modified 20 or the highest natural result of another die.

The producer must count actual completed rolls, including planar rolls for
non-numeric ordinal triggers, excluding ignored and superseded rerolls. Choosing
one result without ignoring the others does not erase those other rolls. Existing
choice/replacement instructions retain their own procedure. Adjacent Barbarian
Class/Wyll dice-replacement text and Mr. House’s Treasure-payment body remain
separate unclaimed candidates.

CR706.2 distinguishes natural/modified results; CR706.6 excludes ignored rolls;
CR706.7 keeps planar roll triggers but no numeric result. The official
[Unfinity release notes](https://magic.wizards.com/en/news/feature/unfinity-release-notes-2022-10-07)
confirm that third-die triggers work when several dice are rolled together,
and the official [Baldur’s Gate notes](https://media.wizards.com/2022/wpn/w22/EN_MTGCLB_ReleaseNotes_04282022.pdf)
confirm Puzzlebox uses actual nonignored numeric results and retains a real mana
ability despite its die roll. These are source rules, not runtime text parsing.

## Implemented review checkpoint

New core/semantic trigger variants are appended, preserving prior serialized
ordinals. Numeric set/threshold comparisons and natural-result comparisons remain
distinct from turn ordinals. Every real roll producer captures the ordinal only
after its choice/modifier procedure completes. Numeric history and total physical
roll history stay separate so planar faces cannot satisfy numeric predicates.
Single and choose-result effect owners now restore on suspension/failure.

Grouped receipt evidence retains each player's complete nonignored numeric
results. New event-value operations express the batch total and number of results
at least a threshold. Ordinal triggers do not claim a particular physical result
in a simultaneous set. Bare result binding requires compatible typed die context;
a real local result producer takes precedence, and incompatible union arms refuse
the event fallback. Dice do not seed an implicit object subject from the object
that happened to produce the roll.

Authored native/grammar/runtime scenarios cover all six full direct/artifact
bodies, paid rolls, modified versus natural 20, exact graveyard incarnation,
source/roller/player separation, grouped sum/any predicates across later rolls,
third-die batch/turn/native-restore behavior, actual planar rolls, real paid
rerolls, and Puzzlebox's mana ability and 100-counter search. No test was run.

Existing generic recovery refuses unreproduced event/history and program state;
these native receipt and ordinal facts remain retained by GameState savepoints.
The extra-dice/ignore-lowest replacement procedure is still a separate partial
family; choosing one result without ignoring the other physical rolls remains
semantically distinct.

### Typed turn-action resource boundary

Planar and Attraction rolls preflight ordinal capacity before consuming random
state. The planar direct owner is atomic and returns ExecutionError; the special
action query/executor preserves incomplete execution rather than invalid timing.
The Attraction public owner preserves its queue and native state on suspension
or error and retains GameLoopError::ExecutionFailed. Authored real enabled-deck
scenarios pin no consumed forced result/action count, typed failure, and retry.

### Local result precedence

“The roll was N or higher” is a typed pending roll-result query. An actual local
RollDie/RollDiceChooseResult producer binds its exact effect identity before a
singular triggering result is considered; intervening unrelated actions cannot
steal that identity. Re-annotation retains the same producer ID. An enclosing
instruction without a compatible exported scalar explicitly blocks ambient
fallback. A grouped “any of those results” predicate requires a proven grouped
trigger with no superseding local roll. Local batch-result references remain
unsupported and fail closed until their result-set carrier is represented.
Standalone, mixed original/local-trigger, nontrigger, grouped, and repeated
annotation regressions are authored for this boundary and remain unrun.
