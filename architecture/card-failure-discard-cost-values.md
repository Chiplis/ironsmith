# Typed discard cost selectors and X

Status: **UNVALIDATED**. No builds, compilation, or tests run under the implementation-first campaign instruction.

Frozen stack07 `bc9e56e2` identities: `fixtures/discard_cost_values.json.fixture`.

The grammar/payment implementations for Knollspine Invocation, Krovikan Sorcerer,
Sanctum Spirit and Kozilek remain integrated, but all four are **partial** pending
the completed-action hand-disclosure Undo correction. Their whole-card source
coverage claims are withheld. Kozilek's draw body is now represented by the
relative-hand quantity family. See
[the exact disclosure boundary](card-failure-payment-disclosure-boundary.md).

Gix, Yawgmoth Praetor additionally retains its independent plural immediate
play/cast clause blocker; a singular CastTaggedEffect cannot stand in for
choosing and playing multiple cards from the exiled pool.

## Mechanism

- Existing simple discard syntax retains its original compiler payload. Previously unsupported selectors fall back to the complete typed card-filter grammar, retaining nonblack, historic and mana-value-X predicates rather than reducing them to a card type. Group-relational selectors are deliberately rejected by this per-card path until their selection semantics exist.
- A compiler-only variable-count segment lowers to the existing DiscardEffect. No runtime payload or enum wire shape changes.
- DiscardEffect supplies the cost-X hooks for X cards or cards whose mana value equals X. Bounds come from the payer's eligible hand, including same-value groups when more than one card is required. Other players' cards and other filter predicates remain excluded.
- Context-aware preflight uses announced X, source snapshots, tags, and reserved-entry exclusions. Before announcement it checks whether some X is possible; actual payment rejects a missing X. Unresolved counts fail closed instead of silently becoming zero. Casting-source exclusion remains reason-aware.
- X-bearing discard costs retain their full effect through the activation pipeline rather than becoming fixed one-card prompts that lose the X binding.
- The simple display helper defers complex filters to the typed text renderer. The existing `other` qualifier is preserved rather than falling into the unfiltered single-card case.

## Authored regressions

Strict/artifact transport for the three complete proposals; real Knollspine payment/damage and noncontiguous X values, X=0 still requiring one card, payer ownership, historic branches, both Krovikan abilities and the linked newly drawn-card discard, isolated printed Gix/Kozilek costs without claiming their independent bodies, a real spell cast/counter sequence, cancellation rollback, insufficient/unannounced X, unknown tags, and reserved simultaneous-entry objects.

Deferred command: `cargo test -p ironsmith-compiler-runtime --test discard_cost_values -- --nocapture`.
