# Scoped life quantities and conditions

Status: **UNVALIDATED source proposal** on base `1b7ea49a`. No builds, compiler
probes or tests have run. The exact ten frozen sources are preserved in
`fixtures/life_total_quantities.json.fixture`.

## Membership and expected change

Initial checkpoint: nine proposed-complete cards: Anya, Merciless Angel; Arbiter of Knollridge;
Cosmos Elixir; Elenda, Saint of Dusk; Game Over; Malignus; Psychic Transfer;
Resolute Archangel; Scourge of the Skyclaves. These are frozen stack07 failures,
not measured successes. The expected after state is strict metadata-bearing,
nonlossy compilation and executable direct/JSON-restored definitions.

The separate noted-life follow-up closes the initially partial Sigarda's
Splendor source proposal, taking this exact fixture to ten proposed-complete
cards. The receipt correction is described below. No other nearby life-total
card is added to the proposed count; none is measured validated coverage.

## Typed quantities and evaluation

Two serialized Value variants are appended: `MaximumLifeTotal(PlayerFilter)`
and `CountPlayersBelowHalfStartingLifeTotal(PlayerFilter)`. Maximum over your
opponents is distinct from maximum over all players. Only in-game matching
players count, negative maxima remain negative, ties do not produce a choice,
and an empty aggregate is zero. The continuous adapter now has a dedicated
aggregate player lookup; its existing required scalar lookup still rejects a
missing required player rather than silently substituting zero.

The half-starting threshold compares twice the current total with the actual
starting total in widened arithmetic, preserving strict versus inclusive
conditions for odd and negative totals. Half rounded up reuses the existing
`HalfRoundedDown(Add(value,1))` encoding and widened fraction interpreter.
Count-specific anthem rendering remains intact; only the supported new scalar
forms enter the existing capability-guarded Dynamic fallback. Player-scope,
choice, dependency and turn-context visitors retain the new filters.

The absolute-difference encoding is recognized before potentially overflowing
intermediates. Representable scalar results are returned exactly; results
outside i32 remain explicit errors. Numeric condition comparisons use checked
i64 intermediates for the existing Add, Scaled, Min and rounded-division nodes,
so a difference greater than i32::MAX can correctly fail a <=5 predicate, and
starting-life+10 need not overflow to answer a boolean comparison. Unsupported
leaves still use the ordinary typed resolver. This does not introduce arbitrary
precision life, counts or damage, and checked i64 comparison overflow remains
an explicit error rather than clamping.

## Reference and condition boundaries

Psychic Transfer declares exactly one authored target player outside its life
condition. The target is selected even if the condition is false during
announcement; the existing conditional reevaluates current life on resolution.
That explicit declaration also provides the following `that player` antecedent.
Repeated references in the absolute-difference tree share the slot. Anaphoric
`that player's` alone does not create a new target. Multiple distinct targets
and outer replacement `instead` in this bounded reader reject.

Resolute Archangel's numeric `it` is admitted only after a typed intervening-if
whose subject is your LifeTotal. It emits a real SetLifeTotal action and retains
both entry-time and resolution-time intervening-if checks. Missing predicates,
object-power subjects and unconsumed instruction tails are not guessed.

Dynamic life predicates now accept starting-life and offset references. The
existing literal-life and established at-least-starting/noted predicate readers
keep their original ownership. Article-bearing `a player's` and `an opponent's`
subjects preserve Any and Opponent scope. Existing live condition and life-total
write invalidation paths are exercised without manual cache refresh.

## Full bodies and authored, unrun scenarios

The normal compiler-runtime suite contains the nine-source direct/JSON strict
aggregate plus gameplay for every proposed card. It checks Anya's changing
opponent count, indestructibility and controller; Malignus's scoped rounded
size and actual unpreventable damage; Arbiter's original maximum captured for
all recipients despite one player's replacement-doubled gain; Elenda's two
thresholds and controller-specific starting life; Game Over's real reduced
payment and creature destruction; Psychic Transfer's announcement and response
timing; Resolute's captured trigger controller and intervening-if; Scourge's
real kicker payment, odd rounding and live global maximum; and Cosmos's
resolution-time draw/life branch and independent trigger controller. Public
life actions, casting, pending triggers and controller changes are used.

Shared interpreter properties cover scoped and tied maxima, negative and empty
sets, continuous/execution agreement, signed extrema, canonical rounding and
absolute differences. Grammar properties pin exact scalar trees and bounded
pronoun/target ownership. The normal tools target initially checked nine complete metadata-bearing
payloads and now checks ten with the separate Sigarda follow-up.

## Separate follow-up: exact noted-life receipts

Sigarda's frozen error was also a dispatch omission: the existing Verb::Note,
typed NoteLifeTotal action and executor were unreachable through generic chain
recognition. ChainVerbKind now appends Note and dispatches the imperative into
the existing complete `your life total` reader. Unsupported subjects and tails
still reject; a grammar regression checks both routing and the negative cases.

ObjectSnapshot and RetainedObjectSnapshot append an optional noncopiable noted-
life receipt. Constructors, compact-memory fallbacks and the retained payload
mappers preserve it appropriately. Old serialized snapshots may omit this new
field and restore unknown (`None`), never an invented zero. Retained nested
snapshot round trips include a negative note and legacy omission coverage.

LastNotedLifeTotal first reads the exact source's live or explicitly re-noted
annotation. If that object is absent, it reads its exact departure snapshot,
then matching pending-source LKI if departure history is no longer present.
An earlier selection snapshot cannot supersede the actual departure receipt,
and stable-card identity never authorizes following a return. Removal still
clears the live annotation. Note writes (including prepared as-enters notes)
now invalidate both the cached object snapshot revision and continuous state;
a warmed-cache regression isolates this write from the preceding life change.
The existing note executor may subsequently note
again under the old exact source ObjectId; that deliberate instruction updates
later pending copies for that incarnation but cannot touch a returned source.

Two additional normal runtime scenarios cover both draw outcomes, subsequent
noting, an actual white-spell cast/life-gain trigger, and actual ability copying.
They resolve the copy before or after the source departs, return the same card
with a different new note, and verify the independent original trigger uses the
correct older incarnation's latest note. Shared interpreter coverage proves
actual-departure-versus-earlier-snapshot selection and rejects wrong-identity
receipts. All are authored and unrun. This source proposal does not claim a new
general system for arbitrary linked annotations or unbounded numeric values.


The owner-departure follow-up retains the exact source snapshot for pending
stack and pending-trigger owners before all CR800.4a removals, including
phased-out owned objects. Ordinary zone departure reuses the same identity-
and-zone guarded helper. LastNotedLifeTotal consults true departure receipts
(ZoneChange or ObjectLeavesGame), not a stale phase-out snapshot, then the
retained source receipt. A three-player borrowed-Sigarda regression resolves a
copied trigger's newer note before its owner leaves and verifies the remaining
controller's draw both with and without departure history and phasing.
