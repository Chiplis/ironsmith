# Die arithmetic and complete result programs

## Current NEXT03 disposition

Independent final source review cleared `92168f50060c362c88d5c1ecf6477dbbffa11839`. The [NEXT03 admission](card-failure-next-series-03-source-admission.md) records only the bounded source proposals: predicate 5, copy 4 IDs / 5 entries, suspended 4, numeric 4, plural untap 5. All held neighbors remain excluded. All executable scenarios are **UNRUN**, the source remains **UNVALIDATED**, and no new measured recovery is claimed. The historical scoped-work notes below describe their original stages; coordinated compatibility is now artifact 14 / digest 9 / audit 27.


## 2026-10-07 follow-up ownership and announced-X correction: UNVALIDATED / UNRUN

Independent source review of `9295d0584` found three remaining blockers: an
earlier roll could lend ownership to a table after a draw; a die activation
under Station could consume the next actual striation; and paid X was never
seeded from mana costs and could still become the numeric result in lowering.
The source corrections now address those specific paths:

- Printed numeric headers retain compiler-only `IfResultPredicate::DieValue`.
  The immediate predecessor must be a compatible die instruction (or a sibling
  row of that same table). Reference resolution binds the exact exported die
  ID, rather than an arbitrary latest result, and rejects missing, stale,
  conditional, optional, and quoted owners. Ordinary non-die numeric result
  predicates retain their existing behavior.
- Sequential search/reveal/roll owners retain their wrappers, with following
  die rows moved into the terminal roll's local program. Source-wrapped rows
  retain their provenance. Reannotation reuses a resolved row's producer ID
  through singleton source/sequence wrappers, including a nonzero ID allocator.
  A nested roll in one consequence does not change its sibling rows' owner.
- Every statement, triggered, and activated continuation path stops at an N+
  row already owned by a preceding Station keyword. The intervening rolling
  ability retains the earlier charge threshold. A would-be die consequence
  whose N+ surface conflicts with Station is rejected by Station dispatch.
- Typed spell and ability costs establish announced X before effect preparation.
  Imports, exports, frames, preparation, and both value-binding paths preserve
  it. Optional/alternative spell costs are considered independently of printed
  line order; activated and modal costs stay in their own scopes. A permanent's
  printed X does not leak into a separate fixed-cost activation.

The exact original fixture bodies remain unchanged. Both independent public
routes, `compile_to_runtime_definition` and JSON-roundtripped/validated artifact
materialization, have authored full-body assertions for the four candidate IDs
listed below. New typed assertions require every table gate to name one exact
physical roll instruction, and Portent's two draws and scry to retain `Value::X`.
Native scenarios include zero and positive paid X, modified results distinct
from X, paid activated X, fixed-cost activation scope, stale/optional/conditional/
quoted negative owners, an earlier draw followed by a valid terminal roll,
row-local rolls, and the literal Station/3+/roll/9+ counterexample at 0, 2, 3, 8,
9, and 12 charge counters. The existing Druid, Song, Wyll, and Portent complete
body scenarios remain in place. These are source expectations, not execution
evidence or campaign admission credit.

No runtime effect schema or wire representation changes: the compiler-only
die predicate lowers to the existing numeric `EffectPredicate::Value`, and
announced-X metadata stays inside compiler reference scopes. Correctly compiled
outputs can change, so the separately owned compiler/cache boundary must still
cover this source revision. This worker changes no compatibility constants,
campaign ledgers, or fixture status markers. Revivify
`55eb3c21-6834-4c62-8ab9-54b901531fed` remains held for its distinct historical
collection-binding defect. No builds, tests, compiler probes, formatters, corpus
or analysis reruns, generation, or remote writes were performed.

## 2026-10-07 numeric table ownership repair: UNVALIDATED / UNRUN

Four original full bodies were re-held after the strict public compiler route
reported `parser could not lower station threshold line '15+ | Scry X, then draw
X cards.'`. The earlier source-review disposition below is historical evidence,
not execution validation or current campaign count credit for these four cards.

- Diviner's Portent: `119585c7-ddfa-47ed-b2f8-488ebc156222`
- Druid of the Emerald Grove: `acf54a85-0e9e-43fb-99d9-c223c02f13c4`
- Song of Inspiration: `bdb80d7b-672c-4e2a-b93b-9b96721b93f2`
- Wyll's Reversal: `8d35cef8-a52d-45fb-8f5f-cccea26826d0`

The proposed repair gives recognition and lowering one complete numeric-header
shape: exact numbers, ASCII/em-dash inclusive ranges, `N+`, and `N or less`.
It consumes the entire header through the pipe, rejects reversed or incomplete
headers, and lowers the two open bounds to existing `GreaterThanOrEqual` and
`LessThanOrEqual` comparisons without clamping to a die's natural face range.
Statements, labeled/unlabeled activations, and triggers attach numeric rows only
to a local unquoted die-roll owner. An orphan numeric row is an error. Station
retains the same `N+` surface, but document dispatch requires its actual preceding
Station keyword. Ordinary abilities under a Station striation do not consume
the next striation as a numeric result continuation.

The four fixture bodies and identities remain unchanged. Separate native test
entry points compile those exact fixtures with `compile_to_runtime_definition`
and, independently, with `compile_to_artifact`, JSON roundtrip, validation, and
materialization. The artifact compiler's companion definition is deliberately
discarded. Metadata checks retain exact costs, types, Druid's subtypes and 2/2,
the one enter-trigger envelope, and instant spell programs. Both routes exercise:

- Portent's announced/paid X separately from the modified die, including a
  result above 20 and high-row scry-before-draw;
- Druid's zero/one/two-card basic-land search, exact public reveal collection,
  every row's destinations, tapped battlefield entries, unselected library
  members, and one completed shuffle;
- Song's saved zero/one/two-target set, partial illegality and later incarnation,
  surviving mana-value sum, high-row life, and all-illegal no-roll fizzle;
- Wyll's saved spell or ability stack identity, greatest controlled power,
  independent optional retargets, mandatory high-row copy, departed source,
  targetless rejection, and target disappearance.

Full-body assertions include the unique die event's natural/modified result,
player, d20 size, completed ordinal, and turn history. Additional scenarios cover
zero/above-face-range predicates, malformed headers and unsupported row tails,
bare `N+`/`N or less` without a roll, non-die prior instructions, and the complete
The Eternity Elevator Station body at 0/19/20/23 charge counters. Grammar-level
controls retain the next separate ability and genuine Station striations.

Revivify `55eb3c21-6834-4c62-8ab9-54b901531fed` remains held for its distinct
historical-collection binding defect. This patch does not supply that collection
or change any other card's admission disposition. No compatibility constants,
campaign ledgers, fixture status markers, or wire schemas are changed. The wire
and runtime owners are existing numeric comparisons, conditional result
instructions, die receipts, and Station charge-counter gates; only their source
recognition/attachment is corrected. All new and extended scenarios are authored
but unrun. No builds, tests, probes, formatters, generation, corpus reruns, or
remote writes were performed for this repair.

## Earlier proposal and source-review record

Implementation-first source proposal. No builds, compiler probes, tests,
formatters, corpus runs, or external publication have been executed. Native
scenarios are authored and unrun. Independent source review is complete for the seven proposals and two prior
consumer restorations below, through 72c011bae, including the separate Song
scope. Execution validation and campaign count credit remain separate gates.

## Shared owners

- Both labeled activation routes append numeric rows through the ordinary
  activated followup owner. The label, full payment, all rows, and next ability
  keep their original envelopes. Typed statement fast paths likewise retain
  following result rows. Druid reuses the existing numeric-row/station repair.
- The die leaf requires complete tokens and exposes an explicit consumed prefix
  for the named arithmetic reading. That reading owns `roll ... and add/subtract`
  before coordination can interpret add as mana. Arithmetic rejects non-word
  operand tokens before the scalar reader can erase them.
- `DieResultModifier` carries the operation and value through AST, binding,
  lowering, serialization, materialization, rendering, and native execution.
  Mandatory arithmetic participates in the roller's numerical-modifier order
  after rerolls. It never draws another die. Natural result, modified result,
  chosen-number fact, exact local receipt, and completed turn ordinal agree.
  Negative effect results use zero; unrepresentable positive results raise a
  typed resource error, including external numerical modifiers.
- Consult stop filters participate in both numeric-dependency visitors, so the
  preceding roll exports its exact result before a mana-value filter is lowered.
- Reveal consultation opens each examined card publicly before inspecting its
  type/value or deciding to continue. A missing authenticated identity is typed
  incomplete evidence. The untouched suffix stays private. Completed empty
  match sets are explicit, and a present empty characteristic set is known zero;
  absent evidence still errors. Pending/error rollback includes the whole
  resolution program and retains only actual pending-decision routing.
- Each consulted CardRevealed observation and each singular DieRolled instruction
  receives a distinct child event provenance. Dispatch, staged history, native
  trigger matching, and repeated publication preserve every original occurrence
  without duplication.

Rules source: https://media.wizards.com/2026/downloads/MagicCompRules%2020260925.pdf
(CR 706.2–706.3, 107.1b, 107.2, and 608.2).

## Full bodies independently source-cleared

1. Arcane Investigator: complete labeled six-mana activation; low draw row and
   high private three-card partition, exact chosen card to hand and rest to bottom.
2. Herald of Hadar: complete labeled six-mana activation; opponent loss in every
   row, controller gain in middle/high rows, both Treasures only in the high row.
3. Druid of the Emerald Grove: full enter trigger, up-to-two basic land search,
   public reveal, all three rows, tapped placement, remainder to hand, shuffle.
   This supplies independent full-body coverage for the existing envelope fix.
4. Diviner's Portent: paid and announced X stays independent of the roll; count
   the hand after casting; high row scries X before drawing X.
5. Bag of Tricks: full mana/tap activation, d8 result used as the creature-card
   mana-value filter, ordered authenticated reveal until first match, battlefield
   placement, and random remainder. No-match and hidden-identity boundaries included.
6. Wyll's Reversal: saved spell/ability target with one or more targets; current
   greatest controlled-creature power; independent optional original/copy
   retargeting and mandatory copy. Its terminal stack-target cardinality is an
   outer qualifier even for exact early-return ability-filter heads.
7. Song of Inspiration: zero/one/two saved graveyard permanent targets; arithmetic
   and high-row life gain sum the exact surviving set. A reusable `their total
   mana value` scalar reads that prior set. Partial illegality, a later incarnation,
   and all-illegal fizzle are covered. This is a separate continuation scope.

The frozen bodies and identity list are in
`fixtures/die_result_programs.json.fixture`. Independent direct compilation and
JSON artifact roundtrip/materialization scenarios are in
`crates/ironsmith-compiler-runtime/tests/die_result_programs.rs`.

## Confirmed prior consumers of the reveal correction

Erratic Explosion and Explosive Revelation enter the declared mixed-target
whole-document reader, lower Reveal-mode ConsultTopOfLibrary, and use the exact
same runtime owner. Their older full-body scenarios used known identities only;
the former owner tested their nonland stop filter before public authentication.
The correction includes independent native full-body scenarios for both cards:
player/permanent recipients, pending second opening/replay, missing identity,
exact hit versus full-reveal aliases, hand/bottom destinations, private suffix,
empty library, and all-land library. Their earlier source proposals require this
shared correction to be reviewed. No blanket hold is inferred for uninspected
consumers.

## Held candidates

- Danse Macabre: its actor-qualified singular sacrifice characteristic needs an
  exact per-player original sacrifice receipt. The current quantity grammar has
  not established the complete `the toughness of the creature you sacrificed
  this way` operand. The two graveyard result-row selection scopes also need
  full native coverage; generic arithmetic is not enough.
- Gale's Redirection: `that spell's mana value` must retain the original stack
  incarnation's X and face after exile. Ordinary move tags can point at the new
  exiled card, while pre-move history is retained separately. No clearance is
  claimed without binding that original stack value and testing both duration,
  any-color/free-cast permission tails and spell-copy behavior.
- Revivify: high-row `those cards` needs the complete battlefield-to-graveyard
  historical collection, although the low row's action did not execute. Numeric
  sibling environments correctly isolate branch-created tags; the arithmetic
  count alone does not publish an unconditional object selection. A reusable
  descriptive collection binding and whole-body historical controls are needed.

Deck, Wand, Delina, Iron Mastiff, Clay Golem's complete costs, and Xenosquirrels
remain unsupported explicit partials. Cone of Cold belongs to an earlier
source-cleared cohort and receives no duplicate credit here.

## Reviewable checkpoints

- c39f0897c: core arithmetic/envelopes and four original full-body scenarios.
- c81c7927c: consult-filter dependency and Bag known-card scenarios.
- ea021865c: operand token guard plus separate Wyll target/cardinality scope.
- 543864648: per-card public opening, hidden Bag boundary scenarios, rule link.
- e235b7527: exact prior reveal consumer hidden-boundary scenarios.
- 6c6f3c91d: complete outer stack-target cardinality before exact-head dispatch.
- 8ada462aa: unique per-card reveal occurrences and dispatch/history controls.
- bbbcdf124: authoritative empty-set scalar boundary and empty/all-land bodies.
- 39579eacf: separate Song of Inspiration scalar/body continuation.
- 72c011bae: distinct singular die occurrences and two-local-roll history control.

The functional commits are interleaved. The bounded six-body review cleared the
core/arithmetic, Bag, Wyll, both prior-consumer restorations, and occurrence fixes;
Song's 39579eacf was separately source-cleared. The combined disposition includes
72c011bae and all functional checkpoints above. Source-only diff checks are clean;
no execution-validation claims are made.
