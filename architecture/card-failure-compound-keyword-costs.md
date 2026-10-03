# Compound Blight and Forage activation costs

Status: **UNVALIDATED** source implementation and authored regressions. No build,
compilation, or test execution was performed. Proposed coverage is not measured
successful compilation.

## Frozen root family and coverage boundary

Eight `stack07-bc9e56e2` failures report unsupported activation-cost segments
containing Blight or Forage. Full frozen source text and Oracle IDs are in
`fixtures/compound_keyword_costs.json.fixture`.

Seven complete-card candidates:

- Camellia, the Seedmiser
- Champion of the Weird
- Evershrike's Gift
- Gristle Glutton
- Spiral into Solitude
- Sting-Slinger
- Thornvault Forager

One partial candidate, **not counted as complete coverage**: Dawnhand Dissident.
Its two Blight activations share this correction, but its following permission
to cast source-exiled creature spells by removing three counters from among
creatures requires a separate permission reader. The fixture marks this blocker;
the explicitly named cost-only test does not assert full-card compilation.

## Correction

The comma/conjunction activation-cost boundary reader omitted `blight`, despite
an existing typed Blight component and real counter-payment executor. Add the
missing component boundary. Recognize `forage` as its own complete CST component
and assemble it into the established typed Forage action cost. That executor
actually exiles three owned graveyard cards or sacrifices a controlled Food,
validates payment, and emits the Forage action event. No runtime placeholder,
card-name branch, skipped payment, or fallback is introduced.

Existing leading mana, tap, life, and trailing sacrifice components remain in
order and mandatory. Unexpected qualifiers and counts fail closed.

## Authored, unrun regressions

- Compound grammar for mana/tap/Blight, life/Blight, tap/Forage, and trailing
  sacrifice; unsupported keyword suffixes remain errors.
- Seven complete exact cards through strict public compilation and typed JSON
  transport. Dawnhand is a separate partial cost-only probe.
- Sting-Slinger pays mana, taps, and places a counter on the chosen owned
  creature before damage; its effect survives source departure and its Blight
  event records the correct actor.
- Evershrike's Gift requires an owned creature and two counters before its
  graveyard-to-hand return changes object identity.
- Thornvault Forager rejects opponent resources and an incomplete two-card
  graveyard payment, executes each real Food/graveyard alternative, produces
  mana without using the stack, and records one actor-correct Forage event.

Deferred commands:

`cargo test -p ironsmith-compiler-grammar --lib compound_keyword_costs`

`cargo test -p ironsmith-compiler-runtime --test compound_keyword_costs`
