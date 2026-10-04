# Conditional base-size alternative for a complete animation

**UNVALIDATED source proposal; no compilation or test execution.**

One proposed partial closure: **Behind the Mask**. It depends on the existing
Collect Evidence action/optional paid label, complete animation descriptors, and
the optional-size animation model (`1d015b71`, locally `304c1f7e`). Neither its
ordinary 4/3 animation nor its additional cost alone constitutes full coverage.

The bounded pre-parse follow-up recognizes `If <condition>, it has base power and
toughness <P/T> <duration> instead` only when the immediately preceding typed
effect is a complete animation with an existing Some base-size pair and the same
duration. It parses the P/T clause after consuming the syntactically scoped
terminal `instead`, clones the full previous animation, and changes only the
pair in the conditional self-replacement branch. The false branch retains the
original animation. No general diagnostic suppression or global stripping of
`instead` is used. Different durations and absent-size templates remain outside
this bounded rule.

Both alternatives retain the one announced artifact-or-creature target, type
transformation, subtype/color/ability details, and expiration. The resolving spell
uses its existing paid `Evidence` label; unrelated evidence collected earlier
that turn cannot select its replacement branch. The existing typed resolution
program represents the self-replacement before executing either complete action.

Authored unrun regressions:
- Full exact metadata-bearing fixture, normal tools strict/non-lossy aggregate.
- Direct and JSON-artifact runtime paths inspect the single self-replacement and
  both complete animation alternatives.
- Actual optional evidence casting payment (overpaying eight for six) and decline;
  artifact, creature and artifact-land recipients; existing counters and P/T
  modifiers; type/creature-subtype retention and end-of-turn expiration.
- An actual unrelated earlier collect-evidence action does not mark this spell's
  cost paid.
- Grammar compares the true branch against a clone of the false branch with only
  its pair changed, and rejects orphan/different-duration `instead` clauses. A
  normal non-instead later P/T instruction stays a separate conditional action.

Halfdane's end-of-next-upkeep duration remains unresolved. No measured coverage
change is claimed before deferred compilation and gameplay runs.
