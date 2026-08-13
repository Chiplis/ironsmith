# Parser module ownership after PR-32

PR-32 removes the legacy `runtime_backend/families` and
`runtime_backend/sentences` ownership roots. Their remaining reusable leaf
recognizers live with the canonical grammar under two semantic directories:

- `front_end/grammar/ability_rules` owns activated, keyword, restriction, and
  static-ability leaf rules.
- `front_end/grammar/effect_clauses` owns composable effect-clause recognition
  and library-clause support.

The former `families/mod.rs` and `sentences/mod.rs` facades are deleted. They
must not be recreated as re-export layers. The physical move deliberately
preserves rule bodies during the no-compilation migration window; PR-34 owns
the mechanical import corrections exposed when all checkpoints are compiled
together.

Whole-program meaning is not a module category. Document programs,
coordination, control flow, and lexical references are represented by the
compiler-owned nodes in `model/`; the grammar directories above may recognize
only reusable leaves and clauses.

The compiler model is already split by semantic ownership rather than numbered
shards: effects, actions, predicates, costs, legality, selections,
coordination, control flow, document programs, references, and structured
abilities each have one authoritative module. Cross-domain traversal belongs
to `model/visit.rs`; parser modules must not add duplicate recursive walkers.
