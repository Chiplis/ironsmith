# Extrema and ties: bounded source checkpoints

Status: UNVALIDATED. No build, compilation, test or compiler probe was run.
The exact frozen cohort has 12 identities in `fixtures/extrema_quantities.json.fixture`.

## First three complete source proposals

- Freelance Muscle: the maximum of greatest current power and greatest current
  toughness among other creatures its ability controller controls. The typed
  expression uses existing canonical maximum arithmetic, preserving scope and
  resolution-time characteristics. The runtime scenario changes an ally's
  power after the trigger and checks an empty eligible scope and greater enemy
  axes separately.
- Investigator's Journal: greatest one player's creature count, partitioned by
  controller over the eligible battlefield set. The explicit “a player controls”
  suffix sets the partition. This does not capture existing specialized shared
  creature-type subset reductions. Both printed activated draw costs have real
  payment/removal/sacrifice scenarios.
- Repay in Kind: existing LowestLifeTied scalar reference represents the global
  minimum. Tied players have the same number; no player choice is inferred.
  Existing ForPlayers/SetLifeTotal simultaneous proposals freeze all amounts
  before committing any life change. An authored replacement scenario doubles
  the first player's loss, creating a new lower total, while later participants
  still use the original minimum. Opponent-scoped minimum syntax remains
  unclaimed rather than silently broadening to all players.

## Next three complete source proposals

- Wretched Banquet: the authored creature target is unrestricted by the
  later condition. At resolution its current power is compared to the current
  minimum, including ties. Tests announce a nonminimum target, change power in
  response in both directions, and preserve the unchosen tied creature.
- Strength-Testing Hammer: its referenced equipped attacker is checked after
  the real roll-dependent pump. Equality includes every tied maximum without
  selecting a winner. Scenarios exercise real equip/attack/roll/draw and expiry.
- Dispersal Shield: a complete typed mana-value comparison owns its predicate
  before descriptor/disjunction probes can misinterpret it. Its spell target
  remains legal even when the condition is currently false; resolution rechecks
  the maximum after a real permanent destruction response.

The generic complete-value reader now excludes inapplicable demonstrative and
OR fallback probes. These probes do not recover a suffix or suppress diagnostics;
they are simply not dispatched when the whole input already has a typed owner.
Typed full-registry tests assert non-lossy predicate recognition.

## Four single-object tie selections, complete source proposals

Desecrator Hag; Drop of Honey; Porphyry Nodes; Purging Scythe now bind their
trailing “If two or more ... are tied ..., you choose one of them” clarification
to the preceding typed, untargeted extremum action. A real mandatory choice of
one eligible object precedes that action; it never operates on all ties then
runs an additive conditional. Wrong-axis, unrelated, targeted or quantified
antecedents remain unclaimed. Existing no-regeneration flags stay on the actual
destroy operation. The typed comparison retains the explicit tie wording.

Runtime scenarios cover own-graveyard maxima (including negative powers),
least-toughness damage, an empty eligible set, a hexproof tied choice and an
actual regeneration shield. Drop/Nodes also exercise the existing StateBased
trigger owner: only one instance may be pending, and the empty-battlefield
triggering condition is not rechecked as an intervening-if after a new creature
enters in response. Its real sacrifice body still resolves.

## Cabal Conditioning composition, complete source proposal

The existing generic any-number-of-target-players reader and shared scalar
aggregate reader compose the complete body. A normal full-registry regression
pins zero-to-unbounded declaration cardinality; a full runtime scenario targets
two players, removes the caster's largest permanent in response, and preserves
both the caster-relative amount and the unchosen player's hand. A zero-target
variant and a target with fewer cards exercise ordinary do-as-much-as-possible
resolution. No new marker or special-case card parser was added.

## Gor Muldrak tied-minimum participant set, complete source proposal

The appended `PlayerFilter::ControlsFewestTied` selects every in-game player
whose current matching battlefield count is minimal within the effect's range.
Zero counts and all ties are included. The ordinary ForPlayers owner captures
this participant set before any token creation changes the counts. A sequential
per-player condition would incorrectly admit a later player after earlier tied
players gained tokens. Current control is read rather than ownership.

The bounded relative-clause reader accepts an authored `each player` actor and
an unowned battlefield object scope. Other participant scopes remain rejected
rather than silently changing the comparison group. Existing typed token and
protection abilities execute the complete body. Authored direct/artifact cases
cover a positive tie, zero-count tie, a real control-change response, player and
permanent protection, and protection moving to the source's current controller.
An engine scenario checks actual list-adapter agreement, range exclusions and
players leaving the game. All twelve exact identities are source proposals;
none of these new regressions has been executed.

## Hammer exact-incarnation correction

Source review found a pre-existing shared reference seam: attack/block event
identity was treated as permission to follow a tagged creature's stable card id
after a blink before resolution. Combat declarations do not move objects, so
those event kinds now deny that cross-incarnation follow. True in-resolution
movement and explicit zone-change event permissions keep their existing paths.

An ordinary tagged characteristic condition now reads the permitted current
object exclusively; an older matching tag cannot override a failed current
check. When that incarnation is gone, it reads the exact true departure receipt
before the saved tag. Explicit past-state predicates retain their separate owner.
The triggering-object path likewise reads true departure receipts, including
owner departure, without confusing a phasing observation for departure.

Authored scenarios cover Hammer attack, a power change after the attack, then
exile/return before resolution: the new incarnation gets no pump, and the
condition uses actual departure power. A separate spell performs an explicitly
linked exile/return in its own resolution. Engine cases cover all four combat
observation kinds and an old matching snapshot versus failed live comparison.
All remain unrun.
