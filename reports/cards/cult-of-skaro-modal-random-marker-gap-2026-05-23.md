## Card

- Name: `Cult of Skaro`
- Date: `2026-05-23`

## Requested Goal

- Parse strictly and reach semantic similarity `>= 0.99`.

## Current Failure

- `cargo run -p ironsmith-tools --bin compile_oracle_text -- --name "Cult of Skaro" --compare-text`
- Error: `parse failed for Cult of Skaro: compiled text dropped required semantic marker: at-random`

## Investigation Summary

- The card is recognized as a modal triggered ability with bullet modes.
- The strict failure is specifically the missing `at-random` semantic marker from compiled text.
- A parser/lowering pass would need reusable structural support to preserve modal-header random choice semantics through lowering and compiled-text rendering.

## Why This Is Blocked In Single-Card Parser Pass

- The current modal pipeline does not expose a stable, reusable structural signal that survives end-to-end for `choose one at random` modal headers.
- Fixing this correctly requires a reusable model capability (modal random-choice metadata in parser model/lowering/runtime/renderer contract), not a card-specific text rewrite.
- Card-specific output rewriting is explicitly disallowed and would be a comparison hack rather than real support.

## Required Reusable Capability

- Add reusable modal random-choice semantics support so that:
  - parser captures random modal choice in structured modal header data,
  - lowering/runtime effect model preserves it,
  - compiled-text rendering emits random modal choice wording (`choose ... at random`) from AST/effect data.

## Next Step

- Route follow-up to `$ironsmith-effect-creator` (or equivalent multi-subsystem reusable implementation pass) before retrying this card-fix loop.
