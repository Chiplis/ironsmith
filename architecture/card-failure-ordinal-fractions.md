# Ordinal unit fractions for resource quantities

Status: **UNVALIDATED source proposal**. No builds, compilation or tests were run.

Two exact full-card candidates are in `fixtures/ordinal_fraction_quantities.json.fixture`: **Pox** and **Dire Fleet Ravager**. Both frozen stack07 records failed with a missing life keyword because the article in `a third of their life` was being read as the cardinal one. Their proposed after-status is full metadata-bearing strict compilation without loss and executable, correctly rounded quantities.

The grammar now preserves `a/one <ordinal> of` as a denominator, using the existing `DividedRoundedDown` value and adding `denominator - 1` for upward rounding. The discard reader shares this fraction prefix and preserves the hand's player scope. Fractional sacrifice already had a typed denominator and dynamic chooser; the sentence-wide `Round up each time` transform now reaches divided counts as well as halves. There are no serialized model changes.

The shared execution/continuous evaluator widens only the existing fraction rounding-offset intermediate before division and checks that the quotient fits i32. Half-life upward rounding likewise adds in i64. This prevents overflow at i32::MAX without claiming a wider general arithmetic model or support for Mathemagics.

Prerequisite `f1be31f8` fixes the earlier Pox Plague proposal: an unstated half-life value now rounds down, matching the shared arithmetic default and the document normalizer's treatment of `Round down each time`. Explicit upward rounding and a later sentence-wide upward instruction remain effective. Pox Plague is not counted again here.

Authored evidence:
- Normal tools aggregate for both exact full payloads.
- Normal compiler-runtime target with five tests, using direct and round-tripped artifact definitions: complete artifacts; multiplayer Pox life/discard/creature/land quantities with empty and nonempty sets and excluded artifacts; Ravager's live life totals on all players; and a 2/3/4-denominator discard property across zero through eight cards and both rounding directions.
- A maximum-life Ravager scenario uses a public life gain, and interpreter tests cover signed boundaries and continuous/execution agreement.
- Grammar assertions for ordinal recognition/rejection, explicit owner versus participant, the denominator-minus-one offset, and idempotent rounding of dynamic chooser counts.

Personal Incarnation remains separately uncounted until its owner-only redirect body and owner-relative life loss have both been reviewed. Other half/count diagnostics with independent copy, replacement, delayed-X, or characteristic-defining mechanisms are not included.
