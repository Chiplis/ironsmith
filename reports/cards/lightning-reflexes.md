## Lightning Reflexes single-card fix pass

- Card: `Lightning Reflexes`
- Target: strict parse and semantic similarity `>= 0.99`
- Live check command:
  - `cargo run --release -p ironsmith-tools --bin compile_oracle_text -- --name "Lightning Reflexes" --compare-text`

### Current blocker

The card fails on this line family:

`You may cast this spell as though it had flash. If you cast it any time a sorcery couldn't have been cast, the controller of the permanent it becomes sacrifices it at the beginning of the next cleanup step.`

This is a reusable engine gap, not a card-local parser typo.

### Why this is out of single-card scope

To model this line correctly in AST/runtime, Ironsmith needs a reusable capability that combines:

1. **Cast-time timing classification persistence** on the resolving permanent
   - Detect whether the spell was cast at non-sorcery timing.
   - Preserve that fact from stack object to permanent object.
2. **Conditional delayed sacrifice scheduling** keyed to that cast-time fact
   - On resolution/entry, if cast at non-sorcery timing, schedule sacrifice for the resulting permanent's controller.
3. **`next cleanup step` delayed trigger timing support**
   - Existing common delayed-timing support is focused on `next end step`; this clause specifically needs cleanup-step scheduling.

Without those three pieces, any parser-only rewrite would either:

- drop real semantics, or
- introduce card-shaped text hacks.

Both violate AST-first/reusable-fix constraints for this workflow.

### Recommended reusable implementation direction

1. Add a generic runtime flag on spell/permanent lineage for "cast at non-sorcery timing".
2. Add a generic delayed-trigger duration for "at the beginning of the next cleanup step".
3. Add a reusable parser/lowering pattern for this line family that emits structured model fields instead of text fallback.
4. Add regression tests:
   - parser/lowering test for the line family
   - runtime test proving sacrifice only when cast off sorcery timing
   - runtime timing test proving cleanup-step timing (not end step)

### Live verification result (this pass)

`compile_oracle_text` still fails with unsupported line-family parse for the clause above, so strict compile and score target are not yet reachable in this pass.
