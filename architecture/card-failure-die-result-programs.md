# Die arithmetic and complete result programs

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
