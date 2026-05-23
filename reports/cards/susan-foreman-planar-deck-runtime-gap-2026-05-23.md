## Card

- Name: `Susan Foreman`
- Date: `2026-05-23`

## Requested Goal

- Parse strictly and reach semantic similarity `>= 0.99` if feasible.

## Current Failure

- Command: `cargo run -p ironsmith-tools --bin compile_oracle_text -- --name "Susan Foreman" --compare-text`
- Error: `parse failed for Susan Foreman: unsupported look library owner (clause: 'the top two cards of your planar deck'); oracle-only fallback also failed: unsupported look library owner (clause: 'the top two cards of your planar deck')`

## Investigation Summary

- The card uses Planechase-only objects and actions: `planar deck` and `planeswalk`.
- The strict parser currently routes this clause through library-top look parsing, which only accepts owners of `... library` and rejects `... planar deck`.
- The core zone model in `crates/ironsmith-core/src/zone.rs` has no `PlanarDeck` zone, so lowering/runtime cannot represent or execute this effect family structurally.

## Why This Is Blocked In Single-Card Scope

- Mapping `planar deck` to `library` would be a semantic approximation, not a correct structural model.
- Even if the owner parse accepted `planar deck`, this card still needs reusable support for Planechase zone handling and `would planeswalk ... instead ... then planeswalk` replacement semantics.
- Implementing this correctly requires cross-subsystem reusable capability (core zone model, parser/lowering target zone support, runtime execution, and compiled-text rendering), which is outside a narrow parser-only single-card fix.

## Required Reusable Capability

- Add first-class Planechase support, including:
- a reusable `planar deck` zone/model representation,
- effect primitives for looking at/reordering top cards of planar decks,
- replacement semantics support for `if you would planeswalk, instead ... then planeswalk`.

## Next Step

- Route follow-up to `$ironsmith-effect-creator` for a reusable multi-subsystem implementation pass, then rerun the single-card fixer loop.
