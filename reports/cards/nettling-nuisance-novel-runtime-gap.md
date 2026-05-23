## Nettling Nuisance single-card pass report

- Card: `Nettling Nuisance`
- Requested target: semantic similarity `>= 0.99`
- Current live status: parses strictly, but similarity remains below target.

### What was fixed

- Added parser support for passive goad clauses like `the token is goaded ...` in `clause_dispatch`.
- Added focused regression coverage for this passive goad pattern.

### Live verification snapshot

Command:

`cargo run -p ironsmith-tools --bin compile_oracle_text -- --name "Nettling Nuisance" --compare-text`

Result:

- Similarity: `0.6852`
- Parse: successful (no parse error)
- Compiled text:
  `Flying`
  `Whenever one or more Faerie creature you control deal combat damage to a player, that player creates a 4/2 red Pirate creature token with "This token can't block.". Goad that creature.`

### Why the target is blocked

The oracle clause says the created token `is goaded for the rest of the game`.

Current runtime goad execution (`GoadEffect`) hardcodes `Until::YourNextTurn` and does not expose a reusable way to apply a goad designation with a longer duration such as "rest of game". Reaching semantic parity for this card therefore requires a reusable runtime capability upgrade for goad duration modeling, not only parser/lowering text handling.

### Missing reusable capability

- Parameterizable goad duration support in runtime/lowering (for example, allowing goad to use non-default durations including rest-of-game where required by card text).

This goes beyond parser-only single-card scope and should be handled as a reusable effect/runtime enhancement.
